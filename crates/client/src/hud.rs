//! Contador de FPS — para acompanhar a meta de 30 fps em GPU integrada.

use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy::prelude::*;

use crate::network::{LatestSnapshot, LocalPlayer};

pub struct HudPlugin;

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(FrameTimeDiagnosticsPlugin::default())
            .add_systems(Startup, spawn_hud)
            .add_systems(Update, update_fps);
    }
}

#[derive(Component)]
struct FpsText;

fn spawn_hud(mut commands: Commands) {
    commands.spawn((
        FpsText,
        Text::new("FPS: --"),
        TextFont {
            font_size: 18.0,
            ..default()
        },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(8.0),
            left: Val::Px(8.0),
            ..default()
        },
    ));
}

fn update_fps(
    diagnostics: Res<DiagnosticsStore>,
    snapshot: Res<LatestSnapshot>,
    local: Res<LocalPlayer>,
    mut text: Single<&mut Text, With<FpsText>>,
) {
    let Some(fps) = diagnostics
        .get(&FrameTimeDiagnosticsPlugin::FPS)
        .and_then(|d| d.smoothed())
    else {
        return;
    };
    // Estado confirmado pelo servidor, não o pedido local.
    let flying = snapshot
        .players
        .iter()
        .find(|p| Some(p.id) == local.id)
        .is_some_and(|p| p.flying);
    let mode = if flying { "VOO (debug)" } else { "andando" };
    text.0 = format!(
        "FPS: {fps:.0}  |  modo: {mode}  F3: voo  |  clique: capturar mouse  Esc: soltar"
    );
}
