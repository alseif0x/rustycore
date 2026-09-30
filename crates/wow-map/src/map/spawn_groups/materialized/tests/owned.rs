use super::*;
use rand::rngs::StdRng;

fn actor(id: SpawnId, point: bool, map_id: u32) -> (WorldCreature, StdRng) {
    let creature = creature(id, map_id);
    let data = WorldCreature::create_data_from_canonical_like_cpp(&creature);
    let mut actor = WorldCreature::from_canonical(creature, data);
    actor.create_data.npc_flags = 0x9876;
    let rng = actor.seed_actor_storage_runtime(point);
    (actor, rng)
}

#[test]
fn unavailable_and_preparation_rejected_are_distinct_and_rejection_moves_every_payload() {
    let group = group(520, SpawnGroupFlags::NONE);
    let store = store(
        &[group.clone()],
        vec![
            (520, spawn(SpawnObjectType::Creature, 121)),
            (520, spawn(SpawnObjectType::Creature, 122)),
        ],
    );
    let mut rejected = Some(LoadedGridRespawnRecordsLikeCpp {
        pre_add_records: vec![player(2121, 571), player(2122, 530)],
        primary_record: record(122, 571),
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
    let facet_pointer = rejected.as_ref().unwrap().pre_add_records[0]
        .player()
        .unwrap() as *const Player;
    let mut shared = map(true);
    let outcome =
        shared.spawn_group_spawn_materialized(Some(&group), false, false, &store, |_, _, id, _| {
            if id == 121 {
                Ok(None)
            } else {
                Err(rejected.take().unwrap())
            }
        });
    assert_eq!(outcome.summary.blocked_loaded_grid_spawn_loads, 2);
    assert_eq!(outcome.summary.blocked_loaded_grid_creature_loads, 2);
    assert_eq!(outcome.summary.executed_loaded_grid_spawns, 0);
    assert!(outcome.summary.loaded_grid_primary_records.is_empty());
    assert_eq!(shared.map_object_count(), 0);
    assert!(shared.map_reference_order_like_cpp().is_empty());
    assert_eq!(outcome.attempts.len(), 2);
    for attempt in outcome.attempts {
        match attempt.result {
            LoadedGridSpawnAttemptResult::Unavailable => {
                assert_eq!(group_plan(&attempt.plan).spawn_id, 121)
            }
            LoadedGridSpawnAttemptResult::PreparationRejected(records) => {
                assert_eq!(group_plan(&attempt.plan).spawn_id, 122);
                assert_eq!(records.pre_add_records.len(), 2);
                assert_eq!(
                    records.pre_add_records[0].player().unwrap() as *const Player,
                    facet_pointer
                );
                assert_eq!(records.pre_add_records[1].object().map_id(), 530);
                let primary = records.primary_record.creature().unwrap();
                assert_eq!(primary as *const Creature, pointer);
                assert_eq!(primary.current_health(), 75);
                assert!(!primary.unit().world().object().is_in_world());
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
            }
            LoadedGridSpawnAttemptResult::Admitted(_) => panic!("neither load was admitted"),
            LoadedGridSpawnAttemptResult::NoLoader => panic!("Group always has a loader"),
        }
    }
}

#[test]
fn actor_success_moves_motor_and_keeps_pre_add_results_without_summary_clone() {
    let group = group(521, SpawnGroupFlags::NONE);
    let store = store(
        &[group.clone()],
        vec![(521, spawn(SpawnObjectType::Creature, 123))],
    );
    let (incoming, mut rng) = actor(123, true, 571);
    let guid = incoming.guid();
    let mut incoming = Some(incoming);
    let mut shared = map(true);
    let mut outcome =
        shared.spawn_group_spawn_materialized(Some(&group), false, false, &store, |_, _, _, _| {
            Ok(Some(LoadedGridMaterialization::creature(
                vec![player(2123, 571), player(2124, 530)],
                incoming.take().unwrap(),
            )))
        });
    assert_eq!(outcome.summary.executed_loaded_grid_spawns, 1);
    assert_eq!(outcome.summary.blocked_loaded_grid_spawn_add_to_map, 0);
    assert!(outcome.summary.loaded_grid_primary_records.is_empty());
    let LoadedGridSpawnAttemptResult::Admitted(admission) = outcome.attempts.pop().unwrap().result
    else {
        panic!("admission");
    };
    let (facets, primary) = admission.into_parts();
    assert!(facets[0].is_ok());
    assert!(facets[1].is_err());
    assert!(matches!(
        primary,
        LoadedGridPrimaryAdmission::CreatureActor(Ok(FreshCreatureActorAdmission::Inserted { .. }))
    ));
    let stored = shared.creature_actor_mut(guid).unwrap();
    assert_eq!(stored.create_data.npc_flags, 0x9876);
    stored.assert_actor_storage_runtime(&mut rng, true);
}

#[test]
fn actor_record_and_actor_duplicates_are_blocked_add_and_keep_incoming_and_stored_motor() {
    for existing_actor in [false, true] {
        let group = group(522, SpawnGroupFlags::NONE);
        let store = store(
            &[group.clone()],
            vec![(522, spawn(SpawnObjectType::Creature, 124))],
        );
        let mut shared = map(true);
        let (stored, mut stored_rng) = actor(124, true, 571);
        let guid = stored.guid();
        if existing_actor {
            assert!(matches!(
                shared.admit_fresh_creature_actor(stored),
                Ok(FreshCreatureActorAdmission::Inserted { .. })
            ));
        } else {
            shared.insert_map_object_record(record(124, 571)).unwrap();
        }
        let stored_pointer = shared.get_typed_creature(guid).unwrap() as *const Creature;
        let (mut incoming, mut rng) = actor(124, false, 571);
        incoming.creature.unit_mut().set_health(61);
        let mut incoming = Some(incoming);
        let mut outcome = shared.spawn_group_spawn_materialized(
            Some(&group),
            false,
            true,
            &store,
            |_, _, _, force| {
                assert!(force);
                Ok(Some(LoadedGridMaterialization::creature(
                    Vec::new(),
                    incoming.take().unwrap(),
                )))
            },
        );
        assert_eq!(outcome.summary.executed_loaded_grid_spawns, 0);
        assert_eq!(outcome.summary.blocked_loaded_grid_spawn_add_to_map, 1);
        assert_eq!(outcome.summary.blocked_loaded_grid_spawn_loads, 0);
        assert!(outcome.summary.loaded_grid_primary_records.is_empty());
        let LoadedGridSpawnAttemptResult::Admitted(admission) =
            outcome.attempts.pop().unwrap().result
        else {
            panic!("admission");
        };
        let (_, primary) = admission.into_parts();
        let LoadedGridPrimaryAdmission::CreatureActor(Ok(duplicate)) = primary else {
            panic!("duplicate");
        };
        let mut incoming = match duplicate {
            FreshCreatureActorAdmission::ExistingActor { incoming } => {
                assert!(existing_actor);
                incoming
            }
            FreshCreatureActorAdmission::ExistingRecord { incoming } => {
                assert!(!existing_actor);
                incoming
            }
            FreshCreatureActorAdmission::Inserted { .. } => panic!("must not refresh or replace"),
        };
        assert_eq!(incoming.creature.current_health(), 61);
        assert_eq!(incoming.create_data.npc_flags, 0x9876);
        incoming.assert_actor_storage_runtime(&mut rng, false);
        assert_eq!(
            shared.get_typed_creature(guid).unwrap() as *const Creature,
            stored_pointer
        );
        assert_eq!(
            shared.get_typed_creature(guid).unwrap().current_health(),
            75
        );
        if existing_actor {
            shared
                .creature_actor_mut(guid)
                .unwrap()
                .assert_actor_storage_runtime(&mut stored_rng, true);
        }
    }
}

#[test]
fn actor_primary_rejection_keeps_motor_and_does_not_rollback_timer_or_pre_add() {
    let group = group(523, SpawnGroupFlags::NONE);
    let store = store(
        &[group.clone()],
        vec![(523, spawn(SpawnObjectType::Creature, 125))],
    );
    let (incoming, mut rng) = actor(125, false, 530);
    let mut incoming = Some(incoming);
    let mut shared = map(true);
    timer(&mut shared, 125);
    let mut outcome = shared.spawn_group_spawn_materialized(
        Some(&group),
        true,
        false,
        &store,
        |map, kind, id, _| {
            assert_eq!(map.get_respawn_time_like_cpp(kind, id), 0);
            Ok(Some(LoadedGridMaterialization::creature(
                vec![player(2125, 571)],
                incoming.take().unwrap(),
            )))
        },
    );
    assert_eq!(outcome.summary.blocked_loaded_grid_spawn_add_to_map, 1);
    assert_eq!(outcome.summary.executed_loaded_grid_spawns, 0);
    assert_eq!(
        shared.get_respawn_time_like_cpp(SpawnObjectType::Creature, 125),
        0
    );
    assert_eq!(
        shared.map_reference_order_like_cpp(),
        &[ObjectGuid::create_player(1, 2125)]
    );
    let LoadedGridSpawnAttemptResult::Admitted(admission) = outcome.attempts.pop().unwrap().result
    else {
        panic!("admission");
    };
    let (facets, primary) = admission.into_parts();
    assert!(facets[0].is_ok());
    let LoadedGridPrimaryAdmission::CreatureActor(Err((error, mut returned))) = primary else {
        panic!("owned error");
    };
    assert!(matches!(error, FreshCreatureActorAdmissionError::Store(_)));
    assert_eq!(returned.creature.unit().world().map_id(), 530);
    returned.assert_actor_storage_runtime(&mut rng, false);
}
