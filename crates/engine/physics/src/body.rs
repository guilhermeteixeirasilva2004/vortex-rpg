use engine_voxel::VoxelQuery;

use crate::{BodySpec, MoveIntent, FLY_SPEED, GRAVITY, JUMP_VELOCITY, TERMINAL_VELOCITY};

/// Maior deslocamento por eixo numa sub-etapa. Menor que 1 bloco para não
/// atravessar blocos em queda rápida (tunneling).
const MAX_SUBSTEP: f32 = 0.45;
/// Folga para o corpo não ficar exatamente encostado na face (evita que o
/// arredondamento conte o bloco vizinho como sobreposto).
const SKIN: f32 = 1e-3;
/// Quantos blocos no máximo o corpo é empurrado para cima se começar o
/// tick dentro de terreno (ex.: saiu do voo atravessando o chão).
const MAX_UNSTUCK: i32 = 32;
/// Quanto à frente do corpo `obstacle_ahead` procura (blocos). Pequeno para
/// só reagir ao que está quase encostado.
const OBSTACLE_PROBE: f32 = 0.2;
/// Altura acima do pé examinada por `obstacle_ahead` (blocos): só a camada
/// do pé, que é o que um degrau ocupa.
const OBSTACLE_PROBE_HEIGHT: f32 = 0.5;

/// Estado físico de um corpo. É isto que o servidor guarda e avança a cada tick.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Body {
    /// Posição do olho.
    pub position: [f32; 3],
    /// Velocidade atual (blocos/s).
    pub velocity: [f32; 3],
    /// Se terminou o último tick apoiado no chão.
    pub on_ground: bool,
    /// Modo debug: voo livre, sem gravidade nem colisão.
    pub flying: bool,
    /// Forma e velocidade do corpo.
    pub spec: BodySpec,
}

impl Body {
    /// Corpo de jogador parado, com o olho em `position`.
    pub fn at(position: [f32; 3]) -> Self {
        Self::with_spec(position, BodySpec::PLAYER)
    }

    /// Corpo parado com a forma `spec`, com o olho em `position`.
    pub fn with_spec(position: [f32; 3], spec: BodySpec) -> Self {
        Self {
            position,
            spec,
            ..Default::default()
        }
    }

    /// Posição do pé (centro da base da AABB).
    pub fn feet(&self) -> [f32; 3] {
        let [x, y, z] = self.position;
        [x, y - self.spec.eye_height, z]
    }

    /// Avança um tick de `dt` segundos.
    pub fn step(&mut self, intent: &MoveIntent, world: &impl VoxelQuery, dt: f32) {
        let flying = intent.fly;
        if flying != self.flying {
            self.flying = flying;
            self.velocity = [0.0; 3];
            self.on_ground = false;
        }
        if self.flying {
            self.step_fly(intent, dt);
        } else {
            self.step_walk(intent, world, dt);
        }
    }

    /// Se há bloco sólido encostado à frente, na altura do pé, andando na
    /// direção horizontal `dir` (x, z): um degrau ou parede que só se passa
    /// pulando.
    pub fn obstacle_ahead(&self, dir: [f32; 2], world: &impl VoxelQuery) -> bool {
        let len = (dir[0] * dir[0] + dir[1] * dir[1]).sqrt();
        if !(len.is_finite() && len > 0.0) {
            return false;
        }
        let (mut min, mut max) = self.aabb();
        for (axis, d) in [(0, dir[0] / len), (2, dir[1] / len)] {
            min[axis] += d * OBSTACLE_PROBE;
            max[axis] += d * OBSTACLE_PROBE;
        }
        max[1] = min[1] + OBSTACLE_PROBE_HEIGHT;
        let range = |a: usize| block_range(min, max, a);
        range(0).any(|x| range(1).any(|y| range(2).any(|z| world.is_solid(x, y, z))))
    }

