//! Managed-map lifecycle and updater regression scenarios, part 1 of 4.
//!
//! Moved out of the manager.rs root under #644; every test is unchanged.

use super::*;

#[test]
fn game_time_now_ms_uses_runtime_monotonic_subsecond_counter() {
    assert_eq!(game_time_elapsed_ms_u64(Duration::from_millis(999)), 999);
    assert_eq!(
        game_time_elapsed_ms_u64(Duration::from_millis(1_001)),
        1_001
    );

    let first = game_time_now_ms_u64();
    let second = game_time_now_ms_u64();
    assert!(second >= first);
}

#[test]
fn map_manager_update_consumes_dynamic_tree_before_instrumented_tail_like_cpp() {
    let mut manager = MapManager::new(MIN_GRID_DELAY_MS, 1);
    manager.create_world_map(1, 0);
    let map = manager.find_map_mut(1, 0).unwrap();
    map.map_mut()
        .set_dynamic_tree_model_count_for_tests_like_cpp(1);
    map.map_mut()
        .mark_dynamic_tree_unbalanced_for_tests_like_cpp(2);

    manager.update(199);
    let map = manager.find_map(1, 0).unwrap();
    let first = map.last_dynamic_tree_update_summary_like_cpp();
    assert_eq!(first.diff_ms, 199);
    assert!(!first.empty);
    assert_eq!(first.timer_before_ms, 200);
    assert_eq!(first.timer_after_ms, 1);
    assert!(!first.timer_passed);
    assert_eq!(first.unbalanced_before, 2);
    assert_eq!(first.unbalanced_after, 2);
    assert_eq!(map.update_calls(), &[199]);

    manager.update(1);
    let map = manager.find_map(1, 0).unwrap();
    let second = map.last_dynamic_tree_update_summary_like_cpp();
    assert_eq!(second.diff_ms, 1);
    assert_eq!(second.timer_before_ms, 1);
    assert_eq!(second.timer_after_ms, 200);
    assert!(second.timer_passed);
    assert_eq!(second.timer_reset_to_ms, Some(200));
    assert_eq!(second.unbalanced_before, 2);
    assert!(second.balanced);
    assert_eq!(second.unbalanced_after, 0);
    assert_eq!(map.update_calls(), &[199, 1]);
}

#[test]
fn delays_are_clamped_like_map_manager_h() {
    let manager = MapManager::new(1, 0);

    assert_eq!(manager.grid_cleanup_delay_ms(), MIN_GRID_DELAY_MS);
}

#[test]
fn create_and_find_map_uses_cpp_map_key_shape() {
    let mut manager = MapManager::default();

    manager.create_world_map(1, 0);

    let map = manager.find_map(1, 0).unwrap();
    assert_eq!(map.map_id(), 1);
    assert_eq!(map.instance_id(), 0);
    assert!(manager.find_map(1, 1).is_none());
}

#[test]
fn managed_map_tracks_represented_instance_encounter_progress() {
    let mut manager = MapManager::default();
    let map = manager.create_map_entry(
        631,
        9001,
        3,
        ManagedMapKind::Dungeon {
            has_reset_schedule: false,
        },
    );

    assert!(!map.instance_encounter_in_progress_like_cpp());
    map.set_instance_encounter_in_progress_like_cpp(true);
    assert!(map.instance_encounter_in_progress_like_cpp());
}

#[test]
fn map_manager_init_spawn_group_state_hook_runs_once_for_new_maps_only() {
    let mut manager = MapManager::default();
    let calls = Arc::new(AtomicUsize::new(0));
    let hook_calls = Arc::clone(&calls);
    manager.set_spawn_group_initializer_like_cpp(move |map| {
        hook_calls.fetch_add(1, Ordering::SeqCst);
        map.set_instance_encounter_in_progress_like_cpp(true);
    });

    manager.create_world_map(571, 0);
    manager.create_world_map(571, 0);
    manager.create_map_entry(571, 0, 0, ManagedMapKind::World);

    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert!(
        manager
            .find_map(571, 0)
            .unwrap()
            .instance_encounter_in_progress_like_cpp()
    );

    manager.create_map_entry(
        571,
        9,
        1,
        ManagedMapKind::Dungeon {
            has_reset_schedule: false,
        },
    );
    assert_eq!(calls.load(Ordering::SeqCst), 2);

    manager.clear_spawn_group_initializer_like_cpp();
    manager.create_world_map(1, 0);
    assert_eq!(calls.load(Ordering::SeqCst), 2);
}

