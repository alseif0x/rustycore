//! Miscellaneous generated-style DB2 readers still required for full C++ store parity.

use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result};
use tracing::info;

use crate::wdc4::Wdc4Reader;

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

macro_rules! db2_store {
    ($store:ident, $entry:ty) => {
        pub struct $store {
            entries: HashMap<u32, $entry>,
        }

        impl $store {
            pub fn from_entries(entries: impl IntoIterator<Item = $entry>) -> Self {
                Self {
                    entries: entries.into_iter().map(|entry| (entry.id, entry)).collect(),
                }
            }

            pub fn get(&self, id: u32) -> Option<&$entry> {
                self.entries.get(&id)
            }

            pub fn len(&self) -> usize {
                self.entries.len()
            }

            pub fn is_empty(&self) -> bool {
                self.entries.is_empty()
            }
        }
    };
}

db2_store!(AdventureJournalStore, AdventureJournalEntry);
db2_store!(AdventureMapPoiStore, AdventureMapPoiEntry);
db2_store!(BannedAddonsStore, BannedAddonsEntry);
db2_store!(BroadcastTextStore, BroadcastTextEntry);
db2_store!(CfgCategoriesStore, CfgCategoriesEntry);
db2_store!(CfgRegionsStore, CfgRegionsEntry);
db2_store!(ChatChannelsStore, ChatChannelsEntry);
db2_store!(CinematicCameraStore, CinematicCameraEntry);
db2_store!(CinematicSequencesStore, CinematicSequencesEntry);
db2_store!(ConditionalChrModelStore, ConditionalChrModelEntry);
db2_store!(ConditionalContentTuningStore, ConditionalContentTuningEntry);
db2_store!(ExpectedStatStore, ExpectedStatEntry);
db2_store!(ExpectedStatModStore, ExpectedStatModEntry);
db2_store!(GarrAbilityStore, GarrAbilityEntry);
db2_store!(GarrBuildingStore, GarrBuildingEntry);
db2_store!(GarrBuildingPlotInstStore, GarrBuildingPlotInstEntry);
db2_store!(GarrClassSpecStore, GarrClassSpecEntry);
db2_store!(GarrFollowerStore, GarrFollowerEntry);
db2_store!(GarrFollowerXAbilityStore, GarrFollowerXAbilityEntry);
db2_store!(GarrMissionStore, GarrMissionEntry);
db2_store!(GarrPlotStore, GarrPlotEntry);
db2_store!(GarrPlotBuildingStore, GarrPlotBuildingEntry);
db2_store!(GarrPlotInstanceStore, GarrPlotInstanceEntry);
db2_store!(GarrSiteLevelStore, GarrSiteLevelEntry);
db2_store!(GarrSiteLevelPlotInstStore, GarrSiteLevelPlotInstEntry);
db2_store!(GarrTalentTreeStore, GarrTalentTreeEntry);
db2_store!(GemPropertiesStore, GemPropertiesEntry);
db2_store!(GossipNpcOptionStore, GossipNpcOptionEntry);
db2_store!(GuildColorBackgroundStore, GuildColorEntry);
db2_store!(GuildColorBorderStore, GuildColorEntry);
db2_store!(GuildColorEmblemStore, GuildColorEntry);
db2_store!(GuildPerkSpellsStore, GuildPerkSpellsEntry);
db2_store!(HolidaysStore, HolidaysEntry);
db2_store!(KeychainStore, KeychainEntry);
db2_store!(KeystoneAffixStore, KeystoneAffixEntry);
db2_store!(LfgDungeonsStore, LfgDungeonsEntry);
db2_store!(LanguageWordsStore, LanguageWordsEntry);
db2_store!(LanguagesStore, LanguagesEntry);
db2_store!(MailTemplateStore, MailTemplateEntry);
db2_store!(MovieStore, MovieEntry);
db2_store!(MythicPlusSeasonStore, MythicPlusSeasonEntry);
db2_store!(NamesProfanityStore, NamesProfanityEntry);
db2_store!(NamesReservedStore, NamesReservedEntry);
db2_store!(NamesReservedLocaleStore, NamesReservedLocaleEntry);
db2_store!(OverrideSpellDataStore, OverrideSpellDataEntry);
db2_store!(PvpDifficultyStore, PvpDifficultyEntry);
db2_store!(PvpItemStore, PvpItemEntry);
db2_store!(PrestigeLevelInfoStore, PrestigeLevelInfoEntry);
db2_store!(ScenarioStore, ScenarioEntry);
db2_store!(SceneScriptStore, SceneScriptEntry);
db2_store!(SceneScriptGlobalTextStore, SceneScriptTextEntry);
db2_store!(SceneScriptTextStore, SceneScriptTextEntry);
db2_store!(ServerMessagesStore, ServerMessagesEntry);
db2_store!(SoundKitStore, SoundKitEntry);
db2_store!(SpecSetMemberStore, SpecSetMemberEntry);
db2_store!(SpecializationSpellsStore, SpecializationSpellsEntry);
db2_store!(SummonPropertiesStore, SummonPropertiesEntry);
db2_store!(TactKeyStore, TactKeyEntry);
db2_store!(TotemCategoryStore, TotemCategoryEntry);

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

