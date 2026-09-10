//! Spline regressions.
//!
//! Moved out of spline.rs under #685; every test is unchanged.

use super::*;

#[test]
fn flags_match_cpp_values_and_mutators() {
    assert_eq!(MoveSplineFlag::DONE.bits(), 0x0000_0020);
    assert_eq!(MoveSplineFlag::FALLING.bits(), 0x0000_0040);
    assert_eq!(MoveSplineFlag::CYCLIC.bits(), 0x0000_1000);
    assert_eq!(MoveSplineFlag::UNCOMPRESSED_PATH.bits(), 0x0040_0000);

    let mut flags = MoveSplineFlag::FALLING | MoveSplineFlag::FALLING_SLOW;
    flags.enable_parabolic();
    assert!(flags.contains(MoveSplineFlag::PARABOLIC));
    assert!(!flags.intersects(MoveSplineFlag::FALLING | MoveSplineFlag::FALLING_SLOW));

    flags.enable_transport_enter();
    flags.enable_transport_exit();
    assert!(flags.contains(MoveSplineFlag::TRANSPORT_EXIT));
    assert!(!flags.contains(MoveSplineFlag::TRANSPORT_ENTER));
}

#[test]
fn fall_math_matches_cpp_constants() {
    assert!((compute_fall_time(10.0, false) - 1.018_208).abs() < 0.000_01);
    assert!((compute_fall_elevation(1.0, false, 0.0) - 9.645_553).abs() < 0.000_01);
    assert!(compute_fall_time(-1.0, false).abs() < f32::EPSILON);
}

#[test]
fn jump_math_matches_cpp_motion_master_formulas() {
    assert!((compute_jump_max_height_like_cpp(10.0) - 2.591_868).abs() < 0.000_01);

    let mid = calculate_jump_speeds_like_cpp(20.0, 7.0, 7.0, 1.0, 2.0, 10.0);
    assert_eq!(mid.speed_xy, 21.0);
    assert!((mid.speed_z - 9.186_241).abs() < 0.000_01);

    let min_clamped = calculate_jump_speeds_like_cpp(1.0, 7.0, 7.0, 1.0, 2.0, 10.0);
    assert!((min_clamped.speed_z - 8.784_328).abs() < 0.000_01);

    let max_clamped = calculate_jump_speeds_like_cpp(100.0, 7.0, 7.0, 1.0, 2.0, 10.0);
    assert!((max_clamped.speed_z - 19.642_355).abs() < 0.000_01);
}

#[test]
fn init_args_validate_like_cpp() {
    let mut args = linear_args();
    assert_eq!(args.validate(), Ok(()));

    args.path[1] = Position::xyz(0.05, 0.0, 0.0);
    assert_eq!(
        args.validate(),
        Err(MoveSplineValidationError::PathSegmentTooShort)
    );

    args.facing.kind = MonsterMoveType::FacingAngle;
    assert_eq!(args.validate(), Ok(()));
}

#[test]
fn move_spline_init_launch_corrects_path_flags_and_speed_like_cpp() {
    let mut init = MoveSplineInit::new(77);
    init.move_to(Position::xyz(100.0, 0.0, 0.0));

    let mut spline = MoveSpline::new();
    let result = init
        .launch(
            &mut spline,
            MoveSplineLaunchInput {
                current_position: Position::new(10.0, 0.0, 0.0, 1.25),
                active_spline_position: None,
                movement_flags: MovementFlag::BACKWARD,
                selected_speed: 80.0,
                run_speed: 7.0,
                assistance_speed_factor: 1.0,
                on_transport: false,
            },
        )
        .unwrap();

    assert_eq!(result.real_position, Position::new(10.0, 0.0, 0.0, 1.25));
    assert_eq!(result.movement_flags, MovementFlag::FORWARD);
    assert_eq!(spline.id(), 77);
    assert_eq!(spline.flags(), MoveSplineFlag::SMOOTH_GROUND_PATH);
    assert_eq!(spline.velocity(), 28.0);
    assert_eq!(spline.compute_position().unwrap().x, 10.0);
    assert_eq!(result.duration_ms, spline.duration_ms());
}

