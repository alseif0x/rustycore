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
//!   another cell, the GUID moves cells; an unloaded target grid triggers the
//!   respawn-or-removal fallback (`Map::CreatureRespawnRelocation`).

use super::*;
use crate::map_rules::map_record_unit_mut_like_cpp;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CreatureRespawnRelocationOutcomeLikeCpp {
    RespawnRelocated,
    RemoveListQueued,
    PetRemoveRequested,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CreatureRelocationResultLikeCpp {
    pub requested: Result<MapObjectRelocationOutcome, MapObjectRelocationError>,
    pub fallback: Option<CreatureRespawnRelocationOutcomeLikeCpp>,
}

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
        /// The requested relocation and, when its grid is unloaded, the
        /// subsequent home-respawn or removal fallback.
        relocation: Option<CreatureRelocationResultLikeCpp>,
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
    pub(super) fn respawn_relocate_creature_after_unloaded_grid_like_cpp(
        &mut self,
        guid: ObjectGuid,
    ) -> CreatureRespawnRelocationOutcomeLikeCpp {
        let (home, is_pet) = {
            let Some(record) = self.entity_world.get_mut(&guid) else {
                return CreatureRespawnRelocationOutcomeLikeCpp::Failed;
            };
            match record.kind() {
                AccessorObjectKind::Creature => {
                    let Some(creature) = record.creature_mut() else {
                        return CreatureRespawnRelocationOutcomeLikeCpp::Failed;
                    };
                    let home = creature.ai_home_position();
                    creature.combat_stop();
                    (home, false)
                }
                AccessorObjectKind::Pet => {
                    let Some(pet) = record.pet_mut() else {
                        return CreatureRespawnRelocationOutcomeLikeCpp::Failed;
                    };
                    let home = pet.creature().ai_home_position();
                    pet.creature_mut().combat_stop();
                    (home, true)
                }
                _ => return CreatureRespawnRelocationOutcomeLikeCpp::Failed,
            }
        };
        {
            let Some(record) = self.entity_world.get_mut(&guid) else {
                return CreatureRespawnRelocationOutcomeLikeCpp::Failed;
            };
            let Some(unit) = map_record_unit_mut_like_cpp(record) else {
                return CreatureRespawnRelocationOutcomeLikeCpp::Failed;
            };
            unit.subsystems_mut().motion.clear_active();
        }

        let outcome = self.relocate_map_object_like_cpp(guid, home);
        if matches!(&outcome, Ok(outcome) if outcome.relocated) {
            {
                let Some(record) = self.entity_world.get_mut(&guid) else {
                    return CreatureRespawnRelocationOutcomeLikeCpp::Failed;
                };
                let Some(unit) = map_record_unit_mut_like_cpp(record) else {
                    return CreatureRespawnRelocationOutcomeLikeCpp::Failed;
                };
                unit.subsystems_mut().motion.direct_initialize_like_cpp();
            }
            self.mark_nearby_players_for_visibility_like_cpp(guid);
            return CreatureRespawnRelocationOutcomeLikeCpp::RespawnRelocated;
        }

        if is_pet {
            return CreatureRespawnRelocationOutcomeLikeCpp::PetRemoveRequested;
        }
        if self.add_object_to_remove_list_like_cpp(guid).queued {
            CreatureRespawnRelocationOutcomeLikeCpp::RemoveListQueued
        } else {
            CreatureRespawnRelocationOutcomeLikeCpp::Failed
        }
    }

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

    fn relocate_creature_with_respawn_fallback_like_cpp(
        &mut self,
        guid: ObjectGuid,
        requested_position: Position,
    ) -> CreatureRelocationResultLikeCpp {
        let requested = self.relocate_map_object_like_cpp(guid, requested_position);
        if matches!(&requested, Ok(outcome) if outcome.relocated) {
            self.mark_nearby_players_for_visibility_like_cpp(guid);
        }
        let fallback = matches!(&requested, Ok(outcome) if outcome.blocked_by_unloaded_grid)
            .then(|| self.respawn_relocate_creature_after_unloaded_grid_like_cpp(guid));
        CreatureRelocationResultLikeCpp {
            requested,
            fallback,
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
        let relocation = (moved_to != origin.position).then(|| {
            if is_valid_map_coord_2d(moved_to.x, moved_to.y) {
                self.mark_nearby_players_for_visibility_like_cpp(origin.guid);
            }
            self.relocate_creature_with_respawn_fallback_like_cpp(origin.guid, moved_to)
        });
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
    ) -> Option<CreatureRelocationResultLikeCpp> {
        let world = self.get_typed_creature_mut(guid)?.unit_mut().world_mut();
        let moved_to = world.position();
        if moved_to == from {
            return None;
        }
        world.relocate(from);
        if is_valid_map_coord_2d(moved_to.x, moved_to.y) {
            self.mark_nearby_players_for_visibility_like_cpp(guid);
        }
        Some(self.relocate_creature_with_respawn_fallback_like_cpp(guid, moved_to))
    }
}
