use wow_persistence::forever::spells::{
    SpellAuraOptionsRow, SpellAuraRestrictionsRow, SpellCastingRequirementsRow, SpellCategoriesRow,
    SpellClassOptionsRow, SpellCooldownsRow, SpellEffectRow, SpellEquippedItemsRow,
    SpellInterruptsRow, SpellLabelRow, SpellLevelsRow, SpellMiscRow, SpellNameRow,
};

#[test]
fn spell_name_moves_all_source_columns_without_loss() {
    let result = super::super::core::spell_name(SpellNameRow {
        id: u32::MAX,
        name: vec![0xFF, 0, b'a'],
    })
    .unwrap();
    assert_eq!(result.id, u32::MAX);
    assert_eq!(result.name.at(0), Some(&[0xFF, 0, b'a'][..]));
    assert_eq!(result.name.at(6), None);
}

#[test]
fn spell_effect_moves_all_source_columns_without_loss() {
    let result = super::super::core::spell_effect(SpellEffectRow {
        id: u32::MAX,
        effect_aura: i16::MIN,
        difficulty_id: i16::MIN,
        effect_index: i32::MIN,
        effect: u32::MAX,
        effect_amplitude: f32::from_bits(0x7FC0_1234),
        effect_attributes: i32::MIN,
        effect_aura_period: i32::MIN,
        effect_bonus_coefficient: f32::from_bits(0x7FC0_1234),
        effect_chain_amplitude: f32::from_bits(0x7FC0_1234),
        effect_chain_targets: i32::MIN,
        effect_item_type: i32::MIN,
        effect_mechanic: i32::MIN,
        effect_points_per_resource: f32::from_bits(0x7FC0_1234),
        effect_pos_facing: f32::from_bits(0x7FC0_1234),
        effect_real_points_per_level: f32::from_bits(0x7FC0_1234),
        effect_trigger_spell: i32::MIN,
        bonus_coefficient_from_ap: f32::from_bits(0x7FC0_1234),
        pvp_multiplier: f32::from_bits(0x7FC0_1234),
        coefficient: f32::from_bits(0x7FC0_1234),
        variance: f32::from_bits(0x7FC0_1234),
        resource_coefficient: f32::from_bits(0x7FC0_1234),
        group_size_base_points_coefficient: f32::from_bits(0x7FC0_1234),
        effect_base_points: f32::from_bits(0x7FC0_1234),
        scaling_class: i32::MIN,
        target_node_graph: i32::MIN,
        effect_misc_value: [i32::MIN; 2],
        effect_radius_index: [u32::MAX; 2],
        effect_spell_class_mask: [i32::MIN; 4],
        implicit_target: [i16::MIN; 2],
        spell_id: u32::MAX,
    })
    .unwrap();
    assert_eq!(result.id, u32::MAX);
    assert_eq!(result.effect_aura, i16::MIN);
    assert_eq!(result.difficulty_id, i16::MIN);
    assert_eq!(result.effect_index, i32::MIN);
    assert_eq!(result.effect, u32::MAX);
    assert_eq!(result.effect_amplitude.to_bits(), 0x7FC0_1234);
    assert_eq!(result.effect_attributes, i32::MIN);
    assert_eq!(result.effect_aura_period, i32::MIN);
    assert_eq!(result.effect_bonus_coefficient.to_bits(), 0x7FC0_1234);
    assert_eq!(result.effect_chain_amplitude.to_bits(), 0x7FC0_1234);
    assert_eq!(result.effect_chain_targets, i32::MIN);
    assert_eq!(result.effect_item_type, i32::MIN);
    assert_eq!(result.effect_mechanic, i32::MIN);
    assert_eq!(result.effect_points_per_resource.to_bits(), 0x7FC0_1234);
    assert_eq!(result.effect_pos_facing.to_bits(), 0x7FC0_1234);
    assert_eq!(result.effect_real_points_per_level.to_bits(), 0x7FC0_1234);
    assert_eq!(result.effect_trigger_spell, i32::MIN);
    assert_eq!(result.bonus_coefficient_from_ap.to_bits(), 0x7FC0_1234);
    assert_eq!(result.pvp_multiplier.to_bits(), 0x7FC0_1234);
    assert_eq!(result.coefficient.to_bits(), 0x7FC0_1234);
    assert_eq!(result.variance.to_bits(), 0x7FC0_1234);
    assert_eq!(result.resource_coefficient.to_bits(), 0x7FC0_1234);
    assert_eq!(
        result.group_size_base_points_coefficient.to_bits(),
        0x7FC0_1234
    );
    assert_eq!(result.effect_base_points.to_bits(), 0x7FC0_1234);
    assert_eq!(result.scaling_class, i32::MIN);
    assert_eq!(result.target_node_graph, i32::MIN);
    assert_eq!(result.effect_misc_value, [i32::MIN; 2]);
    assert_eq!(result.effect_radius_index, [u32::MAX; 2]);
    assert_eq!(result.effect_spell_class_mask, [i32::MIN as u32; 4]);
    assert_eq!(result.implicit_target, [i16::MIN; 2]);
    assert_eq!(result.spell_id, u32::MAX);
}

