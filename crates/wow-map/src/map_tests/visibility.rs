//! Visibility scenarios for [`super`].
//!
//! Split out of map_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

#[test]
fn world_object_visibility_range_reads_map_visible_distance_like_cpp() {
    let map = world_object_environment_test_map(
        RecordingWorldObjectTerrain::new(true, INVALID_HEIGHT, INVALID_HEIGHT),
        123.5,
    );
    let object = world_object(HighGuid::DynamicObject, 571, 7, true);

    assert_eq!(object.get_visibility_range(&map), 123.5);
}

#[test]
fn add_to_map_marks_nearby_players_for_deferred_visibility_like_cpp() {
    let mut map = test_map();

    let mut player = test_player_for_viewpoint(4510101);
    player
        .unit_mut()
        .world_mut()
        .object_mut()
        .remove_from_world();
    let player_guid = player.guid();
    map.add_map_object_record_to_map_like_cpp(MapObjectRecord::new_player(player).unwrap())
        .unwrap();

    let mut creature = test_creature_for_spawn(45101, 4510102, true);
    creature
        .unit_mut()
        .world_mut()
        .relocate(Position::xyz(10.5, 20.5, 30.0));
    creature
        .unit_mut()
        .world_mut()
        .object_mut()
        .remove_from_world();
    let creature_guid = creature.guid();
    map.add_map_object_record_to_map_like_cpp(MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();

    assert!(
        map.map_object(player_guid)
            .unwrap()
            .object()
            .is_need_notify(ObjectNotifyFlags::VISIBILITY_CHANGED),
        "a nearby Player must be selected for the deferred visibility rail"
    );
    assert!(map.map_object(creature_guid).is_some());
}

#[test]
fn remove_from_map_marks_nearby_players_for_deferred_visibility_like_cpp() {
    let mut map = test_map();

    let mut player = test_player_for_viewpoint(4510201);
    player
        .unit_mut()
        .world_mut()
        .object_mut()
        .remove_from_world();
    let player_guid = player.guid();
    map.add_map_object_record_to_map_like_cpp(MapObjectRecord::new_player(player).unwrap())
        .unwrap();

    let mut creature = test_creature_for_spawn(45102, 4510202, true);
    creature
        .unit_mut()
        .world_mut()
        .relocate(Position::xyz(10.5, 20.5, 30.0));
    creature
        .unit_mut()
        .world_mut()
        .object_mut()
        .remove_from_world();
    let creature_guid = creature.guid();
    map.add_map_object_record_to_map_like_cpp(MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();
    map.get_typed_player_mut(player_guid)
        .unwrap()
        .unit_mut()
        .world_mut()
        .object_mut()
        .reset_all_notifies();

    map.remove_from_map_like_cpp(creature_guid, true).unwrap();

    assert!(
        map.map_object(player_guid)
            .unwrap()
            .object()
            .is_need_notify(ObjectNotifyFlags::VISIBILITY_CHANGED),
        "a nearby Player must be selected before the source is erased"
    );
    assert!(map.map_object(creature_guid).is_none());
}

#[test]
fn grid_activation_range_uses_creature_sight_for_inactive_sources_like_cpp() {
    let mut map = test_map();
    let creature = test_creature_for_spawn(1234, 1234001, true);
    let guid = creature.guid();
    map.add_map_object_record_to_map_like_cpp(MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();

    assert_eq!(
        map.grid_activation_range_for_guid_like_cpp(guid),
        wow_entities::DEFAULT_MONSTER_SIGHT_DISTANCE
    );

    map.with_creature_mut_like_cpp(guid, |creature| {
        creature.unit_mut().world_mut().set_active(true);
    });
    assert_eq!(map.grid_activation_range_for_guid_like_cpp(guid), 100.0);
}

#[test]
fn grid_activation_range_uses_instance_distance_for_active_player_cinematic_like_cpp() {
    let mut map = test_map();
    let mut player = test_player_for_viewpoint(1235);
    let guid = player.guid();

    player
        .gameplay_state_mut()
        .cinematic
        .begin_cinematic_like_cpp(444, [11, 0, 0, 0, 0, 0, 0, 0]);
    map.add_map_object_record_to_map_like_cpp(MapObjectRecord::new_player(player).unwrap())
        .unwrap();

    // Beginning a sequence is not enough: C++ `IsOnCinematic()` observes the
    // camera pointer, which is set only after the first camera is selected.
    assert_eq!(
        map.grid_activation_range_for_guid_like_cpp(guid),
        map.visibility_range()
    );

    map.get_typed_player_mut(guid)
        .unwrap()
        .gameplay_state_mut()
        .cinematic
        .next_cinematic_camera_like_cpp();
    assert_eq!(
        map.grid_activation_range_for_guid_like_cpp(guid),
        wow_entities::DEFAULT_VISIBILITY_INSTANCE
    );

    map.get_typed_player_mut(guid)
        .unwrap()
        .gameplay_state_mut()
        .cinematic
        .end_cinematic_like_cpp();
    assert_eq!(
        map.grid_activation_range_for_guid_like_cpp(guid),
        map.visibility_range()
    );

    // The override is a floor, so a map already wider than the instance
    // distance keeps its configured visibility range.
    let mut wide_map = Map::with_hooks(
        571,
        7,
        1,
        1000,
        true,
        220.0,
        RecordingWorldObjectTerrain::new(true, INVALID_HEIGHT, INVALID_HEIGHT),
        RecordingLifecycle::default(),
    );
    let mut wide_player = test_player_for_viewpoint(1236);
    let wide_guid = wide_player.guid();
    wide_player
        .gameplay_state_mut()
        .cinematic
        .begin_cinematic_like_cpp(444, [11, 0, 0, 0, 0, 0, 0, 0]);
    wide_player
        .gameplay_state_mut()
        .cinematic
        .next_cinematic_camera_like_cpp();
    wide_map
        .add_map_object_record_to_map_like_cpp(MapObjectRecord::new_player(wide_player).unwrap())
        .unwrap();
    assert_eq!(
        wide_map.grid_activation_range_for_guid_like_cpp(wide_guid),
        220.0
    );
}

#[test]
fn personal_phase_tracker_update_enqueues_expired_canonical_object_like_cpp() {
    let mut map = test_map();
    let owner = ObjectGuid::create_player(1, 44001);
    let phase_id = 44;
    let creature = test_creature_for_spawn(44001, 4400101, true);
    let guid = creature.guid();
    map.add_map_object_record_to_map_like_cpp(MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();
    map.register_personal_phase_object_for_test(phase_id, owner, guid);
    map.mark_personal_phases_for_deletion_for_test(owner);

    let early = map.update_personal_phase_tracker_like_cpp(59_999);

    assert_eq!(early, PersonalPhaseTrackerUpdateSummaryLikeCpp::default());
    assert_eq!(map.objects_to_remove_count_like_cpp(), 0);
    assert!(map.map_object_record(guid).is_some());

    let expired = map.update_personal_phase_tracker_like_cpp(1);

    assert_eq!(
        expired,
        PersonalPhaseTrackerUpdateSummaryLikeCpp {
            expired_objects: 1,
            remove_queued: 1,
            missing_or_stale: 0,
            unsupported_kinds: 0,
            duplicate_queued: 0,
        }
    );
    assert_eq!(map.objects_to_remove_count_like_cpp(), 1);
    assert!(map.map_object_record(guid).is_some());
    let creature = map.get_typed_creature(guid).unwrap();
    assert!(creature.unit().world().object().is_destroyed_object());
    assert_eq!(creature.cleanup_before_delete_count(), 1);
}
#[test]
fn personal_phase_tracker_update_counts_missing_expired_guid_like_cpp() {
    let mut map = test_map();
    let owner = ObjectGuid::create_player(1, 44002);
    let missing_guid = guid(HighGuid::Creature, 4400201);
    map.register_personal_phase_object_for_test(44, owner, missing_guid);
    map.mark_personal_phases_for_deletion_for_test(owner);

    let summary = map.update_personal_phase_tracker_like_cpp(60_000);

    assert_eq!(
        summary,
        PersonalPhaseTrackerUpdateSummaryLikeCpp {
            expired_objects: 1,
            remove_queued: 0,
            missing_or_stale: 1,
            unsupported_kinds: 0,
            duplicate_queued: 0,
        }
    );
    assert_eq!(map.objects_to_remove_count_like_cpp(), 0);
    assert_eq!(map.map_object_count(), 0);
}
#[test]
fn farsight_dynamic_object_create_inserts_focus_and_sets_viewpoint_like_cpp() {
    let mut map = test_map();
    let player = test_player_for_viewpoint(4280101);
    let player_guid = player.guid();
    map.insert_map_object_record(MapObjectRecord::new_player(player).unwrap())
        .unwrap();

    let outcome = create_farsight_focus_for_tests(&mut map, player_guid);

    assert_eq!(
        outcome.status,
        FarsightDynamicObjectCreateStatusLikeCpp::Created
    );
    assert_eq!(outcome.caster_player_guid, player_guid);
    assert_eq!(outcome.low_guid, Some(1));
    let dynamic_guid = outcome.dynamic_object_guid.unwrap();
    assert_eq!(dynamic_guid.high_type(), HighGuid::DynamicObject);
    assert_ne!(dynamic_guid.counter(), 12_345);
    assert_eq!(
        map.get_max_low_guid_like_cpp(HighGuid::DynamicObject)
            .unwrap(),
        2
    );
    let add_to_map = outcome.add_to_map.unwrap();
    assert!(add_to_map.inserted);
    assert!(add_to_map.inserted_into_cell);
    assert!(!add_to_map.already_in_world);

    let dynamic_object = map.get_typed_dynamic_object(dynamic_guid).unwrap();
    assert_eq!(dynamic_object.world().guid(), dynamic_guid);
    assert_eq!(dynamic_object.world().map_id(), 571);
    assert_eq!(dynamic_object.world().instance_id(), 7);
    assert_eq!(
        dynamic_object.world().position(),
        Position::new(100.0, 200.0, 30.0, 1.5)
    );
    assert!(dynamic_object.world().object().is_in_world());
    assert!(dynamic_object.world().is_active());
    assert_eq!(dynamic_object.world().object().entry(), 12_345);
    assert_eq!(dynamic_object.world().object().scale(), 1.0);
    assert_eq!(dynamic_object.caster_guid(), player_guid);
    assert_eq!(dynamic_object.bound_caster(), Some(player_guid));
    assert_eq!(
        dynamic_object.data().dynamic_object_type,
        DynamicObjectType::FarsightFocus as u8
    );
    assert_eq!(dynamic_object.data().spell_visual_id, 678);
    assert_eq!(dynamic_object.spell_id(), 12_345);
    assert_eq!(dynamic_object.radius(), 42.5);
    assert_eq!(dynamic_object.data().cast_time_ms, 987_654);
    assert_eq!(dynamic_object.duration_ms(), 30_000);
    assert!(dynamic_object.is_caster_viewpoint());
    assert_eq!(
        map.get_typed_player(player_guid)
            .unwrap()
            .active_data()
            .farsight_object,
        dynamic_guid
    );
    let viewpoint = outcome.caster_viewpoint.unwrap();
    assert_eq!(viewpoint.dynamic_object_guid, dynamic_guid);
    assert_eq!(
        viewpoint.status,
        DynamicObjectCasterViewpointStatusLikeCpp::CasterPlayerResolved
    );
    assert_eq!(
        viewpoint.player_set_viewpoint.status,
        PlayerSetViewpointStatusLikeCpp::Applied
    );
    assert!(viewpoint.player_set_viewpoint.update_visibility_requested);
    assert!(viewpoint.player_set_viewpoint.set_seer_requested);
}
#[test]
fn farsight_dynamic_object_create_invalid_destination_preserves_no_mutation_like_cpp() {
    let invalid_destinations = [
        Position::new(f32::NAN, 200.0, 30.0, 1.5),
        Position::new(100.0, 200.0, f32::NAN, 1.5),
        Position::new(100.0, 200.0, Position::MAP_HALFSIZE_LIKE_CPP, 1.5),
        Position::new(100.0, 200.0, 30.0, f32::NAN),
        Position::new(100.0, 200.0, 30.0, f32::INFINITY),
    ];

    for (index, dest) in invalid_destinations.into_iter().enumerate() {
        let mut map = test_map();
        let player = test_player_for_viewpoint(4280501 + index as i64);
        let player_guid = player.guid();
        map.insert_map_object_record(MapObjectRecord::new_player(player).unwrap())
            .unwrap();

        let outcome = map.create_farsight_dynamic_object_like_cpp(
            player_guid,
            12_345,
            678,
            dest,
            42.5,
            30_000,
            987_654,
            1,
            7,
        );

        assert_eq!(
            outcome.status,
            FarsightDynamicObjectCreateStatusLikeCpp::InvalidDestination
        );
        assert_eq!(map.entity_world.len(), 1);
        assert_eq!(
            map.get_max_low_guid_like_cpp(HighGuid::DynamicObject)
                .unwrap(),
            1
        );
        assert_eq!(
            map.get_typed_player(player_guid)
                .unwrap()
                .active_data()
                .farsight_object,
            ObjectGuid::EMPTY
        );
    }
}
#[test]
fn farsight_dynamic_object_create_reports_viewpoint_no_mutation_without_panicking_like_cpp() {
    let mut map = test_map();
    let mut player = test_player_for_viewpoint(4280601);
    let player_guid = player.guid();
    let existing_guid = guid(HighGuid::Creature, 4280609);
    player.set_farsight_object_like_cpp(existing_guid);
    map.insert_map_object_record(MapObjectRecord::new_player(player).unwrap())
        .unwrap();

    let outcome = create_farsight_focus_for_tests(&mut map, player_guid);

    assert_eq!(
        outcome.status,
        FarsightDynamicObjectCreateStatusLikeCpp::Created
    );
    let dynamic_guid = outcome.dynamic_object_guid.unwrap();
    let viewpoint = outcome.caster_viewpoint.unwrap();
    assert_eq!(
        viewpoint.player_set_viewpoint.status,
        PlayerSetViewpointStatusLikeCpp::AlreadyHasViewpoint
    );
    assert!(!viewpoint.player_set_viewpoint.update_visibility_requested);
    assert!(!viewpoint.player_set_viewpoint.set_seer_requested);
    assert!(viewpoint.dynamic_object_viewpoint_toggled);
    assert_eq!(
        map.get_typed_player(player_guid)
            .unwrap()
            .active_data()
            .farsight_object,
        existing_guid
    );
    assert!(
        map.get_typed_dynamic_object(dynamic_guid)
            .unwrap()
            .is_caster_viewpoint()
    );
}
#[test]
fn player_relocation_visibility_plan_matches_cpp_visible_and_out_of_range_shape() {
    let player = guid(HighGuid::Player, 1);
    let other_player = guid(HighGuid::Player, 2);
    let old_player = guid(HighGuid::Player, 3);
    let creature = guid(HighGuid::Creature, 4);
    let old_creature = guid(HighGuid::Creature, 5);
    let gameobject = guid(HighGuid::GameObject, 6);
    let mut nearby = NearbyCellGuids::default();
    nearby.world.players.insert(player);
    nearby.world.players.insert(other_player);
    nearby.grid.creatures.insert(creature);
    nearby.grid.gameobjects.insert(gameobject);

    let plan = PlayerRelocationVisibilityPlan::from_nearby_like_cpp(
        player,
        [other_player, old_player, old_creature],
        &nearby,
        true,
        [],
        [],
    );

    assert!(plan.visible_guids.contains(&player));
    assert!(plan.visible_guids.contains(&other_player));
    assert!(plan.visible_guids.contains(&creature));
    assert!(plan.visible_guids.contains(&gameobject));
    assert_eq!(
        plan.out_of_range_guids,
        HashSet::from([old_player, old_creature])
    );
    assert_eq!(
        plan.reciprocal_player_updates,
        HashSet::from([other_player, old_player])
    );
    assert_eq!(plan.ai_relocation_checks, vec![(creature, player)]);
}
#[test]
fn player_relocation_visibility_plan_skips_ai_when_not_relocated_for_ai() {
    let player = guid(HighGuid::Player, 1);
    let creature = guid(HighGuid::Creature, 2);
    let mut nearby = NearbyCellGuids::default();
    nearby.grid.creatures.insert(creature);

    let plan = PlayerRelocationVisibilityPlan::from_nearby_like_cpp(
        player,
        [creature],
        &nearby,
        false,
        [],
        [],
    );

    assert!(plan.out_of_range_guids.is_empty());
    assert!(plan.ai_relocation_checks.is_empty());
}
#[test]
fn player_relocation_visibility_plan_filters_targets_needing_cpp_notify() {
    let player = guid(HighGuid::Player, 4440201);
    let player_target_needs_notify = guid(HighGuid::Player, 4440202);
    let player_target_clear = guid(HighGuid::Player, 4440203);
    let old_player_needs_notify = guid(HighGuid::Player, 4440204);
    let old_player_clear = guid(HighGuid::Player, 4440205);
    let creature_needs_notify = guid(HighGuid::Creature, 4440206);
    let creature_clear = guid(HighGuid::Creature, 4440207);
    let mut nearby = NearbyCellGuids::default();
    nearby.world.players.insert(player);
    nearby.world.players.insert(player_target_needs_notify);
    nearby.world.players.insert(player_target_clear);
    nearby.grid.creatures.insert(creature_needs_notify);
    nearby.grid.creatures.insert(creature_clear);

    let plan = PlayerRelocationVisibilityPlan::from_nearby_like_cpp(
        player,
        [old_player_needs_notify, old_player_clear],
        &nearby,
        true,
        [player_target_needs_notify, old_player_needs_notify],
        [creature_needs_notify],
    );

    assert!(
        !plan
            .reciprocal_player_updates
            .contains(&player_target_needs_notify)
    );
    assert!(
        plan.reciprocal_player_updates
            .contains(&player_target_clear)
    );
    assert!(
        !plan
            .reciprocal_player_updates
            .contains(&old_player_needs_notify)
    );
    assert!(plan.reciprocal_player_updates.contains(&old_player_clear));
    assert_eq!(plan.ai_relocation_checks, vec![(creature_clear, player)]);
}
#[test]
fn delayed_unit_relocation_for_cells_uses_player_seer_notify_like_cpp() {
    let mut map = test_map();
    let mut player = test_player_for_viewpoint(4440101);
    let player_guid = player.guid();
    let viewpoint = world_object_with_counter(HighGuid::Creature, 4440102, 571, 7, false);
    let viewpoint_guid = viewpoint.guid();
    player.set_farsight_object_like_cpp(viewpoint_guid);

    let cell = map
        .add_to_map_like_cpp(
            AccessorObjectKind::Player,
            world_object_with_counter(HighGuid::Player, 4440101, 571, 7, false),
        )
        .unwrap()
        .cell;
    map.insert_map_object_record(MapObjectRecord::new_player(player).unwrap())
        .unwrap();
    map.add_to_map_like_cpp(AccessorObjectKind::Creature, viewpoint)
        .unwrap();
    map.entity_world
        .get_mut(&viewpoint_guid)
        .unwrap()
        .object_mut()
        .object_mut()
        .add_to_notify(ObjectNotifyFlags::VISIBILITY_CHANGED);

    let plan = map.delayed_unit_relocation_for_cells_like_cpp([cell], []);
    assert_eq!(plan.cell_plans.len(), 1);
    assert_eq!(
        plan.cell_plans[0].plan.player_relocations,
        vec![player_guid]
    );
    assert!(
        plan.cell_plans[0]
            .plan
            .skipped_invalid_viewpoints
            .is_empty()
    );

    let visibility_plans = map.delayed_unit_relocation_visibility_plans_like_cpp(
        &plan,
        map.delayed_player_relocation_contexts_from_plan_like_cpp(&plan),
        [DelayedCreatureRelocationContext {
            creature_guid: viewpoint_guid,
            source_creature_alive: true,
        }],
    );
    assert_eq!(visibility_plans.player_plans.len(), 1);
    assert_eq!(visibility_plans.player_plans[0].player_guid, player_guid);
    assert_eq!(
        visibility_plans.player_plans[0].viewpoint_guid,
        viewpoint_guid
    );
    assert!(
        visibility_plans.player_plans[0]
            .visibility_plan
            .ai_relocation_checks
            .is_empty()
    );
    let creature_plan = visibility_plans
        .creature_plans
        .iter()
        .find(|plan| plan.creature_guid == viewpoint_guid)
        .unwrap();
    assert!(
        !creature_plan
            .visibility_plan
            .player_visibility_updates
            .contains(&player_guid),
        "CreatureRelocationNotifier must test player->m_seer notify, not the Player notify flag"
    );

    map.entity_world
        .get_mut(&viewpoint_guid)
        .unwrap()
        .object_mut()
        .relocate(Position::xyz(1.0e9, 1.0e9, 0.0));
    map.entity_world
        .get_mut(&viewpoint_guid)
        .unwrap()
        .object_mut()
        .object_mut()
        .add_to_notify(ObjectNotifyFlags::VISIBILITY_CHANGED);

    let skipped = map.delayed_unit_relocation_for_cells_like_cpp([cell], []);
    assert_eq!(skipped.cell_plans.len(), 1);
    assert!(skipped.cell_plans[0].plan.player_relocations.is_empty());
    assert_eq!(
        skipped.cell_plans[0].plan.skipped_invalid_viewpoints,
        vec![player_guid]
    );
}
#[test]
fn delayed_unit_relocation_visibility_plans_filter_player_seers_like_cpp() {
    let mut map = test_map();
    let source_player = world_object_with_counter(HighGuid::Player, 4440301, 571, 7, false);
    let source_player_guid = source_player.guid();
    let target_needs_notify = world_object_with_counter(HighGuid::Player, 4440302, 571, 7, false);
    let target_needs_notify_guid = target_needs_notify.guid();
    let target_clear = world_object_with_counter(HighGuid::Player, 4440303, 571, 7, false);
    let target_clear_guid = target_clear.guid();
    let cell = map
        .add_to_map_like_cpp(AccessorObjectKind::Player, source_player)
        .unwrap()
        .cell;
    map.add_to_map_like_cpp(AccessorObjectKind::Player, target_needs_notify)
        .unwrap();
    map.add_to_map_like_cpp(AccessorObjectKind::Player, target_clear)
        .unwrap();
    for guid in [source_player_guid, target_needs_notify_guid] {
        map.entity_world
            .get_mut(&guid)
            .unwrap()
            .object_mut()
            .object_mut()
            .add_to_notify(ObjectNotifyFlags::VISIBILITY_CHANGED);
    }

    let delayed_plan = map.delayed_unit_relocation_for_cells_like_cpp([cell], []);
    let visibility_plans = map.delayed_unit_relocation_visibility_plans_like_cpp(
        &delayed_plan,
        map.delayed_player_relocation_contexts_from_plan_like_cpp(&delayed_plan),
        std::iter::empty::<DelayedCreatureRelocationContext>(),
    );
    let source_plan = visibility_plans
        .player_plans
        .iter()
        .find(|plan| plan.player_guid == source_player_guid)
        .unwrap();

    assert!(
        !source_plan
            .visibility_plan
            .reciprocal_player_updates
            .contains(&target_needs_notify_guid)
    );
    assert!(
        source_plan
            .visibility_plan
            .reciprocal_player_updates
            .contains(&target_clear_guid)
    );
}

#[test]
fn object_update_plan_for_current_tick_visits_only_nearby_map_objects_like_cpp() {
    let mut map = test_map();

    let mut player = test_player_for_viewpoint(4440401);
    let player_guid = player.guid();
    player
        .unit_mut()
        .world_mut()
        .object_mut()
        .remove_from_world();
    map.add_map_object_record_to_map_like_cpp(MapObjectRecord::new_player(player).unwrap())
        .unwrap();

    let mut nearby_creature = test_creature_for_spawn(44404, 4440402, true);
    nearby_creature
        .unit_mut()
        .world_mut()
        .object_mut()
        .remove_from_world();
    nearby_creature
        .unit_mut()
        .world_mut()
        .relocate(Position::xyz(20.0, 20.0, 30.0));
    let nearby_creature_guid = nearby_creature.guid();
    map.add_map_object_record_to_map_like_cpp(
        MapObjectRecord::new_creature(nearby_creature).unwrap(),
    )
    .unwrap();

    let mut distant_creature = test_creature_for_spawn(44404, 4440403, true);
    distant_creature
        .unit_mut()
        .world_mut()
        .object_mut()
        .remove_from_world();
    distant_creature
        .unit_mut()
        .world_mut()
        .relocate(Position::xyz(400.0, 20.0, 30.0));
    let distant_creature_guid = distant_creature.guid();
    map.add_map_object_record_to_map_like_cpp(
        MapObjectRecord::new_creature(distant_creature).unwrap(),
    )
    .unwrap();

    let mut nearby_gameobject = test_gameobject_for_spawn(44404, 4440404);
    nearby_gameobject
        .world_mut()
        .object_mut()
        .remove_from_world();
    nearby_gameobject
        .world_mut()
        .relocate(Position::xyz(25.0, 20.0, 30.0));
    let nearby_gameobject_guid = nearby_gameobject.world().guid();
    map.add_map_object_record_to_map_like_cpp(
        MapObjectRecord::new_game_object(nearby_gameobject).unwrap(),
    )
    .unwrap();

    let plan = map.object_update_plan_for_current_tick_like_cpp(37);

    assert_eq!(plan.diff_ms, 37);
    assert!(plan.update_guids.contains(&nearby_creature_guid));
    assert!(plan.update_guids.contains(&nearby_gameobject_guid));
    assert!(!plan.update_guids.contains(&distant_creature_guid));
    assert!(!plan.update_guids.contains(&player_guid));
}

#[test]
fn relocation_reuses_object_updater_far_player_sources_like_cpp() {
    let mut map = test_map();
    let mut player = test_player_for_viewpoint(4440451);
    player
        .unit_mut()
        .world_mut()
        .object_mut()
        .remove_from_world();
    let player_guid = player.guid();

    let mut far_creature = test_creature_for_spawn(44405, 4440452, true);
    far_creature
        .unit_mut()
        .world_mut()
        .object_mut()
        .remove_from_world();
    far_creature
        .unit_mut()
        .world_mut()
        .relocate(Position::xyz(180.0, 20.0, 30.0));
    let far_creature_guid = far_creature.guid();

    map.add_map_object_record_to_map_like_cpp(MapObjectRecord::new_player(player).unwrap())
        .unwrap();
    map.add_map_object_record_to_map_like_cpp(MapObjectRecord::new_creature(far_creature).unwrap())
        .unwrap();
    map.get_typed_player_mut(player_guid)
        .unwrap()
        .unit_mut()
        .subsystems_mut()
        .combat
        .set_in_combat_with(far_creature_guid, false, false);

    let sources = map.map_update_player_sources_for_current_tick_like_cpp();
    let source = sources
        .iter()
        .find(|source| source.player_guid == player_guid)
        .expect("in-world player source");
    assert_eq!(source.far_combat_unit_guids, vec![far_creature_guid]);

    let visit =
        map.map_update_visit_plan_like_cpp(sources, std::iter::empty(), std::iter::empty(), 1);
    assert!(visit.nearby_visit_centers.contains(&far_creature_guid));
    assert_eq!(
        map.grid_activation_range_for_guid_like_cpp(far_creature_guid),
        wow_entities::DEFAULT_MONSTER_SIGHT_DISTANCE
    );
}

#[test]
fn delayed_unit_relocation_visibility_plans_use_cpp_max_visibility_visits() {
    let mut map = test_map();
    let source_creature = world_object_with_counter(HighGuid::Creature, 1, 571, 7, false);
    let source_creature_guid = source_creature.guid();
    let other_creature = world_object_with_counter(HighGuid::Creature, 2, 571, 7, false);
    let other_creature_guid = other_creature.guid();
    let notified_creature = world_object_with_counter(HighGuid::Creature, 3, 571, 7, false);
    let notified_creature_guid = notified_creature.guid();
    let player_notify = world_object_with_counter(HighGuid::Player, 4, 571, 7, false);
    let player_notify_guid = player_notify.guid();
    let player_normal = world_object_with_counter(HighGuid::Player, 5, 571, 7, false);
    let player_normal_guid = player_normal.guid();
    let old_player = guid(HighGuid::Player, 6);
    let old_creature = guid(HighGuid::Creature, 7);

    let cell = map
        .add_to_map_like_cpp(AccessorObjectKind::Creature, source_creature)
        .unwrap()
        .cell;
    map.add_to_map_like_cpp(AccessorObjectKind::Creature, other_creature)
        .unwrap();
    map.add_to_map_like_cpp(AccessorObjectKind::Creature, notified_creature)
        .unwrap();
    map.add_to_map_like_cpp(AccessorObjectKind::Player, player_notify)
        .unwrap();
    map.add_to_map_like_cpp(AccessorObjectKind::Player, player_normal)
        .unwrap();
    for guid in [
        source_creature_guid,
        notified_creature_guid,
        player_notify_guid,
    ] {
        map.entity_world
            .get_mut(&guid)
            .unwrap()
            .object_mut()
            .object_mut()
            .add_to_notify(ObjectNotifyFlags::VISIBILITY_CHANGED);
    }

    let delayed_plan = map.delayed_unit_relocation_for_cells_like_cpp([cell], []);
    let plans = map.delayed_unit_relocation_visibility_plans_like_cpp(
        &delayed_plan,
        [DelayedPlayerRelocationContext {
            player_guid: player_notify_guid,
            viewpoint_guid: player_notify_guid,
            previous_client_guids: vec![old_player, old_creature],
            relocated_for_ai: true,
        }],
        [
            DelayedCreatureRelocationContext {
                creature_guid: source_creature_guid,
                source_creature_alive: true,
            },
            DelayedCreatureRelocationContext {
                creature_guid: notified_creature_guid,
                source_creature_alive: true,
            },
        ],
    );

    assert_eq!(plans.creature_plans.len(), 2);
    let source_plan = plans
        .creature_plans
        .iter()
        .find(|plan| plan.creature_guid == source_creature_guid)
        .unwrap();
    assert_eq!(source_plan.cell_coord, cell);
    assert!(
        source_plan
            .visibility_plan
            .player_visibility_updates
            .contains(&player_normal_guid)
    );
    assert!(
        !source_plan
            .visibility_plan
            .player_visibility_updates
            .contains(&player_notify_guid)
    );
    assert!(
        source_plan
            .visibility_plan
            .ai_relocation_checks
            .contains(&(source_creature_guid, other_creature_guid))
    );
    assert!(
        source_plan
            .visibility_plan
            .ai_relocation_checks
            .contains(&(other_creature_guid, source_creature_guid))
    );
    assert!(
        !source_plan
            .visibility_plan
            .ai_relocation_checks
            .contains(&(notified_creature_guid, source_creature_guid))
    );

    assert_eq!(plans.player_plans.len(), 1);
    let player_plan = &plans.player_plans[0];
    assert_eq!(player_plan.player_guid, player_notify_guid);
    assert_eq!(player_plan.viewpoint_guid, player_notify_guid);
    assert!(
        player_plan
            .visibility_plan
            .out_of_range_guids
            .contains(&old_player)
    );
    assert!(
        player_plan
            .visibility_plan
            .out_of_range_guids
            .contains(&old_creature)
    );
    assert!(
        !player_plan
            .visibility_plan
            .ai_relocation_checks
            .contains(&(source_creature_guid, player_notify_guid))
    );
    assert!(
        player_plan
            .visibility_plan
            .ai_relocation_checks
            .contains(&(other_creature_guid, player_notify_guid))
    );
    assert!(
        !player_plan
            .visibility_plan
            .ai_relocation_checks
            .contains(&(notified_creature_guid, player_notify_guid))
    );
}
#[test]
fn delayed_unit_relocation_visibility_plans_report_missing_player_contexts_like_cpp_gap() {
    let mut map = test_map();
    let player = world_object_with_counter(HighGuid::Player, 1, 571, 7, false);
    let player_guid = player.guid();
    let cell = map
        .add_to_map_like_cpp(AccessorObjectKind::Player, player)
        .unwrap()
        .cell;
    map.entity_world
        .get_mut(&player_guid)
        .unwrap()
        .object_mut()
        .object_mut()
        .add_to_notify(ObjectNotifyFlags::VISIBILITY_CHANGED);

    let delayed_plan = map.delayed_unit_relocation_for_cells_like_cpp([cell], []);
    let plans = map.delayed_unit_relocation_visibility_plans_like_cpp(
        &delayed_plan,
        std::iter::empty::<DelayedPlayerRelocationContext>(),
        std::iter::empty::<DelayedCreatureRelocationContext>(),
    );

    assert!(plans.player_plans.is_empty());
    assert_eq!(plans.missing_player_contexts, vec![player_guid]);
}
#[test]
fn player_phase_loading_invokes_personal_phase_tracker_before_activation() {
    let mut store = crate::spawn::SpawnStore::new();
    let spawn = crate::spawn::SpawnData {
        object_type: crate::spawn::SpawnObjectType::Creature,
        spawn_id: 100,
        map_id: 571,
        db_data: true,
        spawn_group: crate::spawn::SpawnGroupTemplateData::default_group(),
        id: 42,
        spawn_point: crate::spawn::SpawnPosition::new(0.0, 0.0, 1.0, 2.0),
        phase_use_flags: 0,
        phase_id: 9,
        phase_group: 0,
        terrain_swap_map: -1,
        pool_id: 0,
        spawn_time_secs: 120,
        spawn_difficulties: vec![1],
        script_id: 0,
        string_id: String::new(),
    };
    store.add_object_spawn(&spawn, |phase_id| phase_id == 9);
    let corpses = crate::object_grid_loader::CorpseCellStore::new();
    let mut loader =
        crate::object_grid_loader::ObjectGridLoader::new(&store, &corpses, 571, 1, 1, 1);
    let owner = ObjectGuid::create_player(1, 100);
    let phase_shift = crate::personal_phase::PhaseShift::new(
        Some(owner),
        vec![crate::personal_phase::PhaseRef::new(9, true)],
    );
    let mut map = test_map();
    let cell = cell_from_grid_center(GridCoord::new(32, 32));

    assert!(map.ensure_grid_loaded_for_player_phase(&cell, &phase_shift, &mut loader));

    let grid = map.get_ngrid(GridCoord::new(32, 32)).unwrap();
    assert_eq!(grid.state(), GridStateKind::Active);
    assert_eq!(
        grid.get_grid_type(0, 0)
            .unwrap()
            .grid_objects
            .creatures
            .len(),
        1
    );
    assert_eq!(map.personal_phase_tracker().tracker_count(), 1);
}
#[test]
fn unload_grid_purges_personal_phase_tracker_before_unloader_like_cpp() {
    let mut store = crate::spawn::SpawnStore::new();
    let spawn = crate::spawn::SpawnData {
        object_type: crate::spawn::SpawnObjectType::Creature,
        spawn_id: 4183,
        map_id: 571,
        db_data: true,
        spawn_group: crate::spawn::SpawnGroupTemplateData::default_group(),
        id: 42,
        spawn_point: crate::spawn::SpawnPosition::new(0.0, 0.0, 1.0, 2.0),
        phase_use_flags: 0,
        phase_id: 9,
        phase_group: 0,
        terrain_swap_map: -1,
        pool_id: 0,
        spawn_time_secs: 120,
        spawn_difficulties: vec![1],
        script_id: 0,
        string_id: String::new(),
    };
    store.add_object_spawn(&spawn, |phase_id| phase_id == 9);
    let corpses = crate::object_grid_loader::CorpseCellStore::new();
    let mut loader =
        crate::object_grid_loader::ObjectGridLoader::new(&store, &corpses, 571, 1, 1, 1);
    let owner = ObjectGuid::create_player(1, 4183);
    let phase_shift = crate::personal_phase::PhaseShift::new(
        Some(owner),
        vec![crate::personal_phase::PhaseRef::new(9, true)],
    );
    let mut map = test_map();
    let coord = GridCoord::new(32, 32);
    let cell = cell_from_grid_center(coord);

    assert!(map.ensure_grid_loaded_for_player_phase(&cell, &phase_shift, &mut loader));
    assert_eq!(map.personal_phase_tracker().tracker_count(), 1);

    assert!(map.unload_grid_at(coord, true));

    assert_eq!(map.personal_phase_tracker().tracker_count(), 0);
    assert!(map.get_ngrid(coord).is_none());
}
