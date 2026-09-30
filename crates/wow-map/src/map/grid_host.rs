// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Map adapters for grid unloading and world-object queries.

use super::*;

impl<Terrain, Lifecycle> GridUnloadEntityStore for Map<Terrain, Lifecycle> {
    fn creature_mut(&mut self, guid: ObjectGuid) -> Option<&mut Creature> {
        self.entity_world
            .get_mut(&guid)
            .and_then(ObjectMut::creature_mut)
    }

    fn game_object_mut(&mut self, guid: ObjectGuid) -> Option<&mut GameObject> {
        self.entity_world
            .get_mut(&guid)
            .and_then(ObjectMut::game_object_mut)
    }

    fn dynamic_object_mut(&mut self, guid: ObjectGuid) -> Option<&mut DynamicObject> {
        self.entity_world
            .get_mut(&guid)
            .and_then(ObjectMut::dynamic_object_mut)
    }

    fn corpse_mut(&mut self, guid: ObjectGuid) -> Option<&mut Corpse> {
        self.entity_world
            .get_mut(&guid)
            .and_then(ObjectMut::corpse_mut)
    }

    fn area_trigger_mut(&mut self, guid: ObjectGuid) -> Option<&mut AreaTrigger> {
        self.entity_world
            .get_mut(&guid)
            .and_then(ObjectMut::area_trigger_mut)
    }

    fn scene_object_mut(&mut self, guid: ObjectGuid) -> Option<&mut SceneObject> {
        self.entity_world
            .get_mut(&guid)
            .and_then(ObjectMut::scene_object_mut)
    }

    fn conversation_mut(&mut self, guid: ObjectGuid) -> Option<&mut Conversation> {
        self.entity_world
            .get_mut(&guid)
            .and_then(ObjectMut::conversation_mut)
    }
}

impl<Terrain, Lifecycle> WorldObjectEnvironment for Map<Terrain, Lifecycle>
where
    Terrain: TerrainGridLoader + MapWorldObjectEnvironment,
    Lifecycle: GridLifecycle,
{
    fn map_id(&self) -> u32 {
        self.map_id
    }

    fn instance_id(&self) -> u32 {
        self.instance_id
    }

    fn visibility_range(&self) -> f32 {
        self.visible_distance
    }

    fn line_of_sight(&self, query: LineOfSightQuery<'_>) -> bool {
        self.terrain.line_of_sight(query)
    }

    fn map_height(
        &self,
        object: &WorldObject,
        x: f32,
        y: f32,
        z: f32,
        query: WorldObjectHeightQuery,
    ) -> f32 {
        self.terrain.map_height(object, x, y, z, query)
    }

    fn floor_z(&self, object: &WorldObject, position: Position, max_search_dist: f32) -> f32 {
        self.terrain.floor_z(object, position, max_search_dist)
    }
}

impl<Terrain, Lifecycle> MapGridHost for Map<Terrain, Lifecycle>
where
    Terrain: TerrainGridLoader,
    Lifecycle: GridLifecycle,
{
    fn active_objects_near_grid(&self, grid: &NGrid) -> bool {
        Map::active_objects_near_grid(self, grid)
    }

    fn stop_grid_objects(&mut self, grid: &NGrid) {
        self.lifecycle.stop_grid_objects(grid);
        self.drain_grid_unload_actions_like_cpp();
    }

    fn reset_grid_expiry(&mut self, grid: &mut NGrid, factor: f32) {
        Map::reset_grid_expiry(self, grid, factor);
    }

    fn unload_grid(&mut self, grid: &mut NGrid, unload_all: bool) -> bool {
        if !self.can_unload_grid(grid, unload_all) {
            return false;
        }

        self.run_unload_lifecycle(grid, unload_all);
        self.grid_state_unloaded = true;
        true
    }
}
