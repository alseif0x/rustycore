//! Manager packets.
//!
//! Separated from runtime.rs under #701.

use super::*;

impl MapManager {
    pub fn new() -> Self {
        let mut manager = Self {
            maps: HashMap::new(),
            free_instance_ids: Vec::new(),
            next_instance_id: 1,
            tick_owner: RuntimeTickOwner::Session,
            terrain: None,
        };
        manager.init_instance_ids_from_max(0);
        manager
    }

    /// Attach the shared, file-backed terrain height store (server startup).
    pub fn set_terrain(&mut self, terrain: Arc<LiveTerrainHeights>) {
        self.terrain = Some(terrain);
    }

    /// Shared terrain height store, if wired. Cloned so callers can use it while
    /// still holding `&mut self` for the spawn/respawn mutation.
    #[must_use]
    pub fn terrain(&self) -> Option<Arc<LiveTerrainHeights>> {
        self.terrain.clone()
    }

    /// Returns the current tick owner for this map manager.
    ///
    /// Returns a `Copy` value; the caller should read this once and release the
    /// lock before performing any tick work.
    pub fn tick_owner(&self) -> RuntimeTickOwner {
        self.tick_owner
    }

    /// Sets the tick owner.
    ///
    /// Production calls this exactly once, at startup
    /// (`crates/world-server/src/app.rs`), *before* the global legacy creature
    /// loop is spawned. Flipping it after the loop is running is the only
    /// window in which both the loop and a session can tick the same creature,
    /// so the single call site is asserted by a test rather than left to
    /// convention (#28).
    pub fn set_tick_owner(&mut self, owner: RuntimeTickOwner) {
        self.tick_owner = owner;
    }

    /// Returns the `(map_id, instance_id)` keys of all currently active map
    /// instances held by this manager.
    ///
    /// The key type matches `self.maps: HashMap<(u16, u32), MapInstance>` exactly.
    /// Order is unspecified (hash map iteration order).
    pub fn active_map_keys(&self) -> Vec<(u16, u32)> {
        self.maps.keys().copied().collect()
    }

    pub fn init_instance_ids_from_max(&mut self, max_existing_instance_id: u32) {
        self.next_instance_id = 1;
        self.free_instance_ids = vec![true; max_existing_instance_id.saturating_add(2) as usize];
        self.free_instance_ids[0] = false;
    }

    pub fn register_instance_id(&mut self, instance_id: u32) {
        let index = instance_id as usize;
        if index >= self.free_instance_ids.len() {
            self.free_instance_ids.resize(index.saturating_add(2), true);
        }

        self.free_instance_ids[index] = false;

        if self.next_instance_id == instance_id {
            self.next_instance_id = self.next_instance_id.saturating_add(1);
        }
    }

    pub fn generate_instance_id(&mut self) -> Option<u32> {
        if self.next_instance_id == u32::MAX {
            return None;
        }

        let new_instance_id = self.next_instance_id;
        let index = new_instance_id as usize;
        if index >= self.free_instance_ids.len() {
            self.free_instance_ids.resize(index.saturating_add(1), true);
        }
        self.free_instance_ids[index] = false;

        let search_start = self.next_instance_id.saturating_add(1) as usize;
        if let Some(next_free_offset) = self.free_instance_ids[search_start..]
            .iter()
            .position(|is_free| *is_free)
        {
            self.next_instance_id = (search_start + next_free_offset) as u32;
        } else {
            self.next_instance_id = self.free_instance_ids.len() as u32;
            self.free_instance_ids.push(true);
        }

        Some(new_instance_id)
    }

    pub fn free_instance_id(&mut self, instance_id: u32) {
        if instance_id == 0 {
            if self.free_instance_ids.is_empty() {
                self.init_instance_ids_from_max(0);
            } else {
                self.free_instance_ids[0] = false;
            }
            return;
        }

        let index = instance_id as usize;
        if index >= self.free_instance_ids.len() {
            self.free_instance_ids.resize(index.saturating_add(2), true);
        }

        self.next_instance_id = self.next_instance_id.min(instance_id);
        self.free_instance_ids[index] = true;
        self.free_instance_ids[0] = false;
    }

