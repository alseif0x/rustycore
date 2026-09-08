// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! C++-ordered serialization for the shared spell cast payload.

use wow_core::ObjectGuid;

use crate::world_packet::WorldPacket;

use super::{SpellCastData, SpellCastVisual, SpellMissTarget, SpellTargetData};

const MAX_16_BIT_COUNT: usize = u16::MAX as usize;
const MAX_9_BIT_COUNT: usize = 0x1ff;
const MAX_U32_COUNT: usize = u32::MAX as usize;

fn assert_count(field: &str, count: usize, max: usize) {
    assert!(
        count <= max,
        "SpellCastData {field} count {count} exceeds wire maximum {max}"
    );
}

/// Write the 3.4.3 `WorldPackets::Spells::SpellCastData` serializer.
///
/// C++ anchors: `SpellPackets.h::SpellCastData` and
/// `SpellPackets.cpp::operator<<(ByteBuffer&, SpellCastData const&)`.
///
/// `SpellStartPkt` and `SpellGoPkt` keep their common header and hit/miss
/// vectors as packet fields. `cast_data` supplies the formerly hardcoded
/// optional fields and is written in the exact C++ order around those fields.
pub(super) fn write_spell_cast_data(
    pkt: &mut WorldPacket,
    caster: &ObjectGuid,
    cast_id: &ObjectGuid,
    original_cast_id: &ObjectGuid,
    spell_id: i32,
    visual: &SpellCastVisual,
    cast_flags: u32,
    cast_flags_ex: u32,
    cast_time_ms: u32,
    target: &SpellTargetData,
    cast_data: &SpellCastData,
    hit_targets: &[ObjectGuid],
    miss_targets: &[SpellMissTarget],
) {
    // Validate every bounded count before writing any bytes. ServerPacket has
    // a non-recoverable `write` contract, so an oversized value fails loudly
    // instead of silently truncating a bit-packed count.
    assert_count("HitTargets", hit_targets.len(), MAX_16_BIT_COUNT);
    assert_count("MissTargets", miss_targets.len(), MAX_16_BIT_COUNT);
    assert_count("MissStatus", miss_targets.len(), MAX_16_BIT_COUNT);
    assert_count(
        "RemainingPower",
        cast_data.remaining_power.len(),
        MAX_9_BIT_COUNT,
    );
    assert_count(
        "TargetPoints",
        cast_data.target_points.len(),
        MAX_16_BIT_COUNT,
    );
    if let Some(runes) = &cast_data.remaining_runes {
        assert_count("RuneCooldowns", runes.cooldowns.len(), MAX_U32_COUNT);
    }

    // CasterGUID, CasterUnit, CastID, OriginalCastID.
    pkt.write_packed_guid(caster);
    // The previous Rust adapter represented unit casts by repeating `caster`.
    // Keep that default wire contract while allowing item/source casters to
    // provide their actual unit caster explicitly.
    pkt.write_packed_guid(cast_data.caster_unit.as_ref().unwrap_or(caster));
    pkt.write_packed_guid(cast_id);
    pkt.write_packed_guid(original_cast_id);

    // SpellID, visual, cast flags, cast flags extension, and cast time.
    pkt.write_int32(spell_id);
    visual.write(pkt);
    pkt.write_uint32(cast_flags);
    pkt.write_uint32(cast_flags_ex);
    pkt.write_uint32(cast_time_ms);

    // MissileTrajectory, destination cast index, immunities, and heal
    // prediction precede the C++ bit-packed counts.
    pkt.write_uint32(cast_data.missile_trajectory.travel_time);
    pkt.write_float(cast_data.missile_trajectory.pitch);
    pkt.write_uint8(cast_data.dest_loc_spell_cast_index);
    pkt.write_uint32(cast_data.immunities.school);
    pkt.write_uint32(cast_data.immunities.value);
    pkt.write_uint32(cast_data.heal_prediction.points);
    pkt.write_uint8(cast_data.heal_prediction.prediction_type);
    pkt.write_packed_guid(&cast_data.heal_prediction.beacon_guid);

    // C++ `SpellCastData::operator<<` bit order.
    pkt.write_bits(hit_targets.len() as u32, 16);
    pkt.write_bits(miss_targets.len() as u32, 16);
    pkt.write_bits(miss_targets.len() as u32, 16);
    pkt.write_bits(cast_data.remaining_power.len() as u32, 9);
    pkt.write_bit(cast_data.remaining_runes.is_some());
    pkt.write_bits(cast_data.target_points.len() as u32, 16);
    pkt.write_bit(cast_data.ammo_display_id.is_some());
    pkt.write_bit(cast_data.ammo_inventory_type.is_some());
    pkt.flush_bits();

    // Target, then each vector in the same order as C++.
    target.write(pkt);

    for guid in hit_targets {
        pkt.write_packed_guid(guid);
    }
    for miss in miss_targets {
        pkt.write_packed_guid(&miss.target);
    }
    for miss in miss_targets {
        miss.status.write(pkt);
    }
    for power in &cast_data.remaining_power {
        pkt.write_int32(power.amount);
        pkt.write_int8(power.power_type);
    }
    if let Some(runes) = &cast_data.remaining_runes {
        pkt.write_uint8(runes.start);
        pkt.write_uint8(runes.count);
        pkt.write_uint32(runes.cooldowns.len() as u32);
        pkt.write_bytes(&runes.cooldowns);
    }
    for target_point in &cast_data.target_points {
        target_point.write(pkt);
    }
    if let Some(ammo_display_id) = cast_data.ammo_display_id {
        pkt.write_int32(ammo_display_id);
    }
    if let Some(ammo_inventory_type) = cast_data.ammo_inventory_type {
        pkt.write_int32(ammo_inventory_type);
    }
}
