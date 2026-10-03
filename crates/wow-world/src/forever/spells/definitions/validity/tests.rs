use super::super::{Definition, learning_fixtures as f};
use super::*;
use std::sync::Arc;
use wow_data::forever_spells::{DifficultyRecord, SpellRecords, SpellText};

fn craft(kind: u32, item: u32) -> super::super::SpellEffectValues {
    let mut effect = f::effect(kind, 0, 0, 0.0);
    effect.item_type = item;
    effect
}
fn learn(spell: u32) -> super::super::SpellEffectValues {
    let mut effect = f::effect(36, 0, 0, 0.0);
    effect.trigger_spell = spell;
    effect
}
fn seed(rows: Vec<(Key, Definition)>) -> SpellDefinitionSeeds {
    f::seeds(rows, Default::default(), Default::default())
}
fn difficulty(id: u32, fallback: i16) -> DifficultyRecord {
    DifficultyRecord {
        id,
        name: SpellText::default(),
        instance_type: 0,
        order_index: 0,
        old_enum_value: 0,
        fallback_difficulty_id: fallback,
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

#[test]
fn nonexistent_and_noncrafting_definitions_never_query_reagent_templates() {
    let mut d = f::definition(vec![f::effect(0, 0, 0, 0.0), craft(59, 99), craft(166, 99)]);
    d.fields.reagents = [99; 8];
    let s = seed(vec![((1, 0), d)]);
    assert_eq!(
        valid(&s, 99, 0, &mut |_| panic!("missing definition")),
        Ok(false)
    );
    assert_eq!(
        valid(&s, 1, 0, &mut |_| panic!("no crafting branch")),
        Ok(true)
    );
    assert!(s.get_exact(1, 0).unwrap().is_loot_crafting());
}

#[test]
fn zero_items_need_source_loot_kind_but_nonzero_fake_items_must_exist() {
    for (kinds, expected) in [
        (vec![24], false),
        (vec![157], true),
        (vec![24, 59], true),
        (vec![24, 157], true),
    ] {
        let s = seed(vec![(
            (1, 0),
            f::definition(kinds.into_iter().map(|kind| craft(kind, 0)).collect()),
        )]);
        assert_eq!(
            valid(&s, 1, 0, &mut |_| panic!("zero ItemType is not looked up")),
            Ok(expected)
        );
    }
    let s = seed(vec![((1, 0), f::definition(vec![craft(157, 99)]))]);
    let mut lookups = vec![];
    assert_eq!(
        valid(&s, 1, 0, &mut |id| {
            lookups.push(id);
            false
        }),
        Ok(false)
    );
    assert_eq!(lookups, [99]);
    assert_eq!(valid(&s, 1, 0, &mut |id| id == 99), Ok(true));
}

#[test]
fn reagents_follow_all_effects_positive_ids_only_and_ignore_counts() {
    let mut d = f::definition(vec![craft(24, 10), craft(157, 20)]);
    d.fields.reagents = [-1, 0, 30, i32::MIN, 40, 30, 0, 50];
    d.fields.reagent_counts = [0, 1, -1, 0, 0, 0, 0, 0];
    let s = seed(vec![((1, 0), d)]);
    let mut seen = vec![];
    assert_eq!(
        valid(&s, 1, 0, &mut |id| {
            seen.push(id);
            id != 50
        }),
        Ok(false)
    );
    assert_eq!(seen, [10, 20, 30, 40, 30, 50]);
    seen.clear();
    assert_eq!(
        valid(&s, 1, 0, &mut |id| {
            seen.push(id);
            id != 20
        }),
        Ok(false)
    );
    assert_eq!(seen, [10, 20]);
}

#[test]
fn recursive_children_keep_depth_first_order_and_repeat_reagent_queries_without_memoization() {
    let mut root = f::definition(vec![craft(24, 10), learn(2), learn(2), craft(24, 11)]);
    root.fields.reagents[0] = 90;
    let mut child = f::definition(vec![craft(24, 20), learn(3)]);
    child.fields.reagents[0] = 80;
    let mut leaf = f::definition(vec![craft(157, 30)]);
    leaf.fields.reagents[0] = 70;
    let s = seed(vec![((1, 0), root), ((2, 0), child), ((3, 0), leaf)]);
    let raw = s.raw_catalog();
    let mut seen = vec![];
    assert_eq!(
        valid(&s, 1, 0, &mut |id| {
            seen.push(id);
            true
        }),
        Ok(true)
    );
    assert_eq!(seen, [10, 20, 30, 70, 80, 20, 30, 70, 80, 11, 90]);
    assert!(Arc::ptr_eq(&raw, &s.raw_catalog()));
    assert_eq!(s.get_exact(1, 0).unwrap().fields().reagents[0], 90);
}

#[test]
fn invalid_early_effect_or_missing_child_precedes_later_recursive_cycle() {
    let s = seed(vec![((1, 0), f::definition(vec![craft(24, 0), learn(1)]))]);
    assert_eq!(valid(&s, 1, 0, &mut |_| true), Ok(false));
    let s = seed(vec![((1, 0), f::definition(vec![learn(99), learn(1)]))]);
    assert_eq!(valid(&s, 1, 0, &mut |_| true), Ok(false));
    let s = seed(vec![
        ((1, 0), f::definition(vec![learn(2)])),
        ((2, 0), f::definition(vec![learn(1)])),
    ]);
    assert_eq!(
        valid(&s, 1, 0, &mut |_| true),
        Err(SpellValidityError::RecursiveLearnCycle)
    );
}

#[test]
fn child_queries_use_regular_fallback_identity_not_parent_difficulty_or_spell_id_cycle_alone() {
    let raw = SpellRecords {
        difficulties: vec![difficulty(0, 1)],
        ..Default::default()
    };
    let s = f::seeds(
        vec![
            ((1, 2), f::definition(vec![learn(1)])),
            ((1, 1), f::definition(vec![])),
        ],
        raw,
        Default::default(),
    );
    assert_eq!(valid(&s, 1, 2, &mut |_| true), Ok(true));
    let raw = SpellRecords {
        difficulties: vec![difficulty(0, 1), difficulty(1, 0)],
        ..Default::default()
    };
    let s = f::seeds(
        vec![((1, 2), f::definition(vec![learn(99)]))],
        raw,
        Default::default(),
    );
    assert_eq!(
        valid(&s, 1, 2, &mut |_| true),
        Err(SpellValidityError::DefinitionLookup(
            SpellDefinitionError::DifficultyCycle
        ))
    );
}

#[test]
fn long_finite_learning_chains_use_explicit_stack_without_a_global_depth_cutoff() {
    let s = seed(
        (1..=4096)
            .map(|id| {
                (
                    (id, 0),
                    f::definition(if id == 4096 {
                        vec![]
                    } else {
                        vec![learn(id + 1)]
                    }),
                )
            })
            .collect(),
    );
    assert_eq!(
        valid(&s, 1, 0, &mut |_| panic!("no crafting effects")),
        Ok(true)
    );
}
