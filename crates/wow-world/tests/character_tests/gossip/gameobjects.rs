use super::*;
use super::fixtures_world::*;

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
    install_gossip_gameobject_state_for_test(&mut session, goober, Some(571), Some(Position::new(1.0, 0.0, 0.0, 0.0)), Some(GAMEOBJECT_TYPE_GOOBER as u8), Some(true));
    set_player_interaction_source_for_test(&mut session, goober);
    push_player_gossip_option_for_test(
        &mut session,
        wow_world::session::GossipOptionInfo {
            gossip_option_id: 71,
            menu_id: GOSSIP_ID,
            order_index: 0,
            option_npc: 0,
            action_menu_id: 0,
        },
    );
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
    assert!(!has_gossip_visible_aura_for_test(&session, FEIGN_SLOT));
    assert!(!canonical_player_has_died_state_like_cpp(&mut session));
}

#[tokio::test]
async fn gossip_select_gameobject_revalidates_cpp_interaction_boundaries() {
    const FEIGN_SLOT: u8 = 20;
    const GOSSIP_ID: u32 = 82;
    // Missing represented icon/type evidence is a Rust-only defensive case:
    // C++ reads the live template and GameObject directly, so it has no
    // equivalent absent-cache branch. The remaining rows mirror C++ gates.
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
        install_gossip_gameobject_state_for_test(&mut session, gameobject, Some(571), Some(position), recorded_type.then_some(go_type), icon_allows);
        set_player_interaction_source_for_test(&mut session, gameobject);
        push_player_gossip_option_for_test(
            &mut session,
            wow_world::session::GossipOptionInfo {
                gossip_option_id: 81,
                menu_id: GOSSIP_ID,
                order_index: 0,
                option_npc: 0,
                action_menu_id: 0,
            },
        );
        seed_represented_feign_death_like_cpp(&mut session, FEIGN_SLOT);
        if in_taxi {
            set_gossip_taxi_flight_for_test(&mut session,
                RepresentedTaxiFlightNodeLikeCpp {
                    map_id: 571,
                    position: Position::new(1.0, 0.0, 0.0, 0.0),
                    teleport_flag: false,
                },
                None,
            );
        }
        if !same_phase {
            set_gossip_player_phase_for_test(&mut session,
                wow_entities::PhaseShift::from_phases([10]),
            );
            set_gossip_gameobject_phase_for_test(&mut session,
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
            has_gossip_visible_aura_for_test(&session, FEIGN_SLOT),
            "{case}: rejected source must preserve fake death"
        );
        assert!(
            canonical_player_has_died_state_like_cpp(&mut session),
            "{case}: rejected source must preserve DIED state"
        );
        assert_eq!(
            player_interaction_source_guid_for_test(&session),
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
    install_gossip_gameobject_state_for_test(&mut session, goober, Some(571), Some(Position::new(1.0, 0.0, 0.0, 0.0)), Some(GAMEOBJECT_TYPE_GOOBER as u8), Some(true));
    set_player_interaction_source_for_test(&mut session, goober);
    push_player_gossip_option_for_test(
        &mut session,
        wow_world::session::GossipOptionInfo {
            gossip_option_id: 91,
            menu_id: GOSSIP_ID,
            order_index: 0,
            option_npc: 6, // Banker is invalid for every C++ GameObject menu.
            action_menu_id: 0,
        },
    );
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
        player_interaction_source_guid_for_test(&session),
        Some(goober)
    );
    assert!(!has_gossip_visible_aura_for_test(&session, FEIGN_SLOT));
    assert!(!canonical_player_has_died_state_like_cpp(&mut session));
}
