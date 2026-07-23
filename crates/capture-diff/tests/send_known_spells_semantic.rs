//! Focused issue-#62 capture semantics for `SMSG_SEND_KNOWN_SPELLS`.
//!
//! C++ builds both wire vectors by iterating `PlayerSpellMap`, an
//! `std::unordered_map`. Exact vector order is not stable, but every spell,
//! favorite, count, body bit, direction, and connection role remains part of
//! the contract.

use capture_diff::diff::DiffReport;
use capture_diff::model::{Capture, CapturedPacket, Direction};
use capture_diff::semantic::{SMSG_SEND_KNOWN_SPELLS, decode_send_known_spells_body};

// Extracted from the live C++ PKT capture whose SHA-256 is
// e7bdd3d9dae3539f988d3ccacf0f478585335df000264c7f56542324a11be234.
const ISSUE_62_CPP_WIRE_ORDER: [u32; 43] = [
    822, 28_730, 1_180, 264, 2_973, 75, 28_877, 203, 6_478, 6_233, 3_050, 8_386, 3_365, 204, 6_247,
    13_358, 21_652, 349_794, 63_644, 9_078, 6_246, 24_949, 9_077, 522, 813, 6_477, 2_382, 81,
    6_603, 45_927, 7_266, 197, 669, 22_027, 22_810, 68_398, 7_267, 21_651, 63_645, 7_355, 9_125,
    34_082, 61_437,
];

// Extracted from the live Rust packet dump produced by source HEAD 7279baa0;
// the complete packet SHA-256 is
// 7e44f336fad055fb195a2da502d9f2ce7d869f52e4c9e2657b9fab6dcddf40d7.
const ISSUE_62_RUST_WIRE_ORDER: [u32; 43] = [
    264, 2_973, 204, 81, 522, 13_358, 24_949, 669, 813, 203, 75, 197, 1_180, 2_382, 3_365, 3_050,
    6_233, 6_246, 6_247, 6_477, 6_478, 6_603, 7_266, 7_267, 7_355, 8_386, 9_125, 21_651, 21_652,
    22_027, 22_810, 34_082, 45_927, 61_437, 63_645, 63_644, 68_398, 349_794, 9_077, 9_078, 28_730,
    822, 28_877,
];

fn body(initial_login: bool, known: &[u32], favorites: &[u32]) -> Vec<u8> {
    let mut body = vec![if initial_login { 0x80 } else { 0 }];
    body.extend((known.len() as u32).to_le_bytes());
    body.extend((favorites.len() as u32).to_le_bytes());
    for spell in known.iter().chain(favorites) {
        body.extend(spell.to_le_bytes());
    }
    body
}

fn packet(connection_id: u32, body: Vec<u8>) -> CapturedPacket {
    CapturedPacket {
        direction: Direction::S2C,
        connection_id,
        opcode: SMSG_SEND_KNOWN_SPELLS,
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
fn captured_issue_62_spell_sets_compare_clean_despite_unordered_map_order() {
    let cpp_body = body(true, &ISSUE_62_CPP_WIRE_ORDER, &[]);
    let rust_body = body(true, &ISSUE_62_RUST_WIRE_ORDER, &[]);
    assert_ne!(
        cpp_body, rust_body,
        "fixture must retain distinct wire order"
    );

    let cpp = decode_send_known_spells_body(&cpp_body).expect("decode captured C++ values");
    let rust = decode_send_known_spells_body(&rust_body).expect("decode captured Rust values");
    assert_eq!(cpp, rust);
    assert!(cpp.initial_login);
    assert_eq!(cpp.known_spells.len(), 43);
    assert!(cpp.favorite_spells.is_empty());

    let report = report(packet(1, cpp_body), packet(1, rust_body));
    assert!(report.is_clean(), "{}", report.render_text());
    let semantic = report.ops[0]
        .body
        .as_ref()
        .and_then(|body| body.semantic.as_ref())
        .expect("SendKnownSpells semantic comparator");
    assert_eq!(
        semantic.comparator,
        "smsg_send_known_spells_unordered_spell_sets"
    );
    assert!(semantic.is_identical());
}

#[test]
fn membership_favorites_initial_bit_and_shape_remain_strict() {
    let valid = body(true, &[75, 81, 822], &[822]);

    for (rust, expected) in [
        (body(true, &[75, 81], &[]), "known_spells"),
        (body(true, &[75, 81, 822], &[81]), "favorite_spells"),
        (body(false, &[75, 81, 822], &[822]), "initial_login"),
    ] {
        let report = report(packet(1, valid.clone()), packet(1, rust));
        assert!(!report.is_clean(), "accepted {expected} mismatch");
        assert!(
            report.render_text().contains(expected),
            "{}",
            report.render_text()
        );
    }

    let mut bad_padding = valid.clone();
    bad_padding[0] |= 0x01;
    for malformed in [
        bad_padding,
        body(true, &[75, 75], &[]),
        body(true, &[75], &[81]),
        body(true, &[0], &[]),
        valid[..valid.len() - 1].to_vec(),
    ] {
        let report = report(packet(1, valid.clone()), packet(1, malformed));
        assert!(!report.is_clean());
        assert!(
            report.ops[0]
                .body
                .as_ref()
                .and_then(|body| body.semantic.as_ref())
                .is_some_and(|semantic| semantic.rust.decode_error.is_some())
        );
    }
}

#[test]
fn unordered_comparator_does_not_hide_direction_or_connection_regressions() {
    let cpp_body = body(true, &ISSUE_62_CPP_WIRE_ORDER, &[]);
    let rust_body = body(true, &ISSUE_62_RUST_WIRE_ORDER, &[]);
    let report = report(packet(0, cpp_body), packet(1, rust_body));

    assert!(!report.is_clean());
    assert_eq!(report.counts.body_mismatches, 0);
    assert_eq!(report.counts.connection_mismatches, 1);

    let c2s = DiffReport::compute(
        &Capture::new(
            "cpp",
            vec![CapturedPacket {
                direction: Direction::C2S,
                connection_id: 1,
                opcode: SMSG_SEND_KNOWN_SPELLS,
                body: body(true, &[75, 81], &[]),
            }],
        ),
        &Capture::new(
            "rust",
            vec![CapturedPacket {
                direction: Direction::C2S,
                connection_id: 1,
                opcode: SMSG_SEND_KNOWN_SPELLS,
                body: body(true, &[81, 75], &[]),
            }],
        ),
        &[Direction::C2S],
    );
    assert!(!c2s.is_clean());
    assert!(c2s.ops[0].body.as_ref().unwrap().semantic.is_none());
}
