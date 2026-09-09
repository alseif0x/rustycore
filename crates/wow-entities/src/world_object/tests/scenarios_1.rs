//! World-object visibility and location model regression scenarios, part 1 of 1.
//!
//! Moved out of the world_object.rs root under #648; every test is unchanged.

use super::*;

#[test]
fn world_location_defaults_match_cpp_world_location() {
    let location = WorldLocation::default();

    assert_eq!(location.map_id(), MAPID_INVALID);
    assert_eq!(location.position(), Position::ZERO);
}

#[test]
fn world_object_constructor_matches_cpp_base_state() {
    let object = WorldObject::new(true, TypeId::Unit, TypeMask::UNIT);

    assert_eq!(object.object().type_id(), TypeId::Unit);
    assert_eq!(object.object().type_mask(), TypeMask::UNIT);
    assert_eq!(object.map_id(), MAPID_INVALID);
    assert_eq!(object.instance_id(), 0);
    assert!(!object.has_current_map());
    assert!(object.is_world_object());
    assert!(!object.is_active());
    assert!(!object.is_far_visible());
    assert_eq!(object.zone_id(), 0);
    assert_eq!(object.area_id(), 0);
    assert_eq!(object.db_phase(), 0);
    assert_eq!(object.combat_reach(), 0.0);
    assert_eq!(object.collision_height_like_cpp(), 0.0);
    assert_eq!(object.static_floor_z(), INVALID_HEIGHT);
    assert_eq!(object.current_cell(), None);
    assert!(object.smooth_phasing_like_cpp().is_none());
}

#[test]
fn smooth_phasing_storage_matches_cpp_single_and_viewer_dependent_shape() {
    let seer = ObjectGuid::create_player(1, 1);
    let other_seer = ObjectGuid::create_player(1, 2);
    let replacement = ObjectGuid::new(1, 3);
    let mut smooth_phasing = SmoothPhasingLikeCpp::default();

    assert!(smooth_phasing.info_for_seer_like_cpp(seer).is_some());
    assert!(!smooth_phasing.is_being_replaced_for_seer_like_cpp(seer));

    smooth_phasing.set_single_info_like_cpp(SmoothPhasingInfoLikeCpp {
        replace_object: Some(replacement),
        ..SmoothPhasingInfoLikeCpp::default()
    });
    assert!(smooth_phasing.is_replacing_like_cpp(replacement));
    assert!(smooth_phasing.info_for_seer_like_cpp(other_seer).is_some());

    smooth_phasing.set_viewer_dependent_info_like_cpp(
        seer,
        SmoothPhasingInfoLikeCpp {
            replace_object: Some(replacement),
            ..SmoothPhasingInfoLikeCpp::default()
        },
    );
    assert!(smooth_phasing.is_being_replaced_for_seer_like_cpp(seer));
    assert!(smooth_phasing.info_for_seer_like_cpp(other_seer).is_none());

    smooth_phasing.disable_replacement_for_seer_like_cpp(seer);
    assert!(!smooth_phasing.is_being_replaced_for_seer_like_cpp(seer));

    smooth_phasing.clear_viewer_dependent_info_like_cpp(seer);
    assert!(smooth_phasing.info_for_seer_like_cpp(seer).is_none());
}

#[test]
fn relocate_normalizes_orientation_like_cpp_position() {
    let mut object = WorldObject::new(false, TypeId::GameObject, TypeMask::GAME_OBJECT);

    object.world_relocate(571, Position::new(1.0, 2.0, 3.0, -1.0));

    assert_eq!(object.map_id(), 571);
    assert!((object.position().orientation - (TAU - 1.0)).abs() < 0.0001);
    assert_eq!(object.object().map_id(), Some(571));
}

