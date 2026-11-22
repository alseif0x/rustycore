use super::*;

const TEST_REALM_ID: u32 = 1;

fn chest_guid() -> Guid {
    let high = (crate::loot_race::HIGH_GUID_GAMEOBJECT << 58)
        | (u64::from(crate::loot_race::RACE_GAMEOBJECT_MAP_ID) << 29)
        | (u64::from(crate::loot_race::DEFAULT_CREATURE_ENTRY) << 6);
    Guid { low: 40, high }
}

fn loot_object_guid() -> Guid {
    Guid {
        low: 41,
        high: (crate::loot_race::HIGH_GUID_LOOT_OBJECT << 58)
            | (u64::from(TEST_REALM_ID) << 42)
            | (u64::from(crate::loot_race::RACE_GAMEOBJECT_MAP_ID) << 29),
    }
}

fn game_object_selection(guid: Guid) -> GameObjectSelectionFact {
    GameObjectSelectionFact {
        action_index: 0,
        spawn_guid: crate::loot_race::DEFAULT_CREATURE_SPAWN_GUID,
        entry: crate::loot_race::DEFAULT_CREATURE_ENTRY,
        map_id: crate::loot_race::RACE_GAMEOBJECT_MAP_ID,
        guid,
        source_event_sequence: 1,
        source_connection: Connection::Instance,
        source_connection_sequence: 1,
        source_event_opcode: RAW_SMSG_UPDATE_OBJECT,
        source_event_sha256: "test-object-update".to_string(),
    }
}

fn loot_response(
    owner: Guid,
    loot: Guid,
    coins: u32,
    loot_method: u8,
    failure_reason: u8,
    acquired: bool,
    item_entry: u32,
    quantity: u32,
) -> Vec<u8> {
    let mut payload = owner.packed();
    payload.extend(loot.packed());
    payload.extend([failure_reason, 0, loot_method, 0]);
    payload.extend(coins.to_le_bytes());
    payload.extend(1u32.to_le_bytes()); // item count
    payload.extend(0u32.to_le_bytes()); // currency count
    payload.push(if acquired { 0x80 } else { 0 });
    payload.push(0); // item bits
    payload.extend((item_entry as i32).to_le_bytes());
    payload.extend(0i32.to_le_bytes()); // random seed
    payload.extend(0i32.to_le_bytes()); // random property
    payload.push(0); // no bonus data
    payload.push(0); // no item modifications
    payload.extend(quantity.to_le_bytes());
    payload.push(0); // loot item type
    payload.push(0); // loot list ID
    payload
}

fn loot_money_notify(money: u64, money_mod: u64, sole_looter: bool) -> Vec<u8> {
    let mut payload = money.to_le_bytes().to_vec();
    payload.extend(money_mod.to_le_bytes());
    payload.push(if sole_looter { 0x80 } else { 0 });
    payload
}

fn pre_restart_plan() -> Plan {
    Plan {
        character_guid: None,
        observe_only: false,
        timeout_ms: 5_000,
        actions: vec![
            Action::OpenLootRecoveryChest {},
            Action::ClaimLootRecoveryMoney {
                expected_coins: crate::loot_race::RACE_GAMEOBJECT_MONEY,
            },
            Action::WaitForServerDisconnect { timeout_ms: 30_000 },
        ],
        expect: vec![
            ExpectedRule::LootResponse {
                coins: crate::loot_race::RACE_GAMEOBJECT_MONEY,
                item_entry: crate::loot_race::DEFAULT_ITEM_ENTRY,
            },
            ExpectedRule::LootCoinRemoved {},
            ExpectedRule::LootMoneyNotify {
                money: u64::from(crate::loot_race::RACE_GAMEOBJECT_MONEY),
                money_mod: 0,
            },
        ],
        finish_policy: FinishPolicy::Disconnect,
    }
}

fn post_restart_plan() -> Plan {
    Plan {
        character_guid: None,
        observe_only: false,
        timeout_ms: 5_000,
        actions: vec![
            Action::OpenLootRecoveryChest {},
            Action::ClaimLootRecoveryMoney { expected_coins: 0 },
            Action::ReleaseLootRecoveryChest {},
        ],
        expect: vec![
            ExpectedRule::LootResponse {
                coins: 0,
                item_entry: crate::loot_race::DEFAULT_ITEM_ENTRY,
            },
            ExpectedRule::LootCoinRemoved {},
            ExpectedRule::LootMoneyNotify {
                money: 0,
                money_mod: 0,
            },
            ExpectedRule::LootRelease {},
        ],
        finish_policy: FinishPolicy::Disconnect,
    }
}

