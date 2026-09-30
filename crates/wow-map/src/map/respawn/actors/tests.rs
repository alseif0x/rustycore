//! Prepared lifecycle/Actor executor contracts, using the actual Map and factory.
use super::*;
use crate::map_manager::{WorldCreature, world_creature_from_pending_respawn_like_cpp};
use crate::spawn::{RespawnStoreLikeCpp, RespawnTransferError};
use crate::{Map, MapKey};
use std::cell::RefCell;
use std::time::Duration;
use wow_core::{Position, guid::HighGuid};
use wow_entities::{Creature, MapObjectRecord};

fn actor(id: u64, spawn_id: u64) -> WorldCreature {
    let mut creature = Creature::new(false);
    creature
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(ObjectGuid::create_world_object(
            HighGuid::Creature,
            0,
            1,
            571,
            7,
            42,
            id as i64,
        ));
    creature.unit_mut().world_mut().set_map(571, 7).unwrap();
    creature
        .unit_mut()
        .world_mut()
        .relocate(Position::xyz(1.0, 2.0, 3.0));
    creature.unit_mut().set_max_health(100);
    creature.unit_mut().set_health(100);
    creature.set_spawn_id(spawn_id);
    let data = WorldCreature::create_data_from_canonical_like_cpp(&creature);
    WorldCreature::from_canonical(creature, data)
}

fn queue(map: &mut Map, id: u64, spawn_id: u64, due: Instant, now: Instant) -> ObjectGuid {
    let actor = actor(id, spawn_id);
    let guid = actor.guid();
    let pending = pending_respawn_from_world_creature_like_cpp(&actor, due, 571);
    if spawn_id != 0 {
        map.respawn_store.save_actor_row(&pending, 571, 7, now, 100);
    }
    map.respawn_store.queue_actor(pending).unwrap();
    guid
}

fn run(map: &mut Map, now: Instant) -> ActorRespawnPhaseOutcome {
    run_with(
        map,
        now,
        world_creature_from_pending_respawn_like_cpp,
        |_, _| {},
    )
}

// Move the SAME concrete Map through the real manager/plan seam. No actor or
// Record is cloned; only this test adapter replaces the temporary empty Map.
fn run_with(
    map: &mut Map,
    now: Instant,
    mut factory: impl FnMut(&PendingRespawn, u32) -> WorldCreature,
    mut snap: impl FnMut(&mut WorldCreature, u16),
) -> ActorRespawnPhaseOutcome {
    let mut manager = crate::MapManager::new(crate::MIN_GRID_DELAY_MS, 200);
    manager.create_world_map(571, 7);
    std::mem::swap(map, manager.find_map_mut(571, 7).unwrap().map_mut());
    let plan = manager.begin_tick_like_cpp(200).into_started().unwrap();
    let participant = plan.updated_maps_like_cpp()[0];
    let mut progress = manager
        .begin_actor_respawn_map(&plan, participant, now, now, 100, true)
        .unwrap();
    let outcome = loop {
        match progress {
            crate::manager::ActorRespawnProgress::Complete(outcome) => break outcome,
            crate::manager::ActorRespawnProgress::Pending(request) => {
                let reply = request.resolve(&mut factory, &mut snap);
                progress = manager.resume_actor_respawn(&plan, reply).unwrap();
            }
        }
    };
    std::mem::swap(map, manager.find_map_mut(571, 7).unwrap().map_mut());
    outcome
}

#[test]
fn just_died_save_remains_invisible_until_corpse_removal_queues_actor() {
    let now = Instant::now();
    let mut map = Map::new(571, 7, 0, 1000);
    let mut incoming = actor(10, 100);
    incoming
        .creature
        .unit_mut()
        .set_death_state(wow_constants::DeathState::JustDied);
    incoming.creature.unit_mut().set_health(0);
    incoming.creature.runtime_state_mut().save_respawn_requested = true;
    let guid = incoming.guid();
    map.admit_fresh_creature_actor(incoming).unwrap();
    let first = run(&mut map, now);
    assert_eq!(first.respawn_db_mutations.len(), 1);
    assert!(matches!(
        first.respawn_db_mutations[0],
        RespawnPersistenceMutationLikeCpp::Save { .. }
    ));
    assert!(
        map.get_respawn_info_like_cpp(SpawnObjectType::Creature, 100)
            .is_none()
    );
    assert_eq!(
        map.get_respawn_time_like_cpp(SpawnObjectType::Creature, 100),
        0
    );
    assert_eq!(map.respawn_store.actor_queue_len(), 0);
    assert!(
        !map.creature_actor(guid)
            .unwrap()
            .creature
            .runtime_state()
            .save_respawn_requested
    );
    assert!(run(&mut map, now).respawn_db_mutations.is_empty());
    let corpse = map.creature_actor_mut(guid).unwrap();
    corpse
        .creature
        .unit_mut()
        .set_death_state(wow_constants::DeathState::Corpse);
    corpse.creature.set_ai_corpse_despawn_at(Some(0));
    let second = run(&mut map, now);
    assert_eq!(second.corpses_removed, 1);
    assert!(!map.contains_map_object_like_cpp(guid));
    assert_eq!(map.respawn_store.actor_queue_len(), 1);
    assert!(
        map.get_respawn_info_like_cpp(SpawnObjectType::Creature, 100)
            .is_some()
    );
    assert_eq!(map.respawn_store.catalog_timer_keys().count(), 0);
    assert!(second.attempts.is_empty());
}

