//! Session scenarios exercising the represented movement responsibility.
//!
//! Split out of session_tests.rs under #626; assertions and registrations
//! are unchanged and the shared fixtures stay in the parent module.

use super::*;

#[tokio::test]
async fn teleport_to_instance_rejects_new_instance_farm_limit_before_transfer_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 795);
    let destination = Position::new(5795.0, 2095.0, 640.0, 3.5);

    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "TeleportInstanceCap".to_string(),
        Position::new(3700.0, 1500.0, 120.0, 0.0),
        571,
        1,
        1,
        80,
        0,
    ));
    session.represented_raid_difficulty_id_like_cpp = 3;
    session.set_max_instances_per_hour_like_cpp(5);
    install_create_map_active_lock_stores_like_cpp(&mut session, 631, 3, 77, 2);
    for instance_id in 100..105 {
        session
            .represented_instance_reset_times_like_cpp
            .insert(instance_id, u64::MAX);
    }

    session.teleport_to(631, destination).await;

    assert_eq!(
        send_rx.try_recv().expect("too many instances abort"),
        wow_packet::packets::misc::TransferAborted {
            map_id: 631,
            arg: 0,
            map_difficulty_x_condition_id: 0,
            transfer_abort: TRANSFER_ABORT_TOO_MANY_INSTANCES_LIKE_CPP,
        }
        .to_bytes()
    );
    assert!(
        send_rx.try_recv().is_err(),
        "C++ Player::TeleportTo returns before SMSG_TRANSFER_PENDING when Map::PlayerCannotEnter rejects the instance cap"
    );
    assert_eq!(session.pending_teleport_like_cpp(), None);
    assert_ne!(session.state, SessionState::Transfer);
    assert!(
        canonical.lock().unwrap().find_map(631, 0).is_none(),
        "teleport preflight must not create the target instance before the client transfer"
    );
}
#[tokio::test]
async fn teleport_to_instance_allows_transfer_after_player_cannot_enter_passes_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let legacy_manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    let player_registry = Arc::new(PlayerRegistry::default());
    let player_guid = ObjectGuid::create_player(1, 791);
    let selected_guid = test_creature_guid(79_101);
    let destination = Position::new(5791.0, 2091.0, 637.0, 3.2);

    session.set_map_manager(Arc::clone(&legacy_manager));
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_player_registry(Arc::clone(&player_registry));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "TeleportAccessAllow".to_string(),
        Position::new(3700.0, 1500.0, 120.0, 0.0),
        571,
        1,
        1,
        80,
        0,
    ));
    insert_session_player_into_canonical_map_like_cpp(&session, &canonical, 571, 0);
    session
        .mutate_canonical_player_like_cpp(|player| {
            player.set_player_flag(PLAYER_FLAGS_CONTESTED_PVP_LIKE_CPP);
            player
                .unit_mut()
                .add_unit_state(UnitState::ATTACK_PLAYER.bits());
        })
        .unwrap();
    session.player_contested_pvp_timer_like_cpp = 77;
    session.register_in_player_registry();
    session.set_selection_guid_like_cpp(Some(selected_guid));
    session.register_world_creature(
        571,
        Position::new(3701.0, 1500.0, 120.0, 0.0),
        test_creature_create_data(selected_guid, 9_101, 80),
        3,
        5,
        20.0,
        0,
        0,
        0,
        0,
        None,
        0,
        0,
        0,
        0,
        -1,
    );
    session.start_player_attack_like_cpp(selected_guid);
    session
        .mutate_world_creature(selected_guid, |creature| {
            creature.enter_combat(player_guid);
            creature
                .creature
                .unit_mut()
                .subsystems_mut()
                .combat
                .add_threat(player_guid, 25.0);
        })
        .unwrap();
    assert!(session.in_combat);
    session.represented_raid_difficulty_id_like_cpp = 3;
    install_create_map_active_lock_stores_like_cpp(&mut session, 631, 3, 77, 2);

    session.teleport_to(631, destination).await;

    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![
            ServerOpcodes::AttackStop,
            ServerOpcodes::CancelCombat,
            ServerOpcodes::TransferPending,
            ServerOpcodes::SuspendToken
        ]
    );
    assert_eq!(
        session.pending_teleport_like_cpp(),
        Some((631, destination))
    );
    assert_eq!(session.state, SessionState::Transfer);
    assert_eq!(
        session.selection_guid_like_cpp(),
        None,
        "C++ far Player::TeleportTo clears selection after entry preflight and before transfer"
    );
    assert_eq!(
        session.player_contested_pvp_timer_like_cpp, 0,
        "C++ far Player::TeleportTo calls ResetContestedPvP before transfer"
    );
    {
        let manager = canonical.lock().unwrap();
        let old_map = manager.find_map(571, 0).unwrap().map();
        assert!(
            old_map.get_typed_player(player_guid).is_none(),
            "C++ far Player::TeleportTo removes the player from oldmap before storing the far teleport destination"
        );
        let creature = old_map
            .with_creature_like_cpp(selected_guid, Clone::clone)
            .unwrap();
        assert!(!creature.unit().has_attacker_like_cpp(player_guid));
        assert!(
            !creature
                .unit()
                .subsystems()
                .combat
                .is_in_combat_with(player_guid)
        );
    }
    {
        let manager = legacy_manager.read().unwrap();
        let creature = manager.find_creature(571, 0, selected_guid).unwrap();
        assert!(!creature.creature.unit().has_attacker_like_cpp(player_guid));
        assert_eq!(creature.creature.ai_ownership().combat_target, None);
        assert_eq!(
            creature
                .creature
                .unit()
                .subsystems()
                .combat
                .threat_value(player_guid),
            Some(0.0)
        );
    }
    assert_eq!(session.combat_target, None);
    assert!(!session.in_combat);
    assert!(
        canonical.lock().unwrap().find_map(631, 0).is_none(),
        "C++ Player::TeleportTo only preflights entry rights; map materialization happens later"
    );
}
#[tokio::test]
async fn teleport_to_blocks_client_without_target_map_expansion_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 792);
    let destination = Position::new(100.0, 200.0, 30.0, 1.5);
    session.expansion = 1;
    session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
        wow_data::MapEntry {
            id: 870,
            instance_type: wow_data::map::MAP_COMMON,
            expansion_id: 2,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
    ])));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "TeleportExpansionReject".to_string(),
        Position::new(10.0, 20.0, 30.0, 0.0),
        571,
        1,
        1,
        80,
        0,
    ));

    session.teleport_to(870, destination).await;

    assert_eq!(
        send_rx.try_recv().expect("SMSG_TRANSFER_ABORTED"),
        wow_packet::packets::misc::TransferAborted {
            map_id: 870,
            arg: 2,
            map_difficulty_x_condition_id: 0,
            transfer_abort: TRANSFER_ABORT_INSUF_EXPAN_LVL_LIKE_CPP,
        }
        .to_bytes()
    );
    assert!(send_rx.try_recv().is_err());
    assert_eq!(session.pending_teleport_like_cpp(), None);
    assert_ne!(session.state, SessionState::Transfer);
}
#[tokio::test]
async fn teleport_to_expansion_abort_preserves_vehicle_state_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 829);
    let registry = Arc::new(PlayerRegistry::with_canonical_player_fixtures_like_cpp());
    let destination = Position::new(100.0, 200.0, 30.0, 1.5);
    session.expansion = 1;
    session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
        wow_data::MapEntry {
            id: 870,
            instance_type: wow_data::map::MAP_COMMON,
            expansion_id: 2,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
    ])));
    session.set_player_registry(Arc::clone(&registry));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "TeleportExpansionRejectVehicle".to_string(),
        Position::new(10.0, 20.0, 30.0, 0.0),
        571,
        1,
        1,
        80,
        0,
    ));
    session.player_vehicle_seat_flags_like_cpp = Some(wow_data::VEHICLE_SEAT_FLAG_CAN_ATTACK);
    session.player_vehicle_seat_id_like_cpp = Some(1004);
    session.register_in_player_registry();

    session.teleport_to(870, destination).await;

    assert_eq!(
        send_rx.try_recv().expect("SMSG_TRANSFER_ABORTED"),
        wow_packet::packets::misc::TransferAborted {
            map_id: 870,
            arg: 2,
            map_difficulty_x_condition_id: 0,
            transfer_abort: TRANSFER_ABORT_INSUF_EXPAN_LVL_LIKE_CPP,
        }
        .to_bytes()
    );
    assert_eq!(
        session.player_vehicle_seat_flags_like_cpp,
        Some(wow_data::VEHICLE_SEAT_FLAG_CAN_ATTACK),
        "C++ returns from the expansion gate before Player::TeleportTo calls ExitVehicle"
    );
    assert_eq!(session.player_vehicle_seat_id_like_cpp, Some(1004));
    let info = registry
        .party_member(player_guid)
        .expect("registered player");
    assert!(info.in_vehicle);
    assert_eq!(info.party_member_vehicle_seat, 1004);
}
#[tokio::test]
async fn teleport_to_allows_target_map_when_session_expansion_matches_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 793);
    let destination = Position::new(101.0, 201.0, 31.0, 1.6);
    session.expansion = 2;
    session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
        wow_data::MapEntry {
            id: 870,
            instance_type: wow_data::map::MAP_COMMON,
            expansion_id: 2,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
    ])));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "TeleportExpansionAllow".to_string(),
        Position::new(10.0, 20.0, 30.0, 0.0),
        571,
        1,
        1,
        80,
        0,
    ));

    session.teleport_to(870, destination).await;

    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![
            ServerOpcodes::CancelCombat,
            ServerOpcodes::TransferPending,
            ServerOpcodes::SuspendToken
        ]
    );
    assert_eq!(
        session.pending_teleport_like_cpp(),
        Some((870, destination))
    );
    assert_eq!(session.state, SessionState::Transfer);
}
#[tokio::test]
async fn teleport_to_battleground_without_assignment_is_silent_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 794);
    let destination = Position::new(102.0, 202.0, 32.0, 1.7);
    session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
        wow_data::MapEntry {
            id: 529,
            instance_type: wow_data::map::MAP_BATTLEGROUND,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
    ])));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "TeleportBgReject".to_string(),
        Position::new(10.0, 20.0, 30.0, 0.0),
        571,
        1,
        1,
        80,
        0,
    ));

    session.teleport_to(529, destination).await;

    assert!(
        send_rx.try_recv().is_err(),
        "C++ Player::TeleportTo returns false without SMSG_TRANSFER_ABORTED for unassigned battleground/arena maps"
    );
    assert_eq!(session.pending_teleport_like_cpp(), None);
    assert_ne!(session.state, SessionState::Transfer);
}
#[tokio::test]
async fn teleport_to_battleground_with_assignment_can_start_transfer_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 795);
    let destination = Position::new(103.0, 203.0, 33.0, 1.8);
    session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
        wow_data::MapEntry {
            id: 529,
            instance_type: wow_data::map::MAP_BATTLEGROUND,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
    ])));
    session.set_player_battleground_context_like_cpp(BATTLEGROUND_AB_LIKE_CPP, 529);
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "TeleportBgAllow".to_string(),
        Position::new(10.0, 20.0, 30.0, 0.0),
        571,
        1,
        1,
        80,
        0,
    ));

    session.teleport_to(529, destination).await;

    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![
            ServerOpcodes::CancelCombat,
            ServerOpcodes::TransferPending,
            ServerOpcodes::SuspendToken
        ]
    );
    assert_eq!(
        session.pending_teleport_like_cpp(),
        Some((529, destination))
    );
    assert_eq!(session.state, SessionState::Transfer);
    assert_eq!(
        session.represented_battleground_leave_requests_like_cpp(),
        0
    );
}
#[tokio::test]
async fn teleport_to_leaves_represented_battleground_when_target_map_differs_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 798);
    let destination = Position::new(104.0, 204.0, 34.0, 1.9);
    session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
        wow_data::MapEntry {
            id: 529,
            instance_type: wow_data::map::MAP_BATTLEGROUND,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
        wow_data::MapEntry {
            id: 571,
            instance_type: wow_data::map::MAP_COMMON,
            expansion_id: 1,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
    ])));
    session.expansion = 1;
    session.set_player_battleground_context_like_cpp(BATTLEGROUND_AB_LIKE_CPP, 529);
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "TeleportBgLeave".to_string(),
        Position::new(10.0, 20.0, 30.0, 0.0),
        529,
        1,
        1,
        80,
        0,
    ));

    session.teleport_to(571, destination).await;

    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![
            ServerOpcodes::CancelCombat,
            ServerOpcodes::TransferPending,
            ServerOpcodes::SuspendToken
        ]
    );
    assert_eq!(
        session.pending_teleport_like_cpp(),
        Some((571, destination))
    );
    assert_eq!(session.state, SessionState::Transfer);
    assert_eq!(
        session.represented_battleground_leave_requests_like_cpp(),
        1
    );
}
#[tokio::test]
async fn teleport_to_far_map_requests_temporary_pet_unsummon_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 799);
    let pet_guid = ObjectGuid::create_world_object(HighGuid::Pet, 0, 1, 571, 0, 500, 799);
    let destination = Position::new(106.0, 206.0, 36.0, 2.1);
    let source = Position::new(10.0, 20.0, 30.0, 0.0);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
        wow_data::MapEntry {
            id: 571,
            instance_type: wow_data::map::MAP_COMMON,
            expansion_id: 1,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
        wow_data::MapEntry {
            id: 0,
            instance_type: wow_data::map::MAP_COMMON,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
    ])));
    session.expansion = 1;
    session.set_represented_pet_mode_state_with_spell_like_cpp(
        Some(pet_guid),
        wow_packet::packets::pet::REACT_DEFENSIVE_LIKE_CPP,
        wow_packet::packets::pet::COMMAND_FOLLOW_LIKE_CPP,
        6_889,
    );
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "TeleportPetUnsummon".to_string(),
        source,
        571,
        1,
        1,
        80,
        0,
    ));
    add_canonical_test_player_on_map(&canonical, player_guid, source, 571, 0);
    add_canonical_test_pet_with_number(
        &canonical,
        pet_guid,
        player_guid,
        500,
        source,
        42,
        6_889,
        0,
    );

    session.teleport_to(0, destination).await;

    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![
            ServerOpcodes::CancelCombat,
            ServerOpcodes::TransferPending,
            ServerOpcodes::SuspendToken
        ]
    );
    assert_eq!(session.pending_teleport_like_cpp(), Some((0, destination)));
    assert_eq!(session.temporary_pet_unsummon_requests_like_cpp(), 1);
    assert_eq!(session.represented_pet_guid_like_cpp, None);
    assert_eq!(
        session.represented_temporary_unsummoned_pet_number_like_cpp,
        42
    );
    assert_eq!(session.represented_old_pet_spell_like_cpp, 6_889);
    {
        let manager = canonical.lock().unwrap();
        assert!(
            manager
                .find_map(571, 0)
                .unwrap()
                .map()
                .get_typed_pet(pet_guid)
                .is_none(),
            "C++ RemovePet(PET_SAVE_AS_CURRENT) removes the temporary-unsummoned pet from the map"
        );
    }
}
#[tokio::test]
async fn teleport_to_far_map_removes_temporary_pet_without_storing_pet_number_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 833);
    let pet_guid = ObjectGuid::create_world_object(HighGuid::Pet, 0, 1, 571, 0, 500, 833);
    let destination = Position::new(116.0, 216.0, 46.0, 2.6);
    let source = Position::new(10.0, 20.0, 30.0, 0.0);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
        wow_data::MapEntry {
            id: 571,
            instance_type: wow_data::map::MAP_COMMON,
            expansion_id: 1,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
        wow_data::MapEntry {
            id: 0,
            instance_type: wow_data::map::MAP_COMMON,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
    ])));
    session.expansion = 1;
    session.set_represented_pet_mode_state_with_spell_like_cpp(
        Some(pet_guid),
        wow_packet::packets::pet::REACT_DEFENSIVE_LIKE_CPP,
        wow_packet::packets::pet::COMMAND_FOLLOW_LIKE_CPP,
        6_890,
    );
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "TeleportTemporaryPetUnsummon".to_string(),
        source,
        571,
        1,
        1,
        80,
        0,
    ));
    add_canonical_test_player_on_map(&canonical, player_guid, source, 571, 0);
    add_canonical_test_pet_with_number(
        &canonical,
        pet_guid,
        player_guid,
        500,
        source,
        43,
        6_890,
        10_000,
    );

    session.teleport_to(0, destination).await;

    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![
            ServerOpcodes::CancelCombat,
            ServerOpcodes::TransferPending,
            ServerOpcodes::SuspendToken
        ]
    );
    assert_eq!(session.temporary_pet_unsummon_requests_like_cpp(), 1);
    assert_eq!(session.represented_pet_guid_like_cpp, None);
    assert_eq!(
        session.represented_temporary_unsummoned_pet_number_like_cpp,
        0
    );
    assert_eq!(session.represented_old_pet_spell_like_cpp, 0);
    {
        let manager = canonical.lock().unwrap();
        assert!(
            manager
                .find_map(571, 0)
                .unwrap()
                .map()
                .get_typed_pet(pet_guid)
                .is_none()
        );
    }
}
#[tokio::test]
async fn teleport_to_far_map_without_pet_does_not_request_pet_unsummon_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 800);
    let destination = Position::new(107.0, 207.0, 37.0, 2.2);
    session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
        wow_data::MapEntry {
            id: 571,
            instance_type: wow_data::map::MAP_COMMON,
            expansion_id: 1,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
        wow_data::MapEntry {
            id: 0,
            instance_type: wow_data::map::MAP_COMMON,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
    ])));
    session.expansion = 1;
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "TeleportNoPetUnsummon".to_string(),
        Position::new(10.0, 20.0, 30.0, 0.0),
        571,
        1,
        1,
        80,
        0,
    ));

    session.teleport_to(0, destination).await;

    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![
            ServerOpcodes::CancelCombat,
            ServerOpcodes::TransferPending,
            ServerOpcodes::SuspendToken
        ]
    );
    assert_eq!(session.pending_teleport_like_cpp(), Some((0, destination)));
    assert_eq!(session.temporary_pet_unsummon_requests_like_cpp(), 0);
}
#[tokio::test]
async fn teleport_to_far_map_delays_when_can_delay_teleport_is_set_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 826);
    let pet_guid = ObjectGuid::create_world_object(HighGuid::Pet, 0, 1, 571, 0, 500, 826);
    let destination = Position::new(108.0, 208.0, 38.0, 2.3);
    session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
        wow_data::MapEntry {
            id: 571,
            instance_type: wow_data::map::MAP_COMMON,
            expansion_id: 1,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
        wow_data::MapEntry {
            id: 0,
            instance_type: wow_data::map::MAP_COMMON,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
    ])));
    session.expansion = 1;
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "FarTeleportDelayed".to_string(),
        Position::new(10.0, 20.0, 30.0, 0.0),
        571,
        1,
        1,
        80,
        0,
    ));
    session.set_represented_pet_mode_state_like_cpp(
        Some(pet_guid),
        wow_packet::packets::pet::REACT_DEFENSIVE_LIKE_CPP,
        wow_packet::packets::pet::COMMAND_FOLLOW_LIKE_CPP,
    );
    session.in_combat = true;
    session.set_represented_can_delay_teleport_like_cpp(true);

    session.teleport_to(0, destination).await;

    assert!(
        send_rx.try_recv().is_err(),
        "C++ delayed far branch stores m_teleport_dest/options and returns before transfer packets"
    );
    assert!(session.represented_has_delayed_teleport_like_cpp());
    assert_eq!(
        session.represented_delayed_teleport_like_cpp(),
        Some((0, destination, TELE_TO_NONE_LIKE_CPP))
    );
    assert!(session.represented_far_teleport_pending_like_cpp());
    assert_eq!(session.pending_teleport_like_cpp(), None);
    assert!(
        session.in_combat,
        "C++ delayed far branch returns before CombatStop"
    );
    assert_eq!(
        session.temporary_pet_unsummon_requests_like_cpp(),
        0,
        "C++ delayed far branch returns before UnsummonPetTemporaryIfAny"
    );
}
#[tokio::test]
async fn update_processes_alive_delayed_far_teleport_like_cpp() {
    let (mut session, _pkt_tx, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 827);
    let source = Position::new(10.0, 20.0, 30.0, 0.0);
    let destination = Position::new(109.0, 209.0, 39.0, 2.4);
    session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
        wow_data::MapEntry {
            id: 571,
            instance_type: wow_data::map::MAP_COMMON,
            expansion_id: 1,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
        wow_data::MapEntry {
            id: 0,
            instance_type: wow_data::map::MAP_COMMON,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
    ])));
    session.expansion = 1;
    session.state = SessionState::LoggedIn;
    session.socket_timeout_deadline_like_cpp = Instant::now() + Duration::from_secs(60);
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "FarTeleportDelayedUpdate".to_string(),
        source,
        571,
        1,
        1,
        80,
        0,
    ));
    session.set_player_health_like_cpp(100, 100);
    session.in_combat = true;
    session.set_represented_can_delay_teleport_like_cpp(true);
    session.teleport_to(0, destination).await;
    assert!(send_rx.try_recv().is_err());

    session.update(50).await;

    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![
            ServerOpcodes::CancelCombat,
            ServerOpcodes::TransferPending,
            ServerOpcodes::SuspendToken
        ]
    );
    assert_eq!(session.state, SessionState::Transfer);
    assert_eq!(session.pending_teleport_like_cpp(), Some((0, destination)));
    assert!(session.represented_far_teleport_pending_like_cpp());
    assert!(!session.represented_has_delayed_teleport_like_cpp());
    assert_eq!(session.represented_delayed_teleport_like_cpp(), None);
    assert!(!session.in_combat);
}
