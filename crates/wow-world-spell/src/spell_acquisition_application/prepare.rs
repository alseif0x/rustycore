// The pure preparation operation moved from the World application boundary.

use super::*;

pub fn prepare_player_spell_acquisition_like_cpp(
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