#[test]
fn spell_misc_moves_all_source_columns_without_loss() {
    let result = super::super::core::spell_misc(SpellMiscRow {
        id: u32::MAX,
        attributes: [i32::MIN; 17],
        difficulty_id: i16::MIN,
        casting_time_index: u16::MAX,
        duration_index: u16::MAX,
        pv_p_duration_index: u16::MAX,
        range_index: u16::MAX,
        school_mask: u8::MAX,
        speed: f32::from_bits(0x7FC0_1234),
        launch_delay: f32::from_bits(0x7FC0_1234),
        min_duration: f32::from_bits(0x7FC0_1234),
        spell_icon_file_data_id: i32::MIN,
        active_icon_file_data_id: i32::MIN,
        content_tuning_id: i32::MIN,
        show_future_spell_player_condition_id: i32::MIN,
        spell_visual_script: i32::MIN,
        active_spell_visual_script: i32::MIN,
        spell_id: u32::MAX,
    })
    .unwrap();
    assert_eq!(result.id, u32::MAX);
    assert_eq!(result.attributes, [i32::MIN; 17]);
    assert_eq!(result.difficulty_id, i16::MIN);
    assert_eq!(result.casting_time_index, u16::MAX);
    assert_eq!(result.duration_index, u16::MAX);
    assert_eq!(result.pv_p_duration_index, u16::MAX);
    assert_eq!(result.range_index, u16::MAX);
    assert_eq!(result.school_mask, u8::MAX);
    assert_eq!(result.speed.to_bits(), 0x7FC0_1234);
    assert_eq!(result.launch_delay.to_bits(), 0x7FC0_1234);
    assert_eq!(result.min_duration.to_bits(), 0x7FC0_1234);
    assert_eq!(result.spell_icon_file_data_id, i32::MIN);
    assert_eq!(result.active_icon_file_data_id, i32::MIN);
    assert_eq!(result.content_tuning_id, i32::MIN);
    assert_eq!(result.show_future_spell_player_condition_id, i32::MIN);
    assert_eq!(result.spell_visual_script, i32::MIN);
    assert_eq!(result.active_spell_visual_script, i32::MIN);
    assert_eq!(result.spell_id, u32::MAX);
}

#[test]
fn spell_aura_options_moves_all_source_columns_without_loss() {
    let result = super::super::core::spell_aura_options(SpellAuraOptionsRow {
        id: u32::MAX,
        difficulty_id: i16::MIN,
        cumulative_aura: u16::MAX,
        proc_category_recovery: i32::MIN,
        proc_chance: u8::MAX,
        proc_charges: i32::MIN,
        spell_procs_per_minute_id: u16::MAX,
        proc_type_mask: [i32::MIN; 2],
        spell_id: u32::MAX,
    })
    .unwrap();
    assert_eq!(result.id, u32::MAX);
    assert_eq!(result.difficulty_id, i16::MIN);
    assert_eq!(result.cumulative_aura, u16::MAX);
    assert_eq!(result.proc_category_recovery, i32::MIN);
    assert_eq!(result.proc_chance, u8::MAX);
    assert_eq!(result.proc_charges, i32::MIN);
    assert_eq!(result.spell_procs_per_minute_id, u16::MAX);
    assert_eq!(result.proc_type_mask, [i32::MIN; 2]);
    assert_eq!(result.spell_id, u32::MAX);
}

