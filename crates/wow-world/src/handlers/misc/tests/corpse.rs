// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! corpse capability handler tests.

use super::*;

#[tokio::test]
async fn request_cemetery_list_without_links_sends_no_response_like_cpp() {
    let (mut session, send_rx) = make_session();
    session.set_player_guid(Some(ObjectGuid::create_player(1, 42)));
    session.set_player_zone_area_like_cpp(1234, 5678);
    session.set_graveyard_store(Arc::new(GraveyardStore::default()));

    session
        .handle_request_cemetery_list(request_cemetery_list_packet(None))
        .await;

    assert!(
        send_rx.try_recv().is_err(),
        "C++ returns without SMSG_REQUEST_CEMETERY_LIST_RESPONSE when no graveyards match"
    );
}

#[tokio::test]
async fn request_cemetery_list_ignores_payload_and_sends_zone_ids_like_cpp() {
    let (mut session, send_rx) = make_session();
    session.set_player_guid(Some(ObjectGuid::create_player(1, 42)));
    session.set_player_zone_area_like_cpp(4321, 8765);
    let (graveyards, condition_store) = graveyard_store_with_links(4321, [11, 12], []);
    session.set_graveyard_store(graveyards);
    session.set_condition_store(condition_store);

    session
        .handle_request_cemetery_list(request_cemetery_list_packet(Some(1)))
        .await;

    let (is_gossip_triggered, cemetery_ids) =
        read_cemetery_list_response(&send_rx.try_recv().unwrap());
    assert!(
        !is_gossip_triggered,
        "C++ request has no gossip bool payload; response always sets false here"
    );
    assert_eq!(cemetery_ids, vec![11, 12]);
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn request_cemetery_list_filters_conditions_and_caps_at_sixteen_like_cpp() {
    let (mut session, send_rx) = make_session();
    session.set_player_guid(Some(ObjectGuid::create_player(1, 42)));
    session.set_loaded_player_identity_like_cpp(571, 1, 1, 80, 0);
    session.set_player_zone_area_like_cpp(2222, 3333);
    let conditions = [
        graveyard_team_condition(2222, 100, wow_data::TEAM_HORDE_LIKE_CPP),
        graveyard_team_condition(2222, 101, wow_data::TEAM_ALLIANCE_LIKE_CPP),
    ];
    let (graveyards, condition_store) = graveyard_store_with_links(2222, 100..120, conditions);
    session.set_graveyard_store(graveyards);
    session.set_condition_store(condition_store);

    session
        .handle_request_cemetery_list(request_cemetery_list_packet(None))
        .await;

    let (_, cemetery_ids) = read_cemetery_list_response(&send_rx.try_recv().unwrap());
    assert_eq!(
        cemetery_ids,
        (101..117).collect::<Vec<_>>(),
        "C++ skips failed condition rows and continues until 16 accepted IDs"
    );
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn resurrect_response_accepts_matching_request_like_cpp() {
    let (mut session, send_rx) = make_session();
    let resurrecter = ObjectGuid::create_player(1, 77);
    let target_position = Position::new(11.0, 22.0, 33.0, 1.5);
    session.set_player_guid(Some(ObjectGuid::create_player(1, 42)));
    session.set_loaded_player_identity_like_cpp(571, 1, 1, 80, 0);
    session.set_player_position_like_cpp(Position::new(1.0, 2.0, 3.0, 0.0));
    session.set_player_health_like_cpp(0, 1_000);
    session.set_represented_resurrection_request_like_cpp(
        wow_entities::PlayerResurrectionRequestLikeCpp {
            resurrecter,
            map_id: 571,
            position: target_position,
            health: 450,
            mana: 120,
            aura: 0,
        },
    );

    session
        .handle_resurrect_response(resurrect_response_packet(resurrecter, 0))
        .await;

    assert!(!session.player_is_alive_like_cpp());
    assert!(
        session
            .represented_resurrection_request_like_cpp()
            .is_none()
    );
    assert!(
        session
            .represented_delayed_resurrection_after_teleport_like_cpp()
            .is_some()
    );
    assert_eq!(
        std::iter::from_fn(|| send_rx.try_recv().ok())
            .map(|bytes| u16::from_le_bytes([bytes[0], bytes[1]]))
            .collect::<Vec<_>>(),
        vec![
            ServerOpcodes::CancelCombat as u16,
            ServerOpcodes::MoveTeleport as u16,
        ]
    );

    let action = session.handle_move_teleport_ack_like_cpp(ObjectGuid::create_player(1, 42), 0, 0);
    assert_eq!(
        action,
        crate::session::MoveTeleportAckActionLikeCpp::Accepted
    );

    assert!(session.player_is_alive_like_cpp());
    assert_eq!(session.player_health_like_cpp(), 450);
    assert_eq!(session.player_position_like_cpp(), Some(target_position));
    assert!(
        session
            .represented_delayed_resurrection_after_teleport_like_cpp()
            .is_none()
    );
}

#[tokio::test]
async fn resurrect_response_decline_clears_request_like_cpp() {
    let (mut session, send_rx) = make_session();
    let resurrecter = ObjectGuid::create_player(1, 78);
    session.set_player_health_like_cpp(0, 1_000);
    session.set_represented_resurrection_request_like_cpp(
        wow_entities::PlayerResurrectionRequestLikeCpp {
            resurrecter,
            map_id: 571,
            position: Position::new(11.0, 22.0, 33.0, 1.5),
            health: 450,
            mana: 120,
            aura: 0,
        },
    );

    session
        .handle_resurrect_response(resurrect_response_packet(resurrecter, 1))
        .await;

    assert!(!session.player_is_alive_like_cpp());
    assert!(
        session
            .represented_resurrection_request_like_cpp()
            .is_none()
    );
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn resurrect_response_ignores_mismatched_resurrecter_like_cpp() {
    let (mut session, send_rx) = make_session();
    let resurrecter = ObjectGuid::create_player(1, 79);
    session.set_player_health_like_cpp(0, 1_000);
    session.set_represented_resurrection_request_like_cpp(
        wow_entities::PlayerResurrectionRequestLikeCpp {
            resurrecter,
            map_id: 571,
            position: Position::new(11.0, 22.0, 33.0, 1.5),
            health: 450,
            mana: 120,
            aura: 0,
        },
    );

    session
        .handle_resurrect_response(resurrect_response_packet(
            ObjectGuid::create_player(1, 80),
            0,
        ))
        .await;

    assert!(!session.player_is_alive_like_cpp());
    assert!(
        session
            .represented_resurrection_request_like_cpp()
            .is_some()
    );
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn resurrect_response_ignores_alive_player_like_cpp() {
    let (mut session, send_rx) = make_session();
    let resurrecter = ObjectGuid::create_player(1, 81);
    session.set_player_health_like_cpp(777, 1_000);
    session.set_represented_resurrection_request_like_cpp(
        wow_entities::PlayerResurrectionRequestLikeCpp {
            resurrecter,
            map_id: 571,
            position: Position::new(11.0, 22.0, 33.0, 1.5),
            health: 450,
            mana: 120,
            aura: 0,
        },
    );

    session
        .handle_resurrect_response(resurrect_response_packet(resurrecter, 0))
        .await;

    assert!(session.player_is_alive_like_cpp());
    assert_eq!(session.player_health_like_cpp(), 777);
    assert!(
        session
            .represented_resurrection_request_like_cpp()
            .is_some()
    );
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn repop_request_dead_non_ghost_sets_ghost_and_repop_count_like_cpp() {
    let (mut session, send_rx) = make_session();
    let canonical = shared_canonical_map_manager_for_misc_test();
    let player_guid = ObjectGuid::create_player(1, 42);
    session.set_player_guid(Some(player_guid));
    session.set_loaded_player_identity_like_cpp(571, 1, 1, 80, 0);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    add_canonical_test_player_on_map_for_misc_test(
        &canonical,
        player_guid,
        Position::new(1.0, 2.0, 3.0, 0.0),
        571,
        0,
    );
    session.set_player_alive_like_cpp(false);

    session
        .handle_repop_request(repop_request_packet(true))
        .await;

    assert!(!session.player_is_alive_like_cpp());
    assert!(session.player_has_ghost_flag_like_cpp());
    assert_eq!(session.represented_repop_at_graveyard_count, 1);
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn repop_request_alive_or_already_ghost_returns_like_cpp() {
    let (mut session, _send_rx) = make_session();
    let canonical = shared_canonical_map_manager_for_misc_test();
    let player_guid = ObjectGuid::create_player(1, 43);
    session.set_player_guid(Some(player_guid));
    session.set_loaded_player_identity_like_cpp(571, 1, 1, 80, 0);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    add_canonical_test_player_on_map_for_misc_test(
        &canonical,
        player_guid,
        Position::new(1.0, 2.0, 3.0, 0.0),
        571,
        0,
    );

    session.set_player_alive_like_cpp(true);
    session
        .handle_repop_request(repop_request_packet(false))
        .await;
    assert_eq!(session.represented_repop_at_graveyard_count, 0);
    assert!(!session.player_has_ghost_flag_like_cpp());

    session.set_player_alive_like_cpp(false);
    session.set_player_ghost_flag_like_cpp(true);
    session
        .handle_repop_request(repop_request_packet(false))
        .await;
    assert_eq!(session.represented_repop_at_graveyard_count, 0);
    assert!(session.player_has_ghost_flag_like_cpp());
}

#[tokio::test]
async fn client_port_graveyard_dead_ghost_repops_like_cpp() {
    let (mut session, send_rx) = make_session();
    let canonical = shared_canonical_map_manager_for_misc_test();
    let player_guid = ObjectGuid::create_player(1, 4301);
    session.set_player_guid(Some(player_guid));
    session.set_loaded_player_identity_like_cpp(571, 1, 1, 80, 0);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    add_canonical_test_player_on_map_for_misc_test(
        &canonical,
        player_guid,
        Position::new(1.0, 2.0, 3.0, 0.0),
        571,
        0,
    );
    session.set_player_alive_like_cpp(false);
    session.set_player_ghost_flag_like_cpp(true);

    let handled = session
        .try_handle_client_port_graveyard_like_cpp(port_graveyard_packet())
        .await;

    assert!(handled);
    assert!(!session.player_is_alive_like_cpp());
    assert!(session.player_has_ghost_flag_like_cpp());
    assert_eq!(session.represented_repop_at_graveyard_count, 1);
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn client_port_graveyard_alive_or_not_ghost_returns_like_cpp() {
    let (mut session, _send_rx) = make_session();
    let canonical = shared_canonical_map_manager_for_misc_test();
    let player_guid = ObjectGuid::create_player(1, 4302);
    session.set_player_guid(Some(player_guid));
    session.set_loaded_player_identity_like_cpp(571, 1, 1, 80, 0);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    add_canonical_test_player_on_map_for_misc_test(
        &canonical,
        player_guid,
        Position::new(1.0, 2.0, 3.0, 0.0),
        571,
        0,
    );

    session.set_player_alive_like_cpp(true);
    assert!(
        session
            .try_handle_client_port_graveyard_like_cpp(port_graveyard_packet())
            .await
    );
    assert_eq!(session.represented_repop_at_graveyard_count, 0);
    assert!(session.player_is_alive_like_cpp());

    session.set_player_alive_like_cpp(false);
    session.set_player_ghost_flag_like_cpp(false);
    assert!(
        session
            .try_handle_client_port_graveyard_like_cpp(port_graveyard_packet())
            .await
    );
    assert_eq!(session.represented_repop_at_graveyard_count, 0);
    assert!(!session.player_has_ghost_flag_like_cpp());

    let mut non_empty = WorldPacket::new_empty();
    non_empty.write_uint8(1);
    non_empty.reset_read();
    assert!(
        !session
            .try_handle_client_port_graveyard_like_cpp(non_empty)
            .await
    );
}

#[tokio::test]
async fn reclaim_corpse_dead_ghost_resurrects_and_clears_ghost_like_cpp() {
    let (mut session, send_rx) = make_session();
    let canonical = shared_canonical_map_manager_for_misc_test();
    let player_guid = ObjectGuid::create_player(1, 44);
    let corpse_guid = ObjectGuid::create_world_object(HighGuid::Corpse, 0, 1, 571, 0, 0, 99);
    session.set_player_guid(Some(player_guid));
    session.set_loaded_player_identity_like_cpp(571, 1, 1, 80, 0);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    add_canonical_test_player_on_map_for_misc_test(
        &canonical,
        player_guid,
        Position::new(1.0, 2.0, 3.0, 0.0),
        571,
        0,
    );
    session.set_player_alive_like_cpp(false);
    session.set_player_ghost_flag_like_cpp(true);
    let _ = session.sync_canonical_player_health_like_cpp(0, 100);

    session
        .handle_reclaim_corpse(reclaim_corpse_packet(corpse_guid))
        .await;

    assert!(session.player_is_alive_like_cpp());
    assert!(!session.player_has_ghost_flag_like_cpp());
    assert_eq!(
        session.canonical_player_health_snapshot_like_cpp(),
        Some((50, 100))
    );
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn reclaim_corpse_alive_or_not_ghost_returns_like_cpp() {
    let (mut session, _send_rx) = make_session();
    let canonical = shared_canonical_map_manager_for_misc_test();
    let player_guid = ObjectGuid::create_player(1, 45);
    let corpse_guid = ObjectGuid::create_world_object(HighGuid::Corpse, 0, 1, 571, 0, 0, 100);
    session.set_player_guid(Some(player_guid));
    session.set_loaded_player_identity_like_cpp(571, 1, 1, 80, 0);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    add_canonical_test_player_on_map_for_misc_test(
        &canonical,
        player_guid,
        Position::new(1.0, 2.0, 3.0, 0.0),
        571,
        0,
    );

    session.set_player_alive_like_cpp(true);
    session
        .handle_reclaim_corpse(reclaim_corpse_packet(corpse_guid))
        .await;
    assert!(session.player_is_alive_like_cpp());
    assert!(!session.player_has_ghost_flag_like_cpp());

    session.set_player_alive_like_cpp(false);
    session
        .handle_reclaim_corpse(reclaim_corpse_packet(corpse_guid))
        .await;
    assert!(!session.player_is_alive_like_cpp());
    assert!(!session.player_has_ghost_flag_like_cpp());
}

#[tokio::test]
async fn instance_lock_response_decline_repops_and_clears_pending_bind_like_cpp() {
    let (mut session, _send_rx) = make_session();
    session.pending_bind = Some(crate::session::RepresentedPendingBind {
        map_id: 0,
        instance_id: 77,
        completed_mask: 0xA5,
        time_until_lock_ms: 60_000,
    });

    session
        .handle_instance_lock_response(WorldPacket::from_bytes(&[0x00]))
        .await;

    assert!(session.pending_bind.is_none());
    assert!(session.represented_confirmed_pending_binds.is_empty());
    assert_eq!(session.represented_repop_at_graveyard_count, 1);
}
