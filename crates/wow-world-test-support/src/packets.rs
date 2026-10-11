//! Session-free server packet decoders and expectations shared by the session scenarios.
//!
//! Moved from `wow-world` `session::tests::fixtures::packets` by #1363 slice 0.

use wow_constants::ServerOpcodes;
use wow_core::ObjectGuid;
use wow_packet::ServerPacket;
use wow_packet::WorldPacket;
use wow_world_core::session::set_active_player_update_bit_like_cpp;

pub fn drain_server_opcodes(send_rx: &flume::Receiver<Vec<u8>>) -> Vec<ServerOpcodes> {
    let mut opcodes = Vec::new();
    while let Ok(bytes) = send_rx.try_recv() {
        if let Some(opcode) = wow_packet::WorldPacket::from_bytes(&bytes).server_opcode() {
            opcodes.push(opcode);
        }
    }
    opcodes
}

pub fn drain_server_packet_bytes(send_rx: &flume::Receiver<Vec<u8>>) -> Vec<Vec<u8>> {
    let mut packets = Vec::new();
    while let Ok(bytes) = send_rx.try_recv() {
        packets.push(bytes);
    }
    packets
}

pub fn party_update_sequence_num_like_cpp(bytes: &[u8]) -> i32 {
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

pub fn packet_contains_quest_ids_in_order(bytes: &[u8], quest_ids: &[u32]) -> bool {
    let mut search_from = 0usize;
    for quest_id in quest_ids {
        let needle = quest_id.to_le_bytes();
        let Some(relative_pos) = bytes[search_from..]
            .windows(needle.len())
            .position(|window| window == needle)
        else {
            return false;
        };
        search_from += relative_pos + needle.len();
    }
    true
}

#[derive(Debug, PartialEq, Eq)]
pub struct QuestGiverRequestItemsSummaryLikeCpp {
    pub giver_creature_id: i32,
    pub quest_id: u32,
    pub status_flags: u32,
    pub auto_launched: bool,
}

pub fn quest_giver_request_items_summary_like_cpp(
    bytes: &[u8],
) -> QuestGiverRequestItemsSummaryLikeCpp {
    let mut pkt = wow_packet::WorldPacket::from_bytes(&bytes[2..]);
    pkt.read_packed_guid().unwrap();
    let giver_creature_id = pkt.read_int32().unwrap();
    let quest_id = pkt.read_int32().unwrap() as u32;
    pkt.read_int32().unwrap(); // CompEmoteDelay
    pkt.read_int32().unwrap(); // CompEmoteType
    pkt.read_uint32().unwrap(); // QuestFlags[0]
    pkt.read_uint32().unwrap(); // QuestFlags[1]
    pkt.read_uint32().unwrap(); // QuestFlags[2]
    pkt.read_int32().unwrap(); // SuggestedPartyMembers
    pkt.read_int32().unwrap(); // MoneyToGet
    let collect_count = pkt.read_int32().unwrap().max(0) as usize;
    let currency_count = pkt.read_int32().unwrap().max(0) as usize;
    let status_flags = pkt.read_int32().unwrap() as u32;

    for _ in 0..collect_count {
        pkt.read_int32().unwrap(); // ObjectID
        pkt.read_int32().unwrap(); // Amount
        pkt.read_uint32().unwrap(); // Flags
    }
    for _ in 0..currency_count {
        pkt.read_int32().unwrap(); // CurrencyID
        pkt.read_int32().unwrap(); // Amount
    }

    QuestGiverRequestItemsSummaryLikeCpp {
        giver_creature_id,
        quest_id,
        status_flags,
        auto_launched: pkt.read_bit().unwrap(),
    }
}

pub fn quest_list_level_fields_like_cpp(bytes: &[u8]) -> Vec<(u32, i32, i32)> {
    let mut pkt = wow_packet::WorldPacket::from_bytes(&bytes[2..]);
    pkt.read_packed_guid().unwrap();
    pkt.read_uint32().unwrap();
    pkt.read_uint32().unwrap();
    let quest_count = pkt.read_uint32().unwrap();
    let greeting_len = pkt.read_bits(11).unwrap();

    let mut levels = Vec::new();
    for _ in 0..quest_count {
        let quest_id = pkt.read_uint32().unwrap();
        let _content_tuning_id = pkt.read_uint32().unwrap();
        let _quest_type = pkt.read_int32().unwrap();
        let quest_level = pkt.read_int32().unwrap();
        let quest_max_scaling_level = pkt.read_int32().unwrap();
        let _quest_flags = pkt.read_uint32().unwrap();
        let _quest_flags_ex = pkt.read_uint32().unwrap();
        let _repeatable = pkt.read_bit().unwrap();
        let _important = pkt.read_bit().unwrap();
        let title_len = pkt.read_bits(9).unwrap();
        let _title = pkt.read_string(title_len as usize).unwrap();
        levels.push((quest_id, quest_level, quest_max_scaling_level));
    }

    let _greeting = pkt.read_string(greeting_len as usize).unwrap();
    levels
}

pub fn expected_active_player_farsight_object_values_update_like_cpp(
    player_guid: ObjectGuid,
    map_id: u16,
    farsight_guid: ObjectGuid,
) -> Vec<u8> {
    use wow_packet::packets::update::{ActivePlayerDataValuesUpdate, UpdateObject};

    let mut data = ActivePlayerDataValuesUpdate::default();
    set_active_player_update_bit_like_cpp(&mut data.active_player_data_mask, 0);
    set_active_player_update_bit_like_cpp(&mut data.active_player_data_mask, 26);
    data.farsight_object = farsight_guid;
    UpdateObject::full_active_player_values_update(player_guid, map_id, data).to_bytes()
}

pub fn expected_dynamic_object_create_packet_like_cpp(
    map_id: u16,
    create_data: wow_packet::packets::update::DynamicObjectCreateData,
) -> Vec<u8> {
    use wow_packet::packets::update::UpdateObject;

    UpdateObject::create_world_objects(
        vec![UpdateObject::create_dynamic_object_block(create_data)],
        map_id,
    )
    .to_bytes()
}

pub fn update_object_packet_count_like_cpp(packets: &[Vec<u8>]) -> usize {
    packets
        .iter()
        .filter(|bytes| {
            wow_packet::WorldPacket::from_bytes(bytes).server_opcode()
                == Some(ServerOpcodes::UpdateObject)
        })
        .count()
}
