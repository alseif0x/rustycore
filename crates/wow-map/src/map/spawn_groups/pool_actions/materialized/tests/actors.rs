use super::*;
use rand::rngs::StdRng;

fn actor(id: SpawnId, point: bool, map_id: u32) -> (WorldCreature, StdRng) {
    let creature = creature(id, map_id);
    let data = WorldCreature::create_data_from_canonical_like_cpp(&creature);
    let mut actor = WorldCreature::from_canonical(creature, data);
    actor.create_data.npc_flags = 0xB00D;
    let rng = actor.seed_actor_storage_runtime(point);
    (actor, rng)
}

#[test]
fn mixed_actor_and_record_receipts_preserve_full_incoming_motors_facets_and_failure_effects() {
    // Fresh Actor, Record duplicate, Actor duplicate, Actor rejection, Record rejection.
    for mode in 0..5 {
        let id = 401 + mode;
        let store = store(&[id]);
        let plan = typed(vec![spawn_action(id)]);
        let mut shared = map(true);
        let mut stored_rng = None;
        let mut stored_pointer = None;
        if mode == 1 {
            let record = record(id, 571);
            let guid = record.object().guid();
            shared.insert_map_object_record(record).unwrap();
            stored_pointer = Some(shared.get_typed_creature(guid).unwrap() as *const Creature);
        } else if mode == 2 {
            let (stored, rng) = actor(id, true, 571);
            let guid = stored.guid();
            assert!(matches!(
                shared.admit_fresh_creature_actor(stored),
                Ok(FreshCreatureActorAdmission::Inserted { .. })
            ));
            stored_rng = Some(rng);
            stored_pointer = Some(shared.get_typed_creature(guid).unwrap() as *const Creature);
        }
        let point = mode != 1;
        let (mut incoming, mut rng) = if mode == 4 {
            (None, None)
        } else {
            let (mut incoming, rng) = actor(id, point, if mode == 3 { 530 } else { 571 });
            incoming.creature.unit_mut().set_health(61);
            (Some(incoming), Some(rng))
        };
        let outcome = owned_typed(
            &mut shared,
            &plan,
            &store,
            Some(&mut |_: &mut Map, _, _| {
                let facets = vec![player(id as i64 + 1000, 571), player(id as i64 + 2000, 530)];
                Ok(Some(if mode == 4 {
                    LoadedGridMaterialization::records(LoadedGridRespawnRecordsLikeCpp {
                        pre_add_records: facets,
                        primary_record: record(id, 530),
                    })
                } else {
                    LoadedGridMaterialization::creature(facets, incoming.take().unwrap())
                }))
            }),
        );
        assert_eq!(
            outcome.summary.executed_loaded_grid_respawns,
            usize::from(mode == 0)
        );
        assert_eq!(
            outcome.summary.blocked_loaded_grid_respawn_add_to_map,
            usize::from(mode != 0)
        );
        assert_eq!(outcome.summary.pool_spawn_actions_blocked_loaded_grid, 0);
        assert!(outcome.summary.loaded_grid_primary_records.is_empty());
        let attempt = outcome.attempts.into_iter().next().unwrap();
        assert_eq!(pool_plan(&attempt.plan).spawn_id, id);
        assert!(!pool_plan(&attempt.plan).respawn);
        let LoadedGridSpawnAttemptResult::Admitted(admission) = attempt.result else {
            panic!("admission");
        };
        let (facets, primary) = admission.into_parts();
        assert_eq!(facets.len(), 2);
        assert!(facets[0].is_ok());
        assert!(facets[1].is_err());
        assert_eq!(
            shared.map_reference_order_like_cpp(),
            &[ObjectGuid::create_player(1, id as i64 + 1000)]
        );
        let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 7, 42, id as i64);
        match primary {
            LoadedGridPrimaryAdmission::Record { snapshot, result } => {
                assert_eq!(mode, 4);
                assert!(result.is_err());
                assert_eq!(snapshot.object().map_id(), 530);
                assert_eq!(snapshot.creature().unwrap().current_health(), 75);
                assert!(shared.get_typed_creature(guid).is_none());
            }
            LoadedGridPrimaryAdmission::CreatureActor(result) => match result {
                Ok(FreshCreatureActorAdmission::Inserted { .. }) => {
                    assert_eq!(mode, 0);
                    let stored = shared.creature_actor_mut(guid).unwrap();
                    assert_eq!(stored.create_data.npc_flags, 0xB00D);
                    assert_eq!(stored.creature.current_health(), 61);
                    stored.assert_actor_storage_runtime(rng.as_mut().unwrap(), point);
                }
                Ok(FreshCreatureActorAdmission::ExistingRecord { mut incoming }) => {
                    assert_eq!(mode, 1);
                    assert_eq!(incoming.creature.current_health(), 61);
                    assert_eq!(incoming.create_data.npc_flags, 0xB00D);
                    incoming.assert_actor_storage_runtime(rng.as_mut().unwrap(), point);
                }
                Ok(FreshCreatureActorAdmission::ExistingActor { mut incoming }) => {
                    assert_eq!(mode, 2);
                    assert_eq!(incoming.creature.current_health(), 61);
                    assert_eq!(incoming.create_data.npc_flags, 0xB00D);
                    incoming.assert_actor_storage_runtime(rng.as_mut().unwrap(), point);
                    shared
                        .creature_actor_mut(guid)
                        .unwrap()
                        .assert_actor_storage_runtime(stored_rng.as_mut().unwrap(), true);
                }
                Err((error, mut incoming)) => {
                    assert_eq!(mode, 3);
                    assert!(matches!(error, FreshCreatureActorAdmissionError::Store(_)));
                    assert_eq!(incoming.creature.current_health(), 61);
                    assert_eq!(incoming.create_data.npc_flags, 0xB00D);
                    incoming.assert_actor_storage_runtime(rng.as_mut().unwrap(), point);
                    assert!(shared.get_typed_creature(guid).is_none());
                }
            },
        }
        if let Some(pointer) = stored_pointer {
            assert_eq!(
                shared.get_typed_creature(guid).unwrap() as *const Creature,
                pointer
            );
            assert_eq!(
                shared.get_typed_creature(guid).unwrap().current_health(),
                75
            );
        }
        assert_eq!(shared.map_object_count(), if mode <= 2 { 2 } else { 1 });
    }
}
