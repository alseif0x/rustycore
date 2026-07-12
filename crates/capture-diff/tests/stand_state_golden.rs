//! Stand-state golden-fixture regression gate for issue [07]/#19.
//!
//! The committed pair is a real C++ TrinityCore capture and the matching
//! RustyCore capture of `CMSG_STAND_STATE_CHANGE` to Sit. The fixture keeps the
//! realm ACK and instance VALUES update on their C++ socket routes, then uses
//! the instance `CMSG_PING` as a deterministic end fence.

use capture_diff::diff::{DiffReport, DivergenceSignature};
use capture_diff::model::Direction;
use capture_diff::{flow, pkt, rustdump};

const STAND_STATE_CHANGE: u16 = 0x318C;
const STAND_STATE_UPDATE: u16 = 0x271C;
const UPDATE_OBJECT: u16 = 0x27CB;
const PING: u16 = 0x3768;

fn load_stand_state_diff() -> (DiffReport, flow::Flow) {
    let flow = flow::load_flow("stand-state").expect("stand-state flow must exist");
    let cpp = pkt::parse_pkt_file(&flow.golden_pkt).expect("parse stand-state C++ cpp.pkt");
    let rust =
        rustdump::parse_rust_dump(&flow.reference_rust).expect("parse stand-state Rust dump");
    let report = DiffReport::compute(&cpp, &rust, &flow.directions);
    (report, flow)
}

#[test]
fn stand_state_diff_matches_clean_committed_baseline() {
    let (report, flow) = load_stand_state_diff();
    let expected_text =
        std::fs::read_to_string(&flow.expected).expect("read expected-divergences.json");
    let expected: Vec<DivergenceSignature> =
        serde_json::from_str(&expected_text).expect("parse expected-divergences.json");

    assert!(
        expected.is_empty(),
        "stand-state is a capture-clean flow and must not pin accepted divergences"
    );
    assert_eq!(report.signatures(), expected, "{}", report.render_text());
    assert!(report.is_clean(), "{}", report.render_text());
    assert_eq!(report.counts.matched, 4);
    assert_eq!(report.counts.connection_mismatches, 0);
}

#[test]
fn stand_state_golden_pins_cpp_packet_order_and_socket_topology() {
    let flow = flow::load_flow("stand-state").expect("stand-state flow must exist");
    let cpp = pkt::parse_pkt_file(&flow.golden_pkt).expect("parse stand-state C++ cpp.pkt");
    let rust =
        rustdump::parse_rust_dump(&flow.reference_rust).expect("parse stand-state Rust dump");

    let expected = [
        (Direction::C2S, 1, STAND_STATE_CHANGE),
        (Direction::S2C, 0, STAND_STATE_UPDATE),
        (Direction::S2C, 1, UPDATE_OBJECT),
        (Direction::C2S, 1, PING),
    ];
    let cpp_shape: Vec<_> = cpp
        .packets
        .iter()
        .map(|packet| (packet.direction, packet.connection_id, packet.opcode))
        .collect();
    let rust_shape: Vec<_> = rust
        .packets
        .iter()
        .map(|packet| (packet.direction, packet.connection_id, packet.opcode))
        .collect();

    assert_eq!(cpp_shape, expected, "C++ golden routing/order drifted");
    assert_eq!(rust_shape, expected, "Rust fixture routing/order drifted");
    assert_eq!(cpp.packets[0].body, [1, 0, 0, 0]);
    assert_eq!(cpp.packets[1].body, [0, 0, 0, 0, 1]);
}

#[test]
fn stand_state_gate_trips_on_realm_instance_routing_regression() {
    let flow = flow::load_flow("stand-state").expect("stand-state flow must exist");
    let cpp = pkt::parse_pkt_file(&flow.golden_pkt).expect("parse stand-state C++ cpp.pkt");
    let mut rust =
        rustdump::parse_rust_dump(&flow.reference_rust).expect("parse stand-state Rust dump");

    rust.packets[1].connection_id = 1;
    let report = DiffReport::compute(&cpp, &rust, &flow.directions);

    assert!(!report.is_clean());
    assert_eq!(report.counts.connection_mismatches, 1);
}
