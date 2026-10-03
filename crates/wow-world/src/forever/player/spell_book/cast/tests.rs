//! Authoritative coordinator + canonical membership; synthetic aura/data inputs.
//! Ordered closures here are NOT GNU unordered_set order proof.
use super::*;
use crate::forever::spells::{SpellConstructorFields, SpellEffectValues, cast_test_definitions};
use wow_data::forever_spells::{DifficultyRecord, SpellText};

type Row = ((u32, i16), SpellConstructorFields, Vec<SpellEffectValues>);
fn row(id: u32) -> Row {
    ((id, 0), Default::default(), vec![])
}
fn aura_row(id: u32, kind: u32, misc: i32, family: u32, mask: [u32; 4], flags: u32) -> Row {
    let mut fields = SpellConstructorFields {
        spell_family_name: family,
        ..Default::default()
    };
    if flags & IGNORE_POWER != 0 {
        fields.attributes[8] = 0x40000;
    }
    if flags & IGNORE_SHAPESHIFT != 0 {
        fields.attributes[11] = 0x200;
    }
    (
        (id, 0),
        fields,
        vec![SpellEffectValues {
            effect: 6,
            aura: kind,
            misc_values: [misc, 0],
            class_mask: mask,
            // These base points must never replace the live amount.
            base_points: 999.0,
            ..Default::default()
        }],
    )
}
fn run(
    book: &PlayerSpellBook,
    s: &SpellDefinitionSeeds,
    id: u32,
    difficulty: i16,
    auras: &CastSpellAuras<'_>,
    order: impl FnMut(&[SpellBookMutation], usize) -> Result<Vec<u32>, &'static str>,
) -> Result<(u32, i16, u32), CastSpellError<&'static str>> {
    book.cast_spell_info(s.get_exact(id, 0).unwrap(), s, difficulty, auras, order)
        .map(|result| {
            (
                result.definition().spell_id(),
                result.definition().difficulty(),
                result.trigger_flags(),
            )
        })
}
fn none() -> CastSpellAuras<'static> {
    CastSpellAuras::new(&[], &[])
}
fn panic_order(_: &[SpellBookMutation], _: usize) -> Result<Vec<u32>, &'static str> {
    panic!("absent group must not call provider")
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
fn identity_fallback_needs_no_known_spell_or_order_provider() {
    let s = cast_test_definitions(vec![row(1)], vec![]);
    let book = PlayerSpellBook::default();
    assert_eq!(run(&book, &s, 1, 0, &none(), panic_order), Ok((1, 0, 0)));
    assert!(!book.has_spell(1));
    assert!(book.is_empty());
}

#[test]
fn context_is_exact_five_slots_with_equality_before_zero_and_no_initial_id() {
    let mut context = Context::default();
    assert!(!context.add(0));
    assert_eq!(context.visited, [0; 5]);
    for id in [u32::MAX, 1, 2, 3, 4] {
        assert!(context.add(id));
    }
    assert!(!context.add(5));
    assert!(!context.add(1));
    assert!(!context.add(0));
    assert_eq!(context.visited, [u32::MAX, 1, 2, 3, 4]);
}

#[test]
fn first_present_native_order_replacement_wins_without_book_active_filter() {
    let s = cast_test_definitions(vec![row(1), row(2), row(3)], vec![]);
    let mut book = PlayerSpellBook::default();
    for id in [999, 2, 3] {
        book.add_override_spell(1, id);
    }
    let result = run(&book, &s, 1, 0, &none(), |history, count| {
        assert_eq!(count, 3);
        assert_eq!(
            history,
            [
                SpellBookMutation::Insert(999),
                SpellBookMutation::Insert(2),
                SpellBookMutation::Insert(3)
            ]
        );
        Ok(vec![999, 3, 2])
    });
    assert_eq!(result, Ok((3, 0, 0)));
    assert!(!book.has_spell(3));
}

