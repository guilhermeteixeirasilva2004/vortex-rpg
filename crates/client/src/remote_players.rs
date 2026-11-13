//! Desenha os outros jogadores como caixas, na posição do último snapshot.

use std::collections::HashMap;

use bevy::prelude::*;
use engine_physics::{EYE_HEIGHT, PLAYER_HEIGHT, PLAYER_WIDTH};
use game_protocol::PlayerId;

use crate::network::{LatestSnapshot, LocalPlayer};

/// Centro do corpo abaixo do olho: o corpo vai do pé até `PLAYER_HEIGHT`.
const BODY_CENTER_BELOW_EYE: f32 = EYE_HEIGHT - PLAYER_HEIGHT / 2.0;

/// Cria, move e remove as entidades dos jogadores remotos.
pub struct RemotePlayersPlugin;

impl Plugin for RemotePlayersPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<RemotePlayers>()
            .add_systems(Update, sync_remote_players);
    }
}

#[derive(Component)]
struct RemotePlayer;

#[derive(Resource, Default)]
struct RemotePlayers(HashMap<PlayerId, Entity>);

fn sync_remote_players(
    mut commands: Commands,
    snapshot: Res<LatestSnapshot>,
    local: Res<LocalPlayer>,
    mut remotes: ResMut<RemotePlayers>,
    mut remote_q: Query<&mut Transform, With<RemotePlayer>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    if !snapshot.is_changed() {
        return;
    }

    let mut seen = Vec::with_capacity(snapshot.players.len());
    for p in &snapshot.players {
        if Some(p.id) == local.id {
            continue;
        }
        seen.push(p.id);
        let rotation = Quat::from_rotation_y(p.yaw);
        let body = Vec3::from_array(p.position) - Vec3::Y * BODY_CENTER_BELOW_EYE;
        match remotes.0.get(&p.id).and_then(|&e| remote_q.get_mut(e).ok()) {
            Some(mut t) => {
                t.translation = body;
                t.rotation = rotation;
            }
            None => {
                let e = commands
                    .spawn((
                        RemotePlayer,
                        Mesh3d(meshes.add(Cuboid::new(PLAYER_WIDTH, PLAYER_HEIGHT, PLAYER_WIDTH))),
                        MeshMaterial3d(materials.add(Color::srgb(0.85, 0.3, 0.3))),
                        Transform::from_translation(body).with_rotation(rotation),
                    ))
                    .id();
                remotes.0.insert(p.id, e);
            }
        }
    }

    remotes.0.retain(|id, e| {
        let keep = seen.contains(id);
        if !keep {
            commands.entity(*e).despawn();
        }
        keep
    });
}
