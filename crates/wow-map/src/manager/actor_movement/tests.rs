//! Real stored motors exercise the dormant manager operation, without Clone.

use super::*;
use crate::manager::{MapCreatureUpdateOwnerLikeCpp, MapObjectUpdateSelectionLikeCpp,
    MapTickCoordinationStateLikeCpp, ObjectMapTickError, MIN_GRID_DELAY_MS};
use crate::map::{CreatureActorAdmission, LoadedGridRespawnRecordsLikeCpp, Map};
use crate::map_manager::{CreatureMovementSource, WorldCreature};
use crate::{MapKey, SpawnId, SpawnObjectType};
use wow_core::Position;
use wow_entities::{Creature, CreatureAiState, MapObjectRecord};
use wow_movement::{WaypointNode, WaypointPath};
use wow_recastdetour::{DetourPathType, DetourPointPath};

#[path = "../../map_manager/movement/step/fixtures.rs"]
mod old_fixtures;
mod equivalence;
mod rejection;
mod complete;
mod disposition;

fn target() -> ChaseTargetSnapshotLikeCpp {
    ChaseTargetSnapshotLikeCpp {
        guid: ObjectGuid::create_player(1, 801),
        position: Position::xyz(50.0, 10.0, 0.0),
        combat_reach: 1.5, in_world: true, in_water: None,
    }
}

fn actor(source: CreatureMovementSource, counter: i64) -> WorldCreature {
    let mut actor = old_fixtures::make_test_world_creature(old_fixtures::test_creature_guid(counter));
    actor.creature.unit_mut().world_mut().set_map(1, 0).unwrap();
    actor.creature.unit_mut().world_mut().phase_shift_mut().insert(42);
    actor.creature.unit_mut().world_mut().phase_shift_mut().add_visible_map_id_like_cpp(609, 1);
    actor.creature.unit_mut().world_mut().object_mut().add_to_world();
    actor.seed_runtime_rng_like_cpp(0x5757);
    match source {
        CreatureMovementSource::Home => {
            actor.creature.set_ai_position(Position::xyz(30.0, 10.0, 0.0));
            actor.creature.set_ai_state(CreatureAiState::Returning);
        }
        CreatureMovementSource::Random => {
            actor.creature.set_default_movement_type_runtime_like_cpp(wow_entities::MovementGeneratorType::Random);
            actor.creature.ai_ownership_mut().wander_radius = 3.0;
        }
        CreatureMovementSource::Waypoint => {
            actor.initialize_default_waypoint_movement_like_cpp(Some(WaypointPath::new(77, vec![
                WaypointNode::new(10, 30.0, 10.0, 0.0),
                WaypointNode::new(20, 40.0, 10.0, 0.0),
            ])));
        }
        CreatureMovementSource::Chase => actor.enter_combat(target().guid),
    }
    actor
}

fn stored(manager: &MapManager, guid: ObjectGuid) -> &WorldCreature {
    manager.find_map(1, 0).unwrap().map().creature_actor(guid).unwrap()
}

fn insert_actor(manager: &mut MapManager, incoming: WorldCreature, cell_resident: bool) {
    let map = manager.find_map_mut(1, 0).unwrap().map_mut();
    if cell_resident {
        // Reuse AddToMap's existing cell-registration fixture. Erasing this
        // temporary Record leaves its cell membership in the current contract;
        // the one real actor is admitted only after that body is gone. This is
        // test setup, not a production admission/ownership migration.
        let mut registration = Creature::new(false);
        registration.unit_mut().world_mut().object_mut().create(incoming.guid());
        registration.unit_mut().world_mut().set_map(1, 0).unwrap();
        registration.unit_mut().world_mut().relocate(incoming.position());
        map.add_map_object_record_to_map_like_cpp(MapObjectRecord::new_creature(registration).unwrap()).unwrap();
        drop(map.remove_map_object(incoming.guid()).unwrap());
    }
    assert!(matches!(map.admit_creature_actor(incoming).unwrap(), CreatureActorAdmission::Inserted { .. }));
}

fn fixture(source: CreatureMovementSource, counter: i64, diff: u32)
    -> (MapManager, MapObjectTickContinuation, ObjectMapUpdateToken, ObjectGuid)
{
    let mut manager = MapManager::new(MIN_GRID_DELAY_MS, 200);
    manager.create_world_map(1, 0);
    let incoming = actor(source, counter);
    let guid = incoming.guid();
    insert_actor(&mut manager, incoming, true);
    let (tick, token) = begin(&mut manager, diff, MapObjectUpdateSelectionLikeCpp::WholeTypedStores);
    (manager, tick, token, guid)
}

fn begin(manager: &mut MapManager, diff: u32, selection: MapObjectUpdateSelectionLikeCpp)
    -> (MapObjectTickContinuation, ObjectMapUpdateToken)
{
    let plan = manager.begin_tick_like_cpp(diff).into_started().unwrap();
    let mut tick = manager.begin_object_tick(plan).unwrap();
    let token = manager.prepare_next_object_map(&mut tick, selection).unwrap().unwrap();
    (tick, token)
}

fn detour(query: &CreaturePathQueryLikeCpp) -> DetourPolyPath {
    DetourPolyPath {
        poly_refs: vec![11, 22, 33],
        point_path: DetourPointPath {
            actual_end: [query.destination.x, query.destination.y, query.destination.z],
            points: vec![
                [query.start.x, query.start.y, query.start.z],
                [(query.start.x + query.destination.x) / 2.0, query.start.y + 1.0, query.start.z],
                [query.destination.x, query.destination.y, query.destination.z],
            ],
            path_type: DetourPathType::NORMAL,
        },
        start_far_from_poly: false, end_far_from_poly: false,
    }
}

fn path_request(progress: ActorMovementProgress) -> (CreaturePathQueryLikeCpp, ActorPathContinuation) {
    match progress {
        ActorMovementProgress::Pending(ActorMovementPending::Path(request)) => request.into_parts(),
        _ => panic!("fixture requires a path request"),
    }
}

type LoadRecord = fn(&mut Map, SpawnObjectType, SpawnId) -> Option<LoadedGridRespawnRecordsLikeCpp>;