impl ExpectedStatModStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "ExpectedStatMod.db2", |id, idx, r| {
            ExpectedStatModEntry {
                id,
                creature_health_mod: f32_field(r, idx, 0),
                player_health_mod: f32_field(r, idx, 1),
                creature_auto_attack_dps_mod: f32_field(r, idx, 2),
                creature_armor_mod: f32_field(r, idx, 3),
                player_mana_mod: f32_field(r, idx, 4),
                player_primary_stat_mod: f32_field(r, idx, 5),
                player_secondary_stat_mod: f32_field(r, idx, 6),
                armor_constant_mod: f32_field(r, idx, 7),
                creature_spell_damage_mod: f32_field(r, idx, 8),
            }
        })
    }
}

impl GarrAbilityStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "GarrAbility.db2", |id, idx, r| {
            GarrAbilityEntry {
                id,
                name: r.get_field_string(idx, 0),
                description: r.get_field_string(idx, 1),
                garr_ability_category_id: r.get_field_u8(idx, 3),
                garr_follower_type_id: r.get_field_u8(idx, 4),
                icon_file_data_id: r.get_field_i32(idx, 5),
                faction_change_garr_ability_id: r.get_field_u16(idx, 6),
                flags: r.get_field_u16(idx, 7),
            }
        })
    }
}

impl GarrBuildingStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "GarrBuilding.db2", |id, idx, r| {
            GarrBuildingEntry {
                id,
                horde_name: r.get_field_string(idx, 0),
                alliance_name: r.get_field_string(idx, 1),
                description: r.get_field_string(idx, 2),
                tooltip: r.get_field_string(idx, 3),
                garr_type_id: r.get_field_u8(idx, 4),
                building_type: r.get_field_u8(idx, 5),
                horde_game_object_id: r.get_field_i32(idx, 6),
                alliance_game_object_id: r.get_field_i32(idx, 7),
                garr_site_id: r.get_field_u8(idx, 8),
                upgrade_level: r.get_field_u8(idx, 9),
                build_seconds: r.get_field_i32(idx, 10),
                currency_type_id: r.get_field_u16(idx, 11),
                currency_qty: r.get_field_i32(idx, 12),
                horde_ui_texture_kit_id: r.get_field_u16(idx, 13),
                alliance_ui_texture_kit_id: r.get_field_u16(idx, 14),
                icon_file_data_id: r.get_field_i32(idx, 15),
                alliance_scene_script_package_id: r.get_field_u16(idx, 16),
                horde_scene_script_package_id: r.get_field_u16(idx, 17),
                max_assignments: r.get_field_i32(idx, 18),
                shipment_capacity: r.get_field_u8(idx, 19),
                garr_ability_id: r.get_field_u16(idx, 20),
                bonus_garr_ability_id: r.get_field_u16(idx, 21),
                gold_cost: r.get_field_u16(idx, 22),
                flags: r.get_field_u8(idx, 23),
            }
        })
    }
}

impl GarrBuildingPlotInstStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(
            data_dir,
            locale,
            "GarrBuildingPlotInst.db2",
            |id, idx, r| GarrBuildingPlotInstEntry {
                id,
                map_offset: f32_array::<2>(r, idx, 0),
                garr_building_id: r.get_relationship_id(idx).unwrap_or(0),
                garr_site_level_plot_inst_id: r.get_field_u16(idx, 3),
                ui_texture_atlas_member_id: r.get_field_u16(idx, 4),
            },
        )
    }
}

impl GarrClassSpecStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "GarrClassSpec.db2", |id, idx, r| {
            GarrClassSpecEntry {
                id,
                class_spec: r.get_field_string(idx, 0),
                class_spec_male: r.get_field_string(idx, 1),
                class_spec_female: r.get_field_string(idx, 2),
                ui_texture_atlas_member_id: r.get_field_u16(idx, 4),
                garr_foll_item_set_id: r.get_field_u16(idx, 5),
                follower_class_limit: r.get_field_u8(idx, 6),
                flags: r.get_field_u8(idx, 7),
            }
        })
    }
}

impl GarrFollowerStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "GarrFollower.db2", |id, idx, r| {
            GarrFollowerEntry {
                id,
                horde_source_text: r.get_field_string(idx, 0),
                alliance_source_text: r.get_field_string(idx, 1),
                title_name: r.get_field_string(idx, 2),
                garr_type_id: r.get_field_u8(idx, 4),
                garr_follower_type_id: r.get_field_u8(idx, 5),
                horde_creature_id: r.get_field_i32(idx, 6),
                alliance_creature_id: r.get_field_i32(idx, 7),
                horde_garr_foll_race_id: r.get_field_u8(idx, 8),
                alliance_garr_foll_race_id: r.get_field_u8(idx, 9),
                horde_garr_class_spec_id: r.get_field_u8(idx, 10),
                alliance_garr_class_spec_id: r.get_field_u8(idx, 11),
                quality: r.get_field_u8(idx, 12),
                follower_level: r.get_field_u8(idx, 13),
                item_level_weapon: r.get_field_u16(idx, 14),
                item_level_armor: r.get_field_u16(idx, 15),
                horde_source_type_enum: r.get_field_i8(idx, 16),
                alliance_source_type_enum: r.get_field_i8(idx, 17),
                horde_icon_file_data_id: r.get_field_i32(idx, 18),
                alliance_icon_file_data_id: r.get_field_i32(idx, 19),
                horde_garr_foll_item_set_id: r.get_field_u16(idx, 20),
                alliance_garr_foll_item_set_id: r.get_field_u16(idx, 21),
                horde_ui_texture_kit_id: r.get_field_u16(idx, 22),
                alliance_ui_texture_kit_id: r.get_field_u16(idx, 23),
                vitality: r.get_field_u8(idx, 24),
                horde_flavor_garr_string_id: r.get_field_u8(idx, 25),
                alliance_flavor_garr_string_id: r.get_field_u8(idx, 26),
                horde_slotting_broadcast_text_id: r.get_field_u32(idx, 27),
                ally_slotting_broadcast_text_id: r.get_field_u32(idx, 28),
                chr_class_id: r.get_field_u8(idx, 29),
                flags: r.get_field_u8(idx, 30),
                gender: r.get_field_u8(idx, 31),
            }
        })
    }
}

impl GarrFollowerXAbilityStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(
            data_dir,
            locale,
            "GarrFollowerXAbility.db2",
            |id, idx, r| GarrFollowerXAbilityEntry {
                id,
                order_index: r.get_field_u8(idx, 0),
                faction_index: r.get_field_u8(idx, 1),
                garr_ability_id: r.get_field_u16(idx, 2),
                garr_follower_id: r.get_relationship_id(idx).unwrap_or(0),
            },
        )
    }
}

impl GarrMissionStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "GarrMission.db2", |id, idx, r| {
            GarrMissionEntry {
                id,
                name: r.get_field_string(idx, 0),
                location: r.get_field_string(idx, 1),
                description: r.get_field_string(idx, 2),
                map_pos: f32_array::<2>(r, idx, 3),
                world_pos: f32_array::<2>(r, idx, 4),
                garr_type_id: r.get_field_u8(idx, 6),
                garr_mission_type_id: r.get_field_u8(idx, 7),
                garr_follower_type_id: r.get_field_u8(idx, 8),
                max_followers: r.get_field_u8(idx, 9),
                mission_cost: r.get_field_u32(idx, 10),
                mission_cost_currency_types_id: r.get_field_u16(idx, 11),
                offered_garr_mission_texture_id: r.get_field_u8(idx, 12),
                ui_texture_kit_id: r.get_field_u16(idx, 13),
                env_garr_mechanic_id: r.get_field_u32(idx, 14),
                env_garr_mechanic_type_id: r.get_field_u8(idx, 15),
                player_condition_id: r.get_field_u32(idx, 16),
                target_level: r.get_field_i8(idx, 17),
                target_item_level: r.get_field_u16(idx, 18),
                mission_duration: r.get_field_i32(idx, 19),
                travel_duration: r.get_field_i32(idx, 20),
                offer_duration: r.get_field_u32(idx, 21),
                base_completion_chance: r.get_field_u8(idx, 22),
                base_follower_xp: r.get_field_u32(idx, 23),
                overmax_reward_pack_id: r.get_field_u32(idx, 24),
                follower_death_chance: r.get_field_u8(idx, 25),
                area_id: r.get_field_u32(idx, 26),
                flags: r.get_field_u32(idx, 27),
                garr_mission_set_id: r.get_relationship_id(idx).unwrap_or(0),
            }
        })
    }
}

impl GarrPlotStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "GarrPlot.db2", |id, idx, r| {
            GarrPlotEntry {
                id,
                name: r.get_field_string(idx, 0),
                plot_type: r.get_field_u8(idx, 1),
                horde_construct_obj_id: r.get_field_i32(idx, 2),
                alliance_construct_obj_id: r.get_field_i32(idx, 3),
                flags: r.get_field_u8(idx, 4),
                ui_category_id: r.get_field_u8(idx, 5),
                upgrade_requirement: std::array::from_fn(|i| r.get_array_element(idx, 6, i, 32)),
            }
        })
    }
}

impl GarrPlotBuildingStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "GarrPlotBuilding.db2", |id, idx, r| {
            GarrPlotBuildingEntry {
                id,
                garr_plot_id: r.get_field_u8(idx, 0),
                garr_building_id: r.get_field_u8(idx, 1),
            }
        })
    }
}

impl GarrPlotInstanceStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "GarrPlotInstance.db2", |id, idx, r| {
            GarrPlotInstanceEntry {
                id,
                name: r.get_field_string(idx, 0),
                garr_plot_id: r.get_field_u8(idx, 1),
            }
        })
    }
}

impl GarrSiteLevelStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "GarrSiteLevel.db2", |id, idx, r| {
            GarrSiteLevelEntry {
                id,
                town_hall_ui_pos: f32_array::<2>(r, idx, 0),
                garr_site_id: r.get_field_u32(idx, 1),
                garr_level: r.get_field_u8(idx, 2),
                map_id: r.get_field_u16(idx, 3),
                upgrade_movie_id: r.get_field_u16(idx, 4),
                ui_texture_kit_id: r.get_field_u16(idx, 5),
                max_building_level: r.get_field_u8(idx, 6),
                upgrade_cost: r.get_field_u16(idx, 7),
                upgrade_gold_cost: r.get_field_u16(idx, 8),
            }
        })
    }
}

impl GarrSiteLevelPlotInstStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(
            data_dir,
            locale,
            "GarrSiteLevelPlotInst.db2",
            |id, idx, r| GarrSiteLevelPlotInstEntry {
                id,
                ui_marker_pos: f32_array::<2>(r, idx, 0),
                garr_site_level_id: r.get_relationship_id(idx).unwrap_or(0),
                garr_plot_instance_id: r.get_field_u8(idx, 2),
                ui_marker_size: r.get_field_u8(idx, 3),
            },
        )
    }
}

