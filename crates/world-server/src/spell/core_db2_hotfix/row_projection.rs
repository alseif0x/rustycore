use wow_data::{
    SpellNameEntry,
    SpellCategoriesEntry,
    SpellMiscEntry,
    SpellEffectDb2Entry,
    SpellShapeshiftEntry,
    SpellInterruptsEntry,
    SpellCastTimesEntry,
    SpellCooldownsEntry,
    SpellCastingRequirementsEntry,
    SpellPowerEntry,
    SpellPowerDifficultyEntry,
    SpellAuraRestrictionsEntry,
    SpellCategoryEntry,
    SpellDurationEntry,
    SpellRadiusEntry,
    SpellRangeEntry,
    SpellEquippedItemsEntry,
    SpellTargetRestrictionsEntry,
    SpellXSpellVisualEntry,
};

use wow_persistence::{
    SpellNameHotfixRowLikeCpp,
    SpellCategoriesHotfixRowLikeCpp,
    SpellMiscHotfixRowLikeCpp,
    SpellEffectHotfixRowLikeCpp,
    SpellShapeshiftHotfixRowLikeCpp,
    SpellInterruptsHotfixRowLikeCpp,
    SpellCastTimesHotfixRowLikeCpp,
    SpellCooldownsHotfixRowLikeCpp,
    SpellCastingRequirementsHotfixRowLikeCpp,
    SpellPowerHotfixRowLikeCpp,
    SpellPowerDifficultyHotfixRowLikeCpp,
    SpellAuraRestrictionsHotfixRowLikeCpp,
    SpellCategoryHotfixRowLikeCpp,
    SpellDurationHotfixRowLikeCpp,
    SpellRadiusHotfixRowLikeCpp,
    SpellRangeHotfixRowLikeCpp,
    SpellEquippedItemsHotfixRowLikeCpp,
    SpellTargetRestrictionsHotfixRowLikeCpp,
    SpellXSpellVisualHotfixRowLikeCpp,
};

pub(super) fn spell_name_entry_like_cpp(row: SpellNameHotfixRowLikeCpp) -> SpellNameEntry {
    SpellNameEntry {
        id: row.id,
        name: row.name,
    }
}

pub(super) fn spell_categories_entry_like_cpp(row: SpellCategoriesHotfixRowLikeCpp) -> SpellCategoriesEntry {
    SpellCategoriesEntry {
        id: row.id,
        difficulty_id: row.difficulty_id,
        category: row.category,
        defense_type: row.defense_type,
        dispel_type: row.dispel_type,
        mechanic: row.mechanic,
        prevention_type: row.prevention_type,
        start_recovery_category: row.start_recovery_category,
        charge_category: row.charge_category,
        spell_id: row.spell_id,
    }
}

pub(super) fn spell_misc_entry_like_cpp(row: SpellMiscHotfixRowLikeCpp) -> SpellMiscEntry {
    SpellMiscEntry {
        id: row.id,
        attributes: row.attributes,
        difficulty_id: row.difficulty_id,
        casting_time_index: row.casting_time_index,
        duration_index: row.duration_index,
        range_index: row.range_index,
        school_mask: row.school_mask,
        speed: row.speed,
        launch_delay: row.launch_delay,
        min_duration: row.min_duration,
        spell_icon_file_data_id: row.spell_icon_file_data_id,
        active_icon_file_data_id: row.active_icon_file_data_id,
        content_tuning_id: row.content_tuning_id,
        show_future_spell_player_condition_id: row.show_future_spell_player_condition_id,
        spell_id: row.spell_id,
    }
}

pub(super) fn spell_effect_entry_like_cpp(row: SpellEffectHotfixRowLikeCpp) -> SpellEffectDb2Entry {
    SpellEffectDb2Entry {
        id: row.id,
        difficulty_id: row.difficulty_id,
        effect_index: row.effect_index,
        effect: row.effect,
        effect_amplitude: row.effect_amplitude,
        effect_attributes: row.effect_attributes,
        effect_aura: row.effect_aura,
        effect_aura_period: row.effect_aura_period,
        effect_base_points: row.effect_base_points,
        effect_bonus_coefficient: row.effect_bonus_coefficient,
        effect_chain_amplitude: row.effect_chain_amplitude,
        effect_chain_targets: row.effect_chain_targets,
        effect_die_sides: row.effect_die_sides,
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
        effect_misc_value: row.effect_misc_value,
        effect_radius_index: row.effect_radius_index,
        effect_spell_class_mask: row.effect_spell_class_mask,
        implicit_target: row.implicit_target,
        spell_id: row.spell_id,
    }
}

pub(super) fn spell_shapeshift_entry_like_cpp(row: SpellShapeshiftHotfixRowLikeCpp) -> SpellShapeshiftEntry {
    SpellShapeshiftEntry {
        id: row.id,
        spell_id: row.spell_id,
        stance_bar_order: row.stance_bar_order,
        shapeshift_exclude: row.shapeshift_exclude,
        shapeshift_mask: row.shapeshift_mask,
    }
}

pub(super) fn spell_interrupts_entry_like_cpp(row: SpellInterruptsHotfixRowLikeCpp) -> SpellInterruptsEntry {
    SpellInterruptsEntry {
        id: row.id,
        difficulty_id: row.difficulty_id,
        interrupt_flags: row.interrupt_flags,
        aura_interrupt_flags: row.aura_interrupt_flags,
        channel_interrupt_flags: row.channel_interrupt_flags,
        spell_id: row.spell_id,
    }
}