#[test]
fn set_map_and_reset_map_follow_cpp_binding_rules() {
    let mut object = WorldObject::new(false, TypeId::Corpse, TypeMask::CORPSE);

    assert_eq!(object.set_map(1, 10), Ok(()));
    assert_eq!(object.map_id(), 1);
    assert_eq!(object.instance_id(), 10);
    assert!(object.has_current_map());
    assert_eq!(object.set_map(1, 10), Ok(()));
    assert!(matches!(
        object.set_map(2, 10),
        Err(MapBindingError::AlreadyBound { .. })
    ));

    object.object_mut().add_to_world();
    assert_eq!(object.reset_map(), Err(MapBindingError::ObjectInWorld));
    object.object_mut().remove_from_world();

    assert_eq!(object.reset_map(), Ok(()));
    assert!(!object.has_current_map());
    assert_eq!(object.map_id(), 1);
    assert_eq!(object.instance_id(), 10);
}

#[test]
fn distance_helpers_subtract_combat_reach_and_clamp_zero() {
    let mut a = WorldObject::new(false, TypeId::Unit, TypeMask::UNIT);
    let mut b = WorldObject::new(false, TypeId::Unit, TypeMask::UNIT);
    a.relocate(Position::xyz(0.0, 0.0, 0.0));
    b.relocate(Position::xyz(3.0, 4.0, 12.0));
    a.set_combat_reach(1.5);
    b.set_combat_reach(2.0);

    assert!((a.exact_distance(&b) - 13.0).abs() < 0.001);
    assert!((a.distance(&b) - 9.5).abs() < 0.001);
    assert!((a.distance_2d(&b) - 1.5).abs() < 0.001);
    assert!((a.distance_z(&b) - 8.5).abs() < 0.001);

    b.relocate(Position::xyz(1.0, 0.0, 0.0));
    assert_eq!(a.distance(&b), 0.0);
}

#[test]
fn within_dist_uses_cpp_strict_less_than_and_radius_options() {
    let mut a = WorldObject::new(false, TypeId::Unit, TypeMask::UNIT);
    let mut b = WorldObject::new(false, TypeId::Unit, TypeMask::UNIT);
    a.relocate(Position::xyz(0.0, 0.0, 0.0));
    b.relocate(Position::xyz(3.0, 4.0, 0.0));

    assert!(!a.is_within_dist(&b, 5.0, true, false, false));
    assert!(a.is_within_dist(&b, 5.01, true, false, false));

    a.set_combat_reach(1.0);
    b.set_combat_reach(1.0);
    assert!(a.is_within_dist(&b, 3.1, true, true, true));
    assert!(!a.is_within_dist(&b, 3.1, true, false, false));
}

#[test]
fn within_dist_in_map_requires_world_map_and_phase() {
    let mut a = WorldObject::new(false, TypeId::Unit, TypeMask::UNIT);
    let mut b = WorldObject::new(false, TypeId::Unit, TypeMask::UNIT);
    a.set_map(530, 1).unwrap();
    b.set_map(530, 1).unwrap();
    a.object_mut().add_to_world();
    b.object_mut().add_to_world();
    a.relocate(Position::xyz(0.0, 0.0, 0.0));
    b.relocate(Position::xyz(1.0, 0.0, 0.0));

    a.phase_shift_mut().insert(10);
    b.phase_shift_mut().insert(20);
    assert!(!a.is_within_dist_in_map(&b, 2.0, true));

    b.phase_shift_mut().insert(10);
    assert!(a.is_within_dist_in_map(&b, 2.0, true));
}

#[test]
fn phase_shift_visible_map_ids_reference_count_like_cpp() {
    let mut phase_shift = PhaseShift::default();

    assert!(phase_shift.add_visible_map_id_like_cpp(609, 1));
    assert!(!phase_shift.add_visible_map_id_like_cpp(609, 1));
    assert!(phase_shift.has_visible_map_id_like_cpp(609));
    assert_eq!(phase_shift.visible_map_id_count_like_cpp(), 1);
    assert_eq!(
        phase_shift
            .visible_map_id_ref_like_cpp(609)
            .map(VisibleMapIdRef::references),
        Some(2)
    );
    assert_eq!(
        phase_shift.visible_map_ids_like_cpp().collect::<Vec<_>>(),
        vec![609]
    );

    assert!(!phase_shift.remove_visible_map_id_like_cpp(609));
    assert_eq!(
        phase_shift
            .visible_map_id_ref_like_cpp(609)
            .map(VisibleMapIdRef::references),
        Some(1)
    );
    assert!(phase_shift.remove_visible_map_id_like_cpp(609));
    assert!(!phase_shift.has_visible_map_id_like_cpp(609));
    assert!(!phase_shift.remove_visible_map_id_like_cpp(609));
}

