// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! trade capability handler tests.

use super::*;
use crate::session::TRADE_STATUS_PLAYER_BUSY_LIKE_CPP;
use wow_packet::packets::misc::{
    TRADE_STATUS_CANCELLED_LIKE_CPP, TRADE_STATUS_PLAYER_IGNORED_LIKE_CPP,
};

#[tokio::test]
async fn cancel_trade_without_active_trade_is_noop_like_cpp() {
    let (mut session, send_rx) = make_session();

    session.handle_cancel_trade(WorldPacket::new_empty()).await;

    assert!(
        session
            .represented_trade_cancel_statuses_like_cpp()
            .is_empty()
    );
    assert!(send_rx.try_recv().is_err());
}

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

#[tokio::test]
async fn cancel_trade_cancels_represented_trade_and_sends_status_like_cpp() {
    let (mut session, send_rx) = make_session();
    let partner_guid = ObjectGuid::create_player(1, 88);
    session.set_represented_active_trade_partner_like_cpp(Some(partner_guid));

    session.handle_cancel_trade(WorldPacket::new_empty()).await;

    assert_eq!(
        session.represented_trade_cancel_statuses_like_cpp(),
        &[TRADE_STATUS_CANCELLED_LIKE_CPP]
    );
    assert!(
        session
            .represented_active_trade_partner_like_cpp()
            .is_none()
    );
    let bytes = send_rx.try_recv().expect("trade status");
    assert_eq!(
        u16::from_le_bytes([bytes[0], bytes[1]]),
        ServerOpcodes::TradeStatus as u16
    );
    assert_eq!(bytes[2], TRADE_STATUS_CANCELLED_LIKE_CPP << 2);
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn cancel_trade_cancels_partner_represented_trade_like_cpp() {
    let (mut source_session, source_send_rx) = make_session();
    let (mut partner_session, partner_send_rx) = make_session();
    let source_guid = ObjectGuid::create_player(1, 77);
    let partner_guid = ObjectGuid::create_player(1, 88);
    source_session.set_player_guid(Some(source_guid));
    partner_session.set_player_guid(Some(partner_guid));
    source_session.set_represented_active_trade_partner_like_cpp(Some(partner_guid));
    partner_session.set_represented_active_trade_partner_like_cpp(Some(source_guid));

    let registry = Arc::new(PlayerRegistry::default());
    let partner_command_tx = partner_session.session_command_tx();
    registry.register_or_replace(
        partner_guid,
        broadcast_info_with_command_tx(partner_command_tx),
        Default::default(),
    );
    source_session.set_player_registry(registry);

    source_session
        .handle_cancel_trade(WorldPacket::new_empty())
        .await;
    partner_session
        .process_represented_session_commands_like_cpp()
        .await;

    assert!(
        source_session
            .represented_active_trade_partner_like_cpp()
            .is_none()
    );
    assert!(
        partner_session
            .represented_active_trade_partner_like_cpp()
            .is_none()
    );
    assert_eq!(
        source_session.represented_trade_cancel_statuses_like_cpp(),
        &[TRADE_STATUS_CANCELLED_LIKE_CPP]
    );
    assert_eq!(
        partner_session.represented_trade_cancel_statuses_like_cpp(),
        &[TRADE_STATUS_CANCELLED_LIKE_CPP]
    );

    let source_bytes = source_send_rx.try_recv().expect("source trade status");
    let partner_bytes = partner_send_rx.try_recv().expect("partner trade status");
    assert_eq!(source_bytes, partner_bytes);
    assert_eq!(
        u16::from_le_bytes([source_bytes[0], source_bytes[1]]),
        ServerOpcodes::TradeStatus as u16
    );
    assert_eq!(source_bytes[2], TRADE_STATUS_CANCELLED_LIKE_CPP << 2);
}

#[tokio::test]
async fn accept_trade_without_active_trade_is_noop_like_cpp() {
    let (mut session, send_rx) = make_session();

    session.handle_accept_trade(accept_trade_packet(0)).await;

    assert!(!session.represented_trade_accepted_like_cpp());
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn accept_trade_state_changed_resets_acceptance_like_cpp() {
    let (mut session, send_rx) = make_session();
    let partner_guid = ObjectGuid::create_player(1, 88);
    session.set_represented_active_trade_partner_like_cpp(Some(partner_guid));
    session.set_represented_partner_trade_server_state_index_like_cpp(7);

    session.handle_accept_trade(accept_trade_packet(8)).await;

    assert!(!session.represented_trade_accepted_like_cpp());
    assert_eq!(
        session.represented_active_trade_partner_like_cpp(),
        Some(partner_guid)
    );
    let bytes = send_rx.try_recv().expect("trade status");
    assert_eq!(
        u16::from_le_bytes([bytes[0], bytes[1]]),
        ServerOpcodes::TradeStatus as u16
    );
    assert_eq!(bytes[2], TRADE_STATUS_STATE_CHANGED_LIKE_CPP << 2);
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn accept_trade_records_acceptance_and_notifies_partner_like_cpp() {
    let (mut source_session, source_send_rx) = make_session();
    let (mut partner_session, partner_send_rx) = make_session();
    let source_guid = ObjectGuid::create_player(1, 77);
    let partner_guid = ObjectGuid::create_player(1, 88);
    source_session.set_player_guid(Some(source_guid));
    partner_session.set_player_guid(Some(partner_guid));
    source_session.set_represented_active_trade_partner_like_cpp(Some(partner_guid));
    partner_session.set_represented_active_trade_partner_like_cpp(Some(source_guid));
    source_session.set_represented_partner_trade_server_state_index_like_cpp(42);

    let registry = Arc::new(PlayerRegistry::default());
    let partner_command_tx = partner_session.session_command_tx();
    registry.register_or_replace(
        partner_guid,
        broadcast_info_with_command_tx(partner_command_tx),
        Default::default(),
    );
    source_session.set_player_registry(registry);

    source_session
        .handle_accept_trade(accept_trade_packet(42))
        .await;
    partner_session
        .process_represented_session_commands_like_cpp()
        .await;

    assert!(source_session.represented_trade_accepted_like_cpp());
    assert!(source_send_rx.try_recv().is_err());
    let partner_bytes = partner_send_rx.try_recv().expect("partner trade status");
    assert_eq!(
        u16::from_le_bytes([partner_bytes[0], partner_bytes[1]]),
        ServerOpcodes::TradeStatus as u16
    );
    assert_eq!(partner_bytes[2], TRADE_STATUS_ACCEPTED_LIKE_CPP << 2);
}

#[tokio::test]
async fn clear_trade_item_without_active_trade_is_noop_like_cpp() {
    let (mut session, send_rx) = make_session();

    session
        .handle_clear_trade_item(clear_trade_item_packet(2))
        .await;

    assert_eq!(session.represented_trade_client_state_index_like_cpp(), 1);
    assert!(session.represented_trade_item_like_cpp(2).is_none());
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn clear_trade_item_invalid_slot_updates_client_state_only_like_cpp() {
    let (mut session, send_rx) = make_session();
    let partner_guid = ObjectGuid::create_player(1, 88);
    session.set_represented_active_trade_partner_like_cpp(Some(partner_guid));
    session.set_represented_trade_accepted_like_cpp_for_test(true);

    session
        .handle_clear_trade_item(clear_trade_item_packet(7))
        .await;

    assert_eq!(session.represented_trade_client_state_index_like_cpp(), 2);
    assert_eq!(session.represented_trade_server_state_index_like_cpp(), 1);
    assert!(session.represented_trade_accepted_like_cpp());
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn clear_trade_item_empty_slot_only_updates_client_state_like_cpp() {
    let (mut session, send_rx) = make_session();
    let partner_guid = ObjectGuid::create_player(1, 88);
    session.set_represented_active_trade_partner_like_cpp(Some(partner_guid));
    session.set_represented_trade_accepted_like_cpp_for_test(true);

    session
        .handle_clear_trade_item(clear_trade_item_packet(2))
        .await;

    assert_eq!(session.represented_trade_client_state_index_like_cpp(), 2);
    assert_eq!(session.represented_trade_server_state_index_like_cpp(), 1);
    assert!(session.represented_trade_accepted_like_cpp());
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn clear_trade_item_clears_slot_and_unaccepts_both_sides_like_cpp() {
    let (mut source_session, source_send_rx) = make_session();
    let (mut partner_session, partner_send_rx) = make_session();
    let source_guid = ObjectGuid::create_player(1, 77);
    let partner_guid = ObjectGuid::create_player(1, 88);
    let item_guid = ObjectGuid::create_item(1, 1234);
    source_session.set_player_guid(Some(source_guid));
    partner_session.set_player_guid(Some(partner_guid));
    source_session.set_represented_active_trade_partner_like_cpp(Some(partner_guid));
    partner_session.set_represented_active_trade_partner_like_cpp(Some(source_guid));
    source_session.set_represented_trade_item_like_cpp_for_test(2, item_guid);
    source_session.set_represented_trade_accepted_like_cpp_for_test(true);
    partner_session.set_represented_trade_accepted_like_cpp_for_test(true);

    let registry = Arc::new(PlayerRegistry::default());
    let partner_command_tx = partner_session.session_command_tx();
    registry.register_or_replace(
        partner_guid,
        broadcast_info_with_command_tx(partner_command_tx),
        Default::default(),
    );
    source_session.set_player_registry(registry);

    source_session
        .handle_clear_trade_item(clear_trade_item_packet(2))
        .await;
    partner_session
        .process_represented_session_commands_like_cpp()
        .await;

    assert_eq!(
        source_session.represented_trade_client_state_index_like_cpp(),
        2
    );
    assert_eq!(
        source_session.represented_trade_server_state_index_like_cpp(),
        2
    );
    assert!(source_session.represented_trade_item_like_cpp(2).is_none());
    assert!(!source_session.represented_trade_accepted_like_cpp());
    assert!(!partner_session.represented_trade_accepted_like_cpp());

    let source_bytes = source_send_rx.try_recv().expect("source unaccepted status");
    let partner_bytes = partner_send_rx
        .try_recv()
        .expect("partner unaccepted status");
    assert_eq!(source_bytes, partner_bytes);
    assert_eq!(
        u16::from_le_bytes([source_bytes[0], source_bytes[1]]),
        ServerOpcodes::TradeStatus as u16
    );
    assert_eq!(source_bytes[2], TRADE_STATUS_UNACCEPTED_LIKE_CPP << 2);
}

#[tokio::test]
async fn set_trade_item_without_active_trade_is_noop_like_cpp() {
    let (mut session, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 77);
    let item_guid = ObjectGuid::create_item(1, 1234);
    session.set_player_guid(Some(player_guid));
    insert_trade_test_item(&mut session, player_guid, 23, item_guid, 700);

    session
        .handle_set_trade_item(set_trade_item_packet(2, 255, 23))
        .await;

    assert_eq!(session.represented_trade_client_state_index_like_cpp(), 1);
    assert!(session.represented_trade_item_like_cpp(2).is_none());
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn set_trade_item_invalid_slot_cancels_without_client_state_like_cpp() {
    let (mut session, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 77);
    let partner_guid = ObjectGuid::create_player(1, 88);
    let item_guid = ObjectGuid::create_item(1, 1234);
    session.set_player_guid(Some(player_guid));
    session.set_represented_active_trade_partner_like_cpp(Some(partner_guid));
    insert_trade_test_item(&mut session, player_guid, 23, item_guid, 700);

    session
        .handle_set_trade_item(set_trade_item_packet(7, 255, 23))
        .await;

    assert_eq!(session.represented_trade_client_state_index_like_cpp(), 1);
    assert!(session.represented_trade_item_like_cpp(2).is_none());
    let bytes = send_rx.try_recv().expect("cancelled trade status");
    assert_eq!(
        u16::from_le_bytes([bytes[0], bytes[1]]),
        ServerOpcodes::TradeStatus as u16
    );
    assert_eq!(bytes[2], TRADE_STATUS_CANCELLED_LIKE_CPP << 2);
}

#[tokio::test]
async fn set_trade_item_missing_inventory_cancels_without_client_state_like_cpp() {
    let (mut session, send_rx) = make_session();
    let partner_guid = ObjectGuid::create_player(1, 88);
    session.set_represented_active_trade_partner_like_cpp(Some(partner_guid));

    session
        .handle_set_trade_item(set_trade_item_packet(2, 255, 23))
        .await;

    assert_eq!(session.represented_trade_client_state_index_like_cpp(), 1);
    assert!(session.represented_trade_item_like_cpp(2).is_none());
    let bytes = send_rx.try_recv().expect("cancelled trade status");
    assert_eq!(bytes[2], TRADE_STATUS_CANCELLED_LIKE_CPP << 2);
}

#[tokio::test]
async fn set_trade_item_duplicate_item_cancels_without_client_state_like_cpp() {
    let (mut session, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 77);
    let partner_guid = ObjectGuid::create_player(1, 88);
    let item_guid = ObjectGuid::create_item(1, 1234);
    session.set_player_guid(Some(player_guid));
    session.set_represented_active_trade_partner_like_cpp(Some(partner_guid));
    session.set_represented_trade_item_like_cpp_for_test(1, item_guid);
    insert_trade_test_item(&mut session, player_guid, 23, item_guid, 700);

    session
        .handle_set_trade_item(set_trade_item_packet(2, 255, 23))
        .await;

    assert_eq!(session.represented_trade_client_state_index_like_cpp(), 1);
    assert_eq!(session.represented_trade_item_like_cpp(1), Some(item_guid));
    assert!(session.represented_trade_item_like_cpp(2).is_none());
    let bytes = send_rx.try_recv().expect("cancelled trade status");
    assert_eq!(bytes[2], TRADE_STATUS_CANCELLED_LIKE_CPP << 2);
}

#[tokio::test]
async fn set_trade_item_records_slot_and_unaccepts_both_sides_like_cpp() {
    let (mut source_session, source_send_rx) = make_session();
    let (mut partner_session, partner_send_rx) = make_session();
    let source_guid = ObjectGuid::create_player(1, 77);
    let partner_guid = ObjectGuid::create_player(1, 88);
    let item_guid = ObjectGuid::create_item(1, 1234);
    source_session.set_player_guid(Some(source_guid));
    partner_session.set_player_guid(Some(partner_guid));
    source_session.set_represented_active_trade_partner_like_cpp(Some(partner_guid));
    partner_session.set_represented_active_trade_partner_like_cpp(Some(source_guid));
    source_session.set_represented_trade_accepted_like_cpp_for_test(true);
    partner_session.set_represented_trade_accepted_like_cpp_for_test(true);
    insert_trade_test_item(&mut source_session, source_guid, 23, item_guid, 700);

    let registry = Arc::new(PlayerRegistry::default());
    let partner_command_tx = partner_session.session_command_tx();
    registry.register_or_replace(
        partner_guid,
        broadcast_info_with_command_tx(partner_command_tx),
        Default::default(),
    );
    source_session.set_player_registry(registry);

    source_session
        .handle_set_trade_item(set_trade_item_packet(2, 255, 23))
        .await;
    partner_session
        .process_represented_session_commands_like_cpp()
        .await;

    assert_eq!(
        source_session.represented_trade_client_state_index_like_cpp(),
        2
    );
    assert_eq!(
        source_session.represented_trade_server_state_index_like_cpp(),
        2
    );
    assert_eq!(
        source_session.represented_trade_item_like_cpp(2),
        Some(item_guid)
    );
    assert!(!source_session.represented_trade_accepted_like_cpp());
    assert!(!partner_session.represented_trade_accepted_like_cpp());

    let source_bytes = source_send_rx.try_recv().expect("source unaccepted status");
    let partner_bytes = partner_send_rx
        .try_recv()
        .expect("partner unaccepted status");
    assert_eq!(source_bytes, partner_bytes);
    assert_eq!(
        u16::from_le_bytes([source_bytes[0], source_bytes[1]]),
        ServerOpcodes::TradeStatus as u16
    );
    assert_eq!(source_bytes[2], TRADE_STATUS_UNACCEPTED_LIKE_CPP << 2);
}

#[tokio::test]
async fn set_trade_gold_without_active_trade_is_noop_like_cpp() {
    let (mut session, send_rx) = make_session();
    session.set_player_gold_like_cpp(100);

    session
        .handle_set_trade_gold(set_trade_gold_packet(50))
        .await;

    assert_eq!(session.represented_trade_client_state_index_like_cpp(), 1);
    assert_eq!(session.represented_trade_money_like_cpp(), 0);
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn set_trade_gold_same_money_only_updates_client_state_like_cpp() {
    let (mut session, send_rx) = make_session();
    let partner_guid = ObjectGuid::create_player(1, 88);
    session.set_player_gold_like_cpp(100);
    session.set_represented_active_trade_partner_like_cpp(Some(partner_guid));

    session
        .handle_set_trade_gold(set_trade_gold_packet(0))
        .await;

    assert_eq!(session.represented_trade_client_state_index_like_cpp(), 2);
    assert_eq!(session.represented_trade_server_state_index_like_cpp(), 1);
    assert_eq!(session.represented_trade_money_like_cpp(), 0);
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn set_trade_gold_not_enough_money_sends_failed_like_cpp() {
    let (mut session, send_rx) = make_session();
    let partner_guid = ObjectGuid::create_player(1, 88);
    session.set_player_gold_like_cpp(10);
    session.set_represented_active_trade_partner_like_cpp(Some(partner_guid));

    session
        .handle_set_trade_gold(set_trade_gold_packet(50))
        .await;

    assert_eq!(session.represented_trade_client_state_index_like_cpp(), 2);
    assert_eq!(session.represented_trade_server_state_index_like_cpp(), 1);
    assert_eq!(session.represented_trade_money_like_cpp(), 0);
    let bytes = send_rx.try_recv().expect("failed trade status");
    assert_eq!(
        u16::from_le_bytes([bytes[0], bytes[1]]),
        ServerOpcodes::TradeStatus as u16
    );
    assert_eq!(bytes[2], TRADE_STATUS_FAILED_LIKE_CPP << 2);
    assert_eq!(
        i32::from_le_bytes([bytes[3], bytes[4], bytes[5], bytes[6]]),
        EQUIP_ERR_NOT_ENOUGH_MONEY_LIKE_CPP
    );
    assert_eq!(
        i32::from_le_bytes([bytes[7], bytes[8], bytes[9], bytes[10]]),
        0
    );
}

#[tokio::test]
async fn set_trade_gold_records_money_and_unaccepts_both_sides_like_cpp() {
    let (mut source_session, source_send_rx) = make_session();
    let (mut partner_session, partner_send_rx) = make_session();
    let source_guid = ObjectGuid::create_player(1, 77);
    let partner_guid = ObjectGuid::create_player(1, 88);
    source_session.set_player_guid(Some(source_guid));
    partner_session.set_player_guid(Some(partner_guid));
    source_session.set_player_gold_like_cpp(100);
    source_session.set_represented_active_trade_partner_like_cpp(Some(partner_guid));
    partner_session.set_represented_active_trade_partner_like_cpp(Some(source_guid));
    source_session.set_represented_trade_accepted_like_cpp_for_test(true);
    partner_session.set_represented_trade_accepted_like_cpp_for_test(true);

    let registry = Arc::new(PlayerRegistry::default());
    let partner_command_tx = partner_session.session_command_tx();
    registry.register_or_replace(
        partner_guid,
        broadcast_info_with_command_tx(partner_command_tx),
        Default::default(),
    );
    source_session.set_player_registry(registry);

    source_session
        .handle_set_trade_gold(set_trade_gold_packet(75))
        .await;
    partner_session
        .process_represented_session_commands_like_cpp()
        .await;

    assert_eq!(
        source_session.represented_trade_client_state_index_like_cpp(),
        2
    );
    assert_eq!(
        source_session.represented_trade_server_state_index_like_cpp(),
        2
    );
    assert_eq!(source_session.represented_trade_money_like_cpp(), 75);
    assert!(!source_session.represented_trade_accepted_like_cpp());
    assert!(!partner_session.represented_trade_accepted_like_cpp());

    let source_bytes = source_send_rx.try_recv().expect("source unaccepted status");
    let partner_bytes = partner_send_rx
        .try_recv()
        .expect("partner unaccepted status");
    assert_eq!(source_bytes, partner_bytes);
    assert_eq!(
        u16::from_le_bytes([source_bytes[0], source_bytes[1]]),
        ServerOpcodes::TradeStatus as u16
    );
    assert_eq!(source_bytes[2], TRADE_STATUS_UNACCEPTED_LIKE_CPP << 2);
}

#[tokio::test]
async fn set_trade_spell_without_active_trade_is_noop_like_cpp() {
    let (mut session, send_rx) = make_session();
    install_trade_test_spell(&mut session, 7418);

    session
        .handle_set_trade_spell(set_trade_spell_packet(7418, 255, 23))
        .await;

    assert_eq!(session.represented_trade_client_state_index_like_cpp(), 1);
    assert_eq!(session.represented_trade_server_state_index_like_cpp(), 1);
    assert_eq!(session.represented_trade_spell_like_cpp(), 0);
    assert!(
        session
            .represented_trade_spell_cast_item_like_cpp()
            .is_none()
    );
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn set_trade_spell_zero_clears_spell_and_unaccepts_both_sides_like_cpp() {
    let (mut source_session, source_send_rx) = make_session();
    let (mut partner_session, partner_send_rx) = make_session();
    let source_guid = ObjectGuid::create_player(1, 77);
    let partner_guid = ObjectGuid::create_player(1, 88);
    let cast_item_guid = ObjectGuid::create_item(1, 1234);
    source_session.set_player_guid(Some(source_guid));
    partner_session.set_player_guid(Some(partner_guid));
    source_session.set_represented_active_trade_partner_like_cpp(Some(partner_guid));
    partner_session.set_represented_active_trade_partner_like_cpp(Some(source_guid));
    source_session.set_represented_trade_spell_like_cpp_for_test(7418, Some(cast_item_guid));
    source_session.set_represented_trade_accepted_like_cpp_for_test(true);
    partner_session.set_represented_trade_accepted_like_cpp_for_test(true);

    let registry = Arc::new(PlayerRegistry::default());
    let partner_command_tx = partner_session.session_command_tx();
    registry.register_or_replace(
        partner_guid,
        broadcast_info_with_command_tx(partner_command_tx),
        Default::default(),
    );
    source_session.set_player_registry(registry);

    source_session
        .handle_set_trade_spell(set_trade_spell_packet(0, 0, 255))
        .await;
    partner_session
        .process_represented_session_commands_like_cpp()
        .await;

    assert_eq!(
        source_session.represented_trade_client_state_index_like_cpp(),
        1
    );
    assert_eq!(
        source_session.represented_trade_server_state_index_like_cpp(),
        2
    );
    assert_eq!(source_session.represented_trade_spell_like_cpp(), 0);
    assert!(
        source_session
            .represented_trade_spell_cast_item_like_cpp()
            .is_none()
    );
    assert!(!source_session.represented_trade_accepted_like_cpp());
    assert!(!partner_session.represented_trade_accepted_like_cpp());

    let source_bytes = source_send_rx.try_recv().expect("source unaccepted status");
    let partner_bytes = partner_send_rx
        .try_recv()
        .expect("partner unaccepted status");
    assert_eq!(source_bytes, partner_bytes);
    assert_eq!(
        u16::from_le_bytes([source_bytes[0], source_bytes[1]]),
        ServerOpcodes::TradeStatus as u16
    );
    assert_eq!(source_bytes[2], TRADE_STATUS_UNACCEPTED_LIKE_CPP << 2);
}

#[tokio::test]
async fn set_trade_spell_missing_spell_info_clears_existing_spell_like_cpp() {
    let (mut session, send_rx) = make_session();
    let partner_guid = ObjectGuid::create_player(1, 88);
    let cast_item_guid = ObjectGuid::create_item(1, 1234);
    session.set_represented_active_trade_partner_like_cpp(Some(partner_guid));
    session.set_represented_trade_spell_like_cpp_for_test(7418, Some(cast_item_guid));
    session.set_represented_trade_accepted_like_cpp_for_test(true);

    session
        .handle_set_trade_spell(set_trade_spell_packet(9999, 0, 255))
        .await;

    assert_eq!(session.represented_trade_client_state_index_like_cpp(), 1);
    assert_eq!(session.represented_trade_server_state_index_like_cpp(), 2);
    assert_eq!(session.represented_trade_spell_like_cpp(), 0);
    assert!(
        session
            .represented_trade_spell_cast_item_like_cpp()
            .is_none()
    );
    assert!(!session.represented_trade_accepted_like_cpp());
    let bytes = send_rx.try_recv().expect("unaccepted status");
    assert_eq!(bytes[2], TRADE_STATUS_UNACCEPTED_LIKE_CPP << 2);
}

#[tokio::test]
async fn set_trade_spell_unknown_spell_clears_existing_spell_like_cpp() {
    let (mut session, send_rx) = make_session();
    let partner_guid = ObjectGuid::create_player(1, 88);
    let cast_item_guid = ObjectGuid::create_item(1, 1234);
    let mut spell_store = SpellStore::new();
    spell_store.insert(7418, trade_test_spell_info(7418));
    session.set_spell_store(Arc::new(spell_store));
    session.set_represented_active_trade_partner_like_cpp(Some(partner_guid));
    session.set_represented_trade_spell_like_cpp_for_test(7418, Some(cast_item_guid));
    session.set_represented_trade_accepted_like_cpp_for_test(true);

    session
        .handle_set_trade_spell(set_trade_spell_packet(7418, 0, 255))
        .await;

    assert_eq!(session.represented_trade_client_state_index_like_cpp(), 1);
    assert_eq!(session.represented_trade_server_state_index_like_cpp(), 2);
    assert_eq!(session.represented_trade_spell_like_cpp(), 0);
    assert!(
        session
            .represented_trade_spell_cast_item_like_cpp()
            .is_none()
    );
    assert!(!session.represented_trade_accepted_like_cpp());
    let bytes = send_rx.try_recv().expect("unaccepted status");
    assert_eq!(bytes[2], TRADE_STATUS_UNACCEPTED_LIKE_CPP << 2);
}

#[tokio::test]
async fn set_trade_spell_valid_records_spell_and_cast_item_like_cpp() {
    let (mut source_session, source_send_rx) = make_session();
    let (mut partner_session, partner_send_rx) = make_session();
    let source_guid = ObjectGuid::create_player(1, 77);
    let partner_guid = ObjectGuid::create_player(1, 88);
    let cast_item_guid = ObjectGuid::create_item(1, 1234);
    source_session.set_player_guid(Some(source_guid));
    partner_session.set_player_guid(Some(partner_guid));
    source_session.set_represented_active_trade_partner_like_cpp(Some(partner_guid));
    partner_session.set_represented_active_trade_partner_like_cpp(Some(source_guid));
    source_session.set_represented_trade_accepted_like_cpp_for_test(true);
    partner_session.set_represented_trade_accepted_like_cpp_for_test(true);
    install_trade_test_spell(&mut source_session, 7418);
    insert_trade_test_item(&mut source_session, source_guid, 23, cast_item_guid, 700);

    let registry = Arc::new(PlayerRegistry::default());
    let partner_command_tx = partner_session.session_command_tx();
    registry.register_or_replace(
        partner_guid,
        broadcast_info_with_command_tx(partner_command_tx),
        Default::default(),
    );
    source_session.set_player_registry(registry);

    source_session
        .handle_set_trade_spell(set_trade_spell_packet(7418, 255, 23))
        .await;
    partner_session
        .process_represented_session_commands_like_cpp()
        .await;

    assert_eq!(
        source_session.represented_trade_client_state_index_like_cpp(),
        1
    );
    assert_eq!(
        source_session.represented_trade_server_state_index_like_cpp(),
        2
    );
    assert_eq!(source_session.represented_trade_spell_like_cpp(), 7418);
    assert_eq!(
        source_session.represented_trade_spell_cast_item_like_cpp(),
        Some(cast_item_guid)
    );
    assert!(!source_session.represented_trade_accepted_like_cpp());
    assert!(!partner_session.represented_trade_accepted_like_cpp());

    let source_bytes = source_send_rx.try_recv().expect("source unaccepted status");
    let partner_bytes = partner_send_rx
        .try_recv()
        .expect("partner unaccepted status");
    assert_eq!(source_bytes, partner_bytes);
    assert_eq!(
        u16::from_le_bytes([source_bytes[0], source_bytes[1]]),
        ServerOpcodes::TradeStatus as u16
    );
    assert_eq!(source_bytes[2], TRADE_STATUS_UNACCEPTED_LIKE_CPP << 2);
}

#[tokio::test]
async fn set_trade_spell_same_spell_and_cast_item_is_noop_like_cpp() {
    let (mut session, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 77);
    let partner_guid = ObjectGuid::create_player(1, 88);
    let cast_item_guid = ObjectGuid::create_item(1, 1234);
    session.set_player_guid(Some(player_guid));
    session.set_represented_active_trade_partner_like_cpp(Some(partner_guid));
    session.set_represented_trade_spell_like_cpp_for_test(7418, Some(cast_item_guid));
    session.set_represented_trade_accepted_like_cpp_for_test(true);
    install_trade_test_spell(&mut session, 7418);
    insert_trade_test_item(&mut session, player_guid, 23, cast_item_guid, 700);

    session
        .handle_set_trade_spell(set_trade_spell_packet(7418, 255, 23))
        .await;

    assert_eq!(session.represented_trade_client_state_index_like_cpp(), 1);
    assert_eq!(session.represented_trade_server_state_index_like_cpp(), 1);
    assert_eq!(session.represented_trade_spell_like_cpp(), 7418);
    assert_eq!(
        session.represented_trade_spell_cast_item_like_cpp(),
        Some(cast_item_guid)
    );
    assert!(session.represented_trade_accepted_like_cpp());
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn sign_petition_records_guid_and_choice_like_cpp_without_runtime_mgr() {
    let (mut session, send_rx) = make_session();
    let petition_guid = ObjectGuid::create_item(1, 91_777);

    session
        .handle_sign_petition(sign_petition_packet(petition_guid, 1))
        .await;

    assert_eq!(
        session.represented_sign_petitions_like_cpp(),
        &[crate::session::RepresentedSignPetitionLikeCpp {
            petition_guid,
            choice: 1,
        }]
    );
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn decline_petition_records_guid_like_cpp_without_client_notification() {
    let (mut session, send_rx) = make_session();
    let petition_guid = ObjectGuid::create_item(1, 91_778);

    session
        .handle_decline_petition(decline_petition_packet(petition_guid))
        .await;

    assert_eq!(
        session.represented_decline_petitions_like_cpp(),
        &[crate::session::RepresentedDeclinePetitionLikeCpp { petition_guid }]
    );
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn query_petition_without_runtime_mgr_sends_not_found_like_cpp() {
    let (mut session, send_rx) = make_session();
    let item_guid = ObjectGuid::create_item(1, 91_779);

    session
        .handle_query_petition(query_petition_packet(123, item_guid))
        .await;

    assert_eq!(
        session.represented_query_petitions_like_cpp(),
        &[crate::session::RepresentedQueryPetitionLikeCpp {
            petition_id: 123,
            item_guid,
        }]
    );

    let bytes = send_rx.try_recv().expect("query petition response");
    let mut body = WorldPacket::from_bytes(&bytes);
    assert_eq!(
        body.server_opcode(),
        Some(ServerOpcodes::QueryPetitionResponse)
    );
    assert_eq!(
        body.read_uint16().unwrap(),
        ServerOpcodes::QueryPetitionResponse as u16
    );
    assert_eq!(body.read_uint32().unwrap(), item_guid.counter() as u32);
    assert!(!body.read_bit().unwrap());
    assert_eq!(body.remaining(), 0);
}

#[tokio::test]
async fn unaccept_trade_without_active_trade_is_noop_like_cpp() {
    let (mut session, send_rx) = make_session();

    session
        .handle_unaccept_trade(WorldPacket::new_empty())
        .await;

    assert!(!session.represented_trade_accepted_like_cpp());
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn unaccept_trade_clears_acceptance_and_notifies_partner_like_cpp() {
    let (mut source_session, source_send_rx) = make_session();
    let (mut partner_session, partner_send_rx) = make_session();
    let source_guid = ObjectGuid::create_player(1, 77);
    let partner_guid = ObjectGuid::create_player(1, 88);
    source_session.set_player_guid(Some(source_guid));
    partner_session.set_player_guid(Some(partner_guid));
    source_session.set_represented_active_trade_partner_like_cpp(Some(partner_guid));
    partner_session.set_represented_active_trade_partner_like_cpp(Some(source_guid));
    source_session.set_represented_partner_trade_server_state_index_like_cpp(1);
    source_session.accept_represented_trade_like_cpp(1);
    assert!(source_session.represented_trade_accepted_like_cpp());

    let registry = Arc::new(PlayerRegistry::default());
    let partner_command_tx = partner_session.session_command_tx();
    registry.register_or_replace(
        partner_guid,
        broadcast_info_with_command_tx(partner_command_tx),
        Default::default(),
    );
    source_session.set_player_registry(registry);

    source_session
        .handle_unaccept_trade(WorldPacket::new_empty())
        .await;
    partner_session
        .process_represented_session_commands_like_cpp()
        .await;

    assert!(!source_session.represented_trade_accepted_like_cpp());
    assert!(source_send_rx.try_recv().is_err());
    let partner_bytes = partner_send_rx.try_recv().expect("partner trade status");
    assert_eq!(
        u16::from_le_bytes([partner_bytes[0], partner_bytes[1]]),
        ServerOpcodes::TradeStatus as u16
    );
    assert_eq!(partner_bytes[2], TRADE_STATUS_UNACCEPTED_LIKE_CPP << 2);
}

#[tokio::test]
async fn busy_trade_without_active_trade_is_noop_like_cpp() {
    let (mut session, send_rx) = make_session();

    session.handle_busy_trade(WorldPacket::new_empty()).await;

    assert!(
        session
            .represented_trade_cancel_statuses_like_cpp()
            .is_empty()
    );
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn busy_trade_cancels_represented_trade_and_sends_status_like_cpp() {
    let (mut session, send_rx) = make_session();
    let partner_guid = ObjectGuid::create_player(1, 88);
    session.set_represented_active_trade_partner_like_cpp(Some(partner_guid));

    session.handle_busy_trade(WorldPacket::new_empty()).await;

    assert_eq!(
        session.represented_trade_cancel_statuses_like_cpp(),
        &[TRADE_STATUS_PLAYER_BUSY_LIKE_CPP]
    );
    assert!(
        session
            .represented_active_trade_partner_like_cpp()
            .is_none()
    );
    let bytes = send_rx.try_recv().expect("trade status");
    assert_eq!(
        u16::from_le_bytes([bytes[0], bytes[1]]),
        ServerOpcodes::TradeStatus as u16
    );
    assert_eq!(bytes[2], TRADE_STATUS_PLAYER_BUSY_LIKE_CPP << 1);
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn busy_trade_cancels_partner_represented_trade_like_cpp() {
    let (mut source_session, source_send_rx) = make_session();
    let (mut partner_session, partner_send_rx) = make_session();
    let source_guid = ObjectGuid::create_player(1, 77);
    let partner_guid = ObjectGuid::create_player(1, 88);
    source_session.set_player_guid(Some(source_guid));
    partner_session.set_player_guid(Some(partner_guid));
    source_session.set_represented_active_trade_partner_like_cpp(Some(partner_guid));
    partner_session.set_represented_active_trade_partner_like_cpp(Some(source_guid));

    let registry = Arc::new(PlayerRegistry::default());
    let partner_command_tx = partner_session.session_command_tx();
    registry.register_or_replace(
        partner_guid,
        broadcast_info_with_command_tx(partner_command_tx),
        Default::default(),
    );
    source_session.set_player_registry(registry);

    source_session
        .handle_busy_trade(WorldPacket::new_empty())
        .await;
    partner_session
        .process_represented_session_commands_like_cpp()
        .await;

    assert!(
        source_session
            .represented_active_trade_partner_like_cpp()
            .is_none()
    );
    assert!(
        partner_session
            .represented_active_trade_partner_like_cpp()
            .is_none()
    );
    assert_eq!(
        source_session.represented_trade_cancel_statuses_like_cpp(),
        &[TRADE_STATUS_PLAYER_BUSY_LIKE_CPP]
    );
    assert_eq!(
        partner_session.represented_trade_cancel_statuses_like_cpp(),
        &[TRADE_STATUS_PLAYER_BUSY_LIKE_CPP]
    );

    let source_bytes = source_send_rx.try_recv().expect("source trade status");
    let partner_bytes = partner_send_rx.try_recv().expect("partner trade status");
    assert_eq!(source_bytes, partner_bytes);
    assert_eq!(
        u16::from_le_bytes([source_bytes[0], source_bytes[1]]),
        ServerOpcodes::TradeStatus as u16
    );
    assert_eq!(source_bytes[2], TRADE_STATUS_PLAYER_BUSY_LIKE_CPP << 1);
}

#[tokio::test]
async fn begin_trade_without_active_trade_is_noop_like_cpp() {
    let (mut session, send_rx) = make_session();

    session.handle_begin_trade(WorldPacket::new_empty()).await;

    assert!(
        session
            .represented_active_trade_partner_like_cpp()
            .is_none()
    );
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn begin_trade_sends_initiated_status_like_cpp() {
    let (mut session, send_rx) = make_session();
    let partner_guid = ObjectGuid::create_player(1, 88);
    session.set_represented_active_trade_partner_like_cpp(Some(partner_guid));

    session.handle_begin_trade(WorldPacket::new_empty()).await;

    assert_eq!(
        session.represented_active_trade_partner_like_cpp(),
        Some(partner_guid)
    );
    let bytes = send_rx.try_recv().expect("trade status");
    assert_eq!(
        u16::from_le_bytes([bytes[0], bytes[1]]),
        ServerOpcodes::TradeStatus as u16
    );
    assert_eq!(bytes[2], TRADE_STATUS_INITIATED_LIKE_CPP << 2);
    assert_eq!(
        u32::from_le_bytes([bytes[3], bytes[4], bytes[5], bytes[6]]),
        0
    );
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn begin_trade_sends_initiated_status_to_partner_like_cpp() {
    let (mut source_session, source_send_rx) = make_session();
    let (mut partner_session, partner_send_rx) = make_session();
    let source_guid = ObjectGuid::create_player(1, 77);
    let partner_guid = ObjectGuid::create_player(1, 88);
    source_session.set_player_guid(Some(source_guid));
    partner_session.set_player_guid(Some(partner_guid));
    source_session.set_represented_active_trade_partner_like_cpp(Some(partner_guid));
    partner_session.set_represented_active_trade_partner_like_cpp(Some(source_guid));

    let registry = Arc::new(PlayerRegistry::default());
    let partner_command_tx = partner_session.session_command_tx();
    registry.register_or_replace(
        partner_guid,
        broadcast_info_with_command_tx(partner_command_tx),
        Default::default(),
    );
    source_session.set_player_registry(registry);

    source_session
        .handle_begin_trade(WorldPacket::new_empty())
        .await;
    partner_session
        .process_represented_session_commands_like_cpp()
        .await;

    assert_eq!(
        source_session.represented_active_trade_partner_like_cpp(),
        Some(partner_guid)
    );
    assert_eq!(
        partner_session.represented_active_trade_partner_like_cpp(),
        Some(source_guid)
    );

    let source_bytes = source_send_rx.try_recv().expect("source trade status");
    let partner_bytes = partner_send_rx.try_recv().expect("partner trade status");
    assert_eq!(source_bytes, partner_bytes);
    assert_eq!(
        u16::from_le_bytes([source_bytes[0], source_bytes[1]]),
        ServerOpcodes::TradeStatus as u16
    );
    assert_eq!(source_bytes[2], TRADE_STATUS_INITIATED_LIKE_CPP << 2);
    assert_eq!(
        u32::from_le_bytes([
            source_bytes[3],
            source_bytes[4],
            source_bytes[5],
            source_bytes[6]
        ]),
        0
    );
}

#[tokio::test]
async fn ignore_trade_without_active_trade_is_noop_like_cpp() {
    let (mut session, send_rx) = make_session();

    session.handle_ignore_trade(WorldPacket::new_empty()).await;

    assert!(
        session
            .represented_trade_cancel_statuses_like_cpp()
            .is_empty()
    );
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn ignore_trade_cancels_represented_trade_and_sends_status_like_cpp() {
    let (mut session, send_rx) = make_session();
    let partner_guid = ObjectGuid::create_player(1, 88);
    session.set_represented_active_trade_partner_like_cpp(Some(partner_guid));

    session.handle_ignore_trade(WorldPacket::new_empty()).await;

    assert_eq!(
        session.represented_trade_cancel_statuses_like_cpp(),
        &[TRADE_STATUS_PLAYER_IGNORED_LIKE_CPP]
    );
    assert!(
        session
            .represented_active_trade_partner_like_cpp()
            .is_none()
    );
    let bytes = send_rx.try_recv().expect("trade status");
    assert_eq!(
        u16::from_le_bytes([bytes[0], bytes[1]]),
        ServerOpcodes::TradeStatus as u16
    );
    assert_eq!(bytes[2], TRADE_STATUS_PLAYER_IGNORED_LIKE_CPP << 2);
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn ignore_trade_cancels_partner_represented_trade_like_cpp() {
    let (mut source_session, source_send_rx) = make_session();
    let (mut partner_session, partner_send_rx) = make_session();
    let source_guid = ObjectGuid::create_player(1, 77);
    let partner_guid = ObjectGuid::create_player(1, 88);
    source_session.set_player_guid(Some(source_guid));
    partner_session.set_player_guid(Some(partner_guid));
    source_session.set_represented_active_trade_partner_like_cpp(Some(partner_guid));
    partner_session.set_represented_active_trade_partner_like_cpp(Some(source_guid));

    let registry = Arc::new(PlayerRegistry::default());
    let partner_command_tx = partner_session.session_command_tx();
    registry.register_or_replace(
        partner_guid,
        broadcast_info_with_command_tx(partner_command_tx),
        Default::default(),
    );
    source_session.set_player_registry(registry);

    source_session
        .handle_ignore_trade(WorldPacket::new_empty())
        .await;
    partner_session
        .process_represented_session_commands_like_cpp()
        .await;

    assert!(
        source_session
            .represented_active_trade_partner_like_cpp()
            .is_none()
    );
    assert!(
        partner_session
            .represented_active_trade_partner_like_cpp()
            .is_none()
    );
    assert_eq!(
        source_session.represented_trade_cancel_statuses_like_cpp(),
        &[TRADE_STATUS_PLAYER_IGNORED_LIKE_CPP]
    );
    assert_eq!(
        partner_session.represented_trade_cancel_statuses_like_cpp(),
        &[TRADE_STATUS_PLAYER_IGNORED_LIKE_CPP]
    );

    let source_bytes = source_send_rx.try_recv().expect("source trade status");
    let partner_bytes = partner_send_rx.try_recv().expect("partner trade status");
    assert_eq!(source_bytes, partner_bytes);
    assert_eq!(
        u16::from_le_bytes([source_bytes[0], source_bytes[1]]),
        ServerOpcodes::TradeStatus as u16
    );
    assert_eq!(source_bytes[2], TRADE_STATUS_PLAYER_IGNORED_LIKE_CPP << 2);
}
