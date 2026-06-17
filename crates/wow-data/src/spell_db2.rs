//! Typed Spell* DB2 readers.

use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result};
use tracing::info;

use crate::wdc4::Wdc4Reader;

pub const MAX_SPELL_REAGENTS: usize = 8;
pub const MAX_SPELL_AURA_INTERRUPT_FLAGS: usize = 2;
pub const MAX_SHAPESHIFT_SPELLS: usize = 8;
pub const MAX_SPELL_TOTEMS: usize = 2;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpellAuraOptionsEntry {
    pub id: u32,
    pub difficulty_id: u8,
    pub cumulative_aura: u32,
    pub proc_category_recovery: i32,
    pub proc_chance: u8,
    pub proc_charges: i32,
    pub spell_procs_per_minute_id: u16,
    pub proc_type_mask: [i32; 2],
    pub spell_id: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpellAuraRestrictionsEntry {
    pub id: u32,
    pub difficulty_id: u8,
    pub caster_aura_state: u8,
    pub target_aura_state: u8,
    pub exclude_caster_aura_state: u8,
    pub exclude_target_aura_state: u8,
    pub caster_aura_spell: i32,
    pub target_aura_spell: i32,
    pub exclude_caster_aura_spell: i32,
    pub exclude_target_aura_spell: i32,
    pub spell_id: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpellCastTimesEntry {
    pub id: u32,
    pub base: i32,
    pub per_level: i16,
    pub minimum: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpellCastingRequirementsEntry {
    pub id: u32,
    pub spell_id: i32,
    pub facing_caster_flags: u8,
    pub min_faction_id: u16,
    pub min_reputation: i32,
    pub required_areas_id: u16,
    pub required_aura_vision: u8,
    pub requires_spell_focus: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpellCategoriesEntry {
    pub id: u32,
    pub difficulty_id: u8,
    pub category: i16,
    pub defense_type: i8,
    pub dispel_type: i8,
    pub mechanic: i8,
    pub prevention_type: i8,
    pub start_recovery_category: i16,
    pub charge_category: i16,
    pub spell_id: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpellCategoryEntry {
    pub id: u32,
    pub name: String,
    pub flags: i32,
    pub uses_per_week: u8,
    pub max_charges: i8,
    pub charge_recovery_time: i32,
    pub type_mask: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpellClassOptionsEntry {
    pub id: u32,
    pub spell_id: i32,
    pub modal_next_spell: u32,
    pub spell_class_set: u8,
    pub spell_class_mask: [u32; 4],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpellCooldownsEntry {
    pub id: u32,
    pub difficulty_id: u8,
    pub category_recovery_time: i32,
    pub recovery_time: i32,
    pub start_recovery_time: i32,
    pub spell_id: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpellDurationEntry {
    pub id: u32,
    pub duration: i32,
    pub duration_per_level: u32,
    pub max_duration: i32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SpellEffectDb2Entry {
    pub id: u32,
    pub difficulty_id: i32,
    pub effect_index: i32,
    pub effect: u32,
    pub effect_amplitude: f32,
    pub effect_attributes: i32,
    pub effect_aura: i16,
    pub effect_aura_period: i32,
    pub effect_base_points: i32,
    pub effect_bonus_coefficient: f32,
    pub effect_chain_amplitude: f32,
    pub effect_chain_targets: i32,
    pub effect_die_sides: i32,
    pub effect_item_type: i32,
    pub effect_mechanic: i32,
    pub effect_points_per_resource: f32,
    pub effect_pos_facing: f32,
    pub effect_real_points_per_level: f32,
    pub effect_trigger_spell: i32,
    pub bonus_coefficient_from_ap: f32,
    pub pvp_multiplier: f32,
    pub coefficient: f32,
    pub variance: f32,
    pub resource_coefficient: f32,
    pub group_size_base_points_coefficient: f32,
    pub effect_misc_value: [i32; 2],
    pub effect_radius_index: [u32; 2],
    pub effect_spell_class_mask: [u32; 4],
    pub implicit_target: [i16; 2],
    pub spell_id: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpellEquippedItemsEntry {
    pub id: u32,
    pub spell_id: i32,
    pub equipped_item_class: i8,
    pub equipped_item_inv_types: i32,
    pub equipped_item_subclass: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpellFocusObjectEntry {
    pub id: u32,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpellInterruptsEntry {
    pub id: u32,
    pub difficulty_id: u8,
    pub interrupt_flags: i16,
    pub aura_interrupt_flags: [i32; MAX_SPELL_AURA_INTERRUPT_FLAGS],
    pub channel_interrupt_flags: [i32; MAX_SPELL_AURA_INTERRUPT_FLAGS],
    pub spell_id: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpellItemEnchantmentConditionEntry {
    pub id: u32,
    pub lt_operand_type: [u8; 5],
    pub lt_operand: [u32; 5],
    pub operator: [u8; 5],
    pub rt_operand_type: [u8; 5],
    pub rt_operand: [u8; 5],
    pub logic: [u8; 5],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpellKeyboundOverrideEntry {
    pub id: u32,
    pub function: String,
    pub override_type: i8,
    pub data: i32,
    pub flags: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpellLabelEntry {
    pub id: u32,
    pub label_id: u32,
    pub spell_id: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpellLearnSpellEntry {
    pub id: u32,
    pub spell_id: i32,
    pub learn_spell_id: i32,
    pub overrides_spell_id: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpellLevelsEntry {
    pub id: u32,
    pub difficulty_id: u8,
    pub base_level: i16,
    pub max_level: i16,
    pub spell_level: i16,
    pub max_passive_aura_level: u8,
    pub spell_id: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SpellMiscEntry {
    pub id: u32,
    pub attributes: [i32; 15],
    pub difficulty_id: u8,
    pub casting_time_index: u16,
    pub duration_index: u16,
    pub range_index: u16,
    pub school_mask: u8,
    pub speed: f32,
    pub launch_delay: f32,
    pub min_duration: f32,
    pub spell_icon_file_data_id: i32,
    pub active_icon_file_data_id: i32,
    pub content_tuning_id: i32,
    pub show_future_spell_player_condition_id: i32,
    pub spell_id: u32,
}

impl SpellMiscEntry {
    /// C++ `SpellInfo::IsPassive`.
    pub fn is_passive_like_cpp(&self) -> bool {
        (self.attributes[0] as u32 & crate::spell::attributes::SPELL_ATTR0_PASSIVE) != 0
    }

    /// C++ `SpellInfo::IsAutocastable`.
    pub fn is_autocastable_like_cpp(&self) -> bool {
        !self.is_passive_like_cpp()
            && (self.attributes[1] as u32 & crate::spell::attributes::SPELL_ATTR1_NO_AUTOCAST_AI)
                == 0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpellNameEntry {
    pub id: u32,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SpellPowerEntry {
    pub id: u32,
    pub order_index: u8,
    pub mana_cost: i32,
    pub mana_cost_per_level: i32,
    pub mana_per_second: i32,
    pub power_display_id: u32,
    pub alt_power_bar_id: i32,
    pub power_cost_pct: f32,
    pub power_cost_max_pct: f32,
    pub power_pct_per_second: f32,
    pub power_type: i8,
    pub required_aura_spell_id: i32,
    pub optional_cost: u32,
    pub spell_id: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpellPowerDifficultyEntry {
    pub id: u32,
    pub difficulty_id: u8,
    pub order_index: u8,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SpellProcsPerMinuteEntry {
    pub id: u32,
    pub base_proc_rate: f32,
    pub flags: u8,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SpellProcsPerMinuteModEntry {
    pub id: u32,
    pub mod_type: u8,
    pub param: i16,
    pub coeff: f32,
    pub spell_procs_per_minute_id: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SpellRadiusEntry {
    pub id: u32,
    pub radius: f32,
    pub radius_per_level: f32,
    pub radius_min: f32,
    pub radius_max: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SpellRangeEntry {
    pub id: u32,
    pub display_name: String,
    pub display_name_short: String,
    pub flags: u8,
    pub range_min: [f32; 2],
    pub range_max: [f32; 2],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpellReagentsEntry {
    pub id: u32,
    pub spell_id: i32,
    pub reagent: [i32; MAX_SPELL_REAGENTS],
    pub reagent_count: [i16; MAX_SPELL_REAGENTS],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpellReagentsCurrencyEntry {
    pub id: u32,
    pub spell_id: u32,
    pub currency_types_id: u16,
    pub currency_count: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpellScalingEntry {
    pub id: u32,
    pub spell_id: i32,
    pub class: i32,
    pub min_scaling_level: u32,
    pub max_scaling_level: u32,
    pub scales_from_item_level: i16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpellShapeshiftEntry {
    pub id: u32,
    pub spell_id: i32,
    pub stance_bar_order: i8,
    pub shapeshift_exclude: [i32; 2],
    pub shapeshift_mask: [i32; 2],
}

#[derive(Debug, Clone, PartialEq)]
pub struct SpellShapeshiftFormEntry {
    pub id: u32,
    pub name: String,
    pub creature_type: i8,
    pub flags: i32,
    pub attack_icon_file_id: i32,
    pub bonus_action_bar: i8,
    pub combat_round_time: i16,
    pub damage_variance: f32,
    pub mount_type_id: u16,
    pub creature_display_id: [u32; 4],
    pub preset_spell_id: [u32; MAX_SHAPESHIFT_SPELLS],
}

#[derive(Debug, Clone, PartialEq)]
pub struct SpellTargetRestrictionsEntry {
    pub id: u32,
    pub difficulty_id: u8,
    pub cone_degrees: f32,
    pub max_targets: u8,
    pub max_target_level: u32,
    pub target_creature_type: i16,
    pub targets: i32,
    pub width: f32,
    pub spell_id: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpellTotemsEntry {
    pub id: u32,
    pub spell_id: i32,
    pub required_totem_category_id: [u16; MAX_SPELL_TOTEMS],
    pub totem: [i32; MAX_SPELL_TOTEMS],
}

#[derive(Debug, Clone, PartialEq)]
pub struct SpellVisualEntry {
    pub id: u32,
    pub missile_cast_offset: [f32; 3],
    pub missile_impact_offset: [f32; 3],
    pub anim_event_sound_id: u32,
    pub flags: i32,
    pub missile_attachment: i8,
    pub missile_destination_attachment: i8,
    pub missile_cast_positioner_id: u32,
    pub missile_impact_positioner_id: u32,
    pub missile_targeting_kit: i32,
    pub hostile_spell_visual_id: u32,
    pub caster_spell_visual_id: u32,
    pub spell_visual_missile_set_id: u16,
    pub damage_number_delay: u16,
    pub low_violence_spell_visual_id: u32,
    pub raid_spell_visual_missile_set_id: u32,
    pub reduced_unexpected_camera_movement_spell_visual_id: i32,
    pub area_model: u16,
    pub has_missile: i8,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SpellVisualEffectNameEntry {
    pub id: u32,
    pub model_file_data_id: i32,
    pub base_missile_speed: f32,
    pub scale: f32,
    pub min_allowed_scale: f32,
    pub max_allowed_scale: f32,
    pub alpha: f32,
    pub flags: u32,
    pub texture_file_data_id: i32,
    pub effect_radius: f32,
    pub effect_type: u32,
    pub generic_id: i32,
    pub ribbon_quality_id: u32,
    pub dissolve_effect_id: i32,
    pub model_position: i32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SpellVisualKitEntry {
    pub id: u32,
    pub fallback_spell_visual_kit_id: u32,
    pub delay_min: u16,
    pub delay_max: u16,
    pub fallback_priority: f32,
    pub flags: [i32; 2],
}

#[derive(Debug, Clone, PartialEq)]
pub struct SpellVisualMissileEntry {
    pub id: u32,
    pub cast_offset: [f32; 3],
    pub impact_offset: [f32; 3],
    pub spell_visual_effect_name_id: u16,
    pub sound_entries_id: u32,
    pub attachment: i8,
    pub destination_attachment: i8,
    pub cast_positioner_id: u16,
    pub impact_positioner_id: u16,
    pub follow_ground_height: i32,
    pub follow_ground_drop_speed: u32,
    pub follow_ground_approach: u16,
    pub flags: u32,
    pub spell_missile_motion_id: u16,
    pub anim_kit_id: u32,
    pub spell_visual_missile_set_id: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SpellXSpellVisualEntry {
    pub id: u32,
    pub difficulty_id: u8,
    pub spell_visual_id: u32,
    pub probability: f32,
    pub flags: u8,
    pub priority: i32,
    pub spell_icon_file_id: i32,
    pub active_icon_file_id: i32,
    pub viewer_unit_condition_id: u16,
    pub viewer_player_condition_id: u32,
    pub caster_unit_condition_id: u16,
    pub caster_player_condition_id: u32,
    pub spell_id: u32,
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

db2_store!(SpellAuraOptionsStore, SpellAuraOptionsEntry);
db2_store!(SpellAuraRestrictionsStore, SpellAuraRestrictionsEntry);
db2_store!(SpellCastTimesStore, SpellCastTimesEntry);
db2_store!(SpellCastingRequirementsStore, SpellCastingRequirementsEntry);
db2_store!(SpellCategoriesStore, SpellCategoriesEntry);
db2_store!(SpellCategoryStore, SpellCategoryEntry);
db2_store!(SpellClassOptionsStore, SpellClassOptionsEntry);
db2_store!(SpellCooldownsStore, SpellCooldownsEntry);
db2_store!(SpellDurationStore, SpellDurationEntry);
db2_store!(SpellEffectDb2Store, SpellEffectDb2Entry);
db2_store!(SpellEquippedItemsStore, SpellEquippedItemsEntry);
db2_store!(SpellFocusObjectStore, SpellFocusObjectEntry);
db2_store!(SpellInterruptsStore, SpellInterruptsEntry);
db2_store!(
    SpellItemEnchantmentConditionStore,
    SpellItemEnchantmentConditionEntry
);
db2_store!(SpellKeyboundOverrideStore, SpellKeyboundOverrideEntry);
db2_store!(SpellLabelStore, SpellLabelEntry);
db2_store!(SpellLearnSpellStore, SpellLearnSpellEntry);
db2_store!(SpellLevelsStore, SpellLevelsEntry);
db2_store!(SpellMiscStore, SpellMiscEntry);
db2_store!(SpellNameStore, SpellNameEntry);
db2_store!(SpellPowerStore, SpellPowerEntry);
db2_store!(SpellPowerDifficultyStore, SpellPowerDifficultyEntry);
db2_store!(SpellProcsPerMinuteStore, SpellProcsPerMinuteEntry);
db2_store!(SpellProcsPerMinuteModStore, SpellProcsPerMinuteModEntry);
db2_store!(SpellRadiusStore, SpellRadiusEntry);
db2_store!(SpellRangeStore, SpellRangeEntry);
db2_store!(SpellReagentsStore, SpellReagentsEntry);
db2_store!(SpellReagentsCurrencyStore, SpellReagentsCurrencyEntry);
db2_store!(SpellScalingStore, SpellScalingEntry);
db2_store!(SpellShapeshiftStore, SpellShapeshiftEntry);
db2_store!(SpellShapeshiftFormStore, SpellShapeshiftFormEntry);
db2_store!(SpellTargetRestrictionsStore, SpellTargetRestrictionsEntry);
db2_store!(SpellTotemsStore, SpellTotemsEntry);
db2_store!(SpellVisualStore, SpellVisualEntry);
db2_store!(SpellVisualEffectNameStore, SpellVisualEffectNameEntry);
db2_store!(SpellVisualKitStore, SpellVisualKitEntry);
db2_store!(SpellVisualMissileStore, SpellVisualMissileEntry);
db2_store!(SpellXSpellVisualStore, SpellXSpellVisualEntry);

impl SpellAuraOptionsStore {
    pub fn entry_for_spell_difficulty_like_cpp(
        &self,
        spell_id: u32,
        difficulty_id: u8,
    ) -> Option<&SpellAuraOptionsEntry> {
        self.entries
            .values()
            .find(|entry| entry.spell_id == spell_id && entry.difficulty_id == difficulty_id)
            .or_else(|| {
                self.entries
                    .values()
                    .find(|entry| entry.spell_id == spell_id && entry.difficulty_id == 0)
            })
    }

    /// C++ `SpellInfo::ProcCharges`, hydrated from `SpellAuraOptionsEntry`.
    pub fn proc_charges_like_cpp(&self, spell_id: u32, difficulty_id: u8) -> u8 {
        self.entry_for_spell_difficulty_like_cpp(spell_id, difficulty_id)
            .map(|entry| entry.proc_charges.clamp(0, i32::from(u8::MAX)) as u8)
            .unwrap_or(0)
    }

    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "SpellAuraOptions.db2", |id, idx, r| {
            SpellAuraOptionsEntry {
                id,
                difficulty_id: r.get_field_u8(idx, 0),
                cumulative_aura: r.get_field_u32(idx, 1),
                proc_category_recovery: r.get_field_i32(idx, 2),
                proc_chance: r.get_field_u8(idx, 3),
                proc_charges: r.get_field_i32(idx, 4),
                spell_procs_per_minute_id: r.get_field_u16(idx, 5),
                proc_type_mask: std::array::from_fn(|i| r.get_array_element(idx, 6, i, 32) as i32),
                spell_id: r.get_relationship_id(idx).unwrap_or(0),
            }
        })
    }
}

impl SpellAuraRestrictionsStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(
            data_dir,
            locale,
            "SpellAuraRestrictions.db2",
            |id, idx, r| SpellAuraRestrictionsEntry {
                id,
                difficulty_id: r.get_field_u8(idx, 0),
                caster_aura_state: r.get_field_u8(idx, 1),
                target_aura_state: r.get_field_u8(idx, 2),
                exclude_caster_aura_state: r.get_field_u8(idx, 3),
                exclude_target_aura_state: r.get_field_u8(idx, 4),
                caster_aura_spell: r.get_field_i32(idx, 5),
                target_aura_spell: r.get_field_i32(idx, 6),
                exclude_caster_aura_spell: r.get_field_i32(idx, 7),
                exclude_target_aura_spell: r.get_field_i32(idx, 8),
                spell_id: r.get_relationship_id(idx).unwrap_or(0),
            },
        )
    }
}

impl SpellCastTimesStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "SpellCastTimes.db2", |id, idx, r| {
            SpellCastTimesEntry {
                id,
                base: r.get_field_i32(idx, 0),
                per_level: r.get_field_i16(idx, 1),
                minimum: r.get_field_i32(idx, 2),
            }
        })
    }
}

impl SpellCastingRequirementsStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(
            data_dir,
            locale,
            "SpellCastingRequirements.db2",
            |id, idx, r| SpellCastingRequirementsEntry {
                id,
                spell_id: r.get_field_i32(idx, 0),
                facing_caster_flags: r.get_field_u8(idx, 1),
                min_faction_id: r.get_field_u16(idx, 2),
                min_reputation: r.get_field_i32(idx, 3),
                required_areas_id: r.get_field_u16(idx, 4),
                required_aura_vision: r.get_field_u8(idx, 5),
                requires_spell_focus: r.get_field_u16(idx, 6),
            },
        )
    }
}

impl SpellCategoriesStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "SpellCategories.db2", |id, idx, r| {
            SpellCategoriesEntry {
                id,
                difficulty_id: r.get_field_u8(idx, 0),
                category: r.get_field_i16(idx, 1),
                defense_type: r.get_field_i8(idx, 2),
                dispel_type: r.get_field_i8(idx, 3),
                mechanic: r.get_field_i8(idx, 4),
                prevention_type: r.get_field_i8(idx, 5),
                start_recovery_category: r.get_field_i16(idx, 6),
                charge_category: r.get_field_i16(idx, 7),
                spell_id: r.get_relationship_id(idx).unwrap_or(0),
            }
        })
    }
}

impl SpellCategoryStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "SpellCategory.db2", |id, idx, r| {
            SpellCategoryEntry {
                id,
                name: r.get_field_string(idx, 0),
                flags: r.get_field_i32(idx, 1),
                uses_per_week: r.get_field_u8(idx, 2),
                max_charges: r.get_field_i8(idx, 3),
                charge_recovery_time: r.get_field_i32(idx, 4),
                type_mask: r.get_field_i32(idx, 5),
            }
        })
    }
}

impl SpellClassOptionsStore {
    pub fn entry_for_spell_like_cpp(&self, spell_id: u32) -> Option<&SpellClassOptionsEntry> {
        self.entries
            .values()
            .find(|entry| u32::try_from(entry.spell_id).ok() == Some(spell_id))
    }

    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "SpellClassOptions.db2", |id, idx, r| {
            SpellClassOptionsEntry {
                id,
                spell_id: r.get_field_i32(idx, 0),
                modal_next_spell: r.get_field_u32(idx, 1),
                spell_class_set: r.get_field_u8(idx, 2),
                spell_class_mask: std::array::from_fn(|i| r.get_array_element(idx, 3, i, 32)),
            }
        })
    }
}

impl SpellCooldownsStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "SpellCooldowns.db2", |id, idx, r| {
            SpellCooldownsEntry {
                id,
                difficulty_id: r.get_field_u8(idx, 0),
                category_recovery_time: r.get_field_i32(idx, 1),
                recovery_time: r.get_field_i32(idx, 2),
                start_recovery_time: r.get_field_i32(idx, 3),
                spell_id: r.get_relationship_id(idx).unwrap_or(0),
            }
        })
    }
}

impl SpellDurationStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "SpellDuration.db2", |id, idx, r| {
            SpellDurationEntry {
                id,
                duration: r.get_field_i32(idx, 0),
                duration_per_level: r.get_field_u32(idx, 1),
                max_duration: r.get_field_i32(idx, 2),
            }
        })
    }
}

impl SpellNameStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "SpellName.db2", |id, idx, r| {
            SpellNameEntry {
                id,
                name: r.get_field_string(idx, 0),
            }
        })
    }
}

impl SpellPowerStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "SpellPower.db2", |id, idx, r| {
            SpellPowerEntry {
                id,
                order_index: r.get_field_u8(idx, 0),
                mana_cost: r.get_field_i32(idx, 1),
                mana_cost_per_level: r.get_field_i32(idx, 2),
                mana_per_second: r.get_field_i32(idx, 3),
                power_display_id: r.get_field_u32(idx, 4),
                alt_power_bar_id: r.get_field_i32(idx, 5),
                power_cost_pct: f32_field(r, idx, 6),
                power_cost_max_pct: f32_field(r, idx, 7),
                power_pct_per_second: f32_field(r, idx, 8),
                power_type: r.get_field_i8(idx, 9),
                required_aura_spell_id: r.get_field_i32(idx, 10),
                optional_cost: r.get_field_u32(idx, 11),
                spell_id: r.get_relationship_id(idx).unwrap_or(0),
            }
        })
    }
}

impl SpellPowerDifficultyStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(
            data_dir,
            locale,
            "SpellPowerDifficulty.db2",
            |id, idx, r| SpellPowerDifficultyEntry {
                id,
                difficulty_id: r.get_field_u8(idx, 1),
                order_index: r.get_field_u8(idx, 2),
            },
        )
    }
}

impl SpellMiscStore {
    pub fn entry_for_spell_difficulty_like_cpp(
        &self,
        spell_id: u32,
        difficulty_id: u8,
    ) -> Option<&SpellMiscEntry> {
        self.entries
            .values()
            .find(|entry| entry.spell_id == spell_id && entry.difficulty_id == difficulty_id)
            .or_else(|| {
                self.entries
                    .values()
                    .find(|entry| entry.spell_id == spell_id && entry.difficulty_id == 0)
            })
    }
}

impl SpellProcsPerMinuteStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "SpellProcsPerMinute.db2", |id, idx, r| {
            SpellProcsPerMinuteEntry {
                id,
                base_proc_rate: f32_field(r, idx, 0),
                flags: r.get_field_u8(idx, 1),
            }
        })
    }
}

impl SpellProcsPerMinuteModStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(
            data_dir,
            locale,
            "SpellProcsPerMinuteMod.db2",
            |id, idx, r| SpellProcsPerMinuteModEntry {
                id,
                mod_type: r.get_field_u8(idx, 0),
                param: r.get_field_i16(idx, 1),
                coeff: f32_field(r, idx, 2),
                spell_procs_per_minute_id: r.get_relationship_id(idx).unwrap_or(0),
            },
        )
    }
}