#[test]
fn map_manager_init_spawn_group_state_hook_can_mutate_managed_map_spawn_groups() {
    let manual = SpawnGroupTemplateData {
        group_id: 10,
        name: "manual".to_string(),
        map_id: 571,
        flags: SpawnGroupFlags::MANUAL_SPAWN,
    };
    let automatic = SpawnGroupTemplateData {
        group_id: 11,
        name: "automatic".to_string(),
        map_id: 571,
        flags: SpawnGroupFlags::NONE,
    };
    let system = SpawnGroupTemplateData {
        group_id: 12,
        name: "system".to_string(),
        map_id: 571,
        flags: SpawnGroupFlags::SYSTEM,
    };
    let groups = Arc::new(vec![manual.clone(), automatic.clone(), system.clone()]);

    let mut manager = MapManager::default();
    manager.set_spawn_group_initializer_like_cpp({
        let groups = Arc::clone(&groups);
        move |managed_map| {
            managed_map
                .map_mut()
                .init_spawn_group_state_like_cpp(groups.iter(), |group| {
                    group.group_id == manual.group_id
                });
        }
    });

    manager.create_world_map(571, 0);
    let map = manager.find_map(571, 0).unwrap().map();

    assert!(map.is_spawn_group_active_like_cpp(Some(&groups[0])));
    assert!(!map.is_spawn_group_active_like_cpp(Some(&groups[1])));
    assert!(map.is_spawn_group_active_like_cpp(Some(&groups[2])));
}

#[test]
fn do_for_all_maps_with_map_id_uses_ordered_pair_range() {
    let mut manager = MapManager::default();
    manager.create_world_map(1, 0);
    manager.create_world_map(2, 0);
    manager.create_map_entry(1, 3, 0, ManagedMapKind::World);

    let mut keys = Vec::new();
    manager.do_for_all_maps_with_map_id(1, |map| {
        keys.push((map.map_id(), map.instance_id()));
    });

    assert_eq!(keys, vec![(1, 0), (1, 3)]);
}

#[test]
fn do_for_all_maps_mut_visits_maps_in_btreemap_order_and_allows_mutation() {
    let mut manager = MapManager::default();
    manager.create_world_map(2, 0);
    manager.create_map_entry(1, 3, 0, ManagedMapKind::World);
    manager.create_world_map(1, 0);

    let mut keys = Vec::new();
    manager.do_for_all_maps_mut(|map| {
        keys.push((map.map_id(), map.instance_id()));
        map.set_instance_encounter_in_progress_like_cpp(
            (map.map_id() + map.instance_id()) % 2 == 0,
        );
    });

    assert_eq!(keys, vec![(1, 0), (1, 3), (2, 0)]);
    assert!(
        !manager
            .find_map(1, 0)
            .unwrap()
            .instance_encounter_in_progress_like_cpp()
    );
    assert!(
        manager
            .find_map(1, 3)
            .unwrap()
            .instance_encounter_in_progress_like_cpp()
    );
    assert!(
        manager
            .find_map(2, 0)
            .unwrap()
            .instance_encounter_in_progress_like_cpp()
    );
}

#[test]
fn update_waits_for_interval_then_updates_and_delayed_updates_maps() {
    let mut manager = MapManager::new(MIN_GRID_DELAY_MS, 10);
    manager.create_world_map(1, 0);

    assert_eq!(manager.update(9), None);
    assert!(manager.find_map(1, 0).unwrap().update_calls().is_empty());

    assert_eq!(manager.update(1), Some(10));

    let map = manager.find_map(1, 0).unwrap();
    assert_eq!(map.update_calls(), &[10]);
    assert_eq!(map.delayed_update_calls(), &[10]);
}

