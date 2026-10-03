use super::{SpellBaseline, SpellRecords};
use crate::wdc4::creation::{CreationDb2, SpellTable};
use anyhow::Result;
use std::path::Path;
mod core;
mod costs;
mod custom_sources;
mod dependencies;
mod value_inputs;

pub(super) fn records(directory: &Path, baseline: SpellBaseline) -> Result<SpellRecords> {
    let available = matches!(baseline, SpellBaseline::AvailablePrefixes);
    let mut result = SpellRecords::default();
    let (table, unknown) = CreationDb2::open_spell(directory, SpellTable::SpellName, available)?;
    result.spell_names = table
        .ids()
        .map(|id| core::spell_name(&table, id))
        .collect::<Result<Vec<_>>>()?;
    result.unknown_baseline_records[0] = unknown;
    let (table, unknown) = CreationDb2::open_spell(directory, SpellTable::SpellEffect, available)?;
    result.spell_effects = table
        .ids()
        .map(|id| core::spell_effect(&table, id))
        .collect::<Result<Vec<_>>>()?;
    result.unknown_baseline_records[1] = unknown;
    let (table, unknown) = CreationDb2::open_spell(directory, SpellTable::SpellMisc, available)?;
    result.spell_misc = table
        .ids()
        .map(|id| core::spell_misc(&table, id))
        .collect::<Result<Vec<_>>>()?;
    result.unknown_baseline_records[2] = unknown;
    let (table, unknown) =
        CreationDb2::open_spell(directory, SpellTable::SpellAuraOptions, available)?;
    result.spell_aura_options = table
        .ids()
        .map(|id| core::spell_aura_options(&table, id))
        .collect::<Result<Vec<_>>>()?;
    result.unknown_baseline_records[3] = unknown;
    let (table, unknown) =
        CreationDb2::open_spell(directory, SpellTable::SpellAuraRestrictions, available)?;
    result.spell_aura_restrictions = table
        .ids()
        .map(|id| core::spell_aura_restrictions(&table, id))
        .collect::<Result<Vec<_>>>()?;
    result.unknown_baseline_records[4] = unknown;
    let (table, unknown) =
        CreationDb2::open_spell(directory, SpellTable::SpellCastingRequirements, available)?;
    result.spell_casting_requirements = table
        .ids()
        .map(|id| core::spell_casting_requirements(&table, id))
        .collect::<Result<Vec<_>>>()?;
    result.unknown_baseline_records[5] = unknown;
    let (table, unknown) =
        CreationDb2::open_spell(directory, SpellTable::SpellCategories, available)?;
    result.spell_categories = table
        .ids()
        .map(|id| core::spell_categories(&table, id))
        .collect::<Result<Vec<_>>>()?;
    result.unknown_baseline_records[6] = unknown;
    let (table, unknown) =
        CreationDb2::open_spell(directory, SpellTable::SpellClassOptions, available)?;
    result.spell_class_options = table
        .ids()
        .map(|id| core::spell_class_options(&table, id))
        .collect::<Result<Vec<_>>>()?;
    result.unknown_baseline_records[7] = unknown;
    let (table, unknown) =
        CreationDb2::open_spell(directory, SpellTable::SpellCooldowns, available)?;
    result.spell_cooldowns = table
        .ids()
        .map(|id| core::spell_cooldowns(&table, id))
        .collect::<Result<Vec<_>>>()?;
    result.unknown_baseline_records[8] = unknown;
    let (table, unknown) = CreationDb2::open_spell(directory, SpellTable::SpellEmpower, available)?;
    result.spell_empowers = table
        .ids()
        .map(|id| costs::spell_empower(&table, id))
        .collect::<Result<Vec<_>>>()?;
    result.unknown_baseline_records[9] = unknown;
    let (table, unknown) =
        CreationDb2::open_spell(directory, SpellTable::SpellEmpowerStage, available)?;
    result.spell_empower_stages = table
        .ids()
        .map(|id| costs::spell_empower_stage(&table, id))
        .collect::<Result<Vec<_>>>()?;
    result.unknown_baseline_records[10] = unknown;
    let (table, unknown) =
        CreationDb2::open_spell(directory, SpellTable::SpellEquippedItems, available)?;
    result.spell_equipped_items = table
        .ids()
        .map(|id| core::spell_equipped_items(&table, id))
        .collect::<Result<Vec<_>>>()?;
    result.unknown_baseline_records[11] = unknown;
    let (table, unknown) =
        CreationDb2::open_spell(directory, SpellTable::SpellInterrupts, available)?;
    result.spell_interrupts = table
        .ids()
        .map(|id| core::spell_interrupts(&table, id))
        .collect::<Result<Vec<_>>>()?;
    result.unknown_baseline_records[12] = unknown;
    let (table, unknown) = CreationDb2::open_spell(directory, SpellTable::SpellLabel, available)?;
    result.spell_labels = table
        .ids()
        .map(|id| core::spell_label(&table, id))
        .collect::<Result<Vec<_>>>()?;
    result.unknown_baseline_records[13] = unknown;
    let (table, unknown) = CreationDb2::open_spell(directory, SpellTable::SpellLevels, available)?;
    result.spell_levels = table
        .ids()
        .map(|id| core::spell_levels(&table, id))
        .collect::<Result<Vec<_>>>()?;
    result.unknown_baseline_records[14] = unknown;
    let (table, unknown) = CreationDb2::open_spell(directory, SpellTable::SpellPower, available)?;
    result.spell_powers = table
        .ids()
        .map(|id| costs::spell_power(&table, id))
        .collect::<Result<Vec<_>>>()?;
    result.unknown_baseline_records[15] = unknown;
    let (table, unknown) =
        CreationDb2::open_spell(directory, SpellTable::SpellPowerDifficulty, available)?;
    result.spell_power_difficulties = table
        .ids()
        .map(|id| costs::spell_power_difficulty(&table, id))
        .collect::<Result<Vec<_>>>()?;
    result.unknown_baseline_records[16] = unknown;
    let (table, unknown) =
        CreationDb2::open_spell(directory, SpellTable::SpellReagents, available)?;
    result.spell_reagents = table
        .ids()
        .map(|id| costs::spell_reagents(&table, id))
        .collect::<Result<Vec<_>>>()?;
    result.unknown_baseline_records[17] = unknown;
    let (table, unknown) =
        CreationDb2::open_spell(directory, SpellTable::SpellReagentsCurrency, available)?;
    result.spell_reagents_currencies = table
        .ids()
        .map(|id| costs::spell_reagents_currency(&table, id))
        .collect::<Result<Vec<_>>>()?;
    result.unknown_baseline_records[18] = unknown;
    let (table, unknown) = CreationDb2::open_spell(directory, SpellTable::SpellScaling, available)?;
    result.spell_scaling = table
        .ids()
        .map(|id| costs::spell_scaling(&table, id))
        .collect::<Result<Vec<_>>>()?;
    result.unknown_baseline_records[19] = unknown;
    let (table, unknown) =
        CreationDb2::open_spell(directory, SpellTable::SpellShapeshift, available)?;
    result.spell_shapeshifts = table
        .ids()
        .map(|id| costs::spell_shapeshift(&table, id))
        .collect::<Result<Vec<_>>>()?;
    result.unknown_baseline_records[20] = unknown;
    let (table, unknown) =
        CreationDb2::open_spell(directory, SpellTable::SpellTargetRestrictions, available)?;
    result.spell_target_restrictions = table
        .ids()
        .map(|id| costs::spell_target_restrictions(&table, id))
        .collect::<Result<Vec<_>>>()?;
    result.unknown_baseline_records[21] = unknown;
    let (table, unknown) = CreationDb2::open_spell(directory, SpellTable::SpellTotems, available)?;
    result.spell_totems = table
        .ids()
        .map(|id| costs::spell_totems(&table, id))
        .collect::<Result<Vec<_>>>()?;
    result.unknown_baseline_records[22] = unknown;
    let (table, unknown) =
        CreationDb2::open_spell(directory, SpellTable::SpellXSpellVisual, available)?;
    result.spell_x_spell_visuals = table
        .ids()
        .map(|id| costs::spell_x_spell_visual(&table, id))
        .collect::<Result<Vec<_>>>()?;
    result.unknown_baseline_records[23] = unknown;
    let (table, unknown) = CreationDb2::open_spell(directory, SpellTable::Difficulty, available)?;
    result.difficulties = table
        .ids()
        .map(|id| dependencies::difficulty(&table, id))
        .collect::<Result<Vec<_>>>()?;
    result.unknown_baseline_records[24] = unknown;
    let (table, unknown) =
        CreationDb2::open_spell(directory, SpellTable::SpellCastTimes, available)?;
    result.spell_cast_times = table
        .ids()
        .map(|id| dependencies::spell_cast_times(&table, id))
        .collect::<Result<Vec<_>>>()?;
    result.unknown_baseline_records[25] = unknown;
    let (table, unknown) =
        CreationDb2::open_spell(directory, SpellTable::SpellDuration, available)?;
    result.spell_durations = table
        .ids()
        .map(|id| dependencies::spell_duration(&table, id))
        .collect::<Result<Vec<_>>>()?;
    result.unknown_baseline_records[26] = unknown;
    let (table, unknown) = CreationDb2::open_spell(directory, SpellTable::SpellRange, available)?;
    result.spell_ranges = table
        .ids()
        .map(|id| dependencies::spell_range(&table, id))
        .collect::<Result<Vec<_>>>()?;
    result.unknown_baseline_records[27] = unknown;
    let (table, unknown) = CreationDb2::open_spell(directory, SpellTable::SpellRadius, available)?;
    result.spell_radii = table
        .ids()
        .map(|id| dependencies::spell_radius(&table, id))
        .collect::<Result<Vec<_>>>()?;
    result.unknown_baseline_records[28] = unknown;
    let (table, unknown) =
        CreationDb2::open_spell(directory, SpellTable::SpellProcsPerMinute, available)?;
    result.spell_procs_per_minute = table
        .ids()
        .map(|id| dependencies::spell_procs_per_minute(&table, id))
        .collect::<Result<Vec<_>>>()?;
    result.unknown_baseline_records[29] = unknown;
    let (table, unknown) =
        CreationDb2::open_spell(directory, SpellTable::SpellProcsPerMinuteMod, available)?;
    result.spell_procs_per_minute_mods = table
        .ids()
        .map(|id| dependencies::spell_procs_per_minute_mod(&table, id))
        .collect::<Result<Vec<_>>>()?;
    result.unknown_baseline_records[30] = unknown;
    let (table, unknown) =
        CreationDb2::open_spell(directory, SpellTable::SpellLearnSpell, available)?;
    result.spell_learn_spells = table
        .ids()
        .map(|id| dependencies::spell_learn_spell(&table, id))
        .collect::<Result<Vec<_>>>()?;
    result.unknown_baseline_records[31] = unknown;
    let (table, unknown) =
        CreationDb2::open_spell(directory, SpellTable::SpellShapeshiftForm, available)?;
    result.spell_shapeshift_forms = table
        .ids()
        .map(|id| dependencies::spell_shapeshift_form(&table, id))
        .collect::<Result<Vec<_>>>()?;
    result.unknown_baseline_records[32] = unknown;
    let (table, unknown) =
        CreationDb2::open_spell(directory, SpellTable::SummonProperties, available)?;
    result.summon_properties = table
        .ids()
        .map(|id| dependencies::summon_properties(&table, id))
        .collect::<Result<Vec<_>>>()?;
    result.unknown_baseline_records[33] = unknown;
    let (table, unknown) =
        CreationDb2::open_spell(directory, SpellTable::BattlePetSpecies, available)?;
    result.battle_pet_species = table
        .ids()
        .map(|id| dependencies::battle_pet_species(&table, id))
        .collect::<Result<Vec<_>>>()?;
    result.unknown_baseline_records[34] = unknown;
    let (table, unknown) =
        CreationDb2::open_spell(directory, SpellTable::SpellCategory, available)?;
    result.spell_category_definitions = table
        .ids()
        .map(|id| dependencies::spell_category(&table, id))
        .collect::<Result<Vec<_>>>()?;
    result.unknown_baseline_records[35] = unknown;
    let (table, unknown) = CreationDb2::open_spell(directory, SpellTable::Talent, available)?;
    result.talents = table
        .ids()
        .map(|id| custom_sources::talent(&table, id))
        .collect::<Result<Vec<_>>>()?;
    result.unknown_baseline_records[36] = unknown;
    let (table, unknown) =
        CreationDb2::open_spell(directory, SpellTable::SpellItemEnchantment, available)?;
    result.spell_item_enchantments = table
        .ids()
        .map(|id| custom_sources::spell_item_enchantment(&table, id))
        .collect::<Result<Vec<_>>>()?;
    result.unknown_baseline_records[37] = unknown;
    let (table, unknown) = CreationDb2::open_spell(directory, SpellTable::SpellVisual, available)?;
    result.spell_visuals = table
        .ids()
        .map(|id| custom_sources::spell_visual(&table, id))
        .collect::<Result<Vec<_>>>()?;
    result.unknown_baseline_records[38] = unknown;
    let (table, unknown) =
        CreationDb2::open_spell(directory, SpellTable::SpellVisualMissile, available)?;
    result.spell_visual_missiles = table
        .ids()
        .map(|id| custom_sources::spell_visual_missile(&table, id))
        .collect::<Result<Vec<_>>>()?;
    result.unknown_baseline_records[39] = unknown;
    let (table, unknown) =
        CreationDb2::open_spell(directory, SpellTable::SpellVisualEffectName, available)?;
    result.spell_visual_effect_names = table
        .ids()
        .map(|id| custom_sources::spell_visual_effect_name(&table, id))
        .collect::<Result<Vec<_>>>()?;
    result.unknown_baseline_records[40] = unknown;
    let (table, unknown) = CreationDb2::open_spell(directory, SpellTable::LiquidType, available)?;
    result.liquid_types = table
        .ids()
        .map(|id| custom_sources::liquid_type(&table, id))
        .collect::<Result<Vec<_>>>()?;
    result.unknown_baseline_records[41] = unknown;
    let (table, unknown) = CreationDb2::open_spell(directory, SpellTable::ExpectedStat, available)?;
    result.expected_stats = table
        .ids()
        .map(|id| value_inputs::expected_stat(&table, id))
        .collect::<Result<Vec<_>>>()?;
    result.unknown_baseline_records[42] = unknown;
    let (table, unknown) =
        CreationDb2::open_spell(directory, SpellTable::ExpectedStatMod, available)?;
    result.expected_stat_mods = table
        .ids()
        .map(|id| value_inputs::expected_stat_mod(&table, id))
        .collect::<Result<Vec<_>>>()?;
    result.unknown_baseline_records[43] = unknown;
    let (table, unknown) =
        CreationDb2::open_spell(directory, SpellTable::ContentTuning, available)?;
    result.content_tunings = table
        .ids()
        .map(|id| value_inputs::content_tuning(&table, id))
        .collect::<Result<Vec<_>>>()?;
    result.unknown_baseline_records[44] = unknown;
    let (table, unknown) =
        CreationDb2::open_spell(directory, SpellTable::ContentTuningXExpected, available)?;
    result.content_tuning_x_expected = table
        .ids()
        .map(|id| value_inputs::content_tuning_x_expected(&table, id))
        .collect::<Result<Vec<_>>>()?;
    result.unknown_baseline_records[45] = unknown;
    let (table, unknown) =
        CreationDb2::open_spell(directory, SpellTable::RandPropPoints, available)?;
    result.rand_prop_points_storage_last_index = Some(table.storage_last_index());
    result.rand_prop_points = table
        .ids()
        .map(|id| value_inputs::rand_prop_points(&table, id))
        .collect::<Result<Vec<_>>>()?;
    result.unknown_baseline_records[46] = unknown;
    let (table, unknown) =
        CreationDb2::open_spell(directory, SpellTable::MythicPlusSeason, available)?;
    result.mythic_plus_seasons = table
        .ids()
        .map(|id| value_inputs::mythic_plus_season(&table, id))
        .collect::<Result<Vec<_>>>()?;
    result.unknown_baseline_records[47] = unknown;
    let (table, unknown) =
        CreationDb2::open_spell(directory, SpellTable::UnitCondition, available)?;
    result.unit_conditions = table
        .ids()
        .map(|id| custom_sources::unit_condition(&table, id))
        .collect::<Result<Vec<_>>>()?;
    result.unknown_baseline_records[48] = unknown;
    Ok(result)
}