#[test]
fn chest_response_and_money_validation_accept_the_two_expected_phases() {
    let selected = chest_guid();
    for (coins, method) in [
        (crate::loot_race::RACE_GAMEOBJECT_MONEY, 0),
        (0, crate::loot_race::PERSONAL_LOOT_METHOD_LIKE_CPP),
    ] {
        let response = loot_response(
            selected,
            loot_object_guid(),
            coins,
            method,
            CXX_DEFAULT_LOOT_RESPONSE_FAILURE_REASON,
            true,
            crate::loot_race::DEFAULT_ITEM_ENTRY,
            1,
        );
        let loot_guid =
            validate_recovery_chest_loot_response(&response, selected, coins, TEST_REALM_ID)
                .unwrap();
        assert_eq!(loot_guid, loot_object_guid());
        assert_ne!(
            loot_guid, selected,
            "LootObject differs from gameobject owner"
        );

        let coin_removed = loot_guid.packed();
        let money_notify = loot_money_notify(u64::from(coins), 0, true);
        validate_recovery_chest_money_outcome(&coin_removed, &money_notify, loot_guid, coins)
            .unwrap();
    }

    let (opcode, body) = recovery_chest_money_claim_packet();
    assert_eq!(opcode, crate::loot_race::CMSG_LOOT_MONEY);
    assert_eq!(body, [0]);
    assert_ne!(opcode, crate::loot_race::CMSG_LOOT_ITEM);
}

#[test]
fn chest_response_validation_rejects_wrong_owner_shape_coins_and_item_contract() {
    let selected = chest_guid();
    let valid = |owner, loot, coins, method, failure, acquired, item, quantity| {
        loot_response(
            owner, loot, coins, method, failure, acquired, item, quantity,
        )
    };
    let invalid_responses = [
        valid(
            Guid {
                low: selected.low + 1,
                ..selected
            },
            loot_object_guid(),
            10,
            0,
            CXX_DEFAULT_LOOT_RESPONSE_FAILURE_REASON,
            true,
            crate::loot_race::DEFAULT_ITEM_ENTRY,
            1,
        ),
        valid(
            selected,
            Guid {
                high: loot_object_guid().high ^ (1 << 58),
                ..loot_object_guid()
            },
            10,
            0,
            CXX_DEFAULT_LOOT_RESPONSE_FAILURE_REASON,
            true,
            crate::loot_race::DEFAULT_ITEM_ENTRY,
            1,
        ),
        valid(
            selected,
            loot_object_guid(),
            9,
            0,
            CXX_DEFAULT_LOOT_RESPONSE_FAILURE_REASON,
            true,
            crate::loot_race::DEFAULT_ITEM_ENTRY,
            1,
        ),
        valid(
            selected,
            loot_object_guid(),
            10,
            3,
            CXX_DEFAULT_LOOT_RESPONSE_FAILURE_REASON,
            true,
            crate::loot_race::DEFAULT_ITEM_ENTRY,
            1,
        ),
        valid(
            selected,
            loot_object_guid(),
            10,
            0,
            1,
            true,
            crate::loot_race::DEFAULT_ITEM_ENTRY,
            1,
        ),
        valid(
            selected,
            loot_object_guid(),
            10,
            0,
            CXX_DEFAULT_LOOT_RESPONSE_FAILURE_REASON,
            false,
            crate::loot_race::DEFAULT_ITEM_ENTRY,
            1,
        ),
        valid(
            selected,
            loot_object_guid(),
            10,
            0,
            CXX_DEFAULT_LOOT_RESPONSE_FAILURE_REASON,
            true,
            crate::loot_race::DEFAULT_ITEM_ENTRY + 1,
            1,
        ),
        valid(
            selected,
            loot_object_guid(),
            10,
            0,
            CXX_DEFAULT_LOOT_RESPONSE_FAILURE_REASON,
            true,
            crate::loot_race::DEFAULT_ITEM_ENTRY,
            2,
        ),
    ];
    for response in invalid_responses {
        assert!(
            validate_recovery_chest_loot_response(&response, selected, 10, TEST_REALM_ID).is_err()
        );
    }
}

#[test]
fn loot_response_expectation_matches_cxx_default_17_and_rejects_zero() {
    let selected = chest_guid();
    for (failure_reason, expected_match) in
        [(CXX_DEFAULT_LOOT_RESPONSE_FAILURE_REASON, true), (0, false)]
    {
        let payload = loot_response(
            selected,
            loot_object_guid(),
            crate::loot_race::RACE_GAMEOBJECT_MONEY,
            0,
            failure_reason,
            true,
            crate::loot_race::DEFAULT_ITEM_ENTRY,
            1,
        );
        let event = decode_event(
            0,
            Connection::Instance,
            crate::loot_race::SMSG_LOOT_RESPONSE,
            &payload,
        );
        assert!(
            event.error.is_none(),
            "fixture payload must decode: {event:?}"
        );
        let mut evidence = Evidence {
            events: vec![event],
            game_object_selections: vec![game_object_selection(selected)],
            expected_rules: vec![expected(ExpectedRule::LootResponse {
                coins: crate::loot_race::RACE_GAMEOBJECT_MONEY,
                item_entry: crate::loot_race::DEFAULT_ITEM_ENTRY,
            })],
            ..Evidence::default()
        };

        report::evaluate(&mut evidence);

        assert_eq!(evidence.expected_rules[0].matched, expected_match);
    }
}