#[test]
fn move_spline_init_launch_uses_active_spline_position_and_root_mask_like_cpp() {
    let mut init = MoveSplineInit::new(78);
    init.set_backward();
    init.set_velocity(5.0);
    init.move_to(Position::xyz(15.0, 0.0, 0.0));

    let mut spline = MoveSpline::new();
    let result = init
        .launch(
            &mut spline,
            MoveSplineLaunchInput {
                current_position: Position::xyz(0.0, 0.0, 0.0),
                active_spline_position: Some(Position::new(5.0, 0.0, 0.0, 0.5)),
                movement_flags: MovementFlag::ROOT | MovementFlag::FORWARD,
                selected_speed: 80.0,
                run_speed: 7.0,
                assistance_speed_factor: 1.0,
                on_transport: true,
            },
        )
        .unwrap();

    assert_eq!(result.real_position, Position::new(5.0, 0.0, 0.0, 0.5));
    assert_eq!(result.movement_flags, MovementFlag::ROOT);
    assert!(spline.on_transport);
    assert!(spline.flags().contains(MoveSplineFlag::BACKWARD));
    assert_eq!(spline.velocity(), 5.0);
    assert_eq!(spline.compute_position().unwrap().x, 5.0);
}

#[test]
fn move_spline_init_stop_reinitializes_done_spline_like_cpp() {
    let mut init = MoveSplineInit::new(79);
    init.set_velocity(5.0);
    init.move_to(Position::xyz(10.0, 0.0, 0.0));

    let mut spline = MoveSpline::new();
    init.launch(
        &mut spline,
        MoveSplineLaunchInput {
            current_position: Position::ZERO,
            selected_speed: 5.0,
            run_speed: 7.0,
            assistance_speed_factor: 1.0,
            ..MoveSplineLaunchInput::new(Position::ZERO)
        },
    )
    .unwrap();

    let stop = init
        .stop(
            &mut spline,
            MoveSplineStopInput {
                current_position: Position::ZERO,
                active_spline_position: Some(Position::new(3.0, 0.0, 0.0, 0.0)),
                on_transport: false,
            },
        )
        .unwrap();

    assert_eq!(stop.position, Position::new(3.0, 0.0, 0.0, 0.0));
    assert_eq!(stop.spline_id, 79);
    assert_eq!(stop.stop_distance_tolerance, 2);
    assert!(spline.finalized());
    assert_eq!(spline.flags(), MoveSplineFlag::DONE);
    assert!(
        init.stop(&mut spline, MoveSplineStopInput::new(Position::ZERO))
            .is_none()
    );
}

#[test]
fn move_spline_init_setters_match_cpp_flag_side_effects() {
    let mut init = MoveSplineInit::new(80);
    init.args.transform_for_transport = true;

    init.set_first_point_id(12);
    init.set_transport_enter();
    init.set_transport_exit();
    init.set_orientation_fixed(true);
    init.set_uncompressed();
    init.set_cyclic();
    init.set_unlimited_speed();
    init.set_facing_angle(-0.5);
    init.set_spell_effect_extra_data(SpellEffectExtraData {
        target: ObjectGuid::create_player(1, 99),
        spell_visual_id: 123,
        progress_curve_id: 456,
        parabolic_curve_id: 789,
    });
    init.disable_transport_path_transformations();

    assert_eq!(init.args.path_idx_offset, 12);
    assert!(init.args.flags.contains(MoveSplineFlag::TRANSPORT_EXIT));
    assert!(!init.args.flags.contains(MoveSplineFlag::TRANSPORT_ENTER));
    assert!(init.args.flags.contains(MoveSplineFlag::ORIENTATION_FIXED));
    assert!(init.args.flags.contains(MoveSplineFlag::UNCOMPRESSED_PATH));
    assert!(init.args.flags.contains(MoveSplineFlag::CYCLIC));
    assert!(init.args.flags.contains(MoveSplineFlag::UNLIMITED_SPEED));
    assert_eq!(init.args.facing.kind, MonsterMoveType::FacingAngle);
    assert!((init.args.facing.angle - (2.0 * PI - 0.5)).abs() < f32::EPSILON);
    assert!(init.args.spell_effect_extra.is_some());
    assert!(!init.args.transform_for_transport);
}

#[test]
fn move_spline_init_visual_effect_setters_are_mutually_exclusive_like_cpp() {
    let mut init = MoveSplineInit::new(81);

    init.set_parabolic(3.5, 0.25);
    assert_eq!(init.args.effect_start_time_percent, 0.25);
    assert_eq!(init.args.parabolic_amplitude, 3.5);
    assert_eq!(init.args.vertical_acceleration, 0.0);
    assert!(init.args.flags.contains(MoveSplineFlag::PARABOLIC));
    assert!(!init.args.flags.contains(MoveSplineFlag::ANIMATION));

    init.set_animation(4, 99, 250);
    assert_eq!(init.args.effect_start_time_percent, 0.0);
    assert_eq!(init.args.effect_start_time_ms, 250);
    assert_eq!(
        init.args.anim_tier,
        Some(AnimTierTransition {
            tier_transition_id: 99,
            anim_tier: 4,
        })
    );
    assert!(init.args.flags.contains(MoveSplineFlag::ANIMATION));
    assert!(!init.args.flags.contains(MoveSplineFlag::PARABOLIC));

    init.set_parabolic_vertical_acceleration(9.0, 0.75);
    assert_eq!(init.args.effect_start_time_percent, 0.75);
    assert_eq!(init.args.parabolic_amplitude, 0.0);
    assert_eq!(init.args.vertical_acceleration, 9.0);
    assert!(init.args.flags.contains(MoveSplineFlag::PARABOLIC));
    assert!(!init.args.flags.contains(MoveSplineFlag::ANIMATION));
}

