//! Init packets.
//!
//! Separated from spline.rs under #693.

use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum MonsterMoveType {
    #[default]
    Normal = 0,
    FacingSpot = 1,
    FacingTarget = 2,
    FacingAngle = 3,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FacingInfo {
    pub spot: Position,
    pub target: ObjectGuid,
    pub angle: f32,
    pub kind: MonsterMoveType,
}

impl Default for FacingInfo {
    fn default() -> Self {
        Self {
            spot: Position::ZERO,
            target: ObjectGuid::EMPTY,
            angle: 0.0,
            kind: MonsterMoveType::Normal,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SpellEffectExtraData {
    pub target: ObjectGuid,
    pub spell_visual_id: u32,
    pub progress_curve_id: u32,
    pub parabolic_curve_id: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct AnimTierTransition {
    pub tier_transition_id: u32,
    pub anim_tier: u8,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct MonsterMovePathData {
    pub points: Vec<Position>,
    pub packed_deltas: Vec<[f32; 3]>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MoveSplineInitArgs {
    pub path: Vec<Position>,
    pub facing: FacingInfo,
    pub flags: MoveSplineFlag,
    pub path_idx_offset: i32,
    pub velocity: f32,
    pub parabolic_amplitude: f32,
    pub vertical_acceleration: f32,
    pub effect_start_time_percent: f32,
    pub effect_start_time_ms: i32,
    pub spline_id: u32,
    pub initial_orientation: f32,
    pub spell_effect_extra: Option<SpellEffectExtraData>,
    pub anim_tier: Option<AnimTierTransition>,
    pub walk: bool,
    pub has_velocity: bool,
    pub transform_for_transport: bool,
}

impl Default for MoveSplineInitArgs {
    fn default() -> Self {
        Self {
            path: Vec::with_capacity(16),
            facing: FacingInfo::default(),
            flags: MoveSplineFlag::SMOOTH_GROUND_PATH,
            path_idx_offset: 0,
            velocity: 0.0,
            parabolic_amplitude: 0.0,
            vertical_acceleration: 0.0,
            effect_start_time_percent: 0.0,
            effect_start_time_ms: 0,
            spline_id: 0,
            initial_orientation: 0.0,
            spell_effect_extra: None,
            anim_tier: None,
            walk: false,
            has_velocity: false,
            transform_for_transport: true,
        }
    }
}

impl MoveSplineInitArgs {
    #[must_use]
    pub fn with_capacity(path_capacity: usize) -> Self {
        Self {
            path: Vec::with_capacity(path_capacity),
            ..Self::default()
        }
    }

    pub fn validate(&self) -> Result<(), MoveSplineValidationError> {
        if self.path.len() <= 1 {
            return Err(MoveSplineValidationError::PathTooShort);
        }
        if self.velocity < 0.01 {
            return Err(MoveSplineValidationError::VelocityTooLow);
        }
        if !(0.0..=1.0).contains(&self.effect_start_time_percent) {
            return Err(MoveSplineValidationError::EffectStartTimePercentOutOfRange);
        }
        self.check_path_lengths()?;
        Ok(())
    }

    fn check_path_lengths(&self) -> Result<(), MoveSplineValidationError> {
        if self.path.len() > 2 || self.facing.kind == MonsterMoveType::Normal {
            for pair in self.path.windows(2) {
                if distance_3d(pair[0], pair[1]) < 0.1 {
                    return Err(MoveSplineValidationError::PathSegmentTooShort);
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MoveSplineLaunchInput {
    pub current_position: Position,
    pub active_spline_position: Option<Position>,
    pub movement_flags: MovementFlag,
    pub selected_speed: f32,
    pub run_speed: f32,
    pub assistance_speed_factor: f32,
    pub on_transport: bool,
}

impl MoveSplineLaunchInput {
    #[must_use]
    pub const fn new(current_position: Position) -> Self {
        Self {
            current_position,
            active_spline_position: None,
            movement_flags: MovementFlag::NONE,
            selected_speed: 0.0,
            run_speed: 0.0,
            assistance_speed_factor: 1.0,
            on_transport: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MoveSplineLaunchResult {
    pub real_position: Position,
    pub movement_flags: MovementFlag,
    pub duration_ms: i32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MoveSplineStopInput {
    pub current_position: Position,
    pub active_spline_position: Option<Position>,
    pub on_transport: bool,
}

impl MoveSplineStopInput {
    #[must_use]
    pub const fn new(current_position: Position) -> Self {
        Self {
            current_position,
            active_spline_position: None,
            on_transport: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MoveSplineStopResult {
    pub position: Position,
    pub spline_id: u32,
    pub stop_distance_tolerance: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoveSplineLaunchError {
    EmptyPath,
    Validation(MoveSplineValidationError),
}

impl From<MoveSplineValidationError> for MoveSplineLaunchError {
    fn from(value: MoveSplineValidationError) -> Self {
        Self::Validation(value)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MoveSplineInit {
    pub args: MoveSplineInitArgs,
}

impl MoveSplineInit {
    #[must_use]
    pub fn new(spline_id: u32) -> Self {
        Self {
            args: MoveSplineInitArgs {
                spline_id,
                transform_for_transport: false,
                flags: MoveSplineFlag::SMOOTH_GROUND_PATH,
                ..MoveSplineInitArgs::default()
            },
        }
    }

    pub fn move_by_path<I>(&mut self, controls: I, path_offset: i32)
    where
        I: IntoIterator<Item = Position>,
    {
        self.args.path_idx_offset = path_offset;
        self.args.path.clear();
        self.args.path.extend(controls);
    }

    pub fn move_to(&mut self, destination: Position) {
        self.args.path_idx_offset = 0;
        self.args.path.resize(2, Position::new(0.0, 0.0, 0.0, 0.0));
        self.args.path[1] = destination;
    }

    pub fn set_first_point_id(&mut self, point_id: i32) {
        self.args.path_idx_offset = point_id;
    }

    pub fn set_velocity(&mut self, velocity: f32) {
        self.args.velocity = velocity;
        self.args.has_velocity = true;
    }

    pub fn set_walk(&mut self, enable: bool) {
        self.args.walk = enable;
    }

    pub fn set_smooth(&mut self) {
        self.args.flags.enable_catmull_rom();
    }

    pub fn set_uncompressed(&mut self) {
        self.args.flags.insert(MoveSplineFlag::UNCOMPRESSED_PATH);
    }

    pub fn set_cyclic(&mut self) {
        self.args.flags.insert(MoveSplineFlag::CYCLIC);
    }

    pub fn set_fly(&mut self) {
        self.args.flags.enable_flying();
    }

    pub fn set_transport_enter(&mut self) {
        self.args.flags.enable_transport_enter();
    }

    pub fn set_transport_exit(&mut self) {
        self.args.flags.enable_transport_exit();
    }

    pub fn set_backward(&mut self) {
        self.args.flags.insert(MoveSplineFlag::BACKWARD);
    }

    pub fn set_unlimited_speed(&mut self) {
        self.args.flags.insert(MoveSplineFlag::UNLIMITED_SPEED);
    }

    pub fn set_orientation_fixed(&mut self, enable: bool) {
        self.args
            .flags
            .set(MoveSplineFlag::ORIENTATION_FIXED, enable);
    }

    pub fn set_fall(&mut self, falling_slow: bool) {
        self.args.flags.enable_falling();
        self.args
            .flags
            .set(MoveSplineFlag::FALLING_SLOW, falling_slow);
    }

    pub fn set_parabolic(&mut self, amplitude: f32, time_shift: f32) {
        self.args.effect_start_time_percent = time_shift;
        self.args.parabolic_amplitude = amplitude;
        self.args.vertical_acceleration = 0.0;
        self.args.flags.enable_parabolic();
    }

    pub fn set_parabolic_vertical_acceleration(
        &mut self,
        vertical_acceleration: f32,
        time_shift: f32,
    ) {
        self.args.effect_start_time_percent = time_shift;
        self.args.parabolic_amplitude = 0.0;
        self.args.vertical_acceleration = vertical_acceleration;
        self.args.flags.enable_parabolic();
    }

    pub fn set_animation(
        &mut self,
        anim_tier: u8,
        tier_transition_id: u32,
        transition_start_time_ms: i32,
    ) {
        self.args.effect_start_time_percent = 0.0;
        self.args.effect_start_time_ms = transition_start_time_ms;
        self.args.anim_tier = Some(AnimTierTransition {
            tier_transition_id,
            anim_tier,
        });
        self.args.flags.enable_animation();
    }

    pub fn set_facing_spot(&mut self, spot: Position) {
        self.args.facing.spot = spot;
        self.args.facing.kind = MonsterMoveType::FacingSpot;
    }

    pub fn set_facing_target_with_angle(&mut self, target: ObjectGuid, absolute_angle: f32) {
        self.args.facing.angle = absolute_angle;
        self.args.facing.target = target;
        self.args.facing.kind = MonsterMoveType::FacingTarget;
    }

    pub fn set_facing_angle(&mut self, angle: f32) {
        self.args.facing.angle = wrap_angle_0_2pi(angle);
        self.args.facing.kind = MonsterMoveType::FacingAngle;
    }

    pub fn set_spell_effect_extra_data(&mut self, spell_effect_extra: SpellEffectExtraData) {
        self.args.spell_effect_extra = Some(spell_effect_extra);
    }

    pub fn disable_transport_path_transformations(&mut self) {
        self.args.transform_for_transport = false;
    }

    pub fn launch(
        &mut self,
        move_spline: &mut MoveSpline,
        input: MoveSplineLaunchInput,
    ) -> Result<MoveSplineLaunchResult, MoveSplineLaunchError> {
        let real_position = input
            .active_spline_position
            .unwrap_or(input.current_position);

        if self.args.path.is_empty() {
            return Err(MoveSplineLaunchError::EmptyPath);
        }

        self.args.path[0] = real_position;
        self.args.initial_orientation = real_position.orientation;
        self.args.flags.set(
            MoveSplineFlag::ENTER_CYCLE,
            self.args.flags.contains(MoveSplineFlag::CYCLIC),
        );
        move_spline.on_transport = input.on_transport;

        let mut movement_flags = input.movement_flags;
        if self.args.flags.contains(MoveSplineFlag::BACKWARD) {
            movement_flags.remove(MovementFlag::FORWARD);
            movement_flags.insert(MovementFlag::BACKWARD);
        } else {
            movement_flags.remove(MovementFlag::BACKWARD);
            movement_flags.insert(MovementFlag::FORWARD);
        }

        if movement_flags.contains(MovementFlag::ROOT) {
            movement_flags.remove(MovementFlag::MASK_MOVING);
        }

        if !self.args.has_velocity {
            self.args.velocity = input.selected_speed * input.assistance_speed_factor;
        }

        self.args.velocity = self.args.velocity.min(self.speed_limit(input.run_speed));
        move_spline.initialize(&self.args)?;

        Ok(MoveSplineLaunchResult {
            real_position,
            movement_flags,
            duration_ms: move_spline.duration_ms(),
        })
    }

    pub fn stop(
        &mut self,
        move_spline: &mut MoveSpline,
        input: MoveSplineStopInput,
    ) -> Option<MoveSplineStopResult> {
        if move_spline.finalized() {
            return None;
        }

        let position = input
            .active_spline_position
            .unwrap_or(input.current_position);
        self.args.flags = MoveSplineFlag::DONE;
        move_spline.on_transport = input.on_transport;
        if move_spline.initialize(&self.args).is_err() {
            return None;
        }

        Some(MoveSplineStopResult {
            position,
            spline_id: move_spline.id(),
            stop_distance_tolerance: 2,
        })
    }

    fn speed_limit(&self, run_speed: f32) -> f32 {
        if self.args.flags.contains(MoveSplineFlag::UNLIMITED_SPEED) {
            return f32::MAX;
        }

        if self.args.flags.intersects(
            MoveSplineFlag::FALLING
                | MoveSplineFlag::CATMULLROM
                | MoveSplineFlag::FLYING
                | MoveSplineFlag::PARABOLIC,
        ) {
            return 50.0;
        }

        28.0_f32.max(run_speed * 4.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoveSplineValidationError {
    PathTooShort,
    VelocityTooLow,
    EffectStartTimePercentOutOfRange,
    PathSegmentTooShort,
}
