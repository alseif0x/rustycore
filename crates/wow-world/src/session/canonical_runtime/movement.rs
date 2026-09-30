//! Complete canonical movement application pass; no producer calls it yet.

use std::collections::HashMap;
use std::sync::Arc;
use wow_core::ObjectGuid;
use wow_map::{ActorMovementError, ActorMovementProgress, MapObjectTickContinuation, ObjectMapUpdateToken};
use crate::map_manager::{ChaseTargetSnapshotLikeCpp, LiveTerrainHeights, RuntimePlan, WorldMMapPathfinderWorkerLikeCpp};
use crate::session::{MMapRuntimeConfigLikeCpp, SharedCanonicalMapManager};

mod error;
mod packets;
mod queries;
pub use error::{CanonicalMovementAbandoned, CanonicalMovementAbandonment, CanonicalMovementError};
use error::MovementFailure;
use packets::{append_completed_movement, MovementTrace};

/// Owned effects remain with the caller until the admitted phase is published.
pub struct CanonicalMovementOutcome {
    pub creatures_seen: usize,
    pub movement_packets: usize,
    pub plan: RuntimePlan,
}

impl CanonicalMovementOutcome {
    fn empty() -> Self {
        Self { creatures_seen: 0, movement_packets: 0, plan: RuntimePlan { events: Vec::new() } }
    }
}

/// Execute only this token's selected actors. The caller retains the tick and
/// token through publication, finish and terminal disposition. Cancelling this
/// future does not clear a pending actor operation or finalize a map tick.
#[allow(clippy::too_many_arguments)]
pub async fn run_canonical_movement(
    manager: &SharedCanonicalMapManager,
    tick: &MapObjectTickContinuation,
    token: &mut ObjectMapUpdateToken,
    config: &MMapRuntimeConfigLikeCpp,
    pathfinder: Option<&Arc<WorldMMapPathfinderWorkerLikeCpp>>,
    terrain: Option<&Arc<LiveTerrainHeights>>,
    chase_targets: HashMap<(u16, u32, ObjectGuid), ChaseTargetSnapshotLikeCpp>,
) -> Result<CanonicalMovementOutcome, CanonicalMovementError> {
    let mut outcome = CanonicalMovementOutcome::empty();
    let key = token.key();
    let map_id = match u16::try_from(key.map_id) {
        Ok(map_id) => map_id,
        Err(_) => return Err(CanonicalMovementError::new(outcome, MovementFailure::MapId(key.map_id))),
    };
    let guids = match manager.lock() {
        Ok(manager) => match manager.selected_actor_guids(tick, token) {
            Ok(guids) => guids,
            Err(error) => return Err(CanonicalMovementError::new(outcome, MovementFailure::Actor(error.into()))),
        },
        Err(_) => return Err(CanonicalMovementError::new(outcome, MovementFailure::Poisoned)),
    };
    for guid in guids {
        let mut progress = {
            let mut manager = match manager.lock() {
                Ok(manager) => manager,
                Err(_) => return Err(CanonicalMovementError::new(outcome, MovementFailure::Poisoned)),
            };
            // Both victim reads precede the mutable actor borrow. Player facts
            // have the original precedence; Creature liquid state stays None.
            let target = movement_target(&manager, key, map_id, guid, &chase_targets);
            match manager.prepare_movement(
                tick, token, guid, target, terrain.is_some(),
                |map_id, ignore| config.should_try_pathfinding_like_cpp(map_id, ignore),
            ) {
                Ok(progress) => progress,
                Err(error) => return Err(CanonicalMovementError::new(outcome, MovementFailure::Actor(error))),
            }
        };
        outcome.creatures_seen += 1;
        loop {
            match progress {
                ActorMovementProgress::Complete(completion) => {
                    let moved = append_completed_movement(
                        &mut outcome.plan, completion.guid, map_id, key.instance_id,
                        completion.position, completion.visibility_range, completion.movement,
                        completion.home_health_update,
                        MovementTrace { entry: completion.trace.entry, map_id: completion.trace.map_id, state: completion.trace.state },
                    );
                    outcome.movement_packets += usize::from(moved);
                    break;
                }
                ActorMovementProgress::Pending(pending) => {
                    let response = match queries::resolve(guid, pending, pathfinder, terrain).await {
                        Ok(response) => response,
                        Err(failure) => return Err(CanonicalMovementError::new(outcome, failure)),
                    };
                    // No map, actor or metadata borrow crosses the query await.
                    progress = match queries::resume(manager, tick, token, response) {
                        Ok(progress) => progress,
                        Err(failure) => return Err(CanonicalMovementError::new(outcome, failure)),
                    };
                }
            }
        }
    }
    Ok(outcome)
}

fn movement_target(
    manager: &wow_map::MapManager,
    key: wow_map::MapKey,
    map_id: u16,
    guid: ObjectGuid,
    chase_targets: &HashMap<(u16, u32, ObjectGuid), ChaseTargetSnapshotLikeCpp>,
) -> Option<ChaseTargetSnapshotLikeCpp> {
    let map = manager.find_map(key.map_id, key.instance_id)?.map();
    let target_guid = map.with_creature_like_cpp(guid, |creature| creature.ai_ownership().combat_target)??;
    chase_targets.get(&(map_id, key.instance_id, target_guid)).copied().or_else(|| {
        map.with_creature_like_cpp(target_guid, |target| ChaseTargetSnapshotLikeCpp {
            guid: target_guid,
            position: target.unit().world().position(),
            combat_reach: target.unit().data().combat_reach.max(0.0),
            in_world: target.is_alive(),
            in_water: None,
        })
    })
}

#[cfg(test)]
mod tests;
