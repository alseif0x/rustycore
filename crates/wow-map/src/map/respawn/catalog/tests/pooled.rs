use super::*;
use std::cell::RefCell;

fn pools(template: bool) -> PoolMgrLikeCpp {
    let mut pools = PoolMgrLikeCpp::new();
    if template {
        pools.insert_template_like_cpp(3, PoolTemplateDataLikeCpp::new(1, 571));
    }
    let mut group = PoolGroupLikeCpp::with_pool_id(PoolMemberKindLikeCpp::Creature, 3);
    for id in [301, 302] {
        group.add_entry_like_cpp(PoolObjectLikeCpp::new(id, 0.0), 1);
    }
    pools
        .insert_or_replace_group_like_cpp(PoolMemberKindLikeCpp::Creature, 3, group)
        .unwrap();
    pools
        .register_spawn_pool_relation_like_cpp(PoolMemberKindLikeCpp::Creature, 301, 3)
        .unwrap();
    pools
}

#[derive(Debug, PartialEq)]
enum Event {
    Roll(PoolMemberKindLikeCpp, u32),
    Pick(Vec<u64>, usize),
    Load(SpawnObjectType, SpawnId, i64),
}

#[test]
fn pooled_planner_and_recursive_loader_match_oracle_before_trigger_timer_removal() {
    let pools = pools(true);
    let store = store(&[
        (SpawnObjectType::Creature, 301),
        (SpawnObjectType::Creature, 302),
    ]);
    let mut old = map(true);
    let mut shared = map(true);
    for map in [&mut old, &mut shared] {
        timer(map, SpawnObjectType::Creature, 301, 100);
    }
    let previous = RefCell::new(Vec::new());
    let expected = old.original_catalog(
        100,
        &store,
        &LinkedRespawnStoreLikeCpp::new(),
        &pools,
        5,
        false,
        |_, _| false,
        |kind, id| {
            previous.borrow_mut().push(Event::Roll(kind, id));
            0.0
        },
        |objects, count| {
            previous.borrow_mut().push(Event::Pick(
                objects.iter().map(|object| object.guid).collect(),
                count,
            ));
            (0..count).collect()
        },
        false,
        |map, kind, id| {
            previous.borrow_mut().push(Event::Load(
                kind,
                id,
                map.get_respawn_time_like_cpp(SpawnObjectType::Creature, 301),
            ));
            Some(LoadedGridRespawnRecordsLikeCpp::primary_only(record(
                kind, id, 571,
            )))
        },
    );
    let events = RefCell::new(Vec::new());
    let actual = shared.process_due_respawns_composite_loaded_grid_respawns_like_cpp(
        100,
        &store,
        &LinkedRespawnStoreLikeCpp::new(),
        &pools,
        5,
        false,
        |_, _| false,
        |kind, id| {
            events.borrow_mut().push(Event::Roll(kind, id));
            0.0
        },
        |objects, count| {
            events.borrow_mut().push(Event::Pick(
                objects.iter().map(|object| object.guid).collect(),
                count,
            ));
            (0..count).collect()
        },
        false,
        |map, kind, id| {
            assert_eq!(
                map.get_respawn_time_like_cpp(SpawnObjectType::Creature, 301),
                100
            );
            events.borrow_mut().push(Event::Load(kind, id, 100));
            Some(LoadedGridRespawnRecordsLikeCpp::primary_only(record(
                kind, id, 571,
            )))
        },
    );
    assert_eq!(events, previous);
    assert_eq!(shared.pool_data_like_cpp(), old.pool_data_like_cpp());
    assert_eq!(actual.processed_pool_timers, 1);
    assert_eq!(
        shared.get_respawn_time_like_cpp(SpawnObjectType::Creature, 301),
        0
    );
    assert_summary(actual, expected);
}

