//! Translate items of application.
//!
//! Separated from application.rs under #711; every item keeps its name,
//! signature and body.

use super::*;

pub(super) fn spell_map_like_cpp(
    snapshot: &PlayerSpellAcquisitionSnapshotLikeCpp,
) -> Result<
    BTreeMap<u32, PlayerSpellAcquisitionRowLikeCpp>,
    PlayerSpellAcquisitionPrepareErrorLikeCpp,
> {
    let mut rows = BTreeMap::new();
    for row in &snapshot.spells {
        if i32::try_from(row.spell_id).is_err() || row.spell_id == 0 {
            return Err(PlayerSpellAcquisitionPrepareErrorLikeCpp::InvalidSpellId(
                row.spell_id,
            ));
        }
        if rows.insert(row.spell_id, *row).is_some() {
            return Err(PlayerSpellAcquisitionPrepareErrorLikeCpp::DuplicateSpell(
                row.spell_id,
            ));
        }
    }
    Ok(rows)
}

pub(super) fn skill_map_like_cpp(
    snapshot: &PlayerSpellAcquisitionSnapshotLikeCpp,
) -> Result<
    BTreeMap<u32, PlayerSkillAcquisitionRowLikeCpp>,
    PlayerSpellAcquisitionPrepareErrorLikeCpp,
> {
    let mut rows = BTreeMap::new();
    for row in &snapshot.skills {
        if u16::try_from(row.skill_id).is_err() || row.skill_id == 0 {
            return Err(PlayerSpellAcquisitionPrepareErrorLikeCpp::InvalidSkillId(
                row.skill_id,
            ));
        }
        if rows.insert(row.skill_id, *row).is_some() {
            return Err(PlayerSpellAcquisitionPrepareErrorLikeCpp::DuplicateSkill(
                row.skill_id,
            ));
        }
    }
    Ok(rows)
}

pub(super) fn override_set_like_cpp(
    snapshot: &PlayerSpellAcquisitionSnapshotLikeCpp,
) -> Result<BTreeSet<(u32, u32)>, PlayerSpellAcquisitionPrepareErrorLikeCpp> {
    let mut overrides = BTreeSet::new();
    for &(overridden, overriding) in &snapshot.overrides {
        if i32::try_from(overridden).is_err() || overridden == 0 {
            return Err(PlayerSpellAcquisitionPrepareErrorLikeCpp::InvalidSpellId(
                overridden,
            ));
        }
        if i32::try_from(overriding).is_err() || overriding == 0 {
            return Err(PlayerSpellAcquisitionPrepareErrorLikeCpp::InvalidSpellId(
                overriding,
            ));
        }
        if !overrides.insert((overridden, overriding)) {
            return Err(
                PlayerSpellAcquisitionPrepareErrorLikeCpp::DuplicateOverride(
                    overridden, overriding,
                ),
            );
        }
    }
    Ok(overrides)
}

