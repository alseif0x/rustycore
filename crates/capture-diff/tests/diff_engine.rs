//! Hand-asserted engine tests on small in-memory captures.
//!
//! These deliberately do NOT go through the fixture generator, so they pin the
//! diff/parse/serialize behaviour independently (breaking any circularity with
//! `gen-fixtures`, which uses the same engine to author the golden).

use std::path::Path;

use capture_diff::diff::{DiffReport, DivergenceKind, baseline_delta};
use capture_diff::model::{Capture, CapturedPacket, Direction, PacketBoundary};
use capture_diff::{pkt, rustdump};

fn s2c(opcode: u16, body: &[u8]) -> CapturedPacket {
    CapturedPacket {
        direction: Direction::S2C,
        connection_id: 0,
        opcode,
        body: body.to_vec(),
    }
}

fn c2s(opcode: u16, body: &[u8]) -> CapturedPacket {
    CapturedPacket {
        direction: Direction::C2S,
        connection_id: 0,
        opcode,
        body: body.to_vec(),
    }
}

const ALL: &[Direction] = &[Direction::S2C, Direction::C2S];

#[test]
fn identical_captures_are_clean() {
    let pkts = vec![s2c(0x0001, &[1, 2, 3]), s2c(0x0002, &[4, 5])];
    let a = Capture::new("a", pkts.clone());
    let b = Capture::new("b", pkts);
    let report = DiffReport::compute(&a, &b, ALL);
    assert!(report.is_clean(), "identical captures must diff clean");
    assert!(report.signatures().is_empty());
    assert_eq!(report.counts.matched, 2);
}

#[test]
fn missing_packet_is_reported_once() {
    let cpp = Capture::new("cpp", vec![s2c(0x0001, &[1]), s2c(0x0002, &[2])]);
    let rust = Capture::new("rust", vec![s2c(0x0001, &[1])]);
    let report = DiffReport::compute(&cpp, &rust, ALL);
    assert!(!report.is_clean());
    assert_eq!(report.counts.missing_in_rust, 1);
    assert_eq!(report.counts.extra_in_rust, 0);
    let sigs = report.signatures();
    assert_eq!(sigs.len(), 1);
    assert_eq!(sigs[0].kind, DivergenceKind::MissingInRust);
    assert_eq!(sigs[0].opcode, "0x0002");
}

#[test]
fn extra_packet_is_reported_once() {
    let cpp = Capture::new("cpp", vec![s2c(0x0001, &[1])]);
    let rust = Capture::new("rust", vec![s2c(0x0001, &[1]), s2c(0x00FF, &[9])]);
    let report = DiffReport::compute(&cpp, &rust, ALL);
    assert_eq!(report.counts.extra_in_rust, 1);
    assert_eq!(report.counts.missing_in_rust, 0);
    let sigs = report.signatures();
    assert_eq!(sigs.len(), 1);
    assert_eq!(sigs[0].kind, DivergenceKind::ExtraInRust);
    assert_eq!(sigs[0].opcode, "0x00FF");
}

#[test]
fn body_mismatch_reports_first_diff_offset_and_lengths() {
    let cpp = Capture::new("cpp", vec![s2c(0x0010, &[0xAA, 0xBB, 0xCC])]);
    let rust = Capture::new("rust", vec![s2c(0x0010, &[0xAA, 0xFF])]);
    let report = DiffReport::compute(&cpp, &rust, ALL);
    assert_eq!(report.counts.body_mismatches, 1);
    let sigs = report.signatures();
    assert_eq!(sigs.len(), 1);
    assert_eq!(sigs[0].kind, DivergenceKind::BodyMismatch);
    assert_eq!(sigs[0].cpp_body_len, Some(3));
    assert_eq!(sigs[0].rust_body_len, Some(2));
    assert_eq!(sigs[0].first_diff_offset, Some(1)); // first differing byte
}

