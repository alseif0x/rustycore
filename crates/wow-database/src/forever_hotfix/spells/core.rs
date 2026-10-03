//! Exact source SQL column order; no string loss or numeric width projection.
use super::{LoadError, SpellRow};
use crate::SqlResult;
use wow_persistence::forever::spells::{
    SpellAuraOptionsRow, SpellAuraRestrictionsRow, SpellCastingRequirementsRow, SpellCategoriesRow,
    SpellClassOptionsRow, SpellCooldownsRow, SpellEffectRow, SpellEquippedItemsRow,
    SpellInterruptsRow, SpellLabelRow, SpellLevelsRow, SpellMiscRow, SpellNameRow,
};

pub(super) fn spell_name(result: &SqlResult) -> Result<SpellNameRow, LoadError> {
    let mut r = SpellRow::new(result, 2)?;
    let row = SpellNameRow {
        id: r.read()?,
        name: r.text()?,
    };
    r.finish()?;
    Ok(row)
}

pub(super) fn spell_effect(result: &SqlResult) -> Result<SpellEffectRow, LoadError> {
    let mut r = SpellRow::new(result, 37)?;
    let row = SpellEffectRow {
        id: r.read()?,
        effect_aura: r.read()?,
        difficulty_id: r.read()?,
        effect_index: r.read()?,
        effect: r.read()?,
        effect_amplitude: r.read()?,
        effect_attributes: r.read()?,
        effect_aura_period: r.read()?,
        effect_bonus_coefficient: r.read()?,
        effect_chain_amplitude: r.read()?,
        effect_chain_targets: r.read()?,
        effect_item_type: r.read()?,
        effect_mechanic: r.read()?,
        effect_points_per_resource: r.read()?,
        effect_pos_facing: r.read()?,
        effect_real_points_per_level: r.read()?,
        effect_trigger_spell: r.read()?,
        bonus_coefficient_from_ap: r.read()?,
        pvp_multiplier: r.read()?,
        coefficient: r.read()?,
        variance: r.read()?,
        resource_coefficient: r.read()?,
        group_size_base_points_coefficient: r.read()?,
        effect_base_points: r.read()?,
        scaling_class: r.read()?,
        target_node_graph: r.read()?,
        effect_misc_value: r.array()?,
        effect_radius_index: r.array()?,
        effect_spell_class_mask: r.array()?,
        implicit_target: r.array()?,
        spell_id: r.read()?,
    };
    r.finish()?;
    Ok(row)
}

pub(super) fn spell_misc(result: &SqlResult) -> Result<SpellMiscRow, LoadError> {
    let mut r = SpellRow::new(result, 34)?;
    let row = SpellMiscRow {
        id: r.read()?,
        attributes: r.array()?,
        difficulty_id: r.read()?,
        casting_time_index: r.read()?,
        duration_index: r.read()?,
        pv_p_duration_index: r.read()?,
        range_index: r.read()?,
        school_mask: r.read()?,
        speed: r.read()?,
        launch_delay: r.read()?,
        min_duration: r.read()?,
        spell_icon_file_data_id: r.read()?,
        active_icon_file_data_id: r.read()?,
        content_tuning_id: r.read()?,
        show_future_spell_player_condition_id: r.read()?,
        spell_visual_script: r.read()?,
        active_spell_visual_script: r.read()?,
        spell_id: r.read()?,
    };
    r.finish()?;
    Ok(row)
}

pub(super) fn spell_aura_options(result: &SqlResult) -> Result<SpellAuraOptionsRow, LoadError> {
    let mut r = SpellRow::new(result, 10)?;
    let row = SpellAuraOptionsRow {
        id: r.read()?,
        difficulty_id: r.read()?,
        cumulative_aura: r.read()?,
        proc_category_recovery: r.read()?,
        proc_chance: r.read()?,
        proc_charges: r.read()?,
        spell_procs_per_minute_id: r.read()?,
        proc_type_mask: r.array()?,
        spell_id: r.read()?,
    };
    r.finish()?;
    Ok(row)
}