impl SpellRadiusStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "SpellRadius.db2", |id, idx, r| {
            SpellRadiusEntry {
                id,
                radius: f32_field(r, idx, 0),
                radius_per_level: f32_field(r, idx, 1),
                radius_min: f32_field(r, idx, 2),
                radius_max: f32_field(r, idx, 3),
            }
        })
    }
}

impl SpellRangeStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "SpellRange.db2", |id, idx, r| {
            SpellRangeEntry {
                id,
                display_name: r.get_field_string(idx, 0),
                display_name_short: r.get_field_string(idx, 1),
                flags: r.get_field_u8(idx, 2),
                range_min: f32_array::<2>(r, idx, 3),
                range_max: f32_array::<2>(r, idx, 4),
            }
        })
    }
}

impl SpellReagentsStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "SpellReagents.db2", |id, idx, r| {
            SpellReagentsEntry {
                id,
                spell_id: r.get_field_i32(idx, 0),
                reagent: std::array::from_fn(|i| r.get_array_element(idx, 1, i, 32) as i32),
                reagent_count: std::array::from_fn(|i| r.get_array_element(idx, 2, i, 16) as i16),
            }
        })
    }
}

impl SpellReagentsCurrencyStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(
            data_dir,
            locale,
            "SpellReagentsCurrency.db2",
            |id, idx, r| SpellReagentsCurrencyEntry {
                id,
                spell_id: r.get_relationship_id(idx).unwrap_or(0),
                currency_types_id: r.get_field_u16(idx, 1),
                currency_count: r.get_field_u16(idx, 2),
            },
        )
    }
}

