use super::support::*;

#[tokio::test]
async fn gameobject_loot_release_fishing_hole_uses_canonical_use_count_when_represented_stale_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    let player_guid = ObjectGuid::create_player(1, 42);
    let fishing_hole = test_gameobject_guid(19_136);
    let mut game_object = make_canonical_gameobject_for_loot_test(
        &session,
        fishing_hole,
        GAMEOBJECT_TYPE_FISHING_HOLE as u8,
    );
    game_object.add_use_like_cpp();
    attach_canonical_gameobject_for_loot_test(&mut session, game_object);
    session.set_player_guid(Some(player_guid));
    set_player_position_for_loot_test(&mut session, Position::ZERO);
    set_active_loot_guid_for_test(&mut session, fishing_hole);
    record_represented_gameobject_runtime_state_for_test(
        &mut session,
        0,
        fishing_hole,
        fishing_hole.entry(),
        Position::ZERO,
        GAMEOBJECT_TYPE_FISHING_HOLE as u8,
    );
    record_fishing_hole_max_opens_for_loot_test(&mut session, fishing_hole, 2);
    set_loot_for_test(
        &mut session,
        fishing_hole,
        CreatureLoot {
            loot_guid: fishing_hole,
            coins: 0,
            unlooted_count: 1,
            loot_type: LOOT_TYPE_FISHINGHOLE_LIKE_CPP,
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
        .handle_loot_release(loot_release_packet(fishing_hole))
        .await;

    assert!(send_rx.try_recv().is_ok());
    let canonical = canonical_gameobject_snapshot_for_loot_test(&session, fishing_hole).unwrap();
    assert_eq!(canonical.use_times(), 2);
    assert_eq!(canonical.loot_state(), LootState::JustDeactivated);
    let hole_state =
        gameobject_loot_release_snapshot_for_test(&session, fishing_hole).unwrap();
    assert_eq!(hole_state.personal_loot_uses, 2);
    assert_eq!(hole_state.loot_state, Some(LootState::JustDeactivated));
}

#[tokio::test]
async fn loot_item_fishing_hole_skips_gameobject_distance_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    let player_guid = ObjectGuid::create_player(1, 42);
    let loot_guid = test_gameobject_guid(19_030);
    let go_position = Position::new(100.0, 0.0, 0.0, 0.0);
    session.set_player_guid(Some(player_guid));
    set_player_position_for_loot_test(&mut session, Position::ZERO);
    set_active_loot_guid_like_cpp(&mut session, loot_guid);
    attach_canonical_map_object(
        &mut session,
        AccessorObjectKind::GameObject,
        canonical_world_object(loot_guid, 0, go_position),
    );
    record_represented_gameobject_runtime_state_for_test(&mut session,
        0,
        loot_guid,
        loot_guid.entry(),
        go_position,
        GAMEOBJECT_TYPE_FISHING_HOLE as u8,
    );
    set_loot_for_test(&mut session,
        loot_guid,
        CreatureLoot {
            loot_guid,
            coins: 0,
            unlooted_count: 0,
            loot_type: LOOT_TYPE_FISHINGHOLE_LIKE_CPP,
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
                flags: LootEntryFlags {
                    blocked: true,
                    ..Default::default()
                },
                allowed_looters: vec![player_guid],
                roll_winner: ObjectGuid::EMPTY,
                ffa_looted_by: Vec::new(),
                taken: false,
            }],
            looted_by_player: false,
        },
    );

    handle_loot_item_for_test(&mut session, loot_item_packet(loot_guid, 0))
        .await;

    let sent = send_rx.try_recv().unwrap();
    let mut sent = WorldPacket::from_bytes(&sent);
    assert_eq!(
        sent.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::LootReleaseAll as u16
    );
    assert_eq!(sent.remaining(), 0);
    assert!(!loot_for_test(&session, loot_guid).unwrap().items[0].taken);
    assert!(is_active_loot_guid_like_cpp(&session, loot_guid));
}

