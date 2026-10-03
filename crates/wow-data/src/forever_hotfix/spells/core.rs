//! 02245dcd DB2StorageBase::WriteRecord, source core spell metadata.
use super::text;
use crate::forever_spells::{
    SpellAuraOptionsRecord, SpellAuraRestrictionsRecord, SpellCastingRequirementsRecord,
    SpellCategoriesRecord, SpellClassOptionsRecord, SpellCooldownsRecord, SpellEffectRecord,
    SpellEquippedItemsRecord, SpellInterruptsRecord, SpellLabelRecord, SpellLevelsRecord,
    SpellMiscRecord, SpellNameRecord,
};

pub(super) fn spell_name(row: &SpellNameRecord, locale: u8) -> Vec<u8> {
    let mut bytes = Vec::new();
    text(&mut bytes, &row.name, locale);
    bytes
}

pub(super) fn spell_effect(row: &SpellEffectRecord, _locale: u8) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&row.effect_aura.to_le_bytes());
    bytes.extend_from_slice(&row.difficulty_id.to_le_bytes());
    bytes.extend_from_slice(&row.effect_index.to_le_bytes());
    bytes.extend_from_slice(&row.effect.to_le_bytes());
    bytes.extend_from_slice(&row.effect_amplitude.to_le_bytes());
    bytes.extend_from_slice(&row.effect_attributes.to_le_bytes());
    bytes.extend_from_slice(&row.effect_aura_period.to_le_bytes());
    bytes.extend_from_slice(&row.effect_bonus_coefficient.to_le_bytes());
    bytes.extend_from_slice(&row.effect_chain_amplitude.to_le_bytes());
    bytes.extend_from_slice(&row.effect_chain_targets.to_le_bytes());
    bytes.extend_from_slice(&row.effect_item_type.to_le_bytes());
    bytes.extend_from_slice(&row.effect_mechanic.to_le_bytes());
    bytes.extend_from_slice(&row.effect_points_per_resource.to_le_bytes());
    bytes.extend_from_slice(&row.effect_pos_facing.to_le_bytes());
    bytes.extend_from_slice(&row.effect_real_points_per_level.to_le_bytes());
    bytes.extend_from_slice(&row.effect_trigger_spell.to_le_bytes());
    bytes.extend_from_slice(&row.bonus_coefficient_from_ap.to_le_bytes());
    bytes.extend_from_slice(&row.pvp_multiplier.to_le_bytes());
    bytes.extend_from_slice(&row.coefficient.to_le_bytes());
    bytes.extend_from_slice(&row.variance.to_le_bytes());
    bytes.extend_from_slice(&row.resource_coefficient.to_le_bytes());
    bytes.extend_from_slice(&row.group_size_base_points_coefficient.to_le_bytes());
    bytes.extend_from_slice(&row.effect_base_points.to_le_bytes());
    bytes.extend_from_slice(&row.scaling_class.to_le_bytes());
    bytes.extend_from_slice(&row.target_node_graph.to_le_bytes());
    for value in row.effect_misc_value {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    for value in row.effect_radius_index {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    for value in row.effect_spell_class_mask {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    for value in row.implicit_target {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    bytes.extend_from_slice(&row.spell_id.to_le_bytes());
    bytes
}

pub(super) fn spell_misc(row: &SpellMiscRecord, _locale: u8) -> Vec<u8> {
    let mut bytes = Vec::new();
    for value in row.attributes {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    bytes.extend_from_slice(&row.difficulty_id.to_le_bytes());
    bytes.extend_from_slice(&row.casting_time_index.to_le_bytes());
    bytes.extend_from_slice(&row.duration_index.to_le_bytes());
    bytes.extend_from_slice(&row.pv_p_duration_index.to_le_bytes());
    bytes.extend_from_slice(&row.range_index.to_le_bytes());
    bytes.extend_from_slice(&row.school_mask.to_le_bytes());
    bytes.extend_from_slice(&row.speed.to_le_bytes());
    bytes.extend_from_slice(&row.launch_delay.to_le_bytes());
    bytes.extend_from_slice(&row.min_duration.to_le_bytes());
    bytes.extend_from_slice(&row.spell_icon_file_data_id.to_le_bytes());
    bytes.extend_from_slice(&row.active_icon_file_data_id.to_le_bytes());
    bytes.extend_from_slice(&row.content_tuning_id.to_le_bytes());
    bytes.extend_from_slice(&row.show_future_spell_player_condition_id.to_le_bytes());
    bytes.extend_from_slice(&row.spell_visual_script.to_le_bytes());
    bytes.extend_from_slice(&row.active_spell_visual_script.to_le_bytes());
    bytes.extend_from_slice(&row.spell_id.to_le_bytes());
    bytes
}

pub(super) fn spell_aura_options(row: &SpellAuraOptionsRecord, _locale: u8) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&row.difficulty_id.to_le_bytes());
    bytes.extend_from_slice(&row.cumulative_aura.to_le_bytes());
    bytes.extend_from_slice(&row.proc_category_recovery.to_le_bytes());
    bytes.extend_from_slice(&row.proc_chance.to_le_bytes());
    bytes.extend_from_slice(&row.proc_charges.to_le_bytes());
    bytes.extend_from_slice(&row.spell_procs_per_minute_id.to_le_bytes());
    for value in row.proc_type_mask {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    bytes.extend_from_slice(&row.spell_id.to_le_bytes());
    bytes
}

pub(super) fn spell_aura_restrictions(row: &SpellAuraRestrictionsRecord, _locale: u8) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&row.difficulty_id.to_le_bytes());
    bytes.extend_from_slice(&row.caster_aura_state.to_le_bytes());
    bytes.extend_from_slice(&row.target_aura_state.to_le_bytes());
    bytes.extend_from_slice(&row.exclude_caster_aura_state.to_le_bytes());
    bytes.extend_from_slice(&row.exclude_target_aura_state.to_le_bytes());
    bytes.extend_from_slice(&row.caster_aura_spell.to_le_bytes());
    bytes.extend_from_slice(&row.target_aura_spell.to_le_bytes());
    bytes.extend_from_slice(&row.exclude_caster_aura_spell.to_le_bytes());
    bytes.extend_from_slice(&row.exclude_target_aura_spell.to_le_bytes());
    bytes.extend_from_slice(&row.caster_aura_type.to_le_bytes());
    bytes.extend_from_slice(&row.target_aura_type.to_le_bytes());
    bytes.extend_from_slice(&row.exclude_caster_aura_type.to_le_bytes());
    bytes.extend_from_slice(&row.exclude_target_aura_type.to_le_bytes());
    bytes.extend_from_slice(&row.spell_id.to_le_bytes());
    bytes
}

pub(super) fn spell_casting_requirements(
    row: &SpellCastingRequirementsRecord,
    _locale: u8,
) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&row.spell_id.to_le_bytes());
    bytes.extend_from_slice(&row.facing_caster_flags.to_le_bytes());
    bytes.extend_from_slice(&row.min_faction_id.to_le_bytes());
    bytes.extend_from_slice(&row.min_reputation.to_le_bytes());
    bytes.extend_from_slice(&row.required_areas_id.to_le_bytes());
    bytes.extend_from_slice(&row.required_aura_vision.to_le_bytes());
    bytes.extend_from_slice(&row.requires_spell_focus.to_le_bytes());
    bytes
}

