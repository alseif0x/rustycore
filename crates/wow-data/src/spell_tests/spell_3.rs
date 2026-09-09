//! Spell scenarios for [`super`].
//!
//! Split out of spell_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

#[test]
fn spell_implicit_target_conditions_attach_to_effects_like_cpp() {
    let mut store = SpellStore::new();
    store.insert(
        100,
        SpellInfo {
            spell_id: 100,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: 0,
            effect_base_points: 0,
            effect_bonus_coefficient: 0.0,
            aura_type: None,
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: vec![
                SpellEffectInfo {
                    effect_index: 0,
                    effect: 0,
                    chain_targets: 0,
                    implicit_target_1: 6,
                    implicit_target_2: 0,
                    ..Default::default()
                },
                SpellEffectInfo {
                    effect_index: 1,
                    effect: 0,
                    chain_targets: 0,
                    implicit_target_1: 7,
                    implicit_target_2: 0,
                    ..Default::default()
                },
            ],
        },
    );
    let conditions = ConditionEntriesByTypeStore::from_conditions_like_cpp([Condition {
        source_type: ConditionSourceType::SpellImplicitTarget,
        source_group: 0b11,
        source_entry: 100,
        condition_type: ConditionType::Aura,
        ..Condition::default()
    }]);

    assert_eq!(
        store.attach_spell_implicit_target_conditions_like_cpp(&conditions),
        2
    );
    assert!(
        store
            .implicit_target_conditions_like_cpp(100, 0)
            .and_then(|reference| reference.upgrade())
            .is_some_and(|conditions| conditions.len() == 1)
    );
    assert!(
        store
            .implicit_target_conditions_like_cpp(100, 1)
            .and_then(|reference| reference.upgrade())
            .is_some_and(|conditions| conditions.len() == 1)
    );
}
#[test]
fn spell_pet_aura_store_loads_first_row_metadata_and_wildcard_like_cpp() {
    let outcome = SpellPetAuraStoreLikeCpp::load_spell_pet_auras_like_cpp(
        [
            SpellPetAuraRowLikeCpp {
                spell_id: 10,
                effect_index: 1,
                pet_entry: 0,
                aura_id: 100,
            },
            SpellPetAuraRowLikeCpp {
                spell_id: 10,
                effect_index: 1,
                pet_entry: 700,
                aura_id: 200,
            },
        ],
        |spell_id, effect_index| {
            assert_eq!((spell_id, effect_index), (10, 1));
            SpellPetAuraSourceLookupLikeCpp::Found(SpellPetAuraSourceEffectLikeCpp {
                effect: spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                apply_aura_name: SPELL_AURA_DUMMY_LIKE_CPP,
                target_a: TARGET_UNIT_PET_LIKE_CPP,
                calc_value: 35,
            })
        },
        |aura_id| matches!(aura_id, 100 | 200),
    );

    assert_eq!(outcome.loaded_row_count, 2);
    assert!(outcome.errors.is_empty());
    let pet_aura = outcome.store.get_pet_aura_like_cpp(10, 1).unwrap();
    assert!(pet_aura.remove_on_change_pet);
    assert_eq!(pet_aura.damage, 35);
    assert_eq!(pet_aura.aura_for_pet_entry_like_cpp(700), 200);
    assert_eq!(
        pet_aura.aura_for_pet_entry_like_cpp(701),
        100,
        "C++ PetAura::GetAura falls back to petEntry 0"
    );
    assert_eq!(
        outcome.store.get_pet_aura_like_cpp(10, 2),
        None,
        "C++ SpellMgr::GetPetAura keys by (spell << 8) + effect index"
    );
}
#[test]
fn spell_pet_aura_store_rejects_invalid_first_rows_like_cpp() {
    let rows = [
        SpellPetAuraRowLikeCpp {
            spell_id: 1,
            effect_index: 0,
            pet_entry: 0,
            aura_id: 10,
        },
        SpellPetAuraRowLikeCpp {
            spell_id: 2,
            effect_index: 3,
            pet_entry: 0,
            aura_id: 20,
        },
        SpellPetAuraRowLikeCpp {
            spell_id: 3,
            effect_index: 0,
            pet_entry: 0,
            aura_id: 30,
        },
        SpellPetAuraRowLikeCpp {
            spell_id: 4,
            effect_index: 0,
            pet_entry: 0,
            aura_id: 40,
        },
    ];

    let outcome = SpellPetAuraStoreLikeCpp::load_spell_pet_auras_like_cpp(
        rows,
        |spell_id, _| match spell_id {
            1 => SpellPetAuraSourceLookupLikeCpp::SpellMissing,
            2 => SpellPetAuraSourceLookupLikeCpp::EffectIndexMissing,
            3 => SpellPetAuraSourceLookupLikeCpp::Found(SpellPetAuraSourceEffectLikeCpp {
                effect: spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                apply_aura_name: 73,
                target_a: 0,
                calc_value: 0,
            }),
            4 => SpellPetAuraSourceLookupLikeCpp::Found(SpellPetAuraSourceEffectLikeCpp {
                effect: spell_effect_types::SPELL_EFFECT_DUMMY,
                apply_aura_name: 0,
                target_a: 0,
                calc_value: 0,
            }),
            _ => unreachable!(),
        },
        |aura_id| aura_id != 40,
    );

    assert_eq!(outcome.loaded_row_count, 0);
    assert!(outcome.store.auras_by_spell_effect_key.is_empty());
    assert_eq!(
        outcome
            .errors
            .iter()
            .map(|error| error.kind)
            .collect::<Vec<_>>(),
        vec![
            SpellPetAuraLoadErrorKindLikeCpp::SpellMissing,
            SpellPetAuraLoadErrorKindLikeCpp::EffectIndexMissing,
            SpellPetAuraLoadErrorKindLikeCpp::SourceEffectNotDummy,
            SpellPetAuraLoadErrorKindLikeCpp::AuraSpellMissing,
        ]
    );
}
#[test]
fn spell_pet_aura_store_duplicate_keys_add_aura_without_revalidation_like_cpp() {
    let mut source_lookups = 0;
    let mut aura_checks = 0;
    let outcome = SpellPetAuraStoreLikeCpp::load_spell_pet_auras_like_cpp(
        [
            SpellPetAuraRowLikeCpp {
                spell_id: 77,
                effect_index: 2,
                pet_entry: 500,
                aura_id: 900,
            },
            SpellPetAuraRowLikeCpp {
                spell_id: 77,
                effect_index: 2,
                pet_entry: 501,
                aura_id: 0,
            },
        ],
        |_, _| {
            source_lookups += 1;
            SpellPetAuraSourceLookupLikeCpp::Found(SpellPetAuraSourceEffectLikeCpp {
                effect: spell_effect_types::SPELL_EFFECT_DUMMY,
                apply_aura_name: 0,
                target_a: 0,
                calc_value: -15,
            })
        },
        |aura_id| {
            aura_checks += 1;
            aura_id == 900
        },
    );

    assert_eq!(
        source_lookups, 1,
        "C++ validates only before creating a new SpellPetAuraMap entry"
    );
    assert_eq!(aura_checks, 1);
    assert_eq!(outcome.loaded_row_count, 2);
    assert!(outcome.errors.is_empty());
    let pet_aura = outcome.store.get_pet_aura_like_cpp(77, 2).unwrap();
    assert!(!pet_aura.remove_on_change_pet);
    assert_eq!(pet_aura.damage, -15);
    assert_eq!(pet_aura.aura_for_pet_entry_like_cpp(500), 900);
    assert_eq!(pet_aura.aura_for_pet_entry_like_cpp(501), 0);
}
#[test]
fn spell_threat_store_skips_missing_spells_like_cpp() {
    let outcome = SpellThreatStoreLikeCpp::from_rows_like_cpp(
        [
            SpellThreatRowLikeCpp {
                spell_id: 100,
                flat_mod: 7,
                pct_mod: 1.25,
                ap_pct_mod: 0.5,
            },
            SpellThreatRowLikeCpp {
                spell_id: 200,
                flat_mod: 9,
                pct_mod: 2.0,
                ap_pct_mod: 0.0,
            },
        ],
        |spell_id| spell_id == 100,
    );

    assert_eq!(outcome.loaded_row_count, 1);
    assert_eq!(outcome.errors.len(), 1);
    assert_eq!(outcome.errors[0].row.spell_id, 200);
    assert_eq!(
        outcome
            .store
            .get_spell_threat_entry_like_cpp(100, |_| unreachable!()),
        Some(&SpellThreatEntryLikeCpp {
            flat_mod: 7,
            pct_mod: 1.25,
            ap_pct_mod: 0.5,
        })
    );
}
#[test]
fn spell_threat_store_duplicate_rows_last_wins_like_cpp() {
    let outcome = SpellThreatStoreLikeCpp::from_rows_like_cpp(
        [
            SpellThreatRowLikeCpp {
                spell_id: 300,
                flat_mod: 1,
                pct_mod: 1.0,
                ap_pct_mod: 0.0,
            },
            SpellThreatRowLikeCpp {
                spell_id: 300,
                flat_mod: -4,
                pct_mod: 0.75,
                ap_pct_mod: 0.25,
            },
        ],
        |_| true,
    );

    assert_eq!(
        outcome.loaded_row_count, 2,
        "C++ increments count for every valid row before unordered_map overwrite visibility"
    );
    assert!(outcome.errors.is_empty());
    assert_eq!(outcome.store.entries_by_spell_id.len(), 1);
    assert_eq!(
        outcome
            .store
            .get_spell_threat_entry_like_cpp(300, |_| unreachable!()),
        Some(&SpellThreatEntryLikeCpp {
            flat_mod: -4,
            pct_mod: 0.75,
            ap_pct_mod: 0.25,
        })
    );
}
#[test]
fn spell_threat_store_falls_back_to_first_spell_in_chain_like_cpp() {
    let outcome = SpellThreatStoreLikeCpp::from_rows_like_cpp(
        [SpellThreatRowLikeCpp {
            spell_id: 11,
            flat_mod: 40,
            pct_mod: 1.5,
            ap_pct_mod: 0.0,
        }],
        |_| true,
    );

    assert_eq!(
        outcome
            .store
            .get_spell_threat_entry_like_cpp(42, |spell_id| {
                assert_eq!(spell_id, 42);
                11
            }),
        Some(&SpellThreatEntryLikeCpp {
            flat_mod: 40,
            pct_mod: 1.5,
            ap_pct_mod: 0.0,
        })
    );
    assert_eq!(
        outcome.store.get_spell_threat_entry_like_cpp(43, |_| 43),
        None
    );
}
#[test]
fn spell_linked_store_skips_missing_trigger_and_effect_like_cpp() {
    let outcome = SpellLinkedStoreLikeCpp::from_rows_like_cpp(
        [
            SpellLinkedRowLikeCpp {
                spell_trigger: 100,
                spell_effect: 200,
                link_type: 0,
            },
            SpellLinkedRowLikeCpp {
                spell_trigger: 300,
                spell_effect: 400,
                link_type: 0,
            },
        ],
        |spell_id| match spell_id {
            100 => Some(SpellLinkedSpellInfoLikeCpp {
                effect_calc_values_by_index: Vec::new(),
            }),
            _ => None,
        },
    );

    assert_eq!(outcome.loaded_row_count, 0);
    assert_eq!(
        outcome
            .errors
            .iter()
            .map(|error| error.kind)
            .collect::<Vec<_>>(),
        vec![
            SpellLinkedLoadErrorKindLikeCpp::EffectSpellMissing,
            SpellLinkedLoadErrorKindLikeCpp::TriggerSpellMissing,
        ]
    );
    assert!(outcome.store.effects_by_type_and_trigger.is_empty());
}
#[test]
fn spell_linked_store_preserves_signed_effects_and_push_order_like_cpp() {
    let outcome = SpellLinkedStoreLikeCpp::from_rows_like_cpp(
        [
            SpellLinkedRowLikeCpp {
                spell_trigger: 10,
                spell_effect: 20,
                link_type: 1,
            },
            SpellLinkedRowLikeCpp {
                spell_trigger: 10,
                spell_effect: -30,
                link_type: 1,
            },
        ],
        |_| {
            Some(SpellLinkedSpellInfoLikeCpp {
                effect_calc_values_by_index: Vec::new(),
            })
        },
    );

    assert_eq!(outcome.loaded_row_count, 2);
    assert!(outcome.errors.is_empty());
    assert_eq!(
        outcome
            .store
            .get_spell_linked_like_cpp(SpellLinkedTypeLikeCpp::Hit, 10),
        Some([20, -30].as_slice())
    );
}
#[test]
fn spell_linked_store_negative_trigger_forces_remove_like_cpp() {
    let outcome = SpellLinkedStoreLikeCpp::from_rows_like_cpp(
        [SpellLinkedRowLikeCpp {
            spell_trigger: -50,
            spell_effect: 60,
            link_type: 1,
        }],
        |_| {
            Some(SpellLinkedSpellInfoLikeCpp {
                effect_calc_values_by_index: Vec::new(),
            })
        },
    );

    assert_eq!(outcome.loaded_row_count, 1);
    assert!(outcome.errors.is_empty());
    assert_eq!(outcome.warnings.len(), 1);
    assert_eq!(
        outcome.warnings[0].kind,
        SpellLinkedLoadWarningKindLikeCpp::NegativeTriggerLinkTypeCoercedToRemove
    );
    assert_eq!(
        outcome
            .store
            .get_spell_linked_like_cpp(SpellLinkedTypeLikeCpp::Remove, 50),
        Some([60].as_slice())
    );
}
#[test]
fn spell_linked_store_invalid_type_and_self_loop_match_cpp() {
    let outcome = SpellLinkedStoreLikeCpp::from_rows_like_cpp(
        [
            SpellLinkedRowLikeCpp {
                spell_trigger: 10,
                spell_effect: 10,
                link_type: 0,
            },
            SpellLinkedRowLikeCpp {
                spell_trigger: 20,
                spell_effect: 20,
                link_type: 2,
            },
            SpellLinkedRowLikeCpp {
                spell_trigger: 30,
                spell_effect: 40,
                link_type: 9,
            },
        ],
        |_| {
            Some(SpellLinkedSpellInfoLikeCpp {
                effect_calc_values_by_index: Vec::new(),
            })
        },
    );

    assert_eq!(outcome.loaded_row_count, 1);
    assert_eq!(
        outcome
            .errors
            .iter()
            .map(|error| error.kind)
            .collect::<Vec<_>>(),
        vec![
            SpellLinkedLoadErrorKindLikeCpp::SelfTriggerLoop,
            SpellLinkedLoadErrorKindLikeCpp::InvalidLinkType,
        ]
    );
    assert_eq!(
        outcome
            .store
            .get_spell_linked_like_cpp(SpellLinkedTypeLikeCpp::Aura, 20),
        Some([20].as_slice())
    );
}
#[test]
fn spell_linked_store_same_base_point_warning_does_not_skip_like_cpp() {
    let outcome = SpellLinkedStoreLikeCpp::from_rows_like_cpp(
        [SpellLinkedRowLikeCpp {
            spell_trigger: 70,
            spell_effect: 12,
            link_type: 0,
        }],
        |spell_id| {
            if spell_id == 70 {
                Some(SpellLinkedSpellInfoLikeCpp {
                    effect_calc_values_by_index: vec![(2, 12)],
                })
            } else {
                Some(SpellLinkedSpellInfoLikeCpp {
                    effect_calc_values_by_index: Vec::new(),
                })
            }
        },
    );

    assert_eq!(outcome.loaded_row_count, 1);
    assert!(outcome.errors.is_empty());
    assert_eq!(
        outcome.warnings[0].kind,
        SpellLinkedLoadWarningKindLikeCpp::TriggerEffectSameBasePoint { effect_index: 2 }
    );
    assert_eq!(
        outcome
            .store
            .get_spell_linked_like_cpp(SpellLinkedTypeLikeCpp::Cast, 70),
        Some([12].as_slice())
    );
}
#[test]
fn spell_totem_model_store_skips_missing_dependencies_like_cpp() {
    let outcome = SpellTotemModelStoreLikeCpp::from_rows_like_cpp(
        [
            SpellTotemModelRowLikeCpp {
                spell_id: 10,
                race_id: 2,
                display_id: 100,
            },
            SpellTotemModelRowLikeCpp {
                spell_id: 20,
                race_id: 2,
                display_id: 100,
            },
            SpellTotemModelRowLikeCpp {
                spell_id: 10,
                race_id: 3,
                display_id: 100,
            },
            SpellTotemModelRowLikeCpp {
                spell_id: 10,
                race_id: 2,
                display_id: 200,
            },
        ],
        |spell_id| spell_id == 10,
        |race_id| race_id == 2,
        |display_id| display_id == 100,
    );

    assert_eq!(outcome.loaded_row_count, 1);
    assert_eq!(
        outcome
            .errors
            .iter()
            .map(|error| error.kind)
            .collect::<Vec<_>>(),
        vec![
            SpellTotemModelLoadErrorKindLikeCpp::SpellMissing,
            SpellTotemModelLoadErrorKindLikeCpp::RaceMissing,
            SpellTotemModelLoadErrorKindLikeCpp::DisplayMissing,
        ]
    );
    assert_eq!(outcome.store.get_model_for_totem_like_cpp(10, 2), 100);
    assert_eq!(outcome.store.get_model_for_totem_like_cpp(10, 3), 0);
}
#[test]
fn spell_totem_model_store_duplicate_rows_last_wins_like_cpp() {
    let outcome = SpellTotemModelStoreLikeCpp::from_rows_like_cpp(
        [
            SpellTotemModelRowLikeCpp {
                spell_id: 50,
                race_id: 8,
                display_id: 1000,
            },
            SpellTotemModelRowLikeCpp {
                spell_id: 50,
                race_id: 8,
                display_id: 2000,
            },
        ],
        |_| true,
        |_| true,
        |_| true,
    );

    assert_eq!(
        outcome.loaded_row_count, 2,
        "C++ increments count for every valid row before std::map overwrite visibility"
    );
    assert!(outcome.errors.is_empty());
    assert_eq!(outcome.store.display_id_by_spell_and_race.len(), 1);
    assert_eq!(outcome.store.get_model_for_totem_like_cpp(50, 8), 2000);
    assert_eq!(outcome.store.get_model_for_totem_like_cpp(50, 2), 0);
}
#[test]
fn spell_required_store_skips_missing_and_same_chain_like_cpp() {
    let outcome = SpellRequiredStoreLikeCpp::from_rows_like_cpp(
        [
            SpellRequiredRowLikeCpp {
                spell_id: 10,
                req_spell: 20,
            },
            SpellRequiredRowLikeCpp {
                spell_id: 30,
                req_spell: 40,
            },
            SpellRequiredRowLikeCpp {
                spell_id: 50,
                req_spell: 60,
            },
        ],
        |spell_id| matches!(spell_id, 10 | 20 | 30 | 50 | 60),
        |spell_id, req_spell| spell_id == 50 && req_spell == 60,
    );

    assert_eq!(outcome.loaded_row_count, 1);
    assert_eq!(
        outcome
            .errors
            .iter()
            .map(|error| error.kind)
            .collect::<Vec<_>>(),
        vec![
            SpellRequiredLoadErrorKindLikeCpp::RequiredSpellMissing,
            SpellRequiredLoadErrorKindLikeCpp::SameRankChain,
        ]
    );
    assert_eq!(outcome.store.spells_required_for_spell_like_cpp(10), &[20]);
    assert_eq!(outcome.store.spells_requiring_spell_like_cpp(20), &[10]);
}
#[test]
fn spell_required_store_skips_missing_spell_id_like_cpp() {
    let outcome = SpellRequiredStoreLikeCpp::from_rows_like_cpp(
        [SpellRequiredRowLikeCpp {
            spell_id: 70,
            req_spell: 80,
        }],
        |spell_id| spell_id == 80,
        |_, _| false,
    );

    assert_eq!(outcome.loaded_row_count, 0);
    assert_eq!(outcome.errors.len(), 1);
    assert_eq!(
        outcome.errors[0].kind,
        SpellRequiredLoadErrorKindLikeCpp::SpellMissing
    );
}
#[test]
fn spell_required_store_skips_duplicate_exact_pair_like_cpp() {
    let outcome = SpellRequiredStoreLikeCpp::from_rows_like_cpp(
        [
            SpellRequiredRowLikeCpp {
                spell_id: 90,
                req_spell: 100,
            },
            SpellRequiredRowLikeCpp {
                spell_id: 90,
                req_spell: 100,
            },
            SpellRequiredRowLikeCpp {
                spell_id: 91,
                req_spell: 100,
            },
        ],
        |_| true,
        |_, _| false,
    );

    assert_eq!(outcome.loaded_row_count, 2);
    assert_eq!(outcome.errors.len(), 1);
    assert_eq!(
        outcome.errors[0].kind,
        SpellRequiredLoadErrorKindLikeCpp::Duplicate
    );
    assert!(outcome.store.is_spell_requiring_spell_like_cpp(90, 100));
    assert!(outcome.store.is_spell_requiring_spell_like_cpp(91, 100));
    assert_eq!(outcome.store.spells_required_for_spell_like_cpp(90), &[100]);
    assert_eq!(
        outcome.store.spells_requiring_spell_like_cpp(100),
        &[90, 91]
    );
}
#[test]
fn spell_learn_skill_store_derives_skill_effect_like_cpp() {
    let outcome = SpellLearnSkillStoreLikeCpp::from_spell_infos_like_cpp([learn_skill_source(
        100,
        true,
        vec![SpellLearnSkillEffectLikeCpp {
            effect: spell_effect_types::SPELL_EFFECT_SKILL,
            misc_value: 755,
            calc_value: 4,
        }],
    )]);

    assert_eq!(outcome.dbc_loaded_row_count, 1);
    assert!(outcome.errors.is_empty());
    assert_eq!(
        outcome.store.get_spell_learn_skill_like_cpp(100),
        Some(&SpellLearnSkillNodeLikeCpp {
            skill: 755,
            step: 4,
            value: 0,
            maxvalue: 0,
        })
    );
    assert_eq!(
        outcome.store.spell_learn_skill_lookup_like_cpp(100),
        SpellLearnSkillLookupLikeCpp::Present(&SpellLearnSkillNodeLikeCpp {
            skill: 755,
            step: 4,
            value: 0,
            maxvalue: 0,
        })
    );
}
#[test]
fn spell_learn_skill_store_derives_dual_wield_like_cpp() {
    let outcome = SpellLearnSkillStoreLikeCpp::from_spell_infos_like_cpp([learn_skill_source(
        200,
        true,
        vec![SpellLearnSkillEffectLikeCpp {
            effect: spell_effect_types::SPELL_EFFECT_DUAL_WIELD,
            misc_value: 0,
            calc_value: 0,
        }],
    )]);

    assert_eq!(outcome.dbc_loaded_row_count, 1);
    assert!(outcome.errors.is_empty());
    assert_eq!(
        outcome.store.get_spell_learn_skill_like_cpp(200),
        Some(&SpellLearnSkillNodeLikeCpp {
            skill: SKILL_DUAL_WIELD_LIKE_CPP,
            step: 1,
            value: 1,
            maxvalue: 1,
        })
    );
}
#[test]
fn spell_learn_skill_store_skips_non_base_difficulty_and_breaks_after_first_match_like_cpp() {
    let outcome = SpellLearnSkillStoreLikeCpp::from_spell_infos_like_cpp([
        learn_skill_source(
            300,
            false,
            vec![SpellLearnSkillEffectLikeCpp {
                effect: spell_effect_types::SPELL_EFFECT_SKILL,
                misc_value: 333,
                calc_value: 3,
            }],
        ),
        learn_skill_source(
            301,
            true,
            vec![
                SpellLearnSkillEffectLikeCpp {
                    effect: spell_effect_types::SPELL_EFFECT_NONE,
                    misc_value: 0,
                    calc_value: 0,
                },
                SpellLearnSkillEffectLikeCpp {
                    effect: spell_effect_types::SPELL_EFFECT_DUAL_WIELD,
                    misc_value: 0,
                    calc_value: 0,
                },
                SpellLearnSkillEffectLikeCpp {
                    effect: spell_effect_types::SPELL_EFFECT_SKILL,
                    misc_value: 755,
                    calc_value: 8,
                },
            ],
        ),
    ]);

    assert_eq!(outcome.dbc_loaded_row_count, 1);
    assert!(outcome.errors.is_empty());
    assert!(outcome.store.get_spell_learn_skill_like_cpp(300).is_none());
    assert_eq!(
        outcome.store.spell_learn_skill_lookup_like_cpp(300),
        SpellLearnSkillLookupLikeCpp::MissingCoverage
    );
    assert_eq!(
        outcome.store.get_spell_learn_skill_like_cpp(301),
        Some(&SpellLearnSkillNodeLikeCpp {
            skill: SKILL_DUAL_WIELD_LIKE_CPP,
            step: 1,
            value: 1,
            maxvalue: 1,
        })
    );
}
#[test]
fn spell_learn_skill_lookup_distinguishes_covered_absence_and_indeterminate() {
    let mut outcome = SpellLearnSkillStoreLikeCpp::from_spell_infos_like_cpp([learn_skill_source(
        500,
        true,
        Vec::new(),
    )]);
    outcome.store.mark_spell_learn_skill_indeterminate_like_cpp(
        501,
        SpellLearnSkillIndeterminateReasonLikeCpp::RngDependentCalcValue {
            record_id: 9,
            domain: AcquisitionValueDomainLikeCpp {
                minimum: 2,
                maximum: 4,
            },
        },
    );

    assert_eq!(
        outcome.store.spell_learn_skill_lookup_like_cpp(500),
        SpellLearnSkillLookupLikeCpp::CoveredWithoutNode
    );
    assert!(matches!(
        outcome.store.spell_learn_skill_lookup_like_cpp(501),
        SpellLearnSkillLookupLikeCpp::Indeterminate(
            SpellLearnSkillIndeterminateReasonLikeCpp::RngDependentCalcValue {
                record_id: 9,
                domain: AcquisitionValueDomainLikeCpp {
                    minimum: 2,
                    maximum: 4,
                },
            }
        )
    ));
    assert_eq!(
        outcome.store.spell_learn_skill_lookup_like_cpp(502),
        SpellLearnSkillLookupLikeCpp::MissingCoverage
    );
}
#[test]
fn spell_learn_skill_store_rejects_duplicate_source_ids_in_every_order() {
    let valid = || {
        learn_skill_source(
            600,
            true,
            vec![SpellLearnSkillEffectLikeCpp {
                effect: spell_effect_types::SPELL_EFFECT_SKILL,
                misc_value: 755,
                calc_value: 4,
            }],
        )
    };
    let invalid = || {
        learn_skill_source(
            600,
            true,
            vec![SpellLearnSkillEffectLikeCpp {
                effect: spell_effect_types::SPELL_EFFECT_SKILL,
                misc_value: -1,
                calc_value: 4,
            }],
        )
    };

    for sources in [[valid(), invalid()], [invalid(), valid()]] {
        let outcome = SpellLearnSkillStoreLikeCpp::from_spell_infos_like_cpp(sources);

        assert_eq!(outcome.dbc_loaded_row_count, 0);
        assert!(
            outcome.store.get_spell_learn_skill_like_cpp(600).is_none(),
            "the legacy getter must not leak a node from either duplicate ordering"
        );
        assert_eq!(
            outcome.store.spell_learn_skill_lookup_like_cpp(600),
            SpellLearnSkillLookupLikeCpp::Indeterminate(
                &SpellLearnSkillIndeterminateReasonLikeCpp::DuplicateSourceSpell
            )
        );
        assert!(outcome.errors.iter().any(|error| {
            error.spell_id == 600
                && error.kind == SpellLearnSkillLoadErrorKindLikeCpp::DuplicateSourceSpell
        }));
    }
}
