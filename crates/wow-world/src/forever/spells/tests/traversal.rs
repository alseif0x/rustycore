//! Replay input metadata is not a traversal or a second spell authority.
use super::*;
use wow_persistence::forever::spells::server::ServerSpellRows;

#[test]
fn helper_input_trace_retains_first_storage_insertion_not_sorted_or_named_only_order() {
    let mut rows = named(&[9, 2, 5]);
    rows.spell_effects = vec![effect(1, 9, 0, 0), effect(2, 2, 0, 0), effect(3, 9, 0, 1)];
    rows.spell_aura_options = vec![spell_aura_options(1, 5), spell_aura_options(2, 9)];
    rows.spell_misc = vec![spell_misc(1, 77), spell_misc(2, 2)];
    let result = plan(rows)
        .with_server_spells(ServerSpellRows::default())
        .unwrap();
    let inputs = result.traversal_inputs().unwrap();
    assert_eq!(
        inputs.helper_insertions(),
        &[(9, 0), (2, 0), (5, 0), (77, 0)]
    );
    assert_eq!(
        inputs.client_keys().collect::<Vec<_>>(),
        vec![(2, 0), (5, 0), (9, 0)]
    );
    assert!(inputs.server_requests().is_empty());
    assert_eq!(result.client_counts().unnamed_helpers, 1);
    assert_eq!(result.len(), 3); // trace's unnamed key did not manufacture a definition
}

#[test]
fn unknown_effect_admission_precedes_trace_and_later_valid_join_is_first_insertion() {
    let mut rows = named(&[9, 2]);
    let mut unknown = effect(1, 9, 0, 0);
    unknown.effect = 361;
    rows.spell_effects = vec![unknown, effect(2, 2, 0, 0)];
    rows.spell_misc = vec![spell_misc(1, 9)];
    let result = plan(rows)
        .with_server_spells(ServerSpellRows::default())
        .unwrap();
    let inputs = result.traversal_inputs().unwrap();
    assert_eq!(inputs.helper_insertions(), &[(2, 0), (9, 0)]);
    assert_eq!(result.client_counts().skipped_effects, 1);
}

#[test]
fn all_twenty_one_helper_families_retain_source_join_order_including_unnamed_keys() {
    // SpellMgr.cpp:2504-2706. Deliberately descending identities distinguish
    // source join order from the canonical BTree lookup order. Every helper is
    // unnamed: none may disappear from the hash replay's insertion history.
    let mut rows = SpellRecords::default();
    rows.spell_effects = vec![spell_effect(1, 121)];
    rows.spell_aura_options = vec![spell_aura_options(1, 120)];
    rows.spell_aura_restrictions = vec![spell_aura_restrictions(1, 119)];
    rows.spell_casting_requirements = vec![spell_casting_requirements(1, 118)];
    rows.spell_categories = vec![spell_categories(1, 117)];
    rows.spell_class_options = vec![spell_class_options(1, 116)];
    rows.spell_cooldowns = vec![spell_cooldowns(1, 115)];
    rows.spell_equipped_items = vec![spell_equipped_items(1, 113)];
    rows.spell_interrupts = vec![spell_interrupts(1, 112)];
    rows.spell_levels = vec![spell_levels(1, 110)];
    rows.spell_misc = vec![spell_misc(1, 109)];
    rows.spell_reagents = vec![spell_reagents(1, 107)];
    rows.spell_scaling = vec![spell_scaling(1, 105)];
    rows.spell_shapeshifts = vec![spell_shapeshift(1, 104)];
    rows.spell_target_restrictions = vec![spell_target_restrictions(1, 103)];
    rows.spell_totems = vec![spell_totems(1, 102)];
    rows.spell_empowers = vec![SpellEmpowerRecord {
        id: 1,
        spell_id: 114,
        unused1000: 0,
    }];
    rows.spell_empower_stages = vec![SpellEmpowerStageRecord {
        id: 1,
        stage: 0,
        duration_ms: 1,
        spell_empower_id: 1,
    }];
    rows.spell_labels = vec![SpellLabelRecord {
        id: 1,
        label_id: 1,
        spell_id: 111,
    }];
    rows.spell_powers = vec![spell_power(1, 108)];
    rows.spell_reagents_currencies = vec![SpellReagentsCurrencyRecord {
        id: 1,
        spell_id: 106,
        currency_types_id: 1,
        currency_count: 1,
        override_recraft_currency_count: 0,
        order_source: 0,
    }];
    rows.spell_x_spell_visuals = vec![spell_x_spell_visual(1, 101)];
    let result = plan(rows)
        .with_server_spells(ServerSpellRows::default())
        .unwrap();
    assert_eq!(
        result.traversal_inputs().unwrap().helper_insertions(),
        &(101..=121).rev().map(|id| (id, 0)).collect::<Vec<_>>()
    );
    assert_eq!(result.client_counts().unnamed_helpers, 21);
    assert!(result.is_empty());
}
