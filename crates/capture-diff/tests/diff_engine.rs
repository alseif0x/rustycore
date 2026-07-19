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

#[allow(clippy::too_many_arguments)]
fn write_dump_record(
    dir: &Path,
    direction: &str,
    seq: u64,
    counter: u64,
    connection_id: &str,
    opcode: u16,
    name: &str,
    body: &[u8],
) -> String {
    std::fs::create_dir_all(dir).unwrap();
    let len = body.len() + 2;
    let stem = format!("rust-{direction}-{seq:08}-counter{counter}-0x{opcode:04X}-{name}-len{len}");
    let mut bin = opcode.to_le_bytes().to_vec();
    bin.extend_from_slice(body);
    std::fs::write(dir.join(format!("{stem}.bin")), bin).unwrap();
    std::fs::write(
        dir.join(format!("{stem}.meta")),
        format!(
            "direction={direction}\nconnection_id={connection_id}\naddr=127.0.0.1:0\nseq={seq}\ncounter={counter}\nopcode=0x{opcode:04X}\nname={name}\nlen={len}\n"
        ),
    )
    .unwrap();
    stem
}

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
fn pkt_rejects_opcode_wider_than_world_opcode() {
    let cap = Capture::new("wide", vec![s2c(0x1234, &[])]);
    let mut bytes = pkt::write_pkt_bytes(&cap);
    // PKT 3.1 log header (66) + packet header (20) + optional address (20).
    bytes[106..110].copy_from_slice(&0x0001_0000_u32.to_le_bytes());
    let error = pkt::parse_pkt_bytes(&bytes).expect_err("wide opcode must fail closed");
    assert!(error.to_string().contains("16-bit world opcode space"));
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
    // SMSG_AUTH_CHALLENGE-ish: opcode 0x256D, body [9], dumped pre-encryption.
    write_dump_record(
        &dir,
        "s2c-unencrypted",
        0,
        0,
        "0",
        0x256D,
        "AuthResponse",
        &[9],
    );
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
    write_dump_record(&dir, "s2c", 0, 0, "instance", 0x256D, "AuthResponse", &[9]);

    let error = rustdump::parse_rust_dump(&dir).unwrap_err();
    assert!(error.to_string().contains("invalid connection_id"));
}

#[test]
fn rust_dump_rejects_duplicate_global_sequence_after_process_restart() {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("duplicate_global_sequence");
    let _ = std::fs::remove_dir_all(&dir);
    write_dump_record(&dir, "s2c", 7, 0, "1", 0x271C, "StandStateUpdate", &[0]);
    write_dump_record(&dir, "s2c", 7, 1, "1", 0x27CB, "UpdateObject", &[0]);

    let error = rustdump::parse_rust_dump(&dir).expect_err("duplicate seq must fail closed");
    assert!(
        error
            .to_string()
            .contains("non-contiguous packet dump seq 7 then 7")
    );
    assert!(error.to_string().contains("may have restarted"));
}

#[test]
fn rust_dump_requires_contiguous_sequence_but_not_zero_origin() {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("sequence_contract");
    let _ = std::fs::remove_dir_all(&dir);
    write_dump_record(&dir, "c2s", 41, 5, "1", 0x3211, "LootItem", &[0]);
    write_dump_record(&dir, "s2c", 42, 9, "0", 0x2615, "LootRemoved", &[0]);
    assert_eq!(rustdump::parse_rust_dump(&dir).unwrap().packets.len(), 2);

    let stem = write_dump_record(&dir, "s2c", 44, 10, "1", 0x27CB, "UpdateObject", &[0]);
    let error = rustdump::parse_rust_dump(&dir).expect_err("sequence gap must fail");
    assert!(error.to_string().contains("seq 42 then 44"));
    std::fs::remove_file(dir.join(format!("{stem}.meta"))).unwrap();
    std::fs::remove_file(dir.join(format!("{stem}.bin"))).unwrap();
}

#[test]
fn rust_dump_rejects_orphans_extras_subdirectories_and_symlinks() {
    let root = Path::new(env!("CARGO_TARGET_TMPDIR")).join("flat_inventory_contract");
    let reset = || {
        let _ = std::fs::remove_dir_all(&root);
        write_dump_record(&root, "c2s", 0, 0, "1", 0x3211, "LootItem", &[0])
    };

    let stem = reset();
    std::fs::remove_file(root.join(format!("{stem}.bin"))).unwrap();
    assert!(
        rustdump::parse_rust_dump(&root)
            .unwrap_err()
            .to_string()
            .contains("orphan .meta")
    );

    reset();
    std::fs::write(root.join("notes.txt"), b"not evidence").unwrap();
    assert!(
        rustdump::parse_rust_dump(&root)
            .unwrap_err()
            .to_string()
            .contains("unexpected file")
    );

    reset();
    std::fs::create_dir(root.join("nested")).unwrap();
    assert!(
        rustdump::parse_rust_dump(&root)
            .unwrap_err()
            .to_string()
            .contains("not a regular")
    );

    reset();
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(root.join(format!("{stem}.bin")), root.join("linked.bin"))
            .unwrap();
        assert!(
            rustdump::parse_rust_dump(&root)
                .unwrap_err()
                .to_string()
                .contains("non-symlink")
        );
    }
}

#[test]
fn rust_dump_binds_metadata_filename_and_binary_length() {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("metadata_binding_contract");
    let _ = std::fs::remove_dir_all(&dir);
    let stem = write_dump_record(&dir, "c2s", 0, 0, "1", 0x3211, "LootItem", &[1, 2]);
    let meta = dir.join(format!("{stem}.meta"));
    let original = std::fs::read_to_string(&meta).unwrap();

    std::fs::write(&meta, original.replace("len=4", "len=5")).unwrap();
    let error = rustdump::parse_rust_dump(&dir).expect_err("filename/meta mismatch must fail");
    assert!(error.to_string().contains("filename stem disagrees"));

    std::fs::write(&meta, &original).unwrap();
    let bin = dir.join(format!("{stem}.bin"));
    std::fs::write(&bin, [0x11, 0x32, 1]).unwrap();
    let error = rustdump::parse_rust_dump(&dir).expect_err("body length mismatch must fail");
    assert!(error.to_string().contains("metadata declares 4"));

    std::fs::write(&bin, [0x11, 0x32, 1, 2]).unwrap();
    std::fs::write(&meta, format!("{original}unknown=value\n")).unwrap();
    assert!(
        rustdump::parse_rust_dump(&dir)
            .unwrap_err()
            .to_string()
            .contains("unknown field")
    );

    std::fs::write(
        &meta,
        original.replace("addr=127.0.0.1:0", "addr=not-a-socket"),
    )
    .unwrap();
    assert!(
        rustdump::parse_rust_dump(&dir)
            .unwrap_err()
            .to_string()
            .contains("invalid socket address")
    );

    std::fs::write(
        &meta,
        original.replace("direction=c2s", "direction=c2s-unencrypted-unencrypted"),
    )
    .unwrap();
    assert!(
        rustdump::parse_rust_dump(&dir)
            .unwrap_err()
            .to_string()
            .contains("invalid direction")
    );
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
        semantic_mismatch: None,
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
