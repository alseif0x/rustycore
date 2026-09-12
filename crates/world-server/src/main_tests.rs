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
    persisted_respawn_info_from_row_like_cpp, process_exit_code_like_cpp,
    queue_respawn_db_delete_like_cpp, queue_respawn_db_save_like_cpp, realm_id_like_cpp,
    realm_list_entry_from_row_like_cpp, repair_cost_rate_like_cpp, reputation_rates_like_cpp,
    reset_schedule_like_cpp, respawn_db_retry_delay,
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

fn legacy_runtime_world_map_store_like_cpp() -> wow_data::MapStore {
    wow_data::MapStore::from_entries([wow_data::MapEntry {
        id: 0,
        instance_type: wow_data::map::MAP_COMMON,
        expansion_id: 0,
        parent_map_id: -1,
        cosmetic_parent_map_id: -1,
        flags1: 0,
        flags2: 0,
    }])
}

fn canonical_test_map_store_like_cpp() -> wow_data::MapStore {
    wow_data::MapStore::from_entries([0, 530, 571, 999].map(|id| wow_data::MapEntry {
        id,
        instance_type: wow_data::map::MAP_COMMON,
        expansion_id: 0,
        parent_map_id: -1,
        cosmetic_parent_map_id: -1,
        flags1: 0,
        flags2: 0,
    }))
}

fn player_registration_fixture_like_cpp(
    send_tx: flume::Sender<Vec<u8>>,
    command_tx: flume::Sender<SessionCommand>,
    player_name: &str,
) -> PlayerSessionRegistrationLikeCpp {
    PlayerSessionRegistrationLikeCpp {
        identity: PlayerDirectoryIdentityLikeCpp {
            player_name: player_name.to_string(),
            account_id: 1,
            recruiter_id: 0,
            race: 1,
            class: 1,
            sex: 0,
            active_expansion: 2,
        },
        placement: PlayerDirectoryPlacementLikeCpp {
            map_id: 0,
            instance_id: 0,
            position: wow_core::Position::ZERO,
            is_in_world: true,
            level: 1,
            is_alive: true,
        },
        active_loot_rolls: Vec::new(),
        realm_send_tx: send_tx.clone(),
        send_tx,
        command_tx,
        session_phase_tx: wow_world::session::directory::detached_session_phase_rail_like_cpp(),
        durable_creature_runtime_commands_like_cpp: Default::default(),
        client_visible_guids_like_cpp: Default::default(),
        advanced_combat_logging_enabled_like_cpp: Default::default(),
        visibility_refresh_pending_like_cpp: Default::default(),
    }
}

fn drain_durable_creature_runtime_commands_like_cpp(
    registry: &PlayerRegistry,
    player_guid: ObjectGuid,
) -> Vec<SessionCommand> {
    let durable = registry
        .fixture_durable_creature_runtime_commands_like_cpp(player_guid)
        .expect("registered player");
    durable
        .lock()
        .expect("durable creature-runtime command lock")
        .drain_like_cpp()
}

fn insert_player_registration_fixture_with_in_world_like_cpp(
    registry: &PlayerRegistry,
    counter: u64,
    send_tx: flume::Sender<Vec<u8>>,
    command_tx: flume::Sender<SessionCommand>,
    is_in_world: bool,
) {
    let mut info =
        player_registration_fixture_like_cpp(send_tx, command_tx, &format!("Player{counter}"));
    info.placement.is_in_world = is_in_world;
    registry.register_or_replace(
        ObjectGuid::create_player(1, counter as i64),
        info,
        Default::default(),
    );
}

fn insert_player_registration_fixture_like_cpp(
    registry: &PlayerRegistry,
    counter: u64,
    send_tx: flume::Sender<Vec<u8>>,
    command_tx: flume::Sender<SessionCommand>,
) {
    insert_player_registration_fixture_with_in_world_like_cpp(
        registry, counter, send_tx, command_tx, true,
    );
}

fn assert_del_respawn_params_like_cpp(
    mutation: &RespawnPersistenceMutationLikeCpp,
    object_type: u16,
    spawn_id: u64,
    map_id: u16,
    instance_id: u32,
) {
    let RespawnPersistenceMutationLikeCpp::Delete { key } = mutation else {
        panic!("expected typed DEL_RESPAWN mutation, got {mutation:?}");
    };
    assert_eq!(key.object_type_raw, object_type);
    assert_eq!(key.spawn_id, spawn_id);
    assert_eq!(key.map_id, map_id);
    assert_eq!(key.instance_id, instance_id);
}

fn assert_rep_respawn_params_like_cpp(
    mutation: &RespawnPersistenceMutationLikeCpp,
    object_type: u16,
    spawn_id: u64,
    respawn_time: i64,
    map_id: u16,
    instance_id: u32,
) {
    let RespawnPersistenceMutationLikeCpp::Save {
        key,
        respawn_time: actual_respawn_time,
    } = mutation
    else {
        panic!("expected typed REP_RESPAWN mutation, got {mutation:?}");
    };
    assert_eq!(key.object_type_raw, object_type);
    assert_eq!(key.spawn_id, spawn_id);
    assert_eq!(*actual_respawn_time, respawn_time);
    assert_eq!(key.map_id, map_id);
    assert_eq!(key.instance_id, instance_id);
}

fn respawn_persistence_key_fixture_like_cpp(spawn_id: u64) -> RespawnPersistenceKeyLikeCpp {
    RespawnPersistenceKeyLikeCpp {
        object_type_raw: 0,
        spawn_id,
        map_id: 571,
        instance_id: 0,
    }
}

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