impl SpellEffectDb2Store {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "SpellEffect.db2", |id, idx, r| {
            SpellEffectDb2Entry {
                id,
                difficulty_id: r.get_field_i32(idx, 0),
                effect_index: r.get_field_i32(idx, 1),
                effect: r.get_field_u32(idx, 2),
                effect_amplitude: f32_field(r, idx, 3),
                effect_attributes: r.get_field_i32(idx, 4),
                effect_aura: r.get_field_i16(idx, 5),
                effect_aura_period: r.get_field_i32(idx, 6),
                effect_base_points: r.get_field_i32(idx, 7),
                effect_bonus_coefficient: f32_field(r, idx, 8),
                effect_chain_amplitude: f32_field(r, idx, 9),
                effect_chain_targets: r.get_field_i32(idx, 10),
                effect_die_sides: r.get_field_i32(idx, 11),
                effect_item_type: r.get_field_i32(idx, 12),
                effect_mechanic: r.get_field_i32(idx, 13),
                effect_points_per_resource: f32_field(r, idx, 14),
                effect_pos_facing: f32_field(r, idx, 15),
                effect_real_points_per_level: f32_field(r, idx, 16),
                effect_trigger_spell: r.get_field_i32(idx, 17),
                bonus_coefficient_from_ap: f32_field(r, idx, 18),
                pvp_multiplier: f32_field(r, idx, 19),
                coefficient: f32_field(r, idx, 20),
                variance: f32_field(r, idx, 21),
                resource_coefficient: f32_field(r, idx, 22),
                group_size_base_points_coefficient: f32_field(r, idx, 23),
                effect_misc_value: std::array::from_fn(|i| {
                    r.get_array_element(idx, 24, i, 32) as i32
                }),
                effect_radius_index: std::array::from_fn(|i| r.get_array_element(idx, 25, i, 32)),
                effect_spell_class_mask: std::array::from_fn(|i| {
                    r.get_array_element(idx, 26, i, 32)
                }),
                implicit_target: std::array::from_fn(|i| {
                    r.get_array_element(idx, 27, i, 16) as i16
                }),
                spell_id: r.get_relationship_id(idx).unwrap_or(0),
            }
        })
    }
}

