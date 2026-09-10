//! Chat packet regressions.
//!
//! Moved out of chat.rs under #685; every test is unchanged.

use super::*;

#[test]
fn chat_register_addon_prefixes_reads_cpp_layout() {
    let mut writer = WorldPacket::new_empty();
    writer.write_uint32(2);
    writer.write_bits(3, 5);
    writer.write_string("ABC");
    writer.write_bits(4, 5);
    writer.write_string("DEFG");

    let mut reader = WorldPacket::from_bytes(writer.data());
    let packet = ChatRegisterAddonPrefixes::read(&mut reader).unwrap();
    assert_eq!(packet.prefixes, vec!["ABC", "DEFG"]);
}

#[test]
fn join_channel_reads_cpp_layout() {
    let mut writer = WorldPacket::new_empty();
    writer.write_int32(0);
    writer.write_bit(false);
    writer.write_bit(false);
    writer.write_bits(5, 7);
    writer.write_bits(4, 7);
    writer.write_string("Trade");
    writer.write_string("pass");

    let mut reader = WorldPacket::from_bytes(writer.data());
    let packet = JoinChannel::read(&mut reader).unwrap();
    assert_eq!(packet.chat_channel_id, 0);
    assert!(!packet.create_voice_session);
    assert!(!packet.internal);
    assert_eq!(packet.channel_name, "Trade");
    assert_eq!(packet.password, "pass");
}

#[test]
fn leave_channel_reads_cpp_layout() {
    let mut writer = WorldPacket::new_empty();
    writer.write_int32(2);
    writer.write_bits(5, 7);
    writer.write_string("Trade");

    let mut reader = WorldPacket::from_bytes(writer.data());
    let packet = LeaveChannel::read(&mut reader).unwrap();
    assert_eq!(packet.zone_channel_id, 2);
    assert_eq!(packet.channel_name, "Trade");
}

#[test]
fn channel_command_reads_cpp_layout() {
    let mut writer = WorldPacket::new_empty();
    writer.write_bits(7, 7);
    writer.write_string("Looking");

    let mut reader = WorldPacket::from_bytes(writer.data());
    let packet = ChannelCommand::read(&mut reader).unwrap();
    assert_eq!(packet.channel_name, "Looking");
}

#[test]
fn channel_player_command_reads_cpp_layout() {
    let mut writer = WorldPacket::new_empty();
    writer.write_bits(5, 7);
    writer.write_bits(6, 9);
    writer.write_string("Trade");
    writer.write_string("Player");

    let mut reader = WorldPacket::from_bytes(writer.data());
    let packet = ChannelPlayerCommand::read(&mut reader).unwrap();
    assert_eq!(packet.channel_name, "Trade");
    assert_eq!(packet.name, "Player");
}

#[test]
fn channel_password_reads_cpp_layout() {
    let mut writer = WorldPacket::new_empty();
    writer.write_bits(5, 7);
    writer.write_bits(4, 7);
    writer.write_string("Trade");
    writer.write_string("pass");

    let mut reader = WorldPacket::from_bytes(writer.data());
    let packet = ChannelPassword::read(&mut reader).unwrap();
    assert_eq!(packet.channel_name, "Trade");
    assert_eq!(packet.password, "pass");
}

#[test]
fn channel_notify_invalid_name_uses_cpp_notice_type() {
    let bytes = ChannelNotify::invalid_name("1bad").to_bytes();
    assert_eq!(
        u16::from_le_bytes([bytes[0], bytes[1]]),
        ServerOpcodes::ChannelNotify as u16
    );
    // C++ `operator<<(ObjectGuid)` writes packed GUIDs. Empty sender/account/target
    // GUIDs are 2 bytes each, not three raw 16-byte values.
    assert_eq!(bytes.len(), 27);

    let mut payload = WorldPacket::from_bytes(&bytes[2..]);
    assert_eq!(
        payload.read_bits(6).unwrap() as u8,
        CHAT_INVALID_NAME_NOTICE_LIKE_CPP
    );
    assert_eq!(payload.read_bits(7).unwrap(), 4);
    assert_eq!(payload.read_bits(6).unwrap(), 0);
    assert_eq!(payload.read_packed_guid().unwrap(), ObjectGuid::EMPTY);
    assert_eq!(payload.read_packed_guid().unwrap(), ObjectGuid::EMPTY);
    assert_eq!(payload.read_uint32().unwrap(), 0);
    assert_eq!(payload.read_packed_guid().unwrap(), ObjectGuid::EMPTY);
    assert_eq!(payload.read_uint32().unwrap(), 0);
    assert_eq!(payload.read_int32().unwrap(), 0);
    assert_eq!(payload.read_string(4).unwrap(), "1bad");
    assert!(payload.is_empty());
}

