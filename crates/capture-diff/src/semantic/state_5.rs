//! Semantic capture diff state definitions, part 5 of 5.
//!
//! Separated from the semantic.rs root under #658. Behaviour is preserved.

use super::*;

pub(super) fn decode_spell_cast_data_body(
    body: &[u8],
    has_spell_go_combat_log_suffix: bool,
) -> Result<DecodedSpellGoBody, String> {
    let mut cursor = 0usize;
    let exact_caster_guid = read_exact_guid(body, &mut cursor, "CasterGUID")?;
    let exact_caster_unit = read_exact_guid(body, &mut cursor, "CasterUnit")?;
    let cast_id = read_exact_guid(body, &mut cursor, "CastID")?;
    let original_cast_id = read_exact_guid(body, &mut cursor, "OriginalCastID")?;
    let spell_id = read_i32(body, &mut cursor, "SpellID")?;
    let spell_visual_id = read_i32(body, &mut cursor, "Visual.SpellXSpellVisualID")?;
    let cast_flags = read_u32(body, &mut cursor, "CastFlags")?;
    let cast_flags_ex = read_u32(body, &mut cursor, "CastFlagsEx")?;
    let cast_time = read_u32(body, &mut cursor, "CastTime")?;
    let missile_travel_time = read_u32(body, &mut cursor, "MissileTrajectory.TravelTime")?;
    let missile_pitch_bits = read_f32_bits(body, &mut cursor, "MissileTrajectory.Pitch")?;
    let dest_loc_spell_cast_index = read_u8(body, &mut cursor, "DestLocSpellCastIndex")?;
    let immunities_school = read_u32(body, &mut cursor, "Immunities.School")?;
    let immunities_value = read_u32(body, &mut cursor, "Immunities.Value")?;
    let prediction_points = read_u32(body, &mut cursor, "Predict.Points")?;
    let prediction_type = read_u8(body, &mut cursor, "Predict.Type")?;
    let prediction_beacon = read_exact_guid(body, &mut cursor, "Predict.BeaconGUID")?;

    let mut counts = MsbBitReader::new(body, cursor, "SpellCastData counts");
    let hit_count = counts.read(16, "HitTargets")? as usize;
    let miss_count = counts.read(16, "MissTargets")? as usize;
    let miss_status_count = counts.read(16, "MissStatus")? as usize;
    let remaining_power_count = counts.read(9, "RemainingPower")? as usize;
    let has_remaining_runes = counts.read(1, "RemainingRunes")? != 0;
    let target_point_count = counts.read(16, "TargetPoints")? as usize;
    let has_ammo_display_id = counts.read(1, "AmmoDisplayID")? != 0;
    let has_ammo_inventory_type = counts.read(1, "AmmoInventoryType")? != 0;
    counts.finish(&mut cursor)?;

    if miss_count != miss_status_count {
        return Err(format!(
            "SpellCastData has {miss_count} MissTargets but {miss_status_count} MissStatus entries"
        ));
    }

    let target = read_spell_target_data(body, &mut cursor, exact_caster_guid)?;

    ensure_count_fits_minimum(body, cursor, hit_count, 2, "HitTargets")?;
    let mut hit_targets = Vec::with_capacity(hit_count);
    for index in 0..hit_count {
        let guid = read_exact_guid(body, &mut cursor, &format!("HitTargets[{index}]"))?;
        hit_targets.push(correlate_caster_guid(guid, exact_caster_guid));
    }

    ensure_count_fits_minimum(body, cursor, miss_count, 2, "MissTargets")?;
    let mut miss_targets = Vec::with_capacity(miss_count);
    for index in 0..miss_count {
        let guid = read_exact_guid(body, &mut cursor, &format!("MissTargets[{index}]"))?;
        miss_targets.push(correlate_caster_guid(guid, exact_caster_guid));
    }

    ensure_count_fits_minimum(body, cursor, miss_status_count, 1, "MissStatus")?;
    let mut miss_status = Vec::with_capacity(miss_status_count);
    for index in 0..miss_status_count {
        let reason = read_u8(body, &mut cursor, &format!("MissStatus[{index}].Reason"))?;
        let reflect_status = if reason == SPELL_MISS_REFLECT {
            Some(read_u8(
                body,
                &mut cursor,
                &format!("MissStatus[{index}].ReflectStatus"),
            )?)
        } else {
            None
        };
        miss_status.push(SpellMissStatusBody {
            reason,
            reflect_status,
        });
    }

    ensure_count_fits_minimum(body, cursor, remaining_power_count, 5, "RemainingPower")?;
    let mut remaining_power = Vec::with_capacity(remaining_power_count);
    for index in 0..remaining_power_count {
        remaining_power.push(SpellPowerDataBody {
            cost: read_i32(body, &mut cursor, &format!("RemainingPower[{index}].Cost"))?,
            power_type: read_i8(body, &mut cursor, &format!("RemainingPower[{index}].Type"))?,
        });
    }

    let remaining_runes = if has_remaining_runes {
        let start = read_u8(body, &mut cursor, "RemainingRunes.Start")?;
        let count = read_u8(body, &mut cursor, "RemainingRunes.Count")?;
        let cooldown_count = read_u32(body, &mut cursor, "RemainingRunes.CooldownsCount")? as usize;
        let cooldowns = read_bytes(
            body,
            &mut cursor,
            cooldown_count,
            "RemainingRunes.Cooldowns",
        )?
        .to_vec();
        Some(SpellRuneDataBody {
            start,
            count,
            cooldowns,
        })
    } else {
        None
    };

    ensure_count_fits_minimum(body, cursor, target_point_count, 14, "TargetPoints")?;
    let mut target_points = Vec::with_capacity(target_point_count);
    for index in 0..target_point_count {
        target_points.push(read_spell_target_location(
            body,
            &mut cursor,
            &format!("TargetPoints[{index}]"),
        )?);
    }

    let ammo_display_id = has_ammo_display_id
        .then(|| read_i32(body, &mut cursor, "AmmoDisplayID"))
        .transpose()?;
    let ammo_inventory_type = has_ammo_inventory_type
        .then(|| read_i32(body, &mut cursor, "AmmoInventoryType"))
        .transpose()?;

    if has_spell_go_combat_log_suffix {
        let mut combat_log = MsbBitReader::new(body, cursor, "SpellGo combat-log bit");
        let has_full_combat_log = combat_log.read(1, "HasLogData")? != 0;
        combat_log.finish(&mut cursor)?;
        if has_full_combat_log {
            return Err(
                "SpellGo carries full SpellCastLogData; creature-spell contract permits only the fully decoded basic packet"
                    .to_string(),
            );
        }
    }
    if cursor != body.len() {
        return Err(format!(
            "trailing bytes after {}: decoded {cursor} of {} bytes",
            if has_spell_go_combat_log_suffix {
                "SpellGo combat-log bit"
            } else {
                "SpellStart SpellCastData"
            },
            body.len()
        ));
    }

    Ok(DecodedSpellGoBody {
        body: SpellGoBody {
            caster_guid: stable_object_guid(exact_caster_guid.low, exact_caster_guid.high),
            caster_unit: stable_object_guid(exact_caster_unit.low, exact_caster_unit.high),
            cast_id: stable_object_guid(cast_id.low, cast_id.high),
            original_cast_id,
            spell_id,
            spell_visual_id,
            cast_flags,
            cast_flags_ex,
            missile_travel_time,
            missile_pitch_bits,
            dest_loc_spell_cast_index,
            immunities_school,
            immunities_value,
            prediction_points,
            prediction_type,
            prediction_beacon,
            target,
            hit_targets,
            miss_targets,
            miss_status,
            remaining_power,
            remaining_runes,
            target_points,
            ammo_display_id,
            ammo_inventory_type,
        },
        exact_caster_guid,
        exact_caster_unit,
        cast_id,
        cast_time,
    })
}

