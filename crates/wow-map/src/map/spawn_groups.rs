// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Spawn groups and spawned pools.

use super::*;

impl<Terrain, Lifecycle> Map<Terrain, Lifecycle>
where
    Terrain: TerrainGridLoader,
    Lifecycle: GridLifecycle,
{
    /// Executes the safe map-local half of actions returned by represented C++
    /// `PoolMgr::UpdatePool` planning.
    ///
    /// C++ anchors:
    /// - `PoolMgr.cpp:183-257` `DespawnObject` / `Despawn1Object` removes
    ///   current map objects and optionally removes respawn timers.
    /// - `PoolMgr.cpp:353-403` `Spawn1Object` / `ReSpawn1Object` create only
    ///   on loaded grids; RustyCore reports that missing runtime instead of
    ///   creating DB-backed entities in `wow-map`.
    pub(super) fn apply_pool_typed_spawn_plan_safe_map_actions_like_cpp(
        &mut self,
        plan: &PoolTypedSpawnPlanLikeCpp,
        spawn_store: &SpawnStore,
        summary: &mut ProcessRespawnsSafeSideEffectsSummaryLikeCpp,
    ) {
        self.apply_pool_typed_spawn_plan_loaded_grid_records_like_cpp::<
            fn(&mut Self, SpawnObjectType, SpawnId) -> Option<LoadedGridRespawnRecordsLikeCpp>,
        >(plan, spawn_store, summary, None);
    }

    pub(super) fn apply_pool_typed_spawn_plan_loaded_grid_records_like_cpp<L>(
        &mut self,
        plan: &PoolTypedSpawnPlanLikeCpp,
        spawn_store: &SpawnStore,
        summary: &mut ProcessRespawnsSafeSideEffectsSummaryLikeCpp,
        mut load_record: Option<&mut L>,
    ) where
        L: FnMut(&mut Self, SpawnObjectType, SpawnId) -> Option<LoadedGridRespawnRecordsLikeCpp>,
    {
        if let Some(object_plan) = plan.object_plan.as_ref() {
            self.apply_pool_spawn_object_plan_loaded_grid_records_like_cpp(
                object_plan,
                spawn_store,
                summary,
                load_record.as_deref_mut(),
            );
        }
    }

    fn apply_pool_spawn_pool_plan_loaded_grid_records_like_cpp<L>(
        &mut self,
        plan: &PoolSpawnPoolPlanLikeCpp,
        spawn_store: &SpawnStore,
        summary: &mut ProcessRespawnsSafeSideEffectsSummaryLikeCpp,
        mut load_record: Option<&mut L>,
    ) where
        L: FnMut(&mut Self, SpawnObjectType, SpawnId) -> Option<LoadedGridRespawnRecordsLikeCpp>,
    {
        for subplan in &plan.subplans {
            self.apply_pool_typed_spawn_plan_loaded_grid_records_like_cpp(
                subplan,
                spawn_store,
                summary,
                load_record.as_deref_mut(),
            );
        }
    }

    fn apply_pool_despawn_pool_plan_safe_map_actions_like_cpp(
        &mut self,
        plan: &PoolDespawnPoolPlanLikeCpp,
        summary: &mut ProcessRespawnsSafeSideEffectsSummaryLikeCpp,
    ) {
        for subplan in &plan.subplans {
            self.apply_pool_typed_despawn_plan_safe_map_actions_like_cpp(subplan, summary);
        }
    }

    fn apply_pool_typed_despawn_plan_safe_map_actions_like_cpp(
        &mut self,
        plan: &PoolTypedDespawnPlanLikeCpp,
        summary: &mut ProcessRespawnsSafeSideEffectsSummaryLikeCpp,
    ) {
        if let Some(object_plan) = plan.object_plan.as_ref() {
            self.apply_pool_despawn_object_plan_safe_map_actions_like_cpp(object_plan, summary);
        }
    }

    fn apply_pool_despawn_object_plan_safe_map_actions_like_cpp(
        &mut self,
        plan: &PoolDespawnObjectPlanLikeCpp,
        summary: &mut ProcessRespawnsSafeSideEffectsSummaryLikeCpp,
    ) {
        let mut child_pool_plans = plan.child_pool_plans.iter();
        for action in &plan.actions {
            match *action {
                PoolSpawnObjectActionLikeCpp::DespawnOne {
                    kind: PoolMemberKindLikeCpp::Pool,
                    ..
                } => {
                    if let Some(child_plan) = child_pool_plans.next() {
                        self.apply_pool_despawn_pool_plan_safe_map_actions_like_cpp(
                            child_plan, summary,
                        );
                    } else {
                        summary.pool_unsupported_action_kind += 1;
                    }
                }
                other => match other {
                    PoolSpawnObjectActionLikeCpp::DespawnOne { kind, guid } => {
                        self.apply_pool_despawn_one_safe_map_action_like_cpp(kind, guid, summary);
                    }
                    PoolSpawnObjectActionLikeCpp::RemoveRespawnTime { kind, guid } => {
                        let Some(object_type) =
                            pool_member_kind_to_spawn_object_type_like_cpp(kind)
                        else {
                            return;
                        };
                        if self
                            .remove_respawn_time_like_cpp(object_type, guid as SpawnId)
                            .is_some()
                        {
                            summary.pool_respawn_timers_removed += 1;
                        } else {
                            summary.pool_respawn_timers_missing += 1;
                        }
                    }
                    PoolSpawnObjectActionLikeCpp::SpawnOne { .. }
                    | PoolSpawnObjectActionLikeCpp::RespawnOne { .. } => {}
                },
            }
        }
    }

    fn apply_pool_spawn_object_plan_loaded_grid_records_like_cpp<L>(
        &mut self,
        plan: &PoolSpawnObjectPlanLikeCpp,
        spawn_store: &SpawnStore,
        summary: &mut ProcessRespawnsSafeSideEffectsSummaryLikeCpp,
        mut load_record: Option<&mut L>,
    ) where
        L: FnMut(&mut Self, SpawnObjectType, SpawnId) -> Option<LoadedGridRespawnRecordsLikeCpp>,
    {
        let mut child_spawn_plans = plan.child_pool_spawn_plans.iter();
        let mut child_despawn_plans = plan.child_pool_despawn_plans.iter();
        for action in &plan.actions {
            match *action {
                PoolSpawnObjectActionLikeCpp::SpawnOne {
                    kind: PoolMemberKindLikeCpp::Pool,
                    ..
                } => {
                    if let Some(child_plan) = child_spawn_plans.next() {
                        self.apply_pool_spawn_pool_plan_loaded_grid_records_like_cpp(
                            child_plan,
                            spawn_store,
                            summary,
                            load_record.as_deref_mut(),
                        );
                    } else {
                        summary.pool_unsupported_action_kind += 1;
                    }
                }
                PoolSpawnObjectActionLikeCpp::DespawnOne {
                    kind: PoolMemberKindLikeCpp::Pool,
                    ..
                } => {
                    if let Some(child_plan) = child_despawn_plans.next() {
                        self.apply_pool_despawn_pool_plan_safe_map_actions_like_cpp(
                            child_plan, summary,
                        );
                    } else {
                        summary.pool_unsupported_action_kind += 1;
                    }
                }
                PoolSpawnObjectActionLikeCpp::RespawnOne {
                    kind: PoolMemberKindLikeCpp::Pool,
                    ..
                }
                | PoolSpawnObjectActionLikeCpp::RemoveRespawnTime {
                    kind: PoolMemberKindLikeCpp::Pool,
                    ..
                } => {}
                other => self.apply_pool_spawn_object_action_loaded_grid_records_like_cpp(
                    other,
                    spawn_store,
                    summary,
                    load_record.as_deref_mut(),
                ),
            }
        }
    }

    fn apply_pool_spawn_object_action_loaded_grid_records_like_cpp<L>(
        &mut self,
        action: PoolSpawnObjectActionLikeCpp,
        spawn_store: &SpawnStore,
        summary: &mut ProcessRespawnsSafeSideEffectsSummaryLikeCpp,
        load_record: Option<&mut L>,
    ) where
        L: FnMut(&mut Self, SpawnObjectType, SpawnId) -> Option<LoadedGridRespawnRecordsLikeCpp>,
    {
        match action {
            PoolSpawnObjectActionLikeCpp::DespawnOne { kind, guid } => {
                self.apply_pool_despawn_one_safe_map_action_like_cpp(kind, guid, summary);
            }
            PoolSpawnObjectActionLikeCpp::RespawnOne { kind, guid } => {
                self.apply_pool_despawn_one_safe_map_action_like_cpp(kind, guid, summary);
                self.report_pool_spawn_one_action_like_cpp(
                    kind,
                    guid,
                    true,
                    spawn_store,
                    summary,
                    load_record,
                );
            }
            PoolSpawnObjectActionLikeCpp::RemoveRespawnTime { kind, guid } => {
                let Some(object_type) = pool_member_kind_to_spawn_object_type_like_cpp(kind) else {
                    return;
                };
                if self
                    .remove_respawn_time_like_cpp(object_type, guid as SpawnId)
                    .is_some()
                {
                    summary.pool_respawn_timers_removed += 1;
                } else {
                    summary.pool_respawn_timers_missing += 1;
                }
            }
            PoolSpawnObjectActionLikeCpp::SpawnOne { kind, guid } => {
                self.report_pool_spawn_one_action_like_cpp(
                    kind,
                    guid,
                    false,
                    spawn_store,
                    summary,
                    load_record,
                );
            }
        }
    }

    fn apply_pool_despawn_one_safe_map_action_like_cpp(
        &mut self,
        kind: PoolMemberKindLikeCpp,
        spawn_id: u64,
        summary: &mut ProcessRespawnsSafeSideEffectsSummaryLikeCpp,
    ) {
        let spawn_id = spawn_id as SpawnId;
        let guids = match kind {
            PoolMemberKindLikeCpp::Creature => {
                self.creature_spawn_id_store_guids_like_cpp(spawn_id)
            }
            PoolMemberKindLikeCpp::GameObject => {
                self.gameobject_spawn_id_store_guids_like_cpp(spawn_id)
            }
            PoolMemberKindLikeCpp::Pool => {
                summary.pool_unsupported_action_kind += 1;
                return;
            }
        };

        for guid in guids {
            if self.map_object_record(guid).is_none() {
                summary.pool_stale_index_entries += 1;
                continue;
            }
            match self.remove_from_map_like_cpp(guid, true) {
                Ok(_removed) => {
                    summary.pool_objects_removed += 1;
                }
                Err(RemoveFromMapError::ObjectNotFound { .. }) => {
                    summary.pool_stale_index_entries += 1;
                }
                Err(_error) => {
                    summary.pool_remove_errors += 1;
                }
            }
        }
    }

    fn report_pool_spawn_one_action_like_cpp<L>(
        &mut self,
        kind: PoolMemberKindLikeCpp,
        spawn_id: u64,
        respawn: bool,
        spawn_store: &SpawnStore,
        summary: &mut ProcessRespawnsSafeSideEffectsSummaryLikeCpp,
        load_record: Option<&mut L>,
    ) where
        L: FnMut(&mut Self, SpawnObjectType, SpawnId) -> Option<LoadedGridRespawnRecordsLikeCpp>,
    {
        let Some(object_type) = pool_member_kind_to_spawn_object_type_like_cpp(kind) else {
            summary.pool_unsupported_action_kind += 1;
            return;
        };
        let spawn_id = spawn_id as SpawnId;
        let Some(spawn_data) = spawn_store.spawn_data(object_type, spawn_id) else {
            summary.pool_spawn_actions_missing_spawn_data += 1;
            return;
        };
        let cell = cell_from_world(spawn_data.spawn_point.x, spawn_data.spawn_point.y);
        let grid = GridCoord::new(cell.grid_x(), cell.grid_y());
        if !self.is_grid_loaded(grid) {
            summary.pool_spawn_actions_skipped_unloaded_grid += 1;
            return;
        }

        let Some(load_record) = load_record else {
            summary.pool_spawn_actions_blocked_loaded_grid += 1;
            summary
                .pool_spawn_action_load_plans
                .push(PoolSpawnActionLoadPlanLikeCpp {
                    object_type,
                    spawn_id,
                    respawn,
                });
            return;
        };

        let Some(records) = load_record(self, object_type, spawn_id) else {
            summary.pool_spawn_actions_blocked_loaded_grid += 1;
            summary
                .pool_spawn_action_load_plans
                .push(PoolSpawnActionLoadPlanLikeCpp {
                    object_type,
                    spawn_id,
                    respawn,
                });
            return;
        };

        for pre_add_record in records.pre_add_records {
            let _ = self.add_map_object_record_to_map_like_cpp(pre_add_record);
        }
        let primary_record = records.primary_record;
        let loaded_grid_primary_record = primary_record.clone();
        match self.add_map_object_record_to_map_like_cpp(primary_record) {
            Ok(_outcome) => {
                summary.executed_loaded_grid_respawns += 1;
                summary
                    .loaded_grid_primary_records
                    .push(loaded_grid_primary_record);
            }
            Err(_error) => {
                summary.blocked_loaded_grid_respawn_add_to_map += 1;
            }
        }
    }

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

    pub const fn pool_data_like_cpp(&self) -> &SpawnedPoolDataLikeCpp {
        &self.pool_data
    }

    pub const fn pool_data_mut_like_cpp(&mut self) -> &mut SpawnedPoolDataLikeCpp {
        &mut self.pool_data
    }

    /// Map-owned facade for a direct C++ `PoolMgr::DespawnPool(spawns, pool_id,
    /// alwaysDeleteRespawnTime)` call.
    ///
    /// Ownership stays one-way: `PoolMgrLikeCpp` plans and mutates only this
    /// map's canonical `SpawnedPoolDataLikeCpp`; `Map` then applies only safe
    /// map-local Creature/GameObject removal and respawn-timer deletion actions
    /// already represented by the plan. It does not fabricate live records,
    /// persist DB state, or fan out packets/scripts/AI.
    pub fn despawn_pool_safe_map_actions_like_cpp(
        &mut self,
        pool_mgr: &PoolMgrLikeCpp,
        pool_id: u32,
        always_delete_respawn_time: bool,
    ) -> Result<ProcessRespawnsSafeSideEffectsSummaryLikeCpp, PoolMgrPlanErrorLikeCpp> {
        let plan = pool_mgr.despawn_pool_plan_like_cpp(
            &mut self.pool_data,
            pool_id,
            always_delete_respawn_time,
        )?;
        let mut summary = ProcessRespawnsSafeSideEffectsSummaryLikeCpp::default();
        self.apply_pool_despawn_pool_plan_safe_map_actions_like_cpp(&plan, &mut summary);
        Ok(summary)
    }

    /// Map-owned facade for a direct C++ `PoolMgr::SpawnPool(spawns, pool_id)`
    /// call over an already loaded canonical map.
    ///
    /// Ownership stays one-way: caller-owned canonical metadata and
    /// `PoolMgrLikeCpp` feed a deterministic `SpawnPool` plan that mutates this
    /// map's canonical `SpawnedPoolDataLikeCpp`; `Map` then consumes only
    /// loaded-grid `Spawn1Object`/recursive child-pool actions through the
    /// caller-supplied typed record loader. `wow-map` does not read DB, create
    /// dummy records, persist state, touch sessions/ObjectAccessor, or fan out.
    pub fn spawn_pool_loaded_grid_records_like_cpp<L>(
        &mut self,
        pool_mgr: &PoolMgrLikeCpp,
        pool_id: u32,
        spawn_store: &SpawnStore,
        explicit_roll_for: impl FnMut(PoolMemberKindLikeCpp, u32) -> f32,
        choose_equal: impl FnMut(&[PoolObjectLikeCpp], usize) -> Vec<usize>,
        mut load_record: L,
    ) -> Result<ProcessRespawnsSafeSideEffectsSummaryLikeCpp, PoolMgrPlanErrorLikeCpp>
    where
        L: FnMut(&mut Self, SpawnObjectType, SpawnId) -> Option<LoadedGridRespawnRecordsLikeCpp>,
    {
        let plan = pool_mgr.spawn_pool_plan_like_cpp(
            &mut self.pool_data,
            pool_id,
            explicit_roll_for,
            choose_equal,
        )?;
        let mut summary = ProcessRespawnsSafeSideEffectsSummaryLikeCpp::default();
        self.apply_pool_spawn_pool_plan_loaded_grid_records_like_cpp(
            &plan,
            spawn_store,
            &mut summary,
            Some(&mut load_record),
        );
        Ok(summary)
    }

    /// C++ `Map` constructor calls `sPoolMgr->InitPoolsForMap(this)` before
    /// startup respawn and spawn-group initialization. This represented seam
    /// applies deterministic autospawn `SpawnPool` plans into the map-owned
    /// `SpawnedPoolDataLikeCpp` and returns action records for future live
    /// `Spawn1Object`/`ReSpawn1Object`/`DespawnObject` owners; it does not create
    /// entities or fan out packets.
    pub fn init_pools_for_map_like_cpp(
        &mut self,
        pool_mgr: &PoolMgrLikeCpp,
        explicit_roll_for: impl FnMut(PoolMemberKindLikeCpp, u32) -> f32,
        choose_equal: impl FnMut(&[PoolObjectLikeCpp], usize) -> Vec<usize>,
    ) -> PoolInitForMapPlanLikeCpp {
        pool_mgr.init_pools_for_map_plan_like_cpp(
            self.map_id,
            &mut self.pool_data,
            explicit_roll_for,
            choose_equal,
        )
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

    pub fn update_game_object_with_pool_update_like_cpp(
        &mut self,
        game_object_guid: ObjectGuid,
        diff_ms: u32,
        game_time_secs: i64,
        spawn_store: &SpawnStore,
        pool_mgr: &PoolMgrLikeCpp,
    ) -> GameObjectUpdateOutcomeLikeCpp {
        self.update_game_object_with_optional_pool_update_like_cpp(
            game_object_guid,
            diff_ms,
            game_time_secs,
            Some((spawn_store, pool_mgr)),
            None::<
                &mut fn(
                    &mut Self,
                    SpawnObjectType,
                    SpawnId,
                ) -> Option<LoadedGridRespawnRecordsLikeCpp>,
            >,
        )
    }

    pub fn update_game_object_with_pool_update_loaded_grid_records_like_cpp<L>(
        &mut self,
        game_object_guid: ObjectGuid,
        diff_ms: u32,
        game_time_secs: i64,
        spawn_store: &SpawnStore,
        pool_mgr: &PoolMgrLikeCpp,
        mut load_record: L,
    ) -> GameObjectUpdateOutcomeLikeCpp
    where
        L: FnMut(&mut Self, SpawnObjectType, SpawnId) -> Option<LoadedGridRespawnRecordsLikeCpp>,
    {
        self.update_game_object_with_optional_pool_update_like_cpp(
            game_object_guid,
            diff_ms,
            game_time_secs,
            Some((spawn_store, pool_mgr)),
            Some(&mut load_record),
        )
    }

    pub(super) fn update_game_object_with_optional_pool_update_like_cpp<L>(
        &mut self,
        game_object_guid: ObjectGuid,
        diff_ms: u32,
        game_time_secs: i64,
        pool_update: Option<(&SpawnStore, &PoolMgrLikeCpp)>,
        mut load_record: Option<&mut L>,
    ) -> GameObjectUpdateOutcomeLikeCpp
    where
        L: FnMut(&mut Self, SpawnObjectType, SpawnId) -> Option<LoadedGridRespawnRecordsLikeCpp>,
    {
        let Some(record) = self.map_object_record(game_object_guid) else {
            return GameObjectUpdateOutcomeLikeCpp {
                game_object_guid,
                diff_ms,
                status: GameObjectUpdateStatusLikeCpp::MissingGameObject,
                despawn_delay_before_ms: None,
                despawn_delay_after_ms: None,
                despawn_respawn_time_secs: None,
                world_update_would_run: false,
                ai_update_not_represented: false,
                go_type_impl_update_not_represented: false,
                despawn_or_unsummon_requested: false,
                entity_update: None,
                remove_list: None,
                linked_trap_guid: None,
                linked_trap_removed: false,
                linked_trap_remove_queued: false,
                linked_trap_missing_or_self: false,
                loot_cleared: false,
                goober_spell_cast_spell_id: None,
                goober_spell_casts_represented: 0,
                goober_users_cleared: false,
                goober_state_reset: false,
                goober_nodespawn_return: false,
                non_consumed_chest_or_goober_return: false,
                non_consumed_restock_armed: false,
                non_consumed_set_ready: false,
                non_consumed_update_visibility_represented: false,
                non_consumed_update_dynamic_flags_represented: false,
                non_consumed_source_missing: false,
                summoned_expired_delete: false,
                summoned_expired_respawn_time_zeroed: false,
                summoned_expired_despawn_represented: false,
                summoned_expired_go_state_ready: false,
                new_flag_drop_owner_in_base_command_represented: false,
                new_flag_drop_owner_missing_or_empty: false,
                new_flag_drop_owner_wrong_kind: false,
                new_flag_drop_owner_not_new_flag: false,
                generic_not_ready: false,
                generic_capture_point_removed_represented: false,
                generic_visual_despawn_represented: false,
                generic_flags_restored_represented: false,
                generic_zero_respawn_delay_return: false,
                generic_despawn_at_action_source_missing: false,
                generic_respawn_scheduled_time: None,
                generic_spawned_by_default_branch: false,
                generic_temporary_respawn_zeroed: false,
                generic_respawn_timer_add: None,
                generic_respawn_save_missing_spawn_id: false,
                generic_respawn_save_missing_gameobject_data: false,
                generic_respawn_compatibility_db_only_represented: false,
                generic_visibility_on_destroy_represented: false,
            };
        };

        if record.kind() != AccessorObjectKind::GameObject {
            return GameObjectUpdateOutcomeLikeCpp {
                game_object_guid,
                diff_ms,
                status: GameObjectUpdateStatusLikeCpp::NotGameObject,
                despawn_delay_before_ms: None,
                despawn_delay_after_ms: None,
                despawn_respawn_time_secs: None,
                world_update_would_run: false,
                ai_update_not_represented: false,
                go_type_impl_update_not_represented: false,
                despawn_or_unsummon_requested: false,
                entity_update: None,
                remove_list: None,
                linked_trap_guid: None,
                linked_trap_removed: false,
                linked_trap_remove_queued: false,
                linked_trap_missing_or_self: false,
                loot_cleared: false,
                goober_spell_cast_spell_id: None,
                goober_spell_casts_represented: 0,
                goober_users_cleared: false,
                goober_state_reset: false,
                goober_nodespawn_return: false,
                non_consumed_chest_or_goober_return: false,
                non_consumed_restock_armed: false,
                non_consumed_set_ready: false,
                non_consumed_update_visibility_represented: false,
                non_consumed_update_dynamic_flags_represented: false,
                non_consumed_source_missing: false,
                summoned_expired_delete: false,
                summoned_expired_respawn_time_zeroed: false,
                summoned_expired_despawn_represented: false,
                summoned_expired_go_state_ready: false,
                new_flag_drop_owner_in_base_command_represented: false,
                new_flag_drop_owner_missing_or_empty: false,
                new_flag_drop_owner_wrong_kind: false,
                new_flag_drop_owner_not_new_flag: false,
                generic_not_ready: false,
                generic_capture_point_removed_represented: false,
                generic_visual_despawn_represented: false,
                generic_flags_restored_represented: false,
                generic_zero_respawn_delay_return: false,
                generic_despawn_at_action_source_missing: false,
                generic_respawn_scheduled_time: None,
                generic_spawned_by_default_branch: false,
                generic_temporary_respawn_zeroed: false,
                generic_respawn_timer_add: None,
                generic_respawn_save_missing_spawn_id: false,
                generic_respawn_save_missing_gameobject_data: false,
                generic_respawn_compatibility_db_only_represented: false,
                generic_visibility_on_destroy_represented: false,
            };
        }

        let Some(game_object) = record.game_object() else {
            return GameObjectUpdateOutcomeLikeCpp {
                game_object_guid,
                diff_ms,
                status: GameObjectUpdateStatusLikeCpp::NotGameObject,
                despawn_delay_before_ms: None,
                despawn_delay_after_ms: None,
                despawn_respawn_time_secs: None,
                world_update_would_run: false,
                ai_update_not_represented: false,
                go_type_impl_update_not_represented: false,
                despawn_or_unsummon_requested: false,
                entity_update: None,
                remove_list: None,
                linked_trap_guid: None,
                linked_trap_removed: false,
                linked_trap_remove_queued: false,
                linked_trap_missing_or_self: false,
                loot_cleared: false,
                goober_spell_cast_spell_id: None,
                goober_spell_casts_represented: 0,
                goober_users_cleared: false,
                goober_state_reset: false,
                goober_nodespawn_return: false,
                non_consumed_chest_or_goober_return: false,
                non_consumed_restock_armed: false,
                non_consumed_set_ready: false,
                non_consumed_update_visibility_represented: false,
                non_consumed_update_dynamic_flags_represented: false,
                non_consumed_source_missing: false,
                summoned_expired_delete: false,
                summoned_expired_respawn_time_zeroed: false,
                summoned_expired_despawn_represented: false,
                summoned_expired_go_state_ready: false,
                new_flag_drop_owner_in_base_command_represented: false,
                new_flag_drop_owner_missing_or_empty: false,
                new_flag_drop_owner_wrong_kind: false,
                new_flag_drop_owner_not_new_flag: false,
                generic_not_ready: false,
                generic_capture_point_removed_represented: false,
                generic_visual_despawn_represented: false,
                generic_flags_restored_represented: false,
                generic_zero_respawn_delay_return: false,
                generic_despawn_at_action_source_missing: false,
                generic_respawn_scheduled_time: None,
                generic_spawned_by_default_branch: false,
                generic_temporary_respawn_zeroed: false,
                generic_respawn_timer_add: None,
                generic_respawn_save_missing_spawn_id: false,
                generic_respawn_save_missing_gameobject_data: false,
                generic_respawn_compatibility_db_only_represented: false,
                generic_visibility_on_destroy_represented: false,
            };
        };

        let despawn_delay_before_ms = game_object.despawn_delay();
        let despawn_respawn_time_secs = game_object.despawn_respawn_time();
        if !game_object.world().object().is_in_world() {
            return GameObjectUpdateOutcomeLikeCpp {
                game_object_guid,
                diff_ms,
                status: GameObjectUpdateStatusLikeCpp::NotInWorld,
                despawn_delay_before_ms: Some(despawn_delay_before_ms),
                despawn_delay_after_ms: Some(despawn_delay_before_ms),
                despawn_respawn_time_secs: Some(despawn_respawn_time_secs),
                world_update_would_run: false,
                ai_update_not_represented: false,
                go_type_impl_update_not_represented: false,
                despawn_or_unsummon_requested: false,
                entity_update: None,
                remove_list: None,
                linked_trap_guid: None,
                linked_trap_removed: false,
                linked_trap_remove_queued: false,
                linked_trap_missing_or_self: false,
                loot_cleared: false,
                goober_spell_cast_spell_id: None,
                goober_spell_casts_represented: 0,
                goober_users_cleared: false,
                goober_state_reset: false,
                goober_nodespawn_return: false,
                non_consumed_chest_or_goober_return: false,
                non_consumed_restock_armed: false,
                non_consumed_set_ready: false,
                non_consumed_update_visibility_represented: false,
                non_consumed_update_dynamic_flags_represented: false,
                non_consumed_source_missing: false,
                summoned_expired_delete: false,
                summoned_expired_respawn_time_zeroed: false,
                summoned_expired_despawn_represented: false,
                summoned_expired_go_state_ready: false,
                new_flag_drop_owner_in_base_command_represented: false,
                new_flag_drop_owner_missing_or_empty: false,
                new_flag_drop_owner_wrong_kind: false,
                new_flag_drop_owner_not_new_flag: false,
                generic_not_ready: false,
                generic_capture_point_removed_represented: false,
                generic_visual_despawn_represented: false,
                generic_flags_restored_represented: false,
                generic_zero_respawn_delay_return: false,
                generic_despawn_at_action_source_missing: false,
                generic_respawn_scheduled_time: None,
                generic_spawned_by_default_branch: false,
                generic_temporary_respawn_zeroed: false,
                generic_respawn_timer_add: None,
                generic_respawn_save_missing_spawn_id: false,
                generic_respawn_save_missing_gameobject_data: false,
                generic_respawn_compatibility_db_only_represented: false,
                generic_visibility_on_destroy_represented: false,
            };
        }

        let entity_update = {
            let Some(record) = self.entity_world.get_mut(&game_object_guid) else {
                return GameObjectUpdateOutcomeLikeCpp {
                    game_object_guid,
                    diff_ms,
                    status: GameObjectUpdateStatusLikeCpp::MissingGameObject,
                    despawn_delay_before_ms: Some(despawn_delay_before_ms),
                    despawn_delay_after_ms: Some(despawn_delay_before_ms),
                    despawn_respawn_time_secs: Some(despawn_respawn_time_secs),
                    world_update_would_run: false,
                    ai_update_not_represented: false,
                    go_type_impl_update_not_represented: false,
                    despawn_or_unsummon_requested: false,
                    entity_update: None,
                    remove_list: None,
                    linked_trap_guid: None,
                    linked_trap_removed: false,
                    linked_trap_remove_queued: false,
                    linked_trap_missing_or_self: false,
                    loot_cleared: false,
                    goober_spell_cast_spell_id: None,
                    goober_spell_casts_represented: 0,
                    goober_users_cleared: false,
                    goober_state_reset: false,
                    goober_nodespawn_return: false,
                    non_consumed_chest_or_goober_return: false,
                    non_consumed_restock_armed: false,
                    non_consumed_set_ready: false,
                    non_consumed_update_visibility_represented: false,
                    non_consumed_update_dynamic_flags_represented: false,
                    non_consumed_source_missing: false,
                    summoned_expired_delete: false,
                    summoned_expired_respawn_time_zeroed: false,
                    summoned_expired_despawn_represented: false,
                    summoned_expired_go_state_ready: false,
                    new_flag_drop_owner_in_base_command_represented: false,
                    new_flag_drop_owner_missing_or_empty: false,
                    new_flag_drop_owner_wrong_kind: false,
                    new_flag_drop_owner_not_new_flag: false,
                    generic_not_ready: false,
                    generic_capture_point_removed_represented: false,
                    generic_visual_despawn_represented: false,
                    generic_flags_restored_represented: false,
                    generic_zero_respawn_delay_return: false,
                    generic_despawn_at_action_source_missing: false,
                    generic_respawn_scheduled_time: None,
                    generic_spawned_by_default_branch: false,
                    generic_temporary_respawn_zeroed: false,
                    generic_respawn_timer_add: None,
                    generic_respawn_save_missing_spawn_id: false,
                    generic_respawn_save_missing_gameobject_data: false,
                    generic_respawn_compatibility_db_only_represented: false,
                    generic_visibility_on_destroy_represented: false,
                };
            };
            let Some(game_object) = record.game_object_mut() else {
                return GameObjectUpdateOutcomeLikeCpp {
                    game_object_guid,
                    diff_ms,
                    status: GameObjectUpdateStatusLikeCpp::NotGameObject,
                    despawn_delay_before_ms: Some(despawn_delay_before_ms),
                    despawn_delay_after_ms: Some(despawn_delay_before_ms),
                    despawn_respawn_time_secs: Some(despawn_respawn_time_secs),
                    world_update_would_run: false,
                    ai_update_not_represented: false,
                    go_type_impl_update_not_represented: false,
                    despawn_or_unsummon_requested: false,
                    entity_update: None,
                    remove_list: None,
                    linked_trap_guid: None,
                    linked_trap_removed: false,
                    linked_trap_remove_queued: false,
                    linked_trap_missing_or_self: false,
                    loot_cleared: false,
                    goober_spell_cast_spell_id: None,
                    goober_spell_casts_represented: 0,
                    goober_users_cleared: false,
                    goober_state_reset: false,
                    goober_nodespawn_return: false,
                    non_consumed_chest_or_goober_return: false,
                    non_consumed_restock_armed: false,
                    non_consumed_set_ready: false,
                    non_consumed_update_visibility_represented: false,
                    non_consumed_update_dynamic_flags_represented: false,
                    non_consumed_source_missing: false,
                    summoned_expired_delete: false,
                    summoned_expired_respawn_time_zeroed: false,
                    summoned_expired_despawn_represented: false,
                    summoned_expired_go_state_ready: false,
                    new_flag_drop_owner_in_base_command_represented: false,
                    new_flag_drop_owner_missing_or_empty: false,
                    new_flag_drop_owner_wrong_kind: false,
                    new_flag_drop_owner_not_new_flag: false,
                    generic_not_ready: false,
                    generic_capture_point_removed_represented: false,
                    generic_visual_despawn_represented: false,
                    generic_flags_restored_represented: false,
                    generic_zero_respawn_delay_return: false,
                    generic_despawn_at_action_source_missing: false,
                    generic_respawn_scheduled_time: None,
                    generic_spawned_by_default_branch: false,
                    generic_temporary_respawn_zeroed: false,
                    generic_respawn_timer_add: None,
                    generic_respawn_save_missing_spawn_id: false,
                    generic_respawn_save_missing_gameobject_data: false,
                    generic_respawn_compatibility_db_only_represented: false,
                    generic_visibility_on_destroy_represented: false,
                };
            };
            game_object.update_like_cpp(diff_ms)
        };

        let (
            linked_trap_guid,
            linked_trap_removed,
            linked_trap_remove_queued,
            linked_trap_missing_or_self,
        ) = if entity_update.status == EntityGameObjectUpdateStatusLikeCpp::DespawnRequested {
            (None, false, false, false)
        } else {
            self.map_object_record(game_object_guid)
                .and_then(MapObjectRecord::game_object)
                .filter(|game_object| game_object.loot_state() == LootState::JustDeactivated)
                .map(|game_object| game_object.linked_trap_guid_like_cpp())
                .map_or((None, false, false, false), |linked_guid| {
                    if linked_guid.is_empty() || linked_guid == game_object_guid {
                        return (
                            (!linked_guid.is_empty()).then_some(linked_guid),
                            false,
                            false,
                            true,
                        );
                    }

                    let linked_trap_exists = self
                        .map_object_record(linked_guid)
                        .filter(|record| record.kind() == AccessorObjectKind::GameObject)
                        .and_then(MapObjectRecord::game_object)
                        .is_some();
                    if !linked_trap_exists {
                        return (Some(linked_guid), false, false, true);
                    }

                    match self.gameobject_delete_from_update_with_optional_loader_like_cpp(
                        linked_guid,
                        pool_update,
                        load_record.as_mut().map(|loader| &mut **loader),
                    ) {
                        Some(delete) => (
                            Some(linked_guid),
                            false,
                            delete
                                .remove_list
                                .as_ref()
                                .is_some_and(|remove| remove.queued || remove.duplicate),
                            false,
                        ),
                        None => (Some(linked_guid), false, false, true),
                    }
                })
        };

        let mut goober_spell_cast_spell_id = None;
        let mut goober_spell_casts_represented = 0;
        let mut goober_users_cleared = false;
        let mut goober_state_reset = false;
        let mut goober_nodespawn_return = false;
        let mut non_consumed_chest_or_goober_return = false;
        let mut non_consumed_restock_armed = false;
        let mut non_consumed_set_ready = false;
        let mut non_consumed_update_visibility_represented = false;
        let mut non_consumed_update_dynamic_flags_represented = false;
        let mut non_consumed_source_missing = false;
        let mut summoned_expired_delete = false;
        let mut summoned_expired_respawn_time_zeroed = false;
        let mut summoned_expired_despawn_represented = false;
        let mut summoned_expired_go_state_ready = false;
        let mut new_flag_drop_owner_in_base_command_represented = false;
        let mut new_flag_drop_owner_missing_or_empty = false;
        let mut new_flag_drop_owner_wrong_kind = false;
        let mut new_flag_drop_owner_not_new_flag = false;
        let mut generic_not_ready = false;
        let mut generic_visual_despawn_represented = false;
        let mut generic_flags_restored_represented = false;
        let mut generic_zero_respawn_delay_return = false;
        let mut generic_despawn_at_action_source_missing = false;
        let mut generic_respawn_scheduled_time = None;
        let mut generic_spawned_by_default_branch = false;
        let mut generic_temporary_respawn_zeroed = false;
        let mut generic_respawn_timer_add = None;
        let mut generic_respawn_save_missing_spawn_id = false;
        let mut generic_respawn_save_missing_gameobject_data = false;
        let mut generic_respawn_compatibility_db_only_represented = false;
        let mut generic_visibility_on_destroy_represented = false;

        if entity_update.status != EntityGameObjectUpdateStatusLikeCpp::DespawnRequested {
            if let Some(game_object) = self
                .entity_world
                .get_mut(&game_object_guid)
                .and_then(MapObjectRecord::game_object_mut)
                .filter(|game_object| game_object.loot_state() == LootState::JustDeactivated)
                .filter(|game_object| game_object.data().type_id == GAMEOBJECT_TYPE_GOOBER as i8)
            {
                if let Some(goober_source) = game_object.represented_goober_use_source_like_cpp() {
                    if goober_source.spell_id != 0 {
                        goober_spell_cast_spell_id = Some(goober_source.spell_id);
                        goober_spell_casts_represented =
                            game_object.unique_users_snapshot_like_cpp().len();
                        game_object.clear_unique_users_and_reset_use_times_like_cpp();
                        goober_users_cleared = true;
                    }

                    if goober_source.lock_id != 0 || goober_source.auto_close_ms != 0 {
                        game_object.set_go_state(GoState::Ready);
                        goober_state_reset = true;
                    }
                }

                goober_nodespawn_return = game_object.data().flags & GO_FLAG_NODESPAWN != 0;
            }
        }

        let loot_cleared = if entity_update.status
            == EntityGameObjectUpdateStatusLikeCpp::DespawnRequested
            || goober_nodespawn_return
        {
            false
        } else if let Some(game_object) = self
            .entity_world
            .get_mut(&game_object_guid)
            .and_then(MapObjectRecord::game_object_mut)
            .filter(|game_object| game_object.loot_state() == LootState::JustDeactivated)
        {
            game_object.clear_loot_like_cpp();
            true
        } else {
            false
        };

        if loot_cleared {
            if let Some(game_object) = self
                .entity_world
                .get_mut(&game_object_guid)
                .and_then(MapObjectRecord::game_object_mut)
            {
                let go_type = game_object.data().type_id as u32;
                let despawn_at_action = match go_type {
                    GAMEOBJECT_TYPE_CHEST => game_object
                        .represented_chest_loot_source_like_cpp()
                        .map(|source| source.chest_consumable),
                    GAMEOBJECT_TYPE_GOOBER => game_object
                        .represented_goober_use_source_like_cpp()
                        .map(|source| source.consumable),
                    _ => None,
                };

                if matches!(go_type, GAMEOBJECT_TYPE_CHEST | GAMEOBJECT_TYPE_GOOBER) {
                    // C++ anchor: GameObject.cpp:1609-1623. This represented seam
                    // deliberately does not call the broader SetLootState facade from
                    // GameObject.cpp:3683-3709 because line 1617 only writes
                    // GO_NOT_READY after arming the fully-looted chest restock timer;
                    // Activated-specific restock/collision semantics are not part of
                    // this branch. Owner/spell-created expiration is consumed below
                    // through the represented `Delete()` seam.
                    if let Some(despawn_at_action) = despawn_at_action {
                        let is_summoned_and_expired = (game_object.owner_guid()
                            != ObjectGuid::EMPTY
                            || game_object.spell_id() != 0)
                            && game_object.respawn_time() == 0;
                        if !despawn_at_action && !is_summoned_and_expired {
                            if go_type == GAMEOBJECT_TYPE_CHEST {
                                if let Some(source) =
                                    game_object.represented_chest_loot_source_like_cpp()
                                {
                                    if source.chest_restock_time_secs > 0 {
                                        let restock_time = game_time_secs.saturating_add(
                                            i64::from(source.chest_restock_time_secs),
                                        );
                                        game_object.set_restock_time_like_cpp(restock_time);
                                        game_object.set_loot_state(LootState::NotReady, None);
                                        non_consumed_restock_armed = true;
                                        non_consumed_update_dynamic_flags_represented = true;
                                    } else {
                                        game_object.set_loot_state(LootState::Ready, None);
                                        non_consumed_set_ready = true;
                                    }
                                }
                            } else {
                                game_object.set_loot_state(LootState::Ready, None);
                                non_consumed_set_ready = true;
                            }
                            non_consumed_chest_or_goober_return = true;
                            non_consumed_update_visibility_represented = true;
                        }
                    } else {
                        non_consumed_source_missing = true;
                    }
                }
            }
        }

        if loot_cleared && !non_consumed_chest_or_goober_return {
            let summoned_snapshot = self
                .map_object_record(game_object_guid)
                .and_then(MapObjectRecord::game_object)
                .filter(|game_object| game_object.loot_state() == LootState::JustDeactivated)
                .map(|game_object| {
                    (
                        game_object.data().type_id as u32,
                        game_object.owner_guid(),
                        game_object.spell_id(),
                        game_object.respawn_time(),
                    )
                });

            if let Some((go_type, owner_guid, spell_id, respawn_time)) = summoned_snapshot {
                let is_summoned_and_expired =
                    (owner_guid != ObjectGuid::EMPTY || spell_id != 0) && respawn_time == 0;
                if is_summoned_and_expired {
                    if let Some(game_object) = self
                        .entity_world
                        .get_mut(&game_object_guid)
                        .and_then(MapObjectRecord::game_object_mut)
                    {
                        game_object.set_respawn_time(0);
                        game_object.set_loot_state(LootState::NotReady, None);
                        summoned_expired_respawn_time_zeroed = true;
                        summoned_expired_despawn_represented = true;
                        if go_type != GAMEOBJECT_TYPE_TRANSPORT {
                            game_object.set_go_state(GoState::Ready);
                            summoned_expired_go_state_ready = true;
                        }
                    }

                    if go_type == GAMEOBJECT_TYPE_NEW_FLAG_DROP {
                        if owner_guid == ObjectGuid::EMPTY {
                            new_flag_drop_owner_missing_or_empty = true;
                        } else {
                            match self.map_object_record(owner_guid) {
                                Some(owner_record)
                                    if owner_record.kind() == AccessorObjectKind::GameObject =>
                                {
                                    match owner_record.game_object() {
                                        Some(owner_go)
                                            if owner_go.data().type_id as u32
                                                == GAMEOBJECT_TYPE_NEW_FLAG =>
                                        {
                                            // C++ NewFlag::SetState(InBase, nullptr) has
                                            // no full Rust go-type state object yet; record
                                            // the exact typed owner command as represented
                                            // evidence only, without faking ZoneScript or
                                            // fanout.
                                            new_flag_drop_owner_in_base_command_represented = true;
                                        }
                                        Some(_) => {
                                            new_flag_drop_owner_not_new_flag = true;
                                        }
                                        None => {
                                            new_flag_drop_owner_wrong_kind = true;
                                        }
                                    }
                                }
                                Some(_) => {
                                    new_flag_drop_owner_wrong_kind = true;
                                }
                                None => {
                                    new_flag_drop_owner_missing_or_empty = true;
                                }
                            }
                        }
                    }

                    summoned_expired_delete = true;
                }
            }
        }

        if loot_cleared && !non_consumed_chest_or_goober_return && !summoned_expired_delete {
            if let Some(game_object) = self
                .entity_world
                .get_mut(&game_object_guid)
                .and_then(MapObjectRecord::game_object_mut)
                .filter(|game_object| game_object.loot_state() == LootState::JustDeactivated)
            {
                // C++ anchor: GameObject.cpp:1639-1651. This represented seam
                // preserves the `if (!m_respawnDelayTime) return;` early return;
                // the positive-delay scheduling/SaveRespawnTime tail is consumed
                // immediately below after releasing the typed GameObject borrow.
                game_object.set_loot_state(LootState::NotReady, None);
                generic_not_ready = true;

                let go_type = game_object.data().type_id as u32;
                let despawn_at_action = match go_type {
                    GAMEOBJECT_TYPE_CHEST => game_object
                        .represented_chest_loot_source_like_cpp()
                        .map(|source| source.chest_consumable),
                    GAMEOBJECT_TYPE_GOOBER => game_object
                        .represented_goober_use_source_like_cpp()
                        .map(|source| source.consumable),
                    _ => Some(false),
                };
                generic_despawn_at_action_source_missing = despawn_at_action.is_none();
                let visual_despawn = despawn_at_action.unwrap_or(false)
                    || game_object.go_anim_progress_like_cpp() > 0;
                if visual_despawn {
                    generic_visual_despawn_represented = true;
                    generic_flags_restored_represented =
                        game_object.restore_represented_baseline_flags_like_cpp();
                }
                generic_zero_respawn_delay_return = game_object.respawn_delay_time() == 0;
            }
        }

        if generic_not_ready && !generic_zero_respawn_delay_return {
            let generic_respawn_snapshot = self
                .map_object_record(game_object_guid)
                .and_then(MapObjectRecord::game_object)
                .map(|game_object| {
                    (
                        game_object.spawned_by_default(),
                        game_object.respawn_compatibility_mode(),
                        game_object.respawn_delay_time(),
                        game_object.spawn_id(),
                        game_object.has_represented_gameobject_data_like_cpp(),
                        game_object.world().object().entry(),
                        game_object.world().position(),
                    )
                });

            if let Some((
                spawned_by_default,
                respawn_compatibility_mode,
                respawn_delay_time,
                spawn_id,
                represented_gameobject_data_present,
                entry,
                position,
            )) = generic_respawn_snapshot
            {
                if spawned_by_default {
                    let scheduled_respawn_time =
                        game_time_secs.saturating_add(i64::from(respawn_delay_time));
                    if let Some(game_object) = self
                        .entity_world
                        .get_mut(&game_object_guid)
                        .and_then(MapObjectRecord::game_object_mut)
                    {
                        game_object.set_respawn_time(scheduled_respawn_time);
                    }
                    generic_respawn_scheduled_time = Some(scheduled_respawn_time);
                    generic_spawned_by_default_branch = true;

                    if !represented_gameobject_data_present {
                        // C++ `GameObject::SaveRespawnTime` is guarded by `m_goData`.
                        // A nonzero spawn id is not enough evidence for map-owned
                        // respawn persistence in this represented seam.
                        generic_respawn_save_missing_gameobject_data = true;
                    } else if spawn_id == 0 {
                        generic_respawn_save_missing_spawn_id = true;
                    } else if scheduled_respawn_time > game_time_secs {
                        if respawn_compatibility_mode {
                            // C++ `SaveRespawnTime` compatibility mode calls
                            // `SaveRespawnInfoDB` only. `wow-map` owns no async DB
                            // writes, so record DB-only evidence without mutating the
                            // map-owned respawn store.
                            generic_respawn_compatibility_db_only_represented = true;
                        } else {
                            let grid = compute_grid_coord(position.x, position.y);
                            let add_outcome = self.add_respawn_info_like_cpp(RespawnInfoLikeCpp {
                                object_type: SpawnObjectType::GameObject,
                                spawn_id,
                                entry,
                                respawn_time: scheduled_respawn_time,
                                grid_id: grid.get_id(),
                            });
                            generic_respawn_timer_add = Some(add_outcome);
                        }
                    }

                    if respawn_compatibility_mode {
                        generic_visibility_on_destroy_represented = true;
                    }
                } else {
                    if let Some(game_object) = self
                        .entity_world
                        .get_mut(&game_object_guid)
                        .and_then(MapObjectRecord::game_object_mut)
                    {
                        game_object.set_respawn_time(0);
                    }
                    generic_temporary_respawn_zeroed = true;
                    generic_visibility_on_destroy_represented = spawn_id != 0;
                }
            }
        }

        if summoned_expired_delete
            || (generic_not_ready
                && !generic_zero_respawn_delay_return
                && !generic_visibility_on_destroy_represented)
        {
            let delete = self.gameobject_delete_from_update_with_optional_loader_like_cpp(
                game_object_guid,
                pool_update,
                load_record.as_mut().map(|loader| &mut **loader),
            );
            let generic_capture_point_removed_represented = delete
                .as_ref()
                .is_some_and(|delete| delete.capture_point_packet_represented);
            let delete_visual_despawn_represented = delete
                .as_ref()
                .is_some_and(|delete| delete.despawn_packet_represented);
            let (status, remove_list) = match delete {
                Some(delete) if delete.pool_update_represented && delete.remove_list.is_none() => {
                    (GameObjectUpdateStatusLikeCpp::DespawnPoolUpdated, None)
                }
                Some(delete) => (
                    GameObjectUpdateStatusLikeCpp::DespawnRemoveQueued,
                    delete.remove_list,
                ),
                None => (GameObjectUpdateStatusLikeCpp::DespawnRemoveQueued, None),
            };
            GameObjectUpdateOutcomeLikeCpp {
                game_object_guid,
                diff_ms,
                status,
                despawn_delay_before_ms: Some(entity_update.despawn_delay_before_ms),
                despawn_delay_after_ms: Some(entity_update.despawn_delay_after_ms),
                despawn_respawn_time_secs: Some(entity_update.despawn_respawn_time_secs),
                world_update_would_run: entity_update.world_update_would_run,
                ai_update_not_represented: entity_update.ai_update_not_represented,
                go_type_impl_update_not_represented: entity_update
                    .go_type_impl_update_not_represented,
                despawn_or_unsummon_requested: entity_update.despawn_or_unsummon_requested,
                entity_update: Some(entity_update),
                remove_list,
                linked_trap_guid,
                linked_trap_removed,
                linked_trap_remove_queued,
                linked_trap_missing_or_self,
                loot_cleared,
                goober_spell_cast_spell_id,
                goober_spell_casts_represented,
                goober_users_cleared,
                goober_state_reset,
                goober_nodespawn_return,
                non_consumed_chest_or_goober_return: false,
                non_consumed_restock_armed: false,
                non_consumed_set_ready: false,
                non_consumed_update_visibility_represented: false,
                non_consumed_update_dynamic_flags_represented: false,
                non_consumed_source_missing,
                summoned_expired_delete,
                summoned_expired_respawn_time_zeroed,
                summoned_expired_despawn_represented,
                summoned_expired_go_state_ready,
                new_flag_drop_owner_in_base_command_represented,
                new_flag_drop_owner_missing_or_empty,
                new_flag_drop_owner_wrong_kind,
                new_flag_drop_owner_not_new_flag,
                generic_not_ready,
                generic_capture_point_removed_represented,
                generic_visual_despawn_represented: generic_visual_despawn_represented
                    || delete_visual_despawn_represented,
                generic_flags_restored_represented,
                generic_zero_respawn_delay_return,
                generic_despawn_at_action_source_missing,
                generic_respawn_scheduled_time,
                generic_spawned_by_default_branch,
                generic_temporary_respawn_zeroed,
                generic_respawn_timer_add,
                generic_respawn_save_missing_spawn_id,
                generic_respawn_save_missing_gameobject_data,
                generic_respawn_compatibility_db_only_represented,
                generic_visibility_on_destroy_represented,
            }
        } else if entity_update.status == EntityGameObjectUpdateStatusLikeCpp::DespawnRequested {
            let delete = self.gameobject_delete_from_update_with_optional_loader_like_cpp(
                game_object_guid,
                pool_update,
                load_record.as_mut().map(|loader| &mut **loader),
            );
            let generic_capture_point_removed_represented = delete
                .as_ref()
                .is_some_and(|delete| delete.capture_point_packet_represented);
            let delete_visual_despawn_represented = delete
                .as_ref()
                .is_some_and(|delete| delete.despawn_packet_represented);
            let (status, remove_list) = match delete {
                Some(delete) if delete.pool_update_represented && delete.remove_list.is_none() => {
                    (GameObjectUpdateStatusLikeCpp::DespawnPoolUpdated, None)
                }
                Some(delete) => (
                    GameObjectUpdateStatusLikeCpp::DespawnRemoveQueued,
                    delete.remove_list,
                ),
                None => (GameObjectUpdateStatusLikeCpp::DespawnRemoveQueued, None),
            };
            GameObjectUpdateOutcomeLikeCpp {
                game_object_guid,
                diff_ms,
                status,
                despawn_delay_before_ms: Some(entity_update.despawn_delay_before_ms),
                despawn_delay_after_ms: Some(entity_update.despawn_delay_after_ms),
                despawn_respawn_time_secs: Some(entity_update.despawn_respawn_time_secs),
                world_update_would_run: entity_update.world_update_would_run,
                ai_update_not_represented: entity_update.ai_update_not_represented,
                go_type_impl_update_not_represented: entity_update
                    .go_type_impl_update_not_represented,
                despawn_or_unsummon_requested: entity_update.despawn_or_unsummon_requested,
                entity_update: Some(entity_update),
                remove_list,
                linked_trap_guid,
                linked_trap_removed,
                linked_trap_remove_queued,
                linked_trap_missing_or_self,
                loot_cleared: false,
                goober_spell_cast_spell_id: None,
                goober_spell_casts_represented: 0,
                goober_users_cleared: false,
                goober_state_reset: false,
                goober_nodespawn_return: false,
                non_consumed_chest_or_goober_return: false,
                non_consumed_restock_armed: false,
                non_consumed_set_ready: false,
                non_consumed_update_visibility_represented: false,
                non_consumed_update_dynamic_flags_represented: false,
                non_consumed_source_missing: false,
                summoned_expired_delete: false,
                summoned_expired_respawn_time_zeroed: false,
                summoned_expired_despawn_represented: false,
                summoned_expired_go_state_ready: false,
                new_flag_drop_owner_in_base_command_represented: false,
                new_flag_drop_owner_missing_or_empty: false,
                new_flag_drop_owner_wrong_kind: false,
                new_flag_drop_owner_not_new_flag: false,
                generic_not_ready: false,
                generic_capture_point_removed_represented,
                generic_visual_despawn_represented: delete_visual_despawn_represented,
                generic_flags_restored_represented: false,
                generic_zero_respawn_delay_return: false,
                generic_despawn_at_action_source_missing: false,
                generic_respawn_scheduled_time: None,
                generic_spawned_by_default_branch: false,
                generic_temporary_respawn_zeroed: false,
                generic_respawn_timer_add: None,
                generic_respawn_save_missing_spawn_id: false,
                generic_respawn_save_missing_gameobject_data: false,
                generic_respawn_compatibility_db_only_represented: false,
                generic_visibility_on_destroy_represented: false,
            }
        } else {
            GameObjectUpdateOutcomeLikeCpp {
                game_object_guid,
                diff_ms,
                status: GameObjectUpdateStatusLikeCpp::Updated,
                despawn_delay_before_ms: Some(entity_update.despawn_delay_before_ms),
                despawn_delay_after_ms: Some(entity_update.despawn_delay_after_ms),
                despawn_respawn_time_secs: Some(entity_update.despawn_respawn_time_secs),
                world_update_would_run: entity_update.world_update_would_run,
                ai_update_not_represented: entity_update.ai_update_not_represented,
                go_type_impl_update_not_represented: entity_update
                    .go_type_impl_update_not_represented,
                despawn_or_unsummon_requested: entity_update.despawn_or_unsummon_requested,
                entity_update: Some(entity_update),
                remove_list: None,
                linked_trap_guid,
                linked_trap_removed,
                linked_trap_remove_queued,
                linked_trap_missing_or_self,
                loot_cleared,
                goober_spell_cast_spell_id,
                goober_spell_casts_represented,
                goober_users_cleared,
                goober_state_reset,
                goober_nodespawn_return,
                non_consumed_chest_or_goober_return,
                non_consumed_restock_armed,
                non_consumed_set_ready,
                non_consumed_update_visibility_represented,
                non_consumed_update_dynamic_flags_represented,
                non_consumed_source_missing,
                summoned_expired_delete,
                summoned_expired_respawn_time_zeroed,
                summoned_expired_despawn_represented,
                summoned_expired_go_state_ready,
                new_flag_drop_owner_in_base_command_represented,
                new_flag_drop_owner_missing_or_empty,
                new_flag_drop_owner_wrong_kind,
                new_flag_drop_owner_not_new_flag,
                generic_not_ready,
                generic_capture_point_removed_represented: false,
                generic_visual_despawn_represented,
                generic_flags_restored_represented,
                generic_zero_respawn_delay_return,
                generic_despawn_at_action_source_missing,
                generic_respawn_scheduled_time,
                generic_spawned_by_default_branch,
                generic_temporary_respawn_zeroed,
                generic_respawn_timer_add,
                generic_respawn_save_missing_spawn_id,
                generic_respawn_save_missing_gameobject_data,
                generic_respawn_compatibility_db_only_represented,
                generic_visibility_on_destroy_represented,
            }
        }
    }

    pub fn update_game_objects_with_pool_update_like_cpp(
        &mut self,
        diff_ms: u32,
        game_time_secs: i64,
        spawn_store: &SpawnStore,
        pool_mgr: &PoolMgrLikeCpp,
    ) -> GameObjectsUpdateSummaryLikeCpp {
        self.update_game_objects_with_optional_pool_update_like_cpp(
            diff_ms,
            game_time_secs,
            Some((spawn_store, pool_mgr)),
            None::<
                &mut fn(
                    &mut Self,
                    SpawnObjectType,
                    SpawnId,
                ) -> Option<LoadedGridRespawnRecordsLikeCpp>,
            >,
        )
    }

    pub fn update_game_objects_with_pool_update_loaded_grid_records_like_cpp<L>(
        &mut self,
        diff_ms: u32,
        game_time_secs: i64,
        spawn_store: &SpawnStore,
        pool_mgr: &PoolMgrLikeCpp,
        mut load_record: L,
    ) -> GameObjectsUpdateSummaryLikeCpp
    where
        L: FnMut(&mut Self, SpawnObjectType, SpawnId) -> Option<LoadedGridRespawnRecordsLikeCpp>,
    {
        self.update_game_objects_with_optional_pool_update_like_cpp(
            diff_ms,
            game_time_secs,
            Some((spawn_store, pool_mgr)),
            Some(&mut load_record),
        )
    }

    pub(super) fn update_game_objects_with_optional_pool_update_like_cpp<L>(
        &mut self,
        diff_ms: u32,
        game_time_secs: i64,
        pool_update: Option<(&SpawnStore, &PoolMgrLikeCpp)>,
        mut load_record: Option<&mut L>,
    ) -> GameObjectsUpdateSummaryLikeCpp
    where
        L: FnMut(&mut Self, SpawnObjectType, SpawnId) -> Option<LoadedGridRespawnRecordsLikeCpp>,
    {
        let game_object_guids = self
            .entity_world
            .iter()
            .filter_map(|(guid, record)| {
                (record.kind() == AccessorObjectKind::GameObject && record.game_object().is_some())
                    .then_some(*guid)
            })
            .collect::<Vec<_>>();

        let mut summary = GameObjectsUpdateSummaryLikeCpp::default();
        for guid in game_object_guids {
            summary.visited += 1;
            let outcome = self.update_game_object_with_optional_pool_update_like_cpp(
                guid,
                diff_ms,
                game_time_secs,
                pool_update,
                load_record.as_mut().map(|loader| &mut **loader),
            );
            if outcome.linked_trap_removed {
                summary.linked_traps_removed += 1;
            }
            if outcome.linked_trap_remove_queued {
                summary.linked_traps_remove_queued += 1;
            }
            if outcome.loot_cleared {
                summary.loot_cleared += 1;
            }
            summary.goober_spell_casts_represented += outcome.goober_spell_casts_represented;
            if outcome.goober_users_cleared {
                summary.goober_users_cleared += 1;
            }
            if outcome.goober_state_reset {
                summary.goober_state_reset += 1;
            }
            if outcome.goober_nodespawn_return {
                summary.goober_nodespawn_returns += 1;
            }
            if outcome.non_consumed_chest_or_goober_return {
                summary.non_consumed_chest_or_goober_returns += 1;
            }
            if outcome.non_consumed_restock_armed {
                summary.non_consumed_restock_armed += 1;
            }
            if outcome.non_consumed_set_ready {
                summary.non_consumed_set_ready += 1;
            }
            if outcome.non_consumed_update_visibility_represented {
                summary.non_consumed_update_visibility_represented += 1;
            }
            if outcome.non_consumed_update_dynamic_flags_represented {
                summary.non_consumed_update_dynamic_flags_represented += 1;
            }
            if outcome.non_consumed_source_missing {
                summary.non_consumed_source_missing += 1;
            }
            if outcome.summoned_expired_delete {
                summary.summoned_expired_deletes += 1;
            }
            if outcome.summoned_expired_respawn_time_zeroed {
                summary.summoned_expired_respawn_time_zeroed += 1;
            }
            if outcome.summoned_expired_despawn_represented {
                summary.summoned_expired_despawn_represented += 1;
            }
            if outcome.summoned_expired_go_state_ready {
                summary.summoned_expired_go_state_ready += 1;
            }
            if outcome.new_flag_drop_owner_in_base_command_represented {
                summary.new_flag_drop_owner_in_base_commands_represented += 1;
            }
            if outcome.new_flag_drop_owner_missing_or_empty {
                summary.new_flag_drop_owner_missing_or_empty += 1;
            }
            if outcome.new_flag_drop_owner_wrong_kind {
                summary.new_flag_drop_owner_wrong_kind += 1;
            }
            if outcome.new_flag_drop_owner_not_new_flag {
                summary.new_flag_drop_owner_not_new_flag += 1;
            }
            if outcome.generic_not_ready {
                summary.generic_not_ready += 1;
            }
            if outcome.generic_capture_point_removed_represented {
                summary.generic_capture_point_removed_represented += 1;
                summary
                    .generic_capture_point_removed_guids
                    .push(outcome.game_object_guid);
            }
            if outcome.generic_visual_despawn_represented {
                summary.generic_visual_despawn_represented += 1;
                summary
                    .generic_visual_despawn_guids
                    .push(outcome.game_object_guid);
            }
            if outcome.generic_flags_restored_represented {
                summary.generic_flags_restored_represented += 1;
            }
            if outcome.generic_zero_respawn_delay_return {
                summary.generic_zero_respawn_delay_returns += 1;
            }
            if outcome.generic_despawn_at_action_source_missing {
                summary.generic_despawn_at_action_source_missing += 1;
            }
            if outcome.generic_respawn_scheduled_time.is_some() {
                summary.generic_respawn_scheduled += 1;
            }
            if outcome.generic_spawned_by_default_branch {
                summary.generic_spawned_by_default_branches += 1;
            }
            if outcome.generic_temporary_respawn_zeroed {
                summary.generic_temporary_respawn_zeroed += 1;
            }
            let map_timer_added = matches!(
                outcome.generic_respawn_timer_add,
                Some(
                    AddRespawnInfoOutcomeLikeCpp::Inserted
                        | AddRespawnInfoOutcomeLikeCpp::ReplacedExisting
                )
            );
            if map_timer_added {
                summary.generic_respawn_timer_added += 1;
            }
            if outcome.generic_respawn_save_missing_spawn_id {
                summary.generic_respawn_save_missing_spawn_id += 1;
            }
            if outcome.generic_respawn_save_missing_gameobject_data {
                summary.generic_respawn_save_missing_gameobject_data += 1;
            }
            if outcome.generic_respawn_compatibility_db_only_represented {
                summary.generic_respawn_compatibility_db_only_represented += 1;
            }
            if (map_timer_added || outcome.generic_respawn_compatibility_db_only_represented)
                && let (Some(respawn_time), Some(game_object)) = (
                    outcome.generic_respawn_scheduled_time,
                    self.map_object_record(outcome.game_object_guid)
                        .and_then(MapObjectRecord::game_object),
                )
            {
                let position = game_object.world().position();
                summary.respawn_db_saves.push(RespawnInfoLikeCpp {
                    object_type: SpawnObjectType::GameObject,
                    spawn_id: game_object.spawn_id(),
                    entry: game_object.world().object().entry(),
                    respawn_time,
                    grid_id: compute_grid_coord(position.x, position.y).get_id(),
                });
            }
            if outcome.generic_visibility_on_destroy_represented {
                summary.generic_visibility_on_destroy_represented += 1;
                summary
                    .generic_visibility_on_destroy_guids
                    .push(outcome.game_object_guid);
            }
            match outcome.status {
                GameObjectUpdateStatusLikeCpp::Updated => summary.updated += 1,
                GameObjectUpdateStatusLikeCpp::DespawnRemoveQueued => {
                    summary.despawn_remove_queued += 1;
                }
                GameObjectUpdateStatusLikeCpp::DespawnPoolUpdated => {
                    summary.despawn_pool_updated += 1;
                }
                GameObjectUpdateStatusLikeCpp::MissingGameObject => summary.missing_or_stale += 1,
                GameObjectUpdateStatusLikeCpp::NotGameObject => summary.not_game_object += 1,
                GameObjectUpdateStatusLikeCpp::NotInWorld => summary.not_in_world += 1,
            }
        }

        summary
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

                for pre_add_record in records.pre_add_records {
                    let _ = self.add_map_object_record_to_map_like_cpp(pre_add_record);
                }
                let primary_record = records.primary_record;
                let loaded_grid_primary_record = primary_record.clone();
                match self.add_map_object_record_to_map_like_cpp(primary_record) {
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
                        spawn_outcome = Some(self.spawn_group_spawn_loaded_grid_records_like_cpp(
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

    /// Bounded map-owned representation of C++ `GameObject::Delete()` with
    /// the compatibility-mode `PoolMgr::UpdatePool<GameObject>` branch.
    ///
    /// C++ anchors:
    /// - `GameObject.cpp:1759-1763`: if `m_respawnCompatibilityMode && poolid`,
    ///   call `sPoolMgr->UpdatePool<GameObject>(..., poolid, GetSpawnId())`;
    ///   otherwise call `AddObjectToRemoveList()`.
    /// - `PoolMgr.cpp:891-905`: `UpdatePool<T>` either updates a mother pool
    ///   or spawns from the typed pool using the triggering spawn id.
    ///
    /// This helper consumes only the represented map-owned PoolMgr plan. It does
    /// not perform DB writes, fabricate DB-backed GameObjects, or fan out packets.
    pub fn gameobject_delete_with_pool_update_like_cpp<R, C>(
        &mut self,
        guid: ObjectGuid,
        spawn_store: &SpawnStore,
        pool_mgr: &PoolMgrLikeCpp,
        explicit_roll_for: R,
        choose_equal: C,
    ) -> Option<GameObjectDeleteOutcomeLikeCpp>
    where
        R: FnMut(PoolMemberKindLikeCpp, u32) -> f32,
        C: FnMut(&[PoolObjectLikeCpp], usize) -> Vec<usize>,
    {
        self.gameobject_delete_with_optional_pool_update_loader_like_cpp::<R, C, fn(
            &mut Self,
            SpawnObjectType,
            SpawnId,
        ) -> Option<LoadedGridRespawnRecordsLikeCpp>>(
            guid,
            spawn_store,
            pool_mgr,
            explicit_roll_for,
            choose_equal,
            None,
        )
    }

    pub fn gameobject_delete_with_pool_update_loaded_grid_records_like_cpp<R, C, L>(
        &mut self,
        guid: ObjectGuid,
        spawn_store: &SpawnStore,
        pool_mgr: &PoolMgrLikeCpp,
        explicit_roll_for: R,
        choose_equal: C,
        load_record: L,
    ) -> Option<GameObjectDeleteOutcomeLikeCpp>
    where
        R: FnMut(PoolMemberKindLikeCpp, u32) -> f32,
        C: FnMut(&[PoolObjectLikeCpp], usize) -> Vec<usize>,
        L: FnMut(&mut Self, SpawnObjectType, SpawnId) -> Option<LoadedGridRespawnRecordsLikeCpp>,
    {
        self.gameobject_delete_with_optional_pool_update_loader_like_cpp(
            guid,
            spawn_store,
            pool_mgr,
            explicit_roll_for,
            choose_equal,
            Some(load_record),
        )
    }

    fn gameobject_delete_with_optional_pool_update_loader_like_cpp<R, C, L>(
        &mut self,
        guid: ObjectGuid,
        spawn_store: &SpawnStore,
        pool_mgr: &PoolMgrLikeCpp,
        mut explicit_roll_for: R,
        mut choose_equal: C,
        load_record: Option<L>,
    ) -> Option<GameObjectDeleteOutcomeLikeCpp>
    where
        R: FnMut(PoolMemberKindLikeCpp, u32) -> f32,
        C: FnMut(&[PoolObjectLikeCpp], usize) -> Vec<usize>,
        L: FnMut(&mut Self, SpawnObjectType, SpawnId) -> Option<LoadedGridRespawnRecordsLikeCpp>,
    {
        let (go_type, spawn_id, respawn_compatibility_mode, represented_gameobject_data_present) =
            self.map_object_record(guid)
                .filter(|record| record.kind() == AccessorObjectKind::GameObject)
                .and_then(MapObjectRecord::game_object)
                .map(|game_object| {
                    (
                        game_object.data().type_id as u32,
                        game_object.spawn_id(),
                        game_object.respawn_compatibility_mode(),
                        game_object.has_represented_gameobject_data_like_cpp(),
                    )
                })?;

        if let Some(game_object) = self
            .entity_world
            .get_mut(&guid)
            .and_then(MapObjectRecord::game_object_mut)
        {
            game_object.loot_authority_like_cpp().detach_like_cpp();
            game_object.set_loot_state(LootState::NotReady, None);
        }
        let remove_from_owner = self.gameobject_remove_from_owner_like_cpp(guid);
        let capture_point_packet_represented = go_type == GAMEOBJECT_TYPE_CAPTURE_POINT;
        let despawn_packet_represented = true;

        let (go_state_ready, flags_restored) = self
            .entity_world
            .get_mut(&guid)
            .and_then(MapObjectRecord::game_object_mut)
            .map(|game_object| {
                let go_state_ready = go_type != GAMEOBJECT_TYPE_TRANSPORT;
                if go_state_ready {
                    game_object.set_go_state(GoState::Ready);
                }
                let flags_restored = game_object.restore_represented_baseline_flags_like_cpp();
                (go_state_ready, flags_restored)
            })
            .unwrap_or((false, false));

        let pool_id =
            if respawn_compatibility_mode && represented_gameobject_data_present && spawn_id != 0 {
                spawn_store
                    .spawn_data(SpawnObjectType::GameObject, spawn_id)
                    .map(|spawn| spawn.pool_id)
                    .unwrap_or(0)
            } else {
                0
            };

        let mut pool_update_plan = None;
        let mut pool_update_error = None;
        let mut pool_update_summary = None;
        let mut remove_list = None;

        if pool_id != 0 {
            match pool_mgr.update_pool_plan_like_cpp(
                &mut self.pool_data,
                pool_id,
                SpawnObjectType::GameObject,
                spawn_id,
                &mut explicit_roll_for,
                &mut choose_equal,
            ) {
                Ok(plan) => {
                    let mut summary = ProcessRespawnsSafeSideEffectsSummaryLikeCpp::default();
                    if let Some(mut load_record) = load_record {
                        self.apply_pool_typed_spawn_plan_loaded_grid_records_like_cpp(
                            &plan,
                            spawn_store,
                            &mut summary,
                            Some(&mut load_record),
                        );
                    } else {
                        self.apply_pool_typed_spawn_plan_safe_map_actions_like_cpp(
                            &plan,
                            spawn_store,
                            &mut summary,
                        );
                    }
                    pool_update_summary = Some(summary);
                    pool_update_plan = Some(plan);
                }
                Err(error) => {
                    pool_update_error = Some(error);
                }
            }
        } else {
            remove_list = Some(self.add_object_to_remove_list_like_cpp(guid));
        }

        Some(GameObjectDeleteOutcomeLikeCpp {
            guid,
            remove_from_owner,
            capture_point_packet_represented,
            despawn_packet_represented,
            go_state_ready,
            flags_restored,
            pool_update_represented: pool_update_plan.is_some(),
            pool_update_plan,
            pool_update_error,
            pool_update_summary,
            remove_list,
        })
    }
}