#[test]
fn phase_shift_can_remove_all_phase_and_visible_map_references_like_cpp() {
    let mut phase_shift = PhaseShift::default();
    phase_shift.add_phase_like_cpp(20, PhaseFlags::PERSONAL, 2);
    phase_shift.add_visible_map_id_like_cpp(609, 2);

    let removed_phase = phase_shift
        .remove_phase_all_references_like_cpp(20)
        .expect("phase should be removed");
    assert_eq!(removed_phase.references(), 2);
    assert!(!phase_shift.has_phase_like_cpp(20));
    assert!(!phase_shift.has_personal_phase_like_cpp());

    let removed_visible_map = phase_shift
        .remove_visible_map_id_all_references_like_cpp(609)
        .expect("visible map should be removed");
    assert_eq!(removed_visible_map.references(), 2);
    assert!(!phase_shift.has_visible_map_id_like_cpp(609));
}

#[test]
fn phase_shift_clear_removes_visible_map_ids_like_cpp() {
    let mut phase_shift = PhaseShift::from_phases([10]);
    phase_shift.add_visible_map_id_like_cpp(609, 1);
    phase_shift.add_ui_map_phase_id_like_cpp(42, 1);

    phase_shift.clear();

    assert!(!phase_shift.has_visible_map_id_like_cpp(609));
    assert!(phase_shift.visible_map_ids_like_cpp().next().is_none());
    assert!(!phase_shift.has_ui_map_phase_id_like_cpp(42));
    assert!(phase_shift.ui_map_phase_ids_like_cpp().next().is_none());
    assert!(phase_shift.can_see(&PhaseShift::default()));
    assert!(!phase_shift.can_see(&PhaseShift::from_phases([20])));
}

#[test]
fn phase_shift_ui_map_phase_ids_reference_count_like_cpp() {
    let mut phase_shift = PhaseShift::default();

    assert!(phase_shift.add_ui_map_phase_id_like_cpp(42, 1));
    assert!(!phase_shift.add_ui_map_phase_id_like_cpp(42, 1));
    assert!(phase_shift.has_ui_map_phase_id_like_cpp(42));
    assert_eq!(
        phase_shift
            .ui_map_phase_id_ref_like_cpp(42)
            .map(UiMapPhaseIdRef::references),
        Some(2)
    );
    assert_eq!(
        phase_shift.ui_map_phase_ids_like_cpp().collect::<Vec<_>>(),
        vec![42]
    );

    assert!(!phase_shift.remove_ui_map_phase_id_like_cpp(42));
    assert_eq!(
        phase_shift
            .ui_map_phase_id_ref_like_cpp(42)
            .map(UiMapPhaseIdRef::references),
        Some(1)
    );
    assert!(phase_shift.remove_ui_map_phase_id_like_cpp(42));
    assert!(!phase_shift.has_ui_map_phase_id_like_cpp(42));
    assert!(!phase_shift.remove_ui_map_phase_id_like_cpp(42));
}

#[test]
fn angle_helpers_match_cpp_position_relative_angle_semantics() {
    let mut object = WorldObject::new(false, TypeId::Unit, TypeMask::UNIT);
    object.relocate(Position::new(0.0, 0.0, 0.0, PI / 2.0));

    assert!((object.absolute_angle_to_position(Position::xyz(1.0, 0.0, 0.0)) - 0.0).abs() < 0.0001);
    assert!((object.to_absolute_angle(PI / 2.0) - PI).abs() < 0.0001);
    assert!((object.to_relative_angle(0.0) - (TAU - PI / 2.0)).abs() < 0.0001);
    assert!((object.relative_angle_to_position(Position::xyz(0.0, 1.0, 0.0)) - 0.0).abs() < 0.0001);
}

