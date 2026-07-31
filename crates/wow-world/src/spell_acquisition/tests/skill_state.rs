use super::*;

#[test]
fn deleted_skill_is_rewarded_before_becoming_changed_without_allocating_a_new_slot() {
    const ROOT_SPELL: u32 = 500;
    const REWARD_SPELL: u32 = 600;
    const SKILL: u32 = 164;
    let metadata = MetadataFixture::new(FixtureInput {
        spell_ids: vec![ROOT_SPELL, REWARD_SPELL],
        learn_skills: vec![(
            ROOT_SPELL,
            SpellLearnSkillNodeLikeCpp {
                skill: SKILL as u16,
                step: 1,
                value: 1,
                maxvalue: 75,
            },
        )],
        skill_lines: vec![skill_line(SKILL, 0, 0)],
        skill_abilities: vec![SkillLineAbilityRecord {
            id: 1,
            race_mask: 0,
            skill_line: SKILL as u16,
            spell: REWARD_SPELL as i32,
            min_skill_line_rank: 0,
            class_mask: 0,
            supercedes_spell: 0,
            acquire_method: SKILL_LINE_ABILITY_LEARNED_ON_SKILL_LEARN_LIKE_CPP,
            trivial_rank_high: 0,
            trivial_rank_low: 0,
            flags: 0,
            num_skill_ups: 1,
            skillup_skill_line_id: 0,
        }],
        ..Default::default()
    });
    let mut input = snapshot();
    input.skills.push(deleted_skill_row(SKILL));
    input.occupied_skill_slots = 1;

    let plan = deterministic(project_spell_acquisition_like_cpp(
        &input,
        metadata.metadata(),
        SpellAcquisitionRootLikeCpp::DirectLearn(ROOT_SPELL),
    ));

    assert_eq!(plan.resulting_snapshot.occupied_skill_slots, 1);
    assert!(plan.resulting_snapshot.spells.iter().any(|spell| {
        spell.spell_id == REWARD_SPELL
            && spell.dependent
            && spell.state == PlayerSpellPersistenceStateLikeCpp::New
    }));
    assert_eq!(plan.skill_transitions.len(), 2);
    assert_eq!(
        plan.skill_transitions[0].before,
        Some(deleted_skill_row(SKILL))
    );
    assert_eq!(
        plan.skill_transitions[0].after.state,
        PlayerSkillPersistenceStateLikeCpp::Deleted
    );
    assert_eq!(
        plan.skill_transitions[1].before,
        Some(plan.skill_transitions[0].after)
    );
    assert_eq!(
        plan.skill_transitions[1].after.state,
        PlayerSkillPersistenceStateLikeCpp::Changed
    );
}

#[test]
fn riding_mount_capability_is_only_refreshed_for_an_existing_skill_increase() {
    const ROOT: u32 = 500;
    let riding = u32::from(SKILL_RIDING_LIKE_CPP);
    let metadata = MetadataFixture::new(FixtureInput {
        spell_ids: vec![ROOT],
        learn_skills: vec![(
            ROOT,
            SpellLearnSkillNodeLikeCpp {
                skill: SKILL_RIDING_LIKE_CPP,
                step: 2,
                value: 150,
                maxvalue: 150,
            },
        )],
        skill_lines: vec![skill_line(riding, 0, 0)],
        ..Default::default()
    });

    let absent = deterministic(project_spell_acquisition_like_cpp(
        &snapshot(),
        metadata.metadata(),
        SpellAcquisitionRootLikeCpp::DirectLearn(ROOT),
    ));
    assert!(
        !absent
            .post_commit_actions
            .contains(&SpellAcquisitionPostCommitActionLikeCpp::UpdateMountCapability)
    );

    let mut existing_input = snapshot();
    existing_input
        .skills
        .push(PlayerSkillAcquisitionRowLikeCpp {
            skill_id: riding,
            step: 1,
            value: 75,
            maximum: 75,
            profession_association: ProfessionAssociationInputLikeCpp::Unassigned,
            state: PlayerSkillPersistenceStateLikeCpp::Unchanged,
        });
    existing_input.occupied_skill_slots = 1;
    let existing = deterministic(project_spell_acquisition_like_cpp(
        &existing_input,
        metadata.metadata(),
        SpellAcquisitionRootLikeCpp::DirectLearn(ROOT),
    ));
    let mount = existing
        .post_commit_actions
        .iter()
        .position(|action| {
            *action == SpellAcquisitionPostCommitActionLikeCpp::UpdateMountCapability
        })
        .expect("existing riding increase refreshes mount capability");
    let raised = existing
        .post_commit_actions
        .iter()
        .position(|action| {
            *action
                == SpellAcquisitionPostCommitActionLikeCpp::UpdateSkillRaisedCriteria {
                    skill_id: riding,
                }
        })
        .expect("SkillRaised criteria");
    let step = existing
        .post_commit_actions
        .iter()
        .position(|action| {
            *action
                == SpellAcquisitionPostCommitActionLikeCpp::UpdateAchieveSkillStepCriteria {
                    skill_id: riding,
                }
        })
        .expect("AchieveSkillStep criteria");
    assert!(mount < raised && raised < step);
}

