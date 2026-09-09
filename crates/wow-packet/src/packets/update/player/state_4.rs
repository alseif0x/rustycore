//! Player update-value structs state definitions, part 4 of 5.
//!
//! Separated from the player.rs root under #650. Behaviour is preserved.

use super::*;

pub fn write_active_player_data_values_update_section(
    buf: &mut WorldPacket,
    data: &ActivePlayerDataValuesUpdate,
) {
    let mut group0 = 0u32;
    let mut group1 = 0u32;
    for block in 0..32 {
        if data.active_player_data_mask[block] != 0 {
            group0 |= 1 << block;
        }
    }
    for block in 32..48 {
        if data.active_player_data_mask[block] != 0 {
            group1 |= 1 << (block - 32);
        }
    }

    buf.write_uint32(group0);
    buf.write_bits(group1, 16);
    for block in data.active_player_data_mask {
        if block != 0 {
            buf.write_bits(block, 32);
        }
    }

    if active_player_mask_has(data, 0) {
        if active_player_mask_has(data, 1) {
            buf.write_bit(data.sort_bags_right_to_left);
        }
        if active_player_mask_has(data, 2) {
            buf.write_bit(data.insert_items_left_to_right);
        }
        if active_player_mask_has(data, 3) {
            write_dynamic_field_update_mask(
                buf,
                data.known_titles.len(),
                data.known_titles_update_mask.as_deref(),
            );
        }
    }
    if active_player_mask_has(data, 20) && active_player_mask_has(data, 21) {
        write_dynamic_field_update_mask(
            buf,
            data.research_sites.len(),
            data.research_sites_update_mask.as_deref(),
        );
    }
    if active_player_mask_has(data, 22) && active_player_mask_has(data, 23) {
        write_dynamic_field_update_mask(
            buf,
            data.research_site_progress.len(),
            data.research_site_progress_update_mask.as_deref(),
        );
    }
    if active_player_mask_has(data, 24) && active_player_mask_has(data, 25) {
        write_dynamic_field_update_mask(
            buf,
            data.research.len(),
            data.research_update_mask.as_deref(),
        );
    }
    if active_player_mask_has(data, 20) && active_player_mask_has(data, 21) {
        for (index, value) in data.research_sites.iter().enumerate() {
            if dynamic_mask_has_index(data.research_sites_update_mask.as_deref(), index) {
                buf.write_uint16(*value);
            }
        }
    }
    if active_player_mask_has(data, 22) && active_player_mask_has(data, 23) {
        for (index, value) in data.research_site_progress.iter().enumerate() {
            if dynamic_mask_has_index(data.research_site_progress_update_mask.as_deref(), index) {
                buf.write_uint32(*value);
            }
        }
    }
    if active_player_mask_has(data, 24) && active_player_mask_has(data, 25) {
        for (index, research) in data.research.iter().enumerate() {
            if dynamic_mask_has_index(data.research_update_mask.as_deref(), index) {
                write_research_values_update(buf, *research);
            }
        }
    }
    buf.flush_bits();

    if active_player_mask_has(data, 0) {
        if active_player_mask_has(data, 4) {
            write_dynamic_field_update_mask(
                buf,
                data.daily_quests_completed.len(),
                data.daily_quests_completed_update_mask.as_deref(),
            );
        }
        if active_player_mask_has(data, 5) {
            write_dynamic_field_update_mask(
                buf,
                data.available_quest_line_x_quest_ids.len(),
                data.available_quest_line_x_quest_ids_update_mask.as_deref(),
            );
        }
        if active_player_mask_has(data, 6) {
            write_dynamic_field_update_mask(
                buf,
                data.field_1000.len(),
                data.field_1000_update_mask.as_deref(),
            );
        }
        if active_player_mask_has(data, 7) {
            write_dynamic_field_update_mask(
                buf,
                data.heirlooms.len(),
                data.heirlooms_update_mask.as_deref(),
            );
        }
        if active_player_mask_has(data, 8) {
            write_dynamic_field_update_mask(
                buf,
                data.heirloom_flags.len(),
                data.heirloom_flags_update_mask.as_deref(),
            );
        }
        if active_player_mask_has(data, 9) {
            write_dynamic_field_update_mask(buf, data.toys.len(), data.toys_update_mask.as_deref());
        }
        if active_player_mask_has(data, 10) {
            write_dynamic_field_update_mask(
                buf,
                data.transmog.len(),
                data.transmog_update_mask.as_deref(),
            );
        }
        if active_player_mask_has(data, 11) {
            write_dynamic_field_update_mask(
                buf,
                data.conditional_transmog.len(),
                data.conditional_transmog_update_mask.as_deref(),
            );
        }
        if active_player_mask_has(data, 12) {
            write_dynamic_field_update_mask(
                buf,
                data.self_res_spells.len(),
                data.self_res_spells_update_mask.as_deref(),
            );
        }
        if active_player_mask_has(data, 13) {
            write_dynamic_field_update_mask(
                buf,
                data.character_restrictions.len(),
                data.character_restrictions_update_mask.as_deref(),
            );
        }
        if active_player_mask_has(data, 14) {
            write_dynamic_field_update_mask(
                buf,
                data.spell_pct_mod_by_label.len(),
                data.spell_pct_mod_by_label_update_mask.as_deref(),
            );
        }
        if active_player_mask_has(data, 15) {
            write_dynamic_field_update_mask(
                buf,
                data.spell_flat_mod_by_label.len(),
                data.spell_flat_mod_by_label_update_mask.as_deref(),
            );
        }
        if active_player_mask_has(data, 16) {
            write_dynamic_field_update_mask(
                buf,
                data.task_quests.len(),
                data.task_quests_update_mask.as_deref(),
            );
        }
        if active_player_mask_has(data, 17) {
            write_dynamic_field_update_mask(
                buf,
                data.trait_configs.len(),
                data.trait_configs_update_mask.as_deref(),
            );
        }
        if active_player_mask_has(data, 18) {
            write_dynamic_field_update_mask(
                buf,
                data.category_cooldown_mods.len(),
                data.category_cooldown_mods_update_mask.as_deref(),
            );
        }
        if active_player_mask_has(data, 19) {
            write_dynamic_field_update_mask(
                buf,
                data.weekly_spell_uses.len(),
                data.weekly_spell_uses_update_mask.as_deref(),
            );
        }
    }
    buf.flush_bits();

    if active_player_mask_has(data, 0) {
        if active_player_mask_has(data, 3) {
            for (index, value) in data.known_titles.iter().enumerate() {
                if dynamic_mask_has_index(data.known_titles_update_mask.as_deref(), index) {
                    buf.write_uint64(*value);
                }
            }
        }
        if active_player_mask_has(data, 4) {
            for (index, value) in data.daily_quests_completed.iter().enumerate() {
                if dynamic_mask_has_index(data.daily_quests_completed_update_mask.as_deref(), index)
                {
                    buf.write_int32(*value);
                }
            }
        }
        if active_player_mask_has(data, 5) {
            for (index, value) in data.available_quest_line_x_quest_ids.iter().enumerate() {
                if dynamic_mask_has_index(
                    data.available_quest_line_x_quest_ids_update_mask.as_deref(),
                    index,
                ) {
                    buf.write_int32(*value);
                }
            }
        }
        if active_player_mask_has(data, 6) {
            for (index, value) in data.field_1000.iter().enumerate() {
                if dynamic_mask_has_index(data.field_1000_update_mask.as_deref(), index) {
                    buf.write_int32(*value);
                }
            }
        }
        if active_player_mask_has(data, 7) {
            for (index, value) in data.heirlooms.iter().enumerate() {
                if dynamic_mask_has_index(data.heirlooms_update_mask.as_deref(), index) {
                    buf.write_int32(*value);
                }
            }
        }
        if active_player_mask_has(data, 8) {
            for (index, value) in data.heirloom_flags.iter().enumerate() {
                if dynamic_mask_has_index(data.heirloom_flags_update_mask.as_deref(), index) {
                    buf.write_uint32(*value);
                }
            }
        }
        if active_player_mask_has(data, 9) {
            for (index, value) in data.toys.iter().enumerate() {
                if dynamic_mask_has_index(data.toys_update_mask.as_deref(), index) {
                    buf.write_int32(*value);
                }
            }
        }
        if active_player_mask_has(data, 10) {
            for (index, value) in data.transmog.iter().enumerate() {
                if dynamic_mask_has_index(data.transmog_update_mask.as_deref(), index) {
                    buf.write_uint32(*value);
                }
            }
        }
        if active_player_mask_has(data, 11) {
            for (index, value) in data.conditional_transmog.iter().enumerate() {
                if dynamic_mask_has_index(data.conditional_transmog_update_mask.as_deref(), index) {
                    buf.write_int32(*value);
                }
            }
        }
        if active_player_mask_has(data, 12) {
            for (index, value) in data.self_res_spells.iter().enumerate() {
                if dynamic_mask_has_index(data.self_res_spells_update_mask.as_deref(), index) {
                    buf.write_int32(*value);
                }
            }
        }
        if active_player_mask_has(data, 14) {
            for (index, value) in data.spell_pct_mod_by_label.iter().enumerate() {
                if dynamic_mask_has_index(data.spell_pct_mod_by_label_update_mask.as_deref(), index)
                {
                    write_spell_pct_mod_by_label_values_update(buf, *value);
                }
            }
        }
        if active_player_mask_has(data, 15) {
            for (index, value) in data.spell_flat_mod_by_label.iter().enumerate() {
                if dynamic_mask_has_index(
                    data.spell_flat_mod_by_label_update_mask.as_deref(),
                    index,
                ) {
                    write_spell_flat_mod_by_label_values_update(buf, *value);
                }
            }
        }
        if active_player_mask_has(data, 16) {
            for (index, value) in data.task_quests.iter().enumerate() {
                if dynamic_mask_has_index(data.task_quests_update_mask.as_deref(), index) {
                    write_quest_log_values_update(buf, value);
                }
            }
        }
        if active_player_mask_has(data, 18) {
            for (index, value) in data.category_cooldown_mods.iter().enumerate() {
                if dynamic_mask_has_index(data.category_cooldown_mods_update_mask.as_deref(), index)
                {
                    write_category_cooldown_mod_values_update(buf, *value);
                }
            }
        }
        if active_player_mask_has(data, 19) {
            for (index, value) in data.weekly_spell_uses.iter().enumerate() {
                if dynamic_mask_has_index(data.weekly_spell_uses_update_mask.as_deref(), index) {
                    write_weekly_spell_use_values_update(buf, *value);
                }
            }
        }
        if active_player_mask_has(data, 13) {
            for (index, value) in data.character_restrictions.iter().enumerate() {
                if dynamic_mask_has_index(data.character_restrictions_update_mask.as_deref(), index)
                {
                    write_character_restriction_values_update(buf, *value);
                }
            }
        }
        if active_player_mask_has(data, 17) {
            for (index, value) in data.trait_configs.iter().enumerate() {
                if dynamic_mask_has_index(data.trait_configs_update_mask.as_deref(), index) {
                    write_trait_config_values_update(buf, value);
                }
            }
        }
        if active_player_mask_has(data, 26) {
            buf.write_packed_guid(&data.farsight_object);
        }
        if active_player_mask_has(data, 27) {
            buf.write_packed_guid(&data.summoned_battle_pet_guid);
        }
        if active_player_mask_has(data, 28) {
            buf.write_uint64(data.coinage);
        }
        if active_player_mask_has(data, 29) {
            buf.write_int32(data.xp);
        }
        if active_player_mask_has(data, 30) {
            buf.write_int32(data.next_level_xp);
        }
        if active_player_mask_has(data, 31) {
            buf.write_int32(data.trial_xp);
        }
        if active_player_mask_has(data, 32) {
            write_skill_info_values_update(buf, &data.skill);
        }
        if active_player_mask_has(data, 33) {
            buf.write_int32(data.character_points);
        }
        if active_player_mask_has(data, 34) {
            buf.write_int32(data.max_talent_tiers);
        }
        if active_player_mask_has(data, 35) {
            buf.write_uint32(data.track_creature_mask);
        }
        if active_player_mask_has(data, 36) {
            buf.write_float(data.mainhand_expertise);
        }
        if active_player_mask_has(data, 37) {
            buf.write_float(data.offhand_expertise);
        }
    }
    if active_player_mask_has(data, 38) {
        if active_player_mask_has(data, 39) {
            buf.write_float(data.ranged_expertise);
        }
        if active_player_mask_has(data, 40) {
            buf.write_float(data.combat_rating_expertise);
        }
        if active_player_mask_has(data, 41) {
            buf.write_float(data.block_percentage);
        }
        if active_player_mask_has(data, 42) {
            buf.write_float(data.dodge_percentage);
        }
        if active_player_mask_has(data, 43) {
            buf.write_float(data.dodge_percentage_from_attribute);
        }
        if active_player_mask_has(data, 44) {
            buf.write_float(data.parry_percentage);
        }
        if active_player_mask_has(data, 45) {
            buf.write_float(data.parry_percentage_from_attribute);
        }
        if active_player_mask_has(data, 46) {
            buf.write_float(data.crit_percentage);
        }
        if active_player_mask_has(data, 47) {
            buf.write_float(data.ranged_crit_percentage);
        }
        if active_player_mask_has(data, 48) {
            buf.write_float(data.offhand_crit_percentage);
        }
        if active_player_mask_has(data, 49) {
            buf.write_int32(data.shield_block);
        }
        if active_player_mask_has(data, 50) {
            buf.write_float(data.shield_block_crit_percentage);
        }
        if active_player_mask_has(data, 51) {
            buf.write_float(data.mastery);
        }
        if active_player_mask_has(data, 52) {
            buf.write_float(data.speed);
        }
        if active_player_mask_has(data, 53) {
            buf.write_float(data.avoidance);
        }
        if active_player_mask_has(data, 54) {
            buf.write_float(data.sturdiness);
        }
        if active_player_mask_has(data, 55) {
            buf.write_int32(data.versatility);
        }
        if active_player_mask_has(data, 56) {
            buf.write_float(data.versatility_bonus);
        }
        if active_player_mask_has(data, 57) {
            buf.write_float(data.pvp_power_damage);
        }
        if active_player_mask_has(data, 58) {
            buf.write_float(data.pvp_power_healing);
        }
        if active_player_mask_has(data, 59) {
            buf.write_int32(data.mod_healing_done_pos);
        }
        if active_player_mask_has(data, 60) {
            buf.write_float(data.mod_healing_percent);
        }
        if active_player_mask_has(data, 61) {
            buf.write_float(data.mod_healing_done_percent);
        }
        if active_player_mask_has(data, 62) {
            buf.write_float(data.mod_periodic_healing_done_percent);
        }
        if active_player_mask_has(data, 63) {
            buf.write_float(data.mod_spell_power_percent);
        }
        if active_player_mask_has(data, 64) {
            buf.write_float(data.mod_resilience_percent);
        }
        if active_player_mask_has(data, 65) {
            buf.write_float(data.override_spell_power_by_ap_percent);
        }
        if active_player_mask_has(data, 66) {
            buf.write_float(data.override_ap_by_spell_power_percent);
        }
        if active_player_mask_has(data, 67) {
            buf.write_int32(data.mod_target_resistance);
        }
        if active_player_mask_has(data, 68) {
            buf.write_int32(data.mod_target_physical_resistance);
        }
        if active_player_mask_has(data, 69) {
            buf.write_uint32(data.local_flags);
        }
    }
    if active_player_mask_has(data, 70) {
        if active_player_mask_has(data, 71) {
            buf.write_uint8(data.grantable_levels);
        }
        if active_player_mask_has(data, 72) {
            buf.write_uint8(data.multi_action_bars);
        }
        if active_player_mask_has(data, 73) {
            buf.write_uint8(data.lifetime_max_rank);
        }
        if active_player_mask_has(data, 74) {
            buf.write_uint8(data.num_respecs);
        }
        if active_player_mask_has(data, 75) {
            buf.write_int32(data.ammo_id);
        }
        if active_player_mask_has(data, 76) {
            buf.write_uint32(data.pvp_medals);
        }
        if active_player_mask_has(data, 77) {
            buf.write_uint16(data.today_honorable_kills);
        }
        if active_player_mask_has(data, 78) {
            buf.write_uint16(data.today_dishonorable_kills);
        }
        if active_player_mask_has(data, 79) {
            buf.write_uint16(data.yesterday_honorable_kills);
        }
        if active_player_mask_has(data, 80) {
            buf.write_uint16(data.yesterday_dishonorable_kills);
        }
        if active_player_mask_has(data, 81) {
            buf.write_uint16(data.last_week_honorable_kills);
        }
        if active_player_mask_has(data, 82) {
            buf.write_uint16(data.last_week_dishonorable_kills);
        }
        if active_player_mask_has(data, 83) {
            buf.write_uint16(data.this_week_honorable_kills);
        }
        if active_player_mask_has(data, 84) {
            buf.write_uint16(data.this_week_dishonorable_kills);
        }
        if active_player_mask_has(data, 85) {
            buf.write_uint32(data.this_week_contribution);
        }
        if active_player_mask_has(data, 86) {
            buf.write_uint32(data.lifetime_honorable_kills);
        }
        if active_player_mask_has(data, 87) {
            buf.write_uint32(data.lifetime_dishonorable_kills);
        }
        if active_player_mask_has(data, 88) {
            buf.write_uint32(data.field_f24);
        }
        if active_player_mask_has(data, 89) {
            buf.write_uint32(data.yesterday_contribution);
        }
        if active_player_mask_has(data, 90) {
            buf.write_uint32(data.last_week_contribution);
        }
        if active_player_mask_has(data, 91) {
            buf.write_uint32(data.last_week_rank);
        }
        if active_player_mask_has(data, 92) {
            buf.write_int32(data.watched_faction_index);
        }
        if active_player_mask_has(data, 93) {
            buf.write_int32(data.max_level);
        }
        if active_player_mask_has(data, 94) {
            buf.write_int32(data.scaling_player_level_delta);
        }
        if active_player_mask_has(data, 95) {
            buf.write_int32(data.max_creature_scaling_level);
        }
        if active_player_mask_has(data, 96) {
            buf.write_int32(data.pet_spell_power);
        }
        if active_player_mask_has(data, 97) {
            buf.write_float(data.ui_hit_modifier);
        }
        if active_player_mask_has(data, 98) {
            buf.write_float(data.ui_spell_hit_modifier);
        }
        if active_player_mask_has(data, 99) {
            buf.write_int32(data.home_realm_time_offset);
        }
        if active_player_mask_has(data, 100) {
            buf.write_float(data.mod_pet_haste);
        }
        if active_player_mask_has(data, 101) {
            buf.write_uint8(data.local_regen_flags);
        }
    }
    if active_player_mask_has(data, 102) {
        if active_player_mask_has(data, 103) {
            buf.write_uint8(data.aura_vision);
        }
        if active_player_mask_has(data, 104) {
            buf.write_uint8(data.num_backpack_slots);
        }
        if active_player_mask_has(data, 105) {
            buf.write_int32(data.override_spells_id);
        }
        if active_player_mask_has(data, 106) {
            buf.write_int32(data.lfg_bonus_faction_id);
        }
        if active_player_mask_has(data, 107) {
            buf.write_uint16(data.loot_spec_id);
        }
        if active_player_mask_has(data, 108) {
            buf.write_uint32(data.override_zone_pvp_type);
        }
        if active_player_mask_has(data, 109) {
            buf.write_int32(data.honor);
        }
        if active_player_mask_has(data, 110) {
            buf.write_int32(data.honor_next_level);
        }
        if active_player_mask_has(data, 111) {
            buf.write_int32(data.field_f74);
        }
        if active_player_mask_has(data, 112) {
            buf.write_int32(data.pvp_tier_max_from_wins);
        }
        if active_player_mask_has(data, 113) {
            buf.write_int32(data.pvp_last_weeks_tier_max_from_wins);
        }
        if active_player_mask_has(data, 114) {
            buf.write_uint8(data.pvp_rank_progress);
        }
        if active_player_mask_has(data, 115) {
            buf.write_int32(data.perks_program_currency);
        }
        if active_player_mask_has(data, 118) {
            buf.write_int32(data.transport_server_time);
        }
        if active_player_mask_has(data, 119) {
            buf.write_uint32(data.active_combat_trait_config_id);
        }
        if active_player_mask_has(data, 120) {
            buf.write_uint8(data.glyphs_enabled);
        }
        if active_player_mask_has(data, 121) {
            buf.write_uint8(data.lfg_roles);
        }
        if active_player_mask_has(data, 123) {
            buf.write_uint8(data.num_stable_slots);
        }
    }
    buf.flush_bits();
    if active_player_mask_has(data, 102) {
        buf.write_bits(data.pet_stable.is_some() as u32, 1);
        if active_player_mask_has(data, 116) {
            write_research_history_values_update(buf, &data.research_history);
        }
        if active_player_mask_has(data, 117) {
            write_perks_vendor_item_values_update(buf, data.frozen_perks_vendor_item);
        }
        if active_player_mask_has(data, 122) {
            if let Some(pet_stable) = &data.pet_stable {
                write_stable_info_values_update(buf, pet_stable);
            }
        }
    }
    if active_player_mask_has(data, 124) {
        for index in 0..141 {
            if active_player_mask_has(data, 125 + index) {
                buf.write_packed_guid(&data.inv_slots[index]);
            }
        }
    }
    if active_player_mask_has(data, 266) {
        for index in 0..2 {
            if active_player_mask_has(data, 267 + index) {
                buf.write_uint32(data.track_resource_mask[index]);
            }
        }
    }
    if active_player_mask_has(data, 269) {
        for index in 0..7 {
            if active_player_mask_has(data, 270 + index) {
                buf.write_float(data.spell_crit_percentage[index]);
            }
            if active_player_mask_has(data, 277 + index) {
                buf.write_int32(data.mod_damage_done_pos[index]);
            }
            if active_player_mask_has(data, 284 + index) {
                buf.write_int32(data.mod_damage_done_neg[index]);
            }
            if active_player_mask_has(data, 291 + index) {
                buf.write_float(data.mod_damage_done_percent[index]);
            }
        }
    }
    if active_player_mask_has(data, 298) {
        for index in 0..240 {
            if active_player_mask_has(data, 299 + index) {
                buf.write_uint64(data.explored_zones[index]);
            }
        }
    }
    if active_player_mask_has(data, 539) {
        for index in 0..2 {
            if active_player_mask_has(data, 540 + index) {
                write_rest_info_values_update(buf, data.rest_info[index]);
            }
        }
    }
    if active_player_mask_has(data, 542) {
        for index in 0..3 {
            if active_player_mask_has(data, 543 + index) {
                buf.write_float(data.weapon_dmg_multipliers[index]);
            }
            if active_player_mask_has(data, 546 + index) {
                buf.write_float(data.weapon_atk_speed_multipliers[index]);
            }
        }
    }
    if active_player_mask_has(data, 549) {
        for index in 0..12 {
            if active_player_mask_has(data, 550 + index) {
                buf.write_uint32(data.buyback_price[index]);
            }
            if active_player_mask_has(data, 562 + index) {
                buf.write_int64(data.buyback_timestamp[index]);
            }
        }
    }
    if active_player_mask_has(data, 574) {
        for index in 0..32 {
            if active_player_mask_has(data, 575 + index) {
                buf.write_int32(data.combat_ratings[index]);
            }
        }
    }
    if active_player_mask_has(data, 615) {
        for index in 0..4 {
            if active_player_mask_has(data, 616 + index) {
                buf.write_uint32(data.no_reagent_cost_mask[index]);
            }
        }
    }
    if active_player_mask_has(data, 620) {
        for index in 0..2 {
            if active_player_mask_has(data, 621 + index) {
                buf.write_int32(data.profession_skill_line[index]);
            }
        }
    }
    if active_player_mask_has(data, 623) {
        for index in 0..4 {
            if active_player_mask_has(data, 624 + index) {
                buf.write_uint32(data.bag_slot_flags[index]);
            }
        }
    }
    if active_player_mask_has(data, 628) {
        for index in 0..7 {
            if active_player_mask_has(data, 629 + index) {
                buf.write_uint32(data.bank_bag_slot_flags[index]);
            }
        }
    }
    if active_player_mask_has(data, 636) {
        for index in 0..875 {
            if active_player_mask_has(data, 637 + index) {
                buf.write_uint64(data.quest_completed[index]);
            }
        }
    }
    if active_player_mask_has(data, 1512) {
        for index in 0..6 {
            if active_player_mask_has(data, 1513 + index) {
                buf.write_uint32(data.glyph_slots[index]);
            }
            if active_player_mask_has(data, 1519 + index) {
                buf.write_uint32(data.glyphs[index]);
            }
        }
    }
    if active_player_mask_has(data, 607) {
        for index in 0..7 {
            if active_player_mask_has(data, 608 + index) {
                write_pvp_info_values_update(buf, data.pvp_info[index]);
            }
        }
    }
    buf.flush_bits();
}
