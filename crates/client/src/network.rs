//! Conexão WebSocket numa thread própria (runtime tokio), conversando com o
//! mundo do Bevy por canais. Os sistemas do Bevy nunca bloqueiam na rede.

use bevy::prelude::*;
use crossbeam_channel::{Receiver, Sender};
use futures_util::{SinkExt, StreamExt};
use shared::protocol::{self, ClientMessage, PlayerId, PlayerState, ServerMessage};
use shared::voxel::Chunk;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};
use tokio_tungstenite::tungstenite::Message;

pub struct NetworkPlugin;

impl Plugin for NetworkPlugin {
    fn build(&self, app: &mut App) {
        let url = std::env::var("VORTEX_SERVER").unwrap_or_else(|_| shared::DEFAULT_SERVER_URL.into());
        app.insert_resource(spawn_network_thread(url))
            .init_resource::<LocalPlayer>()
            .init_resource::<IncomingChunks>()
            .init_resource::<LatestSnapshot>()
            .add_systems(PreUpdate, receive_messages);
    }
}

#[derive(Resource)]
pub struct NetClient {
    outgoing: UnboundedSender<ClientMessage>,
    incoming: Receiver<ServerMessage>,
}

impl NetClient {
    pub fn send(&self, msg: ClientMessage) {
        // Falha só se a thread de rede morreu; o erro já foi logado lá.
        let _ = self.outgoing.send(msg);
    }
}

#[derive(Resource, Default)]
pub struct LocalPlayer {
    pub id: Option<PlayerId>,
}

#[derive(Resource, Default)]
pub struct IncomingChunks(pub Vec<Chunk>);

#[derive(Resource, Default)]
pub struct LatestSnapshot {
    pub tick: u64,
    pub players: Vec<PlayerState>,
    /// Marcado quando chega snapshot novo; consumidores limpam.
    pub fresh: bool,
}

fn receive_messages(
    net: Res<NetClient>,
    mut local: ResMut<LocalPlayer>,
    mut chunks: ResMut<IncomingChunks>,
    mut snapshot: ResMut<LatestSnapshot>,
) {
    for msg in net.incoming.try_iter() {
        match msg {
            ServerMessage::Welcome { player_id, spawn } => {
                info!("conectado como jogador {player_id}, spawn em {spawn:?}");
                local.id = Some(player_id);
            }
            ServerMessage::ChunkData(chunk) => {
                if chunk.is_valid() {
                    chunks.0.push(chunk);
                } else {
                    warn!("chunk inválido recebido em {:?}", chunk.pos);
                }
            }
            ServerMessage::Snapshot { tick, players } => {
                if tick >= snapshot.tick {
                    snapshot.tick = tick;
                    snapshot.players = players;
                    snapshot.fresh = true;
                }
            }
        }
    }
}

fn spawn_network_thread(url: String) -> NetClient {
    let (out_tx, out_rx) = unbounded_channel::<ClientMessage>();
    let (in_tx, in_rx) = crossbeam_channel::unbounded::<ServerMessage>();

    std::thread::Builder::new()
        .name("network".into())
        .spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("falha ao criar runtime tokio");
            rt.block_on(async move {
                info!("conectando em {url}");
                match run_connection(&url, out_rx, in_tx).await {
                    Ok(()) => info!("conexão encerrada"),
                    Err(e) => error!("erro de rede: {e}"),
                }
            });
        })
        .expect("falha ao criar thread de rede");

    NetClient {
        outgoing: out_tx,
        incoming: in_rx,
    }
}

async fn run_connection(
    url: &str,
    mut outgoing: UnboundedReceiver<ClientMessage>,
    incoming: Sender<ServerMessage>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let (ws, _) = tokio_tungstenite::connect_async(url).await?;
    let (mut write, mut read) = ws.split();

    loop {
        tokio::select! {
            msg = read.next() => match msg {
                Some(Ok(Message::Text(text))) => {
                    match protocol::decode::<ServerMessage>(text.as_str()) {
                        Ok(m) => if incoming.send(m).is_err() { break },
                        Err(e) => warn!("mensagem inválida do servidor: {e}"),
                    }
                }
                Some(Ok(Message::Close(_))) | None => break,
                Some(Ok(_)) => {}
                Some(Err(e)) => return Err(e.into()),
            },
            out = outgoing.recv() => match out {
                Some(m) => write.send(Message::text(protocol::encode(&m)?)).await?,
                None => break,
            },
        }
    }
    Ok(())
}
