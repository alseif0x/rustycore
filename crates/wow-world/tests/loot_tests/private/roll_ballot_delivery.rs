//! Original first-open, pass and vote packet sequences.
use super::recovery_support::*;
use std::sync::Mutex;
use wow_entities::Player;
use wow_loot::{LOOT_METHOD_GROUP_LIKE_CPP, ROLL_VOTE_GREED_LIKE_CPP, ROLL_VOTE_NEED_LIKE_CPP, ROLL_VOTE_NOT_EMITTED_YET_LIKE_CPP, ROLL_VOTE_NOT_VALID_LIKE_CPP};
use wow_world::session::mailbox::LootRollVoteCommand;
use wow_world::test_fixtures::loot::{loot_roll_observation_for_test, process_loot_commands_for_test, allocate_loot_guid_for_test, sync_creature_loot_fixture_for_test};

#[tokio::test]
async fn loot_unit_group_loot_first_open_starts_roll_for_blocked_item_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(4);
    let player_guid = ObjectGuid::create_player(1, 42);
    let candidate_guid = ObjectGuid::create_player(1, 77);
    let disconnected_guid = ObjectGuid::create_player(1, 88);
    let owner_guid = test_creature_guid(19_049);
    let loot_object = represented_loot_object_guid_like_cpp(owner_guid);
    let (candidate_tx, candidate_rx) = flume::bounded::<Vec<u8>>(4);
    let player_registry = Arc::new(PlayerRegistry::with_canonical_player_fixtures_like_cpp());
    player_registry.register_or_replace(
        candidate_guid,
        broadcast_info(candidate_guid, candidate_tx),
        Default::default(),
    );
    session.set_player_registry(player_registry);
    session.set_player_guid(Some(player_guid));
    install_group_loot_group(&mut session, player_guid, candidate_guid);
    register_test_creature_like_cpp(&mut session, test_creature(owner_guid, false));
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
            players_looting: Vec::new(),
            allowed_looters: vec![player_guid, candidate_guid],
            items: vec![LootEntry {
                loot_list_id: 0,
                item_id: 25,
                quantity: 1,
                random_properties_id: 0,
                random_properties_seed: 0,
                item_context: 0,
                flags: LootEntryFlags {
                    follow_loot_rules: true,
                    blocked: true,
                    ..Default::default()
                },
                allowed_looters: vec![player_guid, candidate_guid, disconnected_guid],
                roll_winner: ObjectGuid::EMPTY,
                ffa_looted_by: Vec::new(),
                taken: false,
            }],
            looted_by_player: false,
        },
    );

    install_cached_test_creature_loot_authority_like_cpp(&mut session, owner_guid, player_guid);
    handle_loot_unit_for_test(&mut session, loot_unit_packet(owner_guid))
        .await;

    let response = send_rx.try_recv().unwrap();
    let mut response = WorldPacket::from_bytes(&response);
    assert_eq!(
        response.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::LootResponse as u16
    );
    let loot_list = send_rx.try_recv().unwrap();
    let mut loot_list = WorldPacket::from_bytes(&loot_list);
    assert_eq!(
        loot_list.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::LootList as u16
    );

    let start_roll = send_rx.try_recv().unwrap();
    let mut start_roll = WorldPacket::from_bytes(&start_roll);
    assert_eq!(
        start_roll.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::StartLootRoll as u16
    );
    assert_eq!(start_roll.read_packed_guid().unwrap(), loot_object);
    assert_eq!(start_roll.read_int32().unwrap(), 0);
    assert_eq!(start_roll.read_uint32().unwrap(), 60_000);
    assert_eq!(start_roll.read_uint8().unwrap(), 0x07);
    assert_eq!(start_roll.read_uint32().unwrap(), 0);
    assert_eq!(start_roll.read_uint32().unwrap(), 0);
    assert_eq!(start_roll.read_uint32().unwrap(), 0);
    assert_eq!(start_roll.read_uint32().unwrap(), 0);
    assert_eq!(start_roll.read_uint8().unwrap(), LOOT_METHOD_GROUP_LIKE_CPP);
    assert_eq!(start_roll.read_int32().unwrap(), 0);
    assert_eq!(start_roll.read_bits(2).unwrap(), 0);
    assert_eq!(start_roll.read_bits(3).unwrap(), 1);
    assert!(send_rx.try_recv().is_err());

    let remote_loot_list = candidate_rx.try_recv().unwrap();
    let mut remote_loot_list = WorldPacket::from_bytes(&remote_loot_list);
    assert_eq!(
        remote_loot_list.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::LootList as u16
    );
    let remote_start_roll = candidate_rx.try_recv().unwrap();
    let mut remote_start_roll = WorldPacket::from_bytes(&remote_start_roll);
    assert_eq!(
        remote_start_roll.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::StartLootRoll as u16
    );
    assert_eq!(remote_start_roll.read_packed_guid().unwrap(), loot_object);

    let state = loot_roll_observation_for_test(&session, loot_object, 0)
        .unwrap();
    assert_eq!(
        state.vote(player_guid).unwrap().vote,
        ROLL_VOTE_NOT_EMITTED_YET_LIKE_CPP
    );
    assert_eq!(
        state.vote(candidate_guid).unwrap().vote,
        ROLL_VOTE_NOT_EMITTED_YET_LIKE_CPP
    );
    assert_eq!(
        state.vote(disconnected_guid).unwrap().vote,
        ROLL_VOTE_NOT_VALID_LIKE_CPP
    );

    let entry = &loot_recovery_cache_for_test(&session, owner_guid).unwrap().items[0];
    assert!(entry.flags.blocked);
    assert!(!entry.flags.under_threshold);
    assert!(
        loot_recovery_cache_for_test(&session, owner_guid)
            .unwrap()
            .looted_by_player
    );
}

