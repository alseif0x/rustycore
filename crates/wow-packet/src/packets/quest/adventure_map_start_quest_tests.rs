//! Adventure map start quest tests for the quest module.
//!
//! Separated from quest.rs under #687.

use super::*;

#[test]
fn adventure_map_start_quest_reads_signed_quest_id_like_cpp() {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_int32(-123);
    pkt.reset_read();

    let parsed =
        AdventureMapStartQuest::read(&mut pkt).expect("valid AdventureMapStartQuest packet");

    assert_eq!(parsed.quest_id, -123);
    assert_eq!(
        AdventureMapStartQuest::OPCODE,
        ClientOpcodes::AdventureMapStartQuest
    );
}

#[test]
fn adventure_map_start_quest_short_packet_fails_closed() {
    let mut pkt = WorldPacket::from_bytes(&[0x01, 0x02, 0x03]);

    assert!(AdventureMapStartQuest::read(&mut pkt).is_err());
}
