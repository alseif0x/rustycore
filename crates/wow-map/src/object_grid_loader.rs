//! Grid object loading from preloaded spawn stores.
//!
//! C++ references:
//! - `game/Grids/ObjectGridLoader.h`
//! - `game/Grids/ObjectGridLoader.cpp`

use std::collections::BTreeMap;

use wow_core::guid::{HighGuid, ObjectGuid};

use crate::cell::Cell;
use crate::coords::MAX_NUMBER_OF_CELLS;
use crate::grid::NGrid;
use crate::map::GridLifecycle;
use crate::spawn::{
    Difficulty, SpawnData, SpawnGridLoadStateLikeCpp, SpawnId, SpawnObjectType, SpawnStore,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CorpseGridObject {
    pub guid: ObjectGuid,
    pub is_world_object: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CorpseCellStore {
    corpses_by_cell: BTreeMap<u32, Vec<CorpseGridObject>>,
}

impl CorpseCellStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_corpse(&mut self, cell_id: u32, corpse: CorpseGridObject) {
        self.corpses_by_cell
            .entry(cell_id)
            .or_default()
            .push(corpse);
    }

    pub fn corpses_in_cell(&self, cell_id: u32) -> &[CorpseGridObject] {
        self.corpses_by_cell
            .get(&cell_id)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ObjectGridLoadCounts {
    pub gameobjects: u32,
    pub creatures: u32,
    pub corpses: u32,
    pub area_triggers: u32,
}

pub trait GridSpawnLoadFilter {
    fn should_spawn_on_grid_load(
        &mut self,
        object_type: SpawnObjectType,
        spawn_id: SpawnId,
    ) -> bool;
}

#[derive(Debug, Clone, Copy, Default)]
pub struct LoadAllGridSpawns;

impl GridSpawnLoadFilter for LoadAllGridSpawns {
    fn should_spawn_on_grid_load(
        &mut self,
        _object_type: SpawnObjectType,
        _spawn_id: SpawnId,
    ) -> bool {
        true
    }
}

impl<Filter> GridSpawnLoadFilter for &mut Filter
where
    Filter: GridSpawnLoadFilter + ?Sized,
{
    fn should_spawn_on_grid_load(
        &mut self,
        object_type: SpawnObjectType,
        spawn_id: SpawnId,
    ) -> bool {
        (**self).should_spawn_on_grid_load(object_type, spawn_id)
    }
}

impl GridSpawnLoadFilter for SpawnGridLoadStateLikeCpp<'_> {
    fn should_spawn_on_grid_load(
        &mut self,
        object_type: SpawnObjectType,
        spawn_id: SpawnId,
    ) -> bool {
        SpawnGridLoadStateLikeCpp::should_be_spawned_on_grid_load(self, object_type, spawn_id)
    }
}

#[derive(Debug)]
pub struct ObjectGridLoader<'a, Filter = LoadAllGridSpawns> {
    spawn_store: &'a SpawnStore,
    corpse_store: &'a CorpseCellStore,
    map_id: u32,
    difficulty: Difficulty,
    realm_id: u16,
    server_id: u32,
    filter: Filter,
}

#[derive(Debug)]
pub struct SpawnGridLifecycle<'a, Filter = LoadAllGridSpawns> {
    spawn_store: &'a SpawnStore,
    corpse_store: &'a CorpseCellStore,
    map_id: u32,
    difficulty: Difficulty,
    realm_id: u16,
    server_id: u32,
    filter: Filter,
    last_counts: ObjectGridLoadCounts,
}

impl<'a> SpawnGridLifecycle<'a, LoadAllGridSpawns> {
    pub fn new(
        spawn_store: &'a SpawnStore,
        corpse_store: &'a CorpseCellStore,
        map_id: u32,
        difficulty: Difficulty,
        realm_id: u16,
        server_id: u32,
    ) -> Self {
        Self {
            spawn_store,
            corpse_store,
            map_id,
            difficulty,
            realm_id,
            server_id,
            filter: LoadAllGridSpawns,
            last_counts: ObjectGridLoadCounts::default(),
        }
    }
}