    pub fn get_or_create_map(&mut self, map_id: u16, instance_id: u32) -> &mut MapInstance {
        let key = (map_id, instance_id);
        if !self.maps.contains_key(&key) {
            let instance = MapInstance::new(map_id, instance_id);
            self.maps.insert(key, instance);
            info!(
                "Created new map instance: map_id={}, instance_id={}",
                map_id, instance_id
            );
        }
        self.maps.get_mut(&key).unwrap()
    }

    pub fn get_map(&self, map_id: u16, instance_id: u32) -> Option<&MapInstance> {
        self.maps.get(&(map_id, instance_id))
    }

    pub fn get_map_mut(&mut self, map_id: u16, instance_id: u32) -> Option<&mut MapInstance> {
        self.maps.get_mut(&(map_id, instance_id))
    }

    // Convenience methods that delegate to MapInstance

    pub fn get_grid(&self, map_id: u16, instance_id: u32, x: i16, y: i16) -> Option<&Grid> {
        self.get_map(map_id, instance_id)?.get_grid(x, y)
    }

    pub fn get_grid_mut(
        &mut self,
        map_id: u16,
        instance_id: u32,
        x: i16,
        y: i16,
    ) -> Option<&mut Grid> {
        self.get_map_mut(map_id, instance_id)?.get_grid_mut(x, y)
    }

    pub fn get_or_create_grid(
        &mut self,
        map_id: u16,
        instance_id: u32,
        x: i16,
        y: i16,
    ) -> &mut Grid {
        self.get_or_create_map(map_id, instance_id)
            .get_or_create_grid(x, y)
    }

    pub fn add_creature(
        &mut self,
        map_id: u16,
        instance_id: u32,
        x: i16,
        y: i16,
        mut creature: WorldCreature,
    ) -> bool {
        let _ = creature
            .creature
            .unit_mut()
            .world_mut()
            .set_map(u32::from(map_id), instance_id);
        creature
            .creature
            .unit_mut()
            .world_mut()
            .object_mut()
            .add_to_world();
        self.get_or_create_map(map_id, instance_id)
            .add_creature(x, y, creature)
    }

    pub fn get_creature(
        &self,
        map_id: u16,
        instance_id: u32,
        x: i16,
        y: i16,
        guid: ObjectGuid,
    ) -> Option<&WorldCreature> {
        self.get_map(map_id, instance_id)?.get_creature(x, y, guid)
    }

    pub fn get_creature_mut(
        &mut self,
        map_id: u16,
        instance_id: u32,
        x: i16,
        y: i16,
        guid: ObjectGuid,
    ) -> Option<&mut WorldCreature> {
        self.get_map_mut(map_id, instance_id)?
            .get_creature_mut(x, y, guid)
    }

    pub fn find_creature(
        &self,
        map_id: u16,
        instance_id: u32,
        guid: ObjectGuid,
    ) -> Option<&WorldCreature> {
        let map = self.get_map(map_id, instance_id)?;
        map.grids.values().find_map(|grid| grid.get_creature(guid))
    }

    pub fn find_creature_mut(
        &mut self,
        map_id: u16,
        instance_id: u32,
        guid: ObjectGuid,
    ) -> Option<&mut WorldCreature> {
        let map = self.get_map_mut(map_id, instance_id)?;
        map.grids
            .values_mut()
            .find_map(|grid| grid.get_creature_mut(guid))
    }

