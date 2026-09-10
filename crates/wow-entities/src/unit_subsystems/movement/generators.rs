//! Generators packets.
//!
//! Separated from movement.rs under #693.

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
