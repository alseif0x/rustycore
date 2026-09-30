//! Original application cases using the feature fixtures and production operations.
use super::support::*;

#[tokio::test]
async fn process_pending_shared_chest_restock_clears_loot_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send_capacity(4);
    let chest_guid = test_gameobject_guid(19_043);
    session.set_state(SessionState::LoggedIn);
    record_represented_gameobject_runtime_state_for_test(
        &mut session,
        0,
        chest_guid,
        chest_guid.entry(),
        Position::ZERO,
        GAMEOBJECT_TYPE_CHEST as u8,
    );
    mark_chest_restock_expired_for_loot_test(&mut session, chest_guid, LootState::Activated);
    set_loot_for_test(
        &mut session,
        chest_guid,
        CreatureLoot {
            loot_guid: chest_guid,
            coins: 7,
            unlooted_count: 1,
            loot_type: LOOT_TYPE_CHEST_LIKE_CPP,
            dungeon_encounter_id: 0,
            loot_method: 0,
            loot_master: ObjectGuid::EMPTY,
            round_robin_player: ObjectGuid::EMPTY,
            player_ffa_items: Vec::new(),
            players_looting: Vec::new(),
            allowed_looters: Vec::new(),
            items: Vec::new(),
            looted_by_player: false,
        },
    );

    process_pending_for_loot_test(&mut session).await;

    let state = gameobject_loot_release_snapshot_for_test(&session, chest_guid).unwrap();
    assert_eq!(state.loot_state, Some(LootState::Ready));
    assert!(state.chest_restock_until.is_none());
    assert!(!has_loot_for_test(&session, chest_guid));
}

#[tokio::test]
async fn process_pending_shared_chest_restock_syncs_state_to_same_map_viewers_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send_capacity(4);
    let player_guid = ObjectGuid::create_player(1, 42);
    let same_map_guid = ObjectGuid::create_player(1, 77);
    let chest_guid = test_gameobject_guid(19_044);
    let (same_command_tx, same_command_rx) = flume::bounded(2);
    let (same_send_tx, _same_send_rx) = flume::bounded::<Vec<u8>>(1);
    let player_registry = Arc::new(PlayerRegistry::default());
    let mut same_info = broadcast_info(same_map_guid, same_send_tx);
    same_info.placement.map_id = 571;
    same_info.command_tx = same_command_tx;
    player_registry.register_or_replace(same_map_guid, same_info, Default::default());

    session.set_state(SessionState::LoggedIn);
    session.set_player_guid(Some(player_guid));
    set_player_map_position_for_loot_test(&mut session, 571, Position::ZERO);
    session.set_player_registry(player_registry);
    record_represented_gameobject_runtime_state_for_test(
        &mut session,
        0,
        chest_guid,
        chest_guid.entry(),
        Position::ZERO,
        GAMEOBJECT_TYPE_CHEST as u8,
    );
    record_gameobject_chest_release_metadata_for_loot_test(
        &mut session,
        chest_guid,
        GameObjectLootSource {
            loot_id: 7_002,
            chest_restock_time_secs: 45,
            chest_consumable: false,
            ..Default::default()
        },
    );
    mark_chest_restock_expired_for_loot_test(&mut session, chest_guid, LootState::NotReady);
    set_loot_for_test(
        &mut session,
        chest_guid,
        CreatureLoot {
            loot_guid: chest_guid,
            coins: 7,
            unlooted_count: 1,
            loot_type: LOOT_TYPE_CHEST_LIKE_CPP,
            dungeon_encounter_id: 0,
            loot_method: 0,
            loot_master: ObjectGuid::EMPTY,
            round_robin_player: ObjectGuid::EMPTY,
            player_ffa_items: Vec::new(),
            players_looting: Vec::new(),
            allowed_looters: Vec::new(),
            items: Vec::new(),
            looted_by_player: false,
        },
    );

    process_pending_for_loot_test(&mut session).await;

    let command = match same_command_rx.try_recv() {
        Ok(SessionCommand::SyncChestGameobjectStateAndRefreshLikeCpp(command)) => command,
        other => panic!("expected restock chest sync command, got {other:?}"),
    };
    assert_eq!(command.gameobject_guid, chest_guid);
    assert_eq!(command.map_id, 571);
    assert_eq!(command.loot_state, Some(LootState::Ready as u8));
    assert_eq!(command.loot_state_unit_guid, ObjectGuid::EMPTY);
    assert_eq!(command.chest_loot_id, 7_002);
    assert_eq!(command.chest_restock_time_secs, 45);
}
