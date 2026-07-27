use wow_core::{ObjectGuid, Position};

use crate::{
    AbstractFollower, CONTACT_DISTANCE_LIKE_CPP, ChaseAngle, ChaseRange, MovementGenerator,
    MovementGeneratorFlags, MovementGeneratorMode, MovementGeneratorPriority,
    MovementGeneratorState, MovementGeneratorType, normalize_orientation_like_cpp,
};

pub const CHASE_RANGE_CHECK_INTERVAL_MS_LIKE_CPP: i32 = 100;
pub const UNIT_STATE_CHASE_LIKE_CPP: u32 = 0x0000_0020;
pub const UNIT_STATE_CHASE_MOVE_LIKE_CPP: u32 = 0x0400_0000;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ChaseUnitSnapshot {
    pub owner_position: Position,
    pub target_position: Position,
    pub owner_combat_reach: f32,
    pub target_combat_reach: f32,
    pub owner_melee_range: f32,
    pub owner_alive: bool,
    pub target_in_world: bool,
    pub can_move: bool,
    pub movement_prevented_by_casting: bool,
    pub owner_victim_is_target: bool,
    pub owner_has_chase_move: bool,
    pub owner_movespline_finalized: bool,
    pub mutual_chase: bool,
    pub owner_has_los: bool,
    pub target_accessible: bool,
    pub owner_can_fly: bool,
    pub owner_is_creature: bool,
    pub creature_is_pet: bool,
    pub creature_chase_walk: ChaseWalkMode,
    pub owner_is_walking: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ChaseWalkMode {
    #[default]
    Default,
    CanWalk,
    AlwaysWalk,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ChaseRangeBounds {
    pub min_range: f32,
    pub min_target: f32,
    pub max_range: f32,
    pub max_target: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ChaseLaunchPlan {
    pub move_toward: bool,
    /// C++ replaces its owned `PathGenerator` before calculating when the
    /// chase direction flips. The runtime bridge uses this to discard the
    /// retained Detour corridor before it builds the next query.
    pub direction_changed: bool,
    pub desired_distance: f32,
    pub desired_relative_angle: Option<f32>,
    pub shorten_path: bool,
    pub allow_flying_path: bool,
    pub walk: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChaseMovementInform {
    pub movement_type: MovementGeneratorType,
    pub target_counter: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ChaseMovementAction {
    Continue,
    Finished,
    StopMoving,
    StopMovingAndFaceInform(ChaseMovementInform),
    ClearChaseMoveAndFaceInform(ChaseMovementInform),
    CannotReachTarget,
    Launch(ChaseLaunchPlan),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChaseFinalizeAction {
    pub clear_chase_move: bool,
    pub clear_cannot_reach_target: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ChaseMovementGenerator {
    state: MovementGeneratorState,
    follower: AbstractFollower,
    range: Option<ChaseRange>,
    angle: Option<ChaseAngle>,
    range_check_timer_ms: i32,
    moving_towards: bool,
    mutual_chase: bool,
    last_target_position: Option<Position>,
    pub stop_moving_calls: u32,
    pub cannot_reach_target: bool,
    pub finalize_action: Option<ChaseFinalizeAction>,
}

impl ChaseMovementGenerator {
    #[must_use]
    pub const fn new(
        target: ObjectGuid,
        range: Option<ChaseRange>,
        angle: Option<ChaseAngle>,
    ) -> Self {
        Self {
            state: MovementGeneratorState {
                mode: MovementGeneratorMode::Default,
                priority: MovementGeneratorPriority::Normal,
                flags: MovementGeneratorFlags::INITIALIZATION_PENDING,
                base_unit_state: UNIT_STATE_CHASE_LIKE_CPP,
            },
            follower: AbstractFollower::new(target),
            range,
            angle,
            range_check_timer_ms: CHASE_RANGE_CHECK_INTERVAL_MS_LIKE_CPP,
            moving_towards: true,
            mutual_chase: true,
            last_target_position: None,
            stop_moving_calls: 0,
            cannot_reach_target: false,
            finalize_action: None,
        }
    }

    #[must_use]
    pub const fn target(&self) -> Option<ObjectGuid> {
        self.follower.target()
    }

    #[must_use]
    pub const fn range(&self) -> Option<ChaseRange> {
        self.range
    }

    #[must_use]
    pub const fn angle(&self) -> Option<ChaseAngle> {
        self.angle
    }

    #[must_use]
    pub const fn range_check_timer_ms(&self) -> i32 {
        self.range_check_timer_ms
    }

    #[must_use]
    pub const fn moving_towards(&self) -> bool {
        self.moving_towards
    }

    #[must_use]
    pub const fn mutual_chase(&self) -> bool {
        self.mutual_chase
    }

    /// Commits the direction chosen by a launch plan only after the runtime
    /// bridge has actually launched the corresponding spline.
    ///
    /// TrinityCore never assigns `_movingTowards` after calculating
    /// `moveToward`; keeping that omission makes its periodic range check use
    /// the wrong bound after a move-away launch. Rust intentionally repairs
    /// that legacy defect, but does so transactionally so a failed path query
    /// cannot commit a direction the creature never started moving in.
    pub fn confirm_launch_like_cpp(&mut self, plan: ChaseLaunchPlan) {
        self.moving_towards = plan.move_toward;
    }

    /// Publishes the C++ effects that occur only after `CalculatePath`
    /// produced a launchable, non-`PATHFIND_NOPATH` path.
    ///
    /// This is deliberately separate from planning: a failed path preserves
    /// the previous `INFORM_ENABLED` state and keeps `CannotReachTarget` set.
    /// It is also separate from [`Self::confirm_launch_like_cpp`] because C++
    /// applies these effects immediately before `MoveSplineInit::Launch`.
    pub fn confirm_path_ready_like_cpp(&mut self) {
        self.cannot_reach_target = false;
        self.add_flag(MovementGeneratorFlags::INFORM_ENABLED);
    }

    pub fn initialize_like_cpp(&mut self) {
        self.remove_flag(
            MovementGeneratorFlags::INITIALIZATION_PENDING | MovementGeneratorFlags::DEACTIVATED,
        );
        self.add_flag(MovementGeneratorFlags::INITIALIZED | MovementGeneratorFlags::INFORM_ENABLED);
        self.last_target_position = None;
        // A reset has no live spline whose direction can be retained.
        self.moving_towards = true;
    }

    pub fn reset_like_cpp(&mut self) {
        self.remove_flag(MovementGeneratorFlags::DEACTIVATED);
        self.initialize_like_cpp();
    }

    pub fn update_like_cpp(
        &mut self,
        owner_exists: bool,
        target_exists: bool,
        diff_ms: u32,
        snapshot: ChaseUnitSnapshot,
    ) -> ChaseMovementAction {
        if !owner_exists || !snapshot.owner_alive {
            return ChaseMovementAction::Finished;
        }

        let Some(target) = self.target() else {
            return ChaseMovementAction::Finished;
        };
        if !target_exists || !snapshot.target_in_world {
            return ChaseMovementAction::Finished;
        }

        if !snapshot.can_move
            || snapshot.movement_prevented_by_casting
            || !snapshot.owner_victim_is_target
        {
            self.stop_moving_calls = self.stop_moving_calls.saturating_add(1);
            self.last_target_position = None;
            self.cannot_reach_target = false;
            return ChaseMovementAction::StopMoving;
        }

        let bounds = self.bounds_like_cpp(snapshot);
        let angle = (!snapshot.mutual_chase).then_some(self.angle).flatten();

        self.range_check_timer_ms = self.range_check_timer_ms.saturating_sub(diff_ms as i32);
        if self.range_check_timer_ms <= 0 {
            self.range_check_timer_ms = CHASE_RANGE_CHECK_INTERVAL_MS_LIKE_CPP;
            let min_distance = (!self.moving_towards).then_some(bounds.min_target);
            let max_distance = self.moving_towards.then_some(bounds.max_target);
            if self.has_flag(MovementGeneratorFlags::INFORM_ENABLED)
                && position_okay_like_cpp(snapshot, min_distance, max_distance, angle)
            {
                self.remove_flag(MovementGeneratorFlags::INFORM_ENABLED);
                self.stop_moving_calls = self.stop_moving_calls.saturating_add(1);
                self.last_target_position = None;
                self.cannot_reach_target = false;
                return ChaseMovementAction::StopMovingAndFaceInform(self.inform_for(target));
            }
        }

        if snapshot.owner_has_chase_move && snapshot.owner_movespline_finalized {
            self.remove_flag(MovementGeneratorFlags::INFORM_ENABLED);
            self.last_target_position = None;
            self.cannot_reach_target = false;
            return ChaseMovementAction::ClearChaseMoveAndFaceInform(self.inform_for(target));
        }

        let target_changed = self
            .last_target_position
            .is_none_or(|last| position_distance_sq(last, snapshot.target_position) > 0.0)
            || snapshot.mutual_chase != self.mutual_chase;

        if target_changed {
            self.last_target_position = Some(snapshot.target_position);
            self.mutual_chase = snapshot.mutual_chase;
            if snapshot.owner_has_chase_move
                || !position_okay_like_cpp(
                    snapshot,
                    Some(bounds.min_range),
                    Some(bounds.max_range),
                    angle,
                )
            {
                if snapshot.owner_is_creature && !snapshot.target_accessible {
                    self.cannot_reach_target = true;
                    self.stop_moving_calls = self.stop_moving_calls.saturating_add(1);
                    self.last_target_position = None;
                    return ChaseMovementAction::CannotReachTarget;
                }

                let move_toward = !is_in_distance_like_cpp(
                    snapshot.owner_position,
                    snapshot.target_position,
                    bounds.max_range,
                );

                return ChaseMovementAction::Launch(ChaseLaunchPlan {
                    move_toward,
                    direction_changed: move_toward != self.moving_towards,
                    desired_distance: if move_toward {
                        bounds.max_target
                    } else {
                        bounds.min_target
                    },
                    desired_relative_angle: angle.map(|angle| angle.relative_angle),
                    shorten_path: move_toward && angle.is_none(),
                    allow_flying_path: snapshot.owner_can_fly,
                    walk: walk_like_cpp(snapshot),
                });
            }
        }

        ChaseMovementAction::Continue
    }

    pub fn deactivate_like_cpp(&mut self) -> ChaseFinalizeAction {
        self.add_flag(MovementGeneratorFlags::DEACTIVATED);
        self.remove_flag(
            MovementGeneratorFlags::TRANSITORY | MovementGeneratorFlags::INFORM_ENABLED,
        );
        self.cannot_reach_target = false;
        ChaseFinalizeAction {
            clear_chase_move: true,
            clear_cannot_reach_target: true,
        }
    }

    pub fn finalize_like_cpp(&mut self, active: bool) -> ChaseFinalizeAction {
        self.add_flag(MovementGeneratorFlags::FINALIZED);
        let action = ChaseFinalizeAction {
            clear_chase_move: active,
            clear_cannot_reach_target: active,
        };
        if active {
            self.cannot_reach_target = false;
        }
        self.finalize_action = Some(action);
        action
    }

    #[must_use]
    pub fn bounds_like_cpp(&self, snapshot: ChaseUnitSnapshot) -> ChaseRangeBounds {
        let hitbox_sum = snapshot.owner_combat_reach + snapshot.target_combat_reach;
        ChaseRangeBounds {
            min_range: self.range.map_or(CONTACT_DISTANCE_LIKE_CPP, |range| {
                range.min_range + hitbox_sum
            }),
            min_target: self
                .range
                .map_or(hitbox_sum, |range| range.min_tolerance + hitbox_sum),
            max_range: self.range.map_or(snapshot.owner_melee_range, |range| {
                range.max_range + hitbox_sum
            }),
            max_target: self
                .range
                .map_or(CONTACT_DISTANCE_LIKE_CPP + hitbox_sum, |range| {
                    range.max_tolerance + hitbox_sum
                }),
        }
    }

    fn inform_for(&self, target: ObjectGuid) -> ChaseMovementInform {
        ChaseMovementInform {
            movement_type: MovementGeneratorType::Chase,
            target_counter: target.counter() as u32,
        }
    }
}

impl MovementGenerator for ChaseMovementGenerator {
    fn state(&self) -> &MovementGeneratorState {
        &self.state
    }

    fn state_mut(&mut self) -> &mut MovementGeneratorState {
        &mut self.state
    }

    fn kind(&self) -> MovementGeneratorType {
        MovementGeneratorType::Chase
    }

    fn initialize(&mut self) {
        self.initialize_like_cpp();
    }

    fn reset(&mut self) {
        self.reset_like_cpp();
    }

    fn update(&mut self, _diff_ms: u32) -> bool {
        true
    }

    fn deactivate(&mut self) {
        self.deactivate_like_cpp();
    }

    fn finalize(&mut self, active: bool, _movement_inform: bool) {
        self.finalize_like_cpp(active);
    }

    fn unit_speed_changed(&mut self) {
        self.last_target_position = None;
    }
}

#[must_use]
pub fn position_okay_like_cpp(
    snapshot: ChaseUnitSnapshot,
    min_distance: Option<f32>,
    max_distance: Option<f32>,
    angle: Option<ChaseAngle>,
) -> bool {
    let dist_sq = position_distance_sq(snapshot.owner_position, snapshot.target_position);
    if min_distance.is_some_and(|min| dist_sq < min * min) {
        return false;
    }
    if max_distance.is_some_and(|max| dist_sq > max * max) {
        return false;
    }
    if angle.is_some_and(|angle| {
        !angle.is_angle_okay(relative_angle_like_cpp(
            snapshot.target_position,
            snapshot.owner_position,
        ))
    }) {
        return false;
    }
    snapshot.owner_has_los
}

fn walk_like_cpp(snapshot: ChaseUnitSnapshot) -> bool {
    if snapshot.owner_is_creature && !snapshot.creature_is_pet {
        match snapshot.creature_chase_walk {
            ChaseWalkMode::CanWalk => snapshot.owner_is_walking,
            ChaseWalkMode::AlwaysWalk => true,
            ChaseWalkMode::Default => false,
        }
    } else {
        false
    }
}

fn is_in_distance_like_cpp(
    owner_position: Position,
    target_position: Position,
    range: f32,
) -> bool {
    position_distance_sq(owner_position, target_position) <= range * range
}

fn relative_angle_like_cpp(from: Position, to: Position) -> f32 {
    normalize_orientation_like_cpp((to.y - from.y).atan2(to.x - from.x) - from.orientation)
}

fn position_distance_sq(left: Position, right: Position) -> f32 {
    let dx = left.x - right.x;
    let dy = left.y - right.y;
    let dz = left.z - right.z;
    dx * dx + dy * dy + dz * dz
}

#[cfg(test)]
mod tests {
    use super::*;

    fn guid(counter: i64) -> ObjectGuid {
        ObjectGuid::create_uniq(counter)
    }

    fn snapshot(owner: Position, target: Position) -> ChaseUnitSnapshot {
        ChaseUnitSnapshot {
            owner_position: owner,
            target_position: target,
            owner_combat_reach: 0.5,
            target_combat_reach: 0.5,
            owner_melee_range: 5.0,
            owner_alive: true,
            target_in_world: true,
            can_move: true,
            movement_prevented_by_casting: false,
            owner_victim_is_target: true,
            owner_has_chase_move: false,
            owner_movespline_finalized: false,
            mutual_chase: false,
            owner_has_los: true,
            target_accessible: true,
            owner_can_fly: false,
            owner_is_creature: true,
            creature_is_pet: false,
            creature_chase_walk: ChaseWalkMode::Default,
            owner_is_walking: false,
        }
    }

    #[test]
    fn chase_constructor_and_initialize_match_cpp_shape() {
        let mut chase = ChaseMovementGenerator::new(
            guid(7),
            Some(ChaseRange::between(2.0, 6.0)),
            Some(ChaseAngle::with_tolerance(0.0, 0.5)),
        );
        assert_eq!(chase.kind(), MovementGeneratorType::Chase);
        assert_eq!(chase.target(), Some(guid(7)));
        assert_eq!(chase.state().mode, MovementGeneratorMode::Default);
        assert_eq!(chase.state().priority, MovementGeneratorPriority::Normal);
        assert_eq!(
            chase.state().flags,
            MovementGeneratorFlags::INITIALIZATION_PENDING
        );
        assert_eq!(chase.state().base_unit_state, UNIT_STATE_CHASE_LIKE_CPP);
        assert_eq!(
            chase.range_check_timer_ms(),
            CHASE_RANGE_CHECK_INTERVAL_MS_LIKE_CPP
        );
        assert!(chase.moving_towards());
        assert!(chase.mutual_chase());

        chase.initialize_like_cpp();
        assert!(chase.has_flag(MovementGeneratorFlags::INITIALIZED));
        assert!(chase.has_flag(MovementGeneratorFlags::INFORM_ENABLED));
        assert!(!chase.has_flag(MovementGeneratorFlags::INITIALIZATION_PENDING));
    }

    #[test]
    fn chase_bounds_and_position_okay_match_cpp_range_shape() {
        let chase = ChaseMovementGenerator::new(
            guid(7),
            Some(ChaseRange::between(2.0, 6.0)),
            Some(ChaseAngle::with_tolerance(0.0, 0.5)),
        );
        let snap = snapshot(
            Position::new(3.0, 0.0, 0.0, 0.0),
            Position::new(0.0, 0.0, 0.0, 0.0),
        );
        let bounds = chase.bounds_like_cpp(snap);
        assert_eq!(
            bounds,
            ChaseRangeBounds {
                min_range: 3.0,
                min_target: 3.5,
                max_range: 7.0,
                max_target: 6.5,
            }
        );
        assert!(position_okay_like_cpp(
            snap,
            Some(bounds.min_range),
            Some(bounds.max_range),
            chase.angle()
        ));

        let mut no_los = snap;
        no_los.owner_has_los = false;
        assert!(!position_okay_like_cpp(
            no_los,
            Some(bounds.min_range),
            Some(bounds.max_range),
            chase.angle()
        ));
    }

    #[test]
    fn chase_update_stops_when_in_range_and_informs_like_cpp() {
        let mut chase = ChaseMovementGenerator::new(
            guid(7),
            Some(ChaseRange::between(2.0, 6.0)),
            Some(ChaseAngle::with_tolerance(0.0, 0.5)),
        );
        chase.initialize_like_cpp();
        let action = chase.update_like_cpp(
            true,
            true,
            100,
            snapshot(
                Position::new(3.0, 0.0, 0.0, 0.0),
                Position::new(0.0, 0.0, 0.0, 0.0),
            ),
        );
        assert_eq!(
            action,
            ChaseMovementAction::StopMovingAndFaceInform(ChaseMovementInform {
                movement_type: MovementGeneratorType::Chase,
                target_counter: 7,
            })
        );
        assert_eq!(chase.stop_moving_calls, 1);
        assert!(!chase.has_flag(MovementGeneratorFlags::INFORM_ENABLED));
    }

    #[test]
    fn chase_update_launches_when_out_of_range_and_uses_walk_rules_like_cpp() {
        let mut chase = ChaseMovementGenerator::new(
            guid(7),
            Some(ChaseRange::between(2.0, 6.0)),
            Some(ChaseAngle::with_tolerance(0.0, 0.5)),
        );
        chase.remove_flag(MovementGeneratorFlags::INFORM_ENABLED);
        chase.cannot_reach_target = true;
        let mut snap = snapshot(
            Position::new(10.0, 0.0, 0.0, 0.0),
            Position::new(0.0, 0.0, 0.0, 0.0),
        );
        snap.creature_chase_walk = ChaseWalkMode::AlwaysWalk;
        let action = chase.update_like_cpp(true, true, 1, snap);
        assert_eq!(
            action,
            ChaseMovementAction::Launch(ChaseLaunchPlan {
                move_toward: true,
                direction_changed: false,
                desired_distance: 6.5,
                desired_relative_angle: Some(0.0),
                shorten_path: false,
                allow_flying_path: false,
                walk: true,
            })
        );
        assert!(chase.moving_towards());
        assert!(
            !chase.has_flag(MovementGeneratorFlags::INFORM_ENABLED),
            "planning alone must preserve the prior inform lifecycle"
        );
        assert!(
            chase.cannot_reach_target,
            "planning alone must not clear a prior cannot-reach result"
        );
        let ChaseMovementAction::Launch(plan) = action else {
            unreachable!("asserted above")
        };
        chase.confirm_path_ready_like_cpp();
        assert!(!chase.cannot_reach_target);
        assert!(chase.has_flag(MovementGeneratorFlags::INFORM_ENABLED));
        chase.confirm_launch_like_cpp(plan);
        assert!(chase.has_flag(MovementGeneratorFlags::INFORM_ENABLED));
    }

    #[test]
    fn chase_move_away_commits_the_repaired_direction_only_after_launch() {
        let mut chase =
            ChaseMovementGenerator::new(guid(7), Some(ChaseRange::between(2.0, 6.0)), None);
        chase.initialize_like_cpp();
        let too_close = snapshot(
            Position::new(1.0, 0.0, 0.0, 0.0),
            Position::new(0.0, 0.0, 0.0, 0.0),
        );

        let action = chase.update_like_cpp(true, true, 1, too_close);
        assert_eq!(
            action,
            ChaseMovementAction::Launch(ChaseLaunchPlan {
                move_toward: false,
                direction_changed: true,
                desired_distance: 3.5,
                desired_relative_angle: None,
                shorten_path: false,
                allow_flying_path: false,
                walk: false,
            })
        );
        assert!(chase.moving_towards());
        assert!(chase.has_flag(MovementGeneratorFlags::INFORM_ENABLED));
        let ChaseMovementAction::Launch(plan) = action else {
            unreachable!("asserted above")
        };
        chase.confirm_launch_like_cpp(plan);
        assert!(!chase.moving_towards());
        assert!(chase.has_flag(MovementGeneratorFlags::INFORM_ENABLED));

        // TrinityCore leaves `_movingTowards == true`, so this first periodic
        // check uses only the maximum bound and immediately stops the newly
        // launched move-away path while the owner is still too close.
        assert_eq!(
            chase.update_like_cpp(true, true, 99, too_close),
            ChaseMovementAction::Continue
        );

        let far_enough = snapshot(
            Position::new(3.5, 0.0, 0.0, 0.0),
            Position::new(0.0, 0.0, 0.0, 0.0),
        );
        assert!(matches!(
            chase.update_like_cpp(true, true, 100, far_enough),
            ChaseMovementAction::StopMovingAndFaceInform(_)
        ));
    }

    #[test]
    fn chase_reset_restores_the_no_live_spline_direction_after_move_away() {
        let mut chase =
            ChaseMovementGenerator::new(guid(7), Some(ChaseRange::between(2.0, 6.0)), None);
        chase.initialize_like_cpp();
        let too_close = snapshot(
            Position::new(1.0, 0.0, 0.0, 0.0),
            Position::new(0.0, 0.0, 0.0, 0.0),
        );
        let ChaseMovementAction::Launch(away) = chase.update_like_cpp(true, true, 1, too_close)
        else {
            panic!("too-close chase must plan a move-away launch");
        };
        chase.confirm_launch_like_cpp(away);
        assert!(!chase.moving_towards());

        chase.deactivate_like_cpp();
        chase.reset_like_cpp();
        assert!(
            chase.moving_towards(),
            "a reset generator has no live spline direction to preserve"
        );

        let far = snapshot(
            Position::new(1.0, 0.0, 0.0, 0.0),
            Position::new(20.0, 0.0, 0.0, 0.0),
        );
        let ChaseMovementAction::Launch(toward) = chase.update_like_cpp(true, true, 100, far)
        else {
            panic!("reset chase must relaunch toward a far target, not inform");
        };
        assert!(toward.move_toward);
    }

    #[test]
    fn chase_update_handles_blocked_lost_target_arrival_and_inaccessible_like_cpp() {
        let mut chase = ChaseMovementGenerator::new(guid(7), None, None);
        let mut blocked = snapshot(
            Position::new(10.0, 0.0, 0.0, 0.0),
            Position::new(0.0, 0.0, 0.0, 0.0),
        );
        blocked.can_move = false;
        assert_eq!(
            chase.update_like_cpp(true, true, 1, blocked),
            ChaseMovementAction::StopMoving
        );
        assert_eq!(chase.stop_moving_calls, 1);

        let mut arrived = blocked;
        arrived.can_move = true;
        arrived.owner_has_chase_move = true;
        arrived.owner_movespline_finalized = true;
        assert_eq!(
            chase.update_like_cpp(true, true, 1, arrived),
            ChaseMovementAction::ClearChaseMoveAndFaceInform(ChaseMovementInform {
                movement_type: MovementGeneratorType::Chase,
                target_counter: 7,
            })
        );

        let mut inaccessible = snapshot(
            Position::new(10.0, 0.0, 0.0, 0.0),
            Position::new(0.0, 0.0, 0.0, 0.0),
        );
        inaccessible.target_accessible = false;
        assert_eq!(
            chase.update_like_cpp(true, true, 1, inaccessible),
            ChaseMovementAction::CannotReachTarget
        );
        assert!(chase.cannot_reach_target);
    }

    #[test]
    fn chase_deactivate_finalize_and_speed_change_match_cpp_flags() {
        let mut chase = ChaseMovementGenerator::new(guid(7), None, None);
        chase.initialize_like_cpp();
        chase.cannot_reach_target = true;
        chase.unit_speed_changed();
        let deactivate = chase.deactivate_like_cpp();
        assert_eq!(
            deactivate,
            ChaseFinalizeAction {
                clear_chase_move: true,
                clear_cannot_reach_target: true,
            }
        );
        assert!(!chase.cannot_reach_target);
        assert!(chase.has_flag(MovementGeneratorFlags::DEACTIVATED));
        assert!(!chase.has_flag(MovementGeneratorFlags::INFORM_ENABLED));

        chase.cannot_reach_target = true;
        let finalize = chase.finalize_like_cpp(true);
        assert_eq!(
            finalize,
            ChaseFinalizeAction {
                clear_chase_move: true,
                clear_cannot_reach_target: true,
            }
        );
        assert!(!chase.cannot_reach_target);
    }
}
