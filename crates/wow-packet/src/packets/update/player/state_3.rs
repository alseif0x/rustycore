//! Player update-value structs state definitions, part 3 of 5.
//!
//! Separated from the player.rs root under #650. Behaviour is preserved.

use super::*;

pub(super) fn write_trait_config_create_data(buf: &mut WorldPacket, data: &TraitConfigCreateData) {
    buf.write_int32(data.id);
    buf.write_int32(data.config_type);
    buf.write_uint32(data.entries.len() as u32);
    if data.config_type == 2 {
        buf.write_int32(data.skill_line_id);
    }
    if data.config_type == 1 {
        buf.write_int32(data.chr_specialization_id);
        buf.write_int32(data.combat_config_flags);
        buf.write_int32(data.local_identifier);
    }
    if data.config_type == 3 {
        buf.write_int32(data.trait_system_id);
    }
    for entry in &data.entries {
        buf.write_int32(entry.trait_node_id);
        buf.write_int32(entry.trait_node_entry_id);
        buf.write_int32(entry.rank);
        buf.write_int32(entry.granted_ranks);
    }
    buf.write_bits(data.name.len() as u32, 9);
    buf.write_string(&data.name);
    buf.flush_bits();
}

pub(in crate::packets::update) fn debug_player_create_values_len_like_cpp(
    data: &PlayerCreateData,
    is_self: bool,
) -> usize {
    let mut values = WorldPacket::new_empty();
    data.write_values_create(&mut values, is_self);
    values.into_data().len()
}

/// The ActivePlayer block in C++ `Object::BuildMovementUpdate`.
///
/// Written when the `ActivePlayer` bit (bit 16) is set in CreateObjectBits.
/// Contains 3 conditional bits, then optionally: scene instance IDs, rune state,
/// and 180 action buttons (4 bytes each = 720 bytes).
///
/// For a fresh player: HasSceneInstanceIDs=false, HasRuneState=false,
/// HasActionButtons=true, all 180 action ids = 0.
pub(in crate::packets::update) const MAX_ACTION_BUTTONS: usize = 180;

pub(in crate::packets::update) fn write_active_player_movement_block(
    buf: &mut WorldPacket,
    action_buttons: &[u32; MAX_ACTION_BUTTONS],
) {
    // 3 bits: HasSceneInstanceIDs, HasRuneState, HasActionButtons
    buf.write_bit(false); // HasSceneInstanceIDs
    buf.write_bit(false); // HasRuneState
    buf.write_bit(true); // HasActionButtons
    buf.flush_bits();

    // HasSceneInstanceIDs: if true, would write i32 count + i32[] IDs (skipped)
    // HasRuneState: if true, would write rune data (skipped)

    // HasActionButtons: 180 action buttons, each i32 (4 bytes)
    for action_button in action_buttons {
        buf.write_uint32(*action_button);
    }
}