pub(super) fn spell_aura_restrictions(
    result: &SqlResult,
) -> Result<SpellAuraRestrictionsRow, LoadError> {
    let mut r = SpellRow::new(result, 15)?;
    let row = SpellAuraRestrictionsRow {
        id: r.read()?,
        difficulty_id: r.read()?,
        caster_aura_state: r.read()?,
        target_aura_state: r.read()?,
        exclude_caster_aura_state: r.read()?,
        exclude_target_aura_state: r.read()?,
        caster_aura_spell: r.read()?,
        target_aura_spell: r.read()?,
        exclude_caster_aura_spell: r.read()?,
        exclude_target_aura_spell: r.read()?,
        caster_aura_type: r.read()?,
        target_aura_type: r.read()?,
        exclude_caster_aura_type: r.read()?,
        exclude_target_aura_type: r.read()?,
        spell_id: r.read()?,
    };
    r.finish()?;
    Ok(row)
}

pub(super) fn spell_casting_requirements(
    result: &SqlResult,
) -> Result<SpellCastingRequirementsRow, LoadError> {
    let mut r = SpellRow::new(result, 8)?;
    let row = SpellCastingRequirementsRow {
        id: r.read()?,
        spell_id: r.read()?,
        facing_caster_flags: r.read()?,
        min_faction_id: r.read()?,
        min_reputation: r.read()?,
        required_areas_id: r.read()?,
        required_aura_vision: r.read()?,
        requires_spell_focus: r.read()?,
    };
    r.finish()?;
    Ok(row)
}

pub(super) fn spell_categories(result: &SqlResult) -> Result<SpellCategoriesRow, LoadError> {
    let mut r = SpellRow::new(result, 11)?;
    let row = SpellCategoriesRow {
        id: r.read()?,
        difficulty_id: r.read()?,
        category: r.read()?,
        defense_type: r.read()?,
        diminish_type: r.read()?,
        dispel_type: r.read()?,
        mechanic: r.read()?,
        prevention_type: r.read()?,
        start_recovery_category: r.read()?,
        charge_category: r.read()?,
        spell_id: r.read()?,
    };
    r.finish()?;
    Ok(row)
}

pub(super) fn spell_class_options(result: &SqlResult) -> Result<SpellClassOptionsRow, LoadError> {
    let mut r = SpellRow::new(result, 8)?;
    let row = SpellClassOptionsRow {
        id: r.read()?,
        spell_id: r.read()?,
        modal_next_spell: r.read()?,
        spell_class_set: r.read()?,
        spell_class_mask: r.array()?,
    };
    r.finish()?;
    Ok(row)
}

pub(super) fn spell_cooldowns(result: &SqlResult) -> Result<SpellCooldownsRow, LoadError> {
    let mut r = SpellRow::new(result, 7)?;
    let row = SpellCooldownsRow {
        id: r.read()?,
        difficulty_id: r.read()?,
        category_recovery_time: r.read()?,
        recovery_time: r.read()?,
        start_recovery_time: r.read()?,
        aura_spell_id: r.read()?,
        spell_id: r.read()?,
    };
    r.finish()?;
    Ok(row)
}

pub(super) fn spell_equipped_items(result: &SqlResult) -> Result<SpellEquippedItemsRow, LoadError> {
    let mut r = SpellRow::new(result, 5)?;
    let row = SpellEquippedItemsRow {
        id: r.read()?,
        spell_id: r.read()?,
        equipped_item_class: r.read()?,
        equipped_item_inv_types: r.read()?,
        equipped_item_subclass: r.read()?,
    };
    r.finish()?;
    Ok(row)
}

pub(super) fn spell_interrupts(result: &SqlResult) -> Result<SpellInterruptsRow, LoadError> {
    let mut r = SpellRow::new(result, 8)?;
    let row = SpellInterruptsRow {
        id: r.read()?,
        difficulty_id: r.read()?,
        interrupt_flags: r.read()?,
        aura_interrupt_flags: r.array()?,
        channel_interrupt_flags: r.array()?,
        spell_id: r.read()?,
    };
    r.finish()?;
    Ok(row)
}

pub(super) fn spell_label(result: &SqlResult) -> Result<SpellLabelRow, LoadError> {
    let mut r = SpellRow::new(result, 3)?;
    let row = SpellLabelRow {
        id: r.read()?,
        label_id: r.read()?,
        spell_id: r.read()?,
    };
    r.finish()?;
    Ok(row)
}

pub(super) fn spell_levels(result: &SqlResult) -> Result<SpellLevelsRow, LoadError> {
    let mut r = SpellRow::new(result, 7)?;
    let row = SpellLevelsRow {
        id: r.read()?,
        difficulty_id: r.read()?,
        max_level: r.read()?,
        max_passive_aura_level: r.read()?,
        base_level: r.read()?,
        spell_level: r.read()?,
        spell_id: r.read()?,
    };
    r.finish()?;
    Ok(row)
}
