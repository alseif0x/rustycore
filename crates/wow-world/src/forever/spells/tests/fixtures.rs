//! Synthetic public-shape fixtures only, no extracted client values.
use wow_data::forever_spells::*;

pub(super) fn spell_effect(id: u32, spell: u32) -> SpellEffectRecord {
    SpellEffectRecord {
        id: id,
        effect_aura: 0,
        difficulty_id: 0,
        effect_index: 0,
        effect: 0,
        effect_amplitude: 0.0,
        effect_attributes: 0,
        effect_aura_period: 0,
        effect_bonus_coefficient: 0.0,
        effect_chain_amplitude: 0.0,
        effect_chain_targets: 0,
        effect_item_type: 0,
        effect_mechanic: 0,
        effect_points_per_resource: 0.0,
        effect_pos_facing: 0.0,
        effect_real_points_per_level: 0.0,
        effect_trigger_spell: 0,
        bonus_coefficient_from_ap: 0.0,
        pvp_multiplier: 0.0,
        coefficient: 0.0,
        variance: 0.0,
        resource_coefficient: 0.0,
        group_size_base_points_coefficient: 0.0,
        effect_base_points: 0.0,
        scaling_class: 0,
        target_node_graph: 0,
        effect_misc_value: [0; 2],
        effect_radius_index: [0; 2],
        effect_spell_class_mask: [0; 4],
        implicit_target: [0; 2],
        spell_id: spell,
    }
}

pub(super) fn spell_power(id: u32, spell: u32) -> SpellPowerRecord {
    SpellPowerRecord {
        id: id,
        order_index: 0,
        mana_cost: 0,
        mana_cost_per_level: 0,
        mana_per_second: 0,
        power_display_id: 0,
        alt_power_bar_id: 0,
        power_cost_pct: 0.0,
        power_cost_max_pct: 0.0,
        optional_cost_pct: 0.0,
        power_pct_per_second: 0.0,
        power_type: 0,
        required_aura_spell_id: 0,
        optional_cost: 0,
        spell_id: spell,
    }
}

pub(super) fn spell_x_spell_visual(id: u32, spell: u32) -> SpellXSpellVisualRecord {
    SpellXSpellVisualRecord {
        id: id,
        difficulty_id: 0,
        spell_visual_id: 0,
        probability: 0.0,
        flags: 0,
        priority: 0,
        spell_icon_file_id: 0,
        active_icon_file_id: 0,
        viewer_unit_condition_id: 0,
        viewer_player_condition_id: 0,
        caster_unit_condition_id: 0,
        caster_player_condition_id: 0,
        spell_id: spell,
    }
}

pub(super) fn difficulty(id: u32, _spell: u32) -> DifficultyRecord {
    DifficultyRecord {
        id: id,
        name: SpellText::default(),
        instance_type: 0,
        order_index: 0,
        old_enum_value: 0,
        fallback_difficulty_id: 0,
        min_players: 0,
        max_players: 0,
        flags: 0,
        item_context: 0,
        toggle_difficulty_id: 0,
        group_size_health_curve_id: 0,
        group_size_dmg_curve_id: 0,
        group_size_spell_points_curve_id: 0,
        unknown1105: 0,
    }
}

pub(super) fn spell_aura_options(id: u32, spell: u32) -> SpellAuraOptionsRecord {
    SpellAuraOptionsRecord {
        id: id,
        difficulty_id: 0,
        cumulative_aura: 0,
        proc_category_recovery: 0,
        proc_chance: 0,
        proc_charges: 0,
        spell_procs_per_minute_id: 0,
        proc_type_mask: [0; 2],
        spell_id: spell,
    }
}

pub(super) fn spell_aura_restrictions(id: u32, spell: u32) -> SpellAuraRestrictionsRecord {
    SpellAuraRestrictionsRecord {
        id: id,
        difficulty_id: 0,
        caster_aura_state: 0,
        target_aura_state: 0,
        exclude_caster_aura_state: 0,
        exclude_target_aura_state: 0,
        caster_aura_spell: 0,
        target_aura_spell: 0,
        exclude_caster_aura_spell: 0,
        exclude_target_aura_spell: 0,
        caster_aura_type: 0,
        target_aura_type: 0,
        exclude_caster_aura_type: 0,
        exclude_target_aura_type: 0,
        spell_id: spell,
    }
}

pub(super) fn spell_casting_requirements(id: u32, spell: u32) -> SpellCastingRequirementsRecord {
    SpellCastingRequirementsRecord {
        id: id,
        spell_id: spell as i32,
        facing_caster_flags: 0,
        min_faction_id: 0,
        min_reputation: 0,
        required_areas_id: 0,
        required_aura_vision: 0,
        requires_spell_focus: 0,
    }
}

