//! Mundo voxel: blocos, chunks 16³, armazenamento por posição e geração.
//!
//! Núcleo da engine: não conhece rede, serialização nem framework.

mod block;
mod chunk;
mod generator;
mod map;

pub use block::Block;
pub use chunk::{Chunk, ChunkError, ChunkPos, CHUNK_SIZE, CHUNK_SIZE_I32, CHUNK_VOLUME};
pub use generator::{HeightmapGenerator, WorldGenerator};
pub use map::{ChunkMap, VoxelQuery};
