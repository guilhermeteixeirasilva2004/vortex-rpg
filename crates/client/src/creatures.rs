//! Desenha as criaturas como caixas, com cor por tipo e o tamanho que o
//! servidor manda, na posição do último snapshot. Uma caixa menor na frente
//! faz de cabeça, para ver para onde ela está virada.

use std::collections::HashMap;

use bevy::prelude::*;
use game_protocol::{CreatureId, CreatureState};

use crate::network::LatestSnapshot;

/// Lado da cabeça em relação à largura do corpo.
const HEAD_SCALE: f32 = 0.5;
/// Quanto a cor da cabeça escurece em relação à do corpo (0..1).
const HEAD_DARKEN: f32 = 0.3;

/// Cria, move e remove as entidades das criaturas.
pub struct CreaturesPlugin;

impl Plugin for CreaturesPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Creatures>()
            .add_systems(Update, sync_creatures);
    }
}

#[derive(Component)]
struct CreatureMarker;

#[derive(Resource, Default)]
struct Creatures(HashMap<CreatureId, Entity>);

/// Aparência por tipo. Tipo desconhecido (servidor mais novo) fica magenta
/// para chamar atenção, mas aparece.
fn creature_color(kind: &str) -> Color {
    match kind {
        "coelho" => Color::srgb(0.62, 0.45, 0.3),
        "ovelha" => Color::srgb(0.93, 0.93, 0.88),
        _ => Color::srgb(0.85, 0.2, 0.85),
    }
}

/// Centro do corpo e rotação, a partir do pé e do yaw.
fn creature_transform(c: &CreatureState) -> Transform {
    let center = Vec3::from_array(c.feet) + Vec3::Y * (c.height / 2.0);
    Transform::from_translation(center).with_rotation(Quat::from_rotation_y(c.yaw))
}

fn sync_creatures(
    mut commands: Commands,
    snapshot: Res<LatestSnapshot>,
    mut creatures: ResMut<Creatures>,
    mut query: Query<&mut Transform, With<CreatureMarker>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    if !snapshot.is_changed() {
        return;
    }

    for c in &snapshot.creatures {
        let transform = creature_transform(c);
        if let Some(mut t) = creatures.0.get(&c.id).and_then(|&e| query.get_mut(e).ok()) {
            *t = transform;
            continue;
        }
        let color = creature_color(&c.kind);
        let head = c.width * HEAD_SCALE;
        // Frente é -Z, como a câmera; a cabeça encosta no topo da frente.
        let head_offset = Vec3::new(0.0, (c.height - head) / 2.0, -(c.width + head) / 2.0);
        let e = commands
            .spawn((
                CreatureMarker,
                Mesh3d(meshes.add(Cuboid::new(c.width, c.height, c.width))),
                MeshMaterial3d(materials.add(color)),
                transform,
            ))
            .with_child((
                Mesh3d(meshes.add(Cuboid::new(head, head, head))),
                MeshMaterial3d(materials.add(color.darker(HEAD_DARKEN))),
                Transform::from_translation(head_offset),
            ))
            .id();
        creatures.0.insert(c.id, e);
    }

    creatures.0.retain(|id, e| {
        let keep = snapshot.creatures.iter().any(|c| c.id == *id);
        if !keep {
            commands.entity(*e).despawn();
        }
        keep
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transform_stands_on_the_feet() {
        let c = CreatureState {
            id: 1,
            kind: "ovelha".into(),
            feet: [1.5, 10.0, -2.5],
            yaw: 0.0,
            width: 0.9,
            height: 1.3,
        };
        let t = creature_transform(&c);
        assert_eq!(t.translation, Vec3::new(1.5, 10.65, -2.5));
    }

    #[test]
    fn known_kinds_have_their_own_color() {
        let unknown = creature_color("???");
        assert_ne!(creature_color("coelho"), unknown);
        assert_ne!(creature_color("ovelha"), unknown);
        assert_ne!(creature_color("coelho"), creature_color("ovelha"));
    }
}
