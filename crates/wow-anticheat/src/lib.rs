// Copyright (c) 2026 alseif0x
// RustyCore - WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 - https://www.gnu.org/licenses/gpl-3.0.html

//! C++-anchored movement anticheat helpers.
//!
//! This crate starts with `Player::ValidateMovementInfo` from the legacy C++
//! tree. The function mutates the incoming `MovementInfo` in place like C++:
//! it strips impossible flags and never rejects the packet.

use wow_constants::movement::MovementFlag;
use wow_packet::packets::movement::MovementInfo;

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

pub fn validate_movement_info(
    movement_info: &mut MovementInfo,
    player_state: &PlayerState,
) -> ValidationResult {
    let mut result = ValidationResult {
        removed_flags: MovementFlag::empty(),
        added_flags: MovementFlag::empty(),
        stripped_rules: Vec::new(),
    };

    remove_if(
        movement_info,
        &mut result,
        movement_info.flags.contains(MovementFlag::ROOT)
            && !player_state.mover_fixed_position_vehicle,
        MovementFlag::ROOT,
        MovementSanitizerRule::RootWithoutFixedVehicle,
    );

    remove_if(
        movement_info,
        &mut result,
        movement_info.flags.contains(MovementFlag::ROOT)
            && movement_info.flags.intersects(MovementFlag::MASK_MOVING),
        MovementFlag::MASK_MOVING,
        MovementSanitizerRule::RootWithMovingFlags,
    );

    remove_if(
        movement_info,
        &mut result,
        movement_info.flags.contains(MovementFlag::HOVER) && !player_state.has_hover_aura,
        MovementFlag::HOVER,
        MovementSanitizerRule::HoverWithoutAura,
    );

    remove_if(
        movement_info,
        &mut result,
        movement_info
            .flags
            .contains(MovementFlag::ASCENDING | MovementFlag::DESCENDING),
        MovementFlag::ASCENDING | MovementFlag::DESCENDING,
        MovementSanitizerRule::AscendingAndDescending,
    );

    remove_if(
        movement_info,
        &mut result,
        movement_info
            .flags
            .contains(MovementFlag::LEFT | MovementFlag::RIGHT),
        MovementFlag::LEFT | MovementFlag::RIGHT,
        MovementSanitizerRule::LeftAndRight,
    );

    remove_if(
        movement_info,
        &mut result,
        movement_info
            .flags
            .contains(MovementFlag::STRAFE_LEFT | MovementFlag::STRAFE_RIGHT),
        MovementFlag::STRAFE_LEFT | MovementFlag::STRAFE_RIGHT,
        MovementSanitizerRule::StrafeLeftAndRight,
    );

    remove_if(
        movement_info,
        &mut result,
        movement_info
            .flags
            .contains(MovementFlag::PITCH_UP | MovementFlag::PITCH_DOWN),
        MovementFlag::PITCH_UP | MovementFlag::PITCH_DOWN,
        MovementSanitizerRule::PitchUpAndDown,
    );

    remove_if(
        movement_info,
        &mut result,
        movement_info
            .flags
            .contains(MovementFlag::FORWARD | MovementFlag::BACKWARD),
        MovementFlag::FORWARD | MovementFlag::BACKWARD,
        MovementSanitizerRule::ForwardAndBackward,
    );

    remove_if(
        movement_info,
        &mut result,
        movement_info.flags.contains(MovementFlag::WATER_WALK)
            && !player_state.has_water_walk_aura
            && !player_state.has_ghost_aura,
        MovementFlag::WATER_WALK,
        MovementSanitizerRule::WaterWalkWithoutAuraOrGhost,
    );

    remove_if(
        movement_info,
        &mut result,
        movement_info.flags.contains(MovementFlag::FALLING_SLOW)
            && !player_state.has_feather_fall_aura,
        MovementFlag::FALLING_SLOW,
        MovementSanitizerRule::FallingSlowWithoutAura,
    );

    remove_if(
        movement_info,
        &mut result,
        movement_info
            .flags
            .intersects(MovementFlag::FLYING | MovementFlag::CAN_FLY)
            && player_state.is_player_security
            && !player_state.has_fly_aura
            && !player_state.has_mounted_flight_speed_aura,
        MovementFlag::FLYING | MovementFlag::CAN_FLY,
        MovementSanitizerRule::FlyWithoutAuraOrSecurity,
    );

    remove_if(
        movement_info,
        &mut result,
        movement_info
            .flags
            .intersects(MovementFlag::DISABLE_GRAVITY | MovementFlag::CAN_FLY)
            && movement_info.flags.contains(MovementFlag::FALLING),
        MovementFlag::FALLING,
        MovementSanitizerRule::FallingWithGravityDisabledOrCanFly,
    );

    let has_step_up_elevation = movement_info.step_up_start_elevation.abs() > f32::EPSILON;
    remove_if(
        movement_info,
        &mut result,
        movement_info.flags.contains(MovementFlag::SPLINE_ELEVATION) && !has_step_up_elevation,
        MovementFlag::SPLINE_ELEVATION,
        MovementSanitizerRule::SplineElevationWithZeroStep,
    );

    if has_step_up_elevation && !movement_info.flags.contains(MovementFlag::SPLINE_ELEVATION) {
        movement_info.flags.insert(MovementFlag::SPLINE_ELEVATION);
        result.added_flags |= MovementFlag::SPLINE_ELEVATION;
        result
            .stripped_rules
            .push(MovementSanitizerRule::SplineElevationAddedForNonZeroStep);
    }

    result
}

