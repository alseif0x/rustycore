//! Focused tests for the issue-#108 `SMSG_BUY_SUCCEEDED` normalization.
//!
//! The paired C++/Rust runs load the same unique G'eras SQL spawn with
//! different map-runtime counters. Only that counter may differ; the complete
//! stable Creature identity, success fields, packet shape, direction and realm
//! routing stay strict.

use capture_diff::diff::DiffReport;
use capture_diff::model::{Capture, CapturedPacket, Direction};
use capture_diff::semantic::{SMSG_BUY_SUCCEEDED, decode_buy_succeeded_body};

const REAL_CPP_BODY: [u8; 22] = [
    0x01, 0xBF, 0x6F, 0x40, 0x17, 0x12, 0x40, 0x42, 0x04, 0x20, 0x3B, 0x00, 0x00, 0x00, 0xFF, 0xFF,
    0xFF, 0xFF, 0x01, 0x00, 0x00, 0x00,
];
const REAL_RUST_BODY: [u8; 22] = [
    0x01, 0xBF, 0xEA, 0x40, 0x17, 0x12, 0x40, 0x42, 0x04, 0x20, 0x3B, 0x00, 0x00, 0x00, 0xFF, 0xFF,
    0xFF, 0xFF, 0x01, 0x00, 0x00, 0x00,
];

#[derive(Clone, Copy)]
struct VendorGuid {
    entry: u32,
    counter: u64,
}

impl VendorGuid {
    fn geras(counter: u64) -> Self {
        Self {
            entry: 18_525,
            counter,
        }
    }

    fn words(self) -> (u64, u64) {
        let high = (8_u64 << 58) // HighGuid::Creature
            | (1_u64 << 42) // realm 1
            | (530_u64 << 29)
            | (u64::from(self.entry) << 6);
        (self.counter, high)
    }
}

fn packed_guid(low: u64, high: u64) -> Vec<u8> {
    let low_bytes = low.to_le_bytes();
    let high_bytes = high.to_le_bytes();
    let low_mask = low_bytes
        .iter()
        .enumerate()
        .fold(0_u8, |mask, (index, byte)| {
            mask | ((*byte != 0) as u8) << index
        });
    let high_mask = high_bytes
        .iter()
        .enumerate()
        .fold(0_u8, |mask, (index, byte)| {
            mask | ((*byte != 0) as u8) << index
        });
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

fn buy_succeeded_body(
    vendor: VendorGuid,
    muid: u32,
    new_quantity: i32,
    quantity_bought: u32,
) -> Vec<u8> {
    let (low, high) = vendor.words();
    let mut body = packed_guid(low, high);
    body.extend(muid.to_le_bytes());
    body.extend(new_quantity.to_le_bytes());
    body.extend(quantity_bought.to_le_bytes());
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
        packet(Direction::S2C, 0, SMSG_BUY_SUCCEEDED, cpp_body),
        packet(Direction::S2C, 0, SMSG_BUY_SUCCEEDED, rust_body),
        Direction::S2C,
    )
}

#[test]
fn real_pair_decodes_and_omits_only_geras_runtime_counter() {
    let cpp = decode_buy_succeeded_body(&REAL_CPP_BODY).expect("decode real C++ body");
    let rust = decode_buy_succeeded_body(&REAL_RUST_BODY).expect("decode real Rust body");
    assert_eq!(cpp, rust);
    assert_eq!(cpp.vendor.high_type, 8);
    assert_eq!(cpp.vendor.realm_id, 1);
    assert_eq!(cpp.vendor.map_id, 530);
    assert_eq!(cpp.vendor.entry, 18_525);
    assert_eq!(cpp.vendor.subtype, 0);
    assert_eq!(cpp.vendor.server_id, 0);
    assert_eq!(cpp.muid, 59);
    assert_eq!(cpp.new_quantity, -1);
    assert_eq!(cpp.quantity_bought, 1);

    let report = s2c_report(REAL_CPP_BODY.to_vec(), REAL_RUST_BODY.to_vec());
    assert!(report.is_clean(), "{}", report.render_text());
    let semantic = report.ops[0]
        .body
        .as_ref()
        .and_then(|body| body.semantic.as_ref())
        .expect("semantic comparator");
    assert_eq!(
        semantic.comparator,
        "smsg_buy_succeeded_without_vendor_runtime_guid_counter"
    );
    assert_eq!(semantic.cpp.raw_body_sha256, None);
    assert_eq!(semantic.rust.raw_body_sha256, None);
}

#[test]
fn reviewed_vendor_requires_exact_fields_and_nonzero_counters() {
    let valid_cpp = buy_succeeded_body(VendorGuid::geras(111), 59, -1, 1);
    for (rust_body, expected) in [
        (
            buy_succeeded_body(VendorGuid::geras(234), 60, -1, 1),
            "MUID",
        ),
        (
            buy_succeeded_body(VendorGuid::geras(234), 59, 0, 1),
            "NewQuantity",
        ),
        (
            buy_succeeded_body(VendorGuid::geras(234), 59, -1, 2),
            "QuantityBought",
        ),
        (
            buy_succeeded_body(VendorGuid::geras(0), 59, -1, 1),
            "zero runtime GUID counter",
        ),
    ] {
        let report = s2c_report(valid_cpp.clone(), rust_body);
        assert!(!report.is_clean(), "accepted {expected} mismatch");
        assert!(
            report.render_text().contains(expected),
            "{}",
            report.render_text()
        );
    }
}

#[test]
fn another_vendor_keeps_its_runtime_counter_byte_strict() {
    let first = buy_succeeded_body(
        VendorGuid {
            entry: 18_526,
            counter: 1,
        },
        59,
        -1,
        1,
    );
    let second = buy_succeeded_body(
        VendorGuid {
            entry: 18_526,
            counter: 2,
        },
        59,
        -1,
        1,
    );
    let report = s2c_report(first, second);
    assert!(!report.is_clean());
    assert!(report.ops[0].body.as_ref().unwrap().semantic.is_none());
}

#[test]
fn malformed_body_and_wrong_direction_never_receive_normalization() {
    let mut trailing = REAL_CPP_BODY.to_vec();
    trailing.push(0xAA);
    let malformed = s2c_report(trailing.clone(), trailing);
    assert!(!malformed.is_clean());
    assert!(
        malformed.ops[0]
            .body
            .as_ref()
            .and_then(|body| body.semantic.as_ref())
            .is_some_and(|semantic| semantic.cpp.decode_error.is_some())
    );

    let wrong_direction = report(
        packet(
            Direction::C2S,
            0,
            SMSG_BUY_SUCCEEDED,
            REAL_CPP_BODY.to_vec(),
        ),
        packet(
            Direction::C2S,
            0,
            SMSG_BUY_SUCCEEDED,
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
}

#[test]
fn vendor_success_still_requires_cpp_realm_socket_routing() {
    let report = report(
        packet(
            Direction::S2C,
            0,
            SMSG_BUY_SUCCEEDED,
            REAL_CPP_BODY.to_vec(),
        ),
        packet(
            Direction::S2C,
            1,
            SMSG_BUY_SUCCEEDED,
            REAL_RUST_BODY.to_vec(),
        ),
        Direction::S2C,
    );
    assert!(!report.is_clean());
    assert_eq!(report.counts.body_mismatches, 0);
    assert_eq!(report.counts.connection_mismatches, 1);
}