#[test]
fn chat_addon_message_reads_cpp_layout() {
    let mut writer = WorldPacket::new_empty();
    writer.write_bits(3, 5);
    writer.write_bits(5, 8);
    writer.write_bit(true);
    writer.write_int32(ChatMsg::Guild as i32);
    writer.write_string("ABC");
    writer.write_string("hello");

    let mut reader = WorldPacket::from_bytes(writer.data());
    let packet = ChatAddonMessage::read(&mut reader).unwrap();
    assert_eq!(packet.msg_type, ChatMsg::Guild as i32);
    assert_eq!(packet.prefix, "ABC");
    assert_eq!(packet.text, "hello");
    assert!(packet.is_logged);
}

#[test]
fn chat_addon_message_whisper_reads_cpp_layout() {
    let mut writer = WorldPacket::new_empty();
    writer.write_bits(6, 9);
    writer.write_bits(3, 5);
    writer.write_bits(5, 8);
    writer.write_string("Target");
    writer.write_string("ABC");
    writer.write_string("hello");

    let mut reader = WorldPacket::from_bytes(writer.data());
    let packet = ChatAddonMessageWhisper::read(&mut reader).unwrap();
    assert_eq!(packet.target, "Target");
    assert_eq!(packet.prefix, "ABC");
    assert_eq!(packet.message, "hello");
}

#[test]
fn chat_addon_message_targeted_reads_cpp_layout() {
    let channel_guid = ObjectGuid::create_player(1, 42);
    let mut writer = WorldPacket::new_empty();
    writer.write_bits(6, 9);
    writer.write_bits(3, 5);
    writer.write_bits(5, 8);
    writer.write_bit(true);
    writer.write_int32(ChatMsg::Whisper as i32);
    writer.write_string("ABC");
    writer.write_string("hello");
    writer.write_packed_guid(&channel_guid);
    writer.write_string("Target");

    let mut reader = WorldPacket::from_bytes(writer.data());
    let packet = ChatAddonMessageTargeted::read(&mut reader).unwrap();
    assert_eq!(packet.target, "Target");
    assert_eq!(packet.channel_guid, channel_guid);
    assert_eq!(packet.params.msg_type, ChatMsg::Whisper as i32);
    assert_eq!(packet.params.prefix, "ABC");
    assert_eq!(packet.params.text, "hello");
    assert!(packet.params.is_logged);
}

#[test]
fn chat_message_without_secure_bit_defaults_like_cpp() {
    let mut writer = WorldPacket::new_empty();
    writer.write_uint16(ClientOpcodes::ChatMessageYell as u16);
    writer.write_int32(0);
    writer.write_bits(5, 11);
    writer.write_string("hello");

    let mut reader = WorldPacket::from_bytes(writer.data());
    reader.skip_opcode();
    let packet = ChatMessage::read(&mut reader).unwrap();

    assert_eq!(packet.text, "hello");
    assert!(!packet.is_secure);
}

#[test]
fn chat_player_notfound_writes_cpp_layout() {
    let packet = ChatPlayerNotfound {
        name: "Missing".to_string(),
    };
    let data = packet.to_bytes();
    let mut payload = WorldPacket::from_bytes(&data);

    assert_eq!(
        payload.read_uint16().unwrap(),
        ServerOpcodes::ChatPlayerNotfound as u16
    );
    assert_eq!(payload.read_bits(9).unwrap(), 7);
    assert_eq!(payload.read_string(7).unwrap(), "Missing");
    assert!(payload.is_empty());
}

