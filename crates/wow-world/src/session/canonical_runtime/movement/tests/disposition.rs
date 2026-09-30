//! Explicit I/O abandonment retains committed packets and never finishes a tick.

use super::*;
use super::super::{queries, packets, Arc, SharedCanonicalMapManager, MapObjectTickContinuation, ObjectMapUpdateToken};
use queries::ResolvedMovementQuery;
use wow_map::{ActorMovementPending, ActorMovementProgress, MapManager,
    MapObjectUpdateSelectionLikeCpp, MapTickCoordinationStateLikeCpp,
    MapCreatureUpdateOwnerLikeCpp, ObjectMapTickError, GridCoord, SpawnId, SpawnObjectType, MIN_GRID_DELAY_MS};
use wow_map::map::{Map, LoadedGridRespawnRecordsLikeCpp, cell_from_world};
use crate::map_manager::{WorldCreature, RuntimePlan};
use wow_core::{ObjectGuid, Position, guid::HighGuid};
use wow_entities::CreatureAiState;
use std::sync::Mutex;

mod replies;
mod fail_stop;

fn actor(counter: i64) -> WorldCreature {
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 1, 0, 9999, counter);
    let mut actor = WorldCreature::new(guid, 9999, Position::xyz(10.0, 10.0, 0.0),
        100, 2, 3, 5, 20.0, 100, 14, 0, 0);
    actor.creature.unit_mut().world_mut().set_map(1, 0).unwrap();
    actor.creature.set_ai_position(Position::xyz(30.0, 10.0, 0.0));
    actor.creature.set_ai_state(CreatureAiState::Returning);
    actor.seed_runtime_rng_like_cpp(0x5757);
    actor
}

fn insert(manager: &mut MapManager, mut incoming: WorldCreature) {
    let guid = incoming.guid();
    let position = incoming.position();
    incoming.creature.unit_mut().world_mut().set_active(true);
    incoming.creature.unit_mut().world_mut().object_mut().add_to_world();
    let map = manager.find_map_mut(1, 0).unwrap().map_mut();
    map.test_fixture_admit_creature_actor(incoming);
    let cell = cell_from_world(position.x, position.y);
    map.ensure_grid_loaded(&cell);
    map.get_ngrid_mut(GridCoord::new(cell.grid_x(), cell.grid_y())).unwrap()
        .get_grid_type_mut(cell.cell_x(), cell.cell_y()).unwrap()
        .grid_objects.creatures.insert(guid);
    assert!(map.add_to_active_like_cpp(guid).inserted_in_active_set);
}

fn fixture(counter: i64) -> (SharedCanonicalMapManager, MapObjectTickContinuation, ObjectMapUpdateToken, ObjectGuid) {
    let incoming = actor(counter);
    let guid = incoming.guid();
    let mut manager = MapManager::new(MIN_GRID_DELAY_MS, 1);
    manager.create_world_map(1, 0);
    insert(&mut manager, incoming);
    let plan = manager.begin_tick_like_cpp(200).into_started().unwrap();
    let mut tick = manager.begin_object_tick(plan).unwrap();
    let token = manager.prepare_next_object_map(&mut tick, MapObjectUpdateSelectionLikeCpp::NearbyCells).unwrap().unwrap();
    (Arc::new(Mutex::new(manager)), tick, token, guid)
}

fn pending(manager: &SharedCanonicalMapManager, tick: &MapObjectTickContinuation,
    token: &mut ObjectMapUpdateToken, guid: ObjectGuid, stage: u8) -> ActorMovementPending
{
    let progress = manager.lock().unwrap().prepare_movement(tick, token, guid, None, stage != 0, |_, _| true).unwrap();
    let ActorMovementProgress::Pending(request) = progress else { panic!("returning fixture suspends"); };
    if stage != 2 { return request; }
    let ActorMovementPending::StaticHeight(request) = request else { panic!("static first"); };
    let (_, continuation) = request.into_parts();
    match manager.lock().unwrap().resume_movement_static_height(tick, token, continuation, wow_entities::INVALID_HEIGHT) {
        Ok(ActorMovementProgress::Pending(request @ ActorMovementPending::GridHeight(_))) => request,
        _ => panic!("lazy grid fallback"),
    }
}

fn partial() -> CanonicalMovementOutcome {
    let mut previous = actor(650_000);
    previous.creature.unit_mut().set_health(75);
    let health = previous.creature.unit().values_update();
    let movement = previous.step_movement(23, None, None, |_, _| false, |_, _, _, _| None);
    let mut plan = RuntimePlan { events: Vec::new() };
    assert!(packets::append_completed_movement(&mut plan, previous.guid(), 1, 0,
        previous.position(), previous.visibility_range_like_cpp(), movement, Some(health),
        packets::MovementTrace { entry: previous.entry(), map_id: previous.map_id(), state: previous.state() }));
    assert_eq!(plan.events.len(), 2);
    CanonicalMovementOutcome { creatures_seen: 2, movement_packets: 1, plan }
}

