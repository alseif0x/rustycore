//! Vendor-sell and basic-movement smoke wire/logic tests.

use super::*;

fn disposable_bot(account: &str) -> config::BotConfig {
    config::BotConfig {
        account: account.to_string(),
        password: String::new(),
        character_guid: 15,
        account_id: 2,
        lfg_role: 2,
        class: "WARRIOR".to_string(),
        enabled: true,
        session_key_bnet: String::new(),
    }
}

#[test]
fn sell_item_payload_matches_cpp_sell_item_read_order() {
    // ItemPackets.cpp SellItem::Read: VendorGUID, ItemGUID, uint32 Amount.
    let (vendor_low, vendor_high) = create_creature_guid_raw(530, 18_525, 0x1234);
    let vendor = build_packed_guid(vendor_low, vendor_high);
    let (item_low, item_high) = item_guid_raw(77, 1);
    let payload = build_sell_item_payload_like_cpp(&vendor, item_low, item_high, 1);
    assert_eq!(
        payload,
        vec![
            0x03, 0x9F, // packed vendor masks (low, high)
            0x34, 0x12, // low counter 0x1234
            0x40, 0x17, 0x12, 0x40, 0x42, 0x20, // Creature high: type 8, map 530, entry 18525
            0x01, 0xA0, // packed item masks
            0x4D, // item db guid 77
            0x04, 0x0C, // Item high: type 3, realm 1
            0x01, 0x00, 0x00, 0x00, // Amount
        ]
    );
    assert_eq!(CMSG_SELL_ITEM, 0x34A2);
    assert_eq!(SMSG_SELL_RESPONSE, 0x26C5);
}

#[test]
fn sell_response_parses_cpp_send_sell_error_layout() {
    // ItemPackets.cpp SellResponse::Write: VendorGUID, uint32 count,
    // int32 Reason, then each item GUID.
    let (vendor_low, vendor_high) = create_creature_guid_raw(530, 18_525, 0x1234);
    let (item_low, item_high) = item_guid_raw(77, 1);
    let mut payload = build_packed_guid(vendor_low, vendor_high);
    payload.extend(1u32.to_le_bytes());
    payload.extend(2i32.to_le_bytes());
    payload.extend(build_packed_guid(item_low, item_high));
    let parsed = parse_sell_response_like_cpp(&payload).unwrap();
    assert_eq!(parsed.vendor_guid, (vendor_low, vendor_high));
    assert_eq!(parsed.reason, 2);
    assert_eq!(parsed.item_guids, vec![(item_low, item_high)]);
    assert_eq!(sell_result_name(parsed.reason), "SELL_ERR_CANT_SELL_ITEM");

    // Player::SendSellError(SELL_ERR_CANT_FIND_VENDOR, nullptr, guid) writes an
    // empty VendorGUID.
    let mut no_vendor = build_packed_guid(0, 0);
    no_vendor.extend(1u32.to_le_bytes());
    no_vendor.extend(3i32.to_le_bytes());
    no_vendor.extend(build_packed_guid(item_low, item_high));
    let parsed = parse_sell_response_like_cpp(&no_vendor).unwrap();
    assert_eq!(parsed.vendor_guid, (0, 0));
    assert_eq!(sell_result_name(parsed.reason), "SELL_ERR_CANT_FIND_VENDOR");

    let mut trailing = payload.clone();
    trailing.push(0);
    assert!(parse_sell_response_like_cpp(&trailing).is_err());
    assert!(parse_sell_response_like_cpp(&payload[..payload.len() - 1]).is_err());
}

#[test]
fn successful_sale_is_the_absence_of_a_sell_response() {
    // ItemDefines.h SellResult has no success value: HandleSellItemOpcode
    // sends nothing on success, so any SMSG_SELL_RESPONSE fails the smoke.
    assert_eq!(sell_result_name(0), "unknown SellResult");
    let mut evidence = VendorSellEvidence {
        vendor_sell_smoke: true,
        vendor_sell_smoke_passed: Some(true),
        vendor_sell_item_sent: true,
        vendor_sell_db_persisted: true,
        vendor_sell_relogin_verified: true,
        vendor_sell_money_delta: Some(13),
        ..VendorSellEvidence::default()
    };
    assert!(evidence.passed());
    evidence.vendor_sell_response_seen = true;
    assert!(!evidence.passed());
    evidence.vendor_sell_response_seen = false;
    evidence.vendor_sell_money_delta = Some(0);
    assert!(!evidence.passed());
}

