//! SQL-free spell overlay batches and World startup inputs.
//! No effective catalog, ready SpellInfo or Player state.
//! 02245dcd HotfixDatabase/DB2LoadInfo: exact field order and source widths.
mod core;
mod costs;
mod custom;
mod custom_sources;
mod dependencies;
mod immunities;
mod learning;
mod locales;
pub mod server;
mod value_inputs;
pub use self::core::*;
pub use self::costs::*;
pub use self::custom::*;
pub use self::custom_sources::*;
pub use self::dependencies::*;
pub use self::immunities::*;
pub use self::learning::*;
pub use self::locales::*;
pub use self::value_inputs::*;

#[derive(Default)]
pub struct SpellRows {
    pub spell_names: Vec<SpellNameRow>,
    pub spell_effects: Vec<SpellEffectRow>,
    pub spell_misc: Vec<SpellMiscRow>,
    pub spell_aura_options: Vec<SpellAuraOptionsRow>,
    pub spell_aura_restrictions: Vec<SpellAuraRestrictionsRow>,
    pub spell_casting_requirements: Vec<SpellCastingRequirementsRow>,
    pub spell_categories: Vec<SpellCategoriesRow>,
    pub spell_class_options: Vec<SpellClassOptionsRow>,
    pub spell_cooldowns: Vec<SpellCooldownsRow>,
    pub spell_empowers: Vec<SpellEmpowerRow>,
    pub spell_empower_stages: Vec<SpellEmpowerStageRow>,
    pub spell_equipped_items: Vec<SpellEquippedItemsRow>,
    pub spell_interrupts: Vec<SpellInterruptsRow>,
    pub spell_labels: Vec<SpellLabelRow>,
    pub spell_levels: Vec<SpellLevelsRow>,
    pub spell_powers: Vec<SpellPowerRow>,
    pub spell_power_difficulties: Vec<SpellPowerDifficultyRow>,
    pub spell_reagents: Vec<SpellReagentsRow>,
    pub spell_reagents_currencies: Vec<SpellReagentsCurrencyRow>,
    pub spell_scaling: Vec<SpellScalingRow>,
    pub spell_shapeshifts: Vec<SpellShapeshiftRow>,
    pub spell_target_restrictions: Vec<SpellTargetRestrictionsRow>,
    pub spell_totems: Vec<SpellTotemsRow>,
    pub spell_x_spell_visuals: Vec<SpellXSpellVisualRow>,
    pub difficulties: Vec<DifficultyRow>,
    pub spell_cast_times: Vec<SpellCastTimesRow>,
    pub spell_durations: Vec<SpellDurationRow>,
    pub spell_ranges: Vec<SpellRangeRow>,
    pub spell_radii: Vec<SpellRadiusRow>,
    pub spell_procs_per_minute: Vec<SpellProcsPerMinuteRow>,
    pub spell_procs_per_minute_mods: Vec<SpellProcsPerMinuteModRow>,
    pub spell_learn_spells: Vec<SpellLearnSpellRow>,
    pub spell_shapeshift_forms: Vec<SpellShapeshiftFormRow>,
    pub summon_properties: Vec<SummonPropertiesRow>,
    pub battle_pet_species: Vec<BattlePetSpeciesRow>,
    pub spell_category_definitions: Vec<SpellCategoryRow>,
    pub talents: Vec<TalentRow>,
    pub spell_item_enchantments: Vec<SpellItemEnchantmentRow>,
    pub spell_visuals: Vec<SpellVisualRow>,
    pub spell_visual_missiles: Vec<SpellVisualMissileRow>,
    pub spell_visual_effect_names: Vec<SpellVisualEffectNameRow>,
    pub liquid_types: Vec<LiquidTypeRow>,
    pub expected_stats: Vec<ExpectedStatRow>,
    pub expected_stat_mods: Vec<ExpectedStatModRow>,
    pub content_tunings: Vec<ContentTuningRow>,
    pub content_tuning_x_expected: Vec<ContentTuningXExpectedRow>,
    pub rand_prop_points: Vec<RandPropPointsRow>,
    pub mythic_plus_seasons: Vec<MythicPlusSeasonRow>,
    pub unit_conditions: Vec<UnitConditionRow>,
    /// Source MAX(ID)+1 query, issued separately after each nonempty
    /// RandPropPoints row result. None for an unqueried empty/synthetic batch.
    /// The uint64 SQL result is narrowed to uint32 as in DB2DatabaseLoader.
    pub rand_prop_points_sql_index_size: Option<u32>,
}

pub struct SpellOverlays {
    pub official: SpellRows,
    pub custom: SpellRows,
}