#[test]
fn print_notification_writes_cpp_layout() {
    let packet = PrintNotification {
        notify_text: "Server restarting".to_string(),
    };
    let data = packet.to_bytes();
    let mut payload = WorldPacket::from_bytes(&data);

    assert_eq!(
        payload.read_uint16().unwrap(),
        ServerOpcodes::PrintNotification as u16
    );
    assert_eq!(payload.read_bits(12).unwrap(), 17);
    assert_eq!(payload.read_string(17).unwrap(), "Server restarting");
    assert!(payload.is_empty());
}

#[test]
fn chat_server_message_writes_cpp_layout() {
    let packet = ChatServerMessage {
        message_id: 42,
        string_param: "param".to_string(),
    };
    let data = packet.to_bytes();
    let mut payload = WorldPacket::from_bytes(&data);

    assert_eq!(
        payload.read_uint16().unwrap(),
        ServerOpcodes::ChatServerMessage as u16
    );
    assert_eq!(payload.read_int32().unwrap(), 42);
    assert_eq!(payload.read_bits(11).unwrap(), 5);
    assert_eq!(payload.read_string(5).unwrap(), "param");
    assert!(payload.is_empty());
}

#[test]
fn defense_message_writes_cpp_layout() {
    let packet = DefenseMessage {
        zone_id: 1519,
        message_text: "Stormwind is under attack!".to_string(),
    };
    let data = packet.to_bytes();
    let mut payload = WorldPacket::from_bytes(&data);

    assert_eq!(
        payload.read_uint16().unwrap(),
        ServerOpcodes::DefenseMessage as u16
    );
    assert_eq!(payload.read_int32().unwrap(), 1519);
    assert_eq!(payload.read_bits(12).unwrap(), 26);
    assert_eq!(
        payload.read_string(26).unwrap(),
        "Stormwind is under attack!"
    );
    assert!(payload.is_empty());
}

#[test]
fn chat_player_ambiguous_writes_cpp_layout() {
    let packet = ChatPlayerAmbiguous {
        name: "Jaina".to_string(),
    };
    let data = packet.to_bytes();
    let mut payload = WorldPacket::from_bytes(&data);

    assert_eq!(
        payload.read_uint16().unwrap(),
        ServerOpcodes::ChatPlayerAmbiguous as u16
    );
    assert_eq!(payload.read_bits(9).unwrap(), 5);
    assert_eq!(payload.read_string(5).unwrap(), "Jaina");
    assert!(payload.is_empty());
}

#[test]
fn chat_restricted_writes_cpp_layout() {
    let packet = ChatRestricted {
        reason: ChatRestrictionTypeLikeCpp::YellRestricted,
    };
    let data = packet.to_bytes();
    let mut payload = WorldPacket::from_bytes(&data);

    assert_eq!(
        payload.read_uint16().unwrap(),
        ServerOpcodes::ChatRestricted as u16
    );
    assert_eq!(payload.read_uint8().unwrap(), 3);
    assert!(payload.is_empty());
}

#[test]
fn chat_report_ignored_reads_cpp_layout() {
    let ignored_guid = ObjectGuid::create_player(0, 0x12345);
    let mut writer = WorldPacket::new_empty();
    writer.write_packed_guid(&ignored_guid);
    writer.write_uint8(2);

    let mut reader = WorldPacket::from_bytes(writer.data());
    let packet = ChatReportIgnored::read(&mut reader).unwrap();

    assert_eq!(packet.ignored_guid, ignored_guid);
    assert_eq!(packet.reason, 2);
    assert!(reader.is_empty());
}

#[test]
fn chat_report_filtered_reads_cpp_empty_layout() {
    let mut reader = WorldPacket::from_bytes(&[]);
    ChatReportFiltered::read(&mut reader).unwrap();
    assert!(reader.is_empty());
}