#[tokio::test]
async fn loot_item_owned_gameobject_skips_distance_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    let player_guid = ObjectGuid::create_player(1, 42);
    let loot_guid = test_gameobject_guid(19_035);
    let go_position = Position::new(100.0, 0.0, 0.0, 0.0);
    session.set_player_guid(Some(player_guid));
    set_player_position_for_loot_test(&mut session, Position::ZERO);
    set_active_loot_guid_like_cpp(&mut session, loot_guid);
    record_represented_gameobject_runtime_state_for_test(&mut session,
        0,
        loot_guid,
        loot_guid.entry(),
        go_position,
        GAMEOBJECT_TYPE_CHEST as u8,
    );
    record_gameobject_owner_like_cpp(&mut session, loot_guid, player_guid);
    set_loot_for_test(&mut session,
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
                flags: LootEntryFlags {
                    blocked: true,
                    ..Default::default()
                },
                allowed_looters: vec![player_guid],
                roll_winner: ObjectGuid::EMPTY,
                ffa_looted_by: Vec::new(),
                taken: false,
            }],
            looted_by_player: false,
        },
    );

    handle_loot_item_for_test(&mut session, loot_item_packet(loot_guid, 0))
        .await;

    let sent = send_rx.try_recv().unwrap();
    let mut sent = WorldPacket::from_bytes(&sent);
    assert_eq!(
        sent.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::LootReleaseAll as u16
    );
    assert_eq!(sent.remaining(), 0);
    assert!(!loot_for_test(&session, loot_guid).unwrap().items[0].taken);
    assert!(is_active_loot_guid_like_cpp(&session, loot_guid));
}

#[tokio::test]
async fn loot_item_owned_gameobject_skips_distance_from_canonical_created_by_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    let player_guid = ObjectGuid::create_player(1, 42);
    let loot_guid = test_gameobject_guid(19_036);
    let mut game_object =
        make_canonical_gameobject_for_session(&session, loot_guid, GAMEOBJECT_TYPE_CHEST as u8);
    game_object
        .world_mut()
        .relocate(Position::new(100.0, 0.0, 0.0, 0.0));
    game_object.set_created_by(player_guid);

    session.set_player_guid(Some(player_guid));
    set_player_position_for_loot_test(&mut session, Position::ZERO);
    set_active_loot_guid_like_cpp(&mut session, loot_guid);
    attach_canonical_gameobject(&mut session, game_object);
    set_loot_for_test(&mut session,
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
                flags: LootEntryFlags {
                    blocked: true,
                    ..Default::default()
                },
                allowed_looters: vec![player_guid],
                roll_winner: ObjectGuid::EMPTY,
                ffa_looted_by: Vec::new(),
                taken: false,
            }],
            looted_by_player: false,
        },
    );

    handle_loot_item_for_test(&mut session, loot_item_packet(loot_guid, 0))
        .await;

    let sent = send_rx.try_recv().unwrap();
    let mut sent = WorldPacket::from_bytes(&sent);
    assert_eq!(
        sent.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::LootReleaseAll as u16
    );
    assert_eq!(sent.remaining(), 0);
    assert!(!loot_for_test(&session, loot_guid).unwrap().items[0].taken);
    assert!(is_active_loot_guid_like_cpp(&session, loot_guid));
}

