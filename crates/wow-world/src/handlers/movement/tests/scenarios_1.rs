//! Movement handlers regression scenarios, part 1 of 3.
//!
//! Moved out of the movement.rs root under #654; every test is unchanged.

use super::*;

#[test]
fn movement_landing_and_jump_remove_cpp_interruptible_auras() {
    let mut session = make_session();
    session.visible_auras.insert(
        1,
        visible_aura(1, SPELL_AURA_INTERRUPT_FLAG_LANDING_OR_FLIGHT_LIKE_CPP, 0),
    );
    session.visible_auras.insert(
        2,
        visible_aura(2, 0, SPELL_AURA_INTERRUPT_FLAG2_JUMP_LIKE_CPP),
    );
    session.visible_auras.insert(3, visible_aura(3, 0, 0));

    session.apply_movement_side_effects_like_cpp(
        Some(ClientOpcodes::MoveFallLand),
        &MovementInfo::default(),
    );
    assert!(!session.visible_auras.contains_key(&1));
    assert!(session.visible_auras.contains_key(&2));
    assert!(session.visible_auras.contains_key(&3));

    session.apply_movement_side_effects_like_cpp(
        Some(ClientOpcodes::MoveJump),
        &MovementInfo::default(),
    );
    assert!(!session.visible_auras.contains_key(&2));
    assert!(session.visible_auras.contains_key(&3));
    assert_eq!(session.movement_jump_proc_requests_like_cpp(), 1);
}

#[test]
fn movement_stands_sitting_player_and_records_flying_pet_unsummon() {
    let mut session = make_session();
    session.set_player_stand_state_like_cpp(UnitStandStateType::SitChair);
    let mut info = MovementInfo::default();
    info.flags = MovementFlag::FORWARD;

    session.apply_movement_side_effects_like_cpp(Some(ClientOpcodes::MoveSetFly), &info);

    assert_eq!(
        session.player_stand_state_like_cpp(),
        UnitStandStateType::Stand
    );
    assert_eq!(session.temporary_pet_unsummon_requests_like_cpp(), 1);
}

#[test]
fn movement_clears_player_emote_state_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_rx();
    let guid = ObjectGuid::create_player(1, 46);
    session.set_player_guid(Some(guid));
    session.set_player_position_like_cpp(Position::new(1.0, 2.0, 3.0, 0.0));
    session
        .set_player_emote_state_like_cpp(10)
        .expect("emote state update packet");
    assert_eq!(session.player_emote_state_like_cpp(), 10);

    let mut info = MovementInfo::default();
    info.flags = MovementFlag::FORWARD;
    session.apply_movement_side_effects_like_cpp(Some(ClientOpcodes::MoveHeartbeat), &info);

    assert_eq!(session.player_emote_state_like_cpp(), 0);
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::UpdateObject],
        "C++ MovementHandler clears UnitData::EmoteState on accepted player movement"
    );

    session.apply_movement_side_effects_like_cpp(Some(ClientOpcodes::MoveHeartbeat), &info);
    assert_eq!(
        drain_server_opcodes(&send_rx),
        Vec::<ServerOpcodes>::new(),
        "C++ only updates EmoteState when a stateful emote was active"
    );
}

#[tokio::test]
async fn rejected_transport_movement_clears_player_emote_state_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_rx();
    let guid = ObjectGuid::create_player(1, 47);
    session.set_player_guid(Some(guid));
    session.set_player_moved_unit_guid_like_cpp(guid);
    session.set_player_position_like_cpp(Position::new(1.0, 2.0, 3.0, 0.0));
    session
        .set_player_emote_state_like_cpp(10)
        .expect("emote state update packet");

    let info = MovementInfo {
        guid,
        position: Position::new(1.0, 2.0, 3.0, 0.0),
        transport: Some(TransportInfo {
            guid: ObjectGuid::EMPTY,
            x: 76.0,
            y: 0.0,
            z: 0.0,
            o: 0.0,
            seat: 0,
            time: 0,
            prev_time: None,
            vehicle_id: None,
        }),
        ..MovementInfo::default()
    };

    session
        .handle_movement_info_like_cpp(Some(ClientOpcodes::MoveHeartbeat), info)
        .await;

    assert_eq!(session.player_emote_state_like_cpp(), 0);
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::UpdateObject],
        "C++ clears EmoteState before transport movement early returns"
    );
}

