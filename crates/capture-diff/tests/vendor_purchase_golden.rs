//! Vendor-purchase golden-fixture regression gate for issue #108.
//!
//! The committed pair isolates the two realm-routed responses emitted after
//! the extended-cost purchase commits. The retained, hash-bound bot reports
//! separately prove the currency debit, item persistence after fresh auth,
//! and fixture restoration on both servers.

use capture_diff::diff::{DiffReport, DivergenceSignature};
use capture_diff::lineage::{ImportSelection, verify_required_lineage};
use capture_diff::model::{Direction, PacketBoundary};
use capture_diff::semantic::{SMSG_BUY_SUCCEEDED, decode_buy_succeeded_body};
use capture_diff::{flow, pkt, rustdump};

const SMSG_ITEM_PUSH_RESULT: u16 = 0x2623;

fn load_vendor_diff() -> (DiffReport, flow::Flow) {
    let flow =
        flow::load_flow("vendor-extended-cost-purchase").expect("vendor purchase flow must exist");
    let cpp = pkt::parse_pkt_file(&flow.golden_pkt).expect("parse vendor C++ cpp.pkt");
    let rust =
        rustdump::parse_rust_dump(&flow.reference_rust).expect("parse vendor Rust packet dump");
    let report = DiffReport::compute(&cpp, &rust, &flow.directions);
    (report, flow)
}

#[test]
fn vendor_purchase_diff_matches_clean_committed_baseline() {
    let (report, flow) = load_vendor_diff();
    let expected_text =
        std::fs::read_to_string(&flow.expected).expect("read expected-divergences.json");
    let expected: Vec<DivergenceSignature> =
        serde_json::from_str(&expected_text).expect("parse expected-divergences.json");

    assert!(
        expected.is_empty(),
        "vendor purchase must not pin accepted divergences"
    );
    assert_eq!(report.signatures(), expected, "{}", report.render_text());
    assert!(report.is_clean(), "{}", report.render_text());
    assert_eq!(report.counts.matched, 2);
    assert_eq!(report.counts.body_mismatches, 0);
    assert_eq!(report.counts.connection_mismatches, 0);
    assert_eq!(report.counts.missing_in_rust, 0);
    assert_eq!(report.counts.extra_in_rust, 0);

    let semantic = report.ops[0]
        .body
        .as_ref()
        .and_then(|body| body.semantic.as_ref())
        .expect("BuySucceeded must exercise the reviewed GUID comparator");
    assert_eq!(
        semantic.comparator,
        "smsg_buy_succeeded_without_vendor_runtime_guid_counter"
    );
    assert!(semantic.is_identical());
}

#[test]
fn vendor_purchase_golden_pins_order_route_and_purchase_fields() {
    let flow =
        flow::load_flow("vendor-extended-cost-purchase").expect("vendor purchase flow must exist");
    let cpp = pkt::parse_pkt_file(&flow.golden_pkt).expect("parse vendor C++ cpp.pkt");
    let rust =
        rustdump::parse_rust_dump(&flow.reference_rust).expect("parse vendor Rust packet dump");

    let expected = [SMSG_BUY_SUCCEEDED, SMSG_ITEM_PUSH_RESULT];
    for capture in [&cpp, &rust] {
        assert_eq!(capture.packets.len(), expected.len());
        for (packet, opcode) in capture.packets.iter().zip(expected) {
            assert_eq!(packet.direction, Direction::S2C);
            assert_eq!(packet.connection_id, 0, "response must use realm");
            assert_eq!(packet.opcode, opcode);
        }
        let success = decode_buy_succeeded_body(&capture.packets[0].body)
            .expect("decode vendor BuySucceeded");
        assert_eq!(success.vendor.high_type, 8);
        assert_eq!(success.vendor.realm_id, 1);
        assert_eq!(success.vendor.map_id, 530);
        assert_eq!(success.vendor.entry, 18_525);
        assert_eq!(success.vendor.subtype, 0);
        assert_eq!(success.vendor.server_id, 0);
        assert_eq!(success.muid, 59);
        assert_eq!(success.new_quantity, -1);
        assert_eq!(success.quantity_bought, 1);
    }
    assert_ne!(
        cpp.packets[0].body, rust.packets[0].body,
        "fixture must retain the distinct real runtime counters"
    );
}

#[test]
fn vendor_purchase_lineage_revalidates_manifests_and_bot_reports() {
    let flow =
        flow::load_flow("vendor-extended-cost-purchase").expect("vendor purchase flow must exist");
    let flow_dir = flow
        .golden_pkt
        .parent()
        .expect("vendor golden must have a flow directory");
    let selection = ImportSelection::new(
        vec![Direction::S2C],
        Some(PacketBoundary {
            direction: Some(Direction::S2C),
            opcode: SMSG_BUY_SUCCEEDED,
        }),
        Some(PacketBoundary {
            direction: Some(Direction::S2C),
            opcode: SMSG_ITEM_PUSH_RESULT,
        }),
        &[],
        true,
    );

    verify_required_lineage("vendor-extended-cost-purchase", flow_dir, &selection)
        .expect("vendor lineage and retained reports must verify");
}
