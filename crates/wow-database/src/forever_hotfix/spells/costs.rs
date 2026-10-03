//! Exact source SQL column order; no string loss or numeric width projection.
use super::{LoadError, SpellRow};
use crate::SqlResult;
use wow_persistence::forever::spells::{
    SpellEmpowerRow, SpellEmpowerStageRow, SpellPowerDifficultyRow, SpellPowerRow,
    SpellReagentsCurrencyRow, SpellReagentsRow, SpellScalingRow, SpellShapeshiftRow,
    SpellTargetRestrictionsRow, SpellTotemsRow, SpellXSpellVisualRow,
};

pub(super) fn spell_empower(result: &SqlResult) -> Result<SpellEmpowerRow, LoadError> {
    let mut r = SpellRow::new(result, 3)?;
    let row = SpellEmpowerRow {
        id: r.read()?,
        spell_id: r.read()?,
        unused1000: r.read()?,
    };
    r.finish()?;
    Ok(row)
}

pub(super) fn spell_empower_stage(result: &SqlResult) -> Result<SpellEmpowerStageRow, LoadError> {
    let mut r = SpellRow::new(result, 4)?;
    let row = SpellEmpowerStageRow {
        id: r.read()?,
        stage: r.read()?,
        duration_ms: r.read()?,
        spell_empower_id: r.read()?,
    };
    r.finish()?;
    Ok(row)
}

pub(super) fn spell_power(result: &SqlResult) -> Result<SpellPowerRow, LoadError> {
    let mut r = SpellRow::new(result, 15)?;
    let row = SpellPowerRow {
        id: r.read()?,
        order_index: r.read()?,
        mana_cost: r.read()?,
        mana_cost_per_level: r.read()?,
        mana_per_second: r.read()?,
        power_display_id: r.read()?,
        alt_power_bar_id: r.read()?,
        power_cost_pct: r.read()?,
        power_cost_max_pct: r.read()?,
        optional_cost_pct: r.read()?,
        power_pct_per_second: r.read()?,
        power_type: r.read()?,
        required_aura_spell_id: r.read()?,
        optional_cost: r.read()?,
        spell_id: r.read()?,
    };
    r.finish()?;
    Ok(row)
}

pub(super) fn spell_power_difficulty(
    result: &SqlResult,
) -> Result<SpellPowerDifficultyRow, LoadError> {
    let mut r = SpellRow::new(result, 3)?;
    let row = SpellPowerDifficultyRow {
        id: r.read()?,
        difficulty_id: r.read()?,
        order_index: r.read()?,
    };
    r.finish()?;
    Ok(row)
}

pub(super) fn spell_reagents(result: &SqlResult) -> Result<SpellReagentsRow, LoadError> {
    let mut r = SpellRow::new(result, 34)?;
    let row = SpellReagentsRow {
        id: r.read()?,
        spell_id: r.read()?,
        reagent: r.array()?,
        reagent_count: r.array()?,
        reagent_recraft_count: r.array()?,
        reagent_source: r.array()?,
    };
    r.finish()?;
    Ok(row)
}

pub(super) fn spell_reagents_currency(
    result: &SqlResult,
) -> Result<SpellReagentsCurrencyRow, LoadError> {
    let mut r = SpellRow::new(result, 6)?;
    let row = SpellReagentsCurrencyRow {
        id: r.read()?,
        spell_id: r.read()?,
        currency_types_id: r.read()?,
        currency_count: r.read()?,
        override_recraft_currency_count: r.read()?,
        order_source: r.read()?,
    };
    r.finish()?;
    Ok(row)
}

pub(super) fn spell_scaling(result: &SqlResult) -> Result<SpellScalingRow, LoadError> {
    let mut r = SpellRow::new(result, 4)?;
    let row = SpellScalingRow {
        id: r.read()?,
        spell_id: r.read()?,
        min_scaling_level: r.read()?,
        max_scaling_level: r.read()?,
    };
    r.finish()?;
    Ok(row)
}

pub(super) fn spell_shapeshift(result: &SqlResult) -> Result<SpellShapeshiftRow, LoadError> {
    let mut r = SpellRow::new(result, 7)?;
    let row = SpellShapeshiftRow {
        id: r.read()?,
        spell_id: r.read()?,
        stance_bar_order: r.read()?,
        shapeshift_exclude: r.array()?,
        shapeshift_mask: r.array()?,
    };
    r.finish()?;
    Ok(row)
}

pub(super) fn spell_target_restrictions(
    result: &SqlResult,
) -> Result<SpellTargetRestrictionsRow, LoadError> {
    let mut r = SpellRow::new(result, 9)?;
    let row = SpellTargetRestrictionsRow {
        id: r.read()?,
        difficulty_id: r.read()?,
        cone_degrees: r.read()?,
        max_targets: r.read()?,
        max_target_level: r.read()?,
        target_creature_type: r.read()?,
        targets: r.read()?,
        width: r.read()?,
        spell_id: r.read()?,
    };
    r.finish()?;
    Ok(row)
}

pub(super) fn spell_totems(result: &SqlResult) -> Result<SpellTotemsRow, LoadError> {
    let mut r = SpellRow::new(result, 6)?;
    let row = SpellTotemsRow {
        id: r.read()?,
        spell_id: r.read()?,
        required_totem_category_id: r.array()?,
        totem: r.array()?,
    };
    r.finish()?;
    Ok(row)
}

pub(super) fn spell_x_spell_visual(result: &SqlResult) -> Result<SpellXSpellVisualRow, LoadError> {
    let mut r = SpellRow::new(result, 13)?;
    let row = SpellXSpellVisualRow {
        id: r.read()?,
        difficulty_id: r.read()?,
        spell_visual_id: r.read()?,
        probability: r.read()?,
        flags: r.read()?,
        priority: r.read()?,
        spell_icon_file_id: r.read()?,
        active_icon_file_id: r.read()?,
        viewer_unit_condition_id: r.read()?,
        viewer_player_condition_id: r.read()?,
        caster_unit_condition_id: r.read()?,
        caster_player_condition_id: r.read()?,
        spell_id: r.read()?,
    };
    r.finish()?;
    Ok(row)
}