#[tokio::test]
async fn accepted_movement_updates_represented_jump_info_like_cpp() {
    let mut session = make_session();
    let guid = ObjectGuid::create_player(1, 44);
    session.set_player_guid(Some(guid));
    session.set_player_position_like_cpp(wow_core::Position::new(1.0, 2.0, 3.0, 0.0));

    let mut info = MovementInfo {
        guid,
        time: 2_000,
        position: wow_core::Position::new(10.0, 20.0, 30.0, 1.0),
        ..MovementInfo::default()
    };
    info.jump.fall_time = 1_234;
    info.jump.z_speed = 6.25;
    info.jump.has_direction = true;
    info.jump.sin_angle = 0.1;
    info.jump.cos_angle = 0.9;
    info.jump.xy_speed = 7.5;

    session
        .handle_movement_info_like_cpp(Some(ClientOpcodes::MoveJump), info)
        .await;

    let jump = session.player_movement_jump_like_cpp();
    assert_eq!(jump.fall_time, 1_234);
    assert_eq!(jump.z_speed, 6.25);
    assert!(jump.has_direction);
    assert_eq!(jump.sin_angle, 0.1);
    assert_eq!(jump.cos_angle, 0.9);
    assert_eq!(jump.xy_speed, 7.5);
}

#[tokio::test]
async fn player_movement_loads_active_grid_before_visibility_like_cpp() {
    let mut session = make_session();
    let guid = ObjectGuid::create_player(1, 45);
    let login_position = Position::new(1.0, 2.0, 3.0, 0.25);
    let same_grid_position = Position::new(10.0, 20.0, 30.0, 1.0);
    let new_grid_position = Position::new(600.0, 20.0, 30.0, 1.0);
    let calls = Arc::new(AtomicUsize::new(0));
    let seen = Arc::new(Mutex::new(Vec::new()));

    let canonical: crate::SharedCanonicalMapManager =
        Arc::new(Mutex::new(wow_map::MapManager::default()));
    let mut canonical_player = wow_entities::Player::new(Some(1), false);
    canonical_player
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(guid);
    canonical_player
        .unit_mut()
        .world_mut()
        .set_map(1, 77)
        .unwrap();
    canonical_player
        .unit_mut()
        .world_mut()
        .relocate(login_position);
    canonical_player
        .unit_mut()
        .world_mut()
        .object_mut()
        .add_to_world();
    canonical
        .lock()
        .unwrap()
        .create_map_entry(
            1,
            77,
            0,
            wow_map::ManagedMapKind::Dungeon {
                has_reset_schedule: false,
            },
        )
        .map_mut()
        .insert_map_object_record(
            wow_entities::MapObjectRecord::new_player(canonical_player).unwrap(),
        )
        .unwrap();
    session.set_canonical_map_manager(canonical);

    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        guid,
        "MovementGridLoader".to_string(),
        login_position,
        1,
        1,
        3,
        10,
        0,
    ));
    session.set_player_moved_unit_guid_like_cpp(guid);
    let calls_for_resolver = Arc::clone(&calls);
    let seen_for_resolver = Arc::clone(&seen);
    let player_grid_loader: crate::session::PlayerGridLoadResolverLikeCpp =
        Arc::new(move |map_id, instance_id, pos| {
            calls_for_resolver.fetch_add(1, Ordering::SeqCst);
            seen_for_resolver
                .lock()
                .unwrap()
                .push((map_id, instance_id, pos));
            PlayerGridLoadOutcomeLikeCpp {
                grid_loaded_now: true,
                creature_records_added: 7,
                legacy_creature_mirrors: 7,
                ..PlayerGridLoadOutcomeLikeCpp::default()
            }
        });
    let area_triggers = session.area_trigger_catalogs_for_test_like_cpp();
    let creature_spawns = session.creature_spawn_catalogs_for_test_like_cpp();
    let progression = session.progression_catalogs_for_test_like_cpp();

    let same_grid = MovementInfo {
        guid,
        flags: MovementFlag::FORWARD,
        position: same_grid_position,
        ..MovementInfo::default()
    };
    session
        .handle_movement_info_with_catalogs_like_cpp(
            &area_triggers,
            &creature_spawns,
            &progression,
            &player_grid_loader,
            Some(ClientOpcodes::MoveHeartbeat),
            same_grid,
        )
        .await;
    assert_eq!(
        calls.load(Ordering::SeqCst),
        0,
        "C++ Map::PlayerRelocation does not EnsureGridLoadedForActiveObject when the player stays in the same grid"
    );

    let new_grid = MovementInfo {
        guid,
        flags: MovementFlag::FORWARD,
        position: new_grid_position,
        ..MovementInfo::default()
    };
    session
        .handle_movement_info_with_catalogs_like_cpp(
            &area_triggers,
            &creature_spawns,
            &progression,
            &player_grid_loader,
            Some(ClientOpcodes::MoveHeartbeat),
            new_grid,
        )
        .await;

    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert_eq!(
        seen.lock().unwrap().as_slice(),
        &[(1, Some(77), new_grid_position)]
    );
}

