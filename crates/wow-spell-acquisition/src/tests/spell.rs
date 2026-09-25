use super::*;

#[test]
fn direct_learn_projects_basic_spell_without_mutating_input_snapshot() {
    let metadata = MetadataFixture::new(FixtureInput {
        spell_ids: vec![100],
        ..Default::default()
    });
    let input = snapshot();
    let before = input.clone();

    let plan = deterministic(project_spell_acquisition_like_cpp(
        &input,
        metadata.metadata(),
        SpellAcquisitionRootLikeCpp::DirectLearn(100),
    ));

    assert_eq!(input, before);
    assert_eq!(
        plan.resulting_snapshot.spells,
        vec![PlayerSpellAcquisitionRowLikeCpp {
            spell_id: 100,
            active: true,
            disabled: false,
            dependent: false,
            favorite: false,
            trait_definition_id: None,
            state: PlayerSpellPersistenceStateLikeCpp::New,
        }]
    );
    assert_eq!(
        plan.spell_transitions
            .iter()
            .map(|transition| transition.spell_id)
            .collect::<Vec<_>>(),
        vec![100]
    );
}

#[test]
fn spell_and_skill_criteria_intents_preserve_cpp_types_order_and_duplicate_skill_lines() {
    const SPELL: u32 = 100;
    const SKILL: u32 = 164;
    let metadata = MetadataFixture::new(FixtureInput {
        spell_ids: vec![SPELL],
        learn_skills: vec![(
            SPELL,
            SpellLearnSkillNodeLikeCpp {
                skill: SKILL as u16,
                step: 1,
                value: 1,
                maxvalue: 75,
            },
        )],
        skill_lines: vec![skill_line(SKILL, 0, 0)],
        skill_abilities: vec![
            ability(1, SKILL as u16, SPELL, 0),
            ability(2, SKILL as u16, SPELL, 0),
        ],
        ..Default::default()
    });

    let plan = deterministic(project_spell_acquisition_like_cpp(
        &snapshot(),
        metadata.metadata(),
        SpellAcquisitionRootLikeCpp::DirectLearn(SPELL),
    ));

    assert_eq!(
        plan.post_commit_actions,
        vec![
            SpellAcquisitionPostCommitActionLikeCpp::UpdateSkillRaisedCriteria { skill_id: SKILL },
            SpellAcquisitionPostCommitActionLikeCpp::UpdateAchieveSkillStepCriteria {
                skill_id: SKILL,
            },
            SpellAcquisitionPostCommitActionLikeCpp::UpdateLearnTradeskillSkillLineCriteria {
                source_spell_id: SPELL,
                skill_id: SKILL,
            },
            SpellAcquisitionPostCommitActionLikeCpp::UpdateLearnSpellFromSkillLineCriteria {
                source_spell_id: SPELL,
                skill_id: SKILL,
            },
            SpellAcquisitionPostCommitActionLikeCpp::UpdateLearnTradeskillSkillLineCriteria {
                source_spell_id: SPELL,
                skill_id: SKILL,
            },
            SpellAcquisitionPostCommitActionLikeCpp::UpdateLearnSpellFromSkillLineCriteria {
                source_spell_id: SPELL,
                skill_id: SKILL,
            },
            SpellAcquisitionPostCommitActionLikeCpp::UpdateLearnOrKnowSpellCriteria {
                spell_id: SPELL,
            },
            SpellAcquisitionPostCommitActionLikeCpp::LearnedSpell {
                spell_id: SPELL,
                favorite: false,
                suppress_messaging: false,
            },
            SpellAcquisitionPostCommitActionLikeCpp::UpdateLearnSpellQuestObjective {
                spell_id: SPELL,
            },
        ]
    );

    let mut loading = snapshot();
    loading.lifecycle = PlayerAcquisitionLifecycleLikeCpp::Loading;
    let plan = deterministic(project_spell_acquisition_like_cpp(
        &loading,
        metadata.metadata(),
        SpellAcquisitionRootLikeCpp::DirectLearn(SPELL),
    ));
    assert!(!plan.post_commit_actions.iter().any(|action| {
        matches!(
            action,
            SpellAcquisitionPostCommitActionLikeCpp::UpdateLearnTradeskillSkillLineCriteria { .. }
                | SpellAcquisitionPostCommitActionLikeCpp::UpdateLearnSpellFromSkillLineCriteria { .. }
                | SpellAcquisitionPostCommitActionLikeCpp::UpdateLearnOrKnowSpellCriteria { .. }
        )
    }));
}