#[test]
fn existing_zero_profession_association_follows_transitive_new_reward_profession() {
    const ROOT: u32 = 500;
    const REWARD: u32 = 600;
    const EXISTING_PROFESSION: u32 = 164;
    const REWARD_PROFESSION: u32 = 165;
    let metadata = MetadataFixture::new(FixtureInput {
        spell_ids: vec![ROOT, REWARD],
        learn_skills: vec![
            (
                ROOT,
                SpellLearnSkillNodeLikeCpp {
                    skill: EXISTING_PROFESSION as u16,
                    step: 1,
                    value: 1,
                    maxvalue: 75,
                },
            ),
            (
                REWARD,
                SpellLearnSkillNodeLikeCpp {
                    skill: REWARD_PROFESSION as u16,
                    step: 1,
                    value: 1,
                    maxvalue: 75,
                },
            ),
        ],
        skill_lines: vec![
            skill_line(EXISTING_PROFESSION, 0, 0),
            skill_line(REWARD_PROFESSION, 0, 0),
        ],
        skill_abilities: vec![ability(
            1,
            EXISTING_PROFESSION as u16,
            REWARD,
            SKILL_LINE_ABILITY_LEARNED_ON_SKILL_LEARN_LIKE_CPP,
        )],
        ..Default::default()
    });
    let mut input = snapshot();
    input.skills.push(PlayerSkillAcquisitionRowLikeCpp {
        skill_id: EXISTING_PROFESSION,
        step: 0,
        value: 0,
        maximum: 0,
        profession_association: ProfessionAssociationInputLikeCpp::Unassigned,
        state: PlayerSkillPersistenceStateLikeCpp::Unchanged,
    });
    input.occupied_skill_slots = 1;

    let plan = deterministic(project_spell_acquisition_like_cpp(
        &input,
        metadata.metadata(),
        SpellAcquisitionRootLikeCpp::DirectLearn(ROOT),
    ));

    assert_eq!(
        &plan.root_primary_profession_skill_ids,
        &[REWARD_PROFESSION, EXISTING_PROFESSION]
    );

    let analysis = crate::profession::analyze_primary_professions_like_cpp(
        2,
        &metadata.skill_lines,
        std::iter::empty::<crate::profession::PlayerSkillProfessionSnapshotLikeCpp>(),
    )
    .expect("complete primary-profession metadata");
    let capacity = crate::profession::plan_primary_professions_like_cpp(
        &analysis,
        &metadata.skill_lines,
        plan.root_primary_profession_skill_ids,
    )
    .expect("the exact projected closure fits two free profession slots");
    assert_eq!(
        capacity
            .new_professions
            .iter()
            .map(|profession| {
                (
                    profession.skill_id,
                    profession.equipment_slot.map(
                        crate::profession::PrimaryProfessionEquipmentSlotLikeCpp::db_value_like_cpp,
                    ),
                )
            })
            .collect::<Vec<_>>(),
        vec![(REWARD_PROFESSION, Some(0)), (EXISTING_PROFESSION, Some(1))]
    );
}