#[test]
fn live_delayed_update_consumes_removal_grid_state_after_remove_list_like_cpp() {
    let mut manager = MapManager::new(MIN_GRID_DELAY_MS, 1);
    manager.create_world_map(1, 0);
    let position = Position::xyz(3_000.0, 3_000.0, 0.0);
    {
        let managed_map = manager.find_map_mut(1, 0).unwrap();
        assert!(managed_map.map_mut().load_grid(position.x, position.y));
        let cell = crate::map::cell_from_world(position.x, position.y);
        let coord = GridCoord::new(cell.grid_x(), cell.grid_y());
        let grid = managed_map.map_mut().get_ngrid_mut(coord).unwrap();
        grid.set_state(GridStateKind::Removal);
        grid.info_mut().reset_time_tracker(1);
        assert!(managed_map.map().get_ngrid(coord).is_some());
    }

    assert_eq!(manager.update(1), Some(1));

    let managed_map = manager.find_map(1, 0).unwrap();
    let cell = crate::map::cell_from_world(position.x, position.y);
    let coord = GridCoord::new(cell.grid_x(), cell.grid_y());
    let summary = managed_map.last_grid_states_update_summary_like_cpp();
    assert_eq!(managed_map.delayed_update_calls(), &[1]);
    assert_eq!(summary.diff_ms, 1);
    assert_eq!(summary.visited, 1);
    assert_eq!(summary.updated, 1);
    assert_eq!(summary.unloaded, 1);
    assert_eq!(summary.removal_unloaded, 1);
    assert!(!summary.skipped_battleground_or_arena);
    assert!(managed_map.map().get_ngrid(coord).is_none());
}

#[test]
fn battleground_delayed_update_skips_grid_state_update_like_cpp() {
    let mut manager = MapManager::new(MIN_GRID_DELAY_MS, 1);
    manager.create_map_entry(1, 33, 0, ManagedMapKind::Battleground);
    let position = Position::xyz(3_100.0, 3_100.0, 0.0);
    {
        let managed_map = manager.find_map_mut(1, 33).unwrap();
        assert!(managed_map.map_mut().load_grid(position.x, position.y));
        let cell = crate::map::cell_from_world(position.x, position.y);
        let coord = GridCoord::new(cell.grid_x(), cell.grid_y());
        let grid = managed_map.map_mut().get_ngrid_mut(coord).unwrap();
        grid.set_state(GridStateKind::Removal);
        grid.info_mut().reset_time_tracker(1);
    }

    assert_eq!(manager.update(1), Some(1));

    let managed_map = manager.find_map(1, 33).unwrap();
    let cell = crate::map::cell_from_world(position.x, position.y);
    let coord = GridCoord::new(cell.grid_x(), cell.grid_y());
    let summary = managed_map.last_grid_states_update_summary_like_cpp();
    assert_eq!(summary.diff_ms, 1);
    assert!(summary.skipped_battleground_or_arena);
    assert_eq!(summary.visited, 0);
    assert_eq!(summary.unloaded, 0);
    assert_eq!(
        managed_map.map().get_ngrid(coord).unwrap().state(),
        GridStateKind::Removal
    );
}

