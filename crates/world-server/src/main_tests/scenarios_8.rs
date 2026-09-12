//! Scenarios for [`super`], part 8.
//!
//! Split out of main_tests.rs under #628; assertions and registrations are
//! unchanged and shared fixtures stay in the parent module.

use super::*;

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
            session_phase_tx: wow_world::session::directory::detached_session_phase_rail_like_cpp(),
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
