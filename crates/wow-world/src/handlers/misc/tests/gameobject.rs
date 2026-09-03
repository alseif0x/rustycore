// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! gameobject capability handler tests.

use super::*;

fn game_obj_use_test_session_like_cpp(
    template: Option<wow_data::GameObjectQueryTemplateLikeCpp>,
) -> (crate::session::WorldSession, ObjectGuid, ObjectGuid) {
    let (mut session, _send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 99);
    let gameobject_guid =
        ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, 777, 8);

    session.set_player_guid(Some(player_guid));
    session.set_loaded_player_identity_like_cpp(571, 1, 1, 10, 0);
    session.set_player_position_like_cpp(Position::new(10.0, 0.0, 0.0, 0.0));
    session.record_represented_gameobject_runtime_state_like_cpp(
        571,
        gameobject_guid,
        777,
        Position::new(12.0, 0.0, 0.0, 0.0),
        wow_entities::GAMEOBJECT_TYPE_DOOR as u8,
    );
    session
        .client_visible_guids_like_cpp
        .insert(gameobject_guid);
    session.set_object_mgr_catalogs_like_cpp(Arc::new(crate::session::ObjectMgrCatalogsLikeCpp {
        creature: Arc::new(wow_data::CreatureQueryCatalogLikeCpp::default()),
        gameobject: Arc::new(wow_data::GameObjectQueryCatalogLikeCpp::from_rows_like_cpp(
            template,
            [],
        )),
        gameobject_quest_items: Arc::new(wow_data::GameObjectQuestItemStoreLikeCpp::default()),
        page_text: Arc::new(wow_data::PageTextCatalogLikeCpp::default()),
    }));

    (session, player_guid, gameobject_guid)
}

fn game_obj_use_packet_like_cpp(gameobject_guid: ObjectGuid) -> WorldPacket {
    let mut packet = WorldPacket::new_empty();
    packet.write_packed_guid(&gameobject_guid);
    packet.reset_read();
    packet
}

#[tokio::test]
async fn game_obj_use_loaded_template_dispatches_door_without_concrete_db_like_cpp() {
    let mut data = [0_i32; wow_data::WORLD_QUERY_GAMEOBJECT_DATA_COUNT_LIKE_CPP];
    data[2] = 2_500;
    let (mut session, player_guid, gameobject_guid) =
        game_obj_use_test_session_like_cpp(Some(wow_data::GameObjectQueryTemplateLikeCpp {
            entry: 777,
            go_type: wow_entities::GAMEOBJECT_TYPE_DOOR as i32,
            display_id: 0,
            name: "Door".into(),
            icon_name: "Gossip".to_string(),
            cast_bar_caption: String::new(),
            unk_string: String::new(),
            size: 1.0,
            data,
            content_tuning_id: 0,
            min_money: 0,
            max_money: 0,
        }));

    session
        .handle_game_obj_use(game_obj_use_packet_like_cpp(gameobject_guid))
        .await;

    assert!(
        session
            .represented_gameobject_use_effects
            .iter()
            .any(|effect| {
                matches!(
                    effect,
                    crate::session::RepresentedGameObjectUseEffect::DoorOrButtonUsed {
                        gameobject_guid: effect_guid,
                        user_guid,
                        restore_time_ms: 2_500,
                        ..
                    } if *effect_guid == gameobject_guid && *user_guid == player_guid
                )
            })
    );
}

#[tokio::test]
async fn game_obj_use_missing_template_is_noop_like_cpp() {
    let (mut session, _player_guid, gameobject_guid) = game_obj_use_test_session_like_cpp(None);

    session
        .handle_game_obj_use(game_obj_use_packet_like_cpp(gameobject_guid))
        .await;

    assert!(session.represented_gameobject_use_effects.is_empty());
}

#[tokio::test]
async fn game_obj_use_missing_catalog_capability_is_noop_before_template_read_like_cpp() {
    let (mut session, _send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 99);
    let gameobject_guid =
        ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, 777, 9);
    session.set_player_guid(Some(player_guid));
    session.set_player_position_like_cpp(Position::new(10.0, 0.0, 0.0, 0.0));
    session.record_represented_gameobject_runtime_state_like_cpp(
        571,
        gameobject_guid,
        777,
        Position::new(12.0, 0.0, 0.0, 0.0),
        wow_entities::GAMEOBJECT_TYPE_DOOR as u8,
    );
    session
        .client_visible_guids_like_cpp
        .insert(gameobject_guid);

    session
        .handle_game_obj_use(game_obj_use_packet_like_cpp(gameobject_guid))
        .await;

    assert!(session.represented_gameobject_use_effects.is_empty());
}

