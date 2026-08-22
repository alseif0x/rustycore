// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Unit motion master and movement generators.

use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MovementGeneratorKind {
    Idle,
    Random,
    Waypoint,
    Confused,
    Chase,
    Home,
    Flight,
    Point,
    Fleeing,
    Distract,
    Assistance,
    AssistanceDistract,
    TimedFleeing,
    Follow,
    Rotate,
    Effect,
    SplineChain,
    Formation,
    Custom(u32),
}

impl MovementGeneratorKind {
    pub const fn trinity_id(self) -> u8 {
        match self {
            Self::Idle => 0,
            Self::Random => 1,
            Self::Waypoint => 2,
            Self::Confused => 4,
            Self::Chase => 5,
            Self::Home => 6,
            Self::Flight => 7,
            Self::Point => 8,
            Self::Fleeing => 9,
            Self::Distract => 10,
            Self::Assistance => 11,
            Self::AssistanceDistract => 12,
            Self::TimedFleeing => 13,
            Self::Follow => 14,
            Self::Rotate => 15,
            Self::Effect => 16,
            Self::SplineChain => 17,
            Self::Formation => 18,
            Self::Custom(value) => value as u8,
        }
    }

    pub const fn from_trinity_id(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::Idle),
            1 => Some(Self::Random),
            2 => Some(Self::Waypoint),
            3 | 19..=u8::MAX => None,
            4 => Some(Self::Confused),
            5 => Some(Self::Chase),
            6 => Some(Self::Home),
            7 => Some(Self::Flight),
            8 => Some(Self::Point),
            9 => Some(Self::Fleeing),
            10 => Some(Self::Distract),
            11 => Some(Self::Assistance),
            12 => Some(Self::AssistanceDistract),
            13 => Some(Self::TimedFleeing),
            14 => Some(Self::Follow),
            15 => Some(Self::Rotate),
            16 => Some(Self::Effect),
            17 => Some(Self::SplineChain),
            18 => Some(Self::Formation),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(u8)]
pub enum MovementGeneratorMode {
    Default = 0,
    Override = 1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(u8)]
