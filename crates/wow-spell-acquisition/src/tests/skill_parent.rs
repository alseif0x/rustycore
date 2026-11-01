use super::*;

#[test]
fn cyclic_skill_parents_fail_closed_without_exposing_partial_plan() {
    const WRAPPER: u32 = 200;
    const FIRST_SKILL: u32 = 164;
    const SECOND_SKILL: u32 = 165;
    let metadata = MetadataFixture::new(FixtureInput {
        spell_ids: vec![WRAPPER],
        effects: vec![skill_step_effect(1, WRAPPER, 0, FIRST_SKILL, 1)],
        cast_safe_spell_ids: vec![WRAPPER],
        skill_lines: vec![
            skill_line(FIRST_SKILL, SECOND_SKILL, 1),
            skill_line(SECOND_SKILL, FIRST_SKILL, 1),
        ],
        skill_race_class: vec![
            race_class(FIRST_SKILL as u16, 1, 10),
            race_class(SECOND_SKILL as u16, 2, 10),
        ],
        skill_tiers: vec![tiers(10)],
        ..Default::default()
    });
    let input = snapshot();
    let mut input = input;
    input.cast_resolutions.insert(
        WRAPPER,
        PlayerCastAcquisitionResolutionLikeCpp {
            reached_immediate_phase: true,
            executed_hit_target_effect_mask: 1,
            effective_effects: Vec::new(),
            executed_dual_wield_effects: Vec::new(),
        },
    );
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
            SpellAcquisitionIndeterminateLikeCpp::SkillParentCycle {
                skill_ids: vec![FIRST_SKILL, SECOND_SKILL, FIRST_SKILL],
            },
        )
    );
}

