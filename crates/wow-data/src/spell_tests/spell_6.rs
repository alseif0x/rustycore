//! Spell scenarios for [`super`].
//!
//! Split out of spell_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

#[test]
fn spell_proc_event_spell_info_is_affected_matches_cpp_zero_family_name() {
    let event_spell = SpellProcEventSpellInfoLikeCpp {
        spell_family_name: 3,
        spell_family_mask: [0, 0, 0, 0],
    };

    assert!(event_spell.is_affected_like_cpp(0, [0xFFFF, 0xFFFF, 0xFFFF, 0xFFFF]));
}
#[test]
fn implicit_proc_aura_info_matches_cpp_trigger_table() {
    assert_eq!(
        implicit_proc_aura_info_like_cpp(aura_types::SPELL_AURA_DUMMY),
        Some(ImplicitProcAuraInfoLikeCpp {
            spell_type_mask: PROC_SPELL_TYPE_MASK_ALL_LIKE_CPP,
            triggered_can_proc: false,
        })
    );
    assert_eq!(
        implicit_proc_aura_info_like_cpp(aura_types::SPELL_AURA_SCHOOL_ABSORB),
        Some(ImplicitProcAuraInfoLikeCpp {
            spell_type_mask: PROC_SPELL_TYPE_MASK_ALL_LIKE_CPP,
            triggered_can_proc: true,
        })
    );
    assert_eq!(
        implicit_proc_aura_info_like_cpp(aura_types::SPELL_AURA_MOD_STEALTH),
        Some(ImplicitProcAuraInfoLikeCpp {
            spell_type_mask: PROC_SPELL_TYPE_DAMAGE_LIKE_CPP | PROC_SPELL_TYPE_NO_DMG_HEAL_LIKE_CPP,
            triggered_can_proc: true,
        })
    );
    assert_eq!(
        implicit_proc_aura_info_like_cpp(aura_types::SPELL_AURA_MOD_CONFUSE),
        Some(ImplicitProcAuraInfoLikeCpp {
            spell_type_mask: PROC_SPELL_TYPE_DAMAGE_LIKE_CPP,
            triggered_can_proc: true,
        })
    );
    assert_eq!(
        implicit_proc_aura_info_like_cpp(aura_types::SPELL_AURA_MOUNTED),
        None
    );
}
#[test]
fn implicit_spell_proc_entry_matches_cpp_default_generation() {
    let mut source = test_implicit_spell_proc_source_like_cpp();
    source.proc_flags = [
        PROC_FLAG_DEAL_HARMFUL_SPELL_LIKE_CPP | PROC_FLAG_KILL_LIKE_CPP,
        0,
    ];
    source.spell_family_name = 42;
    source.proc_chance = 25.0;
    source.proc_cooldown_ms = 1500;
    source.proc_charges = 3;
    source.effects = vec![
        test_implicit_proc_effect_like_cpp(
            0,
            aura_types::SPELL_AURA_PROC_TRIGGER_SPELL,
            [0x10, 0, 0, 0],
        ),
        test_implicit_proc_effect_like_cpp(1, aura_types::SPELL_AURA_MOUNTED, [0, 0, 0, 0]),
    ];

    let entry = implicit_spell_proc_entry_like_cpp(&source).unwrap();

    assert_eq!(entry.proc_flags, source.proc_flags);
    assert_eq!(entry.spell_family_name, 42);
    assert_eq!(entry.spell_family_mask, [0x10, 0, 0, 0]);
    assert_eq!(entry.spell_type_mask, PROC_SPELL_TYPE_MASK_ALL_LIKE_CPP);
    assert_eq!(entry.spell_phase_mask, PROC_SPELL_PHASE_HIT_LIKE_CPP);
    assert_eq!(entry.disable_effects_mask, 1 << 1);
    assert_eq!(entry.attributes_mask, PROC_ATTR_REQ_EXP_OR_HONOR_LIKE_CPP);
    assert_eq!(entry.chance, 25.0);
    assert_eq!(entry.cooldown_ms, 1500);
    assert_eq!(entry.charges, 3);
}
#[test]
fn implicit_spell_proc_entry_sets_special_phase_and_hit_masks_like_cpp() {
    let mut source = test_implicit_spell_proc_source_like_cpp();
    source.proc_flags = [
        PROC_FLAG_DEAL_MELEE_SWING_LIKE_CPP,
        PROC_FLAG_2_CAST_SUCCESSFUL_LIKE_CPP,
    ];
    source.effects = vec![test_implicit_proc_effect_like_cpp(
        0,
        aura_types::SPELL_AURA_MOD_BLOCK_PERCENT,
        [0, 0, 0, 0],
    )];

    let entry = implicit_spell_proc_entry_like_cpp(&source).unwrap();

    assert_eq!(entry.spell_phase_mask, PROC_SPELL_PHASE_CAST_LIKE_CPP);
    assert_eq!(entry.hit_mask, PROC_HIT_BLOCK_LIKE_CPP);

    source.effects = vec![test_implicit_proc_effect_like_cpp(
        0,
        aura_types::SPELL_AURA_REFLECT_SPELLS,
        [0, 0, 0, 0],
    )];
    assert_eq!(
        implicit_spell_proc_entry_like_cpp(&source)
            .unwrap()
            .hit_mask,
        PROC_HIT_REFLECT_LIKE_CPP
    );

    source.effects = vec![test_implicit_proc_effect_with_calc_like_cpp(
        0,
        aura_types::SPELL_AURA_MOD_HIT_CHANCE,
        -100,
    )];
    assert_eq!(
        implicit_spell_proc_entry_like_cpp(&source)
            .unwrap()
            .hit_mask,
        PROC_HIT_MISS_LIKE_CPP
    );
}
#[test]
fn implicit_spell_proc_entry_applies_taken_trigger_attr_and_skips_invalid_like_cpp() {
    let mut source = test_implicit_spell_proc_source_like_cpp();
    source.proc_flags = [PROC_FLAG_TAKE_HARMFUL_SPELL_LIKE_CPP, 0];
    source.effects = vec![test_implicit_proc_effect_like_cpp(
        0,
        aura_types::SPELL_AURA_PROC_TRIGGER_DAMAGE,
        [0, 0, 0, 0],
    )];

    let entry = implicit_spell_proc_entry_like_cpp(&source).unwrap();
    assert_eq!(entry.attributes_mask, PROC_ATTR_TRIGGERED_CAN_PROC_LIKE_CPP);

    source.proc_flags = [0, 0];
    assert!(implicit_spell_proc_entry_like_cpp(&source).is_none());

    source.proc_flags = [PROC_FLAG_DEAL_HARMFUL_SPELL_LIKE_CPP, 0];
    source.effects = vec![test_implicit_proc_effect_like_cpp(
        0,
        aura_types::SPELL_AURA_MOUNTED,
        [0, 0, 0, 0],
    )];
    assert!(implicit_spell_proc_entry_like_cpp(&source).is_none());
}
#[test]
fn implicit_spell_proc_entry_rejects_can_proc_from_procs_loop_like_cpp() {
    let mut source = test_implicit_spell_proc_source_like_cpp();
    source.proc_flags = [PROC_FLAG_DEAL_HARMFUL_SPELL_LIKE_CPP, 0];
    source.proc_chance = 100.0;
    source.attributes3 = attributes::SPELL_ATTR3_CAN_PROC_FROM_PROCS;
    let mut effect = test_implicit_proc_effect_like_cpp(
        0,
        aura_types::SPELL_AURA_PROC_TRIGGER_SPELL,
        [0, 0, 0, 0],
    );
    effect.trigger_spell = 123;
    source.effects = vec![effect];

    assert!(implicit_spell_proc_entry_like_cpp(&source).is_none());
}
#[test]
fn spell_learn_spell_store_validates_sql_rows_like_cpp() {
    let outcome = SpellLearnSpellStoreLikeCpp::from_sources_like_cpp(
        [
            SpellLearnSpellSqlRowLikeCpp {
                entry: 10,
                spell_id: 20,
                active: false,
            },
            SpellLearnSpellSqlRowLikeCpp {
                entry: 11,
                spell_id: 21,
                active: true,
            },
            SpellLearnSpellSqlRowLikeCpp {
                entry: 12,
                spell_id: 22,
                active: true,
            },
            SpellLearnSpellSqlRowLikeCpp {
                entry: 13,
                spell_id: 23,
                active: true,
            },
        ],
        [],
        [],
        |spell_id| match spell_id {
            10 => Some(learn_source(10, false, false, false, Vec::new())),
            12 => Some(learn_source(12, false, false, false, Vec::new())),
            13 => Some(learn_source(13, true, false, false, Vec::new())),
            _ => None,
        },
        |spell_id| matches!(spell_id, 20 | 23),
    );

    assert!(!outcome.sql_result_empty);
    assert_eq!(outcome.sql_loaded_row_count, 1);
    assert_eq!(outcome.dbc_loaded_row_count, 0);
    assert_eq!(
        outcome
            .errors
            .iter()
            .map(|error| error.kind)
            .collect::<Vec<_>>(),
        vec![
            SpellLearnSpellLoadErrorKindLikeCpp::SqlSourceSpellMissing,
            SpellLearnSpellLoadErrorKindLikeCpp::SqlLearnedSpellMissing,
            SpellLearnSpellLoadErrorKindLikeCpp::SqlSourceIsTalent,
        ]
    );
    assert_eq!(
        outcome.store.get_spell_learn_spell_map_bounds_like_cpp(10),
        &[SpellLearnSpellNodeLikeCpp {
            spell: 20,
            overrides_spell: 0,
            active: false,
            auto_learned: false,
        }]
    );
    assert!(outcome.store.is_spell_learn_spell_like_cpp(10));
    assert!(outcome.store.is_spell_learn_to_spell_like_cpp(10, 20));
    assert!(!outcome.store.is_spell_learn_to_spell_like_cpp(10, 21));
}
#[test]
fn spell_learn_spell_store_keeps_effect_and_db2_edges_when_world_sql_is_empty() {
    let outcome = SpellLearnSpellStoreLikeCpp::from_sources_like_cpp(
        [],
        [learn_source(
            100,
            false,
            false,
            false,
            vec![SpellLearnSpellEffectLikeCpp {
                trigger_spell: 101,
                target_unit_pet: false,
            }],
        )],
        [crate::spell_db2::SpellLearnSpellEntry {
            id: 1,
            spell_id: 200,
            learn_spell_id: 201,
            overrides_spell_id: 0,
        }],
        |_| None,
        |_| true,
    );

    assert!(outcome.sql_result_empty);
    assert_eq!(outcome.sql_loaded_row_count, 0);
    assert_eq!(outcome.dbc_loaded_row_count, 2);
    assert_eq!(
        outcome.store.get_spell_learn_spell_map_bounds_like_cpp(100),
        &[SpellLearnSpellNodeLikeCpp {
            spell: 101,
            overrides_spell: 0,
            active: true,
            auto_learned: false,
        }]
    );
    assert_eq!(
        outcome.store.get_spell_learn_spell_map_bounds_like_cpp(200),
        &[SpellLearnSpellNodeLikeCpp {
            spell: 201,
            overrides_spell: 0,
            active: true,
            auto_learned: false,
        }]
    );
    assert!(outcome.errors.is_empty());
    assert!(outcome.warnings.is_empty());
}
#[test]
fn spell_learn_spell_store_adds_spellinfo_effects_like_cpp() {
    let outcome = SpellLearnSpellStoreLikeCpp::from_sources_like_cpp(
        [SpellLearnSpellSqlRowLikeCpp {
            entry: 10,
            spell_id: 20,
            active: true,
        }],
        [
            learn_source(
                10,
                false,
                false,
                false,
                vec![SpellLearnSpellEffectLikeCpp {
                    trigger_spell: 20,
                    target_unit_pet: false,
                }],
            ),
            learn_source(
                30,
                false,
                true,
                false,
                vec![SpellLearnSpellEffectLikeCpp {
                    trigger_spell: 31,
                    target_unit_pet: false,
                }],
            ),
            SpellLearnSourceSpellInfoLikeCpp {
                spell_id: 40,
                difficulty_none: false,
                is_talent: false,
                is_passive: false,
                has_skill_step_effect: false,
                learn_spell_effects: vec![SpellLearnSpellEffectLikeCpp {
                    trigger_spell: 41,
                    target_unit_pet: true,
                }],
            },
        ],
        [],
        |spell_id| match spell_id {
            10 => Some(learn_source(10, false, false, false, Vec::new())),
            _ => None,
        },
        |spell_id| matches!(spell_id, 20 | 31 | 41),
    );

    assert_eq!(outcome.sql_loaded_row_count, 1);
    assert_eq!(outcome.dbc_loaded_row_count, 1);
    assert_eq!(outcome.warnings.len(), 1);
    assert_eq!(
        outcome.warnings[0].kind,
        SpellLearnSpellLoadWarningKindLikeCpp::RedundantSqlRowForSpellEffect {
            source_spell: 10,
            learned_spell: 20,
        }
    );
    assert_eq!(
        outcome.store.get_spell_learn_spell_map_bounds_like_cpp(30),
        &[SpellLearnSpellNodeLikeCpp {
            spell: 31,
            overrides_spell: 0,
            active: true,
            auto_learned: true,
        }]
    );
    assert!(
        outcome
            .store
            .get_spell_learn_spell_map_bounds_like_cpp(40)
            .is_empty()
    );
}
#[test]
fn spell_learn_spell_store_adds_db2_rows_after_sql_and_spell_effects_like_cpp() {
    let outcome = SpellLearnSpellStoreLikeCpp::from_sources_like_cpp(
        [SpellLearnSpellSqlRowLikeCpp {
            entry: 10,
            spell_id: 20,
            active: true,
        }],
        [learn_source(
            30,
            false,
            false,
            false,
            vec![SpellLearnSpellEffectLikeCpp {
                trigger_spell: 31,
                target_unit_pet: true,
            }],
        )],
        [
            crate::spell_db2::SpellLearnSpellEntry {
                id: 1,
                spell_id: 10,
                learn_spell_id: 20,
                overrides_spell_id: 0,
            },
            crate::spell_db2::SpellLearnSpellEntry {
                id: 2,
                spell_id: 30,
                learn_spell_id: 31,
                overrides_spell_id: 0,
            },
            crate::spell_db2::SpellLearnSpellEntry {
                id: 3,
                spell_id: 40,
                learn_spell_id: 41,
                overrides_spell_id: 42,
            },
            crate::spell_db2::SpellLearnSpellEntry {
                id: 4,
                spell_id: 50,
                learn_spell_id: 51,
                overrides_spell_id: 0,
            },
        ],
        |spell_id| match spell_id {
            10 => Some(learn_source(10, false, false, false, Vec::new())),
            _ => None,
        },
        |spell_id| matches!(spell_id, 10 | 20 | 30 | 31 | 40 | 41 | 51),
    );

    assert_eq!(outcome.sql_loaded_row_count, 1);
    assert_eq!(
        outcome.dbc_loaded_row_count, 2,
        "one SpellInfo effect plus one non-redundant SpellLearnSpell.db2 row"
    );
    assert_eq!(outcome.warnings.len(), 1);
    assert_eq!(
        outcome.warnings[0].kind,
        SpellLearnSpellLoadWarningKindLikeCpp::RedundantSqlRowForDb2 {
            source_spell: 10,
            learned_spell: 20,
        }
    );
    assert_eq!(
        outcome.store.get_spell_learn_spell_map_bounds_like_cpp(40),
        &[SpellLearnSpellNodeLikeCpp {
            spell: 41,
            overrides_spell: 42,
            active: true,
            auto_learned: false,
        }]
    );
    assert!(
        outcome
            .store
            .get_spell_learn_spell_map_bounds_like_cpp(50)
            .is_empty(),
        "C++ silently skips SpellLearnSpell.db2 rows whose source spell is missing"
    );
}
#[test]
fn serverside_spell_effect_store_groups_valid_effects_like_cpp() {
    let mut heroic = serverside_effect_row(100, 1);
    heroic.difficulty_id = 2;
    heroic.effect_radius_index_1 = 7;
    heroic.effect_radius_index_2 = 8;
    heroic.effect_spell_class_mask = [1, 2, 3, 4];
    heroic.implicit_target_1 = implicit_targets::TARGET_DEST_DB as i32;

    let outcome = ServersideSpellEffectStoreLikeCpp::from_rows_like_cpp(
        [heroic],
        |_| false,
        |difficulty| difficulty == 2,
        |radius| matches!(radius, 7 | 8),
    );

    assert_eq!(outcome.loaded_effect_count, 1);
    assert!(outcome.errors.is_empty());
    assert!(outcome.warnings.is_empty());
    let effects = outcome
        .store
        .effects_for_spell_difficulty_like_cpp(100, 2)
        .expect("valid serverside effect should be staged");
    assert_eq!(effects.len(), 1);
    assert_eq!(effects[0].effect_index, 1);
    assert_eq!(effects[0].effect_spell_class_mask, [1, 2, 3, 4]);
    assert_eq!(
        effects[0].implicit_target,
        [implicit_targets::TARGET_DEST_DB as i32, 0]
    );
}
#[test]
fn serverside_spell_effect_store_skips_invalid_rows_like_cpp() {
    let mut regular_spell = serverside_effect_row(10, 0);
    let mut missing_difficulty = serverside_effect_row(20, 0);
    missing_difficulty.difficulty_id = 3;
    let effect_index = serverside_effect_row(30, MAX_SPELL_EFFECTS_LIKE_CPP);
    let mut effect_type = serverside_effect_row(40, 0);
    effect_type.effect = TOTAL_SPELL_EFFECTS_LIKE_CPP;
    let mut aura_type = serverside_effect_row(50, 0);
    aura_type.effect_aura = TOTAL_AURAS_LIKE_CPP;
    let mut target_a = serverside_effect_row(60, 0);
    target_a.implicit_target_1 = TOTAL_SPELL_TARGETS_LIKE_CPP;
    let mut target_b = serverside_effect_row(70, 0);
    target_b.implicit_target_2 = TOTAL_SPELL_TARGETS_LIKE_CPP;
    regular_spell.effect_base_points = 10.0;

    let outcome = ServersideSpellEffectStoreLikeCpp::from_rows_like_cpp(
        [
            regular_spell,
            missing_difficulty,
            effect_index,
            effect_type,
            aura_type,
            target_a,
            target_b,
        ],
        |spell_id| spell_id == 10,
        |_| false,
        |_| true,
    );

    assert_eq!(outcome.loaded_effect_count, 0);
    assert_eq!(
        outcome
            .errors
            .iter()
            .map(|error| error.kind)
            .collect::<Vec<_>>(),
        vec![
            ServersideSpellEffectLoadErrorKindLikeCpp::RegularSpellAlreadyLoaded,
            ServersideSpellEffectLoadErrorKindLikeCpp::DifficultyMissing,
            ServersideSpellEffectLoadErrorKindLikeCpp::EffectIndexOutOfRange,
            ServersideSpellEffectLoadErrorKindLikeCpp::EffectTypeOutOfRange,
            ServersideSpellEffectLoadErrorKindLikeCpp::AuraTypeOutOfRange,
            ServersideSpellEffectLoadErrorKindLikeCpp::ImplicitTarget1OutOfRange,
            ServersideSpellEffectLoadErrorKindLikeCpp::ImplicitTarget2OutOfRange,
        ]
    );
}
#[test]
fn serverside_spell_effect_store_preserves_cpp_radius_warning_without_skip() {
    let mut row = serverside_effect_row(100, -1);
    row.effect_radius_index_1 = 77;
    row.effect_radius_index_2 = 88;

    let outcome = ServersideSpellEffectStoreLikeCpp::from_rows_like_cpp(
        [row],
        |_| false,
        |_| true,
        |_| false,
    );

    assert_eq!(outcome.loaded_effect_count, 1);
    assert!(outcome.errors.is_empty());
    assert_eq!(
        outcome
            .warnings
            .iter()
            .map(|warning| warning.kind)
            .collect::<Vec<_>>(),
        vec![
            ServersideSpellEffectLoadWarningKindLikeCpp::EffectRadius1Missing,
            ServersideSpellEffectLoadWarningKindLikeCpp::EffectRadius2Missing,
        ]
    );
    let effects = outcome
        .store
        .effects_for_spell_difficulty_like_cpp(100, 0)
        .expect("C++ still pushes effects with invalid radius rows");
    assert_eq!(effects[0].effect_index, -1);
    assert_eq!(effects[0].effect_radius_index, [77, 88]);
}
#[test]
fn serverside_spell_check_shapeshift_rejects_excluded_form_like_cpp() {
    let spell = serverside_spell_info_for_shapeshift(0, 1 << 2, 0, 0);
    let form = shapeshift_form(shapeshift_form_flags::STANCE);

    assert_eq!(
        spell.check_shapeshift_like_cpp(3, |_| Some(&form)),
        SpellCastResult::NotShapeshift
    );
}
#[test]
fn serverside_spell_check_shapeshift_allows_explicit_form_like_cpp() {
    let spell = serverside_spell_info_for_shapeshift(1 << 4, 0, 0, 0);
    let form = shapeshift_form(shapeshift_form_flags::STANCE);

    assert_eq!(
        spell.check_shapeshift_like_cpp(5, |_| Some(&form)),
        SpellCastResult::Success
    );
}
#[test]
fn serverside_spell_check_shapeshift_missing_form_allows_like_cpp() {
    let spell =
        serverside_spell_info_for_shapeshift(0, 0, attributes::SPELL_ATTR0_NOT_SHAPESHIFTED, 0);

    assert_eq!(
        spell.check_shapeshift_like_cpp(7, |_| None),
        SpellCastResult::Success
    );
}
#[test]
fn serverside_spell_check_shapeshift_rejects_not_shapeshifted_attr_like_cpp() {
    let spell =
        serverside_spell_info_for_shapeshift(0, 0, attributes::SPELL_ATTR0_NOT_SHAPESHIFTED, 0);
    let form = shapeshift_form(0);

    assert_eq!(
        spell.check_shapeshift_like_cpp(1, |_| Some(&form)),
        SpellCastResult::NotShapeshift
    );
}
#[test]
fn serverside_spell_check_shapeshift_rejects_can_only_cast_shapeshift_spells_like_cpp() {
    let spell = serverside_spell_info_for_shapeshift(0, 0, 0, 0);
    let form = shapeshift_form(shapeshift_form_flags::CAN_ONLY_CAST_SHAPESHIFT_SPELLS);

    assert_eq!(
        spell.check_shapeshift_like_cpp(1, |_| Some(&form)),
        SpellCastResult::NotShapeshift
    );
}
#[test]
fn serverside_spell_check_shapeshift_requires_other_shifted_form_like_cpp() {
    let spell = serverside_spell_info_for_shapeshift(1 << 4, 0, 0, 0);
    let form = shapeshift_form(0);

    assert_eq!(
        spell.check_shapeshift_like_cpp(2, |_| Some(&form)),
        SpellCastResult::OnlyShapeshift
    );
}
#[test]
fn serverside_spell_check_shapeshift_requires_form_when_unshifted_like_cpp() {
    let spell = serverside_spell_info_for_shapeshift(1 << 4, 0, 0, 0);

    assert_eq!(
        spell.check_shapeshift_like_cpp(0, |_| None),
        SpellCastResult::OnlyShapeshift
    );
}
#[test]
fn serverside_spell_check_shapeshift_allows_unshifted_with_attr2_like_cpp() {
    let spell = serverside_spell_info_for_shapeshift(
        1 << 4,
        0,
        0,
        attributes::SPELL_ATTR2_ALLOW_WHILE_NOT_SHAPESHIFTED_CASTER_FORM,
    );

    assert_eq!(
        spell.check_shapeshift_like_cpp(0, |_| None),
        SpellCastResult::Success
    );
}
#[test]
fn serverside_spell_store_composes_rows_with_staged_effects_like_cpp() {
    let effect_outcome = ServersideSpellEffectStoreLikeCpp::from_rows_like_cpp(
        [serverside_effect_row(100, 0)],
        |_| false,
        |_| true,
        |_| true,
    );
    let outcome = ServersideSpellStoreLikeCpp::from_rows_like_cpp(
        [serverside_spell_row(100, 0)],
        &effect_outcome.store,
        |_| false,
    );

    assert_eq!(outcome.loaded_spell_count, 1);
    assert!(outcome.errors.is_empty());
    assert_eq!(
        outcome.store.serverside_spell_names,
        vec![(100, "Serverside 100".to_string())]
    );
    let info = outcome
        .store
        .get_serverside_spell_like_cpp(100, 0)
        .expect("serverside spell should be represented");
    assert_eq!(info.row.attributes_ex[13], 18);
    assert_eq!(info.row.spell_family_flags, [70, 71, 72, 73]);
    assert_eq!(info.effects.len(), 1);
    assert_eq!(info.effects[0].effect_index, 0);
}
#[test]
fn serverside_spell_store_rejects_regular_db2_spell_like_cpp() {
    let outcome = ServersideSpellStoreLikeCpp::from_rows_like_cpp(
        [serverside_spell_row(100, 0)],
        &ServersideSpellEffectStoreLikeCpp::default(),
        |spell_id| spell_id == 100,
    );

    assert_eq!(outcome.loaded_spell_count, 0);
    assert_eq!(outcome.errors.len(), 1);
    assert_eq!(
        outcome.errors[0].kind,
        ServersideSpellLoadErrorKindLikeCpp::RegularSpellAlreadyLoaded
    );
    assert!(outcome.store.serverside_spell_names.is_empty());
    assert!(outcome.store.spell_infos_by_spell_and_difficulty.is_empty());
}
#[test]
fn serverside_spell_store_does_not_validate_main_row_difficulty_like_cpp() {
    let outcome = ServersideSpellStoreLikeCpp::from_rows_like_cpp(
        [serverside_spell_row(100, 999)],
        &ServersideSpellEffectStoreLikeCpp::default(),
        |_| false,
    );

    assert_eq!(outcome.loaded_spell_count, 1);
    assert!(outcome.errors.is_empty());
    assert!(
        outcome
            .store
            .get_serverside_spell_like_cpp(100, 999)
            .is_some(),
        "C++ LoadSpellInfoServerside validates DifficultyID for effect rows, not for the main serverside_spell row"
    );
}
