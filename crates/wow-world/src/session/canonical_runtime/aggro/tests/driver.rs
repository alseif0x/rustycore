//! Full APP/Map entry and failure transport, with independently built actors.
use super::*;
use std::sync::{Mutex, RwLock};

#[tokio::test]
async fn canonical_and_compatibility_driver_preserve_stop_start_bytes_and_order() {
    let victim = ObjectGuid::create_player(1, 814_011);
    let make_actor = || {
        let mut incoming = actor(814_010, Position::xyz(10.0, 20.0, 30.0));
        incoming.begin_move_spline_like_cpp(Position::xyz(20.0, 20.0, 30.0)).unwrap();
        incoming
    };
    let legacy = Arc::new(RwLock::new(crate::map_manager::MapManager::new()));
    legacy.write().unwrap().set_tick_owner(crate::map_manager::RuntimeTickOwner::GlobalLegacy);
    assert!(legacy.write().unwrap().add_creature(1, 0, 32, 32, make_actor()));
    let expected = run_legacy_creature_aggro_tick_once_with_config_like_cpp(&legacy, &[candidate(victim)], config());
    let mut manager = MapManager::new(MIN_GRID_DELAY_MS, 200);
    manager.create_world_map(1, 0);
    insert(&mut manager, make_actor(), true); add_player(&mut manager, victim);
    let (tick, mut token) = begin(&mut manager);
    let shared = Arc::new(Mutex::new(manager));
    let result = run_canonical_aggro(&shared, &tick, &mut token, vec![candidate(victim)], &config(), None).await.unwrap();
    assert_eq!(result.aggro_starts, 1);
    assert_eq!(result.movement_interrupts, expected.movement_interrupts);
    assert_eq!(result.commands[0].victim_guid, victim);
    assert_eq!(result.plan.events.len(), expected.plan.events.len());
    for (actual, expected) in result.plan.events.iter().zip(&expected.plan.events) {
        assert_eq!(actual.source_guid, expected.source_guid);
        assert_eq!(actual.packet_bytes, expected.packet_bytes);
    }
    assert!(shared.lock().unwrap().find_map(1, 0).unwrap().map().get_typed_player(victim).unwrap()
        .unit().subsystems().combat.is_in_combat_with(guid(814_010)));
}

#[tokio::test]
async fn foreign_driver_plan_returns_owned_candidates_without_publication() {
    let mut manager = MapManager::new(MIN_GRID_DELAY_MS, 200);
    manager.create_world_map(1, 0); insert(&mut manager, actor(814_020, Position::xyz(10.0, 20.0, 30.0)), true);
    let (_, mut token) = begin(&mut manager);
    let mut other = MapManager::new(MIN_GRID_DELAY_MS, 200);
    other.create_world_map(1, 0); insert(&mut other, actor(814_020, Position::xyz(10.0, 20.0, 30.0)), true);
    let (foreign_tick, _) = begin(&mut other);
    let shared = Arc::new(Mutex::new(manager));
    let failure = run_canonical_aggro(&shared, &foreign_tick, &mut token,
        vec![candidate(ObjectGuid::create_player(1, 814_021))], &config(), None).await.unwrap_err();
    assert!(failure.partial.plan.events.is_empty());
    let CanonicalAggroFailure::Prepare(failure) = failure.failure else { panic!("origin admission error"); };
    assert_eq!(failure.candidates.len(), 1);
    assert!(matches!(failure.error, wow_map::ActorAggroError::Access(wow_map::ActorTickAccessError::Tick(
        wow_map::ObjectMapTickError::OriginMismatch { .. }))));
}

#[test]
fn owned_pending_error_retains_partial_effects_and_query_panicked_remains_failstop() {
    let victim = ObjectGuid::create_player(1, 814_032);
    let mut manager = MapManager::new(MIN_GRID_DELAY_MS, 200);
    manager.create_world_map(1, 0);
    insert(&mut manager, actor(814_030, Position::xyz(10.0, 20.0, 30.0)), true);
    let mut assistant = actor(814_031, Position::xyz(500.0, 20.0, 30.0));
    assistant.creature.unit_mut().add_unit_state(UnitState::SIGHTLESS.bits());
    insert(&mut manager, assistant, false); add_player(&mut manager, victim);
    let (tick, mut token) = begin(&mut manager);
    let progress = catalogs::with_policies(&config(), |policy| manager.prepare_aggro(&tick, &mut token,
        vec![conversion::owned_candidate(candidate(victim))], conversion::settings(&config(), 1), true, policy)).unwrap();
    let ActorAggroProgress::Pending(request) = progress else { panic!("assistance LOS"); };
    let pointer = request.partial().effects.as_ptr();
    assert!(request.partial().effects.iter().any(|effect| matches!(&effect.kind, AggroEffectKind::AttackStart { .. })));
    let failure = CanonicalAggroError { partial: Default::default(), failure: CanonicalAggroFailure::MissingTerrain(request) };
    assert_eq!(failure.pending_effects().unwrap().effects.as_ptr(), pointer);
    let CanonicalAggroFailure::MissingTerrain(request) = failure.failure else { unreachable!() };
    let failure = CanonicalAggroError { partial: Default::default(), failure: CanonicalAggroFailure::ResumePoisoned {
        continuation: request.into_continuation(), response: true } };
    assert_eq!(failure.pending_effects().unwrap().effects.as_ptr(), pointer);
    drop(failure);
    assert!(manager.begin_tick_like_cpp(1).is_busy());
    let panic = CanonicalAggroError { partial: Default::default(), failure: CanonicalAggroFailure::QueryPanicked };
    assert!(panic.pending_effects().is_none());
    assert!(manager.begin_tick_like_cpp(1).is_busy());
}
