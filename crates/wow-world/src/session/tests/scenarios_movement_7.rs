//! Session scenarios exercising the represented movement responsibility.
//!
//! Split out of session_tests.rs under #626; assertions and registrations
//! are unchanged and the shared fixtures stay in the parent module.

use super::*;

#[tokio::test]
async fn teleport_to_same_map_delays_when_can_delay_teleport_is_set_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 824);
    let pet_guid = ObjectGuid::create_world_object(HighGuid::Pet, 0, 1, 571, 0, 500, 824);
    let creature_guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 7_824, 1);
    let source = Position::new(10.0, 20.0, 30.0, 0.0);
    let destination = Position::new(
        10.0 + wow_entities::DEFAULT_VISIBILITY_DISTANCE + 25.0,
        20.0,
        30.0,
        4.2,
    );
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
    ])));
    session.expansion = 1;
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "NearTeleportDelayed".to_string(),
        source,
        571,
        1,
        1,
        80,
        0,
    ));
    add_canonical_test_player_on_map(&canonical, player_guid, source, 571, 0);
    add_canonical_test_pet(&canonical, pet_guid, player_guid, 500, source, 0);
    session.set_represented_pet_mode_state_like_cpp(
        Some(pet_guid),
        wow_packet::packets::pet::REACT_DEFENSIVE_LIKE_CPP,
        wow_packet::packets::pet::COMMAND_FOLLOW_LIKE_CPP,
    );
    session.combat_target = Some(creature_guid);
    session.in_combat = true;
    session.set_represented_can_delay_teleport_like_cpp(true);

    session.teleport_to(571, destination).await;

    assert!(
        send_rx.try_recv().is_err(),
        "C++ delayed same-map branch stores m_teleport_dest/options and returns before SendTeleportPacket"
    );
    assert!(session.near_teleport_pending_like_cpp());
    assert!(session.represented_has_delayed_teleport_like_cpp());
    assert_eq!(
        session.represented_delayed_teleport_like_cpp(),
        Some((571, destination, TELE_TO_NONE_LIKE_CPP))
    );
    assert_eq!(
        session.temporary_pet_unsummon_requests_like_cpp(),
        0,
        "C++ returns before the same-map pet distance branch while delayed"
    );
    assert!(
        session.in_combat,
        "C++ returns before CombatStop while delayed"
    );
}
#[tokio::test]
async fn update_processes_alive_delayed_same_map_teleport_like_cpp() {
    let (mut session, _pkt_tx, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 825);
    let source = Position::new(10.0, 20.0, 30.0, 0.0);
    let destination = Position::new(124.0, 224.0, 54.0, 4.3);
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
    ])));
    session.expansion = 1;
    session.state = SessionState::LoggedIn;
    session.socket_timeout_deadline_like_cpp = Instant::now() + Duration::from_secs(60);
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "NearTeleportDelayedUpdate".to_string(),
        source,
        571,
        1,
        1,
        80,
        0,
    ));
    session.set_canonical_map_manager(shared_canonical_map_manager());
    assert!(
        session
            .ensure_canonical_world_map_for_current_player_like_cpp()
            .is_some()
    );
    session.set_player_health_like_cpp(100, 100);
    session.set_in_combat_like_cpp(true);
    session.set_represented_can_delay_teleport_like_cpp(true);
    session.teleport_to(571, destination).await;
    assert!(send_rx.try_recv().is_err());

    session.update(50).await;
    assert_eq!(session.state, SessionState::LoggedIn);
    assert!(!session.represented_has_delayed_teleport_like_cpp());

    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::CancelCombat, ServerOpcodes::MoveTeleport]
    );
    assert!(!session.represented_has_delayed_teleport_like_cpp());
    assert_eq!(session.represented_delayed_teleport_like_cpp(), None);
    assert_eq!(session.resolved_in_combat_like_cpp(), Some(false));
    assert!(session.near_teleport_pending_like_cpp());
}
#[tokio::test]
async fn teleport_to_same_map_fanouts_move_update_teleport_to_visible_players_like_cpp() {
    use wow_packet::packets::movement::MovementInfo;

    let registry = Arc::new(PlayerRegistry::default());
    let (mut source, _, source_rx) = make_session();
    let (mut viewer, _, viewer_rx) = make_session();
    let source_guid = ObjectGuid::create_player(1, 817);
    source.set_map_store(crate::teleport_test_fixtures::world_maps([571]));
    let viewer_guid = ObjectGuid::create_player(1, 818);
    let source_position = Position::new(10.0, 20.0, 30.0, 0.5);
    let viewer_position = Position::new(11.0, 21.0, 30.0, 0.0);
    let destination = Position::new(121.0, 221.0, 51.0, 3.6);

    source.set_player_registry(Arc::clone(&registry));
    source.attach_player_controller_like_cpp(SessionPlayerController::new(
        source_guid,
        "NearTeleportSource".to_string(),
        source_position,
        571,
        1,
        1,
        80,
        0,
    ));
    source.set_canonical_map_manager(shared_canonical_map_manager());
    assert!(
        source
            .ensure_canonical_world_map_for_current_player_like_cpp()
            .is_some()
    );
    source.set_player_movement_time_like_cpp(456);
    source.register_in_player_registry();

    viewer.set_player_registry(Arc::clone(&registry));
    viewer.state = SessionState::LoggedIn;
    viewer.attach_player_controller_like_cpp(SessionPlayerController::new(
        viewer_guid,
        "NearTeleportViewer".to_string(),
        viewer_position,
        571,
        1,
        1,
        80,
        0,
    ));
    viewer.client_visible_guids_like_cpp.insert(source_guid);
    viewer.register_in_player_registry();

    source.teleport_to(571, destination).await;

    assert_eq!(
        drain_server_opcodes(&source_rx),
        vec![ServerOpcodes::CancelCombat, ServerOpcodes::MoveTeleport],
        "C++ sends SMSG_MOVE_TELEPORT to the moved player"
    );
    viewer.process_represented_session_commands_like_cpp().await;

    let packet = viewer_rx
        .try_recv()
        .expect("nearby visible player receives SMSG_MOVE_UPDATE_TELEPORT");
    let mut packet = wow_packet::WorldPacket::from_bytes(&packet);
    assert_eq!(
        packet.server_opcode(),
        Some(ServerOpcodes::MoveUpdateTeleport)
    );
    packet.skip_opcode();
    let status = MovementInfo::read(&mut packet).expect("MovementInfo status");
    assert_eq!(status.guid, source_guid);
    assert_eq!(status.time, 456);
    assert_eq!(
        status.position, source_position,
        "C++ player branch broadcasts current m_movementInfo; destination is carried by self SMSG_MOVE_TELEPORT"
    );
    assert!(viewer_rx.try_recv().is_err());
}
#[tokio::test]
async fn teleport_to_same_map_not_leave_combat_preserves_combat_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 815);
    let creature_guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 7_815, 1);
    let destination = Position::new(120.0, 220.0, 50.0, 3.5);
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
    ])));
    session.expansion = 1;
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "NearTeleportCombat".to_string(),
        Position::new(10.0, 20.0, 30.0, 0.0),
        571,
        1,
        1,
        80,
        0,
    ));
    session.set_canonical_map_manager(shared_canonical_map_manager());
    assert!(
        session
            .ensure_canonical_world_map_for_current_player_like_cpp()
            .is_some()
    );
    session.combat_target = Some(creature_guid);
    session.in_combat = true;

    session
        .teleport_to_with_options(571, destination, TELE_TO_NOT_LEAVE_COMBAT_LIKE_CPP)
        .await;

    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::MoveTeleport],
        "C++ same-map TeleportTo skips CombatStop when TELE_TO_NOT_LEAVE_COMBAT is set"
    );
    assert_eq!(session.combat_target, Some(creature_guid));
    assert!(session.in_combat);
    assert!(session.near_teleport_pending_like_cpp());
}
#[tokio::test]
async fn teleport_to_same_map_logout_sets_near_pending_without_packet_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 816);
    let destination = Position::new(121.0, 221.0, 51.0, 3.6);
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
    ])));
    session.expansion = 1;
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "NearTeleportLogout".to_string(),
        Position::new(10.0, 20.0, 30.0, 0.0),
        571,
        1,
        1,
        80,
        0,
    ));
    session.set_canonical_map_manager(shared_canonical_map_manager());
    assert!(
        session
            .ensure_canonical_world_map_for_current_player_like_cpp()
            .is_some()
    );
    session.set_player_logout_like_cpp(true);

    session
        .teleport_to_with_options(571, destination, TELE_TO_NOT_LEAVE_COMBAT_LIKE_CPP)
        .await;

    assert!(
        send_rx.try_recv().is_err(),
        "C++ same-map TeleportTo does not SendTeleportPacket during PlayerLogout"
    );
    assert!(session.near_teleport_pending_like_cpp());
    assert_eq!(session.pending_teleport_like_cpp(), None);
}
#[tokio::test]
async fn teleport_to_same_map_revive_at_teleport_restores_half_health_and_powers_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 819);
    let source = Position::new(10.0, 20.0, 30.0, 0.0);
    let destination = Position::new(122.0, 222.0, 52.0, 3.7);
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
    ])));
    session.expansion = 1;
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "NearTeleportRevive".to_string(),
        source,
        571,
        1,
        1,
        80,
        0,
    ));
    session.set_player_health_like_cpp(0, 400);
    add_canonical_test_player_on_map(&canonical, player_guid, source, 571, 0);
    assert!(
        session
            .ensure_canonical_player_owner_for_map_like_cpp(wow_map::MapKey::new(571, 0), source,)
    );
    session
        .mutate_canonical_player_like_cpp(|player| {
            player.unit_mut().set_max_health(400);
            player
                .unit_mut()
                .set_death_state(wow_constants::DeathState::Corpse);
            player.unit_mut().set_health(0);
            player.unit_mut().set_power_index(PowerType::Mana, Some(0));
            player.unit_mut().set_power_index(PowerType::Rage, Some(1));
            player
                .unit_mut()
                .set_power_index(PowerType::Energy, Some(3));
            player.unit_mut().set_power_index(PowerType::Focus, Some(4));
            player.unit_mut().set_max_power(PowerType::Mana, 200);
            player.unit_mut().set_power(PowerType::Mana, 0);
            player.unit_mut().set_max_power(PowerType::Rage, 100);
            player.unit_mut().set_power(PowerType::Rage, 50);
            player.unit_mut().set_max_power(PowerType::Energy, 120);
            player.unit_mut().set_power(PowerType::Energy, 0);
            player.unit_mut().set_max_power(PowerType::Focus, 80);
            player.unit_mut().set_power(PowerType::Focus, 0);
        })
        .expect("canonical player exists");

    session
        .teleport_to_with_options(571, destination, TELE_REVIVE_AT_TELEPORT_LIKE_CPP)
        .await;

    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::CancelCombat, ServerOpcodes::MoveTeleport]
    );
    assert!(session.player_is_alive_like_cpp());
    assert_eq!(session.player_health_like_cpp(), 200);
    assert!(session.near_teleport_pending_like_cpp());
    assert_eq!(
        session
            .mutate_canonical_player_like_cpp(|player| {
                (
                    player.unit().data().health,
                    player.get_power(PowerType::Mana),
                    player.get_power(PowerType::Rage),
                    player.get_power(PowerType::Energy),
                    player.get_power(PowerType::Focus),
                )
            })
            .expect("canonical player exists"),
        (200, 100, 0, 60, 40),
        "C++ Player::ResurrectPlayer(0.5f) restores health/mana/energy/focus by percent and resets rage"
    );
}
#[tokio::test]
async fn teleport_to_same_map_without_revive_flag_keeps_dead_player_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 820);
    let source = Position::new(10.0, 20.0, 30.0, 0.0);
    let destination = Position::new(123.0, 223.0, 53.0, 3.8);
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
    ])));
    session.expansion = 1;
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "NearTeleportNoRevive".to_string(),
        source,
        571,
        1,
        1,
        80,
        0,
    ));
    session.set_canonical_map_manager(shared_canonical_map_manager());
    assert!(
        session
            .ensure_canonical_world_map_for_current_player_like_cpp()
            .is_some()
    );
    session.set_player_health_like_cpp(0, 400);

    session.teleport_to(571, destination).await;

    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::CancelCombat, ServerOpcodes::MoveTeleport]
    );
    assert!(!session.player_is_alive_like_cpp());
    assert_eq!(session.player_health_like_cpp(), 0);
    assert!(session.near_teleport_pending_like_cpp());
}
#[tokio::test]
async fn teleport_to_same_map_unsummons_pet_when_destination_out_of_visibility_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 821);
    let pet_guid = ObjectGuid::create_world_object(HighGuid::Pet, 0, 1, 571, 0, 500, 821);
    let source = Position::new(10.0, 20.0, 30.0, 0.0);
    let destination = Position::new(
        10.0 + wow_entities::DEFAULT_VISIBILITY_DISTANCE + 25.0,
        20.0,
        30.0,
        3.9,
    );
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
    ])));
    session.expansion = 1;
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "NearTeleportPetFar".to_string(),
        source,
        571,
        1,
        1,
        80,
        0,
    ));
    add_canonical_test_player_on_map(&canonical, player_guid, source, 571, 0);
    add_canonical_test_pet(&canonical, pet_guid, player_guid, 500, source, 0);
    session.set_represented_pet_mode_state_like_cpp(
        Some(pet_guid),
        wow_packet::packets::pet::REACT_DEFENSIVE_LIKE_CPP,
        wow_packet::packets::pet::COMMAND_FOLLOW_LIKE_CPP,
    );

    session.teleport_to(571, destination).await;

    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::CancelCombat, ServerOpcodes::MoveTeleport]
    );
    assert_eq!(session.temporary_pet_unsummon_requests_like_cpp(), 1);
}
#[tokio::test]
async fn teleport_to_same_map_keeps_pet_when_destination_within_visibility_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 822);
    let pet_guid = ObjectGuid::create_world_object(HighGuid::Pet, 0, 1, 571, 0, 500, 822);
    let source = Position::new(10.0, 20.0, 30.0, 0.0);
    let destination = Position::new(35.0, 20.0, 30.0, 4.0);
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
    ])));
    session.expansion = 1;
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "NearTeleportPetNear".to_string(),
        source,
        571,
        1,
        1,
        80,
        0,
    ));
    add_canonical_test_player_on_map(&canonical, player_guid, source, 571, 0);
    add_canonical_test_pet(&canonical, pet_guid, player_guid, 500, source, 0);
    session.set_represented_pet_mode_state_like_cpp(
        Some(pet_guid),
        wow_packet::packets::pet::REACT_DEFENSIVE_LIKE_CPP,
        wow_packet::packets::pet::COMMAND_FOLLOW_LIKE_CPP,
    );

    session.teleport_to(571, destination).await;

    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::CancelCombat, ServerOpcodes::MoveTeleport]
    );
    assert_eq!(session.temporary_pet_unsummon_requests_like_cpp(), 0);
}
#[tokio::test]
async fn teleport_to_same_map_not_unsummon_pet_option_preserves_pet_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 823);
    let pet_guid = ObjectGuid::create_world_object(HighGuid::Pet, 0, 1, 571, 0, 500, 823);
    let source = Position::new(10.0, 20.0, 30.0, 0.0);
    let destination = Position::new(
        10.0 + wow_entities::DEFAULT_VISIBILITY_DISTANCE + 25.0,
        20.0,
        30.0,
        4.1,
    );
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
    ])));
    session.expansion = 1;
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "NearTeleportPetNoUnsummon".to_string(),
        source,
        571,
        1,
        1,
        80,
        0,
    ));
    add_canonical_test_player_on_map(&canonical, player_guid, source, 571, 0);
    add_canonical_test_pet(&canonical, pet_guid, player_guid, 500, source, 0);
    session.set_represented_pet_mode_state_like_cpp(
        Some(pet_guid),
        wow_packet::packets::pet::REACT_DEFENSIVE_LIKE_CPP,
        wow_packet::packets::pet::COMMAND_FOLLOW_LIKE_CPP,
    );

    session
        .teleport_to_with_options(571, destination, TELE_TO_NOT_UNSUMMON_PET_LIKE_CPP)
        .await;

    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::CancelCombat, ServerOpcodes::MoveTeleport]
    );
    assert_eq!(session.temporary_pet_unsummon_requests_like_cpp(), 0);
}
#[test]
fn logout_save_snapshot_uses_fall_damage_synced_to_canonical_health_like_cpp() {
    let (mut session, _, _) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 74);
    let position = Position::new(77.0, 88.0, 99.0, 2.5);

    canonical.lock().unwrap().create_world_map(571, 0);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
        wow_data::MapEntry {
            id: 571,
            instance_type: wow_data::map::MAP_COMMON,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
    ])));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "Damaged".to_string(),
        position,
        571,
        1,
        3,
        80,
        0,
    ));
    let _ = session.ensure_canonical_world_map_for_current_player_like_cpp();
    session
        .mutate_canonical_player_like_cpp(|player| {
            player.unit_mut().set_max_health(6_310);
            player.unit_mut().set_health(6_310);
        })
        .unwrap();
    session.set_player_health_like_cpp(6_310, 6_310);
    session.set_fall_information_like_cpp(1_200, 120.0);
    let mut fall_land = wow_packet::packets::movement::MovementInfo::default();
    fall_land.position.z = 100.0;
    fall_land.jump.fall_time = 1_500;

    let fall = session
        .handle_fall_like_cpp(&fall_land)
        .expect("fall damage should apply");
    assert!(fall.final_damage > 0);
    let damaged_health = session.player_health_like_cpp();
    assert!(damaged_health < 6_310);
    assert_eq!(
        session.canonical_player_health_snapshot_like_cpp(),
        Some((damaged_health, 6_310)),
        "session-side fall damage must sync the canonical Player before SaveToDB snapshots it"
    );

    let snapshot = session
        .current_player_save_to_db_snapshot_like_cpp()
        .expect("snapshot should exist");

    assert_eq!(snapshot.health, damaged_health);
    assert_eq!(snapshot.max_health, 6_310);
    assert_eq!(session.player_health_like_cpp(), damaged_health);
}
#[test]
fn logout_save_snapshot_falls_back_to_session_position_without_canonical_player_like_cpp() {
    let (mut session, _, _) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 71);
    let login_position = Position::new(10.0, 20.0, 30.0, 0.0);
    let moved_position = Position::new(44.0, 55.0, 66.0, 1.25);

    canonical.lock().unwrap().create_world_map(571, 0);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
        wow_data::MapEntry {
            id: 571,
            instance_type: wow_data::map::MAP_COMMON,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
    ])));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "Mover".to_string(),
        login_position,
        571,
        1,
        3,
        10,
        0,
    ));
    session.set_player_level_like_cpp(42);
    session.set_player_xp_like_cpp(1234);
    session.set_player_gold_like_cpp(5678);
    session.set_player_health_like_cpp(33, 100);
    session.set_player_position_like_cpp(moved_position);

    let snapshot = session
        .current_player_save_to_db_snapshot_like_cpp()
        .unwrap();
    assert_eq!(snapshot.position, moved_position);
    assert_eq!(snapshot.map_id, 571);
    assert_eq!(snapshot.level, 42);
    assert_eq!(snapshot.xp, 1234);
    assert_eq!(snapshot.money, 5678);
    assert_eq!(snapshot.health, 33);
    assert_eq!(snapshot.max_health, 100);
    assert_eq!(snapshot.powers, empty_character_power_snapshot_like_cpp());
}
#[test]
fn logout_save_snapshot_uses_pending_far_teleport_destination_like_cpp() {
    let (mut session, _, _) = make_session();
    let player_guid = ObjectGuid::create_player(1, 72);
    let current_position = Position::new(10.0, 20.0, 30.0, 0.0);
    let teleport_destination = Position::new(100.0, 200.0, 300.0, 1.5);

    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "Teleporter".to_string(),
        current_position,
        571,
        1,
        3,
        80,
        0,
    ));
    assert!(session.set_pending_teleport_like_cpp(Some((0, teleport_destination))));

    let snapshot = session
        .current_player_save_to_db_snapshot_like_cpp()
        .expect("snapshot should exist");

    assert_eq!(snapshot.map_id, 0);
    assert_eq!(
        snapshot.instance_id, 0,
        "C++ Player::SaveToDB stores instance 0 while IsBeingTeleported() and saving GetTeleportDest"
    );
    assert_eq!(snapshot.position, teleport_destination);
}
#[test]
fn logout_save_snapshot_uses_pending_near_teleport_destination_like_cpp() {
    let (mut session, _, _) = make_session();
    let player_guid = ObjectGuid::create_player(1, 73);
    let current_position = Position::new(10.0, 20.0, 30.0, 0.0);
    let teleport_destination = Position::new(44.0, 55.0, 66.0, 2.25);

    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "NearTeleporter".to_string(),
        current_position,
        571,
        1,
        3,
        80,
        0,
    ));
    session.set_near_teleport_pending_like_cpp(
        true,
        Some((571, teleport_destination)),
        Some((10, 20)),
    );

    let snapshot = session
        .current_player_save_to_db_snapshot_like_cpp()
        .expect("snapshot should exist");

    assert_eq!(snapshot.map_id, 571);
    assert_eq!(
        snapshot.instance_id, 0,
        "C++ Player::SaveToDB also stores instance 0 for same-map pending teleport destinations"
    );
    assert_eq!(snapshot.position, teleport_destination);
}
#[tokio::test]
async fn periodic_player_save_defers_while_teleport_pending_like_cpp() {
    let (mut session, _, _) = make_session();
    session.state = SessionState::LoggedIn;
    session.set_player_save_interval_ms_like_cpp(100);
    assert!(session.set_pending_teleport_like_cpp(Some((0, Position::new(10.0, 20.0, 30.0, 1.5)))));
    session.update_player_save_timer_like_cpp(100);

    session
        .process_pending_periodic_player_save_like_cpp()
        .await;

    assert!(
        session.pending_periodic_player_save_like_cpp,
        "autosave remains pending until the teleport handshake clears"
    );
    assert_eq!(session.next_player_save_ms_like_cpp, 0);
}
#[test]
fn player_currency_remove_and_save_state_match_cpp() {
    let (mut session, _, _) = make_session();
    session.set_currency_types_store(Arc::new(wow_data::CurrencyTypesStore::from_entries([
        currency_entry(395),
        currency_entry(396),
        currency_entry(397),
    ])));
    session.player_currencies.insert(
        395,
        PlayerCurrency {
            state: PlayerCurrencyState::Unchanged,
            quantity: 42,
            weekly_quantity: 5,
            tracked_quantity: 6,
            increased_cap_quantity: 7,
            earned_quantity: 8,
            flags: 9,
        },
    );
    session.player_currencies.insert(
        396,
        PlayerCurrency {
            state: PlayerCurrencyState::New,
            quantity: 3,
            weekly_quantity: 0,
            tracked_quantity: 0,
            increased_cap_quantity: 0,
            earned_quantity: 0,
            flags: 0,
        },
    );
    session.player_currencies.insert(
        397,
        PlayerCurrency {
            state: PlayerCurrencyState::Unchanged,
            quantity: 10,
            weekly_quantity: 0,
            tracked_quantity: 0,
            increased_cap_quantity: 0,
            earned_quantity: 0,
            flags: 0,
        },
    );

    assert!(session.remove_currency(395, 100));
    assert_eq!(session.player_currency_quantity(395), Some(0));
    assert_eq!(
        session
            .player_currencies
            .get(&395)
            .map(|currency| currency.state),
        Some(PlayerCurrencyState::Changed)
    );
    assert!(!session.remove_currency(999, 1));

    let mut persisted_currencies = session.player_currencies_like_cpp().unwrap();
    let request = session.plan_player_currency_save_like_cpp(1, &mut persisted_currencies);
    session.set_player_currencies_like_cpp(persisted_currencies);
    assert_eq!(request.rows.len(), 2);
    assert_eq!(
        session
            .player_currencies
            .get(&395)
            .map(|currency| currency.state),
        Some(PlayerCurrencyState::Unchanged)
    );
    assert_eq!(
        session
            .player_currencies
            .get(&396)
            .map(|currency| currency.state),
        Some(PlayerCurrencyState::Unchanged)
    );
    assert_eq!(
        session
            .player_currencies
            .get(&397)
            .map(|currency| currency.state),
        Some(PlayerCurrencyState::Unchanged)
    );
}
#[test]
fn send_buy_and_sell_error_mirror_cpp_empty_vendor_fallback() {
    let (session, _, send_rx) = make_session();
    let vendor_guid = ObjectGuid::new(0, 0x0102);
    let item_guid = ObjectGuid::new(0, 0x0506);

    let expected = BuyFailed {
        vendor_guid,
        muid: 6948,
        reason: BuyResult::CantFindItem,
    }
    .to_bytes();
    session.send_buy_error(BuyResult::CantFindItem, Some(vendor_guid), 6948);
    assert_eq!(send_rx.try_recv().unwrap(), expected);

    let expected =
        SellResponse::error(ObjectGuid::EMPTY, item_guid, SellResult::YouDontOwnThatItem)
            .to_bytes();
    session.send_sell_error(SellResult::YouDontOwnThatItem, None, item_guid);
    assert_eq!(send_rx.try_recv().unwrap(), expected);
}