#[test]
fn arc_front_back_and_line_helpers_match_cpp_boundaries() {
    let mut source = WorldObject::new(false, TypeId::Unit, TypeMask::UNIT);
    let mut front = WorldObject::new(false, TypeId::Unit, TypeMask::UNIT);
    let mut side = WorldObject::new(false, TypeId::Unit, TypeMask::UNIT);
    let mut back = WorldObject::new(false, TypeId::Unit, TypeMask::UNIT);
    source.relocate(Position::new(0.0, 0.0, 0.0, 0.0));
    front.relocate(Position::xyz(1.0, 0.0, 0.0));
    side.relocate(Position::xyz(0.0, 1.0, 0.0));
    back.relocate(Position::xyz(-1.0, 0.0, 0.0));

    assert!(source.has_in_arc(PI, &source, 2.0));
    assert!(source.is_in_front(&front, PI));
    assert!(source.is_in_front(&side, PI));
    assert!(!source.is_in_front(&back, PI));
    assert!(source.is_in_back(&back, PI));
    assert!(!source.is_in_back(&front, PI));

    front.relocate(Position::xyz(10.0, 1.0, 0.0));
    assert!(source.has_in_line(&front, 2.0));
    front.relocate(Position::xyz(10.0, 3.0, 0.0));
    assert!(!source.has_in_line(&front, 2.0));
}

#[test]
fn box_and_double_vertical_cylinder_match_cpp_geometry() {
    let mut object = WorldObject::new(false, TypeId::Unit, TypeMask::UNIT);
    let center = Position::new(0.0, 0.0, 5.0, PI / 2.0);

    object.relocate(Position::xyz(0.5, 1.5, 5.5));
    assert!(object.is_within_box(center, 2.0, 1.0, 1.0));

    object.relocate(Position::xyz(1.5, 1.5, 5.5));
    assert!(!object.is_within_box(center, 2.0, 1.0, 1.0));

    object.relocate(Position::xyz(3.0, 4.0, 8.0));
    assert!(!object.is_within_double_vertical_cylinder(center, 5.0, 3.0));
    object.relocate(Position::xyz(3.0, 3.9, 8.0));
    assert!(object.is_within_double_vertical_cylinder(center, 5.0, 3.0));
}

#[test]
fn world_object_visibility_range_uses_override_far_visible_then_map_range() {
    let environment = TestEnvironment {
        visibility_range: 123.0,
        ..TestEnvironment::default()
    };
    let mut object = WorldObject::new(false, TypeId::GameObject, TypeMask::GAME_OBJECT);
    assert_eq!(object.get_visibility_range(&environment), 123.0);

    object.set_far_visible(true);
    assert_eq!(
        object.get_visibility_range(&environment),
        MAX_VISIBILITY_DISTANCE
    );

    let environment = TestEnvironment {
        visibility_override: Some(222.0),
        ..environment
    };
    assert_eq!(object.get_visibility_range(&environment), 222.0);

    let mut player = WorldObject::new(false, TypeId::Player, TypeMask::PLAYER | TypeMask::UNIT);
    player.set_far_visible(true);
    assert_eq!(player.get_visibility_range(&environment), 123.0);
}

