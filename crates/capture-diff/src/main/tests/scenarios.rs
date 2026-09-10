//! Capture-diff CLI regressions.
//!
//! Moved out of main.rs under #685; every test is unchanged.

use super::*;

#[test]
fn strict_import_accepts_awaiting_required_contract_with_valid_shape() {
    let cpp = valid_required_loot_capture("cpp");
    let rust = valid_required_loot_capture("rust");
    let opts = reviewed_required_import_opts();

    validate_required_import(
        "loot-single-item-claim",
        &[Direction::S2C, Direction::C2S],
        &opts,
        &cpp,
        &rust,
    )
    .unwrap();
}

#[test]
fn strict_import_rejects_required_contract_direction_mismatch() {
    let cpp = valid_required_loot_capture("cpp");
    let rust = valid_required_loot_capture("rust");
    let opts = reviewed_required_import_opts();

    let error = validate_required_import(
        "loot-single-item-claim",
        &[Direction::S2C],
        &opts,
        &cpp,
        &rust,
    )
    .expect_err("required flow directions must be exact");

    assert!(error.to_string().contains("reviewed contract"));
}

#[test]
fn strict_import_rejects_required_contract_wrong_route() {
    let cpp = valid_required_loot_capture("cpp");
    let mut rust = valid_required_loot_capture("rust");
    let opts = reviewed_required_import_opts();
    rust.packets[2].connection_id = 0;

    let error = validate_required_import(
        "loot-single-item-claim",
        &[Direction::S2C, Direction::C2S],
        &opts,
        &cpp,
        &rust,
    )
    .expect_err("required flow route must be exact");

    let message = error.to_string();
    assert!(message.contains("packet 2"));
    assert!(message.contains("s2c conn=1 0x2615"));
}

#[test]
fn strict_import_rejects_required_contract_wrong_boundary() {
    let mut cpp = valid_required_loot_capture("cpp");
    let rust = valid_required_loot_capture("rust");
    let opts = reviewed_required_import_opts();
    cpp.packets
        .insert(0, routed_packet(Direction::S2C, 1, 0x2DD4));

    let error = validate_required_import(
        "loot-single-item-claim",
        &[Direction::S2C, Direction::C2S],
        &opts,
        &cpp,
        &rust,
    )
    .expect_err("required flow boundary must be exact");

    assert!(error.to_string().contains("required boundary"));
}

#[test]
fn strict_import_rejects_an_extra_approved_ignore_for_required_flow() {
    let cpp = valid_required_loot_capture("cpp");
    let rust = valid_required_loot_capture("rust");
    let mut opts = reviewed_required_import_opts();
    opts.ignored_opcodes.push(PacketBoundary {
        direction: Some(Direction::S2C),
        opcode: 0x2DD4,
    });

    let error = validate_required_import(
        "loot-single-item-claim",
        &[Direction::S2C, Direction::C2S],
        &opts,
        &cpp,
        &rust,
    )
    .expect_err("an extra filter must not be able to hide required-flow traffic");
    assert!(error.to_string().contains("reviewed contract"));
}

#[test]
fn parses_directional_packet_boundary() {
    assert_eq!(
        parse_packet_boundary("c2s:0x318C", "--from-opcode").unwrap(),
        PacketBoundary {
            direction: Some(Direction::C2S),
            opcode: 0x318C,
        }
    );
    assert_eq!(
        parse_packet_boundary("S2C:271c", "--until-opcode").unwrap(),
        PacketBoundary {
            direction: Some(Direction::S2C),
            opcode: 0x271C,
        }
    );
}

#[test]
fn preserves_legacy_directionless_until_boundary() {
    assert_eq!(
        parse_packet_boundary("0x3A46", "--until-opcode").unwrap(),
        PacketBoundary {
            direction: None,
            opcode: 0x3A46,
        }
    );
}

#[test]
fn rejects_unknown_boundary_direction() {
    let err = parse_packet_boundary("both:0x318C", "--from-opcode").unwrap_err();
    assert!(err.to_string().contains("use c2s:0xNNNN or s2c:0xNNNN"));
}

