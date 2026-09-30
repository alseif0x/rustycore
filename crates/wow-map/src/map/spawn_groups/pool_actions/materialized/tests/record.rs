use super::*;
use std::cell::RefCell;

#[derive(Debug, PartialEq)]
enum Event {
    Roll(PoolMemberKindLikeCpp, u32),
    Equal(Vec<u64>, usize),
    Load(SpawnObjectType, SpawnId, usize),
}

fn payload(id: SpawnId) -> Option<LoadedGridRespawnRecordsLikeCpp> {
    let mut primary = record(id, 571);
    primary.creature_mut().unwrap().unit_mut().subsystems_mut().auras.register_applied_aura(
        AppliedAuraRef::new(47_510, ObjectGuid::create_player(1, 99), 1, 1),
        None, SPELL_AURA_INTERRUPT_FLAG_ENTER_WORLD_LIKE_CPP, 0,
    );
    if id == 102 { primary.object_mut().object_mut().add_to_world(); }
    Some(LoadedGridRespawnRecordsLikeCpp {
        pre_add_records: vec![player(id as i64 + 1000, 571), player(id as i64 + 2000, 530)],
        primary_record: primary,
    })
}

#[test]
fn record_facade_matches_full_oracle_roll_pick_load_counters_and_snapshot_timing() {
    for (id, chance) in [(101, 100.0), (102, 0.0)] {
        let mut manager = PoolMgrLikeCpp::new();
        add_group(&mut manager, 1, PoolMemberKindLikeCpp::Creature, &[(id, chance)], 1);
        let store = store(&[id]);
        let mut old = map(true);
        let mut current = map(true);
        if id == 102 {
            old.insert_map_object_record(record(id, 571)).unwrap();
            current.insert_map_object_record(record(id, 571)).unwrap();
        }
        let previous = RefCell::new(Vec::new());
        let expected = old.original_facade(&manager, 1, &store,
            |kind, pool| { previous.borrow_mut().push(Event::Roll(kind, pool)); 0.0 },
            |candidates, count| {
                previous.borrow_mut().push(Event::Equal(candidates.iter().map(|candidate| candidate.guid).collect(), count));
                (0..count).collect()
            },
            |map, kind, id| { previous.borrow_mut().push(Event::Load(kind, id, map.map_object_count())); payload(id) },
        ).unwrap();
        let events = RefCell::new(Vec::new());
        let actual = current.spawn_pool_loaded_grid_records_like_cpp(&manager, 1, &store,
            |kind, pool| { events.borrow_mut().push(Event::Roll(kind, pool)); 0.0 },
            |candidates, count| {
                events.borrow_mut().push(Event::Equal(candidates.iter().map(|candidate| candidate.guid).collect(), count));
                (0..count).collect()
            },
            |map, kind, id| { events.borrow_mut().push(Event::Load(kind, id, map.map_object_count())); payload(id) },
        ).unwrap();
        assert_eq!(events, previous);
        assert_eq!(actual.executed_loaded_grid_respawns, 1); // Record duplicate still executed
        let snapshot = &actual.loaded_grid_primary_records[0];
        assert_eq!(snapshot.object().object().is_in_world(), id == 102);
        let aura = AppliedAuraRef::new(47_510, ObjectGuid::create_player(1, 99), 1, 1);
        assert!(snapshot.creature().unwrap().unit().subsystems().auras.has_applied(aura));
        if id == 101 { assert!(!current.get_typed_creature(snapshot.object().guid()).unwrap().unit().subsystems().auras.has_applied(aura)); }
        assert_eq!(current.pool_data_like_cpp(), old.pool_data_like_cpp());
        assert_eq!(current.map_reference_order_like_cpp(), old.map_reference_order_like_cpp());
        assert_eq!(current.map_object_count(), old.map_object_count());
        assert_summary(actual, expected);
    }
}

#[test]
fn actual_cycle_and_overflow_preserve_original_error_and_partial_pool_data_without_admission() {
    for child in [1, u64::from(u32::MAX) + 1] {
        let mut manager = PoolMgrLikeCpp::new();
        add_group(&mut manager, 1, PoolMemberKindLikeCpp::Pool, &[(child, 0.0)], 1);
        let mut old = map(true);
        let mut current = map(true);
        let store = SpawnStore::new();
        let expected = old.original_facade(&manager, 1, &store, |_, _| 0.0, |_, count| (0..count).collect(),
            |_, _, _| panic!("planner failed before admission")).unwrap_err();
        let actual = current.spawn_pool_materialized(&manager, 1, &store, |_, _| 0.0, |_, count| (0..count).collect(),
            |_, _, _| panic!("planner failed before materialization")).unwrap_err();
        assert_eq!(actual, expected);
        if child == 1 {
            assert_eq!(actual, PoolMgrPlanErrorLikeCpp::ChildPoolCycle { pool_id: 1 });
        } else {
            assert_eq!(actual, PoolMgrPlanErrorLikeCpp::ChildPoolIdOverflow { child_pool_id: child });
        }
        assert_eq!(current.pool_data_like_cpp(), old.pool_data_like_cpp());
        assert_eq!(current.map_object_count(), 0);
    }
    // MissingGroup is a typed skipped specialization, not a planning error.
    let manager = PoolMgrLikeCpp::new();
    let mut shared = map(true);
    let outcome = shared.spawn_pool_materialized(&manager, 999, &SpawnStore::new(), |_, _| panic!("no group"),
        |_, _| panic!("no group"), |_, _, _| panic!("no loader call")).unwrap();
    assert!(outcome.attempts.is_empty());
    assert_eq!(outcome.summary, ProcessRespawnsSafeSideEffectsSummaryLikeCpp::default());
}
