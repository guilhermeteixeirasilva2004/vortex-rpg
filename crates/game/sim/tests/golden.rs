//! Golden da simulação: roteiro fixo de inputs, posições fixadas bit a bit
//! (via `{:?}` do f32, que é exato). Protege refatorações contra mudança de
//! comportamento. As fixtures em `golden/` foram geradas antes da extração
//! deste crate e não devem ser regeneradas sem uma mudança de gameplay
//! intencional.

use std::fmt::Write;

use engine_physics::MoveIntent;
use engine_voxel::HeightmapGenerator;
use game_sim::{Command, InputFrame, PlayerId, SimConfig, SimEvent, Simulation};

/// Mesmo mundo que o servidor gera.
fn sim(allow_fly: bool) -> Simulation {
    let config = SimConfig {
        world_radius_chunks: 4,
        world_height_chunks: 2,
        allow_fly,
    };
    Simulation::new(&HeightmapGenerator, config)
}

/// Como o protocolo traduz o input do fio (`up` vira pulo ou eixo de voo).
fn frame(
    seq: u32,
    forward: f32,
    right: f32,
    up: f32,
    fly: bool,
    yaw: f32,
    pitch: f32,
) -> InputFrame {
    InputFrame {
        seq,
        intent: MoveIntent {
            forward,
            right,
            yaw,
            jump: up > 0.5,
            fly,
            fly_vertical: up,
        },
        pitch,
    }
}

/// (forward, right, up, fly, yaw) de cada jogador no tick `t`.
fn script(id: PlayerId, t: u32) -> (f32, f32, f32, bool, f32) {
    match id {
        // Anda, pula segurando Espaço, voa, sai do voo e cai.
        1 => match t {
            0..20 => (0.0, 0.0, 0.0, false, 0.0),
            20..60 => (
                1.0,
                0.0,
                if (30..36).contains(&t) { 1.0 } else { 0.0 },
                false,
                0.3,
            ),
            60..80 => (0.0, 1.0, 1.0, true, 0.3),
            _ => (1.0, 1.0, 0.0, false, 2.0),
        },
        // Anda na diagonal sem parar (sobe/desce terreno, bate em degraus).
        2 => (1.0, -1.0, if t % 40 < 5 { 1.0 } else { 0.0 }, false, 4.0),
        // Voa para baixo atravessando o terreno até cair no vazio.
        3 => (0.0, 0.0, -1.0, true, 0.0),
        // Pede voo; no mundo sem voo isso deve ser ignorado.
        _ => (0.5, 0.0, 1.0, true, 5.5),
    }
}

fn run(allow_fly: bool) -> (String, Vec<SimEvent>) {
    let mut s = sim(allow_fly);
    for id in 1..=4 {
        s.apply(Command::Join { id });
    }
    let mut out = String::new();
    let mut events = Vec::new();
    for t in 0..240u32 {
        for id in 1..=4 {
            let (forward, right, up, fly, yaw) = script(id, t);
            let input = frame(t + 1, forward, right, up, fly, yaw, 0.1);
            s.apply(Command::Input { id, input });
        }
        // Pacote atrasado (seq antigo) deve ser ignorado.
        if t > 0 {
            let input = frame(t, -1.0, 0.0, 0.0, false, 0.0, 0.0);
            s.apply(Command::Input { id: 2, input });
        }
        events.extend(s.tick());
        if t % 20 == 19 {
            let mut snap = s.snapshot();
            snap.players.sort_by_key(|p| p.id);
            for p in snap.players {
                writeln!(
                    out,
                    "{} {} {:?} yaw={:?} pitch={:?} fly={} seq={}",
                    snap.tick, p.id, p.position, p.yaw, p.pitch, p.flying, p.last_input_seq
                )
                .unwrap();
            }
        }
    }
    (out, events)
}

#[test]
fn simulation_with_fly_allowed() {
    let (out, events) = run(true);
    assert_eq!(out, include_str!("golden/sim_fly.txt"));
    assert_eq!(events, [SimEvent::FellIntoVoid(3)]);
}

#[test]
fn simulation_with_fly_denied() {
    let (out, events) = run(false);
    assert_eq!(out, include_str!("golden/sim_nofly.txt"));
    assert!(events.is_empty());
}

#[test]
fn fly_requires_permission() {
    let mut s = sim(false);
    s.apply(Command::Join { id: 1 });
    let input = frame(1, 0.0, 0.0, 1.0, true, 0.0, 0.0);
    s.apply(Command::Input { id: 1, input });
    s.tick();
    assert!(!s.snapshot().players[0].flying);
}

#[test]
fn leave_removes_player() {
    let mut s = sim(true);
    s.apply(Command::Join { id: 7 });
    assert!(s.has_players());
    s.apply(Command::Leave { id: 7 });
    assert!(!s.has_players());
    assert!(s.snapshot().players.is_empty());
}
