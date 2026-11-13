//! Adaptador de transporte: WebSocket (axum). Cada conexão recebe o mundo,
//! repassa o input como `Command` para o loop de tick e encaminha os
//! snapshots transmitidos por ele.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use axum::extract::ws::{Message, Utf8Bytes, WebSocket, WebSocketUpgrade};
use axum::extract::State;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::Router;
use engine_voxel::ChunkMap;
use futures_util::{SinkExt, StreamExt};
use game_protocol::{ChunkDto, ClientMessage, ServerMessage};
use game_sim::Command;
use tokio::sync::{broadcast, mpsc};

/// Estado compartilhado entre as conexões.
#[derive(Clone)]
pub struct AppState {
    commands: mpsc::UnboundedSender<Command>,
    snapshots: broadcast::Sender<Utf8Bytes>,
    chunks: Arc<ChunkMap>,
    spawn: [f32; 3],
    next_id: Arc<AtomicU64>,
}

impl AppState {
    /// Liga o transporte aos canais do loop de tick e ao mundo.
    pub fn new(
        commands: mpsc::UnboundedSender<Command>,
        snapshots: broadcast::Sender<Utf8Bytes>,
        chunks: Arc<ChunkMap>,
        spawn: [f32; 3],
    ) -> Self {
        Self {
            commands,
            snapshots,
            chunks,
            spawn,
            next_id: Arc::new(AtomicU64::new(1)),
        }
    }
}

/// Rotas HTTP: `/ws` (jogo) e `/health`.
pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/ws", get(ws_handler))
        .route("/health", get(|| async { "ok" }))
        .with_state(state)
}

async fn ws_handler(ws: WebSocketUpgrade, State(state): State<AppState>) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn send(socket: &mut WebSocket, msg: &ServerMessage) -> bool {
    match game_protocol::encode(msg) {
        Ok(json) => socket.send(Message::Text(json.into())).await.is_ok(),
        Err(e) => {
            tracing::error!("falha ao serializar: {e}");
            false
        }
    }
}

async fn handle_socket(mut socket: WebSocket, state: AppState) {
    let id = state.next_id.fetch_add(1, Ordering::Relaxed);

    // Handshake: id, spawn e o mundo inicial.
    if !send(
        &mut socket,
        &ServerMessage::Welcome {
            player_id: id,
            spawn: state.spawn,
        },
    )
    .await
    {
        return;
    }
    for chunk in state.chunks.iter() {
        if !send(
            &mut socket,
            &ServerMessage::ChunkData(ChunkDto::from(chunk)),
        )
        .await
        {
            return;
        }
    }

    let mut snapshots = state.snapshots.subscribe();
    if state.commands.send(Command::Join { id }).is_err() {
        return;
    }

    let (mut tx, mut rx) = socket.split();

    let mut send_task = tokio::spawn(async move {
        loop {
            match snapshots.recv().await {
                Ok(json) => {
                    if tx.send(Message::Text(json)).await.is_err() {
                        break;
                    }
                }
                // Cliente lento: pula snapshots antigos, só o mais novo importa.
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
                Err(broadcast::error::RecvError::Closed) => break,
            }
        }
    });

    let commands = state.commands.clone();
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = rx.next().await {
            match msg {
                Message::Text(text) => {
                    match game_protocol::decode::<ClientMessage>(text.as_str()) {
                        Ok(ClientMessage::Input(input)) => {
                            if commands
                                .send(Command::Input {
                                    id,
                                    input: input.into(),
                                })
                                .is_err()
                            {
                                break;
                            }
                        }
                        Err(e) => tracing::warn!("jogador {id}: mensagem inválida: {e}"),
                    }
                }
                Message::Close(_) => break,
                _ => {}
            }
        }
    });

    tokio::select! {
        _ = &mut send_task => recv_task.abort(),
        _ = &mut recv_task => send_task.abort(),
    }

    let _ = state.commands.send(Command::Leave { id });
}
