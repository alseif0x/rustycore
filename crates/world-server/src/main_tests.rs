//! Behaviour tests for [`super`].
//!
//! Extracted from `main.rs`. Moving tests moves no invariant: the
//! production module boundary, its visibility and its owners are untouched.
//!
//! Dedenting by one level lets rustfmt collapse some argument lists onto a single
//! line, which drops their trailing commas; that is the only difference from the
//! original text.
#![cfg(test)]

use super::{
    ActiveWorldSessionRegistrationGuardLikeCpp, ActiveWorldSessionRegistryLikeCpp,
    KickAllSessionsSummaryLikeCpp, StopWorldNetworkSummaryLikeCpp,
    UpdateSessionsShutdownFlushSummaryLikeCpp,
};
use super::{
    CanonicalGameEventSchedulerLikeCpp, CanonicalRespawnConditionSchedulerLikeCpp,
    ERROR_EXIT_CODE_LIKE_CPP, FreezeDetectorLikeCpp, FreezeDetectorPollOutcomeLikeCpp,
    GameEventLiveUpdateActionLikeCpp, GameEventLiveUpdateSideEffectSummaryLikeCpp,
    GameEventWorldEventStateDbOperationKindLikeCpp, GameEventWorldEventStateDbOperationLikeCpp,
    ITEM_GUID_DANGLING_REFERENCE_CLEANUP_STATEMENTS_LIKE_CPP,
    LoadedGridCreatureRespawnCachesLikeCpp, PersistedRespawnLoadReportLikeCpp,
    PersistedRespawnTimesLikeCpp, REQUIRED_TDB_CACHE_ID_LIKE_CPP, REQUIRED_TDB_VERSION_LIKE_CPP,
    RESTART_EXIT_CODE_LIKE_CPP, RespawnDbDeleteQueueOutcomeLikeCpp, RespawnDbMailboxLikeCpp,
    RespawnDbRetryQueueLikeCpp, RespawnDbSaveQueueOutcomeLikeCpp, RespawnDbSubmitErrorLikeCpp,
    RespawnDbWriterSenderLikeCpp, SHUTDOWN_EXIT_CODE_LIKE_CPP, WorldDbVersionLikeCpp,
    WorldRuntimeStateLikeCpp, WorldServerCliLikeCpp, WorldUpdateLoopStepOutcomeLikeCpp,
    apply_canonical_creature_attack_starts_like_cpp,
    apply_canonical_creature_attack_stops_like_cpp,
    apply_canonical_spawn_group_condition_update_loaded_grid_records_like_cpp,
    build_loaded_grid_area_trigger_record_like_cpp,
    build_loaded_grid_creature_respawn_record_like_cpp,
    build_loaded_grid_creature_spawn_group_spawn_record_like_cpp,
    build_loaded_grid_gameobject_respawn_record_like_cpp, build_tap_group_index_like_cpp,
    canonical_map_update_tick_set_inactive_like_cpp, clear_online_accounts_sql_like_cpp,
    collect_legacy_creature_aggro_candidates_like_cpp,
    collect_legacy_creature_aggro_candidates_with_canonical_like_cpp,
    consume_game_event_live_update_side_effects_like_cpp, create_pid_file_like_cpp,
    database_pool_size_like_cpp, db_keepalive_database_names_like_cpp,
    db_keepalive_interval_minutes_like_cpp, db_keepalive_sql_like_cpp,
    declined_names_used_for_realm_category_like_cpp,
    deliver_creature_attack_start_commands_like_cpp,
    deliver_creature_melee_damage_commands_like_cpp,
    deliver_refresh_visible_world_creatures_like_cpp, deliver_runtime_plan_like_cpp,
    execute_game_event_world_event_state_db_bridge_like_cpp, execute_respawn_db_attempt_like_cpp,
    fanout_game_event_announcement_to_player_sessions_like_cpp,
    fanout_realm_update_world_state_to_player_sessions_like_cpp,
    fanout_reset_event_seasonal_quests_to_player_sessions_after_db_delete_like_cpp,
    game_event_announcement_lines_like_cpp, game_event_change_equip_or_model_like_cpp,
    game_event_live_update_actions_like_cpp,
    game_event_quest_complete_response_from_summary_like_cpp,
    game_event_spawn_creatures_and_gameobjects_for_event_like_cpp,
    game_event_spawn_for_event_like_cpp, game_event_spawn_pools_for_event_like_cpp,
    game_event_spawn_pools_like_cpp,
    game_event_unspawn_creatures_and_gameobjects_for_event_like_cpp,
    game_event_unspawn_for_event_like_cpp, game_event_unspawn_pools_for_event_like_cpp,
    game_event_unspawn_pools_like_cpp, game_event_update_npc_flags_like_cpp,
    game_event_update_npc_vendor_like_cpp, game_event_update_world_states_like_cpp,
    get_address_for_client_with_local_networks, half_max_core_stuck_time_like_cpp,
    install_canonical_spawn_group_initializer_like_cpp, is_ffa_pvp_realm_type_like_cpp,
    is_pvp_realm_type_like_cpp, kick_all_sessions_like_cpp, legacy_creature_aggro_config_like_cpp,
    legacy_creature_global_runtime_enabled_from_config_like_cpp,
    load_loaded_grid_area_triggers_like_cpp, load_world_config_from, loot_drop_rates_like_cpp,
    loot_quest_required_from_signed_db_like_cpp,
    materialize_game_event_quest_complete_db_bridge_like_cpp,
    materialize_game_event_world_event_state_db_bridge_like_cpp, max_core_stuck_time_ms_like_cpp,
    max_core_stuck_time_secs_like_cpp, max_primary_trade_skills_like_cpp,
    min_world_update_time_ms_like_cpp, mmap_runtime_config_like_cpp,
    next_equipment_set_guid_allocator_start_like_cpp, next_item_guid_allocator_start_like_cpp,
    next_void_storage_item_id_allocator_start_like_cpp, normalize_realm_security_level_like_cpp,
    normalize_realm_type_like_cpp, normalized_realm_name_like_cpp,
    persisted_respawn_info_from_row_like_cpp, player_regeneration_rates_like_cpp,
    process_exit_code_like_cpp, queue_respawn_db_delete_like_cpp, queue_respawn_db_save_like_cpp,
    realm_id_like_cpp, realm_list_entry_from_row_like_cpp, repair_cost_rate_like_cpp,
    reputation_rates_like_cpp, reset_schedule_like_cpp, respawn_db_retry_delay,
    retain_committed_creature_combat_events_like_cpp,
    run_legacy_creature_lifecycle_tick_and_refresh_once_like_cpp,
    run_legacy_creature_melee_tick_and_deliver_once_like_cpp,
    run_legacy_creature_movement_tick_and_deliver_once_like_cpp,
    run_legacy_creature_runtime_tick_and_deliver_once_like_cpp,
    run_world_session_shutdown_finalize_step_like_cpp, set_realm_offline_sql_like_cpp,
    set_realm_online_sql_like_cpp, spawn_legacy_creature_runtime_update_loop_like_cpp,
    spawn_store_loader, stop_world_network_like_cpp, update_sessions_shutdown_flush_once_like_cpp,
    world_config_bool, world_config_f32, world_config_u8, world_config_u16, world_config_u32,
    world_db_version_matches_required_like_cpp, world_db_version_mismatch_message_like_cpp,
    world_update_loop_step_like_cpp, worldserver_cli_help_like_cpp,
    worldserver_full_version_like_cpp, worldserver_revision_like_cpp,
};
use std::collections::{BTreeMap, HashSet};
use std::env;
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use wow_constants::{ConditionSourceType, ConditionType, ServerOpcodes};
use wow_core::{ObjectGuid, ObjectGuidGenerator, Position, guid::HighGuid};
use wow_data::{Condition, ConditionEntriesByTypeStore};
use wow_database::StatementDef;
use wow_entities::{Creature, GameObject, MapObjectRecord, Player};
use wow_instances::ResetSchedule;
use wow_map::{
    LinkedRespawnStoreLikeCpp, PoolGroupLikeCpp, PoolMemberKindLikeCpp, PoolMgrLikeCpp,
    PoolObjectLikeCpp, PoolTemplateDataLikeCpp, RespawnInfoLikeCpp, SpawnData, SpawnGroupFlags,
    SpawnGroupTemplateData, SpawnObjectType, SpawnPosition, SpawnStore, spawn::SpawnGroupMemberRow,
};
use wow_packet::{
    ServerPacket,
    packets::chat::{ChatMsg, ChatPkt},
};
use wow_persistence::{
    GameEventConditionSaveLoadOutcomeLikeCpp, GameEventPersistenceMutationLikeCpp,
    GameEventPersistenceMutationOutcomeLikeCpp, GameEventPersistencePortLikeCpp,
    RespawnPersistenceKeyLikeCpp, RespawnPersistenceLoadOutcomeLikeCpp,
    RespawnPersistenceMutationLikeCpp, RespawnPersistenceMutationOutcomeLikeCpp,
    RespawnPersistencePortLikeCpp, RespawnPersistenceRowLikeCpp,
};

