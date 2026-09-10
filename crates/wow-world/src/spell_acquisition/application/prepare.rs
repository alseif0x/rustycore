//! Prepare items of application.
//!
//! Separated from application.rs under #711; every item keeps its name,
//! signature and body.

use super::*;

pub(crate) fn snapshot_has_pending_durable_save_like_cpp(
    snapshot: &PlayerSpellAcquisitionSnapshotLikeCpp,
) -> bool {
    snapshot.spells.iter().any(|spell| {
        !matches!(
            spell.state,
            PlayerSpellPersistenceStateLikeCpp::Unchanged
                | PlayerSpellPersistenceStateLikeCpp::Temporary
        )
    }) || snapshot
        .skills
        .iter()
        .any(|skill| skill.state != PlayerSkillPersistenceStateLikeCpp::Unchanged)
}

pub(crate) fn prepare_player_spell_acquisition_like_cpp(
    plan: &SpellAcquisitionPlanLikeCpp,
    profession_plan: &PrimaryProfessionCapacityPlanLikeCpp,
    current_snapshot: &PlayerSpellAcquisitionSnapshotLikeCpp,
) -> Result<PreparedPlayerSpellAcquisitionOutcomeLikeCpp, PlayerSpellAcquisitionPrepareErrorLikeCpp>
{
    validate_plan_replay_like_cpp(plan)?;
    validate_profession_plan_like_cpp(plan, profession_plan)?;
    let prepared = translate_plan_like_cpp(plan, profession_plan)?;

    if plan.mutations.is_empty() {
        if current_snapshot != &plan.source_snapshot {
            return Err(PlayerSpellAcquisitionPrepareErrorLikeCpp::StaleSnapshot);
        }
        return if plan.post_commit_actions.is_empty() {
            Ok(PreparedPlayerSpellAcquisitionOutcomeLikeCpp::NoChange)
        } else {
            Ok(PreparedPlayerSpellAcquisitionOutcomeLikeCpp::ActionsOnly(
                PreparedPlayerSpellAcquisitionActionsLikeCpp {
                    character_guid: plan.source_snapshot.character_guid,
                    // No durable mutation occurred, so do not normalize
                    // persistence states or replace runtime authority.
                    runtime_snapshot: plan.resulting_snapshot.clone(),
                    post_commit_actions: plan.post_commit_actions.clone(),
                    runtime_actions_already_applied: false,
                },
            ))
        };
    }
    if !plan.mutations.is_empty() && current_snapshot == &prepared.runtime_snapshot {
        return Ok(PreparedPlayerSpellAcquisitionOutcomeLikeCpp::AlreadyApplied);
    }
    if current_snapshot != &plan.source_snapshot {
        return Err(PlayerSpellAcquisitionPrepareErrorLikeCpp::StaleSnapshot);
    }

    Ok(PreparedPlayerSpellAcquisitionOutcomeLikeCpp::Ready(
        prepared,
    ))
}

