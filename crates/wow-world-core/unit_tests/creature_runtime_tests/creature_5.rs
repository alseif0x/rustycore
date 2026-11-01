//! Creature scenarios for [`super`].
//!
//! Split out of map_manager_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

#[test]
fn world_creature_begin_point_movement_uses_point_lifecycle_and_real_spline() {
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 54323);
    let mut creature = fixture_creature(
        guid,
        1,
        Position::new(10.0, 10.0, 0.0, 0.0),
        50,
        2,
        5,
        10,
        20.0,
        100,
        14,
        0,
        0,
    );
    backdate_runtime_clock_for_test(&mut creature, Duration::from_secs(10));
    let dst = Position::new(14.0, 10.0, 0.0, 0.0);

    let (from, spline) = CreatureMovementLikeCpp::canonical_like_cpp(&mut creature)
        .begin_point_movement_like_cpp(42, dst, true)
        .expect("point movement starts direct spline");

    assert_eq!(from, Position::new(10.0, 10.0, 0.0, 0.0));
    assert!(creature.runtime_like_cpp_mut().active_move_spline.is_some());
    assert_eq!(creature.move_target(), Some(dst));
    assert!(
        creature
            .unit()
            .has_unit_state(UnitState::ROAMING_MOVE.bits())
    );
    let motion = &creature.unit().subsystems().motion;
    let generator = motion.current_movement_generator();
    assert_eq!(generator.kind, MovementGeneratorKind::Point);
    assert_eq!(generator.movement_id, 42);
    assert!(generator.has_flag(wow_entities::MOVEMENTGENERATOR_FLAG_INITIALIZED));
    assert!(!generator.has_flag(wow_entities::MOVEMENTGENERATOR_FLAG_INITIALIZATION_PENDING));
    assert!(motion.spline.enabled);
    assert_eq!(motion.spline.spline_id, spline.id());
    assert_eq!(motion.spline.final_destination, Some((14, 10, 0)));

    {
        let motion = &mut creature.unit_mut().subsystems_mut().motion;
        let generator = motion
            .active_generators
            .iter_mut()
            .find(|generator| generator.kind == MovementGeneratorKind::Point)
            .expect("point generator");
        assert_eq!(
            generator.update_point_like_cpp(true, true),
            PointMovementAction::Finished
        );
    }
    assert_eq!(
        CreatureMovementLikeCpp::canonical_like_cpp(&mut creature)
            .finalize_point_movement_like_cpp(true, true),
        Some(PointMovementInform {
            kind: MovementGeneratorKind::Point,
            movement_id: 42,
        })
    );
    assert!(
        !creature
            .unit()
            .has_unit_state(UnitState::ROAMING_MOVE.bits())
    );
    assert_eq!(
        creature.ai_ownership().last_movement_inform,
        Some(wow_entities::CreatureMovementInform {
            movement_type: MovementGeneratorKind::Point.trinity_id(),
            movement_id: 42,
        })
    );
}
#[test]
fn world_creature_begin_point_movement_handles_blocked_and_prepath_branches() {
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 54324);
    let mut creature = fixture_creature(
        guid,
        1,
        Position::new(10.0, 10.0, 0.0, 0.0),
        50,
        2,
        5,
        10,
        20.0,
        100,
        14,
        0,
        0,
    );
    backdate_runtime_clock_for_test(&mut creature, Duration::from_secs(10));
    let dst = Position::new(14.0, 10.0, 0.0, 0.0);

    assert!(
        CreatureMovementLikeCpp::canonical_like_cpp(&mut creature)
            .begin_point_movement_like_cpp(43, dst, false)
            .is_none()
    );
    assert!(creature.runtime_like_cpp_mut().active_move_spline.is_none());
    let generator = creature
        .unit()
        .subsystems()
        .motion
        .current_movement_generator();
    assert!(generator.has_flag(wow_entities::MOVEMENTGENERATOR_FLAG_INTERRUPTED));
    assert!(creature.unit().subsystems().motion.stopped);

    assert!(
        CreatureMovementLikeCpp::canonical_like_cpp(&mut creature)
            .begin_point_movement_like_cpp(EVENT_CHARGE_PREPATH, dst, true)
            .is_none()
    );
    assert!(creature.runtime_like_cpp_mut().active_move_spline.is_none());
    assert!(
        creature
            .unit()
            .has_unit_state(UnitState::ROAMING_MOVE.bits())
    );
    let generator = creature
        .unit()
        .subsystems()
        .motion
        .current_movement_generator();
    assert_eq!(generator.kind, MovementGeneratorKind::Point);
    assert_eq!(generator.movement_id, EVENT_CHARGE_PREPATH);
    assert_eq!(generator.base_unit_state, UnitState::CHARGING.bits());
}
#[test]
fn world_creature_finalize_generic_movement_records_ai_inform_like_cpp() {
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 54326);
    let mut creature = fixture_creature(
        guid,
        1,
        Position::new(10.0, 10.0, 0.0, 0.0),
        50,
        2,
        5,
        10,
        20.0,
        100,
        14,
        0,
        0,
    );
    let target = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 54327);
    {
        let motion = &mut creature.unit_mut().subsystems_mut().motion;
        motion.launch_generic_movement(
            MovementGeneratorKind::Effect,
            77,
            1_000,
            Some((1234, target)),
        );
        let generator = motion
            .active_generators
            .iter_mut()
            .find(|generator| generator.kind == MovementGeneratorKind::Effect)
            .expect("generic effect generator");
        generator.initialize_generic_like_cpp();
        assert!(!generator.update_generic_like_cpp(1_000, false, false));
    }

    assert_eq!(
        CreatureMovementLikeCpp::canonical_like_cpp(&mut creature)
            .finalize_generic_movement_like_cpp(MovementGeneratorKind::Effect, 77, true),
        Some(GenericMovementInform {
            kind: MovementGeneratorKind::Effect,
            movement_id: 77,
            arrival_spell_id: Some(1234),
            arrival_spell_target_guid: Some(target),
        })
    );
    assert_eq!(
        creature.ai_ownership().last_movement_inform,
        Some(wow_entities::CreatureMovementInform {
            movement_type: MovementGeneratorKind::Effect.trinity_id(),
            movement_id: 77,
        })
    );
}
#[test]
fn world_creature_begin_distract_and_rotate_launch_facing_splines_like_cpp() {
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 54325);
    let mut creature = fixture_creature(
        guid,
        1,
        Position::new(10.0, 10.0, 0.0, 0.0),
        50,
        2,
        5,
        10,
        20.0,
        100,
        14,
        0,
        0,
    );
    backdate_runtime_clock_for_test(&mut creature, Duration::from_secs(10));
    creature
        .unit_mut()
        .set_stand_state_like_cpp(UnitStandStateType::Sit);

    let (action, from, spline) = CreatureMovementLikeCpp::canonical_like_cpp(&mut creature)
        .begin_distract_movement_like_cpp(500, 1.25)
        .expect("distract launches facing spline");

    assert_eq!(
        action,
        DistractMovementAction {
            stand_up: true,
            launch_facing_spline: true,
        }
    );
    assert_eq!(from, Position::new(10.0, 10.0, 0.0, 0.0));
    assert_eq!(
        creature.unit().stand_state_like_cpp(),
        UnitStandStateType::Stand
    );
    assert_eq!(
        spline.facing().kind,
        wow_movement::MonsterMoveType::FacingAngle
    );
    assert!((spline.facing().angle - 1.25).abs() < 0.0001);
    assert!(spline.spline_is_facing_only);
    assert_eq!(creature.spline_id(), spline.id());
    let generator = creature
        .unit()
        .subsystems()
        .motion
        .current_movement_generator();
    assert_eq!(generator.kind, MovementGeneratorKind::Distract);
    assert!(generator.has_flag(wow_entities::MOVEMENTGENERATOR_FLAG_INITIALIZED));
    creature.set_ai_home_position(Position::new(10.0, 10.0, 0.0, 2.5));
    {
        let motion = &mut creature.unit_mut().subsystems_mut().motion;
        let generator = motion
            .active_generators
            .iter_mut()
            .find(|generator| generator.kind == MovementGeneratorKind::Distract)
            .expect("distract generator");
        assert!(!generator.update_distract_like_cpp(true, 501));
    }
    assert!(
        CreatureMovementLikeCpp::canonical_like_cpp(&mut creature)
            .finalize_distract_movement_like_cpp(true)
    );
    assert!((creature.position().orientation - 2.5).abs() < 0.0001);

    creature.unit_mut().subsystems_mut().motion.clear_active();
    assert!(
        creature
            .unit_mut()
            .subsystems_mut()
            .motion
            .move_rotate_like_cpp(8, 1_000, wow_entities::RotateDirection::Left)
    );
    let (update, spline) = CreatureMovementLikeCpp::canonical_like_cpp(&mut creature)
        .tick_rotate_movement_like_cpp(250)
        .expect("rotate tick launches facing spline");
    assert!(update.keep_running);
    let expected_rotate_angle = 2.5 + std::f32::consts::FRAC_PI_2;
    assert!(
        update
            .facing_angle
            .is_some_and(|angle| (angle - expected_rotate_angle).abs() < 0.0001)
    );
    assert_eq!(
        spline.facing().kind,
        wow_movement::MonsterMoveType::FacingAngle
    );
    assert!(
        (spline.facing().angle - expected_rotate_angle).abs() < 0.0001,
        "facing angle was {}",
        spline.facing().angle
    );
    assert!(spline.spline_is_facing_only);
    let generator = creature
        .unit()
        .subsystems()
        .motion
        .current_movement_generator();
    assert_eq!(generator.kind, MovementGeneratorKind::Rotate);
    assert_eq!(generator.duration_ms, Some(750));
    assert_eq!(
        CreatureMovementLikeCpp::canonical_like_cpp(&mut creature)
            .finalize_rotate_movement_like_cpp(true),
        Some(PointMovementInform {
            kind: MovementGeneratorKind::Rotate,
            movement_id: 8,
        })
    );
    assert_eq!(
        creature.ai_ownership().last_movement_inform,
        Some(wow_entities::CreatureMovementInform {
            movement_type: MovementGeneratorKind::Rotate.trinity_id(),
            movement_id: 8,
        })
    );
}
#[test]
fn world_creature_stop_move_spline_emits_cpp_stop_state_before_arrival() {
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 54322);
    let mut creature = fixture_creature(
        guid,
        1,
        Position::new(10.0, 10.0, 0.0, 0.0),
        50,
        2,
        5,
        10,
        20.0,
        100,
        14,
        0,
        0,
    );
    backdate_runtime_clock_for_test(&mut creature, Duration::from_secs(10));
    let dst = Position::new(20.0, 10.0, 0.0, 0.0);
    let (_, spline) = CreatureMovementLikeCpp::canonical_like_cpp(&mut creature)
        .begin_move_spline_like_cpp(dst)
        .expect("valid two-point spline");
    assert!(
        creature
            .movement_flags_like_cpp()
            .contains(MovementFlag::FORWARD)
    );
    let duration_ms = spline.duration_ms() as u32;
    let now_ms = creature.runtime_elapsed_ms_like_cpp();
    creature.ai_ownership_mut().move_start_ms = now_ms.saturating_sub(u64::from(duration_ms / 2));

    let stop = CreatureMovementLikeCpp::canonical_like_cpp(&mut creature)
        .stop_move_spline_like_cpp()
        .expect("active spline stops");

    assert_eq!(stop.spline_id, 3);
    assert_eq!(stop.stop_distance_tolerance, 2);
    assert!(stop.position.x > 10.0 && stop.position.x < 20.0);
    assert_eq!(creature.position(), stop.position);
    assert!(creature.runtime_like_cpp_mut().active_move_spline.is_none());
    assert_eq!(creature.move_target(), None);
    assert!(
        !creature
            .movement_flags_like_cpp()
            .contains(MovementFlag::FORWARD),
        "C++ MoveSplineInit::Stop removes MOVEMENTFLAG_FORWARD"
    );
    assert!(
        !MovementFlag::from_bits_retain(creature_create_data_like_cpp(&creature).movement_flags)
            .contains(MovementFlag::FORWARD)
    );
    assert!(
        !creature
            .unit()
            .has_unit_state(UnitState::ROAMING_MOVE.bits())
    );
    let motion_spline = &creature.unit().subsystems().motion.spline;
    assert!(!motion_spline.enabled);
    assert!(motion_spline.finalized);
    assert_eq!(motion_spline.spline_id, stop.spline_id);
    assert!(
        CreatureMovementLikeCpp::canonical_like_cpp(&mut creature)
            .stop_move_spline_like_cpp()
            .is_none()
    );
}
#[test]
fn world_creature_create_bridge_preserves_npc_flags2_like_cpp() {
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 102);
    let mut creature = fixture_creature(
        guid,
        1,
        Position::new(10.0, 10.0, 0.0, 0.0),
        50,
        1,
        5,
        10,
        20.0,
        0,
        35,
        0x40,
        0,
    );
    creature.set_npc_flags2_runtime_like_cpp(0x0000_0001);

    let mut bridged = creature;
    let create_data = creature_create_data_like_cpp(&bridged);
    install_creature_runtime_like_cpp(&mut bridged, &create_data);

    assert_eq!(bridged.npc_flags(), 0x40);
    assert_eq!(bridged.npc_flags2(), 0x1);
    assert_eq!(bridged.npc_flags_mask_like_cpp(), 0x1_0000_0040);
    assert_eq!(
        creature_create_data_like_cpp(&bridged).npc_flags,
        0x1_0000_0040
    );
}
#[test]
fn respawn_ground_snap_skips_creature_far_below_surface_like_cpp() {
    // Spawn well below ground: probe z < gridHeight - tolerance → GetStaticHeight
    // returns invalid, so C++ does NOT rescue a buried creature.
    let dir = temp_dir_with_constant_tile(0, 32, 32, 77.0);
    let terrain = LiveTerrainHeights::new(&dir);

    let mut pending = make_pending_respawn(Instant::now());
    pending.home_pos.z = 10.0; // far under the 77.0 surface
    let mut creature = creature_from_pending_respawn_like_cpp(&pending, 0);
    snap_respawn_creature_to_ground_like_cpp(&mut creature, 0, &terrain);

    assert!((creature.unit().world().position().z - 10.0).abs() < 1e-3);

    let _ = std::fs::remove_dir_all(&dir);
}
