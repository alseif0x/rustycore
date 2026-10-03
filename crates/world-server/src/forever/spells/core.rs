//! One-time source-width SQL DTO conversion; no retained raw mirror.
use anyhow::Result;
use wow_data::forever_spells::{
    SpellAuraOptionsRecord, SpellAuraRestrictionsRecord, SpellCastingRequirementsRecord,
    SpellCategoriesRecord, SpellClassOptionsRecord, SpellCooldownsRecord, SpellEffectRecord,
    SpellEquippedItemsRecord, SpellInterruptsRecord, SpellLabelRecord, SpellLevelsRecord,
    SpellMiscRecord, SpellNameRecord, SpellText,
};
use wow_persistence::forever::spells::{
    SpellAuraOptionsRow, SpellAuraRestrictionsRow, SpellCastingRequirementsRow, SpellCategoriesRow,
    SpellClassOptionsRow, SpellCooldownsRow, SpellEffectRow, SpellEquippedItemsRow,
    SpellInterruptsRow, SpellLabelRow, SpellLevelsRow, SpellMiscRow, SpellNameRow,
};

pub(super) fn spell_name(row: SpellNameRow) -> Result<SpellNameRecord> {
    Ok(SpellNameRecord {
        id: row.id,
        name: SpellText::from_locale(0, row.name)?,
    })
}

pub(super) fn spell_effect(row: SpellEffectRow) -> Result<SpellEffectRecord> {
    Ok(SpellEffectRecord {
        id: row.id,
        effect_aura: row.effect_aura,
        difficulty_id: row.difficulty_id,
        effect_index: row.effect_index,
        effect: row.effect,
        effect_amplitude: row.effect_amplitude,
        effect_attributes: row.effect_attributes,
        effect_aura_period: row.effect_aura_period,
        effect_bonus_coefficient: row.effect_bonus_coefficient,
        effect_chain_amplitude: row.effect_chain_amplitude,
        effect_chain_targets: row.effect_chain_targets,
        effect_item_type: row.effect_item_type,
        effect_mechanic: row.effect_mechanic,
        effect_points_per_resource: row.effect_points_per_resource,
        effect_pos_facing: row.effect_pos_facing,
        effect_real_points_per_level: row.effect_real_points_per_level,
        effect_trigger_spell: row.effect_trigger_spell,
        bonus_coefficient_from_ap: row.bonus_coefficient_from_ap,
        pvp_multiplier: row.pvp_multiplier,
        coefficient: row.coefficient,
        variance: row.variance,
        resource_coefficient: row.resource_coefficient,
        group_size_base_points_coefficient: row.group_size_base_points_coefficient,
        effect_base_points: row.effect_base_points,
        scaling_class: row.scaling_class,
        target_node_graph: row.target_node_graph,
        effect_misc_value: row.effect_misc_value,
        effect_radius_index: row.effect_radius_index,
        effect_spell_class_mask: row.effect_spell_class_mask.map(|word| word as u32),
        implicit_target: row.implicit_target,
        spell_id: row.spell_id,
    })
}

pub(super) fn spell_misc(row: SpellMiscRow) -> Result<SpellMiscRecord> {
    Ok(SpellMiscRecord {
        id: row.id,
        attributes: row.attributes,
        difficulty_id: row.difficulty_id,
        casting_time_index: row.casting_time_index,
        duration_index: row.duration_index,
        pv_p_duration_index: row.pv_p_duration_index,
        range_index: row.range_index,
        school_mask: row.school_mask,
        speed: row.speed,
        launch_delay: row.launch_delay,
        min_duration: row.min_duration,
        spell_icon_file_data_id: row.spell_icon_file_data_id,
        active_icon_file_data_id: row.active_icon_file_data_id,
        content_tuning_id: row.content_tuning_id,
        show_future_spell_player_condition_id: row.show_future_spell_player_condition_id,
        spell_visual_script: row.spell_visual_script,
        active_spell_visual_script: row.active_spell_visual_script,
        spell_id: row.spell_id,
    })
}

pub(super) fn spell_aura_options(row: SpellAuraOptionsRow) -> Result<SpellAuraOptionsRecord> {
    Ok(SpellAuraOptionsRecord {
        id: row.id,
        difficulty_id: row.difficulty_id,
        cumulative_aura: row.cumulative_aura,
        proc_category_recovery: row.proc_category_recovery,
        proc_chance: row.proc_chance,
        proc_charges: row.proc_charges,
        spell_procs_per_minute_id: row.spell_procs_per_minute_id,
        proc_type_mask: row.proc_type_mask,
        spell_id: row.spell_id,
    })
}

