use crate::Rng;

/// Como uma entidade perambula. Os valores são conteúdo do jogo.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WanderParams {
    /// Até que distância da origem ela escolhe destinos (blocos).
    pub radius: f32,
    /// Menor tempo parada entre caminhadas (ticks).
    pub min_idle_ticks: u32,
    /// Maior tempo parada entre caminhadas (ticks).
    pub max_idle_ticks: u32,
    /// Desiste do destino depois disso andando sem chegar (ticks), para não
    /// ficar presa empurrando uma parede.
    pub max_walk_ticks: u32,
    /// A esta distância do destino conta como chegou (blocos).
    pub arrive_distance: f32,
}

/// O que a entidade está fazendo.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum WanderState {
    /// Parada; volta a andar quando a contagem zerar.
    Idle {
        /// Ticks que ainda fica parada.
        ticks_left: u32,
    },
    /// Andando até `target`.
    Walking {
        /// Destino (x, z).
        target: [f32; 2],
        /// Ticks até desistir.
        ticks_left: u32,
    },
}

/// Perambula em volta de uma origem: fica parada um tempo, escolhe um ponto
/// dentro do raio, anda até ele (ou desiste) e repete.
#[derive(Clone, Debug, PartialEq)]
pub struct Wander {
    home: [f32; 2],
    params: WanderParams,
    state: WanderState,
    rng: Rng,
}

impl Wander {
    /// Começa parada por um tempo sorteado, para entidades criadas juntas
    /// não andarem em sincronia.
    pub fn new(home: [f32; 2], params: WanderParams, seed: u64) -> Self {
        let mut rng = Rng::new(seed);
        let state = WanderState::Idle {
            ticks_left: idle_ticks(&mut rng, &params),
        };
        Self {
            home,
            params,
            state,
            rng,
        }
    }

    /// Origem (x, z) em volta da qual perambula.
    pub fn home(&self) -> [f32; 2] {
        self.home
    }

    /// Estado atual.
    pub fn state(&self) -> WanderState {
        self.state
    }

    /// Avança um tick estando em `position` (x, z) e devolve para onde andar:
    /// direção unitária (x, z), ou `[0, 0]` para ficar parada.
    pub fn update(&mut self, position: [f32; 2]) -> [f32; 2] {
        if let WanderState::Idle { ticks_left } = self.state {
            if ticks_left > 0 {
                self.state = WanderState::Idle {
                    ticks_left: ticks_left - 1,
                };
                return [0.0, 0.0];
            }
            self.state = WanderState::Walking {
                target: self.pick_target(),
                ticks_left: self.params.max_walk_ticks,
            };
        }
        let WanderState::Walking { target, ticks_left } = self.state else {
            return [0.0, 0.0];
        };
        let diff = [target[0] - position[0], target[1] - position[1]];
        let dist = (diff[0] * diff[0] + diff[1] * diff[1]).sqrt();
        // NaN (posição inválida) conta como chegou: melhor parar que andar torto.
        let arrived = dist.is_nan() || dist <= self.params.arrive_distance;
        if ticks_left == 0 || arrived {
            self.state = WanderState::Idle {
                ticks_left: idle_ticks(&mut self.rng, &self.params),
            };
            return [0.0, 0.0];
        }
        self.state = WanderState::Walking {
            target,
            ticks_left: ticks_left - 1,
        };
        [diff[0] / dist, diff[1] / dist]
    }

    /// Ponto uniforme no disco de raio `radius` em volta da origem.
    fn pick_target(&mut self) -> [f32; 2] {
        // sqrt para a densidade não se concentrar perto do centro.
        let r = self.params.radius * self.rng.next_f32().sqrt();
        let angle = self.rng.next_f32() * std::f32::consts::TAU;
        let (sin, cos) = angle.sin_cos();
        [self.home[0] + r * cos, self.home[1] + r * sin]
    }
}

fn idle_ticks(rng: &mut Rng, params: &WanderParams) -> u32 {
    rng.range_u32(params.min_idle_ticks, params.max_idle_ticks)
}

#[cfg(test)]
mod tests {
    use super::*;

    const PARAMS: WanderParams = WanderParams {
        radius: 6.0,
        min_idle_ticks: 3,
        max_idle_ticks: 10,
        max_walk_ticks: 50,
        arrive_distance: 0.3,
    };
    /// Blocos por tick do ponto que segue a direção nos testes.
    const SPEED: f32 = 0.1;