#[test]
fn live_update_delayed_update_drains_dynamic_object_remove_list_like_cpp() {
    let mut manager = MapManager::new(MIN_GRID_DELAY_MS, 1);
    manager.create_world_map(1, 0);
    let dynamic_object_guid = guid(HighGuid::DynamicObject, 4320101, 1, 0);
    let mut dynamic_object = DynamicObject::new(true);
    dynamic_object
        .world_mut()
        .object_mut()
        .create(dynamic_object_guid);
    dynamic_object.world_mut().set_map(1, 0).unwrap();
    dynamic_object
        .world_mut()
        .relocate(Position::xyz(11.0, 21.0, 31.0));
    dynamic_object.world_mut().object_mut().add_to_world();

    {
        let managed_map = manager.find_map_mut(1, 0).unwrap();
        managed_map
            .map_mut()
            .add_map_object_record_to_map_like_cpp(
                MapObjectRecord::new_dynamic_object(dynamic_object).unwrap(),
            )
            .unwrap();
        let queued = managed_map
            .map_mut()
            .add_object_to_remove_list_like_cpp(dynamic_object_guid);
        assert!(queued.queued);
        assert!(
            managed_map
                .map()
                .map_object_record(dynamic_object_guid)
                .is_some()
        );
        assert_eq!(managed_map.map().objects_to_remove_count_like_cpp(), 1);
    }

    assert_eq!(manager.update(1), Some(1));

    let managed_map = manager.find_map(1, 0).unwrap();
    assert_eq!(managed_map.delayed_update_calls(), &[1]);
    assert_eq!(managed_map.map().objects_to_remove_count_like_cpp(), 0);
    assert!(
        managed_map
            .map()
            .map_object_record(dynamic_object_guid)
            .is_none()
    );
}

#[test]
fn map_manager_update_far_spell_callback_queues_remove_before_delayed_remove_drain_like_cpp() {
    let mut manager = MapManager::new(MIN_GRID_DELAY_MS, 1);
    manager.create_world_map(1, 0);
    let dynamic_object_guid = guid(HighGuid::DynamicObject, 487_0101, 1, 0);
    let mut dynamic_object = DynamicObject::new(true);
    dynamic_object
        .world_mut()
        .object_mut()
        .create(dynamic_object_guid);
    dynamic_object.world_mut().set_map(1, 0).unwrap();
    dynamic_object
        .world_mut()
        .relocate(Position::xyz(11.0, 21.0, 31.0));
    dynamic_object.world_mut().object_mut().add_to_world();
    dynamic_object.set_duration(10_000);

    {
        let managed_map = manager.find_map_mut(1, 0).unwrap();
        managed_map
            .map_mut()
            .add_map_object_record_to_map_like_cpp(
                MapObjectRecord::new_dynamic_object(dynamic_object).unwrap(),
            )
            .unwrap();
        managed_map
            .map_mut()
            .add_far_spell_callback_like_cpp(RepresentedFarSpellCallbackLikeCpp {
                id: 487,
                action: RepresentedFarSpellCallbackActionLikeCpp::QueueObjectRemove {
                    guid: dynamic_object_guid,
                },
            });
        assert_eq!(managed_map.map().far_spell_callbacks_count_like_cpp(), 1);
        assert_eq!(managed_map.map().objects_to_remove_count_like_cpp(), 0);
        assert!(
            managed_map
                .map()
                .map_object_record(dynamic_object_guid)
                .is_some()
        );
    }

    assert_eq!(manager.update(1), Some(1));

    let managed_map = manager.find_map(1, 0).unwrap();
    let far_spell = managed_map.last_far_spell_callback_drain_summary_like_cpp();
    assert_eq!(far_spell.processed, 1);
    assert_eq!(far_spell.remove_queued, 1);
    assert_eq!(far_spell.queued_after, 0);
    assert_eq!(managed_map.map().objects_to_remove_count_like_cpp(), 0);
    assert!(
        managed_map
            .map()
            .map_object_record(dynamic_object_guid)
            .is_none()
    );
    assert_eq!(
        managed_map
            .last_grid_states_update_summary_like_cpp()
            .diff_ms,
        1
    );
}

#[test]
fn map_manager_update_visits_live_dynamic_object_without_expiry_like_cpp() {
    let mut manager = MapManager::new(MIN_GRID_DELAY_MS, 1);
    manager.create_world_map(1, 0);
    let dynamic_object_guid = insert_dynamic_object_for_update(&mut manager, 4330101, 10, true);

    assert_eq!(manager.update(1), Some(1));

    let managed_map = manager.find_map(1, 0).unwrap();
    assert_eq!(managed_map.update_calls(), &[1]);
    assert_eq!(managed_map.delayed_update_calls(), &[1]);
    assert_eq!(managed_map.map().objects_to_remove_count_like_cpp(), 0);
    assert_eq!(
        managed_map.last_dynamic_objects_update_summary(),
        DynamicObjectsUpdateSummaryLikeCpp {
            visited: 1,
            updated: 1,
            expired_remove_queued: 0,
            missing_or_stale: 0,
            not_dynamic_object: 0,
            not_in_world: 0,
        }
    );
    let dynamic_object = managed_map
        .map()
        .get_typed_dynamic_object(dynamic_object_guid)
        .unwrap();
    assert_eq!(dynamic_object.duration_ms(), 9);
}

