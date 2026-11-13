use crate::{Block, Chunk, ChunkPos, CHUNK_SIZE};

/// Gera o conteúdo do mundo. Injetado na simulação para que ela não dependa
/// de um gerador específico (e para testar com mundos sintéticos).
pub trait WorldGenerator {
    /// Conteúdo do chunk na posição.
    fn generate_chunk(&self, pos: ChunkPos) -> Chunk;
    /// Y do bloco mais alto da coluna (x, z), em coordenadas de mundo.
    fn surface_height(&self, x: i32, z: i32) -> i32;
}

/// Terreno por heightmap de senos: grama no topo, 2 de terra, pedra abaixo.
/// Determinístico, sem dependências. Placeholder até entrar ruído de verdade.
#[derive(Clone, Copy, Debug, Default)]
pub struct HeightmapGenerator;

impl WorldGenerator for HeightmapGenerator {
    fn surface_height(&self, wx: i32, wz: i32) -> i32 {
        let (x, z) = (wx as f32, wz as f32);
        let h =
            12.0 + 4.0 * (x * 0.08).sin() + 3.0 * (z * 0.11).cos() + 2.0 * ((x + z) * 0.05).sin();
        h.round() as i32
    }

    fn generate_chunk(&self, pos: ChunkPos) -> Chunk {
        let mut chunk = Chunk::empty(pos);
        let [ox, oy, oz] = pos.world_origin();
        for z in 0..CHUNK_SIZE {
            for x in 0..CHUNK_SIZE {
                let height = self.surface_height(ox + x as i32, oz + z as i32);
                for y in 0..CHUNK_SIZE {
                    let wy = oy + y as i32;
                    let block = if wy > height {
                        Block::Air
                    } else if wy == height {
                        Block::Grass
                    } else if wy > height - 3 {
                        Block::Dirt
                    } else {
                        Block::Stone
                    };
                    if block != Block::Air {
                        chunk.set(x, y, z, block);
                    }
                }
            }
        }
        chunk
    }
}
