//! Frozen pre-refinement bodies, used only as a Record compatibility oracle.

use super::*;

impl Map {
    pub(super) fn original_group<L>(
        &mut self,
        group: Option<&SpawnGroupTemplateData>,
        ignore_respawn: bool,
        force: bool,
        spawn_store: &SpawnStore,
        mut load_record: L,
    ) -> SpawnGroupSpawnOutcomeLikeCpp
    where
        L: FnMut(
            &mut Self,
            SpawnObjectType,
            SpawnId,
            bool,
        ) -> Option<LoadedGridRespawnRecordsLikeCpp>,
    {
        let Some(group) = group else {
            return SpawnGroupSpawnOutcomeLikeCpp::blocked_missing_group(0);
        };
        if group.is_system() {
            return SpawnGroupSpawnOutcomeLikeCpp::blocked_system_group(group.group_id);
        }

        let mut outcome = SpawnGroupSpawnOutcomeLikeCpp::executed(group.group_id);
        outcome.applied_active_change =
            Some(self.set_spawn_group_active_like_cpp(Some(group), true));

        if let Some(members) = spawn_store.spawn_group_members(group.group_id) {
            let members = members.iter().copied().collect::<Vec<_>>();
            for member in members {
                let Some(spawn_data) = spawn_store.spawn_data(member.object_type, member.spawn_id)
                else {
                    outcome.stale_index_entries += 1;
                    continue;
                };
                if spawn_data.map_id != self.map_id {
                    continue;
                }

                outcome.metadata_entries += 1;
                match member.object_type {
                    SpawnObjectType::Creature | SpawnObjectType::GameObject => {
                        if force || ignore_respawn {
                            if self
                                .remove_respawn_time_like_cpp(member.object_type, member.spawn_id)
                                .is_some()
                            {
                                outcome.respawn_timers_removed += 1;
                            } else {
                                outcome.respawn_timers_missing += 1;
                            }
                        }

                        if self.get_respawn_time_like_cpp(member.object_type, member.spawn_id) != 0
                        {
                            outcome.skipped_respawn_timer_active += 1;
                            continue;
                        }

                        if !force {
                            let live_blocks = match member.object_type {
                                SpawnObjectType::Creature => self
                                    .get_creature_by_spawn_id_like_cpp(member.spawn_id)
                                    .is_some_and(Creature::is_alive),
                                SpawnObjectType::GameObject => self
                                    .get_gameobject_by_spawn_id_like_cpp(member.spawn_id)
                                    .is_some(),
                                SpawnObjectType::AreaTrigger => false,
                            };
                            if live_blocks {
                                outcome.skipped_live_object_active += 1;
                                continue;
                            }
                        }
                    }
                    SpawnObjectType::AreaTrigger => {
                        outcome.skipped_no_respawn_map += 1;
                        continue;
                    }
                }

                if !spawn_data.spawn_difficulties.contains(&self.spawn_mode()) {
                    outcome.skipped_difficulty_mismatch += 1;
                    continue;
                }

                let cell = cell_from_world(spawn_data.spawn_point.x, spawn_data.spawn_point.y);
                let grid = GridCoord::new(cell.grid_x(), cell.grid_y());
                if !self.is_grid_loaded(grid) {
                    outcome.skipped_unloaded_grid += 1;
                    continue;
                }

                outcome.load_plans.push(SpawnGroupSpawnLoadPlanLikeCpp {
                    object_type: member.object_type,
                    spawn_id: member.spawn_id,
                    force,
                });

                let Some(records) = load_record(self, member.object_type, member.spawn_id, force)
                else {
                    outcome.blocked_loaded_grid_spawn_loads += 1;
                    if member.object_type == SpawnObjectType::Creature {
                        outcome.blocked_loaded_grid_creature_loads += 1;
                    } else if member.object_type == SpawnObjectType::GameObject {
                        outcome.blocked_loaded_grid_gameobject_loads += 1;
                    }
                    continue;
                };

                let (_, loaded_grid_primary_record, primary_result) = self
                    .admit_loaded_grid_materialization(LoadedGridMaterialization::records(records))
                    .into_record_parts();
                match primary_result {
                    Ok(_outcome) => {
                        outcome.executed_loaded_grid_spawns += 1;
                        outcome
                            .loaded_grid_primary_records
                            .push(loaded_grid_primary_record);
                    }
                    Err(_error) => outcome.blocked_loaded_grid_spawn_add_to_map += 1,
                }
            }
        }

        outcome
    }

    pub(super) fn original_conditions<'a, I, F, L>(
        &mut self,
        groups: I,
        spawn_store: &SpawnStore,
        meets_conditions: F,
        mut load_record: L,
    ) -> Vec<SpawnGroupConditionUpdateOutcomeLikeCpp>
    where
        I: IntoIterator<Item = &'a SpawnGroupTemplateData>,
        F: FnMut(&SpawnGroupTemplateData) -> bool,
        L: FnMut(
            &mut Self,
            SpawnObjectType,
            SpawnId,
            bool,
        ) -> Option<LoadedGridRespawnRecordsLikeCpp>,
    {
        let groups = groups.into_iter().collect::<Vec<_>>();
        let planned_actions = self
            .plan_update_spawn_group_conditions_like_cpp(groups.iter().copied(), meets_conditions);

        planned_actions
            .into_iter()
            .zip(groups)
            .map(|((group_id, action), group)| {
                let mut applied_change = None;
                let mut despawn_outcome = None;
                let mut spawn_outcome = None;
                match action {
                    SpawnGroupConditionActionLikeCpp::SetInactive => {
                        applied_change = Some(self.set_spawn_group_inactive_like_cpp(Some(group)));
                    }
                    SpawnGroupConditionActionLikeCpp::Despawn {
                        delete_respawn_times,
                    } => {
                        despawn_outcome = Some(self.spawn_group_despawn_like_cpp(
                            Some(group),
                            delete_respawn_times,
                            spawn_store,
                        ));
                    }
                    SpawnGroupConditionActionLikeCpp::Spawn {
                        ignore_respawn,
                        force,
                    } => {
                        spawn_outcome = Some(self.original_group(
                            Some(group),
                            ignore_respawn,
                            force,
                            spawn_store,
                            &mut load_record,
                        ));
                    }
                    SpawnGroupConditionActionLikeCpp::Noop => {}
                }

                SpawnGroupConditionUpdateOutcomeLikeCpp {
                    group_id,
                    action,
                    applied_change,
                    despawn_outcome,
                    spawn_outcome,
                }
            })
            .collect()
    }
}
