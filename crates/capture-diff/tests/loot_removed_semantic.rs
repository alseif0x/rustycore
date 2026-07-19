//! Focused tests for the issue-#106 `SMSG_LOOT_REMOVED` normalization.
//!
//! Paired real C++/Rust captures proved that the same creature spawn receives
//! a different map-runtime GUID counter in the two independently started
//! servers. The comparator may omit exactly that lower 40-bit owner counter;
//! every other GUID bit, the complete loot-object GUID, list id, body shape,
//! canonical packed encoding, opcode, direction, and routing remain strict.

use capture_diff::diff::DiffReport;
use capture_diff::model::{Capture, CapturedPacket, Direction};
use capture_diff::semantic::{SMSG_LOOT_REMOVED, decode_loot_removed_body};

const REAL_CPP_BODY: [u8; 19] = [
    0x03, 0xBF, 0x0C, 0x01, 0xC0, 0x44, 0x15, 0x40, 0x42, 0x04, 0x20, 0x01, 0xB8, 0x01, 0x40, 0x42,
    0x04, 0x3C, 0x00,
];
const REAL_RUST_BODY: [u8; 18] = [
    0x01, 0xBF, 0x01, 0xC0, 0x44, 0x15, 0x40, 0x42, 0x04, 0x20, 0x01, 0xB8, 0x01, 0x40, 0x42, 0x04,
    0x3C, 0x00,
];

#[derive(Clone, Copy)]
struct GuidFields {
    high_type: u8,
    realm_id: u16,
    map_id: u16,
    entry: u32,
    subtype: u8,
    server_id: u32,
    counter: u64,
}

impl GuidFields {
    fn doctor(counter: u64) -> Self {
        Self {
            high_type: 8, // HighGuid::Creature
            realm_id: 1,
            map_id: 530,
            entry: 21_779,
            subtype: 0,
            server_id: 0,
            counter,
        }
    }

    fn loot_object(counter: u64) -> Self {
        Self {
            high_type: 15, // HighGuid::LootObject
            realm_id: 1,
            map_id: 530,
            entry: 0,
            subtype: 0,
            server_id: 0,
            counter,
        }
    }

    fn words(self) -> (u64, u64) {
        let high = (u64::from(self.high_type) << 58)
            | (u64::from(self.realm_id) << 42)
            | (u64::from(self.map_id) << 29)
            | (u64::from(self.entry) << 6)
            | u64::from(self.subtype);
        let low = (u64::from(self.server_id) << 40) | self.counter;
        (low, high)
    }
}

fn packed_guid(low: u64, high: u64) -> Vec<u8> {
    let low_bytes = low.to_le_bytes();
    let high_bytes = high.to_le_bytes();
    let mut low_mask = 0u8;
    let mut high_mask = 0u8;
    for (index, byte) in low_bytes.iter().enumerate() {
        if *byte != 0 {
            low_mask |= 1 << index;
        }
    }
    for (index, byte) in high_bytes.iter().enumerate() {
        if *byte != 0 {
            high_mask |= 1 << index;
        }
    }

    let mut body = vec![low_mask, high_mask];
    body.extend(
        low_bytes
            .iter()
            .enumerate()
            .filter_map(|(index, byte)| (low_mask & (1 << index) != 0).then_some(*byte)),
    );
    body.extend(
        high_bytes
            .iter()
            .enumerate()
            .filter_map(|(index, byte)| (high_mask & (1 << index) != 0).then_some(*byte)),
    );
    body
}

fn loot_removed_body(owner: GuidFields, loot_obj: GuidFields, loot_list_id: u8) -> Vec<u8> {
    let (owner_low, owner_high) = owner.words();
    let (loot_low, loot_high) = loot_obj.words();
    let mut body = packed_guid(owner_low, owner_high);
    body.extend(packed_guid(loot_low, loot_high));
    body.push(loot_list_id);
    body
}

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

fn s2c_report(cpp_body: Vec<u8>, rust_body: Vec<u8>) -> DiffReport {
    report(
        packet(Direction::S2C, 1, SMSG_LOOT_REMOVED, cpp_body),
        packet(Direction::S2C, 1, SMSG_LOOT_REMOVED, rust_body),
        Direction::S2C,
    )
}

