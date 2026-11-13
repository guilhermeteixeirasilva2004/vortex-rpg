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
    /// -1..1 (Shift..Space). Andando, `up > 0` é pulo; voando, sobe/desce.
    pub up: f32,
    /// Pedido de voo livre (debug, F3). O servidor decide se permite.
    #[serde(default)]
    pub fly: bool,
    /// Radianos, rotação em torno de Y.
    pub yaw: f32,
    /// Radianos, olhar pra cima/baixo.
    pub pitch: f32,
}

impl PlayerInput {
    /// O servidor nunca confia no cliente: limita eixos e ângulos.
    pub fn sanitized(mut self) -> Self {
        let axis = |v: f32| {
            if v.is_finite() {
                v.clamp(-1.0, 1.0)
            } else {
                0.0
            }
        };
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
    /// Voo livre (debug) ativo para este jogador.
    #[serde(default)]
    pub flying: bool,
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

/// Testes "golden": fixam o JSON exato de cada mensagem. Qualquer refatoração
/// precisa manter estes testes passando sem editar as strings esperadas.
#[cfg(test)]
mod golden {
    use super::*;
    use crate::voxel::{Block, ChunkPos};

    fn input() -> PlayerInput {
        PlayerInput {
            seq: 42,
            forward: 1.0,
            right: -0.5,
            up: 1.0,
            fly: true,
            yaw: 1.25,
            pitch: -0.75,
        }
    }

    #[test]
    fn client_input() {
        let json = encode(&ClientMessage::Input(input())).unwrap();
        assert_eq!(
            json,
            r#"{"Input":{"seq":42,"forward":1.0,"right":-0.5,"up":1.0,"fly":true,"yaw":1.25,"pitch":-0.75}}"#
        );
    }

    #[test]
    fn client_input_without_fly_decodes_as_walking() {
        // Clientes antigos não mandam `fly`.
        let json =
            r#"{"Input":{"seq":7,"forward":0.0,"right":0.0,"up":1.0,"yaw":0.0,"pitch":0.0}}"#;
        let ClientMessage::Input(i) = decode::<ClientMessage>(json).unwrap();
        assert_eq!(i.seq, 7);
        assert_eq!(i.up, 1.0);
        assert!(!i.fly);
    }

    #[test]
    fn server_welcome() {
        let json = encode(&ServerMessage::Welcome {
            player_id: 3,
            spawn: [0.5, 17.5, -0.25],
        })
        .unwrap();
        assert_eq!(
            json,
            r#"{"Welcome":{"player_id":3,"spawn":[0.5,17.5,-0.25]}}"#
        );
    }

    #[test]
    fn server_snapshot() {
        let json = encode(&ServerMessage::Snapshot {
            tick: 99,
            players: vec![PlayerState {
                id: 1,
                position: [1.5, 2.0, -3.25],
                yaw: 0.5,
                pitch: 0.25,
                flying: true,
                last_input_seq: 12,
            }],
        })
        .unwrap();
        assert_eq!(
            json,
            r#"{"Snapshot":{"tick":99,"players":[{"id":1,"position":[1.5,2.0,-3.25],"yaw":0.5,"pitch":0.25,"flying":true,"last_input_seq":12}]}}"#
        );
    }

    #[test]
    fn snapshot_without_flying_decodes() {
        let json = r#"{"Snapshot":{"tick":1,"players":[{"id":1,"position":[0.0,0.0,0.0],"yaw":0.0,"pitch":0.0,"last_input_seq":0}]}}"#;
        let ServerMessage::Snapshot { players, .. } = decode::<ServerMessage>(json).unwrap() else {
            panic!("esperava Snapshot");
        };
        assert!(!players[0].flying);
    }

    #[test]
    fn server_chunk_data() {
        let mut chunk = Chunk::empty(ChunkPos::new(1, -2, 3));
        chunk.set(0, 0, 0, Block::Grass);
        chunk.set(1, 0, 0, Block::Dirt);
        chunk.set(0, 0, 1, Block::Stone); // índice 16
        chunk.set(15, 15, 15, Block::Grass); // último índice
        let json = encode(&ServerMessage::ChunkData(chunk)).unwrap();

        let mut blocks = vec!["0"; 4096];
        blocks[0] = "1";
        blocks[1] = "2";
        blocks[16] = "3";
        blocks[4095] = "1";
        let expected = format!(
            r#"{{"ChunkData":{{"pos":{{"x":1,"y":-2,"z":3}},"blocks":[{}]}}}}"#,
            blocks.join(",")
        );
        assert_eq!(json, expected);

        let ServerMessage::ChunkData(back) = decode::<ServerMessage>(&json).unwrap() else {
            panic!("esperava ChunkData");
        };
        assert!(back.is_valid());
        assert_eq!(back.get(0, 0, 1), Some(Block::Stone));
    }
}
