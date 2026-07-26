//! Focused issue-#24 capture semantics for the connected synthetic Detour
//! obstacle fixture.

use capture_diff::semantic::{
    CMSG_MOVE_HEARTBEAT, CMSG_PING, MonsterMoveFaceBody, SMSG_ON_MONSTER_MOVE,
    compare_packet_bodies, decode_monster_move_body, validate_detour_chase_capture,
    validate_detour_chase_monster_move,
};
use capture_diff::{Capture, CapturedPacket, Direction, RequirementStatus, load_requirement};

const START: [f32; 3] = [-10_118.333, 2_671.667, 218.49];
const DESTINATION: [f32; 3] = [-10_118.333, 2_691.667, 218.49];
const FIXTURE_SPAWN_GUID: u64 = 9_102_401;
// Packet 89 of the raw C++ capture named in wow-packet's existing
// `monster_move_matches_real_cpp_waypoint_capture_bytes` regression. Keeping
// one literal C++ body here verifies this independent decoder is not merely
// self-consistent with the synthetic body builder below.
const REAL_CPP_COMPRESSED_WAYPOINT_BODY: &[u8] = &[
    0x03, 0xBF, 0x0B, 0x01, 0x40, 0x3E, 0x15, 0x40, 0x42, 0x04, 0x20, 0xEC, 0xB9, 0x27, 0xC5, 0xE1,
    0xD2, 0x1F, 0x45, 0x12, 0xE5, 0x95, 0x42, 0x06, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x30, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x11, 0x3F, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xFF, 0x00, 0x00, 0x40, 0x00,
    0xA0, 0x33, 0xB7, 0x27, 0xC5, 0xA4, 0x58, 0x22, 0x45, 0x04, 0xDB, 0x95, 0x42, 0x00, 0x00, 0x02,
    0x00, 0x00, 0x80, 0x01, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x80, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x88, 0x3F, 0x00, 0x00, 0x08, 0x3F, 0x00, 0x00, 0x88, 0x3E, 0x00, 0x00, 0x08, 0x3E,
    0x00, 0x00, 0x88, 0x3D, 0x00,
];

fn pack_u64(value: u64) -> (u8, Vec<u8>) {
    let bytes = value.to_le_bytes();
    let mut mask = 0u8;
    let mut packed = Vec::new();
    for (index, byte) in bytes.into_iter().enumerate() {
        if byte != 0 {
            mask |= 1 << index;
            packed.push(byte);
        }
    }
    (mask, packed)
}

fn packed_guid(low: u64, high: u64) -> Vec<u8> {
    let (low_mask, low_bytes) = pack_u64(low);
    let (high_mask, high_bytes) = pack_u64(high);
    let mut body = vec![low_mask, high_mask];
    body.extend(low_bytes);
    body.extend(high_bytes);
    body
}

fn fixture_high(entry: u32) -> u64 {
    (8u64 << 58) | (1u64 << 42) | (1u64 << 29) | (u64::from(entry) << 6)
}

fn packed_xyz(x: f32, y: f32, z: f32) -> u32 {
    ((x / 0.25) as i32 as u32 & 0x7FF)
        | (((y / 0.25) as i32 as u32 & 0x7FF) << 11)
        | (((z / 0.25) as i32 as u32 & 0x3FF) << 22)
}

