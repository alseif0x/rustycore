//! Session scenarios exercising the represented movement responsibility.
//!
//! Split out of session_tests.rs under #626; assertions and registrations
//! are unchanged and the shared fixtures stay in the parent module.

use super::*;

#[test]
fn movement_counter_post_increments_and_resets_like_cpp() {
    // C++ Unit::m_movementCounter: post-increment per movement-control packet, reset to
    // 0 in SendInitialPacketsBeforeAddToMap on a non-seamless add. #NEXT.R8.ENTITIES.1229.
    let (mut session, _, _) = make_session();
    assert_eq!(session.next_movement_counter_like_cpp(), Some(0));
    assert_eq!(session.next_movement_counter_like_cpp(), Some(1));
    assert_eq!(session.next_movement_counter_like_cpp(), Some(2));
    session.reset_movement_counter_like_cpp();
    assert_eq!(session.next_movement_counter_like_cpp(), Some(0));
    assert_eq!(session.next_movement_counter_like_cpp(), Some(1));
}
#[test]
fn create_map_player_context_group_owner_falls_back_to_leader_like_cpp() {
    let (mut session, _, _) = make_session();
    let leader = ObjectGuid::create_player(1, 42);
    let group_registry = Arc::new(GroupRegistry::default());
    let mut group = GroupInfo::new(leader);
    group.raid_difficulty_id = 15;
    group.legacy_raid_difficulty_id = 4;
    let group_guid = group.group_guid;
    group_registry.register_group_like_cpp(group_guid, group);
    let map_entry =
        represented_map_entry_for_create_map_context_like_cpp(631, wow_data::map::MAP_RAID);

    session.group_guid = Some(group_guid);
    session.set_group_registry(group_registry, Arc::new(PendingInvites::default()));
    install_create_map_difficulty_stores_like_cpp(
        &mut session,
        631,
        15,
        DifficultyFlags::CAN_SELECT | DifficultyFlags::DEFAULT,
    );

    let context = session
        .create_map_player_context_like_cpp(631, map_entry, leader)
        .expect("test Player difficulty owner resolves");
    let group = context.group.unwrap();

    assert_eq!(group.difficulty_id, 15);
    assert_eq!(
        group.recent_instance_owner_guid_counter,
        leader.counter() as u64
    );
    assert_eq!(group.recent_instance_id, 0);
}
#[test]
fn create_map_player_context_missing_default_raid_metadata_falls_back_to_legacy_like_cpp() {
    let (mut session, _, _) = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let map_entry =
        represented_map_entry_for_create_map_context_like_cpp(249, wow_data::map::MAP_RAID);

    session.represented_raid_difficulty_id_like_cpp = 15;
    session.represented_legacy_raid_difficulty_id_like_cpp = 4;

    let context = session
        .create_map_player_context_like_cpp(249, map_entry, player_guid)
        .expect("test Player difficulty owner resolves");

    assert_eq!(context.player_difficulty_id, 4);
}
#[tokio::test]
async fn send_if_visible_monster_move_rejects_commands_queued_before_enter_world_cutoff_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let source_guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 777, 1012);
    let packet_bytes = vec![0xD4, 0x2D, 0xAA];
    let manager = shared_map_manager();
    manager.write().unwrap().add_creature(
        571,
        0,
        0,
        0,
        crate::map_manager::WorldCreature::new(
            source_guid,
            777,
            Position::ZERO,
            100,
            80,
            1,
            2,
            0.0,
            1,
            35,
            0,
            0,
        ),
    );
    session.state = SessionState::LoggedIn;
    session.set_map_manager(Arc::clone(&manager));
    session.set_player_map_position_like_cpp(571, Position::ZERO);
    session.client_visible_guids_like_cpp.insert(source_guid);
    let enter_world_cutoff = Instant::now();
    session.suppress_creature_movement_queued_at_or_before_like_cpp = Some(enter_world_cutoff);

    session
        .session_command_tx()
        .try_send(SessionCommand::SendIfVisibleLikeCpp(
            SendIfVisibleLikeCppCommand {
                queued_at: enter_world_cutoff - Duration::from_millis(1),
                source_guid,
                map_id: 571,
                instance_id: 0,
                packet_bytes: packet_bytes.clone(),
            },
        ))
        .expect("command queued");
    session
        .process_represented_session_commands_like_cpp()
        .await;
    assert!(
        send_rx.try_recv().is_err(),
        "C++ baseline has no SMSG_ON_MONSTER_MOVE in the initial enter-world packet burst"
    );

    session
        .session_command_tx()
        .try_send(SessionCommand::SendIfVisibleLikeCpp(
            SendIfVisibleLikeCppCommand {
                queued_at: enter_world_cutoff + Duration::from_millis(1),
                source_guid,
                map_id: 571,
                instance_id: 0,
                packet_bytes: packet_bytes.clone(),
            },
        ))
        .expect("command queued");
    session
        .process_represented_session_commands_like_cpp()
        .await;
    assert_eq!(
        send_rx
            .try_recv()
            .expect("movement queued after enter-world cutoff delivers"),
        packet_bytes
    );
    assert_eq!(
        session.suppress_creature_movement_queued_at_or_before_like_cpp,
        Some(enter_world_cutoff)
    );
    assert!(send_rx.try_recv().is_err(), "no extra packets");
}
#[test]
fn summon_object_slot_session_resolver_cleans_old_slot_and_creates_with_fallback_like_cpp() {
    let (mut session, _, _) = make_session();
    let spell_id = 701_u32;
    let template_entry = 9002_u32;
    let slot = 0_usize;
    let player_guid = ObjectGuid::create_player(1, 7003);
    let canonical = shared_canonical_map_manager();
    let player_position = Position::new(40.0, 50.0, 60.0, 0.0);
    insert_test_player_into_canonical_map_like_cpp(
        &canonical,
        player_guid,
        571,
        0,
        player_position,
    );
    let template_store = summon_go_template_store_like_cpp(template_entry);
    let old_guid = {
        let lifecycle_record = wow_data::gameobject_template_lifecycle_record_like_cpp(
            template_store.get(template_entry).expect("template"),
        );
        let mut manager = canonical.lock().unwrap();
        let managed = manager.find_map_mut(571, 0).expect("map");
        let old = managed
            .map_mut()
            .gameobject_summon_object_for_owner_slot_like_cpp(
                player_guid,
                slot,
                spell_id,
                lifecycle_record,
                Position::new(41.0, 50.0, 60.0, 0.0),
                1_000,
            );
        assert_eq!(
            old.status,
            wow_map::map::GameObjectSummonObjectForOwnerSlotStatusLikeCpp::CreatedAddedAndSlotted
        );
        old.guid.expect("old slot GO")
    };
    session.set_player_guid(Some(player_guid));
    session.set_player_map_position_like_cpp(571, player_position);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_gameobject_template_lifecycle_store(Arc::clone(&template_store));
    session.set_spell_misc_store(Arc::new(wow_data::SpellMiscStore::from_entries([
        summon_go_spell_misc_entry_like_cpp(spell_id, 78),
    ])));
    session.set_spell_duration_store(Arc::new(wow_data::SpellDurationStore::from_entries([
        wow_data::SpellDurationEntry {
            id: 78,
            duration: 8_000,
            duration_per_level: 0,
            max_duration: 0,
        },
    ])));

    let outcome = session
        .apply_effect_summon_object_slot_like_cpp(
            i32::try_from(spell_id).unwrap(),
            &summon_object_slot_effect_like_cpp(i32::try_from(template_entry).unwrap(), 0),
            &SpellTargetData::default(),
        )
        .expect("slot effect should return represented outcome");

    assert_eq!(
        outcome.status,
        ApplyEffectSummonObjectSlotSessionStatusLikeCpp::MapResolved
    );
    assert_eq!(outcome.slot, Some(slot));
    assert_eq!(outcome.template_entry, Some(template_entry));
    assert_eq!(outcome.duration_ms, Some(8_000));
    assert!(!outcome.explicit_destination_used);
    assert!(outcome.close_point_fallback_represented);
    let cleanup = outcome.cleanup_outcome.expect("old slot cleanup");
    assert_eq!(cleanup.slot_guid_before, old_guid);
    assert!(cleanup.slot_had_guid);
    assert!(cleanup.recast_spell_id_cleared);
    assert!(cleanup.slot_cleared);
    let map_outcome = outcome.map_outcome.expect("new slot summon");
    assert_eq!(
        map_outcome.status,
        wow_map::map::GameObjectSummonObjectForOwnerSlotStatusLikeCpp::CreatedAddedAndSlotted
    );
    assert_eq!(map_outcome.respawn_time_secs, Some(8));
    let new_guid = map_outcome.guid.expect("new slot GO");
    assert_ne!(new_guid, old_guid);
    let manager = canonical.lock().unwrap();
    let managed = manager.find_map(571, 0).expect("map");
    let owner = managed
        .map()
        .get_typed_player(player_guid)
        .expect("player remains owner");
    assert_eq!(
        owner.unit().subsystems().control.gameobject_slots[slot],
        new_guid
    );
    let go = managed
        .map()
        .get_typed_game_object(new_guid)
        .expect("summoned GO should be in canonical map");
    assert_eq!(
        go.world().position(),
        Position::new(
            40.0 + wow_map::map::DEFAULT_PLAYER_BOUNDING_RADIUS_LIKE_CPP,
            50.0,
            60.0,
            0.0
        )
    );
    assert_eq!(go.spell_id(), spell_id);
}
#[tokio::test]
async fn teleport_units_uses_cross_map_db_destination_for_player_like_cpp() {
    let (mut session, _, _send_rx) = make_session();
    session.set_map_store(crate::teleport_test_fixtures::world_maps([571, 1]));
    let spell_id = 717_i32;
    let player_guid = ObjectGuid::create_player(1, 7023);
    let player_position = Position::new(260.0, 360.0, 52.0, 0.5);
    let db_destination = Position::new(12.0, 24.0, 36.0, 0.0);
    let canonical = shared_canonical_map_manager();
    let teleport_effect = wow_data::SpellEffectInfo {
        effect_index: 0,
        effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_TELEPORT_UNITS,
        implicit_target_1: wow_data::spell::implicit_targets::TARGET_DEST_DB,
        ..Default::default()
    };
    let spell_info = teleport_units_spell_info_like_cpp(spell_id, vec![teleport_effect.clone()]);

    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "TeleportTarget".to_string(),
        player_position,
        571,
        1,
        1,
        80,
        0,
    ));
    add_canonical_test_player_on_map(&canonical, player_guid, player_position, 571, 0);
    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(spell_id, spell_info.clone());
    session.set_spell_store(Arc::new(spell_store));
    let mut target_spell_store = wow_data::SpellStore::new();
    target_spell_store.insert(spell_id, spell_info);
    session.set_spell_target_position_store(Arc::new(
        wow_data::SpellTargetPositionStoreLikeCpp::from_rows_like_cpp(
            [wow_data::SpellTargetPositionRowLikeCpp {
                spell_id: spell_id as u32,
                effect_index: teleport_effect.effect_index,
                target_map_id: 1,
                x: db_destination.x,
                y: db_destination.y,
                z: db_destination.z,
                orientation: Some(db_destination.orientation),
            }],
            &target_spell_store,
            |map_id| matches!(map_id, 1 | 571),
        ),
    ));

    session
        .execute_spell_with_visual_and_target_data(
            spell_id,
            player_guid,
            ObjectGuid::EMPTY,
            wow_packet::packets::spell::SpellCastVisual {
                spell_visual_id: 717,
                script_visual_id: 0,
            },
            SpellTargetData::default(),
        )
        .await
        .expect("TARGET_DEST_DB teleport should execute");

    assert_eq!(
        session.pending_teleport_like_cpp(),
        Some((
            1,
            Position::new(
                db_destination.x,
                db_destination.y,
                db_destination.z,
                player_position.orientation
            )
        )),
        "C++ EffectTeleportUnits accepts cross-map TARGET_DEST_DB, fills missing orientation from unitTarget, and calls Player::TeleportTo"
    );
    assert_eq!(session.state, SessionState::Transfer);
}
#[tokio::test]
async fn teleport_units_target_dest_home_uses_represented_homebind_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    session.set_map_store(crate::teleport_test_fixtures::world_maps([571, 1]));
    let spell_id = 8690_i32;
    let player_guid = ObjectGuid::create_player(1, 7040);
    let player_position = Position::new(300.0, 400.0, 60.0, 0.5);
    let home_position = Position::new(40.0, 50.0, 60.0, 1.25);
    let canonical = shared_canonical_map_manager();
    let hearth_effect = wow_data::SpellEffectInfo {
        effect_index: 0,
        effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_TELEPORT_UNITS,
        implicit_target_1: wow_data::spell::implicit_targets::TARGET_DEST_HOME,
        ..Default::default()
    };
    let spell_info = teleport_units_spell_info_like_cpp(spell_id, vec![hearth_effect]);

    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "HearthHome".to_string(),
        player_position,
        571,
        1,
        1,
        80,
        0,
    ));
    let _ = session.set_represented_homebind_like_cpp(RepresentedHomebindLikeCpp {
        map_id: 1,
        area_id: 1519,
        position: home_position,
    });
    add_canonical_test_player_on_map(&canonical, player_guid, player_position, 571, 0);
    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(spell_id, spell_info);
    session.set_spell_store(Arc::new(spell_store));

    session
        .execute_spell_with_visual_and_target_data(
            spell_id,
            player_guid,
            ObjectGuid::EMPTY,
            wow_packet::packets::spell::SpellCastVisual {
                spell_visual_id: 8690,
                script_visual_id: 0,
            },
            SpellTargetData {
                flags: 0x0000_0040,
                dst_location: Some(wow_packet::packets::spell::TargetLocation {
                    transport: ObjectGuid::EMPTY,
                    position: Position::new(999.0, 998.0, 997.0, 0.0),
                }),
                map_id: Some(571),
                ..SpellTargetData::default()
            },
        )
        .await
        .expect("TARGET_DEST_HOME hearthstone teleport should execute");

    let bytes = send_rx.try_recv().expect("hearthstone SpellGo");
    let packet_target = decode_spell_go_target_data_like_cpp(&bytes, spell_id);
    assert_ne!(packet_target.flags & 0x0000_0040, 0);
    assert_eq!(
        packet_target
            .dst_location
            .expect("resolved home destination")
            .position,
        Position::new(home_position.x, home_position.y, home_position.z, 0.0),
        "C++ SelectSpellTargets resolves TARGET_DEST_HOME before SpellGo"
    );
    assert_eq!(packet_target.map_id, None);
    assert_eq!(
        session.pending_teleport_like_cpp(),
        Some((1, home_position))
    );
    assert_eq!(session.state, SessionState::Transfer);
}
#[tokio::test]
async fn teleport_units_target_dest_home_without_homebind_is_noop_boundary_like_cpp() {
    let (mut session, _, _send_rx) = make_session();
    let spell_id = 8690_i32;
    let player_guid = ObjectGuid::create_player(1, 7041);
    let player_position = Position::new(301.0, 401.0, 61.0, 0.5);
    let canonical = shared_canonical_map_manager();
    let hearth_effect = wow_data::SpellEffectInfo {
        effect_index: 0,
        effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_TELEPORT_UNITS,
        implicit_target_1: wow_data::spell::implicit_targets::TARGET_DEST_HOME,
        ..Default::default()
    };
    let spell_info = teleport_units_spell_info_like_cpp(spell_id, vec![hearth_effect]);

    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "HearthNoHome".to_string(),
        player_position,
        571,
        1,
        1,
        80,
        0,
    ));
    add_canonical_test_player_on_map(&canonical, player_guid, player_position, 571, 0);
    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(spell_id, spell_info);
    session.set_spell_store(Arc::new(spell_store));

    session
        .execute_spell_with_visual_and_target_data(
            spell_id,
            player_guid,
            ObjectGuid::EMPTY,
            wow_packet::packets::spell::SpellCastVisual {
                spell_visual_id: 8690,
                script_visual_id: 0,
            },
            SpellTargetData::default(),
        )
        .await
        .expect("missing represented homebind keeps current bounded no-op");

    assert_eq!(session.pending_teleport_like_cpp(), None);
    assert_ne!(session.state, SessionState::Transfer);
}
#[tokio::test]
async fn teleport_units_without_destination_is_noop_like_cpp() {
    let (mut session, _, _send_rx) = make_session();
    let spell_id = 718_i32;
    let player_guid = ObjectGuid::create_player(1, 7024);
    let player_position = Position::new(270.0, 370.0, 54.0, 0.625);
    let canonical = shared_canonical_map_manager();
    let teleport_effect = wow_data::SpellEffectInfo {
        effect_index: 0,
        effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_TELEPORT_UNITS,
        ..Default::default()
    };
    let spell_info = teleport_units_spell_info_like_cpp(spell_id, vec![teleport_effect]);

    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "TeleportNoDst".to_string(),
        player_position,
        571,
        1,
        1,
        80,
        0,
    ));
    add_canonical_test_player_on_map(&canonical, player_guid, player_position, 571, 0);
    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(spell_id, spell_info);
    session.set_spell_store(Arc::new(spell_store));

    session
        .execute_spell_with_visual_and_target_data(
            spell_id,
            player_guid,
            ObjectGuid::EMPTY,
            wow_packet::packets::spell::SpellCastVisual {
                spell_visual_id: 718,
                script_visual_id: 0,
            },
            SpellTargetData::default(),
        )
        .await
        .expect("destination-less teleport should execute as effect no-op");

    assert_eq!(
        session.pending_teleport_like_cpp(),
        None,
        "C++ EffectTeleportUnits returns when m_targets.HasDst() is false"
    );
    assert_ne!(session.state, SessionState::Transfer);
}
#[tokio::test]
async fn primary_teleport_units_uses_explicit_destination_like_cpp() {
    let (mut session, _, _send_rx) = make_session();
    session.set_map_store(crate::teleport_test_fixtures::world_maps([571, 1]));
    let spell_id = 730_i32;
    let player_guid = ObjectGuid::create_player(1, 7029);
    let player_position = Position::new(271.0, 371.0, 55.0, 0.75);
    let destination = Position::new(42.0, 84.0, 126.0, 0.0);
    let canonical = shared_canonical_map_manager();

    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "PrimaryTeleportTarget".to_string(),
        player_position,
        571,
        1,
        1,
        80,
        0,
    ));
    add_canonical_test_player_on_map(&canonical, player_guid, player_position, 571, 0);
    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        spell_id,
        wow_data::SpellInfo {
            spell_id,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: wow_data::spell::spell_effect_types::SPELL_EFFECT_TELEPORT_UNITS,
            effect_base_points: 0,
            effect_bonus_coefficient: 0.0,
            aura_type: None,
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: Vec::new(),
        },
    );
    session.set_spell_store(Arc::new(spell_store));

    session
        .execute_spell_with_visual_and_target_data(
            spell_id,
            player_guid,
            ObjectGuid::EMPTY,
            wow_packet::packets::spell::SpellCastVisual {
                spell_visual_id: 730,
                script_visual_id: 0,
            },
            SpellTargetData {
                flags: 0x2,
                unit: player_guid,
                dst_location: Some(wow_packet::packets::spell::TargetLocation {
                    transport: ObjectGuid::EMPTY,
                    position: destination,
                }),
                map_id: Some(1),
                ..SpellTargetData::default()
            },
        )
        .await
        .expect("represented primary teleport should execute");

    assert_eq!(
        session.pending_teleport_like_cpp(),
        Some((
            1,
            Position::new(
                destination.x,
                destination.y,
                destination.z,
                player_position.orientation
            )
        )),
        "C++ EffectTeleportUnits consumes existing m_targets destination and fills missing orientation from unitTarget"
    );
    assert_eq!(session.state, SessionState::Transfer);
}
#[test]
fn represented_player_speed_change_propagates_to_active_pet_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    session.set_player_guid(Some(ObjectGuid::create_player(1, 42)));
    let pet_guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 7_001, 9_001);
    session.client_visible_guids_like_cpp.insert(pet_guid);
    session.set_represented_pet_mode_state_like_cpp(
        Some(pet_guid),
        wow_packet::packets::pet::REACT_DEFENSIVE_LIKE_CPP,
        wow_packet::packets::pet::COMMAND_FOLLOW_LIKE_CPP,
    );

    session.set_player_movement_speed_rate_and_notify_like_cpp(UnitMoveTypeLikeCpp::Run, 2.0);

    assert_eq!(
        session.represented_pet_movement_speed_rate_like_cpp(UnitMoveTypeLikeCpp::Run),
        2.0
    );
    assert_eq!(session.represented_pet_speed_propagations_like_cpp(), 1);
    let opcodes = drain_server_opcodes(&send_rx);
    assert!(
        opcodes.contains(&ServerOpcodes::MoveSplineSetRunSpeed),
        "C++ Pet::SetSpeedRate sends SMSG_MOVE_SPLINE_SET_RUN_SPEED for the pet"
    );
    assert!(
        opcodes.contains(&ServerOpcodes::MoveSetRunSpeed),
        "C++ player SetSpeedRate still sends the player-controlled mover speed packet"
    );

    session.set_player_movement_speed_rate_and_notify_like_cpp(UnitMoveTypeLikeCpp::Run, 2.0);
    assert_eq!(
        session.represented_pet_speed_propagations_like_cpp(),
        1,
        "C++ Pet::SetSpeedRate returns early when the rate is unchanged"
    );
}
#[test]
fn represented_player_speed_change_does_not_propagate_to_pet_in_combat_like_cpp() {
    let (mut session, _, _send_rx) = make_session();
    session.set_player_guid(Some(ObjectGuid::create_player(1, 42)));
    let pet_guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 7_001, 9_002);
    session.set_represented_pet_mode_state_like_cpp(
        Some(pet_guid),
        wow_packet::packets::pet::REACT_DEFENSIVE_LIKE_CPP,
        wow_packet::packets::pet::COMMAND_FOLLOW_LIKE_CPP,
    );
    session.in_combat = true;

    session.set_player_movement_speed_rate_and_notify_like_cpp(UnitMoveTypeLikeCpp::Run, 2.0);

    assert_eq!(
        session.represented_pet_movement_speed_rate_like_cpp(UnitMoveTypeLikeCpp::Run),
        1.0
    );
    assert_eq!(session.represented_pet_speed_propagations_like_cpp(), 0);
}
#[test]
fn represented_player_speed_change_does_not_send_pet_spline_when_pet_not_visible_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    session.set_player_guid(Some(ObjectGuid::create_player(1, 42)));
    let pet_guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 7_001, 9_003);
    session.set_represented_pet_mode_state_like_cpp(
        Some(pet_guid),
        wow_packet::packets::pet::REACT_DEFENSIVE_LIKE_CPP,
        wow_packet::packets::pet::COMMAND_FOLLOW_LIKE_CPP,
    );

    session.set_player_movement_speed_rate_and_notify_like_cpp(UnitMoveTypeLikeCpp::Run, 2.0);

    assert_eq!(
        session.represented_pet_movement_speed_rate_like_cpp(UnitMoveTypeLikeCpp::Run),
        2.0
    );
    let opcodes = drain_server_opcodes(&send_rx);
    assert!(
        !opcodes.contains(&ServerOpcodes::MoveSplineSetRunSpeed),
        "C++ SendMessageToSet only reaches sessions that have the pet at client"
    );
    assert!(
        opcodes.contains(&ServerOpcodes::MoveSetRunSpeed),
        "the player's own speed update is independent from pet visibility"
    );
}
#[test]
fn represented_mount_speed_uses_cpp_not_stack_bonus_order_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    session.set_player_guid(Some(ObjectGuid::create_player(1, 42)));
    session.set_mount_capability_store(Arc::new(wow_data::MountCapabilityStore::from_entries([
        wow_data::MountCapabilityEntry {
            id: 77,
            flags: wow_data::MOUNT_CAPABILITY_FLAG_GROUND,
            req_riding_skill: 0,
            req_area_id: 0,
            req_spell_aura_id: 0,
            req_spell_known_id: 0,
            mod_spell_aura_id: 12_347,
            req_map_id: 0,
        },
    ])));
    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        12_347,
        wow_data::SpellInfo {
            spell_id: 12_347,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: 0,
            effect_base_points: 0,
            effect_bonus_coefficient: 0.0,
            aura_type: None,
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: vec![
                wow_data::SpellEffectInfo {
                    effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                    effect_aura: wow_data::spell::aura_types::SPELL_AURA_MOD_INCREASE_MOUNTED_SPEED,
                    effect_base_points: 60,
                    effect_index: 0,
                    ..Default::default()
                },
                wow_data::SpellEffectInfo {
                    effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                    effect_aura:
                        wow_data::spell::aura_types::SPELL_AURA_MOD_MOUNTED_SPEED_NOT_STACK,
                    effect_base_points: 150,
                    effect_index: 1,
                    ..Default::default()
                },
            ],
        },
    );
    session.set_spell_store(Arc::new(spell_store));
    let effect = wow_data::SpellEffectInfo {
        effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
        effect_aura: wow_data::spell::aura_types::SPELL_AURA_MOUNTED,
        effect_base_points: 77,
        effect_misc_value_1: 0,
        ..Default::default()
    };

    session
        .apply_represented_mounted_aura_like_cpp(100, ObjectGuid::EMPTY, &effect)
        .unwrap();

    assert!(
        (session.player_movement_speed_like_cpp(UnitMoveTypeLikeCpp::Run) - 28.0).abs() < 0.0001,
        "C++ Unit::UpdateSpeed uses max(stack_bonus, non_stack_bonus) before AddPct(main_speed_mod)"
    );
    assert!(
        drain_server_opcodes(&send_rx).contains(&ServerOpcodes::MoveSetRunSpeed),
        "not-stack mounted speed changes are client-visible through Unit::SetSpeedRate"
    );
}
#[test]
fn represented_non_mounted_flight_speed_sums_cpp_flight_modifiers_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    session.set_player_guid(Some(ObjectGuid::create_player(1, 42)));
    let caster = ObjectGuid::create_player(1, 42);
    let flight = wow_data::SpellEffectInfo {
        effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
        effect_aura: wow_data::spell::aura_types::SPELL_AURA_MOD_INCREASE_FLIGHT_SPEED,
        effect_base_points: 20,
        effect_index: 0,
        ..Default::default()
    };
    let vehicle_flight = wow_data::SpellEffectInfo {
        effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
        effect_aura: wow_data::spell::aura_types::SPELL_AURA_MOD_INCREASE_VEHICLE_FLIGHT_SPEED,
        effect_base_points: 30,
        effect_index: 1,
        ..Default::default()
    };
    let not_stack = wow_data::SpellEffectInfo {
        effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
        effect_aura: wow_data::spell::aura_types::SPELL_AURA_MOD_FLIGHT_SPEED_NOT_STACK,
        effect_base_points: 100,
        effect_index: 2,
        ..Default::default()
    };

    session
        .apply_represented_aura_modifier_like_cpp(
            20_024,
            caster,
            &flight,
            RepresentedAuraEffectLikeCpp::FlightSpeed,
            30_000,
        )
        .unwrap();
    session
        .apply_represented_aura_modifier_like_cpp(
            20_025,
            caster,
            &vehicle_flight,
            RepresentedAuraEffectLikeCpp::VehicleFlightSpeed,
            30_000,
        )
        .unwrap();
    session
        .apply_represented_aura_modifier_like_cpp(
            20_026,
            caster,
            &not_stack,
            RepresentedAuraEffectLikeCpp::FlightSpeedNotStack,
            30_000,
        )
        .unwrap();
    session.recompute_represented_flight_speed_rate_like_cpp();

    assert!(
        (session.player_movement_speed_like_cpp(UnitMoveTypeLikeCpp::Flight) - 21.0).abs() < 0.0001,
        "C++ MOVE_FLIGHT non-mounted branch sums flight and vehicle-flight mods after max(not-stack, stack)"
    );
    assert!(
        drain_server_opcodes(&send_rx).contains(&ServerOpcodes::MoveSetFlightSpeed),
        "non-mounted flight speed changes are observable through Unit::SetSpeedRate"
    );
}
#[test]
fn represented_non_mounted_flight_speed_removal_recomputes_like_cpp() {
    let (mut session, _, _send_rx) = make_session();
    session.set_player_guid(Some(ObjectGuid::create_player(1, 42)));
    let caster = ObjectGuid::create_player(1, 42);
    let flight = wow_data::SpellEffectInfo {
        effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
        effect_aura: wow_data::spell::aura_types::SPELL_AURA_MOD_INCREASE_FLIGHT_SPEED,
        effect_base_points: 20,
        effect_index: 0,
        ..Default::default()
    };
    let vehicle_flight = wow_data::SpellEffectInfo {
        effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
        effect_aura: wow_data::spell::aura_types::SPELL_AURA_MOD_INCREASE_VEHICLE_FLIGHT_SPEED,
        effect_base_points: 30,
        effect_index: 1,
        ..Default::default()
    };

    session
        .apply_represented_aura_modifier_like_cpp(
            20_027,
            caster,
            &flight,
            RepresentedAuraEffectLikeCpp::FlightSpeed,
            30_000,
        )
        .unwrap();
    session
        .apply_represented_aura_modifier_like_cpp(
            20_028,
            caster,
            &vehicle_flight,
            RepresentedAuraEffectLikeCpp::VehicleFlightSpeed,
            30_000,
        )
        .unwrap();
    session.recompute_represented_flight_speed_rate_like_cpp();
    assert!(
        (session.player_movement_speed_like_cpp(UnitMoveTypeLikeCpp::Flight) - 10.5).abs() < 0.0001
    );
    let vehicle_flight_slot = session
        .visible_auras
        .iter()
        .find_map(|(&slot, aura)| {
            (aura.represented_effect == Some(RepresentedAuraEffectLikeCpp::VehicleFlightSpeed))
                .then_some(slot)
        })
        .unwrap();

    session.remove_aura(vehicle_flight_slot).unwrap();

    assert!(
        (session.player_movement_speed_like_cpp(UnitMoveTypeLikeCpp::Flight) - 8.4).abs() < 0.0001,
        "C++ aura removal recomputes MOVE_FLIGHT and drops the removed vehicle-flight modifier"
    );
}
