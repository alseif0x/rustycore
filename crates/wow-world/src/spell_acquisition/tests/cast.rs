use super::*;

#[test]
fn wrapper_runs_skill_step_before_two_causal_learn_effects() {
    const WRAPPER: u32 = 200;
    const SKILL: u32 = 164;
    let metadata = MetadataFixture::new(FixtureInput {
        spell_ids: vec![WRAPPER, 300, 301, 400],
        effects: vec![
            skill_step_effect(1, WRAPPER, 0, SKILL, 1),
            learn_effect(2, WRAPPER, 1, 300),
            learn_effect(3, WRAPPER, 2, 400),
        ],
        dependencies: vec![dependency(4, 300, 301)],
        cast_safe_spell_ids: vec![WRAPPER],
        skill_lines: vec![skill_line(SKILL, 0, 0)],
        skill_race_class: vec![race_class(SKILL as u16, 1, 10)],
        skill_tiers: vec![tiers(10)],
        ..Default::default()
    });

    let plan = deterministic(project_spell_acquisition_like_cpp(
        &snapshot_with_cast(WRAPPER, true, [0, 1, 2]),
        metadata.metadata(),
        SpellAcquisitionRootLikeCpp::TrainerWrapperCast(WRAPPER),
    ));

    assert_eq!(
        plan.skill_transitions
            .iter()
            .map(|transition| transition.skill_id)
            .collect::<Vec<_>>(),
        vec![SKILL]
    );
    let first_skill_action = plan
        .post_commit_actions
        .iter()
        .position(|action| {
            *action
                == SpellAcquisitionPostCommitActionLikeCpp::UpdateSkillRaisedCriteria {
                    skill_id: SKILL,
                }
        })
        .expect("skill phase must publish its post-commit intent");
    let dependency_done = plan
        .post_commit_actions
        .iter()
        .position(|action| {
            *action
                == SpellAcquisitionPostCommitActionLikeCpp::UpdateLearnOrKnowSpellCriteria {
                    spell_id: 301,
                }
        })
        .expect("first learn effect must complete its dependency");
    let first_learn_done = plan
        .post_commit_actions
        .iter()
        .position(|action| {
            *action
                == SpellAcquisitionPostCommitActionLikeCpp::UpdateLearnOrKnowSpellCriteria {
                    spell_id: 300,
                }
        })
        .expect("first learn effect completes");
    let second_learn_started = plan
        .post_commit_actions
        .iter()
        .position(|action| {
            *action
                == SpellAcquisitionPostCommitActionLikeCpp::UpdateLearnOrKnowSpellCriteria {
                    spell_id: 400,
                }
        })
        .expect("second learn effect completes");
    assert!(first_skill_action < dependency_done);
    assert!(dependency_done < first_learn_done);
    assert!(first_learn_done < second_learn_started);
}

#[test]
fn pet_target_and_cast_item_learning_paths_are_indeterminate() {
    const WRAPPER: u32 = 200;
    let mut pet_learn = learn_effect(1, WRAPPER, 0, 100);
    pet_learn.implicit_target_raw = [TARGET_UNIT_PET_LIKE_CPP, 0];
    let pet_metadata = MetadataFixture::new(FixtureInput {
        spell_ids: vec![WRAPPER, 100],
        effects: vec![pet_learn],
        cast_safe_spell_ids: vec![WRAPPER],
        ..Default::default()
    });
    assert!(matches!(
        project_spell_acquisition_like_cpp(
            &snapshot_with_cast(WRAPPER, true, [0]),
            pet_metadata.metadata(),
            SpellAcquisitionRootLikeCpp::TrainerWrapperCast(WRAPPER),
        ),
        SpellAcquisitionOutcomeLikeCpp::Indeterminate(
            SpellAcquisitionIndeterminateLikeCpp::PetLearnPath {
                spell_id: WRAPPER,
                effect_index: 0,
            },
        )
    ));

    let cast_item_metadata = MetadataFixture::new(FixtureInput {
        spell_ids: vec![WRAPPER],
        effects: vec![learn_effect(2, WRAPPER, 0, 0)],
        cast_safe_spell_ids: vec![WRAPPER],
        ..Default::default()
    });
    let outcome = project_spell_acquisition_like_cpp(
        &snapshot_with_cast(WRAPPER, true, [0]),
        cast_item_metadata.metadata(),
        SpellAcquisitionRootLikeCpp::TrainerWrapperCast(WRAPPER),
    );
    assert!(
        matches!(
            outcome,
            SpellAcquisitionOutcomeLikeCpp::Indeterminate(
                SpellAcquisitionIndeterminateLikeCpp::CastItemLearnPath {
                    spell_id: WRAPPER,
                    effect_index: 0,
                }
            ) | SpellAcquisitionOutcomeLikeCpp::Indeterminate(
                SpellAcquisitionIndeterminateLikeCpp::IndeterminateSpellMetadata {
                    spell_id: WRAPPER,
                    table: SpellAcquisitionTableLikeCpp::SpellEffect,
                    ..
                }
            )
        ),
        "zero-trigger LEARN must fail closed, got {outcome:?}"
    );
}