#[test]
fn gameobject_point_icon_is_not_interactable_like_cpp() {
    assert!(!represented_gameobject_icon_allows_interaction_like_cpp(
        "Point"
    ));
    assert!(represented_gameobject_icon_allows_interaction_like_cpp(
        "Gossip"
    ));
    assert!(represented_gameobject_icon_allows_interaction_like_cpp(""));
}

#[tokio::test]
async fn game_obj_report_use_records_use_criteria_from_canonical_go_like_cpp() {
    let (mut session, _send_rx) = make_session();
    let canonical = Arc::new(std::sync::Mutex::new(wow_map::MapManager::default()));
    let player_guid = ObjectGuid::create_player(1, 99);
    let gameobject_guid =
        ObjectGuid::create_world_object(wow_core::guid::HighGuid::GameObject, 0, 1, 571, 0, 777, 5);

    session.set_player_guid(Some(player_guid));
    session.set_loaded_player_identity_like_cpp(571, 1, 1, 10, 0);
    session.set_player_position_like_cpp(Position::new(10.0, 0.0, 0.0, 0.0));
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.record_represented_gameobject_runtime_state_like_cpp(
        571,
        gameobject_guid,
        777,
        Position::new(14.0, 0.0, 0.0, 0.0),
        3,
    );

    let mut gameobject = wow_entities::GameObject::new();
    gameobject.world_mut().object_mut().create(gameobject_guid);
    gameobject.world_mut().object_mut().set_entry(777);
    gameobject.world_mut().set_map(571, 0).unwrap();
    gameobject
        .world_mut()
        .relocate(Position::new(14.0, 0.0, 0.0, 0.0));
    gameobject.world_mut().object_mut().add_to_world();
    canonical
        .lock()
        .unwrap()
        .create_world_map(571, 0)
        .map_mut()
        .insert_map_object_record(
            wow_entities::MapObjectRecord::new_game_object(gameobject).unwrap(),
        )
        .unwrap();

    let mut pkt = WorldPacket::new_empty();
    pkt.write_packed_guid(&gameobject_guid);
    session.handle_game_obj_report_use(pkt).await;

    assert_eq!(
        session.represented_gameobject_criteria_events,
        vec![
            crate::session::RepresentedGameObjectCriteriaEvent::UseGameobject {
                player_guid,
                gameobject_entry: 777,
            }
        ]
    );
}

#[tokio::test]
async fn game_obj_report_use_ignores_remote_control_like_cpp() {
    let (mut session, _send_rx) = make_session();
    let canonical = Arc::new(std::sync::Mutex::new(wow_map::MapManager::default()));
    let player_guid = ObjectGuid::create_player(1, 99);
    let controlled_guid = ObjectGuid::create_player(1, 100);
    let gameobject_guid =
        ObjectGuid::create_world_object(wow_core::guid::HighGuid::GameObject, 0, 1, 571, 0, 777, 6);

    session.set_player_guid(Some(player_guid));
    session.set_player_moved_unit_guid_like_cpp(controlled_guid);
    session.set_loaded_player_identity_like_cpp(571, 1, 1, 10, 0);
    session.set_player_position_like_cpp(Position::new(10.0, 0.0, 0.0, 0.0));
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.record_represented_gameobject_runtime_state_like_cpp(
        571,
        gameobject_guid,
        777,
        Position::new(14.0, 0.0, 0.0, 0.0),
        3,
    );

    let mut gameobject = wow_entities::GameObject::new();
    gameobject.world_mut().object_mut().create(gameobject_guid);
    gameobject.world_mut().object_mut().set_entry(777);
    gameobject.world_mut().set_map(571, 0).unwrap();
    gameobject
        .world_mut()
        .relocate(Position::new(14.0, 0.0, 0.0, 0.0));
    gameobject.world_mut().object_mut().add_to_world();
    canonical
        .lock()
        .unwrap()
        .create_world_map(571, 0)
        .map_mut()
        .insert_map_object_record(
            wow_entities::MapObjectRecord::new_game_object(gameobject).unwrap(),
        )
        .unwrap();

    let mut pkt = WorldPacket::new_empty();
    pkt.write_packed_guid(&gameobject_guid);
    session.handle_game_obj_report_use(pkt).await;

    assert!(session.represented_gameobject_criteria_events.is_empty());
}

