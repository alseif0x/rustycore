//! Test-only construction through existing map/grid APIs, without actor clones.

use crate::manager::{
    MapManager, MapObjectTickContinuation, MapObjectUpdateSelectionLikeCpp,
    ObjectMapUpdateToken,
};
use crate::map::{CreatureActorAdmission, Map};
use crate::map_manager::WorldCreature;
use crate::{GridCoord, MIN_GRID_DELAY_MS};
use wow_core::{ObjectGuid, Position, guid::HighGuid};
use wow_entities::Creature;

pub(in crate::manager) fn new_actor(counter: i64, position: Position, active: bool) -> WorldCreature {
    let mut creature = Creature::new(false);
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 1, 0, 42, counter);
    creature.unit_mut().world_mut().object_mut().create(guid);
    creature.unit_mut().world_mut().set_map(1, 0).unwrap();
    creature.unit_mut().world_mut().relocate(position);
    creature.unit_mut().world_mut().set_active(active);
    creature.unit_mut().world_mut().object_mut().add_to_world();
    creature.unit_mut().set_max_health(100);
    creature.unit_mut().set_health(75);
    let data = WorldCreature::create_data_from_canonical_like_cpp(&creature);
    WorldCreature::from_canonical(creature, data)
}

pub(in crate::manager) fn place_in_loaded_cell(map: &mut Map, guid: ObjectGuid, position: Position) {
    let cell = crate::map::cell_from_world(position.x, position.y);
    map.ensure_grid_loaded(&cell);
    map.get_ngrid_mut(GridCoord::new(cell.grid_x(), cell.grid_y())).unwrap()
        .get_grid_type_mut(cell.cell_x(), cell.cell_y()).unwrap()
        .grid_objects.creatures.insert(guid);
}

pub(in crate::manager) fn insert_actor(
    manager: &mut MapManager,
    counter: i64,
    position: Position,
    active: bool,
) -> ObjectGuid {
    let actor = new_actor(counter, position, active);
    let guid = actor.guid();
    let map = manager.find_map_mut(1, 0).unwrap().map_mut();
    assert!(matches!(map.admit_creature_actor(actor).unwrap(), CreatureActorAdmission::Inserted { .. }));
    place_in_loaded_cell(map, guid, position);
    if active {
        assert!(map.add_to_active_like_cpp(guid).inserted_in_active_set);
    }
    guid
}

pub(in crate::manager) fn manager_with_actor(counter: i64) -> (MapManager, ObjectGuid) {
    let mut manager = MapManager::new(MIN_GRID_DELAY_MS, 200);
    manager.create_world_map(1, 0);
    let guid = insert_actor(&mut manager, counter, Position::xyz(10.0, 20.0, 30.0), true);
    manager.updater.activate(1);
    (manager, guid)
}

pub(in crate::manager) fn start(
    manager: &mut MapManager,
    diff_ms: u32,
    selection: MapObjectUpdateSelectionLikeCpp,
) -> (MapObjectTickContinuation, ObjectMapUpdateToken) {
    let plan = manager.begin_tick_like_cpp(diff_ms).into_started().unwrap();
    let mut tick = manager.begin_object_tick(plan).unwrap();
    let token = manager.prepare_next_object_map(&mut tick, selection).unwrap().unwrap();
    (tick, token)
}