#[test]
fn world_object_sight_range_matches_representable_cpp_cases() {
    let environment = TestEnvironment {
        visibility_range: 140.0,
        creature_sight_distance: Some(80.0),
        ..TestEnvironment::default()
    };
    let player = WorldObject::new(false, TypeId::Player, TypeMask::PLAYER | TypeMask::UNIT);
    let creature = WorldObject::new(false, TypeId::Unit, TypeMask::UNIT);
    let unit = WorldObject::new(false, TypeId::Unit, TypeMask::UNIT);
    let mut dyn_object = WorldObject::new(false, TypeId::DynamicObject, TypeMask::DYNAMIC_OBJECT);
    dyn_object.set_active(true);

    assert_eq!(player.get_sight_range(None, &environment), 140.0);
    assert_eq!(creature.get_sight_range(None, &environment), 80.0);
    assert_eq!(
        unit.get_sight_range(None, &TestEnvironment::default()),
        SIGHT_RANGE_UNIT
    );
    assert_eq!(dyn_object.get_sight_range(None, &environment), 140.0);

    let mut target = WorldObject::new(false, TypeId::GameObject, TypeMask::GAME_OBJECT);
    target.set_far_visible(true);
    assert_eq!(
        player.get_sight_range(Some(&target), &environment),
        MAX_VISIBILITY_DISTANCE
    );

    let cinematic = TestEnvironment {
        visibility_range: 140.0,
        cinematic: true,
        ..TestEnvironment::default()
    };
    assert_eq!(
        player.get_sight_range(None, &cinematic),
        DEFAULT_VISIBILITY_INSTANCE
    );
}

#[test]
fn world_object_los_prefilters_map_not_phase_and_uses_partial_raw_endpoint_bridge() {
    let environment = TestEnvironment {
        los: false,
        ..TestEnvironment::default()
    };
    let mut source = WorldObject::new(false, TypeId::Unit, TypeMask::UNIT);
    let mut target = WorldObject::new(false, TypeId::Unit, TypeMask::UNIT);

    assert!(source.is_within_los(
        Position::xyz(1.0, 0.0, 0.0),
        &environment,
        LineOfSightOptions::default()
    ));
    assert_eq!(environment.los_calls.get(), 0);

    source.set_map(571, 1).unwrap();
    target.set_map(571, 2).unwrap();
    source.object_mut().add_to_world();
    target.object_mut().add_to_world();
    assert!(!source.is_within_los_in_map(&target, &environment, LineOfSightOptions::default()));
    assert_eq!(environment.los_calls.get(), 0);

    target.object_mut().remove_from_world();
    target.reset_map().unwrap();
    target.set_map(571, 1).unwrap();
    target.object_mut().add_to_world();
    source.phase_shift_mut().insert(10);
    target.phase_shift_mut().insert(20);
    assert!(!source.in_same_phase(&target));
    assert!(!source.is_within_los_in_map(&target, &environment, LineOfSightOptions::default()));
    assert_eq!(environment.los_calls.get(), 1);

    let environment = TestEnvironment {
        los: true,
        ..environment
    };
    assert!(source.is_within_los_in_map(&target, &environment, LineOfSightOptions::default()));
    assert_eq!(environment.los_calls.get(), 2);
}

#[test]
fn base_world_object_default_los_endpoints_remain_raw_without_adjustments() {
    let environment = TestEnvironment::default();
    let mut source = WorldObject::new(false, TypeId::Unit, TypeMask::UNIT);
    prepare_in_world(&mut source, Position::new(1.0, 2.0, 3.0, 0.25));

    assert_eq!(source.collision_height_like_cpp(), 0.0);
    assert_eq!(source.combat_reach(), 0.0);
    assert!(source.is_within_los(
        Position::new(4.0, 6.0, 8.0, 1.0),
        &environment,
        LineOfSightOptions::default()
    ));

    let query = last_los(&environment);
    assert!(!query.has_target);
    assert_position_close(query.from.position, Position::new(1.0, 2.0, 3.0, 0.0));
    assert_position_close(query.to.position, Position::new(4.0, 6.0, 8.0, 1.0));
    assert!(!query.from.collision_height_adjusted);
    assert!(!query.from.hit_sphere_adjusted);
    assert!(!query.to.collision_height_adjusted);
    assert!(!query.to.hit_sphere_adjusted);
}

