//! Ownership handoff under the real shared manager guards and writer fence.
use super::*;
use crate::session::WorldSession;
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};
use wow_core::{ObjectGuid, Position, guid::HighGuid};
use wow_entities::Creature;
use wow_map::map_manager::{PendingRespawn, WorldCreature, pending_respawn_from_world_creature_like_cpp};
use wow_map::{MapKey, SpawnObjectType};

fn pending(id: u64, due: Instant) -> PendingRespawn {
    let mut creature = Creature::new(false);
    creature.unit_mut().world_mut().object_mut().create(ObjectGuid::create_world_object(
        HighGuid::Creature, 0, 1, 571, 7, 42, id as i64,
    ));
    creature.unit_mut().world_mut().set_map(571, 7).unwrap();
    creature.unit_mut().world_mut().relocate(Position::xyz(1.0, 2.0, 3.0));
    creature.unit_mut().set_max_health(100);
    creature.unit_mut().set_health(100);
    creature.set_spawn_id(id);
    let data = WorldCreature::create_data_from_canonical_like_cpp(&creature);
    pending_respawn_from_world_creature_like_cpp(&WorldCreature::from_canonical(creature, data), due, 571)
}

fn owners() -> (crate::map_manager::SharedMapManager, SharedCanonicalMapManager, u64) {
    let legacy = Arc::new(RwLock::new(crate::map_manager::MapManager::new()));
    legacy.write().unwrap().get_or_create_map(571, 7);
    let mut canonical = wow_map::MapManager::new(wow_map::MIN_GRID_DELAY_MS, 200);
    canonical.create_world_map(571, 7);
    let incarnation = canonical.map_incarnation_like_cpp(MapKey::new(571, 7)).unwrap();
    (legacy, Arc::new(Mutex::new(canonical)), incarnation)
}

#[test]
fn handoff_retires_source_queue_and_merges_catalog_info_without_payload_reconstruction() {
    let now = Instant::now(); let due = now + Duration::from_nanos(123);
    let (legacy, canonical, incarnation) = owners();
    let mut incoming = pending(1, due); incoming.script_name = "retained".into();
    legacy.write().unwrap().push_respawn(571, 7, incoming);
    canonical.lock().unwrap().find_map_mut(571, 7).unwrap().map_mut()
        .add_respawn_info_like_cpp(wow_map::RespawnInfoLikeCpp {
            object_type: SpawnObjectType::Creature, spawn_id: 1,
            entry: 42, respawn_time: 1, grid_id: 7,
        });
    let fence = Mutex::new(()); let guard = fence.lock().unwrap();
    WorldSession::transfer_legacy_respawns_to_canonical(
        &legacy, &canonical, MapKey::new(571, 7), incarnation, &guard,
    ).unwrap();
    assert_eq!(legacy.read().unwrap().respawn_queue_len(571, 7), 0);
    let mut canonical = canonical.lock().unwrap();
    let store = canonical.find_map_mut(571, 7).unwrap().map_mut().respawn_store_like_cpp_mut();
    assert_eq!(store.actor_queue_len(), 1);
    assert_eq!(store.catalog_timer_keys().count(), 0);
    assert_eq!(store.get_respawn_time_like_cpp(SpawnObjectType::Creature, 1), 1);
    assert!(store.drain_ready_actors(now).is_empty());
    let ready = store.drain_ready_actors(due);
    assert_eq!(ready[0].respawn_at, due);
    assert_eq!(ready[0].script_name, "retained");
}

#[test]
fn destination_actor_collision_restores_whole_legacy_queue_in_original_order() {
    let now = Instant::now();
    let (legacy, canonical, incarnation) = owners();
    legacy.write().unwrap().push_respawn(571, 7, pending(1, now));
    legacy.write().unwrap().push_respawn(571, 7, pending(2, now));
    canonical.lock().unwrap().find_map_mut(571, 7).unwrap().map_mut()
        .respawn_store_like_cpp_mut().queue_actor(pending(2, now)).unwrap();
    let fence = Mutex::new(()); let guard = fence.lock().unwrap();
    let error = WorldSession::transfer_legacy_respawns_to_canonical(
        &legacy, &canonical, MapKey::new(571, 7), incarnation, &guard,
    ).unwrap_err();
    assert!(matches!(error, RespawnOwnerTransferError::Admission(RespawnTransferError::OccupiedActor { .. })));
    assert_eq!(canonical.lock().unwrap().find_map(571, 7).unwrap().map()
        .respawn_store_like_cpp().actor_queue_len(), 1);
    assert_eq!(legacy.write().unwrap().drain_ready_respawns(571, 7, now)
        .iter().map(|pending| pending.spawn_id).collect::<Vec<_>>(), vec![1, 2]);
}

#[test]
fn stale_incarnation_and_missing_source_never_extract_or_create_a_map() {
    let now = Instant::now();
    let (legacy, canonical, incarnation) = owners();
    legacy.write().unwrap().push_respawn(571, 7, pending(1, now));
    let fence = Mutex::new(()); let guard = fence.lock().unwrap();
    assert_eq!(WorldSession::transfer_legacy_respawns_to_canonical(
        &legacy, &canonical, MapKey::new(571, 7), incarnation + 1, &guard,
    ), Err(RespawnOwnerTransferError::Admission(RespawnTransferError::StaleIncarnation)));
    assert_eq!(legacy.read().unwrap().respawn_queue_len(571, 7), 1);
    let missing_source = Arc::new(RwLock::new(crate::map_manager::MapManager::new()));
    assert_eq!(WorldSession::transfer_legacy_respawns_to_canonical(
        &missing_source, &canonical, MapKey::new(571, 7), incarnation, &guard,
    ), Err(RespawnOwnerTransferError::MissingLegacyMap));
    assert!(missing_source.read().unwrap().active_map_keys().is_empty());
    assert_eq!(canonical.lock().unwrap().find_map(571, 7).unwrap().map()
        .respawn_store_like_cpp().actor_queue_len(), 0);
}

#[test]
fn invalid_payload_map_restores_saved_rows_and_actor_queue_together() {
    let now = Instant::now();
    let (legacy, canonical, incarnation) = owners();
    let mut incoming = pending(1, now); incoming.map_id = 530;
    legacy.write().unwrap().push_respawn(571, 7, incoming);
    legacy.write().unwrap().get_map_mut(571, 7).unwrap()
        .add_persisted_respawn_time_like_cpp(wow_map::map_manager::PersistedRespawnRowLikeCpp {
            object_type: SpawnObjectType::Creature, spawn_id: 2,
            respawn_time: 100, map_id: 571, instance_id: 7,
        });
    let fence = Mutex::new(()); let guard = fence.lock().unwrap();
    assert_eq!(WorldSession::transfer_legacy_respawns_to_canonical(
        &legacy, &canonical, MapKey::new(571, 7), incarnation, &guard,
    ), Err(RespawnOwnerTransferError::Admission(RespawnTransferError::WrongMap)));
    assert_eq!(legacy.read().unwrap().respawn_queue_len(571, 7), 1);
    assert_eq!(legacy.read().unwrap().persisted_respawn_rows_like_cpp(571, 7).len(), 1);
    assert!(canonical.lock().unwrap().find_map(571, 7).unwrap().map()
        .respawn_store_like_cpp().saved_rows().is_empty());
}