#[test]
fn zero_replacement_is_rejected_by_context_not_removed_from_membership() {
    let s = cast_test_definitions(vec![row(0), row(1), row(2)], vec![]);
    let mut book = PlayerSpellBook::default();
    book.add_override_spell(1, 0);
    book.add_override_spell(1, 2);
    assert_eq!(
        run(&book, &s, 1, 0, &none(), |_, _| Ok(vec![0, 2])),
        Ok((2, 0, 0))
    );
    assert!(book.has_override_spell(1, 0));
}

#[test]
fn self_and_pair_cycles_are_source_bounded_not_cycle_errors_or_sibling_backtracking() {
    let s = cast_test_definitions(vec![row(1), row(2), row(3)], vec![]);
    let mut book = PlayerSpellBook::default();
    book.add_override_spell(1, 1);
    let mut calls = 0;
    assert_eq!(
        run(&book, &s, 1, 0, &none(), |_, _| {
            calls += 1;
            Ok(vec![1])
        }),
        Ok((1, 0, 0))
    );
    assert_eq!(calls, 2); // initial 1 was not previsited
    book.remove_override_spell(1, 1);
    book.add_override_spell(1, 2);
    book.add_override_spell(1, 3);
    book.add_override_spell(2, 1);
    let mut calls = 0;
    // 1->[2,3], 2->[1], 1->[2,3]: visited 2 is skipped, 3 now wins.
    assert_eq!(
        run(&book, &s, 1, 0, &none(), |_, count| {
            calls += 1;
            Ok(if count == 2 { vec![2, 3] } else { vec![1] })
        }),
        Ok((3, 0, 0))
    );
    assert_eq!(calls, 3);
}

#[test]
fn depth_and_missing_ids_share_capacity_but_zero_never_consumes_it() {
    let s = cast_test_definitions((1..=7).map(row).collect(), vec![]);
    let mut book = PlayerSpellBook::default();
    for id in 1..=6 {
        book.add_override_spell(id, id + 1);
    }
    assert_eq!(
        run(&book, &s, 1, 0, &none(), |history, _| {
            let SpellBookMutation::Insert(id) = history[0] else {
                unreachable!()
            };
            Ok(vec![id])
        }),
        Ok((6, 0, 0))
    );
    let mut book = PlayerSpellBook::default();
    for id in [0, 100, 101, 102, 103, 104, 2] {
        book.add_override_spell(1, id);
    }
    assert_eq!(
        run(&book, &s, 1, 0, &none(), |_, _| Ok(vec![
            0, 100, 101, 102, 103, 104, 2
        ])),
        Ok((1, 0, 0))
    );
}

#[test]
fn player_phase_precedes_auras_and_unit_phase_reenters_player_with_shared_flags() {
    let s = cast_test_definitions(
        vec![
            row(1),
            row(2),
            row(3),
            row(4),
            aura_row(20, 333, 1, 0, [0; 4], IGNORE_POWER | IGNORE_SHAPESHIFT),
        ],
        vec![],
    );
    let amount = 2.0;
    let list = [CastOverrideAura::new(
        s.get_exact(20, 0).unwrap(),
        0,
        &amount,
    )];
    let auras = CastSpellAuras::new(&[], &list);
    let mut book = PlayerSpellBook::default();
    book.add_override_spell(1, 4);
    assert_eq!(
        run(&book, &s, 1, 0, &auras, |_, _| Ok(vec![4])),
        Ok((4, 0, 0))
    );
    book.remove_override_spell(1, 4);
    book.add_override_spell(2, 3);
    assert_eq!(
        run(&book, &s, 1, 0, &auras, |_, _| Ok(vec![3])),
        Ok((3, 0, IGNORE_POWER | IGNORE_SHAPESHIFT | IGNORE_CAST_TIME))
    );
}

#[test]
fn regular_auras_precede_triggered_and_keep_applied_effect_list_order() {
    let s = cast_test_definitions(
        vec![
            row(1),
            row(2),
            row(3),
            row(4),
            aura_row(20, 332, 1, 0, [0; 4], 0),
            aura_row(21, 333, 1, 0, [0; 4], IGNORE_POWER),
        ],
        vec![],
    );
    let [a, b, c] = [3.0, 2.0, 4.0];
    let regular = [
        CastOverrideAura::new(s.get_exact(20, 0).unwrap(), 0, &a),
        CastOverrideAura::new(s.get_exact(20, 0).unwrap(), 0, &b),
    ];
    let triggered = [CastOverrideAura::new(s.get_exact(21, 0).unwrap(), 0, &c)];
    assert_eq!(
        run(
            &PlayerSpellBook::default(),
            &s,
            1,
            0,
            &CastSpellAuras::new(&regular, &triggered),
            panic_order
        ),
        Ok((3, 0, 0))
    );
}