#[tokio::test]
async fn loot_release_keeps_unlooted_gameobject_loot_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    let player_guid = ObjectGuid::create_player(1, 42);
    let loot_guid = test_gameobject_guid(19_014);
    session.set_player_guid(Some(player_guid));
    set_active_loot_guid_like_cpp(&mut session, loot_guid);
    insert_client_visible_guid_for_test(&mut session, loot_guid);
    record_represented_gameobject_runtime_state_for_test(&mut session,
        0,
        loot_guid,
        loot_guid.entry(),
        Position::ZERO,
        GAMEOBJECT_TYPE_CHEST as u8,
    );
    set_loot_for_test(&mut session,
        loot_guid,
        CreatureLoot {
            loot_guid,
            coins: 0,
            unlooted_count: 1,
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
        .handle_loot_release(loot_release_packet(loot_guid))
        .await;

    let sent = send_rx.try_recv().unwrap();
    let mut sent = WorldPacket::from_bytes(&sent);
    assert_eq!(
        sent.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::LootRelease as u16
    );
    assert_eq!(sent.read_packed_guid().unwrap(), loot_guid);
    assert_eq!(sent.read_packed_guid().unwrap(), player_guid);
    assert!(!is_active_loot_guid_like_cpp(&session, loot_guid));
    assert!(!loot_for_test(&session, loot_guid).unwrap().items[0].taken);
    let state = represented_gameobject_loot_release_snapshot_like_cpp(&session, loot_guid).unwrap();
    assert_eq!(state.loot_state, Some(LootState::Activated));
    assert_eq!(state.loot_state_unit_guid, player_guid);
}

#[tokio::test]
async fn loot_release_gameobject_too_far_keeps_state_and_loot_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    let player_guid = ObjectGuid::create_player(1, 42);
    let loot_guid = test_gameobject_guid(19_031);
    let go_position = Position::new(6.0, 0.0, 0.0, 0.0);
    session.set_player_guid(Some(player_guid));
    set_player_position_for_loot_test(&mut session, Position::ZERO);
    set_active_loot_guid_like_cpp(&mut session, loot_guid);
    record_represented_gameobject_runtime_state_for_test(&mut session,
        0,
        loot_guid,
        loot_guid.entry(),
        go_position,
        GAMEOBJECT_TYPE_CHEST as u8,
    );
    set_loot_for_test(&mut session,
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
            items: Vec::new(),
            looted_by_player: false,
        },
    );

    session
        .handle_loot_release(loot_release_packet(loot_guid))
        .await;

    let sent = send_rx.try_recv().unwrap();
    let mut sent = WorldPacket::from_bytes(&sent);
    assert_eq!(
        sent.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::LootRelease as u16
    );
    assert_eq!(sent.read_packed_guid().unwrap(), loot_guid);
    assert_eq!(sent.read_packed_guid().unwrap(), player_guid);
    assert!(!is_active_loot_guid_like_cpp(&session, loot_guid));
    assert!(has_loot_for_test(&session, loot_guid));
    assert_eq!(
        represented_gameobject_loot_release_snapshot_like_cpp(&session, loot_guid).unwrap()
            .loot_state,
        None
    );
}

#[tokio::test]
async fn loot_release_owned_gameobject_skips_distance_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    let player_guid = ObjectGuid::create_player(1, 42);
    let loot_guid = test_gameobject_guid(19_036);
    let go_position = Position::new(100.0, 0.0, 0.0, 0.0);
    session.set_player_guid(Some(player_guid));
    set_player_position_for_loot_test(&mut session, Position::ZERO);
    set_active_loot_guid_like_cpp(&mut session, loot_guid);
    record_represented_gameobject_runtime_state_for_test(&mut session,
        0,
        loot_guid,
        loot_guid.entry(),
        go_position,
        GAMEOBJECT_TYPE_CHEST as u8,
    );
    record_gameobject_owner_like_cpp(&mut session, loot_guid, player_guid);
    set_loot_for_test(&mut session,
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
            items: Vec::new(),
            looted_by_player: false,
        },
    );

    session
        .handle_loot_release(loot_release_packet(loot_guid))
        .await;

    let sent = send_rx.try_recv().unwrap();
    let mut sent = WorldPacket::from_bytes(&sent);
    assert_eq!(
        sent.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::LootRelease as u16
    );
    assert!(!has_loot_for_test(&session, loot_guid));
    assert_eq!(
        represented_gameobject_loot_release_snapshot_like_cpp(&session, loot_guid).unwrap()
            .loot_state,
        Some(LootState::JustDeactivated)
    );
}

#[tokio::test]
async fn loot_release_fully_looted_gameobject_just_deactivates_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    let player_guid = ObjectGuid::create_player(1, 42);
    let loot_guid = test_gameobject_guid(19_032);
    session.set_player_guid(Some(player_guid));
    set_player_position_for_loot_test(&mut session, Position::ZERO);
    set_active_loot_guid_like_cpp(&mut session, loot_guid);
    record_represented_gameobject_runtime_state_for_test(&mut session,
        0,
        loot_guid,
        loot_guid.entry(),
        Position::ZERO,
        GAMEOBJECT_TYPE_CHEST as u8,
    );
    set_loot_for_test(&mut session,
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
            items: Vec::new(),
            looted_by_player: false,
        },
    );

    session
        .handle_loot_release(loot_release_packet(loot_guid))
        .await;

    let sent = send_rx.try_recv().unwrap();
    let mut sent = WorldPacket::from_bytes(&sent);
    assert_eq!(
        sent.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::LootRelease as u16
    );
    assert!(!has_loot_for_test(&session, loot_guid));
    assert_eq!(
        represented_gameobject_loot_release_snapshot_like_cpp(&session, loot_guid).unwrap()
            .loot_state,
        Some(LootState::JustDeactivated)
    );
}