impl<'a, Filter> SpawnGridLifecycle<'a, Filter>
where
    Filter: GridSpawnLoadFilter,
{
    pub fn with_filter(
        spawn_store: &'a SpawnStore,
        corpse_store: &'a CorpseCellStore,
        map_id: u32,
        difficulty: Difficulty,
        realm_id: u16,
        server_id: u32,
        filter: Filter,
    ) -> Self {
        Self {
            spawn_store,
            corpse_store,
            map_id,
            difficulty,
            realm_id,
            server_id,
            filter,
            last_counts: ObjectGridLoadCounts::default(),
        }
    }

    pub const fn last_counts(&self) -> ObjectGridLoadCounts {
        self.last_counts
    }
}

impl<Filter> GridLifecycle for SpawnGridLifecycle<'_, Filter>
where
    Filter: GridSpawnLoadFilter,
{
    fn load_grid_objects(&mut self, grid: &mut NGrid, _cell: &Cell) {
        let mut loader = ObjectGridLoader::with_filter(
            self.spawn_store,
            self.corpse_store,
            self.map_id,
            self.difficulty,
            self.realm_id,
            self.server_id,
            &mut self.filter,
        );
        self.last_counts = loader.load_n(grid);
    }

    fn stop_grid_objects(&mut self, _grid: &NGrid) {}
    fn evacuate_grid(&mut self, _grid: &mut NGrid) {}
    fn clean_grid(&mut self, _grid: &mut NGrid) {}
    fn unload_grid_objects(&mut self, _grid: &mut NGrid) {}
}

impl<'a> ObjectGridLoader<'a, LoadAllGridSpawns> {
    pub fn new(
        spawn_store: &'a SpawnStore,
        corpse_store: &'a CorpseCellStore,
        map_id: u32,
        difficulty: Difficulty,
        realm_id: u16,
        server_id: u32,
    ) -> Self {
        Self::with_filter(
            spawn_store,
            corpse_store,
            map_id,
            difficulty,
            realm_id,
            server_id,
            LoadAllGridSpawns,
        )
    }
}

impl<'a, Filter> ObjectGridLoader<'a, Filter>
where
    Filter: GridSpawnLoadFilter,
{
    pub fn with_filter(
        spawn_store: &'a SpawnStore,
        corpse_store: &'a CorpseCellStore,
        map_id: u32,
        difficulty: Difficulty,
        realm_id: u16,
        server_id: u32,
        filter: Filter,
    ) -> Self {
        Self {
            spawn_store,
            corpse_store,
            map_id,
            difficulty,
            realm_id,
            server_id,
            filter,
        }
    }

    pub fn load_n(&mut self, grid: &mut NGrid) -> ObjectGridLoadCounts {
        let mut counts = ObjectGridLoadCounts::default();

        for x in 0..MAX_NUMBER_OF_CELLS {
            for y in 0..MAX_NUMBER_OF_CELLS {
                let cell = grid
                    .get_grid_type_mut(x, y)
                    .expect("NGrid local cell coordinates are bounded by MAX_NUMBER_OF_CELLS");
                self.load_cell(cell, &mut counts);
            }
        }

        counts
    }

    pub fn has_personal_spawns(&self, phase_id: u32) -> bool {
        self.spawn_store
            .has_personal_spawns(self.map_id, self.difficulty, phase_id)
    }

    pub fn load_personal_phase(&mut self, grid: &mut NGrid, phase_id: u32) -> ObjectGridLoadCounts {
        let mut counts = ObjectGridLoadCounts::default();

        for x in 0..MAX_NUMBER_OF_CELLS {
            for y in 0..MAX_NUMBER_OF_CELLS {
                let cell = grid
                    .get_grid_type_mut(x, y)
                    .expect("NGrid local cell coordinates are bounded by MAX_NUMBER_OF_CELLS");
                self.load_personal_phase_cell(cell, phase_id, &mut counts);
            }
        }

        counts
    }

    fn load_cell(&mut self, cell: &mut Cell, counts: &mut ObjectGridLoadCounts) {
        let cell_id = cell.cell_coord().get_id();
        if let Some(cell_guids) =
            self.spawn_store
                .cell_object_guids(self.map_id, self.difficulty, cell_id)
        {
            for spawn_id in &cell_guids.gameobjects {
                if let Some(guid) = self.load_spawn_guid(SpawnObjectType::GameObject, *spawn_id) {
                    cell.grid_objects.gameobjects.insert(guid);
                    counts.gameobjects += 1;
                }
            }

            for spawn_id in &cell_guids.creatures {
                if let Some(guid) = self.load_spawn_guid(SpawnObjectType::Creature, *spawn_id) {
                    cell.grid_objects.creatures.insert(guid);
                    counts.creatures += 1;
                }
            }

            for spawn_id in &cell_guids.area_triggers {
                if !self
                    .filter
                    .should_spawn_on_grid_load(SpawnObjectType::AreaTrigger, *spawn_id)
                {
                    continue;
                }
                if self
                    .spawn_store
                    .spawn_data(SpawnObjectType::AreaTrigger, *spawn_id)
                    .is_none()
                {
                    continue;
                }
                // C++ `ObjectGridLoader::Visit(AreaTriggerMapType&)` calls
                // `AreaTrigger::LoadFromDB`, and `AreaTrigger::Create` then
                // allocates `map->GenerateLowGuid<HighGuid::AreaTrigger>()`.
                // This generic spawn-id -> ObjectGuid helper would use the DB
                // spawn id as counter and create-properties id as entry, which
                // is not a valid loaded AreaTrigger. A higher-level typed
                // record loader must create and add the AreaTrigger instead.
            }
        }

        for corpse in self.corpse_store.corpses_in_cell(cell_id) {
            if corpse.is_world_object {
                cell.world_objects.corpses.insert(corpse.guid);
            } else {
                cell.grid_objects.corpses.insert(corpse.guid);
            }
            counts.corpses += 1;
        }
    }

    fn load_personal_phase_cell(
        &mut self,
        cell: &mut Cell,
        phase_id: u32,
        counts: &mut ObjectGridLoadCounts,
    ) {
        let cell_id = cell.cell_coord().get_id();
        if let Some(cell_guids) = self.spawn_store.cell_personal_object_guids(
            self.map_id,
            self.difficulty,
            phase_id,
            cell_id,
        ) {
            for spawn_id in &cell_guids.gameobjects {
                if let Some(guid) = self.load_spawn_guid(SpawnObjectType::GameObject, *spawn_id) {
                    cell.grid_objects.gameobjects.insert(guid);
                    counts.gameobjects += 1;
                }
            }

            for spawn_id in &cell_guids.creatures {
                if let Some(guid) = self.load_spawn_guid(SpawnObjectType::Creature, *spawn_id) {
                    cell.grid_objects.creatures.insert(guid);
                    counts.creatures += 1;
                }
            }
        }
    }

    fn load_spawn_guid(
        &mut self,
        object_type: SpawnObjectType,
        spawn_id: SpawnId,
    ) -> Option<ObjectGuid> {
        if !self.filter.should_spawn_on_grid_load(object_type, spawn_id) {
            return None;
        }

        let data = self.spawn_store.spawn_data(object_type, spawn_id)?;
        Some(spawn_guid(
            data,
            self.realm_id,
            self.server_id,
            object_type.high_guid(),
        ))
    }
}

