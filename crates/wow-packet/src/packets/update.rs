// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! UpdateObject packet — used to create, update, and destroy game objects
//! in the client's view.
//!
//! Wire format matches RustyCore C# for WoW 3.4.3.54261.

use std::collections::BTreeSet;

use wow_constants::ServerOpcodes;
use wow_core::guid::TypeId;
use wow_core::{ObjectGuid, Position};

use crate::{ServerPacket, WorldPacket};

// ── UpdateType ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum UpdateType {
    Values = 0,
    CreateObject = 1,
    CreateObject2 = 2,
}

// ── MovementBlock ───────────────────────────────────────────────────

/// Movement data included in a CreateObject block.
#[derive(Debug, Clone)]
pub struct MovementBlock {
    pub position: Position,
    pub walk_speed: f32,
    pub run_speed: f32,
    pub run_back_speed: f32,
    pub swim_speed: f32,
    pub swim_back_speed: f32,
    pub fly_speed: f32,
    pub fly_back_speed: f32,
    pub turn_rate: f32,
    pub pitch_rate: f32,
}

impl Default for MovementBlock {
    fn default() -> Self {
        Self {
            position: Position::ZERO,
            walk_speed: 2.5,
            run_speed: 7.0,
            run_back_speed: 4.5,
            swim_speed: 4.72222,
            swim_back_speed: 2.5,
            fly_speed: 7.0,
            fly_back_speed: 4.5,
            turn_rate: std::f32::consts::PI,
            pitch_rate: std::f32::consts::PI,
        }
    }
}

