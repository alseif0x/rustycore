use super::*;

// Same represented position/reach rules as the compatibility caller. This
// operation does not add a phase gate or a terrain/ground query.
pub(super) fn is_within_melee_range_like_cpp(attacker_position: Position,
    attacker_combat_reach: f32, target_position: Position, target_combat_reach: f32) -> bool {
    let melee_range = (attacker_combat_reach.max(0.0) + target_combat_reach.max(0.0) + 4.0 / 3.0)
        .max(5.0);
    attacker_position.distance(&target_position) <= melee_range
}

pub(super) fn is_within_target_boundary_radius_like_cpp(attacker_position: Position,
    attacker_combat_reach: f32, target_position: Position, target_combat_reach: f32,
    target_bounding_radius: f32) -> bool {
    let boundary_radius = target_bounding_radius.max(2.0)
        + attacker_combat_reach.max(0.0) + target_combat_reach.max(0.0);
    attacker_position.distance(&target_position) < boundary_radius
}

pub(super) fn is_unit_facing_target_for_melee_like_cpp(unit_position: Position,
    target_position: Position) -> bool {
    let dx = target_position.x - unit_position.x;
    let dy = target_position.y - unit_position.y;
    if dx.abs() <= f32::EPSILON && dy.abs() <= f32::EPSILON { return true; }
    let target_angle = dy.atan2(dx);
    let mut diff = (target_angle - unit_position.orientation).rem_euclid(std::f32::consts::TAU);
    if diff > std::f32::consts::PI { diff = std::f32::consts::TAU - diff; }
    diff <= std::f32::consts::PI / 3.0
}

pub(super) fn attack_power_multiplier(base_attack_speed_ms: u32) -> f32 {
    if base_attack_speed_ms > 0 { (base_attack_speed_ms as f32 / 1000.0).max(0.25) }
    else { 2.0 }
}

pub(super) fn rolled_melee_outcome_like_cpp(inputs: &RepresentedMeleeOutcomeInputsLikeCpp)
    -> RepresentedMeleeOutcomeLikeCpp {
    let roll = i32::try_from(wow_core::urand_like_cpp(0, MELEE_OUTCOME_ROLL_MAX_LIKE_CPP))
        .unwrap_or_default();
    melee_outcome_like_cpp(inputs, roll)
}
