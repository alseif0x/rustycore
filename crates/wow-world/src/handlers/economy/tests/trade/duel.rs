//! Duel packets.
//!
//! Separated from trade.rs under #709.

use super::*;

#[tokio::test]
async fn can_duel_missing_target_is_noop_like_cpp() {
    let (mut session, send_rx) = make_session();
    let canonical = shared_canonical_map_manager_for_misc_test();
    session.set_canonical_map_manager(Arc::clone(&canonical));
    let target_guid = ObjectGuid::create_player(1, 88);

    session
        .handle_can_duel(can_duel_packet(target_guid, false))
        .await;

    assert!(send_rx.try_recv().is_err());
    assert!(
        session
            .represented_can_duel_spell_casts_like_cpp()
            .is_empty()
    );
}

#[tokio::test]
async fn can_duel_allows_target_without_duel_and_records_spell_like_cpp() {
    let (mut session, send_rx) = make_session();
    let canonical = shared_canonical_map_manager_for_misc_test();
    let target_guid = ObjectGuid::create_player(1, 88);
    add_canonical_test_player_on_map_for_misc_test(
        &canonical,
        target_guid,
        Position::new(1.0, 2.0, 3.0, 0.0),
        571,
        0,
    );
    session.set_canonical_map_manager(Arc::clone(&canonical));

    session
        .handle_can_duel(can_duel_packet(target_guid, true))
        .await;

    let bytes = send_rx.try_recv().expect("can duel result");
    let mut packet = WorldPacket::from_bytes(&bytes);
    assert_eq!(packet.server_opcode(), Some(ServerOpcodes::CanDuelResult));
    assert_eq!(
        packet.read_uint16().unwrap(),
        ServerOpcodes::CanDuelResult as u16
    );
    let guid_bytes = packet.read_bytes(16).unwrap();
    let mut raw = [0u8; 16];
    raw.copy_from_slice(&guid_bytes);
    assert_eq!(ObjectGuid::from_raw_bytes(&raw), target_guid);
    assert!(packet.read_bit().unwrap());
    assert_eq!(
        session.represented_can_duel_spell_casts_like_cpp(),
        &[crate::session::RepresentedCanDuelSpellCastLikeCpp {
            target_guid,
            spell_id: crate::session::SPELL_DUEL_LIKE_CPP,
            to_the_death: true,
        }]
    );
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn can_duel_rejects_target_with_any_duel_info_like_cpp() {
    let (mut session, send_rx) = make_session();
    let canonical = shared_canonical_map_manager_for_misc_test();
    let target_guid = ObjectGuid::create_player(1, 88);
    let opponent_guid = ObjectGuid::create_player(1, 89);
    add_canonical_test_player_on_map_for_misc_test(
        &canonical,
        target_guid,
        Position::new(1.0, 2.0, 3.0, 0.0),
        571,
        0,
    );
    {
        let mut manager = canonical.lock().unwrap();
        let player = manager
            .find_map_mut(571, 0)
            .unwrap()
            .map_mut()
            .get_typed_player_mut(target_guid)
            .unwrap();
        player.set_duel_info_like_cpp(Some(wow_entities::PlayerDuelInfoLikeCpp {
            opponent: opponent_guid,
            state: wow_entities::PlayerDuelStateLikeCpp::Challenged,
        }));
    }
    session.set_canonical_map_manager(Arc::clone(&canonical));

    session
        .handle_can_duel(can_duel_packet(target_guid, false))
        .await;

    let bytes = send_rx.try_recv().expect("can duel result");
    let mut packet = WorldPacket::from_bytes(&bytes);
    assert_eq!(
        packet.read_uint16().unwrap(),
        ServerOpcodes::CanDuelResult as u16
    );
    let _ = packet.read_bytes(16).unwrap();
    assert!(!packet.read_bit().unwrap());
    assert!(
        session
            .represented_can_duel_spell_casts_like_cpp()
            .is_empty()
    );
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn duel_response_accepts_challenged_duel_and_sends_countdown_like_cpp() {
    let (mut source_session, source_send_rx) = make_session();
    let (mut partner_session, partner_send_rx) = make_session();
    let canonical = shared_canonical_map_manager_for_misc_test();
    let source_guid = ObjectGuid::create_player(1, 77);
    let partner_guid = ObjectGuid::create_player(1, 88);
    let arbiter_guid =
        ObjectGuid::create_world_object(wow_core::guid::HighGuid::GameObject, 0, 1, 571, 0, 7, 1);
    add_canonical_test_player_on_map_for_misc_test(
        &canonical,
        source_guid,
        Position::new(1.0, 2.0, 3.0, 0.0),
        571,
        0,
    );
    add_canonical_test_player_on_map_for_misc_test(
        &canonical,
        partner_guid,
        Position::new(2.0, 2.0, 3.0, 0.0),
        571,
        0,
    );
    {
        let mut manager = canonical.lock().unwrap();
        let map = manager.find_map_mut(571, 0).unwrap().map_mut();
        map.get_typed_player_mut(source_guid)
            .unwrap()
            .set_duel_info_like_cpp(Some(wow_entities::PlayerDuelInfoLikeCpp {
                opponent: partner_guid,
                state: wow_entities::PlayerDuelStateLikeCpp::Challenged,
            }));
        map.get_typed_player_mut(partner_guid)
            .unwrap()
            .set_duel_info_like_cpp(Some(wow_entities::PlayerDuelInfoLikeCpp {
                opponent: source_guid,
                state: wow_entities::PlayerDuelStateLikeCpp::Challenged,
            }));
    }
    source_session.set_player_guid(Some(source_guid));
    source_session.set_player_map_position_like_cpp(571, Position::new(1.0, 2.0, 3.0, 0.0));
    source_session.set_canonical_map_manager(Arc::clone(&canonical));
    source_session.set_represented_duel_arbiter_guid_like_cpp(Some(arbiter_guid));
    partner_session.set_player_guid(Some(partner_guid));

    let registry = Arc::new(PlayerRegistry::default());
    registry.register_or_replace(
        partner_guid,
        broadcast_info_with_command_tx(partner_session.session_command_tx()),
        Default::default(),
    );
    source_session.set_player_registry(registry);

    source_session
        .handle_duel_response(duel_response_packet(arbiter_guid, true, false))
        .await;
    partner_session
        .process_represented_session_commands_like_cpp()
        .await;

    assert_eq!(
        source_session.represented_duel_accepts_like_cpp(),
        &[crate::session::RepresentedDuelAcceptedLikeCpp {
            opponent_guid: partner_guid,
            arbiter_guid,
            countdown_ms: crate::session::DUEL_COUNTDOWN_MS_LIKE_CPP,
        }]
    );
    {
        let manager = canonical.lock().unwrap();
        let map = manager.find_map(571, 0).unwrap().map();
        assert_eq!(
            map.get_typed_player(source_guid)
                .unwrap()
                .duel_info_like_cpp()
                .unwrap()
                .state,
            wow_entities::PlayerDuelStateLikeCpp::Countdown
        );
        assert_eq!(
            map.get_typed_player(partner_guid)
                .unwrap()
                .duel_info_like_cpp()
                .unwrap()
                .state,
            wow_entities::PlayerDuelStateLikeCpp::Countdown
        );
    }
    let source_bytes = source_send_rx.try_recv().expect("source duel countdown");
    let partner_bytes = partner_send_rx.try_recv().expect("partner duel countdown");
    assert_eq!(source_bytes, partner_bytes);
    let mut packet = WorldPacket::from_bytes(&source_bytes);
    assert_eq!(
        packet.read_uint16().unwrap(),
        ServerOpcodes::DuelCountdown as u16
    );
    assert_eq!(
        packet.read_uint32().unwrap(),
        crate::session::DUEL_COUNTDOWN_MS_LIKE_CPP
    );
    assert!(source_send_rx.try_recv().is_err());
    assert!(partner_send_rx.try_recv().is_err());
}

#[tokio::test]
async fn duel_response_wrong_arbiter_is_noop_like_cpp() {
    let (mut session, send_rx) = make_session();
    let canonical = shared_canonical_map_manager_for_misc_test();
    let source_guid = ObjectGuid::create_player(1, 77);
    let partner_guid = ObjectGuid::create_player(1, 88);
    let arbiter_guid =
        ObjectGuid::create_world_object(wow_core::guid::HighGuid::GameObject, 0, 1, 571, 0, 7, 2);
    let wrong_arbiter =
        ObjectGuid::create_world_object(wow_core::guid::HighGuid::GameObject, 0, 1, 571, 0, 7, 3);
    add_canonical_test_player_on_map_for_misc_test(
        &canonical,
        source_guid,
        Position::new(1.0, 2.0, 3.0, 0.0),
        571,
        0,
    );
    {
        let mut manager = canonical.lock().unwrap();
        manager
            .find_map_mut(571, 0)
            .unwrap()
            .map_mut()
            .get_typed_player_mut(source_guid)
            .unwrap()
            .set_duel_info_like_cpp(Some(wow_entities::PlayerDuelInfoLikeCpp {
                opponent: partner_guid,
                state: wow_entities::PlayerDuelStateLikeCpp::Challenged,
            }));
    }
    session.set_player_guid(Some(source_guid));
    session.set_player_map_position_like_cpp(571, Position::new(1.0, 2.0, 3.0, 0.0));
    session.set_canonical_map_manager(canonical);
    session.set_represented_duel_arbiter_guid_like_cpp(Some(arbiter_guid));

    session
        .handle_duel_response(duel_response_packet(wrong_arbiter, true, false))
        .await;

    assert!(session.represented_duel_accepts_like_cpp().is_empty());
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn duel_response_cancel_interrupts_challenged_duel_like_cpp() {
    let (mut session, send_rx) = make_session();
    let canonical = shared_canonical_map_manager_for_misc_test();
    let source_guid = ObjectGuid::create_player(1, 77);
    let partner_guid = ObjectGuid::create_player(1, 88);
    let arbiter_guid =
        ObjectGuid::create_world_object(wow_core::guid::HighGuid::GameObject, 0, 1, 571, 0, 7, 4);
    add_canonical_test_player_on_map_for_misc_test(
        &canonical,
        source_guid,
        Position::new(1.0, 2.0, 3.0, 0.0),
        571,
        0,
    );
    add_canonical_test_player_on_map_for_misc_test(
        &canonical,
        partner_guid,
        Position::new(2.0, 2.0, 3.0, 0.0),
        571,
        0,
    );
    {
        let mut manager = canonical.lock().unwrap();
        let map = manager.find_map_mut(571, 0).unwrap().map_mut();
        map.get_typed_player_mut(source_guid)
            .unwrap()
            .set_duel_info_like_cpp(Some(wow_entities::PlayerDuelInfoLikeCpp {
                opponent: partner_guid,
                state: wow_entities::PlayerDuelStateLikeCpp::Challenged,
            }));
        map.get_typed_player_mut(partner_guid)
            .unwrap()
            .set_duel_info_like_cpp(Some(wow_entities::PlayerDuelInfoLikeCpp {
                opponent: source_guid,
                state: wow_entities::PlayerDuelStateLikeCpp::Challenged,
            }));
    }
    session.set_player_guid(Some(source_guid));
    session.set_player_map_position_like_cpp(571, Position::new(1.0, 2.0, 3.0, 0.0));
    session.set_canonical_map_manager(Arc::clone(&canonical));

    session
        .handle_duel_response(duel_response_packet(arbiter_guid, false, false))
        .await;

    assert_eq!(
        session.represented_duel_cancels_like_cpp(),
        &[crate::session::RepresentedDuelCancelledLikeCpp {
            opponent_guid: partner_guid,
            outcome: crate::session::RepresentedDuelCancelOutcomeLikeCpp::Interrupted,
            beg_spell_id: None,
        }]
    );
    let manager = canonical.lock().unwrap();
    let map = manager.find_map(571, 0).unwrap().map();
    assert!(
        map.get_typed_player(source_guid)
            .unwrap()
            .duel_info_like_cpp()
            .is_none()
    );
    assert!(
        map.get_typed_player(partner_guid)
            .unwrap()
            .duel_info_like_cpp()
            .is_none()
    );
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn duel_response_forfeit_records_surrender_spell_like_cpp() {
    let (mut session, send_rx) = make_session();
    let canonical = shared_canonical_map_manager_for_misc_test();
    let source_guid = ObjectGuid::create_player(1, 77);
    let partner_guid = ObjectGuid::create_player(1, 88);
    let arbiter_guid =
        ObjectGuid::create_world_object(wow_core::guid::HighGuid::GameObject, 0, 1, 571, 0, 7, 5);
    add_canonical_test_player_on_map_for_misc_test(
        &canonical,
        source_guid,
        Position::new(1.0, 2.0, 3.0, 0.0),
        571,
        0,
    );
    add_canonical_test_player_on_map_for_misc_test(
        &canonical,
        partner_guid,
        Position::new(2.0, 2.0, 3.0, 0.0),
        571,
        0,
    );
    {
        let mut manager = canonical.lock().unwrap();
        let map = manager.find_map_mut(571, 0).unwrap().map_mut();
        map.get_typed_player_mut(source_guid)
            .unwrap()
            .set_duel_info_like_cpp(Some(wow_entities::PlayerDuelInfoLikeCpp {
                opponent: partner_guid,
                state: wow_entities::PlayerDuelStateLikeCpp::InProgress,
            }));
        map.get_typed_player_mut(partner_guid)
            .unwrap()
            .set_duel_info_like_cpp(Some(wow_entities::PlayerDuelInfoLikeCpp {
                opponent: source_guid,
                state: wow_entities::PlayerDuelStateLikeCpp::InProgress,
            }));
    }
    session.set_player_guid(Some(source_guid));
    session.set_player_map_position_like_cpp(571, Position::new(1.0, 2.0, 3.0, 0.0));
    session.set_canonical_map_manager(canonical);

    session
        .handle_duel_response(duel_response_packet(arbiter_guid, true, true))
        .await;

    assert_eq!(
        session.represented_duel_cancels_like_cpp(),
        &[crate::session::RepresentedDuelCancelledLikeCpp {
            opponent_guid: partner_guid,
            outcome: crate::session::RepresentedDuelCancelOutcomeLikeCpp::Surrendered,
            beg_spell_id: Some(crate::session::SPELL_DUEL_BEG_LIKE_CPP),
        }]
    );
    assert!(send_rx.try_recv().is_err());
}

#[test]
fn duel_response_handler_metadata_like_cpp() {
    let entry = inventory::iter::<PacketHandlerEntry>
        .into_iter()
        .find(|entry| entry.opcode == ClientOpcodes::DuelResponse)
        .expect("DuelResponse handler entry");

    assert_eq!(entry.status, SessionStatus::LoggedIn);
    assert_eq!(entry.processing, PacketProcessing::ThreadUnsafe);
    assert_eq!(entry.handler_name, "handle_duel_response");
}
