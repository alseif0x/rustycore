//! Gameobject scenarios for [`super`].
//!
//! Split out of loot_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

#[tokio::test]
async fn represented_gameobject_chest_push_unique_use_records_effects_like_cpp() {
    let mut session = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let gameobject_guid = test_gameobject_guid(91_004);
    session.set_player_guid(Some(player_guid));
    session
        .client_visible_guids_like_cpp
        .insert(gameobject_guid);

    let source = GameObjectLootSource {
        loot_id: 0,
        use_group_loot_rules: false,
        dungeon_encounter_id: 0,
        personal_loot_id: 0,
        push_loot_id: 99,
        triggered_event_id: 321,
        linked_trap_entry: 654,
        ..Default::default()
    };

    session
        .open_represented_gameobject_chest_like_cpp(gameobject_guid, source)
        .await;
    session
        .open_represented_gameobject_chest_like_cpp(gameobject_guid, source)
        .await;

    assert_eq!(
        session.represented_gameobject_use_effects,
        vec![
            RepresentedGameObjectUseEffect::TriggerGameEvent {
                gameobject_guid,
                player_guid,
                event_id: 321,
            },
            RepresentedGameObjectUseEffect::TriggerLinkedTrap {
                gameobject_guid,
                player_guid,
                trap_entry: 654,
            },
        ]
    );
}
#[tokio::test]
async fn represented_gameobject_chest_no_loot_unique_use_records_effects_like_cpp() {
    let mut session = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let gameobject_guid = test_gameobject_guid(91_005);
    session.set_player_guid(Some(player_guid));
    session
        .client_visible_guids_like_cpp
        .insert(gameobject_guid);

    let source = GameObjectLootSource {
        loot_id: 0,
        use_group_loot_rules: false,
        dungeon_encounter_id: 0,
        personal_loot_id: 0,
        push_loot_id: 0,
        triggered_event_id: 901,
        linked_trap_entry: 902,
        ..Default::default()
    };

    session
        .open_represented_gameobject_chest_like_cpp(gameobject_guid, source)
        .await;
    session
        .open_represented_gameobject_chest_like_cpp(gameobject_guid, source)
        .await;

    assert_eq!(
        session.represented_gameobject_use_effects,
        vec![
            RepresentedGameObjectUseEffect::TriggerGameEvent {
                gameobject_guid,
                player_guid,
                event_id: 901,
            },
            RepresentedGameObjectUseEffect::TriggerLinkedTrap {
                gameobject_guid,
                player_guid,
                trap_entry: 902,
            },
        ]
    );
}
#[tokio::test]
async fn represented_gameobject_chest_use_sets_activated_loot_state_like_cpp() {
    let mut session = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let gameobject_guid = test_gameobject_guid(91_006);
    session.set_player_guid(Some(player_guid));
    session
        .client_visible_guids_like_cpp
        .insert(gameobject_guid);

    session
        .open_represented_gameobject_chest_like_cpp(
            gameobject_guid,
            GameObjectLootSource::default(),
        )
        .await;

    let state = session
        .represented_gameobject_use_states
        .get(&gameobject_guid)
        .expect("represented chest use records GO loot state");
    assert_eq!(state.loot_state, Some(LootState::Activated));
    assert_eq!(state.loot_state_unit_guid, player_guid);
}
#[tokio::test]
async fn represented_chest_use_syncs_state_to_same_map_viewers_like_cpp() {
    let mut session = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let same_map_guid = ObjectGuid::create_player(1, 77);
    let other_map_guid = ObjectGuid::create_player(1, 88);
    let gameobject_guid = test_gameobject_guid(91_010);
    let (same_command_tx, same_command_rx) = flume::bounded(2);
    let (other_command_tx, other_command_rx) = flume::bounded(2);
    let (same_send_tx, _same_send_rx) = flume::bounded::<Vec<u8>>(1);
    let (other_send_tx, _other_send_rx) = flume::bounded::<Vec<u8>>(1);
    let player_registry = Arc::new(PlayerRegistry::default());
    let mut same_info = broadcast_info(same_map_guid, same_send_tx);
    same_info.placement.map_id = 571;
    same_info.command_tx = same_command_tx;
    player_registry.register_or_replace(same_map_guid, same_info, Default::default());
    let mut other_info = broadcast_info(other_map_guid, other_send_tx);
    other_info.placement.map_id = 1;
    other_info.command_tx = other_command_tx;
    player_registry.register_or_replace(other_map_guid, other_info, Default::default());
    let source = GameObjectLootSource {
        loot_id: 190_010,
        personal_loot_id: 190_011,
        push_loot_id: 190_012,
        chest_restock_time_secs: 30,
        chest_consumable: false,
        chest_quest_id: 777,
        linked_trap_entry: 190_013,
        ..Default::default()
    };

    session.set_player_guid(Some(player_guid));
    session.set_player_map_position_like_cpp(571, Position::ZERO);
    session.set_player_registry(player_registry);
    session
        .client_visible_guids_like_cpp
        .insert(gameobject_guid);

    session
        .open_represented_gameobject_chest_like_cpp(gameobject_guid, source)
        .await;

    let command = match same_command_rx.try_recv() {
        Ok(SessionCommand::SyncChestGameobjectStateAndRefreshLikeCpp(command)) => command,
        other => panic!("expected chest sync command, got {other:?}"),
    };
    assert_eq!(command.gameobject_guid, gameobject_guid);
    assert_eq!(command.map_id, 571);
    assert_eq!(command.go_type, wow_entities::GAMEOBJECT_TYPE_CHEST as u8);
    assert_eq!(
        command.loot_state,
        Some(wow_entities::LootState::Activated as u8)
    );
    assert_eq!(command.chest_loot_id, 190_010);
    assert_eq!(command.chest_personal_loot_id, 190_011);
    assert_eq!(command.chest_push_loot_id, 190_012);
    assert_eq!(command.chest_quest_id, 777);
    assert_eq!(command.chest_restock_time_secs, 30);
    assert!(!command.chest_consumable);
    assert_eq!(command.linked_trap_entry, Some(190_013));
    assert_eq!(command.linked_trap_guid, None);
    assert!(other_command_rx.try_recv().is_err());
}
#[tokio::test]
async fn chest_state_sync_command_updates_receiver_before_refresh_like_cpp() {
    let mut session = make_session();
    let player_guid = ObjectGuid::create_player(1, 77);
    let gameobject_guid = test_gameobject_guid(91_011);
    session.set_state(SessionState::LoggedIn);
    session.set_player_guid(Some(player_guid));
    session.set_player_map_position_like_cpp(571, Position::ZERO);

    session
        .session_command_tx()
        .try_send(SessionCommand::SyncChestGameobjectStateAndRefreshLikeCpp(
            SyncChestGameobjectStateAndRefreshLikeCppCommand {
                gameobject_guid,
                map_id: 571,
                instance_id: 0,
                go_type: wow_entities::GAMEOBJECT_TYPE_CHEST as u8,
                loot_state: Some(wow_entities::LootState::Activated as u8),
                loot_state_unit_guid: ObjectGuid::create_player(1, 42),
                chest_loot_id: 190_011,
                chest_personal_loot_id: 190_012,
                chest_push_loot_id: 190_013,
                chest_quest_id: 778,
                chest_restock_time_secs: 45,
                chest_consumable: false,
                linked_trap_entry: Some(190_014),
                linked_trap_guid: Some(test_gameobject_guid(91_014)),
            },
        ))
        .expect("command queued");

    session
        .process_represented_session_commands_like_cpp()
        .await;

    let state = session
        .represented_gameobject_use_states
        .get(&gameobject_guid)
        .expect("synced chest state");
    assert_eq!(
        state.go_type,
        Some(wow_entities::GAMEOBJECT_TYPE_CHEST as u8)
    );
    assert_eq!(state.loot_state, Some(wow_entities::LootState::Activated));
    assert_eq!(state.chest_restock_time_secs, Some(45));
    assert_eq!(state.chest_consumable, Some(false));
    assert_eq!(state.chest_personal_loot_id, Some(190_012));
    assert_eq!(state.linked_trap_entry, Some(190_014));
    assert_eq!(state.linked_trap_guid, Some(test_gameobject_guid(91_014)));
    let source = state.chest_loot_source.expect("synced chest source");
    assert_eq!(source.loot_id, 190_011);
    assert_eq!(source.personal_loot_id, 190_012);
    assert_eq!(source.push_loot_id, 190_013);
    assert_eq!(source.chest_quest_id, 778);
}
#[test]
fn represented_goober_use_syncs_shared_state_to_same_map_viewers_like_cpp() {
    let mut session = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let same_map_guid = ObjectGuid::create_player(1, 77);
    let other_map_guid = ObjectGuid::create_player(1, 88);
    let gameobject_guid = test_gameobject_guid(91_012);
    let (same_command_tx, same_command_rx) = flume::bounded(2);
    let (other_command_tx, other_command_rx) = flume::bounded(2);
    let (same_send_tx, _same_send_rx) = flume::bounded::<Vec<u8>>(1);
    let (other_send_tx, _other_send_rx) = flume::bounded::<Vec<u8>>(1);
    let player_registry = Arc::new(PlayerRegistry::default());
    let mut same_info = broadcast_info(same_map_guid, same_send_tx);
    same_info.placement.map_id = 571;
    same_info.command_tx = same_command_tx;
    player_registry.register_or_replace(same_map_guid, same_info, Default::default());
    let mut other_info = broadcast_info(other_map_guid, other_send_tx);
    other_info.placement.map_id = 1;
    other_info.command_tx = other_command_tx;
    player_registry.register_or_replace(other_map_guid, other_info, Default::default());

    session.set_player_guid(Some(player_guid));
    session.set_player_map_position_like_cpp(571, Position::ZERO);
    session.set_player_registry(player_registry);
    session
        .represented_gameobject_use_states
        .entry(gameobject_guid)
        .or_default()
        .linked_trap_entry = Some(190_015);

    assert!(session.use_represented_gameobject_goober_state_like_cpp(
        gameobject_guid,
        player_guid,
        777,
        wow_entities::GooberUseSource {
            auto_close_ms: 3_000,
            linked_trap_entry: 190_015,
            ..Default::default()
        },
    ));

    let command = match same_command_rx.try_recv() {
        Ok(SessionCommand::SyncGooberGameobjectStateAndRefreshLikeCpp(command)) => command,
        other => panic!("expected goober sync command, got {other:?}"),
    };
    assert_eq!(command.gameobject_guid, gameobject_guid);
    assert_eq!(command.map_id, 571);
    assert_eq!(command.go_type, GAMEOBJECT_TYPE_GOOBER as u8);
    assert_eq!(command.gameobject_flags & wow_entities::GO_FLAG_IN_USE, 1);
    assert_eq!(
        command.loot_state,
        Some(wow_entities::LootState::Activated as u8)
    );
    assert_eq!(command.loot_state_unit_guid, player_guid);
    assert_eq!(command.go_state, Some(wow_entities::GoState::Active as i8));
    assert_eq!(command.linked_trap_entry, Some(190_015));
    assert!(other_command_rx.try_recv().is_err());
}
#[tokio::test]
async fn goober_state_sync_command_updates_receiver_before_refresh_like_cpp() {
    let mut session = make_session();
    let player_guid = ObjectGuid::create_player(1, 77);
    let owner_guid = ObjectGuid::create_player(1, 42);
    let gameobject_guid = test_gameobject_guid(91_013);
    session.set_state(SessionState::LoggedIn);
    session.set_player_guid(Some(player_guid));
    session.set_player_map_position_like_cpp(571, Position::ZERO);

    session
        .session_command_tx()
        .try_send(SessionCommand::SyncGooberGameobjectStateAndRefreshLikeCpp(
            SyncGooberGameobjectStateAndRefreshLikeCppCommand {
                gameobject_guid,
                map_id: 571,
                instance_id: 0,
                go_type: GAMEOBJECT_TYPE_GOOBER as u8,
                gameobject_flags: wow_entities::GO_FLAG_IN_USE,
                loot_state: Some(wow_entities::LootState::Activated as u8),
                loot_state_unit_guid: owner_guid,
                go_state: Some(wow_entities::GoState::Active as i8),
                dynamic_flags: wow_entities::GO_DYNFLAG_LO_NO_INTERACT,
                linked_trap_entry: Some(190_016),
                linked_trap_guid: Some(test_gameobject_guid(91_016)),
            },
        ))
        .expect("command queued");

    session
        .process_represented_session_commands_like_cpp()
        .await;

    let state = session
        .represented_gameobject_use_states
        .get(&gameobject_guid)
        .expect("synced goober state");
    assert_eq!(state.go_type, Some(GAMEOBJECT_TYPE_GOOBER as u8));
    assert_eq!(state.gameobject_flags & wow_entities::GO_FLAG_IN_USE, 1);
    assert_eq!(state.loot_state, Some(wow_entities::LootState::Activated));
    assert_eq!(state.loot_state_unit_guid, owner_guid);
    assert_eq!(state.go_state, Some(wow_entities::GoState::Active));
    assert_eq!(
        state.dynamic_flags & wow_entities::GO_DYNFLAG_LO_NO_INTERACT,
        wow_entities::GO_DYNFLAG_LO_NO_INTERACT
    );
    assert_eq!(state.linked_trap_entry, Some(190_016));
    assert_eq!(state.linked_trap_guid, Some(test_gameobject_guid(91_016)));
    assert!(state.cooldown_until.is_none());
    assert!(state.goober_use_source.is_none());
}
#[tokio::test]
async fn represented_gathering_node_use_refreshes_same_map_gameobject_viewers_like_cpp() {
    let mut session = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let same_map_guid = ObjectGuid::create_player(1, 77);
    let other_map_guid = ObjectGuid::create_player(1, 88);
    let gameobject_guid = test_gameobject_guid(91_008);
    let (same_command_tx, same_command_rx) = flume::bounded(2);
    let (other_command_tx, other_command_rx) = flume::bounded(2);
    let (same_send_tx, _same_send_rx) = flume::bounded::<Vec<u8>>(1);
    let (other_send_tx, _other_send_rx) = flume::bounded::<Vec<u8>>(1);
    let player_registry = Arc::new(PlayerRegistry::default());
    let mut same_info = broadcast_info(same_map_guid, same_send_tx);
    same_info.placement.map_id = 571;
    same_info.command_tx = same_command_tx;
    player_registry.register_or_replace(same_map_guid, same_info, Default::default());
    let mut other_info = broadcast_info(other_map_guid, other_send_tx);
    other_info.placement.map_id = 1;
    other_info.command_tx = other_command_tx;
    player_registry.register_or_replace(other_map_guid, other_info, Default::default());

    session.set_player_guid(Some(player_guid));
    session.set_player_map_position_like_cpp(571, Position::ZERO);
    session.set_player_registry(player_registry);
    session
        .client_visible_guids_like_cpp
        .insert(gameobject_guid);

    let source = GatheringNodeUseSource {
        loot_id: 0,
        despawn_delay_secs: 0,
        triggered_event_id: 0,
        xp_difficulty: 0,
        spell_id: 0,
        max_loots: 1,
        linked_trap_entry: 0,
    };

    session
        .open_represented_gathering_node_like_cpp(gameobject_guid, 190_008, source)
        .await;

    let command = match same_command_rx.try_recv() {
        Ok(SessionCommand::SyncGatheringNodeGameobjectStateAndRefreshLikeCpp(command)) => command,
        other => panic!("expected gathering-node sync command, got {other:?}"),
    };
    assert_eq!(command.gameobject_guid, gameobject_guid);
    assert_eq!(command.map_id, 571);
    assert_eq!(
        command.go_type,
        wow_entities::GAMEOBJECT_TYPE_GATHERING_NODE as u8
    );
    assert_eq!(
        command.loot_state,
        Some(wow_entities::LootState::Activated as u8)
    );
    assert_eq!(command.go_state, Some(wow_entities::GoState::Active as i8));
    assert_eq!(command.gathering_node_loot_id, Some(0));
    assert_eq!(
        command.dynamic_flags & wow_entities::GO_DYNFLAG_LO_NO_INTERACT,
        wow_entities::GO_DYNFLAG_LO_NO_INTERACT
    );
    assert!(other_command_rx.try_recv().is_err());
}
#[test]
fn represented_gameobject_group_loot_keeps_round_robin_empty_like_cpp() {
    let mut session = make_session();
    let opener = ObjectGuid::create_player(1, 42);
    let candidate = ObjectGuid::create_player(1, 77);
    install_master_loot_group(&mut session, opener, candidate);

    let (loot_method, loot_master, round_robin_player) =
        session.represented_gameobject_chest_group_state_like_cpp(true, opener);

    assert_eq!(loot_method, LOOT_METHOD_MASTER_LIKE_CPP);
    assert_eq!(loot_master, opener);
    assert_eq!(round_robin_player, ObjectGuid::EMPTY);
}
#[test]
fn gameobject_interaction_distance_uses_cpp_type_branches() {
    assert_eq!(
        represented_gameobject_interaction_distance_like_cpp(
            Some(GAMEOBJECT_TYPE_CHEST as u8),
            Some(725)
        ),
        7.25
    );
    assert_eq!(
        represented_gameobject_interaction_distance_like_cpp(
            Some(GAMEOBJECT_TYPE_AREADAMAGE as u8),
            None
        ),
        0.0
    );
    assert_eq!(
        represented_gameobject_interaction_distance_like_cpp(
            Some(GAMEOBJECT_TYPE_QUESTGIVER as u8),
            None
        ),
        5.5555553
    );
    assert_eq!(
        represented_gameobject_interaction_distance_like_cpp(
            Some(GAMEOBJECT_TYPE_BINDER as u8),
            None
        ),
        10.0
    );
    assert_eq!(
        represented_gameobject_interaction_distance_like_cpp(
            Some(GAMEOBJECT_TYPE_CHAIR as u8),
            None
        ),
        3.0
    );
    assert_eq!(
        represented_gameobject_interaction_distance_like_cpp(
            Some(GAMEOBJECT_TYPE_FISHING_NODE as u8),
            None
        ),
        100.0
    );
    assert_eq!(
        represented_gameobject_interaction_distance_like_cpp(
            Some(GAMEOBJECT_TYPE_FISHING_HOLE as u8),
            None
        ),
        20.0 + wow_movement::CONTACT_DISTANCE_LIKE_CPP
    );
    assert_eq!(
        represented_gameobject_interaction_distance_like_cpp(
            Some(GAMEOBJECT_TYPE_DOOR as u8),
            None
        ),
        5.0
    );
    assert_eq!(
        represented_gameobject_interaction_distance_like_cpp(
            Some(GAMEOBJECT_TYPE_GUILD_BANK as u8),
            None
        ),
        10.0
    );
    assert_eq!(
        represented_gameobject_interaction_distance_like_cpp(None, None),
        5.0
    );
}
#[test]
fn gameobject_display_box_interaction_matches_cpp_contains_branch() {
    let display_info = wow_data::GameObjectDisplayInfoEntry {
        id: 77,
        model_name: "test".to_string(),
        geo_box_min: wow_data::Db2Pos3 {
            x: -2.0,
            y: -1.0,
            z: -0.5,
        },
        geo_box_max: wow_data::Db2Pos3 {
            x: 2.0,
            y: 1.0,
            z: 0.5,
        },
        file_data_id: 0,
        object_effect_package_id: 0,
        override_loot_effect_scale: 0.0,
        override_name_scale: 0.0,
    };
    let go_position = Position::ZERO;

    assert!(represented_gameobject_display_box_contains_like_cpp(
        go_position,
        Position::xyz(6.9, 0.0, 0.0),
        &display_info,
        1.0,
        [0.0, 0.0, 0.0, 1.0],
        5.0,
    ));
    assert!(!represented_gameobject_display_box_contains_like_cpp(
        go_position,
        Position::xyz(7.1, 0.0, 0.0),
        &display_info,
        1.0,
        [0.0, 0.0, 0.0, 1.0],
        5.0,
    ));
}
#[test]
fn gameobject_loot_distance_uses_display_box_when_db2_exists_like_cpp() {
    let mut session = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let gameobject_guid = test_gameobject_guid(19_041);
    session.set_player_guid(Some(player_guid));
    session.record_represented_gameobject_runtime_state_like_cpp(
        0,
        gameobject_guid,
        gameobject_guid.entry(),
        Position::ZERO,
        GAMEOBJECT_TYPE_CHEST as u8,
    );
    session.record_represented_gameobject_display_model_like_cpp(
        gameobject_guid,
        77,
        1.0,
        [0.0, 0.0, 0.0, 1.0],
    );
    session.set_gameobject_display_info_store(Arc::new(
        wow_data::GameObjectDisplayInfoStore::from_entries([
            wow_data::GameObjectDisplayInfoEntry {
                id: 77,
                model_name: "test".to_string(),
                geo_box_min: wow_data::Db2Pos3 {
                    x: -2.0,
                    y: -1.0,
                    z: -0.5,
                },
                geo_box_max: wow_data::Db2Pos3 {
                    x: 2.0,
                    y: 1.0,
                    z: 0.5,
                },
                file_data_id: 0,
                object_effect_package_id: 0,
                override_loot_effect_scale: 0.0,
                override_name_scale: 0.0,
            },
        ]),
    ));

    session.set_player_position_like_cpp(Position::xyz(6.9, 0.0, 0.0));
    assert!(
        session
            .represented_gameobject_can_autostore_loot_item_like_cpp(gameobject_guid, player_guid)
    );

    session.set_player_position_like_cpp(Position::xyz(7.1, 0.0, 0.0));
    assert!(
        !session
            .represented_gameobject_can_autostore_loot_item_like_cpp(gameobject_guid, player_guid)
    );
}
#[test]
fn gameobject_loot_distance_uses_spell_lock_range_like_cpp() {
    let mut session = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let gameobject_guid = test_gameobject_guid(19_042);
    session.set_player_guid(Some(player_guid));
    session.set_player_position_like_cpp(Position::xyz(11.0, 0.0, 0.0));
    session.record_represented_gameobject_runtime_state_like_cpp(
        0,
        gameobject_guid,
        gameobject_guid.entry(),
        Position::ZERO,
        GAMEOBJECT_TYPE_CHEST as u8,
    );
    session.record_represented_gameobject_lock_id_like_cpp(gameobject_guid, 501);
    session.set_lock_store(Arc::new(wow_data::LockStore::from_entries([
        wow_data::LockEntry {
            id: 501,
            index: [7001, 0, 0, 0, 0, 0, 0, 0],
            skill: [0; wow_data::lock::MAX_LOCK_CASE],
            lock_type: [LOCK_KEY_SPELL_LIKE_CPP, 0, 0, 0, 0, 0, 0, 0],
            action: [0; wow_data::lock::MAX_LOCK_CASE],
        },
    ])));
    let mut spell_store = SpellStore::new();
    spell_store.insert(
        7001,
        SpellInfo {
            spell_id: 7001,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: 0,
            effect_base_points: 0,
            effect_bonus_coefficient: 0.0,
            aura_type: None,
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: Vec::new(),
        },
    );
    session.set_spell_store(Arc::new(spell_store));
    session.set_spell_misc_store(Arc::new(SpellMiscStore::from_entries([SpellMiscEntry {
        id: 7001,
        attributes: [0; 15],
        difficulty_id: 0,
        casting_time_index: 0,
        duration_index: 0,
        range_index: 77,
        school_mask: 0,
        speed: 0.0,
        launch_delay: 0.0,
        min_duration: 0.0,
        spell_icon_file_data_id: 0,
        active_icon_file_data_id: 0,
        content_tuning_id: 0,
        show_future_spell_player_condition_id: 0,
        spell_id: 7001,
    }])));
    session.set_spell_range_store(Arc::new(SpellRangeStore::from_entries([SpellRangeEntry {
        id: 77,
        display_name: "lock".to_string(),
        display_name_short: "lock".to_string(),
        flags: 0,
        range_min: [0.0, 0.0],
        range_max: [12.0, 12.0],
    }])));

    assert!(
        session
            .represented_gameobject_can_autostore_loot_item_like_cpp(gameobject_guid, player_guid)
    );

    session.set_player_position_like_cpp(Position::xyz(12.1, 0.0, 0.0));
    assert!(
        !session
            .represented_gameobject_can_autostore_loot_item_like_cpp(gameobject_guid, player_guid)
    );
}
#[test]
fn gameobject_loot_distance_uses_known_open_lock_skill_spell_like_cpp() {
    let mut session = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let gameobject_guid = test_gameobject_guid(19_043);
    session.set_player_guid(Some(player_guid));
    session.set_player_position_like_cpp(Position::xyz(8.0, 0.0, 0.0));
    session.set_known_spells_like_cpp(vec![8001]);
    session.record_represented_gameobject_runtime_state_like_cpp(
        0,
        gameobject_guid,
        gameobject_guid.entry(),
        Position::ZERO,
        GAMEOBJECT_TYPE_CHEST as u8,
    );
    session.record_represented_gameobject_lock_id_like_cpp(gameobject_guid, 502);
    session.set_lock_store(Arc::new(wow_data::LockStore::from_entries([
        wow_data::LockEntry {
            id: 502,
            index: [333, 0, 0, 0, 0, 0, 0, 0],
            skill: [50, 0, 0, 0, 0, 0, 0, 0],
            lock_type: [LOCK_KEY_SKILL_LIKE_CPP, 0, 0, 0, 0, 0, 0, 0],
            action: [0; wow_data::lock::MAX_LOCK_CASE],
        },
    ])));
    let mut spell_store = SpellStore::new();
    spell_store.insert(
        8001,
        SpellInfo {
            spell_id: 8001,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: SPELL_EFFECT_OPEN_LOCK_LIKE_CPP,
            effect_base_points: 75,
            effect_bonus_coefficient: 0.0,
            aura_type: None,
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: vec![SpellEffectInfo {
                effect_index: 0,
                effect: SPELL_EFFECT_OPEN_LOCK_LIKE_CPP,
                effect_aura: 0,
                effect_base_points: 75,
                effect_misc_value_1: 333,
                effect_misc_value_2: 0,
                effect_radius_index_1: 0,
                chain_targets: 0,
                implicit_target_1: 0,
                implicit_target_2: 0,
                ..Default::default()
            }],
        },
    );
    session.set_spell_store(Arc::new(spell_store));
    session.set_spell_misc_store(Arc::new(SpellMiscStore::from_entries([SpellMiscEntry {
        id: 8001,
        attributes: [0; 15],
        difficulty_id: 0,
        casting_time_index: 0,
        duration_index: 0,
        range_index: 88,
        school_mask: 0,
        speed: 0.0,
        launch_delay: 0.0,
        min_duration: 0.0,
        spell_icon_file_data_id: 0,
        active_icon_file_data_id: 0,
        content_tuning_id: 0,
        show_future_spell_player_condition_id: 0,
        spell_id: 8001,
    }])));
    session.set_spell_range_store(Arc::new(SpellRangeStore::from_entries([SpellRangeEntry {
        id: 88,
        display_name: "skill".to_string(),
        display_name_short: "skill".to_string(),
        flags: 0,
        range_min: [0.0, 0.0],
        range_max: [9.0, 9.0],
    }])));

    assert!(
        session
            .represented_gameobject_can_autostore_loot_item_like_cpp(gameobject_guid, player_guid)
    );
}
#[tokio::test]
async fn loot_item_missing_gameobject_uses_cpp_release() {
    let (mut session, send_rx) = make_session_with_send();
    let player_guid = ObjectGuid::create_player(1, 42);
    let loot_guid = test_gameobject_guid(19_010);
    session.set_player_guid(Some(player_guid));
    session.set_active_loot_guid(loot_guid);
    session.loot_table.insert(
        loot_guid,
        CreatureLoot {
            loot_guid,
            coins: 0,
            unlooted_count: 0,
            loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
            dungeon_encounter_id: 0,
            loot_method: 0,
            loot_master: ObjectGuid::EMPTY,
            round_robin_player: ObjectGuid::EMPTY,
            player_ffa_items: Vec::new(),
            players_looting: Vec::new(),
            allowed_looters: Vec::new(),
            items: vec![LootEntry {
                loot_list_id: 0,
                item_id: 25,
                quantity: 1,
                random_properties_id: 0,
                random_properties_seed: 0,
                item_context: 0,
                flags: LootEntryFlags::default(),
                allowed_looters: vec![player_guid],
                roll_winner: ObjectGuid::EMPTY,
                ffa_looted_by: Vec::new(),
                taken: false,
            }],
            looted_by_player: false,
        },
    );

    session
        .handle_loot_item(loot_item_packet(loot_guid, 0))
        .await;

    let sent = send_rx.try_recv().unwrap();
    let mut sent = WorldPacket::from_bytes(&sent);
    assert_eq!(
        sent.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::LootRelease as u16
    );
    assert_eq!(sent.read_packed_guid().unwrap(), loot_guid);
    assert_eq!(sent.read_packed_guid().unwrap(), player_guid);
    assert!(!session.loot_table.get(&loot_guid).unwrap().items[0].taken);
    assert!(session.is_active_loot_guid(loot_guid));
}
#[tokio::test]
async fn loot_item_gameobject_too_far_uses_cpp_release() {
    let (mut session, send_rx) = make_session_with_send();
    let player_guid = ObjectGuid::create_player(1, 42);
    let loot_guid = test_gameobject_guid(19_029);
    let go_position = Position::new(6.0, 0.0, 0.0, 0.0);
    session.set_player_guid(Some(player_guid));
    session.set_player_position_like_cpp(Position::ZERO);
    session.set_active_loot_guid(loot_guid);
    attach_canonical_map_object(
        &mut session,
        AccessorObjectKind::GameObject,
        canonical_world_object(loot_guid, 0, go_position),
    );
    session.record_represented_gameobject_runtime_state_like_cpp(
        0,
        loot_guid,
        loot_guid.entry(),
        go_position,
        GAMEOBJECT_TYPE_CHEST as u8,
    );
    session.loot_table.insert(
        loot_guid,
        CreatureLoot {
            loot_guid,
            coins: 0,
            unlooted_count: 0,
            loot_type: LOOT_TYPE_CHEST_LIKE_CPP,
            dungeon_encounter_id: 0,
            loot_method: 0,
            loot_master: ObjectGuid::EMPTY,
            round_robin_player: ObjectGuid::EMPTY,
            player_ffa_items: Vec::new(),
            players_looting: Vec::new(),
            allowed_looters: Vec::new(),
            items: vec![LootEntry {
                loot_list_id: 0,
                item_id: 25,
                quantity: 1,
                random_properties_id: 0,
                random_properties_seed: 0,
                item_context: 0,
                flags: LootEntryFlags::default(),
                allowed_looters: vec![player_guid],
                roll_winner: ObjectGuid::EMPTY,
                ffa_looted_by: Vec::new(),
                taken: false,
            }],
            looted_by_player: false,
        },
    );

    session
        .handle_loot_item(loot_item_packet(loot_guid, 0))
        .await;

    let sent = send_rx.try_recv().unwrap();
    let mut sent = WorldPacket::from_bytes(&sent);
    assert_eq!(
        sent.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::LootRelease as u16
    );
    assert_eq!(sent.read_packed_guid().unwrap(), loot_guid);
    assert_eq!(sent.read_packed_guid().unwrap(), player_guid);
    assert!(!session.loot_table.get(&loot_guid).unwrap().items[0].taken);
    assert!(session.is_active_loot_guid(loot_guid));
}
