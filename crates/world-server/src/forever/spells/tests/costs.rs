use wow_persistence::forever::spells::{
    SpellEmpowerRow, SpellEmpowerStageRow, SpellPowerDifficultyRow, SpellPowerRow,
    SpellReagentsCurrencyRow, SpellReagentsRow, SpellScalingRow, SpellShapeshiftRow,
    SpellTargetRestrictionsRow, SpellTotemsRow, SpellXSpellVisualRow,
};

#[test]
fn spell_empower_moves_all_source_columns_without_loss() {
    let result = super::super::costs::spell_empower(SpellEmpowerRow {
        id: u32::MAX,
        spell_id: i32::MIN,
        unused1000: i32::MIN,
    })
    .unwrap();
    assert_eq!(result.id, u32::MAX);
    assert_eq!(result.spell_id, i32::MIN);
    assert_eq!(result.unused1000, i32::MIN);
}

#[test]
fn spell_empower_stage_moves_all_source_columns_without_loss() {
    let result = super::super::costs::spell_empower_stage(SpellEmpowerStageRow {
        id: u32::MAX,
        stage: i32::MIN,
        duration_ms: i32::MIN,
        spell_empower_id: u32::MAX,
    })
    .unwrap();
    assert_eq!(result.id, u32::MAX);
    assert_eq!(result.stage, i32::MIN);
    assert_eq!(result.duration_ms, i32::MIN);
    assert_eq!(result.spell_empower_id, u32::MAX);
}

#[test]
fn spell_power_moves_all_source_columns_without_loss() {
    let result = super::super::costs::spell_power(SpellPowerRow {
        id: u32::MAX,
        order_index: u8::MAX,
        mana_cost: i32::MIN,
        mana_cost_per_level: i32::MIN,
        mana_per_second: i32::MIN,
        power_display_id: u32::MAX,
        alt_power_bar_id: i32::MIN,
        power_cost_pct: f32::from_bits(0x7FC0_1234),
        power_cost_max_pct: f32::from_bits(0x7FC0_1234),
        optional_cost_pct: f32::from_bits(0x7FC0_1234),
        power_pct_per_second: f32::from_bits(0x7FC0_1234),
        power_type: i8::MIN,
        required_aura_spell_id: i32::MIN,
        optional_cost: u32::MAX,
        spell_id: u32::MAX,
    })
    .unwrap();
    assert_eq!(result.id, u32::MAX);
    assert_eq!(result.order_index, u8::MAX);
    assert_eq!(result.mana_cost, i32::MIN);
    assert_eq!(result.mana_cost_per_level, i32::MIN);
    assert_eq!(result.mana_per_second, i32::MIN);
    assert_eq!(result.power_display_id, u32::MAX);
    assert_eq!(result.alt_power_bar_id, i32::MIN);
    assert_eq!(result.power_cost_pct.to_bits(), 0x7FC0_1234);
    assert_eq!(result.power_cost_max_pct.to_bits(), 0x7FC0_1234);
    assert_eq!(result.optional_cost_pct.to_bits(), 0x7FC0_1234);
    assert_eq!(result.power_pct_per_second.to_bits(), 0x7FC0_1234);
    assert_eq!(result.power_type, i8::MIN);
    assert_eq!(result.required_aura_spell_id, i32::MIN);
    assert_eq!(result.optional_cost, u32::MAX);
    assert_eq!(result.spell_id, u32::MAX);
}

#[test]
fn spell_power_difficulty_moves_all_source_columns_without_loss() {
    let result = super::super::costs::spell_power_difficulty(SpellPowerDifficultyRow {
        id: u32::MAX,
        difficulty_id: i16::MIN,
        order_index: u8::MAX,
    })
    .unwrap();
    assert_eq!(result.id, u32::MAX);
    assert_eq!(result.difficulty_id, i16::MIN);
    assert_eq!(result.order_index, u8::MAX);
}

#[test]
fn spell_reagents_moves_all_source_columns_without_loss() {
    let result = super::super::costs::spell_reagents(SpellReagentsRow {
        id: u32::MAX,
        spell_id: i32::MIN,
        reagent: [i32::MIN; 8],
        reagent_count: [i16::MIN; 8],
        reagent_recraft_count: [i16::MIN; 8],
        reagent_source: [u8::MAX; 8],
    })
    .unwrap();
    assert_eq!(result.id, u32::MAX);
    assert_eq!(result.spell_id, i32::MIN);
    assert_eq!(result.reagent, [i32::MIN; 8]);
    assert_eq!(result.reagent_count, [i16::MIN; 8]);
    assert_eq!(result.reagent_recraft_count, [i16::MIN; 8]);
    assert_eq!(result.reagent_source, [u8::MAX; 8]);
}