impl SpellEquippedItemsStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "SpellEquippedItems.db2", |id, idx, r| {
            SpellEquippedItemsEntry {
                id,
                spell_id: r.get_field_i32(idx, 0),
                equipped_item_class: r.get_field_i8(idx, 1),
                equipped_item_inv_types: r.get_field_i32(idx, 2),
                equipped_item_subclass: r.get_field_i32(idx, 3),
            }
        })
    }
}

impl SpellFocusObjectStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "SpellFocusObject.db2", |id, idx, r| {
            SpellFocusObjectEntry {
                id,
                name: r.get_field_string(idx, 0),
            }
        })
    }
}

impl SpellInterruptsStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "SpellInterrupts.db2", |id, idx, r| {
            SpellInterruptsEntry {
                id,
                difficulty_id: r.get_field_u8(idx, 0),
                interrupt_flags: r.get_field_i16(idx, 1),
                aura_interrupt_flags: std::array::from_fn(|i| {
                    r.get_array_element(idx, 2, i, 32) as i32
                }),
                channel_interrupt_flags: std::array::from_fn(|i| {
                    r.get_array_element(idx, 3, i, 32) as i32
                }),
                spell_id: r.get_relationship_id(idx).unwrap_or(0),
            }
        })
    }
}

impl SpellItemEnchantmentConditionStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(
            data_dir,
            locale,
            "SpellItemEnchantmentCondition.db2",
            |id, idx, r| SpellItemEnchantmentConditionEntry {
                id,
                lt_operand_type: std::array::from_fn(|i| r.get_array_element(idx, 0, i, 8) as u8),
                lt_operand: std::array::from_fn(|i| r.get_array_element(idx, 1, i, 32)),
                operator: std::array::from_fn(|i| r.get_array_element(idx, 2, i, 8) as u8),
                rt_operand_type: std::array::from_fn(|i| r.get_array_element(idx, 3, i, 8) as u8),
                rt_operand: std::array::from_fn(|i| r.get_array_element(idx, 4, i, 8) as u8),
                logic: std::array::from_fn(|i| r.get_array_element(idx, 5, i, 8) as u8),
            },
        )
    }
}

impl SpellKeyboundOverrideStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(
            data_dir,
            locale,
            "SpellKeyboundOverride.db2",
            |id, idx, r| SpellKeyboundOverrideEntry {
                id,
                function: r.get_field_string(idx, 0),
                override_type: r.get_field_i8(idx, 1),
                data: r.get_field_i32(idx, 2),
                flags: r.get_field_i32(idx, 3),
            },
        )
    }
}

impl SpellLabelStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "SpellLabel.db2", |id, idx, r| {
            SpellLabelEntry {
                id,
                label_id: r.get_field_u32(idx, 0),
                spell_id: r.get_relationship_id(idx).unwrap_or(0),
            }
        })
    }
}

impl SpellLearnSpellStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "SpellLearnSpell.db2", |id, idx, r| {
            SpellLearnSpellEntry {
                id,
                spell_id: r.get_field_i32(idx, 0),
                learn_spell_id: r.get_field_i32(idx, 1),
                overrides_spell_id: r.get_field_i32(idx, 2),
            }
        })
    }
}

impl SpellLevelsStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "SpellLevels.db2", |id, idx, r| {
            SpellLevelsEntry {
                id,
                difficulty_id: r.get_field_u8(idx, 0),
                base_level: r.get_field_i16(idx, 1),
                max_level: r.get_field_i16(idx, 2),
                spell_level: r.get_field_i16(idx, 3),
                max_passive_aura_level: r.get_field_u8(idx, 4),
                spell_id: r.get_relationship_id(idx).unwrap_or(0),
            }
        })
    }

    pub fn entry_for_spell_difficulty_like_cpp(
        &self,
        spell_id: u32,
        difficulty_id: u8,
    ) -> Option<&SpellLevelsEntry> {
        self.entries
            .values()
            .find(|entry| entry.spell_id == spell_id && entry.difficulty_id == difficulty_id)
    }

    pub fn entries_like_cpp(&self) -> impl Iterator<Item = &SpellLevelsEntry> {
        self.entries.values()
    }
}

impl SpellMiscStore {
    pub fn get_by_spell_id(&self, spell_id: u32) -> Option<&SpellMiscEntry> {
        self.entries
            .values()
            .find(|entry| entry.spell_id == spell_id)
    }

    /// C++ `SpellInfo::IsAutocastable`.
    pub fn is_autocastable_like_cpp(&self, spell_id: u32) -> bool {
        self.get_by_spell_id(spell_id)
            .is_none_or(SpellMiscEntry::is_autocastable_like_cpp)
    }