#[test]
fn factory_ground_snap_and_insertion_follow_ordinal_order() {
    let now = Instant::now();
    let mut map = Map::new(571, 7, 0, 1000);
    let first = queue(&mut map, 20, 200, now, now);
    let second = queue(&mut map, 21, 201, now - Duration::from_secs(1), now);
    let trace = RefCell::new(Vec::new());
    let result = run_with(
        &mut map,
        now,
        |pending, instance| {
            trace
                .borrow_mut()
                .push(("factory", pending.create_data.guid));
            world_creature_from_pending_respawn_like_cpp(pending, instance)
        },
        |actor, _| {
            trace.borrow_mut().push(("snap", actor.guid()));
            actor
                .creature
                .unit_mut()
                .world_mut()
                .relocate(Position::xyz(1.0, 2.0, 9.0));
        },
    );
    assert_eq!(
        *trace.borrow(),
        vec![
            ("factory", first),
            ("snap", first),
            ("factory", second),
            ("snap", second)
        ]
    );
    assert_eq!(
        result.attempts.iter().map(|a| a.guid).collect::<Vec<_>>(),
        vec![first, second]
    );
    assert!(
        result
            .attempts
            .iter()
            .all(|a| a.status == ActorRespawnStatus::Inserted)
    );
    assert_eq!(result.respawn_db_mutations.len(), 2);
    for guid in [first, second] {
        assert!(map.object_is_in_world(guid));
        assert_eq!(
            map.creature_actor(guid)
                .unwrap()
                .creature
                .unit()
                .world()
                .position()
                .z,
            9.0
        );
    }
    assert!(map.respawn_store.saved_rows().is_empty());
}

#[test]
fn occupied_guid_even_dead_skips_factory_and_snap_and_deletes_saved_row() {
    let now = Instant::now();
    let mut map = Map::new(571, 7, 0, 1000);
    let guid = queue(&mut map, 30, 300, now, now);
    let mut current = actor(30, 300);
    current
        .creature
        .unit_mut()
        .set_death_state(wow_constants::DeathState::Dead);
    current.creature.unit_mut().set_health(0);
    map.insert_map_object_record(MapObjectRecord::new_creature(current.creature).unwrap())
        .unwrap();
    let result = run_with(
        &mut map,
        now,
        |_, _| panic!("GUID guard precedes factory"),
        |_, _| panic!("GUID guard precedes snap"),
    );
    assert_eq!(result.attempts[0].status, ActorRespawnStatus::GuidOccupied);
    assert_eq!(result.attempts[0].guid, guid);
    assert_eq!(result.respawn_db_mutations.len(), 1);
    assert!(matches!(
        result.respawn_db_mutations[0],
        RespawnPersistenceMutationLikeCpp::Delete { .. }
    ));
    assert!(map.creature_actor(guid).is_none()); // Record was neither promoted nor overwritten.
}

#[test]
fn only_alive_matching_spawn_blocks_a_different_guid() {
    let now = Instant::now();
    for alive in [false, true] {
        let mut map = Map::new(571, 7, 0, 1000);
        let guid = queue(&mut map, 40, 400, now, now);
        let mut other = actor(41, 400);
        if !alive {
            other
                .creature
                .unit_mut()
                .set_death_state(wow_constants::DeathState::Dead);
            other.creature.unit_mut().set_health(0);
        }
        map.insert_map_object_record(MapObjectRecord::new_creature(other.creature).unwrap())
            .unwrap();
        let result = run(&mut map, now);
        assert_eq!(
            result.attempts[0].status,
            if alive {
                ActorRespawnStatus::SpawnOccupied
            } else {
                ActorRespawnStatus::Inserted
            }
        );
        assert_eq!(map.creature_actor(guid).is_some(), !alive);
    }
}