#[test]
fn decoder_pins_real_cpp_loot_removed_bytes_and_field_order() {
    let decoded = decode_loot_removed_body(&REAL_CPP_BODY).expect("decode real C++ body");
    assert_eq!(decoded.owner.high_type, 8);
    assert_eq!(decoded.owner.realm_id, 1);
    assert_eq!(decoded.owner.map_id, 530);
    assert_eq!(decoded.owner.entry, 21_779);
    assert_eq!(decoded.owner.subtype, 0);
    assert_eq!(decoded.owner.server_id, 0);
    assert_eq!(decoded.loot_obj.low, 1);
    assert_eq!(decoded.loot_obj.high, 0x3C00_0442_4000_0000);
    assert_eq!(decoded.loot_list_id, 0);

    let rust = decode_loot_removed_body(&REAL_RUST_BODY).expect("decode real Rust body");
    assert_eq!(decoded, rust, "only the omitted owner counter may differ");
}

#[test]
fn real_paired_bodies_ignore_only_creature_owner_runtime_counter() {
    let report = s2c_report(REAL_CPP_BODY.to_vec(), REAL_RUST_BODY.to_vec());

    assert!(report.is_clean(), "{}", report.render_text());
    assert_eq!(report.counts.matched, 1);
    let body = report.ops[0].body.as_ref().expect("matched body");
    assert_ne!(body.cpp_len, body.rust_len);
    assert_eq!(body.first_diff_offset, None);
    let semantic = body.semantic.as_ref().expect("semantic comparator");
    assert_eq!(
        semantic.comparator,
        "smsg_loot_removed_without_creature_owner_runtime_guid_counter"
    );
    assert_eq!(semantic.cpp.raw_body_sha256, None);
    assert_eq!(semantic.rust.raw_body_sha256, None);
}

#[test]
fn loot_removed_compares_every_stable_owner_component() {
    let cpp = GuidFields::doctor(268);
    let variants = [
        (
            GuidFields {
                high_type: 9,
                ..cpp
            },
            "owner.high_type",
        ),
        (GuidFields { realm_id: 2, ..cpp }, "owner.realm_id"),
        (GuidFields { map_id: 571, ..cpp }, "owner.map_id"),
        (
            GuidFields {
                entry: 21_780,
                ..cpp
            },
            "owner.entry",
        ),
        (GuidFields { subtype: 1, ..cpp }, "owner.subtype"),
        (
            GuidFields {
                server_id: 1,
                ..cpp
            },
            "owner.server_id",
        ),
    ];

    for (changed, expected_field) in variants {
        let report = s2c_report(
            loot_removed_body(cpp, GuidFields::loot_object(1), 0),
            loot_removed_body(changed, GuidFields::loot_object(1), 0),
        );
        assert!(!report.is_clean(), "accepted {expected_field} mismatch");
        let semantic = report.ops[0]
            .body
            .as_ref()
            .and_then(|body| body.semantic.as_ref())
            .expect("semantic mismatch");
        assert!(
            semantic.mismatch_summary().contains(expected_field),
            "unexpected summary: {}",
            semantic.mismatch_summary()
        );
    }
}

#[test]
fn loot_removed_keeps_complete_loot_object_and_list_id_strict() {
    let owner_cpp = GuidFields::doctor(268);
    let owner_rust = GuidFields::doctor(1);
    let loot_obj = GuidFields::loot_object(1);
    let variants = [
        (
            GuidFields {
                counter: 2,
                ..loot_obj
            },
            0,
            "loot_obj.low",
        ),
        (
            GuidFields {
                map_id: 571,
                ..loot_obj
            },
            0,
            "loot_obj.high",
        ),
        (loot_obj, 1, "loot_list_id"),
    ];

    for (changed_loot_obj, changed_list_id, expected_field) in variants {
        let report = s2c_report(
            loot_removed_body(owner_cpp, loot_obj, 0),
            loot_removed_body(owner_rust, changed_loot_obj, changed_list_id),
        );
        assert!(!report.is_clean(), "accepted {expected_field} mismatch");
        let semantic = report.ops[0]
            .body
            .as_ref()
            .and_then(|body| body.semantic.as_ref())
            .expect("semantic mismatch");
        assert!(semantic.mismatch_summary().contains(expected_field));
    }
}

#[test]
fn creature_owner_requires_nonzero_runtime_counter_on_both_sides() {
    for (cpp_counter, rust_counter) in [(0, 1), (1, 0), (0, 0)] {
        let report = s2c_report(
            loot_removed_body(
                GuidFields::doctor(cpp_counter),
                GuidFields::loot_object(1),
                0,
            ),
            loot_removed_body(
                GuidFields::doctor(rust_counter),
                GuidFields::loot_object(1),
                0,
            ),
        );
        assert!(
            !report.is_clean(),
            "accepted owner counters {cpp_counter}/{rust_counter}"
        );
        assert!(report.render_text().contains("zero runtime GUID counter"));
    }
}