#[test]
fn repeated_root_primary_profession_is_deduplicated_before_capacity_input() {
    const WRAPPER: u32 = 500;
    const PROFESSION: u32 = 164;
    let metadata = MetadataFixture::new(FixtureInput {
        spell_ids: vec![WRAPPER],
        effects: vec![
            skill_step_effect(1, WRAPPER, 0, PROFESSION, 1),
            skill_step_effect(2, WRAPPER, 1, PROFESSION, 2),
        ],
        cast_safe_spell_ids: vec![WRAPPER],
        skill_lines: vec![skill_line(PROFESSION, 0, 0)],
        skill_race_class: vec![race_class(PROFESSION as u16, 1, 10)],
        skill_tiers: vec![tiers(10)],
        ..Default::default()
    });

    let plan = deterministic(project_spell_acquisition_like_cpp(
        &snapshot_with_cast(WRAPPER, true, [0, 1]),
        metadata.metadata(),
        SpellAcquisitionRootLikeCpp::TrainerWrapperCast(WRAPPER),
    ));
    assert_eq!(
        plan.skill_transitions
            .iter()
            .filter_map(|transition| match &transition.provenance {
                SpellAcquisitionProvenanceLikeCpp::WrapperEffect { record_id, .. }
                    if transition.skill_id == PROFESSION =>
                {
                    Some(*record_id)
                }
                _ => None,
            })
            .collect::<BTreeSet<_>>(),
        BTreeSet::from([1, 2]),
        "both wrapper effects must reach the same profession"
    );
    assert_eq!(plan.root_primary_profession_skill_ids, vec![PROFESSION]);
}

#[test]
fn profession_association_slot_above_cpp_range_is_an_invalid_snapshot() {
    const ROOT_SPELL: u32 = 500;
    const SKILL: u32 = 164;
    let metadata = MetadataFixture::new(FixtureInput {
        spell_ids: vec![ROOT_SPELL],
        ..Default::default()
    });
    let mut input = snapshot();
    input.skills.push(PlayerSkillAcquisitionRowLikeCpp {
        skill_id: SKILL,
        step: 1,
        value: 1,
        maximum: 75,
        profession_association: ProfessionAssociationInputLikeCpp::Slot(2),
        state: PlayerSkillPersistenceStateLikeCpp::Unchanged,
    });
    input.occupied_skill_slots = 1;

    assert_eq!(
        project_spell_acquisition_like_cpp(
            &input,
            metadata.metadata(),
            SpellAcquisitionRootLikeCpp::DirectLearn(ROOT_SPELL),
        ),
        SpellAcquisitionOutcomeLikeCpp::Indeterminate(
            SpellAcquisitionIndeterminateLikeCpp::InvalidSnapshot {
                field: "profession_association",
                value: 2,
            },
        )
    );
}

#[test]
fn deleted_skill_line_id_still_requires_an_occupied_snapshot_slot() {
    const ROOT_SPELL: u32 = 500;
    const SKILL: u32 = 164;
    let metadata = MetadataFixture::new(FixtureInput {
        spell_ids: vec![ROOT_SPELL],
        ..Default::default()
    });
    let mut input = snapshot();
    input.skills.push(deleted_skill_row(SKILL));

    assert_eq!(
        project_spell_acquisition_like_cpp(
            &input,
            metadata.metadata(),
            SpellAcquisitionRootLikeCpp::DirectLearn(ROOT_SPELL),
        ),
        SpellAcquisitionOutcomeLikeCpp::Indeterminate(
            SpellAcquisitionIndeterminateLikeCpp::InvalidSnapshot {
                field: "occupied_skill_slots",
                value: 0,
            },
        )
    );
}