#[tokio::test]
async fn loot_roll_all_passed_unblocks_without_all_passed_to_valid_voters_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(8);
    let player_guid = ObjectGuid::create_player(1, 42);
    let candidate_guid = ObjectGuid::create_player(1, 77);
    let owner_guid = test_creature_guid(19_054);
    let loot_object = represented_loot_object_guid_like_cpp(owner_guid);
    let (player_tx, player_rx) = flume::bounded::<Vec<u8>>(8);
    let (candidate_tx, candidate_rx) = flume::bounded::<Vec<u8>>(8);
    let player_registry = Arc::new(PlayerRegistry::with_canonical_player_fixtures_like_cpp());
    player_registry.register_or_replace(
        player_guid,
        broadcast_info(player_guid, player_tx),
        Default::default(),
    );
    player_registry.register_or_replace(
        candidate_guid,
        broadcast_info(candidate_guid, candidate_tx),
        Default::default(),
    );
    session.set_player_registry(player_registry);
    session.set_player_guid(Some(player_guid));
    install_group_loot_group(&mut session, player_guid, candidate_guid);
    register_test_creature_like_cpp(&mut session, test_creature(owner_guid, false));
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
            players_looting: Vec::new(),
            allowed_looters: vec![player_guid, candidate_guid],
            items: vec![LootEntry {
                loot_list_id: 0,
                item_id: 25,
                quantity: 1,
                random_properties_id: 0,
                random_properties_seed: 0,
                item_context: 0,
                flags: LootEntryFlags {
                    follow_loot_rules: true,
                    blocked: true,
                    ..Default::default()
                },
                allowed_looters: vec![player_guid, candidate_guid],
                roll_winner: ObjectGuid::EMPTY,
                ffa_looted_by: Vec::new(),
                taken: false,
            }],
            looted_by_player: false,
        },
    );

    install_cached_test_creature_loot_authority_like_cpp(&mut session, owner_guid, player_guid);
    handle_loot_unit_for_test(&mut session, loot_unit_packet(owner_guid)).await;
    let _response = send_rx.try_recv().unwrap();
    let _loot_list = send_rx.try_recv().unwrap();
    let _start_roll = send_rx.try_recv().unwrap();
    let _remote_loot_list = candidate_rx.try_recv().unwrap();
    let _remote_start_roll = candidate_rx.try_recv().unwrap();

    handle_loot_roll_for_test(&mut session, LootRoll {
            loot_obj: loot_object,
            loot_list_id: 0,
            roll_type: ROLL_VOTE_PASS_LIKE_CPP,
        })
        .await;
    let _local_pass_roll = send_rx.try_recv().unwrap();
    let _remote_pass_roll = candidate_rx.try_recv().unwrap();
    let pass_state = loot_roll_observation_for_test(&session, loot_object, 0)
        .expect("roll state should stay open until every voter passes");
    assert_eq!(
        pass_state.vote(player_guid).unwrap().roll_number,
        0,
        "C++ LootRoll::PlayerVote does not call urand for Pass"
    );

    session.set_player_guid(Some(candidate_guid));
    handle_loot_roll_for_test(&mut session, LootRoll {
            loot_obj: loot_object,
            loot_list_id: 0,
            roll_type: ROLL_VOTE_PASS_LIKE_CPP,
        })
        .await;

    let candidate_pass_roll = send_rx.try_recv().unwrap();
    let mut candidate_pass_roll = WorldPacket::from_bytes(&candidate_pass_roll);
    assert_eq!(
        candidate_pass_roll.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::LootRoll as u16
    );
    assert_eq!(candidate_pass_roll.read_packed_guid().unwrap(), loot_object);
    assert_eq!(
        candidate_pass_roll.read_packed_guid().unwrap(),
        candidate_guid
    );
    assert_eq!(candidate_pass_roll.read_int32().unwrap(), -1);
    assert_eq!(
        candidate_pass_roll.read_uint8().unwrap(),
        ROLL_VOTE_PASS_LIKE_CPP
    );

    let original_pass_roll = player_rx.try_recv().unwrap();
    let mut original_pass_roll = WorldPacket::from_bytes(&original_pass_roll);
    assert_eq!(
        original_pass_roll.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::LootRoll as u16
    );
    assert!(send_rx.try_recv().is_err());
    assert!(player_rx.try_recv().is_err());
    assert!(candidate_rx.try_recv().is_err());

    let entry = &loot_recovery_cache_for_test(&session, owner_guid).unwrap().items[0];
    assert!(!entry.flags.blocked);
    assert!(entry.roll_winner.is_empty());
    assert!(
        !loot_roll_observation_for_test(&session, loot_object, 0).is_some()
    );
}

