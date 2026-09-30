//! Real-plan, owned-query boundaries. These tests are authored, not executed.
use super::*;
use std::time::Duration;
use wow_core::{Position, guid::HighGuid};
use wow_entities::{Creature, MapObjectRecord};
use crate::map_manager::{pending_respawn_from_world_creature_like_cpp,
    world_creature_from_pending_respawn_like_cpp};
use crate::spawn::{RespawnStoreLikeCpp, RespawnTransferError, RespawnInfoLikeCpp};

mod ownership;

fn pending(id: i64, spawn_id: u64, due: Instant) -> PendingRespawn {
    let mut creature = Creature::new(false);
    creature.unit_mut().world_mut().object_mut().create(ObjectGuid::create_world_object(
        HighGuid::Creature, 0, 1, 571, 7, 42, id,
    ));
    creature.unit_mut().world_mut().set_map(571, 7).unwrap();
    creature.unit_mut().world_mut().relocate(Position::xyz(1.0, 2.0, 3.0));
    creature.unit_mut().set_max_health(100);
    creature.unit_mut().set_health(100);
    creature.set_spawn_id(spawn_id);
    let data = WorldCreature::create_data_from_canonical_like_cpp(&creature);
    pending_respawn_from_world_creature_like_cpp(&WorldCreature::from_canonical(creature, data), due, 571)
}

fn setup() -> MapManager {
    let mut manager = MapManager::new(crate::MIN_GRID_DELAY_MS, 200);
    manager.create_world_map(571, 7);
    manager
}

fn queue(manager: &mut MapManager, incoming: PendingRespawn, now: Instant) {
    let store = manager.find_map_mut(571, 7).unwrap().map_mut().respawn_store_like_cpp_mut();
    if incoming.persistent_spawn { store.save_actor_row(&incoming, 571, 7, now, 100); }
    store.queue_actor(incoming).unwrap();
}

fn start(manager: &mut MapManager, now: Instant) -> (MapTickPlanLikeCpp, ActorRespawnRequest) {
    let plan = manager.begin_tick_like_cpp(200).into_started().unwrap();
    let participant = plan.updated_maps_like_cpp()[0];
    let request = match manager.begin_actor_respawn_map(&plan, participant, now, now, 100, true).unwrap() {
        ActorRespawnProgress::Pending(request) => request,
        _ => panic!("fixture has a ready actor"),
    };
    (plan, request)
}

#[test]
fn idle_and_foreign_origin_are_rejected_before_prefix() {
    let now = Instant::now(); let mut owner = setup(); let mut recipient = setup();
    queue(&mut recipient, pending(1, 1, now), now);
    let plan = owner.begin_tick_like_cpp(200).into_started().unwrap();
    let participant = plan.updated_maps_like_cpp()[0];
    assert_eq!(recipient.begin_actor_respawn_map(&plan, participant, now, now, 100, true).unwrap_err(),
        ActorRespawnError::OriginMismatch);
    assert_eq!(recipient.find_map(571, 7).unwrap().map().respawn_store_like_cpp().actor_queue_len(), 1);
    assert!(!recipient.find_map(571, 7).unwrap().map().respawn_store_like_cpp().has_reservations());
    recipient.tick_origin = Arc::clone(&owner.tick_origin);
    assert_eq!(recipient.begin_actor_respawn_map(&plan, participant, now, now, 100, true).unwrap_err(),
        ActorRespawnError::WrongState);
}

#[test]
fn wrong_epoch_and_stale_incarnation_do_not_consume_payload_or_save_again() {
    let now = Instant::now(); let mut manager = setup();
    queue(&mut manager, pending(2, 2, now), now);
    let plan = manager.begin_tick_like_cpp(200).into_started().unwrap();
    let participant = plan.updated_maps_like_cpp()[0];
    manager.tick_coordination_like_cpp = MapTickCoordinationStateLikeCpp::AwaitingSessions(plan.epoch_like_cpp() + 1);
    assert_eq!(manager.begin_actor_respawn_map(&plan, participant, now, now, 100, true).unwrap_err(),
        ActorRespawnError::WrongEpoch);
    manager.tick_coordination_like_cpp = MapTickCoordinationStateLikeCpp::AwaitingSessions(plan.epoch_like_cpp());
    manager.map_incarnations_like_cpp.insert(participant.key, participant.incarnation + 1);
    assert_eq!(manager.begin_actor_respawn_map(&plan, participant, now, now, 100, true).unwrap_err(),
        ActorRespawnError::StaleParticipant);
    let store = manager.find_map(571, 7).unwrap().map().respawn_store_like_cpp();
    assert_eq!(store.actor_queue_len(), 1);
    assert!(store.saved_row(SpawnObjectType::Creature, 2).is_some());
    assert!(!store.has_reservations());
}

