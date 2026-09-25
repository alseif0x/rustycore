//! Contents packets.
//!
//! Separated from trade.rs under #709.

use super::*;

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