#[tokio::test]
async fn loot_roll_need_vote_broadcasts_immediate_roll_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(5);
    let player_guid = ObjectGuid::create_player(1, 42);
    let candidate_guid = ObjectGuid::create_player(1, 77);
    let owner_guid = test_creature_guid(19_052);
    let loot_object = represented_loot_object_guid_like_cpp(owner_guid);
    let (candidate_tx, candidate_rx) = flume::bounded::<Vec<u8>>(5);
    let player_registry = Arc::new(PlayerRegistry::with_canonical_player_fixtures_like_cpp());
    player_registry.register_or_replace(
        candidate_guid,
        broadcast_info(candidate_guid, candidate_tx),
        Default::default(),
    );
    session.set_player_registry(player_registry);
    session.set_player_guid(Some(player_guid));
    install_group_loot_group(&mut session, player_guid, candidate_guid);
    register_test_creature_like_cpp(&mut session, test_creature(owner_guid, false));
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
            players_looting: Vec::new(),
            allowed_looters: vec![player_guid, candidate_guid],
            items: vec![LootEntry {
                loot_list_id: 0,
                item_id: 25,
                quantity: 1,
                random_properties_id: 0,
                random_properties_seed: 0,
                item_context: 0,
                flags: LootEntryFlags {
                    follow_loot_rules: true,
                    blocked: true,
                    ..Default::default()
                },
                allowed_looters: vec![player_guid, candidate_guid],
                roll_winner: ObjectGuid::EMPTY,
                ffa_looted_by: Vec::new(),
                taken: false,
            }],
            looted_by_player: false,
        },
    );

    install_cached_test_creature_loot_authority_like_cpp(&mut session, owner_guid, player_guid);
    handle_loot_unit_for_test(&mut session, loot_unit_packet(owner_guid)).await;
    let _response = send_rx.try_recv().unwrap();
    let _loot_list = send_rx.try_recv().unwrap();
    let _start_roll = send_rx.try_recv().unwrap();
    let _remote_loot_list = candidate_rx.try_recv().unwrap();
    let _remote_start_roll = candidate_rx.try_recv().unwrap();
    handle_loot_roll_for_test(&mut session, LootRoll {
            loot_obj: loot_object,
            loot_list_id: 0,
            roll_type: ROLL_VOTE_NEED_LIKE_CPP,
        })
        .await;

    let local_roll = send_rx.try_recv().unwrap();
    let mut local_roll = WorldPacket::from_bytes(&local_roll);
    assert_eq!(
        local_roll.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::LootRoll as u16
    );
    assert_eq!(local_roll.read_packed_guid().unwrap(), loot_object);
    assert_eq!(local_roll.read_packed_guid().unwrap(), player_guid);
    assert_eq!(local_roll.read_int32().unwrap(), 0);
    assert_eq!(local_roll.read_uint8().unwrap(), ROLL_VOTE_NEED_LIKE_CPP);
    assert_eq!(local_roll.read_int32().unwrap(), 0);
    assert_eq!(local_roll.read_bits(2).unwrap(), 0);
    assert_eq!(local_roll.read_bits(3).unwrap(), 1);

    let remote_roll = candidate_rx.try_recv().unwrap();
    let mut remote_roll = WorldPacket::from_bytes(&remote_roll);
    assert_eq!(
        remote_roll.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::LootRoll as u16
    );
    assert_eq!(remote_roll.read_packed_guid().unwrap(), loot_object);
    assert_eq!(remote_roll.read_packed_guid().unwrap(), player_guid);
}
