use super::*;
use std::cell::RefCell;

#[test]
fn real_nested_plan_preserves_pool_gameobject_creature_order_and_never_replans_during_loads() {
    let mut manager = PoolMgrLikeCpp::new();
    add_group(&mut manager, 1, PoolMemberKindLikeCpp::Pool, &[(2, 0.0)], 1);
    add_group(&mut manager, 1, PoolMemberKindLikeCpp::GameObject, &[(303, 0.0)], 1);
    add_group(&mut manager, 1, PoolMemberKindLikeCpp::Creature, &[(304, 0.0)], 1);
    add_group(&mut manager, 2, PoolMemberKindLikeCpp::GameObject, &[(301, 0.0)], 1);
    add_group(&mut manager, 2, PoolMemberKindLikeCpp::Creature, &[(302, 0.0)], 1);
    let mut store = SpawnStore::new();
    for (kind, id) in [(SpawnObjectType::GameObject, 301), (SpawnObjectType::Creature, 302),
        (SpawnObjectType::GameObject, 303), (SpawnObjectType::Creature, 304)] {
        store.add_object_spawn(&metadata(kind, id), |_| false);
    }
    let mut old = map(true);
    let mut shared = map(true);
    let old_events = RefCell::new(Vec::new());
    let mut expected = old.original_facade(&manager, 1, &store,
        |_, _| { old_events.borrow_mut().push("roll"); 0.0 },
        |_, count| { old_events.borrow_mut().push("pick"); (0..count).collect() },
        |_, kind, id| { old_events.borrow_mut().push("load");
            Some(LoadedGridRespawnRecordsLikeCpp::primary_only(kind_record(kind, id))) },
    ).unwrap();
    let events = RefCell::new(Vec::new());
    let mut calls = Vec::new();
    let outcome = shared.spawn_pool_materialized(&manager, 1, &store,
        |_, _| { events.borrow_mut().push("roll"); 0.0 },
        |_, count| { events.borrow_mut().push("pick"); (0..count).collect() },
        |_, kind, id| { events.borrow_mut().push("load"); calls.push((kind, id));
            Ok(Some(LoadedGridMaterialization::records(LoadedGridRespawnRecordsLikeCpp::primary_only(kind_record(kind, id))))) },
    ).unwrap();
    assert_eq!(events, old_events);
    let events = events.into_inner();
    let first_load = events.iter().position(|event| *event == "load").unwrap();
    assert!(events[first_load..].iter().all(|event| *event == "load"));
    assert_eq!(calls, vec![(SpawnObjectType::GameObject, 301), (SpawnObjectType::Creature, 302),
        (SpawnObjectType::GameObject, 303), (SpawnObjectType::Creature, 304)]);
    let snapshots = std::mem::take(&mut expected.loaded_grid_primary_records);
    assert_eq!(outcome.summary, expected);
    assert!(outcome.summary.loaded_grid_primary_records.is_empty());
    assert_eq!(outcome.attempts.len(), snapshots.len());
    for (attempt, snapshot) in outcome.attempts.into_iter().zip(snapshots) {
        assert!(!pool_plan(&attempt.plan).respawn);
        let LoadedGridSpawnAttemptResult::Admitted(admission) = attempt.result else { panic!("admission"); };
        let (_, primary) = admission.into_parts();
        let LoadedGridPrimaryAdmission::Record { snapshot: actual, result } = primary else { panic!("Record"); };
        assert!(result.is_ok());
        assert_eq!(actual.kind(), snapshot.kind());
        assert_eq!(actual.object().guid(), snapshot.object().guid());
    }
    assert_eq!(shared.pool_data_like_cpp(), old.pool_data_like_cpp());
    assert_eq!(shared.map_object_count(), old.map_object_count());
    // A skipped typed specialization does not emit a fake load receipt.
    let mut skipped = typed(Vec::new());
    skipped.object_plan = None;
    let skipped = owned_typed(&mut shared, &skipped, &store, Some(&mut |_: &mut Map, _, _| panic!("skip")));
    assert!(skipped.attempts.is_empty());
}

