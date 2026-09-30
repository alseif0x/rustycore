//! Dormant APP consumer of the prepared canonical Actor respawn operations.
//! Activation must quiesce producers/borrowers and retire the legacy rail first.
use super::*;
use std::time::Instant;
use wow_map::map_manager::{
    LiveTerrainHeights, snap_respawn_creature_to_ground_like_cpp,
    world_creature_from_pending_respawn_like_cpp,
};
use wow_map::spawn::ActorRespawnPhaseOutcome;

/// Caller has stopped lifecycle/spell queue producers and settled every payload
/// previously drained outside the legacy guard. This function acquires the SAME
/// runtime mutation-order fence; it does not create a lock, task or scheduler.
/// No producer calls it until the coordinated activation checkpoint.
#[allow(dead_code)]
pub(crate) fn transfer_respawns_under_quiescence(
    legacy: &SharedMapManager,
    canonical: &wow_world::session::SharedCanonicalMapManager,
    writer_fence: &SharedRespawnDbMutationOrderLikeCpp,
    key: wow_map::MapKey,
    incarnation: u64,
) -> Result<(), wow_world::session::RespawnOwnerTransferError> {
    let guard = writer_fence
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    wow_world::session::WorldSession::transfer_legacy_respawns_to_canonical(
        legacy,
        canonical,
        key,
        incarnation,
        &guard,
    )
}

/// APP retains all owned evidence on rejection; no implicit disposal or replay.
#[derive(Debug)]
pub(crate) struct PreparedActorRespawnFailure {
    pub(crate) error: Option<wow_map::manager::ActorRespawnError>,
    pub(crate) reply: Option<wow_map::manager::ActorRespawnReply>,
    pub(crate) completed: Vec<(wow_map::MapKey, ActorRespawnPhaseOutcome)>,
}

/// One admitted Map at a time, before ObjectUpdater. No factory or terrain call
/// occurs inside a canonical guard; supplied clocks are never recaptured.
#[allow(dead_code, clippy::too_many_arguments)]
pub(crate) fn prepared_actor_respawn_phase(
    canonical: &wow_world::session::SharedCanonicalMapManager,
    plan: &wow_map::MapTickPlanLikeCpp,
    map_store: &wow_data::MapStore,
    terrain: Option<&LiveTerrainHeights>,
    now: Instant,
    conversion_now: Instant,
    conversion_now_secs: i64,
) -> Result<Vec<(wow_map::MapKey, ActorRespawnPhaseOutcome)>, PreparedActorRespawnFailure> {
    drive_prepared_actor_respawns(
        canonical,
        plan,
        map_store,
        now,
        conversion_now,
        conversion_now_secs,
        world_creature_from_pending_respawn_like_cpp,
        |actor, map_id| {
            if let Some(terrain) = terrain {
                snap_respawn_creature_to_ground_like_cpp(actor, map_id, terrain);
            }
        },
    )
}

