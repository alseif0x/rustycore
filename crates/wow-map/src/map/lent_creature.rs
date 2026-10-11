// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1263 F6-8D3b-3: lending a canonical creature out of its map record.
//!
//! The session mutation root mutates a canonical creature by value: it takes the
//! incarnation out of its `MapObjectRecord`, lends it to the mutation and puts
//! it back. C++ never does that — a `Creature` stays in its `Map` and is mutated
//! through its pointer — so the return is not allowed to fail or to leave the
//! map's derived indexes stale:
//!
//! - the creature goes back into the record it left, at the position whose cell
//!   still holds its GUID, with the map binding it left with;
//! - a position the borrower changed is then applied through the map's own
//!   relocation, which is C++ `Unit::UpdatePosition` → `Map::CreatureRelocation`
//!   (`Map.cpp:1042-1072`): same cell, the position is updated in place;
//!   another cell, the GUID moves cells; an unloaded target grid, nothing moves
//!   (`if (!respawnRelocationOnFail && !getNGrid(...)) return;`).

use super::*;

/// Where a lent creature came from: what it must return to.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LentCreatureOriginLikeCpp {
    pub guid: ObjectGuid,
    pub map_id: u32,
    pub instance_id: u32,
    /// The position whose cell holds the GUID while the creature is lent.
    pub position: Position,
}

/// What putting a lent creature back did.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LentCreatureReturnLikeCpp {
    Returned {
        /// The borrower rebound or unbound the creature's map, and the binding
        /// it left with was put back so the record could return.
        binding_restored: bool,
        /// The relocation to the borrower's position, when the borrower moved
        /// the creature. A blocked or refused relocation leaves the creature at
        /// its origin, in the cell that holds its GUID.
        relocation: Option<Result<MapObjectRelocationOutcome, MapObjectRelocationError>>,
    },
    /// The record could not be rebuilt; the creature is gone. Unreachable while
    /// a creature's GUID cannot be changed, and reported loudly by the caller.
    Lost { error: MapObjectStoreError },
}

impl<Terrain, Lifecycle> Map<Terrain, Lifecycle>
where
    Terrain: TerrainGridLoader,
    Lifecycle: GridLifecycle,
{
    /// Take the canonical creature `guid` out of its map record, by value.
    ///
    /// The GUID stays in its grid cell: the caller must give the creature back
    /// with [`Self::return_lent_creature_like_cpp`]. A record that is not a
    /// typed creature is left in place and `None` is returned.
    pub fn lend_creature_like_cpp(
        &mut self,
        guid: ObjectGuid,
    ) -> Option<(Creature, LentCreatureOriginLikeCpp)> {
        self.get_typed_creature(guid)?;
        let record = self.remove_map_object(guid)?;
        match record.into_creature() {
            Ok(creature) => {
                let origin = LentCreatureOriginLikeCpp {
                    guid,
                    map_id: self.map_id,
                    instance_id: self.instance_id,
                    position: creature.unit().world().position(),
                };
                Some((*creature, origin))
            }
            Err(record) => {
                let _ = self.insert_map_object_record(record);
                None
            }
        }
    }

    /// Give a creature lent by [`Self::lend_creature_like_cpp`] back to its
    /// record, then apply the position the borrower left it at as a map
    /// relocation (C++ `Map::CreatureRelocation`).
    pub fn return_lent_creature_like_cpp(
        &mut self,
        mut creature: Creature,
        origin: LentCreatureOriginLikeCpp,
    ) -> LentCreatureReturnLikeCpp {
        let world = creature.unit_mut().world_mut();
        let moved_to = world.position();
        let binding_restored = !world.has_current_map()
            || world.map_id() != origin.map_id
            || world.instance_id() != origin.instance_id;
        if binding_restored {
            world.restore_lent_map_binding_like_cpp(origin.map_id, origin.instance_id);
        }
        // Back where its cell says it is; the move, if any, is the relocation's.
        world.relocate(origin.position);
        let record = match MapObjectRecord::new_creature(creature) {
            Ok(record) => record,
            Err(error) => {
                return LentCreatureReturnLikeCpp::Lost {
                    error: MapObjectStoreError::InvalidRecord(error),
                };
            }
        };
        if let Err(error) = self.insert_map_object_record(record) {
            return LentCreatureReturnLikeCpp::Lost { error };
        }
        let relocation = (moved_to != origin.position)
            .then(|| self.relocate_map_object_like_cpp(origin.guid, moved_to));
        LentCreatureReturnLikeCpp::Returned {
            binding_restored,
            relocation,
        }
    }

    /// C++ `Map::CreatureRelocation` for a creature whose owner already wrote
    /// its new position in place (the movement generators' `UpdatePosition`).
    ///
    /// `from` is the position the creature had when its GUID was last placed
    /// in a cell. The creature is put back there and relocated to where it was
    /// moved, so its cell follows it. `None` when it is not a typed creature or
    /// did not move.
    pub fn relocate_creature_moved_in_place_like_cpp(
        &mut self,
        guid: ObjectGuid,
        from: Position,
    ) -> Option<Result<MapObjectRelocationOutcome, MapObjectRelocationError>> {
        let world = self.get_typed_creature_mut(guid)?.unit_mut().world_mut();
        let moved_to = world.position();
        if moved_to == from {
            return None;
        }
        world.relocate(from);
        Some(self.relocate_map_object_like_cpp(guid, moved_to))
    }
}
