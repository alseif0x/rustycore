//! Scenarios for [`super`], part 1.
//!
//! Split out of main_tests.rs under #628; assertions and registrations are
//! unchanged and shared fixtures stay in the parent module.

use super::*;
use wow_world::session::directory::detached_session_phase_rail_like_cpp;

#[test]
fn dungeon_encounter_catalog_is_loaded_without_per_session_retention() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let app = fs::read_to_string(root.join("src/app.rs")).unwrap();
    let resources = fs::read_to_string(root.join("src/session_resources.rs")).unwrap();
    let session = fs::read_to_string(root.join("../wow-world/src/session/mod.rs")).unwrap();
    assert!(app.contains("wow_data::DungeonEncounterStore::load(&data_dir, &locale)"));
    assert!(app.contains("Failed to load DungeonEncounter.db2"));
    assert!(!app.contains("dungeon_encounter_store: Arc::clone"));
    assert!(!resources.contains("dungeon_encounter_store"));
    assert!(!session.contains("dungeon_encounter_store"));
}
#[test]
fn signed_tinyint_quest_required_preserves_cpp_boolean_semantics() {
    assert!(!loot_quest_required_from_signed_db_like_cpp(0));
    assert!(loot_quest_required_from_signed_db_like_cpp(1));
    assert!(loot_quest_required_from_signed_db_like_cpp(-1));
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

    let entry = realm_list_entry_from_row_like_cpp(super::super::RealmListRawRowLikeCpp {
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
    let first = super::super::RealmHandleLikeCpp::new_like_cpp(1, 2, 7);
    let same_realm_different_subregion = super::super::RealmHandleLikeCpp::new_like_cpp(9, 8, 7);
    let second = super::super::RealmHandleLikeCpp::new_like_cpp(1, 2, 8);

    assert_eq!(first, same_realm_different_subregion);
    assert_eq!(
        first.cmp(&same_realm_different_subregion),
        std::cmp::Ordering::Equal
    );
    assert!(first < second);
}
#[test]
fn realm_list_snapshot_replace_counts_added_updated_removed_like_cpp() {
    let mut current = super::super::RealmListSnapshotLikeCpp::default();
    let first = realm_list_entry_from_row_like_cpp(super::super::RealmListRawRowLikeCpp {
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
    let second = realm_list_entry_from_row_like_cpp(super::super::RealmListRawRowLikeCpp {
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

    let mut next = super::super::RealmListSnapshotLikeCpp::default();
    next.sub_regions
        .insert(first.id.sub_region_address_like_cpp());
    next.realms.insert(first.id, first.clone());
    assert_eq!(
        current.replace_like_cpp(next),
        super::super::RealmListRefreshSummaryLikeCpp {
            realms: 1,
            sub_regions: 1,
            added: 1,
            updated: 0,
            removed: 0,
        }
    );
    assert!(current.get_realm_like_cpp(first.id).is_some());

    let mut replacement = super::super::RealmListSnapshotLikeCpp::default();
    replacement
        .sub_regions
        .insert(second.id.sub_region_address_like_cpp());
    replacement.realms.insert(second.id, second.clone());
    assert_eq!(
        current.replace_like_cpp(replacement),
        super::super::RealmListRefreshSummaryLikeCpp {
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
    let mut snapshot = super::super::RealmListSnapshotLikeCpp::default();
    let entry = realm_list_entry_from_row_like_cpp(super::super::RealmListRawRowLikeCpp {
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
        super::super::load_realm_info_from_snapshot_like_cpp(&snapshot, 9).expect("realm found"),
        entry
    );
    let loaded =
        super::super::load_realm_info_from_snapshot_like_cpp(&snapshot, 9).expect("realm found");
    assert_eq!(loaded.id.region, 5);
    assert_eq!(loaded.id.site, 6);
    assert_eq!(loaded.id.address_like_cpp(), 0x0506_0009);
    assert_eq!(
        super::super::realm_name_records_from_snapshot_like_cpp(&snapshot).as_ref(),
        &vec![(0x0506_0009, "Icecrown".to_string(), "Icecrown".to_string())]
    );
    assert!(super::super::load_realm_info_from_snapshot_like_cpp(&snapshot, 10).is_err());
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
        .try_register(41, command_tx_a, detached_session_phase_rail_like_cpp())
        .expect("open registry accepts the existing session")
        .0;

    registry.begin_shutdown_like_cpp();

    let (command_tx_b, _command_rx_b) = flume::bounded(1);
    assert!(
        registry
            .try_register(42, command_tx_b, detached_session_phase_rail_like_cpp())
            .is_none()
    );
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
    let (id, cancellation, _ready_for_phases) = registry
        .try_register(44, command_tx, detached_session_phase_rail_like_cpp())
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
        .is_some()
    );
    assert_eq!(world.get_exit_code_like_cpp(), SHUTDOWN_EXIT_CODE_LIKE_CPP);
}
#[tokio::test]
async fn world_session_shutdown_finalize_timeout_sets_terminal_error_like_cpp() {
    let world = WorldRuntimeStateLikeCpp::new();

    assert!(
        run_world_session_shutdown_finalize_step_like_cpp(
            &world,
            Duration::from_millis(1),
            std::future::pending::<()>(),
        )
        .await
        .is_none()
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

    assert!(
        super::super::can_create_missing_login_grid_as_world_map_like_cpp(*world.get(1).unwrap())
    );
    assert!(
        !super::super::can_create_missing_login_grid_as_world_map_like_cpp(
            *dungeon.get(33).unwrap()
        )
    );
    assert!(
        !super::super::can_create_missing_login_grid_as_world_map_like_cpp(
            *battleground.get(489).unwrap()
        )
    );
    assert!(!super::super::can_create_missing_login_grid_as_world_map_like_cpp(garrison));
    assert!(
        !super::super::can_create_missing_login_grid_as_world_map_like_cpp(faction_split_world)
    );
    assert!(
        super::super::existing_login_grid_map_matches_map_entry_like_cpp(
            *world.get(1).unwrap(),
            wow_map::ManagedMapKind::World,
            0,
            false,
        )
    );
    assert!(
        super::super::existing_login_grid_map_matches_map_entry_like_cpp(
            *dungeon.get(33).unwrap(),
            wow_map::ManagedMapKind::Dungeon {
                has_reset_schedule: false,
            },
            77,
            true,
        )
    );
    assert!(
        !super::super::existing_login_grid_map_matches_map_entry_like_cpp(
            *dungeon.get(33).unwrap(),
            wow_map::ManagedMapKind::Dungeon {
                has_reset_schedule: false,
            },
            0,
            true,
        )
    );
    assert!(
        !super::super::existing_login_grid_map_matches_map_entry_like_cpp(
            *dungeon.get(33).unwrap(),
            wow_map::ManagedMapKind::World,
            77,
            true,
        )
    );
    assert!(
        !super::super::existing_login_grid_map_matches_map_entry_like_cpp(
            faction_split_world,
            wow_map::ManagedMapKind::World,
            0,
            false,
        )
    );
    assert!(
        super::super::existing_login_grid_map_matches_map_entry_like_cpp(
            faction_split_world,
            wow_map::ManagedMapKind::World,
            0,
            true,
        )
    );
    assert!(
        !super::super::existing_login_grid_map_matches_map_entry_like_cpp(
            garrison,
            wow_map::ManagedMapKind::World,
            0,
            false,
        )
    );
    assert!(
        super::super::existing_login_grid_map_matches_map_entry_like_cpp(
            garrison,
            wow_map::ManagedMapKind::World,
            0,
            true,
        )
    );
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
