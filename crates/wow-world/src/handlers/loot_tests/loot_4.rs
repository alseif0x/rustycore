//! Loot scenarios for [`super`].
//!
//! Split out of loot_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

#[tokio::test]
async fn loot_roll_all_voted_finishes_need_winner_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(8);
    let player_guid = ObjectGuid::create_player(1, 42);
    let candidate_guid = ObjectGuid::create_player(1, 77);
    let owner_guid = test_creature_guid(19_053);
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
    session.handle_loot_unit(loot_unit_packet(owner_guid)).await;
    let _response = send_rx.try_recv().unwrap();
    let _loot_list = send_rx.try_recv().unwrap();
    let _start_roll = send_rx.try_recv().unwrap();
    let _remote_loot_list = candidate_rx.try_recv().unwrap();
    let _remote_start_roll = candidate_rx.try_recv().unwrap();

    session
        .handle_loot_roll(LootRoll {
            loot_obj: loot_object,
            loot_list_id: 0,
            roll_type: ROLL_VOTE_NEED_LIKE_CPP,
        })
        .await;
    let _local_need_roll = send_rx.try_recv().unwrap();
    let _remote_need_roll = candidate_rx.try_recv().unwrap();

    session.set_player_guid(Some(candidate_guid));
    session
        .handle_loot_roll(LootRoll {
            loot_obj: loot_object,
            loot_list_id: 0,
            roll_type: ROLL_VOTE_GREED_LIKE_CPP,
        })
        .await;

    let local_greed_roll = send_rx.try_recv().unwrap();
    let mut local_greed_roll = WorldPacket::from_bytes(&local_greed_roll);
    assert_eq!(
        local_greed_roll.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::LootRoll as u16
    );
    assert_eq!(local_greed_roll.read_packed_guid().unwrap(), loot_object);
    assert_eq!(local_greed_roll.read_packed_guid().unwrap(), candidate_guid);

    let mut local_won_locked =
        recv_packet_with_opcode(&send_rx, wow_constants::ServerOpcodes::LootRollWon);
    assert_eq!(local_won_locked.read_packed_guid().unwrap(), loot_object);
    assert_eq!(local_won_locked.read_packed_guid().unwrap(), player_guid);
    let winner_roll = local_won_locked.read_int32().unwrap();
    assert!((1..=100).contains(&winner_roll));
    assert_eq!(
        local_won_locked.read_uint8().unwrap(),
        ROLL_VOTE_NEED_LIKE_CPP
    );
    assert_eq!(local_won_locked.read_int32().unwrap(), 0);
    assert_eq!(local_won_locked.read_bits(2).unwrap(), 0);
    assert_eq!(local_won_locked.read_bits(3).unwrap(), 2);

    let mut original_greed_roll =
        recv_packet_with_opcode(&player_rx, wow_constants::ServerOpcodes::LootRoll);
    assert_eq!(original_greed_roll.read_packed_guid().unwrap(), loot_object);
    assert_eq!(
        original_greed_roll.read_packed_guid().unwrap(),
        candidate_guid
    );

    let final_replay_to_winner =
        recv_packet_with_opcode(&player_rx, wow_constants::ServerOpcodes::LootRoll);
    let mut final_replay_to_winner = final_replay_to_winner;
    assert!(matches!(
        final_replay_to_winner.read_packed_guid().unwrap(),
        guid if guid == loot_object
    ));
    let _replay_player = final_replay_to_winner.read_packed_guid().unwrap();
    let replay_roll = final_replay_to_winner.read_int32().unwrap();
    assert!((0..=100).contains(&replay_roll));

    let mut original_won_allow =
        recv_packet_with_opcode(&player_rx, wow_constants::ServerOpcodes::LootRollWon);
    assert_eq!(original_won_allow.read_packed_guid().unwrap(), loot_object);
    assert_eq!(original_won_allow.read_packed_guid().unwrap(), player_guid);
    let _roll = original_won_allow.read_int32().unwrap();
    assert_eq!(
        original_won_allow.read_uint8().unwrap(),
        ROLL_VOTE_NEED_LIKE_CPP
    );
    assert_eq!(original_won_allow.read_int32().unwrap(), 0);
    assert_eq!(original_won_allow.read_bits(2).unwrap(), 0);
    assert_eq!(original_won_allow.read_bits(3).unwrap(), 0);

    let entry = &session.loot_table.get(&owner_guid).unwrap().items[0];
    assert!(!entry.flags.blocked);
    assert_eq!(entry.roll_winner, player_guid);
    assert!(
        !session
            .represented_loot_rolls
            .contains_key(&(loot_object, 0))
    );
    assert_eq!(
        session.represented_loot_roll_criteria_events[0],
        RepresentedLootRollCriteriaEvent::RollAnyNeed {
            player_guid,
            quantity: 1
        }
    );
    assert_eq!(
        session.represented_loot_roll_criteria_events[1],
        RepresentedLootRollCriteriaEvent::RollAnyGreed {
            player_guid: candidate_guid,
            quantity: 1
        }
    );
    match session.represented_loot_roll_criteria_events[2] {
        RepresentedLootRollCriteriaEvent::RollNeed {
            player_guid: criteria_player,
            item_id,
            roll_number,
        } => {
            assert_eq!(criteria_player, player_guid);
            assert_eq!(item_id, 25);
            assert!((1..=100).contains(&roll_number));
        }
        other => panic!("unexpected criteria event: {other:?}"),
    }
}
#[tokio::test]
async fn loot_roll_timer_expiry_finishes_current_winner_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(8);
    let player_guid = ObjectGuid::create_player(1, 42);
    let candidate_guid = ObjectGuid::create_player(1, 77);
    let owner_guid = test_creature_guid(19_057);
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
    session.handle_loot_unit(loot_unit_packet(owner_guid)).await;
    let _response = send_rx.try_recv().unwrap();
    let _loot_list = send_rx.try_recv().unwrap();
    let _start_roll = send_rx.try_recv().unwrap();
    let _remote_loot_list = candidate_rx.try_recv().unwrap();
    let _remote_start_roll = candidate_rx.try_recv().unwrap();

    session
        .handle_loot_roll(LootRoll {
            loot_obj: loot_object,
            loot_list_id: 0,
            roll_type: ROLL_VOTE_GREED_LIKE_CPP,
        })
        .await;
    let _local_greed_roll = send_rx.try_recv().unwrap();
    let _remote_greed_roll = candidate_rx.try_recv().unwrap();

    session
        .represented_loot_rolls
        .get_mut(&(loot_object, 0))
        .unwrap()
        .end_time = Instant::now() - Duration::from_millis(1);
    session.tick_represented_loot_rolls_like_cpp().await;

    let mut local_final_replay =
        recv_packet_with_opcode(&send_rx, wow_constants::ServerOpcodes::LootRoll);
    assert_eq!(local_final_replay.read_packed_guid().unwrap(), loot_object);
    let _replay_player = local_final_replay.read_packed_guid().unwrap();

    let mut local_won_allow =
        recv_packet_with_opcode(&send_rx, wow_constants::ServerOpcodes::LootRollWon);
    assert_eq!(local_won_allow.read_packed_guid().unwrap(), loot_object);
    assert_eq!(local_won_allow.read_packed_guid().unwrap(), player_guid);
    assert!((1..=100).contains(&local_won_allow.read_int32().unwrap()));
    assert_eq!(
        local_won_allow.read_uint8().unwrap(),
        ROLL_VOTE_GREED_LIKE_CPP
    );

    let mut remote_final_replay =
        recv_packet_with_opcode(&candidate_rx, wow_constants::ServerOpcodes::LootRoll);
    assert_eq!(remote_final_replay.read_packed_guid().unwrap(), loot_object);
    let _remote_replay_player = remote_final_replay.read_packed_guid().unwrap();

    let mut remote_won_locked =
        recv_packet_with_opcode(&candidate_rx, wow_constants::ServerOpcodes::LootRollWon);
    assert_eq!(remote_won_locked.read_packed_guid().unwrap(), loot_object);
    assert_eq!(remote_won_locked.read_packed_guid().unwrap(), player_guid);
    assert!((1..=100).contains(&remote_won_locked.read_int32().unwrap()));
    assert_eq!(
        remote_won_locked.read_uint8().unwrap(),
        ROLL_VOTE_GREED_LIKE_CPP
    );

    let entry = &session.loot_table.get(&owner_guid).unwrap().items[0];
    assert!(!entry.flags.blocked);
    assert_eq!(entry.roll_winner, player_guid);
    assert!(
        !session
            .represented_loot_rolls
            .contains_key(&(loot_object, 0))
    );
    assert_eq!(
        session.represented_loot_roll_criteria_events[0],
        RepresentedLootRollCriteriaEvent::RollAnyGreed {
            player_guid,
            quantity: 1
        }
    );
    match session.represented_loot_roll_criteria_events[1] {
        RepresentedLootRollCriteriaEvent::RollGreed {
            player_guid: criteria_player,
            item_id,
            roll_number,
        } => {
            assert_eq!(criteria_player, player_guid);
            assert_eq!(item_id, 25);
            assert!((1..=100).contains(&roll_number));
        }
        other => panic!("unexpected criteria event: {other:?}"),
    }
}
#[tokio::test]
async fn stale_loot_roll_vote_does_not_mutate_replacement_generation_like_cpp() {
    let (mut session, send_rx, candidate_rx, player_guid, candidate_guid, owner_guid) =
        open_generation_guarded_group_roll_like_cpp(19_060).await;
    let loot_object = represented_loot_object_guid_like_cpp(owner_guid);
    let old_generation = session
        .represented_loot_rolls
        .get(&(loot_object, 0))
        .unwrap()
        .authority_generation;
    let replacement_generation = replace_generation_guarded_group_loot_like_cpp(
        &mut session,
        owner_guid,
        player_guid,
        candidate_guid,
    );
    assert_ne!(old_generation, replacement_generation);

    session
        .handle_loot_roll(LootRoll {
            loot_obj: loot_object,
            loot_list_id: 0,
            roll_type: ROLL_VOTE_NEED_LIKE_CPP,
        })
        .await;

    assert!(
        !session
            .represented_loot_rolls
            .contains_key(&(loot_object, 0)),
        "the stale roll must be cancelled instead of routed or voted"
    );
    assert!(send_rx.try_recv().is_err());
    assert!(candidate_rx.try_recv().is_err());
    assert!(session.represented_loot_roll_criteria_events.is_empty());

    let authority = session
        .represented_owned_loot_authority_like_cpp(owner_guid)
        .unwrap();
    let replacement = authority.shared_snapshot_like_cpp().unwrap();
    assert_eq!(replacement.generation, replacement_generation);
    let entry = &replacement.loot.items[0];
    assert!(entry.flags.blocked);
    assert!(entry.roll_winner.is_empty());
    assert!(!entry.taken);
    assert_eq!(replacement.loot.unlooted_count, 1);
}
#[tokio::test]
async fn stale_loot_roll_expiry_does_not_mutate_replacement_generation_like_cpp() {
    let (mut session, send_rx, candidate_rx, player_guid, candidate_guid, owner_guid) =
        open_generation_guarded_group_roll_like_cpp(19_061).await;
    let loot_object = represented_loot_object_guid_like_cpp(owner_guid);
    let old_generation = session
        .represented_loot_rolls
        .get(&(loot_object, 0))
        .unwrap()
        .authority_generation;
    let replacement_generation = replace_generation_guarded_group_loot_like_cpp(
        &mut session,
        owner_guid,
        player_guid,
        candidate_guid,
    );
    assert_ne!(old_generation, replacement_generation);
    session
        .represented_loot_rolls
        .get_mut(&(loot_object, 0))
        .unwrap()
        .end_time = Instant::now() - Duration::from_millis(1);

    session.tick_represented_loot_rolls_like_cpp().await;

    assert!(
        !session
            .represented_loot_rolls
            .contains_key(&(loot_object, 0)),
        "the stale timer must be cancelled without finishing against replacement loot"
    );
    assert!(send_rx.try_recv().is_err());
    assert!(candidate_rx.try_recv().is_err());
    assert!(session.represented_loot_roll_criteria_events.is_empty());

    let authority = session
        .represented_owned_loot_authority_like_cpp(owner_guid)
        .unwrap();
    let replacement = authority.shared_snapshot_like_cpp().unwrap();
    assert_eq!(replacement.generation, replacement_generation);
    let entry = &replacement.loot.items[0];
    assert!(entry.flags.blocked);
    assert!(entry.roll_winner.is_empty());
    assert!(!entry.taken);
    assert_eq!(replacement.loot.unlooted_count, 1);
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
    session.handle_loot_unit(loot_unit_packet(owner_guid)).await;
    let _response = send_rx.try_recv().unwrap();
    let _loot_list = send_rx.try_recv().unwrap();
    let _start_roll = send_rx.try_recv().unwrap();
    let _remote_loot_list = candidate_rx.try_recv().unwrap();
    let _remote_start_roll = candidate_rx.try_recv().unwrap();

    session
        .handle_loot_roll(LootRoll {
            loot_obj: loot_object,
            loot_list_id: 0,
            roll_type: ROLL_VOTE_PASS_LIKE_CPP,
        })
        .await;
    let _local_pass_roll = send_rx.try_recv().unwrap();
    let _remote_pass_roll = candidate_rx.try_recv().unwrap();
    let pass_state = session
        .represented_loot_rolls
        .get(&(loot_object, 0))
        .expect("roll state should stay open until every voter passes");
    assert_eq!(
        pass_state.voters.get(&player_guid).unwrap().roll_number,
        0,
        "C++ LootRoll::PlayerVote does not call urand for Pass"
    );

    session.set_player_guid(Some(candidate_guid));
    session
        .handle_loot_roll(LootRoll {
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

    let entry = &session.loot_table.get(&owner_guid).unwrap().items[0];
    assert!(!entry.flags.blocked);
    assert!(entry.roll_winner.is_empty());
    assert!(
        !session
            .represented_loot_rolls
            .contains_key(&(loot_object, 0))
    );
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
    session.handle_loot_unit(loot_unit_packet(owner_guid)).await;
    let _response = send_rx.try_recv().unwrap();
    let _loot_list = send_rx.try_recv().unwrap();
    let _start_roll = send_rx.try_recv().unwrap();
    let _remote_loot_list = candidate_rx.try_recv().unwrap();
    let _remote_start_roll = candidate_rx.try_recv().unwrap();
    let roll_identity = session
        .represented_loot_rolls
        .get(&(loot_object, 0))
        .unwrap()
        .command_identity
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
    session
        .process_represented_session_commands_like_cpp()
        .await;

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

    let state = session
        .represented_loot_rolls
        .get(&(loot_object, 0))
        .unwrap();
    assert_eq!(
        state.voters.get(&candidate_guid).unwrap().vote,
        ROLL_VOTE_GREED_LIKE_CPP
    );
}
#[test]
fn loot_roll_vote_command_accepts_exact_enqueued_roll_identity_like_cpp() {
    let loot_object = represented_loot_object_guid_like_cpp(test_creature_guid(19_062));
    let authority = OwnedLootAuthority::new();
    let roll_identity = LootRollCommandIdentityLikeCpp::new_like_cpp(loot_object, 0, authority, 7);
    let command = LootRollVoteCommand {
        voter_guid: ObjectGuid::create_player(1, 77),
        loot_obj: loot_object,
        loot_list_id: 0,
        roll_type: ROLL_VOTE_GREED_LIKE_CPP,
        pass_on_group_loot: false,
        roll_identity: roll_identity.clone(),
    };

    assert!(
        WorldSession::represented_loot_roll_vote_command_targets_identity_like_cpp(
            &command,
            &roll_identity,
        )
    );
}
#[test]
fn queued_loot_roll_vote_rejects_replacement_with_same_key_and_generation_like_cpp() {
    let loot_object = represented_loot_object_guid_like_cpp(test_creature_guid(19_063));
    let authority = OwnedLootAuthority::new();
    let stale_identity =
        LootRollCommandIdentityLikeCpp::new_like_cpp(loot_object, 0, authority.clone(), 7);
    let replacement_identity =
        LootRollCommandIdentityLikeCpp::new_like_cpp(loot_object, 0, authority, 7);
    let stale_command = LootRollVoteCommand {
        voter_guid: ObjectGuid::create_player(1, 77),
        loot_obj: loot_object,
        loot_list_id: 0,
        roll_type: ROLL_VOTE_GREED_LIKE_CPP,
        pass_on_group_loot: false,
        roll_identity: stale_identity,
    };

    assert!(
        !WorldSession::represented_loot_roll_vote_command_targets_identity_like_cpp(
            &stale_command,
            &replacement_identity,
        ),
        "a command queued for the destroyed C++ LootRoll* must not vote on its replacement"
    );
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
    let loot_object = owner_session
        .next_represented_loot_object_guid_like_cpp(owner_guid)
        .expect("the canonical owner map must allocate the C++ LootObject identity");

    register_test_creature_like_cpp(&mut owner_session, test_creature(owner_guid, false));
    let mut loot = generation_guarded_group_loot_like_cpp(owner_guid, player_guid, candidate_guid);
    loot.loot_guid = loot_object;
    owner_session.loot_table.insert(owner_guid, loot);
    owner_session
        .sync_represented_creature_loot_to_canonical_like_cpp(owner_guid, player_guid)
        .expect("the fixture loot must be installed into the object-owned authority");
    let installed = owner_session
        .represented_owned_loot_authority_like_cpp(owner_guid)
        .and_then(|authority| authority.shared_snapshot_like_cpp())
        .expect("the canonical creature must expose the installed shared loot");
    assert_eq!(installed.loot.loot_guid, loot_object);

    owner_session
        .handle_loot_unit(loot_unit_packet(owner_guid))
        .await;
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

    remote_session
        .handle_loot_roll(LootRoll {
            loot_obj: loot_object,
            loot_list_id: 0,
            roll_type: ROLL_VOTE_GREED_LIKE_CPP,
        })
        .await;
    owner_session
        .process_represented_session_commands_like_cpp()
        .await;

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
