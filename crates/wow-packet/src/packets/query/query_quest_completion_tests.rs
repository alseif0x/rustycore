//! Query quest completion tests for the query module.
//!
//! Separated from query.rs under #687.

use super::*;
use num_traits::ToPrimitive;

fn client_payload(bytes: &[u8]) -> WorldPacket {
    let mut data = Vec::from(
        ClientOpcodes::QueryQuestCompletionNpcs
            .to_u16()
            .unwrap()
            .to_le_bytes(),
    );
    data.extend_from_slice(bytes);
    let mut pkt = WorldPacket::from_bytes(&data);
    pkt.skip_opcode();
    pkt
}

#[test]
fn query_quest_completion_parse_valid_ids() {
    let mut payload = Vec::new();
    payload.extend_from_slice(&2u32.to_le_bytes());
    payload.extend_from_slice(&123i32.to_le_bytes());
    payload.extend_from_slice(&(-45i32).to_le_bytes());

    let mut pkt = client_payload(&payload);
    let parsed = QueryQuestCompletionNpcs::read(&mut pkt).unwrap();

    assert_eq!(parsed.quest_ids, vec![123, -45]);
}

#[test]
fn query_quest_completion_rejects_short_payload() {
    let mut payload = Vec::new();
    payload.extend_from_slice(&1u32.to_le_bytes());
    payload.extend_from_slice(&[0x34, 0x12]);

    let mut pkt = client_payload(&payload);

    assert!(matches!(
        QueryQuestCompletionNpcs::read(&mut pkt),
        Err(PacketError::ReadPastEnd { .. })
    ));
}

#[test]
fn query_quest_completion_rejects_over_cap() {
    let mut payload = Vec::new();
    payload.extend_from_slice(&101u32.to_le_bytes());

    let mut pkt = client_payload(&payload);

    assert!(matches!(
        QueryQuestCompletionNpcs::read(&mut pkt),
        Err(PacketError::TooLarge { size: 101 })
    ));
}

#[test]
fn query_quest_completion_response_serializes_creature_and_masked_go() {
    let response = QuestCompletionNpcResponse {
        quests: vec![QuestCompletionNpc {
            quest_id: 77,
            npcs: vec![1234, 0x8000_5678u32 as i32],
        }],
    };

    let bytes = response.to_bytes();

    let mut expected = Vec::from(
        ServerOpcodes::QuestCompletionNpcResponse
            .to_u16()
            .unwrap()
            .to_le_bytes(),
    );
    expected.extend_from_slice(&1u32.to_le_bytes());
    expected.extend_from_slice(&77i32.to_le_bytes());
    expected.extend_from_slice(&2u32.to_le_bytes());
    expected.extend_from_slice(&1234i32.to_le_bytes());
    expected.extend_from_slice(&0x8000_5678u32.to_le_bytes());

    assert_eq!(bytes, expected);
}
