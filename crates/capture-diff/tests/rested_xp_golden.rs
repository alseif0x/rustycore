//! Rested-XP golden-fixture regression gate for issue [81]/#81.
//!
//! The committed pair isolates the real C++ and Rust `SMSG_LOG_XP_GAIN`
//! emitted for the same rested Mana Wyrm kill. The victim runtime counter is
//! allocation-specific; routing, every stable GUID field, and every XP field
//! remain strict.

use capture_diff::diff::{DiffReport, DivergenceSignature};
use capture_diff::model::Direction;
use capture_diff::semantic::{SMSG_LOG_XP_GAIN, StableObjectGuid, decode_log_xp_gain_body};
use capture_diff::{flow, pkt, rustdump};

const EXPECTED_VICTIM: StableObjectGuid = StableObjectGuid {
    high_type: 8, // HighGuid::Creature
    realm_id: 1,
    map_id: 530,
    entry: 15_274,
    subtype: 0,
    server_id: 0,
};

fn load_rested_xp_diff() -> (DiffReport, flow::Flow) {
    let flow = flow::load_flow("rested-xp-kill").expect("rested-XP flow must exist");
    let cpp = pkt::parse_pkt_file(&flow.golden_pkt).expect("parse rested-XP C++ cpp.pkt");
    let rust = rustdump::parse_rust_dump(&flow.reference_rust).expect("parse rested-XP Rust dump");
    let report = DiffReport::compute(&cpp, &rust, &flow.directions);
    (report, flow)
}

#[test]
fn rested_xp_diff_matches_clean_committed_baseline() {
    let (report, flow) = load_rested_xp_diff();
    let expected_text =
        std::fs::read_to_string(&flow.expected).expect("read expected-divergences.json");
    let expected: Vec<DivergenceSignature> =
        serde_json::from_str(&expected_text).expect("parse expected-divergences.json");

    assert!(
        expected.is_empty(),
        "rested-XP is capture-clean and must not pin accepted divergences"
    );
    assert_eq!(report.signatures(), expected, "{}", report.render_text());
    assert!(report.is_clean(), "{}", report.render_text());
    assert_eq!(report.counts.matched, 1);
    assert_eq!(report.counts.body_mismatches, 0);
    assert_eq!(report.counts.connection_mismatches, 0);
    assert_eq!(report.counts.missing_in_rust, 0);
    assert_eq!(report.counts.extra_in_rust, 0);

    let body = report.ops[0].body.as_ref().expect("matched XP body diff");
    let semantic = body
        .semantic
        .as_ref()
        .expect("rested kill must exercise the reviewed semantic comparator");
    assert_eq!(
        semantic.comparator,
        "smsg_log_xp_gain_without_runtime_guid_counter"
    );
    assert!(semantic.is_identical());
}

#[test]
fn rested_xp_golden_pins_routing_stable_guid_and_xp_fields() {
    let flow = flow::load_flow("rested-xp-kill").expect("rested-XP flow must exist");
    let cpp = pkt::parse_pkt_file(&flow.golden_pkt).expect("parse rested-XP C++ cpp.pkt");
    let rust = rustdump::parse_rust_dump(&flow.reference_rust).expect("parse rested-XP Rust dump");

    assert_eq!(cpp.packets.len(), 1);
    assert_eq!(rust.packets.len(), 1);
    for packet in [&cpp.packets[0], &rust.packets[0]] {
        assert_eq!(packet.direction, Direction::S2C);
        assert_eq!(packet.connection_id, 0, "XP must use the realm socket");
        assert_eq!(packet.opcode, SMSG_LOG_XP_GAIN);

        let decoded = decode_log_xp_gain_body(&packet.body).expect("decode rested-XP body");
        assert_eq!(decoded.victim, EXPECTED_VICTIM);
        assert_eq!(decoded.original, 100);
        assert_eq!(decoded.reason, 0); // Kill
        assert_eq!(decoded.amount, 50);
        assert_eq!(decoded.group_bonus_bits, 1.0f32.to_bits());
    }

    assert_ne!(
        cpp.packets[0].body, rust.packets[0].body,
        "fixture must retain distinct real runtime counters"
    );
}

#[test]
fn rested_xp_gate_trips_on_realm_instance_routing_regression() {
    let flow = flow::load_flow("rested-xp-kill").expect("rested-XP flow must exist");
    let cpp = pkt::parse_pkt_file(&flow.golden_pkt).expect("parse rested-XP C++ cpp.pkt");
    let mut rust =
        rustdump::parse_rust_dump(&flow.reference_rust).expect("parse rested-XP Rust dump");

    rust.packets[0].connection_id = 1;
    let report = DiffReport::compute(&cpp, &rust, &flow.directions);

    assert!(!report.is_clean());
    assert_eq!(report.counts.body_mismatches, 0);
    assert_eq!(report.counts.connection_mismatches, 1);
}