#[test]
fn spell_reagents_currency_moves_all_source_columns_without_loss() {
    let result = super::super::costs::spell_reagents_currency(SpellReagentsCurrencyRow {
        id: u32::MAX,
        spell_id: u32::MAX,
        currency_types_id: i32::MIN,
        currency_count: i32::MIN,
        override_recraft_currency_count: i32::MIN,
        order_source: u8::MAX,
    })
    .unwrap();
    assert_eq!(result.id, u32::MAX);
    assert_eq!(result.spell_id, u32::MAX);
    assert_eq!(result.currency_types_id, i32::MIN);
    assert_eq!(result.currency_count, i32::MIN);
    assert_eq!(result.override_recraft_currency_count, i32::MIN);
    assert_eq!(result.order_source, u8::MAX);
}

#[test]
fn spell_scaling_moves_all_source_columns_without_loss() {
    let result = super::super::costs::spell_scaling(SpellScalingRow {
        id: u32::MAX,
        spell_id: i32::MIN,
        min_scaling_level: u32::MAX,
        max_scaling_level: u32::MAX,
    })
    .unwrap();
    assert_eq!(result.id, u32::MAX);
    assert_eq!(result.spell_id, i32::MIN);
    assert_eq!(result.min_scaling_level, u32::MAX);
    assert_eq!(result.max_scaling_level, u32::MAX);
}

#[test]
fn spell_shapeshift_moves_all_source_columns_without_loss() {
    let result = super::super::costs::spell_shapeshift(SpellShapeshiftRow {
        id: u32::MAX,
        spell_id: i32::MIN,
        stance_bar_order: i8::MIN,
        shapeshift_exclude: [i32::MIN; 2],
        shapeshift_mask: [i32::MIN; 2],
    })
    .unwrap();
    assert_eq!(result.id, u32::MAX);
    assert_eq!(result.spell_id, i32::MIN);
    assert_eq!(result.stance_bar_order, i8::MIN);
    assert_eq!(result.shapeshift_exclude, [i32::MIN; 2]);
    assert_eq!(result.shapeshift_mask, [i32::MIN; 2]);
}

#[test]
fn spell_target_restrictions_moves_all_source_columns_without_loss() {
    let result = super::super::costs::spell_target_restrictions(SpellTargetRestrictionsRow {
        id: u32::MAX,
        difficulty_id: i16::MIN,
        cone_degrees: f32::from_bits(0x7FC0_1234),
        max_targets: u8::MAX,
        max_target_level: u32::MAX,
        target_creature_type: i16::MIN,
        targets: i32::MIN,
        width: f32::from_bits(0x7FC0_1234),
        spell_id: u32::MAX,
    })
    .unwrap();
    assert_eq!(result.id, u32::MAX);
    assert_eq!(result.difficulty_id, i16::MIN);
    assert_eq!(result.cone_degrees.to_bits(), 0x7FC0_1234);
    assert_eq!(result.max_targets, u8::MAX);
    assert_eq!(result.max_target_level, u32::MAX);
    assert_eq!(result.target_creature_type, i16::MIN);
    assert_eq!(result.targets, i32::MIN);
    assert_eq!(result.width.to_bits(), 0x7FC0_1234);
    assert_eq!(result.spell_id, u32::MAX);
}

#[test]
fn spell_totems_moves_all_source_columns_without_loss() {
    let result = super::super::costs::spell_totems(SpellTotemsRow {
        id: u32::MAX,
        spell_id: i32::MIN,
        required_totem_category_id: [u16::MAX; 2],
        totem: [i32::MIN; 2],
    })
    .unwrap();
    assert_eq!(result.id, u32::MAX);
    assert_eq!(result.spell_id, i32::MIN);
    assert_eq!(result.required_totem_category_id, [u16::MAX; 2]);
    assert_eq!(result.totem, [i32::MIN; 2]);
}

#[test]
fn spell_x_spell_visual_moves_all_source_columns_without_loss() {
    let result = super::super::costs::spell_x_spell_visual(SpellXSpellVisualRow {
        id: u32::MAX,
        difficulty_id: i16::MIN,
        spell_visual_id: u32::MAX,
        probability: f32::from_bits(0x7FC0_1234),
        flags: i32::MIN,
        priority: i32::MIN,
        spell_icon_file_id: i32::MIN,
        active_icon_file_id: i32::MIN,
        viewer_unit_condition_id: u16::MAX,
        viewer_player_condition_id: u32::MAX,
        caster_unit_condition_id: u16::MAX,
        caster_player_condition_id: u32::MAX,
        spell_id: u32::MAX,
    })
    .unwrap();
    assert_eq!(result.id, u32::MAX);
    assert_eq!(result.difficulty_id, i16::MIN);
    assert_eq!(result.spell_visual_id, u32::MAX);
    assert_eq!(result.probability.to_bits(), 0x7FC0_1234);
    assert_eq!(result.flags, i32::MIN);
    assert_eq!(result.priority, i32::MIN);
    assert_eq!(result.spell_icon_file_id, i32::MIN);
    assert_eq!(result.active_icon_file_id, i32::MIN);
    assert_eq!(result.viewer_unit_condition_id, u16::MAX);
    assert_eq!(result.viewer_player_condition_id, u32::MAX);
    assert_eq!(result.caster_unit_condition_id, u16::MAX);
    assert_eq!(result.caster_player_condition_id, u32::MAX);
    assert_eq!(result.spell_id, u32::MAX);
}
