//! Chunk meshing com culling de faces ocultas: só gera faces de blocos sólidos
//! que encostam em ar. Próximo passo de otimização: greedy meshing.

use bevy::prelude::*;
use bevy::render::mesh::{Indices, Mesh};
use bevy::render::render_asset::RenderAssetUsages;
use bevy::render::render_resource::PrimitiveTopology;
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

const FACES: [Face; 6] = [
    Face {
        normal: [1.0, 0.0, 0.0],
        dir: [1, 0, 0],
        corners: [
            [1.0, 0.0, 0.0],
            [1.0, 1.0, 0.0],
            [1.0, 1.0, 1.0],
            [1.0, 0.0, 1.0],
        ],
        shade: 0.8,
    },
    Face {
        normal: [-1.0, 0.0, 0.0],
        dir: [-1, 0, 0],
        corners: [
            [0.0, 0.0, 1.0],
            [0.0, 1.0, 1.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0],
        ],
        shade: 0.8,
    },
    Face {
        normal: [0.0, 1.0, 0.0],
        dir: [0, 1, 0],
        corners: [
            [0.0, 1.0, 0.0],
            [0.0, 1.0, 1.0],
            [1.0, 1.0, 1.0],
            [1.0, 1.0, 0.0],
        ],
        shade: 1.0,
    },
    Face {
        normal: [0.0, -1.0, 0.0],
        dir: [0, -1, 0],
        corners: [
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [1.0, 0.0, 1.0],
            [0.0, 0.0, 1.0],
        ],
        shade: 0.5,
    },
    Face {
        normal: [0.0, 0.0, 1.0],
        dir: [0, 0, 1],
        corners: [
            [0.0, 0.0, 1.0],
            [1.0, 0.0, 1.0],
            [1.0, 1.0, 1.0],
            [0.0, 1.0, 1.0],
        ],
        shade: 0.7,
    },
    Face {
        normal: [0.0, 0.0, -1.0],
        dir: [0, 0, -1],
        corners: [
            [1.0, 0.0, 0.0],
            [0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [1.0, 1.0, 0.0],
        ],
        shade: 0.7,
    },
];

fn block_color(block: Block) -> [f32; 3] {
    match block {
        Block::Grass => [0.30, 0.62, 0.22],
        Block::Dirt => [0.47, 0.33, 0.20],
        Block::Stone => [0.50, 0.50, 0.52],
        Block::Air => [0.0, 0.0, 0.0],
    }
}

/// Gera a malha de um chunk. `neighbor` resolve blocos fora do chunk
/// (coordenadas locais fora de 0..16) para não desenhar faces entre chunks.
/// Retorna `None` se o chunk não tem nenhuma face visível.
pub fn build_chunk_mesh(chunk: &Chunk, neighbor: impl Fn(i32, i32, i32) -> Block) -> Option<Mesh> {
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

    // RENDER_WORLD: a cópia na CPU é descartada após o upload (economiza RAM).
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    mesh.insert_indices(Indices::U32(indices));
    Some(mesh)
}
