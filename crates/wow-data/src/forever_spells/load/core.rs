use super::super::*;
use crate::wdc4::creation::CreationDb2;
use anyhow::Result;

pub(super) fn spell_name(table: &CreationDb2, id: u32) -> Result<SpellNameRecord> {
    Ok(SpellNameRecord {
        id: id,
        name: SpellText::from_locale(6, table.spell_text(id, 0)?)?,
    })
}

pub(super) fn spell_effect(table: &CreationDb2, id: u32) -> Result<SpellEffectRecord> {
    Ok(SpellEffectRecord {
        id: id,
        effect_aura: table.bits(id, 0, 0)? as i16,
        difficulty_id: table.bits(id, 1, 0)? as i16,
        effect_index: table.bits(id, 2, 0)? as i32,
        effect: table.bits(id, 3, 0)?,
        effect_amplitude: f32::from_bits(table.bits(id, 4, 0)?),
        effect_attributes: table.bits(id, 5, 0)? as i32,
        effect_aura_period: table.bits(id, 6, 0)? as i32,
        effect_bonus_coefficient: f32::from_bits(table.bits(id, 7, 0)?),
        effect_chain_amplitude: f32::from_bits(table.bits(id, 8, 0)?),
        effect_chain_targets: table.bits(id, 9, 0)? as i32,
        effect_item_type: table.bits(id, 10, 0)? as i32,
        effect_mechanic: table.bits(id, 11, 0)? as i32,
        effect_points_per_resource: f32::from_bits(table.bits(id, 12, 0)?),
        effect_pos_facing: f32::from_bits(table.bits(id, 13, 0)?),
        effect_real_points_per_level: f32::from_bits(table.bits(id, 14, 0)?),
        effect_trigger_spell: table.bits(id, 15, 0)? as i32,
        bonus_coefficient_from_ap: f32::from_bits(table.bits(id, 16, 0)?),
        pvp_multiplier: f32::from_bits(table.bits(id, 17, 0)?),
        coefficient: f32::from_bits(table.bits(id, 18, 0)?),
        variance: f32::from_bits(table.bits(id, 19, 0)?),
        resource_coefficient: f32::from_bits(table.bits(id, 20, 0)?),
        group_size_base_points_coefficient: f32::from_bits(table.bits(id, 21, 0)?),
        effect_base_points: f32::from_bits(table.bits(id, 22, 0)?),
        scaling_class: table.bits(id, 23, 0)? as i32,
        target_node_graph: table.bits(id, 24, 0)? as i32,
        effect_misc_value: [table.bits(id, 25, 0)? as i32, table.bits(id, 25, 1)? as i32],
        effect_radius_index: [table.bits(id, 26, 0)?, table.bits(id, 26, 1)?],
        effect_spell_class_mask: [
            table.bits(id, 27, 0)?,
            table.bits(id, 27, 1)?,
            table.bits(id, 27, 2)?,
            table.bits(id, 27, 3)?,
        ],
        implicit_target: [table.bits(id, 28, 0)? as i16, table.bits(id, 28, 1)? as i16],
        spell_id: table.bits(id, 29, 0)?,
    })
}

