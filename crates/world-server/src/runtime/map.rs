//! Canonical map respawn persistence, periodic work, and update loops.

use wow_persistence::{
    GameEventPersistenceMutationLikeCpp, GameEventPersistenceMutationOutcomeLikeCpp,
    GameEventPersistencePortLikeCpp,
};

use super::*;

/// Supply the Group owner's loaded-difficulty port from the DB2 store.
///
/// `wow-social` owns the Group rules but must not depend on a data adapter, so
/// the composition root binds the port to the concrete `wow_data` store. Both
/// the trait and the store are foreign to this crate, so the binding is an
/// explicit borrowing adapter rather than a blanket impl. Every method forwards
/// to the existing C++-anchored validation unchanged.
pub(crate) struct GroupDifficultyStorePortLikeCpp<'a>(pub(crate) &'a wow_data::DifficultyStore);

impl wow_social::group::GroupDifficultyValidatorLikeCpp for GroupDifficultyStorePortLikeCpp<'_> {
    fn check_loaded_dungeon_difficulty_id_like_cpp(&self, difficulty: u32) -> u32 {
        self.0
            .check_loaded_dungeon_difficulty_id_like_cpp(difficulty)
    }

    fn check_loaded_raid_difficulty_id_like_cpp(&self, difficulty: u32) -> u32 {
        self.0.check_loaded_raid_difficulty_id_like_cpp(difficulty)
    }

    fn check_loaded_legacy_raid_difficulty_id_like_cpp(&self, difficulty: u32) -> u32 {
        self.0
            .check_loaded_legacy_raid_difficulty_id_like_cpp(difficulty)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CanonicalRespawnConditionSchedulerLikeCpp {
    pub(crate) timer_ms: u32,
    pub(crate) interval_ms: u32,
}

impl CanonicalRespawnConditionSchedulerLikeCpp {
    pub(crate) fn new(interval_ms: u32) -> Self {
        let interval_ms = interval_ms.max(1);
        Self {
            timer_ms: interval_ms,
            interval_ms,
        }
    }

    pub(crate) fn update(&mut self, diff_ms: u32) -> bool {
        if self.timer_ms <= diff_ms {
            self.timer_ms = self.interval_ms;
            true
        } else {
            self.timer_ms -= diff_ms;
            false
        }
    }

    #[cfg(test)]
    pub(crate) const fn timer_ms(&self) -> u32 {
        self.timer_ms
    }
}

#[derive(Debug, Clone)]
pub(crate) struct RespawnDbDeleteLikeCpp {
    pub(crate) object_type: wow_map::SpawnObjectType,
    pub(crate) spawn_id: wow_map::SpawnId,
    pub(crate) map_id: u16,
    pub(crate) instance_id: u32,
    pub(crate) mutation: RespawnPersistenceMutationLikeCpp,
}

#[derive(Debug, Clone)]
pub(crate) enum RespawnDbDeleteQueueOutcomeLikeCpp {
    Queued(RespawnDbDeleteLikeCpp),
    SkippedNonWorldMap,
    SkippedInstanceableMap,
    SkippedInvalidMapId,
}

#[derive(Debug, Clone)]
pub(crate) struct RespawnDbSaveLikeCpp {
    pub(crate) object_type: wow_map::SpawnObjectType,
    pub(crate) spawn_id: wow_map::SpawnId,
    pub(crate) respawn_time: i64,
    pub(crate) map_id: u16,
    pub(crate) instance_id: u32,
    pub(crate) mutation: RespawnPersistenceMutationLikeCpp,
}

#[derive(Debug, Clone)]
pub(crate) enum RespawnDbSaveQueueOutcomeLikeCpp {
    Queued(RespawnDbSaveLikeCpp),
    SkippedNonWorldMap,
    SkippedInstanceableMap,
    SkippedInvalidMapId,
}

pub(crate) fn queue_respawn_db_delete_like_cpp(
    map_kind: wow_map::ManagedMapKind,
    map_is_instanceable: bool,
    map_id: u32,
    instance_id: u32,
    object_type: wow_map::SpawnObjectType,
    spawn_id: wow_map::SpawnId,
) -> RespawnDbDeleteQueueOutcomeLikeCpp {
    if !matches!(map_kind, wow_map::ManagedMapKind::World) {
        return RespawnDbDeleteQueueOutcomeLikeCpp::SkippedNonWorldMap;
    }
    if map_is_instanceable {
        return RespawnDbDeleteQueueOutcomeLikeCpp::SkippedInstanceableMap;
    }

    let Ok(map_id) = u16::try_from(map_id) else {
        return RespawnDbDeleteQueueOutcomeLikeCpp::SkippedInvalidMapId;
    };

    let mutation = RespawnPersistenceMutationLikeCpp::Delete {
        key: RespawnPersistenceKeyLikeCpp {
            object_type_raw: u16::from(object_type as u8),
            spawn_id,
            map_id,
            instance_id,
        },
    };
    RespawnDbDeleteQueueOutcomeLikeCpp::Queued(RespawnDbDeleteLikeCpp {
        object_type,
        spawn_id,
        map_id,
        instance_id,
        mutation,
    })
}

pub(crate) fn queue_respawn_db_save_like_cpp(
    map_kind: wow_map::ManagedMapKind,
    map_is_instanceable: bool,
    map_id: u32,
    instance_id: u32,
    info: wow_map::RespawnInfoLikeCpp,
) -> RespawnDbSaveQueueOutcomeLikeCpp {
    if !matches!(map_kind, wow_map::ManagedMapKind::World) {
        return RespawnDbSaveQueueOutcomeLikeCpp::SkippedNonWorldMap;
    }
    if map_is_instanceable {
        return RespawnDbSaveQueueOutcomeLikeCpp::SkippedInstanceableMap;
    }

    let Ok(map_id) = u16::try_from(map_id) else {
        return RespawnDbSaveQueueOutcomeLikeCpp::SkippedInvalidMapId;
    };

    let mutation = RespawnPersistenceMutationLikeCpp::Save {
        key: RespawnPersistenceKeyLikeCpp {
            object_type_raw: u16::from(info.object_type as u8),
            spawn_id: info.spawn_id,
            map_id,
            instance_id,
        },
        respawn_time: info.respawn_time,
    };
    RespawnDbSaveQueueOutcomeLikeCpp::Queued(RespawnDbSaveLikeCpp {
        object_type: info.object_type,
        spawn_id: info.spawn_id,
        respawn_time: info.respawn_time,
        map_id,
        instance_id,
        mutation,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GameEventWorldEventStateDbOperationKindLikeCpp {
    Save,
    Delete,
}

#[derive(Debug, Clone)]
pub(crate) struct GameEventWorldEventStateDbOperationLikeCpp {
    pub(crate) event_id: u8,
    pub(crate) kind: GameEventWorldEventStateDbOperationKindLikeCpp,
    pub(crate) delete_condition_saves: bool,
    pub(crate) delete_world_event_state: bool,
    pub(crate) mutation: GameEventPersistenceMutationLikeCpp,
}

#[derive(Debug, Default, Clone)]
pub(crate) struct GameEventWorldEventStateDbBridgeSummaryLikeCpp {
    pub(crate) saves_queued: usize,
    pub(crate) saves_executed: usize,
    pub(crate) saves_failed: usize,
    pub(crate) saves_skipped_event_id_out_of_range: usize,
    pub(crate) saves_skipped_missing_event: usize,
    pub(crate) deletes_queued: usize,
    pub(crate) deletes_executed: usize,
    pub(crate) deletes_failed: usize,
    pub(crate) deletes_skipped_event_id_out_of_range: usize,
    pub(crate) condition_delete_rows_queued: usize,
    pub(crate) condition_delete_rows_executed: usize,
    pub(crate) condition_delete_rows_failed: usize,
    pub(crate) operations: Vec<GameEventWorldEventStateDbOperationLikeCpp>,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub(crate) struct GameEventQuestCompleteConditionSaveDbOperationLikeCpp {
    pub(crate) event_id: u8,
    pub(crate) condition_id: u32,
    pub(crate) mutation: GameEventPersistenceMutationLikeCpp,
}

#[allow(dead_code)]
#[derive(Debug, Default, Clone)]
pub(crate) struct GameEventQuestCompleteDbBridgeSummaryLikeCpp {
    pub(crate) condition_save_updates_queued: usize,
    pub(crate) condition_save_updates_executed: usize,
    pub(crate) condition_save_updates_failed: usize,
    pub(crate) condition_save_updates_skipped_non_progress: usize,
    pub(crate) world_event_state_save_requested: usize,
    pub(crate) force_game_event_update_requested: usize,
    pub(crate) save_world_event_state_requested: bool,
    pub(crate) force_game_event_update_requested_flag: bool,
    pub(crate) world_event_state_summary: GameEventWorldEventStateDbBridgeSummaryLikeCpp,
    pub(crate) operations: Vec<GameEventQuestCompleteConditionSaveDbOperationLikeCpp>,
}

pub(crate) async fn load_groups_from_character_database_like_cpp(
    persistence: &dyn wow_persistence::RepresentedGroupStartupLoadPortLikeCpp,
    group_registry: &GroupRegistry,
    difficulty_store: &wow_data::DifficultyStore,
) -> Result<GroupLoadSummaryLikeCpp> {
    let (characters, groups, members) = match persistence.load_represented_groups_like_cpp().await {
        wow_persistence::RepresentedGroupStartupLoadOutcomeLikeCpp::Loaded {
            characters,
            groups,
            members,
        } => (characters, groups, members),
        wow_persistence::RepresentedGroupStartupLoadOutcomeLikeCpp::Failed { stage, reason } => {
            anyhow::bail!("represented Group startup persistence failed at {stage:?}: {reason}")
        }
    };
    let character_cache = characters
        .into_iter()
        .filter(|character| character.guid != 0)
        .map(|character| {
            (
                character.guid,
                GroupMemberCharacterLikeCpp {
                    name: character.name,
                    race: character.race,
                    class: character.class,
                },
            )
        })
        .collect::<BTreeMap<_, _>>();
    let group_rows = groups.into_iter().map(|row| GroupDbRowLikeCpp {
        leader_guid_low: row.leader_guid_low,
        loot_method: row.loot_method,
        looter_guid_low: row.looter_guid_low,
        loot_threshold: row.loot_threshold,
        target_icons: row.target_icons,
        group_flags: row.group_flags,
        dungeon_difficulty_id: row.dungeon_difficulty_id,
        raid_difficulty_id: row.raid_difficulty_id,
        legacy_raid_difficulty_id: row.legacy_raid_difficulty_id,
        master_looter_guid_low: row.master_looter_guid_low,
        db_store_id: row.db_store_id,
        lfg_dungeon_id: row.lfg_dungeon_id,
        lfg_state: row.lfg_state,
    });
    let member_rows = members.into_iter().map(|row| GroupMemberDbRowLikeCpp {
        db_store_id: row.db_store_id,
        member_guid_low: row.member_guid_low,
        member_flags: row.member_flags,
        subgroup: row.subgroup,
        roles: row.roles,
    });

    Ok(load_groups_from_db_rows_like_cpp(
        group_registry,
        group_rows,
        member_rows,
        &character_cache,
        &GroupDifficultyStorePortLikeCpp(difficulty_store),
    ))
}

#[allow(dead_code)]
pub(crate) fn materialize_game_event_quest_complete_db_bridge_like_cpp(
    outcome: &spawn_store_loader::GameEventQuestCompleteOutcomeLikeCpp,
    metadata: &spawn_store_loader::CanonicalSpawnMetadataLikeCpp,
) -> GameEventQuestCompleteDbBridgeSummaryLikeCpp {
    let mut summary = GameEventQuestCompleteDbBridgeSummaryLikeCpp::default();
    let spawn_store_loader::GameEventQuestCompleteOutcomeLikeCpp::Progress(
        spawn_store_loader::GameEventConditionProgressOutcomeLikeCpp::Progressed(progress),
    ) = outcome
    else {
        summary.condition_save_updates_skipped_non_progress += 1;
        return summary;
    };

    if progress.save_world_event_state_requested {
        summary.world_event_state_save_requested += 1;
        summary.save_world_event_state_requested = true;
    }
    if progress.force_game_event_update_requested {
        summary.force_game_event_update_requested += 1;
        summary.force_game_event_update_requested_flag = true;
    }

    summary.condition_save_updates_queued += 1;
    summary
        .operations
        .push(GameEventQuestCompleteConditionSaveDbOperationLikeCpp {
            event_id: progress.persistence_event_id,
            condition_id: progress.condition_id,
            mutation: GameEventPersistenceMutationLikeCpp::ReplaceConditionSave {
                event_id: progress.persistence_event_id,
                condition_id: progress.condition_id,
                done: progress.done_after,
            },
        });

    if progress.save_world_event_state_requested {
        game_event_world_event_state_db_save_operation_like_cpp(
            progress.event_id,
            metadata,
            &mut summary.world_event_state_summary,
        );
    }

    summary
}

#[allow(dead_code)]
pub(crate) async fn execute_game_event_quest_complete_condition_save_db_bridge_like_cpp(
    persistence: &dyn GameEventPersistencePortLikeCpp,
    summary: &mut GameEventQuestCompleteDbBridgeSummaryLikeCpp,
) {
    let operation_total = summary.operations.len();
    for (operation_index, operation) in summary.operations.drain(..).enumerate() {
        match persistence
            .execute_mutation_like_cpp(operation.mutation)
            .await
        {
            GameEventPersistenceMutationOutcomeLikeCpp::Applied => {
                summary.condition_save_updates_executed += 1
            }
            GameEventPersistenceMutationOutcomeLikeCpp::Failed { reason } => {
                summary.condition_save_updates_failed += 1;
                tracing::error!(
                    error = %reason,
                    operation_index = operation_index + 1,
                    operation_total,
                    event_id = operation.event_id,
                    condition_id = operation.condition_id,
                    "Failed to execute C++ GameEventMgr quest-complete condition-save DB transaction; continuing live update loop"
                );
            }
        }
    }
}

pub(crate) fn game_event_world_event_state_db_save_operation_like_cpp(
    event_id: u16,
    metadata: &spawn_store_loader::CanonicalSpawnMetadataLikeCpp,
    summary: &mut GameEventWorldEventStateDbBridgeSummaryLikeCpp,
) {
    let Ok(event_id_u8) = u8::try_from(event_id) else {
        summary.saves_skipped_event_id_out_of_range += 1;
        return;
    };
    let Some(event) = metadata.game_event_like_cpp(event_id) else {
        summary.saves_skipped_missing_event += 1;
        return;
    };
    let Ok(next_start) = i64::try_from(event.next_start) else {
        summary.saves_skipped_missing_event += 1;
        return;
    };

    summary.saves_queued += 1;
    summary
        .operations
        .push(GameEventWorldEventStateDbOperationLikeCpp {
            event_id: event_id_u8,
            kind: GameEventWorldEventStateDbOperationKindLikeCpp::Save,
            delete_condition_saves: false,
            delete_world_event_state: false,
            mutation: GameEventPersistenceMutationLikeCpp::SaveWorldEventState {
                event_id: event_id_u8,
                state: event.state_raw,
                next_start,
            },
        });
}

pub(crate) fn game_event_world_event_state_db_delete_operation_like_cpp(
    event_id: u16,
    delete_condition_saves_requested: bool,
    delete_world_event_state_requested: bool,
    summary: &mut GameEventWorldEventStateDbBridgeSummaryLikeCpp,
) {
    if !delete_condition_saves_requested && !delete_world_event_state_requested {
        return;
    }
    let Ok(event_id_u8) = u8::try_from(event_id) else {
        summary.deletes_skipped_event_id_out_of_range += 1;
        return;
    };

    if delete_condition_saves_requested {
        summary.condition_delete_rows_queued += 1;
    }
    if delete_world_event_state_requested {
        summary.deletes_queued += 1;
    }

    summary
        .operations
        .push(GameEventWorldEventStateDbOperationLikeCpp {
            event_id: event_id_u8,
            kind: GameEventWorldEventStateDbOperationKindLikeCpp::Delete,
            delete_condition_saves: delete_condition_saves_requested,
            delete_world_event_state: delete_world_event_state_requested,
            mutation: GameEventPersistenceMutationLikeCpp::DeleteWorldEventState {
                event_id: event_id_u8,
                delete_condition_saves: delete_condition_saves_requested,
                delete_world_event_state: delete_world_event_state_requested,
            },
        });
}

pub(crate) fn materialize_game_event_world_event_state_db_bridge_like_cpp(
    outcome: &spawn_store_loader::GameEventUpdateOutcomeLikeCpp,
    metadata: &spawn_store_loader::CanonicalSpawnMetadataLikeCpp,
) -> GameEventWorldEventStateDbBridgeSummaryLikeCpp {
    let mut summary = GameEventWorldEventStateDbBridgeSummaryLikeCpp::default();

    for save in &outcome.world_nextphase_finished {
        if save.save_state_requested {
            game_event_world_event_state_db_save_operation_like_cpp(
                save.event_id,
                metadata,
                &mut summary,
            );
        }
    }
    for save in &outcome.world_conditions_save_requested {
        game_event_world_event_state_db_save_operation_like_cpp(
            save.event_id,
            metadata,
            &mut summary,
        );
    }
    for start_outcome in &outcome.start_outcomes {
        if let spawn_store_loader::GameEventStartOutcomeLikeCpp::Started(start) = start_outcome {
            if start.save_world_event_state_requested {
                game_event_world_event_state_db_save_operation_like_cpp(
                    start.event_id,
                    metadata,
                    &mut summary,
                );
            }
        }
    }
    for stop_outcome in &outcome.stop_outcomes {
        if let spawn_store_loader::GameEventStopOutcomeLikeCpp::Stopped(stop) = stop_outcome {
            game_event_world_event_state_db_delete_operation_like_cpp(
                stop.event_id,
                stop.delete_condition_saves_requested,
                stop.delete_world_event_state_requested,
                &mut summary,
            );
        }
    }

    summary
}

pub(crate) async fn execute_game_event_world_event_state_db_bridge_like_cpp(
    persistence: &dyn GameEventPersistencePortLikeCpp,
    summary: &mut GameEventWorldEventStateDbBridgeSummaryLikeCpp,
) {
    let operation_total = summary.operations.len();
    for (operation_index, operation) in summary.operations.drain(..).enumerate() {
        match persistence
            .execute_mutation_like_cpp(operation.mutation)
            .await
        {
            GameEventPersistenceMutationOutcomeLikeCpp::Applied => match operation.kind {
                GameEventWorldEventStateDbOperationKindLikeCpp::Save => summary.saves_executed += 1,
                GameEventWorldEventStateDbOperationKindLikeCpp::Delete => {
                    if operation.delete_world_event_state {
                        summary.deletes_executed += 1;
                    }
                    if operation.delete_condition_saves {
                        summary.condition_delete_rows_executed += 1;
                    }
                }
            },
            GameEventPersistenceMutationOutcomeLikeCpp::Failed { reason } => {
                match operation.kind {
                    GameEventWorldEventStateDbOperationKindLikeCpp::Save => {
                        summary.saves_failed += 1;
                    }
                    GameEventWorldEventStateDbOperationKindLikeCpp::Delete => {
                        if operation.delete_world_event_state {
                            summary.deletes_failed += 1;
                        }
                        if operation.delete_condition_saves {
                            summary.condition_delete_rows_failed += 1;
                        }
                    }
                }
                tracing::error!(
                    error = %reason,
                    operation_index = operation_index + 1,
                    operation_total,
                    event_id = operation.event_id,
                    operation_kind = ?operation.kind,
                    "Failed to execute C++ GameEventMgr world-event state DB transaction; continuing live update loop"
                );
            }
        }
    }
}

#[derive(Clone)]
pub(crate) struct LoadedGridCreatureRespawnCachesLikeCpp {
    pub(crate) realm_id: u16,
    pub(crate) template_store: Arc<wow_data::CreatureTemplateLifecycleStoreLikeCpp>,
    pub(crate) sparring_store: Arc<wow_data::CreatureTemplateSparringStoreLikeCpp>,
    pub(crate) difficulty_store: Arc<wow_data::CreatureDifficultyStoreLikeCpp>,
    pub(crate) base_stats_store: Arc<wow_data::CreatureBaseStatsStoreLikeCpp>,
    pub(crate) chr_classes_store: Arc<wow_data::character_progression::ChrClassesStore>,
    pub(crate) power_type_store: Arc<wow_data::character_progression::PowerTypeStore>,
    pub(crate) health_rates: wow_data::CreatureClassificationHealthRatesLikeCpp,
    pub(crate) display_store: Arc<wow_data::CreatureDisplayInfoStore>,
    pub(crate) model_store: Arc<wow_data::CreatureModelDataStore>,
    pub(crate) model_info_store: Arc<wow_data::CreatureModelInfoStoreLikeCpp>,
    pub(crate) creature_equipment_store: Arc<wow_data::CreatureEquipmentStoreLikeCpp>,
    pub(crate) creature_addon_store: Arc<wow_data::CreatureAddonStoreLikeCpp>,
    pub(crate) vehicle_store: Arc<wow_data::VehicleStore>,
    pub(crate) vehicle_seat_store: Arc<wow_data::VehicleSeatStore>,
    pub(crate) vehicle_accessory_store: Arc<wow_data::VehicleAccessoryStoreLikeCpp>,
    pub(crate) gameobject_template_store: Arc<wow_data::GameObjectTemplateLifecycleStoreLikeCpp>,
    pub(crate) gameobject_override_store: Arc<wow_data::GameObjectOverrideLifecycleStoreLikeCpp>,
}

pub(crate) struct MapCreatureModelSelectionRandomLikeCpp<'a, Terrain, Lifecycle>
where
    Terrain: wow_map::TerrainGridLoader,
    Lifecycle: wow_map::GridLifecycle,
{
    map: &'a mut wow_map::Map<Terrain, Lifecycle>,
}

impl<Terrain, Lifecycle> wow_data::CreatureModelSelectionRandomLikeCpp
    for MapCreatureModelSelectionRandomLikeCpp<'_, Terrain, Lifecycle>
where
    Terrain: wow_map::TerrainGridLoader,
    Lifecycle: wow_map::GridLifecycle,
{
    fn weighted_model_roll_like_cpp(&mut self, total_weight: f32) -> f32 {
        self.map.frand_exclusive_like_cpp(0.0, total_weight)
    }

    fn other_gender_roll_zero_like_cpp(&mut self) -> bool {
        self.map.urand_inclusive_like_cpp(0, 1) == 0
    }
}

impl<Terrain, Lifecycle> creature_loaded_grid::LoadedGridCreatureRandomSourceLikeCpp
    for MapCreatureModelSelectionRandomLikeCpp<'_, Terrain, Lifecycle>
where
    Terrain: wow_map::TerrainGridLoader,
    Lifecycle: wow_map::GridLifecycle,
{
    fn select_creature_level_like_cpp(&mut self, min_level: u8, max_level: u8) -> u8 {
        self.map
            .select_creature_level_like_cpp(min_level, max_level)
    }
}

#[derive(Debug, Default, Clone)]
pub(crate) struct CanonicalSpawnGroupConditionTickSummaryLikeCpp {
    pub(crate) expired_pvp_combat_refs: Vec<(u32, u32, wow_core::ObjectGuid, wow_core::ObjectGuid)>,
    pub(crate) maps_evaluated: usize,
    pub(crate) outcomes: usize,
    pub(crate) applied_set_inactive: usize,
    pub(crate) planned_spawn: usize,
    pub(crate) condition_spawn_executed_loaded_grid_spawns: usize,
    pub(crate) condition_spawn_legacy_creature_mirrors: usize,
    pub(crate) condition_spawn_blocked_loaded_grid_spawn_loads: usize,
    pub(crate) condition_spawn_blocked_loaded_grid_creature_loads: usize,
    pub(crate) condition_spawn_blocked_loaded_grid_gameobject_loads: usize,
    pub(crate) condition_spawn_blocked_loaded_grid_spawn_add_to_map: usize,
    pub(crate) condition_spawn_load_plan_count: usize,
    pub(crate) condition_spawn_unsupported_spawn_types: usize,
    pub(crate) condition_spawn_skipped_respawn_timer_active: usize,
    pub(crate) condition_spawn_skipped_live_object_active: usize,
    pub(crate) condition_spawn_skipped_unloaded_grid: usize,
    pub(crate) condition_spawn_skipped_difficulty_mismatch: usize,
    pub(crate) planned_despawn: usize,
    pub(crate) despawn_executed: usize,
    pub(crate) despawn_objects_removed: usize,
    pub(crate) despawn_respawn_timers_removed: usize,
    pub(crate) despawn_blocked_missing_group: usize,
    pub(crate) despawn_blocked_system_group: usize,
    pub(crate) despawn_unsupported_live_types: usize,
    pub(crate) despawn_respawn_timer_unsupported_types: usize,
    pub(crate) despawn_stale_index_entries: usize,
    pub(crate) despawn_remove_errors: usize,
    pub(crate) respawn_deleted_inactive_spawn_group: usize,
    pub(crate) respawn_deleted_live_object_blocker: usize,
    pub(crate) respawn_processed_pool_timers: usize,
    pub(crate) respawn_processed_unloaded_grid_respawns: usize,
    pub(crate) respawn_executed_loaded_grid_respawns: usize,
    pub(crate) respawn_legacy_creature_mirrors: usize,
    pub(crate) respawn_blocked_loaded_grid_respawn_loads: usize,
    pub(crate) respawn_blocked_loaded_grid_respawn_add_to_map: usize,
    pub(crate) respawn_pool_update_plans: usize,
    pub(crate) respawn_blocked_pool_plan_errors: usize,
    pub(crate) respawn_blocked_missing_spawn_data: usize,
    pub(crate) respawn_blocked_pool_runtime: usize,
    pub(crate) respawn_blocked_do_respawn_runtime: usize,
    pub(crate) respawn_blocked_linked_respawn_non_future: usize,
    pub(crate) respawn_blocked_unsupported_spawn_type: usize,
    pub(crate) respawn_db_delete_queued: usize,
    pub(crate) respawn_db_delete_executed: usize,
    pub(crate) respawn_db_delete_failed: usize,
    pub(crate) respawn_db_delete_skipped_non_world_map: usize,
    pub(crate) respawn_db_delete_skipped_instanceable_map: usize,
    pub(crate) respawn_db_delete_skipped_invalid_map_id: usize,
    pub(crate) respawn_db_deletes: Vec<RespawnDbDeleteLikeCpp>,
    pub(crate) respawn_db_save_queued: usize,
    pub(crate) respawn_db_save_executed: usize,
    pub(crate) respawn_db_save_failed: usize,
    pub(crate) respawn_db_save_skipped_non_world_map: usize,
    pub(crate) respawn_db_save_skipped_instanceable_map: usize,
    pub(crate) respawn_db_save_skipped_invalid_map_id: usize,
    pub(crate) respawn_db_saves: Vec<RespawnDbSaveLikeCpp>,
}

pub(crate) fn build_loaded_grid_creature_respawn_record_like_cpp(
    map: &mut wow_map::Map,
    object_type: wow_map::SpawnObjectType,
    spawn_id: wow_map::SpawnId,
    canonical_spawn_metadata: &spawn_store_loader::CanonicalSpawnMetadataLikeCpp,
    caches: &LoadedGridCreatureRespawnCachesLikeCpp,
) -> Option<wow_map::map::LoadedGridRespawnRecordsLikeCpp> {
    let Some(respawn_time) = map
        .get_respawn_info_like_cpp(object_type, spawn_id)
        .map(|info| info.respawn_time)
    else {
        debug!(
            spawn_id,
            respawn_type = object_type as u8,
            "C++ loaded-grid Creature DoRespawn blocked: missing map-owned respawn timer before LoadFromDB"
        );
        return None;
    };
    build_loaded_grid_creature_record_with_respawn_time_like_cpp(
        map,
        object_type,
        spawn_id,
        canonical_spawn_metadata,
        caches,
        respawn_time,
    )
}

pub(crate) fn build_loaded_grid_creature_spawn_group_spawn_record_like_cpp(
    map: &mut wow_map::Map,
    object_type: wow_map::SpawnObjectType,
    spawn_id: wow_map::SpawnId,
    canonical_spawn_metadata: &spawn_store_loader::CanonicalSpawnMetadataLikeCpp,
    caches: &LoadedGridCreatureRespawnCachesLikeCpp,
) -> Option<wow_map::map::LoadedGridRespawnRecordsLikeCpp> {
    build_loaded_grid_creature_record_with_respawn_time_like_cpp(
        map,
        object_type,
        spawn_id,
        canonical_spawn_metadata,
        caches,
        0,
    )
}

pub(crate) fn build_loaded_grid_creature_record_with_respawn_time_like_cpp(
    map: &mut wow_map::Map,
    object_type: wow_map::SpawnObjectType,
    spawn_id: wow_map::SpawnId,
    canonical_spawn_metadata: &spawn_store_loader::CanonicalSpawnMetadataLikeCpp,
    caches: &LoadedGridCreatureRespawnCachesLikeCpp,
    respawn_time: i64,
) -> Option<wow_map::map::LoadedGridRespawnRecordsLikeCpp> {
    if object_type != wow_map::SpawnObjectType::Creature {
        return None;
    }

    let Some(spawn) = canonical_spawn_metadata
        .spawn_store()
        .spawn_data(object_type, spawn_id)
    else {
        debug!(
            respawn_type = object_type as u8,
            spawn_id, "C++ loaded-grid Creature DoRespawn blocked: missing canonical SpawnData"
        );
        return None;
    };
    let Some(runtime_row) = canonical_spawn_metadata.creature_runtime_row_like_cpp(spawn_id) else {
        debug!(
            spawn_id,
            entry = spawn.id,
            "C++ loaded-grid Creature DoRespawn blocked: missing DB-backed creature runtime row"
        );
        return None;
    };
    let Ok(map_id) = u16::try_from(map.map_id()) else {
        warn!(
            map_id = map.map_id(),
            spawn_id,
            entry = spawn.id,
            "C++ loaded-grid Creature DoRespawn blocked: map id does not fit ObjectGuid world-object map field"
        );
        return None;
    };
    let difficulty_id = map.spawn_mode();
    let instance_id = map.instance_id();
    let formation_info = canonical_spawn_metadata
        .creature_formation_info_like_cpp(spawn_id)
        .copied();
    let mut random = MapCreatureModelSelectionRandomLikeCpp { map };
    let inputs =
        creature_loaded_grid::build_loaded_grid_creature_inputs_with_power_stores_from_db_like_cpp(
            spawn,
            runtime_row,
            caches.template_store.as_ref(),
            caches.difficulty_store.as_ref(),
            caches.base_stats_store.as_ref(),
            &caches.health_rates,
            caches.display_store.as_ref(),
            caches.model_store.as_ref(),
            caches.model_info_store.as_ref(),
            Some(caches.creature_equipment_store.as_ref()),
            caches.creature_addon_store.as_ref(),
            Some(caches.chr_classes_store.as_ref()),
            Some(caches.power_type_store.as_ref()),
            difficulty_id,
            instance_id,
            respawn_time,
            true,
            formation_info,
            &mut random,
        );
    let (template, resolved_spawn, runtime_selection) = match inputs {
        Ok(inputs) => inputs,
        Err(error) => {
            debug!(
                ?error,
                spawn_id,
                entry = spawn.id,
                "C++ loaded-grid Creature DoRespawn blocked: failed to compose DB-backed LoadFromDB inputs"
            );
            return None;
        }
    };

    let low = match map.generate_low_guid_like_cpp(HighGuid::Creature) {
        Ok(low) => low,
        Err(error) => {
            debug!(
                ?error,
                spawn_id,
                entry = spawn.id,
                "C++ loaded-grid Creature DoRespawn blocked: map-owned Creature low-guid generation failed"
            );
            return None;
        }
    };
    let mut template = template;
    template.sparring_health_pct = caches
        .sparring_store
        .values_for_entry_like_cpp(template.entry)
        .and_then(|values| {
            if values.is_empty() {
                None
            } else {
                let max = u32::try_from(values.len().saturating_sub(1)).unwrap_or(0);
                let index = map.urand_inclusive_like_cpp(0, max) as usize;
                values.get(index).copied()
            }
        });
    if let Some(vehicle_id) = template.vehicle_id {
        if let Some(vehicle_entry) = caches.vehicle_store.get(vehicle_id) {
            template.vehicle_kit_create_input = Some(wow_entities::VehicleKitCreateInputLikeCpp {
                vehicle_id,
                creature_entry: template.entry,
                loading: true,
                seat_defs: caches
                    .vehicle_seat_store
                    .seat_defs_for_vehicle_like_cpp(vehicle_entry),
            });
            template.add_to_world_vehicle_reset_context =
                Some(wow_entities::CreatureAddToWorldVehicleResetContextLikeCpp {
                    is_mechanical_creature: template.creature_type
                        == CREATURE_TYPE_MECHANICAL_LIKE_CPP,
                    is_world_boss: template.type_flags & CREATURE_TYPE_FLAG_BOSS_MOB_LIKE_CPP != 0,
                    accessories: caches
                        .vehicle_accessory_store
                        .accessories_for_vehicle_like_cpp(Some(spawn_id), template.entry)
                        .map(ToOwned::to_owned)
                        .unwrap_or_default(),
                });
        }
    }

    let map_object_high = if template.vehicle_id.is_some() {
        HighGuid::Vehicle
    } else {
        HighGuid::Creature
    };
    let map_object_guid = match map_object_high {
        HighGuid::Vehicle => {
            ObjectGuid::create_vehicle_like_cpp(caches.realm_id, map_id, template.entry, low)
        }
        HighGuid::Creature => {
            ObjectGuid::create_creature_like_cpp(caches.realm_id, map_id, template.entry, low)
        }
        _ => unreachable!("loaded-grid creature records only create Creature or Vehicle GUIDs"),
    };
    let resolver = creature_loaded_grid::CreatureLoadedGridLifecycleResolverLikeCpp::new(
        [template],
        [resolved_spawn],
        [(spawn.id, runtime_selection)],
    );
    match resolver.resolve_loaded_grid_creature_like_cpp(spawn_id, map_object_guid) {
        Ok(resolved) => resolved.map_object_record.map(|primary_record| {
            wow_map::map::LoadedGridRespawnRecordsLikeCpp::primary_only(primary_record)
        }),
        Err(error) => {
            debug!(
                ?error,
                spawn_id,
                entry = spawn.id,
                guid = ?map_object_guid,
                "C++ loaded-grid Creature DoRespawn blocked: resolver rejected loaded Creature record"
            );
            None
        }
    }
}

pub(crate) fn build_loaded_grid_gameobject_respawn_record_like_cpp(
    map: &mut wow_map::Map,
    object_type: wow_map::SpawnObjectType,
    spawn_id: wow_map::SpawnId,
    canonical_spawn_metadata: &spawn_store_loader::CanonicalSpawnMetadataLikeCpp,
    caches: &LoadedGridCreatureRespawnCachesLikeCpp,
) -> Option<wow_map::map::LoadedGridRespawnRecordsLikeCpp> {
    if object_type != wow_map::SpawnObjectType::GameObject {
        return None;
    }

    let Some(spawn) = canonical_spawn_metadata
        .spawn_store()
        .spawn_data(object_type, spawn_id)
    else {
        debug!(
            respawn_type = object_type as u8,
            spawn_id, "C++ loaded-grid GameObject DoRespawn blocked: missing canonical SpawnData"
        );
        return None;
    };
    let Some(runtime_row) = canonical_spawn_metadata.gameobject_runtime_row_like_cpp(spawn_id)
    else {
        debug!(
            spawn_id,
            entry = spawn.id,
            "C++ loaded-grid GameObject DoRespawn blocked: missing DB-backed gameobject runtime row"
        );
        return None;
    };
    // C++ `Map::ProcessRespawns` erases the due map-owned respawn timer before
    // `DoRespawn -> GameObject::LoadFromDB(addToMap=true)`. Therefore
    // `GetMap()->GetGORespawnTime(m_spawnId)` observes no timer and the newly
    // respawned object's effective `m_respawnTime` is 0.
    let inputs = gameobject_loaded_grid::build_loaded_grid_gameobject_inputs_from_db_like_cpp(
        spawn,
        runtime_row,
        caches.gameobject_template_store.as_ref(),
        caches.gameobject_override_store.as_ref(),
        map.instance_id(),
        0,
        true,
    );
    let (template, resolved_spawn) = match inputs {
        Ok(inputs) => inputs,
        Err(error) => {
            debug!(
                ?error,
                spawn_id,
                entry = spawn.id,
                "C++ loaded-grid GameObject DoRespawn blocked: failed to compose DB-backed LoadFromDB inputs"
            );
            return None;
        }
    };

    let map_object_guid = if template.go_type == wow_entities::GAMEOBJECT_TYPE_TRANSPORT {
        let low = match map.generate_low_guid_like_cpp(HighGuid::Transport) {
            Ok(low) => low,
            Err(error) => {
                debug!(
                    ?error,
                    spawn_id,
                    entry = spawn.id,
                    "C++ loaded-grid GameObject DoRespawn blocked: map-owned Transport low-guid generation failed"
                );
                return None;
            }
        };
        ObjectGuid::create_transport(HighGuid::Transport, low)
    } else {
        let Ok(map_id) = u16::try_from(map.map_id()) else {
            warn!(
                map_id = map.map_id(),
                spawn_id,
                entry = spawn.id,
                "C++ loaded-grid GameObject DoRespawn blocked: map id does not fit ObjectGuid world-object map field"
            );
            return None;
        };
        let low = match map.generate_low_guid_like_cpp(HighGuid::GameObject) {
            Ok(low) => low,
            Err(error) => {
                debug!(
                    ?error,
                    spawn_id,
                    entry = spawn.id,
                    "C++ loaded-grid GameObject DoRespawn blocked: map-owned GameObject low-guid generation failed"
                );
                return None;
            }
        };
        ObjectGuid::create_gameobject_like_cpp(map_id, template.entry, low)
    };
    let mut linked_trap_guid = None;
    let mut resolver_templates = vec![template.clone()];
    let linked_entry = wow_entities::GameObjectTemplateData::new(template.go_type, template.data)
        .get_linked_gameobject_entry_like_cpp();
    if linked_entry != 0 && template.go_type != wow_entities::GAMEOBJECT_TYPE_TRANSPORT {
        if let Some(linked_template_record) = caches.gameobject_template_store.get(linked_entry) {
            let linked_template =
                match gameobject_loaded_grid::resolved_template_from_lifecycle_record_like_cpp(
                    linked_template_record,
                    None,
                ) {
                    Ok(linked_template)
                        if linked_template.go_type != wow_entities::GAMEOBJECT_TYPE_TRANSPORT =>
                    {
                        Some(linked_template)
                    }
                    Ok(_) => {
                        debug!(
                            spawn_id,
                            entry = spawn.id,
                            linked_entry,
                            "C++ loaded-grid GameObject linked trap skipped: linked transport template not represented by this seam"
                        );
                        None
                    }
                    Err(error) => {
                        debug!(
                            ?error,
                            spawn_id,
                            entry = spawn.id,
                            linked_entry,
                            "C++ loaded-grid GameObject linked trap skipped: linked template rejected"
                        );
                        None
                    }
                };
            if let Some(linked_template) = linked_template {
                let Ok(map_id) = u16::try_from(map.map_id()) else {
                    warn!(
                        map_id = map.map_id(),
                        spawn_id,
                        entry = spawn.id,
                        linked_entry,
                        "C++ loaded-grid GameObject linked trap skipped: map id does not fit ObjectGuid world-object map field"
                    );
                    let resolver =
                        gameobject_loaded_grid::GameObjectLoadedGridLifecycleResolverLikeCpp::new(
                            resolver_templates,
                            [resolved_spawn],
                        );
                    return match resolver
                        .resolve_loaded_grid_gameobject_like_cpp(spawn_id, map_object_guid)
                    {
                        Ok(resolved) => resolved.map_object_record.map(|primary_record| {
                            wow_map::map::LoadedGridRespawnRecordsLikeCpp {
                                pre_add_records: resolved.pre_add_records,
                                primary_record,
                            }
                        }),
                        Err(error) => {
                            debug!(
                                ?error,
                                spawn_id,
                                entry = spawn.id,
                                guid = ?map_object_guid,
                                "C++ loaded-grid GameObject DoRespawn blocked: resolver rejected loaded GameObject record"
                            );
                            None
                        }
                    };
                };
                let trap_low = match map.generate_low_guid_like_cpp(HighGuid::GameObject) {
                    Ok(low) => Some(low),
                    Err(error) => {
                        debug!(
                            ?error,
                            spawn_id,
                            entry = spawn.id,
                            linked_entry,
                            "C++ loaded-grid GameObject linked trap skipped: map-owned GameObject low-guid generation failed"
                        );
                        None
                    }
                };
                if let Some(trap_low) = trap_low {
                    linked_trap_guid = Some(ObjectGuid::create_gameobject_like_cpp(
                        map_id,
                        linked_entry,
                        trap_low,
                    ));
                    resolver_templates.push(linked_template);
                }
            }
        } else {
            debug!(
                spawn_id,
                entry = spawn.id,
                linked_entry,
                "C++ loaded-grid GameObject linked trap skipped: missing linked trap template"
            );
        }
    }
    let resolver = gameobject_loaded_grid::GameObjectLoadedGridLifecycleResolverLikeCpp::new(
        resolver_templates,
        [resolved_spawn],
    );
    match resolver.resolve_loaded_grid_gameobject_with_linked_trap_like_cpp(
        spawn_id,
        map_object_guid,
        linked_trap_guid,
    ) {
        Ok(resolved) => resolved.map_object_record.map(|primary_record| {
            wow_map::map::LoadedGridRespawnRecordsLikeCpp {
                pre_add_records: resolved.pre_add_records,
                primary_record,
            }
        }),
        Err(error) => {
            debug!(
                ?error,
                spawn_id,
                entry = spawn.id,
                guid = ?map_object_guid,
                "C++ loaded-grid GameObject DoRespawn blocked: resolver rejected loaded GameObject record"
            );
            None
        }
    }
}

#[allow(dead_code)]
pub(crate) fn build_loaded_grid_area_trigger_record_like_cpp(
    map: &mut wow_map::Map,
    object_type: wow_map::SpawnObjectType,
    spawn_id: wow_map::SpawnId,
    canonical_spawn_metadata: &spawn_store_loader::CanonicalSpawnMetadataLikeCpp,
    area_trigger_template_store: &wow_data::AreaTriggerTemplateStore,
) -> Option<wow_map::map::LoadedGridRespawnRecordsLikeCpp> {
    if object_type != wow_map::SpawnObjectType::AreaTrigger {
        return None;
    }

    let Some(spawn) = canonical_spawn_metadata
        .spawn_store()
        .spawn_data(object_type, spawn_id)
    else {
        debug!(
            respawn_type = object_type as u8,
            spawn_id, "C++ loaded-grid AreaTrigger load blocked: missing canonical SpawnData"
        );
        return None;
    };
    let Some(runtime_row) = canonical_spawn_metadata.area_trigger_runtime_row_like_cpp(spawn_id)
    else {
        debug!(
            spawn_id,
            create_properties_id = spawn.id,
            "C++ loaded-grid AreaTrigger load blocked: missing DB-backed area trigger runtime row"
        );
        return None;
    };
    let Some(create_properties) = area_trigger_template_store
        .get_create_properties_like_cpp(runtime_row.create_properties_id)
    else {
        debug!(
            spawn_id,
            create_properties_id = runtime_row.create_properties_id.id,
            "C++ loaded-grid AreaTrigger load blocked: missing create-properties row"
        );
        return None;
    };
    let template = create_properties
        .template_id
        .and_then(|template_id| area_trigger_template_store.get_template_like_cpp(template_id));

    match area_trigger_loaded_grid::build_loaded_grid_area_trigger_record_from_spawn_data_like_cpp(
        map,
        spawn,
        runtime_row,
        create_properties,
        template,
        0,
    ) {
        Ok(records) => Some(records),
        Err(error) => {
            debug!(
                ?error,
                spawn_id,
                create_properties_id = runtime_row.create_properties_id.id,
                "C++ loaded-grid AreaTrigger load blocked: failed to compose DB-backed LoadFromDB record"
            );
            None
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(crate) struct LoadedGridAreaTriggerLoadSummaryLikeCpp {
    pub(crate) maps_evaluated: usize,
    pub(crate) loaded_grids_evaluated: usize,
    pub(crate) grid_not_loaded: usize,
    pub(crate) metadata_entries: usize,
    pub(crate) skipped_already_loaded: usize,
    pub(crate) skipped_should_not_spawn: usize,
    pub(crate) stale_index_entries: usize,
    pub(crate) skipped_difficulty_mismatch: usize,
    pub(crate) load_record_missing: usize,
    pub(crate) pre_add_records_added: usize,
    pub(crate) loaded_grid_primary_records: usize,
    pub(crate) loaded_area_trigger_guids: Vec<ObjectGuid>,
    pub(crate) add_to_map_errors: usize,
}

impl LoadedGridAreaTriggerLoadSummaryLikeCpp {
    pub(crate) fn accumulate(
        &mut self,
        grid: &wow_map::map::LoadedGridAreaTriggerRecordsSummaryLikeCpp,
    ) {
        self.grid_not_loaded += usize::from(grid.grid_not_loaded);
        self.metadata_entries += grid.metadata_entries;
        self.skipped_already_loaded += grid.skipped_already_loaded;
        self.skipped_should_not_spawn += grid.skipped_should_not_spawn;
        self.stale_index_entries += grid.stale_index_entries;
        self.skipped_difficulty_mismatch += grid.skipped_difficulty_mismatch;
        self.load_record_missing += grid.load_record_missing;
        self.pre_add_records_added += grid.pre_add_records_added;
        self.loaded_grid_primary_records += grid.loaded_grid_primary_records.len();
        self.loaded_area_trigger_guids.extend(
            grid.loaded_grid_primary_records
                .iter()
                .map(|record| record.object().guid()),
        );
        self.add_to_map_errors += grid.add_to_map_errors;
    }
}

#[allow(dead_code)]
pub(crate) fn load_loaded_grid_area_triggers_like_cpp(
    manager: &mut wow_map::MapManager,
    canonical_spawn_metadata: &spawn_store_loader::CanonicalSpawnMetadataLikeCpp,
    area_trigger_template_store: &wow_data::AreaTriggerTemplateStore,
) -> LoadedGridAreaTriggerLoadSummaryLikeCpp {
    let mut summary = LoadedGridAreaTriggerLoadSummaryLikeCpp::default();
    manager.do_for_all_maps_mut(|managed_map| {
        summary.maps_evaluated += 1;
        let loaded_grid_coords = managed_map.map().loaded_grid_coords_like_cpp();
        summary.loaded_grids_evaluated += loaded_grid_coords.len();
        for coord in loaded_grid_coords {
            let grid_summary = managed_map
                .map_mut()
                .load_loaded_grid_area_trigger_records_like_cpp(
                    coord,
                    canonical_spawn_metadata.spawn_store(),
                    |map, object_type, spawn_id| {
                        build_loaded_grid_area_trigger_record_like_cpp(
                            map,
                            object_type,
                            spawn_id,
                            canonical_spawn_metadata,
                            area_trigger_template_store,
                        )
                    },
                );
            summary.accumulate(&grid_summary);
        }
    });
    summary
}

pub(crate) fn canonical_map_update_tick_set_inactive_like_cpp(
    manager: &mut wow_map::MapManager,
    legacy_manager: Option<&SharedMapManager>,
    diff_ms: u32,
    scheduler: &mut CanonicalRespawnConditionSchedulerLikeCpp,
    canonical_spawn_metadata: &spawn_store_loader::CanonicalSpawnMetadataLikeCpp,
    condition_store: &wow_data::ConditionEntriesByTypeStore,
    map_store: &wow_data::MapStore,
    loaded_grid_creature_respawn_caches: &LoadedGridCreatureRespawnCachesLikeCpp,
) -> Option<CanonicalSpawnGroupConditionTickSummaryLikeCpp> {
    let Some(effective_diff_ms) = manager.update_with_pool_update_loaded_grid_records_context(
        diff_ms,
        canonical_spawn_metadata.spawn_store(),
        canonical_spawn_metadata.pool_mgr_like_cpp(),
        |map, object_type, spawn_id| match object_type {
            wow_map::SpawnObjectType::GameObject => {
                build_loaded_grid_gameobject_respawn_record_like_cpp(
                    map,
                    object_type,
                    spawn_id,
                    canonical_spawn_metadata,
                    loaded_grid_creature_respawn_caches,
                )
            }
            wow_map::SpawnObjectType::Creature | wow_map::SpawnObjectType::AreaTrigger => None,
        },
    ) else {
        return None;
    };
    let mut summary = CanonicalSpawnGroupConditionTickSummaryLikeCpp::default();
    manager.do_for_all_maps_mut(|managed_map| {
        summary.expired_pvp_combat_refs.extend(
            managed_map
                .last_expired_pvp_combat_refs_like_cpp()
                .iter()
                .map(|(owner, target)| {
                    (
                        managed_map.map_id(),
                        managed_map.instance_id(),
                        *owner,
                        *target,
                    )
                }),
        );
        let map_kind = managed_map.kind();
        let map_id = managed_map.map_id();
        let instance_id = managed_map.instance_id();
        let map_is_instanceable = map_store
            .get(map_id)
            .is_some_and(|entry| entry.is_instanceable_like_cpp());
        for info in managed_map
            .last_game_objects_update_summary()
            .respawn_db_saves
        {
            match queue_respawn_db_save_like_cpp(
                map_kind,
                map_is_instanceable,
                map_id,
                instance_id,
                info,
            ) {
                RespawnDbSaveQueueOutcomeLikeCpp::Queued(save) => {
                    summary.respawn_db_save_queued += 1;
                    summary.respawn_db_saves.push(save);
                }
                RespawnDbSaveQueueOutcomeLikeCpp::SkippedNonWorldMap => {
                    summary.respawn_db_save_skipped_non_world_map += 1;
                }
                RespawnDbSaveQueueOutcomeLikeCpp::SkippedInstanceableMap => {
                    summary.respawn_db_save_skipped_instanceable_map += 1;
                }
                RespawnDbSaveQueueOutcomeLikeCpp::SkippedInvalidMapId => {
                    summary.respawn_db_save_skipped_invalid_map_id += 1;
                }
            }
        }
    });
    if !scheduler.update(effective_diff_ms) {
        return (!summary.respawn_db_saves.is_empty()
            || !summary.expired_pvp_combat_refs.is_empty())
        .then_some(summary);
    }

    // C++ `Map::Update` runs `ProcessRespawns()` immediately before
    // `UpdateSpawnGroupConditions()` when `_respawnCheckTimer` expires.
    // This tick executes the safe in-memory ProcessRespawns side effects produced
    // by represented composite CheckRespawn guards: zero-delete for inactive
    // spawn-group/live-object blockers, linked-respawn future reschedules, pooled
    // timer UpdatePool plans, and the safe `DoRespawn` unloaded-grid early-return
    // branch after timer removal. DB delete/save effects are queued for async
    // execution after releasing the MapManager lock. Loaded-grid Creature
    // DB-backed loading is wired through the map-owned seam for supported
    // fixed-level and variable-level cases, including DB-backed FormationInfo
    // propagation into the bounded SearchFormation/AddCreatureToGroup seam;
    // AddToWorld ObjectAccessor/fanout, scripts/AI, vehicle runtime beyond local
    // evidence, zonescript, formation movement/combat/full CreatureGroup runtime,
    // dynamic-tree, full GameObject physical-removal lifecycle, AreaTrigger
    // runtime and full PoolMgr runtime remain gaps.
    // RustyCore does not yet expose CONFIG_RESPAWN_DYNAMIC_ESCORTNPC
    // or Creature::IsEscorted ownership here, so the bridge passes false/false.
    let now_secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| {
            i64::try_from(duration.as_secs()).unwrap_or(i64::MAX)
        });
    manager.do_for_all_maps_mut(|managed_map| {
        summary.maps_evaluated += 1;
        let map_kind = managed_map.kind();
        let map_id = managed_map.map_id();
        let instance_id = managed_map.instance_id();
        let map_is_instanceable = map_store
            .get(map_id)
            .is_some_and(|entry| entry.is_instanceable_like_cpp());
        let before_respawn_keys = managed_map
            .map()
            .respawn_timer_keys_like_cpp()
            .collect::<BTreeSet<_>>();
        let respawn_summary = managed_map
            .map_mut()
            .process_due_respawns_composite_loaded_grid_respawns_like_cpp(
                now_secs,
                canonical_spawn_metadata.spawn_store(),
                canonical_spawn_metadata.linked_respawns_like_cpp(),
                canonical_spawn_metadata.pool_mgr_like_cpp(),
                5,
                false,
                |_, _| false,
                |_, _| 0.0,
                |_candidates, count| (0..count).collect(),
                true,
                |map, object_type, spawn_id| match object_type {
                    wow_map::SpawnObjectType::Creature => {
                        build_loaded_grid_creature_respawn_record_like_cpp(
                            map,
                            object_type,
                            spawn_id,
                            canonical_spawn_metadata,
                            loaded_grid_creature_respawn_caches,
                        )
                    }
                    wow_map::SpawnObjectType::GameObject => {
                        build_loaded_grid_gameobject_respawn_record_like_cpp(
                            map,
                            object_type,
                            spawn_id,
                            canonical_spawn_metadata,
                            loaded_grid_creature_respawn_caches,
                        )
                    }
                    wow_map::SpawnObjectType::AreaTrigger => None,
                },
            );
        summary.respawn_deleted_inactive_spawn_group +=
            respawn_summary.deleted_inactive_spawn_group;
        summary.respawn_deleted_live_object_blocker += respawn_summary.deleted_live_object_blocker;
        for rescheduled in respawn_summary.rescheduled_linked_respawns {
            match queue_respawn_db_save_like_cpp(
                map_kind,
                map_is_instanceable,
                map_id,
                instance_id,
                rescheduled,
            ) {
                RespawnDbSaveQueueOutcomeLikeCpp::Queued(save) => {
                    summary.respawn_db_save_queued += 1;
                    summary.respawn_db_saves.push(save);
                }
                RespawnDbSaveQueueOutcomeLikeCpp::SkippedNonWorldMap => {
                    summary.respawn_db_save_skipped_non_world_map += 1;
                }
                RespawnDbSaveQueueOutcomeLikeCpp::SkippedInstanceableMap => {
                    summary.respawn_db_save_skipped_instanceable_map += 1;
                }
                RespawnDbSaveQueueOutcomeLikeCpp::SkippedInvalidMapId => {
                    summary.respawn_db_save_skipped_invalid_map_id += 1;
                }
            }
        }
        summary.respawn_processed_pool_timers += respawn_summary.processed_pool_timers;
        summary.respawn_processed_unloaded_grid_respawns +=
            respawn_summary.processed_unloaded_grid_respawns;
        summary.respawn_executed_loaded_grid_respawns +=
            respawn_summary.executed_loaded_grid_respawns;
        summary.respawn_legacy_creature_mirrors +=
            mirror_loaded_grid_primary_records_to_legacy_like_cpp(
                legacy_manager,
                canonical_spawn_metadata.waypoint_paths_like_cpp(),
                &respawn_summary.loaded_grid_primary_records,
            );
        summary.respawn_blocked_loaded_grid_respawn_loads +=
            respawn_summary.blocked_loaded_grid_respawn_loads;
        summary.respawn_blocked_loaded_grid_respawn_add_to_map +=
            respawn_summary.blocked_loaded_grid_respawn_add_to_map;
        summary.respawn_pool_update_plans += respawn_summary.pool_update_plans.len();
        summary.respawn_blocked_pool_plan_errors += respawn_summary.blocked_pool_plan_errors.len();
        summary.respawn_blocked_missing_spawn_data += respawn_summary.blocked_missing_spawn_data;
        summary.respawn_blocked_pool_runtime += respawn_summary.blocked_pool_runtime;
        summary.respawn_blocked_do_respawn_runtime += respawn_summary.blocked_do_respawn_runtime;
        summary.respawn_blocked_linked_respawn_non_future +=
            respawn_summary.blocked_linked_respawn_non_future;
        summary.respawn_blocked_unsupported_spawn_type +=
            respawn_summary.blocked_unsupported_spawn_type;

        let outcomes = apply_canonical_spawn_group_condition_update_loaded_grid_records_like_cpp(
            managed_map,
            canonical_spawn_metadata,
            condition_store,
            loaded_grid_creature_respawn_caches,
        );
        summary.outcomes += outcomes.len();
        summary.applied_set_inactive += outcomes
            .iter()
            .filter(|outcome| outcome.applied_change.is_some())
            .count();
        summary.planned_spawn += outcomes
            .iter()
            .filter(|outcome| {
                matches!(
                    outcome.action,
                    wow_map::map::SpawnGroupConditionActionLikeCpp::Spawn { .. }
                )
            })
            .count();
        summary.planned_despawn += outcomes
            .iter()
            .filter(|outcome| {
                matches!(
                    outcome.action,
                    wow_map::map::SpawnGroupConditionActionLikeCpp::Despawn { .. }
                )
            })
            .count();
        for spawn in outcomes
            .iter()
            .filter_map(|outcome| outcome.spawn_outcome.as_ref())
        {
            summary.condition_spawn_executed_loaded_grid_spawns +=
                spawn.executed_loaded_grid_spawns;
            summary.condition_spawn_legacy_creature_mirrors +=
                mirror_loaded_grid_primary_records_to_legacy_like_cpp(
                    legacy_manager,
                    canonical_spawn_metadata.waypoint_paths_like_cpp(),
                    &spawn.loaded_grid_primary_records,
                );
            summary.condition_spawn_blocked_loaded_grid_spawn_loads +=
                spawn.blocked_loaded_grid_spawn_loads;
            summary.condition_spawn_blocked_loaded_grid_creature_loads +=
                spawn.blocked_loaded_grid_creature_loads;
            summary.condition_spawn_blocked_loaded_grid_gameobject_loads +=
                spawn.blocked_loaded_grid_gameobject_loads;
            summary.condition_spawn_blocked_loaded_grid_spawn_add_to_map +=
                spawn.blocked_loaded_grid_spawn_add_to_map;
            summary.condition_spawn_load_plan_count += spawn.load_plans.len();
            summary.condition_spawn_unsupported_spawn_types += spawn.unsupported_spawn_types;
            summary.condition_spawn_skipped_respawn_timer_active +=
                spawn.skipped_respawn_timer_active;
            summary.condition_spawn_skipped_live_object_active += spawn.skipped_live_object_active;
            summary.condition_spawn_skipped_unloaded_grid += spawn.skipped_unloaded_grid;
            summary.condition_spawn_skipped_difficulty_mismatch +=
                spawn.skipped_difficulty_mismatch;
        }
        for despawn in outcomes
            .iter()
            .filter_map(|outcome| outcome.despawn_outcome)
        {
            if despawn.blocked_missing_group == 0 && despawn.blocked_system_group == 0 {
                summary.despawn_executed += 1;
            }
            summary.despawn_objects_removed += despawn.objects_removed;
            summary.despawn_respawn_timers_removed += despawn.respawn_timers_removed;
            summary.despawn_blocked_missing_group += despawn.blocked_missing_group;
            summary.despawn_blocked_system_group += despawn.blocked_system_group;
            summary.despawn_unsupported_live_types += despawn.unsupported_live_despawn_types;
            summary.despawn_respawn_timer_unsupported_types +=
                despawn.respawn_timer_unsupported_types;
            summary.despawn_stale_index_entries += despawn.stale_index_entries;
            summary.despawn_remove_errors += despawn.remove_errors;
        }
        let after_respawn_keys = managed_map
            .map()
            .respawn_timer_keys_like_cpp()
            .collect::<BTreeSet<_>>();
        for &(object_type, spawn_id) in before_respawn_keys.difference(&after_respawn_keys) {
            match queue_respawn_db_delete_like_cpp(
                map_kind,
                map_is_instanceable,
                map_id,
                instance_id,
                object_type,
                spawn_id,
            ) {
                RespawnDbDeleteQueueOutcomeLikeCpp::Queued(delete) => {
                    summary.respawn_db_delete_queued += 1;
                    summary.respawn_db_deletes.push(delete);
                }
                RespawnDbDeleteQueueOutcomeLikeCpp::SkippedNonWorldMap => {
                    summary.respawn_db_delete_skipped_non_world_map += 1;
                }
                RespawnDbDeleteQueueOutcomeLikeCpp::SkippedInstanceableMap => {
                    summary.respawn_db_delete_skipped_instanceable_map += 1;
                }
                RespawnDbDeleteQueueOutcomeLikeCpp::SkippedInvalidMapId => {
                    summary.respawn_db_delete_skipped_invalid_map_id += 1;
                }
            }
        }
    });

    Some(summary)
}

/// C++ `Group::UpdateReadyCheck` tick: decrements every active group's
/// ready-check timer each `tick_interval_ms` and broadcasts
/// `ReadyCheckCompleted` to connected members when the timer expires.
pub(crate) fn spawn_group_ready_check_tick_loop(
    group_registry: Arc<GroupRegistry>,
    player_registry: Arc<PlayerRegistry>,
    tick_interval_ms: u32,
) -> tokio::task::JoinHandle<()> {
    use wow_packet::ServerPacket;
    use wow_packet::packets::party::{ReadyCheckCompleted, ReadyCheckResponse, ReadyCheckStarted};

    tokio::spawn(async move {
        let mut interval =
            tokio::time::interval(Duration::from_millis(u64::from(tick_interval_ms)));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

        loop {
            interval.tick().await;

            let expired = tick_all_group_ready_checks_like_cpp(&group_registry, tick_interval_ms);

            for (group_guid, events) in expired {
                // Snapshot member txs outside the group lock.
                let recipients = if let Some(group) = group_registry.get(&group_guid) {
                    player_registry.group_presences_in_order(&group.members)
                } else {
                    continue;
                };

                // Drop the DashMap ref before sending.
                for event in &events {
                    let bytes = match *event {
                        ReadyCheckEventLikeCpp::Started {
                            party_index,
                            party_guid,
                            initiator_guid,
                            duration_ms,
                        } => ReadyCheckStarted {
                            party_index,
                            party_guid,
                            initiator_guid,
                            duration_ms,
                        }
                        .to_bytes(),
                        ReadyCheckEventLikeCpp::Response {
                            party_guid,
                            player,
                            is_ready,
                        } => ReadyCheckResponse {
                            party_guid,
                            player,
                            is_ready,
                        }
                        .to_bytes(),
                        ReadyCheckEventLikeCpp::Completed {
                            party_index,
                            party_guid,
                        } => ReadyCheckCompleted {
                            party_index,
                            party_guid,
                        }
                        .to_bytes(),
                    };

                    for recipient in &recipients {
                        let _ = player_registry
                            .send_current_packet(recipient.registration, bytes.clone());
                    }
                }
            }
        }
    })
}

pub(crate) fn spawn_canonical_map_update_loop(
    map_manager: SharedCanonicalMapManager,
    legacy_map_manager: SharedMapManager,
    tick_interval_ms: u32,
    respawn_condition_interval_ms: u32,
    canonical_spawn_metadata: SharedCanonicalSpawnMetadataLikeCpp,
    condition_store: Arc<wow_data::ConditionEntriesByTypeStore>,
    map_store: Arc<wow_data::MapStore>,
    game_event_persistence: Arc<dyn GameEventPersistencePortLikeCpp>,
    respawn_db_writer_tx: RespawnDbWriterSenderLikeCpp,
    respawn_db_mutation_order: SharedRespawnDbMutationOrderLikeCpp,
    respawn_db_producer_stop: SharedRespawnDbProducerStopLikeCpp,
    loaded_grid_creature_respawn_caches: LoadedGridCreatureRespawnCachesLikeCpp,
    area_trigger_template_store: Arc<wow_data::AreaTriggerTemplateStore>,
    mut game_event_scheduler: CanonicalGameEventSchedulerLikeCpp,
    player_registry: Arc<PlayerRegistry>,
    battlemaster_list_store: Arc<wow_data::BattlemasterListStore>,
    world_state_mgr: SharedWorldStateMgrLikeCpp,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut interval =
            tokio::time::interval(Duration::from_millis(u64::from(tick_interval_ms)));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

        let mut last_tick = Instant::now();
        let mut respawn_condition_scheduler =
            CanonicalRespawnConditionSchedulerLikeCpp::new(respawn_condition_interval_ms);
        loop {
            interval.tick().await;
            let stop_after_tick = respawn_db_producer_stop.load(Ordering::Acquire);

            let now = Instant::now();
            let mut diff_ms = now
                .duration_since(last_tick)
                .as_millis()
                .min(u128::from(u32::MAX)) as u32;
            last_tick = now;

            if diff_ms == 0 {
                if !stop_after_tick {
                    continue;
                }
                diff_ms = 1;
            }

            let (area_trigger_sweep_summary, tick_summary) = {
                // Canonical and legacy respawn mutations share this ordering
                // gate. Statements are coalesced before releasing it, so
                // mailbox replacement order is the same as mutation order even
                // when this loop later awaits unrelated game-event DB work.
                let _respawn_db_mutation_order = respawn_db_mutation_order
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                let Ok(mut manager) = map_manager.lock() else {
                    tracing::error!(
                        "Canonical MapManager mutex poisoned; stopping map update loop"
                    );
                    break;
                };
                let Ok(canonical_spawn_metadata) = canonical_spawn_metadata.lock() else {
                    tracing::error!(
                        "CanonicalSpawnMetadataLikeCpp mutex poisoned; stopping map update loop"
                    );
                    break;
                };
                let area_trigger_sweep_summary = load_loaded_grid_area_triggers_like_cpp(
                    &mut manager,
                    &canonical_spawn_metadata,
                    area_trigger_template_store.as_ref(),
                );
                let mut tick_summary = canonical_map_update_tick_set_inactive_like_cpp(
                    &mut manager,
                    Some(&legacy_map_manager),
                    diff_ms,
                    &mut respawn_condition_scheduler,
                    &canonical_spawn_metadata,
                    condition_store.as_ref(),
                    map_store.as_ref(),
                    &loaded_grid_creature_respawn_caches,
                );
                drop(canonical_spawn_metadata);
                drop(manager);

                if let Some(summary) = tick_summary.as_mut() {
                    let mut reconciled_players = BTreeSet::new();
                    for (map_id, instance_id, owner_guid, target_guid) in
                        summary.expired_pvp_combat_refs.drain(..)
                    {
                        let Ok(map_id) = u16::try_from(map_id) else {
                            continue;
                        };
                        for player_guid in [owner_guid, target_guid] {
                            if !reconciled_players.insert((map_id, instance_id, player_guid)) {
                                continue;
                            }
                            let Some(recipient) = player_registry.runtime_recipient(player_guid)
                            else {
                                continue;
                            };
                            let command =
                                wow_world::session::mailbox::ReconcilePvpCombatExpiryLikeCppCommand {
                                    player_guid,
                                    map_id,
                                    instance_id,
                                };
                            player_registry
                                .publish_current_pvp_combat_expiry(recipient.registration, command);
                        }
                    }
                    for save in summary.respawn_db_saves.drain(..) {
                        if respawn_db_writer_tx.send(save.mutation).is_err() {
                            summary.respawn_db_save_failed += 1;
                            tracing::error!(
                                "Shared respawn DB writer stopped before canonical REP_RESPAWN submission"
                            );
                        }
                    }
                    // A timer can be created and consumed in one canonical
                    // update. Submit deletes last so the final state wins.
                    for delete in summary.respawn_db_deletes.drain(..) {
                        if respawn_db_writer_tx.send(delete.mutation).is_err() {
                            summary.respawn_db_delete_failed += 1;
                            tracing::error!(
                                "Shared respawn DB writer stopped before canonical DEL_RESPAWN submission"
                            );
                        }
                    }
                }

                (area_trigger_sweep_summary, tick_summary)
            };

            if area_trigger_sweep_summary.loaded_grid_primary_records > 0
                || area_trigger_sweep_summary.load_record_missing > 0
                || area_trigger_sweep_summary.add_to_map_errors > 0
            {
                debug!(
                    maps_evaluated = area_trigger_sweep_summary.maps_evaluated,
                    loaded_grids_evaluated = area_trigger_sweep_summary.loaded_grids_evaluated,
                    metadata_entries = area_trigger_sweep_summary.metadata_entries,
                    skipped_already_loaded = area_trigger_sweep_summary.skipped_already_loaded,
                    skipped_should_not_spawn = area_trigger_sweep_summary.skipped_should_not_spawn,
                    stale_index_entries = area_trigger_sweep_summary.stale_index_entries,
                    skipped_difficulty_mismatch =
                        area_trigger_sweep_summary.skipped_difficulty_mismatch,
                    load_record_missing = area_trigger_sweep_summary.load_record_missing,
                    loaded_grid_primary_records =
                        area_trigger_sweep_summary.loaded_grid_primary_records,
                    add_to_map_errors = area_trigger_sweep_summary.add_to_map_errors,
                    "C++ ObjectGridLoader AreaTrigger loaded-grid sweep materialized canonical AreaTrigger records for already-loaded grids; ObjectAccessor/fanout/scripts/actions/dynamic-tree runtime remain pending"
                );
            }

            if !stop_after_tick && game_event_scheduler.update(diff_ms) {
                let current_time_secs = current_unix_time_secs_like_cpp();
                let (game_event_outcome, active_event_ids, mut db_bridge_summary) = {
                    let Ok(mut canonical_spawn_metadata) = canonical_spawn_metadata.lock() else {
                        tracing::error!(
                            "CanonicalSpawnMetadataLikeCpp mutex poisoned during GameEvent update; stopping map update loop"
                        );
                        break;
                    };
                    let outcome = canonical_spawn_metadata.update_game_events_like_cpp(
                        current_time_secs,
                        true,
                        represented_game_event_world_conditions_met_like_cpp,
                    );
                    game_event_scheduler.set_interval_and_reset(outcome.next_update_delay_millis);
                    let db_bridge_summary =
                        materialize_game_event_world_event_state_db_bridge_like_cpp(
                            &outcome,
                            &canonical_spawn_metadata,
                        );
                    let active_event_ids = canonical_spawn_metadata
                        .game_event_active_set_like_cpp()
                        .active_event_ids_like_cpp()
                        .collect::<Vec<_>>();
                    (outcome, active_event_ids, db_bridge_summary)
                };
                warn_about_sync_queries_scope_like_cpp(
                    execute_game_event_world_event_state_db_bridge_like_cpp(
                        game_event_persistence.as_ref(),
                        &mut db_bridge_summary,
                    ),
                )
                .await;
                let mut side_effect_summary = {
                    let Ok(mut manager) = map_manager.lock() else {
                        tracing::error!(
                            "Canonical MapManager mutex poisoned during GameEvent side effects; stopping map update loop"
                        );
                        break;
                    };
                    let Ok(mut canonical_spawn_metadata) = canonical_spawn_metadata.lock() else {
                        tracing::error!(
                            "CanonicalSpawnMetadataLikeCpp mutex poisoned during GameEvent side effects; stopping map update loop"
                        );
                        break;
                    };
                    let Ok(mut world_state_mgr) = world_state_mgr.lock() else {
                        tracing::error!(
                            "WorldStateMgrLikeCpp mutex poisoned during GameEvent side effects; stopping map update loop"
                        );
                        break;
                    };
                    consume_game_event_live_update_side_effects_like_cpp(
                        &mut manager,
                        Some(&legacy_map_manager),
                        &mut canonical_spawn_metadata,
                        &loaded_grid_creature_respawn_caches,
                        Some(battlemaster_list_store.as_ref()),
                        Some(&mut world_state_mgr),
                        Some(player_registry.as_ref()),
                        &active_event_ids,
                        &game_event_outcome,
                        false,
                    )
                };
                warn_about_sync_queries_scope_like_cpp(
                    execute_game_event_seasonal_quest_db_deletes_like_cpp(
                        game_event_persistence.as_ref(),
                        &mut side_effect_summary,
                    ),
                )
                .await;
                fanout_reset_event_seasonal_quests_to_player_sessions_after_db_delete_like_cpp(
                    Some(player_registry.as_ref()),
                    &mut side_effect_summary,
                );
                debug!(
                    scanned_event_ids = game_event_outcome.scanned_event_ids.len(),
                    queued_activation_event_ids =
                        game_event_outcome.queued_activation_event_ids.len(),
                    queued_deactivation_event_ids =
                        game_event_outcome.queued_deactivation_event_ids.len(),
                    start_outcomes = game_event_outcome.start_outcomes.len(),
                    stop_outcomes = game_event_outcome.stop_outcomes.len(),
                    negative_spawn_event_ids = game_event_outcome.negative_spawn_event_ids.len(),
                    world_nextphase_finished = game_event_outcome.world_nextphase_finished.len(),
                    world_conditions_save_requested =
                        game_event_outcome.world_conditions_save_requested.len(),
                    game_event_db_saves_queued = db_bridge_summary.saves_queued,
                    game_event_db_saves_executed = db_bridge_summary.saves_executed,
                    game_event_db_saves_failed = db_bridge_summary.saves_failed,
                    game_event_db_saves_skipped_event_id_out_of_range =
                        db_bridge_summary.saves_skipped_event_id_out_of_range,
                    game_event_db_saves_skipped_missing_event =
                        db_bridge_summary.saves_skipped_missing_event,
                    game_event_db_deletes_queued = db_bridge_summary.deletes_queued,
                    game_event_db_deletes_executed = db_bridge_summary.deletes_executed,
                    game_event_db_deletes_failed = db_bridge_summary.deletes_failed,
                    game_event_db_deletes_skipped_event_id_out_of_range =
                        db_bridge_summary.deletes_skipped_event_id_out_of_range,
                    game_event_db_condition_delete_rows_queued =
                        db_bridge_summary.condition_delete_rows_queued,
                    game_event_db_condition_delete_rows_executed =
                        db_bridge_summary.condition_delete_rows_executed,
                    game_event_db_condition_delete_rows_failed =
                        db_bridge_summary.condition_delete_rows_failed,
                    invalid_check_outcomes = game_event_outcome.invalid_check_outcomes.len(),
                    invalid_next_check_outcomes =
                        game_event_outcome.invalid_next_check_outcomes.len(),
                    next_update_delay_millis = game_event_outcome.next_update_delay_millis,
                    side_effect_actions = side_effect_summary.actions.len(),
                    spawn_actions = side_effect_summary.spawn_actions,
                    unspawn_actions = side_effect_summary.unspawn_actions,
                    announce_event_actions = side_effect_summary.announce_event_actions,
                    announce_event_description_len_total =
                        side_effect_summary.announce_event_description_len_total,
                    announce_event_world_text_unimplemented =
                        side_effect_summary.announce_event_world_text_unimplemented,
                    announce_event_session_fanout_unimplemented =
                        side_effect_summary.announce_event_session_fanout_unimplemented,
                    change_equip_or_model_actions =
                        side_effect_summary.change_equip_or_model_actions,
                    change_equip_or_model_records_seen =
                        side_effect_summary.change_equip_or_model_records_seen,
                    change_equip_or_model_records_applied =
                        side_effect_summary.change_equip_or_model_records_applied,
                    change_equip_or_model_maps_matched =
                        side_effect_summary.change_equip_or_model_maps_matched,
                    change_equip_or_model_live_creatures_mutated =
                        side_effect_summary.change_equip_or_model_live_creatures_mutated,
                    change_equip_or_model_model_validation_unavailable =
                        side_effect_summary.change_equip_or_model_model_validation_unavailable,
                    update_event_quests_actions = side_effect_summary.update_event_quests_actions,
                    update_event_quests_creature_records_seen =
                        side_effect_summary.update_event_quests_creature_records_seen,
                    update_event_quests_gameobject_records_seen =
                        side_effect_summary.update_event_quests_gameobject_records_seen,
                    update_event_quests_creature_inserted =
                        side_effect_summary.update_event_quests_creature_inserted,
                    update_event_quests_gameobject_inserted =
                        side_effect_summary.update_event_quests_gameobject_inserted,
                    update_event_quests_creature_removed =
                        side_effect_summary.update_event_quests_creature_removed,
                    update_event_quests_gameobject_removed =
                        side_effect_summary.update_event_quests_gameobject_removed,
                    update_event_quests_creature_remove_misses =
                        side_effect_summary.update_event_quests_creature_remove_misses,
                    update_event_quests_gameobject_remove_misses =
                        side_effect_summary.update_event_quests_gameobject_remove_misses,
                    update_event_quests_creature_skipped_active_other_event =
                        side_effect_summary.update_event_quests_creature_skipped_active_other_event,
                    update_event_quests_gameobject_skipped_active_other_event = side_effect_summary
                        .update_event_quests_gameobject_skipped_active_other_event,
                    update_world_states_actions = side_effect_summary.update_world_states_actions,
                    update_world_states_no_holiday =
                        side_effect_summary.update_world_states_no_holiday,
                    update_world_states_missing_event =
                        side_effect_summary.update_world_states_missing_event,
                    update_world_states_holiday_lookup_unrepresented =
                        side_effect_summary.update_world_states_holiday_lookup_unrepresented,
                    update_npc_flags_actions = side_effect_summary.update_npc_flags_actions,
                    update_npc_flags_records_seen =
                        side_effect_summary.update_npc_flags_records_seen,
                    update_npc_flags_maps_matched =
                        side_effect_summary.update_npc_flags_maps_matched,
                    update_npc_flags_live_creatures_mutated =
                        side_effect_summary.update_npc_flags_live_creatures_mutated,
                    update_npc_flags2_applied =
                        side_effect_summary.update_npc_flags2_applied,
                    update_npc_vendor_actions = side_effect_summary.update_npc_vendor_actions,
                    update_npc_vendor_records_seen =
                        side_effect_summary.update_npc_vendor_records_seen,
                    update_npc_vendor_items_added =
                        side_effect_summary.update_npc_vendor_items_added,
                    update_npc_vendor_items_removed =
                        side_effect_summary.update_npc_vendor_items_removed,
                    update_npc_vendor_missing_event_buckets =
                        side_effect_summary.update_npc_vendor_missing_event_buckets,
                    update_npc_vendor_remove_misses =
                        side_effect_summary.update_npc_vendor_remove_misses,
                    update_npc_vendor_no_match = side_effect_summary.update_npc_vendor_no_match,
                    reset_event_seasonal_quests_actions =
                        side_effect_summary.reset_event_seasonal_quests_actions,
                    reset_event_seasonal_quests_event_start_time_zero =
                        side_effect_summary.reset_event_seasonal_quests_event_start_time_zero,
                    reset_event_seasonal_quests_event_start_time_nonzero =
                        side_effect_summary.reset_event_seasonal_quests_event_start_time_nonzero,
                    reset_event_seasonal_quests_player_session_runtime_unimplemented =
                        side_effect_summary
                            .reset_event_seasonal_quests_player_session_runtime_unimplemented,
                    reset_event_seasonal_quests_character_db_statement_unimplemented =
                        side_effect_summary
                            .reset_event_seasonal_quests_character_db_statement_unimplemented,
                    reset_event_seasonal_quests_character_db_delete_queued = side_effect_summary
                        .reset_event_seasonal_quests_character_db_delete_queued,
                    reset_event_seasonal_quests_character_db_delete_executed = side_effect_summary
                        .reset_event_seasonal_quests_character_db_delete_executed,
                    reset_event_seasonal_quests_character_db_delete_failed = side_effect_summary
                        .reset_event_seasonal_quests_character_db_delete_failed,
                    reset_event_seasonal_quests_character_db_delete_skipped_event_start_time_out_of_range = side_effect_summary
                        .reset_event_seasonal_quests_character_db_delete_skipped_event_start_time_out_of_range,
                    "C++ WUPDATE_EVENTS represented timer fired; updated canonical GameEvent metadata and consumed represented GameEventSpawn/GameEventUnspawn plus bounded ChangeEquipOrModel, UpdateEventQuests cache, represented UpdateWorldStates HolidayWorldState -> WorldStateMgr::SetValue evidence, UpdateEventNPCFlags, UpdateEventNPCVendor cache, RunSmartAIScripts evidence, ResetEventSeasonalQuests character DB delete bridge, and represented announcement evidence-only side effects; ConditionMgr world-event rows, real SendWorldText/session fanout, quest packets/session gossip refresh, full ObjectMgr quest runtime, real WorldStateMgr storage/session fanout/login/GM worldstate, SmartAI script dispatch, and Player/session seasonal quest reset remain pending"
                );
            }

            if let Some(summary) = tick_summary {
                debug!(
                    maps_evaluated = summary.maps_evaluated,
                    outcomes = summary.outcomes,
                    applied_set_inactive = summary.applied_set_inactive,
                    planned_spawn = summary.planned_spawn,
                    condition_spawn_executed_loaded_grid_spawns =
                        summary.condition_spawn_executed_loaded_grid_spawns,
                    condition_spawn_blocked_loaded_grid_spawn_loads =
                        summary.condition_spawn_blocked_loaded_grid_spawn_loads,
                    condition_spawn_blocked_loaded_grid_creature_loads =
                        summary.condition_spawn_blocked_loaded_grid_creature_loads,
                    condition_spawn_blocked_loaded_grid_gameobject_loads =
                        summary.condition_spawn_blocked_loaded_grid_gameobject_loads,
                    condition_spawn_blocked_loaded_grid_spawn_add_to_map =
                        summary.condition_spawn_blocked_loaded_grid_spawn_add_to_map,
                    condition_spawn_load_plan_count = summary.condition_spawn_load_plan_count,
                    condition_spawn_unsupported_spawn_types =
                        summary.condition_spawn_unsupported_spawn_types,
                    condition_spawn_skipped_respawn_timer_active =
                        summary.condition_spawn_skipped_respawn_timer_active,
                    condition_spawn_skipped_live_object_active =
                        summary.condition_spawn_skipped_live_object_active,
                    condition_spawn_skipped_unloaded_grid =
                        summary.condition_spawn_skipped_unloaded_grid,
                    condition_spawn_skipped_difficulty_mismatch =
                        summary.condition_spawn_skipped_difficulty_mismatch,
                    planned_despawn = summary.planned_despawn,
                    despawn_executed = summary.despawn_executed,
                    despawn_objects_removed = summary.despawn_objects_removed,
                    despawn_respawn_timers_removed = summary.despawn_respawn_timers_removed,
                    despawn_blocked_missing_group = summary.despawn_blocked_missing_group,
                    despawn_blocked_system_group = summary.despawn_blocked_system_group,
                    despawn_unsupported_live_types = summary.despawn_unsupported_live_types,
                    despawn_respawn_timer_unsupported_types =
                        summary.despawn_respawn_timer_unsupported_types,
                    despawn_stale_index_entries = summary.despawn_stale_index_entries,
                    despawn_remove_errors = summary.despawn_remove_errors,
                    respawn_deleted_inactive_spawn_group =
                        summary.respawn_deleted_inactive_spawn_group,
                    respawn_deleted_live_object_blocker =
                        summary.respawn_deleted_live_object_blocker,
                    respawn_processed_pool_timers = summary.respawn_processed_pool_timers,
                    respawn_processed_unloaded_grid_respawns =
                        summary.respawn_processed_unloaded_grid_respawns,
                    respawn_executed_loaded_grid_respawns =
                        summary.respawn_executed_loaded_grid_respawns,
                    respawn_blocked_loaded_grid_respawn_loads =
                        summary.respawn_blocked_loaded_grid_respawn_loads,
                    respawn_blocked_loaded_grid_respawn_add_to_map =
                        summary.respawn_blocked_loaded_grid_respawn_add_to_map,
                    respawn_pool_update_plans = summary.respawn_pool_update_plans,
                    respawn_blocked_pool_plan_errors = summary.respawn_blocked_pool_plan_errors,
                    respawn_blocked_missing_spawn_data = summary.respawn_blocked_missing_spawn_data,
                    respawn_blocked_pool_runtime = summary.respawn_blocked_pool_runtime,
                    respawn_blocked_do_respawn_runtime = summary.respawn_blocked_do_respawn_runtime,
                    respawn_blocked_linked_respawn_non_future =
                        summary.respawn_blocked_linked_respawn_non_future,
                    respawn_blocked_unsupported_spawn_type =
                        summary.respawn_blocked_unsupported_spawn_type,
                    respawn_db_delete_queued = summary.respawn_db_delete_queued,
                    respawn_db_delete_executed = summary.respawn_db_delete_executed,
                    respawn_db_delete_failed = summary.respawn_db_delete_failed,
                    respawn_db_delete_skipped_non_world_map =
                        summary.respawn_db_delete_skipped_non_world_map,
                    respawn_db_delete_skipped_instanceable_map =
                        summary.respawn_db_delete_skipped_instanceable_map,
                    respawn_db_delete_skipped_invalid_map_id =
                        summary.respawn_db_delete_skipped_invalid_map_id,
                    respawn_db_save_queued = summary.respawn_db_save_queued,
                    respawn_db_save_executed = summary.respawn_db_save_executed,
                    respawn_db_save_failed = summary.respawn_db_save_failed,
                    respawn_db_save_skipped_non_world_map =
                        summary.respawn_db_save_skipped_non_world_map,
                    respawn_db_save_skipped_instanceable_map =
                        summary.respawn_db_save_skipped_instanceable_map,
                    respawn_db_save_skipped_invalid_map_id =
                        summary.respawn_db_save_skipped_invalid_map_id,
                    "C++ respawn-check timer fired; executed safe ProcessRespawns composite zero-delete branches plus linked future reschedules, represented pooled timer UpdatePool plans, safe DoRespawn unloaded-grid early-return timer removals, map-local SpawnGroupDespawn condition-failure side effects, and bounded loaded-grid SpawnGroupSpawn condition loads; submitted DEL_RESPAWN/REP_RESPAWN side effects to the shared async DB writer outside the MapManager lock; full SpawnGroupSpawn AreaTrigger/ObjectAccessor/fanout/scripts/AI and Spawn1Object/ReSpawn1Object runtime remain pending"
                );
            }

            if stop_after_tick {
                break;
            }
        }
    })
}

#[cfg(test)]
mod represented_group_startup_tests {
    use super::*;
    use wow_persistence::{
        PersistenceFutureLikeCpp, RepresentedGroupStartupLoadOutcomeLikeCpp,
        RepresentedGroupStartupLoadPortLikeCpp, RepresentedGroupStartupLoadStageLikeCpp,
    };

    struct FixedGroupStartupPortLikeCpp(RepresentedGroupStartupLoadOutcomeLikeCpp);

    impl RepresentedGroupStartupLoadPortLikeCpp for FixedGroupStartupPortLikeCpp {
        fn load_represented_groups_like_cpp(
            &self,
        ) -> PersistenceFutureLikeCpp<'_, RepresentedGroupStartupLoadOutcomeLikeCpp> {
            let outcome = self.0.clone();
            Box::pin(async move { outcome })
        }
    }

    #[tokio::test]
    async fn typed_empty_group_startup_rows_materialize_an_empty_registry() {
        let port =
            FixedGroupStartupPortLikeCpp(RepresentedGroupStartupLoadOutcomeLikeCpp::Loaded {
                characters: Vec::new(),
                groups: Vec::new(),
                members: Vec::new(),
            });
        let registry = GroupRegistry::new();
        let summary = load_groups_from_character_database_like_cpp(
            &port,
            &registry,
            &wow_data::DifficultyStore::from_ids([]),
        )
        .await
        .unwrap();
        assert_eq!(summary, GroupLoadSummaryLikeCpp::default());
    }

    #[tokio::test]
    async fn typed_group_startup_failure_never_materializes_partial_state() {
        let port =
            FixedGroupStartupPortLikeCpp(RepresentedGroupStartupLoadOutcomeLikeCpp::Failed {
                stage: RepresentedGroupStartupLoadStageLikeCpp::Groups,
                reason: "query failed".to_owned(),
            });
        let registry = GroupRegistry::new();
        let error = load_groups_from_character_database_like_cpp(
            &port,
            &registry,
            &wow_data::DifficultyStore::from_ids([]),
        )
        .await
        .unwrap_err();
        assert!(error.to_string().contains("Groups: query failed"));
        assert!(registry.snapshots().is_empty());
    }
}