pub(super) fn spell_aura_restrictions(
    row: SpellAuraRestrictionsRow,
) -> Result<SpellAuraRestrictionsRecord> {
    Ok(SpellAuraRestrictionsRecord {
        id: row.id,
        difficulty_id: row.difficulty_id,
        caster_aura_state: row.caster_aura_state,
        target_aura_state: row.target_aura_state,
        exclude_caster_aura_state: row.exclude_caster_aura_state,
        exclude_target_aura_state: row.exclude_target_aura_state,
        caster_aura_spell: row.caster_aura_spell,
        target_aura_spell: row.target_aura_spell,
        exclude_caster_aura_spell: row.exclude_caster_aura_spell,
        exclude_target_aura_spell: row.exclude_target_aura_spell,
        caster_aura_type: row.caster_aura_type,
        target_aura_type: row.target_aura_type,
        exclude_caster_aura_type: row.exclude_caster_aura_type,
        exclude_target_aura_type: row.exclude_target_aura_type,
        spell_id: row.spell_id,
    })
}

pub(super) fn spell_casting_requirements(
    row: SpellCastingRequirementsRow,
) -> Result<SpellCastingRequirementsRecord> {
    Ok(SpellCastingRequirementsRecord {
        id: row.id,
        spell_id: row.spell_id,
        facing_caster_flags: row.facing_caster_flags,
        min_faction_id: row.min_faction_id,
        min_reputation: row.min_reputation,
        required_areas_id: row.required_areas_id,
        required_aura_vision: row.required_aura_vision,
        requires_spell_focus: row.requires_spell_focus,
    })
}

pub(super) fn spell_categories(row: SpellCategoriesRow) -> Result<SpellCategoriesRecord> {
    Ok(SpellCategoriesRecord {
        id: row.id,
        difficulty_id: row.difficulty_id,
        category: row.category,
        defense_type: row.defense_type,
        diminish_type: row.diminish_type,
        dispel_type: row.dispel_type,
        mechanic: row.mechanic,
        prevention_type: row.prevention_type,
        start_recovery_category: row.start_recovery_category,
        charge_category: row.charge_category,
        spell_id: row.spell_id,
    })
}

pub(super) fn spell_class_options(row: SpellClassOptionsRow) -> Result<SpellClassOptionsRecord> {
    Ok(SpellClassOptionsRecord {
        id: row.id,
        spell_id: row.spell_id,
        modal_next_spell: row.modal_next_spell,
        spell_class_set: row.spell_class_set,
        spell_class_mask: row.spell_class_mask.map(|word| word as u32),
    })
}

pub(super) fn spell_cooldowns(row: SpellCooldownsRow) -> Result<SpellCooldownsRecord> {
    Ok(SpellCooldownsRecord {
        id: row.id,
        difficulty_id: row.difficulty_id,
        category_recovery_time: row.category_recovery_time,
        recovery_time: row.recovery_time,
        start_recovery_time: row.start_recovery_time,
        aura_spell_id: row.aura_spell_id,
        spell_id: row.spell_id,
    })
}

pub(super) fn spell_equipped_items(row: SpellEquippedItemsRow) -> Result<SpellEquippedItemsRecord> {
    Ok(SpellEquippedItemsRecord {
        id: row.id,
        spell_id: row.spell_id,
        equipped_item_class: row.equipped_item_class,
        equipped_item_inv_types: row.equipped_item_inv_types,
        equipped_item_subclass: row.equipped_item_subclass,
    })
}

pub(super) fn spell_interrupts(row: SpellInterruptsRow) -> Result<SpellInterruptsRecord> {
    Ok(SpellInterruptsRecord {
        id: row.id,
        difficulty_id: row.difficulty_id,
        interrupt_flags: row.interrupt_flags,
        aura_interrupt_flags: row.aura_interrupt_flags,
        channel_interrupt_flags: row.channel_interrupt_flags,
        spell_id: row.spell_id,
    })
}

pub(super) fn spell_label(row: SpellLabelRow) -> Result<SpellLabelRecord> {
    Ok(SpellLabelRecord {
        id: row.id,
        label_id: row.label_id,
        spell_id: row.spell_id,
    })
}

pub(super) fn spell_levels(row: SpellLevelsRow) -> Result<SpellLevelsRecord> {
    Ok(SpellLevelsRecord {
        id: row.id,
        difficulty_id: row.difficulty_id,
        max_level: row.max_level,
        max_passive_aura_level: row.max_passive_aura_level,
        base_level: row.base_level,
        spell_level: row.spell_level,
        spell_id: row.spell_id,
    })
}
