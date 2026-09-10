//! Values update packets.
//!
//! Separated from unit.rs under #689.

use super::*;

// ── ItemCreateData ──────────────────────────────────────────────────

/// C++ `Unit::Update` → `ModifyAuraState` health-derived `UNIT_FIELD_AURASTATE` bits
/// (Unit.cpp:469-476), applied to EVERY alive unit including the player. A full-HP unit
/// yields 0x00D00000. Mirrors `WorldCreature::health_aura_state_like_cpp` in wow-world
/// (both implement the same AURA_STATE 1-based-index bit math; kept in sync).
pub(in crate::packets::update) fn health_aura_state_like_cpp(
    health: i64,
    max_health: i64,
    alive: bool,
) -> u32 {
    if !alive || max_health <= 0 {
        return 0;
    }
    let below = |p: i64| health.saturating_mul(100) < max_health.saturating_mul(p);
    let above = |p: i64| health.saturating_mul(100) > max_health.saturating_mul(p);
    let mut state = 0u32;
    let mut set = |idx: u32, on: bool| {
        if on {
            state |= 1 << (idx - 1);
        }
    };
    set(2, below(20)); // AURA_STATE_WOUNDED_20_PERCENT
    set(6, below(25)); // AURA_STATE_WOUNDED_25_PERCENT
    set(13, below(35)); // AURA_STATE_WOUNDED_35_PERCENT
    set(21, below(20) || above(80)); // AURA_STATE_WOUND_HEALTH_20_80
    set(23, above(75)); // AURA_STATE_HEALTHY_75_PERCENT
    set(24, below(35) || above(80)); // AURA_STATE_WOUND_HEALTH_35_80
    state
}

/// Get power type for a class (0=mana, 1=rage, 3=energy).
pub(in crate::packets::update) fn power_type_for_class(class: u8) -> u8 {
    match class {
        1 => 1,  // Warrior → Rage
        4 => 3,  // Rogue → Energy
        11 => 0, // Druid → Mana
        6 => 6,  // DeathKnight → Runic Power (POWER_RUNIC_POWER, C++ SharedDefines.h:287)
        _ => 0,  // Default → Mana
    }
}

pub(in crate::packets::update) const VALUES_TYPE_UNIT: u32 = 1 << 5;

