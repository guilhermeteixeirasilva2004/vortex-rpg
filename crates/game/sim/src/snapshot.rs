use crate::{CreatureId, PlayerId};

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

/// Estado público de uma criatura num tick.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CreatureSnapshot {
    /// Criatura.
    pub id: CreatureId,
    /// Nome do tipo, da tabela de `content`.
    pub kind: &'static str,
    /// Posição do pé (centro da base do corpo).
    pub feet: [f32; 3],
    /// Radianos, em torno de Y: para onde andou por último.
    pub yaw: f32,
    /// Largura do corpo em X e Z (blocos).
    pub width: f32,
    /// Altura do corpo (blocos).
    pub height: f32,
}

/// Estado de todas as entidades após um tick.
#[derive(Clone, Debug, PartialEq)]
pub struct Snapshot {
    /// Número do tick.
    pub tick: u64,
    /// Jogadores, em ordem de id.
    pub players: Vec<PlayerSnapshot>,
    /// Criaturas, em ordem de id.
    pub creatures: Vec<CreatureSnapshot>,
}
