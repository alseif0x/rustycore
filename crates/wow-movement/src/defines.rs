use std::f32::consts::{FRAC_PI_4, TAU};

use wow_core::ObjectGuid;
pub use wow_data_model::jump_charge::{JumpChargeParams, JumpChargeSpec};

pub const CONTACT_DISTANCE_LIKE_CPP: f32 = 0.5;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum RotateDirection {
    Left = 0,
    Right = 1,
}

impl RotateDirection {
    #[must_use]
    pub const fn from_trinity_id(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::Left),
            1 => Some(Self::Right),
            2..=u8::MAX => None,
        }
    }

    #[must_use]
    pub const fn trinity_id(self) -> u8 {
        self as u8
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum MovementWalkRunSpeedSelectionMode {
    #[default]
    Default,
    ForceRun,
    ForceWalk,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ChaseRange {
    pub min_range: f32,
    pub min_tolerance: f32,
    pub max_range: f32,
    pub max_tolerance: f32,
}

impl ChaseRange {
    #[must_use]
    pub fn new(range: f32) -> Self {
        Self {
            min_range: if range > CONTACT_DISTANCE_LIKE_CPP {
                0.0
            } else {
                range - CONTACT_DISTANCE_LIKE_CPP
            },
            min_tolerance: range,
            max_range: range + CONTACT_DISTANCE_LIKE_CPP,
            max_tolerance: range,
        }
    }

    #[must_use]
    pub fn between(min_range: f32, max_range: f32) -> Self {
        let min_tolerance =
            (min_range + CONTACT_DISTANCE_LIKE_CPP).min((min_range + max_range) / 2.0);
        Self {
            min_range,
            min_tolerance,
            max_range,
            max_tolerance: (max_range - CONTACT_DISTANCE_LIKE_CPP).max(min_tolerance),
        }
    }

    #[must_use]
    pub const fn exact(
        min_range: f32,
        min_tolerance: f32,
        max_tolerance: f32,
        max_range: f32,
    ) -> Self {
        Self {
            min_range,
            min_tolerance,
            max_range,
            max_tolerance,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ChaseAngle {
    pub relative_angle: f32,
    pub tolerance: f32,
}

impl ChaseAngle {
    #[must_use]
    pub fn new(angle: f32) -> Self {
        Self::with_tolerance(angle, FRAC_PI_4)
    }

    #[must_use]
    pub fn with_tolerance(angle: f32, tolerance: f32) -> Self {
        Self {
            relative_angle: normalize_orientation_like_cpp(angle),
            tolerance,
        }
    }

    #[must_use]
    pub fn upper_bound(self) -> f32 {
        normalize_orientation_like_cpp(self.relative_angle + self.tolerance)
    }

    #[must_use]
    pub fn lower_bound(self) -> f32 {
        normalize_orientation_like_cpp(self.relative_angle - self.tolerance)
    }

    #[must_use]
    pub fn is_angle_okay(self, relative_angle: f32) -> bool {
        let diff = (relative_angle - self.relative_angle).abs();
        diff.min(TAU - diff) <= self.tolerance
    }
}

#[must_use]
pub fn normalize_orientation_like_cpp(mut orientation: f32) -> f32 {
    if orientation < 0.0 {
        orientation = -orientation;
        orientation %= TAU;
        -orientation + TAU
    } else {
        orientation % TAU
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct JumpArrivalCastArgs {
    pub spell_id: u32,
    pub target: ObjectGuid,
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPSILON: f32 = 0.000_01;

    fn assert_close(left: f32, right: f32) {
        assert!((left - right).abs() < EPSILON, "left={left}, right={right}");
    }

    #[test]
    fn movement_walk_run_speed_selection_mode_defaults_like_cpp() {
        assert_eq!(
            MovementWalkRunSpeedSelectionMode::default(),
            MovementWalkRunSpeedSelectionMode::Default
        );
    }

    #[test]
    fn rotate_direction_values_match_cpp() {
        assert_eq!(RotateDirection::Left.trinity_id(), 0);
        assert_eq!(RotateDirection::Right.trinity_id(), 1);
        assert_eq!(
            RotateDirection::from_trinity_id(0),
            Some(RotateDirection::Left)
        );
        assert_eq!(
            RotateDirection::from_trinity_id(1),
            Some(RotateDirection::Right)
        );
        assert_eq!(RotateDirection::from_trinity_id(2), None);
    }

    #[test]
    fn chase_range_single_range_matches_cpp_contact_distance_rules() {
        let melee = ChaseRange::new(0.25);
        assert_close(melee.min_range, -0.25);
        assert_close(melee.min_tolerance, 0.25);
        assert_close(melee.max_range, 0.75);
        assert_close(melee.max_tolerance, 0.25);

        let ranged = ChaseRange::new(5.0);
        assert_close(ranged.min_range, 0.0);
        assert_close(ranged.min_tolerance, 5.0);
        assert_close(ranged.max_range, 5.5);
        assert_close(ranged.max_tolerance, 5.0);
    }

    #[test]
    fn chase_range_min_max_constructor_matches_cpp_tolerance_clamps() {
        let wide = ChaseRange::between(2.0, 10.0);
        assert_close(wide.min_range, 2.0);
        assert_close(wide.min_tolerance, 2.5);
        assert_close(wide.max_range, 10.0);
        assert_close(wide.max_tolerance, 9.5);

        let tight = ChaseRange::between(2.0, 2.2);
        assert_close(tight.min_tolerance, 2.1);
        assert_close(tight.max_tolerance, 2.1);
    }

    #[test]
    fn chase_angle_bounds_and_wrap_match_cpp() {
        let angle = ChaseAngle::with_tolerance(-0.25, 0.5);
        assert_close(angle.relative_angle, TAU - 0.25);
        assert_close(angle.upper_bound(), 0.25);
        assert_close(angle.lower_bound(), TAU - 0.75);
        assert!(angle.is_angle_okay(0.1));
        assert!(angle.is_angle_okay(TAU - 0.4));
        assert!(!angle.is_angle_okay(1.0));
    }

    #[test]
    fn jump_arrival_and_charge_params_match_cpp_field_shape() {
        let target = ObjectGuid::create_uniq(0x1234);
        let arrival = JumpArrivalCastArgs {
            spell_id: 1234,
            target,
        };
        assert_eq!(arrival.spell_id, 1234);
        assert_eq!(arrival.target, target);
        assert_eq!(JumpArrivalCastArgs::default().spell_id, 0);
        assert_eq!(JumpArrivalCastArgs::default().target, ObjectGuid::EMPTY);

        let speed = JumpChargeParams::with_speed(14.5)
            .with_jump_gravity(19.0)
            .with_spell_visual_id(7)
            .with_progress_curve_id(8)
            .with_parabolic_curve_id(9);
        assert_eq!(speed.spec.value(), 14.5);
        assert!(!speed.spec.treat_speed_as_move_time_seconds());
        assert_close(speed.jump_gravity, 19.0);
        assert_eq!(speed.spell_visual_id, Some(7));
        assert_eq!(speed.progress_curve_id, Some(8));
        assert_eq!(speed.parabolic_curve_id, Some(9));

        let timed = JumpChargeParams::with_move_time_seconds(2.5);
        assert_eq!(timed.spec.value(), 2.5);
        assert!(timed.spec.treat_speed_as_move_time_seconds());
        assert_close(timed.jump_gravity, 0.0);
        assert_eq!(timed.spell_visual_id, None);
    }
}
