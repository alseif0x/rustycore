//! Validate post commit items of application.
//!
//! Separated from application.rs under #711; every item keeps its name,
//! signature and body.

use super::*;

pub(super) fn validate_post_commit_actions_like_cpp(
    plan: &SpellAcquisitionPlanLikeCpp,
) -> Result<(), PlayerSpellAcquisitionPrepareErrorLikeCpp> {
    let actual_publication_requirements = plan
        .post_commit_actions
        .iter()
        .filter_map(|action| match action {
            SpellAcquisitionPostCommitActionLikeCpp::LearnedSpell {
                spell_id,
                favorite,
                suppress_messaging,
            } => Some(SpellAcquisitionPublicationRequirementLikeCpp::LearnedSpell {
                spell_id: *spell_id,
                favorite: *favorite,
                suppress_messaging: *suppress_messaging,
            }),
            SpellAcquisitionPostCommitActionLikeCpp::UpdateLearnSpellQuestObjective {
                spell_id,
            } => Some(
                SpellAcquisitionPublicationRequirementLikeCpp::UpdateLearnSpellQuestObjective {
                    spell_id: *spell_id,
                },
            ),
            SpellAcquisitionPostCommitActionLikeCpp::UpdateLearnTradeskillSkillLineCriteria {
                source_spell_id,
                skill_id,
            } => Some(
                SpellAcquisitionPublicationRequirementLikeCpp::UpdateLearnTradeskillSkillLineCriteria {
                    source_spell_id: *source_spell_id,
                    skill_id: *skill_id,
                },
            ),
            SpellAcquisitionPostCommitActionLikeCpp::UpdateLearnSpellFromSkillLineCriteria {
                source_spell_id,
                skill_id,
            } => Some(
                SpellAcquisitionPublicationRequirementLikeCpp::UpdateLearnSpellFromSkillLineCriteria {
                    source_spell_id: *source_spell_id,
                    skill_id: *skill_id,
                },
            ),
            SpellAcquisitionPostCommitActionLikeCpp::UpdateLearnOrKnowSpellCriteria {
                spell_id,
            } => Some(
                SpellAcquisitionPublicationRequirementLikeCpp::UpdateLearnOrKnowSpellCriteria {
                    spell_id: *spell_id,
                },
            ),
            _ => None,
        })
        .collect::<Vec<_>>();
    if actual_publication_requirements != plan.publication_requirements {
        return Err(
            PlayerSpellAcquisitionPrepareErrorLikeCpp::PostCommitActionCausalityMismatch {
                action: "required publication tape",
                id: 0,
            },
        );
    }
    let resulting_spells = spell_map_like_cpp(&plan.resulting_snapshot)?;
    let resulting_skills = skill_map_like_cpp(&plan.resulting_snapshot)?;
    let mut learned_evidence = BTreeMap::<(u32, bool), usize>::new();
    let mut refresh_evidence = BTreeMap::<u32, usize>::new();
    let mut spell_transition_evidence = BTreeMap::<u32, usize>::new();
    let mut skill_transition_evidence = BTreeMap::<u32, usize>::new();
    let mut deactivation_evidence = Vec::<(usize, u32)>::new();
    let mut supersede_evidence = BTreeMap::<(u32, u32), Vec<usize>>::new();
    let mut replayed_spells = spell_map_like_cpp(&plan.source_snapshot)?;
    let mut prior_activations = Vec::<(usize, u32, SpellAcquisitionProvenanceLikeCpp)>::new();
    let mut superseding_activations = BTreeSet::<usize>::new();
    for (transition_index, transition) in plan.spell_transitions.iter().enumerate() {
        *spell_transition_evidence
            .entry(transition.spell_id)
            .or_default() += 1;
        let before_active = transition.before.is_some_and(|before| {
            before.state != PlayerSpellPersistenceStateLikeCpp::Removed
                && before.active
                && !before.disabled
        });
        let after_active = transition.after.is_some_and(|after| {
            after.state != PlayerSpellPersistenceStateLikeCpp::Removed
                && after.active
                && !after.disabled
        });
        if let Some(after) = transition.after {
            replayed_spells.insert(transition.spell_id, after);
        } else {
            replayed_spells.remove(&transition.spell_id);
        }
        if !before_active && after_active {
            let favorite = transition.after.is_some_and(|after| after.favorite);
            *learned_evidence
                .entry((transition.spell_id, favorite))
                .or_default() += 1;
            *refresh_evidence.entry(transition.spell_id).or_default() += 1;
            prior_activations.push((
                transition_index,
                transition.spell_id,
                transition.provenance.clone(),
            ));
        }
        if before_active && !after_active {
            deactivation_evidence.push((transition_index, transition.spell_id));
            for (candidate_index, candidate_spell_id, candidate_provenance) in &prior_activations {
                if *candidate_spell_id != transition.spell_id
                    && *candidate_provenance == transition.provenance
                    && replayed_spells.get(candidate_spell_id).is_some_and(|row| {
                        row.state != PlayerSpellPersistenceStateLikeCpp::Removed
                            && row.active
                            && !row.disabled
                    })
                {
                    superseding_activations.insert(*candidate_index);
                    supersede_evidence
                        .entry((transition.spell_id, *candidate_spell_id))
                        .or_default()
                        .push(transition_index);
                }
            }
        }
    }
    // C++ AddSpell activates the new rank before it discovers and publishes
    // the supersede relationship. That activation is causal evidence for the
    // SupersededSpell packet, not for an additional LearnedSpell packet:
    // AddSpell returns false to LearnSpell when it replaced an active rank.
    for transition_index in superseding_activations {
        let transition = &plan.spell_transitions[transition_index];
        let favorite = transition.after.is_some_and(|after| after.favorite);
        let _ = consume_action_evidence_like_cpp(
            &mut learned_evidence,
            (transition.spell_id, favorite),
        );
    }
    for transition in &plan.skill_transitions {
        *skill_transition_evidence
            .entry(transition.skill_id)
            .or_default() += 1;
    }
    let mut mount_skill_evidence = skill_transition_evidence.clone();
    let mut raised_skill_evidence = skill_transition_evidence.clone();
    let mut achieve_skill_evidence = skill_transition_evidence.clone();
    let mut consumed_deactivations = BTreeSet::<usize>::new();
    let resulting_spell_is_present = |spell_id| {
        resulting_spells
            .get(&spell_id)
            .is_some_and(|spell| spell.state != PlayerSpellPersistenceStateLikeCpp::Removed)
    };
    let resulting_spell_is_inactive = |spell_id| {
        !resulting_spells.get(&spell_id).is_some_and(|spell| {
            spell.state != PlayerSpellPersistenceStateLikeCpp::Removed
                && spell.active
                && !spell.disabled
        })
    };
    let mut dual_wield_effect_evidence = BTreeMap::<(u32, u32, u8), usize>::new();
    for (&spell_id, resolution) in &plan.source_snapshot.cast_resolutions {
        if !resolution.reached_immediate_phase {
            continue;
        }
        for effect in &resolution.executed_dual_wield_effects {
            *dual_wield_effect_evidence
                .entry((spell_id, effect.effect_record_id, effect.effect_index))
                .or_default() += 1;
        }
    }

    for (action_index, action) in plan.post_commit_actions.iter().enumerate() {
        let mismatch = |action, id| {
            Err(
                PlayerSpellAcquisitionPrepareErrorLikeCpp::PostCommitActionCausalityMismatch {
                    action,
                    id,
                },
            )
        };
        match action {
            SpellAcquisitionPostCommitActionLikeCpp::LearnedSpell {
                spell_id, favorite, ..
            } => {
                let row_matches = resulting_spells
                    .get(spell_id)
                    .is_some_and(|row| row.favorite == *favorite);
                if !row_matches
                    || !consume_action_evidence_like_cpp(
                        &mut learned_evidence,
                        (*spell_id, *favorite),
                    )
                {
                    return Err(
                        PlayerSpellAcquisitionPrepareErrorLikeCpp::LearnedActionRowMismatch(
                            *spell_id,
                        ),
                    );
                }
            }
            SpellAcquisitionPostCommitActionLikeCpp::SupersededSpell {
                old_spell_id,
                new_spell_id,
            } => {
                for spell_id in [*old_spell_id, *new_spell_id] {
                    if spell_id == 0 || i32::try_from(spell_id).is_err() {
                        return Err(
                            PlayerSpellAcquisitionPrepareErrorLikeCpp::InvalidPostCommitAction {
                                domain: "spell",
                                id: spell_id,
                            },
                        );
                    }
                }
                let evidence_index = supersede_evidence
                    .get_mut(&(*old_spell_id, *new_spell_id))
                    .and_then(|indices| {
                        indices
                            .iter()
                            .copied()
                            .find(|index| !consumed_deactivations.contains(index))
                    });
                if old_spell_id == new_spell_id || evidence_index.is_none() {
                    return mismatch("SupersededSpell", *old_spell_id);
                }
                consumed_deactivations.insert(evidence_index.expect("checked above"));
            }
            SpellAcquisitionPostCommitActionLikeCpp::UnlearnedSpell { spell_id } => {
                validate_post_commit_id_like_cpp("spell", *spell_id, false)?;
                let evidence_index =
                    deactivation_evidence
                        .iter()
                        .find_map(|(index, evidence_spell_id)| {
                            (*evidence_spell_id == *spell_id
                                && !consumed_deactivations.contains(index))
                            .then_some(*index)
                        });
                if evidence_index.is_none() || !resulting_spell_is_inactive(*spell_id) {
                    return mismatch("UnlearnedSpell", *spell_id);
                }
                consumed_deactivations.insert(evidence_index.expect("checked above"));
            }
            SpellAcquisitionPostCommitActionLikeCpp::RefreshPassive { spell_id } => {
                validate_post_commit_id_like_cpp("spell", *spell_id, false)?;
                if !consume_action_evidence_like_cpp(&mut refresh_evidence, *spell_id)
                    || !resulting_spell_is_present(*spell_id)
                {
                    return mismatch("RefreshPassive", *spell_id);
                }
            }
            SpellAcquisitionPostCommitActionLikeCpp::UpdateLearnSpellQuestObjective {
                spell_id,
            } => {
                validate_post_commit_id_like_cpp("spell", *spell_id, false)?;
                if !resulting_spell_is_present(*spell_id) {
                    return mismatch("UpdateLearnSpellQuestObjective", *spell_id);
                }
            }
            SpellAcquisitionPostCommitActionLikeCpp::UpdateLearnOrKnowSpellCriteria {
                spell_id,
            } => {
                validate_post_commit_id_like_cpp("spell", *spell_id, false)?;
                if !resulting_spell_is_present(*spell_id) {
                    return mismatch("UpdateLearnOrKnowSpellCriteria", *spell_id);
                }
            }
            SpellAcquisitionPostCommitActionLikeCpp::GrantDualWield {
                source_spell_id,
                effect_record_id,
                effect_index,
            } => {
                validate_post_commit_id_like_cpp("spell", *source_spell_id, false)?;
                let evidence_count = dual_wield_effect_evidence.get_mut(&(
                    *source_spell_id,
                    *effect_record_id,
                    *effect_index,
                ));
                if *effect_record_id == 0
                    || !evidence_count.is_some_and(|count| {
                        if *count == 0 {
                            false
                        } else {
                            *count -= 1;
                            true
                        }
                    })
                {
                    return mismatch("GrantDualWield", *source_spell_id);
                }
            }
            SpellAcquisitionPostCommitActionLikeCpp::UpdateLearnTradeskillSkillLineCriteria {
                source_spell_id,
                skill_id,
            } => {
                validate_post_commit_id_like_cpp("spell", *source_spell_id, false)?;
                validate_post_commit_id_like_cpp("skill", *skill_id, true)?;
                let paired = plan.post_commit_actions.get(action_index + 1).is_some_and(
                    |next| {
                        matches!(
                            next,
                            SpellAcquisitionPostCommitActionLikeCpp::UpdateLearnSpellFromSkillLineCriteria {
                                source_spell_id: next_source_spell_id,
                                skill_id: next_skill_id,
                            } if next_source_spell_id == source_spell_id && next_skill_id == skill_id
                        )
                    },
                );
                if !paired
                    || !spell_transition_evidence.contains_key(source_spell_id)
                    || !resulting_spell_is_present(*source_spell_id)
                {
                    return mismatch("tradeskill skill-line criteria", *source_spell_id);
                }
            }
            SpellAcquisitionPostCommitActionLikeCpp::UpdateLearnSpellFromSkillLineCriteria {
                source_spell_id,
                skill_id,
            } => {
                validate_post_commit_id_like_cpp("spell", *source_spell_id, false)?;
                validate_post_commit_id_like_cpp("skill", *skill_id, true)?;
                let paired = action_index.checked_sub(1).is_some_and(|previous_index| {
                    matches!(
                        &plan.post_commit_actions[previous_index],
                        SpellAcquisitionPostCommitActionLikeCpp::UpdateLearnTradeskillSkillLineCriteria {
                            source_spell_id: previous_source_spell_id,
                            skill_id: previous_skill_id,
                        } if previous_source_spell_id == source_spell_id && previous_skill_id == skill_id
                    )
                });
                let continues_same_spell_block = plan
                    .post_commit_actions
                    .get(action_index + 1)
                    .is_some_and(|next| {
                        matches!(
                            next,
                            SpellAcquisitionPostCommitActionLikeCpp::UpdateLearnTradeskillSkillLineCriteria {
                                source_spell_id: next_source_spell_id,
                                ..
                            } | SpellAcquisitionPostCommitActionLikeCpp::UpdateLearnOrKnowSpellCriteria {
                                spell_id: next_source_spell_id,
                            } if next_source_spell_id == source_spell_id
                        )
                    });
                if !paired
                    || !continues_same_spell_block
                    || !spell_transition_evidence.contains_key(source_spell_id)
                    || !resulting_spell_is_present(*source_spell_id)
                {
                    return mismatch("learn-spell skill-line criteria", *source_spell_id);
                }
            }
            SpellAcquisitionPostCommitActionLikeCpp::UpdateMountCapability { skill_id } => {
                validate_post_commit_id_like_cpp("skill", *skill_id, true)?;
                if *skill_id != u32::from(SKILL_RIDING_LIKE_CPP)
                    || !consume_action_evidence_like_cpp(&mut mount_skill_evidence, *skill_id)
                    || !resulting_skills.get(skill_id).is_some_and(|skill| {
                        skill.state != PlayerSkillPersistenceStateLikeCpp::Deleted
                    })
                {
                    return mismatch("UpdateMountCapability", *skill_id);
                }
            }
            SpellAcquisitionPostCommitActionLikeCpp::UpdateSkillRaisedCriteria { skill_id } => {
                validate_post_commit_id_like_cpp("skill", *skill_id, true)?;
                if !consume_action_evidence_like_cpp(&mut raised_skill_evidence, *skill_id)
                    || !resulting_skills.get(skill_id).is_some_and(|skill| {
                        skill.state != PlayerSkillPersistenceStateLikeCpp::Deleted
                    })
                {
                    return mismatch("skill action", *skill_id);
                }
            }
            SpellAcquisitionPostCommitActionLikeCpp::UpdateAchieveSkillStepCriteria {
                skill_id,
            } => {
                validate_post_commit_id_like_cpp("skill", *skill_id, true)?;
                if !consume_action_evidence_like_cpp(&mut achieve_skill_evidence, *skill_id)
                    || !resulting_skills.get(skill_id).is_some_and(|skill| {
                        skill.state != PlayerSkillPersistenceStateLikeCpp::Deleted
                    })
                {
                    return mismatch("skill action", *skill_id);
                }
            }
        }
    }
    if let Some((&(spell_id, _, _), _)) = dual_wield_effect_evidence
        .iter()
        .find(|(_, count)| **count != 0)
    {
        return Err(
            PlayerSpellAcquisitionPrepareErrorLikeCpp::PostCommitActionCausalityMismatch {
                action: "DualWieldEffectProjected",
                id: spell_id,
            },
        );
    }
    Ok(())
}

pub(super) fn consume_action_evidence_like_cpp<K: Ord>(
    evidence: &mut BTreeMap<K, usize>,
    key: K,
) -> bool {
    evidence.get_mut(&key).is_some_and(|count| {
        if *count == 0 {
            false
        } else {
            *count -= 1;
            true
        }
    })
}

pub(super) fn validate_post_commit_id_like_cpp(
    domain: &'static str,
    id: u32,
    u16_required: bool,
) -> Result<(), PlayerSpellAcquisitionPrepareErrorLikeCpp> {
    if id == 0 || i32::try_from(id).is_err() || (u16_required && u16::try_from(id).is_err()) {
        return Err(
            PlayerSpellAcquisitionPrepareErrorLikeCpp::InvalidPostCommitAction { domain, id },
        );
    }
    Ok(())
}