pub(in crate::packets::update) fn write_unit_data_values_update_section(
    buf: &mut WorldPacket,
    data: &UnitDataValuesDeltaUpdate,
) {
    write_update_field_blocks_mask_u32(buf, &data.unit_data_mask, 8);

    if unit_mask_has(data, 0) && unit_mask_has(data, 1) {
        buf.write_bits(data.state_world_effect_ids.len() as u32, 32);
        for effect_id in &data.state_world_effect_ids {
            buf.write_uint32(*effect_id);
        }
    }
    buf.flush_bits();

    if unit_mask_has(data, 0) {
        if unit_mask_has(data, 2) {
            write_dynamic_field_update_mask(
                buf,
                data.passive_spells.len(),
                data.passive_spells_update_mask.as_deref(),
            );
        }
        if unit_mask_has(data, 3) {
            write_dynamic_field_update_mask(
                buf,
                data.world_effects.len(),
                data.world_effects_update_mask.as_deref(),
            );
        }
        if unit_mask_has(data, 4) {
            write_dynamic_field_update_mask(
                buf,
                data.channel_objects.len(),
                data.channel_objects_update_mask.as_deref(),
            );
        }
    }
    buf.flush_bits();

    if unit_mask_has(data, 0) {
        if unit_mask_has(data, 2) {
            for (index, spell) in data.passive_spells.iter().enumerate() {
                if dynamic_mask_has_index(data.passive_spells_update_mask.as_deref(), index) {
                    write_passive_spell_history_values_update(buf, spell);
                }
            }
        }
        if unit_mask_has(data, 3) {
            write_changed_i32_dynamic_values(
                buf,
                &data.world_effects,
                data.world_effects_update_mask.as_deref(),
            );
        }
        if unit_mask_has(data, 4) {
            for (index, guid) in data.channel_objects.iter().enumerate() {
                if dynamic_mask_has_index(data.channel_objects_update_mask.as_deref(), index) {
                    buf.write_packed_guid(guid);
                }
            }
        }
        if unit_mask_has(data, 5) {
            buf.write_int64(data.health);
        }
        if unit_mask_has(data, 6) {
            buf.write_int64(data.max_health);
        }
        if unit_mask_has(data, 7) {
            buf.write_int32(data.display_id);
        }
        if unit_mask_has(data, 8) {
            buf.write_uint32(data.state_spell_visual_id);
        }
        if unit_mask_has(data, 9) {
            buf.write_uint32(data.state_anim_id);
        }
        if unit_mask_has(data, 10) {
            buf.write_uint32(data.state_anim_kit_id);
        }
        for (bit, guid) in [
            (11, &data.charm),
            (12, &data.summon),
            (13, &data.critter),
            (14, &data.charmed_by),
            (15, &data.summoned_by),
            (16, &data.created_by),
            (17, &data.demon_creator),
            (18, &data.look_at_controller_target),
            (19, &data.target),
            (20, &data.battle_pet_companion_guid),
        ] {
            if unit_mask_has(data, bit) {
                buf.write_packed_guid(guid);
            }
        }
        if unit_mask_has(data, 21) {
            buf.write_uint64(data.battle_pet_db_id);
        }
        if unit_mask_has(data, 22) {
            write_unit_channel_values_update(buf, &data.channel_data);
        }
        if unit_mask_has(data, 23) {
            buf.write_uint32(data.summoned_by_home_realm);
        }
        if unit_mask_has(data, 24) {
            buf.write_uint8(data.race);
        }
        if unit_mask_has(data, 25) {
            buf.write_uint8(data.class_id);
        }
        if unit_mask_has(data, 26) {
            buf.write_uint8(data.player_class_id);
        }
        if unit_mask_has(data, 27) {
            buf.write_uint8(data.sex);
        }
        if unit_mask_has(data, 28) {
            buf.write_uint8(data.display_power);
        }
        if unit_mask_has(data, 29) {
            buf.write_uint32(data.override_display_power_id);
        }
        if unit_mask_has(data, 30) {
            buf.write_int32(data.level);
        }
        if unit_mask_has(data, 31) {
            buf.write_int32(data.effective_level);
        }
    }

    if unit_mask_has(data, 32) {
        for (bit, value) in [
            (33, data.content_tuning_id),
            (34, data.scaling_level_min),
            (35, data.scaling_level_max),
            (36, data.scaling_level_delta),
            (37, data.scaling_faction_group),
            (38, data.scaling_health_item_level_curve_id),
            (39, data.scaling_damage_item_level_curve_id),
            (40, data.faction_template),
        ] {
            if unit_mask_has(data, bit) {
                buf.write_int32(value);
            }
        }
        if unit_mask_has(data, 41) {
            buf.write_uint32(data.flags);
        }
        if unit_mask_has(data, 42) {
            buf.write_uint32(data.flags2);
        }
        if unit_mask_has(data, 43) {
            buf.write_uint32(data.flags3);
        }
        if unit_mask_has(data, 44) {
            buf.write_uint32(data.aura_state);
        }
        if unit_mask_has(data, 45) {
            buf.write_uint32(data.ranged_attack_round_base_time);
        }
        for (bit, value) in [
            (46, data.bounding_radius),
            (47, data.combat_reach),
            (48, data.display_scale),
        ] {
            if unit_mask_has(data, bit) {
                buf.write_float(value);
            }
        }
        if unit_mask_has(data, 49) {
            buf.write_int32(data.native_display_id);
        }
        if unit_mask_has(data, 50) {
            buf.write_float(data.native_display_scale);
        }
        if unit_mask_has(data, 51) {
            buf.write_int32(data.mount_display_id);
        }
        for (bit, value) in [
            (52, data.min_damage),
            (53, data.max_damage),
            (54, data.min_off_hand_damage),
            (55, data.max_off_hand_damage),
        ] {
            if unit_mask_has(data, bit) {
                buf.write_float(value);
            }
        }
        for (bit, value) in [
            (56, data.stand_state),
            (57, data.pet_talent_points),
            (58, data.vis_flags),
            (59, data.anim_tier),
        ] {
            if unit_mask_has(data, bit) {
                buf.write_uint8(value);
            }
        }
        for (bit, value) in [
            (60, data.pet_number),
            (61, data.pet_name_timestamp),
            (62, data.pet_experience),
            (63, data.pet_next_level_experience),
        ] {
            if unit_mask_has(data, bit) {
                buf.write_uint32(value);
            }
        }
    }

    if unit_mask_has(data, 64) {
        for (bit, value) in [
            (65, data.mod_casting_speed),
            (66, data.mod_spell_haste),
            (67, data.mod_haste),
            (68, data.mod_ranged_haste),
            (69, data.mod_haste_regen),
            (70, data.mod_time_rate),
        ] {
            if unit_mask_has(data, bit) {
                buf.write_float(value);
            }
        }
        for (bit, value) in [(71, data.created_by_spell), (72, data.emote_state)] {
            if unit_mask_has(data, bit) {
                buf.write_int32(value);
            }
        }
        if unit_mask_has(data, 73) {
            buf.write_int16(data.training_points_used);
        }
        if unit_mask_has(data, 74) {
            buf.write_int16(data.training_points_total);
        }
        if unit_mask_has(data, 75) {
            buf.write_int32(data.base_mana);
        }
        if unit_mask_has(data, 76) {
            buf.write_int32(data.base_health);
        }
        for (bit, value) in [
            (77, data.sheathe_state),
            (78, data.pvp_flags),
            (79, data.pet_flags),
            (80, data.shapeshift_form),
        ] {
            if unit_mask_has(data, bit) {
                buf.write_uint8(value);
            }
        }
        for (bit, value) in [
            (81, data.attack_power),
            (82, data.attack_power_mod_pos),
            (83, data.attack_power_mod_neg),
        ] {
            if unit_mask_has(data, bit) {
                buf.write_int32(value);
            }
        }
        if unit_mask_has(data, 84) {
            buf.write_float(data.attack_power_multiplier);
        }
        for (bit, value) in [
            (85, data.ranged_attack_power),
            (86, data.ranged_attack_power_mod_pos),
            (87, data.ranged_attack_power_mod_neg),
        ] {
            if unit_mask_has(data, bit) {
                buf.write_int32(value);
            }
        }
        if unit_mask_has(data, 88) {
            buf.write_float(data.ranged_attack_power_multiplier);
        }
        if unit_mask_has(data, 89) {
            buf.write_int32(data.set_attack_speed_aura);
        }
        for (bit, value) in [
            (90, data.lifesteal),
            (91, data.min_ranged_damage),
            (92, data.max_ranged_damage),
            (93, data.max_health_modifier),
            (94, data.hover_height),
        ] {
            if unit_mask_has(data, bit) {
                buf.write_float(value);
            }
        }
        if unit_mask_has(data, 95) {
            buf.write_int32(data.min_item_level_cutoff);
        }
    }

    if unit_mask_has(data, 96) {
        for (bit, value) in [
            (97, data.min_item_level),
            (98, data.max_item_level),
            (99, data.wild_battle_pet_level),
        ] {
            if unit_mask_has(data, bit) {
                buf.write_int32(value);
            }
        }
        if unit_mask_has(data, 100) {
            buf.write_uint32(data.battle_pet_companion_name_timestamp);
        }
        for (bit, value) in [
            (101, data.interact_spell_id),
            (102, data.scale_duration),
            (103, data.looks_like_mount_id),
            (104, data.looks_like_creature_id),
            (105, data.look_at_controller_id),
            (106, data.perks_vendor_item_id),
        ] {
            if unit_mask_has(data, bit) {
                buf.write_int32(value);
            }
        }
        if unit_mask_has(data, 107) {
            buf.write_packed_guid(&data.guild_guid);
        }
        if unit_mask_has(data, 108) {
            buf.write_packed_guid(&data.skinning_owner_guid);
        }
        if unit_mask_has(data, 109) {
            buf.write_int32(data.flight_capability_id);
        }
        if unit_mask_has(data, 110) {
            buf.write_float(data.glide_event_speed_divisor);
        }
        if unit_mask_has(data, 111) {
            buf.write_uint32(data.current_area_id);
        }
        if unit_mask_has(data, 112) {
            buf.write_packed_guid(&data.combo_target);
        }
    }

    if unit_mask_has(data, 113) {
        for i in 0..2 {
            if unit_mask_has(data, 114 + i) {
                buf.write_uint32(data.npc_flags[i]);
            }
        }
    }

    if unit_mask_has(data, 116) {
        for i in 0..10 {
            if unit_mask_has(data, 117 + i) {
                buf.write_float(data.power_regen_flat_modifier[i]);
            }
            if unit_mask_has(data, 127 + i) {
                buf.write_float(data.power_regen_interrupted_flat_modifier[i]);
            }
            if unit_mask_has(data, 137 + i) {
                buf.write_int32(data.power[i]);
            }
            if unit_mask_has(data, 147 + i) {
                buf.write_int32(data.max_power[i]);
            }
            if unit_mask_has(data, 157 + i) {
                buf.write_float(data.mod_power_regen[i]);
            }
        }
    }

    if unit_mask_has(data, 167) {
        for i in 0..3 {
            if unit_mask_has(data, 168 + i) {
                write_visible_item_values_update(buf, &data.virtual_items[i]);
            }
        }
    }

    if unit_mask_has(data, 171) {
        for i in 0..2 {
            if unit_mask_has(data, 172 + i) {
                buf.write_uint32(data.attack_round_base_time[i]);
            }
        }
    }

    if unit_mask_has(data, 174) {
        for i in 0..5 {
            if unit_mask_has(data, 175 + i) {
                buf.write_int32(data.stats[i]);
            }
            if unit_mask_has(data, 180 + i) {
                buf.write_int32(data.stat_pos_buff[i]);
            }
            if unit_mask_has(data, 185 + i) {
                buf.write_int32(data.stat_neg_buff[i]);
            }
        }
    }

    if unit_mask_has(data, 190) {
        for i in 0..7 {
            if unit_mask_has(data, 191 + i) {
                buf.write_int32(data.resistances[i]);
            }
            if unit_mask_has(data, 198 + i) {
                buf.write_int32(data.power_cost_modifier[i]);
            }
            if unit_mask_has(data, 205 + i) {
                buf.write_float(data.power_cost_multiplier[i]);
            }
        }
    }

    if unit_mask_has(data, 212) {
        for i in 0..7 {
            if unit_mask_has(data, 213 + i) {
                buf.write_int32(data.resistance_buff_mods_positive[i]);
            }
            if unit_mask_has(data, 220 + i) {
                buf.write_int32(data.resistance_buff_mods_negative[i]);
            }
        }
    }
}

