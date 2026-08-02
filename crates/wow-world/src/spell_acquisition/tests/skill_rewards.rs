use super::*;

#[test]
fn both_new_and_existing_skill_updates_expand_reward_spells() {
    const WRAPPER: u32 = 200;
    const REWARD: u32 = 500;
    const SKILL: u32 = 164;
    let metadata = MetadataFixture::new(FixtureInput {
        spell_ids: vec![WRAPPER, REWARD],
        effects: vec![skill_step_effect(1, WRAPPER, 0, SKILL, 2)],
        cast_safe_spell_ids: vec![WRAPPER],
        skill_lines: vec![skill_line(SKILL, 0, 0)],
        skill_abilities: vec![ability(
            1,
            SKILL as u16,
            REWARD,
            SKILL_LINE_ABILITY_LEARNED_ON_SKILL_LEARN_LIKE_CPP,
        )],
        skill_race_class: vec![race_class(SKILL as u16, 1, 10)],
        skill_tiers: vec![tiers(10)],
        ..Default::default()
    });

    for mut input in [snapshot(), {
        let mut existing = snapshot();
        existing.skills.push(PlayerSkillAcquisitionRowLikeCpp {
            skill_id: SKILL,
            step: 1,
            value: 1,
            maximum: 75,
            profession_association: ProfessionAssociationInputLikeCpp::Unassigned,
            state: PlayerSkillPersistenceStateLikeCpp::Unchanged,
        });
        existing.occupied_skill_slots = 1;
        existing.primary_profession_skill_ids = vec![SKILL];
        existing
    }] {
        let had_existing = !input.skills.is_empty();
        input.cast_resolutions.insert(
            WRAPPER,
            PlayerCastAcquisitionResolutionLikeCpp {
                reached_immediate_phase: true,
                executed_hit_target_effect_mask: 1,
                effective_effects: Vec::new(),
                executed_dual_wield_effects: Vec::new(),
            },
        );
        let plan = deterministic(project_spell_acquisition_like_cpp(
            &input,
            metadata.metadata(),
            SpellAcquisitionRootLikeCpp::TrainerWrapperCast(WRAPPER),
        ));
        assert!(plan.resulting_snapshot.spells.iter().any(|spell| {
            spell.spell_id == REWARD
                && spell.dependent
                && spell.state == PlayerSpellPersistenceStateLikeCpp::New
        }));
        assert_eq!(
            plan.skill_transitions
                .last()
                .expect("skill persistence transition")
                .after
                .state,
            if had_existing {
                PlayerSkillPersistenceStateLikeCpp::Changed
            } else {
                PlayerSkillPersistenceStateLikeCpp::New
            }
        );
        let skill_mutation = plan
            .mutations
            .iter()
            .position(|mutation| {
                matches!(
                    mutation,
                    PlannedAcquisitionMutationLikeCpp::Skill(transition)
                        if transition.skill_id == SKILL
                )
            })
            .expect("skill fields are written before reward expansion");
        let reward_mutation = plan
            .mutations
            .iter()
            .position(|mutation| {
                matches!(
                    mutation,
                    PlannedAcquisitionMutationLikeCpp::Spell(transition)
                        if transition.spell_id == REWARD
                )
            })
            .expect("reward spell mutation");
        assert!(skill_mutation < reward_mutation);
        // Make accidental mutation of the loop-owned input immediately visible.
        assert_eq!(input.skills.is_empty(), !had_existing);
        input.future_player_condition_resolutions.clear();
    }
}