#[tokio::test]
async fn gameobject_owned_loot_release_partial_chest_uses_canonical_is_fully_looted_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    let player_guid = ObjectGuid::create_player(1, 42);
    let loot_guid = test_gameobject_guid(19_132);
    let mut game_object =
        make_canonical_gameobject_for_session(&session, loot_guid, GAMEOBJECT_TYPE_CHEST as u8);
    game_object.set_personal_loot_like_cpp(player_guid, GameObjectOwnedLoot::new(0, 1));
    attach_canonical_gameobject(&mut session, game_object);
    session.set_player_guid(Some(player_guid));
    set_player_position_for_loot_test(&mut session, Position::ZERO);
    set_active_loot_guid_like_cpp(&mut session, loot_guid);
    record_represented_gameobject_runtime_state_for_test(&mut session,
        0,
        loot_guid,
        loot_guid.entry(),
        Position::ZERO,
        GAMEOBJECT_TYPE_CHEST as u8,
    );
    record_gameobject_chest_release_metadata_like_cpp(&mut session, 
        loot_guid,
        GameObjectLootSource {
            chest_consumable: false,
            chest_restock_time_secs: 7,
            ..Default::default()
        },
    );
    set_loot_for_test(&mut session,
        loot_guid,
        CreatureLoot {
            loot_guid: represented_loot_object_guid_like_cpp(loot_guid),
            coins: 0,
            unlooted_count: 1,
            loot_type: LOOT_TYPE_CHEST_LIKE_CPP,
            dungeon_encounter_id: 0,
            loot_method: 0,
            loot_master: ObjectGuid::EMPTY,
            round_robin_player: ObjectGuid::EMPTY,
            player_ffa_items: Vec::new(),
            players_looting: vec![player_guid],
            allowed_looters: vec![player_guid],
            items: vec![represented_loot_entry_for_test(0, 25, player_guid)],
            looted_by_player: false,
        },
    );

    session
        .handle_loot_release(loot_release_packet(loot_guid))
        .await;

    assert!(send_rx.try_recv().is_ok());
    let canonical = canonical_gameobject_snapshot(&session, loot_guid).unwrap();
    assert_eq!(
        canonical.shared_loot_like_cpp(),
        Some(&GameObjectOwnedLoot::new(0, 1))
    );
    assert_eq!(canonical.personal_loot_count_like_cpp(), 0);
    assert_eq!(
        canonical.loot_for_player_like_cpp(player_guid),
        Some(&GameObjectOwnedLoot::new(0, 1))
    );
    assert!(!canonical.is_fully_looted_like_cpp());
    assert_eq!(canonical.loot_state(), LootState::Activated);
    assert_eq!(canonical.loot_state_unit_guid(), player_guid);
    assert!(canonical.restock_time() > 0);
    assert!(!has_loot_for_test(&session, loot_guid));
    assert!(reconcile_represented_loot_cache_like_cpp(&mut session, loot_guid, player_guid));
    assert!(has_loot_for_test(&session, loot_guid));
    assert_eq!(
        represented_gameobject_loot_release_snapshot_like_cpp(&session, loot_guid).unwrap()
            .loot_state,
        Some(LootState::Activated)
    );
}