#[test]
fn chat_message_afk_reads_cpp_layout() {
    let mut writer = WorldPacket::new_empty();
    writer.write_bits(9, 11);
    writer.write_string("bio break");

    let mut reader = WorldPacket::from_bytes(writer.data());
    let packet = ChatMessageAfk::read(&mut reader).unwrap();

    assert_eq!(packet.text, "bio break");
    assert!(reader.is_empty());
}

#[test]
fn chat_message_dnd_reads_cpp_layout() {
    let mut writer = WorldPacket::new_empty();
    writer.write_bits(4, 11);
    writer.write_string("busy");

    let mut reader = WorldPacket::from_bytes(writer.data());
    let packet = ChatMessageDnd::read(&mut reader).unwrap();

    assert_eq!(packet.text, "busy");
    assert!(reader.is_empty());
}

#[test]
fn chat_message_channel_reads_cpp_layout_without_secure_bit() {
    let channel_guid = ObjectGuid::create_player(1, 0x1122);
    let mut writer = WorldPacket::new_empty();
    writer.write_int32(7);
    writer.write_packed_guid(&channel_guid);
    writer.write_bits(7, 9);
    writer.write_bits(5, 11);
    writer.write_bit(false);
    writer.write_string("General");
    writer.write_string("hello");

    let mut reader = WorldPacket::from_bytes(writer.data());
    let packet = ChatMessageChannel::read(&mut reader).unwrap();

    assert_eq!(packet.language, 7);
    assert_eq!(packet.channel_guid, channel_guid);
    assert_eq!(packet.target, "General");
    assert_eq!(packet.text, "hello");
    assert_eq!(packet.is_secure, None);
    assert!(reader.is_empty());
}

#[test]
fn update_aadc_status_reads_cpp_layout() {
    let mut writer = WorldPacket::new_empty();
    writer.write_bit(true);
    writer.flush_bits();

    let mut reader = WorldPacket::from_bytes(writer.data());
    let packet = UpdateAadcStatus::read(&mut reader).unwrap();

    assert!(packet.chat_disabled);
    assert!(reader.is_empty());
}

#[test]
fn update_aadc_status_response_writes_cpp_layout() {
    let bytes = UpdateAadcStatusResponse {
        success: true,
        chat_disabled: false,
    }
    .to_bytes();
    let mut reader = WorldPacket::from_bytes(&bytes);

    assert_eq!(
        reader.read_uint16().unwrap(),
        ServerOpcodes::UpdateAadcStatusResponse as u16
    );
    assert!(reader.read_bit().unwrap());
    assert!(!reader.read_bit().unwrap());
    assert!(reader.is_empty());
}

#[test]
fn ctext_emote_reads_visual_kit_count_as_uint32_like_cpp() {
    let target = ObjectGuid::create_player(1, 0x44);
    let mut writer = WorldPacket::new_empty();
    writer.write_packed_guid(&target);
    writer.write_int32(66);
    writer.write_int32(7);
    writer.write_uint32(2);
    writer.write_int32(9);
    writer.write_int32(101);
    writer.write_int32(202);

    let mut reader = WorldPacket::from_bytes(writer.data());
    let packet = CTextEmote::read(&mut reader).unwrap();

    assert_eq!(packet.target, target);
    assert_eq!(packet.emote_id, 66);
    assert_eq!(packet.sound_index, 7);
    assert_eq!(packet.sequence_variation, 9);
    assert_eq!(packet.spell_visual_kit_ids, vec![101, 202]);
    assert!(reader.is_empty());
}

#[test]
fn ctext_emote_rejects_visual_kit_count_larger_than_remaining_payload() {
    let target = ObjectGuid::create_player(1, 0x45);
    let mut writer = WorldPacket::new_empty();
    writer.write_packed_guid(&target);
    writer.write_int32(66);
    writer.write_int32(7);
    writer.write_uint32(u32::MAX);
    writer.write_int32(9);

    let mut reader = WorldPacket::from_bytes(writer.data());
    let err = match CTextEmote::read(&mut reader) {
        Ok(_) => panic!("oversized visual kit count should fail"),
        Err(err) => err,
    };

    assert!(matches!(
        err,
        PacketError::ReadPastEnd {
            wanted,
            available: 0
        } if wanted > 0
    ));
}