pub(in crate::packets::update) fn write_full_unit_values_update_block(
    buf: &mut WorldPacket,
    guid: &ObjectGuid,
    data: &UnitDataValuesDeltaUpdate,
) {
    buf.write_uint8(UpdateType::Values as u8);
    buf.write_packed_guid(guid);

    let mut val_buf = WorldPacket::new_empty();
    val_buf.write_uint32(data.changed_object_type_mask);

    if data.changed_object_type_mask & VALUES_TYPE_OBJECT != 0 {
        if let Some(object_data) = data.object_data {
            write_object_data_values_update_section(&mut val_buf, object_data);
        } else {
            write_object_data_values_update_section(
                &mut val_buf,
                ObjectDataValuesUpdate {
                    changed_object_type_mask: VALUES_TYPE_OBJECT,
                    object_data_mask: 0,
                    entry_id: 0,
                    dynamic_flags: 0,
                    scale: 0.0,
                },
            );
        }
    }

    if data.changed_object_type_mask & VALUES_TYPE_UNIT != 0 {
        write_unit_data_values_update_section(&mut val_buf, data);
    }

    let val_data = val_buf.into_data();
    buf.write_uint32(val_data.len() as u32);
    buf.write_bytes(&val_data);
}

/// UnitData VALUES update: VirtualItems[3] and/or stat fields.
///
/// C++ `UF::UnitData::WriteUpdate` format
/// (`Entities/Object/Updates/UpdateFields.cpp:852-900`):
///   WriteBits(blocksMask, 8) — which of 8 blocks have changes
///   for each active block: WriteBits(block, 32)
///   [dynamic arrays if block 0 active]
///   FlushBits()
///   [field values in generated C++ update-field definition order]
///
/// Field write order (C++ `UnitData::WriteUpdate`):
///   Block 0: Health(5), MaxHealth(6)
///   Block 1: MinDamage(52→20), MaxDamage(53→21)
///   Block 2: BaseMana(75→11), BaseHealth(76→12), AttackPower(81-84→17-20),
///            RangedAttackPower(85-88→21-24), MinRangedDamage(91→27), MaxRangedDamage(92→28)
///   Block 3: Power parent(116→20)
///   Block 4: Power[0](137→9), MaxPower[0](147→19)
///   Block 5: VirtualItems(167-170→7-10), Stats(174-179→14-19),
///            StatPosBuff(180-184→20-24), StatNegBuff(185-189→25-29),
///            Resistances(190-191→30-31)
pub(in crate::packets::update) fn write_unit_data_values_update(
    buf: &mut WorldPacket,
    virtual_item_changes: &[(u8, i32, u16, u16)],
    stat_changes: Option<&PlayerStatChanges>,
) {
    let mut blocks = [0u32; 8];

    // VirtualItems in block 5
    if !virtual_item_changes.is_empty() {
        blocks[5] |= 1 << 7; // parent bit 167
        for &(idx, _, _, _) in virtual_item_changes {
            if idx < 3 {
                blocks[5] |= 1 << (8 + idx);
            }
        }
    }

    // Stat change bits
    if stat_changes.is_some() {
        blocks[0] |= (1 << 0) | (1 << 5) | (1 << 6);
        blocks[1] |= (1 << 0) | (1 << 20) | (1 << 21);
        blocks[2] |= (1 << 0)
            | (1 << 11)
            | (1 << 12)
            | (1 << 17)
            | (1 << 18)
            | (1 << 19)
            | (1 << 20)
            | (1 << 21)
            | (1 << 22)
            | (1 << 23)
            | (1 << 24)
            | (1 << 27)
            | (1 << 28);
        blocks[3] |= (1 << 20) | (1 << 21) | (1 << 31);
        blocks[4] |= (1 << 9) | (1 << 19) | (1 << 29);
        blocks[5] |= (1 << 14)
            | (1 << 15)
            | (1 << 16)
            | (1 << 17)
            | (1 << 18)
            | (1 << 19)
            | (1 << 20)
            | (1 << 21)
            | (1 << 22)
            | (1 << 23)
            | (1 << 24)
            | (1 << 25)
            | (1 << 26)
            | (1 << 27)
            | (1 << 28)
            | (1 << 29)
            | (1 << 30)
            | (1 << 31);
    }

    let mut blocks_mask: u32 = 0;
    for i in 0..8 {
        if blocks[i] != 0 {
            blocks_mask |= 1 << i;
        }
    }

    buf.write_bits(blocks_mask, 8);
    for i in 0..8 {
        if blocks[i] != 0 {
            buf.write_bits(blocks[i], 32);
        }
    }

    // Dynamic arrays: block 0 bit 0 set enters the generated C++ dynamic-array
    // check, but bits 1-4 are NOT set, so nothing is written.
    buf.flush_bits();

    // ── Field values in generated C++ definition order ──
    // Blocks 0-4: only stat fields
    if let Some(sc) = stat_changes {
        // Block 0: Health, MaxHealth
        buf.write_int64(sc.health);
        buf.write_int64(sc.max_health);

        // Block 1: MinDamage, MaxDamage
        buf.write_float(sc.min_damage);
        buf.write_float(sc.max_damage);

        // Block 2: BaseMana, BaseHealth, AP base/modifiers, ranged AP
        //          base/modifiers, MinRangedDamage, MaxRangedDamage
        buf.write_int32(sc.base_mana);
        buf.write_int32(sc.base_health);
        buf.write_int32(sc.attack_power);
        buf.write_int32(sc.attack_power_mod_pos);
        buf.write_int32(sc.attack_power_mod_neg);
        buf.write_float(sc.attack_power_multiplier);
        buf.write_int32(sc.ranged_attack_power);
        buf.write_int32(sc.ranged_attack_power_mod_pos);
        buf.write_int32(sc.ranged_attack_power_mod_neg);
        buf.write_float(sc.ranged_attack_power_multiplier);
        buf.write_float(sc.min_ranged_damage);
        buf.write_float(sc.max_ranged_damage);

        // Blocks 3-4: Power interleaved loop (index 0)
        // C++ writes PowerRegenFlat, PowerRegenInterrupted, Power, MaxPower,
        // ModPowerRegen in generated update-field order.
        buf.write_float(sc.mana_regen); // PowerRegenFlatModifier[0]
        buf.write_float(sc.mana_regen_combat); // PowerRegenInterruptedFlatModifier[0]
        buf.write_int32(sc.power0); // Power[0]
        buf.write_int32(sc.max_power0); // MaxPower[0]
        buf.write_float(sc.mana_regen_mp5); // ModPowerRegen[0]
    }

    // Block 5: VirtualItems FIRST (bits 7-10), then Stats (14-24), then Resistances (30-31)
    for idx in 0..3u8 {
        if let Some(&(_, item_id, app_mod, item_visual)) =
            virtual_item_changes.iter().find(|&&(i, _, _, _)| i == idx)
        {
            buf.write_bits(0x0Fu32, 4);
            buf.flush_bits();
            buf.write_int32(item_id);
            buf.write_uint16(app_mod);
            buf.write_uint16(item_visual);
        }
    }

    // Stats/StatPosBuff/StatNegBuff are interleaved per index in generated C++
    // update-field order, then Resistances after VirtualItems in block 5.
    if let Some(sc) = stat_changes {
        for i in 0..5 {
            buf.write_int32(sc.stats[i]); // Stats[i]
            buf.write_int32(sc.stat_pos_buff[i]); // StatPosBuff[i]
            buf.write_int32(sc.stat_neg_buff[i]); // StatNegBuff[i]
        }
        buf.write_int32(sc.armor); // Resistances[0]
    }
}