    pub fn set_creature_anim_kit_id_like_cpp(
        &mut self,
        map_id: u16,
        instance_id: u32,
        guid: ObjectGuid,
        slot: CreatureAnimKitSlotLikeCpp,
        anim_kit_id: u16,
        anim_kit_exists: impl Fn(u16) -> bool,
    ) -> Option<RuntimeEvent> {
        use wow_packet::ServerPacket;

        let creature = self.find_creature_mut(map_id, instance_id, guid)?;
        if anim_kit_id != 0 && !anim_kit_exists(anim_kit_id) {
            return None;
        }

        let changed = match slot {
            CreatureAnimKitSlotLikeCpp::Ai => {
                let changed = creature
                    .creature
                    .unit_mut()
                    .set_ai_anim_kit_id_like_cpp(anim_kit_id);
                if changed {
                    creature.create_data.ai_anim_kit_id = anim_kit_id;
                }
                changed
            }
            CreatureAnimKitSlotLikeCpp::Movement => {
                let changed = creature
                    .creature
                    .unit_mut()
                    .set_movement_anim_kit_id_like_cpp(anim_kit_id);
                if changed {
                    creature.create_data.movement_anim_kit_id = anim_kit_id;
                }
                changed
            }
            CreatureAnimKitSlotLikeCpp::Melee => {
                let changed = creature
                    .creature
                    .unit_mut()
                    .set_melee_anim_kit_id_like_cpp(anim_kit_id);
                if changed {
                    creature.create_data.melee_anim_kit_id = anim_kit_id;
                }
                changed
            }
        };
        if !changed {
            return None;
        }

        let packet_bytes = match slot {
            CreatureAnimKitSlotLikeCpp::Ai => wow_packet::packets::misc::SetAiAnimKit {
                unit: guid,
                anim_kit_id,
            }
            .to_bytes(),
            CreatureAnimKitSlotLikeCpp::Movement => wow_packet::packets::misc::SetMovementAnimKit {
                unit: guid,
                anim_kit_id,
            }
            .to_bytes(),
            CreatureAnimKitSlotLikeCpp::Melee => wow_packet::packets::misc::SetMeleeAnimKit {
                unit: guid,
                anim_kit_id,
            }
            .to_bytes(),
        };
        let source_position = creature.position();
        let range = creature.visibility_range_like_cpp();
        Some(RuntimeEvent {
            source_guid: guid,
            recipients: RecipientRule::NearbyVisible {
                source_guid: guid,
                map_id,
                instance_id,
                source_position,
                range,
                required_3d: false,
            },
            packet_bytes,
        })
    }

