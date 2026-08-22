// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! ActivePlayer and Player update blocks.

use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QuestLogValuesUpdate {
    pub quest_log_mask: u32,
    pub end_time: i64,
    pub quest_id: i32,
    pub state_flags: u32,
    pub objective_progress: [u16; 24],
}

impl Default for QuestLogValuesUpdate {
    fn default() -> Self {
        Self {
            quest_log_mask: 0,
            end_time: 0,
            quest_id: 0,
            state_flags: 0,
            objective_progress: [0; 24],
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SkillInfoValuesUpdate {
    pub skill_info_mask: [u32; 57],
    pub skill_line_id: [u16; 256],
    pub skill_step: [u16; 256],
    pub skill_rank: [u16; 256],
    pub skill_starting_rank: [u16; 256],
    pub skill_max_rank: [u16; 256],
    pub skill_temp_bonus: [i16; 256],
    pub skill_perm_bonus: [u16; 256],
}

impl Default for SkillInfoValuesUpdate {
    fn default() -> Self {
        Self {
            skill_info_mask: [0; 57],
            skill_line_id: [0; 256],
            skill_step: [0; 256],
            skill_rank: [0; 256],
            skill_starting_rank: [0; 256],
            skill_max_rank: [0; 256],
            skill_temp_bonus: [0; 256],
            skill_perm_bonus: [0; 256],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RestInfoValuesUpdate {
    pub rest_info_mask: u8,
    pub threshold: u32,
    pub state_id: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PvpInfoValuesUpdate {
    pub pvp_info_mask: u32,
    pub disqualified: bool,
    pub bracket: i8,
    pub pvp_rating_id: i32,
    pub weekly_played: u32,
    pub weekly_won: u32,
    pub season_played: u32,
    pub season_won: u32,
    pub rating: u32,
    pub weekly_best_rating: u32,
    pub season_best_rating: u32,
    pub pvp_tier_id: u32,
    pub weekly_best_win_pvp_tier_id: u32,
    pub field_28: u32,
    pub field_2c: u32,
    pub weekly_rounds_played: u32,
    pub weekly_rounds_won: u32,
    pub season_rounds_played: u32,
    pub season_rounds_won: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CharacterRestrictionValuesUpdate {
    pub field_0: i32,
    pub field_4: i32,
    pub field_8: i32,
    pub restriction_type: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TraitEntryValuesUpdate {
    pub trait_node_id: i32,
    pub trait_node_entry_id: i32,
    pub rank: i32,
    pub granted_ranks: i32,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct TraitConfigValuesUpdate {
    pub trait_config_mask: u16,
    pub entries: Vec<TraitEntryValuesUpdate>,
    pub entries_update_mask: Option<Vec<u32>>,
    pub id: i32,
    pub name: String,
    pub config_type: i32,
    pub skill_line_id: i32,
    pub chr_specialization_id: i32,
    pub combat_config_flags: i32,
    pub local_identifier: i32,
    pub trait_system_id: i32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ActivePlayerDataValuesUpdate {
    pub active_player_data_mask: [u32; 48],
    pub sort_bags_right_to_left: bool,
    pub insert_items_left_to_right: bool,
    pub research_sites: Vec<u16>,
    pub research_sites_update_mask: Option<Vec<u32>>,
    pub research_site_progress: Vec<u32>,
    pub research_site_progress_update_mask: Option<Vec<u32>>,
    pub research: Vec<ResearchValuesUpdate>,
    pub research_update_mask: Option<Vec<u32>>,
    pub known_titles: Vec<u64>,
    pub known_titles_update_mask: Option<Vec<u32>>,
    pub daily_quests_completed: Vec<i32>,
    pub daily_quests_completed_update_mask: Option<Vec<u32>>,
    pub available_quest_line_x_quest_ids: Vec<i32>,
    pub available_quest_line_x_quest_ids_update_mask: Option<Vec<u32>>,
    pub field_1000: Vec<i32>,
    pub field_1000_update_mask: Option<Vec<u32>>,
    pub heirlooms: Vec<i32>,
    pub heirlooms_update_mask: Option<Vec<u32>>,
    pub heirloom_flags: Vec<u32>,
    pub heirloom_flags_update_mask: Option<Vec<u32>>,
    pub toys: Vec<i32>,
    pub toys_update_mask: Option<Vec<u32>>,
    pub transmog: Vec<u32>,
    pub transmog_update_mask: Option<Vec<u32>>,
    pub conditional_transmog: Vec<i32>,
    pub conditional_transmog_update_mask: Option<Vec<u32>>,
    pub self_res_spells: Vec<i32>,
    pub self_res_spells_update_mask: Option<Vec<u32>>,
    pub spell_pct_mod_by_label: Vec<SpellPctModByLabelValuesUpdate>,
    pub spell_pct_mod_by_label_update_mask: Option<Vec<u32>>,
    pub spell_flat_mod_by_label: Vec<SpellFlatModByLabelValuesUpdate>,
    pub spell_flat_mod_by_label_update_mask: Option<Vec<u32>>,
    pub task_quests: Vec<QuestLogValuesUpdate>,
    pub task_quests_update_mask: Option<Vec<u32>>,
    pub category_cooldown_mods: Vec<CategoryCooldownModValuesUpdate>,
    pub category_cooldown_mods_update_mask: Option<Vec<u32>>,
    pub weekly_spell_uses: Vec<WeeklySpellUseValuesUpdate>,
    pub weekly_spell_uses_update_mask: Option<Vec<u32>>,
    pub character_restrictions: Vec<CharacterRestrictionValuesUpdate>,
    pub character_restrictions_update_mask: Option<Vec<u32>>,
    pub trait_configs: Vec<TraitConfigValuesUpdate>,
    pub trait_configs_update_mask: Option<Vec<u32>>,
    pub farsight_object: ObjectGuid,
    pub summoned_battle_pet_guid: ObjectGuid,
    pub coinage: u64,
    pub xp: i32,
    pub next_level_xp: i32,
    pub trial_xp: i32,
    pub skill: SkillInfoValuesUpdate,
    pub character_points: i32,
    pub max_talent_tiers: i32,
    pub track_creature_mask: u32,
    pub mainhand_expertise: f32,
    pub offhand_expertise: f32,
    pub ranged_expertise: f32,
    pub combat_rating_expertise: f32,
    pub block_percentage: f32,
    pub dodge_percentage: f32,
    pub dodge_percentage_from_attribute: f32,
    pub parry_percentage: f32,
    pub parry_percentage_from_attribute: f32,
    pub crit_percentage: f32,
    pub ranged_crit_percentage: f32,
    pub offhand_crit_percentage: f32,
    pub shield_block: i32,
    pub shield_block_crit_percentage: f32,
    pub mastery: f32,
    pub speed: f32,
    pub avoidance: f32,
    pub sturdiness: f32,
    pub versatility: i32,
    pub versatility_bonus: f32,
    pub pvp_power_damage: f32,
    pub pvp_power_healing: f32,
    pub mod_healing_done_pos: i32,
    pub mod_healing_percent: f32,
    pub mod_healing_done_percent: f32,
    pub mod_periodic_healing_done_percent: f32,
    pub mod_spell_power_percent: f32,
    pub mod_resilience_percent: f32,
    pub override_spell_power_by_ap_percent: f32,
    pub override_ap_by_spell_power_percent: f32,
    pub mod_target_resistance: i32,
    pub mod_target_physical_resistance: i32,
    pub local_flags: u32,
    pub grantable_levels: u8,
    pub multi_action_bars: u8,
    pub lifetime_max_rank: u8,
    pub num_respecs: u8,
    pub ammo_id: i32,
    pub pvp_medals: u32,
    pub today_honorable_kills: u16,
    pub today_dishonorable_kills: u16,
    pub yesterday_honorable_kills: u16,
    pub yesterday_dishonorable_kills: u16,
    pub last_week_honorable_kills: u16,
    pub last_week_dishonorable_kills: u16,
    pub this_week_honorable_kills: u16,
    pub this_week_dishonorable_kills: u16,
    pub this_week_contribution: u32,
    pub lifetime_honorable_kills: u32,
    pub lifetime_dishonorable_kills: u32,
    pub field_f24: u32,
    pub yesterday_contribution: u32,
    pub last_week_contribution: u32,
    pub last_week_rank: u32,
    pub watched_faction_index: i32,
    pub max_level: i32,
    pub scaling_player_level_delta: i32,
    pub max_creature_scaling_level: i32,
    pub pet_spell_power: i32,
    pub ui_hit_modifier: f32,
    pub ui_spell_hit_modifier: f32,
    pub home_realm_time_offset: i32,
    pub mod_pet_haste: f32,
    pub local_regen_flags: u8,
    pub aura_vision: u8,
    pub num_backpack_slots: u8,
    pub override_spells_id: i32,
    pub lfg_bonus_faction_id: i32,
    pub loot_spec_id: u16,
    pub override_zone_pvp_type: u32,
    pub honor: i32,
    pub honor_next_level: i32,
    pub field_f74: i32,
    pub pvp_tier_max_from_wins: i32,
    pub pvp_last_weeks_tier_max_from_wins: i32,
    pub pvp_rank_progress: u8,
    pub perks_program_currency: i32,
    pub research_history: ResearchHistoryValuesUpdate,
    pub frozen_perks_vendor_item: PerksVendorItemValuesUpdate,
    pub transport_server_time: i32,
    pub active_combat_trait_config_id: u32,
    pub glyphs_enabled: u8,
    pub lfg_roles: u8,
    pub pet_stable: Option<StableInfoValuesUpdate>,
    pub num_stable_slots: u8,
    pub inv_slots: [ObjectGuid; 141],
    pub track_resource_mask: [u32; 2],
    pub spell_crit_percentage: [f32; 7],
    pub mod_damage_done_pos: [i32; 7],
    pub mod_damage_done_neg: [i32; 7],
    pub mod_damage_done_percent: [f32; 7],
    pub explored_zones: [u64; 240],
    pub rest_info: [RestInfoValuesUpdate; 2],
    pub weapon_dmg_multipliers: [f32; 3],
    pub weapon_atk_speed_multipliers: [f32; 3],
    pub buyback_price: [u32; 12],
    pub buyback_timestamp: [i64; 12],
    pub combat_ratings: [i32; 32],
    pub pvp_info: [PvpInfoValuesUpdate; 7],
    pub no_reagent_cost_mask: [u32; 4],
    pub profession_skill_line: [i32; 2],
    pub bag_slot_flags: [u32; 4],
    pub bank_bag_slot_flags: [u32; 7],
    pub quest_completed: [u64; 875],
    pub glyph_slots: [u32; 6],
    pub glyphs: [u32; 6],
}

impl Default for ActivePlayerDataValuesUpdate {
    fn default() -> Self {
        Self {
            active_player_data_mask: [0; 48],
            sort_bags_right_to_left: false,
            insert_items_left_to_right: false,
            research_sites: Vec::new(),
            research_sites_update_mask: None,
            research_site_progress: Vec::new(),
            research_site_progress_update_mask: None,
            research: Vec::new(),
            research_update_mask: None,
            known_titles: Vec::new(),
            known_titles_update_mask: None,
            daily_quests_completed: Vec::new(),
            daily_quests_completed_update_mask: None,
            available_quest_line_x_quest_ids: Vec::new(),
            available_quest_line_x_quest_ids_update_mask: None,
            field_1000: Vec::new(),
            field_1000_update_mask: None,
            heirlooms: Vec::new(),
            heirlooms_update_mask: None,
            heirloom_flags: Vec::new(),
            heirloom_flags_update_mask: None,
            toys: Vec::new(),
            toys_update_mask: None,
            transmog: Vec::new(),
            transmog_update_mask: None,
            conditional_transmog: Vec::new(),
            conditional_transmog_update_mask: None,
            self_res_spells: Vec::new(),
            self_res_spells_update_mask: None,
            spell_pct_mod_by_label: Vec::new(),
            spell_pct_mod_by_label_update_mask: None,
            spell_flat_mod_by_label: Vec::new(),
            spell_flat_mod_by_label_update_mask: None,
            task_quests: Vec::new(),
            task_quests_update_mask: None,
            category_cooldown_mods: Vec::new(),
            category_cooldown_mods_update_mask: None,
            weekly_spell_uses: Vec::new(),
            weekly_spell_uses_update_mask: None,
            character_restrictions: Vec::new(),
            character_restrictions_update_mask: None,
            trait_configs: Vec::new(),
            trait_configs_update_mask: None,
            farsight_object: ObjectGuid::EMPTY,
            summoned_battle_pet_guid: ObjectGuid::EMPTY,
            coinage: 0,
            xp: 0,
            next_level_xp: 0,
            trial_xp: 0,
            skill: SkillInfoValuesUpdate::default(),
            character_points: 0,
            max_talent_tiers: 0,
            track_creature_mask: 0,
            mainhand_expertise: 0.0,
            offhand_expertise: 0.0,
            ranged_expertise: 0.0,
            combat_rating_expertise: 0.0,
            block_percentage: 0.0,
            dodge_percentage: 0.0,
            dodge_percentage_from_attribute: 0.0,
            parry_percentage: 0.0,
            parry_percentage_from_attribute: 0.0,
            crit_percentage: 0.0,
            ranged_crit_percentage: 0.0,
            offhand_crit_percentage: 0.0,
            shield_block: 0,
            shield_block_crit_percentage: 0.0,
            mastery: 0.0,
            speed: 0.0,
            avoidance: 0.0,
            sturdiness: 0.0,
            versatility: 0,
            versatility_bonus: 0.0,
            pvp_power_damage: 0.0,
            pvp_power_healing: 0.0,
            mod_healing_done_pos: 0,
            mod_healing_percent: 0.0,
            mod_healing_done_percent: 0.0,
            mod_periodic_healing_done_percent: 0.0,
            mod_spell_power_percent: 0.0,
            mod_resilience_percent: 0.0,
            override_spell_power_by_ap_percent: 0.0,
            override_ap_by_spell_power_percent: 0.0,
            mod_target_resistance: 0,
            mod_target_physical_resistance: 0,
            local_flags: 0,
            grantable_levels: 0,
            multi_action_bars: 0,
            lifetime_max_rank: 0,
            num_respecs: 0,
            ammo_id: 0,
            pvp_medals: 0,
            today_honorable_kills: 0,
            today_dishonorable_kills: 0,
            yesterday_honorable_kills: 0,
            yesterday_dishonorable_kills: 0,
            last_week_honorable_kills: 0,
            last_week_dishonorable_kills: 0,
            this_week_honorable_kills: 0,
            this_week_dishonorable_kills: 0,
            this_week_contribution: 0,
            lifetime_honorable_kills: 0,
            lifetime_dishonorable_kills: 0,
            field_f24: 0,
            yesterday_contribution: 0,
            last_week_contribution: 0,
            last_week_rank: 0,
            watched_faction_index: 0,
            max_level: 0,
            scaling_player_level_delta: 0,
            max_creature_scaling_level: 0,
            pet_spell_power: 0,
            ui_hit_modifier: 0.0,
            ui_spell_hit_modifier: 0.0,
            home_realm_time_offset: 0,
            mod_pet_haste: 0.0,
            local_regen_flags: 0,
            aura_vision: 0,
            num_backpack_slots: 0,
            override_spells_id: 0,
            lfg_bonus_faction_id: 0,
            loot_spec_id: 0,
            override_zone_pvp_type: 0,
            honor: 0,
            honor_next_level: 0,
            field_f74: 0,
            pvp_tier_max_from_wins: 0,
            pvp_last_weeks_tier_max_from_wins: 0,
            pvp_rank_progress: 0,
            perks_program_currency: 0,
            research_history: ResearchHistoryValuesUpdate::default(),
            frozen_perks_vendor_item: PerksVendorItemValuesUpdate::default(),
            transport_server_time: 0,
            active_combat_trait_config_id: 0,
            glyphs_enabled: 0,
            lfg_roles: 0,
            pet_stable: None,
            num_stable_slots: 0,
            inv_slots: [ObjectGuid::EMPTY; 141],
            track_resource_mask: [0; 2],
            spell_crit_percentage: [0.0; 7],
            mod_damage_done_pos: [0; 7],
            mod_damage_done_neg: [0; 7],
            mod_damage_done_percent: [0.0; 7],
            explored_zones: [0; 240],
            rest_info: [RestInfoValuesUpdate::default(); 2],
            weapon_dmg_multipliers: [0.0; 3],
            weapon_atk_speed_multipliers: [0.0; 3],
            buyback_price: [0; 12],
            buyback_timestamp: [0; 12],
            combat_ratings: [0; 32],
            pvp_info: [PvpInfoValuesUpdate::default(); 7],
            no_reagent_cost_mask: [0; 4],
            profession_skill_line: [0; 2],
            bag_slot_flags: [0; 4],
            bank_bag_slot_flags: [0; 7],
            quest_completed: [0; 875],
            glyph_slots: [0; 6],
            glyphs: [0; 6],
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PlayerDataValuesDeltaUpdate {
    pub changed_object_type_mask: u32,
    pub object_data: Option<ObjectDataValuesUpdate>,
    pub unit_data: Option<UnitDataValuesDeltaUpdate>,
    pub active_player_data: Option<ActivePlayerDataValuesUpdate>,
    pub player_data_mask: [u32; 4],
    pub customizations: Vec<ChrCustomizationChoiceValuesUpdate>,
    pub customizations_update_mask: Option<Vec<u32>>,
    pub arena_cooldowns: Vec<ArenaCooldownValuesUpdate>,
    pub arena_cooldowns_update_mask: Option<Vec<u32>>,
    pub visual_item_replacements: Vec<i32>,
    pub visual_item_replacements_update_mask: Option<Vec<u32>>,
    pub duel_arbiter: ObjectGuid,
    pub wow_account: ObjectGuid,
    pub loot_target_guid: ObjectGuid,
    pub player_flags: u32,
    pub player_flags_ex: u32,
    pub guild_rank_id: u32,
    pub guild_delete_date: u32,
    pub guild_level: i32,
    pub num_bank_slots: u8,
    pub native_sex: u8,
    pub inebriation: u8,
    pub pvp_title: u8,
    pub arena_faction: u8,
    pub pvp_rank: u8,
    pub field_88: i32,
    pub duel_team: u32,
    pub guild_time_stamp: i32,
    pub player_title: i32,
    pub fake_inebriation: i32,
    pub virtual_player_realm: u32,
    pub current_spec_id: u32,
    pub taxi_mount_anim_kit_id: i32,
    pub current_battle_pet_breed_quality: u8,
    pub honor_level: i32,
    pub logout_time: i64,
    pub current_battle_pet_species_id: i32,
    pub bnet_account: ObjectGuid,
    pub dungeon_score: DungeonScoreSummaryValuesUpdate,
    pub party_type: [u8; 2],
    pub quest_log: [QuestLogValuesUpdate; 25],
    pub visible_items: [VisibleItemValuesUpdate; 19],
    pub avg_item_level: [f32; 6],
    pub field_3120: [u32; 19],
}

impl Default for PlayerDataValuesDeltaUpdate {
    fn default() -> Self {
        Self {
            changed_object_type_mask: VALUES_TYPE_PLAYER,
            object_data: None,
            unit_data: None,
            active_player_data: None,
            player_data_mask: [0; 4],
            customizations: Vec::new(),
            customizations_update_mask: None,
            arena_cooldowns: Vec::new(),
            arena_cooldowns_update_mask: None,
            visual_item_replacements: Vec::new(),
            visual_item_replacements_update_mask: None,
            duel_arbiter: ObjectGuid::EMPTY,
            wow_account: ObjectGuid::EMPTY,
            loot_target_guid: ObjectGuid::EMPTY,
            player_flags: 0,
            player_flags_ex: 0,
            guild_rank_id: 0,
            guild_delete_date: 0,
            guild_level: 0,
            num_bank_slots: 0,
            native_sex: 0,
            inebriation: 0,
            pvp_title: 0,
            arena_faction: 0,
            pvp_rank: 0,
            field_88: 0,
            duel_team: 0,
            guild_time_stamp: 0,
            player_title: 0,
            fake_inebriation: 0,
            virtual_player_realm: 0,
            current_spec_id: 0,
            taxi_mount_anim_kit_id: 0,
            current_battle_pet_breed_quality: 0,
            honor_level: 0,
            logout_time: 0,
            current_battle_pet_species_id: 0,
            bnet_account: ObjectGuid::EMPTY,
            dungeon_score: DungeonScoreSummaryValuesUpdate::default(),
            party_type: [0; 2],
            quest_log: [QuestLogValuesUpdate::default(); 25],
            visible_items: [VisibleItemValuesUpdate::default(); 19],
            avg_item_level: [0.0; 6],
            field_3120: [0; 19],
        }
    }
}

/// Stat values for a VALUES update after equip/desequip.
///
/// Contains all UnitData fields that change when gear changes,
/// used by `UpdateObject::player_stat_update` to send a partial
/// VALUES update without recreating the whole player object.
#[derive(Debug, Clone, Copy)]
pub struct PlayerStatChanges {
    pub health: i64,
    pub max_health: i64,
    pub min_damage: f32,
    pub max_damage: f32,
    pub base_mana: i32,
    pub base_health: i32,
    pub attack_power: i32,
    pub attack_power_mod_pos: i32,
    pub attack_power_mod_neg: i32,
    pub attack_power_multiplier: f32,
    pub ranged_attack_power: i32,
    pub ranged_attack_power_mod_pos: i32,
    pub ranged_attack_power_mod_neg: i32,
    pub ranged_attack_power_multiplier: f32,
    pub min_ranged_damage: f32,
    pub max_ranged_damage: f32,
    pub power0: i32,             // Mana/Rage/Energy current
    pub max_power0: i32,         // Mana/Rage/Energy max
    pub stats: [i32; 5],         // STR, AGI, STA, INT, SPI
    pub stat_pos_buff: [i32; 5], // gear bonuses shown as positive buffs
    pub stat_neg_buff: [i32; 5], // negative item/aura stat modifiers
    pub armor: i32,              // Resistances[0] = Physical
    // ActivePlayerData secondary stats
    pub combat_ratings: [i32; 32], // CombatRatings[32] (indices per CombatRating enum, 0-24 used)
    pub spell_power: i32,          // ModDamageDonePos for magic schools 1-6
    // Percentage fields (server-computed, displayed by client)
    pub block_pct: f32,           // BlockPercentage (bit 41)
    pub dodge_pct: f32,           // DodgePercentage (bit 42)
    pub parry_pct: f32,           // ParryPercentage (bit 44)
    pub crit_pct: f32,            // CritPercentage (bit 46) — melee
    pub ranged_crit_pct: f32,     // RangedCritPercentage (bit 47)
    pub spell_crit_pct: [f32; 7], // SpellCritPercentage[7] (bits 270-276)
    // UnitData: mana regen (parent 116 interleaved loop)
    pub mana_regen: f32,        // PowerRegenFlatModifier[0] (bit 117)
    pub mana_regen_combat: f32, // PowerRegenInterruptedFlatModifier[0] (bit 127)
    pub mana_regen_mp5: f32,    // ModPowerRegen[0] (bit 157)
    // ActivePlayerData parent 0: expertise (bits 36-37)
    pub mainhand_expertise: f32, // MainhandExpertise (bit 36)
    pub offhand_expertise: f32,  // OffhandExpertise (bit 37)
    // ActivePlayerData parent 38: extended fields (bits 39-69)
    pub ranged_expertise: f32,         // bit 39
    pub combat_rating_expertise: f32,  // bit 40
    pub dodge_from_attr: f32,          // bit 43
    pub parry_from_attr: f32,          // bit 45
    pub offhand_crit_pct: f32,         // bit 48
    pub shield_block: i32,             // bit 49
    pub shield_block_crit_pct: f32,    // bit 50
    pub mod_healing_pct: f32,          // bit 60 (1.0)
    pub mod_healing_done_pct: f32,     // bit 61 (1.0)
    pub mod_periodic_healing_pct: f32, // bit 62 (1.0)
    pub mod_spell_power_pct: f32,      // bit 63 (1.0)
}

impl Default for PlayerStatChanges {
    fn default() -> Self {
        Self {
            health: 0,
            max_health: 0,
            min_damage: 0.0,
            max_damage: 0.0,
            base_mana: 0,
            base_health: 0,
            attack_power: 0,
            attack_power_mod_pos: 0,
            attack_power_mod_neg: 0,
            attack_power_multiplier: 0.0,
            ranged_attack_power: 0,
            ranged_attack_power_mod_pos: 0,
            ranged_attack_power_mod_neg: 0,
            ranged_attack_power_multiplier: 0.0,
            min_ranged_damage: 0.0,
            max_ranged_damage: 0.0,
            power0: 0,
            max_power0: 0,
            stats: [0; 5],
            stat_pos_buff: [0; 5],
            stat_neg_buff: [0; 5],
            armor: 0,
            combat_ratings: [0; 32],
            spell_power: 0,
            block_pct: 0.0,
            dodge_pct: 0.0,
            parry_pct: 0.0,
            crit_pct: 0.0,
            ranged_crit_pct: 0.0,
            spell_crit_pct: [0.0; 7],
            mana_regen: 0.0,
            mana_regen_combat: 0.0,
            mana_regen_mp5: 0.0,
            mainhand_expertise: 0.0,
            offhand_expertise: 0.0,
            ranged_expertise: 0.0,
            combat_rating_expertise: 0.0,
            dodge_from_attr: 0.0,
            parry_from_attr: 0.0,
            offhand_crit_pct: 0.0,
            shield_block: 0,
            shield_block_crit_pct: 0.0,
            mod_healing_pct: 1.0,
            mod_healing_done_pct: 1.0,
            mod_periodic_healing_pct: 1.0,
            mod_spell_power_pct: 1.0,
        }
    }
}

// ── PlayerCombatStats ──────────────────────────────────────────────

/// All combat-related stats computed from base stats + gear.
///
/// Passed as a single struct to `create_player` to avoid 20+ parameters.
#[derive(Debug, Clone, Copy)]
pub struct PlayerCombatStats {
    pub health: i64,
    pub max_health: i64,
    pub stats: [i32; 5],
    pub stat_pos_buff: [i32; 5],
    pub stat_neg_buff: [i32; 5],
    pub base_armor: i32,
    pub base_mana: i32,
    pub max_mana: i64,
    pub attack_power: i32,
    pub attack_power_mod_pos: i32,
    pub ranged_attack_power: i32,
    pub ranged_attack_power_mod_pos: i32,
    pub min_damage: f32,
    pub max_damage: f32,
    pub min_ranged_damage: f32,
    pub max_ranged_damage: f32,
    pub block_pct: f32,
    pub dodge_pct: f32,
    pub dodge_from_attr: f32,
    pub parry_pct: f32,
    pub parry_from_attr: f32,
    pub crit_pct: f32,
    pub ranged_crit_pct: f32,
    pub offhand_crit_pct: f32,
    pub spell_crit_pct: [f32; 7],
    pub combat_ratings: [i32; 32],
    pub spell_power: i32,
}

impl Default for PlayerCombatStats {
    fn default() -> Self {
        Self {
            health: 100,
            max_health: 100,
            stats: [0; 5],
            stat_pos_buff: [0; 5],
            stat_neg_buff: [0; 5],
            base_armor: 0,
            base_mana: 0,
            max_mana: 60,
            attack_power: 0,
            attack_power_mod_pos: 0,
            ranged_attack_power: 0,
            ranged_attack_power_mod_pos: 0,
            min_damage: 1.0,
            max_damage: 2.0,
            min_ranged_damage: 0.0,
            max_ranged_damage: 0.0,
            block_pct: 0.0,
            dodge_pct: 0.0,
            dodge_from_attr: 0.0,
            parry_pct: 0.0,
            parry_from_attr: 0.0,
            crit_pct: 5.0,
            ranged_crit_pct: 5.0,
            offhand_crit_pct: 5.0,
            spell_crit_pct: [5.0; 7],
            combat_ratings: [0; 32],
            spell_power: 0,
        }
    }
}

// ── PlayerCreateData ────────────────────────────────────────────────

/// Data needed to build a full player create packet for the client.
pub struct PlayerCreateData {
    pub guid: ObjectGuid,
    /// PlayerData::WowAccount.
    pub wow_account: ObjectGuid,
    /// PlayerData::BnetAccount.
    pub bnet_account: ObjectGuid,
    pub race: u8,
    pub class: u8,
    pub sex: u8,
    pub level: u8,
    pub display_id: u32,
    pub native_display_id: u32,
    pub health: i64,
    pub max_health: i64,
    pub faction_template: i32,
    pub current_area_id: u32,
    /// PlayerData::PlayerFlags.
    pub player_flags: u32,
    /// PlayerData::PlayerFlagsEx.
    pub player_flags_ex: u32,
    /// Primary stats: [STR, AGI, STA, INT, SPI].
    pub stats: [i32; 5],
    pub stat_pos_buff: [i32; 5],
    pub stat_neg_buff: [i32; 5],
    /// Base armor (AGI * 2).
    pub base_armor: i32,
    /// C++ `UnitData::BaseMana` / `Player::GetCreateMana`.
    pub base_mana: i32,
    /// Max mana from level stats (for caster classes).
    pub max_mana: i64,
    /// Current primary power stored in `UnitData::Power[0]`.
    pub current_power0: i32,
    /// Melee attack power.
    pub attack_power: i32,
    pub attack_power_mod_pos: i32,
    /// Ranged attack power.
    pub ranged_attack_power: i32,
    pub ranged_attack_power_mod_pos: i32,
    /// Melee min/max damage (unarmed base).
    pub min_damage: f32,
    pub max_damage: f32,
    /// Ranged min/max damage.
    pub min_ranged_damage: f32,
    pub max_ranged_damage: f32,
    pub block_pct: f32,
    /// Dodge percentage.
    pub dodge_pct: f32,
    pub dodge_from_attr: f32,
    /// Parry percentage.
    pub parry_pct: f32,
    pub parry_from_attr: f32,
    /// Melee crit percentage.
    pub crit_pct: f32,
    /// Ranged crit percentage.
    pub ranged_crit_pct: f32,
    pub offhand_crit_pct: f32,
    /// Spell crit percentage by school.
    pub spell_crit_pct: [f32; 7],
    pub combat_ratings: [i32; 32],
    pub spell_power: i32,
    /// Visible equipment items (19 slots).
    /// Each entry: (ItemID, AppearanceModID, ItemVisual).
    /// Slots: Head(0), Neck(1), Shoulders(2), Shirt(3), Chest(4), Waist(5),
    /// Legs(6), Feet(7), Wrist(8), Hands(9), Finger1(10), Finger2(11),
    /// Trinket1(12), Trinket2(13), Cloak(14), MainHand(15), OffHand(16),
    /// Ranged(17), Tabard(18).
    pub visible_items: [(i32, u16, u16); 19],
    /// PlayerData::Customizations dynamic field.
    pub customizations: Vec<ChrCustomizationChoiceValuesUpdate>,
    /// Inventory slots (141 entries) for ActivePlayerData.
    /// Slots 0-18 = equipped, 19-22 = bag containers, rest = backpack/bank.
    /// Each entry is an Item ObjectGuid (or EMPTY).
    pub inv_slots: [ObjectGuid; 141],
    /// ActivePlayerData::FarsightObject written after InvSlots in WriteCreate.
    pub farsight_object: ObjectGuid,
    /// C++ `Player::m_actionButtons` written by `Object::BuildMovementUpdate`
    /// when `CreateObjectBits::ActivePlayer` is set for the self create block.
    pub action_buttons: [u32; MAX_ACTION_BUTTONS],
    /// Character's learned skills for the SkillInfo array (up to 256).
    /// Each entry: (skill_id, step, rank, starting_rank, max_rank, temp_bonus, perm_bonus).
    pub skill_info: Vec<(u16, u16, u16, u16, u16, i16, u16)>,
    /// Quest log slots — up to 25 active quests.
    /// (quest_id, state_flags, end_time, objective_progress[24])
    /// C++ ref: `UF::PlayerData::WriteCreate` only emits `QuestLog`
    /// when `UpdateFieldFlag::PartyMember` is present. For self-view,
    /// `Player::BuildValuesCreate` uses Owner|PartyMember.
    pub quest_log: Vec<(u32, u32, i64, [u16; 24])>,
    /// PlayerData::PartyType[2], indexed by C++ GroupCategory.
    pub party_type: [u8; 2],
    /// Current money in copper (Coinage field in ActivePlayerData).
    pub coinage: u64,
    /// ActivePlayerData::XP.
    pub xp: i32,
    /// ActivePlayerData::NextLevelXP.
    pub next_level_xp: i32,
    /// ActivePlayerData::MaxLevel.
    pub max_level: i32,
    /// ActivePlayerData::ScalingPlayerLevelDelta.
    pub scaling_player_level_delta: i32,
    /// ActivePlayerData::RestInfo[REST_TYPE_XP/HONOR].
    pub rest_info: [RestInfoValuesUpdate; 2],
    /// ActivePlayerData::WatchedFactionIndex.
    pub watched_faction_index: i32,
    /// ActivePlayerData::Heirlooms.
    pub heirlooms: Vec<i32>,
    /// ActivePlayerData::HeirloomFlags.
    pub heirloom_flags: Vec<u32>,
    /// ActivePlayerData::Toys.
    pub toys: Vec<i32>,
    /// ActivePlayerData::Transmog dynamic field blocks.
    ///
    /// C++ `CollectionMgr::LoadAccountItemAppearances` expands account
    /// appearance masks into `Player::m_activePlayerData->Transmog` before
    /// `ActivePlayerData::WriteCreate`.
    pub transmog: Vec<u32>,
    /// ActivePlayerData::TraitConfigs.
    pub trait_configs: Vec<TraitConfigCreateData>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraitEntryCreateData {
    pub trait_node_id: i32,
    pub trait_node_entry_id: i32,
    pub rank: i32,
    pub granted_ranks: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraitConfigCreateData {
    pub id: i32,
    pub config_type: i32,
    pub skill_line_id: i32,
    pub chr_specialization_id: i32,
    pub combat_config_flags: i32,
    pub local_identifier: i32,
    pub trait_system_id: i32,
    pub name: String,
    pub entries: Vec<TraitEntryCreateData>,
}

impl PlayerCreateData {
    /// Get the faction template for a race.
    pub fn faction_for_race(race: u8) -> i32 {
        match race {
            1 => 1,     // Human
            2 => 2,     // Orc
            3 => 3,     // Dwarf
            4 => 4,     // NightElf
            5 => 5,     // Undead
            6 => 6,     // Tauren
            7 => 115,   // Gnome
            8 => 116,   // Troll
            10 => 1610, // BloodElf
            11 => 1629, // Draenei
            22 => 1,    // Worgen → Human faction
            _ => 1,
        }
    }

    /// Get the max power value for slot 0, using real mana for caster classes.
    ///
    /// - Warrior (1): rage = 1000 (stored as 10×)
    /// - Rogue (4): energy = 100
    /// - DK (6): runic power = 1000 (stored as 10×)
    /// - All others: mana from C++ `GtBaseMP`
    pub(super) fn max_power_for_slot0(&self) -> i32 {
        match self.class {
            1 => 1000,                 // Warrior: rage
            4 => 100,                  // Rogue: energy
            6 => 1000,                 // DK: runic power
            _ => self.max_mana as i32, // Casters: real mana from DB
        }
    }

    pub(super) fn current_power_for_slot0(&self) -> i32 {
        self.current_power0
            .clamp(0, self.max_power_for_slot0().max(0))
    }

    pub(super) fn base_mana_for_create_like_cpp(&self) -> i32 {
        if power_type_for_class(self.class) == 0 {
            self.base_mana.max(0)
        } else {
            0
        }
    }

    /// Write the complete values block for CREATE (no change masks).
    ///
    /// Format: `[u32 size][u8 flags][ObjectData][UnitData][PlayerData][ActivePlayerData?]`
    pub fn write_values_create(&self, pkt: &mut WorldPacket, is_self: bool) {
        // Build into a temp buffer so we can prefix with size
        let mut buf = WorldPacket::new_empty();

        // C++ refs:
        // - `Player::BuildValuesCreate` writes TypeId sections in Object, Unit,
        //   Player, ActivePlayer order.
        // - `WorldObject::GetUpdateFieldFlagsFor` returns Owner|PartyMember
        //   for the self receiver, which enables self-only PlayerData fields.
        let flags: u8 = if is_self { 0x03 } else { 0x00 }; // 0x01=Owner 0x02=PartyMember
        buf.write_uint8(flags);

        let object_start = buf.data().len();
        self.write_object_data(&mut buf);
        let unit_start = buf.data().len();
        self.write_unit_data(&mut buf, flags);
        let player_start = buf.data().len();
        self.write_player_data(&mut buf, flags);
        let active_start = buf.data().len();
        if is_self {
            self.write_active_player_data(&mut buf);
        }
        let end_pos = buf.data().len();

        if std::env::var_os("RUSTYCORE_UPDATEOBJECT_TRACE").is_some() {
            eprintln!(
                "RUST_UPDATEOBJECT player_values guid={:?} self={} flags=0x{:X} totalValues={} object={} unit={} player={} active={}",
                self.guid,
                is_self,
                flags,
                end_pos,
                unit_start - object_start,
                player_start - unit_start,
                active_start - player_start,
                end_pos - active_start
            );
        }

        let data = buf.into_data();
        pkt.write_uint32(data.len() as u32); // Size prefix
        pkt.write_bytes(&data);
    }

    // ── ObjectFieldData.WriteCreate ─────────────────────────────

    fn write_object_data(&self, buf: &mut WorldPacket) {
        buf.write_int32(0); // EntryId (0 for players)
        buf.write_uint32(0); // DynamicFlags
        buf.write_float(1.0); // Scale
    }

    // ── UnitData.WriteCreate ────────────────────────────────────

    pub(super) fn write_unit_data(&self, buf: &mut WorldPacket, flags: u8) {
        let is_owner = flags & 0x01 != 0;

        // Health / MaxHealth
        buf.write_int64(self.health);
        buf.write_int64(self.max_health);

        // DisplayId
        buf.write_int32(self.display_id as i32);

        // NpcFlags[2]
        buf.write_uint32(0);
        buf.write_uint32(0);

        // StateSpellVisualID, StateAnimID, StateAnimKitID.
        // C++ Player::Player (Player.cpp:22134) ALSO seeds StateAnimID with
        // DB2Manager::GetEmptyAnimStateID() = 1772 (DB2Stores.cpp:1765) — the Classic
        // client expects the retail AnimationData storage size for EVERY unit, including
        // the player itself. Shipping 0 makes the client index its AnimationData storage
        // out of range -> NULL deref in the render/anim worker (~4-5s in-world, ERROR #132)
        // when the player's own model animates. This is independent of nearby creatures.
        const EMPTY_ANIM_STATE_ID_LIKE_CPP: i32 = 1772;
        buf.write_int32(0);
        buf.write_int32(EMPTY_ANIM_STATE_ID_LIKE_CPP);
        buf.write_int32(0);

        // StateWorldEffectIDs.Count (dynamic array size = 0)
        buf.write_int32(0);

        // 10 PackedGuids: Charm, Summon, [Critter if Owner], CharmedBy,
        // SummonedBy, CreatedBy, DemonCreator, LookAtControllerTarget,
        // Target, BattlePetCompanionGUID
        write_empty_guid(buf); // Charm
        write_empty_guid(buf); // Summon
        if is_owner {
            write_empty_guid(buf); // Critter (only if Owner)
        }
        write_empty_guid(buf); // CharmedBy
        write_empty_guid(buf); // SummonedBy
        write_empty_guid(buf); // CreatedBy
        write_empty_guid(buf); // DemonCreator
        write_empty_guid(buf); // LookAtControllerTarget
        write_empty_guid(buf); // Target
        write_empty_guid(buf); // BattlePetCompanionGUID

        // BattlePetDBID
        buf.write_uint64(0);

        // ChannelData (UnitChannel.WriteCreate): SpellID + SpellXSpellVisualID
        buf.write_int32(0);
        buf.write_int32(0);

        // SummonedByHomeRealm
        buf.write_uint32(0);

        // Race, ClassId, PlayerClassId, Sex, DisplayPower
        buf.write_uint8(self.race);
        buf.write_uint8(self.class);
        buf.write_uint8(self.class); // PlayerClassId = same as ClassId for players
        buf.write_uint8(self.sex);
        buf.write_uint8(power_type_for_class(self.class)); // DisplayPower

        // OverrideDisplayPowerID
        buf.write_int32(0);

        // PowerRegen + PowerRegenInterrupted (Owner|UnitAll only)
        if is_owner {
            for _ in 0..10 {
                buf.write_float(0.0); // PowerRegenFlatModifier
                buf.write_float(0.0); // PowerRegenInterruptedFlatModifier
            }
        }

        // Power[10], MaxPower[10], ModPowerRegen[10]
        let current_power0 = self.current_power_for_slot0();
        let max_power0 = self.max_power_for_slot0();
        for i in 0..10 {
            if i == 0 {
                buf.write_int32(current_power0);
                buf.write_int32(max_power0);
            } else {
                buf.write_int32(0);
                buf.write_int32(0);
            }
            buf.write_float(0.0); // ModPowerRegen
        }

        // Level, EffectiveLevel, ContentTuningID, Scaling fields (9x i32)
        buf.write_int32(self.level as i32);
        buf.write_int32(self.level as i32); // EffectiveLevel
        buf.write_int32(0); // ContentTuningID
        buf.write_int32(0); // ScalingLevelMin
        buf.write_int32(0); // ScalingLevelMax
        buf.write_int32(0); // ScalingLevelDelta
        buf.write_int32(0); // ScalingFactionGroup
        buf.write_int32(0); // ScalingHealthItemLevelCurveID
        buf.write_int32(0); // ScalingDamageItemLevelCurveID

        // FactionTemplate
        buf.write_int32(self.faction_template);

        // VirtualItems[3] — weapons visible on character model
        // [0]=MainHand(slot 15), [1]=OffHand(slot 16), [2]=Ranged(slot 17)
        for &slot in &[15usize, 16, 17] {
            let (item_id, appearance_mod, item_visual) = self.visible_items[slot];
            buf.write_int32(item_id);
            buf.write_uint16(appearance_mod);
            buf.write_uint16(item_visual);
        }

        // Flags, Flags2, Flags3, AuraState
        buf.write_uint32(0x0000_0008); // UnitFlags: UNIT_FLAG_PLAYER_CONTROLLED
        buf.write_uint32(0); // Flags2
        buf.write_uint32(0); // Flags3
        // AuraState — C++ Unit::Update/ModifyAuraState applies health-based aura states to
        // EVERY alive unit incl. the player (Unit.cpp:469-476); full HP => 0x00D00000.
        buf.write_uint32(health_aura_state_like_cpp(
            self.health,
            self.max_health,
            self.health > 0,
        )); // AuraState

        // AttackRoundBaseTime[2]
        buf.write_uint32(2000); // MainHand
        buf.write_uint32(2000); // OffHand

        // RangedAttackRoundBaseTime (Owner only)
        if is_owner {
            buf.write_uint32(0);
        }

        // BoundingRadius, CombatReach, DisplayScale
        // C++ DEFAULT_PLAYER_BOUNDING_RADIUS = 0.388999998569489 (ObjectDefines.h:39),
        // set via Player::SetObjectScale -> SetBoundingRadius(scale * DEFAULT) (scale=1.0 here).
        buf.write_float(0.388_999_998_569_489); // BoundingRadius
        buf.write_float(1.5); // CombatReach
        buf.write_float(1.0); // DisplayScale

        // NativeDisplayID, NativeXDisplayScale, MountDisplayID
        buf.write_int32(self.native_display_id as i32);
        buf.write_float(1.0); // NativeXDisplayScale
        buf.write_int32(0); // MountDisplayID

        // MinDamage, MaxDamage, MinOffHandDamage, MaxOffHandDamage (Owner|Empath)
        if is_owner {
            buf.write_float(self.min_damage);
            buf.write_float(self.max_damage);
            buf.write_float(0.0); // MinOffHandDamage
            buf.write_float(0.0); // MaxOffHandDamage
        }

        // StandState, PetTalentPoints, VisFlags, AnimTier
        buf.write_uint8(0); // StandState (UNIT_STAND_STATE_STAND)
        buf.write_uint8(0); // PetTalentPoints
        buf.write_uint8(0); // VisFlags
        buf.write_uint8(0); // AnimTier

        // PetNumber, PetNameTimestamp, PetExperience, PetNextLevelExperience
        buf.write_int32(0);
        buf.write_int32(0);
        buf.write_int32(0);
        buf.write_int32(0);

        // ModCastingSpeed, ModSpellHaste, ModHaste, ModRangedHaste,
        // ModHasteRegen, ModTimeRate.
        // C++ 3.4.3 `UnitData::WriteCreate` writes exactly these six floats
        // before CreatedBySpell (`UpdateFields.cpp:750-756`).
        buf.write_float(1.0);
        buf.write_float(1.0);
        buf.write_float(1.0);
        buf.write_float(1.0);
        buf.write_float(1.0);
        buf.write_float(1.0);

        // CreatedBySpell, EmoteState
        buf.write_int32(0);
        buf.write_int32(0);

        // TrainingPointsUsed, TrainingPointsTotal (2x i16)
        buf.write_int16(0);
        buf.write_int16(0);

        // Stats[5], StatPosBuff[5], StatNegBuff[5] (Owner only)
        if is_owner {
            for i in 0..5 {
                buf.write_int32(self.stats[i]); // Stat
                buf.write_int32(self.stat_pos_buff[i]); // StatPosBuff
                buf.write_int32(self.stat_neg_buff[i]); // StatNegBuff
            }
        }

        // Resistances[7] (Owner|Empath): Physical, Holy, Fire, Nature, Frost, Shadow, Arcane
        if is_owner {
            buf.write_int32(self.base_armor); // [0] Physical = base armor
            for _ in 1..7 {
                buf.write_int32(0); // [1-6] spell resistances
            }
        }

        // PowerCostModifier[7], PowerCostMultiplier[7] (Owner only)
        if is_owner {
            for _ in 0..7 {
                buf.write_int32(0); // PowerCostModifier
                buf.write_float(1.0); // PowerCostMultiplier
            }
        }

        // ResistanceBuffModsPositive[7], ResistanceBuffModsNegative[7]
        for _ in 0..7 {
            buf.write_int32(0); // Positive
            buf.write_int32(0); // Negative
        }

        // BaseMana — C++ GtBaseMP create mana for caster classes.
        buf.write_int32(self.base_mana_for_create_like_cpp());

        // C++ Player::InitStatsForLevel sets CreateHealth/BaseHealth to zero.
        if is_owner {
            buf.write_int32(0);
        }

        // SheatheState, PvpFlags, PetFlags, ShapeshiftForm
        buf.write_uint8(0); // SheatheState
        buf.write_uint8(0); // PvpFlags
        buf.write_uint8(0); // PetFlags
        buf.write_uint8(0); // ShapeshiftForm

        // AttackPower block (Owner only — 13 fields)
        if is_owner {
            buf.write_int32(self.attack_power); // AttackPower
            buf.write_int32(self.attack_power_mod_pos); // AttackPowerModPos
            buf.write_int32(0); // AttackPowerModNeg
            buf.write_float(0.0); // AttackPowerMultiplier
            buf.write_int32(self.ranged_attack_power); // RangedAttackPower
            buf.write_int32(self.ranged_attack_power_mod_pos); // RangedAttackPowerModPos
            buf.write_int32(0); // RangedAttackPowerModNeg
            buf.write_float(0.0); // RangedAttackPowerMultiplier
            buf.write_int32(0); // SetAttackSpeedAura
            buf.write_float(0.0); // Lifesteal
            buf.write_float(self.min_ranged_damage); // MinRangedDamage
            buf.write_float(self.max_ranged_damage); // MaxRangedDamage
            buf.write_float(1.0); // MaxHealthModifier
        }

        // HoverHeight + misc fields
        buf.write_float(1.0); // HoverHeight
        buf.write_int32(0); // MinItemLevelCutoff
        buf.write_int32(0); // MinItemLevel
        buf.write_int32(0); // MaxItemLevel
        buf.write_int32(0); // WildBattlePetLevel
        buf.write_int32(0); // BattlePetCompanionNameTimestamp
        buf.write_int32(0); // InteractSpellId
        buf.write_int32(0); // ScaleDuration
        buf.write_int32(0); // LooksLikeMountID
        buf.write_int32(0); // LooksLikeCreatureID
        buf.write_int32(0); // LookAtControllerID
        buf.write_int32(0); // PerksVendorItemID
        write_empty_guid(buf); // GuildGUID

        // Dynamic array sizes: PassiveSpells, WorldEffects, ChannelObjects
        buf.write_int32(0);
        buf.write_int32(0);
        buf.write_int32(0);

        write_empty_guid(buf); // SkinningOwnerGUID

        // FlightCapabilityID, GlideEventSpeedDivisor, CurrentAreaID
        buf.write_int32(0);
        buf.write_float(0.0);
        buf.write_uint32(self.current_area_id);

        // ComboTarget (Owner only)
        if is_owner {
            write_empty_guid(buf);
        }

        // Dynamic arrays (all empty — sizes were 0 above)
    }

    // ── PlayerData.WriteCreate ──────────────────────────────────

    pub(super) fn write_player_data(&self, buf: &mut WorldPacket, flags: u8) {
        let is_party = flags & 0x02 != 0; // UpdateFieldFlag::PartyMember = 0x02

        // 3 PackedGuids
        write_empty_guid(buf); // DuelArbiter
        buf.write_packed_guid(&self.wow_account); // WowAccount
        write_empty_guid(buf); // LootTargetGUID

        // PlayerFlags, PlayerFlagsEx
        buf.write_uint32(self.player_flags);
        buf.write_uint32(self.player_flags_ex);

        // GuildRankID, GuildDeleteDate, GuildLevel
        buf.write_int32(0);
        buf.write_uint32(0);
        buf.write_int32(0);

        // Customizations.Size
        buf.write_uint32(self.customizations.len() as u32);

        // PartyType[2]
        buf.write_uint8(self.party_type[0]);
        buf.write_uint8(self.party_type[1]);

        // NumBankSlots, NativeSex, Inebriation, PvpTitle, ArenaFaction, PvpRank
        buf.write_uint8(0);
        buf.write_uint8(self.sex);
        buf.write_uint8(0);
        buf.write_uint8(0);
        buf.write_uint8(0);
        buf.write_uint8(0);

        // Field_88, DuelTeam, GuildTimeStamp
        buf.write_int32(0);
        buf.write_uint32(0);
        buf.write_int32(0);

        // QuestLog[25] — written when PartyMember flag is set.
        // For self-view, C++ `WorldObject::GetUpdateFieldFlagsFor` includes
        // `UpdateFieldFlag::PartyMember`.
        // C++ `UF::QuestLog::WriteCreate`: int64 EndTime + int32 QuestID
        // + uint32 StateFlags + uint16[24] ObjectiveProgress.
        if is_party {
            // Fill 25 slots; empty slots get quest_id=0
            let empty_slot: (u32, u32, i64, [u16; 24]) = (0, 0, 0, [0u16; 24]);
            for i in 0..25usize {
                let (quest_id, state_flags, end_time, obj_progress) =
                    self.quest_log.get(i).copied().unwrap_or(empty_slot);
                buf.write_int64(end_time); // EndTime (int64)
                buf.write_int32(quest_id as i32); // QuestID (int32)
                buf.write_uint32(state_flags); // StateFlags (uint32)
                for progress in &obj_progress {
                    // ObjectiveProgress[24] (uint16 each)
                    buf.write_uint16(*progress);
                }
            }
        }

        // VisibleItems[19] (each: i32 ItemID + u16 AppearanceModID + u16 ItemVisual)
        for &(item_id, appearance_mod, item_visual) in &self.visible_items {
            buf.write_int32(item_id);
            buf.write_uint16(appearance_mod);
            buf.write_uint16(item_visual);
        }

        // PlayerTitle, FakeInebriation, VirtualPlayerRealm, CurrentSpecID, TaxiMountAnimKitID
        buf.write_int32(0);
        buf.write_int32(0);
        buf.write_uint32(0);
        buf.write_int32(0);
        buf.write_int32(0);

        // AvgItemLevel[6]
        for _ in 0..6 {
            buf.write_float(0.0);
        }

        // CurrentBattlePetBreedQuality
        buf.write_uint8(0);

        // HonorLevel
        buf.write_int32(0);

        // LogoutTime
        buf.write_int64(0);

        // ArenaCooldowns.Size, CurrentBattlePetSpeciesID
        buf.write_int32(0);
        buf.write_int32(0);

        // BnetAccount
        buf.write_packed_guid(&self.bnet_account);

        // VisualItemReplacements.Size
        buf.write_int32(0);

        // Field_3120[19]
        for _ in 0..19 {
            buf.write_uint32(0);
        }

        for customization in &self.customizations {
            write_chr_customization_choice_values_update(buf, customization);
        }

        // Dynamic arrays (empty — ArenaCooldowns, VisualItemReplacements)

        // DungeonScoreSummary.Write:
        //   OverallScoreCurrentSeason(f32), LadderScoreCurrentSeason(f32), Runs.Count(i32)
        buf.write_float(0.0);
        buf.write_float(0.0);
        buf.write_int32(0);
    }

    // ── ActivePlayerData.WriteCreate ────────────────────────────

    pub(super) fn write_active_player_data(&self, buf: &mut WorldPacket) {
        let trace_sections = std::env::var_os("RUSTYCORE_UPDATEOBJECT_TRACE").is_some();
        let active_base = buf.data().len();
        let trace = |buf: &WorldPacket, label: &str| {
            if trace_sections {
                eprintln!(
                    "RUST_UPDATEOBJECT active_section {label} offset={} size={}",
                    buf.data().len() - active_base,
                    buf.data().len()
                );
            }
        };

        // InvSlots[141]
        for i in 0..141 {
            buf.write_packed_guid(&self.inv_slots[i]);
        }
        trace(buf, "inv_slots");

        // FarsightObject, SummonedBattlePetGUID
        buf.write_packed_guid(&self.farsight_object);
        write_empty_guid(buf);
        trace(buf, "farsight_battlepet");

        // KnownTitles.Size
        buf.write_uint32(0);

        // Coinage, XP, NextLevelXP, TrialXP
        buf.write_int64(self.coinage as i64);
        buf.write_int32(self.xp);
        buf.write_int32(self.next_level_xp);
        buf.write_int32(0);

        // SkillInfo.WriteCreate: 256 entries × 7 u16s each
        for i in 0..256 {
            if i < self.skill_info.len() {
                let (id, step, rank, start, max, temp, perm) = self.skill_info[i];
                buf.write_uint16(id); // SkillLineID
                buf.write_uint16(step); // SkillStep
                buf.write_uint16(rank); // SkillRank
                buf.write_uint16(start); // SkillStartingRank
                buf.write_uint16(max); // SkillMaxRank
                buf.write_int16(temp); // SkillTempBonus
                buf.write_uint16(perm); // SkillPermBonus
            } else {
                buf.write_uint16(0);
                buf.write_uint16(0);
                buf.write_uint16(0);
                buf.write_uint16(0);
                buf.write_uint16(0);
                buf.write_int16(0);
                buf.write_uint16(0);
            }
        }
        trace(buf, "skill");

        // CharacterPoints, MaxTalentTiers
        buf.write_int32(0);
        buf.write_int32(0);

        // TrackCreatureMask
        buf.write_uint32(0);

        // TrackResourceMask[2]
        buf.write_uint32(0);
        buf.write_uint32(0);

        // Expertise floats: Mainhand, Offhand, Ranged, CombatRating
        buf.write_float(0.0);
        buf.write_float(0.0);
        buf.write_float(0.0);
        buf.write_float(0.0);

        // Block, Dodge, DodgeFromAttr, Parry, ParryFromAttr, Crit, RangedCrit, OffhandCrit
        buf.write_float(self.block_pct); // Block
        buf.write_float(self.dodge_pct); // Dodge
        buf.write_float(self.dodge_from_attr); // DodgeFromAttr
        buf.write_float(self.parry_pct); // Parry
        buf.write_float(self.parry_from_attr); // ParryFromAttr
        buf.write_float(self.crit_pct); // CritPercentage
        buf.write_float(self.ranged_crit_pct); // RangedCritPercentage
        buf.write_float(self.offhand_crit_pct); // OffhandCritPercentage

        // SpellCritPercentage[7], ModDamageDonePos[7], ModDamageDoneNeg[7], ModDamageDonePercent[7]
        for school in 0..7 {
            buf.write_float(self.spell_crit_pct[school]); // SpellCritPercentage per school
            buf.write_int32(if school == 0 { 0 } else { self.spell_power }); // ModDamageDonePos
            buf.write_int32(0); // ModDamageDoneNeg
            buf.write_float(1.0); // ModDamageDonePercent
        }

        // ShieldBlock, ShieldBlockCritPercentage
        buf.write_int32(0);
        buf.write_float(0.0);

        // Mastery, Speed, Avoidance, Sturdiness
        buf.write_float(0.0);
        buf.write_float(0.0);
        buf.write_float(0.0);
        buf.write_float(0.0);

        // Versatility, VersatilityBonus
        buf.write_int32(0);
        buf.write_float(0.0);

        // PvpPowerDamage, PvpPowerHealing
        buf.write_float(0.0);
        buf.write_float(0.0);

        // ExploredZones[240] (all zero u64s)
        for _ in 0..240 {
            buf.write_uint64(0);
        }
        trace(buf, "explored_zones");

        // RestInfo[2] (each: i32 Threshold + u8 StateID)
        // StateID: 1=Rested, 2=Normal, 6=RAFLinked — must NOT be 0 (invalid)
        for rest_info in self.rest_info {
            buf.write_int32(rest_info.threshold as i32);
            buf.write_uint8(rest_info.state_id);
        }

        // ModHealingDonePos, ModHealingPercent, ModHealingDonePercent, ModPeriodicHealingDonePercent
        buf.write_int32(self.spell_power);
        buf.write_float(1.0);
        buf.write_float(1.0);
        buf.write_float(1.0);

        // WeaponDmgMultipliers[3], WeaponAtkSpeedMultipliers[3]
        for _ in 0..3 {
            buf.write_float(1.0); // WeaponDmgMultipliers
            buf.write_float(1.0); // WeaponAtkSpeedMultipliers
        }

        // ModSpellPowerPercent, ModResiliencePercent
        buf.write_float(1.0);
        buf.write_float(0.0);

        // OverrideSpellPowerByAPPercent, OverrideAPBySpellPowerPercent
        buf.write_float(-1.0);
        buf.write_float(-1.0);

        // ModTargetResistance, ModTargetPhysicalResistance
        buf.write_int32(0);
        buf.write_int32(0);

        // LocalFlags
        buf.write_uint32(0);

        // GrantableLevels, MultiActionBars, LifetimeMaxRank, NumRespecs
        buf.write_uint8(0);
        buf.write_uint8(0);
        buf.write_uint8(0);
        buf.write_uint8(0);

        // AmmoID, PvpMedals
        buf.write_int32(0);
        buf.write_uint32(0);

        // BuybackPrice[12] + BuybackTimestamp[12]
        for _ in 0..12 {
            buf.write_uint32(0); // BuybackPrice
            buf.write_int64(0); // BuybackTimestamp
        }
        trace(buf, "buyback");

        // HonorableKills/DishonorableKills (8x u16)
        buf.write_uint16(0); // TodayHonorableKills
        buf.write_uint16(0); // TodayDishonorableKills
        buf.write_uint16(0); // YesterdayHonorableKills
        buf.write_uint16(0); // YesterdayDishonorableKills
        buf.write_uint16(0); // LastWeekHonorableKills
        buf.write_uint16(0); // LastWeekDishonorableKills
        buf.write_uint16(0); // ThisWeekHonorableKills
        buf.write_uint16(0); // ThisWeekDishonorableKills

        // ThisWeekContribution, LifetimeHonorableKills, LifetimeDishonorableKills
        buf.write_uint32(0);
        buf.write_int32(0);
        buf.write_int32(0);

        // Field_F24, YesterdayContribution, LastWeekContribution, LastWeekRank
        buf.write_uint32(0);
        buf.write_uint32(0);
        buf.write_uint32(0);
        buf.write_uint32(0);

        // WatchedFactionIndex
        buf.write_int32(self.watched_faction_index);

        // CombatRatings[32]
        for rating in self.combat_ratings {
            buf.write_int32(rating);
        }
        trace(buf, "combat_ratings");

        // MaxLevel, ScalingPlayerLevelDelta, MaxCreatureScalingLevel
        buf.write_int32(self.max_level);
        buf.write_int32(self.scaling_player_level_delta);
        buf.write_int32(0);

        // NoReagentCostMask[4]
        for _ in 0..4 {
            buf.write_uint32(0);
        }

        // PetSpellPower
        buf.write_int32(0);

        // ProfessionSkillLine[2]
        buf.write_int32(0);
        buf.write_int32(0);

        // UiHitModifier, UiSpellHitModifier
        buf.write_float(0.0);
        buf.write_float(0.0);

        // HomeRealmTimeOffset
        buf.write_int32(0);

        // ModPetHaste
        buf.write_float(1.0);

        // LocalRegenFlags, AuraVision, NumBackpackSlots
        buf.write_uint8(0);
        buf.write_uint8(0);
        buf.write_uint8(16); // 16 default backpack slots

        // OverrideSpellsID, LfgBonusFactionID
        buf.write_int32(0);
        buf.write_int32(0);

        // LootSpecID
        buf.write_uint16(0);

        // OverrideZonePVPType
        buf.write_uint32(0);

        // BagSlotFlags[4]
        for _ in 0..4 {
            buf.write_uint32(0);
        }

        // BankBagSlotFlags[7]
        for _ in 0..7 {
            buf.write_uint32(0);
        }

        // QuestCompleted[875] (all zero u64s)
        for _ in 0..875 {
            buf.write_uint64(0);
        }
        trace(buf, "quest_completed");

        // Honor, HonorNextLevel, Field_F74, PvpTierMaxFromWins, PvpLastWeeksTierMaxFromWins
        buf.write_int32(0);
        buf.write_int32(0);
        buf.write_int32(0);
        buf.write_int32(0);
        buf.write_int32(0);

        // PvpRankProgress
        buf.write_uint8(0);

        // PerksProgramCurrency
        buf.write_int32(0);

        // ResearchSites loop (1 iteration): 3 sizes (all 0) + no dynamic data
        buf.write_int32(0); // ResearchSites[0].Size()
        buf.write_int32(0); // ResearchSiteProgress[0].Size()
        buf.write_int32(0); // Research[0].Size()

        // DailyQuestsCompleted.Size, AvailableQuestLineXQuestIDs.Size, Field_1000.Size
        buf.write_int32(0);
        buf.write_int32(0);
        buf.write_int32(0);

        // Heirlooms.Size, HeirloomFlags.Size, Toys.Size, Transmog.Size
        buf.write_int32(self.heirlooms.len() as i32);
        buf.write_int32(self.heirloom_flags.len() as i32);
        buf.write_int32(self.toys.len() as i32);
        buf.write_int32(self.transmog.len() as i32);

        // ConditionalTransmog.Size, SelfResSpells.Size, CharacterRestrictions.Size
        buf.write_int32(0);
        buf.write_int32(0);
        buf.write_int32(0);

        // SpellPctModByLabel.Size, SpellFlatModByLabel.Size, TaskQuests.Size
        buf.write_int32(0);
        buf.write_int32(0);
        buf.write_int32(0);

        // TransportServerTime
        buf.write_uint32(0);

        // TraitConfigs.Size
        buf.write_int32(self.trait_configs.len() as i32);

        // ActiveCombatTraitConfigID
        buf.write_int32(0);

        // GlyphSlots[6] + Glyphs[6]
        for _ in 0..6 {
            buf.write_int32(0); // GlyphSlots
            buf.write_int32(0); // Glyphs
        }

        // GlyphsEnabled, LfgRoles
        buf.write_uint8(0);
        buf.write_uint8(0);

        // CategoryCooldownMods.Size, WeeklySpellUses.Size
        buf.write_int32(0);
        buf.write_int32(0);

        // NumStableSlots
        buf.write_uint8(0);
        trace(buf, "dynamic_sizes");

        for value in &self.heirlooms {
            buf.write_int32(*value);
        }
        for value in &self.heirloom_flags {
            buf.write_uint32(*value);
        }
        for value in &self.toys {
            buf.write_int32(*value);
        }
        for value in &self.transmog {
            buf.write_uint32(*value);
        }
        trace(buf, "dynamic_payloads");

        // Remaining dynamic arrays are empty (KnownTitles, DailyQuests, etc.).

        // PvpInfo[7].WriteCreate (each: i8 Bracket + 16 i32/u32 fields + bit Disqualified)
        for _ in 0..7 {
            buf.write_int8(0); // Bracket
            buf.write_int32(0); // PvpRatingID
            buf.write_int32(0); // WeeklyPlayed
            buf.write_int32(0); // WeeklyWon
            buf.write_int32(0); // SeasonPlayed
            buf.write_int32(0); // SeasonWon
            buf.write_int32(0); // Rating
            buf.write_int32(0); // WeeklyBestRating
            buf.write_int32(0); // SeasonBestRating
            buf.write_int32(0); // PvpTierID
            buf.write_int32(0); // WeeklyBestWinPvpTierID
            buf.write_uint32(0); // Field_28
            buf.write_uint32(0); // Field_2C
            buf.write_int32(0); // WeeklyRoundsPlayed
            buf.write_int32(0); // WeeklyRoundsWon
            buf.write_int32(0); // SeasonRoundsPlayed
            buf.write_int32(0); // SeasonRoundsWon
            buf.write_bit(false); // Disqualified
            buf.flush_bits();
        }
        trace(buf, "pvp_info");

        // Trailing bits + FlushBits
        buf.flush_bits();

        // SortBagsRightToLeft, InsertItemsLeftToRight, PetStable has value
        buf.write_bit(false);
        buf.write_bit(false);
        buf.write_bits(0, 1); // PetStable.HasValue = false
        buf.flush_bits();

        // ResearchHistory.WriteCreate: CompletedProjects.Size (i32)
        buf.write_int32(0);
        trace(buf, "research_history");

        // FrozenPerksVendorItem.Write: 8 i32 + 1 i64 + 1 bit
        buf.write_int32(0); // VendorItemID
        buf.write_int32(0); // MountID
        buf.write_int32(0); // BattlePetSpeciesID
        buf.write_int32(0); // TransmogSetID
        buf.write_int32(0); // ItemModifiedAppearanceID
        buf.write_int32(0); // Field_14
        buf.write_int32(0); // Field_18
        buf.write_int32(0); // Price
        buf.write_int64(0); // AvailableUntil
        buf.write_bit(false); // Disabled
        buf.flush_bits();
        trace(buf, "frozen_perks");

        // CharacterRestrictions (size 0, no data)
        for trait_config in &self.trait_configs {
            write_trait_config_create_data(buf, trait_config);
        }
        // PetStable (not present)

        buf.flush_bits();
        trace(buf, "end");
    }
}

fn write_trait_config_create_data(buf: &mut WorldPacket, data: &TraitConfigCreateData) {
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

// ── Helpers ─────────────────────────────────────────────────────────

pub(super) fn debug_player_create_values_len_like_cpp(
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
pub(super) const MAX_ACTION_BUTTONS: usize = 180;

pub(super) fn write_active_player_movement_block(
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
pub(super) fn write_player_values_update_block(
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

pub(super) const VALUES_TYPE_PLAYER: u32 = 1 << 6;

pub(super) const VALUES_TYPE_ACTIVE_PLAYER: u32 = 1 << 7;

fn player_mask_has(data: &PlayerDataValuesDeltaUpdate, bit: usize) -> bool {
    let block = bit / 32;
    let offset = bit % 32;
    data.player_data_mask.get(block).copied().unwrap_or(0) & (1 << offset) != 0
}

fn write_quest_log_values_update(buf: &mut WorldPacket, data: &QuestLogValuesUpdate) {
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

fn write_quest_log_values_create(buf: &mut WorldPacket, data: &QuestLogValuesUpdate) {
    buf.write_int64(data.end_time);
    buf.write_int32(data.quest_id);
    buf.write_uint32(data.state_flags);
    for progress in &data.objective_progress {
        buf.write_uint16(*progress);
    }
}

fn write_player_data_values_update_section(
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

pub(super) fn write_full_player_values_update_block(
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

pub(super) fn write_full_active_player_values_update_block(
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
fn write_player_data_values_update(
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

fn active_player_mask_has(data: &ActivePlayerDataValuesUpdate, bit: usize) -> bool {
    field_blocks_have(&data.active_player_data_mask, bit)
}

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

/// ActivePlayerData VALUES update for the runtime paths currently emitted by
/// RustyCore: InvSlots[141], buyback, coinage and combat stats.
///
/// C++ `UF::ActivePlayerData::WriteUpdate` format:
///   WriteUInt32(blocksMask group 0) — byte-aligned u32 for first 32 blocks
///   WriteBits(blocksMask group 1, 16) — 16 bits for remaining 16 blocks
///   for each active block: WriteBits(block, 32)
///   FlushBits()
///   [second dynamic-mask pass for parent-0 fields 4..19]
///   FlushBits()
///   [field values]
///
/// This writer intentionally does not cover the full 1525-bit
/// ActivePlayerData surface yet. `#026i` tracks the remaining generic writer
/// work: SkillInfo, quest/title/toy/transmog/trait dynamics, research,
/// PVP/rest/profession/bag flags, quest completed and glyph arrays.
///
/// InvSlots: parent=124, elements=125-265. Span multiple blocks.
///
/// ActivePlayerData secondary stats (from stat_changes):
///   Parent 0:            bits 36-37 (expertise)  → block 0 bit 0, block 1 bits 4-5
///   Parent 38:           bits 39-69 (all 31 fields) → block 1 bits 6-31, block 2 bits 0-5
///   ModDamageDonePos[7]: parent=269, bits=277-283 → block 8 bits 13,21-27
///   CombatRatings[32]:   parent=574, bits=575-606 → block 17 bits 30-31, block 18 bits 0-30
///
/// C++ WriteUpdate order for these fields:
/// parent 0 → parent 38 → InvSlots(124) → SpellCrit/ModDamageDone(269)
/// → Buyback(549) → CombatRatings(574).
pub(super) fn write_active_player_data_values_update(
    buf: &mut WorldPacket,
    inv_slot_changes: &[(u8, ObjectGuid)],
    buyback_changes: &[(u8, u32, i64)],
    stat_changes: Option<&PlayerStatChanges>,
    coinage_change: Option<u64>,
) {
    let mut blocks = [0u32; 48];

    // Coinage: block 0 bit 28 (ActivePlayerData.Coinage = new(0, 28))
    if coinage_change.is_some() {
        blocks[0] |= 1 << 0;
        blocks[0] |= 1 << 28;
    }

    // InvSlots: parent bit 124 = block 3 bit 28
    if !inv_slot_changes.is_empty() {
        blocks[3] |= 1 << 28;
        for &(slot, _) in inv_slot_changes {
            if (slot as u32) >= 141 {
                continue;
            }
            let bit = 125 + slot as u32;
            let block_idx = (bit / 32) as usize;
            let bit_in_block = bit % 32;
            if block_idx < 48 {
                blocks[block_idx] |= 1 << bit_in_block;
            }
        }
    }

    // BuybackPrice[12]: parent bit 549, price bits 550-561, timestamp bits 562-573.
    if !buyback_changes.is_empty() {
        blocks[17] |= 1 << 5;
        for &(slot, _, _) in buyback_changes {
            if !(94..106).contains(&slot) {
                continue;
            }
            let index = u32::from(slot - 94);
            for bit in [550 + index, 562 + index] {
                let block_idx = (bit / 32) as usize;
                let bit_in_block = bit % 32;
                if block_idx < 48 {
                    blocks[block_idx] |= 1 << bit_in_block;
                }
            }
        }
    }

    // Secondary stats from stat_changes
    if stat_changes.is_some() {
        // Parent 0 section: MainhandExpertise(bit 36→b1:4), OffhandExpertise(bit 37→b1:5)
        blocks[0] |= 1 << 0;
        blocks[1] |= (1 << 4) | (1 << 5);

        // Parent 38 section: 30 fields (bits 39-49, 51-69). This represented
        // stats VALUES writer is a narrow runtime path, not the final generic
        // C++ update-field writer. The 3.4.3.54261 client rejects bit 50 in
        // this packet shape; emitting it shifts every following ActivePlayerData
        // field by +4 bytes and desyncs the client's value walk. C++ still
        // defines ShieldBlockCritPercentage, so this is not a schema deletion:
        // the final generic writer must emit exactly the C++ change mask for
        // the object state being updated.
        // parent=38→b1:6, bits 39-63→b1:7-31 EXCEPT bit 50→b1:18, bits 64-69→b2:0-5
        blocks[1] |= 0xFFFB_FFC0; // bits 6-31 except bit 18 (field 50, reserved)
        blocks[2] |= 0x3F; // bits 0-5

        // Parent 269 section (block 8): SpellCritPercentage[7] + ModDamageDonePos[7]
        // parent=269→bit13, SpellCrit[0-6]=270-276→bits14-20, ModDmgPos[0-6]=277-283→bits21-27
        blocks[8] |= (1 << 13) | (0x7F << 14) | (0x7F << 21);

        // CombatRatings[32]: parent bit 574 (block 17 bit 30), CR[0] bit 575 (block 17 bit 31)
        blocks[17] |= (1 << 30) | (1 << 31);
        // CR[1-31]: bits 576-606 → block 18 bits 0-30
        blocks[18] |= 0x7FFF_FFFF;
    }

    // Group masks (which blocks have changes)
    let mut group0: u32 = 0;
    let mut group1: u32 = 0;
    for i in 0..32 {
        if blocks[i] != 0 {
            group0 |= 1 << i;
        }
    }
    for i in 32..48 {
        if blocks[i] != 0 {
            group1 |= 1 << (i - 32);
        }
    }

    // C++ `UF::ActivePlayerData::WriteUpdate`: WriteUInt32 for group 0
    // (byte-aligned), then WriteBits for group 1 (16 bits).
    buf.write_uint32(group0);
    buf.write_bits(group1, 16);

    // Write block masks for blocks with changes
    for i in 0..48 {
        if blocks[i] != 0 {
            buf.write_bits(blocks[i], 32);
        }
    }

    // First C++ FlushBits point. The supported runtime paths do not emit any
    // early bit payloads here (SortBags/InsertItems/KnownTitles/research).
    buf.flush_bits();

    // Second C++ dynamic-mask pass for parent-0 fields 4..19. Those fields are
    // outside this runtime writer, so no bits are emitted; keep this explicit
    // so future ActivePlayerData work does not collapse the C++ phases.
    buf.flush_bits();

    // Field values in C++ `UF::ActivePlayerData::WriteUpdate` order.

    // Block 0: Coinage (bit 28) — written before all other ActivePlayerData fields.
    // C++ `ActivePlayerData::Coinage` is written in the block-0 field pass.
    if let Some(coinage) = coinage_change {
        buf.write_int64(coinage as i64);
    }

    // Parent 0 section: expertise (bits 36-37) — BEFORE parent 38
    if let Some(sc) = stat_changes {
        buf.write_float(sc.mainhand_expertise); // bit 36: MainhandExpertise
        buf.write_float(sc.offhand_expertise); // bit 37: OffhandExpertise
    }

    // Parent 38 section: 30 fields (bits 39-49, 51-69) in C++ definition order.
    // Field bit 50 (ShieldBlockCritPercentage) is skipped in this represented
    // stats packet shape; see the mask above.
    if let Some(sc) = stat_changes {
        buf.write_float(sc.ranged_expertise); // bit 39: RangedExpertise
        buf.write_float(sc.combat_rating_expertise); // bit 40: CombatRatingExpertise
        buf.write_float(sc.block_pct); // bit 41: BlockPercentage
        buf.write_float(sc.dodge_pct); // bit 42: DodgePercentage
        buf.write_float(sc.dodge_from_attr); // bit 43: DodgePercentageFromAttribute
        buf.write_float(sc.parry_pct); // bit 44: ParryPercentage
        buf.write_float(sc.parry_from_attr); // bit 45: ParryPercentageFromAttribute
        buf.write_float(sc.crit_pct); // bit 46: CritPercentage
        buf.write_float(sc.ranged_crit_pct); // bit 47: RangedCritPercentage
        buf.write_float(sc.offhand_crit_pct); // bit 48: OffhandCritPercentage
        buf.write_int32(sc.shield_block); // bit 49: ShieldBlock
        // bit 50: ShieldBlockCritPercentage — RESERVED in the 54261 client grammar,
        // no property; never masked (see blocks[1] above) and never written here.
        buf.write_float(0.0); // bit 51: Mastery
        buf.write_float(0.0); // bit 52: Speed
        buf.write_float(0.0); // bit 53: Avoidance
        buf.write_float(0.0); // bit 54: Sturdiness
        buf.write_int32(0); // bit 55: Versatility
        buf.write_float(0.0); // bit 56: VersatilityBonus
        buf.write_float(0.0); // bit 57: PvpPowerDamage
        buf.write_float(0.0); // bit 58: PvpPowerHealing
        buf.write_int32(sc.spell_power); // bit 59: ModHealingDonePos
        buf.write_float(sc.mod_healing_pct); // bit 60: ModHealingPercent
        buf.write_float(sc.mod_healing_done_pct); // bit 61: ModHealingDonePercent
        buf.write_float(sc.mod_periodic_healing_pct); // bit 62: ModPeriodicHealingDonePercent
        buf.write_float(sc.mod_spell_power_pct); // bit 63: ModSpellPowerPercent
        buf.write_float(0.0); // bit 64: ModResiliencePercent
        buf.write_float(-1.0); // bit 65: OverrideSpellPowerByAPPercent
        buf.write_float(-1.0); // bit 66: OverrideAPBySpellPowerPercent
        buf.write_int32(0); // bit 67: ModTargetResistance
        buf.write_int32(0); // bit 68: ModTargetPhysicalResistance
        buf.write_uint32(0); // bit 69: LocalFlags
    }

    // Parent 124 section: InvSlots
    for slot in 0..141u8 {
        if let Some(&(_, ref guid)) = inv_slot_changes.iter().find(|&&(s, _)| s == slot) {
            buf.write_packed_guid(guid);
        }
    }

    // Parent 269 section: SpellCritPercentage[7] + ModDamageDonePos[7]
    // C++ interleaves SpellCritPct/ModDmgDonePos/ModDmgDoneNeg/ModDmgDonePct per school.
    // Both SpellCritPct bits (270-276) and ModDmgDonePos bits (277-283) are set.
    if let Some(sc) = stat_changes {
        for i in 0..7 {
            buf.write_float(sc.spell_crit_pct[i]); // SpellCritPercentage[i]
            if i == 0 {
                buf.write_int32(0); // Physical school: no spell power
            } else {
                buf.write_int32(sc.spell_power); // Magic schools 1-6
            }
            // ModDamageDoneNeg[i] bits 284-290: NOT set → skip
            // ModDamageDonePercent[i] bits 291-297: NOT set → skip
        }
    }

    for slot in 94..106u8 {
        if let Some(&(_, price, timestamp)) = buyback_changes.iter().find(|&&(s, _, _)| s == slot) {
            buf.write_uint32(price);
            buf.write_int64(timestamp);
        }
    }

    // Parent 574 section: CombatRatings[0-31]
    if let Some(sc) = stat_changes {
        for i in 0..32 {
            buf.write_int32(sc.combat_ratings[i]);
        }
    }
}
