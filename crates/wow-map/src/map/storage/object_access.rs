// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Borrowed and owned canonical object projections; no storage representation escapes.

use super::*;

impl<Terrain, Lifecycle> Map<Terrain, Lifecycle>
where
    Terrain: TerrainGridLoader,
    Lifecycle: GridLifecycle,
{

    pub(in crate::map) fn player_viewpoint_guid_like_cpp(
        &self,
        player_guid: ObjectGuid,
    ) -> Option<ObjectGuid> {
        let record = self.map_object_record(player_guid)?;
        if record.kind() != AccessorObjectKind::Player {
            return None;
        }
        let Some(player) = record.player() else {
            return Some(player_guid);
        };
        let farsight = player.active_data().farsight_object;
        Some(if farsight.is_empty() {
            player_guid
        } else {
            farsight
        })
    }

    pub(in crate::map) fn exact_cell_guids_like_cpp(&self, cell_coord: CellCoord) -> NearbyCellGuids {
        let mut nearby = NearbyCellGuids::default();
        let cell = Cell::from_cell_coord(cell_coord);
        let Some(grid) = self.get_ngrid(GridCoord::new(cell.grid_x(), cell.grid_y())) else {
            return nearby;
        };
        let Some(local_cell) = grid.get_grid_type(cell.cell_x(), cell.cell_y()) else {
            return nearby;
        };

        nearby.visited_cells = 1;
        nearby.merge_world(&local_cell.world_objects);
        nearby.merge_grid(&local_cell.grid_objects);
        nearby
    }

    pub fn get_creature(&self, guid: ObjectGuid) -> Option<&WorldObject> {
        self.map_object_by_kind(guid, &[AccessorObjectKind::Creature])
    }

    pub(crate) fn get_typed_creature(&self, guid: ObjectGuid) -> Option<&Creature> {
        let record = self.map_object_record(guid)?;
        if record.kind() != AccessorObjectKind::Creature {
            return None;
        }
        record.creature()
    }

    /// Return an owned transform/vitals view of an exact canonical Creature.
    /// This is the preferred external read seam for systems that do not need the
    /// complete entity and remains compatible with the selected private ECS
    /// backend.
    pub fn creature_transform_vitals_snapshot_like_cpp(
        &self,
        guid: ObjectGuid,
    ) -> Option<CreatureTransformVitalsSnapshotLikeCpp> {
        self.entity_world.creature_transform_vitals_snapshot(guid)
    }

    /// Run a synchronous read against one exact canonical Creature without
    /// exposing the storage representation. `R` is owned independently of the
    /// callback borrow, so a future ECS guard cannot escape this method.
    pub fn with_creature_like_cpp<R>(
        &self,
        guid: ObjectGuid,
        read: impl FnOnce(&Creature) -> R,
    ) -> Option<R> {
        self.entity_world.with_creature(guid, read)
    }

    /// Run one synchronous mutation inside the canonical entity owner and
    /// return only an owned result.
    pub fn with_creature_mut_like_cpp<R>(
        &mut self,
        guid: ObjectGuid,
        write: impl FnOnce(&mut Creature) -> R,
    ) -> Option<R> {
        self.entity_world.with_creature_mut(guid, write)
    }

    pub fn get_typed_creature_mut(&mut self, guid: ObjectGuid) -> Option<&mut Creature> {
        let record = self.entity_world.get_mut(&guid)?;
        if record.kind() != AccessorObjectKind::Creature {
            return None;
        }
        record.creature_mut()
    }

    pub fn get_pet(&self, guid: ObjectGuid) -> Option<&WorldObject> {
        self.map_object_by_kind(guid, &[AccessorObjectKind::Pet])
    }

    pub fn get_typed_pet(&self, guid: ObjectGuid) -> Option<&Pet> {
        let record = self.map_object_record(guid)?;
        if record.kind() != AccessorObjectKind::Pet {
            return None;
        }
        record.pet()
    }

    pub fn get_typed_pet_mut(&mut self, guid: ObjectGuid) -> Option<&mut Pet> {
        let record = self.entity_world.get_mut(&guid)?;
        if record.kind() != AccessorObjectKind::Pet {
            return None;
        }
        record.pet_mut()
    }

    /// Run one synchronous read against an exact canonical Pet without exposing
    /// the map's storage representation or allowing the borrowed entity to
    /// escape. This is the migration-safe equivalent of C++'s typed object-store
    /// lookup for adapter code that returns an owned snapshot.
    pub fn with_pet_like_cpp<R>(
        &self,
        guid: ObjectGuid,
        read: impl FnOnce(&Pet) -> R,
    ) -> Option<R> {
        let record = self.entity_world.get(&guid)?;
        if record.kind() != AccessorObjectKind::Pet {
            return None;
        }
        record.pet().map(read)
    }

    pub fn with_game_object_like_cpp<R>(
        &self,
        guid: ObjectGuid,
        read: impl FnOnce(&GameObject) -> R,
    ) -> Option<R> {
        let record = self.entity_world.get(&guid)?;
        if record.kind() != AccessorObjectKind::GameObject {
            return None;
        }
        record.game_object().map(read)
    }

    /// Read either the exact Creature or Pet body addressed by `guid` while the
    /// callback is in scope. The optional owner is populated only for Pets.
    pub fn with_creature_or_pet_like_cpp<R>(
        &self,
        guid: ObjectGuid,
        read: impl FnOnce(&Creature, Option<ObjectGuid>) -> R,
    ) -> Option<R> {
        let record = self.entity_world.get(&guid)?;
        match record.kind() {
            AccessorObjectKind::Creature => record.creature().map(|creature| read(creature, None)),
            AccessorObjectKind::Pet => record
                .pet()
                .map(|pet| read(pet.creature(), Some(pet.owner_guid()))),
            _ => None,
        }
    }

    /// Read the common WorldObject projection for one of the explicitly
    /// accepted canonical kinds. The callback result must be owned, so this
    /// remains compatible with a guard-based entity backend.
    pub fn with_world_object_by_kinds_like_cpp<R>(
        &self,
        guid: ObjectGuid,
        allowed: &[AccessorObjectKind],
        read: impl FnOnce(&WorldObject) -> R,
    ) -> Option<R> {
        let record = self.entity_world.get(&guid)?;
        allowed
            .contains(&record.kind())
            .then(|| read(record.object()))
    }

    pub fn contains_map_object_like_cpp(&self, guid: ObjectGuid) -> bool {
        self.entity_world.get(&guid).is_some()
    }

    pub fn with_area_trigger_like_cpp<R>(
        &self,
        guid: ObjectGuid,
        read: impl FnOnce(&AreaTrigger) -> R,
    ) -> Option<R> {
        let record = self.entity_world.get(&guid)?;
        if record.kind() != AccessorObjectKind::AreaTrigger {
            return None;
        }
        record.area_trigger().map(read)
    }

    pub fn with_scene_object_like_cpp<R>(
        &self,
        guid: ObjectGuid,
        read: impl FnOnce(&SceneObject) -> R,
    ) -> Option<R> {
        let record = self.entity_world.get(&guid)?;
        if record.kind() != AccessorObjectKind::SceneObject {
            return None;
        }
        record.scene_object().map(read)
    }

    pub fn with_conversation_like_cpp<R>(
        &self,
        guid: ObjectGuid,
        read: impl FnOnce(&Conversation) -> R,
    ) -> Option<R> {
        let record = self.entity_world.get(&guid)?;
        if record.kind() != AccessorObjectKind::Conversation {
            return None;
        }
        record.conversation().map(read)
    }

    pub fn get_typed_player(&self, guid: ObjectGuid) -> Option<&Player> {
        let record = self.map_object_record(guid)?;
        if record.kind() != AccessorObjectKind::Player {
            return None;
        }
        record.player()
    }

    pub fn get_typed_player_mut(&mut self, guid: ObjectGuid) -> Option<&mut Player> {
        let record = self.entity_world.get_mut(&guid)?;
        if record.kind() != AccessorObjectKind::Player {
            return None;
        }
        record.player_mut()
    }

    pub fn get_typed_corpse(&self, guid: ObjectGuid) -> Option<&Corpse> {
        let record = self.map_object_record(guid)?;
        if record.kind() != AccessorObjectKind::Corpse {
            return None;
        }
        record.corpse()
    }

    pub fn get_typed_corpse_mut(&mut self, guid: ObjectGuid) -> Option<&mut Corpse> {
        let record = self.entity_world.get_mut(&guid)?;
        if record.kind() != AccessorObjectKind::Corpse {
            return None;
        }
        record.corpse_mut()
    }

    pub fn get_typed_dynamic_object(&self, guid: ObjectGuid) -> Option<&DynamicObject> {
        let record = self.map_object_record(guid)?;
        if record.kind() != AccessorObjectKind::DynamicObject {
            return None;
        }
        record.dynamic_object()
    }

    pub fn get_typed_dynamic_object_mut(&mut self, guid: ObjectGuid) -> Option<&mut DynamicObject> {
        let record = self.entity_world.get_mut(&guid)?;
        if record.kind() != AccessorObjectKind::DynamicObject {
            return None;
        }
        record.dynamic_object_mut()
    }

    pub fn typed_combat_unit_guids_like_cpp(&self) -> Vec<ObjectGuid> {
        self.entity_world
            .iter()
            .filter_map(|(guid, record)| {
                matches!(
                    record.kind(),
                    AccessorObjectKind::Player | AccessorObjectKind::Creature
                )
                .then_some(*guid)
            })
            .collect()
    }

    pub fn get_dynamic_object(&self, guid: ObjectGuid) -> Option<&WorldObject> {
        self.map_object_by_kind(guid, &[AccessorObjectKind::DynamicObject])
    }

    pub fn get_corpse(&self, guid: ObjectGuid) -> Option<&WorldObject> {
        self.map_object_by_kind(guid, &[AccessorObjectKind::Corpse])
    }

}
