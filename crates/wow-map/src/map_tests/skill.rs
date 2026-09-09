//! Skill scenarios for [`super`].
//!
//! Split out of map_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

#[test]
fn summon_object_wild_position_keeps_explicit_destination_like_cpp() {
    let caster = Position::new(10.0, 20.0, 30.0, 1.5);
    let destination = Position::new(4.0, 5.0, 6.0, 0.75);

    let outcome =
        spell_effect_summon_object_wild_position_like_cpp(caster, 2.0, 2.25, Some(destination));

    assert_eq!(outcome.position, destination);
    assert!(outcome.explicit_destination_used);
    assert!(!outcome.close_point_fallback_used);
    assert!(!outcome.normalized_map_coords);
    assert!(outcome.focus_object_orientation_represented);
    assert!(!outcome.collision_los_adjustment_represented);
}
