//! Loot scenarios for [`super`].
//!
//! Split out of loot_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

#[test]
fn map_owned_loot_guid_sequence_is_shared_across_owner_kinds_like_cpp() {
    let mut session = make_session();
    let map_id = 571;
    let realm_id = 7;
    session.set_realm_id(realm_id);
    let shared_owner_counter = 19_700;
    let creature_owner = ObjectGuid::create_world_object(
        HighGuid::Creature,
        0,
        11,
        map_id,
        17,
        101,
        shared_owner_counter,
    );
    let gameobject_owner = ObjectGuid::create_world_object(
        HighGuid::GameObject,
        0,
        12,
        map_id,
        18,
        202,
        shared_owner_counter,
    );
    attach_loot_guid_allocator_for_owner(&mut session, creature_owner);

    let creature_loot = session
        .next_canonical_loot_object_guid_like_cpp(creature_owner)
        .expect("the creature owner map should allocate a LootObject");
    let gameobject_loot = session
        .next_canonical_loot_object_guid_like_cpp(gameobject_owner)
        .expect("the gameobject owner map should share the LootObject sequence");

    assert_ne!(creature_loot, gameobject_loot);
    assert_eq!(creature_loot.counter(), 1);
    assert_eq!(gameobject_loot.counter(), 2);
    for loot_guid in [creature_loot, gameobject_loot] {
        assert_eq!(loot_guid.high_type(), HighGuid::LootObject);
        assert_eq!(loot_guid.sub_type(), 0);
        assert_eq!(loot_guid.realm_id(), realm_id);
        assert_eq!(loot_guid.map_id(), map_id);
        assert_eq!(loot_guid.server_id(), 0);
        assert_eq!(loot_guid.entry(), 0);
    }

    let manager = session.canonical_map_manager.as_ref().unwrap();
    let mut manager = manager.lock().unwrap();
    assert_eq!(
        manager
            .find_map_mut(u32::from(map_id), 0)
            .unwrap()
            .map_mut()
            .get_max_low_guid_like_cpp(HighGuid::LootObject)
            .unwrap(),
        3
    );
}
#[test]
fn loot_guid_allocator_without_canonical_map_fails_without_advancing_like_cpp() {
    let mut session = make_session();
    let owner_guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 9, 571, 7, 303, 19_701);

    assert!(
        session
            .next_canonical_loot_object_guid_like_cpp(owner_guid)
            .is_none()
    );

    attach_loot_guid_allocator_for_owner(&mut session, owner_guid);
    let first_allocated = session
        .next_canonical_loot_object_guid_like_cpp(owner_guid)
        .expect("the first allocation after attaching the map should succeed");
    assert_eq!(first_allocated.counter(), 1);
}
#[test]
fn loot_guid_allocator_refuses_different_owner_map_without_advancing_like_cpp() {
    let mut session = make_session();
    let canonical_owner =
        ObjectGuid::create_world_object(HighGuid::GameObject, 0, 9, 571, 7, 404, 19_702);
    let other_map_owner =
        ObjectGuid::create_world_object(HighGuid::Creature, 0, 9, 0, 7, 505, 19_703);
    attach_loot_guid_allocator_for_owner(&mut session, canonical_owner);

    assert!(
        session
            .next_canonical_loot_object_guid_like_cpp(other_map_owner)
            .is_none()
    );

    let first_allocated = session
        .next_canonical_loot_object_guid_like_cpp(canonical_owner)
        .expect("the rejected owner must not consume the canonical map sequence");
    assert_eq!(first_allocated.counter(), 1);
}
#[test]
fn personal_loot_pools_receive_distinct_map_owned_guids_like_cpp() {
    let mut session = make_session();
    session.set_realm_id(7);
    let owner_guid =
        ObjectGuid::create_world_object(HighGuid::GameObject, 0, 9, 571, 7, 606, 19_704);
    let first_player = ObjectGuid::create_player(1, 42);
    let second_player = ObjectGuid::create_player(1, 77);
    attach_loot_guid_allocator_for_owner(&mut session, owner_guid);

    let mut loot = authoritative_test_loot_like_cpp(0, true);
    loot.loot_guid = session
        .next_canonical_loot_object_guid_like_cpp(owner_guid)
        .expect("the base personal pool should receive a map-owned LootObject");
    loot.allowed_looters = vec![second_player, first_player];

    let (shared, personal) = session
        .represented_loot_authority_pools_like_cpp(owner_guid, first_player, loot, true)
        .expect("both personal pools should materialize");

    assert!(shared.is_none());
    assert_eq!(personal.len(), 2);
    let first_pool = personal.get(&first_player).unwrap();
    let second_pool = personal.get(&second_player).unwrap();
    assert_ne!(first_pool.loot_guid, second_pool.loot_guid);
    assert_eq!(first_pool.loot_guid.counter(), 1);
    assert_eq!(second_pool.loot_guid.counter(), 2);
    for (player_guid, pool) in [(first_player, first_pool), (second_player, second_pool)] {
        assert_eq!(pool.loot_guid.high_type(), HighGuid::LootObject);
        assert_eq!(pool.loot_guid.realm_id(), 7);
        assert_eq!(pool.loot_guid.map_id(), 571);
        assert_eq!(pool.loot_guid.server_id(), 0);
        assert_eq!(pool.loot_guid.entry(), 0);
        assert_eq!(pool.allowed_looters, vec![player_guid]);
        assert_eq!(pool.items[0].allowed_looters, vec![player_guid]);
    }
}
#[test]
fn represented_loot_type_for_client_matches_cpp_aliases() {
    assert_eq!(
        loot_type_for_client_like_cpp(LOOT_TYPE_NONE_LIKE_CPP),
        LOOT_TYPE_NONE_LIKE_CPP
    );
    assert_eq!(
        loot_type_for_client_like_cpp(LOOT_TYPE_CORPSE_LIKE_CPP),
        LOOT_TYPE_CORPSE_LIKE_CPP
    );
    assert_eq!(
        loot_type_for_client_like_cpp(LOOT_TYPE_ITEM_LIKE_CPP),
        LOOT_TYPE_ITEM_LIKE_CPP
    );
    assert_eq!(
        loot_type_for_client_like_cpp(LOOT_TYPE_GATHERING_NODE_LIKE_CPP),
        LOOT_TYPE_GATHERING_NODE_LIKE_CPP
    );
    assert_eq!(
        loot_type_for_client_like_cpp(LOOT_TYPE_CHEST_LIKE_CPP),
        LOOT_TYPE_CHEST_LIKE_CPP
    );
    assert_eq!(
        loot_type_for_client_like_cpp(LOOT_TYPE_CORPSE_PERSONAL_LIKE_CPP),
        LOOT_TYPE_CORPSE_PERSONAL_LIKE_CPP
    );
    assert_eq!(
        loot_type_for_client_like_cpp(LOOT_TYPE_PROSPECTING_LIKE_CPP),
        LOOT_TYPE_DISENCHANTING_LIKE_CPP
    );
    assert_eq!(
        loot_type_for_client_like_cpp(LOOT_TYPE_MILLING_LIKE_CPP),
        LOOT_TYPE_DISENCHANTING_LIKE_CPP
    );
    assert_eq!(
        loot_type_for_client_like_cpp(LOOT_TYPE_INSIGNIA_LIKE_CPP),
        LOOT_TYPE_SKINNING_LIKE_CPP
    );
    assert_eq!(
        loot_type_for_client_like_cpp(LOOT_TYPE_FISHINGHOLE_LIKE_CPP),
        LOOT_TYPE_FISHING_LIKE_CPP
    );
    assert_eq!(
        loot_type_for_client_like_cpp(LOOT_TYPE_FISHING_JUNK_LIKE_CPP),
        LOOT_TYPE_FISHING_LIKE_CPP
    );
}
#[tokio::test]
async fn represented_loot_response_acquire_reason_uses_cpp_loot_type_mapping() {
    let mut session = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let owner_guid = test_creature_guid(19_096);
    let loot_guid = represented_loot_object_guid_like_cpp(owner_guid);
    let entry = represented_loot_entry(0, 25, player_guid);
    register_test_creature_like_cpp(&mut session, test_creature(owner_guid, false));
    session.loot_table.insert(
        owner_guid,
        CreatureLoot {
            loot_guid,
            coins: 0,
            unlooted_count: 1,
            loot_type: LOOT_TYPE_PROSPECTING_LIKE_CPP,
            dungeon_encounter_id: 0,
            loot_method: 0,
            loot_master: ObjectGuid::EMPTY,
            round_robin_player: ObjectGuid::EMPTY,
            player_ffa_items: Vec::new(),
            players_looting: Vec::new(),
            allowed_looters: vec![player_guid],
            items: vec![entry],
            looted_by_player: false,
        },
    );
    install_cached_test_creature_loot_authority_like_cpp(&mut session, owner_guid, player_guid);

    let response = session
        .represented_loot_response_for_owner_like_cpp(owner_guid, player_guid, false)
        .await
        .unwrap();

    assert_eq!(response.acquire_reason, LOOT_TYPE_DISENCHANTING_LIKE_CPP);
}
#[test]
fn represented_start_loot_roll_carries_cpp_dungeon_encounter_id() {
    let player_guid = ObjectGuid::create_player(1, 42);
    let loot_obj = ObjectGuid::create_world_object(HighGuid::LootObject, 0, 1, 0, 0, 1, 900);
    let entry = represented_loot_entry(0, 25, player_guid);

    let packet = start_loot_roll_packet_like_cpp(
        loot_obj,
        571,
        LOOT_METHOD_GROUP_LIKE_CPP,
        &entry,
        ROLL_ALL_TYPE_NO_DISENCHANT_LIKE_CPP,
        615,
    );

    assert_eq!(packet.dungeon_encounter_id, 615);
}
#[test]
fn represented_loot_removed_uses_players_looting_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    let player_guid = ObjectGuid::create_player(1, 42);
    let open_guid = ObjectGuid::create_player(1, 77);
    let closed_guid = ObjectGuid::create_player(1, 88);
    let stale_guid = ObjectGuid::create_player(1, 99);
    let owner_guid = test_creature_guid(19_095);
    let loot_object = represented_loot_object_guid_like_cpp(owner_guid);
    let (open_tx, open_rx) = flume::bounded::<Vec<u8>>(1);
    let (closed_tx, closed_rx) = flume::bounded::<Vec<u8>>(1);
    let player_registry = Arc::new(PlayerRegistry::default());
    player_registry.register_or_replace(
        open_guid,
        broadcast_info(open_guid, open_tx),
        Default::default(),
    );
    player_registry.register_or_replace(
        closed_guid,
        broadcast_info(closed_guid, closed_tx),
        Default::default(),
    );
    session.set_player_registry(player_registry);
    session.set_player_guid(Some(player_guid));
    session.loot_table.insert(
        owner_guid,
        CreatureLoot {
            loot_guid: loot_object,
            coins: 0,
            unlooted_count: 0,
            loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
            dungeon_encounter_id: 0,
            loot_method: LOOT_METHOD_GROUP_LIKE_CPP,
            loot_master: ObjectGuid::EMPTY,
            round_robin_player: ObjectGuid::EMPTY,
            player_ffa_items: Vec::new(),
            players_looting: vec![player_guid, open_guid, stale_guid],
            allowed_looters: vec![player_guid, open_guid, closed_guid, stale_guid],
            items: vec![LootEntry {
                loot_list_id: 0,
                item_id: 25,
                quantity: 1,
                random_properties_id: 0,
                random_properties_seed: 0,
                item_context: 0,
                flags: LootEntryFlags::default(),
                allowed_looters: vec![player_guid, open_guid, closed_guid, stale_guid],
                roll_winner: ObjectGuid::EMPTY,
                ffa_looted_by: Vec::new(),
                taken: false,
            }],
            looted_by_player: false,
        },
    );

    session.represented_notify_loot_item_removed_like_cpp(owner_guid, 0);

    let sent = send_rx.try_recv().unwrap();
    let mut sent = WorldPacket::from_bytes(&sent);
    assert_eq!(
        sent.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::LootRemoved as u16
    );
    assert_eq!(sent.read_packed_guid().unwrap(), owner_guid);
    assert_eq!(sent.read_packed_guid().unwrap(), loot_object);
    assert_eq!(sent.read_uint8().unwrap(), 0);

    let sent = open_rx.try_recv().unwrap();
    let mut sent = WorldPacket::from_bytes(&sent);
    assert_eq!(
        sent.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::LootRemoved as u16
    );
    assert!(closed_rx.try_recv().is_err());
    assert_eq!(
        session.loot_table.get(&owner_guid).unwrap().players_looting,
        vec![player_guid, open_guid]
    );
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
    session.loot_table.insert(
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

    session.represented_notify_money_removed_like_cpp(owner_guid);

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
        session.loot_table.get(&owner_guid).unwrap().players_looting,
        vec![player_guid, open_guid]
    );
}
#[test]
fn loot_directory_delivery_rejects_replaced_session_generation_like_cpp() {
    let guid = ObjectGuid::create_player(1, 91);
    let registry = PlayerRegistry::new();
    let (first_tx, first_rx) = flume::bounded(2);
    registry.register_or_replace(guid, broadcast_info(guid, first_tx), Default::default());
    let stale = registry
        .loot_presence(guid)
        .expect("first connected loot recipient");

    let (replacement_tx, replacement_rx) = flume::bounded(2);
    registry.register_or_replace(
        guid,
        broadcast_info(guid, replacement_tx),
        Default::default(),
    );

    assert_eq!(
        registry.send_current_packet(stale.registration, vec![0xAA]),
        Err(crate::session::directory::PlayerDirectorySendError::StaleRegistration)
    );
    assert!(first_rx.try_recv().is_err());
    assert!(replacement_rx.try_recv().is_err());

    let current = registry
        .loot_presence(guid)
        .expect("replacement loot recipient");
    registry
        .send_current_packet(current.registration, vec![0xBB])
        .expect("current generation receives its packet");
    assert_eq!(replacement_rx.try_recv().unwrap(), vec![0xBB]);
}
#[tokio::test]
async fn full_loot_response_queue_rolls_back_open_without_blocking_authority_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(1);
    let player_guid = ObjectGuid::create_player(1, 61_900);
    let owner_guid = test_creature_guid(61_901);
    session.set_player_guid(Some(player_guid));
    session.set_player_position_like_cpp(Position::ZERO);
    register_test_creature_like_cpp(&mut session, test_creature(owner_guid, false));

    let mut loot = authoritative_test_loot_like_cpp(0, true);
    loot.loot_guid = represented_loot_object_guid_like_cpp(owner_guid);
    loot.allowed_looters = vec![player_guid];
    loot.items[0].allowed_looters = vec![player_guid];
    session.loot_table.insert(owner_guid, loot);
    install_cached_test_creature_loot_authority_like_cpp(&mut session, owner_guid, player_guid);
    let authority = session
        .represented_owned_loot_authority_like_cpp(owner_guid)
        .unwrap();
    session.set_active_loot_guid(owner_guid);
    let response = authoritative_test_loot_response_like_cpp(
        owner_guid,
        &session.loot_table[&owner_guid],
        player_guid,
    );

    let sentinel = vec![0xAA, 0x55];
    session.send_tx().send(sentinel.clone()).unwrap();
    let (done_tx, done_rx) = std::sync::mpsc::channel();
    let open_thread = std::thread::spawn(move || {
        session.represented_on_loot_opened_like_cpp(owner_guid, player_guid, response);
        done_tx
            .send((
                session.loot_table.contains_key(&owner_guid),
                session.active_loot_view_owners.contains(&owner_guid),
            ))
            .unwrap();
    });

    let (cached, active) = done_rx
        .recv_timeout(Duration::from_millis(500))
        .expect("a full socket queue must not block while loot authority is locked");
    open_thread.join().unwrap();
    assert!(!cached);
    assert!(!active);
    assert_eq!(send_rx.try_recv().unwrap(), sentinel);
    assert!(send_rx.try_recv().is_err(), "no LootResponse was enqueued");

    let rejected = authority.snapshot_for_player_like_cpp(player_guid).unwrap();
    assert!(rejected.loot.players_looting.is_empty());
    assert!(!rejected.loot.looted_by_player);

    let claim = tokio::time::timeout(
        Duration::from_millis(500),
        authority.reserve_item_like_cpp(player_guid, 0),
    )
    .await
    .expect("a failed response enqueue must release the authority mutex")
    .unwrap();
    assert!(claim.rollback_like_cpp());
    authority.retire_like_cpp();
    assert!(authority.is_retired_like_cpp());
}
#[tokio::test]
async fn successful_loot_open_queues_response_before_claim_removal_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(4);
    let player_guid = ObjectGuid::create_player(1, 61_910);
    let owner_guid = test_creature_guid(61_911);
    session.set_player_guid(Some(player_guid));
    session.set_player_position_like_cpp(Position::ZERO);
    register_test_creature_like_cpp(&mut session, test_creature(owner_guid, false));

    let mut loot = authoritative_test_loot_like_cpp(0, true);
    loot.loot_guid = represented_loot_object_guid_like_cpp(owner_guid);
    loot.allowed_looters = vec![player_guid];
    loot.items[0].allowed_looters = vec![player_guid];
    session.loot_table.insert(owner_guid, loot);
    install_cached_test_creature_loot_authority_like_cpp(&mut session, owner_guid, player_guid);
    let authority = session
        .represented_owned_loot_authority_like_cpp(owner_guid)
        .unwrap();
    session.set_active_loot_guid(owner_guid);
    let response = authoritative_test_loot_response_like_cpp(
        owner_guid,
        &session.loot_table[&owner_guid],
        player_guid,
    );

    session.represented_on_loot_opened_like_cpp(owner_guid, player_guid, response);
    let claim = authority
        .reserve_item_like_cpp(player_guid, 0)
        .await
        .unwrap();
    assert_eq!(claim.commit_like_cpp(), Ok(true));
    let committed = authority.snapshot_for_player_like_cpp(player_guid).unwrap();
    session.represented_notify_loot_item_removed_from_snapshot_like_cpp(
        owner_guid,
        Some(&authority),
        &committed,
        0,
    );

    let opcodes = drain_server_opcodes_like_cpp(&send_rx);
    let response_index = opcodes
        .iter()
        .position(|opcode| *opcode == wow_constants::ServerOpcodes::LootResponse as u16)
        .expect("the accepted opening response was queued");
    let removal_index = opcodes
        .iter()
        .position(|opcode| *opcode == wow_constants::ServerOpcodes::LootRemoved as u16)
        .expect("the committed claim removal was queued");
    assert!(
        response_index < removal_index,
        "the authority lock must order LootResponse before LootRemoved: {opcodes:?}"
    );
}
#[tokio::test]
async fn two_sessions_claim_one_authoritative_money_pool_exactly_once_like_cpp() {
    let (first, _first_rx, second, _second_rx, owner, first_guid, _) =
        two_sessions_with_authoritative_creature_loot_like_cpp(authoritative_test_loot_like_cpp(
            9, false,
        ));
    let barrier = Arc::new(tokio::sync::Barrier::new(3));
    let first_barrier = Arc::clone(&barrier);
    let first_task = tokio::spawn(async move {
        let mut first = first;
        first_barrier.wait().await;
        first.handle_loot_money(loot_money_packet()).await;
        first
    });
    let second_barrier = Arc::clone(&barrier);
    let second_task = tokio::spawn(async move {
        let mut second = second;
        second_barrier.wait().await;
        second.handle_loot_money(loot_money_packet()).await;
        second
    });
    barrier.wait().await;

    let mut first = first_task.await.unwrap();
    let second = second_task.await.unwrap();
    assert_eq!(
        first.player_gold_like_cpp() + second.player_gold_like_cpp(),
        9
    );
    let authority = first
        .represented_owned_loot_authority_like_cpp(owner)
        .unwrap();
    assert_eq!(
        authority
            .snapshot_for_player_like_cpp(first_guid)
            .unwrap()
            .loot
            .coins,
        0
    );
}
#[tokio::test]
async fn failed_authoritative_money_persistence_rolls_back_for_retry_like_cpp() {
    let (mut first, _first_rx, _second, _second_rx, owner, first_guid, _) =
        two_sessions_with_authoritative_creature_loot_like_cpp(authoritative_test_loot_like_cpp(
            7, false,
        ));
    first.set_loot_money_persistence_test_result_like_cpp(false);

    first.handle_loot_money(loot_money_packet()).await;
    let authority = first
        .represented_owned_loot_authority_like_cpp(owner)
        .unwrap();
    assert_eq!(first.player_gold_like_cpp(), 0);
    assert_eq!(
        authority
            .snapshot_for_player_like_cpp(first_guid)
            .unwrap()
            .loot
            .coins,
        7
    );

    first.set_loot_money_persistence_test_result_like_cpp(true);
    first.handle_loot_money(loot_money_packet()).await;
    assert_eq!(first.player_gold_like_cpp(), 7);
    assert_eq!(
        authority
            .snapshot_for_player_like_cpp(first_guid)
            .unwrap()
            .loot
            .coins,
        0
    );
}
#[tokio::test]
async fn stale_active_money_view_cannot_claim_replacement_generation_like_cpp() {
    let (mut first, _first_rx, _second, _second_rx, owner, first_guid, second_guid) =
        two_sessions_with_authoritative_creature_loot_like_cpp(authoritative_test_loot_like_cpp(
            7, false,
        ));
    let authority = first
        .represented_owned_loot_authority_like_cpp(owner)
        .unwrap();
    let opened_generation = first.active_loot_view_generations_like_cpp[&owner];
    let mut replacement = authoritative_test_loot_like_cpp(11, false);
    replacement.loot_guid = represented_loot_object_guid_like_cpp(owner);
    replacement.allowed_looters = vec![first_guid, second_guid];
    let replacement_generation = authority.replace_like_cpp(Some(replacement), HashMap::new());
    assert_ne!(opened_generation, replacement_generation);

    first.handle_loot_money(loot_money_packet()).await;

    assert_eq!(first.player_gold_like_cpp(), 0);
    let snapshot = authority.snapshot_for_player_like_cpp(first_guid).unwrap();
    assert_eq!(snapshot.generation, replacement_generation);
    assert_eq!(snapshot.loot.coins, 11);
}
#[tokio::test]
async fn durable_money_delta_is_order_independent_near_gold_cap_like_cpp() {
    let start = MAX_MONEY_AMOUNT - 7;
    let (after_first, first_delta) = loot_money_durable_outcome_like_cpp(start, 5);
    let (after_second, second_delta) = loot_money_durable_outcome_like_cpp(after_first, 5);
    assert_eq!((after_second, first_delta, second_delta), (start + 5, 5, 0));

    let (mut session, send_rx) = make_session_with_send_capacity(8);
    let player_guid = ObjectGuid::create_player(1, 42);
    session.set_player_guid(Some(player_guid));
    session.set_player_gold_like_cpp(start);
    let committed = Arc::new(AtomicBool::new(false));
    let command_authority = OwnedLootAuthority::new();
    let command = |durable_delta| ApplyLootMoneyLikeCppCommand {
        recipient: player_guid,
        loot_owner: test_creature_guid(19_510),
        loot_obj: represented_loot_object_guid_like_cpp(test_creature_guid(19_510)),
        amount: 5,
        durable_applied_amount: Arc::new(AtomicU64::new(durable_delta)),
        durable_persistence_tracker: Default::default(),
        sole_looter: false,
        authority: command_authority.clone(),
        authority_generation: 1,
        authority_committed: Arc::clone(&committed),
        send_coin_removed: Arc::new(AtomicBool::new(false)),
        applied: Arc::new(AtomicBool::new(false)),
        published: Arc::new(AtomicBool::new(false)),
    };

    // Runtime delivery is deliberately opposite to the locked DB order.
    session
        .handle_apply_loot_money_like_cpp_command(command(second_delta))
        .await;
    session
        .handle_apply_loot_money_like_cpp_command(command(first_delta))
        .await;

    assert_eq!(session.player_gold_like_cpp(), start + 5);
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
async fn durable_old_generation_payout_does_not_touch_replacement_loot_like_cpp() {
    let (mut first, first_rx, _second, _second_rx, owner, first_guid, second_guid) =
        two_sessions_with_authoritative_creature_loot_like_cpp(authoritative_test_loot_like_cpp(
            7, false,
        ));
    let _ = drain_server_opcodes_like_cpp(&first_rx);
    let authority = first
        .represented_owned_loot_authority_like_cpp(owner)
        .unwrap();
    let old_generation = first.active_loot_view_generations_like_cpp[&owner];
    let mut replacement = authoritative_test_loot_like_cpp(17, false);
    replacement.loot_guid = represented_loot_object_guid_like_cpp(owner);
    replacement.allowed_looters = vec![first_guid, second_guid];
    authority.replace_like_cpp(Some(replacement), HashMap::new());
    authority.add_viewer_like_cpp(first_guid).unwrap();

    first
        .handle_apply_loot_money_like_cpp_command(ApplyLootMoneyLikeCppCommand {
            recipient: first_guid,
            loot_owner: owner,
            loot_obj: represented_loot_object_guid_like_cpp(owner),
            amount: 3,
            durable_applied_amount: Arc::new(AtomicU64::new(3)),
            durable_persistence_tracker: Default::default(),
            sole_looter: true,
            authority: authority.clone(),
            authority_generation: old_generation,
            authority_committed: Arc::new(AtomicBool::new(true)),
            send_coin_removed: Arc::new(AtomicBool::new(true)),
            applied: Arc::new(AtomicBool::new(false)),
            published: Arc::new(AtomicBool::new(false)),
        })
        .await;

    assert_eq!(first.player_gold_like_cpp(), 3);
    let snapshot = authority.snapshot_for_player_like_cpp(first_guid).unwrap();
    assert_eq!(snapshot.loot.coins, 17);
    assert!(snapshot.loot.players_looting.contains(&first_guid));
    let opcodes = drain_server_opcodes_like_cpp(&first_rx);
    assert!(!opcodes.contains(&(wow_constants::ServerOpcodes::CoinRemoved as u16)));
    assert_eq!(
        opcodes
            .iter()
            .filter(|opcode| { **opcode == wow_constants::ServerOpcodes::LootMoneyNotify as u16 })
            .count(),
        1
    );
}
#[tokio::test]
async fn remote_group_money_is_one_atomic_durable_fanout_like_cpp() {
    let (mut first, first_rx, mut second, second_rx, owner, first_guid, second_guid) =
        two_sessions_with_authoritative_creature_loot_like_cpp(authoritative_test_loot_like_cpp(
            9, false,
        ));
    install_group_loot_group(&mut first, first_guid, second_guid);
    let player_registry = Arc::new(PlayerRegistry::default());
    let (first_presence_tx, _first_presence_rx) = flume::bounded(1);
    player_registry.register_or_replace(
        first_guid,
        broadcast_info(first_guid, first_presence_tx),
        Default::default(),
    );
    let (registry_send_tx, _registry_send_rx) = flume::bounded(8);
    let mut second_info = broadcast_info(second_guid, registry_send_tx);
    second_info.command_tx = second.session_command_tx();
    player_registry.register_or_replace(second_guid, second_info, Default::default());
    first.set_player_registry(player_registry);
    let authority = first
        .represented_owned_loot_authority_like_cpp(owner)
        .unwrap();
    let _ = drain_server_opcodes_like_cpp(&first_rx);
    let _ = drain_server_opcodes_like_cpp(&second_rx);

    first.handle_loot_money(loot_money_packet()).await;
    second.process_represented_session_commands_like_cpp().await;

    // C++ divides integral copper and discards the remainder.
    assert_eq!(first.player_gold_like_cpp(), 4);
    assert_eq!(second.player_gold_like_cpp(), 4);
    assert_eq!(
        authority
            .snapshot_for_player_like_cpp(first_guid)
            .unwrap()
            .loot
            .coins,
        0
    );
    for opcodes in [
        drain_server_opcodes_like_cpp(&first_rx),
        drain_server_opcodes_like_cpp(&second_rx),
    ] {
        let coin = opcodes
            .iter()
            .position(|opcode| *opcode == wow_constants::ServerOpcodes::CoinRemoved as u16)
            .expect("active viewer must receive CoinRemoved");
        let money = opcodes
            .iter()
            .position(|opcode| *opcode == wow_constants::ServerOpcodes::LootMoneyNotify as u16)
            .expect("payout recipient must receive LootMoneyNotify");
        assert!(coin < money, "C++ removes coins before notifying payout");
    }
}
#[test]
fn pickpocket_money_is_not_shared_with_the_group_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send();
    let player_guid = ObjectGuid::create_player(1, 42);
    let member_guid = ObjectGuid::create_player(1, 43);
    let owner = test_creature_guid(19_501);
    session.set_player_guid(Some(player_guid));
    session.set_player_position_like_cpp(Position::ZERO);
    install_group_loot_group(&mut session, player_guid, member_guid);
    let registry = Arc::new(PlayerRegistry::default());
    let (member_tx, _member_rx) = flume::bounded(1);
    registry.register_or_replace(
        member_guid,
        broadcast_info(member_guid, member_tx),
        Default::default(),
    );
    session.set_player_registry(registry);
    let mut loot = authoritative_test_loot_like_cpp(8, false);
    loot.loot_type = LOOT_TYPE_PICKPOCKETING_LIKE_CPP;
    loot.allowed_looters = vec![player_guid, member_guid];
    session.loot_table.insert(owner, loot);

    assert_eq!(
        session.represented_loot_money_recipients_like_cpp(owner),
        vec![player_guid]
    );
}
#[test]
fn vehicle_corpse_money_shares_and_pool_allowed_looters_control_membership_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send();
    let player_guid = ObjectGuid::create_player(1, 42);
    let member_guid = ObjectGuid::create_player(1, 43);
    let owner = ObjectGuid::create_vehicle_like_cpp(1, 0, 1, 19_502);
    session.set_player_guid(Some(player_guid));
    session.set_player_position_like_cpp(Position::ZERO);
    install_group_loot_group(&mut session, player_guid, member_guid);
    let registry = Arc::new(PlayerRegistry::default());
    let (player_tx, _player_rx) = flume::bounded(1);
    registry.register_or_replace(
        player_guid,
        broadcast_info(player_guid, player_tx),
        Default::default(),
    );
    let (member_tx, _member_rx) = flume::bounded(1);
    registry.register_or_replace(
        member_guid,
        broadcast_info(member_guid, member_tx),
        Default::default(),
    );
    session.set_player_registry(registry);

    let mut loot = authoritative_test_loot_like_cpp(8, false);
    loot.loot_type = LOOT_TYPE_CORPSE_LIKE_CPP;
    loot.allowed_looters = vec![player_guid];
    session.loot_table.insert(owner, loot);
    assert_eq!(
        session.represented_loot_money_recipients_like_cpp(owner),
        vec![player_guid]
    );

    session
        .loot_table
        .get_mut(&owner)
        .unwrap()
        .allowed_looters
        .push(member_guid);
    assert_eq!(
        session.represented_loot_money_recipients_like_cpp(owner),
        vec![player_guid, member_guid]
    );
}
