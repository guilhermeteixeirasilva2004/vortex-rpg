use serde::{Deserialize, Serialize};

pub const CHUNK_SIZE: usize = 16;
pub const CHUNK_SIZE_I32: i32 = CHUNK_SIZE as i32;
pub const CHUNK_VOLUME: usize = CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE;

#[repr(u8)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Block {
    #[default]
    Air = 0,
    Grass = 1,
    Dirt = 2,
    Stone = 3,
}

impl Block {
    pub fn from_u8(v: u8) -> Self {
        match v {
            1 => Block::Grass,
            2 => Block::Dirt,
            3 => Block::Stone,
            _ => Block::Air,
        }
    }

    pub fn is_solid(self) -> bool {
        self != Block::Air
    }
}

/// Coordenada de chunk (em unidades de chunk, não de bloco).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ChunkPos {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl ChunkPos {
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

/// 16x16x16 blocos, um byte por bloco. Índice = x + z*16 + y*256.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Chunk {
    pub pos: ChunkPos,
    blocks: Vec<u8>,
}

impl Chunk {
    pub fn empty(pos: ChunkPos) -> Self {
        Self {
            pos,
            blocks: vec![0; CHUNK_VOLUME],
        }
    }

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

    #[inline]
    pub fn set(&mut self, x: usize, y: usize, z: usize, block: Block) {
        self.blocks[Self::index(x, y, z)] = block as u8;
    }

    pub fn is_empty(&self) -> bool {
        self.blocks.iter().all(|&b| b == 0)
    }

    /// Valida dados vindos da rede (tamanho do buffer).
    pub fn is_valid(&self) -> bool {
        self.blocks.len() == CHUNK_VOLUME
    }
}

/// Altura do terreno numa coluna (x, z) do mundo. Determinístico, sem dependências.
/// Placeholder até entrar um gerador com ruído de verdade.
pub fn terrain_height(wx: i32, wz: i32) -> i32 {
    let (x, z) = (wx as f32, wz as f32);
    let h = 12.0 + 4.0 * (x * 0.08).sin() + 3.0 * (z * 0.11).cos() + 2.0 * ((x + z) * 0.05).sin();
    h.round() as i32
}

/// Gera o conteúdo de um chunk a partir do heightmap.
pub fn generate_chunk(pos: ChunkPos) -> Chunk {
    let mut chunk = Chunk::empty(pos);
    let [ox, oy, oz] = pos.world_origin();
    for z in 0..CHUNK_SIZE {
        for x in 0..CHUNK_SIZE {
            let height = terrain_height(ox + x as i32, oz + z as i32);
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
