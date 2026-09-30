use super::*;
use wow_constants::PhaseFlags;

fn unit(low: u64) -> Unit {
    let mut unit = Unit::new(true);
    unit.world_mut()
        .object_mut()
        .create(ObjectGuid::new(1, low));
    unit
}

fn sees(seer: &Unit, target: &UnitVisibilityTargetFacts) -> bool {
    seer.can_see_or_detect_target(target, false, true, false)
}

#[test]
fn capture_keeps_target_invisibility_and_observes_later_seer_detection() {
    let mut seer = unit(1);
    let mut target = unit(2);
    target.set_invisibility_like_cpp(37, 25);
    let captured = target.capture_visibility_target();
    target.set_invisibility_like_cpp(37, 0);
    target.set_never_visible_for_seer_like_cpp(true);
    assert!(!sees(&seer, &captured));
    seer.set_invisibility_detect_like_cpp(37, 24);
    assert!(!sees(&seer, &captured));
    seer.set_invisibility_detect_like_cpp(37, 25);
    assert!(sees(&seer, &captured));
    assert!(!seer.can_see_or_detect_unit_like_cpp(&target, false, true, false));
}

#[test]
fn capture_keeps_smooth_replacement_and_uses_later_seer_guid() {
    let mut seer = unit(1);
    let mut target = unit(3);
    let later_guid = ObjectGuid::new(1, 2);
    target.set_always_detectable_for_seer_like_cpp(true);
    target
        .world_mut()
        .get_or_create_smooth_phasing_like_cpp()
        .set_viewer_dependent_info_like_cpp(later_guid, crate::SmoothPhasingInfoLikeCpp::default());
    let captured = target.capture_visibility_target();
    target
        .world_mut()
        .smooth_phasing_mut_like_cpp()
        .unwrap()
        .disable_replacement_for_seer_like_cpp(later_guid);
    assert!(sees(&seer, &captured));
    seer.world_mut().object_mut().create(later_guid);
    assert!(!sees(&seer, &captured));
    assert!(seer.can_see_or_detect_unit_like_cpp(&target, false, true, false));
}

#[test]
fn capture_keeps_stalked_casters_and_uses_later_seer_guid() {
    let mut seer = unit(1);
    let mut target = unit(3);
    let later_guid = ObjectGuid::new(1, 2);
    target.set_invisibility_like_cpp(0, 100);
    target
        .subsystems_mut()
        .auras
        .register_applied_aura_type_like_cpp(
            AppliedAuraRef::new(53338, later_guid, 0, 1),
            SPELL_AURA_MOD_STALKED_LIKE_CPP,
        );
    let captured = target.capture_visibility_target();
    target
        .subsystems_mut()
        .auras
        .remove_auras_by_type_like_cpp(SPELL_AURA_MOD_STALKED_LIKE_CPP);
    assert!(!sees(&seer, &captured));
    seer.world_mut().object_mut().create(later_guid);
    assert!(sees(&seer, &captured));
    assert!(!seer.can_see_or_detect_unit_like_cpp(&target, false, true, false));
}

#[test]
fn never_visible_precedes_always_visible_but_same_nonempty_guid_precedes_both() {
    let mut seer = unit(1);
    let mut target = unit(2);
    target.set_never_visible_for_seer_like_cpp(true);
    target.set_always_visible_for_seer_like_cpp(true);
    let captured = target.capture_visibility_target();
    assert!(!sees(&seer, &captured));
    seer.world_mut().object_mut().create(target.world().guid());
    assert!(sees(&seer, &captured));
    assert!(seer.can_see_or_detect_unit_like_cpp(&target, false, true, false));
}

#[test]
fn empty_guid_is_not_same_guid_shortcut_and_some_empty_owner_is_preserved() {
    let seer = Unit::new(true);
    let mut target = Unit::new(true);
    target.set_invisibility_like_cpp(0, 100);
    assert!(!sees(&seer, &target.capture_visibility_target()));
    target
        .subsystems_mut()
        .control
        .set_owner_guid(Some(ObjectGuid::EMPTY));
    let captured = target.capture_visibility_target();
    target.subsystems_mut().control.set_owner_guid(None);
    assert!(sees(&seer, &captured));
    assert!(!sees(&seer, &target.capture_visibility_target()));
}

#[test]
fn charmer_precedes_owner_and_owner_group_visibility_precedes_private_gate() {
    let seer = unit(1);
    let mut target = unit(2);
    target
        .subsystems_mut()
        .control
        .set_owner_guid(Some(seer.world().guid()));
    target
        .subsystems_mut()
        .control
        .set_charmer(ObjectGuid::new(1, 3), false);
    target.set_invisibility_like_cpp(0, 100);
    assert!(!sees(&seer, &target.capture_visibility_target()));
    target.subsystems_mut().control.remove_charmer();
    assert!(sees(&seer, &target.capture_visibility_target()));
    target.subsystems_mut().control.set_owner_guid(None);
    target.set_private_object_owner_like_cpp(ObjectGuid::new(1, 4));
    target.set_target_owner_group_visible_for_seer_like_cpp(true);
    assert!(sees(&seer, &target.capture_visibility_target()));
}