#[test]
fn point_los_non_player_uses_collision_height_and_hit_sphere_like_cpp() {
    let environment = TestEnvironment::default();
    let mut source = WorldObject::new(false, TypeId::Unit, TypeMask::UNIT);
    prepare_in_world(&mut source, Position::xyz(0.0, 0.0, 0.0));
    source.set_collision_height_like_cpp(2.0);
    source.set_combat_reach(1.0);

    assert!(source.is_within_los(
        Position::xyz(4.0, 0.0, 0.0),
        &environment,
        LineOfSightOptions::default()
    ));

    let query = last_los(&environment);
    assert_position_close(query.to.position, Position::xyz(4.0, 0.0, 2.0));
    assert!(query.to.collision_height_adjusted);
    assert!(!query.to.hit_sphere_adjusted);
    assert_position_close(query.from.position, Position::xyz(1.0, 0.0, 2.0));
    assert!(query.from.collision_height_adjusted);
    assert!(query.from.hit_sphere_adjusted);
}

#[test]
fn los_in_map_non_player_endpoints_use_opposite_collision_height_destinations() {
    let environment = TestEnvironment::default();
    let mut source = WorldObject::new(false, TypeId::Unit, TypeMask::UNIT);
    let mut target = WorldObject::new(false, TypeId::Unit, TypeMask::UNIT);
    prepare_in_world(&mut source, Position::xyz(0.0, 0.0, 0.0));
    prepare_in_world(&mut target, Position::xyz(10.0, 0.0, 0.0));
    source.set_collision_height_like_cpp(2.0);
    source.set_combat_reach(1.0);
    target.set_collision_height_like_cpp(4.0);
    target.set_combat_reach(2.0);

    assert!(source.is_within_los_in_map(&target, &environment, LineOfSightOptions::default()));

    let query = last_los(&environment);
    assert!(query.has_target);
    assert_position_close(
        query.from.position,
        Position::xyz(0.9805807, 0.0, 2.1961162),
    );
    assert!(query.from.collision_height_adjusted);
    assert!(query.from.hit_sphere_adjusted);
    assert_position_close(
        query.to.position,
        Position::new(8.038839, 0.0, 3.6077678, PI),
    );
    assert!(query.to.collision_height_adjusted);
    assert!(query.to.hit_sphere_adjusted);
}

#[test]
fn los_in_map_target_player_uses_source_collision_height_cpp_quirk() {
    let environment = TestEnvironment::default();
    let mut source = WorldObject::new(false, TypeId::Unit, TypeMask::UNIT);
    let mut target = WorldObject::new(false, TypeId::Player, TypeMask::PLAYER | TypeMask::UNIT);
    prepare_in_world(&mut source, Position::xyz(0.0, 0.0, 0.0));
    prepare_in_world(&mut target, Position::xyz(5.0, 0.0, 10.0));
    source.set_collision_height_like_cpp(2.0);
    source.set_combat_reach(0.0);
    target.set_collision_height_like_cpp(7.0);

    assert!(source.is_within_los_in_map(&target, &environment, LineOfSightOptions::default()));

    let query = last_los(&environment);
    assert_position_close(query.to.position, Position::xyz(5.0, 0.0, 12.0));
    assert!(query.to.collision_height_adjusted);
    assert!(!query.to.hit_sphere_adjusted);
    assert_position_close(query.from.position, Position::xyz(0.0, 0.0, 2.0));
    assert!(query.from.collision_height_adjusted);
    assert!(!query.from.hit_sphere_adjusted);
}

#[test]
fn player_source_los_endpoint_uses_position_plus_collision_height_not_hit_sphere() {
    let environment = TestEnvironment::default();
    let mut source = WorldObject::new(false, TypeId::Player, TypeMask::PLAYER | TypeMask::UNIT);
    prepare_in_world(&mut source, Position::xyz(1.0, 2.0, 3.0));
    source.set_collision_height_like_cpp(2.5);
    source.set_combat_reach(3.0);

    assert!(source.is_within_los(
        Position::xyz(11.0, 2.0, 3.0),
        &environment,
        LineOfSightOptions::default()
    ));

    let query = last_los(&environment);
    assert_position_close(query.from.position, Position::xyz(1.0, 2.0, 5.5));
    assert!(query.from.collision_height_adjusted);
    assert!(!query.from.hit_sphere_adjusted);
    assert_position_close(query.to.position, Position::xyz(11.0, 2.0, 5.5));
    assert!(query.to.collision_height_adjusted);
    assert!(!query.to.hit_sphere_adjusted);
}