    /// Movimento livre: atravessa tudo.
    fn step_fly(&mut self, intent: &MoveIntent, dt: f32) {
        let [x, z] = intent.horizontal_dir();
        let mut dir = [x, intent.fly_vertical, z];
        let len = (dir[0] * dir[0] + dir[1] * dir[1] + dir[2] * dir[2]).sqrt();
        if len > 1.0 {
            dir.iter_mut().for_each(|c| *c /= len);
        }
        for (p, d) in self.position.iter_mut().zip(dir) {
            *p += d * FLY_SPEED * dt;
        }
    }

    fn step_walk(&mut self, intent: &MoveIntent, world: &impl VoxelQuery, dt: f32) {
        self.unstuck(world);

        // Controle horizontal direto (sem inércia): responde na hora, inclusive no ar.
        let [x, z] = intent.horizontal_dir();
        self.velocity[0] = x * self.spec.walk_speed;
        self.velocity[2] = z * self.spec.walk_speed;

        if self.on_ground && intent.jump {
            self.velocity[1] = JUMP_VELOCITY;
        }
        self.velocity[1] = (self.velocity[1] - GRAVITY * dt).max(-TERMINAL_VELOCITY);

        let delta = self.velocity.map(|v| v * dt);
        let max = delta.iter().fold(0.0_f32, |m, d| m.max(d.abs()));
        let steps = ((max / MAX_SUBSTEP).ceil() as u32).max(1);

        self.on_ground = false;
        for _ in 0..steps {
            // Vertical primeiro: assim o chão é resolvido antes de deslizar nas paredes.
            for axis in [1, 0, 2] {
                let d = delta[axis] / steps as f32;
                if d != 0.0 && self.move_axis(axis, d, world) {
                    if axis == 1 && d < 0.0 {
                        self.on_ground = true;
                    }
                    self.velocity[axis] = 0.0;
                }
            }
        }
    }

    /// Move num eixo e, se entrar num bloco sólido, encosta na face dele.
    /// Retorna `true` se colidiu.
    fn move_axis(&mut self, axis: usize, d: f32, world: &impl VoxelQuery) -> bool {
        self.position[axis] += d;
        let (min, max) = self.aabb();
        let range = |a: usize| block_range(min, max, a);
        let (xs, ys, zs) = (range(0), range(1), range(2));

        // Bloco sólido mais próximo na direção do movimento.
        let mut hit: Option<i32> = None;
        for bx in xs.clone() {
            for by in ys.clone() {
                for bz in zs.clone() {
                    if !world.is_solid(bx, by, bz) {
                        continue;
                    }
                    let b = [bx, by, bz][axis];
                    hit = Some(match hit {
                        Some(h) if d > 0.0 => h.min(b),
                        Some(h) => h.max(b),
                        None => b,
                    });
                }
            }
        }
        let Some(b) = hit else { return false };

        let (lo_off, hi_off) = self.extent_offsets(axis);
        self.position[axis] = if d > 0.0 {
            b as f32 - hi_off - SKIN
        } else {
            (b + 1) as f32 + lo_off + SKIN
        };
        true
    }

    /// Se a AABB começa dentro de terreno, sobe bloco a bloco até ficar livre.
    fn unstuck(&mut self, world: &impl VoxelQuery) {
        for _ in 0..MAX_UNSTUCK {
            if !self.overlaps_solid(world) {
                return;
            }
            let eye = self.spec.eye_height;
            let feet = self.position[1] - eye;
            self.position[1] = feet.floor() + 1.0 + SKIN + eye;
            self.velocity[1] = 0.0;
        }
    }

    fn overlaps_solid(&self, world: &impl VoxelQuery) -> bool {
        let (min, max) = self.aabb();
        let range = |a: usize| block_range(min, max, a);
        range(0).any(|x| range(1).any(|y| range(2).any(|z| world.is_solid(x, y, z))))
    }

    /// Distância do `position` até a face mínima e máxima da AABB, por eixo
    /// (a face mínima é `position - lo`, a máxima `position + hi`).
    fn extent_offsets(&self, axis: usize) -> (f32, f32) {
        let s = self.spec;
        match axis {
            1 => (s.eye_height, s.height - s.eye_height),
            _ => (s.width / 2.0, s.width / 2.0),
        }
    }

