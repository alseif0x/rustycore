use wow_core::Position;

use crate::{
    MovementGenerator, MovementGeneratorFlags, MovementGeneratorMode, MovementGeneratorPriority,
    MovementGeneratorState, MovementGeneratorType, normalize_orientation_like_cpp,
};

pub const RANDOM_PATH_LENGTH_LIMIT_LIKE_CPP: f32 = 30.0;
pub const RANDOM_LOS_RETRY_MS_LIKE_CPP: i32 = 200;
pub const RANDOM_PATH_RETRY_MS_LIKE_CPP: i32 = 100;
pub const RANDOM_PAUSE_MIN_MS_LIKE_CPP: i32 = 4_000;
pub const RANDOM_PAUSE_MAX_MS_LIKE_CPP: i32 = 10_000;
pub const RANDOM_WANDER_STEPS_MIN_LIKE_CPP: u8 = 2;
pub const RANDOM_WANDER_STEPS_MAX_LIKE_CPP: u8 = 10;
pub const UNIT_STATE_RANDOM_ROAMING_LIKE_CPP: u32 = 0x0000_0010;
pub const UNIT_STATE_RANDOM_ROAMING_MOVE_LIKE_CPP: u32 = 0x0080_0000;
pub const UNIT_STATE_RANDOM_NOT_MOVE_LIKE_CPP: u32 = 0x0000_0409;
pub const UNIT_STATE_RANDOM_LOST_CONTROL_LIKE_CPP: u32 = 0x0007_008c;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RandomUnitSnapshot {
    pub owner_position: Position,
    pub owner_alive: bool,
    pub owner_unit_state: u32,
    pub movement_prevented_by_casting: bool,
    pub move_spline_finalized: bool,
    pub owner_wander_distance: f32,
    pub has_los_to_destination: bool,
    pub path_result: RandomPathResult,
    pub movement_template: CreatureRandomMovementType,
    pub owner_is_walking: bool,
    pub travel_time_ms: i32,
    pub distance_roll: f32,
    pub angle_roll: f32,
    pub next_wander_steps_roll: u8,
    pub pause_seconds_roll: i32,
    pub ai_enabled: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RandomPathResult {
    Success,
    Failed,
    NoPath,
    Shortcut,
    FarFromPoly,
}

impl RandomPathResult {
    #[must_use]
    pub const fn is_usable_like_cpp(self) -> bool {
        !matches!(self, Self::Failed | Self::NoPath | Self::Shortcut)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CreatureRandomMovementType {
    #[default]
    AlwaysWalk,
    CanRun,
    AlwaysRun,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RandomDestinationPlan {
    pub reference: Position,
    pub distance: f32,
    pub angle: f32,
    pub destination: Position,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RandomLaunchPlan {
    pub destination: Position,
    pub path_length_limit: f32,
    pub walk: bool,
    pub timer_ms: i32,
    pub signal_formation: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RandomMovementAction {
    Continue,
    Finished,
    StopMoving,
    DurationFinished,
    RetryAfterLosFailure {
        timer_ms: i32,
    },
    RetryAfterPathFailure {
        timer_ms: i32,
        result: RandomPathResult,
    },
    Launch(RandomLaunchPlan),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RandomFinalizeAction {
    pub clear_roaming_move: bool,
    pub stop_moving: bool,
    pub set_walk_false: bool,
    pub inform: Option<RandomMovementInform>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RandomMovementInform {
    pub movement_type: MovementGeneratorType,
    pub point_id: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RandomMovementGenerator {
    state: MovementGeneratorState,
    timer_ms: i32,
    duration_ms: Option<i32>,
    reference: Position,
    wander_distance: f32,
    wander_steps: u8,
    path_allocated: bool,
    last_destination_plan: Option<RandomDestinationPlan>,
    pub stop_moving_calls: u32,
    pub signal_formation_calls: u32,
    pub finalize_action: Option<RandomFinalizeAction>,
}

impl RandomMovementGenerator {
    #[must_use]
    pub fn new(wander_distance: f32, duration_ms: Option<i32>) -> Self {
        Self {
            state: MovementGeneratorState {
                mode: MovementGeneratorMode::Default,
                priority: MovementGeneratorPriority::Normal,
                flags: MovementGeneratorFlags::INITIALIZATION_PENDING,
                base_unit_state: UNIT_STATE_RANDOM_ROAMING_LIKE_CPP,
            },
            timer_ms: 0,
            duration_ms,
            reference: Position::new(0.0, 0.0, 0.0, 0.0),
            wander_distance,
            wander_steps: 0,
            path_allocated: false,
            last_destination_plan: None,
            stop_moving_calls: 0,
            signal_formation_calls: 0,
            finalize_action: None,
        }
    }

    #[must_use]
    pub const fn timer_ms(&self) -> i32 {
        self.timer_ms
    }

    #[must_use]
    pub const fn duration_ms(&self) -> Option<i32> {
        self.duration_ms
    }

    #[must_use]
    pub const fn reference(&self) -> Position {
        self.reference
    }

    #[must_use]
    pub const fn wander_distance(&self) -> f32 {
        self.wander_distance
    }

    #[must_use]
    pub const fn wander_steps(&self) -> u8 {
        self.wander_steps
    }

    #[must_use]
    pub const fn path_allocated(&self) -> bool {
        self.path_allocated
    }

    #[must_use]
    pub const fn last_destination_plan(&self) -> Option<RandomDestinationPlan> {
        self.last_destination_plan
    }

    pub fn pause_like_cpp(&mut self, timer_ms: u32) {
        if timer_ms > 0 {
            self.add_flag(MovementGeneratorFlags::TIMED_PAUSED);
            self.timer_ms = timer_ms as i32;
            self.remove_flag(MovementGeneratorFlags::PAUSED);
        } else {
            self.add_flag(MovementGeneratorFlags::PAUSED);
            self.remove_flag(MovementGeneratorFlags::TIMED_PAUSED);
        }
    }

    pub fn resume_like_cpp(&mut self, override_timer_ms: u32) {
        if override_timer_ms > 0 {
            self.timer_ms = override_timer_ms as i32;
        }
        self.remove_flag(MovementGeneratorFlags::PAUSED);
    }

    pub fn initialize_like_cpp(
        &mut self,
        owner_exists: bool,
        snapshot: RandomUnitSnapshot,
    ) -> RandomMovementAction {
        self.remove_flag(
            MovementGeneratorFlags::INITIALIZATION_PENDING
                | MovementGeneratorFlags::TRANSITORY
                | MovementGeneratorFlags::DEACTIVATED
                | MovementGeneratorFlags::TIMED_PAUSED,
        );
        self.add_flag(MovementGeneratorFlags::INITIALIZED);

        if !owner_exists || !snapshot.owner_alive {
            return RandomMovementAction::Continue;
        }

        self.reference = snapshot.owner_position;
        self.stop_moving_calls = self.stop_moving_calls.saturating_add(1);
        if self.wander_distance == 0.0 {
            self.wander_distance = snapshot.owner_wander_distance;
        }
        self.wander_steps = clamp_wander_steps(snapshot.next_wander_steps_roll);
        self.timer_ms = 0;
        self.path_allocated = false;
        RandomMovementAction::StopMoving
    }

    pub fn reset_like_cpp(
        &mut self,
        owner_exists: bool,
        snapshot: RandomUnitSnapshot,
    ) -> RandomMovementAction {
        self.remove_flag(MovementGeneratorFlags::TRANSITORY | MovementGeneratorFlags::DEACTIVATED);
        self.initialize_like_cpp(owner_exists, snapshot)
    }

    pub fn update_like_cpp(
        &mut self,
        owner_exists: bool,
        diff_ms: u32,
        snapshot: RandomUnitSnapshot,
    ) -> RandomMovementAction {
        if !owner_exists || !snapshot.owner_alive {
            return RandomMovementAction::Continue;
        }

        if self.has_flag(MovementGeneratorFlags::FINALIZED | MovementGeneratorFlags::PAUSED) {
            return RandomMovementAction::Continue;
        }

        if let Some(duration) = self.duration_ms {
            let remaining = duration.saturating_sub(diff_ms as i32);
            self.duration_ms = Some(remaining);
            if remaining <= 0 {
                self.remove_flag(MovementGeneratorFlags::TRANSITORY);
                self.add_flag(MovementGeneratorFlags::INFORM_ENABLED);
                return RandomMovementAction::DurationFinished;
            }
        }

        if snapshot.owner_unit_state & UNIT_STATE_RANDOM_NOT_MOVE_LIKE_CPP != 0
            || snapshot.movement_prevented_by_casting
        {
            self.add_flag(MovementGeneratorFlags::INTERRUPTED);
            self.stop_moving_calls = self.stop_moving_calls.saturating_add(1);
            self.path_allocated = false;
            return RandomMovementAction::StopMoving;
        }

        self.remove_flag(MovementGeneratorFlags::INTERRUPTED);
        self.timer_ms = self.timer_ms.saturating_sub(diff_ms as i32);
        let speed_update_pending = self.has_flag(MovementGeneratorFlags::SPEED_UPDATE_PENDING);
        if (speed_update_pending && !snapshot.move_spline_finalized)
            || (self.timer_ms <= 0 && snapshot.move_spline_finalized)
        {
            return self.set_random_location_like_cpp(snapshot);
        }

        RandomMovementAction::Continue
    }

    pub fn set_random_location_like_cpp(
        &mut self,
        snapshot: RandomUnitSnapshot,
    ) -> RandomMovementAction {
        if snapshot.owner_unit_state
            & (UNIT_STATE_RANDOM_NOT_MOVE_LIKE_CPP | UNIT_STATE_RANDOM_LOST_CONTROL_LIKE_CPP)
            != 0
            || snapshot.movement_prevented_by_casting
        {
            self.add_flag(MovementGeneratorFlags::INTERRUPTED);
            self.stop_moving_calls = self.stop_moving_calls.saturating_add(1);
            self.path_allocated = false;
            return RandomMovementAction::StopMoving;
        }

        let destination_plan = compute_random_destination_like_cpp(
            self.reference,
            self.wander_distance,
            snapshot.distance_roll,
            snapshot.angle_roll,
        );
        self.last_destination_plan = Some(destination_plan);

        if !snapshot.has_los_to_destination {
            self.timer_ms = RANDOM_LOS_RETRY_MS_LIKE_CPP;
            return RandomMovementAction::RetryAfterLosFailure {
                timer_ms: self.timer_ms,
            };
        }

        self.path_allocated = true;
        if !snapshot.path_result.is_usable_like_cpp() {
            self.timer_ms = RANDOM_PATH_RETRY_MS_LIKE_CPP;
            return RandomMovementAction::RetryAfterPathFailure {
                timer_ms: self.timer_ms,
                result: snapshot.path_result,
            };
        }

        self.remove_flag(MovementGeneratorFlags::TRANSITORY | MovementGeneratorFlags::TIMED_PAUSED);
        let walk = random_walk_like_cpp(snapshot.movement_template, snapshot.owner_is_walking);
        self.wander_steps = self.wander_steps.saturating_sub(1);
        let timer_ms = if self.wander_steps > 0 {
            snapshot.travel_time_ms
        } else {
            self.wander_steps = clamp_wander_steps(snapshot.next_wander_steps_roll);
            snapshot.travel_time_ms + clamp_pause_seconds(snapshot.pause_seconds_roll) * 1_000
        };
        self.timer_ms = timer_ms;
        self.signal_formation_calls = self.signal_formation_calls.saturating_add(1);

        RandomMovementAction::Launch(RandomLaunchPlan {
            destination: destination_plan.destination,
            path_length_limit: RANDOM_PATH_LENGTH_LIMIT_LIKE_CPP,
            walk,
            timer_ms,
            signal_formation: true,
        })
    }

    pub fn adjust_launch_timer_for_actual_travel_time_like_cpp(
        &mut self,
        planned_travel_time_ms: i32,
        actual_travel_time_ms: i32,
    ) {
        let pause_ms = self.timer_ms.saturating_sub(planned_travel_time_ms);
        self.timer_ms = actual_travel_time_ms.saturating_add(pause_ms);
    }

    pub fn deactivate_like_cpp(&mut self) -> RandomFinalizeAction {
        self.add_flag(MovementGeneratorFlags::DEACTIVATED);
        RandomFinalizeAction {
            clear_roaming_move: true,
            stop_moving: false,
            set_walk_false: false,
            inform: None,
        }
    }

    pub fn finalize_like_cpp(
        &mut self,
        active: bool,
        movement_inform: bool,
        ai_enabled: bool,
    ) -> RandomFinalizeAction {
        self.add_flag(MovementGeneratorFlags::FINALIZED);
        let action = RandomFinalizeAction {
            clear_roaming_move: active,
            stop_moving: active,
            set_walk_false: active,
            inform: (movement_inform
                && ai_enabled
                && self.has_flag(MovementGeneratorFlags::INFORM_ENABLED))
            .then_some(RandomMovementInform {
                movement_type: MovementGeneratorType::Random,
                point_id: 0,
            }),
        };
        if active {
            self.stop_moving_calls = self.stop_moving_calls.saturating_add(1);
        }
        self.finalize_action = Some(action);
        action
    }
}

impl Default for RandomMovementGenerator {
    fn default() -> Self {
        Self::new(0.0, None)
    }
}

impl MovementGenerator for RandomMovementGenerator {
    fn state(&self) -> &MovementGeneratorState {
        &self.state
    }

    fn state_mut(&mut self) -> &mut MovementGeneratorState {
        &mut self.state
    }

    fn kind(&self) -> MovementGeneratorType {
        MovementGeneratorType::Random
    }

    fn initialize(&mut self) {
        self.remove_flag(
            MovementGeneratorFlags::INITIALIZATION_PENDING
                | MovementGeneratorFlags::TRANSITORY
                | MovementGeneratorFlags::DEACTIVATED
                | MovementGeneratorFlags::TIMED_PAUSED,
        );
        self.add_flag(MovementGeneratorFlags::INITIALIZED);
    }

    fn reset(&mut self) {
        self.remove_flag(MovementGeneratorFlags::TRANSITORY | MovementGeneratorFlags::DEACTIVATED);
        self.initialize();
    }

    fn update(&mut self, _diff_ms: u32) -> bool {
        true
    }

    fn deactivate(&mut self) {
        self.deactivate_like_cpp();
    }

    fn finalize(&mut self, active: bool, movement_inform: bool) {
        self.finalize_like_cpp(active, movement_inform, false);
    }

    fn unit_speed_changed(&mut self) {
        self.add_flag(MovementGeneratorFlags::SPEED_UPDATE_PENDING);
    }

    fn pause(&mut self, timer_ms: u32) {
        self.pause_like_cpp(timer_ms);
    }

    fn resume(&mut self, override_timer_ms: u32) {
        self.resume_like_cpp(override_timer_ms);
    }
}

#[must_use]
pub fn compute_random_destination_like_cpp(
    reference: Position,
    wander_distance: f32,
    distance_roll: f32,
    angle_roll: f32,
) -> RandomDestinationPlan {
    let distance = distance_roll * wander_distance;
    let angle = normalize_orientation_like_cpp(angle_roll * std::f32::consts::PI * 2.0);
    let destination = Position::new(
        reference.x + distance * angle.cos(),
        reference.y + distance * angle.sin(),
        reference.z,
        reference.orientation,
    );

    RandomDestinationPlan {
        reference,
        distance,
        angle,
        destination,
    }
}

#[must_use]
pub const fn random_walk_like_cpp(
    random_type: CreatureRandomMovementType,
    owner_is_walking: bool,
) -> bool {
    match random_type {
        CreatureRandomMovementType::CanRun => owner_is_walking,
        CreatureRandomMovementType::AlwaysRun => false,
        CreatureRandomMovementType::AlwaysWalk => true,
    }
}

fn clamp_wander_steps(value: u8) -> u8 {
    value.clamp(
        RANDOM_WANDER_STEPS_MIN_LIKE_CPP,
        RANDOM_WANDER_STEPS_MAX_LIKE_CPP,
    )
}

fn clamp_pause_seconds(value: i32) -> i32 {
    value.clamp(
        RANDOM_PAUSE_MIN_MS_LIKE_CPP / 1_000,
        RANDOM_PAUSE_MAX_MS_LIKE_CPP / 1_000,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snapshot(owner: Position) -> RandomUnitSnapshot {
        RandomUnitSnapshot {
            owner_position: owner,
            owner_alive: true,
            owner_unit_state: 0,
            movement_prevented_by_casting: false,
            move_spline_finalized: true,
            owner_wander_distance: 7.0,
            has_los_to_destination: true,
            path_result: RandomPathResult::Success,
            movement_template: CreatureRandomMovementType::AlwaysWalk,
            owner_is_walking: true,
            travel_time_ms: 300,
            distance_roll: 1.0,
            angle_roll: 0.0,
            next_wander_steps_roll: 3,
            pause_seconds_roll: 4,
            ai_enabled: true,
        }
    }

    #[test]
    fn random_constructor_and_initialize_match_cpp_shape() {
        let owner = Position::new(10.0, 20.0, 30.0, 1.5);
        let mut random = RandomMovementGenerator::new(0.0, Some(10_000));

        assert_eq!(random.kind(), MovementGeneratorType::Random);
        assert_eq!(random.state().mode, MovementGeneratorMode::Default);
        assert_eq!(random.state().priority, MovementGeneratorPriority::Normal);
        assert_eq!(
            random.state().flags,
            MovementGeneratorFlags::INITIALIZATION_PENDING
        );
        assert_eq!(
            random.state().base_unit_state,
            UNIT_STATE_RANDOM_ROAMING_LIKE_CPP
        );

        assert_eq!(
            random.initialize_like_cpp(true, snapshot(owner)),
            RandomMovementAction::StopMoving
        );
        assert!(random.has_flag(MovementGeneratorFlags::INITIALIZED));
        assert!(!random.has_flag(MovementGeneratorFlags::INITIALIZATION_PENDING));
        assert_eq!(random.reference(), owner);
        assert_eq!(random.wander_distance(), 7.0);
        assert_eq!(random.wander_steps(), 3);
        assert_eq!(random.timer_ms(), 0);
        assert!(!random.path_allocated());
        assert_eq!(random.stop_moving_calls, 1);
    }

    #[test]
    fn random_pause_and_resume_match_cpp_flags() {
        let mut random = RandomMovementGenerator::new(5.0, None);
        random.pause_like_cpp(500);
        assert!(random.has_flag(MovementGeneratorFlags::TIMED_PAUSED));
        assert!(!random.has_flag(MovementGeneratorFlags::PAUSED));
        assert_eq!(random.timer_ms(), 500);

        random.pause_like_cpp(0);
        assert!(random.has_flag(MovementGeneratorFlags::PAUSED));
        assert!(!random.has_flag(MovementGeneratorFlags::TIMED_PAUSED));

        random.resume_like_cpp(250);
        assert!(!random.has_flag(MovementGeneratorFlags::PAUSED));
        assert_eq!(random.timer_ms(), 250);
    }

    #[test]
    fn random_destination_and_walk_rules_match_cpp_shape() {
        let reference = Position::new(10.0, 20.0, 30.0, 0.0);
        let dest = compute_random_destination_like_cpp(reference, 8.0, 0.5, 0.25);
        assert_eq!(dest.distance, 4.0);
        assert!((dest.angle - std::f32::consts::FRAC_PI_2).abs() < f32::EPSILON);
        assert!((dest.destination.x - 10.0).abs() < 0.0001);
        assert!((dest.destination.y - 24.0).abs() < 0.0001);

        assert!(random_walk_like_cpp(
            CreatureRandomMovementType::AlwaysWalk,
            false
        ));
        assert!(random_walk_like_cpp(
            CreatureRandomMovementType::CanRun,
            true
        ));
        assert!(!random_walk_like_cpp(
            CreatureRandomMovementType::CanRun,
            false
        ));
        assert!(!random_walk_like_cpp(
            CreatureRandomMovementType::AlwaysRun,
            true
        ));
    }

    #[test]
    fn random_update_duration_pause_and_blocked_rules_match_cpp() {
        let owner = Position::new(0.0, 0.0, 0.0, 0.0);
        let mut random = RandomMovementGenerator::new(5.0, Some(100));
        random.initialize_like_cpp(true, snapshot(owner));
        assert_eq!(
            random.update_like_cpp(true, 100, snapshot(owner)),
            RandomMovementAction::DurationFinished
        );
        assert!(random.has_flag(MovementGeneratorFlags::INFORM_ENABLED));

        let mut random = RandomMovementGenerator::new(5.0, None);
        random.initialize_like_cpp(true, snapshot(owner));
        random.pause_like_cpp(0);
        assert_eq!(
            random.update_like_cpp(true, 1, snapshot(owner)),
            RandomMovementAction::Continue
        );

        let mut random = RandomMovementGenerator::new(5.0, None);
        random.initialize_like_cpp(true, snapshot(owner));
        let mut blocked = snapshot(owner);
        blocked.owner_unit_state = UNIT_STATE_RANDOM_NOT_MOVE_LIKE_CPP;
        assert_eq!(
            random.update_like_cpp(true, 1, blocked),
            RandomMovementAction::StopMoving
        );
        assert!(random.has_flag(MovementGeneratorFlags::INTERRUPTED));
        assert_eq!(random.stop_moving_calls, 2);
    }

    #[test]
    fn random_set_location_retries_los_and_path_failures_like_cpp() {
        let owner = Position::new(0.0, 0.0, 0.0, 0.0);
        let mut random = RandomMovementGenerator::new(5.0, None);
        random.initialize_like_cpp(true, snapshot(owner));

        let mut no_los = snapshot(owner);
        no_los.has_los_to_destination = false;
        assert_eq!(
            random.set_random_location_like_cpp(no_los),
            RandomMovementAction::RetryAfterLosFailure {
                timer_ms: RANDOM_LOS_RETRY_MS_LIKE_CPP,
            }
        );
        assert_eq!(random.timer_ms(), RANDOM_LOS_RETRY_MS_LIKE_CPP);
        assert!(!random.path_allocated());

        let mut far_from_poly = snapshot(owner);
        far_from_poly.path_result = RandomPathResult::FarFromPoly;
        assert!(matches!(
            random.set_random_location_like_cpp(far_from_poly),
            RandomMovementAction::Launch(_)
        ));

        let mut shortcut = snapshot(owner);
        shortcut.path_result = RandomPathResult::Shortcut;
        assert_eq!(
            random.set_random_location_like_cpp(shortcut),
            RandomMovementAction::RetryAfterPathFailure {
                timer_ms: RANDOM_PATH_RETRY_MS_LIKE_CPP,
                result: RandomPathResult::Shortcut,
            }
        );
    }

    #[test]
    fn random_launch_counts_steps_and_pause_like_cpp() {
        let owner = Position::new(0.0, 0.0, 0.0, 0.0);
        let mut random = RandomMovementGenerator::new(5.0, None);
        let mut snap = snapshot(owner);
        snap.next_wander_steps_roll = 2;
        random.initialize_like_cpp(true, snap);

        let action = random.set_random_location_like_cpp(snapshot(owner));
        assert_eq!(
            action,
            RandomMovementAction::Launch(RandomLaunchPlan {
                destination: Position::new(5.0, 0.0, 0.0, 0.0),
                path_length_limit: RANDOM_PATH_LENGTH_LIMIT_LIKE_CPP,
                walk: true,
                timer_ms: 300,
                signal_formation: true,
            })
        );
        assert_eq!(random.wander_steps(), 1);
        assert_eq!(random.signal_formation_calls, 1);

        let action = random.set_random_location_like_cpp(snapshot(owner));
        assert_eq!(
            action,
            RandomMovementAction::Launch(RandomLaunchPlan {
                destination: Position::new(5.0, 0.0, 0.0, 0.0),
                path_length_limit: RANDOM_PATH_LENGTH_LIMIT_LIKE_CPP,
                walk: true,
                timer_ms: 4_300,
                signal_formation: true,
            })
        );
        assert_eq!(random.wander_steps(), 3);
    }

    #[test]
    fn random_deactivate_and_finalize_match_cpp() {
        let mut random = RandomMovementGenerator::new(5.0, None);
        assert_eq!(
            random.deactivate_like_cpp(),
            RandomFinalizeAction {
                clear_roaming_move: true,
                stop_moving: false,
                set_walk_false: false,
                inform: None,
            }
        );

        random.add_flag(MovementGeneratorFlags::INFORM_ENABLED);
        assert_eq!(
            random.finalize_like_cpp(true, true, true),
            RandomFinalizeAction {
                clear_roaming_move: true,
                stop_moving: true,
                set_walk_false: true,
                inform: Some(RandomMovementInform {
                    movement_type: MovementGeneratorType::Random,
                    point_id: 0,
                }),
            }
        );
    }
}
