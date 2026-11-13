//! Chunk meshing com culling de faces ocultas: só gera faces de blocos sólidos
//! que encostam em ar. Próximo passo de otimização: greedy meshing.
//!
//! Produz geometria pura; converter para o formato do renderizador (ex.:
//! `bevy::Mesh`) é trabalho de quem usa.

use engine_voxel::{Block, Chunk, CHUNK_SIZE_I32};

struct Face {
    normal: [f32; 3],
    /// Vizinho na direção da normal.
    dir: [i32; 3],
    /// Cantos do quad, em ordem anti-horária vista de fora (front face do Bevy).
    corners: [[f32; 3]; 4],
    /// Sombreamento fixo por direção: dá leitura de volume sem custo de sombra.
    shade: f32,
}

#[rustfmt::skip]
const FACES: [Face; 6] = [
    Face { normal: [1.0, 0.0, 0.0], dir: [1, 0, 0], corners: [[1.0, 0.0, 0.0], [1.0, 1.0, 0.0], [1.0, 1.0, 1.0], [1.0, 0.0, 1.0]], shade: 0.8 },
    Face { normal: [-1.0, 0.0, 0.0], dir: [-1, 0, 0], corners: [[0.0, 0.0, 1.0], [0.0, 1.0, 1.0], [0.0, 1.0, 0.0], [0.0, 0.0, 0.0]], shade: 0.8 },
    Face { normal: [0.0, 1.0, 0.0], dir: [0, 1, 0], corners: [[0.0, 1.0, 0.0], [0.0, 1.0, 1.0], [1.0, 1.0, 1.0], [1.0, 1.0, 0.0]], shade: 1.0 },
    Face { normal: [0.0, -1.0, 0.0], dir: [0, -1, 0], corners: [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [1.0, 0.0, 1.0], [0.0, 0.0, 1.0]], shade: 0.5 },
    Face { normal: [0.0, 0.0, 1.0], dir: [0, 0, 1], corners: [[0.0, 0.0, 1.0], [1.0, 0.0, 1.0], [1.0, 1.0, 1.0], [0.0, 1.0, 1.0]], shade: 0.7 },
    Face { normal: [0.0, 0.0, -1.0], dir: [0, 0, -1], corners: [[1.0, 0.0, 0.0], [0.0, 0.0, 0.0], [0.0, 1.0, 0.0], [1.0, 1.0, 0.0]], shade: 0.7 },
];

fn block_color(block: Block) -> [f32; 3] {
    match block {
        Block::Grass => [0.30, 0.62, 0.22],
        Block::Dirt => [0.47, 0.33, 0.20],
        Block::Stone => [0.50, 0.50, 0.52],
        Block::Air => [0.0, 0.0, 0.0],
    }
}

/// Malha de um chunk em coordenadas locais (0..16), pronta para virar uma
/// lista de triângulos.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ChunkGeometry {
    /// Posição de cada vértice.
    pub positions: Vec<[f32; 3]>,
    /// Normal de cada vértice.
    pub normals: Vec<[f32; 3]>,
    /// Cor RGBA linear de cada vértice (já com o sombreamento da face).
    pub colors: Vec<[f32; 4]>,
    /// Triângulos, três índices cada, em ordem anti-horária vista de fora.
    pub indices: Vec<u32>,
}

/// Gera a malha de um chunk. `neighbor` resolve blocos fora do chunk
/// (coordenadas locais fora de 0..16) para não desenhar faces entre chunks.
/// Retorna `None` se o chunk não tem nenhuma face visível.
pub fn build_chunk_geometry(
    chunk: &Chunk,
    neighbor: impl Fn(i32, i32, i32) -> Block,
) -> Option<ChunkGeometry> {
    let mut positions: Vec<[f32; 3]> = Vec::new();
    let mut normals: Vec<[f32; 3]> = Vec::new();
    let mut colors: Vec<[f32; 4]> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();

    let solid_at = |x: i32, y: i32, z: i32| -> bool {
        chunk
            .get(x, y, z)
            .unwrap_or_else(|| neighbor(x, y, z))
            .is_solid()
    };

    for y in 0..CHUNK_SIZE_I32 {
        for z in 0..CHUNK_SIZE_I32 {
            for x in 0..CHUNK_SIZE_I32 {
                let block = chunk.get(x, y, z).unwrap_or_default();
                if !block.is_solid() {
                    continue;
                }
                let base = block_color(block);
                for face in &FACES {
                    if solid_at(x + face.dir[0], y + face.dir[1], z + face.dir[2]) {
                        continue;
                    }
                    let start = positions.len() as u32;
                    let color = [
                        base[0] * face.shade,
                        base[1] * face.shade,
                        base[2] * face.shade,
                        1.0,
                    ];
                    for c in face.corners {
                        positions.push([x as f32 + c[0], y as f32 + c[1], z as f32 + c[2]]);
                        normals.push(face.normal);
                        colors.push(color);
                    }
                    indices.extend_from_slice(&[
                        start,
                        start + 1,
                        start + 2,
                        start,
                        start + 2,
                        start + 3,
                    ]);
                }
            }
        }
    }

    if positions.is_empty() {
        return None;
    }

    Some(ChunkGeometry {
        positions,
        normals,
        colors,
        indices,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use engine_voxel::ChunkPos;

    fn chunk_with(blocks: &[(usize, usize, usize)]) -> Chunk {
        let mut c = Chunk::empty(ChunkPos::new(0, 0, 0));
        for &(x, y, z) in blocks {
            c.set(x, y, z, Block::Stone);
        }
        c
    }

    #[test]
    fn empty_chunk_has_no_geometry() {
        assert_eq!(
            build_chunk_geometry(&chunk_with(&[]), |_, _, _| Block::Air),
            None
        );
    }

    #[test]
    fn single_block_has_six_faces() {
        let g = build_chunk_geometry(&chunk_with(&[(3, 4, 5)]), |_, _, _| Block::Air).unwrap();
        assert_eq!(g.positions.len(), 24);
        assert_eq!(g.normals.len(), 24);
        assert_eq!(g.colors.len(), 24);
        assert_eq!(g.indices.len(), 36);
        assert!(g.positions.iter().all(|p| (3.0..=4.0).contains(&p[0])));
    }

    #[test]
    fn touching_blocks_hide_shared_faces() {
        let g = build_chunk_geometry(&chunk_with(&[(0, 0, 0), (1, 0, 0)]), |_, _, _| Block::Air)
            .unwrap();
        assert_eq!(g.indices.len(), 10 * 6);
    }

    #[test]
    fn neighbor_chunk_hides_border_faces() {
        // Bloco na borda x = 0; vizinho sólido em x = -1 esconde a face -X.
        let g = build_chunk_geometry(&chunk_with(&[(0, 0, 0)]), |x, _, _| {
            if x < 0 {
                Block::Stone
            } else {
                Block::Air
            }
        })
        .unwrap();
        assert_eq!(g.indices.len(), 5 * 6);
    }
}
