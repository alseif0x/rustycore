//! Spawn object and group model regression scenarios, part 1 of 1.
//!
//! Moved out of the spawn.rs root under #644; every test is unchanged.

use super::*;

#[test]
fn respawn_info_rejects_area_trigger_and_zero_spawn_id_like_cpp() {
    let mut store = RespawnStoreLikeCpp::new();

    assert_eq!(
        store.add_respawn_info_like_cpp(respawn_info(SpawnObjectType::AreaTrigger, 10, 100)),
        AddRespawnInfoOutcomeLikeCpp::RejectedUnsupportedType
    );
    assert_eq!(
        store.get_respawn_time_like_cpp(SpawnObjectType::AreaTrigger, 10),
        0
    );
    assert_eq!(
        store.add_respawn_info_like_cpp(respawn_info(SpawnObjectType::Creature, 0, 100)),
        AddRespawnInfoOutcomeLikeCpp::RejectedZeroSpawnId
    );
    assert!(store.respawn_timer_keys_like_cpp().next().is_none());
}

#[test]
fn respawn_info_add_replace_and_later_reject_follow_cpp_ordering() {
    let mut store = RespawnStoreLikeCpp::new();

    assert_eq!(
        store.add_respawn_info_like_cpp(respawn_info(SpawnObjectType::Creature, 10, 200)),
        AddRespawnInfoOutcomeLikeCpp::Inserted
    );
    assert_eq!(
        store.add_respawn_info_like_cpp(respawn_info(SpawnObjectType::Creature, 10, 250)),
        AddRespawnInfoOutcomeLikeCpp::RejectedExistingSoonerOrEqual
    );
    assert_eq!(
        store.get_respawn_time_like_cpp(SpawnObjectType::Creature, 10),
        200
    );

    assert_eq!(
        store.add_respawn_info_like_cpp(respawn_info(SpawnObjectType::Creature, 10, 200)),
        AddRespawnInfoOutcomeLikeCpp::ReplacedExisting
    );
    assert_eq!(
        store.add_respawn_info_like_cpp(respawn_info(SpawnObjectType::Creature, 10, 150)),
        AddRespawnInfoOutcomeLikeCpp::ReplacedExisting
    );
    assert_eq!(
        store.get_respawn_time_like_cpp(SpawnObjectType::Creature, 10),
        150
    );
    assert_eq!(
        store.respawn_timer_keys_like_cpp().collect::<Vec<_>>(),
        vec![(SpawnObjectType::Creature, 10)]
    );
}

#[test]
fn respawn_info_remove_and_unload_keep_maps_and_queue_coherent() {
    let mut store = RespawnStoreLikeCpp::new();
    store.add_respawn_info_like_cpp(respawn_info(SpawnObjectType::Creature, 10, 100));
    store.add_respawn_info_like_cpp(respawn_info(SpawnObjectType::GameObject, 20, 90));

    let removed = store.remove_respawn_time_like_cpp(SpawnObjectType::Creature, 10);
    assert_eq!(removed.map(|info| info.spawn_id), Some(10));
    assert_eq!(
        store.get_respawn_time_like_cpp(SpawnObjectType::Creature, 10),
        0
    );
    assert_eq!(
        store.respawn_timer_keys_like_cpp().collect::<Vec<_>>(),
        vec![(SpawnObjectType::GameObject, 20)]
    );

    store.unload_all_respawn_infos_like_cpp();
    assert_eq!(
        store.get_respawn_time_like_cpp(SpawnObjectType::GameObject, 20),
        0
    );
    assert!(store.respawn_timer_keys_like_cpp().next().is_none());
}

#[test]
fn process_respawns_stops_at_first_future_timer_like_cpp() {
    let mut store = RespawnStoreLikeCpp::new();
    store.add_respawn_info_like_cpp(respawn_info(SpawnObjectType::Creature, 1, 200));
    store.add_respawn_info_like_cpp(respawn_info(SpawnObjectType::Creature, 2, 100));

    let actions = store.process_due_respawns_like_cpp(
        99,
        |_, _| None,
        |_| CheckRespawnOutcomeLikeCpp::Allowed,
    );

    assert!(actions.is_empty());
    assert_eq!(store.respawn_timer_keys_like_cpp().count(), 2);
}

