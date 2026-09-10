//! Validate plan items of application.
//!
//! Separated from application.rs under #711; every item keeps its name,
//! signature and body.

use super::*;

pub(super) fn validate_plan_replay_like_cpp(
    plan: &SpellAcquisitionPlanLikeCpp,
) -> Result<(), PlayerSpellAcquisitionPrepareErrorLikeCpp> {
    validate_root_like_cpp(plan.root)?;
    validate_snapshot_like_cpp(&plan.source_snapshot)?;
    validate_snapshot_like_cpp(&plan.resulting_snapshot)?;
    validate_snapshot_identity_like_cpp(&plan.source_snapshot, &plan.resulting_snapshot)?;

    let mut spells = spell_map_like_cpp(&plan.source_snapshot)?;
    let mut skills = skill_map_like_cpp(&plan.source_snapshot)?;
    let mut overrides = override_set_like_cpp(&plan.source_snapshot)?;
    let mut replayed_spells = Vec::new();
    let mut replayed_skills = Vec::new();
    let mut replayed_overrides = Vec::new();

    for mutation in &plan.mutations {
        match mutation {
            PlannedAcquisitionMutationLikeCpp::Spell(transition) => {
                validate_provenance_like_cpp(&transition.provenance, plan.root)?;
                if transition
                    .before
                    .is_some_and(|row| row.spell_id != transition.spell_id)
                    || transition
                        .after
                        .is_some_and(|row| row.spell_id != transition.spell_id)
                {
                    return Err(
                        PlayerSpellAcquisitionPrepareErrorLikeCpp::TransitionIdMismatch {
                            domain: "spell",
                            id: transition.spell_id,
                        },
                    );
                }
                if spells.get(&transition.spell_id).copied() != transition.before {
                    return Err(
                        PlayerSpellAcquisitionPrepareErrorLikeCpp::TransitionBeforeMismatch {
                            domain: "spell",
                            id: transition.spell_id,
                        },
                    );
                }
                if let Some(after) = transition.after {
                    spells.insert(transition.spell_id, after);
                } else {
                    spells.remove(&transition.spell_id);
                }
                replayed_spells.push(transition.clone());
            }
            PlannedAcquisitionMutationLikeCpp::Skill(transition) => {
                validate_provenance_like_cpp(&transition.provenance, plan.root)?;
                if transition.after.skill_id != transition.skill_id
                    || transition
                        .before
                        .is_some_and(|row| row.skill_id != transition.skill_id)
                {
                    return Err(
                        PlayerSpellAcquisitionPrepareErrorLikeCpp::TransitionIdMismatch {
                            domain: "skill",
                            id: transition.skill_id,
                        },
                    );
                }
                if skills.get(&transition.skill_id).copied() != transition.before {
                    return Err(
                        PlayerSpellAcquisitionPrepareErrorLikeCpp::TransitionBeforeMismatch {
                            domain: "skill",
                            id: transition.skill_id,
                        },
                    );
                }
                skills.insert(transition.skill_id, transition.after);
                replayed_skills.push(transition.clone());
            }
            PlannedAcquisitionMutationLikeCpp::Override(transition) => {
                let pair = (
                    transition.overridden_spell_id,
                    transition.overriding_spell_id,
                );
                if transition.add {
                    if !overrides.insert(pair) {
                        return Err(
                            PlayerSpellAcquisitionPrepareErrorLikeCpp::DuplicateOverrideMutation {
                                overridden: pair.0,
                                overriding: pair.1,
                            },
                        );
                    }
                } else if !overrides.remove(&pair) {
                    return Err(
                        PlayerSpellAcquisitionPrepareErrorLikeCpp::MissingOverrideMutation {
                            overridden: pair.0,
                            overriding: pair.1,
                        },
                    );
                }
                replayed_overrides.push(*transition);
            }
        }
    }

    if replayed_spells != plan.spell_transitions {
        return Err(
            PlayerSpellAcquisitionPrepareErrorLikeCpp::TypedProjectionMismatch("spell_transitions"),
        );
    }
    if replayed_skills != plan.skill_transitions {
        return Err(
            PlayerSpellAcquisitionPrepareErrorLikeCpp::TypedProjectionMismatch("skill_transitions"),
        );
    }
    if replayed_overrides != plan.override_transitions {
        return Err(
            PlayerSpellAcquisitionPrepareErrorLikeCpp::TypedProjectionMismatch(
                "override_transitions",
            ),
        );
    }

    let resulting_spells = spell_map_like_cpp(&plan.resulting_snapshot)?;
    let resulting_skills = skill_map_like_cpp(&plan.resulting_snapshot)?;
    let resulting_overrides = override_set_like_cpp(&plan.resulting_snapshot)?;
    if spells != resulting_spells || skills != resulting_skills || overrides != resulting_overrides
    {
        return Err(PlayerSpellAcquisitionPrepareErrorLikeCpp::ResultingSnapshotMismatch);
    }

    // Action causality is checked only after the complete mutation stream has
    // replayed successfully. It may then consume transition evidence by
    // occurrence without trusting malformed typed projections.
    validate_post_commit_actions_like_cpp(plan)?;

    let mut profession_inputs = plan.profession_association_inputs.clone();
    profession_inputs.sort_by_key(|skill| skill.skill_id);
    let mut resulting_skill_rows = plan.resulting_snapshot.skills.clone();
    resulting_skill_rows.sort_by_key(|skill| skill.skill_id);
    if profession_inputs != resulting_skill_rows {
        return Err(PlayerSpellAcquisitionPrepareErrorLikeCpp::ProfessionInputsMismatch);
    }
    Ok(())
}

