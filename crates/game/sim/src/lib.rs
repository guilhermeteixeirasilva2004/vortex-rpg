//! Núcleo da aplicação: entidades (jogadores e criaturas), comandos (entrar,
//! sair, input), avanço de um tick e snapshot.
//!
//! Puro: não lê relógio, ambiente nem rede. O mundo vem de um
//! `WorldGenerator` injetado e as regras e a seed de `SimConfig`; quem chama
//! decide quando avançar o tick e o que fazer com o snapshot.

mod command;
pub mod content;
mod entity;
mod simulation;
mod snapshot;

pub use command::{Command, InputFrame, SimEvent};
pub use entity::{CreatureId, PlayerId};
pub use simulation::Simulation;
pub use snapshot::{CreatureSnapshot, PlayerSnapshot, Snapshot};

/// Ticks de simulação por segundo.
pub const TICK_RATE: u32 = 20;
/// Duração de um tick, em segundos.
pub const TICK_DT: f32 = 1.0 / TICK_RATE as f32;

/// Regras, tamanho do mundo e seed, decididos por quem monta a simulação.
#[derive(Clone, Copy, Debug)]
pub struct SimConfig {
    /// Mundo gerado vai de `-r..r` chunks em X e Z.
    pub world_radius_chunks: i32,
    /// Mundo gerado vai de `0..h` chunks em Y.
    pub world_height_chunks: i32,
    /// Se o voo livre (debug) pedido pelos jogadores é aceito.
    pub allow_fly: bool,
    /// Origem de toda aleatoriedade (comportamento das criaturas): a mesma
    /// seed com os mesmos comandos dá a mesma partida.
    pub seed: u64,
}