#[test]
fn factory_admission_failure_consumes_attempt_without_retry_and_continues() {
    let now = Instant::now();
    let mut map = Map::new(571, 7, 0, 1000);
    queue(&mut map, 50, 500, now, now);
    queue(&mut map, 51, 501, now, now);
    let mut factories = 0;
    let result = run_with(
        &mut map,
        now,
        |pending, instance| {
            factories += 1;
            let mut actor = world_creature_from_pending_respawn_like_cpp(pending, instance);
            if pending.spawn_id == 500 {
                actor.creature.unit_mut().world_mut().reset_map().unwrap();
                actor
                    .creature
                    .unit_mut()
                    .world_mut()
                    .set_map(530, instance)
                    .unwrap();
            }
            actor
        },
        |_, _| {},
    );
    assert_eq!(factories, 2);
    assert_eq!(
        result.attempts.iter().map(|a| a.status).collect::<Vec<_>>(),
        vec![
            ActorRespawnStatus::AdmissionRejected,
            ActorRespawnStatus::Inserted
        ]
    );
    assert_eq!(map.respawn_store.actor_queue_len(), 0);
    assert!(
        map.respawn_store
            .saved_row(SpawnObjectType::Creature, 500)
            .is_some()
    );
    assert!(
        map.get_respawn_info_like_cpp(SpawnObjectType::Creature, 500)
            .is_none()
    );
    assert!(run(&mut map, now).attempts.is_empty());
}

#[test]
fn actor_exact_instant_survives_an_already_due_unix_projection() {
    let now = Instant::now();
    let due = now + Duration::from_nanos(1);
    let mut map = Map::new(571, 7, 0, 1000);
    queue(&mut map, 60, 600, due, now);
    map.add_respawn_info_like_cpp(RespawnInfoLikeCpp {
        object_type: SpawnObjectType::Creature,
        spawn_id: 600,
        entry: 42,
        respawn_time: 0,
        grid_id: 7,
    });
    assert!(
        map.process_due_respawns_like_cpp(
            i64::MAX,
            |_, _| panic!("Actor is not Catalog"),
            |_| panic!("Actor is not Catalog")
        )
        .is_empty()
    );
    assert!(run(&mut map, now).attempts.is_empty());
    assert_eq!(
        run(&mut map, due).attempts[0].status,
        ActorRespawnStatus::Inserted
    );
}

#[test]
fn fresh_respawn_admission_moves_the_factory_motor_rng_clock_and_plan_intact() {
    let now = Instant::now();
    let mut map = Map::new(571, 7, 0, 1000);
    let guid = queue(&mut map, 70, 700, now, now);
    let mut expected_rng = None;
    let result = run_with(
        &mut map,
        now,
        |pending, instance| {
            let mut actor = world_creature_from_pending_respawn_like_cpp(pending, instance);
            expected_rng = Some(actor.seed_actor_storage_runtime(true));
            actor
        },
        |_, _| {},
    );
    assert_eq!(result.attempts[0].status, ActorRespawnStatus::Inserted);
    map.creature_actor_mut(guid)
        .unwrap()
        .assert_actor_storage_runtime(&mut expected_rng.unwrap(), true);
}

#[test]
fn transient_actor_fallback_is_not_a_persistent_row_or_catalog_spawn_id() {
    let now = Instant::now();
    let mut map = Map::new(571, 7, 0, 1000);
    let guid = queue(&mut map, 80, 0, now, now);
    let result = run(&mut map, now);
    assert_eq!(
        result.attempts[0].key,
        RespawnKey::TransientCreature(guid.low_value() as u64)
    );
    assert_eq!(result.attempts[0].status, ActorRespawnStatus::Inserted);
    assert!(result.respawn_db_mutations.is_empty());
    assert_eq!(map.creature_actor(guid).unwrap().creature.spawn_id(), 0);
    assert_eq!(map.respawn_timer_keys_like_cpp().count(), 0);
}

#[test]
fn manager_transfer_checks_current_incarnation_before_mutation_and_returns_source() {
    let now = Instant::now();
    let mut manager = crate::MapManager::new(crate::MIN_GRID_DELAY_MS, 200);
    let key = MapKey::new(571, 7);
    manager.create_world_map(571, 7);
    let incarnation = manager.map_incarnation_like_cpp(key).unwrap();
    let fence = std::sync::Mutex::new(());
    let guard = fence.lock().unwrap();
    let mut source = RespawnStoreLikeCpp::new();
    source
        .queue_actor(pending_respawn_from_world_creature_like_cpp(
            &actor(90, 900),
            now,
            571,
        ))
        .unwrap();
    let transfer = source.take_transfer(key, incarnation + 1, &guard);
    let (error, transfer) = manager.accept_respawn_transfer(transfer).unwrap_err();
    assert_eq!(error, RespawnTransferError::StaleIncarnation);
    assert_eq!(
        manager
            .find_map(571, 7)
            .unwrap()
            .map()
            .respawn_store
            .actor_queue_len(),
        0
    );
    source.restore_transfer(transfer).unwrap();
    manager
        .accept_respawn_transfer(source.take_transfer(key, incarnation, &guard))
        .unwrap();
    assert_eq!(source.actor_queue_len(), 0);
    let result = run(manager.find_map_mut(571, 7).unwrap().map_mut(), now);
    assert_eq!(result.attempts[0].status, ActorRespawnStatus::Inserted);
}