#[test]
fn movement_fall_land_applies_cpp_base_fall_damage_and_updates_fall_info() {
    let (mut session, send_rx) = make_session_with_send_rx();
    session.set_player_guid(Some(ObjectGuid::create_player(1, 41)));
    session.set_player_health_like_cpp(1_000, 1_000);
    session.set_fall_information_like_cpp(1_200, 120.0);
    let mut info = MovementInfo::default();
    info.position.z = 100.0;
    info.jump.fall_time = 1_500;

    session.apply_movement_side_effects_like_cpp(Some(ClientOpcodes::MoveFallLand), &info);

    let events = session.fall_damage_events_like_cpp();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].damage, 117);
    assert_eq!(events[0].final_damage, 117);
    assert_eq!(session.player_health_like_cpp(), 883);
    let sent = send_rx.try_recv().expect("fall health update");
    let opcode = u16::from_le_bytes([sent[0], sent[1]]);
    assert_eq!(opcode, ServerOpcodes::HealthUpdate as u16);
    let sent = send_rx.try_recv().expect("fall environmental damage log");
    let opcode = u16::from_le_bytes([sent[0], sent[1]]);
    assert_eq!(opcode, ServerOpcodes::EnvironmentalDamageLog as u16);
    assert!(
        send_rx.try_recv().is_err(),
        "non-lethal fall damage must not send a death values update"
    );

    let mut harmless = MovementInfo::default();
    harmless.position.z = 99.0;
    harmless.jump.fall_time = 1_600;
    session.apply_movement_side_effects_like_cpp(Some(ClientOpcodes::MoveFallLand), &harmless);
    assert_eq!(session.fall_damage_events_like_cpp().len(), 1);
}

#[test]
fn movement_fall_land_lethal_damage_sends_player_values_update_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_rx();
    session.set_player_guid(Some(ObjectGuid::create_player(1, 43)));
    session.set_player_health_like_cpp(1_000, 1_000);
    session.set_fall_information_like_cpp(1_200, 300.0);
    let mut info = MovementInfo::default();
    info.position.z = 100.0;
    info.jump.fall_time = 1_500;

    session.apply_movement_side_effects_like_cpp(Some(ClientOpcodes::MoveFallLand), &info);

    let events = session.fall_damage_events_like_cpp();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].damage, 1_000);
    assert_eq!(events[0].final_damage, 1_000);
    assert_eq!(session.player_health_like_cpp(), 0);
    assert!(!session.player_is_alive_like_cpp());
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![
            ServerOpcodes::HealthUpdate,
            ServerOpcodes::EnvironmentalDamageLog,
            ServerOpcodes::UpdateObject,
        ],
        "C++ lethal EnvironmentalDamage goes through Unit::Kill/Player::setDeathState before release/cemetery flows; Rust must publish the zero-health values update, not only the combat log"
    );
}