#[test]
fn startup_first_learn_skill_node_wins_when_spell_has_multiple_skill_effects() {
    const SPELL: u32 = 100;
    const FIRST_SKILL: u32 = 164;
    const SECOND_SKILL: u32 = 165;
    let metadata = MetadataFixture::new(FixtureInput {
        spell_ids: vec![SPELL],
        effects: vec![
            skill_effect(1, SPELL, 0, FIRST_SKILL, 1),
            skill_effect(2, SPELL, 1, SECOND_SKILL, 2),
        ],
        learn_skills: vec![(
            SPELL,
            SpellLearnSkillNodeLikeCpp {
                skill: FIRST_SKILL as u16,
                step: 1,
                value: 1,
                maxvalue: 75,
            },
        )],
        skill_lines: vec![
            skill_line(FIRST_SKILL, 0, 0),
            skill_line(SECOND_SKILL, 0, 0),
        ],
        ..Default::default()
    });

    let plan = deterministic(project_spell_acquisition_like_cpp(
        &snapshot(),
        metadata.metadata(),
        SpellAcquisitionRootLikeCpp::DirectLearn(SPELL),
    ));

    assert_eq!(
        plan.resulting_snapshot
            .skills
            .iter()
            .map(|skill| skill.skill_id)
            .collect::<Vec<_>>(),
        vec![FIRST_SKILL]
    );
}

#[test]
fn skill_line_fallback_processes_all_rows_and_from_skill_prevents_recursive_self_learning() {
    const ROOT_SPELL: u32 = 500;
    const REWARD_SPELL: u32 = 600;
    const FIRST_SKILL: u32 = 164;
    const SECOND_SKILL: u32 = 165;
    let metadata = MetadataFixture::new(FixtureInput {
        spell_ids: vec![ROOT_SPELL, REWARD_SPELL],
        learn_skills: vec![(
            ROOT_SPELL,
            SpellLearnSkillNodeLikeCpp {
                skill: FIRST_SKILL as u16,
                step: 1,
                value: 1,
                maxvalue: 75,
            },
        )],
        skill_lines: vec![
            skill_line(FIRST_SKILL, 0, 0),
            skill_line(SECOND_SKILL, 0, 0),
        ],
        skill_abilities: vec![
            ability(
                1,
                FIRST_SKILL as u16,
                REWARD_SPELL,
                SKILL_LINE_ABILITY_LEARNED_ON_SKILL_LEARN_LIKE_CPP,
            ),
            ability(
                2,
                SECOND_SKILL as u16,
                REWARD_SPELL,
                SKILL_LINE_ABILITY_LEARNED_ON_SKILL_LEARN_LIKE_CPP,
            ),
        ],
        skill_race_class: vec![race_class(SECOND_SKILL as u16, 2, 10)],
        skill_tiers: vec![tiers(10)],
        ..Default::default()
    });

    let plan = deterministic(project_spell_acquisition_like_cpp(
        &snapshot(),
        metadata.metadata(),
        SpellAcquisitionRootLikeCpp::DirectLearn(ROOT_SPELL),
    ));

    let skill_ids = plan
        .resulting_snapshot
        .skills
        .iter()
        .map(|skill| skill.skill_id)
        .collect::<BTreeSet<_>>();
    assert_eq!(skill_ids, BTreeSet::from([FIRST_SKILL, SECOND_SKILL]));
    assert_eq!(
        plan.skill_transitions
            .iter()
            .filter(|transition| transition.skill_id == FIRST_SKILL)
            .count(),
        1,
        "fromSkill must suppress the reward spell's fallback to its owning skill"
    );
}