fn game_event_quest_complete_progressed_outcome_like_cpp(
    save_world_event_state_requested: bool,
    force_game_event_update_requested: bool,
) -> spawn_store_loader::GameEventQuestCompleteOutcomeLikeCpp {
    spawn_store_loader::GameEventQuestCompleteOutcomeLikeCpp::Progress(
        spawn_store_loader::GameEventConditionProgressOutcomeLikeCpp::Progressed(
            spawn_store_loader::GameEventConditionProgressSummaryLikeCpp {
                event_id: 7,
                condition_id: 44,
                done_before: 2.5,
                done_after: 5.25,
                req_num: 10.0,
                persistence_event_id: 7,
                completed_event: save_world_event_state_requested,
                check_outcome: spawn_store_loader::GameEventConditionCheckOutcomeLikeCpp::Completed(
                    spawn_store_loader::GameEventConditionCheckSummaryLikeCpp {
                        event_id: 7,
                        condition_count: 1,
                        state_before_raw: 2,
                        state_after_raw: 3,
                        next_start_before: 0,
                        next_start_after: 1_234,
                    },
                ),
                save_world_event_state_requested,
                force_game_event_update_requested,
            },
        ),
    )
}

fn linked_respawn_guid_like_cpp(
    high: wow_core::guid::HighGuid,
    entry: u32,
    spawn_id: u64,
) -> wow_core::ObjectGuid {
    wow_core::ObjectGuid::create_world_object(high, 0, 0, 571, 0, entry, spawn_id as i64)
}

fn empty_loaded_grid_creature_respawn_caches_like_cpp() -> LoadedGridCreatureRespawnCachesLikeCpp {
    LoadedGridCreatureRespawnCachesLikeCpp {
        realm_id: 1,
        template_store: Arc::new(wow_data::CreatureTemplateLifecycleStoreLikeCpp::default()),
        sparring_store: Arc::new(wow_data::CreatureTemplateSparringStoreLikeCpp::default()),
        difficulty_store: Arc::new(wow_data::CreatureDifficultyStoreLikeCpp::default()),
        base_stats_store: Arc::new(wow_data::CreatureBaseStatsStoreLikeCpp::default()),
        chr_classes_store: Arc::new(
            wow_data::character_progression::ChrClassesStore::from_entries([]),
        ),
        power_type_store: Arc::new(
            wow_data::character_progression::PowerTypeStore::from_entries([]),
        ),
        health_rates: wow_data::CreatureClassificationHealthRatesLikeCpp::default(),
        display_store: Arc::new(wow_data::CreatureDisplayInfoStore::from_entries([])),
        model_store: Arc::new(wow_data::CreatureModelDataStore::from_entries([])),
        model_info_store: Arc::new(wow_data::CreatureModelInfoStoreLikeCpp::from_entries([])),
        creature_equipment_store: Arc::new(wow_data::CreatureEquipmentStoreLikeCpp::default()),
        creature_addon_store: Arc::new(wow_data::CreatureAddonStoreLikeCpp::default()),
        vehicle_store: Arc::new(wow_data::VehicleStore::from_entries([])),
        vehicle_seat_store: Arc::new(wow_data::VehicleSeatStore::from_entries([])),
        vehicle_accessory_store: Arc::new(wow_data::VehicleAccessoryStoreLikeCpp::from_parts(
            [],
            [],
        )),
        gameobject_template_store: Arc::new(
            wow_data::GameObjectTemplateLifecycleStoreLikeCpp::default(),
        ),
        gameobject_override_store: Arc::new(
            wow_data::GameObjectOverrideLifecycleStoreLikeCpp::default(),
        ),
    }
}

fn loaded_grid_map_store_like_cpp(map_id: u32, instance_type: i8) -> wow_data::MapStore {
    wow_data::MapStore::from_entries([wow_data::MapEntry {
        id: map_id,
        instance_type,
        expansion_id: 0,
        parent_map_id: -1,
        cosmetic_parent_map_id: -1,
        flags1: 0,
        flags2: 0,
    }])
}

fn area_trigger_template_store_for_loaded_grid_like_cpp(
    create_properties_id: u32,
    template_id: u32,
) -> wow_data::AreaTriggerTemplateStore {
    let map_store = wow_data::MapStore::from_entries([wow_data::MapEntry {
        id: 571,
        instance_type: 0,
        expansion_id: 0,
        parent_map_id: -1,
        cosmetic_parent_map_id: -1,
        flags1: 0,
        flags2: 0,
    }]);
    let world_safe_locs = wow_data::WorldSafeLocStore::from_rows_like_cpp([], &map_store).0;
    let mut shape_data =
        [0.0; wow_data::area_trigger_template::MAX_AREATRIGGER_ENTITY_DATA_LIKE_CPP];
    shape_data[0] = 4.0;
    shape_data[1] = 7.0;

    wow_data::AreaTriggerTemplateStore::from_rows_like_cpp(
        [wow_data::AreaTriggerTemplateRowLikeCpp {
            id: template_id,
            is_custom: false,
            flags: wow_data::area_trigger_template::AREATRIGGER_FLAG_IS_SERVER_SIDE_LIKE_CPP,
        }],
        [],
        [],
        [],
        [wow_data::AreaTriggerCreatePropertiesRowLikeCpp {
            id: create_properties_id,
            is_custom: false,
            area_trigger_id: template_id,
            is_areatrigger_custom: false,
            flags:
                wow_data::area_trigger_template::AREATRIGGER_CREATE_PROPERTIES_FLAG_UNK3_LIKE_CPP,
            move_curve_id: 0,
            scale_curve_id: 0,
            morph_curve_id: 0,
            facing_curve_id: 0,
            anim_id: 11,
            anim_kit_id: 22,
            decal_properties_id: 77,
            time_to_target: 0,
            time_to_target_scale: 0,
            shape: wow_data::area_trigger_template::AREATRIGGER_SHAPE_SPHERE_LIKE_CPP,
            shape_data,
            script_name: String::new(),
        }],
        [],
        &world_safe_locs,
        |_| true,
        |_| wow_data::ScriptIdLikeCpp(0),
    )
    .store
}

static TEST_LOCK: Mutex<()> = Mutex::new(());

fn respawn_info_like_cpp(
    object_type: SpawnObjectType,
    spawn_id: wow_map::SpawnId,
    respawn_time: i64,
) -> RespawnInfoLikeCpp {
    RespawnInfoLikeCpp {
        object_type,
        spawn_id,
        entry: 42,
        respawn_time,
        grid_id: 7,
    }
}