#[test]
fn wrapper_hit_skill_phase_precedes_lower_index_learn_target_phase() {
    const WRAPPER: u32 = 200;
    const LEARNED: u32 = 300;
    const SKILL: u32 = 164;
    let metadata = MetadataFixture::new(FixtureInput {
        spell_ids: vec![WRAPPER, LEARNED],
        effects: vec![
            learn_effect(1, WRAPPER, 0, LEARNED),
            skill_effect(2, WRAPPER, 1, SKILL, 1),
        ],
        cast_safe_spell_ids: vec![WRAPPER],
        skill_lines: vec![skill_line(SKILL, 0, 0)],
        skill_race_class: vec![race_class(SKILL as u16, 1, 10)],
        skill_tiers: vec![tiers(10)],
        ..Default::default()
    });

    let plan = deterministic(project_spell_acquisition_like_cpp(
        &snapshot_with_cast(WRAPPER, true, [0]),
        metadata.metadata(),
        SpellAcquisitionRootLikeCpp::TrainerWrapperCast(WRAPPER),
    ));

    let skill_action = plan
        .post_commit_actions
        .iter()
        .position(|action| {
            *action
                == SpellAcquisitionPostCommitActionLikeCpp::UpdateSkillRaisedCriteria {
                    skill_id: SKILL,
                }
        })
        .expect("SKILL hit phase");
    let learned_action = plan
        .post_commit_actions
        .iter()
        .position(|action| {
            *action
                == SpellAcquisitionPostCommitActionLikeCpp::UpdateLearnOrKnowSpellCriteria {
                    spell_id: LEARNED,
                }
        })
        .expect("LEARN hit-target phase");
    assert!(skill_action < learned_action);
}

