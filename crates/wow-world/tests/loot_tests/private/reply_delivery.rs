//! Preserved loot response and publication scenarios.
use super::recovery_support::*;
use wow_constants::InventoryResult;
use wow_entities::INVENTORY_SLOT_BAG_0;
use wow_packet::packets::loot::*;
use wow_world::test_fixtures::loot::{loot_client_type_for_test, loot_fixture_response, loot_response_for_test, master_loot_inventory_error_for_test, notify_cached_loot_item_for_test, notify_committed_loot_item_for_test, open_loot_response_for_test, push_loot_item_for_test, send_loot_failure_for_test};

fn loot_response_threshold(sent: &[u8]) -> u8 {
    let mut pkt = WorldPacket::from_bytes(&sent[2..]);
    let _owner = pkt.read_packed_guid().unwrap();
    let _loot_obj = pkt.read_packed_guid().unwrap();
    let _failure = pkt.read_uint8().unwrap();
    let _acquire = pkt.read_uint8().unwrap();
    pkt.read_uint8().unwrap();
    pkt.read_uint8().unwrap()
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
    set_loot_for_test(&mut session, owner_guid, loot);
    install_cached_test_creature_loot_authority_like_cpp(&mut session, owner_guid, player_guid);
    let authority = loot_recovery_authority_for_test(&mut session, owner_guid)
        .unwrap();
    set_active_loot_guid_for_test(&mut session, owner_guid);
    let response = loot_fixture_response(
        owner_guid,
        loot_recovery_cache_for_test(&session, owner_guid).unwrap(),
        player_guid,
    );

    let sentinel = vec![0xAA, 0x55];
    session.send_tx().send(sentinel.clone()).unwrap();
    let (done_tx, done_rx) = std::sync::mpsc::channel();
    let open_thread = std::thread::spawn(move || {
        open_loot_response_for_test(&mut session, owner_guid, player_guid, response);
        done_tx
            .send((
                has_loot_for_test(&session, owner_guid),
                active_loot_view_owners_for_test(&session).contains(&owner_guid),
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
    set_loot_for_test(&mut session, owner_guid, loot);
    install_cached_test_creature_loot_authority_like_cpp(&mut session, owner_guid, player_guid);
    let authority = loot_recovery_authority_for_test(&mut session, owner_guid)
        .unwrap();
    set_active_loot_guid_for_test(&mut session, owner_guid);
    let response = loot_fixture_response(
        owner_guid,
        loot_recovery_cache_for_test(&session, owner_guid).unwrap(),
        player_guid,
    );

    open_loot_response_for_test(&mut session, owner_guid, player_guid, response);
    let claim = authority
        .reserve_item_like_cpp(player_guid, 0)
        .await
        .unwrap();
    assert_eq!(claim.commit_like_cpp(), Ok(true));
    let committed = authority.snapshot_for_player_like_cpp(player_guid).unwrap();
    notify_committed_loot_item_for_test(&mut session, 
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

#[test]
fn represented_loot_item_push_result_uses_realm_route_and_cpp_encounter_fields() {
    let (mut session, instance_rx) = make_session_with_send();
    let (realm_tx, realm_rx) = flume::bounded(1);
    session.install_realm_send_channel_for_test(realm_tx);
    let player_guid = ObjectGuid::create_player(1, 42);
    let item_guid = ObjectGuid::create_item(1, 700);
    let entry = represented_loot_entry(0, 25, player_guid);

    push_loot_item_for_test(&session, player_guid, item_guid, &entry, 0, 0, 0, 1, 1, false, 615);

    assert!(instance_rx.try_recv().is_err());
    let sent = realm_rx.try_recv().unwrap();
    let mut sent = WorldPacket::from_bytes(&sent);
    assert_eq!(
        sent.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::ItemPushResult as u16
    );
    assert_eq!(sent.read_packed_guid().unwrap(), player_guid);
    assert_eq!(sent.read_uint8().unwrap(), u8::from(INVENTORY_SLOT_BAG_0));
    assert_eq!(sent.read_int32().unwrap(), 0);
    assert_eq!(sent.read_int32().unwrap(), 0);
    assert_eq!(sent.read_int32().unwrap(), 1);
    assert_eq!(sent.read_int32().unwrap(), 1);
    assert_eq!(sent.read_int32().unwrap(), 615);
    assert_eq!(sent.read_int32().unwrap(), 0);
    assert_eq!(sent.read_int32().unwrap(), 0);
    assert_eq!(sent.read_uint32().unwrap(), 0);
    assert_eq!(sent.read_int32().unwrap(), 0);
    assert_eq!(sent.read_packed_guid().unwrap(), item_guid);
    assert!(!sent.read_bit().unwrap());
    assert!(!sent.read_bit().unwrap());
    assert_eq!(sent.read_bits(3).unwrap(), 2);
    assert!(!sent.read_bit().unwrap());
    assert!(sent.read_bit().unwrap());
    assert_eq!(sent.read_int32().unwrap(), 25);
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
    set_loot_for_test(&mut session, 
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

    notify_cached_loot_item_for_test(&mut session, owner_guid, 0);

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
        loot_recovery_cache_for_test(&session, owner_guid).unwrap().players_looting,
        vec![player_guid, open_guid]
    );
}
