//! Mensagens trocadas via WebSocket.
//!
//! Por enquanto em JSON (fácil de depurar). Trocar por um formato binário
//! (bincode/postcard) quando o volume de chunks começar a pesar.

use serde::{Deserialize, Serialize};

use crate::voxel::Chunk;

pub type PlayerId = u64;

/// Estado de input "segurado" no momento. O cliente não manda posição:
/// o servidor aplica o input e decide onde o jogador está.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub struct PlayerInput {
    pub seq: u32,
    /// -1..1 (S..W)
    pub forward: f32,
    /// -1..1 (A..D)
    pub right: f32,
    /// -1..1 (Shift..Space)
    pub up: f32,
    /// Radianos, rotação em torno de Y.
    pub yaw: f32,
    /// Radianos, olhar pra cima/baixo.
    pub pitch: f32,
}

impl PlayerInput {
    /// O servidor nunca confia no cliente: limita eixos e ângulos.
    pub fn sanitized(mut self) -> Self {
        let axis = |v: f32| if v.is_finite() { v.clamp(-1.0, 1.0) } else { 0.0 };
        self.forward = axis(self.forward);
        self.right = axis(self.right);
        self.up = axis(self.up);
        self.yaw = if self.yaw.is_finite() {
            self.yaw.rem_euclid(std::f32::consts::TAU)
        } else {
            0.0
        };
        self.pitch = if self.pitch.is_finite() {
            self.pitch.clamp(-1.55, 1.55)
        } else {
            0.0
        };
        self
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ClientMessage {
    Input(PlayerInput),
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct PlayerState {
    pub id: PlayerId,
    pub position: [f32; 3],
    pub yaw: f32,
    pub pitch: f32,
    /// Último `seq` de input processado (base para predição/reconciliação futura).
    pub last_input_seq: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ServerMessage {
    Welcome {
        player_id: PlayerId,
        spawn: [f32; 3],
    },
    ChunkData(Chunk),
    Snapshot {
        tick: u64,
        players: Vec<PlayerState>,
    },
}

pub fn encode<T: Serialize>(msg: &T) -> Result<String, serde_json::Error> {
    serde_json::to_string(msg)
}

pub fn decode<'a, T: Deserialize<'a>>(text: &'a str) -> Result<T, serde_json::Error> {
    serde_json::from_str(text)
}
