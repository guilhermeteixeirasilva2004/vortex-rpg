//! Núcleo da aplicação: estado dos jogadores, comandos (entrar, sair, input),
//! avanço de um tick e snapshot.
//!
//! Puro: não lê relógio, ambiente nem rede. O mundo vem de um
//! `WorldGenerator` injetado e as regras de `SimConfig`; quem chama decide
//! quando avançar o tick e o que fazer com o snapshot.

use std::collections::HashMap;
use std::sync::Arc;

use engine_physics::{Body, MoveIntent, EYE_HEIGHT};
use engine_voxel::{ChunkMap, ChunkPos, WorldGenerator};

/// Identificador de jogador, atribuído pela camada de transporte.
pub type PlayerId = u64;

/// Ticks de simulação por segundo.
pub const TICK_RATE: u32 = 20;
/// Duração de um tick, em segundos.
pub const TICK_DT: f32 = 1.0 / TICK_RATE as f32;

/// Abaixo disso o jogador caiu para fora do mundo e volta ao spawn.
const VOID_Y: f32 = -64.0;

/// Regras e tamanho do mundo, decididos por quem monta a simulação.
#[derive(Clone, Copy, Debug)]
pub struct SimConfig {
    /// Mundo gerado vai de `-r..r` chunks em X e Z.
    pub world_radius_chunks: i32,
    /// Mundo gerado vai de `0..h` chunks em Y.
    pub world_height_chunks: i32,
    /// Se o voo livre (debug) pedido pelos jogadores é aceito.
    pub allow_fly: bool,
}

/// Input mais recente de um jogador, já validado pela borda.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct InputFrame {
    /// Número de sequência; inputs com `seq` menor que o atual são ignorados.
    pub seq: u32,
    /// O que o jogador quer fazer (o voo ainda passa pela permissão).
    pub intent: MoveIntent,
    /// Radianos, olhar pra cima/baixo (só repassado no snapshot).
    pub pitch: f32,
}

/// O que pode acontecer com a simulação entre ticks.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Command {
    /// Jogador entrou: nasce no spawn.
    Join {
        /// Quem entrou.
        id: PlayerId,
    },
    /// Jogador saiu.
    Leave {
        /// Quem saiu.
        id: PlayerId,
    },
    /// Novo input "segurado" do jogador.
    Input {
        /// De quem é o input.
        id: PlayerId,
        /// O input.
        input: InputFrame,
    },
}

/// Algo notável que aconteceu durante um tick (para log/telemetria).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SimEvent {
    /// O jogador caiu abaixo do mundo e voltou ao spawn.
    FellIntoVoid(PlayerId),
}

/// Estado público de um jogador num tick.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlayerSnapshot {
    /// Jogador.
    pub id: PlayerId,
    /// Posição do olho.
    pub position: [f32; 3],
    /// Radianos, em torno de Y.
    pub yaw: f32,
    /// Radianos, olhar pra cima/baixo.
    pub pitch: f32,
    /// Voo livre (debug) ativo.
    pub flying: bool,
    /// Último `seq` de input aplicado.
    pub last_input_seq: u32,
}

/// Estado de todos os jogadores após um tick.
#[derive(Clone, Debug, PartialEq)]
pub struct Snapshot {
    /// Número do tick.
    pub tick: u64,
    /// Jogadores, em ordem não especificada.
    pub players: Vec<PlayerSnapshot>,
}

struct Player {
    body: Body,
    input: InputFrame,
}

/// A simulação autoritativa do mundo.
pub struct Simulation {
    /// Única cópia dos chunks; quem precisa enviá-los compartilha o `Arc`.
    /// Chunks ausentes (vazios ou fora do mundo gerado) contam como ar.
    chunks: Arc<ChunkMap>,
    spawn: [f32; 3],
    players: HashMap<PlayerId, Player>,
    tick: u64,
    config: SimConfig,
}

impl Simulation {
    /// Gera o mundo com `generator` e começa sem jogadores.
    pub fn new(generator: &impl WorldGenerator, config: SimConfig) -> Self {
        let (r, h) = (config.world_radius_chunks, config.world_height_chunks);
        let mut chunks = ChunkMap::new();
        for x in -r..r {
            for z in -r..r {
                for y in 0..h {
                    let chunk = generator.generate_chunk(ChunkPos::new(x, y, z));
                    if !chunk.is_empty() {
                        chunks.insert(chunk);
                    }
                }
            }
        }
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
            config,
        }
    }

    /// Chunks do mundo (compartilhados, imutáveis).
    pub fn chunks(&self) -> &Arc<ChunkMap> {
        &self.chunks
    }

    /// Onde jogadores nascem (e renascem ao cair no vazio).
    pub fn spawn(&self) -> [f32; 3] {
        self.spawn
    }

    /// Se há algum jogador no mundo.
    pub fn has_players(&self) -> bool {
        !self.players.is_empty()
    }

    /// Aplica um comando; vale a partir do próximo tick.
    pub fn apply(&mut self, command: Command) {
        match command {
            Command::Join { id } => {
                self.players.insert(
                    id,
                    Player {
                        body: Body::at(self.spawn),
                        input: InputFrame::default(),
                    },
                );
            }
            Command::Leave { id } => {
                self.players.remove(&id);
            }
            Command::Input { id, input } => {
                if let Some(p) = self.players.get_mut(&id) {
                    // Ignora pacotes fora de ordem.
                    if input.seq >= p.input.seq {
                        p.input = input;
                    }
                }
            }
        }
    }

    /// Avança um tick de `TICK_DT`.
    pub fn tick(&mut self) -> Vec<SimEvent> {
        self.tick += 1;
        let mut events = Vec::new();
        for (&id, p) in self.players.iter_mut() {
            // O cliente só pede o voo; quem decide é a configuração.
            let mut intent = p.input.intent;
            intent.fly &= self.config.allow_fly;
            p.body.step(&intent, self.chunks.as_ref(), TICK_DT);
            if p.body.position[1] < VOID_Y {
                events.push(SimEvent::FellIntoVoid(id));
                p.body = Body::at(self.spawn);
            }
        }
        events
    }

    /// Estado atual de todos os jogadores.
    pub fn snapshot(&self) -> Snapshot {
        Snapshot {
            tick: self.tick,
            players: self
                .players
                .iter()
                .map(|(&id, p)| PlayerSnapshot {
                    id,
                    position: p.body.position,
                    yaw: p.input.intent.yaw,
                    pitch: p.input.pitch,
                    flying: p.body.flying,
                    last_input_seq: p.input.seq,
                })
                .collect(),
        }
    }
}
