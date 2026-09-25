// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! account_data capability handler tests.

use super::*;
use wow_packet::packets::misc::{
    MAX_ACCOUNT_DATA_SIZE_LIKE_CPP, NUM_ACCOUNT_DATA_TYPES, compress_account_data_like_cpp,
    decompress_account_data_like_cpp,
};

#[tokio::test]
async fn update_account_data_stores_decompressed_cstring_like_cpp() {
    let (mut session, _send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    session.set_player_guid(Some(player_guid));

    session
        .handle_update_account_data(update_account_data_packet(
            player_guid,
            4,
            1234,
            "macros-cache",
        ))
        .await;

    let account_data = session.account_data_like_cpp(4).unwrap();
    assert_eq!(account_data.time, 1234);
    assert_eq!(account_data.data, "macros-cache");
}

#[tokio::test]
async fn update_account_data_size_zero_erases_like_cpp() {
    let (mut session, _send_rx) = make_session();
    assert!(session.set_account_data_like_cpp(4, 999, "old-cache".to_string()));
    let player_guid = ObjectGuid::create_player(1, 42);
    let mut pkt = WorldPacket::new_empty();
    pkt.write_packed_guid(&player_guid);
    pkt.write_int64(1234);
    pkt.write_uint32(0);
    pkt.write_bits(4, 4);
    pkt.write_uint32(0);

    session.handle_update_account_data(pkt).await;

    let account_data = session.account_data_like_cpp(4).unwrap();
    assert_eq!(account_data.time, 0);
    assert!(account_data.data.is_empty());
}

#[tokio::test]
async fn update_account_data_ignores_per_character_data_without_recent_player_guid_like_cpp() {
    let (mut session, _send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);

    session
        .handle_update_account_data(update_account_data_packet(
            player_guid,
            1,
            1234,
            "layout-cache",
        ))
        .await;

    let account_data = session.account_data_like_cpp(1).unwrap();
    assert_eq!(account_data.time, 0);
    assert!(account_data.data.is_empty());
}

#[tokio::test]
async fn update_account_data_accepts_per_character_data_after_logout_like_cpp() {
    let (mut session, _send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    session.set_player_guid(Some(player_guid));
    session.set_player_guid(None);

    session
        .handle_update_account_data(update_account_data_packet(
            player_guid,
            1,
            1234,
            "SET trackedQuests \"v11#|h#|U$2=\"\r\n",
        ))
        .await;

    let account_data = session.account_data_like_cpp(1).unwrap();
    assert_eq!(account_data.time, 1234);
    assert_eq!(account_data.data, "SET trackedQuests \"v11#|h#|U$2=\"\r\n");
}

#[tokio::test]
async fn update_account_data_rejects_invalid_type_and_oversize_like_cpp() {
    let (mut session, _send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);

    session
        .handle_update_account_data(update_account_data_packet(
            player_guid,
            NUM_ACCOUNT_DATA_TYPES as u8,
            1234,
            "ignored",
        ))
        .await;
    assert!(session.account_data_like_cpp(0).unwrap().data.is_empty());

    let compressed_data = compress_account_data_like_cpp("ignored").unwrap();
    let mut oversized = WorldPacket::new_empty();
    oversized.write_packed_guid(&player_guid);
    oversized.write_int64(1234);
    oversized.write_uint32(MAX_ACCOUNT_DATA_SIZE_LIKE_CPP + 1);
    oversized.write_bits(4, 4);
    oversized.write_uint32(compressed_data.len() as u32);
    oversized.write_bytes(&compressed_data);

    session.handle_update_account_data(oversized).await;
    assert!(session.account_data_like_cpp(4).unwrap().data.is_empty());
}

#[tokio::test]
async fn request_account_data_sends_update_account_data_like_cpp() {
    let (mut session, instance_rx, realm_rx) = make_session_with_realm_send();
    let player_guid = ObjectGuid::create_player(1, 42);
    session.set_player_guid(Some(player_guid));
    assert!(session.set_account_data_like_cpp(4, 5678, "macro-cache".to_string()));

    session
        .handle_request_account_data(request_account_data_packet(player_guid, 4))
        .await;

    assert!(instance_rx.try_recv().is_err());
    let encoded = realm_rx.try_recv().unwrap();
    let mut packet = WorldPacket::new_client(encoded.as_slice().into());
    assert_eq!(
        packet.server_opcode(),
        Some(wow_constants::ServerOpcodes::UpdateAccountData)
    );
    packet.skip_opcode();
    assert_eq!(packet.read_packed_guid().unwrap(), player_guid);
    assert_eq!(packet.read_int64().unwrap(), 5678);
    let decompressed_size = packet.read_uint32().unwrap();
    assert_eq!(decompressed_size, "macro-cache".len() as u32);
    assert_eq!(packet.read_bits(4).unwrap(), 4);
    let compressed_size = packet.read_uint32().unwrap() as usize;
    let compressed_data = packet.read_bytes(compressed_size).unwrap();
    assert_eq!(
        decompress_account_data_like_cpp(&compressed_data, decompressed_size).unwrap(),
        "macro-cache"
    );
    assert_eq!(packet.remaining(), 0);
}

#[tokio::test]
async fn addon_list_is_silent_like_cpp_log_only_handler() {
    let (mut session, send_rx) = make_session();
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint32(1);
    pkt.write_bits(5, 10);
    pkt.flush_bits();
    pkt.write_string("Atlas");
    pkt.reset_read();

    session.handle_addon_list(pkt).await;

    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn unregister_all_addon_prefixes_preserves_filter_flag_like_cpp() {
    let (mut session, _send_rx) = make_session();
    session.registered_addon_prefixes = vec!["ABC".to_string()];
    session.filter_addon_messages = true;
    assert!(session.is_addon_registered_like_cpp("ABC"));

    session
        .handle_chat_unregister_all_addon_prefixes(WorldPacket::from_bytes(&[]))
        .await;

    assert!(session.registered_addon_prefixes.is_empty());
    assert!(session.filter_addon_messages);
    assert!(!session.is_addon_registered_like_cpp("ABC"));
}

#[tokio::test]
async fn save_cuf_profiles_replaces_and_clears_slots_like_cpp() {
    let (mut session, _send_rx) = make_session();
    assert!(session.represented_save_cuf_profiles_like_cpp(vec![
        cuf_profile("Old0", 10),
        cuf_profile("Old1", 11),
        cuf_profile("Old2", 12),
    ]));

    session
        .handle_save_cuf_profiles(save_cuf_profiles_packet([
            cuf_profile("Raid", 72),
            cuf_profile("Party", 64),
        ]))
        .await;

    let profiles = session.represented_cuf_profiles_like_cpp();
    assert_eq!(profiles[0].as_ref().unwrap().profile_name, "Raid");
    assert_eq!(profiles[0].as_ref().unwrap().frame_height, 72);
    assert_eq!(profiles[1].as_ref().unwrap().profile_name, "Party");
    assert_eq!(profiles[1].as_ref().unwrap().frame_height, 64);
    assert!(profiles[2].is_none());
    assert!(profiles[3].is_none());
    assert!(profiles[4].is_none());
}

#[tokio::test]
async fn save_cuf_profiles_rejects_above_cpp_max_without_mutation() {
    let (mut session, _send_rx) = make_session();
    assert!(session.represented_save_cuf_profiles_like_cpp(vec![cuf_profile("Keep", 10)]));

    session
        .handle_save_cuf_profiles(save_cuf_profiles_packet([
            cuf_profile("A", 1),
            cuf_profile("B", 2),
            cuf_profile("C", 3),
            cuf_profile("D", 4),
            cuf_profile("E", 5),
            cuf_profile("F", 6),
        ]))
        .await;

    let profiles = session.represented_cuf_profiles_like_cpp();
    assert_eq!(profiles[0].as_ref().unwrap().profile_name, "Keep");
    assert!(profiles[1].is_none());
}

#[tokio::test]
async fn report_enabled_addons_is_silent_like_cpp_handle_null() {
    let (mut session, send_rx) = make_session();

    session
        .handle_report_enabled_addons(WorldPacket::new_empty())
        .await;

    assert!(send_rx.try_recv().is_err());
}
