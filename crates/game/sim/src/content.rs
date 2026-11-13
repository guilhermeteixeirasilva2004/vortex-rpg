//! Conteúdo: quais criaturas existem e onde nascem. Só dados; o
//! comportamento vem de `engine-ai` e a física de `engine-physics`.

use engine_ai::WanderParams;
use engine_physics::BodySpec;

use crate::TICK_RATE;

/// Um tipo de criatura.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CreatureType {
    /// Nome mostrado e enviado ao cliente; único na tabela.
    pub name: &'static str,
    /// Tamanho e velocidade do corpo.
    pub body: BodySpec,
    /// Como perambula em volta de onde nasceu.
    pub wander: WanderParams,
}

/// Onde uma criatura nasce.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CreatureSpawn {
    /// Tipo da criatura.
    pub kind: &'static CreatureType,
    /// Coluna (x, z) em blocos, relativa à coluna do spawn dos jogadores.
    pub offset: [i32; 2],
}

/// Pequeno e rápido; para pouco tempo.
pub const COELHO: CreatureType = CreatureType {
    name: "coelho",
    body: BodySpec {
        width: 0.4,
        height: 0.5,
        eye_height: 0.4,
        walk_speed: 3.5,
    },
    wander: WanderParams {
        radius: 5.0,
        min_idle_ticks: TICK_RATE,
        max_idle_ticks: 3 * TICK_RATE,
        max_walk_ticks: 4 * TICK_RATE,
        arrive_distance: 0.3,
    },
};

/// Grande e lenta; fica parada mais tempo.
pub const OVELHA: CreatureType = CreatureType {
    name: "ovelha",
    body: BodySpec {
        width: 0.9,
        height: 1.3,
        eye_height: 1.1,
        walk_speed: 1.8,
    },
    wander: WanderParams {
        radius: 7.0,
        min_idle_ticks: 2 * TICK_RATE,
        max_idle_ticks: 6 * TICK_RATE,
        max_walk_ticks: 8 * TICK_RATE,
        arrive_distance: 0.4,
    },
};

/// Todos os tipos de criatura.
pub const CREATURE_TYPES: &[&CreatureType] = &[&COELHO, &OVELHA];

/// Criaturas que nascem com o mundo, perto do spawn.
pub const CREATURE_SPAWNS: &[CreatureSpawn] = &[
    CreatureSpawn {
        kind: &COELHO,
        offset: [6, 4],
    },
    CreatureSpawn {
        kind: &COELHO,
        offset: [-5, 8],
    },
    CreatureSpawn {
        kind: &OVELHA,
        offset: [-8, -6],
    },
    CreatureSpawn {
        kind: &OVELHA,
        offset: [10, -9],
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creature_names_are_unique() {
        for (i, a) in CREATURE_TYPES.iter().enumerate() {
            for b in &CREATURE_TYPES[i + 1..] {
                assert_ne!(a.name, b.name);
            }
        }
    }

    #[test]
    fn creature_bodies_are_sane() {
        for t in CREATURE_TYPES {
            let b = t.body;
            assert!(
                b.width > 0.0 && b.width < 1.0,
                "{}: cabe em vão de 1 bloco",
                t.name
            );
            assert!(b.eye_height > 0.0 && b.eye_height <= b.height, "{}", t.name);
            assert!(b.walk_speed > 0.0, "{}", t.name);
            assert!(
                t.wander.min_idle_ticks <= t.wander.max_idle_ticks,
                "{}",
                t.name
            );
        }
    }

    #[test]
    fn every_spawn_uses_a_listed_type() {
        for s in CREATURE_SPAWNS {
            assert!(CREATURE_TYPES.iter().any(|t| t.name == s.kind.name));
        }
    }
}