#[test]
fn pending_is_busy_and_never_advances_timer_or_object_updater() {
    let now = Instant::now(); let mut manager = setup();
    queue(&mut manager, pending(3, 3, now), now);
    let (plan, request) = start(&mut manager, now);
    let before = manager.timer.current();
    assert!(manager.begin_tick_like_cpp(99_999).is_busy());
    assert_eq!(manager.timer.current(), before);
    assert!(!manager.can_resume_tick(&plan));
    assert!(matches!(manager.begin_object_tick(plan), Err(ObjectMapTickError::RespawnOperationInFlight)));
    assert!(manager.active_respawn.is_some());
    drop(request);
    assert!(manager.begin_tick_like_cpp(1).is_busy());
    assert_eq!(manager.timer.current(), before);
}

#[test]
fn reserved_queue_info_save_cancel_and_transport_return_inputs_without_competing() {
    let now = Instant::now(); let mut manager = setup();
    queue(&mut manager, pending(4, 4, now), now);
    let (_plan, request) = start(&mut manager, now);
    let key = RespawnKey::Persistent(SpawnObjectType::Creature, 4);
    let store = manager.find_map_mut(571, 7).unwrap().map_mut().respawn_store_like_cpp_mut();
    let (error, incoming) = store.try_queue_actor(pending(4, 4, now - Duration::from_secs(1))).unwrap_err();
    assert_eq!(error.key, key); assert_eq!(incoming.create_data.guid, request.guid());
    let info = RespawnInfoLikeCpp { object_type: SpawnObjectType::Creature, spawn_id: 4,
        entry: 42, respawn_time: 0, grid_id: 7 };
    let (error, returned) = store.try_add_info(info).unwrap_err();
    assert_eq!(error.key, key); assert_eq!(returned.respawn_time, 0);
    let row = *store.saved_row(SpawnObjectType::Creature, 4).unwrap();
    assert_eq!(store.try_save_row(row).unwrap_err().1, row);
    assert_eq!(store.try_save_actor_row(pending(4, 4, now), 571, 7, now, 100).unwrap_err().0.key, key);
    assert_eq!(store.try_cancel_owned(key).unwrap_err().key, key);
    assert_eq!(store.try_remove_info(SpawnObjectType::Creature, 4).unwrap_err().key, key);
    assert_eq!(store.try_remove_saved_row(SpawnObjectType::Creature, 4).unwrap_err().key, key);
    let fence = std::sync::Mutex::new(()); let guard = fence.lock().unwrap();
    assert_eq!(store.try_take_transfer(MapKey::new(571, 7), 1, &guard).unwrap_err(), RespawnTransferError::MapBusy);
    assert!(store.catalog_timer_keys().next().is_none());
    assert!(store.drain_ready_actors(now).is_empty());
    assert!(store.saved_row(SpawnObjectType::Creature, 4).is_some());
    drop(request);
}

#[test]
fn initial_occupied_guard_deletes_but_query_collision_keeps_saved_row() {
    let now = Instant::now();
    for late in [false, true] {
        let mut manager = setup();
        queue(&mut manager, pending(5, 5, now), now);
        let current = world_creature_from_pending_respawn_like_cpp(&pending(5, 5, now), 7);
        if !late {
            manager.find_map_mut(571, 7).unwrap().map_mut().insert_map_object_record(
                MapObjectRecord::new_creature(current.creature).unwrap()).unwrap();
            let plan = manager.begin_tick_like_cpp(200).into_started().unwrap();
            let outcome = match manager.begin_actor_respawn_map(&plan, plan.updated_maps_like_cpp()[0],
                now, now, 100, true).unwrap() {
                ActorRespawnProgress::Complete(outcome) => outcome,
                _ => panic!("initial guard skips factory"),
            };
            assert_eq!(outcome.attempts[0].status, ActorRespawnStatus::GuidOccupied);
            assert_eq!(outcome.respawn_db_mutations.len(), 1);
        } else {
            let (plan, request) = start(&mut manager, now);
            let reply = request.resolve(world_creature_from_pending_respawn_like_cpp, |_, _| {});
            manager.find_map_mut(571, 7).unwrap().map_mut().insert_map_object_record(
                MapObjectRecord::new_creature(current.creature).unwrap()).unwrap();
            let outcome = match manager.resume_actor_respawn(&plan, reply).unwrap() {
                ActorRespawnProgress::Complete(outcome) => outcome,
                _ => panic!("one attempt"),
            };
            assert_eq!(outcome.attempts[0].status, ActorRespawnStatus::AdmissionRejected);
            assert!(outcome.respawn_db_mutations.is_empty());
        }
        assert_eq!(manager.find_map(571, 7).unwrap().map().respawn_store_like_cpp()
            .saved_row(SpawnObjectType::Creature, 5).is_some(), late);
    }
}