#[test]
fn subsequent_regular_aura_clears_each_flag_from_a_triggered_aura() {
    for keep in [
        0,
        IGNORE_POWER,
        IGNORE_SHAPESHIFT,
        IGNORE_POWER | IGNORE_SHAPESHIFT,
    ] {
        let s = cast_test_definitions(
            vec![
                row(1),
                row(2),
                row(3),
                aura_row(20, 333, 1, 0, [0; 4], IGNORE_POWER | IGNORE_SHAPESHIFT),
                aura_row(21, 332, 2, 0, [0; 4], keep),
            ],
            vec![],
        );
        let [a, b] = [2.0, 3.0];
        let triggered = [CastOverrideAura::new(s.get_exact(20, 0).unwrap(), 0, &a)];
        let regular = [CastOverrideAura::new(s.get_exact(21, 0).unwrap(), 0, &b)];
        assert_eq!(
            run(
                &PlayerSpellBook::default(),
                &s,
                1,
                0,
                &CastSpellAuras::new(&regular, &triggered),
                panic_order
            ),
            Ok((3, 0, keep))
        );
    }
}

#[test]
fn missing_aura_replacement_does_not_set_or_clear_flags() {
    let s = cast_test_definitions(
        vec![
            row(1),
            row(2),
            aura_row(20, 333, 1, 0, [0; 4], IGNORE_POWER | IGNORE_SHAPESHIFT),
            aura_row(21, 332, 2, 0, [0; 4], 0),
        ],
        vec![],
    );
    let [a, b] = [2.0, 999.0];
    let regular = [CastOverrideAura::new(s.get_exact(21, 0).unwrap(), 0, &b)];
    let triggered = [CastOverrideAura::new(s.get_exact(20, 0).unwrap(), 0, &a)];
    assert_eq!(
        run(
            &PlayerSpellBook::default(),
            &s,
            1,
            0,
            &CastSpellAuras::new(&regular, &triggered),
            panic_order
        ),
        Ok((2, 0, IGNORE_POWER | IGNORE_SHAPESHIFT | IGNORE_CAST_TIME))
    );
}

#[test]
fn missing_player_ids_consume_the_same_slots_as_matching_aura_ids() {
    let s = cast_test_definitions(
        vec![
            row(1),
            row(2),
            aura_row(20, 333, 1, 0, [0; 4], IGNORE_POWER),
        ],
        vec![],
    );
    let amount = 2.0;
    let list = [CastOverrideAura::new(
        s.get_exact(20, 0).unwrap(),
        0,
        &amount,
    )];
    let mut book = PlayerSpellBook::default();
    for id in 100..105 {
        book.add_override_spell(1, id);
    }
    assert_eq!(
        run(
            &book,
            &s,
            1,
            0,
            &CastSpellAuras::new(&[], &list),
            |_, _| Ok((100..105).collect())
        ),
        Ok((1, 0, 0))
    );
    book.remove_override_spell(1, 104);
    assert_eq!(
        run(
            &book,
            &s,
            1,
            0,
            &CastSpellAuras::new(&[], &list),
            |_, _| Ok((100..104).collect())
        ),
        Ok((2, 0, IGNORE_POWER | IGNORE_CAST_TIME))
    );
}

#[test]
fn misc_nonzero_uses_unsigned_exact_id_and_bypasses_family_masks() {
    let mut target = row(u32::MAX);
    target.1.spell_family_name = 1;
    let s = cast_test_definitions(
        vec![target, row(2), aura_row(20, 332, -1, 99, [u32::MAX; 4], 0)],
        vec![],
    );
    let amount = 2.9;
    let list = [CastOverrideAura::new(
        s.get_exact(20, 0).unwrap(),
        0,
        &amount,
    )];
    assert_eq!(
        run(
            &PlayerSpellBook::default(),
            &s,
            u32::MAX,
            0,
            &CastSpellAuras::new(&list, &[]),
            panic_order
        ),
        Ok((2, 0, 0))
    );
}

