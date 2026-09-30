use super::*;
use rand::rngs::StdRng;

fn actor(id: SpawnId, map_id: u32, point: bool) -> (WorldCreature, StdRng) {
    let creature = creature(id, map_id);
    let data = WorldCreature::create_data_from_canonical_like_cpp(&creature);
    let mut actor = WorldCreature::from_canonical(creature, data);
    actor.create_data.npc_flags = 0xB00D;
    let rng = actor.seed_actor_storage_runtime(point);
    (actor, rng)
}

#[test]
fn actor_success_collisions_and_reject_preserve_incoming_motor_facets_and_same_existing_owner() {
    for mode in 0..5 {
        let id = 501 + mode;
        let mut spawn = metadata(SpawnObjectType::Creature, id);
        spawn.spawn_group.flags |= SpawnGroupFlags::ESCORTQUESTNPC;
        let mut store = SpawnStore::new();
        store.add_object_spawn(&spawn, |_| false);
        let mut shared = map(true);
        let guid = guid(HighGuid::Creature, id);
        let mut stored_rng = None;
        let mut pointer = None;
        if mode == 1 || mode == 4 {
            shared.insert_map_object_record(record(SpawnObjectType::Creature, id, 571)).unwrap();
            pointer = Some(shared.get_typed_creature(guid).unwrap() as *const Creature);
        } else if mode == 2 {
            let (stored, rng) = actor(id, 571, true);
            shared.admit_fresh_creature_actor(stored).unwrap();
            stored_rng = Some(rng);
            pointer = Some(shared.get_typed_creature(guid).unwrap() as *const Creature);
        }
        timer(&mut shared, SpawnObjectType::Creature, id, 100);
        let point = mode != 1;
        let (incoming, mut rng) = actor(id, if mode == 3 { 530 } else { 571 }, point);
        let mut incoming = Some(incoming);
        let mut escort_calls = 0;
        let outcome = shared.process_due_respawns_materialized(100, &store, &LinkedRespawnStoreLikeCpp::new(), &PoolMgrLikeCpp::new(),
            5, true, |_, _| { escort_calls += 1; true }, |_, _| 0.0, |_, count| (0..count).collect(), false,
            |map, kind, id| {
                assert_eq!(map.get_respawn_time_like_cpp(kind, id), 100);
                let facets = vec![player(id as i64 + 1000, 571), player(id as i64 + 2000, 530)];
                Ok(Some(if mode == 4 {
                    LoadedGridMaterialization::records(LoadedGridRespawnRecordsLikeCpp { pre_add_records: facets, primary_record: record(kind, id, 571) })
                } else {
                    let mut incoming = incoming.take().unwrap();
                    incoming.creature.unit_mut().set_health(61);
                    LoadedGridMaterialization::creature(facets, incoming)
                }))
            });
        assert_eq!(escort_calls, usize::from(mode == 1 || mode == 2 || mode == 4));
        assert_eq!(outcome.summary.executed_loaded_grid_respawns, usize::from(mode == 0 || mode == 4));
        assert_eq!(outcome.summary.blocked_loaded_grid_respawn_add_to_map, usize::from(mode != 0 && mode != 4));
        assert!(outcome.summary.loaded_grid_primary_records.is_empty());
        assert_eq!(shared.get_respawn_time_like_cpp(SpawnObjectType::Creature, id), 0);
        assert_eq!(shared.map_reference_order_like_cpp(), &[ObjectGuid::create_player(1, id as i64 + 1000)]);
        let attempt = outcome.attempts.into_iter().next().unwrap();
        assert_eq!(catalog_plan(&attempt.plan), (SpawnObjectType::Creature, id));
        let LoadedGridSpawnAttemptResult::Admitted(admission) = attempt.result else { panic!("admission"); };
        let (facets, primary) = admission.into_parts();
        assert!(facets[0].is_ok());
        assert!(facets[1].is_err());
        match primary {
            LoadedGridPrimaryAdmission::Record { result, .. } => { assert_eq!(mode, 4); assert!(result.is_ok()); }
            LoadedGridPrimaryAdmission::CreatureActor(result) => match result {
                Ok(FreshCreatureActorAdmission::Inserted { .. }) => {
                    assert_eq!(mode, 0);
                    let stored = shared.creature_actor_mut(guid).unwrap();
                    assert_eq!(stored.creature.current_health(), 61);
                    assert_eq!(stored.create_data.npc_flags, 0xB00D);
                    stored.assert_actor_storage_runtime(&mut rng, point);
                }
                Ok(FreshCreatureActorAdmission::ExistingRecord { mut incoming }) => {
                    assert_eq!(mode, 1);
                    assert_eq!(incoming.creature.current_health(), 61);
                    assert_eq!(incoming.create_data.npc_flags, 0xB00D);
                    incoming.assert_actor_storage_runtime(&mut rng, point);
                }
                Ok(FreshCreatureActorAdmission::ExistingActor { mut incoming }) => {
                    assert_eq!(mode, 2);
                    incoming.assert_actor_storage_runtime(&mut rng, point);
                    shared.creature_actor_mut(guid).unwrap().assert_actor_storage_runtime(stored_rng.as_mut().unwrap(), true);
                }
                Err((error, mut incoming)) => {
                    assert_eq!(mode, 3);
                    assert!(matches!(error, FreshCreatureActorAdmissionError::Store(_)));
                    assert_eq!(incoming.create_data.npc_flags, 0xB00D);
                    incoming.assert_actor_storage_runtime(&mut rng, point);
                    assert!(shared.get_typed_creature(guid).is_none());
                }
            },
        }
        if mode == 1 || mode == 2 {
            assert_eq!(shared.get_typed_creature(guid).unwrap() as *const Creature, pointer.unwrap());
            assert_eq!(shared.get_typed_creature(guid).unwrap().current_health(), 75);
        }
    }
}
