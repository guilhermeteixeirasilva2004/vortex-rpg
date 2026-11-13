//! Entidades: tudo que tem corpo no mundo. O tipo diz quem decide o
//! movimento (input do jogador ou comportamento da criatura).

use engine_ai::Wander;
use engine_physics::{Body, MoveIntent};
use engine_voxel::VoxelQuery;

use crate::content::CreatureType;
use crate::InputFrame;

/// Identificador de jogador, atribuído pela camada de transporte.
pub type PlayerId = u64;
/// Identificador de criatura, atribuído pela simulação.
pub type CreatureId = u32;

/// Identificador de entidade. Jogadores e criaturas têm espaços de id
/// próprios (o do jogador vem de fora), então o tipo faz parte do id.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum EntityId {
    Player(PlayerId),
    Creature(CreatureId),
}

/// Algo com corpo físico no mundo.
pub(crate) struct Entity {
    pub body: Body,
    pub kind: EntityKind,
}

pub(crate) enum EntityKind {
    Player(Player),
    Creature(Creature),
}

pub(crate) struct Player {
    pub input: InputFrame,
}

pub(crate) struct Creature {
    pub kind: &'static CreatureType,
    /// Onde nasceu (olho); volta para cá se cair do mundo.
    pub home: [f32; 3],
    pub brain: Wander,
    /// Para onde andou por último (radianos); mantido enquanto parada.
    pub yaw: f32,
}

impl Creature {
    /// Decide o movimento deste tick: segue o comportamento e pula quando um
    /// degrau bloqueia o caminho.
    pub fn intent(&mut self, body: &Body, world: &impl VoxelQuery) -> MoveIntent {
        let dir = self.brain.update([body.position[0], body.position[2]]);
        let jump = body.on_ground && body.obstacle_ahead(dir, world);
        let intent = MoveIntent::walk_towards(dir, jump);
        if intent.forward != 0.0 {
            self.yaw = intent.yaw;
        }
        intent
    }
}
