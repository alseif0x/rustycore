//! Production-boundary scenarios retained with the world adapters.
use super::*;

#[test]
fn effect_learn_spell_preserves_base_row_and_defers_unavailable_nested_cast_atomically() {
    const PASSIVE: u32 = 500;
    const NESTED: u32 = 600;
    const SPELL_ATTR0_PASSIVE: i64 = 0x40;
    let metadata = MetadataFixture::new(FixtureInput {
        spell_ids: vec![PASSIVE, NESTED],
        effects: vec![learn_effect(1, PASSIVE, 0, NESTED)],
        misc_rows: vec![misc(PASSIVE, SPELL_ATTR0_PASSIVE, 0, 0)],
        ..Default::default()
    });

    let source = snapshot();
    let plan = deterministic(project_effect_learn_spell_acquisition_like_cpp(
        &source,
        metadata.metadata(),
        PASSIVE,
    ));

    assert!(plan.resulting_snapshot.spells.iter().any(|row| {
        row.spell_id == PASSIVE && row.state != PlayerSpellPersistenceStateLikeCpp::Removed
    }));
    assert!(
        !plan
            .resulting_snapshot
            .spells
            .iter()
            .any(|row| row.spell_id == NESTED)
    );
    assert!(plan.diagnostics.iter().any(|diagnostic| matches!(
        diagnostic,
        SpellAcquisitionDiagnosticLikeCpp::AcquisitionCastDeferred {
            spell_id: PASSIVE,
            reason: PlannedAcquisitionCastReasonLikeCpp::PassiveLearn,
            cause,
        } if matches!(
            cause.as_ref(),
            SpellAcquisitionIndeterminateLikeCpp::CastAuthority {
                spell_id: PASSIVE,
                ..
            }
        )
    )));
    assert!(!plan.diagnostics.iter().any(|diagnostic| matches!(
        diagnostic,
        SpellAcquisitionDiagnosticLikeCpp::AcquisitionCastProjected {
            spell_id: PASSIVE,
            ..
        }
    )));
    let profession_plan = crate::profession::PrimaryProfessionCapacityPlanLikeCpp {
        configured_max: 2,
        used_before: 0,
        free_before: 2,
        existing_professions: Vec::new(),
        new_professions: Vec::new(),
        slot_normalizations: Vec::new(),
    };
    assert!(matches!(
        prepare_player_spell_acquisition_like_cpp(&plan, &profession_plan, &source),
        Ok(PreparedPlayerSpellAcquisitionOutcomeLikeCpp::Ready(_))
    ));
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
fn recursive_three_rank_supersedes_prepare_against_causal_state() {
    const LOWER: u32 = 100;
    const MIDDLE: u32 = 101;
    const HIGHER: u32 = 102;
    let metadata = MetadataFixture::new(FixtureInput {
        spell_ids: vec![LOWER, MIDDLE, HIGHER],
        chains: vec![
            (LOWER, rank_node(None, Some(MIDDLE), LOWER, HIGHER, 1)),
            (
                MIDDLE,
                rank_node(Some(LOWER), Some(HIGHER), LOWER, HIGHER, 2),
            ),
            (HIGHER, rank_node(Some(MIDDLE), None, LOWER, HIGHER, 3)),
        ],
        ..Default::default()
    });
    let source = snapshot();
    let plan = deterministic(project_spell_acquisition_like_cpp(
        &source,
        metadata.metadata(),
        SpellAcquisitionRootLikeCpp::DirectLearn(HIGHER),
    ));
    let profession_plan = crate::profession::PrimaryProfessionCapacityPlanLikeCpp {
        configured_max: 2,
        used_before: 0,
        free_before: 2,
        existing_professions: Vec::new(),
        new_professions: Vec::new(),
        slot_normalizations: Vec::new(),
    };

    assert!(matches!(
        prepare_player_spell_acquisition_like_cpp(&plan, &profession_plan, &source),
        Ok(PreparedPlayerSpellAcquisitionOutcomeLikeCpp::Ready(_))
    ));
    assert_eq!(
        plan.post_commit_actions
            .iter()
            .filter_map(|action| match action {
                SpellAcquisitionPostCommitActionLikeCpp::SupersededSpell {
                    old_spell_id,
                    new_spell_id,
                } => Some((*old_spell_id, *new_spell_id)),
                _ => None,
            })
            .collect::<Vec<_>>(),
        vec![(LOWER, MIDDLE), (MIDDLE, HIGHER)]
    );
}
