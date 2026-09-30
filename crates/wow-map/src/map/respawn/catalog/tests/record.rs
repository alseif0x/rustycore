use super::*;

fn payload(kind: SpawnObjectType, id: SpawnId, valid: bool) -> LoadedGridRespawnRecordsLikeCpp {
    let mut primary = record(kind, id, if valid { 571 } else { 530 });
    if let Some(creature) = primary.creature_mut() {
        creature
            .unit_mut()
            .subsystems_mut()
            .auras
            .register_applied_aura(
                aura(),
                None,
                SPELL_AURA_INTERRUPT_FLAG_ENTER_WORLD_LIKE_CPP,
                0,
            );
    }
    LoadedGridRespawnRecordsLikeCpp {
        pre_add_records: vec![player(id as i64 + 1000, 571), player(id as i64 + 2000, 530)],
        primary_record: primary,
    }
}

#[test]
fn record_facade_matches_whole_oracle_summary_queue_load_order_and_snapshot_timing() {
    for consume in [false, true] {
        for mode in 0..3 {
            let store = store(&[
                (SpawnObjectType::Creature, 101),
                (SpawnObjectType::GameObject, 102),
            ]);
            let mut old = map(true);
            let mut shared = map(true);
            for map in [&mut old, &mut shared] {
                timer(map, SpawnObjectType::Creature, 101, 90);
                timer(map, SpawnObjectType::GameObject, 102, 100);
            }
            let mut previous = Vec::new();
            let expected = run_original(
                &mut old,
                &store,
                &PoolMgrLikeCpp::new(),
                consume,
                |map, kind, id| {
                    assert!(map.get_respawn_info_like_cpp(kind, id).is_some());
                    previous.push((kind, id));
                    if mode == 0 {
                        None
                    } else {
                        Some(payload(kind, id, mode == 1))
                    }
                },
            );
            let mut calls = Vec::new();
            let actual = run_record(
                &mut shared,
                &store,
                &PoolMgrLikeCpp::new(),
                consume,
                |map, kind, id| {
                    assert!(map.get_respawn_info_like_cpp(kind, id).is_some());
                    calls.push((kind, id));
                    if mode == 0 {
                        None
                    } else {
                        Some(payload(kind, id, mode == 1))
                    }
                },
            );
            assert_eq!(calls, previous);
            assert_eq!(
                shared.respawn_timer_keys_like_cpp().collect::<Vec<_>>(),
                old.respawn_timer_keys_like_cpp().collect::<Vec<_>>()
            );
            assert_eq!(shared.map_object_count(), old.map_object_count());
            assert_eq!(
                shared.map_reference_order_like_cpp(),
                old.map_reference_order_like_cpp()
            );
            if mode == 1 {
                assert_eq!(actual.executed_loaded_grid_respawns, 2);
                let snapshot = actual.loaded_grid_primary_records[0].creature().unwrap();
                assert!(snapshot.unit().subsystems().auras.has_applied(aura()));
                assert!(
                    !shared
                        .get_typed_creature(snapshot.guid())
                        .unwrap()
                        .unit()
                        .subsystems()
                        .auras
                        .has_applied(aura())
                );
            }
            if mode == 2 {
                assert_eq!(actual.blocked_loaded_grid_respawn_add_to_map, 2);
                assert!(shared.respawn_timer_keys_like_cpp().next().is_none());
                assert_eq!(shared.map_reference_order_like_cpp().len(), 2); // successful facets remain
            }
            assert_summary(actual, expected);
        }
    }
}

#[test]
fn owned_record_receipts_keep_best_effort_facets_and_full_failed_snapshot_without_rollback() {
    for valid in [false, true] {
        let store = store(&[(SpawnObjectType::Creature, 111)]);
        let mut shared = map(true);
        timer(&mut shared, SpawnObjectType::Creature, 111, 100);
        let outcome = run_owned(
            &mut shared,
            &store,
            &PoolMgrLikeCpp::new(),
            false,
            |map, kind, id| {
                assert_eq!(map.get_respawn_time_like_cpp(kind, id), 100);
                Ok(Some(LoadedGridMaterialization::records(payload(
                    kind, id, valid,
                ))))
            },
        );
        assert_eq!(
            outcome.summary.executed_loaded_grid_respawns,
            usize::from(valid)
        );
        assert_eq!(
            outcome.summary.blocked_loaded_grid_respawn_add_to_map,
            usize::from(!valid)
        );
        assert!(outcome.summary.loaded_grid_primary_records.is_empty());
        assert_eq!(
            shared.get_respawn_time_like_cpp(SpawnObjectType::Creature, 111),
            0
        );
        assert_eq!(
            shared.map_reference_order_like_cpp(),
            &[ObjectGuid::create_player(1, 1111)]
        );
        let attempt = outcome.attempts.into_iter().next().unwrap();
        assert_eq!(
            catalog_plan(&attempt.plan),
            (SpawnObjectType::Creature, 111)
        );
        let LoadedGridSpawnAttemptResult::Admitted(admission) = attempt.result else {
            panic!("admitted");
        };
        let (facets, primary) = admission.into_parts();
        assert!(facets[0].is_ok());
        assert!(facets[1].is_err());
        let LoadedGridPrimaryAdmission::Record { snapshot, result } = primary else {
            panic!("Record");
        };
        assert_eq!(result.is_ok(), valid);
        assert_eq!(snapshot.creature().unwrap().current_health(), 75);
        assert!(
            snapshot
                .creature()
                .unwrap()
                .unit()
                .subsystems()
                .auras
                .has_applied(aura())
        );
    }
}
