// Copyright (c) 2026 alseif0x
// RustyCore - WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 - https://www.gnu.org/licenses/gpl-3.0.html

//! C++-anchored movement anticheat helpers.
//!
//! This crate starts with `Player::ValidateMovementInfo` from the legacy C++
//! tree. It mutates the represented movement flags in place like C++: it strips
//! impossible flags and never rejects the packet.

use wow_constants::movement::MovementFlag;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerState {
    pub mover_fixed_position_vehicle: bool,
    pub has_hover_aura: bool,
    pub has_water_walk_aura: bool,
    pub has_ghost_aura: bool,
    pub has_feather_fall_aura: bool,
    pub has_fly_aura: bool,
    pub has_mounted_flight_speed_aura: bool,
    pub is_player_security: bool,
}

impl Default for PlayerState {
    fn default() -> Self {
        Self {
            mover_fixed_position_vehicle: false,
            has_hover_aura: false,
            has_water_walk_aura: false,
            has_ghost_aura: false,
            has_feather_fall_aura: false,
            has_fly_aura: false,
            has_mounted_flight_speed_aura: false,
            is_player_security: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MovementSanitizerRule {
    RootWithoutFixedVehicle,
    RootWithMovingFlags,
    HoverWithoutAura,
    AscendingAndDescending,
    LeftAndRight,
    StrafeLeftAndRight,
    PitchUpAndDown,
    ForwardAndBackward,
    WaterWalkWithoutAuraOrGhost,
    FallingSlowWithoutAura,
    FlyWithoutAuraOrSecurity,
    FallingWithGravityDisabledOrCanFly,
    SplineElevationWithZeroStep,
    SplineElevationAddedForNonZeroStep,
}

impl MovementSanitizerRule {
    #[must_use]
    pub fn trace_rule_name_like_cpp(self) -> &'static str {
        match self {
            Self::RootWithoutFixedVehicle => "Player.ValidateMovementInfo.RootWithoutFixedVehicle",
            Self::RootWithMovingFlags => "Player.ValidateMovementInfo.RootWithMovingFlags",
            Self::HoverWithoutAura => "Player.ValidateMovementInfo.HoverWithoutAura",
            Self::AscendingAndDescending => "Player.ValidateMovementInfo.AscendingAndDescending",
            Self::LeftAndRight => "Player.ValidateMovementInfo.LeftAndRight",
            Self::StrafeLeftAndRight => "Player.ValidateMovementInfo.StrafeLeftAndRight",
            Self::PitchUpAndDown => "Player.ValidateMovementInfo.PitchUpAndDown",
            Self::ForwardAndBackward => "Player.ValidateMovementInfo.ForwardAndBackward",
            Self::WaterWalkWithoutAuraOrGhost => {
                "Player.ValidateMovementInfo.WaterWalkWithoutAuraOrGhost"
            }
            Self::FallingSlowWithoutAura => "Player.ValidateMovementInfo.FallingSlowWithoutAura",
            Self::FlyWithoutAuraOrSecurity => {
                "Player.ValidateMovementInfo.FlyWithoutAuraOrSecurity"
            }
            Self::FallingWithGravityDisabledOrCanFly => {
                "Player.ValidateMovementInfo.FallingWithGravityDisabledOrCanFly"
            }
            Self::SplineElevationWithZeroStep => {
                "Player.ValidateMovementInfo.SplineElevationWithZeroStep"
            }
            Self::SplineElevationAddedForNonZeroStep => {
                "Player.ValidateMovementInfo.SplineElevationAddedForNonZeroStep"
            }
        }
    }

    #[must_use]
    pub fn removes_flags_like_cpp(self) -> bool {
        !matches!(self, Self::SplineElevationAddedForNonZeroStep)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationResult {
    pub removed_flags: MovementFlag,
    pub added_flags: MovementFlag,
    pub stripped_rules: Vec<MovementSanitizerRule>,
}

impl ValidationResult {
    #[must_use]
    pub fn clean(&self) -> bool {
        self.removed_flags.is_empty() && self.added_flags.is_empty()
    }
}

pub fn validate_movement_flags(
    flags: &mut MovementFlag,
    step_up_start_elevation: f32,
    player_state: &PlayerState,
) -> ValidationResult {
    let mut result = ValidationResult {
        removed_flags: MovementFlag::empty(),
        added_flags: MovementFlag::empty(),
        stripped_rules: Vec::new(),
    };

    let should_remove =
        flags.contains(MovementFlag::ROOT) && !player_state.mover_fixed_position_vehicle;
    remove_if(
        flags,
        &mut result,
        should_remove,
        MovementFlag::ROOT,
        MovementSanitizerRule::RootWithoutFixedVehicle,
    );

    let should_remove =
        flags.contains(MovementFlag::ROOT) && flags.intersects(MovementFlag::MASK_MOVING);
    remove_if(
        flags,
        &mut result,
        should_remove,
        MovementFlag::MASK_MOVING,
        MovementSanitizerRule::RootWithMovingFlags,
    );

    let should_remove = flags.contains(MovementFlag::HOVER) && !player_state.has_hover_aura;
    remove_if(
        flags,
        &mut result,
        should_remove,
        MovementFlag::HOVER,
        MovementSanitizerRule::HoverWithoutAura,
    );

    let should_remove = flags.contains(MovementFlag::ASCENDING | MovementFlag::DESCENDING);
    remove_if(
        flags,
        &mut result,
        should_remove,
        MovementFlag::ASCENDING | MovementFlag::DESCENDING,
        MovementSanitizerRule::AscendingAndDescending,
    );

    let should_remove = flags.contains(MovementFlag::LEFT | MovementFlag::RIGHT);
    remove_if(
        flags,
        &mut result,
        should_remove,
        MovementFlag::LEFT | MovementFlag::RIGHT,
        MovementSanitizerRule::LeftAndRight,
    );

    let should_remove = flags.contains(MovementFlag::STRAFE_LEFT | MovementFlag::STRAFE_RIGHT);
    remove_if(
        flags,
        &mut result,
        should_remove,
        MovementFlag::STRAFE_LEFT | MovementFlag::STRAFE_RIGHT,
        MovementSanitizerRule::StrafeLeftAndRight,
    );

    let should_remove = flags.contains(MovementFlag::PITCH_UP | MovementFlag::PITCH_DOWN);
    remove_if(
        flags,
        &mut result,
        should_remove,
        MovementFlag::PITCH_UP | MovementFlag::PITCH_DOWN,
        MovementSanitizerRule::PitchUpAndDown,
    );

    let should_remove = flags.contains(MovementFlag::FORWARD | MovementFlag::BACKWARD);
    remove_if(
        flags,
        &mut result,
        should_remove,
        MovementFlag::FORWARD | MovementFlag::BACKWARD,
        MovementSanitizerRule::ForwardAndBackward,
    );

    let should_remove = flags.contains(MovementFlag::WATER_WALK)
        && !player_state.has_water_walk_aura
        && !player_state.has_ghost_aura;
    remove_if(
        flags,
        &mut result,
        should_remove,
        MovementFlag::WATER_WALK,
        MovementSanitizerRule::WaterWalkWithoutAuraOrGhost,
    );

    let should_remove =
        flags.contains(MovementFlag::FALLING_SLOW) && !player_state.has_feather_fall_aura;
    remove_if(
        flags,
        &mut result,
        should_remove,
        MovementFlag::FALLING_SLOW,
        MovementSanitizerRule::FallingSlowWithoutAura,
    );

    let should_remove = flags.intersects(MovementFlag::FLYING | MovementFlag::CAN_FLY)
        && player_state.is_player_security
        && !player_state.has_fly_aura
        && !player_state.has_mounted_flight_speed_aura;
    remove_if(
        flags,
        &mut result,
        should_remove,
        MovementFlag::FLYING | MovementFlag::CAN_FLY,
        MovementSanitizerRule::FlyWithoutAuraOrSecurity,
    );

    let should_remove = flags.intersects(MovementFlag::DISABLE_GRAVITY | MovementFlag::CAN_FLY)
        && flags.contains(MovementFlag::FALLING);
    remove_if(
        flags,
        &mut result,
        should_remove,
        MovementFlag::FALLING,
        MovementSanitizerRule::FallingWithGravityDisabledOrCanFly,
    );

    let has_step_up_elevation = step_up_start_elevation.abs() > f32::EPSILON;
    let should_remove = flags.contains(MovementFlag::SPLINE_ELEVATION) && !has_step_up_elevation;
    remove_if(
        flags,
        &mut result,
        should_remove,
        MovementFlag::SPLINE_ELEVATION,
        MovementSanitizerRule::SplineElevationWithZeroStep,
    );

    if has_step_up_elevation && !flags.contains(MovementFlag::SPLINE_ELEVATION) {
        flags.insert(MovementFlag::SPLINE_ELEVATION);
        result.added_flags |= MovementFlag::SPLINE_ELEVATION;
        result
            .stripped_rules
            .push(MovementSanitizerRule::SplineElevationAddedForNonZeroStep);
    }

    result
}

fn remove_if(
    current_flags: &mut MovementFlag,
    result: &mut ValidationResult,
    condition: bool,
    flags_to_remove: MovementFlag,
    rule: MovementSanitizerRule,
) {
    if condition {
        current_flags.remove(flags_to_remove);
        result.removed_flags |= flags_to_remove;
        result.stripped_rules.push(rule);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn movement(flags: MovementFlag) -> MovementFlag {
        flags
    }

    #[test]
    fn root_order_matches_cpp_for_non_fixed_vehicle() {
        let mut flags = movement(MovementFlag::ROOT | MovementFlag::FORWARD);

        let result = validate_movement_flags(&mut flags, 0.0, &PlayerState::default());

        assert_eq!(flags, MovementFlag::FORWARD);
        assert!(result.removed_flags.contains(MovementFlag::ROOT));
        assert!(!result.removed_flags.contains(MovementFlag::FORWARD));
        assert_eq!(
            result.stripped_rules,
            vec![MovementSanitizerRule::RootWithoutFixedVehicle]
        );
    }

    #[test]
    fn root_on_fixed_vehicle_strips_moving_flags_like_cpp() {
        let mut flags = movement(MovementFlag::ROOT | MovementFlag::FORWARD);
        let state = PlayerState {
            mover_fixed_position_vehicle: true,
            ..PlayerState::default()
        };

        let result = validate_movement_flags(&mut flags, 0.0, &state);

        assert_eq!(flags, MovementFlag::ROOT);
        assert!(result.removed_flags.contains(MovementFlag::FORWARD));
        assert!(!result.removed_flags.contains(MovementFlag::ROOT));
        assert_eq!(
            result.stripped_rules,
            vec![MovementSanitizerRule::RootWithMovingFlags]
        );
    }

    #[test]
    fn aura_gated_flags_match_cpp() {
        for (flag, rule) in [
            (MovementFlag::HOVER, MovementSanitizerRule::HoverWithoutAura),
            (
                MovementFlag::WATER_WALK,
                MovementSanitizerRule::WaterWalkWithoutAuraOrGhost,
            ),
            (
                MovementFlag::FALLING_SLOW,
                MovementSanitizerRule::FallingSlowWithoutAura,
            ),
        ] {
            let mut flags = movement(flag);
            let result = validate_movement_flags(&mut flags, 0.0, &PlayerState::default());

            assert!(flags.is_empty(), "{flag:?}");
            assert_eq!(result.stripped_rules, vec![rule], "{flag:?}");
        }
    }

    #[test]
    fn aura_exceptions_keep_flags_like_cpp() {
        let state = PlayerState {
            has_hover_aura: true,
            has_water_walk_aura: true,
            has_feather_fall_aura: true,
            has_fly_aura: true,
            ..PlayerState::default()
        };
        let mut flags = movement(
            MovementFlag::HOVER
                | MovementFlag::WATER_WALK
                | MovementFlag::FALLING_SLOW
                | MovementFlag::FLYING
                | MovementFlag::CAN_FLY,
        );

        let result = validate_movement_flags(&mut flags, 0.0, &state);

        assert!(result.clean());
        assert!(
            flags.contains(
                MovementFlag::HOVER | MovementFlag::WATER_WALK | MovementFlag::FALLING_SLOW
            )
        );
        assert!(flags.contains(MovementFlag::FLYING | MovementFlag::CAN_FLY));
    }

    #[test]
    fn ghost_keeps_water_walk_like_cpp() {
        let state = PlayerState {
            has_ghost_aura: true,
            ..PlayerState::default()
        };
        let mut flags = movement(MovementFlag::WATER_WALK);

        let result = validate_movement_flags(&mut flags, 0.0, &state);

        assert!(result.clean());
        assert_eq!(flags, MovementFlag::WATER_WALK);
    }

    #[test]
    fn gm_and_mounted_flight_speed_keep_flying_like_cpp() {
        let mut gm_flags = movement(MovementFlag::FLYING | MovementFlag::CAN_FLY);
        let gm_state = PlayerState {
            is_player_security: false,
            ..PlayerState::default()
        };
        assert!(validate_movement_flags(&mut gm_flags, 0.0, &gm_state).clean());
        assert!(gm_flags.contains(MovementFlag::FLYING | MovementFlag::CAN_FLY));

        let mut mounted_flight_flags = movement(MovementFlag::FLYING | MovementFlag::CAN_FLY);
        let mounted_flight_state = PlayerState {
            has_mounted_flight_speed_aura: true,
            ..PlayerState::default()
        };
        assert!(
            validate_movement_flags(&mut mounted_flight_flags, 0.0, &mounted_flight_state).clean()
        );
        assert!(mounted_flight_flags.contains(MovementFlag::FLYING | MovementFlag::CAN_FLY));
    }

    #[test]
    fn incompatible_pairs_strip_independently_like_cpp() {
        for (left, right, rule) in [
            (
                MovementFlag::ASCENDING,
                MovementFlag::DESCENDING,
                MovementSanitizerRule::AscendingAndDescending,
            ),
            (
                MovementFlag::LEFT,
                MovementFlag::RIGHT,
                MovementSanitizerRule::LeftAndRight,
            ),
            (
                MovementFlag::STRAFE_LEFT,
                MovementFlag::STRAFE_RIGHT,
                MovementSanitizerRule::StrafeLeftAndRight,
            ),
            (
                MovementFlag::PITCH_UP,
                MovementFlag::PITCH_DOWN,
                MovementSanitizerRule::PitchUpAndDown,
            ),
            (
                MovementFlag::FORWARD,
                MovementFlag::BACKWARD,
                MovementSanitizerRule::ForwardAndBackward,
            ),
        ] {
            let mut flags = movement(left | right);

            let result = validate_movement_flags(&mut flags, 0.0, &PlayerState::default());

            assert!(flags.is_empty(), "{left:?} | {right:?}");
            assert!(
                result.removed_flags.contains(left | right),
                "{left:?} | {right:?}"
            );
            assert_eq!(result.stripped_rules, vec![rule], "{left:?} | {right:?}");
        }
    }

    #[test]
    fn flying_and_falling_rules_match_cpp() {
        let mut flying = movement(MovementFlag::FLYING | MovementFlag::CAN_FLY);
        let flying_result = validate_movement_flags(&mut flying, 0.0, &PlayerState::default());
        assert!(flying.is_empty());
        assert_eq!(
            flying_result.stripped_rules,
            vec![MovementSanitizerRule::FlyWithoutAuraOrSecurity]
        );

        let mut falling = movement(MovementFlag::DISABLE_GRAVITY | MovementFlag::FALLING);
        let falling_result = validate_movement_flags(&mut falling, 0.0, &PlayerState::default());
        assert_eq!(falling, MovementFlag::DISABLE_GRAVITY);
        assert_eq!(
            falling_result.stripped_rules,
            vec![MovementSanitizerRule::FallingWithGravityDisabledOrCanFly]
        );
    }

    #[test]
    fn spline_elevation_rules_match_cpp() {
        let mut zero = movement(MovementFlag::SPLINE_ELEVATION);
        let zero_result = validate_movement_flags(&mut zero, 0.0, &PlayerState::default());
        assert!(zero.is_empty());
        assert_eq!(
            zero_result.stripped_rules,
            vec![MovementSanitizerRule::SplineElevationWithZeroStep]
        );

        let mut non_zero = movement(MovementFlag::empty());
        let non_zero_result = validate_movement_flags(&mut non_zero, 1.0, &PlayerState::default());
        assert!(non_zero.contains(MovementFlag::SPLINE_ELEVATION));
        assert_eq!(non_zero_result.added_flags, MovementFlag::SPLINE_ELEVATION);
        assert_eq!(
            non_zero_result.stripped_rules,
            vec![MovementSanitizerRule::SplineElevationAddedForNonZeroStep]
        );

        let mut negative_step = movement(MovementFlag::empty());
        let negative_step_result =
            validate_movement_flags(&mut negative_step, -1.0, &PlayerState::default());
        assert!(negative_step.contains(MovementFlag::SPLINE_ELEVATION));
        assert_eq!(
            negative_step_result.added_flags,
            MovementFlag::SPLINE_ELEVATION
        );

        let mut epsilon_step = movement(MovementFlag::SPLINE_ELEVATION);
        let epsilon_result =
            validate_movement_flags(&mut epsilon_step, f32::EPSILON, &PlayerState::default());
        assert!(epsilon_step.is_empty());
        assert_eq!(
            epsilon_result.stripped_rules,
            vec![MovementSanitizerRule::SplineElevationWithZeroStep]
        );

        let mut nan_step = movement(MovementFlag::SPLINE_ELEVATION);
        let nan_result = validate_movement_flags(&mut nan_step, f32::NAN, &PlayerState::default());
        assert!(nan_step.is_empty());
        assert_eq!(
            nan_result.stripped_rules,
            vec![MovementSanitizerRule::SplineElevationWithZeroStep]
        );
    }
}