/// Write a player VALUES update block.
///
/// Wire format:
/// ```text
/// [u8]  UpdateType = 0 (Values)
/// [PackedGuid] player GUID
/// [u32] values data size
///   [u8] updateFieldFlags (0x01 = Owner)
///   ObjectData.WriteUpdate (4-bit mask, no changes)
///   UnitData.WriteUpdate (8 blocks, VirtualItems at bits 167-170)
///   PlayerData.WriteUpdate (4 blocks, VisibleItems at bits 61-80)
///   ActivePlayerData.WriteUpdate (48 blocks, InvSlots at bits 124-265)
/// ```
pub(in crate::packets::update) fn write_player_values_update_block(
    buf: &mut WorldPacket,
    guid: &ObjectGuid,
    inv_slot_changes: &[(u8, ObjectGuid)],
    buyback_changes: &[(u8, u32, i64)],
    visible_item_changes: &[(u8, i32, u16, u16)],
    virtual_item_changes: &[(u8, i32, u16, u16)],
    stat_changes: Option<&PlayerStatChanges>,
    coinage_change: Option<u64>,
) {
    // UpdateType = Values (0)
    buf.write_uint8(UpdateType::Values as u8);

    // Object GUID
    buf.write_packed_guid(guid);

    // Build values data into temp buffer for size prefix.
    //
    // C++ `Player::BuildValuesUpdate` writes:
    //   [u32] ChangedObjectTypeMask — which TypeId sections have changes
    //   [section data for each changed TypeId]
    //
    // TypeId enum: Object=0, Unit=5, Player=6, ActivePlayer=7
    let mut val_buf = WorldPacket::new_empty();

    // Compute which sections have changes
    let has_unit = !virtual_item_changes.is_empty() || stat_changes.is_some();
    let has_player = !visible_item_changes.is_empty();
    let has_active_player = !inv_slot_changes.is_empty()
        || !buyback_changes.is_empty()
        || stat_changes.is_some()
        || coinage_change.is_some();

    let mut type_mask: u32 = 0;
    if has_unit {
        type_mask |= 1 << 5;
    } // TypeId::Unit = 5
    if has_player {
        type_mask |= 1 << 6;
    } // TypeId::Player = 6
    if has_active_player {
        type_mask |= 1 << 7;
    } // TypeId::ActivePlayer = 7

    val_buf.write_uint32(type_mask);

    // Write only sections that have changes; C++ checks `HasChanged`
    // per TypeId section before writing section payload.
    if has_unit {
        write_unit_data_values_update(&mut val_buf, virtual_item_changes, stat_changes);
    }
    if has_player {
        write_player_data_values_update(&mut val_buf, visible_item_changes);
    }
    if has_active_player {
        write_active_player_data_values_update(
            &mut val_buf,
            inv_slot_changes,
            buyback_changes,
            stat_changes,
            coinage_change,
        );
    }

    // Write with size prefix
    let val_data = val_buf.into_data();
    buf.write_uint32(val_data.len() as u32);
    buf.write_bytes(&val_data);
}

pub(in crate::packets::update) const VALUES_TYPE_PLAYER: u32 = 1 << 6;

pub(in crate::packets::update) const VALUES_TYPE_ACTIVE_PLAYER: u32 = 1 << 7;

pub(super) fn player_mask_has(data: &PlayerDataValuesDeltaUpdate, bit: usize) -> bool {
    let block = bit / 32;
    let offset = bit % 32;
    data.player_data_mask.get(block).copied().unwrap_or(0) & (1 << offset) != 0
}

pub(super) fn write_quest_log_values_update(buf: &mut WorldPacket, data: &QuestLogValuesUpdate) {
    let mask = u64::from(data.quest_log_mask & 0x1FFF_FFFF);
    write_update_field_blocks_mask(buf, mask, 1);
    buf.flush_bits();

    if field_mask_has(mask, 0) {
        if field_mask_has(mask, 1) {
            buf.write_int64(data.end_time);
        }
        if field_mask_has(mask, 2) {
            buf.write_int32(data.quest_id);
        }
        if field_mask_has(mask, 3) {
            buf.write_uint32(data.state_flags);
        }
    }
    if field_mask_has(mask, 4) {
        for (index, progress) in data.objective_progress.iter().enumerate() {
            if field_mask_has(mask, 5 + index) {
                buf.write_uint16(*progress);
            }
        }
    }
}

pub(super) fn write_quest_log_values_create(buf: &mut WorldPacket, data: &QuestLogValuesUpdate) {
    buf.write_int64(data.end_time);
    buf.write_int32(data.quest_id);
    buf.write_uint32(data.state_flags);
    for progress in &data.objective_progress {
        buf.write_uint16(*progress);
    }
}