#[test]
fn world_object_height_floor_and_ground_update_use_map_bridge() {
    let environment = TestEnvironment {
        height: 20.0,
        floor: 30.0,
        ..TestEnvironment::default()
    };
    let mut unit = WorldObject::new(false, TypeId::Unit, TypeMask::UNIT);
    unit.set_map(571, 1).unwrap();
    unit.object_mut().add_to_world();
    unit.relocate(Position::xyz(1.0, 2.0, 10.0));

    assert_eq!(
        unit.get_map_height(
            &environment,
            1.0,
            2.0,
            10.0,
            WorldObjectHeightQuery::default()
        ),
        20.0
    );
    assert_eq!(
        unit.update_ground_position_z(&environment, 1.0, 2.0, 10.0, 1.25),
        21.25
    );

    unit.set_static_floor_z(25.0);
    assert_eq!(unit.get_floor_z(&environment), 30.0);

    let environment = TestEnvironment {
        height: INVALID_HEIGHT,
        floor: INVALID_HEIGHT,
        ..environment
    };
    assert_eq!(
        unit.update_ground_position_z(&environment, 1.0, 2.0, 10.0, 1.25),
        10.0
    );
    assert_eq!(unit.get_floor_z(&environment), 25.0);
}

#[test]
fn update_allowed_position_z_matches_cpp_branches() {
    let environment = TestEnvironment {
        height: 20.0,
        ..TestEnvironment::default()
    };
    let mut unit = WorldObject::new(false, TypeId::Unit, TypeMask::UNIT);
    unit.set_map(571, 1).unwrap();
    unit.object_mut().add_to_world();
    unit.relocate(Position::xyz(1.0, 2.0, 10.0));

    // Grounded unit below ground (z=10 < ground=20): snap up to ground+hover.
    let walk = AllowedPositionZCaps {
        on_transport: false,
        can_fly: false,
        can_swim: false,
        hover_offset: 1.25,
    };
    assert_eq!(
        unit.update_allowed_position_z_like_cpp(&environment, walk, 1.0, 2.0, 10.0),
        21.25
    );

    // On a transport: Z is owned by the transport, left untouched.
    let on_transport = AllowedPositionZCaps {
        on_transport: true,
        ..walk
    };
    assert_eq!(
        unit.update_allowed_position_z_like_cpp(&environment, on_transport, 1.0, 2.0, 10.0),
        10.0
    );

    // Flying unit *above* ground stays put (raise-only). Ground here is 5.0.
    let low_ground = TestEnvironment {
        height: 5.0,
        ..TestEnvironment::default()
    };
    let fly = AllowedPositionZCaps {
        on_transport: false,
        can_fly: true,
        can_swim: false,
        hover_offset: 0.0,
    };
    assert_eq!(
        unit.update_allowed_position_z_like_cpp(&low_ground, fly, 1.0, 2.0, 10.0),
        10.0
    );
    // Flying unit *below* ground is lifted to ground+hover.
    assert_eq!(
        unit.update_allowed_position_z_like_cpp(&environment, fly, 1.0, 2.0, 10.0),
        20.0
    );

    // No terrain under the probe: Z unchanged.
    let no_terrain = TestEnvironment {
        height: INVALID_HEIGHT,
        ..TestEnvironment::default()
    };
    assert_eq!(
        unit.update_allowed_position_z_like_cpp(&no_terrain, walk, 1.0, 2.0, 10.0),
        10.0
    );

    // Non-unit (GameObject): snapped flat to ground, no hover.
    let mut gameobject = WorldObject::new(false, TypeId::GameObject, TypeMask::GAME_OBJECT);
    gameobject.set_map(571, 1).unwrap();
    gameobject.object_mut().add_to_world();
    gameobject.relocate(Position::xyz(1.0, 2.0, 10.0));
    assert_eq!(
        gameobject.update_allowed_position_z_like_cpp(&environment, walk, 1.0, 2.0, 10.0),
        20.0
    );
}