#[test]
fn previous_rank_is_learned_then_superseded_by_requested_rank() {
    let metadata = MetadataFixture::new(FixtureInput {
        spell_ids: vec![100, 101],
        chains: vec![
            (100, rank_node(None, Some(101), 100, 101, 1)),
            (101, rank_node(Some(100), None, 100, 101, 2)),
        ],
        ..Default::default()
    });

    let plan = deterministic(project_spell_acquisition_like_cpp(
        &snapshot(),
        metadata.metadata(),
        SpellAcquisitionRootLikeCpp::DirectLearn(101),
    ));

    let by_id = plan
        .resulting_snapshot
        .spells
        .iter()
        .map(|spell| (spell.spell_id, *spell))
        .collect::<BTreeMap<_, _>>();
    assert!(!by_id[&100].active);
    assert!(by_id[&100].dependent);
    assert!(by_id[&101].active);
    assert!(plan.post_commit_actions.contains(
        &SpellAcquisitionPostCommitActionLikeCpp::SupersededSpell {
            old_spell_id: 100,
            new_spell_id: 101,
        },
    ));
    assert_eq!(
        plan.mutations
            .iter()
            .filter_map(|mutation| match mutation {
                PlannedAcquisitionMutationLikeCpp::Spell(transition) => Some((
                    transition.spell_id,
                    transition.before.map(|spell| spell.active),
                    transition.after.map(|spell| spell.active),
                )),
                PlannedAcquisitionMutationLikeCpp::Skill(_)
                | PlannedAcquisitionMutationLikeCpp::Override(_) => None,
            })
            .collect::<Vec<_>>(),
        vec![
            (100, None, Some(true)),
            (101, None, Some(true)),
            (100, Some(true), Some(false)),
        ],
        "C++ learns the previous rank, inserts the requested rank, then deactivates the old rank"
    );
    assert_eq!(
        plan.post_commit_actions
            .iter()
            .filter_map(|action| match action {
                SpellAcquisitionPostCommitActionLikeCpp::LearnedSpell { spell_id, .. } => {
                    Some(("learned", *spell_id, 0))
                }
                SpellAcquisitionPostCommitActionLikeCpp::SupersededSpell {
                    old_spell_id,
                    new_spell_id,
                } => Some(("superseded", *old_spell_id, *new_spell_id)),
                _ => None,
            })
            .collect::<Vec<_>>(),
        vec![("learned", 100, 0), ("superseded", 100, 101),],
        "C++ uses the supersede packet as the requested rank's publication and AddSpell returns false when it replaced an old active rank"
    );
}

#[test]
fn exact_existing_spell_is_a_noop_and_direct_learning_never_clears_dependent_state() {
    const SPELL: u32 = 100;
    let metadata = MetadataFixture::new(FixtureInput {
        spell_ids: vec![SPELL],
        ..Default::default()
    });

    let mut exact_input = snapshot();
    exact_input.spells.push(spell_row(SPELL, true, false));
    let exact_before = exact_input.clone();
    let exact = deterministic(project_spell_acquisition_like_cpp(
        &exact_input,
        metadata.metadata(),
        SpellAcquisitionRootLikeCpp::DirectLearn(SPELL),
    ));
    assert_eq!(exact_input, exact_before);
    assert_eq!(exact.resulting_snapshot, exact_before);
    assert!(exact.spell_transitions.is_empty());
    assert!(exact.diagnostics.contains(
        &SpellAcquisitionDiagnosticLikeCpp::ExistingSpellAlreadyMatches { spell_id: SPELL }
    ));
    assert!(!exact.post_commit_actions.iter().any(|action| matches!(
        action,
        SpellAcquisitionPostCommitActionLikeCpp::LearnedSpell {
            spell_id: SPELL,
            ..
        }
    )));

    let mut dependent_input = snapshot();
    dependent_input.spells.push(spell_row(SPELL, true, true));
    let dependent_before = dependent_input.clone();
    let dependent = deterministic(project_spell_acquisition_like_cpp(
        &dependent_input,
        metadata.metadata(),
        SpellAcquisitionRootLikeCpp::DirectLearn(SPELL),
    ));
    assert_eq!(dependent_input, dependent_before);
    assert!(dependent.resulting_snapshot.spells[0].dependent);
    assert!(!dependent.post_commit_actions.iter().any(|action| matches!(
        action,
        SpellAcquisitionPostCommitActionLikeCpp::LearnedSpell {
            spell_id: SPELL,
            ..
        }
    )));
}