fn fixture_monster_move_with(counter: u64, spline_id: u32, flags: u32, deltas: &[u32]) -> Vec<u8> {
    let mut body = packed_guid(counter, fixture_high(15_271));
    for value in START {
        body.extend_from_slice(&value.to_le_bytes());
    }
    body.extend_from_slice(&spline_id.to_le_bytes());
    body.extend_from_slice(&[0; 12]); // C++ leaves the outer destination zero.
    body.push(0); // CrzTeleport=false, tolerance=0, canonical padding.
    body.extend_from_slice(&flags.to_le_bytes());
    body.extend_from_slice(&0i32.to_le_bytes());
    body.extend_from_slice(&5_000u32.to_le_bytes());
    body.extend_from_slice(&0u32.to_le_bytes());
    body.push(0); // mode
    body.extend_from_slice(&[0, 0]); // empty transport GUID masks
    body.push(0xFF); // vehicle seat -1

    let header = (2u64 << 38) | (1u64 << 22) | ((deltas.len() as u64) << 4);
    body.extend_from_slice(&header.to_be_bytes()[3..]); // exact five-byte bit header
    body.extend_from_slice(&std::f32::consts::FRAC_PI_2.to_le_bytes());
    body.extend(packed_guid(15, (2u64 << 58) | (1u64 << 42)));
    for value in DESTINATION {
        body.extend_from_slice(&value.to_le_bytes());
    }
    for delta in deltas {
        body.extend_from_slice(&delta.to_le_bytes());
    }
    body
}

fn fixture_monster_move(counter: u64, spline_id: u32) -> Vec<u8> {
    fixture_monster_move_with(
        counter,
        spline_id,
        0x0030_0000,
        &[packed_xyz(6.0, 5.0, 0.0), packed_xyz(6.0, -5.0, 0.0)],
    )
}

fn replace_fixture_current_x(body: &mut [u8], counter: u64, x: f32) {
    let offset = packed_guid(counter, fixture_high(15_271)).len();
    body[offset..offset + 4].copy_from_slice(&x.to_le_bytes());
}

fn heartbeat() -> Vec<u8> {
    let player_high = (2u64 << 58) | (1u64 << 42);
    let mut body = packed_guid(15, player_high);
    body.extend_from_slice(&[0; 16]); // flags x3 + client time
    for value in DESTINATION {
        body.extend_from_slice(&value.to_le_bytes());
    }
    body.extend_from_slice(&(-std::f32::consts::FRAC_PI_2).to_le_bytes());
    body.extend_from_slice(&[0; 16]); // pitch, step elevation, force count, move index
    body.push(0); // no optional movement sections
    body
}

fn fixture_capture() -> Capture {
    Capture::new(
        "synthetic detour capture",
        vec![
            CapturedPacket {
                direction: Direction::C2S,
                connection_id: 1,
                opcode: CMSG_MOVE_HEARTBEAT,
                body: heartbeat(),
            },
            CapturedPacket {
                direction: Direction::S2C,
                connection_id: 1,
                opcode: SMSG_ON_MONSTER_MOVE,
                body: fixture_monster_move(FIXTURE_SPAWN_GUID, 91),
            },
            CapturedPacket {
                direction: Direction::C2S,
                connection_id: 1,
                opcode: CMSG_PING,
                body: [b'D', b'T', b'O', b'R', 0, 0, 0, 0].to_vec(),
            },
        ],
    )
}

#[test]
fn decoder_matches_a_literal_real_cpp_compressed_waypoint_body() {
    let decoded = decode_monster_move_body(REAL_CPP_COMPRESSED_WAYPOINT_BODY)
        .expect("literal raw C++ MonsterMove must decode");
    assert_eq!(decoded.spline_id, 6);
    assert_eq!(decoded.body.flags, 0x0030_0000);
    assert_eq!(decoded.body.elapsed, 0);
    assert_eq!(decoded.body.move_time, 16_145);
    assert_eq!(decoded.body.points.len(), 1);
    assert_eq!(decoded.body.packed_deltas.len(), 10);
    assert!(decoded.body.spline_filter.is_none());
    assert!(decoded.body.spell_effect_extra.is_none());
    assert!(decoded.body.jump_extra.is_none());
    assert!(decoded.body.anim_tier_transition.is_none());
}