/// Project the exact Character DB rows represented by a clean runtime source.
/// Temporary spells have no durable row; any other dirty state is ambiguous
/// until the ordinary C++ save lifecycle consumes it, so trainer persistence
/// must fail closed instead of guessing the database pre-state.
pub(super) fn stable_source_durable_authority_like_cpp(
    snapshot: &PlayerSpellAcquisitionSnapshotLikeCpp,
) -> Option<DurablePlayerSpellAcquisitionAuthorityLikeCpp> {
    let mut spells = Vec::new();
    let mut favorite_spell_ids = Vec::new();
    for spell in &snapshot.spells {
        if spell.state == PlayerSpellPersistenceStateLikeCpp::Temporary {
            continue;
        }
        if spell.state != PlayerSpellPersistenceStateLikeCpp::Unchanged {
            return None;
        }
        let spell_id = i32::try_from(spell.spell_id).ok()?;
        if spell.favorite {
            favorite_spell_ids.push(spell_id);
        }
        if !spell.dependent {
            spells.push(DurablePlayerSpellRowLikeCpp {
                spell_id,
                active: spell.active,
                disabled: spell.disabled,
            });
        }
    }
    spells.sort_by_key(|spell| spell.spell_id);
    favorite_spell_ids.sort_unstable();

    let non_durable_tombstones = snapshot
        .non_durable_skill_tombstone_ids
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    let mut skills = Vec::new();
    for skill in &snapshot.skills {
        if skill.state != PlayerSkillPersistenceStateLikeCpp::Unchanged {
            return None;
        }
        let skill_id = u16::try_from(skill.skill_id).ok()?;
        if non_durable_tombstones.contains(&skill.skill_id) {
            continue;
        }
        skills.push(DurablePlayerSkillRowLikeCpp {
            skill_id,
            value: skill.value,
            maximum: skill.maximum,
            profession_slot: skill.profession_association.database_value_like_cpp(),
        });
    }
    skills.sort_by_key(|skill| skill.skill_id);

    Some(DurablePlayerSpellAcquisitionAuthorityLikeCpp {
        spells,
        favorite_spell_ids,
        skills,
    })
}

/// Converts an already validated application plan into the complete SQLx-free
/// transaction request consumed by the Character-database adapter.
pub(crate) fn player_spell_acquisition_persistence_request_like_cpp(
    guid_counter: u64,
    prepared: &PreparedPlayerSpellAcquisitionLikeCpp,
    money_before: u64,
    money_after: u64,
    operation_token: [u8; 16],
) -> Result<PlayerSpellAcquisitionPersistenceRequestLikeCpp, String> {
    let prepared_counter = prepared
        .character_guid
        .and_then(|guid| u64::try_from(guid.counter()).ok());
    if prepared_counter != Some(guid_counter) {
        return Err("prepared player spell acquisition character GUID mismatch".to_owned());
    }
    let source_authority = stable_source_durable_authority_like_cpp(&prepared.source_snapshot)
        .ok_or_else(|| {
            "trainer acquisition source contains unsaved persistence state".to_owned()
        })?;
    Ok(PlayerSpellAcquisitionPersistenceRequestLikeCpp {
        player_guid: guid_counter,
        money_before,
        money_after,
        operation_token,
        source_authority,
        resulting_authority: DurablePlayerSpellAcquisitionAuthorityLikeCpp {
            spells: prepared.durable_spells.clone(),
            favorite_spell_ids: prepared.durable_favorite_spell_ids.clone(),
            skills: prepared.durable_skills.clone(),
        },
        operations: prepared.durable_operations.clone(),
    })
}

pub(crate) async fn persist_player_spell_acquisition_through_port_like_cpp(
    port: &dyn PlayerSpellAcquisitionPersistencePortLikeCpp,
    request: PlayerSpellAcquisitionPersistenceRequestLikeCpp,
) -> PlayerSpellAcquisitionPersistenceOutcomeLikeCpp {
    use wow_persistence::{
        PlayerSpellAcquisitionMoneyReconciliationLikeCpp as Reconciliation,
        PlayerSpellAcquisitionPersistenceAttemptLikeCpp as Attempt,
    };

    match port
        .attempt_player_spell_acquisition_like_cpp(request.clone())
        .await
    {
        Attempt::Applied => PlayerSpellAcquisitionPersistenceOutcomeLikeCpp::Applied,
        Attempt::DefinitelyRolledBack { reason, .. } => {
            PlayerSpellAcquisitionPersistenceOutcomeLikeCpp::DefinitelyRolledBack(reason)
        }
        Attempt::CommitOutcomeUnknown { reason } => match port
            .reconcile_player_spell_acquisition_like_cpp(request)
            .await
        {
            Reconciliation::Committed => {
                PlayerSpellAcquisitionPersistenceOutcomeLikeCpp::ReconciledCommit(reason)
            }
            Reconciliation::Indeterminate => {
                PlayerSpellAcquisitionPersistenceOutcomeLikeCpp::Indeterminate(reason)
            }
        },
    }
}
