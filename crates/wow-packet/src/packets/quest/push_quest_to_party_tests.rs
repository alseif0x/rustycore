//! Push quest to party tests for the quest module.
//!
//! Separated from quest.rs under #687.

use super::*;

#[test]
fn push_quest_to_party_reads_uint32_quest_id_like_cpp() {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint32(0xA1B2_C3D4);

    let parsed = PushQuestToParty::read(&mut pkt).expect("valid PushQuestToParty packet");

    assert_eq!(parsed.quest_id, 0xA1B2_C3D4);
    assert_eq!(PushQuestToParty::OPCODE, ClientOpcodes::PushQuestToParty);
}

#[test]
fn push_quest_to_party_short_packet_fails_closed() {
    let mut pkt = WorldPacket::from_bytes(&[0x9F, 0x34, 0x00]);

    assert!(PushQuestToParty::read(&mut pkt).is_err());
}