#[test]
fn process_respawns_equal_time_ties_match_cpp_spawn_and_type_priority() {
    let mut store = RespawnStoreLikeCpp::new();
    store.add_respawn_info_like_cpp(respawn_info(SpawnObjectType::Creature, 5, 100));
    store.add_respawn_info_like_cpp(respawn_info(SpawnObjectType::GameObject, 5, 100));
    store.add_respawn_info_like_cpp(respawn_info(SpawnObjectType::Creature, 6, 100));
    store.add_respawn_info_like_cpp(respawn_info(SpawnObjectType::Creature, 1, 90));

    let actions = store.process_due_respawns_like_cpp(
        100,
        |_, _| None,
        |_| CheckRespawnOutcomeLikeCpp::Allowed,
    );

    assert_eq!(
        actions,
        vec![
            ProcessRespawnActionLikeCpp::DoRespawn {
                object_type: SpawnObjectType::Creature,
                spawn_id: 1,
                grid_id: 7,
            },
            ProcessRespawnActionLikeCpp::DoRespawn {
                object_type: SpawnObjectType::Creature,
                spawn_id: 6,
                grid_id: 7,
            },
            ProcessRespawnActionLikeCpp::DoRespawn {
                object_type: SpawnObjectType::GameObject,
                spawn_id: 5,
                grid_id: 7,
            },
            ProcessRespawnActionLikeCpp::DoRespawn {
                object_type: SpawnObjectType::Creature,
                spawn_id: 5,
                grid_id: 7,
            },
        ]
    );
}

#[test]
fn process_respawns_pool_branch_deletes_before_check_respawn_like_cpp() {
    let mut store = RespawnStoreLikeCpp::new();
    store.add_respawn_info_like_cpp(respawn_info(SpawnObjectType::Creature, 1, 100));
    let mut check_called = false;

    let actions = store.process_due_respawns_like_cpp(
        100,
        |object_type, spawn_id| {
            assert_eq!(object_type, SpawnObjectType::Creature);
            assert_eq!(spawn_id, 1);
            Some(55)
        },
        |_| {
            check_called = true;
            CheckRespawnOutcomeLikeCpp::Allowed
        },
    );

    assert_eq!(
        actions,
        vec![ProcessRespawnActionLikeCpp::UpdatePool {
            pool_id: 55,
            object_type: SpawnObjectType::Creature,
            spawn_id: 1,
        }]
    );
    assert!(!check_called);
    assert_eq!(
        store.get_respawn_time_like_cpp(SpawnObjectType::Creature, 1),
        0
    );
}

#[test]
fn process_respawns_check_true_deletes_and_plans_do_respawn_like_cpp() {
    let mut store = RespawnStoreLikeCpp::new();
    store.add_respawn_info_like_cpp(respawn_info(SpawnObjectType::GameObject, 2, 100));

    let actions = store.process_due_respawns_like_cpp(
        100,
        |_, _| None,
        |_| CheckRespawnOutcomeLikeCpp::Allowed,
    );

    assert_eq!(
        actions,
        vec![ProcessRespawnActionLikeCpp::DoRespawn {
            object_type: SpawnObjectType::GameObject,
            spawn_id: 2,
            grid_id: 7,
        }]
    );
    assert_eq!(
        store.get_respawn_time_like_cpp(SpawnObjectType::GameObject, 2),
        0
    );
}

#[test]
fn process_respawns_check_false_zero_deletes_entry_like_cpp() {
    let mut store = RespawnStoreLikeCpp::new();
    store.add_respawn_info_like_cpp(respawn_info(SpawnObjectType::Creature, 3, 100));

    let actions = store.process_due_respawns_like_cpp(
        100,
        |_, _| None,
        |info| {
            info.respawn_time = 0;
            CheckRespawnOutcomeLikeCpp::Blocked
        },
    );

    assert_eq!(
        actions,
        vec![ProcessRespawnActionLikeCpp::DeleteRespawn {
            object_type: SpawnObjectType::Creature,
            spawn_id: 3,
        }]
    );
    assert_eq!(
        store.get_respawn_time_like_cpp(SpawnObjectType::Creature, 3),
        0
    );
}