#[test]
fn chat_pkt_system_universal_writes_cpp_wire_values() {
    let packet = ChatPkt {
        msg_type: ChatMsg::System,
        language: 0,
        sender_guid: ObjectGuid::EMPTY,
        sender_name: String::new(),
        target_guid: ObjectGuid::EMPTY,
        target_name: String::new(),
        prefix: String::new(),
        channel: String::new(),
        text: "hello".to_string(),
        virtual_realm: 0,
    };
    let mut writer = WorldPacket::new_empty();
    packet.write(&mut writer);
    let payload = writer.data();

    assert_eq!(payload[0], 0x00, "CHAT_MSG_SYSTEM must be 0x00 on wire");
    assert_eq!(&payload[1..5], &[0x00, 0x00, 0x00, 0x00]);
}

#[test]
fn chat_pkt_writes_display_time_spell_id_before_bitpacked_chat_flags_like_cpp() {
    let packet = ChatPkt {
        msg_type: ChatMsg::Say,
        language: 7,
        sender_guid: ObjectGuid::EMPTY,
        sender_name: "Luq".to_string(),
        target_guid: ObjectGuid::EMPTY,
        target_name: "Mob".to_string(),
        prefix: "P".to_string(),
        channel: "C".to_string(),
        text: "hello".to_string(),
        virtual_realm: 0x1122_3344,
    };
    let mut writer = WorldPacket::new_empty();
    packet.write(&mut writer);
    let mut reader = WorldPacket::from_bytes(writer.data());

    assert_eq!(reader.read_uint8().unwrap(), ChatMsg::Say as u8);
    assert_eq!(reader.read_uint32().unwrap(), 7);
    assert_eq!(reader.read_packed_guid().unwrap(), ObjectGuid::EMPTY);
    assert_eq!(reader.read_packed_guid().unwrap(), ObjectGuid::EMPTY);
    assert_eq!(reader.read_packed_guid().unwrap(), ObjectGuid::EMPTY);
    assert_eq!(reader.read_packed_guid().unwrap(), ObjectGuid::EMPTY);
    assert_eq!(reader.read_uint32().unwrap(), 0x1122_3344);
    assert_eq!(reader.read_uint32().unwrap(), 0x1122_3344);
    assert_eq!(reader.read_int32().unwrap(), 0);

    // C++ `Chat::Write` emits DisplayTime and SpellID here, then starts the
    // bit block with SenderName length. A uint16 ChatFlags inserted here
    // would make this first length read as zero for this packet.
    assert_eq!(reader.read_float().unwrap(), 0.0);
    assert_eq!(reader.read_int32().unwrap(), 0);
    assert_eq!(reader.read_bits(11).unwrap(), 3);
    assert_eq!(reader.read_bits(11).unwrap(), 3);
    assert_eq!(reader.read_bits(5).unwrap(), 1);
    assert_eq!(reader.read_bits(7).unwrap(), 1);
    assert_eq!(reader.read_bits(12).unwrap(), 5);
    assert_eq!(reader.read_bits(15).unwrap(), 0);
    assert!(!reader.read_bit().unwrap());
    assert!(!reader.read_bit().unwrap());
    assert!(!reader.read_bit().unwrap());
    assert!(!reader.read_bit().unwrap());
    reader.flush_bits();
    assert_eq!(reader.read_string(3).unwrap(), "Luq");
    assert_eq!(reader.read_string(3).unwrap(), "Mob");
    assert_eq!(reader.read_string(1).unwrap(), "P");
    assert_eq!(reader.read_string(1).unwrap(), "C");
    assert_eq!(reader.read_string(5).unwrap(), "hello");
    assert!(reader.is_empty());
}