// Keep fixture helpers available to the existing scenario modules through this root.
#[path = "main_tests/loaded_grid_fixtures.rs"]
mod loaded_grid_fixtures;
use loaded_grid_fixtures::{
    area_trigger_template_store_for_loaded_grid_like_cpp, canonical_test_map_store_like_cpp,
    creature_base_stats_record_like_cpp, empty_loaded_grid_creature_respawn_caches_like_cpp,
    loaded_grid_map_store_like_cpp, mapid_condition, test_spawn, test_spawn_metadata,
    test_spawn_metadata_with_explicit_spawn_ids, test_spawn_metadata_with_flags,
    variable_loaded_grid_creature_respawn_caches_like_cpp,
    variable_loaded_grid_creature_respawn_caches_with_vehicle_id_and_difficulty_like_cpp,
    variable_loaded_grid_creature_respawn_caches_with_vehicle_id_like_cpp,
    vehicle_seat_store_for_loaded_grid_test, vehicle_store_for_loaded_grid_test,
};

#[path = "main_tests/game_event_fixtures.rs"]
mod game_event_fixtures;
use game_event_fixtures::{
    add_spawn_data_like_cpp, assert_game_event_save_operation_like_cpp,
    canonical_spawn_metadata_with_pool_mgr_like_cpp,
    canonical_spawn_metadata_with_store_and_game_event_guids_like_cpp,
    canonical_spawn_metadata_with_store_and_pool_mgr_like_cpp,
    canonical_spawn_metadata_with_store_pool_mgr_and_game_event_pools_like_cpp,
    empty_game_event_update_outcome_for_db_bridge_like_cpp,
    game_event_live_update_npc_vendor_metadata_like_cpp,
    game_event_live_update_npc_vendor_record_like_cpp, game_event_npc_flag_template_store_like_cpp,
    game_event_quest_complete_progressed_outcome_like_cpp, game_event_spawn_test_caches_like_cpp,
    game_event_spawn_test_spawn_data_like_cpp, game_event_world_state_metadata_like_cpp,
    game_event_world_state_start_outcome_like_cpp, insert_live_creature_for_spawn_like_cpp,
    insert_live_gameobject_for_spawn_like_cpp, live_npc_flags_like_cpp, live_npc_flags2_like_cpp,
    pool_mgr_with_creature_pool_like_cpp, push_game_event_guid_for_test_like_cpp,
    spawn_data_like_cpp, test_guid_like_cpp,
};

