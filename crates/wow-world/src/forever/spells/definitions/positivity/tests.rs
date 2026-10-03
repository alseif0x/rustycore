//! Synthetic source-rule/order cases, not full custom/native/Player acceptance.
use super::super::Definition;
use super::*;
use crate::forever::spells::SpellLoadPlan;
use std::sync::Arc;
use wow_data::{Db2HotfixRemovalStoreLikeCpp, forever_spells::*};

fn effect(kind: u32, aura: u32, bp: f32) -> SpellEffectValues {
    SpellEffectValues {
        effect: kind,
        aura,
        base_points: bp,
        ..Default::default()
    }
}
fn seeds(rows: impl IntoIterator<Item = (Key, Vec<SpellEffectValues>)>) -> SpellDefinitionSeeds {
    seeds_with_catalog(SpellRecords::default(), rows)
}
fn seeds_with_catalog(
    records: SpellRecords,
    rows: impl IntoIterator<Item = (Key, Vec<SpellEffectValues>)>,
) -> SpellDefinitionSeeds {
    let catalog = records
        .finish(
            Default::default(),
            Default::default(),
            6,
            Default::default(),
            Default::default(),
            &Db2HotfixRemovalStoreLikeCpp::default(),
        )
        .unwrap();
    let mut seeds = SpellLoadPlan::build(Arc::new(catalog))
        .unwrap()
        .with_server_spells(Default::default())
        .unwrap();
    for (key, mut effects) in rows {
        for (index, effect) in effects.iter_mut().enumerate() {
            effect.index = index as u32;
        }
        let mut definition = Definition::empty_server(Vec::new());
        definition.fields.attributes[12] = 0x0020_0000; // preserve base sign
        definition.effects = effects;
        seeds.definitions.insert(key, definition);
    }
    seeds
}
fn items() -> ItemCatalog {
    wow_data::forever_birth::item_records::ItemRecords::default()
        .finish(
            Default::default(),
            Default::default(),
            &Db2HotfixRemovalStoreLikeCpp::default(),
        )
        .unwrap()
}
fn no_draw(_: f32, _: f32) -> Result<f32, SpellValueError> {
    panic!("zero variance draws nothing")
}
fn flags(seeds: &SpellDefinitionSeeds, key: Key) -> Vec<bool> {
    let definition = &seeds.definitions[&key];
    definition.negative_effects[..definition.effects.len()].to_vec()
}

#[test]
fn early_priority_preserves_existing_bits_and_skips_variance_before_passive_buff_or_harmful_rules()
{
    // Existing negative wins over passive; passive wins over forced debuff;
    // forced debuff wins over forced buff, which wins over IsHarmful.
    for (inactive, negative, passive, debuff, buff, harmful, expected) in [
        (true, true, true, true, true, true, true),
        (false, true, true, false, false, false, true),
        (false, false, true, true, false, true, false),
        (false, false, false, true, true, false, true),
        (false, false, false, false, true, true, false),
        (false, false, false, false, false, true, true),
    ] {
        let mut row = effect(if inactive { 0 } else { 3 }, 0, 100.0);
        row.scaling_variance = 0.5;
        row.attributes = if harmful { 0x1000 } else { 0 };
        let key = (900_001, 0);
        let mut seeds = seeds([(key, vec![row])]);
        let definition = seeds.definitions.get_mut(&key).unwrap();
        definition.negative_effects[0] = negative;
        definition.fields.attributes[0] =
            u32::from(passive) * 0x40 | u32::from(debuff) * 0x0400_0000;
        definition.fields.attributes[4] = u32::from(buff) * 0x1000;
        initialize(&mut seeds, key, &items(), &mut no_draw).unwrap();
        assert_eq!(flags(&seeds, key), [expected]);
    }
}

#[test]
fn value_draw_precedes_family_id_immunity_and_whole_spell_early_returns() {
    for (id, family, mechanic, expected) in [
        (40268, 0, 29, true),
        (24732, 0, 0, false),
        (32645, 8, 0, false),
        (40251, 8, 29, true),
        (900_001, 0, 29, false),
    ] {
        let mut row = effect(2, 0, 100.0);
        row.scaling_variance = 0.5;
        let key = (id, 0);
        let mut seeds = seeds([(key, vec![row])]);
        let fields = &mut seeds.definitions.get_mut(&key).unwrap().fields;
        fields.spell_family_name = family;
        fields.mechanic = mechanic;
        let mut calls = 0;
        initialize(&mut seeds, key, &items(), &mut |_, _| {
            calls += 1;
            Ok(0.0)
        })
        .unwrap();
        assert_eq!(calls, 1);
        assert_eq!(flags(&seeds, key), [expected]);
    }
}

