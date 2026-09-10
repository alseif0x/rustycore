//! Capture-diff CLI regressions.
//!
//! Separated from main.rs under #685.

use crate::*;

use super::*;

fn packet(direction: Direction, opcode: u16) -> capture_diff::CapturedPacket {
    capture_diff::CapturedPacket {
        direction,
        connection_id: 0,
        opcode,
        body: Vec::new(),
    }
}

fn routed_packet(
    direction: Direction,
    connection_id: u32,
    opcode: u16,
) -> capture_diff::CapturedPacket {
    capture_diff::CapturedPacket {
        direction,
        connection_id,
        opcode,
        body: Vec::new(),
    }
}

fn ambient_packet(
    direction: Direction,
    connection_id: u32,
    opcode: u16,
    body: Vec<u8>,
) -> capture_diff::CapturedPacket {
    capture_diff::CapturedPacket {
        direction,
        connection_id,
        opcode,
        body,
    }
}

fn minimal_monster_move_body() -> Vec<u8> {
    let mut body = vec![0x01, 0x00, 0x01]; // canonical non-empty mover GUID
    body.extend_from_slice(&[0; 12]); // current XYZ
    body.extend_from_slice(&[0; 4]); // spline id
    body.extend_from_slice(&[0; 12]); // destination XYZ
    body.push(0); // CrzTeleport + tolerance + zero padding
    body.extend_from_slice(&[0; 17]); // flags/elapsed/time/fade/mode
    body.extend_from_slice(&[0, 0]); // empty transport GUID
    body.push(0xFF); // vehicle seat
    body.extend_from_slice(&[0; 5]); // normal face, zero path/options
    assert!(valid_monster_move_body(&body));
    body
}

fn valid_required_loot_capture(source: &str) -> Capture {
    let pinned = flow::load_flow("loot-single-item-claim").expect("committed loot flow");
    let mut capture = load_capture(&pinned.reference_rust).expect("committed Rust fixture");
    capture.source = source.to_string();
    capture
}

fn reviewed_required_import_opts() -> Opts {
    parse_opts(&[
        "loot-single-item-claim".into(),
        "--from-opcode".into(),
        "c2s:0x3211".into(),
        "--until-opcode".into(),
        "c2s:0x3768".into(),
        "--ignore-opcode".into(),
        "s2c:0x2DD2".into(),
        "--ignore-opcode".into(),
        "c2s:0x3A3D".into(),
        "--ignore-opcode".into(),
        "s2c:0x2DD4".into(),
        "--direction".into(),
        "both".into(),
        "--strict".into(),
    ])
    .unwrap()
}

mod scenarios;