#[test]
fn direct_learn_skill_projects_language_level_mono_and_rank_ranges() {
    const LANGUAGE_SPELL: u32 = 100;
    const LEVEL_SPELL: u32 = 101;
    const MONO_SPELL: u32 = 102;
    const RANK_SPELL: u32 = 103;
    const LANGUAGE_SKILL: u32 = 200;
    const LEVEL_SKILL: u32 = 201;
    const MONO_SKILL: u32 = 202;
    const RANK_SKILL: u32 = 203;
    let metadata = MetadataFixture::new(FixtureInput {
        spell_ids: vec![LANGUAGE_SPELL, LEVEL_SPELL, MONO_SPELL, RANK_SPELL],
        learn_skills: vec![
            (
                LANGUAGE_SPELL,
                SpellLearnSkillNodeLikeCpp {
                    skill: LANGUAGE_SKILL as u16,
                    step: 0,
                    value: 1,
                    maxvalue: 0,
                },
            ),
            (
                LEVEL_SPELL,
                SpellLearnSkillNodeLikeCpp {
                    skill: LEVEL_SKILL as u16,
                    step: 0,
                    value: 1,
                    maxvalue: 0,
                },
            ),
            (
                MONO_SPELL,
                SpellLearnSkillNodeLikeCpp {
                    skill: MONO_SKILL as u16,
                    step: 0,
                    value: 1,
                    maxvalue: 0,
                },
            ),
            (
                RANK_SPELL,
                SpellLearnSkillNodeLikeCpp {
                    skill: RANK_SKILL as u16,
                    step: 2,
                    value: 1,
                    maxvalue: 0,
                },
            ),
        ],
        skill_lines: vec![
            skill_line_with_category(LANGUAGE_SKILL, SKILL_CATEGORY_LANGUAGES_LIKE_CPP, 0, 0),
            skill_line_with_category(LEVEL_SKILL, 6, 0, 0),
            skill_line_with_category(MONO_SKILL, SKILL_CATEGORY_ARMOR_LIKE_CPP, 0, 0),
            skill_line(RANK_SKILL, 0, 0),
        ],
        skill_race_class: vec![
            race_class(LANGUAGE_SKILL as u16, 1, 0),
            race_class(LEVEL_SKILL as u16, 2, 0),
            race_class(MONO_SKILL as u16, 3, 0),
            race_class(RANK_SKILL as u16, 4, 10),
        ],
        skill_tiers: vec![tiers(10)],
        ..Default::default()
    });

    for (spell_id, expected_skill_id, expected_step, expected_value, expected_maximum) in [
        (LANGUAGE_SPELL, LANGUAGE_SKILL, 0, 300, 300),
        (LEVEL_SPELL, LEVEL_SKILL, 0, 1, 400),
        (MONO_SPELL, MONO_SKILL, 0, 1, 1),
        (RANK_SPELL, RANK_SKILL, 2, 1, 150),
    ] {
        let plan = deterministic(project_spell_acquisition_like_cpp(
            &snapshot(),
            metadata.metadata(),
            SpellAcquisitionRootLikeCpp::DirectLearn(spell_id),
        ));
        assert_eq!(plan.resulting_snapshot.skills.len(), 1);
        let skill = plan.resulting_snapshot.skills[0];
        assert_eq!(skill.skill_id, expected_skill_id);
        assert_eq!(skill.step, expected_step);
        assert_eq!(skill.value, expected_value);
        assert_eq!(skill.maximum, expected_maximum);
    }
}

#[test]
fn direct_learn_skill_accepts_cpp_array_boundary_and_rejects_larger_steps_atomically() {
    const ROOT_SPELL: u32 = 500;
    const SKILL: u32 = 164;
    let boundary_step = wow_data::MAX_SKILL_STEP_LIKE_CPP as u16;
    let boundary_metadata = MetadataFixture::new(FixtureInput {
        spell_ids: vec![ROOT_SPELL],
        learn_skills: vec![(
            ROOT_SPELL,
            SpellLearnSkillNodeLikeCpp {
                skill: SKILL as u16,
                step: boundary_step,
                value: 1,
                maxvalue: 75,
            },
        )],
        skill_lines: vec![skill_line(SKILL, 0, 0)],
        ..Default::default()
    });
    let boundary_plan = deterministic(project_spell_acquisition_like_cpp(
        &snapshot(),
        boundary_metadata.metadata(),
        SpellAcquisitionRootLikeCpp::DirectLearn(ROOT_SPELL),
    ));
    assert_eq!(
        boundary_plan.resulting_snapshot.skills[0].step,
        boundary_step
    );

    let invalid_step = boundary_step + 1;
    let invalid_metadata = MetadataFixture::new(FixtureInput {
        spell_ids: vec![ROOT_SPELL],
        learn_skills: vec![(
            ROOT_SPELL,
            SpellLearnSkillNodeLikeCpp {
                skill: SKILL as u16,
                step: invalid_step,
                value: 1,
                maxvalue: 75,
            },
        )],
        skill_lines: vec![skill_line(SKILL, 0, 0)],
        ..Default::default()
    });
    let input = snapshot();
    let before = input.clone();
    let outcome = project_spell_acquisition_like_cpp(
        &input,
        invalid_metadata.metadata(),
        SpellAcquisitionRootLikeCpp::DirectLearn(ROOT_SPELL),
    );

    assert_eq!(input, before);
    assert_eq!(
        outcome,
        SpellAcquisitionOutcomeLikeCpp::Indeterminate(
            SpellAcquisitionIndeterminateLikeCpp::InvalidSkillStep {
                skill_id: SKILL,
                step: i64::from(invalid_step),
            },
        )
    );
}

