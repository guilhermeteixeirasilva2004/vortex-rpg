use std::collections::HashMap;
use std::time::Duration;

use axum::extract::ws::Utf8Bytes;
use shared::movement::{self, EYE_HEIGHT};
use shared::protocol::{self, PlayerId, PlayerInput, PlayerState, ServerMessage};
use shared::voxel::{terrain_height, Chunk};
use shared::{SERVER_TICK_RATE, TICK_DT};
use tokio::sync::{broadcast, mpsc};
use tokio::time::MissedTickBehavior;

/// Eventos que as tasks de conexão mandam para o loop de jogo.
#[derive(Debug)]
pub enum GameEvent {
    Join { id: PlayerId },
    Leave { id: PlayerId },
    Input { id: PlayerId, input: PlayerInput },
}

struct Player {
    position: [f32; 3],
    input: PlayerInput,
}

pub struct GameWorld {
    chunks: Vec<Chunk>,
    players: HashMap<PlayerId, Player>,
    tick: u64,
}

pub fn spawn_point() -> [f32; 3] {
    [0.5, terrain_height(0, 0) as f32 + 1.0 + EYE_HEIGHT, 0.5]
}

impl GameWorld {
    pub fn new(chunks: Vec<Chunk>) -> Self {
        Self {
            chunks,
            players: HashMap::new(),
            tick: 0,
        }
    }

    pub fn chunks(&self) -> &[Chunk] {
        &self.chunks
    }

    fn handle(&mut self, event: GameEvent) {
        match event {
            GameEvent::Join { id } => {
                tracing::info!("jogador {id} entrou");
                self.players.insert(
                    id,
                    Player {
                        position: spawn_point(),
                        input: PlayerInput::default(),
                    },
                );
            }
            GameEvent::Leave { id } => {
                tracing::info!("jogador {id} saiu");
                self.players.remove(&id);
            }
            GameEvent::Input { id, input } => {
                if let Some(p) = self.players.get_mut(&id) {
                    // Ignora pacotes fora de ordem.
                    if input.seq >= p.input.seq {
                        p.input = input.sanitized();
                    }
                }
            }
        }
    }

    fn simulate(&mut self) {
        self.tick += 1;
        for p in self.players.values_mut() {
            p.position = movement::step(p.position, &p.input, TICK_DT);
        }
    }

    fn snapshot(&self) -> ServerMessage {
        ServerMessage::Snapshot {
            tick: self.tick,
            players: self
                .players
                .iter()
                .map(|(&id, p)| PlayerState {
                    id,
                    position: p.position,
                    yaw: p.input.yaw,
                    pitch: p.input.pitch,
                    last_input_seq: p.input.seq,
                })
                .collect(),
        }
    }
}

/// Loop de simulação em taxa fixa. Único dono do estado do jogo.
pub async fn run(
    mut world: GameWorld,
    mut events: mpsc::UnboundedReceiver<GameEvent>,
    snapshots: broadcast::Sender<Utf8Bytes>,
) {
    let mut interval = tokio::time::interval(Duration::from_secs_f64(
        1.0 / f64::from(SERVER_TICK_RATE),
    ));
    interval.set_missed_tick_behavior(MissedTickBehavior::Skip);

    loop {
        interval.tick().await;

        while let Ok(event) = events.try_recv() {
            world.handle(event);
        }
        world.simulate();

        if world.players.is_empty() {
            continue;
        }
        match protocol::encode(&world.snapshot()) {
            // Erro só significa "ninguém inscrito agora".
            Ok(json) => {
                let _ = snapshots.send(json.into());
            }
            Err(e) => tracing::error!("falha ao serializar snapshot: {e}"),
        }
    }
}
