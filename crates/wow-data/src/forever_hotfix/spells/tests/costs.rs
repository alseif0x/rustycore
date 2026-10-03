//! Synthetic Source-width wire goldens; no client values or keys.
use crate::forever_spells::*;

#[test]
fn spell_empower_writes_complete_metadata_order_and_widths() {
    let row = SpellEmpowerRecord {
        id: u32::MAX,
        spell_id: i32::MIN,
        unused1000: i32::MIN,
    };
    assert_eq!(
        super::super::costs::spell_empower(&row, 6),
        [255, 255, 255, 255, 0, 0, 0, 128, 0, 0, 0, 128]
    );
}

#[test]
fn spell_empower_stage_writes_complete_metadata_order_and_widths() {
    let row = SpellEmpowerStageRecord {
        id: u32::MAX,
        stage: i32::MIN,
        duration_ms: i32::MIN,
        spell_empower_id: u32::MAX,
    };
    assert_eq!(
        super::super::costs::spell_empower_stage(&row, 6),
        [0, 0, 0, 128, 0, 0, 0, 128, 255, 255, 255, 255]
    );
}

#[test]
fn spell_power_writes_complete_metadata_order_and_widths() {
    let row = SpellPowerRecord {
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
    };
    assert_eq!(
        super::super::costs::spell_power(&row, 6),
        [
            255, 255, 255, 255, 255, 0, 0, 0, 128, 0, 0, 0, 128, 0, 0, 0, 128, 255, 255, 255, 255,
            0, 0, 0, 128, 52, 18, 192, 127, 52, 18, 192, 127, 52, 18, 192, 127, 52, 18, 192, 127,
            128, 0, 0, 0, 128, 255, 255, 255, 255, 255, 255, 255, 255
        ]
    );
}

#[test]
fn spell_power_difficulty_writes_complete_metadata_order_and_widths() {
    let row = SpellPowerDifficultyRecord {
        id: u32::MAX,
        difficulty_id: i16::MIN,
        order_index: u8::MAX,
    };
    assert_eq!(
        super::super::costs::spell_power_difficulty(&row, 6),
        [0, 128, 255]
    );
}

#[test]
fn spell_reagents_writes_complete_metadata_order_and_widths() {
    let row = SpellReagentsRecord {
        id: u32::MAX,
        spell_id: i32::MIN,
        reagent: [i32::MIN; 8],
        reagent_count: [i16::MIN; 8],
        reagent_recraft_count: [i16::MIN; 8],
        reagent_source: [u8::MAX; 8],
    };
    assert_eq!(
        super::super::costs::spell_reagents(&row, 6),
        [
            0, 0, 0, 128, 0, 0, 0, 128, 0, 0, 0, 128, 0, 0, 0, 128, 0, 0, 0, 128, 0, 0, 0, 128, 0,
            0, 0, 128, 0, 0, 0, 128, 0, 0, 0, 128, 0, 128, 0, 128, 0, 128, 0, 128, 0, 128, 0, 128,
            0, 128, 0, 128, 0, 128, 0, 128, 0, 128, 0, 128, 0, 128, 0, 128, 0, 128, 0, 128, 255,
            255, 255, 255, 255, 255, 255, 255
        ]
    );
}

#[test]
fn spell_reagents_currency_writes_complete_metadata_order_and_widths() {
    let row = SpellReagentsCurrencyRecord {
        id: u32::MAX,
        spell_id: u32::MAX,
        currency_types_id: i32::MIN,
        currency_count: i32::MIN,
        override_recraft_currency_count: i32::MIN,
        order_source: u8::MAX,
    };
    assert_eq!(
        super::super::costs::spell_reagents_currency(&row, 6),
        [
            255, 255, 255, 255, 0, 0, 0, 128, 0, 0, 0, 128, 0, 0, 0, 128, 255
        ]
    );
}

#[test]
fn spell_scaling_writes_complete_metadata_order_and_widths() {
    let row = SpellScalingRecord {
        id: u32::MAX,
        spell_id: i32::MIN,
        min_scaling_level: u32::MAX,
        max_scaling_level: u32::MAX,
    };
    assert_eq!(
        super::super::costs::spell_scaling(&row, 6),
        [0, 0, 0, 128, 255, 255, 255, 255, 255, 255, 255, 255]
    );
}

#[test]
fn spell_shapeshift_writes_complete_metadata_order_and_widths() {
    let row = SpellShapeshiftRecord {
        id: u32::MAX,
        spell_id: i32::MIN,
        stance_bar_order: i8::MIN,
        shapeshift_exclude: [i32::MIN; 2],
        shapeshift_mask: [i32::MIN; 2],
    };
    assert_eq!(
        super::super::costs::spell_shapeshift(&row, 6),
        [
            0, 0, 0, 128, 128, 0, 0, 0, 128, 0, 0, 0, 128, 0, 0, 0, 128, 0, 0, 0, 128
        ]
    );
}

#[test]
fn spell_target_restrictions_writes_complete_metadata_order_and_widths() {
    let row = SpellTargetRestrictionsRecord {
        id: u32::MAX,
        difficulty_id: i16::MIN,
        cone_degrees: f32::from_bits(0x7FC0_1234),
        max_targets: u8::MAX,
        max_target_level: u32::MAX,
        target_creature_type: i16::MIN,
        targets: i32::MIN,
        width: f32::from_bits(0x7FC0_1234),
        spell_id: u32::MAX,
    };
    assert_eq!(
        super::super::costs::spell_target_restrictions(&row, 6),
        [
            0, 128, 52, 18, 192, 127, 255, 255, 255, 255, 255, 0, 128, 0, 0, 0, 128, 52, 18, 192,
            127, 255, 255, 255, 255
        ]
    );
}

#[test]
fn spell_totems_writes_complete_metadata_order_and_widths() {
    let row = SpellTotemsRecord {
        id: u32::MAX,
        spell_id: i32::MIN,
        required_totem_category_id: [u16::MAX; 2],
        totem: [i32::MIN; 2],
    };
    assert_eq!(
        super::super::costs::spell_totems(&row, 6),
        [0, 0, 0, 128, 255, 255, 255, 255, 0, 0, 0, 128, 0, 0, 0, 128]
    );
}

#[test]
fn spell_x_spell_visual_writes_complete_metadata_order_and_widths() {
    let row = SpellXSpellVisualRecord {
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
    };
    assert_eq!(
        super::super::costs::spell_x_spell_visual(&row, 6),
        [
            255, 255, 255, 255, 0, 128, 255, 255, 255, 255, 52, 18, 192, 127, 0, 0, 0, 128, 0, 0,
            0, 128, 0, 0, 0, 128, 0, 0, 0, 128, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
            255, 255, 255, 255, 255, 255
        ]
    );
}