#[tokio::test]
async fn loot_release_partial_chest_syncs_state_to_same_map_viewers_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    let player_guid = ObjectGuid::create_player(1, 42);
    let same_map_guid = ObjectGuid::create_player(1, 77);
    let loot_guid = test_gameobject_guid(19_138);
    let (same_command_tx, same_command_rx) = flume::bounded(2);
    let (same_send_tx, _same_send_rx) = flume::bounded::<Vec<u8>>(1);
    let player_registry = Arc::new(PlayerRegistry::default());
    let mut same_info = broadcast_info(same_map_guid, same_send_tx);
    same_info.placement.map_id = 571;
    same_info.command_tx = same_command_tx;
    player_registry.register_or_replace(same_map_guid, same_info, Default::default());

    session.set_player_registry(player_registry);
    session.set_player_guid(Some(player_guid));
    set_player_position_for_loot_test(&mut session, Position::ZERO);
    set_player_map_position_for_loot_test(&mut session, 571, Position::ZERO);
    set_active_loot_guid_like_cpp(&mut session, loot_guid);
    record_represented_gameobject_runtime_state_for_test(&mut session,
        0,
        loot_guid,
        loot_guid.entry(),
        Position::ZERO,
        GAMEOBJECT_TYPE_CHEST as u8,
    );
    record_gameobject_chest_release_metadata_like_cpp(&mut session, 
        loot_guid,
        GameObjectLootSource {
            loot_id: 7_001,
            chest_restock_time_secs: 45,
            chest_consumable: false,
            ..Default::default()
        },
    );
    set_loot_for_test(&mut session,
        loot_guid,
        CreatureLoot {
            loot_guid: represented_loot_object_guid_like_cpp(loot_guid),
            coins: 0,
            unlooted_count: 1,
            loot_type: LOOT_TYPE_CHEST_LIKE_CPP,
            dungeon_encounter_id: 0,
            loot_method: 0,
            loot_master: ObjectGuid::EMPTY,
            round_robin_player: ObjectGuid::EMPTY,
            player_ffa_items: Vec::new(),
            players_looting: vec![player_guid],
            allowed_looters: Vec::new(),
            items: vec![LootEntry {
                loot_list_id: 0,
                item_id: 25,
                quantity: 1,
                random_properties_id: 0,
                random_properties_seed: 0,
                item_context: 0,
                flags: LootEntryFlags::default(),
                allowed_looters: Vec::new(),
                roll_winner: ObjectGuid::EMPTY,
                ffa_looted_by: Vec::new(),
                taken: false,
            }],
            looted_by_player: false,
        },
    );

    session
        .handle_loot_release(loot_release_packet(loot_guid))
        .await;

    let release_bytes = send_rx.try_recv().unwrap();
    let mut release = WorldPacket::from_bytes(&release_bytes);
    assert_eq!(
        release.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::LootRelease as u16
    );
    let command = match same_command_rx.try_recv() {
        Ok(SessionCommand::SyncChestGameobjectStateAndRefreshLikeCpp(command)) => command,
        other => panic!("expected chest release sync command, got {other:?}"),
    };
    assert_eq!(command.gameobject_guid, loot_guid);
    assert_eq!(command.map_id, 571);
    assert_eq!(
        command.loot_state,
        Some(wow_entities::LootState::Activated as u8)
    );
    assert_eq!(command.loot_state_unit_guid, player_guid);
    assert_eq!(command.chest_loot_id, 7_001);
    assert_eq!(command.chest_restock_time_secs, 45);
}