#[test]
fn connection_mismatch_is_a_separate_non_clean_divergence() {
    let mut cpp_packet = s2c(0x271C, &[0, 0, 0, 0, 1]);
    cpp_packet.connection_id = 0;
    let mut rust_packet = cpp_packet.clone();
    rust_packet.connection_id = 1;

    let report = DiffReport::compute(
        &Capture::new("cpp", vec![cpp_packet]),
        &Capture::new("rust", vec![rust_packet]),
        ALL,
    );

    assert!(!report.is_clean());
    assert_eq!(report.counts.matched, 0);
    assert_eq!(report.counts.body_mismatches, 0);
    assert_eq!(report.counts.connection_mismatches, 1);
    let signatures = report.signatures();
    assert_eq!(signatures.len(), 1);
    assert_eq!(signatures[0].kind, DivergenceKind::ConnectionMismatch);
    assert_eq!(signatures[0].cpp_connection_id, Some(0));
    assert_eq!(signatures[0].rust_connection_id, Some(1));
    let rendered = report.render_text();
    assert!(rendered.contains("~ ROUTE"));
    assert!(rendered.contains("connection cpp=0 rust=1"));
    assert!(rendered.contains("DIVERGENT"));
}

#[test]
fn equal_prefix_but_longer_body_diffs_at_min_len() {
    let cpp = Capture::new("cpp", vec![s2c(0x0010, &[1, 2, 3, 4])]);
    let rust = Capture::new("rust", vec![s2c(0x0010, &[1, 2, 3])]);
    let report = DiffReport::compute(&cpp, &rust, ALL);
    let sigs = report.signatures();
    assert_eq!(sigs.len(), 1);
    assert_eq!(sigs[0].kind, DivergenceKind::BodyMismatch);
    assert_eq!(sigs[0].first_diff_offset, Some(3)); // == min length
}

#[test]
fn reorder_surfaces_as_missing_plus_extra() {
    let cpp = Capture::new("cpp", vec![s2c(0x0001, &[]), s2c(0x0002, &[])]);
    let rust = Capture::new("rust", vec![s2c(0x0002, &[]), s2c(0x0001, &[])]);
    let report = DiffReport::compute(&cpp, &rust, ALL);
    assert!(!report.is_clean(), "a reorder is a divergence");
    // One opcode stays matched (the LCS), the moved one shows as missing+extra.
    assert_eq!(report.counts.missing_in_rust, 1);
    assert_eq!(report.counts.extra_in_rust, 1);
}

#[test]
fn directions_are_compared_independently() {
    // Same opcode value in different directions must not cross-match.
    let cpp = Capture::new(
        "cpp",
        vec![
            CapturedPacket {
                direction: Direction::C2S,
                connection_id: 0,
                opcode: 0x0001,
                body: vec![1],
            },
            CapturedPacket {
                direction: Direction::S2C,
                connection_id: 0,
                opcode: 0x0001,
                body: vec![2],
            },
        ],
    );
    let rust = cpp.clone();
    let report = DiffReport::compute(&cpp, &rust, ALL);
    assert!(report.is_clean());
    assert_eq!(report.counts.matched, 2);
}

#[test]
fn directional_slice_includes_deferred_state_through_capture_fence() {
    let capture = Capture::new(
        "full session",
        vec![
            s2c(0x271C, &[0xFF]),
            s2c(0x256D, &[0xAA]),
            c2s(0x318C, &[1, 0, 0, 0]),
            s2c(0x2DD2, &[0xBB]),
            s2c(0x271C, &[0, 0, 0, 0, 1]),
            s2c(0x27CB, &[0xCC]),
            s2c(0x2C1F, &[0xDD]),
            c2s(0x3768, &[0x4E, 0x41, 0x54, 0x53, 0, 0, 0, 0]),
        ],
    );

    let sliced = capture
        .sliced_between(
            PacketBoundary {
                direction: Some(Direction::C2S),
                opcode: 0x318C,
            },
            PacketBoundary {
                direction: Some(Direction::C2S),
                opcode: 0x3768,
            },
        )
        .unwrap();

    assert_eq!(
        sliced.packets,
        vec![
            c2s(0x318C, &[1, 0, 0, 0]),
            s2c(0x2DD2, &[0xBB]),
            s2c(0x271C, &[0, 0, 0, 0, 1]),
            s2c(0x27CB, &[0xCC]),
            s2c(0x2C1F, &[0xDD]),
            c2s(0x3768, &[0x4E, 0x41, 0x54, 0x53, 0, 0, 0, 0]),
        ]
    );
}

