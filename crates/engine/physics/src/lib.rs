//! Física do jogador. Pura e determinística: o servidor roda para decidir a
//! posição e, no futuro, o cliente pode rodar a mesma coisa para predição.
//!
//! Convenção igual à do Bevy: câmera olha para -Z, yaw gira em torno de +Y.
//! `position` é sempre a posição do olho; o corpo é uma AABB de
//! `PLAYER_WIDTH` x `PLAYER_HEIGHT` cujo pé fica em `position.y - EYE_HEIGHT`.

use engine_voxel::VoxelQuery;

/// Velocidade horizontal andando (blocos/s).
pub const MOVE_SPEED: f32 = 5.0;
/// Velocidade no modo voo (debug).
pub const FLY_SPEED: f32 = 8.0;
/// Altura do olho acima do pé.
pub const EYE_HEIGHT: f32 = 1.7;
/// Largura (X e Z) da AABB do jogador.
pub const PLAYER_WIDTH: f32 = 0.6;
/// Altura da AABB do jogador.
pub const PLAYER_HEIGHT: f32 = 1.8;
/// Aceleração da gravidade (blocos/s²).
pub const GRAVITY: f32 = 28.0;
/// Velocidade vertical do pulo: sqrt(2 * g * h) para h ≈ 1.25 bloco.
pub const JUMP_VELOCITY: f32 = 8.4;
/// Velocidade máxima de queda (blocos/s).
pub const TERMINAL_VELOCITY: f32 = 50.0;

/// Maior deslocamento por eixo numa sub-etapa. Menor que 1 bloco para não
/// atravessar blocos em queda rápida (tunneling).
const MAX_SUBSTEP: f32 = 0.45;
/// Folga para o corpo não ficar exatamente encostado na face (evita que o
/// arredondamento conte o bloco vizinho como sobreposto).
const SKIN: f32 = 1e-3;
/// Quantos blocos no máximo o jogador é empurrado para cima se começar o
/// tick dentro de terreno (ex.: saiu do voo atravessando o chão).
const MAX_UNSTUCK: i32 = 32;

/// O que o jogador quer fazer neste tick, já decidido pela aplicação
/// (inclusive se o voo é permitido). Não é um tipo de rede.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct MoveIntent {
    /// -1..1, trás..frente.
    pub forward: f32,
    /// -1..1, esquerda..direita.
    pub right: f32,
    /// Radianos, rotação em torno de Y (direção de "frente").
    pub yaw: f32,
    /// Pular, se estiver no chão (só andando).
    pub jump: bool,
    /// Voo livre: sem gravidade nem colisão.
    pub fly: bool,
    /// -1..1, desce..sobe (só voando).
    pub fly_vertical: f32,
}

/// Estado físico de um jogador. É isto que o servidor guarda e avança a cada tick.
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
}

impl Body {
    /// Corpo parado, com o olho em `position`.
    pub fn at(position: [f32; 3]) -> Self {
        Self {
            position,
            ..Default::default()
        }
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

    /// Movimento livre: atravessa tudo.
    fn step_fly(&mut self, intent: &MoveIntent, dt: f32) {
        let [x, z] = horizontal_dir(intent);
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
        let [x, z] = horizontal_dir(intent);
        self.velocity[0] = x * MOVE_SPEED;
        self.velocity[2] = z * MOVE_SPEED;

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
            let feet = self.position[1] - EYE_HEIGHT;
            self.position[1] = feet.floor() + 1.0 + SKIN + EYE_HEIGHT;
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
        match axis {
            1 => (EYE_HEIGHT, PLAYER_HEIGHT - EYE_HEIGHT),
            _ => (PLAYER_WIDTH / 2.0, PLAYER_WIDTH / 2.0),
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

/// Direção horizontal (x, z) da intenção, normalizada se passar de 1.
fn horizontal_dir(intent: &MoveIntent) -> [f32; 2] {
    let (sin, cos) = intent.yaw.sin_cos();
    let forward = [-sin, -cos];
    let right = [cos, -sin];
    let mut dir = [
        forward[0] * intent.forward + right[0] * intent.right,
        forward[1] * intent.forward + right[1] * intent.right,
    ];
    let len = (dir[0] * dir[0] + dir[1] * dir[1]).sqrt();
    if len > 1.0 {
        dir.iter_mut().for_each(|c| *c /= len);
    }
    dir
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
