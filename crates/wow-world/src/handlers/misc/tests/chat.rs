// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! chat capability handler tests.

use super::*;

#[tokio::test]
async fn realm_connection_responses_route_to_realm_channel_like_cpp_after_connect_to() {
    let (mut session, instance_rx, realm_rx) = make_session_with_realm_send();

    session
        .handle_request_lfg_list_blacklist(WorldPacket::new_empty())
        .await;
    session
        .handle_lfg_list_get_status(WorldPacket::new_empty())
        .await;
    session
        .handle_calendar_get_num_pending(WorldPacket::new_empty())
        .await;
    session
        .handle_gm_ticket_get_case_status(WorldPacket::new_empty())
        .await;
    session
        .handle_request_raid_info(WorldPacket::new_empty())
        .await;
    session
        .handle_battle_pet_request_journal_lock(battle_pet_request_journal_lock_packet())
        .await;

    let expected = [
        ServerOpcodes::LfgListUpdateBlacklist,
        ServerOpcodes::LfgUpdateStatus,
        ServerOpcodes::CalendarSendNumPending,
        ServerOpcodes::GmTicketCaseStatus,
        ServerOpcodes::InstanceInfo,
        ServerOpcodes::BattlePetJournalLockAcquired,
        ServerOpcodes::BattlePetJournal,
    ];

    for opcode in expected {
        let bytes = realm_rx.try_recv().expect("realm-routed packet");
        assert_eq!(u16::from_le_bytes([bytes[0], bytes[1]]), opcode as u16);
    }
    assert!(realm_rx.try_recv().is_err());
    assert!(instance_rx.try_recv().is_err());
}

#[tokio::test]
async fn chat_join_channel_invalid_custom_name_sends_notice_like_cpp() {
    let (mut session, send_rx) = make_session();
    let mut pkt = WorldPacket::new_empty();
    pkt.write_int32(0);
    pkt.write_bit(false);
    pkt.write_bit(false);
    pkt.write_bits(4, 7);
    pkt.write_bits(0, 7);
    pkt.write_string("1bad");
    pkt.reset_read();

    session.handle_chat_join_channel(pkt).await;

    let bytes = send_rx.try_recv().expect("channel notify packet");
    assert_eq!(
        u16::from_le_bytes([bytes[0], bytes[1]]),
        ServerOpcodes::ChannelNotify as u16
    );
    let mut payload = WorldPacket::from_bytes(&bytes[2..]);
    assert_eq!(
        payload.read_bits(6).unwrap() as u8,
        wow_packet::packets::chat::CHAT_INVALID_NAME_NOTICE_LIKE_CPP
    );
    assert_eq!(payload.read_bits(7).unwrap(), 4);
}

#[tokio::test]
async fn chat_join_channel_too_long_custom_name_sends_notice_like_cpp() {
    let (mut session, send_rx) = make_session();
    let channel_name = "A".repeat(MAX_CHANNEL_NAME_STR_LIKE_CPP + 1);
    let mut pkt = WorldPacket::new_empty();
    pkt.write_int32(0);
    pkt.write_bit(false);
    pkt.write_bit(false);
    pkt.write_bits(channel_name.len() as u32, 7);
    pkt.write_bits(0, 7);
    pkt.write_string(&channel_name);
    pkt.reset_read();

    session.handle_chat_join_channel(pkt).await;

    let bytes = send_rx.try_recv().expect("channel notify packet");
    assert_eq!(
        u16::from_le_bytes([bytes[0], bytes[1]]),
        ServerOpcodes::ChannelNotify as u16
    );
    let mut payload = WorldPacket::from_bytes(&bytes[2..]);
    assert_eq!(
        payload.read_bits(6).unwrap() as u8,
        wow_packet::packets::chat::CHAT_INVALID_NAME_NOTICE_LIKE_CPP
    );
    assert_eq!(payload.read_bits(7).unwrap(), channel_name.len() as u32);
}

#[test]
fn chat_join_channel_precheck_rejects_too_long_password_like_cpp() {
    let request = JoinChannel {
        chat_channel_id: 0,
        create_voice_session: false,
        internal: false,
        channel_name: "Valid".to_string(),
        password: "p".repeat(MAX_CHANNEL_PASS_STR_LIKE_CPP + 1),
    };

    assert_eq!(
        join_channel_custom_precheck_like_cpp(&request),
        JoinChannelPrecheckLikeCpp::PasswordTooLong
    );
}

#[tokio::test]
async fn chat_leave_channel_empty_request_is_silent_like_cpp() {
    let (mut session, send_rx) = make_session();
    let mut pkt = WorldPacket::new_empty();
    pkt.write_int32(0);
    pkt.write_bits(0, 7);
    pkt.reset_read();

    session.handle_chat_leave_channel(pkt).await;

    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn chat_channel_command_without_channel_mgr_is_silent_like_cpp() {
    let (mut session, send_rx) = make_session();
    let mut pkt = WorldPacket::new_empty();
    pkt.write_bits(5, 7);
    pkt.write_string("Trade");
    pkt.reset_read();

    session.handle_chat_channel_command(pkt).await;

    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn chat_channel_player_command_too_long_name_is_silent_like_cpp() {
    let (mut session, send_rx) = make_session();
    let player_name = "P".repeat(MAX_CHANNEL_NAME_STR_LIKE_CPP);
    let mut pkt = WorldPacket::new_empty();
    pkt.write_bits(5, 7);
    pkt.write_bits(player_name.len() as u32, 9);
    pkt.write_string("Trade");
    pkt.write_string(&player_name);
    pkt.reset_read();

    session.handle_chat_channel_player_command(pkt).await;

    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn chat_channel_password_without_channel_mgr_is_silent_like_cpp() {
    let (mut session, send_rx) = make_session();
    let mut pkt = WorldPacket::new_empty();
    pkt.write_bits(5, 7);
    pkt.write_bits(4, 7);
    pkt.write_string("Trade");
    pkt.write_string("pass");
    pkt.reset_read();

    session.handle_chat_channel_password(pkt).await;

    assert!(send_rx.try_recv().is_err());
}