#[tokio::test]
async fn gameobject_owned_loot_release_fully_consumed_chest_uses_canonical_is_fully_looted_like_cpp()
 {
    let (mut session, send_rx) = make_session_with_send();
    let player_guid = ObjectGuid::create_player(1, 42);
    let loot_guid = test_gameobject_guid(19_133);
    let mut game_object =
        make_canonical_gameobject_for_session(&session, loot_guid, GAMEOBJECT_TYPE_CHEST as u8);
    game_object.set_shared_loot_like_cpp(GameObjectOwnedLoot::new(0, 1));
    attach_canonical_gameobject(&mut session, game_object);
    session.set_player_guid(Some(player_guid));
    set_player_position_for_loot_test(&mut session, Position::ZERO);
    set_active_loot_guid_like_cpp(&mut session, loot_guid);
    record_represented_gameobject_runtime_state_for_test(&mut session,
        0,
        loot_guid,
        loot_guid.entry(),
        Position::ZERO,
        GAMEOBJECT_TYPE_CHEST as u8,
    );
    record_gameobject_chest_release_metadata_like_cpp(&mut session, 
        loot_guid,
        GameObjectLootSource {
            chest_consumable: false,
            chest_restock_time_secs: 7,
            ..Default::default()
        },
    );
    set_loot_for_test(&mut session,
        loot_guid,
        CreatureLoot {
            loot_guid: represented_loot_object_guid_like_cpp(loot_guid),
            coins: 0,
            unlooted_count: 0,
            loot_type: LOOT_TYPE_CHEST_LIKE_CPP,
            dungeon_encounter_id: 0,
            loot_method: 0,
            loot_master: ObjectGuid::EMPTY,
            round_robin_player: ObjectGuid::EMPTY,
            player_ffa_items: Vec::new(),
            players_looting: vec![player_guid],
            allowed_looters: vec![player_guid],
            items: Vec::new(),
            looted_by_player: false,
        },
    );

    session
        .handle_loot_release(loot_release_packet(loot_guid))
        .await;

    assert!(send_rx.try_recv().is_ok());
    let canonical = canonical_gameobject_snapshot(&session, loot_guid).unwrap();
    assert_eq!(
        canonical.shared_loot_like_cpp(),
        Some(&GameObjectOwnedLoot::default())
    );
    assert!(canonical.is_fully_looted_like_cpp());
    assert_eq!(canonical.loot_state(), LootState::JustDeactivated);
    assert_eq!(canonical.loot_state_unit_guid(), ObjectGuid::EMPTY);
    assert_eq!(canonical.restock_time(), 0);
    assert!(!has_loot_for_test(&session, loot_guid));
    assert_eq!(
        represented_gameobject_loot_release_snapshot_like_cpp(&session, loot_guid).unwrap()
            .loot_state,
        Some(LootState::JustDeactivated)
    );
}

#[tokio::test]
async fn gameobject_owned_loot_release_personal_chest_syncs_current_player_and_despawns_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(2);
    let player_guid = ObjectGuid::create_player(1, 42);
    let loot_guid = test_gameobject_guid(19_134);
    let game_object =
        make_canonical_gameobject_for_session(&session, loot_guid, GAMEOBJECT_TYPE_CHEST as u8);
    attach_canonical_gameobject(&mut session, game_object);
    session.set_player_guid(Some(player_guid));
    set_player_position_for_loot_test(&mut session, Position::ZERO);
    set_active_loot_guid_like_cpp(&mut session, loot_guid);
    record_represented_gameobject_runtime_state_for_test(&mut session,
        0,
        loot_guid,
        loot_guid.entry(),
        Position::ZERO,
        GAMEOBJECT_TYPE_CHEST as u8,
    );
    record_gameobject_chest_release_metadata_like_cpp(&mut session, 
        loot_guid,
        GameObjectLootSource {
            personal_loot_id: 55,
            chest_consumable: false,
            chest_restock_time_secs: 7,
            ..Default::default()
        },
    );
    set_personal_loot_like_cpp(&mut session, loot_guid, player_guid, 0);
    set_loot_for_test(&mut session,
        loot_guid,
        CreatureLoot {
            loot_guid: represented_loot_object_guid_like_cpp(loot_guid),
            coins: 0,
            unlooted_count: 0,
            loot_type: LOOT_TYPE_CHEST_LIKE_CPP,
            dungeon_encounter_id: 1,
            loot_method: 0,
            loot_master: ObjectGuid::EMPTY,
            round_robin_player: ObjectGuid::EMPTY,
            player_ffa_items: Vec::new(),
            players_looting: vec![player_guid],
            allowed_looters: vec![player_guid],
            items: vec![LootEntry {
                taken: true,
                ..represented_loot_entry_for_test(0, 25, player_guid)
            }],
            looted_by_player: false,
        },
    );

    session
        .handle_loot_release(loot_release_packet(loot_guid))
        .await;

    assert!(send_rx.try_recv().is_ok());
    let canonical = canonical_gameobject_snapshot(&session, loot_guid).unwrap();
    assert_eq!(canonical.shared_loot_like_cpp(), None);
    assert_eq!(
        canonical.personal_loot_like_cpp(player_guid),
        Some(&GameObjectOwnedLoot::default())
    );
    let state = represented_gameobject_loot_release_snapshot_like_cpp(&session, loot_guid).unwrap();
    assert_eq!(state.per_player_state_player_guid, Some(player_guid));
    assert_eq!(state.per_player_despawn_secs, Some(7));
    assert!(state.per_player_despawn_until.is_some());
}