    fn aabb(&self) -> ([f32; 3], [f32; 3]) {
        let mut min = [0.0; 3];
        let mut max = [0.0; 3];
        for a in 0..3 {
            let (lo, hi) = self.extent_offsets(a);
            min[a] = self.position[a] - lo;
            max[a] = self.position[a] + hi;
        }
        (min, max)
    }
}

/// Blocos inteiros que a AABB ocupa num eixo: i tal que i < max e i + 1 > min.
fn block_range(min: [f32; 3], max: [f32; 3], axis: usize) -> std::ops::Range<i32> {
    min[axis].floor() as i32..max[axis].ceil() as i32
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{EYE_HEIGHT, PLAYER_WIDTH};

    /// Tick de 20 Hz, como no servidor.
    const TICK_DT: f32 = 1.0 / 20.0;

    /// Chão plano: tudo com y < 10 é sólido, mais um muro em x >= 5.
    struct Flat;
    impl VoxelQuery for Flat {
        fn is_solid(&self, x: i32, y: i32, _z: i32) -> bool {
            y < 10 || (x >= 5 && y < 15)
        }
    }

    fn run(body: &mut Body, intent: MoveIntent, ticks: u32) {
        for _ in 0..ticks {
            body.step(&intent, &Flat, TICK_DT);
        }
    }

    fn feet(b: &Body) -> f32 {
        b.position[1] - EYE_HEIGHT
    }

    #[test]
    fn falls_and_lands_on_ground() {
        let mut b = Body::at([0.5, 30.0, 0.5]);
        run(&mut b, MoveIntent::default(), 60);
        assert!(b.on_ground);
        assert!((feet(&b) - 10.0).abs() < 0.01, "pé em {}", feet(&b));
        assert_eq!(b.velocity[1], 0.0);
    }

    #[test]
    fn fast_fall_does_not_tunnel() {
        let mut b = Body::at([0.5, 12.0 + EYE_HEIGHT, 0.5]);
        b.velocity[1] = -TERMINAL_VELOCITY;
        run(&mut b, MoveIntent::default(), 1);
        assert!(feet(&b) >= 10.0 - 0.01, "atravessou: pé em {}", feet(&b));
    }

    #[test]
    fn jump_only_from_ground() {
        let mut b = Body::at([0.5, 10.0 + SKIN + EYE_HEIGHT, 0.5]);
        run(&mut b, MoveIntent::default(), 2);
        assert!(b.on_ground);

        let jump = MoveIntent {
            jump: true,
            ..Default::default()
        };
        let mut peak = 0.0_f32;
        for _ in 0..20 {
            b.step(&jump, &Flat, TICK_DT);
            peak = peak.max(feet(&b));
            if b.on_ground {
                break;
            }
        }
        assert!(
            peak > 11.0 && peak < 11.6,
            "altura do pulo: {}",
            peak - 10.0
        );
    }

    #[test]
    fn wall_blocks_horizontal_movement() {
        let mut b = Body::at([0.5, 10.0 + SKIN + EYE_HEIGHT, 0.5]);
        // yaw = -90°: "frente" vira +X.
        let walk = MoveIntent {
            forward: 1.0,
            yaw: -std::f32::consts::FRAC_PI_2,
            ..Default::default()
        };
        run(&mut b, walk, 40);
        assert!(
            b.position[0] + PLAYER_WIDTH / 2.0 <= 5.0,
            "x = {}",
            b.position[0]
        );
        assert!(b.position[0] > 4.5);
        assert!(b.on_ground);
    }

    #[test]
    fn fly_ignores_gravity_and_collision() {
        let mut b = Body::at([0.5, 5.0, 0.5]); // dentro do chão
        let fly = MoveIntent {
            fly: true,
            ..Default::default()
        };
        run(&mut b, fly, 20);
        assert_eq!(b.position, [0.5, 5.0, 0.5]);
        assert!(b.flying);
    }

    #[test]
    fn leaving_fly_inside_terrain_pops_out() {
        let mut b = Body::at([0.5, 5.0, 0.5]);
        run(&mut b, MoveIntent::default(), 5);
        assert!(feet(&b) >= 10.0 - 0.01, "pé em {}", feet(&b));
    }

    /// Corpo pequeno e lento, como o de uma criatura.
    const SMALL: BodySpec = BodySpec {
        width: 0.4,
        height: 0.5,
        eye_height: 0.4,
        walk_speed: 2.0,
    };

    #[test]
    fn spec_sets_size_and_speed() {
        let mut b = Body::with_spec([0.5, 10.0 + SKIN + SMALL.eye_height, 0.5], SMALL);
        run(&mut b, MoveIntent::walk_towards([0.0, 1.0], false), 20);
        assert!(b.on_ground);
        assert!((b.feet()[1] - 10.0).abs() < 0.01, "pé em {:?}", b.feet());
        // 20 ticks de 0,05 s a 2 blocos/s: 2 blocos em +Z.
        assert!((b.position[2] - 2.5).abs() < 1e-3, "z = {}", b.position[2]);
    }

    #[test]
    fn small_body_fits_under_low_ceiling() {
        // Túnel de 1 bloco de altura (só y = 10 é livre): o jogador (1,8)
        // não caberia; o corpo de 0,5 assenta no chão sem ser empurrado.
        struct Tunnel;
        impl VoxelQuery for Tunnel {
            fn is_solid(&self, _x: i32, y: i32, _z: i32) -> bool {
                !(10..=10).contains(&y)
            }
        }
        let mut b = Body::with_spec([0.5, 10.0 + SKIN + SMALL.eye_height, 0.5], SMALL);
        for _ in 0..10 {
            b.step(&MoveIntent::default(), &Tunnel, TICK_DT);
        }
        assert!(b.on_ground);
        assert!((b.feet()[1] - 10.0).abs() < 0.01, "pé em {:?}", b.feet());
    }

    #[test]
    fn obstacle_ahead_sees_only_what_blocks_the_feet() {
        // Parado em x = 4,5 encostando no muro de `Flat` (x >= 5).
        let b = Body::at([
            5.0 - PLAYER_WIDTH / 2.0 - SKIN,
            10.0 + SKIN + EYE_HEIGHT,
            0.5,
        ]);
        assert!(b.obstacle_ahead([1.0, 0.0], &Flat));
        assert!(!b.obstacle_ahead([-1.0, 0.0], &Flat));
        assert!(!b.obstacle_ahead([0.0, 1.0], &Flat));
        assert!(!b.obstacle_ahead([0.0, 0.0], &Flat));
        // Longe do muro, nada à frente; o chão embaixo do pé não conta.
        let far = Body::at([0.5, 10.0 + SKIN + EYE_HEIGHT, 0.5]);
        assert!(!far.obstacle_ahead([1.0, 0.0], &Flat));
    }

    #[test]
    fn jumping_at_obstacle_climbs_one_block_step() {
        // Degrau de 1 bloco em x >= 3, sobre o chão y < 10.
        struct Step;
        impl VoxelQuery for Step {
            fn is_solid(&self, x: i32, y: i32, _z: i32) -> bool {
                y < 10 || (x >= 3 && y < 11)
            }
        }
        let dir = [1.0, 0.0];
        let mut b = Body::with_spec([0.5, 10.0 + SKIN + SMALL.eye_height, 0.5], SMALL);
        for _ in 0..80 {
            let jump = b.obstacle_ahead(dir, &Step);
            b.step(&MoveIntent::walk_towards(dir, jump), &Step, TICK_DT);
        }
        assert!(b.position[0] > 4.0, "x = {}", b.position[0]);
        assert!((b.feet()[1] - 11.0).abs() < 0.01, "pé em {:?}", b.feet());
    }
}