pub(super) fn validate_root_like_cpp(
    root: SpellAcquisitionRootLikeCpp,
) -> Result<(), PlayerSpellAcquisitionPrepareErrorLikeCpp> {
    let spell_id = match root {
        SpellAcquisitionRootLikeCpp::DirectLearn(spell_id)
        | SpellAcquisitionRootLikeCpp::TrainerWrapperCast(spell_id) => spell_id,
    };
    if spell_id == 0 || i32::try_from(spell_id).is_err() {
        return Err(PlayerSpellAcquisitionPrepareErrorLikeCpp::InvalidSpellId(
            spell_id,
        ));
    }
    Ok(())
}

pub(super) fn validate_profession_plan_like_cpp(
    acquisition_plan: &SpellAcquisitionPlanLikeCpp,
    profession_plan: &PrimaryProfessionCapacityPlanLikeCpp,
) -> Result<(), PlayerSpellAcquisitionPrepareErrorLikeCpp> {
    if profession_plan.configured_max > MAX_PRIMARY_TRADE_SKILLS_CONFIG_LIKE_CPP
        || profession_plan.used_before != profession_plan.existing_professions.len()
        || profession_plan.free_before
            != usize::from(profession_plan.configured_max)
                .saturating_sub(profession_plan.used_before)
        || profession_plan.new_professions.len() > profession_plan.free_before
    {
        return Err(
            PlayerSpellAcquisitionPrepareErrorLikeCpp::ProfessionPlanMismatch(
                "capacity arithmetic",
            ),
        );
    }

    let new_ids = profession_plan
        .new_professions
        .iter()
        .map(|profession| profession.skill_id)
        .collect::<Vec<_>>();
    if new_ids != acquisition_plan.root_primary_profession_skill_ids {
        return Err(
            PlayerSpellAcquisitionPrepareErrorLikeCpp::ProfessionPlanMismatch(
                "new profession order",
            ),
        );
    }

    let source_skills = skill_map_like_cpp(&acquisition_plan.source_snapshot)?;
    let resulting_skills = skill_map_like_cpp(&acquisition_plan.resulting_snapshot)?;
    let expected_existing_ids = acquisition_plan
        .source_snapshot
        .primary_profession_skill_ids
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    let expected_resulting_primary_ids = expected_existing_ids
        .iter()
        .copied()
        .chain(
            acquisition_plan
                .root_primary_profession_skill_ids
                .iter()
                .copied(),
        )
        .collect::<BTreeSet<_>>();
    if acquisition_plan
        .resulting_snapshot
        .primary_profession_skill_ids
        .iter()
        .copied()
        .collect::<BTreeSet<_>>()
        != expected_resulting_primary_ids
    {
        return Err(
            PlayerSpellAcquisitionPrepareErrorLikeCpp::ProfessionPlanMismatch(
                "resulting primary profession authority",
            ),
        );
    }
    let actual_existing_ids = profession_plan
        .existing_professions
        .iter()
        .map(|profession| profession.skill_id)
        .collect::<BTreeSet<_>>();
    if actual_existing_ids != expected_existing_ids
        || actual_existing_ids.len() != profession_plan.existing_professions.len()
    {
        return Err(
            PlayerSpellAcquisitionPrepareErrorLikeCpp::ProfessionPlanMismatch(
                "complete existing profession membership",
            ),
        );
    }
    let mut assigned_skill_ids = BTreeSet::new();
    let mut assigned_slots = BTreeSet::new();
    for (profession, existing) in profession_plan
        .existing_professions
        .iter()
        .map(|profession| (profession, true))
        .chain(
            profession_plan
                .new_professions
                .iter()
                .map(|profession| (profession, false)),
        )
    {
        if !assigned_skill_ids.insert(profession.skill_id)
            || !resulting_skills
                .get(&profession.skill_id)
                .is_some_and(|skill| skill.state != PlayerSkillPersistenceStateLikeCpp::Deleted)
            || (existing
                && !source_skills
                    .get(&profession.skill_id)
                    .is_some_and(|skill| skill.value != 0))
            || profession
                .equipment_slot
                .is_some_and(|slot| !assigned_slots.insert(slot))
        {
            return Err(
                PlayerSpellAcquisitionPrepareErrorLikeCpp::ProfessionPlanMismatch(
                    "profession membership or slot assignment",
                ),
            );
        }
    }

    let mut normalized_skill_ids = BTreeSet::new();
    for normalization in &profession_plan.slot_normalizations {
        let Some(source) = source_skills.get(&normalization.skill_id) else {
            return Err(
                PlayerSpellAcquisitionPrepareErrorLikeCpp::ProfessionPlanMismatch(
                    "normalization source skill",
                ),
            );
        };
        let assigned_slot = profession_plan
            .existing_professions
            .iter()
            .chain(&profession_plan.new_professions)
            .find(|profession| profession.skill_id == normalization.skill_id)
            .map(|profession| profession.equipment_slot);
        if !normalized_skill_ids.insert(normalization.skill_id)
            || source.profession_association.database_value_like_cpp()
                != normalization.original_slot
            || !resulting_skills.contains_key(&normalization.skill_id)
            || match assigned_slot {
                Some(slot) => slot != normalization.normalized_slot,
                None => normalization.normalized_slot.is_some(),
            }
        {
            return Err(
                PlayerSpellAcquisitionPrepareErrorLikeCpp::ProfessionPlanMismatch(
                    "slot normalization",
                ),
            );
        }
    }
    Ok(())
}

