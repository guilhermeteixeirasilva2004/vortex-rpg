/// Tipo de bloco. O valor numérico é o byte guardado no chunk.
#[repr(u8)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Block {
    /// Vazio.
    #[default]
    Air = 0,
    /// Grama (topo do terreno).
    Grass = 1,
    /// Terra (logo abaixo da grama).
    Dirt = 2,
    /// Pedra (resto do subsolo).
    Stone = 3,
}

impl Block {
    /// Converte o byte guardado; valores desconhecidos viram ar.
    pub fn from_u8(v: u8) -> Self {
        match v {
            1 => Block::Grass,
            2 => Block::Dirt,
            3 => Block::Stone,
            _ => Block::Air,
        }
    }

    /// Se o bloco ocupa espaço (colide e é desenhado).
    pub fn is_solid(self) -> bool {
        self != Block::Air
    }
}
