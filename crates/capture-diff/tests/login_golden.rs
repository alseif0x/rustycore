//! Login-flow golden-fixture regression gate (issue [01]/#66 "Done" criterion).
//!
//! The committed fixtures are a **real capture** (2026-06-28): a C++ TrinityCore
//! `PacketLogFile` golden (`flows/login/cpp.pkt`) and a RustyCore dump
//! (`flows/login/rust/`) of the same character logging in, trimmed to the login
//! flow (first `CMSG_MOVE_INIT_ACTIVE_MOVER_COMPLETE`). The flow diffs **s2c**
//! only (c2s carries per-session crypto/timestamps — see `flow.json`).
//!
//! `login_diff_matches_committed_baseline` is the gate: it parses the committed
//! pair, diffs them, and asserts the result equals the committed accepted-
//! divergence baseline. When the Rust login output changes, re-capture and
//! re-pin with `capture-diff import login --cpp … --rust … --until-opcode 0x3A46`.

// Product names (RustyCore, TrinityCore) appear in the docs as prose.
#![allow(clippy::doc_markdown)]

use capture_diff::diff::{DiffReport, DivergenceSignature};
use capture_diff::{flow, pkt, rustdump};

const AUTH_RESPONSE: u16 = 0x256D;
const PHASE_SHIFT_CHANGE: u16 = 0x2578;
const LOAD_CUF_PROFILES: u16 = 0x25BC;
const INIT_WORLD_STATES: u16 = 0x2746;
const UPDATE_OBJECT: u16 = 0x27CB;
const AURA_UPDATE: u16 = 0x2C1F;

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
         If this is an intentional change to Rust login output, re-capture and re-pin:\n\
         cargo run -p capture-diff -- import login --cpp <pkt> --rust <dir> --until-opcode 0x3A46\n\nactual:\n{}",
        report.render_text()
    );
}

#[test]
fn golden_is_a_real_cpp_pkt_capture() {
    let flow = flow::load_flow("login").unwrap();
    let bytes = std::fs::read(&flow.golden_pkt).unwrap();
    assert_eq!(
        &bytes[0..3],
        b"PKT",
        "golden must be a real C++ PKT capture"
    );

    let cpp = pkt::parse_pkt_file(&flow.golden_pkt).unwrap();
    // A real login capture has the auth response and the in-world player CREATE.
    assert!(
        cpp.packets.iter().any(|p| p.opcode == AUTH_RESPONSE),
        "golden must contain SMSG_AUTH_RESPONSE"
    );
    assert!(
        cpp.packets.iter().any(|p| p.opcode == UPDATE_OBJECT),
        "golden must contain the in-world SMSG_UPDATE_OBJECT (player create)"
    );
}

#[test]
fn rust_reference_parses_and_covers_the_login_burst() {
    let flow = flow::load_flow("login").unwrap();
    let rust = rustdump::parse_rust_dump(&flow.reference_rust).unwrap();
    assert!(rust.packets.iter().any(|p| p.opcode == AUTH_RESPONSE));
    assert!(rust.packets.iter().any(|p| p.opcode == UPDATE_OBJECT));
}

#[test]
fn load_cuf_profiles_matches_cpp_body_and_post_add_order() {
    let flow = flow::load_flow("login").unwrap();
    let cpp = pkt::parse_pkt_file(&flow.golden_pkt).unwrap();
    let rust = rustdump::parse_rust_dump(&flow.reference_rust).unwrap();

    let cpp_cuf = cpp
        .packets
        .iter()
        .find(|packet| packet.opcode == LOAD_CUF_PROFILES)
        .expect("C++ login capture must contain LoadCufProfiles");
    let rust_cuf = rust
        .packets
        .iter()
        .find(|packet| packet.opcode == LOAD_CUF_PROFILES)
        .expect("Rust login capture must contain LoadCufProfiles");
    assert_eq!(
        rust_cuf.body, cpp_cuf.body,
        "Rust LoadCufProfiles must be byte-identical to the C++ golden packet"
    );

    let expected = [
        INIT_WORLD_STATES,
        LOAD_CUF_PROFILES,
        AURA_UPDATE,
        PHASE_SHIFT_CHANGE,
    ];
    for (label, capture) in [("C++", &cpp), ("Rust", &rust)] {
        assert!(
            capture
                .packets
                .windows(expected.len())
                .any(|packets| packets.iter().map(|packet| packet.opcode).eq(expected)),
            "{label} login must send InitWorldStates, LoadCufProfiles, AuraUpdate and the \
             OnMapChange PhaseShiftChange in C++ order"
        );
    }
}

#[test]
fn login_has_real_divergences() {
    // Rust login is not yet byte-clean vs C++; the gate's value is locking the
    // current divergence set so any change is surfaced.
    let (report, _) = load_login_diff();
    assert!(
        !report.is_clean(),
        "expected the real capture to still diverge from C++ (login parity is in progress)"
    );
}

#[test]
fn baseline_gate_trips_on_regression() {
    // Dropping a packet that currently matches C++ must change the divergence
    // signature set — i.e. the gate would catch a real regression.
    let flow = flow::load_flow("login").unwrap();
    let cpp = pkt::parse_pkt_file(&flow.golden_pkt).unwrap();
    let mut rust = rustdump::parse_rust_dump(&flow.reference_rust).unwrap();

    let baseline = DiffReport::compute(&cpp, &rust, &flow.directions).signatures();
    rust.packets.retain(|p| p.opcode != AUTH_RESPONSE); // drop a clean-matching packet
    let regressed = DiffReport::compute(&cpp, &rust, &flow.directions).signatures();

    assert_ne!(
        baseline, regressed,
        "removing a matching packet must change the divergence signature set"
    );
}
