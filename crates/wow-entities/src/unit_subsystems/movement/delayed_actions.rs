//! Delayed actions packets.
//!
//! Separated from movement.rs under #693.

use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum MotionMasterDelayedActionType {
    Clear = 0,
    ClearSlot = 1,
    ClearMode = 2,
    ClearPriority = 3,
    Add = 4,
    Remove = 5,
    RemoveType = 6,
    Initialize = 7,
}

impl MotionMasterDelayedActionType {
    pub const fn trinity_id(self) -> u8 {
        self as u8
    }

    pub const fn from_trinity_id(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::Clear),
            1 => Some(Self::ClearSlot),
            2 => Some(Self::ClearMode),
            3 => Some(Self::ClearPriority),
            4 => Some(Self::Add),
            5 => Some(Self::Remove),
            6 => Some(Self::RemoveType),
            7 => Some(Self::Initialize),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MotionMasterDelayedActionPayload {
    Clear,
    ClearSlot(MovementSlot),
    ClearMode(MovementGeneratorMode),
    ClearPriority(MovementGeneratorPriority),
    Add(MovementGeneratorRef),
    Remove {
        kind: MovementGeneratorKind,
        slot: MovementSlot,
    },
    RemoveType {
        kind: MovementGeneratorKind,
        slot: MovementSlot,
    },
    Initialize,
}

impl MotionMasterDelayedActionPayload {
    pub const fn action_type(self) -> MotionMasterDelayedActionType {
        match self {
            Self::Clear => MotionMasterDelayedActionType::Clear,
            Self::ClearSlot(_) => MotionMasterDelayedActionType::ClearSlot,
            Self::ClearMode(_) => MotionMasterDelayedActionType::ClearMode,
            Self::ClearPriority(_) => MotionMasterDelayedActionType::ClearPriority,
            Self::Add(_) => MotionMasterDelayedActionType::Add,
            Self::Remove { .. } => MotionMasterDelayedActionType::Remove,
            Self::RemoveType { .. } => MotionMasterDelayedActionType::RemoveType,
            Self::Initialize => MotionMasterDelayedActionType::Initialize,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MotionMasterDelayedAction {
    pub payload: MotionMasterDelayedActionPayload,
    pub validator_passed: bool,
}

impl MotionMasterDelayedAction {
    pub const fn new(payload: MotionMasterDelayedActionPayload) -> Self {
        Self {
            payload,
            validator_passed: true,
        }
    }

    pub const fn with_validator(
        payload: MotionMasterDelayedActionPayload,
        validator_passed: bool,
    ) -> Self {
        Self {
            payload,
            validator_passed,
        }
    }

    pub const fn action_type(self) -> MotionMasterDelayedActionType {
        self.payload.action_type()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MotionMasterResolvedDelayedAction {
    pub action_type: MotionMasterDelayedActionType,
    pub executed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MotionMasterUpdateContext {
    pub diff_ms: u32,
    pub can_move: bool,
    pub owner_exists: bool,
    pub owner_is_standing: bool,
    pub spline_finalized: bool,
    pub spline_cyclic: bool,
    pub current_orientation: f32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MotionMasterUpdateOutcome {
    Stalled,
    Empty,
    Updated {
        popped: Option<MovementGeneratorRef>,
        resolved_delayed_actions: Vec<MotionMasterResolvedDelayedAction>,
    },
}

/// Represented local evidence for C++ `MotionMaster::AddToWorld()`
/// (`MotionMaster.cpp:120-132`).
///
/// This preserves the C++ initialization-pending guard and flag transitions,
/// calls the existing represented `DirectInitialize`/delayed-action helpers,
/// and does not claim real movement-generator runtime, pathing, packets, or
/// owner/fanout behavior.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MotionMasterAddToWorldOutcomeLikeCpp {
    pub had_initialization_pending: bool,
    pub entered_initializing: bool,
    pub direct_initialize_represented: bool,
    pub resolved_delayed_actions: Vec<MotionMasterResolvedDelayedAction>,
    pub exited_initializing: bool,
    pub flags_before: u8,
    pub flags_after: u8,
    pub current_generator_after: MovementGeneratorKind,
}
