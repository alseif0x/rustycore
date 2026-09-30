// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Exact Creature admission stages, formation membership and live indexed equipment.

use super::*;

impl<Terrain, Lifecycle> Map<Terrain, Lifecycle>
where
    Terrain: TerrainGridLoader,
    Lifecycle: GridLifecycle,
{
    // Preserve the coordinator's captured admission facts; no new reads or context.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn add_creature_entry(
        &mut self,
        mut entry: ObjectEntry,
        kind: AccessorObjectKind,
        guid: ObjectGuid,
        cell: Cell,
        grid: GridCoord,
        active_object: bool,
        grid_loaded: bool,
        grid_created: bool,
    ) -> Result<AddToMapOutcome, AddToMapError> {
        entry
            .as_mut()
            .object_mut()
            .set_current_cell(cell.cell_x(), cell.cell_y());
        let previous = self.insert_object_entry(entry)?;

        let creature_store_inserted_before_add_to_world = self
            .map_object_record(guid)
            .is_some_and(|view| view.creature().is_some());
        let creature_spawn_indexed_before_add_to_world = self
            .map_object_record(guid)
            .and_then(|view| view.creature())
            .is_some_and(|creature| {
                let spawn_id = creature.spawn_id();
                spawn_id != 0
                    && self
                        .creature_spawn_id_store_guids_like_cpp(spawn_id)
                        .contains(&guid)
            });

        let creature_unit_add_to_world = self
            .entity_world
            .get_mut(&guid)
            .and_then(ObjectMut::creature_mut)
            .map(|creature| creature.unit_mut().add_to_world_like_cpp());
        let creature_search_formation = self
            .map_object_record(guid)
            .and_then(|view| view.creature())
            .map(Creature::search_formation_like_cpp);
        if let Some(outcome) = creature_search_formation {
            self.apply_creature_search_formation_like_cpp(guid, outcome);
        }

        let creature_aim_initialize = self
            .map_object_record(guid)
            .and_then(|view| view.creature())
            .map(Creature::aim_initialize_like_cpp);

        let creature_vehicle_reset = self
            .entity_world
            .get_mut(&guid)
            .and_then(ObjectMut::creature_mut)
            .and_then(|creature| {
                let context = creature
                    .add_to_world_vehicle_reset_context_like_cpp()?
                    .clone();
                let base_is_alive = creature.is_alive();
                creature
                    .unit_mut()
                    .subsystems_mut()
                    .vehicle
                    .reset_vehicle_kit_for_creature_add_to_world_like_cpp(&context, base_is_alive)
            });

        let creature_vehicle_install = self
            .entity_world
            .get_mut(&guid)
            .and_then(ObjectMut::creature_mut)
            .and_then(|creature| {
                let install = creature
                    .unit_mut()
                    .subsystems_mut()
                    .vehicle
                    .install_vehicle_kit_like_cpp();
                install.had_kit.then_some(install)
            });

        let creature_zone_script_create = self
            .map_object_record(guid)
            .and_then(|view| view.creature())
            .is_some()
            .then_some(CreatureZoneScriptCreateOutcomeLikeCpp {
                guid,
                represented_callback: true,
                script_dispatch_represented: false,
            });
        let add_to_map_tail =
            self.represent_add_to_map_post_add_to_world_tail_like_cpp(kind, guid, active_object);
        if kind == AccessorObjectKind::Transport {
            self.mark_transport_players_for_visibility_like_cpp(guid);
        } else {
            self.mark_nearby_players_for_visibility_like_cpp(guid);
        }

        return Ok(AddToMapOutcome {
            guid,
            cell: cell.cell_coord(),
            grid,
            inserted: previous.is_none(),
            already_in_world: false,
            grid_created,
            grid_loaded,
            inserted_into_cell: true,
            gameobject_model_insert: None,
            gameobject_collision_enable: None,
            gameobject_zone_script_create: None,
            gameobject_store_inserted_before_add_to_world: None,
            gameobject_spawn_indexed_before_add_to_world: None,
            creature_store_inserted_before_add_to_world: Some(
                creature_store_inserted_before_add_to_world,
            ),
            creature_spawn_indexed_before_add_to_world: Some(
                creature_spawn_indexed_before_add_to_world,
            ),
            creature_unit_add_to_world,
            creature_search_formation,
            creature_aim_initialize,
            creature_vehicle_reset,
            creature_vehicle_install,
            creature_zone_script_create,
            add_to_map_tail,
        });
    }
    pub fn creature_group_holder_member_count_like_cpp(&self, leader_spawn_id: SpawnId) -> usize {
        self.creature_group_holder_like_cpp
            .get(&leader_spawn_id)
            .map_or(0, HashSet::len)
    }

    /// Bounded map-owned consumer for C++ `GameEventMgr::ChangeEquipOrModel` live creature loop.
    ///
    /// Mirrors the `GetCreatureBySpawnIdStore().equal_range(spawn_id)` direction over the
    /// map-owned creature by-spawn index. This only mutates canonical `MapObjectRecord::Creature`
    /// equipment/display fields; it does not implement full `Creature::LoadEquipment`, DB2
    /// `GetCreatureModelInfo`, values/session fanout, scripts, AI, or ObjectAccessor side effects.
    pub fn change_game_event_equip_or_model_by_spawn_id_like_cpp(
        &mut self,
        spawn_id: SpawnId,
        equipment_id: u8,
        model_id: u32,
        model_info_available: bool,
    ) -> GameEventChangeEquipOrModelLiveOutcomeLikeCpp {
        let guids = self.creature_spawn_id_store_guids_like_cpp(spawn_id);
        let mut outcome = GameEventChangeEquipOrModelLiveOutcomeLikeCpp {
            spawn_id,
            indexed_guids: guids.len(),
            ..GameEventChangeEquipOrModelLiveOutcomeLikeCpp::default()
        };

        for guid in guids {
            let Some(record) = self.entity_world.get_mut(&guid) else {
                outcome.stale_index_or_wrong_kind += 1;
                continue;
            };
            let Some(creature) = record.creature_mut() else {
                outcome.stale_index_or_wrong_kind += 1;
                continue;
            };
            if creature.spawn_id() != spawn_id {
                outcome.stale_index_or_wrong_kind += 1;
                continue;
            }

            outcome.live_creatures_mutated += 1;
            creature.set_equipment_id_like_cpp(equipment_id);
            outcome.equipment_changed += 1;

            if model_id > 0 && creature.unit().data().display_id as u32 != model_id {
                if model_info_available {
                    creature.set_display_id(model_id, true, None);
                    outcome.display_changed += 1;
                } else {
                    outcome.model_validation_unavailable += 1;
                }
            }
        }

        outcome
    }

    pub(super) fn apply_creature_search_formation_like_cpp(
        &mut self,
        current_guid: ObjectGuid,
        outcome: CreatureSearchFormationOutcomeLikeCpp,
    ) {
        if !outcome.add_to_group_requested {
            return;
        }

        let Some(leader_spawn_id) = outcome.leader_spawn_id else {
            return;
        };

        let stale_member_guids = self.creature_spawn_id_store_guids_like_cpp(outcome.spawn_id);
        let group = self
            .creature_group_holder_like_cpp
            .entry(leader_spawn_id)
            .or_default();
        for stale_guid in stale_member_guids {
            if stale_guid != current_guid {
                group.remove(&stale_guid);
            }
        }
        group.insert(current_guid);
    }
}
