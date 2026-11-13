use thiserror::Error;

use crate::Block;

/// Lado do chunk, em blocos.
pub const CHUNK_SIZE: usize = 16;
/// `CHUNK_SIZE` como `i32`, para aritmética com coordenadas de mundo.
pub const CHUNK_SIZE_I32: i32 = CHUNK_SIZE as i32;
/// Quantidade de blocos num chunk.
pub const CHUNK_VOLUME: usize = CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE;

/// Coordenada de chunk (em unidades de chunk, não de bloco).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ChunkPos {
    /// Eixo X, em chunks.
    pub x: i32,
    /// Eixo Y, em chunks.
    pub y: i32,
    /// Eixo Z, em chunks.
    pub z: i32,
}

impl ChunkPos {
    /// Cria a partir das coordenadas em chunks.
    pub const fn new(x: i32, y: i32, z: i32) -> Self {
        Self { x, y, z }
    }

    /// Chunk que contém o bloco em coordenadas de mundo.
    pub fn from_world_block(x: i32, y: i32, z: i32) -> Self {
        Self::new(
            x.div_euclid(CHUNK_SIZE_I32),
            y.div_euclid(CHUNK_SIZE_I32),
            z.div_euclid(CHUNK_SIZE_I32),
        )
    }

    /// Origem do chunk em coordenadas de mundo (bloco).
    pub fn world_origin(self) -> [i32; 3] {
        [
            self.x * CHUNK_SIZE_I32,
            self.y * CHUNK_SIZE_I32,
            self.z * CHUNK_SIZE_I32,
        ]
    }

    /// Os seis chunks que encostam neste por uma face.
    pub fn neighbors(self) -> [ChunkPos; 6] {
        let ChunkPos { x, y, z } = self;
        [
            ChunkPos::new(x + 1, y, z),
            ChunkPos::new(x - 1, y, z),
            ChunkPos::new(x, y + 1, z),
            ChunkPos::new(x, y - 1, z),
            ChunkPos::new(x, y, z + 1),
            ChunkPos::new(x, y, z - 1),
        ]
    }
}

/// Erro ao montar um chunk a partir de bytes externos.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum ChunkError {
    /// O buffer não tem exatamente `CHUNK_VOLUME` bytes.
    #[error("chunk em {pos:?} com {len} bytes (esperado {CHUNK_VOLUME})")]
    WrongSize {
        /// Posição declarada do chunk.
        pos: ChunkPos,
        /// Tamanho recebido.
        len: usize,
    },
}

/// 16x16x16 blocos, um byte por bloco. Índice = x + z*16 + y*256.
///
/// Invariante: o buffer sempre tem `CHUNK_VOLUME` bytes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Chunk {
    pos: ChunkPos,
    blocks: Vec<u8>,
}

impl Chunk {
    /// Chunk só de ar.
    pub fn empty(pos: ChunkPos) -> Self {
        Self {
            pos,
            blocks: vec![0; CHUNK_VOLUME],
        }
    }

    /// Monta a partir de bytes crus (ex.: vindos da rede), validando o tamanho.
    pub fn from_bytes(pos: ChunkPos, blocks: Vec<u8>) -> Result<Self, ChunkError> {
        if blocks.len() == CHUNK_VOLUME {
            Ok(Self { pos, blocks })
        } else {
            Err(ChunkError::WrongSize {
                pos,
                len: blocks.len(),
            })
        }
    }

    /// Bytes crus, na ordem de `index`.
    pub fn as_bytes(&self) -> &[u8] {
        &self.blocks
    }

    /// Posição do chunk.
    pub fn pos(&self) -> ChunkPos {
        self.pos
    }

    /// Índice no buffer para coordenadas locais (0..16).
    #[inline]
    pub fn index(x: usize, y: usize, z: usize) -> usize {
        x + z * CHUNK_SIZE + y * CHUNK_SIZE * CHUNK_SIZE
    }

    /// Coordenadas locais (0..16). Retorna `None` fora do chunk.
    #[inline]
    pub fn get(&self, x: i32, y: i32, z: i32) -> Option<Block> {
        let r = 0..CHUNK_SIZE_I32;
        if r.contains(&x) && r.contains(&y) && r.contains(&z) {
            Some(Block::from_u8(
                self.blocks[Self::index(x as usize, y as usize, z as usize)],
            ))
        } else {
            None
        }
    }

    /// Troca o bloco em coordenadas locais (0..16).
    #[inline]
    pub fn set(&mut self, x: usize, y: usize, z: usize, block: Block) {
        self.blocks[Self::index(x, y, z)] = block as u8;
    }

    /// Se o chunk é só ar.
    pub fn is_empty(&self) -> bool {
        self.blocks.iter().all(|&b| b == 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_bytes_validates_size() {
        let pos = ChunkPos::new(0, 0, 0);
        assert!(Chunk::from_bytes(pos, vec![0; CHUNK_VOLUME]).is_ok());
        assert_eq!(
            Chunk::from_bytes(pos, vec![0; 10]),
            Err(ChunkError::WrongSize { pos, len: 10 })
        );
    }

    #[test]
    fn negative_world_coords_map_to_chunk() {
        assert_eq!(
            ChunkPos::from_world_block(-1, 0, 16),
            ChunkPos::new(-1, 0, 1)
        );
    }
}
