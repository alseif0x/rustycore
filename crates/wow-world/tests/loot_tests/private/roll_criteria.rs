//! Original criterion callback, winner wire and stale-roll cases.
use super::roll_criteria_support::*;

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

    handle_loot_roll_for_test(
        &mut session,
        LootRoll {
            loot_obj: loot_object,
            loot_list_id: 0,
            roll_type: ROLL_VOTE_NEED_LIKE_CPP,
        },
    )
    .await;
    let _local_need_roll = send_rx.try_recv().unwrap();
    let _remote_need_roll = candidate_rx.try_recv().unwrap();

    session.set_player_guid(Some(candidate_guid));
    handle_loot_roll_for_test(
        &mut session,
        LootRoll {
            loot_obj: loot_object,
            loot_list_id: 0,
            roll_type: ROLL_VOTE_GREED_LIKE_CPP,
        },
    )
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

    let entry = &loot_for_test(&session, owner_guid).unwrap().items[0];
    assert!(!entry.flags.blocked);
    assert_eq!(entry.roll_winner, player_guid);
    assert!(!loot_roll_observation_for_test(&session, loot_object, 0).is_some());
    assert_eq!(
        loot_criterion_for_test(&session, 0),
        LootCriterionExpectation::any_need(player_guid, 1)
    );
    assert_eq!(
        loot_criterion_for_test(&session, 1),
        LootCriterionExpectation::any_greed(candidate_guid, 1)
    );
    match loot_criterion_for_test(&session, 2).need() {
        Some((criteria_player, item_id, roll_number)) => {
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

    handle_loot_roll_for_test(
        &mut session,
        LootRoll {
            loot_obj: loot_object,
            loot_list_id: 0,
            roll_type: ROLL_VOTE_GREED_LIKE_CPP,
        },
    )
    .await;
    let _local_greed_roll = send_rx.try_recv().unwrap();
    let _remote_greed_roll = candidate_rx.try_recv().unwrap();

    set_loot_roll_deadline_for_test(
        &mut session,
        loot_object,
        0,
        Instant::now() - Duration::from_millis(1),
    );
    tick_loot_rolls_for_test(&mut session).await;

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

    let entry = &loot_for_test(&session, owner_guid).unwrap().items[0];
    assert!(!entry.flags.blocked);
    assert_eq!(entry.roll_winner, player_guid);
    assert!(!loot_roll_observation_for_test(&session, loot_object, 0).is_some());
    assert_eq!(
        loot_criterion_for_test(&session, 0),
        LootCriterionExpectation::any_greed(player_guid, 1)
    );
    match loot_criterion_for_test(&session, 1).greed() {
        Some((criteria_player, item_id, roll_number)) => {
            assert_eq!(criteria_player, player_guid);
            assert_eq!(item_id, 25);
            assert!((1..=100).contains(&roll_number));
        }
        other => panic!("unexpected criteria event: {other:?}"),
    }
}

#[tokio::test]
async fn stale_loot_roll_expiry_does_not_mutate_replacement_generation_like_cpp() {
    let (mut session, send_rx, candidate_rx, player_guid, candidate_guid, owner_guid) =
        open_generation_guarded_group_roll_like_cpp(19_061).await;
    let loot_object = represented_loot_object_guid_like_cpp(owner_guid);
    let old_generation = loot_roll_observation_for_test(&session, loot_object, 0)
        .unwrap()
        .authority_generation();
    let replacement_generation = replace_generation_guarded_group_loot_like_cpp(
        &mut session,
        owner_guid,
        player_guid,
        candidate_guid,
    );
    assert_ne!(old_generation, replacement_generation);
    set_loot_roll_deadline_for_test(
        &mut session,
        loot_object,
        0,
        Instant::now() - Duration::from_millis(1),
    );

    tick_loot_rolls_for_test(&mut session).await;

    assert!(
        !loot_roll_observation_for_test(&session, loot_object, 0).is_some(),
        "the stale timer must be cancelled without finishing against replacement loot"
    );
    assert!(send_rx.try_recv().is_err());
    assert!(candidate_rx.try_recv().is_err());
    assert!(loot_criteria_empty_for_test(&session));

    let authority = loot_recovery_authority_for_test(&mut session, owner_guid).unwrap();
    let replacement = authority.shared_snapshot_like_cpp().unwrap();
    assert_eq!(replacement.generation, replacement_generation);
    let entry = &replacement.loot.items[0];
    assert!(entry.flags.blocked);
    assert!(entry.roll_winner.is_empty());
    assert!(!entry.taken);
    assert_eq!(replacement.loot.unlooted_count, 1);
}

#[tokio::test]
async fn stale_loot_roll_vote_does_not_mutate_replacement_generation_like_cpp() {
    let (mut session, send_rx, candidate_rx, player_guid, candidate_guid, owner_guid) =
        open_generation_guarded_group_roll_like_cpp(19_060).await;
    let loot_object = represented_loot_object_guid_like_cpp(owner_guid);
    let old_generation = loot_roll_observation_for_test(&session, loot_object, 0)
        .unwrap()
        .authority_generation();
    let replacement_generation = replace_generation_guarded_group_loot_like_cpp(
        &mut session,
        owner_guid,
        player_guid,
        candidate_guid,
    );
    assert_ne!(old_generation, replacement_generation);

    handle_loot_roll_for_test(
        &mut session,
        LootRoll {
            loot_obj: loot_object,
            loot_list_id: 0,
            roll_type: ROLL_VOTE_NEED_LIKE_CPP,
        },
    )
    .await;

    assert!(
        !loot_roll_observation_for_test(&session, loot_object, 0).is_some(),
        "the stale roll must be cancelled instead of routed or voted"
    );
    assert!(send_rx.try_recv().is_err());
    assert!(candidate_rx.try_recv().is_err());
    assert!(loot_criteria_empty_for_test(&session));

    let authority = loot_recovery_authority_for_test(&mut session, owner_guid).unwrap();
    let replacement = authority.shared_snapshot_like_cpp().unwrap();
    assert_eq!(replacement.generation, replacement_generation);
    let entry = &replacement.loot.items[0];
    assert!(entry.flags.blocked);
    assert!(entry.roll_winner.is_empty());
    assert!(!entry.taken);
    assert_eq!(replacement.loot.unlooted_count, 1);
}