#[test]
fn map_manager_update_expires_dynamic_object_then_delayed_update_drains_like_cpp() {
    let mut manager = MapManager::new(MIN_GRID_DELAY_MS, 1);
    manager.create_world_map(1, 0);
    let dynamic_object_guid = insert_dynamic_object_for_update(&mut manager, 4330201, 1, true);

    assert_eq!(manager.update(1), Some(1));

    let managed_map = manager.find_map(1, 0).unwrap();
    assert_eq!(managed_map.update_calls(), &[1]);
    assert_eq!(managed_map.delayed_update_calls(), &[1]);
    assert_eq!(managed_map.map().objects_to_remove_count_like_cpp(), 0);
    assert_eq!(
        managed_map.last_dynamic_objects_update_summary(),
        DynamicObjectsUpdateSummaryLikeCpp {
            visited: 1,
            updated: 0,
            expired_remove_queued: 1,
            missing_or_stale: 0,
            not_dynamic_object: 0,
            not_in_world: 0,
        }
    );
    assert!(
        managed_map
            .map()
            .map_object_record(dynamic_object_guid)
            .is_none()
    );
}

#[test]
fn map_manager_update_personal_phase_expiry_enqueues_and_delayed_update_drains_like_cpp() {
    let mut manager = MapManager::new(MIN_GRID_DELAY_MS, 1);
    manager.create_world_map(1, 0);
    let creature_guid = insert_creature_for_update(&mut manager, 4400301, true);
    let owner = ObjectGuid::create_player(1, 44003);
    {
        let map = manager.find_map_mut(1, 0).unwrap().map_mut();
        map.register_personal_phase_object_for_test(44, owner, creature_guid);
        map.mark_personal_phases_for_deletion_for_test(owner);
    }

    assert_eq!(manager.update(60_000), Some(60_000));

    let managed_map = manager.find_map(1, 0).unwrap();
    assert_eq!(managed_map.update_calls(), &[60_000]);
    assert_eq!(managed_map.delayed_update_calls(), &[60_000]);
    assert_eq!(
        managed_map.last_personal_phase_tracker_update_summary(),
        PersonalPhaseTrackerUpdateSummaryLikeCpp {
            expired_objects: 1,
            remove_queued: 1,
            missing_or_stale: 0,
            unsupported_kinds: 0,
            duplicate_queued: 0,
        }
    );
    assert_eq!(managed_map.map().objects_to_remove_count_like_cpp(), 0);
    assert!(managed_map.map().map_object_record(creature_guid).is_none());
}

