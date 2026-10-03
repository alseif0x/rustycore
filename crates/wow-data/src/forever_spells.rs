//! Raw and effective build-70170 spell inputs, with target SQL/locale/removal
//! composition. Not SpellInfo, a learned spellbook, a Player or a gameplay
//! fallback to 3.4.3.
use anyhow::Result;
use std::path::Path;
mod core;
mod costs;
#[cfg(test)]
pub(crate) mod custom_source_fixtures;
mod custom_sources;
mod dependencies;
mod effective;
mod load;
mod locales;
mod text;
#[cfg(test)]
pub(crate) mod value_input_fixtures;
mod value_inputs;
pub use self::core::*;
pub use self::costs::*;
pub use self::custom_sources::*;
pub use self::dependencies::*;
pub use effective::{
    SPELL_TABLE_HASHES, SpellCatalog, SummonPropertiesPatch, SummonPropertiesPatchCounts,
};
pub use locales::*;
pub use text::SpellText;
pub use value_inputs::*;

#[derive(Clone, Copy)]
pub enum SpellBaseline {
    Complete,
    /// Twenty-four exact 70170/esES readable prefixes; unknown rows stay unknown.
    /// Other tables are still complete reads. Never an automatic fallback.
    AvailablePrefixes,
}

pub struct SpellRecords {
    pub spell_names: Vec<SpellNameRecord>,
    pub spell_effects: Vec<SpellEffectRecord>,
    pub spell_misc: Vec<SpellMiscRecord>,
    pub spell_aura_options: Vec<SpellAuraOptionsRecord>,
    pub spell_aura_restrictions: Vec<SpellAuraRestrictionsRecord>,
    pub spell_casting_requirements: Vec<SpellCastingRequirementsRecord>,
    pub spell_categories: Vec<SpellCategoriesRecord>,
    pub spell_class_options: Vec<SpellClassOptionsRecord>,
    pub spell_cooldowns: Vec<SpellCooldownsRecord>,
    pub spell_empowers: Vec<SpellEmpowerRecord>,
    pub spell_empower_stages: Vec<SpellEmpowerStageRecord>,
    pub spell_equipped_items: Vec<SpellEquippedItemsRecord>,
    pub spell_interrupts: Vec<SpellInterruptsRecord>,
    pub spell_labels: Vec<SpellLabelRecord>,
    pub spell_levels: Vec<SpellLevelsRecord>,
    pub spell_powers: Vec<SpellPowerRecord>,
    pub spell_power_difficulties: Vec<SpellPowerDifficultyRecord>,
    pub spell_reagents: Vec<SpellReagentsRecord>,
    pub spell_reagents_currencies: Vec<SpellReagentsCurrencyRecord>,
    pub spell_scaling: Vec<SpellScalingRecord>,
    pub spell_shapeshifts: Vec<SpellShapeshiftRecord>,
    pub spell_target_restrictions: Vec<SpellTargetRestrictionsRecord>,
    pub spell_totems: Vec<SpellTotemsRecord>,
    pub spell_x_spell_visuals: Vec<SpellXSpellVisualRecord>,
    pub difficulties: Vec<DifficultyRecord>,
    pub spell_cast_times: Vec<SpellCastTimesRecord>,
    pub spell_durations: Vec<SpellDurationRecord>,
    pub spell_ranges: Vec<SpellRangeRecord>,
    pub spell_radii: Vec<SpellRadiusRecord>,
    pub spell_procs_per_minute: Vec<SpellProcsPerMinuteRecord>,
    pub spell_procs_per_minute_mods: Vec<SpellProcsPerMinuteModRecord>,
    pub spell_learn_spells: Vec<SpellLearnSpellRecord>,
    pub spell_shapeshift_forms: Vec<SpellShapeshiftFormRecord>,
    pub summon_properties: Vec<SummonPropertiesRecord>,
    pub battle_pet_species: Vec<BattlePetSpeciesRecord>,
    pub spell_category_definitions: Vec<SpellCategoryRecord>,
    pub talents: Vec<TalentRecord>,
    pub spell_item_enchantments: Vec<SpellItemEnchantmentRecord>,
    pub spell_visuals: Vec<SpellVisualRecord>,
    pub spell_visual_missiles: Vec<SpellVisualMissileRecord>,
    pub spell_visual_effect_names: Vec<SpellVisualEffectNameRecord>,
    pub liquid_types: Vec<LiquidTypeRecord>,
    pub expected_stats: Vec<ExpectedStatRecord>,
    pub expected_stat_mods: Vec<ExpectedStatModRecord>,
    pub content_tunings: Vec<ContentTuningRecord>,
    pub content_tuning_x_expected: Vec<ContentTuningXExpectedRecord>,
    pub rand_prop_points: Vec<RandPropPointsRecord>,
    pub mythic_plus_seasons: Vec<MythicPlusSeasonRecord>,
    pub unit_conditions: Vec<UnitConditionRecord>,
    /// File loader GetMaxId includes unresolved copy destinations. This is
    /// allocation metadata, not the greatest surviving effective record ID.
    /// SQL batches leave it None; synthetic baselines may derive it from rows.
    pub rand_prop_points_storage_last_index: Option<u32>,
    /// Separate source SQL allocation observation, not a transactional
    /// snapshot or a file bound. None for baselines/synthetic row batches.
    pub rand_prop_points_sql_index_size: Option<u32>,
    /// Counts of unknown direct baseline rows in source table order. This is
    /// not complete ID coverage, unknown copy coverage or effective absence.
    pub unknown_baseline_records: [usize; 49],
}