#[test]
fn private_owner_uses_later_seer_state_and_bypasses_only_object_conditions() {
    let mut seer = unit(1);
    let mut target = unit(2);
    let owner = ObjectGuid::new(1, 3);
    target.set_private_object_owner_like_cpp(owner);
    target.set_object_id_visibility_conditions_met_like_cpp(false);
    let captured = target.capture_visibility_target();
    assert!(!sees(&seer, &captured));
    seer.set_seer_private_object_owner_like_cpp(owner);
    assert!(sees(&seer, &captured));
    seer.set_seer_private_object_owner_like_cpp(ObjectGuid::EMPTY);
    seer.set_seer_group_visible_for_private_owner_like_cpp(true);
    assert!(sees(&seer, &captured));
    target.set_private_object_owner_like_cpp(ObjectGuid::EMPTY);
    assert!(!sees(&seer, &target.capture_visibility_target()));
}

#[test]
fn gm_visibility_precedes_ghost_despawn_and_detection_but_not_conditions() {
    let mut seer = unit(1);
    let mut target = unit(2);
    target.set_server_side_gm_visibility_like_cpp(2);
    target.set_invisible_due_to_despawn_like_cpp(true);
    target.set_invisibility_like_cpp(0, 100);
    let captured = target.capture_visibility_target();
    seer.set_server_side_gm_visibility_detect_like_cpp(1);
    assert!(!sees(&seer, &captured));
    seer.set_server_side_gm_visibility_detect_like_cpp(2);
    assert!(sees(&seer, &captured));
    target.set_object_id_visibility_conditions_met_like_cpp(false);
    assert!(!sees(&seer, &target.capture_visibility_target()));
    target.set_object_id_visibility_conditions_met_like_cpp(true);
    target.set_server_side_gm_visibility_like_cpp(0);
    assert!(sees(&seer, &target.capture_visibility_target()));
}

#[test]
fn ghost_and_despawn_precede_always_detectable_and_implicit_detection() {
    let mut seer = unit(1);
    let mut target = unit(2);
    target.set_server_side_ghost_visibility_like_cpp(2);
    target.set_always_detectable_for_seer_like_cpp(true);
    let captured = target.capture_visibility_target();
    assert!(!sees(&seer, &captured));
    assert!(!seer.can_see_or_detect_target(&captured, true, true, false));
    seer.set_server_side_ghost_visibility_detect_like_cpp(2);
    assert!(sees(&seer, &captured));
    target.set_invisible_due_to_despawn_like_cpp(true);
    assert!(!sees(&seer, &target.capture_visibility_target()));
    target.set_invisible_due_to_despawn_like_cpp(false);
    target.set_ghost_visible_to_seer_by_group_like_cpp(true);
    seer.set_server_side_ghost_visibility_detect_like_cpp(1);
    let group = target.capture_visibility_target();
    assert!(seer.can_see_or_detect_target(&group, false, true, false));
    assert!(!seer.can_see_or_detect_target(&group, false, false, false));
}

#[test]
fn map_binding_and_in_world_gate_is_conditional_on_both_bindings() {
    let mut seer = unit(1);
    let mut target = unit(2);
    seer.world_mut().set_map(571, 7).unwrap();
    seer.world_mut().object_mut().add_to_world();
    assert!(sees(&seer, &target.capture_visibility_target()));
    target.world_mut().set_map(571, 7).unwrap();
    let out_of_world = target.capture_visibility_target();
    assert!(!sees(&seer, &out_of_world));
    target.world_mut().object_mut().add_to_world();
    assert!(!sees(&seer, &out_of_world));
    assert!(sees(&seer, &target.capture_visibility_target()));
    let mut other_instance = unit(3);
    other_instance.world_mut().set_map(571, 8).unwrap();
    other_instance.world_mut().object_mut().add_to_world();
    assert!(!sees(&seer, &other_instance.capture_visibility_target()));
    let mut other_map = unit(4);
    other_map.world_mut().set_map(0, 7).unwrap();
    other_map.world_mut().object_mut().add_to_world();
    assert!(!sees(&seer, &other_map.capture_visibility_target()));
}

#[test]
fn phase_capture_keeps_personal_identity_and_observes_later_seer_phase() {
    let mut seer = unit(1);
    let mut target = unit(2);
    let phase_flags = PhaseFlags::PERSONAL;
    target
        .world_mut()
        .phase_shift_mut()
        .add_phase_like_cpp(42, phase_flags, 1);
    target
        .world_mut()
        .phase_shift_mut()
        .set_personal_guid_like_cpp(ObjectGuid::new(1, 3));
    let captured = target.capture_visibility_target();
    target.world_mut().phase_shift_mut().clear();
    assert!(!sees(&seer, &captured));
    seer.world_mut()
        .phase_shift_mut()
        .add_phase_like_cpp(42, phase_flags, 1);
    seer.world_mut()
        .phase_shift_mut()
        .set_personal_guid_like_cpp(ObjectGuid::new(1, 4));
    assert!(!sees(&seer, &captured));
    seer.world_mut()
        .phase_shift_mut()
        .set_personal_guid_like_cpp(ObjectGuid::new(1, 3));
    assert!(sees(&seer, &captured));
}

