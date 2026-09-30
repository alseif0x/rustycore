use super::*;

fn payload(id: SpawnId) -> Option<LoadedGridRespawnRecordsLikeCpp> {
    if id == 101 { return None; }
    let mut primary = record(id, if id == 102 { 530 } else { 571 });
    primary.creature_mut().unwrap().unit_mut().subsystems_mut().auras.register_applied_aura(
        aura(), None, SPELL_AURA_INTERRUPT_FLAG_ENTER_WORLD_LIKE_CPP, 0,
    );
    if id == 104 { primary.object_mut().object_mut().add_to_world(); }
    Some(LoadedGridRespawnRecordsLikeCpp {
        pre_add_records: vec![player(id as i64 + 1000, 571), player(id as i64 + 2000, 530)],
        primary_record: primary,
    })
}

#[test]
fn record_wrapper_matches_frozen_whole_summary_calls_and_snapshot_timing() {
    let group = group(501, SpawnGroupFlags::NONE);
    let store = store(&[group.clone()], (101..=104).map(|id| (501, spawn(SpawnObjectType::Creature, id))).collect());
    let mut original = map(true);
    let mut current = map(true);
    for shared in [&mut original, &mut current] {
        timer(shared, 103);
        shared.insert_map_object_record(record(104, 571)).unwrap();
    }
    let mut old_calls = Vec::new();
    let old = original.original_group(Some(&group), false, true, &store, |map, kind, id, force| {
        old_calls.push((kind, id, force, map.get_respawn_time_like_cpp(kind, id), map.map_object_count()));
        payload(id)
    });
    let mut calls = Vec::new();
    let actual = current.spawn_group_spawn_loaded_grid_records_like_cpp(Some(&group), false, true, &store,
        |map, kind, id, force| {
            calls.push((kind, id, force, map.get_respawn_time_like_cpp(kind, id), map.map_object_count()));
            payload(id)
        });
    assert_eq!(calls, old_calls);
    assert_eq!(actual.executed_loaded_grid_spawns, 2);
    assert_eq!(actual.blocked_loaded_grid_spawn_loads, 1);
    assert_eq!(actual.blocked_loaded_grid_spawn_add_to_map, 1);
    let fresh = actual.loaded_grid_primary_records.iter().find(|record| record.creature().unwrap().spawn_id() == 103).unwrap();
    assert!(fresh.creature().unwrap().unit().subsystems().auras.has_applied(aura()));
    assert!(!fresh.object().object().is_in_world());
    assert!(!current.get_typed_creature(fresh.object().guid()).unwrap().unit().subsystems().auras.has_applied(aura()));
    let duplicate = actual.loaded_grid_primary_records.iter().find(|record| record.creature().unwrap().spawn_id() == 104).unwrap();
    assert!(duplicate.object().object().is_in_world());
    assert_eq!(current.map_reference_order_like_cpp(), original.map_reference_order_like_cpp());
    assert_eq!(current.map_object_count(), original.map_object_count());
    assert_record_summary(actual, old);
}

#[test]
fn owned_record_success_and_add_failure_keep_complete_receipts_without_summary_records() {
    let group = group(502, SpawnGroupFlags::NONE);
    let store = store(&[group.clone()], vec![(502, spawn(SpawnObjectType::Creature, 102)), (502, spawn(SpawnObjectType::Creature, 103))]);
    let mut shared = map(true);
    let outcome = shared.spawn_group_spawn_materialized(Some(&group), false, false, &store,
        |_, _, id, _| Ok(payload(id).map(LoadedGridMaterialization::records)));
    assert_eq!(outcome.summary.executed_loaded_grid_spawns, 1);
    assert_eq!(outcome.summary.blocked_loaded_grid_spawn_add_to_map, 1);
    assert!(outcome.summary.loaded_grid_primary_records.is_empty());
    assert_eq!(outcome.attempts.len(), 2);
    for attempt in outcome.attempts {
        assert!(!group_plan(&attempt.plan).force);
        let LoadedGridSpawnAttemptResult::Admitted(admission) = attempt.result else { panic!("admission"); };
        let (pre_add, primary) = admission.into_parts();
        assert_eq!(pre_add.len(), 2);
        assert!(pre_add[0].is_ok());
        assert!(pre_add[1].is_err());
        let LoadedGridPrimaryAdmission::Record { snapshot, result } = primary else { panic!("Record"); };
        assert!(snapshot.creature().unwrap().unit().subsystems().auras.has_applied(aura()));
        assert_eq!(result.is_ok(), group_plan(&attempt.plan).spawn_id == 103);
    }
    assert_eq!(shared.map_object_count(), 3); // two facets, one accepted primary
}