pub(super) fn validate_provenance_like_cpp(
    provenance: &SpellAcquisitionProvenanceLikeCpp,
    root: SpellAcquisitionRootLikeCpp,
) -> Result<(), PlayerSpellAcquisitionPrepareErrorLikeCpp> {
    let valid_spell = |spell_id: u32| spell_id != 0 && i32::try_from(spell_id).is_ok();
    let valid_skill = |skill_id: u32| skill_id != 0 && u16::try_from(skill_id).is_ok();
    let valid = match provenance {
        SpellAcquisitionProvenanceLikeCpp::Root {
            root: provenance_root,
        } => *provenance_root == root,
        SpellAcquisitionProvenanceLikeCpp::PreviousRank { requested_spell_id } => {
            valid_spell(*requested_spell_id)
        }
        SpellAcquisitionProvenanceLikeCpp::LearnDependency { source_spell_id }
        | SpellAcquisitionProvenanceLikeCpp::HigherDisabledRank { source_spell_id }
        | SpellAcquisitionProvenanceLikeCpp::DirectLearnSkill { source_spell_id } => {
            valid_spell(*source_spell_id)
        }
        SpellAcquisitionProvenanceLikeCpp::RequiredDisabledSpell { required_spell_id } => {
            valid_spell(*required_spell_id)
        }
        SpellAcquisitionProvenanceLikeCpp::SkillLineAbilityFallback {
            source_spell_id,
            record_id,
        } => valid_spell(*source_spell_id) && *record_id != 0,
        SpellAcquisitionProvenanceLikeCpp::ParentSkill { child_skill_id } => {
            valid_skill(*child_skill_id)
        }
        SpellAcquisitionProvenanceLikeCpp::RootChildSkill { parent_skill_id } => {
            valid_skill(*parent_skill_id)
        }
        SpellAcquisitionProvenanceLikeCpp::SkillReward {
            skill_id,
            record_id,
        } => valid_skill(*skill_id) && *record_id != 0,
        SpellAcquisitionProvenanceLikeCpp::WrapperEffect {
            wrapper_spell_id,
            record_id,
            ..
        } => valid_spell(*wrapper_spell_id) && *record_id != 0,
        SpellAcquisitionProvenanceLikeCpp::AutocastEffect {
            source_spell_id,
            record_id,
            ..
        } => valid_spell(*source_spell_id) && *record_id != 0,
    };
    if !valid {
        return Err(
            PlayerSpellAcquisitionPrepareErrorLikeCpp::ProvenanceMismatch(
                "invalid or root-mismatched transition provenance",
            ),
        );
    }
    Ok(())
}

pub(super) fn validate_snapshot_identity_like_cpp(
    source: &PlayerSpellAcquisitionSnapshotLikeCpp,
    resulting: &PlayerSpellAcquisitionSnapshotLikeCpp,
) -> Result<(), PlayerSpellAcquisitionPrepareErrorLikeCpp> {
    for (same, field) in [
        (
            source.character_guid == resulting.character_guid,
            "character_guid",
        ),
        (source.race == resulting.race, "race"),
        (source.class == resulting.class, "class"),
        (source.level == resulting.level, "level"),
        (source.lifecycle == resulting.lifecycle, "lifecycle"),
        (
            source.future_player_condition_resolutions
                == resulting.future_player_condition_resolutions,
            "future_player_condition_resolutions",
        ),
        (
            source.cast_resolutions == resulting.cast_resolutions,
            "cast_resolutions",
        ),
    ] {
        if !same {
            return Err(PlayerSpellAcquisitionPrepareErrorLikeCpp::SnapshotIdentityChanged(field));
        }
    }
    Ok(())
}