pub(super) fn translate_plan_like_cpp(
    plan: &SpellAcquisitionPlanLikeCpp,
    profession_plan: &PrimaryProfessionCapacityPlanLikeCpp,
) -> Result<PreparedPlayerSpellAcquisitionLikeCpp, PlayerSpellAcquisitionPrepareErrorLikeCpp> {
    let mut runtime_snapshot = plan.resulting_snapshot.clone();
    for normalization in &profession_plan.slot_normalizations {
        let skill = runtime_snapshot
            .skills
            .iter_mut()
            .find(|skill| skill.skill_id == normalization.skill_id)
            .expect("validated profession normalization skill");
        skill.profession_association =
            profession_association_like_cpp(normalization.normalized_slot);
    }
    for profession in profession_plan
        .existing_professions
        .iter()
        .chain(&profession_plan.new_professions)
    {
        let skill = runtime_snapshot
            .skills
            .iter_mut()
            .find(|skill| skill.skill_id == profession.skill_id)
            .expect("validated profession assignment skill");
        skill.profession_association = profession_association_like_cpp(profession.equipment_slot);
    }
    let pending_save_runtime_snapshot = runtime_snapshot.clone();
    let mut durable_spells = Vec::new();
    let mut durable_favorite_spell_ids = Vec::new();
    for spell in &plan.resulting_snapshot.spells {
        if spell.state == PlayerSpellPersistenceStateLikeCpp::Removed
            || spell.state == PlayerSpellPersistenceStateLikeCpp::Temporary
        {
            continue;
        }
        // C++ `_SaveSpells` suppresses the `character_spell` insert for a
        // dependent row, but favorite maintenance remains outside that
        // dependent check and therefore still persists independently.
        if spell.favorite {
            durable_favorite_spell_ids.push(i32::try_from(spell.spell_id).map_err(|_| {
                PlayerSpellAcquisitionPrepareErrorLikeCpp::InvalidSpellId(spell.spell_id)
            })?);
        }
        if spell.dependent {
            continue;
        }
        durable_spells.push(DurablePlayerSpellRowLikeCpp {
            spell_id: i32::try_from(spell.spell_id).map_err(|_| {
                PlayerSpellAcquisitionPrepareErrorLikeCpp::InvalidSpellId(spell.spell_id)
            })?,
            active: spell.active,
            disabled: spell.disabled,
        });
    }
    durable_spells.sort_by_key(|spell| spell.spell_id);
    durable_favorite_spell_ids.sort_unstable();

    let mut durable_skills = Vec::new();
    let source_non_durable_skill_tombstone_ids = plan
        .source_snapshot
        .non_durable_skill_tombstone_ids
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    let mut non_durable_skill_tombstone_ids = BTreeSet::new();
    for skill in &runtime_snapshot.skills {
        let remains_saved_tombstone = source_non_durable_skill_tombstone_ids
            .contains(&skill.skill_id)
            && skill.step == 0
            && skill.value == 0
            && skill.maximum == 0
            && skill.profession_association == ProfessionAssociationInputLikeCpp::Unassigned
            && matches!(
                skill.state,
                PlayerSkillPersistenceStateLikeCpp::Unchanged
                    | PlayerSkillPersistenceStateLikeCpp::Deleted
            );
        if skill.state == PlayerSkillPersistenceStateLikeCpp::Deleted || remains_saved_tombstone {
            non_durable_skill_tombstone_ids.insert(u16::try_from(skill.skill_id).map_err(
                |_| PlayerSpellAcquisitionPrepareErrorLikeCpp::InvalidSkillId(skill.skill_id),
            )?);
            continue;
        }
        durable_skills.push(DurablePlayerSkillRowLikeCpp {
            skill_id: u16::try_from(skill.skill_id).map_err(|_| {
                PlayerSpellAcquisitionPrepareErrorLikeCpp::InvalidSkillId(skill.skill_id)
            })?,
            value: skill.value,
            maximum: skill.maximum,
            profession_slot: skill.profession_association.database_value_like_cpp(),
        });
    }
    durable_skills.sort_by_key(|skill| skill.skill_id);
    runtime_snapshot.non_durable_skill_tombstone_ids = non_durable_skill_tombstone_ids
        .iter()
        .map(|skill_id| u32::from(*skill_id))
        .collect();

    runtime_snapshot
        .spells
        .retain(|spell| spell.state != PlayerSpellPersistenceStateLikeCpp::Removed);
    for spell in &mut runtime_snapshot.spells {
        if spell.state != PlayerSpellPersistenceStateLikeCpp::Temporary {
            spell.state = PlayerSpellPersistenceStateLikeCpp::Unchanged;
        }
    }
    for skill in &mut runtime_snapshot.skills {
        skill.state = PlayerSkillPersistenceStateLikeCpp::Unchanged;
    }

    let mut durable_operations = vec![
        PlayerSpellAcquisitionDurableOperationLikeCpp::LockCharacter,
        PlayerSpellAcquisitionDurableOperationLikeCpp::DeleteSpells,
        PlayerSpellAcquisitionDurableOperationLikeCpp::DeleteFavoriteSpells,
        PlayerSpellAcquisitionDurableOperationLikeCpp::DeleteSkills,
    ];
    durable_operations.extend(
        durable_spells
            .iter()
            .copied()
            .map(PlayerSpellAcquisitionDurableOperationLikeCpp::InsertSpell),
    );
    durable_operations.extend(
        durable_favorite_spell_ids
            .iter()
            .copied()
            .map(PlayerSpellAcquisitionDurableOperationLikeCpp::InsertFavoriteSpell),
    );
    durable_operations.extend(
        durable_skills
            .iter()
            .copied()
            .map(PlayerSpellAcquisitionDurableOperationLikeCpp::InsertSkill),
    );

    Ok(PreparedPlayerSpellAcquisitionLikeCpp {
        character_guid: plan.source_snapshot.character_guid,
        root: plan.root,
        source_snapshot: plan.source_snapshot.clone(),
        pending_save_runtime_snapshot,
        runtime_snapshot,
        durable_spells,
        durable_favorite_spell_ids,
        durable_skills,
        non_durable_skill_tombstone_ids,
        durable_operations,
        post_commit_actions: plan.post_commit_actions.clone(),
    })
}

pub(super) fn profession_association_like_cpp(
    slot: Option<PrimaryProfessionEquipmentSlotLikeCpp>,
) -> ProfessionAssociationInputLikeCpp {
    slot.map(|slot| ProfessionAssociationInputLikeCpp::Slot(slot.db_value_like_cpp() as u8))
        .unwrap_or(ProfessionAssociationInputLikeCpp::Unassigned)
}