#[test]
fn movement_fall_damage_applies_cpp_aura_modifiers_and_guards() {
    let mut session = make_session();
    session.set_player_health_like_cpp(1_000, 1_000);
    session.set_fall_information_like_cpp(1_200, 150.0);
    session.visible_auras.insert(
        4,
        fall_aura(4, RepresentedAuraEffectLikeCpp::SafeFall, 10, 1.0),
    );
    session.visible_auras.insert(
        5,
        fall_aura(5, RepresentedAuraEffectLikeCpp::ModifyFallDamagePct, 0, 0.5),
    );
    let mut info = MovementInfo::default();
    info.position.z = 100.0;
    info.jump.fall_time = 1_500;

    session.apply_movement_side_effects_like_cpp(Some(ClientOpcodes::MoveFallLand), &info);

    let events = session.fall_damage_events_like_cpp();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].damage, 238);
    assert_eq!(events[0].final_damage, 238);
    assert_eq!(session.player_health_like_cpp(), 762);

    let mut guarded = make_session();
    guarded.set_player_health_like_cpp(1_000, 1_000);
    guarded.set_fall_information_like_cpp(1_200, 150.0);
    guarded.visible_auras.insert(
        6,
        fall_aura(6, RepresentedAuraEffectLikeCpp::FeatherFall, 0, 1.0),
    );
    guarded.apply_movement_side_effects_like_cpp(Some(ClientOpcodes::MoveFallLand), &info);
    assert!(guarded.fall_damage_events_like_cpp().is_empty());

    let mut god = make_session();
    god.set_player_health_like_cpp(1_000, 1_000);
    god.set_fall_information_like_cpp(1_200, 150.0);
    god.set_player_cheat_god_like_cpp(true);
    god.apply_movement_side_effects_like_cpp(Some(ClientOpcodes::MoveFallLand), &info);
    assert!(god.fall_damage_events_like_cpp().is_empty());

    let mut gm = make_session();
    gm.set_player_health_like_cpp(1_000, 1_000);
    gm.set_fall_information_like_cpp(1_200, 150.0);
    gm.set_player_game_master_like_cpp(true);
    gm.apply_movement_side_effects_like_cpp(Some(ClientOpcodes::MoveFallLand), &info);
    assert!(gm.fall_damage_events_like_cpp().is_empty());

    let mut immune = make_session();
    immune.set_player_health_like_cpp(1_000, 1_000);
    immune.set_fall_information_like_cpp(1_200, 150.0);
    immune.set_player_normal_damage_immune_like_cpp(true);
    immune.apply_movement_side_effects_like_cpp(Some(ClientOpcodes::MoveFallLand), &info);
    assert!(immune.fall_damage_events_like_cpp().is_empty());

    let mut environmental = make_session();
    environmental.set_player_health_like_cpp(1_000, 1_000);
    environmental.set_fall_information_like_cpp(1_200, 150.0);
    environmental.set_player_environmental_damage_immune_like_cpp(true);
    environmental.apply_movement_side_effects_like_cpp(Some(ClientOpcodes::MoveFallLand), &info);
    assert_eq!(environmental.fall_damage_events_like_cpp()[0].damage, 657);
    assert_eq!(
        environmental.fall_damage_events_like_cpp()[0].final_damage,
        0
    );
    assert_eq!(environmental.player_health_like_cpp(), 1_000);
}

#[test]
fn movement_under_map_applies_cpp_void_damage_and_flag() {
    let (mut session, send_rx) = make_session_with_send_rx();
    session.set_player_guid(Some(ObjectGuid::create_player(1, 42)));
    session.set_player_health_like_cpp(1_000, 1_000);
    let mut info = MovementInfo::default();
    info.position.z = -501.0;

    session.apply_movement_side_effects_like_cpp(Some(ClientOpcodes::MoveHeartbeat), &info);

    assert_eq!(session.under_map_damage_events_like_cpp().len(), 1);
    assert_eq!(
        session.under_map_damage_events_like_cpp()[0].min_height,
        crate::map_manager::DEFAULT_MIN_HEIGHT_LIKE_CPP
    );
    assert_eq!(session.player_health_like_cpp(), 0);
    assert!(!session.player_is_alive_like_cpp());
    assert!(session.player_out_of_bounds_like_cpp());
    let sent = send_rx.try_recv().expect("void health update");
    let opcode = u16::from_le_bytes([sent[0], sent[1]]);
    assert_eq!(opcode, ServerOpcodes::HealthUpdate as u16);
    let sent = send_rx.try_recv().expect("void environmental damage log");
    let opcode = u16::from_le_bytes([sent[0], sent[1]]);
    assert_eq!(opcode, ServerOpcodes::EnvironmentalDamageLog as u16);
    let sent = send_rx.try_recv().expect("void death values update");
    let opcode = u16::from_le_bytes([sent[0], sent[1]]);
    assert_eq!(opcode, ServerOpcodes::UpdateObject as u16);

    info.position.z = -499.0;
    session.apply_movement_side_effects_like_cpp(Some(ClientOpcodes::MoveHeartbeat), &info);
    assert!(!session.player_out_of_bounds_like_cpp());
}

#[test]
fn move_init_active_mover_complete_sets_cpp_transport_state() {
    let mut session = make_session();
    let before = crate::session_rules::game_time_ms_like_cpp();

    session.apply_move_init_active_mover_complete_like_cpp(25);

    assert!(
        session.active_player_local_flags_like_cpp()
            & crate::session::PLAYER_LOCAL_FLAG_OVERRIDE_TRANSPORT_SERVER_TIME_LIKE_CPP
            != 0
    );
    assert!(session.active_player_transport_server_time_like_cpp() >= 0);
    assert!(
        session.active_player_transport_server_time_like_cpp()
            <= crate::session_rules::game_time_ms_like_cpp() as i32
    );
    assert!(
        session.active_player_transport_server_time_like_cpp() >= before.saturating_sub(25) as i32
    );
    assert_eq!(session.movement_visibility_refresh_requests_like_cpp(), 0);
}

