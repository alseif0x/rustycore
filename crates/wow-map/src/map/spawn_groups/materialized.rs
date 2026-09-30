//! One Group/Conditions planner shared by Record wrappers and owned loaders.
//! Owned Actor results are dormant admission contracts, not Record refresh parity.

use super::*;

pub(super) enum ConditionResults {
    Record(Vec<SpawnGroupConditionUpdateOutcomeLikeCpp>),
    Owned(Vec<LoadedGridConditionOutcome>),
}

impl<Terrain, Lifecycle> Map<Terrain, Lifecycle>
where
    Terrain: TerrainGridLoader,
    Lifecycle: GridLifecycle,
{
    pub fn spawn_group_spawn_materialized<L>(
        &mut self,
        group: Option<&SpawnGroupTemplateData>,
        ignore_respawn: bool,
        force: bool,
        spawn_store: &SpawnStore,
        load_record: L,
    ) -> LoadedGridSpawnOutcome
    where
        L: FnMut(
            &mut Self,
            SpawnObjectType,
            SpawnId,
            bool,
        )
            -> Result<Option<LoadedGridMaterialization>, LoadedGridRespawnRecordsLikeCpp>,
    {
        self.spawn_group_materialized_core(
            group,
            ignore_respawn,
            force,
            spawn_store,
            load_record,
            LoadedGridReceipts::Owned(Vec::new()),
        )
    }

    pub(super) fn spawn_group_materialized_core<L>(
        &mut self,
        group: Option<&SpawnGroupTemplateData>,
        ignore_respawn: bool,
        force: bool,
        spawn_store: &SpawnStore,
        mut load_record: L,
        mut receipts: LoadedGridReceipts,
    ) -> LoadedGridSpawnOutcome
    where
        L: FnMut(
            &mut Self,
            SpawnObjectType,
            SpawnId,
            bool,
        )
            -> Result<Option<LoadedGridMaterialization>, LoadedGridRespawnRecordsLikeCpp>,
    {
        let Some(group) = group else {
            return receipts.finish(SpawnGroupSpawnOutcomeLikeCpp::blocked_missing_group(0));
        };
        if group.is_system() {
            return receipts.finish(SpawnGroupSpawnOutcomeLikeCpp::blocked_system_group(
                group.group_id,
            ));
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

                let plan = SpawnGroupSpawnLoadPlanLikeCpp {
                    object_type: member.object_type,
                    spawn_id: member.spawn_id,
                    force,
                };
                let materialization =
                    match load_record(self, member.object_type, member.spawn_id, force) {
                        Ok(Some(materialization)) => materialization,
                        failure => {
                            receipts.load_failed(plan, failure);
                            outcome.blocked_loaded_grid_spawn_loads += 1;
                            if member.object_type == SpawnObjectType::Creature {
                                outcome.blocked_loaded_grid_creature_loads += 1;
                            } else if member.object_type == SpawnObjectType::GameObject {
                                outcome.blocked_loaded_grid_gameobject_loads += 1;
                            }
                            continue;
                        }
                    };

                let admission = self.admit_loaded_grid_materialization(materialization);
                receipts.admitted(plan, admission, &mut outcome);
            }
        }

        receipts.finish(outcome)
    }

    pub fn update_spawn_group_conditions_materialized<'a, I, F, L>(
        &mut self,
        groups: I,
        spawn_store: &SpawnStore,
        meets_conditions: F,
        load_record: L,
    ) -> Vec<LoadedGridConditionOutcome>
    where
        I: IntoIterator<Item = &'a SpawnGroupTemplateData>,
        F: FnMut(&SpawnGroupTemplateData) -> bool,
        L: FnMut(
            &mut Self,
            SpawnObjectType,
            SpawnId,
            bool,
        )
            -> Result<Option<LoadedGridMaterialization>, LoadedGridRespawnRecordsLikeCpp>,
    {
        match self.update_spawn_group_conditions_core(
            groups,
            spawn_store,
            meets_conditions,
            load_record,
            LoadedGridReceipts::Owned(Vec::new()),
        ) {
            ConditionResults::Owned(outcomes) => outcomes,
            ConditionResults::Record(_) => unreachable!("Owned wrapper selects owned receipts"),
        }
    }

    pub(super) fn update_spawn_group_conditions_core<'a, I, F, L>(
        &mut self,
        groups: I,
        spawn_store: &SpawnStore,
        meets_conditions: F,
        mut load_record: L,
        receipts: LoadedGridReceipts,
    ) -> ConditionResults
    where
        I: IntoIterator<Item = &'a SpawnGroupTemplateData>,
        F: FnMut(&SpawnGroupTemplateData) -> bool,
        L: FnMut(
            &mut Self,
            SpawnObjectType,
            SpawnId,
            bool,
        )
            -> Result<Option<LoadedGridMaterialization>, LoadedGridRespawnRecordsLikeCpp>,
    {
        let groups = groups.into_iter().collect::<Vec<_>>();
        let planned_actions = self
            .plan_update_spawn_group_conditions_like_cpp(groups.iter().copied(), meets_conditions);

        let outcomes =
            planned_actions
                .into_iter()
                .zip(groups)
                .map(|((group_id, action), group)| {
                    let mut applied_change = None;
                    let mut despawn_outcome = None;
                    let mut spawn_outcome = None;
                    match action {
                        SpawnGroupConditionActionLikeCpp::SetInactive => {
                            applied_change =
                                Some(self.set_spawn_group_inactive_like_cpp(Some(group)));
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
                            spawn_outcome = Some(self.spawn_group_materialized_core(
                                Some(group),
                                ignore_respawn,
                                force,
                                spawn_store,
                                &mut load_record,
                                receipts.for_operation(),
                            ));
                        }
                        SpawnGroupConditionActionLikeCpp::Noop => {}
                    }

                    LoadedGridConditionOutcome {
                        group_id,
                        action,
                        applied_change,
                        despawn_outcome,
                        spawn_outcome,
                    }
                });
        match receipts {
            LoadedGridReceipts::RecordCompatibility => ConditionResults::Record(
                outcomes
                    .map(LoadedGridConditionOutcome::into_record_outcome)
                    .collect(),
            ),
            LoadedGridReceipts::Owned(_) => ConditionResults::Owned(outcomes.collect()),
        }
    }
}

#[cfg(test)]
mod tests;
