use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use axum::extract::ws::Utf8Bytes;
use engine_voxel::{ChunkMap, ChunkPos, WorldGenerator};
use shared::movement::{Body, EYE_HEIGHT};
use shared::protocol::{self, PlayerId, PlayerInput, PlayerState, ServerMessage};
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

/// Mundo inicial: 8x2x8 chunks ao redor da origem.
const WORLD_RADIUS_CHUNKS: i32 = 4;
const WORLD_HEIGHT_CHUNKS: i32 = 2;

pub struct GameWorld {
    /// Única cópia dos chunks; a camada de rede compartilha o mesmo `Arc`.
    /// Chunks ausentes (vazios ou fora do mundo gerado) contam como ar.
    chunks: Arc<ChunkMap>,
    spawn: [f32; 3],
    players: HashMap<PlayerId, Player>,
    tick: u64,
    /// Se o servidor aceita o modo voo (debug) pedido pelos clientes.
    allow_fly: bool,
}

impl GameWorld {
    pub fn new(generator: &impl WorldGenerator, allow_fly: bool) -> Self {
        let mut chunks = ChunkMap::new();
        for x in -WORLD_RADIUS_CHUNKS..WORLD_RADIUS_CHUNKS {
            for z in -WORLD_RADIUS_CHUNKS..WORLD_RADIUS_CHUNKS {
                for y in 0..WORLD_HEIGHT_CHUNKS {
                    let chunk = generator.generate_chunk(ChunkPos::new(x, y, z));
                    if !chunk.is_empty() {
                        chunks.insert(chunk);
                    }
                }
            }
        }
        tracing::info!("mundo gerado: {} chunks não vazios", chunks.len());
        let spawn = [
            0.5,
            generator.surface_height(0, 0) as f32 + 1.0 + EYE_HEIGHT,
            0.5,
        ];
        Self {
            chunks: Arc::new(chunks),
            spawn,
            players: HashMap::new(),
            tick: 0,
            allow_fly,
        }
    }

    pub fn chunks(&self) -> &Arc<ChunkMap> {
        &self.chunks
    }

    /// Onde jogadores nascem (e renascem ao cair no vazio).
    pub fn spawn(&self) -> [f32; 3] {
        self.spawn
    }

    fn handle(&mut self, event: GameEvent) {
        match event {
            GameEvent::Join { id } => {
                tracing::info!("jogador {id} entrou");
                self.players.insert(
                    id,
                    Player {
                        body: Body::at(self.spawn),
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
            p.body
                .step(&p.input, self.chunks.as_ref(), TICK_DT, self.allow_fly);
            if p.body.position[1] < VOID_Y {
                tracing::info!("jogador {id} caiu no vazio, voltando ao spawn");
                p.body = Body::at(self.spawn);
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
    let mut interval =
        tokio::time::interval(Duration::from_secs_f64(1.0 / f64::from(SERVER_TICK_RATE)));
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

/// Golden da simulação: roteiro fixo de inputs, posições fixadas bit a bit
/// (via `{:?}` do f32, que é exato). Protege a refatoração contra mudança de
/// comportamento.
#[cfg(test)]
mod golden {
    use std::fmt::Write;

    use super::*;
    use engine_voxel::HeightmapGenerator;

    fn world(allow_fly: bool) -> GameWorld {
        GameWorld::new(&HeightmapGenerator, allow_fly)
    }

    /// (forward, right, up, fly, yaw) de cada jogador no tick `t`.
    fn script(id: PlayerId, t: u32) -> (f32, f32, f32, bool, f32) {
        match id {
            // Anda, pula segurando Espaço, voa, sai do voo e cai.
            1 => match t {
                0..20 => (0.0, 0.0, 0.0, false, 0.0),
                20..60 => (
                    1.0,
                    0.0,
                    if (30..36).contains(&t) { 1.0 } else { 0.0 },
                    false,
                    0.3,
                ),
                60..80 => (0.0, 1.0, 1.0, true, 0.3),
                _ => (1.0, 1.0, 0.0, false, 2.0),
            },
            // Anda na diagonal sem parar (sobe/desce terreno, bate em degraus).
            2 => (1.0, -1.0, if t % 40 < 5 { 1.0 } else { 0.0 }, false, 4.0),
            // Voa para baixo atravessando o terreno até cair no vazio.
            3 => (0.0, 0.0, -1.0, true, 0.0),
            // Pede voo; no mundo sem voo isso deve ser ignorado.
            _ => (0.5, 0.0, 1.0, true, 5.5),
        }
    }

    fn run(allow_fly: bool) -> String {
        let mut w = world(allow_fly);
        for id in 1..=4 {
            w.handle(GameEvent::Join { id });
        }
        let mut out = String::new();
        for t in 0..240u32 {
            for id in 1..=4 {
                let (forward, right, up, fly, yaw) = script(id, t);
                let input = PlayerInput {
                    seq: t + 1,
                    forward,
                    right,
                    up,
                    fly,
                    yaw,
                    pitch: 0.1,
                };
                w.handle(GameEvent::Input { id, input });
            }
            // Pacote atrasado (seq antigo) deve ser ignorado.
            if t > 0 {
                let stale = PlayerInput {
                    seq: t,
                    forward: -1.0,
                    ..Default::default()
                };
                w.handle(GameEvent::Input {
                    id: 2,
                    input: stale,
                });
            }
            w.simulate();
            if t % 20 == 19 {
                let ServerMessage::Snapshot { tick, mut players } = w.snapshot() else {
                    unreachable!()
                };
                players.sort_by_key(|p| p.id);
                for p in players {
                    writeln!(
                        out,
                        "{tick} {} {:?} yaw={:?} pitch={:?} fly={} seq={}",
                        p.id, p.position, p.yaw, p.pitch, p.flying, p.last_input_seq
                    )
                    .unwrap();
                }
            }
        }
        out
    }

    #[test]
    fn simulation_with_fly_allowed() {
        assert_eq!(run(true), include_str!("golden/sim_fly.txt"));
    }

    #[test]
    fn simulation_with_fly_denied() {
        assert_eq!(run(false), include_str!("golden/sim_nofly.txt"));
    }
}