fn canonical_spawn_metadata_with_pool_mgr_like_cpp(
    pool_mgr: PoolMgrLikeCpp,
) -> spawn_store_loader::CanonicalSpawnMetadataLikeCpp {
    spawn_store_loader::CanonicalSpawnMetadataLikeCpp::new(SpawnStore::new(), BTreeMap::new())
        .with_pool_mgr_like_cpp(pool_mgr)
}

fn canonical_spawn_metadata_with_store_and_pool_mgr_like_cpp(
    spawn_store: SpawnStore,
    pool_mgr: PoolMgrLikeCpp,
) -> spawn_store_loader::CanonicalSpawnMetadataLikeCpp {
    spawn_store_loader::CanonicalSpawnMetadataLikeCpp::new(spawn_store, BTreeMap::new())
        .with_pool_mgr_like_cpp(pool_mgr)
}

fn canonical_spawn_metadata_with_store_pool_mgr_and_game_event_pools_like_cpp(
    spawn_store: SpawnStore,
    pool_mgr: PoolMgrLikeCpp,
    game_event_pools: spawn_store_loader::GameEventPoolIdsLikeCpp,
) -> spawn_store_loader::CanonicalSpawnMetadataLikeCpp {
    spawn_store_loader::CanonicalSpawnMetadataLikeCpp::new(spawn_store, BTreeMap::new())
        .with_pool_mgr_like_cpp(pool_mgr)
        .with_game_event_pools_like_cpp(game_event_pools)
}

fn pool_mgr_with_creature_pool_like_cpp(
    pool_id: u32,
    map_id: i32,
    spawn_id: wow_map::SpawnId,
) -> PoolMgrLikeCpp {
    let mut pool_mgr = PoolMgrLikeCpp::new();
    pool_mgr.insert_template_like_cpp(pool_id, PoolTemplateDataLikeCpp::new(1, map_id));
    let mut group = PoolGroupLikeCpp::with_pool_id(PoolMemberKindLikeCpp::Creature, pool_id);
    group.add_entry_like_cpp(PoolObjectLikeCpp::new(spawn_id, 0.0), 1);
    pool_mgr
        .insert_or_replace_group_like_cpp(PoolMemberKindLikeCpp::Creature, pool_id, group)
        .expect("test creature pool group");
    pool_mgr
}

fn spawn_data_like_cpp(
    object_type: SpawnObjectType,
    spawn_id: wow_map::SpawnId,
    map_id: u32,
) -> SpawnData {
    SpawnData {
        object_type,
        spawn_id,
        map_id,
        db_data: true,
        spawn_group: SpawnGroupTemplateData {
            group_id: 534,
            name: "game-event-object-guid-unspawn".to_string(),
            map_id,
            flags: SpawnGroupFlags::NONE,
        },
        id: 99,
        spawn_point: SpawnPosition::new(1_000.0, 1_000.0, 0.0, 0.0),
        phase_use_flags: 0,
        phase_id: 0,
        phase_group: 0,
        terrain_swap_map: 0,
        pool_id: 0,
        spawn_time_secs: 0,
        spawn_difficulties: vec![1],
        script_id: 0,
        string_id: String::new(),
    }
}

fn add_spawn_data_like_cpp(
    store: &mut SpawnStore,
    object_type: SpawnObjectType,
    spawn_id: wow_map::SpawnId,
    map_id: u32,
) {
    store.add_object_spawn(&spawn_data_like_cpp(object_type, spawn_id, map_id), |_| {
        false
    });
}

fn game_event_npc_flag_template_store_like_cpp() -> wow_data::CreatureTemplateLifecycleStoreLikeCpp
{
    wow_data::CreatureTemplateLifecycleStoreLikeCpp::from_templates([
        wow_data::CreatureTemplateLifecycleRecordLikeCpp {
            entry: 99,
            name: "Game Event NPC Flag Template".to_string(),
            ai_name: String::new(),
            script_name: String::new(),
            required_expansion: 2,
            faction: 35,
            npc_flags: 0x80,
            speed_walk: 1.0,
            speed_run: 1.14286,
            scale: 1.0,
            classification: 0,
            damage_school: wow_constants::spell::SpellSchools::Normal as u8,
            unit_flags: 0,
            unit_flags2: 0,
            unit_flags3: 0,
            creature_type: 0,
            family: 0,
            trainer_class: 0,
            unit_class: 1,
            vehicle_id: 0,
            movement_type: 0,
            ground_movement_type: wow_constants::CreatureGroundMovementType::Run as u8,
            swim_allowed: true,
            flight_movement_type: 0,
            rooted: false,
            chase_movement_type: wow_constants::CreatureChaseMovementType::Run as u8,
            random_movement_type: wow_constants::CreatureRandomMovementType::Walk as u8,
            interaction_pause_timer_ms:
                wow_entities::DEFAULT_CREATURE_INTERACTION_PAUSE_TIMER_MS_LIKE_CPP,
            flags_extra: wow_constants::creature::CreatureFlagsExtra::WORLDEVENT.bits(),
            string_id: String::new(),
            regen_health: true,
            spells: [0; wow_data::MAX_CREATURE_SPELLS_LIKE_CPP],
            models: Vec::new(),
        },
    ])
}

fn game_event_spawn_test_spawn_data_like_cpp(
    object_type: SpawnObjectType,
    spawn_id: wow_map::SpawnId,
    map_id: u32,
    entry: u32,
    x: f32,
    y: f32,
    spawn_time_secs: i32,
) -> SpawnData {
    SpawnData {
        object_type,
        spawn_id,
        map_id,
        db_data: true,
        spawn_group: SpawnGroupTemplateData {
            group_id: 535,
            name: "game-event-object-guid-spawn".to_string(),
            map_id,
            flags: SpawnGroupFlags::NONE,
        },
        id: entry,
        spawn_point: SpawnPosition::new(x, y, 0.0, 0.0),
        phase_use_flags: 0,
        phase_id: 0,
        phase_group: 0,
        terrain_swap_map: 0,
        pool_id: 0,
        spawn_time_secs,
        spawn_difficulties: vec![0],
        script_id: 0,
        string_id: String::new(),
    }
}