impl GarrTalentTreeStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "GarrTalentTree.db2", |id, idx, r| {
            GarrTalentTreeEntry {
                id,
                name: r.get_field_string(idx, 0),
                garr_type_id: r.get_field_i32(idx, 1),
                class_id: r.get_field_i32(idx, 2),
                max_tiers: r.get_field_i8(idx, 3),
                ui_order: r.get_field_i8(idx, 4),
                flags: r.get_field_i8(idx, 5),
                ui_texture_kit_id: r.get_field_u16(idx, 6),
            }
        })
    }
}

impl GemPropertiesStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "GemProperties.db2", |id, idx, r| {
            GemPropertiesEntry {
                id,
                enchant_id: r.get_field_u16(idx, 0),
                gem_type: r.get_field_i32(idx, 1),
                min_item_level: r.get_field_u16(idx, 2),
            }
        })
    }
}

impl GossipNpcOptionStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "GossipNPCOption.db2", |id, idx, r| {
            GossipNpcOptionEntry {
                id,
                gossip_npc_option: r.get_field_i32(idx, 0),
                lfg_dungeons_id: r.get_field_i32(idx, 1),
                unk_341: std::array::from_fn(|i| r.get_field_i32(idx, i + 2)),
                gossip_option_id: r.get_field_i32(idx, 11),
            }
        })
    }
}

macro_rules! impl_guild_color_load {
    ($store:ident, $file:literal) => {
        impl $store {
            pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
                load_store(data_dir, locale, $file, |id, idx, r| GuildColorEntry {
                    id,
                    red: r.get_field_u8(idx, 0),
                    blue: r.get_field_u8(idx, 1),
                    green: r.get_field_u8(idx, 2),
                })
            }
        }
    };
}

impl_guild_color_load!(GuildColorBackgroundStore, "GuildColorBackground.db2");
impl_guild_color_load!(GuildColorBorderStore, "GuildColorBorder.db2");
impl_guild_color_load!(GuildColorEmblemStore, "GuildColorEmblem.db2");

impl GuildPerkSpellsStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "GuildPerkSpells.db2", |id, idx, r| {
            GuildPerkSpellsEntry {
                id,
                spell_id: r.get_field_i32(idx, 0),
            }
        })
    }
}

impl HolidaysStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "Holidays.db2", |id, idx, r| {
            HolidaysEntry {
                id,
                region: r.get_field_u16(idx, 1),
                looping: r.get_field_u8(idx, 2),
                holiday_name_id: r.get_field_u32(idx, 3),
                holiday_description_id: r.get_field_u32(idx, 4),
                priority: r.get_field_u8(idx, 5),
                calendar_filter_type: r.get_field_i8(idx, 6),
                flags: r.get_field_u8(idx, 7),
                world_state_expression_id: r.get_field_u32(idx, 8),
                duration: std::array::from_fn(|i| r.get_array_element(idx, 9, i, 16) as u16),
                date: std::array::from_fn(|i| r.get_array_element(idx, 10, i, 32)),
                calendar_flags: std::array::from_fn(|i| r.get_array_element(idx, 11, i, 8) as u8),
                texture_file_data_id: std::array::from_fn(|i| {
                    r.get_array_element(idx, 12, i, 32) as i32
                }),
            }
        })
    }
}

impl KeychainStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "Keychain.db2", |id, idx, r| {
            KeychainEntry {
                id,
                key: std::array::from_fn(|i| r.get_array_element(idx, 0, i, 8) as u8),
            }
        })
    }
}

impl KeystoneAffixStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "KeystoneAffix.db2", |id, idx, r| {
            KeystoneAffixEntry {
                id,
                name: r.get_field_string(idx, 0),
                description: r.get_field_string(idx, 1),
                file_data_id: r.get_field_i32(idx, 3),
            }
        })
    }
}

impl LfgDungeonsStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "LFGDungeons.db2", |id, idx, r| {
            LfgDungeonsEntry {
                id,
                name: r.get_field_string(idx, 0),
                description: r.get_field_string(idx, 1),
                min_level: r.get_field_u8(idx, 2),
                max_level: r.get_field_u16(idx, 3),
                type_id: r.get_field_u8(idx, 4),
                subtype: r.get_field_u8(idx, 5),
                faction: r.get_field_i8(idx, 6),
                icon_texture_file_id: r.get_field_i32(idx, 7),
                rewards_bg_texture_file_id: r.get_field_i32(idx, 8),
                popup_bg_texture_file_id: r.get_field_i32(idx, 9),
                expansion_level: r.get_field_u8(idx, 10),
                map_id: r.get_field_i16(idx, 11),
                difficulty_id: r.get_field_u8(idx, 12),
                min_gear: f32_field(r, idx, 13),
                group_id: r.get_field_u8(idx, 14),
                order_index: r.get_field_u8(idx, 15),
                required_player_condition_id: r.get_field_u32(idx, 16),
                target_level: r.get_field_u8(idx, 17),
                target_level_min: r.get_field_u8(idx, 18),
                target_level_max: r.get_field_u16(idx, 19),
                random_id: r.get_field_u16(idx, 20),
                scenario_id: r.get_field_u16(idx, 21),
                final_encounter_id: r.get_field_u16(idx, 22),
                count_tank: r.get_field_u8(idx, 23),
                count_healer: r.get_field_u8(idx, 24),
                count_damage: r.get_field_u8(idx, 25),
                min_count_tank: r.get_field_u8(idx, 26),
                min_count_healer: r.get_field_u8(idx, 27),
                min_count_damage: r.get_field_u8(idx, 28),
                bonus_reputation_amount: r.get_field_u16(idx, 29),
                mentor_item_level: r.get_field_u16(idx, 30),
                mentor_char_level: r.get_field_u8(idx, 31),
                flags: std::array::from_fn(|i| r.get_array_element(idx, 32, i, 32) as i32),
            }
        })
    }

    pub fn get_by_map_and_difficulty_like_cpp(
        &self,
        map_id: u32,
        difficulty_id: u8,
    ) -> Option<&LfgDungeonsEntry> {
        self.entries
            .values()
            .find(|entry| entry.map_id == map_id as i16 && entry.difficulty_id == difficulty_id)
    }
}