#[test]
fn destroy_unload_transfer_and_instance_free_are_blocked_before_effects() {
    let now = Instant::now();
    let mut manager = MapManager::new(crate::MIN_GRID_DELAY_MS, 200);
    manager.create_map_entry(571, 7, 0, ManagedMapKind::Dungeon { has_reset_schedule: false });
    manager.create_world_map(572, 8);
    queue(&mut manager, pending(6, 6, now), now);
    let (_plan, request) = start(&mut manager, now);
    let ids = manager.instance_ids.clone();
    let unload_calls = manager.find_map(571, 7).unwrap().unload_all_calls;
    assert!(!manager.destroy_map(571, 7));
    assert!(!manager.destroy_map(572, 8));
    assert!(manager.unload_all().is_err());
    assert!(manager.find_map(572, 8).is_some());
    assert_eq!(manager.find_map(571, 7).unwrap().unload_all_calls, unload_calls);
    assert_eq!(manager.instance_ids, ids);
    let map = manager.maps.get_mut(&MapKey::new(571, 7)).unwrap();
    assert!(!MapManager::destroy_map_inner(map, &mut manager.instance_ids, true));
    let fence = std::sync::Mutex::new(()); let guard = fence.lock().unwrap();
    let mut source = RespawnStoreLikeCpp::new();
    source.queue_actor(pending(66, 66, now)).unwrap();
    let incoming = source.take_transfer(MapKey::new(571, 7), request.continuation.participant.incarnation, &guard);
    let (error, incoming) = manager.accept_respawn_transfer(incoming).unwrap_err();
    assert_eq!(error, RespawnTransferError::MapBusy);
    source.restore_transfer(incoming).unwrap();
    assert_eq!(source.actor_queue_len(), 1);
    assert_eq!(manager.instance_ids, ids);
    drop(request);
}

#[test]
fn map_plan_order_and_original_ordinal_order_do_not_prepare_later_prefix_early() {
    let now = Instant::now(); let mut manager = setup(); manager.create_world_map(572, 8);
    queue(&mut manager, pending(7, 7, now), now);
    queue(&mut manager, pending(8, 8, now - Duration::from_secs(1)), now);
    let mut later = world_creature_from_pending_respawn_like_cpp(&pending(9, 9, now), 8);
    later.creature.unit_mut().world_mut().reset_map().unwrap();
    later.creature.unit_mut().world_mut().set_map(572, 8).unwrap();
    later.creature.unit_mut().set_death_state(wow_constants::DeathState::JustDied);
    later.creature.unit_mut().set_health(0);
    later.creature.runtime_state_mut().save_respawn_requested = true;
    let later_guid = later.guid();
    manager.find_map_mut(572, 8).unwrap().map_mut().admit_fresh_creature_actor(later).unwrap();
    let plan = manager.begin_tick_like_cpp(200).into_started().unwrap();
    let first = plan.updated_maps_like_cpp()[0]; let second = plan.updated_maps_like_cpp()[1];
    assert_eq!(manager.begin_actor_respawn_map(&plan, second, now, now, 100, true).unwrap_err(),
        ActorRespawnError::ParticipantOrder);
    let mut progress = manager.begin_actor_respawn_map(&plan, first, now, now, 100, true).unwrap();
    let mut seen = Vec::new();
    loop {
        assert!(manager.find_map(572, 8).unwrap().map().creature_actor(later_guid).unwrap()
            .creature.runtime_state().save_respawn_requested);
        match progress {
            ActorRespawnProgress::Complete(outcome) => {
                assert_eq!(outcome.attempts.iter().map(|a| a.guid).collect::<Vec<_>>(), seen);
                break;
            }
            ActorRespawnProgress::Pending(request) => {
                seen.push(request.guid());
                let reply = request.resolve(world_creature_from_pending_respawn_like_cpp, |_, _| {});
                progress = manager.resume_actor_respawn(&plan, reply).unwrap();
            }
        }
    }
    assert_eq!(seen, vec![pending(7, 7, now).create_data.guid, pending(8, 8, now).create_data.guid]);
    assert!(!manager.can_resume_tick(&plan));
    let outcome = match manager.begin_actor_respawn_map(&plan, second, now, now, 100, true).unwrap() {
        ActorRespawnProgress::Complete(outcome) => outcome, _ => panic!("later Actor is not Corpse"),
    };
    assert_eq!(outcome.respawn_db_mutations.len(), 1);
    assert!(manager.can_resume_tick(&plan));
    assert_eq!(manager.begin_actor_respawn_map(&plan, first, now, now, 100, true).unwrap_err(),
        ActorRespawnError::ParticipantOrder);
}
