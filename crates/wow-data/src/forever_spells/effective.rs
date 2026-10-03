//! Full raw store composition, not SpellInfo/difficulty assembly or Player state.
//! 02245dcd DB2Store::LoadFromDB/LoadStringsFromDB, DB2DatabaseLoader::Load
//! and AddString, then DB2Stores::LoadHotfixData's final removal pass.
mod compose;
mod expected_values;
mod patches;
mod value_inputs;
pub use patches::{SummonPropertiesPatch, SummonPropertiesPatchCounts};
#[cfg(test)]
mod tests;
use super::*;
use crate::{Db2HotfixRemovalStoreLikeCpp, wdc4::creation::SpellTable};
use anyhow::{Result, ensure};
use std::collections::BTreeMap;

pub const SPELL_TABLE_HASHES: [u32; 49] = [
    SpellTable::SpellName.table_hash(),
    SpellTable::SpellEffect.table_hash(),
    SpellTable::SpellMisc.table_hash(),
    SpellTable::SpellAuraOptions.table_hash(),
    SpellTable::SpellAuraRestrictions.table_hash(),
    SpellTable::SpellCastingRequirements.table_hash(),
    SpellTable::SpellCategories.table_hash(),
    SpellTable::SpellClassOptions.table_hash(),
    SpellTable::SpellCooldowns.table_hash(),
    SpellTable::SpellEmpower.table_hash(),
    SpellTable::SpellEmpowerStage.table_hash(),
    SpellTable::SpellEquippedItems.table_hash(),
    SpellTable::SpellInterrupts.table_hash(),
    SpellTable::SpellLabel.table_hash(),
    SpellTable::SpellLevels.table_hash(),
    SpellTable::SpellPower.table_hash(),
    SpellTable::SpellPowerDifficulty.table_hash(),
    SpellTable::SpellReagents.table_hash(),
    SpellTable::SpellReagentsCurrency.table_hash(),
    SpellTable::SpellScaling.table_hash(),
    SpellTable::SpellShapeshift.table_hash(),
    SpellTable::SpellTargetRestrictions.table_hash(),
    SpellTable::SpellTotems.table_hash(),
    SpellTable::SpellXSpellVisual.table_hash(),
    SpellTable::Difficulty.table_hash(),
    SpellTable::SpellCastTimes.table_hash(),
    SpellTable::SpellDuration.table_hash(),
    SpellTable::SpellRange.table_hash(),
    SpellTable::SpellRadius.table_hash(),
    SpellTable::SpellProcsPerMinute.table_hash(),
    SpellTable::SpellProcsPerMinuteMod.table_hash(),
    SpellTable::SpellLearnSpell.table_hash(),
    SpellTable::SpellShapeshiftForm.table_hash(),
    SpellTable::SummonProperties.table_hash(),
    SpellTable::BattlePetSpecies.table_hash(),
    SpellTable::SpellCategory.table_hash(),
    SpellTable::Talent.table_hash(),
    SpellTable::SpellItemEnchantment.table_hash(),
    SpellTable::SpellVisual.table_hash(),
    SpellTable::SpellVisualMissile.table_hash(),
    SpellTable::SpellVisualEffectName.table_hash(),
    SpellTable::LiquidType.table_hash(),
    SpellTable::ExpectedStat.table_hash(),
    SpellTable::ExpectedStatMod.table_hash(),
    SpellTable::ContentTuning.table_hash(),
    SpellTable::ContentTuningXExpected.table_hash(),
    SpellTable::RandPropPoints.table_hash(),
    SpellTable::MythicPlusSeason.table_hash(),
    SpellTable::UnitCondition.table_hash(),
];

/// One effective record authority. Startup DTOs/raw vectors are consumed;
/// exclusive startup corrections may mutate before immutable Arc publication.
/// Gameplay and hotfix readers share it and never mutate a mirror.
pub struct SpellCatalog {
    spell_names: BTreeMap<u32, SpellNameRecord>,
    spell_effects: BTreeMap<u32, SpellEffectRecord>,
    spell_misc: BTreeMap<u32, SpellMiscRecord>,
    spell_aura_options: BTreeMap<u32, SpellAuraOptionsRecord>,
    spell_aura_restrictions: BTreeMap<u32, SpellAuraRestrictionsRecord>,
    spell_casting_requirements: BTreeMap<u32, SpellCastingRequirementsRecord>,
    spell_categories: BTreeMap<u32, SpellCategoriesRecord>,
    spell_class_options: BTreeMap<u32, SpellClassOptionsRecord>,
    spell_cooldowns: BTreeMap<u32, SpellCooldownsRecord>,
    spell_empowers: BTreeMap<u32, SpellEmpowerRecord>,
    spell_empower_stages: BTreeMap<u32, SpellEmpowerStageRecord>,
    spell_equipped_items: BTreeMap<u32, SpellEquippedItemsRecord>,
    spell_interrupts: BTreeMap<u32, SpellInterruptsRecord>,
    spell_labels: BTreeMap<u32, SpellLabelRecord>,
    spell_levels: BTreeMap<u32, SpellLevelsRecord>,
    spell_powers: BTreeMap<u32, SpellPowerRecord>,
    spell_power_difficulties: BTreeMap<u32, SpellPowerDifficultyRecord>,
    spell_reagents: BTreeMap<u32, SpellReagentsRecord>,
    spell_reagents_currencies: BTreeMap<u32, SpellReagentsCurrencyRecord>,
    spell_scaling: BTreeMap<u32, SpellScalingRecord>,
    spell_shapeshifts: BTreeMap<u32, SpellShapeshiftRecord>,
    spell_target_restrictions: BTreeMap<u32, SpellTargetRestrictionsRecord>,
    spell_totems: BTreeMap<u32, SpellTotemsRecord>,
    spell_x_spell_visuals: BTreeMap<u32, SpellXSpellVisualRecord>,
    difficulties: BTreeMap<u32, DifficultyRecord>,
    spell_cast_times: BTreeMap<u32, SpellCastTimesRecord>,
    spell_durations: BTreeMap<u32, SpellDurationRecord>,
    spell_ranges: BTreeMap<u32, SpellRangeRecord>,
    spell_radii: BTreeMap<u32, SpellRadiusRecord>,
    spell_procs_per_minute: BTreeMap<u32, SpellProcsPerMinuteRecord>,
    spell_procs_per_minute_mods: BTreeMap<u32, SpellProcsPerMinuteModRecord>,
    spell_learn_spells: BTreeMap<u32, SpellLearnSpellRecord>,
    spell_shapeshift_forms: BTreeMap<u32, SpellShapeshiftFormRecord>,
    summon_properties: BTreeMap<u32, SummonPropertiesRecord>,
    battle_pet_species: BTreeMap<u32, BattlePetSpeciesRecord>,
    spell_category_definitions: BTreeMap<u32, SpellCategoryRecord>,
    talents: BTreeMap<u32, TalentRecord>,
    spell_item_enchantments: BTreeMap<u32, SpellItemEnchantmentRecord>,
    spell_visuals: BTreeMap<u32, SpellVisualRecord>,
    spell_visual_missiles: BTreeMap<u32, SpellVisualMissileRecord>,
    // IDs-only secondary index, built from final records; never a row mirror.
    spell_visual_missiles_by_set: BTreeMap<u32, Vec<u32>>,
    spell_visual_effect_names: BTreeMap<u32, SpellVisualEffectNameRecord>,
    liquid_types: BTreeMap<u32, LiquidTypeRecord>,
    expected_stats: BTreeMap<u32, ExpectedStatRecord>,
    expected_stat_mods: BTreeMap<u32, ExpectedStatModRecord>,
    content_tunings: BTreeMap<u32, ContentTuningRecord>,
    content_tuning_x_expected: BTreeMap<u32, ContentTuningXExpectedRecord>,
    rand_prop_points: BTreeMap<u32, RandPropPointsRecord>,
    // Source index allocation survives final EraseRecord. No record mirror.
    rand_prop_points_storage_last_index: u32,
    mythic_plus_seasons: BTreeMap<u32, MythicPlusSeasonRecord>,
    unit_conditions: BTreeMap<u32, UnitConditionRecord>,
    // Final canonical IDs only; never copied stat/modifier rows.
    expected_stat_by_level: BTreeMap<(u32, i32), u32>,
    expected_mods_by_content_tuning: BTreeMap<u32, Vec<u32>>,
    unknown_baseline_records: [usize; 49],
}