#[test]
fn shared_diff_and_import_selection_slices_and_filters_ambient_packets() {
    let opts = parse_opts(&[
        "stand-state".into(),
        "--from-opcode".into(),
        "c2s:0x318C".into(),
        "--until-opcode".into(),
        "c2s:0x3768".into(),
        "--ignore-opcode".into(),
        "s2c:0x2DD4".into(),
    ])
    .unwrap();
    let capture = Capture::new(
        "full-session",
        vec![
            packet(Direction::S2C, 0x256D),
            packet(Direction::C2S, 0x318C),
            ambient_packet(Direction::S2C, 1, 0x2DD4, minimal_monster_move_body()),
            packet(Direction::S2C, 0x271C),
            packet(Direction::C2S, 0x3768),
            packet(Direction::S2C, 0x304E),
        ],
    );

    let sliced = apply_capture_selection(capture, &opts).unwrap();
    assert_eq!(
        sliced
            .packets
            .iter()
            .map(|packet| (packet.direction, packet.opcode))
            .collect::<Vec<_>>(),
        vec![
            (Direction::C2S, 0x318C),
            (Direction::S2C, 0x271C),
            (Direction::C2S, 0x3768),
        ]
    );
}

#[test]
fn import_can_remove_only_the_reviewed_time_sync_request_response_pair() {
    let opts = parse_opts(&[
        "loot-single-item-claim".into(),
        "--from-opcode".into(),
        "c2s:0x3211".into(),
        "--until-opcode".into(),
        "c2s:0x3768".into(),
        "--ignore-opcode".into(),
        "s2c:0x2DD2".into(),
        "--ignore-opcode".into(),
        "c2s:0x3A3D".into(),
    ])
    .unwrap();
    let capture = Capture::new(
        "loot-window",
        vec![
            packet(Direction::C2S, 0x3211),
            ambient_packet(Direction::S2C, 1, 0x2DD2, 7_u32.to_le_bytes().to_vec()),
            ambient_packet(Direction::C2S, 1, 0x3A3D, {
                let mut body = 7_u32.to_le_bytes().to_vec();
                body.extend_from_slice(&1234_u32.to_le_bytes());
                body
            }),
            packet(Direction::S2C, 0x2615),
            packet(Direction::C2S, 0x3768),
        ],
    );

    let selected = apply_capture_selection(capture, &opts).unwrap();
    assert_eq!(
        selected
            .packets
            .iter()
            .map(|packet| (packet.direction, packet.opcode))
            .collect::<Vec<_>>(),
        vec![
            (Direction::C2S, 0x3211),
            (Direction::S2C, 0x2615),
            (Direction::C2S, 0x3768),
        ]
    );
}

#[test]
fn ignored_opcode_requires_an_explicit_direction() {
    let error = parse_opts(&[
        "stand-state".into(),
        "--ignore-opcode".into(),
        "0x2DD4".into(),
    ])
    .err()
    .expect("directionless ignore must fail closed");
    assert!(error.to_string().contains("requires a direction"));
}

#[test]
fn ignored_opcode_rejects_functional_packets_outside_the_ambient_allowlist() {
    let opts = parse_opts(&[
        "stand-state".into(),
        "--ignore-opcode".into(),
        "s2c:0x271C".into(),
    ])
    .unwrap();
    let error = apply_capture_selection(Capture::new("capture", Vec::new()), &opts)
        .expect_err("stand-state ACK must never be filterable");

    assert!(error.to_string().contains("not approved ambient traffic"));
}

#[test]
fn ignored_ambient_opcode_cannot_also_be_an_action_boundary() {
    let opts = parse_opts(&[
        "stand-state".into(),
        "--until-opcode".into(),
        "s2c:0x2DD4".into(),
        "--ignore-opcode".into(),
        "s2c:0x2DD4".into(),
    ])
    .unwrap();
    let error = apply_capture_selection(Capture::new("capture", Vec::new()), &opts)
        .expect_err("an action boundary must never be filterable");

    assert!(
        error
            .to_string()
            .contains("cannot remove an action boundary")
    );
}

