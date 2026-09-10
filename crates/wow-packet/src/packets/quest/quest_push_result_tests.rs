//! Quest push result tests for the quest module.
//!
//! Separated from quest.rs under #687.

use super::*;

#[test]
fn quest_push_result_response_writes_empty_title_like_cpp() {
    let response = QuestPushResultResponse {
        sender_guid: ObjectGuid::EMPTY,
        result: quest_push_reason::NOT_ALLOWED,
        quest_title: String::new(),
    };

    let bytes = response.to_bytes();

    assert_eq!(
        &bytes,
        &[
            0x90, 0x2A, // SMSG_QUEST_PUSH_RESULT
            0x00, 0x00, // empty packed ObjectGuid masks
            19,   // NotAllowed
            0x00, 0x00, // 9-bit empty title length + flush padding
        ]
    );
    assert_eq!(
        QuestPushResultResponse::OPCODE,
        ServerOpcodes::QuestPushResult
    );
}

#[test]
fn quest_push_result_response_writes_title_length_bits_and_string_like_cpp() {
    let response = QuestPushResultResponse {
        sender_guid: ObjectGuid::EMPTY,
        result: quest_push_reason::NOT_DAILY,
        quest_title: String::from("Hi"),
    };

    let bytes = response.to_bytes();

    assert_eq!(
        &bytes,
        &[
            0x90, 0x2A, // SMSG_QUEST_PUSH_RESULT
            0x00, 0x00, // empty packed ObjectGuid masks
            14,   // NotDaily
            0x01, 0x00, // 9-bit length 2: 00000001 0xxxxxxx after flush
            b'H', b'i',
        ]
    );
}

#[test]
fn quest_push_result_reads_sender_quest_id_result_in_cpp_order() {
    let sender_guid = ObjectGuid::create_player(1, 0x1234);
    let mut pkt = WorldPacket::new_empty();
    pkt.write_packed_guid(&sender_guid);
    pkt.write_uint32(0xA1B2_C3D4);
    pkt.write_uint8(0x2A);

    let parsed = QuestPushResult::read(&mut pkt).expect("valid QuestPushResult packet");

    assert_eq!(parsed.sender_guid, sender_guid);
    assert_eq!(parsed.quest_id, 0xA1B2_C3D4);
    assert_eq!(parsed.result, 0x2A);
}

#[test]
fn quest_push_result_short_packet_fails_closed() {
    let sender_guid = ObjectGuid::create_player(1, 0x1234);
    let mut pkt = WorldPacket::new_empty();
    pkt.write_packed_guid(&sender_guid);
    pkt.write_uint32(0xA1B2_C3D4);
    let mut short = WorldPacket::from_bytes(&pkt.into_data());

    assert!(QuestPushResult::read(&mut short).is_err());
}
