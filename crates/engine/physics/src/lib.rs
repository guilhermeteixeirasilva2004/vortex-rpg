//! Física de corpos (jogador e criaturas). Pura e determinística: o servidor
//! roda para decidir a posição e, no futuro, o cliente pode rodar a mesma
//! coisa para predição.
//!
//! Convenção igual à do Bevy: câmera olha para -Z, yaw gira em torno de +Y.
//! `position` é sempre a posição do olho; o corpo é uma AABB de
//! `width` x `height` (ver `BodySpec`) cujo pé fica em
//! `position.y - eye_height`.

mod body;
mod intent;
mod spec;

pub use body::Body;
pub use intent::MoveIntent;
pub use spec::BodySpec;

/// Velocidade horizontal do jogador andando (blocos/s).
pub const MOVE_SPEED: f32 = 5.0;
/// Velocidade no modo voo (debug), em blocos/s.
pub const FLY_SPEED: f32 = 8.0;
/// Altura do olho do jogador acima do pé (blocos).
pub const EYE_HEIGHT: f32 = 1.7;
/// Largura (X e Z) da AABB do jogador (blocos).
pub const PLAYER_WIDTH: f32 = 0.6;
/// Altura da AABB do jogador (blocos).
pub const PLAYER_HEIGHT: f32 = 1.8;
/// Aceleração da gravidade (blocos/s²).
pub const GRAVITY: f32 = 28.0;
/// Velocidade vertical do pulo: sqrt(2 * g * h) para h ≈ 1.25 bloco.
pub const JUMP_VELOCITY: f32 = 8.4;
/// Velocidade máxima de queda (blocos/s).
pub const TERMINAL_VELOCITY: f32 = 50.0;