fn packet_buffers(partial: &CanonicalMovementOutcome) -> Vec<(usize, Vec<u8>)> {
    partial.plan.events.iter().map(|event| (event.packet_bytes.as_ptr() as usize, event.packet_bytes.clone())).collect()
}

fn assert_partial(partial: &CanonicalMovementOutcome, buffers: &[(usize, Vec<u8>)]) {
    assert_eq!((partial.creatures_seen, partial.movement_packets), (2, 1));
    assert_eq!(partial.plan.events.len(), buffers.len());
    for (event, (pointer, bytes)) in partial.plan.events.iter().zip(buffers) {
        assert_eq!(event.packet_bytes.as_ptr() as usize, *pointer);
        assert_eq!(&event.packet_bytes, bytes);
    }
    assert!(matches!(partial.plan.events[0].recipients, crate::map_manager::RecipientRule::NearbyVisibleDurable { .. }));
    assert!(matches!(partial.plan.events[1].recipients, crate::map_manager::RecipientRule::NearbyVisible { .. }));
}

type LoadRecord = fn(&mut Map, SpawnObjectType, SpawnId) -> Option<LoadedGridRespawnRecordsLikeCpp>;

fn assert_busy(manager: &SharedCanonicalMapManager) {
    let mut manager = manager.lock().unwrap();
    assert!(matches!(manager.tick_coordination_like_cpp(), MapTickCoordinationStateLikeCpp::Resuming(_)));
    assert!(manager.begin_tick_like_cpp(1).is_busy());
}

#[tokio::test]
async fn missing_terrain_disposer_retains_partial_packets_and_abandons_only_owned_request() {
    for stage in [1, 2] {
        let (manager, tick, mut token, guid) = fixture(650_001 + i64::from(stage));
        let request = pending(&manager, &tick, &mut token, guid, stage);
        let failure = match queries::resolve(guid, request, None, None).await {
            Err(failure @ MovementFailure::MissingTerrain(_)) => failure,
            _ => panic!("missing terrain retains the unlaunched request"),
        };
        let partial = partial();
        let buffers = packet_buffers(&partial);
        let error = CanonicalMovementError::new(partial, failure);
        let abandoned = error.dispose_owned_request(&manager, &tick, &mut token)
            .unwrap_or_else(|error| panic!("{error}"));
        assert_eq!(abandoned.abandonment, CanonicalMovementAbandonment::UnlaunchedRequestDiscarded);
        assert_partial(&abandoned.partial, &buffers);
        assert_busy(&manager);
        let progress = manager.lock().unwrap().prepare_movement(&tick, &mut token, guid, None, false, |_, _| true);
        assert!(progress.is_ok(), "the original slot was explicitly released");
        drop(progress);
        assert_busy(&manager);
    }
}

#[tokio::test]
async fn missing_terrain_disposal_rejection_keeps_request_and_partial_plan_for_original_token() {
    let (manager, tick, mut token, guid) = fixture(650_004);
    let (_, foreign_tick, mut foreign_token, _) = fixture(650_004);
    let request = pending(&manager, &tick, &mut token, guid, 1);
    let failure = match queries::resolve(guid, request, None, None).await {
        Err(failure @ MovementFailure::MissingTerrain(_)) => failure,
        _ => panic!("missing terrain"),
    };
    let partial = partial();
    let buffers = packet_buffers(&partial);
    let error = CanonicalMovementError::new(partial, failure);
    let error = match error.dispose_owned_request(&manager, &foreign_tick, &mut token) {
        Err(error) => error, Ok(_) => panic!("foreign tick cannot discard"),
    };
    let error = match error.dispose_owned_request(&manager, &tick, &mut foreign_token) {
        Err(error) => error, Ok(_) => panic!("foreign token cannot discard"),
    };
    assert!(matches!(&error.failure, MovementFailure::MissingTerrain(ActorMovementPending::StaticHeight(_))));
    assert_partial(&error.partial, &buffers);
    let abandoned = error.dispose_owned_request(&manager, &tick, &mut token).unwrap_or_else(|error| panic!("{error}"));
    assert_eq!(abandoned.abandonment, CanonicalMovementAbandonment::UnlaunchedRequestDiscarded);
    assert_partial(&abandoned.partial, &buffers);
    assert_busy(&manager);
}
