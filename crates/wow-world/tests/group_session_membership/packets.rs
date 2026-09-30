//! Original receiver sequence and destroyed PartyUpdate wire decoders.

use super::*;

pub(super) fn drain_server_packet_bytes(send_rx: &flume::Receiver<Vec<u8>>) -> Vec<Vec<u8>> {
    let mut packets = Vec::new();
    while let Ok(bytes) = send_rx.try_recv() {
        packets.push(bytes);
    }
    packets
}

pub(super) fn party_update_sequence_num_like_cpp(bytes: &[u8]) -> i32 {
    let mut pkt = WorldPacket::from_bytes(bytes);
    assert_eq!(
        pkt.read_uint16().unwrap(),
        ServerOpcodes::PartyUpdate as u16
    );
    let _party_flags = pkt.read_uint16().unwrap();
    let _party_index = pkt.read_uint8().unwrap();
    let _party_type = pkt.read_uint8().unwrap();
    let _my_index = pkt.read_int32().unwrap();
    let _party_guid = pkt.read_packed_guid().unwrap();
    pkt.read_int32().unwrap()
}

/// Parses the destroyed `PartyUpdate` that C++
/// `Group::SendUpdateDestroyGroupToPlayer` (`Group.cpp:917-926`) sends so
/// the removed member's client tears down its party frames.
pub(super) fn assert_destroyed_party_update_like_cpp(bytes: &[u8], group_guid: u64) {
    let mut packet = WorldPacket::from_bytes(bytes);
    assert_eq!(
        packet.read_uint16().expect("opcode"),
        ServerOpcodes::PartyUpdate as u16
    );
    assert_eq!(
        packet.read_uint16().expect("party flags"),
        wow_social::group::GROUP_FLAG_DESTROYED_LIKE_CPP
    );
    assert_eq!(
        packet.read_uint8().expect("party index"),
        wow_social::group::GROUP_CATEGORY_HOME_LIKE_CPP
    );
    assert_eq!(
        packet.read_uint8().expect("party type"),
        wow_social::group::GROUP_TYPE_NONE_LIKE_CPP
    );
    assert_eq!(packet.read_int32().expect("my index"), -1);
    assert_eq!(
        packet.read_packed_guid().expect("party guid"),
        ObjectGuid::create_group(group_guid)
    );
    let _sequence_num = packet.read_int32().expect("sequence num");
    assert_eq!(
        packet.read_packed_guid().expect("leader guid"),
        ObjectGuid::EMPTY
    );
}