pub(super) fn spell_misc(table: &CreationDb2, id: u32) -> Result<SpellMiscRecord> {
    Ok(SpellMiscRecord {
        id: id,
        attributes: [
            table.bits(id, 0, 0)? as i32,
            table.bits(id, 0, 1)? as i32,
            table.bits(id, 0, 2)? as i32,
            table.bits(id, 0, 3)? as i32,
            table.bits(id, 0, 4)? as i32,
            table.bits(id, 0, 5)? as i32,
            table.bits(id, 0, 6)? as i32,
            table.bits(id, 0, 7)? as i32,
            table.bits(id, 0, 8)? as i32,
            table.bits(id, 0, 9)? as i32,
            table.bits(id, 0, 10)? as i32,
            table.bits(id, 0, 11)? as i32,
            table.bits(id, 0, 12)? as i32,
            table.bits(id, 0, 13)? as i32,
            table.bits(id, 0, 14)? as i32,
            table.bits(id, 0, 15)? as i32,
            table.bits(id, 0, 16)? as i32,
        ],
        difficulty_id: table.bits(id, 1, 0)? as i16,
        casting_time_index: table.bits(id, 2, 0)? as u16,
        duration_index: table.bits(id, 3, 0)? as u16,
        pv_p_duration_index: table.bits(id, 4, 0)? as u16,
        range_index: table.bits(id, 5, 0)? as u16,
        school_mask: table.bits(id, 6, 0)? as u8,
        speed: f32::from_bits(table.bits(id, 7, 0)?),
        launch_delay: f32::from_bits(table.bits(id, 8, 0)?),
        min_duration: f32::from_bits(table.bits(id, 9, 0)?),
        spell_icon_file_data_id: table.bits(id, 10, 0)? as i32,
        active_icon_file_data_id: table.bits(id, 11, 0)? as i32,
        content_tuning_id: table.bits(id, 12, 0)? as i32,
        show_future_spell_player_condition_id: table.bits(id, 13, 0)? as i32,
        spell_visual_script: table.bits(id, 14, 0)? as i32,
        active_spell_visual_script: table.bits(id, 15, 0)? as i32,
        spell_id: table.bits(id, 16, 0)?,
    })
}

pub(super) fn spell_aura_options(table: &CreationDb2, id: u32) -> Result<SpellAuraOptionsRecord> {
    Ok(SpellAuraOptionsRecord {
        id: id,
        difficulty_id: table.bits(id, 0, 0)? as i16,
        cumulative_aura: table.bits(id, 1, 0)? as u16,
        proc_category_recovery: table.bits(id, 2, 0)? as i32,
        proc_chance: table.bits(id, 3, 0)? as u8,
        proc_charges: table.bits(id, 4, 0)? as i32,
        spell_procs_per_minute_id: table.bits(id, 5, 0)? as u16,
        proc_type_mask: [table.bits(id, 6, 0)? as i32, table.bits(id, 6, 1)? as i32],
        spell_id: table.bits(id, 7, 0)?,
    })
}

pub(super) fn spell_aura_restrictions(
    table: &CreationDb2,
    id: u32,
) -> Result<SpellAuraRestrictionsRecord> {
    Ok(SpellAuraRestrictionsRecord {
        id: id,
        difficulty_id: table.bits(id, 0, 0)? as i16,
        caster_aura_state: table.bits(id, 1, 0)? as i32,
        target_aura_state: table.bits(id, 2, 0)? as i32,
        exclude_caster_aura_state: table.bits(id, 3, 0)? as i32,
        exclude_target_aura_state: table.bits(id, 4, 0)? as i32,
        caster_aura_spell: table.bits(id, 5, 0)? as i32,
        target_aura_spell: table.bits(id, 6, 0)? as i32,
        exclude_caster_aura_spell: table.bits(id, 7, 0)? as i32,
        exclude_target_aura_spell: table.bits(id, 8, 0)? as i32,
        caster_aura_type: table.bits(id, 9, 0)? as i16,
        target_aura_type: table.bits(id, 10, 0)? as i16,
        exclude_caster_aura_type: table.bits(id, 11, 0)? as i16,
        exclude_target_aura_type: table.bits(id, 12, 0)? as i16,
        spell_id: table.bits(id, 13, 0)?,
    })
}

pub(super) fn spell_casting_requirements(
    table: &CreationDb2,
    id: u32,
) -> Result<SpellCastingRequirementsRecord> {
    Ok(SpellCastingRequirementsRecord {
        id: id,
        spell_id: table.bits(id, 0, 0)? as i32,
        facing_caster_flags: table.bits(id, 1, 0)? as i32,
        min_faction_id: table.bits(id, 2, 0)? as u16,
        min_reputation: table.bits(id, 3, 0)? as i32,
        required_areas_id: table.bits(id, 4, 0)? as u16,
        required_aura_vision: table.bits(id, 5, 0)? as u8,
        requires_spell_focus: table.bits(id, 6, 0)? as u16,
    })
}

