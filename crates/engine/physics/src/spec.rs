use crate::{EYE_HEIGHT, MOVE_SPEED, PLAYER_HEIGHT, PLAYER_WIDTH};

/// Forma e velocidade de um corpo. Cada tipo de entidade tem a sua; a
/// física é a mesma para todos.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BodySpec {
    /// Largura da AABB em X e Z (blocos).
    pub width: f32,
    /// Altura da AABB (blocos).
    pub height: f32,
    /// Altura do olho acima do pé (blocos); `Body::position` é o olho.
    pub eye_height: f32,
    /// Velocidade horizontal andando (blocos/s).
    pub walk_speed: f32,
}

impl BodySpec {
    /// O corpo do jogador.
    pub const PLAYER: BodySpec = BodySpec {
        width: PLAYER_WIDTH,
        height: PLAYER_HEIGHT,
        eye_height: EYE_HEIGHT,
        walk_speed: MOVE_SPEED,
    };
}

impl Default for BodySpec {
    fn default() -> Self {
        Self::PLAYER
    }
}