#[test]
fn respawn_one_removes_body_before_loader_and_retains_unrelated_timer_and_respawn_metadata() {
    let store = store(&[311, 312]);
    let plan = typed(vec![PoolSpawnObjectActionLikeCpp::RespawnOne { kind: PoolMemberKindLikeCpp::Creature, guid: 311 }, spawn_action(312)]);
    let mut shared = map(true);
    let old_guid = record(311, 571).object().guid();
    shared.insert_map_object_record(record(311, 571)).unwrap();
    timer(&mut shared, 399);
    let mut calls = Vec::new();
    let outcome = owned_typed(&mut shared, &plan, &store, Some(&mut |map: &mut Map, _, id| {
        calls.push(id);
        if id == 311 {
            assert!(map.get_typed_creature(old_guid).is_none());
            assert_eq!(map.creature_spawn_id_store_count_like_cpp(311), 0);
        }
        Ok(Some(LoadedGridMaterialization::records(LoadedGridRespawnRecordsLikeCpp::primary_only(record(id, 571)))))
    }));
    assert_eq!(calls, vec![311, 312]);
    assert_eq!(outcome.summary.pool_objects_removed, 1);
    assert_eq!(outcome.summary.executed_loaded_grid_respawns, 2);
    assert!(pool_plan(&outcome.attempts[0].plan).respawn);
    assert!(!pool_plan(&outcome.attempts[1].plan).respawn);
    assert_eq!(shared.get_respawn_time_like_cpp(SpawnObjectType::Creature, 399), 12345);
}

#[test]
fn nested_despawn_and_timer_actions_keep_original_cursor_order_and_removal_summary() {
    let store = store(&[321, 322]);
    let mut plan = typed(vec![PoolSpawnObjectActionLikeCpp::DespawnOne { kind: PoolMemberKindLikeCpp::Pool, guid: 5 }, spawn_action(322)]);
    plan.object_plan.as_mut().unwrap().child_pool_despawn_plans.push(PoolDespawnPoolPlanLikeCpp {
        pool_id: 5, always_delete_respawn_time: true,
        subplans: vec![PoolTypedDespawnPlanLikeCpp {
            kind: PoolMemberKindLikeCpp::Creature, pool_id: 5, requested_guid: 0,
            always_delete_respawn_time: true, skip_reason: None,
            object_plan: Some(PoolDespawnObjectPlanLikeCpp {
                actions: vec![PoolSpawnObjectActionLikeCpp::DespawnOne { kind: PoolMemberKindLikeCpp::Creature, guid: 321 },
                    PoolSpawnObjectActionLikeCpp::RemoveRespawnTime { kind: PoolMemberKindLikeCpp::Creature, guid: 323 },
                    PoolSpawnObjectActionLikeCpp::RemoveRespawnTime { kind: PoolMemberKindLikeCpp::Creature, guid: 323 }],
                ..Default::default()
            }),
        }],
    });
    let mut old = map(true);
    let mut shared = map(true);
    for map in [&mut old, &mut shared] {
        map.insert_map_object_record(record(321, 571)).unwrap();
        timer(map, 323);
    }
    let guid = record(321, 571).object().guid();
    let mut expected = ProcessRespawnsSafeSideEffectsSummaryLikeCpp::default();
    old.original_typed(&plan, &store, &mut expected, Some(&mut |map: &mut Map, _, id| {
        assert!(map.get_typed_creature(guid).is_none());
        assert_eq!(map.get_respawn_time_like_cpp(SpawnObjectType::Creature, 323), 0);
        Some(LoadedGridRespawnRecordsLikeCpp::primary_only(record(id, 571)))
    }));
    let outcome = owned_typed(&mut shared, &plan, &store, Some(&mut |map: &mut Map, _, id| {
        assert!(map.get_typed_creature(guid).is_none());
        assert_eq!(map.get_respawn_time_like_cpp(SpawnObjectType::Creature, 323), 0);
        Ok(Some(LoadedGridMaterialization::records(LoadedGridRespawnRecordsLikeCpp::primary_only(record(id, 571)))))
    }));
    expected.loaded_grid_primary_records.clear();
    assert_eq!(outcome.summary, expected);
    assert_eq!(outcome.summary.pool_objects_removed, 1);
    assert_eq!(outcome.summary.pool_respawn_timers_removed, 1);
    assert_eq!(outcome.summary.pool_respawn_timers_missing, 1);
    assert_eq!(outcome.attempts.len(), 1);
    assert_eq!(pool_plan(&outcome.attempts[0].plan).spawn_id, 322);
}
