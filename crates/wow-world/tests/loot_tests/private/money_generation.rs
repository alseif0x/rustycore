//! Money application scenarios backed by the original persistence operations.

use super::money_support::*;

#[tokio::test]
async fn represented_gameobject_personal_encounter_open_reads_player_money_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(4);
    let player_guid = ObjectGuid::create_player(1, 42);
    let gameobject_guid = test_gameobject_guid(91_012);
    let loot_object = represented_loot_object_guid_like_cpp(gameobject_guid);
    session.set_player_guid(Some(player_guid));
    prepare_personal_money_fixture(&mut session, true);
    insert_client_visible_guid_for_test(&mut session, gameobject_guid);
    set_loot_for_test(&mut session, 
        gameobject_guid,
        CreatureLoot {
            loot_guid: loot_object,
            coins: 999,
            unlooted_count: 0,
            loot_type: LOOT_TYPE_CHEST_LIKE_CPP,
            dungeon_encounter_id: 733,
            loot_method: 0,
            loot_master: ObjectGuid::EMPTY,
            round_robin_player: ObjectGuid::EMPTY,
            player_ffa_items: Vec::new(),
            players_looting: Vec::new(),
            allowed_looters: vec![player_guid],
            items: Vec::new(),
            looted_by_player: false,
        },
    );
    set_personal_loot_for_loot_test(&mut session, gameobject_guid, player_guid, 123);
    let source = GameObjectLootSource {
        loot_id: 0,
        use_group_loot_rules: false,
        dungeon_encounter_id: 733,
        personal_loot_id: 10_001,
        push_loot_id: 0,
        triggered_event_id: 0,
        linked_trap_entry: 0,
        ..Default::default()
    };

    open_personal_money_loot_for_test(&mut session, gameobject_guid, source).await;

    let mut response =
        recv_packet_with_opcode(&send_rx, wow_constants::ServerOpcodes::LootResponse);
    assert_eq!(response.read_packed_guid().unwrap(), gameobject_guid);
    assert_eq!(response.read_packed_guid().unwrap(), loot_object);
    // failure_reason: C++ LootResponse::FailureReason defaults to 17 (LOOT_ERROR_NO_LOOT,
    // LootPackets.h:72 "Most common value") and is left unset on a successful loot — the
    // client ignores it once the window opens. (Previously, wrongly asserted as 0.)
    assert_eq!(response.read_uint8().unwrap(), 17);
    assert_eq!(response.read_uint8().unwrap(), LOOT_TYPE_CHEST_LIKE_CPP);
    assert_eq!(response.read_uint8().unwrap(), 0);
    assert_eq!(response.read_uint8().unwrap(), 2);
    assert_eq!(response.read_uint32().unwrap(), 123);
}

#[tokio::test]
async fn represented_gameobject_personal_encounter_money_pickup_consumes_only_player_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(4);
    let player_guid = ObjectGuid::create_player(1, 42);
    let other_tapper = ObjectGuid::create_player(1, 77);
    let gameobject_guid = test_gameobject_guid(91_013);
    let loot_object = represented_loot_object_guid_like_cpp(gameobject_guid);
    session.set_player_guid(Some(player_guid));
    prepare_personal_money_fixture(&mut session, true);
    set_active_loot_guid_for_test(&mut session, gameobject_guid);
    set_loot_for_test(&mut session, 
        gameobject_guid,
        CreatureLoot {
            loot_guid: loot_object,
            coins: 999,
            unlooted_count: 0,
            loot_type: LOOT_TYPE_CHEST_LIKE_CPP,
            dungeon_encounter_id: 733,
            loot_method: 0,
            loot_master: ObjectGuid::EMPTY,
            round_robin_player: ObjectGuid::EMPTY,
            player_ffa_items: Vec::new(),
            players_looting: Vec::new(),
            allowed_looters: vec![player_guid, other_tapper],
            items: Vec::new(),
            looted_by_player: false,
        },
    );
    set_personal_loot_for_loot_test(&mut session, gameobject_guid, player_guid, 123);
    set_personal_loot_for_loot_test(&mut session, gameobject_guid, other_tapper, 456);

    consume_personal_money_loot_for_test(&mut session, loot_money_packet()).await;

    let mut notify =
        recv_packet_with_opcode(&send_rx, wow_constants::ServerOpcodes::LootMoneyNotify);
    assert_eq!(notify.read_uint64().unwrap(), 123);
    assert_eq!(
        personal_loot_marker_for_test(&session, gameobject_guid, player_guid).1.as_ref(),
        Some(&0)
    );
    assert_eq!(
        personal_loot_marker_for_test(&session, gameobject_guid, other_tapper).1.as_ref(),
        Some(&456)
    );
    assert_eq!(loot_recovery_cache_for_test(&session, gameobject_guid).unwrap().coins, 999);
}

#[test]
fn stored_item_money_completion_applies_db_delta_to_divergent_runtime_base_like_cpp() {
    let (db_after, durable_delta) = accepted_money_delta_for_test(100, 7);
    let runtime_before = 500;
    let runtime_after = runtime_before + durable_delta;

    assert_eq!(db_after, 107);
    assert_eq!(runtime_after, 507);
    assert_ne!(runtime_after, db_after);
}