#[test]
fn process_respawns_check_false_future_reschedules_and_saves_like_cpp() {
    let mut store = RespawnStoreLikeCpp::new();
    store.add_respawn_info_like_cpp(respawn_info(SpawnObjectType::Creature, 4, 100));

    let actions = store.process_due_respawns_like_cpp(
        100,
        |_, _| None,
        |info| {
            info.respawn_time = 150;
            CheckRespawnOutcomeLikeCpp::Blocked
        },
    );

    assert_eq!(
        actions,
        vec![ProcessRespawnActionLikeCpp::RescheduleAndSave {
            info: respawn_info(SpawnObjectType::Creature, 4, 150),
        }]
    );
    assert_eq!(
        store.get_respawn_time_like_cpp(SpawnObjectType::Creature, 4),
        150
    );
}

#[test]
fn process_respawns_invalid_non_future_reschedule_reports_and_does_not_loop() {
    let mut store = RespawnStoreLikeCpp::new();
    store.add_respawn_info_like_cpp(respawn_info(SpawnObjectType::Creature, 5, 100));
    store.add_respawn_info_like_cpp(respawn_info(SpawnObjectType::Creature, 6, 100));

    let actions = store.process_due_respawns_like_cpp(
        100,
        |_, _| None,
        |info| {
            info.respawn_time = 99;
            CheckRespawnOutcomeLikeCpp::Blocked
        },
    );

    assert_eq!(
        actions,
        vec![ProcessRespawnActionLikeCpp::InvalidRescheduleNotFuture {
            info: respawn_info(SpawnObjectType::Creature, 6, 99),
        }]
    );
    assert_eq!(
        store.get_respawn_time_like_cpp(SpawnObjectType::Creature, 6),
        99
    );
    assert_eq!(store.respawn_timer_keys_like_cpp().count(), 2);
}

#[test]
fn spawn_constants_match_spawn_data_h() {
    assert_eq!(SpawnObjectType::Creature as u8, 0);
    assert_eq!(SpawnObjectType::GameObject as u8, 1);
    assert_eq!(SpawnObjectType::AreaTrigger as u8, 2);
    assert_eq!(SpawnObjectType::Creature.mask(), 0x1);
    assert_eq!(SpawnObjectType::GameObject.mask(), 0x2);
    assert_eq!(SpawnObjectType::AreaTrigger.mask(), 0x4);
    assert_eq!(SpawnGroupFlags::ALL.0, 0x3f);
    assert!(
        SpawnGroupFlags(0xff)
            .truncate_to_all()
            .contains(SpawnGroupFlags::SYSTEM)
    );
}

#[test]
fn object_spawn_store_indexes_creatures_by_map_difficulty_and_cell() {
    let mut store = SpawnStore::new();
    let data = spawn(SpawnObjectType::Creature, 100, 0.0, 0.0);
    let cell_id = data.cell_id();

    store.add_object_spawn(&data, |_| false);

    assert!(
        store
            .cell_object_guids(571, 0, cell_id)
            .unwrap()
            .creatures
            .contains(&100)
    );
    assert!(
        store
            .cell_object_guids(571, 1, cell_id)
            .unwrap()
            .creatures
            .contains(&100)
    );
    assert!(store.cell_object_guids(571, 2, cell_id).is_none());
}

#[test]
fn insert_spawn_metadata_like_cpp_does_not_touch_grid_indexes() {
    let mut store = SpawnStore::new();
    let data = spawn(SpawnObjectType::Creature, 150, 0.0, 0.0);
    let cell_id = data.cell_id();

    store.insert_spawn_metadata_like_cpp(&data);

    assert_eq!(
        store
            .spawn_data(SpawnObjectType::Creature, 150)
            .map(|spawn| spawn.spawn_id),
        Some(150)
    );
    assert!(store.cell_object_guids(571, 0, cell_id).is_none());
    assert!(store.cell_object_guids(571, 1, cell_id).is_none());
}

#[test]
fn object_spawn_store_uses_personal_phase_index_for_creatures_and_gameobjects() {
    let mut store = SpawnStore::new();
    let mut data = spawn(SpawnObjectType::GameObject, 200, 0.0, 0.0);
    data.phase_id = 9001;
    let cell_id = data.cell_id();

    store.add_object_spawn(&data, |phase_id| phase_id == 9001);

    assert!(store.cell_object_guids(571, 0, cell_id).is_none());
    assert!(store.has_personal_spawns(571, 0, 9001));
    assert!(
        store
            .cell_personal_object_guids(571, 0, 9001, cell_id)
            .unwrap()
            .gameobjects
            .contains(&200)
    );
}