#[test]
fn pooled_receipts_remain_pool_and_failures_consume_trigger_independently_of_nonpool_flag() {
    for consume in [false, true] {
        for mode in 0..4 {
            let pools = pools(true);
            let store = store(&[
                (SpawnObjectType::Creature, 301),
                (SpawnObjectType::Creature, 302),
            ]);
            let mut shared = map(true);
            timer(&mut shared, SpawnObjectType::Creature, 301, 100);
            let mut rejected = Some(LoadedGridRespawnRecordsLikeCpp {
                pre_add_records: vec![player(3302, 571)],
                primary_record: record(SpawnObjectType::Creature, 302, 571),
            });
            let pointer = rejected
                .as_ref()
                .unwrap()
                .primary_record
                .creature()
                .unwrap() as *const Creature;
            let outcome = run_owned(&mut shared, &store, &pools, consume, |map, kind, id| {
                assert_eq!(
                    map.get_respawn_time_like_cpp(SpawnObjectType::Creature, 301),
                    100
                );
                assert_eq!(id, 302);
                match mode {
                    0 => Ok(None),
                    1 => Err(rejected.take().unwrap()),
                    _ => Ok(Some(LoadedGridMaterialization::records(
                        LoadedGridRespawnRecordsLikeCpp::primary_only(record(
                            kind,
                            id,
                            if mode == 2 { 571 } else { 530 },
                        )),
                    ))),
                }
            });
            assert_eq!(outcome.summary.processed_pool_timers, 1);
            assert_eq!(outcome.summary.pool_update_plans.len(), 1);
            assert_eq!(outcome.summary.blocked_loaded_grid_respawn_loads, 0);
            assert_eq!(outcome.summary.blocked_do_respawn_runtime, 0);
            assert_eq!(
                outcome.summary.pool_spawn_actions_blocked_loaded_grid,
                usize::from(mode <= 1)
            );
            assert_eq!(
                shared.get_respawn_time_like_cpp(SpawnObjectType::Creature, 301),
                0
            );
            let attempt = outcome.attempts.into_iter().next().unwrap();
            let LoadedGridAttemptPlan::Pool(plan) = attempt.plan else {
                panic!("pooled receipt");
            };
            assert_eq!(plan.spawn_id, 302);
            match attempt.result {
                LoadedGridSpawnAttemptResult::Unavailable => assert_eq!(mode, 0),
                LoadedGridSpawnAttemptResult::PreparationRejected(records) => {
                    assert_eq!(mode, 1);
                    assert_eq!(
                        records.primary_record.creature().unwrap() as *const Creature,
                        pointer
                    );
                    assert_eq!(records.pre_add_records.len(), 1);
                }
                LoadedGridSpawnAttemptResult::Admitted(admission) => {
                    let (_, primary) = admission.into_parts();
                    let LoadedGridPrimaryAdmission::Record { result, .. } = primary else {
                        panic!("Record");
                    };
                    assert_eq!(result.is_ok(), mode == 2);
                }
                LoadedGridSpawnAttemptResult::NoLoader => {
                    panic!("Catalog always forwards its actual loader")
                }
            }
        }
    }
}

#[test]
fn actual_pool_missing_template_failure_retains_trigger_and_original_pool_data() {
    let pools = pools(false);
    let store = store(&[
        (SpawnObjectType::Creature, 301),
        (SpawnObjectType::Creature, 302),
    ]);
    let mut old = map(true);
    let mut shared = map(true);
    for map in [&mut old, &mut shared] {
        timer(map, SpawnObjectType::Creature, 301, 100);
    }
    let expected = run_original(&mut old, &store, &pools, true, |_, _, _| {
        panic!("planner failure")
    });
    let actual = run_owned(&mut shared, &store, &pools, true, |_, _, _| {
        panic!("planner failure")
    });
    assert_eq!(actual.summary, expected);
    assert_eq!(
        actual.summary.blocked_pool_plan_errors,
        vec![PoolMgrPlanErrorLikeCpp::MissingTemplate { pool_id: 3 }]
    );
    assert!(actual.attempts.is_empty());
    assert_eq!(
        shared.get_respawn_time_like_cpp(SpawnObjectType::Creature, 301),
        100
    );
    assert_eq!(shared.pool_data_like_cpp(), old.pool_data_like_cpp());
}
