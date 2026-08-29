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
    database_auto_create_enabled_like_cpp, database_pool_size_like_cpp,
    db_keepalive_database_names_like_cpp, db_keepalive_interval_minutes_like_cpp,
    db_keepalive_sql_like_cpp, db_updater_step_like_cpp,
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
    run_legacy_creature_lifecycle_tick_and_refresh_once_like_cpp,
    run_legacy_creature_melee_tick_and_deliver_once_like_cpp,
    run_legacy_creature_movement_tick_and_deliver_once_like_cpp,
    run_legacy_creature_runtime_tick_and_deliver_once_like_cpp,
    run_world_session_shutdown_finalize_step_like_cpp, set_realm_offline_sql_like_cpp,
    set_realm_online_sql_like_cpp, spawn_legacy_creature_runtime_update_loop_like_cpp,
    spawn_store_loader, stop_world_network_like_cpp, update_sessions_shutdown_flush_once_like_cpp,
    updates_auto_setup_enabled_like_cpp, updates_database_mask_like_cpp,
    updates_enabled_for_database_like_cpp, world_config_bool, world_config_f32, world_config_u8,
    world_config_u16, world_config_u32, world_db_core_version_update_sql_like_cpp,
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
use wow_database::{
    DATABASE_CHARACTER_LIKE_CPP, DATABASE_HOTFIX_LIKE_CPP, DATABASE_LOGIN_LIKE_CPP,
    DATABASE_MASK_ALL_LIKE_CPP, DATABASE_WORLD_LIKE_CPP, StatementDef,
};
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

#[test]
fn signed_tinyint_quest_required_preserves_cpp_boolean_semantics() {
    assert!(!loot_quest_required_from_signed_db_like_cpp(0));
    assert!(loot_quest_required_from_signed_db_like_cpp(1));
    assert!(loot_quest_required_from_signed_db_like_cpp(-1));
}

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

#[test]
fn item_guid_allocator_start_is_max_plus_one_and_fails_before_generator_panic_like_cpp() {
    assert_eq!(next_item_guid_allocator_start_like_cpp(None).unwrap(), 1);
    let start = next_item_guid_allocator_start_like_cpp(Some(41)).unwrap();
    assert_eq!(start, 42);
    let generator = ObjectGuidGenerator::new(HighGuid::Item, start);
    assert_eq!(
        generator.generate(),
        42,
        "fetch_add returns the configured MAX+1 start before advancing"
    );

    let generator_limit = ObjectGuid::max_counter(HighGuid::Item) - 1;
    assert_eq!(
        next_item_guid_allocator_start_like_cpp(Some((generator_limit - 2) as u64)).unwrap(),
        generator_limit - 1
    );
    assert!(
        next_item_guid_allocator_start_like_cpp(Some((generator_limit - 1) as u64)).is_err(),
        "startup must reject the value that ObjectGuidGenerator::generate would panic on"
    );
    assert!(next_item_guid_allocator_start_like_cpp(Some(u64::MAX)).is_err());
}

#[test]
fn equipment_set_guid_allocator_start_uses_shared_cpp_maximum_and_fails_closed() {
    assert_eq!(
        next_equipment_set_guid_allocator_start_like_cpp(None).unwrap(),
        1
    );
    let start = next_equipment_set_guid_allocator_start_like_cpp(Some(41)).unwrap();
    assert_eq!(start, 42);
    let generator = wow_core::EquipmentSetGuidGeneratorLikeCpp::new(start);
    assert_eq!(generator.generate(), 42);

    let limit = wow_core::EQUIPMENT_SET_GUID_LIMIT_LIKE_CPP;
    assert_eq!(
        next_equipment_set_guid_allocator_start_like_cpp(Some(limit - 2)).unwrap(),
        limit - 1
    );
    assert!(
        next_equipment_set_guid_allocator_start_like_cpp(Some(limit - 1)).is_err(),
        "startup must reject the value that the C++ generator refuses to allocate"
    );
    assert!(next_equipment_set_guid_allocator_start_like_cpp(Some(u64::MAX)).is_err());
}

#[test]
fn void_storage_item_id_allocator_start_matches_cpp_and_fails_closed() {
    assert_eq!(
        next_void_storage_item_id_allocator_start_like_cpp(None).unwrap(),
        1
    );
    let start = next_void_storage_item_id_allocator_start_like_cpp(Some(41)).unwrap();
    assert_eq!(start, 42);
    let generator = wow_core::VoidStorageItemIdGeneratorLikeCpp::new(start);
    assert_eq!(generator.generate(), 42);

    let limit = wow_core::VOID_STORAGE_ITEM_ID_LIMIT_LIKE_PACKET_GUID;
    assert_eq!(
        next_void_storage_item_id_allocator_start_like_cpp(Some(limit - 2)).unwrap(),
        limit - 1
    );
    assert!(next_void_storage_item_id_allocator_start_like_cpp(Some(limit - 1)).is_err());
    assert!(next_void_storage_item_id_allocator_start_like_cpp(Some(u64::MAX)).is_err());
}

#[test]
fn item_guid_allocator_cleans_every_dangling_reference_before_publication() {
    let sql =
        ITEM_GUID_DANGLING_REFERENCE_CLEANUP_STATEMENTS_LIKE_CPP.map(|statement| statement.sql());
    assert_eq!(sql.len(), 6);
    assert!(sql[0].contains("character_inventory"));
    assert!(sql[1].contains("mail_items"));
    assert!(sql[2].contains("auctionhouse"));
    assert!(sql[3].contains("guild_bank_item"));
    assert_eq!(
        sql[4],
        "DELETE FROM item_loot_items WHERE container_id >= ?"
    );
    assert_eq!(
        sql[5],
        "DELETE FROM item_loot_money WHERE container_id >= ?"
    );
    for statement in sql {
        assert!(statement.contains(">= ?"));
    }
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

#[test]
fn realm_list_entry_normalizes_realm_type_and_security_like_cpp() {
    assert_eq!(normalize_realm_type_like_cpp(16), 1);
    assert_eq!(normalize_realm_type_like_cpp(14), 0);
    assert_eq!(normalize_realm_type_like_cpp(6), 6);
    assert_eq!(normalize_realm_security_level_like_cpp(9), 3);
    assert_eq!(normalize_realm_security_level_like_cpp(2), 2);
    assert_eq!(
        normalized_realm_name_like_cpp("Ice Crown\t Citadel\n"),
        "IceCrownCitadel"
    );

    let entry = realm_list_entry_from_row_like_cpp(super::RealmListRawRowLikeCpp {
        realm_id: 7,
        name: "Northrend".to_string(),
        address: "203.0.113.10".to_string(),
        local_address: "10.0.0.10".to_string(),
        port: 8085,
        icon: 16,
        flag: 2,
        timezone: 1,
        allowed_security_level: 9,
        population: 0.75,
        build: 51943,
        region: 2,
        battlegroup: 3,
    });

    assert_eq!(entry.id.address_like_cpp(), 0x0203_0007);
    assert_eq!(entry.id.address_string_like_cpp(), "2-3-7");
    assert_eq!(entry.id.sub_region_address_like_cpp(), "2-3-0");
    assert_eq!(entry.normalized_name, "Northrend");
    assert_eq!(entry.icon, 1);
    assert_eq!(entry.allowed_security_level, 3);
}

#[test]
fn pvp_realm_classification_matches_cpp_realm_types() {
    assert!(!is_pvp_realm_type_like_cpp(0));
    assert!(is_pvp_realm_type_like_cpp(1));
    assert!(!is_pvp_realm_type_like_cpp(6));
    assert!(is_pvp_realm_type_like_cpp(8));
    assert!(is_pvp_realm_type_like_cpp(16));

    assert!(!is_ffa_pvp_realm_type_like_cpp(0));
    assert!(!is_ffa_pvp_realm_type_like_cpp(1));
    assert!(!is_ffa_pvp_realm_type_like_cpp(8));
    assert!(is_ffa_pvp_realm_type_like_cpp(16));
}

#[test]
fn connect_to_address_uses_shared_select_address_priority_like_cpp() {
    assert_eq!(
        get_address_for_client_with_local_networks(
            Some("127.0.0.1".parse().unwrap()),
            [198, 51, 100, 10],
            [10, 0, 0, 10],
            &[],
        ),
        [10, 0, 0, 10]
    );
    assert_eq!(
        get_address_for_client_with_local_networks(
            Some("10.0.0.42".parse().unwrap()),
            [198, 51, 100, 10],
            [10, 0, 0, 10],
            &[],
        ),
        [10, 0, 0, 10]
    );
    assert_eq!(
        get_address_for_client_with_local_networks(
            Some("203.0.113.42".parse().unwrap()),
            [198, 51, 100, 10],
            [10, 0, 0, 10],
            &[],
        ),
        [198, 51, 100, 10]
    );
}

#[test]
fn realm_handle_ordering_matches_cpp_realm_id_only() {
    let first = super::RealmHandleLikeCpp::new_like_cpp(1, 2, 7);
    let same_realm_different_subregion = super::RealmHandleLikeCpp::new_like_cpp(9, 8, 7);
    let second = super::RealmHandleLikeCpp::new_like_cpp(1, 2, 8);

    assert_eq!(first, same_realm_different_subregion);
    assert_eq!(
        first.cmp(&same_realm_different_subregion),
        std::cmp::Ordering::Equal
    );
    assert!(first < second);
}

#[test]
fn realm_list_snapshot_replace_counts_added_updated_removed_like_cpp() {
    let mut current = super::RealmListSnapshotLikeCpp::default();
    let first = realm_list_entry_from_row_like_cpp(super::RealmListRawRowLikeCpp {
        realm_id: 1,
        name: "A".to_string(),
        address: "127.0.0.1".to_string(),
        local_address: "127.0.0.1".to_string(),
        port: 8085,
        icon: 1,
        flag: 0,
        timezone: 1,
        allowed_security_level: 0,
        population: 0.5,
        build: 51943,
        region: 1,
        battlegroup: 1,
    });
    let second = realm_list_entry_from_row_like_cpp(super::RealmListRawRowLikeCpp {
        realm_id: 2,
        name: "B".to_string(),
        address: "127.0.0.2".to_string(),
        local_address: "127.0.0.2".to_string(),
        port: 8086,
        icon: 1,
        flag: 0,
        timezone: 1,
        allowed_security_level: 0,
        population: 0.5,
        build: 51943,
        region: 1,
        battlegroup: 2,
    });

    let mut next = super::RealmListSnapshotLikeCpp::default();
    next.sub_regions
        .insert(first.id.sub_region_address_like_cpp());
    next.realms.insert(first.id, first.clone());
    assert_eq!(
        current.replace_like_cpp(next),
        super::RealmListRefreshSummaryLikeCpp {
            realms: 1,
            sub_regions: 1,
            added: 1,
            updated: 0,
            removed: 0,
        }
    );
    assert!(current.get_realm_like_cpp(first.id).is_some());

    let mut replacement = super::RealmListSnapshotLikeCpp::default();
    replacement
        .sub_regions
        .insert(second.id.sub_region_address_like_cpp());
    replacement.realms.insert(second.id, second.clone());
    assert_eq!(
        current.replace_like_cpp(replacement),
        super::RealmListRefreshSummaryLikeCpp {
            realms: 1,
            sub_regions: 1,
            added: 1,
            updated: 0,
            removed: 1,
        }
    );
    assert!(current.get_realm_like_cpp(first.id).is_none());
    assert!(current.get_realm_like_cpp(second.id).is_some());
}

#[test]
fn load_realm_info_reads_active_realm_from_snapshot_like_cpp() {
    let mut snapshot = super::RealmListSnapshotLikeCpp::default();
    let entry = realm_list_entry_from_row_like_cpp(super::RealmListRawRowLikeCpp {
        realm_id: 9,
        name: "Icecrown".to_string(),
        address: "198.51.100.9".to_string(),
        local_address: "10.0.0.9".to_string(),
        port: 8085,
        icon: 1,
        flag: 0,
        timezone: 1,
        allowed_security_level: 0,
        population: 0.2,
        build: 51943,
        region: 5,
        battlegroup: 6,
    });
    snapshot.realms.insert(entry.id, entry.clone());
    let snapshot = Arc::new(Mutex::new(snapshot));

    assert_eq!(
        super::load_realm_info_from_snapshot_like_cpp(&snapshot, 9).expect("realm found"),
        entry
    );
    let loaded = super::load_realm_info_from_snapshot_like_cpp(&snapshot, 9).expect("realm found");
    assert_eq!(loaded.id.region, 5);
    assert_eq!(loaded.id.site, 6);
    assert_eq!(loaded.id.address_like_cpp(), 0x0506_0009);
    assert_eq!(
        super::realm_name_records_from_snapshot_like_cpp(&snapshot).as_ref(),
        &vec![(0x0506_0009, "Icecrown".to_string(), "Icecrown".to_string())]
    );
    assert!(super::load_realm_info_from_snapshot_like_cpp(&snapshot, 10).is_err());
}

#[test]
fn kick_all_sessions_queues_world_kick_for_every_registered_session_like_cpp() {
    let registry = ActiveWorldSessionRegistryLikeCpp::new();
    let (command_tx_a, command_rx_a) = flume::bounded(1);
    let (command_tx_b, command_rx_b) = flume::bounded(1);

    let first_id = registry.register(10, command_tx_a);
    let second_id = registry.register(20, command_tx_b);
    assert_ne!(first_id, second_id);
    assert_eq!(registry.len(), 2);

    assert_eq!(
        kick_all_sessions_like_cpp(&registry),
        KickAllSessionsSummaryLikeCpp {
            sessions_seen: 2,
            queued: 2,
            send_failed: 0,
        }
    );

    for rx in [command_rx_a, command_rx_b] {
        let command = rx.try_recv().expect("kick command queued");
        let SessionCommand::KickLikeCpp(command) = command else {
            panic!("expected KickLikeCpp command");
        };
        assert_eq!(command.reason, "World::KickAll");
    }
}

#[test]
fn kick_all_sessions_counts_full_command_channel_without_blocking_like_cpp() {
    let registry = ActiveWorldSessionRegistryLikeCpp::new();
    let (command_tx, _command_rx) = flume::bounded(0);

    registry.register(30, command_tx);

    assert_eq!(
        kick_all_sessions_like_cpp(&registry),
        KickAllSessionsSummaryLikeCpp {
            sessions_seen: 1,
            queued: 0,
            send_failed: 1,
        }
    );
}

#[test]
fn active_world_session_registry_unregisters_finished_sessions_like_cpp() {
    let registry = ActiveWorldSessionRegistryLikeCpp::new();
    let (command_tx, _command_rx) = flume::bounded(1);
    let id = registry.register(40, command_tx);

    assert_eq!(registry.len(), 1);
    assert_eq!(
        registry.unregister(id).map(|session| session.account_id),
        Some(40)
    );
    assert_eq!(registry.len(), 0);
    assert!(registry.unregister(id).is_none());
}

#[test]
fn active_world_session_registry_shutdown_gate_rejects_late_registration_like_cpp() {
    let registry = ActiveWorldSessionRegistryLikeCpp::new();
    let (command_tx_a, _command_rx_a) = flume::bounded(1);
    let first_id = registry
        .try_register(41, command_tx_a)
        .expect("open registry accepts the existing session")
        .0;

    registry.begin_shutdown_like_cpp();

    let (command_tx_b, _command_rx_b) = flume::bounded(1);
    assert!(registry.try_register(42, command_tx_b).is_none());
    assert!(registry.is_shutting_down_like_cpp());
    assert!(!registry.should_stop_sessions_like_cpp());
    registry.request_session_stop_like_cpp();
    assert!(registry.should_stop_sessions_like_cpp());
    assert_eq!(registry.len(), 1);
    registry.unregister(first_id);
}

#[tokio::test]
async fn active_world_session_registry_closed_wait_observes_final_unregister_like_cpp() {
    let registry = Arc::new(ActiveWorldSessionRegistryLikeCpp::new());
    let (command_tx, _command_rx) = flume::bounded(1);
    let id = registry.register(43, command_tx);
    registry.begin_shutdown_like_cpp();

    let unregister_registry = Arc::clone(&registry);
    tokio::spawn(async move {
        tokio::task::yield_now().await;
        unregister_registry.unregister(id);
    });

    assert!(
        registry
            .wait_until_empty_like_cpp(Duration::from_secs(1))
            .await
    );
    assert!(registry.is_empty_like_cpp());
}

#[tokio::test]
async fn active_world_session_registry_force_cancel_drops_registration_guard_like_cpp() {
    let registry = Arc::new(ActiveWorldSessionRegistryLikeCpp::new());
    let (command_tx, _command_rx) = flume::bounded(1);
    let (id, cancellation) = registry
        .try_register(44, command_tx)
        .expect("open registry accepts session");
    let registration = ActiveWorldSessionRegistrationGuardLikeCpp {
        registry: Arc::clone(&registry),
        id,
    };
    registry.begin_shutdown_like_cpp();

    let session_task = tokio::spawn(async move {
        cancellation.cancelled_like_cpp().await;
        drop(registration);
    });
    assert_eq!(registry.cancel_all_sessions_like_cpp(), 1);
    assert!(
        registry
            .wait_until_empty_like_cpp(Duration::from_secs(1))
            .await
    );
    session_task.await.expect("cancelled session task joined");
    assert!(registry.is_empty_like_cpp());
}

#[tokio::test]
async fn world_session_shutdown_finalize_success_keeps_clean_exit_like_cpp() {
    let world = WorldRuntimeStateLikeCpp::new();

    assert!(
        run_world_session_shutdown_finalize_step_like_cpp(
            &world,
            Duration::from_secs(1),
            async {},
        )
        .await
    );
    assert_eq!(world.get_exit_code_like_cpp(), SHUTDOWN_EXIT_CODE_LIKE_CPP);
}

#[tokio::test]
async fn world_session_shutdown_finalize_timeout_sets_terminal_error_like_cpp() {
    let world = WorldRuntimeStateLikeCpp::new();

    assert!(
        !run_world_session_shutdown_finalize_step_like_cpp(
            &world,
            Duration::from_millis(1),
            std::future::pending::<()>(),
        )
        .await
    );
    assert!(world.is_stopped_like_cpp());
    assert_eq!(world.get_exit_code_like_cpp(), ERROR_EXIT_CODE_LIKE_CPP);
}

#[tokio::test]
async fn active_world_session_registry_wait_empty_returns_immediately_like_cpp() {
    let registry = ActiveWorldSessionRegistryLikeCpp::new();

    assert!(
        registry
            .wait_until_empty_like_cpp(Duration::from_millis(1))
            .await
    );
}

#[tokio::test]
async fn active_world_session_registry_wait_empty_observes_unregister_like_cpp() {
    let registry = Arc::new(ActiveWorldSessionRegistryLikeCpp::new());
    let (command_tx, _command_rx) = flume::bounded(1);
    let id = registry.register(41, command_tx);
    let unregister_registry = Arc::clone(&registry);

    let unregister_task = tokio::spawn(async move {
        unregister_registry.unregister(id);
    });

    assert!(
        registry
            .wait_until_empty_like_cpp(Duration::from_secs(1))
            .await
    );
    unregister_task.await.expect("unregister task joined");
    assert_eq!(registry.len(), 0);
}

#[tokio::test]
async fn active_world_session_registry_wait_empty_times_out_like_cpp() {
    let registry = ActiveWorldSessionRegistryLikeCpp::new();
    let (command_tx, _command_rx) = flume::bounded(1);

    registry.register(42, command_tx);

    assert!(
        !registry
            .wait_until_empty_like_cpp(Duration::from_millis(1))
            .await
    );
    assert_eq!(registry.len(), 1);
}

#[tokio::test]
async fn shutdown_flush_queues_update_sessions_ack_command_like_cpp() {
    let registry = ActiveWorldSessionRegistryLikeCpp::new();
    let (command_tx, command_rx) = flume::bounded(1);

    registry.register(50, command_tx);

    let responder = tokio::spawn(async move {
        let command = command_rx.recv_async().await.expect("flush command queued");
        let SessionCommand::WorldSessionShutdownFlushLikeCpp(command) = command else {
            panic!("expected shutdown flush command");
        };
        assert_eq!(command.diff_ms, 1);
        command
            .response_tx
            .try_send(WorldSessionShutdownFlushResultLikeCpp {
                diff_ms: command.diff_ms,
                disconnecting: true,
            })
            .expect("ack accepted");
    });

    assert_eq!(
        update_sessions_shutdown_flush_once_like_cpp(&registry, 1, Duration::from_secs(1)).await,
        UpdateSessionsShutdownFlushSummaryLikeCpp {
            sessions_seen: 1,
            queued: 1,
            send_failed: 0,
            acked: 1,
            ack_failed: 0,
            ack_timeout: 0,
            disconnecting: 1,
        }
    );
    responder.await.expect("responder joined");
}

#[tokio::test]
async fn shutdown_flush_counts_full_command_channel_without_blocking_like_cpp() {
    let registry = ActiveWorldSessionRegistryLikeCpp::new();
    let (command_tx, _command_rx) = flume::bounded(0);

    registry.register(60, command_tx);

    assert_eq!(
        update_sessions_shutdown_flush_once_like_cpp(&registry, 1, Duration::from_millis(1)).await,
        UpdateSessionsShutdownFlushSummaryLikeCpp {
            sessions_seen: 1,
            queued: 0,
            send_failed: 1,
            acked: 0,
            ack_failed: 0,
            ack_timeout: 0,
            disconnecting: 0,
        }
    );
}

#[tokio::test]
async fn shutdown_flush_counts_unacknowledged_session_timeout_like_cpp() {
    let registry = ActiveWorldSessionRegistryLikeCpp::new();
    let (command_tx, _command_rx) = flume::bounded(1);

    registry.register(70, command_tx);

    assert_eq!(
        update_sessions_shutdown_flush_once_like_cpp(&registry, 1, Duration::from_millis(1)).await,
        UpdateSessionsShutdownFlushSummaryLikeCpp {
            sessions_seen: 1,
            queued: 1,
            send_failed: 0,
            acked: 0,
            ack_failed: 0,
            ack_timeout: 1,
            disconnecting: 0,
        }
    );
}

#[tokio::test]
async fn stop_world_network_aborts_realm_and_instance_listeners_like_cpp() {
    let realm_task = tokio::spawn(async {
        std::future::pending::<()>().await;
    });
    let instance_task = tokio::spawn(async {
        std::future::pending::<()>().await;
    });
    let realm_abort = realm_task.abort_handle();
    let instance_abort = instance_task.abort_handle();

    assert_eq!(
        stop_world_network_like_cpp([("realm", &realm_abort), ("instance", &instance_abort)]),
        StopWorldNetworkSummaryLikeCpp { listeners: 2 }
    );

    assert!(
        realm_task
            .await
            .expect_err("realm listener aborted")
            .is_cancelled()
    );
    assert!(
        instance_task
            .await
            .expect_err("instance listener aborted")
            .is_cancelled()
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

#[test]
fn login_grid_world_fallback_rejects_instanceable_map_kinds_like_cpp() {
    let world = loaded_grid_map_store_like_cpp(1, wow_data::map::MAP_COMMON);
    let dungeon = loaded_grid_map_store_like_cpp(33, wow_data::map::MAP_INSTANCE);
    let battleground = loaded_grid_map_store_like_cpp(489, wow_data::map::MAP_BATTLEGROUND);
    let garrison = wow_data::MapEntry {
        id: 1_151,
        instance_type: wow_data::map::MAP_COMMON,
        expansion_id: 0,
        parent_map_id: -1,
        cosmetic_parent_map_id: -1,
        flags1: wow_data::map::MAP_FLAG_GARRISON,
        flags2: 0,
    };
    let faction_split_world = wow_data::MapEntry {
        id: 609,
        instance_type: wow_data::map::MAP_COMMON,
        expansion_id: 0,
        parent_map_id: -1,
        cosmetic_parent_map_id: -1,
        flags1: 0,
        flags2: 0,
    };

    assert!(super::can_create_missing_login_grid_as_world_map_like_cpp(
        *world.get(1).unwrap()
    ));
    assert!(!super::can_create_missing_login_grid_as_world_map_like_cpp(
        *dungeon.get(33).unwrap()
    ));
    assert!(!super::can_create_missing_login_grid_as_world_map_like_cpp(
        *battleground.get(489).unwrap()
    ));
    assert!(!super::can_create_missing_login_grid_as_world_map_like_cpp(
        garrison
    ));
    assert!(!super::can_create_missing_login_grid_as_world_map_like_cpp(
        faction_split_world
    ));
    assert!(super::existing_login_grid_map_matches_map_entry_like_cpp(
        *world.get(1).unwrap(),
        wow_map::ManagedMapKind::World,
        0,
        false,
    ));
    assert!(super::existing_login_grid_map_matches_map_entry_like_cpp(
        *dungeon.get(33).unwrap(),
        wow_map::ManagedMapKind::Dungeon {
            has_reset_schedule: false,
        },
        77,
        true,
    ));
    assert!(!super::existing_login_grid_map_matches_map_entry_like_cpp(
        *dungeon.get(33).unwrap(),
        wow_map::ManagedMapKind::Dungeon {
            has_reset_schedule: false,
        },
        0,
        true,
    ));
    assert!(!super::existing_login_grid_map_matches_map_entry_like_cpp(
        *dungeon.get(33).unwrap(),
        wow_map::ManagedMapKind::World,
        77,
        true,
    ));
    assert!(!super::existing_login_grid_map_matches_map_entry_like_cpp(
        faction_split_world,
        wow_map::ManagedMapKind::World,
        0,
        false,
    ));
    assert!(super::existing_login_grid_map_matches_map_entry_like_cpp(
        faction_split_world,
        wow_map::ManagedMapKind::World,
        0,
        true,
    ));
    assert!(!super::existing_login_grid_map_matches_map_entry_like_cpp(
        garrison,
        wow_map::ManagedMapKind::World,
        0,
        false,
    ));
    assert!(super::existing_login_grid_map_matches_map_entry_like_cpp(
        garrison,
        wow_map::ManagedMapKind::World,
        0,
        true,
    ));
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

#[test]
fn clear_online_accounts_sql_matches_cpp_startdb_cleanup() {
    let [account_sql, character_sql, battleground_sql] = clear_online_accounts_sql_like_cpp(3);

    assert_eq!(
        account_sql,
        "UPDATE account SET online = 0 WHERE online > 0 AND id IN (SELECT acctid FROM realmcharacters WHERE realmid = 3)"
    );
    assert_eq!(
        character_sql,
        "UPDATE characters SET online = 0 WHERE online <> 0"
    );
    assert_eq!(
        battleground_sql,
        "UPDATE character_battleground_data SET instanceId = 0"
    );
}

#[test]
fn realm_online_offline_sql_matches_cpp_lifecycle() {
    assert_eq!(
        set_realm_offline_sql_like_cpp(3),
        "UPDATE realmlist SET flag = flag | 2 WHERE id = 3"
    );
    assert_eq!(
        set_realm_online_sql_like_cpp(3),
        "UPDATE realmlist SET flag = flag & ~2, population = 0 WHERE id = 3"
    );
}

#[test]
fn create_pid_file_writes_current_process_id_like_cpp() {
    let root = unique_temp_dir("pid_file");
    let pid_file = root.join("world.pid");

    let pid = create_pid_file_like_cpp(&pid_file).expect("pid file should be created");

    assert_eq!(pid, std::process::id());
    assert_eq!(
        fs::read_to_string(&pid_file).expect("pid file should be readable"),
        std::process::id().to_string()
    );

    fs::remove_dir_all(root).expect("cleanup failed");
}

#[test]
fn game_event_unspawn_creature_gameobject_guids_queue_loaded_map_records_like_cpp() {
    let mut manager = wow_map::MapManager::default();
    manager.create_world_map(1, 0);
    manager.create_world_map(2, 0);
    let event_id = 1;
    let creature_spawn_id = 534101;
    let gameobject_spawn_id = 534201;
    let mut store = SpawnStore::new();
    add_spawn_data_like_cpp(&mut store, SpawnObjectType::Creature, creature_spawn_id, 1);
    add_spawn_data_like_cpp(
        &mut store,
        SpawnObjectType::GameObject,
        gameobject_spawn_id,
        1,
    );
    let mut guids =
        spawn_store_loader::GameEventSpawnGuidsLikeCpp::from_game_event_max_entry_like_cpp(Some(3));
    guids = push_game_event_guid_for_test_like_cpp(
        guids,
        SpawnObjectType::Creature,
        event_id,
        creature_spawn_id,
    );
    guids = push_game_event_guid_for_test_like_cpp(
        guids,
        SpawnObjectType::GameObject,
        event_id,
        gameobject_spawn_id,
    );
    let metadata = canonical_spawn_metadata_with_store_and_game_event_guids_like_cpp(store, guids);
    for object_type in [SpawnObjectType::Creature, SpawnObjectType::GameObject] {
        manager
            .find_map_mut(1, 0)
            .expect("test map 1")
            .map_mut()
            .add_respawn_info_like_cpp(respawn_info_like_cpp(
                object_type,
                if object_type == SpawnObjectType::Creature {
                    creature_spawn_id
                } else {
                    gameobject_spawn_id
                },
                534000,
            ));
        manager
            .find_map_mut(2, 0)
            .expect("test map 2")
            .map_mut()
            .add_respawn_info_like_cpp(respawn_info_like_cpp(
                object_type,
                if object_type == SpawnObjectType::Creature {
                    creature_spawn_id
                } else {
                    gameobject_spawn_id
                },
                534000,
            ));
    }
    insert_live_creature_for_spawn_like_cpp(&mut manager, 1, creature_spawn_id, 5341011);
    insert_live_creature_for_spawn_like_cpp(&mut manager, 1, creature_spawn_id, 5341012);
    insert_live_gameobject_for_spawn_like_cpp(&mut manager, 1, gameobject_spawn_id, 5342011);
    insert_live_gameobject_for_spawn_like_cpp(&mut manager, 1, gameobject_spawn_id, 5342012);

    let summary = game_event_unspawn_creatures_and_gameobjects_for_event_like_cpp(
        &mut manager,
        &metadata,
        &[],
        event_id,
    );

    assert_eq!(summary.event_id, event_id);
    assert!(!summary.missing_event_creature_guids);
    assert!(!summary.missing_event_gameobject_guids);
    assert_eq!(summary.creature.guids_seen, 1);
    assert_eq!(summary.creature.maps_matched, 1);
    assert_eq!(summary.creature.represented_object_mgr_grid_removals, 1);
    assert_eq!(summary.creature.respawn_timers_removed, 1);
    assert_eq!(summary.creature.live_objects_queued, 2);
    assert_eq!(summary.gameobject.guids_seen, 1);
    assert_eq!(summary.gameobject.maps_matched, 1);
    assert_eq!(summary.gameobject.represented_object_mgr_grid_removals, 1);
    assert_eq!(summary.gameobject.respawn_timers_removed, 1);
    assert_eq!(summary.gameobject.live_objects_queued, 2);
    assert!(
        manager
            .find_map(2, 0)
            .expect("test map 2")
            .map()
            .respawn_timer_keys_like_cpp()
            .any(|(_, spawn_id)| spawn_id == creature_spawn_id || spawn_id == gameobject_spawn_id)
    );
    let map_1 = manager.find_map_mut(1, 0).expect("test map 1").map_mut();
    let drained = map_1.remove_all_objects_in_remove_list_like_cpp();
    assert_eq!(drained.removed, 4);
}

#[test]
fn game_event_unspawn_positive_event_skips_guid_active_in_other_event_like_cpp() {
    let mut manager = wow_map::MapManager::default();
    manager.create_world_map(1, 0);
    let event_id = 1;
    let other_event_id = 2;
    let spawn_id = 534301;
    let mut store = SpawnStore::new();
    add_spawn_data_like_cpp(&mut store, SpawnObjectType::Creature, spawn_id, 1);
    let mut guids =
        spawn_store_loader::GameEventSpawnGuidsLikeCpp::from_game_event_max_entry_like_cpp(Some(3));
    guids = push_game_event_guid_for_test_like_cpp(
        guids,
        SpawnObjectType::Creature,
        event_id,
        spawn_id,
    );
    guids = push_game_event_guid_for_test_like_cpp(
        guids,
        SpawnObjectType::Creature,
        other_event_id,
        spawn_id,
    );
    let metadata = canonical_spawn_metadata_with_store_and_game_event_guids_like_cpp(store, guids);
    manager
        .find_map_mut(1, 0)
        .expect("test map")
        .map_mut()
        .add_respawn_info_like_cpp(respawn_info_like_cpp(
            SpawnObjectType::Creature,
            spawn_id,
            534000,
        ));
    insert_live_creature_for_spawn_like_cpp(&mut manager, 1, spawn_id, 5343011);

    let summary = game_event_unspawn_creatures_and_gameobjects_for_event_like_cpp(
        &mut manager,
        &metadata,
        &[other_event_id as u16],
        event_id,
    );

    assert_eq!(summary.creature.guids_seen, 1);
    assert_eq!(summary.creature.skipped_active_in_other_event, 1);
    assert_eq!(summary.creature.respawn_timers_removed, 0);
    assert_eq!(summary.creature.live_objects_queued, 0);
    assert!(
        manager
            .find_map(1, 0)
            .expect("test map")
            .map()
            .respawn_timer_keys_like_cpp()
            .any(|(_, timer_spawn_id)| timer_spawn_id == spawn_id)
    );
}

#[test]
fn game_event_unspawn_negative_event_does_not_apply_active_event_protection_like_cpp() {
    let mut manager = wow_map::MapManager::default();
    manager.create_world_map(1, 0);
    let event_id = -1;
    let positive_event_id = 1;
    let spawn_id = 534401;
    let mut store = SpawnStore::new();
    add_spawn_data_like_cpp(&mut store, SpawnObjectType::GameObject, spawn_id, 1);
    let mut guids =
        spawn_store_loader::GameEventSpawnGuidsLikeCpp::from_game_event_max_entry_like_cpp(Some(3));
    guids = push_game_event_guid_for_test_like_cpp(
        guids,
        SpawnObjectType::GameObject,
        event_id,
        spawn_id,
    );
    guids = push_game_event_guid_for_test_like_cpp(
        guids,
        SpawnObjectType::GameObject,
        positive_event_id,
        spawn_id,
    );
    let metadata = canonical_spawn_metadata_with_store_and_game_event_guids_like_cpp(store, guids);
    manager
        .find_map_mut(1, 0)
        .expect("test map")
        .map_mut()
        .add_respawn_info_like_cpp(respawn_info_like_cpp(
            SpawnObjectType::GameObject,
            spawn_id,
            534000,
        ));
    insert_live_gameobject_for_spawn_like_cpp(&mut manager, 1, spawn_id, 5344011);

    let summary = game_event_unspawn_creatures_and_gameobjects_for_event_like_cpp(
        &mut manager,
        &metadata,
        &[positive_event_id as u16],
        event_id,
    );

    assert_eq!(summary.gameobject.guids_seen, 1);
    assert_eq!(summary.gameobject.skipped_active_in_other_event, 0);
    assert_eq!(summary.gameobject.respawn_timers_removed, 1);
    assert_eq!(summary.gameobject.live_objects_queued, 1);
}

#[test]
fn game_event_unspawn_missing_creature_guid_list_returns_before_gameobjects_like_cpp() {
    let mut manager = wow_map::MapManager::default();
    manager.create_world_map(1, 0);
    let event_id = 99;
    let gameobject_spawn_id = 534501;
    let mut store = SpawnStore::new();
    add_spawn_data_like_cpp(
        &mut store,
        SpawnObjectType::GameObject,
        gameobject_spawn_id,
        1,
    );
    let mut guids =
        spawn_store_loader::GameEventSpawnGuidsLikeCpp::from_game_event_max_entry_like_cpp(Some(2));
    guids = push_game_event_guid_for_test_like_cpp(
        guids,
        SpawnObjectType::GameObject,
        1,
        gameobject_spawn_id,
    );
    let metadata = canonical_spawn_metadata_with_store_and_game_event_guids_like_cpp(store, guids);
    manager
        .find_map_mut(1, 0)
        .expect("test map")
        .map_mut()
        .add_respawn_info_like_cpp(respawn_info_like_cpp(
            SpawnObjectType::GameObject,
            gameobject_spawn_id,
            534000,
        ));

    let summary = game_event_unspawn_creatures_and_gameobjects_for_event_like_cpp(
        &mut manager,
        &metadata,
        &[],
        event_id,
    );

    assert_eq!(summary.event_id, event_id);
    assert!(summary.missing_event_creature_guids);
    assert!(!summary.missing_event_gameobject_guids);
    assert_eq!(summary.gameobject.guids_seen, 0);
    assert!(
        manager
            .find_map(1, 0)
            .expect("test map")
            .map()
            .respawn_timer_keys_like_cpp()
            .any(|(_, spawn_id)| spawn_id == gameobject_spawn_id)
    );
}

#[test]
fn game_event_unspawn_for_event_applies_non_pool_then_pool_like_cpp() {
    let mut manager = wow_map::MapManager::default();
    manager.create_world_map(1, 0);
    manager.create_world_map(2, 0);
    let event_id = 3;
    let creature_spawn_id = 536101;
    let gameobject_spawn_id = 536102;
    let pool_id = 536103;
    let pool_spawn_id = 536104;
    let mut store = SpawnStore::new();
    add_spawn_data_like_cpp(&mut store, SpawnObjectType::Creature, creature_spawn_id, 1);
    add_spawn_data_like_cpp(
        &mut store,
        SpawnObjectType::GameObject,
        gameobject_spawn_id,
        1,
    );
    let mut guids =
        spawn_store_loader::GameEventSpawnGuidsLikeCpp::from_game_event_max_entry_like_cpp(Some(
            10,
        ));
    guids = push_game_event_guid_for_test_like_cpp(
        guids,
        SpawnObjectType::Creature,
        event_id,
        creature_spawn_id,
    );
    guids = push_game_event_guid_for_test_like_cpp(
        guids,
        SpawnObjectType::GameObject,
        event_id,
        gameobject_spawn_id,
    );
    let game_event_pools =
        spawn_store_loader::GameEventPoolIdsLikeCpp::from_game_event_max_entry_like_cpp(Some(10))
            .with_pool_ids_for_event_like_cpp(event_id, [pool_id]);
    let metadata = spawn_store_loader::CanonicalSpawnMetadataLikeCpp::new(store, BTreeMap::new())
        .with_pool_mgr_like_cpp(pool_mgr_with_creature_pool_like_cpp(
            pool_id,
            1,
            pool_spawn_id,
        ))
        .with_game_event_pools_like_cpp(game_event_pools)
        .with_game_event_spawn_guids_like_cpp(guids);
    for (object_type, spawn_id) in [
        (SpawnObjectType::Creature, creature_spawn_id),
        (SpawnObjectType::GameObject, gameobject_spawn_id),
        (SpawnObjectType::Creature, pool_spawn_id),
    ] {
        manager
            .find_map_mut(1, 0)
            .expect("test map")
            .map_mut()
            .add_respawn_info_like_cpp(respawn_info_like_cpp(object_type, spawn_id, 536000));
    }
    insert_live_creature_for_spawn_like_cpp(&mut manager, 1, creature_spawn_id, 5361011);
    insert_live_gameobject_for_spawn_like_cpp(&mut manager, 1, gameobject_spawn_id, 5361021);
    manager
        .find_map_mut(1, 0)
        .expect("test map")
        .map_mut()
        .pool_data_mut_like_cpp()
        .add_spawn_like_cpp(SpawnObjectType::Creature, pool_spawn_id, pool_id)
        .expect("test spawned creature pool data");

    let summary = game_event_unspawn_for_event_like_cpp(&mut manager, &metadata, &[], event_id);

    assert_eq!(summary.event_id, event_id);
    assert!(!summary.pool_skipped_due_to_non_pool_bucket);
    assert!(!summary.non_pool.missing_event_creature_guids);
    assert!(!summary.non_pool.missing_event_gameobject_guids);
    assert_eq!(summary.non_pool.creature.respawn_timers_removed, 1);
    assert_eq!(summary.non_pool.creature.live_objects_queued, 1);
    assert_eq!(summary.non_pool.gameobject.respawn_timers_removed, 1);
    assert_eq!(summary.non_pool.gameobject.live_objects_queued, 1);
    assert!(!summary.pool.missing_event_pool_ids);
    assert_eq!(summary.pool.pool_summary.event_pool_ids_seen, 1);
    assert_eq!(summary.pool.pool_summary.maps_matched, 1);
    assert!(
        summary
            .pool
            .pool_summary
            .blocked_pool_plan_errors
            .is_empty()
    );
    let map = manager.find_map(1, 0).expect("test map").map();
    assert!(
        !map.pool_data_like_cpp()
            .is_spawned_creature_like_cpp(pool_spawn_id)
    );
    assert_eq!(
        map.get_respawn_time_like_cpp(SpawnObjectType::Creature, creature_spawn_id),
        0
    );
    assert_eq!(
        map.get_respawn_time_like_cpp(SpawnObjectType::GameObject, gameobject_spawn_id),
        0
    );
    let drained = manager
        .find_map_mut(1, 0)
        .expect("test map")
        .map_mut()
        .remove_all_objects_in_remove_list_like_cpp();
    assert_eq!(drained.removed, 2);
}

#[test]
fn game_event_unspawn_for_event_missing_creature_bucket_skips_gameobjects_and_pool_like_cpp() {
    let mut manager = wow_map::MapManager::default();
    manager.create_world_map(1, 0);
    let event_id = 99;
    let pool_id = 536201;
    let pool_spawn_id = 536202;
    let gameobject_spawn_id = 536203;
    let mut store = SpawnStore::new();
    add_spawn_data_like_cpp(
        &mut store,
        SpawnObjectType::GameObject,
        gameobject_spawn_id,
        1,
    );
    let guids = push_game_event_guid_for_test_like_cpp(
        spawn_store_loader::GameEventSpawnGuidsLikeCpp::from_game_event_max_entry_like_cpp(Some(2)),
        SpawnObjectType::GameObject,
        1,
        gameobject_spawn_id,
    );
    let game_event_pools =
        spawn_store_loader::GameEventPoolIdsLikeCpp::from_game_event_max_entry_like_cpp(Some(100))
            .with_pool_ids_for_event_like_cpp(event_id, [pool_id]);
    let metadata = spawn_store_loader::CanonicalSpawnMetadataLikeCpp::new(store, BTreeMap::new())
        .with_pool_mgr_like_cpp(pool_mgr_with_creature_pool_like_cpp(
            pool_id,
            1,
            pool_spawn_id,
        ))
        .with_game_event_pools_like_cpp(game_event_pools)
        .with_game_event_spawn_guids_like_cpp(guids);
    let map = manager.find_map_mut(1, 0).expect("test map").map_mut();
    map.add_respawn_info_like_cpp(respawn_info_like_cpp(
        SpawnObjectType::GameObject,
        gameobject_spawn_id,
        536200,
    ));
    map.add_respawn_info_like_cpp(respawn_info_like_cpp(
        SpawnObjectType::Creature,
        pool_spawn_id,
        536200,
    ));
    map.pool_data_mut_like_cpp()
        .add_spawn_like_cpp(SpawnObjectType::Creature, pool_spawn_id, pool_id)
        .expect("test spawned creature pool data");

    let summary = game_event_unspawn_for_event_like_cpp(&mut manager, &metadata, &[], event_id);

    assert_eq!(summary.event_id, event_id);
    assert!(summary.non_pool.missing_event_creature_guids);
    assert!(!summary.non_pool.missing_event_gameobject_guids);
    assert_eq!(summary.non_pool.gameobject.guids_seen, 0);
    assert!(summary.pool_skipped_due_to_non_pool_bucket);
    assert!(!summary.pool.missing_event_pool_ids);
    assert_eq!(summary.pool.pool_summary.event_pool_ids_seen, 0);
    let map = manager.find_map(1, 0).expect("test map").map();
    assert!(
        map.respawn_timer_keys_like_cpp()
            .any(|(_, spawn_id)| spawn_id == gameobject_spawn_id)
    );
    assert!(
        map.pool_data_like_cpp()
            .is_spawned_creature_like_cpp(pool_spawn_id)
    );
    assert!(
        map.respawn_timer_keys_like_cpp()
            .any(|(_, spawn_id)| spawn_id == pool_spawn_id)
    );
}

#[test]
fn game_event_unspawn_for_event_missing_pool_bucket_keeps_non_pool_effects_like_cpp() {
    let mut manager = wow_map::MapManager::default();
    manager.create_world_map(1, 0);
    let event_id = 99;
    let creature_spawn_id = 536301;
    let gameobject_spawn_id = 536302;
    let mut store = SpawnStore::new();
    add_spawn_data_like_cpp(&mut store, SpawnObjectType::Creature, creature_spawn_id, 1);
    add_spawn_data_like_cpp(
        &mut store,
        SpawnObjectType::GameObject,
        gameobject_spawn_id,
        1,
    );
    let mut guids =
        spawn_store_loader::GameEventSpawnGuidsLikeCpp::from_game_event_max_entry_like_cpp(Some(
            100,
        ));
    guids = push_game_event_guid_for_test_like_cpp(
        guids,
        SpawnObjectType::Creature,
        event_id,
        creature_spawn_id,
    );
    guids = push_game_event_guid_for_test_like_cpp(
        guids,
        SpawnObjectType::GameObject,
        event_id,
        gameobject_spawn_id,
    );
    let metadata = spawn_store_loader::CanonicalSpawnMetadataLikeCpp::new(store, BTreeMap::new())
        .with_game_event_pools_like_cpp(
            spawn_store_loader::GameEventPoolIdsLikeCpp::from_game_event_max_entry_like_cpp(Some(
                2,
            )),
        )
        .with_game_event_spawn_guids_like_cpp(guids);
    let map = manager.find_map_mut(1, 0).expect("test map").map_mut();
    map.add_respawn_info_like_cpp(respawn_info_like_cpp(
        SpawnObjectType::Creature,
        creature_spawn_id,
        536300,
    ));
    map.add_respawn_info_like_cpp(respawn_info_like_cpp(
        SpawnObjectType::GameObject,
        gameobject_spawn_id,
        536300,
    ));
    insert_live_creature_for_spawn_like_cpp(&mut manager, 1, creature_spawn_id, 5363011);
    insert_live_gameobject_for_spawn_like_cpp(&mut manager, 1, gameobject_spawn_id, 5363021);

    let summary = game_event_unspawn_for_event_like_cpp(&mut manager, &metadata, &[], event_id);

    assert!(!summary.pool_skipped_due_to_non_pool_bucket);
    assert_eq!(summary.non_pool.creature.respawn_timers_removed, 1);
    assert_eq!(summary.non_pool.creature.live_objects_queued, 1);
    assert_eq!(summary.non_pool.gameobject.respawn_timers_removed, 1);
    assert_eq!(summary.non_pool.gameobject.live_objects_queued, 1);
    assert!(summary.pool.missing_event_pool_ids);
    assert_eq!(summary.pool.pool_summary.event_pool_ids_seen, 0);
    let map = manager.find_map(1, 0).expect("test map").map();
    assert_eq!(
        map.get_respawn_time_like_cpp(SpawnObjectType::Creature, creature_spawn_id),
        0
    );
    assert_eq!(
        map.get_respawn_time_like_cpp(SpawnObjectType::GameObject, gameobject_spawn_id),
        0
    );
}

#[test]
fn game_event_spawn_non_pool_creature_and_gameobject_loaded_grid_adds_records_like_cpp() {
    let mut manager = wow_map::MapManager::default();
    let map = manager.create_world_map(571, 0);
    assert!(map.map_mut().load_grid(0.0, 0.0));
    let legacy_manager: wow_world::SharedMapManager =
        Arc::new(std::sync::RwLock::new(wow_world::MapManager::new()));
    let event_id = 1;
    let creature_spawn_id = 535101;
    let gameobject_spawn_id = 535201;
    let creature_entry = 42;
    let gameobject_entry = 9001;
    let mut store = SpawnStore::new();
    store.add_object_spawn(
        &game_event_spawn_test_spawn_data_like_cpp(
            SpawnObjectType::Creature,
            creature_spawn_id,
            571,
            creature_entry,
            0.0,
            0.0,
            120,
        ),
        |_| false,
    );
    store.add_object_spawn(
        &game_event_spawn_test_spawn_data_like_cpp(
            SpawnObjectType::GameObject,
            gameobject_spawn_id,
            571,
            gameobject_entry,
            0.0,
            0.0,
            30,
        ),
        |_| false,
    );
    let mut guids =
        spawn_store_loader::GameEventSpawnGuidsLikeCpp::from_game_event_max_entry_like_cpp(Some(3));
    guids = push_game_event_guid_for_test_like_cpp(
        guids,
        SpawnObjectType::Creature,
        event_id,
        creature_spawn_id,
    );
    guids = push_game_event_guid_for_test_like_cpp(
        guids,
        SpawnObjectType::GameObject,
        event_id,
        gameobject_spawn_id,
    );
    let metadata = spawn_store_loader::CanonicalSpawnMetadataLikeCpp::new(store, BTreeMap::new())
        .with_game_event_spawn_guids_like_cpp(guids)
        .with_creature_runtime_rows_like_cpp(BTreeMap::from([(
            creature_spawn_id,
            spawn_store_loader::CreatureSpawnRuntimeRowLikeCpp {
                spawn_id: creature_spawn_id,
                model_id: 999,
                equipment_id: 3,
                wander_distance: 15.0,
                curhealth: 0,
                curmana: 0,
                movement_type: 1,
                npc_flags: None,
                unit_flags: None,
                unit_flags2: None,
                unit_flags3: None,
                ground_movement_type: wow_constants::CreatureGroundMovementType::Run as u8,
                swim_allowed: true,
                flight_movement_type: 0,
                rooted: false,
                chase_movement_type: wow_constants::CreatureChaseMovementType::Run as u8,
                random_movement_type: wow_constants::CreatureRandomMovementType::Walk as u8,
                interaction_pause_timer_ms:
                    wow_entities::DEFAULT_CREATURE_INTERACTION_PAUSE_TIMER_MS_LIKE_CPP,
                string_id: "game-event-spawn-creature".to_string(),
                spawn_time_secs: 120,
            },
        )]))
        .with_gameobject_runtime_rows_like_cpp(BTreeMap::from([(
            gameobject_spawn_id,
            spawn_store_loader::GameObjectSpawnRuntimeRowLikeCpp {
                spawn_id: gameobject_spawn_id,
                rotation: [0.0, 0.0, 0.0, 1.0],
                anim_progress: 55,
                state: 1,
                string_id: "game-event-spawn-go".to_string(),
                spawn_time_secs: 30,
            },
        )]));
    let caches = game_event_spawn_test_caches_like_cpp(creature_entry, gameobject_entry);
    let map = manager.find_map_mut(571, 0).expect("test map").map_mut();
    map.add_respawn_info_like_cpp(respawn_info_like_cpp(
        SpawnObjectType::Creature,
        creature_spawn_id,
        535000,
    ));
    map.add_respawn_info_like_cpp(respawn_info_like_cpp(
        SpawnObjectType::GameObject,
        gameobject_spawn_id,
        535000,
    ));

    let summary = game_event_spawn_for_event_like_cpp(
        &mut manager,
        Some(&legacy_manager),
        &metadata,
        &caches,
        event_id,
    );

    assert_eq!(summary.event_id, event_id);
    assert!(!summary.non_pool.missing_event_creature_guids);
    assert!(!summary.non_pool.missing_event_gameobject_guids);
    assert_eq!(summary.non_pool.creature.guids_seen, 1);
    assert_eq!(summary.non_pool.creature.respawn_timers_removed, 1);
    assert_eq!(summary.non_pool.creature.load_attempts, 1);
    assert_eq!(summary.non_pool.creature.successful_loaded_grid_spawns, 1);
    assert_eq!(summary.non_pool.creature.legacy_creature_mirrors, 1);
    assert_eq!(summary.non_pool.gameobject.guids_seen, 1);
    assert_eq!(summary.non_pool.gameobject.respawn_timers_removed, 1);
    assert_eq!(summary.non_pool.gameobject.load_attempts, 1);
    assert_eq!(summary.non_pool.gameobject.successful_loaded_grid_spawns, 1);
    assert!(summary.pool.missing_event_pool_ids);
    let map = manager.find_map(571, 0).expect("test map").map();
    assert_eq!(
        map.get_respawn_time_like_cpp(SpawnObjectType::Creature, creature_spawn_id),
        0
    );
    assert_eq!(
        map.get_respawn_time_like_cpp(SpawnObjectType::GameObject, gameobject_spawn_id),
        0
    );
    let creature = map
        .get_creature_by_spawn_id_like_cpp(creature_spawn_id)
        .expect("GameEventSpawn should add loaded-grid Creature");
    assert_eq!(creature.respawn_time(), 0);
    assert!(
        legacy_manager
            .read()
            .unwrap()
            .find_creature(571, 0, creature.guid())
            .is_some(),
        "Rust split runtime must mirror C++ AddToMap-loaded creatures into the legacy tick manager"
    );
    let gameobject = map
        .get_gameobject_by_spawn_id_like_cpp(gameobject_spawn_id)
        .expect("GameEventSpawn should add spawned-by-default GameObject");
    assert_eq!(gameobject.respawn_time(), 0);
    assert!(gameobject.spawned_by_default());
}

#[test]
fn game_event_spawn_for_event_missing_creature_bucket_skips_gameobjects_and_pool_like_cpp() {
    let mut manager = wow_map::MapManager::default();
    manager.create_world_map(1, 0);
    let event_id = 99;
    let pool_id = 535901;
    let pool_spawn_id = 535902;
    let gameobject_spawn_id = 535903;
    let mut store = SpawnStore::new();
    add_spawn_data_like_cpp(&mut store, SpawnObjectType::Creature, pool_spawn_id, 1);
    add_spawn_data_like_cpp(
        &mut store,
        SpawnObjectType::GameObject,
        gameobject_spawn_id,
        1,
    );
    let mut guids =
        spawn_store_loader::GameEventSpawnGuidsLikeCpp::from_game_event_max_entry_like_cpp(Some(2));
    guids = push_game_event_guid_for_test_like_cpp(
        guids,
        SpawnObjectType::GameObject,
        1,
        gameobject_spawn_id,
    );
    let game_event_pools =
        spawn_store_loader::GameEventPoolIdsLikeCpp::from_game_event_max_entry_like_cpp(Some(100))
            .with_pool_ids_for_event_like_cpp(event_id, [pool_id]);
    let metadata = spawn_store_loader::CanonicalSpawnMetadataLikeCpp::new(store, BTreeMap::new())
        .with_pool_mgr_like_cpp(pool_mgr_with_creature_pool_like_cpp(
            pool_id,
            1,
            pool_spawn_id,
        ))
        .with_game_event_pools_like_cpp(game_event_pools)
        .with_game_event_spawn_guids_like_cpp(guids);
    let caches = empty_loaded_grid_creature_respawn_caches_like_cpp();

    let summary =
        game_event_spawn_for_event_like_cpp(&mut manager, None, &metadata, &caches, event_id);

    assert_eq!(summary.event_id, event_id);
    assert!(summary.non_pool.missing_event_creature_guids);
    assert!(!summary.non_pool.missing_event_gameobject_guids);
    assert_eq!(summary.non_pool.gameobject.guids_seen, 0);
    assert!(summary.pool_skipped_due_to_non_pool_bucket);
    assert!(!summary.pool.missing_event_pool_ids);
    assert_eq!(summary.pool.pool_summary.event_pool_ids_seen, 0);
    let map = manager.find_map(1, 0).expect("test map").map();
    assert!(
        !map.pool_data_like_cpp()
            .is_spawned_creature_like_cpp(pool_spawn_id)
    );
    assert_eq!(
        map.get_respawn_time_like_cpp(SpawnObjectType::GameObject, gameobject_spawn_id),
        0
    );
}

#[test]
fn game_event_spawn_for_event_missing_gameobject_bucket_skips_pool_after_creatures_like_cpp() {
    let mut manager = wow_map::MapManager::default();
    let map = manager.create_world_map(571, 0);
    assert!(map.map_mut().load_grid(0.0, 0.0));
    manager.create_world_map(1, 0);
    let event_id = 7;
    let creature_spawn_id = 535904;
    let pool_id = 535905;
    let pool_spawn_id = 535906;
    let creature_entry = 42;
    let mut store = SpawnStore::new();
    store.add_object_spawn(
        &game_event_spawn_test_spawn_data_like_cpp(
            SpawnObjectType::Creature,
            creature_spawn_id,
            571,
            creature_entry,
            0.0,
            0.0,
            120,
        ),
        |_| false,
    );
    add_spawn_data_like_cpp(&mut store, SpawnObjectType::Creature, pool_spawn_id, 1);
    let mut guids =
        spawn_store_loader::GameEventSpawnGuidsLikeCpp::from_game_event_max_entry_like_cpp(Some(
            10,
        ));
    guids = push_game_event_guid_for_test_like_cpp(
        guids,
        SpawnObjectType::Creature,
        event_id,
        creature_spawn_id,
    )
    .truncate_gameobject_guid_buckets_for_test_like_cpp(17);
    let game_event_pools =
        spawn_store_loader::GameEventPoolIdsLikeCpp::from_game_event_max_entry_like_cpp(Some(10))
            .with_pool_ids_for_event_like_cpp(event_id, [pool_id]);
    let metadata = spawn_store_loader::CanonicalSpawnMetadataLikeCpp::new(store, BTreeMap::new())
        .with_pool_mgr_like_cpp(pool_mgr_with_creature_pool_like_cpp(
            pool_id,
            1,
            pool_spawn_id,
        ))
        .with_game_event_pools_like_cpp(game_event_pools)
        .with_game_event_spawn_guids_like_cpp(guids)
        .with_creature_runtime_rows_like_cpp(BTreeMap::from([(
            creature_spawn_id,
            spawn_store_loader::CreatureSpawnRuntimeRowLikeCpp {
                spawn_id: creature_spawn_id,
                model_id: 999,
                equipment_id: 3,
                wander_distance: 15.0,
                curhealth: 0,
                curmana: 0,
                movement_type: 1,
                npc_flags: None,
                unit_flags: None,
                unit_flags2: None,
                unit_flags3: None,
                ground_movement_type: wow_constants::CreatureGroundMovementType::Run as u8,
                swim_allowed: true,
                flight_movement_type: 0,
                rooted: false,
                chase_movement_type: wow_constants::CreatureChaseMovementType::Run as u8,
                random_movement_type: wow_constants::CreatureRandomMovementType::Walk as u8,
                interaction_pause_timer_ms:
                    wow_entities::DEFAULT_CREATURE_INTERACTION_PAUSE_TIMER_MS_LIKE_CPP,
                string_id: "game-event-spawn-creature-before-missing-go".to_string(),
                spawn_time_secs: 120,
            },
        )]));
    let caches = game_event_spawn_test_caches_like_cpp(creature_entry, 9001);
    manager
        .find_map_mut(571, 0)
        .expect("test map")
        .map_mut()
        .add_respawn_info_like_cpp(respawn_info_like_cpp(
            SpawnObjectType::Creature,
            creature_spawn_id,
            535000,
        ));

    let summary =
        game_event_spawn_for_event_like_cpp(&mut manager, None, &metadata, &caches, event_id);

    assert_eq!(summary.event_id, event_id);
    assert!(!summary.non_pool.missing_event_creature_guids);
    assert!(summary.non_pool.missing_event_gameobject_guids);
    assert_eq!(summary.non_pool.creature.guids_seen, 1);
    assert_eq!(summary.non_pool.creature.respawn_timers_removed, 1);
    assert_eq!(summary.non_pool.creature.successful_loaded_grid_spawns, 1);
    assert_eq!(summary.non_pool.gameobject.guids_seen, 0);
    assert!(summary.pool_skipped_due_to_non_pool_bucket);
    assert!(!summary.pool.missing_event_pool_ids);
    assert_eq!(summary.pool.pool_summary.event_pool_ids_seen, 0);
    let creature_map = manager.find_map(571, 0).expect("creature map").map();
    assert!(
        creature_map
            .get_creature_by_spawn_id_like_cpp(creature_spawn_id)
            .is_some()
    );
    let pool_map = manager.find_map(1, 0).expect("pool map").map();
    assert!(
        !pool_map
            .pool_data_like_cpp()
            .is_spawned_creature_like_cpp(pool_spawn_id)
    );
}

#[test]
fn game_event_spawn_non_pool_unloaded_grid_removes_timer_without_fabricating_like_cpp() {
    let mut manager = wow_map::MapManager::default();
    manager.create_world_map(571, 0);
    let event_id = 1;
    let spawn_id = 535301;
    let entry = 42;
    let mut store = SpawnStore::new();
    store.add_object_spawn(
        &game_event_spawn_test_spawn_data_like_cpp(
            SpawnObjectType::Creature,
            spawn_id,
            571,
            entry,
            1_000.0,
            1_000.0,
            120,
        ),
        |_| false,
    );
    let guids = push_game_event_guid_for_test_like_cpp(
        spawn_store_loader::GameEventSpawnGuidsLikeCpp::from_game_event_max_entry_like_cpp(Some(2)),
        SpawnObjectType::Creature,
        event_id,
        spawn_id,
    );
    let metadata = spawn_store_loader::CanonicalSpawnMetadataLikeCpp::new(store, BTreeMap::new())
        .with_game_event_spawn_guids_like_cpp(guids)
        .with_creature_runtime_rows_like_cpp(BTreeMap::from([(
            spawn_id,
            spawn_store_loader::CreatureSpawnRuntimeRowLikeCpp {
                spawn_id,
                model_id: 999,
                equipment_id: 3,
                wander_distance: 15.0,
                curhealth: 0,
                curmana: 0,
                movement_type: 1,
                npc_flags: None,
                unit_flags: None,
                unit_flags2: None,
                unit_flags3: None,
                ground_movement_type: wow_constants::CreatureGroundMovementType::Run as u8,
                swim_allowed: true,
                flight_movement_type: 0,
                rooted: false,
                chase_movement_type: wow_constants::CreatureChaseMovementType::Run as u8,
                random_movement_type: wow_constants::CreatureRandomMovementType::Walk as u8,
                interaction_pause_timer_ms:
                    wow_entities::DEFAULT_CREATURE_INTERACTION_PAUSE_TIMER_MS_LIKE_CPP,
                string_id: "game-event-unloaded-creature".to_string(),
                spawn_time_secs: 120,
            },
        )]));
    let caches = game_event_spawn_test_caches_like_cpp(entry, 9001);
    manager
        .find_map_mut(571, 0)
        .expect("test map")
        .map_mut()
        .add_respawn_info_like_cpp(respawn_info_like_cpp(
            SpawnObjectType::Creature,
            spawn_id,
            535000,
        ));

    let summary = game_event_spawn_creatures_and_gameobjects_for_event_like_cpp(
        &mut manager,
        None,
        &metadata,
        &caches,
        event_id,
    );

    assert_eq!(summary.creature.guids_seen, 1);
    assert_eq!(summary.creature.maps_matched, 1);
    assert_eq!(summary.creature.respawn_timers_removed, 1);
    assert_eq!(summary.creature.unloaded_grid_skips, 1);
    assert_eq!(summary.creature.load_attempts, 0);
    assert_eq!(summary.creature.successful_loaded_grid_spawns, 0);
    let map = manager.find_map(571, 0).expect("test map").map();
    assert_eq!(
        map.get_respawn_time_like_cpp(SpawnObjectType::Creature, spawn_id),
        0
    );
    assert!(map.get_creature_by_spawn_id_like_cpp(spawn_id).is_none());
}

#[test]
fn game_event_spawn_missing_creature_bucket_returns_before_gameobjects_like_cpp() {
    let mut manager = wow_map::MapManager::default();
    let map = manager.create_world_map(571, 0);
    assert!(map.map_mut().load_grid(0.0, 0.0));
    let event_id = 99;
    let gameobject_spawn_id = 535401;
    let gameobject_entry = 9001;
    let mut store = SpawnStore::new();
    store.add_object_spawn(
        &game_event_spawn_test_spawn_data_like_cpp(
            SpawnObjectType::GameObject,
            gameobject_spawn_id,
            571,
            gameobject_entry,
            0.0,
            0.0,
            30,
        ),
        |_| false,
    );
    let guids = push_game_event_guid_for_test_like_cpp(
        spawn_store_loader::GameEventSpawnGuidsLikeCpp::from_game_event_max_entry_like_cpp(Some(2)),
        SpawnObjectType::GameObject,
        1,
        gameobject_spawn_id,
    );
    let metadata = spawn_store_loader::CanonicalSpawnMetadataLikeCpp::new(store, BTreeMap::new())
        .with_game_event_spawn_guids_like_cpp(guids)
        .with_gameobject_runtime_rows_like_cpp(BTreeMap::from([(
            gameobject_spawn_id,
            spawn_store_loader::GameObjectSpawnRuntimeRowLikeCpp {
                spawn_id: gameobject_spawn_id,
                rotation: [0.0, 0.0, 0.0, 1.0],
                anim_progress: 55,
                state: 1,
                string_id: "game-event-missing-creature-bucket-go".to_string(),
                spawn_time_secs: 30,
            },
        )]));
    let caches = game_event_spawn_test_caches_like_cpp(42, gameobject_entry);
    manager
        .find_map_mut(571, 0)
        .expect("test map")
        .map_mut()
        .add_respawn_info_like_cpp(respawn_info_like_cpp(
            SpawnObjectType::GameObject,
            gameobject_spawn_id,
            535000,
        ));

    let summary = game_event_spawn_creatures_and_gameobjects_for_event_like_cpp(
        &mut manager,
        None,
        &metadata,
        &caches,
        event_id,
    );

    assert_eq!(summary.event_id, event_id);
    assert!(summary.missing_event_creature_guids);
    assert_eq!(summary.gameobject.guids_seen, 0);
    let map = manager.find_map(571, 0).expect("test map").map();
    assert_eq!(
        map.get_respawn_time_like_cpp(SpawnObjectType::GameObject, gameobject_spawn_id),
        535000
    );
    assert!(
        map.get_gameobject_by_spawn_id_like_cpp(gameobject_spawn_id)
            .is_none()
    );
}

#[test]
fn game_event_spawn_non_pool_gameobject_not_spawned_by_default_is_not_added_like_cpp() {
    let mut manager = wow_map::MapManager::default();
    let map = manager.create_world_map(571, 0);
    assert!(map.map_mut().load_grid(0.0, 0.0));
    let event_id = 1;
    let spawn_id = 535501;
    let entry = 9001;
    let mut store = SpawnStore::new();
    store.add_object_spawn(
        &game_event_spawn_test_spawn_data_like_cpp(
            SpawnObjectType::GameObject,
            spawn_id,
            571,
            entry,
            0.0,
            0.0,
            -30,
        ),
        |_| false,
    );
    let guids = push_game_event_guid_for_test_like_cpp(
        spawn_store_loader::GameEventSpawnGuidsLikeCpp::from_game_event_max_entry_like_cpp(Some(2)),
        SpawnObjectType::GameObject,
        event_id,
        spawn_id,
    );
    let metadata = spawn_store_loader::CanonicalSpawnMetadataLikeCpp::new(store, BTreeMap::new())
        .with_game_event_spawn_guids_like_cpp(guids)
        .with_gameobject_runtime_rows_like_cpp(BTreeMap::from([(
            spawn_id,
            spawn_store_loader::GameObjectSpawnRuntimeRowLikeCpp {
                spawn_id,
                rotation: [0.0, 0.0, 0.0, 1.0],
                anim_progress: 55,
                state: 1,
                string_id: "game-event-go-not-default".to_string(),
                spawn_time_secs: -30,
            },
        )]));
    let caches = game_event_spawn_test_caches_like_cpp(42, entry);
    manager
        .find_map_mut(571, 0)
        .expect("test map")
        .map_mut()
        .add_respawn_info_like_cpp(respawn_info_like_cpp(
            SpawnObjectType::GameObject,
            spawn_id,
            535000,
        ));

    let summary = game_event_spawn_creatures_and_gameobjects_for_event_like_cpp(
        &mut manager,
        None,
        &metadata,
        &caches,
        event_id,
    );

    assert_eq!(summary.gameobject.guids_seen, 1);
    assert_eq!(summary.gameobject.respawn_timers_removed, 1);
    assert_eq!(summary.gameobject.load_attempts, 1);
    assert_eq!(
        summary.gameobject.gameobject_not_spawned_by_default_skips,
        1
    );
    assert_eq!(summary.gameobject.successful_loaded_grid_spawns, 0);
    let map = manager.find_map(571, 0).expect("test map").map();
    assert_eq!(
        map.get_respawn_time_like_cpp(SpawnObjectType::GameObject, spawn_id),
        0
    );
    assert!(map.get_gameobject_by_spawn_id_like_cpp(spawn_id).is_none());
}

#[test]
fn game_event_pool_spawn_uses_canonical_event_pool_ids_like_cpp() {
    let mut manager = wow_map::MapManager::default();
    manager.create_world_map(1, 0);
    manager.create_world_map(2, 0);
    let event_id = 7;
    let pool_id = 5321;
    let spawn_id = 532101;
    let mut store = SpawnStore::new();
    store.add_object_spawn(
        &SpawnData {
            object_type: SpawnObjectType::Creature,
            spawn_id,
            map_id: 1,
            db_data: true,
            spawn_group: SpawnGroupTemplateData {
                group_id: 5321,
                name: "game-event-canonical-spawn".to_string(),
                map_id: 1,
                flags: SpawnGroupFlags::NONE,
            },
            id: 99,
            spawn_point: SpawnPosition::new(1_000.0, 1_000.0, 0.0, 0.0),
            phase_use_flags: 0,
            phase_id: 0,
            phase_group: 0,
            terrain_swap_map: 0,
            pool_id,
            spawn_time_secs: 0,
            spawn_difficulties: vec![1],
            script_id: 0,
            string_id: String::new(),
        },
        |_| false,
    );
    let game_event_pools =
        spawn_store_loader::GameEventPoolIdsLikeCpp::from_game_event_max_entry_like_cpp(Some(10))
            .with_pool_ids_for_event_like_cpp(event_id, [pool_id]);
    let metadata = canonical_spawn_metadata_with_store_pool_mgr_and_game_event_pools_like_cpp(
        store,
        pool_mgr_with_creature_pool_like_cpp(pool_id, 1, spawn_id),
        game_event_pools,
    );
    let caches = empty_loaded_grid_creature_respawn_caches_like_cpp();

    let summary =
        game_event_spawn_pools_for_event_like_cpp(&mut manager, None, &metadata, &caches, event_id);

    assert_eq!(summary.event_id, event_id);
    assert!(!summary.missing_event_pool_ids);
    assert_eq!(summary.pool_summary.event_pool_ids_seen, 1);
    assert_eq!(summary.pool_summary.maps_matched, 1);
    assert!(
        manager
            .find_map(1, 0)
            .expect("test map 1")
            .map()
            .pool_data_like_cpp()
            .is_spawned_creature_like_cpp(spawn_id)
    );
    assert!(
        !manager
            .find_map(2, 0)
            .expect("test map 2")
            .map()
            .pool_data_like_cpp()
            .is_spawned_creature_like_cpp(spawn_id)
    );
}

#[test]
fn game_event_pool_unspawn_uses_canonical_event_pool_ids_like_cpp() {
    let mut manager = wow_map::MapManager::default();
    manager.create_world_map(1, 0);
    manager.create_world_map(2, 0);
    let event_id = 8;
    let pool_id = 5322;
    let spawn_id = 532201;
    let game_event_pools =
        spawn_store_loader::GameEventPoolIdsLikeCpp::from_game_event_max_entry_like_cpp(Some(10))
            .with_pool_ids_for_event_like_cpp(event_id, [pool_id]);
    let metadata = canonical_spawn_metadata_with_store_pool_mgr_and_game_event_pools_like_cpp(
        SpawnStore::new(),
        pool_mgr_with_creature_pool_like_cpp(pool_id, 1, spawn_id),
        game_event_pools,
    );
    for map_id in [1, 2] {
        let map = manager
            .find_map_mut(map_id, 0)
            .expect("test canonical map")
            .map_mut();
        map.add_respawn_info_like_cpp(respawn_info_like_cpp(
            SpawnObjectType::Creature,
            spawn_id,
            532200,
        ));
        map.pool_data_mut_like_cpp()
            .add_spawn_like_cpp(SpawnObjectType::Creature, spawn_id, pool_id)
            .expect("test spawned creature pool data");
    }

    let summary = game_event_unspawn_pools_for_event_like_cpp(&mut manager, &metadata, event_id);

    assert_eq!(summary.event_id, event_id);
    assert!(!summary.missing_event_pool_ids);
    assert_eq!(summary.pool_summary.event_pool_ids_seen, 1);
    assert_eq!(summary.pool_summary.maps_matched, 1);
    assert!(
        !manager
            .find_map(1, 0)
            .expect("test map 1")
            .map()
            .pool_data_like_cpp()
            .is_spawned_creature_like_cpp(spawn_id)
    );
    assert!(
        manager
            .find_map(2, 0)
            .expect("test map 2")
            .map()
            .pool_data_like_cpp()
            .is_spawned_creature_like_cpp(spawn_id)
    );
}

#[test]
fn game_event_pool_missing_event_id_is_noop_like_cpp() {
    let mut manager = wow_map::MapManager::default();
    manager.create_world_map(1, 0);
    let pool_id = 5323;
    let spawn_id = 532301;
    let metadata = canonical_spawn_metadata_with_store_pool_mgr_and_game_event_pools_like_cpp(
        SpawnStore::new(),
        pool_mgr_with_creature_pool_like_cpp(pool_id, 1, spawn_id),
        spawn_store_loader::GameEventPoolIdsLikeCpp::from_game_event_max_entry_like_cpp(Some(2)),
    );
    manager
        .find_map_mut(1, 0)
        .expect("test map")
        .map_mut()
        .pool_data_mut_like_cpp()
        .add_spawn_like_cpp(SpawnObjectType::Creature, spawn_id, pool_id)
        .expect("test spawned creature pool data");
    let caches = empty_loaded_grid_creature_respawn_caches_like_cpp();

    let spawn_summary =
        game_event_spawn_pools_for_event_like_cpp(&mut manager, None, &metadata, &caches, 99);
    let unspawn_summary = game_event_unspawn_pools_for_event_like_cpp(&mut manager, &metadata, 99);

    assert!(spawn_summary.missing_event_pool_ids);
    assert_eq!(spawn_summary.pool_summary.event_pool_ids_seen, 0);
    assert!(unspawn_summary.missing_event_pool_ids);
    assert_eq!(unspawn_summary.pool_summary.event_pool_ids_seen, 0);
    assert!(
        manager
            .find_map(1, 0)
            .expect("test map")
            .map()
            .pool_data_like_cpp()
            .is_spawned_creature_like_cpp(spawn_id)
    );
}

#[test]
fn game_event_pool_empty_event_id_list_is_noop_like_cpp() {
    let mut manager = wow_map::MapManager::default();
    manager.create_world_map(1, 0);
    let event_id = 1;
    let game_event_pools =
        spawn_store_loader::GameEventPoolIdsLikeCpp::from_game_event_max_entry_like_cpp(Some(2));
    let metadata = canonical_spawn_metadata_with_store_pool_mgr_and_game_event_pools_like_cpp(
        SpawnStore::new(),
        PoolMgrLikeCpp::new(),
        game_event_pools,
    );
    let caches = empty_loaded_grid_creature_respawn_caches_like_cpp();

    let spawn_summary =
        game_event_spawn_pools_for_event_like_cpp(&mut manager, None, &metadata, &caches, event_id);
    let unspawn_summary =
        game_event_unspawn_pools_for_event_like_cpp(&mut manager, &metadata, event_id);

    assert!(!spawn_summary.missing_event_pool_ids);
    assert_eq!(spawn_summary.pool_summary.event_pool_ids_seen, 0);
    assert!(!unspawn_summary.missing_event_pool_ids);
    assert_eq!(unspawn_summary.pool_summary.event_pool_ids_seen, 0);
    assert_eq!(spawn_summary.pool_summary.maps_matched, 0);
    assert_eq!(unspawn_summary.pool_summary.maps_matched, 0);
}

#[test]
fn game_event_pool_spawn_filters_by_pool_template_map_id_like_cpp() {
    let mut manager = wow_map::MapManager::default();
    manager.create_world_map(1, 0);
    manager.create_world_map(2, 0);
    let pool_id = 5301;
    let spawn_id = 530101;
    let mut store = SpawnStore::new();
    store.add_object_spawn(
        &SpawnData {
            object_type: SpawnObjectType::Creature,
            spawn_id,
            map_id: 1,
            db_data: true,
            spawn_group: SpawnGroupTemplateData {
                group_id: 5301,
                name: "game-event-spawn".to_string(),
                map_id: 1,
                flags: SpawnGroupFlags::NONE,
            },
            id: 99,
            spawn_point: SpawnPosition::new(1_000.0, 1_000.0, 0.0, 0.0),
            phase_use_flags: 0,
            phase_id: 0,
            phase_group: 0,
            terrain_swap_map: 0,
            pool_id,
            spawn_time_secs: 0,
            spawn_difficulties: vec![1],
            script_id: 0,
            string_id: String::new(),
        },
        |_| false,
    );
    let metadata = canonical_spawn_metadata_with_store_and_pool_mgr_like_cpp(
        store,
        pool_mgr_with_creature_pool_like_cpp(pool_id, 1, spawn_id),
    );
    let caches = empty_loaded_grid_creature_respawn_caches_like_cpp();

    let summary =
        game_event_spawn_pools_like_cpp(&mut manager, None, &metadata, &caches, &[pool_id]);

    assert_eq!(summary.event_pool_ids_seen, 1);
    assert_eq!(summary.missing_pool_templates, 0);
    assert_eq!(summary.maps_matched, 1);
    assert_eq!(summary.pools_without_loaded_canonical_maps, 0);
    assert_eq!(summary.pool_spawn_actions_skipped_unloaded_grid, 1);
    assert_eq!(summary.pool_spawn_actions_blocked_loaded_grid, 0);
    assert!(summary.blocked_pool_plan_errors.is_empty());
    assert!(
        manager
            .find_map(1, 0)
            .expect("test map 1")
            .map()
            .pool_data_like_cpp()
            .is_spawned_creature_like_cpp(spawn_id)
    );
    assert!(
        !manager
            .find_map(2, 0)
            .expect("test map 2")
            .map()
            .pool_data_like_cpp()
            .is_spawned_creature_like_cpp(spawn_id)
    );
}

#[test]
fn game_event_pool_spawn_missing_pool_template_is_counted_noop_like_cpp() {
    let mut manager = wow_map::MapManager::default();
    manager.create_world_map(1, 0);
    let metadata = canonical_spawn_metadata_with_pool_mgr_like_cpp(PoolMgrLikeCpp::new());
    let caches = empty_loaded_grid_creature_respawn_caches_like_cpp();

    let summary = game_event_spawn_pools_like_cpp(&mut manager, None, &metadata, &caches, &[5302]);

    assert_eq!(summary.event_pool_ids_seen, 1);
    assert_eq!(summary.missing_pool_templates, 1);
    assert_eq!(summary.maps_matched, 0);
    assert_eq!(summary.pool_spawn_actions_skipped_unloaded_grid, 0);
    assert!(summary.blocked_pool_plan_errors.is_empty());
    assert!(
        !manager
            .find_map(1, 0)
            .expect("test map")
            .map()
            .pool_data_like_cpp()
            .is_spawned_creature_like_cpp(530201)
    );
}

#[test]
fn game_event_pool_spawn_loaded_grid_records_blocked_loader_and_unloaded_skips_loader_like_cpp() {
    let mut manager = wow_map::MapManager::default();
    manager.create_world_map(1, 0);
    let loaded_spawn_id = 530301;
    let unloaded_spawn_id = 530302;
    let mut store = SpawnStore::new();
    let group = SpawnGroupTemplateData {
        group_id: 5303,
        name: "game-event-spawn-loaded-grid".to_string(),
        map_id: 1,
        flags: SpawnGroupFlags::NONE,
    };
    store.add_object_spawn(
        &SpawnData {
            object_type: SpawnObjectType::Creature,
            spawn_id: loaded_spawn_id,
            map_id: 1,
            db_data: true,
            spawn_group: group.clone(),
            id: 99,
            spawn_point: SpawnPosition::new(0.0, 0.0, 0.0, 0.0),
            phase_use_flags: 0,
            phase_id: 0,
            phase_group: 0,
            terrain_swap_map: 0,
            pool_id: 5303,
            spawn_time_secs: 0,
            spawn_difficulties: vec![1],
            script_id: 0,
            string_id: String::new(),
        },
        |_| false,
    );
    store.add_object_spawn(
        &SpawnData {
            object_type: SpawnObjectType::Creature,
            spawn_id: unloaded_spawn_id,
            map_id: 1,
            db_data: true,
            spawn_group: group,
            id: 99,
            spawn_point: SpawnPosition::new(1_000.0, 1_000.0, 0.0, 0.0),
            phase_use_flags: 0,
            phase_id: 0,
            phase_group: 0,
            terrain_swap_map: 0,
            pool_id: 5303,
            spawn_time_secs: 0,
            spawn_difficulties: vec![1],
            script_id: 0,
            string_id: String::new(),
        },
        |_| false,
    );
    let mut pool_mgr = PoolMgrLikeCpp::new();
    pool_mgr.insert_template_like_cpp(5303, PoolTemplateDataLikeCpp::new(2, 1));
    let mut group = PoolGroupLikeCpp::with_pool_id(PoolMemberKindLikeCpp::Creature, 5303);
    group.add_entry_like_cpp(PoolObjectLikeCpp::new(loaded_spawn_id, 0.0), 2);
    group.add_entry_like_cpp(PoolObjectLikeCpp::new(unloaded_spawn_id, 0.0), 2);
    pool_mgr
        .insert_or_replace_group_like_cpp(PoolMemberKindLikeCpp::Creature, 5303, group)
        .expect("test creature pool group");
    manager
        .find_map_mut(1, 0)
        .expect("test map")
        .map_mut()
        .ensure_grid_loaded(&wow_map::cell_from_world(0.0, 0.0));
    let metadata = canonical_spawn_metadata_with_store_and_pool_mgr_like_cpp(store, pool_mgr);
    let caches = empty_loaded_grid_creature_respawn_caches_like_cpp();

    let summary = game_event_spawn_pools_like_cpp(&mut manager, None, &metadata, &caches, &[5303]);

    assert_eq!(summary.maps_matched, 1);
    assert_eq!(summary.pool_spawn_actions_blocked_loaded_grid, 1);
    assert_eq!(summary.pool_spawn_action_load_plans, 1);
    assert_eq!(summary.pool_spawn_actions_skipped_unloaded_grid, 1);
    assert_eq!(summary.executed_loaded_grid_respawns, 0);
    let map = manager.find_map(1, 0).expect("test map").map();
    assert!(
        map.pool_data_like_cpp()
            .is_spawned_creature_like_cpp(loaded_spawn_id)
    );
    assert!(
        map.pool_data_like_cpp()
            .is_spawned_creature_like_cpp(unloaded_spawn_id)
    );
}

#[test]
fn game_event_pool_unspawn_filters_by_pool_template_map_id_like_cpp() {
    let mut manager = wow_map::MapManager::default();
    manager.create_world_map(1, 0);
    manager.create_world_map(2, 0);
    let pool_id = 5291;
    let spawn_id = 529101;
    let metadata = canonical_spawn_metadata_with_pool_mgr_like_cpp(
        pool_mgr_with_creature_pool_like_cpp(pool_id, 1, spawn_id),
    );

    for map_id in [1, 2] {
        let map = manager
            .find_map_mut(map_id, 0)
            .expect("test canonical map")
            .map_mut();
        map.add_respawn_info_like_cpp(respawn_info_like_cpp(
            SpawnObjectType::Creature,
            spawn_id,
            200,
        ));
        map.pool_data_mut_like_cpp()
            .add_spawn_like_cpp(SpawnObjectType::Creature, spawn_id, pool_id)
            .expect("test spawned creature pool data");
    }

    let summary = game_event_unspawn_pools_like_cpp(&mut manager, &metadata, &[pool_id]);

    assert_eq!(summary.event_pool_ids_seen, 1);
    assert_eq!(summary.missing_pool_templates, 0);
    assert_eq!(summary.maps_matched, 1);
    assert_eq!(summary.pools_without_loaded_canonical_maps, 0);
    assert_eq!(summary.pool_respawn_timers_removed, 0);
    assert_eq!(summary.pool_respawn_timers_missing, 0);
    assert!(summary.blocked_pool_plan_errors.is_empty());
    let map_1 = manager.find_map(1, 0).expect("test map 1").map();
    assert_eq!(
        map_1.get_respawn_time_like_cpp(SpawnObjectType::Creature, spawn_id),
        200
    );
    assert!(
        !map_1
            .pool_data_like_cpp()
            .is_spawned_creature_like_cpp(spawn_id)
    );
    let map_2 = manager.find_map(2, 0).expect("test map 2").map();
    assert_eq!(
        map_2.get_respawn_time_like_cpp(SpawnObjectType::Creature, spawn_id),
        200
    );
    assert!(
        map_2
            .pool_data_like_cpp()
            .is_spawned_creature_like_cpp(spawn_id)
    );
}

#[test]
fn game_event_pool_unspawn_missing_pool_template_is_counted_noop_like_cpp() {
    let mut manager = wow_map::MapManager::default();
    manager.create_world_map(1, 0);
    let spawn_id = 529201;
    let map = manager.find_map_mut(1, 0).expect("test map").map_mut();
    map.add_respawn_info_like_cpp(respawn_info_like_cpp(
        SpawnObjectType::Creature,
        spawn_id,
        300,
    ));
    let metadata = canonical_spawn_metadata_with_pool_mgr_like_cpp(PoolMgrLikeCpp::new());

    let summary = game_event_unspawn_pools_like_cpp(&mut manager, &metadata, &[5292]);

    assert_eq!(summary.event_pool_ids_seen, 1);
    assert_eq!(summary.missing_pool_templates, 1);
    assert_eq!(summary.maps_matched, 0);
    assert_eq!(summary.pool_respawn_timers_removed, 0);
    assert!(summary.blocked_pool_plan_errors.is_empty());
    assert_eq!(
        manager
            .find_map(1, 0)
            .expect("test map")
            .map()
            .get_respawn_time_like_cpp(SpawnObjectType::Creature, spawn_id),
        300
    );
}

#[test]
fn game_event_pool_unspawn_always_delete_removes_non_spawned_member_timer_like_cpp() {
    let mut manager = wow_map::MapManager::default();
    manager.create_world_map(1, 0);
    let pool_id = 5293;
    let spawn_id = 529301;
    let metadata = canonical_spawn_metadata_with_pool_mgr_like_cpp(
        pool_mgr_with_creature_pool_like_cpp(pool_id, 1, spawn_id),
    );
    manager
        .find_map_mut(1, 0)
        .expect("test map")
        .map_mut()
        .add_respawn_info_like_cpp(respawn_info_like_cpp(
            SpawnObjectType::Creature,
            spawn_id,
            400,
        ));

    let summary = game_event_unspawn_pools_like_cpp(&mut manager, &metadata, &[pool_id]);

    assert_eq!(summary.maps_matched, 1);
    assert_eq!(summary.pool_objects_removed, 0);
    assert_eq!(summary.pool_respawn_timers_removed, 1);
    assert_eq!(summary.pool_respawn_timers_missing, 0);
    assert_eq!(
        manager
            .find_map(1, 0)
            .expect("test map")
            .map()
            .get_respawn_time_like_cpp(SpawnObjectType::Creature, spawn_id),
        0
    );
}

#[test]
fn worldserver_cli_defaults_match_cpp_startup_options() {
    let cli = WorldServerCliLikeCpp::parse_from(Vec::<String>::new());

    assert_eq!(cli.config_file, None);
    assert_eq!(cli.config_dir, PathBuf::from("worldserver.conf.d"));
    assert!(!cli.update_databases_only);
    assert!(!cli.show_help);
    assert!(!cli.show_version);
}

#[test]
fn worldserver_cli_parses_short_and_long_options_like_cpp() {
    let cli = WorldServerCliLikeCpp::parse_from(
        [
            "--unknown",
            "--config",
            "/tmp/world.conf",
            "-cd",
            "/tmp/world.conf.d",
            "-u",
        ]
        .into_iter()
        .map(str::to_string),
    );

    assert_eq!(cli.config_file, Some(PathBuf::from("/tmp/world.conf")));
    assert_eq!(cli.config_dir, PathBuf::from("/tmp/world.conf.d"));
    assert!(cli.update_databases_only);

    let cli = WorldServerCliLikeCpp::parse_from(
        [
            "--config=/etc/rustycore/worldserver.conf",
            "--config-dir=/etc/rustycore/worldserver.conf.d",
            "--help",
            "--version",
        ]
        .into_iter()
        .map(str::to_string),
    );

    assert_eq!(
        cli.config_file,
        Some(PathBuf::from("/etc/rustycore/worldserver.conf"))
    );
    assert_eq!(
        cli.config_dir,
        PathBuf::from("/etc/rustycore/worldserver.conf.d")
    );
    assert!(cli.show_help);
    assert!(cli.show_version);
}

#[test]
fn worldserver_cli_help_and_version_match_cpp_surface() {
    let help = worldserver_cli_help_like_cpp();
    assert!(help.contains("--config"));
    assert!(help.contains("--config-dir"));
    assert!(help.contains("--update-databases-only"));
    assert!(help.contains("--version"));
    assert!(help.contains("--help"));

    let version = worldserver_full_version_like_cpp();
    assert!(version.contains("RustyCore World Server"));
    assert!(version.contains(env!("CARGO_PKG_VERSION")));
    let revision = worldserver_revision_like_cpp();
    assert!(
        matches!(revision.len(), 40 | 64)
            && revision
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)),
        "world-server builds from a Git checkout must embed the exact source revision"
    );
    assert!(version.contains(revision));
}

#[test]
fn world_db_core_version_update_sql_matches_cpp_shape() {
    let sql = world_db_core_version_update_sql_like_cpp();

    assert!(sql.starts_with("UPDATE version SET core_version = '"));
    assert!(sql.contains("RustyCore World Server"));
    assert!(sql.contains(env!("CARGO_PKG_VERSION")));
    assert!(sql.contains("core_revision = '"));
    assert!(sql.contains(worldserver_revision_like_cpp()));
    assert!(!sql.contains('\n'));
}

#[test]
fn world_runtime_state_stop_and_counter_match_cpp_contract() {
    let world = WorldRuntimeStateLikeCpp::new();

    assert!(!world.is_stopped_like_cpp());
    assert_eq!(world.get_exit_code_like_cpp(), SHUTDOWN_EXIT_CODE_LIKE_CPP);
    assert_eq!(world.world_loop_counter_like_cpp(), 0);

    assert_eq!(world.increment_world_loop_counter_like_cpp(), 1);
    assert_eq!(world.increment_world_loop_counter_like_cpp(), 2);
    assert_eq!(world.world_loop_counter_like_cpp(), 2);

    world.stop_now_like_cpp(1);
    assert!(world.is_stopped_like_cpp());
    assert_eq!(world.get_exit_code_like_cpp(), 1);
    assert_eq!(
        process_exit_code_like_cpp(2),
        std::process::ExitCode::from(2)
    );
    assert_eq!(ERROR_EXIT_CODE_LIKE_CPP, 1);
    assert_eq!(RESTART_EXIT_CODE_LIKE_CPP, 2);
}

#[test]
fn freeze_detector_poll_matches_cpp_counter_contract() {
    let mut detector = FreezeDetectorLikeCpp::new(60_000, 1_000);

    assert_eq!(
        detector.poll_once_like_cpp(2_000, 1),
        FreezeDetectorPollOutcomeLikeCpp::Advanced
    );
    assert_eq!(
        detector.poll_once_like_cpp(61_000, 1),
        FreezeDetectorPollOutcomeLikeCpp::StillAlive
    );
    assert_eq!(
        detector.poll_once_like_cpp(62_001, 1),
        FreezeDetectorPollOutcomeLikeCpp::Abort { stuck_ms: 60_001 }
    );
    assert_eq!(
        detector.poll_once_like_cpp(63_000, 2),
        FreezeDetectorPollOutcomeLikeCpp::Advanced
    );
}

#[test]
fn world_update_loop_step_matches_cpp_timing_contract() {
    let world = WorldRuntimeStateLikeCpp::new();

    assert_eq!(
        half_max_core_stuck_time_like_cpp(0),
        u32::MAX,
        "C++ uses numeric_limits<uint32>::max() when halfMaxCoreStuckTime is zero"
    );

    let sleep = world_update_loop_step_like_cpp(&world, 1_000, 1_003, 10, 60_000);
    assert_eq!(
        sleep,
        WorldUpdateLoopStepOutcomeLikeCpp::Sleep {
            sleep_ms: 7,
            log_waiting_like_cpp: false
        }
    );
    assert_eq!(
        world.world_loop_counter_like_cpp(),
        1,
        "C++ increments m_worldLoopCounter before the sleep branch"
    );

    let long_sleep = world_update_loop_step_like_cpp(&world, 2_000, 2_000, 30_000, 60_000);
    assert_eq!(
        long_sleep,
        WorldUpdateLoopStepOutcomeLikeCpp::Sleep {
            sleep_ms: 30_000,
            log_waiting_like_cpp: true
        }
    );

    let update = world_update_loop_step_like_cpp(&world, 3_000, 3_025, 10, 60_000);
    assert_eq!(
        update,
        WorldUpdateLoopStepOutcomeLikeCpp::Update {
            diff_ms: 25,
            next_real_prev_time_ms: 3_025
        }
    );

    let wrap_update = world_update_loop_step_like_cpp(&world, u32::MAX - 4, 5, 1, 60_000);
    assert_eq!(
        wrap_update,
        WorldUpdateLoopStepOutcomeLikeCpp::Update {
            diff_ms: 10,
            next_real_prev_time_ms: 5
        }
    );
}

#[test]
fn world_update_loop_direct_configs_match_cpp_defaults_and_keys() {
    let _guard = TEST_LOCK.lock().expect("test lock poisoned");
    let root = unique_temp_dir("world_update_loop_direct_configs");
    let config = root.join("worldserver.conf");

    fs::write(&config, "").expect("write empty config failed");
    wow_config::load_config(config.to_str().expect("utf8 config path"))
        .expect("load empty config failed");

    assert_eq!(min_world_update_time_ms_like_cpp(), 1);
    assert_eq!(max_core_stuck_time_secs_like_cpp(), 60);
    assert_eq!(max_core_stuck_time_ms_like_cpp(), 60_000);

    fs::write(&config, "MinWorldUpdateTime = 7\nMaxCoreStuckTime = 0\n")
        .expect("write override config failed");
    wow_config::load_config(config.to_str().expect("utf8 config path"))
        .expect("load override config failed");

    assert_eq!(min_world_update_time_ms_like_cpp(), 7);
    assert_eq!(max_core_stuck_time_secs_like_cpp(), 0);
    assert_eq!(
        max_core_stuck_time_ms_like_cpp(),
        0,
        "C++ treats MaxCoreStuckTime=0 as disabled before constructing FreezeDetector"
    );
}

#[test]
fn world_config_resolution_prefers_lowercase_cpp_name() {
    let _guard = TEST_LOCK.lock().expect("test lock poisoned");
    let root = unique_temp_dir("world_config_resolution");
    let lower = root.join("worldserver.conf");
    let legacy = root.join("WorldServer.conf");

    fs::write(&lower, "WorldServerPort = 8085\n").expect("write lower failed");
    fs::write(&legacy, "WorldServerPort = 9000\n").expect("write legacy failed");

    let report = load_world_config_from(
        &[
            lower.to_str().expect("utf8 path"),
            legacy.to_str().expect("utf8 path"),
        ],
        root.join("worldserver.conf.d").to_str().expect("utf8 path"),
    )
    .expect("config should load");

    assert_eq!(report.candidate_index, 0);
    assert_eq!(wow_config::get_value::<u16>("WorldServerPort"), Some(8085));

    fs::remove_dir_all(root).expect("cleanup failed");
}

#[test]
fn world_config_cli_config_uses_exact_file_like_cpp() {
    let _guard = TEST_LOCK.lock().expect("test lock poisoned");
    let root = unique_temp_dir("world_config_cli_exact");
    let default_file = root.join("worldserver.conf");
    let override_file = root.join("custom-world.conf");
    let config_dir = root.join("custom-world.conf.d");

    fs::create_dir_all(&config_dir).expect("config dir failed");
    fs::write(&default_file, "WorldServerPort = 8085\n").expect("write default failed");
    fs::write(&override_file, "WorldServerPort = 9100\n").expect("write override failed");
    fs::write(
        config_dir.join("overlay.conf"),
        "InstanceServerPort = 9101\n",
    )
    .expect("write overlay failed");

    let override_path = override_file.to_string_lossy().into_owned();
    let config_dir_path = config_dir.to_string_lossy().into_owned();
    let report = load_world_config_from(&[override_path.as_str()], &config_dir_path)
        .expect("config should load");

    assert_eq!(report.initial_file, override_path);
    assert_eq!(report.candidate_index, 0);
    assert_eq!(wow_config::get_value::<u16>("WorldServerPort"), Some(9100));
    assert_eq!(
        wow_config::get_value::<u16>("InstanceServerPort"),
        Some(9101)
    );

    fs::remove_dir_all(root).expect("cleanup failed");
}

#[test]
fn world_network_config_uses_resolved_world_configs() {
    let _guard = TEST_LOCK.lock().expect("test lock poisoned");
    wow_config::load_config_from_str(
        r#"
WorldServerPort = 70000
InstanceServerPort = 70001
Expansion = 9
"#,
    )
    .expect("config should load");

    let configs = wow_config::load_world_config_values();
    assert_eq!(world_config_u16(&configs, "CONFIG_PORT_WORLD", 8085), 4464);
    assert_eq!(
        world_config_u16(&configs, "CONFIG_PORT_INSTANCE", 8086),
        4465
    );
    assert_eq!(world_config_u8(&configs, "CONFIG_EXPANSION", 2), 9);
}

#[test]
fn world_server_binary_delegates_to_the_library_composition_root() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let binary_source = fs::read_to_string(manifest_dir.join("src/main.rs"))
        .expect("world-server binary source should be readable");

    assert!(binary_source.contains("world_server::run("));
    assert!(!binary_source.contains("start_world_listener"));
    assert!(!binary_source.contains("create_session"));
}

#[test]
fn library_composition_preserves_cpp_startup_and_shutdown_order() {
    let source = fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/app.rs"))
        .expect("world-server composition source should be readable");

    let mut cursor = 0;
    for stage in [
        "let config_report = load_world_config(&cli)?;",
        "LoginDatabase::open_with_pool_size_and_auto_create_like_cpp(",
        "CharacterDatabase::open_with_pool_size_and_auto_create_like_cpp(",
        "WorldDatabase::open_with_pool_size_and_auto_create_like_cpp(",
        "HotfixDatabase::open_with_pool_size_and_auto_create_like_cpp(",
        "set_realm_offline(&login_db, realm_id).await?;",
        "load_realm_info_from_snapshot_like_cpp(&realm_list, realm_id)?;",
        "wow_network::start_world_listener(",
        "set_realm_online(&login_db, realm_id).await",
        "shutdown_signal()",
        "active_session_registry.begin_shutdown_like_cpp();",
        "stop_world_network_like_cpp([",
        "drain_respawn_db_writer_like_cpp(",
        "set_realm_offline(&login_db, realm_id).await",
    ] {
        let offset = source[cursor..]
            .find(stage)
            .unwrap_or_else(|| panic!("missing or reordered composition stage: {stage}"));
        cursor += offset + stage.len();
    }
}

#[test]
fn world_listener_captures_application_resources_outside_transport_boundary() {
    let source = fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/app.rs"))
        .expect("world-server composition source should be readable");
    let listener_start = source
        .find("wow_network::start_world_listener(")
        .expect("world listener call must exist");
    let listener_end = listener_start
        + source[listener_start..]
            .find("realm_listener_ready_tx,")
            .expect("world listener readiness argument must exist");
    let listener_call = &source[listener_start..listener_end];

    assert!(source[..listener_start].contains("let resources = Arc::clone(&session_resources);"));
    assert!(
        listener_call.contains(
            "move |account, pkt_rx, send_tx, send_write_fence_like_cpp, socket_timeouts|"
        )
    );
    assert!(listener_call.contains("let resources = Arc::clone(&resources);"));
    assert!(listener_call.contains("create_session("));
    assert!(
        !listener_call.contains("session_resources"),
        "the application aggregate must be captured outside the listener call"
    );
}

#[test]
fn primary_profession_capacity_config_and_session_resource_wiring_are_pinned() {
    let _guard = TEST_LOCK.lock().expect("test lock poisoned");

    for (configured, expected) in [
        (None, 2),
        (Some("0"), 0),
        (Some("1"), 1),
        (Some("2"), 2),
        (Some("11"), 11),
        (Some("-1"), 2),
        (Some("12"), 2),
    ] {
        let source = configured
            .map(|value| format!("MaxPrimaryTradeSkill = {value}\n"))
            .unwrap_or_default();
        wow_config::load_config_from_str(&source).expect("config should load");
        let configs = wow_config::load_world_config_values();
        assert_eq!(
            max_primary_trade_skills_like_cpp(&configs),
            expected,
            "configured value {configured:?}"
        );
    }

    let composition_source =
        fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/app.rs"))
            .expect("world-server composition source should be readable");
    let session_factory_source = fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/session_factory.rs"),
    )
    .expect("world-server session factory source should be readable");
    let materialization_needle = [
        "max_primary_trade_skills:",
        " max_primary_trade_skills_like_cpp(&world_configs),",
    ]
    .concat();
    let propagation_needle = [
        "session.set_max_primary_trade_skills_like_cpp(",
        "resources.max_primary_trade_skills);",
    ]
    .concat();
    assert!(
        composition_source.contains(&materialization_needle),
        "SessionResources must materialize the validated configuration"
    );
    assert!(
        session_factory_source.contains(&propagation_needle),
        "create_session must propagate SessionResources into WorldSession"
    );
}

#[test]
fn realm_id_config_is_required_and_non_zero_like_cpp() {
    let _guard = TEST_LOCK.lock().expect("test lock poisoned");
    wow_config::load_config_from_str("").expect("config should load");
    let missing = realm_id_like_cpp().expect_err("missing RealmID must fail");
    assert!(
        missing
            .to_string()
            .contains("Realm ID not defined in configuration file")
    );

    wow_config::load_config_from_str("RealmID = 0\n").expect("config should load");
    let zero = realm_id_like_cpp().expect_err("RealmID 0 must fail");
    assert!(
        zero.to_string()
            .contains("Realm ID not defined in configuration file")
    );

    wow_config::load_config_from_str("RealmID = 3\n").expect("config should load");
    assert_eq!(realm_id_like_cpp().expect("valid RealmID"), 3);
}

#[test]
fn db_keepalive_config_and_pool_scope_match_cpp() {
    let _guard = TEST_LOCK.lock().expect("test lock poisoned");

    wow_config::load_config_from_str("").expect("config should load");
    let configs = wow_config::load_world_config_values();
    assert_eq!(db_keepalive_interval_minutes_like_cpp(&configs), 30);

    wow_config::load_config_from_str("MaxPingTime = 7\n").expect("config should load");
    let configs = wow_config::load_world_config_values();
    assert_eq!(db_keepalive_interval_minutes_like_cpp(&configs), 7);
    assert_eq!(
        db_keepalive_database_names_like_cpp(),
        ["Character", "Login", "World"]
    );
    assert_eq!(db_keepalive_sql_like_cpp(), "SELECT 1");
}

#[test]
fn db_updater_step_errors_are_fatal_with_context_like_cpp() {
    let ok = db_updater_step_like_cpp::<u8>(Ok(7), "Login", "populate")
        .expect("successful updater step should pass through");
    assert_eq!(ok, 7);

    let error = db_updater_step_like_cpp::<()>(
        Err(anyhow::anyhow!("base file missing")),
        "Character",
        "populate",
    )
    .expect_err("failed updater step should abort startup");
    let rendered = format!("{error:#}");

    assert!(rendered.contains("Could not populate the Character database"));
    assert!(rendered.contains("base file missing"));
}

#[test]
fn world_db_version_sentinel_accepts_only_current_tdb_like_cpp() {
    assert_eq!(REQUIRED_TDB_VERSION_LIKE_CPP, "TDB 343.24081");
    assert_eq!(REQUIRED_TDB_CACHE_ID_LIKE_CPP, 24081);

    let current = WorldDbVersionLikeCpp {
        db_version: REQUIRED_TDB_VERSION_LIKE_CPP.to_string(),
        cache_id: REQUIRED_TDB_CACHE_ID_LIKE_CPP,
    };
    assert!(world_db_version_matches_required_like_cpp(&current));

    let wrong_version = WorldDbVersionLikeCpp {
        db_version: "TDB 343.24080".to_string(),
        cache_id: REQUIRED_TDB_CACHE_ID_LIKE_CPP,
    };
    assert!(!world_db_version_matches_required_like_cpp(&wrong_version));

    let wrong_cache = WorldDbVersionLikeCpp {
        db_version: REQUIRED_TDB_VERSION_LIKE_CPP.to_string(),
        cache_id: 24080,
    };
    assert!(!world_db_version_matches_required_like_cpp(&wrong_cache));
}

#[test]
fn world_db_version_mismatch_reports_expected_and_found_like_cpp() {
    let mismatch = WorldDbVersionLikeCpp {
        db_version: "TDB 343.00000".to_string(),
        cache_id: 0,
    };
    let message = world_db_version_mismatch_message_like_cpp(Some(&mismatch));

    assert!(message.contains("World database version mismatch"));
    assert!(message.contains("expected TDB 343.24081 / cache_id 24081"));
    assert!(message.contains("found TDB 343.00000 / cache_id 0"));

    let missing = world_db_version_mismatch_message_like_cpp(None);
    assert!(missing.contains("Unknown world database."));
}

#[test]
fn database_pool_size_uses_cpp_worker_and_synch_thread_keys() {
    let _guard = TEST_LOCK.lock().expect("test lock poisoned");

    wow_config::load_config_from_str("").expect("config should load");
    assert_eq!(database_pool_size_like_cpp("Login"), 2);
    assert_eq!(database_pool_size_like_cpp("Character"), 2);

    wow_config::load_config_from_str(
        r#"
LoginDatabase.WorkerThreads = 3
LoginDatabase.SynchThreads = 5
CharacterDatabase.WorkerThreads = 1
CharacterDatabase.SynchThreads = 2
WorldDatabase.WorkerThreads = 0
WorldDatabase.SynchThreads = 33
"#,
    )
    .expect("config should load");

    assert_eq!(database_pool_size_like_cpp("Login"), 8);
    assert_eq!(database_pool_size_like_cpp("Character"), 3);
    assert_eq!(database_pool_size_like_cpp("World"), 2);
}

#[test]
fn updates_auto_setup_defaults_enabled_like_cpp() {
    let _guard = TEST_LOCK.lock().expect("test lock poisoned");

    wow_config::load_config_from_str("").expect("config should load");
    assert!(updates_auto_setup_enabled_like_cpp());

    wow_config::load_config_from_str("Updates.AutoSetup = 0\n").expect("config should load");
    assert!(!updates_auto_setup_enabled_like_cpp());

    wow_config::load_config_from_str("Updates.AutoSetup = false\n").expect("config should load");
    assert!(!updates_auto_setup_enabled_like_cpp());

    wow_config::load_config_from_str("Updates.AutoSetup = 1\n").expect("config should load");
    assert!(updates_auto_setup_enabled_like_cpp());
}

#[test]
fn updates_enable_databases_mask_matches_cpp() {
    let _guard = TEST_LOCK.lock().expect("test lock poisoned");

    wow_config::load_config_from_str("").expect("config should load");
    assert_eq!(updates_database_mask_like_cpp(), DATABASE_MASK_ALL_LIKE_CPP);
    for flag in [
        DATABASE_LOGIN_LIKE_CPP,
        DATABASE_CHARACTER_LIKE_CPP,
        DATABASE_WORLD_LIKE_CPP,
        DATABASE_HOTFIX_LIKE_CPP,
    ] {
        assert!(updates_enabled_for_database_like_cpp(
            updates_database_mask_like_cpp(),
            flag
        ));
    }

    wow_config::load_config_from_str("Updates.EnableDatabases = 5\n").expect("config should load");
    let mask = updates_database_mask_like_cpp();
    assert!(updates_enabled_for_database_like_cpp(
        mask,
        DATABASE_LOGIN_LIKE_CPP
    ));
    assert!(!updates_enabled_for_database_like_cpp(
        mask,
        DATABASE_CHARACTER_LIKE_CPP
    ));
    assert!(updates_enabled_for_database_like_cpp(
        mask,
        DATABASE_WORLD_LIKE_CPP
    ));
    assert!(!updates_enabled_for_database_like_cpp(
        mask,
        DATABASE_HOTFIX_LIKE_CPP
    ));

    wow_config::load_config_from_str("Updates.EnableDatabases = 0\n").expect("config should load");
    let mask = updates_database_mask_like_cpp();
    assert!(!updates_enabled_for_database_like_cpp(
        mask,
        DATABASE_LOGIN_LIKE_CPP
    ));
    assert!(!updates_enabled_for_database_like_cpp(
        mask,
        DATABASE_CHARACTER_LIKE_CPP
    ));
    assert!(!updates_enabled_for_database_like_cpp(
        mask,
        DATABASE_WORLD_LIKE_CPP
    ));
    assert!(!updates_enabled_for_database_like_cpp(
        mask,
        DATABASE_HOTFIX_LIKE_CPP
    ));

    assert!(!database_auto_create_enabled_like_cpp(
        true,
        mask,
        DATABASE_LOGIN_LIKE_CPP
    ));
    assert!(!database_auto_create_enabled_like_cpp(
        false,
        DATABASE_MASK_ALL_LIKE_CPP,
        DATABASE_LOGIN_LIKE_CPP
    ));
    assert!(database_auto_create_enabled_like_cpp(
        true,
        DATABASE_MASK_ALL_LIKE_CPP,
        DATABASE_LOGIN_LIKE_CPP
    ));
}

/// The sessionless tap index must answer what the session answered.
///
/// `WorldSession::current_group_member_guids_for_tap_like_cpp` reads the
/// session's own `group_guid` mirror plus the registry. The tick owner has no
/// session, so it asks the membership authority directly — and the two must
/// agree, member for member, or relocating the melee phase would silently
/// change who is tapped in (#28).
#[test]
fn sessionless_tap_group_index_matches_the_session_answer_like_cpp() {
    use wow_social::group::{GroupInfo, GroupRegistry};

    let leader = ObjectGuid::create_player(1, 6_001);
    let second = ObjectGuid::create_player(1, 6_002);
    let third = ObjectGuid::create_player(1, 6_003);
    let ungrouped = ObjectGuid::create_player(1, 6_004);

    let registry = Arc::new(GroupRegistry::default());
    let mut group = GroupInfo::new(leader);
    assert!(group.add_member(second));
    assert!(group.add_member(third));
    let group_guid = group.group_guid;
    registry.register_group_like_cpp(group_guid, group);

    let index = build_tap_group_index_like_cpp(Some(&registry));

    // Every member sees the others and never itself — the exact contract of
    // `current_group_member_guids_for_tap_like_cpp`.
    for (member, expected) in [
        (leader, vec![second, third]),
        (second, vec![leader, third]),
        (third, vec![leader, second]),
    ] {
        let mut actual = index.get(&member).cloned().unwrap_or_default();
        actual.sort();
        let mut expected = expected;
        expected.sort();
        assert_eq!(actual, expected, "tap group for {member:?}");
        assert!(!actual.contains(&member), "a member never taps itself in");
    }

    assert!(
        index.get(&ungrouped).is_none(),
        "an ungrouped player has no tap group, as the session returns an empty vec"
    );
    assert!(
        build_tap_group_index_like_cpp(None).is_empty(),
        "no registry means no tap groups, not a panic"
    );
}

/// The tick owner is decided once, before the loop that reads it starts.
///
/// A flip after `spawn_legacy_creature_runtime_update_loop_like_cpp` is the only
/// remaining window in which the loop and a session can both tick the same
/// creature, so the single production call site is asserted rather than left to
/// convention (#28).
#[test]
fn set_tick_owner_has_exactly_one_production_call_site_before_the_loop_spawns() {
    let app = include_str!("app.rs");
    let calls: Vec<_> = app.match_indices("set_tick_owner(").collect();
    assert_eq!(
        calls.len(),
        1,
        "production must decide the tick owner exactly once, found {}",
        calls.len()
    );
    let spawn = app
        .find("spawn_legacy_creature_runtime_update_loop_like_cpp(")
        .expect("the global legacy creature loop is spawned in app.rs");
    assert!(
        calls[0].0 < spawn,
        "the owner must be set before the loop that reads it is spawned"
    );

    for source in [
        include_str!("lib.rs"),
        include_str!("runtime/delivery.rs"),
        include_str!("runtime/map.rs"),
    ] {
        assert!(
            !source.contains("set_tick_owner("),
            "only app.rs may decide the tick owner"
        );
    }
}

#[test]
fn database_update_bootstrap_uses_only_typed_adapter_operations_like_cpp() {
    let app = include_str!("app.rs");
    let (_, update_tail) = app
        .split_once("// ── Database auto-update")
        .expect("database updater section starts");
    let (updates, _) = update_tail
        .split_once("// ─────────────────────────────────────────────────────────────────────")
        .expect("database updater section ends");

    assert!(
        !updates.contains("DbUpdater"),
        "the composition root must not own the concrete updater"
    );
    assert!(
        !updates.contains(".pool()"),
        "the composition root must not extract a raw SQLx pool"
    );
    assert_eq!(
        updates.matches("populate_typed_database_like_cpp(").count(),
        2,
        "Login and Character retain their C++ populate phase"
    );
    assert_eq!(
        updates.matches("update_typed_database_like_cpp(").count(),
        4,
        "all four typed databases retain their C++ update phase"
    );

    let login = updates.find("if login_updates_enabled").unwrap();
    let characters = updates.find("if character_updates_enabled").unwrap();
    let world = updates.find("if world_updates_enabled").unwrap();
    let hotfix = updates.find("if hotfix_updates_enabled").unwrap();
    assert!(login < characters && characters < world && world < hotfix);
}

#[test]
fn legacy_creature_global_runtime_config_defaults_to_cpp_map_owned_runtime() {
    let _guard = TEST_LOCK.lock().expect("test lock poisoned");

    wow_config::load_config_from_str("").expect("config should load");
    assert!(legacy_creature_global_runtime_enabled_from_config_like_cpp());

    wow_config::load_config_from_str("RustyCore.LegacyCreatureGlobalRuntime = 0\n")
        .expect("config should load");
    assert!(!legacy_creature_global_runtime_enabled_from_config_like_cpp());

    wow_config::load_config_from_str("RustyCore.LegacyCreatureGlobalRuntime = 1\n")
        .expect("config should load");
    assert!(legacy_creature_global_runtime_enabled_from_config_like_cpp());
}

#[test]
fn legacy_creature_aggro_config_uses_cpp_no_gray_aggro_keys_like_cpp() {
    let _guard = TEST_LOCK.lock().expect("test lock poisoned");

    wow_config::load_config_from_str(
        r#"
MaxPlayerLevel = 70
NoGrayAggro.Above = 80
NoGrayAggro.Below = 90
Rate.Creature.Aggro = 2
Visibility.Distance.Continents = 20
Visibility.Distance.Instances = 9999
Visibility.Distance.BG = 140
Visibility.Distance.Arenas = 150
CreatureFamilyAssistanceRadius = 22
CreatureFamilyAssistanceDelay = 3456
"#,
    )
    .expect("config should load");
    let configs = wow_config::load_world_config_values();
    let config = legacy_creature_aggro_config_like_cpp(&configs);

    // C++ first clamps NoGrayAggro values to MaxPlayerLevel, then clamps
    // Below down to Above when Above > 0 && Above < Below.
    assert_eq!(config.no_gray_aggro_above, 70);
    assert_eq!(config.no_gray_aggro_below, 70);
    assert_eq!(config.creature_aggro_rate, 2.0);
    assert_eq!(config.max_player_level_config, 70);
    assert_eq!(config.visibility_distance_continents, 90.0);
    assert_eq!(
        config.visibility_distance_instances,
        wow_entities::MAX_VISIBILITY_DISTANCE
    );
    assert_eq!(config.visibility_distance_battlegrounds, 140.0);
    assert_eq!(config.visibility_distance_arenas, 150.0);
    assert_eq!(config.family_assistance_radius, 22.0);
    assert_eq!(config.family_assistance_delay_ms, 3_456);
}

#[test]
fn loot_drop_rates_use_cpp_world_config_keys() {
    let _guard = TEST_LOCK.lock().expect("test lock poisoned");
    wow_config::load_config_from_str(
        r#"
Rate.Drop.Item.Poor = 0.5
Rate.Drop.Item.Rare = 3
Rate.Drop.Item.Referenced = 4
Rate.Drop.Item.ReferencedAmount = 2
Rate.Drop.Money = 6
Rate.Corpse.Decay.Looted = 0.25
"#,
    )
    .expect("config should load");

    let configs = wow_config::load_world_config_values();
    let rates = loot_drop_rates_like_cpp(&configs);
    assert_eq!(rates.item_poor, 0.5);
    assert_eq!(rates.item_normal, 1.0);
    assert_eq!(rates.item_rare, 3.0);
    assert_eq!(rates.item_referenced, 4.0);
    assert_eq!(rates.item_referenced_amount, 2.0);
    assert_eq!(rates.money, 6.0);
    assert_eq!(rates.corpse_decay_looted, 0.25);
}

#[test]
fn reputation_rates_use_cpp_world_config_keys() {
    let _guard = TEST_LOCK.lock().expect("test lock poisoned");
    wow_config::load_config_from_str(
        r#"
Rate.Reputation.Gain = 2
Rate.Reputation.LowLevel.Kill = 0.25
Rate.Reputation.LowLevel.Quest = 0.5
Rate.Reputation.RecruitAFriendBonus = 0.2
MaxRecruitAFriendBonusDistance = 45
"#,
    )
    .expect("config should load");

    let configs = wow_config::load_world_config_values();
    let rates = reputation_rates_like_cpp(&configs);
    assert_eq!(rates.gain, 2.0);
    assert_eq!(rates.low_level_kill, 0.25);
    assert_eq!(rates.low_level_quest, 0.5);
    assert_eq!(rates.recruit_a_friend_bonus, 0.2);
    assert_eq!(rates.recruit_a_friend_distance, 45.0);
}

#[test]
fn repair_cost_rate_uses_cpp_world_config_key_and_clamps_negative_like_cpp() {
    let _guard = TEST_LOCK.lock().expect("test lock poisoned");
    wow_config::load_config_from_str("Rate.RepairCost = 2.5\n").expect("config should load");

    let configs = wow_config::load_world_config_values();
    assert_eq!(repair_cost_rate_like_cpp(&configs), 2.5);

    wow_config::load_config_from_str("Rate.RepairCost = -1\n").expect("config should load");
    let configs = wow_config::load_world_config_values();
    assert_eq!(repair_cost_rate_like_cpp(&configs), 0.0);
}

#[test]
fn reset_schedule_uses_cpp_world_config_defaults_and_keys() {
    let _guard = TEST_LOCK.lock().expect("test lock poisoned");
    wow_config::load_config_from_str("").expect("config should load");

    let configs = wow_config::load_world_config_values();
    assert_eq!(
        reset_schedule_like_cpp(&configs),
        ResetSchedule {
            hour: 8,
            week_day: 2,
        }
    );

    wow_config::load_config_from_str(
        r#"
ResetSchedule.Hour = 6
ResetSchedule.WeekDay = 5
"#,
    )
    .expect("config should load");

    let configs = wow_config::load_world_config_values();
    assert_eq!(
        reset_schedule_like_cpp(&configs),
        ResetSchedule {
            hour: 6,
            week_day: 5,
        }
    );
}

#[test]
fn enable_ae_loot_uses_cpp_world_config_key() {
    let _guard = TEST_LOCK.lock().expect("test lock poisoned");
    wow_config::load_config_from_str("EnableAELoot = 1\n").expect("config should load");

    let configs = wow_config::load_world_config_values();
    assert!(world_config_bool(&configs, "CONFIG_ENABLE_AE_LOOT", false));
}

#[test]
fn addon_channel_uses_cpp_world_config_key() {
    let _guard = TEST_LOCK.lock().expect("test lock poisoned");
    wow_config::load_config_from_str("AddonChannel = 0\n").expect("config should load");

    let configs = wow_config::load_world_config_values();
    assert!(!world_config_bool(&configs, "CONFIG_ADDON_CHANNEL", true));
}

#[test]
fn no_reset_talent_cost_uses_cpp_world_config_key() {
    let _guard = TEST_LOCK.lock().expect("test lock poisoned");
    wow_config::load_config_from_str("NoResetTalentsCost = 1\n").expect("config should load");

    let configs = wow_config::load_world_config_values();
    assert!(world_config_bool(
        &configs,
        "CONFIG_NO_RESET_TALENT_COST",
        false
    ));
}

#[test]
fn offhand_check_at_spell_unlearn_uses_cpp_world_config_key() {
    let _guard = TEST_LOCK.lock().expect("test lock poisoned");
    wow_config::load_config_from_str("OffhandCheckAtSpellUnlearn = 0\n")
        .expect("config should load");

    let configs = wow_config::load_world_config_values();
    assert!(!world_config_bool(
        &configs,
        "CONFIG_OFFHAND_CHECK_AT_SPELL_UNLEARN",
        true
    ));
}

#[test]
fn vmap_indoor_check_uses_cpp_world_config_key() {
    let _guard = TEST_LOCK.lock().expect("test lock poisoned");
    wow_config::load_config_from_str("vmap.enableIndoorCheck = 1\n").expect("config should load");

    let configs = wow_config::load_world_config_values();
    assert!(world_config_bool(
        &configs,
        "CONFIG_VMAP_INDOOR_CHECK",
        false
    ));
}

#[test]
fn player_start_explored_and_reputation_use_cpp_world_config_keys() {
    let _guard = TEST_LOCK.lock().expect("test lock poisoned");
    wow_config::load_config_from_str(
        "PlayerStart.MapsExplored = 1\nPlayerStart.AllReputation = 1\n",
    )
    .expect("config should load");

    let configs = wow_config::load_world_config_values();
    assert!(world_config_bool(
        &configs,
        "CONFIG_START_ALL_EXPLORED",
        false
    ));
    assert!(world_config_bool(&configs, "CONFIG_START_ALL_REP", false));
}

#[test]
fn instance_ignore_raid_uses_cpp_world_config_key() {
    let _guard = TEST_LOCK.lock().expect("test lock poisoned");
    wow_config::load_config_from_str("Instance.IgnoreRaid = 1\n").expect("config should load");

    let configs = wow_config::load_world_config_values();
    assert!(world_config_bool(
        &configs,
        "CONFIG_INSTANCE_IGNORE_RAID",
        false
    ));
}

#[test]
fn instance_ignore_level_uses_cpp_world_config_key() {
    let _guard = TEST_LOCK.lock().expect("test lock poisoned");
    wow_config::load_config_from_str("Instance.IgnoreLevel = 1\n").expect("config should load");

    let configs = wow_config::load_world_config_values();
    assert!(world_config_bool(
        &configs,
        "CONFIG_INSTANCE_IGNORE_LEVEL",
        false
    ));
}

#[test]
fn account_instances_per_hour_uses_cpp_world_config_key() {
    let _guard = TEST_LOCK.lock().expect("test lock poisoned");
    wow_config::load_config_from_str("AccountInstancesPerHour = 7\n").expect("config should load");

    let configs = wow_config::load_world_config_values();
    assert_eq!(
        world_config_u32(&configs, "CONFIG_MAX_INSTANCES_PER_HOUR", 5),
        7
    );
}

#[test]
fn chat_fake_message_preventing_uses_cpp_world_config_key() {
    let _guard = TEST_LOCK.lock().expect("test lock poisoned");
    wow_config::load_config_from_str("ChatFakeMessagePreventing = 1\n")
        .expect("config should load");

    let configs = wow_config::load_world_config_values();
    assert!(world_config_bool(
        &configs,
        "CONFIG_CHAT_FAKE_MESSAGE_PREVENTING",
        false
    ));
}

#[test]
fn party_raid_warnings_uses_cpp_world_config_key() {
    let _guard = TEST_LOCK.lock().expect("test lock poisoned");
    wow_config::load_config_from_str("PartyRaidWarnings = 1\n").expect("config should load");

    let configs = wow_config::load_world_config_values();
    assert!(world_config_bool(
        &configs,
        "CONFIG_CHAT_PARTY_RAID_WARNINGS",
        false
    ));
}

#[test]
fn party_invite_configs_use_cpp_world_config_keys() {
    let _guard = TEST_LOCK.lock().expect("test lock poisoned");
    wow_config::load_config_from_str(
        "GM.AllowInvite = 1\n\
         AllowTwoSide.Interaction.Group = 1\n\
         PartyLevelReq = 12\n",
    )
    .expect("config should load");

    let configs = wow_config::load_world_config_values();
    assert!(world_config_bool(&configs, "CONFIG_ALLOW_GM_GROUP", false));
    assert!(world_config_bool(
        &configs,
        "CONFIG_ALLOW_TWO_SIDE_INTERACTION_GROUP",
        false
    ));
    assert_eq!(world_config_u32(&configs, "CONFIG_PARTY_LEVEL_REQ", 1), 12);
}

#[test]
fn chat_strict_link_checking_kick_uses_cpp_world_config_key() {
    let _guard = TEST_LOCK.lock().expect("test lock poisoned");
    wow_config::load_config_from_str("ChatStrictLinkChecking.Kick = 1\n")
        .expect("config should load");

    let configs = wow_config::load_world_config_values();
    assert_ne!(
        world_config_u8(&configs, "CONFIG_CHAT_STRICT_LINK_CHECKING_KICK", 0),
        0
    );
}

#[test]
fn chat_level_requirements_use_cpp_world_config_keys() {
    let _guard = TEST_LOCK.lock().expect("test lock poisoned");
    wow_config::load_config_from_str(
        "ChatLevelReq.Channel = 2\n\
         ChatLevelReq.Whisper = 3\n\
         ChatLevelReq.Emote = 4\n\
         ChatLevelReq.Say = 5\n\
         ChatLevelReq.Yell = 6\n",
    )
    .expect("config should load");

    let configs = wow_config::load_world_config_values();
    assert_eq!(
        world_config_u8(&configs, "CONFIG_CHAT_CHANNEL_LEVEL_REQ", 1),
        2
    );
    assert_eq!(
        world_config_u8(&configs, "CONFIG_CHAT_WHISPER_LEVEL_REQ", 1),
        3
    );
    assert_eq!(
        world_config_u8(&configs, "CONFIG_CHAT_EMOTE_LEVEL_REQ", 1),
        4
    );
    assert_eq!(world_config_u8(&configs, "CONFIG_CHAT_SAY_LEVEL_REQ", 1), 5);
    assert_eq!(
        world_config_u8(&configs, "CONFIG_CHAT_YELL_LEVEL_REQ", 1),
        6
    );
}

#[test]
fn chat_listen_ranges_use_cpp_world_config_keys() {
    let _guard = TEST_LOCK.lock().expect("test lock poisoned");
    wow_config::load_config_from_str(
        "ListenRange.Say = 40\n\
         ListenRange.TextEmote = 41\n\
         ListenRange.Yell = 301\n",
    )
    .expect("config should load");

    let configs = wow_config::load_world_config_values();
    assert_eq!(
        world_config_f32(&configs, "CONFIG_LISTEN_RANGE_SAY", 25.0),
        40.0
    );
    assert_eq!(
        world_config_f32(&configs, "CONFIG_LISTEN_RANGE_TEXTEMOTE", 25.0),
        41.0
    );
    assert_eq!(
        world_config_f32(&configs, "CONFIG_LISTEN_RANGE_YELL", 300.0),
        301.0
    );
}

#[test]
fn declined_names_are_forced_for_russian_realm_categories_like_cpp() {
    let categories = wow_data::CfgCategoriesStore::from_entries([
        wow_data::CfgCategoriesEntry {
            id: 1,
            name: "Development".to_string(),
            locale_mask: 0,
            create_charset_mask: 0x01,
            existing_charset_mask: 0,
            flags: 0,
            order: 0,
        },
        wow_data::CfgCategoriesEntry {
            id: 12,
            name: "Russian".to_string(),
            locale_mask: 0,
            create_charset_mask: 0x04,
            existing_charset_mask: 0,
            flags: 0,
            order: 0,
        },
    ]);

    assert!(!declined_names_used_for_realm_category_like_cpp(
        false,
        1,
        &categories
    ));
    assert!(declined_names_used_for_realm_category_like_cpp(
        false,
        12,
        &categories
    ));
    assert!(declined_names_used_for_realm_category_like_cpp(
        true,
        1,
        &categories
    ));
}

#[test]
fn chat_flood_config_uses_cpp_world_config_keys() {
    let _guard = TEST_LOCK.lock().expect("test lock poisoned");
    wow_config::load_config_from_str(
        "ChatFlood.MessageCount = 2\n\
         ChatFlood.MessageDelay = 3\n\
         ChatFlood.AddonMessageCount = 4\n\
         ChatFlood.AddonMessageDelay = 5\n\
         ChatFlood.MuteTime = 6\n",
    )
    .expect("config should load");

    let configs = wow_config::load_world_config_values();
    assert_eq!(
        world_config_u32(&configs, "CONFIG_CHATFLOOD_MESSAGE_COUNT", 10),
        2
    );
    assert_eq!(
        world_config_u32(&configs, "CONFIG_CHATFLOOD_MESSAGE_DELAY", 1),
        3
    );
    assert_eq!(
        world_config_u32(&configs, "CONFIG_CHATFLOOD_ADDON_MESSAGE_COUNT", 100),
        4
    );
    assert_eq!(
        world_config_u32(&configs, "CONFIG_CHATFLOOD_ADDON_MESSAGE_DELAY", 1),
        5
    );
    assert_eq!(
        world_config_u32(&configs, "CONFIG_CHATFLOOD_MUTE_TIME", 10),
        6
    );
}

#[test]
fn max_overspeed_pings_reads_cpp_world_config_key() {
    let _guard = TEST_LOCK.lock().expect("test lock poisoned");
    wow_config::load_config_from_str("MaxOverspeedPings = 7\n").expect("config should load");

    let configs = wow_config::load_world_config_values();
    assert_eq!(
        world_config_u32(&configs, "CONFIG_MAX_OVERSPEED_PINGS", 2),
        7
    );
}

#[test]
fn socket_timeouts_read_cpp_world_config_keys_as_seconds() {
    let _guard = TEST_LOCK.lock().expect("test lock poisoned");
    wow_config::load_config_from_str(
        "SocketTimeOutTime = 120000\nSocketTimeOutTimeActive = 45000\n",
    )
    .expect("config should load");

    let configs = wow_config::load_world_config_values();
    let timeouts = wow_network::SocketTimeoutsLikeCpp {
        unauthenticated_secs: u64::from(world_config_u32(
            &configs,
            "CONFIG_SOCKET_TIMEOUTTIME",
            900,
        )),
        active_secs: u64::from(world_config_u32(
            &configs,
            "CONFIG_SOCKET_TIMEOUTTIME_ACTIVE",
            60,
        )),
    };

    assert_eq!(
        timeouts,
        wow_network::SocketTimeoutsLikeCpp {
            unauthenticated_secs: 120,
            active_secs: 45,
        }
    );
}

#[test]
fn packet_spoof_config_reads_cpp_world_config_keys() {
    let _guard = TEST_LOCK.lock().expect("test lock poisoned");
    wow_config::load_config_from_str(
        "PacketSpoof.Policy = 2\nPacketSpoof.BanMode = 2\nPacketSpoof.BanDuration = 12345\n",
    )
    .expect("config should load");

    let configs = wow_config::load_world_config_values();
    let packet_spoof = wow_world::PacketSpoofConfigLikeCpp {
        policy: world_config_u32(&configs, "CONFIG_PACKET_SPOOF_POLICY", 1),
        ban_mode: world_config_u32(&configs, "CONFIG_PACKET_SPOOF_BANMODE", 0),
        ban_duration_secs: world_config_u32(&configs, "CONFIG_PACKET_SPOOF_BANDURATION", 86_400),
    };

    assert_eq!(
        packet_spoof,
        wow_world::PacketSpoofConfigLikeCpp {
            policy: 2,
            ban_mode: 2,
            ban_duration_secs: 12_345,
        }
    );
}

#[test]
fn mmap_runtime_config_uses_cpp_world_config_key_and_data_dir() {
    let _guard = TEST_LOCK.lock().expect("test lock poisoned");
    wow_config::load_config_from_str(
        r#"
DataDir = "/srv/wow-data"
mmap.enablePathFinding = 0
"#,
    )
    .expect("config should load");

    let configs = wow_config::load_world_config_values();
    let mmap_config = mmap_runtime_config_like_cpp(&configs, HashSet::from([1]));
    assert_eq!(mmap_config.data_dir, "/srv/wow-data");
    assert!(!mmap_config.enabled);
    assert!(!mmap_config.pathfinding_enabled_for_map_like_cpp(0));
    assert!(!mmap_config.pathfinding_enabled_for_map_like_cpp(1));
}

#[test]
fn mmap_runtime_config_applies_cpp_disable_mgr_map_gate() {
    let _guard = TEST_LOCK.lock().expect("test lock poisoned");
    wow_config::load_config_from_str("mmap.enablePathFinding = 1\n").expect("config should load");

    let configs = wow_config::load_world_config_values();
    let mmap_config = mmap_runtime_config_like_cpp(&configs, HashSet::from([571]));
    assert!(mmap_config.pathfinding_enabled_for_map_like_cpp(0));
    assert!(!mmap_config.pathfinding_enabled_for_map_like_cpp(571));
}

#[test]
fn canonical_spawn_group_initializer_applies_mapid_conditions_on_new_maps() {
    let metadata = Arc::new(Mutex::new(test_spawn_metadata([(10, 571), (11, 530)])));
    let condition_store = Arc::new(ConditionEntriesByTypeStore::from_conditions_like_cpp([
        mapid_condition(10, 571),
        mapid_condition(11, 571),
    ]));
    let mut manager = wow_map::MapManager::new(60_000, 10);
    install_canonical_spawn_group_initializer_like_cpp(
        &mut manager,
        Arc::clone(&metadata),
        condition_store,
        Arc::new(PersistedRespawnTimesLikeCpp::default()),
        Arc::new(canonical_test_map_store_like_cpp()),
    );

    let group_571 = metadata
        .lock()
        .expect("test metadata lock")
        .spawn_group_templates()
        .get(&10)
        .expect("test group 10")
        .clone();
    let map_571 = manager.create_world_map(571, 0);
    assert!(
        map_571
            .map()
            .is_spawn_group_active_like_cpp(Some(&group_571))
    );

    let group_530 = metadata
        .lock()
        .expect("test metadata lock")
        .spawn_group_templates()
        .get(&11)
        .expect("test group 11")
        .clone();
    let map_530 = manager.create_world_map(530, 0);
    assert!(
        !map_530
            .map()
            .is_spawn_group_active_like_cpp(Some(&group_530))
    );
}

#[test]
fn canonical_spawn_group_initializer_does_not_reexecute_for_existing_map() {
    let metadata = Arc::new(Mutex::new(test_spawn_metadata([(20, 571)])));
    let condition_store = Arc::new(ConditionEntriesByTypeStore::from_conditions_like_cpp([
        mapid_condition(20, 530),
    ]));
    let mut manager = wow_map::MapManager::new(60_000, 10);
    install_canonical_spawn_group_initializer_like_cpp(
        &mut manager,
        Arc::clone(&metadata),
        condition_store,
        Arc::new(PersistedRespawnTimesLikeCpp::default()),
        Arc::new(canonical_test_map_store_like_cpp()),
    );

    let group = metadata
        .lock()
        .expect("test metadata lock")
        .spawn_group_templates()
        .get(&20)
        .expect("test group 20")
        .clone();
    let map = manager.create_world_map(571, 0);
    assert!(!map.map().is_spawn_group_active_like_cpp(Some(&group)));
    map.map_mut()
        .set_spawn_group_active_like_cpp(Some(&group), true);
    assert!(map.map().is_spawn_group_active_like_cpp(Some(&group)));

    let existing = manager.create_world_map(571, 0);
    assert!(existing.map().is_spawn_group_active_like_cpp(Some(&group)));
}

#[test]
fn canonical_spawn_group_initializer_no_groups_is_noop() {
    let metadata = Arc::new(Mutex::new(test_spawn_metadata([])));
    let condition_store = Arc::new(ConditionEntriesByTypeStore::default());
    let mut manager = wow_map::MapManager::new(60_000, 10);
    install_canonical_spawn_group_initializer_like_cpp(
        &mut manager,
        metadata,
        condition_store,
        Arc::new(PersistedRespawnTimesLikeCpp::default()),
        Arc::new(canonical_test_map_store_like_cpp()),
    );

    let map = manager.create_world_map(999, 0);
    assert!(
        map.map()
            .spawn_group_state()
            .toggled_spawn_group_ids()
            .is_empty()
    );
}

#[test]
fn canonical_map_creation_loads_persisted_respawns_for_world_maps_before_spawn_groups() {
    let mut store = SpawnStore::new();
    let mut creature = test_spawn(77, 571);
    creature.id = 7001;
    creature.spawn_point = SpawnPosition::new(533.0, -533.0, 12.0, 1.0);
    store.add_object_spawn(&creature, |_| false);
    let mut gameobject = test_spawn(88, 571);
    gameobject.object_type = SpawnObjectType::GameObject;
    gameobject.id = 9001;
    gameobject.spawn_point = SpawnPosition::new(-100.0, 200.0, 13.0, 2.0);
    store.add_object_spawn(&gameobject, |_| false);
    let metadata = Arc::new(Mutex::new(
        super::spawn_store_loader::CanonicalSpawnMetadataLikeCpp::new(store, BTreeMap::new()),
    ));
    let mut snapshot = PersistedRespawnTimesLikeCpp::default();
    snapshot.push(
        wow_map::MapKey::new(571, 0),
        RespawnInfoLikeCpp {
            object_type: SpawnObjectType::Creature,
            spawn_id: 77,
            entry: 7001,
            respawn_time: 12345,
            grid_id: wow_map::compute_grid_coord(creature.spawn_point.x, creature.spawn_point.y)
                .get_id(),
        },
    );
    snapshot.push(
        wow_map::MapKey::new(571, 0),
        RespawnInfoLikeCpp {
            object_type: SpawnObjectType::GameObject,
            spawn_id: 88,
            entry: 9001,
            respawn_time: 67890,
            grid_id: wow_map::compute_grid_coord(
                gameobject.spawn_point.x,
                gameobject.spawn_point.y,
            )
            .get_id(),
        },
    );
    let mut manager = wow_map::MapManager::new(60_000, 10);
    install_canonical_spawn_group_initializer_like_cpp(
        &mut manager,
        metadata,
        Arc::new(ConditionEntriesByTypeStore::default()),
        Arc::new(snapshot),
        Arc::new(canonical_test_map_store_like_cpp()),
    );

    let map = manager.create_world_map(571, 0);
    assert_eq!(
        map.map()
            .get_respawn_time_like_cpp(SpawnObjectType::Creature, 77),
        12345
    );
    assert_eq!(
        map.map()
            .get_respawn_time_like_cpp(SpawnObjectType::GameObject, 88),
        67890
    );
    assert_eq!(
        map.map()
            .get_respawn_info_like_cpp(SpawnObjectType::Creature, 77)
            .expect("creature respawn loaded")
            .grid_id,
        wow_map::compute_grid_coord(creature.spawn_point.x, creature.spawn_point.y).get_id()
    );
}

#[test]
fn canonical_map_creation_init_pools_before_persisted_respawns_and_spawn_groups() {
    let mut pool_mgr = PoolMgrLikeCpp::new();
    pool_mgr.insert_template_like_cpp(10, PoolTemplateDataLikeCpp::new(1, 571));
    let mut group = PoolGroupLikeCpp::with_pool_id(PoolMemberKindLikeCpp::GameObject, 10);
    group.add_entry_like_cpp(PoolObjectLikeCpp::new(88, 0.0), 1);
    pool_mgr
        .insert_or_replace_group_like_cpp(PoolMemberKindLikeCpp::GameObject, 10, group)
        .expect("test pool group");
    pool_mgr.add_auto_spawn_pool_like_cpp(571, 10);

    let mut store = SpawnStore::new();
    let mut gameobject = test_spawn(88, 571);
    gameobject.object_type = SpawnObjectType::GameObject;
    gameobject.id = 9001;
    gameobject.spawn_point = SpawnPosition::new(-100.0, 200.0, 13.0, 2.0);
    store.add_object_spawn(&gameobject, |_| false);
    let metadata = Arc::new(Mutex::new(
        super::spawn_store_loader::CanonicalSpawnMetadataLikeCpp::new(store, BTreeMap::new())
            .with_pool_mgr_like_cpp(pool_mgr),
    ));
    let mut snapshot = PersistedRespawnTimesLikeCpp::default();
    snapshot.push(
        wow_map::MapKey::new(571, 0),
        RespawnInfoLikeCpp {
            object_type: SpawnObjectType::GameObject,
            spawn_id: 88,
            entry: 9001,
            respawn_time: 67890,
            grid_id: wow_map::compute_grid_coord(
                gameobject.spawn_point.x,
                gameobject.spawn_point.y,
            )
            .get_id(),
        },
    );
    let mut manager = wow_map::MapManager::new(60_000, 10);
    install_canonical_spawn_group_initializer_like_cpp(
        &mut manager,
        metadata,
        Arc::new(ConditionEntriesByTypeStore::default()),
        Arc::new(snapshot),
        Arc::new(canonical_test_map_store_like_cpp()),
    );

    let map = manager.create_world_map(571, 0);
    assert!(
        map.map()
            .pool_data_like_cpp()
            .is_spawned_gameobject_like_cpp(88)
    );
    assert_eq!(
        map.map()
            .pool_data_like_cpp()
            .get_spawned_objects_like_cpp(10),
        1
    );
    assert_eq!(
        map.map()
            .get_respawn_time_like_cpp(SpawnObjectType::GameObject, 88),
        67890
    );
}

#[test]
fn canonical_map_creation_skips_persisted_respawns_for_dungeon_maps() {
    let metadata = Arc::new(Mutex::new(test_spawn_metadata([])));
    let mut snapshot = PersistedRespawnTimesLikeCpp::default();
    snapshot.push(
        wow_map::MapKey::new(571, 1),
        RespawnInfoLikeCpp {
            object_type: SpawnObjectType::Creature,
            spawn_id: 1,
            entry: 42,
            respawn_time: 12345,
            grid_id: 7,
        },
    );
    let mut manager = wow_map::MapManager::new(60_000, 10);
    install_canonical_spawn_group_initializer_like_cpp(
        &mut manager,
        metadata,
        Arc::new(ConditionEntriesByTypeStore::default()),
        Arc::new(snapshot),
        Arc::new(canonical_test_map_store_like_cpp()),
    );

    let map = manager.create_map_entry(
        571,
        1,
        0,
        wow_map::ManagedMapKind::Dungeon {
            has_reset_schedule: false,
        },
    );
    assert_eq!(
        map.map()
            .get_respawn_time_like_cpp(SpawnObjectType::Creature, 1),
        0
    );
}

#[test]
fn canonical_map_creation_skips_persisted_respawns_for_instanceable_world_kind_like_cpp() {
    let metadata = Arc::new(Mutex::new(test_spawn_metadata([])));
    let mut snapshot = PersistedRespawnTimesLikeCpp::default();
    snapshot.push(
        wow_map::MapKey::new(1_151, 42),
        RespawnInfoLikeCpp {
            object_type: SpawnObjectType::Creature,
            spawn_id: 1,
            entry: 42,
            respawn_time: 12_345,
            grid_id: 7,
        },
    );
    let map_store = wow_data::MapStore::from_entries([wow_data::MapEntry {
        id: 1_151,
        instance_type: wow_data::map::MAP_SCENARIO,
        expansion_id: 0,
        parent_map_id: -1,
        cosmetic_parent_map_id: -1,
        flags1: wow_data::map::MAP_FLAG_GARRISON,
        flags2: 0,
    }]);
    let mut manager = wow_map::MapManager::new(60_000, 10);
    install_canonical_spawn_group_initializer_like_cpp(
        &mut manager,
        metadata,
        Arc::new(ConditionEntriesByTypeStore::default()),
        Arc::new(snapshot),
        Arc::new(map_store),
    );

    // The current canonical manager represents garrisons as `World`, but
    // C++ gates respawn persistence on `MapEntry::Instanceable()`.
    let map = manager.create_world_map(1_151, 42);
    assert_eq!(
        map.map()
            .get_respawn_time_like_cpp(SpawnObjectType::Creature, 1),
        0
    );
}

#[test]
fn persisted_respawn_loader_rejects_invalid_areatrigger_and_missing_metadata_rows() {
    let metadata = test_spawn_metadata([]);
    let mut report = PersistedRespawnLoadReportLikeCpp::default();

    assert!(
        persisted_respawn_info_from_row_like_cpp(
            RespawnPersistenceRowLikeCpp {
                object_type_raw: 99,
                spawn_id: 1,
                respawn_time: 10,
                map_id: 571,
                instance_id: 0,
            },
            &metadata,
            &mut report,
        )
        .is_none()
    );
    assert!(
        persisted_respawn_info_from_row_like_cpp(
            RespawnPersistenceRowLikeCpp {
                object_type_raw: 256,
                spawn_id: 1,
                respawn_time: 10,
                map_id: 571,
                instance_id: 0,
            },
            &metadata,
            &mut report,
        )
        .is_none()
    );
    assert!(
        persisted_respawn_info_from_row_like_cpp(
            RespawnPersistenceRowLikeCpp {
                object_type_raw: SpawnObjectType::AreaTrigger as u16,
                spawn_id: 1,
                respawn_time: 10,
                map_id: 571,
                instance_id: 0,
            },
            &metadata,
            &mut report,
        )
        .is_none()
    );
    assert!(
        persisted_respawn_info_from_row_like_cpp(
            RespawnPersistenceRowLikeCpp {
                object_type_raw: SpawnObjectType::Creature as u16,
                spawn_id: 404,
                respawn_time: 10,
                map_id: 571,
                instance_id: 0,
            },
            &metadata,
            &mut report,
        )
        .is_none()
    );

    assert_eq!(report.rows, 4);
    assert_eq!(report.loaded, 0);
    assert_eq!(report.invalid_type, 2);
    assert_eq!(report.unsupported_area_trigger, 1);
    assert_eq!(report.missing_spawn_metadata, 1);
}

// C++ anchors for the focused condition-update helper tests:
// - Maps/Map.cpp:666-688 (`Map::Update` respawn timer calls `UpdateSpawnGroupConditions`).
// - Maps/Map.cpp:2471-2502 (`UpdateSpawnGroupConditions` branch order).
// - Maps/Map.cpp:2427-2453 (map-owned spawn-group toggle state).
// - GameObject.cpp:772-779 and 4256-4277 (capture-point paths trigger condition updates).
#[test]
fn spawn_group_condition_update_set_inactive_applies_for_failed_automatic_group() {
    let metadata = test_spawn_metadata([(30, 571)]);
    let condition_store =
        ConditionEntriesByTypeStore::from_conditions_like_cpp([mapid_condition(30, 530)]);
    let mut manager = wow_map::MapManager::new(60_000, 10);
    let group = metadata
        .spawn_group_templates()
        .get(&30)
        .expect("test group 30");
    let map = manager.create_world_map(571, 0);
    assert!(map.map().is_spawn_group_active_like_cpp(Some(group)));

    let outcomes = apply_canonical_spawn_group_condition_update_loaded_grid_records_like_cpp(
        map,
        &metadata,
        &condition_store,
        &empty_loaded_grid_creature_respawn_caches_like_cpp(),
    );

    assert_eq!(outcomes.len(), 1);
    assert_eq!(outcomes[0].group_id, 30);
    assert_eq!(
        outcomes[0].action,
        wow_map::map::SpawnGroupConditionActionLikeCpp::SetInactive
    );
    assert!(matches!(
        outcomes[0].applied_change,
        Some(
            wow_map::SpawnGroupActiveChange::Toggled
                | wow_map::SpawnGroupActiveChange::ClearedToggle
        )
    ));
    assert!(!map.map().is_spawn_group_active_like_cpp(Some(group)));
}

#[test]
fn spawn_group_condition_update_set_inactive_executes_spawn_active_seam_and_despawn_toggles() {
    let metadata = test_spawn_metadata_with_flags([
        (40, 571, SpawnGroupFlags::NONE),
        (41, 571, SpawnGroupFlags::DESPAWN_ON_CONDITION_FAILURE),
    ]);
    let condition_store = ConditionEntriesByTypeStore::from_conditions_like_cpp([
        mapid_condition(40, 571),
        mapid_condition(41, 530),
    ]);
    let mut manager = wow_map::MapManager::new(60_000, 10);
    let spawn_group = metadata
        .spawn_group_templates()
        .get(&40)
        .expect("test group 40");
    let despawn_group = metadata
        .spawn_group_templates()
        .get(&41)
        .expect("test group 41");
    let map = manager.create_world_map(571, 0);
    map.map_mut()
        .set_spawn_group_inactive_like_cpp(Some(spawn_group));
    assert!(!map.map().is_spawn_group_active_like_cpp(Some(spawn_group)));
    assert!(
        map.map()
            .is_spawn_group_active_like_cpp(Some(despawn_group))
    );

    let outcomes = apply_canonical_spawn_group_condition_update_loaded_grid_records_like_cpp(
        map,
        &metadata,
        &condition_store,
        &empty_loaded_grid_creature_respawn_caches_like_cpp(),
    );

    let spawn_outcome = outcomes
        .iter()
        .find(|outcome| outcome.group_id == 40)
        .expect("spawn outcome");
    assert_eq!(
        spawn_outcome.action,
        wow_map::map::SpawnGroupConditionActionLikeCpp::spawn_group_spawn_default()
    );
    assert_eq!(spawn_outcome.applied_change, None);
    let spawn = spawn_outcome
        .spawn_outcome
        .as_ref()
        .expect("condition-success spawn executes active-state seam");
    assert_eq!(spawn.blocked_missing_group, 0);
    assert_eq!(spawn.blocked_system_group, 0);
    assert_eq!(
        spawn.applied_active_change,
        Some(wow_map::SpawnGroupActiveChange::ClearedToggle)
    );
    let despawn_outcome = outcomes
        .iter()
        .find(|outcome| outcome.group_id == 41)
        .expect("despawn outcome");
    assert_eq!(
        despawn_outcome.action,
        wow_map::map::SpawnGroupConditionActionLikeCpp::condition_failure_despawn()
    );
    assert_eq!(despawn_outcome.applied_change, None);
    let despawn = despawn_outcome
        .despawn_outcome
        .expect("condition-failure despawn executes");
    assert_eq!(despawn.blocked_missing_group, 0);
    assert_eq!(despawn.blocked_system_group, 0);
    assert_eq!(
        despawn.applied_inactive_change,
        Some(wow_map::SpawnGroupActiveChange::Toggled)
    );
    assert!(map.map().is_spawn_group_active_like_cpp(Some(spawn_group)));
    assert!(
        !map.map()
            .is_spawn_group_active_like_cpp(Some(despawn_group))
    );
}

#[test]
fn spawn_group_condition_update_set_inactive_no_groups_is_noop() {
    let metadata = test_spawn_metadata([]);
    let condition_store = ConditionEntriesByTypeStore::default();
    let mut manager = wow_map::MapManager::new(60_000, 10);
    let map = manager.create_world_map(999, 0);

    let outcomes = apply_canonical_spawn_group_condition_update_loaded_grid_records_like_cpp(
        map,
        &metadata,
        &condition_store,
        &empty_loaded_grid_creature_respawn_caches_like_cpp(),
    );

    assert!(outcomes.is_empty());
    assert!(
        map.map()
            .spawn_group_state()
            .toggled_spawn_group_ids()
            .is_empty()
    );
}

#[test]
fn respawn_condition_scheduler_like_cpp_waits_fires_and_resets() {
    let mut scheduler = CanonicalRespawnConditionSchedulerLikeCpp::new(100);

    assert!(!scheduler.update(40));
    assert_eq!(scheduler.timer_ms(), 60);
    assert!(!scheduler.update(59));
    assert_eq!(scheduler.timer_ms(), 1);
    assert!(scheduler.update(1));
    assert_eq!(scheduler.timer_ms(), 100);
    assert!(scheduler.update(150));
    assert_eq!(scheduler.timer_ms(), 100);
    assert!(!scheduler.update(25));
    assert_eq!(scheduler.timer_ms(), 75);
}

#[test]
fn game_event_scheduler_like_cpp_waits_fires_resets_and_installs_dynamic_delay() {
    let mut scheduler = CanonicalGameEventSchedulerLikeCpp::start_system(100);

    assert_eq!(scheduler.interval_ms(), 100);
    assert!(!scheduler.update(40));
    assert_eq!(scheduler.timer_ms(), 60);
    assert!(!scheduler.update(59));
    assert_eq!(scheduler.timer_ms(), 1);
    assert!(scheduler.update(1));
    assert_eq!(scheduler.timer_ms(), 100);

    scheduler.set_interval_and_reset(250);
    assert_eq!(scheduler.interval_ms(), 250);
    assert_eq!(scheduler.timer_ms(), 250);
    assert!(!scheduler.update(249));
    assert_eq!(scheduler.timer_ms(), 1);
    assert!(scheduler.update(1));
    assert_eq!(scheduler.timer_ms(), 250);

    scheduler.set_interval_and_reset(u64::from(u32::MAX) + 1);
    assert_eq!(scheduler.interval_ms(), u32::MAX);
    assert_eq!(scheduler.timer_ms(), u32::MAX);
    scheduler.set_interval_and_reset(0);
    assert_eq!(scheduler.interval_ms(), 1);
    assert_eq!(scheduler.timer_ms(), 1);
}

#[test]
fn game_event_start_system_first_update_records_negative_spawn_then_init_update_skips_it() {
    let event = spawn_store_loader::GameEventDataLikeCpp {
        event_id: 1,
        start: 100,
        end: 1_000,
        occurence: 10,
        length: 2,
        ..spawn_store_loader::GameEventDataLikeCpp::default()
    };
    let store =
        spawn_store_loader::GameEventDataStoreLikeCpp::from_game_event_max_entry_like_cpp(Some(1))
            .with_event_like_cpp(event);
    let mut metadata = spawn_store_loader::CanonicalSpawnMetadataLikeCpp::new(
        SpawnStore::default(),
        BTreeMap::new(),
    )
    .with_game_events_like_cpp(store);

    metadata.clear_active_game_events_like_cpp();
    let start_outcome = metadata.update_game_events_like_cpp(650, false, |_| false);
    assert_eq!(start_outcome.negative_spawn_event_ids, vec![-1]);
    assert_eq!(start_outcome.next_update_delay_millis, 51_000);
    let mut scheduler =
        CanonicalGameEventSchedulerLikeCpp::start_system(start_outcome.next_update_delay_millis);
    assert_eq!(scheduler.interval_ms(), 51_000);

    assert!(scheduler.update(51_000));
    let tick_outcome = metadata.update_game_events_like_cpp(650, true, |_| false);
    scheduler.set_interval_and_reset(tick_outcome.next_update_delay_millis);
    assert!(tick_outcome.negative_spawn_event_ids.is_empty());
    assert_eq!(
        scheduler.interval_ms(),
        tick_outcome.next_update_delay_millis as u32
    );
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

#[test]
fn game_event_db_bridge_materializes_semantic_save_with_zero_next_start_like_cpp() {
    let metadata = game_event_world_state_metadata_like_cpp(
        1,
        &[spawn_store_loader::GameEventDataLikeCpp {
            event_id: 1,
            state_raw: 2,
            next_start: 0,
            ..spawn_store_loader::GameEventDataLikeCpp::default()
        }],
    );
    let mut outcome = empty_game_event_update_outcome_for_db_bridge_like_cpp();
    outcome.start_outcomes = vec![spawn_store_loader::GameEventStartOutcomeLikeCpp::Started(
        spawn_store_loader::GameEventStartSummaryLikeCpp {
            event_id: 1,
            state_before_raw: 1,
            state_after_raw: 2,
            active_added: true,
            active_was_present: false,
            apply_new_event_requested: true,
            save_world_event_state_requested: true,
            force_game_event_update_requested: false,
            completed: false,
        },
    )];

    let summary = materialize_game_event_world_event_state_db_bridge_like_cpp(&outcome, &metadata);

    assert_eq!(summary.saves_queued, 1);
    assert_eq!(summary.operations.len(), 1);
    assert_game_event_save_operation_like_cpp(&summary.operations[0], 1, 2, 0);
}

#[tokio::test]
async fn game_event_db_bridge_classifies_typed_port_failure_and_success_like_cpp() {
    let metadata = game_event_world_state_metadata_like_cpp(
        1,
        &[spawn_store_loader::GameEventDataLikeCpp {
            event_id: 1,
            state_raw: 2,
            next_start: 0,
            ..spawn_store_loader::GameEventDataLikeCpp::default()
        }],
    );
    let mut outcome = empty_game_event_update_outcome_for_db_bridge_like_cpp();
    outcome.world_conditions_save_requested =
        vec![spawn_store_loader::GameEventWorldStateSaveEvidenceLikeCpp {
            event_id: 1,
            state_after_raw: 2,
            next_start_after: 0,
        }];
    let port = FakeGameEventPersistencePortLikeCpp::default();
    port.fail_mutations
        .store(true, std::sync::atomic::Ordering::Release);
    let mut failed =
        materialize_game_event_world_event_state_db_bridge_like_cpp(&outcome, &metadata);
    execute_game_event_world_event_state_db_bridge_like_cpp(&port, &mut failed).await;
    assert_eq!(failed.saves_executed, 0);
    assert_eq!(failed.saves_failed, 1);
    assert!(failed.operations.is_empty());

    port.fail_mutations
        .store(false, std::sync::atomic::Ordering::Release);
    let mut applied =
        materialize_game_event_world_event_state_db_bridge_like_cpp(&outcome, &metadata);
    execute_game_event_world_event_state_db_bridge_like_cpp(&port, &mut applied).await;
    assert_eq!(applied.saves_executed, 1);
    assert_eq!(applied.saves_failed, 0);
    assert_eq!(port.mutations.lock().unwrap().len(), 2);
}

#[test]
fn game_event_db_bridge_materializes_world_nextphase_and_conditions_in_cpp_order() {
    let metadata = game_event_world_state_metadata_like_cpp(
        3,
        &[
            spawn_store_loader::GameEventDataLikeCpp {
                event_id: 1,
                state_raw: 3,
                next_start: 10,
                ..spawn_store_loader::GameEventDataLikeCpp::default()
            },
            spawn_store_loader::GameEventDataLikeCpp {
                event_id: 2,
                state_raw: 4,
                next_start: 20,
                ..spawn_store_loader::GameEventDataLikeCpp::default()
            },
        ],
    );
    let mut outcome = empty_game_event_update_outcome_for_db_bridge_like_cpp();
    outcome.world_nextphase_finished =
        vec![spawn_store_loader::GameEventWorldNextPhaseFinishedLikeCpp {
            event_id: 2,
            was_active_before_queue: true,
            state_before_raw: 1,
            state_after_raw: 4,
            next_start_before: 0,
            next_start_after: 20,
            save_state_requested: true,
        }];
    outcome.world_conditions_save_requested =
        vec![spawn_store_loader::GameEventWorldStateSaveEvidenceLikeCpp {
            event_id: 1,
            state_after_raw: 3,
            next_start_after: 10,
        }];

    let summary = materialize_game_event_world_event_state_db_bridge_like_cpp(&outcome, &metadata);

    assert_eq!(summary.saves_queued, 2);
    assert_eq!(summary.operations.len(), 2);
    assert_game_event_save_operation_like_cpp(&summary.operations[0], 2, 4, 20);
    assert_game_event_save_operation_like_cpp(&summary.operations[1], 1, 3, 10);
}

#[test]
fn game_event_db_bridge_materializes_stop_delete_condition_saves_before_event_save() {
    let metadata = game_event_world_state_metadata_like_cpp(1, &[]);
    let mut outcome = empty_game_event_update_outcome_for_db_bridge_like_cpp();
    outcome.stop_outcomes = vec![spawn_store_loader::GameEventStopOutcomeLikeCpp::Stopped(
        spawn_store_loader::GameEventStopSummaryLikeCpp {
            event_id: 1,
            state_before_raw: 1,
            state_after_raw: 0,
            active_removed: true,
            active_was_present: true,
            unapply_event_requested: true,
            serverwide: true,
            condition_reset_requested: true,
            delete_world_event_state_requested: true,
            delete_condition_saves_requested: true,
        },
    )];

    let summary = materialize_game_event_world_event_state_db_bridge_like_cpp(&outcome, &metadata);

    assert_eq!(summary.deletes_queued, 1);
    assert_eq!(summary.condition_delete_rows_queued, 1);
    assert_eq!(summary.operations.len(), 1);
    let operation = &summary.operations[0];
    assert_eq!(
        operation.kind,
        GameEventWorldEventStateDbOperationKindLikeCpp::Delete
    );
    assert!(operation.delete_condition_saves);
    assert!(operation.delete_world_event_state);
    assert_eq!(
        operation.mutation,
        wow_persistence::GameEventPersistenceMutationLikeCpp::DeleteWorldEventState {
            event_id: 1,
            delete_condition_saves: true,
            delete_world_event_state: true,
        }
    );
}

#[test]
fn game_event_db_bridge_finished_no_overwrite_stop_without_delete_flags_is_noop() {
    let metadata = game_event_world_state_metadata_like_cpp(1, &[]);
    let mut outcome = empty_game_event_update_outcome_for_db_bridge_like_cpp();
    outcome.stop_outcomes = vec![spawn_store_loader::GameEventStopOutcomeLikeCpp::Stopped(
        spawn_store_loader::GameEventStopSummaryLikeCpp {
            event_id: 1,
            state_before_raw: 2,
            state_after_raw: 2,
            active_removed: false,
            active_was_present: true,
            unapply_event_requested: false,
            serverwide: true,
            condition_reset_requested: false,
            delete_world_event_state_requested: false,
            delete_condition_saves_requested: false,
        },
    )];

    let summary = materialize_game_event_world_event_state_db_bridge_like_cpp(&outcome, &metadata);

    assert_eq!(summary.deletes_queued, 0);
    assert_eq!(summary.condition_delete_rows_queued, 0);
    assert!(summary.operations.is_empty());
}

#[test]
fn game_event_db_bridge_out_of_range_event_id_skips_without_panic() {
    let metadata = game_event_world_state_metadata_like_cpp(
        300,
        &[spawn_store_loader::GameEventDataLikeCpp {
            event_id: 300,
            state_raw: 1,
            next_start: 0,
            ..spawn_store_loader::GameEventDataLikeCpp::default()
        }],
    );
    let mut outcome = empty_game_event_update_outcome_for_db_bridge_like_cpp();
    outcome.world_conditions_save_requested =
        vec![spawn_store_loader::GameEventWorldStateSaveEvidenceLikeCpp {
            event_id: 300,
            state_after_raw: 1,
            next_start_after: 0,
        }];
    outcome.stop_outcomes = vec![spawn_store_loader::GameEventStopOutcomeLikeCpp::Stopped(
        spawn_store_loader::GameEventStopSummaryLikeCpp {
            event_id: 300,
            state_before_raw: 1,
            state_after_raw: 0,
            active_removed: true,
            active_was_present: true,
            unapply_event_requested: true,
            serverwide: true,
            condition_reset_requested: true,
            delete_world_event_state_requested: true,
            delete_condition_saves_requested: true,
        },
    )];

    let summary = materialize_game_event_world_event_state_db_bridge_like_cpp(&outcome, &metadata);

    assert_eq!(summary.saves_skipped_event_id_out_of_range, 1);
    assert_eq!(summary.deletes_skipped_event_id_out_of_range, 1);
    assert_eq!(summary.saves_queued, 0);
    assert_eq!(summary.deletes_queued, 0);
    assert!(summary.operations.is_empty());
}

#[test]
fn game_event_quest_complete_db_bridge_materializes_condition_save_then_world_event_save() {
    let metadata = game_event_world_state_metadata_like_cpp(
        7,
        &[spawn_store_loader::GameEventDataLikeCpp {
            event_id: 7,
            state_raw: 3,
            next_start: 1_234,
            ..spawn_store_loader::GameEventDataLikeCpp::default()
        }],
    );
    let outcome = game_event_quest_complete_progressed_outcome_like_cpp(true, true);

    let summary = materialize_game_event_quest_complete_db_bridge_like_cpp(&outcome, &metadata);

    assert_eq!(summary.condition_save_updates_queued, 1);
    assert_eq!(summary.condition_save_updates_skipped_non_progress, 0);
    assert_eq!(summary.world_event_state_save_requested, 1);
    assert_eq!(summary.force_game_event_update_requested, 1);
    assert!(summary.save_world_event_state_requested);
    assert!(summary.force_game_event_update_requested_flag);
    assert_eq!(summary.operations.len(), 1);

    let operation = &summary.operations[0];
    assert_eq!(operation.event_id, 7);
    assert_eq!(operation.condition_id, 44);
    assert_eq!(
        operation.mutation,
        wow_persistence::GameEventPersistenceMutationLikeCpp::ReplaceConditionSave {
            event_id: 7,
            condition_id: 44,
            done: 5.25,
        }
    );

    assert_eq!(summary.world_event_state_summary.saves_queued, 1);
    assert_eq!(
        summary
            .world_event_state_summary
            .saves_skipped_missing_event,
        0
    );
    assert_eq!(
        summary
            .world_event_state_summary
            .saves_skipped_event_id_out_of_range,
        0
    );
    assert_eq!(summary.world_event_state_summary.operations.len(), 1);
    assert_game_event_save_operation_like_cpp(
        &summary.world_event_state_summary.operations[0],
        7,
        3,
        1_234,
    );
}

#[test]
fn game_event_quest_complete_response_dto_includes_condition_and_world_event_flags_like_cpp() {
    let metadata = game_event_world_state_metadata_like_cpp(
        7,
        &[spawn_store_loader::GameEventDataLikeCpp {
            event_id: 7,
            state_raw: 3,
            next_start: 5_000,
            ..Default::default()
        }],
    );
    let outcome = game_event_quest_complete_progressed_outcome_like_cpp(true, true);

    let mut summary = materialize_game_event_quest_complete_db_bridge_like_cpp(&outcome, &metadata);
    summary.condition_save_updates_executed = 1;
    summary.world_event_state_summary.saves_executed = 1;
    let response = game_event_quest_complete_response_from_summary_like_cpp(1234, &summary);

    assert_eq!(response.quest_id, 1234);
    assert_eq!(response.condition_save_updates_queued, 1);
    assert_eq!(response.condition_save_updates_executed, 1);
    assert_eq!(response.condition_save_updates_failed, 0);
    assert_eq!(response.condition_save_updates_skipped_non_progress, 0);
    assert!(response.save_world_event_state_requested);
    assert_eq!(response.world_event_state_save_requested, 1);
    assert_eq!(response.world_event_state_saves_queued, 1);
    assert_eq!(response.world_event_state_saves_executed, 1);
    assert_eq!(response.world_event_state_saves_failed, 0);
    assert!(response.force_game_event_update_requested);
    assert_eq!(response.force_game_event_update_requests, 1);
    assert!(!response.processor_failed);
}

#[test]
fn game_event_quest_complete_response_dto_reports_non_progress_noop_like_cpp() {
    let metadata = game_event_world_state_metadata_like_cpp(7, &[]);
    let outcome = spawn_store_loader::GameEventQuestCompleteOutcomeLikeCpp::MissingQuestMapping {
        quest_id: 9999,
    };

    let summary = materialize_game_event_quest_complete_db_bridge_like_cpp(&outcome, &metadata);
    let response = game_event_quest_complete_response_from_summary_like_cpp(9999, &summary);

    assert_eq!(response.quest_id, 9999);
    assert_eq!(response.condition_save_updates_queued, 0);
    assert_eq!(response.condition_save_updates_skipped_non_progress, 1);
    assert!(!response.save_world_event_state_requested);
    assert_eq!(response.world_event_state_saves_queued, 0);
    assert!(!response.force_game_event_update_requested);
    assert!(!response.processor_failed);
}

#[test]
fn game_event_quest_complete_db_bridge_preserves_condition_save_without_world_event_save() {
    let metadata = game_event_world_state_metadata_like_cpp(
        7,
        &[spawn_store_loader::GameEventDataLikeCpp {
            event_id: 7,
            state_raw: 2,
            next_start: 0,
            ..spawn_store_loader::GameEventDataLikeCpp::default()
        }],
    );
    let outcome = game_event_quest_complete_progressed_outcome_like_cpp(false, false);

    let summary = materialize_game_event_quest_complete_db_bridge_like_cpp(&outcome, &metadata);

    assert_eq!(summary.condition_save_updates_queued, 1);
    assert_eq!(summary.operations.len(), 1);
    assert_eq!(summary.world_event_state_save_requested, 0);
    assert!(!summary.save_world_event_state_requested);
    assert_eq!(summary.world_event_state_summary.saves_queued, 0);
    assert!(summary.world_event_state_summary.operations.is_empty());
}

#[test]
fn game_event_quest_complete_db_bridge_skips_world_event_save_when_metadata_missing() {
    let metadata = game_event_world_state_metadata_like_cpp(6, &[]);
    let outcome = game_event_quest_complete_progressed_outcome_like_cpp(true, true);

    let summary = materialize_game_event_quest_complete_db_bridge_like_cpp(&outcome, &metadata);

    assert_eq!(summary.condition_save_updates_queued, 1);
    assert_eq!(summary.operations.len(), 1);
    assert_eq!(summary.world_event_state_save_requested, 1);
    assert!(summary.save_world_event_state_requested);
    assert_eq!(summary.world_event_state_summary.saves_queued, 0);
    assert_eq!(
        summary
            .world_event_state_summary
            .saves_skipped_missing_event,
        1
    );
    assert!(summary.world_event_state_summary.operations.is_empty());
}

#[test]
fn game_event_quest_complete_db_bridge_skips_missing_or_non_progress() {
    let metadata = game_event_world_state_metadata_like_cpp(
        7,
        &[spawn_store_loader::GameEventDataLikeCpp {
            event_id: 7,
            state_raw: 3,
            next_start: 1_234,
            ..spawn_store_loader::GameEventDataLikeCpp::default()
        }],
    );
    let missing = spawn_store_loader::GameEventQuestCompleteOutcomeLikeCpp::MissingQuestMapping {
        quest_id: 12_345,
    };
    let missing_summary =
        materialize_game_event_quest_complete_db_bridge_like_cpp(&missing, &metadata);
    assert_eq!(missing_summary.condition_save_updates_queued, 0);
    assert_eq!(
        missing_summary.condition_save_updates_skipped_non_progress,
        1
    );
    assert!(missing_summary.operations.is_empty());
    assert_eq!(missing_summary.world_event_state_summary.saves_queued, 0);
    assert!(
        missing_summary
            .world_event_state_summary
            .operations
            .is_empty()
    );

    let inactive = spawn_store_loader::GameEventQuestCompleteOutcomeLikeCpp::Progress(
        spawn_store_loader::GameEventConditionProgressOutcomeLikeCpp::InactiveEvent { event_id: 7 },
    );
    let inactive_summary =
        materialize_game_event_quest_complete_db_bridge_like_cpp(&inactive, &metadata);
    assert_eq!(inactive_summary.condition_save_updates_queued, 0);
    assert_eq!(
        inactive_summary.condition_save_updates_skipped_non_progress,
        1
    );
    assert!(inactive_summary.operations.is_empty());
    assert_eq!(inactive_summary.world_event_state_summary.saves_queued, 0);
    assert!(
        inactive_summary
            .world_event_state_summary
            .operations
            .is_empty()
    );

    let already_complete = spawn_store_loader::GameEventQuestCompleteOutcomeLikeCpp::Progress(
        spawn_store_loader::GameEventConditionProgressOutcomeLikeCpp::AlreadyComplete {
            event_id: 7,
            condition_id: 44,
            done: 10.0,
            req_num: 10.0,
        },
    );
    let complete_summary =
        materialize_game_event_quest_complete_db_bridge_like_cpp(&already_complete, &metadata);
    assert_eq!(complete_summary.condition_save_updates_queued, 0);
    assert_eq!(
        complete_summary.condition_save_updates_skipped_non_progress,
        1
    );
    assert!(complete_summary.operations.is_empty());
    assert_eq!(complete_summary.world_event_state_summary.saves_queued, 0);
    assert!(
        complete_summary
            .world_event_state_summary
            .operations
            .is_empty()
    );
}

#[test]
fn game_event_world_state_no_holiday_action_is_represented_noop_like_cpp() {
    let mut metadata = game_event_world_state_metadata_like_cpp(
        1,
        &[spawn_store_loader::GameEventDataLikeCpp {
            event_id: 1,
            holiday_id: 0,
            length: 1,
            ..spawn_store_loader::GameEventDataLikeCpp::default()
        }],
    );
    let mut manager = wow_map::MapManager::default();
    let outcome = game_event_world_state_start_outcome_like_cpp(1);

    let summary = consume_game_event_live_update_side_effects_like_cpp(
        &mut manager,
        None,
        &mut metadata,
        &empty_loaded_grid_creature_respawn_caches_like_cpp(),
        None,
        None,
        None,
        &[1],
        &outcome,
        false,
    );

    assert!(
        summary
            .actions
            .contains(&GameEventLiveUpdateActionLikeCpp::UpdateWorldStates {
                event_id: 1,
                activate: true,
            })
    );
    assert_eq!(summary.update_world_states_actions, 1);
    assert_eq!(summary.update_world_states_no_holiday, 1);
    assert_eq!(summary.update_world_states_missing_event, 0);
    assert_eq!(summary.update_world_states_holiday_lookup_unrepresented, 0);
}

#[test]
fn game_event_world_state_holiday_lookup_remains_unrepresented_like_cpp() {
    let mut metadata = game_event_world_state_metadata_like_cpp(
        1,
        &[spawn_store_loader::GameEventDataLikeCpp {
            event_id: 1,
            holiday_id: 283,
            length: 1,
            ..spawn_store_loader::GameEventDataLikeCpp::default()
        }],
    );
    let mut manager = wow_map::MapManager::default();
    let outcome = game_event_world_state_start_outcome_like_cpp(1);

    let summary = consume_game_event_live_update_side_effects_like_cpp(
        &mut manager,
        None,
        &mut metadata,
        &empty_loaded_grid_creature_respawn_caches_like_cpp(),
        None,
        None,
        None,
        &[1],
        &outcome,
        false,
    );

    assert_eq!(summary.update_world_states_actions, 1);
    assert_eq!(summary.update_world_states_no_holiday, 0);
    assert_eq!(summary.update_world_states_missing_event, 0);
    assert_eq!(summary.update_world_states_holiday_lookup_unrepresented, 1);
}

#[test]
fn game_event_world_state_missing_event_is_counted_without_panic_like_cpp() {
    let mut metadata = game_event_world_state_metadata_like_cpp(0, &[]);
    let mut manager = wow_map::MapManager::default();
    let outcome = game_event_world_state_start_outcome_like_cpp(1);

    let summary = consume_game_event_live_update_side_effects_like_cpp(
        &mut manager,
        None,
        &mut metadata,
        &empty_loaded_grid_creature_respawn_caches_like_cpp(),
        None,
        None,
        None,
        &[],
        &outcome,
        false,
    );

    assert_eq!(summary.update_world_states_actions, 1);
    assert_eq!(summary.update_world_states_missing_event, 1);
    assert_eq!(summary.update_world_states_no_holiday, 0);
    assert_eq!(summary.update_world_states_holiday_lookup_unrepresented, 0);
}

#[test]
fn game_event_world_state_holiday_set_value_activate_is_represented_like_cpp() {
    let metadata = game_event_world_state_metadata_like_cpp(
        1,
        &[spawn_store_loader::GameEventDataLikeCpp {
            event_id: 1,
            holiday_id: wow_data::HOLIDAY_CALL_TO_ARMS_AV_LIKE_CPP,
            length: 1,
            ..spawn_store_loader::GameEventDataLikeCpp::default()
        }],
    );
    let store = wow_data::BattlemasterListStore::from_entries([wow_data::BattlemasterListEntry {
        id: wow_data::BATTLEGROUND_AV_LIKE_CPP,
        instance_type: wow_data::MAP_BATTLEGROUND_LIKE_CPP,
        holiday_world_state: 777,
        flags: 0,
    }]);

    let summary =
        game_event_update_world_states_like_cpp(&metadata, Some(&store), None, None, 1, true);

    assert_eq!(summary.update_world_states_set_value_represented, 1);
    assert_eq!(summary.update_world_states_last_world_state_id, Some(777));
    assert_eq!(summary.update_world_states_last_world_state_value, Some(1));
    assert_eq!(summary.update_world_states_holiday_lookup_unrepresented, 0);
}

#[test]
fn game_event_world_state_holiday_set_value_deactivate_is_represented_like_cpp() {
    let metadata = game_event_world_state_metadata_like_cpp(
        1,
        &[spawn_store_loader::GameEventDataLikeCpp {
            event_id: 1,
            holiday_id: wow_data::HOLIDAY_CALL_TO_ARMS_AB_LIKE_CPP,
            length: 1,
            ..spawn_store_loader::GameEventDataLikeCpp::default()
        }],
    );
    let store = wow_data::BattlemasterListStore::from_entries([wow_data::BattlemasterListEntry {
        id: wow_data::BATTLEGROUND_AB_LIKE_CPP,
        instance_type: wow_data::MAP_BATTLEGROUND_LIKE_CPP,
        holiday_world_state: 888,
        flags: 0,
    }]);

    let summary =
        game_event_update_world_states_like_cpp(&metadata, Some(&store), None, None, 1, false);

    assert_eq!(summary.update_world_states_set_value_represented, 1);
    assert_eq!(summary.update_world_states_last_world_state_id, Some(888));
    assert_eq!(summary.update_world_states_last_world_state_value, Some(0));
    assert_eq!(summary.update_world_states_holiday_lookup_unrepresented, 0);
}

#[test]
fn game_event_world_state_live_consumer_propagates_holiday_lookup_counters_like_cpp() {
    fn consume_world_state_summary_like_cpp(
        metadata: &mut spawn_store_loader::CanonicalSpawnMetadataLikeCpp,
        battlemaster_list_store: Option<&wow_data::BattlemasterListStore>,
    ) -> GameEventLiveUpdateSideEffectSummaryLikeCpp {
        let mut manager = wow_map::MapManager::default();
        let outcome = game_event_world_state_start_outcome_like_cpp(1);
        consume_game_event_live_update_side_effects_like_cpp(
            &mut manager,
            None,
            metadata,
            &empty_loaded_grid_creature_respawn_caches_like_cpp(),
            battlemaster_list_store,
            None,
            None,
            &[1],
            &outcome,
            false,
        )
    }

    let mut missing_store_metadata = game_event_world_state_metadata_like_cpp(
        1,
        &[spawn_store_loader::GameEventDataLikeCpp {
            event_id: 1,
            holiday_id: wow_data::HOLIDAY_CALL_TO_ARMS_AV_LIKE_CPP,
            length: 1,
            ..spawn_store_loader::GameEventDataLikeCpp::default()
        }],
    );
    let missing_store_summary =
        consume_world_state_summary_like_cpp(&mut missing_store_metadata, None);
    assert_eq!(missing_store_summary.update_world_states_actions, 1);
    assert_eq!(missing_store_summary.update_world_states_store_missing, 1);
    assert_eq!(
        missing_store_summary.update_world_states_holiday_lookup_unrepresented,
        1
    );
    assert_eq!(
        missing_store_summary.update_world_states_battlemaster_list_missing,
        0
    );
    assert_eq!(
        missing_store_summary.update_world_states_holiday_world_state_zero,
        0
    );

    let missing_battlemaster_store = wow_data::BattlemasterListStore::from_entries([]);
    let mut missing_battlemaster_metadata = game_event_world_state_metadata_like_cpp(
        1,
        &[spawn_store_loader::GameEventDataLikeCpp {
            event_id: 1,
            holiday_id: wow_data::HOLIDAY_CALL_TO_ARMS_AV_LIKE_CPP,
            length: 1,
            ..spawn_store_loader::GameEventDataLikeCpp::default()
        }],
    );
    let missing_battlemaster_summary = consume_world_state_summary_like_cpp(
        &mut missing_battlemaster_metadata,
        Some(&missing_battlemaster_store),
    );
    assert_eq!(
        missing_battlemaster_summary.update_world_states_store_missing,
        0
    );
    assert_eq!(
        missing_battlemaster_summary.update_world_states_battlemaster_list_missing,
        1
    );
    assert_eq!(
        missing_battlemaster_summary.update_world_states_holiday_lookup_unrepresented,
        1
    );
    assert_eq!(
        missing_battlemaster_summary.update_world_states_holiday_world_state_zero,
        0
    );

    let zero_store =
        wow_data::BattlemasterListStore::from_entries([wow_data::BattlemasterListEntry {
            id: wow_data::BATTLEGROUND_AV_LIKE_CPP,
            instance_type: wow_data::MAP_BATTLEGROUND_LIKE_CPP,
            holiday_world_state: 0,
            flags: 0,
        }]);
    let mut zero_metadata = game_event_world_state_metadata_like_cpp(
        1,
        &[spawn_store_loader::GameEventDataLikeCpp {
            event_id: 1,
            holiday_id: wow_data::HOLIDAY_CALL_TO_ARMS_AV_LIKE_CPP,
            length: 1,
            ..spawn_store_loader::GameEventDataLikeCpp::default()
        }],
    );
    let zero_summary = consume_world_state_summary_like_cpp(&mut zero_metadata, Some(&zero_store));
    assert_eq!(zero_summary.update_world_states_store_missing, 0);
    assert_eq!(
        zero_summary.update_world_states_battlemaster_list_missing,
        0
    );
    assert_eq!(zero_summary.update_world_states_holiday_world_state_zero, 1);
    assert_eq!(
        zero_summary.update_world_states_holiday_lookup_unrepresented,
        0
    );
    assert_eq!(zero_summary.update_world_states_set_value_represented, 0);
}

#[test]
fn game_event_world_state_missing_battlemaster_store_is_explicit_skip_like_cpp() {
    let metadata = game_event_world_state_metadata_like_cpp(
        1,
        &[spawn_store_loader::GameEventDataLikeCpp {
            event_id: 1,
            holiday_id: wow_data::HOLIDAY_CALL_TO_ARMS_AV_LIKE_CPP,
            length: 1,
            ..spawn_store_loader::GameEventDataLikeCpp::default()
        }],
    );

    let summary = game_event_update_world_states_like_cpp(&metadata, None, None, None, 1, true);

    assert_eq!(summary.update_world_states_store_missing, 1);
    assert_eq!(summary.update_world_states_holiday_lookup_unrepresented, 1);
    assert_eq!(summary.update_world_states_set_value_represented, 0);
}

#[test]
fn game_event_world_state_missing_or_zero_battlemaster_row_is_explicit_skip_like_cpp() {
    let metadata = game_event_world_state_metadata_like_cpp(
        1,
        &[spawn_store_loader::GameEventDataLikeCpp {
            event_id: 1,
            holiday_id: wow_data::HOLIDAY_CALL_TO_ARMS_AV_LIKE_CPP,
            length: 1,
            ..spawn_store_loader::GameEventDataLikeCpp::default()
        }],
    );
    let missing_store = wow_data::BattlemasterListStore::from_entries([]);
    let missing_summary = game_event_update_world_states_like_cpp(
        &metadata,
        Some(&missing_store),
        None,
        None,
        1,
        true,
    );
    assert_eq!(
        missing_summary.update_world_states_battlemaster_list_missing,
        1
    );
    assert_eq!(
        missing_summary.update_world_states_holiday_lookup_unrepresented,
        1
    );
    assert_eq!(missing_summary.update_world_states_set_value_represented, 0);

    let zero_store =
        wow_data::BattlemasterListStore::from_entries([wow_data::BattlemasterListEntry {
            id: wow_data::BATTLEGROUND_AV_LIKE_CPP,
            instance_type: wow_data::MAP_BATTLEGROUND_LIKE_CPP,
            holiday_world_state: 0,
            flags: 0,
        }]);
    let zero_summary =
        game_event_update_world_states_like_cpp(&metadata, Some(&zero_store), None, None, 1, true);
    assert_eq!(zero_summary.update_world_states_holiday_world_state_zero, 1);
    assert_eq!(
        zero_summary.update_world_states_holiday_lookup_unrepresented,
        0
    );
    assert_eq!(zero_summary.update_world_states_set_value_represented, 0);
}

#[test]
fn game_event_world_state_mgr_realm_default_change_global_message_represented_like_cpp() {
    let metadata = game_event_world_state_metadata_like_cpp(
        1,
        &[spawn_store_loader::GameEventDataLikeCpp {
            event_id: 1,
            holiday_id: wow_data::HOLIDAY_CALL_TO_ARMS_AV_LIKE_CPP,
            length: 1,
            ..spawn_store_loader::GameEventDataLikeCpp::default()
        }],
    );
    let store = wow_data::BattlemasterListStore::from_entries([wow_data::BattlemasterListEntry {
        id: wow_data::BATTLEGROUND_AV_LIKE_CPP,
        instance_type: wow_data::MAP_BATTLEGROUND_LIKE_CPP,
        holiday_world_state: 777,
        flags: 0,
    }]);
    let mut world_state_mgr =
        spawn_store_loader::WorldStateMgrLikeCpp::from_templates_and_saved_values(
            [spawn_store_loader::WorldStateTemplateLikeCpp::realm_wide(
                777, 0,
            )],
            [],
        );

    let summary = game_event_update_world_states_like_cpp(
        &metadata,
        Some(&store),
        Some(&mut world_state_mgr),
        None,
        1,
        true,
    );

    assert_eq!(summary.update_world_states_set_value_attempts, 1);
    assert_eq!(summary.update_world_states_realm_changed_or_inserted, 1);
    assert_eq!(summary.update_world_states_global_message_represented, 1);
    assert_eq!(summary.update_world_states_realm_unchanged_noop, 0);
    assert_eq!(summary.update_world_states_last_world_state_id, Some(777));
    assert_eq!(summary.update_world_states_last_world_state_value, Some(1));
    assert_eq!(world_state_mgr.realm_value_like_cpp(777), 1);
}

#[test]
fn game_event_world_state_mgr_realm_same_value_is_noop_like_cpp() {
    let metadata = game_event_world_state_metadata_like_cpp(
        1,
        &[spawn_store_loader::GameEventDataLikeCpp {
            event_id: 1,
            holiday_id: wow_data::HOLIDAY_CALL_TO_ARMS_AV_LIKE_CPP,
            length: 1,
            ..spawn_store_loader::GameEventDataLikeCpp::default()
        }],
    );
    let store = wow_data::BattlemasterListStore::from_entries([wow_data::BattlemasterListEntry {
        id: wow_data::BATTLEGROUND_AV_LIKE_CPP,
        instance_type: wow_data::MAP_BATTLEGROUND_LIKE_CPP,
        holiday_world_state: 778,
        flags: 0,
    }]);
    let mut world_state_mgr =
        spawn_store_loader::WorldStateMgrLikeCpp::from_templates_and_saved_values(
            [spawn_store_loader::WorldStateTemplateLikeCpp::realm_wide(
                778, 1,
            )],
            [],
        );

    let summary = game_event_update_world_states_like_cpp(
        &metadata,
        Some(&store),
        Some(&mut world_state_mgr),
        None,
        1,
        true,
    );

    assert_eq!(summary.update_world_states_set_value_attempts, 1);
    assert_eq!(summary.update_world_states_realm_unchanged_noop, 1);
    assert_eq!(summary.update_world_states_global_message_represented, 0);
    assert_eq!(world_state_mgr.realm_value_like_cpp(778), 1);
}

#[test]
fn game_event_world_state_mgr_missing_template_inserts_realm_value_like_cpp() {
    let metadata = game_event_world_state_metadata_like_cpp(
        1,
        &[spawn_store_loader::GameEventDataLikeCpp {
            event_id: 1,
            holiday_id: wow_data::HOLIDAY_CALL_TO_ARMS_AV_LIKE_CPP,
            length: 1,
            ..spawn_store_loader::GameEventDataLikeCpp::default()
        }],
    );
    let store = wow_data::BattlemasterListStore::from_entries([wow_data::BattlemasterListEntry {
        id: wow_data::BATTLEGROUND_AV_LIKE_CPP,
        instance_type: wow_data::MAP_BATTLEGROUND_LIKE_CPP,
        holiday_world_state: 779,
        flags: 0,
    }]);
    let mut world_state_mgr = spawn_store_loader::WorldStateMgrLikeCpp::default();

    let summary = game_event_update_world_states_like_cpp(
        &metadata,
        Some(&store),
        Some(&mut world_state_mgr),
        None,
        1,
        true,
    );

    assert_eq!(summary.update_world_states_set_value_attempts, 1);
    assert_eq!(summary.update_world_states_realm_changed_or_inserted, 1);
    assert_eq!(summary.update_world_states_global_message_represented, 1);
    assert_eq!(world_state_mgr.realm_value_like_cpp(779), 1);
}

#[test]
fn game_event_world_state_mgr_map_specific_null_map_is_unsupported_like_cpp() {
    let metadata = game_event_world_state_metadata_like_cpp(
        1,
        &[spawn_store_loader::GameEventDataLikeCpp {
            event_id: 1,
            holiday_id: wow_data::HOLIDAY_CALL_TO_ARMS_AV_LIKE_CPP,
            length: 1,
            ..spawn_store_loader::GameEventDataLikeCpp::default()
        }],
    );
    let store = wow_data::BattlemasterListStore::from_entries([wow_data::BattlemasterListEntry {
        id: wow_data::BATTLEGROUND_AV_LIKE_CPP,
        instance_type: wow_data::MAP_BATTLEGROUND_LIKE_CPP,
        holiday_world_state: 780,
        flags: 0,
    }]);
    let mut world_state_mgr =
        spawn_store_loader::WorldStateMgrLikeCpp::from_templates_and_saved_values(
            [spawn_store_loader::WorldStateTemplateLikeCpp::map_specific(
                780,
                0,
                [1],
            )],
            [],
        );

    let summary = game_event_update_world_states_like_cpp(
        &metadata,
        Some(&store),
        Some(&mut world_state_mgr),
        None,
        1,
        true,
    );

    assert_eq!(summary.update_world_states_set_value_attempts, 1);
    assert_eq!(
        summary.update_world_states_map_specific_no_map_unsupported,
        1
    );
    assert_eq!(summary.update_world_states_global_message_represented, 0);
    assert_eq!(world_state_mgr.realm_value_like_cpp(780), 0);
}

#[test]
fn game_event_world_state_global_fanout_sends_update_to_active_players_like_cpp() {
    let metadata = game_event_world_state_metadata_like_cpp(
        1,
        &[spawn_store_loader::GameEventDataLikeCpp {
            event_id: 1,
            holiday_id: wow_data::HOLIDAY_CALL_TO_ARMS_AV_LIKE_CPP,
            length: 1,
            ..spawn_store_loader::GameEventDataLikeCpp::default()
        }],
    );
    let store = wow_data::BattlemasterListStore::from_entries([wow_data::BattlemasterListEntry {
        id: wow_data::BATTLEGROUND_AV_LIKE_CPP,
        instance_type: wow_data::MAP_BATTLEGROUND_LIKE_CPP,
        holiday_world_state: 777,
        flags: 0,
    }]);
    let mut world_state_mgr =
        spawn_store_loader::WorldStateMgrLikeCpp::from_templates_and_saved_values(
            [spawn_store_loader::WorldStateTemplateLikeCpp::realm_wide(
                777, 0,
            )],
            [],
        );
    let registry = PlayerRegistry::with_canonical_player_fixtures_like_cpp();
    let (send_tx_a, send_rx_a) = flume::bounded(2);
    let (command_tx_a, _command_rx_a) = flume::bounded(1);
    let (send_tx_b, send_rx_b) = flume::bounded(2);
    let (command_tx_b, _command_rx_b) = flume::bounded(1);
    insert_player_registration_fixture_like_cpp(&registry, 7001, send_tx_a, command_tx_a);
    insert_player_registration_fixture_like_cpp(&registry, 7002, send_tx_b, command_tx_b);

    let summary = game_event_update_world_states_like_cpp(
        &metadata,
        Some(&store),
        Some(&mut world_state_mgr),
        Some(&registry),
        1,
        true,
    );

    let expected = wow_packet::packets::misc::UpdateWorldState {
        variable_id: 777,
        value: 1,
        hidden: false,
    }
    .to_bytes();
    assert_eq!(summary.update_world_states_realm_changed_or_inserted, 1);
    assert_eq!(summary.update_world_states_global_message_represented, 1);
    assert_eq!(summary.update_world_states_global_message_send_attempted, 2);
    assert_eq!(summary.update_world_states_global_message_send_queued, 2);
    assert_eq!(summary.update_world_states_global_message_send_failed, 0);
    assert_eq!(send_rx_a.try_recv().expect("player A update"), expected);
    assert_eq!(send_rx_b.try_recv().expect("player B update"), expected);
    assert!(send_rx_a.try_recv().is_err());
    assert!(send_rx_b.try_recv().is_err());
}

#[test]
fn game_event_world_state_global_fanout_skips_not_in_world_player_like_cpp() {
    let registry = PlayerRegistry::with_canonical_player_fixtures_like_cpp();
    let (in_world_tx, in_world_rx) = flume::bounded(1);
    let (in_world_command_tx, _in_world_command_rx) = flume::bounded(1);
    let (not_in_world_tx, not_in_world_rx) = flume::bounded(1);
    let (not_in_world_command_tx, _not_in_world_command_rx) = flume::bounded(1);
    insert_player_registration_fixture_with_in_world_like_cpp(
        &registry,
        7901,
        in_world_tx,
        in_world_command_tx,
        true,
    );
    insert_player_registration_fixture_with_in_world_like_cpp(
        &registry,
        7902,
        not_in_world_tx,
        not_in_world_command_tx,
        false,
    );
    let mut summary = GameEventLiveUpdateSideEffectSummaryLikeCpp::default();

    fanout_realm_update_world_state_to_player_sessions_like_cpp(
        Some(&registry),
        782,
        1,
        false,
        &mut summary,
    );

    let expected = wow_packet::packets::misc::UpdateWorldState {
        variable_id: 782,
        value: 1,
        hidden: false,
    }
    .to_bytes();
    assert_eq!(summary.update_world_states_global_message_send_attempted, 1);
    assert_eq!(summary.update_world_states_global_message_send_queued, 1);
    assert_eq!(summary.update_world_states_global_message_send_failed, 0);
    assert_eq!(
        summary.update_world_states_global_message_not_in_world_skipped,
        1
    );
    assert_eq!(
        in_world_rx.try_recv().expect("in-world player update"),
        expected
    );
    assert!(not_in_world_rx.try_recv().is_err());
}

#[test]
fn game_event_world_state_global_fanout_preserves_signed_value_and_wrapped_variable_like_cpp() {
    let registry = PlayerRegistry::with_canonical_player_fixtures_like_cpp();
    let (send_tx, send_rx) = flume::bounded(1);
    let (command_tx, _command_rx) = flume::bounded(1);
    insert_player_registration_fixture_like_cpp(&registry, 7003, send_tx, command_tx);
    let mut summary = GameEventLiveUpdateSideEffectSummaryLikeCpp::default();

    fanout_realm_update_world_state_to_player_sessions_like_cpp(
        Some(&registry),
        -1,
        -42,
        false,
        &mut summary,
    );

    let expected = wow_packet::packets::misc::UpdateWorldState {
        variable_id: u32::MAX,
        value: -42,
        hidden: false,
    }
    .to_bytes();
    assert_eq!(summary.update_world_states_global_message_send_attempted, 1);
    assert_eq!(summary.update_world_states_global_message_send_queued, 1);
    assert_eq!(
        send_rx.try_recv().expect("wrapped world-state update"),
        expected
    );
}

#[test]
fn game_event_world_state_realm_unchanged_does_not_fanout_like_cpp() {
    let metadata = game_event_world_state_metadata_like_cpp(
        1,
        &[spawn_store_loader::GameEventDataLikeCpp {
            event_id: 1,
            holiday_id: wow_data::HOLIDAY_CALL_TO_ARMS_AV_LIKE_CPP,
            length: 1,
            ..spawn_store_loader::GameEventDataLikeCpp::default()
        }],
    );
    let store = wow_data::BattlemasterListStore::from_entries([wow_data::BattlemasterListEntry {
        id: wow_data::BATTLEGROUND_AV_LIKE_CPP,
        instance_type: wow_data::MAP_BATTLEGROUND_LIKE_CPP,
        holiday_world_state: 778,
        flags: 0,
    }]);
    let mut world_state_mgr =
        spawn_store_loader::WorldStateMgrLikeCpp::from_templates_and_saved_values(
            [spawn_store_loader::WorldStateTemplateLikeCpp::realm_wide(
                778, 1,
            )],
            [],
        );
    let registry = PlayerRegistry::with_canonical_player_fixtures_like_cpp();
    let (send_tx, send_rx) = flume::bounded(1);
    let (command_tx, _command_rx) = flume::bounded(1);
    insert_player_registration_fixture_like_cpp(&registry, 7004, send_tx, command_tx);

    let summary = game_event_update_world_states_like_cpp(
        &metadata,
        Some(&store),
        Some(&mut world_state_mgr),
        Some(&registry),
        1,
        true,
    );

    assert_eq!(summary.update_world_states_realm_unchanged_noop, 1);
    assert_eq!(summary.update_world_states_global_message_send_attempted, 0);
    assert!(send_rx.try_recv().is_err());
}

#[test]
fn game_event_world_state_realm_change_without_player_registry_is_counted_like_cpp() {
    let metadata = game_event_world_state_metadata_like_cpp(
        1,
        &[spawn_store_loader::GameEventDataLikeCpp {
            event_id: 1,
            holiday_id: wow_data::HOLIDAY_CALL_TO_ARMS_AV_LIKE_CPP,
            length: 1,
            ..spawn_store_loader::GameEventDataLikeCpp::default()
        }],
    );
    let store = wow_data::BattlemasterListStore::from_entries([wow_data::BattlemasterListEntry {
        id: wow_data::BATTLEGROUND_AV_LIKE_CPP,
        instance_type: wow_data::MAP_BATTLEGROUND_LIKE_CPP,
        holiday_world_state: 779,
        flags: 0,
    }]);
    let mut world_state_mgr = spawn_store_loader::WorldStateMgrLikeCpp::default();

    let summary = game_event_update_world_states_like_cpp(
        &metadata,
        Some(&store),
        Some(&mut world_state_mgr),
        None,
        1,
        true,
    );

    assert_eq!(summary.update_world_states_realm_changed_or_inserted, 1);
    assert_eq!(summary.update_world_states_global_message_represented, 1);
    assert_eq!(
        summary.update_world_states_global_message_registry_missing,
        1
    );
    assert_eq!(summary.update_world_states_global_message_send_attempted, 0);
}

#[test]
fn game_event_world_state_map_specific_null_map_does_not_fanout_like_cpp() {
    let metadata = game_event_world_state_metadata_like_cpp(
        1,
        &[spawn_store_loader::GameEventDataLikeCpp {
            event_id: 1,
            holiday_id: wow_data::HOLIDAY_CALL_TO_ARMS_AV_LIKE_CPP,
            length: 1,
            ..spawn_store_loader::GameEventDataLikeCpp::default()
        }],
    );
    let store = wow_data::BattlemasterListStore::from_entries([wow_data::BattlemasterListEntry {
        id: wow_data::BATTLEGROUND_AV_LIKE_CPP,
        instance_type: wow_data::MAP_BATTLEGROUND_LIKE_CPP,
        holiday_world_state: 780,
        flags: 0,
    }]);
    let mut world_state_mgr =
        spawn_store_loader::WorldStateMgrLikeCpp::from_templates_and_saved_values(
            [spawn_store_loader::WorldStateTemplateLikeCpp::map_specific(
                780,
                0,
                [1],
            )],
            [],
        );
    let registry = PlayerRegistry::with_canonical_player_fixtures_like_cpp();
    let (send_tx, send_rx) = flume::bounded(1);
    let (command_tx, _command_rx) = flume::bounded(1);
    insert_player_registration_fixture_like_cpp(&registry, 7005, send_tx, command_tx);

    let summary = game_event_update_world_states_like_cpp(
        &metadata,
        Some(&store),
        Some(&mut world_state_mgr),
        Some(&registry),
        1,
        true,
    );

    assert_eq!(
        summary.update_world_states_map_specific_no_map_unsupported,
        1
    );
    assert_eq!(summary.update_world_states_global_message_send_attempted, 0);
    assert!(send_rx.try_recv().is_err());
}

#[test]
fn game_event_world_state_global_fanout_counts_full_channel_failure_like_cpp() {
    let metadata = game_event_world_state_metadata_like_cpp(
        1,
        &[spawn_store_loader::GameEventDataLikeCpp {
            event_id: 1,
            holiday_id: wow_data::HOLIDAY_CALL_TO_ARMS_AV_LIKE_CPP,
            length: 1,
            ..spawn_store_loader::GameEventDataLikeCpp::default()
        }],
    );
    let store = wow_data::BattlemasterListStore::from_entries([wow_data::BattlemasterListEntry {
        id: wow_data::BATTLEGROUND_AV_LIKE_CPP,
        instance_type: wow_data::MAP_BATTLEGROUND_LIKE_CPP,
        holiday_world_state: 781,
        flags: 0,
    }]);
    let mut world_state_mgr = spawn_store_loader::WorldStateMgrLikeCpp::default();
    let registry = PlayerRegistry::with_canonical_player_fixtures_like_cpp();
    let (queued_tx, queued_rx) = flume::bounded(1);
    let (queued_command_tx, _queued_command_rx) = flume::bounded(1);
    let (full_tx, _full_rx) = flume::bounded(0);
    let (full_command_tx, _full_command_rx) = flume::bounded(1);
    insert_player_registration_fixture_like_cpp(&registry, 7006, queued_tx, queued_command_tx);
    insert_player_registration_fixture_like_cpp(&registry, 7007, full_tx, full_command_tx);

    let summary = game_event_update_world_states_like_cpp(
        &metadata,
        Some(&store),
        Some(&mut world_state_mgr),
        Some(&registry),
        1,
        true,
    );

    assert_eq!(summary.update_world_states_global_message_send_attempted, 2);
    assert_eq!(summary.update_world_states_global_message_send_queued, 1);
    assert_eq!(summary.update_world_states_global_message_send_failed, 1);
    assert!(queued_rx.try_recv().is_ok());
}

#[test]
fn game_event_announce_start_order_before_spawn_and_stop_has_no_announce_like_cpp() {
    let metadata = game_event_world_state_metadata_like_cpp(
        3,
        &[spawn_store_loader::GameEventDataLikeCpp {
            event_id: 2,
            description: "Darkmoon Faire".to_string(),
            announce: 1,
            ..spawn_store_loader::GameEventDataLikeCpp::default()
        }],
    );
    let mut outcome = game_event_world_state_start_outcome_like_cpp(2);
    outcome.stop_outcomes = vec![spawn_store_loader::GameEventStopOutcomeLikeCpp::Stopped(
        spawn_store_loader::GameEventStopSummaryLikeCpp {
            event_id: 3,
            state_before_raw: 0,
            state_after_raw: 0,
            active_removed: true,
            active_was_present: true,
            unapply_event_requested: true,
            serverwide: false,
            condition_reset_requested: false,
            delete_world_event_state_requested: false,
            delete_condition_saves_requested: false,
        },
    )];

    let actions = game_event_live_update_actions_like_cpp(&metadata, &outcome, false);

    assert_eq!(
        actions.first(),
        Some(&GameEventLiveUpdateActionLikeCpp::AnnounceEvent {
            event_id: 2,
            description: "Darkmoon Faire".to_string(),
            description_len: "Darkmoon Faire".len(),
            announce: 1,
            config_event_announce: false,
        })
    );
    assert_eq!(
        actions.get(1),
        Some(&GameEventLiveUpdateActionLikeCpp::Spawn(2))
    );
    assert_eq!(
        actions
            .iter()
            .filter(|action| matches!(
                action,
                GameEventLiveUpdateActionLikeCpp::AnnounceEvent { .. }
            ))
            .count(),
        1
    );
    assert!(matches!(
        actions.iter().rev().take(8).last(),
        Some(GameEventLiveUpdateActionLikeCpp::RunSmartAIScripts {
            event_id: 3,
            activate: false
        })
    ));
}

#[test]
fn game_event_announce_gating_matches_cpp_config_like_cpp() {
    let mut event = spawn_store_loader::GameEventDataLikeCpp {
        event_id: 1,
        description: "config gated".to_string(),
        ..spawn_store_loader::GameEventDataLikeCpp::default()
    };
    let outcome = game_event_world_state_start_outcome_like_cpp(1);

    event.announce = 1;
    let metadata = game_event_world_state_metadata_like_cpp(1, &[event.clone()]);
    assert!(matches!(
        game_event_live_update_actions_like_cpp(&metadata, &outcome, false).first(),
        Some(GameEventLiveUpdateActionLikeCpp::AnnounceEvent { announce: 1, .. })
    ));

    event.announce = 2;
    let metadata = game_event_world_state_metadata_like_cpp(1, &[event.clone()]);
    assert!(
        !game_event_live_update_actions_like_cpp(&metadata, &outcome, false)
            .iter()
            .any(|action| matches!(
                action,
                GameEventLiveUpdateActionLikeCpp::AnnounceEvent { .. }
            ))
    );
    assert!(matches!(
        game_event_live_update_actions_like_cpp(&metadata, &outcome, true).first(),
        Some(GameEventLiveUpdateActionLikeCpp::AnnounceEvent {
            announce: 2,
            config_event_announce: true,
            ..
        })
    ));

    for announce in [0_u8, 3_u8] {
        event.announce = announce;
        let metadata = game_event_world_state_metadata_like_cpp(1, &[event.clone()]);
        assert!(
            !game_event_live_update_actions_like_cpp(&metadata, &outcome, true)
                .iter()
                .any(|action| matches!(
                    action,
                    GameEventLiveUpdateActionLikeCpp::AnnounceEvent { .. }
                ))
        );
    }
}

#[test]
fn game_event_announce_consumption_fans_out_system_chat_like_cpp() {
    let mut manager = wow_map::MapManager::default();
    let mut metadata = game_event_world_state_metadata_like_cpp(
        1,
        &[spawn_store_loader::GameEventDataLikeCpp {
            event_id: 1,
            description: "Darkmoon Faire".to_string(),
            announce: 1,
            ..spawn_store_loader::GameEventDataLikeCpp::default()
        }],
    );
    let outcome = game_event_world_state_start_outcome_like_cpp(1);
    let registry = PlayerRegistry::with_canonical_player_fixtures_like_cpp();
    let (send_tx_a, send_rx_a) = flume::bounded(2);
    let (command_tx_a, _command_rx_a) = flume::bounded(1);
    let (send_tx_b, send_rx_b) = flume::bounded(2);
    let (command_tx_b, _command_rx_b) = flume::bounded(1);
    insert_player_registration_fixture_like_cpp(&registry, 7101, send_tx_a, command_tx_a);
    insert_player_registration_fixture_like_cpp(&registry, 7102, send_tx_b, command_tx_b);

    let summary = consume_game_event_live_update_side_effects_like_cpp(
        &mut manager,
        None,
        &mut metadata,
        &empty_loaded_grid_creature_respawn_caches_like_cpp(),
        None,
        None,
        Some(&registry),
        &[1],
        &outcome,
        false,
    );

    let expected_packet = ChatPkt {
        msg_type: ChatMsg::System,
        language: 0,
        sender_guid: ObjectGuid::EMPTY,
        sender_name: String::new(),
        target_guid: ObjectGuid::EMPTY,
        target_name: String::new(),
        prefix: String::new(),
        channel: String::new(),
        text: "|cffff0000[Event Message]: Darkmoon Faire|r".to_string(),
        virtual_realm: 0,
    };
    let mut expected_payload = wow_packet::world_packet::WorldPacket::new_empty();
    expected_packet.write(&mut expected_payload);
    assert_eq!(
        expected_payload.data()[0],
        0x00,
        "CHAT_MSG_SYSTEM must be 0x00 on wire"
    );
    assert_eq!(&expected_payload.data()[1..5], &[0x00, 0x00, 0x00, 0x00]);
    let expected = expected_packet.to_bytes();

    assert_eq!(summary.announce_event_actions, 1);
    assert_eq!(
        summary.announce_event_description_len_total,
        "Darkmoon Faire".len()
    );
    assert_eq!(summary.announce_event_world_text_represented, 1);
    assert_eq!(summary.announce_event_localization_unrepresented, 1);
    assert_eq!(summary.announce_event_in_world_filter_unrepresented, 0);
    assert_eq!(summary.announce_event_not_in_world_skipped, 0);
    assert_eq!(summary.announce_event_lines, 1);
    assert_eq!(summary.announce_event_send_attempted, 2);
    assert_eq!(summary.announce_event_send_queued, 2);
    assert_eq!(summary.announce_event_send_failed, 0);
    assert_eq!(summary.announce_event_world_text_unimplemented, 0);
    assert_eq!(summary.announce_event_session_fanout_unimplemented, 0);
    let received_a = send_rx_a.try_recv().expect("player A packet");
    let received_b = send_rx_b.try_recv().expect("player B packet");
    let payload_offset = 2; // ServerPacket::to_bytes prepends the u16 opcode.
    assert_eq!(
        received_a[payload_offset], 0x00,
        "received CHAT_MSG_SYSTEM must be 0x00 on wire"
    );
    assert_eq!(
        &received_a[payload_offset + 1..payload_offset + 5],
        &[0x00, 0x00, 0x00, 0x00]
    );
    assert_eq!(
        received_b[payload_offset], 0x00,
        "received CHAT_MSG_SYSTEM must be 0x00 on wire"
    );
    assert_eq!(
        &received_b[payload_offset + 1..payload_offset + 5],
        &[0x00, 0x00, 0x00, 0x00]
    );
    assert_eq!(received_a, expected);
    assert_eq!(received_b, expected);
    assert!(send_rx_a.try_recv().is_err());
    assert!(send_rx_b.try_recv().is_err());
    assert_eq!(summary.spawn_actions, 1);
}

#[test]
fn game_event_announce_fanout_skips_not_in_world_player_like_cpp() {
    let registry = PlayerRegistry::with_canonical_player_fixtures_like_cpp();
    let (in_world_tx, in_world_rx) = flume::bounded(1);
    let (in_world_command_tx, _in_world_command_rx) = flume::bounded(1);
    let (not_in_world_tx, not_in_world_rx) = flume::bounded(1);
    let (not_in_world_command_tx, _not_in_world_command_rx) = flume::bounded(1);
    insert_player_registration_fixture_with_in_world_like_cpp(
        &registry,
        7903,
        in_world_tx,
        in_world_command_tx,
        true,
    );
    insert_player_registration_fixture_with_in_world_like_cpp(
        &registry,
        7904,
        not_in_world_tx,
        not_in_world_command_tx,
        false,
    );
    let mut summary = GameEventLiveUpdateSideEffectSummaryLikeCpp::default();

    fanout_game_event_announcement_to_player_sessions_like_cpp(
        Some(&registry),
        "Darkmoon Faire",
        &mut summary,
    );

    let expected = ChatPkt {
        msg_type: ChatMsg::System,
        language: 0,
        sender_guid: ObjectGuid::EMPTY,
        sender_name: String::new(),
        target_guid: ObjectGuid::EMPTY,
        target_name: String::new(),
        prefix: String::new(),
        channel: String::new(),
        text: "|cffff0000[Event Message]: Darkmoon Faire|r".to_string(),
        virtual_realm: 0,
    }
    .to_bytes();
    assert_eq!(summary.announce_event_world_text_represented, 1);
    assert_eq!(summary.announce_event_localization_unrepresented, 1);
    assert_eq!(summary.announce_event_in_world_filter_unrepresented, 0);
    assert_eq!(summary.announce_event_not_in_world_skipped, 1);
    assert_eq!(summary.announce_event_lines, 1);
    assert_eq!(summary.announce_event_send_attempted, 1);
    assert_eq!(summary.announce_event_send_queued, 1);
    assert_eq!(summary.announce_event_send_failed, 0);
    assert_eq!(
        in_world_rx.try_recv().expect("in-world player chat"),
        expected
    );
    assert!(not_in_world_rx.try_recv().is_err());
}

#[test]
fn game_event_announce_missing_registry_counts_gap_without_panic_like_cpp() {
    let mut summary = GameEventLiveUpdateSideEffectSummaryLikeCpp::default();

    fanout_game_event_announcement_to_player_sessions_like_cpp(
        None,
        "Love is in the Air",
        &mut summary,
    );

    assert_eq!(summary.announce_event_world_text_represented, 1);
    assert_eq!(summary.announce_event_localization_unrepresented, 1);
    assert_eq!(summary.announce_event_registry_missing, 1);
    assert_eq!(summary.announce_event_lines, 1);
    assert_eq!(summary.announce_event_send_attempted, 0);
    assert_eq!(summary.announce_event_send_queued, 0);
    assert_eq!(summary.announce_event_send_failed, 0);
}

#[test]
fn game_event_announce_newline_split_after_fallback_format_like_cpp() {
    assert_eq!(
        game_event_announcement_lines_like_cpp(""),
        vec!["|cffff0000[Event Message]: |r".to_string()]
    );
    assert_eq!(
        game_event_announcement_lines_like_cpp("\n\n"),
        vec!["|cffff0000[Event Message]: ".to_string(), "|r".to_string(),]
    );
    assert_eq!(
        game_event_announcement_lines_like_cpp("A\n\nB"),
        vec![
            "|cffff0000[Event Message]: A".to_string(),
            "B|r".to_string(),
        ]
    );
}

#[test]
fn game_event_smart_ai_game_event_seasonal_start_stop_order_matches_cpp_live_update_like_cpp() {
    let metadata = game_event_world_state_metadata_like_cpp(
        3,
        &[spawn_store_loader::GameEventDataLikeCpp {
            event_id: 2,
            start: 100,
            occurence: 10,
            state_raw: spawn_store_loader::GameEventStateLikeCpp::Normal as u8,
            ..spawn_store_loader::GameEventDataLikeCpp::default()
        }],
    );
    let outcome = spawn_store_loader::GameEventUpdateOutcomeLikeCpp {
        current_time_secs: 1_350,
        scanned_event_ids: vec![],
        check_outcomes: vec![],
        next_check_outcomes: vec![],
        queued_activation_event_ids: vec![2],
        queued_deactivation_event_ids: vec![3],
        start_outcomes: vec![spawn_store_loader::GameEventStartOutcomeLikeCpp::Started(
            spawn_store_loader::GameEventStartSummaryLikeCpp {
                event_id: 2,
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
        stop_outcomes: vec![spawn_store_loader::GameEventStopOutcomeLikeCpp::Stopped(
            spawn_store_loader::GameEventStopSummaryLikeCpp {
                event_id: 3,
                state_before_raw: 0,
                state_after_raw: 0,
                active_removed: true,
                active_was_present: true,
                unapply_event_requested: true,
                serverwide: true,
                condition_reset_requested: false,
                delete_world_event_state_requested: false,
                delete_condition_saves_requested: false,
            },
        )],
        negative_spawn_event_ids: vec![-1],
        world_nextphase_finished: vec![],
        world_conditions_save_requested: vec![],
        invalid_check_outcomes: vec![],
        invalid_next_check_outcomes: vec![],
        next_event_delay_secs_before_padding: 0,
        next_update_delay_millis: 1_000,
    };

    assert_eq!(
        game_event_live_update_actions_like_cpp(&metadata, &outcome, false),
        vec![
            GameEventLiveUpdateActionLikeCpp::Spawn(-1),
            GameEventLiveUpdateActionLikeCpp::Spawn(2),
            GameEventLiveUpdateActionLikeCpp::Unspawn(-2),
            GameEventLiveUpdateActionLikeCpp::ChangeEquipOrModel {
                event_id: 2,
                activate: true,
            },
            GameEventLiveUpdateActionLikeCpp::UpdateEventQuests {
                event_id: 2,
                activate: true,
            },
            GameEventLiveUpdateActionLikeCpp::UpdateWorldStates {
                event_id: 2,
                activate: true,
            },
            GameEventLiveUpdateActionLikeCpp::UpdateNpcFlags { event_id: 2 },
            GameEventLiveUpdateActionLikeCpp::UpdateNpcVendor {
                event_id: 2,
                activate: true,
            },
            GameEventLiveUpdateActionLikeCpp::RunSmartAIScripts {
                event_id: 2,
                activate: true,
            },
            GameEventLiveUpdateActionLikeCpp::ResetEventSeasonalQuests {
                event_id: 2,
                event_start_time: 1_300,
            },
            GameEventLiveUpdateActionLikeCpp::RunSmartAIScripts {
                event_id: 3,
                activate: false,
            },
            GameEventLiveUpdateActionLikeCpp::Unspawn(3),
            GameEventLiveUpdateActionLikeCpp::Spawn(-3),
            GameEventLiveUpdateActionLikeCpp::ChangeEquipOrModel {
                event_id: 3,
                activate: false,
            },
            GameEventLiveUpdateActionLikeCpp::UpdateEventQuests {
                event_id: 3,
                activate: false,
            },
            GameEventLiveUpdateActionLikeCpp::UpdateWorldStates {
                event_id: 3,
                activate: false,
            },
            GameEventLiveUpdateActionLikeCpp::UpdateNpcFlags { event_id: 3 },
            GameEventLiveUpdateActionLikeCpp::UpdateNpcVendor {
                event_id: 3,
                activate: false,
            },
        ]
    );
}

#[test]
fn game_event_smart_ai_consume_no_maps_missing_event_noops_and_counts_action_like_cpp() {
    let mut manager = wow_map::MapManager::default();
    let mut metadata = game_event_world_state_metadata_like_cpp(0, &[]);
    let outcome = spawn_store_loader::GameEventUpdateOutcomeLikeCpp {
        current_time_secs: 650,
        scanned_event_ids: vec![],
        check_outcomes: vec![],
        next_check_outcomes: vec![],
        queued_activation_event_ids: vec![7],
        queued_deactivation_event_ids: vec![],
        start_outcomes: vec![spawn_store_loader::GameEventStartOutcomeLikeCpp::Started(
            spawn_store_loader::GameEventStartSummaryLikeCpp {
                event_id: 7,
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
    };

    let summary = consume_game_event_live_update_side_effects_like_cpp(
        &mut manager,
        None,
        &mut metadata,
        &empty_loaded_grid_creature_respawn_caches_like_cpp(),
        None,
        None,
        None,
        &[7],
        &outcome,
        false,
    );

    assert_eq!(summary.run_smart_ai_actions, 1);
    assert_eq!(summary.run_smart_ai_maps_visited, 0);
    assert_eq!(summary.run_smart_ai_creature_candidates, 0);
    assert_eq!(summary.run_smart_ai_gameobject_candidates, 0);
    assert_eq!(summary.run_smart_ai_script_dispatch_unrepresented, 0);
}

#[test]
fn game_event_seasonal_consume_records_evidence_without_player_or_db_mutation_like_cpp() {
    let mut manager = wow_map::MapManager::default();
    let mut metadata = game_event_world_state_metadata_like_cpp(
        7,
        &[spawn_store_loader::GameEventDataLikeCpp {
            event_id: 7,
            start: 100,
            occurence: 10,
            state_raw: spawn_store_loader::GameEventStateLikeCpp::Normal as u8,
            ..spawn_store_loader::GameEventDataLikeCpp::default()
        }],
    );
    let outcome = game_event_world_state_start_outcome_like_cpp(7);

    let mut summary = consume_game_event_live_update_side_effects_like_cpp(
        &mut manager,
        None,
        &mut metadata,
        &empty_loaded_grid_creature_respawn_caches_like_cpp(),
        None,
        None,
        None,
        &[7],
        &outcome,
        false,
    );

    assert_eq!(summary.reset_event_seasonal_quests_actions, 1);
    assert_eq!(summary.reset_event_seasonal_quests_event_start_time_zero, 0);
    assert_eq!(
        summary.reset_event_seasonal_quests_event_start_time_nonzero,
        1
    );
    assert_eq!(
        summary.reset_event_seasonal_quests_player_session_runtime_unimplemented,
        0
    );
    assert_eq!(
        summary.reset_event_seasonal_quests_player_session_registry_missing,
        0
    );
    assert_eq!(
        summary.reset_event_seasonal_quests_character_db_statement_unimplemented,
        0
    );
    assert_eq!(
        summary.reset_event_seasonal_quests_character_db_delete_queued,
        1
    );
    assert_eq!(
        summary
            .reset_event_seasonal_quests_character_db_delete_skipped_event_start_time_out_of_range,
        0
    );
    fanout_reset_event_seasonal_quests_to_player_sessions_after_db_delete_like_cpp(
        None,
        &mut summary,
    );
    assert_eq!(
        summary.reset_event_seasonal_quests_player_session_registry_missing,
        1
    );
    let [db_delete] = summary.reset_event_seasonal_quest_db_deletes.as_slice() else {
        panic!("expected exactly one seasonal quest DB delete")
    };
    assert_eq!(
        db_delete.mutation,
        wow_persistence::GameEventPersistenceMutationLikeCpp::ResetSeasonalQuests {
            event_id: 7,
            event_start_time: 100,
        }
    );
}

#[test]
fn game_event_seasonal_db_delete_preserves_zero_event_start_time_like_cpp() {
    let mut manager = wow_map::MapManager::default();
    let mut metadata = game_event_world_state_metadata_like_cpp(
        8,
        &[spawn_store_loader::GameEventDataLikeCpp {
            event_id: 8,
            start: 100,
            occurence: 0,
            state_raw: spawn_store_loader::GameEventStateLikeCpp::Normal as u8,
            ..spawn_store_loader::GameEventDataLikeCpp::default()
        }],
    );
    let outcome = game_event_world_state_start_outcome_like_cpp(8);

    let mut summary = consume_game_event_live_update_side_effects_like_cpp(
        &mut manager,
        None,
        &mut metadata,
        &empty_loaded_grid_creature_respawn_caches_like_cpp(),
        None,
        None,
        None,
        &[8],
        &outcome,
        false,
    );

    assert_eq!(summary.reset_event_seasonal_quests_actions, 1);
    assert_eq!(summary.reset_event_seasonal_quests_event_start_time_zero, 1);
    assert_eq!(
        summary.reset_event_seasonal_quests_event_start_time_nonzero,
        0
    );
    assert_eq!(
        summary.reset_event_seasonal_quests_player_session_runtime_unimplemented,
        0
    );
    assert_eq!(
        summary.reset_event_seasonal_quests_player_session_registry_missing,
        0
    );
    assert_eq!(
        summary.reset_event_seasonal_quests_character_db_statement_unimplemented,
        0
    );
    assert_eq!(
        summary.reset_event_seasonal_quests_character_db_delete_queued,
        1
    );
    fanout_reset_event_seasonal_quests_to_player_sessions_after_db_delete_like_cpp(
        None,
        &mut summary,
    );
    assert_eq!(
        summary.reset_event_seasonal_quests_player_session_registry_missing,
        1
    );
    let [db_delete] = summary.reset_event_seasonal_quest_db_deletes.as_slice() else {
        panic!("expected exactly one seasonal quest DB delete")
    };
    assert_eq!(
        db_delete.mutation,
        wow_persistence::GameEventPersistenceMutationLikeCpp::ResetSeasonalQuests {
            event_id: 8,
            event_start_time: 0,
        }
    );
}

#[test]
fn game_event_seasonal_post_db_delete_fanout_queues_session_command_like_cpp() {
    let mut manager = wow_map::MapManager::default();
    let mut metadata = game_event_world_state_metadata_like_cpp(
        9,
        &[spawn_store_loader::GameEventDataLikeCpp {
            event_id: 9,
            start: 345,
            occurence: 10,
            state_raw: spawn_store_loader::GameEventStateLikeCpp::Normal as u8,
            ..spawn_store_loader::GameEventDataLikeCpp::default()
        }],
    );
    let outcome = game_event_world_state_start_outcome_like_cpp(9);
    let registry = PlayerRegistry::with_canonical_player_fixtures_like_cpp();
    let (send_tx, _send_rx) = flume::bounded(1);
    let (command_tx, command_rx) = flume::bounded(1);
    let player_guid = ObjectGuid::create_player(1, 9009);
    registry.register_or_replace(
        player_guid,
        PlayerSessionRegistrationLikeCpp {
            identity: PlayerDirectoryIdentityLikeCpp {
                player_name: "SeasonalTester".to_string(),
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
            durable_creature_runtime_commands_like_cpp: Default::default(),
            client_visible_guids_like_cpp: Default::default(),
            advanced_combat_logging_enabled_like_cpp: Default::default(),
            visibility_refresh_pending_like_cpp: Default::default(),
        },
        Default::default(),
    );

    let mut summary = consume_game_event_live_update_side_effects_like_cpp(
        &mut manager,
        None,
        &mut metadata,
        &empty_loaded_grid_creature_respawn_caches_like_cpp(),
        None,
        None,
        None,
        &[9],
        &outcome,
        false,
    );

    assert!(command_rx.try_recv().is_err());
    assert_eq!(
        summary.reset_event_seasonal_quests_character_db_delete_queued,
        1
    );
    fanout_reset_event_seasonal_quests_to_player_sessions_after_db_delete_like_cpp(
        Some(&registry),
        &mut summary,
    );

    assert_eq!(
        summary.reset_event_seasonal_quests_player_session_send_attempted,
        1
    );
    assert_eq!(
        summary.reset_event_seasonal_quests_player_session_send_queued,
        1
    );
    assert_eq!(
        summary.reset_event_seasonal_quests_player_session_send_failed,
        0
    );
    let command = command_rx
        .try_recv()
        .expect("post-delete fanout command queued");
    let SessionCommand::ResetSeasonalQuestStatus(command) = command else {
        panic!("expected ResetSeasonalQuestStatus command")
    };
    assert_eq!(command.event_id, 9);
    assert_eq!(command.event_start_time, 345);
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

#[test]
fn game_event_live_update_npc_vendor_activation_adds_represented_cache_like_cpp() {
    let mut metadata = game_event_live_update_npc_vendor_metadata_like_cpp(
        1,
        &[(1, 100, 9001, 6000, 2), (1, 101, 9001, 6001, 2)],
    );

    let summary = game_event_update_npc_vendor_like_cpp(&mut metadata, 1, true);

    assert_eq!(summary.update_npc_vendor_records_seen, 2);
    assert_eq!(summary.update_npc_vendor_items_added, 2);
    assert_eq!(summary.update_npc_vendor_items_removed, 0);
    assert_eq!(
        metadata
            .game_event_active_npc_vendor_items_like_cpp(9001)
            .iter()
            .map(|record| record.item)
            .collect::<Vec<_>>(),
        vec![6000, 6001]
    );
}

#[test]
fn game_event_live_update_npc_vendor_deactivation_removes_represented_cache_like_cpp() {
    let mut metadata = game_event_live_update_npc_vendor_metadata_like_cpp(
        2,
        &[(1, 100, 9001, 6000, 2), (2, 200, 9001, 6000, 2)],
    );
    game_event_update_npc_vendor_like_cpp(&mut metadata, 1, true);
    game_event_update_npc_vendor_like_cpp(&mut metadata, 2, true);

    let summary = game_event_update_npc_vendor_like_cpp(&mut metadata, 2, false);

    assert_eq!(summary.update_npc_vendor_records_seen, 1);
    assert_eq!(summary.update_npc_vendor_items_removed, 2);
    assert!(
        metadata
            .game_event_active_npc_vendor_items_like_cpp(9001)
            .is_empty()
    );
}

#[test]
fn game_event_live_update_npc_vendor_missing_bucket_counted_like_cpp() {
    let mut metadata =
        game_event_live_update_npc_vendor_metadata_like_cpp(1, &[(1, 100, 9001, 6000, 2)]);

    let summary = game_event_update_npc_vendor_like_cpp(&mut metadata, 2, true);

    assert_eq!(summary.update_npc_vendor_missing_event_buckets, 1);
    assert_eq!(summary.update_npc_vendor_records_seen, 0);
    assert_eq!(summary.update_npc_vendor_actions, 0);
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

#[test]
fn game_event_npc_flag_live_activation_applies_template_base_and_active_overlay_like_cpp() {
    let mut manager = wow_map::MapManager::default();
    manager.create_world_map(1, 0);
    manager.create_world_map(2, 0);
    let spawn_id = 547101;
    insert_live_creature_for_spawn_like_cpp(&mut manager, 1, spawn_id, 547101);
    let mut store = SpawnStore::new();
    add_spawn_data_like_cpp(&mut store, SpawnObjectType::Creature, spawn_id, 1);
    let mut npc_flags =
        spawn_store_loader::GameEventNpcFlagsLikeCpp::from_game_event_max_entry_like_cpp(Some(2));
    assert!(npc_flags.push_record_like_cpp(
        1,
        spawn_store_loader::GameEventNpcFlagRecordLikeCpp {
            spawn_id,
            npcflag: 0x20,
        },
    ));
    assert!(npc_flags.push_record_like_cpp(
        2,
        spawn_store_loader::GameEventNpcFlagRecordLikeCpp {
            spawn_id,
            npcflag: 0x1_0000_0040,
        },
    ));
    let metadata = spawn_store_loader::CanonicalSpawnMetadataLikeCpp::new(store, BTreeMap::new())
        .with_game_event_npc_flags_like_cpp(npc_flags);

    let template_store = game_event_npc_flag_template_store_like_cpp();
    let summary = game_event_update_npc_flags_like_cpp(
        &mut manager,
        &metadata,
        &template_store,
        None,
        1,
        &[1, 2],
    );

    assert_eq!(summary.update_npc_flags_records_seen, 1);
    assert_eq!(summary.update_npc_flags_template_npcflag_missing, 0);
    assert_eq!(summary.update_npc_flags_maps_matched, 1);
    assert_eq!(summary.update_npc_flags_live_creatures_mutated, 1);
    assert_eq!(summary.update_npc_flags_low_applied, 1);
    assert_eq!(summary.update_npc_flags2_applied, 1);
    assert_eq!(live_npc_flags_like_cpp(&manager, 1, spawn_id), 0xE0);
    assert_eq!(live_npc_flags2_like_cpp(&manager, 1, spawn_id), 0x1);
}

#[test]
fn game_event_npc_flag_update_queues_visible_session_update_command_like_cpp() {
    let mut manager = wow_map::MapManager::default();
    manager.create_world_map(1, 0);
    let spawn_id = 547102;
    let creature_guid = test_guid_like_cpp(HighGuid::Creature, 547102, 99);
    insert_live_creature_for_spawn_like_cpp(&mut manager, 1, spawn_id, 547102);
    let mut store = SpawnStore::new();
    add_spawn_data_like_cpp(&mut store, SpawnObjectType::Creature, spawn_id, 1);
    let mut npc_flags =
        spawn_store_loader::GameEventNpcFlagsLikeCpp::from_game_event_max_entry_like_cpp(Some(1));
    assert!(npc_flags.push_record_like_cpp(
        1,
        spawn_store_loader::GameEventNpcFlagRecordLikeCpp {
            spawn_id,
            npcflag: 0x1_0000_0040,
        },
    ));
    let metadata = spawn_store_loader::CanonicalSpawnMetadataLikeCpp::new(store, BTreeMap::new())
        .with_game_event_npc_flags_like_cpp(npc_flags);
    let registry = PlayerRegistry::with_canonical_player_fixtures_like_cpp();
    let (send_tx, send_rx) = flume::bounded(1);
    let (command_tx, command_rx) = flume::bounded(1);
    let player_guid = ObjectGuid::create_player(1, 7201);
    let mut registration = player_registration_fixture_like_cpp(send_tx, command_tx, "Player7201");
    registration.placement.map_id = 1;
    registry.register_or_replace(player_guid, registration, Default::default());

    let template_store = game_event_npc_flag_template_store_like_cpp();
    let summary = game_event_update_npc_flags_like_cpp(
        &mut manager,
        &metadata,
        &template_store,
        Some(&registry),
        1,
        &[1],
    );

    assert_eq!(summary.update_npc_flags_live_creatures_mutated, 1);
    assert_eq!(summary.update_npc_flags_values_updates_built, 1);
    assert_eq!(summary.update_npc_flags_values_update_send_attempted, 1);
    assert_eq!(summary.update_npc_flags_values_update_send_queued, 1);
    assert!(send_rx.try_recv().is_err());
    let command = command_rx.try_recv().expect("visible update command");
    match command {
        SessionCommand::SendVisibleObjectValuesUpdate(command) => {
            assert_eq!(command.object_guid, creature_guid);
            assert_eq!(command.map_id, 1);
            assert!(!command.packet_bytes.is_empty());
        }
        other => panic!("unexpected command: {other:?}"),
    }
}

#[test]
fn game_event_npc_flag_live_deactivation_recomputes_from_remaining_active_events_like_cpp() {
    let mut manager = wow_map::MapManager::default();
    manager.create_world_map(1, 0);
    let spawn_id = 547201;
    insert_live_creature_for_spawn_like_cpp(&mut manager, 1, spawn_id, 547201);
    let mut store = SpawnStore::new();
    add_spawn_data_like_cpp(&mut store, SpawnObjectType::Creature, spawn_id, 1);
    let mut npc_flags =
        spawn_store_loader::GameEventNpcFlagsLikeCpp::from_game_event_max_entry_like_cpp(Some(2));
    assert!(npc_flags.push_record_like_cpp(
        1,
        spawn_store_loader::GameEventNpcFlagRecordLikeCpp {
            spawn_id,
            npcflag: 0x20,
        },
    ));
    assert!(npc_flags.push_record_like_cpp(
        2,
        spawn_store_loader::GameEventNpcFlagRecordLikeCpp {
            spawn_id,
            npcflag: 0x40,
        },
    ));
    let metadata = spawn_store_loader::CanonicalSpawnMetadataLikeCpp::new(store, BTreeMap::new())
        .with_game_event_npc_flags_like_cpp(npc_flags);

    let template_store = game_event_npc_flag_template_store_like_cpp();
    let start_summary = game_event_update_npc_flags_like_cpp(
        &mut manager,
        &metadata,
        &template_store,
        None,
        1,
        &[1, 2],
    );
    assert_eq!(start_summary.update_npc_flags_live_creatures_mutated, 1);
    assert_eq!(start_summary.update_npc_flags_template_npcflag_missing, 0);
    assert_eq!(live_npc_flags_like_cpp(&manager, 1, spawn_id), 0xE0);

    let stop_summary = game_event_update_npc_flags_like_cpp(
        &mut manager,
        &metadata,
        &template_store,
        None,
        1,
        &[2],
    );

    assert_eq!(stop_summary.update_npc_flags_records_seen, 1);
    assert_eq!(stop_summary.update_npc_flags_template_npcflag_missing, 0);
    assert_eq!(stop_summary.update_npc_flags_live_creatures_mutated, 1);
    assert_eq!(live_npc_flags_like_cpp(&manager, 1, spawn_id), 0xC0);
}

#[test]
fn game_event_change_equip_or_model_missing_bucket_counted_once_like_cpp() {
    let mut manager = wow_map::MapManager::default();
    let mut metadata =
        spawn_store_loader::CanonicalSpawnMetadataLikeCpp::new(SpawnStore::new(), BTreeMap::new());

    let summary = game_event_change_equip_or_model_like_cpp(&mut manager, &mut metadata, 7, true);

    assert_eq!(summary.change_equip_or_model_missing_event_buckets, 1);
    assert_eq!(summary.change_equip_or_model_records_seen, 0);
    assert_eq!(summary.change_equip_or_model_records_applied, 0);
}

#[test]
fn spawn_group_condition_update_tick_uses_effective_map_update_diff_only() {
    let metadata = test_spawn_metadata([(51, 571)]);
    let condition_store =
        ConditionEntriesByTypeStore::from_conditions_like_cpp([mapid_condition(51, 530)]);
    let mut manager = wow_map::MapManager::new(60_000, 10);
    let group = metadata
        .spawn_group_templates()
        .get(&51)
        .expect("test group 51")
        .clone();
    manager.create_world_map(571, 0);
    assert!(
        manager
            .find_map(571, 0)
            .unwrap()
            .map()
            .is_spawn_group_active_like_cpp(Some(&group))
    );
    let mut scheduler = CanonicalRespawnConditionSchedulerLikeCpp::new(10);

    let early = canonical_map_update_tick_set_inactive_like_cpp(
        &mut manager,
        None,
        9,
        &mut scheduler,
        &metadata,
        &condition_store,
        &canonical_test_map_store_like_cpp(),
        &empty_loaded_grid_creature_respawn_caches_like_cpp(),
    );
    assert!(early.is_none());
    assert_eq!(scheduler.timer_ms(), 10);
    assert!(
        manager
            .find_map(571, 0)
            .unwrap()
            .map()
            .is_spawn_group_active_like_cpp(Some(&group))
    );

    let summary = canonical_map_update_tick_set_inactive_like_cpp(
        &mut manager,
        None,
        1,
        &mut scheduler,
        &metadata,
        &condition_store,
        &canonical_test_map_store_like_cpp(),
        &empty_loaded_grid_creature_respawn_caches_like_cpp(),
    )
    .expect("map update accumulates 10ms and scheduler fires with effective diff");
    assert_eq!(summary.maps_evaluated, 1);
    assert_eq!(summary.outcomes, 1);
    assert_eq!(summary.applied_set_inactive, 1);
    assert_eq!(summary.planned_spawn, 0);
    assert_eq!(summary.planned_despawn, 0);
    assert_eq!(scheduler.timer_ms(), 10);
    assert!(
        !manager
            .find_map(571, 0)
            .unwrap()
            .map()
            .is_spawn_group_active_like_cpp(Some(&group))
    );
}

#[test]
fn spawn_group_condition_update_tick_applies_set_inactive_only_when_scheduler_fires() {
    let metadata = test_spawn_metadata([(50, 571)]);
    let condition_store =
        ConditionEntriesByTypeStore::from_conditions_like_cpp([mapid_condition(50, 530)]);
    let mut manager = wow_map::MapManager::new(60_000, 1);
    let group = metadata
        .spawn_group_templates()
        .get(&50)
        .expect("test group 50")
        .clone();
    manager.create_world_map(571, 0);
    assert!(
        manager
            .find_map(571, 0)
            .unwrap()
            .map()
            .is_spawn_group_active_like_cpp(Some(&group))
    );
    let mut scheduler = CanonicalRespawnConditionSchedulerLikeCpp::new(100);

    let early = canonical_map_update_tick_set_inactive_like_cpp(
        &mut manager,
        None,
        99,
        &mut scheduler,
        &metadata,
        &condition_store,
        &canonical_test_map_store_like_cpp(),
        &empty_loaded_grid_creature_respawn_caches_like_cpp(),
    );
    assert!(early.is_none());
    assert!(
        manager
            .find_map(571, 0)
            .unwrap()
            .map()
            .is_spawn_group_active_like_cpp(Some(&group))
    );

    let summary = canonical_map_update_tick_set_inactive_like_cpp(
        &mut manager,
        None,
        1,
        &mut scheduler,
        &metadata,
        &condition_store,
        &canonical_test_map_store_like_cpp(),
        &empty_loaded_grid_creature_respawn_caches_like_cpp(),
    )
    .expect("scheduler fires at interval");
    assert_eq!(summary.maps_evaluated, 1);
    assert_eq!(summary.outcomes, 1);
    assert_eq!(summary.applied_set_inactive, 1);
    assert_eq!(summary.planned_spawn, 0);
    assert_eq!(summary.planned_despawn, 0);
    assert_eq!(scheduler.timer_ms(), 100);
    assert!(
        !manager
            .find_map(571, 0)
            .unwrap()
            .map()
            .is_spawn_group_active_like_cpp(Some(&group))
    );
}

#[test]
fn respawn_db_delete_mutation_like_cpp_preserves_char_del_respawn_values_without_truncation() {
    let outcome = queue_respawn_db_delete_like_cpp(
        wow_map::ManagedMapKind::World,
        false,
        571,
        0,
        SpawnObjectType::Creature,
        1,
    );
    let RespawnDbDeleteQueueOutcomeLikeCpp::Queued(delete) = outcome else {
        panic!("world map delete should queue");
    };

    assert_eq!(delete.object_type, SpawnObjectType::Creature);
    assert_eq!(delete.spawn_id, 1);
    assert_eq!(delete.map_id, 571);
    assert_eq!(delete.instance_id, 0);
    assert_del_respawn_params_like_cpp(&delete.mutation, 0, 1, 571, 0);
}

#[test]
fn respawn_db_delete_statement_like_cpp_skips_non_world_and_invalid_map_id() {
    let non_world = queue_respawn_db_delete_like_cpp(
        wow_map::ManagedMapKind::Dungeon {
            has_reset_schedule: false,
        },
        false,
        571,
        1,
        SpawnObjectType::GameObject,
        2,
    );
    assert!(matches!(
        non_world,
        RespawnDbDeleteQueueOutcomeLikeCpp::SkippedNonWorldMap
    ));

    let instanceable = queue_respawn_db_delete_like_cpp(
        wow_map::ManagedMapKind::World,
        true,
        1_151,
        42,
        SpawnObjectType::Creature,
        1,
    );
    assert!(matches!(
        instanceable,
        RespawnDbDeleteQueueOutcomeLikeCpp::SkippedInstanceableMap
    ));

    let invalid_map_id = queue_respawn_db_delete_like_cpp(
        wow_map::ManagedMapKind::World,
        false,
        u32::from(u16::MAX) + 1,
        0,
        SpawnObjectType::Creature,
        1,
    );
    assert!(matches!(
        invalid_map_id,
        RespawnDbDeleteQueueOutcomeLikeCpp::SkippedInvalidMapId
    ));
}

#[test]
fn respawn_db_save_mutation_like_cpp_preserves_char_rep_respawn_values_without_truncation() {
    let info = RespawnInfoLikeCpp {
        object_type: SpawnObjectType::GameObject,
        spawn_id: u64::from(u32::MAX) + 17,
        entry: 9001,
        respawn_time: 1_777_777_777,
        grid_id: 7,
    };
    let outcome = queue_respawn_db_save_like_cpp(
        wow_map::ManagedMapKind::World,
        false,
        571,
        u32::MAX,
        info.clone(),
    );
    let RespawnDbSaveQueueOutcomeLikeCpp::Queued(save) = outcome else {
        panic!("world map save should queue");
    };

    assert_eq!(save.object_type, SpawnObjectType::GameObject);
    assert_eq!(save.spawn_id, info.spawn_id);
    assert_eq!(save.respawn_time, info.respawn_time);
    assert_eq!(save.map_id, 571);
    assert_eq!(save.instance_id, u32::MAX);
    assert_rep_respawn_params_like_cpp(
        &save.mutation,
        1,
        info.spawn_id,
        info.respawn_time,
        571,
        u32::MAX,
    );
}

#[test]
fn respawn_db_save_statement_like_cpp_skips_non_world_and_invalid_map_id() {
    let info = RespawnInfoLikeCpp {
        object_type: SpawnObjectType::Creature,
        spawn_id: 1,
        entry: 42,
        respawn_time: 123,
        grid_id: 7,
    };

    let non_world = queue_respawn_db_save_like_cpp(
        wow_map::ManagedMapKind::Dungeon {
            has_reset_schedule: false,
        },
        false,
        571,
        1,
        info.clone(),
    );
    assert!(matches!(
        non_world,
        RespawnDbSaveQueueOutcomeLikeCpp::SkippedNonWorldMap
    ));

    let instanceable = queue_respawn_db_save_like_cpp(
        wow_map::ManagedMapKind::World,
        true,
        1_151,
        42,
        info.clone(),
    );
    assert!(matches!(
        instanceable,
        RespawnDbSaveQueueOutcomeLikeCpp::SkippedInstanceableMap
    ));

    let invalid_map_id = queue_respawn_db_save_like_cpp(
        wow_map::ManagedMapKind::World,
        false,
        u32::from(u16::MAX) + 1,
        0,
        info,
    );
    assert!(matches!(
        invalid_map_id,
        RespawnDbSaveQueueOutcomeLikeCpp::SkippedInvalidMapId
    ));
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

#[test]
fn respawn_db_mailbox_coalesces_before_writer_poll_like_cpp() {
    let sender = RespawnDbWriterSenderLikeCpp::new_like_cpp();

    for respawn_time in 0_i64..100_000 {
        sender
            .send(respawn_db_save_mutation_fixture_like_cpp(18, respawn_time))
            .expect("recognized respawn statement accepted");
    }

    let state = sender
        .mailbox
        .state
        .lock()
        .expect("respawn DB mailbox lock");
    assert_eq!(state.queue.pending_len(), 1);
    assert_rep_respawn_params_like_cpp(
        &state.queue.pending[&respawn_persistence_key_fixture_like_cpp(18)].mutation,
        0,
        18,
        99_999,
        571,
        0,
    );
}

#[test]
fn respawn_db_mailbox_keeps_latest_rep_del_order_and_rejects_after_close_like_cpp() {
    let sender = RespawnDbWriterSenderLikeCpp::new_like_cpp();
    sender
        .send(respawn_db_save_mutation_fixture_like_cpp(19, 100))
        .expect("initial REP_RESPAWN accepted");
    sender
        .send(respawn_db_delete_mutation_fixture_like_cpp(19))
        .expect("newer DEL_RESPAWN accepted");

    {
        let state = sender
            .mailbox
            .state
            .lock()
            .expect("respawn DB mailbox lock");
        assert_eq!(state.queue.pending_len(), 1);
        assert!(matches!(
            state.queue.pending[&respawn_persistence_key_fixture_like_cpp(19)].mutation,
            RespawnPersistenceMutationLikeCpp::Delete { .. }
        ));
    }

    sender.close_like_cpp();
    assert_eq!(
        sender.send(respawn_db_save_mutation_fixture_like_cpp(19, 200)),
        Err(RespawnDbSubmitErrorLikeCpp::Closed)
    );
    let mut state = sender
        .mailbox
        .state
        .lock()
        .expect("respawn DB mailbox lock");
    assert!(state.closed);
    assert_eq!(
        state
            .queue
            .take_due(Instant::now())
            .expect("close makes retained state immediately due")
            .pending
            .mutation,
        RespawnPersistenceMutationLikeCpp::Delete {
            key: respawn_persistence_key_fixture_like_cpp(19)
        }
    );
}

#[tokio::test]
async fn respawn_db_mailbox_idle_writer_wakeup_is_not_lost_like_cpp() {
    let sender = RespawnDbWriterSenderLikeCpp::new_like_cpp();
    let notified = sender.mailbox.notify.notified();

    sender
        .send(respawn_db_save_mutation_fixture_like_cpp(20, 100))
        .expect("recognized respawn statement accepted");

    tokio::time::timeout(Duration::from_secs(1), notified)
        .await
        .expect("idle writer notification retained across mailbox check race");
}

#[test]
fn respawn_db_retry_backoff_is_exponential_and_capped() {
    let expected_delays = [1, 2, 4, 8, 16, 30, 30];
    for (failed_flushes, expected_secs) in (1_u32..).zip(expected_delays) {
        assert_eq!(
            respawn_db_retry_delay(failed_flushes),
            Duration::from_secs(expected_secs)
        );
    }
    assert_eq!(respawn_db_retry_delay(u32::MAX), Duration::from_secs(30));

    let start = std::time::Instant::now();
    let mut queue = RespawnDbRetryQueueLikeCpp::default();
    queue.enqueue_latest(respawn_db_save_mutation_fixture_like_cpp(11, 100), start);
    let attempted = queue.take_due(start).expect("first attempt due");
    assert_eq!(
        queue.retry_failed(attempted, start),
        (Duration::from_secs(1), 1)
    );
    assert!(queue.take_due(start + Duration::from_millis(999)).is_none());
    assert!(queue.take_due(start + Duration::from_secs(1)).is_some());
}

#[test]
fn respawn_db_retry_queue_does_not_retry_each_map_tick() {
    let start = std::time::Instant::now();
    let mut queue = RespawnDbRetryQueueLikeCpp::default();
    queue.enqueue_latest(respawn_db_save_mutation_fixture_like_cpp(12, 100), start);
    let failed = queue.take_due(start).expect("first attempt due");
    queue.retry_failed(failed, start);

    for tick in 1..100 {
        assert!(
            queue
                .take_due(start + Duration::from_millis(tick * 10))
                .is_none()
        );
    }
    assert!(queue.take_due(start + Duration::from_secs(1)).is_some());
}

#[test]
fn respawn_db_retry_queue_does_not_delay_fresh_unrelated_key() {
    let start = std::time::Instant::now();
    let mut queue = RespawnDbRetryQueueLikeCpp::default();
    queue.enqueue_latest(respawn_db_save_mutation_fixture_like_cpp(13, 100), start);
    let failed = queue.take_due(start).expect("first attempt due");
    queue.retry_failed(failed, start);

    let fresh_at = start + Duration::from_millis(10);
    queue.enqueue_latest(respawn_db_save_mutation_fixture_like_cpp(14, 200), fresh_at);
    let fresh = queue
        .take_due(fresh_at)
        .expect("unrelated fresh key remains immediately eligible");
    assert_eq!(fresh.key.spawn_id, 14);
    assert!(queue.take_due(fresh_at).is_none());
    assert!(queue.take_due(start + Duration::from_secs(1)).is_some());
}

#[test]
fn respawn_db_retry_queue_coalesces_latest_and_makes_new_state_immediate() {
    let start = std::time::Instant::now();
    let mut queue = RespawnDbRetryQueueLikeCpp::default();
    queue.enqueue_latest(respawn_db_save_mutation_fixture_like_cpp(15, 100), start);
    let failed = queue.take_due(start).expect("first attempt due");
    queue.retry_failed(failed, start);

    let replacement_at = start + Duration::from_millis(10);
    queue.enqueue_latest(
        respawn_db_delete_mutation_fixture_like_cpp(15),
        replacement_at,
    );
    assert_eq!(queue.pending_len(), 1);
    let replacement = queue
        .take_due(replacement_at)
        .expect("newer same-key state must not wait behind stale backoff");
    assert!(matches!(
        replacement.pending.mutation,
        RespawnPersistenceMutationLikeCpp::Delete { .. }
    ));

    queue.enqueue_latest(
        respawn_db_save_mutation_fixture_like_cpp(15, 300),
        replacement_at,
    );
    queue.enqueue_latest(
        respawn_db_save_mutation_fixture_like_cpp(16, 400),
        replacement_at,
    );
    assert_eq!(queue.pending_len(), 2);
    assert_rep_respawn_params_like_cpp(
        &queue.pending[&respawn_persistence_key_fixture_like_cpp(15)].mutation,
        0,
        15,
        300,
        571,
        0,
    );
    assert_rep_respawn_params_like_cpp(
        &queue.pending[&respawn_persistence_key_fixture_like_cpp(16)].mutation,
        0,
        16,
        400,
        571,
        0,
    );
}

#[test]
fn respawn_db_retry_queue_shutdown_makes_existing_backoff_immediately_due() {
    let start = std::time::Instant::now();
    let mut queue = RespawnDbRetryQueueLikeCpp::default();
    queue.enqueue_latest(respawn_db_save_mutation_fixture_like_cpp(17, 100), start);
    let failed = queue.take_due(start).expect("first attempt due");
    queue.retry_failed(failed, start);

    let shutdown_at = start + Duration::from_millis(10);
    assert!(queue.take_due(shutdown_at).is_none());
    queue.make_all_due(shutdown_at);
    assert_eq!(
        queue
            .take_due(shutdown_at)
            .expect("shutdown drain must bypass stale retry deadline")
            .key
            .spawn_id,
        17
    );
}

#[tokio::test]
async fn respawn_db_writer_retries_failed_typed_mutation_then_applies_once_like_cpp() {
    let port = FakeRespawnPersistencePortLikeCpp::default();
    port.fail_mutations
        .store(true, std::sync::atomic::Ordering::Release);
    let mailbox = RespawnDbMailboxLikeCpp::default();
    let mutation = respawn_db_save_mutation_fixture_like_cpp(21, 500);
    {
        let mut state = mailbox.state.lock().unwrap();
        state.queue.enqueue_latest(mutation, Instant::now());
    }

    let first = mailbox
        .state
        .lock()
        .unwrap()
        .queue
        .take_due(Instant::now())
        .expect("fresh typed mutation is due");
    execute_respawn_db_attempt_like_cpp(first, &mailbox, &port).await;
    assert_eq!(mailbox.state.lock().unwrap().queue.pending_len(), 1);

    port.fail_mutations
        .store(false, std::sync::atomic::Ordering::Release);
    let second = {
        let mut state = mailbox.state.lock().unwrap();
        state.queue.make_all_due(Instant::now());
        state
            .queue
            .take_due(Instant::now())
            .expect("failed mutation remains retryable")
    };
    execute_respawn_db_attempt_like_cpp(second, &mailbox, &port).await;

    assert_eq!(mailbox.state.lock().unwrap().queue.pending_len(), 0);
    assert_eq!(
        port.mutations.lock().unwrap().as_slice(),
        [mutation, mutation]
    );
}

#[test]
fn spawn_group_condition_update_tick_process_respawns_delete_only_removes_inactive_due_timer() {
    let metadata = test_spawn_metadata_with_flags([(60, 571, SpawnGroupFlags::MANUAL_SPAWN)]);
    let condition_store = ConditionEntriesByTypeStore::default();
    let mut manager = wow_map::MapManager::new(60_000, 1);
    let map = manager.create_world_map(571, 0);
    map.map_mut().add_respawn_info_like_cpp(RespawnInfoLikeCpp {
        object_type: SpawnObjectType::Creature,
        spawn_id: 1,
        entry: 42,
        respawn_time: 0,
        grid_id: 7,
    });
    let mut scheduler = CanonicalRespawnConditionSchedulerLikeCpp::new(1);

    let summary = canonical_map_update_tick_set_inactive_like_cpp(
        &mut manager,
        None,
        1,
        &mut scheduler,
        &metadata,
        &condition_store,
        &canonical_test_map_store_like_cpp(),
        &empty_loaded_grid_creature_respawn_caches_like_cpp(),
    )
    .expect("scheduler fires");

    assert_eq!(summary.maps_evaluated, 1);
    assert_eq!(summary.respawn_deleted_inactive_spawn_group, 1);
    assert_eq!(summary.respawn_blocked_do_respawn_runtime, 0);
    assert_eq!(summary.respawn_db_delete_queued, 1);
    assert_eq!(summary.respawn_db_delete_skipped_non_world_map, 0);
    assert_eq!(summary.respawn_db_delete_skipped_invalid_map_id, 0);
    assert_eq!(summary.respawn_db_deletes.len(), 1);
    let delete = &summary.respawn_db_deletes[0];
    assert_eq!(delete.object_type, SpawnObjectType::Creature);
    assert_eq!(delete.spawn_id, 1);
    assert_eq!(delete.map_id, 571);
    assert_eq!(delete.instance_id, 0);
    assert_del_respawn_params_like_cpp(&delete.mutation, 0, 1, 571, 0);
    assert_eq!(
        manager
            .find_map(571, 0)
            .unwrap()
            .map()
            .get_respawn_time_like_cpp(SpawnObjectType::Creature, 1),
        0
    );
}

#[test]
fn respawn_db_save_tick_queues_linked_future_reschedule_like_cpp() {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("test clock after unix epoch")
        .as_secs() as i64;
    let linked_respawn_time = now + 3_600;
    let expected_respawn_time = linked_respawn_time + 5;
    let mut linked_respawns = LinkedRespawnStoreLikeCpp::new();
    linked_respawns.insert_like_cpp(
        linked_respawn_guid_like_cpp(wow_core::guid::HighGuid::Creature, 42, 1),
        linked_respawn_guid_like_cpp(wow_core::guid::HighGuid::Creature, 42, 2),
    );
    let metadata = test_spawn_metadata_with_flags([
        (62, 571, SpawnGroupFlags::NONE),
        (63, 571, SpawnGroupFlags::NONE),
    ])
    .with_linked_respawns_like_cpp(linked_respawns);
    let condition_store = ConditionEntriesByTypeStore::default();
    let mut manager = wow_map::MapManager::new(60_000, 1);
    let map = manager.create_world_map(571, 0);
    map.map_mut().add_respawn_info_like_cpp(RespawnInfoLikeCpp {
        object_type: SpawnObjectType::Creature,
        spawn_id: 1,
        entry: 42,
        respawn_time: 0,
        grid_id: 7,
    });
    map.map_mut().add_respawn_info_like_cpp(RespawnInfoLikeCpp {
        object_type: SpawnObjectType::Creature,
        spawn_id: 2,
        entry: 42,
        respawn_time: linked_respawn_time,
        grid_id: 8,
    });
    let mut scheduler = CanonicalRespawnConditionSchedulerLikeCpp::new(1);

    let summary = canonical_map_update_tick_set_inactive_like_cpp(
        &mut manager,
        None,
        1,
        &mut scheduler,
        &metadata,
        &condition_store,
        &canonical_test_map_store_like_cpp(),
        &empty_loaded_grid_creature_respawn_caches_like_cpp(),
    )
    .expect("scheduler fires");

    assert_eq!(summary.maps_evaluated, 1);
    assert_eq!(summary.respawn_db_save_queued, 1);
    assert_eq!(summary.respawn_db_save_skipped_non_world_map, 0);
    assert_eq!(summary.respawn_db_save_skipped_invalid_map_id, 0);
    assert_eq!(summary.respawn_db_saves.len(), 1);
    let save = &summary.respawn_db_saves[0];
    assert_eq!(save.object_type, SpawnObjectType::Creature);
    assert_eq!(save.spawn_id, 1);
    assert_eq!(save.respawn_time, expected_respawn_time);
    assert_eq!(save.map_id, 571);
    assert_eq!(save.instance_id, 0);
    assert_rep_respawn_params_like_cpp(&save.mutation, 0, 1, expected_respawn_time, 571, 0);
    let map = manager.find_map(571, 0).expect("world map");
    assert_eq!(
        map.map()
            .get_respawn_time_like_cpp(SpawnObjectType::Creature, 1),
        expected_respawn_time
    );
    assert!(
        map.map()
            .get_respawn_time_like_cpp(SpawnObjectType::Creature, 1)
            > now
    );
}

#[test]
fn canonical_gameobject_timer_replace_queues_respawn_save_before_condition_tick_like_cpp() {
    let metadata = test_spawn_metadata([]);
    let condition_store = ConditionEntriesByTypeStore::default();
    let mut manager = wow_map::MapManager::new(60_000, 1);
    manager.create_world_map(571, 0);
    let spawn_id = 77;
    let guid = test_guid_like_cpp(HighGuid::GameObject, 77, 99);
    insert_live_gameobject_for_spawn_like_cpp(&mut manager, 571, spawn_id, 77);
    {
        let gameobject = manager
            .find_map_mut(571, 0)
            .unwrap()
            .map_mut()
            .get_typed_game_object_mut(guid)
            .expect("test GameObject");
        gameobject.set_represented_gameobject_data_present_like_cpp(true);
        gameobject.set_respawn_compatibility_mode(false);
        gameobject.set_respawn_delay_time(30);
        gameobject.set_spawned_by_default(true);
        gameobject.set_loot_state(wow_entities::LootState::JustDeactivated, None);
    }
    manager
        .find_map_mut(571, 0)
        .unwrap()
        .map_mut()
        .add_respawn_info_like_cpp(RespawnInfoLikeCpp {
            object_type: SpawnObjectType::GameObject,
            spawn_id,
            entry: 99,
            respawn_time: i64::MAX,
            grid_id: 7,
        });
    // The spawn-group/ProcessRespawns timer deliberately does not fire.
    // Persisting GameObject::SaveRespawnTime belongs to Map::Update itself.
    let mut scheduler = CanonicalRespawnConditionSchedulerLikeCpp::new(100);

    let summary = canonical_map_update_tick_set_inactive_like_cpp(
        &mut manager,
        None,
        1,
        &mut scheduler,
        &metadata,
        &condition_store,
        &canonical_test_map_store_like_cpp(),
        &empty_loaded_grid_creature_respawn_caches_like_cpp(),
    )
    .expect("replaced GameObject timer must surface a DB save without waiting 100ms");

    assert_eq!(summary.maps_evaluated, 0);
    assert_eq!(summary.respawn_db_save_queued, 1);
    assert_eq!(summary.respawn_db_saves.len(), 1);
    let save = &summary.respawn_db_saves[0];
    assert_eq!(save.object_type, SpawnObjectType::GameObject);
    assert_eq!(save.spawn_id, spawn_id);
    assert!(save.respawn_time > 0);
    assert_ne!(save.respawn_time, i64::MAX);
    assert_rep_respawn_params_like_cpp(&save.mutation, 1, spawn_id, save.respawn_time, 571, 0);
    assert_eq!(
        manager
            .find_map(571, 0)
            .unwrap()
            .map()
            .get_respawn_time_like_cpp(SpawnObjectType::GameObject, spawn_id),
        save.respawn_time
    );
}

#[test]
fn canonical_gameobject_compatibility_mode_queues_db_only_respawn_save_like_cpp() {
    let metadata = test_spawn_metadata([]);
    let condition_store = ConditionEntriesByTypeStore::default();
    let mut manager = wow_map::MapManager::new(60_000, 1);
    manager.create_world_map(571, 0);
    let spawn_id = 78;
    let guid = test_guid_like_cpp(HighGuid::GameObject, 78, 99);
    insert_live_gameobject_for_spawn_like_cpp(&mut manager, 571, spawn_id, 78);
    {
        let gameobject = manager
            .find_map_mut(571, 0)
            .unwrap()
            .map_mut()
            .get_typed_game_object_mut(guid)
            .expect("test GameObject");
        gameobject.set_represented_gameobject_data_present_like_cpp(true);
        gameobject.set_respawn_compatibility_mode(true);
        gameobject.set_respawn_delay_time(30);
        gameobject.set_spawned_by_default(true);
        gameobject.set_loot_state(wow_entities::LootState::JustDeactivated, None);
    }
    let mut scheduler = CanonicalRespawnConditionSchedulerLikeCpp::new(100);

    let summary = canonical_map_update_tick_set_inactive_like_cpp(
        &mut manager,
        None,
        1,
        &mut scheduler,
        &metadata,
        &condition_store,
        &canonical_test_map_store_like_cpp(),
        &empty_loaded_grid_creature_respawn_caches_like_cpp(),
    )
    .expect("compatibility-mode GameObject must surface its DB-only save");

    assert_eq!(summary.respawn_db_save_queued, 1);
    assert_eq!(summary.respawn_db_saves.len(), 1);
    let save = &summary.respawn_db_saves[0];
    assert_eq!(save.object_type, SpawnObjectType::GameObject);
    assert_eq!(save.spawn_id, spawn_id);
    assert_rep_respawn_params_like_cpp(&save.mutation, 1, spawn_id, save.respawn_time, 571, 0);
    assert_eq!(
        manager
            .find_map(571, 0)
            .unwrap()
            .map()
            .get_respawn_time_like_cpp(SpawnObjectType::GameObject, spawn_id),
        0,
        "C++ compatibility mode writes DB directly without adding a map-owned timer"
    );
}

#[test]
fn canonical_gameobject_compatibility_save_skips_instanceable_map_like_cpp() {
    let metadata = test_spawn_metadata([]);
    let condition_store = ConditionEntriesByTypeStore::default();
    let map_id = 1_151;
    let map_store = wow_data::MapStore::from_entries([wow_data::MapEntry {
        id: map_id,
        instance_type: wow_data::map::MAP_SCENARIO,
        expansion_id: 0,
        parent_map_id: -1,
        cosmetic_parent_map_id: -1,
        flags1: wow_data::map::MAP_FLAG_GARRISON,
        flags2: 0,
    }]);
    let mut manager = wow_map::MapManager::new(60_000, 1);
    manager.create_world_map(map_id, 0);
    let spawn_id = 79;
    let guid = test_guid_like_cpp(HighGuid::GameObject, 79, 99);
    insert_live_gameobject_for_spawn_like_cpp(&mut manager, map_id, spawn_id, 79);
    {
        let gameobject = manager
            .find_map_mut(map_id, 0)
            .unwrap()
            .map_mut()
            .get_typed_game_object_mut(guid)
            .expect("test GameObject");
        gameobject.set_represented_gameobject_data_present_like_cpp(true);
        gameobject.set_respawn_compatibility_mode(true);
        gameobject.set_respawn_delay_time(30);
        gameobject.set_spawned_by_default(true);
        gameobject.set_loot_state(wow_entities::LootState::JustDeactivated, None);
    }
    let mut scheduler = CanonicalRespawnConditionSchedulerLikeCpp::new(1);

    let summary = canonical_map_update_tick_set_inactive_like_cpp(
        &mut manager,
        None,
        1,
        &mut scheduler,
        &metadata,
        &condition_store,
        &map_store,
        &empty_loaded_grid_creature_respawn_caches_like_cpp(),
    )
    .expect("scheduler fires");

    assert_eq!(summary.respawn_db_save_queued, 0);
    assert_eq!(summary.respawn_db_save_skipped_instanceable_map, 1);
    assert!(summary.respawn_db_saves.is_empty());
}

#[test]
fn spawn_group_condition_update_tick_pool_timer_uses_canonical_pool_mgr_and_queues_delete() {
    let mut pool_mgr = PoolMgrLikeCpp::new();
    pool_mgr.insert_template_like_cpp(70, PoolTemplateDataLikeCpp::new(1, 571));
    let mut group = PoolGroupLikeCpp::with_pool_id(PoolMemberKindLikeCpp::Creature, 70);
    group.add_entry_like_cpp(PoolObjectLikeCpp::new(1, 0.0), 1);
    group.add_entry_like_cpp(PoolObjectLikeCpp::new(101, 0.0), 1);
    pool_mgr
        .insert_or_replace_group_like_cpp(PoolMemberKindLikeCpp::Creature, 70, group)
        .expect("test pool group");
    pool_mgr
        .register_spawn_pool_relation_like_cpp(PoolMemberKindLikeCpp::Creature, 1, 70)
        .expect("test spawn pool relation");
    let metadata = test_spawn_metadata_with_flags([(64, 571, SpawnGroupFlags::NONE)])
        .with_pool_mgr_like_cpp(pool_mgr);
    let condition_store = ConditionEntriesByTypeStore::default();
    let mut manager = wow_map::MapManager::new(60_000, 1);
    let map = manager.create_world_map(571, 0);
    map.map_mut().add_respawn_info_like_cpp(RespawnInfoLikeCpp {
        object_type: SpawnObjectType::Creature,
        spawn_id: 1,
        entry: 42,
        respawn_time: 0,
        grid_id: 7,
    });
    let mut scheduler = CanonicalRespawnConditionSchedulerLikeCpp::new(1);

    let summary = canonical_map_update_tick_set_inactive_like_cpp(
        &mut manager,
        None,
        1,
        &mut scheduler,
        &metadata,
        &condition_store,
        &canonical_test_map_store_like_cpp(),
        &empty_loaded_grid_creature_respawn_caches_like_cpp(),
    )
    .expect("scheduler fires");

    assert_eq!(summary.maps_evaluated, 1);
    assert_eq!(summary.respawn_processed_pool_timers, 1);
    assert_eq!(summary.respawn_processed_unloaded_grid_respawns, 0);
    assert_eq!(summary.respawn_pool_update_plans, 1);
    assert_eq!(summary.respawn_blocked_pool_plan_errors, 0);
    assert_eq!(summary.respawn_blocked_pool_runtime, 0);
    assert_eq!(summary.respawn_blocked_do_respawn_runtime, 0);
    assert_eq!(summary.respawn_db_delete_queued, 1);
    assert_eq!(summary.respawn_db_deletes.len(), 1);
    let delete = &summary.respawn_db_deletes[0];
    assert_eq!(delete.object_type, SpawnObjectType::Creature);
    assert_eq!(delete.spawn_id, 1);
    assert_eq!(delete.map_id, 571);
    assert_eq!(delete.instance_id, 0);
    assert_del_respawn_params_like_cpp(&delete.mutation, 0, 1, 571, 0);
    let map = manager.find_map(571, 0).expect("world map");
    assert!(
        map.map()
            .pool_data_like_cpp()
            .is_spawned_creature_like_cpp(101)
    );
    assert_eq!(
        map.map()
            .get_respawn_time_like_cpp(SpawnObjectType::Creature, 1),
        0
    );
    assert!(
        map.map()
            .get_respawn_info_like_cpp(SpawnObjectType::Creature, 1)
            .is_none()
    );
}

#[test]
fn spawn_group_condition_update_tick_process_respawns_unloaded_grid_queues_delete_without_spawn() {
    let metadata = test_spawn_metadata_with_flags([(61, 571, SpawnGroupFlags::NONE)]);
    let condition_store = ConditionEntriesByTypeStore::default();
    let mut manager = wow_map::MapManager::new(60_000, 1);
    let map = manager.create_world_map(571, 0);
    map.map_mut().add_respawn_info_like_cpp(RespawnInfoLikeCpp {
        object_type: SpawnObjectType::Creature,
        spawn_id: 1,
        entry: 42,
        respawn_time: 0,
        grid_id: 7,
    });
    let mut scheduler = CanonicalRespawnConditionSchedulerLikeCpp::new(1);

    let summary = canonical_map_update_tick_set_inactive_like_cpp(
        &mut manager,
        None,
        1,
        &mut scheduler,
        &metadata,
        &condition_store,
        &canonical_test_map_store_like_cpp(),
        &empty_loaded_grid_creature_respawn_caches_like_cpp(),
    )
    .expect("scheduler fires");

    assert_eq!(summary.maps_evaluated, 1);
    assert_eq!(summary.respawn_deleted_inactive_spawn_group, 0);
    assert_eq!(summary.respawn_processed_unloaded_grid_respawns, 1);
    assert_eq!(summary.respawn_blocked_do_respawn_runtime, 0);
    assert_eq!(summary.respawn_db_delete_queued, 1);
    assert_eq!(summary.respawn_db_deletes.len(), 1);
    let delete = &summary.respawn_db_deletes[0];
    assert_eq!(delete.object_type, SpawnObjectType::Creature);
    assert_eq!(delete.spawn_id, 1);
    assert_eq!(delete.map_id, 571);
    assert_eq!(delete.instance_id, 0);
    assert_del_respawn_params_like_cpp(&delete.mutation, 0, 1, 571, 0);
    assert_eq!(
        manager
            .find_map(571, 0)
            .unwrap()
            .map()
            .get_respawn_time_like_cpp(SpawnObjectType::Creature, 1),
        0
    );
    assert!(
        manager
            .find_map(571, 0)
            .unwrap()
            .map()
            .get_respawn_info_like_cpp(SpawnObjectType::Creature, 1)
            .is_none()
    );
}

#[test]
fn persisted_restart_timer_respawns_once_through_canonical_owner_and_mirrors_legacy_like_cpp() {
    let spawn_id = 54_987;
    let entry = 42;
    let mut metadata =
        test_spawn_metadata_with_explicit_spawn_ids([(69, 571, SpawnGroupFlags::NONE, spawn_id)]);
    metadata = metadata.with_creature_runtime_rows_like_cpp(BTreeMap::from([(
        spawn_id,
        super::spawn_store_loader::CreatureSpawnRuntimeRowLikeCpp {
            spawn_id,
            model_id: 999,
            equipment_id: 3,
            wander_distance: 15.0,
            curhealth: 0,
            curmana: 0,
            movement_type: 1,
            npc_flags: None,
            unit_flags: None,
            unit_flags2: None,
            unit_flags3: None,
            ground_movement_type: wow_constants::CreatureGroundMovementType::Run as u8,
            swim_allowed: true,
            flight_movement_type: 0,
            rooted: false,
            chase_movement_type: wow_constants::CreatureChaseMovementType::Run as u8,
            random_movement_type: wow_constants::CreatureRandomMovementType::Walk as u8,
            interaction_pause_timer_ms:
                wow_entities::DEFAULT_CREATURE_INTERACTION_PAUSE_TIMER_MS_LIKE_CPP,
            string_id: "restart-canonical-owner".to_string(),
            spawn_time_secs: 120,
        },
    )]));
    let metadata = Arc::new(Mutex::new(metadata));
    let condition_store = Arc::new(ConditionEntriesByTypeStore::default());
    let map_store = Arc::new(canonical_test_map_store_like_cpp());
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("test clock after unix epoch")
        .as_secs() as i64;
    let mut snapshot = PersistedRespawnTimesLikeCpp::default();
    snapshot.push(
        wow_map::MapKey::new(571, 0),
        RespawnInfoLikeCpp {
            object_type: SpawnObjectType::Creature,
            spawn_id,
            entry,
            respawn_time: now.saturating_sub(1),
            grid_id: wow_map::compute_grid_coord(0.0, 0.0).get_id(),
        },
    );
    let mut manager = wow_map::MapManager::new(60_000, 1);
    install_canonical_spawn_group_initializer_like_cpp(
        &mut manager,
        Arc::clone(&metadata),
        Arc::clone(&condition_store),
        Arc::new(snapshot),
        Arc::clone(&map_store),
    );
    let map = manager.create_world_map(571, 0);
    assert!(map.map_mut().load_grid(0.0, 0.0));
    assert_eq!(map.map().map_object_count(), 0);
    assert_eq!(
        map.map()
            .get_respawn_time_like_cpp(SpawnObjectType::Creature, spawn_id),
        now.saturating_sub(1)
    );

    let legacy: wow_world::SharedMapManager =
        Arc::new(std::sync::RwLock::new(wow_world::MapManager::new()));
    let caches =
        variable_loaded_grid_creature_respawn_caches_with_vehicle_id_and_difficulty_like_cpp(
            entry, 0, 0,
        );
    let mut scheduler = CanonicalRespawnConditionSchedulerLikeCpp::new(1);
    let metadata_guard = metadata.lock().unwrap();
    let summary = canonical_map_update_tick_set_inactive_like_cpp(
        &mut manager,
        Some(&legacy),
        1,
        &mut scheduler,
        &metadata_guard,
        condition_store.as_ref(),
        map_store.as_ref(),
        &caches,
    )
    .expect("due persisted timer must run through canonical ProcessRespawns");

    assert_eq!(summary.respawn_executed_loaded_grid_respawns, 1);
    assert_eq!(summary.respawn_legacy_creature_mirrors, 1);
    assert_eq!(summary.respawn_db_delete_queued, 1);
    assert_eq!(summary.respawn_db_deletes.len(), 1);
    assert_del_respawn_params_like_cpp(
        &summary.respawn_db_deletes[0].mutation,
        0,
        spawn_id,
        571,
        0,
    );
    let creature = manager
        .find_map(571, 0)
        .unwrap()
        .map()
        .get_creature_by_spawn_id_like_cpp(spawn_id)
        .expect("canonical restart respawn");
    let creature_guid = creature.guid();
    assert!(
        legacy
            .read()
            .unwrap()
            .find_creature(571, 0, creature_guid)
            .is_some()
    );
    assert_eq!(
        manager
            .find_map(571, 0)
            .unwrap()
            .map()
            .get_respawn_time_like_cpp(SpawnObjectType::Creature, spawn_id),
        0
    );

    let second = canonical_map_update_tick_set_inactive_like_cpp(
        &mut manager,
        Some(&legacy),
        1,
        &mut scheduler,
        &metadata_guard,
        condition_store.as_ref(),
        map_store.as_ref(),
        &caches,
    )
    .expect("second scheduler tick");
    assert_eq!(second.respawn_executed_loaded_grid_respawns, 0);
    assert_eq!(second.respawn_legacy_creature_mirrors, 0);
    assert_eq!(second.respawn_db_delete_queued, 0);
    assert_eq!(
        manager
            .find_map(571, 0)
            .unwrap()
            .map()
            .creature_spawn_id_store_count_like_cpp(spawn_id),
        1
    );
}

#[test]
fn persisted_gameobject_restart_timer_respawns_once_and_queues_delete_like_cpp() {
    let spawn_id = 54_988;
    let entry = 9_001;
    let spawn = SpawnData {
        object_type: SpawnObjectType::GameObject,
        spawn_id,
        map_id: 571,
        db_data: true,
        spawn_group: SpawnGroupTemplateData::default_group(),
        id: entry,
        spawn_point: SpawnPosition::new(0.0, 0.0, 0.0, 0.0),
        phase_use_flags: 0,
        phase_id: 0,
        phase_group: 0,
        terrain_swap_map: -1,
        pool_id: 0,
        spawn_time_secs: 30,
        spawn_difficulties: vec![0],
        script_id: 0,
        string_id: String::new(),
    };
    let mut store = SpawnStore::new();
    store.add_object_spawn(&spawn, |_| false);
    let metadata = Arc::new(Mutex::new(
        super::spawn_store_loader::CanonicalSpawnMetadataLikeCpp::new(store, BTreeMap::new())
            .with_gameobject_runtime_rows_like_cpp(BTreeMap::from([(
                spawn_id,
                super::spawn_store_loader::GameObjectSpawnRuntimeRowLikeCpp {
                    spawn_id,
                    rotation: [0.0, 0.0, 0.0, 1.0],
                    anim_progress: 55,
                    state: 1,
                    string_id: "restart-gameobject".to_string(),
                    spawn_time_secs: 30,
                },
            )])),
    ));
    let mut data = [0; wow_entities::MAX_GAMEOBJECT_DATA];
    data[11] = 1;
    let mut caches = empty_loaded_grid_creature_respawn_caches_like_cpp();
    caches.gameobject_template_store = Arc::new(
        wow_data::GameObjectTemplateLifecycleStoreLikeCpp::from_templates([
            wow_data::GameObjectTemplateLifecycleRecordLikeCpp {
                entry,
                go_type: wow_entities::GAMEOBJECT_TYPE_GOOBER,
                display_id: 44,
                name: "Restart GameObject".to_string(),
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
    let condition_store = Arc::new(ConditionEntriesByTypeStore::default());
    let map_store = Arc::new(canonical_test_map_store_like_cpp());
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("test clock after unix epoch")
        .as_secs() as i64;
    let mut snapshot = PersistedRespawnTimesLikeCpp::default();
    snapshot.push(
        wow_map::MapKey::new(571, 0),
        RespawnInfoLikeCpp {
            object_type: SpawnObjectType::GameObject,
            spawn_id,
            entry,
            respawn_time: now.saturating_sub(1),
            grid_id: wow_map::compute_grid_coord(0.0, 0.0).get_id(),
        },
    );
    let mut manager = wow_map::MapManager::new(60_000, 1);
    install_canonical_spawn_group_initializer_like_cpp(
        &mut manager,
        Arc::clone(&metadata),
        Arc::clone(&condition_store),
        Arc::new(snapshot),
        Arc::clone(&map_store),
    );
    let map = manager.create_world_map(571, 0);
    assert!(map.map_mut().load_grid(0.0, 0.0));
    assert_eq!(map.map().map_object_count(), 0);

    let mut scheduler = CanonicalRespawnConditionSchedulerLikeCpp::new(1);
    let metadata_guard = metadata.lock().unwrap();
    let summary = canonical_map_update_tick_set_inactive_like_cpp(
        &mut manager,
        None,
        1,
        &mut scheduler,
        &metadata_guard,
        condition_store.as_ref(),
        map_store.as_ref(),
        &caches,
    )
    .expect("due persisted GameObject timer must run through ProcessRespawns");

    assert_eq!(summary.respawn_executed_loaded_grid_respawns, 1);
    assert_eq!(summary.respawn_db_delete_queued, 1);
    assert_del_respawn_params_like_cpp(
        &summary.respawn_db_deletes[0].mutation,
        1,
        spawn_id,
        571,
        0,
    );
    assert_eq!(
        manager
            .find_map(571, 0)
            .unwrap()
            .map()
            .gameobject_spawn_id_store_count_like_cpp(spawn_id),
        1
    );
    assert_eq!(
        manager
            .find_map(571, 0)
            .unwrap()
            .map()
            .get_respawn_time_like_cpp(SpawnObjectType::GameObject, spawn_id),
        0
    );

    let second = canonical_map_update_tick_set_inactive_like_cpp(
        &mut manager,
        None,
        1,
        &mut scheduler,
        &metadata_guard,
        condition_store.as_ref(),
        map_store.as_ref(),
        &caches,
    )
    .expect("second scheduler tick");
    assert_eq!(second.respawn_executed_loaded_grid_respawns, 0);
    assert_eq!(second.respawn_db_delete_queued, 0);
    assert_eq!(
        manager
            .find_map(571, 0)
            .unwrap()
            .map()
            .gameobject_spawn_id_store_count_like_cpp(spawn_id),
        1
    );
}

#[test]
fn loaded_grid_area_trigger_record_returns_area_trigger_record_like_cpp() {
    let spawn_id = 88;
    let create_properties_id = 2001;
    let template_id = 9001;
    let mut store = SpawnStore::new();
    let spawn = SpawnData {
        object_type: SpawnObjectType::AreaTrigger,
        spawn_id,
        map_id: 571,
        db_data: true,
        spawn_group: SpawnGroupTemplateData::default_group(),
        id: create_properties_id,
        spawn_point: SpawnPosition::new(1.0, 2.0, 3.0, 1.0),
        phase_use_flags: 0,
        phase_id: 0,
        phase_group: 0,
        terrain_swap_map: -1,
        pool_id: 0,
        spawn_time_secs: 0,
        spawn_difficulties: vec![0],
        script_id: 0,
        string_id: String::new(),
    };
    store.add_area_trigger_spawn(&spawn);
    let metadata =
        super::spawn_store_loader::CanonicalSpawnMetadataLikeCpp::new(store, BTreeMap::new())
            .with_area_trigger_runtime_rows_like_cpp(BTreeMap::from([(
                spawn_id,
                super::spawn_store_loader::AreaTriggerSpawnRuntimeRowLikeCpp {
                    spawn_id,
                    create_properties_id: wow_data::AreaTriggerIdLikeCpp {
                        id: create_properties_id,
                        is_custom: false,
                    },
                    spell_for_visuals: None,
                },
            )]));
    let template_store =
        area_trigger_template_store_for_loaded_grid_like_cpp(create_properties_id, template_id);
    let mut map = wow_map::Map::new(571, 0, 0, 60_000);

    let record = build_loaded_grid_area_trigger_record_like_cpp(
        &mut map,
        SpawnObjectType::AreaTrigger,
        spawn_id,
        &metadata,
        &template_store,
    )
    .expect("loaded-grid AreaTrigger builder should return loaded-grid records");
    let area_trigger = record
        .primary_record
        .area_trigger()
        .expect("builder should return a typed AreaTrigger MapObjectRecord");

    assert_eq!(
        record.primary_record.kind(),
        wow_entities::AccessorObjectKind::AreaTrigger
    );
    assert_eq!(area_trigger.spawn_id(), spawn_id);
    assert!(area_trigger.is_static_spawn());
    assert_eq!(
        area_trigger.world().guid().high_type(),
        wow_core::guid::HighGuid::AreaTrigger
    );
    assert_eq!(u32::from(area_trigger.world().guid().map_id()), 571);
    assert_eq!(area_trigger.world().guid().entry(), template_id);
    assert_eq!(area_trigger.world().guid().counter(), 1);
    assert_eq!(
        area_trigger.create_properties_id().unwrap().id,
        create_properties_id
    );
    assert_eq!(area_trigger.template_id().unwrap().id, template_id);
    assert_eq!(area_trigger.data().spell_visual_id, 0);
}

#[test]
fn loaded_grid_area_trigger_loader_materializes_loaded_map_grid_like_cpp() {
    let spawn_id = 89;
    let create_properties_id = 2002;
    let template_id = 9002;
    let mut store = SpawnStore::new();
    let spawn = SpawnData {
        object_type: SpawnObjectType::AreaTrigger,
        spawn_id,
        map_id: 571,
        db_data: true,
        spawn_group: SpawnGroupTemplateData::default_group(),
        id: create_properties_id,
        spawn_point: SpawnPosition::new(1.0, 2.0, 3.0, 1.0),
        phase_use_flags: 0,
        phase_id: 0,
        phase_group: 0,
        terrain_swap_map: -1,
        pool_id: 0,
        spawn_time_secs: 0,
        spawn_difficulties: vec![0],
        script_id: 0,
        string_id: String::new(),
    };
    store.add_area_trigger_spawn(&spawn);
    let metadata =
        super::spawn_store_loader::CanonicalSpawnMetadataLikeCpp::new(store, BTreeMap::new())
            .with_area_trigger_runtime_rows_like_cpp(BTreeMap::from([(
                spawn_id,
                super::spawn_store_loader::AreaTriggerSpawnRuntimeRowLikeCpp {
                    spawn_id,
                    create_properties_id: wow_data::AreaTriggerIdLikeCpp {
                        id: create_properties_id,
                        is_custom: false,
                    },
                    spell_for_visuals: None,
                },
            )]));
    let template_store =
        area_trigger_template_store_for_loaded_grid_like_cpp(create_properties_id, template_id);
    let mut manager = wow_map::MapManager::default();
    manager.create_world_map(571, 0);
    manager
        .find_map_mut(571, 0)
        .expect("created map")
        .map_mut()
        .ensure_grid_loaded(&wow_map::map::cell_from_world(1.0, 2.0));

    let summary = load_loaded_grid_area_triggers_like_cpp(&mut manager, &metadata, &template_store);

    assert_eq!(summary.maps_evaluated, 1);
    assert_eq!(summary.loaded_grids_evaluated, 1);
    assert_eq!(summary.grid_not_loaded, 0);
    assert_eq!(summary.metadata_entries, 1);
    assert_eq!(summary.loaded_grid_primary_records, 1);
    assert_eq!(summary.loaded_area_trigger_guids.len(), 1);
    assert_eq!(summary.add_to_map_errors, 0);
    let area_trigger = manager
        .find_map_mut(571, 0)
        .expect("created map")
        .map()
        .get_area_trigger_by_spawn_id_like_cpp(spawn_id)
        .expect("AreaTrigger should be materialized on the loaded grid");
    assert_eq!(
        summary.loaded_area_trigger_guids,
        vec![area_trigger.world().guid()]
    );
    assert_eq!(area_trigger.spawn_id(), spawn_id);
    assert_eq!(area_trigger.template_id().unwrap().id, template_id);
    assert_eq!(
        area_trigger.world().guid().high_type(),
        wow_core::guid::HighGuid::AreaTrigger
    );

    let second = load_loaded_grid_area_triggers_like_cpp(&mut manager, &metadata, &template_store);
    assert_eq!(second.maps_evaluated, 1);
    assert_eq!(second.loaded_grids_evaluated, 1);
    assert_eq!(second.metadata_entries, 0);
    assert_eq!(second.loaded_grid_primary_records, 0);
    assert!(second.loaded_area_trigger_guids.is_empty());
    assert_eq!(second.skipped_already_loaded, 1);
}

#[test]
fn loaded_grid_gameobject_respawn_record_returns_gameobject_record_like_cpp() {
    let spawn_id = 77;
    let entry = 9001;
    let mut store = SpawnStore::new();
    let spawn = SpawnData {
        object_type: SpawnObjectType::GameObject,
        spawn_id,
        map_id: 571,
        db_data: true,
        spawn_group: SpawnGroupTemplateData::default_group(),
        id: entry,
        spawn_point: SpawnPosition::new(1.0, 2.0, 3.0, 1.0),
        phase_use_flags: 0,
        phase_id: 0,
        phase_group: 0,
        terrain_swap_map: -1,
        pool_id: 0,
        spawn_time_secs: 30,
        spawn_difficulties: vec![0],
        script_id: 0,
        string_id: String::new(),
    };
    store.add_object_spawn(&spawn, |_| false);
    let metadata =
        super::spawn_store_loader::CanonicalSpawnMetadataLikeCpp::new(store, BTreeMap::new())
            .with_gameobject_runtime_rows_like_cpp(BTreeMap::from([(
                spawn_id,
                super::spawn_store_loader::GameObjectSpawnRuntimeRowLikeCpp {
                    spawn_id,
                    rotation: [0.0, 0.0, 0.0, 1.0],
                    anim_progress: 55,
                    state: 1,
                    string_id: "live-gameobject".to_string(),
                    spawn_time_secs: 30,
                },
            )]));
    let mut data = [0; wow_entities::MAX_GAMEOBJECT_DATA];
    data[11] = 1;
    let mut caches = empty_loaded_grid_creature_respawn_caches_like_cpp();
    caches.gameobject_template_store = Arc::new(
        wow_data::GameObjectTemplateLifecycleStoreLikeCpp::from_templates([
            wow_data::GameObjectTemplateLifecycleRecordLikeCpp {
                entry,
                go_type: wow_entities::GAMEOBJECT_TYPE_GOOBER,
                display_id: 44,
                name: "Live Loaded GO".to_string(),
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
    let mut map = wow_map::Map::new(571, 0, 0, 60_000);
    map.add_respawn_info_like_cpp(RespawnInfoLikeCpp {
        object_type: SpawnObjectType::GameObject,
        spawn_id,
        entry,
        respawn_time: 1_234,
        grid_id: 7,
    });

    let record = build_loaded_grid_gameobject_respawn_record_like_cpp(
        &mut map,
        SpawnObjectType::GameObject,
        spawn_id,
        &metadata,
        &caches,
    )
    .expect("loaded-grid GameObject builder should return loaded-grid records");
    let game_object = record
        .primary_record
        .game_object()
        .expect("builder should return a typed GameObject MapObjectRecord");

    assert_eq!(
        record.primary_record.kind(),
        wow_entities::AccessorObjectKind::GameObject
    );
    assert_eq!(game_object.spawn_id(), spawn_id);
    assert_eq!(
        game_object.world().guid().high_type(),
        wow_core::guid::HighGuid::GameObject
    );
    assert_eq!(u32::from(game_object.world().guid().map_id()), 571);
    assert_eq!(game_object.world().guid().entry(), entry);
    assert_eq!(game_object.world().guid().counter(), 1);
    assert_eq!(
        game_object.respawn_time(),
        0,
        "ProcessRespawns erases due timer before LoadFromDB, so new GO observes no map respawn time"
    );
}

#[test]
fn loaded_grid_creature_respawn_record_variable_level_returns_creature_record_like_cpp() {
    let spawn_id = 54_984;
    let entry = 42;
    let mut metadata =
        test_spawn_metadata_with_explicit_spawn_ids([(67, 571, SpawnGroupFlags::NONE, spawn_id)]);
    metadata = metadata.with_creature_runtime_rows_like_cpp(BTreeMap::from([(
        spawn_id,
        super::spawn_store_loader::CreatureSpawnRuntimeRowLikeCpp {
            spawn_id,
            model_id: 999,
            equipment_id: 3,
            wander_distance: 15.0,
            curhealth: 0,
            curmana: 0,
            movement_type: 1,
            npc_flags: None,
            unit_flags: None,
            unit_flags2: None,
            unit_flags3: None,
            ground_movement_type: wow_constants::CreatureGroundMovementType::Run as u8,
            swim_allowed: true,
            flight_movement_type: 0,
            rooted: false,
            chase_movement_type: wow_constants::CreatureChaseMovementType::Run as u8,
            random_movement_type: wow_constants::CreatureRandomMovementType::Walk as u8,
            interaction_pause_timer_ms:
                wow_entities::DEFAULT_CREATURE_INTERACTION_PAUSE_TIMER_MS_LIKE_CPP,
            string_id: "variable-level-live".to_string(),
            spawn_time_secs: 120,
        },
    )]));
    let mut caches = variable_loaded_grid_creature_respawn_caches_like_cpp(entry);
    caches.realm_id = 7;
    let mut map = wow_map::Map::new(571, 0, 2, 60_000);
    map.add_respawn_info_like_cpp(RespawnInfoLikeCpp {
        object_type: SpawnObjectType::Creature,
        spawn_id,
        entry,
        respawn_time: 0,
        grid_id: 7,
    });

    let record = build_loaded_grid_creature_respawn_record_like_cpp(
        &mut map,
        SpawnObjectType::Creature,
        spawn_id,
        &metadata,
        &caches,
    )
    .expect("variable-level loaded-grid Creature builder should no longer block");
    let creature = record
        .primary_record
        .creature()
        .expect("builder should return a typed Creature MapObjectRecord");
    let level = creature.ai_level();

    assert!((18..=20).contains(&level));
    assert_eq!(
        record.primary_record.kind(),
        wow_entities::AccessorObjectKind::Creature
    );
    assert_eq!(creature.lifecycle_metadata().spawn_id, spawn_id);
    assert_eq!(
        creature.guid().high_type(),
        wow_core::guid::HighGuid::Creature
    );
    assert_eq!(creature.guid().realm_id(), 7);
    assert_eq!(u32::from(creature.guid().map_id()), 571);
    assert_eq!(creature.guid().entry(), entry);
    assert_eq!(creature.guid().counter(), 1);
    assert_ne!(creature.guid().counter(), spawn_id as i64);
    assert_eq!(creature.ai_max_health(), u64::from(level) * 20);
    assert_eq!(creature.ai_current_health(), creature.ai_max_health());
}

#[test]
fn login_grid_load_preserves_precreated_dungeon_kind_like_cpp() {
    let canonical: wow_world::SharedCanonicalMapManager =
        Arc::new(Mutex::new(wow_map::MapManager::default()));
    canonical.lock().unwrap().create_map_entry(
        33,
        77,
        0,
        wow_map::ManagedMapKind::Dungeon {
            has_reset_schedule: false,
        },
    );
    let legacy: wow_world::SharedMapManager = Arc::new(RwLock::new(wow_world::MapManager::new()));
    let metadata = Arc::new(Mutex::new(
        super::spawn_store_loader::CanonicalSpawnMetadataLikeCpp::new(
            SpawnStore::new(),
            BTreeMap::new(),
        ),
    ));
    let caches = empty_loaded_grid_creature_respawn_caches_like_cpp();
    let area_trigger_templates = area_trigger_template_store_for_loaded_grid_like_cpp(1, 1);
    let map_store = loaded_grid_map_store_like_cpp(33, wow_data::map::MAP_INSTANCE);

    let outcome = super::ensure_login_player_grid_loaded_like_cpp(
        &canonical,
        &legacy,
        &metadata,
        &caches,
        &area_trigger_templates,
        Some(&map_store),
        33,
        Some(77),
        Position::ZERO,
    );

    assert!(!outcome.map_unavailable);
    assert!(!outcome.map_created);
    assert!(matches!(
        canonical.lock().unwrap().find_map(33, 77).unwrap().kind(),
        wow_map::ManagedMapKind::Dungeon { .. }
    ));
}

#[test]
fn login_grid_load_accepts_authoritative_garrison_world_map_like_cpp() {
    let canonical: wow_world::SharedCanonicalMapManager =
        Arc::new(Mutex::new(wow_map::MapManager::default()));
    canonical.lock().unwrap().create_world_map(1_151, 0);
    let legacy: wow_world::SharedMapManager = Arc::new(RwLock::new(wow_world::MapManager::new()));
    let metadata = Arc::new(Mutex::new(
        super::spawn_store_loader::CanonicalSpawnMetadataLikeCpp::new(
            SpawnStore::new(),
            BTreeMap::new(),
        ),
    ));
    let caches = empty_loaded_grid_creature_respawn_caches_like_cpp();
    let area_trigger_templates = area_trigger_template_store_for_loaded_grid_like_cpp(1, 1);
    let map_store = wow_data::MapStore::from_entries([wow_data::MapEntry {
        id: 1_151,
        instance_type: wow_data::map::MAP_COMMON,
        expansion_id: 0,
        parent_map_id: -1,
        cosmetic_parent_map_id: -1,
        flags1: wow_data::map::MAP_FLAG_GARRISON,
        flags2: 0,
    }]);

    let outcome = super::ensure_login_player_grid_loaded_like_cpp(
        &canonical,
        &legacy,
        &metadata,
        &caches,
        &area_trigger_templates,
        Some(&map_store),
        1_151,
        Some(0),
        Position::ZERO,
    );

    assert!(!outcome.map_unavailable);
    assert!(!outcome.map_created);
    assert!(matches!(
        canonical.lock().unwrap().find_map(1_151, 0).unwrap().kind(),
        wow_map::ManagedMapKind::World
    ));
}

#[test]
fn login_grid_load_does_not_fabricate_missing_instanceable_map_like_cpp() {
    let canonical: wow_world::SharedCanonicalMapManager =
        Arc::new(Mutex::new(wow_map::MapManager::default()));
    let legacy: wow_world::SharedMapManager = Arc::new(RwLock::new(wow_world::MapManager::new()));
    let metadata = Arc::new(Mutex::new(
        super::spawn_store_loader::CanonicalSpawnMetadataLikeCpp::new(
            SpawnStore::new(),
            BTreeMap::new(),
        ),
    ));
    let caches = empty_loaded_grid_creature_respawn_caches_like_cpp();
    let area_trigger_templates = area_trigger_template_store_for_loaded_grid_like_cpp(1, 1);
    let map_store = loaded_grid_map_store_like_cpp(33, wow_data::map::MAP_INSTANCE);

    let outcome = super::ensure_login_player_grid_loaded_like_cpp(
        &canonical,
        &legacy,
        &metadata,
        &caches,
        &area_trigger_templates,
        Some(&map_store),
        33,
        Some(77),
        Position::ZERO,
    );

    assert!(outcome.map_unavailable);
    assert!(!outcome.map_created);
    assert!(canonical.lock().unwrap().find_map(33, 77).is_none());
}

#[test]
fn login_grid_load_rejects_stale_dungeon_zero_world_map_like_cpp() {
    let canonical: wow_world::SharedCanonicalMapManager =
        Arc::new(Mutex::new(wow_map::MapManager::default()));
    canonical.lock().unwrap().create_world_map(33, 0);
    let legacy: wow_world::SharedMapManager = Arc::new(RwLock::new(wow_world::MapManager::new()));
    let metadata = Arc::new(Mutex::new(
        super::spawn_store_loader::CanonicalSpawnMetadataLikeCpp::new(
            SpawnStore::new(),
            BTreeMap::new(),
        ),
    ));
    let caches = empty_loaded_grid_creature_respawn_caches_like_cpp();
    let area_trigger_templates = area_trigger_template_store_for_loaded_grid_like_cpp(1, 1);
    let map_store = loaded_grid_map_store_like_cpp(33, wow_data::map::MAP_INSTANCE);

    let outcome = super::ensure_login_player_grid_loaded_like_cpp(
        &canonical,
        &legacy,
        &metadata,
        &caches,
        &area_trigger_templates,
        Some(&map_store),
        33,
        None,
        Position::ZERO,
    );

    assert!(outcome.map_unavailable);
    assert!(!outcome.grid_loaded_now);
    assert!(matches!(
        canonical.lock().unwrap().find_map(33, 0).unwrap().kind(),
        wow_map::ManagedMapKind::World
    ));
}

#[test]
fn login_grid_load_can_materialize_missing_common_world_map_like_cpp() {
    let canonical: wow_world::SharedCanonicalMapManager =
        Arc::new(Mutex::new(wow_map::MapManager::default()));
    let legacy: wow_world::SharedMapManager = Arc::new(RwLock::new(wow_world::MapManager::new()));
    let metadata = Arc::new(Mutex::new(
        super::spawn_store_loader::CanonicalSpawnMetadataLikeCpp::new(
            SpawnStore::new(),
            BTreeMap::new(),
        ),
    ));
    let caches = empty_loaded_grid_creature_respawn_caches_like_cpp();
    let area_trigger_templates = area_trigger_template_store_for_loaded_grid_like_cpp(1, 1);
    let map_store = loaded_grid_map_store_like_cpp(571, wow_data::map::MAP_COMMON);

    let outcome = super::ensure_login_player_grid_loaded_like_cpp(
        &canonical,
        &legacy,
        &metadata,
        &caches,
        &area_trigger_templates,
        Some(&map_store),
        571,
        None,
        Position::ZERO,
    );

    assert!(!outcome.map_unavailable);
    assert!(outcome.map_created);
    assert!(matches!(
        canonical.lock().unwrap().find_map(571, 0).unwrap().kind(),
        wow_map::ManagedMapKind::World
    ));
}

#[test]
fn login_grid_load_does_not_guess_missing_faction_split_world_map_like_cpp() {
    let canonical: wow_world::SharedCanonicalMapManager =
        Arc::new(Mutex::new(wow_map::MapManager::default()));
    let legacy: wow_world::SharedMapManager = Arc::new(RwLock::new(wow_world::MapManager::new()));
    let metadata = Arc::new(Mutex::new(
        super::spawn_store_loader::CanonicalSpawnMetadataLikeCpp::new(
            SpawnStore::new(),
            BTreeMap::new(),
        ),
    ));
    let caches = empty_loaded_grid_creature_respawn_caches_like_cpp();
    let area_trigger_templates = area_trigger_template_store_for_loaded_grid_like_cpp(1, 1);
    let map_store = loaded_grid_map_store_like_cpp(609, wow_data::map::MAP_COMMON);

    let outcome = super::ensure_login_player_grid_loaded_like_cpp(
        &canonical,
        &legacy,
        &metadata,
        &caches,
        &area_trigger_templates,
        Some(&map_store),
        609,
        None,
        Position::ZERO,
    );

    assert!(outcome.map_unavailable);
    assert!(!outcome.map_created);
    assert!(canonical.lock().unwrap().find_map(609, 0).is_none());
}

#[test]
fn login_grid_load_mirrors_already_loaded_canonical_creature_to_legacy_like_cpp() {
    let spawn_id = 70_001;
    let entry = 42;
    let position = Position::new(1_000.0, 1_000.0, 0.0, 0.0);
    let guid =
        ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, entry, spawn_id as i64);

    let canonical: wow_world::SharedCanonicalMapManager =
        Arc::new(Mutex::new(wow_map::MapManager::default()));
    {
        let mut creature = Creature::new(false);
        creature.unit_mut().world_mut().object_mut().create(guid);
        creature
            .unit_mut()
            .world_mut()
            .object_mut()
            .set_entry(entry);
        creature.unit_mut().world_mut().set_map(571, 0).unwrap();
        creature.unit_mut().world_mut().relocate(position);
        creature.unit_mut().world_mut().object_mut().add_to_world();
        creature.set_spawn_id(spawn_id);

        canonical
            .lock()
            .unwrap()
            .create_world_map(571, 0)
            .map_mut()
            .add_map_object_record_to_map_like_cpp(MapObjectRecord::new_creature(creature).unwrap())
            .expect("test canonical creature add to map");
    }

    let legacy: wow_world::SharedMapManager = Arc::new(RwLock::new(wow_world::MapManager::new()));
    let mut store = SpawnStore::new();
    store.add_object_spawn(
        &SpawnData {
            object_type: SpawnObjectType::Creature,
            spawn_id,
            map_id: 571,
            db_data: true,
            spawn_group: SpawnGroupTemplateData::default_group(),
            id: entry,
            spawn_point: SpawnPosition::new(
                position.x,
                position.y,
                position.z,
                position.orientation,
            ),
            phase_use_flags: 0,
            phase_id: 0,
            phase_group: 0,
            terrain_swap_map: -1,
            pool_id: 0,
            spawn_time_secs: 120,
            spawn_difficulties: vec![0],
            script_id: 0,
            string_id: String::new(),
        },
        |_| false,
    );
    let metadata = Arc::new(Mutex::new(
        super::spawn_store_loader::CanonicalSpawnMetadataLikeCpp::new(store, BTreeMap::new()),
    ));
    let caches = empty_loaded_grid_creature_respawn_caches_like_cpp();

    let area_trigger_templates = area_trigger_template_store_for_loaded_grid_like_cpp(1, 1);
    let map_store = loaded_grid_map_store_like_cpp(571, wow_data::map::MAP_COMMON);

    let outcome = super::ensure_login_player_grid_loaded_like_cpp(
        &canonical,
        &legacy,
        &metadata,
        &caches,
        &area_trigger_templates,
        Some(&map_store),
        571,
        Some(0),
        position,
    );

    assert_eq!(outcome.skipped_already_loaded, 1);
    assert_eq!(outcome.creature_records_added, 0);
    assert_eq!(
        outcome.legacy_creature_mirrors, 1,
        "C++ has one Map object store; Rust's temporary canonical/legacy split must mirror already-loaded canonical creatures into the legacy tick world"
    );
    assert!(
        legacy.read().unwrap().find_creature(571, 0, guid).is_some(),
        "already-loaded canonical creature must be present in legacy MapManager so the creature tick can move it"
    );
}

#[test]
fn login_grid_load_materializes_visible_adjacent_ngrid_creature_like_cpp() {
    let spawn_id = 304_317;
    let entry = 3_114;
    let player_position = Position::new(545.38, -4209.53, 15.9, 0.0);
    let creature_position = Position::new(520.972, -4209.32, 15.9, 0.0);
    let player_cell = wow_map::cell_from_world(player_position.x, player_position.y);
    let creature_cell = wow_map::cell_from_world(creature_position.x, creature_position.y);
    assert_ne!(
        player_cell.grid_x(),
        creature_cell.grid_x(),
        "regression setup must place the player and nearby creature across a C++ NGrid boundary"
    );
    assert!(
        player_position.distance_2d(&creature_position)
            <= wow_world::map_manager::VISIBILITY_RADIUS,
        "regression creature must be inside the client visibility radius"
    );

    let canonical: wow_world::SharedCanonicalMapManager =
        Arc::new(Mutex::new(wow_map::MapManager::default()));
    let legacy: wow_world::SharedMapManager = Arc::new(RwLock::new(wow_world::MapManager::new()));
    let mut store = SpawnStore::new();
    store.add_object_spawn(
        &SpawnData {
            object_type: SpawnObjectType::Creature,
            spawn_id,
            map_id: 1,
            db_data: true,
            spawn_group: SpawnGroupTemplateData::default_group(),
            id: entry,
            spawn_point: SpawnPosition::new(
                creature_position.x,
                creature_position.y,
                creature_position.z,
                creature_position.orientation,
            ),
            phase_use_flags: 0,
            phase_id: 0,
            phase_group: 0,
            terrain_swap_map: -1,
            pool_id: 0,
            spawn_time_secs: 120,
            spawn_difficulties: vec![0],
            script_id: 0,
            string_id: String::new(),
        },
        |_| false,
    );
    let metadata = Arc::new(Mutex::new(
        super::spawn_store_loader::CanonicalSpawnMetadataLikeCpp::new(store, BTreeMap::new())
            .with_creature_runtime_rows_like_cpp(BTreeMap::from([(
                spawn_id,
                super::spawn_store_loader::CreatureSpawnRuntimeRowLikeCpp {
                    spawn_id,
                    model_id: 111,
                    equipment_id: 0,
                    wander_distance: 8.0,
                    curhealth: 0,
                    curmana: 0,
                    movement_type: 1,
                    npc_flags: None,
                    unit_flags: None,
                    unit_flags2: None,
                    unit_flags3: None,
                    ground_movement_type: wow_constants::CreatureGroundMovementType::Run as u8,
                    swim_allowed: true,
                    flight_movement_type: 0,
                    rooted: false,
                    chase_movement_type: wow_constants::CreatureChaseMovementType::Run as u8,
                    random_movement_type: wow_constants::CreatureRandomMovementType::Walk as u8,
                    interaction_pause_timer_ms:
                        wow_entities::DEFAULT_CREATURE_INTERACTION_PAUSE_TIMER_MS_LIKE_CPP,
                    string_id: "visible-adjacent-grid-creature".to_string(),
                    spawn_time_secs: 120,
                },
            )])),
    ));
    let mut caches =
        variable_loaded_grid_creature_respawn_caches_with_vehicle_id_and_difficulty_like_cpp(
            entry, 0, 0,
        );
    caches.realm_id = 7;
    let area_trigger_templates = area_trigger_template_store_for_loaded_grid_like_cpp(1, 1);
    canonical.lock().unwrap().create_world_map(1, 0);
    let map_store = loaded_grid_map_store_like_cpp(1, wow_data::map::MAP_COMMON);

    let outcome = super::ensure_login_player_grid_loaded_like_cpp(
        &canonical,
        &legacy,
        &metadata,
        &caches,
        &area_trigger_templates,
        Some(&map_store),
        1,
        Some(0),
        player_position,
    );

    assert_eq!(outcome.creature_records_added, 1);
    assert_eq!(outcome.legacy_creature_mirrors, 1);
    assert_eq!(outcome.load_record_missing, 0);
    assert_eq!(outcome.add_to_map_errors, 0);
    let guid = {
        let guard = canonical.lock().unwrap();
        guard
            .find_map(1, 0)
            .expect("login grid load should create the world map")
            .map()
            .get_creature_by_spawn_id_like_cpp(spawn_id)
            .expect("visible adjacent NGrid creature should be materialized")
            .guid()
    };
    assert_eq!(guid.realm_id(), 7);
    assert!(
        legacy.read().unwrap().find_creature(1, 0, guid).is_some(),
        "materialized canonical creature must be mirrored into the legacy visible/tick world"
    );
    assert!(
        legacy
            .read()
            .unwrap()
            .get_visible_creatures(
                1,
                0,
                player_position.x,
                player_position.y,
                player_position.z
            )
            .iter()
            .any(|creature| creature.guid() == guid),
        "nearby creature from the adjacent C++ NGrid must be visible after login"
    );
}

#[test]
fn login_grid_load_materializes_area_triggers_like_cpp() {
    let spawn_id = 70_101;
    let create_properties_id = 2003;
    let template_id = 9003;
    let position = Position::new(1.0, 2.0, 3.0, 0.5);

    let canonical: wow_world::SharedCanonicalMapManager =
        Arc::new(Mutex::new(wow_map::MapManager::default()));
    let legacy: wow_world::SharedMapManager = Arc::new(RwLock::new(wow_world::MapManager::new()));
    let mut store = SpawnStore::new();
    store.add_area_trigger_spawn(&SpawnData {
        object_type: SpawnObjectType::AreaTrigger,
        spawn_id,
        map_id: 571,
        db_data: true,
        spawn_group: SpawnGroupTemplateData::default_group(),
        id: create_properties_id,
        spawn_point: SpawnPosition::new(position.x, position.y, position.z, position.orientation),
        phase_use_flags: 0,
        phase_id: 0,
        phase_group: 0,
        terrain_swap_map: -1,
        pool_id: 0,
        spawn_time_secs: 0,
        spawn_difficulties: vec![0],
        script_id: 0,
        string_id: String::new(),
    });
    let metadata = Arc::new(Mutex::new(
        super::spawn_store_loader::CanonicalSpawnMetadataLikeCpp::new(store, BTreeMap::new())
            .with_area_trigger_runtime_rows_like_cpp(BTreeMap::from([(
                spawn_id,
                super::spawn_store_loader::AreaTriggerSpawnRuntimeRowLikeCpp {
                    spawn_id,
                    create_properties_id: wow_data::AreaTriggerIdLikeCpp {
                        id: create_properties_id,
                        is_custom: false,
                    },
                    spell_for_visuals: None,
                },
            )])),
    ));
    let caches = empty_loaded_grid_creature_respawn_caches_like_cpp();
    let area_trigger_templates =
        area_trigger_template_store_for_loaded_grid_like_cpp(create_properties_id, template_id);
    canonical.lock().unwrap().create_world_map(571, 0);
    let map_store = loaded_grid_map_store_like_cpp(571, wow_data::map::MAP_COMMON);

    let outcome = super::ensure_login_player_grid_loaded_like_cpp(
        &canonical,
        &legacy,
        &metadata,
        &caches,
        &area_trigger_templates,
        Some(&map_store),
        571,
        Some(0),
        position,
    );

    assert_eq!(outcome.area_trigger_records_added, 1);
    assert_eq!(outcome.load_record_missing, 0);
    assert_eq!(outcome.add_to_map_errors, 0);
    let guard = canonical.lock().unwrap();
    let area_trigger = guard
        .find_map(571, 0)
        .expect("login grid load should create the map")
        .map()
        .get_area_trigger_by_spawn_id_like_cpp(spawn_id)
        .expect("login grid load should materialize DB-backed AreaTrigger");
    assert_eq!(area_trigger.spawn_id(), spawn_id);
    assert_eq!(area_trigger.template_id().unwrap().id, template_id);
    assert_eq!(
        area_trigger.world().guid().high_type(),
        wow_core::guid::HighGuid::AreaTrigger
    );
}

#[test]
fn loaded_grid_creature_spawn_group_spawn_record_does_not_require_respawn_timer_like_cpp() {
    let spawn_id = 54_985;
    let entry = 42;
    let mut metadata =
        test_spawn_metadata_with_explicit_spawn_ids([(68, 571, SpawnGroupFlags::NONE, spawn_id)]);
    metadata = metadata.with_creature_runtime_rows_like_cpp(BTreeMap::from([(
        spawn_id,
        super::spawn_store_loader::CreatureSpawnRuntimeRowLikeCpp {
            spawn_id,
            model_id: 999,
            equipment_id: 3,
            wander_distance: 15.0,
            curhealth: 0,
            curmana: 0,
            movement_type: 1,
            npc_flags: None,
            unit_flags: None,
            unit_flags2: None,
            unit_flags3: None,
            ground_movement_type: wow_constants::CreatureGroundMovementType::Run as u8,
            swim_allowed: true,
            flight_movement_type: 0,
            rooted: false,
            chase_movement_type: wow_constants::CreatureChaseMovementType::Run as u8,
            random_movement_type: wow_constants::CreatureRandomMovementType::Walk as u8,
            interaction_pause_timer_ms:
                wow_entities::DEFAULT_CREATURE_INTERACTION_PAUSE_TIMER_MS_LIKE_CPP,
            string_id: "condition-spawn-no-timer".to_string(),
            spawn_time_secs: 120,
        },
    )]));
    let caches =
        variable_loaded_grid_creature_respawn_caches_with_vehicle_id_and_difficulty_like_cpp(
            entry, 0, 0,
        );
    let mut map = wow_map::Map::new(571, 0, 0, 60_000);
    assert_eq!(
        map.get_respawn_time_like_cpp(SpawnObjectType::Creature, spawn_id),
        0
    );

    let record = build_loaded_grid_creature_spawn_group_spawn_record_like_cpp(
        &mut map,
        SpawnObjectType::Creature,
        spawn_id,
        &metadata,
        &caches,
    )
    .expect("SpawnGroupSpawn loaded-grid Creature loader must not require a respawn timer");
    let creature = record
        .primary_record
        .creature()
        .expect("builder should return a typed Creature MapObjectRecord");

    assert_eq!(creature.respawn_time(), 0);
    assert_eq!(creature.lifecycle_metadata().spawn_id, spawn_id);
    assert_eq!(creature.guid().entry(), entry);
    assert_eq!(creature.guid().counter(), 1);
    assert_ne!(creature.guid().counter(), spawn_id as i64);
}

#[test]
fn spawn_group_condition_update_spawn_loads_loaded_grid_creature_without_respawn_timer_like_cpp() {
    let spawn_id = 54_986;
    let entry = 42;
    let mut metadata =
        test_spawn_metadata_with_explicit_spawn_ids([(69, 571, SpawnGroupFlags::NONE, spawn_id)]);
    metadata = metadata.with_creature_runtime_rows_like_cpp(BTreeMap::from([(
        spawn_id,
        super::spawn_store_loader::CreatureSpawnRuntimeRowLikeCpp {
            spawn_id,
            model_id: 999,
            equipment_id: 3,
            wander_distance: 15.0,
            curhealth: 0,
            curmana: 0,
            movement_type: 1,
            npc_flags: None,
            unit_flags: None,
            unit_flags2: None,
            unit_flags3: None,
            ground_movement_type: wow_constants::CreatureGroundMovementType::Run as u8,
            swim_allowed: true,
            flight_movement_type: 0,
            rooted: false,
            chase_movement_type: wow_constants::CreatureChaseMovementType::Run as u8,
            random_movement_type: wow_constants::CreatureRandomMovementType::Walk as u8,
            interaction_pause_timer_ms:
                wow_entities::DEFAULT_CREATURE_INTERACTION_PAUSE_TIMER_MS_LIKE_CPP,
            string_id: "condition-spawn-caller-no-timer".to_string(),
            spawn_time_secs: 120,
        },
    )]));
    let condition_store =
        ConditionEntriesByTypeStore::from_conditions_like_cpp([mapid_condition(69, 571)]);
    let caches =
        variable_loaded_grid_creature_respawn_caches_with_vehicle_id_and_difficulty_like_cpp(
            entry, 0, 0,
        );
    let mut manager = wow_map::MapManager::new(60_000, 10);
    let group = metadata
        .spawn_group_templates()
        .get(&69)
        .expect("test group 69")
        .clone();
    let map = manager.create_world_map(571, 0);
    map.map_mut()
        .set_spawn_group_inactive_like_cpp(Some(&group));
    assert!(map.map_mut().load_grid(0.0, 0.0));
    assert_eq!(
        map.map()
            .get_respawn_time_like_cpp(SpawnObjectType::Creature, spawn_id),
        0
    );

    let outcomes = apply_canonical_spawn_group_condition_update_loaded_grid_records_like_cpp(
        map,
        &metadata,
        &condition_store,
        &caches,
    );

    let spawn_outcome = outcomes
        .iter()
        .find(|outcome| outcome.group_id == 69)
        .and_then(|outcome| outcome.spawn_outcome.as_ref())
        .expect("condition-success SpawnGroupSpawn outcome");
    assert_eq!(spawn_outcome.executed_loaded_grid_spawns, 1);
    assert_eq!(spawn_outcome.blocked_loaded_grid_creature_loads, 0);
    assert_eq!(spawn_outcome.blocked_loaded_grid_spawn_loads, 0);
    assert_eq!(spawn_outcome.skipped_respawn_timer_active, 0);
    assert_eq!(map.map().map_object_count(), 1);
    let creature = map
        .map()
        .get_creature_by_spawn_id_like_cpp(spawn_id)
        .expect("loaded-grid Creature should be indexed by spawn id");
    assert_eq!(creature.respawn_time(), 0);
    assert_eq!(creature.lifecycle_metadata().spawn_id, spawn_id);
    assert_eq!(creature.guid().counter(), 1);
    assert_ne!(creature.guid().counter(), spawn_id as i64);
}

#[test]
fn spawn_group_condition_update_tick_mirrors_loaded_grid_creature_to_legacy_like_cpp() {
    let spawn_id = 54_987;
    let entry = 42;
    let mut metadata =
        test_spawn_metadata_with_explicit_spawn_ids([(69, 571, SpawnGroupFlags::NONE, spawn_id)]);
    metadata = metadata.with_creature_runtime_rows_like_cpp(BTreeMap::from([(
        spawn_id,
        super::spawn_store_loader::CreatureSpawnRuntimeRowLikeCpp {
            spawn_id,
            model_id: 999,
            equipment_id: 3,
            wander_distance: 15.0,
            curhealth: 0,
            curmana: 0,
            movement_type: 1,
            npc_flags: None,
            unit_flags: None,
            unit_flags2: None,
            unit_flags3: None,
            ground_movement_type: wow_constants::CreatureGroundMovementType::Run as u8,
            swim_allowed: true,
            flight_movement_type: 0,
            rooted: false,
            chase_movement_type: wow_constants::CreatureChaseMovementType::Run as u8,
            random_movement_type: wow_constants::CreatureRandomMovementType::Walk as u8,
            interaction_pause_timer_ms:
                wow_entities::DEFAULT_CREATURE_INTERACTION_PAUSE_TIMER_MS_LIKE_CPP,
            string_id: "condition-spawn-caller-legacy-mirror".to_string(),
            spawn_time_secs: 120,
        },
    )]));
    let condition_store =
        ConditionEntriesByTypeStore::from_conditions_like_cpp([mapid_condition(69, 571)]);
    let caches =
        variable_loaded_grid_creature_respawn_caches_with_vehicle_id_and_difficulty_like_cpp(
            entry, 0, 0,
        );
    let legacy: wow_world::SharedMapManager =
        Arc::new(std::sync::RwLock::new(wow_world::MapManager::new()));
    let mut manager = wow_map::MapManager::new(60_000, 1);
    let group = metadata
        .spawn_group_templates()
        .get(&69)
        .expect("test group 69")
        .clone();
    let map = manager.create_world_map(571, 0);
    map.map_mut()
        .set_spawn_group_inactive_like_cpp(Some(&group));
    assert!(map.map_mut().load_grid(0.0, 0.0));
    let mut scheduler = CanonicalRespawnConditionSchedulerLikeCpp::new(1);

    let summary = canonical_map_update_tick_set_inactive_like_cpp(
        &mut manager,
        Some(&legacy),
        1,
        &mut scheduler,
        &metadata,
        &condition_store,
        &canonical_test_map_store_like_cpp(),
        &caches,
    )
    .expect("scheduler fires and condition spawn executes");

    assert_eq!(summary.condition_spawn_executed_loaded_grid_spawns, 1);
    assert_eq!(summary.condition_spawn_legacy_creature_mirrors, 1);
    let creature = manager
        .find_map(571, 0)
        .expect("canonical map")
        .map()
        .get_creature_by_spawn_id_like_cpp(spawn_id)
        .expect("canonical loaded-grid creature");
    assert_eq!(creature.guid().counter(), 1);
    assert_ne!(creature.guid().counter(), spawn_id as i64);
    assert!(
        legacy
            .read()
            .unwrap()
            .find_creature(571, 0, creature.guid())
            .is_some(),
        "C++ AddToMap has one live runtime; Rust split runtime must mirror internal wow-map loaded-grid inserts into legacy"
    );
}

#[test]
fn loaded_grid_creature_respawn_record_vehicle_template_uses_creature_low_vehicle_high_like_cpp() {
    let spawn_id = 54_988;
    let entry = 42;
    let mut metadata =
        test_spawn_metadata_with_explicit_spawn_ids([(67, 571, SpawnGroupFlags::NONE, spawn_id)]);
    metadata = metadata.with_creature_runtime_rows_like_cpp(BTreeMap::from([(
        spawn_id,
        super::spawn_store_loader::CreatureSpawnRuntimeRowLikeCpp {
            spawn_id,
            model_id: 999,
            equipment_id: 3,
            wander_distance: 15.0,
            curhealth: 0,
            curmana: 0,
            movement_type: 1,
            npc_flags: None,
            unit_flags: None,
            unit_flags2: None,
            unit_flags3: None,
            ground_movement_type: wow_constants::CreatureGroundMovementType::Run as u8,
            swim_allowed: true,
            flight_movement_type: 0,
            rooted: false,
            chase_movement_type: wow_constants::CreatureChaseMovementType::Run as u8,
            random_movement_type: wow_constants::CreatureRandomMovementType::Walk as u8,
            interaction_pause_timer_ms:
                wow_entities::DEFAULT_CREATURE_INTERACTION_PAUSE_TIMER_MS_LIKE_CPP,
            string_id: "vehicle-template-live".to_string(),
            spawn_time_secs: 120,
        },
    )]));
    let mut caches =
        variable_loaded_grid_creature_respawn_caches_with_vehicle_id_like_cpp(entry, 101);
    caches.realm_id = 7;
    let entry_accessory = wow_entities::VehicleAccessory {
        accessory_entry: 7001,
        seat_id: 1,
        is_minion: false,
        summoned_type: 6,
        summon_time_ms: 3_000,
    };
    let spawn_accessory = wow_entities::VehicleAccessory {
        accessory_entry: 8001,
        seat_id: 2,
        is_minion: true,
        summoned_type: 8,
        summon_time_ms: 4_000,
    };
    caches.vehicle_accessory_store = Arc::new(wow_data::VehicleAccessoryStoreLikeCpp::from_parts(
        [(spawn_id, vec![spawn_accessory])],
        [(entry, vec![entry_accessory])],
    ));
    let mut map = wow_map::Map::new(571, 0, 2, 60_000);
    map.add_respawn_info_like_cpp(RespawnInfoLikeCpp {
        object_type: SpawnObjectType::Creature,
        spawn_id,
        entry,
        respawn_time: 0,
        grid_id: 7,
    });

    let record = build_loaded_grid_creature_respawn_record_like_cpp(
        &mut map,
        SpawnObjectType::Creature,
        spawn_id,
        &metadata,
        &caches,
    )
    .expect("vehicle-template loaded-grid Creature builder should resolve");
    let creature = record
        .primary_record
        .creature()
        .expect("builder should return a typed Creature MapObjectRecord");

    assert_eq!(
        creature.guid().high_type(),
        wow_core::guid::HighGuid::Vehicle
    );
    assert_eq!(creature.guid().realm_id(), 7);
    assert_eq!(creature.guid().counter(), 1);
    assert_ne!(creature.guid().counter(), spawn_id as i64);
    assert_eq!(creature.guid().entry(), entry);
    assert_eq!(creature.lifecycle_metadata().spawn_id, spawn_id);
    assert_eq!(creature.lifecycle_metadata().vehicle_id, Some(101));
    let kit = creature
        .unit()
        .subsystems()
        .vehicle
        .kit
        .as_ref()
        .expect("VehicleEntry-backed template should create a local kit");
    assert_eq!(kit.kit_id(), 101);
    assert!(kit.active());
    assert!(!kit.installed());
    assert_eq!(kit.seat_count(), 2);
    assert_eq!(kit.usable_seat_num(), 1);
    let outcome = creature
        .unit()
        .subsystems()
        .vehicle
        .last_create_outcome
        .as_ref()
        .expect("CreateVehicleKit evidence should be recorded");
    assert!(outcome.created);
    assert_eq!(outcome.seat_count, 2);
    assert_eq!(outcome.usable_seat_num, 1);
    assert!(outcome.update_display_power_represented);
    assert!(!outcome.send_set_vehicle_rec_id_represented);
    let reset_context = creature
        .add_to_world_vehicle_reset_context_like_cpp()
        .expect("VehicleEntry-backed template should build AddToWorld reset context");
    assert!(!reset_context.is_mechanical_creature);
    assert!(!reset_context.is_world_boss);
    assert_eq!(reset_context.accessories, vec![spawn_accessory]);
}

#[test]
fn loaded_grid_creature_respawn_record_vehicle_high_guid_without_kit_when_vehicle_row_missing_like_cpp()
 {
    let mut metadata = test_spawn_metadata_with_flags([(67, 571, SpawnGroupFlags::NONE)]);
    let spawn_id = 1;
    let entry = 42;
    metadata = metadata.with_creature_runtime_rows_like_cpp(BTreeMap::from([(
        spawn_id,
        super::spawn_store_loader::CreatureSpawnRuntimeRowLikeCpp {
            spawn_id,
            model_id: 999,
            equipment_id: 3,
            wander_distance: 15.0,
            curhealth: 0,
            curmana: 0,
            movement_type: 1,
            npc_flags: None,
            unit_flags: None,
            unit_flags2: None,
            unit_flags3: None,
            ground_movement_type: wow_constants::CreatureGroundMovementType::Run as u8,
            swim_allowed: true,
            flight_movement_type: 0,
            rooted: false,
            chase_movement_type: wow_constants::CreatureChaseMovementType::Run as u8,
            random_movement_type: wow_constants::CreatureRandomMovementType::Walk as u8,
            interaction_pause_timer_ms:
                wow_entities::DEFAULT_CREATURE_INTERACTION_PAUSE_TIMER_MS_LIKE_CPP,
            string_id: "vehicle-template-missing-row".to_string(),
            spawn_time_secs: 120,
        },
    )]));
    let mut caches =
        variable_loaded_grid_creature_respawn_caches_with_vehicle_id_like_cpp(entry, 101);
    caches.vehicle_store = Arc::new(wow_data::VehicleStore::from_entries([]));
    let mut map = wow_map::Map::new(571, 0, 2, 60_000);
    map.add_respawn_info_like_cpp(RespawnInfoLikeCpp {
        object_type: SpawnObjectType::Creature,
        spawn_id,
        entry,
        respawn_time: 0,
        grid_id: 7,
    });

    let record = build_loaded_grid_creature_respawn_record_like_cpp(
        &mut map,
        SpawnObjectType::Creature,
        spawn_id,
        &metadata,
        &caches,
    )
    .expect("vehicle-template loaded-grid Creature builder should still resolve");
    let creature = record
        .primary_record
        .creature()
        .expect("builder should return a typed Creature MapObjectRecord");

    assert_eq!(
        creature.guid().high_type(),
        wow_core::guid::HighGuid::Vehicle
    );
    assert_eq!(creature.lifecycle_metadata().vehicle_id, Some(101));
    assert!(creature.unit().subsystems().vehicle.kit.is_none());
    let outcome = creature
        .unit()
        .subsystems()
        .vehicle
        .last_create_outcome
        .as_ref()
        .expect("CreateVehicleKit false evidence should be recorded");
    assert_eq!(outcome.kit_id, Some(101));
    assert!(!outcome.created);
    assert!(!outcome.update_display_power_represented);
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
        .get_typed_creature(guid)
        .expect("canonical test creature")
        .clone();
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

/// (1) NearbyVisible: players on a different map_id are not enqueued.
/// C++ anchor: MessageDistDeliverer::Visit — map-id check before distance.
#[test]
fn nearby_visible_filters_by_map_id_like_cpp() {
    let registry = PlayerRegistry::with_canonical_player_fixtures_like_cpp();
    let guid = ObjectGuid::create_player(1, 1);
    let (info, command_rx) = make_registry_player_like_cpp(530, 0, Position::ZERO, true); // wrong map
    registry.register_or_replace(guid, info, Default::default());

    let event = make_nearby_visible_event_like_cpp(571, 0, Position::ZERO, 100.0, false);
    let plan = wow_world::map_manager::RuntimePlan {
        events: vec![event],
    };
    let summary = deliver_runtime_plan_like_cpp(&plan, &registry);

    assert_eq!(summary.candidates_queued, 0);
    assert_eq!(summary.candidates_skipped_wrong_map, 1);
    assert!(command_rx.try_recv().is_err());
}

/// (2) NearbyVisible: players on a different instance_id are not enqueued.
/// Slice 4A.1b requirement — instance separation.
#[test]
fn nearby_visible_filters_by_instance_id_like_cpp() {
    let registry = PlayerRegistry::with_canonical_player_fixtures_like_cpp();
    let guid = ObjectGuid::create_player(1, 2);
    let (info, command_rx) = make_registry_player_like_cpp(571, 99, Position::ZERO, true); // wrong instance
    registry.register_or_replace(guid, info, Default::default());

    let event = make_nearby_visible_event_like_cpp(571, 0, Position::ZERO, 100.0, false);
    let plan = wow_world::map_manager::RuntimePlan {
        events: vec![event],
    };
    let summary = deliver_runtime_plan_like_cpp(&plan, &registry);

    assert_eq!(summary.candidates_queued, 0);
    assert_eq!(summary.candidates_skipped_wrong_instance, 1);
    assert!(command_rx.try_recv().is_err());
}

/// (3) NearbyVisible: players not in world are not enqueued.
/// C++ anchor: MessageDistDeliverer::Visit — `Player::IsInWorld()` gate.
#[test]
fn nearby_visible_filters_is_in_world_like_cpp() {
    let registry = PlayerRegistry::with_canonical_player_fixtures_like_cpp();
    let guid = ObjectGuid::create_player(1, 3);
    let (info, command_rx) = make_registry_player_like_cpp(571, 0, Position::ZERO, false); // not in world
    registry.register_or_replace(guid, info, Default::default());

    let event = make_nearby_visible_event_like_cpp(571, 0, Position::ZERO, 100.0, false);
    let plan = wow_world::map_manager::RuntimePlan {
        events: vec![event],
    };
    let summary = deliver_runtime_plan_like_cpp(&plan, &registry);

    assert_eq!(summary.candidates_queued, 0);
    assert_eq!(summary.candidates_skipped_not_in_world, 1);
    assert!(command_rx.try_recv().is_err());
}

/// (4) NearbyVisible: 2D distance check excludes players beyond range when
/// `required_3d == false` — Z-axis is ignored.
/// C++ anchor: GridNotifiersImpl.h MessageDistDeliverer::Visit ~43-46.
#[test]
fn nearby_visible_uses_2d_distance_when_required_3d_false_like_cpp() {
    let registry = PlayerRegistry::with_canonical_player_fixtures_like_cpp();
    // Player is far on Z but close in XY — should be INCLUDED with 2D check.
    let near_guid = ObjectGuid::create_player(1, 4);
    let (near_info, near_rx) =
        make_registry_player_like_cpp(571, 0, Position::new(5.0, 0.0, 1000.0, 0.0), true);
    registry.register_or_replace(near_guid, near_info, Default::default());

    // Player is far in XY — should be EXCLUDED.
    let far_guid = ObjectGuid::create_player(1, 5);
    let (far_info, far_rx) =
        make_registry_player_like_cpp(571, 0, Position::new(200.0, 0.0, 0.0, 0.0), true);
    registry.register_or_replace(far_guid, far_info, Default::default());

    let source = Position::new(0.0, 0.0, 0.0, 0.0);
    let event = make_nearby_visible_event_like_cpp(571, 0, source, 100.0, false);
    let plan = wow_world::map_manager::RuntimePlan {
        events: vec![event],
    };
    let summary = deliver_runtime_plan_like_cpp(&plan, &registry);

    assert_eq!(summary.candidates_queued, 1, "only the XY-near player");
    assert_eq!(summary.candidates_skipped_distance, 1);
    assert!(near_rx.try_recv().is_ok(), "near player got command");
    assert!(far_rx.try_recv().is_err(), "far player did not get command");
}

/// (5) NearbyVisible: 3D distance check excludes players beyond range when
/// `required_3d == true` — Z-axis contributes to distance.
/// C++ anchor: GridNotifiersImpl.h MessageDistDeliverer::Visit ~43-46.
#[test]
fn nearby_visible_uses_3d_distance_when_required_3d_true_like_cpp() {
    let registry = PlayerRegistry::with_canonical_player_fixtures_like_cpp();
    // Player is close in XY but far on Z — should be EXCLUDED with 3D check.
    let near_xy_guid = ObjectGuid::create_player(1, 6);
    let (near_xy_info, near_xy_rx) =
        make_registry_player_like_cpp(571, 0, Position::new(5.0, 0.0, 200.0, 0.0), true);
    registry.register_or_replace(near_xy_guid, near_xy_info, Default::default());

    // Player is close in 3D — should be INCLUDED.
    let near_3d_guid = ObjectGuid::create_player(1, 7);
    let (near_3d_info, near_3d_rx) =
        make_registry_player_like_cpp(571, 0, Position::new(3.0, 3.0, 3.0, 0.0), true);
    registry.register_or_replace(near_3d_guid, near_3d_info, Default::default());

    let source = Position::new(0.0, 0.0, 0.0, 0.0);
    let event = make_nearby_visible_event_like_cpp(571, 0, source, 10.0, true);
    let plan = wow_world::map_manager::RuntimePlan {
        events: vec![event],
    };
    let summary = deliver_runtime_plan_like_cpp(&plan, &registry);

    assert_eq!(summary.candidates_queued, 1, "only the 3D-near player");
    assert_eq!(summary.candidates_skipped_distance, 1);
    assert!(near_xy_rx.try_recv().is_err(), "far-Z player excluded");
    assert!(near_3d_rx.try_recv().is_ok(), "3D-near player included");
}

#[test]
fn nearby_visible_durable_uses_committed_fifo_instead_of_bounded_queue() {
    let registry = PlayerRegistry::with_canonical_player_fixtures_like_cpp();
    let guid = ObjectGuid::create_player(1, 8);
    let (info, command_rx) = make_registry_player_like_cpp(571, 0, Position::ZERO, true);
    let durable = Arc::clone(&info.durable_creature_runtime_commands_like_cpp);
    registry.register_or_replace(guid, info, Default::default());

    let event = wow_world::map_manager::RuntimeEvent {
        source_guid: make_source_guid(),
        recipients: wow_world::map_manager::RecipientRule::NearbyVisibleDurable {
            source_guid: make_source_guid(),
            map_id: 571,
            instance_id: 0,
            source_position: Position::ZERO,
            range: 100.0,
            required_3d: false,
        },
        packet_bytes: vec![0xAA],
    };
    let plan = wow_world::map_manager::RuntimePlan {
        events: vec![event],
    };
    let summary = deliver_runtime_plan_like_cpp(&plan, &registry);

    assert_eq!(summary.candidates_queued, 1);
    assert!(command_rx.try_recv().is_err());
    let drained = durable.lock().unwrap().drain_like_cpp();
    assert!(matches!(
        drained.as_slice(),
        [SessionCommand::SendIfVisibleLikeCpp(command)]
            if command.packet_bytes == vec![0xAA]
    ));
}

#[test]
fn creature_spell_start_go_is_one_atomic_observer_command_without_victim_drain_like_cpp() {
    let registry = PlayerRegistry::with_canonical_player_fixtures_like_cpp();
    let victim_guid = ObjectGuid::create_player(1, 80);
    let observer_guid = ObjectGuid::create_player(1, 81);
    let (victim_info, _victim_command_rx) =
        make_registry_player_like_cpp(571, 0, Position::ZERO, true);
    let (observer_info, _observer_command_rx) =
        make_registry_player_like_cpp(571, 0, Position::new(25.0, 0.0, 0.0, 0.0), true);
    victim_info
        .client_visible_guids_like_cpp
        .insert(make_source_guid());
    observer_info
        .client_visible_guids_like_cpp
        .insert(make_source_guid());
    registry.register_or_replace(victim_guid, victim_info, Default::default());
    registry.register_or_replace(observer_guid, observer_info, Default::default());

    let (plan, start_bytes, basic_go_bytes, full_go_bytes) =
        make_creature_spell_runtime_plan_like_cpp(victim_guid);
    let plan_delivery = deliver_runtime_plan_like_cpp(&plan, &registry);
    assert_eq!(plan_delivery.events_seen, 1);
    assert_eq!(plan_delivery.candidates_seen, 2);
    assert_eq!(plan_delivery.candidates_queued, 2);

    // Drain the observer first: its START/GO delivery does not depend on
    // the victim session making progress on its own durable FIFO.
    let observer_commands =
        drain_durable_creature_runtime_commands_like_cpp(&registry, observer_guid);
    let [SessionCommand::SendCreatureSpellCastIfVisibleLikeCpp(cast)] =
        observer_commands.as_slice()
    else {
        panic!("observer must receive one atomic START+GO command: {observer_commands:?}");
    };
    assert_eq!(cast.start_packet_bytes, start_bytes);
    assert_eq!(
        cast.go_packet_bytes, basic_go_bytes,
        "a receiver without advanced combat logging commits the basic frame"
    );
    assert_ne!(cast.go_packet_bytes, full_go_bytes);
    assert_eq!(
        u16::from_le_bytes(cast.start_packet_bytes[..2].try_into().unwrap()),
        ServerOpcodes::SpellStart as u16
    );
    assert_eq!(
        u16::from_le_bytes(cast.go_packet_bytes[..2].try_into().unwrap()),
        ServerOpcodes::SpellGo as u16
    );

    // The victim's untouched FIFO retains one indivisible copy as well.
    let victim_commands = drain_durable_creature_runtime_commands_like_cpp(&registry, victim_guid);
    let [SessionCommand::SendCreatureSpellCastIfVisibleLikeCpp(victim_cast)] =
        victim_commands.as_slice()
    else {
        panic!("victim FIFO must contain one atomic START+GO command: {victim_commands:?}");
    };
    assert_eq!(victim_cast.start_packet_bytes, start_bytes);
    assert_eq!(victim_cast.go_packet_bytes, basic_go_bytes);
}

#[test]
fn creature_spell_plan_skips_invisible_observer_but_reaches_victim_like_cpp() {
    let registry = PlayerRegistry::with_canonical_player_fixtures_like_cpp();
    let victim_guid = ObjectGuid::create_player(1, 82);
    let invisible_observer_guid = ObjectGuid::create_player(1, 83);
    let (victim_info, _victim_command_rx) =
        make_registry_player_like_cpp(571, 0, Position::ZERO, true);
    let (invisible_observer_info, _observer_command_rx) =
        make_registry_player_like_cpp(571, 0, Position::new(250.0, 0.0, 0.0, 0.0), true);
    // Both viewers already have the caster at client; only range separates
    // them here.
    victim_info
        .client_visible_guids_like_cpp
        .insert(make_source_guid());
    invisible_observer_info
        .client_visible_guids_like_cpp
        .insert(make_source_guid());
    registry.register_or_replace(victim_guid, victim_info, Default::default());
    registry.register_or_replace(
        invisible_observer_guid,
        invisible_observer_info,
        Default::default(),
    );

    let (plan, start_bytes, basic_go_bytes, full_go_bytes) =
        make_creature_spell_runtime_plan_like_cpp(victim_guid);
    let plan_delivery = deliver_runtime_plan_like_cpp(&plan, &registry);
    assert_eq!(plan_delivery.events_seen, 1);
    assert_eq!(plan_delivery.candidates_seen, 2);
    assert_eq!(plan_delivery.candidates_queued, 1);
    assert_eq!(plan_delivery.candidates_skipped_distance, 1);

    assert!(
        drain_durable_creature_runtime_commands_like_cpp(&registry, invisible_observer_guid)
            .is_empty(),
        "out-of-range observer must not receive START or GO"
    );

    let victim_commands = drain_durable_creature_runtime_commands_like_cpp(&registry, victim_guid);
    let [SessionCommand::SendCreatureSpellCastIfVisibleLikeCpp(cast)] = victim_commands.as_slice()
    else {
        panic!("victim must retain one atomic START+GO command: {victim_commands:?}");
    };
    assert_eq!(cast.start_packet_bytes, start_bytes);
    assert_eq!(cast.go_packet_bytes, basic_go_bytes);
}

/// C++ selects recipients inside `SendSpellGo` from each viewer's
/// `HaveAtClient`, so a viewer that does not have the caster at client when
/// the cast resolves never gets a command — becoming visible afterwards
/// cannot deliver the older cast.
#[test]
fn creature_spell_plan_commits_have_at_client_at_resolution_like_cpp() {
    let registry = PlayerRegistry::with_canonical_player_fixtures_like_cpp();
    let victim_guid = ObjectGuid::create_player(1, 84);
    let unaware_guid = ObjectGuid::create_player(1, 85);
    let (victim_info, _victim_command_rx) =
        make_registry_player_like_cpp(571, 0, Position::ZERO, true);
    let (unaware_info, _unaware_command_rx) =
        make_registry_player_like_cpp(571, 0, Position::new(25.0, 0.0, 0.0, 0.0), true);
    victim_info
        .client_visible_guids_like_cpp
        .insert(make_source_guid());
    let unaware_visibility = unaware_info.client_visible_guids_like_cpp.clone();
    registry.register_or_replace(victim_guid, victim_info, Default::default());
    registry.register_or_replace(unaware_guid, unaware_info, Default::default());

    let (plan, _start_bytes, _basic_go_bytes, _full_go_bytes) =
        make_creature_spell_runtime_plan_like_cpp(victim_guid);
    let plan_delivery = deliver_runtime_plan_like_cpp(&plan, &registry);

    assert_eq!(plan_delivery.candidates_seen, 2);
    assert_eq!(plan_delivery.candidates_queued, 1);
    assert_eq!(plan_delivery.candidates_skipped_not_visible, 1);
    assert_eq!(plan_delivery.candidates_skipped_distance, 0);

    // The caster becoming visible after the cast resolved must not conjure a
    // command that was never committed.
    unaware_visibility.insert(make_source_guid());
    assert!(
        drain_durable_creature_runtime_commands_like_cpp(&registry, unaware_guid).is_empty(),
        "a viewer that was not selected at commit time receives nothing"
    );
    assert_eq!(
        drain_durable_creature_runtime_commands_like_cpp(&registry, victim_guid).len(),
        1
    );
}

/// (6) MapBroadcastVisible: enqueues all players on the same map/instance
/// regardless of distance, but respects map/instance/in_world.
/// C++ anchor: WorldObject::SendMessageToSet map-wide broadcast path.
#[test]
fn map_broadcast_visible_ignores_distance_but_respects_map_instance_in_world_like_cpp() {
    let registry = PlayerRegistry::with_canonical_player_fixtures_like_cpp();

    // In range player — correct map/instance.
    let in_guid = ObjectGuid::create_player(1, 10);
    let (in_info, in_rx) =
        make_registry_player_like_cpp(571, 0, Position::new(9999.0, 9999.0, 0.0, 0.0), true);
    registry.register_or_replace(in_guid, in_info, Default::default());

    // Wrong map.
    let wrong_map_guid = ObjectGuid::create_player(1, 11);
    let (wrong_map_info, wrong_map_rx) =
        make_registry_player_like_cpp(530, 0, Position::ZERO, true);
    registry.register_or_replace(wrong_map_guid, wrong_map_info, Default::default());

    // Not in world.
    let no_world_guid = ObjectGuid::create_player(1, 12);
    let (no_world_info, no_world_rx) = make_registry_player_like_cpp(571, 0, Position::ZERO, false);
    registry.register_or_replace(no_world_guid, no_world_info, Default::default());

    let event = wow_world::map_manager::RuntimeEvent {
        source_guid: make_source_guid(),
        recipients: wow_world::map_manager::RecipientRule::MapBroadcastVisible {
            map_id: 571,
            instance_id: 0,
        },
        packet_bytes: vec![0xCC],
    };
    let plan = wow_world::map_manager::RuntimePlan {
        events: vec![event],
    };
    let summary = deliver_runtime_plan_like_cpp(&plan, &registry);

    assert_eq!(summary.candidates_queued, 1);
    assert!(in_rx.try_recv().is_ok(), "valid player got command");
    assert!(wrong_map_rx.try_recv().is_err(), "wrong-map excluded");
    assert!(no_world_rx.try_recv().is_err(), "not-in-world excluded");
}

/// (7) ExplicitPlayer: command sent to exactly one GUID, no other sessions.
/// C++ anchor: WorldObject::SendMessageToSet explicit receiver path.
#[test]
fn explicit_player_routes_only_to_target_guid_like_cpp() {
    let registry = PlayerRegistry::with_canonical_player_fixtures_like_cpp();
    let target_guid = ObjectGuid::create_player(1, 20);
    let other_guid = ObjectGuid::create_player(1, 21);
    let (target_info, target_rx) = make_registry_player_like_cpp(571, 0, Position::ZERO, true);
    let (other_info, other_rx) = make_registry_player_like_cpp(571, 0, Position::ZERO, true);
    registry.register_or_replace(target_guid, target_info, Default::default());
    registry.register_or_replace(other_guid, other_info, Default::default());

    let event = wow_world::map_manager::RuntimeEvent {
        source_guid: make_source_guid(),
        recipients: wow_world::map_manager::RecipientRule::ExplicitPlayer(target_guid),
        packet_bytes: vec![0xDD],
    };
    let plan = wow_world::map_manager::RuntimePlan {
        events: vec![event],
    };
    let summary = deliver_runtime_plan_like_cpp(&plan, &registry);

    assert_eq!(summary.candidates_queued, 1);
    assert!(target_rx.try_recv().is_ok(), "target received command");
    assert!(other_rx.try_recv().is_err(), "other session NOT notified");
}

#[test]
fn runtime_directory_delivery_rejects_replaced_recipient_generation() {
    let registry = PlayerRegistry::with_canonical_player_fixtures_like_cpp();
    let guid = ObjectGuid::create_player(1, 22);
    let (first_info, first_rx) = make_registry_player_like_cpp(571, 0, Position::ZERO, true);
    registry.register_or_replace(guid, first_info, Default::default());
    let stale = registry.runtime_recipient(guid).expect("first recipient");

    let (second_info, second_rx) = make_registry_player_like_cpp(571, 0, Position::ZERO, true);
    let current = registry.register_or_replace(guid, second_info, Default::default());
    let command = SessionCommand::KickLikeCpp(wow_world::session::mailbox::KickLikeCppCommand {
        reason: "stale runtime delivery".to_string(),
    });

    assert_eq!(
        registry.try_send_current_command(stale.registration, command),
        Err(wow_world::session::directory::PlayerDirectorySendError::StaleRegistration)
    );
    assert!(first_rx.try_recv().is_err());
    assert!(second_rx.try_recv().is_err());

    registry
        .try_send_current_command(
            current,
            SessionCommand::KickLikeCpp(wow_world::session::mailbox::KickLikeCppCommand {
                reason: "current runtime delivery".to_string(),
            }),
        )
        .expect("current recipient remains addressable");
    assert!(second_rx.try_recv().is_ok());
}

/// (8) SelfOnly: NO broadcast global; increments self_only_skipped counter.
/// Guarantees SelfOnly events are not distributed to any registry session.
/// C++ anchor: WorldObject::SendMessageToSet — self-send path bypasses
/// MessageDistDeliverer entirely.
#[test]
fn self_only_does_not_broadcast_to_any_session_like_cpp() {
    let registry = PlayerRegistry::with_canonical_player_fixtures_like_cpp();
    // Even with a matching player in registry, SelfOnly must NOT deliver.
    let guid = ObjectGuid::create_player(1, 30);
    let (info, command_rx) = make_registry_player_like_cpp(571, 0, Position::ZERO, true);
    registry.register_or_replace(guid, info, Default::default());

    let event = wow_world::map_manager::RuntimeEvent {
        source_guid: make_source_guid(),
        recipients: wow_world::map_manager::RecipientRule::SelfOnly,
        packet_bytes: vec![0xEE],
    };
    let plan = wow_world::map_manager::RuntimePlan {
        events: vec![event],
    };
    let summary = deliver_runtime_plan_like_cpp(&plan, &registry);

    assert_eq!(summary.self_only_skipped, 1, "must count skipped SelfOnly");
    assert_eq!(
        summary.candidates_queued, 0,
        "must NOT broadcast to registry"
    );
    assert_eq!(summary.candidates_seen, 0, "no candidates should be seen");
    assert!(
        command_rx.try_recv().is_err(),
        "session must NOT receive command"
    );
}

/// (9) try_send on a full channel increments send_failed and does NOT block.
/// Backpressure requirement from Slice 4A.1b spec.
#[test]
fn full_command_channel_increments_send_failed_and_does_not_block_like_cpp() {
    let registry = PlayerRegistry::with_canonical_player_fixtures_like_cpp();
    let guid = ObjectGuid::create_player(1, 40);

    let (send_tx, _send_rx) = flume::bounded::<Vec<u8>>(1);
    // Drop the receiver so try_send returns Err::Disconnected immediately.
    let (command_tx, command_rx) = flume::bounded::<SessionCommand>(1);
    drop(command_rx);

    let mut info = player_registration_fixture_like_cpp(send_tx, command_tx, "Full");
    info.placement.map_id = 571;
    info.placement.instance_id = 0;
    info.placement.is_in_world = true;
    info.placement.position = Position::ZERO;
    registry.register_or_replace(guid, info, Default::default());

    let event = make_nearby_visible_event_like_cpp(571, 0, Position::ZERO, 1000.0, false);
    let plan = wow_world::map_manager::RuntimePlan {
        events: vec![event],
    };
    let summary = deliver_runtime_plan_like_cpp(&plan, &registry);

    assert_eq!(
        summary.send_failed, 1,
        "disconnected channel counted as send_failed"
    );
    assert_eq!(summary.candidates_queued, 0);
}

/// 4A.3c dormant rail: map/instance scoped creature visibility refresh.
///
/// C++ anchor: `Player::UpdateVisibilityOf` (Player.cpp:23138+) is the
/// seam that mutates `m_clientGUIDs` and emits CREATE/DESTROY. The global
/// runtime must wake matching sessions to run that seam rather than trying
/// to send raw CREATE bytes through HaveAtClient.
#[test]
fn refresh_visible_world_creatures_routes_by_map_instance_in_world_like_cpp() {
    let registry = PlayerRegistry::with_canonical_player_fixtures_like_cpp();

    let in_a = ObjectGuid::create_player(1, 50);
    let (in_a_info, in_a_rx) = make_registry_player_like_cpp(571, 7, Position::ZERO, true);
    registry.register_or_replace(in_a, in_a_info, Default::default());

    let in_b = ObjectGuid::create_player(1, 51);
    let (in_b_info, in_b_rx) =
        make_registry_player_like_cpp(571, 7, Position::new(9000.0, 0.0, 0.0, 0.0), true);
    registry.register_or_replace(in_b, in_b_info, Default::default());

    let wrong_map = ObjectGuid::create_player(1, 52);
    let (wrong_map_info, wrong_map_rx) =
        make_registry_player_like_cpp(530, 7, Position::ZERO, true);
    registry.register_or_replace(wrong_map, wrong_map_info, Default::default());
    let wrong_instance = ObjectGuid::create_player(1, 53);
    let (wrong_instance_info, wrong_instance_rx) =
        make_registry_player_like_cpp(571, 8, Position::ZERO, true);
    registry.register_or_replace(wrong_instance, wrong_instance_info, Default::default());

    let not_in_world = ObjectGuid::create_player(1, 54);
    let (not_in_world_info, not_in_world_rx) =
        make_registry_player_like_cpp(571, 7, Position::ZERO, false);
    registry.register_or_replace(not_in_world, not_in_world_info, Default::default());

    let summary = deliver_refresh_visible_world_creatures_like_cpp(571, 7, &registry);

    assert_eq!(summary.candidates_seen, 5);
    assert_eq!(summary.candidates_queued, 2);
    assert_eq!(summary.candidates_skipped_wrong_map, 1);
    assert_eq!(summary.candidates_skipped_wrong_instance, 1);
    assert_eq!(summary.candidates_skipped_not_in_world, 1);

    for command in [
        in_a_rx.try_recv().expect("same-map player A refresh"),
        in_b_rx.try_recv().expect("same-map player B refresh"),
    ] {
        let SessionCommand::RefreshVisibleWorldCreaturesLikeCpp(command) = command else {
            panic!("expected RefreshVisibleWorldCreaturesLikeCpp command");
        };
        assert_eq!(command.map_id, 571);
        assert_eq!(command.instance_id, 7);
    }
    assert!(wrong_map_rx.try_recv().is_err());
    assert!(wrong_instance_rx.try_recv().is_err());
    assert!(not_in_world_rx.try_recv().is_err());
}

/// Backpressure on the refresh rail must not block the runtime task.
#[test]
fn refresh_visible_world_creatures_full_channel_counts_send_failed_like_cpp() {
    let registry = PlayerRegistry::with_canonical_player_fixtures_like_cpp();
    let guid = ObjectGuid::create_player(1, 55);

    let (send_tx, _send_rx) = flume::bounded::<Vec<u8>>(1);
    let (command_tx, command_rx) = flume::bounded::<SessionCommand>(1);
    drop(command_rx);

    let mut info = player_registration_fixture_like_cpp(send_tx, command_tx, "RefreshFull");
    info.placement.map_id = 571;
    info.placement.instance_id = 7;
    info.placement.is_in_world = true;
    registry.register_or_replace(guid, info, Default::default());

    let summary = deliver_refresh_visible_world_creatures_like_cpp(571, 7, &registry);

    assert_eq!(summary.candidates_seen, 1);
    assert_eq!(summary.candidates_queued, 0);
    assert_eq!(summary.send_failed, 1);
}

#[test]
fn collect_legacy_creature_aggro_candidates_uses_living_in_world_players_like_cpp() {
    let registry = PlayerRegistry::default();
    let in_world = ObjectGuid::create_player(1, 64);
    let not_in_world = ObjectGuid::create_player(1, 65);
    let dead_in_world = ObjectGuid::create_player(1, 66);
    let (mut in_world_info, _) =
        make_registry_player_like_cpp(571, 2, Position::new(1.0, 2.0, 3.0, 0.0), true);
    let (not_in_world_info, _) =
        make_registry_player_like_cpp(571, 2, Position::new(9.0, 9.0, 9.0, 0.0), false);
    let (mut dead_in_world_info, _) =
        make_registry_player_like_cpp(571, 2, Position::new(4.0, 4.0, 4.0, 0.0), true);
    dead_in_world_info.placement.is_alive = false;
    registry.register_or_replace(in_world, in_world_info, Default::default());
    registry.register_or_replace(not_in_world, not_in_world_info, Default::default());
    registry.register_or_replace(dead_in_world, dead_in_world_info, Default::default());
    let canonical: wow_world::SharedCanonicalMapManager =
        Arc::new(Mutex::new(wow_map::MapManager::default()));
    canonical
        .lock()
        .unwrap()
        .create_map_entry(571, 2, 0, wow_map::ManagedMapKind::World);
    add_canonical_test_player_on_map_like_cpp(
        &canonical,
        in_world,
        Position::new(1.0, 2.0, 3.0, 0.0),
        571,
        2,
        100,
    );
    {
        let mut manager = canonical.lock().unwrap();
        let player = manager
            .find_map_mut(571, 2)
            .unwrap()
            .map_mut()
            .get_typed_player_mut(in_world)
            .unwrap();
        player.unit_mut().set_combat_reach(1.5);
        player.unit_mut().set_faction(1);
        player.gameplay_state_mut().liquid_status =
            wow_world::session::LIQUID_MAP_IN_WATER_LIKE_CPP;
        player.gameplay_state_mut().forced_reputation_ranks = vec![(87, 1)];
    }
    assert!(registry.bind_canonical_map_manager(canonical));

    let candidates = collect_legacy_creature_aggro_candidates_like_cpp(&registry);

    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0].player_guid, in_world);
    assert_eq!(candidates[0].map_id, 571);
    assert_eq!(candidates[0].instance_id, 2);
    assert_eq!(candidates[0].position, Position::new(1.0, 2.0, 3.0, 0.0));
    assert!(!candidates[0].player_visibility_represented);
    assert_eq!(candidates[0].player_combat_reach, 1.5);
    assert_eq!(
        candidates[0].player_liquid_status_like_cpp,
        wow_world::session::LIQUID_MAP_IN_WATER_LIKE_CPP
    );
    assert_eq!(candidates[0].player_level, 1);
    assert_eq!(candidates[0].player_gray_level, 0);
    assert_eq!(candidates[0].player_faction_template_id, 1);
    assert_eq!(
        candidates[0].player_forced_reputation_ranks,
        vec![(87, wow_data::reputation::ReputationRankLikeCpp::Hostile)]
    );
}

#[test]
fn collect_legacy_creature_aggro_candidates_hydrates_canonical_visibility_like_cpp() {
    let registry = PlayerRegistry::default();
    let player_guid = ObjectGuid::create_player(1, 68);
    let position = Position::new(1.0, 2.0, 3.0, 0.0);
    let (info, _) = make_registry_player_like_cpp(571, 2, position, true);
    registry.register_or_replace(player_guid, info, Default::default());

    let canonical: wow_world::SharedCanonicalMapManager =
        Arc::new(Mutex::new(wow_map::MapManager::default()));
    canonical
        .lock()
        .unwrap()
        .create_map_entry(571, 2, 2, wow_map::ManagedMapKind::World);
    add_canonical_test_player_on_map_like_cpp(&canonical, player_guid, position, 571, 2, 100);
    {
        let mut guard = canonical.lock().unwrap();
        let player = guard
            .find_map_mut(571, 2)
            .unwrap()
            .map_mut()
            .get_typed_player_mut(player_guid)
            .unwrap();
        *player.unit_mut().world_mut().phase_shift_mut() =
            wow_entities::PhaseShift::from_phases([77]);
        player.unit_mut().set_invisibility_like_cpp(0, 100);
        player
            .unit_mut()
            .subsystems_mut()
            .auras
            .register_applied_aura_modifier_like_cpp(
                wow_entities::AppliedAuraRef::new(91_136, player_guid, 0, 0x1),
                wow_data::spell::aura_types::SPELL_AURA_MOD_DETECTED_RANGE,
                6,
            );
        let school_immunity = wow_entities::AppliedAuraRef::new(91_137, player_guid, 1, 0x1);
        player
            .unit_mut()
            .subsystems_mut()
            .auras
            .register_applied_aura_effect_like_cpp(
                school_immunity,
                wow_data::spell::aura_types::SPELL_AURA_SCHOOL_IMMUNITY,
                99,
                0x1,
            );
        let confuse = wow_entities::AppliedAuraRef::new(91_138, player_guid, 2, 0x1);
        player
            .unit_mut()
            .subsystems_mut()
            .auras
            .register_applied_aura_type_like_cpp(
                confuse,
                wow_data::spell::aura_types::SPELL_AURA_MOD_CONFUSE,
            );
        let breakable_stun = wow_entities::AppliedAuraRef::new(91_139, player_guid, 3, 0x1);
        let auras = &mut player.unit_mut().subsystems_mut().auras;
        auras.register_applied_aura_type_like_cpp(
            breakable_stun,
            wow_data::spell::aura_types::SPELL_AURA_MOD_STUN,
        );
        auras.register_applied_aura(
            breakable_stun,
            None,
            wow_constants::SpellAuraInterruptFlags::DAMAGE.bits(),
            0,
        );
    }
    assert!(registry.bind_canonical_map_manager(Arc::clone(&canonical)));

    let candidates = collect_legacy_creature_aggro_candidates_with_canonical_like_cpp(
        &registry,
        Some(&canonical),
    );

    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0].map_difficulty_id, 2);
    assert!(candidates[0].player_visibility_represented);
    assert!(candidates[0].player_phase_shift.has_phase_like_cpp(77));
    assert_ne!(
        candidates[0].player_visibility_detection,
        wow_entities::UnitVisibilityDetectionStateLikeCpp::default()
    );
    assert_eq!(candidates[0].player_detected_range_aura_mod, 6.0);
    assert_eq!(candidates[0].player_school_immunity_mask, 0x1);
    assert!(candidates[0].player_has_confuse_aura);
    assert!(candidates[0].player_has_breakable_stun_aura);
}

/// The aggro scan's reputation/flag inputs come off the canonical player (#252).
///
/// These four values used to be copied into `PlayerBroadcastInfo` at registration
/// and refreshed on every registry sync. They are read in the collector's existing
/// canonical pass now, which takes the map lock once for the whole batch, so the
/// redirect adds no lock and no nesting.
///
/// C++ anchor: `Creature::CanCreatureAttack`/`Unit::IsValidAttackTarget` consult the
/// inspected `Player`'s own reputation state and unit flags, not a per-session copy.
#[test]
fn collect_legacy_creature_aggro_candidates_reads_reputation_and_flags_from_canonical_like_cpp() {
    let registry = PlayerRegistry::with_canonical_player_fixtures_like_cpp();
    let player_guid = ObjectGuid::create_player(1, 69);
    let position = Position::new(1.0, 2.0, 3.0, 0.0);
    let (info, _) = make_registry_player_like_cpp(571, 2, position, true);
    registry.register_or_replace(player_guid, info, Default::default());

    let canonical: wow_world::SharedCanonicalMapManager =
        Arc::new(Mutex::new(wow_map::MapManager::default()));
    canonical
        .lock()
        .unwrap()
        .create_map_entry(571, 2, 2, wow_map::ManagedMapKind::World);
    add_canonical_test_player_on_map_like_cpp(&canonical, player_guid, position, 571, 2, 100);
    {
        let mut guard = canonical.lock().unwrap();
        let player = guard
            .find_map_mut(571, 2)
            .unwrap()
            .map_mut()
            .get_typed_player_mut(player_guid)
            .unwrap();
        player
            .unit_mut()
            .set_unit_flags2_like_cpp(wow_constants::UnitFlags2::IGNORE_REPUTATION);
        player.set_player_flag(
            wow_world::canonical_player_access::PLAYER_FLAGS_CONTESTED_PVP_LIKE_CPP,
        );
        player
            .gameplay_state_mut()
            .reputations
            .push(wow_entities::PlayerReputationRecord {
                faction_id: 72,
                standing: -6000,
                flags: wow_entities::REPUTATION_FLAG_AT_WAR_LIKE_CPP,
            });
        player.set_forced_reputation_rank_like_cpp(87, true);
    }

    let candidates = collect_legacy_creature_aggro_candidates_with_canonical_like_cpp(
        &registry,
        Some(&canonical),
    );

    assert_eq!(candidates.len(), 1);
    assert_eq!(
        candidates[0].player_unit_flags2,
        wow_constants::UnitFlags2::IGNORE_REPUTATION.bits()
    );
    assert!(candidates[0].player_is_contested_pvp);
    assert_eq!(
        candidates[0].player_reputation_standings,
        vec![(72, -6_000)]
    );
    assert_eq!(
        candidates[0].player_reputation_state_flags,
        vec![(72, wow_entities::REPUTATION_FLAG_AT_WAR_LIKE_CPP)]
    );
    assert_eq!(candidates[0].player_forced_reputation_faction_ids, vec![87]);
}

/// Negative branch: a directory entry without its canonical owner is unknown and
/// cannot become an aggro candidate. The far-teleport window must not manufacture
/// zero/default gameplay values for a player that is temporarily on no map.
#[test]
fn collect_legacy_creature_aggro_candidates_skips_unknown_canonical_owner_like_cpp() {
    let registry = PlayerRegistry::default();
    let player_guid = ObjectGuid::create_player(1, 70);
    let position = Position::new(1.0, 2.0, 3.0, 0.0);
    let (info, _) = make_registry_player_like_cpp(571, 2, position, true);
    registry.register_or_replace(player_guid, info, Default::default());

    let candidates = collect_legacy_creature_aggro_candidates_like_cpp(&registry);

    assert!(candidates.is_empty());
}

#[test]
fn creature_attack_start_delivery_routes_only_to_victim_like_cpp() {
    let registry = PlayerRegistry::with_canonical_player_fixtures_like_cpp();
    let victim = ObjectGuid::create_player(1, 66);
    let other = ObjectGuid::create_player(1, 67);
    let attacker = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 9001, 90_060);
    let (victim_info, _victim_rx) = make_registry_player_like_cpp(571, 4, Position::ZERO, true);
    let (other_info, other_rx) = make_registry_player_like_cpp(571, 4, Position::ZERO, true);
    registry.register_or_replace(victim, victim_info, Default::default());
    registry.register_or_replace(other, other_info, Default::default());

    let commands = vec![
        wow_world::session::mailbox::CreatureAttackStartLikeCppCommand {
            attacker_guid: attacker,
            victim_guid: victim,
            previous_victim_guid: None,
            map_id: 571,
            instance_id: 4,
            packet_already_broadcast: false,
        },
    ];
    let summary = deliver_creature_attack_start_commands_like_cpp(&commands, &registry);

    assert_eq!(summary.commands_seen, 1);
    assert_eq!(summary.candidates_seen, 1);
    assert_eq!(summary.candidates_queued, 1);
    let SessionCommand::CreatureAttackStartLikeCpp(command) =
        drain_durable_creature_runtime_commands_like_cpp(&registry, victim)
            .pop()
            .expect("victim receives attack-start")
    else {
        panic!("expected CreatureAttackStartLikeCpp command");
    };
    assert_eq!(command.attacker_guid, attacker);
    assert_eq!(command.victim_guid, victim);
    assert!(
        other_rx.try_recv().is_err(),
        "non-victim session is untouched"
    );
}

#[test]
fn creature_assistance_start_establishes_canonical_combat_for_both_creatures_like_cpp() {
    let canonical: wow_world::SharedCanonicalMapManager =
        Arc::new(Mutex::new(wow_map::MapManager::default()));
    let attacker = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 9001, 90_070);
    let victim = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 9002, 90_071);
    add_canonical_test_creature_on_map_like_cpp(&canonical, attacker, Position::ZERO, 571, 4, 100);
    add_canonical_test_creature_on_map_like_cpp(&canonical, victim, Position::ZERO, 571, 4, 100);
    let commands = [
        wow_world::session::mailbox::CreatureAttackStartLikeCppCommand {
            attacker_guid: attacker,
            victim_guid: victim,
            previous_victim_guid: None,
            map_id: 571,
            instance_id: 4,
            packet_already_broadcast: true,
        },
    ];

    assert_eq!(
        apply_canonical_creature_attack_starts_like_cpp(&commands, Some(&canonical)),
        1
    );
    let guard = canonical.lock().unwrap();
    let map = guard.find_map(571, 4).unwrap().map();
    let attacker_unit = map.get_typed_creature(attacker).unwrap().unit();
    let victim_unit = map.get_typed_creature(victim).unwrap().unit();
    assert!(attacker_unit.subsystems().combat.is_in_combat_with(victim));
    assert!(victim_unit.subsystems().combat.is_in_combat_with(attacker));
    assert_eq!(
        attacker_unit.subsystems().combat.threat_value(victim),
        Some(0.0),
        "C++ EngageWithTarget creates the assistant's zero-threat forward reference"
    );
    assert!(
        victim_unit
            .subsystems()
            .combat
            .threatened_by_me_owner_guids()
            .contains(&attacker),
        "the victim must carry the reciprocal reference used by helpful-threat fanout"
    );
}

#[test]
fn creature_assistance_stop_purges_canonical_combat_for_both_creatures_like_cpp() {
    let canonical: wow_world::SharedCanonicalMapManager =
        Arc::new(Mutex::new(wow_map::MapManager::default()));
    let attacker = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 9001, 90_072);
    let victim = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 9002, 90_073);
    add_canonical_test_creature_on_map_like_cpp(&canonical, attacker, Position::ZERO, 571, 4, 100);
    add_canonical_test_creature_on_map_like_cpp(&canonical, victim, Position::ZERO, 571, 4, 100);
    let starts = [
        wow_world::session::mailbox::CreatureAttackStartLikeCppCommand {
            attacker_guid: attacker,
            victim_guid: victim,
            previous_victim_guid: None,
            map_id: 571,
            instance_id: 4,
            packet_already_broadcast: true,
        },
    ];
    let stops = [
        wow_world::session::mailbox::CreatureAttackStopLikeCppCommand {
            attacker_guid: attacker,
            victim_guid: victim,
            map_id: 571,
            instance_id: 4,
        },
    ];

    assert_eq!(
        apply_canonical_creature_attack_starts_like_cpp(&starts, Some(&canonical)),
        1
    );
    assert_eq!(
        apply_canonical_creature_attack_stops_like_cpp(&stops, Some(&canonical)),
        1
    );
    let guard = canonical.lock().unwrap();
    let map = guard.find_map(571, 4).unwrap().map();
    let attacker_unit = map.get_typed_creature(attacker).unwrap().unit();
    let victim_unit = map.get_typed_creature(victim).unwrap().unit();
    assert!(!attacker_unit.subsystems().combat.is_in_combat_with(victim));
    assert!(!victim_unit.subsystems().combat.is_in_combat_with(attacker));
    assert!(
        !victim_unit
            .subsystems()
            .combat
            .attackers
            .contains(&attacker)
    );
}

#[test]
fn creature_attack_start_delivery_filters_registry_state_like_cpp() {
    let registry = PlayerRegistry::with_canonical_player_fixtures_like_cpp();
    let attacker = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 9001, 90_061);
    let wrong_map = ObjectGuid::create_player(1, 68);
    let wrong_instance = ObjectGuid::create_player(1, 69);
    let not_in_world = ObjectGuid::create_player(1, 70);
    let missing = ObjectGuid::create_player(1, 71);
    let dead = ObjectGuid::create_player(1, 72);
    let (wrong_map_info, wrong_map_rx) =
        make_registry_player_like_cpp(530, 0, Position::ZERO, true);
    let (wrong_instance_info, wrong_instance_rx) =
        make_registry_player_like_cpp(571, 9, Position::ZERO, true);
    let (not_in_world_info, not_in_world_rx) =
        make_registry_player_like_cpp(571, 0, Position::ZERO, false);
    let (mut dead_info, dead_rx) = make_registry_player_like_cpp(571, 0, Position::ZERO, true);
    dead_info.placement.is_alive = false;
    registry.register_or_replace(wrong_map, wrong_map_info, Default::default());
    registry.register_or_replace(wrong_instance, wrong_instance_info, Default::default());
    registry.register_or_replace(not_in_world, not_in_world_info, Default::default());
    registry.register_or_replace(dead, dead_info, Default::default());

    let make_command =
        |victim_guid| wow_world::session::mailbox::CreatureAttackStartLikeCppCommand {
            attacker_guid: attacker,
            victim_guid,
            previous_victim_guid: None,
            map_id: 571,
            instance_id: 0,
            packet_already_broadcast: false,
        };
    let commands = vec![
        make_command(wrong_map),
        make_command(wrong_instance),
        make_command(not_in_world),
        make_command(missing),
        make_command(dead),
    ];
    let summary = deliver_creature_attack_start_commands_like_cpp(&commands, &registry);

    assert_eq!(summary.commands_seen, 5);
    assert_eq!(summary.candidates_seen, 4);
    assert_eq!(summary.candidates_queued, 0);
    assert_eq!(summary.candidates_skipped_wrong_map, 1);
    assert_eq!(summary.candidates_skipped_wrong_instance, 1);
    assert_eq!(summary.candidates_skipped_not_in_world, 1);
    assert_eq!(summary.candidates_skipped_dead, 1);
    assert_eq!(summary.candidates_skipped_missing_victim, 1);
    assert!(wrong_map_rx.try_recv().is_err());
    assert!(wrong_instance_rx.try_recv().is_err());
    assert!(not_in_world_rx.try_recv().is_err());
    assert!(dead_rx.try_recv().is_err());
}

#[test]
fn creature_attack_start_delivery_uses_durable_rail_when_general_queue_is_full_like_cpp() {
    let registry = PlayerRegistry::with_canonical_player_fixtures_like_cpp();
    let victim = ObjectGuid::create_player(1, 73);
    let attacker = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 9001, 90_062);
    let (send_tx, _send_rx) = flume::bounded::<Vec<u8>>(1);
    let (command_tx, command_rx) = flume::bounded::<SessionCommand>(1);
    let mut info = player_registration_fixture_like_cpp(send_tx, command_tx.clone(), "AggroFull");
    info.placement.map_id = 571;
    info.placement.instance_id = 0;
    info.placement.is_in_world = true;
    info.placement.is_alive = true;
    registry.register_or_replace(victim, info, Default::default());
    let command = wow_world::session::mailbox::CreatureAttackStartLikeCppCommand {
        attacker_guid: attacker,
        victim_guid: victim,
        previous_victim_guid: None,
        map_id: 571,
        instance_id: 0,
        packet_already_broadcast: false,
    };
    command_tx
        .send(SessionCommand::CreatureAttackStartLikeCpp(command.clone()))
        .unwrap();

    let summary = deliver_creature_attack_start_commands_like_cpp(&[command], &registry);
    assert_eq!(summary.candidates_queued, 1);
    assert_eq!(summary.send_failed, 0);
    assert_eq!(command_rx.len(), 1, "bounded general queue remains full");
    assert!(matches!(
        drain_durable_creature_runtime_commands_like_cpp(&registry, victim)
            .pop()
            .unwrap(),
        SessionCommand::CreatureAttackStartLikeCpp(_)
    ));
}

/// 4C.4 bridge coverage: the combined global runtime body can perform the
/// C++ `CreatureAI::MoveInLineOfSight`-style aggro transition once from the
/// map owner and deliver both the authoritative session transition and its
/// visible attack-start packet to the victim session in FIFO order.
#[test]
fn legacy_creature_runtime_bridge_delivers_aggro_start_like_cpp() {
    let legacy: wow_world::SharedMapManager =
        Arc::new(std::sync::RwLock::new(wow_world::MapManager::new()));
    let canonical: wow_world::SharedCanonicalMapManager =
        Arc::new(Mutex::new(wow_map::MapManager::default()));
    canonical.lock().unwrap().create_world_map(0, 0);

    let victim = ObjectGuid::create_player(1, 93_001);
    let victim_position = Position::new(10.5, 10.5, 0.0, 0.0);
    add_canonical_test_player_on_map_like_cpp(&canonical, victim, victim_position, 0, 0, 100);

    let attacker = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 9001, 93_002);
    let attacker_position = Position::new(10.0, 10.0, 0.0, 0.0);
    let mut creature = wow_world::map_manager::WorldCreature::new(
        attacker,
        9001,
        attacker_position,
        25,
        2,
        3,
        5,
        5.0,
        100,
        14,
        0,
        0,
    );
    {
        let ai = creature.creature.ai_ownership_mut();
        ai.wander_delay_ms = u64::MAX;
        ai.swing_timer_ms = u64::MAX;
    }

    {
        let mut manager = legacy.write().unwrap();
        manager.add_creature(
            0,
            0,
            wow_world::map_manager::world_to_grid_x(attacker_position.x),
            wow_world::map_manager::world_to_grid_y(attacker_position.y),
            creature,
        );
        manager.set_tick_owner(wow_world::map_manager::RuntimeTickOwner::GlobalLegacy);
    }

    let registry = PlayerRegistry::default();
    assert!(registry.bind_canonical_map_manager(Arc::clone(&canonical)));
    let (mut victim_info, victim_rx) = make_registry_player_like_cpp(0, 0, victim_position, true);
    registry.register_or_replace(victim, victim_info, Default::default());
    let wrong_map = ObjectGuid::create_player(1, 93_003);
    let (wrong_map_info, wrong_map_rx) = make_registry_player_like_cpp(1, 0, victim_position, true);
    registry.register_or_replace(wrong_map, wrong_map_info, Default::default());
    add_canonical_test_player_on_map_like_cpp(&canonical, wrong_map, victim_position, 1, 0, 100);
    let mmap_config = wow_world::MMapRuntimeConfigLikeCpp {
        enabled: false,
        ..Default::default()
    };
    let aggro_config = wow_world::session::LegacyCreatureAggroConfigLikeCpp {
        faction_template_store: Some(Arc::new(
            wow_data::progression_rewards::FactionTemplateStore::from_entries([
                wow_data::progression_rewards::FactionTemplateEntry {
                    id: 14,
                    faction: 72,
                    flags: 0,
                    faction_group: 0,
                    friend_group: 0,
                    enemy_group: 0,
                    enemies: [930, 0, 0, 0, 0, 0, 0, 0],
                    friend: [0; 8],
                },
                wow_data::progression_rewards::FactionTemplateEntry {
                    id: 1,
                    faction: 930,
                    flags: 0,
                    faction_group: 0,
                    friend_group: 0,
                    enemy_group: 0,
                    enemies: [0; 8],
                    friend: [0; 8],
                },
            ]),
        )),
        faction_store: Some(Arc::new(
            wow_data::progression_rewards::FactionStore::from_entries([
                wow_data::progression_rewards::FactionEntry::for_test_like_cpp(72, 1),
            ]),
        )),
        ..Default::default()
    };
    let candidates = collect_legacy_creature_aggro_candidates_like_cpp(&registry);
    assert_eq!(candidates.len(), 2, "{candidates:?}");
    let outcome = run_legacy_creature_runtime_tick_and_deliver_once_like_cpp(
        &legacy,
        Some(&canonical),
        &legacy_runtime_world_map_store_like_cpp(),
        &mmap_config,
        None,
        aggro_config,
        10,
        std::time::Instant::now(),
        &registry,
        None,
        None,
        None,
        &Arc::new(Mutex::new(Default::default())),
    );

    assert!(!outcome.aggro.skipped_owner_not_global);
    assert_eq!(outcome.aggro.maps_seen, 1);
    assert_eq!(outcome.aggro.creatures_seen, 1);
    assert_eq!(outcome.aggro.candidates_seen, 1);
    assert_eq!(outcome.aggro.aggro_starts, 1);
    assert_eq!(outcome.aggro.commands.len(), 1);
    assert_eq!(outcome.aggro_delivery.commands_seen, 1);
    assert_eq!(outcome.aggro_delivery.candidates_seen, 1);
    assert_eq!(outcome.aggro_delivery.candidates_queued, 1);
    assert_eq!(outcome.aggro_delivery.candidates_skipped_wrong_map, 0);
    assert_eq!(outcome.aggro_plan_delivery.events_seen, 1);
    assert_eq!(outcome.aggro_plan_delivery.candidates_seen, 2);
    assert_eq!(outcome.aggro_plan_delivery.candidates_queued, 1);
    assert_eq!(outcome.aggro_plan_delivery.candidates_skipped_distance, 1);
    assert_eq!(outcome.movement.movement_packets, 0);
    assert_eq!(outcome.melee.swings_ready, 0);

    let commands = drain_durable_creature_runtime_commands_like_cpp(&registry, victim);
    let [
        SessionCommand::CreatureAttackStartLikeCpp(command),
        SessionCommand::SendIfVisibleLikeCpp(visual),
    ] = commands.as_slice()
    else {
        panic!("expected authoritative then visual attack-start commands: {commands:?}");
    };
    assert_eq!(command.attacker_guid, attacker);
    assert_eq!(command.victim_guid, victim);
    assert_eq!(command.map_id, 0);
    assert_eq!(command.instance_id, 0);
    assert_eq!(visual.source_guid, attacker);
    assert_eq!(visual.map_id, 0);
    assert_eq!(visual.instance_id, 0);
    assert_eq!(
        u16::from_le_bytes([visual.packet_bytes[0], visual.packet_bytes[1]]),
        wow_constants::ServerOpcodes::AttackStart as u16
    );
    assert!(victim_rx.try_recv().is_err());
    assert!(wrong_map_rx.try_recv().is_err());

    let combat_target = {
        let guard = legacy.read().unwrap();
        guard
            .find_creature(0, 0, attacker)
            .unwrap()
            .creature
            .ai_ownership()
            .combat_target
    };
    assert_eq!(combat_target, Some(victim));
}

/// 4C.3 dormant rail: map-owned creature melee results route to exactly
/// the victim session. C++ anchor: `Unit::AttackerStateUpdate` resolves a
/// single melee hit for one victim, then `Unit::DealDamage` mutates health.
#[test]
fn creature_melee_damage_delivery_routes_only_to_victim_like_cpp() {
    let registry = PlayerRegistry::with_canonical_player_fixtures_like_cpp();
    let victim = ObjectGuid::create_player(1, 56);
    let other = ObjectGuid::create_player(1, 57);
    let attacker = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 9001, 90_056);
    let (victim_info, _victim_rx) = make_registry_player_like_cpp(571, 3, Position::ZERO, true);
    let (other_info, other_rx) = make_registry_player_like_cpp(571, 3, Position::ZERO, true);
    registry.register_or_replace(victim, victim_info, Default::default());
    registry.register_or_replace(other, other_info, Default::default());

    let commands = vec![
        wow_world::session::mailbox::ApplyCreatureMeleeDamageLikeCppCommand {
            attacker_guid: attacker,
            victim_guid: victim,
            map_id: 571,
            instance_id: 3,
            damage: 17,
            over_damage: -1,
            target_level: 80,
            victim_health_after: 83,
            victim_health_state_revision_after: 7,
        },
    ];
    let summary = deliver_creature_melee_damage_commands_like_cpp(&commands, &registry);

    assert_eq!(summary.commands_seen, 1);
    assert_eq!(summary.candidates_seen, 1);
    assert_eq!(summary.candidates_queued, 1);
    let SessionCommand::ApplyCreatureMeleeDamageLikeCpp(command) =
        drain_durable_creature_runtime_commands_like_cpp(&registry, victim)
            .pop()
            .expect("victim receives melee command")
    else {
        panic!("expected ApplyCreatureMeleeDamageLikeCpp command");
    };
    assert_eq!(command.attacker_guid, attacker);
    assert_eq!(command.victim_guid, victim);
    assert_eq!(command.victim_health_after, 83);
    assert_eq!(command.victim_health_state_revision_after, 7);
    assert!(
        other_rx.try_recv().is_err(),
        "non-victim session is untouched"
    );
}

#[test]
fn creature_melee_damage_delivery_filters_registry_state_like_cpp() {
    let registry = PlayerRegistry::with_canonical_player_fixtures_like_cpp();
    let attacker = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 9001, 90_057);
    let wrong_map = ObjectGuid::create_player(1, 58);
    let wrong_instance = ObjectGuid::create_player(1, 59);
    let not_in_world = ObjectGuid::create_player(1, 60);
    let missing = ObjectGuid::create_player(1, 61);
    let (wrong_map_info, wrong_map_rx) =
        make_registry_player_like_cpp(530, 0, Position::ZERO, true);
    let (wrong_instance_info, wrong_instance_rx) =
        make_registry_player_like_cpp(571, 9, Position::ZERO, true);
    let (not_in_world_info, not_in_world_rx) =
        make_registry_player_like_cpp(571, 0, Position::ZERO, false);
    registry.register_or_replace(wrong_map, wrong_map_info, Default::default());
    registry.register_or_replace(wrong_instance, wrong_instance_info, Default::default());
    registry.register_or_replace(not_in_world, not_in_world_info, Default::default());

    let make_command =
        |victim_guid| wow_world::session::mailbox::ApplyCreatureMeleeDamageLikeCppCommand {
            attacker_guid: attacker,
            victim_guid,
            map_id: 571,
            instance_id: 0,
            damage: 5,
            over_damage: -1,
            target_level: 80,
            victim_health_after: 95,
            victim_health_state_revision_after: 1,
        };
    let commands = vec![
        make_command(wrong_map),
        make_command(wrong_instance),
        make_command(not_in_world),
        make_command(missing),
    ];
    let summary = deliver_creature_melee_damage_commands_like_cpp(&commands, &registry);

    assert_eq!(summary.commands_seen, 4);
    assert_eq!(summary.candidates_seen, 3);
    assert_eq!(summary.candidates_queued, 0);
    assert_eq!(summary.candidates_skipped_wrong_map, 1);
    assert_eq!(summary.candidates_skipped_wrong_instance, 1);
    assert_eq!(summary.candidates_skipped_not_in_world, 1);
    assert_eq!(summary.candidates_skipped_missing_victim, 1);
    assert!(wrong_map_rx.try_recv().is_err());
    assert!(wrong_instance_rx.try_recv().is_err());
    assert!(not_in_world_rx.try_recv().is_err());
}

#[test]
fn creature_melee_damage_delivery_poisoned_durable_rail_counts_send_failed_like_cpp() {
    let registry = PlayerRegistry::with_canonical_player_fixtures_like_cpp();
    let victim = ObjectGuid::create_player(1, 62);
    let attacker = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 9001, 90_058);
    let (send_tx, _send_rx) = flume::bounded::<Vec<u8>>(1);
    let (command_tx, _command_rx) = flume::bounded::<SessionCommand>(1);
    let mut info = player_registration_fixture_like_cpp(send_tx, command_tx, "MeleeFull");
    info.placement.map_id = 571;
    info.placement.instance_id = 0;
    info.placement.is_in_world = true;
    let durable = Arc::clone(&info.durable_creature_runtime_commands_like_cpp);
    let _ = std::thread::spawn(move || {
        let _guard = durable.lock().unwrap();
        panic!("poison durable rail for delivery failure coverage");
    })
    .join();
    registry.register_or_replace(victim, info, Default::default());

    let commands = vec![
        wow_world::session::mailbox::ApplyCreatureMeleeDamageLikeCppCommand {
            attacker_guid: attacker,
            victim_guid: victim,
            map_id: 571,
            instance_id: 0,
            damage: 5,
            over_damage: -1,
            target_level: 80,
            victim_health_after: 95,
            victim_health_state_revision_after: 1,
        },
    ];
    let summary = deliver_creature_melee_damage_commands_like_cpp(&commands, &registry);

    assert_eq!(summary.commands_seen, 1);
    assert_eq!(summary.candidates_seen, 1);
    assert_eq!(summary.candidates_queued, 0);
    assert_eq!(summary.send_failed, 1);
}

#[test]
fn creature_melee_damage_delivery_preserves_every_swing_when_general_queue_is_full_like_cpp() {
    let registry = PlayerRegistry::with_canonical_player_fixtures_like_cpp();
    let victim = ObjectGuid::create_player(1, 64);
    let attacker = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 9001, 90_063);
    let (send_tx, _send_rx) = flume::bounded::<Vec<u8>>(1);
    let (command_tx, command_rx) = flume::bounded::<SessionCommand>(1);
    let mut info = player_registration_fixture_like_cpp(send_tx, command_tx.clone(), "MeleeRetry");
    info.placement.map_id = 571;
    info.placement.instance_id = 0;
    info.placement.is_in_world = true;
    registry.register_or_replace(victim, info, Default::default());
    let command = wow_world::session::mailbox::ApplyCreatureMeleeDamageLikeCppCommand {
        attacker_guid: attacker,
        victim_guid: victim,
        map_id: 571,
        instance_id: 0,
        damage: 5,
        over_damage: -1,
        target_level: 80,
        victim_health_after: 95,
        victim_health_state_revision_after: 7,
    };
    command_tx
        .send(SessionCommand::ApplyCreatureMeleeDamageLikeCpp(
            command.clone(),
        ))
        .unwrap();

    let mut latest = command.clone();
    latest.victim_health_after = 90;
    latest.victim_health_state_revision_after = 8;
    let summary = deliver_creature_melee_damage_commands_like_cpp(&[command, latest], &registry);
    assert_eq!(summary.candidates_queued, 2);
    assert_eq!(summary.send_failed, 0);
    assert_eq!(command_rx.len(), 1, "bounded general queue remains full");
    let commands = drain_durable_creature_runtime_commands_like_cpp(&registry, victim);
    assert_eq!(
        commands.len(),
        2,
        "every committed swing remains observable"
    );
    let SessionCommand::ApplyCreatureMeleeDamageLikeCpp(first) = &commands[0] else {
        panic!("expected first durable melee command");
    };
    let SessionCommand::ApplyCreatureMeleeDamageLikeCpp(second) = &commands[1] else {
        panic!("expected second durable melee command");
    };
    assert_eq!(first.victim_health_after, 95);
    assert_eq!(first.victim_health_state_revision_after, 7);
    assert_eq!(second.victim_health_after, 90);
    assert_eq!(second.victim_health_state_revision_after, 8);
}

/// 4C.3 compatibility bridge: canonical health is applied once and the
/// final-health command is delivered outside all map locks. The outcome is
/// still marked unrepresented until full `CalculateMeleeDamage` exists.
#[test]
fn legacy_creature_melee_tick_delivers_compatibility_victim_command_like_cpp() {
    let legacy: wow_world::SharedMapManager =
        Arc::new(std::sync::RwLock::new(wow_world::MapManager::new()));
    let canonical: wow_world::SharedCanonicalMapManager =
        Arc::new(Mutex::new(wow_map::MapManager::default()));
    canonical.lock().unwrap().create_world_map(0, 0);

    let victim = ObjectGuid::create_player(1, 63);
    let attacker = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 9001, 90_059);
    let attacker_position = Position::new(5.0, 5.0, 0.0, 0.0);
    add_canonical_test_player_on_map_like_cpp(&canonical, victim, attacker_position, 0, 0, 100);
    add_canonical_test_creature_on_map_like_cpp(&canonical, attacker, attacker_position, 0, 0, 25);
    let mut world_creature =
        mirror_canonical_melee_test_creature_like_cpp(&canonical, attacker, 0, 0);
    world_creature.enter_combat(victim);
    world_creature.creature.ai_ownership_mut().swing_timer_ms = 0;

    {
        let mut manager = legacy.write().unwrap();
        manager.add_creature(
            0,
            0,
            wow_world::map_manager::world_to_grid_x(attacker_position.x),
            wow_world::map_manager::world_to_grid_y(attacker_position.y),
            world_creature,
        );
        manager.set_tick_owner(wow_world::map_manager::RuntimeTickOwner::GlobalLegacy);
    }

    let registry = PlayerRegistry::with_canonical_player_fixtures_like_cpp();
    let (victim_info, _victim_rx) = make_registry_player_like_cpp(0, 0, attacker_position, true);
    registry.register_or_replace(victim, victim_info, Default::default());

    let (outcome, delivery, plan_delivery) =
        run_legacy_creature_melee_tick_and_deliver_once_like_cpp(
            &legacy,
            Some(&canonical),
            &registry,
        );

    assert!(!outcome.skipped_owner_not_global);
    assert_eq!(outcome.maps_seen, 1);
    assert_eq!(outcome.creatures_seen, 1);
    assert_eq!(outcome.swings_ready, 1);
    assert_eq!(outcome.melee_outcomes_unrepresented, 1);
    assert_eq!(outcome.runtime_rng_authority_rejections, 0);
    assert_eq!(outcome.canonical_hits, 1);
    assert_eq!(outcome.commands.len(), 1);
    assert_eq!(delivery.commands_seen, 1);
    assert_eq!(delivery.candidates_seen, 1);
    assert_eq!(delivery.candidates_queued, 1);
    assert_eq!(plan_delivery.events_seen, 0);
    let command = match drain_durable_creature_runtime_commands_like_cpp(&registry, victim)
        .pop()
        .expect("victim session receives final-health melee command")
    {
        SessionCommand::ApplyCreatureMeleeDamageLikeCpp(command) => command,
        other => panic!("expected ApplyCreatureMeleeDamageLikeCpp, got {other:?}"),
    };
    assert_eq!(command.attacker_guid, attacker);
    assert_eq!(command.victim_guid, victim);
    assert!((3..=5).contains(&command.damage));

    let canonical_health = canonical
        .lock()
        .unwrap()
        .find_map(0, 0)
        .unwrap()
        .map()
        .get_typed_player(victim)
        .unwrap()
        .unit()
        .data()
        .health;
    assert_eq!(canonical_health, command.victim_health_after);
    assert_eq!(canonical_health, 100 - u64::from(command.damage));
}

#[test]
fn legacy_creature_melee_tick_delivers_compatibility_creature_plan_like_cpp() {
    let legacy: wow_world::SharedMapManager =
        Arc::new(std::sync::RwLock::new(wow_world::MapManager::new()));
    let canonical: wow_world::SharedCanonicalMapManager =
        Arc::new(Mutex::new(wow_map::MapManager::default()));
    canonical.lock().unwrap().create_world_map(0, 0);

    let victim = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 9002, 90_060);
    let attacker = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 9001, 90_061);
    let position = Position::new(5.0, 5.0, 0.0, 0.0);
    add_canonical_test_creature_on_map_like_cpp(&canonical, victim, position, 0, 0, 100);
    add_canonical_test_creature_on_map_like_cpp(&canonical, attacker, position, 0, 0, 25);
    let mut world_creature =
        mirror_canonical_melee_test_creature_like_cpp(&canonical, attacker, 0, 0);
    world_creature.enter_combat(victim);
    world_creature.creature.ai_ownership_mut().swing_timer_ms = 0;

    {
        let mut manager = legacy.write().unwrap();
        manager.add_creature(
            0,
            0,
            wow_world::map_manager::world_to_grid_x(position.x),
            wow_world::map_manager::world_to_grid_y(position.y),
            world_creature,
        );
        manager.set_tick_owner(wow_world::map_manager::RuntimeTickOwner::GlobalLegacy);
    }

    let registry = PlayerRegistry::with_canonical_player_fixtures_like_cpp();
    let viewer = ObjectGuid::create_player(1, 91_030);
    let (viewer_info, viewer_rx) = make_registry_player_like_cpp(0, 0, position, true);
    registry.register_or_replace(viewer, viewer_info, Default::default());

    let (outcome, delivery, plan_delivery) =
        run_legacy_creature_melee_tick_and_deliver_once_like_cpp(
            &legacy,
            Some(&canonical),
            &registry,
        );

    assert_eq!(outcome.swings_ready, 1);
    assert_eq!(outcome.melee_outcomes_unrepresented, 1);
    assert_eq!(outcome.runtime_rng_authority_rejections, 0);
    assert_eq!(outcome.canonical_hits, 1);
    assert_eq!(outcome.canonical_creature_hits, 1);
    assert!(outcome.commands.is_empty());
    assert_eq!(delivery.commands_seen, 0);
    assert_eq!(plan_delivery.events_seen, 2);
    assert_eq!(plan_delivery.candidates_queued, 2);
    for _ in 0..2 {
        let SessionCommand::SendIfVisibleLikeCpp(command) = viewer_rx
            .try_recv()
            .expect("viewer receives creature-victim melee fanout")
        else {
            panic!("expected SendIfVisibleLikeCpp");
        };
        assert!(command.source_guid == attacker || command.source_guid == victim);
        assert!(!command.packet_bytes.is_empty());
    }
}

/// 4A.3c bridge: lifecycle changes happen once under the global owner, then
/// matching sessions are woken to run their own visibility pass.
#[test]
fn legacy_creature_lifecycle_tick_refreshes_sessions_after_ready_respawn_like_cpp() {
    let legacy: wow_world::SharedMapManager =
        Arc::new(std::sync::RwLock::new(wow_world::MapManager::new()));
    let canonical: wow_world::SharedCanonicalMapManager =
        Arc::new(Mutex::new(wow_map::MapManager::default()));
    canonical.lock().unwrap().create_world_map(0, 0);
    legacy
        .write()
        .unwrap()
        .set_tick_owner(wow_world::map_manager::RuntimeTickOwner::GlobalLegacy);

    let now = std::time::Instant::now();
    let creature_guid =
        ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 9001, 90_012);
    let mut world_creature = wow_world::map_manager::WorldCreature::new(
        creature_guid,
        9001,
        Position::new(20.0, 20.0, 0.0, 0.0),
        30,
        4,
        5,
        9,
        20.0,
        100,
        14,
        0,
        0,
    );
    world_creature
        .creature
        .unit_mut()
        .world_mut()
        .phase_shift_mut()
        .add_phase_like_cpp(77, wow_constants::PhaseFlags::empty(), 1);
    let pending = wow_world::map_manager::pending_respawn_from_world_creature_like_cpp(
        &world_creature,
        now - std::time::Duration::from_secs(1),
        0,
    );
    legacy.write().unwrap().push_respawn(0, 0, pending);

    let registry = PlayerRegistry::with_canonical_player_fixtures_like_cpp();
    let same_a = ObjectGuid::create_player(1, 91_001);
    let (same_a_info, same_a_rx) = make_registry_player_like_cpp(0, 0, Position::ZERO, true);
    registry.register_or_replace(same_a, same_a_info, Default::default());
    let same_b = ObjectGuid::create_player(1, 91_002);
    let (same_b_info, same_b_rx) =
        make_registry_player_like_cpp(0, 0, Position::new(9000.0, 0.0, 0.0, 0.0), true);
    registry.register_or_replace(same_b, same_b_info, Default::default());
    let wrong_map = ObjectGuid::create_player(1, 91_003);
    let (wrong_map_info, wrong_map_rx) = make_registry_player_like_cpp(1, 0, Position::ZERO, true);
    registry.register_or_replace(wrong_map, wrong_map_info, Default::default());

    let (outcome, delivery) = run_legacy_creature_lifecycle_tick_and_refresh_once_like_cpp(
        &legacy,
        Some(&canonical),
        &legacy_runtime_world_map_store_like_cpp(),
        now,
        &registry,
    );

    assert!(!outcome.skipped_owner_not_global);
    assert_eq!(outcome.respawns_processed, 1);
    assert_eq!(outcome.canonical_inserts, 1);
    assert_eq!(outcome.refresh_map_keys, vec![(0, 0)]);
    assert_eq!(delivery.candidates_seen, 3);
    assert_eq!(delivery.candidates_queued, 2);
    assert_eq!(delivery.candidates_skipped_wrong_map, 1);

    for command in [
        same_a_rx.try_recv().expect("same-map session A refresh"),
        same_b_rx.try_recv().expect("same-map session B refresh"),
    ] {
        let SessionCommand::RefreshVisibleWorldCreaturesLikeCpp(command) = command else {
            panic!("expected RefreshVisibleWorldCreaturesLikeCpp command");
        };
        assert_eq!(command.map_id, 0);
        assert_eq!(command.instance_id, 0);
    }
    assert!(wrong_map_rx.try_recv().is_err());

    let canonical_guard = canonical.lock().unwrap();
    let typed = canonical_guard
        .find_map(0, 0)
        .unwrap()
        .map()
        .get_typed_creature(creature_guid)
        .expect("lifecycle bridge must sync canonical respawn");
    assert!(typed.unit().world().phase_shift().has_phase_like_cpp(77));
}

/// Slice 4A.4 test-only bridge: one gated global movement tick runs from a
/// spawned task, produces a `RuntimePlan`, syncs canonical state outside
/// the legacy lock, then delivers `SendIfVisibleLikeCpp` commands to
/// candidate sessions.
///
/// No production loop calls this yet; this only proves the cross-crate
/// task/ownership path with `GlobalLegacy` flipped only in the test.
#[tokio::test]
async fn legacy_creature_global_tick_task_delivers_movement_plan_like_cpp() {
    let legacy: wow_world::SharedMapManager =
        Arc::new(std::sync::RwLock::new(wow_world::MapManager::new()));
    let canonical: wow_world::SharedCanonicalMapManager =
        Arc::new(Mutex::new(wow_map::MapManager::default()));
    canonical.lock().unwrap().create_world_map(0, 0);

    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 9001, 90_009);
    let position = Position::new(10.0, 10.0, 0.0, 0.0);
    let mut world_creature = wow_world::map_manager::WorldCreature::new(
        guid, 9001, position, 25, 2, 3, 5, 20.0, 100, 14, 0, 0,
    );
    {
        let ai = world_creature.creature.ai_ownership_mut();
        ai.wander_delay_ms = 0;
        ai.move_start_ms = 0;
        ai.wander_radius = 3.0;
    }
    world_creature
        .creature
        .set_default_movement_type_runtime_like_cpp(wow_entities::MovementGeneratorType::Random);
    world_creature.seed_runtime_rng_like_cpp(0x9009);

    let mut canonical_creature = world_creature.creature.clone();
    canonical_creature
        .unit_mut()
        .world_mut()
        .set_map(0, 0)
        .unwrap();
    canonical_creature
        .unit_mut()
        .world_mut()
        .object_mut()
        .add_to_world();
    canonical
        .lock()
        .unwrap()
        .find_map_mut(0, 0)
        .unwrap()
        .map_mut()
        .insert_map_object_record(MapObjectRecord::new_creature(canonical_creature).unwrap())
        .unwrap();

    {
        let mut manager = legacy.write().unwrap();
        manager.add_creature(
            0,
            0,
            wow_world::map_manager::world_to_grid_x(position.x),
            wow_world::map_manager::world_to_grid_y(position.y),
            world_creature,
        );
        manager.set_tick_owner(wow_world::map_manager::RuntimeTickOwner::GlobalLegacy);
    }

    let registry = Arc::new(PlayerRegistry::with_canonical_player_fixtures_like_cpp());
    let near_a = ObjectGuid::create_player(1, 90_001);
    let (near_a_info, near_a_rx) =
        make_registry_player_like_cpp(0, 0, Position::new(11.0, 10.0, 999.0, 0.0), true);
    registry.register_or_replace(near_a, near_a_info, Default::default());
    let near_b = ObjectGuid::create_player(1, 90_002);
    let (near_b_info, near_b_rx) =
        make_registry_player_like_cpp(0, 0, Position::new(12.0, 10.0, -999.0, 0.0), true);
    registry.register_or_replace(near_b, near_b_info, Default::default());
    let wrong_map = ObjectGuid::create_player(1, 90_003);
    let (wrong_map_info, wrong_map_rx) =
        make_registry_player_like_cpp(1, 0, Position::new(10.0, 10.0, 0.0, 0.0), true);
    registry.register_or_replace(wrong_map, wrong_map_info, Default::default());

    let mmap_config = wow_world::MMapRuntimeConfigLikeCpp {
        enabled: false,
        ..Default::default()
    };
    let legacy_for_task = Arc::clone(&legacy);
    let canonical_for_task = Arc::clone(&canonical);
    let registry_for_task = Arc::clone(&registry);
    let handle = tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_millis(1));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        interval.tick().await;
        tokio::task::spawn_blocking(move || {
            run_legacy_creature_movement_tick_and_deliver_once_like_cpp(
                &legacy_for_task,
                Some(&canonical_for_task),
                &mmap_config,
                None,
                1,
                registry_for_task.as_ref(),
            )
        })
        .await
        .expect("legacy global tick task must not panic")
    });
    let (outcome, delivery) = handle.await.expect("tick task must complete");

    assert!(!outcome.skipped_owner_not_global);
    assert_eq!(outcome.maps_seen, 1);
    assert_eq!(outcome.creatures_seen, 1);
    assert_eq!(outcome.movement_packets, 1);
    assert_eq!(outcome.canonical_syncs, 1);
    assert_eq!(delivery.events_seen, 1);
    assert_eq!(delivery.candidates_seen, 3);
    assert_eq!(delivery.candidates_queued, 2);
    assert_eq!(delivery.candidates_skipped_wrong_map, 1);

    for command in [
        near_a_rx.try_recv().expect("near player A command"),
        near_b_rx.try_recv().expect("near player B command"),
    ] {
        let SessionCommand::SendIfVisibleLikeCpp(command) = command else {
            panic!("expected SendIfVisibleLikeCpp command");
        };
        assert_eq!(command.source_guid, guid);
        assert_eq!(command.map_id, 0);
        assert_eq!(command.instance_id, 0);
        let opcode = u16::from_le_bytes([command.packet_bytes[0], command.packet_bytes[1]]);
        assert_eq!(opcode, wow_constants::ServerOpcodes::OnMonsterMove as u16);
    }
    assert!(wrong_map_rx.try_recv().is_err());

    let guard = canonical.lock().unwrap();
    let typed = guard
        .find_map(0, 0)
        .unwrap()
        .map()
        .get_typed_creature(guid)
        .expect("canonical creature record stays synced by the single-shot driver");
    assert_eq!(
        typed.ai_state(),
        wow_entities::CreatureAiState::WalkingRandom
    );
}

/// Combined runtime bridge: one test-only task runs lifecycle first
/// (map-owned despawn/respawn visibility refresh), then movement
/// (NearbyVisible MonsterMove fanout), then the transitional melee
/// compatibility bridge while retaining the unrepresented-outcome marker.
/// `GlobalLegacy` is enabled only inside the test.
#[tokio::test]
async fn legacy_creature_global_runtime_task_delivers_lifecycle_movement_and_melee_like_cpp() {
    let legacy: wow_world::SharedMapManager =
        Arc::new(std::sync::RwLock::new(wow_world::MapManager::new()));
    let canonical: wow_world::SharedCanonicalMapManager =
        Arc::new(Mutex::new(wow_map::MapManager::default()));
    canonical.lock().unwrap().create_world_map(0, 0);

    let melee_victim = ObjectGuid::create_player(1, 92_004);
    let melee_position = Position::new(300.0, 300.0, 0.0, 0.0);
    add_canonical_test_player_on_map_like_cpp(&canonical, melee_victim, melee_position, 0, 0, 100);

    let moving_guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 9001, 90_013);
    let moving_position = Position::new(10.0, 10.0, 0.0, 0.0);
    let mut moving_creature = wow_world::map_manager::WorldCreature::new(
        moving_guid,
        9001,
        moving_position,
        25,
        2,
        3,
        5,
        20.0,
        100,
        14,
        0,
        0,
    );
    {
        let ai = moving_creature.creature.ai_ownership_mut();
        ai.wander_delay_ms = 0;
        ai.move_start_ms = 0;
        ai.wander_radius = 3.0;
        ai.aggro_radius = 0.0;
    }
    moving_creature
        .creature
        .set_default_movement_type_runtime_like_cpp(wow_entities::MovementGeneratorType::Random);
    moving_creature.seed_runtime_rng_like_cpp(0x900D);
    let mut canonical_moving = moving_creature.creature.clone();
    canonical_moving
        .unit_mut()
        .world_mut()
        .set_map(0, 0)
        .unwrap();
    canonical_moving
        .unit_mut()
        .world_mut()
        .object_mut()
        .add_to_world();
    canonical
        .lock()
        .unwrap()
        .find_map_mut(0, 0)
        .unwrap()
        .map_mut()
        .insert_map_object_record(MapObjectRecord::new_creature(canonical_moving).unwrap())
        .unwrap();

    let corpse_guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 9001, 90_014);
    let mut corpse_creature = wow_world::map_manager::WorldCreature::new(
        corpse_guid,
        9001,
        Position::new(20.0, 20.0, 0.0, 0.0),
        10,
        2,
        3,
        5,
        20.0,
        100,
        14,
        0,
        0,
    );
    corpse_creature.take_damage(10);
    corpse_creature.set_corpse_despawn_at(Some(
        std::time::Instant::now() - std::time::Duration::from_secs(1),
    ));

    let melee_guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 9001, 90_015);
    add_canonical_test_creature_on_map_like_cpp(&canonical, melee_guid, melee_position, 0, 0, 25);
    let mut melee_creature =
        mirror_canonical_melee_test_creature_like_cpp(&canonical, melee_guid, 0, 0);
    melee_creature.enter_combat(melee_victim);
    melee_creature.creature.ai_ownership_mut().swing_timer_ms = 0;

    {
        let mut manager = legacy.write().unwrap();
        manager.add_creature(
            0,
            0,
            wow_world::map_manager::world_to_grid_x(moving_position.x),
            wow_world::map_manager::world_to_grid_y(moving_position.y),
            moving_creature,
        );
        manager.add_creature(
            0,
            0,
            wow_world::map_manager::world_to_grid_x(20.0),
            wow_world::map_manager::world_to_grid_y(20.0),
            corpse_creature,
        );
        manager.add_creature(
            0,
            0,
            wow_world::map_manager::world_to_grid_x(melee_position.x),
            wow_world::map_manager::world_to_grid_y(melee_position.y),
            melee_creature,
        );
        manager.set_tick_owner(wow_world::map_manager::RuntimeTickOwner::GlobalLegacy);
    }

    let registry = Arc::new(PlayerRegistry::default());
    assert!(registry.bind_canonical_map_manager(Arc::clone(&canonical)));
    let near_a = ObjectGuid::create_player(1, 92_001);
    let (near_a_info, near_a_rx) =
        make_registry_player_like_cpp(0, 0, Position::new(11.0, 10.0, 999.0, 0.0), true);
    registry.register_or_replace(near_a, near_a_info, Default::default());
    let near_b = ObjectGuid::create_player(1, 92_002);
    let (near_b_info, near_b_rx) =
        make_registry_player_like_cpp(0, 0, Position::new(12.0, 10.0, -999.0, 0.0), true);
    registry.register_or_replace(near_b, near_b_info, Default::default());
    let wrong_map = ObjectGuid::create_player(1, 92_003);
    let (wrong_map_info, wrong_map_rx) =
        make_registry_player_like_cpp(1, 0, Position::new(10.0, 10.0, 0.0, 0.0), true);
    registry.register_or_replace(wrong_map, wrong_map_info, Default::default());
    let (mut victim_info, victim_rx) = make_registry_player_like_cpp(0, 0, melee_position, true);
    registry.register_or_replace(melee_victim, victim_info, Default::default());
    add_canonical_test_player_on_map_like_cpp(
        &canonical,
        near_a,
        Position::new(11.0, 10.0, 999.0, 0.0),
        0,
        0,
        100,
    );
    add_canonical_test_player_on_map_like_cpp(
        &canonical,
        near_b,
        Position::new(12.0, 10.0, -999.0, 0.0),
        0,
        0,
        100,
    );
    canonical.lock().unwrap().create_world_map(1, 0);
    add_canonical_test_player_on_map_like_cpp(
        &canonical,
        wrong_map,
        Position::new(10.0, 10.0, 0.0, 0.0),
        1,
        0,
        100,
    );
    let mmap_config = wow_world::MMapRuntimeConfigLikeCpp {
        enabled: false,
        ..Default::default()
    };
    let aggro_config = wow_world::session::LegacyCreatureAggroConfigLikeCpp {
        faction_template_store: Some(Arc::new(
            wow_data::progression_rewards::FactionTemplateStore::from_entries([
                wow_data::progression_rewards::FactionTemplateEntry {
                    id: 14,
                    faction: 72,
                    flags: 0,
                    faction_group: 0,
                    friend_group: 0,
                    enemy_group: 0,
                    enemies: [930, 0, 0, 0, 0, 0, 0, 0],
                    friend: [0; 8],
                },
                wow_data::progression_rewards::FactionTemplateEntry {
                    id: 1,
                    faction: 930,
                    flags: 0,
                    faction_group: 0,
                    friend_group: 0,
                    enemy_group: 0,
                    enemies: [0; 8],
                    friend: [0; 8],
                },
            ]),
        )),
        faction_store: Some(Arc::new(
            wow_data::progression_rewards::FactionStore::from_entries([
                wow_data::progression_rewards::FactionEntry::for_test_like_cpp(72, 1),
            ]),
        )),
        ..Default::default()
    };
    let legacy_for_task = Arc::clone(&legacy);
    let canonical_for_task = Arc::clone(&canonical);
    let registry_for_task = Arc::clone(&registry);
    let map_store_for_task = legacy_runtime_world_map_store_like_cpp();
    let tick_now = std::time::Instant::now() + std::time::Duration::from_millis(1);
    let handle = tokio::spawn(async move {
        tokio::task::spawn_blocking(move || {
            run_legacy_creature_runtime_tick_and_deliver_once_like_cpp(
                &legacy_for_task,
                Some(&canonical_for_task),
                &map_store_for_task,
                &mmap_config,
                None,
                aggro_config,
                10,
                tick_now,
                registry_for_task.as_ref(),
                None,
                None,
                None,
                &Arc::new(Mutex::new(Default::default())),
            )
        })
        .await
        .expect("combined legacy runtime tick task must not panic")
    });
    let outcome = handle.await.expect("combined tick task must complete");

    assert!(!outcome.lifecycle.skipped_owner_not_global);
    assert_eq!(outcome.lifecycle.maps_seen, 1);
    assert_eq!(outcome.lifecycle.creatures_seen, 3);
    assert_eq!(outcome.lifecycle.corpses_despawned, 1);
    assert_eq!(outcome.lifecycle.refresh_map_keys, vec![(0, 0)]);
    assert_eq!(outcome.lifecycle_delivery.candidates_seen, 4);
    assert_eq!(outcome.lifecycle_delivery.candidates_queued, 3);
    assert_eq!(outcome.lifecycle_delivery.candidates_skipped_wrong_map, 1);

    assert!(!outcome.movement.skipped_owner_not_global);
    assert_eq!(outcome.movement.maps_seen, 1);
    assert_eq!(outcome.movement.creatures_seen, 2);
    assert_eq!(outcome.movement.movement_packets, 2);
    assert_eq!(outcome.movement_delivery.events_seen, 2);
    assert_eq!(outcome.movement_delivery.candidates_seen, 8);
    assert_eq!(outcome.movement_delivery.candidates_queued, 3);
    assert_eq!(outcome.movement_delivery.candidates_skipped_distance, 3);
    assert_eq!(outcome.movement_delivery.candidates_skipped_wrong_map, 2);

    assert!(!outcome.aggro.skipped_owner_not_global);
    assert_eq!(outcome.aggro.maps_seen, 1);
    assert_eq!(outcome.aggro.creatures_seen, 2);
    assert_eq!(outcome.aggro.candidates_seen, 3);
    assert_eq!(outcome.aggro.aggro_starts, 0);
    assert_eq!(outcome.aggro_delivery.commands_seen, 0);
    assert_eq!(outcome.aggro_delivery.candidates_queued, 0);

    assert!(!outcome.melee.skipped_owner_not_global);
    assert_eq!(outcome.melee.maps_seen, 1);
    assert_eq!(outcome.melee.creatures_seen, 2);
    assert_eq!(outcome.melee.swings_ready, 1);
    assert_eq!(outcome.melee.melee_outcomes_unrepresented, 1);
    assert_eq!(outcome.melee.runtime_rng_authority_rejections, 0);
    assert_eq!(outcome.melee.canonical_hits, 1);
    assert_eq!(outcome.melee_delivery.commands_seen, 1);
    assert_eq!(outcome.melee_delivery.candidates_seen, 1);
    assert_eq!(outcome.melee_delivery.candidates_queued, 1);
    assert_eq!(outcome.melee_plan_delivery.events_seen, 0);

    for command_rx in [&near_a_rx, &near_b_rx] {
        let SessionCommand::RefreshVisibleWorldCreaturesLikeCpp(refresh) = command_rx
            .try_recv()
            .expect("same-map player must receive lifecycle refresh")
        else {
            panic!("expected RefreshVisibleWorldCreaturesLikeCpp command");
        };
        assert_eq!(refresh.map_id, 0);
        assert_eq!(refresh.instance_id, 0);

        let SessionCommand::SendIfVisibleLikeCpp(move_command) = command_rx
            .try_recv()
            .expect("same-map player must receive movement command")
        else {
            panic!("expected SendIfVisibleLikeCpp command");
        };
        assert_eq!(move_command.source_guid, moving_guid);
        assert_eq!(move_command.map_id, 0);
        assert_eq!(move_command.instance_id, 0);
        let opcode =
            u16::from_le_bytes([move_command.packet_bytes[0], move_command.packet_bytes[1]]);
        assert_eq!(opcode, wow_constants::ServerOpcodes::OnMonsterMove as u16);
    }

    let SessionCommand::RefreshVisibleWorldCreaturesLikeCpp(refresh) = victim_rx
        .try_recv()
        .expect("victim same-map session must receive lifecycle refresh")
    else {
        panic!("expected RefreshVisibleWorldCreaturesLikeCpp command for victim");
    };
    assert_eq!(refresh.map_id, 0);
    assert_eq!(refresh.instance_id, 0);
    let SessionCommand::SendIfVisibleLikeCpp(chase_stop) = victim_rx
        .try_recv()
        .expect("melee victim receives its attacker's in-range chase stop")
    else {
        panic!("expected SendIfVisibleLikeCpp chase-stop command for victim");
    };
    assert_eq!(chase_stop.source_guid, melee_guid);
    assert_eq!(chase_stop.map_id, 0);
    assert_eq!(chase_stop.instance_id, 0);
    assert_eq!(
        u16::from_le_bytes([chase_stop.packet_bytes[0], chase_stop.packet_bytes[1]]),
        wow_constants::ServerOpcodes::OnMonsterMove as u16
    );
    let SessionCommand::ApplyCreatureMeleeDamageLikeCpp(melee_command) =
        drain_durable_creature_runtime_commands_like_cpp(registry.as_ref(), melee_victim)
            .pop()
            .expect("victim must receive the compatibility melee command")
    else {
        panic!("expected ApplyCreatureMeleeDamageLikeCpp command for victim");
    };
    assert_eq!(melee_command.attacker_guid, melee_guid);
    assert_eq!(melee_command.victim_guid, melee_victim);
    assert!((3..=5).contains(&melee_command.damage));
    assert!(
        victim_rx.try_recv().is_err(),
        "victim receives only its own attacker's chase stop"
    );
    assert!(wrong_map_rx.try_recv().is_err());

    {
        let guard = legacy.read().unwrap();
        assert!(
            guard.find_creature(0, 0, corpse_guid).is_none(),
            "expired corpse must be removed by lifecycle before movement"
        );
        assert!(
            guard.find_creature(0, 0, moving_guid).is_some(),
            "alive moving creature must remain in the legacy map"
        );
        assert!(
            guard.find_creature(0, 0, melee_guid).is_some(),
            "alive melee creature must remain in the legacy map"
        );
    }
    let guard = canonical.lock().unwrap();
    let typed = guard
        .find_map(0, 0)
        .unwrap()
        .map()
        .get_typed_creature(moving_guid)
        .expect("movement phase must keep canonical moving creature synced");
    assert_eq!(
        typed.ai_state(),
        wow_entities::CreatureAiState::WalkingRandom
    );
    let victim_health = guard
        .find_map(0, 0)
        .unwrap()
        .map()
        .get_typed_player(melee_victim)
        .unwrap()
        .unit()
        .data()
        .health;
    assert_eq!(victim_health, melee_command.victim_health_after);
    assert_eq!(victim_health, 100 - u64::from(melee_command.damage));
}

/// 4B.2a smoke: exercise the real experimental production loop wrapper,
/// not only the single-shot bridge.  The loop remains disabled by default;
/// this test flips `GlobalLegacy` explicitly, waits for one visible
/// movement command, then aborts the forever-running task.
#[tokio::test]
async fn legacy_creature_runtime_loop_smoke_delivers_visible_work_like_cpp() {
    let legacy: wow_world::SharedMapManager =
        Arc::new(std::sync::RwLock::new(wow_world::MapManager::new()));
    let canonical: wow_world::SharedCanonicalMapManager =
        Arc::new(Mutex::new(wow_map::MapManager::default()));
    canonical.lock().unwrap().create_world_map(0, 0);

    let creature_guid =
        ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 9001, 94_001);
    let creature_position = Position::new(10.0, 10.0, 0.0, 0.0);
    let mut world_creature = wow_world::map_manager::WorldCreature::new(
        creature_guid,
        9001,
        creature_position,
        25,
        2,
        3,
        5,
        20.0,
        100,
        14,
        0,
        0,
    );
    {
        let ai = world_creature.creature.ai_ownership_mut();
        ai.wander_delay_ms = 0;
        ai.move_start_ms = 0;
        ai.wander_radius = 3.0;
        ai.aggro_radius = 0.0;
        ai.swing_timer_ms = u64::MAX;
    }
    world_creature
        .creature
        .set_default_movement_type_runtime_like_cpp(wow_entities::MovementGeneratorType::Random);
    world_creature.seed_runtime_rng_like_cpp(0x9401);

    let mut canonical_creature = world_creature.creature.clone();
    canonical_creature
        .unit_mut()
        .world_mut()
        .set_map(0, 0)
        .unwrap();
    canonical_creature
        .unit_mut()
        .world_mut()
        .object_mut()
        .add_to_world();
    canonical
        .lock()
        .unwrap()
        .find_map_mut(0, 0)
        .unwrap()
        .map_mut()
        .insert_map_object_record(MapObjectRecord::new_creature(canonical_creature).unwrap())
        .unwrap();

    {
        let mut manager = legacy.write().unwrap();
        manager.add_creature(
            0,
            0,
            wow_world::map_manager::world_to_grid_x(creature_position.x),
            wow_world::map_manager::world_to_grid_y(creature_position.y),
            world_creature,
        );
        manager.set_tick_owner(wow_world::map_manager::RuntimeTickOwner::GlobalLegacy);
    }

    let registry = Arc::new(PlayerRegistry::with_canonical_player_fixtures_like_cpp());
    let player = ObjectGuid::create_player(1, 94_002);
    let (player_info, player_rx) =
        make_registry_player_like_cpp(0, 0, Position::new(11.0, 10.0, 0.0, 0.0), true);
    registry.register_or_replace(player, player_info, Default::default());

    let handle = spawn_legacy_creature_runtime_update_loop_like_cpp(
        true,
        Arc::clone(&legacy),
        Arc::clone(&canonical),
        Arc::new(legacy_runtime_world_map_store_like_cpp()),
        wow_world::MMapRuntimeConfigLikeCpp {
            enabled: false,
            ..Default::default()
        },
        None,
        wow_world::session::LegacyCreatureAggroConfigLikeCpp::default(),
        1,
        None,
        Arc::new(Mutex::new(())),
        Arc::new(std::sync::atomic::AtomicBool::new(false)),
        None,
        Arc::clone(&registry),
    );

    let command = tokio::time::timeout(std::time::Duration::from_secs(2), player_rx.recv_async())
        .await
        .expect("runtime loop should deliver visible work")
        .expect("command channel should stay open");
    handle.abort();
    let _ = handle.await;

    let SessionCommand::SendIfVisibleLikeCpp(command) = command else {
        panic!("expected SendIfVisibleLikeCpp movement command");
    };
    assert_eq!(command.source_guid, creature_guid);
    assert_eq!(command.map_id, 0);
    assert_eq!(command.instance_id, 0);
    let opcode = u16::from_le_bytes([command.packet_bytes[0], command.packet_bytes[1]]);
    assert_eq!(opcode, wow_constants::ServerOpcodes::OnMonsterMove as u16);

    let guard = canonical.lock().unwrap();
    let typed = guard
        .find_map(0, 0)
        .unwrap()
        .map()
        .get_typed_creature(creature_guid)
        .expect("production loop must keep canonical creature synced");
    assert_eq!(
        typed.ai_state(),
        wow_entities::CreatureAiState::WalkingRandom
    );
}

#[tokio::test]
async fn legacy_respawn_producer_stop_runs_final_lifecycle_flush_like_cpp() {
    let legacy: wow_world::SharedMapManager =
        Arc::new(std::sync::RwLock::new(wow_world::MapManager::new()));
    let canonical: wow_world::SharedCanonicalMapManager =
        Arc::new(Mutex::new(wow_map::MapManager::default()));
    canonical.lock().unwrap().create_world_map(0, 0);

    let creature_guid =
        ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 9001, 95_001);
    let position = Position::new(10.0, 10.0, 0.0, 0.0);
    let mut creature = wow_world::map_manager::WorldCreature::new(
        creature_guid,
        9001,
        position,
        10,
        2,
        3,
        5,
        20.0,
        100,
        14,
        0,
        0,
    );
    creature.creature.set_spawn_id(95_001);
    creature.take_damage(10);
    {
        let mut manager = legacy.write().unwrap();
        manager.add_creature(
            0,
            0,
            wow_world::map_manager::world_to_grid_x(position.x),
            wow_world::map_manager::world_to_grid_y(position.y),
            creature,
        );
        manager.set_tick_owner(wow_world::map_manager::RuntimeTickOwner::GlobalLegacy);
    }

    let producer_stop = Arc::new(std::sync::atomic::AtomicBool::new(true));
    let writer_tx = RespawnDbWriterSenderLikeCpp::new_like_cpp();
    let writer_probe = writer_tx.clone();
    let handle = spawn_legacy_creature_runtime_update_loop_like_cpp(
        true,
        legacy,
        canonical,
        Arc::new(legacy_runtime_world_map_store_like_cpp()),
        wow_world::MMapRuntimeConfigLikeCpp {
            enabled: false,
            ..Default::default()
        },
        None,
        wow_world::session::LegacyCreatureAggroConfigLikeCpp::default(),
        1,
        Some(writer_tx),
        Arc::new(Mutex::new(())),
        producer_stop,
        None,
        Arc::new(PlayerRegistry::with_canonical_player_fixtures_like_cpp()),
    );

    tokio::time::timeout(Duration::from_secs(2), handle)
        .await
        .expect("stopping producer must finish its final lifecycle tick")
        .expect("final lifecycle tick must not panic");
    let mut mailbox = writer_probe
        .mailbox
        .state
        .lock()
        .expect("respawn DB mailbox lock");
    assert_eq!(mailbox.queue.pending_len(), 1);
    let mutation = mailbox
        .queue
        .take_due(Instant::now())
        .expect("final lifecycle tick must persist the pending death")
        .pending
        .mutation;
    assert!(matches!(
        mutation,
        RespawnPersistenceMutationLikeCpp::Save { .. }
    ));
    assert_eq!(mailbox.queue.pending_len(), 0);
}