pub(super) fn validate_snapshot_like_cpp(
    snapshot: &PlayerSpellAcquisitionSnapshotLikeCpp,
) -> Result<(), PlayerSpellAcquisitionPrepareErrorLikeCpp> {
    if snapshot
        .character_guid
        .is_some_and(|guid| !guid.is_player() || guid.counter() == 0)
    {
        return Err(
            PlayerSpellAcquisitionPrepareErrorLikeCpp::SnapshotIdentityChanged("character_guid"),
        );
    }
    let spells = spell_map_like_cpp(snapshot)?;
    let skills = skill_map_like_cpp(snapshot)?;
    let _ = override_set_like_cpp(snapshot)?;
    if snapshot
        .primary_profession_skill_ids
        .windows(2)
        .any(|pair| pair[0] >= pair[1])
        || snapshot
            .primary_profession_skill_ids
            .iter()
            .any(|skill_id| {
                !skills.get(skill_id).is_some_and(|skill| {
                    skill.state != PlayerSkillPersistenceStateLikeCpp::Deleted && skill.value != 0
                })
            })
    {
        return Err(
            PlayerSpellAcquisitionPrepareErrorLikeCpp::ProfessionPlanMismatch(
                "snapshot primary profession authority",
            ),
        );
    }
    if let Some(invalid_skill_id) = snapshot
        .non_durable_skill_tombstone_ids
        .windows(2)
        .find_map(|pair| (pair[0] >= pair[1]).then_some(pair[1]))
        .or_else(|| {
            snapshot
                .non_durable_skill_tombstone_ids
                .iter()
                .copied()
                .find(|skill_id| {
                    !skills.get(skill_id).is_some_and(|skill| {
                        skill.step == 0
                            && skill.value == 0
                            && skill.maximum == 0
                            && skill.profession_association
                                == ProfessionAssociationInputLikeCpp::Unassigned
                            && matches!(
                                skill.state,
                                PlayerSkillPersistenceStateLikeCpp::Unchanged
                                    | PlayerSkillPersistenceStateLikeCpp::Deleted
                            )
                    })
                })
        })
    {
        return Err(
            PlayerSpellAcquisitionPrepareErrorLikeCpp::InvalidNonDurableSkillTombstone(
                invalid_skill_id,
            ),
        );
    }
    if usize::from(snapshot.occupied_skill_slots) != skills.len()
        || snapshot.occupied_skill_slots > 256
    {
        return Err(PlayerSpellAcquisitionPrepareErrorLikeCpp::SkillOccupancyMismatch);
    }

    let mut profession_slots = BTreeMap::<u8, u32>::new();
    for skill in skills.values() {
        if skill.state == PlayerSkillPersistenceStateLikeCpp::Deleted
            && (skill.step != 0
                || skill.value != 0
                || skill.maximum != 0
                || skill.profession_association != ProfessionAssociationInputLikeCpp::Unassigned)
        {
            return Err(
                PlayerSpellAcquisitionPrepareErrorLikeCpp::InvalidDeletedSkill(skill.skill_id),
            );
        }
        match skill.profession_association {
            ProfessionAssociationInputLikeCpp::Unassigned => {}
            ProfessionAssociationInputLikeCpp::Slot(slot @ 0..=1) => {
                if profession_slots.insert(slot, skill.skill_id).is_some() {
                    return Err(
                        PlayerSpellAcquisitionPrepareErrorLikeCpp::ConflictingProfessionAssociation(
                            slot,
                        ),
                    );
                }
            }
            ProfessionAssociationInputLikeCpp::Slot(slot) => {
                return Err(
                    PlayerSpellAcquisitionPrepareErrorLikeCpp::InvalidProfessionAssociation(
                        slot as i8,
                    ),
                );
            }
            ProfessionAssociationInputLikeCpp::Invalid(value) => {
                return Err(
                    PlayerSpellAcquisitionPrepareErrorLikeCpp::InvalidProfessionAssociation(value),
                );
            }
        }
    }
    for spell in spells.values() {
        if let Some(trait_definition_id) = spell.trait_definition_id
            && trait_definition_id <= 0
        {
            return Err(
                PlayerSpellAcquisitionPrepareErrorLikeCpp::InvalidTraitDefinitionId(
                    trait_definition_id,
                ),
            );
        }
    }
    Ok(())
}
