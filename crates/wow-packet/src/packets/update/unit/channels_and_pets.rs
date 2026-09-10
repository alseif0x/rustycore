//! Channels and pets packets.
//!
//! Separated from unit.rs under #689.

use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArtifactPowerValuesUpdate {
    pub artifact_power_id: i16,
    pub purchased_rank: u8,
    pub current_rank_with_bonus: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct UnitChannelValuesUpdate {
    pub spell_id: i32,
    pub spell_visual_id: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StablePetInfoValuesUpdate {
    pub stable_pet_mask: u8,
    pub pet_slot: u32,
    pub pet_number: u32,
    pub creature_id: u32,
    pub display_id: u32,
    pub experience_level: u32,
    pub name: String,
    pub pet_flags: u8,
}

pub(in crate::packets::update) fn write_artifact_power_values_update(
    buf: &mut WorldPacket,
    data: &ArtifactPowerValuesUpdate,
) {
    buf.write_int16(data.artifact_power_id);
    buf.write_uint8(data.purchased_rank);
    buf.write_uint8(data.current_rank_with_bonus);
}

pub(super) fn write_unit_channel_values_update(
    buf: &mut WorldPacket,
    data: &UnitChannelValuesUpdate,
) {
    buf.write_int32(data.spell_id);
    buf.write_int32(data.spell_visual_id);
}

pub fn write_stable_pet_info_values_update(
    buf: &mut WorldPacket,
    data: &StablePetInfoValuesUpdate,
) {
    let mask = data.stable_pet_mask;
    buf.write_bits(mask as u32, 8);

    buf.flush_bits();
    if mask & 0x01 != 0 {
        if mask & 0x02 != 0 {
            buf.write_uint32(data.pet_slot);
        }
        if mask & 0x04 != 0 {
            buf.write_uint32(data.pet_number);
        }
        if mask & 0x08 != 0 {
            buf.write_uint32(data.creature_id);
        }
        if mask & 0x10 != 0 {
            buf.write_uint32(data.display_id);
        }
        if mask & 0x20 != 0 {
            buf.write_uint32(data.experience_level);
        }
        if mask & 0x80 != 0 {
            buf.write_uint8(data.pet_flags);
        }
        if mask & 0x40 != 0 {
            buf.write_bits(data.name.len() as u32, 8);
            buf.write_string(&data.name);
        }
    }
    buf.flush_bits();
}
