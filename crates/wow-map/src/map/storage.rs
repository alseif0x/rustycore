// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Grid and cell storage, terrain loading and object lookup.

use super::*;

mod corpses;
mod entries;
mod grids;
mod guid_provenance;
mod object_access;

impl<Terrain, Lifecycle> Map<Terrain, Lifecycle>
where
    Terrain: TerrainGridLoader,
    Lifecycle: GridLifecycle,
{
    /// C++ `Map::AddFarSpellCallback` represented as a map-owned FIFO action queue.
    ///
    /// This helper only accepts explicit represented actions; it does not expose a
    /// general closure/callback runtime or real Spell/Aura side effects.
    pub fn add_far_spell_callback_like_cpp(
        &mut self,
        callback: RepresentedFarSpellCallbackLikeCpp,
    ) {
        self.far_spell_callbacks_like_cpp.push_back(callback);
    }

    /// C++ `Trinity::ObjectUpdater` visits creature containers reachable from
    /// the map's loaded grids, not the global object accessor/store. Keeping the
    /// visitation anchored to cells prevents unloaded-grid records from being
    /// updated after `Map::UnloadGrid` has removed their NGrid.
    pub(super) fn object_updater_creature_guids_like_cpp(&self) -> Vec<ObjectGuid> {
        let mut creature_guids = Vec::new();
        for grid in self.grids.iter().filter_map(|grid| grid.as_deref()) {
            grid.visit_all_grids(|cell| {
                creature_guids.extend(cell.grid_objects.creatures.iter().copied());
                creature_guids.extend(cell.world_objects.creatures.iter().copied());
            });
        }
        sort_dedup(&mut creature_guids);
        creature_guids
    }

    /// C++ `Map::AddObjectToSwitchList` represented over canonical map records.
    ///
    /// C++ anchors:
    /// - `Map.h:345-346` declares `AddObjectToRemoveList` beside
    ///   `AddObjectToSwitchList`; `Map.h:651-652` owns both queues.
    /// - `Map.cpp:2557-2572` accepts only `TYPEID_UNIT`, inserts first toggle,
    ///   cancels an opposite pending toggle, and aborts on duplicate direction.
    /// - `Object.cpp:910-915` shows `WorldObject::SetWorldObject(on)` enqueues
    ///   through the owning map only when the object is already in world.
    pub fn add_object_to_switch_list_like_cpp(
        &mut self,
        guid: ObjectGuid,
        on: bool,
    ) -> AddObjectToSwitchListOutcomeLikeCpp {
        let Some(record) = self.map_object_record(guid) else {
            return AddObjectToSwitchListOutcomeLikeCpp {
                guid,
                on,
                status: AddObjectToSwitchListStatusLikeCpp::MissingOrStale,
            };
        };

        debug_assert_eq!(record.object().map_id(), self.map_id);
        debug_assert_eq!(record.object().instance_id(), self.instance_id);

        if !switch_list_unit_kind_like_cpp(record.kind()) {
            return AddObjectToSwitchListOutcomeLikeCpp {
                guid,
                on,
                status: AddObjectToSwitchListStatusLikeCpp::IgnoredNonUnit,
            };
        }

        match self.objects_to_switch.get(&guid).copied() {
            None => {
                self.objects_to_switch.insert(guid, on);
                AddObjectToSwitchListOutcomeLikeCpp {
                    guid,
                    on,
                    status: AddObjectToSwitchListStatusLikeCpp::Queued,
                }
            }
            Some(existing) if existing != on => {
                self.objects_to_switch.remove(&guid);
                AddObjectToSwitchListOutcomeLikeCpp {
                    guid,
                    on,
                    status: AddObjectToSwitchListStatusLikeCpp::CancelledOppositeToggle,
                }
            }
            Some(_) => AddObjectToSwitchListOutcomeLikeCpp {
                guid,
                on,
                status: AddObjectToSwitchListStatusLikeCpp::DuplicateSameDirectionAbort,
            },
        }
    }
}
