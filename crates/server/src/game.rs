use std::collections::HashMap;
use std::time::Duration;

use axum::extract::ws::Utf8Bytes;
use shared::movement::{Body, VoxelQuery, EYE_HEIGHT};
use shared::protocol::{self, PlayerId, PlayerInput, PlayerState, ServerMessage};
use shared::voxel::{terrain_height, Chunk, ChunkPos, CHUNK_SIZE_I32};
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

/// Abaixo disso o jogador caiu para fora do mundo e volta ao spawn.
const VOID_Y: f32 = -64.0;

struct Player {
    body: Body,
    input: PlayerInput,
}

/// Chunks indexados por posição, para a física consultar blocos.
/// Chunks ausentes (vazios ou fora do mundo gerado) contam como ar.
struct Terrain(HashMap<ChunkPos, Chunk>);

impl VoxelQuery for Terrain {
    fn is_solid(&self, x: i32, y: i32, z: i32) -> bool {
        self.0
            .get(&ChunkPos::from_world_block(x, y, z))
            .and_then(|c| {
                c.get(
                    x.rem_euclid(CHUNK_SIZE_I32),
                    y.rem_euclid(CHUNK_SIZE_I32),
                    z.rem_euclid(CHUNK_SIZE_I32),
                )
            })
            .is_some_and(|b| b.is_solid())
    }
}

pub struct GameWorld {
    chunks: Vec<Chunk>,
    terrain: Terrain,
    players: HashMap<PlayerId, Player>,
    tick: u64,
    /// Se o servidor aceita o modo voo (debug) pedido pelos clientes.
    allow_fly: bool,
}

pub fn spawn_point() -> [f32; 3] {
    [0.5, terrain_height(0, 0) as f32 + 1.0 + EYE_HEIGHT, 0.5]
}

impl GameWorld {
    pub fn new(chunks: Vec<Chunk>, allow_fly: bool) -> Self {
        let terrain = Terrain(chunks.iter().map(|c| (c.pos, c.clone())).collect());
        Self {
            chunks,
            terrain,
            players: HashMap::new(),
            tick: 0,
            allow_fly,
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
                        body: Body::at(spawn_point()),
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
        for (id, p) in self.players.iter_mut() {
            p.body.step(&p.input, &self.terrain, TICK_DT, self.allow_fly);
            if p.body.position[1] < VOID_Y {
                tracing::info!("jogador {id} caiu no vazio, voltando ao spawn");
                p.body = Body::at(spawn_point());
            }
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
                    position: p.body.position,
                    yaw: p.input.yaw,
                    pitch: p.input.pitch,
                    flying: p.body.flying,
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