#[test]
fn movement_ack_helpers_validate_and_apply_cpp_side_effects() {
    let mut session = make_session();
    let guid = ObjectGuid::create_player(1, 42);
    session.set_player_guid(Some(guid));

    let status = MovementInfo {
        guid,
        time: 1_000,
        position: wow_core::Position::new(10.0, 20.0, 30.0, 1.5),
        ..MovementInfo::default()
    };
    let mut ack = wow_packet::packets::movement::MovementAck {
        status: status.clone(),
        ack_index: 7,
    };

    assert!(session.apply_knock_back_ack_like_cpp(ClientOpcodes::MoveKnockBackAck, &mut ack));
    assert_eq!(session.player_position_like_cpp(), Some(status.position));
    assert_eq!(session.movement_ack_events_like_cpp().len(), 1);
    assert!(session.movement_ack_events_like_cpp()[0].accepted);
    assert_eq!(session.movement_ack_events_like_cpp()[0].ack_index, Some(7));
    assert_eq!(
        session.movement_ack_events_like_cpp()[0].adjusted_time,
        Some(session.player_movement_time_like_cpp())
    );

    session.set_player_movement_time_like_cpp(100);
    assert!(session.apply_move_time_skipped_like_cpp(guid, 25));
    assert_eq!(session.player_movement_time_like_cpp(), 125);
    assert_eq!(session.movement_ack_events_like_cpp().len(), 2);
    assert_eq!(
        session.movement_ack_events_like_cpp()[1].opcode,
        ClientOpcodes::MoveTimeSkipped
    );
    assert_eq!(
        session.movement_ack_events_like_cpp()[1].time_skipped,
        Some(25)
    );

    let wrong_guid = ObjectGuid::create_player(1, 43);
    assert!(!session.apply_move_time_skipped_like_cpp(wrong_guid, 25));
    assert_eq!(session.player_movement_time_like_cpp(), 125);
    assert!(!session.movement_ack_events_like_cpp()[2].accepted);
}

#[test]
fn movement_force_ack_helpers_record_cpp_adjusted_time_and_force_id() {
    let mut session = make_session();
    let guid = ObjectGuid::create_player(1, 42);
    let force_guid =
        ObjectGuid::create_world_object(wow_core::guid::HighGuid::GameObject, 0, 1, 0, 0, 9, 88);
    session.set_player_guid(Some(guid));

    let mut ack = wow_packet::packets::movement::MovementAck {
        status: MovementInfo {
            guid,
            time: 1_000,
            position: wow_core::Position::new(10.0, 20.0, 30.0, 1.5),
            ..MovementInfo::default()
        },
        ack_index: 44,
    };
    let force = wow_packet::packets::movement::MovementForce {
        id: force_guid,
        origin: [1.0, 2.0, 3.0],
        direction: [4.0, 5.0, 6.0],
        transport_id: 0,
        magnitude: 7.0,
        unused_910: 0,
        force_type: wow_packet::packets::movement::MovementForceType::Gravity,
    };

    assert!(session.record_apply_movement_force_ack_like_cpp(&mut ack, &force));
    assert_eq!(session.movement_ack_events_like_cpp().len(), 1);
    assert_eq!(
        session.movement_ack_events_like_cpp()[0].opcode,
        ClientOpcodes::MoveApplyMovementForceAck
    );
    assert_eq!(
        session.movement_ack_events_like_cpp()[0].movement_force_id,
        Some(force_guid)
    );
    assert_eq!(
        session.movement_ack_events_like_cpp()[0].movement_force_type,
        Some(1)
    );
    assert!(
        session.movement_ack_events_like_cpp()[0]
            .adjusted_time
            .is_some()
    );

    assert!(session.record_remove_movement_force_ack_like_cpp(&mut ack, force_guid));
    assert_eq!(
        session.movement_ack_events_like_cpp()[1].opcode,
        ClientOpcodes::MoveRemoveMovementForceAck
    );
    assert_eq!(
        session.movement_ack_events_like_cpp()[1].movement_force_id,
        Some(force_guid)
    );
}