#[test]
fn complete_fixture_body_decodes_and_proves_a_detour() {
    let decoded = decode_monster_move_body(&fixture_monster_move(FIXTURE_SPAWN_GUID, 91)).unwrap();
    assert_eq!(decoded.mover_runtime_counter, FIXTURE_SPAWN_GUID);
    assert_eq!(decoded.spline_id, 91);
    let path = validate_detour_chase_monster_move(&decoded).unwrap();
    assert_eq!(path.len(), 4);
    assert!(path[1][0] < -10_123.333);
    assert!(path[2][0] < -10_123.333);

    let generated_counter = decode_monster_move_body(&fixture_monster_move(1, 91)).unwrap();
    assert!(validate_detour_chase_monster_move(&generated_counter).is_ok());
    let zero_counter = decode_monster_move_body(&fixture_monster_move(0, 91)).unwrap();
    assert!(validate_detour_chase_monster_move(&zero_counter).is_err());

    let mut nonzero_elapsed = decoded.clone();
    nonzero_elapsed.body.elapsed = 1;
    assert!(validate_detour_chase_monster_move(&nonzero_elapsed).is_err());

    let mut negative_zero_destination = decoded;
    negative_zero_destination.body.destination.x_bits = (-0.0f32).to_bits();
    assert!(validate_detour_chase_monster_move(&negative_zero_destination).is_err());

    let mut normal_facing =
        decode_monster_move_body(&fixture_monster_move(FIXTURE_SPAWN_GUID, 91)).unwrap();
    normal_facing.body.face = MonsterMoveFaceBody::Normal;
    assert!(
        validate_detour_chase_monster_move(&normal_facing)
            .unwrap_err()
            .contains("chase target")
    );

    let mut wrong_target =
        decode_monster_move_body(&fixture_monster_move(FIXTURE_SPAWN_GUID, 91)).unwrap();
    let MonsterMoveFaceBody::Target { target, .. } = &mut wrong_target.body.face else {
        unreachable!("fixture builder always emits facing-target");
    };
    target.low += 1;
    assert!(
        validate_detour_chase_monster_move(&wrong_target)
            .unwrap_err()
            .contains("character 15")
    );
}

#[test]
fn comparator_requires_nonzero_fixture_counter_and_omits_runtime_ids() {
    let cpp = fixture_monster_move(FIXTURE_SPAWN_GUID, 91);
    let rust = fixture_monster_move(FIXTURE_SPAWN_GUID, 4_001);
    let semantic = compare_packet_bodies(Direction::S2C, SMSG_ON_MONSTER_MOVE, &cpp, &rust)
        .expect("exact fixture selects semantic comparator");
    assert!(semantic.is_identical());
    assert!(semantic.cpp.raw_body_sha256.is_none());
    assert!(semantic.rust.raw_body_sha256.is_none());

    let changed = fixture_monster_move_with(
        FIXTURE_SPAWN_GUID,
        4_001,
        0x0030_0001,
        &[packed_xyz(6.0, 5.0, 0.0), packed_xyz(6.0, -5.0, 0.0)],
    );
    let semantic = compare_packet_bodies(Direction::S2C, SMSG_ON_MONSTER_MOVE, &cpp, &changed)
        .expect("fixture comparator");
    assert!(!semantic.is_identical());
    assert!(semantic.mismatch_summary().contains("flags"));

    let zero_spline = fixture_monster_move(FIXTURE_SPAWN_GUID, 0);
    let semantic = compare_packet_bodies(Direction::S2C, SMSG_ON_MONSTER_MOVE, &cpp, &zero_spline)
        .expect("fixture comparator");
    assert!(!semantic.is_identical());
    assert!(semantic.mismatch_summary().contains("zero spline ID"));

    let generated_counter = fixture_monster_move(1, 4_001);
    let semantic =
        compare_packet_bodies(Direction::S2C, SMSG_ON_MONSTER_MOVE, &cpp, &generated_counter)
            .expect("both nonzero process-local counters select the fixture comparator");
    assert!(semantic.is_identical());

    let zero_counter = fixture_monster_move(0, 4_001);
    let semantic = compare_packet_bodies(Direction::S2C, SMSG_ON_MONSTER_MOVE, &cpp, &zero_counter)
        .expect("one valid fixture side selects the fail-closed comparator");
    assert!(!semantic.is_identical());
    assert!(semantic.mismatch_summary().contains("zero runtime GUID counter"));
}

