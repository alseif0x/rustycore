use super::*;

#[test]
fn unavailable_and_full_preparation_rejection_follow_both_queue_policies_without_losing_payload() {
    for consume in [false, true] {
        for rejected in [false, true] {
            let store = store(&[(SpawnObjectType::Creature, 201), (SpawnObjectType::Creature, 202)]);
            let mut shared = map(true);
            timer(&mut shared, SpawnObjectType::Creature, 201, 90);
            timer(&mut shared, SpawnObjectType::Creature, 202, 100);
            let mut records = Some(LoadedGridRespawnRecordsLikeCpp {
                pre_add_records: vec![player(2201, 571), player(2202, 530)],
                primary_record: record(SpawnObjectType::Creature, 201, 571),
            });
            let creature = records.as_ref().unwrap().primary_record.creature().unwrap();
            let pointer = creature as *const Creature;
            let loot = creature.loot_authority_like_cpp().clone();
            let timeline = creature.unit().health_state_revision_authority_like_cpp();
            let facet = records.as_ref().unwrap().pre_add_records[0].player().unwrap() as *const Player;
            let mut calls = Vec::new();
            let outcome = run_owned(&mut shared, &store, &PoolMgrLikeCpp::new(), consume, |map, kind, id| {
                assert!(map.get_respawn_info_like_cpp(kind, id).is_some());
                calls.push(id);
                if id == 201 {
                    if rejected { Err(records.take().unwrap()) } else { Ok(None) }
                } else {
                    Ok(Some(LoadedGridMaterialization::records(LoadedGridRespawnRecordsLikeCpp::primary_only(record(kind, id, 571)))))
                }
            });
            assert_eq!(calls, if consume { vec![201, 202] } else { vec![201] });
            assert_eq!(outcome.summary.blocked_loaded_grid_respawn_loads, 1);
            assert_eq!(outcome.summary.blocked_do_respawn_runtime, 1);
            assert_eq!(outcome.summary.executed_loaded_grid_respawns, usize::from(consume));
            assert_eq!(shared.get_respawn_info_like_cpp(SpawnObjectType::Creature, 201).is_none(), consume);
            assert_eq!(shared.get_respawn_info_like_cpp(SpawnObjectType::Creature, 202).is_none(), consume);
            let attempt = outcome.attempts.into_iter().next().unwrap();
            assert_eq!(catalog_plan(&attempt.plan), (SpawnObjectType::Creature, 201));
            match attempt.result {
                LoadedGridSpawnAttemptResult::PreparationRejected(records) => {
                    assert!(rejected);
                    assert_eq!(records.primary_record.creature().unwrap() as *const Creature, pointer);
                    assert_eq!(records.pre_add_records.len(), 2);
                    assert_eq!(records.pre_add_records[0].player().unwrap() as *const Player, facet);
                    let creature = records.primary_record.creature().unwrap();
                    assert!(creature.loot_authority_like_cpp().shares_storage_like_cpp(&loot));
                    assert!(creature.unit().shares_health_state_revision_authority_like_cpp(&timeline));
                }
                LoadedGridSpawnAttemptResult::Unavailable => assert!(!rejected),
                _ => panic!("Catalog load failure never becomes NoLoader/admission"),
            }
            assert!(shared.map_reference_order_like_cpp().is_empty());
        }
    }
}

#[test]
fn missing_metadata_consumes_or_blocks_without_fabricating_a_load_receipt() {
    for consume in [false, true] {
        let store = store(&[(SpawnObjectType::Creature, 212)]);
        let mut shared = map(true);
        timer(&mut shared, SpawnObjectType::Creature, 211, 90);
        timer(&mut shared, SpawnObjectType::Creature, 212, 100);
        let mut calls = Vec::new();
        let outcome = run_owned(&mut shared, &store, &PoolMgrLikeCpp::new(), consume, |_, kind, id| {
            calls.push(id);
            Ok(Some(LoadedGridMaterialization::records(LoadedGridRespawnRecordsLikeCpp::primary_only(record(kind, id, 571)))))
        });
        assert_eq!(outcome.summary.blocked_missing_spawn_data, 1);
        assert_eq!(calls, if consume { vec![212] } else { vec![] });
        assert_eq!(outcome.attempts.len(), usize::from(consume));
        assert_eq!(shared.get_respawn_info_like_cpp(SpawnObjectType::Creature, 211).is_none(), consume);
    }
}
