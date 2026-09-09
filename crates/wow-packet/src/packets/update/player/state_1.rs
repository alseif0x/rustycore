//! Player update-value structs state definitions, part 1 of 5.
//!
//! Separated from the player.rs root under #650. Behaviour is preserved.

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
