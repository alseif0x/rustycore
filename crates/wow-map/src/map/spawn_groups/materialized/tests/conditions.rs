use super::*;
use std::cell::Cell;

#[test]
fn condition_record_wrapper_matches_original_all_actions_counters_and_loader_order() {
    let groups = [group(531, SpawnGroupFlags::NONE), group(532, SpawnGroupFlags::MANUAL_SPAWN),
        group(533, SpawnGroupFlags::NONE), group(534, SpawnGroupFlags::DESPAWN_ON_CONDITION_FAILURE)];
    let store = store(&groups, vec![(531, spawn(SpawnObjectType::Creature, 131)),
        (531, spawn(SpawnObjectType::Creature, 132)), (534, spawn(SpawnObjectType::Creature, 134))]);
    let mut old = map(true);
    let mut current = map(true);
    for shared in [&mut old, &mut current] {
        shared.insert_map_object_record(record(134, 571)).unwrap();
        timer(shared, 134);
    }
    let old_plans = Cell::new(0);
    let mut old_calls = Vec::new();
    let expected = old.original_conditions(groups.iter(), &store,
        |group| { old_plans.set(old_plans.get() + 1); group.group_id == 531 },
        |_, kind, id, force| {
            assert_eq!(old_plans.get(), 4);
            old_calls.push((kind, id, force));
            (id == 132).then(|| LoadedGridRespawnRecordsLikeCpp::primary_only(record(id, 571)))
        });
    let plans = Cell::new(0);
    let mut calls = Vec::new();
    let actual = current.apply_update_spawn_group_conditions_loaded_grid_records_like_cpp(groups.iter(), &store,
        |group| { plans.set(plans.get() + 1); group.group_id == 531 },
        |_, kind, id, force| {
            assert_eq!(plans.get(), 4);
            calls.push((kind, id, force));
            (id == 132).then(|| LoadedGridRespawnRecordsLikeCpp::primary_only(record(id, 571)))
        });
    assert_eq!(plans.get(), old_plans.get());
    assert_eq!(calls, old_calls);
    assert_eq!(actual.iter().map(|outcome| outcome.group_id).collect::<Vec<_>>(), vec![531, 532, 533, 534]);
    assert_eq!(actual.len(), expected.len());
    for (actual, expected) in actual.into_iter().zip(expected) {
        assert_eq!(actual.action, expected.action);
        assert_eq!(actual.applied_change, expected.applied_change);
        assert_eq!(actual.despawn_outcome, expected.despawn_outcome);
        match (actual.spawn_outcome, expected.spawn_outcome) {
            (Some(actual), Some(expected)) => assert_record_summary(actual, expected),
            (None, None) => {}
            _ => panic!("same spawn branch"),
        }
    }
    assert_eq!(current.map_object_count(), old.map_object_count());
    assert_eq!(current.get_respawn_time_like_cpp(SpawnObjectType::Creature, 134), 0);
    assert_eq!(current.get_respawn_time_like_cpp(SpawnObjectType::Creature, 134), old.get_respawn_time_like_cpp(SpawnObjectType::Creature, 134));
}

#[test]
fn owned_conditions_plan_once_forward_one_loader_and_keep_ordered_rejection_and_admission() {
    let groups = [group(535, SpawnGroupFlags::NONE), group(536, SpawnGroupFlags::NONE)];
    let store = store(&groups, vec![(535, spawn(SpawnObjectType::Creature, 135)),
        (536, spawn(SpawnObjectType::Creature, 136))]);
    let mut shared = map(true);
    shared.set_spawn_group_inactive_like_cpp(Some(&groups[0]));
    shared.set_spawn_group_inactive_like_cpp(Some(&groups[1]));
    let planned = Cell::new(0);
    let mut calls = Vec::new();
    let mut rejected = Some(LoadedGridRespawnRecordsLikeCpp {
        pre_add_records: vec![player(3135, 571)], primary_record: record(135, 571),
    });
    let pointer = rejected.as_ref().unwrap().primary_record.creature().unwrap() as *const Creature;
    let outcomes = shared.update_spawn_group_conditions_materialized(groups.iter(), &store,
        |_| { planned.set(planned.get() + 1); true },
        |_, kind, id, force| {
            assert_eq!(planned.get(), 2);
            calls.push((kind, id, force));
            if id == 135 { Err(rejected.take().unwrap()) }
            else { Ok(Some(LoadedGridMaterialization::records(LoadedGridRespawnRecordsLikeCpp {
                pre_add_records: vec![player(3136, 571)], primary_record: record(136, 530),
            }))) }
        });
    assert_eq!(planned.get(), 2);
    assert_eq!(calls, vec![(SpawnObjectType::Creature, 135, false), (SpawnObjectType::Creature, 136, false)]);
    assert_eq!(outcomes.iter().map(|outcome| outcome.group_id).collect::<Vec<_>>(), vec![535, 536]);
    let mut outcomes = outcomes.into_iter();
    let first = outcomes.next().unwrap().spawn_outcome.unwrap();
    assert_eq!(first.summary.blocked_loaded_grid_spawn_loads, 1);
    assert!(first.summary.loaded_grid_primary_records.is_empty());
    let LoadedGridSpawnAttemptResult::PreparationRejected(records) = first.attempts.into_iter().next().unwrap().result else { panic!("full rejection"); };
    assert_eq!(records.primary_record.creature().unwrap() as *const Creature, pointer);
    assert_eq!(records.pre_add_records.len(), 1);
    let second = outcomes.next().unwrap().spawn_outcome.unwrap();
    assert_eq!(second.summary.blocked_loaded_grid_spawn_add_to_map, 1);
    assert!(second.summary.loaded_grid_primary_records.is_empty());
    let LoadedGridSpawnAttemptResult::Admitted(admission) = second.attempts.into_iter().next().unwrap().result else { panic!("admission"); };
    let (facets, primary) = admission.into_parts();
    assert!(facets[0].is_ok());
    assert!(matches!(primary, LoadedGridPrimaryAdmission::Record { result: Err(_), .. }));
    // Rejected preparation does not add its facet; a later primary failure
    // keeps that later facet and both groups' already-applied active changes.
    assert_eq!(shared.map_reference_order_like_cpp(), &[ObjectGuid::create_player(1, 3136)]);
    for group in &groups { assert!(shared.is_spawn_group_active_like_cpp(Some(group))); }
}
