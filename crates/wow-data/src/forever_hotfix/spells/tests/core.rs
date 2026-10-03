//! Synthetic Source-width wire goldens; no client values or keys.
use crate::forever_spells::*;

#[test]
fn spell_name_writes_complete_metadata_order_and_widths() {
    let row = SpellNameRecord {
        id: u32::MAX,
        name: SpellText::from_locale(6, vec![0xFF, 0, b'a']).unwrap(),
    };
    assert_eq!(super::super::core::spell_name(&row, 6), [255, 0]);
}

#[test]
fn spell_effect_writes_complete_metadata_order_and_widths() {
    let row = SpellEffectRecord {
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
        effect_spell_class_mask: [u32::MAX; 4],
        implicit_target: [i16::MIN; 2],
        spell_id: u32::MAX,
    };
    assert_eq!(
        super::super::core::spell_effect(&row, 6),
        [
            0, 128, 0, 128, 0, 0, 0, 128, 255, 255, 255, 255, 52, 18, 192, 127, 0, 0, 0, 128, 0, 0,
            0, 128, 52, 18, 192, 127, 52, 18, 192, 127, 0, 0, 0, 128, 0, 0, 0, 128, 0, 0, 0, 128,
            52, 18, 192, 127, 52, 18, 192, 127, 52, 18, 192, 127, 0, 0, 0, 128, 52, 18, 192, 127,
            52, 18, 192, 127, 52, 18, 192, 127, 52, 18, 192, 127, 52, 18, 192, 127, 52, 18, 192,
            127, 52, 18, 192, 127, 0, 0, 0, 128, 0, 0, 0, 128, 0, 0, 0, 128, 0, 0, 0, 128, 255,
            255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
            255, 255, 255, 255, 255, 255, 0, 128, 0, 128, 255, 255, 255, 255
        ]
    );
}

#[test]
fn spell_misc_writes_complete_metadata_order_and_widths() {
    let row = SpellMiscRecord {
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
    };
    assert_eq!(
        super::super::core::spell_misc(&row, 6),
        [
            0, 0, 0, 128, 0, 0, 0, 128, 0, 0, 0, 128, 0, 0, 0, 128, 0, 0, 0, 128, 0, 0, 0, 128, 0,
            0, 0, 128, 0, 0, 0, 128, 0, 0, 0, 128, 0, 0, 0, 128, 0, 0, 0, 128, 0, 0, 0, 128, 0, 0,
            0, 128, 0, 0, 0, 128, 0, 0, 0, 128, 0, 0, 0, 128, 0, 0, 0, 128, 0, 128, 255, 255, 255,
            255, 255, 255, 255, 255, 255, 52, 18, 192, 127, 52, 18, 192, 127, 52, 18, 192, 127, 0,
            0, 0, 128, 0, 0, 0, 128, 0, 0, 0, 128, 0, 0, 0, 128, 0, 0, 0, 128, 0, 0, 0, 128, 255,
            255, 255, 255
        ]
    );
}

#[test]
fn spell_aura_options_writes_complete_metadata_order_and_widths() {
    let row = SpellAuraOptionsRecord {
        id: u32::MAX,
        difficulty_id: i16::MIN,
        cumulative_aura: u16::MAX,
        proc_category_recovery: i32::MIN,
        proc_chance: u8::MAX,
        proc_charges: i32::MIN,
        spell_procs_per_minute_id: u16::MAX,
        proc_type_mask: [i32::MIN; 2],
        spell_id: u32::MAX,
    };
    assert_eq!(
        super::super::core::spell_aura_options(&row, 6),
        [
            0, 128, 255, 255, 0, 0, 0, 128, 255, 0, 0, 0, 128, 255, 255, 0, 0, 0, 128, 0, 0, 0,
            128, 255, 255, 255, 255
        ]
    );
}

#[test]
fn spell_aura_restrictions_writes_complete_metadata_order_and_widths() {
    let row = SpellAuraRestrictionsRecord {
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
    };
    assert_eq!(
        super::super::core::spell_aura_restrictions(&row, 6),
        [
            0, 128, 0, 0, 0, 128, 0, 0, 0, 128, 0, 0, 0, 128, 0, 0, 0, 128, 0, 0, 0, 128, 0, 0, 0,
            128, 0, 0, 0, 128, 0, 0, 0, 128, 0, 128, 0, 128, 0, 128, 0, 128, 255, 255, 255, 255
        ]
    );
}

