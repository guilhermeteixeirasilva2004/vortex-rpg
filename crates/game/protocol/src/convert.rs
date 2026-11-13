//! Conversões entre DTOs e tipos de domínio.

use engine_physics::MoveIntent;
use engine_voxel::{Chunk, ChunkError, ChunkPos};
use game_sim::{CreatureSnapshot, InputFrame, PlayerSnapshot, Snapshot};

use crate::{ChunkDto, ChunkPosDto, CreatureState, PlayerInput, PlayerState, ServerMessage};

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

impl From<PlayerInput> for InputFrame {
    /// Sanitiza e separa o eixo `up` do fio em pulo (andando) e
    /// subir/descer (voando). `fly` continua sendo só um pedido.
    fn from(input: PlayerInput) -> Self {
        let input = input.sanitized();
        InputFrame {
            seq: input.seq,
            intent: MoveIntent {
                forward: input.forward,
                right: input.right,
                yaw: input.yaw,
                jump: input.up > 0.5,
                fly: input.fly,
                fly_vertical: input.up,
            },
            pitch: input.pitch,
        }
    }
}

impl From<&PlayerSnapshot> for PlayerState {
    fn from(p: &PlayerSnapshot) -> Self {
        PlayerState {
            id: p.id,
            position: p.position,
            yaw: p.yaw,
            pitch: p.pitch,
            flying: p.flying,
            last_input_seq: p.last_input_seq,
        }
    }
}

impl From<&CreatureSnapshot> for CreatureState {
    fn from(c: &CreatureSnapshot) -> Self {
        CreatureState {
            id: c.id,
            kind: c.kind.to_owned(),
            feet: c.feet,
            yaw: c.yaw,
            width: c.width,
            height: c.height,
        }
    }
}

impl From<&Snapshot> for ServerMessage {
    fn from(s: &Snapshot) -> Self {
        ServerMessage::Snapshot {
            tick: s.tick,
            players: s.players.iter().map(PlayerState::from).collect(),
            creatures: s.creatures.iter().map(CreatureState::from).collect(),
        }
    }
}

impl From<&Chunk> for ChunkDto {
    fn from(chunk: &Chunk) -> Self {
        let ChunkPos { x, y, z } = chunk.pos();
        Self {
            pos: ChunkPosDto { x, y, z },
            blocks: chunk.as_bytes().to_vec(),
        }
    }
}

impl TryFrom<ChunkDto> for Chunk {
    type Error = ChunkError;

    /// Valida o tamanho do buffer (dados de rede não são confiáveis).
    fn try_from(dto: ChunkDto) -> Result<Self, ChunkError> {
        let ChunkPosDto { x, y, z } = dto.pos;
        Chunk::from_bytes(ChunkPos::new(x, y, z), dto.blocks)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn up_axis_splits_into_jump_and_fly_vertical() {
        let walk = |up| {
            InputFrame::from(PlayerInput {
                up,
                ..Default::default()
            })
            .intent
        };
        assert!(walk(1.0).jump);
        assert!(!walk(0.5).jump);
        assert!(!walk(-1.0).jump);
        assert_eq!(walk(-1.0).fly_vertical, -1.0);
        assert_eq!(walk(0.3).fly_vertical, 0.3);
    }

    #[test]
    fn input_is_sanitized_at_the_edge() {
        let frame = InputFrame::from(PlayerInput {
            seq: 9,
            forward: 5.0,
            right: f32::NAN,
            up: f32::INFINITY,
            fly: true,
            yaw: -1.0,
            pitch: 3.0,
        });
        assert_eq!(frame.seq, 9);
        assert_eq!(frame.intent.forward, 1.0);
        assert_eq!(frame.intent.right, 0.0);
        assert_eq!(frame.intent.fly_vertical, 0.0);
        assert!(!frame.intent.jump);
        assert!(frame.intent.fly);
        assert_eq!(
            frame.intent.yaw,
            (-1.0f32).rem_euclid(std::f32::consts::TAU)
        );
        assert_eq!(frame.pitch, 1.55);
    }
}
