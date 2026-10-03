//! One-time source-width SQL DTO conversion; no retained raw mirror.
use anyhow::Result;
use wow_data::forever_spells::{
    SpellEmpowerRecord, SpellEmpowerStageRecord, SpellPowerDifficultyRecord, SpellPowerRecord,
    SpellReagentsCurrencyRecord, SpellReagentsRecord, SpellScalingRecord, SpellShapeshiftRecord,
    SpellTargetRestrictionsRecord, SpellTotemsRecord, SpellXSpellVisualRecord,
};
use wow_persistence::forever::spells::{
    SpellEmpowerRow, SpellEmpowerStageRow, SpellPowerDifficultyRow, SpellPowerRow,
    SpellReagentsCurrencyRow, SpellReagentsRow, SpellScalingRow, SpellShapeshiftRow,
    SpellTargetRestrictionsRow, SpellTotemsRow, SpellXSpellVisualRow,
};

pub(super) fn spell_empower(row: SpellEmpowerRow) -> Result<SpellEmpowerRecord> {
    Ok(SpellEmpowerRecord {
        id: row.id,
        spell_id: row.spell_id,
        unused1000: row.unused1000,
    })
}

pub(super) fn spell_empower_stage(row: SpellEmpowerStageRow) -> Result<SpellEmpowerStageRecord> {
    Ok(SpellEmpowerStageRecord {
        id: row.id,
        stage: row.stage,
        duration_ms: row.duration_ms,
        spell_empower_id: row.spell_empower_id,
    })
}

pub(super) fn spell_power(row: SpellPowerRow) -> Result<SpellPowerRecord> {
    Ok(SpellPowerRecord {
        id: row.id,
        order_index: row.order_index,
        mana_cost: row.mana_cost,
        mana_cost_per_level: row.mana_cost_per_level,
        mana_per_second: row.mana_per_second,
        power_display_id: row.power_display_id,
        alt_power_bar_id: row.alt_power_bar_id,
        power_cost_pct: row.power_cost_pct,
        power_cost_max_pct: row.power_cost_max_pct,
        optional_cost_pct: row.optional_cost_pct,
        power_pct_per_second: row.power_pct_per_second,
        power_type: row.power_type,
        required_aura_spell_id: row.required_aura_spell_id,
        optional_cost: row.optional_cost,
        spell_id: row.spell_id,
    })
}

pub(super) fn spell_power_difficulty(
    row: SpellPowerDifficultyRow,
) -> Result<SpellPowerDifficultyRecord> {
    Ok(SpellPowerDifficultyRecord {
        id: row.id,
        difficulty_id: row.difficulty_id,
        order_index: row.order_index,
    })
}

pub(super) fn spell_reagents(row: SpellReagentsRow) -> Result<SpellReagentsRecord> {
    Ok(SpellReagentsRecord {
        id: row.id,
        spell_id: row.spell_id,
        reagent: row.reagent,
        reagent_count: row.reagent_count,
        reagent_recraft_count: row.reagent_recraft_count,
        reagent_source: row.reagent_source,
    })
}

pub(super) fn spell_reagents_currency(
    row: SpellReagentsCurrencyRow,
) -> Result<SpellReagentsCurrencyRecord> {
    Ok(SpellReagentsCurrencyRecord {
        id: row.id,
        spell_id: row.spell_id,
        currency_types_id: row.currency_types_id,
        currency_count: row.currency_count,
        override_recraft_currency_count: row.override_recraft_currency_count,
        order_source: row.order_source,
    })
}

pub(super) fn spell_scaling(row: SpellScalingRow) -> Result<SpellScalingRecord> {
    Ok(SpellScalingRecord {
        id: row.id,
        spell_id: row.spell_id,
        min_scaling_level: row.min_scaling_level,
        max_scaling_level: row.max_scaling_level,
    })
}

pub(super) fn spell_shapeshift(row: SpellShapeshiftRow) -> Result<SpellShapeshiftRecord> {
    Ok(SpellShapeshiftRecord {
        id: row.id,
        spell_id: row.spell_id,
        stance_bar_order: row.stance_bar_order,
        shapeshift_exclude: row.shapeshift_exclude,
        shapeshift_mask: row.shapeshift_mask,
    })
}

pub(super) fn spell_target_restrictions(
    row: SpellTargetRestrictionsRow,
) -> Result<SpellTargetRestrictionsRecord> {
    Ok(SpellTargetRestrictionsRecord {
        id: row.id,
        difficulty_id: row.difficulty_id,
        cone_degrees: row.cone_degrees,
        max_targets: row.max_targets,
        max_target_level: row.max_target_level,
        target_creature_type: row.target_creature_type,
        targets: row.targets,
        width: row.width,
        spell_id: row.spell_id,
    })
}

pub(super) fn spell_totems(row: SpellTotemsRow) -> Result<SpellTotemsRecord> {
    Ok(SpellTotemsRecord {
        id: row.id,
        spell_id: row.spell_id,
        required_totem_category_id: row.required_totem_category_id,
        totem: row.totem,
    })
}

pub(super) fn spell_x_spell_visual(row: SpellXSpellVisualRow) -> Result<SpellXSpellVisualRecord> {
    Ok(SpellXSpellVisualRecord {
        id: row.id,
        difficulty_id: row.difficulty_id,
        spell_visual_id: row.spell_visual_id,
        probability: row.probability,
        flags: row.flags,
        priority: row.priority,
        spell_icon_file_id: row.spell_icon_file_id,
        active_icon_file_id: row.active_icon_file_id,
        viewer_unit_condition_id: row.viewer_unit_condition_id,
        viewer_player_condition_id: row.viewer_player_condition_id,
        caster_unit_condition_id: row.caster_unit_condition_id,
        caster_player_condition_id: row.caster_player_condition_id,
        spell_id: row.spell_id,
    })
}