pub(super) fn read_spell_target_data(
    body: &[u8],
    cursor: &mut usize,
    caster: ExactObjectGuid,
) -> Result<SpellTargetDataBody, String> {
    let mut bits = MsbBitReader::new(body, *cursor, "SpellTargetData bits");
    let flags = bits.read(28, "Flags")?;
    let has_src_location = bits.read(1, "HasSrcLocation")? != 0;
    let has_dst_location = bits.read(1, "HasDstLocation")? != 0;
    let has_orientation = bits.read(1, "HasOrientation")? != 0;
    let has_map_id = bits.read(1, "HasMapID")? != 0;
    let name_len = bits.read(7, "NameLength")? as usize;
    bits.finish(cursor)?;

    let unit = correlate_caster_guid(read_exact_guid(body, cursor, "Target.Unit")?, caster);
    let item = read_exact_guid(body, cursor, "Target.Item")?;
    let src_location = has_src_location
        .then(|| read_spell_target_location(body, cursor, "Target.SrcLocation"))
        .transpose()?;
    let dst_location = has_dst_location
        .then(|| read_spell_target_location(body, cursor, "Target.DstLocation"))
        .transpose()?;
    let orientation_bits = has_orientation
        .then(|| read_f32_bits(body, cursor, "Target.Orientation"))
        .transpose()?;
    let map_id = has_map_id
        .then(|| read_i32(body, cursor, "Target.MapID"))
        .transpose()?;
    let name = read_bytes(body, cursor, name_len, "Target.Name")?.to_vec();

    Ok(SpellTargetDataBody {
        flags,
        unit,
        item,
        src_location,
        dst_location,
        orientation_bits,
        map_id,
        name,
    })
}

