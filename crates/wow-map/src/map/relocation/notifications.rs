// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Relocation notification selection, delayed plans and visibility projections.

use super::*;

impl<Terrain, Lifecycle> Map<Terrain, Lifecycle>
where
    Terrain: TerrainGridLoader,
    Lifecycle: GridLifecycle,
{

    /// Live represented C++ `Map::Update` source selection for
    /// `ProcessRelocationNotifies(t_diff)` (`Map.cpp:692-717,797-805,830-905`).
    ///
    /// Source of truth stays map-owned canonical `entity_world`: the same typed
    /// player sources used by `ObjectUpdater` include viewpoints, far combat
    /// creatures, aura casters and summons. Active non-Players remain separate
    /// sources. The existing visit/relocation helpers consume marked cells and
    /// reset notify flags; packet, ObjectAccessor and AI side effects remain
    /// outside this map phase.
    pub fn process_live_relocation_notifies_like_cpp(
        &mut self,
        diff_ms: u32,
        visibility_notify_period_ms: i64,
    ) -> ProcessRelocationNotifiesOutcome {
        let mut player_sources = self.map_update_player_sources_for_current_tick_like_cpp();
        let active_non_player_guids = self.represented_active_non_player_sources_like_cpp();
        player_sources.sort_by_key(|source| source.player_guid);
        player_sources.dedup_by_key(|source| source.player_guid);

        let visit_plan = self.map_update_visit_plan_like_cpp(
            player_sources,
            active_non_player_guids,
            std::iter::empty(),
            diff_ms,
        );
        if !visit_plan.process_relocation_notifies {
            return ProcessRelocationNotifiesOutcome::default();
        }

        let centers =
            visit_plan
                .nearby_visit_centers
                .into_iter()
                .map(|guid| NearbyCellVisitCenter {
                    guid,
                    activation_radius: self.grid_activation_range_for_guid_like_cpp(guid),
                });
        let nearby_plan = self.visit_nearby_cells_of_like_cpp(centers);
        self.process_relocation_notifies_like_cpp(
            nearby_plan.marked_cells,
            diff_ms,
            visibility_notify_period_ms,
            std::iter::empty(),
        )
    }

    pub fn process_relocation_notifies_plan_like_cpp(
        &mut self,
        marked_cells: impl IntoIterator<Item = CellCoord>,
        diff_ms: u32,
        visibility_notify_period_ms: i64,
    ) -> RelocationNotifyProcessPlan {
        let marked_cells: HashSet<_> = marked_cells.into_iter().collect();
        let mut delayed_relocation_cells = Vec::new();
        let mut reset_notify_cells = Vec::new();
        let mut reset_timer_grids = Vec::new();
        let mut expired_active_grids = Vec::new();

        for grid_x in 0..MAX_NUMBER_OF_GRIDS {
            for grid_y in 0..MAX_NUMBER_OF_GRIDS {
                let coord = GridCoord::new(grid_x, grid_y);
                let Some(grid) = self.get_ngrid_mut(coord) else {
                    continue;
                };
                if grid.state() != GridStateKind::Active {
                    continue;
                }

                grid.info_mut()
                    .relocation_timer_mut()
                    .tracker_update(diff_ms);
                if !grid.info().relocation_timer().tracker_passed() {
                    continue;
                }

                expired_active_grids.push(coord);
                delayed_relocation_cells
                    .extend(marked_cells_in_grid_like_cpp(coord, &marked_cells));
            }
        }

        for coord in &expired_active_grids {
            let Some(grid) = self.get_ngrid_mut(*coord) else {
                continue;
            };
            if grid.state() != GridStateKind::Active {
                continue;
            }
            if !grid.info().relocation_timer().tracker_passed() {
                continue;
            }

            grid.info_mut()
                .relocation_timer_mut()
                .tracker_reset(diff_ms, visibility_notify_period_ms);
            reset_timer_grids.push(*coord);
            reset_notify_cells.extend(marked_cells_in_grid_like_cpp(*coord, &marked_cells));
        }

        RelocationNotifyProcessPlan {
            diff_ms,
            delayed_relocation_cells,
            reset_notify_cells,
            reset_timer_grids,
        }
    }

    pub fn process_relocation_notifies_like_cpp(
        &mut self,
        marked_cells: impl IntoIterator<Item = CellCoord>,
        diff_ms: u32,
        visibility_notify_period_ms: i64,
        invalid_non_self_viewpoints: impl IntoIterator<Item = ObjectGuid>,
    ) -> ProcessRelocationNotifiesOutcome {
        let process_plan = self.process_relocation_notifies_plan_like_cpp(
            marked_cells,
            diff_ms,
            visibility_notify_period_ms,
        );
        let delayed_plan = self.delayed_unit_relocation_for_cells_like_cpp(
            process_plan.delayed_relocation_cells.iter().copied(),
            invalid_non_self_viewpoints,
        );
        // C++ runs DelayedUnitRelocation's CreatureRelocationNotifier and
        // PlayerRelocationNotifier while NOTIFY_VISIBILITY_CHANGED is still set,
        // before ResetNotifier clears the cell. Rust exposes only represented
        // visibility/AI evidence here: no packets, sessions, ObjectAccessor fanout,
        // real UpdateObjectVisibility, or SendObjectUpdates are executed.
        let visibility_plans = self.delayed_unit_relocation_visibility_plans_like_cpp(
            &delayed_plan,
            self.delayed_player_relocation_contexts_from_plan_like_cpp(&delayed_plan),
            self.delayed_creature_relocation_contexts_from_plan_like_cpp(&delayed_plan),
        );
        let reset_outcome = self
            .reset_notify_flags_for_cells_like_cpp(process_plan.reset_notify_cells.iter().copied());

        ProcessRelocationNotifiesOutcome {
            process_plan,
            delayed_plan,
            visibility_plans,
            reset_outcome,
        }
    }

    pub fn delayed_unit_relocation_for_cells_like_cpp(
        &self,
        cells: impl IntoIterator<Item = CellCoord>,
        invalid_non_self_viewpoints: impl IntoIterator<Item = ObjectGuid>,
    ) -> DelayedUnitRelocationForCellsPlan {
        let invalid_non_self_viewpoints: HashSet<_> =
            invalid_non_self_viewpoints.into_iter().collect();
        let mut cell_plans = Vec::new();

        for cell_coord in cells {
            let nearby = self.exact_cell_guids_like_cpp(cell_coord);
            let creatures_needing_notify = nearby
                .world
                .creatures
                .iter()
                .chain(nearby.grid.creatures.iter())
                .copied()
                .filter(|guid| self.object_needs_notify_visibility(*guid));
            let mut plan = DelayedUnitRelocationPlan::from_nearby_like_cpp(
                &nearby,
                creatures_needing_notify,
                std::iter::empty::<ObjectGuid>(),
                std::iter::empty::<ObjectGuid>(),
            );
            let mut players: Vec<_> = nearby.world.players.iter().copied().collect();
            players.sort();
            for player_guid in players {
                let Some(viewpoint_guid) = self.player_viewpoint_guid_like_cpp(player_guid) else {
                    continue;
                };
                if !self.object_needs_notify_visibility(viewpoint_guid) {
                    continue;
                }
                if player_guid != viewpoint_guid
                    && (invalid_non_self_viewpoints.contains(&player_guid)
                        || invalid_non_self_viewpoints.contains(&viewpoint_guid)
                        || self.viewpoint_has_invalid_position_like_cpp(viewpoint_guid))
                {
                    plan.skipped_invalid_viewpoints.push(player_guid);
                    continue;
                }
                plan.player_relocations.push(player_guid);
            }
            sort_dedup(&mut plan.player_relocations);
            sort_dedup(&mut plan.skipped_invalid_viewpoints);
            if !plan.creature_relocations.is_empty()
                || !plan.player_relocations.is_empty()
                || !plan.skipped_invalid_viewpoints.is_empty()
            {
                cell_plans.push(DelayedUnitRelocationCellPlan { cell_coord, plan });
            }
        }

        DelayedUnitRelocationForCellsPlan { cell_plans }
    }

    pub fn delayed_unit_relocation_visibility_plans_like_cpp(
        &self,
        delayed_plan: &DelayedUnitRelocationForCellsPlan,
        player_contexts: impl IntoIterator<Item = DelayedPlayerRelocationContext>,
        creature_contexts: impl IntoIterator<Item = DelayedCreatureRelocationContext>,
    ) -> DelayedUnitRelocationVisibilityPlans {
        let player_contexts: HashMap<_, _> = player_contexts
            .into_iter()
            .map(|context| (context.player_guid, context))
            .collect();
        let creature_contexts: HashMap<_, _> = creature_contexts
            .into_iter()
            .map(|context| (context.creature_guid, context))
            .collect();
        let mut creature_plans = Vec::new();
        let mut player_plans = Vec::new();
        let mut skipped_missing_sources = Vec::new();
        let mut skipped_invalid_source_positions = Vec::new();
        let mut missing_player_contexts = Vec::new();

        for cell_plan in &delayed_plan.cell_plans {
            for creature_guid in &cell_plan.plan.creature_relocations {
                let Some(creature) = self.map_object(*creature_guid) else {
                    skipped_missing_sources.push(*creature_guid);
                    continue;
                };
                let position = creature.position();
                if !is_valid_map_coord_2d(position.x, position.y) {
                    skipped_invalid_source_positions.push(*creature_guid);
                    continue;
                }

                let nearby = self.nearby_cell_guids_like_cpp(
                    position.x,
                    position.y,
                    MAX_VISIBILITY_DISTANCE + creature.combat_reach(),
                );
                let player_seers_needing_notify = nearby
                    .world
                    .players
                    .iter()
                    .copied()
                    .filter(|guid| self.player_seer_needs_notify_visibility_like_cpp(*guid));
                let creatures_needing_notify = nearby
                    .world
                    .creatures
                    .iter()
                    .chain(nearby.grid.creatures.iter())
                    .copied()
                    .filter(|guid| self.object_needs_notify_visibility(*guid));
                let Some(creature_context) = creature_contexts.get(creature_guid) else {
                    skipped_missing_sources.push(*creature_guid);
                    continue;
                };
                let source_creature_alive = creature_context.source_creature_alive;
                let visibility_plan = CreatureRelocationVisibilityPlan::from_nearby_like_cpp(
                    *creature_guid,
                    source_creature_alive,
                    &nearby,
                    player_seers_needing_notify,
                    creatures_needing_notify,
                );
                creature_plans.push(CreatureDelayedRelocationVisibilityPlan {
                    creature_guid: *creature_guid,
                    cell_coord: cell_plan.cell_coord,
                    nearby,
                    visibility_plan,
                });
            }

            for player_guid in &cell_plan.plan.player_relocations {
                let Some(context) = player_contexts.get(player_guid) else {
                    missing_player_contexts.push(*player_guid);
                    continue;
                };
                let Some(viewpoint) = self.map_object(context.viewpoint_guid) else {
                    skipped_missing_sources.push(context.viewpoint_guid);
                    continue;
                };
                let position = viewpoint.position();
                if !is_valid_map_coord_2d(position.x, position.y) {
                    skipped_invalid_source_positions.push(context.viewpoint_guid);
                    continue;
                }

                let nearby = self.nearby_cell_guids_like_cpp(
                    position.x,
                    position.y,
                    MAX_VISIBILITY_DISTANCE + viewpoint.combat_reach(),
                );
                let player_seers_needing_notify = nearby
                    .world
                    .players
                    .iter()
                    .copied()
                    .filter(|guid| self.player_seer_needs_notify_visibility_like_cpp(*guid));
                let creatures_needing_notify = nearby
                    .world
                    .creatures
                    .iter()
                    .chain(nearby.grid.creatures.iter())
                    .copied()
                    .filter(|guid| self.object_needs_notify_visibility(*guid));
                let visibility_plan = PlayerRelocationVisibilityPlan::from_nearby_like_cpp(
                    *player_guid,
                    context.previous_client_guids.iter().copied(),
                    &nearby,
                    context.relocated_for_ai,
                    player_seers_needing_notify,
                    creatures_needing_notify,
                );
                player_plans.push(PlayerDelayedRelocationVisibilityPlan {
                    player_guid: *player_guid,
                    viewpoint_guid: context.viewpoint_guid,
                    cell_coord: cell_plan.cell_coord,
                    nearby,
                    visibility_plan,
                });
            }
        }

        sort_dedup(&mut skipped_missing_sources);
        sort_dedup(&mut skipped_invalid_source_positions);
        sort_dedup(&mut missing_player_contexts);

        DelayedUnitRelocationVisibilityPlans {
            creature_plans,
            player_plans,
            skipped_missing_sources,
            skipped_invalid_source_positions,
            missing_player_contexts,
        }
    }

    pub(in crate::map) fn delayed_player_relocation_contexts_from_plan_like_cpp(
        &self,
        delayed_plan: &DelayedUnitRelocationForCellsPlan,
    ) -> Vec<DelayedPlayerRelocationContext> {
        let mut player_guids: Vec<_> = delayed_plan
            .cell_plans
            .iter()
            .flat_map(|cell_plan| cell_plan.plan.player_relocations.iter().copied())
            .collect();
        sort_dedup(&mut player_guids);

        player_guids
            .into_iter()
            .filter_map(|player_guid| {
                let viewpoint_guid = self.player_viewpoint_guid_like_cpp(player_guid)?;
                Some(DelayedPlayerRelocationContext {
                    player_guid,
                    viewpoint_guid,
                    // Map-owned live relocation currently has no canonical client
                    // object-list source; keep this empty as an explicit visibility
                    // fanout gap rather than inventing session state.
                    previous_client_guids: Vec::new(),
                    relocated_for_ai: viewpoint_guid == player_guid,
                })
            })
            .collect()
    }

    fn delayed_creature_relocation_contexts_from_plan_like_cpp(
        &self,
        delayed_plan: &DelayedUnitRelocationForCellsPlan,
    ) -> Vec<DelayedCreatureRelocationContext> {
        let mut creature_guids: Vec<_> = delayed_plan
            .cell_plans
            .iter()
            .flat_map(|cell_plan| cell_plan.plan.creature_relocations.iter().copied())
            .collect();
        sort_dedup(&mut creature_guids);

        creature_guids
            .into_iter()
            .filter_map(|creature_guid| {
                let creature = self.get_typed_creature(creature_guid)?;
                Some(DelayedCreatureRelocationContext {
                    creature_guid,
                    source_creature_alive: creature.is_alive(),
                })
            })
            .collect()
    }

}
