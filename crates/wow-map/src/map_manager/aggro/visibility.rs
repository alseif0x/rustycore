//! Shared Aggro rules, preserving the legacy source gates and order.
use super::*;

pub fn candidate_targetable(
    candidate: &AggroCandidate,
) -> bool {
    let player_flags = UnitFlags::from_bits_truncate(candidate.player_unit_flags);
    let player_state = UnitState::from_bits_truncate(candidate.player_unit_state);

    if candidate.player_is_game_master {
        return false;
    }

    // C++ anchors:
    // - `Unit::isTargetableForAttack(false)` rejects dead, `UNIT_STATE_UNATTACKABLE`,
    //   non-attackable, uninteractible and GM players. In Trinity 3.3.5
    //   `UNIT_STATE_UNATTACKABLE` is currently `UNIT_STATE_IN_FLIGHT`.
    // - `Creature::_IsTargetAcceptable` rejects `UNIT_STATE_DIED` unless the
    //   creature can detect feign death; that override is not represented in
    //   the transitional global aggro scan yet.
    // - `WorldObject::IsValidAttackTarget` rejects untargetable/taxi targets
    //   and `UNIT_FLAG_IMMUNE_TO_NPC` for creature-vs-player attacks.
    if player_state.intersects(UnitState::DIED | UnitState::IN_FLIGHT) {
        return false;
    }

    !player_flags.intersects(
        UnitFlags::NON_ATTACKABLE
            | UnitFlags::UNINTERACTIBLE
            | UnitFlags::NON_ATTACKABLE_2
            | UnitFlags::ON_TAXI
            | UnitFlags::NOT_ATTACKABLE_1
            | UnitFlags::IMMUNE_TO_NPC,
    )
}
fn candidate_unit(
    candidate: &AggroCandidate,
) -> Unit {
    let mut unit = Unit::new(true);
    unit.world_mut().object_mut().create(candidate.player_guid);
    let _ = unit
        .world_mut()
        .set_map(u32::from(candidate.map_id), candidate.instance_id);
    unit.world_mut().relocate(candidate.position);
    *unit.world_mut().phase_shift_mut() = candidate.player_phase_shift.clone();
    unit.set_level(candidate.player_level);
    unit.set_combat_reach(candidate.player_combat_reach);
    unit.set_unit_flags_like_cpp(UnitFlags::from_bits_truncate(candidate.player_unit_flags));
    unit.set_unit_flags2_like_cpp(UnitFlags2::from_bits_truncate(candidate.player_unit_flags2));
    unit.add_unit_state(candidate.player_unit_state);
    unit.replace_visibility_detection_like_cpp(candidate.player_visibility_detection.clone());
    let _ = unit.add_to_world_like_cpp();
    unit
}
fn creature_unit(
    creature: &WorldCreature,
    map_id: u16,
    instance_id: u32,
) -> Unit {
    let source = creature.creature.unit();
    let mut unit = Unit::new(true);
    unit.world_mut()
        .object_mut()
        .create(source.world().object().guid());
    let _ = unit.world_mut().set_map(u32::from(map_id), instance_id);
    unit.world_mut().relocate(creature.position());
    *unit.world_mut().phase_shift_mut() = source.world().phase_shift().clone();
    unit.set_level(creature.level());
    unit.set_combat_reach(source.data().combat_reach);
    unit.set_unit_flags_like_cpp(source.unit_flags_like_cpp());
    unit.set_unit_flags2_like_cpp(source.unit_flags2_like_cpp());
    unit.add_unit_state(source.unit_state());
    unit.replace_visibility_detection_like_cpp(source.visibility_detection_like_cpp().clone());
    let _ = unit.add_to_world_like_cpp();
    unit
}
pub fn candidate_visibility(
    creature: &WorldCreature,
    map_id: u16,
    instance_id: u32,
    candidate: &AggroCandidate,
    check_alert: bool,
) -> AggroVisibility {
    if !candidate.player_visibility_represented {
        return AggroVisibility::Unrepresented;
    }

    let seer = creature_unit(creature, map_id, instance_id);
    let target = candidate_unit(candidate);
    if seer.can_see_or_detect_unit_like_cpp(&target, false, false, check_alert) {
        AggroVisibility::Allowed
    } else {
        AggroVisibility::Rejected
    }
}
pub fn candidate_has_stealth(
    candidate: &AggroCandidate,
) -> bool {
    candidate_unit(candidate).has_stealth_aura_like_cpp()
}
pub fn candidate_accessible(
    creature: &WorldCreature,
    candidate: &AggroCandidate,
) -> bool {
    let victim_is_in_water = candidate.player_liquid_status
        & (LIQUID_MAP_IN_WATER_LIKE_CPP | LIQUID_MAP_UNDER_WATER_LIKE_CPP)
        != 0;

    // C++ `Unit::isInAccessiblePlaceFor(Creature const*)`:
    // water victims require `Creature::CanEnterWater`; non-water victims
    // require `Creature::CanWalk() || Creature::CanFly()`.
    if victim_is_in_water {
        creature.creature.can_enter_water_like_cpp()
    } else {
        creature.creature.can_walk_like_cpp() || creature.creature.can_fly_like_cpp()
    }
}
