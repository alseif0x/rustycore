//! Session scenarios exercising the represented instances responsibility.
//!
//! Split out of session_tests.rs under #626; assertions and registrations
//! are unchanged and the shared fixtures stay in the parent module.

use super::*;

#[test]
fn represented_failed_map_difficulty_x_condition_matches_cpp_first_failed_order() {
    let (mut session, _, _) = make_session();
    session.player_class = 1;
    session.set_player_condition_store(Arc::new(wow_data::PlayerConditionStore::from_entries([
        wow_data::PlayerConditionEntry {
            id: 42,
            class_mask: 1,
            ..Default::default()
        },
        wow_data::PlayerConditionEntry {
            id: 43,
            class_mask: 1 << 1,
            ..Default::default()
        },
    ])));
    session.set_map_difficulty_x_condition_store(Arc::new(
        wow_data::MapDifficultyXConditionStore::from_entries([
            wow_data::MapDifficultyXConditionEntry {
                id: 100,
                failure_description: String::new(),
                player_condition_id: 43,
                order_index: 20,
                map_difficulty_id: 7,
            },
            wow_data::MapDifficultyXConditionEntry {
                id: 101,
                failure_description: String::new(),
                player_condition_id: 42,
                order_index: 10,
                map_difficulty_id: 7,
            },
        ]),
    ));

    assert_eq!(
        session.represented_failed_map_difficulty_x_condition_like_cpp(7),
        Some(100)
    );
    assert_eq!(
        session.represented_failed_map_difficulty_x_condition_like_cpp(8),
        None
    );
}
#[test]
fn mmap_runtime_config_matches_cpp_pathfinding_gate() {
    let mut config = MMapRuntimeConfigLikeCpp::default();
    config.disabled_map_ids.insert(571);

    assert!(config.should_try_pathfinding_like_cpp(0, false));
    assert!(!config.should_try_pathfinding_like_cpp(571, false));
    assert!(!config.should_try_pathfinding_like_cpp(0, true));

    config.enabled = false;
    assert!(!config.should_try_pathfinding_like_cpp(0, false));
}
#[test]
fn canonical_visibility_uses_player_instance() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 49_600);
    let position = Position::new(100.0, 200.0, 30.0, 0.0);
    let instance_guid = test_gameobject_guid(49_601, 49_601);
    let default_instance_guid = test_gameobject_guid(49_600, 49_600);

    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "InstanceOwner".to_string(),
        position,
        571,
        1,
        1,
        80,
        0,
    ));
    add_canonical_test_player_on_map(&canonical, player_guid, position, 571, 7);
    add_canonical_test_gameobject_on_map(
        &canonical,
        default_instance_guid,
        49_600,
        position,
        571,
        0,
    );
    add_canonical_test_gameobject_on_map(&canonical, instance_guid, 49_601, position, 571, 7);
    session.represented_gameobject_use_states.insert(
        default_instance_guid,
        RepresentedGameObjectUseState {
            display_id: Some(7_600),
            go_type: Some(3),
            map_id: Some(571),
            position: Some(position),
            ..Default::default()
        },
    );
    session.represented_gameobject_use_states.insert(
        instance_guid,
        RepresentedGameObjectUseState {
            display_id: Some(7_601),
            go_type: Some(3),
            map_id: Some(571),
            position: Some(position),
            ..Default::default()
        },
    );

    let visible = session
        .visible_gameobjects_from_canonical_map_like_cpp(571, &position, 100.0)
        .expect("typed canonical player should select the player's map instance");

    assert_eq!(visible.len(), 1);
    assert_eq!(visible[0].guid, instance_guid);
    assert_eq!(visible[0].entry, 49_601);
}
#[test]
fn dynamic_object_visibility_uses_player_instance() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 49_610);
    let position = Position::new(100.0, 200.0, 30.0, 0.0);
    let instance_guid = test_dynamic_object_guid(49_611, 49_611);
    let default_instance_guid = test_dynamic_object_guid(49_610, 49_610);

    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "InstanceOwner".to_string(),
        position,
        571,
        1,
        1,
        80,
        0,
    ));
    add_canonical_test_player_on_map(&canonical, player_guid, position, 571, 7);
    add_canonical_test_dynamic_object_on_map(
        &canonical,
        default_instance_guid,
        player_guid,
        49_610,
        position,
        571,
        0,
    );
    add_canonical_test_dynamic_object_on_map(
        &canonical,
        instance_guid,
        player_guid,
        49_611,
        position,
        571,
        7,
    );

    let visible = session
        .visible_dynamic_objects_from_canonical_map_like_cpp(571, &position, 100.0)
        .expect("typed canonical player should select the player's map instance");

    assert_eq!(visible.len(), 1);
    assert_eq!(visible[0].guid, instance_guid);
    assert_eq!(visible[0].entry_id, 49_611);
}
#[tokio::test]
async fn dynamic_object_values_snapshot_visible_emits_once_after_map_clear_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let canonical = Arc::new(std::sync::Mutex::new(wow_map::MapManager::new(60_000, 1)));
    let player_guid = ObjectGuid::create_player(1, 50_401);
    let dynamic_guid = test_dynamic_object_guid(601001, 50_402);

    configure_dynamic_object_values_snapshot_session_like_cpp(
        &mut session,
        &canonical,
        player_guid,
        571,
        7,
    );
    add_canonical_test_dynamic_object_on_map(
        &canonical,
        dynamic_guid,
        player_guid,
        601001,
        Position::new(11.0, 21.0, 31.0, 0.0),
        571,
        7,
    );
    prepare_dynamic_object_values_snapshot_like_cpp(&canonical, 571, 7, dynamic_guid, 37.5);
    session.client_visible_guids_like_cpp.insert(dynamic_guid);

    session.process_pending().await;

    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::UpdateObject]
    );
}
#[tokio::test]
async fn dynamic_object_values_snapshot_uses_canonical_map_not_legacy_session_map_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let canonical = Arc::new(std::sync::Mutex::new(wow_map::MapManager::new(60_000, 1)));
    let player_guid = ObjectGuid::create_player(1, 50_451);
    let dynamic_guid = test_dynamic_object_guid(601051, 50_452);
    let canonical_map_id = 1;
    let instance_id = 7;

    configure_dynamic_object_values_snapshot_session_like_cpp(
        &mut session,
        &canonical,
        player_guid,
        canonical_map_id,
        instance_id,
    );
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "DynamicObjectViewer".to_string(),
        Position::new(10.0, 20.0, 30.0, 0.0),
        571,
        1,
        1,
        80,
        0,
    ));
    assert_eq!(session.player_map_id_like_cpp(), 571);
    add_canonical_test_dynamic_object_on_map(
        &canonical,
        dynamic_guid,
        player_guid,
        601051,
        Position::new(11.0, 21.0, 31.0, 0.0),
        canonical_map_id,
        instance_id,
    );
    prepare_dynamic_object_values_snapshot_like_cpp(
        &canonical,
        canonical_map_id,
        instance_id,
        dynamic_guid,
        37.5,
    );
    session.client_visible_guids_like_cpp.insert(dynamic_guid);
    let expected_packet = {
        let guard = canonical.lock().unwrap();
        let summary = guard
            .find_map(canonical_map_id, instance_id)
            .unwrap()
            .last_send_object_updates_summary_like_cpp();
        let represented_update = summary.dynamic_object_values_updates.first().unwrap();
        dynamic_object_values_update_to_update_object(
            dynamic_guid,
            u16::try_from(canonical_map_id).unwrap(),
            &represented_update.values_update,
        )
        .unwrap()
        .to_bytes()
    };

    session.process_pending().await;

    assert_eq!(drain_server_packet_bytes(&send_rx), vec![expected_packet]);
}
#[tokio::test]
async fn dynamic_object_values_snapshot_same_map_wrong_instance_not_consumed_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let canonical = Arc::new(std::sync::Mutex::new(wow_map::MapManager::new(60_000, 1)));
    let player_guid = ObjectGuid::create_player(1, 50_431);
    let dynamic_guid = test_dynamic_object_guid(601031, 50_432);

    configure_dynamic_object_values_snapshot_session_like_cpp(
        &mut session,
        &canonical,
        player_guid,
        571,
        7,
    );
    add_canonical_test_dynamic_object_on_map(
        &canonical,
        dynamic_guid,
        player_guid,
        601031,
        Position::new(11.0, 21.0, 31.0, 0.0),
        571,
        0,
    );
    prepare_dynamic_object_values_snapshot_like_cpp(&canonical, 571, 0, dynamic_guid, 37.5);
    session.client_visible_guids_like_cpp.insert(dynamic_guid);

    session.process_pending().await;

    assert!(drain_server_packet_bytes(&send_rx).is_empty());
}
#[test]
fn set_viewpoint_target_visibility_wrong_instance_does_not_fabricate_like_cpp() {
    let (mut session, _pkt_tx, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 50_020);
    let dynamic_object_guid = test_dynamic_object_guid(50_020, 50_020);
    let player_position = Position::new(10.0, 20.0, 30.0, 0.0);
    let dynamic_object_position = Position::new(10_000.0, 20_000.0, 30.0, 1.5);

    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "Farseer".to_string(),
        player_position,
        571,
        1,
        1,
        80,
        0,
    ));
    add_canonical_test_player_on_map(&canonical, player_guid, player_position, 571, 7);
    add_canonical_test_dynamic_object_on_map(
        &canonical,
        dynamic_object_guid,
        player_guid,
        50_020,
        dynamic_object_position,
        571,
        0,
    );

    assert!(!session.send_set_viewpoint_target_visibility_like_cpp(dynamic_object_guid));
    assert!(drain_server_packet_bytes(&send_rx).is_empty());
    assert!(
        !session
            .client_visible_guids_like_cpp
            .contains(&dynamic_object_guid),
        "wrong-instance/missing canonical target must not insert client visibility"
    );
}
#[tokio::test]
async fn update_visibility_uses_map_sources_without_world_db_like_cpp() {
    let (mut session, _pkt_tx, send_rx) = make_session();
    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 74_312);
    let player_position = Position::new(10.0, 10.0, 0.0, 0.0);
    let creature_guid = test_creature_guid(40);
    let gameobject_guid = test_gameobject_guid(912, 41);
    let dynamic_object_guid = test_dynamic_object_guid(913, 45);
    let area_trigger_guid = test_area_trigger_guid(914, 46);
    let corpse_guid =
        ObjectGuid::create_world_object(wow_core::guid::HighGuid::Corpse, 0, 1, 571, 0, 501, 42);
    let scene_object_guid = ObjectGuid::create_world_object(
        wow_core::guid::HighGuid::SceneObject,
        0,
        1,
        571,
        0,
        502,
        43,
    );
    let conversation_guid = ObjectGuid::create_world_object(
        wow_core::guid::HighGuid::Conversation,
        0,
        1,
        571,
        0,
        503,
        44,
    );
    let other_player_guid = ObjectGuid::create_player(1, 74_313);
    let registry = Arc::new(PlayerRegistry::default());

    session.set_map_manager(Arc::clone(&manager));
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_player_registry(Arc::clone(&registry));
    session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
        wow_data::MapEntry {
            id: 571,
            instance_type: wow_data::map::MAP_COMMON,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
    ])));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "MapVisibility".to_string(),
        player_position,
        571,
        1,
        1,
        80,
        0,
    ));
    session
        .ensure_canonical_world_map_for_current_player_like_cpp()
        .expect("visibility fixture should install its canonical Player owner");
    session.apply_move_init_active_mover_complete_like_cpp(0);
    let _ = drain_server_packet_bytes(&send_rx);

    let creature_position = Position::new(20.0, 20.0, 0.0, 0.0);
    let (grid_x, grid_y) =
        crate::map_manager::world_to_grid_coords(creature_position.x, creature_position.y);
    manager.write().unwrap().add_creature(
        571,
        0,
        grid_x,
        grid_y,
        crate::map_manager::WorldCreature::new(
            creature_guid,
            901,
            creature_position,
            100,
            80,
            1,
            2,
            0.0,
            1,
            35,
            0,
            0,
        ),
    );

    canonical.lock().unwrap().create_world_map(571, 0);
    session.record_represented_gameobject_runtime_state_like_cpp(
        571,
        gameobject_guid,
        912,
        Position::new(30.0, 30.0, 0.0, 0.0),
        3,
    );
    session.record_represented_gameobject_display_model_like_cpp(
        gameobject_guid,
        7002,
        1.0,
        [0.0, 0.0, 0.0, 1.0],
    );
    add_canonical_visibility_misc_objects_on_map(
        &canonical,
        corpse_guid,
        scene_object_guid,
        conversation_guid,
        Position::new(40.0, 40.0, 0.0, 0.0),
        571,
        0,
    );
    add_canonical_test_dynamic_object_on_map(
        &canonical,
        dynamic_object_guid,
        player_guid,
        913,
        Position::new(42.0, 42.0, 0.0, 0.0),
        571,
        0,
    );
    add_canonical_test_area_trigger_on_map(
        &canonical,
        area_trigger_guid,
        player_guid,
        914,
        Position::new(44.0, 44.0, 0.0, 0.0),
        571,
        0,
    );
    let (other_tx, _other_rx) = flume::bounded(1);
    let mut other_info = broadcast_info(other_player_guid, other_tx);
    other_info.placement.map_id = 571;
    other_info.placement.position = Position::new(50.0, 50.0, 0.0, 0.0);
    add_canonical_test_player_on_map(
        &canonical,
        other_player_guid,
        other_info.placement.position,
        571,
        0,
    );
    registry.register_or_replace(other_player_guid, other_info, Default::default());

    session.update_visibility().await;

    for guid in [
        creature_guid,
        gameobject_guid,
        dynamic_object_guid,
        area_trigger_guid,
        corpse_guid,
        scene_object_guid,
        conversation_guid,
        other_player_guid,
    ] {
        assert!(
            session.client_visible_guids_like_cpp.contains(&guid),
            "C++ VisibleNotifier class missing from client GUID cache: {guid:?}"
        );
    }
    assert_eq!(session.last_visibility_pos, Some(player_position));
    let packet = send_rx
        .try_recv()
        .expect("map-driven visibility should send create data without DB");
    let opcode = u16::from_le_bytes([packet[0], packet[1]]);
    assert_eq!(opcode, ServerOpcodes::UpdateObject as u16);
    assert_eq!(
        u32::from_le_bytes(packet[2..6].try_into().unwrap()),
        8,
        "C++ VisibleNotifier sends all eight instantiated object classes in one UpdateData"
    );
    assert!(
        send_rx.try_recv().is_err(),
        "C++ VisibleNotifier::SendToSelf builds one UpdateData packet for mixed visibility"
    );
}
#[tokio::test]
async fn send_initial_packets_after_add_to_map_rebuilds_visibility_after_login_clear_like_cpp() {
    let (mut session, _pkt_tx, send_rx) = make_session();
    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 74_332);
    let player_position = Position::new(10.0, 10.0, 0.0, 0.0);
    let creature_guid = test_creature_guid(74_333);

    session.set_map_manager(Arc::clone(&manager));
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_map_store(canonical_player_transfer_test_map_store_like_cpp());
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "LoginAfterAddVisibility".to_string(),
        player_position,
        571,
        1,
        1,
        80,
        0,
    ));
    session
        .ensure_canonical_world_map_for_current_player_like_cpp()
        .expect("canonical viewer map");
    session.apply_move_init_active_mover_complete_like_cpp(0);
    let _ = drain_server_packet_bytes(&send_rx);

    let creature_position = Position::new(20.0, 20.0, 0.0, 0.0);
    let (grid_x, grid_y) =
        crate::map_manager::world_to_grid_coords(creature_position.x, creature_position.y);
    manager.write().unwrap().add_creature(
        571,
        0,
        grid_x,
        grid_y,
        crate::map_manager::WorldCreature::new(
            creature_guid,
            901,
            creature_position,
            100,
            80,
            1,
            2,
            0.0,
            1,
            35,
            0,
            0,
        ),
    );

    session.last_visibility_pos = Some(player_position);
    session.client_visible_guids_like_cpp.clear();

    session
        .send_initial_packets_after_add_to_map(player_guid, &player_position, 571, false)
        .await;

    assert!(
        session
            .client_visible_guids_like_cpp
            .contains(&creature_guid),
        "C++ SendInitialPacketsAfterAddToMap::UpdateVisibilityForPlayer rebuilds visibility after Map::AddPlayerToMap clears m_clientGUIDs"
    );
    assert_eq!(session.last_visibility_pos, Some(player_position));
    let packets = drain_server_packet_bytes(&send_rx);
    assert!(
        packets.iter().any(|packet| {
            packet.len() >= 2
                && u16::from_le_bytes([packet[0], packet[1]]) == ServerOpcodes::UpdateObject as u16
        }),
        "post-add visibility rebuild should send creature CREATE update data during login"
    );
}
#[test]
fn canonical_world_map_login_binding_uses_cpp_split_faction_instance() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let guid = ObjectGuid::create_player(1, 42);

    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
        wow_data::MapEntry {
            id: 609,
            instance_type: wow_data::map::MAP_COMMON,
            expansion_id: 0,
            parent_map_id: 571,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
    ])));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        guid,
        "Orc".to_string(),
        Position::new(1.0, 2.0, 3.0, 0.0),
        609,
        2,
        1,
        10,
        0,
    ));

    let decision = session
        .ensure_canonical_world_map_for_current_player_like_cpp()
        .expect("world map decision");

    assert_eq!(
        decision,
        wow_map::CreateMapDecision::Create {
            key: wow_map::MapKey::new(609, 1),
            difficulty_id: 0,
            kind: wow_map::ManagedMapKind::World,
            side_effects: Vec::new(),
        }
    );
    assert!(canonical.lock().unwrap().find_map(609, 1).is_some());
}
#[test]
fn canonical_player_difficulty_and_loot_preferences_follow_detached_and_stale_ownership_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 5_566);

    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_map_store(canonical_player_transfer_test_map_store_like_cpp());
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "PreferenceOwner".to_string(),
        Position::new(3700.0, 1500.0, 120.0, 0.0),
        571,
        1,
        1,
        80,
        0,
    ));
    session
        .ensure_canonical_world_map_for_current_player_like_cpp()
        .expect("initial world map");
    let old_handle = session.player_handle_like_cpp.expect("canonical handle");

    assert!(session.replace_player_difficulty_preferences_like_cpp(2, 15, 4));
    assert!(session.set_pass_on_group_loot_like_cpp(true));
    assert_eq!(
        session.player_difficulty_preferences_snapshot_like_cpp(),
        Some((2, 15, 4))
    );
    assert_eq!(session.resolved_pass_on_group_loot_like_cpp(), Some(true));

    assert!(session.remove_current_player_from_canonical_current_map_like_cpp());
    assert_eq!(
        canonical
            .lock()
            .unwrap()
            .player_residence_like_cpp(old_handle),
        Some(wow_map::PlayerResidenceLikeCpp::Detached)
    );
    assert!(
        session
            .mutate_player_difficulty_preferences_like_cpp(|dungeon, raid, legacy_raid| {
                *dungeon = 1;
                *raid = 14;
                *legacy_raid = 3;
            })
            .is_some()
    );
    assert!(session.set_pass_on_group_loot_like_cpp(false));
    assert_eq!(
        session.player_difficulty_preferences_snapshot_like_cpp(),
        Some((1, 14, 3))
    );
    assert_eq!(session.resolved_pass_on_group_loot_like_cpp(), Some(false));

    let mut replacement = Box::new(Player::new(Some(2), false));
    replacement
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(player_guid);
    replacement.replace_difficulty_preferences_like_cpp(9, 8, 7);
    replacement.set_pass_on_group_loot_like_cpp(true);
    let replacement_handle = canonical
        .lock()
        .unwrap()
        .install_detached_player_like_cpp(replacement)
        .expect("replacement owner");

    assert_eq!(
        session.player_difficulty_preferences_snapshot_like_cpp(),
        None
    );
    assert_eq!(session.resolved_pass_on_group_loot_like_cpp(), None);
    assert!(!session.replace_player_difficulty_preferences_like_cpp(2, 15, 4));
    assert!(!session.set_pass_on_group_loot_like_cpp(false));
    assert_eq!(
        canonical
            .lock()
            .unwrap()
            .with_player_like_cpp(replacement_handle, |player| (
                player.difficulty_preferences_like_cpp(),
                player.pass_on_group_loot_like_cpp(),
            )),
        Some(((9, 8, 7), true))
    );
}
#[test]
fn canonical_access_requirement_map_difficulty_message_sends_difficulty_abort_like_cpp() {
    let (mut session, _pkt_tx, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 97);

    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "AccessMapDifficultyMessage".to_string(),
        Position::new(3700.0, 1500.0, 120.0, 0.0),
        631,
        1,
        1,
        79,
        0,
    ));
    session.represented_raid_difficulty_id_like_cpp = 3;
    install_create_map_active_lock_stores_with_expansion_and_max_players_like_cpp(
        &mut session,
        631,
        3,
        77,
        2,
        2,
        25,
    );
    session.set_map_difficulty_store(Arc::new(MapDifficultyStore::from_entries([
        MapDifficultyEntry {
            id: 900,
            message: "localized difficulty failure".to_string(),
            map_id: 631,
            difficulty_id: 3,
            lock_id: 77,
            reset_interval: 2,
            max_players: 25,
            flags: 0,
        },
    ])));
    let mut requirement = access_requirement_like_cpp(631, 3);
    requirement.level_min = 80;
    install_access_requirement_store_like_cpp(&mut session, requirement);

    assert_eq!(
        session.ensure_canonical_world_map_for_current_player_like_cpp(),
        Some(wow_map::CreateMapDecision::Reject {
            side_effects: Vec::new()
        })
    );
    assert_eq!(
        send_rx.try_recv().expect("SMSG_TRANSFER_ABORTED"),
        wow_packet::packets::misc::TransferAborted {
            map_id: 631,
            arg: 3,
            map_difficulty_x_condition_id: 0,
            transfer_abort: TRANSFER_ABORT_DIFFICULTY_LIKE_CPP,
        }
        .to_bytes()
    );
    assert!(send_rx.try_recv().is_err());
}
#[test]
fn canonical_access_requirement_map_difficulty_condition_sends_condition_abort_like_cpp() {
    let (mut session, _pkt_tx, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 87);

    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "AccessCondition".to_string(),
        Position::new(3700.0, 1500.0, 120.0, 0.0),
        631,
        1,
        1,
        80,
        0,
    ));
    session.represented_raid_difficulty_id_like_cpp = 3;
    install_create_map_active_lock_stores_like_cpp(&mut session, 631, 3, 77, 2);
    session.set_map_difficulty_x_condition_store(Arc::new(
        wow_data::MapDifficultyXConditionStore::from_entries([
            wow_data::MapDifficultyXConditionEntry {
                id: 222,
                failure_description: String::new(),
                player_condition_id: 333,
                order_index: 0,
                map_difficulty_id: 900,
            },
        ]),
    ));
    session.set_player_condition_store(Arc::new(wow_data::PlayerConditionStore::from_entries([
        wow_data::PlayerConditionEntry {
            id: 333,
            class_mask: 1 << 1,
            ..Default::default()
        },
    ])));

    assert_eq!(
        session.ensure_canonical_world_map_for_current_player_like_cpp(),
        Some(wow_map::CreateMapDecision::Reject {
            side_effects: Vec::new()
        })
    );
    assert_eq!(
        send_rx.try_recv().expect("map difficulty condition abort"),
        wow_packet::packets::misc::TransferAborted {
            map_id: 631,
            arg: 3,
            map_difficulty_x_condition_id: 222,
            transfer_abort: TRANSFER_ABORT_DIFFICULTY_LIKE_CPP,
        }
        .to_bytes()
    );
}
#[test]
fn instance_count_allows_under_limit_and_existing_instance_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    session.set_max_instances_per_hour_like_cpp(5);
    for instance_id in 100..104 {
        session.add_instance_enter_time_like_cpp(instance_id, 1_000);
    }

    assert!(session.check_instance_count_at_like_cpp(200, 1_000));
    session.add_instance_enter_time_like_cpp(104, 1_000);
    assert!(session.check_instance_count_at_like_cpp(102, 1_000));
    assert!(!session.check_instance_count_at_like_cpp(200, 1_000));
}
#[test]
fn instance_count_prunes_expired_entries_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    session.set_max_instances_per_hour_like_cpp(2);
    session
        .represented_instance_reset_times_like_cpp
        .insert(1, 10);
    session
        .represented_instance_reset_times_like_cpp
        .insert(2, 5_000);

    assert!(session.check_instance_count_at_like_cpp(3, 20));
    assert!(
        !session
            .represented_instance_reset_times_like_cpp
            .contains_key(&1)
    );
    assert!(
        session
            .represented_instance_reset_times_like_cpp
            .contains_key(&2)
    );
}
