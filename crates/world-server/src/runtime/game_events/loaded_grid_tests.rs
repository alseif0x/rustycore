//! Actual loaded-grid summary consumers against canonical Map actors.

use super::initialize_loaded_grid_primary_records_default_motion_like_cpp;
use crate::runtime::map::LoadedGridAreaTriggerLoadSummaryLikeCpp;
use crate::spawn_store_loader::WaypointPathStoreLikeCpp;
use wow_constants::{DeathState, TypeId, TypeMask};
use wow_core::{ObjectGuid, Position, guid::HighGuid};
use wow_entities::{
    AccessorObjectKind, Creature, GameObject, MapObjectRecord, MovementGeneratorType, WorldObject,
};
use wow_map::map::{LoadedGridAreaTriggerRecordsSummaryLikeCpp, LoadedGridRespawnRecordsLikeCpp};
use wow_map::spawn::{
    SpawnData, SpawnGroupTemplateData, SpawnObjectType, SpawnPosition, SpawnStore,
};

fn object_guid(high: HighGuid, counter: i64) -> ObjectGuid {
    ObjectGuid::create_world_object(high, 0, 1, 571, 0, 42, counter)
}

fn creature_record(counter: i64, movement: MovementGeneratorType) -> MapObjectRecord {
    let mut creature = Creature::new(false);
    creature
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(object_guid(HighGuid::Creature, counter));
    creature.unit_mut().world_mut().object_mut().set_entry(42);
    creature.unit_mut().world_mut().set_map(571, 0).unwrap();
    creature.unit_mut().world_mut().relocate(Position::ZERO);
    creature.unit_mut().set_death_state(DeathState::Alive);
    creature.unit_mut().set_max_health(100);
    creature.unit_mut().set_health(100);
    creature.set_default_movement_type_runtime_like_cpp(movement);
    creature.load_path_like_cpp(10);
    MapObjectRecord::new_creature(creature).unwrap()
}

fn stub_record(counter: i64) -> MapObjectRecord {
    let mut object = WorldObject::new(false, TypeId::Unit, TypeMask::UNIT);
    object
        .object_mut()
        .create(object_guid(HighGuid::Creature, counter));
    object.set_map(571, 0).unwrap();
    object.relocate(Position::ZERO);
    MapObjectRecord::new(AccessorObjectKind::Creature, object).unwrap()
}

fn gameobject_record(counter: i64) -> MapObjectRecord {
    let mut object = GameObject::new();
    object
        .world_mut()
        .object_mut()
        .create(object_guid(HighGuid::GameObject, counter));
    object.world_mut().set_map(571, 0).unwrap();
    object.world_mut().relocate(Position::ZERO);
    MapObjectRecord::new_game_object(object).unwrap()
}

fn materialize(
    map: &mut wow_map::ManagedMapInnerLikeCpp,
    records: Vec<MapObjectRecord>,
) -> LoadedGridAreaTriggerRecordsSummaryLikeCpp {
    let mut spawns = SpawnStore::new();
    for spawn_id in 1..=records.len() as u64 {
        spawns.add_area_trigger_spawn(&SpawnData {
            object_type: SpawnObjectType::AreaTrigger,
            spawn_id,
            map_id: 571,
            db_data: true,
            spawn_group: SpawnGroupTemplateData::legacy_group(),
            id: 42,
            spawn_point: SpawnPosition::new(0.0, 0.0, 0.0, 0.0),
            phase_use_flags: 0,
            phase_id: 0,
            phase_group: 0,
            terrain_swap_map: 0,
            pool_id: 0,
            spawn_time_secs: 0,
            spawn_difficulties: vec![map.spawn_mode()],
            script_id: 0,
            string_id: String::new(),
        });
    }
    map.ensure_grid_loaded(&wow_map::cell_from_world(0.0, 0.0));
    let mut records = records.into_iter();
    map.load_loaded_grid_area_trigger_records_like_cpp(
        wow_map::GridCoord::new(32, 32),
        &spawns,
        |_, object_type, _| {
            assert_eq!(object_type, SpawnObjectType::AreaTrigger);
            Some(LoadedGridRespawnRecordsLikeCpp::primary_only(
                records.next().unwrap(),
            ))
        },
    )
}

fn waypoint_paths() -> WaypointPathStoreLikeCpp {
    let (store, report) = WaypointPathStoreLikeCpp::from_rows_like_cpp(
        [30, 40].map(
            |path_id| wow_persistence::WaypointPathPersistenceRowLikeCpp {
                path_id,
                move_type: 1,
                flags: 0,
            },
        ),
        [30, 40].map(
            |path_id| wow_persistence::WaypointPathNodePersistenceRowLikeCpp {
                path_id,
                node_id: 7,
                x: 11.0,
                y: 12.0,
                z: 13.0,
                orientation: Some(1.5),
                delay: 250,
            },
        ),
    );
    assert_eq!(report.paths_loaded, 2);
    assert_eq!(report.nodes_loaded, 2);
    store
}

