//! Prepared APP consumer and admission-incarnation filtering; no producer activation.
use super::*;
use std::sync::{Arc, Mutex, RwLock};
use std::time::Duration;
use wow_core::{ObjectGuid, Position, guid::HighGuid};
use wow_entities::Creature;
use wow_map::map_manager::{
    PendingRespawn, WorldCreature, pending_respawn_from_world_creature_like_cpp,
};
use wow_map::spawn::ActorRespawnStatus;
use wow_map::{MapKey, MapTickParticipantLikeCpp, SpawnObjectType};

mod query_boundary;

fn pending(id: u64, due: Instant) -> PendingRespawn {
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
    creature.set_spawn_id(id);
    let data = WorldCreature::create_data_from_canonical_like_cpp(&creature);
    pending_respawn_from_world_creature_like_cpp(
        &WorldCreature::from_canonical(creature, data),
        due,
        571,
    )
}

fn canonical() -> (wow_map::MapManager, MapTickParticipantLikeCpp) {
    let mut manager = wow_map::MapManager::new(wow_map::MIN_GRID_DELAY_MS, 200);
    manager.create_world_map(571, 7);
    let key = MapKey::new(571, 7);
    let participant = MapTickParticipantLikeCpp {
        key,
        incarnation: manager.map_incarnation_like_cpp(key).unwrap(),
    };
    (manager, participant)
}

// Earn a real plan and preserve its keys/order. Only participant incarnations
// are fixture inputs; the production driver decides whether each is stale.
fn prepared_actor_respawn_phase(
    manager: &mut wow_map::MapManager,
    participants: &[MapTickParticipantLikeCpp],
    maps: &wow_data::MapStore,
    terrain: Option<&LiveTerrainHeights>,
    now: Instant,
    conversion_now: Instant,
    conversion_now_secs: i64,
) -> Vec<(MapKey, ActorRespawnPhaseOutcome)> {
    let mut plan = manager.begin_tick_like_cpp(200).into_started().unwrap();
    assert_eq!(
        plan.updated_maps_like_cpp()
            .iter()
            .map(|item| item.key)
            .collect::<Vec<_>>(),
        participants.iter().map(|item| item.key).collect::<Vec<_>>()
    );
    for participant in participants {
        assert!(
            plan.fixture_set_respawn_participant_incarnation(
                participant.key,
                participant.incarnation,
            )
        );
    }
    let moved = std::mem::replace(
        manager,
        wow_map::MapManager::new(wow_map::MIN_GRID_DELAY_MS, 200),
    );
    let canonical = Arc::new(Mutex::new(moved));
    let outcomes = super::prepared_actor_respawn_phase(
        &canonical,
        &plan,
        maps,
        terrain,
        now,
        conversion_now,
        conversion_now_secs,
    )
    .unwrap();
    let mut moved = Arc::try_unwrap(canonical).unwrap().into_inner().unwrap();
    moved.abandon_tick_like_cpp(plan);
    *manager = moved;
    outcomes
}

#[test]
fn prepared_phase_skips_stale_incarnation_without_consuming_actor() {
    let now = Instant::now();
    let (mut manager, participant) = canonical();
    manager
        .find_map_mut(571, 7)
        .unwrap()
        .map_mut()
        .respawn_store_like_cpp_mut()
        .queue_actor(pending(1, now))
        .unwrap();
    let stale = MapTickParticipantLikeCpp {
        incarnation: participant.incarnation + 1,
        ..participant
    };
    let maps = wow_data::MapStore::from_entries([]);
    assert!(
        prepared_actor_respawn_phase(&mut manager, &[stale], &maps, None, now, now, 100).is_empty()
    );
    assert_eq!(
        manager
            .find_map(571, 7)
            .unwrap()
            .map()
            .respawn_store_like_cpp()
            .actor_queue_len(),
        1
    );
    let ready =
        prepared_actor_respawn_phase(&mut manager, &[participant], &maps, None, now, now, 100);
    assert_eq!(ready.len(), 1);
    assert_eq!(ready[0].0, participant.key);
    assert_eq!(ready[0].1.attempts[0].status, ActorRespawnStatus::Inserted);
    assert!(
        manager
            .find_map(571, 7)
            .unwrap()
            .map()
            .get_creature(ready[0].1.attempts[0].guid)
            .is_some()
    );
}

#[test]
fn stale_driver_preserves_queue_and_timer_without_reservations_or_tick_blocking() {
    let now = Instant::now();
    let (mut manager, participant) = canonical();
    manager
        .find_map_mut(571, 7)
        .unwrap()
        .map_mut()
        .respawn_store_like_cpp_mut()
        .queue_actor(pending(5, now))
        .unwrap();
    let mut plan = manager.begin_tick_like_cpp(200).into_started().unwrap();
    assert!(!plan.fixture_set_respawn_participant_incarnation(MapKey::new(0, 0), 99));
    assert_eq!(plan.updated_maps_like_cpp(), &[participant]);
    assert!(
        plan.fixture_set_respawn_participant_incarnation(
            participant.key,
            participant.incarnation + 1,
        )
    );
    let canonical = Arc::new(Mutex::new(manager));
    let outcomes = super::prepared_actor_respawn_phase(
        &canonical,
        &plan,
        &wow_data::MapStore::from_entries([]),
        None,
        now,
        now,
        100,
    )
    .unwrap();
    assert!(outcomes.is_empty());
    let mut manager = canonical.lock().unwrap();
    let store = manager
        .find_map(571, 7)
        .unwrap()
        .map()
        .respawn_store_like_cpp();
    assert_eq!(store.actor_queue_len(), 1);
    assert!(!store.has_reservations());
    assert!(manager.can_resume_tick(&plan));
    assert!(matches!(
        manager.begin_tick_like_cpp(999),
        wow_map::MapTickBeginLikeCpp::Busy { .. }
    ));
    assert!(matches!(
        manager.abandon_tick_like_cpp(plan),
        wow_map::MapTickResumeLikeCpp::Resumed
    ));
    let next = manager.begin_tick_like_cpp(0).into_started().unwrap();
    assert_eq!(next.effective_diff_ms(), 200);
    assert!(matches!(
        manager.abandon_tick_like_cpp(next),
        wow_map::MapTickResumeLikeCpp::Resumed
    ));
}

