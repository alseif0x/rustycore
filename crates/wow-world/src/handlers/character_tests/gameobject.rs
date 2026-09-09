//! Gameobject scenarios for [`super`].
//!
//! Split out of character_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

#[tokio::test]
async fn gossip_select_accepts_represented_goober_menu_and_removes_feign_like_cpp() {
    const FEIGN_SLOT: u8 = 19;
    const GOSSIP_ID: u32 = 72;
    let (mut session, send_rx, canonical) = make_bank_slot_session(2);
    insert_bank_test_player_in_world(&session, &canonical);
    let goober = gameobject_guid(9304, 304);
    insert_gossip_gameobject(
        &canonical,
        goober,
        9304,
        Position::new(1.0, 0.0, 0.0, 0.0),
        GAMEOBJECT_TYPE_GOOBER as u8,
        true,
    );
    session.represented_gameobject_use_states.insert(
        goober,
        RepresentedGameObjectUseState {
            map_id: Some(571),
            position: Some(Position::new(1.0, 0.0, 0.0, 0.0)),
            go_type: Some(GAMEOBJECT_TYPE_GOOBER as u8),
            icon_name_allows_interaction_like_cpp: Some(true),
            ..Default::default()
        },
    );
    session.set_player_interaction_source_like_cpp(goober);
    session
        .gossip_options
        .push(crate::session::GossipOptionInfo {
            gossip_option_id: 71,
            menu_id: GOSSIP_ID,
            order_index: 0,
            option_npc: 0,
            action_menu_id: 0,
        });
    seed_represented_feign_death_like_cpp(&mut session, FEIGN_SLOT);

    session
        .handle_gossip_select_option(wow_packet::packets::gossip::GossipSelectOption {
            gossip_unit: goober,
            gossip_id: GOSSIP_ID as i32,
            gossip_option_id: 71,
            promotion_code: String::new(),
        })
        .await;

    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::AuraUpdate],
        "a valid GOOBER follows the C++ generic GameObject interaction path"
    );
    assert!(!session.visible_auras.contains_key(&FEIGN_SLOT));
    assert!(!canonical_player_has_died_state_like_cpp(&mut session));
}
#[tokio::test]
async fn gossip_select_gameobject_revalidates_cpp_interaction_boundaries() {
    const FEIGN_SLOT: u8 = 20;
    const GOSSIP_ID: u32 = 82;
    for (case, go_type, recorded_type, position, in_world, icon_allows, in_taxi, same_phase) in [
        (
            "point-icon",
            GAMEOBJECT_TYPE_GOOBER as u8,
            true,
            Position::new(1.0, 0.0, 0.0, 0.0),
            true,
            Some(false),
            false,
            true,
        ),
        (
            "missing-icon-evidence",
            GAMEOBJECT_TYPE_GOOBER as u8,
            true,
            Position::new(1.0, 0.0, 0.0, 0.0),
            true,
            None,
            false,
            true,
        ),
        (
            "missing-runtime-type",
            GAMEOBJECT_TYPE_GOOBER as u8,
            false,
            Position::new(1.0, 0.0, 0.0, 0.0),
            true,
            Some(true),
            false,
            true,
        ),
        (
            "gameobject-not-in-world",
            GAMEOBJECT_TYPE_GOOBER as u8,
            true,
            Position::new(1.0, 0.0, 0.0, 0.0),
            false,
            Some(true),
            false,
            true,
        ),
        (
            "outside-interaction-distance",
            GAMEOBJECT_TYPE_GOOBER as u8,
            true,
            Position::new(50.0, 0.0, 0.0, 0.0),
            true,
            Some(true),
            false,
            true,
        ),
        (
            "player-in-taxi-flight",
            GAMEOBJECT_TYPE_GOOBER as u8,
            true,
            Position::new(1.0, 0.0, 0.0, 0.0),
            true,
            Some(true),
            true,
            true,
        ),
        (
            "incompatible-phase",
            GAMEOBJECT_TYPE_GOOBER as u8,
            true,
            Position::new(1.0, 0.0, 0.0, 0.0),
            true,
            Some(true),
            false,
            false,
        ),
    ] {
        let (mut session, send_rx, canonical) = make_bank_slot_session(2);
        insert_bank_test_player_in_world(&session, &canonical);
        let gameobject = gameobject_guid(9305, 305);
        insert_gossip_gameobject(&canonical, gameobject, 9305, position, go_type, in_world);
        session.represented_gameobject_use_states.insert(
            gameobject,
            RepresentedGameObjectUseState {
                map_id: Some(571),
                position: Some(position),
                go_type: recorded_type.then_some(go_type),
                icon_name_allows_interaction_like_cpp: icon_allows,
                ..Default::default()
            },
        );
        session.set_player_interaction_source_like_cpp(gameobject);
        session
            .gossip_options
            .push(crate::session::GossipOptionInfo {
                gossip_option_id: 81,
                menu_id: GOSSIP_ID,
                order_index: 0,
                option_npc: 0,
                action_menu_id: 0,
            });
        seed_represented_feign_death_like_cpp(&mut session, FEIGN_SLOT);
        if in_taxi {
            session.set_taxi_flight_state_like_cpp(
                RepresentedTaxiFlightNodeLikeCpp {
                    map_id: 571,
                    position: Position::new(1.0, 0.0, 0.0, 0.0),
                    teleport_flag: false,
                },
                None,
            );
        }
        if !same_phase {
            session.set_represented_player_phase_shift_like_cpp(
                wow_entities::PhaseShift::from_phases([10]),
            );
            session.record_represented_gameobject_phase_shift_like_cpp(
                gameobject,
                wow_entities::PhaseShift::from_phases([20]),
            );
        }

        session
            .handle_gossip_select_option(wow_packet::packets::gossip::GossipSelectOption {
                gossip_unit: gameobject,
                gossip_id: GOSSIP_ID as i32,
                gossip_option_id: 81,
                promotion_code: String::new(),
            })
            .await;

        assert!(
            send_rx.try_recv().is_err(),
            "{case}: rejection must precede fake-death removal and action routing"
        );
        assert!(
            session.visible_auras.contains_key(&FEIGN_SLOT),
            "{case}: rejected source must preserve fake death"
        );
        assert!(
            canonical_player_has_died_state_like_cpp(&mut session),
            "{case}: rejected source must preserve DIED state"
        );
        assert_eq!(
            session.player_interaction_source_guid_like_cpp(),
            Some(gameobject)
        );
    }
}
#[tokio::test]
async fn gossip_select_gameobject_rejects_npc_service_option_after_feign_like_cpp() {
    const FEIGN_SLOT: u8 = 23;
    const GOSSIP_ID: u32 = 92;
    let (mut session, send_rx, canonical) = make_bank_slot_session(2);
    insert_bank_test_player_in_world(&session, &canonical);
    let goober = gameobject_guid(9306, 306);
    insert_gossip_gameobject(
        &canonical,
        goober,
        9306,
        Position::new(1.0, 0.0, 0.0, 0.0),
        GAMEOBJECT_TYPE_GOOBER as u8,
        true,
    );
    session.represented_gameobject_use_states.insert(
        goober,
        RepresentedGameObjectUseState {
            map_id: Some(571),
            position: Some(Position::new(1.0, 0.0, 0.0, 0.0)),
            go_type: Some(GAMEOBJECT_TYPE_GOOBER as u8),
            icon_name_allows_interaction_like_cpp: Some(true),
            ..Default::default()
        },
    );
    session.set_player_interaction_source_like_cpp(goober);
    session
        .gossip_options
        .push(crate::session::GossipOptionInfo {
            gossip_option_id: 91,
            menu_id: GOSSIP_ID,
            order_index: 0,
            option_npc: 6, // Banker is invalid for every C++ GameObject menu.
            action_menu_id: 0,
        });
    seed_represented_feign_death_like_cpp(&mut session, FEIGN_SLOT);

    session
        .handle_gossip_select_option(wow_packet::packets::gossip::GossipSelectOption {
            gossip_unit: goober,
            gossip_id: GOSSIP_ID as i32,
            gossip_option_id: 91,
            promotion_code: String::new(),
        })
        .await;

    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::AuraUpdate],
        "C++ revalidates the GO and removes fake death before rejecting its NPC service option"
    );
    assert_eq!(
        session.player_interaction_source_guid_like_cpp(),
        Some(goober)
    );
    assert!(!session.visible_auras.contains_key(&FEIGN_SLOT));
    assert!(!canonical_player_has_died_state_like_cpp(&mut session));
}
#[tokio::test]
async fn quest_giver_status_tracked_supplied_gameobject_uses_uint64_status_like_cpp() {
    let (mut session, send_rx) = make_quest_status_session();
    let mut store = store_with_quests(&[3002]);
    assert!(store.insert_gameobject_starter_relation_like_cpp(9302, 3002));
    session.set_quest_store(Arc::new(store));
    let guid = gameobject_guid(9302, 302);
    let mut manager = wow_map::MapManager::default();
    insert_gameobject(&mut manager, guid, 9302);
    attach_map_manager(&mut session, manager);
    mark_gameobject_questgiver(&mut session, guid);

    session
        .handle_quest_giver_status_tracked_query(tracked_query_packet(&[guid]))
        .await;

    assert_eq!(
        recv_status_multiple(&send_rx),
        vec![(guid, quest_giver_status::TRIVIAL)]
    );
}
#[tokio::test]
async fn gameobject_query_uses_typed_catalog_and_preserves_packet_projection_like_cpp() {
    let row = gameobject_query_catalog_row_like_cpp();
    let guid = ObjectGuid::create_world_object(HighGuid::GameObject, 0, 0, 571, 0, 42, 99);
    let (mut session, send_rx) = make_session_with_send_capacity(1);
    install_world_query_catalogs_like_cpp(
        &mut session,
        [],
        [row.clone()],
        [],
        [(42, 43, 0), (42, 44, 1)],
    );

    session
        .handle_query_game_object(QueryGameObject {
            game_object_id: 42,
            guid,
        })
        .await;

    let mut names: [String; 4] = Default::default();
    names[0] = row.name;
    assert_eq!(
        send_rx.try_recv().unwrap(),
        QueryGameObjectResponse {
            game_object_id: 42,
            guid,
            allow: true,
            stats: Some(GameObjectStats {
                names,
                icon_name: row.icon_name,
                cast_bar_caption: row.cast_bar_caption,
                unk_string: row.unk_string,
                go_type: row.go_type,
                display_id: row.display_id,
                data: row.data,
                size: row.size,
                quest_items: vec![43, 44],
                content_tuning_id: row.content_tuning_id,
            }),
        }
        .to_bytes()
    );
}
#[tokio::test]
async fn gameobject_query_missing_or_failed_catalog_preserves_guid_and_disallows_like_cpp() {
    let guid = ObjectGuid::create_world_object(HighGuid::GameObject, 0, 0, 571, 0, 43, 100);
    for with_empty_capability in [false, true] {
        let (mut session, send_rx) = make_session_with_send_capacity(1);
        if with_empty_capability {
            install_world_query_catalogs_like_cpp(&mut session, [], [], [], []);
        }

        session
            .handle_query_game_object(QueryGameObject {
                game_object_id: 43,
                guid,
            })
            .await;

        assert_eq!(
            send_rx.try_recv().unwrap(),
            QueryGameObjectResponse {
                game_object_id: 43,
                guid,
                allow: false,
                stats: None,
            }
            .to_bytes()
        );
    }
}