#[test]
fn existing_spell_is_upgraded_to_dependent_through_learning_edge() {
    let metadata = MetadataFixture::new(FixtureInput {
        spell_ids: vec![100, 200],
        dependencies: vec![dependency(1, 200, 100)],
        ..Default::default()
    });
    let mut input = snapshot();
    input.spells.push(spell_row(100, true, false));
    let before = input.clone();

    let plan = deterministic(project_spell_acquisition_like_cpp(
        &input,
        metadata.metadata(),
        SpellAcquisitionRootLikeCpp::DirectLearn(200),
    ));

    assert_eq!(input, before);
    let upgraded = plan
        .resulting_snapshot
        .spells
        .iter()
        .find(|spell| spell.spell_id == 100)
        .expect("dependent spell retained");
    assert!(upgraded.dependent);
    assert_eq!(upgraded.state, PlayerSpellPersistenceStateLikeCpp::Changed);
}

#[test]
fn dependency_cycle_converges_after_provisional_spell_insertion() {
    let metadata = MetadataFixture::new(FixtureInput {
        spell_ids: vec![100, 200],
        dependencies: vec![dependency(1, 100, 200), dependency(2, 200, 100)],
        ..Default::default()
    });

    let plan = deterministic(project_spell_acquisition_like_cpp(
        &snapshot(),
        metadata.metadata(),
        SpellAcquisitionRootLikeCpp::DirectLearn(100),
    ));

    let spell_ids = plan
        .resulting_snapshot
        .spells
        .iter()
        .map(|spell| spell.spell_id)
        .collect::<BTreeSet<_>>();
    assert_eq!(spell_ids, BTreeSet::from([100, 200]));
    assert!(
        plan.resulting_snapshot
            .spells
            .iter()
            .all(|spell| spell.dependent),
        "the back-edge upgrades the root to dependent exactly once"
    );
}

#[test]
fn malformed_rank_cycle_fails_closed_without_exposing_partial_state() {
    const FIRST: u32 = 100;
    const SECOND: u32 = 101;
    let mut metadata = MetadataFixture::new(FixtureInput {
        spell_ids: vec![FIRST, SECOND],
        ..Default::default()
    });
    metadata.spell_chains = SpellChainStoreLikeCpp::from_skill_line_ability_supercedes_like_cpp(
        [
            wow_data::SpellRankEdgeLikeCpp {
                spell_id: SECOND,
                supercedes_spell_id: FIRST,
            },
            wow_data::SpellRankEdgeLikeCpp {
                spell_id: FIRST,
                supercedes_spell_id: SECOND,
            },
        ],
        |_| true,
    );
    let input = snapshot();
    let before = input.clone();

    let outcome = project_spell_acquisition_like_cpp(
        &input,
        metadata.metadata(),
        SpellAcquisitionRootLikeCpp::DirectLearn(FIRST),
    );

    assert_eq!(input, before);
    assert_eq!(
        outcome,
        SpellAcquisitionOutcomeLikeCpp::Indeterminate(
            SpellAcquisitionIndeterminateLikeCpp::RankChain {
                spell_id: FIRST,
                diagnostics: vec![SpellChainLoadDiagnosticLikeCpp::Cycle {
                    spell_ids: vec![FIRST, SECOND],
                }],
            }
        )
    );
}

