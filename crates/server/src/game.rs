use std::time::Duration;

use axum::extract::ws::Utf8Bytes;
use game_protocol::ServerMessage;
use game_sim::{Command, SimEvent, Simulation, TICK_RATE};
use tokio::sync::{broadcast, mpsc};
use tokio::time::MissedTickBehavior;

/// Loop de simulação em taxa fixa. Único dono da simulação: aplica os
/// comandos que chegaram das conexões, avança um tick e transmite o snapshot.
pub async fn run(
    mut sim: Simulation,
    mut commands: mpsc::UnboundedReceiver<Command>,
    snapshots: broadcast::Sender<Utf8Bytes>,
) {
    let mut interval = tokio::time::interval(Duration::from_secs_f64(1.0 / f64::from(TICK_RATE)));
    interval.set_missed_tick_behavior(MissedTickBehavior::Skip);

    loop {
        interval.tick().await;

        while let Ok(command) = commands.try_recv() {
            match command {
                Command::Join { id } => tracing::info!("jogador {id} entrou"),
                Command::Leave { id } => tracing::info!("jogador {id} saiu"),
                Command::Input { .. } => {}
            }
            sim.apply(command);
        }
        for event in sim.tick() {
            match event {
                SimEvent::FellIntoVoid(id) => {
                    tracing::info!("jogador {id} caiu no vazio, voltando ao spawn")
                }
            }
        }

        if !sim.has_players() {
            continue;
        }
        match game_protocol::encode(&ServerMessage::from(&sim.snapshot())) {
            // Erro só significa "ninguém inscrito agora".
            Ok(json) => {
                let _ = snapshots.send(json.into());
            }
            Err(e) => tracing::error!("falha ao serializar snapshot: {e}"),
        }
    }
}
