//! QA bot scenarios, part 2.
//!
//! Split out of the inline test module under #630; assertions unchanged.

use super::*;

#[test]
fn homebind_smoke_keeps_match_after_later_unrelated_spell_go() {
    let (caster_low, caster_high) = create_creature_guid_raw(1, 12_196, 733);
    let player_low = 99;
    let player_high = (2u64 << 58) | (1u64 << 42);
    let matching = bind_spell_go_fixture(caster_low, caster_high, player_low, player_high);
    let mut unrelated = matching.clone();
    let spell_offset =
        2 * build_packed_guid(caster_low, caster_high).len() + 2 * build_packed_guid(1, 0).len();
    unrelated[spell_offset..spell_offset + 4].copy_from_slice(&1u32.to_le_bytes());

    let seen = homebind_spell_go_seen_after_packet(
        false,
        &matching,
        caster_low,
        caster_high,
        player_low,
        player_high,
    );
    assert!(homebind_spell_go_seen_after_packet(
        seen,
        &unrelated,
        caster_low,
        caster_high,
        player_low,
        player_high,
    ));
}
#[test]
fn rested_xp_smoke_cli_validation_is_fail_closed_only_when_enabled() {
    const NOW: u64 = 2_000_000_000;
    assert!(validate_rested_xp_cli_values(false, false, 0, 0, 0, 0, NOW).is_ok());
    assert!(validate_rested_xp_cli_values(true, true, 1, 15_274, 86_400, 45, NOW).is_ok());

    let stray_ack = validate_rested_xp_cli_values(false, true, 0, 0, 0, 0, NOW)
        .expect_err("the destructive ACK must not apply to other modes");
    assert!(stray_ack.to_string().contains("only valid"));

    let missing_ack = validate_rested_xp_cli_values(true, false, 1, 15_274, 86_400, 45, NOW)
        .expect_err("rested-XP smoke must require an explicit destructive ACK");
    assert!(missing_ack
        .to_string()
        .contains(ACK_DISPOSABLE_RESTED_XP_FLAG));

    let bot_count = validate_rested_xp_cli_values(true, true, 2, 15_274, 86_400, 45, NOW)
        .expect_err("multiple bots must be rejected");
    assert!(bot_count.to_string().contains("exactly one bot"));

    let entry = validate_rested_xp_cli_values(true, true, 1, 0, 86_400, 45, NOW)
        .expect_err("zero creature entry must be rejected");
    assert!(entry.to_string().contains("must be nonzero"));

    let offline = validate_rested_xp_cli_values(true, true, 1, 15_274, 0, 45, NOW)
        .expect_err("zero offline duration must be rejected");
    assert!(offline.to_string().contains("greater than zero"));

    let overflow =
        validate_rested_xp_cli_values(true, true, 1, 15_274, u64::from(u32::MAX) + 1, 45, u64::MAX)
            .expect_err("legacy C++ cannot represent a wider offline interval");
    assert!(overflow.to_string().contains("uint32"));

    let current_or_future = validate_rested_xp_cli_values(true, true, 1, 15_274, NOW, 45, NOW)
        .expect_err("logout_time=0/future fixtures must be rejected");
    assert!(current_or_future.to_string().contains("Unix timestamp"));

    let timeout = validate_rested_xp_cli_values(true, true, 1, 15_274, 86_400, 0, NOW)
        .expect_err("zero timeout must be rejected");
    assert!(timeout.to_string().contains("greater than zero"));
}
#[test]
fn rested_xp_destructive_ack_parser_accepts_only_the_exact_cli_flag() {
    let mut acknowledged = false;
    assert!(!parse_ack_disposable_rested_xp_arg(
        "--ack-disposable-rested-xp=true",
        &mut acknowledged,
    ));
    assert!(!acknowledged);
    assert!(parse_ack_disposable_rested_xp_arg(
        ACK_DISPOSABLE_RESTED_XP_FLAG,
        &mut acknowledged,
    ));
    assert!(acknowledged);
}
#[test]
fn detour_chase_capture_cli_is_explicit_and_pinned() {
    let manifest = "/fixture/fixture.json";
    assert!(validate_detour_chase_cli_values(false, false, None, 0, None).is_ok());
    assert!(validate_detour_chase_cli_values(
        true,
        true,
        Some(DETOUR_CHASE_FIXTURE_ACCOUNT),
        30,
        Some(manifest),
    )
    .is_ok());

    let stray_ack = validate_detour_chase_cli_values(false, true, None, 0, None)
        .expect_err("the destructive acknowledgement must not leak into another mode");
    assert!(stray_ack.to_string().contains("only valid"));

    let missing_ack = validate_detour_chase_cli_values(
        true,
        false,
        Some(DETOUR_CHASE_FIXTURE_ACCOUNT),
        30,
        Some(manifest),
    )
    .expect_err("capture must require explicit disposable-fixture acknowledgement");
    assert!(missing_ack
        .to_string()
        .contains(ACK_DISPOSABLE_DETOUR_FIXTURE_FLAG));

    let wrong_account = validate_detour_chase_cli_values(
        true,
        true,
        Some("TESTBOT1@bot.local"),
        30,
        Some(manifest),
    )
    .expect_err("the fixture character must remain bound to its pinned bot");
    assert!(wrong_account
        .to_string()
        .contains(DETOUR_CHASE_FIXTURE_ACCOUNT));

    let zero_timeout = validate_detour_chase_cli_values(
        true,
        true,
        Some(DETOUR_CHASE_FIXTURE_ACCOUNT),
        0,
        Some(manifest),
    )
    .expect_err("capture must have a positive deadline");
    assert!(zero_timeout.to_string().contains("greater than zero"));

    let missing_manifest =
        validate_detour_chase_cli_values(true, true, Some(DETOUR_CHASE_FIXTURE_ACCOUNT), 30, None)
            .expect_err("capture must pin an on-disk fixture manifest");
    assert!(missing_manifest
        .to_string()
        .contains("--detour-fixture-manifest"));
}
#[test]
fn detour_chase_destructive_ack_parser_accepts_only_exact_cli_flag() {
    let mut acknowledged = false;
    assert!(!parse_ack_disposable_detour_fixture_arg(
        "--ack-disposable-detour-fixture=true",
        &mut acknowledged,
    ));
    assert!(!acknowledged);
    assert!(parse_ack_disposable_detour_fixture_arg(
        ACK_DISPOSABLE_DETOUR_FIXTURE_FLAG,
        &mut acknowledged,
    ));
    assert!(acknowledged);
}
#[test]
fn committed_detour_chase_manifest_and_assets_match_pinned_contract() {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../crates/capture-diff/flows/detour-chase-around-obstacle/fixture/fixture.json");
    validate_detour_fixture_manifest(&manifest)
        .expect("committed issue #24 fixture manifest/assets must remain exact");
}
#[test]
fn detour_chase_identity_gate_pins_account_and_character() {
    assert!(validate_detour_fixture_identity(DETOUR_CHASE_FIXTURE_ACCOUNT, 15).is_ok());
    assert!(validate_detour_fixture_identity("testbot2@BOT.LOCAL", 15).is_ok());

    let account = validate_detour_fixture_identity("TESTBOT1@bot.local", 15).unwrap_err();
    assert!(account.to_string().contains(DETOUR_CHASE_FIXTURE_ACCOUNT));
    let guid = validate_detour_fixture_identity(DETOUR_CHASE_FIXTURE_ACCOUNT, 14).unwrap_err();
    assert!(guid.to_string().contains("guid 15"));
}
#[test]
fn detour_chase_can_use_pinned_testbot2_without_committing_a_credential() {
    let mut bots = Vec::new();
    add_pinned_detour_fixture_bot_if_missing(&mut bots, true);
    assert_eq!(bots.len(), 1);
    assert_eq!(bots[0].account, DETOUR_CHASE_FIXTURE_ACCOUNT);
    assert_eq!(bots[0].account_id, DETOUR_CHASE_FIXTURE_ACCOUNT_ID);
    assert_eq!(bots[0].character_guid, DETOUR_CHASE_FIXTURE_CHARACTER_GUID);
    assert!(bots[0].password.is_empty());
    assert_eq!(
        password_env_name(&bots[0].account),
        "WOW_BOT_PASSWORD_TESTBOT2_BOT_LOCAL"
    );

    let mut configured = vec![config::BotConfig {
        account: DETOUR_CHASE_FIXTURE_ACCOUNT.to_string(),
        password: "local-only".to_string(),
        character_guid: DETOUR_CHASE_FIXTURE_CHARACTER_GUID,
        account_id: DETOUR_CHASE_FIXTURE_ACCOUNT_ID,
        lfg_role: 4,
        class: "priest".to_string(),
        enabled: true,
        session_key_bnet: String::new(),
    }];
    add_pinned_detour_fixture_bot_if_missing(&mut configured, true);
    assert_eq!(configured.len(), 1);

    let mut ordinary = Vec::new();
    add_pinned_detour_fixture_bot_if_missing(&mut ordinary, false);
    assert!(ordinary.is_empty());
}
#[test]
fn detour_chase_options_are_derived_only_from_the_pinned_manifest() {
    let manifest = expected_detour_fixture_manifest();
    let options = detour_chase_options_from_pinned_manifest(&manifest, 30).expect("pinned options");
    assert_eq!(options.map_id, 1);
    assert_eq!(options.target_entry, 15_271);
    assert_eq!(options.target_spawn_guid, 9_102_401);
    assert_eq!(options.target_y, 2_671.667);
    assert_eq!(options.destination_y, 2_691.667);
    assert_eq!(
        options.destination_orientation,
        -std::f32::consts::FRAC_PI_2
    );

    let mut changed = manifest;
    changed.creature.spawn_guid += 1;
    assert!(detour_chase_options_from_pinned_manifest(&changed, 30).is_err());
    assert!(
        detour_chase_options_from_pinned_manifest(&expected_detour_fixture_manifest(), 0).is_err()
    );
}
#[test]
fn detour_chase_combat_movement_and_pong_helpers_match_cpp_layouts() {
    let player = create_player_guid_raw(15, 1);
    let creature = create_creature_guid_raw(1, 15_271, 9_102_401);
    let mut attack_start = build_packed_guid(player.0, player.1);
    attack_start.extend(build_packed_guid(creature.0, creature.1));
    assert_eq!(
        parse_attack_start_guids_like_cpp(&attack_start).unwrap(),
        (player, creature)
    );
    let mut trailing_attack = attack_start;
    trailing_attack.push(0);
    assert!(parse_attack_start_guids_like_cpp(&trailing_attack).is_err());

    let mut monster_move = build_packed_guid(creature.0, creature.1);
    monster_move.extend_from_slice(&[0; 16]);
    assert_eq!(
        monster_move_mover_guid_like_cpp(&monster_move).unwrap(),
        creature
    );
    assert!(monster_move_mover_guid_like_cpp(&[]).is_err());

    assert_eq!(
        parse_pong_serial_like_cpp(&ISSUE_24_PING_FENCE_SERIAL.to_le_bytes()).unwrap(),
        ISSUE_24_PING_FENCE_SERIAL
    );
    assert!(parse_pong_serial_like_cpp(&[0; 3]).is_err());
    assert_eq!(
        build_ping_payload(ISSUE_24_PING_FENCE_SERIAL),
        [
            ISSUE_24_PING_FENCE_WIRE[0],
            ISSUE_24_PING_FENCE_WIRE[1],
            ISSUE_24_PING_FENCE_WIRE[2],
            ISSUE_24_PING_FENCE_WIRE[3],
            0,
            0,
            0,
            0,
        ]
    );
    assert_eq!(ISSUE_24_PING_FENCE_WIRE, *b"DTOR");
    assert_eq!(
        ISSUE_24_PING_FENCE_SERIAL,
        u32::from_le_bytes(ISSUE_24_PING_FENCE_WIRE)
    );
}
#[test]
fn creature_spell_success_requires_disconnect_without_logout() {
    let mut result = BotRunResult {
        creature_spell_capture: true,
        creature_spell_capture_passed: Some(true),
        world_auth: true,
        enum_characters: true,
        player_login_verified: true,
        creature_spell_target_discovered: true,
        creature_spell_heartbeat_sent: true,
        creature_spell_start_opcode: Some(SMSG_SPELL_START),
        creature_spell_go_opcode: Some(SMSG_SPELL_GO),
        creature_spell_spell_id: Some(CREATURE_SPELL_FIXTURE_SPELL_ID),
        creature_spell_go_hit_target_count: Some(1),
        creature_spell_go_miss_target_count: Some(0),
        creature_spell_full_combat_log: Some(false),
        creature_spell_adjacent_start_go: true,
        ..BotRunResult::default()
    };
    assert!(
        !result.success(false, false, false),
        "socket disconnect proof is part of the end-to-end capture"
    );

    result.creature_spell_disconnect_confirmed = true;
    assert!(result.success(false, false, false));
    let json = serde_json::to_value(&result).expect("serialize creature-spell evidence");
    assert_eq!(json["creature_spell_disconnect_confirmed"], true);
    assert_eq!(json["creature_spell_logout_confirmed"], false);

    result.creature_spell_logout_confirmed = true;
    assert!(
        !result.success(false, false, false),
        "a combat logout must not satisfy creature-spell capture"
    );
}
#[test]
fn creature_spell_preflight_rejects_persisted_or_orphaned_ghost_state() {
    validate_creature_spell_no_persisted_ghost_state(0, 0)
        .expect("a clean fixture character has no ghost persistence");

    for (auras, effects) in [(1, 3), (1, 0), (0, 3)] {
        let error = validate_creature_spell_no_persisted_ghost_state(auras, effects)
            .expect_err("any spell 8326 aura/effect state must fail closed");
        let message = error.to_string();
        assert!(message.contains("ghost spell 8326"));
        assert!(message.contains(&format!("auras={auras}")));
        assert!(message.contains(&format!("effects={effects}")));
    }
}
#[test]
fn detour_chase_success_and_json_require_complete_capture_evidence() {
    let mut result = BotRunResult {
        detour_chase_capture: true,
        detour_chase_capture_passed: Some(true),
        world_auth: true,
        enum_characters: true,
        player_login_verified: true,
        detour_chase_target_discovered: true,
        detour_chase_active_mover_ack_sent: true,
        detour_chase_attack_start_confirmed: true,
        detour_chase_first_swing_confirmed: true,
        detour_chase_heartbeat_sent: true,
        detour_chase_heartbeat_sha256: Some("cd".repeat(32)),
        detour_chase_window_target_moves: 1,
        detour_chase_monster_move_sha256: Some("ab".repeat(32)),
        detour_chase_monster_move_bytes: Some(96),
        detour_chase_ping_serial: Some(ISSUE_24_PING_FENCE_SERIAL),
        detour_chase_pong_confirmed: true,
        ..BotRunResult::default()
    };
    assert!(
        !result.success(false, false, false),
        "clean logout is part of the end-to-end capture proof"
    );
    result.detour_chase_logout_confirmed = true;
    assert!(result.success(false, false, false));

    result.detour_chase_failure = Some("fixture failure".to_string());
    let json = serde_json::to_value(&result).expect("serialize detour evidence");
    assert_eq!(json["detour_chase_window_target_moves"], 1);
    assert_eq!(json["detour_chase_monster_move_bytes"], 96);
    assert_eq!(json["detour_chase_ping_serial"], ISSUE_24_PING_FENCE_SERIAL);
    assert_eq!(json["detour_chase_failure"], "fixture failure");
}
#[test]
fn detour_chase_discovery_exposes_spatial_ambiguity_instead_of_picking_nearest() {
    let mut payload =
        rested_xp_create_object_fixture(1, 15_271, 71, -10_118.333, 2_671.667, 218.49);
    payload.extend(rested_xp_create_object_fixture(
        1,
        15_271,
        72,
        -10_118.233,
        2_671.667,
        218.49,
    ));
    let candidates = find_creature_guids_near_position_in_update_object(
        &payload,
        1,
        15_271,
        -10_118.333,
        2_671.667,
        218.49,
        DETOUR_CHASE_TARGET_MATCH_RADIUS,
        None,
    );
    assert_eq!(
        candidates
            .iter()
            .map(|candidate| candidate.low)
            .collect::<Vec<_>>(),
        [71, 72]
    );
}
#[test]
fn detour_chase_prelogin_failures_still_serialize_a_result() {
    let bot = config::BotConfig {
        account: DETOUR_CHASE_FIXTURE_ACCOUNT.to_string(),
        password: String::new(),
        character_guid: 15,
        account_id: 2,
        lfg_role: 2,
        class: "WARRIOR".to_string(),
        enabled: true,
        session_key_bnet: String::new(),
    };
    let options = detour_chase_options_from_pinned_manifest(
        &expected_detour_fixture_manifest(),
        DEFAULT_DETOUR_CHASE_TIMEOUT_SECS,
    )
    .unwrap();
    let result = detour_chase_failure_result(&bot, 259, &options, "preflight failed".to_string());
    assert!(!result.success(false, false, false));
    let json = serde_json::to_value(result).unwrap();
    assert_eq!(json["account"], DETOUR_CHASE_FIXTURE_ACCOUNT);
    assert_eq!(json["character_guid"], 15);
    assert_eq!(json["detour_chase_target_spawn_guid"], 9_102_401);
    assert_eq!(json["detour_chase_failure"], "preflight failed");
}
#[test]
fn rested_xp_fixture_safety_requires_exclusive_clean_disposable_scope() {
    let safe = RestedXpFixtureSafetyState {
        bnet_email_matches_configured_account: true,
        characters_on_game_account: 1,
        game_accounts_on_bnet_account: 1,
        ..RestedXpFixtureSafetyState::default()
    };
    assert!(validate_rested_xp_fixture_safety_state(&safe).is_ok());

    let mut at_login = RestedXpFixtureSafetyState {
        bnet_email_matches_configured_account: true,
        characters_on_game_account: 1,
        game_accounts_on_bnet_account: 1,
        at_login: 0x20,
        ..RestedXpFixtureSafetyState::default()
    };
    let error = validate_rested_xp_fixture_safety_state(&at_login)
        .expect_err("first-login state must be rejected");
    assert!(error.to_string().contains("at_login"));

    at_login.at_login = 0;
    at_login.characters_on_game_account = 2;
    let error = validate_rested_xp_fixture_safety_state(&at_login)
        .expect_err("shared game accounts must be rejected");
    assert!(error.to_string().contains("exactly one character"));

    let dirty = RestedXpFixtureSafetyState {
        bnet_email_matches_configured_account: true,
        characters_on_game_account: 1,
        game_accounts_on_bnet_account: 1,
        nonempty_side_state: vec![("character_inventory".to_string(), 3)],
        ..RestedXpFixtureSafetyState::default()
    };
    let error = validate_rested_xp_fixture_safety_state(&dirty)
        .expect_err("non-restored side tables must be rejected");
    assert!(error.to_string().contains("character_inventory=3"));

    let crossed_identity = RestedXpFixtureSafetyState {
        characters_on_game_account: 1,
        game_accounts_on_bnet_account: 1,
        ..RestedXpFixtureSafetyState::default()
    };
    let error = validate_rested_xp_fixture_safety_state(&crossed_identity)
        .expect_err("a configured bot email must own the selected game account");
    assert!(error.to_string().contains("configured @bot.local"));
}
#[test]
fn rested_xp_cleanup_covers_cpp_generated_character_rows() {
    let labels: Vec<_> = RESTED_XP_CPP_GENERATED_CHARACTER_ROWS
        .iter()
        .map(|(label, _, _)| *label)
        .collect();
    assert_eq!(
        labels,
        [
            "character_glyphs",
            "character_reputation",
            "character_skills"
        ]
    );

    for (label, delete_sql, count_sql) in RESTED_XP_CPP_GENERATED_CHARACTER_ROWS {
        assert!(
            delete_sql.starts_with(&format!("DELETE FROM {label} ")),
            "cleanup for {label} must delete only from its own table"
        );
        assert!(
            delete_sql.ends_with("WHERE guid = ?"),
            "cleanup for {label} must remain scoped to the fixture character"
        );
        assert!(
            count_sql.starts_with(&format!("SELECT COUNT(*) FROM {label} ")),
            "verification for {label} must query the same table"
        );
        assert!(
            count_sql.ends_with("WHERE guid = ?"),
            "verification for {label} must remain scoped to the fixture character"
        );
    }

    assert!(
        RESTED_XP_SELECT_TRAIT_CONFIGS_SQL.contains("WHERE guid = ?"),
        "trait config snapshot must remain scoped to the fixture character"
    );
    assert!(
        RESTED_XP_SELECT_TRAIT_CONFIGS_SQL.ends_with("ORDER BY traitConfigId"),
        "trait config snapshot order must be deterministic"
    );
    assert!(
        RESTED_XP_SELECT_TRAIT_ENTRIES_SQL.contains("WHERE guid = ?"),
        "trait entry snapshot must remain scoped to the fixture character"
    );
    assert!(
        RESTED_XP_SELECT_TRAIT_ENTRIES_SQL
            .ends_with("ORDER BY traitConfigId, traitNodeId, traitNodeEntryId"),
        "trait entry snapshot order must be deterministic"
    );
}
#[test]
fn rested_xp_respawn_cleanup_wait_covers_the_selected_spawn_timer() {
    assert_eq!(rested_xp_respawn_cleanup_wait_secs(120, 300), 315);
    assert_eq!(rested_xp_respawn_cleanup_wait_secs(180, 30), 180);
    assert_eq!(rested_xp_respawn_cleanup_wait_secs(1, 1), 16);
    assert_eq!(
        rested_xp_observed_respawn_remaining_secs(1_360, 1_000).unwrap(),
        375
    );
    assert_eq!(
        rested_xp_observed_respawn_remaining_secs(900, 1_000).unwrap(),
        15
    );
    assert!(rested_xp_observed_respawn_remaining_secs(2_000, 1_000).is_err());
}
#[test]
fn rested_xp_time_sync_response_matches_cpp_wire_layout() {
    assert_eq!(
        build_time_sync_response_payload(0x1122_3344, 0x5566_7788),
        [0x44, 0x33, 0x22, 0x11, 0x88, 0x77, 0x66, 0x55]
    );
    assert_eq!(
        parse_time_sync_request_sequence(&[4, 3, 2, 1]).unwrap(),
        0x0102_0304
    );
    assert!(parse_time_sync_request_sequence(&[0, 1, 2]).is_err());
}
#[test]
fn rested_xp_create_discovery_filters_position_and_runtime_counter() {
    let payload = rested_xp_create_object_fixture(530, 15_274, 77_001, 10_188.0, -6_347.5, 30.5);
    let discovered = find_creature_guid_near_position_in_update_object(
        &payload,
        530,
        15_274,
        10_187.8,
        -6_347.56,
        30.459,
        2.0,
        Some(77_001),
    )
    .expect("matching CREATE_OBJECT must be discovered");
    assert_eq!(discovered.low, 77_001);
    assert!((discovered.x - 10_188.0).abs() < f32::EPSILON);

    assert!(
        find_creature_guid_near_position_in_update_object(
            &payload,
            530,
            15_274,
            10_187.8,
            -6_347.56,
            30.459,
            2.0,
            Some(77_002),
        )
        .is_none(),
        "an override for another runtime object must not bind this SQL-position candidate"
    );
    assert!(
        find_creature_guid_near_position_in_update_object(
            &payload, 530, 15_274, 10_000.0, -6_347.56, 30.459, 2.0, None,
        )
        .is_none(),
        "a same-entry runtime object away from the selected SQL spawn must be rejected"
    );
}
#[test]
fn rested_xp_runtime_override_must_match_discovered_sql_position_candidate() {
    let target = rested_xp_target_fixture(77_001);
    let (low, high) = create_creature_guid_raw(target.map_id, target.entry, 77_001);
    let candidate = DiscoveredCreatureGuid {
        low,
        high,
        x: target.x as f32,
        y: target.y as f32,
        z: target.z as f32,
    };
    assert_eq!(
        resolve_rested_xp_runtime_target(&target, Some(candidate)).unwrap(),
        candidate
    );

    let (wrong_low, wrong_high) = create_creature_guid_raw(target.map_id, target.entry, 77_002);
    let wrong_candidate = DiscoveredCreatureGuid {
        low: wrong_low,
        high: wrong_high,
        ..candidate
    };
    let mismatch = resolve_rested_xp_runtime_target(&target, Some(wrong_candidate))
        .expect_err("a counter from another spawn must fail closed");
    assert!(mismatch
        .to_string()
        .contains("did not match discovered counter"));

    let missing = resolve_rested_xp_runtime_target(&target, None)
        .expect_err("an unobserved override cannot prove SQL spawn identity");
    assert!(missing.to_string().contains("cannot be linked safely"));
}
#[test]
fn rested_xp_realm_routing_rejects_any_instance_duplicate() {
    assert!(validate_rested_xp_instance_post_realm_opcode(SMSG_UPDATE_OBJECT).is_ok());
    let duplicate = validate_rested_xp_instance_post_realm_opcode(SMSG_LOG_XP_GAIN)
        .expect_err("instance XP must invalidate a realm observation");
    assert!(duplicate.to_string().contains("duplicated/misrouted"));
    assert_eq!(NOMINAL_MELEE_RANGE_LIKE_CPP, 5.0);
}
#[test]
fn rested_xp_target_rejects_cpp_dynamic_no_xp_critters_and_vehicle_guids() {
    assert!(validate_rested_xp_target_template(15_274, 1, 0).is_ok());

    let critter = validate_rested_xp_target_template(15_274, CREATURE_TYPE_CRITTER, 0)
        .expect_err("critters must be rejected even without a persisted NO_XP flag");
    assert!(critter.to_string().contains("critter"));

    let vehicle = validate_rested_xp_target_template(15_274, 1, 123)
        .expect_err("vehicles use a different C++ HighGuid and must fail closed");
    assert!(vehicle.to_string().contains("HighGuid::Vehicle"));
}
#[test]
fn rested_xp_offline_math_and_cap_match_both_cpp_references() {
    let wilderness = offline_rest_bonus_like_cpp(400, 86_400, REST_OFFLINE_WILDERNESS_BUBBLE, 1.0);
    let resting = offline_rest_bonus_like_cpp(400, 86_400, REST_OFFLINE_TAVERN_OR_CITY_BUBBLE, 1.0);

    assert!((wilderness - 14.88).abs() < 0.001);
    assert!((resting - 60.0).abs() < 0.001);
    assert_eq!(offline_rest_bonus_like_cpp(400, u64::MAX, 1.0, 1.0), 300.0);
    assert!((REST_BONUS_CAP_NEXT_LEVEL_FACTOR - 0.75).abs() < f32::EPSILON);
}
#[test]
fn rested_xp_saved_state_distinguishes_active_relog_from_offline_save() {
    let active = RestedXpDbState {
        level: 1,
        xp: 100,
        rest_state: REST_STATE_RESTED,
        rest_bonus: 250.0,
        online: 1,
    };

    assert!(
        validate_rested_xp_persistence_state(active, 1, 100, 250.0, 1, "active relog",).is_ok()
    );
    assert!(
        validate_rested_xp_persistence_state(active, 1, 100, 250.0, 0, "offline save",).is_err()
    );
}
#[test]
fn rested_xp_world_config_rate_is_case_insensitive_and_last_wins() {
    let contents = r#"
        Rate.Rest.Offline.InWilderness = 1.0
        rate.rest.offline.inwilderness = "1.5" # effective value
    "#;
    assert_eq!(
        worldserver_config_f32_from_contents(contents, "Rate.Rest.Offline.InWilderness", 0.25,)
            .unwrap(),
        1.5
    );
    assert_eq!(
        worldserver_config_f32_from_contents(contents, "Missing.Rate", 0.25).unwrap(),
        0.25
    );

    let error = worldserver_config_f32_from_contents("Rate.Rest = nope", "Rate.Rest", 1.0)
        .expect_err("malformed configured rate must not fall back silently");
    assert!(error.to_string().contains("invalid Rate.Rest value"));

    let stats = r#"
        PlayerSave.Stats.MinLevel = 80
        playersave.stats.minlevel = "0" # effective value
    "#;
    assert_eq!(
        worldserver_config_u32_from_contents(stats, "PlayerSave.Stats.MinLevel", 1).unwrap(),
        0
    );
    assert_eq!(
        worldserver_config_u32_from_contents(stats, "Missing.Integer", 7).unwrap(),
        7
    );
    let error = worldserver_config_u32_from_contents(
        "PlayerSave.Stats.MinLevel = nope",
        "PlayerSave.Stats.MinLevel",
        0,
    )
    .expect_err("malformed stats gate must not fall back silently");
    assert!(error
        .to_string()
        .contains("invalid PlayerSave.Stats.MinLevel value"));
}
#[test]
fn pinned_instance_port_rejects_invalid_or_different_connect_to_target() {
    assert!(validate_pinned_instance_port(8086, None).is_ok());
    assert!(validate_pinned_instance_port(8086, Some("8086")).is_ok());

    let different = validate_pinned_instance_port(9000, Some("8086"))
        .expect_err("a different advertised instance port must fail closed");
    assert!(different
        .to_string()
        .contains("advertised instance port 9000"));
    assert!(validate_pinned_instance_port(8086, Some("0")).is_err());
    assert!(validate_pinned_instance_port(8086, Some("not-a-port")).is_err());
}
#[test]
fn loot_mode_rejects_generic_account_provisioning() {
    assert!(validate_provisioning_mode(false, false).is_ok());
    assert!(validate_provisioning_mode(false, true).is_ok());
    assert!(validate_provisioning_mode(true, false).is_ok());
    let error = validate_provisioning_mode(true, true)
        .expect_err("loot mode must reject provisioning before any DB mutation");
    assert!(error.to_string().contains("forbid --ensure-test-accounts"));
}
#[test]
fn create_only_provisioning_rejects_partial_identity_collisions() {
    assert_eq!(
        create_only_provisioning_plan(false, false).unwrap(),
        CreateOnlyProvisioningPlan::CreateBoth
    );
    assert_eq!(
        create_only_provisioning_plan(true, true).unwrap(),
        CreateOnlyProvisioningPlan::ValidateExisting
    );
    assert!(create_only_provisioning_plan(true, false).is_err());
    assert!(create_only_provisioning_plan(false, true).is_err());
}
#[test]
fn void_storage_contents_parser_matches_cpp_packed_guid_layout() {
    let mut payload = vec![1];
    payload.extend(build_packed_guid(77, 0x0C00_0400_0000_0000));
    payload.extend(build_packed_guid(0, 0));
    payload.extend_from_slice(&5u32.to_le_bytes());
    payload.extend_from_slice(&2589i32.to_le_bytes());
    payload.extend_from_slice(&0i32.to_le_bytes());
    payload.extend_from_slice(&0i32.to_le_bytes());
    payload.push(0); // no bonus list
    payload.push(0); // zero 6-bit item modifiers

    assert_eq!(
        parse_void_storage_contents(&payload).unwrap(),
        vec![VoidStorageItemWire {
            item_id: 77,
            slot: 5,
            item_entry: 2589,
        }]
    );
    payload.push(0);
    assert!(parse_void_storage_contents(&payload).is_err());
}
#[test]
fn void_storage_success_requires_every_fresh_login_checkpoint() {
    let mut result = BotRunResult {
        world_auth: true,
        enum_characters: true,
        player_login_verified: true,
        void_storage_smoke: true,
        void_storage_smoke_passed: Some(true),
        void_storage_unlock_persisted: true,
        void_storage_deposit_persisted: true,
        void_storage_deposit_relogin_verified: true,
        void_storage_swap_persisted: true,
        void_storage_swap_relogin_verified: true,
        void_storage_withdraw_persisted: true,
        ..BotRunResult::default()
    };
    assert!(!result.success(false, false, false));
    result.void_storage_withdraw_relogin_verified = true;
    assert!(result.success(false, false, false));
}
#[test]
fn void_storage_query_capture_has_its_own_success_contract() {
    let mut result = BotRunResult {
        world_auth: true,
        enum_characters: true,
        player_login_verified: true,
        void_storage_query_capture: true,
        ..BotRunResult::default()
    };
    assert!(!result.success(false, false, false));
    result.void_storage_query_capture_passed = Some(true);
    assert!(result.success(false, false, false));
    assert!(!result.void_storage_smoke);
}
#[test]
fn void_storage_explicit_runtime_guid_does_not_require_login_discovery() {
    assert!(void_storage_login_target_ready(false, false));
    assert!(!void_storage_login_target_ready(true, false));
    assert!(void_storage_login_target_ready(true, true));
}
#[test]
fn explicit_creature_guid_includes_active_realm_like_cpp() {
    assert_eq!(
        create_void_storage_creature_guid_raw(571, 31_810, 24, 1),
        (24, 0x2000_0447_601F_1080)
    );
    assert_eq!(
        create_void_storage_creature_guid_raw(530, 18_525, 111, 0),
        create_creature_guid_raw(530, 18_525, 111)
    );
}
#[test]
fn void_storage_wire_guid_keeps_active_realm_like_runtime() {
    let target = ResolvedCreatureTarget {
        entry: 31_810,
        spawn_guid: 24,
        guid_counter: 24,
        map_id: 571,
        x: 0.0,
        y: 0.0,
        z: 0.0,
        orientation: 0.0,
        packed_guid: Vec::new(),
    };
    assert_eq!(
        vault_keeper_packed_guid(&target, 1),
        [0x01, 0xBF, 24, 0x80, 0x10, 0x1F, 0x60, 0x47, 0x04, 0x20]
    );
}
#[test]
fn void_storage_item_guid_keeps_active_realm_like_runtime() {
    assert_eq!(item_guid_raw(77, 1), (77, (3u64 << 58) | (1u64 << 42)));
    assert_eq!(item_guid_raw(77, 2), (77, (3u64 << 58) | (2u64 << 42)));
}