/// Write a creature VALUES update block containing only health + max_health.
///
/// C++ `UF::UnitData::WriteUpdate` field positions:
///   `Health    = new(0, 5)` → block 0, bit 5
///   `MaxHealth = new(0, 6)` → block 0, bit 6
///   Bit 0 is the parent/dynamic-array indicator bit.
///
/// Wire format:
/// ```text
/// [u8]  UpdateType = 0 (Values)
/// [PackedGuid] creature GUID
/// [u32] data_size
///   [u32] ChangedObjectTypeMask = 1<<5 (TypeId::Unit)
///   UnitData block masks (8 words): only block 0 is non-zero = 0x61 (bits 0|5|6)
///   block 0 values: Health (i64), MaxHealth (i64)
/// ```
pub(in crate::packets::update) fn write_creature_health_update_block(
    buf: &mut WorldPacket,
    guid: &ObjectGuid,
    health: i64,
    max_health: i64,
) {
    buf.write_uint8(UpdateType::Values as u8);
    buf.write_packed_guid(guid);

    let mut val_buf = WorldPacket::new_empty();

    // ChangedObjectTypeMask: TypeId::Unit = 5 → bit 5 = 32
    val_buf.write_uint32(1 << 5);

    // UnitData section
    // 8 block words, only block 0 is set (bits 0, 5, 6).
    let block0: u32 = (1 << 0) | (1 << 5) | (1 << 6);
    // Emit: non-zero block mask (which blocks to include), then block 0 only.
    // The encoding is: 8-bit mask of which of the 8 words are present,
    // then the non-zero words in order.
    val_buf.write_bits(0x01u32, 8); // only block 0
    val_buf.write_bits(block0, 32);
    val_buf.flush_bits();

    // block 0 fields: Health (i64) then MaxHealth (i64).
    val_buf.write_int64(health);
    val_buf.write_int64(max_health);

    let data = val_buf.into_data();
    buf.write_uint32(data.len() as u32);
    buf.write_bytes(&data);
}