impl Default for SpellRecords {
    fn default() -> Self {
        Self {
            spell_names: Vec::new(),
            spell_effects: Vec::new(),
            spell_misc: Vec::new(),
            spell_aura_options: Vec::new(),
            spell_aura_restrictions: Vec::new(),
            spell_casting_requirements: Vec::new(),
            spell_categories: Vec::new(),
            spell_class_options: Vec::new(),
            spell_cooldowns: Vec::new(),
            spell_empowers: Vec::new(),
            spell_empower_stages: Vec::new(),
            spell_equipped_items: Vec::new(),
            spell_interrupts: Vec::new(),
            spell_labels: Vec::new(),
            spell_levels: Vec::new(),
            spell_powers: Vec::new(),
            spell_power_difficulties: Vec::new(),
            spell_reagents: Vec::new(),
            spell_reagents_currencies: Vec::new(),
            spell_scaling: Vec::new(),
            spell_shapeshifts: Vec::new(),
            spell_target_restrictions: Vec::new(),
            spell_totems: Vec::new(),
            spell_x_spell_visuals: Vec::new(),
            difficulties: Vec::new(),
            spell_cast_times: Vec::new(),
            spell_durations: Vec::new(),
            spell_ranges: Vec::new(),
            spell_radii: Vec::new(),
            spell_procs_per_minute: Vec::new(),
            spell_procs_per_minute_mods: Vec::new(),
            spell_learn_spells: Vec::new(),
            spell_shapeshift_forms: Vec::new(),
            summon_properties: Vec::new(),
            battle_pet_species: Vec::new(),
            spell_category_definitions: Vec::new(),
            talents: Vec::new(),
            spell_item_enchantments: Vec::new(),
            spell_visuals: Vec::new(),
            spell_visual_missiles: Vec::new(),
            spell_visual_effect_names: Vec::new(),
            liquid_types: Vec::new(),
            expected_stats: Vec::new(),
            expected_stat_mods: Vec::new(),
            content_tunings: Vec::new(),
            content_tuning_x_expected: Vec::new(),
            rand_prop_points: Vec::new(),
            mythic_plus_seasons: Vec::new(),
            unit_conditions: Vec::new(),
            rand_prop_points_storage_last_index: None,
            rand_prop_points_sql_index_size: None,
            unknown_baseline_records: [0; 49],
        }
    }
}

impl SpellRecords {
    /// All forty-nine checked reads finish before publishing the raw batch.
    pub fn load(directory: &Path, baseline: SpellBaseline) -> Result<Self> {
        load::records(directory, baseline)
    }

    /// Materialized known rows (including copies), unknown direct rows.
    /// Only public schema names/counts; no values, text or identifiers.
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