#[test]
fn map_manager_update_processes_weather_between_scripts_and_personal_phase_like_cpp() {
    let mut manager = MapManager::new(MIN_GRID_DELAY_MS, 1);
    manager.create_world_map(1, 0);
    let creature_guid = insert_creature_for_update(&mut manager, 4460101, true);
    let owner = ObjectGuid::create_player(1, 44601);
    {
        let map = manager.find_map_mut(1, 0).unwrap().map_mut();
        let schedule = map.schedule_represented_script_action_like_cpp(
            0,
            1,
            ObjectGuid::create_player(1, 44602),
            creature_guid,
            owner,
            446,
        );
        assert!(schedule.immediate_process.is_none());
        assert_eq!(map.represented_script_schedule_count_like_cpp(), 1);
        map.register_represented_zone_default_weather_for_test(447);
        map.register_personal_phase_object_for_test(446, owner, creature_guid);
        map.mark_personal_phases_for_deletion_for_test(owner);
    }

    assert_eq!(manager.update(60_000), Some(60_000));

    let managed_map = manager.find_map(1, 0).unwrap();
    let script_summary = managed_map.last_script_schedule_process_summary_like_cpp();
    assert_eq!(script_summary.queued_before, 1);
    assert_eq!(script_summary.processed, 1);
    assert_eq!(script_summary.remaining, 0);
    assert!(script_summary.lock_entered);
    assert_eq!(script_summary.processed_actions[0].command_id, 446);
    assert_eq!(
        managed_map
            .map()
            .represented_script_schedule_count_like_cpp(),
        0
    );
    let weather_summary = managed_map.last_weather_update_summary_like_cpp();
    assert!(weather_summary.timer_passed);
    assert_eq!(weather_summary.timer_current_before, 0);
    assert_eq!(weather_summary.timer_current_after_update, 60_000);
    assert_eq!(weather_summary.timer_current_after_reset, 0);
    assert_eq!(weather_summary.zones_seen, 1);
    assert_eq!(weather_summary.default_weather_updated, 1);
    assert_eq!(weather_summary.weather_update_call_diff_ms, Some(1_000));
    assert_eq!(
        managed_map
            .map()
            .represented_zone_default_weather_update_diffs_like_cpp(447),
        Some([1_000].as_slice())
    );
    assert_eq!(
        managed_map.last_personal_phase_tracker_update_summary(),
        PersonalPhaseTrackerUpdateSummaryLikeCpp {
            expired_objects: 1,
            remove_queued: 1,
            missing_or_stale: 0,
            unsupported_kinds: 0,
            duplicate_queued: 0,
        }
    );
    assert_eq!(
        managed_map
            .last_live_move_list_drain_summary_like_cpp()
            .creature
            .processed,
        0
    );
    assert!(managed_map.map().map_object_record(creature_guid).is_none());
}

#[test]
fn map_update_tail_empty_map_records_hook_and_zero_metrics_like_cpp() {
    let mut managed_map = ManagedMap::new(
        571,
        77,
        0,
        MIN_GRID_DELAY_MS.into(),
        ManagedMapKind::Dungeon {
            has_reset_schedule: true,
        },
    );

    assert_eq!(
        managed_map.last_map_update_tail_summary_like_cpp(),
        MapUpdateTailSummaryLikeCpp::default()
    );

    managed_map.update(37);

    assert_eq!(
        managed_map.last_map_update_tail_summary_like_cpp(),
        MapUpdateTailSummaryLikeCpp {
            script_hook: MapUpdateScriptHookSummaryLikeCpp {
                invoked: true,
                diff_ms: 37,
                map_id: 571,
                instance_id: 77,
                kind: ManagedMapKind::Dungeon {
                    has_reset_schedule: true,
                },
                script_dispatch_represented: false,
            },
            metrics: MapUpdateMetricsSummaryLikeCpp {
                creature_count: 0,
                gameobject_count: 0,
                map_id: 571,
                instance_id: 77,
            },
        }
    );
    assert_eq!(
        managed_map.last_process_relocation_notifies_outcome_like_cpp(),
        ProcessRelocationNotifiesOutcome::default()
    );
}