fn game_event_spawn_test_caches_like_cpp(
    creature_entry: u32,
    gameobject_entry: u32,
) -> LoadedGridCreatureRespawnCachesLikeCpp {
    let mut caches =
        variable_loaded_grid_creature_respawn_caches_with_vehicle_id_and_difficulty_like_cpp(
            creature_entry,
            0,
            0,
        );
    let mut data = [0; wow_entities::MAX_GAMEOBJECT_DATA];
    data[11] = 1;
    caches.gameobject_template_store = Arc::new(
        wow_data::GameObjectTemplateLifecycleStoreLikeCpp::from_templates([
            wow_data::GameObjectTemplateLifecycleRecordLikeCpp {
                entry: gameobject_entry,
                go_type: wow_entities::GAMEOBJECT_TYPE_GOOBER,
                display_id: 44,
                name: "GameEventSpawn GO".to_string(),
                size: 1.0,
                data,
                content_tuning_id: 0,
                ai_name: String::new(),
                script_name: String::new(),
                string_id: String::new(),
                addon: None,
            },
        ]),
    );
    caches
}

fn canonical_spawn_metadata_with_store_and_game_event_guids_like_cpp(
    spawn_store: SpawnStore,
    game_event_guids: spawn_store_loader::GameEventSpawnGuidsLikeCpp,
) -> spawn_store_loader::CanonicalSpawnMetadataLikeCpp {
    spawn_store_loader::CanonicalSpawnMetadataLikeCpp::new(spawn_store, BTreeMap::new())
        .with_game_event_spawn_guids_like_cpp(game_event_guids)
}

fn push_game_event_guid_for_test_like_cpp(
    mut guids: spawn_store_loader::GameEventSpawnGuidsLikeCpp,
    object_type: SpawnObjectType,
    event_id: i16,
    spawn_id: wow_map::SpawnId,
) -> spawn_store_loader::GameEventSpawnGuidsLikeCpp {
    assert!(
        guids.push_guid_like_cpp(object_type, event_id, spawn_id),
        "test event id/type must fit C++ GameEvent creature/gameobject GUID range"
    );
    guids
}

fn test_guid_like_cpp(high: HighGuid, counter: i64, entry: u32) -> ObjectGuid {
    match high {
        HighGuid::Creature => ObjectGuid::create_creature_like_cpp(1, 1, entry, counter),
        HighGuid::Vehicle => ObjectGuid::create_vehicle_like_cpp(1, 1, entry, counter),
        HighGuid::GameObject => ObjectGuid::create_gameobject_like_cpp(1, entry, counter),
        HighGuid::AreaTrigger => ObjectGuid::create_area_trigger_like_cpp(1, entry, counter),
        _ => ObjectGuid::create_world_object(high, 0, 0, 1, 0, entry, counter),
    }
}

fn insert_live_creature_for_spawn_like_cpp(
    manager: &mut wow_map::MapManager,
    map_id: u32,
    spawn_id: wow_map::SpawnId,
    counter: i64,
) {
    let mut creature = Creature::new(false);
    creature
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(test_guid_like_cpp(HighGuid::Creature, counter, 99));
    creature.unit_mut().world_mut().set_map(map_id, 0).unwrap();
    creature
        .unit_mut()
        .world_mut()
        .relocate(Position::xyz(1_000.0, 1_000.0, 0.0));
    creature.unit_mut().world_mut().object_mut().add_to_world();
    creature.set_spawn_id(spawn_id);
    manager
        .find_map_mut(map_id, 0)
        .expect("test map")
        .map_mut()
        .add_map_object_record_to_map_like_cpp(MapObjectRecord::new_creature(creature).unwrap())
        .expect("test creature add to map");
}

fn insert_live_gameobject_for_spawn_like_cpp(
    manager: &mut wow_map::MapManager,
    map_id: u32,
    spawn_id: wow_map::SpawnId,
    counter: i64,
) {
    let mut gameobject = GameObject::new();
    gameobject
        .world_mut()
        .object_mut()
        .create(test_guid_like_cpp(HighGuid::GameObject, counter, 99));
    gameobject.world_mut().set_map(map_id, 0).unwrap();
    gameobject
        .world_mut()
        .relocate(Position::xyz(1_000.0, 1_000.0, 0.0));
    gameobject.world_mut().object_mut().add_to_world();
    gameobject.set_spawn_id(spawn_id);
    manager
        .find_map_mut(map_id, 0)
        .expect("test map")
        .map_mut()
        .add_map_object_record_to_map_like_cpp(
            MapObjectRecord::new_game_object(gameobject).unwrap(),
        )
        .expect("test gameobject add to map");
}

fn game_event_world_state_metadata_like_cpp(
    max_event_entry: u32,
    events: &[spawn_store_loader::GameEventDataLikeCpp],
) -> spawn_store_loader::CanonicalSpawnMetadataLikeCpp {
    let store = events.iter().cloned().fold(
        spawn_store_loader::GameEventDataStoreLikeCpp::from_game_event_max_entry_like_cpp(Some(
            max_event_entry,
        )),
        spawn_store_loader::GameEventDataStoreLikeCpp::with_event_like_cpp,
    );
    spawn_store_loader::CanonicalSpawnMetadataLikeCpp::new(SpawnStore::new(), BTreeMap::new())
        .with_game_events_like_cpp(store)
}

fn game_event_world_state_start_outcome_like_cpp(
    event_id: u16,
) -> spawn_store_loader::GameEventUpdateOutcomeLikeCpp {
    spawn_store_loader::GameEventUpdateOutcomeLikeCpp {
        current_time_secs: 650,
        scanned_event_ids: vec![],
        check_outcomes: vec![],
        next_check_outcomes: vec![],
        queued_activation_event_ids: vec![event_id],
        queued_deactivation_event_ids: vec![],
        start_outcomes: vec![spawn_store_loader::GameEventStartOutcomeLikeCpp::Started(
            spawn_store_loader::GameEventStartSummaryLikeCpp {
                event_id,
                state_before_raw: 0,
                state_after_raw: 0,
                active_added: true,
                active_was_present: false,
                apply_new_event_requested: true,
                save_world_event_state_requested: false,
                force_game_event_update_requested: false,
                completed: false,
            },
        )],
        stop_outcomes: vec![],
        negative_spawn_event_ids: vec![],
        world_nextphase_finished: vec![],
        world_conditions_save_requested: vec![],
        invalid_check_outcomes: vec![],
        invalid_next_check_outcomes: vec![],
        next_event_delay_secs_before_padding: 0,
        next_update_delay_millis: 1_000,
    }
}

