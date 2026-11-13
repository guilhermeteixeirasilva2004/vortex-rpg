//! Conexão WebSocket numa thread própria (runtime tokio), conversando com o
//! mundo do Bevy por canais. Os sistemas do Bevy nunca bloqueiam na rede.

use bevy::prelude::*;
use crossbeam_channel::{Receiver, Sender};
use engine_voxel::Chunk;
use futures_util::{SinkExt, StreamExt};
use game_protocol::{ClientMessage, CreatureState, PlayerId, PlayerState, ServerMessage};
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};
use tokio_tungstenite::tungstenite::Message;

/// Servidor padrão (sobrescrito com `VORTEX_SERVER`).
const DEFAULT_SERVER_URL: &str = "ws://127.0.0.1:3000/ws";

pub struct NetworkPlugin;

impl Plugin for NetworkPlugin {
    fn build(&self, app: &mut App) {
        let url = std::env::var("VORTEX_SERVER").unwrap_or_else(|_| DEFAULT_SERVER_URL.into());
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

/// Chunks recebidos e ainda não processados pelo mundo.
#[derive(Resource, Default)]
pub struct IncomingChunks(pub Vec<Chunk>);

/// Snapshot mais recente do servidor. Só é escrito quando chega um novo,
/// então consumidores usam `is_changed()` para saber se há novidade.
#[derive(Resource, Default)]
pub struct LatestSnapshot {
    /// Tick do snapshot.
    pub tick: u64,
    /// Estado dos jogadores.
    pub players: Vec<PlayerState>,
    /// Estado das criaturas.
    pub creatures: Vec<CreatureState>,
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
            ServerMessage::ChunkData(dto) => match Chunk::try_from(dto) {
                Ok(chunk) => chunks.0.push(chunk),
                Err(e) => warn!("chunk inválido recebido: {e}"),
            },
            ServerMessage::Snapshot {
                tick,
                players,
                creatures,
            } => {
                if tick >= snapshot.tick {
                    snapshot.tick = tick;
                    snapshot.players = players;
                    snapshot.creatures = creatures;
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
                    match game_protocol::decode::<ServerMessage>(text.as_str()) {
                        Ok(m) => if incoming.send(m).is_err() { break },
                        Err(e) => warn!("mensagem inválida do servidor: {e}"),
                    }
                }
                Some(Ok(Message::Close(_))) | None => break,
                Some(Ok(_)) => {}
                Some(Err(e)) => return Err(e.into()),
            },
            out = outgoing.recv() => match out {
                Some(m) => write.send(Message::text(game_protocol::encode(&m)?)).await?,
                None => break,
            },
        }
    }
    Ok(())
}
