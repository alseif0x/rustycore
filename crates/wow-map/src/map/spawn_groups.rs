// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Spawn groups and spawned pools.
mod pool_actions;

use super::loaded_grid_admission::{
    LoadedGridConditionOutcome, LoadedGridReceipts, LoadedGridSpawnOutcome,
};
use super::*;

mod materialized;
use materialized::ConditionResults;

impl<Terrain, Lifecycle> Map<Terrain, Lifecycle>
where
    Terrain: TerrainGridLoader,
    Lifecycle: GridLifecycle,
{
    /// Compatibility wrapper for the original inactive-spawn-group delete-only seam.
    pub fn process_due_respawns_spawn_group_delete_only_like_cpp(
        &mut self,
        now: i64,
        spawn_store: &SpawnStore,
    ) -> ProcessRespawnsDeleteOnlySummaryLikeCpp {
        let linked_store = LinkedRespawnStoreLikeCpp::new();
        self.process_due_respawns_composite_safe_side_effects_like_cpp(
            now,
            spawn_store,
            &linked_store,
            &PoolMgrLikeCpp::new(),
            5,
            false,
            |_, _| false,
            |_, _| 0.0,
            |_candidates, count| (0..count).collect(),
        )
    }

    /// First represented guard from C++ `Map::CheckRespawn`.
    ///
    /// C++ anchors:
    /// - `Map.cpp:1956-1957` resolves `SpawnData` and asserts when missing.
    /// - `Map.cpp:1959-1964` clears `respawnTime` and returns false when the
    ///   spawn group is inactive.
    ///
    /// This is only the spawn-group subdependency of `CheckRespawn`. It does not
    /// implement live by-spawn existence, escort dynamic rules, gameobject live
    /// checks, linked respawn, random 5-15 reschedule, PoolMgr, `DoRespawn`, DB
    /// save/delete, or world-server tick integration. Missing `SpawnData` is a
    /// temporary defensive fallback for incomplete ownership: C++ would assert;
    /// RustyCore returns `MissingSpawnData`, does not mutate `respawn_time`, and
    /// leaves timer deletion/reschedule decisions to the caller.
    pub fn check_respawn_spawn_group_guard_like_cpp(
        &self,
        info: &mut RespawnInfoLikeCpp,
        spawn_store: &SpawnStore,
    ) -> CheckRespawnSpawnGroupGuardOutcomeLikeCpp {
        let Some(spawn_data) = spawn_store.spawn_data(info.object_type, info.spawn_id) else {
            return CheckRespawnSpawnGroupGuardOutcomeLikeCpp::MissingSpawnData;
        };

        if !self.is_spawn_group_active_like_cpp(Some(&spawn_data.spawn_group)) {
            info.respawn_time = 0;
            return CheckRespawnSpawnGroupGuardOutcomeLikeCpp::InactiveSpawnGroupDeletedTimer;
        }

        CheckRespawnSpawnGroupGuardOutcomeLikeCpp::Allowed
    }

    /// Map-owned bridge for C++ `Map::_toggledSpawnGroupIds`.
    ///
    /// C++ anchors:
    /// - `Map.h:780-781` stores toggled spawn group ids on `Map`.
    /// - `Map.cpp:2427-2439` toggles only non-system existing groups.
    /// - `Map.cpp:2441-2453` queries missing/system/default/manual semantics.
    ///
    /// RustyCore does not yet wire ObjectMgr/SpawnStore ownership into `Map`, so
    /// callers must pass the already-resolved template as an honest bridge.
    pub const fn spawn_group_state(&self) -> &SpawnGroupRuntimeState {
        &self.spawn_group_state
    }

    pub fn set_spawn_group_active_like_cpp(
        &mut self,
        group: Option<&SpawnGroupTemplateData>,
        state: bool,
    ) -> SpawnGroupActiveChange {
        self.spawn_group_state
            .set_spawn_group_active_like_cpp(group, state)
    }

    pub fn set_spawn_group_inactive_like_cpp(
        &mut self,
        group: Option<&SpawnGroupTemplateData>,
    ) -> SpawnGroupActiveChange {
        self.set_spawn_group_active_like_cpp(group, false)
    }

    pub fn is_spawn_group_active_like_cpp(&self, group: Option<&SpawnGroupTemplateData>) -> bool {
        self.spawn_group_state.is_spawn_group_active_like_cpp(group)
    }

    /// Pure bridge for C++ `Map::InitSpawnGroupState` over pre-resolved group
    /// templates. It intentionally applies only active-state toggles; live
    /// spawn/despawn, pool runtime, respawn persistence, and fanout are later gaps.
    pub fn init_spawn_group_state_like_cpp<'a, I, F>(
        &mut self,
        groups: I,
        mut meets_conditions: F,
    ) -> Vec<(u32, SpawnGroupActiveChange)>
    where
        I: IntoIterator<Item = &'a SpawnGroupTemplateData>,
        F: FnMut(&SpawnGroupTemplateData) -> bool,
    {
        let mut changes = Vec::new();
        for group in groups {
            if group.is_system() {
                continue;
            }
            let active = meets_conditions(group);
            changes.push((
                group.group_id,
                self.set_spawn_group_active_like_cpp(Some(group), active),
            ));
        }
        changes
    }

    /// Pure action planner for C++ `Map::UpdateSpawnGroupConditions` over
    /// pre-resolved spawn-group templates.
    ///
    /// C++ anchors:
    /// - `Map.cpp:2471-2502` loops map groups, compares
    ///   `IsSpawnGroupActive` with `ConditionMgr`, and runs spawn/despawn or
    ///   inactive branches.
    /// - `Map.cpp:2427-2453` owns `_toggledSpawnGroupIds` semantics through
    ///   `SetSpawnGroupActive` / `IsSpawnGroupActive`.
    /// - `SpawnData.h:51-63` defines manual and condition-failure flags.
    ///
    /// This does not run live `SpawnGroupSpawn`/`SpawnGroupDespawn`, touch DB,
    /// mutate toggles, simulate pools, persist respawns, create entities, or
    /// fan out updates. The closure only replaces C++
    /// `ConditionMgr::IsMapMeetingNotGroupedConditions` for already-resolved
    /// condition outcomes.
    pub fn plan_update_spawn_group_conditions_like_cpp<'a, I, F>(
        &self,
        groups: I,
        mut meets_conditions: F,
    ) -> Vec<(u32, SpawnGroupConditionActionLikeCpp)>
    where
        I: IntoIterator<Item = &'a SpawnGroupTemplateData>,
        F: FnMut(&SpawnGroupTemplateData) -> bool,
    {
        let mut actions = Vec::new();
        for group in groups {
            let is_active = self.is_spawn_group_active_like_cpp(Some(group));
            let should_be_active = meets_conditions(group);

            if group.flags.contains(SpawnGroupFlags::MANUAL_SPAWN) {
                if is_active
                    && !should_be_active
                    && group
                        .flags
                        .contains(SpawnGroupFlags::DESPAWN_ON_CONDITION_FAILURE)
                {
                    actions.push((
                        group.group_id,
                        SpawnGroupConditionActionLikeCpp::condition_failure_despawn(),
                    ));
                } else {
                    actions.push((group.group_id, SpawnGroupConditionActionLikeCpp::Noop));
                }
                continue;
            }

            if is_active == should_be_active {
                actions.push((group.group_id, SpawnGroupConditionActionLikeCpp::Noop));
                continue;
            }

            let action = if should_be_active {
                SpawnGroupConditionActionLikeCpp::spawn_group_spawn_default()
            } else if group
                .flags
                .contains(SpawnGroupFlags::DESPAWN_ON_CONDITION_FAILURE)
            {
                SpawnGroupConditionActionLikeCpp::condition_failure_despawn()
            } else {
                SpawnGroupConditionActionLikeCpp::SetInactive
            };
            actions.push((group.group_id, action));
        }
        actions
    }

    /// C++ `Map::SpawnGroupDespawn(groupId, deleteRespawnTimes)` represented over
    /// map-owned runtime state and caller-supplied ObjectMgr-like `SpawnStore`.
    ///
    /// C++ anchors:
    /// - `Map.cpp:2404-2425` validates existing/non-system group, iterates
    ///   `sObjectMgr->GetSpawnMetadataForGroup`, optionally calls
    ///   `RemoveRespawnTime`, calls `DespawnAll`, then marks the group inactive.
    /// - `Map.cpp:2140-2163` DB delete is owned by callers; this helper only
    ///   mutates map-owned respawn timers so world-server can derive before/after
    ///   `CHAR_DEL_RESPAWN` work outside the lock.
    pub fn spawn_group_despawn_like_cpp(
        &mut self,
        group: Option<&SpawnGroupTemplateData>,
        delete_respawn_times: bool,
        spawn_store: &SpawnStore,
    ) -> SpawnGroupDespawnOutcomeLikeCpp {
        let Some(group) = group else {
            return SpawnGroupDespawnOutcomeLikeCpp::blocked_missing_group(0);
        };
        if group.is_system() {
            return SpawnGroupDespawnOutcomeLikeCpp::blocked_system_group(group.group_id);
        }

        let mut outcome = SpawnGroupDespawnOutcomeLikeCpp::executed(group.group_id);
        if let Some(members) = spawn_store.spawn_group_members(group.group_id) {
            let members = members.iter().copied().collect::<Vec<_>>();
            for member in members {
                let Some(spawn_data) = spawn_store.spawn_data(member.object_type, member.spawn_id)
                else {
                    outcome.metadata_entries += 1;
                    outcome.stale_index_entries += 1;
                    continue;
                };
                if spawn_data.map_id != self.map_id {
                    continue;
                }

                outcome.metadata_entries += 1;
                if delete_respawn_times {
                    match member.object_type {
                        SpawnObjectType::Creature | SpawnObjectType::GameObject => {
                            if self
                                .remove_respawn_time_like_cpp(member.object_type, member.spawn_id)
                                .is_some()
                            {
                                outcome.respawn_timers_removed += 1;
                            } else {
                                outcome.respawn_timers_missing += 1;
                            }
                        }
                        SpawnObjectType::AreaTrigger => {
                            outcome.respawn_timer_unsupported_types += 1;
                        }
                    }
                }

                let despawn =
                    self.despawn_all_by_spawn_id_like_cpp(member.object_type, member.spawn_id);
                outcome.objects_removed += despawn.queued;
                outcome.stale_index_entries += despawn.stale_index_entries;
                outcome.remove_errors += despawn.remove_errors;
                outcome.unsupported_live_despawn_types += despawn.unsupported_live_despawn_type;
            }
        }
        outcome.applied_inactive_change =
            Some(self.set_spawn_group_active_like_cpp(Some(group), false));
        outcome
    }

    /// C++ `Map::SpawnGroupSpawn(groupId, ignoreRespawn, force)` represented as a
    /// safe map-local planning/execution seam over map-owned active state,
    /// respawn timers, by-spawn live-object indexes, and optional caller-supplied
    /// loaded-grid DB-backed records.
    ///
    /// C++ anchors:
    /// - `Map.cpp:2315-2324` validates existing/non-system group and marks it
    ///   active before iterating metadata.
    /// - `Map.cpp:2326-2353` iterates ObjectMgr spawn metadata, removes respawn
    ///   timers when forced/ignoring, skips active timers and live objects.
    /// - `Map.cpp:2326-2334` skips types whose `GetRespawnMapForType` is null;
    ///   `Map.h:751-763,765-777` currently returns null for AreaTrigger, so that
    ///   type is continued before timers, TypeHasData, difficulty, grid, or loader
    ///   planning.
    /// - `Map.cpp:2356-2385` checks difficulty/grid-loaded before calling
    ///   Creature/GameObject `LoadFromDB` and retaining the loaded object.
    /// - `Map.cpp:2387-2395` contains an AreaTrigger switch branch, but it is
    ///   unreachable with the current respawn-map guard. This does not implement
    ///   `AreaTrigger::LoadFromDB` or live AreaTrigger runtime.
    ///
    /// Ownership: `Map` owns active spawn-group state, respawn timers, live indexes,
    /// and `AddToMap`. The caller owns DB/template/runtime selection and may provide
    /// typed `LoadedGridRespawnRecordsLikeCpp` records. Synchronization is strictly
    /// caller loader -> map-owned `AddToMap`; this method never fabricates fallback
    /// records and never reaches into DB/world-server/session state.
    pub fn spawn_group_spawn_loaded_grid_records_like_cpp<L>(
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
        self.spawn_group_materialized_core(
            group,
            ignore_respawn,
            force,
            spawn_store,
            |map, kind, spawn_id, force| {
                Ok(load_record(map, kind, spawn_id, force).map(LoadedGridMaterialization::records))
            },
            LoadedGridReceipts::RecordCompatibility,
        )
        .summary
    }

    /// Compatibility wrapper preserving the pre-loader `SpawnGroupSpawn` seam:
    /// loaded-grid Creature/GameObject attempts are planned and counted as blocked,
    /// but no DB-backed records are fabricated or inserted.
    pub fn spawn_group_spawn_like_cpp(
        &mut self,
        group: Option<&SpawnGroupTemplateData>,
        ignore_respawn: bool,
        force: bool,
        spawn_store: &SpawnStore,
    ) -> SpawnGroupSpawnOutcomeLikeCpp {
        self.spawn_group_spawn_loaded_grid_records_like_cpp(
            group,
            ignore_respawn,
            force,
            spawn_store,
            |_map, _object_type, _spawn_id, _force| None,
        )
    }

    /// C++-shaped `Map::UpdateSpawnGroupConditions` bridge over pre-resolved
    /// templates that executes the complete represented `SetSpawnGroupInactive`
    /// branch, the map-local `SpawnGroupDespawn(..., true)` condition-failure
    /// branch, and the safe map-local `SpawnGroupSpawn` loaded-grid branch with
    /// caller-supplied records.
    ///
    /// Ownership remains split like `spawn_group_spawn_loaded_grid_records_like_cpp`:
    /// this map owns active-state/timer/live/grid/difficulty/AddToMap decisions;
    /// the caller owns DB/template/runtime composition and may return no record to
    /// preserve the pre-loader planned/blocked outcome.
    pub fn apply_update_spawn_group_conditions_loaded_grid_records_like_cpp<'a, I, F, L>(
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
        match self.update_spawn_group_conditions_core(
            groups,
            spawn_store,
            meets_conditions,
            |map, kind, spawn_id, force| {
                Ok(load_record(map, kind, spawn_id, force).map(LoadedGridMaterialization::records))
            },
            LoadedGridReceipts::RecordCompatibility,
        ) {
            ConditionResults::Record(outcomes) => outcomes,
            ConditionResults::Owned(_) => unreachable!("Record wrapper selects Record receipts"),
        }
    }

    /// Compatibility wrapper preserving the pre-loader `UpdateSpawnGroupConditions`
    /// seam: loaded-grid Creature/GameObject spawn attempts are planned and counted
    /// as blocked, but no DB-backed records are fabricated or inserted.
    pub fn apply_update_spawn_group_conditions_represented_like_cpp<'a, I, F>(
        &mut self,
        groups: I,
        spawn_store: &SpawnStore,
        meets_conditions: F,
    ) -> Vec<SpawnGroupConditionUpdateOutcomeLikeCpp>
    where
        I: IntoIterator<Item = &'a SpawnGroupTemplateData>,
        F: FnMut(&SpawnGroupTemplateData) -> bool,
    {
        self.apply_update_spawn_group_conditions_loaded_grid_records_like_cpp(
            groups,
            spawn_store,
            meets_conditions,
            |_map, _object_type, _spawn_id, _force| None,
        )
    }

    /// Legacy wrapper preserving the pre-#391 SetInactive-only seam for focused
    /// tests/callers that explicitly require planned-only despawn evidence.
    pub fn apply_update_spawn_group_conditions_set_inactive_like_cpp<'a, I, F>(
        &mut self,
        groups: I,
        meets_conditions: F,
    ) -> Vec<SpawnGroupConditionUpdateOutcomeLikeCpp>
    where
        I: IntoIterator<Item = &'a SpawnGroupTemplateData>,
        F: FnMut(&SpawnGroupTemplateData) -> bool,
    {
        let groups = groups.into_iter().collect::<Vec<_>>();
        let planned_actions = self
            .plan_update_spawn_group_conditions_like_cpp(groups.iter().copied(), meets_conditions);

        planned_actions
            .into_iter()
            .zip(groups)
            .map(|((group_id, action), group)| {
                let applied_change = if action == SpawnGroupConditionActionLikeCpp::SetInactive {
                    Some(self.set_spawn_group_inactive_like_cpp(Some(group)))
                } else {
                    None
                };

                SpawnGroupConditionUpdateOutcomeLikeCpp {
                    group_id,
                    action,
                    applied_change,
                    despawn_outcome: None,
                    spawn_outcome: None,
                }
            })
            .collect()
    }
}