#[path = "main_tests/respawn_persistence_fixtures.rs"]
mod respawn_persistence_fixtures;
use respawn_persistence_fixtures::{
    assert_del_respawn_params_like_cpp, assert_rep_respawn_params_like_cpp,
    linked_respawn_guid_like_cpp, respawn_db_delete_mutation_fixture_like_cpp,
    respawn_db_save_mutation_fixture_like_cpp, respawn_info_like_cpp,
    respawn_persistence_key_fixture_like_cpp,
};

#[path = "main_tests/runtime_delivery_fixtures.rs"]
mod runtime_delivery_fixtures;
use runtime_delivery_fixtures::{
    add_canonical_test_creature_on_map_like_cpp, add_canonical_test_player_on_map_like_cpp,
    drain_durable_creature_runtime_commands_like_cpp, insert_player_registration_fixture_like_cpp,
    insert_player_registration_fixture_with_in_world_like_cpp,
    legacy_runtime_world_map_store_like_cpp, make_creature_spell_runtime_plan_like_cpp,
    make_nearby_visible_event_like_cpp, make_registry_player_like_cpp, make_source_guid,
    mirror_canonical_melee_test_creature_like_cpp, player_registration_fixture_like_cpp,
    unique_temp_dir,
};

#[derive(Default)]
struct FakeGameEventPersistencePortLikeCpp {
    fail_mutations: std::sync::atomic::AtomicBool,
    mutations: Mutex<Vec<GameEventPersistenceMutationLikeCpp>>,
}

