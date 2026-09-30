use super::*;
use std::sync::Mutex;
use wow_map::{
    ActorMovementPending, GridCoord, MapCreatureUpdateOwnerLikeCpp, MapManager,
    MapObjectTickContinuation, MapObjectUpdateSelectionLikeCpp, MapTickCoordinationStateLikeCpp,
    ObjectMapTickError, ObjectMapUpdateToken, SpawnId, SpawnObjectType, MIN_GRID_DELAY_MS,
};
use wow_map::map::{LoadedGridRespawnRecordsLikeCpp, Map, cell_from_world};
use wow_entities::MapObjectRecord;

// A test-fixtures-only admission constructor moves a real stored motor. Cell
// setup uses the same grid APIs as the manager's own selected-Actor cases.
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

fn begin(manager: &mut MapManager, diff: u32)
    -> (MapObjectTickContinuation, ObjectMapUpdateToken)
{
    let plan = manager.begin_tick_like_cpp(diff).into_started().unwrap();
    let mut tick = manager.begin_object_tick(plan).unwrap();
    let token = manager.prepare_next_object_map(
        &mut tick, MapObjectUpdateSelectionLikeCpp::NearbyCells,
    ).unwrap().unwrap();
    (tick, token)
}

fn fixture(incoming: WorldCreature, diff: u32)
    -> (SharedCanonicalMapManager, MapObjectTickContinuation, ObjectMapUpdateToken)
{
    let mut manager = MapManager::new(MIN_GRID_DELAY_MS, 1);
    manager.create_world_map(1, 0);
    insert(&mut manager, incoming);
    let (tick, token) = begin(&mut manager, diff);
    (Arc::new(Mutex::new(manager)), tick, token)
}

fn returning(counter: i64) -> WorldCreature {
    let mut incoming = actor(counter);
    incoming.creature.set_ai_position(Position::xyz(30.0, 10.0, 0.0));
    incoming.creature.set_ai_state(CreatureAiState::Returning);
    incoming
}

type LoadRecord = fn(&mut Map, SpawnObjectType, SpawnId) -> Option<LoadedGridRespawnRecordsLikeCpp>;

fn assert_pending_cannot_finish(
    manager: &SharedCanonicalMapManager,
    tick: &mut MapObjectTickContinuation,
    token: ObjectMapUpdateToken,
    guid: ObjectGuid,
) {
    let mut manager = manager.lock().unwrap();
    let failure = match manager.try_finish_object_map::<LoadRecord>(
        tick, token, None, None, MapCreatureUpdateOwnerLikeCpp::ExternalRuntime,
    ) {
        Err(failure) => failure,
        Ok(_) => panic!("an unresolved movement operation cannot run the tail"),
    };
    assert!(matches!(failure.0, ObjectMapTickError::ActorOperationInFlight { guid: pending } if pending == guid));
    drop(failure.1);
    assert!(matches!(manager.tick_coordination_like_cpp(), MapTickCoordinationStateLikeCpp::Resuming(_)));
    assert!(manager.begin_tick_like_cpp(1).is_busy());
}

#[tokio::test]
async fn selected_canonical_driver_matches_original_launch_bytes() {
    let mut original = returning(620_001);
    let expected = step_creature_movement_like_cpp(
        &mut original, guid(620_001), &config(), None, None, None, 23,
    ).unwrap();
    let (manager, tick, mut token) = fixture(returning(620_001), 23);
    let outcome = run_canonical_movement(
        &manager, &tick, &mut token, &config(), None, None, HashMap::new(),
    ).await.unwrap();
    assert_eq!(outcome.creatures_seen, 1);
    assert_eq!(outcome.movement_packets, 1);
    assert_eq!(outcome.plan.events.len(), 1);
    assert_eq!(outcome.plan.events[0].packet_bytes, expected);
}

#[tokio::test]
async fn owned_path_reply_matches_original_missing_navmesh_launch() {
    let enabled = MMapRuntimeConfigLikeCpp { enabled: true, ..Default::default() };
    let mut original = returning(620_008);
    let expected = step_creature_movement_like_cpp(
        &mut original, guid(620_008), &enabled, None, None, None, 200,
    ).unwrap();
    let (manager, tick, mut token) = fixture(returning(620_008), 200);
    let outcome = run_canonical_movement(
        &manager, &tick, &mut token, &enabled, None, None, HashMap::new(),
    ).await.unwrap();
    assert_eq!(outcome.creatures_seen, 1);
    assert_eq!(outcome.movement_packets, 1);
    assert_eq!(outcome.plan.events[0].packet_bytes, expected);
}

#[tokio::test]
async fn real_home_health_pending_is_taken_once_by_canonical_completion() {
    let mut incoming = returning(620_009);
    incoming.creature.unit_mut().set_health(75);
    incoming.step_movement(200, None, None, |_, _| false, |_, _, _, _| None);
    incoming.step_movement(10_000, None, None, |_, _| false, |_, _, _, _| None);
    assert_eq!(incoming.state(), CreatureAiState::Idle);
    assert_eq!(incoming.creature.current_health(), 100);
    let (manager, mut tick, mut token) = fixture(incoming, 1);
    let first = run_canonical_movement(
        &manager, &tick, &mut token, &config(), None, None, HashMap::new(),
    ).await.unwrap();
    assert_eq!(first.movement_packets, 0);
    assert_eq!(first.plan.events.len(), 1);
    assert!(matches!(first.plan.events[0].recipients, RecipientRule::NearbyVisibleDurable { .. }));
    {
        let mut manager = manager.lock().unwrap();
        assert!(manager.finish_object_map::<LoadRecord>(
            &mut tick, token, None, None, MapCreatureUpdateOwnerLikeCpp::ExternalRuntime,
        ).is_ok());
        assert!(manager.finalize_object_tick(tick).is_ok());
        (tick, token) = begin(&mut manager, 1);
    }
    let second = run_canonical_movement(
        &manager, &tick, &mut token, &config(), None, None, HashMap::new(),
    ).await.unwrap();
    assert!(second.plan.events.is_empty(), "the health flag is not replayed on the next tick");
}