#[test]
fn mixed_catalog_load_failure_keeps_load_before_delete_and_original_consume_policy() {
    for consume in [false, true] {
        let now = Instant::now();
        let mut map = Map::new(571, 7, 0, 1000);
        queue(&mut map, 100, 1000, now + Duration::from_secs(60), now);
        let grid_id = crate::compute_grid_coord(1.0, 2.0).get_id();
        map.ensure_grid_loaded(&crate::Cell::from_world(1.0, 2.0));
        map.add_respawn_info_like_cpp(RespawnInfoLikeCpp {
            object_type: SpawnObjectType::Creature,
            spawn_id: 1000,
            entry: 42,
            respawn_time: 0,
            grid_id,
        });
        let mut store = SpawnStore::new();
        for spawn_id in [1001, 1002] {
            store.add_object_spawn(
                &crate::spawn::SpawnData {
                    object_type: SpawnObjectType::Creature,
                    spawn_id,
                    map_id: 571,
                    db_data: true,
                    spawn_group: crate::spawn::SpawnGroupTemplateData::default_group(),
                    id: 42,
                    spawn_point: crate::spawn::SpawnPosition::new(1.0, 2.0, 3.0, 0.0),
                    phase_use_flags: 0,
                    phase_id: 0,
                    phase_group: 0,
                    terrain_swap_map: -1,
                    pool_id: 0,
                    spawn_time_secs: 120,
                    spawn_difficulties: vec![0],
                    script_id: 0,
                    string_id: String::new(),
                },
                |_| false,
            );
            map.add_respawn_info_like_cpp(RespawnInfoLikeCpp {
                object_type: SpawnObjectType::Creature,
                spawn_id,
                entry: 42,
                respawn_time: 20,
                grid_id,
            });
        }
        let mut loaded = Vec::new();
        let summary = map.process_due_respawns_composite_loaded_grid_respawns_like_cpp(
            20,
            &store,
            &LinkedRespawnStoreLikeCpp::new(),
            &PoolMgrLikeCpp::new(),
            5,
            false,
            |_, _| false,
            |_, _| 0.0,
            |_, count| (0..count).collect(),
            consume,
            |map, object_type, spawn_id| {
                assert!(
                    map.get_respawn_info_like_cpp(object_type, spawn_id)
                        .is_some()
                );
                loaded.push(spawn_id);
                None
            },
        );
        assert_eq!(
            loaded,
            if consume {
                vec![1002, 1001]
            } else {
                vec![1002]
            }
        );
        assert_eq!(
            summary.blocked_loaded_grid_respawn_loads,
            if consume { 2 } else { 1 }
        );
        assert_eq!(
            map.get_respawn_info_like_cpp(SpawnObjectType::Creature, 1002)
                .is_none(),
            consume
        );
        assert_eq!(map.respawn_store.actor_queue_len(), 1);
        assert!(
            map.get_respawn_info_like_cpp(SpawnObjectType::Creature, 1000)
                .is_some()
        );
        assert!(map.respawn_store.drain_ready_actors(now).is_empty());
    }
}

#[test]
fn split_tick_busy_transfer_returns_ownership_without_touching_either_store() {
    let now = Instant::now();
    let mut manager = crate::MapManager::new(crate::MIN_GRID_DELAY_MS, 200);
    let key = MapKey::new(571, 7);
    manager.create_world_map(571, 7);
    let incarnation = manager.map_incarnation_like_cpp(key).unwrap();
    let plan = manager
        .begin_tick_like_cpp(5000)
        .into_started()
        .expect("admitted split tick");
    let fence = std::sync::Mutex::new(());
    let guard = fence.lock().unwrap();
    let mut source = RespawnStoreLikeCpp::new();
    source
        .queue_actor(pending_respawn_from_world_creature_like_cpp(
            &actor(101, 1001),
            now,
            571,
        ))
        .unwrap();
    let (error, incoming) = manager
        .accept_respawn_transfer(source.take_transfer(key, incarnation, &guard))
        .unwrap_err();
    assert_eq!(error, RespawnTransferError::MapBusy);
    assert_eq!(
        manager
            .find_map(571, 7)
            .unwrap()
            .map()
            .respawn_store
            .actor_queue_len(),
        0
    );
    source.restore_transfer(incoming).unwrap();
    assert_eq!(source.actor_queue_len(), 1);
    assert_eq!(
        manager.tick_coordination_like_cpp(),
        crate::MapTickCoordinationStateLikeCpp::AwaitingSessions(plan.epoch_like_cpp())
    );
}
