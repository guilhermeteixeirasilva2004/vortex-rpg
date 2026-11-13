//! Criaturas: nascem perto do spawn, perambulam sem atravessar blocos nem se
//! afastar da origem, sobem degraus de 1 bloco e são determinísticas.

use engine_voxel::{Block, CHUNK_SIZE};
use engine_voxel::{Chunk, ChunkMap, ChunkPos, HeightmapGenerator, VoxelQuery, WorldGenerator};
use game_sim::content::CREATURE_SPAWNS;
use game_sim::{Command, CreatureSnapshot, SimConfig, Simulation};

/// Folga além do raio de perambular (blocos): a criatura para a até
/// `arrive_distance` do destino e o último passo pode passar um pouco.
const RADIUS_SLACK: f32 = 0.5;

fn config(seed: u64) -> SimConfig {
    SimConfig {
        world_radius_chunks: 4,
        world_height_chunks: 2,
        allow_fly: false,
        seed,
    }
}

fn creatures(s: &Simulation) -> Vec<CreatureSnapshot> {
    s.snapshot().creatures
}

/// Se a AABB da criatura ocupa algum bloco sólido.
fn inside_terrain(world: &ChunkMap, c: &CreatureSnapshot) -> bool {
    // Pequena margem para dentro: encostar na face não é atravessar.
    const EPS: f32 = 1e-4;
    let half = c.width / 2.0 - EPS;
    let [x, y, z] = c.feet;
    let r = |lo: f32, hi: f32| (lo.floor() as i32)..(hi.ceil() as i32);
    r(x - half, x + half).any(|bx| {
        r(y + EPS, y + c.height - EPS)
            .any(|by| r(z - half, z + half).any(|bz| world.is_solid(bx, by, bz)))
    })
}

fn horizontal_distance(a: [f32; 3], b: [f32; 3]) -> f32 {
    (a[0] - b[0]).hypot(a[2] - b[2])
}

#[test]
fn creatures_spawn_on_the_surface_near_spawn() {
    let s = Simulation::new(&HeightmapGenerator, config(1));
    let cs = creatures(&s);
    assert_eq!(cs.len(), CREATURE_SPAWNS.len());
    for (c, spawn) in cs.iter().zip(CREATURE_SPAWNS) {
        let [x, z] = spawn.offset;
        assert_eq!(c.kind, spawn.kind.name);
        assert_eq!(c.width, spawn.kind.body.width);
        assert_eq!(c.height, spawn.kind.body.height);
        let ground = HeightmapGenerator.surface_height(x, z) as f32 + 1.0;
        assert_eq!(c.feet, [x as f32 + 0.5, ground, z as f32 + 0.5]);
    }
}

#[test]
fn creatures_wander_without_entering_blocks_or_straying() {
    let mut s = Simulation::new(&HeightmapGenerator, config(7));
    // Um jogador no mundo não muda nada para as criaturas.
    s.apply(Command::Join { id: 1 });
    let start = creatures(&s);
    let world = s.chunks().clone();
    let mut farthest = vec![0.0_f32; start.len()];
    // 120 s de jogo.
    for _ in 0..2400 {
        s.tick();
        for (i, c) in creatures(&s).iter().enumerate() {
            assert!(
                !inside_terrain(&world, c),
                "{} {} dentro do terreno",
                c.kind,
                c.id
            );
            let d = horizontal_distance(c.feet, start[i].feet);
            let radius = CREATURE_SPAWNS[i].kind.wander.radius;
            assert!(
                d <= radius + RADIUS_SLACK,
                "{} {} longe demais: {d}",
                c.kind,
                c.id
            );
            farthest[i] = farthest[i].max(d);
        }
    }
    for (i, d) in farthest.iter().enumerate() {
        assert!(*d > 1.0, "criatura {i} quase não andou: {d}");
    }
}

#[test]
fn creatures_move_without_players() {
    let mut s = Simulation::new(&HeightmapGenerator, config(3));
    let before = creatures(&s);
    for _ in 0..400 {
        s.tick();
    }
    assert!(!s.has_players());
    assert_ne!(creatures(&s), before);
}

#[test]
fn same_seed_same_creatures() {
    let run = |seed| {
        let mut s = Simulation::new(&HeightmapGenerator, config(seed));
        (0..600)
            .map(|_| {
                s.tick();
                creatures(&s)
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(run(11), run(11));
    assert_ne!(run(11), run(12));
}

/// Escada: o terreno sobe 1 bloco a cada 2 colunas em X.
struct Stairs;

impl Stairs {
    const BASE: i32 = 8;
}

impl WorldGenerator for Stairs {
    fn surface_height(&self, x: i32, _z: i32) -> i32 {
        Self::BASE + x.div_euclid(2)
    }

    fn generate_chunk(&self, pos: ChunkPos) -> Chunk {
        let mut chunk = Chunk::empty(pos);
        let [ox, oy, oz] = pos.world_origin();
        for x in 0..CHUNK_SIZE {
            for z in 0..CHUNK_SIZE {
                let top = self.surface_height(ox + x as i32, oz + z as i32);
                for y in 0..CHUNK_SIZE {
                    if oy + y as i32 <= top {
                        chunk.set(x, y, z, Block::Stone);
                    }
                }
            }
        }
        chunk
    }
}

#[test]
fn creatures_climb_one_block_steps() {
    let mut s = Simulation::new(&Stairs, config(5));
    let start = creatures(&s);
    let world = s.chunks().clone();
    let mut climbed = vec![0.0_f32; start.len()];
    for _ in 0..2400 {
        s.tick();
        for (i, c) in creatures(&s).iter().enumerate() {
            assert!(
                !inside_terrain(&world, c),
                "{} {} dentro do terreno",
                c.kind,
                c.id
            );
            climbed[i] = climbed[i].max(c.feet[1] - start[i].feet[1]);
        }
    }
    // Perambulando num raio de 5 a 7 blocos numa escada de 1 bloco a cada 2,
    // toda criatura sobe pelo menos um degrau acima de onde nasceu.
    for (i, h) in climbed.iter().enumerate() {
        assert!(*h >= 1.0 - 0.01, "criatura {i} não subiu degrau: {h}");
    }
}
