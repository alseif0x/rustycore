use super::*;

fn pool(current: i32, max: i32) -> Unit {
    let mut unit = Unit::new(true);
    unit.set_power_index(PowerType::Mana, Some(0));
    unit.set_max_power(PowerType::Mana, max);
    unit.set_power(PowerType::Mana, current);
    unit.clear_unit_data_changes();
    unit
}

#[test]
fn energize_reports_requested_and_applied_at_the_upper_cap() {
    let mut unit = pool(95, 100);
    assert_eq!(
        unit.energize_spell_power(PowerType::Mana, SpellPowerAmount::Flat(50)),
        Some(SpellPowerGain { requested: 50, applied: 5 }),
    );
    assert_eq!(unit.get_power(PowerType::Mana), 100);
    assert!(unit.unit_data_changes_mask().is_set(UNIT_DATA_POWER_PARENT_BIT));
    assert!(unit.unit_data_changes_mask().is_set(UNIT_DATA_POWER_FIRST_BIT));
}

#[test]
fn percent_energize_uses_wide_product_and_signed_truncation() {
    let mut unit = pool(10, 101);
    assert_eq!(
        unit.energize_spell_power(PowerType::Mana, SpellPowerAmount::Percent(-5)),
        Some(SpellPowerGain { requested: -5, applied: -5 }),
    );
    assert_eq!(unit.get_power(PowerType::Mana), 5);
    unit.set_max_power(PowerType::Mana, i32::MAX);
    unit.set_power(PowerType::Mana, i32::MAX - 1);
    assert_eq!(
        unit.energize_spell_power(PowerType::Mana, SpellPowerAmount::Percent(i32::MAX)),
        Some(SpellPowerGain { requested: i32::MAX, applied: 1 }),
    );
    assert_eq!(unit.get_power(PowerType::Mana), i32::MAX);
}

#[test]
fn energize_refuses_missing_or_nonpositive_max_without_writing() {
    let mut unit = Unit::new(true);
    assert_eq!(unit.energize_spell_power(PowerType::Mana, SpellPowerAmount::Flat(20)), None);
    assert!(!unit.unit_data_changes_mask().is_set(UNIT_DATA_POWER_PARENT_BIT));
    let mut unit = pool(0, 0);
    assert_eq!(unit.energize_spell_power(PowerType::Mana, SpellPowerAmount::Percent(20)), None);
    assert!(!unit.unit_data_changes_mask().is_set(UNIT_DATA_POWER_PARENT_BIT));
    unit.data.max_power[0] = -1;
    assert_eq!(unit.energize_spell_power(PowerType::Mana, SpellPowerAmount::Flat(20)), None);
    assert_eq!(unit.get_power(PowerType::Mana), 0);
}

#[test]
fn zero_and_negative_energize_preserve_the_original_clamp() {
    let mut unit = pool(-10, 100);
    assert_eq!(
        unit.energize_spell_power(PowerType::Mana, SpellPowerAmount::Flat(0)),
        Some(SpellPowerGain { requested: 0, applied: 10 }),
    );
    assert_eq!(unit.get_power(PowerType::Mana), 0);
    unit.set_power(PowerType::Mana, 10);
    assert_eq!(
        unit.energize_spell_power(PowerType::Mana, SpellPowerAmount::Flat(-50)),
        Some(SpellPowerGain { requested: -50, applied: -10 }),
    );
    assert_eq!(unit.get_power(PowerType::Mana), 0);
}

#[test]
fn drain_mismatch_does_not_write_and_matching_drain_marks_power() {
    let mut unit = pool(30, 100);
    assert_eq!(unit.drain_spell_power(PowerType::Mana, PowerType::Energy, 20), None);
    assert_eq!(unit.get_power(PowerType::Mana), 30);
    assert!(!unit.unit_data_changes_mask().is_set(UNIT_DATA_POWER_PARENT_BIT));
    assert_eq!(unit.drain_spell_power(PowerType::Mana, PowerType::Mana, 50), Some(30));
    assert_eq!(unit.get_power(PowerType::Mana), 0);
    assert!(unit.unit_data_changes_mask().is_set(UNIT_DATA_POWER_FIRST_BIT));
}

#[test]
fn raw_drain_remains_distinct_from_the_setter_delta() {
    let mut unit = pool(0, 100);
    // A loaded inconsistent pool is retained as an input to the old operation.
    unit.data.power[0] = 150;
    assert_eq!(unit.drain_spell_power(PowerType::Mana, PowerType::Mana, 10), Some(10));
    assert_eq!(unit.get_power(PowerType::Mana), 100);
    assert_eq!(150 - unit.get_power(PowerType::Mana), 50);
}

#[test]
fn zero_drain_keeps_missing_index_and_negative_pool_semantics() {
    let mut unit = Unit::new(true);
    assert_eq!(unit.drain_spell_power(PowerType::Mana, PowerType::Mana, 20), Some(0));
    assert!(!unit.unit_data_changes_mask().is_set(UNIT_DATA_POWER_PARENT_BIT));
    let mut unit = pool(-10, 100);
    assert_eq!(unit.drain_spell_power(PowerType::Mana, PowerType::Mana, 20), Some(0));
    assert_eq!(unit.get_power(PowerType::Mana), 0);
    assert!(unit.unit_data_changes_mask().is_set(UNIT_DATA_POWER_FIRST_BIT));
}

#[test]
fn zero_energize_in_a_valid_pool_preserves_a_clean_change_mask() {
    let mut unit = pool(25, 100);
    assert_eq!(
        unit.energize_spell_power(PowerType::Mana, SpellPowerAmount::Flat(0)),
        Some(SpellPowerGain { requested: 0, applied: 0 }),
    );
    assert_eq!(unit.get_power(PowerType::Mana), 25);
    assert!(!unit.unit_data_changes_mask().is_set(UNIT_DATA_POWER_PARENT_BIT));
}
