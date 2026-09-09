//! Unit values, visibility and health-revision state regression scenarios.
//!
//! Separated from the unit.rs root under #636.

use super::*;
use crate::{
    AppliedAuraRef, AuraRef, CurrentSpellRef, CurrentSpellSlot, MAX_SUMMON_SLOT,
    MOTIONMASTER_FLAG_INITIALIZATION_PENDING, MOTIONMASTER_FLAG_INITIALIZING,
    MotionMasterDelayedActionPayload, MovementGeneratorKind, MovementGeneratorRef, MovementSlot,
    OwnedAuraRef,
};

mod scenarios_1;
mod scenarios_2;
mod scenarios_3;