#[test]
fn directional_slice_rejects_missing_end_after_request() {
    let capture = Capture::new(
        "failed stand-state flow",
        vec![
            s2c(0x271C, &[0xFF]),
            c2s(0x318C, &[1, 0, 0, 0]),
            s2c(0x271C, &[0, 0, 0, 0, 1]),
            s2c(0x27CB, &[0xCC]),
        ],
    );

    let err = capture
        .sliced_between(
            PacketBoundary {
                direction: Some(Direction::C2S),
                opcode: 0x318C,
            },
            PacketBoundary {
                direction: Some(Direction::C2S),
                opcode: 0x3768,
            },
        )
        .unwrap_err();

    assert!(err.to_string().contains("no end boundary c2s:0x3768"));
}

#[test]
fn pkt_round_trips() {
    let cap = Capture::new(
        "rt",
        vec![
            CapturedPacket {
                direction: Direction::C2S,
                connection_id: 0xAABB_CCDD,
                opcode: 0x35EB,
                body: vec![1, 2, 3],
            },
            CapturedPacket {
                direction: Direction::S2C,
                connection_id: 0,
                opcode: 0x256D,
                body: vec![],
            },
            CapturedPacket {
                direction: Direction::S2C,
                connection_id: 1,
                opcode: 0x270A,
                body: vec![9; 130],
            },
        ],
    );
    let bytes = pkt::write_pkt_bytes(&cap);
    let parsed = pkt::parse_pkt_bytes(&bytes).expect("parse PKT");
    assert_eq!(parsed.packets, cap.packets);
}

#[test]
fn pkt_rejects_bad_signature() {
    let err = pkt::parse_pkt_bytes(b"XYZ\x01\x03").unwrap_err();
    assert!(err.to_string().contains("signature"), "got: {err}");
}

#[test]
fn pkt_rejects_wrong_version() {
    // "PKT" + version 0x0302 (unsupported)
    let mut bytes = b"PKT".to_vec();
    bytes.extend_from_slice(&0x0302u16.to_le_bytes());
    let err = pkt::parse_pkt_bytes(&bytes).unwrap_err();
    assert!(err.to_string().contains("version"), "got: {err}");
}

#[test]
fn rust_dump_round_trips() {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("rust_dump_round_trips");
    let _ = std::fs::remove_dir_all(&dir);
    let cap = Capture::new(
        "rt",
        vec![
            CapturedPacket {
                direction: Direction::S2C,
                connection_id: 1,
                opcode: 0x256D,
                body: vec![1, 2, 3, 4],
            },
            CapturedPacket {
                direction: Direction::C2S,
                connection_id: 0,
                opcode: 0x3187,
                body: vec![2],
            },
        ],
    );
    rustdump::write_rust_dump(&dir, &cap).expect("write dump");
    let parsed = rustdump::parse_rust_dump(&dir).expect("parse dump");
    assert_eq!(parsed.packets, cap.packets);
}

#[test]
fn rust_dump_round_trips_via_pkt_normalization() {
    // A capture written as a Rust dump and a capture written as PKT must
    // normalize to byte-identical packets (the two formats agree).
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("cross_format");
    let _ = std::fs::remove_dir_all(&dir);
    let cap = Capture::new(
        "x",
        vec![CapturedPacket {
            direction: Direction::S2C,
            connection_id: 1,
            opcode: 0x2C27,
            body: vec![7, 7, 7],
        }],
    );
    rustdump::write_rust_dump(&dir, &cap).unwrap();
    let from_dump = rustdump::parse_rust_dump(&dir).unwrap();
    let pkt_bytes = pkt::write_pkt_bytes(&cap);
    let from_pkt = pkt::parse_pkt_bytes(&pkt_bytes).unwrap();
    assert_eq!(from_dump.packets, from_pkt.packets);
}