fn empty_game_event_update_outcome_for_db_bridge_like_cpp()
-> spawn_store_loader::GameEventUpdateOutcomeLikeCpp {
    spawn_store_loader::GameEventUpdateOutcomeLikeCpp {
        current_time_secs: 650,
        scanned_event_ids: vec![],
        check_outcomes: vec![],
        next_check_outcomes: vec![],
        queued_activation_event_ids: vec![],
        queued_deactivation_event_ids: vec![],
        start_outcomes: vec![],
        stop_outcomes: vec![],
        negative_spawn_event_ids: vec![],
        world_nextphase_finished: vec![],
        world_conditions_save_requested: vec![],
        invalid_check_outcomes: vec![],
        invalid_next_check_outcomes: vec![],
        next_event_delay_secs_before_padding: 0,
        next_update_delay_millis: 1_000,
    }
}

fn assert_game_event_save_operation_like_cpp(
    operation: &GameEventWorldEventStateDbOperationLikeCpp,
    event_id: u8,
    state: u8,
    next_start: i64,
) {
    assert_eq!(operation.event_id, event_id);
    assert_eq!(
        operation.kind,
        GameEventWorldEventStateDbOperationKindLikeCpp::Save
    );
    assert_eq!(
        operation.mutation,
        wow_persistence::GameEventPersistenceMutationLikeCpp::SaveWorldEventState {
            event_id,
            state,
            next_start,
        }
    );
}

fn game_event_live_update_npc_vendor_record_like_cpp(
    spawn_id: wow_map::SpawnId,
    entry: u32,
    item: u32,
    vendor_type: u8,
) -> spawn_store_loader::GameEventNpcVendorRecordLikeCpp {
    spawn_store_loader::GameEventNpcVendorRecordLikeCpp {
        spawn_id,
        guid: spawn_id,
        entry,
        item,
        maxcount: 0,
        incrtime: 0,
        extended_cost: 0,
        vendor_type,
        item_type: vendor_type,
        bonus_list_ids: Vec::new(),
        player_condition_id: 0,
        ignore_filtering: false,
        event_npc_flag_low32: 0,
    }
}

fn game_event_live_update_npc_vendor_metadata_like_cpp(
    max_event_entry: u32,
    records: &[(u16, wow_map::SpawnId, u32, u32, u8)],
) -> spawn_store_loader::CanonicalSpawnMetadataLikeCpp {
    let mut vendors =
        spawn_store_loader::GameEventNpcVendorsLikeCpp::from_game_event_max_entry_like_cpp(Some(
            max_event_entry,
        ));
    for (event_id, spawn_id, entry, item, vendor_type) in records {
        assert!(vendors.push_record_like_cpp(
            *event_id,
            game_event_live_update_npc_vendor_record_like_cpp(
                *spawn_id,
                *entry,
                *item,
                *vendor_type,
            ),
        ));
    }
    spawn_store_loader::CanonicalSpawnMetadataLikeCpp::new(SpawnStore::new(), BTreeMap::new())
        .with_game_event_npc_vendors_like_cpp(vendors)
}

fn live_npc_flags_like_cpp(
    manager: &wow_map::MapManager,
    map_id: u32,
    spawn_id: wow_map::SpawnId,
) -> u32 {
    manager
        .find_map(map_id, 0)
        .expect("test map")
        .map()
        .get_creature_by_spawn_id_like_cpp(spawn_id)
        .expect("test live creature")
        .ai_ownership()
        .npc_flags
}

fn live_npc_flags2_like_cpp(
    manager: &wow_map::MapManager,
    map_id: u32,
    spawn_id: wow_map::SpawnId,
) -> u32 {
    manager
        .find_map(map_id, 0)
        .expect("test map")
        .map()
        .get_creature_by_spawn_id_like_cpp(spawn_id)
        .expect("test live creature")
        .ai_ownership()
        .npc_flags2
}

fn respawn_db_save_mutation_fixture_like_cpp(
    spawn_id: u64,
    respawn_time: i64,
) -> RespawnPersistenceMutationLikeCpp {
    let RespawnDbSaveQueueOutcomeLikeCpp::Queued(save) = queue_respawn_db_save_like_cpp(
        wow_map::ManagedMapKind::World,
        false,
        571,
        0,
        RespawnInfoLikeCpp {
            object_type: SpawnObjectType::Creature,
            spawn_id,
            entry: 42,
            respawn_time,
            grid_id: 7,
        },
    ) else {
        panic!("world-map fixture must queue REP_RESPAWN");
    };
    save.mutation
}

fn respawn_db_delete_mutation_fixture_like_cpp(spawn_id: u64) -> RespawnPersistenceMutationLikeCpp {
    let RespawnDbDeleteQueueOutcomeLikeCpp::Queued(delete) = queue_respawn_db_delete_like_cpp(
        wow_map::ManagedMapKind::World,
        false,
        571,
        0,
        SpawnObjectType::Creature,
        spawn_id,
    ) else {
        panic!("world-map fixture must queue DEL_RESPAWN");
    };
    delete.mutation
}

fn variable_loaded_grid_creature_respawn_caches_like_cpp(
    entry: u32,
) -> LoadedGridCreatureRespawnCachesLikeCpp {
    variable_loaded_grid_creature_respawn_caches_with_vehicle_id_like_cpp(entry, 0)
}

fn variable_loaded_grid_creature_respawn_caches_with_vehicle_id_like_cpp(
    entry: u32,
    vehicle_id: u32,
) -> LoadedGridCreatureRespawnCachesLikeCpp {
    variable_loaded_grid_creature_respawn_caches_with_vehicle_id_and_difficulty_like_cpp(
        entry, vehicle_id, 2,
    )
}

