//! One manager-admitted Actor movement operation, dormant until producer migration.
//!
//! The token owner validates admission and maintains the per-map operation slot.
//! These adapters retain only immutable identity and the existing owned Step
//! continuations. They neither publish packets nor own an actor or a guard.

use super::{MapManager, MapObjectTickContinuation, ObjectMapUpdateToken};
use super::ActorTickAccessError;
use super::tick_objects::ActorStepIdentity;
use crate::map_manager::{
    ChaseTargetSnapshotLikeCpp, CreatureMovementStep, CreaturePathQueryLikeCpp,
    StepGridHeightContinuation, StepPathContinuation,
    StepPending, StepProgress, StepStaticHeightContinuation,
};
use wow_core::ObjectGuid;
use wow_entities::PhaseShift;
use wow_recastdetour::DetourPolyPath;

mod requests;
mod resume;
mod access;
mod completion;
mod error;
mod disposition;

pub use completion::{ActorMovementCompletion, ActorMovementTraceFacts};
pub use error::ActorMovementError;

pub use requests::{
    ActorGridHeightContinuation, ActorGridHeightRequest,
    ActorMovementPending, ActorMovementProgress,
    ActorPathContinuation, ActorPathRequest,
    ActorStaticHeightContinuation, ActorStaticHeightRequest,
    ActorStaticHeightQuery, ActorGridHeightQuery,
};

/// A rejected resume preserves its complete owned input for an explicit retry
/// or shutdown disposition. It does not release or recreate the operation slot.
pub struct ActorMovementResumeFailure<Continuation, Response> {
    pub error: ActorMovementError,
    pub continuation: Continuation,
    pub response: Response,
}

impl MapManager {
    /// Begin one selected Actor operation through the token owner's admission
    /// gate. Neither the Step prefix nor policy runs on a rejected admission.
    pub fn prepare_movement(
        &mut self,
        tick: &MapObjectTickContinuation,
        token: &mut ObjectMapUpdateToken,
        guid: ObjectGuid,
        target: Option<ChaseTargetSnapshotLikeCpp>,
        terrain_enabled: bool,
        policy: impl FnMut(u32, bool) -> bool,
    ) -> Result<ActorMovementProgress, ActorMovementError> {
        self.begin_actor_step(
            tick, token, guid,
            |actor, diff_ms| actor.prepare_movement_step(diff_ms, target, terrain_enabled, policy),
        ).map_err(Into::into)
    }
}

#[cfg(test)]
mod tests;