    pub fn creature_guids(&self, map_id: u16, instance_id: u32) -> Vec<ObjectGuid> {
        self.get_map(map_id, instance_id)
            .map(|map| {
                map.grids
                    .values()
                    .flat_map(|grid| grid.creatures.keys().copied())
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn active_creature_guids_for_player_update_like_cpp(
        &self,
        map_id: u16,
        instance_id: u32,
        player_position: Position,
        player_phase_shift: &PhaseShift,
    ) -> Vec<ObjectGuid> {
        let Some(map) = self.get_map(map_id, instance_id) else {
            return Vec::new();
        };
        let (low, high) = calculate_cell_area_like_cpp(player_position, VISIBILITY_RADIUS);
        let mut guids = Vec::new();

        for grid in map.grids.values() {
            for creature in grid.creatures.values() {
                if !creature.creature.unit().world().object().is_in_world() {
                    continue;
                }
                if !player_phase_shift.can_see(creature.phase_shift()) {
                    continue;
                }
                let Some(cell) =
                    cell_area_contains_position_like_cpp(low, high, creature.position())
                else {
                    continue;
                };
                guids.push((cell, creature.guid()));
            }
        }

        guids.sort_by_key(|(cell, guid)| (cell.x, cell.y, guid.high_value(), guid.low_value()));
        guids.into_iter().map(|(_, guid)| guid).collect()
    }

    pub fn with_creature_mut<F, R>(
        &mut self,
        map_id: u16,
        instance_id: u32,
        x: i16,
        y: i16,
        guid: ObjectGuid,
        f: F,
    ) -> Option<R>
    where
        F: FnOnce(&mut WorldCreature) -> R,
    {
        self.get_map_mut(map_id, instance_id)?
            .get_grid_mut(x, y)?
            .get_creature_mut(guid)
            .map(f)
    }

    // ── Respawn queue delegates (Slice 4A.2a) ─────────────────────────────────

    pub fn player_enter_grid(
        &mut self,
        map_id: u16,
        instance_id: u32,
        x: i16,
        y: i16,
        player_guid: ObjectGuid,
        _pos: Position,
    ) {
        let grid = self.get_or_create_grid(map_id, instance_id, x, y);
        grid.player_enter(player_guid);
        debug!(
            "Player {:?} entered grid ({}, {}) in map {}",
            player_guid, x, y, map_id
        );
    }

    pub fn player_leave_grid(
        &mut self,
        map_id: u16,
        instance_id: u32,
        x: i16,
        y: i16,
        player_guid: ObjectGuid,
    ) {
        if let Some(grid) = self.get_grid_mut(map_id, instance_id, x, y) {
            grid.player_leave(player_guid);
            debug!(
                "Player {:?} left grid ({}, {}) in map {}",
                player_guid, x, y, map_id
            );
        }
    }

    pub fn get_visible_creatures(
        &self,
        map_id: u16,
        instance_id: u32,
        x: f32,
        y: f32,
        _z: f32,
    ) -> Vec<WorldCreature> {
        self.get_visible_creatures_in_phase(map_id, instance_id, x, y, _z, VISIBILITY_RADIUS, None)
    }

    pub fn get_visible_creatures_in_phase(
        &self,
        map_id: u16,
        instance_id: u32,
        x: f32,
        y: f32,
        z: f32,
        visibility_range: f32,
        seer_phase_shift: Option<&PhaseShift>,
    ) -> Vec<WorldCreature> {
        let center_x = world_to_grid_x(x);
        let center_y = world_to_grid_y(y);

        let mut creatures = Vec::new();

        // Get creatures from 3x3 grid area
        for dx in -1..=1 {
            for dy in -1..=1 {
                let grid_x = center_x + dx;
                let grid_y = center_y + dy;

                if let Some(grid) = self.get_grid(map_id, instance_id, grid_x, grid_y) {
                    for creature in grid.creatures.values() {
                        if let Some(seer_phase_shift) = seer_phase_shift
                            && !seer_phase_shift.can_see(creature.phase_shift())
                        {
                            continue;
                        }

                        // C++ `CanSeeOrDetect(..., distanceCheck=true)` uses
                        // `IsWithinDist(..., is3D=false)` for visibility
                        // (`Object.cpp:1609`). Keep the legacy map path aligned
                        // with the canonical map visibility path.
                        let dist = Position::new(x, y, z, 0.0).distance_2d(&creature.position());
                        if dist <= visibility_range {
                            creatures.push(creature.clone());
                        }
                    }
                }
            }
        }

        creatures
    }

    pub fn unload_distant_grids(
        &mut self,
        map_id: u16,
        instance_id: u32,
        center_x: i16,
        center_y: i16,
        range: i16,
    ) {
        if let Some(map) = self.get_map_mut(map_id, instance_id) {
            let to_remove: Vec<GridCoord> = map
                .grids
                .keys()
                .filter(|coord| {
                    let dx = (coord.x - center_x).abs();
                    let dy = (coord.y - center_y).abs();
                    dx > range || dy > range
                })
                .copied()
                .collect();

            for coord in to_remove {
                if let Some(grid) = map.grids.get(&coord) {
                    if grid.should_unload(map.grid_unload_timeout) {
                        info!("Unloading distant grid {:?} from map {}", coord, map_id);
                        map.grids.remove(&coord);
                        map.personal_phases
                            .unload_grid_like_cpp(coord.personal_phase_grid_id_like_cpp());
                    }
                }
            }
        }
    }

    pub fn is_grid_loaded(&self, map_id: u16, instance_id: u32, x: i16, y: i16) -> bool {
        self.get_map(map_id, instance_id)
            .map(|m| m.is_grid_loaded(x, y))
            .unwrap_or(false)
    }

    pub fn min_height_like_cpp(&self, map_id: u16, instance_id: u32, x: f32, y: f32) -> f32 {
        self.get_map(map_id, instance_id)
            .map(|m| m.min_height_like_cpp(x, y))
            .unwrap_or(DEFAULT_MIN_HEIGHT_LIKE_CPP)
    }

    pub fn create_grid(&mut self, map_id: u16, instance_id: u32, x: i16, y: i16) -> &mut Grid {
        self.get_or_create_grid(map_id, instance_id, x, y)
    }

    pub fn creature_count(&self) -> usize {
        self.maps.values().map(|m| m.creature_count()).sum()
    }
}