#[test]
fn family_zero_and_empty_mask_are_wildcards_and_all_four_words_intersect() {
    for (family, mask, expected) in [
        (0, [u32::MAX; 4], 2),
        (7, [0; 4], 2),
        (7, [0, 0, 0, 4], 2),
        (7, [0, 0, 0, 8], 1),
        (8, [0; 4], 1),
    ] {
        let mut target = row(1);
        target.1.spell_family_name = 7;
        target.1.spell_family_flags = [0, 0, 0, 4];
        let s = cast_test_definitions(
            vec![target, row(2), aura_row(20, 332, 0, family, mask, 0)],
            vec![],
        );
        let amount = 2.0;
        let list = [CastOverrideAura::new(
            s.get_exact(20, 0).unwrap(),
            0,
            &amount,
        )];
        assert_eq!(
            run(
                &PlayerSpellBook::default(),
                &s,
                1,
                0,
                &CastSpellAuras::new(&list, &[]),
                panic_order
            ),
            Ok((expected, 0, 0))
        );
    }
}

#[test]
fn invalid_amount_errors_only_if_aura_matches_and_negative_amount_promotes_to_uint32() {
    for amount in [
        f32::NAN,
        f32::INFINITY,
        f32::NEG_INFINITY,
        2147483648.0,
        -4294967296.0,
    ] {
        for misc in [1, 2] {
            let s =
                cast_test_definitions(vec![row(1), aura_row(20, 332, misc, 0, [0; 4], 0)], vec![]);
            let list = [CastOverrideAura::new(
                s.get_exact(20, 0).unwrap(),
                0,
                &amount,
            )];
            let result = run(
                &PlayerSpellBook::default(),
                &s,
                1,
                0,
                &CastSpellAuras::new(&list, &[]),
                panic_order,
            );
            assert_eq!(
                result,
                if misc == 1 {
                    Err(CastSpellError::UndefinedAmountNarrowing)
                } else {
                    Ok((1, 0, 0))
                }
            );
        }
    }
    for (amount, target) in [(-1.9, u32::MAX), (-2147483648.0, 0x80000000), (0.9, 1)] {
        let s = cast_test_definitions(
            vec![
                row(1),
                row(u32::MAX),
                row(0x80000000),
                aura_row(20, 332, 1, 0, [0; 4], 0),
            ],
            vec![],
        );
        let list = [CastOverrideAura::new(
            s.get_exact(20, 0).unwrap(),
            0,
            &amount,
        )];
        assert_eq!(
            run(
                &PlayerSpellBook::default(),
                &s,
                1,
                0,
                &CastSpellAuras::new(&list, &[]),
                panic_order
            ),
            Ok((target, 0, 0))
        );
    }
}

#[test]
fn replacement_uses_map_difficulty_and_actual_fallback_not_initial_regular_difficulty() {
    let mut exact = row(2);
    exact.0.1 = 9;
    let mut fallback = row(2);
    fallback.0.1 = 8;
    let s = cast_test_definitions(
        vec![row(1), row(2), exact, fallback],
        vec![difficulty(10, 8)],
    );
    let mut book = PlayerSpellBook::default();
    book.add_override_spell(1, 2);
    for (requested, expected) in [(9, (2, 9, 0)), (10, (2, 8, 0)), (11, (1, 0, 0))] {
        assert_eq!(
            run(&book, &s, 1, requested, &none(), |_, _| Ok(vec![2])),
            Ok(expected)
        );
    }
}

#[test]
fn fallback_cycle_is_an_error_not_a_missing_replacement_or_next_sibling() {
    let s = cast_test_definitions(
        vec![row(1), row(3)],
        vec![difficulty(9, 8), difficulty(8, 9)],
    );
    let mut book = PlayerSpellBook::default();
    book.add_override_spell(1, 2);
    book.add_override_spell(1, 3);
    assert_eq!(
        run(&book, &s, 1, 9, &none(), |_, _| Ok(vec![2, 3])),
        Err(CastSpellError::Definition(
            SpellDefinitionError::DifficultyCycle
        ))
    );
}

