//! Session scenarios exercising the represented movement responsibility.
//!
//! Split out of session_tests.rs under #626; assertions and registrations
//! are unchanged and the shared fixtures stay in the parent module.

use super::*;

#[test]
fn canonical_player_movement_control_follows_active_detached_and_stale_ownership_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 5_575);

    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_map_store(canonical_player_transfer_test_map_store_like_cpp());
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "MovementControlOwner".to_string(),
        Position::new(3700.0, 1500.0, 120.0, 0.0),
        571,
        1,
        1,
        80,
        0,
    ));
    session
        .ensure_canonical_world_map_for_current_player_like_cpp()
        .expect("initial world map");
    let old_handle = session.player_handle_like_cpp.expect("canonical handle");

    assert!(session.set_fall_information_like_cpp(1_200, 87.5));
    session.set_forced_speed_changes_like_cpp(UnitMoveTypeLikeCpp::Run, 2);
    session.set_movement_force_mod_magnitude_changes_like_cpp(3);
    session.set_player_movement_speed_rate_like_cpp(UnitMoveTypeLikeCpp::Run, 1.5);
    session.set_movement_force_mod_magnitude_like_cpp(1.25);
    assert!(
        session
            .with_owned_player_mut_like_cpp(|player| {
                let movement = &mut player.gameplay_state_mut().movement_control;
                movement.can_swim_to_fly_transition = true;
                movement.mover_fixed_position_vehicle = true;
                movement.scale_duration = 250;
            })
            .is_some()
    );
    assert_eq!(session.next_movement_counter_like_cpp(), Some(0));
    assert_eq!(session.fall_information_like_cpp(), (1_200, 87.5));
    assert_eq!(
        session.forced_speed_changes_like_cpp(UnitMoveTypeLikeCpp::Run),
        2
    );
    assert_eq!(
        session.resolved_movement_force_mod_magnitude_changes_like_cpp(),
        Some(3)
    );
    assert_eq!(
        session.resolved_player_movement_speed_rate_like_cpp(UnitMoveTypeLikeCpp::Run),
        Some(1.5)
    );
    assert_eq!(
        session.resolved_movement_force_mod_magnitude_like_cpp(),
        Some(1.25)
    );
    assert_eq!(session.movement_counter_like_cpp(), Some(1));
    assert_eq!(
        session.resolved_can_swim_to_fly_transition_like_cpp(),
        Some(true)
    );
    assert_eq!(
        session.resolved_mover_fixed_position_vehicle_like_cpp(),
        Some(true)
    );
    assert_eq!(session.resolved_player_scale_duration_like_cpp(), Some(250));

    assert!(session.remove_current_player_from_canonical_current_map_like_cpp());
    assert_eq!(
        canonical
            .lock()
            .unwrap()
            .player_residence_like_cpp(old_handle),
        Some(wow_map::PlayerResidenceLikeCpp::Detached)
    );
    assert_eq!(session.fall_information_like_cpp(), (1_200, 87.5));
    assert_eq!(
        session.forced_speed_changes_like_cpp(UnitMoveTypeLikeCpp::Run),
        2
    );
    assert_eq!(session.movement_counter_like_cpp(), Some(1));
    assert_eq!(
        session.resolved_can_swim_to_fly_transition_like_cpp(),
        Some(true)
    );
    assert_eq!(
        session.resolved_mover_fixed_position_vehicle_like_cpp(),
        Some(true)
    );
    assert_eq!(session.resolved_player_scale_duration_like_cpp(), Some(250));

    let mut replacement = Box::new(Player::new(Some(2), false));
    replacement
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(player_guid);
    replacement.set_fall_information_like_cpp(900, 44.0);
    assert!(replacement.set_forced_speed_changes_like_cpp(UnitMoveTypeLikeCpp::Run.index(), 7));
    replacement.set_movement_force_mod_magnitude_changes_like_cpp(8);
    assert!(
        replacement
            .unit_mut()
            .set_speed_rate_at_like_cpp(UnitMoveTypeLikeCpp::Run.index(), 0.75)
    );
    replacement
        .unit_mut()
        .set_movement_force_mod_magnitude_like_cpp(0.8);
    assert_eq!(replacement.unit_mut().next_movement_counter_like_cpp(), 0);
    assert_eq!(replacement.unit_mut().next_movement_counter_like_cpp(), 1);
    let replacement_handle = canonical
        .lock()
        .unwrap()
        .install_detached_player_like_cpp(replacement)
        .expect("replacement owner");

    assert_eq!(session.resolved_fall_information_like_cpp(), None);
    assert_eq!(
        session.resolved_forced_speed_changes_like_cpp(UnitMoveTypeLikeCpp::Run),
        None
    );
    assert_eq!(
        session.resolved_movement_force_mod_magnitude_changes_like_cpp(),
        None
    );
    assert_eq!(
        session.resolved_player_movement_speed_rate_like_cpp(UnitMoveTypeLikeCpp::Run),
        None
    );
    assert_eq!(
        session.resolved_movement_force_mod_magnitude_like_cpp(),
        None
    );
    assert_eq!(session.movement_counter_like_cpp(), None);
    assert_eq!(session.resolved_can_swim_to_fly_transition_like_cpp(), None);
    assert_eq!(
        session.resolved_mover_fixed_position_vehicle_like_cpp(),
        None
    );
    assert_eq!(session.resolved_player_scale_duration_like_cpp(), None);
    assert!(!session.set_fall_information_like_cpp(2_000, 10.0));
    assert!(!session.set_represented_can_swim_to_fly_transition_like_cpp(false));
    session.set_represented_mover_fixed_position_vehicle_like_cpp(false);
    assert_eq!(session.next_movement_counter_like_cpp(), None);
    assert_eq!(
        canonical
            .lock()
            .unwrap()
            .with_player_like_cpp(replacement_handle, |player| (
                player.fall_information_like_cpp(),
                player.forced_speed_changes_like_cpp(UnitMoveTypeLikeCpp::Run.index()),
                player.movement_force_mod_magnitude_changes_like_cpp(),
                player
                    .unit()
                    .speed_rate_at_like_cpp(UnitMoveTypeLikeCpp::Run.index()),
                player.unit().movement_force_mod_magnitude_like_cpp(),
                player.unit().movement_counter_like_cpp(),
                player
                    .gameplay_state()
                    .movement_control
                    .can_swim_to_fly_transition,
                player
                    .gameplay_state()
                    .movement_control
                    .mover_fixed_position_vehicle,
                player.gameplay_state().movement_control.scale_duration,
            )),
        Some(((900, 44.0), Some(7), 8, Some(0.75), 0.8, 2, false, false, 0))
    );
}
#[test]
fn canonical_player_teleport_follows_active_detached_and_stale_ownership_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 5_580);
    let destination = Position::new(3701.0, 1501.0, 121.0, 0.5);
    let owned = wow_entities::PlayerTeleportStateLikeCpp {
        recovery: Default::default(),
        far_destination: Some((1, destination)),
        post_add: Some(wow_entities::PlayerWorldportPostAddLikeCpp {
            map_id: 1,
            position: destination,
            phase: wow_entities::PlayerWorldportPostAddPhaseLikeCpp::BeforeZone,
        }),
        can_delay: true,
        has_delayed: true,
        near_pending: true,
        far_pending: false,
        near_destination: Some((571, destination)),
        delayed: Some((571, destination, TELE_TO_SPELL_LIKE_CPP)),
        near_destination_zone_area: Some((20, 21)),
    };

    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_map_store(canonical_player_transfer_test_map_store_like_cpp());
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "TeleportOwner".to_string(),
        Position::new(3700.0, 1500.0, 120.0, 0.0),
        571,
        1,
        1,
        80,
        0,
    ));
    session
        .ensure_canonical_world_map_for_current_player_like_cpp()
        .expect("initial world map");
    let old_handle = session.player_handle_like_cpp.expect("canonical handle");

    assert!(session.update_player_teleport_state_like_cpp(|state| *state = owned));
    assert_eq!(session.pending_teleport_like_cpp(), owned.far_destination);
    assert_eq!(
        session.player_teleport_state_snapshot_like_cpp(),
        Some(owned)
    );

    assert!(session.remove_current_player_from_canonical_current_map_like_cpp());
    assert_eq!(
        canonical
            .lock()
            .unwrap()
            .player_residence_like_cpp(old_handle),
        Some(wow_map::PlayerResidenceLikeCpp::Detached)
    );
    assert_eq!(
        session.player_teleport_state_snapshot_like_cpp(),
        Some(owned)
    );

    let replacement_state = wow_entities::PlayerTeleportStateLikeCpp {
        recovery: Default::default(),
        far_destination: None,
        post_add: None,
        can_delay: false,
        has_delayed: false,
        near_pending: false,
        far_pending: true,
        near_destination: None,
        delayed: None,
        near_destination_zone_area: None,
    };
    let mut replacement = Box::new(Player::new(Some(2), false));
    replacement
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(player_guid);
    *replacement.teleport_state_mut_like_cpp() = replacement_state;
    let replacement_handle = canonical
        .lock()
        .unwrap()
        .install_detached_player_like_cpp(replacement)
        .expect("replacement owner");

    assert_eq!(session.player_teleport_state_snapshot_like_cpp(), None);
    assert_eq!(session.pending_teleport_like_cpp(), None);
    assert!(!session.finish_worldport_native_before_disconnect_like_cpp());
    assert!(!session.set_pending_teleport_like_cpp(Some((1, destination))));
    assert!(!session.set_represented_can_delay_teleport_like_cpp(true));
    assert!(!session.set_represented_far_teleport_pending_like_cpp(false));
    assert!(!session.set_near_teleport_pending_like_cpp(
        true,
        Some((571, destination)),
        Some((20, 21)),
    ));
    assert_eq!(
        canonical
            .lock()
            .unwrap()
            .with_player_like_cpp(replacement_handle, |player| *player
                .teleport_state_like_cpp()),
        Some(replacement_state)
    );
}
#[test]
fn canonical_player_rejected_map_sync_does_not_remove_existing_map_player_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 54);

    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
        wow_data::MapEntry {
            id: 571,
            instance_type: wow_data::map::MAP_INSTANCE,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
    ])));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "RejectMap".to_string(),
        Position::new(3700.0, 1500.0, 120.0, 0.0),
        571,
        1,
        1,
        80,
        0,
    ));
    insert_session_player_into_canonical_map_like_cpp(&session, &canonical, 571, 0);

    assert_eq!(
        session.ensure_canonical_world_map_for_current_player_like_cpp(),
        Some(wow_map::CreateMapDecision::Reject {
            side_effects: Vec::new()
        })
    );
    let manager = canonical.lock().unwrap();
    assert!(manager.find_map(0, 0).is_none());
    assert!(
        manager
            .find_map(571, 0)
            .unwrap()
            .map()
            .get_typed_player(player_guid)
            .is_some()
    );
}
#[test]
fn canonical_player_dungeon_missing_map_difficulty_sends_transfer_abort_like_cpp() {
    let (mut session, _pkt_tx, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 63);

    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
        wow_data::MapEntry {
            id: 631,
            instance_type: wow_data::map::MAP_RAID,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
    ])));
    session.set_difficulty_store(Arc::new(DifficultyStore::from_entries([DifficultyEntry {
        id: 3,
        instance_type: MAP_RAID_LIKE_CPP,
        flags: DifficultyFlags::CAN_SELECT.bits(),
        fallback_difficulty_id: 0,
        toggle_difficulty_id: 0,
    }])));
    session.set_map_difficulty_store(Arc::new(MapDifficultyStore::from_entries([])));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "MissingMapDifficulty".to_string(),
        Position::new(3700.0, 1500.0, 120.0, 0.0),
        631,
        1,
        1,
        80,
        0,
    ));
    session.represented_raid_difficulty_id_like_cpp = 3;

    assert_eq!(
        session.ensure_canonical_world_map_for_current_player_like_cpp(),
        Some(wow_map::CreateMapDecision::Reject {
            side_effects: Vec::new()
        })
    );
    assert_eq!(
        send_rx.try_recv().expect("SMSG_TRANSFER_ABORTED"),
        wow_packet::packets::misc::TransferAborted {
            map_id: 631,
            arg: 0,
            map_difficulty_x_condition_id: 0,
            transfer_abort: TRANSFER_ABORT_DIFFICULTY_LIKE_CPP,
        }
        .to_bytes()
    );
    assert!(
        canonical.lock().unwrap().find_map(631, 0).is_none(),
        "rejected dungeon difficulty must not create or sync a canonical map"
    );
}
#[test]
fn canonical_player_existing_raid_in_progress_sends_transfer_abort_like_cpp() {
    let (mut session, _pkt_tx, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let leader = ObjectGuid::create_player(1, 74);
    let member = ObjectGuid::create_player(1, 75);

    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        member,
        "RaidInProgressReject".to_string(),
        Position::new(3700.0, 1500.0, 120.0, 0.0),
        631,
        1,
        1,
        80,
        0,
    ));
    session.represented_raid_difficulty_id_like_cpp = 3;
    install_create_map_active_lock_stores_like_cpp(&mut session, 631, 3, 77, 0);

    let group_registry = Arc::new(GroupRegistry::default());
    let mut group = GroupInfo::new(leader);
    group.raid_difficulty_id = 3;
    group.add_member(member);
    group.set_recent_instance_like_cpp(631, leader, 9001);
    let group_guid = group.group_guid;
    group_registry.register_group_like_cpp(group_guid, group);
    session.group_guid = Some(group_guid);
    session.set_group_registry(group_registry, Arc::new(PendingInvites::default()));

    {
        let mut manager = canonical.lock().unwrap();
        let map = manager.create_map_entry(
            631,
            9001,
            3,
            wow_map::ManagedMapKind::Dungeon {
                has_reset_schedule: false,
            },
        );
        map.set_instance_encounter_in_progress_like_cpp(true);
    }

    assert_eq!(
        session.ensure_canonical_world_map_for_current_player_like_cpp(),
        Some(wow_map::CreateMapDecision::Reject {
            side_effects: Vec::new()
        })
    );
    assert_eq!(
        send_rx.try_recv().expect("SMSG_TRANSFER_ABORTED"),
        wow_packet::packets::misc::TransferAborted {
            map_id: 631,
            arg: 0,
            map_difficulty_x_condition_id: 0,
            transfer_abort: TRANSFER_ABORT_ZONE_IN_COMBAT_LIKE_CPP,
        }
        .to_bytes()
    );
    assert!(
        canonical
            .lock()
            .unwrap()
            .find_map(631, 9001)
            .unwrap()
            .map()
            .get_typed_player(member)
            .is_none(),
        "zone-in-combat rejection must not synchronize the player"
    );
}
#[test]
fn loot_reconciliation_rejects_map_key_after_player_transfer_like_cpp() {
    let (mut session, _, _) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 61_706);
    let position = Position::new(10.0, 20.0, 30.0, 1.0);

    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "TransferredOwner".to_string(),
        position,
        571,
        1,
        1,
        80,
        0,
    ));
    add_canonical_test_player_on_map(&canonical, player_guid, position, 571, 7);
    let captured = session
        .current_canonical_player_map_key_like_cpp()
        .expect("the first instance owns the player at capture time");
    assert!(session.loot_reconciliation_map_key_still_valid_like_cpp(captured, true));

    canonical
        .lock()
        .unwrap()
        .find_map_mut(571, 7)
        .unwrap()
        .map_mut()
        .remove_from_map_like_cpp(player_guid, true)
        .unwrap();
    add_canonical_test_player_on_map(&canonical, player_guid, position, 571, 8);

    assert_eq!(
        session.current_canonical_player_map_key_like_cpp(),
        Some(wow_map::MapKey::new(571, 8))
    );
    assert!(
        !session.loot_reconciliation_map_key_still_valid_like_cpp(captured, true),
        "a reconciliation started in instance 7 cannot finish against instance 8"
    );
}
#[test]
fn logged_in_loot_lookup_does_not_fallback_when_canonical_player_is_absent_like_cpp() {
    let (mut session, _, _) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 61_707);
    let creature_guid = test_creature_guid(61_708);
    let position = Position::new(10.0, 20.0, 30.0, 1.0);

    session.set_state(SessionState::LoggedIn);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "PlayerBetweenMaps".to_string(),
        position,
        571,
        1,
        1,
        80,
        0,
    ));
    add_canonical_test_creature_on_map(&canonical, creature_guid, 9_108, position, 0, 571, 7);

    assert_eq!(session.current_canonical_player_map_key_like_cpp(), None);
    assert_eq!(session.canonical_object_lookup_map_key_like_cpp(571), None);
    assert!(
        session
            .read_canonical_creature_loot_authority_like_cpp(creature_guid)
            .is_none()
    );
}
#[test]
fn represented_move_dismiss_vehicle_uses_charmed_guid_gate_like_cpp() {
    let (mut session, _, _) = make_session();
    let guid = ObjectGuid::create_player(1, 53);
    let vehicle_guid = test_creature_guid(53_001);
    let canonical = shared_canonical_map_manager();
    let registry = Arc::new(PlayerRegistry::default());
    let position = Position::new(1.0, 2.0, 3.0, 0.0);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_player_guid(Some(guid));
    session.set_player_map_position_like_cpp(571, position);
    session.player_name = Some("MoveDismissVehicleTester".to_string());
    session.player_vehicle_seat_flags_like_cpp = Some(wow_data::VEHICLE_SEAT_FLAG_CAN_ATTACK);
    session.player_vehicle_seat_id_like_cpp = Some(1004);
    session.set_player_registry(Arc::clone(&registry));
    add_canonical_test_player_on_map(&canonical, guid, position, 571, 0);
    {
        let mut guard = canonical.lock().unwrap();
        guard
            .find_map_mut(571, 0)
            .unwrap()
            .map_mut()
            .get_typed_player_mut(guid)
            .unwrap()
            .unit_mut()
            .subsystems_mut()
            .control
            .set_charmed(vehicle_guid);
    }
    session.register_in_player_registry();

    let dismiss_position = Position::new(4.0, 5.0, 6.0, 1.0);
    let mut status = wow_packet::packets::movement::MovementInfo {
        guid,
        flags: MovementFlag::ROOT | MovementFlag::FORWARD,
        time: 12_345,
        position: dismiss_position,
        ..wow_packet::packets::movement::MovementInfo::default()
    };

    assert!(session.represented_move_dismiss_vehicle_like_cpp(&mut status));

    assert!(session.player_vehicle_seat_flags_like_cpp.is_none());
    assert!(session.player_vehicle_seat_id_like_cpp.is_none());
    assert_eq!(session.player_movement_time_like_cpp(), 12_345);
    assert_eq!(session.player_position_like_cpp(), Some(dismiss_position));
    assert!(
        !session
            .player_movement_flags_like_cpp
            .contains(MovementFlag::ROOT)
    );
    assert!(
        session
            .player_movement_flags_like_cpp
            .contains(MovementFlag::FORWARD),
        "C++ ValidateMovementInfo removes ROOT first, so FORWARD remains"
    );
    assert_eq!(
        session.represented_vehicle_dismiss_movements_like_cpp(),
        &[RepresentedVehicleDismissMovementLikeCpp {
            vehicle_guid,
            sanitized_flags: MovementFlag::FORWARD,
            position: dismiss_position,
            time: 12_345,
        }]
    );
    let info = registry.party_member(guid).expect("registered player");
    assert!(!info.in_vehicle);
    assert_eq!(info.party_member_vehicle_seat, 0);
}
#[test]
fn represented_move_dismiss_vehicle_uses_charm_not_seat_gate_like_cpp() {
    let (mut session, _, _) = make_session();
    let guid = ObjectGuid::create_player(1, 55);
    let vehicle_guid = test_creature_guid(55_001);
    let canonical = shared_canonical_map_manager();
    let position = Position::new(1.0, 2.0, 3.0, 0.0);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_player_guid(Some(guid));
    session.set_player_map_position_like_cpp(571, position);
    session.player_name = Some("MoveDismissVehicleCharmOnlyTester".to_string());
    add_canonical_test_player_on_map(&canonical, guid, position, 571, 0);
    {
        let mut guard = canonical.lock().unwrap();
        guard
            .find_map_mut(571, 0)
            .unwrap()
            .map_mut()
            .get_typed_player_mut(guid)
            .unwrap()
            .unit_mut()
            .subsystems_mut()
            .control
            .set_charmed(vehicle_guid);
    }

    let dismiss_position = Position::new(7.0, 8.0, 9.0, 1.5);
    let mut status = wow_packet::packets::movement::MovementInfo {
        guid,
        flags: MovementFlag::FORWARD,
        time: 7_777,
        position: dismiss_position,
        ..wow_packet::packets::movement::MovementInfo::default()
    };

    assert!(session.represented_move_dismiss_vehicle_like_cpp(&mut status));
    assert_eq!(session.player_movement_time_like_cpp(), 7_777);
    assert_eq!(session.player_position_like_cpp(), Some(dismiss_position));
    assert_eq!(
        session.player_movement_flags_like_cpp,
        MovementFlag::FORWARD
    );
    assert_eq!(
        session.represented_vehicle_dismiss_movements_like_cpp(),
        &[RepresentedVehicleDismissMovementLikeCpp {
            vehicle_guid,
            sanitized_flags: MovementFlag::FORWARD,
            position: dismiss_position,
            time: 7_777,
        }]
    );
}
#[tokio::test]
async fn move_dismiss_vehicle_handler_copies_status_like_cpp() {
    let (mut session, _, _) = make_session();
    let guid = ObjectGuid::create_player(1, 56);
    let vehicle_guid = test_creature_guid(56_001);
    let canonical = shared_canonical_map_manager();
    let position = Position::new(1.0, 2.0, 3.0, 0.0);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_player_guid(Some(guid));
    session.set_player_map_position_like_cpp(571, position);
    session.player_name = Some("MoveDismissVehicleHandlerTester".to_string());
    session.player_vehicle_seat_flags_like_cpp = Some(wow_data::VEHICLE_SEAT_FLAG_CAN_ATTACK);
    session.player_vehicle_seat_id_like_cpp = Some(1006);
    add_canonical_test_player_on_map(&canonical, guid, position, 571, 0);
    {
        let mut guard = canonical.lock().unwrap();
        guard
            .find_map_mut(571, 0)
            .unwrap()
            .map_mut()
            .get_typed_player_mut(guid)
            .unwrap()
            .unit_mut()
            .subsystems_mut()
            .control
            .set_charmed(vehicle_guid);
    }

    let dismiss_position = Position::new(11.0, 12.0, 13.0, 2.0);
    session
        .handle_move_dismiss_vehicle(wow_packet::packets::vehicle::MoveDismissVehicle {
            status: wow_packet::packets::movement::MovementInfo {
                guid,
                flags: MovementFlag::FORWARD,
                time: 45_678,
                position: dismiss_position,
                ..wow_packet::packets::movement::MovementInfo::default()
            },
        })
        .await;

    assert!(session.player_vehicle_seat_flags_like_cpp.is_none());
    assert_eq!(session.player_movement_time_like_cpp(), 45_678);
    assert_eq!(session.player_position_like_cpp(), Some(dismiss_position));
    assert_eq!(
        session.player_movement_flags_like_cpp,
        MovementFlag::FORWARD
    );
    assert_eq!(
        session.represented_vehicle_dismiss_movements_like_cpp(),
        &[RepresentedVehicleDismissMovementLikeCpp {
            vehicle_guid,
            sanitized_flags: MovementFlag::FORWARD,
            position: dismiss_position,
            time: 45_678,
        }]
    );
}
#[test]
fn represented_move_dismiss_vehicle_rejects_without_charmed_guid_like_cpp() {
    let (mut session, _, _) = make_session();
    let guid = ObjectGuid::create_player(1, 54);
    let canonical = shared_canonical_map_manager();
    let registry = Arc::new(PlayerRegistry::default());
    let position = Position::new(1.0, 2.0, 3.0, 0.0);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_player_guid(Some(guid));
    session.set_player_map_position_like_cpp(571, position);
    session.player_name = Some("MoveDismissVehicleRejectTester".to_string());
    session.player_vehicle_seat_flags_like_cpp = Some(wow_data::VEHICLE_SEAT_FLAG_CAN_CONTROL);
    session.player_vehicle_seat_id_like_cpp = Some(1005);
    session.set_player_registry(Arc::clone(&registry));
    add_canonical_test_player_on_map(&canonical, guid, position, 571, 0);
    session.register_in_player_registry();

    let mut status = wow_packet::packets::movement::MovementInfo {
        guid,
        flags: MovementFlag::FORWARD,
        time: 9_999,
        position: Position::new(4.0, 5.0, 6.0, 1.0),
        ..wow_packet::packets::movement::MovementInfo::default()
    };

    assert!(!session.represented_move_dismiss_vehicle_like_cpp(&mut status));

    assert_eq!(
        session.player_vehicle_seat_flags_like_cpp,
        Some(wow_data::VEHICLE_SEAT_FLAG_CAN_CONTROL)
    );
    assert_eq!(session.player_vehicle_seat_id_like_cpp, Some(1005));
    assert_eq!(session.player_movement_time_like_cpp(), 0);
    assert_eq!(session.player_position_like_cpp(), Some(position));
    assert!(
        session
            .represented_vehicle_dismiss_movements_like_cpp()
            .is_empty()
    );
    let info = registry.party_member(guid).expect("registered player");
    assert!(info.in_vehicle);
    assert_eq!(info.party_member_vehicle_seat, 1005);
}
#[tokio::test]
async fn teleport_to_instance_rejects_access_requirements_before_transfer_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_registry = Arc::new(PlayerRegistry::default());
    let player_guid = ObjectGuid::create_player(1, 790);
    let selected_guid = test_creature_guid(79_001);
    let destination = Position::new(5790.0, 2090.0, 636.0, 3.1);

    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_player_registry(Arc::clone(&player_registry));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "TeleportAccessReject".to_string(),
        Position::new(3700.0, 1500.0, 120.0, 0.0),
        571,
        1,
        1,
        79,
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
    session.represented_raid_difficulty_id_like_cpp = 3;
    session.set_selection_guid_like_cpp(Some(selected_guid));
    install_create_map_active_lock_stores_like_cpp(&mut session, 631, 3, 77, 2);
    install_access_notification_stores_like_cpp(&mut session);
    let mut requirement = access_requirement_like_cpp(631, 3);
    requirement.level_min = 80;
    install_access_requirement_store_like_cpp(&mut session, requirement);

    session.teleport_to(631, destination).await;

    assert_eq!(
        send_rx.try_recv().expect("SMSG_PRINT_NOTIFICATION"),
        PrintNotification {
            notify_text: "You must be at least level 80 to enter.".to_string(),
        }
        .to_bytes()
    );
    assert_eq!(
        send_rx.try_recv().expect("SMSG_TRANSFER_ABORTED"),
        wow_packet::packets::misc::TransferAborted {
            map_id: 631,
            arg: 0,
            map_difficulty_x_condition_id: 0,
            transfer_abort: TRANSFER_ABORT_ERROR_LIKE_CPP,
        }
        .to_bytes()
    );
    assert!(
        send_rx.try_recv().is_err(),
        "C++ Player::TeleportTo returns before SMSG_TRANSFER_PENDING when Map::PlayerCannotEnter rejects"
    );
    assert_eq!(session.pending_teleport_like_cpp(), None);
    assert_ne!(session.state, SessionState::Transfer);
    assert_eq!(
        session.selection_guid_like_cpp(),
        Some(selected_guid),
        "C++ Player::TeleportTo returns before SetSelection(Empty) when PlayerCannotEnter rejects"
    );
    assert_eq!(
        session.player_contested_pvp_timer_like_cpp, 77,
        "C++ Player::TeleportTo returns before ResetContestedPvP when PlayerCannotEnter rejects"
    );
    {
        let manager = canonical.lock().unwrap();
        let player = manager
            .find_map(571, 0)
            .unwrap()
            .map()
            .get_typed_player(player_guid)
            .unwrap();
        assert!(player.has_player_flag(PLAYER_FLAGS_CONTESTED_PVP_LIKE_CPP));
        assert!(
            player
                .unit()
                .has_unit_state(UnitState::ATTACK_PLAYER.bits())
        );
    }
    assert!(
        canonical.lock().unwrap().find_map(631, 0).is_none(),
        "teleport preflight must not create the target instance before the client transfer"
    );
}
