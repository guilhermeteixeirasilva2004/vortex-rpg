//! Servidor autoritativo: recebe input dos clientes, simula a 20 Hz e
//! transmite snapshots. Clientes nunca dizem onde estão, só o que apertaram.

mod game;
mod net;

use std::sync::Arc;

use engine_voxel::HeightmapGenerator;
use game_sim::{SimConfig, Simulation};
use tokio::sync::{broadcast, mpsc};
use tracing_subscriber::EnvFilter;

use crate::net::AppState;

/// Endereço padrão de escuta.
const DEFAULT_BIND_ADDR: &str = "0.0.0.0:3000";

/// Mundo inicial: 8x2x8 chunks ao redor da origem.
const WORLD_RADIUS_CHUNKS: i32 = 4;
const WORLD_HEIGHT_CHUNKS: i32 = 2;

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
    let config = SimConfig {
        world_radius_chunks: WORLD_RADIUS_CHUNKS,
        world_height_chunks: WORLD_HEIGHT_CHUNKS,
        allow_fly,
    };
    let sim = Simulation::new(&HeightmapGenerator, config);
    tracing::info!("mundo gerado: {} chunks não vazios", sim.chunks().len());
    tracing::info!(
        "modo voo (debug): {}",
        if allow_fly { "permitido" } else { "bloqueado" }
    );
    let state = AppState::new(
        event_tx,
        snapshot_tx.clone(),
        Arc::clone(sim.chunks()),
        sim.spawn(),
    );

    tokio::spawn(game::run(sim, event_rx, snapshot_tx));

    let addr = std::env::var("VORTEX_BIND").unwrap_or_else(|_| DEFAULT_BIND_ADDR.into());
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("escutando em ws://{addr}/ws");
    axum::serve(listener, net::router(state)).await?;
    Ok(())
}