pub(super) fn write_player_data_values_update_section(
    buf: &mut WorldPacket,
    data: &PlayerDataValuesDeltaUpdate,
) {
    write_update_field_blocks_mask_u32(buf, &data.player_data_mask, 4);

    // C++ currently returns false from IsQuestLogChangesMaskSkipped().
    let no_quest_log_changes_mask = false;
    buf.write_bit(no_quest_log_changes_mask);

    if player_mask_has(data, 0) {
        if player_mask_has(data, 1) {
            write_dynamic_field_update_mask(
                buf,
                data.customizations.len(),
                data.customizations_update_mask.as_deref(),
            );
        }
        if player_mask_has(data, 2) {
            write_dynamic_field_update_mask(
                buf,
                data.arena_cooldowns.len(),
                data.arena_cooldowns_update_mask.as_deref(),
            );
        }
        if player_mask_has(data, 3) {
            write_dynamic_field_update_mask(
                buf,
                data.visual_item_replacements.len(),
                data.visual_item_replacements_update_mask.as_deref(),
            );
        }
    }
    buf.flush_bits();

    if player_mask_has(data, 0) {
        if player_mask_has(data, 1) {
            for (index, customization) in data.customizations.iter().enumerate() {
                if dynamic_mask_has_index(data.customizations_update_mask.as_deref(), index) {
                    write_chr_customization_choice_values_update(buf, customization);
                }
            }
        }
        if player_mask_has(data, 2) {
            for (index, cooldown) in data.arena_cooldowns.iter().enumerate() {
                if dynamic_mask_has_index(data.arena_cooldowns_update_mask.as_deref(), index) {
                    write_arena_cooldown_values_update(buf, cooldown);
                }
            }
        }
        if player_mask_has(data, 3) {
            write_changed_i32_dynamic_values(
                buf,
                &data.visual_item_replacements,
                data.visual_item_replacements_update_mask.as_deref(),
            );
        }
        for (bit, guid) in [
            (4, &data.duel_arbiter),
            (5, &data.wow_account),
            (6, &data.loot_target_guid),
        ] {
            if player_mask_has(data, bit) {
                buf.write_packed_guid(guid);
            }
        }
        for (bit, value) in [
            (7, data.player_flags),
            (8, data.player_flags_ex),
            (9, data.guild_rank_id),
            (10, data.guild_delete_date),
        ] {
            if player_mask_has(data, bit) {
                buf.write_uint32(value);
            }
        }
        if player_mask_has(data, 11) {
            buf.write_int32(data.guild_level);
        }
        for (bit, value) in [
            (12, data.num_bank_slots),
            (13, data.native_sex),
            (14, data.inebriation),
            (15, data.pvp_title),
            (16, data.arena_faction),
            (17, data.pvp_rank),
        ] {
            if player_mask_has(data, bit) {
                buf.write_uint8(value);
            }
        }
        if player_mask_has(data, 18) {
            buf.write_int32(data.field_88);
        }
        if player_mask_has(data, 19) {
            buf.write_uint32(data.duel_team);
        }
        for (bit, value) in [
            (20, data.guild_time_stamp),
            (21, data.player_title),
            (22, data.fake_inebriation),
        ] {
            if player_mask_has(data, bit) {
                buf.write_int32(value);
            }
        }
        if player_mask_has(data, 23) {
            buf.write_uint32(data.virtual_player_realm);
        }
        if player_mask_has(data, 24) {
            buf.write_uint32(data.current_spec_id);
        }
        if player_mask_has(data, 25) {
            buf.write_int32(data.taxi_mount_anim_kit_id);
        }
        if player_mask_has(data, 26) {
            buf.write_uint8(data.current_battle_pet_breed_quality);
        }
        if player_mask_has(data, 27) {
            buf.write_int32(data.honor_level);
        }
        if player_mask_has(data, 28) {
            buf.write_int64(data.logout_time);
        }
        if player_mask_has(data, 29) {
            buf.write_int32(data.current_battle_pet_species_id);
        }
        if player_mask_has(data, 30) {
            buf.write_packed_guid(&data.bnet_account);
        }
        if player_mask_has(data, 31) {
            write_dungeon_score_summary_values_update(buf, &data.dungeon_score);
        }
    }

    if player_mask_has(data, 32) {
        for i in 0..2 {
            if player_mask_has(data, 33 + i) {
                buf.write_uint8(data.party_type[i]);
            }
        }
    }

    if player_mask_has(data, 35) {
        for i in 0..25 {
            if player_mask_has(data, 36 + i) {
                if no_quest_log_changes_mask {
                    write_quest_log_values_create(buf, &data.quest_log[i]);
                } else {
                    write_quest_log_values_update(buf, &data.quest_log[i]);
                }
            }
        }
    }

    if player_mask_has(data, 61) {
        for i in 0..19 {
            if player_mask_has(data, 62 + i) {
                write_visible_item_values_update(buf, &data.visible_items[i]);
            }
        }
    }

    if player_mask_has(data, 81) {
        for i in 0..6 {
            if player_mask_has(data, 82 + i) {
                buf.write_float(data.avg_item_level[i]);
            }
        }
    }

    if player_mask_has(data, 88) {
        for i in 0..19 {
            if player_mask_has(data, 89 + i) {
                buf.write_uint32(data.field_3120[i]);
            }
        }
    }
}