#[test]
fn world_object_transport_relocation_roundtrips_offset_and_global_position() {
    let transport = Position::new(100.0, 200.0, 10.0, PI / 2.0);
    let offset = Position::new(4.0, -2.0, 3.0, PI / 4.0);
    let mut passenger = WorldObject::new(false, TypeId::Unit, TypeMask::UNIT);

    let global = passenger.relocate_on_transport(transport, offset);
    assert_eq!(passenger.position(), global);

    let roundtrip = passenger.transport_offset_from_position(transport);
    assert!((roundtrip.x - offset.x).abs() < 0.0001);
    assert!((roundtrip.y - offset.y).abs() < 0.0001);
    assert!((roundtrip.z - offset.z).abs() < 0.0001);
    assert!((roundtrip.orientation - offset.orientation).abs() < 0.0001);
}

#[test]
fn phase_shift_phase_refs_update_flags_like_cpp() {
    let mut phase_shift = PhaseShift::default();

    assert!(
        phase_shift
            .flags_like_cpp()
            .contains(PhaseShiftFlags::UNPHASED)
    );
    assert!(phase_shift.add_phase_like_cpp(10, PhaseFlags::NONE, 1));
    assert!(phase_shift.has_phase_like_cpp(10));
    assert!(
        !phase_shift
            .flags_like_cpp()
            .contains(PhaseShiftFlags::UNPHASED)
    );
    assert_eq!(
        phase_shift.phase_ref_like_cpp(10).map(PhaseRef::references),
        Some(1)
    );

    assert!(!phase_shift.add_phase_like_cpp(10, PhaseFlags::NONE, 1));
    assert_eq!(
        phase_shift.phase_ref_like_cpp(10).map(PhaseRef::references),
        Some(2)
    );
    assert!(!phase_shift.remove_phase_like_cpp(10));
    assert!(phase_shift.remove_phase_like_cpp(10));
    assert!(
        phase_shift
            .flags_like_cpp()
            .contains(PhaseShiftFlags::UNPHASED)
    );
}

#[test]
fn phase_shift_can_see_honors_always_visible_like_cpp() {
    let viewer = PhaseShift::from_phases([10]);
    let mut target = PhaseShift::from_phases([20]);

    assert!(!viewer.can_see(&target));
    target.set_always_visible_like_cpp(true);
    assert!(viewer.can_see(&target));
}

#[test]
fn phase_shift_can_see_requires_matching_personal_guid_like_cpp() {
    let personal_owner = ObjectGuid::create_player(1, 42);
    let other_owner = ObjectGuid::create_player(1, 43);
    let mut viewer = PhaseShift::default();
    let mut target = PhaseShift::default();

    viewer.add_phase_like_cpp(10, PhaseFlags::PERSONAL, 1);
    target.add_phase_like_cpp(10, PhaseFlags::PERSONAL, 1);
    viewer.set_personal_guid_like_cpp(personal_owner);
    target.set_personal_guid_like_cpp(other_owner);
    assert!(viewer.has_personal_phase_like_cpp());
    assert!(!viewer.can_see(&target));

    target.set_personal_guid_like_cpp(personal_owner);
    assert!(viewer.can_see(&target));
}

#[test]
fn phase_shift_can_see_honors_inverse_like_cpp() {
    let normal = PhaseShift::from_phases([10]);
    let mut inverse_same = PhaseShift::from_phases([10]);
    inverse_same.set_inversed_like_cpp(true);
    assert!(!normal.can_see(&inverse_same));

    let mut inverse_other = PhaseShift::from_phases([20]);
    inverse_other.set_inversed_like_cpp(true);
    assert!(normal.can_see(&inverse_other));

    let mut unphased = PhaseShift::default();
    let mut inverse_unphased = PhaseShift::default();
    inverse_unphased.set_inversed_like_cpp(true);
    assert!(!unphased.can_see(&inverse_unphased));

    unphased.set_always_visible_like_cpp(true);
    assert!(unphased.can_see(&inverse_unphased));
}
