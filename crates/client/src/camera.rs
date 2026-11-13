//! Câmera first-person: mouse look é local (resposta imediata), posição vem
//! do servidor e é suavizada.

use bevy::input::mouse::AccumulatedMouseMotion;
use bevy::prelude::*;
use bevy::window::{CursorGrabMode, PrimaryWindow};

use crate::network::{LatestSnapshot, LocalPlayer};

const MOUSE_SENSITIVITY: f32 = 0.002;
const PITCH_LIMIT: f32 = 1.54;
/// Quão rápido a câmera alcança a posição do servidor (maior = mais seco).
const POSITION_SMOOTHING: f32 = 15.0;

/// Câmera do jogador local: captura do cursor, olhar com o mouse e
/// acompanhamento da posição autoritativa.
pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_camera).add_systems(
            Update,
            (
                cursor_grab,
                mouse_look,
                follow_server_position,
                smooth_follow,
            )
                .chain(),
        );
    }
}

/// Estado da câmera do jogador local.
#[derive(Component)]
pub struct PlayerCamera {
    /// Radianos, em torno de Y (enviado ao servidor como direção de andar).
    pub yaw: f32,
    /// Radianos, olhar pra cima/baixo.
    pub pitch: f32,
    /// Posição autoritativa mais recente.
    target: Vec3,
}

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

/// Atualiza o alvo da câmera quando chega snapshot novo.
fn follow_server_position(
    snapshot: Res<LatestSnapshot>,
    local: Res<LocalPlayer>,
    mut camera: Single<&mut PlayerCamera>,
) {
    if !snapshot.is_changed() {
        return;
    }
    if let Some(me) = snapshot.players.iter().find(|p| Some(p.id) == local.id) {
        camera.target = Vec3::from_array(me.position);
    }
}

fn smooth_follow(time: Res<Time>, mut camera: Single<(&PlayerCamera, &mut Transform)>) {
    let (cam, ref mut transform) = *camera;
    let t = 1.0 - (-POSITION_SMOOTHING * time.delta_secs()).exp();
    transform.translation = transform.translation.lerp(cam.target, t);
}
