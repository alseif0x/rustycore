//! Fail-closed tests for the issue-#106 `SMSG_UPDATE_OBJECT` cadence filter.
//!
//! Two independent C++ captures emitted the same 51-byte one-player VALUES
//! body: UnitData contained only parent bit 116 (`Power`) with no child and no
//! payload, followed by one ActivePlayer InvSlots value. Rust emitted the same
//! update without that empty parent in 46 bytes. The comparator may remove only
//! that C++-side five-byte mask fragment; this is capture cadence noise, not
//! evidence that power-regeneration gameplay is equivalent.

use capture_diff::diff::DiffReport;
use capture_diff::model::{Capture, CapturedPacket, Direction};
use capture_diff::semantic::SMSG_UPDATE_OBJECT;

const REAL_CPP_BODY: [u8; 51] = [
    0x01, 0x00, 0x00, 0x00, 0x12, 0x02, 0x00, 0x28, 0x00, 0x00, 0x00, 0x00, 0x01, 0xA0, 0x0F, 0x04,
    0x08, 0x1E, 0x00, 0x00, 0x00, 0xA0, 0x00, 0x00, 0x00, 0x08, 0x00, 0x10, 0x00, 0x00, 0x88, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x10, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x80, 0x07, 0xA0, 0xFA, 0x84,
    0x1E, 0x04, 0x0C,
];
const REAL_RUST_BODY: [u8; 46] = [
    0x01, 0x00, 0x00, 0x00, 0x12, 0x02, 0x00, 0x23, 0x00, 0x00, 0x00, 0x00, 0x01, 0xA0, 0x0F, 0x04,
    0x08, 0x19, 0x00, 0x00, 0x00, 0x80, 0x00, 0x00, 0x00, 0x88, 0x00, 0x00, 0x00, 0x00, 0x00, 0x10,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x80, 0x07, 0xA0, 0xFA, 0x84, 0x1E, 0x04, 0x0C,
];

fn packet(direction: Direction, connection_id: u32, opcode: u16, body: Vec<u8>) -> CapturedPacket {
    CapturedPacket {
        direction,
        connection_id,
        opcode,
        body,
    }
}

fn report(cpp: CapturedPacket, rust: CapturedPacket, direction: Direction) -> DiffReport {
    DiffReport::compute(
        &Capture::new("cpp", vec![cpp]),
        &Capture::new("rust", vec![rust]),
        &[direction],
    )
}

fn s2c_report(cpp: Vec<u8>, rust: Vec<u8>) -> DiffReport {
    report(
        packet(Direction::S2C, 1, SMSG_UPDATE_OBJECT, cpp),
        packet(Direction::S2C, 1, SMSG_UPDATE_OBJECT, rust),
        Direction::S2C,
    )
}

fn semantic(report: &DiffReport) -> &capture_diff::SemanticBodyDiff {
    report.ops[0]
        .body
        .as_ref()
        .and_then(|body| body.semantic.as_ref())
        .expect("reviewed UpdateObject comparator")
}

