//! Session packets.
//!
//! Separated from trade.rs under #709.

use super::*;

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