impl LanguageWordsStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "LanguageWords.db2", |id, idx, r| {
            LanguageWordsEntry {
                id,
                word: r.get_field_string(idx, 0),
                language_id: r.get_field_u8(idx, 1),
            }
        })
    }
}

impl LanguagesStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "Languages.db2", |id, idx, r| {
            LanguagesEntry {
                id,
                name: r.get_field_string(idx, 0),
                flags: r.get_field_i32(idx, 2),
                ui_texture_kit_id: r.get_field_i32(idx, 3),
                ui_texture_kit_element_count: r.get_field_i32(idx, 4),
                learning_curve_id: r.get_field_i32(idx, 5),
            }
        })
    }
}

impl MailTemplateStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "MailTemplate.db2", |id, idx, r| {
            MailTemplateEntry {
                id,
                body: r.get_field_string(idx, 0),
            }
        })
    }
}

impl MovieStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "Movie.db2", |id, idx, r| MovieEntry {
            id,
            volume: r.get_field_u8(idx, 0),
            key_id: r.get_field_u8(idx, 1),
            audio_file_data_id: r.get_field_u32(idx, 2),
            subtitle_file_data_id: r.get_field_u32(idx, 3),
        })
    }
}

impl MythicPlusSeasonStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "MythicPlusSeason.db2", |id, idx, r| {
            MythicPlusSeasonEntry {
                id,
                milestone_season: r.get_field_i32(idx, 1),
                expansion_level: r.get_field_i32(idx, 2),
                heroic_lfg_dungeon_min_gear: r.get_field_i32(idx, 3),
            }
        })
    }
}

impl NamesProfanityStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "NamesProfanity.db2", |id, idx, r| {
            NamesProfanityEntry {
                id,
                name: r.get_field_string(idx, 0),
                language: r.get_field_i8(idx, 1),
            }
        })
    }
}

impl NamesReservedStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "NamesReserved.db2", |id, idx, r| {
            NamesReservedEntry {
                id,
                name: r.get_field_string(idx, 0),
            }
        })
    }
}

impl NamesReservedLocaleStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "NamesReservedLocale.db2", |id, idx, r| {
            NamesReservedLocaleEntry {
                id,
                name: r.get_field_string(idx, 0),
                locale_mask: r.get_field_u8(idx, 1),
            }
        })
    }
}

impl OverrideSpellDataStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "OverrideSpellData.db2", |id, idx, r| {
            OverrideSpellDataEntry {
                id,
                spells: std::array::from_fn(|i| r.get_array_element(idx, 0, i, 32) as i32),
                player_action_bar_file_data_id: r.get_field_i32(idx, 1),
                flags: r.get_field_u8(idx, 2),
            }
        })
    }
}

impl PvpDifficultyStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "PVPDifficulty.db2", |id, idx, r| {
            PvpDifficultyEntry {
                id,
                range_index: r.get_field_u8(idx, 0),
                min_level: r.get_field_u8(idx, 1),
                max_level: r.get_field_u8(idx, 2),
                map_id: r.get_relationship_id(idx).unwrap_or(0),
            }
        })
    }
}

impl PvpItemStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "PVPItem.db2", |id, idx, r| PvpItemEntry {
            id,
            item_id: r.get_field_i32(idx, 0),
            item_level_delta: r.get_field_u8(idx, 1),
        })
    }
}

impl PrestigeLevelInfoStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "PrestigeLevelInfo.db2", |id, idx, r| {
            PrestigeLevelInfoEntry {
                id,
                name: r.get_field_string(idx, 0),
                prestige_level: r.get_field_i32(idx, 1),
                badge_texture_file_data_id: r.get_field_i32(idx, 2),
                flags: r.get_field_u8(idx, 3),
                awarded_achievement_id: r.get_field_i32(idx, 4),
            }
        })
    }
}

impl ScenarioStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "Scenario.db2", |id, idx, r| {
            ScenarioEntry {
                id,
                name: r.get_field_string(idx, 0),
                area_table_id: r.get_field_u16(idx, 1),
                scenario_type: r.get_field_u8(idx, 2),
                flags: r.get_field_u8(idx, 3),
                ui_texture_kit_id: r.get_field_u32(idx, 4),
            }
        })
    }
}

impl SceneScriptStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "SceneScript.db2", |id, idx, r| {
            SceneScriptEntry {
                id,
                first_scene_script_id: r.get_field_u16(idx, 0),
                next_scene_script_id: r.get_field_u16(idx, 1),
                unknown_915: r.get_field_i32(idx, 2),
            }
        })
    }
}

macro_rules! impl_scene_text_load {
    ($store:ident, $file:literal) => {
        impl $store {
            pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
                load_store(data_dir, locale, $file, |id, idx, r| SceneScriptTextEntry {
                    id,
                    name: r.get_field_string(idx, 0),
                    script: r.get_field_string(idx, 1),
                })
            }
        }
    };
}