#[test]
fn ignored_time_sync_requires_a_well_formed_matched_pair_on_instance_socket() {
    let opts = parse_opts(&[
        "loot-single-item-claim".into(),
        "--ignore-opcode".into(),
        "s2c:0x2DD2".into(),
        "--ignore-opcode".into(),
        "c2s:0x3A3D".into(),
    ])
    .unwrap();
    let request = ambient_packet(Direction::S2C, 1, 0x2DD2, 9_u32.to_le_bytes().to_vec());
    let response = ambient_packet(Direction::C2S, 1, 0x3A3D, {
        let mut body = 9_u32.to_le_bytes().to_vec();
        body.extend_from_slice(&44_u32.to_le_bytes());
        body
    });
    assert!(
        apply_capture_selection(
            Capture::new("valid", vec![request.clone(), response.clone()]),
            &opts
        )
        .unwrap()
        .packets
        .is_empty()
    );

    let mut wrong_socket = request.clone();
    wrong_socket.connection_id = 0;
    let error = apply_capture_selection(
        Capture::new("wrong socket", vec![wrong_socket, response.clone()]),
        &opts,
    )
    .expect_err("wrong socket must fail");
    assert!(error.to_string().contains("expected instance connection 1"));

    let error = apply_capture_selection(Capture::new("orphan", vec![response.clone()]), &opts)
        .expect_err("orphan response must fail");
    assert!(error.to_string().contains("orphan or duplicate"));

    let error = apply_capture_selection(
        Capture::new(
            "duplicate",
            vec![request.clone(), request.clone(), response],
        ),
        &opts,
    )
    .expect_err("duplicate request must fail");
    assert!(error.to_string().contains("duplicates time-sync request"));

    let malformed = ambient_packet(Direction::S2C, 1, 0x2DD2, vec![9, 0, 0]);
    let error = apply_capture_selection(Capture::new("malformed", vec![malformed]), &opts)
        .expect_err("malformed request must fail");
    assert!(error.to_string().contains("malformed 3-byte body"));
}

#[test]
fn paired_selection_rejects_asymmetric_ambient_counts() {
    let opts = parse_opts(&[
        "stand-state".into(),
        "--ignore-opcode".into(),
        "s2c:0x2DD4".into(),
    ])
    .unwrap();
    let body = minimal_monster_move_body();
    let cpp = Capture::new("cpp", vec![ambient_packet(Direction::S2C, 1, 0x2DD4, body)]);
    let rust = Capture::new("rust", Vec::new());
    let error = apply_capture_pair_selection(cpp, rust, &opts)
        .expect_err("different ignore cardinality must fail");
    assert!(error.to_string().contains("count mismatch"));
}

#[test]
fn ignored_monster_move_requires_instance_route_and_structural_body() {
    let opts = parse_opts(&[
        "stand-state".into(),
        "--ignore-opcode".into(),
        "s2c:0x2DD4".into(),
    ])
    .unwrap();
    let malformed = ambient_packet(Direction::S2C, 1, 0x2DD4, vec![1; 12]);
    let error = apply_capture_selection(Capture::new("malformed", vec![malformed]), &opts)
        .expect_err("truncated movement must fail");
    assert!(
        error
            .to_string()
            .contains("malformed monster-movement body")
    );

    let valid_body = minimal_monster_move_body();
    let wrong_route = ambient_packet(Direction::S2C, 0, 0x2DD4, valid_body);
    let error = apply_capture_selection(Capture::new("wrong route", vec![wrong_route]), &opts)
        .expect_err("realm-routed movement must fail");
    assert!(error.to_string().contains("expected instance connection 1"));

    let mut trailing = minimal_monster_move_body();
    trailing.push(0);
    let error = apply_capture_selection(
        Capture::new(
            "trailing bytes",
            vec![ambient_packet(Direction::S2C, 1, 0x2DD4, trailing)],
        ),
        &opts,
    )
    .expect_err("trailing movement bytes must fail");
    assert!(error.to_string().contains("malformed monster-movement"));
}

