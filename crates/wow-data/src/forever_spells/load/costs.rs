use super::super::*;
use crate::wdc4::creation::CreationDb2;
use anyhow::Result;

pub(super) fn spell_empower(table: &CreationDb2, id: u32) -> Result<SpellEmpowerRecord> {
    Ok(SpellEmpowerRecord {
        id: id,
        spell_id: table.bits(id, 1, 0)? as i32,
        unused1000: table.bits(id, 2, 0)? as i32,
    })
}

pub(super) fn spell_empower_stage(table: &CreationDb2, id: u32) -> Result<SpellEmpowerStageRecord> {
    Ok(SpellEmpowerStageRecord {
        id: id,
        stage: table.bits(id, 0, 0)? as i32,
        duration_ms: table.bits(id, 1, 0)? as i32,
        spell_empower_id: table.bits(id, 2, 0)?,
    })
}

pub(super) fn spell_power(table: &CreationDb2, id: u32) -> Result<SpellPowerRecord> {
    Ok(SpellPowerRecord {
        id: id,
        order_index: table.bits(id, 1, 0)? as u8,
        mana_cost: table.bits(id, 2, 0)? as i32,
        mana_cost_per_level: table.bits(id, 3, 0)? as i32,
        mana_per_second: table.bits(id, 4, 0)? as i32,
        power_display_id: table.bits(id, 5, 0)?,
        alt_power_bar_id: table.bits(id, 6, 0)? as i32,
        power_cost_pct: f32::from_bits(table.bits(id, 7, 0)?),
        power_cost_max_pct: f32::from_bits(table.bits(id, 8, 0)?),
        optional_cost_pct: f32::from_bits(table.bits(id, 9, 0)?),
        power_pct_per_second: f32::from_bits(table.bits(id, 10, 0)?),
        power_type: table.bits(id, 11, 0)? as i8,
        required_aura_spell_id: table.bits(id, 12, 0)? as i32,
        optional_cost: table.bits(id, 13, 0)?,
        spell_id: table.bits(id, 14, 0)?,
    })
}

pub(super) fn spell_power_difficulty(
    table: &CreationDb2,
    id: u32,
) -> Result<SpellPowerDifficultyRecord> {
    Ok(SpellPowerDifficultyRecord {
        id: id,
        difficulty_id: table.bits(id, 0, 0)? as i16,
        order_index: table.bits(id, 1, 0)? as u8,
    })
}

pub(super) fn spell_reagents(table: &CreationDb2, id: u32) -> Result<SpellReagentsRecord> {
    Ok(SpellReagentsRecord {
        id: id,
        spell_id: table.bits(id, 0, 0)? as i32,
        reagent: [
            table.bits(id, 1, 0)? as i32,
            table.bits(id, 1, 1)? as i32,
            table.bits(id, 1, 2)? as i32,
            table.bits(id, 1, 3)? as i32,
            table.bits(id, 1, 4)? as i32,
            table.bits(id, 1, 5)? as i32,
            table.bits(id, 1, 6)? as i32,
            table.bits(id, 1, 7)? as i32,
        ],
        reagent_count: [
            table.bits(id, 2, 0)? as i16,
            table.bits(id, 2, 1)? as i16,
            table.bits(id, 2, 2)? as i16,
            table.bits(id, 2, 3)? as i16,
            table.bits(id, 2, 4)? as i16,
            table.bits(id, 2, 5)? as i16,
            table.bits(id, 2, 6)? as i16,
            table.bits(id, 2, 7)? as i16,
        ],
        reagent_recraft_count: [
            table.bits(id, 3, 0)? as i16,
            table.bits(id, 3, 1)? as i16,
            table.bits(id, 3, 2)? as i16,
            table.bits(id, 3, 3)? as i16,
            table.bits(id, 3, 4)? as i16,
            table.bits(id, 3, 5)? as i16,
            table.bits(id, 3, 6)? as i16,
            table.bits(id, 3, 7)? as i16,
        ],
        reagent_source: [
            table.bits(id, 4, 0)? as u8,
            table.bits(id, 4, 1)? as u8,
            table.bits(id, 4, 2)? as u8,
            table.bits(id, 4, 3)? as u8,
            table.bits(id, 4, 4)? as u8,
            table.bits(id, 4, 5)? as u8,
            table.bits(id, 4, 6)? as u8,
            table.bits(id, 4, 7)? as u8,
        ],
    })
}