impl SpellRecords {
    /// Main/enUS official -> custom, then selected locale official -> custom,
    /// then final removals. All 49 stores complete before catalog publication.
    pub fn finish(
        self,
        official: Self,
        custom: Self,
        locale: u8,
        official_locale: SpellLocaleRecords,
        custom_locale: SpellLocaleRecords,
        removals: &Db2HotfixRemovalStoreLikeCpp,
    ) -> Result<SpellCatalog> {
        ensure!(locale < 12 && locale != 9, "Invalid target spell locale");
        ensure!(
            official.unknown_baseline_records == [0; 49]
                && custom.unknown_baseline_records == [0; 49]
                && official.rand_prop_points_storage_last_index.is_none()
                && custom.rand_prop_points_storage_last_index.is_none(),
            "SQL spell overlays cannot assert unknown baseline coverage"
        );
        ensure!(
            self.rand_prop_points_sql_index_size.is_none(),
            "File baseline cannot assert a SQL storage bound"
        );
        let file_last_index = self.rand_prop_points_storage_last_index.unwrap_or_else(|| {
            self.rand_prop_points
                .iter()
                .map(|row| row.id)
                .max()
                .unwrap_or(0)
        });
        ensure!(
            self.rand_prop_points
                .iter()
                .all(|row| row.id <= file_last_index),
            "Random-property baseline exceeds its storage bound"
        );
        let (rand_prop_points, rand_prop_points_storage_last_index) = compose::random_points(
            self.rand_prop_points,
            official.rand_prop_points,
            custom.rand_prop_points,
            file_last_index,
            official.rand_prop_points_sql_index_size,
            custom.rand_prop_points_sql_index_size,
        )?;
        let mut result = SpellCatalog {
            rand_prop_points_storage_last_index,
            expected_stat_by_level: BTreeMap::new(),
            expected_mods_by_content_tuning: BTreeMap::new(),
            spell_visual_missiles_by_set: BTreeMap::new(),
            spell_names: compose::localized(
                self.spell_names,
                official.spell_names,
                custom.spell_names,
                |r| r.id,
                compose::spell_name,
            )?,
            spell_effects: compose::numeric(
                self.spell_effects,
                official.spell_effects,
                custom.spell_effects,
                |r| r.id,
            )?,
            spell_misc: compose::numeric(
                self.spell_misc,
                official.spell_misc,
                custom.spell_misc,
                |r| r.id,
            )?,
            spell_aura_options: compose::numeric(
                self.spell_aura_options,
                official.spell_aura_options,
                custom.spell_aura_options,
                |r| r.id,
            )?,
            spell_aura_restrictions: compose::numeric(
                self.spell_aura_restrictions,
                official.spell_aura_restrictions,
                custom.spell_aura_restrictions,
                |r| r.id,
            )?,
            spell_casting_requirements: compose::numeric(
                self.spell_casting_requirements,
                official.spell_casting_requirements,
                custom.spell_casting_requirements,
                |r| r.id,
            )?,
            spell_categories: compose::numeric(
                self.spell_categories,
                official.spell_categories,
                custom.spell_categories,
                |r| r.id,
            )?,
            spell_class_options: compose::numeric(
                self.spell_class_options,
                official.spell_class_options,
                custom.spell_class_options,
                |r| r.id,
            )?,
            spell_cooldowns: compose::numeric(
                self.spell_cooldowns,
                official.spell_cooldowns,
                custom.spell_cooldowns,
                |r| r.id,
            )?,
            spell_empowers: compose::numeric(
                self.spell_empowers,
                official.spell_empowers,
                custom.spell_empowers,
                |r| r.id,
            )?,
            spell_empower_stages: compose::numeric(
                self.spell_empower_stages,
                official.spell_empower_stages,
                custom.spell_empower_stages,
                |r| r.id,
            )?,
            spell_equipped_items: compose::numeric(
                self.spell_equipped_items,
                official.spell_equipped_items,
                custom.spell_equipped_items,
                |r| r.id,
            )?,
            spell_interrupts: compose::numeric(
                self.spell_interrupts,
                official.spell_interrupts,
                custom.spell_interrupts,
                |r| r.id,
            )?,
            spell_labels: compose::numeric(
                self.spell_labels,
                official.spell_labels,
                custom.spell_labels,
                |r| r.id,
            )?,
            spell_levels: compose::numeric(
                self.spell_levels,
                official.spell_levels,
                custom.spell_levels,
                |r| r.id,
            )?,
            spell_powers: compose::numeric(
                self.spell_powers,
                official.spell_powers,
                custom.spell_powers,
                |r| r.id,
            )?,
            spell_power_difficulties: compose::numeric(
                self.spell_power_difficulties,
                official.spell_power_difficulties,
                custom.spell_power_difficulties,
                |r| r.id,
            )?,
            spell_reagents: compose::numeric(
                self.spell_reagents,
                official.spell_reagents,
                custom.spell_reagents,
                |r| r.id,
            )?,
            spell_reagents_currencies: compose::numeric(
                self.spell_reagents_currencies,
                official.spell_reagents_currencies,
                custom.spell_reagents_currencies,
                |r| r.id,
            )?,
            spell_scaling: compose::numeric(
                self.spell_scaling,
                official.spell_scaling,
                custom.spell_scaling,
                |r| r.id,
            )?,
            spell_shapeshifts: compose::numeric(
                self.spell_shapeshifts,
                official.spell_shapeshifts,
                custom.spell_shapeshifts,
                |r| r.id,
            )?,
            spell_target_restrictions: compose::numeric(
                self.spell_target_restrictions,
                official.spell_target_restrictions,
                custom.spell_target_restrictions,
                |r| r.id,
            )?,
            spell_totems: compose::numeric(
                self.spell_totems,
                official.spell_totems,
                custom.spell_totems,
                |r| r.id,
            )?,
            spell_x_spell_visuals: compose::numeric(
                self.spell_x_spell_visuals,
                official.spell_x_spell_visuals,
                custom.spell_x_spell_visuals,
                |r| r.id,
            )?,
            difficulties: compose::localized(
                self.difficulties,
                official.difficulties,
                custom.difficulties,
                |r| r.id,
                compose::difficulty,
            )?,
            spell_cast_times: compose::numeric(
                self.spell_cast_times,
                official.spell_cast_times,
                custom.spell_cast_times,
                |r| r.id,
            )?,
            spell_durations: compose::numeric(
                self.spell_durations,
                official.spell_durations,
                custom.spell_durations,
                |r| r.id,
            )?,
            spell_ranges: compose::localized(
                self.spell_ranges,
                official.spell_ranges,
                custom.spell_ranges,
                |r| r.id,
                compose::spell_range,
            )?,
            spell_radii: compose::numeric(
                self.spell_radii,
                official.spell_radii,
                custom.spell_radii,
                |r| r.id,
            )?,
            spell_procs_per_minute: compose::numeric(
                self.spell_procs_per_minute,
                official.spell_procs_per_minute,
                custom.spell_procs_per_minute,
                |r| r.id,
            )?,
            spell_procs_per_minute_mods: compose::numeric(
                self.spell_procs_per_minute_mods,
                official.spell_procs_per_minute_mods,
                custom.spell_procs_per_minute_mods,
                |r| r.id,
            )?,
            spell_learn_spells: compose::numeric(
                self.spell_learn_spells,
                official.spell_learn_spells,
                custom.spell_learn_spells,
                |r| r.id,
            )?,
            spell_shapeshift_forms: compose::localized(
                self.spell_shapeshift_forms,
                official.spell_shapeshift_forms,
                custom.spell_shapeshift_forms,
                |r| r.id,
                compose::spell_shapeshift_form,
            )?,
            summon_properties: compose::numeric(
                self.summon_properties,
                official.summon_properties,
                custom.summon_properties,
                |r| r.id,
            )?,
            battle_pet_species: compose::localized(
                self.battle_pet_species,
                official.battle_pet_species,
                custom.battle_pet_species,
                |r| r.id,
                compose::battle_pet_species,
            )?,
            spell_category_definitions: compose::localized(
                self.spell_category_definitions,
                official.spell_category_definitions,
                custom.spell_category_definitions,
                |r| r.id,
                compose::spell_category,
            )?,
            talents: compose::localized(
                self.talents,
                official.talents,
                custom.talents,
                |r| r.id,
                compose::talent,
            )?,
            spell_item_enchantments: compose::localized(
                self.spell_item_enchantments,
                official.spell_item_enchantments,
                custom.spell_item_enchantments,
                |r| r.id,
                compose::spell_item_enchantment,
            )?,
            spell_visuals: compose::numeric(
                self.spell_visuals,
                official.spell_visuals,
                custom.spell_visuals,
                |r| r.id,
            )?,
            spell_visual_missiles: compose::numeric(
                self.spell_visual_missiles,
                official.spell_visual_missiles,
                custom.spell_visual_missiles,
                |r| r.id,
            )?,
            spell_visual_effect_names: compose::numeric(
                self.spell_visual_effect_names,
                official.spell_visual_effect_names,
                custom.spell_visual_effect_names,
                |r| r.id,
            )?,
            liquid_types: compose::numeric(
                self.liquid_types,
                official.liquid_types,
                custom.liquid_types,
                |r| r.id,
            )?,
            expected_stats: compose::numeric(
                self.expected_stats,
                official.expected_stats,
                custom.expected_stats,
                |r| r.id,
            )?,
            expected_stat_mods: compose::numeric(
                self.expected_stat_mods,
                official.expected_stat_mods,
                custom.expected_stat_mods,
                |r| r.id,
            )?,
            content_tunings: compose::numeric(
                self.content_tunings,
                official.content_tunings,
                custom.content_tunings,
                |r| r.id,
            )?,
            content_tuning_x_expected: compose::numeric(
                self.content_tuning_x_expected,
                official.content_tuning_x_expected,
                custom.content_tuning_x_expected,
                |r| r.id,
            )?,
            rand_prop_points,
            mythic_plus_seasons: compose::numeric(
                self.mythic_plus_seasons,
                official.mythic_plus_seasons,
                custom.mythic_plus_seasons,
                |r| r.id,
            )?,
            unknown_baseline_records: self.unknown_baseline_records,
            unit_conditions: compose::numeric(
                self.unit_conditions,
                official.unit_conditions,
                custom.unit_conditions,
                |r| r.id,
            )?,
        };
        for batch in [official_locale, custom_locale] {
            for row in batch.talents {
                if let Some(target) = result.talents.get_mut(&row.id) {
                    target.description.overlay_locale(locale, row.description);
                }
            }
            for row in batch.spell_item_enchantments {
                if let Some(target) = result.spell_item_enchantments.get_mut(&row.id) {
                    target.name.overlay_locale(locale, row.name);
                    target.horde_name.overlay_locale(locale, row.horde_name);
                }
            }

            for row in batch.spell_names {
                if let Some(target) = result.spell_names.get_mut(&row.id) {
                    target.name.overlay_locale(locale, row.name);
                }
            }
            for row in batch.difficulties {
                if let Some(target) = result.difficulties.get_mut(&row.id) {
                    target.name.overlay_locale(locale, row.name);
                }
            }
            for row in batch.spell_ranges {
                if let Some(target) = result.spell_ranges.get_mut(&row.id) {
                    target.display_name.overlay_locale(locale, row.display_name);
                    target
                        .display_name_short
                        .overlay_locale(locale, row.display_name_short);
                }
            }
            for row in batch.spell_shapeshift_forms {
                if let Some(target) = result.spell_shapeshift_forms.get_mut(&row.id) {
                    target.name.overlay_locale(locale, row.name);
                }
            }
            for row in batch.battle_pet_species {
                if let Some(target) = result.battle_pet_species.get_mut(&row.id) {
                    target.description.overlay_locale(locale, row.description);
                    target.source_text.overlay_locale(locale, row.source_text);
                }
            }
            for row in batch.spell_category_definitions {
                if let Some(target) = result.spell_category_definitions.get_mut(&row.id) {
                    target.name.overlay_locale(locale, row.name);
                }
            }
        }
        result
            .spell_names
            .retain(|&id, _| !removals.contains_like_cpp(SPELL_TABLE_HASHES[0], id as i32));
        result
            .spell_effects
            .retain(|&id, _| !removals.contains_like_cpp(SPELL_TABLE_HASHES[1], id as i32));
        result
            .spell_misc
            .retain(|&id, _| !removals.contains_like_cpp(SPELL_TABLE_HASHES[2], id as i32));
        result
            .spell_aura_options
            .retain(|&id, _| !removals.contains_like_cpp(SPELL_TABLE_HASHES[3], id as i32));
        result
            .spell_aura_restrictions
            .retain(|&id, _| !removals.contains_like_cpp(SPELL_TABLE_HASHES[4], id as i32));
        result
            .spell_casting_requirements
            .retain(|&id, _| !removals.contains_like_cpp(SPELL_TABLE_HASHES[5], id as i32));
        result
            .spell_categories
            .retain(|&id, _| !removals.contains_like_cpp(SPELL_TABLE_HASHES[6], id as i32));
        result
            .spell_class_options
            .retain(|&id, _| !removals.contains_like_cpp(SPELL_TABLE_HASHES[7], id as i32));
        result
            .spell_cooldowns
            .retain(|&id, _| !removals.contains_like_cpp(SPELL_TABLE_HASHES[8], id as i32));
        result
            .spell_empowers
            .retain(|&id, _| !removals.contains_like_cpp(SPELL_TABLE_HASHES[9], id as i32));
        result
            .spell_empower_stages
            .retain(|&id, _| !removals.contains_like_cpp(SPELL_TABLE_HASHES[10], id as i32));
        result
            .spell_equipped_items
            .retain(|&id, _| !removals.contains_like_cpp(SPELL_TABLE_HASHES[11], id as i32));
        result
            .spell_interrupts
            .retain(|&id, _| !removals.contains_like_cpp(SPELL_TABLE_HASHES[12], id as i32));
        result
            .spell_labels
            .retain(|&id, _| !removals.contains_like_cpp(SPELL_TABLE_HASHES[13], id as i32));
        result
            .spell_levels
            .retain(|&id, _| !removals.contains_like_cpp(SPELL_TABLE_HASHES[14], id as i32));
        result
            .spell_powers
            .retain(|&id, _| !removals.contains_like_cpp(SPELL_TABLE_HASHES[15], id as i32));
        result
            .spell_power_difficulties
            .retain(|&id, _| !removals.contains_like_cpp(SPELL_TABLE_HASHES[16], id as i32));
        result
            .spell_reagents
            .retain(|&id, _| !removals.contains_like_cpp(SPELL_TABLE_HASHES[17], id as i32));
        result
            .spell_reagents_currencies
            .retain(|&id, _| !removals.contains_like_cpp(SPELL_TABLE_HASHES[18], id as i32));
        result
            .spell_scaling
            .retain(|&id, _| !removals.contains_like_cpp(SPELL_TABLE_HASHES[19], id as i32));
        result
            .spell_shapeshifts
            .retain(|&id, _| !removals.contains_like_cpp(SPELL_TABLE_HASHES[20], id as i32));
        result
            .spell_target_restrictions
            .retain(|&id, _| !removals.contains_like_cpp(SPELL_TABLE_HASHES[21], id as i32));
        result
            .spell_totems
            .retain(|&id, _| !removals.contains_like_cpp(SPELL_TABLE_HASHES[22], id as i32));
        result
            .spell_x_spell_visuals
            .retain(|&id, _| !removals.contains_like_cpp(SPELL_TABLE_HASHES[23], id as i32));
        result
            .difficulties
            .retain(|&id, _| !removals.contains_like_cpp(SPELL_TABLE_HASHES[24], id as i32));
        result
            .spell_cast_times
            .retain(|&id, _| !removals.contains_like_cpp(SPELL_TABLE_HASHES[25], id as i32));
        result
            .spell_durations
            .retain(|&id, _| !removals.contains_like_cpp(SPELL_TABLE_HASHES[26], id as i32));
        result
            .spell_ranges
            .retain(|&id, _| !removals.contains_like_cpp(SPELL_TABLE_HASHES[27], id as i32));
        result
            .spell_radii
            .retain(|&id, _| !removals.contains_like_cpp(SPELL_TABLE_HASHES[28], id as i32));
        result
            .spell_procs_per_minute
            .retain(|&id, _| !removals.contains_like_cpp(SPELL_TABLE_HASHES[29], id as i32));
        result
            .spell_procs_per_minute_mods
            .retain(|&id, _| !removals.contains_like_cpp(SPELL_TABLE_HASHES[30], id as i32));
        result
            .spell_learn_spells
            .retain(|&id, _| !removals.contains_like_cpp(SPELL_TABLE_HASHES[31], id as i32));
        result
            .spell_shapeshift_forms
            .retain(|&id, _| !removals.contains_like_cpp(SPELL_TABLE_HASHES[32], id as i32));
        result
            .summon_properties
            .retain(|&id, _| !removals.contains_like_cpp(SPELL_TABLE_HASHES[33], id as i32));
        result
            .battle_pet_species
            .retain(|&id, _| !removals.contains_like_cpp(SPELL_TABLE_HASHES[34], id as i32));
        result
            .spell_category_definitions
            .retain(|&id, _| !removals.contains_like_cpp(SPELL_TABLE_HASHES[35], id as i32));
        result
            .talents
            .retain(|&id, _| !removals.contains_like_cpp(SPELL_TABLE_HASHES[36], id as i32));
        result
            .spell_item_enchantments
            .retain(|&id, _| !removals.contains_like_cpp(SPELL_TABLE_HASHES[37], id as i32));
        result
            .spell_visuals
            .retain(|&id, _| !removals.contains_like_cpp(SPELL_TABLE_HASHES[38], id as i32));
        result
            .spell_visual_missiles
            .retain(|&id, _| !removals.contains_like_cpp(SPELL_TABLE_HASHES[39], id as i32));
        result
            .spell_visual_effect_names
            .retain(|&id, _| !removals.contains_like_cpp(SPELL_TABLE_HASHES[40], id as i32));
        result
            .liquid_types
            .retain(|&id, _| !removals.contains_like_cpp(SPELL_TABLE_HASHES[41], id as i32));
        result
            .expected_stats
            .retain(|&id, _| !removals.contains_like_cpp(SPELL_TABLE_HASHES[42], id as i32));
        result
            .expected_stat_mods
            .retain(|&id, _| !removals.contains_like_cpp(SPELL_TABLE_HASHES[43], id as i32));
        result
            .content_tunings
            .retain(|&id, _| !removals.contains_like_cpp(SPELL_TABLE_HASHES[44], id as i32));
        result
            .content_tuning_x_expected
            .retain(|&id, _| !removals.contains_like_cpp(SPELL_TABLE_HASHES[45], id as i32));
        result
            .rand_prop_points
            .retain(|&id, _| !removals.contains_like_cpp(SPELL_TABLE_HASHES[46], id as i32));
        result
            .mythic_plus_seasons
            .retain(|&id, _| !removals.contains_like_cpp(SPELL_TABLE_HASHES[47], id as i32));
        result
            .unit_conditions
            .retain(|&id, _| !removals.contains_like_cpp(SPELL_TABLE_HASHES[48], id as i32));
        // 02245dcd DB2Stores.cpp:1565-1566, after effective hotfix/removal.
        // DB2Storage iterates ascending storage ID, including readable copies.
        for (&id, row) in &result.spell_visual_missiles {
            result
                .spell_visual_missiles_by_set
                .entry(row.spell_visual_missile_set_id)
                .or_default()
                .push(id);
        }
        result.build_value_indexes();
        Ok(result)
    }
}

