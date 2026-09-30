//! Application-owned profession capacity, membership and slot assignments.

use super::*;

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

    let source_skills = acquisition_skill_rows(&acquisition_plan.source_snapshot)?;
    let resulting_skills = acquisition_skill_rows(&acquisition_plan.resulting_snapshot)?;
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
