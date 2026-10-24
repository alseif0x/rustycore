use super::*;
use crate::{Creature, Player, UnitVisibilityContext};

fn unit(counter: i64) -> Unit {
    let mut unit = Unit::new(true);
    unit.world_mut()
        .object_mut()
        .create(ObjectGuid::new(1, counter));
    unit
}

fn player(counter: i64) -> Player {
    let mut player = Player::new(None, false);
    player
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(ObjectGuid::create_player(1, counter));
    player
}

fn creature(counter: i64) -> Creature {
    let mut creature = Creature::new(true);
    creature.unit_mut().world_mut().object_mut().create(
        ObjectGuid::create_creature_like_cpp(1, 571, 9_101, counter),
    );
    creature
}

fn context(
    distance_allowed: bool,
    corpse_visibility: bool,
    player_group_visibility: bool,
) -> UnitVisibilityContext {
    UnitVisibilityContext {
        distance_allowed,
        corpse_visibility,
        player_group_visibility,
    }
}

#[test]
fn distance_result_precedes_gm_visibility() {
    let seer = unit(1);
    let mut target = unit(2);
    target.set_server_side_gm_visibility_like_cpp(1);
    let mut seer = seer;
    seer.set_server_side_gm_visibility_detect_like_cpp(1);

    assert!(!seer.can_see_or_detect_unit_with_context(
        &target,
        false,
        false,
        false,
        context(false, false, false),
    ));
    assert!(seer.can_see_or_detect_unit_with_context(
        &target,
        false,
        false,
        false,
        context(true, false, false),
    ));
}

#[test]
fn phase_private_smooth_and_conditions_gates_precede_context_exceptions() {
    let mut seer = player(11);
    let mut target = player(12);
    let seer_guid = seer.unit().world().object().guid();
    target.unit_mut().set_server_side_ghost_visibility_like_cpp(0x2);
    let bypasses = context(true, true, true);

    seer.unit_mut().world_mut().phase_shift_mut().insert(10);
    target.unit_mut().world_mut().phase_shift_mut().insert(20);
    assert!(!seer.unit().can_see_or_detect_unit_with_context(
        target.unit(), true, true, false, bypasses,
    ));

    seer.unit_mut().world_mut().phase_shift_mut().clear();
    target.unit_mut().world_mut().phase_shift_mut().clear();
    target
        .unit_mut()
        .set_private_object_owner_like_cpp(ObjectGuid::new(1, 99));
    assert!(!seer.unit().can_see_or_detect_unit_with_context(
        target.unit(), true, true, false, bypasses,
    ));

    target
        .unit_mut()
        .set_private_object_owner_like_cpp(ObjectGuid::EMPTY);
    target
        .unit_mut()
        .world_mut()
        .get_or_create_smooth_phasing_like_cpp()
        .set_viewer_dependent_info_like_cpp(
            seer_guid,
            crate::SmoothPhasingInfoLikeCpp::default(),
        );
    assert!(!seer.unit().can_see_or_detect_unit_with_context(
        target.unit(), true, true, false, bypasses,
    ));

    target
        .unit_mut()
        .world_mut()
        .smooth_phasing_mut_like_cpp()
        .unwrap()
        .disable_replacement_for_seer_like_cpp(seer_guid);
    target
        .unit_mut()
        .set_object_id_visibility_conditions_met_like_cpp(false);
    assert!(!seer.unit().can_see_or_detect_unit_with_context(
        target.unit(), true, true, false, bypasses,
    ));
}

#[test]
fn always_visible_short_circuit_stays_before_distance_gate() {
    let seer = unit(21);
    let mut target = unit(22);
    target.set_always_visible_for_seer_like_cpp(true);

    assert!(seer.can_see_or_detect_unit_with_context(
        &target,
        false,
        false,
        false,
        context(false, false, false),
    ));
}

#[test]
fn corpse_exception_skips_only_ghost_gate() {
    let seer = player(31);
    let mut target = creature(32);
    target
        .unit_mut()
        .set_server_side_ghost_visibility_like_cpp(0x2);
    let corpse_exception = context(true, true, false);

    assert!(seer.unit().can_see_or_detect_unit_with_context(
        target.unit(),
        false,
        true,
        false,
        corpse_exception,
    ));

    target.unit_mut().set_invisible_due_to_despawn_like_cpp(true);
    assert!(!seer.unit().can_see_or_detect_unit_with_context(
        target.unit(),
        false,
        true,
        false,
        corpse_exception,
    ));

    target
        .unit_mut()
        .set_invisible_due_to_despawn_like_cpp(false);
    target.unit_mut().set_stealth_like_cpp(0, 10_000);
    target
        .unit_mut()
        .world_mut()
        .relocate(wow_core::Position::new(10.0, 0.0, 0.0, 0.0));
    assert!(!seer.unit().can_see_or_detect_unit_with_context(
        target.unit(),
        false,
        true,
        false,
        corpse_exception,
    ));
}

#[test]
fn corpse_exception_requires_a_player_seer_even_for_creature_targets() {
    let mut seer = creature(33);
    let mut target = creature(34);
    seer.unit_mut()
        .set_server_side_ghost_visibility_detect_like_cpp(0x1);
    target
        .unit_mut()
        .set_server_side_ghost_visibility_like_cpp(0x2);

    assert!(!seer.unit().can_see_or_detect_unit_with_context(
        target.unit(),
        false,
        true,
        false,
        context(true, true, false),
    ));
}

#[test]
fn group_exception_requires_actual_player_objects_and_player_seer() {
    let mut seer = player(41);
    let mut target = player(42);
    seer.unit_mut()
        .set_server_side_ghost_visibility_detect_like_cpp(0x1);
    target
        .unit_mut()
        .set_server_side_ghost_visibility_like_cpp(0x2);
    assert_eq!(seer.unit().server_side_ghost_visibility_detect(), 0x1);
    assert_eq!(target.unit().server_side_ghost_visibility(), 0x2);

    let group_exception = context(true, false, true);
    assert!(seer.unit().can_see_or_detect_unit_with_context(
        target.unit(),
        true,
        true,
        false,
        group_exception,
    ));
    assert!(!seer.unit().can_see_or_detect_unit_with_context(
        target.unit(),
        true,
        false,
        false,
        group_exception,
    ));

    let mut creature_target = creature(43);
    creature_target
        .unit_mut()
        .set_server_side_ghost_visibility_like_cpp(0x2);
    assert!(!seer.unit().can_see_or_detect_unit_with_context(
        creature_target.unit(),
        true,
        true,
        false,
        group_exception,
    ));

    let mut creature_seer = creature(44);
    creature_seer
        .unit_mut()
        .set_server_side_ghost_visibility_detect_like_cpp(0x1);
    assert!(!creature_seer.unit().can_see_or_detect_unit_with_context(
        target.unit(),
        true,
        true,
        false,
        group_exception,
    ));
}