#[test]
fn move_spline_init_facing_setters_match_cpp_shapes() {
    let mut init = MoveSplineInit::new(82);
    let spot = Position::xyz(1.0, 2.0, 3.0);
    let target = ObjectGuid::create_player(1, 22);

    init.set_facing_spot(spot);
    assert_eq!(init.args.facing.kind, MonsterMoveType::FacingSpot);
    assert_eq!(init.args.facing.spot, spot);

    init.set_facing_target_with_angle(target, 1.25);
    assert_eq!(init.args.facing.kind, MonsterMoveType::FacingTarget);
    assert_eq!(init.args.facing.target, target);
    assert_eq!(init.args.facing.angle, 1.25);

    init.set_facing_angle(2.5 * PI);
    assert_eq!(init.args.facing.kind, MonsterMoveType::FacingAngle);
    assert!((init.args.facing.angle - 0.5 * PI).abs() < 0.000_001);
}

#[test]
fn linear_spline_duration_position_and_finalize_match_cpp_shape() {
    let args = linear_args();
    let mut spline = MoveSpline::new();
    spline.initialize(&args).unwrap();

    assert!(spline.initialized());
    assert_eq!(spline.id(), 42);
    assert_eq!(spline.duration_ms(), 2001);
    assert_eq!(spline.current_path_index(), 0);
    assert_eq!(
        spline.compute_position().unwrap(),
        Position::new(0.0, 0.0, 0.0, 0.0)
    );

    let mid = spline.compute_position_offset(1000).unwrap();
    assert!((mid.x - 4.997_501).abs() < 0.000_1);
    assert!(mid.orientation.abs() < f32::EPSILON);

    assert_eq!(spline.update_state(2001), vec![SplineUpdateResult::Arrived]);
    assert!(spline.finalized());
    assert_eq!(spline.time_passed_ms(), 2001);
    assert_eq!(spline.current_path_index(), 1);
}

#[test]
fn compute_position_percent_uses_cpp_spline_index_rules() {
    let args = MoveSplineInitArgs {
        path: vec![
            Position::xyz(0.0, 0.0, 0.0),
            Position::xyz(10.0, 0.0, 0.0),
            Position::xyz(10.0, 10.0, 0.0),
        ],
        velocity: 10.0,
        ..MoveSplineInitArgs::default()
    };
    let mut spline = MoveSpline::new();
    spline.initialize(&args).unwrap();

    let start = spline.compute_position_percent(0.0).unwrap();
    let mid = spline.compute_position_percent(0.5).unwrap();
    let end = spline.compute_position_percent(1.0).unwrap();

    assert!(start.x.abs() < f32::EPSILON);
    assert!(start.y.abs() < f32::EPSILON);
    assert!(mid.x > 9.9);
    assert!(mid.y.abs() < 0.1);
    assert!((end.x - 10.0).abs() < f32::EPSILON);
    assert!((end.y - 10.0).abs() < f32::EPSILON);
    assert!(spline.compute_position_percent(-0.1).is_none());
    assert!(spline.compute_position_percent(1.1).is_none());
}

#[test]
fn cyclic_spline_wraps_without_finalizing() {
    let args = MoveSplineInitArgs {
        path: vec![
            Position::xyz(0.0, 0.0, 0.0),
            Position::xyz(10.0, 0.0, 0.0),
            Position::xyz(10.0, 10.0, 0.0),
        ],
        flags: MoveSplineFlag::CYCLIC,
        velocity: 10.0,
        ..MoveSplineInitArgs::default()
    };
    let mut spline = MoveSpline::new();
    spline.initialize(&args).unwrap();
    let duration = spline.duration_ms();

    let results = spline.update_state(duration + 1);
    assert!(results.contains(&SplineUpdateResult::NextCycle));
    assert!(!spline.finalized());
    assert_eq!(spline.current_spline_index(), 1);
}

#[test]
fn parabolic_amplitude_uses_cpp_acceleration_formula() {
    let args = MoveSplineInitArgs {
        path: vec![Position::xyz(0.0, 0.0, 0.0), Position::xyz(10.0, 0.0, 0.0)],
        flags: MoveSplineFlag::PARABOLIC,
        velocity: 10.0,
        parabolic_amplitude: 4.0,
        ..MoveSplineInitArgs::default()
    };
    let mut spline = MoveSpline::new();
    spline.initialize(&args).unwrap();

    let mid = spline
        .compute_position_offset(spline.duration_ms() / 2)
        .unwrap();
    assert!(mid.z > 3.9 && mid.z < 4.1);
}