pub enum MovementGeneratorPriority {
    None = 0,
    Normal = 1,
    Highest = 2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum MovementSlot {
    Default = 0,
    Active = 1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MovementGeneratorRef {
    pub kind: MovementGeneratorKind,
    pub mode: MovementGeneratorMode,
    pub priority: MovementGeneratorPriority,
    pub slot: MovementSlot,
    pub flags: u16,
    pub base_unit_state: u32,
    pub target_guid: Option<ObjectGuid>,
    pub movement_id: u32,
    pub duration_ms: Option<u32>,
    pub max_duration_ms: Option<u32>,
    pub elapsed_ms: u32,
    pub arrival_spell_id: u32,
    pub arrival_spell_target_guid: ObjectGuid,
    pub rotate_direction: Option<RotateDirection>,
}

impl MovementGeneratorRef {
    pub const fn new(kind: MovementGeneratorKind, slot: MovementSlot) -> Self {
        Self {
            kind,
            mode: MovementGeneratorMode::Default,
            priority: MovementGeneratorPriority::None,
            slot,
            flags: MOVEMENTGENERATOR_FLAG_NONE,
            base_unit_state: 0,
            target_guid: None,
            movement_id: 0,
            duration_ms: None,
            max_duration_ms: None,
            elapsed_ms: 0,
            arrival_spell_id: 0,
            arrival_spell_target_guid: ObjectGuid::EMPTY,
            rotate_direction: None,
        }
    }

    pub const fn with_mode(mut self, mode: MovementGeneratorMode) -> Self {
        self.mode = mode;
        self
    }

    pub const fn with_priority(mut self, priority: MovementGeneratorPriority) -> Self {
        self.priority = priority;
        self
    }

    pub const fn with_flags(mut self, flags: u16) -> Self {
        self.flags = flags;
        self
    }

    pub const fn with_base_unit_state(mut self, base_unit_state: u32) -> Self {
        self.base_unit_state = base_unit_state;
        self
    }

    pub const fn with_target_guid(mut self, target_guid: ObjectGuid) -> Self {
        self.target_guid = Some(target_guid);
        self
    }

    pub const fn with_movement_id(mut self, movement_id: u32) -> Self {
        self.movement_id = movement_id;
        self
    }

    pub const fn with_duration_ms(mut self, duration_ms: u32) -> Self {
        self.duration_ms = Some(duration_ms);
        self
    }

    pub const fn with_max_duration_ms(mut self, max_duration_ms: u32) -> Self {
        self.max_duration_ms = Some(max_duration_ms);
        self
    }

    pub const fn with_rotate_direction(mut self, direction: RotateDirection) -> Self {
        self.rotate_direction = Some(direction);
        self
    }

    pub const fn with_arrival_spell(mut self, spell_id: u32, target_guid: ObjectGuid) -> Self {
        self.arrival_spell_id = spell_id;
        self.arrival_spell_target_guid = target_guid;
        self
    }

    pub const fn has_flag(&self, flag: u16) -> bool {
        (self.flags & flag) != 0
    }

    pub fn initialize_for_motion_master_update_like_cpp(
        &mut self,
        context: MotionMasterUpdateContext,
    ) {
        match self.kind {
            MovementGeneratorKind::Idle => {
                self.initialize_idle_like_cpp();
            }
            MovementGeneratorKind::Point => {
                self.initialize_point_like_cpp(context.can_move);
            }
            MovementGeneratorKind::Rotate => {
                self.initialize_rotate_like_cpp();
            }
            MovementGeneratorKind::Distract => {
                self.initialize_distract_like_cpp(context.owner_is_standing);
            }
            MovementGeneratorKind::Effect => self.initialize_generic_like_cpp(),
            _ => {
                self.flags &= !(MOVEMENTGENERATOR_FLAG_INITIALIZATION_PENDING
                    | MOVEMENTGENERATOR_FLAG_DEACTIVATED);
                self.flags |= MOVEMENTGENERATOR_FLAG_INITIALIZED;
            }
        }
    }

    pub fn reset_for_motion_master_update_like_cpp(&mut self, context: MotionMasterUpdateContext) {
        match self.kind {
            MovementGeneratorKind::Point => {
                self.reset_point_like_cpp(context.can_move);
            }
            MovementGeneratorKind::Rotate => {
                self.reset_rotate_like_cpp();
            }
            MovementGeneratorKind::Distract => {
                self.reset_distract_like_cpp(context.owner_is_standing);
            }
            _ => self.initialize_for_motion_master_update_like_cpp(context),
        }
    }

    pub fn update_for_motion_master_like_cpp(
        &mut self,
        context: MotionMasterUpdateContext,
    ) -> bool {
        match self.kind {
            MovementGeneratorKind::Idle => self.update_idle_like_cpp(),
            MovementGeneratorKind::Point => {
                self.update_point_like_cpp(context.can_move, context.spline_finalized)
                    != PointMovementAction::Finished
            }
            MovementGeneratorKind::Rotate => {
                self.update_rotate_like_cpp(
                    context.owner_exists,
                    context.diff_ms,
                    context.current_orientation,
                )
                .keep_running
            }
            MovementGeneratorKind::Distract => {
                self.update_distract_like_cpp(context.owner_exists, context.diff_ms)
            }
            MovementGeneratorKind::Effect => self.update_generic_like_cpp(
                context.diff_ms,
                context.spline_cyclic,
                context.spline_finalized,
            ),
            _ => true,
        }
    }

    pub fn initialize_generic_like_cpp(&mut self) {
        if self.has_flag(MOVEMENTGENERATOR_FLAG_DEACTIVATED)
            && !self.has_flag(MOVEMENTGENERATOR_FLAG_INITIALIZATION_PENDING)
        {
            self.flags &= !MOVEMENTGENERATOR_FLAG_DEACTIVATED;
            self.flags |= MOVEMENTGENERATOR_FLAG_FINALIZED;
            return;
        }

        self.flags &=
            !(MOVEMENTGENERATOR_FLAG_INITIALIZATION_PENDING | MOVEMENTGENERATOR_FLAG_DEACTIVATED);
        self.flags |= MOVEMENTGENERATOR_FLAG_INITIALIZED;
        self.elapsed_ms = 0;
    }

    pub fn update_generic_like_cpp(
        &mut self,
        diff_ms: u32,
        spline_cyclic: bool,
        spline_finalized: bool,
    ) -> bool {
        if self.has_flag(MOVEMENTGENERATOR_FLAG_FINALIZED) {
            return false;
        }

        if !spline_cyclic {
            self.elapsed_ms = self.elapsed_ms.saturating_add(diff_ms);
        }

        if self
            .duration_ms
            .is_some_and(|duration_ms| self.elapsed_ms >= duration_ms)
            || spline_finalized
        {
            self.flags |= MOVEMENTGENERATOR_FLAG_INFORM_ENABLED;
            return false;
        }
        true
    }

    pub fn deactivate_generic_like_cpp(&mut self) {
        self.flags |= MOVEMENTGENERATOR_FLAG_DEACTIVATED;
    }

    pub fn finalize_generic_like_cpp(
        &mut self,
        movement_inform: bool,
    ) -> Option<GenericMovementInform> {
        self.flags |= MOVEMENTGENERATOR_FLAG_FINALIZED;
        if movement_inform && self.has_flag(MOVEMENTGENERATOR_FLAG_INFORM_ENABLED) {
            return Some(GenericMovementInform {
                kind: self.kind,
                movement_id: self.movement_id,
                arrival_spell_id: (self.arrival_spell_id != 0).then_some(self.arrival_spell_id),
                arrival_spell_target_guid: (self.arrival_spell_id != 0)
                    .then_some(self.arrival_spell_target_guid),
            });
        }
        None
    }

    pub fn initialize_point_like_cpp(&mut self, can_move: bool) -> PointMovementAction {
        self.flags &= !(MOVEMENTGENERATOR_FLAG_INITIALIZATION_PENDING
            | MOVEMENTGENERATOR_FLAG_TRANSITORY
            | MOVEMENTGENERATOR_FLAG_DEACTIVATED);
        self.flags |= MOVEMENTGENERATOR_FLAG_INITIALIZED;

        if self.movement_id == EVENT_CHARGE_PREPATH {
            return PointMovementAction::MarkRoamingMove;
        }

        if !can_move {
            self.flags |= MOVEMENTGENERATOR_FLAG_INTERRUPTED;
            return PointMovementAction::StopMoving;
        }

        PointMovementAction::LaunchSpline
    }

    pub fn reset_point_like_cpp(&mut self, can_move: bool) -> PointMovementAction {
        self.flags &= !(MOVEMENTGENERATOR_FLAG_TRANSITORY | MOVEMENTGENERATOR_FLAG_DEACTIVATED);
        self.initialize_point_like_cpp(can_move)
    }

    pub fn update_point_like_cpp(
        &mut self,
        can_move: bool,
        spline_finalized: bool,
    ) -> PointMovementAction {
        if self.movement_id == EVENT_CHARGE_PREPATH {
            if spline_finalized {
                self.flags |= MOVEMENTGENERATOR_FLAG_INFORM_ENABLED;
                return PointMovementAction::Finished;
            }
            return PointMovementAction::Continue;
        }

        if !can_move {
            self.flags |= MOVEMENTGENERATOR_FLAG_INTERRUPTED;
            return PointMovementAction::StopMovingAndContinue;
        }

        if (self.has_flag(MOVEMENTGENERATOR_FLAG_INTERRUPTED) && spline_finalized)
            || (self.has_flag(MOVEMENTGENERATOR_FLAG_SPEED_UPDATE_PENDING) && !spline_finalized)
        {
            self.flags &=
                !(MOVEMENTGENERATOR_FLAG_INTERRUPTED | MOVEMENTGENERATOR_FLAG_SPEED_UPDATE_PENDING);
            return PointMovementAction::RelaunchSpline;
        }

        if spline_finalized {
            self.flags &= !MOVEMENTGENERATOR_FLAG_TRANSITORY;
            self.flags |= MOVEMENTGENERATOR_FLAG_INFORM_ENABLED;
            return PointMovementAction::Finished;
        }

        PointMovementAction::Continue
    }

    pub fn deactivate_point_like_cpp(&mut self) -> PointMovementAction {
        self.flags |= MOVEMENTGENERATOR_FLAG_DEACTIVATED;
        PointMovementAction::ClearRoamingMove
    }

    pub fn finalize_point_like_cpp(
        &mut self,
        active: bool,
        movement_inform: bool,
    ) -> PointMovementFinalize {
        self.flags |= MOVEMENTGENERATOR_FLAG_FINALIZED;
        PointMovementFinalize {
            clear_roaming_move: active,
            inform: (movement_inform && self.has_flag(MOVEMENTGENERATOR_FLAG_INFORM_ENABLED))
                .then_some(PointMovementInform {
                    kind: MovementGeneratorKind::Point,
                    movement_id: if self.movement_id == EVENT_CHARGE_PREPATH {
                        EVENT_CHARGE
                    } else {
                        self.movement_id
                    },
                }),
        }
    }

    pub fn finalize_assistance_like_cpp(
        &mut self,
        active: bool,
        movement_inform: bool,
        owner_is_creature: bool,
        owner_is_alive: bool,
    ) -> AssistanceMovementFinalize {
        self.flags |= MOVEMENTGENERATOR_FLAG_FINALIZED;
        let can_inform = movement_inform
            && self.has_flag(MOVEMENTGENERATOR_FLAG_INFORM_ENABLED)
            && owner_is_creature;
        AssistanceMovementFinalize {
            clear_roaming_move: active,
            set_no_call_assistance: can_inform.then_some(false),
            call_assistance: can_inform,
            seek_assistance_distract_ms: (can_inform && owner_is_alive)
                .then_some(CREATURE_FAMILY_ASSISTANCE_DELAY_MS_LIKE_CPP),
        }
    }

    pub fn finalize_assistance_distract_like_cpp(
        &mut self,
        movement_inform: bool,
        owner_is_creature: bool,
    ) -> AssistanceDistractFinalize {
        self.flags |= MOVEMENTGENERATOR_FLAG_FINALIZED;
        AssistanceDistractFinalize {
            set_react_aggressive: movement_inform
                && self.has_flag(MOVEMENTGENERATOR_FLAG_INFORM_ENABLED)
                && owner_is_creature,
        }
    }

    pub fn initialize_idle_like_cpp(&self) -> IdleMovementAction {
        IdleMovementAction::StopMoving
    }

    pub fn reset_idle_like_cpp(&self) -> IdleMovementAction {
        IdleMovementAction::StopMoving
    }

    pub fn update_idle_like_cpp(&self) -> bool {
        true
    }

    pub fn finalize_idle_like_cpp(&mut self) {
        self.flags |= MOVEMENTGENERATOR_FLAG_FINALIZED;
    }

    pub fn initialize_rotate_like_cpp(&mut self) -> IdleMovementAction {
        self.flags &=
            !(MOVEMENTGENERATOR_FLAG_INITIALIZATION_PENDING | MOVEMENTGENERATOR_FLAG_DEACTIVATED);
        self.flags |= MOVEMENTGENERATOR_FLAG_INITIALIZED;
        IdleMovementAction::StopMoving
    }

    pub fn reset_rotate_like_cpp(&mut self) -> IdleMovementAction {
        self.flags &= !MOVEMENTGENERATOR_FLAG_DEACTIVATED;
        self.initialize_rotate_like_cpp()
    }

    pub fn update_rotate_like_cpp(
        &mut self,
        owner_exists: bool,
        diff_ms: u32,
        current_orientation: f32,
    ) -> RotateMovementUpdate {
        if !owner_exists {
            return RotateMovementUpdate {
                keep_running: false,
                facing_angle: None,
            };
        }

        let max_duration_ms = self.max_duration_ms.unwrap_or(0);
        let direction = self.rotate_direction.unwrap_or(RotateDirection::Left);
        let facing_angle = if max_duration_ms == 0 {
            current_orientation
        } else {
            let sign = match direction {
                RotateDirection::Left => 1.0,
                RotateDirection::Right => -1.0,
            };
            (current_orientation
                + (diff_ms as f32 * std::f32::consts::TAU / max_duration_ms as f32) * sign)
                .clamp(0.0, std::f32::consts::TAU)
        };

        let remaining = self.duration_ms.unwrap_or(0);
        if remaining > diff_ms {
            self.duration_ms = Some(remaining - diff_ms);
            RotateMovementUpdate {
                keep_running: true,
                facing_angle: Some(facing_angle),
            }
        } else {
            self.flags |= MOVEMENTGENERATOR_FLAG_INFORM_ENABLED;
            RotateMovementUpdate {
                keep_running: false,
                facing_angle: Some(facing_angle),
            }
        }
    }

    pub fn deactivate_timed_idle_like_cpp(&mut self) {
        self.flags |= MOVEMENTGENERATOR_FLAG_DEACTIVATED;
    }

    pub fn finalize_rotate_like_cpp(
        &mut self,
        movement_inform: bool,
        owner_is_creature: bool,
    ) -> RotateMovementFinalize {
        self.flags |= MOVEMENTGENERATOR_FLAG_FINALIZED;
        RotateMovementFinalize {
            inform: (movement_inform && owner_is_creature).then_some(PointMovementInform {
                kind: MovementGeneratorKind::Rotate,
                movement_id: self.movement_id,
            }),
        }
    }

    pub fn initialize_distract_like_cpp(
        &mut self,
        owner_is_standing: bool,
    ) -> DistractMovementAction {
        self.flags &=
            !(MOVEMENTGENERATOR_FLAG_INITIALIZATION_PENDING | MOVEMENTGENERATOR_FLAG_DEACTIVATED);
        self.flags |= MOVEMENTGENERATOR_FLAG_INITIALIZED;
        DistractMovementAction {
            stand_up: !owner_is_standing,
            launch_facing_spline: true,
        }
    }

    pub fn reset_distract_like_cpp(&mut self, owner_is_standing: bool) -> DistractMovementAction {
        self.flags &= !MOVEMENTGENERATOR_FLAG_DEACTIVATED;
        self.initialize_distract_like_cpp(owner_is_standing)
    }

    pub fn update_distract_like_cpp(&mut self, owner_exists: bool, diff_ms: u32) -> bool {
        if !owner_exists {
            return false;
        }

        let remaining = self.duration_ms.unwrap_or(0);
        if diff_ms > remaining {
            self.flags |= MOVEMENTGENERATOR_FLAG_INFORM_ENABLED;
            return false;
        }

        self.duration_ms = Some(remaining - diff_ms);
        true
    }

    pub fn finalize_distract_like_cpp(
        &mut self,
        movement_inform: bool,
        owner_is_creature: bool,
    ) -> DistractMovementFinalize {
        self.flags |= MOVEMENTGENERATOR_FLAG_FINALIZED;
        DistractMovementFinalize {
            set_home_orientation: movement_inform
                && self.has_flag(MOVEMENTGENERATOR_FLAG_INFORM_ENABLED)
                && owner_is_creature,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GenericMovementInform {
    pub kind: MovementGeneratorKind,
    pub movement_id: u32,
    pub arrival_spell_id: Option<u32>,
    pub arrival_spell_target_guid: Option<ObjectGuid>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PointMovementAction {
    Continue,
    MarkRoamingMove,
    LaunchSpline,
    RelaunchSpline,
    StopMoving,
    StopMovingAndContinue,
    ClearRoamingMove,
    Finished,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PointMovementInform {
    pub kind: MovementGeneratorKind,
    pub movement_id: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PointMovementFinalize {
    pub clear_roaming_move: bool,
    pub inform: Option<PointMovementInform>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AssistanceMovementFinalize {
    pub clear_roaming_move: bool,
    pub set_no_call_assistance: Option<bool>,
    pub call_assistance: bool,
    pub seek_assistance_distract_ms: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdleMovementAction {
    StopMoving,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RotateMovementUpdate {
    pub keep_running: bool,
    pub facing_angle: Option<f32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RotateMovementFinalize {
    pub inform: Option<PointMovementInform>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DistractMovementAction {
    pub stand_up: bool,
    pub launch_facing_spline: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DistractMovementFinalize {
    pub set_home_orientation: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MoveSplineState {
    pub enabled: bool,
    pub finalized: bool,
    pub cyclic: bool,
    pub on_transport: bool,
    pub spline_id: u32,
    pub progress_ms: u32,
    pub duration_ms: u32,
    pub velocity: Option<u32>,
    pub final_destination: Option<(i32, i32, i32)>,
    pub current_destination: Option<(i32, i32, i32)>,
}

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MotionSubsystem {
    pub default_generator: MovementGeneratorRef,
    pub active_generators: Vec<MovementGeneratorRef>,
    pub current_generator: MovementGeneratorKind,
    pub base_unit_states: HashMap<u32, usize>,
    pub flags: u8,
    pub delayed_actions: Vec<MotionMasterDelayedAction>,
    pub paused: bool,
    pub stopped: bool,
    pub spline: MoveSplineState,
}

impl MotionSubsystem {
    pub const fn has_motion_master_flag(&self, flag: u8) -> bool {
        self.flags & flag != 0
    }

    pub const fn should_delay_motion_master_action_like_cpp(&self) -> bool {
        self.has_motion_master_flag(MOTIONMASTER_FLAG_DELAYED)
    }

    pub fn push_delayed_action_like_cpp(&mut self, action_type: MotionMasterDelayedActionType) {
        let payload = match action_type {
            MotionMasterDelayedActionType::Clear => MotionMasterDelayedActionPayload::Clear,
            MotionMasterDelayedActionType::ClearSlot => {
                MotionMasterDelayedActionPayload::ClearSlot(MovementSlot::Active)
            }
            MotionMasterDelayedActionType::ClearMode => {
                MotionMasterDelayedActionPayload::ClearMode(MovementGeneratorMode::Default)
            }
            MotionMasterDelayedActionType::ClearPriority => {
                MotionMasterDelayedActionPayload::ClearPriority(MovementGeneratorPriority::Normal)
            }
            MotionMasterDelayedActionType::Add => MotionMasterDelayedActionPayload::Add(
                MovementGeneratorRef::new(MovementGeneratorKind::Idle, MovementSlot::Active),
            ),
            MotionMasterDelayedActionType::Remove => MotionMasterDelayedActionPayload::Remove {
                kind: MovementGeneratorKind::Idle,
                slot: MovementSlot::Active,
            },
            MotionMasterDelayedActionType::RemoveType => {
                MotionMasterDelayedActionPayload::RemoveType {
                    kind: MovementGeneratorKind::Idle,
                    slot: MovementSlot::Active,
                }
            }
            MotionMasterDelayedActionType::Initialize => {
                MotionMasterDelayedActionPayload::Initialize
            }
        };
        self.push_delayed_payload_like_cpp(payload);
    }

    pub fn push_delayed_action_with_validator_like_cpp(
        &mut self,
        action_type: MotionMasterDelayedActionType,
        validator_passed: bool,
    ) {
        self.delayed_actions
            .push(MotionMasterDelayedAction::with_validator(
                match action_type {
                    MotionMasterDelayedActionType::Clear => MotionMasterDelayedActionPayload::Clear,
                    MotionMasterDelayedActionType::ClearSlot => {
                        MotionMasterDelayedActionPayload::ClearSlot(MovementSlot::Active)
                    }
                    MotionMasterDelayedActionType::ClearMode => {
                        MotionMasterDelayedActionPayload::ClearMode(MovementGeneratorMode::Default)
                    }
                    MotionMasterDelayedActionType::ClearPriority => {
                        MotionMasterDelayedActionPayload::ClearPriority(
                            MovementGeneratorPriority::Normal,
                        )
                    }
                    MotionMasterDelayedActionType::Add => {
                        MotionMasterDelayedActionPayload::Add(MovementGeneratorRef::new(
                            MovementGeneratorKind::Idle,
                            MovementSlot::Active,
                        ))
                    }
                    MotionMasterDelayedActionType::Remove => {
                        MotionMasterDelayedActionPayload::Remove {
                            kind: MovementGeneratorKind::Idle,
                            slot: MovementSlot::Active,
                        }
                    }
                    MotionMasterDelayedActionType::RemoveType => {
                        MotionMasterDelayedActionPayload::RemoveType {
                            kind: MovementGeneratorKind::Idle,
                            slot: MovementSlot::Active,
                        }
                    }
                    MotionMasterDelayedActionType::Initialize => {
                        MotionMasterDelayedActionPayload::Initialize
                    }
                },
                validator_passed,
            ));
    }

    pub fn push_delayed_payload_like_cpp(&mut self, payload: MotionMasterDelayedActionPayload) {
        self.delayed_actions
            .push(MotionMasterDelayedAction::new(payload));
    }

    pub fn push_delayed_payload_with_validator_like_cpp(
        &mut self,
        payload: MotionMasterDelayedActionPayload,
        validator_passed: bool,
    ) {
        self.delayed_actions
            .push(MotionMasterDelayedAction::with_validator(
                payload,
                validator_passed,
            ));
    }

    pub fn resolve_delayed_actions_like_cpp(&mut self) -> Vec<MotionMasterResolvedDelayedAction> {
        self.delayed_actions
            .drain(..)
            .map(|action| MotionMasterResolvedDelayedAction {
                action_type: action.action_type(),
                executed: action.validator_passed,
            })
            .collect()
    }

    pub fn resolve_delayed_action_payloads_like_cpp(
        &mut self,
    ) -> Vec<MotionMasterResolvedDelayedAction> {
        let mut resolved = Vec::new();
        while !self.delayed_actions.is_empty() {
            let action = self.delayed_actions.remove(0);
            if action.validator_passed {
                self.apply_delayed_action_payload_like_cpp(action.payload);
            }
            resolved.push(MotionMasterResolvedDelayedAction {
                action_type: action.action_type(),
                executed: action.validator_passed,
            });
        }
        resolved
    }

    pub fn update_motion_master_like_cpp(
        &mut self,
        context: MotionMasterUpdateContext,
    ) -> MotionMasterUpdateOutcome {
        if self.has_motion_master_flag(
            MOTIONMASTER_FLAG_INITIALIZATION_PENDING | MOTIONMASTER_FLAG_INITIALIZING,
        ) {
            return MotionMasterUpdateOutcome::Stalled;
        }

        if self.is_empty() {
            return MotionMasterUpdateOutcome::Empty;
        }

        self.flags |= MOTIONMASTER_FLAG_UPDATE;

        if self.has_motion_master_flag(MOTIONMASTER_FLAG_STATIC_INITIALIZATION_PENDING)
            && self.current_slot() == MovementSlot::Default
        {
            self.flags &= !MOTIONMASTER_FLAG_STATIC_INITIALIZATION_PENDING;
            self.default_generator
                .initialize_for_motion_master_update_like_cpp(context);
        }

        let keep_running = if self.active_generators.is_empty() {
            initialize_or_reset_for_motion_master_update_like_cpp(
                &mut self.default_generator,
                context,
            );
            self.default_generator
                .update_for_motion_master_like_cpp(context)
        } else {
            let top = &mut self.active_generators[0];
            initialize_or_reset_for_motion_master_update_like_cpp(top, context);
            top.update_for_motion_master_like_cpp(context)
        };

        let popped = if !keep_running && !self.active_generators.is_empty() {
            Some(self.remove_generator_at(0))
        } else {
            None
        };
        self.current_generator = self.current_movement_generator().kind;

        self.flags &= !MOTIONMASTER_FLAG_UPDATE;
        let resolved_delayed_actions = self.resolve_delayed_action_payloads_like_cpp();

        MotionMasterUpdateOutcome::Updated {
            popped,
            resolved_delayed_actions,
        }
    }

    pub fn set_current_generator(&mut self, generator: MovementGeneratorKind) {
        self.add_generator(MovementGeneratorRef::new(generator, MovementSlot::Active));
    }

    pub fn add_to_world(&mut self) {
        let _ = self.add_to_world_like_cpp();
    }

    pub fn add_to_world_like_cpp(&mut self) -> MotionMasterAddToWorldOutcomeLikeCpp {
        let flags_before = self.flags;
        let had_initialization_pending =
            self.has_motion_master_flag(MOTIONMASTER_FLAG_INITIALIZATION_PENDING);

        if !had_initialization_pending {
            return MotionMasterAddToWorldOutcomeLikeCpp {
                had_initialization_pending,
                entered_initializing: false,
                direct_initialize_represented: false,
                resolved_delayed_actions: Vec::new(),
                exited_initializing: false,
                flags_before,
                flags_after: self.flags,
                current_generator_after: self.current_generator,
            };
        }

        self.flags |= MOTIONMASTER_FLAG_INITIALIZING;
        self.flags &= !MOTIONMASTER_FLAG_INITIALIZATION_PENDING;

        self.direct_initialize_like_cpp();
        let resolved_delayed_actions = self.resolve_delayed_action_payloads_like_cpp();

        self.flags &= !MOTIONMASTER_FLAG_INITIALIZING;
        self.current_generator = self.current_movement_generator().kind;

        MotionMasterAddToWorldOutcomeLikeCpp {
            had_initialization_pending,
            entered_initializing: (flags_before & MOTIONMASTER_FLAG_INITIALIZING) == 0,
            direct_initialize_represented: true,
            resolved_delayed_actions,
            exited_initializing: !self.has_motion_master_flag(MOTIONMASTER_FLAG_INITIALIZING),
            flags_before,
            flags_after: self.flags,
            current_generator_after: self.current_generator,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.active_generators.is_empty()
            && self.default_generator.kind == MovementGeneratorKind::Custom(u32::MAX)
    }

    pub fn size(&self) -> usize {
        1 + self.active_generators.len()
    }

    pub fn current_slot(&self) -> MovementSlot {
        if self.active_generators.is_empty() {
            MovementSlot::Default
        } else {
            MovementSlot::Active
        }
    }

    pub fn current_movement_generator(&self) -> MovementGeneratorRef {
        self.active_generators
            .first()
            .copied()
            .unwrap_or(self.default_generator)
    }

    fn movement_generator_for_slot_mut(
        &mut self,
        slot: MovementSlot,
    ) -> Option<&mut MovementGeneratorRef> {
        match slot {
            MovementSlot::Default => Some(&mut self.default_generator),
            MovementSlot::Active => self.active_generators.first_mut(),
        }
    }

    pub fn add_generator(&mut self, mut generator: MovementGeneratorRef) {
        match generator.slot {
            MovementSlot::Default => {
                generator.slot = MovementSlot::Default;
                self.default_generator = generator;
                if generator.kind == MovementGeneratorKind::Idle {
                    self.flags |= MOTIONMASTER_FLAG_STATIC_INITIALIZATION_PENDING;
                }
            }
            MovementSlot::Active => {
                generator.slot = MovementSlot::Active;
                if let Some(top) = self.active_generators.first().copied() {
                    if generator.priority >= top.priority {
                        if generator.priority == top.priority {
                            self.remove_generator_at(0);
                        } else if let Some(top) = self.active_generators.first_mut() {
                            top.flags |= MOVEMENTGENERATOR_FLAG_DEACTIVATED;
                        }
                    } else if let Some(index) = self
                        .active_generators
                        .iter()
                        .position(|known| known.priority == generator.priority)
                    {
                        self.remove_generator_at(index);
                    }
                }

                self.add_base_unit_state(generator.base_unit_state);
                self.active_generators.push(generator);
                self.sort_active_generators();
            }
        }
        self.current_generator = self.current_movement_generator().kind;
        self.stopped = false;
    }

    pub fn remove_generator_kind(
        &mut self,
        kind: MovementGeneratorKind,
        slot: MovementSlot,
    ) -> Option<MovementGeneratorRef> {
        let removed = match slot {
            MovementSlot::Default if self.default_generator.kind == kind => {
                let previous = self.default_generator;
                self.move_idle();
                Some(previous)
            }
            MovementSlot::Default => None,
            MovementSlot::Active => self
                .active_generators
                .iter()
                .position(|generator| generator.kind == kind)
                .map(|index| self.remove_generator_at(index)),
        };
        self.current_generator = self.current_movement_generator().kind;
        removed
    }

    pub fn clear_active(&mut self) -> Vec<MovementGeneratorRef> {
        let removed = std::mem::take(&mut self.active_generators);
        self.base_unit_states.clear();
        self.current_generator = self.default_generator.kind;
        removed
    }

    pub fn clear_slot(&mut self, slot: MovementSlot) -> Vec<MovementGeneratorRef> {
        match slot {
            MovementSlot::Default => {
                let previous = self.default_generator;
                self.move_idle();
                vec![previous]
            }
            MovementSlot::Active => self.clear_active(),
        }
    }

    pub fn clear_by_priority(
        &mut self,
        priority: MovementGeneratorPriority,
    ) -> Vec<MovementGeneratorRef> {
        let mut removed = Vec::new();
        let mut index = 0;
        while index < self.active_generators.len() {
            if self.active_generators[index].priority == priority {
                removed.push(self.remove_generator_at(index));
            } else {
                index += 1;
            }
        }
        self.current_generator = self.current_movement_generator().kind;
        removed
    }

    pub fn clear_by_mode(&mut self, mode: MovementGeneratorMode) -> Vec<MovementGeneratorRef> {
        let mut removed = Vec::new();
        let mut index = 0;
        while index < self.active_generators.len() {
            if self.active_generators[index].mode == mode {
                removed.push(self.remove_generator_at(index));
            } else {
                index += 1;
            }
        }
        self.current_generator = self.current_movement_generator().kind;
        removed
    }

    pub fn direct_initialize_like_cpp(&mut self) {
        let selected_default = self.default_generator.kind;
        self.clear_active();
        self.initialize_default_generator_like_cpp(selected_default);
    }

    fn apply_delayed_action_payload_like_cpp(&mut self, payload: MotionMasterDelayedActionPayload) {
        match payload {
            MotionMasterDelayedActionPayload::Clear => {
                self.clear_active();
            }
            MotionMasterDelayedActionPayload::ClearSlot(slot) => {
                self.clear_slot(slot);
            }
            MotionMasterDelayedActionPayload::ClearMode(mode) => {
                self.clear_by_mode(mode);
            }
            MotionMasterDelayedActionPayload::ClearPriority(priority) => {
                self.clear_by_priority(priority);
            }
            MotionMasterDelayedActionPayload::Add(generator) => {
                self.add_generator(generator);
            }
            MotionMasterDelayedActionPayload::Remove { kind, slot }
            | MotionMasterDelayedActionPayload::RemoveType { kind, slot } => {
                self.remove_generator_kind(kind, slot);
            }
            MotionMasterDelayedActionPayload::Initialize => {
                self.direct_initialize_like_cpp();
            }
        }
    }

    pub fn move_idle(&mut self) {
        self.initialize_default_generator_like_cpp(MovementGeneratorKind::Idle);
        self.flags |= MOTIONMASTER_FLAG_STATIC_INITIALIZATION_PENDING;
        if self.active_generators.is_empty() {
            self.current_generator = MovementGeneratorKind::Idle;
        }
    }

    pub fn initialize_default_generator_like_cpp(&mut self, kind: MovementGeneratorKind) {
        self.default_generator = match kind {
            MovementGeneratorKind::Random | MovementGeneratorKind::Waypoint => {
                MovementGeneratorRef::new(kind, MovementSlot::Default)
                    .with_priority(MovementGeneratorPriority::Normal)
                    .with_flags(MOVEMENTGENERATOR_FLAG_INITIALIZATION_PENDING)
                    .with_base_unit_state(UnitState::ROAMING.bits())
            }
            MovementGeneratorKind::Idle => {
                MovementGeneratorRef::new(MovementGeneratorKind::Idle, MovementSlot::Default)
                    .with_priority(MovementGeneratorPriority::Normal)
                    .with_flags(MOVEMENTGENERATOR_FLAG_INITIALIZED)
            }
            other => MovementGeneratorRef::new(other, MovementSlot::Default)
                .with_priority(MovementGeneratorPriority::Normal)
                .with_flags(MOVEMENTGENERATOR_FLAG_INITIALIZATION_PENDING),
        };
        if self.active_generators.is_empty() {
            self.current_generator = self.default_generator.kind;
        }
    }

    pub fn move_point(&mut self, movement_id: u32) {
        self.add_generator(
            MovementGeneratorRef::new(MovementGeneratorKind::Point, MovementSlot::Active)
                .with_priority(MovementGeneratorPriority::Normal)
                .with_flags(MOVEMENTGENERATOR_FLAG_INITIALIZATION_PENDING)
                .with_base_unit_state(UnitState::ROAMING.bits())
                .with_movement_id(movement_id),
        );
    }

    pub fn move_seek_assistance_like_cpp(&mut self) -> SeekAssistancePlan {
        self.add_generator(
            MovementGeneratorRef::new(MovementGeneratorKind::Assistance, MovementSlot::Active)
                .with_priority(MovementGeneratorPriority::Normal)
                .with_flags(MOVEMENTGENERATOR_FLAG_INITIALIZATION_PENDING)
                .with_base_unit_state(UnitState::ROAMING.bits())
                .with_movement_id(EVENT_ASSIST_MOVE),
        );
        SeekAssistancePlan {
            attack_stop: true,
            cast_stop: true,
            do_not_reacquire_spell_focus_target: true,
            set_react_passive: true,
            generator_added: true,
        }
    }

    pub fn move_seek_assistance_distract_like_cpp(&mut self, timer_ms: u32) {
        self.add_generator(
            MovementGeneratorRef::new(
                MovementGeneratorKind::AssistanceDistract,
                MovementSlot::Active,
            )
            .with_priority(MovementGeneratorPriority::Normal)
            .with_flags(MOVEMENTGENERATOR_FLAG_INITIALIZATION_PENDING)
            .with_base_unit_state(UnitState::DISTRACTED.bits())
            .with_duration_ms(timer_ms),
        );
    }

    pub fn move_distract_like_cpp(&mut self, timer_ms: u32) {
        self.add_generator(
            MovementGeneratorRef::new(MovementGeneratorKind::Distract, MovementSlot::Active)
                .with_priority(MovementGeneratorPriority::Highest)
                .with_flags(MOVEMENTGENERATOR_FLAG_INITIALIZATION_PENDING)
                .with_base_unit_state(UnitState::DISTRACTED.bits())
                .with_duration_ms(timer_ms),
        );
    }

    pub fn move_rotate_like_cpp(
        &mut self,
        movement_id: u32,
        time_ms: u32,
        direction: RotateDirection,
    ) -> bool {
        if time_ms == 0 {
            return false;
        }

        self.add_generator(
            MovementGeneratorRef::new(MovementGeneratorKind::Rotate, MovementSlot::Active)
                .with_priority(MovementGeneratorPriority::Normal)
                .with_flags(MOVEMENTGENERATOR_FLAG_INITIALIZATION_PENDING)
                .with_base_unit_state(UnitState::ROTATING.bits())
                .with_movement_id(movement_id)
                .with_duration_ms(time_ms)
                .with_max_duration_ms(time_ms)
                .with_rotate_direction(direction),
        );
        true
    }

    pub fn move_charge(&mut self, movement_id: u32) {
        self.add_generator(
            MovementGeneratorRef::new(MovementGeneratorKind::Point, MovementSlot::Active)
                .with_priority(MovementGeneratorPriority::Highest)
                .with_flags(MOVEMENTGENERATOR_FLAG_INITIALIZATION_PENDING)
                .with_base_unit_state(UnitState::CHARGING.bits())
                .with_movement_id(movement_id),
        );
    }

    pub fn move_follow(&mut self, target_guid: ObjectGuid, duration_ms: Option<u32>) {
        let mut generator =
            MovementGeneratorRef::new(MovementGeneratorKind::Follow, MovementSlot::Active)
                .with_priority(MovementGeneratorPriority::Normal)
                .with_target_guid(target_guid);
        if let Some(duration_ms) = duration_ms {
            generator = generator.with_duration_ms(duration_ms);
        }
        self.add_generator(generator);
    }

    pub fn move_chase_like_cpp(&mut self, target_guid: ObjectGuid) {
        self.add_generator(
            MovementGeneratorRef::new(MovementGeneratorKind::Chase, MovementSlot::Active)
                .with_priority(MovementGeneratorPriority::Normal)
                .with_flags(MOVEMENTGENERATOR_FLAG_INITIALIZATION_PENDING)
                .with_base_unit_state(UnitState::CHASE.bits())
                .with_target_guid(target_guid),
        );
    }

    pub fn launch_generic_movement(
        &mut self,
        kind: MovementGeneratorKind,
        movement_id: u32,
        duration_ms: u32,
        arrival_spell: Option<(u32, ObjectGuid)>,
    ) {
        self.add_generic_movement(
            kind,
            movement_id,
            duration_ms,
            MovementGeneratorPriority::Normal,
            UnitState::ROAMING.bits(),
            MOVEMENTGENERATOR_FLAG_INITIALIZATION_PENDING,
            arrival_spell,
        );
    }

    pub fn launch_move_spline_like_cpp(
        &mut self,
        kind: MovementGeneratorKind,
        movement_id: u32,
        priority: MovementGeneratorPriority,
        duration_ms: u32,
    ) -> bool {
        let trinity_type = kind.trinity_id();
        if trinity_type == 3 || trinity_type >= 19 {
            return false;
        }

        self.add_generic_movement(
            kind,
            movement_id,
            duration_ms,
            priority,
            UnitState::ROAMING.bits(),
            MOVEMENTGENERATOR_FLAG_INITIALIZATION_PENDING,
            None,
        );
        true
    }

    pub fn move_jump_like_cpp(
        &mut self,
        movement_id: u32,
        duration_ms: u32,
        speed_xy: f32,
        arrival_spell: Option<(u32, ObjectGuid)>,
    ) -> bool {
        if speed_xy < 0.01 {
            return false;
        }

        self.add_generic_movement(
            MovementGeneratorKind::Effect,
            movement_id,
            duration_ms,
            MovementGeneratorPriority::Highest,
            UnitState::JUMPING.bits(),
            MOVEMENTGENERATOR_FLAG_INITIALIZATION_PENDING,
            arrival_spell,
        );
        true
    }

    pub fn move_jump_with_gravity_like_cpp(
        &mut self,
        movement_id: u32,
        duration_ms: u32,
        speed_xy: f32,
        arrival_spell: Option<(u32, ObjectGuid)>,
    ) -> bool {
        if speed_xy < 0.01 {
            return false;
        }

        self.add_generic_movement(
            MovementGeneratorKind::Effect,
            movement_id,
            duration_ms,
            MovementGeneratorPriority::Highest,
            UnitState::JUMPING.bits(),
            MOVEMENTGENERATOR_FLAG_INITIALIZATION_PENDING | MOVEMENTGENERATOR_FLAG_PERSIST_ON_DEATH,
            arrival_spell,
        );
        true
    }

    pub fn move_knockback_from_like_cpp(
        &mut self,
        is_player: bool,
        duration_ms: u32,
        speed_xy: f32,
    ) -> bool {
        if is_player || speed_xy < 0.01 {
            return false;
        }

        self.add_generic_movement(
            MovementGeneratorKind::Effect,
            0,
            duration_ms,
            MovementGeneratorPriority::Highest,
            0,
            MOVEMENTGENERATOR_FLAG_INITIALIZATION_PENDING | MOVEMENTGENERATOR_FLAG_PERSIST_ON_DEATH,
            None,
        );
        true
    }

    pub fn move_fall_like_cpp(
        &mut self,
        movement_id: u32,
        duration_ms: u32,
        has_valid_ground_height: bool,
        vertical_delta: f32,
        has_root_or_stun_state: bool,
        is_player: bool,
    ) -> MoveFallPlan {
        if !has_valid_ground_height || vertical_delta.abs() < 0.1 || has_root_or_stun_state {
            return MoveFallPlan::Noop;
        }

        if is_player {
            return MoveFallPlan::PlayerFallInfo;
        }

        self.add_generic_movement(
            MovementGeneratorKind::Effect,
            movement_id,
            duration_ms,
            MovementGeneratorPriority::Highest,
            0,
            MOVEMENTGENERATOR_FLAG_INITIALIZATION_PENDING,
            None,
        );
        MoveFallPlan::SplineStarted
    }

    fn add_generic_movement(
        &mut self,
        kind: MovementGeneratorKind,
        movement_id: u32,
        duration_ms: u32,
        priority: MovementGeneratorPriority,
        base_unit_state: u32,
        flags: u16,
        arrival_spell: Option<(u32, ObjectGuid)>,
    ) {
        let mut generator = MovementGeneratorRef::new(kind, MovementSlot::Active)
            .with_priority(priority)
            .with_flags(flags)
            .with_base_unit_state(base_unit_state)
            .with_movement_id(movement_id)
            .with_duration_ms(duration_ms);
        if let Some((spell_id, target_guid)) = arrival_spell {
            generator = generator.with_arrival_spell(spell_id, target_guid);
        }
        self.add_generator(generator);
    }

    pub fn stop_on_death(&mut self) -> bool {
        if self
            .active_generators
            .first()
            .is_some_and(|generator| generator.has_flag(MOVEMENTGENERATOR_FLAG_PERSIST_ON_DEATH))
        {
            return false;
        }

        self.clear_active();
        self.move_idle();
        self.stop_moving();
        true
    }

    pub fn pause_movement(&mut self) {
        self.paused = true;
    }

    pub fn pause_current_movement_like_cpp(
        &mut self,
        timer_ms: u32,
        slot: MovementSlot,
        forced: bool,
    ) -> bool {
        let Some(generator) = self.movement_generator_for_slot_mut(slot) else {
            return false;
        };

        if timer_ms > 0 {
            generator.flags |= MOVEMENTGENERATOR_FLAG_TIMED_PAUSED;
            generator.flags &= !MOVEMENTGENERATOR_FLAG_PAUSED;
            generator.duration_ms = Some(timer_ms);
            generator.elapsed_ms = 0;
        } else {
            generator.flags |= MOVEMENTGENERATOR_FLAG_PAUSED;
            generator.flags &= !MOVEMENTGENERATOR_FLAG_TIMED_PAUSED;
        }

        self.paused = true;
        if forced && self.current_slot() == slot {
            self.stop_moving();
        }
        true
    }

    pub fn resume_movement(&mut self) {
        self.paused = false;
    }

    pub fn stop_moving(&mut self) {
        self.stopped = true;
        self.finalize_spline();
    }

    pub fn start_spline(&mut self, spline_id: u32, duration_ms: u32) {
        self.spline = MoveSplineState {
            enabled: true,
            finalized: false,
            cyclic: false,
            on_transport: false,
            spline_id,
            progress_ms: 0,
            duration_ms,
            velocity: None,
            final_destination: None,
            current_destination: None,
        };
        self.stopped = false;
    }

    pub fn launch_spline(
        &mut self,
        spline_id: u32,
        duration_ms: u32,
        destination: (i32, i32, i32),
        cyclic: bool,
        on_transport: bool,
        velocity: Option<u32>,
    ) {
        self.spline = MoveSplineState {
            enabled: true,
            finalized: false,
            cyclic,
            on_transport,
            spline_id,
            progress_ms: 0,
            duration_ms,
            velocity,
            final_destination: Some(destination),
            current_destination: Some(destination),
        };
        self.stopped = false;
    }

    pub fn set_spline_progress(&mut self, progress_ms: u32) {
        self.spline.progress_ms = progress_ms.min(self.spline.duration_ms);
        if self.spline.progress_ms >= self.spline.duration_ms && !self.spline.cyclic {
            self.finalize_spline();
        }
    }

    pub fn update_spline(&mut self, diff_ms: u32) -> bool {
        if !self.spline.enabled || self.spline.finalized {
            return false;
        }
        let next_progress = self.spline.progress_ms.saturating_add(diff_ms);
        if self.spline.cyclic && self.spline.duration_ms > 0 {
            self.spline.progress_ms = next_progress % self.spline.duration_ms;
            return false;
        }
        self.set_spline_progress(next_progress);
        self.spline.finalized
    }

    pub fn finalize_spline(&mut self) {
        self.spline.enabled = false;
        self.spline.finalized = true;
        self.spline.progress_ms = self.spline.duration_ms;
    }

    pub fn interrupt_spline(&mut self) {
        self.finalize_spline();
        self.spline.current_destination = None;
    }

    fn sort_active_generators(&mut self) {
        self.active_generators.sort_by(|left, right| {
            right
                .mode
                .cmp(&left.mode)
                .then_with(|| right.priority.cmp(&left.priority))
        });
    }

    fn remove_generator_at(&mut self, index: usize) -> MovementGeneratorRef {
        let removed = self.active_generators.remove(index);
        self.clear_base_unit_state(removed.base_unit_state);
        removed
    }

    fn add_base_unit_state(&mut self, base_unit_state: u32) {
        if base_unit_state != 0 {
            *self.base_unit_states.entry(base_unit_state).or_insert(0) += 1;
        }
    }

    fn clear_base_unit_state(&mut self, base_unit_state: u32) {
        if base_unit_state == 0 {
            return;
        }
        if let Some(count) = self.base_unit_states.get_mut(&base_unit_state) {
            *count = count.saturating_sub(1);
            if *count == 0 {
                self.base_unit_states.remove(&base_unit_state);
            }
        }
    }
}
