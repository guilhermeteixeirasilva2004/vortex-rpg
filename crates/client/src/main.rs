//! Cliente: renderiza o mundo e manda input. Não simula nada autoritativo.

mod camera;
mod hud;
mod input;
mod network;
mod remote_players;
mod world;

use bevy::prelude::*;
use bevy::window::PresentMode;

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::srgb(0.55, 0.75, 0.95)))
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Vortex RPG".into(),
                resolution: (1280.0_f32, 720.0_f32).into(),
                // VSync: evita queimar GPU integrada renderizando frames que ninguém vê.
                present_mode: PresentMode::AutoVsync,
                ..default()
            }),
            ..default()
        }))
        .add_plugins((
            network::NetworkPlugin,
            world::WorldPlugin,
            camera::CameraPlugin,
            input::InputPlugin,
            remote_players::RemotePlayersPlugin,
            hud::HudPlugin,
        ))
        .run();
}