#[test]
fn removed_and_disabled_spell_rows_follow_distinct_cpp_reactivation_paths() {
    const SPELL: u32 = 100;
    let metadata = MetadataFixture::new(FixtureInput {
        spell_ids: vec![SPELL],
        ..Default::default()
    });

    let mut removed_input = snapshot();
    let mut removed = spell_row(SPELL, false, false);
    removed.state = PlayerSpellPersistenceStateLikeCpp::Removed;
    removed_input.spells.push(removed);
    let removed_plan = deterministic(project_spell_acquisition_like_cpp(
        &removed_input,
        metadata.metadata(),
        SpellAcquisitionRootLikeCpp::DirectLearn(SPELL),
    ));
    let restored = removed_plan
        .resulting_snapshot
        .spells
        .iter()
        .find(|spell| spell.spell_id == SPELL)
        .expect("removed row must be restored");
    assert!(restored.active);
    assert!(!restored.disabled);
    assert_eq!(restored.state, PlayerSpellPersistenceStateLikeCpp::Changed);

    let mut disabled_input = snapshot();
    let mut disabled = spell_row(SPELL, true, false);
    disabled.disabled = true;
    disabled.favorite = true;
    disabled_input.spells.push(disabled);
    let disabled_plan = deterministic(project_spell_acquisition_like_cpp(
        &disabled_input,
        metadata.metadata(),
        SpellAcquisitionRootLikeCpp::DirectLearn(SPELL),
    ));
    let restored = disabled_plan
        .resulting_snapshot
        .spells
        .iter()
        .find(|spell| spell.spell_id == SPELL)
        .expect("disabled row must be retained");
    assert!(restored.active);
    assert!(!restored.disabled);
    assert!(restored.favorite);
    assert_eq!(restored.state, PlayerSpellPersistenceStateLikeCpp::Changed);
}

#[test]
fn reactivating_disabled_lower_rank_walks_higher_and_required_rows_without_false_learned_packet() {
    const LOWER: u32 = 100;
    const HIGHER: u32 = 101;
    const REQUIRED_DEPENDENT: u32 = 200;
    let metadata = MetadataFixture::new(FixtureInput {
        spell_ids: vec![LOWER, HIGHER, REQUIRED_DEPENDENT],
        chains: vec![
            (LOWER, rank_node(None, Some(HIGHER), LOWER, HIGHER, 1)),
            (HIGHER, rank_node(Some(LOWER), None, LOWER, HIGHER, 2)),
        ],
        required_edges: vec![(REQUIRED_DEPENDENT, LOWER)],
        ..Default::default()
    });
    let mut input = snapshot();
    let mut lower = spell_row(LOWER, false, false);
    lower.disabled = true;
    let mut higher = spell_row(HIGHER, true, false);
    higher.disabled = true;
    let mut required = spell_row(REQUIRED_DEPENDENT, true, false);
    required.disabled = true;
    input.spells = vec![lower, higher, required];

    let plan = deterministic(project_spell_acquisition_like_cpp(
        &input,
        metadata.metadata(),
        SpellAcquisitionRootLikeCpp::DirectLearn(LOWER),
    ));

    assert!(
        plan.resulting_snapshot
            .spells
            .iter()
            .all(|spell| !spell.disabled)
    );
    assert!(!plan.post_commit_actions.iter().any(|action| {
        matches!(
            action,
            SpellAcquisitionPostCommitActionLikeCpp::LearnedSpell {
                spell_id: LOWER,
                ..
            }
        )
    }));
    assert!(plan.spell_transitions.iter().any(|transition| {
        transition.spell_id == HIGHER
            && matches!(
                transition.provenance,
                SpellAcquisitionProvenanceLikeCpp::HigherDisabledRank {
                    source_spell_id: LOWER,
                }
            )
    }));
    assert!(plan.spell_transitions.iter().any(|transition| {
        transition.spell_id == REQUIRED_DEPENDENT
            && matches!(
                transition.provenance,
                SpellAcquisitionProvenanceLikeCpp::RequiredDisabledSpell {
                    required_spell_id: LOWER,
                }
            )
    }));
}