pub(super) fn read_spell_target_location(
    body: &[u8],
    cursor: &mut usize,
    field: &str,
) -> Result<SpellTargetLocationBody, String> {
    Ok(SpellTargetLocationBody {
        transport: read_exact_guid(body, cursor, &format!("{field}.Transport"))?,
        position: read_wire_position(body, cursor, &format!("{field}.Location"))?,
    })
}

pub(super) fn correlate_caster_guid(
    guid: ExactObjectGuid,
    caster: ExactObjectGuid,
) -> CorrelatedSpellGuidBody {
    if guid == caster {
        CorrelatedSpellGuidBody::Caster
    } else {
        CorrelatedSpellGuidBody::Exact { guid }
    }
}

pub(super) fn validate_decoded_creature_spell_go(
    decoded: &DecodedSpellGoBody,
) -> Result<(), String> {
    validate_creature_spell_cast_shape(
        &decoded.body,
        decoded.exact_caster_guid,
        decoded.exact_caster_unit,
        decoded.cast_id,
    )
}

pub(super) fn validate_creature_spell_cast_shape(
    body: &SpellGoBody,
    exact_caster_guid: ExactObjectGuid,
    exact_caster_unit: ExactObjectGuid,
    exact_cast_id: ExactObjectGuid,
) -> Result<(), String> {
    if exact_caster_guid != exact_caster_unit {
        return Err("Creature SpellCast CasterGUID does not equal CasterUnit".to_string());
    }
    if exact_guid_high_type(exact_caster_guid) != HIGH_GUID_CREATURE {
        return Err("Creature SpellCast caster is not a Creature GUID".to_string());
    }
    if exact_caster_guid.low & OBJECT_GUID_COUNTER_MASK == 0 {
        return Err("Creature SpellCast caster has a zero runtime counter".to_string());
    }
    if body.spell_id <= 0 {
        return Err(format!(
            "Creature SpellCast has invalid spell ID {}",
            body.spell_id
        ));
    }
    if body.original_cast_id != (ExactObjectGuid { low: 0, high: 0 }) {
        return Err("Creature AI SpellCast OriginalCastID is not EMPTY".to_string());
    }
    let cast = body.cast_id;
    if cast.high_type != HIGH_GUID_CAST {
        return Err(format!(
            "Creature SpellCast CastID HighGuid is {}, expected Cast ({HIGH_GUID_CAST})",
            cast.high_type
        ));
    }
    if cast.subtype != SPELL_CAST_SOURCE_NORMAL {
        return Err(format!(
            "Creature SpellCast CastID source is {}, expected NORMAL ({SPELL_CAST_SOURCE_NORMAL})",
            cast.subtype
        ));
    }
    if cast.realm_id != body.caster_guid.realm_id
        || cast.map_id != body.caster_guid.map_id
        || cast.entry != body.spell_id as u32
        || cast.server_id != 0
    {
        return Err(format!(
            "Creature SpellCast CastID identity {:?} does not match caster realm/map and spell {} with server 0",
            cast, body.spell_id
        ));
    }
    if exact_cast_id.low & OBJECT_GUID_COUNTER_MASK == 0 {
        return Err("Creature SpellCast CastID has a zero runtime counter".to_string());
    }
    Ok(())
}