#[test]
fn spell_casting_requirements_writes_complete_metadata_order_and_widths() {
    let row = SpellCastingRequirementsRecord {
        id: u32::MAX,
        spell_id: i32::MIN,
        facing_caster_flags: i32::MIN,
        min_faction_id: u16::MAX,
        min_reputation: i32::MIN,
        required_areas_id: u16::MAX,
        required_aura_vision: u8::MAX,
        requires_spell_focus: u16::MAX,
    };
    assert_eq!(
        super::super::core::spell_casting_requirements(&row, 6),
        [
            0, 0, 0, 128, 0, 0, 0, 128, 255, 255, 0, 0, 0, 128, 255, 255, 255, 255, 255
        ]
    );
}

#[test]
fn spell_categories_writes_complete_metadata_order_and_widths() {
    let row = SpellCategoriesRecord {
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
    };
    assert_eq!(
        super::super::core::spell_categories(&row, 6),
        [
            0, 128, 0, 128, 128, 0, 0, 0, 128, 128, 128, 0, 0, 0, 128, 0, 128, 0, 128, 255, 255,
            255, 255
        ]
    );
}

#[test]
fn spell_class_options_writes_complete_metadata_order_and_widths() {
    let row = SpellClassOptionsRecord {
        id: u32::MAX,
        spell_id: i32::MIN,
        modal_next_spell: u32::MAX,
        spell_class_set: i32::MIN,
        spell_class_mask: [u32::MAX; 4],
    };
    assert_eq!(
        super::super::core::spell_class_options(&row, 6),
        [
            0, 0, 0, 128, 255, 255, 255, 255, 0, 0, 0, 128, 255, 255, 255, 255, 255, 255, 255, 255,
            255, 255, 255, 255, 255, 255, 255, 255
        ]
    );
}

#[test]
fn spell_cooldowns_writes_complete_metadata_order_and_widths() {
    let row = SpellCooldownsRecord {
        id: u32::MAX,
        difficulty_id: i16::MIN,
        category_recovery_time: i32::MIN,
        recovery_time: i32::MIN,
        start_recovery_time: i32::MIN,
        aura_spell_id: i32::MIN,
        spell_id: u32::MAX,
    };
    assert_eq!(
        super::super::core::spell_cooldowns(&row, 6),
        [
            0, 128, 0, 0, 0, 128, 0, 0, 0, 128, 0, 0, 0, 128, 0, 0, 0, 128, 255, 255, 255, 255
        ]
    );
}

#[test]
fn spell_equipped_items_writes_complete_metadata_order_and_widths() {
    let row = SpellEquippedItemsRecord {
        id: u32::MAX,
        spell_id: i32::MIN,
        equipped_item_class: i32::MIN,
        equipped_item_inv_types: i32::MIN,
        equipped_item_subclass: i32::MIN,
    };
    assert_eq!(
        super::super::core::spell_equipped_items(&row, 6),
        [0, 0, 0, 128, 0, 0, 0, 128, 0, 0, 0, 128, 0, 0, 0, 128]
    );
}

#[test]
fn spell_interrupts_writes_complete_metadata_order_and_widths() {
    let row = SpellInterruptsRecord {
        id: u32::MAX,
        difficulty_id: i16::MIN,
        interrupt_flags: i32::MIN,
        aura_interrupt_flags: [i32::MIN; 2],
        channel_interrupt_flags: [i32::MIN; 2],
        spell_id: u32::MAX,
    };
    assert_eq!(
        super::super::core::spell_interrupts(&row, 6),
        [
            0, 128, 0, 0, 0, 128, 0, 0, 0, 128, 0, 0, 0, 128, 0, 0, 0, 128, 0, 0, 0, 128, 255, 255,
            255, 255
        ]
    );
}

#[test]
fn spell_label_writes_complete_metadata_order_and_widths() {
    let row = SpellLabelRecord {
        id: u32::MAX,
        label_id: u32::MAX,
        spell_id: u32::MAX,
    };
    assert_eq!(
        super::super::core::spell_label(&row, 6),
        [255, 255, 255, 255, 255, 255, 255, 255]
    );
}

#[test]
fn spell_levels_writes_complete_metadata_order_and_widths() {
    let row = SpellLevelsRecord {
        id: u32::MAX,
        difficulty_id: i16::MIN,
        max_level: i16::MIN,
        max_passive_aura_level: u8::MAX,
        base_level: i32::MIN,
        spell_level: i32::MIN,
        spell_id: u32::MAX,
    };
    assert_eq!(
        super::super::core::spell_levels(&row, 6),
        [
            0, 128, 0, 128, 255, 0, 0, 0, 128, 0, 0, 0, 128, 255, 255, 255, 255
        ]
    );
}