pub(super) fn spell_categories(table: &CreationDb2, id: u32) -> Result<SpellCategoriesRecord> {
    Ok(SpellCategoriesRecord {
        id: id,
        difficulty_id: table.bits(id, 0, 0)? as i16,
        category: table.bits(id, 1, 0)? as i16,
        defense_type: table.bits(id, 2, 0)? as i8,
        diminish_type: table.bits(id, 3, 0)? as i32,
        dispel_type: table.bits(id, 4, 0)? as i8,
        mechanic: table.bits(id, 5, 0)? as i8,
        prevention_type: table.bits(id, 6, 0)? as i32,
        start_recovery_category: table.bits(id, 7, 0)? as i16,
        charge_category: table.bits(id, 8, 0)? as i16,
        spell_id: table.bits(id, 9, 0)?,
    })
}

pub(super) fn spell_class_options(table: &CreationDb2, id: u32) -> Result<SpellClassOptionsRecord> {
    Ok(SpellClassOptionsRecord {
        id: id,
        spell_id: table.bits(id, 0, 0)? as i32,
        modal_next_spell: table.bits(id, 1, 0)?,
        spell_class_set: table.bits(id, 2, 0)? as i32,
        spell_class_mask: [
            table.bits(id, 3, 0)?,
            table.bits(id, 3, 1)?,
            table.bits(id, 3, 2)?,
            table.bits(id, 3, 3)?,
        ],
    })
}

pub(super) fn spell_cooldowns(table: &CreationDb2, id: u32) -> Result<SpellCooldownsRecord> {
    Ok(SpellCooldownsRecord {
        id: id,
        difficulty_id: table.bits(id, 0, 0)? as i16,
        category_recovery_time: table.bits(id, 1, 0)? as i32,
        recovery_time: table.bits(id, 2, 0)? as i32,
        start_recovery_time: table.bits(id, 3, 0)? as i32,
        aura_spell_id: table.bits(id, 4, 0)? as i32,
        spell_id: table.bits(id, 5, 0)?,
    })
}

pub(super) fn spell_equipped_items(
    table: &CreationDb2,
    id: u32,
) -> Result<SpellEquippedItemsRecord> {
    Ok(SpellEquippedItemsRecord {
        id: id,
        spell_id: table.bits(id, 0, 0)? as i32,
        equipped_item_class: table.bits(id, 1, 0)? as i32,
        equipped_item_inv_types: table.bits(id, 2, 0)? as i32,
        equipped_item_subclass: table.bits(id, 3, 0)? as i32,
    })
}

pub(super) fn spell_interrupts(table: &CreationDb2, id: u32) -> Result<SpellInterruptsRecord> {
    Ok(SpellInterruptsRecord {
        id: id,
        difficulty_id: table.bits(id, 0, 0)? as i16,
        interrupt_flags: table.bits(id, 1, 0)? as i32,
        aura_interrupt_flags: [table.bits(id, 2, 0)? as i32, table.bits(id, 2, 1)? as i32],
        channel_interrupt_flags: [table.bits(id, 3, 0)? as i32, table.bits(id, 3, 1)? as i32],
        spell_id: table.bits(id, 4, 0)?,
    })
}

pub(super) fn spell_label(table: &CreationDb2, id: u32) -> Result<SpellLabelRecord> {
    Ok(SpellLabelRecord {
        id: id,
        label_id: table.bits(id, 0, 0)?,
        spell_id: table.bits(id, 1, 0)?,
    })
}

pub(super) fn spell_levels(table: &CreationDb2, id: u32) -> Result<SpellLevelsRecord> {
    Ok(SpellLevelsRecord {
        id: id,
        difficulty_id: table.bits(id, 0, 0)? as i16,
        max_level: table.bits(id, 1, 0)? as i16,
        max_passive_aura_level: table.bits(id, 2, 0)? as u8,
        base_level: table.bits(id, 3, 0)? as i32,
        spell_level: table.bits(id, 4, 0)? as i32,
        spell_id: table.bits(id, 5, 0)?,
    })
}
