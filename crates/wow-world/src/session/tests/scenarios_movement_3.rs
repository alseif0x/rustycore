//! Session scenarios exercising the represented movement responsibility.
//!
//! Split out of session_tests.rs under #626; assertions and registrations
//! are unchanged and the shared fixtures stay in the parent module.

use super::*;

#[test]
fn represented_run_speed_minimum_speed_floor_applies_after_slow_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    session.set_player_guid(Some(ObjectGuid::create_player(1, 42)));
    let caster = ObjectGuid::create_player(1, 42);
    let speed = wow_data::SpellEffectInfo {
        effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
        effect_aura: wow_data::spell::aura_types::SPELL_AURA_MOD_INCREASE_SPEED,
        effect_base_points: 100,
        effect_index: 0,
        ..Default::default()
    };
    let slow = wow_data::SpellEffectInfo {
        effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
        effect_aura: wow_data::spell::aura_types::SPELL_AURA_MOD_DECREASE_SPEED,
        effect_base_points: -80,
        effect_index: 1,
        ..Default::default()
    };
    let minimum = wow_data::SpellEffectInfo {
        effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
        effect_aura: wow_data::spell::aura_types::SPELL_AURA_MOD_MINIMUM_SPEED,
        effect_base_points: 75,
        effect_index: 2,
        ..Default::default()
    };

    session
        .apply_represented_aura_modifier_like_cpp(
            20_010,
            caster,
            &speed,
            RepresentedAuraEffectLikeCpp::Speed,
            30_000,
        )
        .unwrap();
    session
        .apply_represented_aura_modifier_like_cpp(
            20_011,
            caster,
            &slow,
            RepresentedAuraEffectLikeCpp::DecreaseSpeed,
            30_000,
        )
        .unwrap();
    session
        .apply_represented_aura_modifier_like_cpp(
            20_012,
            caster,
            &minimum,
            RepresentedAuraEffectLikeCpp::MinimumSpeed,
            30_000,
        )
        .unwrap();
    session.recompute_represented_run_speed_rate_like_cpp();

    assert!(
        (session.player_movement_speed_like_cpp(UnitMoveTypeLikeCpp::Run) - 5.25).abs() < 0.0001,
        "C++ SPELL_AURA_MOD_MINIMUM_SPEED floors the final slowed MOVE_RUN rate"
    );
    assert!(
        drain_server_opcodes(&send_rx).contains(&ServerOpcodes::MoveSetRunSpeed),
        "minimum-speed floor changes are observable through Unit::SetSpeedRate"
    );
}
#[test]
fn represented_run_speed_minimum_speed_removal_recomputes_like_cpp() {
    let (mut session, _, _send_rx) = make_session();
    session.set_player_guid(Some(ObjectGuid::create_player(1, 42)));
    let caster = ObjectGuid::create_player(1, 42);
    let speed = wow_data::SpellEffectInfo {
        effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
        effect_aura: wow_data::spell::aura_types::SPELL_AURA_MOD_INCREASE_SPEED,
        effect_base_points: 100,
        effect_index: 0,
        ..Default::default()
    };
    let slow = wow_data::SpellEffectInfo {
        effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
        effect_aura: wow_data::spell::aura_types::SPELL_AURA_MOD_DECREASE_SPEED,
        effect_base_points: -80,
        effect_index: 1,
        ..Default::default()
    };
    let minimum = wow_data::SpellEffectInfo {
        effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
        effect_aura: wow_data::spell::aura_types::SPELL_AURA_MOD_MINIMUM_SPEED,
        effect_base_points: 75,
        effect_index: 2,
        ..Default::default()
    };

    session
        .apply_represented_aura_modifier_like_cpp(
            20_013,
            caster,
            &speed,
            RepresentedAuraEffectLikeCpp::Speed,
            30_000,
        )
        .unwrap();
    session
        .apply_represented_aura_modifier_like_cpp(
            20_014,
            caster,
            &slow,
            RepresentedAuraEffectLikeCpp::DecreaseSpeed,
            30_000,
        )
        .unwrap();
    session
        .apply_represented_aura_modifier_like_cpp(
            20_015,
            caster,
            &minimum,
            RepresentedAuraEffectLikeCpp::MinimumSpeed,
            30_000,
        )
        .unwrap();
    session.recompute_represented_run_speed_rate_like_cpp();
    assert!(
        (session.player_movement_speed_like_cpp(UnitMoveTypeLikeCpp::Run) - 5.25).abs() < 0.0001
    );
    let minimum_slot = session
        .visible_auras
        .iter()
        .find_map(|(&slot, aura)| {
            (aura.represented_effect == Some(RepresentedAuraEffectLikeCpp::MinimumSpeed))
                .then_some(slot)
        })
        .unwrap();

    session.remove_aura(minimum_slot).unwrap();

    assert!(
        (session.player_movement_speed_like_cpp(UnitMoveTypeLikeCpp::Run) - 2.8).abs() < 0.0001,
        "C++ aura removal recomputes MOVE_RUN and drops the removed minimum-speed floor"
    );
}
#[test]
fn represented_run_speed_minimum_speed_rate_floor_applies_before_slow_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    session.set_player_guid(Some(ObjectGuid::create_player(1, 42)));
    let caster = ObjectGuid::create_player(1, 42);
    let slow = wow_data::SpellEffectInfo {
        effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
        effect_aura: wow_data::spell::aura_types::SPELL_AURA_MOD_DECREASE_SPEED,
        effect_base_points: -50,
        effect_index: 0,
        ..Default::default()
    };
    let minimum_rate = wow_data::SpellEffectInfo {
        effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
        effect_aura: wow_data::spell::aura_types::SPELL_AURA_MOD_MINIMUM_SPEED_RATE,
        effect_base_points: 10,
        effect_index: 1,
        ..Default::default()
    };

    session
        .apply_represented_aura_modifier_like_cpp(
            20_016,
            caster,
            &slow,
            RepresentedAuraEffectLikeCpp::DecreaseSpeed,
            30_000,
        )
        .unwrap();
    session
        .apply_represented_aura_modifier_like_cpp(
            20_017,
            caster,
            &minimum_rate,
            RepresentedAuraEffectLikeCpp::MinimumSpeedRate,
            30_000,
        )
        .unwrap();
    session.recompute_represented_run_speed_rate_like_cpp();

    assert!(
        (session.player_movement_speed_like_cpp(UnitMoveTypeLikeCpp::Run) - 5.0).abs() < 0.0001,
        "C++ SPELL_AURA_MOD_MINIMUM_SPEED_RATE floors MOVE_RUN before the strongest slow"
    );
    assert!(
        drain_server_opcodes(&send_rx).contains(&ServerOpcodes::MoveSetRunSpeed),
        "minimum-speed-rate changes are observable through Unit::SetSpeedRate"
    );
}
#[test]
fn represented_run_speed_minimum_speed_rate_removal_recomputes_like_cpp() {
    let (mut session, _, _send_rx) = make_session();
    session.set_player_guid(Some(ObjectGuid::create_player(1, 42)));
    let caster = ObjectGuid::create_player(1, 42);
    let slow = wow_data::SpellEffectInfo {
        effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
        effect_aura: wow_data::spell::aura_types::SPELL_AURA_MOD_DECREASE_SPEED,
        effect_base_points: -50,
        effect_index: 0,
        ..Default::default()
    };
    let minimum_rate = wow_data::SpellEffectInfo {
        effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
        effect_aura: wow_data::spell::aura_types::SPELL_AURA_MOD_MINIMUM_SPEED_RATE,
        effect_base_points: 10,
        effect_index: 1,
        ..Default::default()
    };

    session
        .apply_represented_aura_modifier_like_cpp(
            20_018,
            caster,
            &slow,
            RepresentedAuraEffectLikeCpp::DecreaseSpeed,
            30_000,
        )
        .unwrap();
    session
        .apply_represented_aura_modifier_like_cpp(
            20_019,
            caster,
            &minimum_rate,
            RepresentedAuraEffectLikeCpp::MinimumSpeedRate,
            30_000,
        )
        .unwrap();
    session.recompute_represented_run_speed_rate_like_cpp();
    assert!(
        (session.player_movement_speed_like_cpp(UnitMoveTypeLikeCpp::Run) - 5.0).abs() < 0.0001
    );
    let minimum_rate_slot = session
        .visible_auras
        .iter()
        .find_map(|(&slot, aura)| {
            (aura.represented_effect == Some(RepresentedAuraEffectLikeCpp::MinimumSpeedRate))
                .then_some(slot)
        })
        .unwrap();

    session.remove_aura(minimum_rate_slot).unwrap();

    assert!(
        (session.player_movement_speed_like_cpp(UnitMoveTypeLikeCpp::Run) - 3.5).abs() < 0.0001,
        "C++ aura removal recomputes MOVE_RUN and drops the pre-slow minimum-speed-rate floor"
    );
}
#[test]
fn represented_mount_capability_uses_login_zone_area_fallback_like_cpp() {
    let (mut session, _, _) = make_session();
    session.current_map_id = 571;
    install_canonical_player_owner_for_test(&mut session, 571, 0);
    session.set_player_skill_values_like_cpp(HashMap::from([(SKILL_RIDING_LIKE_CPP, 75)]));
    session.set_area_table_store(Arc::new(wow_data::AreaTableStore::from_entries([
        wow_data::AreaTableEntry {
            id: 1519,
            continent_id: 0,
            parent_area_id: 0,
            area_bit: -1,
            exploration_level: 0,
            mount_flags: i32::from(wow_data::AREA_MOUNT_FLAG_ALLOW_GROUND_MOUNTS),
            flags: 0,
        },
    ])));
    session.set_mount_capability_store(Arc::new(wow_data::MountCapabilityStore::from_entries([
        wow_data::MountCapabilityEntry {
            id: 11,
            flags: wow_data::MOUNT_CAPABILITY_FLAG_GROUND,
            req_riding_skill: 75,
            req_area_id: 0,
            req_spell_aura_id: 0,
            req_spell_known_id: 0,
            mod_spell_aura_id: 1001,
            req_map_id: -1,
        },
    ])));
    session.set_mount_type_x_capability_store(Arc::new(
        wow_data::MountTypeXCapabilityStore::from_entries([wow_data::MountTypeXCapabilityEntry {
            id: 2,
            mount_type_id: 7,
            mount_capability_id: 11,
            order_index: 1,
        }]),
    ));

    assert_eq!(session.player_zone_area_like_cpp(), Some((0, 0)));
    assert_eq!(
        session
            .represented_mount_capability_for_type_from_session_like_cpp(7, None)
            .map(|capability| capability.id),
        Some(11),
        "Rust uses a ground-mount fallback for represented area 0 until TerrainMgr can resolve C++ area ids"
    );

    // C++ resolves the exact area from terrain after adding the player to
    // the map. Rust currently seeds the represented runtime from the DB
    // zone until TerrainMgr parity exists; using the zone as area is still
    // closer than evaluating mount restrictions against area 0.
    session.set_player_zone_area_like_cpp(1519, 1519);
    assert_eq!(
        session
            .represented_mount_capability_for_type_from_session_like_cpp(7, None)
            .map(|capability| capability.id),
        Some(11)
    );
}
#[test]
fn adjust_client_movement_time_uses_clock_delta_or_cpp_fallback() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    session.time_sync_clock_delta = 250;
    assert_eq!(session.adjust_client_movement_time_like_cpp(1_000), 1_250);

    session.time_sync_clock_delta = 0;
    let adjusted = session.adjust_client_movement_time_like_cpp(1_000);
    assert_ne!(adjusted, 1_000);
}
#[test]
fn canonical_visibility_uses_player_instance_cross_map_blocks_instance_zero_fallback() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 49_620);
    let session_position = Position::new(100.0, 200.0, 30.0, 0.0);
    let canonical_player_position = Position::new(10.0, 20.0, 30.0, 0.0);
    let default_instance_guid = test_gameobject_guid(49_620, 49_620);

    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "CrossMapOwner".to_string(),
        session_position,
        571,
        1,
        1,
        80,
        0,
    ));
    add_canonical_test_player_on_map(&canonical, player_guid, canonical_player_position, 1, 7);
    add_canonical_test_gameobject_on_map(
        &canonical,
        default_instance_guid,
        49_620,
        session_position,
        571,
        0,
    );
    session.represented_gameobject_use_states.insert(
        default_instance_guid,
        RepresentedGameObjectUseState {
            display_id: Some(7_620),
            go_type: Some(3),
            map_id: Some(571),
            position: Some(session_position),
            ..Default::default()
        },
    );

    let visible =
        session.visible_gameobjects_from_canonical_map_like_cpp(571, &session_position, 100.0);

    assert!(
        visible.is_none(),
        "typed canonical player on another map must block legacy instance-0 fallback"
    );
}
#[tokio::test]
async fn far_sight_update_visibility_falls_back_for_unsupported_seer_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 493);
    let gameobject_seer_guid = test_gameobject_guid(4930, 4930);
    let far_creature_guid = test_creature_guid(4931);
    let player_position = Position::new(0.0, 0.0, 0.0, 0.0);
    let unsupported_seer_position = Position::new(3000.0, 3000.0, 0.0, 0.0);
    let far_creature_position = Position::new(3010.0, 3010.0, 0.0, 0.0);

    session.set_map_manager(Arc::clone(&manager));
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
        "UnsupportedFarsightVisibility".to_string(),
        player_position,
        571,
        1,
        1,
        80,
        0,
    ));
    session
        .ensure_canonical_world_map_for_current_player_like_cpp()
        .expect("canonical player map");
    session.record_represented_gameobject_runtime_state_like_cpp(
        571,
        gameobject_seer_guid,
        4930,
        unsupported_seer_position,
        3,
    );

    let (grid_x, grid_y) =
        crate::map_manager::world_to_grid_coords(far_creature_position.x, far_creature_position.y);
    manager.write().unwrap().add_creature(
        571,
        0,
        grid_x,
        grid_y,
        crate::map_manager::WorldCreature::new(
            far_creature_guid,
            4931,
            far_creature_position,
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

    set_canonical_player_farsight_object_like_cpp(&canonical, player_guid, gameobject_seer_guid);
    session.represented_seer_guid_like_cpp = Some(gameobject_seer_guid);

    assert_eq!(
        session.represented_visibility_source_position_like_cpp(),
        Some(player_position)
    );

    session.update_visibility().await;

    assert!(
        !session
            .client_visible_guids_like_cpp
            .contains(&far_creature_guid),
        "unsupported GameObject m_seer must not become the visibility source"
    );
    assert_eq!(session.last_visibility_pos, Some(player_position));
}
#[test]
fn npc_interaction_legacy_fallback_uses_player_instance_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 47);
    let questgiver_guid = test_creature_guid(18);
    let player_position = Position::new(10.0, 0.0, 0.0, 0.0);
    let npc_position = Position::new(14.0, 0.0, 0.0, 0.0);
    let canonical = shared_canonical_map_manager();
    let manager = shared_map_manager();

    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "Tester".to_string(),
        player_position,
        571,
        1,
        1,
        80,
        0,
    ));
    session.set_canonical_map_manager(Arc::clone(&canonical));
    add_canonical_test_player_on_map(&canonical, player_guid, player_position, 571, 7);
    manager.write().unwrap().add_creature(
        571,
        0,
        0,
        0,
        crate::map_manager::WorldCreature::new(
            questgiver_guid,
            999,
            npc_position,
            100,
            80,
            1,
            2,
            0.0,
            1,
            35,
            wow_constants::unit::NPCFlags1::QUEST_GIVER.bits(),
            0,
        ),
    );
    manager.write().unwrap().add_creature(
        571,
        7,
        0,
        0,
        crate::map_manager::WorldCreature::new(
            questgiver_guid,
            508,
            npc_position,
            100,
            80,
            1,
            2,
            0.0,
            1,
            35,
            wow_constants::unit::NPCFlags1::QUEST_GIVER.bits(),
            0,
        ),
    );
    session.set_map_manager(manager);

    assert_eq!(
        session.represented_npc_can_interact_with_like_cpp(
            questgiver_guid,
            wow_constants::unit::NPCFlags1::QUEST_GIVER.bits(),
            0,
        ),
        Some(RepresentedCreatureAccessLikeCpp {
            entry: 508,
            position: npc_position,
            npc_flags: wow_constants::unit::NPCFlags1::QUEST_GIVER.bits(),
            npc_flags2: 0,
            faction_template_id: 35,
            trainer_class: 0,
        }),
        "C++ ObjectAccessor resolves relative to the player's current instance, not a hard-coded instance 0"
    );
}
#[test]
fn npc_interaction_rejects_legacy_fallback_when_canonical_player_is_out_of_world_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 46);
    let questgiver_guid = test_creature_guid(17);
    let position = Position::new(10.0, 0.0, 0.0, 0.0);
    let canonical = shared_canonical_map_manager();
    let manager = shared_map_manager();

    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "Tester".to_string(),
        position,
        571,
        1,
        1,
        80,
        0,
    ));
    session.set_canonical_map_manager(Arc::clone(&canonical));
    add_canonical_test_player_on_map(&canonical, player_guid, position, 571, 0);
    session
        .mutate_canonical_player_like_cpp(|player| {
            player
                .unit_mut()
                .world_mut()
                .object_mut()
                .remove_from_world();
        })
        .unwrap();
    manager.write().unwrap().add_creature(
        571,
        0,
        0,
        0,
        crate::map_manager::WorldCreature::new(
            questgiver_guid,
            507,
            Position::new(14.0, 0.0, 0.0, 0.0),
            100,
            80,
            1,
            2,
            0.0,
            1,
            35,
            wow_constants::unit::NPCFlags1::QUEST_GIVER.bits(),
            0,
        ),
    );
    session.set_map_manager(manager);

    assert_eq!(
        session.represented_npc_can_interact_with_like_cpp(
            questgiver_guid,
            wow_constants::unit::NPCFlags1::QUEST_GIVER.bits(),
            0,
        ),
        None,
        "C++ GetNPCIfCanInteractWith requires the canonical player to be in world; stale legacy state must not authorize interaction"
    );
}
#[test]
fn canonical_player_map_transfer_sync_removes_stale_old_map_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let (mut other_session, _, _) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 51);
    let other_player_guid = ObjectGuid::create_player(1, 52);
    let creature_guid = test_creature_guid(19_051);
    let target_position = Position::new(-8815.0, 635.0, 94.0, 1.0);

    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_map_store(canonical_player_transfer_test_map_store_like_cpp());
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "TransferSelf".to_string(),
        Position::new(3700.0, 1500.0, 120.0, 0.0),
        571,
        1,
        1,
        80,
        0,
    ));
    insert_session_player_into_canonical_map_like_cpp(&session, &canonical, 571, 0);

    other_session.set_player_guid(Some(other_player_guid));
    other_session.player_name = Some("TransferOther".into());
    other_session.player_position = Some(Position::new(3710.0, 1510.0, 120.0, 0.0));
    other_session.current_map_id = 571;
    insert_session_player_into_canonical_map_like_cpp(&other_session, &canonical, 571, 0);
    add_canonical_test_creature(
        &canonical,
        creature_guid,
        123,
        Position::new(3720.0, 1520.0, 120.0, 0.0),
        0,
    );

    session.set_player_map_position_like_cpp(0, target_position);
    session
        .ensure_canonical_world_map_for_current_player_like_cpp()
        .expect("target world map decision");

    let manager = canonical.lock().unwrap();
    let old_map = manager.find_map(571, 0).unwrap().map();
    assert!(old_map.get_typed_player(player_guid).is_none());
    assert!(old_map.get_typed_player(other_player_guid).is_some());
    assert!(
        old_map
            .with_creature_like_cpp(creature_guid, Clone::clone)
            .is_some()
    );

    let target_player = manager
        .find_map(0, 0)
        .unwrap()
        .map()
        .get_typed_player(player_guid)
        .unwrap();
    assert_eq!(target_player.unit().world().map_id(), 0);
    assert_eq!(target_player.unit().world().position(), target_position);
}
#[test]
fn canonical_player_far_teleport_keeps_one_detached_identity_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 55);
    let target_guid = test_creature_guid(19_055);
    let target_position = Position::new(-8815.0, 635.0, 94.0, 1.0);

    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_map_store(canonical_player_transfer_test_map_store_like_cpp());
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "FarTeleport".to_string(),
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
    let handle = session.player_handle_like_cpp.expect("canonical handle");
    session
        .mutate_canonical_player_like_cpp(|player| {
            player.unit_mut().set_attacking(Some(target_guid));
            player.set_player_flag(PLAYER_FLAGS_UBER_LIKE_CPP);
        })
        .unwrap();

    assert!(session.remove_current_player_from_canonical_current_map_like_cpp());
    assert_eq!(
        canonical.lock().unwrap().player_residence_like_cpp(handle),
        Some(wow_map::PlayerResidenceLikeCpp::Detached)
    );
    assert_eq!(
        session.canonical_player_snapshot_like_cpp(|player| (
            player.unit().attacking(),
            player.has_player_flag(PLAYER_FLAGS_UBER_LIKE_CPP),
        )),
        Some((Some(target_guid), true))
    );

    session.set_player_map_position_like_cpp(0, target_position);
    session
        .ensure_canonical_world_map_for_current_player_like_cpp()
        .expect("target world map");

    assert_eq!(session.player_handle_like_cpp, Some(handle));
    let manager = canonical.lock().unwrap();
    assert_eq!(
        manager.player_residence_like_cpp(handle),
        Some(wow_map::PlayerResidenceLikeCpp::Active(
            wow_map::MapKey::new(0, 0)
        ))
    );
    assert_eq!(
        manager.with_player_like_cpp(handle, |player| (
            player.unit().world().position(),
            player.unit().attacking(),
            player.has_player_flag(PLAYER_FLAGS_UBER_LIKE_CPP),
        )),
        Some((target_position, Some(target_guid), true))
    );
}