#[test]
fn area_trigger_store_follows_cpp_non_personal_location_index() {
    let mut store = SpawnStore::new();
    let mut data = spawn(SpawnObjectType::AreaTrigger, 300, 0.0, 0.0);
    data.phase_id = 9001;
    let cell_id = data.cell_id();

    store.add_object_spawn(&data, |phase_id| phase_id == 9001);

    assert!(
        store
            .cell_object_guids(571, 0, cell_id)
            .unwrap()
            .area_triggers
            .contains(&300)
    );
    assert!(
        store
            .cell_personal_object_guids(571, 0, 9001, cell_id)
            .is_none()
    );
}

#[test]
fn removing_spawn_cleans_empty_cell_entry() {
    let mut store = SpawnStore::new();
    let data = spawn(SpawnObjectType::Creature, 100, 0.0, 0.0);
    let cell_id = data.cell_id();

    store.add_object_spawn(&data, |_| false);
    assert!(store.cell_object_guids(571, 0, cell_id).is_some());

    store.remove_object_spawn(&data, |_| false);
    assert!(store.cell_object_guids(571, 0, cell_id).is_none());
    assert!(store.cell_object_guids(571, 1, cell_id).is_none());
}

#[test]
fn spawn_group_defaults_match_cpp_system_unassigned_shape() {
    let default = SpawnGroupTemplateData::default_group();

    assert_eq!(SPAWNGROUP_MAP_UNSET, 0xffff_ffff);
    assert_eq!(default.group_id, 0);
    assert_eq!(default.name, "Default Group");
    assert_eq!(default.map_id, 0);
    assert!(default.is_system());

    let legacy = SpawnGroupTemplateData::legacy_group();
    assert_eq!(legacy.group_id, 1);
    assert_eq!(legacy.map_id, 0);
    assert!(legacy.is_system());
    assert!(legacy.flags.contains(SpawnGroupFlags::COMPATIBILITY_MODE));
}

#[test]
fn spawn_group_system_active_even_when_set_attempt_is_noop() {
    let system = template(10, 571, SpawnGroupFlags::SYSTEM);
    let mut state = SpawnGroupRuntimeState::new();

    assert!(state.is_spawn_group_active_like_cpp(Some(&system)));
    assert_eq!(
        state.set_spawn_group_active_like_cpp(Some(&system), false),
        SpawnGroupActiveChange::SystemGroup
    );
    assert!(!state.is_toggled(10));
    assert!(state.is_spawn_group_active_like_cpp(Some(&system)));
}

#[test]
fn spawn_group_non_manual_defaults_active_and_toggle_matches_cpp() {
    let group = template(10, 571, SpawnGroupFlags::NONE);
    let mut state = SpawnGroupRuntimeState::new();

    assert!(state.is_spawn_group_active_like_cpp(Some(&group)));
    assert_eq!(
        state.set_spawn_group_active_like_cpp(Some(&group), false),
        SpawnGroupActiveChange::Toggled
    );
    assert!(state.is_toggled(10));
    assert!(!state.is_spawn_group_active_like_cpp(Some(&group)));

    assert_eq!(
        state.set_spawn_group_active_like_cpp(Some(&group), true),
        SpawnGroupActiveChange::ClearedToggle
    );
    assert!(!state.is_toggled(10));
    assert!(state.is_spawn_group_active_like_cpp(Some(&group)));
}

#[test]
fn spawn_group_manual_defaults_inactive_and_toggle_matches_cpp() {
    let group = template(10, 571, SpawnGroupFlags::MANUAL_SPAWN);
    let mut state = SpawnGroupRuntimeState::new();

    assert!(!state.is_spawn_group_active_like_cpp(Some(&group)));
    assert_eq!(
        state.set_spawn_group_active_like_cpp(Some(&group), true),
        SpawnGroupActiveChange::Toggled
    );
    assert!(state.is_toggled(10));
    assert!(state.is_spawn_group_active_like_cpp(Some(&group)));

    assert_eq!(
        state.set_spawn_group_active_like_cpp(Some(&group), false),
        SpawnGroupActiveChange::ClearedToggle
    );
    assert!(!state.is_toggled(10));
    assert!(!state.is_spawn_group_active_like_cpp(Some(&group)));
}

