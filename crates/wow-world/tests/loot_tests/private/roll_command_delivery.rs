//! Original remote and mailbox vote routing scenarios.
use super::recovery_support::*;
use std::sync::Mutex;
use wow_entities::Player;
use wow_loot::{
    LOOT_METHOD_GROUP_LIKE_CPP, ROLL_VOTE_GREED_LIKE_CPP, ROLL_VOTE_NEED_LIKE_CPP,
    ROLL_VOTE_NOT_EMITTED_YET_LIKE_CPP, ROLL_VOTE_NOT_VALID_LIKE_CPP,
};
use wow_world::session::mailbox::LootRollVoteCommand;
use wow_world::test_fixtures::loot::{
    allocate_loot_guid_for_test, loot_roll_observation_for_test, process_loot_commands_for_test,
    sync_creature_loot_fixture_for_test,
};

fn generation_guarded_group_loot_like_cpp(
    owner_guid: ObjectGuid,
    player_guid: ObjectGuid,
    candidate_guid: ObjectGuid,
) -> CreatureLoot {
    CreatureLoot {
        loot_guid: represented_loot_object_guid_like_cpp(owner_guid),
        coins: 0,
        unlooted_count: 1,
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
    }
}

#[tokio::test]
async fn loot_roll_remote_session_routes_vote_to_owner_session_like_cpp() {
    let (mut owner_session, owner_rx) = make_session_with_send_capacity(8);
    let (mut remote_session, _remote_rx) = make_session_with_send_capacity(2);
    let player_guid = ObjectGuid::create_player(1, 42);
    let candidate_guid = ObjectGuid::create_player(1, 77);
    let owner_guid = test_creature_guid(19_056);
    let (candidate_tx, candidate_rx) = flume::bounded::<Vec<u8>>(8);
    let (owner_registry_tx, _owner_registry_rx) = flume::bounded::<Vec<u8>>(8);
    let player_registry = Arc::new(PlayerRegistry::default());

    let mut owner_info = broadcast_info(player_guid, owner_registry_tx);
    owner_info.command_tx = owner_session.session_command_tx();
    player_registry.register_or_replace(player_guid, owner_info, Default::default());
    player_registry.register_or_replace(
        candidate_guid,
        broadcast_info(candidate_guid, candidate_tx),
        Default::default(),
    );

    owner_session.set_player_registry(Arc::clone(&player_registry));
    owner_session.set_player_guid(Some(player_guid));
    remote_session.set_player_registry(Arc::clone(&player_registry));
    remote_session.set_player_guid(Some(candidate_guid));
    install_group_loot_group(&mut owner_session, player_guid, candidate_guid);

    let mut canonical_player = Player::new(Some(1), false);
    canonical_player
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(player_guid);
    canonical_player
        .unit_mut()
        .world_mut()
        .set_map(u32::from(owner_guid.map_id()), 0)
        .unwrap();
    canonical_player
        .unit_mut()
        .world_mut()
        .relocate(Position::ZERO);
    canonical_player
        .unit_mut()
        .world_mut()
        .object_mut()
        .add_to_world();
    let mut canonical_candidate = Player::new(Some(1), false);
    canonical_candidate
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(candidate_guid);
    canonical_candidate
        .unit_mut()
        .world_mut()
        .set_map(u32::from(owner_guid.map_id()), 0)
        .unwrap();
    canonical_candidate
        .unit_mut()
        .world_mut()
        .relocate(Position::ZERO);
    canonical_candidate
        .unit_mut()
        .world_mut()
        .object_mut()
        .add_to_world();
    let canonical_creature = make_canonical_creature_for_session(&owner_session, owner_guid);
    let canonical_manager = Arc::new(Mutex::new(wow_map::MapManager::default()));
    {
        let mut manager = canonical_manager.lock().unwrap();
        let map = manager.create_world_map(u32::from(owner_guid.map_id()), 0);
        map.map_mut()
            .insert_map_object_record(
                wow_entities::MapObjectRecord::new_player(canonical_player).unwrap(),
            )
            .unwrap();
        map.map_mut()
            .insert_map_object_record(
                wow_entities::MapObjectRecord::new_player(canonical_candidate).unwrap(),
            )
            .unwrap();
        map.map_mut()
            .insert_map_object_record(
                wow_entities::MapObjectRecord::new_creature(canonical_creature).unwrap(),
            )
            .unwrap();
    }
    assert!(player_registry.bind_canonical_map_manager(Arc::clone(&canonical_manager)));
    owner_session.set_canonical_map_manager(Arc::clone(&canonical_manager));
    remote_session.set_canonical_map_manager(canonical_manager);
    let loot_object = allocate_loot_guid_for_test(&mut owner_session, owner_guid)
        .expect("the canonical owner map must allocate the C++ LootObject identity");

    register_test_creature_like_cpp(&mut owner_session, test_creature(owner_guid, false));
    let mut loot = generation_guarded_group_loot_like_cpp(owner_guid, player_guid, candidate_guid);
    loot.loot_guid = loot_object;
    set_loot_for_test(&mut owner_session, owner_guid, loot);
    sync_creature_loot_fixture_for_test(&mut owner_session, owner_guid, player_guid)
        .expect("the fixture loot must be installed into the object-owned authority");
    let installed = loot_recovery_authority_for_test(&mut owner_session, owner_guid)
        .and_then(|authority| authority.shared_snapshot_like_cpp())
        .expect("the canonical creature must expose the installed shared loot");
    assert_eq!(installed.loot.loot_guid, loot_object);

    handle_loot_unit_for_test(&mut owner_session, loot_unit_packet(owner_guid)).await;
    let _response = owner_rx.try_recv().unwrap();
    let _loot_list = owner_rx.try_recv().unwrap();
    let _start_roll = owner_rx.try_recv().unwrap();
    let _remote_loot_list = candidate_rx.try_recv().unwrap();
    let _remote_start_roll = candidate_rx.try_recv().unwrap();

    assert!(
        player_registry
            .fixture_active_loot_rolls(player_guid)
            .unwrap()
            .iter()
            .any(|identity| identity.matches_key_like_cpp(loot_object, 0))
    );

    handle_loot_roll_for_test(
        &mut remote_session,
        LootRoll {
            loot_obj: loot_object,
            loot_list_id: 0,
            roll_type: ROLL_VOTE_GREED_LIKE_CPP,
        },
    )
    .await;
    process_loot_commands_for_test(&mut owner_session).await;

    let local_roll = owner_rx.try_recv().unwrap();
    let mut local_roll = WorldPacket::from_bytes(&local_roll);
    assert_eq!(
        local_roll.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::LootRoll as u16
    );
    assert_eq!(local_roll.read_packed_guid().unwrap(), loot_object);
    assert_eq!(local_roll.read_packed_guid().unwrap(), candidate_guid);
    assert_eq!(local_roll.read_int32().unwrap(), -1);
    assert_eq!(local_roll.read_uint8().unwrap(), ROLL_VOTE_GREED_LIKE_CPP);

    let remote_roll = candidate_rx.try_recv().unwrap();
    let mut remote_roll = WorldPacket::from_bytes(&remote_roll);
    assert_eq!(
        remote_roll.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::LootRoll as u16
    );
    assert_eq!(remote_roll.read_packed_guid().unwrap(), loot_object);
    assert_eq!(remote_roll.read_packed_guid().unwrap(), candidate_guid);
}

