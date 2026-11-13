//! Servidor autoritativo: recebe input dos clientes, simula a 20 Hz e
//! transmite snapshots. Clientes nunca dizem onde estão, só o que apertaram.
//!
//! Este arquivo é só a composition root: lê a configuração, monta a
//! simulação com o gerador de mundo e liga os adaptadores (loop de tick e
//! transporte WebSocket).

mod config;
mod net;
mod tick;

use std::sync::Arc;

use engine_voxel::HeightmapGenerator;
use game_sim::{SimConfig, Simulation};
use tokio::sync::{broadcast, mpsc};
use tracing_subscriber::EnvFilter;

use crate::config::Config;
use crate::net::AppState;

/// Mundo inicial: 8x2x8 chunks ao redor da origem.
const WORLD_RADIUS_CHUNKS: i32 = 4;
const WORLD_HEIGHT_CHUNKS: i32 = 2;
/// Seed fixa: o mesmo servidor sempre começa a partida igual.
const WORLD_SEED: u64 = 0x5EED;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::from_env();

    let filter = config
        .log_filter
        .as_deref()
        .and_then(|f| EnvFilter::try_new(f).ok())
        .unwrap_or_else(|| EnvFilter::new("info"));
    tracing_subscriber::fmt().with_env_filter(filter).init();

    let sim = Simulation::new(
        &HeightmapGenerator,
        SimConfig {
            world_radius_chunks: WORLD_RADIUS_CHUNKS,
            world_height_chunks: WORLD_HEIGHT_CHUNKS,
            allow_fly: config.allow_fly,
            seed: WORLD_SEED,
        },
    );
    tracing::info!("mundo gerado: {} chunks não vazios", sim.chunks().len());
    tracing::info!(
        "modo voo (debug): {}",
        if config.allow_fly {
            "permitido"
        } else {
            "bloqueado"
        }
    );

    let (command_tx, command_rx) = mpsc::unbounded_channel();
    let (snapshot_tx, _) = broadcast::channel(64);
    let state = AppState::new(
        command_tx,
        snapshot_tx.clone(),
        Arc::clone(sim.chunks()),
        sim.spawn(),
    );

    tokio::spawn(tick::run(sim, command_rx, snapshot_tx));

    let listener = tokio::net::TcpListener::bind(&config.bind_addr).await?;
    tracing::info!("escutando em ws://{}/ws", config.bind_addr);
    axum::serve(listener, net::router(state)).await?;
    Ok(())
}
