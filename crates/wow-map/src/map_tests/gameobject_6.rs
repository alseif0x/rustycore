//! Gameobject scenarios for [`super`].
//!
//! Split out of map_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

#[test]
fn gameobject_update_just_deactivated_empty_self_missing_trap_is_noop_like_cpp() {
    let mut map = test_map();
    let mut empty = game_object_with_counter(4580301, 571, 7, false);
    let mut self_linked = game_object_with_counter(4580302, 571, 7, false);
    let mut missing = game_object_with_counter(4580303, 571, 7, false);
    let unrelated = game_object_with_counter(4580304, 571, 7, false);
    let empty_guid = empty.world().guid();
    let self_guid = self_linked.world().guid();
    let missing_guid = missing.world().guid();
    let missing_trap_guid = guid(HighGuid::GameObject, 4580399);
    let unrelated_guid = unrelated.world().guid();
    empty.set_loot_state(LootState::JustDeactivated, None);
    self_linked.set_loot_state(LootState::JustDeactivated, None);
    self_linked.set_linked_trap_like_cpp(self_guid);
    missing.set_loot_state(LootState::JustDeactivated, None);
    missing.set_linked_trap_like_cpp(missing_trap_guid);

    map.add_map_object_record_to_map_like_cpp(MapObjectRecord::new_game_object(empty).unwrap())
        .unwrap();
    map.add_map_object_record_to_map_like_cpp(
        MapObjectRecord::new_game_object(self_linked).unwrap(),
    )
    .unwrap();
    map.add_map_object_record_to_map_like_cpp(MapObjectRecord::new_game_object(missing).unwrap())
        .unwrap();
    map.add_map_object_record_to_map_like_cpp(MapObjectRecord::new_game_object(unrelated).unwrap())
        .unwrap();

    let empty_outcome = map.update_game_object_like_cpp(empty_guid, 1, 1_000);
    let self_outcome = map.update_game_object_like_cpp(self_guid, 1, 1_000);
    let missing_outcome = map.update_game_object_like_cpp(missing_guid, 1, 1_000);

    assert_eq!(empty_outcome.linked_trap_guid, None);
    assert!(!empty_outcome.linked_trap_removed);
    assert!(empty_outcome.linked_trap_missing_or_self);
    assert_eq!(self_outcome.linked_trap_guid, Some(self_guid));
    assert!(!self_outcome.linked_trap_removed);
    assert!(self_outcome.linked_trap_missing_or_self);
    assert_eq!(missing_outcome.linked_trap_guid, Some(missing_trap_guid));
    assert!(!missing_outcome.linked_trap_removed);
    assert!(missing_outcome.linked_trap_missing_or_self);
    assert!(map.map_object_record(empty_guid).is_some());
    assert!(map.map_object_record(self_guid).is_some());
    assert!(map.map_object_record(missing_guid).is_some());
    assert!(map.map_object_record(unrelated_guid).is_some());
}
#[test]
fn gameobject_update_summary_counts_linked_trap_remove_queue_like_cpp() {
    let mut map = test_map();
    let mut owner = game_object_with_counter(4580401, 571, 7, false);
    let trap = game_object_with_counter(4580402, 571, 7, false);
    let owner_guid = owner.world().guid();
    let trap_guid = trap.world().guid();
    owner.set_loot_state(LootState::JustDeactivated, None);
    owner.set_respawn_delay_time(0);
    owner.set_linked_trap_like_cpp(trap_guid);

    map.add_map_object_record_to_map_like_cpp(MapObjectRecord::new_game_object(trap).unwrap())
        .unwrap();
    map.add_map_object_record_to_map_like_cpp(MapObjectRecord::new_game_object(owner).unwrap())
        .unwrap();

    let summary = map.update_game_objects_like_cpp(1, 1_000);

    assert_eq!(summary.linked_traps_removed, 0);
    assert_eq!(summary.linked_traps_remove_queued, 1);
    assert_eq!(summary.loot_cleared, 1);
    assert!(summary.visited >= 1);
    assert!(map.map_object_record(owner_guid).is_some());
    assert!(map.map_object_record(trap_guid).is_some());
    assert_eq!(map.objects_to_remove_count_like_cpp(), 1);
}
#[test]
fn remove_list_drain_gameobject_owner_removes_linked_trap_like_cpp() {
    let mut map = test_map();
    let mut owner = game_object_with_counter(4190601, 571, 7, false);
    let trap = game_object_with_counter(4190602, 571, 7, false);
    let owner_guid = owner.world().guid();
    let trap_guid = trap.world().guid();
    owner.set_linked_trap_like_cpp(trap_guid);

    map.add_map_object_record_to_map_like_cpp(MapObjectRecord::new_game_object(trap).unwrap())
        .unwrap();
    map.add_map_object_record_to_map_like_cpp(MapObjectRecord::new_game_object(owner).unwrap())
        .unwrap();
    assert!(map.add_object_to_remove_list_like_cpp(owner_guid).queued);

    let outcome = map.remove_all_objects_in_remove_list_like_cpp();

    assert_eq!(outcome.processed, 2);
    assert_eq!(outcome.removed, 2);
    assert_eq!(map.objects_to_remove_count_like_cpp(), 0);
    assert!(map.map_object_record(owner_guid).is_none());
    assert!(map.map_object_record(trap_guid).is_none());
}
#[test]
fn gameobject_delete_compatibility_pool_updates_pool_without_remove_list_like_cpp() {
    let mut map = test_map();
    let mut store = SpawnStore::new();
    let active = spawn_group(531, SpawnGroupFlags::NONE);
    let mut trigger_spawn = spawn_data(SpawnObjectType::GameObject, 53101, active.clone());
    trigger_spawn.pool_id = 55;
    let mut replacement_spawn = spawn_data(SpawnObjectType::GameObject, 53102, active);
    replacement_spawn.pool_id = 55;
    store.add_object_spawn(&trigger_spawn, |_| false);
    store.add_object_spawn(&replacement_spawn, |_| false);

    let mut pool_mgr = PoolMgrLikeCpp::new();
    pool_mgr.insert_template_like_cpp(55, PoolTemplateDataLikeCpp::new(1, 571));
    let mut group = PoolGroupLikeCpp::with_pool_id(PoolMemberKindLikeCpp::GameObject, 55);
    group.add_entry_like_cpp(PoolObjectLikeCpp::new(53101, 0.0), 1);
    group.add_entry_like_cpp(PoolObjectLikeCpp::new(53102, 0.0), 1);
    pool_mgr
        .insert_or_replace_group_like_cpp(PoolMemberKindLikeCpp::GameObject, 55, group)
        .expect("test pool group");
    pool_mgr
        .register_spawn_pool_relation_like_cpp(PoolMemberKindLikeCpp::GameObject, 53101, 55)
        .expect("test pool relation");

    let mut gameobject = test_gameobject_for_spawn(53101, 5310101);
    let guid = gameobject.world().guid();
    gameobject.set_respawn_compatibility_mode(true);
    gameobject.set_represented_gameobject_data_present_like_cpp(true);
    gameobject.set_go_state(GoState::Active);
    map.add_map_object_record_to_map_like_cpp(
        MapObjectRecord::new_game_object(gameobject).unwrap(),
    )
    .unwrap();
    assert!(
        map.pool_data_mut_like_cpp()
            .add_spawn_like_cpp(SpawnObjectType::GameObject, 53101, 55)
            .is_ok()
    );

    let outcome = map
        .gameobject_delete_with_pool_update_like_cpp(
            guid,
            &store,
            &pool_mgr,
            |_, _| 0.0,
            |_candidates, _count| vec![1],
        )
        .expect("delete outcome");

    assert!(outcome.pool_update_represented);
    assert!(outcome.pool_update_error.is_none());
    assert!(outcome.pool_update_plan.is_some());
    assert!(outcome.pool_update_summary.is_some());
    assert!(outcome.remove_list.is_none());
    assert_eq!(map.objects_to_remove_count_like_cpp(), 0);
    let summary = outcome.pool_update_summary.as_ref().unwrap();
    assert_eq!(summary.pool_objects_removed, 1);
    assert_eq!(summary.pool_spawn_actions_skipped_unloaded_grid, 1);
    assert!(
        map.pool_data_like_cpp()
            .is_spawned_gameobject_like_cpp(53102)
    );
    assert_eq!(map.pool_data_like_cpp().get_spawned_objects_like_cpp(55), 1);
    assert!(map.map_object_record(guid).is_none());
}
#[test]
fn gameobject_delete_without_compatibility_pool_keeps_remove_list_like_cpp() {
    let mut map = test_map();
    let mut store = SpawnStore::new();
    let active = spawn_group(532, SpawnGroupFlags::NONE);
    let mut trigger_spawn = spawn_data(SpawnObjectType::GameObject, 53201, active);
    trigger_spawn.pool_id = 56;
    store.add_object_spawn(&trigger_spawn, |_| false);

    let mut pool_mgr = PoolMgrLikeCpp::new();
    pool_mgr.insert_template_like_cpp(56, PoolTemplateDataLikeCpp::new(1, 571));
    let mut group = PoolGroupLikeCpp::with_pool_id(PoolMemberKindLikeCpp::GameObject, 56);
    group.add_entry_like_cpp(PoolObjectLikeCpp::new(53201, 0.0), 1);
    pool_mgr
        .insert_or_replace_group_like_cpp(PoolMemberKindLikeCpp::GameObject, 56, group)
        .expect("test pool group");
    pool_mgr
        .register_spawn_pool_relation_like_cpp(PoolMemberKindLikeCpp::GameObject, 53201, 56)
        .expect("test pool relation");

    let mut gameobject = test_gameobject_for_spawn(53201, 5320101);
    let guid = gameobject.world().guid();
    gameobject.set_respawn_compatibility_mode(false);
    gameobject.set_represented_gameobject_data_present_like_cpp(true);
    gameobject.replace_loot_authority_like_cpp(None, HashMap::new());
    let loot_authority = gameobject.loot_authority_like_cpp().clone();
    assert!(!loot_authority.is_retired_like_cpp());
    map.add_map_object_record_to_map_like_cpp(
        MapObjectRecord::new_game_object(gameobject).unwrap(),
    )
    .unwrap();

    let outcome = map
        .gameobject_delete_with_pool_update_like_cpp(
            guid,
            &store,
            &pool_mgr,
            |_, _| 0.0,
            |_candidates, count| (0..count).collect(),
        )
        .expect("delete outcome");

    assert!(!outcome.pool_update_represented);
    assert!(outcome.pool_update_plan.is_none());
    assert!(outcome.pool_update_summary.is_none());
    assert!(
        outcome
            .remove_list
            .as_ref()
            .is_some_and(|remove| remove.queued)
    );
    assert_eq!(map.objects_to_remove_count_like_cpp(), 1);
    assert_eq!(map.pool_data_like_cpp().get_spawned_objects_like_cpp(56), 0);
    assert!(
        loot_authority.is_retired_like_cpp(),
        "queued GameObject deletion must invalidate Arc-held loot claims"
    );
}