// ── ObjectData VALUES delta ─────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ObjectDataValuesUpdate {
    pub changed_object_type_mask: u32,
    pub object_data_mask: u32,
    pub entry_id: i32,
    pub dynamic_flags: u32,
    pub scale: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DynamicObjectDataValuesUpdate {
    pub changed_object_type_mask: u32,
    pub object_data: Option<ObjectDataValuesUpdate>,
    pub dynamic_object_data_mask: u32,
    pub caster: ObjectGuid,
    pub dynamic_object_type: u8,
    pub spell_visual_id: i32,
    pub spell_id: i32,
    pub radius: f32,
    pub cast_time_ms: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SceneObjectDataValuesUpdate {
    pub changed_object_type_mask: u32,
    pub object_data: Option<ObjectDataValuesUpdate>,
    pub scene_object_data_mask: u32,
    pub script_package_id: i32,
    pub rnd_seed_val: u32,
    pub created_by: ObjectGuid,
    pub scene_type: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConversationLineValuesUpdate {
    pub conversation_line_id: i32,
    pub start_time: u32,
    pub ui_camera_id: i32,
    pub actor_index: u8,
    pub flags: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConversationActorValuesUpdate {
    pub actor_type: u32,
    pub id: i32,
    pub creature_id: u32,
    pub creature_display_info_id: u32,
    pub actor_guid: ObjectGuid,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConversationDataValuesUpdate {
    pub changed_object_type_mask: u32,
    pub object_data: Option<ObjectDataValuesUpdate>,
    pub conversation_data_mask: u32,
    pub lines: Vec<ConversationLineValuesUpdate>,
    pub actors: Vec<ConversationActorValuesUpdate>,
    /// C++ `DynamicUpdateField<ConversationActor>` nested mask blocks.
    ///
    /// `None` represents `ignoreNestedChangesMask=true`, so all actors present in
    /// `actors` are marked and written. `Some(blocks)` writes exactly those
    /// nested change-mask bits and serializes only marked actor indices.
    pub actor_update_mask: Option<Vec<u32>>,
    pub last_line_end_time: i32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GameObjectDataValuesUpdate {
    pub changed_object_type_mask: u32,
    pub object_data: Option<ObjectDataValuesUpdate>,
    pub game_object_data_mask: u32,
    pub state_world_effect_ids: Vec<u32>,
    pub enable_doodad_sets: Vec<i32>,
    pub enable_doodad_sets_update_mask: Option<Vec<u32>>,
    pub world_effects: Vec<i32>,
    pub world_effects_update_mask: Option<Vec<u32>>,
    pub display_id: i32,
    pub spell_visual_id: u32,
    pub state_spell_visual_id: u32,
    pub spawn_tracking_state_anim_id: u32,
    pub spawn_tracking_state_anim_kit_id: u32,
    pub created_by: ObjectGuid,
    pub guild_guid: ObjectGuid,
    pub flags: u32,
    pub parent_rotation: [f32; 4],
    pub faction_template: i32,
    pub level: i32,
    pub state: i8,
    pub type_id: i8,
    pub percent_health: u8,
    pub art_kit: u32,
    pub custom_param: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChrCustomizationChoiceValuesUpdate {
    pub option_id: u32,
    pub choice_id: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CorpseDataValuesUpdate {
    pub changed_object_type_mask: u32,
    pub object_data: Option<ObjectDataValuesUpdate>,
    pub corpse_data_mask: u32,
    pub customizations: Vec<ChrCustomizationChoiceValuesUpdate>,
    pub customizations_update_mask: Option<Vec<u32>>,
    pub dynamic_flags: u32,
    pub owner: ObjectGuid,
    pub party_guid: ObjectGuid,
    pub guild_guid: ObjectGuid,
    pub display_id: u32,
    pub race_id: u8,
    pub sex: u8,
    pub class: u8,
    pub flags: u32,
    pub faction_template: i32,
    pub items: [u32; 19],
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScaleCurveValuesUpdate {
    pub scale_curve_mask: u32,
    pub override_active: bool,
    pub start_time_offset: u32,
    pub parameter_curve: u32,
    pub points: [(f32, f32); 2],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VisualAnimValuesUpdate {
    pub visual_anim_mask: u32,
    pub field_c: bool,
    pub animation_data_id: u32,
    pub anim_kit_id: u32,
    pub anim_progress: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AreaTriggerDataValuesUpdate {
    pub changed_object_type_mask: u32,
    pub object_data: Option<ObjectDataValuesUpdate>,
    pub area_trigger_data_mask: u32,
    pub override_scale_curve: ScaleCurveValuesUpdate,
    pub extra_scale_curve: ScaleCurveValuesUpdate,
    pub override_move_curve_x: ScaleCurveValuesUpdate,
    pub override_move_curve_y: ScaleCurveValuesUpdate,
    pub override_move_curve_z: ScaleCurveValuesUpdate,
    pub caster: ObjectGuid,
    pub duration: u32,
    pub time_to_target: u32,
    pub time_to_target_scale: u32,
    pub time_to_target_extra_scale: u32,
    pub time_to_target_pos: u32,
    pub spell_id: i32,
    pub spell_for_visuals: i32,
    pub spell_visual_id: i32,
    pub bounds_radius_2d: f32,
    pub decal_properties_id: u32,
    pub creating_effect_guid: ObjectGuid,
    pub orbit_path_target: ObjectGuid,
    pub visual_anim: VisualAnimValuesUpdate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ItemEnchantmentValuesUpdate {
    pub item_enchantment_mask: u32,
    pub id: i32,
    pub duration: u32,
    pub charges: i16,
    pub field_a: u8,
    pub field_b: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArtifactPowerValuesUpdate {
    pub artifact_power_id: i16,
    pub purchased_rank: u8,
    pub current_rank_with_bonus: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SocketedGemValuesUpdate {
    pub socketed_gem_mask: u32,
    pub item_id: i32,
    pub context: u8,
    pub bonus_list_ids: [u16; 16],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ItemModValuesUpdate {
    pub value: i32,
    pub item_mod_type: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemModListValuesUpdate {
    pub item_mod_list_mask: u32,
    pub values: Vec<ItemModValuesUpdate>,
    pub values_update_mask: Option<Vec<u32>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ItemBonusKeyValuesUpdate {
    pub item_id: i32,
    pub bonus_list_ids: Vec<i32>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ItemDataValuesDeltaUpdate {
    pub changed_object_type_mask: u32,
    pub object_data: Option<ObjectDataValuesUpdate>,
    pub item_data_mask: u64,
    pub artifact_powers: Vec<ArtifactPowerValuesUpdate>,
    pub artifact_powers_update_mask: Option<Vec<u32>>,
    pub gems: Vec<SocketedGemValuesUpdate>,
    pub gems_update_mask: Option<Vec<u32>>,
    pub owner: ObjectGuid,
    pub contained_in: ObjectGuid,
    pub creator: ObjectGuid,
    pub gift_creator: ObjectGuid,
    pub stack_count: u32,
    pub expiration: u32,
    pub dynamic_flags: u32,
    pub property_seed: i32,
    pub random_properties_id: i32,
    pub durability: u32,
    pub max_durability: u32,
    pub create_played_time: u32,
    pub context: i32,
    pub create_time: i64,
    pub artifact_xp: u64,
    pub item_appearance_mod_id: u8,
    pub modifiers: ItemModListValuesUpdate,
    pub dynamic_flags2: u32,
    pub item_bonus_key: ItemBonusKeyValuesUpdate,
    pub debug_item_level: u16,
    pub spell_charges: [i32; 5],
    pub enchantments: [ItemEnchantmentValuesUpdate; 13],
}

#[derive(Debug, Clone, PartialEq)]
pub struct ContainerDataValuesUpdate {
    pub changed_object_type_mask: u32,
    pub object_data: Option<ObjectDataValuesUpdate>,
    pub item_data: Option<ItemDataValuesDeltaUpdate>,
    pub container_data_mask: u64,
    pub num_slots: u32,
    pub slots: [ObjectGuid; 36],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PassiveSpellHistoryValuesUpdate {
    pub spell_id: i32,
    pub aura_spell_id: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct UnitChannelValuesUpdate {
    pub spell_id: i32,
    pub spell_visual_id: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct VisibleItemValuesUpdate {
    pub visible_item_mask: u32,
    pub item_id: i32,
    pub appearance_mod_id: u16,
    pub item_visual: u16,
}

#[derive(Debug, Clone, PartialEq)]
pub struct UnitDataValuesDeltaUpdate {
    pub changed_object_type_mask: u32,
    pub object_data: Option<ObjectDataValuesUpdate>,
    pub unit_data_mask: [u32; 8],
    pub state_world_effect_ids: Vec<u32>,
    pub passive_spells: Vec<PassiveSpellHistoryValuesUpdate>,
    pub passive_spells_update_mask: Option<Vec<u32>>,
    pub world_effects: Vec<i32>,
    pub world_effects_update_mask: Option<Vec<u32>>,
    pub channel_objects: Vec<ObjectGuid>,
    pub channel_objects_update_mask: Option<Vec<u32>>,
    pub health: i64,
    pub max_health: i64,
    pub display_id: i32,
    pub state_spell_visual_id: u32,
    pub state_anim_id: u32,
    pub state_anim_kit_id: u32,
    pub charm: ObjectGuid,
    pub summon: ObjectGuid,
    pub critter: ObjectGuid,
    pub charmed_by: ObjectGuid,
    pub summoned_by: ObjectGuid,
    pub created_by: ObjectGuid,
    pub demon_creator: ObjectGuid,
    pub look_at_controller_target: ObjectGuid,
    pub target: ObjectGuid,
    pub battle_pet_companion_guid: ObjectGuid,
    pub battle_pet_db_id: u64,
    pub channel_data: UnitChannelValuesUpdate,
    pub summoned_by_home_realm: u32,
    pub race: u8,
    pub class_id: u8,
    pub player_class_id: u8,
    pub sex: u8,
    pub display_power: u8,
    pub override_display_power_id: u32,
    pub level: i32,
    pub effective_level: i32,
    pub content_tuning_id: i32,
    pub scaling_level_min: i32,
    pub scaling_level_max: i32,
    pub scaling_level_delta: i32,
    pub scaling_faction_group: i32,
    pub scaling_health_item_level_curve_id: i32,
    pub scaling_damage_item_level_curve_id: i32,
    pub faction_template: i32,
    pub flags: u32,
    pub flags2: u32,
    pub flags3: u32,
    pub aura_state: u32,
    pub ranged_attack_round_base_time: u32,
    pub bounding_radius: f32,
    pub combat_reach: f32,
    pub display_scale: f32,
    pub native_display_id: i32,
    pub native_display_scale: f32,
    pub mount_display_id: i32,
    pub min_damage: f32,
    pub max_damage: f32,
    pub min_off_hand_damage: f32,
    pub max_off_hand_damage: f32,
    pub stand_state: u8,
    pub pet_talent_points: u8,
    pub vis_flags: u8,
    pub anim_tier: u8,
    pub pet_number: u32,
    pub pet_name_timestamp: u32,
    pub pet_experience: u32,
    pub pet_next_level_experience: u32,
    pub mod_casting_speed: f32,
    pub mod_spell_haste: f32,
    pub mod_haste: f32,
    pub mod_ranged_haste: f32,
    pub mod_haste_regen: f32,
    pub mod_time_rate: f32,
    pub created_by_spell: i32,
    pub emote_state: i32,
    pub training_points_used: i16,
    pub training_points_total: i16,
    pub base_mana: i32,
    pub base_health: i32,
    pub sheathe_state: u8,
    pub pvp_flags: u8,
    pub pet_flags: u8,
    pub shapeshift_form: u8,
    pub attack_power: i32,
    pub attack_power_mod_pos: i32,
    pub attack_power_mod_neg: i32,
    pub attack_power_multiplier: f32,
    pub ranged_attack_power: i32,
    pub ranged_attack_power_mod_pos: i32,
    pub ranged_attack_power_mod_neg: i32,
    pub ranged_attack_power_multiplier: f32,
    pub set_attack_speed_aura: i32,
    pub lifesteal: f32,
    pub min_ranged_damage: f32,
    pub max_ranged_damage: f32,
    pub max_health_modifier: f32,
    pub hover_height: f32,
    pub min_item_level_cutoff: i32,
    pub min_item_level: i32,
    pub max_item_level: i32,
    pub wild_battle_pet_level: i32,
    pub battle_pet_companion_name_timestamp: u32,
    pub interact_spell_id: i32,
    pub scale_duration: i32,
    pub looks_like_mount_id: i32,
    pub looks_like_creature_id: i32,
    pub look_at_controller_id: i32,
    pub perks_vendor_item_id: i32,
    pub guild_guid: ObjectGuid,
    pub skinning_owner_guid: ObjectGuid,
    pub flight_capability_id: i32,
    pub glide_event_speed_divisor: f32,
    pub current_area_id: u32,
    pub combo_target: ObjectGuid,
    pub npc_flags: [u32; 2],
    pub power_regen_flat_modifier: [f32; 10],
    pub power_regen_interrupted_flat_modifier: [f32; 10],
    pub power: [i32; 10],
    pub max_power: [i32; 10],
    pub mod_power_regen: [f32; 10],
    pub virtual_items: [VisibleItemValuesUpdate; 3],
    pub attack_round_base_time: [u32; 2],
    pub stats: [i32; 5],
    pub stat_pos_buff: [i32; 5],
    pub stat_neg_buff: [i32; 5],
    pub resistances: [i32; 7],
    pub power_cost_modifier: [i32; 7],
    pub power_cost_multiplier: [f32; 7],
    pub resistance_buff_mods_positive: [i32; 7],
    pub resistance_buff_mods_negative: [i32; 7],
}

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ArenaCooldownValuesUpdate {
    pub arena_cooldown_mask: u32,
    pub spell_id: i32,
    pub item_id: i32,
    pub charges: i32,
    pub flags: u32,
    pub start_time: u32,
    pub end_time: u32,
    pub next_charge_time: u32,
    pub max_charges: u8,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DungeonScoreMapSummaryValuesUpdate {
    pub challenge_mode_id: i32,
    pub map_score: f32,
    pub best_run_level: i32,
    pub best_run_duration_ms: i32,
    pub finished_success: bool,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct DungeonScoreSummaryValuesUpdate {
    pub overall_score_current_season: f32,
    pub ladder_score_current_season: f32,
    pub runs: Vec<DungeonScoreMapSummaryValuesUpdate>,
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
pub struct ResearchValuesUpdate {
    pub research_project_id: i16,
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

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct SpellPctModByLabelValuesUpdate {
    pub mod_index: i32,
    pub modifier_value: f32,
    pub label_id: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SpellFlatModByLabelValuesUpdate {
    pub mod_index: i32,
    pub modifier_value: i32,
    pub label_id: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CategoryCooldownModValuesUpdate {
    pub spell_category_id: i32,
    pub mod_cooldown: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct WeeklySpellUseValuesUpdate {
    pub spell_category_id: i32,
    pub uses: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CompletedProjectValuesUpdate {
    pub completed_project_mask: u8,
    pub project_id: u32,
    pub first_completed: i64,
    pub completion_count: u32,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct ResearchHistoryValuesUpdate {
    pub research_history_mask: u8,
    pub completed_projects: Vec<CompletedProjectValuesUpdate>,
    pub completed_projects_update_mask: Option<Vec<u32>>,
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

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StableInfoValuesUpdate {
    pub stable_info_mask: u8,
    pub pets: Vec<StablePetInfoValuesUpdate>,
    pub pets_update_mask: Option<Vec<u32>>,
    pub stable_master: ObjectGuid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PerksVendorItemValuesUpdate {
    pub vendor_item_id: i32,
    pub mount_id: i32,
    pub battle_pet_species_id: i32,
    pub transmog_set_id: i32,
    pub item_modified_appearance_id: i32,
    pub field_14: i32,
    pub field_18: i32,
    pub price: i32,
    pub available_until: i64,
    pub disabled: bool,
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

impl Default for UnitDataValuesDeltaUpdate {
    fn default() -> Self {
        Self {
            changed_object_type_mask: VALUES_TYPE_UNIT,
            object_data: None,
            unit_data_mask: [0; 8],
            state_world_effect_ids: Vec::new(),
            passive_spells: Vec::new(),
            passive_spells_update_mask: None,
            world_effects: Vec::new(),
            world_effects_update_mask: None,
            channel_objects: Vec::new(),
            channel_objects_update_mask: None,
            health: 0,
            max_health: 0,
            display_id: 0,
            state_spell_visual_id: 0,
            state_anim_id: 0,
            state_anim_kit_id: 0,
            charm: ObjectGuid::EMPTY,
            summon: ObjectGuid::EMPTY,
            critter: ObjectGuid::EMPTY,
            charmed_by: ObjectGuid::EMPTY,
            summoned_by: ObjectGuid::EMPTY,
            created_by: ObjectGuid::EMPTY,
            demon_creator: ObjectGuid::EMPTY,
            look_at_controller_target: ObjectGuid::EMPTY,
            target: ObjectGuid::EMPTY,
            battle_pet_companion_guid: ObjectGuid::EMPTY,
            battle_pet_db_id: 0,
            channel_data: UnitChannelValuesUpdate::default(),
            summoned_by_home_realm: 0,
            race: 0,
            class_id: 0,
            player_class_id: 0,
            sex: 0,
            display_power: 0,
            override_display_power_id: 0,
            level: 0,
            effective_level: 0,
            content_tuning_id: 0,
            scaling_level_min: 0,
            scaling_level_max: 0,
            scaling_level_delta: 0,
            scaling_faction_group: 0,
            scaling_health_item_level_curve_id: 0,
            scaling_damage_item_level_curve_id: 0,
            faction_template: 0,
            flags: 0,
            flags2: 0,
            flags3: 0,
            aura_state: 0,
            ranged_attack_round_base_time: 0,
            bounding_radius: 0.0,
            combat_reach: 0.0,
            display_scale: 0.0,
            native_display_id: 0,
            native_display_scale: 0.0,
            mount_display_id: 0,
            min_damage: 0.0,
            max_damage: 0.0,
            min_off_hand_damage: 0.0,
            max_off_hand_damage: 0.0,
            stand_state: 0,
            pet_talent_points: 0,
            vis_flags: 0,
            anim_tier: 0,
            pet_number: 0,
            pet_name_timestamp: 0,
            pet_experience: 0,
            pet_next_level_experience: 0,
            mod_casting_speed: 0.0,
            mod_spell_haste: 0.0,
            mod_haste: 0.0,
            mod_ranged_haste: 0.0,
            mod_haste_regen: 0.0,
            mod_time_rate: 0.0,
            created_by_spell: 0,
            emote_state: 0,
            training_points_used: 0,
            training_points_total: 0,
            base_mana: 0,
            base_health: 0,
            sheathe_state: 0,
            pvp_flags: 0,
            pet_flags: 0,
            shapeshift_form: 0,
            attack_power: 0,
            attack_power_mod_pos: 0,
            attack_power_mod_neg: 0,
            attack_power_multiplier: 0.0,
            ranged_attack_power: 0,
            ranged_attack_power_mod_pos: 0,
            ranged_attack_power_mod_neg: 0,
            ranged_attack_power_multiplier: 0.0,
            set_attack_speed_aura: 0,
            lifesteal: 0.0,
            min_ranged_damage: 0.0,
            max_ranged_damage: 0.0,
            max_health_modifier: 0.0,
            hover_height: 0.0,
            min_item_level_cutoff: 0,
            min_item_level: 0,
            max_item_level: 0,
            wild_battle_pet_level: 0,
            battle_pet_companion_name_timestamp: 0,
            interact_spell_id: 0,
            scale_duration: 0,
            looks_like_mount_id: 0,
            looks_like_creature_id: 0,
            look_at_controller_id: 0,
            perks_vendor_item_id: 0,
            guild_guid: ObjectGuid::EMPTY,
            skinning_owner_guid: ObjectGuid::EMPTY,
            flight_capability_id: 0,
            glide_event_speed_divisor: 0.0,
            current_area_id: 0,
            combo_target: ObjectGuid::EMPTY,
            npc_flags: [0; 2],
            power_regen_flat_modifier: [0.0; 10],
            power_regen_interrupted_flat_modifier: [0.0; 10],
            power: [0; 10],
            max_power: [0; 10],
            mod_power_regen: [0.0; 10],
            virtual_items: [VisibleItemValuesUpdate::default(); 3],
            attack_round_base_time: [0; 2],
            stats: [0; 5],
            stat_pos_buff: [0; 5],
            stat_neg_buff: [0; 5],
            resistances: [0; 7],
            power_cost_modifier: [0; 7],
            power_cost_multiplier: [0.0; 7],
            resistance_buff_mods_positive: [0; 7],
            resistance_buff_mods_negative: [0; 7],
        }
    }
}

// ── ItemCreateData ──────────────────────────────────────────────────

/// Data needed to build an Item CREATE_OBJECT block for the client.
///
/// Each equipped item must exist as a separate game object so the client
/// can display it in the character panel / inventory UI.
pub struct ItemCreateData {
    pub item_guid: ObjectGuid,
    pub entry_id: i32,
    pub owner_guid: ObjectGuid,
    pub contained_in: ObjectGuid,
    pub stack_count: u32,
    pub dynamic_flags: u32,
    pub durability: u32,
    pub max_durability: u32,
    pub random_properties_seed: i32,
    pub random_properties_id: i32,
    pub context: u8,
}

// ── PlayerStatChanges ──────────────────────────────────────────────

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
    pub ranged_attack_power: i32,
    pub min_ranged_damage: f32,
    pub max_ranged_damage: f32,
    pub power0: i32,             // Mana/Rage/Energy current
    pub max_power0: i32,         // Mana/Rage/Energy max
    pub stats: [i32; 5],         // STR, AGI, STA, INT, SPI
    pub stat_pos_buff: [i32; 5], // gear bonuses shown as positive buffs
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

// ── PlayerCombatStats ──────────────────────────────────────────────

/// All combat-related stats computed from base stats + gear.
///
/// Passed as a single struct to `create_player` to avoid 20+ parameters.
#[derive(Debug, Clone, Copy)]
pub struct PlayerCombatStats {
    pub health: i64,
    pub max_health: i64,
    pub stats: [i32; 5],
    pub base_armor: i32,
    pub max_mana: i64,
    pub attack_power: i32,
    pub ranged_attack_power: i32,
    pub min_damage: f32,
    pub max_damage: f32,
    pub min_ranged_damage: f32,
    pub max_ranged_damage: f32,
    pub dodge_pct: f32,
    pub parry_pct: f32,
    pub crit_pct: f32,
    pub ranged_crit_pct: f32,
    pub spell_crit_pct: f32,
}

impl Default for PlayerCombatStats {
    fn default() -> Self {
        Self {
            health: 100,
            max_health: 100,
            stats: [0; 5],
            base_armor: 0,
            max_mana: 60,
            attack_power: 0,
            ranged_attack_power: 0,
            min_damage: 1.0,
            max_damage: 2.0,
            min_ranged_damage: 0.0,
            max_ranged_damage: 0.0,
            dodge_pct: 0.0,
            parry_pct: 0.0,
            crit_pct: 5.0,
            ranged_crit_pct: 5.0,
            spell_crit_pct: 0.0,
        }
    }
}

// ── PlayerCreateData ────────────────────────────────────────────────

/// Data needed to build a full player create packet for the client.
pub struct PlayerCreateData {
    pub guid: ObjectGuid,
    pub race: u8,
    pub class: u8,
    pub sex: u8,
    pub level: u8,
    pub display_id: u32,
    pub native_display_id: u32,
    pub health: i64,
    pub max_health: i64,
    pub faction_template: i32,
    pub zone_id: u32,
    /// Primary stats: [STR, AGI, STA, INT, SPI].
    pub stats: [i32; 5],
    /// Base armor (AGI * 2).
    pub base_armor: i32,
    /// Max mana from level stats (for caster classes).
    pub max_mana: i64,
    /// Melee attack power.
    pub attack_power: i32,
    /// Ranged attack power.
    pub ranged_attack_power: i32,
    /// Melee min/max damage (unarmed base).
    pub min_damage: f32,
    pub max_damage: f32,
    /// Ranged min/max damage.
    pub min_ranged_damage: f32,
    pub max_ranged_damage: f32,
    /// Dodge percentage.
    pub dodge_pct: f32,
    /// Parry percentage.
    pub parry_pct: f32,
    /// Melee crit percentage.
    pub crit_pct: f32,
    /// Ranged crit percentage.
    pub ranged_crit_pct: f32,
    /// Spell crit percentage (applied to all 7 schools).
    pub spell_crit_pct: f32,
    /// Visible equipment items (19 slots).
    /// Each entry: (ItemID, AppearanceModID, ItemVisual).
    /// Slots: Head(0), Neck(1), Shoulders(2), Shirt(3), Chest(4), Waist(5),
    /// Legs(6), Feet(7), Wrist(8), Hands(9), Finger1(10), Finger2(11),
    /// Trinket1(12), Trinket2(13), Cloak(14), MainHand(15), OffHand(16),
    /// Ranged(17), Tabard(18).
    pub visible_items: [(i32, u16, u16); 19],
    /// Inventory slots (141 entries) for ActivePlayerData.
    /// Slots 0-18 = equipped, 19-22 = bag containers, rest = backpack/bank.
    /// Each entry is an Item ObjectGuid (or EMPTY).
    pub inv_slots: [ObjectGuid; 141],
    /// ActivePlayerData::FarsightObject written after InvSlots in WriteCreate.
    pub farsight_object: ObjectGuid,
    /// Character's learned skills for the SkillInfo array (up to 256).
    /// Each entry: (skill_id, step, rank, starting_rank, max_rank, temp_bonus, perm_bonus).
    pub skill_info: Vec<(u16, u16, u16, u16, u16, i16, u16)>,
    /// Quest log slots — up to 25 active quests.
    /// (quest_id, state_flags, end_time, objective_progress[24])
    /// C# ref: QuestLog.WriteCreate — only sent with PartyMember flag (= self-view)
    pub quest_log: Vec<(u32, u32, i64, [u16; 24])>,
    /// Current money in copper (Coinage field in ActivePlayerData).
    pub coinage: u64,
    /// ActivePlayerData::WatchedFactionIndex.
    pub watched_faction_index: i32,
    /// ActivePlayerData::Heirlooms.
    pub heirlooms: Vec<i32>,
    /// ActivePlayerData::HeirloomFlags.
    pub heirloom_flags: Vec<u32>,
    /// ActivePlayerData::Toys.
    pub toys: Vec<i32>,
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

    /// Get the power value for slot 0, using real mana for caster classes.
    ///
    /// - Warrior (1): rage = 1000 (stored as 10×)
    /// - Rogue (4): energy = 100
    /// - DK (6): runic power = 1000 (stored as 10×)
    /// - All others: mana from `max_mana` field (loaded from player_levelstats)
    fn power_for_slot0(&self) -> i32 {
        match self.class {
            1 => 1000,                 // Warrior: rage
            4 => 100,                  // Rogue: energy
            6 => 1000,                 // DK: runic power
            _ => self.max_mana as i32, // Casters: real mana from DB
        }
    }

    /// Write the complete values block for CREATE (no change masks).
    ///
    /// Format: `[u32 size][u8 flags][ObjectData][UnitData][PlayerData][ActivePlayerData?]`
    pub fn write_values_create(&self, pkt: &mut WorldPacket, is_self: bool) {
        // Build into a temp buffer so we can prefix with size
        let mut buf = WorldPacket::new_empty();

        // UpdateFieldFlag: Owner=0x01 | PartyMember=0x02 for self (IsInSameRaidWith(self)==true)
        // C# ref: Player.GetUpdateFieldFlagsFor(target) — PartyMember set when in same raid
        let flags: u8 = if is_self { 0x03 } else { 0x00 }; // 0x01=Owner 0x02=PartyMember
        buf.write_uint8(flags);

        self.write_object_data(&mut buf);
        self.write_unit_data(&mut buf, flags);
        self.write_player_data(&mut buf, flags);
        if is_self {
            self.write_active_player_data(&mut buf);
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

    fn write_unit_data(&self, buf: &mut WorldPacket, flags: u8) {
        let is_owner = flags & 0x01 != 0;

        // Health / MaxHealth
        buf.write_int64(self.health);
        buf.write_int64(self.max_health);

        // DisplayId
        buf.write_int32(self.display_id as i32);

        // NpcFlags[2]
        buf.write_uint32(0);
        buf.write_uint32(0);

        // StateSpellVisualID, StateAnimID, StateAnimKitID
        buf.write_int32(0);
        buf.write_int32(0);
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
        let power0 = self.power_for_slot0();
        for i in 0..10 {
            if i == 0 {
                buf.write_int32(power0);
                buf.write_int32(power0);
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
        buf.write_uint32(0); // AuraState

        // AttackRoundBaseTime[2]
        buf.write_uint32(2000); // MainHand
        buf.write_uint32(2000); // OffHand

        // RangedAttackRoundBaseTime (Owner only)
        if is_owner {
            buf.write_uint32(0);
        }

        // BoundingRadius, CombatReach, DisplayScale
        buf.write_float(0.306); // BoundingRadius (human default)
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

        // ModCastingSpeed, ModSpellHaste, ModHaste, ModRangedHaste, ModHasteRegen, ModTimeRate
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
                buf.write_int32(0); // StatPosBuff
                buf.write_int32(0); // StatNegBuff
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

        // BaseMana — use real mana from stats store for caster classes
        buf.write_int32(self.power_for_slot0());

        // BaseHealth (Owner only)
        if is_owner {
            buf.write_int32(self.max_health as i32);
        }

        // SheatheState, PvpFlags, PetFlags, ShapeshiftForm
        buf.write_uint8(0); // SheatheState
        buf.write_uint8(0); // PvpFlags
        buf.write_uint8(0); // PetFlags
        buf.write_uint8(0); // ShapeshiftForm

        // AttackPower block (Owner only — 13 fields)
        if is_owner {
            buf.write_int32(self.attack_power); // AttackPower
            buf.write_int32(0); // AttackPowerModPos
            buf.write_int32(0); // AttackPowerModNeg
            buf.write_float(1.0); // AttackPowerMultiplier
            buf.write_int32(self.ranged_attack_power); // RangedAttackPower
            buf.write_int32(0); // RangedAttackPowerModPos
            buf.write_int32(0); // RangedAttackPowerModNeg
            buf.write_float(1.0); // RangedAttackPowerMultiplier
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
        buf.write_uint32(self.zone_id);

        // ComboTarget (Owner only)
        if is_owner {
            write_empty_guid(buf);
        }

        // Dynamic arrays (all empty — sizes were 0 above)
    }

    // ── PlayerData.WriteCreate ──────────────────────────────────

    fn write_player_data(&self, buf: &mut WorldPacket, flags: u8) {
        let is_party = flags & 0x02 != 0; // UpdateFieldFlag::PartyMember = 0x02

        // 3 PackedGuids
        write_empty_guid(buf); // DuelArbiter
        write_empty_guid(buf); // WowAccount
        write_empty_guid(buf); // LootTargetGUID

        // PlayerFlags, PlayerFlagsEx
        buf.write_uint32(0);
        buf.write_uint32(0);

        // GuildRankID, GuildDeleteDate, GuildLevel
        buf.write_int32(0);
        buf.write_uint32(0);
        buf.write_int32(0);

        // Customizations.Size
        buf.write_int32(0);

        // PartyType[2]
        buf.write_uint8(0);
        buf.write_uint8(0);

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
        // For self-view (is_self=true), C# always includes this (IsInSameRaidWith(self)==true).
        // C# ref: QuestLog.WriteCreate: int64 EndTime + int32 QuestID + uint32 StateFlags + uint16[24] ObjectiveProgress
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
        write_empty_guid(buf);

        // VisualItemReplacements.Size
        buf.write_int32(0);

        // Field_3120[19]
        for _ in 0..19 {
            buf.write_uint32(0);
        }

        // Dynamic arrays (all empty — Customizations, ArenaCooldowns, VisualItemReplacements)

        // DungeonScoreSummary.Write:
        //   OverallScoreCurrentSeason(f32), LadderScoreCurrentSeason(f32), Runs.Count(i32)
        buf.write_float(0.0);
        buf.write_float(0.0);
        buf.write_int32(0);
    }

    // ── ActivePlayerData.WriteCreate ────────────────────────────

    fn write_active_player_data(&self, buf: &mut WorldPacket) {
        // InvSlots[141]
        for i in 0..141 {
            buf.write_packed_guid(&self.inv_slots[i]);
        }

        // FarsightObject, SummonedBattlePetGUID
        buf.write_packed_guid(&self.farsight_object);
        write_empty_guid(buf);

        // KnownTitles.Size
        buf.write_uint32(0);

        // Coinage, XP, NextLevelXP, TrialXP
        buf.write_int64(self.coinage as i64);
        buf.write_int32(0);
        buf.write_int32(400); // NextLevelXP for level 1
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
        buf.write_float(0.0); // Block (need shield)
        buf.write_float(self.dodge_pct); // Dodge
        buf.write_float(self.dodge_pct); // DodgeFromAttr (same as dodge for display)
        buf.write_float(self.parry_pct); // Parry
        buf.write_float(self.parry_pct); // ParryFromAttr
        buf.write_float(self.crit_pct); // CritPercentage
        buf.write_float(self.ranged_crit_pct); // RangedCritPercentage
        buf.write_float(self.crit_pct); // OffhandCritPercentage

        // SpellCritPercentage[7], ModDamageDonePos[7], ModDamageDoneNeg[7], ModDamageDonePercent[7]
        for _ in 0..7 {
            buf.write_float(self.spell_crit_pct); // SpellCritPercentage per school
            buf.write_int32(0); // ModDamageDonePos (spell power from gear)
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

        // RestInfo[2] (each: i32 Threshold + u8 StateID)
        // StateID: 1=Rested, 2=Normal, 6=RAFLinked — must NOT be 0 (invalid)
        for _ in 0..2 {
            buf.write_int32(0); // Threshold (no rest bonus)
            buf.write_uint8(2); // StateID = Normal
        }

        // ModHealingDonePos, ModHealingPercent, ModHealingDonePercent, ModPeriodicHealingDonePercent
        buf.write_int32(0);
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
        for _ in 0..32 {
            buf.write_int32(0);
        }

        // MaxLevel, ScalingPlayerLevelDelta, MaxCreatureScalingLevel
        buf.write_int32(80);
        buf.write_int32(0);
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
        buf.write_int32(0);

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
        buf.write_int32(0);

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

        for value in &self.heirlooms {
            buf.write_int32(*value);
        }
        for value in &self.heirloom_flags {
            buf.write_uint32(*value);
        }
        for value in &self.toys {
            buf.write_int32(*value);
        }

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

        // Trailing bits + FlushBits
        buf.flush_bits();

        // SortBagsRightToLeft, InsertItemsLeftToRight, PetStable has value
        buf.write_bit(false);
        buf.write_bit(false);
        buf.write_bits(0, 1); // PetStable.HasValue = false
        buf.flush_bits();

        // ResearchHistory.WriteCreate: CompletedProjects.Size (i32)
        buf.write_int32(0);

        // FrozenPerksVendorItem.Write: 7 i32 + 1 i64 + 1 bit
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

        // CharacterRestrictions (size 0, no data)
        // TraitConfigs (size 0, no data)
        // PetStable (not present)

        buf.flush_bits();
    }
}

// ── Helpers ─────────────────────────────────────────────────────────

/// Write an empty packed GUID (2 zero mask bytes).
fn write_empty_guid(buf: &mut WorldPacket) {
    buf.write_packed_guid(&ObjectGuid::EMPTY);
}

/// Get power type for a class (0=mana, 1=rage, 3=energy).
fn power_type_for_class(class: u8) -> u8 {
    match class {
        1 => 1,  // Warrior → Rage
        4 => 3,  // Rogue → Energy
        11 => 0, // Druid → Mana
        6 => 5,  // DeathKnight → Runic Power
        _ => 0,  // Default → Mana
    }
}

/// Get starting max power for a class at a given level.
fn max_power_for_class(class: u8, _level: u8) -> i32 {
    match class {
        1 => 1000, // Warrior: 1000 rage (stored as 10x)
        4 => 100,  // Rogue: 100 energy
        6 => 1000, // DK: 1000 runic power (stored as 10x)
        _ => 60,   // Casters: base mana
    }
}

// ── CreatureCreateData ──────────────────────────────────────────────

/// Data needed to build a creature create packet for the client.
#[derive(Debug, Clone)]
pub struct CreatureCreateData {
    pub guid: ObjectGuid,
    pub entry: u32,
    pub display_id: u32,
    pub native_display_id: u32,
    pub health: i64,
    pub max_health: i64,
    pub level: u8,
    pub faction_template: i32,
    pub npc_flags: u64,
    pub unit_flags: u32,
    pub unit_flags2: u32,
    pub unit_flags3: u32,
    pub damage_school: u8,
    pub scale: f32,
    pub unit_class: u8,
    pub base_attack_time: u32,
    pub ranged_attack_time: u32,
    pub zone_id: u32,
    /// Speed rate from creature_template.speed_walk (1.0 = default).
    pub speed_walk_rate: f32,
    /// Speed rate from creature_template.speed_run (1.14286 = default).
    pub speed_run_rate: f32,
    pub ai_anim_kit_id: u16,
    pub movement_anim_kit_id: u16,
    pub melee_anim_kit_id: u16,
}

impl CreatureCreateData {
    /// Write the complete values block for CREATE (no change masks).
    ///
    /// For creatures: ObjectData + UnitData only (no PlayerData/ActivePlayerData).
    /// Flags = 0x00 (not owner), so many conditional blocks are skipped.
    pub fn write_values_create(&self, pkt: &mut WorldPacket) {
        let mut buf = WorldPacket::new_empty();

        // UpdateFieldFlag: 0x00 for creatures viewed by a non-owner
        buf.write_uint8(0x00);

        self.write_object_data(&mut buf);
        self.write_unit_data(&mut buf);

        let data = buf.into_data();
        pkt.write_uint32(data.len() as u32);
        pkt.write_bytes(&data);
    }

    fn write_object_data(&self, buf: &mut WorldPacket) {
        buf.write_int32(self.entry as i32); // EntryId (non-zero for creatures)
        buf.write_uint32(0); // DynamicFlags
        buf.write_float(self.scale); // Scale
    }

    fn write_unit_data(&self, buf: &mut WorldPacket) {
        // Health / MaxHealth
        buf.write_int64(self.health);
        buf.write_int64(self.max_health);

        // DisplayId
        buf.write_int32(self.display_id as i32);

        // NpcFlags[2] (split 64-bit into two u32s)
        buf.write_uint32(self.npc_flags as u32);
        buf.write_uint32((self.npc_flags >> 32) as u32);

        // StateSpellVisualID, StateAnimID, StateAnimKitID
        buf.write_int32(0);
        buf.write_int32(0);
        buf.write_int32(0);

        // StateWorldEffectIDs.Count
        buf.write_int32(0);

        // 9 PackedGuids (no Critter — that's Owner-only)
        for _ in 0..9 {
            write_empty_guid(buf);
        }

        // BattlePetDBID
        buf.write_uint64(0);

        // ChannelData: SpellID + SpellXSpellVisualID
        buf.write_int32(0);
        buf.write_int32(0);

        // SummonedByHomeRealm
        buf.write_uint32(0);

        // Race, ClassId, PlayerClassId, Sex, DisplayPower
        buf.write_uint8(0); // Race (0 for creatures)
        buf.write_uint8(self.unit_class);
        buf.write_uint8(0); // PlayerClassId (0 for creatures)
        buf.write_uint8(0); // Sex
        buf.write_uint8(0); // DisplayPower (mana)

        // OverrideDisplayPowerID
        buf.write_int32(0);

        // NO PowerRegen (Owner-only)

        // Power[10], MaxPower[10], ModPowerRegen[10]
        for _ in 0..10 {
            buf.write_int32(0); // Power
            buf.write_int32(0); // MaxPower
            buf.write_float(0.0); // ModPowerRegen
        }

        // Level, EffectiveLevel, ContentTuningID, Scaling fields (9x i32)
        buf.write_int32(self.level as i32);
        buf.write_int32(self.level as i32);
        buf.write_int32(0); // ContentTuningID
        buf.write_int32(0); // ScalingLevelMin
        buf.write_int32(0); // ScalingLevelMax
        buf.write_int32(0); // ScalingLevelDelta
        buf.write_int32(0); // ScalingFactionGroup
        buf.write_int32(0); // ScalingHealthItemLevelCurveID
        buf.write_int32(0); // ScalingDamageItemLevelCurveID

        // FactionTemplate
        buf.write_int32(self.faction_template);

        // VirtualItems[3]
        for _ in 0..3 {
            buf.write_int32(0);
            buf.write_uint16(0);
            buf.write_uint16(0);
        }

        // Flags, Flags2, Flags3, AuraState
        buf.write_uint32(self.unit_flags);
        buf.write_uint32(self.unit_flags2);
        buf.write_uint32(self.unit_flags3);
        buf.write_uint32(0); // AuraState

        // AttackRoundBaseTime[2]
        buf.write_uint32(self.base_attack_time);
        buf.write_uint32(self.base_attack_time);

        // NO RangedAttackRoundBaseTime (Owner-only)

        // BoundingRadius, CombatReach, DisplayScale
        buf.write_float(0.389); // BoundingRadius (common default)
        buf.write_float(1.5); // CombatReach
        buf.write_float(1.0); // DisplayScale

        // NativeDisplayID, NativeXDisplayScale, MountDisplayID
        buf.write_int32(self.native_display_id as i32);
        buf.write_float(1.0);
        buf.write_int32(0);

        // NO damage floats (Owner|Empath only)

        // StandState, PetTalentPoints, VisFlags, AnimTier
        buf.write_uint8(0);
        buf.write_uint8(0);
        buf.write_uint8(0);
        buf.write_uint8(0);

        // PetNumber, PetNameTimestamp, PetExperience, PetNextLevelExperience
        buf.write_int32(0);
        buf.write_int32(0);
        buf.write_int32(0);
        buf.write_int32(0);

        // ModCastingSpeed, ModSpellHaste, ModHaste, ModRangedHaste, ModHasteRegen, ModTimeRate
        buf.write_float(1.0);
        buf.write_float(1.0);
        buf.write_float(1.0);
        buf.write_float(1.0);
        buf.write_float(1.0);
        buf.write_float(1.0);

        // CreatedBySpell, EmoteState
        buf.write_int32(0);
        buf.write_int32(0);

        // TrainingPointsUsed, TrainingPointsTotal
        buf.write_int16(0);
        buf.write_int16(0);

        // NO Stats/StatBuff (Owner-only)
        // NO Resistances (Owner|Empath only)
        // NO PowerCostModifier/Multiplier (Owner-only)

        // ResistanceBuffModsPositive[7] + Negative[7]
        for _ in 0..7 {
            buf.write_int32(0);
            buf.write_int32(0);
        }

        // BaseMana
        buf.write_int32(0);

        // NO BaseHealth (Owner-only)

        // SheatheState, PvpFlags, PetFlags, ShapeshiftForm
        buf.write_uint8(0);
        buf.write_uint8(0);
        buf.write_uint8(0);
        buf.write_uint8(0);

        // NO AttackPower block (Owner-only)

        // HoverHeight + misc fields
        buf.write_float(1.0);
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
        buf.write_uint32(self.zone_id);

        // NO ComboTarget (Owner-only)
    }
}

// ── UpdateBlock ─────────────────────────────────────────────────────

// ── GameObjectCreateData ──────────────────────────────────────────

/// Data needed to build a gameobject create packet for the client.
pub struct GameObjectCreateData {
    pub guid: ObjectGuid,
    pub entry: u32,
    pub dynamic_flags: u32,
    pub display_id: u32,
    pub go_type: u8,
    pub position: Position,
    pub rotation: [f32; 4], // rotation0..3 (quaternion)
    pub anim_progress: u8,
    pub state: i8,
    pub created_by: ObjectGuid,
    pub faction_template: i32,
    pub gameobject_flags: u32,
    pub scale: f32,
}

impl GameObjectCreateData {
    /// Write the values block for CREATE.
    ///
    /// For GameObjects: ObjectData + GameObjectFieldData (no UnitData/PlayerData).
    pub fn write_values_create(&self, pkt: &mut WorldPacket) {
        let mut buf = WorldPacket::new_empty();

        // UpdateFieldFlag: 0x00 for non-owner
        buf.write_uint8(0x00);

        // ObjectFieldData.WriteCreate
        buf.write_int32(self.entry as i32); // EntryId
        buf.write_uint32(self.dynamic_flags); // DynamicFlags
        buf.write_float(self.scale); // Scale

        // GameObjectFieldData.WriteCreate (matches C# GameObjectFieldData.WriteCreate)
        buf.write_int32(self.display_id as i32); // DisplayID
        buf.write_int32(0); // SpellVisualID
        buf.write_int32(0); // StateSpellVisualID
        buf.write_int32(0); // SpawnTrackingStateAnimID
        buf.write_int32(0); // SpawnTrackingStateAnimKitID
        buf.write_int32(0); // StateWorldEffectIDs.Count
        // No StateWorldEffectIDs entries (count=0)
        buf.write_packed_guid(&self.created_by); // CreatedBy
        write_empty_guid(&mut buf); // GuildGUID
        buf.write_uint32(self.gameobject_flags); // Flags
        // ParentRotation (Quaternion: x, y, z, w)
        // In C# this comes from gameobject_addon.parent_rotation, NOT from gameobject.rotation0-3.
        // For most GameObjects it's the identity quaternion (0, 0, 0, 1).
        // Only some transports have non-standard parent rotation.
        buf.write_float(0.0); // ParentRotation.X
        buf.write_float(0.0); // ParentRotation.Y
        buf.write_float(0.0); // ParentRotation.Z
        buf.write_float(1.0); // ParentRotation.W
        buf.write_int32(self.faction_template); // FactionTemplate
        buf.write_uint32(0); // Level
        buf.write_int8(self.state); // State
        buf.write_int8(self.go_type as i8); // TypeID (gameobject type)
        buf.write_uint8(self.anim_progress); // PercentHealth (anim progress)
        buf.write_int32(0); // ArtKit
        buf.write_int32(0); // EnableDoodadSets.Size
        buf.write_int32(0); // CustomParam
        buf.write_int32(0); // WorldEffects.Size
        // No EnableDoodadSets/WorldEffects entries

        let data = buf.into_data();
        pkt.write_uint32(data.len() as u32);
        pkt.write_bytes(&data);
    }

    /// Pack the local rotation as a 64-bit integer for the Rotation flag.
    ///
    /// Matches C# `GameObject.UpdatePackedRotation()` exactly:
    /// ```csharp
    /// const int PACK_YZ = 1 << 20;          // 1,048,576
    /// const int PACK_X  = PACK_YZ << 1;     // 2,097,152
    /// const int PACK_YZ_MASK = (PACK_YZ << 1) - 1;  // 0x1FFFFF (21 bits)
    /// const int PACK_X_MASK  = (PACK_X << 1) - 1;   // 0x3FFFFF (22 bits)
    /// sbyte w_sign = (sbyte)(W >= 0 ? 1 : -1);
    /// long x = (int)(X * PACK_X)  * w_sign & PACK_X_MASK;
    /// long y = (int)(Y * PACK_YZ) * w_sign & PACK_YZ_MASK;
    /// long z = (int)(Z * PACK_YZ) * w_sign & PACK_YZ_MASK;
    /// result = z | (y << 21) | (x << 42);
    /// ```
    /// Layout: bits[0:20]=Z(21), bits[21:41]=Y(21), bits[42:63]=X(22).
    pub fn packed_rotation(&self) -> i64 {
        const PACK_YZ: i64 = 1 << 20; // 1,048,576
        const PACK_X: i64 = PACK_YZ << 1; // 2,097,152
        const PACK_YZ_MASK: i64 = (PACK_YZ << 1) - 1; // 0x1FFFFF
        const PACK_X_MASK: i64 = (PACK_X << 1) - 1; // 0x3FFFFF

        // Normalize quaternion (C# SetLocalRotation does this before packing)
        let (rx, ry, rz, rw) = {
            let dot = self.rotation[0] * self.rotation[0]
                + self.rotation[1] * self.rotation[1]
                + self.rotation[2] * self.rotation[2]
                + self.rotation[3] * self.rotation[3];
            let inv_len = 1.0 / dot.sqrt();
            (
                self.rotation[0] * inv_len,
                self.rotation[1] * inv_len,
                self.rotation[2] * inv_len,
                self.rotation[3] * inv_len,
            )
        };

        let w_sign: i32 = if rw >= 0.0 { 1 } else { -1 };

        let x = ((rx * PACK_X as f32) as i32 as i64) * w_sign as i64 & PACK_X_MASK;
        let y = ((ry * PACK_YZ as f32) as i32 as i64) * w_sign as i64 & PACK_YZ_MASK;
        let z = ((rz * PACK_YZ as f32) as i32 as i64) * w_sign as i64 & PACK_YZ_MASK;

        z | (y << 21) | (x << 42)
    }
}

// ── DynamicObjectCreateData ────────────────────────────────────────

/// Data needed to build a DynamicObject create packet for the client.
///
/// C++ anchors:
/// - `DynamicObject::DynamicObject(bool)` sets Stationary create flag.
/// - `DynamicObject::BuildValuesCreate` writes ObjectData then DynamicObjectData.
pub struct DynamicObjectCreateData {
    pub guid: ObjectGuid,
    pub entry_id: u32,
    pub dynamic_flags: u32,
    pub scale: f32,
    pub position: Position,
    pub caster: ObjectGuid,
    pub dynamic_object_type: u8,
    pub spell_visual_id: i32,
    pub spell_id: i32,
    pub radius: f32,
    pub cast_time_ms: u32,
}

impl DynamicObjectCreateData {
    /// Write the create-time values block: `[u32 size][u8 flags][ObjectData][DynamicObjectData]`.
    ///
    /// This is a CREATE values section, not an `UpdateType::Values` block; it intentionally
    /// does not write a packed object GUID or update masks inside the values payload.
    pub fn write_values_create(&self, pkt: &mut WorldPacket) {
        let mut buf = WorldPacket::new_empty();

        // UpdateFieldFlag: 0x00 for non-owner.
        buf.write_uint8(0x00);

        // ObjectData::WriteCreate.
        buf.write_int32(self.entry_id as i32);
        buf.write_uint32(self.dynamic_flags);
        buf.write_float(self.scale);

        // DynamicObjectData::WriteCreate.
        buf.write_packed_guid(&self.caster);
        buf.write_uint8(self.dynamic_object_type);
        buf.write_int32(self.spell_visual_id);
        buf.write_int32(self.spell_id);
        buf.write_float(self.radius);
        buf.write_uint32(self.cast_time_ms);

        let data = buf.into_data();
        pkt.write_uint32(data.len() as u32);
        pkt.write_bytes(&data);
    }
}

/// A single update block within an UpdateObject packet.
pub enum UpdateBlock {
    CreateObject {
        update_type: UpdateType,
        guid: ObjectGuid,
        type_id: TypeId,
        movement: Option<MovementBlock>,
        create_data: PlayerCreateData,
        is_self: bool,
    },
    CreateCreature {
        guid: ObjectGuid,
        movement: MovementBlock,
        create_data: CreatureCreateData,
    },
    CreateGameObject {
        guid: ObjectGuid,
        create_data: GameObjectCreateData,
    },
    CreateDynamicObject {
        guid: ObjectGuid,
        create_data: DynamicObjectCreateData,
    },
    CreateItem {
        guid: ObjectGuid,
        create_data: ItemCreateData,
    },
    /// VALUES update for an item: currently only StackCount is needed by direct inventory stores.
    ItemValuesUpdate { guid: ObjectGuid, stack_count: u32 },
    /// VALUES update for a player: only changed InvSlots, VisibleItems, VirtualItems.
    PlayerValuesUpdate {
        guid: ObjectGuid,
        /// Changed InvSlots: (slot_index 0-140, new ObjectGuid or EMPTY).
        inv_slot_changes: Vec<(u8, ObjectGuid)>,
        /// Changed BuybackPrice/BuybackTimestamp rows: (buyback slot 94-105, price, timestamp).
        buyback_changes: Vec<(u8, u32, i64)>,
        /// Changed VisibleItems in PlayerData: (slot 0-18, item_id, appearance_mod, visual).
        visible_item_changes: Vec<(u8, i32, u16, u16)>,
        /// Changed VirtualItems in UnitData: (index 0-2 for MH/OH/Ranged, item_id, app, visual).
        virtual_item_changes: Vec<(u8, i32, u16, u16)>,
        /// Optional stat changes to include in UnitData section.
        stat_changes: Option<PlayerStatChanges>,
        /// Optional coinage update (ActivePlayerData.Coinage field, block 0 bit 28).
        coinage_change: Option<u64>,
    },
    /// VALUES update for a creature: only health and max health.
    CreatureHealthUpdate {
        guid: ObjectGuid,
        health: i64,
        max_health: i64,
    },
    /// Generic ObjectData VALUES update.
    ObjectValuesUpdate {
        guid: ObjectGuid,
        data: ObjectDataValuesUpdate,
    },
    /// VALUES update for DynamicObjectData.
    DynamicObjectValuesUpdate {
        guid: ObjectGuid,
        data: DynamicObjectDataValuesUpdate,
    },
    /// VALUES update for SceneObjectData.
    SceneObjectValuesUpdate {
        guid: ObjectGuid,
        data: SceneObjectDataValuesUpdate,
    },
    /// VALUES update for ConversationData.
    ConversationValuesUpdate {
        guid: ObjectGuid,
        data: ConversationDataValuesUpdate,
    },
    /// VALUES update for GameObjectData.
    GameObjectValuesUpdate {
        guid: ObjectGuid,
        data: GameObjectDataValuesUpdate,
    },
    /// VALUES update for CorpseData.
    CorpseValuesUpdate {
        guid: ObjectGuid,
        data: CorpseDataValuesUpdate,
    },
    /// VALUES update for AreaTriggerData.
    AreaTriggerValuesUpdate {
        guid: ObjectGuid,
        data: AreaTriggerDataValuesUpdate,
    },
    /// VALUES update for ItemData.
    FullItemValuesUpdate {
        guid: ObjectGuid,
        data: ItemDataValuesDeltaUpdate,
    },
    /// VALUES update for UnitData.
    UnitValuesUpdate {
        guid: ObjectGuid,
        data: UnitDataValuesDeltaUpdate,
    },
    /// VALUES update for PlayerData, optionally including UnitData.
    FullPlayerValuesUpdate {
        guid: ObjectGuid,
        data: PlayerDataValuesDeltaUpdate,
    },
    /// VALUES update for ActivePlayerData.
    FullActivePlayerValuesUpdate {
        guid: ObjectGuid,
        data: ActivePlayerDataValuesUpdate,
    },
    /// VALUES update for ContainerData, optionally including ItemData.
    ContainerValuesUpdate {
        guid: ObjectGuid,
        data: ContainerDataValuesUpdate,
    },
    /// Out-of-range destroy (removes object from client view without full destroy).
    DestroyOutOfRange { guid: ObjectGuid },
}

// ── UpdateObject (SMSG_UPDATE_OBJECT) ───────────────────────────────

/// The main update packet used to create, update, or destroy objects.
///
/// Wire format (matches C# UpdateData.BuildPacket + UpdateObject.Write):
/// ```text
/// [u32] NumObjUpdates
/// [u16] MapID
/// [byte[]] Data — built from:
///   [bit] HasDestroyOrOutOfRange
///     if true: [u16 destroyCount][i32 totalCount][PackedGuid... destroy][PackedGuid... oor]
///   [i32] dataBlockSize
///   [bytes] concatenated update blocks
/// ```
pub struct UpdateObject {
    pub map_id: u16,
    pub num_updates: u32,
    pub destroy_guids: Vec<ObjectGuid>,
    pub out_of_range_guids: Vec<ObjectGuid>,
    pub blocks: Vec<UpdateBlock>,
}

impl UpdateObject {
    /// Create a creature spawn block.
    ///
    /// Speed rates from `creature_template` are multiplied by base speeds:
    /// walk = rate × 2.5, run = rate × 7.0.
    pub fn create_creature_block(
        create_data: CreatureCreateData,
        position: &Position,
    ) -> UpdateBlock {
        let walk_speed = create_data.speed_walk_rate * 2.5;
        let run_speed = create_data.speed_run_rate * 7.0;
        let movement = MovementBlock {
            position: *position,
            walk_speed,
            run_speed,
            ..Default::default()
        };
        UpdateBlock::CreateCreature {
            guid: create_data.guid,
            movement,
            create_data,
        }
    }

    /// Create a gameobject spawn block.
    pub fn create_gameobject_block(create_data: GameObjectCreateData) -> UpdateBlock {
        UpdateBlock::CreateGameObject {
            guid: create_data.guid,
            create_data,
        }
    }

    /// Create a dynamic object spawn block.
    pub fn create_dynamic_object_block(create_data: DynamicObjectCreateData) -> UpdateBlock {
        UpdateBlock::CreateDynamicObject {
            guid: create_data.guid,
            create_data,
        }
    }

    /// Create a batched UpdateObject with mixed world-object create blocks.
    pub fn create_world_objects(blocks: Vec<UpdateBlock>, map_id: u16) -> Self {
        Self {
            map_id,
            num_updates: blocks.len() as u32,
            destroy_guids: Vec::new(),
            out_of_range_guids: Vec::new(),
            blocks,
        }
    }

    /// Create a batched UpdateObject with multiple creature blocks.
    pub fn create_creatures(blocks: Vec<UpdateBlock>, map_id: u16) -> Self {
        Self {
            map_id,
            num_updates: blocks.len() as u32,
            destroy_guids: Vec::new(),
            out_of_range_guids: Vec::new(),
            blocks,
        }
    }

    /// Create a player create packet for login.
    pub fn create_player(
        guid: ObjectGuid,
        race: u8,
        class: u8,
        sex: u8,
        level: u8,
        display_id: u32,
        position: &Position,
        map_id: u16,
        zone_id: u32,
        is_self: bool,
        visible_items: [(i32, u16, u16); 19],
        inv_slots: [ObjectGuid; 141],
        combat: PlayerCombatStats,
        skill_info: Vec<(u16, u16, u16, u16, u16, i16, u16)>,
        coinage: u64,
        quest_log: Vec<(u32, u32, i64, [u16; 24])>,
    ) -> Self {
        let faction = PlayerCreateData::faction_for_race(race);

        let create_data = PlayerCreateData {
            guid,
            race,
            class,
            sex,
            level,
            display_id,
            native_display_id: display_id,
            health: combat.health,
            max_health: combat.max_health,
            faction_template: faction,
            zone_id,
            stats: combat.stats,
            base_armor: combat.base_armor,
            max_mana: combat.max_mana,
            attack_power: combat.attack_power,
            ranged_attack_power: combat.ranged_attack_power,
            min_damage: combat.min_damage,
            max_damage: combat.max_damage,
            min_ranged_damage: combat.min_ranged_damage,
            max_ranged_damage: combat.max_ranged_damage,
            dodge_pct: combat.dodge_pct,
            parry_pct: combat.parry_pct,
            crit_pct: combat.crit_pct,
            ranged_crit_pct: combat.ranged_crit_pct,
            spell_crit_pct: combat.spell_crit_pct,
            visible_items,
            inv_slots,
            farsight_object: ObjectGuid::EMPTY,
            skill_info,
            coinage,
            watched_faction_index: -1,
            heirlooms: Vec::new(),
            heirloom_flags: Vec::new(),
            toys: Vec::new(),
            quest_log,
        };

        let movement = MovementBlock {
            position: *position,
            ..Default::default()
        };

        let type_id = if is_self {
            TypeId::ActivePlayer
        } else {
            TypeId::Player
        };

        Self {
            map_id,
            num_updates: 1,
            destroy_guids: Vec::new(),
            out_of_range_guids: Vec::new(),
            blocks: vec![UpdateBlock::CreateObject {
                update_type: UpdateType::CreateObject2,
                guid,
                type_id,
                movement: Some(movement),
                create_data,
                is_self,
            }],
        }
    }

    /// Populate account collection dynamic fields on the player CREATE block.
    ///
    /// C++ `CollectionMgr::LoadToys` / `LoadHeirlooms` mutates
    /// `ActivePlayerData` before the create values are written during login.
    pub fn set_player_collection_dynamic_fields_like_cpp(
        &mut self,
        toys: Vec<i32>,
        heirlooms: Vec<(i32, u32)>,
    ) {
        for block in &mut self.blocks {
            if let UpdateBlock::CreateObject {
                create_data,
                is_self: true,
                ..
            } = block
            {
                create_data.toys = toys;
                create_data.heirlooms = heirlooms.iter().map(|(item_id, _)| *item_id).collect();
                create_data.heirloom_flags =
                    heirlooms.into_iter().map(|(_, flags)| flags).collect();
                return;
            }
        }
    }

    /// Create a player VALUES update for changed inventory fields.
    ///
    /// Used when items are swapped/equipped/unequipped to update the client's
    /// InvSlots (ActivePlayerData) and VisibleItems (PlayerData) without
    /// recreating the entire player object.
    pub fn player_values_update(
        guid: ObjectGuid,
        map_id: u16,
        inv_slot_changes: Vec<(u8, ObjectGuid)>,
        visible_item_changes: Vec<(u8, i32, u16, u16)>,
        virtual_item_changes: Vec<(u8, i32, u16, u16)>,
    ) -> Self {
        Self {
            map_id,
            num_updates: 1,
            destroy_guids: Vec::new(),
            out_of_range_guids: Vec::new(),
            blocks: vec![UpdateBlock::PlayerValuesUpdate {
                guid,
                inv_slot_changes,
                buyback_changes: Vec::new(),
                visible_item_changes,
                virtual_item_changes,
                stat_changes: None,
                coinage_change: None,
            }],
        }
    }

    /// Create a player VALUES update for changed inventory and buyback fields.
    pub fn player_values_buyback_update(
        guid: ObjectGuid,
        map_id: u16,
        inv_slot_changes: Vec<(u8, ObjectGuid)>,
        buyback_changes: Vec<(u8, u32, i64)>,
        coinage: Option<u64>,
    ) -> Self {
        Self {
            map_id,
            num_updates: 1,
            destroy_guids: Vec::new(),
            out_of_range_guids: Vec::new(),
            blocks: vec![UpdateBlock::PlayerValuesUpdate {
                guid,
                inv_slot_changes,
                buyback_changes,
                visible_item_changes: Vec::new(),
                virtual_item_changes: Vec::new(),
                stat_changes: None,
                coinage_change: coinage,
            }],
        }
    }

    /// Create a VALUES update for player coinage + optional inv slot change.
    ///
    /// Used after buy/sell to update the client's displayed gold and inventory.
    pub fn player_money_update(
        guid: ObjectGuid,
        map_id: u16,
        coinage: u64,
        inv_slot_change: Option<(u8, ObjectGuid)>,
    ) -> Self {
        Self {
            map_id,
            num_updates: 1,
            destroy_guids: Vec::new(),
            out_of_range_guids: Vec::new(),
            blocks: vec![UpdateBlock::PlayerValuesUpdate {
                guid,
                inv_slot_changes: inv_slot_change.map(|c| vec![c]).unwrap_or_default(),
                buyback_changes: Vec::new(),
                visible_item_changes: Vec::new(),
                virtual_item_changes: Vec::new(),
                stat_changes: None,
                coinage_change: Some(coinage),
            }],
        }
    }

    /// Create a VALUES update for player stats only (after equip/desequip).
    pub fn player_stat_update(guid: ObjectGuid, map_id: u16, changes: PlayerStatChanges) -> Self {
        Self {
            map_id,
            num_updates: 1,
            destroy_guids: Vec::new(),
            out_of_range_guids: Vec::new(),
            blocks: vec![UpdateBlock::PlayerValuesUpdate {
                guid,
                inv_slot_changes: Vec::new(),
                buyback_changes: Vec::new(),
                visible_item_changes: Vec::new(),
                virtual_item_changes: Vec::new(),
                stat_changes: Some(changes),
                coinage_change: None,
            }],
        }
    }

    /// Create a VALUES update for the base `UF::ObjectData` section.
    ///
    /// The mask follows TrinityCore `UF::ObjectData`: bit 0 is the parent bit,
    /// bits 1/2/3 are EntryID/DynamicFlags/Scale.
    pub fn object_values_update(
        guid: ObjectGuid,
        map_id: u16,
        data: ObjectDataValuesUpdate,
    ) -> Self {
        Self {
            map_id,
            num_updates: 1,
            destroy_guids: Vec::new(),
            out_of_range_guids: Vec::new(),
            blocks: vec![UpdateBlock::ObjectValuesUpdate { guid, data }],
        }
    }

    /// Create a VALUES update for `UF::DynamicObjectData`.
    pub fn dynamic_object_values_update(
        guid: ObjectGuid,
        map_id: u16,
        data: DynamicObjectDataValuesUpdate,
    ) -> Self {
        Self {
            map_id,
            num_updates: 1,
            destroy_guids: Vec::new(),
            out_of_range_guids: Vec::new(),
            blocks: vec![UpdateBlock::DynamicObjectValuesUpdate { guid, data }],
        }
    }

    /// Create a VALUES update for `UF::SceneObjectData`.
    pub fn scene_object_values_update(
        guid: ObjectGuid,
        map_id: u16,
        data: SceneObjectDataValuesUpdate,
    ) -> Self {
        Self {
            map_id,
            num_updates: 1,
            destroy_guids: Vec::new(),
            out_of_range_guids: Vec::new(),
            blocks: vec![UpdateBlock::SceneObjectValuesUpdate { guid, data }],
        }
    }

    /// Create a VALUES update for `UF::ConversationData`.
    pub fn conversation_values_update(
        guid: ObjectGuid,
        map_id: u16,
        data: ConversationDataValuesUpdate,
    ) -> Self {
        Self {
            map_id,
            num_updates: 1,
            destroy_guids: Vec::new(),
            out_of_range_guids: Vec::new(),
            blocks: vec![UpdateBlock::ConversationValuesUpdate { guid, data }],
        }
    }

    /// Create a VALUES update for `UF::GameObjectData`.
    pub fn game_object_values_update(
        guid: ObjectGuid,
        map_id: u16,
        data: GameObjectDataValuesUpdate,
    ) -> Self {
        Self {
            map_id,
            num_updates: 1,
            destroy_guids: Vec::new(),
            out_of_range_guids: Vec::new(),
            blocks: vec![UpdateBlock::GameObjectValuesUpdate { guid, data }],
        }
    }

    /// Create a VALUES update for `UF::CorpseData`.
    pub fn corpse_values_update(
        guid: ObjectGuid,
        map_id: u16,
        data: CorpseDataValuesUpdate,
    ) -> Self {
        Self {
            map_id,
            num_updates: 1,
            destroy_guids: Vec::new(),
            out_of_range_guids: Vec::new(),
            blocks: vec![UpdateBlock::CorpseValuesUpdate { guid, data }],
        }
    }

    /// Create a VALUES update for `UF::AreaTriggerData`.
    pub fn area_trigger_values_update(
        guid: ObjectGuid,
        map_id: u16,
        data: AreaTriggerDataValuesUpdate,
    ) -> Self {
        Self {
            map_id,
            num_updates: 1,
            destroy_guids: Vec::new(),
            out_of_range_guids: Vec::new(),
            blocks: vec![UpdateBlock::AreaTriggerValuesUpdate { guid, data }],
        }
    }

    /// Create a full VALUES update for `UF::ItemData`.
    pub fn full_item_values_update(
        guid: ObjectGuid,
        map_id: u16,
        data: ItemDataValuesDeltaUpdate,
    ) -> Self {
        Self {
            map_id,
            num_updates: 1,
            destroy_guids: Vec::new(),
            out_of_range_guids: Vec::new(),
            blocks: vec![UpdateBlock::FullItemValuesUpdate { guid, data }],
        }
    }

    /// Create a full VALUES update for `UF::UnitData`.
    pub fn unit_values_update(
        guid: ObjectGuid,
        map_id: u16,
        data: UnitDataValuesDeltaUpdate,
    ) -> Self {
        Self {
            map_id,
            num_updates: 1,
            destroy_guids: Vec::new(),
            out_of_range_guids: Vec::new(),
            blocks: vec![UpdateBlock::UnitValuesUpdate { guid, data }],
        }
    }

    /// Create a full VALUES update for `UF::PlayerData`.
    pub fn full_player_values_update(
        guid: ObjectGuid,
        map_id: u16,
        data: PlayerDataValuesDeltaUpdate,
    ) -> Self {
        Self {
            map_id,
            num_updates: 1,
            destroy_guids: Vec::new(),
            out_of_range_guids: Vec::new(),
            blocks: vec![UpdateBlock::FullPlayerValuesUpdate { guid, data }],
        }
    }

    /// Create a full VALUES update for `UF::ActivePlayerData`.
    pub fn full_active_player_values_update(
        guid: ObjectGuid,
        map_id: u16,
        data: ActivePlayerDataValuesUpdate,
    ) -> Self {
        Self {
            map_id,
            num_updates: 1,
            destroy_guids: Vec::new(),
            out_of_range_guids: Vec::new(),
            blocks: vec![UpdateBlock::FullActivePlayerValuesUpdate { guid, data }],
        }
    }

    /// Create a VALUES update for `UF::ContainerData`, with optional `ItemData`.
    pub fn container_values_update(
        guid: ObjectGuid,
        map_id: u16,
        data: ContainerDataValuesUpdate,
    ) -> Self {
        Self {
            map_id,
            num_updates: 1,
            destroy_guids: Vec::new(),
            out_of_range_guids: Vec::new(),
            blocks: vec![UpdateBlock::ContainerValuesUpdate { guid, data }],
        }
    }

    /// Create an UpdateObject with item CREATE blocks.
    ///
    /// Each item gets its own block. Sent BEFORE the player CREATE packet
    /// so the client has item objects when it processes InvSlots.
    pub fn create_items(items: Vec<ItemCreateData>, map_id: u16) -> Self {
        let num = items.len() as u32;
        let blocks = items
            .into_iter()
            .map(|data| {
                let guid = data.item_guid;
                UpdateBlock::CreateItem {
                    guid,
                    create_data: data,
                }
            })
            .collect();

        Self {
            map_id,
            num_updates: num,
            destroy_guids: Vec::new(),
            out_of_range_guids: Vec::new(),
            blocks,
        }
    }

    /// Create an item VALUES update for changed stack count.
    pub fn item_stack_count_update(guid: ObjectGuid, map_id: u16, stack_count: u32) -> Self {
        Self {
            map_id,
            num_updates: 1,
            destroy_guids: Vec::new(),
            out_of_range_guids: Vec::new(),
            blocks: vec![UpdateBlock::ItemValuesUpdate { guid, stack_count }],
        }
    }
}

impl ServerPacket for UpdateObject {
    const OPCODE: ServerOpcodes = ServerOpcodes::UpdateObject;

    fn write(&self, pkt: &mut WorldPacket) {
        // Top level: NumObjUpdates + MapID
        pkt.write_uint32(self.num_updates);
        pkt.write_uint16(self.map_id);

        // Build the Data buffer (matches C# UpdateData.BuildPacket)
        let mut data_buf = WorldPacket::new_empty();
        let destroy_guids: BTreeSet<ObjectGuid> = self.destroy_guids.iter().copied().collect();
        let out_of_range_guids: BTreeSet<ObjectGuid> =
            self.out_of_range_guids.iter().copied().collect();

        // HasDestroyOrOutOfRange bit
        let has_destroy_or_oor = !destroy_guids.is_empty() || !out_of_range_guids.is_empty();
        data_buf.write_bit(has_destroy_or_oor);

        if has_destroy_or_oor {
            data_buf.write_uint16(destroy_guids.len() as u16);
            data_buf.write_uint32((destroy_guids.len() + out_of_range_guids.len()) as u32);
            for g in &destroy_guids {
                data_buf.write_packed_guid(g);
            }
            for g in &out_of_range_guids {
                data_buf.write_packed_guid(g);
            }
        }

        // Build all update blocks into a separate buffer
        let mut blocks_buf = WorldPacket::new_empty();
        for block in &self.blocks {
            match block {
                UpdateBlock::CreateObject {
                    update_type,
                    guid,
                    type_id,
                    movement,
                    create_data,
                    is_self,
                } => {
                    write_create_block(
                        &mut blocks_buf,
                        *update_type,
                        guid,
                        *type_id,
                        movement.as_ref(),
                        create_data,
                        *is_self,
                    );
                }
                UpdateBlock::CreateCreature {
                    guid,
                    movement,
                    create_data,
                } => {
                    write_creature_create_block(&mut blocks_buf, guid, movement, create_data);
                }
                UpdateBlock::CreateGameObject { guid, create_data } => {
                    write_gameobject_create_block(&mut blocks_buf, guid, create_data);
                }
                UpdateBlock::CreateDynamicObject { guid, create_data } => {
                    write_dynamic_object_create_block(&mut blocks_buf, guid, create_data);
                }
                UpdateBlock::CreateItem { guid, create_data } => {
                    write_item_create_block(&mut blocks_buf, guid, create_data);
                }
                UpdateBlock::ItemValuesUpdate { guid, stack_count } => {
                    write_item_values_update_block(&mut blocks_buf, guid, *stack_count);
                }
                UpdateBlock::PlayerValuesUpdate {
                    guid,
                    inv_slot_changes,
                    buyback_changes,
                    visible_item_changes,
                    virtual_item_changes,
                    stat_changes,
                    coinage_change,
                } => {
                    write_player_values_update_block(
                        &mut blocks_buf,
                        guid,
                        inv_slot_changes,
                        buyback_changes,
                        visible_item_changes,
                        virtual_item_changes,
                        stat_changes.as_ref(),
                        *coinage_change,
                    );
                }
                UpdateBlock::CreatureHealthUpdate {
                    guid,
                    health,
                    max_health,
                } => {
                    write_creature_health_update_block(&mut blocks_buf, guid, *health, *max_health);
                }
                UpdateBlock::ObjectValuesUpdate { guid, data } => {
                    write_object_values_update_block(&mut blocks_buf, guid, *data);
                }
                UpdateBlock::DynamicObjectValuesUpdate { guid, data } => {
                    write_dynamic_object_values_update_block(&mut blocks_buf, guid, *data);
                }
                UpdateBlock::SceneObjectValuesUpdate { guid, data } => {
                    write_scene_object_values_update_block(&mut blocks_buf, guid, *data);
                }
                UpdateBlock::ConversationValuesUpdate { guid, data } => {
                    write_conversation_values_update_block(&mut blocks_buf, guid, data);
                }
                UpdateBlock::GameObjectValuesUpdate { guid, data } => {
                    write_game_object_values_update_block(&mut blocks_buf, guid, data);
                }
                UpdateBlock::CorpseValuesUpdate { guid, data } => {
                    write_corpse_values_update_block(&mut blocks_buf, guid, data);
                }
                UpdateBlock::AreaTriggerValuesUpdate { guid, data } => {
                    write_area_trigger_values_update_block(&mut blocks_buf, guid, data);
                }
                UpdateBlock::FullItemValuesUpdate { guid, data } => {
                    write_full_item_values_update_block(&mut blocks_buf, guid, data);
                }
                UpdateBlock::UnitValuesUpdate { guid, data } => {
                    write_full_unit_values_update_block(&mut blocks_buf, guid, data);
                }
                UpdateBlock::FullPlayerValuesUpdate { guid, data } => {
                    write_full_player_values_update_block(&mut blocks_buf, guid, data);
                }
                UpdateBlock::FullActivePlayerValuesUpdate { guid, data } => {
                    write_full_active_player_values_update_block(&mut blocks_buf, guid, data);
                }
                UpdateBlock::ContainerValuesUpdate { guid, data } => {
                    write_container_values_update_block(&mut blocks_buf, guid, data);
                }
                UpdateBlock::DestroyOutOfRange { .. } => {
                    // Handled via destroy_guids / out_of_range_guids, not as a block.
                }
            }
        }

        let blocks_data = blocks_buf.into_data();
        data_buf.write_uint32(blocks_data.len() as u32); // Data block size
        data_buf.write_bytes(&blocks_data);

        // Write the assembled Data buffer into the packet
        let assembled = data_buf.into_data();
        pkt.write_bytes(&assembled);
    }
}

/// Write a single CreateObject block.
fn write_create_block(
    buf: &mut WorldPacket,
    update_type: UpdateType,
    guid: &ObjectGuid,
    type_id: TypeId,
    movement: Option<&MovementBlock>,
    create_data: &PlayerCreateData,
    is_self: bool,
) {
    // UpdateType byte
    buf.write_uint8(update_type as u8);

    // Object GUID
    buf.write_packed_guid(guid);

    // TypeId byte
    buf.write_uint8(type_id as u8);

    // ── 18-bit CreateObjectBits ────────────────────────────────
    let has_movement = movement.is_some();
    buf.write_bit(false); // 0: NoBirthAnim
    buf.write_bit(false); // 1: EnablePortals
    buf.write_bit(false); // 2: PlayHoverAnim
    buf.write_bit(has_movement); // 3: MovementUpdate
    buf.write_bit(false); // 4: MovementTransport
    buf.write_bit(false); // 5: Stationary
    buf.write_bit(false); // 6: CombatVictim
    buf.write_bit(false); // 7: ServerTime
    buf.write_bit(false); // 8: Vehicle
    buf.write_bit(false); // 9: AnimKit
    buf.write_bit(false); // 10: Rotation
    buf.write_bit(false); // 11: AreaTrigger
    buf.write_bit(false); // 12: GameObject
    buf.write_bit(false); // 13: SmoothPhasing
    buf.write_bit(is_self); // 14: ThisIsYou
    buf.write_bit(false); // 15: SceneObject
    buf.write_bit(is_self); // 16: ActivePlayer
    buf.write_bit(false); // 17: Conversation
    buf.flush_bits();

    // ── MovementUpdate block ───────────────────────────────────
    if let Some(mv) = movement {
        write_movement_update(buf, guid, mv);
    }

    // PauseTimes count (i32) — always 0, written after movement regardless of flags
    buf.write_int32(0);

    // No Stationary, CombatVictim, ServerTime, Vehicle, AnimKit, Rotation,
    // AreaTrigger, GameObject, SmoothPhasing, SceneObject blocks
    // (all flags are false)

    // MovementTransport block — not present (bit 4 = false)

    // ── ActivePlayer block (bit 16) ─────────────────────────────
    // C# BuildMovementUpdate writes this when flags.ActivePlayer is true.
    // Contains: 3 bits (HasSceneInstanceIDs, HasRuneState, HasActionButtons)
    //           + optional scene IDs, rune data, and 180 action buttons.
    if is_self {
        write_active_player_movement_block(buf);
    }

    // No Conversation block (bit 17 = false)

    // ── Values block ───────────────────────────────────────────
    create_data.write_values_create(buf, is_self);
}

/// Write the movement update block (when bit 3 = true).
fn write_movement_update(buf: &mut WorldPacket, guid: &ObjectGuid, mv: &MovementBlock) {
    // MoverGUID
    buf.write_packed_guid(guid);

    // MovementFlags, MovementFlags2, ExtraMovementFlags2
    buf.write_uint32(0);
    buf.write_uint32(0);
    buf.write_uint32(0);

    // MoveTime
    buf.write_uint32(0);

    // Position
    buf.write_float(mv.position.x);
    buf.write_float(mv.position.y);
    buf.write_float(mv.position.z);
    buf.write_float(mv.position.orientation);

    // Pitch
    buf.write_float(0.0);

    // StepUpStartElevation (f32, NOT u32!)
    buf.write_float(0.0);

    // RemoveForcesIDs.Count
    buf.write_uint32(0);

    // MoveIndex
    buf.write_uint32(0);

    // 7 conditional bits
    buf.write_bit(false); // HasStandingOnGameObjectGUID
    buf.write_bit(false); // HasTransport
    buf.write_bit(false); // HasFall
    buf.write_bit(false); // HasSpline
    buf.write_bit(false); // HeightChangeFailed
    buf.write_bit(false); // RemoteTimeValid
    buf.write_bit(false); // HasInertia
    // Note: no FlushBits here — we continue writing after conditional blocks

    // No transport, standing, inertia, advFlying, fall blocks (all bits false)

    // 9 movement speeds
    buf.write_float(mv.walk_speed);
    buf.write_float(mv.run_speed);
    buf.write_float(mv.run_back_speed);
    buf.write_float(mv.swim_speed);
    buf.write_float(mv.swim_back_speed);
    buf.write_float(mv.fly_speed);
    buf.write_float(mv.fly_back_speed);
    buf.write_float(mv.turn_rate);
    buf.write_float(mv.pitch_rate);

    // MovementForces count + modMagnitude
    buf.write_int32(0);
    buf.write_float(1.0);

    // 17 AdvancedFlying parameters (hardcoded defaults from C#)
    buf.write_float(2.0); // airFriction
    buf.write_float(65.0); // maxVel
    buf.write_float(1.0); // liftCoefficient
    buf.write_float(3.0); // doubleJumpVelMod
    buf.write_float(10.0); // glideStartMinHeight
    buf.write_float(100.0); // addImpulseMaxSpeed
    buf.write_float(90.0); // minBankingRate
    buf.write_float(140.0); // maxBankingRate
    buf.write_float(180.0); // minPitchingRateDown
    buf.write_float(360.0); // maxPitchingRateDown
    buf.write_float(90.0); // minPitchingRateUp
    buf.write_float(270.0); // maxPitchingRateUp
    buf.write_float(30.0); // minTurnVelThreshold
    buf.write_float(80.0); // maxTurnVelThreshold
    buf.write_float(2.75); // surfaceFriction
    buf.write_float(7.0); // overMaxDeceleration
    buf.write_float(0.4); // launchSpeedCoefficient

    // HasSplineData bit
    buf.write_bit(false);
    buf.flush_bits();

    // No movement forces, no spline data
}

/// The ActivePlayer block in BuildMovementUpdate (C# lines 733-768).
///
/// Written when the `ActivePlayer` bit (bit 16) is set in CreateObjectBits.
/// Contains 3 conditional bits, then optionally: scene instance IDs, rune state,
/// and 180 action buttons (4 bytes each = 720 bytes).
///
/// For a fresh player: HasSceneInstanceIDs=false, HasRuneState=false,
/// HasActionButtons=true, all 180 buttons = 0.
const MAX_ACTION_BUTTONS: usize = 180;

fn write_active_player_movement_block(buf: &mut WorldPacket) {
    // 3 bits: HasSceneInstanceIDs, HasRuneState, HasActionButtons
    buf.write_bit(false); // HasSceneInstanceIDs
    buf.write_bit(false); // HasRuneState
    buf.write_bit(true); // HasActionButtons
    buf.flush_bits();

    // HasSceneInstanceIDs: if true, would write i32 count + i32[] IDs (skipped)
    // HasRuneState: if true, would write rune data (skipped)

    // HasActionButtons: 180 action buttons, each i32 (4 bytes)
    for _ in 0..MAX_ACTION_BUTTONS {
        buf.write_uint32(0); // No action buttons configured
    }
}

/// Write a single CreateObject block for a creature (TypeId::Unit).
fn write_creature_create_block(
    buf: &mut WorldPacket,
    guid: &ObjectGuid,
    movement: &MovementBlock,
    create_data: &CreatureCreateData,
) {
    // UpdateType: CreateObject2 — always used when object appears for the first time
    // to a player (matches C# Map.AddToMap → SetIsNewObject(true) → CreateObject2).
    buf.write_uint8(UpdateType::CreateObject2 as u8);

    // Object GUID
    buf.write_packed_guid(guid);

    // TypeId = Unit (5)
    buf.write_uint8(TypeId::Unit as u8);

    // ── 18-bit CreateObjectBits ────────────────────────────
    let has_anim_kit = create_data.ai_anim_kit_id != 0
        || create_data.movement_anim_kit_id != 0
        || create_data.melee_anim_kit_id != 0;
    buf.write_bit(false); // 0: NoBirthAnim
    buf.write_bit(false); // 1: EnablePortals
    buf.write_bit(false); // 2: PlayHoverAnim
    buf.write_bit(true); // 3: MovementUpdate (always true for Unit)
    buf.write_bit(false); // 4: MovementTransport
    buf.write_bit(false); // 5: Stationary
    buf.write_bit(false); // 6: CombatVictim
    buf.write_bit(false); // 7: ServerTime
    buf.write_bit(false); // 8: Vehicle
    buf.write_bit(has_anim_kit); // 9: AnimKit
    buf.write_bit(false); // 10: Rotation
    buf.write_bit(false); // 11: AreaTrigger
    buf.write_bit(false); // 12: GameObject
    buf.write_bit(false); // 13: SmoothPhasing
    buf.write_bit(false); // 14: ThisIsYou (false for creatures)
    buf.write_bit(false); // 15: SceneObject
    buf.write_bit(false); // 16: ActivePlayer (false for creatures)
    buf.write_bit(false); // 17: Conversation
    buf.flush_bits();

    // ── MovementUpdate block ───────────────────────────────
    write_movement_update(buf, guid, movement);

    // PauseTimes count
    buf.write_int32(0);

    if has_anim_kit {
        buf.write_uint16(create_data.ai_anim_kit_id);
        buf.write_uint16(create_data.movement_anim_kit_id);
        buf.write_uint16(create_data.melee_anim_kit_id);
    }

    // No ActivePlayer block (bit 16 = false)

    // ── Values block ───────────────────────────────────────
    create_data.write_values_create(buf);
}

/// Write a single CreateObject block for a gameobject (TypeId::GameObject).
///
/// GameObjects use: Stationary (bit 5) + Rotation (bit 10) + GameObject (bit 12) flags.
/// No MovementUpdate block.
fn write_gameobject_create_block(
    buf: &mut WorldPacket,
    guid: &ObjectGuid,
    create_data: &GameObjectCreateData,
) {
    // UpdateType: CreateObject2 — first appearance of this object to the client
    buf.write_uint8(UpdateType::CreateObject2 as u8);

    // Object GUID
    buf.write_packed_guid(guid);

    // TypeId = GameObject (8)
    buf.write_uint8(TypeId::GameObject as u8);

    // ── 18-bit CreateObjectBits ────────────────────────────
    buf.write_bit(false); // 0: NoBirthAnim
    buf.write_bit(false); // 1: EnablePortals
    buf.write_bit(false); // 2: PlayHoverAnim
    buf.write_bit(false); // 3: MovementUpdate (false for GOs)
    buf.write_bit(false); // 4: MovementTransport
    buf.write_bit(true); // 5: Stationary (true for GOs)
    buf.write_bit(false); // 6: CombatVictim
    buf.write_bit(false); // 7: ServerTime
    buf.write_bit(false); // 8: Vehicle
    buf.write_bit(false); // 9: AnimKit
    buf.write_bit(true); // 10: Rotation (true for GOs)
    buf.write_bit(false); // 11: AreaTrigger
    buf.write_bit(true); // 12: GameObject (true for GOs)
    buf.write_bit(false); // 13: SmoothPhasing
    buf.write_bit(false); // 14: ThisIsYou
    buf.write_bit(false); // 15: SceneObject
    buf.write_bit(false); // 16: ActivePlayer
    buf.write_bit(false); // 17: Conversation
    buf.flush_bits();

    // No MovementUpdate (bit 3 = false)

    // PauseTimes count (i32) — always 0
    buf.write_int32(0);

    // ── Stationary block (bit 5 = true) ─────────────────────
    buf.write_float(create_data.position.x);
    buf.write_float(create_data.position.y);
    buf.write_float(create_data.position.z);
    buf.write_float(create_data.position.orientation);

    // ── Rotation block (bit 10 = true) ──────────────────────
    buf.write_int64(create_data.packed_rotation());

    // ── GameObject block (bit 12 = true) ─────────────────────
    buf.write_int32(0); // WorldEffectID
    buf.write_bit(false); // has extra u32
    buf.flush_bits();

    // ── Values block ─────────────────────────────────────────
    create_data.write_values_create(buf);
}

/// Write a single CreateObject block for a dynamic object (TypeId::DynamicObject).
///
/// DynamicObjects use Stationary (bit 5), no MovementUpdate, no Unit shared-vision payload.
fn write_dynamic_object_create_block(
    buf: &mut WorldPacket,
    guid: &ObjectGuid,
    create_data: &DynamicObjectCreateData,
) {
    // UpdateType: CreateObject2 — first appearance of this object to the client
    buf.write_uint8(UpdateType::CreateObject2 as u8);

    // Object GUID
    buf.write_packed_guid(guid);

    // TypeId = DynamicObject (9)
    buf.write_uint8(TypeId::DynamicObject as u8);

    // ── 18-bit CreateObjectBits ────────────────────────────
    buf.write_bit(false); // 0: NoBirthAnim
    buf.write_bit(false); // 1: EnablePortals
    buf.write_bit(false); // 2: PlayHoverAnim
    buf.write_bit(false); // 3: MovementUpdate (false for DynamicObjects)
    buf.write_bit(false); // 4: MovementTransport
    buf.write_bit(true); // 5: Stationary (true for DynamicObjects)
    buf.write_bit(false); // 6: CombatVictim
    buf.write_bit(false); // 7: ServerTime
    buf.write_bit(false); // 8: Vehicle
    buf.write_bit(false); // 9: AnimKit
    buf.write_bit(false); // 10: Rotation
    buf.write_bit(false); // 11: AreaTrigger
    buf.write_bit(false); // 12: GameObject
    buf.write_bit(false); // 13: SmoothPhasing
    buf.write_bit(false); // 14: ThisIsYou
    buf.write_bit(false); // 15: SceneObject
    buf.write_bit(false); // 16: ActivePlayer
    buf.write_bit(false); // 17: Conversation
    buf.flush_bits();

    // No MovementUpdate (bit 3 = false)

    // PauseTimes count (i32) — always 0
    buf.write_int32(0);

    // ── Stationary block (bit 5 = true) ─────────────────────
    buf.write_float(create_data.position.x);
    buf.write_float(create_data.position.y);
    buf.write_float(create_data.position.z);
    buf.write_float(create_data.position.orientation);

    // ── Values block ─────────────────────────────────────────
    create_data.write_values_create(buf);
}

/// Write a single CreateObject block for an Item (TypeId::Item).
///
/// Items have NO movement block, NO stationary, and all 18 bits are false.
/// Values = ObjectData + ItemData (with Owner conditional fields).
fn write_item_create_block(buf: &mut WorldPacket, guid: &ObjectGuid, data: &ItemCreateData) {
    // UpdateType: CreateObject2 — first appearance of item to the client
    buf.write_uint8(UpdateType::CreateObject2 as u8);

    // Object GUID
    buf.write_packed_guid(guid);

    // TypeId = Item (1)
    buf.write_uint8(TypeId::Item as u8);

    // ── 18-bit CreateObjectBits (all false for items) ────
    for _ in 0..18 {
        buf.write_bit(false);
    }
    buf.flush_bits();

    // PauseTimes count (i32) — always 0
    buf.write_int32(0);

    // ── Values block ─────────────────────────────────────
    let mut val_buf = WorldPacket::new_empty();
    let flags: u8 = 0x01; // Owner
    val_buf.write_uint8(flags);

    // -- ObjectData (3 fields) --
    val_buf.write_int32(data.entry_id); // EntryId
    val_buf.write_uint32(0); // DynamicFlags
    val_buf.write_float(1.0); // Scale

    // -- ItemData --
    // Owner, ContainedIn, Creator, GiftCreator
    val_buf.write_packed_guid(&data.owner_guid);
    val_buf.write_packed_guid(&data.contained_in);
    write_empty_guid(&mut val_buf); // Creator
    write_empty_guid(&mut val_buf); // GiftCreator

    // Owner conditional block 1
    val_buf.write_int32(data.stack_count as i32); // StackCount
    val_buf.write_int32(0); // Expiration
    for _ in 0..5 {
        val_buf.write_int32(0); // SpellCharges[5]
    }

    // DynamicFlags
    val_buf.write_uint32(data.dynamic_flags);

    // 13 x ItemEnchantment (all zeros)
    for _ in 0..13 {
        val_buf.write_int32(0); // ID
        val_buf.write_int32(0); // Duration
        val_buf.write_int16(0); // Charges
        val_buf.write_uint8(0); // Field_A
        val_buf.write_uint8(0); // Field_B
    }

    // PropertySeed, RandomPropertiesID
    val_buf.write_int32(data.random_properties_seed);
    val_buf.write_int32(data.random_properties_id);

    // Owner conditional block 2
    val_buf.write_int32(data.durability as i32); // Durability
    val_buf.write_int32(data.max_durability as i32); // MaxDurability

    // CreatePlayedTime, Context, CreateTime
    val_buf.write_int32(0);
    val_buf.write_int32(i32::from(data.context));
    val_buf.write_int64(0);

    // Owner conditional block 3
    val_buf.write_int64(0); // ArtifactXP
    val_buf.write_uint8(0); // ItemAppearanceModID

    // ArtifactPowers.Size, Gems.Size
    val_buf.write_int32(0);
    val_buf.write_int32(0);

    // Owner conditional block 4
    val_buf.write_uint32(0); // DynamicFlags2

    // ItemBonusKey: ItemID + BonusCount
    val_buf.write_int32(0); // ItemID
    val_buf.write_int32(0); // BonusListIDs.Count

    // Owner conditional block 5
    val_buf.write_uint16(0); // DEBUGItemLevel

    // ItemModList (dynamic) — 6 bits for size = 0, then FlushBits
    val_buf.write_bits(0, 6);
    val_buf.flush_bits();

    // Write values block with size prefix
    let val_data = val_buf.into_data();
    buf.write_uint32(val_data.len() as u32);
    buf.write_bytes(&val_data);
}

// ── VALUES update (UpdateType::Values) ─────────────────────────────

/// Write an ItemData VALUES update containing StackCount only.
///
/// C++ refs:
/// - `Item::SetCount`
/// - `Object::BuildValuesUpdate`
/// - `UF::ItemData::WriteUpdate`
fn write_item_values_update_block(buf: &mut WorldPacket, guid: &ObjectGuid, stack_count: u32) {
    buf.write_uint8(UpdateType::Values as u8);
    buf.write_packed_guid(guid);

    let mut val_buf = WorldPacket::new_empty();
    val_buf.write_uint32(1 << 1); // TypeId::Item

    // ItemData has 43 bits: two 32-bit field blocks and a 2-bit blocks mask.
    // Parent bit 0 and StackCount bit 7 are set for a count-only update.
    val_buf.write_bits(0x01, 2);
    val_buf.write_bits((1 << 0) | (1 << 7), 32);
    val_buf.flush_bits();
    val_buf.write_int32(stack_count as i32);

    let val_data = val_buf.into_data();
    buf.write_uint32(val_data.len() as u32);
    buf.write_bytes(&val_data);
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
fn write_player_values_update_block(
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
    // C# Player.BuildValuesUpdate writes:
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

    // Write only sections that have changes (C# checks HasChanged per TypeId)
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

/// Write a VALUES update block containing only the base `UF::ObjectData` delta.
///
/// C++ refs:
/// - `Object::PrepareValuesUpdateBuffer`
/// - `Unit/GameObject/...::BuildValuesUpdate`
/// - `UF::ObjectData::WriteUpdate`
fn write_object_values_update_block(
    buf: &mut WorldPacket,
    guid: &ObjectGuid,
    data: ObjectDataValuesUpdate,
) {
    buf.write_uint8(UpdateType::Values as u8);
    buf.write_packed_guid(guid);

    let mut val_buf = WorldPacket::new_empty();
    val_buf.write_uint32(data.changed_object_type_mask);

    if data.changed_object_type_mask & 1 != 0 {
        let mask = data.object_data_mask & 0x0F;
        val_buf.write_bits(mask, 4);
        val_buf.flush_bits();

        if mask & 0x01 != 0 {
            if mask & 0x02 != 0 {
                val_buf.write_int32(data.entry_id);
            }
            if mask & 0x04 != 0 {
                val_buf.write_uint32(data.dynamic_flags);
            }
            if mask & 0x08 != 0 {
                val_buf.write_float(data.scale);
            }
        }
    }

    let val_data = val_buf.into_data();
    buf.write_uint32(val_data.len() as u32);
    buf.write_bytes(&val_data);
}

const VALUES_TYPE_OBJECT: u32 = 1 << 0;
const VALUES_TYPE_ITEM: u32 = 1 << 1;
const VALUES_TYPE_CONTAINER: u32 = 1 << 2;
const VALUES_TYPE_UNIT: u32 = 1 << 5;
const VALUES_TYPE_PLAYER: u32 = 1 << 6;
const VALUES_TYPE_ACTIVE_PLAYER: u32 = 1 << 7;
const VALUES_TYPE_GAME_OBJECT: u32 = 1 << 8;
const VALUES_TYPE_DYNAMIC_OBJECT: u32 = 1 << 9;
const VALUES_TYPE_CORPSE: u32 = 1 << 10;
const VALUES_TYPE_AREA_TRIGGER: u32 = 1 << 11;
const VALUES_TYPE_SCENE_OBJECT: u32 = 1 << 12;
const VALUES_TYPE_CONVERSATION: u32 = 1 << 13;

fn write_object_data_values_update_section(buf: &mut WorldPacket, data: ObjectDataValuesUpdate) {
    let mask = data.object_data_mask & 0x0F;
    buf.write_bits(mask, 4);
    buf.flush_bits();

    if mask & 0x01 != 0 {
        if mask & 0x02 != 0 {
            buf.write_int32(data.entry_id);
        }
        if mask & 0x04 != 0 {
            buf.write_uint32(data.dynamic_flags);
        }
        if mask & 0x08 != 0 {
            buf.write_float(data.scale);
        }
    }
}

fn write_dynamic_object_values_update_block(
    buf: &mut WorldPacket,
    guid: &ObjectGuid,
    data: DynamicObjectDataValuesUpdate,
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

    if data.changed_object_type_mask & VALUES_TYPE_DYNAMIC_OBJECT != 0 {
        let mask = data.dynamic_object_data_mask & 0x7F;
        val_buf.write_bits(mask, 7);
        val_buf.flush_bits();

        if mask & 0x01 != 0 {
            if mask & 0x02 != 0 {
                val_buf.write_packed_guid(&data.caster);
            }
            if mask & 0x04 != 0 {
                val_buf.write_uint8(data.dynamic_object_type);
            }
            if mask & 0x08 != 0 {
                val_buf.write_int32(data.spell_visual_id);
            }
            if mask & 0x10 != 0 {
                val_buf.write_int32(data.spell_id);
            }
            if mask & 0x20 != 0 {
                val_buf.write_float(data.radius);
            }
            if mask & 0x40 != 0 {
                val_buf.write_uint32(data.cast_time_ms);
            }
        }
    }

    let val_data = val_buf.into_data();
    buf.write_uint32(val_data.len() as u32);
    buf.write_bytes(&val_data);
}

fn write_scene_object_values_update_block(
    buf: &mut WorldPacket,
    guid: &ObjectGuid,
    data: SceneObjectDataValuesUpdate,
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

    if data.changed_object_type_mask & VALUES_TYPE_SCENE_OBJECT != 0 {
        let mask = data.scene_object_data_mask & 0x1F;
        val_buf.write_bits(mask, 5);
        val_buf.flush_bits();

        if mask & 0x01 != 0 {
            if mask & 0x02 != 0 {
                val_buf.write_int32(data.script_package_id);
            }
            if mask & 0x04 != 0 {
                val_buf.write_uint32(data.rnd_seed_val);
            }
            if mask & 0x08 != 0 {
                val_buf.write_packed_guid(&data.created_by);
            }
            if mask & 0x10 != 0 {
                val_buf.write_uint32(data.scene_type);
            }
        }
    }

    let val_data = val_buf.into_data();
    buf.write_uint32(val_data.len() as u32);
    buf.write_bytes(&val_data);
}

fn write_conversation_line_values_update(
    buf: &mut WorldPacket,
    line: &ConversationLineValuesUpdate,
) {
    buf.write_int32(line.conversation_line_id);
    buf.write_uint32(line.start_time);
    buf.write_int32(line.ui_camera_id);
    buf.write_uint8(line.actor_index);
    buf.write_uint8(line.flags);
}

fn write_conversation_actor_values_update(
    buf: &mut WorldPacket,
    actor: &ConversationActorValuesUpdate,
) {
    buf.write_bits(actor.actor_type & 1, 1);
    buf.write_int32(actor.id);

    if actor.actor_type == 1 {
        buf.write_uint32(actor.creature_id);
        buf.write_uint32(actor.creature_display_info_id);
    }

    if actor.actor_type == 0 {
        buf.write_packed_guid(&actor.actor_guid);
    }

    buf.flush_bits();
}

fn dynamic_mask_block(mask_blocks: &[u32], block_index: usize) -> u32 {
    mask_blocks.get(block_index).copied().unwrap_or(0)
}

fn write_dynamic_field_update_mask(
    buf: &mut WorldPacket,
    size: usize,
    update_mask: Option<&[u32]>,
) {
    write_dynamic_field_update_mask_bits(buf, size, update_mask, 32);
}

fn write_dynamic_field_update_mask_bits(
    buf: &mut WorldPacket,
    size: usize,
    update_mask: Option<&[u32]>,
    bits_for_size: u32,
) {
    buf.write_bits(size as u32, bits_for_size);

    if size > 32 {
        for block in 0..(size / 32) {
            let mask = update_mask
                .map(|blocks| dynamic_mask_block(blocks, block))
                .unwrap_or(0xFFFF_FFFF);
            buf.write_uint32(mask);
        }
    } else if size == 32 {
        let mask = update_mask
            .map(|blocks| dynamic_mask_block(blocks, 0))
            .unwrap_or(0xFFFF_FFFF);
        buf.write_bits(mask, 32);
        return;
    }

    if size % 32 != 0 {
        let block = size / 32;
        let bits = (size % 32) as u32;
        let mask = update_mask
            .map(|blocks| dynamic_mask_block(blocks, block))
            .unwrap_or(0xFFFF_FFFF);
        buf.write_bits(mask, bits);
    }
}

fn dynamic_mask_has_index(update_mask: Option<&[u32]>, index: usize) -> bool {
    match update_mask {
        None => true,
        Some(blocks) => {
            let block = index / 32;
            let bit = index % 32;
            dynamic_mask_block(blocks, block) & (1 << bit) != 0
        }
    }
}

fn write_conversation_values_update_block(
    buf: &mut WorldPacket,
    guid: &ObjectGuid,
    data: &ConversationDataValuesUpdate,
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

    if data.changed_object_type_mask & VALUES_TYPE_CONVERSATION != 0 {
        let mask = data.conversation_data_mask & 0x0F;
        val_buf.write_bits(mask, 4);

        if mask & 0x01 != 0 {
            if mask & 0x02 != 0 {
                val_buf.write_bits(data.lines.len() as u32, 32);
                for line in &data.lines {
                    write_conversation_line_values_update(&mut val_buf, line);
                }
            }
        }
        val_buf.flush_bits();

        if mask & 0x01 != 0 {
            if mask & 0x04 != 0 {
                write_dynamic_field_update_mask(
                    &mut val_buf,
                    data.actors.len(),
                    data.actor_update_mask.as_deref(),
                );
            }
        }
        val_buf.flush_bits();

        if mask & 0x01 != 0 {
            if mask & 0x04 != 0 {
                for (index, actor) in data.actors.iter().enumerate() {
                    if dynamic_mask_has_index(data.actor_update_mask.as_deref(), index) {
                        write_conversation_actor_values_update(&mut val_buf, actor);
                    }
                }
            }
            if mask & 0x08 != 0 {
                val_buf.write_int32(data.last_line_end_time);
            }
        }
    }

    let val_data = val_buf.into_data();
    buf.write_uint32(val_data.len() as u32);
    buf.write_bytes(&val_data);
}

fn write_changed_i32_dynamic_values(
    buf: &mut WorldPacket,
    values: &[i32],
    update_mask: Option<&[u32]>,
) {
    for (index, value) in values.iter().enumerate() {
        if dynamic_mask_has_index(update_mask, index) {
            buf.write_int32(*value);
        }
    }
}

fn write_game_object_values_update_block(
    buf: &mut WorldPacket,
    guid: &ObjectGuid,
    data: &GameObjectDataValuesUpdate,
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

    if data.changed_object_type_mask & VALUES_TYPE_GAME_OBJECT != 0 {
        let mask = data.game_object_data_mask & 0x000F_FFFF;
        val_buf.write_bits(mask, 20);

        if mask & 0x0000_0001 != 0 && mask & 0x0000_0002 != 0 {
            val_buf.write_bits(data.state_world_effect_ids.len() as u32, 32);
            for effect_id in &data.state_world_effect_ids {
                val_buf.write_uint32(*effect_id);
            }
        }
        val_buf.flush_bits();

        if mask & 0x0000_0001 != 0 {
            if mask & 0x0000_0004 != 0 {
                write_dynamic_field_update_mask(
                    &mut val_buf,
                    data.enable_doodad_sets.len(),
                    data.enable_doodad_sets_update_mask.as_deref(),
                );
            }
            if mask & 0x0000_0008 != 0 {
                write_dynamic_field_update_mask(
                    &mut val_buf,
                    data.world_effects.len(),
                    data.world_effects_update_mask.as_deref(),
                );
            }
        }
        val_buf.flush_bits();

        if mask & 0x0000_0001 != 0 {
            if mask & 0x0000_0004 != 0 {
                write_changed_i32_dynamic_values(
                    &mut val_buf,
                    &data.enable_doodad_sets,
                    data.enable_doodad_sets_update_mask.as_deref(),
                );
            }
            if mask & 0x0000_0008 != 0 {
                write_changed_i32_dynamic_values(
                    &mut val_buf,
                    &data.world_effects,
                    data.world_effects_update_mask.as_deref(),
                );
            }
            if mask & 0x0000_0010 != 0 {
                val_buf.write_int32(data.display_id);
            }
            if mask & 0x0000_0020 != 0 {
                val_buf.write_uint32(data.spell_visual_id);
            }
            if mask & 0x0000_0040 != 0 {
                val_buf.write_uint32(data.state_spell_visual_id);
            }
            if mask & 0x0000_0080 != 0 {
                val_buf.write_uint32(data.spawn_tracking_state_anim_id);
            }
            if mask & 0x0000_0100 != 0 {
                val_buf.write_uint32(data.spawn_tracking_state_anim_kit_id);
            }
            if mask & 0x0000_0200 != 0 {
                val_buf.write_packed_guid(&data.created_by);
            }
            if mask & 0x0000_0400 != 0 {
                val_buf.write_packed_guid(&data.guild_guid);
            }
            if mask & 0x0000_0800 != 0 {
                val_buf.write_uint32(data.flags);
            }
            if mask & 0x0000_1000 != 0 {
                for component in data.parent_rotation {
                    val_buf.write_float(component);
                }
            }
            if mask & 0x0000_2000 != 0 {
                val_buf.write_int32(data.faction_template);
            }
            if mask & 0x0000_4000 != 0 {
                val_buf.write_int32(data.level);
            }
            if mask & 0x0000_8000 != 0 {
                val_buf.write_int8(data.state);
            }
            if mask & 0x0001_0000 != 0 {
                val_buf.write_int8(data.type_id);
            }
            if mask & 0x0002_0000 != 0 {
                val_buf.write_uint8(data.percent_health);
            }
            if mask & 0x0004_0000 != 0 {
                val_buf.write_uint32(data.art_kit);
            }
            if mask & 0x0008_0000 != 0 {
                val_buf.write_uint32(data.custom_param);
            }
        }
    }

    let val_data = val_buf.into_data();
    buf.write_uint32(val_data.len() as u32);
    buf.write_bytes(&val_data);
}

fn write_chr_customization_choice_values_update(
    buf: &mut WorldPacket,
    choice: &ChrCustomizationChoiceValuesUpdate,
) {
    buf.write_uint32(choice.option_id);
    buf.write_uint32(choice.choice_id);
}

fn write_corpse_values_update_block(
    buf: &mut WorldPacket,
    guid: &ObjectGuid,
    data: &CorpseDataValuesUpdate,
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

    if data.changed_object_type_mask & VALUES_TYPE_CORPSE != 0 {
        let mask = data.corpse_data_mask;
        val_buf.write_bits(mask, 32);

        if mask & 0x0000_0001 != 0 && mask & 0x0000_0002 != 0 {
            write_dynamic_field_update_mask(
                &mut val_buf,
                data.customizations.len(),
                data.customizations_update_mask.as_deref(),
            );
        }
        val_buf.flush_bits();

        if mask & 0x0000_0001 != 0 {
            if mask & 0x0000_0002 != 0 {
                for (index, customization) in data.customizations.iter().enumerate() {
                    if dynamic_mask_has_index(data.customizations_update_mask.as_deref(), index) {
                        write_chr_customization_choice_values_update(&mut val_buf, customization);
                    }
                }
            }
            if mask & 0x0000_0004 != 0 {
                val_buf.write_uint32(data.dynamic_flags);
            }
            if mask & 0x0000_0008 != 0 {
                val_buf.write_packed_guid(&data.owner);
            }
            if mask & 0x0000_0010 != 0 {
                val_buf.write_packed_guid(&data.party_guid);
            }
            if mask & 0x0000_0020 != 0 {
                val_buf.write_packed_guid(&data.guild_guid);
            }
            if mask & 0x0000_0040 != 0 {
                val_buf.write_uint32(data.display_id);
            }
            if mask & 0x0000_0080 != 0 {
                val_buf.write_uint8(data.race_id);
            }
            if mask & 0x0000_0100 != 0 {
                val_buf.write_uint8(data.sex);
            }
            if mask & 0x0000_0200 != 0 {
                val_buf.write_uint8(data.class);
            }
            if mask & 0x0000_0400 != 0 {
                val_buf.write_uint32(data.flags);
            }
            if mask & 0x0000_0800 != 0 {
                val_buf.write_int32(data.faction_template);
            }
        }

        if mask & 0x0000_1000 != 0 {
            for (index, item) in data.items.iter().enumerate() {
                if mask & (1 << (13 + index)) != 0 {
                    val_buf.write_uint32(*item);
                }
            }
        }
    }

    let val_data = val_buf.into_data();
    buf.write_uint32(val_data.len() as u32);
    buf.write_bytes(&val_data);
}

fn write_scale_curve_values_update(buf: &mut WorldPacket, data: &ScaleCurveValuesUpdate) {
    let mask = data.scale_curve_mask & 0x7F;
    buf.write_bits(mask, 7);

    if mask & 0x01 != 0 && mask & 0x02 != 0 {
        buf.write_bit(data.override_active);
    }
    buf.flush_bits();

    if mask & 0x01 != 0 {
        if mask & 0x04 != 0 {
            buf.write_uint32(data.start_time_offset);
        }
        if mask & 0x08 != 0 {
            buf.write_uint32(data.parameter_curve);
        }
    }

    if mask & 0x10 != 0 {
        for (index, point) in data.points.iter().enumerate() {
            if mask & (1 << (5 + index)) != 0 {
                buf.write_float(point.0);
                buf.write_float(point.1);
            }
        }
    }
    buf.flush_bits();
}

fn write_visual_anim_values_update(buf: &mut WorldPacket, data: &VisualAnimValuesUpdate) {
    let mask = data.visual_anim_mask & 0x1F;
    buf.write_bits(mask, 5);

    if mask & 0x01 != 0 && mask & 0x02 != 0 {
        buf.write_bit(data.field_c);
    }
    buf.flush_bits();

    if mask & 0x01 != 0 {
        if mask & 0x04 != 0 {
            buf.write_uint32(data.animation_data_id);
        }
        if mask & 0x08 != 0 {
            buf.write_uint32(data.anim_kit_id);
        }
        if mask & 0x10 != 0 {
            buf.write_uint32(data.anim_progress);
        }
    }
    buf.flush_bits();
}

fn write_area_trigger_values_update_block(
    buf: &mut WorldPacket,
    guid: &ObjectGuid,
    data: &AreaTriggerDataValuesUpdate,
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

    if data.changed_object_type_mask & VALUES_TYPE_AREA_TRIGGER != 0 {
        let mask = data.area_trigger_data_mask & 0x000F_FFFF;
        val_buf.write_bits(mask, 20);
        val_buf.flush_bits();

        if mask & 0x0000_0001 != 0 {
            if mask & 0x0000_0002 != 0 {
                write_scale_curve_values_update(&mut val_buf, &data.override_scale_curve);
            }
            if mask & 0x0000_0040 != 0 {
                val_buf.write_packed_guid(&data.caster);
            }
            if mask & 0x0000_0080 != 0 {
                val_buf.write_uint32(data.duration);
            }
            if mask & 0x0000_0100 != 0 {
                val_buf.write_uint32(data.time_to_target);
            }
            if mask & 0x0000_0200 != 0 {
                val_buf.write_uint32(data.time_to_target_scale);
            }
            if mask & 0x0000_0400 != 0 {
                val_buf.write_uint32(data.time_to_target_extra_scale);
            }
            if mask & 0x0000_0800 != 0 {
                val_buf.write_uint32(data.time_to_target_pos);
            }
            if mask & 0x0000_1000 != 0 {
                val_buf.write_int32(data.spell_id);
            }
            if mask & 0x0000_2000 != 0 {
                val_buf.write_int32(data.spell_for_visuals);
            }
            if mask & 0x0000_4000 != 0 {
                val_buf.write_int32(data.spell_visual_id);
            }
            if mask & 0x0000_8000 != 0 {
                val_buf.write_float(data.bounds_radius_2d);
            }
            if mask & 0x0001_0000 != 0 {
                val_buf.write_uint32(data.decal_properties_id);
            }
            if mask & 0x0002_0000 != 0 {
                val_buf.write_packed_guid(&data.creating_effect_guid);
            }
            if mask & 0x0004_0000 != 0 {
                val_buf.write_packed_guid(&data.orbit_path_target);
            }
            if mask & 0x0000_0004 != 0 {
                write_scale_curve_values_update(&mut val_buf, &data.extra_scale_curve);
            }
            if mask & 0x0000_0008 != 0 {
                write_scale_curve_values_update(&mut val_buf, &data.override_move_curve_x);
            }
            if mask & 0x0000_0010 != 0 {
                write_scale_curve_values_update(&mut val_buf, &data.override_move_curve_y);
            }
            if mask & 0x0000_0020 != 0 {
                write_scale_curve_values_update(&mut val_buf, &data.override_move_curve_z);
            }
            if mask & 0x0008_0000 != 0 {
                write_visual_anim_values_update(&mut val_buf, &data.visual_anim);
            }
        }
    }

    let val_data = val_buf.into_data();
    buf.write_uint32(val_data.len() as u32);
    buf.write_bytes(&val_data);
}

fn write_update_field_blocks_mask(buf: &mut WorldPacket, mask: u64, block_count: u32) {
    let mut blocks_mask = 0u32;
    for block in 0..block_count {
        if ((mask >> (block * 32)) & 0xFFFF_FFFF) != 0 {
            blocks_mask |= 1 << block;
        }
    }

    buf.write_bits(blocks_mask, block_count);
    for block in 0..block_count {
        let block_bits = ((mask >> (block * 32)) & 0xFFFF_FFFF) as u32;
        if block_bits != 0 {
            buf.write_bits(block_bits, 32);
        }
    }
}

fn write_update_field_blocks_mask_u32(
    buf: &mut WorldPacket,
    blocks: &[u32],
    block_count_bits: u32,
) {
    let mut blocks_mask = 0u32;
    for (block, value) in blocks.iter().enumerate() {
        if *value != 0 {
            blocks_mask |= 1 << block;
        }
    }

    buf.write_bits(blocks_mask, block_count_bits);
    for value in blocks {
        if *value != 0 {
            buf.write_bits(*value, 32);
        }
    }
}

fn field_mask_has(mask: u64, bit: usize) -> bool {
    mask & (1u64 << bit) != 0
}

fn field_blocks_have(blocks: &[u32], bit: usize) -> bool {
    let block = bit / 32;
    let bit_in_block = bit % 32;
    blocks.get(block).copied().unwrap_or(0) & (1 << bit_in_block) != 0
}

fn write_artifact_power_values_update(buf: &mut WorldPacket, data: &ArtifactPowerValuesUpdate) {
    buf.write_int16(data.artifact_power_id);
    buf.write_uint8(data.purchased_rank);
    buf.write_uint8(data.current_rank_with_bonus);
}

fn write_socketed_gem_values_update(buf: &mut WorldPacket, data: &SocketedGemValuesUpdate) {
    let mask = u64::from(data.socketed_gem_mask & 0x000F_FFFF);
    write_update_field_blocks_mask(buf, mask, 1);
    buf.flush_bits();

    if field_mask_has(mask, 0) {
        if field_mask_has(mask, 1) {
            buf.write_int32(data.item_id);
        }
        if field_mask_has(mask, 2) {
            buf.write_uint8(data.context);
        }
    }

    if field_mask_has(mask, 3) {
        for (index, bonus) in data.bonus_list_ids.iter().enumerate() {
            if field_mask_has(mask, 4 + index) {
                buf.write_uint16(*bonus);
            }
        }
    }
}

fn write_item_enchantment_values_update(buf: &mut WorldPacket, data: &ItemEnchantmentValuesUpdate) {
    let mask = data.item_enchantment_mask & 0x3F;
    buf.write_bits(mask, 6);
    buf.flush_bits();

    if mask & 0x01 != 0 {
        if mask & 0x02 != 0 {
            buf.write_int32(data.id);
        }
        if mask & 0x04 != 0 {
            buf.write_uint32(data.duration);
        }
        if mask & 0x08 != 0 {
            buf.write_int16(data.charges);
        }
        if mask & 0x10 != 0 {
            buf.write_uint8(data.field_a);
        }
        if mask & 0x20 != 0 {
            buf.write_uint8(data.field_b);
        }
    }
}

fn write_item_mod_values_update(buf: &mut WorldPacket, data: &ItemModValuesUpdate) {
    buf.write_int32(data.value);
    buf.write_uint8(data.item_mod_type);
}

fn write_item_mod_list_values_update(buf: &mut WorldPacket, data: &ItemModListValuesUpdate) {
    let mask = data.item_mod_list_mask & 0x01;
    buf.write_bits(mask, 1);

    if mask & 0x01 != 0 {
        write_dynamic_field_update_mask_bits(
            buf,
            data.values.len(),
            data.values_update_mask.as_deref(),
            6,
        );
    }
    buf.flush_bits();

    if mask & 0x01 != 0 {
        for (index, value) in data.values.iter().enumerate() {
            if dynamic_mask_has_index(data.values_update_mask.as_deref(), index) {
                write_item_mod_values_update(buf, value);
            }
        }
    }
    buf.flush_bits();
}

fn write_item_bonus_key_values_update(buf: &mut WorldPacket, data: &ItemBonusKeyValuesUpdate) {
    buf.write_int32(data.item_id);
    buf.write_uint32(data.bonus_list_ids.len() as u32);
    for bonus in &data.bonus_list_ids {
        buf.write_int32(*bonus);
    }
}

fn write_item_data_values_update_section(buf: &mut WorldPacket, data: &ItemDataValuesDeltaUpdate) {
    let mask = data.item_data_mask & ((1u64 << 43) - 1);
    write_update_field_blocks_mask(buf, mask, 2);

    if field_mask_has(mask, 0) {
        if field_mask_has(mask, 1) {
            write_dynamic_field_update_mask(
                buf,
                data.artifact_powers.len(),
                data.artifact_powers_update_mask.as_deref(),
            );
        }
        if field_mask_has(mask, 2) {
            write_dynamic_field_update_mask(buf, data.gems.len(), data.gems_update_mask.as_deref());
        }
    }
    buf.flush_bits();

    if field_mask_has(mask, 0) {
        if field_mask_has(mask, 1) {
            for (index, artifact_power) in data.artifact_powers.iter().enumerate() {
                if dynamic_mask_has_index(data.artifact_powers_update_mask.as_deref(), index) {
                    write_artifact_power_values_update(buf, artifact_power);
                }
            }
        }
        if field_mask_has(mask, 2) {
            for (index, gem) in data.gems.iter().enumerate() {
                if dynamic_mask_has_index(data.gems_update_mask.as_deref(), index) {
                    write_socketed_gem_values_update(buf, gem);
                }
            }
        }
        if field_mask_has(mask, 3) {
            buf.write_packed_guid(&data.owner);
        }
        if field_mask_has(mask, 4) {
            buf.write_packed_guid(&data.contained_in);
        }
        if field_mask_has(mask, 5) {
            buf.write_packed_guid(&data.creator);
        }
        if field_mask_has(mask, 6) {
            buf.write_packed_guid(&data.gift_creator);
        }
        if field_mask_has(mask, 7) {
            buf.write_uint32(data.stack_count);
        }
        if field_mask_has(mask, 8) {
            buf.write_uint32(data.expiration);
        }
        if field_mask_has(mask, 9) {
            buf.write_uint32(data.dynamic_flags);
        }
        if field_mask_has(mask, 10) {
            buf.write_int32(data.property_seed);
        }
        if field_mask_has(mask, 11) {
            buf.write_int32(data.random_properties_id);
        }
        if field_mask_has(mask, 12) {
            buf.write_uint32(data.durability);
        }
        if field_mask_has(mask, 13) {
            buf.write_uint32(data.max_durability);
        }
        if field_mask_has(mask, 14) {
            buf.write_uint32(data.create_played_time);
        }
        if field_mask_has(mask, 15) {
            buf.write_int32(data.context);
        }
        if field_mask_has(mask, 16) {
            buf.write_int64(data.create_time);
        }
        if field_mask_has(mask, 17) {
            buf.write_uint64(data.artifact_xp);
        }
        if field_mask_has(mask, 18) {
            buf.write_uint8(data.item_appearance_mod_id);
        }
        if field_mask_has(mask, 20) {
            buf.write_uint32(data.dynamic_flags2);
        }
        if field_mask_has(mask, 21) {
            write_item_bonus_key_values_update(buf, &data.item_bonus_key);
        }
        if field_mask_has(mask, 22) {
            buf.write_uint16(data.debug_item_level);
        }
        if field_mask_has(mask, 19) {
            write_item_mod_list_values_update(buf, &data.modifiers);
        }
    }

    if field_mask_has(mask, 23) {
        for (index, charge) in data.spell_charges.iter().enumerate() {
            if field_mask_has(mask, 24 + index) {
                buf.write_int32(*charge);
            }
        }
    }

    if field_mask_has(mask, 29) {
        for (index, enchantment) in data.enchantments.iter().enumerate() {
            if field_mask_has(mask, 30 + index) {
                write_item_enchantment_values_update(buf, enchantment);
            }
        }
    }
}

fn write_full_item_values_update_block(
    buf: &mut WorldPacket,
    guid: &ObjectGuid,
    data: &ItemDataValuesDeltaUpdate,
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

    if data.changed_object_type_mask & VALUES_TYPE_ITEM != 0 {
        write_item_data_values_update_section(&mut val_buf, data);
    }

    let val_data = val_buf.into_data();
    buf.write_uint32(val_data.len() as u32);
    buf.write_bytes(&val_data);
}

fn write_container_data_values_update_section(
    buf: &mut WorldPacket,
    data: &ContainerDataValuesUpdate,
) {
    let mask = data.container_data_mask & ((1u64 << 39) - 1);
    write_update_field_blocks_mask(buf, mask, 2);
    buf.flush_bits();

    if field_mask_has(mask, 0) && field_mask_has(mask, 1) {
        buf.write_uint32(data.num_slots);
    }

    if field_mask_has(mask, 2) {
        for (index, slot) in data.slots.iter().enumerate() {
            if field_mask_has(mask, 3 + index) {
                buf.write_packed_guid(slot);
            }
        }
    }
}

fn write_container_values_update_block(
    buf: &mut WorldPacket,
    guid: &ObjectGuid,
    data: &ContainerDataValuesUpdate,
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

    if data.changed_object_type_mask & VALUES_TYPE_ITEM != 0 {
        if let Some(item_data) = &data.item_data {
            write_item_data_values_update_section(&mut val_buf, item_data);
        }
    }

    if data.changed_object_type_mask & VALUES_TYPE_CONTAINER != 0 {
        write_container_data_values_update_section(&mut val_buf, data);
    }

    let val_data = val_buf.into_data();
    buf.write_uint32(val_data.len() as u32);
    buf.write_bytes(&val_data);
}

fn unit_mask_has(data: &UnitDataValuesDeltaUpdate, bit: usize) -> bool {
    let block = bit / 32;
    let offset = bit % 32;
    data.unit_data_mask.get(block).copied().unwrap_or(0) & (1 << offset) != 0
}

fn write_passive_spell_history_values_update(
    buf: &mut WorldPacket,
    data: &PassiveSpellHistoryValuesUpdate,
) {
    buf.write_int32(data.spell_id);
    buf.write_int32(data.aura_spell_id);
}

fn write_unit_channel_values_update(buf: &mut WorldPacket, data: &UnitChannelValuesUpdate) {
    buf.write_int32(data.spell_id);
    buf.write_int32(data.spell_visual_id);
}

fn write_visible_item_values_update(buf: &mut WorldPacket, data: &VisibleItemValuesUpdate) {
    let mask = data.visible_item_mask & 0x0F;
    buf.write_bits(mask, 4);
    buf.flush_bits();

    if mask & 0x01 != 0 {
        if mask & 0x02 != 0 {
            buf.write_int32(data.item_id);
        }
        if mask & 0x04 != 0 {
            buf.write_uint16(data.appearance_mod_id);
        }
        if mask & 0x08 != 0 {
            buf.write_uint16(data.item_visual);
        }
    }
}

fn write_unit_data_values_update_section(buf: &mut WorldPacket, data: &UnitDataValuesDeltaUpdate) {
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

fn write_full_unit_values_update_block(
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

fn write_arena_cooldown_values_update(buf: &mut WorldPacket, data: &ArenaCooldownValuesUpdate) {
    let mask = data.arena_cooldown_mask & 0x01FF;
    buf.write_bits(mask, 9);
    buf.flush_bits();

    if mask & 0x001 != 0 {
        if mask & 0x002 != 0 {
            buf.write_int32(data.spell_id);
        }
        if mask & 0x004 != 0 {
            buf.write_int32(data.item_id);
        }
        if mask & 0x008 != 0 {
            buf.write_int32(data.charges);
        }
        if mask & 0x010 != 0 {
            buf.write_uint32(data.flags);
        }
        if mask & 0x020 != 0 {
            buf.write_uint32(data.start_time);
        }
        if mask & 0x040 != 0 {
            buf.write_uint32(data.end_time);
        }
        if mask & 0x080 != 0 {
            buf.write_uint32(data.next_charge_time);
        }
        if mask & 0x100 != 0 {
            buf.write_uint8(data.max_charges);
        }
    }
}

fn write_dungeon_score_summary_values_update(
    buf: &mut WorldPacket,
    data: &DungeonScoreSummaryValuesUpdate,
) {
    buf.write_float(data.overall_score_current_season);
    buf.write_float(data.ladder_score_current_season);
    buf.write_uint32(data.runs.len() as u32);
    for run in &data.runs {
        buf.write_int32(run.challenge_mode_id);
        buf.write_float(run.map_score);
        buf.write_int32(run.best_run_level);
        buf.write_int32(run.best_run_duration_ms);
        buf.write_bit(run.finished_success);
        buf.flush_bits();
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

fn write_full_player_values_update_block(
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

fn write_full_active_player_values_update_block(
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

/// UnitData VALUES update: VirtualItems[3] and/or stat fields.
///
/// C# UnitData.WriteUpdate format:
///   WriteBits(blocksMask, 8) — which of 8 blocks have changes
///   for each active block: WriteBits(block, 32)
///   [dynamic arrays if block 0 active]
///   FlushBits()
///   [field values in C# field definition order]
///
/// Field write order (C# UnitData.WriteUpdate):
///   Block 0: Health(5), MaxHealth(6)
///   Block 1: MinDamage(52→20), MaxDamage(53→21)
///   Block 2: BaseMana(75→11), BaseHealth(76→12), AttackPower(81→17),
///            RangedAttackPower(85→21), MinRangedDamage(91→27), MaxRangedDamage(92→28)
///   Block 3: Power parent(116→20)
///   Block 4: Power[0](137→9), MaxPower[0](147→19)
///   Block 5: VirtualItems(167-170→7-10), Stats(174-179→14-19),
///            StatPosBuff(180-184→20-24), Resistances(190-191→30-31)
fn write_unit_data_values_update(
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
        blocks[2] |=
            (1 << 0) | (1 << 11) | (1 << 12) | (1 << 17) | (1 << 21) | (1 << 27) | (1 << 28);
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

    // Dynamic arrays: block 0 bit 0 set → C# enters dynamic array check,
    // but bits 1-4 are NOT set, so nothing to write.
    buf.flush_bits();

    // ── Field values in C# definition order ──
    // Blocks 0-4: only stat fields
    if let Some(sc) = stat_changes {
        // Block 0: Health, MaxHealth
        buf.write_int64(sc.health);
        buf.write_int64(sc.max_health);

        // Block 1: MinDamage, MaxDamage
        buf.write_float(sc.min_damage);
        buf.write_float(sc.max_damage);

        // Block 2: BaseMana, BaseHealth, AttackPower, RangedAttackPower,
        //          MinRangedDamage, MaxRangedDamage
        buf.write_int32(sc.base_mana);
        buf.write_int32(sc.base_health);
        buf.write_int32(sc.attack_power);
        buf.write_int32(sc.ranged_attack_power);
        buf.write_float(sc.min_ranged_damage);
        buf.write_float(sc.max_ranged_damage);

        // Blocks 3-4: Power interleaved loop (index 0)
        // C# writes PowerRegenFlat, PowerRegenInterrupted, Power, MaxPower, ModPowerRegen
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

    // Stats/StatPosBuff/StatNegBuff INTERLEAVED per index (C# lines 1728-1744),
    // then Resistances — after VirtualItems in block 5
    if let Some(sc) = stat_changes {
        for i in 0..5 {
            buf.write_int32(sc.stats[i]); // Stats[i]
            buf.write_int32(sc.stat_pos_buff[i]); // StatPosBuff[i]
            // StatNegBuff[i] bits not set → skip
        }
        buf.write_int32(sc.armor); // Resistances[0]
    }
}

/// PlayerData VALUES update: VisibleItems[19] (equipment display).
///
/// C# PlayerData.WriteUpdate format:
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

    // C# PlayerData.WriteUpdate ALWAYS writes this bit after block masks:
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

pub fn write_research_values_update(buf: &mut WorldPacket, data: ResearchValuesUpdate) {
    buf.write_int16(data.research_project_id);
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

pub fn write_spell_pct_mod_by_label_values_update(
    buf: &mut WorldPacket,
    data: SpellPctModByLabelValuesUpdate,
) {
    buf.write_int32(data.mod_index);
    buf.write_float(data.modifier_value);
    buf.write_int32(data.label_id);
}

pub fn write_spell_flat_mod_by_label_values_update(
    buf: &mut WorldPacket,
    data: SpellFlatModByLabelValuesUpdate,
) {
    buf.write_int32(data.mod_index);
    buf.write_int32(data.modifier_value);
    buf.write_int32(data.label_id);
}

pub fn write_category_cooldown_mod_values_update(
    buf: &mut WorldPacket,
    data: CategoryCooldownModValuesUpdate,
) {
    buf.write_int32(data.spell_category_id);
    buf.write_int32(data.mod_cooldown);
}

pub fn write_weekly_spell_use_values_update(
    buf: &mut WorldPacket,
    data: WeeklySpellUseValuesUpdate,
) {
    buf.write_int32(data.spell_category_id);
    buf.write_uint8(data.uses);
}

pub fn write_completed_project_values_update(
    buf: &mut WorldPacket,
    data: CompletedProjectValuesUpdate,
) {
    let mask = data.completed_project_mask & 0x0F;
    buf.write_bits(mask as u32, 4);

    buf.flush_bits();
    if mask & 0x01 != 0 {
        if mask & 0x02 != 0 {
            buf.write_uint32(data.project_id);
        }
        if mask & 0x04 != 0 {
            buf.write_int64(data.first_completed);
        }
        if mask & 0x08 != 0 {
            buf.write_uint32(data.completion_count);
        }
    }
}

pub fn write_research_history_values_update(
    buf: &mut WorldPacket,
    data: &ResearchHistoryValuesUpdate,
) {
    let mask = data.research_history_mask & 0x03;
    buf.write_bits(mask as u32, 2);

    if mask & 0x01 != 0 && mask & 0x02 != 0 {
        write_dynamic_field_update_mask(
            buf,
            data.completed_projects.len(),
            data.completed_projects_update_mask.as_deref(),
        );
    }
    buf.flush_bits();

    if mask & 0x01 != 0 && mask & 0x02 != 0 {
        for (index, project) in data.completed_projects.iter().enumerate() {
            if dynamic_mask_has_index(data.completed_projects_update_mask.as_deref(), index) {
                write_completed_project_values_update(buf, *project);
            }
        }
    }
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

pub fn write_stable_info_values_update(buf: &mut WorldPacket, data: &StableInfoValuesUpdate) {
    let mask = data.stable_info_mask & 0x07;
    buf.write_bits(mask as u32, 3);

    if mask & 0x01 != 0 && mask & 0x02 != 0 {
        write_dynamic_field_update_mask(buf, data.pets.len(), data.pets_update_mask.as_deref());
    }
    buf.flush_bits();

    if mask & 0x01 != 0 {
        if mask & 0x02 != 0 {
            for (index, pet) in data.pets.iter().enumerate() {
                if dynamic_mask_has_index(data.pets_update_mask.as_deref(), index) {
                    write_stable_pet_info_values_update(buf, pet);
                }
            }
        }
        if mask & 0x04 != 0 {
            buf.write_packed_guid(&data.stable_master);
        }
    }
}

pub fn write_perks_vendor_item_values_update(
    buf: &mut WorldPacket,
    data: PerksVendorItemValuesUpdate,
) {
    buf.write_int32(data.vendor_item_id);
    buf.write_int32(data.mount_id);
    buf.write_int32(data.battle_pet_species_id);
    buf.write_int32(data.transmog_set_id);
    buf.write_int32(data.item_modified_appearance_id);
    buf.write_int32(data.field_14);
    buf.write_int32(data.field_18);
    buf.write_int32(data.price);
    buf.write_int64(data.available_until);
    buf.write_bit(data.disabled);
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
fn write_active_player_data_values_update(
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

        // Parent 38 section: ALL 31 fields (bits 39-69)
        // parent=38→b1:6, bits 39-63→b1:7-31, bits 64-69→b2:0-5
        blocks[1] |= 0xFFFF_FFC0; // bits 6-31
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

    // C#: WriteUInt32 for group 0 (byte-aligned), WriteBits for group 1 (16 bits)
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

    // ── Field values in C# WriteUpdate order ──

    // Block 0: Coinage (bit 28) — written before all other ActivePlayerData fields.
    // C# ref: ActivePlayerData.Coinage = new(0, 28) → written in block-0 field pass.
    if let Some(coinage) = coinage_change {
        buf.write_int64(coinage as i64);
    }

    // Parent 0 section: expertise (bits 36-37) — BEFORE parent 38
    if let Some(sc) = stat_changes {
        buf.write_float(sc.mainhand_expertise); // bit 36: MainhandExpertise
        buf.write_float(sc.offhand_expertise); // bit 37: OffhandExpertise
    }

    // Parent 38 section: ALL 31 fields (bits 39-69) in C# definition order
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
        buf.write_float(sc.shield_block_crit_pct); // bit 50: ShieldBlockCritPercentage
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
    // C# interleaves SpellCritPct/ModDmgDonePos/ModDmgDoneNeg/ModDmgDonePct per school.
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

/// Write a creature VALUES update block containing only health + max_health.
///
/// C# UnitData field positions:
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
fn write_creature_health_update_block(
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

impl UpdateObject {
    /// Build a single-creature health VALUES update packet.
    pub fn creature_health_update(
        guid: ObjectGuid,
        health: i64,
        max_health: i64,
        map_id: u16,
    ) -> Self {
        Self {
            map_id,
            num_updates: 1,
            destroy_guids: Vec::new(),
            out_of_range_guids: Vec::new(),
            blocks: vec![UpdateBlock::CreatureHealthUpdate {
                guid,
                health,
                max_health,
            }],
        }
    }

    /// Build an UpdateObject that hard-destroys objects (they no longer exist).
    pub fn destroy_objects(guids: Vec<ObjectGuid>, map_id: u16) -> Self {
        Self {
            map_id,
            num_updates: 0, // no create/update blocks
            destroy_guids: guids,
            out_of_range_guids: Vec::new(),
            blocks: Vec::new(),
        }
    }

    /// Build an UpdateObject that removes objects from the client's view
    /// because they moved out of range (they still exist in the world).
    /// C#: WorldObject.BuildOutOfRangeUpdateBlock → UpdateData.AddOutOfRangeGUID
    pub fn out_of_range_objects(guids: Vec<ObjectGuid>, map_id: u16) -> Self {
        Self {
            map_id,
            num_updates: 0, // no create/update blocks
            destroy_guids: Vec::new(),
            out_of_range_guids: guids,
            blocks: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gameobject_create_values_serializes_created_by_guid_like_cpp() {
        let base = GameObjectCreateData {
            guid: ObjectGuid::create_world_object(
                wow_core::guid::HighGuid::GameObject,
                0,
                1,
                0,
                0,
                123,
                456,
            ),
            entry: 123,
            dynamic_flags: 0,
            display_id: 456,
            go_type: 3,
            position: Position::ZERO,
            rotation: [0.0, 0.0, 0.0, 1.0],
            anim_progress: 255,
            state: 1,
            created_by: ObjectGuid::EMPTY,
            faction_template: 0,
            gameobject_flags: 0,
            scale: 1.0,
        };

        let mut empty_owner_packet = WorldPacket::new_empty();
        base.write_values_create(&mut empty_owner_packet);

        let mut owned = base;
        owned.created_by = ObjectGuid::create_player(1, 42);
        let mut owned_packet = WorldPacket::new_empty();
        owned.write_values_create(&mut owned_packet);

        assert!(owned_packet.data().len() > empty_owner_packet.data().len());
        assert_ne!(owned_packet.data(), empty_owner_packet.data());
    }

    #[test]
    fn gameobject_create_values_serializes_flags_and_faction_template_like_cpp() {
        let create = GameObjectCreateData {
            guid: ObjectGuid::create_world_object(
                wow_core::guid::HighGuid::GameObject,
                0,
                1,
                0,
                0,
                123,
                456,
            ),
            entry: 123,
            dynamic_flags: 0x44,
            display_id: 456,
            go_type: 3,
            position: Position::ZERO,
            rotation: [0.0, 0.0, 0.0, 1.0],
            anim_progress: 255,
            state: 1,
            created_by: ObjectGuid::EMPTY,
            faction_template: 1735,
            gameobject_flags: 0x20,
            scale: 1.0,
        };

        let mut packet = WorldPacket::new_empty();
        create.write_values_create(&mut packet);
        let data = packet.data();
        assert!(
            data.windows(4)
                .any(|window| window == 1735i32.to_le_bytes())
        );
        assert!(
            data.windows(4)
                .any(|window| window == 0x20u32.to_le_bytes())
        );
        assert!(
            data.windows(4)
                .any(|window| window == 0x44u32.to_le_bytes())
        );
    }

    #[test]
    fn dynamic_object_create_block_serializes_stationary_create_values_like_cpp() {
        let guid = ObjectGuid::create_world_object(
            wow_core::guid::HighGuid::DynamicObject,
            0,
            1,
            571,
            0,
            7001,
            9001,
        );
        let caster = ObjectGuid::create_player(1, 42);
        let position = Position::new(11.0, 22.0, 33.0, 1.5);
        let pkt = UpdateObject::create_world_objects(
            vec![UpdateObject::create_dynamic_object_block(
                DynamicObjectCreateData {
                    guid,
                    entry_id: 7001,
                    dynamic_flags: 0,
                    scale: 1.0,
                    position,
                    caster,
                    dynamic_object_type: 2,
                    spell_visual_id: 456,
                    spell_id: 777,
                    radius: 12.5,
                    cast_time_ms: 12345,
                },
            )],
            571,
        );

        let bytes = pkt.to_bytes();
        assert_eq!(
            u32::from_le_bytes([bytes[2], bytes[3], bytes[4], bytes[5]]),
            1
        );
        assert!(
            bytes
                .windows(1)
                .any(|window| window == [UpdateType::CreateObject2 as u8])
        );
        assert!(
            bytes
                .windows(1)
                .any(|window| window == [TypeId::DynamicObject as u8])
        );
        assert!(
            bytes
                .windows(4)
                .any(|window| window == position.x.to_le_bytes())
        );
        assert!(
            bytes
                .windows(4)
                .any(|window| window == position.y.to_le_bytes())
        );
        assert!(
            bytes
                .windows(4)
                .any(|window| window == position.z.to_le_bytes())
        );
        assert!(
            bytes
                .windows(4)
                .any(|window| window == position.orientation.to_le_bytes())
        );
        assert!(
            bytes
                .windows(4)
                .any(|window| window == 7001i32.to_le_bytes())
        );
        assert!(
            bytes
                .windows(4)
                .any(|window| window == 456i32.to_le_bytes())
        );
        assert!(
            bytes
                .windows(4)
                .any(|window| window == 777i32.to_le_bytes())
        );
        assert!(
            bytes
                .windows(4)
                .any(|window| window == 12.5f32.to_le_bytes())
        );
        assert!(
            bytes
                .windows(4)
                .any(|window| window == 12345u32.to_le_bytes())
        );
        assert!(!bytes.windows(1).all(|window| window == [0]));
    }

    #[test]
    fn update_object_create_player_serializes() {
        let guid = ObjectGuid::create_player(1, 42);
        let pos = Position::new(-8949.95, -132.493, 83.5312, 0.0);

        let pkt = UpdateObject::create_player(
            guid,
            1,
            1,
            0,
            1,
            49,
            &pos,
            0,
            12,
            true,
            [(0, 0, 0); 19],
            [ObjectGuid::EMPTY; 141],
            PlayerCombatStats::default(),
            Vec::new(),
            0,
            Vec::new(),
        );
        let bytes = pkt.to_bytes();
        // Should be a substantial packet (many KB with ActivePlayerData)
        assert!(
            bytes.len() > 1000,
            "Packet too small: {} bytes",
            bytes.len()
        );
    }

    #[test]
    fn update_object_out_of_range() {
        let pkt = UpdateObject {
            map_id: 0,
            num_updates: 0,
            destroy_guids: Vec::new(),
            out_of_range_guids: vec![
                ObjectGuid::create_player(1, 1),
                ObjectGuid::create_player(1, 2),
            ],
            blocks: Vec::new(),
        };
        let bytes = pkt.to_bytes();
        assert!(bytes.len() > 10);
    }

    #[test]
    fn item_create_serializes_random_properties_and_context() {
        let item_guid = ObjectGuid::create_item(1, 900);
        let owner_guid = ObjectGuid::create_player(1, 42);
        let pkt = UpdateObject::create_items(
            vec![ItemCreateData {
                item_guid,
                entry_id: 700,
                owner_guid,
                contained_in: owner_guid,
                stack_count: 7,
                dynamic_flags: 0,
                durability: 12,
                max_durability: 20,
                random_properties_seed: 456,
                random_properties_id: -77,
                context: 2,
            }],
            0,
        );

        let bytes = pkt.to_bytes();

        assert!(bytes.windows(4).any(|window| window == 7i32.to_le_bytes()));
        assert!(bytes.windows(4).any(|window| window == 12i32.to_le_bytes()));
        assert!(bytes.windows(4).any(|window| window == 20i32.to_le_bytes()));
        assert!(
            bytes
                .windows(4)
                .any(|window| window == 456i32.to_le_bytes())
        );
        assert!(
            bytes
                .windows(4)
                .any(|window| window == (-77i32).to_le_bytes())
        );
        assert!(bytes.windows(4).any(|window| window == 2i32.to_le_bytes()));
    }

    #[test]
    fn item_stack_count_update_serializes_item_values_delta() {
        let item_guid = ObjectGuid::create_item(1, 900);
        let pkt = UpdateObject::item_stack_count_update(item_guid, 0, 19);

        let bytes = pkt.to_bytes();

        assert!(bytes.len() > 20);
        assert!(bytes.windows(4).any(|window| window == 19i32.to_le_bytes()));
    }

    #[test]
    fn movement_block_default_speeds() {
        let mv = MovementBlock::default();
        assert!((mv.walk_speed - 2.5).abs() < 0.01);
        assert!((mv.run_speed - 7.0).abs() < 0.01);
        assert!((mv.swim_speed - 4.72222).abs() < 0.01);
    }

    #[test]
    fn player_create_data_faction() {
        assert_eq!(PlayerCreateData::faction_for_race(1), 1); // Human
        assert_eq!(PlayerCreateData::faction_for_race(2), 2); // Orc
        assert_eq!(PlayerCreateData::faction_for_race(10), 1610); // BloodElf
        assert_eq!(PlayerCreateData::faction_for_race(11), 1629); // Draenei
    }

    #[test]
    fn update_object_envelope_format() {
        // Verify the top-level format: opcode + NumObjUpdates + MapID + Data
        let guid = ObjectGuid::create_player(1, 1);
        let pos = Position::new(0.0, 0.0, 0.0, 0.0);
        let pkt = UpdateObject::create_player(
            guid,
            1,
            1,
            0,
            1,
            49,
            &pos,
            0,
            12,
            true,
            [(0, 0, 0); 19],
            [ObjectGuid::EMPTY; 141],
            PlayerCombatStats::default(),
            Vec::new(),
            0,
            Vec::new(),
        );
        let bytes = pkt.to_bytes();

        // opcode (2 bytes)
        let opcode = u16::from_le_bytes([bytes[0], bytes[1]]);
        assert_eq!(opcode, ServerOpcodes::UpdateObject as u16);

        // NumObjUpdates (u32 at offset 2)
        let num_updates = u32::from_le_bytes([bytes[2], bytes[3], bytes[4], bytes[5]]);
        assert_eq!(num_updates, 1);

        // MapID (u16 at offset 6)
        let map_id = u16::from_le_bytes([bytes[6], bytes[7]]);
        assert_eq!(map_id, 0);
    }

    #[test]
    fn update_object_destroy_and_oor() {
        let pkt = UpdateObject {
            map_id: 0,
            num_updates: 0,
            destroy_guids: vec![ObjectGuid::create_player(1, 10)],
            out_of_range_guids: vec![ObjectGuid::create_player(1, 20)],
            blocks: Vec::new(),
        };
        let bytes = pkt.to_bytes();
        // Should contain destroy + oor data
        assert!(bytes.len() > 20);
    }

    #[test]
    fn update_object_destroy_sets_dedupe_like_cpp() {
        let guid1 = ObjectGuid::create_player(1, 1);
        let guid2 = ObjectGuid::create_player(1, 2);
        let guid3 = ObjectGuid::create_player(1, 3);
        let pkt = UpdateObject {
            map_id: 0,
            num_updates: 0,
            destroy_guids: vec![guid2, guid1, guid1],
            out_of_range_guids: vec![guid3, guid3],
            blocks: Vec::new(),
        };
        let bytes = pkt.to_bytes();

        // opcode(2) + NumObjUpdates(4) + MapID(2) + HasDestroy bit byte(1)
        let destroy_count = u16::from_le_bytes([bytes[9], bytes[10]]);
        let total_count = u32::from_le_bytes([bytes[11], bytes[12], bytes[13], bytes[14]]);
        assert_eq!(destroy_count, 2);
        assert_eq!(total_count, 3);
    }

    #[test]
    fn object_values_update_block_matches_cpp_objectdata_delta_shape() {
        let mut block = WorldPacket::new_empty();
        write_object_values_update_block(
            &mut block,
            &ObjectGuid::EMPTY,
            ObjectDataValuesUpdate {
                changed_object_type_mask: 1,
                object_data_mask: 0b1011,
                entry_id: 42,
                dynamic_flags: 0x80,
                scale: 2.0,
            },
        );

        let bytes = block.into_data();
        assert_eq!(bytes[0], UpdateType::Values as u8);
        assert_eq!(&bytes[1..3], &[0, 0]);
        assert_eq!(u32::from_le_bytes(bytes[3..7].try_into().unwrap()), 13);
        assert_eq!(u32::from_le_bytes(bytes[7..11].try_into().unwrap()), 1);
        assert_eq!(bytes[11], 0b1011_0000);
        assert_eq!(i32::from_le_bytes(bytes[12..16].try_into().unwrap()), 42);
        assert_eq!(f32::from_le_bytes(bytes[16..20].try_into().unwrap()), 2.0);
        assert_eq!(bytes.len(), 20);
    }

    #[test]
    fn dynamic_object_values_update_block_matches_cpp_dynamicobjectdata_delta_shape() {
        let mut block = WorldPacket::new_empty();
        write_dynamic_object_values_update_block(
            &mut block,
            &ObjectGuid::EMPTY,
            DynamicObjectDataValuesUpdate {
                changed_object_type_mask: VALUES_TYPE_DYNAMIC_OBJECT,
                object_data: None,
                dynamic_object_data_mask: 0b111_1111,
                caster: ObjectGuid::EMPTY,
                dynamic_object_type: 1,
                spell_visual_id: 42,
                spell_id: 1337,
                radius: 8.5,
                cast_time_ms: 123_456,
            },
        );

        let bytes = block.into_data();
        assert_eq!(bytes[0], UpdateType::Values as u8);
        assert_eq!(&bytes[1..3], &[0, 0]);
        assert_eq!(u32::from_le_bytes(bytes[3..7].try_into().unwrap()), 24);
        assert_eq!(
            u32::from_le_bytes(bytes[7..11].try_into().unwrap()),
            VALUES_TYPE_DYNAMIC_OBJECT
        );
        assert_eq!(bytes[11], 0b1111_1110);
        assert_eq!(&bytes[12..14], &[0, 0]);
        assert_eq!(bytes[14], 1);
        assert_eq!(i32::from_le_bytes(bytes[15..19].try_into().unwrap()), 42);
        assert_eq!(i32::from_le_bytes(bytes[19..23].try_into().unwrap()), 1337);
        assert_eq!(f32::from_le_bytes(bytes[23..27].try_into().unwrap()), 8.5);
        assert_eq!(
            u32::from_le_bytes(bytes[27..31].try_into().unwrap()),
            123_456
        );
        assert_eq!(bytes.len(), 31);
    }

    #[test]
    fn scene_object_values_update_block_matches_cpp_sceneobjectdata_delta_shape() {
        let mut block = WorldPacket::new_empty();
        write_scene_object_values_update_block(
            &mut block,
            &ObjectGuid::EMPTY,
            SceneObjectDataValuesUpdate {
                changed_object_type_mask: VALUES_TYPE_SCENE_OBJECT,
                object_data: None,
                scene_object_data_mask: 0b1_1111,
                script_package_id: 77,
                rnd_seed_val: 0xAABB_CCDD,
                created_by: ObjectGuid::EMPTY,
                scene_type: 1,
            },
        );

        let bytes = block.into_data();
        assert_eq!(bytes[0], UpdateType::Values as u8);
        assert_eq!(&bytes[1..3], &[0, 0]);
        assert_eq!(u32::from_le_bytes(bytes[3..7].try_into().unwrap()), 19);
        assert_eq!(
            u32::from_le_bytes(bytes[7..11].try_into().unwrap()),
            VALUES_TYPE_SCENE_OBJECT
        );
        assert_eq!(bytes[11], 0b1111_1000);
        assert_eq!(i32::from_le_bytes(bytes[12..16].try_into().unwrap()), 77);
        assert_eq!(
            u32::from_le_bytes(bytes[16..20].try_into().unwrap()),
            0xAABB_CCDD
        );
        assert_eq!(&bytes[20..22], &[0, 0]);
        assert_eq!(u32::from_le_bytes(bytes[22..26].try_into().unwrap()), 1);
        assert_eq!(bytes.len(), 26);
    }

    #[test]
    fn conversation_values_update_block_matches_cpp_last_line_delta_shape() {
        let mut block = WorldPacket::new_empty();
        write_conversation_values_update_block(
            &mut block,
            &ObjectGuid::EMPTY,
            &ConversationDataValuesUpdate {
                changed_object_type_mask: VALUES_TYPE_CONVERSATION,
                object_data: None,
                conversation_data_mask: 0b1001,
                lines: Vec::new(),
                actors: Vec::new(),
                actor_update_mask: None,
                last_line_end_time: 12_345,
            },
        );

        let bytes = block.into_data();
        assert_eq!(bytes[0], UpdateType::Values as u8);
        assert_eq!(&bytes[1..3], &[0, 0]);
        assert_eq!(u32::from_le_bytes(bytes[3..7].try_into().unwrap()), 9);
        assert_eq!(
            u32::from_le_bytes(bytes[7..11].try_into().unwrap()),
            VALUES_TYPE_CONVERSATION
        );
        assert_eq!(bytes[11], 0b1001_0000);
        assert_eq!(
            i32::from_le_bytes(bytes[12..16].try_into().unwrap()),
            12_345
        );
        assert_eq!(bytes.len(), 16);
    }

    #[test]
    fn conversation_values_update_block_matches_cpp_lines_actors_delta_shape() {
        let mut block = WorldPacket::new_empty();
        write_conversation_values_update_block(
            &mut block,
            &ObjectGuid::EMPTY,
            &ConversationDataValuesUpdate {
                changed_object_type_mask: VALUES_TYPE_CONVERSATION,
                object_data: None,
                conversation_data_mask: 0b1111,
                lines: vec![ConversationLineValuesUpdate {
                    conversation_line_id: 7,
                    start_time: 100,
                    ui_camera_id: -3,
                    actor_index: 2,
                    flags: 0x80,
                }],
                actors: vec![ConversationActorValuesUpdate {
                    actor_type: 1,
                    id: 55,
                    creature_id: 12_345,
                    creature_display_info_id: 54_321,
                    actor_guid: ObjectGuid::EMPTY,
                }],
                actor_update_mask: None,
                last_line_end_time: 777,
            },
        );

        let bytes = block.into_data();
        assert_eq!(bytes[0], UpdateType::Values as u8);
        assert_eq!(&bytes[1..3], &[0, 0]);
        assert_eq!(u32::from_le_bytes(bytes[3..7].try_into().unwrap()), 45);
        assert_eq!(
            u32::from_le_bytes(bytes[7..11].try_into().unwrap()),
            VALUES_TYPE_CONVERSATION
        );
        assert_eq!(&bytes[11..16], &[0xF0, 0x00, 0x00, 0x00, 0x10]);
        assert_eq!(i32::from_le_bytes(bytes[16..20].try_into().unwrap()), 7);
        assert_eq!(u32::from_le_bytes(bytes[20..24].try_into().unwrap()), 100);
        assert_eq!(i32::from_le_bytes(bytes[24..28].try_into().unwrap()), -3);
        assert_eq!(&bytes[28..30], &[2, 0x80]);
        assert_eq!(&bytes[30..35], &[0x00, 0x00, 0x00, 0x01, 0x80]);
        assert_eq!(bytes[35], 0x80);
        assert_eq!(i32::from_le_bytes(bytes[36..40].try_into().unwrap()), 55);
        assert_eq!(
            u32::from_le_bytes(bytes[40..44].try_into().unwrap()),
            12_345
        );
        assert_eq!(
            u32::from_le_bytes(bytes[44..48].try_into().unwrap()),
            54_321
        );
        assert_eq!(i32::from_le_bytes(bytes[48..52].try_into().unwrap()), 777);
        assert_eq!(bytes.len(), 52);
    }

    #[test]
    fn game_object_values_update_block_matches_cpp_gameobjectdata_delta_shape() {
        let mut block = WorldPacket::new_empty();
        write_game_object_values_update_block(
            &mut block,
            &ObjectGuid::EMPTY,
            &GameObjectDataValuesUpdate {
                changed_object_type_mask: VALUES_TYPE_GAME_OBJECT,
                object_data: None,
                game_object_data_mask: 0x0003_8011,
                state_world_effect_ids: Vec::new(),
                enable_doodad_sets: Vec::new(),
                enable_doodad_sets_update_mask: None,
                world_effects: Vec::new(),
                world_effects_update_mask: None,
                display_id: 123,
                spell_visual_id: 0,
                state_spell_visual_id: 0,
                spawn_tracking_state_anim_id: 0,
                spawn_tracking_state_anim_kit_id: 0,
                created_by: ObjectGuid::EMPTY,
                guild_guid: ObjectGuid::EMPTY,
                flags: 0,
                parent_rotation: [0.0; 4],
                faction_template: 0,
                level: 0,
                state: -1,
                type_id: 5,
                percent_health: 90,
                art_kit: 0,
                custom_param: 0,
            },
        );

        let bytes = block.into_data();
        assert_eq!(bytes[0], UpdateType::Values as u8);
        assert_eq!(&bytes[1..3], &[0, 0]);
        assert_eq!(u32::from_le_bytes(bytes[3..7].try_into().unwrap()), 14);
        assert_eq!(
            u32::from_le_bytes(bytes[7..11].try_into().unwrap()),
            VALUES_TYPE_GAME_OBJECT
        );
        assert_eq!(&bytes[11..14], &[0x38, 0x01, 0x10]);
        assert_eq!(i32::from_le_bytes(bytes[14..18].try_into().unwrap()), 123);
        assert_eq!(&bytes[18..21], &[0xFF, 5, 90]);
        assert_eq!(bytes.len(), 21);
    }

    #[test]
    fn corpse_values_update_block_matches_cpp_corpse_data_delta_shape() {
        let mut items = [0u32; 19];
        items[0] = 0xAABB_CCDD;

        let mut block = WorldPacket::new_empty();
        write_corpse_values_update_block(
            &mut block,
            &ObjectGuid::EMPTY,
            &CorpseDataValuesUpdate {
                changed_object_type_mask: VALUES_TYPE_CORPSE,
                object_data: None,
                corpse_data_mask: 0x0000_3007,
                customizations: vec![ChrCustomizationChoiceValuesUpdate {
                    option_id: 11,
                    choice_id: 22,
                }],
                customizations_update_mask: None,
                dynamic_flags: 0x44,
                owner: ObjectGuid::EMPTY,
                party_guid: ObjectGuid::EMPTY,
                guild_guid: ObjectGuid::EMPTY,
                display_id: 0,
                race_id: 0,
                sex: 0,
                class: 0,
                flags: 0,
                faction_template: 0,
                items,
            },
        );

        let bytes = block.into_data();
        assert_eq!(bytes[0], UpdateType::Values as u8);
        assert_eq!(&bytes[1..3], &[0, 0]);
        assert_eq!(u32::from_le_bytes(bytes[3..7].try_into().unwrap()), 29);
        assert_eq!(
            u32::from_le_bytes(bytes[7..11].try_into().unwrap()),
            VALUES_TYPE_CORPSE
        );
        assert_eq!(&bytes[11..15], &[0x00, 0x00, 0x30, 0x07]);
        assert_eq!(&bytes[15..20], &[0x00, 0x00, 0x00, 0x01, 0x80]);
        assert_eq!(u32::from_le_bytes(bytes[20..24].try_into().unwrap()), 11);
        assert_eq!(u32::from_le_bytes(bytes[24..28].try_into().unwrap()), 22);
        assert_eq!(u32::from_le_bytes(bytes[28..32].try_into().unwrap()), 0x44);
        assert_eq!(
            u32::from_le_bytes(bytes[32..36].try_into().unwrap()),
            0xAABB_CCDD
        );
        assert_eq!(bytes.len(), 36);
    }

    #[test]
    fn area_trigger_values_update_block_matches_cpp_areatriggerdata_delta_shape() {
        let empty_curve = ScaleCurveValuesUpdate {
            scale_curve_mask: 0,
            override_active: false,
            start_time_offset: 0,
            parameter_curve: 0,
            points: [(0.0, 0.0); 2],
        };

        let mut block = WorldPacket::new_empty();
        write_area_trigger_values_update_block(
            &mut block,
            &ObjectGuid::EMPTY,
            &AreaTriggerDataValuesUpdate {
                changed_object_type_mask: VALUES_TYPE_AREA_TRIGGER,
                object_data: None,
                area_trigger_data_mask: 0x0008_1081,
                override_scale_curve: empty_curve,
                extra_scale_curve: empty_curve,
                override_move_curve_x: empty_curve,
                override_move_curve_y: empty_curve,
                override_move_curve_z: empty_curve,
                caster: ObjectGuid::EMPTY,
                duration: 12_000,
                time_to_target: 0,
                time_to_target_scale: 0,
                time_to_target_extra_scale: 0,
                time_to_target_pos: 0,
                spell_id: 99,
                spell_for_visuals: 0,
                spell_visual_id: 0,
                bounds_radius_2d: 0.0,
                decal_properties_id: 0,
                creating_effect_guid: ObjectGuid::EMPTY,
                orbit_path_target: ObjectGuid::EMPTY,
                visual_anim: VisualAnimValuesUpdate {
                    visual_anim_mask: 0b0_0111,
                    field_c: true,
                    animation_data_id: 77,
                    anim_kit_id: 0,
                    anim_progress: 0,
                },
            },
        );

        let bytes = block.into_data();
        assert_eq!(bytes[0], UpdateType::Values as u8);
        assert_eq!(&bytes[1..3], &[0, 0]);
        assert_eq!(u32::from_le_bytes(bytes[3..7].try_into().unwrap()), 20);
        assert_eq!(
            u32::from_le_bytes(bytes[7..11].try_into().unwrap()),
            VALUES_TYPE_AREA_TRIGGER
        );
        assert_eq!(&bytes[11..14], &[0x81, 0x08, 0x10]);
        assert_eq!(
            u32::from_le_bytes(bytes[14..18].try_into().unwrap()),
            12_000
        );
        assert_eq!(i32::from_le_bytes(bytes[18..22].try_into().unwrap()), 99);
        assert_eq!(bytes[22], 0x3C);
        assert_eq!(u32::from_le_bytes(bytes[23..27].try_into().unwrap()), 77);
        assert_eq!(bytes.len(), 27);
    }

    fn test_item_data(mask: u64) -> ItemDataValuesDeltaUpdate {
        ItemDataValuesDeltaUpdate {
            changed_object_type_mask: VALUES_TYPE_ITEM,
            object_data: None,
            item_data_mask: mask,
            artifact_powers: Vec::new(),
            artifact_powers_update_mask: None,
            gems: Vec::new(),
            gems_update_mask: None,
            owner: ObjectGuid::EMPTY,
            contained_in: ObjectGuid::EMPTY,
            creator: ObjectGuid::EMPTY,
            gift_creator: ObjectGuid::EMPTY,
            stack_count: 5,
            expiration: 0,
            dynamic_flags: 0,
            property_seed: 0,
            random_properties_id: 0,
            durability: 0,
            max_durability: 0,
            create_played_time: 0,
            context: 0,
            create_time: 0,
            artifact_xp: 0,
            item_appearance_mod_id: 0,
            modifiers: ItemModListValuesUpdate {
                item_mod_list_mask: 0,
                values: Vec::new(),
                values_update_mask: None,
            },
            dynamic_flags2: 0,
            item_bonus_key: ItemBonusKeyValuesUpdate::default(),
            debug_item_level: 0,
            spell_charges: [0; 5],
            enchantments: [ItemEnchantmentValuesUpdate::default(); 13],
        }
    }

    #[test]
    fn full_item_values_update_block_matches_cpp_itemdata_stack_delta_shape() {
        let mut block = WorldPacket::new_empty();
        write_full_item_values_update_block(
            &mut block,
            &ObjectGuid::EMPTY,
            &test_item_data((1 << 0) | (1 << 7)),
        );

        let bytes = block.into_data();
        assert_eq!(bytes[0], UpdateType::Values as u8);
        assert_eq!(&bytes[1..3], &[0, 0]);
        assert_eq!(u32::from_le_bytes(bytes[3..7].try_into().unwrap()), 13);
        assert_eq!(
            u32::from_le_bytes(bytes[7..11].try_into().unwrap()),
            VALUES_TYPE_ITEM
        );
        assert_eq!(&bytes[11..16], &[0x40, 0x00, 0x00, 0x20, 0x40]);
        assert_eq!(u32::from_le_bytes(bytes[16..20].try_into().unwrap()), 5);
        assert_eq!(bytes.len(), 20);
    }

    #[test]
    fn container_values_update_block_matches_cpp_containerdata_slot_delta_shape() {
        let mut slots = [ObjectGuid::EMPTY; 36];
        slots[0] = ObjectGuid::EMPTY;

        let mut block = WorldPacket::new_empty();
        write_container_values_update_block(
            &mut block,
            &ObjectGuid::EMPTY,
            &ContainerDataValuesUpdate {
                changed_object_type_mask: VALUES_TYPE_CONTAINER,
                object_data: None,
                item_data: None,
                container_data_mask: 0x0F,
                num_slots: 16,
                slots,
            },
        );

        let bytes = block.into_data();
        assert_eq!(bytes[0], UpdateType::Values as u8);
        assert_eq!(&bytes[1..3], &[0, 0]);
        assert_eq!(u32::from_le_bytes(bytes[3..7].try_into().unwrap()), 15);
        assert_eq!(
            u32::from_le_bytes(bytes[7..11].try_into().unwrap()),
            VALUES_TYPE_CONTAINER
        );
        assert_eq!(&bytes[11..16], &[0x40, 0x00, 0x00, 0x03, 0xC0]);
        assert_eq!(u32::from_le_bytes(bytes[16..20].try_into().unwrap()), 16);
        assert_eq!(&bytes[20..22], &[0, 0]);
        assert_eq!(bytes.len(), 22);
    }

    #[test]
    fn full_unit_values_update_block_matches_cpp_unitdata_health_delta_shape() {
        let mut data = UnitDataValuesDeltaUpdate {
            health: 77,
            max_health: 99,
            ..Default::default()
        };
        data.unit_data_mask[0] = (1 << 0) | (1 << 5) | (1 << 6);

        let mut block = WorldPacket::new_empty();
        write_full_unit_values_update_block(&mut block, &ObjectGuid::EMPTY, &data);

        let bytes = block.into_data();
        assert_eq!(bytes[0], UpdateType::Values as u8);
        assert_eq!(&bytes[1..3], &[0, 0]);
        assert_eq!(u32::from_le_bytes(bytes[3..7].try_into().unwrap()), 25);
        assert_eq!(
            u32::from_le_bytes(bytes[7..11].try_into().unwrap()),
            VALUES_TYPE_UNIT
        );
        assert_eq!(&bytes[11..16], &[0x01, 0x00, 0x00, 0x00, 0x61]);
        assert_eq!(i64::from_le_bytes(bytes[16..24].try_into().unwrap()), 77);
        assert_eq!(i64::from_le_bytes(bytes[24..32].try_into().unwrap()), 99);
        assert_eq!(bytes.len(), 32);
    }

    #[test]
    fn full_unit_values_update_block_matches_cpp_unitdata_virtual_item_delta_shape() {
        let mut data = UnitDataValuesDeltaUpdate::default();
        data.unit_data_mask[5] = (1 << 7) | (1 << 8);
        data.virtual_items[0] = VisibleItemValuesUpdate {
            visible_item_mask: 0x0F,
            item_id: 19019,
            appearance_mod_id: 2,
            item_visual: 3,
        };

        let mut block = WorldPacket::new_empty();
        write_full_unit_values_update_block(&mut block, &ObjectGuid::EMPTY, &data);

        let bytes = block.into_data();
        assert_eq!(bytes[0], UpdateType::Values as u8);
        assert_eq!(&bytes[1..3], &[0, 0]);
        assert_eq!(u32::from_le_bytes(bytes[3..7].try_into().unwrap()), 18);
        assert_eq!(
            u32::from_le_bytes(bytes[7..11].try_into().unwrap()),
            VALUES_TYPE_UNIT
        );
        assert_eq!(&bytes[11..16], &[0x20, 0x00, 0x00, 0x01, 0x80]);
        assert_eq!(bytes[16], 0xF0);
        assert_eq!(i32::from_le_bytes(bytes[17..21].try_into().unwrap()), 19019);
        assert_eq!(u16::from_le_bytes(bytes[21..23].try_into().unwrap()), 2);
        assert_eq!(u16::from_le_bytes(bytes[23..25].try_into().unwrap()), 3);
        assert_eq!(bytes.len(), 25);
    }

    #[test]
    fn full_unit_values_update_block_matches_cpp_unitdata_npc_flags_delta_shape() {
        let mut data = UnitDataValuesDeltaUpdate::default();
        data.unit_data_mask[3] = (1 << 17) | (1 << 18) | (1 << 19);
        data.npc_flags = [0x40, 0x1];

        let mut block = WorldPacket::new_empty();
        write_full_unit_values_update_block(&mut block, &ObjectGuid::EMPTY, &data);

        let bytes = block.into_data();
        assert_eq!(bytes[0], UpdateType::Values as u8);
        assert_eq!(&bytes[1..3], &[0, 0]);
        assert_eq!(u32::from_le_bytes(bytes[3..7].try_into().unwrap()), 17);
        assert_eq!(
            u32::from_le_bytes(bytes[7..11].try_into().unwrap()),
            VALUES_TYPE_UNIT
        );
        assert_eq!(u32::from_le_bytes(bytes[16..20].try_into().unwrap()), 0x40);
        assert_eq!(u32::from_le_bytes(bytes[20..24].try_into().unwrap()), 0x1);
        assert_eq!(bytes.len(), 24);
    }

    #[test]
    fn full_player_values_update_block_matches_cpp_playerdata_visible_item_delta_shape() {
        let mut data = PlayerDataValuesDeltaUpdate::default();
        data.player_data_mask[1] = (1 << 29) | (1 << 30);
        data.visible_items[0] = VisibleItemValuesUpdate {
            visible_item_mask: 0x0F,
            item_id: 19019,
            appearance_mod_id: 2,
            item_visual: 3,
        };

        let mut block = WorldPacket::new_empty();
        write_full_player_values_update_block(&mut block, &ObjectGuid::EMPTY, &data);

        let bytes = block.into_data();
        assert_eq!(bytes[0], UpdateType::Values as u8);
        assert_eq!(&bytes[1..3], &[0, 0]);
        assert_eq!(u32::from_le_bytes(bytes[3..7].try_into().unwrap()), 18);
        assert_eq!(
            u32::from_le_bytes(bytes[7..11].try_into().unwrap()),
            VALUES_TYPE_PLAYER
        );
        assert_eq!(&bytes[11..16], &[0x26, 0x00, 0x00, 0x00, 0x00]);
        assert_eq!(bytes[16], 0xF0);
        assert_eq!(i32::from_le_bytes(bytes[17..21].try_into().unwrap()), 19019);
        assert_eq!(u16::from_le_bytes(bytes[21..23].try_into().unwrap()), 2);
        assert_eq!(u16::from_le_bytes(bytes[23..25].try_into().unwrap()), 3);
        assert_eq!(bytes.len(), 25);
    }

    #[test]
    fn full_player_values_update_block_can_append_active_player_section_like_cpp() {
        let mut active_data = ActivePlayerDataValuesUpdate {
            coinage: 1234,
            ..Default::default()
        };
        set_active_player_bit(&mut active_data, 0);
        set_active_player_bit(&mut active_data, 28);

        let mut data = PlayerDataValuesDeltaUpdate {
            changed_object_type_mask: VALUES_TYPE_PLAYER | VALUES_TYPE_ACTIVE_PLAYER,
            active_player_data: Some(active_data),
            ..Default::default()
        };
        data.player_data_mask[1] = (1 << 29) | (1 << 30);
        data.visible_items[0] = VisibleItemValuesUpdate {
            visible_item_mask: 0x0F,
            item_id: 19019,
            appearance_mod_id: 2,
            item_visual: 3,
        };

        let mut block = WorldPacket::new_empty();
        write_full_player_values_update_block(&mut block, &ObjectGuid::EMPTY, &data);

        let bytes = block.into_data();
        assert_eq!(bytes[0], UpdateType::Values as u8);
        assert_eq!(&bytes[1..3], &[0, 0]);
        assert_eq!(
            u32::from_le_bytes(bytes[7..11].try_into().unwrap()),
            VALUES_TYPE_PLAYER | VALUES_TYPE_ACTIVE_PLAYER
        );
        assert_eq!(&bytes[11..16], &[0x26, 0x00, 0x00, 0x00, 0x00]);
        assert_eq!(bytes[16], 0xF0);
        assert_eq!(i32::from_le_bytes(bytes[17..21].try_into().unwrap()), 19019);
        assert_eq!(&bytes[25..29], &[0x01, 0x00, 0x00, 0x00]);
        assert_eq!(&bytes[29..31], &[0x00, 0x00]);
        assert_eq!(&bytes[31..35], &[0x10, 0x00, 0x00, 0x01]);
        assert_eq!(u64::from_le_bytes(bytes[35..43].try_into().unwrap()), 1234);
        assert_eq!(bytes.len(), 43);
    }

    #[test]
    fn creature_health_values_update_has_no_create_flags_byte_like_cpp() {
        let mut block = WorldPacket::new_empty();
        write_creature_health_update_block(&mut block, &ObjectGuid::EMPTY, 7, 11);

        let bytes = block.into_data();
        assert_eq!(bytes[0], UpdateType::Values as u8);
        assert_eq!(&bytes[1..3], &[0, 0]);
        assert_eq!(u32::from_le_bytes(bytes[3..7].try_into().unwrap()), 25);
        assert_eq!(u32::from_le_bytes(bytes[7..11].try_into().unwrap()), 1 << 5);
        assert_eq!(bytes[11], 0x01);
        assert_eq!(&bytes[12..16], &[0, 0, 0, 0x61]);
        assert_eq!(i64::from_le_bytes(bytes[16..24].try_into().unwrap()), 7);
        assert_eq!(i64::from_le_bytes(bytes[24..32].try_into().unwrap()), 11);
        assert_eq!(bytes.len(), 32);
    }

    #[test]
    fn buyback_values_update_interleaves_price_and_timestamp_like_cpp() {
        let mut values = WorldPacket::new_empty();
        write_active_player_data_values_update(
            &mut values,
            &[],
            &[(94, 123, 456), (95, 789, 101112)],
            None,
            None,
        );

        let bytes = values.into_data();
        let tail = &bytes[bytes.len() - 24..];
        assert_eq!(u32::from_le_bytes(tail[0..4].try_into().unwrap()), 123);
        assert_eq!(i64::from_le_bytes(tail[4..12].try_into().unwrap()), 456);
        assert_eq!(u32::from_le_bytes(tail[12..16].try_into().unwrap()), 789);
        assert_eq!(i64::from_le_bytes(tail[16..24].try_into().unwrap()), 101112);
    }

    #[test]
    fn active_player_coinage_values_update_matches_cpp_mask_shape() {
        let mut values = WorldPacket::new_empty();
        write_active_player_data_values_update(&mut values, &[], &[], None, Some(1234));

        let bytes = values.into_data();
        assert_eq!(&bytes[0..4], &[0x01, 0x00, 0x00, 0x00]); // group 0: block 0
        assert_eq!(&bytes[4..6], &[0x00, 0x00]); // group 1: no blocks 32..47
        assert_eq!(&bytes[6..10], &[0x10, 0x00, 0x00, 0x01]); // block 0: bits 0 and 28
        assert_eq!(u64::from_le_bytes(bytes[10..18].try_into().unwrap()), 1234);
        assert_eq!(bytes.len(), 18);
    }

    #[test]
    fn active_player_stats_values_update_matches_cpp_common_runtime_masks() {
        let mut combat_ratings = [0; 32];
        combat_ratings[0] = 11;
        combat_ratings[31] = 99;

        let stats = PlayerStatChanges {
            health: 0,
            max_health: 0,
            min_damage: 0.0,
            max_damage: 0.0,
            base_mana: 0,
            base_health: 0,
            attack_power: 0,
            ranged_attack_power: 0,
            min_ranged_damage: 0.0,
            max_ranged_damage: 0.0,
            power0: 0,
            max_power0: 0,
            stats: [0; 5],
            stat_pos_buff: [0; 5],
            armor: 0,
            combat_ratings,
            spell_power: 123,
            block_pct: 1.0,
            dodge_pct: 2.0,
            parry_pct: 3.0,
            crit_pct: 4.0,
            ranged_crit_pct: 5.0,
            spell_crit_pct: [6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0],
            mana_regen: 0.0,
            mana_regen_combat: 0.0,
            mana_regen_mp5: 0.0,
            mainhand_expertise: 13.0,
            offhand_expertise: 14.0,
            ranged_expertise: 15.0,
            combat_rating_expertise: 16.0,
            dodge_from_attr: 17.0,
            parry_from_attr: 18.0,
            offhand_crit_pct: 19.0,
            shield_block: 20,
            shield_block_crit_pct: 21.0,
            mod_healing_pct: 1.0,
            mod_healing_done_pct: 1.0,
            mod_periodic_healing_pct: 1.0,
            mod_spell_power_pct: 1.0,
        };

        let mut values = WorldPacket::new_empty();
        write_active_player_data_values_update(&mut values, &[], &[], Some(&stats), None);

        let bytes = values.into_data();
        assert_eq!(&bytes[0..4], &[0x07, 0x01, 0x06, 0x00]); // blocks 0,1,2,8,17,18
        assert_eq!(&bytes[4..6], &[0x00, 0x00]);
        assert_eq!(&bytes[6..10], &[0x00, 0x00, 0x00, 0x01]);
        assert_eq!(&bytes[10..14], &[0xFF, 0xFF, 0xFF, 0xF0]);
        assert_eq!(&bytes[14..18], &[0x00, 0x00, 0x00, 0x3F]);
        assert_eq!(&bytes[18..22], &[0x0F, 0xFF, 0xE0, 0x00]);
        assert_eq!(&bytes[22..26], &[0xC0, 0x00, 0x00, 0x00]);
        assert_eq!(&bytes[26..30], &[0x7F, 0xFF, 0xFF, 0xFF]);

        let expertise = 13.0f32.to_le_bytes();
        let values_start = bytes
            .windows(4)
            .position(|window| window == expertise)
            .expect("mainhand expertise value must be present after ActivePlayerData masks");
        let mut offset = values_start;
        assert_eq!(
            f32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap()),
            13.0
        );
        offset += 4;
        assert_eq!(
            f32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap()),
            14.0
        );
        offset += 4;
        offset += 31 * 4;

        for expected in [6.0f32, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0] {
            assert_eq!(
                f32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap()),
                expected
            );
            offset += 4;
            offset += 4; // ModDamageDonePos for the same school.
        }

        assert_eq!(
            i32::from_le_bytes(
                bytes[bytes.len() - 128..bytes.len() - 124]
                    .try_into()
                    .unwrap()
            ),
            11
        );
        assert_eq!(
            i32::from_le_bytes(bytes[bytes.len() - 4..].try_into().unwrap()),
            99
        );
    }

    fn set_active_player_bit(data: &mut ActivePlayerDataValuesUpdate, bit: usize) {
        data.active_player_data_mask[bit / 32] |= 1 << (bit % 32);
    }

    #[test]
    fn full_active_player_values_update_matches_cpp_coinage_shape() {
        let mut data = ActivePlayerDataValuesUpdate {
            coinage: 1234,
            ..Default::default()
        };
        set_active_player_bit(&mut data, 0);
        set_active_player_bit(&mut data, 28);

        let mut values = WorldPacket::new_empty();
        write_active_player_data_values_update_section(&mut values, &data);

        let bytes = values.into_data();
        assert_eq!(&bytes[0..4], &[0x01, 0x00, 0x00, 0x00]); // group 0: block 0
        assert_eq!(&bytes[4..6], &[0x00, 0x00]); // group 1: no blocks 32..47
        assert_eq!(&bytes[6..10], &[0x10, 0x00, 0x00, 0x01]); // block 0: bits 0 and 28
        assert_eq!(u64::from_le_bytes(bytes[10..18].try_into().unwrap()), 1234);
        assert_eq!(bytes.len(), 18);
    }

    #[test]
    fn full_active_player_values_update_block_uses_active_player_type_mask() {
        let mut data = ActivePlayerDataValuesUpdate {
            coinage: 1234,
            ..Default::default()
        };
        set_active_player_bit(&mut data, 0);
        set_active_player_bit(&mut data, 28);

        let mut block = WorldPacket::new_empty();
        write_full_active_player_values_update_block(&mut block, &ObjectGuid::EMPTY, &data);

        let bytes = block.into_data();
        assert_eq!(bytes[0], UpdateType::Values as u8);
        assert_eq!(&bytes[1..3], &[0, 0]);
        assert_eq!(
            u32::from_le_bytes(bytes[7..11].try_into().unwrap()),
            VALUES_TYPE_ACTIVE_PLAYER
        );
        assert_eq!(u64::from_le_bytes(bytes[21..29].try_into().unwrap()), 1234);
    }

    #[test]
    fn full_active_player_values_update_matches_cpp_late_array_order() {
        let mut data = ActivePlayerDataValuesUpdate::default();
        set_active_player_bit(&mut data, 636);
        set_active_player_bit(&mut data, 637);
        set_active_player_bit(&mut data, 1512);
        set_active_player_bit(&mut data, 1513);
        set_active_player_bit(&mut data, 1519);
        set_active_player_bit(&mut data, 607);
        set_active_player_bit(&mut data, 608);
        data.quest_completed[0] = 0x0102_0304_0506_0708;
        data.glyph_slots[0] = 55;
        data.glyphs[0] = 66;
        data.pvp_info[0] = PvpInfoValuesUpdate {
            pvp_info_mask: 0x0D,
            bracket: 7,
            pvp_rating_id: 99,
            ..Default::default()
        };

        let mut values = WorldPacket::new_empty();
        write_active_player_data_values_update_section(&mut values, &data);

        let bytes = values.into_data();
        let quest_pos = bytes
            .windows(8)
            .position(|window| window == 0x0102_0304_0506_0708u64.to_le_bytes())
            .expect("QuestCompleted value must be present");
        let glyph_slot_pos = bytes
            .windows(4)
            .position(|window| window == 55u32.to_le_bytes())
            .expect("GlyphSlots value must be present");
        let glyph_pos = bytes
            .windows(4)
            .position(|window| window == 66u32.to_le_bytes())
            .expect("Glyphs value must be present");
        let pvp_pos = bytes
            .windows(4)
            .position(|window| window == 99i32.to_le_bytes())
            .expect("PVP rating value must be present");

        assert!(quest_pos < glyph_slot_pos);
        assert!(glyph_slot_pos < glyph_pos);
        assert!(glyph_pos < pvp_pos);
    }

    #[test]
    fn skill_info_values_update_matches_cpp_mask_and_value_order() {
        let mut data = SkillInfoValuesUpdate::default();
        data.skill_info_mask[0] = (1 << 0) | (1 << 1);
        data.skill_info_mask[16] = 1 << 1; // global bit 513: SkillRank[0]
        data.skill_line_id[0] = 164;
        data.skill_rank[0] = 75;

        let mut values = WorldPacket::new_empty();
        write_skill_info_values_update(&mut values, &data);

        let bytes = values.into_data();
        assert_eq!(&bytes[0..4], &[0x01, 0x00, 0x01, 0x00]); // blocks 0 and 16
        assert_eq!(
            u16::from_le_bytes(bytes[bytes.len() - 4..bytes.len() - 2].try_into().unwrap()),
            164
        );
        assert_eq!(
            u16::from_le_bytes(bytes[bytes.len() - 2..].try_into().unwrap()),
            75
        );
    }

    #[test]
    fn active_player_nested_simple_values_update_match_cpp_order() {
        let mut research = WorldPacket::new_empty();
        write_research_values_update(
            &mut research,
            ResearchValuesUpdate {
                research_project_id: -123,
            },
        );
        assert_eq!(
            i16::from_le_bytes(research.into_data().try_into().unwrap()),
            -123
        );

        let mut rest = WorldPacket::new_empty();
        write_rest_info_values_update(
            &mut rest,
            RestInfoValuesUpdate {
                rest_info_mask: 0x07,
                threshold: 10_000,
                state_id: 3,
            },
        );
        let rest_bytes = rest.into_data();
        assert_eq!(rest_bytes[0] & 0xE0, 0xE0); // 3-bit mask 0b111
        assert_eq!(
            u32::from_le_bytes(rest_bytes[1..5].try_into().unwrap()),
            10_000
        );
        assert_eq!(rest_bytes[5], 3);

        let mut pvp = WorldPacket::new_empty();
        write_pvp_info_values_update(
            &mut pvp,
            PvpInfoValuesUpdate {
                pvp_info_mask: 0x0F,
                disqualified: true,
                bracket: -1,
                pvp_rating_id: 42,
                ..Default::default()
            },
        );
        let pvp_bytes = pvp.into_data();
        assert_eq!(pvp_bytes[pvp_bytes.len() - 5], 0xFFu8); // Bracket i8
        assert_eq!(
            i32::from_le_bytes(pvp_bytes[pvp_bytes.len() - 4..].try_into().unwrap()),
            42
        );
    }

    #[test]
    fn active_player_dynamic_entry_values_update_match_cpp_order() {
        let mut restriction = WorldPacket::new_empty();
        write_character_restriction_values_update(
            &mut restriction,
            CharacterRestrictionValuesUpdate {
                field_0: 1,
                field_4: 2,
                field_8: 3,
                restriction_type: 17,
            },
        );
        let restriction_bytes = restriction.into_data();
        assert_eq!(
            i32::from_le_bytes(restriction_bytes[0..4].try_into().unwrap()),
            1
        );
        assert_eq!(
            i32::from_le_bytes(restriction_bytes[4..8].try_into().unwrap()),
            2
        );
        assert_eq!(
            i32::from_le_bytes(restriction_bytes[8..12].try_into().unwrap()),
            3
        );
        assert_eq!(restriction_bytes[12] & 0xF8, 0x88); // 5-bit type 17.

        let mut pct = WorldPacket::new_empty();
        write_spell_pct_mod_by_label_values_update(
            &mut pct,
            SpellPctModByLabelValuesUpdate {
                mod_index: 4,
                modifier_value: 1.5,
                label_id: 6,
            },
        );
        let pct_bytes = pct.into_data();
        assert_eq!(i32::from_le_bytes(pct_bytes[0..4].try_into().unwrap()), 4);
        assert_eq!(f32::from_le_bytes(pct_bytes[4..8].try_into().unwrap()), 1.5);
        assert_eq!(i32::from_le_bytes(pct_bytes[8..12].try_into().unwrap()), 6);

        let mut flat = WorldPacket::new_empty();
        write_spell_flat_mod_by_label_values_update(
            &mut flat,
            SpellFlatModByLabelValuesUpdate {
                mod_index: 7,
                modifier_value: 8,
                label_id: 9,
            },
        );
        assert_eq!(flat.into_data(), [7, 0, 0, 0, 8, 0, 0, 0, 9, 0, 0, 0]);

        let mut cooldown = WorldPacket::new_empty();
        write_category_cooldown_mod_values_update(
            &mut cooldown,
            CategoryCooldownModValuesUpdate {
                spell_category_id: 10,
                mod_cooldown: 11,
            },
        );
        assert_eq!(cooldown.into_data(), [10, 0, 0, 0, 11, 0, 0, 0]);

        let mut weekly = WorldPacket::new_empty();
        write_weekly_spell_use_values_update(
            &mut weekly,
            WeeklySpellUseValuesUpdate {
                spell_category_id: 12,
                uses: 13,
            },
        );
        assert_eq!(weekly.into_data(), [12, 0, 0, 0, 13]);
    }

    #[test]
    fn active_player_dynamic_nested_values_update_match_cpp_order() {
        let mut research_history = WorldPacket::new_empty();
        write_research_history_values_update(
            &mut research_history,
            &ResearchHistoryValuesUpdate {
                research_history_mask: 0x03,
                completed_projects: vec![CompletedProjectValuesUpdate {
                    completed_project_mask: 0x0F,
                    project_id: 101,
                    first_completed: 202,
                    completion_count: 3,
                }],
                completed_projects_update_mask: None,
            },
        );
        let rh = research_history.into_data();
        assert_eq!(
            u32::from_le_bytes(rh[rh.len() - 16..rh.len() - 12].try_into().unwrap()),
            101
        );
        assert_eq!(
            i64::from_le_bytes(rh[rh.len() - 12..rh.len() - 4].try_into().unwrap()),
            202
        );
        assert_eq!(
            u32::from_le_bytes(rh[rh.len() - 4..].try_into().unwrap()),
            3
        );

        let mut trait_config = WorldPacket::new_empty();
        write_trait_config_values_update(
            &mut trait_config,
            &TraitConfigValuesUpdate {
                trait_config_mask: 0x07F,
                entries: vec![TraitEntryValuesUpdate {
                    trait_node_id: 1,
                    trait_node_entry_id: 2,
                    rank: 3,
                    granted_ranks: 4,
                }],
                entries_update_mask: None,
                id: 55,
                name: "Spec".to_string(),
                config_type: 2,
                skill_line_id: 777,
                ..Default::default()
            },
        );
        let tc = trait_config.into_data();
        assert!(tc.windows(4).any(|window| window == [1, 0, 0, 0]));
        assert!(tc.windows(4).any(|window| window == [55, 0, 0, 0]));
        assert!(tc.windows(4).any(|window| window == 777u32.to_le_bytes()));
        assert!(tc.windows(4).any(|window| window == b"Spec"));

        let mut stable = WorldPacket::new_empty();
        write_stable_info_values_update(
            &mut stable,
            &StableInfoValuesUpdate {
                stable_info_mask: 0x07,
                pets: vec![StablePetInfoValuesUpdate {
                    stable_pet_mask: 0xFF,
                    pet_slot: 1,
                    pet_number: 2,
                    creature_id: 3,
                    display_id: 4,
                    experience_level: 5,
                    name: "Pet".to_string(),
                    pet_flags: 6,
                }],
                pets_update_mask: None,
                stable_master: ObjectGuid::EMPTY,
            },
        );
        let stable_bytes = stable.into_data();
        assert!(stable_bytes.windows(4).any(|window| window == [1, 0, 0, 0]));
        assert!(stable_bytes.windows(4).any(|window| window == [5, 0, 0, 0]));
        assert!(stable_bytes.windows(3).any(|window| window == b"Pet"));
    }

    fn test_player_create_data_with_farsight(farsight_object: ObjectGuid) -> PlayerCreateData {
        PlayerCreateData {
            guid: ObjectGuid::create_player(1, 42),
            race: 1,
            class: 1,
            sex: 0,
            level: 1,
            display_id: 49,
            native_display_id: 49,
            health: 100,
            max_health: 100,
            faction_template: PlayerCreateData::faction_for_race(1),
            zone_id: 12,
            stats: [0; 5],
            base_armor: 0,
            max_mana: 0,
            attack_power: 0,
            ranged_attack_power: 0,
            min_damage: 1.0,
            max_damage: 2.0,
            min_ranged_damage: 0.0,
            max_ranged_damage: 0.0,
            dodge_pct: 0.0,
            parry_pct: 0.0,
            crit_pct: 5.0,
            ranged_crit_pct: 5.0,
            spell_crit_pct: 0.0,
            visible_items: [(0, 0, 0); 19],
            inv_slots: [ObjectGuid::EMPTY; 141],
            farsight_object,
            skill_info: Vec::new(),
            quest_log: Vec::new(),
            coinage: 0,
            watched_faction_index: -1,
            heirlooms: Vec::new(),
            heirloom_flags: Vec::new(),
            toys: Vec::new(),
        }
    }

    #[test]
    fn active_player_create_writes_farsight_after_inventory_slots() {
        let farsight_object = ObjectGuid::new(0x0102_0304_0506_0708, 0x1112_1314_1516_1718);
        let create = test_player_create_data_with_farsight(farsight_object);
        let mut packet = WorldPacket::new_empty();
        create.write_active_player_data(&mut packet);
        let data = packet.data();

        let mut expected_guid = WorldPacket::new_empty();
        expected_guid.write_packed_guid(&farsight_object);
        let expected_guid = expected_guid.into_data();
        let farsight_offset = 141 * 2;
        let summoned_battle_pet_offset = farsight_offset + expected_guid.len();

        assert_ne!(expected_guid, [0, 0]);
        assert_eq!(
            &data[farsight_offset..summoned_battle_pet_offset],
            expected_guid.as_slice()
        );
        assert_eq!(
            &data[summoned_battle_pet_offset..summoned_battle_pet_offset + 2],
            [0, 0]
        );
    }

    #[test]
    fn active_player_create_writes_collection_dynamic_fields_like_cpp() {
        let mut create = test_player_create_data_with_farsight(ObjectGuid::EMPTY);
        create.heirlooms = vec![44_000, 44_001];
        create.heirloom_flags = vec![0x03, 0x04];
        create.toys = vec![30_000];

        let mut packet = WorldPacket::new_empty();
        create.write_active_player_data(&mut packet);
        let data = packet.data();

        assert!(data.windows(12).any(|window| {
            window
                == [
                    2, 0, 0, 0, // Heirlooms.Size
                    2, 0, 0, 0, // HeirloomFlags.Size
                    1, 0, 0, 0, // Toys.Size
                ]
        }));
        assert!(
            data.windows(12)
                .any(|window| window == [224, 171, 0, 0, 225, 171, 0, 0, 3, 0, 0, 0])
        );
        assert!(
            data.windows(8)
                .any(|window| window == [4, 0, 0, 0, 48, 117, 0, 0])
        );
    }

    #[test]
    fn create_player_defaults_farsight_object_empty() {
        let guid = ObjectGuid::create_player(1, 42);
        let pos = Position::new(0.0, 0.0, 0.0, 0.0);
        let packet = UpdateObject::create_player(
            guid,
            1,
            1,
            0,
            1,
            49,
            &pos,
            0,
            12,
            true,
            [(0, 0, 0); 19],
            [ObjectGuid::EMPTY; 141],
            PlayerCombatStats::default(),
            Vec::new(),
            0,
            Vec::new(),
        );

        let UpdateBlock::CreateObject { create_data, .. } = &packet.blocks[0] else {
            panic!("create_player should emit one CreateObject block");
        };
        assert_eq!(create_data.farsight_object, ObjectGuid::EMPTY);
    }

    #[test]
    fn create_player_non_self() {
        // Non-self player should be smaller (no ActivePlayerData)
        let guid = ObjectGuid::create_player(1, 42);
        let pos = Position::new(0.0, 0.0, 0.0, 0.0);
        let self_pkt = UpdateObject::create_player(
            guid,
            1,
            1,
            0,
            1,
            49,
            &pos,
            0,
            12,
            true,
            [(0, 0, 0); 19],
            [ObjectGuid::EMPTY; 141],
            PlayerCombatStats::default(),
            Vec::new(),
            0,
            Vec::new(),
        );
        let other_pkt = UpdateObject::create_player(
            guid,
            1,
            1,
            0,
            1,
            49,
            &pos,
            0,
            12,
            false,
            [(0, 0, 0); 19],
            [ObjectGuid::EMPTY; 141],
            PlayerCombatStats::default(),
            Vec::new(),
            0,
            Vec::new(),
        );
        let self_bytes = self_pkt.to_bytes();
        let other_bytes = other_pkt.to_bytes();
        // Self packet should be much larger due to ActivePlayerData
        assert!(
            self_bytes.len() > other_bytes.len() + 1000,
            "Self ({}) should be much larger than other ({})",
            self_bytes.len(),
            other_bytes.len()
        );
    }

    #[test]
    fn power_type_mapping() {
        assert_eq!(power_type_for_class(1), 1); // Warrior → Rage
        assert_eq!(power_type_for_class(2), 0); // Paladin → Mana
        assert_eq!(power_type_for_class(4), 3); // Rogue → Energy
        assert_eq!(power_type_for_class(6), 5); // DK → Runic Power
    }

    #[test]
    fn creature_create_serializes() {
        let guid = ObjectGuid::create_world_object(
            wow_core::guid::HighGuid::Creature,
            0,
            1,
            0,
            1,
            1234,
            5678,
        );
        let pos = Position::new(-8949.0, -132.0, 83.0, 0.0);
        let data = CreatureCreateData {
            guid,
            entry: 1234,
            display_id: 856,
            native_display_id: 856,
            health: 500,
            max_health: 500,
            level: 5,
            faction_template: 14,
            npc_flags: 0,
            unit_flags: 0,
            unit_flags2: 0,
            unit_flags3: 0,
            damage_school: wow_constants::spell::SpellSchools::Normal as u8,
            scale: 1.0,
            unit_class: 1,
            base_attack_time: 2000,
            ranged_attack_time: 0,
            zone_id: 12,
            speed_walk_rate: 1.0,
            speed_run_rate: 1.14286,
            ai_anim_kit_id: 0,
            movement_anim_kit_id: 0,
            melee_anim_kit_id: 0,
        };
        let block = UpdateObject::create_creature_block(data, &pos);
        let pkt = UpdateObject::create_creatures(vec![block], 0);
        let bytes = pkt.to_bytes();
        // Creature packet should be much smaller than player (no PlayerData/ActivePlayerData)
        assert!(
            bytes.len() > 100,
            "Creature packet too small: {} bytes",
            bytes.len()
        );
        assert!(
            bytes.len() < 2000,
            "Creature packet too large: {} bytes",
            bytes.len()
        );
    }

    #[test]
    fn creature_create_serializes_cpp_anim_kit_block_when_any_anim_kit_is_set() {
        let guid = ObjectGuid::create_world_object(
            wow_core::guid::HighGuid::Creature,
            0,
            1,
            0,
            1,
            1234,
            5678,
        );
        let pos = Position::new(-8949.0, -132.0, 83.0, 0.0);
        let data = CreatureCreateData {
            guid,
            entry: 1234,
            display_id: 856,
            native_display_id: 856,
            health: 500,
            max_health: 500,
            level: 5,
            faction_template: 14,
            npc_flags: 0,
            unit_flags: 0,
            unit_flags2: 0,
            unit_flags3: 0,
            damage_school: wow_constants::spell::SpellSchools::Normal as u8,
            scale: 1.0,
            unit_class: 1,
            base_attack_time: 2000,
            ranged_attack_time: 0,
            zone_id: 12,
            speed_walk_rate: 1.0,
            speed_run_rate: 1.14286,
            ai_anim_kit_id: 11,
            movement_anim_kit_id: 22,
            melee_anim_kit_id: 33,
        };
        let block = UpdateObject::create_creature_block(data, &pos);
        let pkt = UpdateObject::create_creatures(vec![block], 0);
        let bytes = pkt.to_bytes();

        assert!(
            bytes
                .windows(6)
                .any(|window| window == [11, 0, 22, 0, 33, 0]),
            "C++ CreateObjectBits::AnimKit writes AiID, MovementID, MeleeID as u16 payload"
        );
    }

    #[test]
    fn creature_smaller_than_player() {
        let player_guid = ObjectGuid::create_player(1, 42);
        let creature_guid =
            ObjectGuid::create_world_object(wow_core::guid::HighGuid::Creature, 0, 1, 0, 1, 100, 1);
        let pos = Position::new(0.0, 0.0, 0.0, 0.0);

        let player_pkt = UpdateObject::create_player(
            player_guid,
            1,
            1,
            0,
            1,
            49,
            &pos,
            0,
            12,
            false,
            [(0, 0, 0); 19],
            [ObjectGuid::EMPTY; 141],
            PlayerCombatStats::default(),
            Vec::new(),
            0,
            Vec::new(),
        );

        let creature_data = CreatureCreateData {
            guid: creature_guid,
            entry: 100,
            display_id: 856,
            native_display_id: 856,
            health: 100,
            max_health: 100,
            level: 1,
            faction_template: 14,
            npc_flags: 0,
            unit_flags: 0,
            unit_flags2: 0,
            unit_flags3: 0,
            damage_school: wow_constants::spell::SpellSchools::Normal as u8,
            scale: 1.0,
            unit_class: 1,
            base_attack_time: 2000,
            ranged_attack_time: 0,
            zone_id: 12,
            speed_walk_rate: 1.0,
            speed_run_rate: 1.14286,
            ai_anim_kit_id: 0,
            movement_anim_kit_id: 0,
            melee_anim_kit_id: 0,
        };
        let block = UpdateObject::create_creature_block(creature_data, &pos);
        let creature_pkt = UpdateObject::create_creatures(vec![block], 0);

        let player_bytes = player_pkt.to_bytes();
        let creature_bytes = creature_pkt.to_bytes();

        // Creature has no PlayerData, so it should be smaller than even a non-self player
        assert!(
            creature_bytes.len() < player_bytes.len(),
            "Creature ({}) should be smaller than non-self player ({})",
            creature_bytes.len(),
            player_bytes.len()
        );
    }

    #[test]
    fn creature_batched_multiple() {
        let pos = Position::new(0.0, 0.0, 0.0, 0.0);
        let mut blocks = Vec::new();
        for i in 0..5 {
            let guid = ObjectGuid::create_world_object(
                wow_core::guid::HighGuid::Creature,
                0,
                1,
                0,
                1,
                100,
                i,
            );
            let data = CreatureCreateData {
                guid,
                entry: 100,
                display_id: 856,
                native_display_id: 856,
                health: 100,
                max_health: 100,
                level: 1,
                faction_template: 14,
                npc_flags: 0,
                unit_flags: 0,
                unit_flags2: 0,
                unit_flags3: 0,
                damage_school: wow_constants::spell::SpellSchools::Normal as u8,
                scale: 1.0,
                unit_class: 1,
                base_attack_time: 2000,
                ranged_attack_time: 0,
                zone_id: 12,
                speed_walk_rate: 1.0,
                speed_run_rate: 1.14286,
                ai_anim_kit_id: 0,
                movement_anim_kit_id: 0,
                melee_anim_kit_id: 0,
            };
            blocks.push(UpdateObject::create_creature_block(data, &pos));
        }
        let pkt = UpdateObject::create_creatures(blocks, 0);
        let bytes = pkt.to_bytes();

        // 5 creatures should be 5x the single creature data
        assert!(
            bytes.len() > 500,
            "Batched packet too small: {} bytes",
            bytes.len()
        );

        // Check num_updates = 5
        let num_updates = u32::from_le_bytes([bytes[2], bytes[3], bytes[4], bytes[5]]);
        assert_eq!(num_updates, 5);
    }

    #[test]
    fn creature_npc_flags_written_correctly() {
        // Verify that NpcFlags value appears in the creature's values block.
        // NpcFlags=1 (Gossip) should be written as 0x01000000 in the packet.
        let guid = ObjectGuid::create_world_object(
            wow_core::guid::HighGuid::Creature,
            0,
            1,
            0,
            1,
            3296,
            1,
        );
        let pos = Position::new(1600.0, -4400.0, 10.0, 0.0);
        let data = CreatureCreateData {
            guid,
            entry: 3296,
            display_id: 4500,
            native_display_id: 4500,
            health: 500,
            max_health: 500,
            level: 55,
            faction_template: 85,
            npc_flags: 0x1_0000_0001, // Gossip flag plus NPCFlags2 bit 0
            unit_flags: 32768,
            unit_flags2: 2048,
            unit_flags3: 0,
            damage_school: wow_constants::spell::SpellSchools::Normal as u8,
            scale: 1.0,
            unit_class: 1,
            base_attack_time: 2000,
            ranged_attack_time: 0,
            zone_id: 1637,
            speed_walk_rate: 1.0,
            speed_run_rate: 1.14286,
            ai_anim_kit_id: 0,
            movement_anim_kit_id: 0,
            melee_anim_kit_id: 0,
        };
        let block = UpdateObject::create_creature_block(data, &pos);
        let pkt = UpdateObject::create_creatures(vec![block], 1);
        let bytes = pkt.to_bytes();

        // Find NpcFlags=1 in the packet bytes.
        // The values block contains:
        //   [u8 flags=0x00]
        //   [i32 EntryId] [u32 DynamicFlags] [f32 Scale]  (ObjectData: 4+4+4=12 bytes)
        //   [i64 Health] [i64 MaxHealth] [i32 DisplayId]   (UnitData: 8+8+4=20 bytes)
        //   [u32 NpcFlags[0]] [u32 NpcFlags[1]]            (UnitData: 4+4=8 bytes)
        // So NpcFlags[0] starts at offset 1+12+20 = 33 from values block start.
        // The value 1 in little-endian is [0x01, 0x00, 0x00, 0x00].
        // Search for this pattern preceded by DisplayId (4500 = 0x94110000 LE).
        let display_le = 4500u32.to_le_bytes();
        let npc_le = 1u32.to_le_bytes();
        let npc2_le = 1u32.to_le_bytes();
        let mut found = false;
        for i in 0..bytes.len().saturating_sub(8) {
            if bytes[i..i + 4] == display_le && bytes[i + 4..i + 8] == npc_le {
                found = true;
                // Also check NpcFlags[1] = 1
                assert_eq!(bytes[i + 8..i + 12], npc2_le, "NpcFlags[1] should be 1");
                break;
            }
        }
        assert!(
            found,
            "NpcFlags=1 not found after DisplayId={} in packet ({} bytes). \
            This means NpcFlags are not being written correctly!",
            4500,
            bytes.len()
        );
    }

    #[test]
    fn active_player_movement_block_adds_721_bytes() {
        // Self-view packets include a 721-byte ActivePlayer block in
        // BuildMovementUpdate: 1 byte (3 bits + flush) + 180 action buttons (720 bytes).
        // Non-self packets don't have this block.
        let guid = ObjectGuid::create_player(1, 42);
        let pos = Position::new(0.0, 0.0, 0.0, 0.0);
        let self_pkt = UpdateObject::create_player(
            guid,
            1,
            1,
            0,
            1,
            49,
            &pos,
            0,
            12,
            true,
            [(0, 0, 0); 19],
            [ObjectGuid::EMPTY; 141],
            PlayerCombatStats::default(),
            Vec::new(),
            0,
            Vec::new(),
        );
        let other_pkt = UpdateObject::create_player(
            guid,
            1,
            1,
            0,
            1,
            49,
            &pos,
            0,
            12,
            false,
            [(0, 0, 0); 19],
            [ObjectGuid::EMPTY; 141],
            PlayerCombatStats::default(),
            Vec::new(),
            0,
            Vec::new(),
        );
        let self_bytes = self_pkt.to_bytes();
        let other_bytes = other_pkt.to_bytes();

        // The difference between self and non-self should include:
        // - 721 bytes from ActivePlayer movement block
        // - plus the ActivePlayerData values block difference
        // The ActivePlayer movement block alone is 721 bytes.
        let diff = self_bytes.len() - other_bytes.len();
        assert!(
            diff > 721,
            "Self/non-self difference ({}) should be > 721 (ActivePlayer block)",
            diff
        );
    }
}