#[test]
fn vendor_sell_money_delta_requires_gain_and_optional_exact_price() {
    assert_eq!(vendor_sell_money_delta(100, 113, None).unwrap(), 13);
    assert_eq!(vendor_sell_money_delta(100, 113, Some(13)).unwrap(), 13);
    assert!(vendor_sell_money_delta(100, 112, Some(13)).is_err());
    assert!(vendor_sell_money_delta(100, 100, None).is_err());
    assert!(vendor_sell_money_delta(100, 90, None).is_err());
}

#[test]
fn movement_plan_is_start_heartbeats_stop_at_run_speed() {
    let start = MovementSmokePoint {
        map_id: 0,
        x: 100.0,
        y: 200.0,
        z: 50.0,
        orientation: 0.0,
    };
    let plan = build_movement_smoke_plan(&start);
    let opcodes: Vec<u16> = plan.iter().map(|step| step.opcode).collect();
    assert_eq!(opcodes, vec![0x39E4, 0x3A10, 0x3A10, 0x39E6]);
    let flags: Vec<u32> = plan.iter().map(|step| step.movement_flags).collect();
    assert_eq!(flags, vec![1, 1, 1, 0]);
    let times: Vec<u32> = plan.iter().map(|step| step.client_time).collect();
    assert_eq!(times, vec![0, 200, 400, 600]);
    for (index, step) in plan.iter().enumerate() {
        assert!((step.x - (100.0 + 1.4 * index as f32)).abs() < 1e-4);
        assert!((step.y - 200.0).abs() < 1e-4);
        assert_eq!(step.z, 50.0);
        assert_eq!(step.orientation, 0.0);
    }
    // Facing pi/2 runs along +Y and keeps the end point within a few yards.
    let north = MovementSmokePoint {
        orientation: std::f32::consts::FRAC_PI_2,
        ..start
    };
    let last = *build_movement_smoke_plan(&north).last().unwrap();
    assert!((last.x - 100.0).abs() < 1e-4);
    assert!((last.y - 204.2).abs() < 1e-3);
    let distance = ((last.x - start.x).powi(2) + (last.y - start.y).powi(2)).sqrt();
    assert!(distance < 5.0);
}

#[test]
fn movement_step_payload_matches_cpp_movement_info_read() {
    let (player_low, player_high) = create_player_guid_raw(15, 1);
    let step = MovementStep {
        opcode: CMSG_MOVE_START_FORWARD,
        movement_flags: MOVEMENTFLAG_FORWARD,
        client_time: 400,
        x: 1.0,
        y: 2.0,
        z: 3.0,
        orientation: 0.5,
    };
    let payload = movement_step_payload(&step, player_low, player_high);
    let guid = build_packed_guid(player_low, player_high);
    let mut expected = guid.clone();
    expected.extend(1u32.to_le_bytes()); // flags
    expected.extend(0u32.to_le_bytes()); // flags2
    expected.extend(0u32.to_le_bytes()); // flags3
    expected.extend(400u32.to_le_bytes()); // time
    for value in [1.0f32, 2.0, 3.0, 0.5, 0.0, 0.0] {
        expected.extend(value.to_le_bytes()); // XYZO, pitch, step-up
    }
    expected.extend(0u32.to_le_bytes()); // remove forces count
    expected.extend(0u32.to_le_bytes()); // move index
    expected.push(0); // eight presence bits
    assert_eq!(payload, expected);
    // The heartbeat helper keeps its old (flags 0, time 0) bytes.
    let heartbeat = build_move_heartbeat_payload(player_low, player_high, 1.0, 2.0, 3.0, 0.5);
    let mut legacy = expected.clone();
    legacy[guid.len()..guid.len() + 4].copy_from_slice(&0u32.to_le_bytes());
    legacy[guid.len() + 12..guid.len() + 16].copy_from_slice(&0u32.to_le_bytes());
    assert_eq!(heartbeat, legacy);
}

#[test]
fn login_verify_world_parses_cpp_write_order() {
    let mut payload = Vec::new();
    payload.extend(530i32.to_le_bytes());
    for value in [-1863.5f32, 5430.25, -10.5, 3.0] {
        payload.extend(value.to_le_bytes());
    }
    payload.extend(0u32.to_le_bytes());
    let parsed = parse_login_verify_world_like_cpp(&payload).unwrap();
    assert_eq!(
        parsed,
        LoginVerifyWorldPosition {
            map_id: 530,
            x: -1863.5,
            y: 5430.25,
            z: -10.5,
            orientation: 3.0,
            reason: 0,
        }
    );
    assert!(parse_login_verify_world_like_cpp(&payload[..23]).is_err());
}

