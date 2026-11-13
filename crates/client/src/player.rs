//! Câmera first-person: mouse look é local (resposta imediata), posição vem
//! do servidor e é suavizada. Input de teclado é enviado na taxa do tick.

use std::collections::HashMap;

use bevy::input::mouse::AccumulatedMouseMotion;
use bevy::prelude::*;
use bevy::window::{CursorGrabMode, PrimaryWindow};
use game_protocol::{ClientMessage, PlayerId, PlayerInput};

use crate::network::{LatestSnapshot, LocalPlayer, NetClient};

const MOUSE_SENSITIVITY: f32 = 0.002;
const PITCH_LIMIT: f32 = 1.54;
/// Quão rápido a câmera alcança a posição do servidor (maior = mais seco).
const POSITION_SMOOTHING: f32 = 15.0;

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Time::<Fixed>::from_hz(f64::from(game_protocol::TICK_RATE)))
            .init_resource::<InputSeq>()
            .init_resource::<FlyRequest>()
            .init_resource::<RemotePlayers>()
            .add_systems(Startup, spawn_camera)
            .add_systems(
                Update,
                (
                    cursor_grab,
                    toggle_fly,
                    mouse_look,
                    apply_snapshot,
                    smooth_follow,
                )
                    .chain(),
            )
            .add_systems(FixedUpdate, send_input);
    }
}

#[derive(Component)]
pub struct PlayerCamera {
    yaw: f32,
    pitch: f32,
    /// Posição autoritativa mais recente.
    target: Vec3,
}

#[derive(Component)]
struct RemotePlayer;

#[derive(Resource, Default)]
struct InputSeq(u32);

/// Modo voo (debug) pedido ao servidor. Quem decide se vale é o servidor;
/// o estado real chega no snapshot (`PlayerState::flying`).
#[derive(Resource, Default)]
pub struct FlyRequest(pub bool);

#[derive(Resource, Default)]
struct RemotePlayers(HashMap<PlayerId, Entity>);

fn spawn_camera(mut commands: Commands) {
    let start = Vec3::new(0.0, 30.0, 0.0);
    commands.spawn((
        Camera3d::default(),
        Projection::Perspective(PerspectiveProjection {
            fov: 75.0_f32.to_radians(),
            far: 500.0,
            ..default()
        }),
        // MSAA desligado: bom ganho em GPU integrada; voxels têm bordas retas mesmo.
        Msaa::Off,
        Transform::from_translation(start),
        PlayerCamera {
            yaw: 0.0,
            pitch: 0.0,
            target: start,
        },
    ));
}

fn cursor_grab(
    mut window: Single<&mut Window, With<PrimaryWindow>>,
    mouse: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
) {
    if mouse.just_pressed(MouseButton::Left) {
        // X11 não suporta Locked; Confined + movimento bruto do mouse funciona igual.
        window.cursor_options.grab_mode = if cfg!(target_os = "macos") {
            CursorGrabMode::Locked
        } else {
            CursorGrabMode::Confined
        };
        window.cursor_options.visible = false;
    }
    if keys.just_pressed(KeyCode::Escape) {
        window.cursor_options.grab_mode = CursorGrabMode::None;
        window.cursor_options.visible = true;
    }
}

fn toggle_fly(keys: Res<ButtonInput<KeyCode>>, mut fly: ResMut<FlyRequest>) {
    if keys.just_pressed(KeyCode::F3) {
        fly.0 = !fly.0;
        info!("modo voo pedido: {}", fly.0);
    }
}

fn mouse_look(
    motion: Res<AccumulatedMouseMotion>,
    window: Single<&Window, With<PrimaryWindow>>,
    mut camera: Single<(&mut PlayerCamera, &mut Transform)>,
) {
    if window.cursor_options.grab_mode == CursorGrabMode::None || motion.delta == Vec2::ZERO {
        return;
    }
    let (ref mut cam, ref mut transform) = *camera;
    cam.yaw -= motion.delta.x * MOUSE_SENSITIVITY;
    cam.pitch = (cam.pitch - motion.delta.y * MOUSE_SENSITIVITY).clamp(-PITCH_LIMIT, PITCH_LIMIT);
    transform.rotation = Quat::from_euler(EulerRot::YXZ, cam.yaw, cam.pitch, 0.0);
}

fn axis(keys: &ButtonInput<KeyCode>, pos: KeyCode, neg: KeyCode) -> f32 {
    (keys.pressed(pos) as i8 - keys.pressed(neg) as i8) as f32
}

fn send_input(
    keys: Res<ButtonInput<KeyCode>>,
    camera: Single<&PlayerCamera>,
    net: Res<NetClient>,
    local: Res<LocalPlayer>,
    fly: Res<FlyRequest>,
    mut seq: ResMut<InputSeq>,
) {
    if local.id.is_none() {
        return;
    }
    seq.0 = seq.0.wrapping_add(1);
    net.send(ClientMessage::Input(PlayerInput {
        seq: seq.0,
        forward: axis(&keys, KeyCode::KeyW, KeyCode::KeyS),
        right: axis(&keys, KeyCode::KeyD, KeyCode::KeyA),
        // Andando: Espaço = pulo. Voando: Espaço/Shift sobem/descem.
        up: axis(&keys, KeyCode::Space, KeyCode::ShiftLeft),
        fly: fly.0,
        yaw: camera.yaw,
        pitch: camera.pitch,
    }));
}

// Temporário: some quando o player.rs for dividido em plugins (fase 6).
#[allow(clippy::too_many_arguments)]
fn apply_snapshot(
    mut commands: Commands,
    mut snapshot: ResMut<LatestSnapshot>,
    local: Res<LocalPlayer>,
    mut camera: Single<&mut PlayerCamera>,
    mut remotes: ResMut<RemotePlayers>,
    mut remote_q: Query<&mut Transform, With<RemotePlayer>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    if !snapshot.fresh {
        return;
    }
    snapshot.fresh = false;

    let mut seen = Vec::with_capacity(snapshot.players.len());
    for p in &snapshot.players {
        let pos = Vec3::from_array(p.position);
        if Some(p.id) == local.id {
            camera.target = pos;
            continue;
        }
        seen.push(p.id);
        let rotation = Quat::from_rotation_y(p.yaw);
        // Corpo centralizado abaixo do olho.
        let body = pos - Vec3::Y * 0.8;
        match remotes.0.get(&p.id).and_then(|&e| remote_q.get_mut(e).ok()) {
            Some(mut t) => {
                t.translation = body;
                t.rotation = rotation;
            }
            None => {
                let e = commands
                    .spawn((
                        RemotePlayer,
                        Mesh3d(meshes.add(Cuboid::new(0.6, 1.8, 0.6))),
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

fn smooth_follow(time: Res<Time>, mut camera: Single<(&PlayerCamera, &mut Transform)>) {
    let (cam, ref mut transform) = *camera;
    let t = 1.0 - (-POSITION_SMOOTHING * time.delta_secs()).exp();
    transform.translation = transform.translation.lerp(cam.target, t);
}