fn remove_if(
    movement_info: &mut MovementInfo,
    result: &mut ValidationResult,
    condition: bool,
    flags: MovementFlag,
    rule: MovementSanitizerRule,
) {
    if condition {
        movement_info.flags.remove(flags);
        result.removed_flags |= flags;
        result.stripped_rules.push(rule);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn movement(flags: MovementFlag) -> MovementInfo {
        MovementInfo {
            flags,
            ..MovementInfo::default()
        }
    }

    #[test]
    fn root_order_matches_cpp_for_non_fixed_vehicle() {
        let mut info = movement(MovementFlag::ROOT | MovementFlag::FORWARD);

        let result = validate_movement_info(&mut info, &PlayerState::default());

        assert_eq!(info.flags, MovementFlag::FORWARD);
        assert!(result.removed_flags.contains(MovementFlag::ROOT));
        assert!(!result.removed_flags.contains(MovementFlag::FORWARD));
        assert_eq!(
            result.stripped_rules,
            vec![MovementSanitizerRule::RootWithoutFixedVehicle]
        );
    }

    #[test]
    fn root_on_fixed_vehicle_strips_moving_flags_like_cpp() {
        let mut info = movement(MovementFlag::ROOT | MovementFlag::FORWARD);
        let state = PlayerState {
            mover_fixed_position_vehicle: true,
            ..PlayerState::default()
        };

        let result = validate_movement_info(&mut info, &state);

        assert_eq!(info.flags, MovementFlag::ROOT);
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
            let mut info = movement(flag);
            let result = validate_movement_info(&mut info, &PlayerState::default());

            assert!(info.flags.is_empty(), "{flag:?}");
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
        let mut info = movement(
            MovementFlag::HOVER
                | MovementFlag::WATER_WALK
                | MovementFlag::FALLING_SLOW
                | MovementFlag::FLYING
                | MovementFlag::CAN_FLY,
        );

        let result = validate_movement_info(&mut info, &state);

        assert!(result.clean());
        assert!(
            info.flags.contains(
                MovementFlag::HOVER | MovementFlag::WATER_WALK | MovementFlag::FALLING_SLOW
            )
        );
        assert!(
            info.flags
                .contains(MovementFlag::FLYING | MovementFlag::CAN_FLY)
        );
    }

    #[test]
    fn ghost_keeps_water_walk_like_cpp() {
        let state = PlayerState {
            has_ghost_aura: true,
            ..PlayerState::default()
        };
        let mut info = movement(MovementFlag::WATER_WALK);

        let result = validate_movement_info(&mut info, &state);

        assert!(result.clean());
        assert_eq!(info.flags, MovementFlag::WATER_WALK);
    }

    #[test]
    fn gm_and_mounted_flight_speed_keep_flying_like_cpp() {
        let mut gm_info = movement(MovementFlag::FLYING | MovementFlag::CAN_FLY);
        let gm_state = PlayerState {
            is_player_security: false,
            ..PlayerState::default()
        };
        assert!(validate_movement_info(&mut gm_info, &gm_state).clean());
        assert!(
            gm_info
                .flags
                .contains(MovementFlag::FLYING | MovementFlag::CAN_FLY)
        );

        let mut mounted_flight_info = movement(MovementFlag::FLYING | MovementFlag::CAN_FLY);
        let mounted_flight_state = PlayerState {
            has_mounted_flight_speed_aura: true,
            ..PlayerState::default()
        };
        assert!(validate_movement_info(&mut mounted_flight_info, &mounted_flight_state).clean());
        assert!(
            mounted_flight_info
                .flags
                .contains(MovementFlag::FLYING | MovementFlag::CAN_FLY)
        );
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
            let mut info = movement(left | right);

            let result = validate_movement_info(&mut info, &PlayerState::default());

            assert!(info.flags.is_empty(), "{left:?} | {right:?}");
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
        let flying_result = validate_movement_info(&mut flying, &PlayerState::default());
        assert!(flying.flags.is_empty());
        assert_eq!(
            flying_result.stripped_rules,
            vec![MovementSanitizerRule::FlyWithoutAuraOrSecurity]
        );

        let mut falling = movement(MovementFlag::DISABLE_GRAVITY | MovementFlag::FALLING);
        let falling_result = validate_movement_info(&mut falling, &PlayerState::default());
        assert_eq!(falling.flags, MovementFlag::DISABLE_GRAVITY);
        assert_eq!(
            falling_result.stripped_rules,
            vec![MovementSanitizerRule::FallingWithGravityDisabledOrCanFly]
        );
    }

    #[test]
    fn spline_elevation_rules_match_cpp() {
        let mut zero = movement(MovementFlag::SPLINE_ELEVATION);
        let zero_result = validate_movement_info(&mut zero, &PlayerState::default());
        assert!(zero.flags.is_empty());
        assert_eq!(
            zero_result.stripped_rules,
            vec![MovementSanitizerRule::SplineElevationWithZeroStep]
        );

        let mut non_zero = movement(MovementFlag::empty());
        non_zero.step_up_start_elevation = 1.0;
        let non_zero_result = validate_movement_info(&mut non_zero, &PlayerState::default());
        assert!(non_zero.flags.contains(MovementFlag::SPLINE_ELEVATION));
        assert_eq!(non_zero_result.added_flags, MovementFlag::SPLINE_ELEVATION);
        assert_eq!(
            non_zero_result.stripped_rules,
            vec![MovementSanitizerRule::SplineElevationAddedForNonZeroStep]
        );
    }
}