#[test]
fn persisted_position_tolerance_check() {
    let expected = [100.0, 200.0, 50.0];
    assert!(position_within_tolerance(0, expected, 0, expected, 0.1));
    assert!(position_within_tolerance(
        0,
        expected,
        0,
        [100.09, 199.91, 50.05],
        MOVEMENT_SMOKE_POSITION_TOLERANCE
    ));
    assert!(!position_within_tolerance(
        0,
        expected,
        0,
        [100.2, 200.0, 50.0],
        MOVEMENT_SMOKE_POSITION_TOLERANCE
    ));
    assert!(!position_within_tolerance(
        0,
        expected,
        1,
        expected,
        MOVEMENT_SMOKE_POSITION_TOLERANCE
    ));
}

#[test]
fn fixtures_refuse_non_local_accounts_before_any_database_access() {
    assert!(require_bot_local_account("TESTBOT1@bot.local", "movement").is_ok());
    assert!(require_bot_local_account("player@example.com", "movement").is_err());
    let movement = prepare_movement_smoke_fixture(&disposable_bot("player@example.com"));
    assert!(movement
        .unwrap_err()
        .to_string()
        .contains("refusing destructive movement fixture setup"));
    let cli = VendorSellCli {
        vendor_entry: DEFAULT_VENDOR_ENTRY,
        vendor_spawn_guid: DEFAULT_VENDOR_SPAWN_GUID,
        runtime_counter: None,
        item_entry: DEFAULT_VENDOR_SELL_ITEM_ENTRY,
        expected_money_delta: None,
        timeout_secs: 8,
    };
    let vendor_sell = prepare_vendor_sell_fixture(&disposable_bot("player@example.com"), &cli);
    assert!(vendor_sell
        .unwrap_err()
        .to_string()
        .contains("refusing destructive vendor-sell fixture setup"));
}

#[test]
fn report_flattens_vendor_sell_and_movement_fields() {
    let mut result = BotRunResult {
        world_auth: true,
        enum_characters: true,
        player_login_verified: true,
        ..BotRunResult::default()
    };
    result.vendor_sell.vendor_sell_smoke = true;
    let json = serde_json::to_value(&result).unwrap();
    assert_eq!(json["vendor_sell_smoke"], serde_json::json!(true));
    assert_eq!(json["movement_smoke"], serde_json::json!(false));
    assert!(json.get("vendor_sell_response_seen").is_some());
    assert!(json.get("movement_relogin_verified").is_some());
    assert!(!result.success(false, false, false));
    assert!(post_login_smoke_summary(&result)
        .unwrap()
        .starts_with("vendor_sell_smoke"));
}

#[test]
fn cast_lifecycle_character_guid_override_requires_one_single_account_and_reports_it() {
    let configured_bot = config::BotConfig {
        account: "TESTBOT1@bot.local".to_string(),
        password: String::new(),
        character_guid: 15,
        account_id: 2,
        lfg_role: 2,
        class: "WARRIOR".to_string(),
        enabled: true,
        session_key_bnet: String::new(),
    };
    let mut selected = vec![configured_bot.clone()];

    assert!(
        apply_cast_lifecycle_character_guid_override(&mut selected, Some(41), true, false,)
            .is_err()
    );
    assert!(
        apply_cast_lifecycle_character_guid_override(&mut selected, Some(41), false, true,)
            .is_err()
    );
    let mut multiple = vec![configured_bot.clone(), configured_bot.clone()];
    assert!(
        apply_cast_lifecycle_character_guid_override(&mut multiple, Some(41), true, true,).is_err()
    );
    assert_eq!(selected[0].character_guid, 15);

    apply_cast_lifecycle_character_guid_override(&mut selected, Some(41), true, true).unwrap();
    assert_eq!(selected[0].character_guid, 41);
    assert_eq!(selected[0].account, configured_bot.account);
    assert_eq!(selected[0].password, configured_bot.password);
    assert_eq!(configured_bot.character_guid, 15);

    let failure = cast_lifecycle::failure_result(&selected[0], 259, "preauth failed".to_string());
    let json = serde_json::to_value(failure).unwrap();
    assert_eq!(json["account"], "TESTBOT1@bot.local");
    assert_eq!(json["character_guid"], 41);
    assert_eq!(json["cast_lifecycle"]["failure"], "preauth failed");
}