#[test]
fn spawn_group_missing_query_is_false_and_set_reports_missing() {
    let mut state = SpawnGroupRuntimeState::new();

    assert!(!state.is_spawn_group_active_like_cpp(None));
    assert_eq!(
        state.set_spawn_group_active_like_cpp(None, true),
        SpawnGroupActiveChange::MissingGroup
    );
}

#[test]
fn should_spawn_on_grid_load_honors_respawn_before_group_and_pool() {
    let mut store = SpawnStore::new();
    let mut data = spawn(SpawnObjectType::Creature, 100, 0.0, 0.0);
    data.spawn_group = template(10, 571, SpawnGroupFlags::MANUAL_SPAWN);
    data.pool_id = 55;
    store.add_object_spawn(&data, |_| false);
    let state = SpawnGroupRuntimeState::new();
    let filter = SpawnGridLoadStateLikeCpp::new(&store, &state)
        .with_respawn_timers([(SpawnObjectType::Creature, 100)]);

    assert!(!filter.should_be_spawned_on_grid_load(SpawnObjectType::Creature, 100));
}

#[test]
fn should_spawn_on_grid_load_rejects_non_system_inactive_group() {
    let mut store = SpawnStore::new();
    let mut data = spawn(SpawnObjectType::Creature, 100, 0.0, 0.0);
    data.spawn_group = template(10, 571, SpawnGroupFlags::MANUAL_SPAWN);
    store.add_object_spawn(&data, |_| false);
    let state = SpawnGroupRuntimeState::new();
    let filter = SpawnGridLoadStateLikeCpp::new(&store, &state);

    assert!(!filter.should_be_spawned_on_grid_load(SpawnObjectType::Creature, 100));
}

#[test]
fn should_spawn_on_grid_load_pool_zero_ignores_pool_selection() {
    let mut store = SpawnStore::new();
    let mut data = spawn(SpawnObjectType::Creature, 100, 0.0, 0.0);
    data.spawn_group = template(10, 571, SpawnGroupFlags::NONE);
    data.pool_id = 0;
    store.add_object_spawn(&data, |_| false);
    let state = SpawnGroupRuntimeState::new();
    let filter = SpawnGridLoadStateLikeCpp::new(&store, &state);

    assert!(filter.should_be_spawned_on_grid_load(SpawnObjectType::Creature, 100));
}

#[test]
fn should_spawn_on_grid_load_pooled_spawn_requires_selected_object() {
    let mut store = SpawnStore::new();
    let mut data = spawn(SpawnObjectType::Creature, 100, 0.0, 0.0);
    data.spawn_group = template(10, 571, SpawnGroupFlags::NONE);
    data.pool_id = 55;
    store.add_object_spawn(&data, |_| false);
    let state = SpawnGroupRuntimeState::new();
    let mut filter = SpawnGridLoadStateLikeCpp::new(&store, &state);

    assert!(!filter.should_be_spawned_on_grid_load(SpawnObjectType::Creature, 100));
    filter.add_pool_spawned_object(SpawnObjectType::Creature, 100);
    assert!(filter.should_be_spawned_on_grid_load(SpawnObjectType::Creature, 100));
}

#[test]
fn should_spawn_on_grid_load_allows_system_no_respawn_no_pool() {
    let mut store = SpawnStore::new();
    let mut data = spawn(SpawnObjectType::Creature, 100, 0.0, 0.0);
    data.spawn_group = template(10, 571, SpawnGroupFlags::SYSTEM);
    store.add_object_spawn(&data, |_| false);
    let state = SpawnGroupRuntimeState::new();
    let filter = SpawnGridLoadStateLikeCpp::new(&store, &state);

    assert!(filter.should_be_spawned_on_grid_load(SpawnObjectType::Creature, 100));
}

#[test]
fn should_spawn_on_grid_load_missing_metadata_is_false_without_panic() {
    let store = SpawnStore::new();
    let state = SpawnGroupRuntimeState::new();
    let filter = SpawnGridLoadStateLikeCpp::new(&store, &state);

    assert!(!filter.should_be_spawned_on_grid_load(SpawnObjectType::Creature, 999));
}

