//! Focused tests for the only normalization in the rested-XP capture gate.
//!
//! The victim's runtime `ObjectGuid` counter differs naturally between separate
//! C++ and Rust runs. The comparator must ignore exactly those lower 40 bits,
//! while retaining socket routing, all other GUID fields, and every XP field.

use capture_diff::diff::DiffReport;
use capture_diff::model::{Capture, CapturedPacket, Direction};
use capture_diff::semantic::{SMSG_LOG_XP_GAIN, decode_log_xp_gain_body};

#[derive(Clone, Copy)]
struct XpFields {
    high_type: u8,
    realm_id: u16,
    map_id: u16,
    entry: u32,
    subtype: u8,
    server_id: u32,
    counter: u64,
    original: i32,
    reason: u8,
    amount: i32,
    group_bonus_bits: u32,
}

impl Default for XpFields {
    fn default() -> Self {
        Self {
            high_type: 8, // HighGuid::Creature
            realm_id: 1,
            map_id: 530,
            entry: 15_367,
            subtype: 0,
            server_id: 7,
            counter: 1,
            original: 100,
            reason: 0,
            amount: 50,
            group_bonus_bits: 1.0f32.to_bits(),
        }
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
    for (index, byte) in low_bytes.iter().enumerate() {
        if low_mask & (1 << index) != 0 {
            body.push(*byte);
        }
    }
    for (index, byte) in high_bytes.iter().enumerate() {
        if high_mask & (1 << index) != 0 {
            body.push(*byte);
        }
    }
    body
}

fn log_xp_gain_body(fields: XpFields) -> Vec<u8> {
    let high = (u64::from(fields.high_type) << 58)
        | (u64::from(fields.realm_id) << 42)
        | (u64::from(fields.map_id) << 29)
        | (u64::from(fields.entry) << 6)
        | u64::from(fields.subtype);
    let low = (u64::from(fields.server_id) << 40) | fields.counter;
    let mut body = packed_guid(low, high);
    body.extend_from_slice(&fields.original.to_le_bytes());
    body.push(fields.reason);
    body.extend_from_slice(&fields.amount.to_le_bytes());
    body.extend_from_slice(&fields.group_bonus_bits.to_le_bytes());
    body
}

fn packet(connection_id: u32, opcode: u16, body: Vec<u8>) -> CapturedPacket {
    CapturedPacket {
        direction: Direction::S2C,
        connection_id,
        opcode,
        body,
    }
}

fn report(cpp: CapturedPacket, rust: CapturedPacket) -> DiffReport {
    DiffReport::compute(
        &Capture::new("cpp", vec![cpp]),
        &Capture::new("rust", vec![rust]),
        &[Direction::S2C],
    )
}

#[test]
fn decoder_pins_cpp_packed_guid_and_field_order_from_literal_wire_bytes() {
    // GUID high=0x2000_0442_400F_01C0 (Creature, realm 1, map 530,
    // entry 15367), low=0x0000_0700_0000_0001 (server 7, counter 1).
    // Followed by Original=100, Reason=Kill, Amount=50, GroupBonus=1.0.
    let body = [
        0x21, 0xBF, // low/high masks
        0x01, 0x07, // packed low bytes
        0xC0, 0x01, 0x0F, 0x40, 0x42, 0x04, 0x20, // packed high bytes
        0x64, 0x00, 0x00, 0x00, // Original
        0x00, // Reason
        0x32, 0x00, 0x00, 0x00, // Amount
        0x00, 0x00, 0x80, 0x3F, // GroupBonus
    ];

    let decoded = decode_log_xp_gain_body(&body).expect("decode literal C++ layout");
    assert_eq!(decoded.victim.high_type, 8);
    assert_eq!(decoded.victim.realm_id, 1);
    assert_eq!(decoded.victim.map_id, 530);
    assert_eq!(decoded.victim.entry, 15_367);
    assert_eq!(decoded.victim.subtype, 0);
    assert_eq!(decoded.victim.server_id, 7);
    assert_eq!(decoded.original, 100);
    assert_eq!(decoded.reason, 0);
    assert_eq!(decoded.amount, 50);
    assert_eq!(decoded.group_bonus_bits, 1.0f32.to_bits());
}

#[test]
fn log_xp_gain_ignores_only_the_runtime_guid_counter() {
    let cpp = XpFields {
        counter: 1,
        ..XpFields::default()
    };
    let rust = XpFields {
        // Different non-zero byte shape also proves that packed-body length is
        // allowed to differ when and only when the lower 40-bit counter does.
        counter: 0x01_0203_0405,
        ..XpFields::default()
    };

    let report = report(
        packet(0, SMSG_LOG_XP_GAIN, log_xp_gain_body(cpp)),
        packet(0, SMSG_LOG_XP_GAIN, log_xp_gain_body(rust)),
    );

    assert!(report.is_clean(), "{}", report.render_text());
    assert_eq!(report.counts.matched, 1);
    let body = report.ops[0].body.as_ref().expect("matched body diff");
    assert!(body.semantic.is_some());
    assert!(body.is_identical());
    assert_ne!(body.cpp_len, body.rust_len);
}

#[test]
fn log_xp_gain_compares_every_stable_guid_component() {
    let original = XpFields::default();
    let variants = [
        (
            XpFields {
                high_type: 9,
                ..original
            },
            "victim.high_type",
        ),
        (
            XpFields {
                realm_id: 2,
                ..original
            },
            "victim.realm_id",
        ),
        (
            XpFields {
                map_id: 571,
                ..original
            },
            "victim.map_id",
        ),
        (
            XpFields {
                entry: 15_368,
                ..original
            },
            "victim.entry",
        ),
        (
            XpFields {
                subtype: 1,
                ..original
            },
            "victim.subtype",
        ),
        (
            XpFields {
                server_id: 8,
                ..original
            },
            "victim.server_id",
        ),
    ];

    for (changed, expected_field) in variants {
        let report = report(
            packet(0, SMSG_LOG_XP_GAIN, log_xp_gain_body(original)),
            packet(0, SMSG_LOG_XP_GAIN, log_xp_gain_body(changed)),
        );
        assert!(!report.is_clean(), "accepted {expected_field} mismatch");
        let semantic = report.ops[0]
            .body
            .as_ref()
            .and_then(|body| body.semantic.as_ref())
            .expect("semantic body diff");
        assert_eq!(report.ops[0].body.as_ref().unwrap().first_diff_offset, None);
        assert!(
            semantic.mismatch_summary().contains(expected_field),
            "unexpected summary: {}",
            semantic.mismatch_summary()
        );
    }
}

#[test]
fn log_xp_gain_compares_all_xp_fields_exactly() {
    let original = XpFields::default();
    let variants = [
        (
            XpFields {
                original: 101,
                ..original
            },
            "original",
        ),
        (
            XpFields {
                reason: 1,
                ..original
            },
            "reason",
        ),
        (
            XpFields {
                amount: 51,
                ..original
            },
            "amount",
        ),
        (
            XpFields {
                group_bonus_bits: f32::from_bits(1.0f32.to_bits() + 1).to_bits(),
                ..original
            },
            "group_bonus",
        ),
    ];

    for (changed, expected_field) in variants {
        let report = report(
            packet(0, SMSG_LOG_XP_GAIN, log_xp_gain_body(original)),
            packet(0, SMSG_LOG_XP_GAIN, log_xp_gain_body(changed)),
        );
        assert!(!report.is_clean(), "accepted {expected_field} mismatch");
        let semantic = report.ops[0]
            .body
            .as_ref()
            .and_then(|body| body.semantic.as_ref())
            .expect("semantic body diff");
        assert!(semantic.mismatch_summary().contains(expected_field));
    }
}

#[test]
fn log_xp_gain_still_requires_cpp_realm_socket_routing() {
    let cpp = XpFields {
        counter: 1,
        ..XpFields::default()
    };
    let rust = XpFields {
        counter: 2,
        ..XpFields::default()
    };
    let report = report(
        packet(0, SMSG_LOG_XP_GAIN, log_xp_gain_body(cpp)),
        packet(1, SMSG_LOG_XP_GAIN, log_xp_gain_body(rust)),
    );

    assert!(!report.is_clean());
    assert_eq!(report.counts.body_mismatches, 0);
    assert_eq!(report.counts.connection_mismatches, 1);
}

#[test]
fn malformed_log_xp_gain_is_never_capture_clean() {
    let malformed = vec![0x01]; // truncated before the high mask
    let report = report(
        packet(0, SMSG_LOG_XP_GAIN, malformed.clone()),
        packet(0, SMSG_LOG_XP_GAIN, malformed),
    );

    assert!(!report.is_clean());
    assert_eq!(report.counts.body_mismatches, 1);
    assert!(report.render_text().contains("decode error"));
}

#[test]
fn kill_xp_requires_nonzero_creature_runtime_counters() {
    for (cpp_counter, rust_counter) in [(0, 1), (1, 0), (0, 0)] {
        let report = report(
            packet(
                0,
                SMSG_LOG_XP_GAIN,
                log_xp_gain_body(XpFields {
                    counter: cpp_counter,
                    ..XpFields::default()
                }),
            ),
            packet(
                0,
                SMSG_LOG_XP_GAIN,
                log_xp_gain_body(XpFields {
                    counter: rust_counter,
                    ..XpFields::default()
                }),
            ),
        );

        assert!(
            !report.is_clean(),
            "accepted counters {cpp_counter}/{rust_counter}"
        );
        assert!(report.render_text().contains("zero runtime GUID counter"));
    }
}

#[test]
fn non_kill_xp_keeps_raw_victim_counter_comparison() {
    let cpp = log_xp_gain_body(XpFields {
        reason: 1,
        counter: 1,
        ..XpFields::default()
    });
    let rust = log_xp_gain_body(XpFields {
        reason: 1,
        counter: 2,
        ..XpFields::default()
    });
    let report = report(
        packet(0, SMSG_LOG_XP_GAIN, cpp),
        packet(0, SMSG_LOG_XP_GAIN, rust),
    );

    assert!(!report.is_clean());
    assert_eq!(report.counts.body_mismatches, 1);
    assert!(report.ops[0].body.as_ref().unwrap().semantic.is_none());
}

#[test]
fn no_other_opcode_receives_guid_counter_normalization() {
    let cpp = log_xp_gain_body(XpFields {
        counter: 1,
        ..XpFields::default()
    });
    let rust = log_xp_gain_body(XpFields {
        counter: 2,
        ..XpFields::default()
    });
    let report = report(packet(0, 0x26E4, cpp), packet(0, 0x26E4, rust));

    assert!(!report.is_clean());
    assert_eq!(report.counts.body_mismatches, 1);
    assert!(report.ops[0].body.as_ref().unwrap().semantic.is_none());
}
