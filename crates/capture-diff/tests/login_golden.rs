//! Login-flow golden-fixture regression gate (issue [01]/#66 "Done" criterion).
//!
//! Parses the committed C++ golden (`flows/login/cpp.pkt`) and reference Rust
//! dump (`flows/login/rust/`), diffs them, and asserts the result matches the
//! committed accepted-divergence baseline. Any change to the Rust login output
//! (or the harness) shifts the diff and trips this test — the milestone gate.

use capture_diff::diff::{DiffReport, DivergenceKind, DivergenceSignature};
use capture_diff::model::Direction;
use capture_diff::{flow, pkt, rustdump};

fn load_login_diff() -> (DiffReport, flow::Flow) {
    let flow = flow::load_flow("login").expect("login flow must exist");
    let cpp = pkt::parse_pkt_file(&flow.golden_pkt).expect("parse golden cpp.pkt");
    let rust = rustdump::parse_rust_dump(&flow.reference_rust).expect("parse reference rust dump");
    let report = DiffReport::compute(&cpp, &rust, &flow.directions);
    (report, flow)
}

#[test]
fn login_diff_matches_committed_baseline() {
    let (report, flow) = load_login_diff();

    let expected_text =
        std::fs::read_to_string(&flow.expected).expect("read expected-divergences.json");
    let expected: Vec<DivergenceSignature> =
        serde_json::from_str(&expected_text).expect("parse expected-divergences.json");

    let actual = report.signatures();
    assert_eq!(
        actual,
        expected,
        "login diff drifted from committed baseline.\n\
         If this is an intentional fix, regenerate with:\n\
         cargo run -p capture-diff -- update-baseline login\n\nactual:\n{}",
        report.render_text()
    );
}

#[test]
fn login_has_known_divergences() {
    let (report, _) = load_login_diff();
    assert!(
        !report.is_clean(),
        "the synthetic login fixture is expected to diverge (it models the audit gaps)"
    );
}

#[test]
fn login_surfaces_each_audit_class() {
    let (report, _) = load_login_diff();
    let sigs = report.signatures();

    let has = |kind: DivergenceKind, opcode: &str| {
        sigs.iter()
            .any(|s| s.kind == kind && s.opcode.eq_ignore_ascii_case(opcode))
    };

    // Missing in Rust: global AccountDataTimes resend (#1202), TutorialFlags (#1203).
    assert!(
        has(DivergenceKind::MissingInRust, "0x270A"),
        "expected AccountDataTimes missing"
    );
    assert!(
        has(DivergenceKind::MissingInRust, "0x27BE"),
        "expected TutorialFlags missing"
    );
    // Extra in Rust: LfgListUpdateBlacklist (#1208).
    assert!(
        has(DivergenceKind::ExtraInRust, "0x2A2A"),
        "expected LfgListUpdateBlacklist extra"
    );
    // Value: FeatureSystemStatus default vs config-populated.
    assert!(
        has(DivergenceKind::BodyMismatch, "0x25BF"),
        "expected FeatureSystemStatus body mismatch"
    );
}

#[test]
fn client_to_server_stream_is_clean() {
    // The same client drives both servers, so the c2s stream must match exactly.
    let (report, _) = load_login_diff();
    let c2s_divergences = report
        .signatures()
        .into_iter()
        .filter(|s| s.direction == Direction::C2S)
        .count();
    assert_eq!(c2s_divergences, 0, "c2s stream must be identical");
}

#[test]
fn baseline_gate_trips_on_regression() {
    // Drop a packet from the reference Rust capture and confirm the diff's
    // signature set changes — i.e. the gate would catch a real regression.
    let flow = flow::load_flow("login").unwrap();
    let cpp = pkt::parse_pkt_file(&flow.golden_pkt).unwrap();
    let mut rust = rustdump::parse_rust_dump(&flow.reference_rust).unwrap();

    let baseline = DiffReport::compute(&cpp, &rust, &flow.directions).signatures();
    rust.packets.retain(|p| p.opcode != 0x257d); // drop BindPointUpdate
    let regressed = DiffReport::compute(&cpp, &rust, &flow.directions).signatures();

    assert_ne!(
        baseline, regressed,
        "removing a packet must change the divergence signature set"
    );
}

#[test]
fn golden_pkt_is_real_pkt_format() {
    let flow = flow::load_flow("login").unwrap();
    let bytes = std::fs::read(&flow.golden_pkt).unwrap();
    assert_eq!(&bytes[0..3], b"PKT", "golden must be a PKT capture");
    let cap = pkt::parse_pkt_file(&flow.golden_pkt).unwrap();
    assert!(!cap.packets.is_empty(), "golden capture must have packets");
}
