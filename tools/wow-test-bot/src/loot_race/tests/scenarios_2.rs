//! Loot-race scenarios, part 2.
//!
//! Split out of the inline test module under #634; assertions unchanged.

use super::*;

#[test]
fn atomic_item_wire_rejects_foreign_push_extra_removal_and_late_duplicate() {
    let (mut foreign_push, removal) = valid_atomic_item_outcome(true);
    foreign_push[1].item_pushes[0].item_entry += 1;
    assert!(validate_atomic_item_wire_outcome_like_cpp(
        &foreign_push,
        ITEM_TEST_CHARACTERS,
        ITEM_TEST_ENTRY,
        1,
        removal,
        ITEM_TEST_REALM,
    )
    .is_err());

    let (mut extra_removal, removal) = valid_atomic_item_outcome(false);
    extra_removal[0].loot_removed.push(removal);
    assert!(validate_atomic_item_wire_outcome_like_cpp(
        &extra_removal,
        ITEM_TEST_CHARACTERS,
        ITEM_TEST_ENTRY,
        1,
        removal,
        ITEM_TEST_REALM,
    )
    .is_err());

    let (mut late_duplicate, removal) = valid_atomic_item_outcome(false);
    validate_atomic_item_wire_outcome_like_cpp(
        &late_duplicate,
        ITEM_TEST_CHARACTERS,
        ITEM_TEST_ENTRY,
        1,
        removal,
        ITEM_TEST_REALM,
    )
    .unwrap();
    late_duplicate[0].merge(WireEvidence {
        item_pushes: vec![valid_atomic_item_push()],
        ..Default::default()
    });
    assert!(validate_atomic_item_wire_outcome_like_cpp(
        &late_duplicate,
        ITEM_TEST_CHARACTERS,
        ITEM_TEST_ENTRY,
        1,
        removal,
        ITEM_TEST_REALM,
    )
    .is_err());
}
#[test]
fn atomic_item_wire_rejects_foreign_or_additional_inventory_failure() {
    let (mut foreign, removal) = valid_atomic_item_outcome(false);
    foreign[1].inventory_failures[0].result = 49;
    assert!(validate_atomic_item_wire_outcome_like_cpp(
        &foreign,
        ITEM_TEST_CHARACTERS,
        ITEM_TEST_ENTRY,
        1,
        removal,
        ITEM_TEST_REALM,
    )
    .is_err());

    let (mut additional, removal) = valid_atomic_item_outcome(false);
    additional[0]
        .inventory_failures
        .push(additional[1].inventory_failures[0]);
    assert!(validate_atomic_item_wire_outcome_like_cpp(
        &additional,
        ITEM_TEST_CHARACTERS,
        ITEM_TEST_ENTRY,
        1,
        removal,
        ITEM_TEST_REALM,
    )
    .is_err());
}
#[test]
fn persisted_item_row_binds_wire_guid_owner_entry_count_and_slot() {
    let expected = ExpectedPersistedItemGrant {
        owner_guid: ITEM_TEST_CHARACTERS[0],
        push: valid_atomic_item_push(),
    };
    let valid = PersistedItemGrantRow {
        item_guid: expected.push.item_guid_low,
        owner_guid: expected.owner_guid,
        item_entry: expected.push.item_entry,
        count: 1,
        inventory_owner: Some(expected.owner_guid),
        bag_guid: Some(0),
        slot: Some(INVENTORY_SLOT_ITEM_START),
        bag_slot: None,
    };
    validate_persisted_item_grant_like_cpp(expected, valid).unwrap();

    for malformed in [
        PersistedItemGrantRow {
            item_guid: valid.item_guid + 1,
            ..valid
        },
        PersistedItemGrantRow {
            owner_guid: valid.owner_guid + 1,
            ..valid
        },
        PersistedItemGrantRow {
            item_entry: valid.item_entry + 1,
            ..valid
        },
        PersistedItemGrantRow { count: 2, ..valid },
        PersistedItemGrantRow {
            slot: Some(INVENTORY_SLOT_ITEM_START + 1),
            ..valid
        },
    ] {
        assert!(validate_persisted_item_grant_like_cpp(expected, malformed).is_err());
    }
}
#[test]
fn tattered_chest_template_data_pins_shared_group_rules_and_loot_id() {
    assert_eq!(RACE_GAMEOBJECT_TEMPLATE_DATA.len(), 35);
    assert_eq!(RACE_GAMEOBJECT_TEMPLATE_DATA[0], 57);
    assert_eq!(RACE_GAMEOBJECT_TEMPLATE_DATA[1], 2_278);
    assert_eq!(RACE_GAMEOBJECT_TEMPLATE_DATA[3], 1);
    assert_eq!(RACE_GAMEOBJECT_TEMPLATE_DATA[10], 1);
    assert_eq!(RACE_GAMEOBJECT_TEMPLATE_DATA[12], 1);
    assert_eq!(RACE_GAMEOBJECT_TEMPLATE_DATA[15], 1);
    assert!(RACE_GAMEOBJECT_TEMPLATE_DATA
        .iter()
        .enumerate()
        .all(|(index, value)| matches!(index, 0 | 1 | 3 | 10 | 12 | 15) || *value == 0));
}
#[test]
fn loot_object_guid_requires_exact_cpp_world_object_structure() {
    let realm_id = 7u32;
    let map_id = 571u16;
    let counter = 41u64;
    let high =
        (HIGH_GUID_LOOT_OBJECT << 58) | (u64::from(realm_id) << 42) | (u64::from(map_id) << 29);

    validate_loot_object_guid_like_cpp(counter, high, map_id, realm_id).unwrap();

    let malformed = [
        (counter, high ^ (1 << 58)),
        (counter, high ^ (1 << 42)),
        (counter, high ^ (1 << 29)),
        (counter, high | (1 << 6)),
        (counter, high | 1),
        (counter | (1 << 40), high),
        (0, high),
    ];
    for (low, high) in malformed {
        assert!(
            validate_loot_object_guid_like_cpp(low, high, map_id, realm_id).is_err(),
            "malformed LootObject unexpectedly passed: low={low:#x}, high={high:#x}"
        );
    }
}
#[test]
fn serialized_money_race_requires_one_positive_and_one_zero_fanout_like_cpp() {
    let source = (0x11, 0x22);
    let positive = MoneyNotify {
        money: 10,
        money_mod: 0,
        sole_looter: true,
    };
    let zero = MoneyNotify {
        money: 0,
        money_mod: 0,
        sole_looter: true,
    };
    let evidence = [
        WireEvidence {
            money_notifies: vec![positive],
            coin_removed: vec![source, source],
            ..Default::default()
        },
        WireEvidence {
            money_notifies: vec![zero],
            coin_removed: vec![source, source],
            ..Default::default()
        },
    ];

    assert_eq!(
        validate_serialized_gameobject_money_wire_outcome_like_cpp(&evidence, source, 10).unwrap(),
        0
    );

    let reversed = [evidence[1].clone(), evidence[0].clone()];
    assert_eq!(
        validate_serialized_gameobject_money_wire_outcome_like_cpp(&reversed, source, 10).unwrap(),
        1
    );
}
#[test]
fn serialized_money_race_rejects_duplicate_positive_or_missing_coin_fanout() {
    let source = (0x11, 0x22);
    let positive = MoneyNotify {
        money: 10,
        money_mod: 0,
        sole_looter: true,
    };
    let zero = MoneyNotify {
        money: 0,
        money_mod: 0,
        sole_looter: true,
    };
    let valid = WireEvidence {
        money_notifies: vec![zero],
        coin_removed: vec![source, source],
        ..Default::default()
    };

    let duplicate_positive = WireEvidence {
        money_notifies: vec![positive],
        coin_removed: vec![source, source],
        ..Default::default()
    };
    assert!(validate_serialized_gameobject_money_wire_outcome_like_cpp(
        &[duplicate_positive.clone(), duplicate_positive],
        source,
        10,
    )
    .is_err());

    let missing_coin = WireEvidence {
        money_notifies: vec![positive],
        coin_removed: vec![source],
        ..Default::default()
    };
    assert!(validate_serialized_gameobject_money_wire_outcome_like_cpp(
        &[valid.clone(), missing_coin],
        source,
        10,
    )
    .is_err());

    let wrong_sole_looter = WireEvidence {
        money_notifies: vec![MoneyNotify {
            money: 10,
            money_mod: 0,
            sole_looter: false,
        }],
        coin_removed: vec![source, source],
        ..Default::default()
    };
    assert!(validate_serialized_gameobject_money_wire_outcome_like_cpp(
        &[valid, wrong_sole_looter],
        source,
        10,
    )
    .is_err());
}
#[test]
fn loot_gone_failure_matches_cpp_wire_and_rejects_a_tail() {
    let mut payload = 50i32.to_le_bytes().to_vec();
    payload.extend_from_slice(&build_packed_guid(0, 0));
    payload.extend_from_slice(&build_packed_guid(0, 0));
    payload.push(0);
    assert_eq!(
        parse_inventory_failure(&payload).unwrap(),
        InventoryFailure {
            result: 50,
            item_0_low: 0,
            item_0_high: 0,
            item_1_low: 0,
            item_1_high: 0,
            container_b_slot: 0,
        }
    );
    payload.push(0);
    assert!(parse_inventory_failure(&payload).is_err());
}
#[test]
fn money_notify_reads_two_u64s_and_msb_sole_looter_bit() {
    let mut payload = 3u64.to_le_bytes().to_vec();
    payload.extend_from_slice(&2u64.to_le_bytes());
    payload.push(0x80);
    let mut evidence = WireEvidence::default();
    record_evidence(SMSG_LOOT_MONEY_NOTIFY, &payload, (0, 0), &mut evidence).unwrap();
    assert_eq!(
        evidence.money_notifies,
        vec![MoneyNotify {
            money: 3,
            money_mod: 2,
            sole_looter: true,
        }]
    );
    payload.push(0);
    assert!(record_evidence(SMSG_LOOT_MONEY_NOTIFY, &payload, (0, 0), &mut evidence).is_err());
}
#[test]
fn single_item_capture_requires_one_owner_push_one_removal_and_no_money() {
    let window = LootWindow {
        owner_low: 1,
        owner_high: 2,
        loot_low: 0x11,
        loot_high: 0x22,
        coins: 7,
        item_entry: 56_147,
        quantity: 1,
        loot_list_id: 3,
        loot_method: 0,
    };
    let expected_player = (0x33, 0x44);
    let evidence = WireEvidence {
        item_pushes: vec![ItemPush {
            player_low: expected_player.0,
            player_high: expected_player.1,
            item_entry: window.item_entry,
            slot: INVENTORY_SLOT_BAG_0,
            slot_in_bag: i32::from(LOOT_ITEM_CAPTURE_KEYRING_SLOT),
            quantity: 1,
            quantity_in_inventory: 1,
            ..Default::default()
        }],
        loot_removed: vec![LootRemovedEvidence {
            owner_low: window.owner_low,
            owner_high: window.owner_high,
            loot_low: window.loot_low,
            loot_high: window.loot_high,
            loot_list_id: window.loot_list_id,
        }],
        ..Default::default()
    };

    validate_single_item_capture_evidence(&evidence, expected_player, window.item_entry, &window)
        .unwrap();

    let mut with_money = evidence.clone();
    with_money.money_notifies.push(MoneyNotify {
        money: 7,
        money_mod: 0,
        sole_looter: true,
    });
    assert!(validate_single_item_capture_evidence(
        &with_money,
        expected_player,
        window.item_entry,
        &window,
    )
    .is_err());

    let mut displaced_key = evidence.clone();
    displaced_key.item_pushes[0].slot_in_bag = i32::from(INVENTORY_SLOT_ITEM_START);
    assert!(validate_single_item_capture_evidence(
        &displaced_key,
        expected_player,
        window.item_entry,
        &window,
    )
    .is_err());

    let mut wrong_route_owner = evidence;
    wrong_route_owner.item_pushes[0].player_low ^= 1;
    assert!(validate_single_item_capture_evidence(
        &wrong_route_owner,
        expected_player,
        window.item_entry,
        &window,
    )
    .is_err());
}
#[test]
fn capture_collector_fails_closed_on_duplicate_or_failure_before_fence() {
    let push = ItemPush {
        player_low: 1,
        player_high: 2,
        item_entry: 56_147,
        ..Default::default()
    };
    validate_single_item_capture_candidate(&WireEvidence {
        item_pushes: vec![push],
        ..Default::default()
    })
    .unwrap();

    assert!(validate_single_item_capture_candidate(&WireEvidence {
        item_pushes: vec![push, push],
        ..Default::default()
    })
    .is_err());
    assert!(validate_single_item_capture_candidate(&WireEvidence {
        inventory_failures: vec![InventoryFailure {
            result: 50,
            item_0_low: 0,
            item_0_high: 0,
            item_1_low: 0,
            item_1_high: 0,
            container_b_slot: 0,
        }],
        ..Default::default()
    })
    .is_err());
}
#[test]
fn capture_exclusivity_requires_zero_globally_online_characters() {
    validate_online_character_count(0, "test preflight").unwrap();
    let error = validate_online_character_count(1, "test preflight")
        .expect_err("one unrelated online character must fail closed");
    assert!(error.to_string().contains("exclusive world access"));
    assert!(error.to_string().contains("1 character(s)"));
}
#[test]
fn progress_restore_table_scope_has_no_duplicates_or_global_tables() {
    let unique = CHARACTER_PROGRESS_TABLES
        .iter()
        .copied()
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(unique.len(), CHARACTER_PROGRESS_TABLES.len());
    assert!(unique.contains("character_achievement"));
    assert!(unique.contains("character_achievement_progress"));
    assert!(unique.contains("character_queststatus_objectives_criteria_progress"));
    assert!(unique.contains("character_reputation"));
    assert!(!unique.iter().any(|table| table.starts_with("guild_")));
}
