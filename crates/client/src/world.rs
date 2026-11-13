//! Guarda os chunks recebidos do servidor e mantém as malhas atualizadas.

use std::collections::{HashMap, HashSet};

use bevy::prelude::*;
use bevy::render::mesh::{Indices, Mesh};
use bevy::render::render_asset::RenderAssetUsages;
use bevy::render::render_resource::PrimitiveTopology;
use engine_meshing::{build_chunk_geometry, ChunkGeometry};
use engine_voxel::{ChunkMap, ChunkPos};

use crate::network::IncomingChunks;

/// Máximo de chunks remeshados por frame, para não travar o frame quando
/// chegam muitos de uma vez.
const MESH_BUDGET_PER_FRAME: usize = 8;

/// Chunks recebidos, malhas e iluminação da cena.
pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ClientWorld>()
            .add_systems(Startup, setup_scene)
            .add_systems(Update, (ingest_chunks, remesh_dirty_chunks).chain());
    }
}

#[derive(Resource, Default)]
struct ClientWorld {
    chunks: ChunkMap,
    entities: HashMap<ChunkPos, Entity>,
    dirty: HashSet<ChunkPos>,
}

#[derive(Resource)]
struct VoxelMaterial(Handle<StandardMaterial>);

fn setup_scene(mut commands: Commands, mut materials: ResMut<Assets<StandardMaterial>>) {
    // Material único para todos os chunks: cor vem do vertex color. Um material
    // só = batching e menos troca de estado na GPU.
    let material = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        perceptual_roughness: 1.0,
        reflectance: 0.1,
        ..default()
    });
    commands.insert_resource(VoxelMaterial(material));

    commands.spawn((
        DirectionalLight {
            illuminance: 8_000.0,
            // Sombras são caras em GPU integrada; desligadas por enquanto.
            shadows_enabled: false,
            ..default()
        },
        Transform::from_xyz(0.0, 0.0, 0.0).looking_at(Vec3::new(-0.4, -1.0, -0.3), Vec3::Y),
    ));
    commands.insert_resource(AmbientLight {
        color: Color::WHITE,
        brightness: 400.0,
        ..default()
    });
}

fn ingest_chunks(mut incoming: ResMut<IncomingChunks>, mut world: ResMut<ClientWorld>) {
    if incoming.0.is_empty() {
        return;
    }
    for chunk in incoming.0.drain(..) {
        let pos = chunk.pos();
        world.chunks.insert(chunk);
        world.dirty.insert(pos);
        // Vizinhos já carregados podem ter faces que agora ficaram ocultas.
        for n in pos.neighbors() {
            if world.chunks.contains(n) {
                world.dirty.insert(n);
            }
        }
    }
}

fn remesh_dirty_chunks(
    mut commands: Commands,
    mut world: ResMut<ClientWorld>,
    mut meshes: ResMut<Assets<Mesh>>,
    material: Res<VoxelMaterial>,
) {
    if world.dirty.is_empty() {
        return;
    }
    let batch: Vec<ChunkPos> = world
        .dirty
        .iter()
        .copied()
        .take(MESH_BUDGET_PER_FRAME)
        .collect();

    for pos in batch {
        world.dirty.remove(&pos);
        let Some(chunk) = world.chunks.get(pos) else {
            continue;
        };
        let [ox, oy, oz] = pos.world_origin();
        let mesh =
            build_chunk_geometry(chunk, |x, y, z| world.chunks.block(ox + x, oy + y, oz + z));

        if let Some(old) = world.entities.remove(&pos) {
            commands.entity(old).despawn();
        }
        if let Some(mesh) = mesh.map(to_mesh) {
            let entity = commands
                .spawn((
                    Mesh3d(meshes.add(mesh)),
                    MeshMaterial3d(material.0.clone()),
                    Transform::from_xyz(ox as f32, oy as f32, oz as f32),
                ))
                .id();
            world.entities.insert(pos, entity);
        }
    }
}

/// Converte a geometria pura do chunk para o formato do Bevy.
fn to_mesh(g: ChunkGeometry) -> Mesh {
    // RENDER_WORLD: a cópia na CPU é descartada após o upload (economiza RAM).
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, g.positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, g.normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, g.colors);
    mesh.insert_indices(Indices::U32(g.indices));
    mesh
}