#[test]
fn map_update_tail_metrics_count_only_typed_creature_and_gameobject_like_cpp() {
    let mut manager = MapManager::new(MIN_GRID_DELAY_MS, 1);
    manager.create_world_map(1, 0);

    let _creature_guid = insert_creature_for_update(&mut manager, 4480101, true);
    let _game_object_guid = insert_game_object_for_update(&mut manager, 4480102, 0, true);
    let _dynamic_object_guid = insert_dynamic_object_for_update(&mut manager, 4480103, 10, true);
    let _area_trigger_guid = insert_area_trigger_for_update(&mut manager, 4480104, 10, true);
    let _transport_guid = insert_transport_for_update(&mut manager, 4480105, true, 1);
    let _player_guid =
        insert_player_for_relocation_notify(&mut manager, 4480106, Position::xyz(16.0, 26.0, 36.0));
    insert_generic_world_object_record_for_metrics(
        &mut manager,
        AccessorObjectKind::Creature,
        guid(HighGuid::Creature, 4480107, 1, 0),
        TypeId::Unit,
        TypeMask::UNIT,
    );
    insert_generic_world_object_record_for_metrics(
        &mut manager,
        AccessorObjectKind::GameObject,
        guid(HighGuid::GameObject, 4480108, 1, 0),
        TypeId::GameObject,
        TypeMask::GAME_OBJECT,
    );

    assert_eq!(manager.update(1), Some(1));

    let managed_map = manager.find_map(1, 0).unwrap();
    let tail = managed_map.last_map_update_tail_summary_like_cpp();
    assert_eq!(
        tail.script_hook,
        MapUpdateScriptHookSummaryLikeCpp {
            invoked: true,
            diff_ms: 1,
            map_id: 1,
            instance_id: 0,
            kind: ManagedMapKind::World,
            script_dispatch_represented: false,
        }
    );
    assert_eq!(
        tail.metrics,
        MapUpdateMetricsSummaryLikeCpp {
            creature_count: 1,
            gameobject_count: 1,
            map_id: 1,
            instance_id: 0,
        }
    );
}

#[test]
fn map_manager_update_empty_script_schedule_reports_noop_like_cpp() {
    let mut manager = MapManager::new(MIN_GRID_DELAY_MS, 1);
    manager.create_world_map(1, 0);

    assert_eq!(manager.update(1), Some(1));

    let managed_map = manager.find_map(1, 0).unwrap();
    let summary = managed_map.last_script_schedule_process_summary_like_cpp();
    assert!(summary.empty_noop);
    assert_eq!(summary.queued_before, 0);
    assert_eq!(summary.processed, 0);
    assert_eq!(summary.remaining, 0);
    assert!(!summary.lock_entered);
}

#[test]
fn map_manager_update_skips_not_in_world_dynamic_object_like_cpp() {
    let mut manager = MapManager::new(MIN_GRID_DELAY_MS, 1);
    manager.create_world_map(1, 0);
    let dynamic_object_guid = insert_dynamic_object_for_update(&mut manager, 4330301, 10, false);

    assert_eq!(manager.update(1), Some(1));

    let managed_map = manager.find_map(1, 0).unwrap();
    assert_eq!(managed_map.update_calls(), &[1]);
    assert_eq!(managed_map.delayed_update_calls(), &[1]);
    assert_eq!(managed_map.map().objects_to_remove_count_like_cpp(), 0);
    assert_eq!(
        managed_map.last_dynamic_objects_update_summary(),
        DynamicObjectsUpdateSummaryLikeCpp {
            visited: 1,
            updated: 0,
            expired_remove_queued: 0,
            missing_or_stale: 0,
            not_dynamic_object: 0,
            not_in_world: 1,
        }
    );
    let dynamic_object = managed_map
        .map()
        .get_typed_dynamic_object(dynamic_object_guid)
        .unwrap();
    assert_eq!(dynamic_object.duration_ms(), 10);
}

#[test]
fn map_manager_game_object_update_visits_live_game_object_without_expiry_like_cpp() {
    let mut manager = MapManager::new(MIN_GRID_DELAY_MS, 1);
    manager.create_world_map(1, 0);
    let game_object_guid = insert_game_object_for_update(&mut manager, 4380101, 0, true);

    assert_eq!(manager.update(1), Some(1));

    let managed_map = manager.find_map(1, 0).unwrap();
    assert_eq!(managed_map.update_calls(), &[1]);
    assert_eq!(managed_map.delayed_update_calls(), &[1]);
    assert_eq!(managed_map.map().objects_to_remove_count_like_cpp(), 0);
    assert_eq!(
        managed_map.last_game_objects_update_summary(),
        GameObjectsUpdateSummaryLikeCpp {
            visited: 1,
            updated: 1,
            despawn_remove_queued: 0,
            missing_or_stale: 0,
            not_game_object: 0,
            not_in_world: 0,
            linked_traps_removed: 0,
            loot_cleared: 0,
            goober_spell_casts_represented: 0,
            goober_users_cleared: 0,
            goober_state_reset: 0,
            goober_nodespawn_returns: 0,
            ..GameObjectsUpdateSummaryLikeCpp::default()
        }
    );
    let game_object = managed_map
        .map()
        .get_typed_game_object(game_object_guid)
        .unwrap();
    assert_eq!(game_object.despawn_delay(), 0);
}