    /// Move um ponto livre (sem colisão) seguindo a máquina por `ticks`.
    fn follow(w: &mut Wander, mut pos: [f32; 2], ticks: u32) -> Vec<[f32; 2]> {
        (0..ticks)
            .map(|_| {
                let d = w.update(pos);
                pos = [pos[0] + d[0] * SPEED, pos[1] + d[1] * SPEED];
                pos
            })
            .collect()
    }

    fn dist(a: [f32; 2], b: [f32; 2]) -> f32 {
        ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2)).sqrt()
    }

    #[test]
    fn starts_idle_within_bounds() {
        for seed in 0..50 {
            let WanderState::Idle { ticks_left } = Wander::new([0.0; 2], PARAMS, seed).state()
            else {
                panic!("devia começar parada");
            };
            assert!((PARAMS.min_idle_ticks..=PARAMS.max_idle_ticks).contains(&ticks_left));
        }
    }

    #[test]
    fn idle_stands_still_then_walks() {
        let mut w = Wander::new([0.0; 2], PARAMS, 1);
        let WanderState::Idle { ticks_left } = w.state() else {
            panic!("devia começar parada");
        };
        for _ in 0..ticks_left {
            assert_eq!(w.update([0.0; 2]), [0.0, 0.0]);
        }
        let d = w.update([0.0; 2]);
        assert!(matches!(w.state(), WanderState::Walking { .. }));
        assert!((d[0].hypot(d[1]) - 1.0).abs() < 1e-5, "direção {d:?}");
    }

    #[test]
    fn targets_stay_within_radius() {
        let home = [10.0, -4.0];
        for seed in 0..200 {
            let mut w = Wander::new(home, PARAMS, seed);
            // Parada na origem, ela sorteia vários destinos seguidos.
            let mut targets = 0;
            for _ in 0..2000 {
                w.update(home);
                if let WanderState::Walking { target, .. } = w.state() {
                    assert!(dist(target, home) <= PARAMS.radius + 1e-4, "{target:?}");
                    targets += 1;
                }
            }
            assert!(targets > 0, "nunca escolheu destino");
        }
    }

    #[test]
    fn arrives_and_rests() {
        let mut w = Wander::new([0.0; 2], PARAMS, 4);
        let mut walked = false;
        let mut rested_after_walking = false;
        let mut pos = [0.0; 2];
        for _ in 0..500 {
            let d = w.update(pos);
            pos = [pos[0] + d[0] * SPEED, pos[1] + d[1] * SPEED];
            match w.state() {
                WanderState::Walking { .. } => walked = true,
                WanderState::Idle { .. } if walked => rested_after_walking = true,
                WanderState::Idle { .. } => {}
            }
        }
        assert!(walked && rested_after_walking);
    }

    #[test]
    fn gives_up_when_stuck() {
        let mut w = Wander::new([0.0; 2], PARAMS, 9);
        // Nunca sai do lugar, como se estivesse contra uma parede.
        let mut walking_ticks = 0;
        for _ in 0..(PARAMS.max_idle_ticks + PARAMS.max_walk_ticks + 2) {
            w.update([0.0; 2]);
            if matches!(w.state(), WanderState::Walking { .. }) {
                walking_ticks += 1;
            } else if walking_ticks > 0 {
                break;
            }
        }
        assert!(walking_ticks > 0, "nunca andou");
        assert!(
            walking_ticks <= PARAMS.max_walk_ticks + 1,
            "{walking_ticks}"
        );
        assert!(matches!(w.state(), WanderState::Idle { .. }));
    }

    #[test]
    fn never_strays_far_from_home() {
        let home = [3.0, 3.0];
        for seed in 0..20 {
            let mut w = Wander::new(home, PARAMS, seed);
            for p in follow(&mut w, home, 3000) {
                assert!(dist(p, home) <= PARAMS.radius + SPEED + 1e-3, "{p:?}");
            }
        }
    }

    #[test]
    fn deterministic_per_seed() {
        let path = |seed| follow(&mut Wander::new([0.0; 2], PARAMS, seed), [0.0; 2], 400);
        assert_eq!(path(42), path(42));
        assert_ne!(path(42), path(43));
    }
}