#[test]
fn whole_spell_scan_keeps_source_effect_order_instead_of_collecting_heal_or_instakill_flags() {
    let key = (900_001, 0);
    let mut s = seeds([(key, vec![effect(10, 0, 100.0), effect(2, 0, 100.0)])]);
    initialize(&mut s, key, &items(), &mut no_draw).unwrap();
    assert_eq!(flags(&s, key), [false, false]);
    let mut s = seeds([(
        key,
        vec![
            effect(1, 0, 100.0),
            effect(10, 0, 100.0),
            effect(3, 0, 100.0),
        ],
    )]);
    initialize(&mut s, key, &items(), &mut no_draw).unwrap();
    assert_eq!(flags(&s, key), [false, true, true]);
}

#[test]
fn all_effect_kinds_keep_source_hard_negative_target_and_threat_rules() {
    for kind in 0..361 {
        for enemy in [false, true] {
            for bp in [-100.0, 0.0, 100.0] {
                let mut row = effect(kind, 0, bp);
                row.implicit_targets = [if enemy { 6 } else { 0 }, 0];
                let key = (900_001, 0);
                let mut s = seeds([(key, vec![row])]);
                initialize(&mut s, key, &items(), &mut no_draw).unwrap();
                let expected = matches!(
                    kind,
                    58 | 17
                        | 121
                        | 31
                        | 2
                        | 7
                        | 9
                        | 1
                        | 8
                        | 126
                        | 68
                        | 71
                        | 87
                        | 111
                        | 115
                        | 129
                        | 55
                        | 69
                ) || (enemy && matches!(kind, 98 | 96 | 27 | 114 | 62 | 38))
                    || (enemy && bp > 0.0 && matches!(kind, 63 | 125));
                assert_eq!(
                    flags(&s, key),
                    [expected],
                    "kind={kind} enemy={enemy} bp={bp}"
                );
            }
        }
    }
}

