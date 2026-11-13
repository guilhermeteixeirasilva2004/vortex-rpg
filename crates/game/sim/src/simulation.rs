use std::collections::BTreeMap;
use std::sync::Arc;

use engine_ai::{Rng, Wander};
use engine_physics::{Body, EYE_HEIGHT};
use engine_voxel::{ChunkMap, ChunkPos, WorldGenerator};

use crate::content::{CreatureSpawn, CREATURE_SPAWNS};
use crate::entity::{Creature, CreatureId, Entity, EntityId, EntityKind, Player};
use crate::{
    Command, CreatureSnapshot, InputFrame, PlayerSnapshot, SimConfig, SimEvent, Snapshot, TICK_DT,
};

/// Abaixo disso (blocos) a entidade caiu para fora do mundo e volta para
/// onde nasceu.
const VOID_Y: f32 = -64.0;

/// A simulação autoritativa do mundo.
pub struct Simulation {
    /// Única cópia dos chunks; quem precisa enviá-los compartilha o `Arc`.
    /// Chunks ausentes (vazios ou fora do mundo gerado) contam como ar.
    chunks: Arc<ChunkMap>,
    spawn: [f32; 3],
    /// Ordenado por id: o tick e o snapshot percorrem sempre na mesma ordem.
    entities: BTreeMap<EntityId, Entity>,
    tick: u64,
    config: SimConfig,
}

impl Simulation {
    /// Gera o mundo com `generator`, cria as criaturas de `content` e começa
    /// sem jogadores.
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

        // Uma seed por criatura, tirada da seed do mundo em ordem fixa.
        let mut rng = Rng::new(config.seed);
        let entities = (0..)
            .zip(CREATURE_SPAWNS)
            .map(|(id, s)| {
                let creature = spawn_creature(generator, s, rng.next_u64());
                (EntityId::Creature(id), creature)
            })
            .collect();

        Self {
            chunks: Arc::new(chunks),
            spawn,
            entities,
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
        self.entities
            .keys()
            .any(|id| matches!(id, EntityId::Player(_)))
    }

    /// Aplica um comando; vale a partir do próximo tick.
    pub fn apply(&mut self, command: Command) {
        match command {
            Command::Join { id } => {
                self.entities.insert(
                    EntityId::Player(id),
                    Entity {
                        body: Body::at(self.spawn),
                        kind: EntityKind::Player(Player {
                            input: InputFrame::default(),
                        }),
                    },
                );
            }
            Command::Leave { id } => {
                self.entities.remove(&EntityId::Player(id));
            }
            Command::Input { id, input } => {
                if let Some(Entity {
                    kind: EntityKind::Player(p),
                    ..
                }) = self.entities.get_mut(&EntityId::Player(id))
                {
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
        let world = self.chunks.as_ref();
        let mut events = Vec::new();
        for (&id, e) in self.entities.iter_mut() {
            match &mut e.kind {
                EntityKind::Player(p) => {
                    // O cliente só pede o voo; quem decide é a configuração.
                    let mut intent = p.input.intent;
                    intent.fly &= self.config.allow_fly;
                    e.body.step(&intent, world, TICK_DT);
                    if e.body.position[1] < VOID_Y {
                        if let EntityId::Player(pid) = id {
                            events.push(SimEvent::FellIntoVoid(pid));
                        }
                        e.body = Body::at(self.spawn);
                    }
                }
                EntityKind::Creature(c) => {
                    let intent = c.intent(&e.body, world);
                    e.body.step(&intent, world, TICK_DT);
                    if e.body.position[1] < VOID_Y {
                        e.body = Body::with_spec(c.home, c.kind.body);
                    }
                }
            }
        }
        events
    }

    /// Estado atual de todas as entidades.
    pub fn snapshot(&self) -> Snapshot {
        let mut players = Vec::new();
        let mut creatures = Vec::new();
        for (&id, e) in &self.entities {
            match (id, &e.kind) {
                (EntityId::Player(id), EntityKind::Player(p)) => players.push(PlayerSnapshot {
                    id,
                    position: e.body.position,
                    yaw: p.input.intent.yaw,
                    pitch: p.input.pitch,
                    flying: e.body.flying,
                    last_input_seq: p.input.seq,
                }),
                (EntityId::Creature(id), EntityKind::Creature(c)) => {
                    creatures.push(creature_snapshot(id, &e.body, c))
                }
                // O id sempre é criado junto com o tipo certo.
                _ => {}
            }
        }
        Snapshot {
            tick: self.tick,
            players,
            creatures,
        }
    }
}

/// Criatura parada em cima do terreno na coluna do `spawn`.
fn spawn_creature(generator: &impl WorldGenerator, s: &CreatureSpawn, seed: u64) -> Entity {
    let [x, z] = s.offset;
    let body = s.kind.body;
    let home = [
        x as f32 + 0.5,
        generator.surface_height(x, z) as f32 + 1.0 + body.eye_height,
        z as f32 + 0.5,
    ];
    Entity {
        body: Body::with_spec(home, body),
        kind: EntityKind::Creature(Creature {
            kind: s.kind,
            home,
            brain: Wander::new([home[0], home[2]], s.kind.wander, seed),
            yaw: 0.0,
        }),
    }
}

fn creature_snapshot(id: CreatureId, body: &Body, c: &Creature) -> CreatureSnapshot {
    CreatureSnapshot {
        id,
        kind: c.kind.name,
        feet: body.feet(),
        yaw: c.yaw,
        width: body.spec.width,
        height: body.spec.height,
    }
}