#[test]
fn malformed_trailing_and_non_fixture_bodies_do_not_gain_normalization() {
    let mut trailing = fixture_monster_move(FIXTURE_SPAWN_GUID, 91);
    trailing.push(0);
    assert!(decode_monster_move_body(&trailing).is_err());

    let other = {
        let mut body = fixture_monster_move(FIXTURE_SPAWN_GUID, 91);
        let fixture_high_bytes = fixture_high(15_271).to_le_bytes();
        let other_high_bytes = fixture_high(15_272).to_le_bytes();
        let byte = body
            .iter()
            .position(|value| *value == fixture_high_bytes[1])
            .expect("encoded high byte");
        body[byte] = other_high_bytes[1];
        body
    };
    assert!(compare_packet_bodies(Direction::S2C, SMSG_ON_MONSTER_MOVE, &other, &other).is_none());

    let mut invalid_cpp = fixture_monster_move(FIXTURE_SPAWN_GUID, 91);
    replace_fixture_current_x(&mut invalid_cpp, FIXTURE_SPAWN_GUID, 0.0);
    let mut invalid_rust = fixture_monster_move(FIXTURE_SPAWN_GUID, 4_001);
    replace_fixture_current_x(&mut invalid_rust, FIXTURE_SPAWN_GUID, 0.0);
    assert!(
        compare_packet_bodies(
            Direction::S2C,
            SMSG_ON_MONSTER_MOVE,
            &invalid_cpp,
            &invalid_rust,
        )
        .is_none(),
        "entry/map identity alone must not activate fixture normalization"
    );

    let valid = fixture_monster_move(FIXTURE_SPAWN_GUID, 91);
    let semantic =
        compare_packet_bodies(Direction::S2C, SMSG_ON_MONSTER_MOVE, &valid, &invalid_rust)
            .expect("one valid fixture side must expose the malformed peer");
    assert!(!semantic.is_identical());
    assert!(semantic.mismatch_summary().contains("current position"));
}

#[test]
fn required_contract_rejects_straight_or_wrong_fence_evidence() {
    let capture = fixture_capture();
    validate_detour_chase_capture(&capture).unwrap();

    let mut straight = capture.clone();
    straight.packets[1].body = fixture_monster_move_with(FIXTURE_SPAWN_GUID, 91, 0x0030_0000, &[]);
    assert!(
        validate_detour_chase_capture(&straight)
            .unwrap_err()
            .contains("no packed intermediate")
    );

    let mut wrong_fence = capture;
    wrong_fence.packets[2].body[0] ^= 1;
    assert!(
        validate_detour_chase_capture(&wrong_fence)
            .unwrap_err()
            .contains("PING fence")
    );

    let mut wrong_orientation = fixture_capture();
    let player_high = (2u64 << 58) | (1u64 << 42);
    let orientation_offset = packed_guid(15, player_high).len() + 16 + 12;
    wrong_orientation.packets[0].body[orientation_offset..orientation_offset + 4]
        .copy_from_slice(&0.0f32.to_le_bytes());
    assert!(
        validate_detour_chase_capture(&wrong_orientation)
            .unwrap_err()
            .contains("orientation")
    );
}

#[test]
fn committed_requirement_is_present_but_fail_closed_until_live_pair() {
    let requirement = load_requirement("detour-chase-around-obstacle").unwrap();
    assert_eq!(requirement.status, RequirementStatus::AwaitingRealCaptures);
    requirement.validate_capture(&fixture_capture()).unwrap();
    assert!(requirement.require_ready().is_err());
}