#[test]
fn reward_gate_matrix_pins_accept_and_reject_pairs_for_each_cpp_gate() {
    const ROOT: u32 = 500;
    const SKILL: u32 = 164;
    const INVALID_METHOD: u32 = 601;
    const RACE_REJECT: u32 = 602;
    const CLASS_REJECT: u32 = 603;
    const LEVEL_REJECT: u32 = 604;
    const MIN_RANK_REJECT: u32 = 605;
    const FUTURE_REJECT: u32 = 606;
    const FUTURE_ACCEPT: u32 = 607;
    const METHOD_ACCEPT: u32 = 608;
    const RACE_ACCEPT: u32 = 609;
    const CLASS_ACCEPT: u32 = 610;
    const LEVEL_ACCEPT: u32 = 611;
    const MIN_RANK_ACCEPT: u32 = 612;

    let mut invalid_method = ability(1, SKILL as u16, INVALID_METHOD, 0);
    invalid_method.num_skill_ups = 0;
    let mut race_reject = ability(
        2,
        SKILL as u16,
        RACE_REJECT,
        SKILL_LINE_ABILITY_LEARNED_ON_SKILL_LEARN_LIKE_CPP,
    );
    race_reject.race_mask = 1_i64 << 1;
    let mut class_reject = ability(
        3,
        SKILL as u16,
        CLASS_REJECT,
        SKILL_LINE_ABILITY_LEARNED_ON_SKILL_LEARN_LIKE_CPP,
    );
    class_reject.class_mask = 1_i32 << 1;
    let level_reject = ability(
        4,
        SKILL as u16,
        LEVEL_REJECT,
        SKILL_LINE_ABILITY_LEARNED_ON_SKILL_LEARN_LIKE_CPP,
    );
    let mut min_rank_reject = ability(
        5,
        SKILL as u16,
        MIN_RANK_REJECT,
        SKILL_LINE_ABILITY_LEARNED_ON_SKILL_VALUE_LIKE_CPP,
    );
    min_rank_reject.min_skill_line_rank = 50;
    let mut future_reject = ability(
        6,
        SKILL as u16,
        FUTURE_REJECT,
        SKILL_LINE_ABILITY_REWARDED_FROM_QUEST_LIKE_CPP,
    );
    future_reject.flags = SKILL_LINE_ABILITY_CAN_FALLBACK_TO_LEARNED_ON_SKILL_LEARN_LIKE_CPP;
    let mut future_accept = ability(
        7,
        SKILL as u16,
        FUTURE_ACCEPT,
        SKILL_LINE_ABILITY_REWARDED_FROM_QUEST_LIKE_CPP,
    );
    future_accept.flags = SKILL_LINE_ABILITY_CAN_FALLBACK_TO_LEARNED_ON_SKILL_LEARN_LIKE_CPP;
    let method_accept = ability(
        8,
        SKILL as u16,
        METHOD_ACCEPT,
        SKILL_LINE_ABILITY_LEARNED_ON_SKILL_LEARN_LIKE_CPP,
    );
    let mut race_accept = ability(
        9,
        SKILL as u16,
        RACE_ACCEPT,
        SKILL_LINE_ABILITY_LEARNED_ON_SKILL_LEARN_LIKE_CPP,
    );
    race_accept.race_mask = 1;
    let mut class_accept = ability(
        10,
        SKILL as u16,
        CLASS_ACCEPT,
        SKILL_LINE_ABILITY_LEARNED_ON_SKILL_LEARN_LIKE_CPP,
    );
    class_accept.class_mask = 1;
    let level_accept = ability(
        11,
        SKILL as u16,
        LEVEL_ACCEPT,
        SKILL_LINE_ABILITY_LEARNED_ON_SKILL_LEARN_LIKE_CPP,
    );
    let mut min_rank_accept = ability(
        12,
        SKILL as u16,
        MIN_RANK_ACCEPT,
        SKILL_LINE_ABILITY_LEARNED_ON_SKILL_VALUE_LIKE_CPP,
    );
    min_rank_accept.min_skill_line_rank = 1;

    let metadata = MetadataFixture::new(FixtureInput {
        spell_ids: vec![
            ROOT,
            INVALID_METHOD,
            RACE_REJECT,
            CLASS_REJECT,
            LEVEL_REJECT,
            MIN_RANK_REJECT,
            FUTURE_REJECT,
            FUTURE_ACCEPT,
            METHOD_ACCEPT,
            RACE_ACCEPT,
            CLASS_ACCEPT,
            LEVEL_ACCEPT,
            MIN_RANK_ACCEPT,
        ],
        learn_skills: vec![(
            ROOT,
            SpellLearnSkillNodeLikeCpp {
                skill: SKILL as u16,
                step: 1,
                value: 1,
                maxvalue: 75,
            },
        )],
        misc_rows: vec![misc(FUTURE_REJECT, 0, 0, 1), misc(FUTURE_ACCEPT, 0, 0, 2)],
        level_rows: vec![levels(LEVEL_REJECT, 90, 90), levels(LEVEL_ACCEPT, 80, 80)],
        skill_lines: vec![skill_line(SKILL, 0, 0)],
        skill_abilities: vec![
            invalid_method,
            race_reject,
            class_reject,
            level_reject,
            min_rank_reject,
            future_reject,
            future_accept,
            method_accept,
            race_accept,
            class_accept,
            level_accept,
            min_rank_accept,
        ],
        ..Default::default()
    });
    let mut input = snapshot();
    input.future_player_condition_resolutions = vec![
        PlayerFuturePlayerConditionResolutionLikeCpp {
            condition_id: 1,
            allowed: false,
        },
        PlayerFuturePlayerConditionResolutionLikeCpp {
            condition_id: 2,
            allowed: true,
        },
    ];

    let plan = deterministic(project_spell_acquisition_like_cpp(
        &input,
        metadata.metadata(),
        SpellAcquisitionRootLikeCpp::DirectLearn(ROOT),
    ));

    let gates = plan
        .diagnostics
        .iter()
        .filter_map(|diagnostic| match diagnostic {
            SpellAcquisitionDiagnosticLikeCpp::RewardGateRejected { gate, .. } => Some(*gate),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert!(gates.contains(&"acquire method"));
    assert_eq!(
        gates.iter().filter(|gate| **gate == "race/class").count(),
        2
    );
    assert!(gates.contains(&"player level"));
    assert!(gates.contains(&"minimum skill rank"));
    assert!(gates.contains(&"future player condition"));
    for accepted in [
        FUTURE_ACCEPT,
        METHOD_ACCEPT,
        RACE_ACCEPT,
        CLASS_ACCEPT,
        LEVEL_ACCEPT,
        MIN_RANK_ACCEPT,
    ] {
        assert!(
            plan.resulting_snapshot
                .spells
                .iter()
                .any(|spell| spell.spell_id == accepted),
            "gate unexpectedly rejected spell {accepted}"
        );
    }
    for rejected in [
        INVALID_METHOD,
        RACE_REJECT,
        CLASS_REJECT,
        LEVEL_REJECT,
        MIN_RANK_REJECT,
        FUTURE_REJECT,
    ] {
        assert!(
            !plan
                .resulting_snapshot
                .spells
                .iter()
                .any(|spell| spell.spell_id == rejected),
            "gate unexpectedly admitted spell {rejected}"
        );
    }
}

#[test]
fn skill_reward_race_gates_use_cpp_race_mask_bits() {
    const ROOT: u32 = 500;
    const SKILL: u32 = 164;
    const RACE_34_CANONICAL: u32 = 601;
    const RACE_34_NON_CANONICAL: u32 = 602;
    const RACE_70_CANONICAL: u32 = 603;

    let mut race_34_canonical = ability(
        1,
        SKILL as u16,
        RACE_34_CANONICAL,
        SKILL_LINE_ABILITY_LEARNED_ON_SKILL_LEARN_LIKE_CPP,
    );
    // C++ `RaceMask::GetRaceBit(34)` maps Dark Iron Dwarf to bit 11.
    race_34_canonical.race_mask = 1_i64 << 11;
    let mut race_34_non_canonical = ability(
        2,
        SKILL as u16,
        RACE_34_NON_CANONICAL,
        SKILL_LINE_ABILITY_LEARNED_ON_SKILL_LEARN_LIKE_CPP,
    );
    // A direct `race - 1` shift would incorrectly accept this bit.
    race_34_non_canonical.race_mask = 1_i64 << 33;
    let mut race_70_canonical = ability(
        3,
        SKILL as u16,
        RACE_70_CANONICAL,
        SKILL_LINE_ABILITY_LEARNED_ON_SKILL_LEARN_LIKE_CPP,
    );
    // C++ `RaceMask::GetRaceBit(70)` maps Horde Dracthyr to bit 15.
    race_70_canonical.race_mask = 1_i64 << 15;

    let metadata = MetadataFixture::new(FixtureInput {
        spell_ids: vec![
            ROOT,
            RACE_34_CANONICAL,
            RACE_34_NON_CANONICAL,
            RACE_70_CANONICAL,
        ],
        learn_skills: vec![(
            ROOT,
            SpellLearnSkillNodeLikeCpp {
                skill: SKILL as u16,
                step: 1,
                value: 1,
                maxvalue: 75,
            },
        )],
        skill_lines: vec![skill_line(SKILL, 0, 0)],
        skill_abilities: vec![race_34_canonical, race_34_non_canonical, race_70_canonical],
        ..Default::default()
    });

    for (race, expected_spell) in [(34, RACE_34_CANONICAL), (70, RACE_70_CANONICAL)] {
        let mut input = snapshot();
        input.race = race;
        let plan = deterministic(project_spell_acquisition_like_cpp(
            &input,
            metadata.metadata(),
            SpellAcquisitionRootLikeCpp::DirectLearn(ROOT),
        ));
        let rewarded_spells = plan
            .spell_transitions
            .iter()
            .filter_map(|transition| match &transition.provenance {
                SpellAcquisitionProvenanceLikeCpp::SkillReward { .. } => Some(transition.spell_id),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(rewarded_spells, vec![expected_spell]);
        assert!(!rewarded_spells.contains(&RACE_34_NON_CANONICAL));
    }
}

#[test]
fn non_player_race_and_class_ids_fail_closed_before_planning() {
    const ROOT: u32 = 500;
    let metadata = MetadataFixture::new(FixtureInput {
        spell_ids: vec![ROOT],
        ..Default::default()
    });

    for (field, value, input) in [
        ("race", 33, {
            let mut input = snapshot();
            input.race = 33;
            input
        }),
        ("class", 15, {
            let mut input = snapshot();
            input.class = 15;
            input
        }),
    ] {
        assert_eq!(
            project_spell_acquisition_like_cpp(
                &input,
                metadata.metadata(),
                SpellAcquisitionRootLikeCpp::DirectLearn(ROOT),
            ),
            SpellAcquisitionOutcomeLikeCpp::Indeterminate(
                SpellAcquisitionIndeterminateLikeCpp::InvalidSnapshot { field, value },
            )
        );
    }
}

#[test]
fn multiple_skill_rewards_follow_effective_record_id_order() {
    const ROOT: u32 = 500;
    const SKILL: u32 = 164;
    const FIRST: u32 = 601;
    const SECOND: u32 = 602;
    const THIRD: u32 = 603;
    let metadata = MetadataFixture::new(FixtureInput {
        spell_ids: vec![ROOT, FIRST, SECOND, THIRD],
        learn_skills: vec![(
            ROOT,
            SpellLearnSkillNodeLikeCpp {
                skill: SKILL as u16,
                step: 1,
                value: 1,
                maxvalue: 75,
            },
        )],
        skill_lines: vec![skill_line(SKILL, 0, 0)],
        // #163 publishes the final effective index in ascending RecordID;
        // the planner must preserve that causal order exactly.
        skill_abilities: vec![
            ability(
                10,
                SKILL as u16,
                FIRST,
                SKILL_LINE_ABILITY_LEARNED_ON_SKILL_LEARN_LIKE_CPP,
            ),
            ability(
                20,
                SKILL as u16,
                SECOND,
                SKILL_LINE_ABILITY_LEARNED_ON_SKILL_LEARN_LIKE_CPP,
            ),
            ability(
                30,
                SKILL as u16,
                THIRD,
                SKILL_LINE_ABILITY_LEARNED_ON_SKILL_LEARN_LIKE_CPP,
            ),
        ],
        ..Default::default()
    });

    let plan = deterministic(project_spell_acquisition_like_cpp(
        &snapshot(),
        metadata.metadata(),
        SpellAcquisitionRootLikeCpp::DirectLearn(ROOT),
    ));
    assert_eq!(
        plan.mutations
            .iter()
            .filter_map(|mutation| match mutation {
                PlannedAcquisitionMutationLikeCpp::Spell(transition) =>
                    match &transition.provenance {
                        SpellAcquisitionProvenanceLikeCpp::SkillReward { record_id, .. } => {
                            Some((*record_id, transition.spell_id))
                        }
                        _ => None,
                    },
                _ => None,
            })
            .collect::<Vec<_>>(),
        vec![(10, FIRST), (20, SECOND), (30, THIRD)]
    );
}

#[test]
fn repeated_future_condition_consumes_distinct_causal_results_and_preserves_tape() {
    const ROOT: u32 = 500;
    const SKILL: u32 = 164;
    const FIRST: u32 = 601;
    const INTERMEDIATE: u32 = 602;
    const SECOND: u32 = 603;
    const CONDITION: u32 = 77;

    let mut first = ability(
        1,
        SKILL as u16,
        FIRST,
        SKILL_LINE_ABILITY_REWARDED_FROM_QUEST_LIKE_CPP,
    );
    first.flags = SKILL_LINE_ABILITY_CAN_FALLBACK_TO_LEARNED_ON_SKILL_LEARN_LIKE_CPP;
    let intermediate = ability(
        2,
        SKILL as u16,
        INTERMEDIATE,
        SKILL_LINE_ABILITY_LEARNED_ON_SKILL_LEARN_LIKE_CPP,
    );
    let mut second = ability(
        3,
        SKILL as u16,
        SECOND,
        SKILL_LINE_ABILITY_REWARDED_FROM_QUEST_LIKE_CPP,
    );
    second.flags = SKILL_LINE_ABILITY_CAN_FALLBACK_TO_LEARNED_ON_SKILL_LEARN_LIKE_CPP;
    let metadata = MetadataFixture::new(FixtureInput {
        spell_ids: vec![ROOT, FIRST, INTERMEDIATE, SECOND],
        learn_skills: vec![(
            ROOT,
            SpellLearnSkillNodeLikeCpp {
                skill: SKILL as u16,
                step: 1,
                value: 1,
                maxvalue: 75,
            },
        )],
        misc_rows: vec![
            misc(FIRST, 0, 0, i64::from(CONDITION)),
            misc(SECOND, 0, 0, i64::from(CONDITION)),
        ],
        skill_lines: vec![skill_line(SKILL, 0, 0)],
        skill_abilities: vec![first, intermediate, second],
        ..Default::default()
    });
    let tape = vec![
        PlayerFuturePlayerConditionResolutionLikeCpp {
            condition_id: CONDITION,
            allowed: false,
        },
        PlayerFuturePlayerConditionResolutionLikeCpp {
            condition_id: CONDITION,
            allowed: true,
        },
    ];
    let mut input = snapshot();
    input.future_player_condition_resolutions = tape.clone();

    let plan = deterministic(project_spell_acquisition_like_cpp(
        &input,
        metadata.metadata(),
        SpellAcquisitionRootLikeCpp::DirectLearn(ROOT),
    ));
    assert!(
        !plan
            .resulting_snapshot
            .spells
            .iter()
            .any(|spell| spell.spell_id == FIRST)
    );
    assert!(
        plan.resulting_snapshot
            .spells
            .iter()
            .any(|spell| spell.spell_id == INTERMEDIATE)
    );
    assert!(
        plan.resulting_snapshot
            .spells
            .iter()
            .any(|spell| spell.spell_id == SECOND)
    );
    assert_eq!(
        plan.resulting_snapshot.future_player_condition_resolutions,
        tape
    );

    for (provided, expected) in [
        (
            Vec::new(),
            SpellAcquisitionIndeterminateLikeCpp::MissingFuturePlayerConditionResolution {
                spell_id: FIRST,
                condition_id: CONDITION,
                occurrence_index: 0,
            },
        ),
        (
            vec![PlayerFuturePlayerConditionResolutionLikeCpp {
                condition_id: CONDITION + 1,
                allowed: false,
            }],
            SpellAcquisitionIndeterminateLikeCpp::FuturePlayerConditionResolutionMismatch {
                spell_id: FIRST,
                occurrence_index: 0,
                expected_condition_id: CONDITION,
                actual_condition_id: CONDITION + 1,
            },
        ),
    ] {
        let mut invalid = snapshot();
        invalid.future_player_condition_resolutions = provided;
        let before = invalid.clone();
        let outcome = project_spell_acquisition_like_cpp(
            &invalid,
            metadata.metadata(),
            SpellAcquisitionRootLikeCpp::DirectLearn(ROOT),
        );
        assert_eq!(invalid, before);
        assert_eq!(
            outcome,
            SpellAcquisitionOutcomeLikeCpp::Indeterminate(expected)
        );
    }
}

#[test]
fn riding_reward_requires_on_learn_and_exactly_one_skill_up() {
    const ROOT: u32 = 500;
    const REJECTED_REWARD: u32 = 600;
    const ACCEPTED_REWARD: u32 = 601;
    let mut invalid_riding_reward = ability(
        1,
        SKILL_RIDING_LIKE_CPP,
        REJECTED_REWARD,
        SKILL_LINE_ABILITY_LEARNED_ON_SKILL_VALUE_LIKE_CPP,
    );
    invalid_riding_reward.num_skill_ups = 2;
    let valid_riding_reward = ability(
        2,
        SKILL_RIDING_LIKE_CPP,
        ACCEPTED_REWARD,
        SKILL_LINE_ABILITY_LEARNED_ON_SKILL_LEARN_LIKE_CPP,
    );
    let metadata = MetadataFixture::new(FixtureInput {
        spell_ids: vec![ROOT, REJECTED_REWARD, ACCEPTED_REWARD],
        learn_skills: vec![(
            ROOT,
            SpellLearnSkillNodeLikeCpp {
                skill: SKILL_RIDING_LIKE_CPP,
                step: 1,
                value: 1,
                maxvalue: 75,
            },
        )],
        skill_lines: vec![skill_line(u32::from(SKILL_RIDING_LIKE_CPP), 0, 0)],
        skill_abilities: vec![invalid_riding_reward, valid_riding_reward],
        ..Default::default()
    });

    let plan = deterministic(project_spell_acquisition_like_cpp(
        &snapshot(),
        metadata.metadata(),
        SpellAcquisitionRootLikeCpp::DirectLearn(ROOT),
    ));
    assert!(
        plan.diagnostics
            .contains(&SpellAcquisitionDiagnosticLikeCpp::RewardGateRejected {
                skill_id: u32::from(SKILL_RIDING_LIKE_CPP),
                record_id: 1,
                gate: "riding auto-learn",
            })
    );
    assert!(
        !plan
            .resulting_snapshot
            .spells
            .iter()
            .any(|spell| spell.spell_id == REJECTED_REWARD)
    );
    assert!(
        plan.resulting_snapshot
            .spells
            .iter()
            .any(|spell| spell.spell_id == ACCEPTED_REWARD)
    );
}

#[test]
fn below_minimum_existing_reward_spell_requires_explicit_removal_projection() {
    const ROOT: u32 = 500;
    const REWARD: u32 = 600;
    const SKILL: u32 = 164;
    let mut reward = ability(
        1,
        SKILL as u16,
        REWARD,
        SKILL_LINE_ABILITY_LEARNED_ON_SKILL_VALUE_LIKE_CPP,
    );
    reward.min_skill_line_rank = 50;
    let metadata = MetadataFixture::new(FixtureInput {
        spell_ids: vec![ROOT, REWARD],
        learn_skills: vec![(
            ROOT,
            SpellLearnSkillNodeLikeCpp {
                skill: SKILL as u16,
                step: 1,
                value: 1,
                maxvalue: 75,
            },
        )],
        skill_lines: vec![skill_line(SKILL, 0, 0)],
        skill_abilities: vec![reward],
        ..Default::default()
    });
    let mut input = snapshot();
    input.spells.push(spell_row(REWARD, true, true));
    let before = input.clone();

    let outcome = project_spell_acquisition_like_cpp(
        &input,
        metadata.metadata(),
        SpellAcquisitionRootLikeCpp::DirectLearn(ROOT),
    );
    assert_eq!(input, before);
    assert_eq!(
        outcome,
        SpellAcquisitionOutcomeLikeCpp::Indeterminate(
            SpellAcquisitionIndeterminateLikeCpp::RewardSpellRemovalRequired {
                skill_id: SKILL,
                spell_id: REWARD,
                record_id: 1,
            },
        )
    );
}