pub(super) fn spell_cast_times_entry_like_cpp(row: SpellCastTimesHotfixRowLikeCpp) -> SpellCastTimesEntry {
    SpellCastTimesEntry {
        id: row.id,
        base: row.base,
        per_level: row.per_level,
        minimum: row.minimum,
    }
}

pub(super) fn spell_cooldowns_entry_like_cpp(row: SpellCooldownsHotfixRowLikeCpp) -> SpellCooldownsEntry {
    SpellCooldownsEntry {
        id: row.id,
        difficulty_id: row.difficulty_id,
        category_recovery_time: row.category_recovery_time,
        recovery_time: row.recovery_time,
        start_recovery_time: row.start_recovery_time,
        spell_id: row.spell_id,
    }
}

pub(super) fn spell_casting_requirements_entry_like_cpp(
    row: SpellCastingRequirementsHotfixRowLikeCpp,
) -> SpellCastingRequirementsEntry {
    SpellCastingRequirementsEntry {
        id: row.id,
        spell_id: row.spell_id,
        facing_caster_flags: row.facing_caster_flags,
        min_faction_id: row.min_faction_id,
        min_reputation: row.min_reputation,
        required_areas_id: row.required_areas_id,
        required_aura_vision: row.required_aura_vision,
        requires_spell_focus: row.requires_spell_focus,
    }
}

pub(super) fn spell_power_entry_like_cpp(row: SpellPowerHotfixRowLikeCpp) -> SpellPowerEntry {
    SpellPowerEntry {
        id: row.id,
        order_index: row.order_index,
        mana_cost: row.mana_cost,
        mana_cost_per_level: row.mana_cost_per_level,
        mana_per_second: row.mana_per_second,
        power_display_id: row.power_display_id,
        alt_power_bar_id: row.alt_power_bar_id,
        power_cost_pct: row.power_cost_pct,
        power_cost_max_pct: row.power_cost_max_pct,
        power_pct_per_second: row.power_pct_per_second,
        power_type: row.power_type,
        required_aura_spell_id: row.required_aura_spell_id,
        optional_cost: row.optional_cost,
        spell_id: row.spell_id,
    }
}

pub(super) fn spell_power_difficulty_entry_like_cpp(
    row: SpellPowerDifficultyHotfixRowLikeCpp,
) -> SpellPowerDifficultyEntry {
    SpellPowerDifficultyEntry {
        id: row.id,
        difficulty_id: row.difficulty_id,
        order_index: row.order_index,
    }
}

pub(super) fn spell_aura_restrictions_entry_like_cpp(
    row: SpellAuraRestrictionsHotfixRowLikeCpp,
) -> SpellAuraRestrictionsEntry {
    SpellAuraRestrictionsEntry {
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
        spell_id: row.spell_id,
    }
}

pub(super) fn spell_category_entry_like_cpp(row: SpellCategoryHotfixRowLikeCpp) -> SpellCategoryEntry {
    SpellCategoryEntry {
        id: row.id,
        name: row.name,
        flags: row.flags,
        uses_per_week: row.uses_per_week,
        max_charges: row.max_charges,
        charge_recovery_time: row.charge_recovery_time,
        type_mask: row.type_mask,
    }
}

pub(super) fn spell_duration_entry_like_cpp(row: SpellDurationHotfixRowLikeCpp) -> SpellDurationEntry {
    SpellDurationEntry {
        id: row.id,
        duration: row.duration,
        duration_per_level: row.duration_per_level,
        max_duration: row.max_duration,
    }
}

pub(super) fn spell_radius_entry_like_cpp(row: SpellRadiusHotfixRowLikeCpp) -> SpellRadiusEntry {
    SpellRadiusEntry {
        id: row.id,
        radius: row.radius,
        radius_per_level: row.radius_per_level,
        radius_min: row.radius_min,
        radius_max: row.radius_max,
    }
}

pub(super) fn spell_range_entry_like_cpp(row: SpellRangeHotfixRowLikeCpp) -> SpellRangeEntry {
    SpellRangeEntry {
        id: row.id,
        display_name: row.display_name,
        display_name_short: row.display_name_short,
        flags: row.flags,
        range_min: row.range_min,
        range_max: row.range_max,
    }
}

pub(super) fn spell_equipped_items_entry_like_cpp(
    row: SpellEquippedItemsHotfixRowLikeCpp,
) -> SpellEquippedItemsEntry {
    SpellEquippedItemsEntry {
        id: row.id,
        spell_id: row.spell_id,
        equipped_item_class: row.equipped_item_class,
        equipped_item_inv_types: row.equipped_item_inv_types,
        equipped_item_subclass: row.equipped_item_subclass,
    }
}

pub(super) fn spell_target_restrictions_entry_like_cpp(
    row: SpellTargetRestrictionsHotfixRowLikeCpp,
) -> SpellTargetRestrictionsEntry {
    SpellTargetRestrictionsEntry {
        id: row.id,
        difficulty_id: row.difficulty_id,
        cone_degrees: row.cone_degrees,
        max_targets: row.max_targets,
        max_target_level: row.max_target_level,
        target_creature_type: row.target_creature_type,
        targets: row.targets,
        width: row.width,
        spell_id: row.spell_id,
    }
}

pub(super) fn spell_x_spell_visual_entry_like_cpp(
    row: SpellXSpellVisualHotfixRowLikeCpp,
) -> SpellXSpellVisualEntry {
    SpellXSpellVisualEntry {
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
    }
}
