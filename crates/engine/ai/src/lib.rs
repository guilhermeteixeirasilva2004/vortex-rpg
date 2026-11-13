//! Comportamento de entidades controladas pelo jogo.
//!
//! Só decide para onde andar: não conhece física, mundo nem conteúdo. Quem
//! chama transforma a direção em movimento. Determinístico: toda
//! aleatoriedade sai de um `Rng` criado com a seed recebida.

mod rng;
mod wander;

pub use rng::Rng;
pub use wander::{Wander, WanderParams, WanderState};
