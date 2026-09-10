//! Actions packets.
//!
//! Separated from movement.rs under #693.

use super::*;

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