fn variable_loaded_grid_creature_respawn_caches_with_vehicle_id_and_difficulty_like_cpp(
    entry: u32,
    vehicle_id: u32,
    difficulty_id: u8,
) -> LoadedGridCreatureRespawnCachesLikeCpp {
    LoadedGridCreatureRespawnCachesLikeCpp {
        realm_id: 1,
        template_store: Arc::new(
            wow_data::CreatureTemplateLifecycleStoreLikeCpp::from_templates([
                wow_data::CreatureTemplateLifecycleRecordLikeCpp {
                    entry,
                    name: "Variable Level Live Creature".to_string(),
                    ai_name: String::new(),
                    script_name: String::new(),
                    required_expansion: 2,
                    faction: 35,
                    npc_flags: 0,
                    speed_walk: 1.0,
                    speed_run: 1.14286,
                    scale: 1.0,
                    classification: 0,
                    damage_school: wow_constants::spell::SpellSchools::Normal as u8,
                    unit_flags: 0,
                    unit_flags2: 0,
                    unit_flags3: 0,
                    creature_type: 0,
                    family: 0,
                    trainer_class: 0,
                    unit_class: 1,
                    vehicle_id,
                    movement_type: 1,
                    ground_movement_type: wow_constants::CreatureGroundMovementType::Run as u8,
                    swim_allowed: true,
                    flight_movement_type: 0,
                    rooted: false,
                    chase_movement_type: wow_constants::CreatureChaseMovementType::Run as u8,
                    random_movement_type: wow_constants::CreatureRandomMovementType::Walk as u8,
                    interaction_pause_timer_ms:
                        wow_entities::DEFAULT_CREATURE_INTERACTION_PAUSE_TIMER_MS_LIKE_CPP,
                    flags_extra: 0,
                    string_id: String::new(),
                    regen_health: true,
                    spells: [0; 8],
                    models: vec![wow_data::CreatureTemplateLifecycleModelLikeCpp {
                        creature_display_id: 111,
                        display_scale: 1.0,
                        probability: 100.0,
                    }],
                },
            ]),
        ),
        sparring_store: Arc::new(wow_data::CreatureTemplateSparringStoreLikeCpp::default()),
        difficulty_store: Arc::new(wow_data::CreatureDifficultyStoreLikeCpp::from_records(
            [wow_data::CreatureDifficultyRecordLikeCpp {
                entry,
                difficulty_id,
                min_level: 18,
                max_level: 20,
                health_scaling_expansion: -1,
                health_modifier: 2.0,
                mana_modifier: 1.0,
                armor_modifier: 1.0,
                damage_modifier: 1.0,
                creature_difficulty_id: 0,
                type_flags: 0,
                type_flags2: 0,
                loot_id: 0,
                pickpocket_loot_id: 0,
                skin_loot_id: 0,
                gold_min: 0,
                gold_max: 0,
                static_flags: [0; 8],
            }],
            |_| 1.0,
        )),
        base_stats_store: Arc::new(wow_data::CreatureBaseStatsStoreLikeCpp::from_records([
            (18, 1, creature_base_stats_record_like_cpp(180)),
            (19, 1, creature_base_stats_record_like_cpp(190)),
            (20, 1, creature_base_stats_record_like_cpp(200)),
        ])),
        chr_classes_store: Arc::new(
            wow_data::character_progression::ChrClassesStore::from_entries([]),
        ),
        power_type_store: Arc::new(
            wow_data::character_progression::PowerTypeStore::from_entries([]),
        ),
        health_rates: wow_data::CreatureClassificationHealthRatesLikeCpp::default(),
        display_store: Arc::new(wow_data::CreatureDisplayInfoStore::from_entries([])),
        model_store: Arc::new(wow_data::CreatureModelDataStore::from_entries([])),
        model_info_store: Arc::new(wow_data::CreatureModelInfoStoreLikeCpp::from_entries([
            wow_data::CreatureModelInfoLikeCpp {
                display_id: 111,
                bounding_radius: 0.0,
                combat_reach: 1.5,
                display_id_other_gender: 0,
                is_trigger: false,
            },
            wow_data::CreatureModelInfoLikeCpp {
                display_id: 999,
                bounding_radius: 0.0,
                combat_reach: 1.5,
                display_id_other_gender: 0,
                is_trigger: false,
            },
        ])),
        creature_equipment_store: Arc::new(wow_data::CreatureEquipmentStoreLikeCpp::default()),
        creature_addon_store: Arc::new(wow_data::CreatureAddonStoreLikeCpp::default()),
        vehicle_store: Arc::new(vehicle_store_for_loaded_grid_test(vehicle_id)),
        vehicle_seat_store: Arc::new(vehicle_seat_store_for_loaded_grid_test()),
        vehicle_accessory_store: Arc::new(wow_data::VehicleAccessoryStoreLikeCpp::from_parts(
            [],
            [],
        )),
        gameobject_template_store: Arc::new(
            wow_data::GameObjectTemplateLifecycleStoreLikeCpp::default(),
        ),
        gameobject_override_store: Arc::new(
            wow_data::GameObjectOverrideLifecycleStoreLikeCpp::default(),
        ),
    }
}

fn vehicle_store_for_loaded_grid_test(vehicle_id: u32) -> wow_data::VehicleStore {
    if vehicle_id == 0 {
        return wow_data::VehicleStore::from_entries([]);
    }
    let mut seat_ids = [0u16; 8];
    seat_ids[0] = 700;
    seat_ids[2] = 701;
    wow_data::VehicleStore::from_entries([wow_data::VehicleEntry {
        id: vehicle_id,
        flags: 0,
        flags_b: 0,
        seat_ids,
    }])
}

fn vehicle_seat_store_for_loaded_grid_test() -> wow_data::VehicleSeatStore {
    wow_data::VehicleSeatStore::from_entries([
        wow_data::VehicleSeatEntry {
            id: 700,
            attachment_offset_x: 0.0,
            attachment_offset_y: 0.0,
            attachment_offset_z: 0.0,
            flags: wow_data::VEHICLE_SEAT_FLAG_CAN_ENTER_OR_EXIT,
            flags_b: 0,
            flags_c: 0,
        },
        wow_data::VehicleSeatEntry {
            id: 701,
            attachment_offset_x: 0.0,
            attachment_offset_y: 0.0,
            attachment_offset_z: 0.0,
            flags: 0,
            flags_b: 0,
            flags_c: 0,
        },
    ])
}