impl GameEventPersistencePortLikeCpp for FakeGameEventPersistencePortLikeCpp {
    fn load_condition_saves_like_cpp<'a>(
        &'a self,
    ) -> wow_persistence::PersistenceFutureLikeCpp<'a, GameEventConditionSaveLoadOutcomeLikeCpp>
    {
        Box::pin(async { GameEventConditionSaveLoadOutcomeLikeCpp::Loaded(Vec::new()) })
    }

    fn execute_mutation_like_cpp<'a>(
        &'a self,
        mutation: GameEventPersistenceMutationLikeCpp,
    ) -> wow_persistence::PersistenceFutureLikeCpp<'a, GameEventPersistenceMutationOutcomeLikeCpp>
    {
        Box::pin(async move {
            self.mutations.lock().unwrap().push(mutation);
            if self
                .fail_mutations
                .load(std::sync::atomic::Ordering::Acquire)
            {
                GameEventPersistenceMutationOutcomeLikeCpp::Failed {
                    reason: "fixture failure".to_string(),
                }
            } else {
                GameEventPersistenceMutationOutcomeLikeCpp::Applied
            }
        })
    }
}
use wow_world::session::directory::{
    PlayerDirectoryIdentityLikeCpp, PlayerDirectoryPlacementLikeCpp, PlayerRegistry,
    PlayerSessionRegistrationLikeCpp,
};
use wow_world::session::mailbox::{SessionCommand, WorldSessionShutdownFlushResultLikeCpp};

#[derive(Default)]
struct FakeRespawnPersistencePortLikeCpp {
    fail_mutations: std::sync::atomic::AtomicBool,
    mutations: Mutex<Vec<RespawnPersistenceMutationLikeCpp>>,
}

impl RespawnPersistencePortLikeCpp for FakeRespawnPersistencePortLikeCpp {
    fn load_for_map_like_cpp<'a>(
        &'a self,
        _map_id: u16,
        _instance_id: u32,
    ) -> wow_persistence::PersistenceFutureLikeCpp<'a, RespawnPersistenceLoadOutcomeLikeCpp> {
        Box::pin(async { RespawnPersistenceLoadOutcomeLikeCpp::Loaded(Vec::new()) })
    }

    fn load_all_like_cpp<'a>(
        &'a self,
    ) -> wow_persistence::PersistenceFutureLikeCpp<'a, RespawnPersistenceLoadOutcomeLikeCpp> {
        Box::pin(async { RespawnPersistenceLoadOutcomeLikeCpp::Loaded(Vec::new()) })
    }

    fn execute_mutation_like_cpp<'a>(
        &'a self,
        mutation: RespawnPersistenceMutationLikeCpp,
    ) -> wow_persistence::PersistenceFutureLikeCpp<'a, RespawnPersistenceMutationOutcomeLikeCpp>
    {
        Box::pin(async move {
            self.mutations.lock().unwrap().push(mutation);
            if self
                .fail_mutations
                .load(std::sync::atomic::Ordering::Acquire)
            {
                RespawnPersistenceMutationOutcomeLikeCpp::Failed {
                    reason: "fixture failure".to_string(),
                }
            } else {
                RespawnPersistenceMutationOutcomeLikeCpp::Applied { affected_rows: 1 }
            }
        })
    }
}

static TEST_LOCK: Mutex<()> = Mutex::new(());

#[path = "main_tests/scenarios_1.rs"]
mod scenarios_1;
#[path = "main_tests/scenarios_10.rs"]
mod scenarios_10;
#[path = "main_tests/scenarios_11.rs"]
mod scenarios_11;
#[path = "main_tests/scenarios_12.rs"]
mod scenarios_12;
#[path = "main_tests/scenarios_13.rs"]
mod scenarios_13;
#[path = "main_tests/scenarios_14.rs"]
mod scenarios_14;
#[path = "main_tests/scenarios_2.rs"]
mod scenarios_2;
#[path = "main_tests/scenarios_3.rs"]
mod scenarios_3;
#[path = "main_tests/scenarios_4.rs"]
mod scenarios_4;
#[path = "main_tests/scenarios_5.rs"]
mod scenarios_5;
#[path = "main_tests/scenarios_6.rs"]
mod scenarios_6;
#[path = "main_tests/scenarios_7.rs"]
mod scenarios_7;
#[path = "main_tests/scenarios_8.rs"]
mod scenarios_8;
#[path = "main_tests/scenarios_9.rs"]
mod scenarios_9;
