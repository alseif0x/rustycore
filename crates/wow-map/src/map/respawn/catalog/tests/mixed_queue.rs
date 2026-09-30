use super::*;

#[test]
fn catalog_failures_never_consume_actor_slots_even_when_their_info_is_due_first() {
    for consume in [false, true] {
        let mut shared = map(true);
        saved_actor_timer(&mut shared, 601, 20);
        let store = store(&[(SpawnObjectType::Creature, 602), (SpawnObjectType::Creature, 603)]);
        timer(&mut shared, SpawnObjectType::Creature, 602, 90);
        timer(&mut shared, SpawnObjectType::Creature, 603, 100);
        let actor_time = shared.get_respawn_time_like_cpp(SpawnObjectType::Creature, 601);
        let mut calls = Vec::new();
        let outcome = run_owned(&mut shared, &store, &PoolMgrLikeCpp::new(), consume, |map, kind, id| {
            assert!(map.get_respawn_info_like_cpp(kind, id).is_some());
            calls.push(id);
            Ok(None)
        });
        assert_eq!(calls, if consume { vec![602, 603] } else { vec![602] });
        assert_eq!(outcome.summary.blocked_loaded_grid_respawn_loads, if consume { 2 } else { 1 });
        assert_eq!(shared.respawn_store.actor_queue_len(), 1);
        assert_eq!(shared.get_respawn_time_like_cpp(SpawnObjectType::Creature, 601), actor_time);
        assert_eq!(outcome.attempts.len(), calls.len());
        for attempt in outcome.attempts {
            assert_ne!(catalog_plan(&attempt.plan).1, 601);
            assert!(matches!(attempt.result, LoadedGridSpawnAttemptResult::Unavailable));
        }
    }
}