fn set_u32_le(body: &mut [u8], offset: usize, value: u32) {
    body[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

#[test]
fn real_51_and_46_byte_bodies_remove_only_cpp_empty_unit_power_parent() {
    let report = s2c_report(REAL_CPP_BODY.to_vec(), REAL_RUST_BODY.to_vec());

    assert!(report.is_clean(), "{}", report.render_text());
    assert_eq!(report.counts.matched, 1);
    let body = report.ops[0].body.as_ref().expect("body comparison");
    assert_eq!(body.cpp_len, 51);
    assert_eq!(body.rust_len, 46);
    assert_eq!(body.first_diff_offset, None);
    let semantic = semantic(&report);
    assert_eq!(
        semantic.comparator,
        "smsg_update_object_without_cpp_empty_unit_power_parent"
    );
    assert_eq!(semantic.cpp.raw_body_sha256, None);
    assert_eq!(semantic.rust.raw_body_sha256, None);

    let cpp = semantic
        .cpp
        .update_object_inv_slots
        .as_ref()
        .expect("decoded C++ update");
    let rust = semantic
        .rust
        .update_object_inv_slots
        .as_ref()
        .expect("decoded Rust update");
    assert_eq!(cpp, rust);
    assert_eq!(cpp.map_id, 530);
    assert_eq!(cpp.player.low, 15);
    assert_eq!(cpp.player.high, 0x0800_0400_0000_0000);
    assert_eq!(cpp.inv_slots.len(), 1);
    assert_eq!(cpp.inv_slots[0].slot, 106);
    assert_eq!(cpp.inv_slots[0].item.low, 0x001E_84FA);
    assert_eq!(cpp.inv_slots[0].item.high, 0x0C00_0400_0000_0000);
}

#[test]
fn map_and_player_identity_remain_strict() {
    let mut changed_map = REAL_RUST_BODY.to_vec();
    changed_map[4] = 0x13;
    let map_report = s2c_report(REAL_CPP_BODY.to_vec(), changed_map);
    assert!(!map_report.is_clean());
    assert!(semantic(&map_report).mismatch_summary().contains("map_id"));

    let mut changed_player = REAL_RUST_BODY.to_vec();
    changed_player[14] = 0x10;
    let player_report = s2c_report(REAL_CPP_BODY.to_vec(), changed_player);
    assert!(!player_report.is_clean());
    assert!(
        semantic(&player_report)
            .mismatch_summary()
            .contains("player")
    );
}

#[test]
fn inv_slot_and_item_value_remain_strict() {
    let mut changed_slot = REAL_RUST_BODY.to_vec();
    // ActivePlayer block 7: field 231 / slot 106 -> field 232 / slot 107.
    changed_slot[37] = 0x01;
    changed_slot[38] = 0x00;
    let slot_report = s2c_report(REAL_CPP_BODY.to_vec(), changed_slot);
    assert!(!slot_report.is_clean());
    assert!(
        semantic(&slot_report)
            .mismatch_summary()
            .contains("inv_slots.slot")
    );

    let mut changed_item = REAL_RUST_BODY.to_vec();
    changed_item[41] = 0xFB;
    let item_report = s2c_report(REAL_CPP_BODY.to_vec(), changed_item);
    assert!(!item_report.is_clean());
    assert!(
        semantic(&item_report)
            .mismatch_summary()
            .contains("inv_slots.item")
    );
}

#[test]
fn destroy_out_of_range_and_noncanonical_outer_bit_padding_fail() {
    let mut destroy = REAL_RUST_BODY.to_vec();
    destroy[6] = 0x80;
    let report = s2c_report(REAL_CPP_BODY.to_vec(), destroy);
    assert!(!report.is_clean());
    assert!(semantic(&report).rust.decode_error.is_some());

    let mut padding = REAL_RUST_BODY.to_vec();
    padding[6] = 0x01;
    let report = s2c_report(REAL_CPP_BODY.to_vec(), padding);
    assert!(!report.is_clean());
    assert!(
        semantic(&report)
            .rust
            .decode_error
            .as_deref()
            .unwrap()
            .contains("padding")
    );
}

#[test]
fn another_object_type_parent_or_active_mask_fails() {
    let mut another_type = REAL_RUST_BODY.to_vec();
    another_type[21] = 0x81;
    let report = s2c_report(REAL_CPP_BODY.to_vec(), another_type);
    assert!(!report.is_clean());
    assert!(semantic(&report).rust.decode_error.is_some());

    // Add ActivePlayer block 0 / field 0 alongside the real InvSlots blocks.
    let mut another_active_mask = REAL_RUST_BODY.to_vec();
    another_active_mask[25] = 0x89;
    another_active_mask.splice(31..31, [0x00, 0x00, 0x00, 0x01]);
    set_u32_le(&mut another_active_mask, 7, 39);
    set_u32_le(&mut another_active_mask, 17, 29);
    let report = s2c_report(REAL_CPP_BODY.to_vec(), another_active_mask);
    assert!(!report.is_clean());
    assert!(
        semantic(&report)
            .rust
            .decode_error
            .as_deref()
            .unwrap()
            .contains("non-InvSlots")
    );
}

#[test]
fn any_unit_child_other_parent_or_payload_fails() {
    let mut another_parent = REAL_CPP_BODY.to_vec();
    another_parent[27] = 0x08;
    let report = s2c_report(another_parent, REAL_RUST_BODY.to_vec());
    assert!(!report.is_clean());
    assert!(
        semantic(&report)
            .cpp
            .decode_error
            .as_deref()
            .unwrap()
            .contains("parent bit 116")
    );

    // Add child bit 117 and a four-byte field payload before ActivePlayer.
    let mut child_and_payload = REAL_CPP_BODY.to_vec();
    child_and_payload[27] = 0x30;
    child_and_payload.splice(30..30, [0x00, 0x00, 0x00, 0x00]);
    set_u32_le(&mut child_and_payload, 7, 44);
    set_u32_le(&mut child_and_payload, 17, 34);
    let report = s2c_report(child_and_payload, REAL_RUST_BODY.to_vec());
    assert!(!report.is_clean());
    assert!(
        semantic(&report)
            .cpp
            .decode_error
            .as_deref()
            .unwrap()
            .contains("parent bit 116")
    );

    // Even payload bytes without a child mask cannot be swallowed.
    let mut payload_only = REAL_CPP_BODY.to_vec();
    payload_only.splice(30..30, [0xAA, 0xBB, 0xCC, 0xDD]);
    set_u32_le(&mut payload_only, 7, 44);
    set_u32_le(&mut payload_only, 17, 34);
    let report = s2c_report(payload_only, REAL_RUST_BODY.to_vec());
    assert!(!report.is_clean());
    assert!(semantic(&report).cpp.decode_error.is_some());
}

#[test]
fn length_truncation_trailing_bytes_and_packed_guid_noncanonicality_fail() {
    let mut bad_outer_len = REAL_RUST_BODY.to_vec();
    set_u32_le(&mut bad_outer_len, 7, 34);
    let report = s2c_report(REAL_CPP_BODY.to_vec(), bad_outer_len);
    assert!(!report.is_clean());
    assert!(semantic(&report).rust.decode_error.is_some());

    let mut bad_values_len = REAL_RUST_BODY.to_vec();
    set_u32_le(&mut bad_values_len, 17, 24);
    let report = s2c_report(REAL_CPP_BODY.to_vec(), bad_values_len);
    assert!(!report.is_clean());
    assert!(semantic(&report).rust.decode_error.is_some());

    let mut truncated = REAL_RUST_BODY.to_vec();
    truncated.pop();
    let report = s2c_report(REAL_CPP_BODY.to_vec(), truncated);
    assert!(!report.is_clean());
    assert!(semantic(&report).rust.decode_error.is_some());

    let mut trailing = REAL_RUST_BODY.to_vec();
    trailing.push(0xAA);
    let report = s2c_report(REAL_CPP_BODY.to_vec(), trailing);
    assert!(!report.is_clean());
    assert!(semantic(&report).rust.decode_error.is_some());

    let mut bad_player_packing = REAL_RUST_BODY.to_vec();
    bad_player_packing[14] = 0;
    let report = s2c_report(REAL_CPP_BODY.to_vec(), bad_player_packing);
    assert!(!report.is_clean());
    assert!(
        semantic(&report)
            .rust
            .decode_error
            .as_deref()
            .unwrap()
            .contains("non-canonical")
    );

    let mut bad_item_packing = REAL_RUST_BODY.to_vec();
    bad_item_packing[41] = 0;
    let report = s2c_report(REAL_CPP_BODY.to_vec(), bad_item_packing);
    assert!(!report.is_clean());
    assert!(
        semantic(&report)
            .rust
            .decode_error
            .as_deref()
            .unwrap()
            .contains("non-canonical")
    );
}

#[test]
fn player_and_item_global_guid_reserved_high_bits_fail_without_length_changes() {
    // ObjectGuidFactory::CreatePlayer uses only HighGuid bits 58..63 and realm
    // bits 42..57. Changing byte 5 from 0x04 to 0x05 sets forbidden bit 40
    // while preserving the packed length and HighGuid::Player.
    let mut cpp_bad_player = REAL_CPP_BODY.to_vec();
    cpp_bad_player[15] = 0x05;
    let mut rust_bad_player = REAL_RUST_BODY.to_vec();
    rust_bad_player[15] = 0x05;
    let report = s2c_report(cpp_bad_player, rust_bad_player);
    assert!(!report.is_clean());
    assert!(
        semantic(&report)
            .cpp
            .decode_error
            .as_deref()
            .unwrap()
            .contains("reserved high-word bits 0..41")
    );
    assert!(
        semantic(&report)
            .rust
            .decode_error
            .as_deref()
            .unwrap()
            .contains("reserved high-word bits 0..41")
    );

    // ObjectGuidFactory::CreateItem has the same high-word layout. These are
    // the matching high-byte positions in the literal 51/46-byte bodies.
    let mut cpp_bad_item = REAL_CPP_BODY.to_vec();
    cpp_bad_item[49] = 0x05;
    let mut rust_bad_item = REAL_RUST_BODY.to_vec();
    rust_bad_item[44] = 0x05;
    let report = s2c_report(cpp_bad_item, rust_bad_item);
    assert!(!report.is_clean());
    assert!(
        semantic(&report)
            .cpp
            .decode_error
            .as_deref()
            .unwrap()
            .contains("reserved high-word bits 0..41")
    );
    assert!(
        semantic(&report)
            .rust
            .decode_error
            .as_deref()
            .unwrap()
            .contains("reserved high-word bits 0..41")
    );
}

#[test]
fn identical_malformed_candidate_is_never_capture_clean() {
    let mut malformed = REAL_RUST_BODY.to_vec();
    malformed[14] = 0;
    let report = s2c_report(malformed.clone(), malformed);

    assert!(!report.is_clean());
    assert_eq!(report.counts.body_mismatches, 1);
    assert!(semantic(&report).cpp.decode_error.is_some());
    assert!(semantic(&report).rust.decode_error.is_some());
    assert!(semantic(&report).cpp.raw_body_sha256.is_some());
    assert!(semantic(&report).rust.raw_body_sha256.is_some());
}

#[test]
fn mixed_not_eligible_and_malformed_shapes_keep_raw_comparison_in_both_orientations() {
    let mut not_eligible = REAL_RUST_BODY.to_vec();
    set_u32_le(&mut not_eligible, 0, 2);
    let mut malformed = REAL_RUST_BODY.to_vec();
    malformed[14] = 0;

    for report in [
        s2c_report(not_eligible.clone(), malformed.clone()),
        s2c_report(malformed.clone(), not_eligible.clone()),
    ] {
        assert!(!report.is_clean());
        assert_eq!(report.counts.body_mismatches, 1);
        let body = report.ops[0].body.as_ref().expect("raw body comparison");
        assert!(body.semantic.is_none());
        assert!(body.first_diff_offset.is_some());
    }
}

#[test]
fn cpp_only_orientation_is_mandatory() {
    let report = s2c_report(REAL_RUST_BODY.to_vec(), REAL_CPP_BODY.to_vec());

    assert!(!report.is_clean());
    assert!(semantic(&report).mismatch_summary().contains("C++-only"));
}

#[test]
fn create_object_and_other_update_object_shapes_receive_no_normalization() {
    let mut cpp_create = REAL_CPP_BODY.to_vec();
    let mut rust_create = REAL_RUST_BODY.to_vec();
    cpp_create[11] = 1;
    rust_create[11] = 1;
    let report = s2c_report(cpp_create, rust_create);
    assert!(!report.is_clean());
    assert!(report.ops[0].body.as_ref().unwrap().semantic.is_none());

    let mut cpp_many = REAL_CPP_BODY.to_vec();
    let mut rust_many = REAL_RUST_BODY.to_vec();
    set_u32_le(&mut cpp_many, 0, 2);
    set_u32_le(&mut rust_many, 0, 2);
    let report = s2c_report(cpp_many, rust_many);
    assert!(!report.is_clean());
    assert!(report.ops[0].body.as_ref().unwrap().semantic.is_none());
}

#[test]
fn same_side_shape_receives_no_normalization() {
    let mut changed = REAL_CPP_BODY.to_vec();
    changed[14] = 0x10;
    let report = s2c_report(REAL_CPP_BODY.to_vec(), changed);

    assert!(!report.is_clean());
    assert!(report.ops[0].body.as_ref().unwrap().semantic.is_none());
}

#[test]
fn opcode_direction_and_instance_routing_remain_strict() {
    let wrong_opcode = report(
        packet(
            Direction::S2C,
            1,
            SMSG_UPDATE_OBJECT - 1,
            REAL_CPP_BODY.to_vec(),
        ),
        packet(
            Direction::S2C,
            1,
            SMSG_UPDATE_OBJECT - 1,
            REAL_RUST_BODY.to_vec(),
        ),
        Direction::S2C,
    );
    assert!(!wrong_opcode.is_clean());
    assert!(
        wrong_opcode.ops[0]
            .body
            .as_ref()
            .unwrap()
            .semantic
            .is_none()
    );

    let wrong_direction = report(
        packet(
            Direction::C2S,
            1,
            SMSG_UPDATE_OBJECT,
            REAL_CPP_BODY.to_vec(),
        ),
        packet(
            Direction::C2S,
            1,
            SMSG_UPDATE_OBJECT,
            REAL_RUST_BODY.to_vec(),
        ),
        Direction::C2S,
    );
    assert!(!wrong_direction.is_clean());
    assert!(
        wrong_direction.ops[0]
            .body
            .as_ref()
            .unwrap()
            .semantic
            .is_none()
    );

    let wrong_route = report(
        packet(
            Direction::S2C,
            1,
            SMSG_UPDATE_OBJECT,
            REAL_CPP_BODY.to_vec(),
        ),
        packet(
            Direction::S2C,
            0,
            SMSG_UPDATE_OBJECT,
            REAL_RUST_BODY.to_vec(),
        ),
        Direction::S2C,
    );
    assert!(!wrong_route.is_clean());
    assert_eq!(wrong_route.counts.body_mismatches, 0);
    assert_eq!(wrong_route.counts.connection_mismatches, 1);
}

#[test]
fn semantic_mismatch_baseline_keeps_all_stable_values() {
    let mut changed_slot = REAL_RUST_BODY.to_vec();
    changed_slot[37] = 0x01;
    changed_slot[38] = 0x00;
    let slot_signature = s2c_report(REAL_CPP_BODY.to_vec(), changed_slot).signatures();

    let mut changed_item = REAL_RUST_BODY.to_vec();
    changed_item[41] = 0xFB;
    let item_signature = s2c_report(REAL_CPP_BODY.to_vec(), changed_item).signatures();

    assert_eq!(slot_signature.len(), 1);
    assert_eq!(item_signature.len(), 1);
    assert_ne!(slot_signature, item_signature);
    assert_eq!(slot_signature[0].cpp_body_len, None);
    assert_eq!(slot_signature[0].rust_body_len, None);
    assert_eq!(slot_signature[0].first_diff_offset, None);
}