#[test]
fn spell_aura_restrictions_moves_all_source_columns_without_loss() {
    let result = super::super::core::spell_aura_restrictions(SpellAuraRestrictionsRow {
        id: u32::MAX,
        difficulty_id: i16::MIN,
        caster_aura_state: i32::MIN,
        target_aura_state: i32::MIN,
        exclude_caster_aura_state: i32::MIN,
        exclude_target_aura_state: i32::MIN,
        caster_aura_spell: i32::MIN,
        target_aura_spell: i32::MIN,
        exclude_caster_aura_spell: i32::MIN,
        exclude_target_aura_spell: i32::MIN,
        caster_aura_type: i16::MIN,
        target_aura_type: i16::MIN,
        exclude_caster_aura_type: i16::MIN,
        exclude_target_aura_type: i16::MIN,
        spell_id: u32::MAX,
    })
    .unwrap();
    assert_eq!(result.id, u32::MAX);
    assert_eq!(result.difficulty_id, i16::MIN);
    assert_eq!(result.caster_aura_state, i32::MIN);
    assert_eq!(result.target_aura_state, i32::MIN);
    assert_eq!(result.exclude_caster_aura_state, i32::MIN);
    assert_eq!(result.exclude_target_aura_state, i32::MIN);
    assert_eq!(result.caster_aura_spell, i32::MIN);
    assert_eq!(result.target_aura_spell, i32::MIN);
    assert_eq!(result.exclude_caster_aura_spell, i32::MIN);
    assert_eq!(result.exclude_target_aura_spell, i32::MIN);
    assert_eq!(result.caster_aura_type, i16::MIN);
    assert_eq!(result.target_aura_type, i16::MIN);
    assert_eq!(result.exclude_caster_aura_type, i16::MIN);
    assert_eq!(result.exclude_target_aura_type, i16::MIN);
    assert_eq!(result.spell_id, u32::MAX);
}

#[test]
fn spell_casting_requirements_moves_all_source_columns_without_loss() {
    let result = super::super::core::spell_casting_requirements(SpellCastingRequirementsRow {
        id: u32::MAX,
        spell_id: i32::MIN,
        facing_caster_flags: i32::MIN,
        min_faction_id: u16::MAX,
        min_reputation: i32::MIN,
        required_areas_id: u16::MAX,
        required_aura_vision: u8::MAX,
        requires_spell_focus: u16::MAX,
    })
    .unwrap();
    assert_eq!(result.id, u32::MAX);
    assert_eq!(result.spell_id, i32::MIN);
    assert_eq!(result.facing_caster_flags, i32::MIN);
    assert_eq!(result.min_faction_id, u16::MAX);
    assert_eq!(result.min_reputation, i32::MIN);
    assert_eq!(result.required_areas_id, u16::MAX);
    assert_eq!(result.required_aura_vision, u8::MAX);
    assert_eq!(result.requires_spell_focus, u16::MAX);
}

#[test]
fn spell_categories_moves_all_source_columns_without_loss() {
    let result = super::super::core::spell_categories(SpellCategoriesRow {
        id: u32::MAX,
        difficulty_id: i16::MIN,
        category: i16::MIN,
        defense_type: i8::MIN,
        diminish_type: i32::MIN,
        dispel_type: i8::MIN,
        mechanic: i8::MIN,
        prevention_type: i32::MIN,
        start_recovery_category: i16::MIN,
        charge_category: i16::MIN,
        spell_id: u32::MAX,
    })
    .unwrap();
    assert_eq!(result.id, u32::MAX);
    assert_eq!(result.difficulty_id, i16::MIN);
    assert_eq!(result.category, i16::MIN);
    assert_eq!(result.defense_type, i8::MIN);
    assert_eq!(result.diminish_type, i32::MIN);
    assert_eq!(result.dispel_type, i8::MIN);
    assert_eq!(result.mechanic, i8::MIN);
    assert_eq!(result.prevention_type, i32::MIN);
    assert_eq!(result.start_recovery_category, i16::MIN);
    assert_eq!(result.charge_category, i16::MIN);
    assert_eq!(result.spell_id, u32::MAX);
}

