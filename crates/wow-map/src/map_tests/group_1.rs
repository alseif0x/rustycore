//! Group scenarios for [`super`].
//!
//! Split out of map_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

#[test]
fn map_owned_respawn_grid_load_state_uses_map_timer_and_group_sources_like_cpp() {
    let mut map = test_map();
    let mut store = SpawnStore::new();
    let manual = spawn_group(12, SpawnGroupFlags::MANUAL_SPAWN);
    let spawn = spawn_data(SpawnObjectType::Creature, 42, manual.clone());
    store.add_object_spawn(&spawn, |_| false);

    assert!(
        !map.spawn_grid_load_state_like_cpp(&store)
            .should_be_spawned_on_grid_load(SpawnObjectType::Creature, 42)
    );

    map.set_spawn_group_active_like_cpp(Some(&manual), true);
    assert!(
        map.spawn_grid_load_state_like_cpp(&store)
            .should_be_spawned_on_grid_load(SpawnObjectType::Creature, 42)
    );

    map.add_respawn_info_like_cpp(respawn_info(SpawnObjectType::Creature, 42, 100));
    assert!(
        !map.spawn_grid_load_state_like_cpp(&store)
            .should_be_spawned_on_grid_load(SpawnObjectType::Creature, 42)
    );

    map.remove_respawn_time_like_cpp(SpawnObjectType::Creature, 42);
    assert!(
        map.spawn_grid_load_state_like_cpp(&store)
            .should_be_spawned_on_grid_load(SpawnObjectType::Creature, 42)
    );
}
#[test]
fn process_respawns_delete_only_inactive_spawn_group_removes_map_owned_timer_like_cpp() {
    let mut map = test_map();
    let mut store = SpawnStore::new();
    let manual = spawn_group(12, SpawnGroupFlags::MANUAL_SPAWN);
    store.add_object_spawn(&spawn_data(SpawnObjectType::Creature, 42, manual), |_| {
        false
    });
    map.add_respawn_info_like_cpp(respawn_info(SpawnObjectType::Creature, 42, 100));

    let summary = map.process_due_respawns_spawn_group_delete_only_like_cpp(100, &store);

    assert_eq!(summary.deleted_inactive_spawn_group, 1);
    assert_eq!(summary.blocked_missing_spawn_data, 0);
    assert_eq!(summary.blocked_pool_runtime, 0);
    assert_eq!(summary.blocked_do_respawn_runtime, 0);
    assert_eq!(
        map.get_respawn_time_like_cpp(SpawnObjectType::Creature, 42),
        0
    );
}
#[test]
fn check_respawn_spawn_group_guard_inactive_manual_group_clears_timer_like_cpp() {
    let map = test_map();
    let mut store = SpawnStore::new();
    let manual = spawn_group(12, SpawnGroupFlags::MANUAL_SPAWN);
    store.add_object_spawn(&spawn_data(SpawnObjectType::Creature, 42, manual), |_| {
        false
    });
    let mut info = respawn_info(SpawnObjectType::Creature, 42, 100);

    let outcome = map.check_respawn_spawn_group_guard_like_cpp(&mut info, &store);

    assert_eq!(
        outcome,
        CheckRespawnSpawnGroupGuardOutcomeLikeCpp::InactiveSpawnGroupDeletedTimer
    );
    assert_eq!(info.respawn_time, 0);
}
#[test]
fn check_respawn_spawn_group_guard_active_manual_group_preserves_timer_like_cpp() {
    let mut map = test_map();
    let mut store = SpawnStore::new();
    let manual = spawn_group(12, SpawnGroupFlags::MANUAL_SPAWN);
    store.add_object_spawn(
        &spawn_data(SpawnObjectType::Creature, 42, manual.clone()),
        |_| false,
    );
    map.set_spawn_group_active_like_cpp(Some(&manual), true);
    let mut info = respawn_info(SpawnObjectType::Creature, 42, 100);

    let outcome = map.check_respawn_spawn_group_guard_like_cpp(&mut info, &store);

    assert_eq!(outcome, CheckRespawnSpawnGroupGuardOutcomeLikeCpp::Allowed);
    assert_eq!(info.respawn_time, 100);
}
#[test]
fn check_respawn_spawn_group_guard_system_group_preserves_timer_like_cpp() {
    let map = test_map();
    let mut store = SpawnStore::new();
    let system = spawn_group(1, SpawnGroupFlags::SYSTEM);
    store.add_object_spawn(&spawn_data(SpawnObjectType::GameObject, 43, system), |_| {
        false
    });
    let mut info = respawn_info(SpawnObjectType::GameObject, 43, 100);

    let outcome = map.check_respawn_spawn_group_guard_like_cpp(&mut info, &store);

    assert_eq!(outcome, CheckRespawnSpawnGroupGuardOutcomeLikeCpp::Allowed);
    assert_eq!(info.respawn_time, 100);
}
#[test]
fn check_respawn_spawn_group_guard_missing_metadata_preserves_timer_like_cpp() {
    let map = test_map();
    let store = SpawnStore::new();
    let mut info = respawn_info(SpawnObjectType::Creature, 44, 100);

    let outcome = map.check_respawn_spawn_group_guard_like_cpp(&mut info, &store);

    assert_eq!(
        outcome,
        CheckRespawnSpawnGroupGuardOutcomeLikeCpp::MissingSpawnData
    );
    assert_eq!(info.respawn_time, 100);
}
#[test]
fn map_spawn_group_initial_state_system_active_and_not_toggleable() {
    let mut map = test_map();
    let system = spawn_group(1, SpawnGroupFlags::SYSTEM);

    assert!(map.spawn_group_state().toggled_spawn_group_ids().is_empty());
    assert!(map.is_spawn_group_active_like_cpp(Some(&system)));
    assert_eq!(
        map.set_spawn_group_active_like_cpp(Some(&system), false),
        SpawnGroupActiveChange::SystemGroup
    );
    assert!(map.spawn_group_state().toggled_spawn_group_ids().is_empty());
    assert!(map.is_spawn_group_active_like_cpp(Some(&system)));
}
#[test]
fn map_spawn_group_manual_default_inactive_activate_toggles_deactivate_clears() {
    let mut map = test_map();
    let manual = spawn_group(10, SpawnGroupFlags::MANUAL_SPAWN);

    assert!(!map.is_spawn_group_active_like_cpp(Some(&manual)));
    assert_eq!(
        map.set_spawn_group_active_like_cpp(Some(&manual), true),
        SpawnGroupActiveChange::Toggled
    );
    assert!(map.spawn_group_state().is_toggled(manual.group_id));
    assert!(map.is_spawn_group_active_like_cpp(Some(&manual)));

    assert_eq!(
        map.set_spawn_group_inactive_like_cpp(Some(&manual)),
        SpawnGroupActiveChange::ClearedToggle
    );
    assert!(!map.spawn_group_state().is_toggled(manual.group_id));
    assert!(!map.is_spawn_group_active_like_cpp(Some(&manual)));
}
#[test]
fn map_spawn_group_non_manual_default_active_deactivate_toggles_activate_clears() {
    let mut map = test_map();
    let automatic = spawn_group(11, SpawnGroupFlags::NONE);

    assert!(map.is_spawn_group_active_like_cpp(Some(&automatic)));
    assert_eq!(
        map.set_spawn_group_inactive_like_cpp(Some(&automatic)),
        SpawnGroupActiveChange::Toggled
    );
    assert!(map.spawn_group_state().is_toggled(automatic.group_id));
    assert!(!map.is_spawn_group_active_like_cpp(Some(&automatic)));

    assert_eq!(
        map.set_spawn_group_active_like_cpp(Some(&automatic), true),
        SpawnGroupActiveChange::ClearedToggle
    );
    assert!(!map.spawn_group_state().is_toggled(automatic.group_id));
    assert!(map.is_spawn_group_active_like_cpp(Some(&automatic)));
}
#[test]
fn map_spawn_group_missing_group_returns_false_and_does_not_mutate_toggles() {
    let mut map = test_map();

    assert!(!map.is_spawn_group_active_like_cpp(None));
    assert_eq!(
        map.set_spawn_group_active_like_cpp(None, true),
        SpawnGroupActiveChange::MissingGroup
    );
    assert_eq!(
        map.set_spawn_group_inactive_like_cpp(None),
        SpawnGroupActiveChange::MissingGroup
    );
    assert!(map.spawn_group_state().toggled_spawn_group_ids().is_empty());
}
#[test]
fn map_spawn_group_grid_load_bridge_uses_map_owned_toggle_state() {
    let mut map = test_map();
    let mut store = SpawnStore::new();
    let manual = spawn_group(12, SpawnGroupFlags::MANUAL_SPAWN);
    let spawn = crate::spawn::SpawnData {
        object_type: SpawnObjectType::Creature,
        spawn_id: 42,
        map_id: 571,
        db_data: true,
        spawn_group: manual.clone(),
        id: 99,
        spawn_point: crate::spawn::SpawnPosition::new(0.0, 0.0, 0.0, 0.0),
        phase_use_flags: 0,
        phase_id: 0,
        phase_group: 0,
        terrain_swap_map: 0,
        pool_id: 0,
        spawn_time_secs: 0,
        spawn_difficulties: vec![1],
        script_id: 0,
        string_id: String::new(),
    };
    store.add_object_spawn(&spawn, |_| false);

    assert!(
        !map.spawn_grid_load_state_like_cpp(&store)
            .should_be_spawned_on_grid_load(SpawnObjectType::Creature, 42)
    );

    map.set_spawn_group_active_like_cpp(Some(&manual), true);
    assert!(
        map.spawn_grid_load_state_like_cpp(&store)
            .should_be_spawned_on_grid_load(SpawnObjectType::Creature, 42)
    );
}
#[test]
fn map_spawn_group_init_bridge_skips_system_and_applies_condition_semantics() {
    let mut map = test_map();
    let system = spawn_group(1, SpawnGroupFlags::SYSTEM);
    let manual = spawn_group(20, SpawnGroupFlags::MANUAL_SPAWN);
    let automatic = spawn_group(21, SpawnGroupFlags::NONE);
    let groups = [&system, &manual, &automatic];

    let changes = map.init_spawn_group_state_like_cpp(groups, |group| group.group_id == 20);

    assert_eq!(
        changes,
        vec![
            (20, SpawnGroupActiveChange::Toggled),
            (21, SpawnGroupActiveChange::Toggled)
        ]
    );
    assert!(!map.spawn_group_state().is_toggled(system.group_id));
    assert!(map.spawn_group_state().is_toggled(manual.group_id));
    assert!(map.spawn_group_state().is_toggled(automatic.group_id));
    assert!(map.is_spawn_group_active_like_cpp(Some(&manual)));
    assert!(!map.is_spawn_group_active_like_cpp(Some(&automatic)));
}
#[test]
fn update_spawn_group_conditions_manual_active_condition_false_with_despawn_flag_plans_despawn() {
    let mut map = test_map();
    let manual = spawn_group(
        30,
        spawn_group_flags(
            SpawnGroupFlags::MANUAL_SPAWN,
            SpawnGroupFlags::DESPAWN_ON_CONDITION_FAILURE,
        ),
    );
    map.set_spawn_group_active_like_cpp(Some(&manual), true);

    let actions = map.plan_update_spawn_group_conditions_like_cpp([&manual], |_| false);

    assert_eq!(
        actions,
        vec![(
            30,
            SpawnGroupConditionActionLikeCpp::Despawn {
                delete_respawn_times: true
            }
        )]
    );
}
#[test]
fn update_spawn_group_conditions_manual_active_condition_true_is_noop() {
    let mut map = test_map();
    let manual = spawn_group(
        31,
        spawn_group_flags(
            SpawnGroupFlags::MANUAL_SPAWN,
            SpawnGroupFlags::DESPAWN_ON_CONDITION_FAILURE,
        ),
    );
    map.set_spawn_group_active_like_cpp(Some(&manual), true);

    let actions = map.plan_update_spawn_group_conditions_like_cpp([&manual], |_| true);

    assert_eq!(actions, vec![(31, SpawnGroupConditionActionLikeCpp::Noop)]);
}
#[test]
fn update_spawn_group_conditions_automatic_inactive_condition_true_plans_spawn() {
    let mut map = test_map();
    let automatic = spawn_group(32, SpawnGroupFlags::NONE);
    map.set_spawn_group_inactive_like_cpp(Some(&automatic));

    let actions = map.plan_update_spawn_group_conditions_like_cpp([&automatic], |_| true);

    assert_eq!(
        actions,
        vec![(
            32,
            SpawnGroupConditionActionLikeCpp::Spawn {
                ignore_respawn: false,
                force: false
            }
        )]
    );
}
#[test]
fn update_spawn_group_conditions_automatic_active_condition_false_with_despawn_flag_plans_despawn()
{
    let map = test_map();
    let automatic = spawn_group(33, SpawnGroupFlags::DESPAWN_ON_CONDITION_FAILURE);

    let actions = map.plan_update_spawn_group_conditions_like_cpp([&automatic], |_| false);

    assert_eq!(
        actions,
        vec![(
            33,
            SpawnGroupConditionActionLikeCpp::Despawn {
                delete_respawn_times: true
            }
        )]
    );
}
#[test]
fn update_spawn_group_conditions_automatic_active_condition_false_without_despawn_flag_sets_inactive()
 {
    let map = test_map();
    let automatic = spawn_group(34, SpawnGroupFlags::NONE);

    let actions = map.plan_update_spawn_group_conditions_like_cpp([&automatic], |_| false);

    assert_eq!(
        actions,
        vec![(34, SpawnGroupConditionActionLikeCpp::SetInactive)]
    );
}
#[test]
fn update_spawn_group_conditions_automatic_active_condition_true_is_noop() {
    let map = test_map();
    let automatic = spawn_group(35, SpawnGroupFlags::NONE);

    let actions = map.plan_update_spawn_group_conditions_like_cpp([&automatic], |_| true);

    assert_eq!(actions, vec![(35, SpawnGroupConditionActionLikeCpp::Noop)]);
}
#[test]
fn update_spawn_group_conditions_planner_is_pure_and_preserves_spawn_group_state() {
    let mut map = test_map();
    let manual = spawn_group(
        36,
        spawn_group_flags(
            SpawnGroupFlags::MANUAL_SPAWN,
            SpawnGroupFlags::DESPAWN_ON_CONDITION_FAILURE,
        ),
    );
    let automatic = spawn_group(37, SpawnGroupFlags::NONE);
    map.set_spawn_group_active_like_cpp(Some(&manual), true);
    let before = map
        .spawn_group_state()
        .toggled_spawn_group_ids()
        .iter()
        .copied()
        .collect::<Vec<_>>();

    let actions = map.plan_update_spawn_group_conditions_like_cpp([&manual, &automatic], |_| false);
    let after = map
        .spawn_group_state()
        .toggled_spawn_group_ids()
        .iter()
        .copied()
        .collect::<Vec<_>>();

    assert_eq!(
        actions,
        vec![
            (
                36,
                SpawnGroupConditionActionLikeCpp::Despawn {
                    delete_respawn_times: true
                }
            ),
            (37, SpawnGroupConditionActionLikeCpp::SetInactive),
        ]
    );
    assert_eq!(after, before);
    assert!(map.is_spawn_group_active_like_cpp(Some(&manual)));
    assert!(map.is_spawn_group_active_like_cpp(Some(&automatic)));
}
#[test]
fn update_spawn_group_conditions_apply_automatic_condition_failure_without_despawn_sets_inactive() {
    let mut map = test_map();
    let automatic = spawn_group(38, SpawnGroupFlags::NONE);

    let outcomes =
        map.apply_update_spawn_group_conditions_set_inactive_like_cpp([&automatic], |_| false);

    assert_eq!(
        outcomes,
        vec![SpawnGroupConditionUpdateOutcomeLikeCpp {
            group_id: 38,
            action: SpawnGroupConditionActionLikeCpp::SetInactive,
            applied_change: Some(SpawnGroupActiveChange::Toggled),
            despawn_outcome: None,
            spawn_outcome: None,
        }]
    );
    assert!(!map.is_spawn_group_active_like_cpp(Some(&automatic)));
    assert!(map.spawn_group_state().is_toggled(automatic.group_id));
}
#[test]
fn update_spawn_group_conditions_apply_automatic_condition_failure_with_despawn_only_plans_despawn()
{
    let mut map = test_map();
    let automatic = spawn_group(39, SpawnGroupFlags::DESPAWN_ON_CONDITION_FAILURE);

    let outcomes =
        map.apply_update_spawn_group_conditions_set_inactive_like_cpp([&automatic], |_| false);

    assert_eq!(
        outcomes,
        vec![SpawnGroupConditionUpdateOutcomeLikeCpp {
            group_id: 39,
            action: SpawnGroupConditionActionLikeCpp::Despawn {
                delete_respawn_times: true
            },
            applied_change: None,
            despawn_outcome: None,
            spawn_outcome: None,
        }]
    );
    assert!(map.is_spawn_group_active_like_cpp(Some(&automatic)));
    assert!(map.spawn_group_state().toggled_spawn_group_ids().is_empty());
}
#[test]
fn update_spawn_group_conditions_apply_automatic_inactive_condition_true_only_plans_spawn() {
    let mut map = test_map();
    let automatic = spawn_group(40, SpawnGroupFlags::NONE);
    assert_eq!(
        map.set_spawn_group_inactive_like_cpp(Some(&automatic)),
        SpawnGroupActiveChange::Toggled
    );

    let outcomes =
        map.apply_update_spawn_group_conditions_set_inactive_like_cpp([&automatic], |_| true);

    assert_eq!(
        outcomes,
        vec![SpawnGroupConditionUpdateOutcomeLikeCpp {
            group_id: 40,
            action: SpawnGroupConditionActionLikeCpp::Spawn {
                ignore_respawn: false,
                force: false,
            },
            applied_change: None,
            despawn_outcome: None,
            spawn_outcome: None,
        }]
    );
    assert!(!map.is_spawn_group_active_like_cpp(Some(&automatic)));
    assert!(map.spawn_group_state().is_toggled(automatic.group_id));
}
#[test]
fn update_spawn_group_conditions_condition_failure_despawns_live_objects_and_timers_like_cpp() {
    let group = spawn_group(391, SpawnGroupFlags::DESPAWN_ON_CONDITION_FAILURE);
    let mut store = SpawnStore::new();
    let mut templates = BTreeMap::from([(group.group_id, group.clone())]);
    let creature_spawn = spawn_data(
        SpawnObjectType::Creature,
        10,
        SpawnGroupTemplateData::default_group(),
    );
    let gameobject_spawn = spawn_data(
        SpawnObjectType::GameObject,
        20,
        SpawnGroupTemplateData::default_group(),
    );
    store.add_object_spawn(&creature_spawn, |_| false);
    store.add_object_spawn(&gameobject_spawn, |_| false);
    store.apply_spawn_groups_like_cpp(
        &mut templates,
        [
            crate::spawn::SpawnGroupMemberRow {
                group_id: group.group_id,
                spawn_type: SpawnObjectType::Creature as u8,
                spawn_id: 10,
            },
            crate::spawn::SpawnGroupMemberRow {
                group_id: group.group_id,
                spawn_type: SpawnObjectType::GameObject as u8,
                spawn_id: 20,
            },
        ],
    );
    let group = templates.get(&391).expect("group resolved").clone();
    let mut map = test_map();
    map.insert_map_object_record(
        MapObjectRecord::new_creature(test_creature_for_spawn(10, 10, true)).unwrap(),
    )
    .unwrap();
    map.insert_map_object_record(
        MapObjectRecord::new_game_object(test_gameobject_for_spawn(20, 20)).unwrap(),
    )
    .unwrap();
    assert_eq!(map.creature_spawn_id_store_count_like_cpp(10), 1);
    assert_eq!(map.gameobject_spawn_id_store_count_like_cpp(20), 1);
    map.add_respawn_info_like_cpp(respawn_info(SpawnObjectType::Creature, 10, 100));
    map.add_respawn_info_like_cpp(respawn_info(SpawnObjectType::GameObject, 20, 100));

    let outcomes =
        map.apply_update_spawn_group_conditions_represented_like_cpp([&group], &store, |_| false);

    assert_eq!(outcomes.len(), 1);
    assert_eq!(
        outcomes[0].action,
        SpawnGroupConditionActionLikeCpp::condition_failure_despawn()
    );
    let despawn = outcomes[0].despawn_outcome.expect("despawn executed");
    assert_eq!(despawn.objects_removed, 2);
    assert_eq!(despawn.respawn_timers_removed, 2);
    assert_eq!(despawn.blocked_missing_group, 0);
    assert_eq!(despawn.blocked_system_group, 0);
    assert_eq!(despawn.unsupported_live_despawn_types, 0);
    assert_eq!(
        despawn.applied_inactive_change,
        Some(SpawnGroupActiveChange::Toggled)
    );
    map.remove_all_objects_in_remove_list_like_cpp();
    assert_eq!(map.map_object_count(), 0);
    assert_eq!(map.creature_spawn_id_store_count_like_cpp(10), 0);
    assert_eq!(map.gameobject_spawn_id_store_count_like_cpp(20), 0);
    assert_eq!(
        map.get_respawn_time_like_cpp(SpawnObjectType::Creature, 10),
        0
    );
    assert_eq!(
        map.get_respawn_time_like_cpp(SpawnObjectType::GameObject, 20),
        0
    );
    assert!(!map.is_spawn_group_active_like_cpp(Some(&group)));
}
#[test]
fn spawn_group_spawn_missing_or_system_group_blocks_without_activation_like_cpp() {
    let mut map = test_map();
    let store = SpawnStore::new();
    let system = spawn_group(3940, SpawnGroupFlags::SYSTEM);

    let missing = map.spawn_group_spawn_like_cpp(None, false, false, &store);
    let system_outcome = map.spawn_group_spawn_like_cpp(Some(&system), false, false, &store);

    assert_eq!(missing.blocked_missing_group, 1);
    assert_eq!(missing.applied_active_change, None);
    assert_eq!(system_outcome.blocked_system_group, 1);
    assert_eq!(system_outcome.applied_active_change, None);
    assert!(map.spawn_group_state().toggled_spawn_group_ids().is_empty());
}
#[test]
fn spawn_group_spawn_respawn_timer_skips_unless_ignore_or_force_removes_like_cpp() {
    let group = spawn_group(3942, SpawnGroupFlags::NONE);
    let (group, store) = spawn_group_store(
        group,
        vec![spawn_data(
            SpawnObjectType::Creature,
            102,
            SpawnGroupTemplateData::default_group(),
        )],
    );
    let mut blocked_map = test_map();
    blocked_map.load_grid(0.0, 0.0);
    blocked_map.add_respawn_info_like_cpp(respawn_info(SpawnObjectType::Creature, 102, 100));

    let blocked = blocked_map.spawn_group_spawn_like_cpp(Some(&group), false, false, &store);

    assert_eq!(blocked.skipped_respawn_timer_active, 1);
    assert_eq!(blocked.load_plans.len(), 0);
    assert_eq!(
        blocked_map.get_respawn_time_like_cpp(SpawnObjectType::Creature, 102),
        100
    );

    let mut ignore_map = test_map();
    ignore_map.load_grid(0.0, 0.0);
    ignore_map.add_respawn_info_like_cpp(respawn_info(SpawnObjectType::Creature, 102, 100));
    let ignored = ignore_map.spawn_group_spawn_like_cpp(Some(&group), true, false, &store);

    assert_eq!(ignored.respawn_timers_removed, 1);
    assert_eq!(ignored.skipped_respawn_timer_active, 0);
    assert_eq!(ignored.blocked_loaded_grid_creature_loads, 1);
    assert_eq!(
        ignore_map.get_respawn_time_like_cpp(SpawnObjectType::Creature, 102),
        0
    );

    let mut force_map = test_map();
    force_map.load_grid(0.0, 0.0);
    force_map.add_respawn_info_like_cpp(respawn_info(SpawnObjectType::Creature, 102, 100));
    let forced = force_map.spawn_group_spawn_like_cpp(Some(&group), false, true, &store);
    assert_eq!(forced.respawn_timers_removed, 1);
    assert_eq!(forced.load_plans[0].force, true);
}
#[test]
fn spawn_group_spawn_live_object_skip_is_bypassed_by_force_like_cpp() {
    let group = spawn_group(3943, SpawnGroupFlags::NONE);
    let (group, store) = spawn_group_store(
        group,
        vec![
            spawn_data(
                SpawnObjectType::Creature,
                103,
                SpawnGroupTemplateData::default_group(),
            ),
            spawn_data(
                SpawnObjectType::GameObject,
                203,
                SpawnGroupTemplateData::default_group(),
            ),
        ],
    );
    let mut map = test_map();
    map.load_grid(0.0, 0.0);
    map.insert_map_object_record(
        MapObjectRecord::new_creature(test_creature_for_spawn(103, 103, true)).unwrap(),
    )
    .unwrap();
    map.insert_map_object_record(
        MapObjectRecord::new_game_object(test_gameobject_for_spawn(203, 203)).unwrap(),
    )
    .unwrap();

    let skipped = map.spawn_group_spawn_like_cpp(Some(&group), false, false, &store);

    assert_eq!(skipped.skipped_live_object_active, 2);
    assert!(skipped.load_plans.is_empty());
    assert_eq!(map.map_object_count(), 2);

    let forced = map.spawn_group_spawn_like_cpp(Some(&group), false, true, &store);
    assert_eq!(forced.skipped_live_object_active, 0);
    assert_eq!(forced.load_plans.len(), 2);
    assert_eq!(forced.respawn_timers_missing, 2);
    assert_eq!(map.map_object_count(), 2);
}
#[test]
fn spawn_group_spawn_difficulty_mismatch_precedes_unloaded_grid_like_cpp() {
    let group = spawn_group(3944, SpawnGroupFlags::NONE);
    let mut spawn = spawn_data(
        SpawnObjectType::Creature,
        104,
        SpawnGroupTemplateData::default_group(),
    );
    spawn.spawn_difficulties = vec![2];
    let (group, store) = spawn_group_store(group, vec![spawn]);
    let mut map = test_map();

    let outcome = map.spawn_group_spawn_like_cpp(Some(&group), false, false, &store);

    assert_eq!(outcome.skipped_difficulty_mismatch, 1);
    assert_eq!(outcome.skipped_unloaded_grid, 0);
    assert!(outcome.load_plans.is_empty());
}
#[test]
fn spawn_group_spawn_unloaded_grid_skips_before_plan_like_cpp() {
    let group = spawn_group(3945, SpawnGroupFlags::NONE);
    let (group, store) = spawn_group_store(
        group,
        vec![spawn_data(
            SpawnObjectType::GameObject,
            205,
            SpawnGroupTemplateData::default_group(),
        )],
    );
    let mut map = test_map();

    let outcome = map.spawn_group_spawn_like_cpp(Some(&group), false, false, &store);

    assert_eq!(outcome.skipped_unloaded_grid, 1);
    assert_eq!(outcome.blocked_loaded_grid_gameobject_loads, 0);
    assert!(outcome.load_plans.is_empty());
}
#[test]
fn spawn_group_spawn_area_trigger_skips_no_respawn_map_before_loader_like_cpp() {
    let group = spawn_group(3946, SpawnGroupFlags::NONE);
    let (group, store) = spawn_group_store(
        group,
        vec![spawn_data(
            SpawnObjectType::AreaTrigger,
            305,
            SpawnGroupTemplateData::default_group(),
        )],
    );
    let mut map = test_map();
    map.load_grid(0.0, 0.0);
    let mut loader_calls = 0;

    let outcome = map.spawn_group_spawn_loaded_grid_records_like_cpp(
        Some(&group),
        false,
        false,
        &store,
        |_map, _object_type, _spawn_id, _force| {
            loader_calls += 1;
            None
        },
    );

    assert_eq!(outcome.metadata_entries, 1);
    assert_eq!(outcome.skipped_no_respawn_map, 1);
    assert_eq!(outcome.unsupported_spawn_types, 0);
    assert_eq!(outcome.blocked_loaded_grid_spawn_loads, 0);
    assert_eq!(loader_calls, 0);
    assert!(outcome.load_plans.is_empty());
    assert_eq!(map.map_object_count(), 0);
    assert!(map.is_spawn_group_active_like_cpp(Some(&group)));
}
#[test]
fn spawn_group_spawn_loader_none_blocks_and_continues_to_later_member_like_cpp() {
    let group = spawn_group(3948, SpawnGroupFlags::NONE);
    let (group, store) = spawn_group_store(
        group,
        vec![
            spawn_data(
                SpawnObjectType::Creature,
                108,
                SpawnGroupTemplateData::default_group(),
            ),
            spawn_data(
                SpawnObjectType::GameObject,
                208,
                SpawnGroupTemplateData::default_group(),
            ),
        ],
    );
    let mut map = test_map();
    map.load_grid(0.0, 0.0);
    let mut calls = Vec::new();

    let outcome = map.spawn_group_spawn_loaded_grid_records_like_cpp(
        Some(&group),
        false,
        false,
        &store,
        |_map, object_type, spawn_id, force| {
            calls.push((object_type, spawn_id, force));
            if object_type == SpawnObjectType::Creature {
                None
            } else {
                Some(LoadedGridRespawnRecordsLikeCpp::primary_only(
                    MapObjectRecord::new_game_object(test_gameobject_for_spawn(spawn_id, 208))
                        .unwrap(),
                ))
            }
        },
    );

    assert_eq!(calls.len(), 2);
    assert_eq!(outcome.load_plans.len(), 2);
    assert_eq!(outcome.blocked_loaded_grid_spawn_loads, 1);
    assert_eq!(outcome.blocked_loaded_grid_creature_loads, 1);
    assert_eq!(outcome.blocked_loaded_grid_gameobject_loads, 0);
    assert_eq!(outcome.executed_loaded_grid_spawns, 1);
    assert_eq!(map.map_object_count(), 1);
    assert_eq!(map.gameobject_spawn_id_store_count_like_cpp(208), 1);
}
#[test]
fn spawn_group_spawn_primary_add_to_map_failure_blocks_without_executed_like_cpp() {
    let group = spawn_group(3949, SpawnGroupFlags::NONE);
    let (group, store) = spawn_group_store(
        group,
        vec![spawn_data(
            SpawnObjectType::GameObject,
            209,
            SpawnGroupTemplateData::default_group(),
        )],
    );
    let mut map = test_map();
    map.load_grid(0.0, 0.0);

    let outcome = map.spawn_group_spawn_loaded_grid_records_like_cpp(
        Some(&group),
        false,
        false,
        &store,
        |_map, _object_type, spawn_id, _force| {
            let mut gameobject = GameObject::new();
            gameobject
                .world_mut()
                .object_mut()
                .create(guid(HighGuid::GameObject, 209));
            gameobject.world_mut().object_mut().set_entry(42);
            gameobject.world_mut().set_map(999, 7).unwrap();
            gameobject
                .world_mut()
                .relocate(Position::xyz(1.0, 2.0, 3.0));
            gameobject.set_spawn_id(spawn_id);
            Some(LoadedGridRespawnRecordsLikeCpp::primary_only(
                MapObjectRecord::new_game_object(gameobject).unwrap(),
            ))
        },
    );

    assert_eq!(outcome.load_plans.len(), 1);
    assert_eq!(outcome.executed_loaded_grid_spawns, 0);
    assert_eq!(outcome.blocked_loaded_grid_spawn_loads, 0);
    assert_eq!(outcome.blocked_loaded_grid_gameobject_loads, 0);
    assert_eq!(outcome.blocked_loaded_grid_spawn_add_to_map, 1);
    assert_eq!(map.map_object_count(), 0);
    assert_eq!(map.gameobject_spawn_id_store_count_like_cpp(209), 0);
}
#[test]
fn spawn_group_spawn_passes_force_to_loader_after_force_bypasses_live_skip_like_cpp() {
    let group = spawn_group(3950, SpawnGroupFlags::NONE);
    let (group, store) = spawn_group_store(
        group,
        vec![spawn_data(
            SpawnObjectType::Creature,
            110,
            SpawnGroupTemplateData::default_group(),
        )],
    );
    let mut map = test_map();
    map.load_grid(0.0, 0.0);
    map.insert_map_object_record(
        MapObjectRecord::new_creature(test_creature_for_spawn(110, 110, true)).unwrap(),
    )
    .unwrap();
    let mut captured_force = Vec::new();

    let outcome = map.spawn_group_spawn_loaded_grid_records_like_cpp(
        Some(&group),
        false,
        true,
        &store,
        |_map, _object_type, _spawn_id, force| {
            captured_force.push(force);
            None
        },
    );

    assert_eq!(captured_force, vec![true]);
    assert_eq!(outcome.skipped_live_object_active, 0);
    assert_eq!(outcome.load_plans.len(), 1);
    assert_eq!(outcome.blocked_loaded_grid_spawn_loads, 1);
    assert_eq!(outcome.blocked_loaded_grid_creature_loads, 1);
}