#[allow(clippy::too_many_arguments)]
fn drive_prepared_actor_respawns(
    canonical: &wow_world::session::SharedCanonicalMapManager,
    plan: &wow_map::MapTickPlanLikeCpp,
    map_store: &wow_data::MapStore,
    now: Instant,
    conversion_now: Instant,
    conversion_now_secs: i64,
    mut factory: impl FnMut(
        &wow_map::map_manager::PendingRespawn,
        u32,
    ) -> wow_map::map_manager::WorldCreature,
    mut snap: impl FnMut(&mut wow_map::map_manager::WorldCreature, u16),
) -> Result<Vec<(wow_map::MapKey, ActorRespawnPhaseOutcome)>, PreparedActorRespawnFailure> {
    use wow_map::manager::{ActorRespawnError, ActorRespawnProgress};
    let mut outcomes = Vec::new();
    {
        let Ok(manager) = canonical.lock() else {
            return Err(PreparedActorRespawnFailure {
                error: None,
                reply: None,
                completed: outcomes,
            });
        };
        if let Err(error) = manager.validate_actor_respawn_phase(plan) {
            return Err(PreparedActorRespawnFailure {
                error: Some(error),
                reply: None,
                completed: outcomes,
            });
        }
    }
    for participant in plan.updated_maps_like_cpp() {
        let persistent_world_map = map_store
            .get(participant.key.map_id)
            .is_some_and(|entry| !entry.is_instanceable_like_cpp());
        let mut progress = {
            let Ok(mut manager) = canonical.lock() else {
                return Err(PreparedActorRespawnFailure {
                    error: None,
                    reply: None,
                    completed: outcomes,
                });
            };
            match manager.begin_actor_respawn_map(
                plan,
                *participant,
                now,
                conversion_now,
                conversion_now_secs,
                persistent_world_map,
            ) {
                Ok(progress) => progress,
                Err(ActorRespawnError::StaleParticipant) => {
                    if let Err(error) = manager.skip_stale_actor_respawn_map(plan, *participant) {
                        return Err(PreparedActorRespawnFailure {
                            error: Some(error),
                            reply: None,
                            completed: outcomes,
                        });
                    }
                    continue;
                }
                Err(error) => {
                    return Err(PreparedActorRespawnFailure {
                        error: Some(error),
                        reply: None,
                        completed: outcomes,
                    });
                }
            }
        }; // Drop canonical guard before entropy/ground query.
        loop {
            match progress {
                ActorRespawnProgress::Complete(outcome) => {
                    outcomes.push((participant.key, outcome));
                    break;
                }
                ActorRespawnProgress::Pending(request) => {
                    let reply = request.resolve(&mut factory, &mut snap);
                    let Ok(mut manager) = canonical.lock() else {
                        return Err(PreparedActorRespawnFailure {
                            error: None,
                            reply: Some(reply),
                            completed: outcomes,
                        });
                    };
                    progress = match manager.resume_actor_respawn(plan, reply) {
                        Ok(progress) => progress,
                        Err(rejected) => {
                            return Err(PreparedActorRespawnFailure {
                                error: Some(rejected.error),
                                reply: Some(rejected.owned),
                                completed: outcomes,
                            });
                        }
                    };
                }
            }
        }
    }
    Ok(outcomes)
}

#[cfg(test)]
mod tests;

/// Complete prepared APP boundary: memory under the canonical guard, mailbox
/// submission after releasing it, same existing writer fence through submission.
/// The returned scalar outcomes feed ordinary visibility publication afterwards.
/// This entry point has no active runtime caller until coordinated activation.
#[allow(dead_code, clippy::too_many_arguments)]
pub(crate) fn run_prepared_actor_respawns(
    canonical: &wow_world::session::SharedCanonicalMapManager,
    plan: &wow_map::MapTickPlanLikeCpp,
    map_store: &wow_data::MapStore,
    terrain: Option<&LiveTerrainHeights>,
    now: Instant,
    conversion_now: Instant,
    conversion_now_secs: i64,
    writer_fence: &SharedRespawnDbMutationOrderLikeCpp,
    writer: &RespawnDbWriterSenderLikeCpp,
) -> Result<
    (
        Vec<(wow_map::MapKey, ActorRespawnPhaseOutcome)>,
        usize,
        usize,
    ),
    PreparedActorRespawnFailure,
> {
    let _writer_guard = writer_fence
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let mut outcomes = prepared_actor_respawn_phase(
        canonical,
        plan,
        map_store,
        terrain,
        now,
        conversion_now,
        conversion_now_secs,
    )?;
    let mut produced = 0;
    let mut submitted = 0;
    for (_, outcome) in &mut outcomes {
        produced += outcome.respawn_db_mutations.len();
        for mutation in outcome.respawn_db_mutations.drain(..) {
            if writer.send(mutation).is_err() {
                tracing::error!(
                    "Shared respawn DB writer stopped before prepared Actor statement submission"
                );
            } else {
                submitted += 1;
            }
        }
    }
    Ok((outcomes, produced, submitted))
}
