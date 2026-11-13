//! Testes com mundos sintéticos, via `WorldGenerator` injetado.

use engine_physics::EYE_HEIGHT;
use engine_voxel::{Chunk, ChunkPos, WorldGenerator};
use game_sim::{Command, SimConfig, SimEvent, Simulation};

/// Mundo sem nenhum bloco: o jogador só pode cair.
struct EmptyWorld;

impl WorldGenerator for EmptyWorld {
    fn generate_chunk(&self, pos: ChunkPos) -> Chunk {
        Chunk::empty(pos)
    }

    fn surface_height(&self, _x: i32, _z: i32) -> i32 {
        0
    }
}

fn sim() -> Simulation {
    let config = SimConfig {
        world_radius_chunks: 1,
        world_height_chunks: 1,
        allow_fly: false,
    };
    Simulation::new(&EmptyWorld, config)
}

#[test]
fn spawn_comes_from_generator() {
    assert_eq!(sim().spawn(), [0.5, 1.0 + EYE_HEIGHT, 0.5]);
}

#[test]
fn empty_chunks_are_not_stored() {
    assert!(sim().chunks().is_empty());
}

#[test]
fn falling_into_void_respawns() {
    let mut s = sim();
    s.apply(Command::Join { id: 1 });
    let mut events = Vec::new();
    for _ in 0..100 {
        events.extend(s.tick());
        if !events.is_empty() {
            break;
        }
    }
    assert_eq!(events, [SimEvent::FellIntoVoid(1)]);
    assert_eq!(s.snapshot().players[0].position, s.spawn());
}