#[test]
fn wrapper_orders_two_hit_skills_by_effect_index_before_a_state_observing_learn() {
    const WRAPPER: u32 = 200;
    const LEARNED: u32 = 300;
    const FIRST_SKILL: u32 = 164;
    const SECOND_SKILL: u32 = 165;
    let metadata = MetadataFixture::new(FixtureInput {
        spell_ids: vec![WRAPPER, LEARNED],
        // Deliberately oppose source order to EffectIndex. The effective
        // projection and both C++ handler phases are ordered by EffectIndex.
        effects: vec![
            skill_effect(30, WRAPPER, 2, SECOND_SKILL, 1),
            learn_effect(10, WRAPPER, 0, LEARNED),
            skill_effect(20, WRAPPER, 1, FIRST_SKILL, 1),
        ],
        learn_skills: vec![(
            LEARNED,
            SpellLearnSkillNodeLikeCpp {
                skill: FIRST_SKILL as u16,
                step: 2,
                value: 1,
                maxvalue: 150,
            },
        )],
        cast_safe_spell_ids: vec![WRAPPER],
        skill_lines: vec![
            skill_line(FIRST_SKILL, 0, 0),
            skill_line(SECOND_SKILL, 0, 0),
        ],
        skill_race_class: vec![
            race_class(FIRST_SKILL as u16, 1, 10),
            race_class(SECOND_SKILL as u16, 2, 10),
        ],
        skill_tiers: vec![tiers(10)],
        ..Default::default()
    });
    let input = snapshot_with_cast(WRAPPER, true, [0]);
    let before = input.clone();
    let project = || {
        project_spell_acquisition_like_cpp(
            &input,
            metadata.metadata(),
            SpellAcquisitionRootLikeCpp::TrainerWrapperCast(WRAPPER),
        )
    };

    let first_outcome = project();
    let second_outcome = project();
    assert_eq!(first_outcome, second_outcome);
    assert_eq!(input, before);
    let plan = deterministic(first_outcome);

    let wrapper_skill_mutations = plan
        .mutations
        .iter()
        .enumerate()
        .filter_map(|(position, mutation)| match mutation {
            PlannedAcquisitionMutationLikeCpp::Skill(transition) => match &transition.provenance {
                SpellAcquisitionProvenanceLikeCpp::WrapperEffect {
                    wrapper_spell_id,
                    effect_index,
                    ..
                } if *wrapper_spell_id == WRAPPER => {
                    Some((position, *effect_index, transition.skill_id))
                }
                _ => None,
            },
            PlannedAcquisitionMutationLikeCpp::Spell(_)
            | PlannedAcquisitionMutationLikeCpp::Override(_) => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        wrapper_skill_mutations
            .iter()
            .map(|(_, effect_index, skill_id)| (*effect_index, *skill_id))
            .collect::<Vec<_>>(),
        vec![(1, FIRST_SKILL), (2, SECOND_SKILL)]
    );

    let learned_position = plan
        .mutations
        .iter()
        .position(|mutation| {
            matches!(
                mutation,
                PlannedAcquisitionMutationLikeCpp::Spell(transition)
                    if transition.spell_id == LEARNED
                        && matches!(
                            &transition.provenance,
                            SpellAcquisitionProvenanceLikeCpp::WrapperEffect {
                                wrapper_spell_id,
                                effect_index: 0,
                                ..
                            } if *wrapper_spell_id == WRAPPER
                        )
            )
        })
        .expect("LEARN_SPELL target-phase mutation");
    assert!(
        wrapper_skill_mutations
            .iter()
            .all(|(position, _, _)| *position < learned_position)
    );

    let observed_prior_skill = plan
        .skill_transitions
        .iter()
        .find(|transition| {
            transition.skill_id == FIRST_SKILL
                && matches!(
                    &transition.provenance,
                    SpellAcquisitionProvenanceLikeCpp::DirectLearnSkill {
                        source_spell_id,
                    } if *source_spell_id == LEARNED
                )
        })
        .and_then(|transition| transition.before)
        .expect("the learned spell observes the skill inserted by HANDLE_HIT");
    assert_eq!(observed_prior_skill.step, 1);
}

#[test]
fn acquisition_cast_requires_player_resolution_even_with_safe_static_authority() {
    const WRAPPER: u32 = 200;
    const LEARNED: u32 = 300;
    let metadata = MetadataFixture::new(FixtureInput {
        spell_ids: vec![WRAPPER, LEARNED],
        effects: vec![learn_effect(1, WRAPPER, 0, LEARNED)],
        cast_safe_spell_ids: vec![WRAPPER],
        ..Default::default()
    });

    assert_eq!(
        project_spell_acquisition_like_cpp(
            &snapshot(),
            metadata.metadata(),
            SpellAcquisitionRootLikeCpp::TrainerWrapperCast(WRAPPER),
        ),
        SpellAcquisitionOutcomeLikeCpp::Indeterminate(
            SpellAcquisitionIndeterminateLikeCpp::MissingCastResolution { spell_id: WRAPPER },
        )
    );
}

#[test]
fn cast_stopped_before_immediate_phase_projects_no_acquisition_change() {
    const WRAPPER: u32 = 200;
    const LEARNED: u32 = 300;
    const SKILL: u32 = 164;
    let metadata = MetadataFixture::new(FixtureInput {
        spell_ids: vec![WRAPPER, LEARNED],
        effects: vec![
            learn_effect(1, WRAPPER, 0, LEARNED),
            skill_effect(2, WRAPPER, 1, SKILL, 1),
        ],
        cast_safe_spell_ids: vec![WRAPPER],
        skill_lines: vec![skill_line(SKILL, 0, 0)],
        skill_race_class: vec![race_class(SKILL as u16, 1, 10)],
        skill_tiers: vec![tiers(10)],
        ..Default::default()
    });

    let plan = deterministic(project_spell_acquisition_like_cpp(
        &snapshot_with_cast(WRAPPER, false, std::iter::empty::<u8>()),
        metadata.metadata(),
        SpellAcquisitionRootLikeCpp::TrainerWrapperCast(WRAPPER),
    ));
    assert!(plan.spell_transitions.is_empty());
    assert!(plan.skill_transitions.is_empty());
    assert!(plan.post_commit_actions.is_empty());
    assert!(plan.diagnostics.contains(
        &SpellAcquisitionDiagnosticLikeCpp::CastStoppedBeforeImmediatePhase { spell_id: WRAPPER },
    ));
}

#[test]
fn immediate_skill_executes_while_immune_learn_target_is_suppressed() {
    const WRAPPER: u32 = 200;
    const LEARNED: u32 = 300;
    const SKILL: u32 = 164;
    let metadata = MetadataFixture::new(FixtureInput {
        spell_ids: vec![WRAPPER, LEARNED],
        effects: vec![
            learn_effect(1, WRAPPER, 0, LEARNED),
            skill_effect(2, WRAPPER, 1, SKILL, 1),
        ],
        cast_safe_spell_ids: vec![WRAPPER],
        skill_lines: vec![skill_line(SKILL, 0, 0)],
        skill_race_class: vec![race_class(SKILL as u16, 1, 10)],
        skill_tiers: vec![tiers(10)],
        ..Default::default()
    });

    let plan = deterministic(project_spell_acquisition_like_cpp(
        &snapshot_with_cast(WRAPPER, true, std::iter::empty::<u8>()),
        metadata.metadata(),
        SpellAcquisitionRootLikeCpp::TrainerWrapperCast(WRAPPER),
    ));
    assert_eq!(
        plan.resulting_snapshot
            .skills
            .iter()
            .map(|skill| skill.skill_id)
            .collect::<Vec<_>>(),
        vec![SKILL]
    );
    assert!(
        !plan
            .resulting_snapshot
            .spells
            .iter()
            .any(|spell| spell.spell_id == LEARNED)
    );
    assert!(plan.diagnostics.contains(
        &SpellAcquisitionDiagnosticLikeCpp::HitTargetEffectSuppressed {
            spell_id: WRAPPER,
            effect_index: 0,
        },
    ));
}

#[test]
fn direct_learning_a_wrapper_and_casting_it_as_trainer_have_distinct_roots() {
    const WRAPPER: u32 = 200;
    const LEARNED: u32 = 300;
    let metadata = MetadataFixture::new(FixtureInput {
        spell_ids: vec![WRAPPER, LEARNED],
        effects: vec![learn_effect(1, WRAPPER, 0, LEARNED)],
        cast_safe_spell_ids: vec![WRAPPER],
        ..Default::default()
    });

    let direct = deterministic(project_spell_acquisition_like_cpp(
        &snapshot(),
        metadata.metadata(),
        SpellAcquisitionRootLikeCpp::DirectLearn(WRAPPER),
    ));
    assert_eq!(
        direct
            .resulting_snapshot
            .spells
            .iter()
            .map(|spell| spell.spell_id)
            .collect::<Vec<_>>(),
        vec![WRAPPER]
    );

    let trainer = deterministic(project_spell_acquisition_like_cpp(
        &snapshot_with_cast(WRAPPER, true, [0]),
        metadata.metadata(),
        SpellAcquisitionRootLikeCpp::TrainerWrapperCast(WRAPPER),
    ));
    assert_eq!(
        trainer
            .resulting_snapshot
            .spells
            .iter()
            .map(|spell| spell.spell_id)
            .collect::<Vec<_>>(),
        vec![LEARNED]
    );
}

#[test]
fn runtime_dispatched_cast_effect_is_never_silently_treated_as_noop() {
    const WRAPPER: u32 = 500;
    let metadata = MetadataFixture::new(FixtureInput {
        spell_ids: vec![WRAPPER],
        effects: vec![effect(1, WRAPPER, 0, SPELL_EFFECT_TRIGGER_SPELL_LIKE_CPP)],
        cast_safe_spell_ids: vec![WRAPPER],
        ..Default::default()
    });

    assert_eq!(
        project_spell_acquisition_like_cpp(
            &snapshot(),
            metadata.metadata(),
            SpellAcquisitionRootLikeCpp::TrainerWrapperCast(WRAPPER),
        ),
        SpellAcquisitionOutcomeLikeCpp::Indeterminate(
            SpellAcquisitionIndeterminateLikeCpp::UnsupportedRuntimeEffect {
                spell_id: WRAPPER,
                effect_index: 0,
                effect_type: SPELL_EFFECT_TRIGGER_SPELL_LIKE_CPP,
            },
        )
    );
}

#[test]
fn battle_pet_summon_path_is_indeterminate_and_preserves_the_input() {
    const WRAPPER: u32 = 500;
    const CREATURE: u32 = 900;
    const SUMMON_PROPERTIES: u32 = 700;
    const SPECIES: u32 = 50;
    let mut summon = effect(1, WRAPPER, 3, SPELL_EFFECT_SUMMON_LIKE_CPP);
    summon.effect_misc_value_raw = [i64::from(CREATURE), i64::from(SUMMON_PROPERTIES)];
    let mut metadata = MetadataFixture::new(FixtureInput {
        spell_ids: vec![WRAPPER],
        effects: vec![summon.clone()],
        cast_safe_spell_ids: vec![WRAPPER],
        ..Default::default()
    });
    metadata.catalog = SpellAcquisitionCatalogLikeCpp::from_effective_rows_like_cpp(
        [SpellAcquisitionCoverageSeedLikeCpp::covered(WRAPPER, 0)],
        EffectiveSpellAcquisitionRowsLikeCpp {
            spell_effects: vec![summon],
            summon_properties: vec![wow_data::SpellAcquisitionSummonPropertiesLikeCpp {
                record_id: SUMMON_PROPERTIES,
                // C++ SummonProperties::Slot::Minipet plus
                // SUMMON_PROP_FLAG_1_SUMMON_FROM_BATTLE_PET_JOURNAL.
                slot_raw: 5,
                flags_1_raw: 0x0020_0000,
            }],
            battle_pet_species: vec![wow_data::SpellAcquisitionBattlePetSpeciesLikeCpp {
                species_id: SPECIES,
                creature_id_raw: i64::from(CREATURE),
            }],
            ..Default::default()
        },
        SpellAcquisitionTableHashesLikeCpp::default(),
        Vec::new(),
    );
    assert_eq!(
        metadata.catalog.battle_pet_classification_like_cpp(WRAPPER),
        wow_data::BattlePetClassificationLikeCpp::Species(SPECIES)
    );
    let input = snapshot();
    let before = input.clone();

    let outcome = project_spell_acquisition_like_cpp(
        &input,
        metadata.metadata(),
        SpellAcquisitionRootLikeCpp::TrainerWrapperCast(WRAPPER),
    );

    assert_eq!(input, before);
    assert_eq!(
        outcome,
        SpellAcquisitionOutcomeLikeCpp::Indeterminate(
            SpellAcquisitionIndeterminateLikeCpp::BattlePetOrSummonPath {
                spell_id: WRAPPER,
                effect_index: 3,
            }
        )
    );
}

#[test]
fn every_cast_audit_evidence_blocker_maps_to_its_typed_reason() {
    type MutateEvidence = fn(&mut SpellAcquisitionCastAuditEvidenceLikeCpp);
    let cases: [(
        &'static str,
        SpellAcquisitionCastIndeterminateReasonLikeCpp,
        MutateEvidence,
    ); 21] = [
        (
            "script binding",
            SpellAcquisitionCastIndeterminateReasonLikeCpp::ScriptBinding,
            |row| row.has_script_binding = true,
        ),
        (
            "legacy spell_script command",
            SpellAcquisitionCastIndeterminateReasonLikeCpp::LegacySpellScriptCommand,
            |row| row.has_legacy_spell_script_command = true,
        ),
        (
            "pet aura",
            SpellAcquisitionCastIndeterminateReasonLikeCpp::SpellPetAura,
            |row| row.has_spell_pet_aura = true,
        ),
        (
            "linked cast",
            SpellAcquisitionCastIndeterminateReasonLikeCpp::LinkedCast,
            |row| row.has_linked_cast = true,
        ),
        (
            "linked hit",
            SpellAcquisitionCastIndeterminateReasonLikeCpp::LinkedHit,
            |row| row.has_linked_hit = true,
        ),
        (
            "linked aura",
            SpellAcquisitionCastIndeterminateReasonLikeCpp::LinkedAura,
            |row| row.has_linked_aura = true,
        ),
        (
            "cast condition",
            SpellAcquisitionCastIndeterminateReasonLikeCpp::CastCondition,
            |row| row.has_cast_condition = true,
        ),
        (
            "target condition",
            SpellAcquisitionCastIndeterminateReasonLikeCpp::TargetCondition,
            |row| row.has_target_condition = true,
        ),
        (
            "class options modifier",
            SpellAcquisitionCastIndeterminateReasonLikeCpp::SpellModifierClassOptions,
            |row| row.has_spell_modifier_class_options = true,
        ),
        (
            "label modifier",
            SpellAcquisitionCastIndeterminateReasonLikeCpp::SpellModifierLabel,
            |row| row.has_spell_modifier_label = true,
        ),
        (
            "aura 195 learn-spell",
            SpellAcquisitionCastIndeterminateReasonLikeCpp::AuraLearnSpell,
            |row| row.has_aura_learn_spell = true,
        ),
        (
            "runtime CalcValue",
            SpellAcquisitionCastIndeterminateReasonLikeCpp::RuntimeCalcValue,
            |row| row.has_runtime_calc_value = true,
        ),
        (
            "disabled spell",
            SpellAcquisitionCastIndeterminateReasonLikeCpp::DisabledSpell,
            |row| row.is_disabled = true,
        ),
        (
            "hardcoded dummy handler",
            SpellAcquisitionCastIndeterminateReasonLikeCpp::HardcodedDummyHandler,
            |row| row.has_hardcoded_dummy_handler = true,
        ),
        (
            "delayed or channelled",
            SpellAcquisitionCastIndeterminateReasonLikeCpp::DelayedOrChanneled,
            |row| row.is_delayed_or_channeled = true,
        ),
        (
            "unsupported target selection",
            SpellAcquisitionCastIndeterminateReasonLikeCpp::UnsupportedTargetSelection,
            |row| row.has_unsupported_target_selection = true,
        ),
        (
            "unmodelled CheckCast",
            SpellAcquisitionCastIndeterminateReasonLikeCpp::UnmodelledCheckCast,
            |row| row.has_unmodelled_check_cast = true,
        ),
        (
            "runtime mutation before causal closure",
            SpellAcquisitionCastIndeterminateReasonLikeCpp::RuntimeStateMutationBeforeClosure,
            |row| row.has_runtime_state_mutation_before_closure = true,
        ),
        (
            "incomplete authority",
            SpellAcquisitionCastIndeterminateReasonLikeCpp::IncompleteAuthority,
            |row| row.all_sources_complete = false,
        ),
        (
            "passive prerequisites",
            SpellAcquisitionCastIndeterminateReasonLikeCpp::PassiveCastPrerequisites,
            |row| {
                row.is_passive_cast = true;
                row.passive_cast_prerequisites_proven = false;
            },
        ),
        (
            "ordinary passive evidence remains complete",
            SpellAcquisitionCastIndeterminateReasonLikeCpp::IncompleteAuthority,
            |row| {
                row.is_passive_cast = false;
                row.passive_cast_prerequisites_proven = false;
                row.all_sources_complete = false;
            },
        ),
    ];

    for (name, expected_reason, mutate) in cases {
        const SPELL: u32 = 500;
        let mut evidence = cast_evidence(SPELL);
        mutate(&mut evidence);
        let authority = SpellAcquisitionCastAuthorityLikeCpp::from_evidence_like_cpp([evidence]);
        assert_eq!(
            authority.require_safe_like_cpp(SPELL),
            Err(SpellAcquisitionIndeterminateLikeCpp::CastAuthority {
                spell_id: SPELL,
                reasons: vec![expected_reason],
            }),
            "{name}"
        );
    }

    const SAFE: u32 = 501;
    let authority =
        SpellAcquisitionCastAuthorityLikeCpp::from_evidence_like_cpp([cast_evidence(SAFE)]);
    assert_eq!(authority.require_safe_like_cpp(SAFE), Ok(()));
}

#[test]
fn passive_with_projected_acquisition_has_no_runtime_refresh_or_second_cast_owner() {
    const PASSIVE: u32 = 500;
    const LEARNED: u32 = 600;
    const SPELL_ATTR0_PASSIVE: i64 = 0x40;
    let metadata = MetadataFixture::new(FixtureInput {
        spell_ids: vec![PASSIVE, LEARNED],
        effects: vec![learn_effect(1, PASSIVE, 0, LEARNED)],
        misc_rows: vec![misc(PASSIVE, SPELL_ATTR0_PASSIVE, 0, 0)],
        cast_safe_spell_ids: vec![PASSIVE],
        ..Default::default()
    });

    let plan = deterministic(project_spell_acquisition_like_cpp(
        &snapshot_with_cast(PASSIVE, true, [0]),
        metadata.metadata(),
        SpellAcquisitionRootLikeCpp::DirectLearn(PASSIVE),
    ));

    assert_eq!(
        plan.diagnostics,
        vec![
            SpellAcquisitionDiagnosticLikeCpp::AcquisitionCastProjected {
                spell_id: PASSIVE,
                reason: PlannedAcquisitionCastReasonLikeCpp::PassiveLearn,
            }
        ]
    );
    assert!(!plan.post_commit_actions.iter().any(|action| {
        matches!(
            action,
            SpellAcquisitionPostCommitActionLikeCpp::RefreshPassive { spell_id: PASSIVE }
        )
    }));
    assert_eq!(
        plan.spell_transitions
            .iter()
            .filter(|transition| transition.spell_id == LEARNED)
            .count(),
        1
    );
}

#[test]
fn passive_without_projected_acquisition_retains_runtime_refresh_intent() {
    const PASSIVE: u32 = 500;
    const SPELL_ATTR0_PASSIVE: i64 = 0x40;
    let metadata = MetadataFixture::new(FixtureInput {
        spell_ids: vec![PASSIVE],
        misc_rows: vec![misc(PASSIVE, SPELL_ATTR0_PASSIVE, 0, 0)],
        cast_safe_spell_ids: vec![PASSIVE],
        ..Default::default()
    });

    let plan = deterministic(project_spell_acquisition_like_cpp(
        &snapshot(),
        metadata.metadata(),
        SpellAcquisitionRootLikeCpp::DirectLearn(PASSIVE),
    ));
    assert!(
        plan.post_commit_actions.contains(
            &SpellAcquisitionPostCommitActionLikeCpp::RefreshPassive { spell_id: PASSIVE },
        )
    );
}

#[test]
fn cast_when_learned_with_acquisition_is_projected_instead_of_labeled_harmless() {
    const CAST_WHEN_LEARNED: u32 = 500;
    const LEARNED: u32 = 600;
    const SPELL_ATTR1_CAST_WHEN_LEARNED: i64 = 0x8000_0000;
    let metadata = MetadataFixture::new(FixtureInput {
        spell_ids: vec![CAST_WHEN_LEARNED, LEARNED],
        effects: vec![learn_effect(1, CAST_WHEN_LEARNED, 0, LEARNED)],
        misc_rows: vec![misc(CAST_WHEN_LEARNED, 0, SPELL_ATTR1_CAST_WHEN_LEARNED, 0)],
        cast_safe_spell_ids: vec![CAST_WHEN_LEARNED],
        ..Default::default()
    });
    let input = snapshot_with_cast(CAST_WHEN_LEARNED, true, [0]);
    let before = input.clone();

    let plan = deterministic(project_spell_acquisition_like_cpp(
        &input,
        metadata.metadata(),
        SpellAcquisitionRootLikeCpp::DirectLearn(CAST_WHEN_LEARNED),
    ));

    assert_eq!(input, before);
    assert!(plan.diagnostics.contains(
        &SpellAcquisitionDiagnosticLikeCpp::AcquisitionCastProjected {
            spell_id: CAST_WHEN_LEARNED,
            reason: PlannedAcquisitionCastReasonLikeCpp::CastWhenLearned,
        }
    ));
    assert_eq!(
        plan.spell_transitions
            .iter()
            .filter(|transition| transition.spell_id == LEARNED)
            .count(),
        1
    );
    assert!(!plan.diagnostics.iter().any(|diagnostic| matches!(
        diagnostic,
        SpellAcquisitionDiagnosticLikeCpp::EffectHadNoRuntimeChange {
            spell_id: CAST_WHEN_LEARNED,
            ..
        }
    )));
}