#[test]
fn movement_speed_ack_matches_cpp_counters_and_anticheat() {
    let mut session = make_session();
    let guid = ObjectGuid::create_player(1, 42);
    session.set_player_guid(Some(guid));
    let mut ack = wow_packet::packets::movement::MovementAck {
        status: MovementInfo {
            guid,
            time: 1_000,
            position: wow_core::Position::new(10.0, 20.0, 30.0, 1.5),
            ..MovementInfo::default()
        },
        ack_index: 10,
    };

    session.set_player_movement_speed_rate_like_cpp(UnitMoveTypeLikeCpp::Run, 1.0);
    session.set_forced_speed_changes_like_cpp(UnitMoveTypeLikeCpp::Run, 2);
    assert!(session.handle_force_speed_change_ack_like_cpp(
        ClientOpcodes::MoveForceRunSpeedChangeAck,
        &mut ack,
        1.0,
    ));
    let first = session.movement_speed_ack_events_like_cpp().last().unwrap();
    assert_eq!(first.action, MovementSpeedAckActionLikeCpp::SkippedPending);
    assert_eq!(first.remaining_forced_changes, Some(1));
    assert!(!session.is_disconnecting());

    assert!(session.handle_force_speed_change_ack_like_cpp(
        ClientOpcodes::MoveForceRunSpeedChangeAck,
        &mut ack,
        6.0,
    ));
    let corrected = session.movement_speed_ack_events_like_cpp().last().unwrap();
    assert_eq!(corrected.expected_speed, Some(7.0));
    assert_eq!(corrected.action, MovementSpeedAckActionLikeCpp::Corrected);
    assert!(!session.is_disconnecting());

    session.set_player_on_transport_like_cpp(true);
    assert!(session.handle_force_speed_change_ack_like_cpp(
        ClientOpcodes::MoveForceRunSpeedChangeAck,
        &mut ack,
        8.0,
    ));
    let transport = session.movement_speed_ack_events_like_cpp().last().unwrap();
    assert_eq!(transport.action, MovementSpeedAckActionLikeCpp::Accepted);
    assert!(!session.is_disconnecting());

    session.set_player_on_transport_like_cpp(false);
    assert!(!session.handle_force_speed_change_ack_like_cpp(
        ClientOpcodes::MoveForceRunSpeedChangeAck,
        &mut ack,
        8.0,
    ));
    let kicked = session.movement_speed_ack_events_like_cpp().last().unwrap();
    assert_eq!(kicked.action, MovementSpeedAckActionLikeCpp::Kicked);
    assert!(session.is_disconnecting());
}

#[tokio::test]
async fn movement_speed_ack_correction_matches_legacy_no_resync_packet() {
    let (mut session, send_rx) = make_session_with_send_rx();
    let guid = ObjectGuid::create_player(1, 42);
    session.set_player_guid(Some(guid));
    session.set_player_movement_speed_rate_like_cpp(UnitMoveTypeLikeCpp::Run, 1.0);

    session
        .handle_movement_speed_ack(
            ClientOpcodes::MoveForceRunSpeedChangeAck,
            wow_packet::packets::movement::MovementSpeedAck {
                ack: wow_packet::packets::movement::MovementAck {
                    status: MovementInfo {
                        guid,
                        time: 1_000,
                        position: wow_core::Position::new(10.0, 20.0, 30.0, 1.5),
                        ..MovementInfo::default()
                    },
                    ack_index: 10,
                },
                speed: 6.0,
            },
        )
        .await;

    let corrected = session.movement_speed_ack_events_like_cpp().last().unwrap();
    assert_eq!(corrected.expected_speed, Some(7.0));
    assert_eq!(corrected.action, MovementSpeedAckActionLikeCpp::Corrected);
    assert!(
        send_rx.try_recv().is_err(),
        "legacy C++ calls SetSpeedRate(GetSpeedRate()), but Unit::SetSpeedRate returns early when the rate is unchanged"
    );
}

#[test]
fn movement_force_magnitude_ack_matches_cpp_counter_validation() {
    let mut session = make_session();
    let guid = ObjectGuid::create_player(1, 42);
    session.set_player_guid(Some(guid));
    session.set_movement_force_mod_magnitude_changes_like_cpp(1);
    session.set_movement_force_mod_magnitude_like_cpp(1.25);
    let mut ack = wow_packet::packets::movement::MovementAck {
        status: MovementInfo {
            guid,
            time: 1_000,
            position: wow_core::Position::new(10.0, 20.0, 30.0, 1.5),
            ..MovementInfo::default()
        },
        ack_index: 11,
    };

    assert!(session.handle_movement_force_mod_magnitude_ack_like_cpp(
        ClientOpcodes::MoveSetModMovementForceMagnitudeAck,
        &mut ack,
        1.25,
    ));
    let accepted = session.movement_speed_ack_events_like_cpp().last().unwrap();
    assert_eq!(accepted.action, MovementSpeedAckActionLikeCpp::Accepted);
    assert_eq!(accepted.remaining_forced_changes, Some(0));

    session.set_movement_force_mod_magnitude_changes_like_cpp(1);
    assert!(!session.handle_movement_force_mod_magnitude_ack_like_cpp(
        ClientOpcodes::MoveSetModMovementForceMagnitudeAck,
        &mut ack,
        1.5,
    ));
    let kicked = session.movement_speed_ack_events_like_cpp().last().unwrap();
    assert_eq!(kicked.action, MovementSpeedAckActionLikeCpp::Kicked);
    assert!(session.is_disconnecting());
}