impl_scene_text_load!(SceneScriptGlobalTextStore, "SceneScriptGlobalText.db2");
impl_scene_text_load!(SceneScriptTextStore, "SceneScriptText.db2");

impl ServerMessagesStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "ServerMessages.db2", |id, idx, r| {
            ServerMessagesEntry {
                id,
                text: r.get_field_string(idx, 0),
            }
        })
    }
}

impl SoundKitStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "SoundKit.db2", |id, idx, r| {
            SoundKitEntry {
                id,
                sound_type: r.get_field_u8(idx, 1),
                volume_float: f32_field(r, idx, 2),
                flags: r.get_field_u16(idx, 3),
                min_distance: f32_field(r, idx, 4),
                distance_cutoff: f32_field(r, idx, 5),
                eax_def: r.get_field_u8(idx, 6),
                sound_kit_advanced_id: r.get_field_u32(idx, 7),
                volume_variation_plus: f32_field(r, idx, 8),
                volume_variation_minus: f32_field(r, idx, 9),
                pitch_variation_plus: f32_field(r, idx, 10),
                pitch_variation_minus: f32_field(r, idx, 11),
                dialog_type: r.get_field_i8(idx, 12),
                pitch_adjust: f32_field(r, idx, 13),
                bus_overwrite_id: r.get_field_u16(idx, 14),
                max_instances: r.get_field_u8(idx, 15),
                sound_mix_group_id: r.get_field_u32(idx, 16),
            }
        })
    }
}

impl SpecSetMemberStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "SpecSetMember.db2", |id, idx, r| {
            SpecSetMemberEntry {
                id,
                chr_specialization_id: r.get_field_i32(idx, 0),
                spec_set_id: r.get_relationship_id(idx).unwrap_or(0),
            }
        })
    }
}

impl SpecializationSpellsStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(
            data_dir,
            locale,
            "SpecializationSpells.db2",
            |id, idx, r| SpecializationSpellsEntry {
                id,
                description: r.get_field_string(idx, 0),
                spec_id: r.get_field_u16(idx, 2),
                spell_id: r.get_field_i32(idx, 3),
                overrides_spell_id: r.get_field_i32(idx, 4),
                display_order: r.get_field_u8(idx, 5),
            },
        )
    }
}

impl SummonPropertiesStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "SummonProperties.db2", |id, idx, r| {
            SummonPropertiesEntry {
                id,
                control: r.get_field_i32(idx, 0),
                faction: r.get_field_i32(idx, 1),
                title: r.get_field_i32(idx, 2),
                slot: r.get_field_i32(idx, 3),
                flags: std::array::from_fn(|i| r.get_array_element(idx, 4, i, 32) as i32),
            }
        })
    }
}

impl TactKeyStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "TactKey.db2", |id, idx, r| TactKeyEntry {
            id,
            key: std::array::from_fn(|i| r.get_array_element(idx, 0, i, 8) as u8),
        })
    }
}

impl TotemCategoryStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "TotemCategory.db2", |id, idx, r| {
            TotemCategoryEntry {
                id,
                name: r.get_field_string(idx, 0),
                totem_category_type: r.get_field_u8(idx, 1),
                totem_category_mask: r.get_field_i32(idx, 2),
            }
        })
    }
}

fn load_store<T, S>(
    data_dir: &str,
    locale: &str,
    file_name: &str,
    mut read: impl FnMut(u32, usize, &Wdc4Reader) -> T,
) -> Result<S>
where
    S: FromEntries<T>,
{
    let path = Path::new(data_dir).join("dbc").join(locale).join(file_name);
    let reader =
        Wdc4Reader::open(&path).with_context(|| format!("failed to open {}", path.display()))?;

    let mut entries = Vec::with_capacity(reader.total_count());
    for (id, idx) in reader.iter_records() {
        entries.push(read(id, idx, &reader));
    }

    let store = S::from_entries(entries);
    info!("Loaded {} rows from {}", store.len(), path.display());
    Ok(store)
}

fn f32_field(reader: &Wdc4Reader, idx: usize, field: usize) -> f32 {
    f32::from_bits(reader.get_field_u32(idx, field))
}

fn f32_array<const N: usize>(reader: &Wdc4Reader, idx: usize, field: usize) -> [f32; N] {
    std::array::from_fn(|i| f32::from_bits(reader.get_array_element(idx, field, i, 32)))
}

trait FromEntries<T> {
    fn from_entries(entries: impl IntoIterator<Item = T>) -> Self;
    fn len(&self) -> usize;
}

macro_rules! impl_from_entries {
    ($store:ident, $entry:ty) => {
        impl FromEntries<$entry> for $store {
            fn from_entries(entries: impl IntoIterator<Item = $entry>) -> Self {
                Self::from_entries(entries)
            }

            fn len(&self) -> usize {
                self.len()
            }
        }
    };
}