#[test]
fn missing_and_out_of_range_skill_tiers_are_typed_indeterminate_results() {
    const WRAPPER: u32 = 500;
    const SKILL: u32 = 164;
    for (tiers_rows, expected) in [
        (
            Vec::new(),
            SpellAcquisitionIndeterminateLikeCpp::MissingSkillTier {
                skill_id: SKILL,
                skill_tier_id: 10,
            },
        ),
        (
            vec![{
                let mut value = [0; wow_data::MAX_SKILL_STEP_LIKE_CPP];
                value[0] = u32::from(u16::MAX) + 1;
                SkillTiersRowLikeCpp { id: 10, value }
            }],
            SpellAcquisitionIndeterminateLikeCpp::InvalidSkillTierValue {
                skill_id: SKILL,
                value: u32::from(u16::MAX) + 1,
            },
        ),
    ] {
        let metadata = MetadataFixture::new(FixtureInput {
            spell_ids: vec![WRAPPER],
            effects: vec![skill_step_effect(1, WRAPPER, 0, SKILL, 1)],
            cast_safe_spell_ids: vec![WRAPPER],
            skill_lines: vec![skill_line(SKILL, 0, 0)],
            skill_race_class: vec![race_class(SKILL as u16, 1, 10)],
            skill_tiers: tiers_rows,
            ..Default::default()
        });
        assert_eq!(
            project_spell_acquisition_like_cpp(
                &snapshot_with_cast(WRAPPER, true, [0]),
                metadata.metadata(),
                SpellAcquisitionRootLikeCpp::TrainerWrapperCast(WRAPPER),
            ),
            SpellAcquisitionOutcomeLikeCpp::Indeterminate(expected)
        );
    }
}

#[test]
fn invalid_skill_step_trigger_and_wide_skill_identifier_fail_closed() {
    const WRAPPER: u32 = 500;

    let step_metadata = MetadataFixture::new(FixtureInput {
        spell_ids: vec![WRAPPER],
        effects: vec![skill_step_effect(
            1,
            WRAPPER,
            0,
            164,
            wow_data::MAX_SKILL_STEP_LIKE_CPP as i32 + 1,
        )],
        cast_safe_spell_ids: vec![WRAPPER],
        ..Default::default()
    });
    assert_eq!(
        project_spell_acquisition_like_cpp(
            &snapshot_with_cast(WRAPPER, true, [0]),
            step_metadata.metadata(),
            SpellAcquisitionRootLikeCpp::TrainerWrapperCast(WRAPPER),
        ),
        SpellAcquisitionOutcomeLikeCpp::Indeterminate(
            SpellAcquisitionIndeterminateLikeCpp::InvalidSkillStep {
                skill_id: 164,
                step: wow_data::MAX_SKILL_STEP_LIKE_CPP as i64 + 1,
            },
        )
    );

    let mut invalid_trigger = learn_effect(2, WRAPPER, 0, 1);
    invalid_trigger.effect_trigger_spell_raw = -1;
    let trigger_metadata = MetadataFixture::new(FixtureInput {
        spell_ids: vec![WRAPPER],
        effects: vec![invalid_trigger],
        cast_safe_spell_ids: vec![WRAPPER],
        ..Default::default()
    });
    assert_eq!(
        project_spell_acquisition_like_cpp(
            &snapshot_with_cast(WRAPPER, true, [0]),
            trigger_metadata.metadata(),
            SpellAcquisitionRootLikeCpp::TrainerWrapperCast(WRAPPER),
        ),
        SpellAcquisitionOutcomeLikeCpp::Indeterminate(
            SpellAcquisitionIndeterminateLikeCpp::InvalidEffectiveValue {
                record_id: 2,
                field: "SpellEffect.EffectTriggerSpell",
                raw: -1,
            },
        )
    );

    const TOO_WIDE_SKILL: u32 = u16::MAX as u32 + 1;
    let wide_skill_metadata = MetadataFixture::new(FixtureInput {
        spell_ids: vec![WRAPPER],
        effects: vec![skill_step_effect(3, WRAPPER, 0, TOO_WIDE_SKILL, 1)],
        cast_safe_spell_ids: vec![WRAPPER],
        ..Default::default()
    });
    assert_eq!(
        project_spell_acquisition_like_cpp(
            &snapshot_with_cast(WRAPPER, true, [0]),
            wide_skill_metadata.metadata(),
            SpellAcquisitionRootLikeCpp::TrainerWrapperCast(WRAPPER),
        ),
        SpellAcquisitionOutcomeLikeCpp::Indeterminate(
            SpellAcquisitionIndeterminateLikeCpp::InvalidSkillIdentifier {
                value: i64::from(TOO_WIDE_SKILL),
                source: "SpellEffect.EffectMiscValue",
            },
        )
    );
}

