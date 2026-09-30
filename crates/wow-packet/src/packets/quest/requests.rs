//! Quest request wire readers. Target QuestPackets.cpp:273-280, 547-560 at a5f8da2e.
//! Preserve the existing Rust defaults for truncated ID/boolean fields.

use wow_core::ObjectGuid;
use crate::{PacketError, WorldPacket};

pub fn read_quest_giver_query_quest(
    pkt: &mut WorldPacket,
) -> Result<(ObjectGuid, u32, bool), PacketError> {
    let guid = pkt.read_packed_guid()?;
    let quest_id = pkt.read_uint32().unwrap_or(0);
    let respond_to_giver = pkt.read_bit().unwrap_or(false);
    Ok((guid, quest_id, respond_to_giver))
}

pub fn read_quest_giver_accept_quest(
    pkt: &mut WorldPacket,
) -> Result<(ObjectGuid, u32, bool), PacketError> {
    let guid = pkt.read_packed_guid()?;
    let quest_id = pkt.read_uint32().unwrap_or(0);
    let start_cheat = pkt.read_bit().unwrap_or(false);
    Ok((guid, quest_id, start_cheat))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QuestChoiceItem {
    pub loot_item_type: u8,
    pub item_id: u32,
    pub quantity: i32,
}

pub fn read_quest_choice_item(
    pkt: &mut WorldPacket,
) -> Result<QuestChoiceItem, PacketError> {
    // C++ `QuestChoiceItem` starts with `ResetBitPos(); ReadBits(2)`, then
    // an `Item::ItemInstance`, then signed `Quantity`.
    pkt.reset_bits();
    let loot_item_type = pkt.read_bits(2)? as u8;

    let item_id = pkt.read_int32()? as u32;
    let _random_properties_seed = pkt.read_int32()?;
    let _random_properties_id = pkt.read_int32()?;

    let has_item_bonus = pkt.read_bit()?;
    pkt.reset_bits();

    let item_mod_count = pkt.read_bits(6)?;
    pkt.reset_bits();
    for _ in 0..item_mod_count {
        let _value = pkt.read_int32()?;
        let _modifier_type = pkt.read_uint8()?;
    }

    if has_item_bonus {
        let _context = pkt.read_uint8()?;
        let bonus_count = pkt.read_uint32()?;
        for _ in 0..bonus_count {
            let _bonus_id = pkt.read_uint32()?;
        }
    }

    let quantity = pkt.read_int32()?;

    Ok(QuestChoiceItem {
        loot_item_type,
        item_id,
        quantity,
    })
}

#[cfg(test)]
mod tests;
