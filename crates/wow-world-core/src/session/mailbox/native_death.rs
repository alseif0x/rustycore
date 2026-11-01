//! Required Player death work inside an admitted Map combat future.

use std::sync::Arc;

use super::{MapPhaseAdmissionLikeCpp, SessionPhasePassOutcomeLikeCpp, SessionPhasePermitLikeCpp};

/// Bound to the current Map pass and the Player's own operation identity.
/// Several operations may run in one tick; this does not advance a phase clock.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NativePlayerDeathAdmission {
    pub map_phase: MapPhaseAdmissionLikeCpp,
    pub map_tick_epoch: u64,
    pub operation: wow_entities::PlayerDeathOperationId,
    /// Frozen before callbacks, as in Unit::Kill's controlling-player lookup.
    pub pvp_death: bool,
}

#[derive(Clone, Debug)]
pub struct RunNativePlayerDeathRequest {
    pub admission: NativePlayerDeathAdmission,
    pub permit: Arc<SessionPhasePermitLikeCpp>,
    pub response_tx: flume::Sender<NativePlayerDeathReply>,
    /// CalcAbsorbResist and AttackerStateUpdate precede Unit::Kill. This is
    /// consumed by the narrow prefix publisher, never the late health handler.
    pub creature_melee_prefix: Option<super::ApplyCreatureMeleeDamageLikeCppCommand>,
    pub attack_start: Option<super::CreatureAttackStartLikeCppCommand>,
    /// Existing aggro frames through AttackStart, in their original order.
    pub attack_start_packets: Vec<Vec<u8>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NativePlayerDeathReply {
    pub admission: NativePlayerDeathAdmission,
    pub outcome: SessionPhasePassOutcomeLikeCpp,
    /// Only Some means the native body and post-Kill penalty completed.
    /// The combat future must acknowledge the matching completed operation.
    pub health_after: Option<u64>,
}

/// A committed primary melee hit whose absorb callbacks and presentation must
/// finish before the next swing. This carries no arbitrary mailbox command.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CommittedCreatureMeleeAdmission {
    pub map_phase: MapPhaseAdmissionLikeCpp,
    pub map_tick_epoch: u64,
}

/// The three observable stages of one primary creature melee presentation.
/// Production runs the prefix before share damage and the health update only
/// after the canonical primary transition has committed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CreatureMeleePresentationStageLikeCpp {
    Whole,
    AbsorbOnly,
    AttackerStateOnly,
    HealthOnly,
}

#[derive(Clone, Debug)]
pub struct RunCommittedCreatureMeleeRequest {
    pub admission: CommittedCreatureMeleeAdmission,
    pub command: super::ApplyCreatureMeleeDamageLikeCppCommand,
    pub presentation_stage: CreatureMeleePresentationStageLikeCpp,
    pub attack_start: Option<super::CreatureAttackStartLikeCppCommand>,
    pub attack_start_packets: Vec<Vec<u8>>,
    pub permit: Arc<SessionPhasePermitLikeCpp>,
    pub response_tx: flume::Sender<CommittedCreatureMeleeReply>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommittedCreatureMeleeReply {
    pub admission: CommittedCreatureMeleeAdmission,
    pub outcome: SessionPhasePassOutcomeLikeCpp,
    pub completed: bool,
}