#[test]
fn malformed_noncanonical_and_trailing_bodies_never_compare_clean() {
    let mut noncanonical = REAL_RUST_BODY.to_vec();
    noncanonical[2] = 0; // Owner low mask promises a byte that must be nonzero.
    let mut trailing = REAL_RUST_BODY.to_vec();
    trailing.push(0xAA);

    for malformed in [vec![0x01], noncanonical, trailing] {
        let report = s2c_report(malformed.clone(), malformed);
        assert!(!report.is_clean(), "accepted malformed body");
        assert_eq!(report.counts.body_mismatches, 1);
        let semantic = report.ops[0]
            .body
            .as_ref()
            .and_then(|body| body.semantic.as_ref())
            .expect("malformed body must select fail-closed comparator");
        assert!(semantic.cpp.decode_error.is_some());
        assert!(semantic.rust.decode_error.is_some());
        assert!(semantic.cpp.raw_body_sha256.is_some());
        assert!(semantic.rust.raw_body_sha256.is_some());
    }
}

#[test]
fn two_non_creature_owners_keep_raw_counter_comparison() {
    let cpp_owner = GuidFields {
        high_type: 11, // HighGuid::GameObject
        ..GuidFields::doctor(1)
    };
    let rust_owner = GuidFields {
        counter: 2,
        ..cpp_owner
    };
    let report = s2c_report(
        loot_removed_body(cpp_owner, GuidFields::loot_object(1), 0),
        loot_removed_body(rust_owner, GuidFields::loot_object(1), 0),
    );

    assert!(!report.is_clean());
    assert_eq!(report.counts.body_mismatches, 1);
    assert!(report.ops[0].body.as_ref().unwrap().semantic.is_none());
}

#[test]
fn another_creature_identity_keeps_its_instance_counter_strict() {
    let first = GuidFields {
        entry: 21_780,
        counter: 1,
        ..GuidFields::doctor(1)
    };
    let second = GuidFields {
        counter: 2,
        ..first
    };
    let report = s2c_report(
        loot_removed_body(first, GuidFields::loot_object(1), 0),
        loot_removed_body(second, GuidFields::loot_object(1), 0),
    );

    assert!(!report.is_clean());
    assert_eq!(report.counts.body_mismatches, 1);
    assert!(
        report.ops[0].body.as_ref().unwrap().semantic.is_none(),
        "only the exact unique issue-#106 Doctor fixture may omit its instance counter"
    );
}

#[test]
fn no_other_opcode_or_direction_receives_loot_owner_normalization() {
    let cpp = REAL_CPP_BODY.to_vec();
    let rust = REAL_RUST_BODY.to_vec();

    let wrong_opcode = report(
        packet(Direction::S2C, 1, SMSG_LOOT_REMOVED - 1, cpp.clone()),
        packet(Direction::S2C, 1, SMSG_LOOT_REMOVED - 1, rust.clone()),
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
        packet(Direction::C2S, 1, SMSG_LOOT_REMOVED, cpp),
        packet(Direction::C2S, 1, SMSG_LOOT_REMOVED, rust),
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
}

#[test]
fn loot_removed_still_requires_cpp_instance_socket_routing() {
    let report = report(
        packet(Direction::S2C, 1, SMSG_LOOT_REMOVED, REAL_CPP_BODY.to_vec()),
        packet(
            Direction::S2C,
            0,
            SMSG_LOOT_REMOVED,
            REAL_RUST_BODY.to_vec(),
        ),
        Direction::S2C,
    );

    assert!(!report.is_clean());
    assert_eq!(report.counts.body_mismatches, 0);
    assert_eq!(report.counts.connection_mismatches, 1);
}

#[test]
fn semantic_baseline_identity_excludes_only_owner_counter_dependent_lengths() {
    let expected = s2c_report(
        loot_removed_body(GuidFields::doctor(1), GuidFields::loot_object(1), 0),
        loot_removed_body(GuidFields::doctor(2), GuidFields::loot_object(2), 0),
    );
    let current = s2c_report(
        loot_removed_body(
            GuidFields::doctor(0x01_0203_0405),
            GuidFields::loot_object(1),
            0,
        ),
        loot_removed_body(
            GuidFields::doctor(0x05_0403_0201),
            GuidFields::loot_object(2),
            0,
        ),
    );

    assert_ne!(
        expected.ops[0].body.as_ref().unwrap().cpp_len,
        current.ops[0].body.as_ref().unwrap().cpp_len
    );
    assert_eq!(expected.signatures(), current.signatures());
    let signature = &expected.signatures()[0];
    assert_eq!(signature.cpp_body_len, None);
    assert_eq!(signature.rust_body_len, None);
    assert_eq!(signature.first_diff_offset, None);
}
