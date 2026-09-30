//! Individual row projections and acquisition snapshot invariants.

use super::*;

pub(super) fn acquisition_spell_rows(
    snapshot: &PlayerSpellAcquisitionSnapshotLikeCpp,
) -> Result<
    BTreeMap<u32, PlayerSpellAcquisitionRowLikeCpp>,
    AcquisitionPlanError,
> {
    let mut rows = BTreeMap::new();
    for row in &snapshot.spells {
        if i32::try_from(row.spell_id).is_err() || row.spell_id == 0 {
            return Err(AcquisitionPlanError::InvalidSpellId(
                row.spell_id,
            ));
        }
        if rows.insert(row.spell_id, *row).is_some() {
            return Err(AcquisitionPlanError::DuplicateSpell(
                row.spell_id,
            ));
        }
    }
    Ok(rows)
}

/// Project the skill rows consumed independently by application profession checks.
pub fn acquisition_skill_rows(
    snapshot: &PlayerSpellAcquisitionSnapshotLikeCpp,
) -> Result<
    BTreeMap<u32, PlayerSkillAcquisitionRowLikeCpp>,
    AcquisitionPlanError,
> {
    let mut rows = BTreeMap::new();
    for row in &snapshot.skills {
        if u16::try_from(row.skill_id).is_err() || row.skill_id == 0 {
            return Err(AcquisitionPlanError::InvalidSkillId(
                row.skill_id,
            ));
        }
        if rows.insert(row.skill_id, *row).is_some() {
            return Err(AcquisitionPlanError::DuplicateSkill(
                row.skill_id,
            ));
        }
    }
    Ok(rows)
}

pub(super) fn acquisition_override_pairs(
    snapshot: &PlayerSpellAcquisitionSnapshotLikeCpp,
) -> Result<BTreeSet<(u32, u32)>, AcquisitionPlanError> {
    let mut overrides = BTreeSet::new();
    for &(overridden, overriding) in &snapshot.overrides {
        if i32::try_from(overridden).is_err() || overridden == 0 {
            return Err(AcquisitionPlanError::InvalidSpellId(
                overridden,
            ));
        }
        if i32::try_from(overriding).is_err() || overriding == 0 {
            return Err(AcquisitionPlanError::InvalidSpellId(
                overriding,
            ));
        }
        if !overrides.insert((overridden, overriding)) {
            return Err(
                AcquisitionPlanError::DuplicateOverride(
                    overridden, overriding,
                ),
            );
        }
    }
    Ok(overrides)
}

pub(super) fn validate_snapshot_identity(
    source: &PlayerSpellAcquisitionSnapshotLikeCpp,
    resulting: &PlayerSpellAcquisitionSnapshotLikeCpp,
) -> Result<(), AcquisitionPlanError> {
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
            return Err(AcquisitionPlanError::SnapshotIdentityChanged(field));
        }
    }
    Ok(())
}

pub(super) fn validate_snapshot(
    snapshot: &PlayerSpellAcquisitionSnapshotLikeCpp,
) -> Result<(), AcquisitionPlanError> {
    if snapshot
        .character_guid
        .is_some_and(|guid| !guid.is_player() || guid.counter() == 0)
    {
        return Err(
            AcquisitionPlanError::SnapshotIdentityChanged("character_guid"),
        );
    }
    let spells = acquisition_spell_rows(snapshot)?;
    let skills = acquisition_skill_rows(snapshot)?;
    let _ = acquisition_override_pairs(snapshot)?;
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
            AcquisitionPlanError::ProfessionPlanMismatch(
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
            AcquisitionPlanError::InvalidNonDurableSkillTombstone(
                invalid_skill_id,
            ),
        );
    }
    if usize::from(snapshot.occupied_skill_slots) != skills.len()
        || snapshot.occupied_skill_slots > 256
    {
        return Err(AcquisitionPlanError::SkillOccupancyMismatch);
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
                AcquisitionPlanError::InvalidDeletedSkill(skill.skill_id),
            );
        }
        match skill.profession_association {
            ProfessionAssociationInputLikeCpp::Unassigned => {}
            ProfessionAssociationInputLikeCpp::Slot(slot @ 0..=1) => {
                if profession_slots.insert(slot, skill.skill_id).is_some() {
                    return Err(
                        AcquisitionPlanError::ConflictingProfessionAssociation(
                            slot,
                        ),
                    );
                }
            }
            ProfessionAssociationInputLikeCpp::Slot(slot) => {
                return Err(
                    AcquisitionPlanError::InvalidProfessionAssociation(
                        slot as i8,
                    ),
                );
            }
            ProfessionAssociationInputLikeCpp::Invalid(value) => {
                return Err(
                    AcquisitionPlanError::InvalidProfessionAssociation(value),
                );
            }
        }
    }
    for spell in spells.values() {
        if let Some(trait_definition_id) = spell.trait_definition_id
            && trait_definition_id <= 0
        {
            return Err(
                AcquisitionPlanError::InvalidTraitDefinitionId(
                    trait_definition_id,
                ),
            );
        }
    }
    Ok(())
}
