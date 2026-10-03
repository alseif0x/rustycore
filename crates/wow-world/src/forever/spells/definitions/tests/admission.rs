use super::*;

#[test]
fn any_regular_difficulty_rejects_server_effects_and_name_only_identity_bans_server_main() {
    let mut existing = client_rows().spell_effects.remove(0);
    existing.difficulty_id = 2;
    let raw = catalog(SpellRecords {
        spell_names: vec![named(1), named(2)],
        spell_effects: vec![existing],
        ..Default::default()
    });
    let mut regular = server_effect(1, 0);
    regular.effect_index = -1;
    let result = SpellLoadPlan::build(raw)
        .unwrap()
        .with_server_spells(ServerSpellRows {
            spells: vec![server_spell(1, 0), server_spell(2, 0)],
            effects: vec![regular, server_effect(2, 0)],
        })
        .unwrap();
    assert_eq!(result.len(), 1);
    assert!(result.get_exact(1, 2).is_some());
    assert!(result.get_exact(1, 0).is_none());
    assert!(result.get_exact(2, 0).is_none());
    let counts = result.server_counts();
    assert_eq!(counts.skipped_regular_effects, 1);
    assert_eq!(counts.skipped_invalid_effects, 0);
    assert_eq!(counts.rejected_client_names, 2);
    assert_eq!(counts.definitions_added, 0);
    assert_eq!(counts.orphan_effect_groups, 1);
}

#[test]
fn server_effect_upper_bounds_skip_rows_and_last_supported_values_remain_inputs() {
    let mut rows = Vec::new();
    let mut too_many = server_effect(7, 0);
    too_many.effect_index = 32;
    rows.push(too_many);
    let mut unsupported_effect = server_effect(7, 0);
    unsupported_effect.effect = 361;
    rows.push(unsupported_effect);
    let mut signed_effect = server_effect(7, 0);
    signed_effect.effect = -1;
    rows.push(signed_effect);
    let mut unsupported_aura = server_effect(7, 0);
    unsupported_aura.effect_aura = 665;
    rows.push(unsupported_aura);
    for target in 0..2 {
        let mut unsupported_target = server_effect(7, 0);
        unsupported_target.implicit_target[target] = 153;
        rows.push(unsupported_target);
    }
    let mut supported = server_effect(7, 0);
    supported.effect_index = 31;
    supported.effect = 360;
    supported.effect_aura = 664;
    supported.implicit_target = [152; 2];
    rows.push(supported);
    let result = build(ServerSpellRows {
        spells: vec![server_spell(7, 0)],
        effects: rows,
    });
    assert_eq!(result.server_counts().skipped_invalid_effects, 6);
    assert_eq!(result.get_exact(7, 0).unwrap().effect_count(), 32);
    assert_eq!(
        result
            .get_exact(7, 0)
            .unwrap()
            .effect(31)
            .unwrap()
            .values()
            .effect,
        360
    );
}

#[test]
fn otherwise_admitted_negative_index_aura_and_target_fail_before_seed_publication() {
    let mut negative_index = server_effect(7, 0);
    negative_index.effect_index = -1;
    let mut negative_aura = server_effect(7, 0);
    negative_aura.effect_aura = -1;
    let mut negative_target = server_effect(7, 0);
    negative_target.implicit_target[0] = -1;
    for (row, error) in [
        (negative_index, SpellDefinitionError::EffectIndex),
        (negative_aura, SpellDefinitionError::NegativeAura),
        (
            negative_target,
            SpellDefinitionError::NegativeImplicitTarget,
        ),
    ] {
        let raw = catalog(client_rows());
        let result = SpellLoadPlan::build(raw.clone())
            .unwrap()
            .with_server_spells(ServerSpellRows {
                spells: vec![server_spell(7, 0)],
                effects: vec![row],
            });
        assert!(matches!(result, Err(actual) if actual == error));
        assert!(raw.spell_name(7).is_none());
        assert_eq!(raw.counts()[0].1, 1);
    }
}

