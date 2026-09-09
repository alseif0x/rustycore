//! Miscellaneous DB2 readers state definitions, part 1 of 2.
//!
//! Separated from the misc_generated.rs root under #664. Behaviour is preserved.

use super::*;

pub const MAX_BROADCAST_TEXT_EMOTES: usize = 3;

pub const KEYCHAIN_SIZE: usize = 32;

pub const MAX_HOLIDAY_DURATIONS: usize = 10;

pub const MAX_HOLIDAY_DATES: usize = 16;

pub const MAX_HOLIDAY_FLAGS: usize = 10;

pub const MAX_OVERRIDE_SPELL: usize = 10;

pub const TACTKEY_SIZE: usize = 16;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConditionalChrModelEntry {
    pub id: u32,
    pub db2_id: i32,
    pub chr_model_id: u32,
    pub chr_customization_req_id: i32,
    pub player_condition_id: i32,
    pub flags: i32,
    pub chr_customization_category_id: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConditionalContentTuningEntry {
    pub id: u32,
    pub order_index: i32,
    pub redirect_content_tuning_id: i32,
    pub redirect_flag: i32,
    pub parent_content_tuning_id: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AdventureJournalEntry {
    pub id: u32,
    pub name: String,
    pub description: String,
    pub button_text: String,
    pub reward_description: String,
    pub continue_description: String,
    pub journal_type: u8,
    pub player_condition_id: u32,
    pub flags: i32,
    pub button_action_type: u8,
    pub texture_file_data_id: i32,
    pub lfg_dungeon_id: u16,
    pub quest_id: u32,
    pub battle_master_list_id: u16,
    pub priority_min: u8,
    pub priority_max: u8,
    pub item_id: i32,
    pub item_quantity: u32,
    pub currency_type: u16,
    pub currency_quantity: u8,
    pub ui_map_id: u16,
    pub bonus_player_condition_id: [u32; 2],
    pub bonus_value: [u8; 2],
}

#[derive(Debug, Clone, PartialEq)]
pub struct AdventureMapPoiEntry {
    pub id: u32,
    pub title: String,
    pub description: String,
    pub world_position: [f32; 2],
    pub poi_type: i8,
    pub player_condition_id: u32,
    pub quest_id: u32,
    pub lfg_dungeon_id: u32,
    pub reward_item_id: i32,
    pub ui_texture_atlas_member_id: u32,
    pub ui_texture_kit_id: u32,
    pub map_id: i32,
    pub area_table_id: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BannedAddonsEntry {
    pub id: u32,
    pub name: String,
    pub version: String,
    pub flags: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BroadcastTextEntry {
    pub id: u32,
    pub text: String,
    pub text1: String,
    pub language_id: i32,
    pub condition_id: i32,
    pub emotes_id: u16,
    pub flags: u8,
    pub chat_bubble_duration_ms: u32,
    pub voice_over_priority_id: i32,
    pub sound_kit_id: [u32; 2],
    pub emote_id: [u16; MAX_BROADCAST_TEXT_EMOTES],
    pub emote_delay: [u16; MAX_BROADCAST_TEXT_EMOTES],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CfgCategoriesEntry {
    pub id: u32,
    pub name: String,
    pub locale_mask: u16,
    pub create_charset_mask: u8,
    pub existing_charset_mask: u8,
    pub flags: u8,
    pub order: i8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CfgRegionsEntry {
    pub id: u32,
    pub tag: String,
    pub region_id: u16,
    pub raid_origin: u32,
    pub region_group_mask: u8,
    pub challenge_origin: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatChannelsEntry {
    pub id: u32,
    pub name: String,
    pub shortcut: String,
    pub flags: i32,
    pub faction_group: i8,
    pub ruleset: i32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CinematicCameraEntry {
    pub id: u32,
    pub origin: [f32; 3],
    pub sound_id: u32,
    pub origin_facing: f32,
    pub file_data_id: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CinematicSequencesEntry {
    pub id: u32,
    pub sound_id: u32,
    pub camera: [u16; 8],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GossipNpcOptionEntry {
    pub id: u32,
    pub gossip_npc_option: i32,
    pub lfg_dungeons_id: i32,
    pub unk_341: [i32; 9],
    pub gossip_option_id: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuildColorEntry {
    pub id: u32,
    pub red: u8,
    pub blue: u8,
    pub green: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuildPerkSpellsEntry {
    pub id: u32,
    pub spell_id: i32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExpectedStatEntry {
    pub id: u32,
    pub expansion_id: i32,
    pub creature_health: f32,
    pub player_health: f32,
    pub creature_auto_attack_dps: f32,
    pub creature_armor: f32,
    pub player_mana: f32,
    pub player_primary_stat: f32,
    pub player_secondary_stat: f32,
    pub armor_constant: f32,
    pub creature_spell_damage: f32,
    pub lvl: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExpectedStatModEntry {
    pub id: u32,
    pub creature_health_mod: f32,
    pub player_health_mod: f32,
    pub creature_auto_attack_dps_mod: f32,
    pub creature_armor_mod: f32,
    pub player_mana_mod: f32,
    pub player_primary_stat_mod: f32,
    pub player_secondary_stat_mod: f32,
    pub armor_constant_mod: f32,
    pub creature_spell_damage_mod: f32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GarrAbilityEntry {
    pub id: u32,
    pub name: String,
    pub description: String,
    pub garr_ability_category_id: u8,
    pub garr_follower_type_id: u8,
    pub icon_file_data_id: i32,
    pub faction_change_garr_ability_id: u16,
    pub flags: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GarrBuildingEntry {
    pub id: u32,
    pub horde_name: String,
    pub alliance_name: String,
    pub description: String,
    pub tooltip: String,
    pub garr_type_id: u8,
    pub building_type: u8,
    pub horde_game_object_id: i32,
    pub alliance_game_object_id: i32,
    pub garr_site_id: u8,
    pub upgrade_level: u8,
    pub build_seconds: i32,
    pub currency_type_id: u16,
    pub currency_qty: i32,
    pub horde_ui_texture_kit_id: u16,
    pub alliance_ui_texture_kit_id: u16,
    pub icon_file_data_id: i32,
    pub alliance_scene_script_package_id: u16,
    pub horde_scene_script_package_id: u16,
    pub max_assignments: i32,
    pub shipment_capacity: u8,
    pub garr_ability_id: u16,
    pub bonus_garr_ability_id: u16,
    pub gold_cost: u16,
    pub flags: u8,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GarrBuildingPlotInstEntry {
    pub id: u32,
    pub map_offset: [f32; 2],
    pub garr_building_id: u32,
    pub garr_site_level_plot_inst_id: u16,
    pub ui_texture_atlas_member_id: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GarrClassSpecEntry {
    pub id: u32,
    pub class_spec: String,
    pub class_spec_male: String,
    pub class_spec_female: String,
    pub ui_texture_atlas_member_id: u16,
    pub garr_foll_item_set_id: u16,
    pub follower_class_limit: u8,
    pub flags: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GarrFollowerEntry {
    pub id: u32,
    pub horde_source_text: String,
    pub alliance_source_text: String,
    pub title_name: String,
    pub garr_type_id: u8,
    pub garr_follower_type_id: u8,
    pub horde_creature_id: i32,
    pub alliance_creature_id: i32,
    pub horde_garr_foll_race_id: u8,
    pub alliance_garr_foll_race_id: u8,
    pub horde_garr_class_spec_id: u8,
    pub alliance_garr_class_spec_id: u8,
    pub quality: u8,
    pub follower_level: u8,
    pub item_level_weapon: u16,
    pub item_level_armor: u16,
    pub horde_source_type_enum: i8,
    pub alliance_source_type_enum: i8,
    pub horde_icon_file_data_id: i32,
    pub alliance_icon_file_data_id: i32,
    pub horde_garr_foll_item_set_id: u16,
    pub alliance_garr_foll_item_set_id: u16,
    pub horde_ui_texture_kit_id: u16,
    pub alliance_ui_texture_kit_id: u16,
    pub vitality: u8,
    pub horde_flavor_garr_string_id: u8,
    pub alliance_flavor_garr_string_id: u8,
    pub horde_slotting_broadcast_text_id: u32,
    pub ally_slotting_broadcast_text_id: u32,
    pub chr_class_id: u8,
    pub flags: u8,
    pub gender: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GarrFollowerXAbilityEntry {
    pub id: u32,
    pub order_index: u8,
    pub faction_index: u8,
    pub garr_ability_id: u16,
    pub garr_follower_id: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GarrMissionEntry {
    pub id: u32,
    pub name: String,
    pub location: String,
    pub description: String,
    pub map_pos: [f32; 2],
    pub world_pos: [f32; 2],
    pub garr_type_id: u8,
    pub garr_mission_type_id: u8,
    pub garr_follower_type_id: u8,
    pub max_followers: u8,
    pub mission_cost: u32,
    pub mission_cost_currency_types_id: u16,
    pub offered_garr_mission_texture_id: u8,
    pub ui_texture_kit_id: u16,
    pub env_garr_mechanic_id: u32,
    pub env_garr_mechanic_type_id: u8,
    pub player_condition_id: u32,
    pub target_level: i8,
    pub target_item_level: u16,
    pub mission_duration: i32,
    pub travel_duration: i32,
    pub offer_duration: u32,
    pub base_completion_chance: u8,
    pub base_follower_xp: u32,
    pub overmax_reward_pack_id: u32,
    pub follower_death_chance: u8,
    pub area_id: u32,
    pub flags: u32,
    pub garr_mission_set_id: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GarrPlotEntry {
    pub id: u32,
    pub name: String,
    pub plot_type: u8,
    pub horde_construct_obj_id: i32,
    pub alliance_construct_obj_id: i32,
    pub flags: u8,
    pub ui_category_id: u8,
    pub upgrade_requirement: [u32; 2],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GarrPlotBuildingEntry {
    pub id: u32,
    pub garr_plot_id: u8,
    pub garr_building_id: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GarrPlotInstanceEntry {
    pub id: u32,
    pub name: String,
    pub garr_plot_id: u8,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GarrSiteLevelEntry {
    pub id: u32,
    pub town_hall_ui_pos: [f32; 2],
    pub garr_site_id: u32,
    pub garr_level: u8,
    pub map_id: u16,
    pub upgrade_movie_id: u16,
    pub ui_texture_kit_id: u16,
    pub max_building_level: u8,
    pub upgrade_cost: u16,
    pub upgrade_gold_cost: u16,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GarrSiteLevelPlotInstEntry {
    pub id: u32,
    pub ui_marker_pos: [f32; 2],
    pub garr_site_level_id: u32,
    pub garr_plot_instance_id: u8,
    pub ui_marker_size: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GarrTalentTreeEntry {
    pub id: u32,
    pub name: String,
    pub garr_type_id: i32,
    pub class_id: i32,
    pub max_tiers: i8,
    pub ui_order: i8,
    pub flags: i8,
    pub ui_texture_kit_id: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GemPropertiesEntry {
    pub id: u32,
    pub enchant_id: u16,
    pub gem_type: i32,
    pub min_item_level: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HolidaysEntry {
    pub id: u32,
    pub region: u16,
    pub looping: u8,
    pub holiday_name_id: u32,
    pub holiday_description_id: u32,
    pub priority: u8,
    pub calendar_filter_type: i8,
    pub flags: u8,
    pub world_state_expression_id: u32,
    pub duration: [u16; MAX_HOLIDAY_DURATIONS],
    pub date: [u32; MAX_HOLIDAY_DATES],
    pub calendar_flags: [u8; MAX_HOLIDAY_FLAGS],
    pub texture_file_data_id: [i32; 3],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeychainEntry {
    pub id: u32,
    pub key: [u8; KEYCHAIN_SIZE],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeystoneAffixEntry {
    pub id: u32,
    pub name: String,
    pub description: String,
    pub file_data_id: i32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LfgDungeonsEntry {
    pub id: u32,
    pub name: String,
    pub description: String,
    pub min_level: u8,
    pub max_level: u16,
    pub type_id: u8,
    pub subtype: u8,
    pub faction: i8,
    pub icon_texture_file_id: i32,
    pub rewards_bg_texture_file_id: i32,
    pub popup_bg_texture_file_id: i32,
    pub expansion_level: u8,
    pub map_id: i16,
    pub difficulty_id: u8,
    pub min_gear: f32,
    pub group_id: u8,
    pub order_index: u8,
    pub required_player_condition_id: u32,
    pub target_level: u8,
    pub target_level_min: u8,
    pub target_level_max: u16,
    pub random_id: u16,
    pub scenario_id: u16,
    pub final_encounter_id: u16,
    pub count_tank: u8,
    pub count_healer: u8,
    pub count_damage: u8,
    pub min_count_tank: u8,
    pub min_count_healer: u8,
    pub min_count_damage: u8,
    pub bonus_reputation_amount: u16,
    pub mentor_item_level: u16,
    pub mentor_char_level: u8,
    pub flags: [i32; 2],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LanguageWordsEntry {
    pub id: u32,
    pub word: String,
    pub language_id: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LanguagesEntry {
    pub id: u32,
    pub name: String,
    pub flags: i32,
    pub ui_texture_kit_id: i32,
    pub ui_texture_kit_element_count: i32,
    pub learning_curve_id: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MailTemplateEntry {
    pub id: u32,
    pub body: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MovieEntry {
    pub id: u32,
    pub volume: u8,
    pub key_id: u8,
    pub audio_file_data_id: u32,
    pub subtitle_file_data_id: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MythicPlusSeasonEntry {
    pub id: u32,
    pub milestone_season: i32,
    pub expansion_level: i32,
    pub heroic_lfg_dungeon_min_gear: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NamesProfanityEntry {
    pub id: u32,
    pub name: String,
    pub language: i8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NamesReservedEntry {
    pub id: u32,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NamesReservedLocaleEntry {
    pub id: u32,
    pub name: String,
    pub locale_mask: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OverrideSpellDataEntry {
    pub id: u32,
    pub spells: [i32; MAX_OVERRIDE_SPELL],
    pub player_action_bar_file_data_id: i32,
    pub flags: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PvpDifficultyEntry {
    pub id: u32,
    pub range_index: u8,
    pub min_level: u8,
    pub max_level: u8,
    pub map_id: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PvpItemEntry {
    pub id: u32,
    pub item_id: i32,
    pub item_level_delta: u8,
}

pub struct PvpItemStore {
    pub(super) entries: HashMap<u32, PvpItemEntry>,
    pub(super) item_level_bonus_by_item_id: HashMap<i32, u8>,
}

impl PvpItemStore {
    pub fn from_entries(entries: impl IntoIterator<Item = PvpItemEntry>) -> Self {
        let mut by_id = HashMap::new();
        let mut item_level_bonus_by_item_id = HashMap::new();
        for entry in entries {
            item_level_bonus_by_item_id.insert(entry.item_id, entry.item_level_delta);
            by_id.insert(entry.id, entry);
        }

        Self {
            entries: by_id,
            item_level_bonus_by_item_id,
        }
    }

    pub fn get(&self, id: u32) -> Option<&PvpItemEntry> {
        self.entries.get(&id)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// C++ `DB2Manager::GetPvpItemLevelBonus(itemId)`.
    pub fn item_level_bonus_like_cpp(&self, item_id: u32) -> u8 {
        let Ok(item_id) = i32::try_from(item_id) else {
            return 0;
        };

        self.item_level_bonus_by_item_id
            .get(&item_id)
            .copied()
            .unwrap_or(0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrestigeLevelInfoEntry {
    pub id: u32,
    pub name: String,
    pub prestige_level: i32,
    pub badge_texture_file_data_id: i32,
    pub flags: u8,
    pub awarded_achievement_id: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScenarioEntry {
    pub id: u32,
    pub name: String,
    pub area_table_id: u16,
    pub scenario_type: u8,
    pub flags: u8,
    pub ui_texture_kit_id: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SceneScriptEntry {
    pub id: u32,
    pub first_scene_script_id: u16,
    pub next_scene_script_id: u16,
    pub unknown_915: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SceneScriptTextEntry {
    pub id: u32,
    pub name: String,
    pub script: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerMessagesEntry {
    pub id: u32,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SoundKitEntry {
    pub id: u32,
    pub sound_type: u8,
    pub volume_float: f32,
    pub flags: u16,
    pub min_distance: f32,
    pub distance_cutoff: f32,
    pub eax_def: u8,
    pub sound_kit_advanced_id: u32,
    pub volume_variation_plus: f32,
    pub volume_variation_minus: f32,
    pub pitch_variation_plus: f32,
    pub pitch_variation_minus: f32,
    pub dialog_type: i8,
    pub pitch_adjust: f32,
    pub bus_overwrite_id: u16,
    pub max_instances: u8,
    pub sound_mix_group_id: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpecSetMemberEntry {
    pub id: u32,
    pub chr_specialization_id: i32,
    pub spec_set_id: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpecializationSpellsEntry {
    pub id: u32,
    pub description: String,
    pub spec_id: u16,
    pub spell_id: i32,
    pub overrides_spell_id: i32,
    pub display_order: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SummonPropertiesEntry {
    pub id: u32,
    pub control: i32,
    pub faction: i32,
    pub title: i32,
    pub slot: i32,
    pub flags: [i32; 2],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TactKeyEntry {
    pub id: u32,
    pub key: [u8; TACTKEY_SIZE],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TotemCategoryEntry {
    pub id: u32,
    pub name: String,
    pub totem_category_type: u8,
    pub totem_category_mask: i32,
}

impl AdventureJournalStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "AdventureJournal.db2", |id, idx, r| {
            AdventureJournalEntry {
                id,
                name: r.get_field_string(idx, 0),
                description: r.get_field_string(idx, 1),
                button_text: r.get_field_string(idx, 2),
                reward_description: r.get_field_string(idx, 3),
                continue_description: r.get_field_string(idx, 4),
                journal_type: r.get_field_u8(idx, 5),
                player_condition_id: r.get_field_u32(idx, 6),
                flags: r.get_field_i32(idx, 7),
                button_action_type: r.get_field_u8(idx, 8),
                texture_file_data_id: r.get_field_i32(idx, 9),
                lfg_dungeon_id: r.get_field_u16(idx, 10),
                quest_id: r.get_field_u32(idx, 11),
                battle_master_list_id: r.get_field_u16(idx, 12),
                priority_min: r.get_field_u8(idx, 13),
                priority_max: r.get_field_u8(idx, 14),
                item_id: r.get_field_i32(idx, 15),
                item_quantity: r.get_field_u32(idx, 16),
                currency_type: r.get_field_u16(idx, 17),
                currency_quantity: r.get_field_u8(idx, 18),
                ui_map_id: r.get_field_u16(idx, 19),
                bonus_player_condition_id: std::array::from_fn(|i| {
                    r.get_array_element(idx, 20, i, 32)
                }),
                bonus_value: std::array::from_fn(|i| r.get_array_element(idx, 21, i, 8) as u8),
            }
        })
    }
}

impl AdventureMapPoiStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "AdventureMapPOI.db2", |id, idx, r| {
            AdventureMapPoiEntry {
                id,
                title: r.get_field_string(idx, 0),
                description: r.get_field_string(idx, 1),
                world_position: f32_array::<2>(r, idx, 2),
                poi_type: r.get_field_i8(idx, 3),
                player_condition_id: r.get_field_u32(idx, 4),
                quest_id: r.get_field_u32(idx, 5),
                lfg_dungeon_id: r.get_field_u32(idx, 6),
                reward_item_id: r.get_field_i32(idx, 7),
                ui_texture_atlas_member_id: r.get_field_u32(idx, 8),
                ui_texture_kit_id: r.get_field_u32(idx, 9),
                map_id: r.get_field_i32(idx, 10),
                area_table_id: r.get_field_u32(idx, 11),
            }
        })
    }

    /// C++ `sAdventureMapPOIStore` scan used by `HandleAdventureMapStartQuest`.
    pub fn find_start_quest_poi_like_cpp(
        &self,
        quest_id: u32,
        mut meets_player_condition: impl FnMut(u32) -> bool,
    ) -> Option<&AdventureMapPoiEntry> {
        let mut entries: Vec<_> = self.entries.values().collect();
        entries.sort_by_key(|entry| entry.id);
        entries.into_iter().find(|entry| {
            entry.quest_id == quest_id && meets_player_condition(entry.player_condition_id)
        })
    }
}

impl BannedAddonsStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "BannedAddons.db2", |id, idx, r| {
            BannedAddonsEntry {
                id,
                name: r.get_field_string(idx, 0),
                version: r.get_field_string(idx, 1),
                flags: r.get_field_u8(idx, 2),
            }
        })
    }
}

impl BroadcastTextStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "BroadcastText.db2", |id, idx, r| {
            BroadcastTextEntry {
                id,
                text: r.get_field_string(idx, 0),
                text1: r.get_field_string(idx, 1),
                language_id: r.get_field_i32(idx, 3),
                condition_id: r.get_field_i32(idx, 4),
                emotes_id: r.get_field_u16(idx, 5),
                flags: r.get_field_u8(idx, 6),
                chat_bubble_duration_ms: r.get_field_u32(idx, 7),
                voice_over_priority_id: r.get_field_i32(idx, 8),
                sound_kit_id: std::array::from_fn(|i| r.get_array_element(idx, 9, i, 32)),
                emote_id: std::array::from_fn(|i| r.get_array_element(idx, 10, i, 16) as u16),
                emote_delay: std::array::from_fn(|i| r.get_array_element(idx, 11, i, 16) as u16),
            }
        })
    }
}

impl CfgCategoriesStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "Cfg_Categories.db2", |id, idx, r| {
            CfgCategoriesEntry {
                id,
                name: r.get_field_string(idx, 0),
                locale_mask: r.get_field_u16(idx, 1),
                create_charset_mask: r.get_field_u8(idx, 2),
                existing_charset_mask: r.get_field_u8(idx, 3),
                flags: r.get_field_u8(idx, 4),
                order: r.get_field_i8(idx, 5),
            }
        })
    }
}

impl CfgRegionsStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "Cfg_Regions.db2", |id, idx, r| {
            CfgRegionsEntry {
                id,
                tag: r.get_field_string(idx, 0),
                region_id: r.get_field_u16(idx, 1),
                raid_origin: r.get_field_u32(idx, 2),
                region_group_mask: r.get_field_u8(idx, 3),
                challenge_origin: r.get_field_u32(idx, 4),
            }
        })
    }
}

impl ChatChannelsStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "ChatChannels.db2", |id, idx, r| {
            ChatChannelsEntry {
                id,
                name: r.get_field_string(idx, 0),
                shortcut: r.get_field_string(idx, 1),
                flags: r.get_field_i32(idx, 3),
                faction_group: r.get_field_i8(idx, 4),
                ruleset: r.get_field_i32(idx, 5),
            }
        })
    }
}

impl CinematicCameraStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "CinematicCamera.db2", |id, idx, r| {
            CinematicCameraEntry {
                id,
                origin: f32_array::<3>(r, idx, 0),
                sound_id: r.get_field_u32(idx, 1),
                origin_facing: f32_field(r, idx, 2),
                file_data_id: r.get_field_u32(idx, 3),
            }
        })
    }
}

impl CinematicSequencesStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "CinematicSequences.db2", |id, idx, r| {
            CinematicSequencesEntry {
                id,
                sound_id: r.get_field_u32(idx, 0),
                camera: std::array::from_fn(|i| r.get_array_element(idx, 1, i, 16) as u16),
            }
        })
    }
}

impl ConditionalChrModelStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "ConditionalChrModel.db2", |id, idx, r| {
            ConditionalChrModelEntry {
                id,
                db2_id: r.get_field_i32(idx, 0),
                chr_model_id: id,
                chr_customization_req_id: r.get_field_i32(idx, 2),
                player_condition_id: r.get_field_i32(idx, 3),
                flags: r.get_field_i32(idx, 4),
                chr_customization_category_id: r.get_field_i32(idx, 5),
            }
        })
    }
}

impl ConditionalContentTuningStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(
            data_dir,
            locale,
            "ConditionalContentTuning.db2",
            |id, idx, r| ConditionalContentTuningEntry {
                id,
                order_index: r.get_field_i32(idx, 0),
                redirect_content_tuning_id: r.get_field_i32(idx, 1),
                redirect_flag: r.get_field_i32(idx, 2),
                parent_content_tuning_id: r.get_relationship_id(idx).unwrap_or(0),
            },
        )
    }
}

impl ExpectedStatStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "ExpectedStat.db2", |id, idx, r| {
            ExpectedStatEntry {
                id,
                expansion_id: r.get_field_i32(idx, 0),
                creature_health: f32_field(r, idx, 1),
                player_health: f32_field(r, idx, 2),
                creature_auto_attack_dps: f32_field(r, idx, 3),
                creature_armor: f32_field(r, idx, 4),
                player_mana: f32_field(r, idx, 5),
                player_primary_stat: f32_field(r, idx, 6),
                player_secondary_stat: f32_field(r, idx, 7),
                armor_constant: f32_field(r, idx, 8),
                creature_spell_damage: f32_field(r, idx, 9),
                lvl: r.get_relationship_id(idx).unwrap_or(0),
            }
        })
    }
}
