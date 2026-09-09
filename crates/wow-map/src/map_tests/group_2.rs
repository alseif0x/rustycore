//! Group scenarios for [`super`].
//!
//! Split out of map_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

#[test]
fn update_spawn_group_conditions_spawn_branch_returns_spawn_outcome_and_activates_like_cpp() {
    let group = spawn_group(392, SpawnGroupFlags::NONE);
    let mut store = SpawnStore::new();
    let mut templates = BTreeMap::from([(group.group_id, group.clone())]);
    let creature_spawn = spawn_data(
        SpawnObjectType::Creature,
        30,
        SpawnGroupTemplateData::default_group(),
    );
    store.add_object_spawn(&creature_spawn, |_| false);
    store.apply_spawn_groups_like_cpp(
        &mut templates,
        [crate::spawn::SpawnGroupMemberRow {
            group_id: group.group_id,
            spawn_type: SpawnObjectType::Creature as u8,
            spawn_id: 30,
        }],
    );
    let group = templates.get(&392).expect("group resolved").clone();
    let mut map = test_map();
    map.set_spawn_group_inactive_like_cpp(Some(&group));
    map.load_grid(0.0, 0.0);

    let outcomes =
        map.apply_update_spawn_group_conditions_represented_like_cpp([&group], &store, |_| true);

    assert_eq!(outcomes.len(), 1);
    assert_eq!(
        outcomes[0].action,
        SpawnGroupConditionActionLikeCpp::spawn_group_spawn_default()
    );
    assert_eq!(outcomes[0].applied_change, None);
    assert_eq!(outcomes[0].despawn_outcome, None);
    let spawn = outcomes[0].spawn_outcome.as_ref().expect("spawn executed");
    assert_eq!(
        spawn.applied_active_change,
        Some(SpawnGroupActiveChange::ClearedToggle)
    );
    assert_eq!(spawn.blocked_loaded_grid_creature_loads, 1);
    assert_eq!(
        spawn.load_plans,
        vec![SpawnGroupSpawnLoadPlanLikeCpp {
            object_type: SpawnObjectType::Creature,
            spawn_id: 30,
            force: false,
        }]
    );
    assert_eq!(map.map_object_count(), 0);
    assert!(map.is_spawn_group_active_like_cpp(Some(&group)));
}
#[test]
fn update_spawn_group_conditions_spawn_group_spawn_loaded_grid_loader_some_inserts_primary_record_like_cpp()
 {
    let group = spawn_group(393, SpawnGroupFlags::NONE);
    let (group, store) = spawn_group_store(
        group,
        vec![spawn_data(
            SpawnObjectType::GameObject,
            31,
            SpawnGroupTemplateData::default_group(),
        )],
    );
    let mut map = test_map();
    map.set_spawn_group_inactive_like_cpp(Some(&group));
    map.load_grid(0.0, 0.0);
    let mut calls = Vec::new();

    let outcomes = map.apply_update_spawn_group_conditions_loaded_grid_records_like_cpp(
        [&group],
        &store,
        |_| true,
        |_map, object_type, spawn_id, force| {
            calls.push((object_type, spawn_id, force));
            Some(LoadedGridRespawnRecordsLikeCpp::primary_only(
                MapObjectRecord::new_game_object(test_gameobject_for_spawn(spawn_id, 31)).unwrap(),
            ))
        },
    );

    assert_eq!(outcomes.len(), 1);
    assert_eq!(
        outcomes[0].action,
        SpawnGroupConditionActionLikeCpp::spawn_group_spawn_default()
    );
    assert_eq!(outcomes[0].applied_change, None);
    assert_eq!(outcomes[0].despawn_outcome, None);
    let spawn = outcomes[0].spawn_outcome.as_ref().expect("spawn executed");
    assert_eq!(calls, vec![(SpawnObjectType::GameObject, 31, false)]);
    assert_eq!(
        spawn.applied_active_change,
        Some(SpawnGroupActiveChange::ClearedToggle)
    );
    assert_eq!(spawn.load_plans.len(), 1);
    assert_eq!(spawn.executed_loaded_grid_spawns, 1);
    assert_eq!(spawn.blocked_loaded_grid_spawn_loads, 0);
    assert_eq!(spawn.blocked_loaded_grid_spawn_add_to_map, 0);
    assert_eq!(map.map_object_count(), 1);
    assert_eq!(map.gameobject_spawn_id_store_count_like_cpp(31), 1);
    assert!(map.is_spawn_group_active_like_cpp(Some(&group)));
}
#[test]
fn update_spawn_group_conditions_apply_manual_condition_failure_never_sets_inactive() {
    let mut map = test_map();
    let manual_with_despawn = spawn_group(
        41,
        spawn_group_flags(
            SpawnGroupFlags::MANUAL_SPAWN,
            SpawnGroupFlags::DESPAWN_ON_CONDITION_FAILURE,
        ),
    );
    let manual_without_despawn = spawn_group(42, SpawnGroupFlags::MANUAL_SPAWN);
    map.set_spawn_group_active_like_cpp(Some(&manual_with_despawn), true);
    map.set_spawn_group_active_like_cpp(Some(&manual_without_despawn), true);

    let outcomes = map.apply_update_spawn_group_conditions_set_inactive_like_cpp(
        [&manual_with_despawn, &manual_without_despawn],
        |_| false,
    );

    assert_eq!(
        outcomes,
        vec![
            SpawnGroupConditionUpdateOutcomeLikeCpp {
                group_id: 41,
                action: SpawnGroupConditionActionLikeCpp::Despawn {
                    delete_respawn_times: true
                },
                applied_change: None,
                despawn_outcome: None,
                spawn_outcome: None,
            },
            SpawnGroupConditionUpdateOutcomeLikeCpp {
                group_id: 42,
                action: SpawnGroupConditionActionLikeCpp::Noop,
                applied_change: None,
                despawn_outcome: None,
                spawn_outcome: None,
            },
        ]
    );
    assert!(map.is_spawn_group_active_like_cpp(Some(&manual_with_despawn)));
    assert!(map.is_spawn_group_active_like_cpp(Some(&manual_without_despawn)));
    assert!(
        map.spawn_group_state()
            .is_toggled(manual_with_despawn.group_id)
    );
    assert!(
        map.spawn_group_state()
            .is_toggled(manual_without_despawn.group_id)
    );
}
#[test]
fn update_spawn_group_conditions_apply_active_equals_should_is_noop_without_change() {
    let mut map = test_map();
    let automatic = spawn_group(43, SpawnGroupFlags::NONE);
    let manual = spawn_group(44, SpawnGroupFlags::MANUAL_SPAWN);

    let outcomes = map.apply_update_spawn_group_conditions_set_inactive_like_cpp(
        [&automatic, &manual],
        |group| group.group_id == automatic.group_id,
    );

    assert_eq!(
        outcomes,
        vec![
            SpawnGroupConditionUpdateOutcomeLikeCpp {
                group_id: 43,
                action: SpawnGroupConditionActionLikeCpp::Noop,
                applied_change: None,
                despawn_outcome: None,
                spawn_outcome: None,
            },
            SpawnGroupConditionUpdateOutcomeLikeCpp {
                group_id: 44,
                action: SpawnGroupConditionActionLikeCpp::Noop,
                applied_change: None,
                despawn_outcome: None,
                spawn_outcome: None,
            },
        ]
    );
    assert!(map.is_spawn_group_active_like_cpp(Some(&automatic)));
    assert!(!map.is_spawn_group_active_like_cpp(Some(&manual)));
    assert!(map.spawn_group_state().toggled_spawn_group_ids().is_empty());
}
#[test]
fn check_respawn_like_cpp_inactive_spawn_group_stops_before_live_and_linked_like_cpp() {
    let map = test_map();
    let mut store = SpawnStore::new();
    let manual = spawn_group(61, SpawnGroupFlags::MANUAL_SPAWN);
    store.add_object_spawn(&spawn_data(SpawnObjectType::Creature, 100, manual), |_| {
        false
    });
    let this = linked_respawn_guid(HighGuid::Creature, 42, 100);
    let master = linked_respawn_guid(HighGuid::Creature, 77, 200);
    let mut linked = LinkedRespawnStoreLikeCpp::new();
    linked.insert_like_cpp(this, master);
    let mut info = respawn_info(SpawnObjectType::Creature, 100, 55);
    let mut escort_checked = false;

    let outcome = map.check_respawn_like_cpp(&mut info, &store, &linked, 1000, 5, true, |_, _| {
        escort_checked = true;
        false
    });

    assert_eq!(
        outcome,
        CheckRespawnCompositeOutcomeLikeCpp::InactiveSpawnGroupDeletedTimer
    );
    assert_eq!(info.respawn_time, 0);
    assert!(!escort_checked);
}
#[test]
fn check_respawn_like_cpp_areatrigger_manual_inactive_group_preserves_timer_like_cpp() {
    let map = test_map();
    let mut store = SpawnStore::new();
    let manual = spawn_group(66, SpawnGroupFlags::MANUAL_SPAWN);
    store.add_object_spawn(
        &spawn_data(SpawnObjectType::AreaTrigger, 103, manual),
        |_| false,
    );
    let linked = LinkedRespawnStoreLikeCpp::new();
    let mut info = respawn_info(SpawnObjectType::AreaTrigger, 103, 55);
    let mut escort_checked = false;

    let outcome = map.check_respawn_like_cpp(&mut info, &store, &linked, 1000, 5, true, |_, _| {
        escort_checked = true;
        false
    });

    assert_eq!(
        outcome,
        CheckRespawnCompositeOutcomeLikeCpp::UnsupportedSpawnType
    );
    assert_eq!(info.respawn_time, 55);
    assert!(!escort_checked);
}
