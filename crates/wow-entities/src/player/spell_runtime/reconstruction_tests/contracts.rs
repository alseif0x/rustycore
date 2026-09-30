//! Ordered operations and incomplete-input boundaries.

use super::support::*;
use super::*;

type Rows = std::vec::IntoIter<LoadedSpellDependency>;

fn dependency(spell_id: u32, overrides_spell_id: u32, auto_learned: bool) -> LoadedSpellDependency {
    LoadedSpellDependency {
        spell_id,
        overrides_spell_id,
        active: true,
        auto_learned,
    }
}

#[test]
fn reconstruction_keeps_duplicate_roots_flags_before_append_and_auto_overrides() {
    let mut known = vec![10];
    let mut operation = LoadedSpellReconstruction::<Rows>::new(&[-1, 10, 10, 0]);
    assert_eq!(operation.step(), LoadedSpellStep::Dependencies(10));
    operation.advance(
        LoadedSpellInput::Dependencies(
            vec![
                dependency(20, 100, false),
                dependency(30, 200, true),
                dependency(u32::MAX, 300, false),
            ]
            .into_iter(),
        ),
        &mut known,
    );
    assert_eq!(operation.step(), LoadedSpellStep::Flags(20));
    assert_eq!(
        known,
        vec![10],
        "membership and append follow the flags writer"
    );
    operation.advance(LoadedSpellInput::Applied, &mut known);
    assert_eq!(
        known,
        vec![10, 20],
        "a failed flags writer still answers Applied"
    );
    assert_eq!(
        operation.step(),
        LoadedSpellStep::Override {
            overridden: 100,
            replacement: 20
        }
    );
    operation.advance(LoadedSpellInput::Applied, &mut known);
    assert_eq!(
        operation.step(),
        LoadedSpellStep::Override {
            overridden: 200,
            replacement: 30
        }
    );
    operation.advance(LoadedSpellInput::Applied, &mut known);
    assert_eq!(
        operation.step(),
        LoadedSpellStep::Dependencies(10),
        "duplicate roots are not suppressed"
    );
    operation.advance(
        LoadedSpellInput::Dependencies(vec![dependency(20, 100, false)].into_iter()),
        &mut known,
    );
    assert_eq!(
        operation.step(),
        LoadedSpellStep::Flags(20),
        "already known targets still run flags"
    );
    operation.advance(LoadedSpellInput::Applied, &mut known);
    assert_eq!(
        operation.step(),
        LoadedSpellStep::Override {
            overridden: 100,
            replacement: 20
        }
    );
    operation.advance(LoadedSpellInput::Applied, &mut known);
    assert_eq!(
        operation.step(),
        LoadedSpellStep::Dependencies(0),
        "zero is not a new rejection gate"
    );
    operation.advance(
        LoadedSpellInput::Dependencies(Vec::new().into_iter()),
        &mut known,
    );
    assert_eq!(
        operation.step(),
        LoadedSpellStep::Dependencies(20),
        "append enqueues behind original roots"
    );
    operation.advance(
        LoadedSpellInput::Dependencies(Vec::new().into_iter()),
        &mut known,
    );
    assert_eq!(operation.step(), LoadedSpellStep::Done(1));
}

#[test]
fn lower_rank_projection_resolves_only_visited_edges_and_keeps_duplicates() {
    let mut known = vec![-1, 10, 20, 20, 50];
    let mut calls = Vec::new();
    let removed = PlayerSpellRuntimeState::deactivate_lower_loaded_ranks(&mut known, |id| {
        calls.push(id);
        match id {
            10 => 20,
            50 => 60,
            _ => 0,
        }
    });
    assert_eq!(removed, 1);
    assert_eq!(known, vec![-1, 20, 20, 50]);
    assert_eq!(calls, vec![10, 20, 20, 50, 60]);
}

#[test]
fn gain_keeps_prior_canonical_writes_before_missing_coverage() {
    let (mut session, _, _) = make_session();
    session.set_spell_learn_skill_store(Arc::new(wow_data::SpellLearnSkillStoreLikeCpp {
        skill_by_spell_id: BTreeMap::from([(
            10,
            wow_data::SpellLearnSkillNodeLikeCpp {
                skill: 755,
                step: 2,
                value: 75,
                maxvalue: 150,
            },
        )]),
        covered_spell_ids: BTreeSet::from([10, 11]),
        ..Default::default()
    }));
    assert!(!session.run_skills(LearnedSkillOperation::gain(&[11, 10, 12, 13])));
    assert_eq!(
        session.trace,
        vec![
            LearnedSkillStep::GainNode(11),
            LearnedSkillStep::GainNode(10),
            LearnedSkillStep::Value(755),
            LearnedSkillStep::Maximum(755),
            LearnedSkillStep::Write(LearnedSkillWrite {
                skill_id: 755,
                step: 2,
                value: 75,
                max_value: 150
            }),
            LearnedSkillStep::GainNode(12),
            LearnedSkillStep::Done(false),
        ]
    );
    assert_eq!(session.player_skill_value_like_cpp(755), 75);
    assert_eq!(session.player_skill_max_value_like_cpp(755), 150);
    assert_eq!(
        session.player.skill_records_like_cpp()[0].state,
        PlayerSkillLoadState::New
    );
}