#[test]
fn apply_spawn_groups_assigns_all_spawn_types_and_preserves_flags() {
    let mut store = SpawnStore::new();
    for (object_type, spawn_id) in [
        (SpawnObjectType::Creature, 100),
        (SpawnObjectType::GameObject, 200),
        (SpawnObjectType::AreaTrigger, 300),
    ] {
        store.add_object_spawn(&spawn(object_type, spawn_id, 0.0, 0.0), |_| false);
    }
    let mut templates = BTreeMap::from([
        (10, template(10, 571, SpawnGroupFlags::MANUAL_SPAWN)),
        (11, template(11, 571, SpawnGroupFlags::DYNAMIC_SPAWN_RATE)),
        (
            12,
            template(12, 571, SpawnGroupFlags::DESPAWN_ON_CONDITION_FAILURE),
        ),
    ]);

    let report = store.apply_spawn_groups_like_cpp(
        &mut templates,
        [
            SpawnGroupMemberRow {
                group_id: 10,
                spawn_type: 0,
                spawn_id: 100,
            },
            SpawnGroupMemberRow {
                group_id: 11,
                spawn_type: 1,
                spawn_id: 200,
            },
            SpawnGroupMemberRow {
                group_id: 12,
                spawn_type: 2,
                spawn_id: 300,
            },
        ],
    );

    assert_eq!(report.assigned, 3);
    assert_eq!(
        store
            .spawn_data(SpawnObjectType::Creature, 100)
            .unwrap()
            .spawn_group
            .flags,
        SpawnGroupFlags::MANUAL_SPAWN
    );
    assert_eq!(
        store
            .spawn_data(SpawnObjectType::GameObject, 200)
            .unwrap()
            .spawn_group
            .flags,
        SpawnGroupFlags::DYNAMIC_SPAWN_RATE
    );
    assert_eq!(
        store
            .spawn_data(SpawnObjectType::AreaTrigger, 300)
            .unwrap()
            .spawn_group
            .flags,
        SpawnGroupFlags::DESPAWN_ON_CONDITION_FAILURE
    );
    assert!(
        store
            .spawn_group_members(10)
            .unwrap()
            .contains(&SpawnGroupMember {
                object_type: SpawnObjectType::Creature,
                spawn_id: 100
            })
    );
    assert!(
        store
            .spawn_group_members(11)
            .unwrap()
            .contains(&SpawnGroupMember {
                object_type: SpawnObjectType::GameObject,
                spawn_id: 200
            })
    );
    assert!(
        store
            .spawn_group_members(12)
            .unwrap()
            .contains(&SpawnGroupMember {
                object_type: SpawnObjectType::AreaTrigger,
                spawn_id: 300
            })
    );
}

#[test]
fn first_assignment_sets_unset_group_map_and_indexes_group_by_map() {
    let mut store = SpawnStore::new();
    store.add_object_spawn(&spawn(SpawnObjectType::Creature, 100, 0.0, 0.0), |_| false);
    let mut templates = BTreeMap::from([(
        10,
        template(10, SPAWNGROUP_MAP_UNSET, SpawnGroupFlags::NONE),
    )]);

    let report = store.apply_spawn_groups_like_cpp(
        &mut templates,
        [SpawnGroupMemberRow {
            group_id: 10,
            spawn_type: 0,
            spawn_id: 100,
        }],
    );

    assert_eq!(report.assigned, 1);
    assert_eq!(templates.get(&10).unwrap().map_id, 571);
    assert!(store.spawn_group_ids_by_map(571).unwrap().contains(&10));
    assert_eq!(
        store
            .spawn_data(SpawnObjectType::Creature, 100)
            .unwrap()
            .spawn_group
            .map_id,
        571
    );
}