pub(in crate::packets::update) fn write_full_player_values_update_block(
    buf: &mut WorldPacket,
    guid: &ObjectGuid,
    data: &PlayerDataValuesDeltaUpdate,
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
        if let Some(unit_data) = &data.unit_data {
            write_unit_data_values_update_section(&mut val_buf, unit_data);
        }
    }

    if data.changed_object_type_mask & VALUES_TYPE_PLAYER != 0 {
        write_player_data_values_update_section(&mut val_buf, data);
    }

    if data.changed_object_type_mask & VALUES_TYPE_ACTIVE_PLAYER != 0 {
        if let Some(active_player_data) = &data.active_player_data {
            write_active_player_data_values_update_section(&mut val_buf, active_player_data);
        }
    }

    let val_data = val_buf.into_data();
    buf.write_uint32(val_data.len() as u32);
    buf.write_bytes(&val_data);
}

pub(in crate::packets::update) fn write_full_active_player_values_update_block(
    buf: &mut WorldPacket,
    guid: &ObjectGuid,
    data: &ActivePlayerDataValuesUpdate,
) {
    buf.write_uint8(UpdateType::Values as u8);
    buf.write_packed_guid(guid);

    let mut val_buf = WorldPacket::new_empty();
    val_buf.write_uint32(VALUES_TYPE_ACTIVE_PLAYER);
    write_active_player_data_values_update_section(&mut val_buf, data);

    let val_data = val_buf.into_data();
    buf.write_uint32(val_data.len() as u32);
    buf.write_bytes(&val_data);
}

/// PlayerData VALUES update: VisibleItems[19] (equipment display).
///
/// C++ `UF::PlayerData::WriteUpdate` format:
///   WriteBits(blocksMask, 4) — which of 4 blocks have changes
///   for each active block: WriteBits(block, 32)
///   WriteBit(noQuestLogChangesMask) — ALWAYS present after block masks
///   [dynamic array masks if block 0 active: Customizations, ArenaCooldowns, etc.]
///   FlushBits()
///   [dynamic array values]
///   [field values]
///   FlushBits() at end
///
/// VisibleItems: parent=61, elements=62-80. Span blocks 1-2.
pub(super) fn write_player_data_values_update(
    buf: &mut WorldPacket,
    visible_item_changes: &[(u8, i32, u16, u16)],
) {
    let mut blocks = [0u32; 4];

    // Parent bit 61 = block 1 (61/32=1), bit 61%32=29
    blocks[1] |= 1 << 29;

    for &(slot, _, _, _) in visible_item_changes {
        if slot >= 19 {
            continue;
        }
        let bit = 62 + slot as u32;
        let block_idx = (bit / 32) as usize;
        let bit_in_block = bit % 32;
        if block_idx < 4 {
            blocks[block_idx] |= 1 << bit_in_block;
        }
    }

    let mut blocks_mask: u32 = 0;
    for i in 0..4 {
        if blocks[i] != 0 {
            blocks_mask |= 1 << i;
        }
    }

    buf.write_bits(blocks_mask, 4);
    for i in 0..4 {
        if blocks[i] != 0 {
            buf.write_bits(blocks[i], 32);
        }
    }

    // C++ `UF::PlayerData::WriteUpdate` always writes this bit after block masks:
    // bool noQuestLogChangesMask = data.WriteBit(IsQuestLogChangesMaskSkipped());
    // For us, quest log never changed = true (skip it)
    buf.write_bit(true);

    // No dynamic arrays changed (block 0 is not set for VisibleItems-only changes)
    buf.flush_bits();

    // Write VisibleItem values in slot order
    for slot in 0..19u8 {
        if let Some(&(_, item_id, app_mod, item_visual)) =
            visible_item_changes.iter().find(|&&(s, _, _, _)| s == slot)
        {
            // VisibleItem.WriteUpdate: 4-bit mask + flush + data
            buf.write_bits(0x0Fu32, 4);
            buf.flush_bits();
            buf.write_int32(item_id);
            buf.write_uint16(app_mod);
            buf.write_uint16(item_visual);
        }
    }
    buf.flush_bits();
}

