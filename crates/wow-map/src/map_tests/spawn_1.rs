//! Spawn scenarios for [`super`].
//!
//! Split out of map_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

#[test]
fn spawned_pool_data_pool_subpool_membership_and_counts_like_cpp() {
    let mut pool_data = SpawnedPoolDataLikeCpp::new();

    pool_data.add_pool_spawn_like_cpp(70, 7);
    assert!(pool_data.is_spawned_pool_like_cpp(70));
    assert_eq!(pool_data.get_spawned_objects_like_cpp(70), 0);
    assert_eq!(pool_data.get_spawned_objects_like_cpp(7), 1);

    pool_data.remove_pool_spawn_like_cpp(70, 7);
    assert!(!pool_data.is_spawned_pool_like_cpp(70));
    assert_eq!(pool_data.get_spawned_objects_like_cpp(7), 0);

    pool_data.remove_pool_spawn_like_cpp(70, 7);
    assert_eq!(pool_data.get_spawned_objects_like_cpp(7), 0);
}
#[test]
fn process_respawns_delete_only_missing_metadata_preserves_timer_like_cpp() {
    let mut map = test_map();
    let store = SpawnStore::new();
    map.add_respawn_info_like_cpp(respawn_info(SpawnObjectType::Creature, 44, 100));

    let summary = map.process_due_respawns_spawn_group_delete_only_like_cpp(100, &store);

    assert_eq!(summary.deleted_inactive_spawn_group, 0);
    assert_eq!(summary.blocked_missing_spawn_data, 1);
    assert_eq!(
        map.get_respawn_time_like_cpp(SpawnObjectType::Creature, 44),
        100
    );
}
#[test]
fn process_respawns_pool_respawn_one_despawns_without_removing_unrelated_respawn_timer_like_cpp() {
    let mut map = test_map();
    let mut store = SpawnStore::new();
    let active = spawn_group(32, SpawnGroupFlags::NONE);
    store.add_object_spawn(
        &spawn_data(SpawnObjectType::Creature, 73, active.clone()),
        |_| false,
    );
    store.add_object_spawn(&spawn_data(SpawnObjectType::Creature, 74, active), |_| {
        false
    });
    map.insert_map_object_record(
        MapObjectRecord::new_creature(test_creature_for_spawn(73, 7301, true)).unwrap(),
    )
    .unwrap();
    map.ensure_grid_loaded(&cell_from_world(0.0, 0.0));
    map.add_respawn_info_like_cpp(respawn_info(SpawnObjectType::Creature, 74, 150));
    let plan = PoolTypedSpawnPlanLikeCpp {
        kind: PoolMemberKindLikeCpp::Creature,
        pool_id: 173,
        trigger_from: 73,
        max_limit: Some(1),
        object_plan: Some(PoolSpawnObjectPlanLikeCpp {
            actions: vec![PoolSpawnObjectActionLikeCpp::RespawnOne {
                kind: PoolMemberKindLikeCpp::Creature,
                guid: 73,
            }],
            selected: vec![],
            despawned_trigger: None,
            respawned_trigger: true,
            ..PoolSpawnObjectPlanLikeCpp::default()
        }),
        skip_reason: None,
    };
    let mut summary = ProcessRespawnsSafeSideEffectsSummaryLikeCpp::default();

    map.apply_pool_typed_spawn_plan_safe_map_actions_like_cpp(&plan, &store, &mut summary);

    assert_eq!(summary.pool_objects_removed, 1);
    assert_eq!(summary.pool_spawn_actions_skipped_unloaded_grid, 0);
    assert_eq!(summary.pool_spawn_actions_blocked_loaded_grid, 1);
    assert_eq!(
        summary.pool_spawn_action_load_plans,
        vec![PoolSpawnActionLoadPlanLikeCpp {
            object_type: SpawnObjectType::Creature,
            spawn_id: 73,
            respawn: true,
        }]
    );
    assert_eq!(summary.pool_respawn_timers_removed, 0);
    assert_eq!(map.map_object_count(), 0);
    assert_eq!(
        map.get_respawn_time_like_cpp(SpawnObjectType::Creature, 74),
        150
    );
}
#[test]
fn despawn_pool_facade_consumes_child_pool_recursion_without_pool_timer_removal_like_cpp() {
    let mut map = test_map();
    map.insert_map_object_record(
        MapObjectRecord::new_creature(test_creature_for_spawn(528401, 52840101, true)).unwrap(),
    )
    .unwrap();
    map.add_respawn_info_like_cpp(respawn_info(SpawnObjectType::GameObject, 528402, 250));
    map.pool_data_mut_like_cpp()
        .add_pool_spawn_like_cpp(5284, 5280);
    map.pool_data_mut_like_cpp()
        .add_spawn_like_cpp(SpawnObjectType::Creature, 528401, 5284)
        .expect("test child creature pool state");
    let mut pool_mgr = PoolMgrLikeCpp::new();
    let mut parent_pool_group = PoolGroupLikeCpp::with_pool_id(PoolMemberKindLikeCpp::Pool, 5280);
    parent_pool_group.add_entry_like_cpp(PoolObjectLikeCpp::new(5284, 0.0), 1);
    pool_mgr
        .insert_or_replace_group_like_cpp(PoolMemberKindLikeCpp::Pool, 5280, parent_pool_group)
        .expect("test parent pool group");
    let mut child_creature_group =
        PoolGroupLikeCpp::with_pool_id(PoolMemberKindLikeCpp::Creature, 5284);
    child_creature_group.add_entry_like_cpp(PoolObjectLikeCpp::new(528401, 0.0), 1);
    pool_mgr
        .insert_or_replace_group_like_cpp(
            PoolMemberKindLikeCpp::Creature,
            5284,
            child_creature_group,
        )
        .expect("test child creature group");
    let mut child_gameobject_group =
        PoolGroupLikeCpp::with_pool_id(PoolMemberKindLikeCpp::GameObject, 5284);
    child_gameobject_group.add_entry_like_cpp(PoolObjectLikeCpp::new(528402, 0.0), 1);
    pool_mgr
        .insert_or_replace_group_like_cpp(
            PoolMemberKindLikeCpp::GameObject,
            5284,
            child_gameobject_group,
        )
        .expect("test child gameobject group");

    let summary = map
        .despawn_pool_safe_map_actions_like_cpp(&pool_mgr, 5280, true)
        .expect("despawn parent pool plan");

    assert_eq!(summary.pool_objects_removed, 1);
    assert_eq!(summary.pool_respawn_timers_removed, 1);
    assert_eq!(summary.pool_respawn_timers_missing, 0);
    assert_eq!(summary.pool_unsupported_action_kind, 0);
    assert_eq!(map.map_object_count(), 0);
    assert_eq!(map.creature_spawn_id_store_count_like_cpp(528401), 0);
    assert!(
        !map.pool_data_like_cpp()
            .is_spawned_creature_like_cpp(528401)
    );
    assert!(!map.pool_data_like_cpp().is_spawned_pool_like_cpp(5284));
    assert_eq!(
        map.pool_data_like_cpp().get_spawned_objects_like_cpp(5280),
        0
    );
    assert_eq!(
        map.pool_data_like_cpp().get_spawned_objects_like_cpp(5284),
        0
    );
    assert_eq!(
        map.get_respawn_time_like_cpp(SpawnObjectType::GameObject, 528402),
        0
    );
}
#[test]
fn process_respawns_delete_only_preserves_cpp_order_when_first_due_blocks_like_cpp() {
    let mut map = test_map();
    let mut store = SpawnStore::new();
    let active = spawn_group(15, SpawnGroupFlags::NONE);
    let manual = spawn_group(16, SpawnGroupFlags::MANUAL_SPAWN);
    store.add_object_spawn(&spawn_data(SpawnObjectType::Creature, 50, active), |_| {
        false
    });
    store.add_object_spawn(&spawn_data(SpawnObjectType::Creature, 40, manual), |_| {
        false
    });
    map.ensure_grid_loaded(&cell_from_grid_center(GridCoord::new(7, 0)));
    map.add_respawn_info_like_cpp(respawn_info(SpawnObjectType::Creature, 50, 90));
    map.add_respawn_info_like_cpp(respawn_info(SpawnObjectType::Creature, 40, 100));

    let summary = map.process_due_respawns_spawn_group_delete_only_like_cpp(100, &store);

    assert_eq!(summary.deleted_inactive_spawn_group, 0);
    assert_eq!(summary.blocked_do_respawn_runtime, 1);
    assert_eq!(
        map.get_respawn_time_like_cpp(SpawnObjectType::Creature, 50),
        90
    );
    assert_eq!(
        map.get_respawn_time_like_cpp(SpawnObjectType::Creature, 40),
        100
    );
}
#[test]
fn check_respawn_live_object_guard_dynamic_escort_closure_allows_only_when_config_enabled_like_cpp()
{
    let mut map = test_map();
    let mut store = SpawnStore::new();
    let group = spawn_group(23, SpawnGroupFlags::ESCORTQUESTNPC);
    store.add_object_spawn(
        &spawn_data(SpawnObjectType::Creature, 53, group.clone()),
        |_| false,
    );
    map.insert_map_object_record(
        MapObjectRecord::new_creature(test_creature_for_spawn(53, 53, true)).unwrap(),
    )
    .unwrap();

    let mut info_config_enabled = respawn_info(SpawnObjectType::Creature, 53, 100);
    let enabled_outcome = map.check_respawn_live_object_guard_like_cpp(
        &mut info_config_enabled,
        &store,
        true,
        |_, _| true,
    );
    assert_eq!(
        enabled_outcome,
        CheckRespawnLiveObjectGuardOutcomeLikeCpp::Allowed
    );
    assert_eq!(info_config_enabled.respawn_time, 100);

    let mut info_config_disabled = respawn_info(SpawnObjectType::Creature, 53, 100);
    let disabled_outcome = map.check_respawn_live_object_guard_like_cpp(
        &mut info_config_disabled,
        &store,
        false,
        |_, _| true,
    );
    assert_eq!(
        disabled_outcome,
        CheckRespawnLiveObjectGuardOutcomeLikeCpp::AliveCreatureBlocksRespawn
    );
    assert_eq!(info_config_disabled.respawn_time, 0);
}
#[test]
fn check_respawn_live_object_guard_missing_spawn_data_preserves_timer_like_cpp() {
    let map = test_map();
    let store = SpawnStore::new();
    let mut info = respawn_info(SpawnObjectType::Creature, 55, 100);

    let outcome =
        map.check_respawn_live_object_guard_like_cpp(&mut info, &store, false, |_, _| false);

    assert_eq!(
        outcome,
        CheckRespawnLiveObjectGuardOutcomeLikeCpp::MissingSpawnData
    );
    assert_eq!(info.respawn_time, 100);
}
#[test]
fn check_respawn_live_object_guard_area_trigger_unsupported_preserves_timer_like_cpp() {
    let map = test_map();
    let mut store = SpawnStore::new();
    let group = spawn_group(25, SpawnGroupFlags::NONE);
    store.add_object_spawn(&spawn_data(SpawnObjectType::AreaTrigger, 56, group), |_| {
        false
    });
    let mut info = respawn_info(SpawnObjectType::AreaTrigger, 56, 100);

    let outcome =
        map.check_respawn_live_object_guard_like_cpp(&mut info, &store, false, |_, _| false);

    assert_eq!(
        outcome,
        CheckRespawnLiveObjectGuardOutcomeLikeCpp::UnsupportedSpawnType
    );
    assert_eq!(info.respawn_time, 100);
}
#[test]
fn game_event_npc_flag_live_consumer_mutates_exact_spawn_low_bits_like_cpp() {
    let mut map = test_map();
    let mut first = test_creature_for_spawn(547, 54701, true);
    first.ai_ownership_mut().npc_flags = 0x1;
    let mut second = test_creature_for_spawn(547, 54702, true);
    second.ai_ownership_mut().npc_flags = 0x2;
    map.insert_map_object_record(MapObjectRecord::new_creature(first).unwrap())
        .unwrap();
    map.insert_map_object_record(MapObjectRecord::new_creature(second).unwrap())
        .unwrap();

    let outcome = map.update_game_event_npc_flags_by_spawn_id_like_cpp(547, 0x1_0000_00A5);

    assert_eq!(outcome.indexed_guids, 2);
    assert_eq!(outcome.live_creatures_mutated, 2);
    assert_eq!(outcome.npc_flags_low_applied, 2);
    assert_eq!(outcome.npc_flags2_applied, 2);
    for guid in map.creature_spawn_id_store_guids_like_cpp(547) {
        let creature = map
            .map_object_record(guid)
            .and_then(MapObjectRecord::creature)
            .unwrap();
        assert_eq!(creature.ai_ownership().npc_flags, 0xA5);
        assert_eq!(creature.ai_ownership().npc_flags2, 0x1);
        assert_eq!(creature.unit().data().npc_flags, [0xA5, 0x1]);
        assert!(
            creature
                .unit()
                .unit_data_changes_mask()
                .is_set(wow_entities::UNIT_DATA_NPC_FLAGS_PARENT_BIT)
        );
        assert!(
            creature
                .unit()
                .unit_data_changes_mask()
                .is_set(wow_entities::UNIT_DATA_NPC_FLAGS_FIRST_BIT)
        );
        assert!(
            creature
                .unit()
                .unit_data_changes_mask()
                .is_set(wow_entities::UNIT_DATA_NPC_FLAGS_FIRST_BIT + 1)
        );
    }
}
#[test]
fn game_event_npc_flag_live_consumer_wrong_kind_or_mismatched_spawn_no_mutation_like_cpp() {
    let mut map = test_map();
    let mut creature = test_creature_for_spawn(548, 54801, true);
    creature.ai_ownership_mut().npc_flags = 0x11;
    map.insert_map_object_record(MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();
    let guid = guid(HighGuid::Creature, 54801);
    if let Some(creature_index) = map.creatures_by_spawn_id.get_mut(&548) {
        creature_index.retain(|indexed_guid| *indexed_guid != guid);
    }
    map.creatures_by_spawn_id
        .entry(549)
        .or_default()
        .insert(guid);

    let outcome = map.update_game_event_npc_flags_by_spawn_id_like_cpp(549, 0x22);

    assert_eq!(outcome.indexed_guids, 1);
    assert_eq!(outcome.live_creatures_mutated, 0);
    assert_eq!(outcome.stale_index_or_wrong_kind, 1);
    let creature = map
        .map_object_record(guid)
        .and_then(MapObjectRecord::creature)
        .unwrap();
    assert_eq!(creature.ai_ownership().npc_flags, 0x11);
    assert_eq!(creature.ai_ownership().npc_flags2, 0);
}
#[test]
fn world_object_by_spawn_id_typed_getters_return_indexed_objects_like_cpp() {
    let mut map = test_map();
    let creature = test_creature_for_spawn(67, 6701, true);
    let creature_guid = creature.unit().world().guid();
    let gameobject = test_gameobject_for_spawn(68, 6801);
    let gameobject_guid = gameobject.world().guid();
    let area_trigger = test_area_trigger_for_spawn(69, 6901);
    let area_trigger_guid = area_trigger.world().guid();

    map.insert_map_object_record(MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();
    map.insert_map_object_record(MapObjectRecord::new_game_object(gameobject).unwrap())
        .unwrap();
    map.insert_map_object_record(MapObjectRecord::new_area_trigger(area_trigger).unwrap())
        .unwrap();

    let creature = map.get_creature_by_spawn_id_like_cpp(67).unwrap();
    assert_eq!(creature.unit().world().guid(), creature_guid);
    assert_eq!(
        creature.unit().world().position(),
        Position::xyz(1.0, 2.0, 3.0)
    );
    let gameobject = map.get_gameobject_by_spawn_id_like_cpp(68).unwrap();
    assert_eq!(gameobject.world().guid(), gameobject_guid);
    assert_eq!(gameobject.world().position(), Position::xyz(1.0, 2.0, 3.0));
    let area_trigger = map.get_area_trigger_by_spawn_id_like_cpp(69).unwrap();
    assert_eq!(area_trigger.world().guid(), area_trigger_guid);
    assert_eq!(
        area_trigger.world().position(),
        Position::xyz(1.0, 2.0, 3.0)
    );

    assert_eq!(
        map.get_world_object_by_spawn_id_like_cpp(SpawnObjectType::Creature, 67)
            .unwrap()
            .guid(),
        creature_guid
    );
    assert_eq!(
        map.get_world_object_by_spawn_id_like_cpp(SpawnObjectType::GameObject, 68)
            .unwrap()
            .guid(),
        gameobject_guid
    );
    assert_eq!(
        map.get_world_object_by_spawn_id_like_cpp(SpawnObjectType::AreaTrigger, 69)
            .unwrap()
            .guid(),
        area_trigger_guid
    );
}
#[test]
fn world_object_by_spawn_id_absent_and_zero_spawn_return_none_like_cpp() {
    let mut map = test_map();

    assert!(map.get_creature_by_spawn_id_like_cpp(75).is_none());
    assert!(map.get_gameobject_by_spawn_id_like_cpp(75).is_none());
    assert!(map.get_area_trigger_by_spawn_id_like_cpp(75).is_none());
    assert!(
        map.get_world_object_by_spawn_id_like_cpp(SpawnObjectType::Creature, 75)
            .is_none()
    );
    assert!(
        map.get_world_object_by_spawn_id_like_cpp(SpawnObjectType::GameObject, 75)
            .is_none()
    );
    assert!(
        map.get_world_object_by_spawn_id_like_cpp(SpawnObjectType::AreaTrigger, 75)
            .is_none()
    );

    map.insert_map_object_record(
        MapObjectRecord::new_creature(test_creature_for_spawn(0, 6001, true)).unwrap(),
    )
    .unwrap();
    map.insert_map_object_record(
        MapObjectRecord::new_game_object(test_gameobject_for_spawn(0, 6002)).unwrap(),
    )
    .unwrap();
    map.insert_map_object_record(
        MapObjectRecord::new_area_trigger(test_area_trigger_for_spawn(0, 6003)).unwrap(),
    )
    .unwrap();

    assert_eq!(map.creature_spawn_id_store_count_like_cpp(0), 0);
    assert_eq!(map.gameobject_spawn_id_store_count_like_cpp(0), 0);
    assert_eq!(map.area_trigger_spawn_id_store_count_like_cpp(0), 0);
    assert_eq!(map.map_object_count(), 3);
    assert!(map.get_creature_by_spawn_id_like_cpp(0).is_none());
    assert!(map.get_gameobject_by_spawn_id_like_cpp(0).is_none());
    assert!(map.get_area_trigger_by_spawn_id_like_cpp(0).is_none());
    assert!(
        map.get_world_object_by_spawn_id_like_cpp(SpawnObjectType::Creature, 0)
            .is_none()
    );
    assert!(
        map.get_world_object_by_spawn_id_like_cpp(SpawnObjectType::GameObject, 0)
            .is_none()
    );
    assert!(
        map.get_world_object_by_spawn_id_like_cpp(SpawnObjectType::AreaTrigger, 0)
            .is_none()
    );
}
#[test]
fn area_trigger_spawn_id_store_indexes_and_gets_typed_object_like_cpp() {
    let mut map = test_map();
    let area_trigger = test_area_trigger_for_spawn(70, 7001);
    let guid = area_trigger.world().guid();

    map.insert_map_object_record(MapObjectRecord::new_area_trigger(area_trigger).unwrap())
        .unwrap();

    assert_eq!(map.area_trigger_spawn_id_store_count_like_cpp(70), 1);
    assert_eq!(
        map.area_trigger_spawn_id_store_guids_like_cpp(70),
        vec![guid]
    );
    let stored = map.get_area_trigger_by_spawn_id_like_cpp(70).unwrap();
    assert_eq!(stored.world().guid(), guid);
    assert_eq!(stored.spawn_id(), 70);
}
#[test]
fn area_trigger_spawn_id_store_replacing_same_guid_moves_spawn_id_like_cpp() {
    let mut map = test_map();
    let guid = guid(HighGuid::AreaTrigger, 7201);

    map.insert_map_object_record(
        MapObjectRecord::new_area_trigger(test_area_trigger_for_spawn(72, 7201)).unwrap(),
    )
    .unwrap();
    let previous = map
        .insert_map_object_record(
            MapObjectRecord::new_area_trigger(test_area_trigger_for_spawn(73, 7201)).unwrap(),
        )
        .unwrap();

    assert!(previous.is_some());
    assert_eq!(map.area_trigger_spawn_id_store_count_like_cpp(72), 0);
    assert_eq!(map.area_trigger_spawn_id_store_count_like_cpp(73), 1);
    assert_eq!(
        map.area_trigger_spawn_id_store_guids_like_cpp(73),
        vec![guid]
    );
}
#[test]
fn area_trigger_spawn_id_store_absent_query_returns_none_like_cpp() {
    let map = test_map();

    assert_eq!(map.area_trigger_spawn_id_store_count_like_cpp(75), 0);
    assert!(
        map.area_trigger_spawn_id_store_guids_like_cpp(75)
            .is_empty()
    );
    assert!(map.get_area_trigger_by_spawn_id_like_cpp(75).is_none());
}
#[test]
fn dynamic_respawn_bg_or_arena_does_not_scale() {
    let mut context = dynamic_respawn_context(Some(SpawnObjectType::GameObject));
    context.is_battleground_or_arena = true;

    assert_dynamic_respawn_noop(
        context,
        DynamicRespawnScalingNoopReason::BattlegroundOrArena,
    );
}
#[test]
fn linked_respawn_time_missing_link_returns_zero_like_cpp() {
    let map = test_map();
    let store = LinkedRespawnStoreLikeCpp::new();

    assert_eq!(
        map.get_linked_respawn_time_like_cpp(
            linked_respawn_guid(HighGuid::Creature, 42, 100),
            &store,
        ),
        0
    );
}
#[test]
fn check_respawn_linked_respawn_guard_no_linked_time_leaves_info_unchanged_like_cpp() {
    let map = test_map();
    let linked = LinkedRespawnStoreLikeCpp::new();
    let mut info = respawn_info(SpawnObjectType::Creature, 100, 55);
    let original = info.clone();

    let outcome = map.check_respawn_linked_respawn_guard_like_cpp(&mut info, &linked, 1000, 5);

    assert_eq!(
        outcome,
        CheckRespawnLinkedRespawnGuardOutcomeLikeCpp::Allowed
    );
    assert_eq!(info, original);
}
#[test]
fn check_respawn_linked_respawn_guard_self_link_sets_week_like_cpp() {
    let mut map = test_map();
    map.add_respawn_info_like_cpp(RespawnInfoLikeCpp {
        object_type: SpawnObjectType::Creature,
        spawn_id: 100,
        entry: 42,
        respawn_time: 1200,
        grid_id: 7,
    });
    let this = linked_respawn_guid(HighGuid::Creature, 42, 100);
    let mut linked = LinkedRespawnStoreLikeCpp::new();
    linked.insert_like_cpp(this, this);
    let mut info = respawn_info(SpawnObjectType::Creature, 100, 55);

    let outcome = map.check_respawn_linked_respawn_guard_like_cpp(&mut info, &linked, 1000, 5);

    assert_eq!(
        outcome,
        CheckRespawnLinkedRespawnGuardOutcomeLikeCpp::LinkedSelfNeverRespawn
    );
    assert_eq!(info.respawn_time, 1000 + WEEK_SECS_LIKE_CPP);
}
#[test]
fn check_respawn_linked_respawn_guard_infinite_time_sets_i64_max_like_cpp() {
    let mut map = test_map();
    map.add_respawn_info_like_cpp(RespawnInfoLikeCpp {
        object_type: SpawnObjectType::GameObject,
        spawn_id: 200,
        entry: 77,
        respawn_time: i64::MAX,
        grid_id: 7,
    });
    let this = linked_respawn_guid(HighGuid::Creature, 42, 100);
    let master = linked_respawn_guid(HighGuid::GameObject, 77, 200);
    let mut linked = LinkedRespawnStoreLikeCpp::new();
    linked.insert_like_cpp(this, master);
    let mut info = respawn_info(SpawnObjectType::Creature, 100, 55);

    let outcome = map.check_respawn_linked_respawn_guard_like_cpp(&mut info, &linked, 1000, 15);

    assert_eq!(
        outcome,
        CheckRespawnLinkedRespawnGuardOutcomeLikeCpp::LinkedInfinite
    );
    assert_eq!(info.respawn_time, i64::MAX);
}
#[test]
fn check_respawn_linked_respawn_guard_delays_by_max_now_or_linked_plus_jitter_like_cpp() {
    let mut map = test_map();
    map.add_respawn_info_like_cpp(RespawnInfoLikeCpp {
        object_type: SpawnObjectType::Creature,
        spawn_id: 200,
        entry: 77,
        respawn_time: 900,
        grid_id: 7,
    });
    map.add_respawn_info_like_cpp(RespawnInfoLikeCpp {
        object_type: SpawnObjectType::GameObject,
        spawn_id: 300,
        entry: 88,
        respawn_time: 1200,
        grid_id: 7,
    });
    let this_past = linked_respawn_guid(HighGuid::Creature, 42, 100);
    let this_future = linked_respawn_guid(HighGuid::GameObject, 43, 101);
    let mut linked = LinkedRespawnStoreLikeCpp::new();
    linked.insert_like_cpp(this_past, linked_respawn_guid(HighGuid::Creature, 77, 200));
    linked.insert_like_cpp(
        this_future,
        linked_respawn_guid(HighGuid::GameObject, 88, 300),
    );

    let mut past = respawn_info(SpawnObjectType::Creature, 100, 55);
    let past_outcome = map.check_respawn_linked_respawn_guard_like_cpp(&mut past, &linked, 1000, 5);
    assert_eq!(
        past_outcome,
        CheckRespawnLinkedRespawnGuardOutcomeLikeCpp::LinkedDelayed
    );
    assert_eq!(past.respawn_time, 1005);

    let mut future = respawn_info(SpawnObjectType::GameObject, 101, 55);
    future.entry = 43;
    let future_outcome =
        map.check_respawn_linked_respawn_guard_like_cpp(&mut future, &linked, 1000, 15);
    assert_eq!(
        future_outcome,
        CheckRespawnLinkedRespawnGuardOutcomeLikeCpp::LinkedDelayed
    );
    assert_eq!(future.respawn_time, 1215);
}
#[test]
fn check_respawn_like_cpp_live_blocker_stops_before_linked_reschedule_like_cpp() {
    let mut map = test_map();
    let mut store = SpawnStore::new();
    let group = spawn_group(62, SpawnGroupFlags::NONE);
    store.add_object_spawn(&spawn_data(SpawnObjectType::Creature, 100, group), |_| {
        false
    });
    map.insert_map_object_record(
        MapObjectRecord::new_creature(test_creature_for_spawn(100, 100, true)).unwrap(),
    )
    .unwrap();
    map.add_respawn_info_like_cpp(RespawnInfoLikeCpp {
        object_type: SpawnObjectType::Creature,
        spawn_id: 200,
        entry: 77,
        respawn_time: 1200,
        grid_id: 7,
    });
    let this = linked_respawn_guid(HighGuid::Creature, 42, 100);
    let master = linked_respawn_guid(HighGuid::Creature, 77, 200);
    let mut linked = LinkedRespawnStoreLikeCpp::new();
    linked.insert_like_cpp(this, master);
    let mut info = respawn_info(SpawnObjectType::Creature, 100, 55);

    let outcome =
        map.check_respawn_like_cpp(&mut info, &store, &linked, 1000, 5, false, |_, _| false);

    assert_eq!(
        outcome,
        CheckRespawnCompositeOutcomeLikeCpp::AliveCreatureBlocksRespawn
    );
    assert_eq!(info.respawn_time, 0);
}
#[test]
fn check_respawn_like_cpp_linked_delayed_runs_after_allowed_guards_like_cpp() {
    let mut map = test_map();
    let mut store = SpawnStore::new();
    let group = spawn_group(63, SpawnGroupFlags::NONE);
    store.add_object_spawn(&spawn_data(SpawnObjectType::Creature, 100, group), |_| {
        false
    });
    map.add_respawn_info_like_cpp(RespawnInfoLikeCpp {
        object_type: SpawnObjectType::Creature,
        spawn_id: 200,
        entry: 77,
        respawn_time: 1200,
        grid_id: 7,
    });
    let this = linked_respawn_guid(HighGuid::Creature, 42, 100);
    let master = linked_respawn_guid(HighGuid::Creature, 77, 200);
    let mut linked = LinkedRespawnStoreLikeCpp::new();
    linked.insert_like_cpp(this, master);
    let mut info = respawn_info(SpawnObjectType::Creature, 100, 55);

    let outcome =
        map.check_respawn_like_cpp(&mut info, &store, &linked, 1000, 11, false, |_, _| false);

    assert_eq!(outcome, CheckRespawnCompositeOutcomeLikeCpp::LinkedDelayed);
    assert_eq!(info.respawn_time, 1211);
}
#[test]
fn check_respawn_like_cpp_missing_metadata_preserves_timer_and_stops_like_cpp() {
    let map = test_map();
    let store = SpawnStore::new();
    let mut linked = LinkedRespawnStoreLikeCpp::new();
    linked.insert_like_cpp(
        linked_respawn_guid(HighGuid::Creature, 42, 100),
        linked_respawn_guid(HighGuid::Creature, 77, 200),
    );
    let mut info = respawn_info(SpawnObjectType::Creature, 100, 55);
    let mut escort_checked = false;

    let outcome = map.check_respawn_like_cpp(&mut info, &store, &linked, 1000, 5, true, |_, _| {
        escort_checked = true;
        false
    });

    assert_eq!(
        outcome,
        CheckRespawnCompositeOutcomeLikeCpp::MissingSpawnData
    );
    assert_eq!(info.respawn_time, 55);
    assert!(!escort_checked);
}
#[test]
fn check_respawn_like_cpp_unsupported_areatrigger_preserves_timer_like_cpp() {
    let map = test_map();
    let mut store = SpawnStore::new();
    let group = spawn_group(65, SpawnGroupFlags::NONE);
    store.add_object_spawn(
        &spawn_data(SpawnObjectType::AreaTrigger, 102, group),
        |_| false,
    );
    let linked = LinkedRespawnStoreLikeCpp::new();
    let mut info = respawn_info(SpawnObjectType::AreaTrigger, 102, 55);

    let outcome =
        map.check_respawn_like_cpp(&mut info, &store, &linked, 1000, 5, false, |_, _| false);

    assert_eq!(
        outcome,
        CheckRespawnCompositeOutcomeLikeCpp::UnsupportedSpawnType
    );
    assert_eq!(info.respawn_time, 55);
}
#[test]
fn process_respawns_composite_linked_respawn_reschedules_future_timer_like_cpp() {
    let mut map = test_map();
    let mut store = SpawnStore::new();
    let group = spawn_group(69, SpawnGroupFlags::NONE);
    store.add_object_spawn(&spawn_data(SpawnObjectType::Creature, 100, group), |_| {
        false
    });
    map.add_respawn_info_like_cpp(respawn_info(SpawnObjectType::Creature, 100, 10));
    map.add_respawn_info_like_cpp(RespawnInfoLikeCpp {
        object_type: SpawnObjectType::Creature,
        spawn_id: 200,
        entry: 77,
        respawn_time: 1200,
        grid_id: 7,
    });
    let mut linked = LinkedRespawnStoreLikeCpp::new();
    linked.insert_like_cpp(
        linked_respawn_guid(HighGuid::Creature, 42, 100),
        linked_respawn_guid(HighGuid::Creature, 77, 200),
    );

    let summary = map.process_due_respawns_composite_delete_only_like_cpp(
        10,
        &store,
        &linked,
        5,
        false,
        |_, _| false,
    );

    assert_eq!(summary.rescheduled_linked_respawns.len(), 1);
    assert_eq!(summary.blocked_linked_respawn_non_future, 0);
    assert_eq!(summary.deleted_inactive_spawn_group, 0);
    assert_eq!(summary.deleted_live_object_blocker, 0);
    let rescheduled = &summary.rescheduled_linked_respawns[0];
    assert_eq!(rescheduled.spawn_id, 100);
    assert_eq!(rescheduled.respawn_time, 1205);
    assert_eq!(
        map.get_respawn_info_like_cpp(SpawnObjectType::Creature, 100)
            .unwrap()
            .respawn_time,
        1205
    );
}
#[test]
fn process_respawns_composite_linked_reschedule_allows_later_due_delete_like_cpp() {
    let mut map = test_map();
    let mut store = SpawnStore::new();
    let active = spawn_group(70, SpawnGroupFlags::NONE);
    let inactive = spawn_group(71, SpawnGroupFlags::MANUAL_SPAWN);
    store.add_object_spawn(&spawn_data(SpawnObjectType::Creature, 100, active), |_| {
        false
    });
    store.add_object_spawn(
        &spawn_data(SpawnObjectType::Creature, 101, inactive),
        |_| false,
    );
    map.add_respawn_info_like_cpp(respawn_info(SpawnObjectType::Creature, 100, 9));
    map.add_respawn_info_like_cpp(respawn_info(SpawnObjectType::Creature, 101, 10));
    map.add_respawn_info_like_cpp(RespawnInfoLikeCpp {
        object_type: SpawnObjectType::Creature,
        spawn_id: 200,
        entry: 77,
        respawn_time: 1200,
        grid_id: 7,
    });
    let mut linked = LinkedRespawnStoreLikeCpp::new();
    linked.insert_like_cpp(
        linked_respawn_guid(HighGuid::Creature, 42, 100),
        linked_respawn_guid(HighGuid::Creature, 77, 200),
    );

    let summary = map.process_due_respawns_composite_delete_only_like_cpp(
        10,
        &store,
        &linked,
        5,
        false,
        |_, _| false,
    );

    assert_eq!(summary.rescheduled_linked_respawns.len(), 1);
    assert_eq!(summary.deleted_inactive_spawn_group, 1);
    assert_eq!(
        map.get_respawn_info_like_cpp(SpawnObjectType::Creature, 100)
            .unwrap()
            .respawn_time,
        1205
    );
    assert!(
        map.get_respawn_info_like_cpp(SpawnObjectType::Creature, 101)
            .is_none()
    );
}
#[test]
fn dynamic_respawn_unsupported_type_and_missing_metadata_do_not_scale() {
    assert_dynamic_respawn_noop(
        dynamic_respawn_context(Some(SpawnObjectType::AreaTrigger)),
        DynamicRespawnScalingNoopReason::UnsupportedSpawnType,
    );

    let mut context = dynamic_respawn_context(Some(SpawnObjectType::GameObject));
    context.spawn_metadata_present = false;
    assert_dynamic_respawn_noop(
        context,
        DynamicRespawnScalingNoopReason::MissingSpawnMetadata,
    );
}
#[test]
fn dynamic_respawn_without_dynamic_spawn_rate_flag_does_not_scale() {
    let mut context = dynamic_respawn_context(Some(SpawnObjectType::GameObject));
    context.spawn_group_flags = Some(SpawnGroupFlags::NONE);

    assert_dynamic_respawn_noop(
        context,
        DynamicRespawnScalingNoopReason::MissingDynamicSpawnRateFlag,
    );
}
#[test]
fn dynamic_respawn_missing_or_zero_players_do_not_scale() {
    let mut missing = dynamic_respawn_context(Some(SpawnObjectType::GameObject));
    missing.zone_player_count = None;
    assert_dynamic_respawn_noop(
        missing,
        DynamicRespawnScalingNoopReason::MissingZonePlayerCount,
    );

    let mut zero = dynamic_respawn_context(Some(SpawnObjectType::GameObject));
    zero.zone_player_count = Some(0);
    assert_dynamic_respawn_noop(zero, DynamicRespawnScalingNoopReason::ZeroZonePlayers);
}
#[test]
fn dynamic_respawn_adjust_factor_at_least_one_does_not_scale() {
    let mut context = dynamic_respawn_context(Some(SpawnObjectType::GameObject));
    context.zone_player_count = Some(1);
    context.config.gameobject_rate = 1.0;

    assert_dynamic_respawn_noop(
        context,
        DynamicRespawnScalingNoopReason::AdjustFactorAtLeastOne,
    );
}
#[test]
fn dynamic_respawn_delay_at_or_below_minimum_does_not_scale() {
    let context = dynamic_respawn_context(Some(SpawnObjectType::GameObject));
    let outcome = apply_dynamic_mode_respawn_scaling_like_cpp(60, context);

    assert_eq!(outcome.delay_secs, 60);
    assert_eq!(
        outcome.noop_reason,
        Some(DynamicRespawnScalingNoopReason::DelayAtOrBelowMinimum)
    );
}