pub(super) fn decode_buy_succeeded_body_with_counter(
    body: &[u8],
) -> Result<DecodedBuySucceededBody, String> {
    let mut cursor = 0usize;
    let (vendor_low, vendor_high) = read_packed_guid(body, &mut cursor, "VendorGUID")?;
    let muid = read_u32(body, &mut cursor, "Muid")?;
    let new_quantity = read_i32(body, &mut cursor, "NewQuantity")?;
    let quantity_bought = read_u32(body, &mut cursor, "QuantityBought")?;
    if cursor != body.len() {
        return Err(format!(
            "trailing bytes after QuantityBought: decoded {cursor} of {} bytes",
            body.len()
        ));
    }

    Ok(DecodedBuySucceededBody {
        body: BuySucceededBody {
            vendor: stable_object_guid(vendor_low, vendor_high),
            muid,
            new_quantity,
            quantity_bought,
        },
        vendor_runtime_counter: vendor_low & OBJECT_GUID_COUNTER_MASK,
    })
}

pub(super) fn decode_loot_removed_body_with_counter(
    body: &[u8],
) -> Result<DecodedLootRemovedBody, String> {
    let mut cursor = 0usize;
    let (owner_low, owner_high) = read_packed_guid(body, &mut cursor, "Owner")?;
    let (loot_obj_low, loot_obj_high) = read_packed_guid(body, &mut cursor, "LootObj")?;
    let loot_list_id = read_u8(body, &mut cursor, "LootListID")?;
    if cursor != body.len() {
        return Err(format!(
            "trailing bytes after LootListID: decoded {cursor} of {} bytes",
            body.len()
        ));
    }

    Ok(DecodedLootRemovedBody {
        body: LootRemovedBody {
            owner: stable_object_guid(owner_low, owner_high),
            loot_obj: ExactObjectGuid {
                low: loot_obj_low,
                high: loot_obj_high,
            },
            loot_list_id,
        },
        owner_runtime_counter: owner_low & OBJECT_GUID_COUNTER_MASK,
    })
}

pub(super) fn decode_log_xp_gain_body_with_counter(
    body: &[u8],
) -> Result<DecodedLogXpGainBody, String> {
    let mut cursor = 0usize;
    let low_mask = read_u8(body, &mut cursor, "Victim low mask")?;
    let high_mask = read_u8(body, &mut cursor, "Victim high mask")?;
    let low = read_packed_u64(body, &mut cursor, low_mask, "Victim low word")?;
    let high = read_packed_u64(body, &mut cursor, high_mask, "Victim high word")?;

    let original = read_i32(body, &mut cursor, "Original")?;
    let reason = read_u8(body, &mut cursor, "Reason")?;
    let amount = read_i32(body, &mut cursor, "Amount")?;
    let group_bonus_bits = read_u32(body, &mut cursor, "GroupBonus")?;
    if cursor != body.len() {
        return Err(format!(
            "trailing bytes after GroupBonus: decoded {cursor} of {} bytes",
            body.len()
        ));
    }

    // Counter is the only intentionally normalized value. Retain the upper
    // 24 server-id bits from the low word and every bit of the high word.
    Ok(DecodedLogXpGainBody {
        body: LogXpGainBody {
            victim: stable_object_guid(low, high),
            original,
            reason,
            amount,
            group_bonus_bits,
        },
        runtime_counter: low & OBJECT_GUID_COUNTER_MASK,
    })
}

