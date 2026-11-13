//! Contrato de rede entre cliente e servidor.
//!
//! Os tipos daqui (DTOs) definem o formato no fio e só existem na borda: o
//! domínio (`engine-*`, `game-sim`) não conhece serde. As conversões
//! DTO <-> domínio ficam neste crate, e é aqui que o input do cliente é
//! validado antes de entrar na simulação.

mod codec;
mod convert;
mod message;

pub use codec::{decode, encode, ProtocolError};
pub use message::{
    ChunkDto, ChunkPosDto, ClientMessage, CreatureState, PlayerInput, PlayerState, ServerMessage,
};

/// Taxa de tick do servidor; o cliente envia input na mesma taxa.
pub use game_sim::TICK_RATE;
pub use game_sim::{CreatureId, PlayerId};