impl SpellCatalog {
    pub fn unit_condition(&self, id: u32) -> Option<&UnitConditionRecord> {
        self.unit_conditions.get(&id)
    }
    pub fn unit_condition_records(&self) -> impl Iterator<Item = &UnitConditionRecord> {
        self.unit_conditions.values()
    }
    /// Final set membership in source ascending-ID order. Borrow canonical
    /// records; no duplicated mutable missiles or invented unknown membership.
    pub fn spell_visual_missiles_for_set(
        &self,
        set: u32,
    ) -> impl Iterator<Item = &SpellVisualMissileRecord> {
        self.spell_visual_missiles_by_set
            .get(&set)
            .into_iter()
            .flatten()
            .map(|id| &self.spell_visual_missiles[id])
    }

    pub fn talent(&self, id: u32) -> Option<&TalentRecord> {
        self.talents.get(&id)
    }
    pub fn talent_records(&self) -> impl Iterator<Item = &TalentRecord> {
        self.talents.values()
    }
    pub fn spell_item_enchantment(&self, id: u32) -> Option<&SpellItemEnchantmentRecord> {
        self.spell_item_enchantments.get(&id)
    }
    pub fn spell_item_enchantment_records(
        &self,
    ) -> impl Iterator<Item = &SpellItemEnchantmentRecord> {
        self.spell_item_enchantments.values()
    }
    pub fn spell_visual(&self, id: u32) -> Option<&SpellVisualRecord> {
        self.spell_visuals.get(&id)
    }
    pub fn spell_visual_records(&self) -> impl Iterator<Item = &SpellVisualRecord> {
        self.spell_visuals.values()
    }
    pub fn spell_visual_missile(&self, id: u32) -> Option<&SpellVisualMissileRecord> {
        self.spell_visual_missiles.get(&id)
    }
    pub fn spell_visual_missile_records(&self) -> impl Iterator<Item = &SpellVisualMissileRecord> {
        self.spell_visual_missiles.values()
    }
    pub fn spell_visual_effect_name(&self, id: u32) -> Option<&SpellVisualEffectNameRecord> {
        self.spell_visual_effect_names.get(&id)
    }
    pub fn spell_visual_effect_name_records(
        &self,
    ) -> impl Iterator<Item = &SpellVisualEffectNameRecord> {
        self.spell_visual_effect_names.values()
    }
    pub fn liquid_type(&self, id: u32) -> Option<&LiquidTypeRecord> {
        self.liquid_types.get(&id)
    }
    pub fn liquid_type_records(&self) -> impl Iterator<Item = &LiquidTypeRecord> {
        self.liquid_types.values()
    }

