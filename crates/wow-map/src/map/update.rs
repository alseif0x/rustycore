// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! `Map::Update` phase driving, in the C++-anchored order.

use super::*;

impl<Terrain, Lifecycle> Map<Terrain, Lifecycle>
where
    Terrain: TerrainGridLoader,
    Lifecycle: GridLifecycle,
{
    /// Represents the first statement in C++ `Map::Update(uint32 t_diff)`:
    /// `_dynamicTree.update(t_diff)` (`Map.cpp:666-668`).
    ///
    /// This is C++-shaped map-owned state only. It mirrors
    /// `DynTreeImpl::update` (`DynamicTree.cpp:90-101`): return early when the
    /// represented model-key set is empty; otherwise consume a TimeTracker-like
    /// remaining timer; when passed, reset to `CHECK_TREE_PERIOD` (200ms) and
    /// clear `unbalanced_times` only if it was positive, representing `balance()`.
    /// No real BIH/collision/geometry runtime is claimed.
    pub fn update_dynamic_tree_like_cpp(
        &mut self,
        diff_ms: u32,
    ) -> DynamicMapTreeUpdateSummaryLikeCpp {
        let timer_before_ms = self.dynamic_tree_rebalance_timer_remaining_ms_like_cpp;
        let unbalanced_before = self.dynamic_tree_unbalanced_times_like_cpp;
        let empty = self.dynamic_tree_model_keys_like_cpp.is_empty();

        if empty {
            return DynamicMapTreeUpdateSummaryLikeCpp {
                diff_ms,
                empty,
                timer_before_ms,
                timer_after_ms: timer_before_ms,
                timer_passed: false,
                timer_reset_to_ms: None,
                unbalanced_before,
                balanced: false,
                unbalanced_after: unbalanced_before,
            };
        }

        let timer_passed = diff_ms >= timer_before_ms;
        let mut timer_after_ms = timer_before_ms.saturating_sub(diff_ms);
        let mut balanced = false;
        let mut unbalanced_after = unbalanced_before;
        let timer_reset_to_ms = if timer_passed {
            timer_after_ms = DYNAMIC_MAP_TREE_CHECK_PERIOD_MS_LIKE_CPP;
            if unbalanced_before > 0 {
                self.dynamic_tree_unbalanced_times_like_cpp = 0;
                unbalanced_after = 0;
                balanced = true;
            }
            Some(DYNAMIC_MAP_TREE_CHECK_PERIOD_MS_LIKE_CPP)
        } else {
            None
        };

        self.dynamic_tree_rebalance_timer_remaining_ms_like_cpp = timer_after_ms;

        DynamicMapTreeUpdateSummaryLikeCpp {
            diff_ms,
            empty,
            timer_before_ms,
            timer_after_ms,
            timer_passed,
            timer_reset_to_ms,
            unbalanced_before,
            balanced,
            unbalanced_after,
        }
    }

    pub fn personal_phase_tracker(&self) -> &MultiPersonalPhaseTracker {
        &self.personal_phase_tracker
    }

    #[cfg(test)]
    pub(crate) fn register_personal_phase_object_for_test(
        &mut self,
        phase_id: u32,
        phase_owner: ObjectGuid,
        object: ObjectGuid,
    ) {
        self.personal_phase_tracker
            .register_tracked_object(phase_id, phase_owner, object);
    }

    #[cfg(test)]
    pub(crate) fn mark_personal_phases_for_deletion_for_test(&mut self, phase_owner: ObjectGuid) {
        self.personal_phase_tracker
            .mark_all_phases_for_deletion(phase_owner);
    }

    /// C++ `GetMultiPersonalPhaseTracker().Update(this, t_diff)` represented on `Map`.
    ///
    /// C++ anchors:
    /// - `Map.cpp:797-798` calls the map-owned multi personal phase tracker during
    ///   `Map::Update` before deferred move/remove-list processing.
    /// - `PersonalPhaseTracker.cpp:62-78,106-113,192-202` expires per-owner phases,
    ///   calls `Map::AddObjectToRemoveList` for tracked objects, clears phase
    ///   object/grid sets, and removes empty owner trackers.
    ///
    /// Rust ownership: `personal_phase_tracker.update(diff_ms)` is the sole source
    /// of expired GUIDs; `map_objects` remains canonical for real records/removal.
    /// This seam does not drain `objects_to_remove`, rebuild objects from external
    /// caches, or claim session/ObjectAccessor/visibility/DB/script behavior.
    pub fn update_personal_phase_tracker_like_cpp(
        &mut self,
        diff_ms: u32,
    ) -> PersonalPhaseTrackerUpdateSummaryLikeCpp {
        let expired_guids = self.personal_phase_tracker.update(diff_ms);
        let mut summary = PersonalPhaseTrackerUpdateSummaryLikeCpp {
            expired_objects: expired_guids.len(),
            ..Default::default()
        };

        for guid in expired_guids {
            let outcome = self.add_object_to_remove_list_like_cpp(guid);
            if outcome.queued {
                summary.remove_queued += 1;
            }
            if outcome.duplicate {
                summary.duplicate_queued += 1;
            }
            if outcome.missing_or_stale {
                summary.missing_or_stale += 1;
            }
            if outcome.unsupported_kind.is_some() {
                summary.unsupported_kinds += 1;
            }
        }

        summary
    }

    /// Map-owned seam for the non-aura branch of C++ `DynamicObject::Update`.
    ///
    /// C++ anchors:
    /// - `DynamicObject.cpp:136-165` asserts same-map caster, updates aura-bound
    ///   DynamicObjects through the aura path (unsupported here), otherwise
    ///   decrements `_duration` by `p_time` or marks expired, then calls `Remove()`
    ///   on expiry and `sScriptMgr->OnDynamicObjectUpdate` otherwise.
    /// - `DynamicObject.cpp:167-171` makes `Remove()` enqueue through
    ///   `AddObjectToRemoveList()` only when the object is in world.
    /// - `Map.cpp:2547-2555` owns `AddObjectToRemoveList` cleanup and deferred
    ///   remove-list insertion, represented by `add_object_to_remove_list_like_cpp`.
    ///
    /// Ownership: source-of-truth is canonical `Map::map_objects`; this helper
    /// mutates only the typed `MapObjectRecord::DynamicObject` duration and, after
    /// dropping that mutable borrow, enqueues the same GUID through the existing
    /// remove-list facade. Aura-bound DynamicObjects only record represented
    /// `Aura::UpdateOwner` evidence and removed/expired checks. It does not drain removal, run scripts,
    /// write ObjectAccessor/session mirrors, fan out visibility, send packets, or
    /// create fallback records.
    pub fn update_dynamic_object_like_cpp(
        &mut self,
        dynamic_object_guid: ObjectGuid,
        elapsed_ms: u32,
    ) -> DynamicObjectUpdateOutcomeLikeCpp {
        let Some(record) = self.map_object_record(dynamic_object_guid) else {
            return DynamicObjectUpdateOutcomeLikeCpp {
                dynamic_object_guid,
                elapsed_ms,
                status: DynamicObjectUpdateStatusLikeCpp::MissingDynamicObject,
                duration_before_ms: None,
                duration_after_ms: None,
                aura_update_owner_calls_before: None,
                aura_update_owner_calls_after: None,
                script_update_would_run: false,
                remove_list: None,
            };
        };

        if record.kind() != AccessorObjectKind::DynamicObject {
            return DynamicObjectUpdateOutcomeLikeCpp {
                dynamic_object_guid,
                elapsed_ms,
                status: DynamicObjectUpdateStatusLikeCpp::NotDynamicObject,
                duration_before_ms: None,
                duration_after_ms: None,
                aura_update_owner_calls_before: None,
                aura_update_owner_calls_after: None,
                script_update_would_run: false,
                remove_list: None,
            };
        }

        let Some(dynamic_object) = record.dynamic_object() else {
            return DynamicObjectUpdateOutcomeLikeCpp {
                dynamic_object_guid,
                elapsed_ms,
                status: DynamicObjectUpdateStatusLikeCpp::NotDynamicObject,
                duration_before_ms: None,
                duration_after_ms: None,
                aura_update_owner_calls_before: None,
                aura_update_owner_calls_after: None,
                script_update_would_run: false,
                remove_list: None,
            };
        };

        let duration_before_ms = dynamic_object.duration_ms();
        let aura_update_owner_calls_before = dynamic_object.represented_aura_update_owner_count();
        if !dynamic_object.world().object().is_in_world() {
            return DynamicObjectUpdateOutcomeLikeCpp {
                dynamic_object_guid,
                elapsed_ms,
                status: DynamicObjectUpdateStatusLikeCpp::NotInWorld,
                duration_before_ms: Some(duration_before_ms),
                duration_after_ms: Some(duration_before_ms),
                aura_update_owner_calls_before: Some(aura_update_owner_calls_before),
                aura_update_owner_calls_after: Some(aura_update_owner_calls_before),
                script_update_would_run: false,
                remove_list: None,
            };
        }

        let aura_bound_before = dynamic_object.has_aura();

        let (expired, duration_after_ms, aura_update_owner_calls_after) = {
            let Some(record) = self.map_objects.get_mut(&dynamic_object_guid) else {
                return DynamicObjectUpdateOutcomeLikeCpp {
                    dynamic_object_guid,
                    elapsed_ms,
                    status: DynamicObjectUpdateStatusLikeCpp::MissingDynamicObject,
                    duration_before_ms: Some(duration_before_ms),
                    duration_after_ms: Some(duration_before_ms),
                    aura_update_owner_calls_before: Some(aura_update_owner_calls_before),
                    aura_update_owner_calls_after: Some(aura_update_owner_calls_before),
                    script_update_would_run: false,
                    remove_list: None,
                };
            };
            let Some(dynamic_object) = record.dynamic_object_mut() else {
                return DynamicObjectUpdateOutcomeLikeCpp {
                    dynamic_object_guid,
                    elapsed_ms,
                    status: DynamicObjectUpdateStatusLikeCpp::NotDynamicObject,
                    duration_before_ms: Some(duration_before_ms),
                    duration_after_ms: Some(duration_before_ms),
                    aura_update_owner_calls_before: Some(aura_update_owner_calls_before),
                    aura_update_owner_calls_after: Some(aura_update_owner_calls_before),
                    script_update_would_run: false,
                    remove_list: None,
                };
            };
            let expired = if aura_bound_before {
                dynamic_object.update_aura_bound_like_cpp(elapsed_ms)
            } else {
                dynamic_object.update_non_aura_duration(elapsed_ms)
            };
            (
                expired,
                dynamic_object.duration_ms(),
                dynamic_object.represented_aura_update_owner_count(),
            )
        };

        if expired {
            let remove_list = self.add_object_to_remove_list_like_cpp(dynamic_object_guid);
            DynamicObjectUpdateOutcomeLikeCpp {
                dynamic_object_guid,
                elapsed_ms,
                status: DynamicObjectUpdateStatusLikeCpp::ExpiredRemoveQueued,
                duration_before_ms: Some(duration_before_ms),
                duration_after_ms: Some(duration_after_ms),
                aura_update_owner_calls_before: Some(aura_update_owner_calls_before),
                aura_update_owner_calls_after: Some(aura_update_owner_calls_after),
                script_update_would_run: false,
                remove_list: Some(remove_list),
            }
        } else {
            DynamicObjectUpdateOutcomeLikeCpp {
                dynamic_object_guid,
                elapsed_ms,
                status: DynamicObjectUpdateStatusLikeCpp::Updated,
                duration_before_ms: Some(duration_before_ms),
                duration_after_ms: Some(duration_after_ms),
                aura_update_owner_calls_before: Some(aura_update_owner_calls_before),
                aura_update_owner_calls_after: Some(aura_update_owner_calls_after),
                script_update_would_run: true,
                remove_list: None,
            }
        }
    }

    /// Bounded map-owned live visitation seam for C++ `Map::Update` consuming
    /// `Trinity::ObjectUpdater` for `DynamicObject` records only.
    ///
    /// C++ anchors:
    /// - `Map.cpp:666-785` creates `Trinity::ObjectUpdater updater(t_diff)`
    ///   during `Map::Update` and visits object containers before
    ///   `SendObjectUpdates()` / scripts.
    /// - `GridNotifiers.cpp:258-264,296-301` visits each object and calls
    ///   `Update(i_timeDiff)` only when `IsInWorld()`, including the explicit
    ///   `DynamicObject` instantiation.
    /// - `DynamicObject.cpp:136-171` is represented by
    ///   `update_dynamic_object_like_cpp`, including duration/aura-bound evidence
    ///   and expiry enqueue through `AddObjectToRemoveList()`.
    ///
    /// Ownership: source-of-truth is canonical `Map::map_objects`. This method
    /// snapshots typed DynamicObject GUIDs only, then delegates each GUID to the
    /// existing per-object helper. It does not drain the remove-list, visit nearby
    /// cells, update players/sessions or other object families, send object
    /// updates, run scripts/AI, touch dynamic tree/collision, fan out visibility,
    /// write ObjectAccessor/session mirrors, or create fallback records.
    pub fn update_dynamic_objects_like_cpp(
        &mut self,
        elapsed_ms: u32,
    ) -> DynamicObjectsUpdateSummaryLikeCpp {
        let dynamic_object_guids = self
            .map_objects
            .iter()
            .filter_map(|(guid, record)| {
                (record.kind() == AccessorObjectKind::DynamicObject
                    && record.dynamic_object().is_some())
                .then_some(*guid)
            })
            .collect::<Vec<_>>();

        let mut summary = DynamicObjectsUpdateSummaryLikeCpp::default();
        for guid in dynamic_object_guids {
            summary.visited += 1;
            let outcome = self.update_dynamic_object_like_cpp(guid, elapsed_ms);
            match outcome.status {
                DynamicObjectUpdateStatusLikeCpp::Updated => summary.updated += 1,
                DynamicObjectUpdateStatusLikeCpp::ExpiredRemoveQueued => {
                    summary.expired_remove_queued += 1;
                }
                DynamicObjectUpdateStatusLikeCpp::MissingDynamicObject => {
                    summary.missing_or_stale += 1;
                }
                DynamicObjectUpdateStatusLikeCpp::NotDynamicObject => {
                    summary.not_dynamic_object += 1;
                }
                DynamicObjectUpdateStatusLikeCpp::NotInWorld => summary.not_in_world += 1,
            }
        }

        summary
    }

    /// Map-owned seam for C++ `Creature::Update` under `ObjectUpdater`.
    ///
    /// C++ anchors:
    /// - `Map.cpp:666-785` uses `Trinity::ObjectUpdater` during `Map::Update`.
    /// - `GridNotifiers.cpp:258-264,296-301` calls `Update(i_timeDiff)` only for
    ///   in-world objects, including the explicit `Creature` instantiation.
    /// - `Creature.cpp:696-903` is represented here only through the existing
    ///   `Creature::runtime_update_plan(diff, GameTime::GetGameTime(), context)`
    ///   helper; real AI/scripts/Unit::Update/fanout remain outside this slice.
    ///
    /// Ownership: source-of-truth is canonical `Map::map_objects`. Missing,
    /// non-Creature, and not-in-world outcomes do not mutate state. This helper
    /// never creates fallback records, reads session/ObjectAccessor mirrors,
    /// sends packets, runs DB writes, or drains map queues.
    pub fn update_creature_like_cpp(
        &mut self,
        creature_guid: ObjectGuid,
        diff_ms: u32,
        now_secs: i64,
        context: CreatureRuntimeUpdateContext,
    ) -> CreatureUpdateOutcomeLikeCpp {
        let Some(record) = self.map_object_record(creature_guid) else {
            return CreatureUpdateOutcomeLikeCpp {
                creature_guid,
                diff_ms,
                now_secs,
                status: CreatureUpdateStatusLikeCpp::MissingCreature,
                plan: None,
                actions_recorded: 0,
            };
        };

        if record.kind() != AccessorObjectKind::Creature {
            return CreatureUpdateOutcomeLikeCpp {
                creature_guid,
                diff_ms,
                now_secs,
                status: CreatureUpdateStatusLikeCpp::NotCreature,
                plan: None,
                actions_recorded: 0,
            };
        }

        let Some(creature) = record.creature() else {
            return CreatureUpdateOutcomeLikeCpp {
                creature_guid,
                diff_ms,
                now_secs,
                status: CreatureUpdateStatusLikeCpp::NotCreature,
                plan: None,
                actions_recorded: 0,
            };
        };

        if !creature.unit().world().object().is_in_world() {
            return CreatureUpdateOutcomeLikeCpp {
                creature_guid,
                diff_ms,
                now_secs,
                status: CreatureUpdateStatusLikeCpp::NotInWorld,
                plan: None,
                actions_recorded: 0,
            };
        }

        let Some(record) = self.map_objects.get_mut(&creature_guid) else {
            return CreatureUpdateOutcomeLikeCpp {
                creature_guid,
                diff_ms,
                now_secs,
                status: CreatureUpdateStatusLikeCpp::MissingCreature,
                plan: None,
                actions_recorded: 0,
            };
        };
        let Some(creature) = record.creature_mut() else {
            return CreatureUpdateOutcomeLikeCpp {
                creature_guid,
                diff_ms,
                now_secs,
                status: CreatureUpdateStatusLikeCpp::NotCreature,
                plan: None,
                actions_recorded: 0,
            };
        };

        let plan = creature.runtime_update_plan(diff_ms, now_secs, context);
        let actions_recorded = plan.actions().len();
        CreatureUpdateOutcomeLikeCpp {
            creature_guid,
            diff_ms,
            now_secs,
            status: CreatureUpdateStatusLikeCpp::Updated,
            plan: Some(plan),
            actions_recorded,
        }
    }

    /// Bounded map-owned live visitation seam for C++ `Map::Update` consuming
    /// `Trinity::ObjectUpdater` for `CreatureMapType` records only.
    ///
    /// This snapshots canonical typed Creature GUIDs from `Map::map_objects`,
    /// resolves a represented runtime context from the caller before mutable
    /// access, then delegates to `update_creature_like_cpp`. It intentionally
    /// excludes Pet and every non-Creature family unless already stored as a typed
    /// `MapObjectRecord::Creature`.
    pub fn update_creatures_like_cpp<F>(
        &mut self,
        diff_ms: u32,
        now_secs: i64,
        mut context_resolver: F,
    ) -> CreatureUpdateSummaryLikeCpp
    where
        F: FnMut(ObjectGuid, &Creature) -> CreatureRuntimeUpdateContext,
    {
        let creature_guids = self.object_updater_creature_guids_like_cpp();

        let mut summary = CreatureUpdateSummaryLikeCpp::default();
        for guid in creature_guids {
            summary.visited += 1;
            let Some(context) = self
                .map_object_record(guid)
                .and_then(MapObjectRecord::creature)
                .map(|creature| context_resolver(guid, creature))
            else {
                let outcome = self.update_creature_like_cpp(
                    guid,
                    diff_ms,
                    now_secs,
                    CreatureRuntimeUpdateContext::default(),
                );
                match outcome.status {
                    CreatureUpdateStatusLikeCpp::MissingCreature => summary.skipped_missing += 1,
                    CreatureUpdateStatusLikeCpp::NotCreature => summary.skipped_non_creature += 1,
                    CreatureUpdateStatusLikeCpp::NotInWorld => summary.skipped_not_in_world += 1,
                    CreatureUpdateStatusLikeCpp::Updated => {
                        summary.updated += 1;
                        summary.actions_recorded += outcome.actions_recorded;
                    }
                }
                continue;
            };

            let outcome = self.update_creature_like_cpp(guid, diff_ms, now_secs, context);
            match outcome.status {
                CreatureUpdateStatusLikeCpp::Updated => {
                    summary.updated += 1;
                    summary.actions_recorded += outcome.actions_recorded;
                }
                CreatureUpdateStatusLikeCpp::MissingCreature => summary.skipped_missing += 1,
                CreatureUpdateStatusLikeCpp::NotCreature => summary.skipped_non_creature += 1,
                CreatureUpdateStatusLikeCpp::NotInWorld => summary.skipped_not_in_world += 1,
            }
        }

        summary
    }

    /// Bounded map-owned consumer for C++ `GameEventMgr::UpdateEventNPCFlags` live creature loop.
    ///
    /// Mirrors `Map::GetCreatureBySpawnIdStore().equal_range(spawnId)` and applies represented
    /// `ReplaceAllNpcFlags` plus `ReplaceAllNpcFlags2` state to canonical `MapObjectRecord::Creature`.
    /// No values/session fanout, gossip reset, ObjectAccessor, update packets, or template lookup is
    /// performed inside `wow-map`.
    pub fn update_game_event_npc_flags_by_spawn_id_like_cpp(
        &mut self,
        spawn_id: SpawnId,
        npcflag_mask_with_template: u64,
    ) -> GameEventNpcFlagLiveOutcomeLikeCpp {
        let guids = self.creature_spawn_id_store_guids_like_cpp(spawn_id);
        let mut outcome = GameEventNpcFlagLiveOutcomeLikeCpp {
            spawn_id,
            indexed_guids: guids.len(),
            ..GameEventNpcFlagLiveOutcomeLikeCpp::default()
        };
        let npc_flags_low = npcflag_mask_with_template as u32;
        let npc_flags2 = (npcflag_mask_with_template >> 32) as u32;

        for guid in guids {
            let Some(record) = self.map_objects.get_mut(&guid) else {
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

            creature.ai_ownership_mut().npc_flags = npc_flags_low;
            creature.ai_ownership_mut().npc_flags2 = npc_flags2;
            creature.unit_mut().set_npc_flags_like_cpp(npc_flags_low);
            creature.unit_mut().set_npc_flags2_like_cpp(npc_flags2);
            let values_update = creature.unit().values_update();
            if values_update.has_data() {
                outcome
                    .values_updates
                    .push(GameEventNpcFlagValuesUpdateLikeCpp {
                        guid,
                        map_id: self.map_id,
                        values_update,
                    });
            }
            outcome.live_creatures_mutated += 1;
            outcome.npc_flags_low_applied += 1;
            outcome.npc_flags2_applied += 1;
        }

        outcome
    }

    /// C++ `Map::UnloadAll` clears only `_creaturesToMove` and
    /// `_gameObjectsToMove` before calling `UnloadGrid(grid, true)`
    /// (`Map.cpp:1646-1651`). It does not drain or relocate any move-list, and
    /// it does not clear AreaTrigger/DynamicObject delayed moves in that branch.
    ///
    /// Rust has no broader `UnloadAll` entry point in this seam yet; callers
    /// modeling that exact C++ pre-loop cleanup may invoke this helper before
    /// repeatedly calling `unload_grid_at(..., true)`.
    pub fn clear_unload_all_delayed_moves_like_cpp(&mut self) {
        self.creatures_to_move.clear();
        self.gameobjects_to_move.clear();
        self.creature_move_states.clear();
        self.gameobject_move_states.clear();
    }

    /// Tick C++ timed PvP combat references for every canonical combat unit
    /// and remove the reciprocal reference when one side expires.
    pub fn update_all_pvp_combat_refs_like_cpp(
        &mut self,
        diff_ms: u32,
    ) -> Vec<(ObjectGuid, ObjectGuid)> {
        let owner_guids = self.typed_combat_unit_guids_like_cpp();
        let mut expired = Vec::new();

        for owner_guid in owner_guids {
            let targets = if let Some(owner) = self.get_typed_player_mut(owner_guid) {
                owner
                    .unit_mut()
                    .subsystems_mut()
                    .combat
                    .update_pvp_combat(diff_ms)
            } else if let Some(owner) = self.get_typed_creature_mut(owner_guid) {
                owner
                    .unit_mut()
                    .subsystems_mut()
                    .combat
                    .update_pvp_combat(diff_ms)
            } else {
                Vec::new()
            };
            expired.extend(
                targets
                    .into_iter()
                    .map(|target_guid| (owner_guid, target_guid)),
            );
        }

        for (owner_guid, target_guid) in &expired {
            if let Some(target) = self.get_typed_player_mut(*target_guid) {
                target
                    .unit_mut()
                    .subsystems_mut()
                    .combat
                    .purge_combat_ref_like_cpp(*owner_guid);
            } else if let Some(target) = self.get_typed_creature_mut(*target_guid) {
                target
                    .unit_mut()
                    .subsystems_mut()
                    .combat
                    .purge_combat_ref_like_cpp(*owner_guid);
            }
        }

        expired
    }

    pub fn ensure_grid_loaded_for_player_phase<Filter>(
        &mut self,
        cell: &Cell,
        phase_shift: &PhaseShift,
        loader: &mut ObjectGridLoader<'_, Filter>,
    ) -> bool
    where
        Filter: GridSpawnLoadFilter,
    {
        let loaded_now = self.ensure_grid_loaded(cell);
        let coord = GridCoord::new(cell.grid_x(), cell.grid_y());
        self.mark_active_cell(cell.cell_coord());

        let active_expiry_ms = (self.grid_expiry_ms as f32 * 0.1) as i64;
        let index = checked_grid_index(coord);
        let grid = self.grids[index].as_mut().expect("grid was just loaded");
        self.personal_phase_tracker
            .load_grid(phase_shift, grid, loader);

        if grid.state() != GridStateKind::Active {
            grid.info_mut().reset_time_tracker(active_expiry_ms);
            grid.set_state(GridStateKind::Active);
        }

        loaded_now
    }

    pub fn update_loaded_grid_states_like_cpp(
        &mut self,
        diff_ms: u32,
    ) -> GridStatesUpdateSummaryLikeCpp {
        // C++ `Map::DelayedUpdate` increments the GridRefManager iterator before
        // invoking the grid-state update because that update may unload/delete
        // the current grid (`Map.cpp:2536-2542`). Rust snapshots loaded grid
        // coordinates first and then re-checks each slot, never recreating a
        // grid that disappeared earlier in the same delayed-update pass.
        let loaded_grid_coords: Vec<GridCoord> = self
            .grids
            .iter()
            .enumerate()
            .filter_map(|(index, grid)| {
                grid.as_ref().map(|_| {
                    GridCoord::new(
                        (index as u32) / MAX_NUMBER_OF_GRIDS,
                        (index as u32) % MAX_NUMBER_OF_GRIDS,
                    )
                })
            })
            .collect();

        let mut summary = GridStatesUpdateSummaryLikeCpp {
            diff_ms,
            visited: loaded_grid_coords.len(),
            ..GridStatesUpdateSummaryLikeCpp::default()
        };

        for coord in loaded_grid_coords {
            let Some(previous_state) = self.get_ngrid(coord).map(NGrid::state) else {
                summary.missing_after_snapshot += 1;
                continue;
            };

            if matches!(previous_state, GridStateKind::Invalid) {
                summary.skipped_invalid += 1;
            }

            let unloaded = self.update_grid_state_at(coord, diff_ms);
            summary.updated += 1;

            if unloaded {
                summary.unloaded += 1;
                if matches!(previous_state, GridStateKind::Removal) {
                    summary.removal_unloaded += 1;
                }
                continue;
            }

            let Some(next_state) = self.get_ngrid(coord).map(NGrid::state) else {
                summary.missing_after_snapshot += 1;
                continue;
            };

            match (previous_state, next_state) {
                (GridStateKind::Active, GridStateKind::Idle) => summary.active_to_idle += 1,
                (GridStateKind::Idle, GridStateKind::Removal) => summary.idle_to_removal += 1,
                (GridStateKind::Removal, GridStateKind::Removal) => {
                    summary.removal_deferred_or_reset += 1;
                }
                _ => {}
            }
        }

        summary
    }

    pub fn update_grid_state_at(&mut self, coord: GridCoord, diff_ms: u32) -> bool {
        let index = checked_grid_index(coord);
        let Some(mut grid) = self.grids[index].take() else {
            return false;
        };

        self.grid_state_unloaded = false;
        update_grid_state(self, &mut grid, diff_ms);
        if self.grid_state_unloaded {
            self.grid_state_unloaded = false;
            true
        } else {
            self.grids[index] = Some(grid);
            false
        }
    }
}