#[test]
fn source_skip_checks_precede_negative_admission_and_never_publish_a_bad_effect() {
    let mut unsupported = server_effect(7, 0);
    unsupported.effect = 361;
    unsupported.effect_index = -1;
    unsupported.effect_aura = -1;
    unsupported.implicit_target = [-1; 2];
    let mut missing_difficulty = server_effect(7, 9);
    missing_difficulty.effect_index = -1;
    let result = build(ServerSpellRows {
        spells: vec![server_spell(7, 0)],
        effects: vec![unsupported, missing_difficulty],
    });
    assert_eq!(result.get_exact(7, 0).unwrap().effect_count(), 0);
    assert_eq!(result.server_counts().skipped_invalid_effects, 1);
    assert_eq!(result.server_counts().skipped_missing_difficulty_effects, 1);
}

#[test]
fn signed_sql_difficulty_narrows_only_at_enum_boundary_and_preserves_known_parent_lookup() {
    let raw = catalog(SpellRecords {
        difficulties: vec![difficulty(2, 0), difficulty(-1, 0)],
        ..Default::default()
    });
    let result = SpellLoadPlan::build(raw)
        .unwrap()
        .with_server_spells(ServerSpellRows {
            spells: vec![server_spell(7, -65534), server_spell(8, -1)],
            effects: vec![server_effect(7, -65534), server_effect(8, -1)],
        })
        .unwrap();
    assert_eq!(result.get_exact(7, 2).unwrap().effect_count(), 1);
    assert_eq!(result.get_exact(8, -1).unwrap().effect_count(), 1);
    assert_eq!(result.server_counts().skipped_missing_difficulty_effects, 0);
}

#[test]
fn lookup_returns_exact_then_first_existing_fallback_without_default_guessing() {
    let raw = catalog(SpellRecords {
        difficulties: vec![difficulty(2, 3), difficulty(3, 0), difficulty(-1, 2)],
        ..Default::default()
    });
    let result = SpellLoadPlan::build(raw)
        .unwrap()
        .with_server_spells(ServerSpellRows {
            spells: vec![server_spell(7, 0), server_spell(7, 3), server_spell(7, -1)],
            effects: vec![],
        })
        .unwrap();
    assert_eq!(result.get(7, -1).unwrap().unwrap().difficulty(), -1);
    assert_eq!(result.get(7, 2).unwrap().unwrap().difficulty(), 3);
    assert_eq!(result.get(7, 0).unwrap().unwrap().difficulty(), 0);
    assert!(result.get(7, 9).unwrap().is_none());
    assert!(result.get(8, 2).unwrap().is_none());
    assert!(result.get_exact(7, 2).is_none());
}

#[test]
fn lookup_cycles_fail_only_if_no_definition_was_found_before_the_cycle() {
    let raw = catalog(SpellRecords {
        difficulties: vec![difficulty(2, 3), difficulty(3, 2)],
        ..Default::default()
    });
    let result = SpellLoadPlan::build(raw)
        .unwrap()
        .with_server_spells(ServerSpellRows {
            spells: vec![server_spell(7, 3)],
            effects: vec![],
        })
        .unwrap();
    assert_eq!(result.get(7, 3).unwrap().unwrap().difficulty(), 3);
    assert_eq!(result.get(7, 2).unwrap().unwrap().difficulty(), 3);
    assert!(matches!(
        result.get(8, 2),
        Err(SpellDefinitionError::DifficultyCycle)
    ));
}

#[test]
fn empty_complete_batches_are_empty_not_server_definitions_or_successful_players() {
    let result = build(ServerSpellRows::default());
    assert!(result.is_empty());
    assert_eq!(result.records().count(), 0);
    assert_eq!(result.server_counts(), ServerSpellCounts::default());
    assert!(result.get(1, 0).unwrap().is_none());
}