    pub fn is_passive_like_cpp(&self, spell_id: u32) -> bool {
        self.get_by_spell_id(spell_id)
            .is_some_and(SpellMiscEntry::is_passive_like_cpp)
    }

    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "SpellMisc.db2", |id, idx, r| {
            SpellMiscEntry {
                id,
                attributes: std::array::from_fn(|i| r.get_array_element(idx, 0, i, 32) as i32),
                difficulty_id: r.get_field_u8(idx, 1),
                casting_time_index: r.get_field_u16(idx, 2),
                duration_index: r.get_field_u16(idx, 3),
                range_index: r.get_field_u16(idx, 4),
                school_mask: r.get_field_u8(idx, 5),
                speed: f32_field(r, idx, 6),
                launch_delay: f32_field(r, idx, 7),
                min_duration: f32_field(r, idx, 8),
                spell_icon_file_data_id: r.get_field_i32(idx, 9),
                active_icon_file_data_id: r.get_field_i32(idx, 10),
                content_tuning_id: r.get_field_i32(idx, 11),
                show_future_spell_player_condition_id: r.get_field_i32(idx, 12),
                spell_id: r.get_relationship_id(idx).unwrap_or(0),
            }
        })
    }
}

impl SpellScalingStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "SpellScaling.db2", |id, idx, r| {
            SpellScalingEntry {
                id,
                spell_id: r.get_field_i32(idx, 0),
                class: r.get_field_i32(idx, 1),
                min_scaling_level: r.get_field_u32(idx, 2),
                max_scaling_level: r.get_field_u32(idx, 3),
                scales_from_item_level: r.get_field_i16(idx, 4),
            }
        })
    }
}

impl SpellShapeshiftStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "SpellShapeshift.db2", |id, idx, r| {
            SpellShapeshiftEntry {
                id,
                spell_id: r.get_field_i32(idx, 0),
                stance_bar_order: r.get_field_i8(idx, 1),
                shapeshift_exclude: std::array::from_fn(|i| {
                    r.get_array_element(idx, 2, i, 32) as i32
                }),
                shapeshift_mask: std::array::from_fn(|i| r.get_array_element(idx, 3, i, 32) as i32),
            }
        })
    }
}

impl SpellShapeshiftFormStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "SpellShapeshiftForm.db2", |id, idx, r| {
            SpellShapeshiftFormEntry {
                id,
                name: r.get_field_string(idx, 0),
                creature_type: r.get_field_i8(idx, 1),
                flags: r.get_field_i32(idx, 2),
                attack_icon_file_id: r.get_field_i32(idx, 3),
                bonus_action_bar: r.get_field_i8(idx, 4),
                combat_round_time: r.get_field_i16(idx, 5),
                damage_variance: f32_field(r, idx, 6),
                mount_type_id: r.get_field_u16(idx, 7),
                creature_display_id: std::array::from_fn(|i| r.get_array_element(idx, 8, i, 32)),
                preset_spell_id: std::array::from_fn(|i| r.get_array_element(idx, 9, i, 32)),
            }
        })
    }
}

impl SpellTargetRestrictionsStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(
            data_dir,
            locale,
            "SpellTargetRestrictions.db2",
            |id, idx, r| SpellTargetRestrictionsEntry {
                id,
                difficulty_id: r.get_field_u8(idx, 0),
                cone_degrees: f32_field(r, idx, 1),
                max_targets: r.get_field_u8(idx, 2),
                max_target_level: r.get_field_u32(idx, 3),
                target_creature_type: r.get_field_i16(idx, 4),
                targets: r.get_field_i32(idx, 5),
                width: f32_field(r, idx, 6),
                spell_id: r.get_relationship_id(idx).unwrap_or(0),
            },
        )
    }
}

impl SpellTotemsStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "SpellTotems.db2", |id, idx, r| {
            SpellTotemsEntry {
                id,
                spell_id: r.get_field_i32(idx, 0),
                required_totem_category_id: std::array::from_fn(|i| r.get_array_u16(idx, 1, i)),
                totem: std::array::from_fn(|i| r.get_array_element(idx, 2, i, 32) as i32),
            }
        })
    }
}

impl SpellVisualStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "SpellVisual.db2", |id, idx, r| {
            SpellVisualEntry {
                id,
                missile_cast_offset: f32_array::<3>(r, idx, 0),
                missile_impact_offset: f32_array::<3>(r, idx, 1),
                anim_event_sound_id: r.get_field_u32(idx, 2),
                flags: r.get_field_i32(idx, 3),
                missile_attachment: r.get_field_i8(idx, 4),
                missile_destination_attachment: r.get_field_i8(idx, 5),
                missile_cast_positioner_id: r.get_field_u32(idx, 6),
                missile_impact_positioner_id: r.get_field_u32(idx, 7),
                missile_targeting_kit: r.get_field_i32(idx, 8),
                hostile_spell_visual_id: r.get_field_u32(idx, 9),
                caster_spell_visual_id: r.get_field_u32(idx, 10),
                spell_visual_missile_set_id: r.get_field_u16(idx, 11),
                damage_number_delay: r.get_field_u16(idx, 12),
                low_violence_spell_visual_id: r.get_field_u32(idx, 13),
                raid_spell_visual_missile_set_id: r.get_field_u32(idx, 14),
                reduced_unexpected_camera_movement_spell_visual_id: r.get_field_i32(idx, 15),
                area_model: r.get_field_u16(idx, 16),
                has_missile: r.get_field_i8(idx, 17),
            }
        })
    }
}

impl SpellVisualEffectNameStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(
            data_dir,
            locale,
            "SpellVisualEffectName.db2",
            |id, idx, r| SpellVisualEffectNameEntry {
                id,
                model_file_data_id: r.get_field_i32(idx, 0),
                base_missile_speed: f32_field(r, idx, 1),
                scale: f32_field(r, idx, 2),
                min_allowed_scale: f32_field(r, idx, 3),
                max_allowed_scale: f32_field(r, idx, 4),
                alpha: f32_field(r, idx, 5),
                flags: r.get_field_u32(idx, 6),
                texture_file_data_id: r.get_field_i32(idx, 7),
                effect_radius: f32_field(r, idx, 8),
                effect_type: r.get_field_u32(idx, 9),
                generic_id: r.get_field_i32(idx, 10),
                ribbon_quality_id: r.get_field_u32(idx, 11),
                dissolve_effect_id: r.get_field_i32(idx, 12),
                model_position: r.get_field_i32(idx, 13),
            },
        )
    }
}