#[test]
fn spell_class_options_moves_all_source_columns_without_loss() {
    let result = super::super::core::spell_class_options(SpellClassOptionsRow {
        id: u32::MAX,
        spell_id: i32::MIN,
        modal_next_spell: u32::MAX,
        spell_class_set: i32::MIN,
        spell_class_mask: [i32::MIN; 4],
    })
    .unwrap();
    assert_eq!(result.id, u32::MAX);
    assert_eq!(result.spell_id, i32::MIN);
    assert_eq!(result.modal_next_spell, u32::MAX);
    assert_eq!(result.spell_class_set, i32::MIN);
    assert_eq!(result.spell_class_mask, [i32::MIN as u32; 4]);
}

#[test]
fn spell_cooldowns_moves_all_source_columns_without_loss() {
    let result = super::super::core::spell_cooldowns(SpellCooldownsRow {
        id: u32::MAX,
        difficulty_id: i16::MIN,
        category_recovery_time: i32::MIN,
        recovery_time: i32::MIN,
        start_recovery_time: i32::MIN,
        aura_spell_id: i32::MIN,
        spell_id: u32::MAX,
    })
    .unwrap();
    assert_eq!(result.id, u32::MAX);
    assert_eq!(result.difficulty_id, i16::MIN);
    assert_eq!(result.category_recovery_time, i32::MIN);
    assert_eq!(result.recovery_time, i32::MIN);
    assert_eq!(result.start_recovery_time, i32::MIN);
    assert_eq!(result.aura_spell_id, i32::MIN);
    assert_eq!(result.spell_id, u32::MAX);
}

#[test]
fn spell_equipped_items_moves_all_source_columns_without_loss() {
    let result = super::super::core::spell_equipped_items(SpellEquippedItemsRow {
        id: u32::MAX,
        spell_id: i32::MIN,
        equipped_item_class: i32::MIN,
        equipped_item_inv_types: i32::MIN,
        equipped_item_subclass: i32::MIN,
    })
    .unwrap();
    assert_eq!(result.id, u32::MAX);
    assert_eq!(result.spell_id, i32::MIN);
    assert_eq!(result.equipped_item_class, i32::MIN);
    assert_eq!(result.equipped_item_inv_types, i32::MIN);
    assert_eq!(result.equipped_item_subclass, i32::MIN);
}

#[test]
fn spell_interrupts_moves_all_source_columns_without_loss() {
    let result = super::super::core::spell_interrupts(SpellInterruptsRow {
        id: u32::MAX,
        difficulty_id: i16::MIN,
        interrupt_flags: i32::MIN,
        aura_interrupt_flags: [i32::MIN; 2],
        channel_interrupt_flags: [i32::MIN; 2],
        spell_id: u32::MAX,
    })
    .unwrap();
    assert_eq!(result.id, u32::MAX);
    assert_eq!(result.difficulty_id, i16::MIN);
    assert_eq!(result.interrupt_flags, i32::MIN);
    assert_eq!(result.aura_interrupt_flags, [i32::MIN; 2]);
    assert_eq!(result.channel_interrupt_flags, [i32::MIN; 2]);
    assert_eq!(result.spell_id, u32::MAX);
}

#[test]
fn spell_label_moves_all_source_columns_without_loss() {
    let result = super::super::core::spell_label(SpellLabelRow {
        id: u32::MAX,
        label_id: u32::MAX,
        spell_id: u32::MAX,
    })
    .unwrap();
    assert_eq!(result.id, u32::MAX);
    assert_eq!(result.label_id, u32::MAX);
    assert_eq!(result.spell_id, u32::MAX);
}

#[test]
fn spell_levels_moves_all_source_columns_without_loss() {
    let result = super::super::core::spell_levels(SpellLevelsRow {
        id: u32::MAX,
        difficulty_id: i16::MIN,
        max_level: i16::MIN,
        max_passive_aura_level: u8::MAX,
        base_level: i32::MIN,
        spell_level: i32::MIN,
        spell_id: u32::MAX,
    })
    .unwrap();
    assert_eq!(result.id, u32::MAX);
    assert_eq!(result.difficulty_id, i16::MIN);
    assert_eq!(result.max_level, i16::MIN);
    assert_eq!(result.max_passive_aura_level, u8::MAX);
    assert_eq!(result.base_level, i32::MIN);
    assert_eq!(result.spell_level, i32::MIN);
    assert_eq!(result.spell_id, u32::MAX);
}