#[test]
fn loaded_grid_motion_facts_resolve_current_canonical_creature_and_keep_typed_exclusion() {
    let mut manager = wow_map::MapManager::default();
    let map = manager.create_world_map(571, 0).map_mut();
    let record = creature_record(91_001, MovementGeneratorType::Waypoint);
    let allocation = record.creature().unwrap() as *const Creature as usize;
    let summary = materialize(
        map,
        vec![
            record,
            stub_record(91_002),
            creature_record(91_003, MovementGeneratorType::Idle),
            creature_record(91_004, MovementGeneratorType::Waypoint),
            gameobject_record(91_005),
        ],
    );
    assert_eq!(summary.loaded_grid_primary_records.len(), 5);
    assert_eq!(summary.add_to_map_errors, 0);
    assert_eq!(
        summary.loaded_grid_primary_records[1].typed_creature_guid(),
        None
    );
    let active = object_guid(HighGuid::Creature, 91_001);
    let excluded = object_guid(HighGuid::Creature, 91_002);
    let idle = object_guid(HighGuid::Creature, 91_003);
    let removed = object_guid(HighGuid::Creature, 91_004);
    let canonical = map.get_typed_creature_mut(active).unwrap();
    assert_eq!(canonical as *const Creature as usize, allocation);
    canonical.load_path_like_cpp(30);
    canonical.unit_mut().subsystems_mut().motion.stopped = false;

    // The original record was a stub. Even a later typed replacement must not
    // turn that successful-add fact into a typed Creature fact retroactively.
    let mut replacement = creature_record(91_002, MovementGeneratorType::Waypoint);
    let canonical = replacement.creature_mut().unwrap();
    canonical.load_path_like_cpp(40);
    canonical.unit_mut().subsystems_mut().motion.stopped = false;
    map.insert_map_object_record(replacement).unwrap();
    map.get_typed_creature_mut(idle)
        .unwrap()
        .unit_mut()
        .subsystems_mut()
        .motion
        .stopped = false;
    map.remove_from_map_like_cpp(removed, true).unwrap();

    let mut facts = summary.loaded_grid_primary_records;
    facts.push(facts[0]);
    let initialized = initialize_loaded_grid_primary_records_default_motion_like_cpp(
        map,
        &waypoint_paths(),
        &facts,
    );

    assert_eq!(initialized, 2);
    assert!(
        !map.get_typed_creature_mut(excluded)
            .unwrap()
            .unit()
            .subsystems()
            .motion
            .stopped
    );
    assert!(
        !map.get_typed_creature_mut(idle)
            .unwrap()
            .unit()
            .subsystems()
            .motion
            .stopped
    );
    assert!(map.get_typed_creature_mut(removed).is_none());
    let canonical = map.get_typed_creature_mut(active).unwrap();
    assert_eq!(canonical as *const Creature as usize, allocation);
    assert!(canonical.unit().subsystems().motion.stopped);
    assert!(matches!(
        wow_world::creature_movement::CreatureMovementLikeCpp::canonical_like_cpp(canonical)
            .update_default_waypoint_movement_like_cpp(
                wow_movement::WAYPOINT_INITIAL_DELAY_MS_LIKE_CPP as u32
            ),
        wow_movement::WaypointMovementAction::Launch(launch)
            if launch.path_id == 30 && launch.node_id == 7
                && launch.destination == Position::new(11.0, 12.0, 13.0, 0.0)
    ));
}

#[test]
fn loaded_grid_area_trigger_accumulation_keeps_all_object_guids_in_order() {
    let mut manager = wow_map::MapManager::default();
    let map = manager.create_world_map(571, 0).map_mut();
    let grid = materialize(
        map,
        vec![
            creature_record(92_001, MovementGeneratorType::Idle),
            gameobject_record(92_002),
            stub_record(92_003),
        ],
    );
    assert_eq!(grid.metadata_entries, 3);
    assert_eq!(grid.loaded_grid_primary_records.len(), 3);
    let mut total = LoadedGridAreaTriggerLoadSummaryLikeCpp::default();
    total.accumulate(&grid);
    total.accumulate(&grid);
    assert_eq!(total.metadata_entries, 6);
    assert_eq!(total.loaded_grid_primary_records, 6);
    assert_eq!(total.add_to_map_errors, 0);
    assert_eq!(
        total.loaded_area_trigger_guids,
        vec![
            object_guid(HighGuid::Creature, 92_001),
            object_guid(HighGuid::GameObject, 92_002),
            object_guid(HighGuid::Creature, 92_003),
            object_guid(HighGuid::Creature, 92_001),
            object_guid(HighGuid::GameObject, 92_002),
            object_guid(HighGuid::Creature, 92_003),
        ]
    );
}