fn spawn_guid(data: &SpawnData, realm_id: u16, server_id: u32, high_guid: HighGuid) -> ObjectGuid {
    ObjectGuid::create_world_object(
        high_guid,
        0,
        realm_id,
        data.map_id as u16,
        server_id,
        data.id,
        data.spawn_id as i64,
    )
}

trait SpawnObjectTypeExt {
    fn high_guid(self) -> HighGuid;
}

impl SpawnObjectTypeExt for SpawnObjectType {
    fn high_guid(self) -> HighGuid {
        match self {
            SpawnObjectType::Creature => HighGuid::Creature,
            SpawnObjectType::GameObject => HighGuid::GameObject,
            SpawnObjectType::AreaTrigger => HighGuid::AreaTrigger,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::coords::GridCoord;
    use crate::grid::NGrid;
    use crate::spawn::{
        SpawnData, SpawnGroupFlags, SpawnGroupRuntimeState, SpawnGroupTemplateData, SpawnPosition,
    };

    fn spawn(object_type: SpawnObjectType, spawn_id: SpawnId, x: f32, y: f32) -> SpawnData {
        SpawnData {
            object_type,
            spawn_id,
            map_id: 571,
            db_data: true,
            spawn_group: SpawnGroupTemplateData::default_group(),
            id: match object_type {
                SpawnObjectType::Creature => 123,
                SpawnObjectType::GameObject => 456,
                SpawnObjectType::AreaTrigger => 789,
            },
            spawn_point: SpawnPosition::new(x, y, 1.0, 2.0),
            phase_use_flags: 0,
            phase_id: 0,
            phase_group: 0,
            terrain_swap_map: -1,
            pool_id: 0,
            spawn_time_secs: 120,
            spawn_difficulties: vec![1],
            script_id: 0,
            string_id: String::new(),
        }
    }

    #[derive(Debug, Default)]
    struct DenyGameObjects;

    impl GridSpawnLoadFilter for DenyGameObjects {
        fn should_spawn_on_grid_load(
            &mut self,
            object_type: SpawnObjectType,
            _spawn_id: SpawnId,
        ) -> bool {
            object_type != SpawnObjectType::GameObject
        }
    }

    #[test]
    fn load_n_iterates_cells_and_loads_spawn_store_guids_into_grid_containers() {
        let mut store = SpawnStore::new();
        let creature = spawn(SpawnObjectType::Creature, 100, 0.0, 0.0);
        let gameobject = spawn(SpawnObjectType::GameObject, 200, 0.0, 0.0);
        let area_trigger = spawn(SpawnObjectType::AreaTrigger, 300, 0.0, 0.0);
        store.add_object_spawn(&creature, |_| false);
        store.add_object_spawn(&gameobject, |_| false);
        store.add_area_trigger_spawn(&area_trigger);

        let corpse_guid = ObjectGuid::create_world_object(HighGuid::Corpse, 0, 1, 571, 1, 0, 77);
        let mut corpses = CorpseCellStore::new();
        corpses.add_corpse(
            creature.cell_id(),
            CorpseGridObject {
                guid: corpse_guid,
                is_world_object: true,
            },
        );

        let mut grid = NGrid::from_coords(32, 32, 1000, true);
        let mut loader = ObjectGridLoader::new(&store, &corpses, 571, 1, 1, 1);

        let counts = loader.load_n(&mut grid);

        assert_eq!(
            counts,
            ObjectGridLoadCounts {
                gameobjects: 1,
                creatures: 1,
                corpses: 1,
                area_triggers: 0,
            }
        );
        let cell = grid.get_grid_type(0, 0).unwrap();
        assert_eq!(cell.grid_objects.creatures.len(), 1);
        assert_eq!(cell.grid_objects.gameobjects.len(), 1);
        assert!(cell.grid_objects.area_triggers.is_empty());
        assert!(cell.world_objects.corpses.contains(&corpse_guid));
    }

    #[test]
    fn load_n_does_not_fabricate_area_trigger_guid_without_typed_record_loader_like_cpp() {
        let mut store = SpawnStore::new();
        let area_trigger = spawn(SpawnObjectType::AreaTrigger, 300, 0.0, 0.0);
        store.add_area_trigger_spawn(&area_trigger);

        let corpses = CorpseCellStore::new();
        let mut grid = NGrid::from_coords(32, 32, 1000, true);
        let mut loader = ObjectGridLoader::new(&store, &corpses, 571, 1, 1, 1);

        let counts = loader.load_n(&mut grid);

        assert_eq!(counts.area_triggers, 0);
        let cell = grid.get_grid_type(0, 0).unwrap();
        assert!(cell.grid_objects.area_triggers.is_empty());
        assert!(!cell.grid_objects.area_triggers.contains(&spawn_guid(
            &area_trigger,
            1,
            1,
            HighGuid::AreaTrigger,
        )));
    }

    #[test]
    fn load_n_honors_should_spawn_filter_before_inserting_guid() {
        let mut store = SpawnStore::new();
        let creature = spawn(SpawnObjectType::Creature, 100, 0.0, 0.0);
        let gameobject = spawn(SpawnObjectType::GameObject, 200, 0.0, 0.0);
        store.add_object_spawn(&creature, |_| false);
        store.add_object_spawn(&gameobject, |_| false);

        let corpses = CorpseCellStore::new();
        let mut grid = NGrid::from_coords(32, 32, 1000, true);
        let mut loader =
            ObjectGridLoader::with_filter(&store, &corpses, 571, 1, 1, 1, DenyGameObjects);

        let counts = loader.load_n(&mut grid);

        assert_eq!(counts.creatures, 1);
        assert_eq!(counts.gameobjects, 0);
        let cell = grid.get_grid_type(0, 0).unwrap();
        assert_eq!(cell.grid_objects.creatures.len(), 1);
        assert!(cell.grid_objects.gameobjects.is_empty());
    }

    #[test]
    fn load_n_with_cpp_grid_load_state_omits_inactive_spawn_and_loads_active_spawn() {
        let mut store = SpawnStore::new();
        let inactive_group = SpawnGroupTemplateData {
            group_id: 10,
            name: "manual".to_string(),
            map_id: 571,
            flags: SpawnGroupFlags::MANUAL_SPAWN,
        };
        let active_group = SpawnGroupTemplateData {
            group_id: 11,
            name: "default".to_string(),
            map_id: 571,
            flags: SpawnGroupFlags::NONE,
        };
        let mut inactive = spawn(SpawnObjectType::Creature, 100, 0.0, 0.0);
        inactive.spawn_group = inactive_group;
        let mut active = spawn(SpawnObjectType::Creature, 101, 0.0, 0.0);
        active.spawn_group = active_group;
        store.add_object_spawn(&inactive, |_| false);
        store.add_object_spawn(&active, |_| false);

        let state = SpawnGroupRuntimeState::new();
        let filter = SpawnGridLoadStateLikeCpp::new(&store, &state);
        let corpses = CorpseCellStore::new();
        let mut grid = NGrid::from_coords(32, 32, 1000, true);
        let mut loader = ObjectGridLoader::with_filter(&store, &corpses, 571, 1, 1, 1, filter);

        let counts = loader.load_n(&mut grid);

        assert_eq!(counts.creatures, 1);
        let cell = grid.get_grid_type(0, 0).unwrap();
        assert_eq!(cell.grid_objects.creatures.len(), 1);
        assert!(
            cell.grid_objects
                .creatures
                .iter()
                .any(|guid| guid.counter() == 101)
        );
    }

    #[test]
    fn load_n_uses_grid_local_cells_not_only_constructor_cell() {
        let mut store = SpawnStore::new();
        let spawn = spawn(
            SpawnObjectType::Creature,
            100,
            crate::coords::SIZE_OF_GRID_CELL,
            0.0,
        );
        store.add_object_spawn(&spawn, |_| false);

        let corpses = CorpseCellStore::new();
        let mut grid = NGrid::from_coords(32, 32, 1000, true);
        let mut loader = ObjectGridLoader::new(&store, &corpses, 571, 1, 1, 1);

        let counts = loader.load_n(&mut grid);

        assert_eq!(counts.creatures, 1);
        assert!(
            grid.get_grid_type(0, 0)
                .unwrap()
                .grid_objects
                .creatures
                .is_empty()
        );
        assert_eq!(
            grid.get_grid_type(1, 0)
                .unwrap()
                .grid_objects
                .creatures
                .len(),
            1
        );
    }

    #[test]
    fn grid_spawn_guid_keeps_cpp_world_object_shape() {
        let data = spawn(SpawnObjectType::Creature, 100, 0.0, 0.0);

        let guid = spawn_guid(&data, 1, 2, HighGuid::Creature);

        assert_eq!(guid.high_type(), HighGuid::Creature);
        assert_eq!(guid.map_id(), 571);
        assert_eq!(guid.entry(), 123);
        assert_eq!(guid.counter(), 100);
    }

    #[test]
    fn corpse_store_can_load_world_and_grid_corpses_for_same_cell() {
        let mut corpses = CorpseCellStore::new();
        let cell_id = crate::cell::Cell::from_world(0.0, 0.0)
            .cell_coord()
            .get_id();
        let world = ObjectGuid::create_world_object(HighGuid::Corpse, 0, 1, 571, 1, 0, 1);
        let grid = ObjectGuid::create_world_object(HighGuid::Corpse, 0, 1, 571, 1, 0, 2);
        corpses.add_corpse(
            cell_id,
            CorpseGridObject {
                guid: world,
                is_world_object: true,
            },
        );
        corpses.add_corpse(
            cell_id,
            CorpseGridObject {
                guid: grid,
                is_world_object: false,
            },
        );

        let store = SpawnStore::new();
        let mut ngrid = NGrid::from_coords(32, 32, 1000, true);
        let mut loader = ObjectGridLoader::new(&store, &corpses, 571, 1, 1, 1);

        let counts = loader.load_n(&mut ngrid);

        assert_eq!(counts.corpses, 2);
        let cell = ngrid.get_grid_type(0, 0).unwrap();
        assert!(cell.world_objects.corpses.contains(&world));
        assert!(cell.grid_objects.corpses.contains(&grid));
    }

    #[test]
    fn spawn_grid_lifecycle_new_preserves_legacy_load_all_filter() {
        let mut store = SpawnStore::new();
        let manual_group = SpawnGroupTemplateData {
            group_id: 10,
            name: "manual".to_string(),
            map_id: 571,
            flags: SpawnGroupFlags::MANUAL_SPAWN,
        };
        let mut spawn = spawn(SpawnObjectType::Creature, 100, 0.0, 0.0);
        spawn.spawn_group = manual_group;
        let expected_guid = spawn_guid(&spawn, 1, 1, HighGuid::Creature);
        store.add_object_spawn(&spawn, |_| false);
        let corpses = CorpseCellStore::new();
        let lifecycle = SpawnGridLifecycle::new(&store, &corpses, 571, 1, 1, 1);
        let mut map = crate::map::Map::with_hooks(
            571,
            0,
            1,
            1000,
            true,
            100.0,
            crate::map::NoopTerrainGridLoader,
            lifecycle,
        );

        map.ensure_grid_loaded(&crate::map::cell_from_grid_center(GridCoord::new(32, 32)));

        assert_eq!(
            map.lifecycle().last_counts(),
            ObjectGridLoadCounts {
                gameobjects: 0,
                creatures: 1,
                corpses: 0,
                area_triggers: 0,
            }
        );
        let cell = map
            .get_ngrid(GridCoord::new(32, 32))
            .unwrap()
            .get_grid_type(0, 0)
            .unwrap();
        assert_eq!(cell.grid_objects.creatures.len(), 1);
        assert!(cell.grid_objects.creatures.contains(&expected_guid));
    }

    #[test]
    fn spawn_grid_lifecycle_with_cpp_filter_omits_inactive_manual_group_on_grid_load() {
        let mut store = SpawnStore::new();
        let manual_group = SpawnGroupTemplateData {
            group_id: 10,
            name: "manual".to_string(),
            map_id: 571,
            flags: SpawnGroupFlags::MANUAL_SPAWN,
        };
        let default_group = SpawnGroupTemplateData {
            group_id: 11,
            name: "default".to_string(),
            map_id: 571,
            flags: SpawnGroupFlags::NONE,
        };
        let mut inactive = spawn(SpawnObjectType::Creature, 100, 0.0, 0.0);
        inactive.spawn_group = manual_group;
        let inactive_guid = spawn_guid(&inactive, 1, 1, HighGuid::Creature);
        let mut active = spawn(SpawnObjectType::Creature, 101, 0.0, 0.0);
        active.spawn_group = default_group;
        let active_guid = spawn_guid(&active, 1, 1, HighGuid::Creature);
        store.add_object_spawn(&inactive, |_| false);
        store.add_object_spawn(&active, |_| false);

        let state = SpawnGroupRuntimeState::new();
        let filter = SpawnGridLoadStateLikeCpp::new(&store, &state);
        let corpses = CorpseCellStore::new();
        let lifecycle = SpawnGridLifecycle::with_filter(&store, &corpses, 571, 1, 1, 1, filter);
        let mut map = crate::map::Map::with_hooks(
            571,
            0,
            1,
            1000,
            true,
            100.0,
            crate::map::NoopTerrainGridLoader,
            lifecycle,
        );

        map.ensure_grid_loaded(&crate::map::cell_from_grid_center(GridCoord::new(32, 32)));

        assert_eq!(
            map.lifecycle().last_counts(),
            ObjectGridLoadCounts {
                gameobjects: 0,
                creatures: 1,
                corpses: 0,
                area_triggers: 0,
            }
        );
        let cell = map
            .get_ngrid(GridCoord::new(32, 32))
            .unwrap()
            .get_grid_type(0, 0)
            .unwrap();
        assert_eq!(cell.grid_objects.creatures.len(), 1);
        assert!(cell.grid_objects.creatures.contains(&active_guid));
        assert!(!cell.grid_objects.creatures.contains(&inactive_guid));
    }

    #[test]
    fn load_n_does_not_cross_grid_boundaries() {
        let mut store = SpawnStore::new();
        let outside = spawn(SpawnObjectType::Creature, 100, 600.0, 600.0);
        store.add_object_spawn(&outside, |_| false);

        let corpses = CorpseCellStore::new();
        let mut grid = NGrid::from_coords(32, 32, 1000, true);
        let mut loader = ObjectGridLoader::new(&store, &corpses, 571, 1, 1, 1);

        let counts = loader.load_n(&mut grid);

        assert_eq!(counts.creatures, 0);
        assert_eq!(
            grid.get_grid_type(0, 0)
                .unwrap()
                .grid_objects
                .creatures
                .len(),
            0
        );
        assert_eq!(grid.grid_id(), GridCoord::new(32, 32).x_coord * 64 + 32);
    }
}
