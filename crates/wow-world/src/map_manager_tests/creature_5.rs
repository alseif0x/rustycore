//! Creature scenarios for [`super`].
//!
//! Split out of map_manager_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

#[test]
fn world_creature_begin_point_movement_uses_point_lifecycle_and_real_spline() {
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 54323);
    let mut creature = WorldCreature::new(
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
    creature.backdate_runtime_clock_for_test(Duration::from_secs(10));
    let dst = Position::new(14.0, 10.0, 0.0, 0.0);

    let (from, spline) = creature
        .begin_point_movement_like_cpp(42, dst, true)
        .expect("point movement starts direct spline");

    assert_eq!(from, Position::new(10.0, 10.0, 0.0, 0.0));
    assert!(creature.active_move_spline.is_some());
    assert_eq!(creature.move_target(), Some(dst));
    assert!(
        creature
            .creature
            .unit()
            .has_unit_state(UnitState::ROAMING_MOVE.bits())
    );
    let motion = &creature.creature.unit().subsystems().motion;
    let generator = motion.current_movement_generator();
    assert_eq!(generator.kind, MovementGeneratorKind::Point);
    assert_eq!(generator.movement_id, 42);
    assert!(generator.has_flag(wow_entities::MOVEMENTGENERATOR_FLAG_INITIALIZED));
    assert!(!generator.has_flag(wow_entities::MOVEMENTGENERATOR_FLAG_INITIALIZATION_PENDING));
    assert!(motion.spline.enabled);
    assert_eq!(motion.spline.spline_id, spline.id());
    assert_eq!(motion.spline.final_destination, Some((14, 10, 0)));

    {
        let motion = &mut creature.creature.unit_mut().subsystems_mut().motion;
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
        creature.finalize_point_movement_like_cpp(true, true),
        Some(PointMovementInform {
            kind: MovementGeneratorKind::Point,
            movement_id: 42,
        })
    );
    assert!(
        !creature
            .creature
            .unit()
            .has_unit_state(UnitState::ROAMING_MOVE.bits())
    );
    assert_eq!(
        creature.creature.ai_ownership().last_movement_inform,
        Some(wow_entities::CreatureMovementInform {
            movement_type: MovementGeneratorKind::Point.trinity_id(),
            movement_id: 42,
        })
    );
}
#[test]
fn world_creature_begin_point_movement_handles_blocked_and_prepath_branches() {
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 54324);
    let mut creature = WorldCreature::new(
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
    creature.backdate_runtime_clock_for_test(Duration::from_secs(10));
    let dst = Position::new(14.0, 10.0, 0.0, 0.0);

    assert!(
        creature
            .begin_point_movement_like_cpp(43, dst, false)
            .is_none()
    );
    assert!(creature.active_move_spline.is_none());
    let generator = creature
        .creature
        .unit()
        .subsystems()
        .motion
        .current_movement_generator();
    assert!(generator.has_flag(wow_entities::MOVEMENTGENERATOR_FLAG_INTERRUPTED));
    assert!(creature.creature.unit().subsystems().motion.stopped);

    assert!(
        creature
            .begin_point_movement_like_cpp(EVENT_CHARGE_PREPATH, dst, true)
            .is_none()
    );
    assert!(creature.active_move_spline.is_none());
    assert!(
        creature
            .creature
            .unit()
            .has_unit_state(UnitState::ROAMING_MOVE.bits())
    );
    let generator = creature
        .creature
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
    let mut creature = WorldCreature::new(
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
        let motion = &mut creature.creature.unit_mut().subsystems_mut().motion;
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
        creature.finalize_generic_movement_like_cpp(MovementGeneratorKind::Effect, 77, true),
        Some(GenericMovementInform {
            kind: MovementGeneratorKind::Effect,
            movement_id: 77,
            arrival_spell_id: Some(1234),
            arrival_spell_target_guid: Some(target),
        })
    );
    assert_eq!(
        creature.creature.ai_ownership().last_movement_inform,
        Some(wow_entities::CreatureMovementInform {
            movement_type: MovementGeneratorKind::Effect.trinity_id(),
            movement_id: 77,
        })
    );
}
#[test]
fn world_creature_begin_distract_and_rotate_launch_facing_splines_like_cpp() {
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 54325);
    let mut creature = WorldCreature::new(
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
    creature.backdate_runtime_clock_for_test(Duration::from_secs(10));
    creature
        .creature
        .unit_mut()
        .set_stand_state_like_cpp(UnitStandStateType::Sit);

    let (action, from, spline) = creature
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
        creature.creature.unit().stand_state_like_cpp(),
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
        .creature
        .unit()
        .subsystems()
        .motion
        .current_movement_generator();
    assert_eq!(generator.kind, MovementGeneratorKind::Distract);
    assert!(generator.has_flag(wow_entities::MOVEMENTGENERATOR_FLAG_INITIALIZED));
    creature
        .creature
        .set_ai_home_position(Position::new(10.0, 10.0, 0.0, 2.5));
    {
        let motion = &mut creature.creature.unit_mut().subsystems_mut().motion;
        let generator = motion
            .active_generators
            .iter_mut()
            .find(|generator| generator.kind == MovementGeneratorKind::Distract)
            .expect("distract generator");
        assert!(!generator.update_distract_like_cpp(true, 501));
    }
    assert!(creature.finalize_distract_movement_like_cpp(true));
    assert!((creature.position().orientation - 2.5).abs() < 0.0001);

    creature
        .creature
        .unit_mut()
        .subsystems_mut()
        .motion
        .clear_active();
    assert!(
        creature
            .creature
            .unit_mut()
            .subsystems_mut()
            .motion
            .move_rotate_like_cpp(8, 1_000, wow_entities::RotateDirection::Left)
    );
    let (update, spline) = creature
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
        .creature
        .unit()
        .subsystems()
        .motion
        .current_movement_generator();
    assert_eq!(generator.kind, MovementGeneratorKind::Rotate);
    assert_eq!(generator.duration_ms, Some(750));
    assert_eq!(
        creature.finalize_rotate_movement_like_cpp(true),
        Some(PointMovementInform {
            kind: MovementGeneratorKind::Rotate,
            movement_id: 8,
        })
    );
    assert_eq!(
        creature.creature.ai_ownership().last_movement_inform,
        Some(wow_entities::CreatureMovementInform {
            movement_type: MovementGeneratorKind::Rotate.trinity_id(),
            movement_id: 8,
        })
    );
}
#[test]
fn world_creature_stop_move_spline_emits_cpp_stop_state_before_arrival() {
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 54322);
    let mut creature = WorldCreature::new(
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
    creature.backdate_runtime_clock_for_test(Duration::from_secs(10));
    let dst = Position::new(20.0, 10.0, 0.0, 0.0);
    let (_, spline) = creature
        .begin_move_spline_like_cpp(dst)
        .expect("valid two-point spline");
    assert!(
        creature
            .creature
            .movement_flags_like_cpp()
            .contains(MovementFlag::FORWARD)
    );
    let duration_ms = spline.duration_ms() as u32;
    let now_ms = creature.runtime_elapsed_ms_like_cpp();
    creature.creature.ai_ownership_mut().move_start_ms =
        now_ms.saturating_sub(u64::from(duration_ms / 2));

    let stop = creature
        .stop_move_spline_like_cpp()
        .expect("active spline stops");

    assert_eq!(stop.spline_id, 3);
    assert_eq!(stop.stop_distance_tolerance, 2);
    assert!(stop.position.x > 10.0 && stop.position.x < 20.0);
    assert_eq!(creature.position(), stop.position);
    assert!(creature.active_move_spline.is_none());
    assert_eq!(creature.move_target(), None);
    assert!(
        !creature
            .creature
            .movement_flags_like_cpp()
            .contains(MovementFlag::FORWARD),
        "C++ MoveSplineInit::Stop removes MOVEMENTFLAG_FORWARD"
    );
    assert!(
        !MovementFlag::from_bits_retain(creature.create_data.movement_flags)
            .contains(MovementFlag::FORWARD)
    );
    assert!(
        !creature
            .creature
            .unit()
            .has_unit_state(UnitState::ROAMING_MOVE.bits())
    );
    let motion_spline = &creature.creature.unit().subsystems().motion.spline;
    assert!(!motion_spline.enabled);
    assert!(motion_spline.finalized);
    assert_eq!(motion_spline.spline_id, stop.spline_id);
    assert!(creature.stop_move_spline_like_cpp().is_none());
}
#[test]
fn test_visible_creatures() {
    let mut manager = MapManager::new();
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 12345);
    let creature = WorldCreature::new(
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
        0,
        0,
    );

    manager.add_creature(0, 0, 0, 0, creature);

    // Should find creature at (10, 10)
    let visible = manager.get_visible_creatures(0, 0, 10.0, 10.0, 0.0);
    assert!(!visible.is_empty());
    assert_eq!(visible[0].guid(), guid);

    // Should not find creature far away
    let visible = manager.get_visible_creatures(0, 0, 1000.0, 1000.0, 0.0);
    assert!(visible.is_empty());
}
#[test]
fn visible_creatures_in_phase_filters_like_cpp_grid_searchers() {
    let mut manager = MapManager::new();
    let visible_guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 100);
    let hidden_guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 101);

    let mut seer_phase = PhaseShift::default();
    seer_phase.add_phase_like_cpp(20, wow_constants::PhaseFlags::empty(), 1);

    let mut visible_creature = WorldCreature::new(
        visible_guid,
        1,
        Position::new(10.0, 10.0, 0.0, 0.0),
        50,
        1,
        5,
        10,
        20.0,
        0,
        35,
        0,
        0,
    );
    visible_creature
        .creature
        .unit_mut()
        .world_mut()
        .phase_shift_mut()
        .add_phase_like_cpp(20, wow_constants::PhaseFlags::empty(), 1);

    let mut hidden_creature = WorldCreature::new(
        hidden_guid,
        1,
        Position::new(11.0, 10.0, 0.0, 0.0),
        50,
        1,
        5,
        10,
        20.0,
        0,
        35,
        0,
        0,
    );
    hidden_creature
        .creature
        .unit_mut()
        .world_mut()
        .phase_shift_mut()
        .add_phase_like_cpp(30, wow_constants::PhaseFlags::empty(), 1);

    manager.add_creature(0, 0, 0, 0, visible_creature);
    manager.add_creature(0, 0, 0, 0, hidden_creature);

    let visible = manager.get_visible_creatures_in_phase(
        0,
        0,
        10.0,
        10.0,
        0.0,
        VISIBILITY_RADIUS,
        Some(&seer_phase),
    );
    let visible_guids: HashSet<ObjectGuid> = visible.iter().map(WorldCreature::guid).collect();
    assert!(visible_guids.contains(&visible_guid));
    assert!(!visible_guids.contains(&hidden_guid));

    let unfiltered = manager.get_visible_creatures(0, 0, 10.0, 10.0, 0.0);
    let unfiltered_guids: HashSet<ObjectGuid> =
        unfiltered.iter().map(WorldCreature::guid).collect();
    assert!(unfiltered_guids.contains(&visible_guid));
    assert!(unfiltered_guids.contains(&hidden_guid));
}
#[test]
fn get_visible_creatures_uses_cpp_2d_sight_range() {
    let mut manager = MapManager::new();
    manager.get_or_create_map(1, 0);
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 72);
    let creature = WorldCreature::new(
        guid,
        1,
        Position::new(80.0, 0.0, 80.0, 0.0),
        50,
        1,
        5,
        10,
        20.0,
        0,
        35,
        0,
        0,
    );
    manager.add_creature(1, 0, 0, 0, creature);

    let visible = manager.get_visible_creatures_in_phase(1, 0, 0.0, 0.0, 0.0, 100.0, None);

    assert_eq!(
        visible.iter().map(WorldCreature::guid).collect::<Vec<_>>(),
        vec![guid],
        "C++ visibility uses horizontal distance; a vertically separated creature inside sight range must still be sent"
    );
}
#[test]
fn world_creature_create_bridge_preserves_npc_flags2_like_cpp() {
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 102);
    let mut creature = WorldCreature::new(
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
    creature
        .creature
        .set_npc_flags2_runtime_like_cpp(0x0000_0001);

    let bridged = WorldCreature::from_canonical(creature.creature, creature.create_data);

    assert_eq!(bridged.npc_flags(), 0x40);
    assert_eq!(bridged.npc_flags2(), 0x1);
    assert_eq!(bridged.npc_flags_mask_like_cpp(), 0x1_0000_0040);
    assert_eq!(bridged.create_data.npc_flags, 0x1_0000_0040);
}
#[test]
fn set_creature_anim_kit_id_like_cpp_mutates_state_create_data_and_returns_fanout() {
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 1, 103);
    let mut manager = MapManager::new();
    manager.add_creature(
        571,
        0,
        0,
        0,
        WorldCreature::new(
            guid,
            1,
            Position::new(10.0, 20.0, 30.0, 0.0),
            50,
            1,
            5,
            10,
            20.0,
            0,
            35,
            0,
            0,
        ),
    );

    let event = manager
        .set_creature_anim_kit_id_like_cpp(571, 0, guid, CreatureAnimKitSlotLikeCpp::Ai, 77, |id| {
            id == 77
        })
        .expect("valid changed anim kit emits fanout event");

    let creature = manager.find_creature(571, 0, guid).expect("creature");
    assert_eq!(creature.creature.unit().ai_anim_kit_id_like_cpp(), 77);
    assert_eq!(
        creature.create_data.ai_anim_kit_id, 77,
        "late CREATE viewers must see the mutated anim kit state"
    );
    assert_eq!(event.source_guid, guid);
    match event.recipients {
        RecipientRule::NearbyVisible {
            source_guid,
            map_id,
            instance_id,
            source_position,
            range,
            required_3d,
        } => {
            assert_eq!(source_guid, guid);
            assert_eq!(map_id, 571);
            assert_eq!(instance_id, 0);
            assert_eq!(source_position, Position::new(10.0, 20.0, 30.0, 0.0));
            assert_eq!(range, VISIBILITY_RADIUS);
            assert!(!required_3d);
        }
        other => panic!("expected NearbyVisible, got {other:?}"),
    }
    let opcode = u16::from_le_bytes([event.packet_bytes[0], event.packet_bytes[1]]);
    assert_eq!(
        opcode,
        wow_constants::ServerOpcodes::SetAiAnimKit as u16,
        "C++ Unit::SetAIAnimKitId sends SMSG_SET_AI_ANIM_KIT after mutation"
    );
}
#[test]
fn set_creature_anim_kit_id_like_cpp_rejects_same_and_invalid_nonzero_like_cpp() {
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 1, 104);
    let mut manager = MapManager::new();
    manager.add_creature(
        571,
        0,
        0,
        0,
        WorldCreature::new(
            guid,
            1,
            Position::new(10.0, 20.0, 30.0, 0.0),
            50,
            1,
            5,
            10,
            20.0,
            0,
            35,
            0,
            0,
        ),
    );

    assert!(
        manager
            .set_creature_anim_kit_id_like_cpp(
                571,
                0,
                guid,
                CreatureAnimKitSlotLikeCpp::Movement,
                88,
                |_| false,
            )
            .is_none(),
        "C++ Unit::SetMovementAnimKitId rejects nonzero IDs missing from sAnimKitStore"
    );
    assert_eq!(
        manager
            .find_creature(571, 0, guid)
            .unwrap()
            .creature
            .unit()
            .movement_anim_kit_id_like_cpp(),
        0
    );

    assert!(
        manager
            .set_creature_anim_kit_id_like_cpp(
                571,
                0,
                guid,
                CreatureAnimKitSlotLikeCpp::Melee,
                0,
                |_| false,
            )
            .is_none(),
        "same ID must not emit the C++ live packet"
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
    let mut creature = world_creature_from_pending_respawn_like_cpp(&pending, 0);
    snap_respawn_creature_to_ground_like_cpp(&mut creature, 0, &terrain);

    assert!((creature.creature.unit().world().position().z - 10.0).abs() < 1e-3);

    let _ = std::fs::remove_dir_all(&dir);
}