#[test]
fn report_matcher_binds_coin_removed_and_release_to_distinct_response_guids() {
    let chest = chest_guid();
    let loot_object = loot_object_guid();
    let player = Guid {
        low: 42,
        high: (2u64 << 58) | (u64::from(TEST_REALM_ID) << 42),
    };
    let response = loot_response(
        chest,
        loot_object,
        0,
        0,
        CXX_DEFAULT_LOOT_RESPONSE_FAILURE_REASON,
        true,
        crate::loot_race::DEFAULT_ITEM_ENTRY,
        1,
    );
    let mut release = chest.packed();
    release.extend(player.packed());
    let events = vec![
        decode_event(
            0,
            Connection::Instance,
            crate::loot_race::SMSG_LOOT_RESPONSE,
            &response,
        ),
        decode_event(
            1,
            Connection::Instance,
            crate::loot_race::SMSG_COIN_REMOVED,
            &loot_object.packed(),
        ),
        decode_event(
            2,
            Connection::Instance,
            crate::loot_race::SMSG_LOOT_MONEY_NOTIFY,
            &loot_money_notify(0, 0, true),
        ),
        decode_event(3, Connection::Instance, SMSG_LOOT_RELEASE, &release),
    ];
    let mut evidence = Evidence {
        events,
        game_object_selections: vec![game_object_selection(chest)],
        expected_rules: vec![
            expected(ExpectedRule::LootResponse {
                coins: 0,
                item_entry: crate::loot_race::DEFAULT_ITEM_ENTRY,
            }),
            expected(ExpectedRule::LootCoinRemoved {}),
            expected(ExpectedRule::LootMoneyNotify {
                money: 0,
                money_mod: 0,
            }),
            expected(ExpectedRule::LootRelease {}),
        ],
        ..Evidence::default()
    };

    report::evaluate(&mut evidence);

    assert!(
        evidence.passed,
        "zero-money response still reports both money packets"
    );
    assert!(evidence.expected_rules.iter().all(|fact| fact.matched));

    // CoinRemoved carries LootResponse.LootObj, while LootReleaseResponse's
    // first GUID is LootResponse.Owner (the gameobject owner GUID).
    evidence.events[1] = decode_event(
        1,
        Connection::Instance,
        crate::loot_race::SMSG_COIN_REMOVED,
        &chest.packed(),
    );
    report::evaluate(&mut evidence);
    assert!(!evidence.expected_rules[1].matched);

    evidence.events[1] = decode_event(
        1,
        Connection::Instance,
        crate::loot_race::SMSG_COIN_REMOVED,
        &loot_object.packed(),
    );
    let mut wrong_release = loot_object.packed();
    wrong_release.extend(player.packed());
    evidence.events[3] = decode_event(3, Connection::Instance, SMSG_LOOT_RELEASE, &wrong_release);
    report::evaluate(&mut evidence);
    assert!(!evidence.expected_rules[3].matched);
}

#[test]
fn chest_money_validation_binds_chest_amount_modifier_and_sole_looter() {
    let selected = chest_guid();
    let loot_object = loot_object_guid();
    let valid_coin_removed = loot_object.packed();
    for (money, money_mod, sole_looter) in [(9, 0, true), (10, 1, true), (10, 0, false)] {
        let notify = loot_money_notify(money, money_mod, sole_looter);
        assert!(validate_recovery_chest_money_outcome(
            &valid_coin_removed,
            &notify,
            loot_object,
            10
        )
        .is_err());
    }
    let wrong_chest = selected.packed();
    let notify = loot_money_notify(10, 0, true);
    assert!(validate_recovery_chest_money_outcome(&wrong_chest, &notify, loot_object, 10).is_err());
}

#[test]
fn pre_and_post_restart_chest_plans_have_distinct_fenced_sequences() {
    let pre = pre_restart_plan();
    let post = post_restart_plan();
    plan::validate_plan(&pre).expect("10-coin stage must hold for restart disconnect");
    plan::validate_plan(&post).expect("zero-coin stage must release after verification");

    let mut invalid = pre.clone();
    invalid.actions[2] = Action::ReleaseLootRecoveryChest {};
    assert!(plan::validate_plan(&invalid).is_err());

    let mut invalid = post.clone();
    invalid.actions[2] = Action::WaitForServerDisconnect { timeout_ms: 30_000 };
    assert!(plan::validate_plan(&invalid).is_err());

    let mut invalid = pre;
    invalid.expect[0] = ExpectedRule::LootResponse {
        coins: 0,
        item_entry: crate::loot_race::DEFAULT_ITEM_ENTRY,
    };
    assert!(plan::validate_plan(&invalid).is_err());

    let mut invalid = post;
    invalid.expect.swap(1, 2);
    assert!(plan::validate_plan(&invalid).is_err());
}