#[tokio::test]
async fn loot_roll_vote_command_updates_owner_session_roll_state_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(8);
    let player_guid = ObjectGuid::create_player(1, 42);
    let candidate_guid = ObjectGuid::create_player(1, 77);
    let owner_guid = test_creature_guid(19_055);
    let loot_object = represented_loot_object_guid_like_cpp(owner_guid);
    let (candidate_tx, candidate_rx) = flume::bounded::<Vec<u8>>(8);
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
    set_loot_for_test(
        &mut session,
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
    let roll_identity = loot_roll_observation_for_test(&session, loot_object, 0)
        .unwrap()
        .command_identity()
        .clone();

    session
        .session_command_tx()
        .send(SessionCommand::LootRollVote(LootRollVoteCommand {
            voter_guid: candidate_guid,
            loot_obj: loot_object,
            loot_list_id: 0,
            roll_type: ROLL_VOTE_GREED_LIKE_CPP,
            pass_on_group_loot: false,
            roll_identity,
        }))
        .unwrap();
    process_loot_commands_for_test(&mut session).await;

    let local_roll = send_rx.try_recv().unwrap();
    let mut local_roll = WorldPacket::from_bytes(&local_roll);
    assert_eq!(
        local_roll.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::LootRoll as u16
    );
    assert_eq!(local_roll.read_packed_guid().unwrap(), loot_object);
    assert_eq!(local_roll.read_packed_guid().unwrap(), candidate_guid);
    assert_eq!(local_roll.read_int32().unwrap(), -1);
    assert_eq!(local_roll.read_uint8().unwrap(), ROLL_VOTE_GREED_LIKE_CPP);

    let remote_roll = candidate_rx.try_recv().unwrap();
    let mut remote_roll = WorldPacket::from_bytes(&remote_roll);
    assert_eq!(
        remote_roll.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::LootRoll as u16
    );
    assert_eq!(remote_roll.read_packed_guid().unwrap(), loot_object);
    assert_eq!(remote_roll.read_packed_guid().unwrap(), candidate_guid);

    let state = loot_roll_observation_for_test(&session, loot_object, 0).unwrap();
    assert_eq!(
        state.vote(candidate_guid).unwrap().vote,
        ROLL_VOTE_GREED_LIKE_CPP
    );
}