#[test]
fn rust_dump_accepts_unencrypted_handshake_tags() {
    // The live dumper tags pre-encryption handshake packets as
    // `c2s-unencrypted` / `s2c-unencrypted` (world_socket.rs:639,718). The
    // parser MUST accept them — a real login dump always contains them.
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("unencrypted_tags");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    // SMSG_AUTH_CHALLENGE-ish: opcode 0x256D, body [9], dumped pre-encryption.
    std::fs::write(
        dir.join("rust-s2c-unencrypted-00000000-counter0-0x256D-x-len3.bin"),
        [0x6D, 0x25, 0x09],
    )
    .unwrap();
    std::fs::write(
        dir.join("rust-s2c-unencrypted-00000000-counter0-0x256D-x-len3.meta"),
        "direction=s2c-unencrypted\nseq=0\nopcode=0x256D\nlen=3\n",
    )
    .unwrap();
    let cap = rustdump::parse_rust_dump(&dir).expect("must parse -unencrypted tags");
    assert_eq!(cap.packets.len(), 1);
    assert_eq!(cap.packets[0].direction, Direction::S2C);
    assert_eq!(cap.packets[0].connection_id, 0);
    assert_eq!(cap.packets[0].opcode, 0x256D);
    assert_eq!(cap.packets[0].body, vec![0x09]);
}

#[test]
fn rust_dump_rejects_invalid_explicit_connection_id() {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("invalid_connection_id");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("rust-s2c-00000000-counter0-0x256D-x-len3.bin"),
        [0x6D, 0x25, 0x09],
    )
    .unwrap();
    std::fs::write(
        dir.join("rust-s2c-00000000-counter0-0x256D-x-len3.meta"),
        "direction=s2c\nconnection_id=instance\nseq=0\nopcode=0x256D\nlen=3\n",
    )
    .unwrap();

    let error = rustdump::parse_rust_dump(&dir).unwrap_err();
    assert!(error.to_string().contains("invalid connection_id"));
}

#[test]
fn rust_dump_rejects_duplicate_global_sequence_after_process_restart() {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("duplicate_global_sequence");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    for (name, opcode) in [("first", 0x271C_u16), ("second", 0x27CB_u16)] {
        let stem = format!("rust-s2c-{name}-0x{opcode:04X}");
        let mut bin = opcode.to_le_bytes().to_vec();
        bin.push(0);
        std::fs::write(dir.join(format!("{stem}.bin")), bin).unwrap();
        std::fs::write(
            dir.join(format!("{stem}.meta")),
            format!("direction=s2c\nconnection_id=1\nseq=7\nopcode=0x{opcode:04X}\nlen=3\n"),
        )
        .unwrap();
    }

    let error = rustdump::parse_rust_dump(&dir).expect_err("duplicate seq must fail closed");
    assert!(error.to_string().contains("duplicate packet dump seq 7"));
    assert!(error.to_string().contains("may have restarted"));
}

#[test]
fn baseline_delta_is_count_aware() {
    use capture_diff::DivergenceSignature;
    let sig = |off: usize| DivergenceSignature {
        kind: DivergenceKind::MissingInRust,
        direction: Direction::S2C,
        opcode: "0x2BC5".to_string(),
        name: "ChatServerMessage".to_string(),
        cpp_body_len: None,
        rust_body_len: None,
        first_diff_offset: Some(off),
        cpp_connection_id: None,
        rust_connection_id: None,
    };
    // Baseline has 3 identical signatures (e.g. 3 missing MOTD lines).
    let expected = vec![sig(0), sig(0), sig(0)];

    // Equal multiset → matches.
    assert!(baseline_delta(&[sig(0), sig(0), sig(0)], &expected).matches());

    // Regression 3 → 5 (two more identical) must be caught (set-membership would miss this).
    let regressed = baseline_delta(&[sig(0), sig(0), sig(0), sig(0), sig(0)], &expected);
    assert!(!regressed.matches());
    assert_eq!(regressed.new.len(), 2);
    assert!(regressed.fixed.is_empty());

    // Partial fix 3 → 1 must be caught and reported as fixed.
    let improved = baseline_delta(&[sig(0)], &expected);
    assert!(!improved.matches());
    assert_eq!(improved.fixed.len(), 2);
    assert!(improved.new.is_empty());
}
