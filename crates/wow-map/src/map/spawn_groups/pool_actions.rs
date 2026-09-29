// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Pool planning and map-local pool actions.

mod gameobject_update;

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
    pub(in crate::map) fn apply_pool_typed_spawn_plan_safe_map_actions_like_cpp(
        &mut self,
        plan: &PoolTypedSpawnPlanLikeCpp,
        spawn_store: &SpawnStore,
        summary: &mut ProcessRespawnsSafeSideEffectsSummaryLikeCpp,
    ) {
        self.apply_pool_typed_spawn_plan_loaded_grid_records_like_cpp::<
            fn(&mut Self, SpawnObjectType, SpawnId) -> Option<LoadedGridRespawnRecordsLikeCpp>,
        >(plan, spawn_store, summary, None);
    }

    pub(in crate::map) fn apply_pool_typed_spawn_plan_loaded_grid_records_like_cpp<L>(
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

    pub(in crate::map) fn update_game_objects_with_optional_pool_update_like_cpp<L>(
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