#[test]
fn malformed_order_errors_before_use_and_never_mutates_canonical_book() {
    let s = cast_test_definitions(vec![row(1), row(2), row(3)], vec![]);
    let mut book = PlayerSpellBook::default();
    book.add_override_spell(1, 2);
    book.add_override_spell(1, 3);
    for output in [vec![], vec![2], vec![2, 2], vec![2, 999], vec![2, 3, 2]] {
        assert_eq!(
            run(&book, &s, 1, 0, &none(), |_, _| Ok(output.clone())),
            Err(CastSpellError::Order(SpellBookOrderError::InvalidKeySet))
        );
    }
    assert_eq!(
        run(&book, &s, 1, 0, &none(), |_, _| Err("native unavailable")),
        Err(CastSpellError::Order(SpellBookOrderError::Source(
            "native unavailable"
        )))
    );
    assert!(book.has_override_spell(1, 2) && book.has_override_spell(1, 3));
    assert!(book.is_empty());
}

#[test]
fn foreign_initial_or_aura_definition_and_invalid_effect_fail_explicitly() {
    let s = cast_test_definitions(
        vec![row(1), row(2), aura_row(20, 332, 1, 0, [0; 4], 0)],
        vec![],
    );
    let other = cast_test_definitions(vec![row(1), aura_row(20, 332, 1, 0, [0; 4], 0)], vec![]);
    let book = PlayerSpellBook::default();
    let result = book.cast_spell_info(other.get_exact(1, 0).unwrap(), &s, 0, &none(), panic_order);
    assert!(matches!(
        result,
        Err(CastSpellError::MismatchedDefinitionOwner)
    ));
    let amount = 2.0;
    let list = [CastOverrideAura::new(
        other.get_exact(20, 0).unwrap(),
        0,
        &amount,
    )];
    assert_eq!(
        run(
            &book,
            &s,
            1,
            0,
            &CastSpellAuras::new(&list, &[]),
            panic_order
        ),
        Err(CastSpellError::MismatchedDefinitionOwner)
    );
    for (source, slot, triggered) in [(1, 0, false), (20, 1, false), (20, 0, true)] {
        let list = [CastOverrideAura::new(
            s.get_exact(source, 0).unwrap(),
            slot,
            &amount,
        )];
        let auras = if triggered {
            CastSpellAuras::new(&[], &list)
        } else {
            CastSpellAuras::new(&list, &[])
        };
        assert_eq!(
            run(&book, &s, 1, 0, &auras, panic_order),
            Err(CastSpellError::InvalidAuraEffect)
        );
    }
}

#[test]
fn override_group_erasure_resets_history_but_partial_erase_preserves_bucket_witness() {
    let mut book = PlayerSpellBook::default();
    book.remove_override_spell(0, 1);
    assert!(
        book.source_override_spells(0, panic_order)
            .unwrap()
            .is_empty()
    );
    book.add_override_spell(0, 0);
    book.add_override_spell(0, 1);
    book.add_override_spell(0, 1);
    book.remove_override_spell(0, 999);
    book.remove_override_spell(0, 0);
    book.source_override_spells(0, |history, count| {
        assert_eq!(count, 1);
        assert_eq!(
            history,
            [
                SpellBookMutation::Insert(0),
                SpellBookMutation::Insert(1),
                SpellBookMutation::Erase(0)
            ]
        );
        Ok::<_, &'static str>(vec![1])
    })
    .unwrap();
    book.remove_override_spell(0, 1);
    assert!(
        book.source_override_spells(0, panic_order)
            .unwrap()
            .is_empty()
    );
    book.add_override_spell(0, 1);
    book.source_override_spells(0, |history, _| {
        assert_eq!(history, [SpellBookMutation::Insert(1)]);
        Ok::<_, &'static str>(vec![1])
    })
    .unwrap();
}
