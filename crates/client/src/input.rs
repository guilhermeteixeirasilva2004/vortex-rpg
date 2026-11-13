//! Teclado -> input enviado ao servidor, na taxa do tick. F3 alterna o
//! pedido de voo (debug).

use bevy::prelude::*;
use game_protocol::{ClientMessage, PlayerInput};

use crate::camera::PlayerCamera;
use crate::network::{LocalPlayer, NetClient};

/// Lê o teclado e envia `PlayerInput` a cada tick fixo.
pub struct InputPlugin;

impl Plugin for InputPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Time::<Fixed>::from_hz(f64::from(game_protocol::TICK_RATE)))
            .init_resource::<InputSeq>()
            .init_resource::<FlyRequest>()
            .add_systems(Update, toggle_fly)
            .add_systems(FixedUpdate, send_input);
    }
}

#[derive(Resource, Default)]
struct InputSeq(u32);

/// Modo voo (debug) pedido ao servidor. Quem decide se vale é o servidor;
/// o estado real chega no snapshot (`PlayerState::flying`).
#[derive(Resource, Default)]
struct FlyRequest(bool);

fn toggle_fly(keys: Res<ButtonInput<KeyCode>>, mut fly: ResMut<FlyRequest>) {
    if keys.just_pressed(KeyCode::F3) {
        fly.0 = !fly.0;
        info!("modo voo pedido: {}", fly.0);
    }
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