    pub fn spell_name(&self, id: u32) -> Option<&SpellNameRecord> {
        self.spell_names.get(&id)
    }
    pub fn spell_name_records(&self) -> impl Iterator<Item = &SpellNameRecord> {
        self.spell_names.values()
    }
    pub fn spell_effect(&self, id: u32) -> Option<&SpellEffectRecord> {
        self.spell_effects.get(&id)
    }
    pub fn spell_effect_records(&self) -> impl Iterator<Item = &SpellEffectRecord> {
        self.spell_effects.values()
    }
    pub fn spell_misc(&self, id: u32) -> Option<&SpellMiscRecord> {
        self.spell_misc.get(&id)
    }
    pub fn spell_misc_records(&self) -> impl Iterator<Item = &SpellMiscRecord> {
        self.spell_misc.values()
    }
    pub fn spell_aura_options(&self, id: u32) -> Option<&SpellAuraOptionsRecord> {
        self.spell_aura_options.get(&id)
    }
    pub fn spell_aura_options_records(&self) -> impl Iterator<Item = &SpellAuraOptionsRecord> {
        self.spell_aura_options.values()
    }
    pub fn spell_aura_restrictions(&self, id: u32) -> Option<&SpellAuraRestrictionsRecord> {
        self.spell_aura_restrictions.get(&id)
    }
    pub fn spell_aura_restrictions_records(
        &self,
    ) -> impl Iterator<Item = &SpellAuraRestrictionsRecord> {
        self.spell_aura_restrictions.values()
    }
    pub fn spell_casting_requirements(&self, id: u32) -> Option<&SpellCastingRequirementsRecord> {
        self.spell_casting_requirements.get(&id)
    }
    pub fn spell_casting_requirements_records(
        &self,
    ) -> impl Iterator<Item = &SpellCastingRequirementsRecord> {
        self.spell_casting_requirements.values()
    }
    pub fn spell_categories(&self, id: u32) -> Option<&SpellCategoriesRecord> {
        self.spell_categories.get(&id)
    }
    pub fn spell_categories_records(&self) -> impl Iterator<Item = &SpellCategoriesRecord> {
        self.spell_categories.values()
    }
    pub fn spell_class_options(&self, id: u32) -> Option<&SpellClassOptionsRecord> {
        self.spell_class_options.get(&id)
    }
    pub fn spell_class_options_records(&self) -> impl Iterator<Item = &SpellClassOptionsRecord> {
        self.spell_class_options.values()
    }
    pub fn spell_cooldowns(&self, id: u32) -> Option<&SpellCooldownsRecord> {
        self.spell_cooldowns.get(&id)
    }
    pub fn spell_cooldowns_records(&self) -> impl Iterator<Item = &SpellCooldownsRecord> {
        self.spell_cooldowns.values()
    }
    pub fn spell_empower(&self, id: u32) -> Option<&SpellEmpowerRecord> {
        self.spell_empowers.get(&id)
    }
    pub fn spell_empower_records(&self) -> impl Iterator<Item = &SpellEmpowerRecord> {
        self.spell_empowers.values()
    }
    pub fn spell_empower_stage(&self, id: u32) -> Option<&SpellEmpowerStageRecord> {
        self.spell_empower_stages.get(&id)
    }
    pub fn spell_empower_stage_records(&self) -> impl Iterator<Item = &SpellEmpowerStageRecord> {
        self.spell_empower_stages.values()
    }
    pub fn spell_equipped_items(&self, id: u32) -> Option<&SpellEquippedItemsRecord> {
        self.spell_equipped_items.get(&id)
    }
    pub fn spell_equipped_items_records(&self) -> impl Iterator<Item = &SpellEquippedItemsRecord> {
        self.spell_equipped_items.values()
    }
    pub fn spell_interrupts(&self, id: u32) -> Option<&SpellInterruptsRecord> {
        self.spell_interrupts.get(&id)
    }
    pub fn spell_interrupts_records(&self) -> impl Iterator<Item = &SpellInterruptsRecord> {
        self.spell_interrupts.values()
    }
    pub fn spell_label(&self, id: u32) -> Option<&SpellLabelRecord> {
        self.spell_labels.get(&id)
    }
    pub fn spell_label_records(&self) -> impl Iterator<Item = &SpellLabelRecord> {
        self.spell_labels.values()
    }
    pub fn spell_levels(&self, id: u32) -> Option<&SpellLevelsRecord> {
        self.spell_levels.get(&id)
    }
    pub fn spell_levels_records(&self) -> impl Iterator<Item = &SpellLevelsRecord> {
        self.spell_levels.values()
    }
    pub fn spell_power(&self, id: u32) -> Option<&SpellPowerRecord> {
        self.spell_powers.get(&id)
    }
    pub fn spell_power_records(&self) -> impl Iterator<Item = &SpellPowerRecord> {
        self.spell_powers.values()
    }
    pub fn spell_power_difficulty(&self, id: u32) -> Option<&SpellPowerDifficultyRecord> {
        self.spell_power_difficulties.get(&id)
    }
    pub fn spell_power_difficulty_records(
        &self,
    ) -> impl Iterator<Item = &SpellPowerDifficultyRecord> {
        self.spell_power_difficulties.values()
    }
    pub fn spell_reagents(&self, id: u32) -> Option<&SpellReagentsRecord> {
        self.spell_reagents.get(&id)
    }
    pub fn spell_reagents_records(&self) -> impl Iterator<Item = &SpellReagentsRecord> {
        self.spell_reagents.values()
    }
    pub fn spell_reagents_currency(&self, id: u32) -> Option<&SpellReagentsCurrencyRecord> {
        self.spell_reagents_currencies.get(&id)
    }
    pub fn spell_reagents_currency_records(
        &self,
    ) -> impl Iterator<Item = &SpellReagentsCurrencyRecord> {
        self.spell_reagents_currencies.values()
    }
    pub fn spell_scaling(&self, id: u32) -> Option<&SpellScalingRecord> {
        self.spell_scaling.get(&id)
    }
    pub fn spell_scaling_records(&self) -> impl Iterator<Item = &SpellScalingRecord> {
        self.spell_scaling.values()
    }
    pub fn spell_shapeshift(&self, id: u32) -> Option<&SpellShapeshiftRecord> {
        self.spell_shapeshifts.get(&id)
    }
    pub fn spell_shapeshift_records(&self) -> impl Iterator<Item = &SpellShapeshiftRecord> {
        self.spell_shapeshifts.values()
    }
    pub fn spell_target_restrictions(&self, id: u32) -> Option<&SpellTargetRestrictionsRecord> {
        self.spell_target_restrictions.get(&id)
    }
    pub fn spell_target_restrictions_records(
        &self,
    ) -> impl Iterator<Item = &SpellTargetRestrictionsRecord> {
        self.spell_target_restrictions.values()
    }
    pub fn spell_totems(&self, id: u32) -> Option<&SpellTotemsRecord> {
        self.spell_totems.get(&id)
    }
    pub fn spell_totems_records(&self) -> impl Iterator<Item = &SpellTotemsRecord> {
        self.spell_totems.values()
    }
    pub fn spell_x_spell_visual(&self, id: u32) -> Option<&SpellXSpellVisualRecord> {
        self.spell_x_spell_visuals.get(&id)
    }
    pub fn spell_x_spell_visual_records(&self) -> impl Iterator<Item = &SpellXSpellVisualRecord> {
        self.spell_x_spell_visuals.values()
    }
    pub fn difficulty(&self, id: u32) -> Option<&DifficultyRecord> {
        self.difficulties.get(&id)
    }
    pub fn difficulty_records(&self) -> impl Iterator<Item = &DifficultyRecord> {
        self.difficulties.values()
    }
    pub fn spell_cast_times(&self, id: u32) -> Option<&SpellCastTimesRecord> {
        self.spell_cast_times.get(&id)
    }
    pub fn spell_cast_times_records(&self) -> impl Iterator<Item = &SpellCastTimesRecord> {
        self.spell_cast_times.values()
    }
    pub fn spell_duration(&self, id: u32) -> Option<&SpellDurationRecord> {
        self.spell_durations.get(&id)
    }
    pub fn spell_duration_records(&self) -> impl Iterator<Item = &SpellDurationRecord> {
        self.spell_durations.values()
    }
    pub fn spell_range(&self, id: u32) -> Option<&SpellRangeRecord> {
        self.spell_ranges.get(&id)
    }
    pub fn spell_range_records(&self) -> impl Iterator<Item = &SpellRangeRecord> {
        self.spell_ranges.values()
    }
    pub fn spell_radius(&self, id: u32) -> Option<&SpellRadiusRecord> {
        self.spell_radii.get(&id)
    }
    pub fn spell_radius_records(&self) -> impl Iterator<Item = &SpellRadiusRecord> {
        self.spell_radii.values()
    }
    pub fn spell_procs_per_minute(&self, id: u32) -> Option<&SpellProcsPerMinuteRecord> {
        self.spell_procs_per_minute.get(&id)
    }
    pub fn spell_procs_per_minute_records(
        &self,
    ) -> impl Iterator<Item = &SpellProcsPerMinuteRecord> {
        self.spell_procs_per_minute.values()
    }
    pub fn spell_procs_per_minute_mod(&self, id: u32) -> Option<&SpellProcsPerMinuteModRecord> {
        self.spell_procs_per_minute_mods.get(&id)
    }
    pub fn spell_procs_per_minute_mod_records(
        &self,
    ) -> impl Iterator<Item = &SpellProcsPerMinuteModRecord> {
        self.spell_procs_per_minute_mods.values()
    }
    pub fn spell_learn_spell(&self, id: u32) -> Option<&SpellLearnSpellRecord> {
        self.spell_learn_spells.get(&id)
    }
    pub fn spell_learn_spell_records(&self) -> impl Iterator<Item = &SpellLearnSpellRecord> {
        self.spell_learn_spells.values()
    }
    pub fn spell_shapeshift_form(&self, id: u32) -> Option<&SpellShapeshiftFormRecord> {
        self.spell_shapeshift_forms.get(&id)
    }
    pub fn spell_shapeshift_form_records(
        &self,
    ) -> impl Iterator<Item = &SpellShapeshiftFormRecord> {
        self.spell_shapeshift_forms.values()
    }
    pub fn summon_properties(&self, id: u32) -> Option<&SummonPropertiesRecord> {
        self.summon_properties.get(&id)
    }
    pub fn summon_properties_records(&self) -> impl Iterator<Item = &SummonPropertiesRecord> {
        self.summon_properties.values()
    }
    pub fn battle_pet_species(&self, id: u32) -> Option<&BattlePetSpeciesRecord> {
        self.battle_pet_species.get(&id)
    }
    pub fn battle_pet_species_records(&self) -> impl Iterator<Item = &BattlePetSpeciesRecord> {
        self.battle_pet_species.values()
    }
    pub fn spell_category(&self, id: u32) -> Option<&SpellCategoryRecord> {
        self.spell_category_definitions.get(&id)
    }
    pub fn spell_category_records(&self) -> impl Iterator<Item = &SpellCategoryRecord> {
        self.spell_category_definitions.values()
    }
    /// Counts describe known effective records and unknown direct baseline rows.
    /// They do not certify complete ID coverage or missing-spell absence.
    pub fn counts(&self) -> [(&'static str, usize, usize); 49] {
        [
            (
                "SpellName",
                self.spell_names.len(),
                self.unknown_baseline_records[0],
            ),
            (
                "SpellEffect",
                self.spell_effects.len(),
                self.unknown_baseline_records[1],
            ),
            (
                "SpellMisc",
                self.spell_misc.len(),
                self.unknown_baseline_records[2],
            ),
            (
                "SpellAuraOptions",
                self.spell_aura_options.len(),
                self.unknown_baseline_records[3],
            ),
            (
                "SpellAuraRestrictions",
                self.spell_aura_restrictions.len(),
                self.unknown_baseline_records[4],
            ),
            (
                "SpellCastingRequirements",
                self.spell_casting_requirements.len(),
                self.unknown_baseline_records[5],
            ),
            (
                "SpellCategories",
                self.spell_categories.len(),
                self.unknown_baseline_records[6],
            ),
            (
                "SpellClassOptions",
                self.spell_class_options.len(),
                self.unknown_baseline_records[7],
            ),
            (
                "SpellCooldowns",
                self.spell_cooldowns.len(),
                self.unknown_baseline_records[8],
            ),
            (
                "SpellEmpower",
                self.spell_empowers.len(),
                self.unknown_baseline_records[9],
            ),
            (
                "SpellEmpowerStage",
                self.spell_empower_stages.len(),
                self.unknown_baseline_records[10],
            ),
            (
                "SpellEquippedItems",
                self.spell_equipped_items.len(),
                self.unknown_baseline_records[11],
            ),
            (
                "SpellInterrupts",
                self.spell_interrupts.len(),
                self.unknown_baseline_records[12],
            ),
            (
                "SpellLabel",
                self.spell_labels.len(),
                self.unknown_baseline_records[13],
            ),
            (
                "SpellLevels",
                self.spell_levels.len(),
                self.unknown_baseline_records[14],
            ),
            (
                "SpellPower",
                self.spell_powers.len(),
                self.unknown_baseline_records[15],
            ),
            (
                "SpellPowerDifficulty",
                self.spell_power_difficulties.len(),
                self.unknown_baseline_records[16],
            ),
            (
                "SpellReagents",
                self.spell_reagents.len(),
                self.unknown_baseline_records[17],
            ),
            (
                "SpellReagentsCurrency",
                self.spell_reagents_currencies.len(),
                self.unknown_baseline_records[18],
            ),
            (
                "SpellScaling",
                self.spell_scaling.len(),
                self.unknown_baseline_records[19],
            ),
            (
                "SpellShapeshift",
                self.spell_shapeshifts.len(),
                self.unknown_baseline_records[20],
            ),
            (
                "SpellTargetRestrictions",
                self.spell_target_restrictions.len(),
                self.unknown_baseline_records[21],
            ),
            (
                "SpellTotems",
                self.spell_totems.len(),
                self.unknown_baseline_records[22],
            ),
            (
                "SpellXSpellVisual",
                self.spell_x_spell_visuals.len(),
                self.unknown_baseline_records[23],
            ),
            (
                "Difficulty",
                self.difficulties.len(),
                self.unknown_baseline_records[24],
            ),
            (
                "SpellCastTimes",
                self.spell_cast_times.len(),
                self.unknown_baseline_records[25],
            ),
            (
                "SpellDuration",
                self.spell_durations.len(),
                self.unknown_baseline_records[26],
            ),
            (
                "SpellRange",
                self.spell_ranges.len(),
                self.unknown_baseline_records[27],
            ),
            (
                "SpellRadius",
                self.spell_radii.len(),
                self.unknown_baseline_records[28],
            ),
            (
                "SpellProcsPerMinute",
                self.spell_procs_per_minute.len(),
                self.unknown_baseline_records[29],
            ),
            (
                "SpellProcsPerMinuteMod",
                self.spell_procs_per_minute_mods.len(),
                self.unknown_baseline_records[30],
            ),
            (
                "SpellLearnSpell",
                self.spell_learn_spells.len(),
                self.unknown_baseline_records[31],
            ),
            (
                "SpellShapeshiftForm",
                self.spell_shapeshift_forms.len(),
                self.unknown_baseline_records[32],
            ),
            (
                "SummonProperties",
                self.summon_properties.len(),
                self.unknown_baseline_records[33],
            ),
            (
                "BattlePetSpecies",
                self.battle_pet_species.len(),
                self.unknown_baseline_records[34],
            ),
            (
                "SpellCategory",
                self.spell_category_definitions.len(),
                self.unknown_baseline_records[35],
            ),
            (
                "Talent",
                self.talents.len(),
                self.unknown_baseline_records[36],
            ),
            (
                "SpellItemEnchantment",
                self.spell_item_enchantments.len(),
                self.unknown_baseline_records[37],
            ),
            (
                "SpellVisual",
                self.spell_visuals.len(),
                self.unknown_baseline_records[38],
            ),
            (
                "SpellVisualMissile",
                self.spell_visual_missiles.len(),
                self.unknown_baseline_records[39],
            ),
            (
                "SpellVisualEffectName",
                self.spell_visual_effect_names.len(),
                self.unknown_baseline_records[40],
            ),
            (
                "LiquidType",
                self.liquid_types.len(),
                self.unknown_baseline_records[41],
            ),
            (
                "ExpectedStat",
                self.expected_stats.len(),
                self.unknown_baseline_records[42],
            ),
            (
                "ExpectedStatMod",
                self.expected_stat_mods.len(),
                self.unknown_baseline_records[43],
            ),
            (
                "ContentTuning",
                self.content_tunings.len(),
                self.unknown_baseline_records[44],
            ),
            (
                "ContentTuningXExpected",
                self.content_tuning_x_expected.len(),
                self.unknown_baseline_records[45],
            ),
            (
                "RandPropPoints",
                self.rand_prop_points.len(),
                self.unknown_baseline_records[46],
            ),
            (
                "MythicPlusSeason",
                self.mythic_plus_seasons.len(),
                self.unknown_baseline_records[47],
            ),
            (
                "UnitCondition",
                self.unit_conditions.len(),
                self.unknown_baseline_records[48],
            ),
        ]
    }
}