#[test]
fn registry_target_precedes_cross_selection_creature_and_liquid_stays_unknown() {
    let mut incoming = actor(620_002);
    let victim = actor(620_003);
    incoming.enter_combat(victim.guid());
    let (manager, _tick, token) = fixture(incoming, 200);
    let mut manager = manager.lock().unwrap();
    // This victim is deliberately admitted after the NearbyCells plan was
    // frozen. Victim reads may cross selection; primary updates may not.
    let record = MapObjectRecord::new_creature(victim.creature).unwrap();
    manager.find_map_mut(1, 0).unwrap().map_mut()
        .add_map_object_record_to_map_like_cpp(record).unwrap();
    let registry = ChaseTargetSnapshotLikeCpp {
        guid: guid(620_003), position: Position::xyz(75.0, 25.0, 3.0),
        combat_reach: 7.0, in_world: false, in_water: Some(true),
    };
    let mut facts = HashMap::new();
    facts.insert((1, 0, guid(620_003)), registry);
    let preferred = movement_target(&manager, token.key(), 1, guid(620_002), &facts).unwrap();
    assert_eq!(preferred.position, registry.position);
    assert_eq!(preferred.combat_reach, 7.0);
    assert!(!preferred.in_world);
    assert_eq!(preferred.in_water, Some(true));
    facts.clear();
    let fallback = movement_target(&manager, token.key(), 1, guid(620_002), &facts).unwrap();
    assert_eq!(fallback.guid, guid(620_003));
    assert_eq!(fallback.position, Position::xyz(10.0, 10.0, 0.0));
    assert!(fallback.in_world);
    assert_eq!(fallback.in_water, None);
}

#[test]
fn waiting_waypoint_finishes_without_query() {
    let mut incoming = actor(620_004);
    incoming.initialize_default_waypoint_movement_like_cpp(Some(wow_movement::WaypointPath::new(
        77, vec![wow_movement::WaypointNode::new(10, 30.0, 10.0, 0.0)],
    )));
    let (manager, tick, mut token) = fixture(incoming, 1);
    let progress = manager.lock().unwrap().prepare_movement(
        &tick, &mut token, guid(620_004), None, true,
        |_, _| true,
    ).unwrap();
    match progress {
        ActorMovementProgress::Complete(completion) => {
            assert!(completion.movement.is_none());
            assert!(completion.home_health_update.is_none());
        }
        ActorMovementProgress::Pending(_) => panic!("waiting must not create I/O"),
    }
}

#[tokio::test]
async fn replacement_identity_rejects_owned_response_and_keeps_pending_slot() {
    let (manager, mut tick, mut token) = fixture(returning(620_005), 200);
    let pending = match manager.lock().unwrap().prepare_movement(
        &tick, &mut token, guid(620_005), None, false, |_, _| true,
    ).unwrap() {
        ActorMovementProgress::Pending(ActorMovementPending::Path(request)) => ActorMovementPending::Path(request),
        _ => panic!("home launch requires its path request"),
    };
    {
        let mut manager = manager.lock().unwrap();
        let removed = manager.find_map_mut(1, 0).unwrap().map_mut().remove_map_object(guid(620_005)).unwrap();
        insert(&mut manager, returning(620_005));
        drop(removed);
    }
    let response = match queries::resolve(guid(620_005), pending, None, None).await {
        Ok(response) => response,
        Err(_) => panic!("missing navmesh retains the original shortcut behavior"),
    };
    let failure = match queries::resume(&manager, &tick, &mut token, response) {
        Err(failure) => failure,
        Ok(_) => panic!("same GUID does not mean same actor admission"),
    };
    assert!(matches!(failure, MovementFailure::Resume { error: ActorMovementError::WitnessMismatch { .. }, .. }));
    let position = manager.lock().unwrap().find_map(1, 0).unwrap().map()
        .with_creature_like_cpp(guid(620_005), |creature| creature.unit().world().position()).unwrap();
    assert_eq!(position, Position::xyz(30.0, 10.0, 0.0));
    assert_pending_cannot_finish(&manager, &mut tick, token, guid(620_005));
}

#[test]
fn dropping_query_cannot_complete_the_map_or_return_manager_to_idle() {
    let (manager, mut tick, mut token) = fixture(returning(620_006), 200);
    let pending = manager.lock().unwrap().prepare_movement(
        &tick, &mut token, guid(620_006), None, false, |_, _| true,
    ).unwrap();
    assert!(matches!(&pending, ActorMovementProgress::Pending(_)));
    drop(pending);
    assert_pending_cannot_finish(&manager, &mut tick, token, guid(620_006));
}

#[tokio::test]
async fn foreign_origin_driver_returns_no_packets_or_mutation() {
    let (first, _tick, mut token) = fixture(returning(620_007), 200);
    let (_second, foreign_tick, _foreign_token) = fixture(returning(620_007), 200);
    let error = run_canonical_movement(
        &first, &foreign_tick, &mut token, &config(), None, None, HashMap::new(),
    ).await.err().expect("a foreign tick cannot start the operation");
    assert!(matches!(error.actor_error(), Some(ActorMovementError::Tick(ObjectMapTickError::OriginMismatch { .. }))));
    assert_eq!(error.partial.creatures_seen, 0);
    assert!(error.partial.plan.events.is_empty());
}
