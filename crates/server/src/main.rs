//! Servidor autoritativo: recebe input dos clientes, simula a 20 Hz e
//! transmite snapshots. Clientes nunca dizem onde estão, só o que apertaram.

mod game;
mod net;

use std::sync::Arc;

use shared::voxel::{generate_chunk, ChunkPos};
use tokio::sync::{broadcast, mpsc};
use tracing_subscriber::EnvFilter;

use crate::game::GameWorld;
use crate::net::AppState;

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

    let mut chunks = Vec::new();
    for x in -WORLD_RADIUS_CHUNKS..WORLD_RADIUS_CHUNKS {
        for z in -WORLD_RADIUS_CHUNKS..WORLD_RADIUS_CHUNKS {
            for y in 0..WORLD_HEIGHT_CHUNKS {
                let chunk = generate_chunk(ChunkPos::new(x, y, z));
                if !chunk.is_empty() {
                    chunks.push(chunk);
                }
            }
        }
    }
    tracing::info!("mundo gerado: {} chunks não vazios", chunks.len());

    let (event_tx, event_rx) = mpsc::unbounded_channel();
    let (snapshot_tx, _) = broadcast::channel(64);

    let world = GameWorld::new(chunks);
    let state = AppState::new(event_tx, snapshot_tx.clone(), Arc::new(world.chunks().to_vec()));

    tokio::spawn(game::run(world, event_rx, snapshot_tx));

    let addr = std::env::var("VORTEX_BIND").unwrap_or_else(|_| shared::DEFAULT_BIND_ADDR.into());
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("escutando em ws://{addr}/ws");
    axum::serve(listener, net::router(state)).await?;
    Ok(())
}