#[test]
fn high_invisibility_flags_are_not_truncated_to_the_38_value_slots() {
    let mut seer = unit(1);
    let mut target = unit(2);
    target.visibility_detection.invisibility_flags = 1_u64 << 63;
    let captured = target.capture_visibility_target();
    assert!(!seer.can_detect_invisibility_of_like_cpp(&target));
    assert!(!sees(&seer, &captured));
    seer.visibility_detection.invisibility_detect_flags = 1_u64 << 63;
    assert!(seer.can_detect_invisibility_of_like_cpp(&target));
    assert!(sees(&seer, &captured));
    target.set_invisibility_like_cpp(37, 8);
    assert!(!sees(&seer, &target.capture_visibility_target()));
    seer.set_invisibility_detect_like_cpp(37, 8);
    assert!(sees(&seer, &target.capture_visibility_target()));
}

#[test]
fn high_stealth_flags_still_apply_geometry_before_the_38_slot_loop() {
    let mut seer = unit(1);
    let mut target = unit(2);
    seer.data.combat_reach = 0.0;
    target.visibility_detection.stealth_flags = 1_u64 << 63;
    target
        .world_mut()
        .relocate(Position::new(-10.0, 0.0, 0.0, 0.0));
    assert!(!seer.can_detect_stealth_of_like_cpp(&target, true, false));
    assert!(!sees(&seer, &target.capture_visibility_target()));
    target
        .world_mut()
        .relocate(Position::new(10.0, 0.0, 0.0, 0.0));
    assert!(seer.can_detect_stealth_of_like_cpp(&target, true, false));
    assert!(sees(&seer, &target.capture_visibility_target()));
}

#[test]
fn legacy_empty_guid_self_reference_keeps_arc_identity_capture_is_distinct() {
    let mut seer = Unit::new(true);
    seer.data.combat_reach = 0.0;
    seer.world_mut()
        .relocate(Position::new(0.0, 0.0, 0.0, std::f32::consts::PI));
    seer.set_stealth_like_cpp(0, 1);
    assert!(seer.can_detect_stealth_of_like_cpp(&seer, true, false));
    assert!(seer.can_see_or_detect_unit_like_cpp(&seer, false, true, false));
    assert!(!sees(&seer, &seer.capture_visibility_target()));
    seer.world_mut().object_mut().create(ObjectGuid::new(1, 1));
    assert!(sees(&seer, &seer.capture_visibility_target()));
}

#[test]
fn capture_keeps_3d_position_while_seer_level_reach_and_alert_are_late() {
    let mut seer = unit(1);
    let mut target = unit(2);
    seer.data.combat_reach = 0.0;
    seer.set_level(1);
    target
        .world_mut()
        .relocate(Position::new(0.1, 0.0, 9.0, 0.0));
    target.set_stealth_like_cpp(0, 1);
    let captured = target.capture_visibility_target();
    target.world_mut().relocate(Position::ZERO);
    assert!(!sees(&seer, &captured));
    assert!(seer.can_see_or_detect_target(&captured, false, true, true));
    seer.set_level(2);
    assert!(sees(&seer, &captured));
    seer.set_level(1);
    seer.data.combat_reach = 10.0;
    assert!(sees(&seer, &captured));
}

#[test]
fn stealth_player_cap_and_implicit_detect_keep_their_original_positions() {
    let mut seer = unit(1);
    let mut target = unit(2);
    seer.set_level(80);
    target
        .world_mut()
        .relocate(Position::new(35.0, 0.0, 0.0, 0.0));
    target.set_stealth_like_cpp(0, 1);
    let captured = target.capture_visibility_target();
    assert!(!sees(&seer, &captured));
    assert!(seer.can_see_or_detect_target(&captured, false, false, false));
    assert!(seer.can_see_or_detect_target(&captured, true, true, false));
    seer.set_seer_can_never_see_target_like_cpp(true);
    assert!(!seer.can_see_or_detect_target(&captured, true, true, false));
}

#[test]
fn nan_distance_is_not_normalized_and_infinite_distance_is_rejected() {
    let mut seer = unit(1);
    let mut target = unit(2);
    seer.data.combat_reach = 0.0;
    target.set_stealth_like_cpp(0, 1);
    target
        .world_mut()
        .relocate(Position::new(1.0, 0.0, f32::NAN, 0.0));
    assert!(seer.can_detect_stealth_of_like_cpp(&target, true, false));
    assert!(sees(&seer, &target.capture_visibility_target()));
    target
        .world_mut()
        .relocate(Position::new(1.0, 0.0, f32::INFINITY, 0.0));
    assert!(!seer.can_detect_stealth_of_like_cpp(&target, true, false));
    assert!(!sees(&seer, &target.capture_visibility_target()));
}
