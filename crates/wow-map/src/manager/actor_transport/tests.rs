use super::*;
use crate::MIN_GRID_DELAY_MS;

#[path = "../../map/actor_transport/tests/fixtures.rs"]
mod fixtures;

fn owners(counter: i64) -> (MapManager, MapInstance, wow_core::ObjectGuid, u64) {
    let (map, source, guid, _) = fixtures::pair(counter, true);
    let mut manager = MapManager::new(MIN_GRID_DELAY_MS, 200);
    *manager.create_world_map(571, 7).map_mut() = map;
    let incarnation = manager
        .map_incarnation_like_cpp(MapKey::new(571, 7))
        .unwrap();
    (manager, source, guid, incarnation)
}

#[test]
fn busy_awaiting_sessions_rejects_before_source_extraction_or_target_promotion() {
    let (mut manager, mut source, guid, incarnation) = owners(851);
    let plan = manager.begin_tick_like_cpp(200).into_started().unwrap();
    let phase = manager.tick_coordination_like_cpp();
    assert_eq!(
        manager.transport_legacy_creature_ownership(MapKey::new(571, 7), incarnation, &mut source),
        Err(CreatureActorTransportError::MapBusy)
    );
    assert_eq!(manager.tick_coordination_like_cpp(), phase);
    assert!(
        source
            .grids
            .values()
            .any(|grid| grid.creatures.contains_key(&guid))
    );
    assert!(
        manager
            .find_map(571, 7)
            .unwrap()
            .map()
            .creature_actor(guid)
            .is_none()
    );
    drop(plan);
    assert_eq!(manager.tick_coordination_like_cpp(), phase);
}

#[test]
fn busy_resuming_with_map_inflight_is_not_abandoned_by_transport_rejection() {
    let (mut manager, mut source, guid, incarnation) = owners(852);
    let plan = manager.begin_tick_like_cpp(200).into_started().unwrap();
    let mut tick = manager.begin_object_tick(plan).unwrap();
    let token = manager
        .prepare_next_object_map(
            &mut tick,
            super::super::MapObjectUpdateSelectionLikeCpp::NearbyCells,
        )
        .unwrap()
        .unwrap();
    let phase = manager.tick_coordination_like_cpp();
    assert_eq!(
        manager.transport_legacy_creature_ownership(MapKey::new(571, 7), incarnation, &mut source),
        Err(CreatureActorTransportError::MapBusy)
    );
    assert!(
        source
            .grids
            .values()
            .any(|grid| grid.creatures.contains_key(&guid))
    );
    drop(token);
    drop(tick);
    assert_eq!(manager.tick_coordination_like_cpp(), phase);
}

#[test]
fn stale_or_missing_canonical_map_never_consumes_source_or_creates_a_map() {
    let (mut manager, mut source, guid, incarnation) = owners(853);
    let key = MapKey::new(571, 7);
    assert_eq!(
        manager.transport_legacy_creature_ownership(key, incarnation + 1, &mut source),
        Err(CreatureActorTransportError::StaleIncarnation)
    );
    assert_eq!(
        manager.transport_legacy_creature_ownership(MapKey::new(530, 7), incarnation, &mut source),
        Err(CreatureActorTransportError::MissingMap)
    );
    assert!(manager.find_map(530, 7).is_none());
    assert!(
        source
            .grids
            .values()
            .any(|grid| grid.creatures.contains_key(&guid))
    );
    assert!(
        manager
            .find_map(571, 7)
            .unwrap()
            .map()
            .creature_actor(guid)
            .is_none()
    );
}

#[test]
fn same_key_replacement_does_not_authorize_transport_using_the_old_incarnation() {
    let (mut manager, mut source, guid, incarnation) = owners(854);
    let key = MapKey::new(571, 7);
    // Model the existing manager removal bookkeeping, followed by its real
    // constructor that mints a new incarnation for the same key.
    manager.maps.remove(&key);
    manager.map_incarnations_like_cpp.remove(&key);
    manager.create_world_map(571, 7);
    assert_ne!(manager.map_incarnation_like_cpp(key), Some(incarnation));
    assert_eq!(
        manager.transport_legacy_creature_ownership(key, incarnation, &mut source),
        Err(CreatureActorTransportError::StaleIncarnation)
    );
    assert_eq!(
        manager.find_map(571, 7).unwrap().map().map_object_count(),
        0
    );
    assert!(
        source
            .grids
            .values()
            .any(|grid| grid.creatures.contains_key(&guid))
    );
}

#[test]
fn idle_owned_transport_preserves_manager_state_and_mints_one_actor_owner() {
    let (mut manager, mut source, guid, incarnation) = owners(855);
    let key = MapKey::new(571, 7);
    assert_eq!(
        manager
            .transport_legacy_creature_ownership(key, incarnation, &mut source)
            .unwrap()
            .transported,
        1
    );
    assert_eq!(
        manager.tick_coordination_like_cpp(),
        MapTickCoordinationStateLikeCpp::Idle
    );
    assert_eq!(manager.map_incarnation_like_cpp(key), Some(incarnation));
    assert!(source.grids.values().all(|grid| grid.creatures.is_empty()));
    assert!(
        manager
            .find_map(571, 7)
            .unwrap()
            .map()
            .creature_actor(guid)
            .is_some()
    );
}
