use super::*;

#[test]
fn classic_unknown_enum_guard_counts_all_fields_without_manufacturing_effects() {
    let mut rows = named(&[1, 2]);
    let mut unknown = effect(1, 1, 0, 0);
    unknown.effect = 361;
    unknown.effect_aura = 665;
    unknown.implicit_target = [153, 154];
    let mut negative_aura = effect(2, 1, 0, 1);
    negative_aura.effect_aura = -1;
    let mut last_supported = effect(3, 2, 0, 31);
    last_supported.effect = 360;
    last_supported.effect_aura = 664;
    last_supported.implicit_target = [152, 152];
    let mut modifier = effect(4, 2, 0, 0);
    modifier.effect_aura = 107;
    modifier.effect_misc_value[0] = 41;
    rows.spell_effects = vec![unknown, negative_aura, last_supported, modifier];
    let result = plan(rows);
    assert!(result.get(1, 0).is_none());
    assert_eq!(result.get(2, 0).unwrap().effect_slots()[31].unwrap().id, 3);
    assert_eq!(result.get(2, 0).unwrap().effect_slots()[0].unwrap().id, 4);
    let counts = result.counts();
    assert_eq!(counts.skipped_effects, 2);
    assert_eq!(counts.unknown_effect_values, 1);
    assert_eq!(counts.unknown_aura_values, 2);
    assert_eq!(counts.unknown_target_values, 2);
    assert_eq!(counts.invalid_modifier_types, 1);
}

#[test]
fn invalid_array_indices_and_negative_target_abort_publication() {
    for index in [-1, 32, i32::MAX] {
        let mut rows = named(&[1]);
        let mut row = effect(1, 1, 0, index);
        // The source index assertion precedes unknown-enum skipping.
        row.effect = u32::MAX;
        rows.spell_effects = vec![row];
        assert!(matches!(
            SpellLoadPlan::build(catalog(rows)),
            Err(SpellLoadError::EffectIndex)
        ));
    }
    for override_index in [false, true] {
        let mut rows = named(&[1]);
        let mut power = spell_power(1, 1);
        power.order_index = if override_index { 0 } else { 5 };
        rows.spell_powers = vec![power];
        if override_index {
            rows.spell_power_difficulties = vec![SpellPowerDifficultyRecord {
                id: 1,
                difficulty_id: 2,
                order_index: 255,
            }];
        }
        assert!(matches!(
            SpellLoadPlan::build(catalog(rows)),
            Err(SpellLoadError::PowerIndex)
        ));
    }
    let mut rows = named(&[1]);
    let mut row = effect(1, 1, 0, 0);
    row.implicit_target = [-1, 0];
    rows.spell_effects = vec![row];
    assert!(matches!(
        SpellLoadPlan::build(catalog(rows)),
        Err(SpellLoadError::NegativeImplicitTarget)
    ));
}

#[test]
fn difficulty_cycles_fail_closed_but_unnamed_helpers_are_not_resolved() {
    for difficulties in [
        vec![difficulty(2, 2)],
        vec![difficulty(2, 3), difficulty(3, 2)],
    ] {
        let mut rows = named(&[1]);
        rows.difficulties = difficulties.clone();
        rows.spell_effects = vec![effect(1, 1, 2, 0)];
        assert!(matches!(
            SpellLoadPlan::build(catalog(rows)),
            Err(SpellLoadError::DifficultyCycle)
        ));
        let rows = SpellRecords {
            difficulties,
            spell_effects: vec![effect(1, 1, 2, 0)],
            ..Default::default()
        };
        let result = plan(rows);
        assert_eq!(result.records().count(), 0);
        assert_eq!(result.counts().unnamed_helpers, 1);
    }
}

#[test]
fn signed_difficulty_lookup_keeps_cpp_sign_extension_and_skipped_effects_do_not_create_helpers() {
    let mut rows = named(&[1]);
    rows.difficulties = vec![difficulty(-1, 0)];
    rows.spell_effects = vec![effect(1, 1, -1, 1), effect(2, 1, 0, 0)];
    let mut unsupported = effect(3, 1, 8, 0);
    unsupported.effect = u32::MAX;
    unsupported.implicit_target = [-1, 0];
    rows.spell_effects.push(unsupported);
    let result = plan(rows);
    assert_eq!(result.get(1, -1).unwrap().effect_slots()[0].unwrap().id, 2);
    assert!(result.get(1, 8).is_none());
    assert_eq!(result.counts().skipped_effects, 1);
}