#[test]
fn time_sync_ignore_cannot_be_declared_one_sided_or_duplicated() {
    let one_sided =
        parse_opts(&["flow".into(), "--ignore-opcode".into(), "s2c:0x2DD2".into()]).unwrap();
    assert!(
        apply_capture_selection(Capture::new("capture", Vec::new()), &one_sided)
            .unwrap_err()
            .to_string()
            .contains("request/response pair")
    );

    let duplicate = parse_opts(&[
        "flow".into(),
        "--ignore-opcode".into(),
        "s2c:0x2DD4".into(),
        "--ignore-opcode".into(),
        "s2c:0x2DD4".into(),
    ])
    .unwrap();
    assert!(
        apply_capture_selection(Capture::new("capture", Vec::new()), &duplicate)
            .unwrap_err()
            .to_string()
            .contains("duplicated")
    );
}

#[test]
fn combat_values_ignore_is_scoped_to_detour_chase_flow() {
    let detour = parse_opts(&[
        "detour-chase-around-obstacle".into(),
        "--ignore-opcode".into(),
        "s2c:0x27CB".into(),
    ])
    .unwrap();
    apply_capture_selection(Capture::new("capture", Vec::new()), &detour).unwrap();

    let unrelated = parse_opts(&[
        "stand-state".into(),
        "--ignore-opcode".into(),
        "s2c:0x27CB".into(),
    ])
    .unwrap();
    assert!(
        apply_capture_selection(Capture::new("capture", Vec::new()), &unrelated)
            .unwrap_err()
            .to_string()
            .contains("not approved ambient traffic")
    );
}

#[test]
fn shared_boundaries_reject_start_without_end() {
    let opts = parse_opts(&[
        "stand-state".into(),
        "--from-opcode".into(),
        "c2s:0x318C".into(),
    ])
    .unwrap();
    let err = apply_capture_boundaries(Capture::new("capture", Vec::new()), &opts).unwrap_err();
    assert!(err.to_string().contains("requires --until-opcode"));
}

#[test]
fn update_baseline_rejects_boundaries_that_would_desync_fixtures() {
    let error = cmd_update_baseline(&[
        "stand-state".into(),
        "--until-opcode".into(),
        "c2s:0x3768".into(),
    ])
    .unwrap_err();
    assert!(error.to_string().contains("use import"));

    let filtered_error = cmd_update_baseline(&[
        "stand-state".into(),
        "--ignore-opcode".into(),
        "s2c:0x2DD4".into(),
    ])
    .unwrap_err();
    assert!(filtered_error.to_string().contains("use import"));
}

#[test]
fn strict_ad_hoc_diff_fails_on_connection_mismatch() {
    let root = std::env::temp_dir().join(format!(
        "capture-diff-strict-connection-mismatch-{}",
        std::process::id()
    ));
    let rust_dir = root.join("rust");
    let cpp_path = root.join("cpp.pkt");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();

    let cpp = Capture::new(
        "cpp",
        vec![capture_diff::CapturedPacket {
            direction: Direction::S2C,
            connection_id: 0,
            opcode: 0x271C,
            body: vec![0, 0, 0, 0, 1],
        }],
    );
    let mut rust_packet = cpp.packets[0].clone();
    rust_packet.connection_id = 1;
    let rust = Capture::new("rust", vec![rust_packet]);
    std::fs::write(&cpp_path, pkt::write_pkt_bytes(&cpp)).unwrap();
    rustdump::write_rust_dump(&rust_dir, &rust).unwrap();

    let result = cmd_diff(&[
        "--cpp".into(),
        cpp_path.display().to_string(),
        "--rust".into(),
        rust_dir.display().to_string(),
        "--direction".into(),
        "s2c".into(),
        "--strict".into(),
    ])
    .unwrap();
    assert_eq!(result, ExitCode::FAILURE);
}