#[tokio::test]
async fn game_obj_report_use_ai_can_consume_criteria_like_cpp() {
    let (mut session, _send_rx) = make_session();
    let canonical = Arc::new(std::sync::Mutex::new(wow_map::MapManager::default()));
    let player_guid = ObjectGuid::create_player(1, 99);
    let gameobject_guid =
        ObjectGuid::create_world_object(wow_core::guid::HighGuid::GameObject, 0, 1, 571, 0, 777, 7);

    session.set_player_guid(Some(player_guid));
    session.set_loaded_player_identity_like_cpp(571, 1, 1, 10, 0);
    session.set_player_position_like_cpp(Position::new(10.0, 0.0, 0.0, 0.0));
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.record_represented_gameobject_runtime_state_like_cpp(
        571,
        gameobject_guid,
        777,
        Position::new(14.0, 0.0, 0.0, 0.0),
        3,
    );
    session
        .represented_gameobject_use_states
        .get_mut(&gameobject_guid)
        .unwrap()
        .report_use_ai_returns_true = true;

    let mut gameobject = wow_entities::GameObject::new();
    gameobject.world_mut().object_mut().create(gameobject_guid);
    gameobject.world_mut().object_mut().set_entry(777);
    gameobject.world_mut().set_map(571, 0).unwrap();
    gameobject
        .world_mut()
        .relocate(Position::new(14.0, 0.0, 0.0, 0.0));
    gameobject.world_mut().object_mut().add_to_world();
    canonical
        .lock()
        .unwrap()
        .create_world_map(571, 0)
        .map_mut()
        .insert_map_object_record(
            wow_entities::MapObjectRecord::new_game_object(gameobject).unwrap(),
        )
        .unwrap();

    let mut pkt = WorldPacket::new_empty();
    pkt.write_packed_guid(&gameobject_guid);
    session.handle_game_obj_report_use(pkt).await;

    assert_eq!(
        session.represented_gameobject_use_effects,
        vec![
            crate::session::RepresentedGameObjectUseEffect::ReportUseAi {
                gameobject_guid,
                player_guid,
                handled: true,
            }
        ]
    );
    assert!(session.represented_gameobject_criteria_events.is_empty());
}

#[tokio::test]
async fn close_interaction_matching_source_resets_provenance_not_menu_like_cpp() {
    let (mut session, send_rx) = make_session();
    let source_guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 1, 42);
    session.set_player_trainer_interaction_like_cpp(source_guid, 77);
    session
        .gossip_options
        .push(crate::session::GossipOptionInfo {
            gossip_option_id: 1,
            menu_id: 0,
            order_index: 0,
            option_npc: 2,
            action_menu_id: 3,
        });
    let mut pkt = WorldPacket::new_empty();
    pkt.write_packed_guid(&source_guid);
    pkt.reset_read();

    session.handle_close_interaction(pkt).await;

    assert!(session.player_interaction_source_guid_like_cpp().is_none());
    assert_eq!(session.player_interaction_trainer_id_like_cpp(), 0);
    assert_eq!(
        session.gossip_options.len(),
        1,
        "C++ InteractionData::Reset is distinct from PlayerMenu::ClearMenus"
    );
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn close_interaction_nonmatching_source_preserves_gossip_like_cpp() {
    let (mut session, send_rx) = make_session();
    let active_guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 1, 43);
    let other_guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 1, 44);
    session.set_player_trainer_interaction_like_cpp(active_guid, 77);
    session
        .gossip_options
        .push(crate::session::GossipOptionInfo {
            gossip_option_id: 1,
            menu_id: 0,
            order_index: 0,
            option_npc: 2,
            action_menu_id: 3,
        });
    let mut pkt = WorldPacket::new_empty();
    pkt.write_packed_guid(&other_guid);
    pkt.reset_read();

    session.handle_close_interaction(pkt).await;

    assert_eq!(
        session.player_interaction_source_guid_like_cpp(),
        Some(active_guid)
    );
    assert_eq!(session.player_interaction_trainer_id_like_cpp(), 77);
    assert_eq!(session.gossip_options.len(), 1);
    assert!(send_rx.try_recv().is_err());
}
