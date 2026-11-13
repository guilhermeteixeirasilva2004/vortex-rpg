//! Simulação de movimento. Fica no crate compartilhado para que, no futuro,
//! o cliente possa rodar a mesma função para predição local.

use crate::protocol::PlayerInput;

pub const MOVE_SPEED: f32 = 8.0;
pub const EYE_HEIGHT: f32 = 1.7;

/// Movimento livre (fly) por enquanto. Gravidade e colisão com voxels entram aqui.
///
/// Convenção igual à do Bevy: câmera olha para -Z, yaw gira em torno de +Y.
pub fn step(position: [f32; 3], input: &PlayerInput, dt: f32) -> [f32; 3] {
    let (sin, cos) = input.yaw.sin_cos();
    let forward = [-sin, 0.0, -cos];
    let right = [cos, 0.0, -sin];

    let mut dir = [
        forward[0] * input.forward + right[0] * input.right,
        input.up,
        forward[2] * input.forward + right[2] * input.right,
    ];
    let len = (dir[0] * dir[0] + dir[1] * dir[1] + dir[2] * dir[2]).sqrt();
    if len > 1.0 {
        dir.iter_mut().for_each(|c| *c /= len);
    }

    [
        position[0] + dir[0] * MOVE_SPEED * dt,
        position[1] + dir[1] * MOVE_SPEED * dt,
        position[2] + dir[2] * MOVE_SPEED * dt,
    ]
}