#[test]
fn aura_sign_target_and_per_level_sets_keep_their_different_predicates() {
    let cases: &[(&[u32], u8)] = &[
        (
            &[
                29, 30, 400, 49, 135, 59, 20, 21, 290, 54, 55, 57, 140, 192, 65, 216, 143, 91, 133,
                137, 58, 80, 34, 129,
            ],
            0,
        ),
        (
            &[
                9, 138, 13, 22, 101, 189, 99, 124, 79, 252, 193, 166, 136, 118,
            ],
            1,
        ),
        (&[14, 125, 126, 73, 72, 255], 2),
        (&[87], 3),
        (&[88], 4),
    ];
    for &(auras, group) in cases {
        for &aura in auras {
            for enemy in [false, true] {
                for bp in [-100.0, 0.0, 100.0] {
                    for per_level in [-1.0, 0.0, 1.0] {
                        let mut row = effect(6, aura, bp);
                        row.implicit_targets = [if enemy { 6 } else { 0 }, 0];
                        row.real_points_per_level = per_level;
                        let key = (900_001, 0);
                        let mut s = seeds([(key, vec![row])]);
                        initialize(&mut s, key, &items(), &mut no_draw).unwrap();
                        let expected = match group {
                            0 => bp < 0.0 || per_level < 0.0,
                            1 => enemy || bp < 0.0,
                            2 => bp > 0.0,
                            3 => enemy && bp > 0.0,
                            4 => enemy && bp < 0.0,
                            _ => unreachable!(),
                        };
                        assert_eq!(
                            flags(&s, key),
                            [expected],
                            "aura={aura}/{enemy}/{bp}/{per_level}"
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn modifier_operation_casts_to_uint8_and_reads_all_live_negative_bits() {
    for aura in [107, 108, 219, 218] {
        for op in 0..=255i32 {
            for upper in [-256, 0, 256] {
                for previous_negative in [false, true] {
                    for bp in [-100.0, 0.0, 100.0] {
                        let mut row = effect(6, aura, bp);
                        row.misc_values[0] = op + upper;
                        let key = (900_001, 0);
                        let mut s = seeds([(key, vec![row])]);
                        s.definitions.get_mut(&key).unwrap().negative_effects[31] =
                            previous_negative;
                        initialize(&mut s, key, &items(), &mut no_draw).unwrap();
                        let expected = match op {
                            10 | 19 | 30 | 21 => bp > 0.0,
                            11 | 14 | 34 | 39 => previous_negative && bp > 0.0,
                            3 | 12 | 23 | 32 | 33 | 8 | 2 | 20 | 27 => false,
                            1 | 7 | 0 | 17 => previous_negative && bp < 0.0,
                            _ => bp < 0.0,
                        };
                        assert_eq!(
                            flags(&s, key),
                            [expected],
                            "aura={aura} op={op} upper={upper} previous={previous_negative} bp={bp}"
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn visited_is_shared_across_root_effects_and_recursive_checks_do_not_publish_child_bits() {
    let root = (900_001, 0);
    let child = (900_002, 0);
    let mut first = effect(3, 0, 0.0);
    first.trigger_spell = child.0;
    let mut second = effect(3, 0, 0.0);
    second.trigger_spell = child.0;
    let mut s = seeds([
        (root, vec![first, second]),
        (child, vec![effect(2, 0, 100.0)]),
    ]);
    initialize(&mut s, root, &items(), &mut no_draw).unwrap();
    assert_eq!(flags(&s, root), [true, false]);
    assert_eq!(flags(&s, child), [false]);
    // A recursive self trigger terminates via visited rather than being
    // labeled negative merely because a graph contains a cycle.
    s.definitions.get_mut(&root).unwrap().effects[1].trigger_spell = root.0;
    initialize(&mut s, root, &items(), &mut no_draw).unwrap();
    // Its recursion sees the already-negative first slot before visited insert.
    assert_eq!(flags(&s, root), [true, true]);
}

#[test]
fn periodic_trigger_recursion_filters_child_targets_but_non_aura_triggers_do_not() {
    for aura in [0, 48, 227] {
        for enemy in [false, true] {
            let root = (900_001, 0);
            let child = (900_002, 0);
            let mut row = effect(if aura == 0 { 3 } else { 6 }, aura, 0.0);
            row.trigger_spell = child.0;
            let mut trigger = effect(2, 0, 100.0);
            trigger.implicit_targets = [if enemy { 6 } else { 0 }, 0];
            let mut s = seeds([(root, vec![row]), (child, vec![trigger])]);
            initialize(&mut s, root, &items(), &mut no_draw).unwrap();
            assert_eq!(flags(&s, root), [aura == 0 || !enemy]);
        }
    }
}

#[test]
fn additional_marking_checks_only_later_matching_targets_even_on_an_inactive_slot() {
    let key = (900_001, 0);
    for (reverse, different_target) in [(false, false), (true, false), (false, true)] {
        let dummy = effect(6, 4, 0.0);
        let mut damage = effect(2, 0, 100.0);
        damage.implicit_targets[0] = if different_target { 6 } else { 0 };
        let rows = if reverse {
            vec![damage, dummy]
        } else {
            vec![dummy, damage]
        };
        let mut s = seeds([(key, rows)]);
        initialize(&mut s, key, &items(), &mut no_draw).unwrap();
        assert_eq!(
            flags(&s, key),
            if reverse {
                vec![true, false]
            } else {
                vec![!different_target, true]
            }
        );
    }
    let mut s = seeds([(key, vec![effect(6, 4, 0.0), effect(0, 0, 0.0)])]);
    s.definitions.get_mut(&key).unwrap().negative_effects[1] = true;
    assert_eq!(initialize(&mut s, key, &items(), &mut no_draw).unwrap(), 1);
    assert_eq!(flags(&s, key), [true, true]);
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
fn recursive_difficulty_fallback_visits_the_actual_definition_and_invalid_cycles_are_not_positive()
{
    let root = (900_001, 2);
    let child = (900_002, 0);
    let mut row = effect(3, 0, 0.0);
    row.trigger_spell = child.0;
    let s = seeds_with_catalog(
        SpellRecords {
            difficulties: vec![difficulty(2, 1), difficulty(1, 0)],
            ..Default::default()
        },
        [(root, vec![row]), (child, vec![effect(2, 0, 100.0)])],
    );
    let mut visited = Visited::new();
    assert!(!positive(&s, root, 0, &items(), &mut no_draw, &mut visited).unwrap());
    assert!(visited.contains(&(child, 0)));
    assert!(!visited.contains(&((child.0, 2), 0)));
    let mut row = effect(3, 0, 0.0);
    row.trigger_spell = child.0;
    let s = seeds_with_catalog(
        SpellRecords {
            difficulties: vec![difficulty(2, 1), difficulty(1, 2)],
            ..Default::default()
        },
        [(root, vec![row])],
    );
    assert_eq!(
        positive(&s, root, 0, &items(), &mut no_draw, &mut Visited::new()),
        Err(SpellValueError::DefinitionLookup(
            super::super::SpellDefinitionError::DifficultyCycle
        ))
    );
    // Missing trigger with a terminating difficulty chain is a genuine no-op.
    let mut row = effect(3, 0, 0.0);
    row.trigger_spell = child.0;
    let s = seeds([(root, vec![row])]);
    assert!(positive(&s, root, 0, &items(), &mut no_draw, &mut Visited::new()).unwrap());
}

#[test]
fn unique_aura_checks_other_active_targets_before_a_heal_can_override_them() {
    let key = (900_001, 0);
    for inactive in [false, true] {
        let mut other = effect(if inactive { 0 } else { 10 }, 0, 100.0);
        other.implicit_targets = [6, 0];
        let mut s = seeds([(key, vec![effect(3, 0, 100.0), other])]);
        s.definitions.get_mut(&key).unwrap().fields.attributes[1] = 0x800;
        initialize(&mut s, key, &items(), &mut no_draw).unwrap();
        assert_eq!(
            flags(&s, key),
            if inactive {
                vec![false, false]
            } else {
                vec![true, true]
            }
        );
    }
}