fn creature_base_stats_record_like_cpp(
    base_health: u32,
) -> wow_data::CreatureBaseStatsRecordLikeCpp {
    wow_data::CreatureBaseStatsRecordLikeCpp {
        base_health: [base_health / 4, base_health / 2, base_health],
        base_mana: 50,
        base_armor: 0,
        attack_power: 0,
        ranged_attack_power: 0,
        base_damage: [1.0, 2.0, 3.0],
    }
}

fn test_spawn_metadata<const N: usize>(
    groups: [(u32, u32); N],
) -> super::spawn_store_loader::CanonicalSpawnMetadataLikeCpp {
    test_spawn_metadata_with_flags(
        groups.map(|(group_id, map_id)| (group_id, map_id, SpawnGroupFlags::NONE)),
    )
}

fn test_spawn_metadata_with_flags<const N: usize>(
    groups: [(u32, u32, SpawnGroupFlags); N],
) -> super::spawn_store_loader::CanonicalSpawnMetadataLikeCpp {
    let mut store = SpawnStore::new();
    let mut templates = BTreeMap::new();
    let mut rows = Vec::new();
    for (index, (group_id, map_id, flags)) in groups.into_iter().enumerate() {
        templates.insert(
            group_id,
            SpawnGroupTemplateData {
                group_id,
                name: format!("test group {group_id}"),
                map_id: wow_map::spawn::SPAWNGROUP_MAP_UNSET,
                flags,
            },
        );
        let spawn_id = u64::try_from(index).expect("test index fits") + 1;
        let spawn = test_spawn(spawn_id, map_id);
        store.add_object_spawn(&spawn, |_| false);
        rows.push(SpawnGroupMemberRow {
            group_id,
            spawn_type: SpawnObjectType::Creature as u8,
            spawn_id,
        });
    }
    store.apply_spawn_groups_like_cpp(&mut templates, rows);
    super::spawn_store_loader::CanonicalSpawnMetadataLikeCpp::new(store, templates)
}

fn test_spawn_metadata_with_explicit_spawn_ids<const N: usize>(
    groups: [(u32, u32, SpawnGroupFlags, u64); N],
) -> super::spawn_store_loader::CanonicalSpawnMetadataLikeCpp {
    let mut store = SpawnStore::new();
    let mut templates = BTreeMap::new();
    let mut rows = Vec::new();
    for (group_id, map_id, flags, spawn_id) in groups {
        templates.insert(
            group_id,
            SpawnGroupTemplateData {
                group_id,
                name: format!("test group {group_id}"),
                map_id: wow_map::spawn::SPAWNGROUP_MAP_UNSET,
                flags,
            },
        );
        let spawn = test_spawn(spawn_id, map_id);
        store.add_object_spawn(&spawn, |_| false);
        rows.push(SpawnGroupMemberRow {
            group_id,
            spawn_type: SpawnObjectType::Creature as u8,
            spawn_id,
        });
    }
    store.apply_spawn_groups_like_cpp(&mut templates, rows);
    super::spawn_store_loader::CanonicalSpawnMetadataLikeCpp::new(store, templates)
}

fn test_spawn(spawn_id: u64, map_id: u32) -> SpawnData {
    SpawnData {
        object_type: SpawnObjectType::Creature,
        spawn_id,
        map_id,
        db_data: true,
        spawn_group: SpawnGroupTemplateData::default_group(),
        id: 42,
        spawn_point: SpawnPosition::new(0.0, 0.0, 0.0, 0.0),
        phase_use_flags: 0,
        phase_id: 0,
        phase_group: 0,
        terrain_swap_map: -1,
        pool_id: 0,
        spawn_time_secs: 120,
        spawn_difficulties: vec![0],
        script_id: 0,
        string_id: String::new(),
    }
}

fn mapid_condition(spawn_group_id: u32, expected_map_id: u32) -> Condition {
    Condition {
        source_type: ConditionSourceType::SpawnGroup,
        source_group: 0,
        source_entry: spawn_group_id as i32,
        source_id: 0,
        condition_type: ConditionType::MapId,
        condition_value1: expected_map_id,
        ..Condition::default()
    }
}

fn unique_temp_dir(name: &str) -> PathBuf {
    let mut path = env::temp_dir();
    path.push(format!(
        "rustycore_world_server_{name}_{}",
        std::process::id()
    ));

    let _ = fs::remove_dir_all(&path);
    fs::create_dir_all(&path).expect("temp dir failed");
    path
}

// ── Slice 4A.1b: routing tests ───────────────────────────────────────────
// C++ anchors:
//   Object.cpp : WorldObject::SendMessageToSet (~1746-1764)
//   GridNotifiersImpl.h : MessageDistDeliverer::Visit(PlayerMapType&) (~43-46)
//   GridNotifiers.h : MessageDistDeliverer::SendPacket

fn make_source_guid() -> ObjectGuid {
    ObjectGuid::create_world_object(wow_core::guid::HighGuid::Creature, 0, 1, 571, 0, 1, 1)
}

fn make_nearby_visible_event_like_cpp(
    map_id: u16,
    instance_id: u32,
    source_position: Position,
    range: f32,
    required_3d: bool,
) -> wow_world::map_manager::RuntimeEvent {
    wow_world::map_manager::RuntimeEvent {
        source_guid: make_source_guid(),
        recipients: wow_world::map_manager::RecipientRule::NearbyVisible {
            source_guid: make_source_guid(),
            map_id,
            instance_id,
            source_position,
            range,
            required_3d,
        },
        packet_bytes: vec![0xAA, 0xBB],
    }
}