impl_from_entries!(AdventureJournalStore, AdventureJournalEntry);
impl_from_entries!(AdventureMapPoiStore, AdventureMapPoiEntry);
impl_from_entries!(BannedAddonsStore, BannedAddonsEntry);
impl_from_entries!(BroadcastTextStore, BroadcastTextEntry);
impl_from_entries!(CfgCategoriesStore, CfgCategoriesEntry);
impl_from_entries!(CfgRegionsStore, CfgRegionsEntry);
impl_from_entries!(ChatChannelsStore, ChatChannelsEntry);
impl_from_entries!(CinematicCameraStore, CinematicCameraEntry);
impl_from_entries!(CinematicSequencesStore, CinematicSequencesEntry);
impl_from_entries!(ConditionalChrModelStore, ConditionalChrModelEntry);
impl_from_entries!(ConditionalContentTuningStore, ConditionalContentTuningEntry);
impl_from_entries!(ExpectedStatStore, ExpectedStatEntry);
impl_from_entries!(ExpectedStatModStore, ExpectedStatModEntry);
impl_from_entries!(GarrAbilityStore, GarrAbilityEntry);
impl_from_entries!(GarrBuildingStore, GarrBuildingEntry);
impl_from_entries!(GarrBuildingPlotInstStore, GarrBuildingPlotInstEntry);
impl_from_entries!(GarrClassSpecStore, GarrClassSpecEntry);
impl_from_entries!(GarrFollowerStore, GarrFollowerEntry);
impl_from_entries!(GarrFollowerXAbilityStore, GarrFollowerXAbilityEntry);
impl_from_entries!(GarrMissionStore, GarrMissionEntry);
impl_from_entries!(GarrPlotStore, GarrPlotEntry);
impl_from_entries!(GarrPlotBuildingStore, GarrPlotBuildingEntry);
impl_from_entries!(GarrPlotInstanceStore, GarrPlotInstanceEntry);
impl_from_entries!(GarrSiteLevelStore, GarrSiteLevelEntry);
impl_from_entries!(GarrSiteLevelPlotInstStore, GarrSiteLevelPlotInstEntry);
impl_from_entries!(GarrTalentTreeStore, GarrTalentTreeEntry);
impl_from_entries!(GemPropertiesStore, GemPropertiesEntry);
impl_from_entries!(GossipNpcOptionStore, GossipNpcOptionEntry);
impl_from_entries!(GuildColorBackgroundStore, GuildColorEntry);
impl_from_entries!(GuildColorBorderStore, GuildColorEntry);
impl_from_entries!(GuildColorEmblemStore, GuildColorEntry);
impl_from_entries!(GuildPerkSpellsStore, GuildPerkSpellsEntry);
impl_from_entries!(HolidaysStore, HolidaysEntry);
impl_from_entries!(KeychainStore, KeychainEntry);
impl_from_entries!(KeystoneAffixStore, KeystoneAffixEntry);
impl_from_entries!(LfgDungeonsStore, LfgDungeonsEntry);
impl_from_entries!(LanguageWordsStore, LanguageWordsEntry);
impl_from_entries!(LanguagesStore, LanguagesEntry);
impl_from_entries!(MailTemplateStore, MailTemplateEntry);
impl_from_entries!(MovieStore, MovieEntry);
impl_from_entries!(MythicPlusSeasonStore, MythicPlusSeasonEntry);
impl_from_entries!(NamesProfanityStore, NamesProfanityEntry);
impl_from_entries!(NamesReservedStore, NamesReservedEntry);
impl_from_entries!(NamesReservedLocaleStore, NamesReservedLocaleEntry);
impl_from_entries!(OverrideSpellDataStore, OverrideSpellDataEntry);
impl_from_entries!(PvpDifficultyStore, PvpDifficultyEntry);
impl_from_entries!(PvpItemStore, PvpItemEntry);
impl_from_entries!(PrestigeLevelInfoStore, PrestigeLevelInfoEntry);
impl_from_entries!(ScenarioStore, ScenarioEntry);
impl_from_entries!(SceneScriptStore, SceneScriptEntry);
impl_from_entries!(SceneScriptGlobalTextStore, SceneScriptTextEntry);
impl_from_entries!(SceneScriptTextStore, SceneScriptTextEntry);
impl_from_entries!(ServerMessagesStore, ServerMessagesEntry);
impl_from_entries!(SoundKitStore, SoundKitEntry);
impl_from_entries!(SpecSetMemberStore, SpecSetMemberEntry);
impl_from_entries!(SpecializationSpellsStore, SpecializationSpellsEntry);
impl_from_entries!(SummonPropertiesStore, SummonPropertiesEntry);
impl_from_entries!(TactKeyStore, TactKeyEntry);
impl_from_entries!(TotemCategoryStore, TotemCategoryEntry);

#[cfg(test)]
mod tests {
    use super::*;

    fn adventure_map_poi(id: u32, quest_id: u32, player_condition_id: u32) -> AdventureMapPoiEntry {
        AdventureMapPoiEntry {
            id,
            title: String::new(),
            description: String::new(),
            world_position: [0.0, 0.0],
            poi_type: 0,
            player_condition_id,
            quest_id,
            lfg_dungeon_id: 0,
            reward_item_id: 0,
            ui_texture_atlas_member_id: 0,
            ui_texture_kit_id: 0,
            map_id: 0,
            area_table_id: 0,
        }
    }

    #[test]
    fn adventure_map_poi_find_start_quest_matches_quest_and_condition_like_cpp() {
        let store = AdventureMapPoiStore::from_entries([
            adventure_map_poi(20, 900, 44),
            adventure_map_poi(10, 900, 43),
            adventure_map_poi(30, 901, 43),
        ]);

        let selected = store
            .find_start_quest_poi_like_cpp(900, |condition_id| condition_id == 44)
            .expect("matching adventure map poi");

        assert_eq!(selected.id, 20);
        assert!(
            store
                .find_start_quest_poi_like_cpp(900, |condition_id| condition_id == 99)
                .is_none()
        );
    }

