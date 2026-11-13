use std::collections::HashMap;

use crate::{Block, Chunk, ChunkPos, CHUNK_SIZE_I32};

/// O que a física precisa saber do mundo. Costura para testar a física com
/// mundos sintéticos, sem gerar chunks.
pub trait VoxelQuery {
    /// Se o bloco em coordenadas de mundo é sólido.
    fn is_solid(&self, x: i32, y: i32, z: i32) -> bool;
}

/// Chunks indexados por posição. Chunks ausentes contam como ar.
///
/// Guarda em ordem de inserção (iteração determinística, ex.: ordem de envio
/// para os clientes) com um índice por posição para busca O(1).
#[derive(Clone, Debug, Default)]
pub struct ChunkMap {
    chunks: Vec<Chunk>,
    index: HashMap<ChunkPos, usize>,
}

impl ChunkMap {
    /// Mapa vazio.
    pub fn new() -> Self {
        Self::default()
    }

    /// Insere ou substitui o chunk na posição dele.
    pub fn insert(&mut self, chunk: Chunk) {
        match self.index.get(&chunk.pos()) {
            Some(&i) => self.chunks[i] = chunk,
            None => {
                self.index.insert(chunk.pos(), self.chunks.len());
                self.chunks.push(chunk);
            }
        }
    }

    /// Chunk na posição, se carregado.
    pub fn get(&self, pos: ChunkPos) -> Option<&Chunk> {
        self.index.get(&pos).map(|&i| &self.chunks[i])
    }

    /// Se há chunk carregado na posição.
    pub fn contains(&self, pos: ChunkPos) -> bool {
        self.index.contains_key(&pos)
    }

    /// Chunks em ordem de inserção.
    pub fn iter(&self) -> impl Iterator<Item = &Chunk> {
        self.chunks.iter()
    }

    /// Quantidade de chunks.
    pub fn len(&self) -> usize {
        self.chunks.len()
    }

    /// Se não há nenhum chunk.
    pub fn is_empty(&self) -> bool {
        self.chunks.is_empty()
    }

    /// Bloco em coordenadas de mundo. Chunks não carregados contam como ar.
    pub fn block(&self, x: i32, y: i32, z: i32) -> Block {
        self.get(ChunkPos::from_world_block(x, y, z))
            .and_then(|c| {
                c.get(
                    x.rem_euclid(CHUNK_SIZE_I32),
                    y.rem_euclid(CHUNK_SIZE_I32),
                    z.rem_euclid(CHUNK_SIZE_I32),
                )
            })
            .unwrap_or_default()
    }
}

impl FromIterator<Chunk> for ChunkMap {
    fn from_iter<I: IntoIterator<Item = Chunk>>(iter: I) -> Self {
        let mut map = Self::new();
        iter.into_iter().for_each(|c| map.insert(c));
        map
    }
}

impl VoxelQuery for ChunkMap {
    fn is_solid(&self, x: i32, y: i32, z: i32) -> bool {
        self.block(x, y, z).is_solid()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn block_lookup_across_chunks() {
        let mut c = Chunk::empty(ChunkPos::new(-1, 0, 0));
        c.set(15, 3, 0, Block::Stone);
        let map: ChunkMap = [c].into_iter().collect();
        assert_eq!(map.block(-1, 3, 0), Block::Stone);
        assert!(map.is_solid(-1, 3, 0));
        assert_eq!(map.block(0, 3, 0), Block::Air); // chunk ausente
    }

    #[test]
    fn insert_replaces_and_keeps_order() {
        let a = ChunkPos::new(0, 0, 0);
        let b = ChunkPos::new(1, 0, 0);
        let mut map: ChunkMap = [Chunk::empty(a), Chunk::empty(b)].into_iter().collect();
        let mut new_a = Chunk::empty(a);
        new_a.set(0, 0, 0, Block::Dirt);
        map.insert(new_a);
        assert_eq!(map.len(), 2);
        assert_eq!(map.iter().map(Chunk::pos).collect::<Vec<_>>(), [a, b]);
        assert_eq!(map.block(0, 0, 0), Block::Dirt);
    }
}