#[test]
fn absent_parent_is_inserted_before_pending_child_and_child_reward() {
    const WRAPPER: u32 = 200;
    const PARENT_SKILL: u32 = 164;
    const CHILD_SKILL: u32 = 165;
    const CHILD_REWARD: u32 = 600;
    let metadata = MetadataFixture::new(FixtureInput {
        spell_ids: vec![WRAPPER, CHILD_REWARD],
        effects: vec![skill_step_effect(1, WRAPPER, 0, CHILD_SKILL, 1)],
        cast_safe_spell_ids: vec![WRAPPER],
        skill_lines: vec![
            skill_line(PARENT_SKILL, 0, 0),
            skill_line(CHILD_SKILL, PARENT_SKILL, 1),
        ],
        skill_abilities: vec![ability(
            1,
            CHILD_SKILL as u16,
            CHILD_REWARD,
            SKILL_LINE_ABILITY_LEARNED_ON_SKILL_LEARN_LIKE_CPP,
        )],
        skill_race_class: vec![
            race_class(PARENT_SKILL as u16, 1, 10),
            race_class(CHILD_SKILL as u16, 2, 20),
        ],
        skill_tiers: vec![tiers(10), tiers(20)],
        ..Default::default()
    });

    let plan = deterministic(project_spell_acquisition_like_cpp(
        &snapshot_with_cast(WRAPPER, true, [0]),
        metadata.metadata(),
        SpellAcquisitionRootLikeCpp::TrainerWrapperCast(WRAPPER),
    ));

    let causal_ids = plan
        .mutations
        .iter()
        .filter_map(|mutation| match mutation {
            PlannedAcquisitionMutationLikeCpp::Skill(transition) => {
                Some(("skill", transition.skill_id))
            }
            PlannedAcquisitionMutationLikeCpp::Spell(transition)
                if transition.spell_id == CHILD_REWARD =>
            {
                Some(("spell", transition.spell_id))
            }
            PlannedAcquisitionMutationLikeCpp::Spell(_)
            | PlannedAcquisitionMutationLikeCpp::Override(_) => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        causal_ids,
        vec![
            ("skill", PARENT_SKILL),
            ("skill", CHILD_SKILL),
            ("spell", CHILD_REWARD),
        ]
    );
    assert_eq!(plan.root_primary_profession_skill_ids, vec![PARENT_SKILL]);
}

#[test]
fn absent_parent_with_two_children_creates_sibling_without_a_false_cycle() {
    const WRAPPER: u32 = 200;
    const PARENT_SKILL: u32 = 164;
    const REQUESTED_CHILD: u32 = 165;
    const SIBLING_CHILD: u32 = 166;
    let metadata = MetadataFixture::new(FixtureInput {
        spell_ids: vec![WRAPPER],
        effects: vec![skill_step_effect(1, WRAPPER, 0, REQUESTED_CHILD, 1)],
        cast_safe_spell_ids: vec![WRAPPER],
        skill_lines: vec![
            skill_line(PARENT_SKILL, 0, 0),
            skill_line(REQUESTED_CHILD, PARENT_SKILL, 1),
            skill_line(SIBLING_CHILD, PARENT_SKILL, 1),
        ],
        skill_race_class: vec![
            race_class(PARENT_SKILL as u16, 1, 10),
            race_class(REQUESTED_CHILD as u16, 2, 20),
            race_class(SIBLING_CHILD as u16, 3, 30),
        ],
        skill_tiers: vec![tiers(10), tiers(20), tiers(30)],
        ..Default::default()
    });

    let plan = deterministic(project_spell_acquisition_like_cpp(
        &snapshot_with_cast(WRAPPER, true, [0]),
        metadata.metadata(),
        SpellAcquisitionRootLikeCpp::TrainerWrapperCast(WRAPPER),
    ));

    let causal_skills = plan
        .mutations
        .iter()
        .filter_map(|mutation| match mutation {
            PlannedAcquisitionMutationLikeCpp::Skill(transition) => {
                Some((transition.skill_id, transition.provenance.clone()))
            }
            PlannedAcquisitionMutationLikeCpp::Spell(_)
            | PlannedAcquisitionMutationLikeCpp::Override(_) => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        causal_skills,
        vec![
            (
                SIBLING_CHILD,
                SpellAcquisitionProvenanceLikeCpp::RootChildSkill {
                    parent_skill_id: PARENT_SKILL,
                },
            ),
            (
                PARENT_SKILL,
                SpellAcquisitionProvenanceLikeCpp::ParentSkill {
                    child_skill_id: REQUESTED_CHILD,
                },
            ),
            (
                REQUESTED_CHILD,
                SpellAcquisitionProvenanceLikeCpp::WrapperEffect {
                    wrapper_spell_id: WRAPPER,
                    effect_index: 0,
                    record_id: 1,
                },
            ),
        ]
    );
    assert_eq!(plan.resulting_snapshot.occupied_skill_slots, 3);
    assert_eq!(
        plan.resulting_snapshot
            .skills
            .iter()
            .find(|skill| skill.skill_id == SIBLING_CHILD),
        Some(&PlayerSkillAcquisitionRowLikeCpp {
            skill_id: SIBLING_CHILD,
            step: 0,
            value: 0,
            maximum: 0,
            profession_association: ProfessionAssociationInputLikeCpp::Unassigned,
            state: PlayerSkillPersistenceStateLikeCpp::New,
        })
    );
    assert_eq!(plan.root_primary_profession_skill_ids, vec![PARENT_SKILL]);
}

#[test]
fn direct_absent_root_skill_with_a_tiered_child_is_deterministic() {
    const ROOT_SPELL: u32 = 500;
    const PARENT_SKILL: u32 = 164;
    const CHILD_SKILL: u32 = 165;
    let metadata = MetadataFixture::new(FixtureInput {
        spell_ids: vec![ROOT_SPELL],
        learn_skills: vec![(
            ROOT_SPELL,
            SpellLearnSkillNodeLikeCpp {
                skill: PARENT_SKILL as u16,
                step: 1,
                value: 1,
                maxvalue: 75,
            },
        )],
        skill_lines: vec![
            skill_line(PARENT_SKILL, 0, 0),
            skill_line(CHILD_SKILL, PARENT_SKILL, 1),
        ],
        ..Default::default()
    });

    let project = || {
        deterministic(project_spell_acquisition_like_cpp(
            &snapshot(),
            metadata.metadata(),
            SpellAcquisitionRootLikeCpp::DirectLearn(ROOT_SPELL),
        ))
    };
    let first = project();
    let second = project();

    assert_eq!(first, second);
    assert_eq!(
        first
            .resulting_snapshot
            .skills
            .iter()
            .map(|skill| skill.skill_id)
            .collect::<Vec<_>>(),
        vec![PARENT_SKILL, CHILD_SKILL]
    );
    assert_eq!(
        first
            .mutations
            .iter()
            .filter_map(|mutation| match mutation {
                PlannedAcquisitionMutationLikeCpp::Skill(transition) => {
                    Some(transition.skill_id)
                }
                PlannedAcquisitionMutationLikeCpp::Spell(_)
                | PlannedAcquisitionMutationLikeCpp::Override(_) => None,
            })
            .collect::<Vec<_>>(),
        vec![CHILD_SKILL, PARENT_SKILL]
    );
}

#[test]
fn unhydrated_effective_skill_line_cannot_disappear_from_root_child_expansion() {
    const ROOT_SPELL: u32 = 500;
    const ROOT_SKILL: u32 = 164;
    const UNHYDRATED_SKILL: u32 = 165;
    let metadata = MetadataFixture::new(FixtureInput {
        spell_ids: vec![ROOT_SPELL],
        learn_skills: vec![(
            ROOT_SPELL,
            SpellLearnSkillNodeLikeCpp {
                skill: ROOT_SKILL as u16,
                step: 1,
                value: 1,
                maxvalue: 75,
            },
        )],
        skill_lines: vec![skill_line(ROOT_SKILL, 0, 0)],
        incomplete_skill_line_ids: vec![UNHYDRATED_SKILL],
        ..Default::default()
    });
    let input = snapshot();
    let before = input.clone();

    let outcome = project_spell_acquisition_like_cpp(
        &input,
        metadata.metadata(),
        SpellAcquisitionRootLikeCpp::DirectLearn(ROOT_SPELL),
    );

    assert_eq!(input, before);
    assert_eq!(
        outcome,
        SpellAcquisitionOutcomeLikeCpp::Indeterminate(
            SpellAcquisitionIndeterminateLikeCpp::IncompleteSkillLine {
                skill_id: UNHYDRATED_SKILL,
            },
        )
    );
}

#[test]
fn parent_tier_accepts_cpp_array_boundary_and_rejects_larger_steps_atomically() {
    const ROOT_SPELL: u32 = 500;
    const PARENT_SKILL: u32 = 164;
    const CHILD_SKILL: u32 = 165;

    let boundary_metadata = MetadataFixture::new(FixtureInput {
        spell_ids: vec![ROOT_SPELL],
        learn_skills: vec![(
            ROOT_SPELL,
            SpellLearnSkillNodeLikeCpp {
                skill: CHILD_SKILL as u16,
                step: 1,
                value: 1,
                maxvalue: 75,
            },
        )],
        skill_lines: vec![
            skill_line(PARENT_SKILL, 0, 0),
            skill_line(
                CHILD_SKILL,
                PARENT_SKILL,
                wow_data::MAX_SKILL_STEP_LIKE_CPP as i32,
            ),
        ],
        skill_race_class: vec![race_class(PARENT_SKILL as u16, 1, 10)],
        skill_tiers: vec![tiers(10)],
        ..Default::default()
    });
    let boundary_plan = deterministic(project_spell_acquisition_like_cpp(
        &snapshot(),
        boundary_metadata.metadata(),
        SpellAcquisitionRootLikeCpp::DirectLearn(ROOT_SPELL),
    ));
    assert_eq!(
        boundary_plan
            .resulting_snapshot
            .skills
            .iter()
            .find(|skill| skill.skill_id == PARENT_SKILL)
            .map(|skill| skill.step),
        Some(wow_data::MAX_SKILL_STEP_LIKE_CPP as u16),
    );

    let invalid_step = wow_data::MAX_SKILL_STEP_LIKE_CPP as i32 + 1;
    let invalid_metadata = MetadataFixture::new(FixtureInput {
        spell_ids: vec![ROOT_SPELL],
        learn_skills: vec![(
            ROOT_SPELL,
            SpellLearnSkillNodeLikeCpp {
                skill: CHILD_SKILL as u16,
                step: 1,
                value: 1,
                maxvalue: 75,
            },
        )],
        skill_lines: vec![
            skill_line(PARENT_SKILL, 0, 0),
            skill_line(CHILD_SKILL, PARENT_SKILL, invalid_step),
        ],
        skill_race_class: vec![race_class(PARENT_SKILL as u16, 1, 10)],
        skill_tiers: vec![tiers(10)],
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
                skill_id: PARENT_SKILL,
                step: i64::from(invalid_step),
            },
        )
    );
}

#[test]
fn absent_zero_root_profession_consumes_a_slot_but_zero_children_do_not() {
    const ROOT_SPELL: u32 = 500;
    const ROOT_PROFESSION: u32 = 164;
    const CHILD_PROFESSION: u32 = 165;
    let metadata = MetadataFixture::new(FixtureInput {
        spell_ids: vec![ROOT_SPELL],
        learn_skills: vec![(
            ROOT_SPELL,
            SpellLearnSkillNodeLikeCpp {
                skill: ROOT_PROFESSION as u16,
                step: 0,
                value: 0,
                maxvalue: 0,
            },
        )],
        skill_lines: vec![
            skill_line(ROOT_PROFESSION, 0, 0),
            skill_line(CHILD_PROFESSION, ROOT_PROFESSION, 1),
        ],
        ..Default::default()
    });
    let input = snapshot();
    let before = input.clone();
    let plan = deterministic(project_spell_acquisition_like_cpp(
        &input,
        metadata.metadata(),
        SpellAcquisitionRootLikeCpp::DirectLearn(ROOT_SPELL),
    ));

    assert_eq!(input, before);
    assert_eq!(
        plan.root_primary_profession_skill_ids,
        vec![ROOT_PROFESSION],
        "C++ assigns an absent root profession before checking its zero value"
    );
    assert!(plan.resulting_snapshot.skills.iter().all(|skill| {
        skill.value == 0 && matches!(skill.skill_id, ROOT_PROFESSION | CHILD_PROFESSION)
    }));

    let mut existing = snapshot();
    existing.skills.push(PlayerSkillAcquisitionRowLikeCpp {
        skill_id: ROOT_PROFESSION,
        step: 0,
        value: 0,
        maximum: 0,
        profession_association: ProfessionAssociationInputLikeCpp::Unassigned,
        state: PlayerSkillPersistenceStateLikeCpp::Unchanged,
    });
    existing.occupied_skill_slots = 1;
    let existing_before = existing.clone();
    let existing_plan = deterministic(project_spell_acquisition_like_cpp(
        &existing,
        metadata.metadata(),
        SpellAcquisitionRootLikeCpp::DirectLearn(ROOT_SPELL),
    ));
    assert_eq!(existing, existing_before);
    assert!(
        existing_plan.root_primary_profession_skill_ids.is_empty(),
        "C++ does not enter the absent-skill association branch for an existing zero row"
    );
}