#[test]
fn move_spline_done_taxi_final_cleanup_matches_cpp_represented_side_effects() {
    let mut session = make_session();
    let guid = ObjectGuid::create_player(1, 42);
    session.set_player_guid(Some(guid));
    session.set_player_position_like_cpp(wow_core::Position::new(1.0, 2.0, 30.0, 0.5));
    session.set_fall_information_like_cpp(1_200, 120.0);
    session.set_taxi_destinations_like_cpp(vec![100]);
    session.set_taxi_cleanup_state_like_cpp(
        UnitFlags::REMOVE_CLIENT_CONTROL | UnitFlags::ON_TAXI,
        true,
    );
    session.set_player_pvp_hostile_like_cpp(true);

    let mut status = MovementInfo {
        guid,
        time: 1_000,
        position: wow_core::Position::new(1.0, 2.0, 30.0, 0.5),
        ..MovementInfo::default()
    };

    let action = session.handle_move_spline_done_taxi_like_cpp(&mut status, 55);
    assert_eq!(action, MoveSplineDoneTaxiActionLikeCpp::FinalCleanup);
    assert!(session.taxi_destinations_like_cpp().is_empty());
    assert!(!session.taxi_mounted_like_cpp());
    assert_eq!(session.taxi_unit_flags_like_cpp(), UnitFlags::empty());
    assert_eq!(session.fall_information_like_cpp(), (0, 30.0));
    let event = session
        .move_spline_done_taxi_events_like_cpp()
        .last()
        .unwrap();
    assert!(event.honorless_target_cast);
}

#[test]
fn move_spline_done_taxi_far_teleport_matches_cpp_represented_branch() {
    let mut session = make_session();
    let guid = ObjectGuid::create_player(1, 42);
    session.set_player_guid(Some(guid));
    session.set_player_map_position_like_cpp(0, wow_core::Position::new(1.0, 2.0, 3.0, 1.0));
    session.set_taxi_destinations_like_cpp(vec![10, 20]);
    session.set_taxi_node_map_id_like_cpp(20, 1);
    session.set_taxi_flight_state_like_cpp(
        RepresentedTaxiFlightNodeLikeCpp {
            map_id: 0,
            position: wow_core::Position::new(5.0, 6.0, 7.0, 1.0),
            teleport_flag: false,
        },
        Some(RepresentedTaxiFlightNodeLikeCpp {
            map_id: 1,
            position: wow_core::Position::new(50.0, 60.0, 70.0, 1.0),
            teleport_flag: false,
        }),
    );

    let mut status = MovementInfo {
        guid,
        time: 1_000,
        position: wow_core::Position::new(1.0, 2.0, 3.0, 1.0),
        ..MovementInfo::default()
    };

    let action = session.handle_move_spline_done_taxi_like_cpp(&mut status, 56);
    assert_eq!(action, MoveSplineDoneTaxiActionLikeCpp::TeleportRequested);
    assert_eq!(session.player_map_id_like_cpp(), 1);
    assert_eq!(
        session.player_position_like_cpp().unwrap(),
        wow_core::Position::new(50.0, 60.0, 70.0, 1.0)
    );
    let event = session
        .move_spline_done_taxi_events_like_cpp()
        .last()
        .unwrap();
    assert_eq!(event.destination_node_id, Some(20));
    assert_eq!(event.teleport_map_id, Some(1));
    assert_eq!(
        event.teleport_position,
        Some(wow_core::Position::new(50.0, 60.0, 70.0, 1.0))
    );
}