#[test]
fn full_skill_capacity_rejects_new_slot_but_allows_existing_tombstone_reuse() {
    const ROOT: u32 = 500;
    const SKILL: u32 = 1_000;
    let metadata = MetadataFixture::new(FixtureInput {
        spell_ids: vec![ROOT],
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
        ..Default::default()
    });

    let mut incomplete = snapshot();
    incomplete.occupied_skill_slots = 1;
    assert_eq!(
        project_spell_acquisition_like_cpp(
            &incomplete,
            metadata.metadata(),
            SpellAcquisitionRootLikeCpp::DirectLearn(ROOT),
        ),
        SpellAcquisitionOutcomeLikeCpp::Indeterminate(
            SpellAcquisitionIndeterminateLikeCpp::InvalidSnapshot {
                field: "occupied_skill_slots",
                value: 1,
            },
        ),
        "an exact snapshot cannot hide the identity stored in an occupied slot"
    );

    let mut full = snapshot();
    full.skills = (1..=MAX_PLAYER_SKILLS_LIKE_CPP as u32)
        .map(|skill_id| PlayerSkillAcquisitionRowLikeCpp {
            skill_id,
            step: 1,
            value: 1,
            maximum: 1,
            profession_association: ProfessionAssociationInputLikeCpp::Unassigned,
            state: PlayerSkillPersistenceStateLikeCpp::Unchanged,
        })
        .collect();
    full.occupied_skill_slots = MAX_PLAYER_SKILLS_LIKE_CPP as u16;
    let before = full.clone();
    let outcome = project_spell_acquisition_like_cpp(
        &full,
        metadata.metadata(),
        SpellAcquisitionRootLikeCpp::DirectLearn(ROOT),
    );
    assert_eq!(full, before);
    assert_eq!(
        outcome,
        SpellAcquisitionOutcomeLikeCpp::Indeterminate(
            SpellAcquisitionIndeterminateLikeCpp::PlayerSkillCapacityExceeded,
        )
    );

    let mut tombstone = full;
    tombstone.skills.pop();
    tombstone.skills.push(deleted_skill_row(SKILL));
    let plan = deterministic(project_spell_acquisition_like_cpp(
        &tombstone,
        metadata.metadata(),
        SpellAcquisitionRootLikeCpp::DirectLearn(ROOT),
    ));
    assert_eq!(
        plan.resulting_snapshot.occupied_skill_slots,
        MAX_PLAYER_SKILLS_LIKE_CPP as u16
    );
    assert_eq!(
        plan.resulting_snapshot
            .skills
            .iter()
            .find(|skill| skill.skill_id == SKILL)
            .map(|skill| skill.state),
        Some(PlayerSkillPersistenceStateLikeCpp::Changed)
    );
}