#[test]
fn animation_tier_transition_matches_cpp_effect_start_storage() {
    let mut flags = MoveSplineFlag::PARABOLIC | MoveSplineFlag::FALLING_SLOW;
    flags.enable_animation();
    let args = MoveSplineInitArgs {
        path: vec![Position::xyz(0.0, 0.0, 0.0), Position::xyz(10.0, 0.0, 0.0)],
        flags,
        velocity: 10.0,
        effect_start_time_ms: 250,
        anim_tier: Some(AnimTierTransition {
            tier_transition_id: 77,
            anim_tier: 3,
        }),
        ..MoveSplineInitArgs::default()
    };
    let mut spline = MoveSpline::new();
    spline.initialize(&args).unwrap();

    assert!(spline.flags().contains(MoveSplineFlag::ANIMATION));
    assert!(!spline.flags().intersects(
        MoveSplineFlag::PARABOLIC | MoveSplineFlag::FALLING | MoveSplineFlag::FALLING_SLOW
    ));
    assert_eq!(
        spline.anim_tier(),
        Some(AnimTierTransition {
            tier_transition_id: 77,
            anim_tier: 3
        })
    );
    assert_eq!(spline.effect_start_time_ms(), 250);
}

#[test]
fn cyclic_enter_cycle_rewrites_path_and_preserves_duration_like_cpp() {
    let args = MoveSplineInitArgs {
        path: vec![
            Position::xyz(0.0, 0.0, 0.0),
            Position::xyz(10.0, 0.0, 0.0),
            Position::xyz(10.0, 10.0, 0.0),
            Position::xyz(0.0, 10.0, 0.0),
        ],
        flags: MoveSplineFlag::CYCLIC | MoveSplineFlag::ENTER_CYCLE,
        velocity: 10.0,
        ..MoveSplineInitArgs::default()
    };
    let mut spline = MoveSpline::new();
    spline.initialize(&args).unwrap();
    let old_duration = spline.duration_ms();
    let old_point_count = spline.spline.points.len();

    let results = spline.update_state(old_duration);

    assert_eq!(results.last(), Some(&SplineUpdateResult::NextCycle));
    assert!(!spline.flags().contains(MoveSplineFlag::ENTER_CYCLE));
    assert_eq!(spline.duration_ms(), old_duration);
    assert!(spline.spline.points.len() < old_point_count);
    assert_eq!(
        spline.spline.point(spline.spline.first),
        Position::xyz(10.0, 0.0, 0.0)
    );
    assert!(!spline.finalized());
}

#[test]
fn monster_move_path_data_compresses_like_cpp_initialize_spline_data() {
    let args = MoveSplineInitArgs {
        path: vec![
            Position::xyz(0.0, 0.0, 0.0),
            Position::xyz(10.0, 0.0, 0.0),
            Position::xyz(20.0, 0.0, 0.0),
            Position::xyz(30.0, 0.0, 0.0),
        ],
        velocity: 10.0,
        ..MoveSplineInitArgs::default()
    };
    let mut spline = MoveSpline::new();
    spline.initialize(&args).unwrap();

    let path_data = spline.monster_move_path_data();

    assert_eq!(path_data.points, vec![Position::xyz(30.0, 0.0, 0.0)]);
    assert_eq!(
        path_data.packed_deltas,
        vec![[5.0, 0.0, 0.0], [-5.0, 0.0, 0.0]]
    );
}

#[test]
fn monster_move_path_data_uncompressed_cyclic_matches_cpp_point_rules() {
    let args = MoveSplineInitArgs {
        path: vec![
            Position::xyz(0.0, 0.0, 0.0),
            Position::xyz(10.0, 0.0, 0.0),
            Position::xyz(10.0, 10.0, 0.0),
        ],
        flags: MoveSplineFlag::CYCLIC | MoveSplineFlag::UNCOMPRESSED_PATH,
        velocity: 10.0,
        ..MoveSplineInitArgs::default()
    };
    let mut spline = MoveSpline::new();
    spline.initialize(&args).unwrap();

    let path_data = spline.monster_move_path_data();

    assert_eq!(
        path_data.points,
        vec![
            Position::xyz(0.0, 0.0, 0.0),
            Position::xyz(0.0, 0.0, 0.0),
            Position::xyz(10.0, 0.0, 0.0),
            Position::xyz(10.0, 10.0, 0.0),
        ]
    );
    assert!(path_data.packed_deltas.is_empty());
}