#[test]
fn move_teleport_ack_applies_near_teleport_cpp_side_effects() {
    let mut session = make_session();
    let guid = ObjectGuid::create_player(1, 42);
    let destination = wow_core::Position::new(12.0, 13.0, 14.0, 1.5);
    session.set_player_guid(Some(guid));
    session.set_player_map_position_like_cpp(0, wow_core::Position::new(1.0, 2.0, 3.0, 0.5));
    session.set_fall_information_like_cpp(1_200, 80.0);
    session.set_player_zone_area_like_cpp(10, 11);
    session.set_player_pvp_state_like_cpp(true, false, false);
    session.set_near_teleport_pending_like_cpp(true, Some((0, destination)), Some((20, 21)));

    let action = session.handle_move_teleport_ack_like_cpp(guid, 77, 1_234);
    assert_eq!(action, MoveTeleportAckActionLikeCpp::Accepted);
    assert!(!session.near_teleport_pending_like_cpp());
    assert_eq!(session.player_position_like_cpp(), Some(destination));
    assert_eq!(session.fall_information_like_cpp(), (0, 14.0));
    assert_eq!(session.player_zone_area_like_cpp(), Some((20, 21)));
    assert_eq!(session.temporary_pet_resummon_requests_like_cpp(), 1);
    assert_eq!(session.delayed_operations_processed_like_cpp(), 1);

    let event = session.move_teleport_ack_events_like_cpp().last().unwrap();
    assert_eq!(event.action, MoveTeleportAckActionLikeCpp::Accepted);
    assert_eq!(event.old_zone_id, Some(10));
    assert_eq!(event.new_zone_id, Some(20));
    assert!(event.honorless_target_cast);
    assert!(!event.pvp_disabled);
    assert!(event.pet_resummon_requested);
    assert!(event.delayed_operations_processed);
}

#[test]
fn move_teleport_ack_ignores_wrong_or_missing_near_teleport_like_cpp() {
    let mut session = make_session();
    let guid = ObjectGuid::create_player(1, 42);
    let other_guid = ObjectGuid::create_player(1, 43);
    let original_position = wow_core::Position::new(1.0, 2.0, 3.0, 0.5);
    session.set_player_guid(Some(guid));
    session.set_player_map_position_like_cpp(0, original_position);

    let action = session.handle_move_teleport_ack_like_cpp(guid, 1, 2);
    assert_eq!(action, MoveTeleportAckActionLikeCpp::NotBeingTeleportedNear);
    assert_eq!(session.player_position_like_cpp(), Some(original_position));

    session.set_near_teleport_pending_like_cpp(
        true,
        Some((0, wow_core::Position::new(9.0, 9.0, 9.0, 0.0))),
        Some((30, 31)),
    );
    let action = session.handle_move_teleport_ack_like_cpp(other_guid, 3, 4);
    assert_eq!(action, MoveTeleportAckActionLikeCpp::WrongMover);
    assert!(session.near_teleport_pending_like_cpp());
    assert_eq!(session.player_position_like_cpp(), Some(original_position));
    assert_eq!(session.temporary_pet_resummon_requests_like_cpp(), 0);
    assert_eq!(session.delayed_operations_processed_like_cpp(), 0);
}

#[test]
fn validate_movement_info_sanitizes_representable_cpp_flag_violations() {
    let session = make_session();
    let mut info = MovementInfo {
        flags: MovementFlag::FORWARD
            | MovementFlag::BACKWARD
            | MovementFlag::LEFT
            | MovementFlag::RIGHT
            | MovementFlag::ASCENDING
            | MovementFlag::DESCENDING
            | MovementFlag::HOVER
            | MovementFlag::WATER_WALK
            | MovementFlag::FALLING_SLOW
            | MovementFlag::FLYING
            | MovementFlag::CAN_FLY
            | MovementFlag::DISABLE_GRAVITY
            | MovementFlag::FALLING
            | MovementFlag::SPLINE_ELEVATION,
        step_up_start_elevation: 0.0,
        ..MovementInfo::default()
    };

    let removed = session.sanitize_movement_info_flags_represented_like_cpp(&mut info);
    assert!(removed.contains(MovementFlag::FORWARD | MovementFlag::BACKWARD));
    assert!(removed.contains(MovementFlag::LEFT | MovementFlag::RIGHT));
    assert!(removed.contains(MovementFlag::ASCENDING | MovementFlag::DESCENDING));
    assert!(removed.contains(MovementFlag::HOVER));
    assert!(removed.contains(MovementFlag::WATER_WALK));
    assert!(removed.contains(MovementFlag::FALLING_SLOW));
    assert!(removed.contains(MovementFlag::FLYING | MovementFlag::CAN_FLY));
    assert!(removed.contains(MovementFlag::FALLING));
    assert!(removed.contains(MovementFlag::SPLINE_ELEVATION));
    assert_eq!(info.flags, MovementFlag::DISABLE_GRAVITY);
}
