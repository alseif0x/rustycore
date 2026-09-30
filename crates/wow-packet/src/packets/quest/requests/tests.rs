use super::*;
use super::{
    read_quest_giver_accept_quest as read_quest_giver_accept_quest_like_cpp,
    read_quest_giver_query_quest as read_quest_giver_query_quest_like_cpp,
};
use crate::{PacketError, WorldPacket};
use wow_constants::quest::{
    QUEST_CHOICE_LOOT_ITEM_TYPE_CURRENCY as QUEST_CHOICE_LOOT_ITEM_TYPE_CURRENCY_LIKE_CPP,
    QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM as QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP,
};

#[test]
fn quest_giver_query_quest_reads_respond_to_giver_as_bit_like_cpp() {
    let guid =
        ObjectGuid::create_world_object(wow_core::guid::HighGuid::Creature, 0, 1, 571, 0, 123, 456);

    for (bit_byte, expected) in [(0x80, true), (0x00, false), (0x01, false)] {
        let mut packet = quest_giver_cmsg_packet(guid, 7001, bit_byte);
        let (parsed_guid, quest_id, respond_to_giver) =
            read_quest_giver_query_quest_like_cpp(&mut packet).unwrap();

        assert_eq!(parsed_guid, guid);
        assert_eq!(quest_id, 7001);
        assert_eq!(
            respond_to_giver, expected,
            "C++ ReadBit reads the high bit; byte {bit_byte:#04x} must not be treated as bool"
        );
    }
}

#[test]
fn quest_giver_accept_quest_reads_start_cheat_as_bit_like_cpp() {
    let guid =
        ObjectGuid::create_world_object(wow_core::guid::HighGuid::Creature, 0, 1, 571, 0, 123, 456);

    for (bit_byte, expected) in [(0x80, true), (0x00, false), (0x01, false)] {
        let mut packet = quest_giver_cmsg_packet(guid, 7002, bit_byte);
        let (parsed_guid, quest_id, start_cheat) =
            read_quest_giver_accept_quest_like_cpp(&mut packet).unwrap();

        assert_eq!(parsed_guid, guid);
        assert_eq!(quest_id, 7002);
        assert_eq!(
            start_cheat, expected,
            "C++ ReadBit reads the high bit; byte {bit_byte:#04x} must not be treated as bool"
        );
    }
}

#[test]
fn quest_giver_choose_reward_choice_parser_rejects_truncated_cpp_wire() {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_bits(u32::from(QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP), 2);
    write_cpp_item_instance_like_cpp(&mut pkt, 19019, 0, 0, &[], None);

    assert!(read_quest_choice_item_for_test(&mut pkt).is_err());
}

#[test]
fn quest_giver_choose_reward_choice_parser_reads_cpp_wire_item_choice() {
    let guid = ObjectGuid::create_player(1, 42);
    let mut pkt = WorldPacket::new_empty();
    pkt.write_packed_guid(&guid);
    pkt.write_uint32(7001);
    write_cpp_quest_choice_item_like_cpp(
        &mut pkt,
        QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP,
        19019,
        3,
    );

    assert_eq!(pkt.read_packed_guid().unwrap(), guid);
    assert_eq!(pkt.read_uint32().unwrap(), 7001);
    assert_eq!(
        read_quest_choice_item_for_test(&mut pkt).unwrap(),
        (QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP, 19019, 3)
    );
    assert!(pkt.is_empty());
}

#[test]
fn quest_giver_choose_reward_choice_parser_skips_cpp_item_mods_and_bonus() {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_bits(u32::from(QUEST_CHOICE_LOOT_ITEM_TYPE_CURRENCY_LIKE_CPP), 2);
    write_cpp_item_instance_like_cpp(&mut pkt, 392, 11, 22, &[(7, 1), (8, 2)], Some(&[91, 92]));
    pkt.write_int32(5);

    let choice = read_quest_choice_item_for_test(&mut pkt).unwrap();

    assert_eq!(
        choice,
        (QUEST_CHOICE_LOOT_ITEM_TYPE_CURRENCY_LIKE_CPP, 392, 5)
    );
    assert!(pkt.is_empty());
}

fn quest_giver_cmsg_packet(guid: ObjectGuid, quest_id: u32, bit_byte: u8) -> crate::WorldPacket {
    let mut packet = crate::WorldPacket::new_empty();
    packet.write_packed_guid(&guid);
    packet.write_uint32(quest_id);
    packet.write_uint8(bit_byte);
    packet.reset_read();
    packet
}

fn write_cpp_item_instance_like_cpp(
    pkt: &mut WorldPacket,
    item_id: i32,
    random_properties_seed: i32,
    random_properties_id: i32,
    item_mods: &[(i32, u8)],
    item_bonus_ids: Option<&[u32]>,
) {
    pkt.write_int32(item_id);
    pkt.write_int32(random_properties_seed);
    pkt.write_int32(random_properties_id);
    pkt.write_bit(item_bonus_ids.is_some());
    pkt.flush_bits();
    pkt.write_bits(item_mods.len() as u32, 6);
    pkt.flush_bits();
    for (value, modifier_type) in item_mods {
        pkt.write_int32(*value);
        pkt.write_uint8(*modifier_type);
    }
    if let Some(item_bonus_ids) = item_bonus_ids {
        pkt.write_uint8(0);
        pkt.write_uint32(item_bonus_ids.len() as u32);
        for bonus_id in item_bonus_ids {
            pkt.write_uint32(*bonus_id);
        }
    }
}

fn write_cpp_quest_choice_item_like_cpp(
    pkt: &mut WorldPacket,
    loot_item_type: u8,
    item_id: i32,
    quantity: i32,
) {
    pkt.reset_bits();
    pkt.write_bits(u32::from(loot_item_type), 2);
    write_cpp_item_instance_like_cpp(pkt, item_id, 0, 0, &[], None);
    pkt.write_int32(quantity);
}

fn read_quest_choice_item_for_test(pkt: &mut WorldPacket) -> Result<(u8, u32, i32), PacketError> {
    read_quest_choice_item(pkt)
        .map(|choice| (choice.loot_item_type, choice.item_id, choice.quantity))
}

#[test]
fn quest_request_readers_preserve_guid_errors_and_missing_scalar_defaults() {
    let guid = ObjectGuid::create_player(1, 42);
    for read in [
        read_quest_giver_query_quest
            as fn(&mut WorldPacket) -> Result<(ObjectGuid, u32, bool), PacketError>,
        read_quest_giver_accept_quest,
    ] {
        let mut missing_guid = WorldPacket::from_bytes(&[]);
        assert!(read(&mut missing_guid).is_err());
        let mut missing_scalars = WorldPacket::new_empty();
        missing_scalars.write_packed_guid(&guid);
        missing_scalars.reset_read();
        assert_eq!(read(&mut missing_scalars).unwrap(), (guid, 0, false));
        let mut missing_bit = WorldPacket::new_empty();
        missing_bit.write_packed_guid(&guid);
        missing_bit.write_uint32(7001);
        missing_bit.reset_read();
        assert_eq!(read(&mut missing_bit).unwrap(), (guid, 7001, false));
    }
}