#[test]
fn gain_short_circuits_owner_and_maximum_before_range() {
    let roots = [10];
    let node = LearnedSkillNode {
        skill_id: 755,
        step: 1,
        value: 75,
        max_value: 0,
    };
    let mut operation = LearnedSkillOperation::gain(&roots);
    assert_eq!(operation.step(), LearnedSkillStep::GainNode(10));
    operation.advance(LearnedSkillInput::GainNode(LearnedSkillLookup::Present(
        node,
    )));
    assert_eq!(operation.step(), LearnedSkillStep::Value(755));
    operation.advance(LearnedSkillInput::Value(None));
    assert_eq!(operation.step(), LearnedSkillStep::Done(false));

    let mut operation = LearnedSkillOperation::gain(&roots);
    operation.step();
    operation.advance(LearnedSkillInput::GainNode(LearnedSkillLookup::Present(
        node,
    )));
    operation.advance(LearnedSkillInput::Value(Some(80)));
    assert_eq!(operation.step(), LearnedSkillStep::Maximum(755));
    operation.advance(LearnedSkillInput::Maximum(None));
    assert_eq!(operation.step(), LearnedSkillStep::Done(false));
}

#[test]
fn downgrade_previous_search_keeps_zero_lookup_after_first_rank() {
    let node = LearnedSkillNode {
        skill_id: 755,
        step: 3,
        value: 100,
        max_value: 150,
    };
    let mut operation = LearnedSkillOperation::downgrade(node, 30);
    assert_eq!(operation.step(), LearnedSkillStep::PreviousRank(30));
    operation.advance(LearnedSkillInput::Rank(20));
    assert_eq!(operation.step(), LearnedSkillStep::PreviousNode(20));
    operation.advance(LearnedSkillInput::PreviousNode(None));
    assert_eq!(operation.step(), LearnedSkillStep::PreviousRank(20));
    operation.advance(LearnedSkillInput::Rank(0));
    assert_eq!(operation.step(), LearnedSkillStep::FirstRank(0));
    operation.advance(LearnedSkillInput::Rank(0));
    assert_eq!(operation.step(), LearnedSkillStep::PreviousNode(0));
    operation.advance(LearnedSkillInput::PreviousNode(None));
    assert_eq!(
        operation.step(),
        LearnedSkillStep::Write(LearnedSkillWrite {
            skill_id: 755,
            step: 0,
            value: 0,
            max_value: 0,
        })
    );
    operation.advance(LearnedSkillInput::Applied);
    assert_eq!(operation.step(), LearnedSkillStep::Done(true));
}

#[test]
fn gain_tier_step_saturates_index_and_cast_and_ignores_setter_failure() {
    let roots = [10, -1];
    let mut operation = LearnedSkillOperation::gain(&roots);
    operation.step();
    operation.advance(LearnedSkillInput::GainNode(LearnedSkillLookup::Present(
        LearnedSkillNode {
            skill_id: 755,
            step: 0,
            value: 1,
            max_value: 0,
        },
    )));
    operation.advance(LearnedSkillInput::Value(Some(30)));
    operation.advance(LearnedSkillInput::Maximum(Some(75)));
    assert_eq!(operation.step(), LearnedSkillStep::Range(755));
    operation.advance(LearnedSkillInput::Range(LearnedSkillRange::Rank {
        always_max: true,
        tier_id: 9,
    }));
    assert_eq!(
        operation.step(),
        LearnedSkillStep::TierMaximum {
            tier_id: 9,
            index: 0
        }
    );
    operation.advance(LearnedSkillInput::TierMaximum(Some(u32::MAX)));
    assert_eq!(
        operation.step(),
        LearnedSkillStep::Write(LearnedSkillWrite {
            skill_id: 755,
            step: 0,
            value: u16::MAX,
            max_value: u16::MAX,
        })
    );
    operation.advance(LearnedSkillInput::Applied);
    assert_eq!(
        operation.step(),
        LearnedSkillStep::Done(false),
        "only the later signed conversion fails"
    );
}

#[test]
fn missing_range_and_tier_fail_gain_but_downgrade_retains_zero_write() {
    for range in [
        LearnedSkillRange::Unavailable,
        LearnedSkillRange::None { always_max: false },
        LearnedSkillRange::Rank {
            always_max: false,
            tier_id: -1,
        },
    ] {
        for gain in [true, false] {
            let node = LearnedSkillNode {
                skill_id: 755,
                step: 1,
                value: 1,
                max_value: 0,
            };
            let mut operation = if gain {
                LearnedSkillOperation::gain(&[10])
            } else {
                LearnedSkillOperation::downgrade(node, 20)
            };
            operation.step();
            if gain {
                operation.advance(LearnedSkillInput::GainNode(LearnedSkillLookup::Present(
                    node,
                )));
            } else {
                operation.advance(LearnedSkillInput::Rank(10));
                operation.advance(LearnedSkillInput::PreviousNode(Some(node)));
            }
            operation.advance(LearnedSkillInput::Value(Some(75)));
            operation.advance(LearnedSkillInput::Maximum(Some(150)));
            operation.advance(LearnedSkillInput::Range(range));
            if matches!(range, LearnedSkillRange::Rank { .. }) {
                assert_eq!(
                    operation.step(),
                    LearnedSkillStep::TierMaximum {
                        tier_id: -1,
                        index: 0
                    }
                );
                operation.advance(LearnedSkillInput::TierMaximum(None));
            }
            assert_eq!(
                operation.step(),
                if gain {
                    LearnedSkillStep::Done(false)
                } else {
                    LearnedSkillStep::Write(LearnedSkillWrite {
                        skill_id: 755,
                        step: 1,
                        value: 0,
                        max_value: 0,
                    })
                }
            );
        }
    }
}