fn make_creature_spell_runtime_plan_like_cpp(
    target_guid: ObjectGuid,
) -> (
    wow_world::map_manager::RuntimePlan,
    Vec<u8>,
    Vec<u8>,
    Vec<u8>,
) {
    use wow_packet::packets::spell::{
        SpellCastLogData, SpellCastVisual, SpellGoPkt, SpellLogPowerData, SpellStartPkt,
        SpellTargetData,
    };

    let caster_guid = make_source_guid();
    let cast_id = ObjectGuid::create_world_object(HighGuid::Cast, 3, 1, 571, 0, 12_345, 77);
    let visual = SpellCastVisual {
        spell_visual_id: 987,
        script_visual_id: 0,
    };
    let target = SpellTargetData {
        flags: 0x2,
        unit: target_guid,
        item: ObjectGuid::EMPTY,
        ..Default::default()
    };
    let start_bytes = SpellStartPkt {
        cast_data: Default::default(),
        caster: caster_guid,
        cast_id,
        original_cast_id: ObjectGuid::EMPTY,
        spell_id: 12_345,
        visual: visual.clone(),
        cast_flags: 0x0000_0002,
        cast_flags_ex: 0,
        cast_time_ms: 0,
        target: target.clone(),
    }
    .to_bytes();
    let go = SpellGoPkt {
        cast_data: Default::default(),
        caster: caster_guid,
        cast_id,
        original_cast_id: ObjectGuid::EMPTY,
        spell_id: 12_345,
        visual,
        cast_flags: 0x0004_0100,
        cast_flags_ex: 0,
        cast_time_ms: 123,
        target,
        hit_targets: vec![target_guid],
        miss_targets: Vec::new(),
    };
    let basic_go_bytes = go.to_bytes();
    let full_go_bytes = go.to_full_log_bytes_like_cpp(&SpellCastLogData {
        health: 321,
        attack_power: 45,
        spell_power: 0,
        armor: 67,
        power_data: vec![SpellLogPowerData {
            power_type: 0,
            amount: 89,
            cost: 0,
        }],
    });
    let plan = wow_world::map_manager::RuntimePlan {
        events: vec![wow_world::map_manager::RuntimeEvent {
            source_guid: caster_guid,
            recipients: wow_world::map_manager::RecipientRule::NearbyVisibleDurableSpellCast {
                source_guid: caster_guid,
                map_id: 571,
                instance_id: 0,
                source_position: Position::ZERO,
                range: 100.0,
                required_3d: false,
                basic_go_packet_bytes: basic_go_bytes.clone(),
                full_go_packet_bytes: full_go_bytes.clone(),
            },
            packet_bytes: start_bytes.clone(),
        }],
    };
    (plan, start_bytes, basic_go_bytes, full_go_bytes)
}

fn make_registry_player_like_cpp(
    map_id: u16,
    instance_id: u32,
    position: Position,
    is_in_world: bool,
) -> (
    PlayerSessionRegistrationLikeCpp,
    flume::Receiver<SessionCommand>,
) {
    let (send_tx, _send_rx) = flume::bounded(4);
    let (command_tx, command_rx) = flume::bounded(4);
    let mut info = player_registration_fixture_like_cpp(send_tx, command_tx, "Tester");
    info.placement.map_id = map_id;
    info.placement.instance_id = instance_id;
    info.placement.position = position;
    info.placement.is_in_world = is_in_world;
    (info, command_rx)
}

fn add_canonical_test_player_on_map_like_cpp(
    canonical: &wow_world::SharedCanonicalMapManager,
    guid: ObjectGuid,
    position: Position,
    map_id: u32,
    instance_id: u32,
    health: u64,
) {
    let mut player = Player::new(Some(1), false);
    player.unit_mut().world_mut().object_mut().create(guid);
    player.unit_mut().world_mut().set_name("RuntimeVictim");
    player
        .unit_mut()
        .world_mut()
        .set_map(map_id, instance_id)
        .unwrap();
    player.unit_mut().world_mut().relocate(position);
    player.unit_mut().world_mut().object_mut().add_to_world();
    player.unit_mut().set_level(80);
    player.unit_mut().set_faction(1);
    player.unit_mut().set_max_health(health);
    player.unit_mut().set_health(health);

    canonical
        .lock()
        .unwrap()
        .create_world_map(map_id, instance_id)
        .map_mut()
        .insert_map_object_record(MapObjectRecord::new_player(player).unwrap())
        .unwrap();
}

fn add_canonical_test_creature_on_map_like_cpp(
    canonical: &wow_world::SharedCanonicalMapManager,
    guid: ObjectGuid,
    position: Position,
    map_id: u32,
    instance_id: u32,
    health: u64,
) {
    let mut creature = Creature::new(false);
    creature.unit_mut().world_mut().object_mut().create(guid);
    creature.unit_mut().world_mut().object_mut().set_entry(9002);
    creature
        .unit_mut()
        .world_mut()
        .set_map(map_id, instance_id)
        .unwrap();
    creature.unit_mut().world_mut().relocate(position);
    creature.unit_mut().world_mut().set_combat_reach(1.0);
    creature.unit_mut().world_mut().object_mut().add_to_world();
    creature.unit_mut().set_level(80);
    creature.unit_mut().set_max_health(health);
    creature.unit_mut().set_health(health);

    canonical
        .lock()
        .unwrap()
        .create_world_map(map_id, instance_id)
        .map_mut()
        .insert_map_object_record(MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();
}

fn mirror_canonical_melee_test_creature_like_cpp(
    canonical: &wow_world::SharedCanonicalMapManager,
    guid: ObjectGuid,
    map_id: u32,
    instance_id: u32,
) -> wow_world::map_manager::WorldCreature {
    let mut creature = canonical
        .lock()
        .unwrap()
        .find_map(map_id, instance_id)
        .unwrap()
        .map()
        .with_creature_like_cpp(guid, Clone::clone)
        .expect("canonical test creature");
    creature.set_ai_home_position(creature.position());
    creature.set_ai_identity_runtime(100, 14, 0, 0);
    creature
        .unit_mut()
        .set_weapon_damage(wow_constants::WeaponAttackType::BaseAttack, 3.0, 5.0);
    {
        let ai = creature.ai_ownership_mut();
        ai.aggro_radius = 20.0;
        ai.min_damage = 3;
        ai.max_damage = 5;
    }
    let create_data =
        wow_world::map_manager::WorldCreature::create_data_from_canonical_like_cpp(&creature);
    wow_world::map_manager::WorldCreature::from_canonical(creature, create_data)
}

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