pub(super) fn spell_reagents_currency(
    table: &CreationDb2,
    id: u32,
) -> Result<SpellReagentsCurrencyRecord> {
    Ok(SpellReagentsCurrencyRecord {
        id: id,
        spell_id: table.bits(id, 0, 0)?,
        currency_types_id: table.bits(id, 1, 0)? as i32,
        currency_count: table.bits(id, 2, 0)? as i32,
        override_recraft_currency_count: table.bits(id, 3, 0)? as i32,
        order_source: table.bits(id, 4, 0)? as u8,
    })
}

pub(super) fn spell_scaling(table: &CreationDb2, id: u32) -> Result<SpellScalingRecord> {
    Ok(SpellScalingRecord {
        id: id,
        spell_id: table.bits(id, 0, 0)? as i32,
        min_scaling_level: table.bits(id, 1, 0)?,
        max_scaling_level: table.bits(id, 2, 0)?,
    })
}

pub(super) fn spell_shapeshift(table: &CreationDb2, id: u32) -> Result<SpellShapeshiftRecord> {
    Ok(SpellShapeshiftRecord {
        id: id,
        spell_id: table.bits(id, 0, 0)? as i32,
        stance_bar_order: table.bits(id, 1, 0)? as i8,
        shapeshift_exclude: [table.bits(id, 2, 0)? as i32, table.bits(id, 2, 1)? as i32],
        shapeshift_mask: [table.bits(id, 3, 0)? as i32, table.bits(id, 3, 1)? as i32],
    })
}

pub(super) fn spell_target_restrictions(
    table: &CreationDb2,
    id: u32,
) -> Result<SpellTargetRestrictionsRecord> {
    Ok(SpellTargetRestrictionsRecord {
        id: id,
        difficulty_id: table.bits(id, 0, 0)? as i16,
        cone_degrees: f32::from_bits(table.bits(id, 1, 0)?),
        max_targets: table.bits(id, 2, 0)? as u8,
        max_target_level: table.bits(id, 3, 0)?,
        target_creature_type: table.bits(id, 4, 0)? as i16,
        targets: table.bits(id, 5, 0)? as i32,
        width: f32::from_bits(table.bits(id, 6, 0)?),
        spell_id: table.bits(id, 7, 0)?,
    })
}

pub(super) fn spell_totems(table: &CreationDb2, id: u32) -> Result<SpellTotemsRecord> {
    Ok(SpellTotemsRecord {
        id: id,
        spell_id: table.bits(id, 0, 0)? as i32,
        required_totem_category_id: [table.bits(id, 1, 0)? as u16, table.bits(id, 1, 1)? as u16],
        totem: [table.bits(id, 2, 0)? as i32, table.bits(id, 2, 1)? as i32],
    })
}

pub(super) fn spell_x_spell_visual(
    table: &CreationDb2,
    id: u32,
) -> Result<SpellXSpellVisualRecord> {
    Ok(SpellXSpellVisualRecord {
        id: id,
        difficulty_id: table.bits(id, 1, 0)? as i16,
        spell_visual_id: table.bits(id, 2, 0)?,
        probability: f32::from_bits(table.bits(id, 3, 0)?),
        flags: table.bits(id, 4, 0)? as i32,
        priority: table.bits(id, 5, 0)? as i32,
        spell_icon_file_id: table.bits(id, 6, 0)? as i32,
        active_icon_file_id: table.bits(id, 7, 0)? as i32,
        viewer_unit_condition_id: table.bits(id, 8, 0)? as u16,
        viewer_player_condition_id: table.bits(id, 9, 0)?,
        caster_unit_condition_id: table.bits(id, 10, 0)? as u16,
        caster_player_condition_id: table.bits(id, 11, 0)?,
        spell_id: table.bits(id, 12, 0)?,
    })
}