impl SpellVisualKitStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "SpellVisualKit.db2", |id, idx, r| {
            SpellVisualKitEntry {
                id,
                fallback_spell_visual_kit_id: r.get_field_u32(idx, 0),
                delay_min: r.get_field_u16(idx, 1),
                delay_max: r.get_field_u16(idx, 2),
                fallback_priority: f32_field(r, idx, 3),
                flags: std::array::from_fn(|i| r.get_array_element(idx, 4, i, 32) as i32),
            }
        })
    }
}

impl SpellVisualMissileStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "SpellVisualMissile.db2", |id, idx, r| {
            SpellVisualMissileEntry {
                id,
                cast_offset: f32_array::<3>(r, idx, 0),
                impact_offset: f32_array::<3>(r, idx, 1),
                spell_visual_effect_name_id: r.get_field_u16(idx, 2),
                sound_entries_id: r.get_field_u32(idx, 3),
                attachment: r.get_field_i8(idx, 4),
                destination_attachment: r.get_field_i8(idx, 5),
                cast_positioner_id: r.get_field_u16(idx, 6),
                impact_positioner_id: r.get_field_u16(idx, 7),
                follow_ground_height: r.get_field_i32(idx, 8),
                follow_ground_drop_speed: r.get_field_u32(idx, 9),
                follow_ground_approach: r.get_field_u16(idx, 10),
                flags: r.get_field_u32(idx, 11),
                spell_missile_motion_id: r.get_field_u16(idx, 12),
                anim_kit_id: r.get_field_u32(idx, 13),
                spell_visual_missile_set_id: r.get_field_u32(idx, 14),
            }
        })
    }
}

