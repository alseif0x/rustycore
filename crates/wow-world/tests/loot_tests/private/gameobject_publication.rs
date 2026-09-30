//! Original loot application cases with explicit operation-local fixture permission.
use super::interaction_support::*;

#[tokio::test]
async fn represented_gameobject_personal_encounter_open_does_not_auto_allow_non_tapper_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    let player_guid = ObjectGuid::create_player(1, 42);
    let other_tapper = ObjectGuid::create_player(1, 77);
    let gameobject_guid = test_gameobject_guid(91_011);
    session.set_player_guid(Some(player_guid));
    wow_world::test_fixtures::insert_client_visible_guid_for_test(&mut session, gameobject_guid);
    set_loot_gameobject_tappers_for_test(&mut session, gameobject_guid, vec![other_tapper]);
    let source = GameObjectLootSource {
        loot_id: 0,
        use_group_loot_rules: false,
        dungeon_encounter_id: 733,
        personal_loot_id: 10_001,
        push_loot_id: 0,
        triggered_event_id: 0,
        linked_trap_entry: 0,
        ..Default::default()
    };

    prepare_money_player_residence_for_test(&mut session);
    open_gameobject_loot_cycle_for_test(&mut session, gameobject_guid, source).await;

    assert!(send_rx.try_recv().is_err());
    assert!(!is_active_loot_guid_for_test(&session, gameobject_guid));
    assert_eq!(
        session
            .loot_table
            .get(&gameobject_guid)
            .unwrap()
            .allowed_looters,
        vec![other_tapper]
    );
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
    wow_world::test_fixtures::insert_client_visible_guid_for_test(&mut session, gameobject_guid);

    prepare_money_player_residence_for_test(&mut session);
    open_gameobject_loot_cycle_for_test(&mut session, gameobject_guid, source).await;

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
async fn represented_gameobject_chest_use_sets_activated_loot_state_like_cpp() {
    let mut session = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let gameobject_guid = test_gameobject_guid(91_006);
    session.set_player_guid(Some(player_guid));
    wow_world::test_fixtures::insert_client_visible_guid_for_test(&mut session, gameobject_guid);

    prepare_money_player_residence_for_test(&mut session);
    open_gameobject_loot_cycle_for_test(
        &mut session,
        gameobject_guid,
        GameObjectLootSource::default(),
    )
    .await;

    let state = gameobject_loot_release_snapshot_for_test(&session, gameobject_guid)
        .expect("represented chest use records GO loot state");
    assert_eq!(state.loot_state, Some(LootState::Activated));
    assert_eq!(state.loot_state_unit_guid, player_guid);
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
    wow_world::test_fixtures::insert_client_visible_guid_for_test(&mut session, gameobject_guid);

    let source = GatheringNodeUseSource {
        loot_id: 0,
        despawn_delay_secs: 0,
        triggered_event_id: 0,
        xp_difficulty: 0,
        spell_id: 0,
        max_loots: 1,
        linked_trap_entry: 0,
    };

    prepare_money_player_residence_for_test(&mut session);
    open_gathering_loot_cycle_for_test(&mut session, gameobject_guid, 190_008, source).await;

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
