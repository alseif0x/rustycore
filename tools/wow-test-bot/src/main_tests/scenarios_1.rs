//! QA bot scenarios, part 1.
//!
//! Split out of the inline test module under #630; assertions unchanged.

use super::*;

#[test]
fn creature_spell_parser_accepts_only_the_pinned_start_and_hit_go_shapes() {
    let (start, _, _, caster, player) = creature_spell_test_body(false, false);
    let parsed_start = parse_creature_spell_cast_data(&start, false).unwrap();
    assert_eq!(parsed_start.caster, caster);
    assert_eq!(parsed_start.target, player);
    assert_eq!(parsed_start.spell_id, CREATURE_SPELL_FIXTURE_SPELL_ID);
    assert_eq!(parsed_start.cast_flags, 0x2);
    assert!(parsed_start.hit_targets.is_empty());
    assert_eq!(parsed_start.full_combat_log, None);
    assert_eq!(parsed_start.consumed, start.len());

    let (go, _, _, _, _) = creature_spell_test_body(true, false);
    let parsed_go = parse_creature_spell_cast_data(&go, true).unwrap();
    assert_eq!(parsed_go.cast_id, parsed_start.cast_id);
    assert_eq!(parsed_go.cast_flags, 0x100);
    assert_eq!(parsed_go.hit_targets, [player]);
    assert!(parsed_go.miss_targets.is_empty());
    assert_eq!(parsed_go.full_combat_log, Some(false));
    assert_eq!(parsed_go.consumed, go.len());
}
#[test]
fn creature_spell_parser_rejects_miss_padding_and_trailing_bytes() {
    let (miss, _, _, _, _) = creature_spell_test_body(true, true);
    assert!(parse_creature_spell_cast_data(&miss, true)
        .unwrap_err()
        .to_string()
        .contains("miss status"));

    let (mut count_padding, counts_offset, _, _, _) = creature_spell_test_body(true, false);
    count_padding[counts_offset + 9] |= 0x01;
    assert!(parse_creature_spell_cast_data(&count_padding, true).is_err());

    let (mut target_padding, _, target_offset, _, _) = creature_spell_test_body(true, false);
    target_padding[target_offset + 4] |= 0x01;
    assert!(parse_creature_spell_cast_data(&target_padding, true).is_err());

    let (mut trailing, _, _, _, _) = creature_spell_test_body(false, false);
    trailing.push(0);
    assert!(parse_creature_spell_cast_data(&trailing, false).is_err());
}
#[test]
fn creature_spell_cli_and_committed_manifest_are_pinned() {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../crates/capture-diff/flows/creature-spell-casting/fixture/fixture.json");
    assert!(validate_creature_spell_capture_cli_values(
        true,
        Some(CREATURE_SPELL_FIXTURE_ACCOUNT),
        30,
        manifest.to_str(),
    )
    .is_ok());
    assert!(validate_creature_spell_capture_cli_values(
        true,
        Some("TESTBOT1@bot.local"),
        30,
        manifest.to_str(),
    )
    .is_err());
    assert!(validate_creature_spell_capture_cli_values(
        true,
        Some(CREATURE_SPELL_FIXTURE_ACCOUNT),
        0,
        manifest.to_str(),
    )
    .is_err());
    assert_eq!(
        validate_creature_spell_fixture_manifest(&manifest).unwrap(),
        CREATURE_SPELL_FIXTURE_MANIFEST_SHA256
    );
    let pinned: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&manifest).unwrap()).unwrap();
    assert_eq!(pinned["schema_version"], 2);
    assert_eq!(pinned["contract"], CREATURE_SPELL_FIXTURE_CONTRACT);
    assert_eq!(
        pinned["creature_template_difficulty"]["original_static_flags_1"],
        0
    );
    assert_eq!(
        pinned["creature_template_difficulty"]["temporary_static_flags_1"],
        0x0010_0000
    );
    for index in 2..=8 {
        assert_eq!(
            pinned["creature_template_difficulty"][format!("static_flags_{index}")],
            0
        );
    }
}
#[test]
fn optional_login_known_spells_gate_waits_for_both_signals() {
    assert!(!login_known_spells_ready(false, false, false));
    assert!(login_known_spells_ready(true, false, false));
    assert!(!login_known_spells_ready(true, true, false));
    assert!(!login_known_spells_ready(false, true, true));
    assert!(login_known_spells_ready(true, true, true));
}
#[test]
fn login_known_spells_expectation_is_an_exact_unique_set() {
    assert_eq!(
        parse_login_known_spells_expectation("822, 75,81").unwrap(),
        vec![75, 81, 822]
    );
    for invalid in ["", "0", "75,nope", "75,75"] {
        assert!(
            parse_login_known_spells_expectation(invalid).is_err(),
            "accepted {invalid:?}"
        );
    }
}
#[test]
fn login_known_spells_decoder_canonicalizes_only_wire_order() {
    fn body(initial_login: bool, known: &[u32], favorites: &[u32]) -> Vec<u8> {
        let mut body = vec![if initial_login { 0x80 } else { 0 }];
        body.extend((known.len() as u32).to_le_bytes());
        body.extend((favorites.len() as u32).to_le_bytes());
        for spell in known.iter().chain(favorites) {
            body.extend(spell.to_le_bytes());
        }
        body
    }

    let decoded = decode_login_known_spells_like_cpp(&body(true, &[822, 75, 81], &[81])).unwrap();
    assert_eq!(
        decoded,
        LoginKnownSpellsLikeCpp {
            initial_login: true,
            known_spells: vec![75, 81, 822],
            favorite_spells: vec![81],
        }
    );

    let mut bad_padding = body(true, &[75], &[]);
    bad_padding[0] |= 1;
    for malformed in [
        bad_padding,
        body(true, &[75, 75], &[]),
        body(true, &[75], &[81]),
        body(true, &[0], &[]),
        body(true, &[75], &[])[..12].to_vec(),
    ] {
        assert!(
            decode_login_known_spells_like_cpp(&malformed).is_err(),
            "accepted malformed body {malformed:02X?}"
        );
    }
}
#[test]
fn loot_result_requires_verified_relog_for_success() {
    let mut result = BotRunResult {
        world_auth: true,
        enum_characters: true,
        player_login_verified: true,
        loot_race_smoke: true,
        loot_race_smoke_passed: Some(true),
        ..BotRunResult::default()
    };

    assert!(!result.success(false, false, false));

    result.loot_race_relog_verified = true;
    assert!(result.success(false, false, false));
}
#[test]
fn vendor_result_requires_verified_relog_for_success() {
    let mut result = BotRunResult {
        world_auth: true,
        enum_characters: true,
        player_login_verified: true,
        vendor_smoke: true,
        vendor_smoke_passed: Some(true),
        ..BotRunResult::default()
    };

    assert!(!result.success(false, false, false));
    result.vendor_relogin_verified = true;
    assert!(result.success(false, false, false));
}
#[test]
fn equipment_set_result_requires_db_and_fresh_relog_proof() {
    let mut result = BotRunResult {
        world_auth: true,
        enum_characters: true,
        player_login_verified: true,
        equipment_set_smoke: true,
        equipment_set_smoke_passed: Some(true),
        ..BotRunResult::default()
    };

    assert!(!result.success(false, false, false));
    result.equipment_set_db_persisted = true;
    assert!(!result.success(false, false, false));
    result.equipment_set_relogin_verified = true;
    assert!(result.success(false, false, false));
}
#[test]
fn equipment_set_smoke_indices_stay_within_cpp_client_limit() {
    assert!(7 < MAX_EQUIPMENT_SET_INDEX_LIKE_CPP);
    assert!(8 < MAX_EQUIPMENT_SET_INDEX_LIKE_CPP);
}
#[test]
fn equipment_set_fixture_max_query_pins_unsigned_wire_type() {
    assert!(
        SHARED_EQUIPMENT_SET_GUID_MAX_QUERY.starts_with("SELECT CAST(MAX(maxguid) AS UNSIGNED)")
    );
}
#[test]
fn equipment_set_db_verifier_requires_one_row_in_the_expected_table() {
    let options = equipment_set_test_options();
    let equipment_row = expected_equipment_set_db_row(&options, 42);
    let transmog_row = expected_transmog_outfit_db_row(&options, 42);

    assert!(equipment_set_db_rows_match(
        &options,
        42,
        std::slice::from_ref(&equipment_row),
        &[],
    ));
    assert!(!equipment_set_db_rows_match(
        &options,
        42,
        &[equipment_row.clone(), equipment_row.clone()],
        &[],
    ));
    assert!(!equipment_set_db_rows_match(
        &options,
        42,
        &[],
        std::slice::from_ref(&transmog_row),
    ));
    let mut equipment_with_wrong_item = equipment_row;
    equipment_with_wrong_item.items[3] = 99;
    assert!(!equipment_set_db_rows_match(
        &options,
        42,
        std::slice::from_ref(&equipment_with_wrong_item),
        &[],
    ));

    let mut transmog_options = options.clone();
    transmog_options.set_type = 1;
    assert!(equipment_set_db_rows_match(
        &transmog_options,
        42,
        &[],
        std::slice::from_ref(&transmog_row),
    ));
    assert!(!equipment_set_db_rows_match(
        &transmog_options,
        42,
        &[],
        &[transmog_row.clone(), transmog_row.clone()],
    ));
    let mut transmog_with_wrong_appearance = transmog_row.clone();
    transmog_with_wrong_appearance.appearances[4] = 1;
    assert!(!equipment_set_db_rows_match(
        &transmog_options,
        42,
        &[],
        std::slice::from_ref(&transmog_with_wrong_appearance),
    ));
    let mut transmog_with_wrong_enchant = transmog_row;
    transmog_with_wrong_enchant.main_hand_enchant = 7;
    assert!(!equipment_set_db_rows_match(
        &transmog_options,
        42,
        &[],
        std::slice::from_ref(&transmog_with_wrong_enchant),
    ));
}
#[test]
fn equipment_set_save_builder_and_load_parser_share_cpp_shape() {
    let options = equipment_set_test_options();
    let save = build_save_equipment_set_payload(&options).unwrap();
    let mut load = Vec::with_capacity(4 + save.len());
    load.extend_from_slice(&1_u32.to_le_bytes());
    load.extend_from_slice(&save);
    let guid = 0x0102_0304_0506_0708_u64;
    load[8..16].copy_from_slice(&guid.to_le_bytes());

    assert_eq!(
        parse_load_equipment_sets(&load).unwrap(),
        vec![EquipmentSetWire {
            set_type: 0,
            guid,
            set_id: 7,
            ignore_mask: EQUIPMENT_SET_IGNORE_ALL_SLOTS_LIKE_CPP,
            pieces: [[0; 16]; EQUIPMENT_SET_SLOTS_LIKE_CPP],
            appearances: [0; EQUIPMENT_SET_SLOTS_LIKE_CPP],
            enchants: [0; 2],
            secondary_appearances_and_slots: [0; 4],
            assigned_spec_index: -1,
            set_name: "QA Equipment".to_string(),
            set_icon: "INV_Sword_01".to_string(),
        }]
    );

    let mut nonzero_fields = load.clone();
    let first_piece_offset = 4 + 4 + 8 + 4 + 4;
    nonzero_fields[first_piece_offset] = 1;
    let first_appearance_offset = first_piece_offset + 16;
    nonzero_fields[first_appearance_offset..first_appearance_offset + 4]
        .copy_from_slice(&2_i32.to_le_bytes());
    let first_enchant_offset = first_piece_offset + EQUIPMENT_SET_SLOTS_LIKE_CPP * (16 + 4);
    nonzero_fields[first_enchant_offset..first_enchant_offset + 4]
        .copy_from_slice(&3_i32.to_le_bytes());
    let first_secondary_offset = first_enchant_offset + 2 * 4;
    nonzero_fields[first_secondary_offset..first_secondary_offset + 4]
        .copy_from_slice(&4_i32.to_le_bytes());
    let parsed = parse_load_equipment_sets(&nonzero_fields).unwrap();
    assert_eq!(parsed[0].pieces[0][0], 1);
    assert_eq!(parsed[0].appearances[0], 2);
    assert_eq!(parsed[0].enchants[0], 3);
    assert_eq!(parsed[0].secondary_appearances_and_slots[0], 4);

    load.push(0);
    assert!(parse_load_equipment_sets(&load).is_err());
}
#[test]
fn equipment_set_id_parser_requires_exact_cpp_payload() {
    let mut payload = Vec::new();
    payload.extend_from_slice(&42_u64.to_le_bytes());
    payload.extend_from_slice(&1_i32.to_le_bytes());
    payload.extend_from_slice(&72_u32.to_le_bytes());
    assert_eq!(parse_equipment_set_id(&payload).unwrap(), (42, 1, 72));
    payload.push(0);
    assert!(parse_equipment_set_id(&payload).is_err());
}
#[test]
fn equipment_set_id_validator_requires_instance_route() {
    let options = equipment_set_test_options();
    let mut payload = Vec::new();
    payload.extend_from_slice(&42_u64.to_le_bytes());
    payload.extend_from_slice(&options.set_type.to_le_bytes());
    payload.extend_from_slice(&options.set_id.to_le_bytes());

    assert_eq!(
        validate_equipment_set_id_response(false, &payload, &options).unwrap(),
        42
    );
    assert!(validate_equipment_set_id_response(true, &payload, &options).is_err());
}
#[test]
fn vendor_inventory_parser_reads_cpp_plain_item_row_exactly() {
    let (payload, vendor_guid) = vendor_inventory_fixture(0, 0);
    let items = parse_vendor_inventory(&payload, &vendor_guid).unwrap();

    assert_eq!(items.len(), 1);
    let item = &items[0];
    assert_eq!(item.muid, 37);
    assert_eq!(item.item_id, 30183);
    assert_eq!(item.item_type, 1);
    assert_eq!(item.price, 0);
    assert_eq!(item.stack_count, 1);
    assert_eq!(item.extended_cost, 1642);
}
#[test]
fn vendor_inventory_parser_fails_closed_on_unimplemented_item_instance_shapes() {
    let (bonus_payload, vendor_guid) = vendor_inventory_fixture(1, 0);
    assert!(parse_vendor_inventory(&bonus_payload, &vendor_guid).is_err());

    let (modifier_payload, vendor_guid) = vendor_inventory_fixture(0, 1);
    assert!(parse_vendor_inventory(&modifier_payload, &vendor_guid).is_err());

    let (mut trailing_payload, vendor_guid) = vendor_inventory_fixture(0, 0);
    trailing_payload.push(0);
    assert!(parse_vendor_inventory(&trailing_payload, &vendor_guid).is_err());
}
#[test]
fn vendor_buy_payload_uses_cpp_field_order_and_wire_item_instance() {
    let vendor_guid = build_packed_guid(0x1234, 0xF130_0000_485D_0001);
    let payload = build_vendor_buy_item_payload(&vendor_guid, 15, 37, 30183);
    assert!(payload.starts_with(&vendor_guid));

    let mut cursor = vendor_guid.len();
    let (player_guid_len, player_low, player_high) = parse_packed_guid(&payload[cursor..]).unwrap();
    let expected_player = create_player_guid_raw(15, realm_id());
    assert_eq!((player_low, player_high), expected_player);
    cursor += player_guid_len;

    assert_eq!(take_vendor_i32(&payload, &mut cursor).unwrap(), 1);
    assert_eq!(take_vendor_i32(&payload, &mut cursor).unwrap(), 37);
    assert_eq!(take_vendor_i32(&payload, &mut cursor).unwrap(), 255);
    assert_eq!(take_vendor_i32(&payload, &mut cursor).unwrap(), 1);
    assert_eq!(take_vendor_i32(&payload, &mut cursor).unwrap(), 30183);
    assert_eq!(take_vendor_i32(&payload, &mut cursor).unwrap(), 0);
    assert_eq!(take_vendor_i32(&payload, &mut cursor).unwrap(), 0);
    assert_eq!(&payload[cursor..], &[0, 0]);
}
#[test]
fn vendor_buy_succeeded_parser_requires_exact_cpp_fields_and_no_tail() {
    let vendor_guid = build_packed_guid(0x1234, 0xF130_0000_485D_0001);
    let mut payload = vendor_guid.clone();
    payload.extend_from_slice(&59u32.to_le_bytes());
    payload.extend_from_slice(&(-1i32).to_le_bytes());
    payload.extend_from_slice(&1u32.to_le_bytes());
    assert!(parse_vendor_buy_succeeded(&payload, &vendor_guid, 59, 1, -1).is_ok());

    let mut wrong_quantity = payload.clone();
    let quantity_offset = wrong_quantity.len() - 4;
    wrong_quantity[quantity_offset..].copy_from_slice(&2u32.to_le_bytes());
    assert!(parse_vendor_buy_succeeded(&wrong_quantity, &vendor_guid, 59, 1, -1).is_err());

    let mut trailing = payload;
    trailing.push(0);
    assert!(parse_vendor_buy_succeeded(&trailing, &vendor_guid, 59, 1, -1).is_err());
}
#[test]
fn vendor_item_push_validator_requires_exact_cpp_purchase_shape() {
    let realm = 1;
    let character_guid = 15;
    let (player_low, player_high) = create_player_guid_raw(character_guid, realm);
    let item_high = (3u64 << 58) | (u64::from(realm) << 42);
    let mut payload = build_packed_guid(player_low, player_high);
    payload.push(INVENTORY_SLOT_BAG_0);
    payload.extend_from_slice(&i32::from(INVENTORY_SLOT_ITEM_START).to_le_bytes());
    payload.extend_from_slice(&0i32.to_le_bytes()); // QuestLogItemID.
    payload.extend_from_slice(&1i32.to_le_bytes()); // Quantity.
    payload.extend_from_slice(&1i32.to_le_bytes()); // QuantityInInventory.
    payload.extend_from_slice(&0i32.to_le_bytes()); // DungeonEncounterID.
    payload.extend_from_slice(&[0; 16]); // Battle-pet fields.
    payload.extend(build_packed_guid(500, item_high));
    payload.push(0x88); // Pushed + normal display, not created.
    payload.extend_from_slice(&30183i32.to_le_bytes());
    payload.extend_from_slice(&0i32.to_le_bytes()); // Random seed.
    payload.extend_from_slice(&0i32.to_le_bytes()); // Random property.
    payload.push(0); // No ItemBonus.
    payload.push(0); // No modifiers.

    assert!(loot_race::validate_vendor_item_push_result_like_cpp(
        &payload,
        character_guid,
        30183,
        1,
        realm,
    )
    .is_ok());
    payload.push(0);
    assert!(loot_race::validate_vendor_item_push_result_like_cpp(
        &payload,
        character_guid,
        30183,
        1,
        realm,
    )
    .is_err());
}
#[test]
fn set_currency_parser_rejects_negative_wire_values() {
    let mut payload = Vec::new();
    payload.extend_from_slice(&42i32.to_le_bytes());
    payload.extend_from_slice(&15i32.to_le_bytes());
    assert_eq!(parse_set_currency_identity(&payload).unwrap(), (42, 15));

    payload[4..8].copy_from_slice(&(-1i32).to_le_bytes());
    assert!(parse_set_currency_identity(&payload).is_err());
}
#[test]
fn login_verify_budget_is_time_based_not_packet_count() {
    let budget = LoginVerifyBudget::new(Duration::from_secs(1));

    // The former `for _ in 0..30` guard disconnected a second concurrent
    // client after 30 fast CREATE/broadcast packets. Observing packets must
    // not consume the wall-clock budget.
    for _ in 0..64 {
        assert!(budget.next_read_timeout().is_some());
    }

    assert_eq!(
        LoginVerifyBudget::new(Duration::ZERO).next_read_timeout(),
        None
    );
}
#[test]
fn auto_bank_payload_uses_cpp_inv_update_then_source_position() {
    assert_eq!(build_auto_bank_item_payload(35), [0x40, 255, 35, 255, 35]);
    assert_eq!(build_auto_bank_item_payload(59), [0x40, 255, 59, 255, 59]);
}
#[test]
fn inventory_swap_payload_matches_real_cpp_client_layout() {
    assert_eq!(
        build_swap_inv_item_payload(36, 40),
        [0x80, 255, 40, 255, 36, 40, 36]
    );
}
#[test]
fn inventory_swap_invalid_source_payload_matches_cpp_container_order() {
    assert_eq!(
        build_swap_item_invalid_source_payload(40),
        [0x80, 255, 40, 200, 0, 255, 200, 40, 0]
    );
}
#[test]
fn issue20_inventory_fixture_pins_full_enchantment_and_random_property_rows() {
    let enchantments = issue20_item_enchantments_db_string();
    assert_eq!(
        enchantments.split_whitespace().count(),
        ISSUE20_ITEM_ENCHANTMENT_SLOT_COUNT * 3
    );
    assert!(issue20_item_metadata_matches_db_like_cpp(
        &enchantments,
        ISSUE20_ITEM_RANDOM_PROPERTY_ID,
        0,
    ));
    assert!(!issue20_item_metadata_matches_db_like_cpp(
        &enchantments,
        0,
        0,
    ));
    let zero_enchantments = issue20_zero_enchantments_db_string();
    assert!(issue20_item_has_zero_metadata_db_like_cpp(
        &zero_enchantments,
        0,
        0,
    ));
    assert!(!issue20_item_has_zero_metadata_db_like_cpp(
        &enchantments,
        0,
        0,
    ));
}
#[test]
fn issue20_item_create_parser_proves_loaded_metadata_and_exact_block_bytes() {
    use sha2::{Digest, Sha256};

    let payload = issue20_item_create_fixture(
        44_001,
        DEFAULT_INVENTORY_SWAP_ITEM_ENTRY_A,
        15,
        ISSUE20_ITEM_RANDOM_PROPERTY_ID,
        None,
        true,
    );
    let evidence = find_issue20_item_create_in_update_object(
        &payload,
        44_001,
        DEFAULT_INVENTORY_SWAP_ITEM_ENTRY_A,
        15,
        1,
    )
    .expect("valid issue #20 fixture")
    .expect("fixture item CreateObject");
    assert_eq!(evidence.block_sha256, hex::encode(Sha256::digest(&payload)));
    assert_eq!(evidence.enchantments[0], ISSUE20_ITEM_PERMANENT_ENCHANT_ID);
    assert_eq!(
        evidence.enchantments[ISSUE20_ITEM_RANDOM_PROPERTY_SLOT],
        ISSUE20_ITEM_RANDOM_PROPERTY_ENCHANT_ID
    );
    assert_eq!(
        evidence.random_properties_id,
        ISSUE20_ITEM_RANDOM_PROPERTY_ID
    );

    let wrong = issue20_item_create_fixture(
        44_001,
        DEFAULT_INVENTORY_SWAP_ITEM_ENTRY_A,
        15,
        0,
        None,
        true,
    );
    assert!(find_issue20_item_create_in_update_object(
        &wrong,
        44_001,
        DEFAULT_INVENTORY_SWAP_ITEM_ENTRY_A,
        15,
        1,
    )
    .is_err());

    let extra_enchantment = issue20_item_create_fixture(
        44_001,
        DEFAULT_INVENTORY_SWAP_ITEM_ENTRY_A,
        15,
        ISSUE20_ITEM_RANDOM_PROPERTY_ID,
        Some((1, 999)),
        true,
    );
    assert!(find_issue20_item_create_in_update_object(
        &extra_enchantment,
        44_001,
        DEFAULT_INVENTORY_SWAP_ITEM_ENTRY_A,
        15,
        1,
    )
    .is_err());

    let truncated_tail = issue20_item_create_fixture(
        44_001,
        DEFAULT_INVENTORY_SWAP_ITEM_ENTRY_A,
        15,
        ISSUE20_ITEM_RANDOM_PROPERTY_ID,
        None,
        false,
    );
    assert!(find_issue20_item_create_in_update_object(
        &truncated_tail,
        44_001,
        DEFAULT_INVENTORY_SWAP_ITEM_ENTRY_A,
        15,
        1,
    )
    .is_err());
}
#[test]
fn inventory_swap_success_requires_issue20_metadata_and_issue52_validation_proofs() {
    let mut result = BotRunResult {
        inventory_swap_smoke: true,
        inventory_swap_smoke_passed: Some(true),
        world_auth: true,
        enum_characters: true,
        player_login_verified: true,
        ..BotRunResult::default()
    };
    assert!(!result.success(false, false, false));

    result.inventory_swap_item_create_sha256 = Some("ab".repeat(32));
    result.inventory_swap_item_create_relogin_verified = true;
    result.inventory_swap_relogin_after_reverse = true;
    result.inventory_swap_item_metadata_persisted = true;
    assert!(!result.success(false, false, false));

    result.inventory_swap_validation_gate_seen = true;
    assert!(result.success(false, false, false));
}
#[test]
fn stand_state_change_uses_cpp_uint32_wire_layout() {
    assert_eq!(build_stand_state_change(UNIT_STAND_STATE_SIT), [1, 0, 0, 0]);
    assert_eq!(
        build_stand_state_change(UNIT_STAND_STATE_STAND),
        [0, 0, 0, 0]
    );
}
#[test]
fn stand_state_update_requires_exact_cpp_wire_layout() {
    assert!(validate_stand_state_update(&[0, 0, 0, 0, UNIT_STAND_STATE_SIT], 1).is_ok());

    let short = validate_stand_state_update(&[0, 0, 0, 0], UNIT_STAND_STATE_STAND)
        .expect_err("four-byte response must fail");
    assert!(short.to_string().contains("expected 5"));

    let wrong_anim = validate_stand_state_update(
        &[1, 0, 0, 0, UNIT_STAND_STATE_STAND],
        UNIT_STAND_STATE_STAND,
    )
    .expect_err("nonzero AnimKitID must fail");
    assert!(wrong_anim.to_string().contains("AnimKitID"));

    let wrong_state =
        validate_stand_state_update(&[0, 0, 0, 0, UNIT_STAND_STATE_STAND], UNIT_STAND_STATE_SIT)
            .expect_err("unexpected state must fail");
    assert!(wrong_state.to_string().contains("state mismatch"));
}
#[test]
fn stand_state_smoke_only_accepts_states_allowed_by_cpp_handler() {
    for state in [
        UNIT_STAND_STATE_STAND,
        UNIT_STAND_STATE_SIT,
        UNIT_STAND_STATE_SLEEP,
        UNIT_STAND_STATE_KNEEL,
    ] {
        assert!(is_client_stand_state_like_cpp(state));
    }
    assert!(!is_client_stand_state_like_cpp(7));
    assert!(!is_client_stand_state_like_cpp(9));
}
#[test]
fn stand_state_smoke_requires_distinct_realm_and_instance_sockets() {
    assert!(validate_stand_state_socket_topology(true).is_ok());
    let error = validate_stand_state_socket_topology(false).unwrap_err();
    assert!(error
        .to_string()
        .contains("distinct realm/instance sockets"));
}
#[test]
fn stand_state_quiet_drain_ignores_only_periodic_world_traffic() {
    assert!(stand_state_quiet_drain_ambient_opcode(SMSG_ON_MONSTER_MOVE));
    assert!(stand_state_quiet_drain_ambient_opcode(
        SMSG_TIME_SYNC_REQUEST
    ));
    assert!(!stand_state_quiet_drain_ambient_opcode(SMSG_UPDATE_OBJECT));
    assert!(!stand_state_quiet_drain_ambient_opcode(SMSG_AURA_UPDATE));
    assert!(!stand_state_quiet_drain_ambient_opcode(
        SMSG_STAND_STATE_UPDATE
    ));
}
#[test]
fn player_login_guid_uses_configured_realm() {
    let guid = 0x1234;
    let realm_id = 7;
    let far_clip = 500.0f32;
    let mut expected = Vec::new();
    let (low_mask, low_bytes) = pack_u64(guid);
    let high = (2u64 << 58) | ((u64::from(realm_id) & 0x1FFF) << 42);
    let (high_mask, high_bytes) = pack_u64(high);
    expected.push(low_mask);
    expected.push(high_mask);
    expected.extend_from_slice(&low_bytes);
    expected.extend_from_slice(&high_bytes);
    expected.extend_from_slice(&far_clip.to_le_bytes());

    assert_eq!(build_player_login(guid, realm_id, far_clip), expected);
}
#[test]
fn quest_smoke_requires_live_runtime_counter() {
    let error = resolve_quest_runtime_counter(None, 12_345, 15_513).unwrap_err();

    assert!(error.to_string().contains("WOW_BOT_QUEST_RUNTIME_COUNTER"));
}
#[test]
fn legacy_creature_guid_constructor_keeps_counter_map_and_entry_fields() {
    let (low, high) = create_creature_guid_raw(571, 15_513, 77_001);

    assert_eq!(low, 77_001);
    assert_eq!((high >> 58) & 0x3F, 8);
    assert_eq!((high >> 42) & 0x1FFF, 0);
    assert_eq!((high >> 29) & 0x1FFF, 571);
    assert_eq!((high >> 6) & 0x7F_FFFF, 15_513);
}
#[test]
fn rested_xp_move_heartbeat_uses_live_position_and_cpp_movement_layout() {
    let (player_low, player_high) = create_player_guid_raw(14, 1);
    let target = DiscoveredCreatureGuid {
        low: 77_001,
        high: create_creature_guid_raw(530, 15_274, 77_001).1,
        x: 10_188.0,
        y: -6_347.5,
        z: 30.5,
    };
    let player_x = target.x + 1.0;
    let player_y = target.y;
    let player_z = target.z;
    let orientation = (target.y - player_y).atan2(target.x - player_x);
    let payload = build_move_heartbeat_payload(
        player_low,
        player_high,
        player_x,
        player_y,
        player_z,
        orientation,
    );

    let (guid_len, low, high) = parse_packed_guid(&payload).expect("packed player GUID");
    assert_eq!((low, high), (player_low, player_high));
    let movement = &payload[guid_len..];
    assert_eq!(movement.len(), 49);
    assert_eq!(&movement[..16], &[0; 16]);
    assert_eq!(read_f32_at(movement, 16), Some(player_x));
    assert_eq!(read_f32_at(movement, 20), Some(player_y));
    assert_eq!(read_f32_at(movement, 24), Some(player_z));
    assert_eq!(read_f32_at(movement, 28), Some(orientation));
    assert_eq!(&movement[32..48], &[0; 16]);
    assert_eq!(movement[48], 0);

    let distance = ((target.x - player_x).powi(2)
        + (target.y - player_y).powi(2)
        + (target.z - player_z).powi(2))
    .sqrt();
    assert!(distance < NOMINAL_MELEE_RANGE_LIKE_CPP);
    assert!((orientation - std::f32::consts::PI).abs() < f32::EPSILON);
}
#[test]
fn rested_xp_active_mover_ack_matches_cpp_wire_layout() {
    assert_eq!(CMSG_MOVE_INIT_ACTIVE_MOVER_COMPLETE, 0x3A46);
    assert_eq!(
        build_move_init_active_mover_complete_payload(0x1234_5678),
        [0x78, 0x56, 0x34, 0x12]
    );
}
#[test]
fn homebind_smoke_discovers_loaded_creature_guid_from_update_object() {
    let (low, high) = create_creature_guid_raw(1, 12_196, 733);
    let mut payload = vec![0, 0, 0, 0, 1, 0, 0, 0];
    payload.push(1);
    payload.extend(build_packed_guid(low, high));
    payload.push(5);

    assert_eq!(
        find_creature_guid_in_update_object(&payload, 1, 12_196),
        Some((low, high))
    );
    assert_eq!(
        find_creature_guid_in_update_object(&payload, 1, 12_197),
        None
    );
}
#[test]
fn homebind_smoke_decodes_complete_bind_point_update() {
    let mut payload = Vec::new();
    payload.extend_from_slice(&1.25f32.to_le_bytes());
    payload.extend_from_slice(&(-2.5f32).to_le_bytes());
    payload.extend_from_slice(&3.75f32.to_le_bytes());
    payload.extend_from_slice(&571i32.to_le_bytes());
    payload.extend_from_slice(&4395i32.to_le_bytes());

    assert_eq!(
        parse_bind_point_update(&payload, 4.5),
        Some(HomebindRowSnapshot {
            map_id: 571,
            zone_id: 4395,
            x: 1.25,
            y: -2.5,
            z: 3.75,
            orientation: 4.5,
        })
    );
    assert_eq!(parse_bind_point_update(&payload[..19], 4.5), None);
}
#[test]
fn homebind_smoke_validates_exact_player_bound_payload() {
    let (low, high) = create_creature_guid_raw(1, 12_196, 733);
    let mut payload = build_packed_guid(low, high);
    payload.extend_from_slice(&3430u32.to_le_bytes());

    assert!(player_bound_matches(&payload, low, high, 3430));
    assert!(!player_bound_matches(&payload, low, high, 3431));
    assert!(!player_bound_matches(&payload, low + 1, high, 3430));

    payload.push(0);
    assert!(!player_bound_matches(&payload, low, high, 3430));
}
#[test]
fn homebind_smoke_requires_exact_bind_spell_go() {
    let (caster_low, caster_high) = create_creature_guid_raw(1, 12_196, 733);
    let player_low = 99;
    let player_high = (2u64 << 58) | (1u64 << 42);
    let mut payload = bind_spell_go_fixture(caster_low, caster_high, player_low, player_high);

    assert!(spell_go_matches_bind(
        &payload,
        caster_low,
        caster_high,
        player_low,
        player_high,
    ));
    assert!(!spell_go_matches_bind(
        &payload,
        caster_low + 1,
        caster_high,
        player_low,
        player_high,
    ));

    let spell_offset =
        2 * build_packed_guid(caster_low, caster_high).len() + 2 * build_packed_guid(1, 0).len();
    payload[spell_offset..spell_offset + 4].copy_from_slice(&1u32.to_le_bytes());
    assert!(!spell_go_matches_bind(
        &payload,
        caster_low,
        caster_high,
        player_low,
        player_high,
    ));
}
