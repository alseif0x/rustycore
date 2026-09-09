//! DB2 spell entry stores state definitions, part 1 of 3.
//!
//! Separated from the spell_db2.rs root under #646. Behaviour is preserved.

use super::*;

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

#[derive(Debug, Clone, Default, PartialEq, Eq)]
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

#[derive(Debug, Clone, Default, PartialEq, Eq)]
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

#[derive(Debug, Clone, Default, PartialEq)]
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

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SpellNameEffectiveLoadReportLikeCpp {
    pub overlay_rows: usize,
    pub removed_rows: usize,
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

impl SpellTargetRestrictionsEntry {
    /// C++ promotes the signed DB2 `int16` field to `uint32` when evaluating
    /// `SpellInfo::CheckTargetCreatureType`. Preserve that sign extension.
    pub fn target_creature_type_mask_like_cpp(&self) -> u32 {
        i32::from(self.target_creature_type) as u32
    }
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

pub(crate) trait Db2StoreTableHashLikeCpp {
    fn table_hash_like_cpp(&self) -> Option<u32>;
}

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

    /// Load the effective C++ `sSpellAuraRestrictionsStore` authority: file
    /// rows, official SQL replacements, custom SQL replacements, then final
    /// `hotfix_data` tombstones.
    pub fn load_effective_from_hotfix_rows_like_cpp(
        data_dir: &str,
        locale: &str,
        rows: impl IntoIterator<Item = SpellAuraRestrictionsEntry>,
        removals: &crate::Db2HotfixRemovalStoreLikeCpp,
    ) -> Result<Self> {
        let mut store = Self::load(data_dir, locale)?;
        for entry in rows {
            store.overlay_effective_row_like_cpp(entry);
        }

        let table_hash = store
            .table_hash_like_cpp()
            .context("SpellAuraRestrictions.db2 store is missing its WDC4 table hash")?;
        store.apply_hotfix_removals_with_table_hash_like_cpp(table_hash, removals);
        Ok(store)
    }

    pub(super) fn overlay_effective_row_like_cpp(&mut self, entry: SpellAuraRestrictionsEntry) {
        self.entries.insert(entry.id, entry);
    }

    pub(super) fn apply_hotfix_removals_with_table_hash_like_cpp(
        &mut self,
        table_hash: u32,
        removals: &crate::Db2HotfixRemovalStoreLikeCpp,
    ) {
        self.entries
            .retain(|record_id, _| !removals.contains_like_cpp(table_hash, *record_id as i32));
    }

    pub fn entries_for_spell_id_like_cpp(
        &self,
        spell_id: u32,
    ) -> impl Iterator<Item = &SpellAuraRestrictionsEntry> {
        self.entries
            .values()
            .filter(move |entry| entry.spell_id == spell_id)
    }

    /// Resolve the single `SpellInfo` row through C++'s requested difficulty
    /// followed by its `FallbackDifficultyID` chain.
    pub fn resolved_for_difficulty_chain_like_cpp(
        &self,
        spell_id: u32,
        difficulty_chain: impl IntoIterator<Item = u32>,
    ) -> Option<&SpellAuraRestrictionsEntry> {
        difficulty_chain.into_iter().find_map(|difficulty_id| {
            let difficulty_id = u8::try_from(difficulty_id).ok()?;
            self.entries_for_spell_id_like_cpp(spell_id)
                .filter(|entry| entry.difficulty_id == difficulty_id)
                // C++ iterates DB2 storage in record-ID order and later rows
                // replace the same SpellInfo difficulty slot.
                .max_by_key(|entry| entry.id)
        })
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

    /// Load C++'s effective cast-time authority: DB2, official SQL, custom SQL,
    /// then final `hotfix_data` tombstones.
    pub fn load_effective_from_hotfix_rows_like_cpp(
        data_dir: &str,
        locale: &str,
        rows: impl IntoIterator<Item = SpellCastTimesEntry>,
        removals: &crate::Db2HotfixRemovalStoreLikeCpp,
    ) -> Result<Self> {
        let mut store = Self::load(data_dir, locale)?;
        for entry in rows {
            store.overlay_effective_row_like_cpp(entry);
        }
        store.apply_final_hotfix_removals_like_cpp(removals)?;
        Ok(store)
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

    /// Compose C++'s final `SpellCastingRequirements` authority: DB2 file,
    /// official/custom SQL replacements, then final hotfix tombstones.
    pub fn load_effective_from_hotfix_rows_like_cpp(
        data_dir: &str,
        locale: &str,
        rows: impl IntoIterator<Item = SpellCastingRequirementsEntry>,
        removals: &crate::Db2HotfixRemovalStoreLikeCpp,
    ) -> Result<Self> {
        let mut store = Self::load(data_dir, locale)?;
        for entry in rows {
            store.entries.insert(entry.id, entry);
        }

        let table_hash = store
            .table_hash_like_cpp()
            .context("SpellCastingRequirements.db2 store is missing its WDC4 table hash")?;
        store
            .entries
            .retain(|record_id, _| !removals.contains_like_cpp(table_hash, *record_id as i32));
        Ok(store)
    }

    pub fn entry_for_spell_id_like_cpp(
        &self,
        spell_id: i32,
    ) -> Option<&SpellCastingRequirementsEntry> {
        self.entries
            .values()
            .filter(|entry| entry.spell_id == spell_id)
            // C++'s DB2 iteration assigns this DIFFICULTY_NONE slot in
            // record-ID order, so a malformed duplicate resolves to the
            // highest record ID deterministically.
            .max_by_key(|entry| entry.id)
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

    /// Load the effective C++ `sSpellCategoriesStore` authority: DB2 file,
    /// official SQL replacements, custom SQL replacements, then final
    /// `hotfix_data` tombstones.
    pub fn load_effective_from_hotfix_rows_like_cpp(
        data_dir: &str,
        locale: &str,
        rows: impl IntoIterator<Item = SpellCategoriesEntry>,
        removals: &crate::Db2HotfixRemovalStoreLikeCpp,
    ) -> Result<Self> {
        let mut store = Self::load(data_dir, locale)?;
        for entry in rows {
            store.overlay_effective_row_like_cpp(entry);
        }

        let table_hash = store
            .table_hash_like_cpp()
            .context("SpellCategories.db2 store is missing its WDC4 table hash")?;
        store.apply_hotfix_removals_with_table_hash_like_cpp(table_hash, removals);
        Ok(store)
    }

    pub(super) fn overlay_effective_row_like_cpp(&mut self, entry: SpellCategoriesEntry) {
        self.entries.insert(entry.id, entry);
    }

    pub(super) fn apply_hotfix_removals_with_table_hash_like_cpp(
        &mut self,
        table_hash: u32,
        removals: &crate::Db2HotfixRemovalStoreLikeCpp,
    ) {
        self.entries
            .retain(|record_id, _| !removals.contains_like_cpp(table_hash, *record_id as i32));
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

    /// Load C++'s effective category authority: DB2, official SQL, custom SQL,
    /// then final `hotfix_data` tombstones. `SpellCategory.Flags` carries
    /// `SPELL_CATEGORY_FLAG_COOLDOWN_STARTS_ON_EVENT`, so a hotfixed row decides
    /// whether a cooldown may start at publication time.
    pub fn load_effective_from_hotfix_rows_like_cpp(
        data_dir: &str,
        locale: &str,
        rows: impl IntoIterator<Item = SpellCategoryEntry>,
        removals: &crate::Db2HotfixRemovalStoreLikeCpp,
    ) -> Result<Self> {
        let mut store = Self::load(data_dir, locale)?;
        for entry in rows {
            store.overlay_effective_row_like_cpp(entry);
        }
        store.apply_final_hotfix_removals_like_cpp(removals)?;
        Ok(store)
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

    /// Load C++'s effective cooldown authority: DB2, official SQL, custom
    /// SQL, then final `hotfix_data` tombstones.
    pub fn load_effective_from_hotfix_rows_like_cpp(
        data_dir: &str,
        locale: &str,
        rows: impl IntoIterator<Item = SpellCooldownsEntry>,
        removals: &crate::Db2HotfixRemovalStoreLikeCpp,
    ) -> Result<Self> {
        let mut store = Self::load(data_dir, locale)?;
        for entry in rows {
            store.overlay_effective_row_like_cpp(entry);
        }

        let table_hash = store
            .table_hash_like_cpp()
            .context("SpellCooldowns.db2 store is missing its WDC4 table hash")?;
        store.apply_hotfix_removals_with_table_hash_like_cpp(table_hash, removals);
        Ok(store)
    }

    pub(super) fn overlay_effective_row_like_cpp(&mut self, entry: SpellCooldownsEntry) {
        self.entries.insert(entry.id, entry);
    }

    pub(super) fn apply_hotfix_removals_with_table_hash_like_cpp(
        &mut self,
        table_hash: u32,
        removals: &crate::Db2HotfixRemovalStoreLikeCpp,
    ) {
        self.entries
            .retain(|record_id, _| !removals.contains_like_cpp(table_hash, *record_id as i32));
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

    /// Load C++'s effective duration authority: DB2, official SQL, custom SQL,
    /// then final `hotfix_data` tombstones.
    pub fn load_effective_from_hotfix_rows_like_cpp(
        data_dir: &str,
        locale: &str,
        rows: impl IntoIterator<Item = SpellDurationEntry>,
        removals: &crate::Db2HotfixRemovalStoreLikeCpp,
    ) -> Result<Self> {
        let mut store = Self::load(data_dir, locale)?;
        for entry in rows {
            store.overlay_effective_row_like_cpp(entry);
        }
        store.apply_final_hotfix_removals_like_cpp(removals)?;
        Ok(store)
    }
}
