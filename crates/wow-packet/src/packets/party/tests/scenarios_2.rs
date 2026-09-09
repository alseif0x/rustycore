//! Party packets regression scenarios, part 2 of 2.
//!
//! Moved out of the party.rs root under #650; every test is unchanged.

use super::*;

#[test]
fn minimap_ping_server_opcode_and_payload_order_like_cpp() {
    use super::MinimapPing;
    let sender = ObjectGuid::create_player(1, 42);
    let pkt = MinimapPing {
        sender,
        position_x: 111.222,
        position_y: 333.444,
    }
    .to_bytes();

    assert!(!pkt.is_empty());
    let mut reader = WorldPacket::from_bytes(&pkt);
    assert_eq!(
        reader.read_uint16().unwrap(),
        ServerOpcodes::MinimapPing as u16
    );
    let read_guid = reader.read_packed_guid().unwrap();
    assert_eq!(read_guid, sender);
    assert_eq!(reader.read_float().unwrap(), 111.222);
    assert_eq!(reader.read_float().unwrap(), 333.444);
}

#[test]
fn silence_party_talker_reads_guid_and_silent_bit_like_cpp() {
    let target = ObjectGuid::create_player(1, 0x55aa);
    let mut pkt = WorldPacket::new_empty();
    pkt.write_bytes(&target.to_raw_bytes());
    pkt.write_bit(true);
    pkt.flush_bits();
    pkt.reset_read();

    let parsed = SilencePartyTalker::read(&mut pkt).unwrap();

    assert_eq!(parsed.target, target);
    assert!(parsed.silent);
}

#[test]
fn silence_party_talker_opcode_matches_cpp() {
    assert_eq!(SilencePartyTalker::OPCODE as u16, 0x3655);
}