pub fn write_skill_info_values_update(buf: &mut WorldPacket, data: &SkillInfoValuesUpdate) {
    let mut group0 = 0u32;
    let mut group1 = 0u32;
    for block in 0..32 {
        if data.skill_info_mask[block] != 0 {
            group0 |= 1 << block;
        }
    }
    for block in 32..57 {
        if data.skill_info_mask[block] != 0 {
            group1 |= 1 << (block - 32);
        }
    }

    buf.write_uint32(group0);
    buf.write_bits(group1, 25);
    for block in data.skill_info_mask {
        if block != 0 {
            buf.write_bits(block, 32);
        }
    }

    buf.flush_bits();
    if field_blocks_have(&data.skill_info_mask, 0) {
        for index in 0..256 {
            if field_blocks_have(&data.skill_info_mask, 1 + index) {
                buf.write_uint16(data.skill_line_id[index]);
            }
            if field_blocks_have(&data.skill_info_mask, 257 + index) {
                buf.write_uint16(data.skill_step[index]);
            }
            if field_blocks_have(&data.skill_info_mask, 513 + index) {
                buf.write_uint16(data.skill_rank[index]);
            }
            if field_blocks_have(&data.skill_info_mask, 769 + index) {
                buf.write_uint16(data.skill_starting_rank[index]);
            }
            if field_blocks_have(&data.skill_info_mask, 1025 + index) {
                buf.write_uint16(data.skill_max_rank[index]);
            }
            if field_blocks_have(&data.skill_info_mask, 1281 + index) {
                buf.write_int16(data.skill_temp_bonus[index]);
            }
            if field_blocks_have(&data.skill_info_mask, 1537 + index) {
                buf.write_uint16(data.skill_perm_bonus[index]);
            }
        }
    }
}

pub fn write_rest_info_values_update(buf: &mut WorldPacket, data: RestInfoValuesUpdate) {
    let mask = data.rest_info_mask & 0x07;
    buf.write_bits(mask as u32, 3);

    buf.flush_bits();
    if mask & 0x01 != 0 {
        if mask & 0x02 != 0 {
            buf.write_uint32(data.threshold);
        }
        if mask & 0x04 != 0 {
            buf.write_uint8(data.state_id);
        }
    }
}

