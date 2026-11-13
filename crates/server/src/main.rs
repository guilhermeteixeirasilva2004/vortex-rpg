//! Servidor autoritativo: recebe input dos clientes, simula a 20 Hz e
//! transmite snapshots. Clientes nunca dizem onde estão, só o que apertaram.

mod game;
mod net;

use std::sync::Arc;

use engine_voxel::HeightmapGenerator;
use tokio::sync::{broadcast, mpsc};
use tracing_subscriber::EnvFilter;

use crate::game::GameWorld;
use crate::net::AppState;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let (event_tx, event_rx) = mpsc::unbounded_channel();
    let (snapshot_tx, _) = broadcast::channel(64);

    // Voo livre (F3) é ferramenta de debug: liberado em build de debug,
    // ou explicitamente com VORTEX_ALLOW_FLY=1 / desligado com =0.
    let allow_fly = match std::env::var("VORTEX_ALLOW_FLY").as_deref() {
        Ok("1" | "true") => true,
        Ok("0" | "false") => false,
        _ => cfg!(debug_assertions),
    };
    tracing::info!(
        "modo voo (debug): {}",
        if allow_fly { "permitido" } else { "bloqueado" }
    );
    let world = GameWorld::new(&HeightmapGenerator, allow_fly);
    let state = AppState::new(
        event_tx,
        snapshot_tx.clone(),
        Arc::clone(world.chunks()),
        world.spawn(),
    );

    tokio::spawn(game::run(world, event_rx, snapshot_tx));

    let addr = std::env::var("VORTEX_BIND").unwrap_or_else(|_| shared::DEFAULT_BIND_ADDR.into());
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("escutando em ws://{addr}/ws");
    axum::serve(listener, net::router(state)).await?;
    Ok(())
}