#[test]
fn system_group_allows_cross_map_without_member_index_but_non_system_skips() {
    let mut store = SpawnStore::new();
    let mut cross_map = spawn(SpawnObjectType::Creature, 100, 0.0, 0.0);
    cross_map.map_id = 571;
    store.add_object_spawn(&cross_map, |_| false);
    let mut blocked = spawn(SpawnObjectType::GameObject, 200, 0.0, 0.0);
    blocked.map_id = 571;
    store.add_object_spawn(&blocked, |_| false);
    let mut templates = BTreeMap::from([
        (10, template(10, 1, SpawnGroupFlags::SYSTEM)),
        (11, template(11, 1, SpawnGroupFlags::NONE)),
    ]);

    let report = store.apply_spawn_groups_like_cpp(
        &mut templates,
        [
            SpawnGroupMemberRow {
                group_id: 10,
                spawn_type: 0,
                spawn_id: 100,
            },
            SpawnGroupMemberRow {
                group_id: 11,
                spawn_type: 1,
                spawn_id: 200,
            },
        ],
    );

    assert_eq!(report.assigned, 1);
    assert_eq!(report.map_mismatch, 1);
    assert_eq!(
        store
            .spawn_data(SpawnObjectType::Creature, 100)
            .unwrap()
            .spawn_group
            .group_id,
        10
    );
    assert_eq!(
        store
            .spawn_data(SpawnObjectType::GameObject, 200)
            .unwrap()
            .spawn_group
            .group_id,
        0
    );
    assert!(store.spawn_group_members(10).is_none());
    assert!(store.spawn_group_members(11).is_none());
}

#[test]
fn duplicate_nonzero_group_is_reported_and_skipped() {
    let mut store = SpawnStore::new();
    let mut data = spawn(SpawnObjectType::Creature, 100, 0.0, 0.0);
    data.spawn_group = template(20, 571, SpawnGroupFlags::NONE);
    store.add_object_spawn(&data, |_| false);
    let mut templates = BTreeMap::from([(10, template(10, 571, SpawnGroupFlags::NONE))]);

    let report = store.apply_spawn_groups_like_cpp(
        &mut templates,
        [SpawnGroupMemberRow {
            group_id: 10,
            spawn_type: 0,
            spawn_id: 100,
        }],
    );

    assert_eq!(report.assigned, 0);
    assert_eq!(report.duplicate_spawn_group, 1);
    assert_eq!(report.issues[0].existing_group_id, Some(20));
    assert_eq!(
        store
            .spawn_data(SpawnObjectType::Creature, 100)
            .unwrap()
            .spawn_group
            .group_id,
        20
    );
}

#[test]
fn invalid_type_missing_spawn_and_missing_group_report_without_mutation() {
    let mut store = SpawnStore::new();
    store.add_object_spawn(&spawn(SpawnObjectType::Creature, 100, 0.0, 0.0), |_| false);
    let mut templates = BTreeMap::from([(10, template(10, 571, SpawnGroupFlags::NONE))]);

    let report = store.apply_spawn_groups_like_cpp(
        &mut templates,
        [
            SpawnGroupMemberRow {
                group_id: 10,
                spawn_type: 99,
                spawn_id: 100,
            },
            SpawnGroupMemberRow {
                group_id: 10,
                spawn_type: 1,
                spawn_id: 999,
            },
            SpawnGroupMemberRow {
                group_id: 99,
                spawn_type: 0,
                spawn_id: 100,
            },
        ],
    );

    assert_eq!(report.assigned, 0);
    assert_eq!(report.invalid_type, 1);
    assert_eq!(report.missing_spawn, 1);
    assert_eq!(report.missing_group, 1);
    assert_eq!(
        store
            .spawn_data(SpawnObjectType::Creature, 100)
            .unwrap()
            .spawn_group
            .group_id,
        0
    );
    assert!(store.spawn_group_members(10).is_none());
}

#[test]
fn default_group_zero_does_not_trigger_duplicate_and_behaves_unassigned() {
    let mut store = SpawnStore::new();
    let mut data = spawn(SpawnObjectType::Creature, 100, 0.0, 0.0);
    data.spawn_group = SpawnGroupTemplateData::default_group();
    store.add_object_spawn(&data, |_| false);
    let mut templates = BTreeMap::from([(10, template(10, 571, SpawnGroupFlags::NONE))]);

    let report = store.apply_spawn_groups_like_cpp(
        &mut templates,
        [SpawnGroupMemberRow {
            group_id: 10,
            spawn_type: 0,
            spawn_id: 100,
        }],
    );

    assert_eq!(report.assigned, 1);
    assert_eq!(report.duplicate_spawn_group, 0);
    assert_eq!(
        store
            .spawn_data(SpawnObjectType::Creature, 100)
            .unwrap()
            .spawn_group
            .group_id,
        10
    );
}
