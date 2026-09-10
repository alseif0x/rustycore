//! Quest confirm accept tests for the quest module.
//!
//! Separated from quest.rs under #687.

use super::*;

#[test]
fn quest_confirm_accept_reads_signed_quest_id_like_cpp() {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_int32(7001);

    let parsed = QuestConfirmAccept::read(&mut pkt).expect("valid QuestConfirmAccept packet");

    assert_eq!(parsed.quest_id, 7001);
}

#[test]
fn quest_confirm_accept_short_packet_fails_closed() {
    let mut pkt = WorldPacket::from_bytes(&[0x59, 0x1B, 0x00]);

    assert!(QuestConfirmAccept::read(&mut pkt).is_err());
}