pub(super) fn spell_categories(id: u32, spell: u32) -> SpellCategoriesRecord {
    SpellCategoriesRecord {
        id: id,
        difficulty_id: 0,
        category: 0,
        defense_type: 0,
        diminish_type: 0,
        dispel_type: 0,
        mechanic: 0,
        prevention_type: 0,
        start_recovery_category: 0,
        charge_category: 0,
        spell_id: spell,
    }
}

pub(super) fn spell_class_options(id: u32, spell: u32) -> SpellClassOptionsRecord {
    SpellClassOptionsRecord {
        id: id,
        spell_id: spell as i32,
        modal_next_spell: 0,
        spell_class_set: 0,
        spell_class_mask: [0; 4],
    }
}

pub(super) fn spell_cooldowns(id: u32, spell: u32) -> SpellCooldownsRecord {
    SpellCooldownsRecord {
        id: id,
        difficulty_id: 0,
        category_recovery_time: 0,
        recovery_time: 0,
        start_recovery_time: 0,
        aura_spell_id: 0,
        spell_id: spell,
    }
}

pub(super) fn spell_equipped_items(id: u32, spell: u32) -> SpellEquippedItemsRecord {
    SpellEquippedItemsRecord {
        id: id,
        spell_id: spell as i32,
        equipped_item_class: 0,
        equipped_item_inv_types: 0,
        equipped_item_subclass: 0,
    }
}

pub(super) fn spell_interrupts(id: u32, spell: u32) -> SpellInterruptsRecord {
    SpellInterruptsRecord {
        id: id,
        difficulty_id: 0,
        interrupt_flags: 0,
        aura_interrupt_flags: [0; 2],
        channel_interrupt_flags: [0; 2],
        spell_id: spell,
    }
}

pub(super) fn spell_levels(id: u32, spell: u32) -> SpellLevelsRecord {
    SpellLevelsRecord {
        id: id,
        difficulty_id: 0,
        max_level: 0,
        max_passive_aura_level: 0,
        base_level: 0,
        spell_level: 0,
        spell_id: spell,
    }
}

pub(super) fn spell_misc(id: u32, spell: u32) -> SpellMiscRecord {
    SpellMiscRecord {
        id: id,
        attributes: [0; 17],
        difficulty_id: 0,
        casting_time_index: 0,
        duration_index: 0,
        pv_p_duration_index: 0,
        range_index: 0,
        school_mask: 0,
        speed: 0.0,
        launch_delay: 0.0,
        min_duration: 0.0,
        spell_icon_file_data_id: 0,
        active_icon_file_data_id: 0,
        content_tuning_id: 0,
        show_future_spell_player_condition_id: 0,
        spell_visual_script: 0,
        active_spell_visual_script: 0,
        spell_id: spell,
    }
}

pub(super) fn spell_reagents(id: u32, spell: u32) -> SpellReagentsRecord {
    SpellReagentsRecord {
        id: id,
        spell_id: spell as i32,
        reagent: [0; 8],
        reagent_count: [0; 8],
        reagent_recraft_count: [0; 8],
        reagent_source: [0; 8],
    }
}

pub(super) fn spell_scaling(id: u32, spell: u32) -> SpellScalingRecord {
    SpellScalingRecord {
        id: id,
        spell_id: spell as i32,
        min_scaling_level: 0,
        max_scaling_level: 0,
    }
}

pub(super) fn spell_shapeshift(id: u32, spell: u32) -> SpellShapeshiftRecord {
    SpellShapeshiftRecord {
        id: id,
        spell_id: spell as i32,
        stance_bar_order: 0,
        shapeshift_exclude: [0; 2],
        shapeshift_mask: [0; 2],
    }
}

pub(super) fn spell_target_restrictions(id: u32, spell: u32) -> SpellTargetRestrictionsRecord {
    SpellTargetRestrictionsRecord {
        id: id,
        difficulty_id: 0,
        cone_degrees: 0.0,
        max_targets: 0,
        max_target_level: 0,
        target_creature_type: 0,
        targets: 0,
        width: 0.0,
        spell_id: spell,
    }
}

pub(super) fn spell_totems(id: u32, spell: u32) -> SpellTotemsRecord {
    SpellTotemsRecord {
        id: id,
        spell_id: spell as i32,
        required_totem_category_id: [0; 2],
        totem: [0; 2],
    }
}