#[test]
fn auto_learned_dependency_is_not_applied_twice_by_add_spell() {
    const SOURCE: u32 = 100;
    const AUTO_DEPENDENCY: u32 = 200;
    let metadata = MetadataFixture::new(FixtureInput {
        spell_ids: vec![SOURCE, AUTO_DEPENDENCY],
        dependencies: vec![dependency(1, SOURCE, AUTO_DEPENDENCY)],
        dependency_nodes: vec![(
            SOURCE,
            SpellLearnSpellNodeLikeCpp {
                spell: AUTO_DEPENDENCY,
                overrides_spell: 0,
                active: true,
                auto_learned: true,
            },
        )],
        ..Default::default()
    });

    let plan = deterministic(project_spell_acquisition_like_cpp(
        &snapshot(),
        metadata.metadata(),
        SpellAcquisitionRootLikeCpp::DirectLearn(SOURCE),
    ));

    assert_eq!(
        plan.resulting_snapshot
            .spells
            .iter()
            .map(|spell| spell.spell_id)
            .collect::<Vec<_>>(),
        vec![SOURCE]
    );
    assert!(!plan.spell_transitions.iter().any(|transition| {
        matches!(
            transition.provenance,
            SpellAcquisitionProvenanceLikeCpp::LearnDependency {
                source_spell_id: SOURCE,
            }
        )
    }));
}

#[test]
fn identical_snapshot_metadata_and_root_produce_byte_for_byte_equal_outcomes() {
    const ROOT: u32 = 100;
    const DEPENDENCY: u32 = 200;
    let metadata = MetadataFixture::new(FixtureInput {
        spell_ids: vec![ROOT, DEPENDENCY],
        dependencies: vec![dependency(1, ROOT, DEPENDENCY)],
        ..Default::default()
    });
    let input = snapshot();

    let first = project_spell_acquisition_like_cpp(
        &input,
        metadata.metadata(),
        SpellAcquisitionRootLikeCpp::DirectLearn(ROOT),
    );
    let second = project_spell_acquisition_like_cpp(
        &input,
        metadata.metadata(),
        SpellAcquisitionRootLikeCpp::DirectLearn(ROOT),
    );
    assert_eq!(first, second);
}

#[test]
fn learning_lower_rank_under_known_higher_rank_does_not_emit_learned_spell() {
    const LOWER: u32 = 100;
    const HIGHER: u32 = 101;
    let metadata = MetadataFixture::new(FixtureInput {
        spell_ids: vec![LOWER, HIGHER],
        chains: vec![
            (LOWER, rank_node(None, Some(HIGHER), LOWER, HIGHER, 1)),
            (HIGHER, rank_node(Some(LOWER), None, LOWER, HIGHER, 2)),
        ],
        ..Default::default()
    });
    let mut input = snapshot();
    input.spells.push(spell_row(HIGHER, true, false));

    let plan = deterministic(project_spell_acquisition_like_cpp(
        &input,
        metadata.metadata(),
        SpellAcquisitionRootLikeCpp::DirectLearn(LOWER),
    ));

    let lower = plan
        .resulting_snapshot
        .spells
        .iter()
        .find(|spell| spell.spell_id == LOWER)
        .expect("lower rank is persisted");
    assert!(!lower.active);
    assert!(!plan.post_commit_actions.iter().any(|action| {
        matches!(
            action,
            SpellAcquisitionPostCommitActionLikeCpp::LearnedSpell {
                spell_id: LOWER,
                ..
            }
        )
    }));
}