pub(super) fn stable_object_guid(low: u64, high: u64) -> StableObjectGuid {
    let stable_low = low & !OBJECT_GUID_COUNTER_MASK;
    StableObjectGuid {
        high_type: ((high >> 58) & 0x3F) as u8,
        // Keeping all 16 bits between map and high type retains the three
        // reserved bits as part of strict identity for non-canonical inputs.
        realm_id: ((high >> 42) & 0xFFFF) as u16,
        map_id: ((high >> 29) & 0x1FFF) as u16,
        entry: ((high >> 6) & 0x7F_FFFF) as u32,
        subtype: (high & 0x3F) as u8,
        server_id: ((stable_low >> 40) & 0xFF_FFFF) as u32,
    }
}

pub(super) fn exact_guid_high_type(guid: ExactObjectGuid) -> u8 {
    ((guid.high >> 58) & 0x3F) as u8
}

pub(super) fn read_exact_guid(
    body: &[u8],
    cursor: &mut usize,
    field: &str,
) -> Result<ExactObjectGuid, String> {
    let (low, high) = read_packed_guid(body, cursor, field)?;
    Ok(ExactObjectGuid { low, high })
}

pub(super) fn ensure_count_fits_minimum(
    body: &[u8],
    cursor: usize,
    count: usize,
    minimum_width: usize,
    field: &str,
) -> Result<(), String> {
    let minimum = count
        .checked_mul(minimum_width)
        .ok_or_else(|| format!("{field} minimum byte length overflows usize"))?;
    let remaining = body.len().saturating_sub(cursor);
    if minimum > remaining {
        return Err(format!(
            "{field} count {count} needs at least {minimum} bytes but only {remaining} remain"
        ));
    }
    Ok(())
}

pub(super) fn read_bytes<'a>(
    body: &'a [u8],
    cursor: &mut usize,
    len: usize,
    field: &str,
) -> Result<&'a [u8], String> {
    let end = cursor
        .checked_add(len)
        .ok_or_else(|| format!("offset overflow while reading {field}"))?;
    let bytes = body
        .get(*cursor..end)
        .ok_or_else(|| format!("truncated while reading {field} at byte {cursor}"))?;
    *cursor = end;
    Ok(bytes)
}

/// MSB-first bit reader matching TrinityCore `ByteBuffer::ReadBits`.
///
/// Each SpellCastData bit section is explicitly flushed before byte fields.
/// `finish` therefore rejects nonzero low padding bits rather than silently
/// skipping them.
pub(super) struct MsbBitReader<'a> {
    pub(super) body: &'a [u8],
    pub(super) start: usize,
    pub(super) bit_offset: usize,
    pub(super) section: &'static str,
}

impl<'a> MsbBitReader<'a> {
    pub(super) fn new(body: &'a [u8], start: usize, section: &'static str) -> Self {
        Self {
            body,
            start,
            bit_offset: 0,
            section,
        }
    }

    pub(super) fn read(&mut self, width: usize, field: &str) -> Result<u32, String> {
        if width > 32 {
            return Err(format!(
                "{} field {field} requests unsupported {width}-bit width",
                self.section
            ));
        }
        let end_bit = self
            .bit_offset
            .checked_add(width)
            .ok_or_else(|| format!("{} bit offset overflow", self.section))?;
        let available_bits = self.body.len().saturating_sub(self.start).saturating_mul(8);
        if end_bit > available_bits {
            return Err(format!(
                "truncated while reading {}.{field} at bit {}",
                self.section, self.bit_offset
            ));
        }

        let mut value = 0u32;
        while self.bit_offset < end_bit {
            let byte = self.body[self.start + self.bit_offset / 8];
            let shift = 7 - (self.bit_offset % 8);
            value = (value << 1) | u32::from((byte >> shift) & 1);
            self.bit_offset += 1;
        }
        Ok(value)
    }

    pub(super) fn finish(self, cursor: &mut usize) -> Result<(), String> {
        let used_bytes = self.bit_offset.div_ceil(8);
        let remainder = self.bit_offset % 8;
        if remainder != 0 {
            let padding_width = 8 - remainder;
            let padding_mask = ((1u16 << padding_width) - 1) as u8;
            let byte = self.body[self.start + used_bytes - 1];
            if byte & padding_mask != 0 {
                return Err(format!(
                    "{} has non-canonical padding bits in byte 0x{byte:02X}",
                    self.section
                ));
            }
        }
        *cursor = self
            .start
            .checked_add(used_bytes)
            .ok_or_else(|| format!("{} byte offset overflow", self.section))?;
        Ok(())
    }
}

