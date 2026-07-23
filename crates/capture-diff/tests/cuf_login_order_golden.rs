//! Live C++/Rust regression evidence for issue #7's CUF login ordering.

use capture_diff::diff::{DiffReport, DivergenceSignature};
use capture_diff::{flow, pkt, rustdump};

const INIT_WORLD_STATES: u16 = 0x2746;
const LOAD_CUF_PROFILES: u16 = 0x25BC;
const AURA_UPDATE: u16 = 0x2C1F;
const PHASE_SHIFT_CHANGE: u16 = 0x2578;

fn load_capture_pair() -> (capture_diff::Capture, capture_diff::Capture, flow::Flow) {
    let flow = flow::load_flow("cuf-login-order").expect("CUF login-order flow must exist");
    let cpp = pkt::parse_pkt_file(&flow.golden_pkt).expect("parse C++ CUF golden");
    let rust = rustdump::parse_rust_dump(&flow.reference_rust).expect("parse Rust CUF capture");
    (cpp, rust, flow)
}

#[test]
fn cuf_login_order_matches_cpp_and_affected_packets_are_exact() {
    let (cpp, rust, _) = load_capture_pair();
    let expected = [
        INIT_WORLD_STATES,
        LOAD_CUF_PROFILES,
        AURA_UPDATE,
        PHASE_SHIFT_CHANGE,
    ];

    for (label, capture) in [("C++", &cpp), ("Rust", &rust)] {
        assert_eq!(
            capture
                .packets
                .iter()
                .map(|packet| packet.opcode)
                .collect::<Vec<_>>(),
            expected,
            "{label} must contain exactly the C++ post-add packet order"
        );
        assert!(
            capture
                .packets
                .iter()
                .all(|packet| packet.connection_id == 1),
            "{label} post-add packets must all use the instance connection"
        );
    }

    assert!(
        cpp.packets[1].body.len() > 4,
        "the captured C++ CUF packet must contain a non-empty profile"
    );
    assert_eq!(
        rust.packets[1].body, cpp.packets[1].body,
        "Rust LoadCufProfiles must be byte-identical to C++"
    );
    assert_eq!(
        rust.packets[3].body, cpp.packets[3].body,
        "the final OnMapChange PhaseShiftChange must be byte-identical to C++"
    );
}

#[test]
fn cuf_login_order_retains_only_the_reviewed_dynamic_value_baseline() {
    let (cpp, rust, flow) = load_capture_pair();
    let report = DiffReport::compute(&cpp, &rust, &flow.directions);
    let expected_text =
        std::fs::read_to_string(&flow.expected).expect("read expected-divergences.json");
    let expected: Vec<DivergenceSignature> =
        serde_json::from_str(&expected_text).expect("parse expected CUF divergences");

    assert_eq!(
        report.signatures(),
        expected,
        "CUF login-order capture drifted:\n{}",
        report.render_text()
    );
    assert_eq!(
        report.counts.matched, 2,
        "CUF and final PhaseShift must match"
    );
    assert_eq!(
        report.counts.body_mismatches, 2,
        "only world-state/aura values may differ"
    );
    assert_eq!(report.counts.connection_mismatches, 0);
    assert_eq!(report.counts.missing_in_rust, 0);
    assert_eq!(report.counts.extra_in_rust, 0);
}