#[test]
fn map_manager_game_object_update_expired_despawn_then_delayed_update_drains_like_cpp() {
    let mut manager = MapManager::new(MIN_GRID_DELAY_MS, 1);
    manager.create_world_map(1, 0);
    let game_object_guid = insert_game_object_for_update(&mut manager, 4380201, 1, true);
    {
        let game_object = manager
            .find_map_mut(1, 0)
            .unwrap()
            .map_mut()
            .get_typed_game_object_mut(game_object_guid)
            .unwrap();
        game_object.set_loot_state(LootState::Activated, Some(ObjectGuid::create_player(1, 2)));
        assert_eq!(game_object.loot_state(), LootState::Activated);
    }

    assert_eq!(manager.update(1), Some(1));

    let managed_map = manager.find_map(1, 0).unwrap();
    assert_eq!(managed_map.update_calls(), &[1]);
    assert_eq!(managed_map.delayed_update_calls(), &[1]);
    assert_eq!(managed_map.map().objects_to_remove_count_like_cpp(), 0);
    let mut expected_summary = GameObjectsUpdateSummaryLikeCpp {
        visited: 1,
        updated: 0,
        despawn_remove_queued: 1,
        missing_or_stale: 0,
        not_game_object: 0,
        not_in_world: 0,
        linked_traps_removed: 0,
        loot_cleared: 0,
        goober_spell_casts_represented: 0,
        goober_users_cleared: 0,
        goober_state_reset: 0,
        goober_nodespawn_returns: 0,
        generic_visual_despawn_represented: 1,
        ..GameObjectsUpdateSummaryLikeCpp::default()
    };
    expected_summary
        .generic_visual_despawn_guids
        .push(game_object_guid);
    assert_eq!(
        managed_map.last_game_objects_update_summary(),
        expected_summary
    );
    assert!(
        managed_map
            .map()
            .map_object_record(game_object_guid)
            .is_none()
    );
}

#[test]
fn map_manager_game_object_update_without_pool_context_keeps_remove_list_like_cpp() {
    let mut manager = MapManager::new(MIN_GRID_DELAY_MS, 1);
    manager.create_world_map(1, 0);
    let game_object_guid =
        insert_pool_compatible_owner_created_game_object_for_update(&mut manager, 4642101, 464);

    assert_eq!(manager.update(1), Some(1));

    let managed_map = manager.find_map(1, 0).unwrap();
    assert_eq!(managed_map.last_game_objects_update_summary(), {
        let mut expected_summary = GameObjectsUpdateSummaryLikeCpp {
            visited: 1,
            updated: 0,
            despawn_remove_queued: 1,
            despawn_pool_updated: 0,
            loot_cleared: 1,
            summoned_expired_deletes: 1,
            summoned_expired_respawn_time_zeroed: 1,
            summoned_expired_despawn_represented: 1,
            summoned_expired_go_state_ready: 1,
            generic_visual_despawn_represented: 1,
            ..GameObjectsUpdateSummaryLikeCpp::default()
        };
        expected_summary
            .generic_visual_despawn_guids
            .push(game_object_guid);
        expected_summary
    });
    // `MapManager::update` immediately follows `Map::Update` with
    // `Map::DelayedUpdate`, which drains the represented remove-list.
    assert_eq!(managed_map.map().objects_to_remove_count_like_cpp(), 0);
    assert!(
        managed_map
            .map()
            .map_object_record(game_object_guid)
            .is_none()
    );
    assert!(
        !managed_map
            .map()
            .pool_data_like_cpp()
            .is_spawned_gameobject_like_cpp(4642102)
    );
}
