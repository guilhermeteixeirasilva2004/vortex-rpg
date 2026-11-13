//! Testes "golden": fixam o JSON exato de cada mensagem. Qualquer refatoração
//! precisa manter estes testes passando sem editar as strings esperadas.

use engine_voxel::{Block, Chunk, ChunkPos};
use game_protocol::*;
use game_sim::{PlayerSnapshot, Snapshot};

fn input() -> PlayerInput {
    PlayerInput {
        seq: 42,
        forward: 1.0,
        right: -0.5,
        up: 1.0,
        fly: true,
        yaw: 1.25,
        pitch: -0.75,
    }
}

#[test]
fn client_input() {
    let json = encode(&ClientMessage::Input(input())).unwrap();
    assert_eq!(
        json,
        r#"{"Input":{"seq":42,"forward":1.0,"right":-0.5,"up":1.0,"fly":true,"yaw":1.25,"pitch":-0.75}}"#
    );
}

#[test]
fn client_input_without_fly_decodes_as_walking() {
    // Clientes antigos não mandam `fly`.
    let json = r#"{"Input":{"seq":7,"forward":0.0,"right":0.0,"up":1.0,"yaw":0.0,"pitch":0.0}}"#;
    let ClientMessage::Input(i) = decode::<ClientMessage>(json).unwrap();
    assert_eq!(i.seq, 7);
    assert_eq!(i.up, 1.0);
    assert!(!i.fly);
}

#[test]
fn server_welcome() {
    let json = encode(&ServerMessage::Welcome {
        player_id: 3,
        spawn: [0.5, 17.5, -0.25],
    })
    .unwrap();
    assert_eq!(
        json,
        r#"{"Welcome":{"player_id":3,"spawn":[0.5,17.5,-0.25]}}"#
    );
}

#[test]
fn server_snapshot() {
    let json = encode(&ServerMessage::Snapshot {
        tick: 99,
        players: vec![PlayerState {
            id: 1,
            position: [1.5, 2.0, -3.25],
            yaw: 0.5,
            pitch: 0.25,
            flying: true,
            last_input_seq: 12,
        }],
    })
    .unwrap();
    assert_eq!(
        json,
        r#"{"Snapshot":{"tick":99,"players":[{"id":1,"position":[1.5,2.0,-3.25],"yaw":0.5,"pitch":0.25,"flying":true,"last_input_seq":12}]}}"#
    );
}

#[test]
fn server_snapshot_from_domain() {
    let snapshot = Snapshot {
        tick: 99,
        players: vec![PlayerSnapshot {
            id: 1,
            position: [1.5, 2.0, -3.25],
            yaw: 0.5,
            pitch: 0.25,
            flying: true,
            last_input_seq: 12,
        }],
        creatures: vec![],
    };
    let json = encode(&ServerMessage::from(&snapshot)).unwrap();
    assert_eq!(
        json,
        r#"{"Snapshot":{"tick":99,"players":[{"id":1,"position":[1.5,2.0,-3.25],"yaw":0.5,"pitch":0.25,"flying":true,"last_input_seq":12}]}}"#
    );
}

#[test]
fn snapshot_without_flying_decodes() {
    let json = r#"{"Snapshot":{"tick":1,"players":[{"id":1,"position":[0.0,0.0,0.0],"yaw":0.0,"pitch":0.0,"last_input_seq":0}]}}"#;
    let ServerMessage::Snapshot { players, .. } = decode::<ServerMessage>(json).unwrap() else {
        panic!("esperava Snapshot");
    };
    assert!(!players[0].flying);
}

#[test]
fn server_chunk_data() {
    let mut chunk = Chunk::empty(ChunkPos::new(1, -2, 3));
    chunk.set(0, 0, 0, Block::Grass);
    chunk.set(1, 0, 0, Block::Dirt);
    chunk.set(0, 0, 1, Block::Stone); // índice 16
    chunk.set(15, 15, 15, Block::Grass); // último índice
    let json = encode(&ServerMessage::ChunkData(ChunkDto::from(&chunk))).unwrap();

    let mut blocks = vec!["0"; 4096];
    blocks[0] = "1";
    blocks[1] = "2";
    blocks[16] = "3";
    blocks[4095] = "1";
    let expected = format!(
        r#"{{"ChunkData":{{"pos":{{"x":1,"y":-2,"z":3}},"blocks":[{}]}}}}"#,
        blocks.join(",")
    );
    assert_eq!(json, expected);

    let ServerMessage::ChunkData(back) = decode::<ServerMessage>(&json).unwrap() else {
        panic!("esperava ChunkData");
    };
    let back = Chunk::try_from(back).unwrap();
    assert_eq!(back, chunk);
    assert_eq!(back.get(0, 0, 1), Some(Block::Stone));
}