#[test]
fn reactivating_active_disabled_higher_rank_ignores_the_disabled_lower_rank() {
    const LOWER: u32 = 100;
    const HIGHER: u32 = 101;
    let metadata = MetadataFixture::new(FixtureInput {
        spell_ids: vec![LOWER, HIGHER],
        chains: vec![
            (LOWER, rank_node(None, Some(HIGHER), LOWER, HIGHER, 1)),
            (HIGHER, rank_node(Some(LOWER), None, LOWER, HIGHER, 2)),
        ],
        ..Default::default()
    });
    let mut input = snapshot();
    let mut lower = spell_row(LOWER, true, false);
    lower.disabled = true;
    let mut higher = spell_row(HIGHER, true, false);
    higher.disabled = true;
    input.spells = vec![lower, higher];
    let before = input.clone();

    let plan = deterministic(project_spell_acquisition_like_cpp(
        &input,
        metadata.metadata(),
        SpellAcquisitionRootLikeCpp::DirectLearn(HIGHER),
    ));

    assert_eq!(input, before);
    let by_id = plan
        .resulting_snapshot
        .spells
        .iter()
        .map(|spell| (spell.spell_id, *spell))
        .collect::<BTreeMap<_, _>>();
    assert!(by_id[&LOWER].active);
    assert!(by_id[&LOWER].disabled);
    assert!(by_id[&HIGHER].active);
    assert!(!by_id[&HIGHER].disabled);
}

#[test]
fn reactivating_active_disabled_lower_rank_preserves_legacy_multi_active_result() {
    const LOWER: u32 = 100;
    const HIGHER: u32 = 101;
    let metadata = MetadataFixture::new(FixtureInput {
        spell_ids: vec![LOWER, HIGHER],
        chains: vec![
            (LOWER, rank_node(None, Some(HIGHER), LOWER, HIGHER, 1)),
            (HIGHER, rank_node(Some(LOWER), None, LOWER, HIGHER, 2)),
        ],
        ..Default::default()
    });
    let mut input = snapshot();
    let mut lower = spell_row(LOWER, true, false);
    lower.disabled = true;
    let mut higher = spell_row(HIGHER, true, false);
    higher.disabled = true;
    input.spells = vec![lower, higher];
    let before = input.clone();

    let plan = deterministic(project_spell_acquisition_like_cpp(
        &input,
        metadata.metadata(),
        SpellAcquisitionRootLikeCpp::DirectLearn(LOWER),
    ));

    assert_eq!(input, before);
    let enabled_active = plan
        .resulting_snapshot
        .spells
        .iter()
        .filter(|spell| spell.active && !spell.disabled)
        .map(|spell| spell.spell_id)
        .collect::<Vec<_>>();
    assert_eq!(
        enabled_active,
        vec![LOWER, HIGHER],
        "C++ re-enables the existing lower row, then recursively re-enables the disabled higher row; disabled-case reactivation skips rank replacement"
    );

    let replay = deterministic(project_spell_acquisition_like_cpp(
        &plan.resulting_snapshot,
        metadata.metadata(),
        SpellAcquisitionRootLikeCpp::DirectLearn(HIGHER),
    ));
    assert_eq!(
        replay.resulting_snapshot, plan.resulting_snapshot,
        "a C++-reachable multi-active snapshot must remain projectable"
    );
}

#[test]
fn inserting_new_highest_rank_processes_all_active_disabled_ranks_in_stable_order() {
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
    let mut input = snapshot();
    let mut lower = spell_row(LOWER, true, false);
    lower.disabled = true;
    let mut middle = spell_row(MIDDLE, true, false);
    middle.disabled = true;
    input.spells = vec![lower, middle];
    let before = input.clone();

    let plan = deterministic(project_spell_acquisition_like_cpp(
        &input,
        metadata.metadata(),
        SpellAcquisitionRootLikeCpp::DirectLearn(HIGHER),
    ));

    assert_eq!(input, before);
    let by_id = plan
        .resulting_snapshot
        .spells
        .iter()
        .map(|spell| (spell.spell_id, *spell))
        .collect::<BTreeMap<_, _>>();
    assert!(!by_id[&LOWER].active);
    assert!(by_id[&LOWER].disabled);
    assert!(!by_id[&MIDDLE].active);
    assert!(!by_id[&MIDDLE].disabled);
    assert!(by_id[&HIGHER].active);
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
        vec![(LOWER, HIGHER), (MIDDLE, HIGHER)],
        "C++ processes every active candidate; the pure planner makes that traversal stable by spell ID"
    );
}