impl SpellXSpellVisualStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "SpellXSpellVisual.db2", |id, idx, r| {
            SpellXSpellVisualEntry {
                id,
                difficulty_id: r.get_field_u8(idx, 0),
                spell_visual_id: r.get_field_u32(idx, 1),
                probability: f32_field(r, idx, 2),
                flags: r.get_field_u8(idx, 3),
                priority: r.get_field_i32(idx, 4),
                spell_icon_file_id: r.get_field_i32(idx, 5),
                active_icon_file_id: r.get_field_i32(idx, 6),
                viewer_unit_condition_id: r.get_field_u16(idx, 7),
                viewer_player_condition_id: r.get_field_u32(idx, 8),
                caster_unit_condition_id: r.get_field_u16(idx, 9),
                caster_player_condition_id: r.get_field_u32(idx, 10),
                spell_id: r.get_relationship_id(idx).unwrap_or(0),
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

fn f32_field(reader: &Wdc4Reader, record_idx: usize, field: usize) -> f32 {
    f32::from_bits(reader.get_field_u32(record_idx, field))
}

fn f32_array<const N: usize>(reader: &Wdc4Reader, record_idx: usize, field: usize) -> [f32; N] {
    std::array::from_fn(|i| f32::from_bits(reader.get_array_element(record_idx, field, i, 32)))
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

impl_from_entries!(SpellAuraOptionsStore, SpellAuraOptionsEntry);
impl_from_entries!(SpellAuraRestrictionsStore, SpellAuraRestrictionsEntry);
impl_from_entries!(SpellCastTimesStore, SpellCastTimesEntry);
impl_from_entries!(SpellCastingRequirementsStore, SpellCastingRequirementsEntry);
impl_from_entries!(SpellCategoriesStore, SpellCategoriesEntry);
impl_from_entries!(SpellCategoryStore, SpellCategoryEntry);
impl_from_entries!(SpellClassOptionsStore, SpellClassOptionsEntry);
impl_from_entries!(SpellCooldownsStore, SpellCooldownsEntry);
impl_from_entries!(SpellDurationStore, SpellDurationEntry);
impl_from_entries!(SpellEffectDb2Store, SpellEffectDb2Entry);
impl_from_entries!(SpellEquippedItemsStore, SpellEquippedItemsEntry);
impl_from_entries!(SpellFocusObjectStore, SpellFocusObjectEntry);
impl_from_entries!(SpellInterruptsStore, SpellInterruptsEntry);
impl_from_entries!(
    SpellItemEnchantmentConditionStore,
    SpellItemEnchantmentConditionEntry
);
impl_from_entries!(SpellKeyboundOverrideStore, SpellKeyboundOverrideEntry);
impl_from_entries!(SpellLabelStore, SpellLabelEntry);
impl_from_entries!(SpellLearnSpellStore, SpellLearnSpellEntry);
impl_from_entries!(SpellLevelsStore, SpellLevelsEntry);
impl_from_entries!(SpellMiscStore, SpellMiscEntry);
impl_from_entries!(SpellNameStore, SpellNameEntry);
impl_from_entries!(SpellPowerStore, SpellPowerEntry);
impl_from_entries!(SpellPowerDifficultyStore, SpellPowerDifficultyEntry);
impl_from_entries!(SpellProcsPerMinuteStore, SpellProcsPerMinuteEntry);
impl_from_entries!(SpellProcsPerMinuteModStore, SpellProcsPerMinuteModEntry);
impl_from_entries!(SpellRadiusStore, SpellRadiusEntry);
impl_from_entries!(SpellRangeStore, SpellRangeEntry);
impl_from_entries!(SpellReagentsStore, SpellReagentsEntry);
impl_from_entries!(SpellReagentsCurrencyStore, SpellReagentsCurrencyEntry);
impl_from_entries!(SpellScalingStore, SpellScalingEntry);
impl_from_entries!(SpellShapeshiftStore, SpellShapeshiftEntry);
impl_from_entries!(SpellShapeshiftFormStore, SpellShapeshiftFormEntry);
impl_from_entries!(SpellTargetRestrictionsStore, SpellTargetRestrictionsEntry);
impl_from_entries!(SpellTotemsStore, SpellTotemsEntry);
impl_from_entries!(SpellVisualStore, SpellVisualEntry);
impl_from_entries!(SpellVisualEffectNameStore, SpellVisualEffectNameEntry);
impl_from_entries!(SpellVisualKitStore, SpellVisualKitEntry);
impl_from_entries!(SpellVisualMissileStore, SpellVisualMissileEntry);
impl_from_entries!(SpellXSpellVisualStore, SpellXSpellVisualEntry);

/// C++ `SpellInfo::CalcDuration` boundary represented from DB2 duration rows.
///
/// Anchor: `SpellInfo.cpp:3894-3910`; player spell mods and passive `-1` fallback
/// are intentionally outside this helper because their runtime metadata is not
/// represented here. Missing entry/index returns `0`; `Duration == -1` remains
/// `-1`; otherwise Rust mirrors C++ `abs(Duration)`.
pub fn spell_duration_ms_like_cpp(
    duration_index: u32,
    duration_store: Option<&SpellDurationStore>,
) -> i32 {
    if duration_index == 0 {
        return 0;
    }
    let Some(entry) = duration_store.and_then(|store| store.get(duration_index)) else {
        return 0;
    };
    if entry.duration == -1 {
        -1
    } else {
        entry.duration.saturating_abs()
    }
}

/// C++ `SpellEffectInfo::CalcRadius` boundary represented from DB2 radius rows.
///
/// Anchor: `SpellInfo.cpp:653-692`; this models the no-caster overload used by
/// `Spell::EffectAddFarsight`: missing entry/index returns `0.0`; radius min is
/// used unless it is zero, in which case radius max is used. Random radius and
/// caster radius mods are intentionally outside this represented slice.
pub fn spell_effect_radius_like_cpp(
    radius_index: u32,
    radius_store: Option<&SpellRadiusStore>,
) -> f32 {
    if radius_index == 0 {
        return 0.0;
    }
    let Some(entry) = radius_store.and_then(|store| store.get(radius_index)) else {
        return 0.0;
    };
    if entry.radius_min == 0.0 {
        entry.radius_max
    } else {
        entry.radius_min
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spell_categories_uses_cpp_parent_relationship() {
        let store = SpellCategoriesStore::from_entries([SpellCategoriesEntry {
            id: 1,
            difficulty_id: 2,
            category: 3,
            defense_type: 4,
            dispel_type: 5,
            mechanic: 6,
            prevention_type: 7,
            start_recovery_category: 8,
            charge_category: 9,
            spell_id: 10,
        }]);

        assert_eq!(store.get(1).unwrap().spell_id, 10);
    }

    #[test]
    fn duration_and_radius_helpers_match_cpp_fallbacks() {
        let duration_store = SpellDurationStore::from_entries([SpellDurationEntry {
            id: 7,
            duration: -5000,
            duration_per_level: 0,
            max_duration: 0,
        }]);
        assert_eq!(spell_duration_ms_like_cpp(0, Some(&duration_store)), 0);
        assert_eq!(spell_duration_ms_like_cpp(99, Some(&duration_store)), 0);
        assert_eq!(spell_duration_ms_like_cpp(7, Some(&duration_store)), 5000);

        let infinite_duration_store = SpellDurationStore::from_entries([SpellDurationEntry {
            id: 8,
            duration: -1,
            duration_per_level: 0,
            max_duration: 0,
        }]);
        assert_eq!(
            spell_duration_ms_like_cpp(8, Some(&infinite_duration_store)),
            -1
        );

        let radius_store = SpellRadiusStore::from_entries([
            SpellRadiusEntry {
                id: 11,
                radius: 0.0,
                radius_per_level: 0.0,
                radius_min: 0.0,
                radius_max: 25.0,
            },
            SpellRadiusEntry {
                id: 12,
                radius: 0.0,
                radius_per_level: 0.0,
                radius_min: 10.0,
                radius_max: 25.0,
            },
        ]);
        assert_eq!(spell_effect_radius_like_cpp(0, Some(&radius_store)), 0.0);
        assert_eq!(spell_effect_radius_like_cpp(99, Some(&radius_store)), 0.0);
        assert_eq!(spell_effect_radius_like_cpp(11, Some(&radius_store)), 25.0);
        assert_eq!(spell_effect_radius_like_cpp(12, Some(&radius_store)), 10.0);
    }

    #[test]
    fn load_spell_core_db2_subbatch_when_fixtures_exist() {
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

        load_if_exists!("SpellAuraOptions.db2", SpellAuraOptionsStore);
        load_if_exists!("SpellAuraRestrictions.db2", SpellAuraRestrictionsStore);
        load_if_exists!("SpellCastTimes.db2", SpellCastTimesStore);
        load_if_exists!(
            "SpellCastingRequirements.db2",
            SpellCastingRequirementsStore
        );
        load_if_exists!("SpellCategories.db2", SpellCategoriesStore);
        load_if_exists!("SpellCategory.db2", SpellCategoryStore);
        load_if_exists!("SpellClassOptions.db2", SpellClassOptionsStore);
        load_if_exists!("SpellCooldowns.db2", SpellCooldownsStore);
        load_if_exists!("SpellDuration.db2", SpellDurationStore);
        load_if_exists!("SpellEffect.db2", SpellEffectDb2Store);
        load_if_exists!("SpellEquippedItems.db2", SpellEquippedItemsStore);
        load_if_exists!("SpellFocusObject.db2", SpellFocusObjectStore);
        load_if_exists!("SpellInterrupts.db2", SpellInterruptsStore);
        load_if_exists!(
            "SpellItemEnchantmentCondition.db2",
            SpellItemEnchantmentConditionStore
        );
        load_if_exists!("SpellKeyboundOverride.db2", SpellKeyboundOverrideStore);
        load_if_exists!("SpellLabel.db2", SpellLabelStore);
        load_if_exists!("SpellLearnSpell.db2", SpellLearnSpellStore);
        load_if_exists!("SpellLevels.db2", SpellLevelsStore);
        load_if_exists!("SpellMisc.db2", SpellMiscStore);
        load_if_exists!("SpellName.db2", SpellNameStore);
        load_if_exists!("SpellPower.db2", SpellPowerStore);
        load_if_exists!("SpellPowerDifficulty.db2", SpellPowerDifficultyStore);
        load_if_exists!("SpellProcsPerMinute.db2", SpellProcsPerMinuteStore);
        load_if_exists!("SpellProcsPerMinuteMod.db2", SpellProcsPerMinuteModStore);
        load_if_exists!("SpellRadius.db2", SpellRadiusStore);
        load_if_exists!("SpellRange.db2", SpellRangeStore);
        load_if_exists!("SpellReagents.db2", SpellReagentsStore);
        load_if_exists!("SpellReagentsCurrency.db2", SpellReagentsCurrencyStore);
        load_if_exists!("SpellScaling.db2", SpellScalingStore);
        load_if_exists!("SpellShapeshift.db2", SpellShapeshiftStore);
        load_if_exists!("SpellShapeshiftForm.db2", SpellShapeshiftFormStore);
        load_if_exists!("SpellTargetRestrictions.db2", SpellTargetRestrictionsStore);
        load_if_exists!("SpellTotems.db2", SpellTotemsStore);
        load_if_exists!("SpellVisual.db2", SpellVisualStore);
        load_if_exists!("SpellVisualEffectName.db2", SpellVisualEffectNameStore);
        load_if_exists!("SpellVisualKit.db2", SpellVisualKitStore);
        load_if_exists!("SpellVisualMissile.db2", SpellVisualMissileStore);
        load_if_exists!("SpellXSpellVisual.db2", SpellXSpellVisualStore);
    }
}