pub(super) fn read_packed_guid(
    body: &[u8],
    cursor: &mut usize,
    field: &str,
) -> Result<(u64, u64), String> {
    let low_mask = read_u8(body, cursor, &format!("{field} low mask"))?;
    let high_mask = read_u8(body, cursor, &format!("{field} high mask"))?;
    let low = read_packed_u64(body, cursor, low_mask, &format!("{field} low word"))?;
    let high = read_packed_u64(body, cursor, high_mask, &format!("{field} high word"))?;
    Ok((low, high))
}

pub(super) fn read_packed_u64(
    body: &[u8],
    cursor: &mut usize,
    mask: u8,
    field: &str,
) -> Result<u64, String> {
    let mut bytes = [0u8; 8];
    for (index, byte) in bytes.iter_mut().enumerate() {
        if mask & (1 << index) != 0 {
            *byte = read_u8(body, cursor, field)?;
            if *byte == 0 {
                return Err(format!(
                    "{field} uses non-canonical packed encoding at byte {index}"
                ));
            }
        }
    }
    Ok(u64::from_le_bytes(bytes))
}

pub(super) fn read_u8(body: &[u8], cursor: &mut usize, field: &str) -> Result<u8, String> {
    let Some(value) = body.get(*cursor).copied() else {
        return Err(format!("truncated while reading {field} at byte {cursor}"));
    };
    *cursor += 1;
    Ok(value)
}

pub(super) fn read_i8(body: &[u8], cursor: &mut usize, field: &str) -> Result<i8, String> {
    Ok(read_u8(body, cursor, field)? as i8)
}

pub(super) fn read_i16(body: &[u8], cursor: &mut usize, field: &str) -> Result<i16, String> {
    Ok(i16::from_le_bytes(read_array(body, cursor, field)?))
}

pub(super) fn read_i32(body: &[u8], cursor: &mut usize, field: &str) -> Result<i32, String> {
    Ok(i32::from_le_bytes(read_array(body, cursor, field)?))
}

pub(super) fn read_u16(body: &[u8], cursor: &mut usize, field: &str) -> Result<u16, String> {
    Ok(u16::from_le_bytes(read_array(body, cursor, field)?))
}

pub(super) fn read_u16_be(body: &[u8], cursor: &mut usize, field: &str) -> Result<u16, String> {
    Ok(u16::from_be_bytes(read_array(body, cursor, field)?))
}

pub(super) fn read_u32(body: &[u8], cursor: &mut usize, field: &str) -> Result<u32, String> {
    Ok(u32::from_le_bytes(read_array(body, cursor, field)?))
}

pub(super) fn read_u32_be(body: &[u8], cursor: &mut usize, field: &str) -> Result<u32, String> {
    Ok(u32::from_be_bytes(read_array(body, cursor, field)?))
}

pub(super) fn read_f32_bits(body: &[u8], cursor: &mut usize, field: &str) -> Result<u32, String> {
    let bits = read_u32(body, cursor, field)?;
    if !f32::from_bits(bits).is_finite() {
        return Err(format!("{field} is not finite"));
    }
    Ok(bits)
}

pub(super) fn read_wire_position(
    body: &[u8],
    cursor: &mut usize,
    field: &str,
) -> Result<WirePosition, String> {
    Ok(WirePosition {
        x_bits: read_f32_bits(body, cursor, &format!("{field}.x"))?,
        y_bits: read_f32_bits(body, cursor, &format!("{field}.y"))?,
        z_bits: read_f32_bits(body, cursor, &format!("{field}.z"))?,
    })
}

pub(super) fn read_array<const N: usize>(
    body: &[u8],
    cursor: &mut usize,
    field: &str,
) -> Result<[u8; N], String> {
    let end = cursor
        .checked_add(N)
        .ok_or_else(|| format!("offset overflow while reading {field}"))?;
    let bytes = body
        .get(*cursor..end)
        .ok_or_else(|| format!("truncated while reading {field} at byte {cursor}"))?;
    *cursor = end;
    bytes
        .try_into()
        .map_err(|_| format!("invalid width while reading {field}"))
}
