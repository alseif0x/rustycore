use super::*;

#[test]
fn optional_no_loader_unavailable_and_full_preparation_rejection_remain_distinct() {
    let plan = typed(vec![spawn_action(201)]);
    let store = store(&[201]);
    type Loader = fn(
        &mut Map,
        SpawnObjectType,
        SpawnId,
    )
        -> Result<Option<LoadedGridMaterialization>, LoadedGridRespawnRecordsLikeCpp>;
    let mut shared = map(true);
    let no_loader = owned_typed::<Loader>(&mut shared, &plan, &store, None);
    assert_eq!(no_loader.summary.pool_spawn_actions_blocked_loaded_grid, 1);
    assert!(matches!(
        no_loader.attempts[0].result,
        LoadedGridSpawnAttemptResult::NoLoader
    ));
    assert!(!pool_plan(&no_loader.attempts[0].plan).respawn);
    let mut unavailable = |_: &mut Map, _, _| Ok(None);
    let unavailable = owned_typed(&mut shared, &plan, &store, Some(&mut unavailable));
    assert_eq!(unavailable.summary, no_loader.summary);
    assert!(matches!(
        unavailable.attempts[0].result,
        LoadedGridSpawnAttemptResult::Unavailable
    ));
    let mut rejected = Some(LoadedGridRespawnRecordsLikeCpp {
        pre_add_records: vec![player(2201, 571), player(2202, 530)],
        primary_record: record(201, 571),
    });
    let primary = rejected
        .as_ref()
        .unwrap()
        .primary_record
        .creature()
        .unwrap();
    let pointer = primary as *const Creature;
    let loot = primary.loot_authority_like_cpp().clone();
    let timeline = primary.unit().health_state_revision_authority_like_cpp();
    let facet = rejected.as_ref().unwrap().pre_add_records[0]
        .player()
        .unwrap() as *const Player;
    let mut loader = |_: &mut Map, _, _| Err(rejected.take().unwrap());
    let rejection = owned_typed(&mut shared, &plan, &store, Some(&mut loader));
    assert_eq!(rejection.summary, no_loader.summary);
    let attempt = rejection.attempts.into_iter().next().unwrap();
    assert_eq!(pool_plan(&attempt.plan).spawn_id, 201);
    let LoadedGridSpawnAttemptResult::PreparationRejected(records) = attempt.result else {
        panic!("full Records");
    };
    assert_eq!(
        records.primary_record.creature().unwrap() as *const Creature,
        pointer
    );
    assert_eq!(records.pre_add_records.len(), 2);
    assert_eq!(
        records.pre_add_records[0].player().unwrap() as *const Player,
        facet
    );
    let primary = records.primary_record.creature().unwrap();
    assert_eq!(primary.current_health(), 75);
    assert!(
        primary
            .loot_authority_like_cpp()
            .shares_storage_like_cpp(&loot)
    );
    assert!(
        primary
            .unit()
            .shares_health_state_revision_authority_like_cpp(&timeline)
    );
    assert_eq!(shared.map_object_count(), 0);
    // The existing Record None adapter is really absent, not Some(|| None).
    let mut old = map(true);
    let mut expected = ProcessRespawnsSafeSideEffectsSummaryLikeCpp::default();
    old.original_typed::<fn(&mut Map, SpawnObjectType, SpawnId) -> Option<LoadedGridRespawnRecordsLikeCpp>>(
        &plan, &store, &mut expected, None,
    );
    let mut actual = ProcessRespawnsSafeSideEffectsSummaryLikeCpp::default();
    shared.apply_pool_typed_spawn_plan_loaded_grid_records_like_cpp::<fn(&mut Map, SpawnObjectType, SpawnId) -> Option<LoadedGridRespawnRecordsLikeCpp>>(
        &plan, &store, &mut actual, None,
    );
    assert_eq!(actual, expected);
}

#[test]
fn metadata_grid_missing_child_and_pool_noop_gates_preserve_counters_without_loader_calls() {
    let mut store = store(&[203]);
    let mut unloaded = metadata(SpawnObjectType::Creature, 202);
    unloaded.spawn_point = SpawnPosition::new(1000.0, 1000.0, 0.0, 0.0);
    store.add_object_spawn(&unloaded, |_| false);
    let plan = typed(vec![
        spawn_action(202),
        spawn_action(999),
        PoolSpawnObjectActionLikeCpp::SpawnOne {
            kind: PoolMemberKindLikeCpp::Pool,
            guid: 88,
        },
        PoolSpawnObjectActionLikeCpp::DespawnOne {
            kind: PoolMemberKindLikeCpp::Pool,
            guid: 89,
        },
        PoolSpawnObjectActionLikeCpp::RespawnOne {
            kind: PoolMemberKindLikeCpp::Pool,
            guid: 90,
        },
        PoolSpawnObjectActionLikeCpp::RemoveRespawnTime {
            kind: PoolMemberKindLikeCpp::Pool,
            guid: 91,
        },
    ]);
    let mut old = map(true);
    let mut expected = ProcessRespawnsSafeSideEffectsSummaryLikeCpp::default();
    old.original_typed(
        &plan,
        &store,
        &mut expected,
        Some(&mut |_: &mut Map, _, _| panic!("gated")),
    );
    let mut shared = map(true);
    let outcome = owned_typed(
        &mut shared,
        &plan,
        &store,
        Some(&mut |_: &mut Map, _, _| panic!("gated")),
    );
    assert_eq!(outcome.summary, expected);
    assert_eq!(outcome.summary.pool_spawn_actions_skipped_unloaded_grid, 1);
    assert_eq!(outcome.summary.pool_spawn_actions_missing_spawn_data, 1);
    assert_eq!(outcome.summary.pool_unsupported_action_kind, 2);
    assert!(outcome.attempts.is_empty());
    assert_eq!(shared.map_object_count(), 0);
}