pub fn write_pvp_info_values_update(buf: &mut WorldPacket, data: PvpInfoValuesUpdate) {
    let mask = data.pvp_info_mask & 0x0007_FFFF;
    buf.write_bits(mask, 19);

    if mask & 0x01 != 0 && mask & 0x02 != 0 {
        buf.write_bit(data.disqualified);
    }
    buf.flush_bits();

    if mask & 0x01 != 0 {
        if mask & 0x0000_0004 != 0 {
            buf.write_int8(data.bracket);
        }
        if mask & 0x0000_0008 != 0 {
            buf.write_int32(data.pvp_rating_id);
        }
        if mask & 0x0000_0010 != 0 {
            buf.write_uint32(data.weekly_played);
        }
        if mask & 0x0000_0020 != 0 {
            buf.write_uint32(data.weekly_won);
        }
        if mask & 0x0000_0040 != 0 {
            buf.write_uint32(data.season_played);
        }
        if mask & 0x0000_0080 != 0 {
            buf.write_uint32(data.season_won);
        }
        if mask & 0x0000_0100 != 0 {
            buf.write_uint32(data.rating);
        }
        if mask & 0x0000_0200 != 0 {
            buf.write_uint32(data.weekly_best_rating);
        }
        if mask & 0x0000_0400 != 0 {
            buf.write_uint32(data.season_best_rating);
        }
        if mask & 0x0000_0800 != 0 {
            buf.write_uint32(data.pvp_tier_id);
        }
        if mask & 0x0000_1000 != 0 {
            buf.write_uint32(data.weekly_best_win_pvp_tier_id);
        }
        if mask & 0x0000_2000 != 0 {
            buf.write_uint32(data.field_28);
        }
        if mask & 0x0000_4000 != 0 {
            buf.write_uint32(data.field_2c);
        }
        if mask & 0x0000_8000 != 0 {
            buf.write_uint32(data.weekly_rounds_played);
        }
        if mask & 0x0001_0000 != 0 {
            buf.write_uint32(data.weekly_rounds_won);
        }
        if mask & 0x0002_0000 != 0 {
            buf.write_uint32(data.season_rounds_played);
        }
        if mask & 0x0004_0000 != 0 {
            buf.write_uint32(data.season_rounds_won);
        }
    }
    buf.flush_bits();
}

pub fn write_character_restriction_values_update(
    buf: &mut WorldPacket,
    data: CharacterRestrictionValuesUpdate,
) {
    buf.write_int32(data.field_0);
    buf.write_int32(data.field_4);
    buf.write_int32(data.field_8);
    buf.write_bits(u32::from(data.restriction_type), 5);
    buf.flush_bits();
}

pub fn write_trait_entry_values_update(buf: &mut WorldPacket, data: TraitEntryValuesUpdate) {
    buf.write_int32(data.trait_node_id);
    buf.write_int32(data.trait_node_entry_id);
    buf.write_int32(data.rank);
    buf.write_int32(data.granted_ranks);
}

pub fn write_trait_config_values_update(buf: &mut WorldPacket, data: &TraitConfigValuesUpdate) {
    let mask = data.trait_config_mask & 0x0FFF;
    buf.write_bits(mask as u32, 12);

    if mask & 0x001 != 0 && mask & 0x002 != 0 {
        write_dynamic_field_update_mask(
            buf,
            data.entries.len(),
            data.entries_update_mask.as_deref(),
        );
    }
    buf.flush_bits();

    if mask & 0x001 != 0 {
        if mask & 0x002 != 0 {
            for (index, entry) in data.entries.iter().enumerate() {
                if dynamic_mask_has_index(data.entries_update_mask.as_deref(), index) {
                    write_trait_entry_values_update(buf, *entry);
                }
            }
        }
        if mask & 0x004 != 0 {
            buf.write_int32(data.id);
        }
    }
    if mask & 0x010 != 0 {
        if mask & 0x020 != 0 {
            buf.write_int32(data.config_type);
        }
        if mask & 0x040 != 0 && data.config_type == 2 {
            buf.write_int32(data.skill_line_id);
        }
        if mask & 0x080 != 0 && data.config_type == 1 {
            buf.write_int32(data.chr_specialization_id);
        }
    }
    if mask & 0x100 != 0 {
        if mask & 0x200 != 0 && data.config_type == 1 {
            buf.write_int32(data.combat_config_flags);
        }
        if mask & 0x400 != 0 && data.config_type == 1 {
            buf.write_int32(data.local_identifier);
        }
        if mask & 0x800 != 0 && data.config_type == 3 {
            buf.write_int32(data.trait_system_id);
        }
    }
    if mask & 0x001 != 0 && mask & 0x008 != 0 {
        buf.write_bits(data.name.len() as u32, 9);
        buf.write_string(&data.name);
    }
    buf.flush_bits();
}

pub(super) fn active_player_mask_has(data: &ActivePlayerDataValuesUpdate, bit: usize) -> bool {
    field_blocks_have(&data.active_player_data_mask, bit)
}
