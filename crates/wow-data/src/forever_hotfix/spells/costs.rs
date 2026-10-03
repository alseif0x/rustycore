//! 02245dcd DB2StorageBase::WriteRecord, source costs spell metadata.
use crate::forever_spells::{
    SpellEmpowerRecord, SpellEmpowerStageRecord, SpellPowerDifficultyRecord, SpellPowerRecord,
    SpellReagentsCurrencyRecord, SpellReagentsRecord, SpellScalingRecord, SpellShapeshiftRecord,
    SpellTargetRestrictionsRecord, SpellTotemsRecord, SpellXSpellVisualRecord,
};

pub(super) fn spell_empower(row: &SpellEmpowerRecord, _locale: u8) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&row.id.to_le_bytes());
    bytes.extend_from_slice(&row.spell_id.to_le_bytes());
    bytes.extend_from_slice(&row.unused1000.to_le_bytes());
    bytes
}

pub(super) fn spell_empower_stage(row: &SpellEmpowerStageRecord, _locale: u8) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&row.stage.to_le_bytes());
    bytes.extend_from_slice(&row.duration_ms.to_le_bytes());
    bytes.extend_from_slice(&row.spell_empower_id.to_le_bytes());
    bytes
}

pub(super) fn spell_power(row: &SpellPowerRecord, _locale: u8) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&row.id.to_le_bytes());
    bytes.extend_from_slice(&row.order_index.to_le_bytes());
    bytes.extend_from_slice(&row.mana_cost.to_le_bytes());
    bytes.extend_from_slice(&row.mana_cost_per_level.to_le_bytes());
    bytes.extend_from_slice(&row.mana_per_second.to_le_bytes());
    bytes.extend_from_slice(&row.power_display_id.to_le_bytes());
    bytes.extend_from_slice(&row.alt_power_bar_id.to_le_bytes());
    bytes.extend_from_slice(&row.power_cost_pct.to_le_bytes());
    bytes.extend_from_slice(&row.power_cost_max_pct.to_le_bytes());
    bytes.extend_from_slice(&row.optional_cost_pct.to_le_bytes());
    bytes.extend_from_slice(&row.power_pct_per_second.to_le_bytes());
    bytes.extend_from_slice(&row.power_type.to_le_bytes());
    bytes.extend_from_slice(&row.required_aura_spell_id.to_le_bytes());
    bytes.extend_from_slice(&row.optional_cost.to_le_bytes());
    bytes.extend_from_slice(&row.spell_id.to_le_bytes());
    bytes
}

pub(super) fn spell_power_difficulty(row: &SpellPowerDifficultyRecord, _locale: u8) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&row.difficulty_id.to_le_bytes());
    bytes.extend_from_slice(&row.order_index.to_le_bytes());
    bytes
}

pub(super) fn spell_reagents(row: &SpellReagentsRecord, _locale: u8) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&row.spell_id.to_le_bytes());
    for value in row.reagent {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    for value in row.reagent_count {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    for value in row.reagent_recraft_count {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    for value in row.reagent_source {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    bytes
}

pub(super) fn spell_reagents_currency(row: &SpellReagentsCurrencyRecord, _locale: u8) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&row.spell_id.to_le_bytes());
    bytes.extend_from_slice(&row.currency_types_id.to_le_bytes());
    bytes.extend_from_slice(&row.currency_count.to_le_bytes());
    bytes.extend_from_slice(&row.override_recraft_currency_count.to_le_bytes());
    bytes.extend_from_slice(&row.order_source.to_le_bytes());
    bytes
}

pub(super) fn spell_scaling(row: &SpellScalingRecord, _locale: u8) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&row.spell_id.to_le_bytes());
    bytes.extend_from_slice(&row.min_scaling_level.to_le_bytes());
    bytes.extend_from_slice(&row.max_scaling_level.to_le_bytes());
    bytes
}

pub(super) fn spell_shapeshift(row: &SpellShapeshiftRecord, _locale: u8) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&row.spell_id.to_le_bytes());
    bytes.extend_from_slice(&row.stance_bar_order.to_le_bytes());
    for value in row.shapeshift_exclude {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    for value in row.shapeshift_mask {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    bytes
}

pub(super) fn spell_target_restrictions(
    row: &SpellTargetRestrictionsRecord,
    _locale: u8,
) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&row.difficulty_id.to_le_bytes());
    bytes.extend_from_slice(&row.cone_degrees.to_le_bytes());
    bytes.extend_from_slice(&row.max_targets.to_le_bytes());
    bytes.extend_from_slice(&row.max_target_level.to_le_bytes());
    bytes.extend_from_slice(&row.target_creature_type.to_le_bytes());
    bytes.extend_from_slice(&row.targets.to_le_bytes());
    bytes.extend_from_slice(&row.width.to_le_bytes());
    bytes.extend_from_slice(&row.spell_id.to_le_bytes());
    bytes
}

pub(super) fn spell_totems(row: &SpellTotemsRecord, _locale: u8) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&row.spell_id.to_le_bytes());
    for value in row.required_totem_category_id {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    for value in row.totem {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    bytes
}

pub(super) fn spell_x_spell_visual(row: &SpellXSpellVisualRecord, _locale: u8) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&row.id.to_le_bytes());
    bytes.extend_from_slice(&row.difficulty_id.to_le_bytes());
    bytes.extend_from_slice(&row.spell_visual_id.to_le_bytes());
    bytes.extend_from_slice(&row.probability.to_le_bytes());
    bytes.extend_from_slice(&row.flags.to_le_bytes());
    bytes.extend_from_slice(&row.priority.to_le_bytes());
    bytes.extend_from_slice(&row.spell_icon_file_id.to_le_bytes());
    bytes.extend_from_slice(&row.active_icon_file_id.to_le_bytes());
    bytes.extend_from_slice(&row.viewer_unit_condition_id.to_le_bytes());
    bytes.extend_from_slice(&row.viewer_player_condition_id.to_le_bytes());
    bytes.extend_from_slice(&row.caster_unit_condition_id.to_le_bytes());
    bytes.extend_from_slice(&row.caster_player_condition_id.to_le_bytes());
    bytes.extend_from_slice(&row.spell_id.to_le_bytes());
    bytes
}