#[tokio::test]
async fn durable_money_delta_is_order_independent_near_gold_cap_like_cpp() {
    let start = MAX_MONEY_AMOUNT - 7;
    let (after_first, first_delta) = accepted_money_delta_for_test(start, 5);
    let (after_second, second_delta) = accepted_money_delta_for_test(after_first, 5);
    assert_eq!((after_second, first_delta, second_delta), (start + 5, 5, 0));

    let (mut session, send_rx) = make_session_with_send_capacity(8);
    let player_guid = ObjectGuid::create_player(1, 42);
    session.set_player_guid(Some(player_guid));
    prepare_money_player_residence_for_test(&mut session);
    set_player_gold_for_test(&mut session, start);
    let committed = Arc::new(AtomicBool::new(false));
    let command_authority = OwnedLootAuthority::new();
    let command = |durable_delta| LootMoneyApplication::new(
        player_guid,
        test_creature_guid(19_510),
        represented_loot_object_guid_like_cpp(test_creature_guid(19_510)),
        5,
        Arc::new(AtomicU64::new(durable_delta)),
        Default::default(),
        false,
        command_authority.clone(),
        1,
        Arc::clone(&committed),
        Arc::new(AtomicBool::new(false)),
        Arc::new(AtomicBool::new(false)),
        Arc::new(AtomicBool::new(false)),
    );

    // Runtime delivery is deliberately opposite to the locked DB order.
    apply_money_command_for_test(&mut session, command(second_delta))
        .await;
    apply_money_command_for_test(&mut session, command(first_delta))
        .await;

    assert_eq!(player_gold_for_test(&session), start + 5);
    let opcodes = drain_server_opcodes_like_cpp(&send_rx);
    assert_eq!(
        opcodes,
        vec![
            wow_constants::ServerOpcodes::LootMoneyNotify as u16,
            wow_constants::ServerOpcodes::LootMoneyNotify as u16,
        ]
    );
}

#[tokio::test]
async fn represented_creature_money_uses_cpp_money_drop_rate() {
    let mut session = make_session();
    let owner_guid = test_creature_guid(1);
    attach_loot_guid_allocator_for_owner(&mut session, owner_guid);
    session.set_loot_drop_rates_like_cpp(LootDropRatesLikeCpp {
        money: 2.5,
        ..LootDropRatesLikeCpp::default()
    });

    let loot = generate_creature_money_loot_for_test(
            &mut session,
            owner_guid,
            ObjectGuid::create_player(1, 42),
            10,
            25,
            0,
            100,
            100,
            0,
        )
        .await
        .expect("canonical owner map allocates a LootObject");

    assert_eq!(loot.coins, 250);
    assert!(loot.items.is_empty());
}

#[tokio::test]
async fn represented_creature_money_zero_gold_max_stays_zero_like_cpp() {
    let mut session = make_session();
    let owner_guid = test_creature_guid(1);
    attach_loot_guid_allocator_for_owner(&mut session, owner_guid);

    let loot = generate_creature_money_loot_for_test(
            &mut session,
            owner_guid,
            ObjectGuid::create_player(1, 42),
            10,
            25,
            0,
            0,
            0,
            0,
        )
        .await
        .expect("canonical owner map allocates a LootObject");

    assert_eq!(loot.coins, 0);
    assert!(loot.items.is_empty());
}

#[tokio::test]
async fn represented_gameobject_chest_uses_resolved_template_money_like_cpp() {
    let mut session = make_session();
    let gameobject_guid = test_gameobject_guid(91_022);
    attach_loot_guid_allocator_for_owner(&mut session, gameobject_guid);

    let loot = generate_chest_money_loot_for_test(
            &mut session,
            gameobject_guid,
            ObjectGuid::create_player(1, 42),
            GameObjectLootSource::default(),
            &[],
            (123, 123),
        )
        .await
        .expect("canonical owner map allocates a LootObject");

    assert_eq!(loot.coins, 123);
}

#[test]
fn represented_money_removed_erases_missing_players_looting_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    let player_guid = ObjectGuid::create_player(1, 42);
    let open_guid = ObjectGuid::create_player(1, 77);
    let stale_guid = ObjectGuid::create_player(1, 99);
    let owner_guid = test_creature_guid(19_096);
    let loot_object = represented_loot_object_guid_like_cpp(owner_guid);
    let (open_tx, open_rx) = flume::bounded::<Vec<u8>>(1);
    let player_registry = Arc::new(PlayerRegistry::default());
    player_registry.register_or_replace(
        open_guid,
        broadcast_info(open_guid, open_tx),
        Default::default(),
    );
    session.set_player_registry(player_registry);
    session.set_player_guid(Some(player_guid));
    prepare_money_player_residence_for_test(&mut session);
    set_loot_for_test(&mut session, 
        owner_guid,
        CreatureLoot {
            loot_guid: loot_object,
            coins: 7,
            unlooted_count: 0,
            loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
            dungeon_encounter_id: 0,
            loot_method: LOOT_METHOD_GROUP_LIKE_CPP,
            loot_master: ObjectGuid::EMPTY,
            round_robin_player: ObjectGuid::EMPTY,
            player_ffa_items: Vec::new(),
            players_looting: vec![player_guid, open_guid, stale_guid],
            allowed_looters: vec![player_guid, open_guid, stale_guid],
            items: Vec::new(),
            looted_by_player: false,
        },
    );

    notify_money_removed_for_test(&mut session, owner_guid);

    let sent = send_rx.try_recv().unwrap();
    let mut sent = WorldPacket::from_bytes(&sent);
    assert_eq!(
        sent.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::CoinRemoved as u16
    );
    assert_eq!(sent.read_packed_guid().unwrap(), loot_object);

    let sent = open_rx.try_recv().unwrap();
    let mut sent = WorldPacket::from_bytes(&sent);
    assert_eq!(
        sent.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::CoinRemoved as u16
    );
    assert_eq!(
        loot_recovery_cache_for_test(&session, owner_guid).unwrap().players_looting,
        vec![player_guid, open_guid]
    );
}