#[test]
fn prepared_transfer_and_phase_preserve_exact_instant_and_do_not_execute_legacy_again() {
    let now = Instant::now();
    let due = now + Duration::from_nanos(999);
    let (manager, participant) = canonical();
    let canonical = Arc::new(Mutex::new(manager));
    let legacy = Arc::new(RwLock::new(wow_map::map_manager::MapManager::new()));
    legacy
        .write()
        .unwrap()
        .push_respawn(571, 7, pending(2, due));
    let writer_fence = Arc::new(Mutex::new(()));
    transfer_respawns_under_quiescence(
        &legacy,
        &canonical,
        &writer_fence,
        participant.key,
        participant.incarnation,
    )
    .unwrap();
    assert!(
        legacy
            .write()
            .unwrap()
            .drain_ready_respawns(571, 7, due)
            .is_empty()
    );
    let maps = wow_data::MapStore::from_entries([]);
    let mut manager = canonical.lock().unwrap();
    let not_due =
        prepared_actor_respawn_phase(&mut manager, &[participant], &maps, None, now, now, 100);
    assert!(not_due[0].1.attempts.is_empty());
    let ready =
        prepared_actor_respawn_phase(&mut manager, &[participant], &maps, None, due, now, 100);
    assert_eq!(ready[0].1.attempts[0].status, ActorRespawnStatus::Inserted);
    assert_eq!(
        manager
            .find_map(571, 7)
            .unwrap()
            .map()
            .respawn_store_like_cpp()
            .actor_queue_len(),
        0
    );
}

#[test]
fn prepared_delivery_submits_delete_after_memory_transition_and_returns_no_record_mirror() {
    let now = Instant::now();
    let (mut manager, participant) = canonical();
    let incoming = pending(3, now);
    let guid = incoming.create_data.guid;
    let store = manager
        .find_map_mut(571, 7)
        .unwrap()
        .map_mut()
        .respawn_store_like_cpp_mut();
    store.save_actor_row(&incoming, 571, 7, now, 100);
    store.queue_actor(incoming).unwrap();
    let plan = manager.begin_tick_like_cpp(200).into_started().unwrap();
    let canonical = Arc::new(Mutex::new(manager));
    let writer_fence = Arc::new(Mutex::new(()));
    let writer = RespawnDbWriterSenderLikeCpp::new_like_cpp();
    let maps = wow_data::MapStore::from_entries([]);
    let (outcomes, produced, submitted) = run_prepared_actor_respawns(
        &canonical,
        &plan,
        &maps,
        None,
        now,
        now,
        100,
        &writer_fence,
        &writer,
    )
    .unwrap();
    assert_eq!(produced, 1);
    assert_eq!(submitted, 1);
    assert_eq!(
        outcomes[0].1.attempts[0].status,
        ActorRespawnStatus::Inserted
    );
    assert!(outcomes[0].1.respawn_db_mutations.is_empty());
    let manager = canonical.lock().unwrap();
    let map = manager.find_map(571, 7).unwrap().map();
    assert!(map.object_is_in_world(guid));
    assert!(
        map.respawn_store_like_cpp()
            .saved_row(SpawnObjectType::Creature, 3)
            .is_none()
    );
    assert_eq!(map.creature_spawn_id_store_count_like_cpp(3), 1);
}

#[test]
fn closed_mailbox_reports_no_submission_without_rolling_back_actor_memory() {
    let now = Instant::now();
    let (mut manager, participant) = canonical();
    let incoming = pending(4, now);
    let guid = incoming.create_data.guid;
    let store = manager
        .find_map_mut(571, 7)
        .unwrap()
        .map_mut()
        .respawn_store_like_cpp_mut();
    store.save_actor_row(&incoming, 571, 7, now, 100);
    store.queue_actor(incoming).unwrap();
    let plan = manager.begin_tick_like_cpp(200).into_started().unwrap();
    let canonical = Arc::new(Mutex::new(manager));
    let writer_fence = Arc::new(Mutex::new(()));
    let writer = RespawnDbWriterSenderLikeCpp::new_like_cpp();
    writer.close_like_cpp();
    let (outcomes, produced, submitted) = run_prepared_actor_respawns(
        &canonical,
        &plan,
        &wow_data::MapStore::from_entries([]),
        None,
        now,
        now,
        100,
        &writer_fence,
        &writer,
    )
    .unwrap();
    assert_eq!(produced, 1);
    assert_eq!(submitted, 0);
    assert_eq!(
        outcomes[0].1.attempts[0].status,
        ActorRespawnStatus::Inserted
    );
    assert!(
        canonical
            .lock()
            .unwrap()
            .find_map(571, 7)
            .unwrap()
            .map()
            .object_is_in_world(guid)
    );
}