pub(super) fn spell_categories(row: &SpellCategoriesRecord, _locale: u8) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&row.difficulty_id.to_le_bytes());
    bytes.extend_from_slice(&row.category.to_le_bytes());
    bytes.extend_from_slice(&row.defense_type.to_le_bytes());
    bytes.extend_from_slice(&row.diminish_type.to_le_bytes());
    bytes.extend_from_slice(&row.dispel_type.to_le_bytes());
    bytes.extend_from_slice(&row.mechanic.to_le_bytes());
    bytes.extend_from_slice(&row.prevention_type.to_le_bytes());
    bytes.extend_from_slice(&row.start_recovery_category.to_le_bytes());
    bytes.extend_from_slice(&row.charge_category.to_le_bytes());
    bytes.extend_from_slice(&row.spell_id.to_le_bytes());
    bytes
}

pub(super) fn spell_class_options(row: &SpellClassOptionsRecord, _locale: u8) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&row.spell_id.to_le_bytes());
    bytes.extend_from_slice(&row.modal_next_spell.to_le_bytes());
    bytes.extend_from_slice(&row.spell_class_set.to_le_bytes());
    for value in row.spell_class_mask {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    bytes
}

pub(super) fn spell_cooldowns(row: &SpellCooldownsRecord, _locale: u8) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&row.difficulty_id.to_le_bytes());
    bytes.extend_from_slice(&row.category_recovery_time.to_le_bytes());
    bytes.extend_from_slice(&row.recovery_time.to_le_bytes());
    bytes.extend_from_slice(&row.start_recovery_time.to_le_bytes());
    bytes.extend_from_slice(&row.aura_spell_id.to_le_bytes());
    bytes.extend_from_slice(&row.spell_id.to_le_bytes());
    bytes
}

pub(super) fn spell_equipped_items(row: &SpellEquippedItemsRecord, _locale: u8) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&row.spell_id.to_le_bytes());
    bytes.extend_from_slice(&row.equipped_item_class.to_le_bytes());
    bytes.extend_from_slice(&row.equipped_item_inv_types.to_le_bytes());
    bytes.extend_from_slice(&row.equipped_item_subclass.to_le_bytes());
    bytes
}

pub(super) fn spell_interrupts(row: &SpellInterruptsRecord, _locale: u8) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&row.difficulty_id.to_le_bytes());
    bytes.extend_from_slice(&row.interrupt_flags.to_le_bytes());
    for value in row.aura_interrupt_flags {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    for value in row.channel_interrupt_flags {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    bytes.extend_from_slice(&row.spell_id.to_le_bytes());
    bytes
}

pub(super) fn spell_label(row: &SpellLabelRecord, _locale: u8) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&row.label_id.to_le_bytes());
    bytes.extend_from_slice(&row.spell_id.to_le_bytes());
    bytes
}

pub(super) fn spell_levels(row: &SpellLevelsRecord, _locale: u8) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&row.difficulty_id.to_le_bytes());
    bytes.extend_from_slice(&row.max_level.to_le_bytes());
    bytes.extend_from_slice(&row.max_passive_aura_level.to_le_bytes());
    bytes.extend_from_slice(&row.base_level.to_le_bytes());
    bytes.extend_from_slice(&row.spell_level.to_le_bytes());
    bytes.extend_from_slice(&row.spell_id.to_le_bytes());
    bytes
}