    #[test]
    fn load_misc_generated_db2_subbatch_when_fixtures_exist() {
        let data_dir = "/home/server/woltk-server-core/Data";
        let locale = "esES";
        let dbc_dir = Path::new(data_dir).join("dbc").join(locale);
        if !dbc_dir.exists() {
            eprintln!(
                "Skipping test: DB2 fixture directory not found at {}",
                dbc_dir.display()
            );
            return;
        }

        macro_rules! load_if_exists {
            ($file:literal, $store:ty) => {
                if dbc_dir.join($file).exists() {
                    let _store = <$store>::load(data_dir, locale)
                        .unwrap_or_else(|error| panic!("failed to load {}: {error:#}", $file));
                }
            };
        }

        load_if_exists!("AdventureJournal.db2", AdventureJournalStore);
        load_if_exists!("AdventureMapPOI.db2", AdventureMapPoiStore);
        load_if_exists!("BannedAddons.db2", BannedAddonsStore);
        load_if_exists!("BroadcastText.db2", BroadcastTextStore);
        load_if_exists!("Cfg_Categories.db2", CfgCategoriesStore);
        load_if_exists!("Cfg_Regions.db2", CfgRegionsStore);
        load_if_exists!("ChatChannels.db2", ChatChannelsStore);
        load_if_exists!("CinematicCamera.db2", CinematicCameraStore);
        load_if_exists!("CinematicSequences.db2", CinematicSequencesStore);
        load_if_exists!("ConditionalChrModel.db2", ConditionalChrModelStore);
        load_if_exists!(
            "ConditionalContentTuning.db2",
            ConditionalContentTuningStore
        );
        load_if_exists!("ExpectedStat.db2", ExpectedStatStore);
        load_if_exists!("ExpectedStatMod.db2", ExpectedStatModStore);
        load_if_exists!("GarrAbility.db2", GarrAbilityStore);
        load_if_exists!("GarrBuilding.db2", GarrBuildingStore);
        load_if_exists!("GarrBuildingPlotInst.db2", GarrBuildingPlotInstStore);
        load_if_exists!("GarrClassSpec.db2", GarrClassSpecStore);
        load_if_exists!("GarrFollower.db2", GarrFollowerStore);
        load_if_exists!("GarrFollowerXAbility.db2", GarrFollowerXAbilityStore);
        load_if_exists!("GarrMission.db2", GarrMissionStore);
        load_if_exists!("GarrPlot.db2", GarrPlotStore);
        load_if_exists!("GarrPlotBuilding.db2", GarrPlotBuildingStore);
        load_if_exists!("GarrPlotInstance.db2", GarrPlotInstanceStore);
        load_if_exists!("GarrSiteLevel.db2", GarrSiteLevelStore);
        load_if_exists!("GarrSiteLevelPlotInst.db2", GarrSiteLevelPlotInstStore);
        load_if_exists!("GarrTalentTree.db2", GarrTalentTreeStore);
        load_if_exists!("GemProperties.db2", GemPropertiesStore);
        load_if_exists!("GossipNPCOption.db2", GossipNpcOptionStore);
        load_if_exists!("GuildColorBackground.db2", GuildColorBackgroundStore);
        load_if_exists!("GuildColorBorder.db2", GuildColorBorderStore);
        load_if_exists!("GuildColorEmblem.db2", GuildColorEmblemStore);
        load_if_exists!("GuildPerkSpells.db2", GuildPerkSpellsStore);
        load_if_exists!("Holidays.db2", HolidaysStore);
        load_if_exists!("Keychain.db2", KeychainStore);
        load_if_exists!("KeystoneAffix.db2", KeystoneAffixStore);
        load_if_exists!("LFGDungeons.db2", LfgDungeonsStore);
        load_if_exists!("LanguageWords.db2", LanguageWordsStore);
        load_if_exists!("Languages.db2", LanguagesStore);
        load_if_exists!("MailTemplate.db2", MailTemplateStore);
        load_if_exists!("Movie.db2", MovieStore);
        load_if_exists!("MythicPlusSeason.db2", MythicPlusSeasonStore);
        load_if_exists!("NamesProfanity.db2", NamesProfanityStore);
        load_if_exists!("NamesReserved.db2", NamesReservedStore);
        load_if_exists!("NamesReservedLocale.db2", NamesReservedLocaleStore);
        load_if_exists!("OverrideSpellData.db2", OverrideSpellDataStore);
        load_if_exists!("PVPDifficulty.db2", PvpDifficultyStore);
        load_if_exists!("PVPItem.db2", PvpItemStore);
        load_if_exists!("PrestigeLevelInfo.db2", PrestigeLevelInfoStore);
        load_if_exists!("Scenario.db2", ScenarioStore);
        load_if_exists!("SceneScript.db2", SceneScriptStore);
        load_if_exists!("SceneScriptGlobalText.db2", SceneScriptGlobalTextStore);
        load_if_exists!("SceneScriptText.db2", SceneScriptTextStore);
        load_if_exists!("ServerMessages.db2", ServerMessagesStore);
        load_if_exists!("SoundKit.db2", SoundKitStore);
        load_if_exists!("SpecSetMember.db2", SpecSetMemberStore);
        load_if_exists!("SpecializationSpells.db2", SpecializationSpellsStore);
        load_if_exists!("SummonProperties.db2", SummonPropertiesStore);
        load_if_exists!("TactKey.db2", TactKeyStore);
        load_if_exists!("TotemCategory.db2", TotemCategoryStore);
    }
}
