//! Unit-subsystem regressions, part 2 of 3.
//!
//! Moved out of the tests.rs root under #648; every test is unchanged.

use super::*;

#[test]
fn combat_can_begin_matches_cpp_guard_order_shape() {
    let valid = CombatBeginContextLikeCpp {
        attacker_in_world: true,
        victim_in_world: true,
        attacker_alive: true,
        victim_alive: true,
        same_map: true,
        same_phase: true,
        ..Default::default()
    };

    assert!(CombatSubsystem::can_begin_combat_like_cpp(valid));
    assert!(!CombatSubsystem::can_begin_combat_like_cpp(
        CombatBeginContextLikeCpp {
            same_unit: true,
            ..valid
        }
    ));
    assert!(!CombatSubsystem::can_begin_combat_like_cpp(
        CombatBeginContextLikeCpp {
            attacker_in_world: false,
            ..valid
        }
    ));
    assert!(!CombatSubsystem::can_begin_combat_like_cpp(
        CombatBeginContextLikeCpp {
            victim_alive: false,
            ..valid
        }
    ));
    assert!(!CombatSubsystem::can_begin_combat_like_cpp(
        CombatBeginContextLikeCpp {
            same_map: false,
            ..valid
        }
    ));
    assert!(!CombatSubsystem::can_begin_combat_like_cpp(
        CombatBeginContextLikeCpp {
            same_phase: false,
            ..valid
        }
    ));
    assert!(!CombatSubsystem::can_begin_combat_like_cpp(
        CombatBeginContextLikeCpp {
            attacker_unit_state: UnitState::EVADE.bits(),
            ..valid
        }
    ));
    assert!(!CombatSubsystem::can_begin_combat_like_cpp(
        CombatBeginContextLikeCpp {
            victim_unit_state: UnitState::IN_FLIGHT.bits(),
            ..valid
        }
    ));
    assert!(!CombatSubsystem::can_begin_combat_like_cpp(
        CombatBeginContextLikeCpp {
            attacker_combat_disallowed: true,
            ..valid
        }
    ));
    assert!(!CombatSubsystem::can_begin_combat_like_cpp(
        CombatBeginContextLikeCpp {
            relation_represented: true,
            victim_is_friendly_to_attacker: true,
            ..valid
        }
    ));
    assert!(!CombatSubsystem::can_begin_combat_like_cpp(
        CombatBeginContextLikeCpp {
            attacker_or_owner_player_is_game_master: true,
            ..valid
        }
    ));
}

#[test]
fn combat_revalidate_removes_invalid_refs_and_related_threat_like_cpp() {
    let mut combat = CombatSubsystem::default();
    combat.initialize_threat_list_capability(true);
    let valid_pve = guid(42);
    let invalid_pve = guid(43);
    let invalid_pvp = guid(44);

    combat.set_in_combat_with(valid_pve, false, false);
    combat.set_in_combat_with(invalid_pve, false, false);
    combat.set_in_combat_with(invalid_pvp, true, false);
    combat.add_threat(valid_pve, 10.0);
    combat.add_threat(invalid_pve, 20.0);
    combat.put_threatened_by_me_ref(invalid_pve, ThreatReferenceState::default());

    let removed =
        combat.revalidate_combat_like_cpp(|guid, _| guid != invalid_pve && guid != invalid_pvp);

    assert_eq!(removed.len(), 2);
    assert!(removed.contains(&invalid_pve));
    assert!(removed.contains(&invalid_pvp));
    assert!(combat.is_in_combat_with(valid_pve));
    assert!(!combat.is_in_combat_with(invalid_pve));
    assert!(!combat.is_in_combat_with(invalid_pvp));
    assert_eq!(combat.threat_value(valid_pve), Some(10.0));
    assert_eq!(combat.threat_value(invalid_pve), None);
    assert!(!combat.is_threatening_to(invalid_pve, true));
}

#[test]
fn combat_purge_ref_removes_ref_and_related_threat_like_cpp_end_combat_side() {
    let mut combat = CombatSubsystem::default();
    combat.initialize_threat_list_capability(true);
    let target = guid(45);
    combat.set_in_combat_with(target, false, false);
    combat.add_threat(target, 30.0);
    combat.put_threatened_by_me_ref(target, ThreatReferenceState::default());

    assert!(combat.purge_combat_ref_like_cpp(target));
    assert!(!combat.is_in_combat_with(target));
    assert_eq!(combat.threat_value(target), None);
    assert!(!combat.is_threatening_to(target, true));
    assert!(!combat.purge_combat_ref_like_cpp(target));
}

#[test]
fn threatened_by_me_refs_follow_cpp_reverse_lookup_shape() {
    let mut combat = CombatSubsystem::default();
    let owner = guid(50);
    let mut reference = ThreatReferenceState::default();
    reference.set_online_state(ThreatOnlineState::Suppressed);
    reference.base_amount = 10.0;

    combat.put_threatened_by_me_ref(owner, reference);
    assert!(combat.is_threatening_anyone(false));
    assert!(combat.is_threatening_to(owner, false));
    combat
        .threatened_by_me
        .get_mut(&owner)
        .expect("reverse threat ref")
        .set_online_state(ThreatOnlineState::Offline);
    reference.set_online_state(ThreatOnlineState::Offline);
    assert!(!combat.is_threatening_anyone(false));
    assert!(combat.is_threatening_anyone(true));
    assert_eq!(combat.purge_threatened_by_me_ref(owner), Some(reference));
    assert!(!combat.is_threatening_anyone(true));
}

#[test]
fn motion_generator_ids_slots_and_priorities_match_cpp_motion_master_shape() {
    assert_eq!(MovementGeneratorKind::Idle.trinity_id(), 0);
    assert_eq!(MovementGeneratorKind::Random.trinity_id(), 1);
    assert_eq!(MovementGeneratorKind::Waypoint.trinity_id(), 2);
    assert_eq!(MovementGeneratorKind::from_trinity_id(3), None);
    assert_eq!(
        MovementGeneratorKind::from_trinity_id(14),
        Some(MovementGeneratorKind::Follow)
    );
    assert_eq!(
        MovementGeneratorKind::from_trinity_id(18),
        Some(MovementGeneratorKind::Formation)
    );
    assert_eq!(MovementSlot::Default as u8, 0);
    assert_eq!(MovementSlot::Active as u8, 1);

    let mut motion = MotionSubsystem::default();
    motion.add_to_world();
    assert_eq!(motion.size(), 1);
    assert_eq!(motion.current_slot(), MovementSlot::Default);
    assert_eq!(
        motion.current_movement_generator().kind,
        MovementGeneratorKind::Idle
    );
    assert_eq!(
        motion.current_movement_generator().priority,
        MovementGeneratorPriority::Normal
    );
    assert!(
        motion
            .current_movement_generator()
            .has_flag(MOVEMENTGENERATOR_FLAG_INITIALIZED)
    );

    motion.add_generator(
        MovementGeneratorRef::new(MovementGeneratorKind::Follow, MovementSlot::Active)
            .with_priority(MovementGeneratorPriority::Normal)
            .with_target_guid(guid(30)),
    );
    assert_eq!(motion.current_slot(), MovementSlot::Active);
    assert_eq!(
        motion.current_movement_generator().kind,
        MovementGeneratorKind::Follow
    );
    assert!(motion.pause_current_movement_like_cpp(750, MovementSlot::Default, false));
    let default_generator = motion.default_generator;
    assert!(default_generator.has_flag(MOVEMENTGENERATOR_FLAG_TIMED_PAUSED));
    assert!(!default_generator.has_flag(MOVEMENTGENERATOR_FLAG_PAUSED));
    assert_eq!(default_generator.duration_ms, Some(750));
    assert!(
        !motion.stopped,
        "C++ PauseMovement only StopMoving()s when the paused slot is current"
    );
    assert!(motion.pause_current_movement_like_cpp(0, MovementSlot::Active, true));
    let current = motion.current_movement_generator();
    assert!(current.has_flag(MOVEMENTGENERATOR_FLAG_PAUSED));
    assert!(!current.has_flag(MOVEMENTGENERATOR_FLAG_TIMED_PAUSED));
    assert!(
        motion.stopped,
        "C++ PauseMovement forced=true stops when the requested slot is current"
    );

    motion.move_charge(42);
    let current = motion.current_movement_generator();
    assert_eq!(current.kind, MovementGeneratorKind::Point);
    assert_eq!(current.priority, MovementGeneratorPriority::Highest);
    assert_eq!(current.base_unit_state, UnitState::CHARGING.bits());
    assert!(current.has_flag(MOVEMENTGENERATOR_FLAG_INITIALIZATION_PENDING));
    assert_eq!(
        motion.base_unit_states.get(&UnitState::CHARGING.bits()),
        Some(&1)
    );
    assert!(
        motion
            .active_generators
            .iter()
            .any(|generator| generator.kind == MovementGeneratorKind::Follow
                && generator.has_flag(MOVEMENTGENERATOR_FLAG_DEACTIVATED))
    );

    let removed = motion.clear_by_priority(MovementGeneratorPriority::Highest);
    assert_eq!(removed.len(), 1);
    assert_eq!(
        motion.base_unit_states.get(&UnitState::CHARGING.bits()),
        None
    );
    assert_eq!(
        motion.current_movement_generator().kind,
        MovementGeneratorKind::Follow
    );
}

#[test]
fn motion_direct_initialize_preserves_selected_waypoint_default_like_cpp() {
    let mut motion = MotionSubsystem::default();
    motion.initialize_default_generator_like_cpp(MovementGeneratorKind::Waypoint);
    motion.add_generator(
        MovementGeneratorRef::new(MovementGeneratorKind::Point, MovementSlot::Active)
            .with_priority(MovementGeneratorPriority::Normal),
    );

    motion.direct_initialize_like_cpp();

    assert!(motion.active_generators.is_empty());
    let current = motion.current_movement_generator();
    assert_eq!(
        current.kind,
        MovementGeneratorKind::Waypoint,
        "C++ MotionMaster::DirectInitialize clears generators then InitializeDefault selects owner GetDefaultMovementType(), not unconditional idle"
    );
    assert_eq!(current.priority, MovementGeneratorPriority::Normal);
    assert_eq!(current.base_unit_state, UnitState::ROAMING.bits());
    assert!(current.has_flag(MOVEMENTGENERATOR_FLAG_INITIALIZATION_PENDING));
    assert!(!current.has_flag(MOVEMENTGENERATOR_FLAG_INITIALIZED));
}

#[test]
fn motion_direct_initialize_preserves_selected_random_default_like_cpp() {
    let mut motion = MotionSubsystem::default();
    motion.initialize_default_generator_like_cpp(MovementGeneratorKind::Random);
    motion.add_generator(
        MovementGeneratorRef::new(MovementGeneratorKind::Point, MovementSlot::Active)
            .with_priority(MovementGeneratorPriority::Normal),
    );

    motion.direct_initialize_like_cpp();

    assert!(motion.active_generators.is_empty());
    let current = motion.current_movement_generator();
    assert_eq!(
        current.kind,
        MovementGeneratorKind::Random,
        "C++ FactorySelector::SelectMovementGenerator returns RandomMovementGenerator for RANDOM_MOTION_TYPE"
    );
    assert_eq!(current.priority, MovementGeneratorPriority::Normal);
    assert_eq!(
        current.base_unit_state,
        UnitState::ROAMING.bits(),
        "C++ RandomMovementGenerator constructor sets BaseUnitState=UNIT_STATE_ROAMING"
    );
    assert!(current.has_flag(MOVEMENTGENERATOR_FLAG_INITIALIZATION_PENDING));
    assert!(!current.has_flag(MOVEMENTGENERATOR_FLAG_INITIALIZED));
}

#[test]
fn motion_master_flags_and_delayed_actions_match_cpp_shape() {
    assert_eq!(MOTIONMASTER_FLAG_NONE, 0x0);
    assert_eq!(MOTIONMASTER_FLAG_UPDATE, 0x1);
    assert_eq!(MOTIONMASTER_FLAG_STATIC_INITIALIZATION_PENDING, 0x2);
    assert_eq!(MOTIONMASTER_FLAG_INITIALIZATION_PENDING, 0x4);
    assert_eq!(MOTIONMASTER_FLAG_INITIALIZING, 0x8);
    assert_eq!(
        MOTIONMASTER_FLAG_DELAYED,
        MOTIONMASTER_FLAG_UPDATE | MOTIONMASTER_FLAG_INITIALIZATION_PENDING
    );

    assert_eq!(MotionMasterDelayedActionType::Clear.trinity_id(), 0);
    assert_eq!(MotionMasterDelayedActionType::ClearSlot.trinity_id(), 1);
    assert_eq!(MotionMasterDelayedActionType::ClearMode.trinity_id(), 2);
    assert_eq!(MotionMasterDelayedActionType::ClearPriority.trinity_id(), 3);
    assert_eq!(MotionMasterDelayedActionType::Add.trinity_id(), 4);
    assert_eq!(MotionMasterDelayedActionType::Remove.trinity_id(), 5);
    assert_eq!(MotionMasterDelayedActionType::RemoveType.trinity_id(), 6);
    assert_eq!(MotionMasterDelayedActionType::Initialize.trinity_id(), 7);
    assert_eq!(
        MotionMasterDelayedActionType::from_trinity_id(6),
        Some(MotionMasterDelayedActionType::RemoveType)
    );
    assert_eq!(MotionMasterDelayedActionType::from_trinity_id(8), None);

    let mut motion = MotionSubsystem::default();
    assert!(motion.should_delay_motion_master_action_like_cpp());
    motion.flags = MOTIONMASTER_FLAG_UPDATE;
    assert!(motion.should_delay_motion_master_action_like_cpp());
    motion.flags = MOTIONMASTER_FLAG_STATIC_INITIALIZATION_PENDING;
    assert!(!motion.should_delay_motion_master_action_like_cpp());
    motion.flags = MOTIONMASTER_FLAG_INITIALIZING;
    assert!(!motion.should_delay_motion_master_action_like_cpp());

    motion.push_delayed_action_like_cpp(MotionMasterDelayedActionType::Add);
    motion.push_delayed_action_with_validator_like_cpp(
        MotionMasterDelayedActionType::RemoveType,
        false,
    );
    motion.push_delayed_action_like_cpp(MotionMasterDelayedActionType::Initialize);

    let resolved = motion.resolve_delayed_actions_like_cpp();
    assert_eq!(
        resolved,
        vec![
            MotionMasterResolvedDelayedAction {
                action_type: MotionMasterDelayedActionType::Add,
                executed: true,
            },
            MotionMasterResolvedDelayedAction {
                action_type: MotionMasterDelayedActionType::RemoveType,
                executed: false,
            },
            MotionMasterResolvedDelayedAction {
                action_type: MotionMasterDelayedActionType::Initialize,
                executed: true,
            },
        ]
    );
    assert!(motion.delayed_actions.is_empty());
}

#[test]
fn motion_master_delayed_action_payloads_apply_fifo_like_cpp() {
    let mut motion = MotionSubsystem::default();
    motion.add_to_world();
    motion.add_generator(
        MovementGeneratorRef::new(MovementGeneratorKind::Follow, MovementSlot::Active)
            .with_priority(MovementGeneratorPriority::Normal)
            .with_base_unit_state(UnitState::FOLLOW.bits()),
    );
    motion.push_delayed_payload_like_cpp(MotionMasterDelayedActionPayload::Add(
        MovementGeneratorRef::new(MovementGeneratorKind::Effect, MovementSlot::Active)
            .with_priority(MovementGeneratorPriority::Highest)
            .with_base_unit_state(UnitState::JUMPING.bits())
            .with_movement_id(7),
    ));
    motion.push_delayed_payload_with_validator_like_cpp(
        MotionMasterDelayedActionPayload::RemoveType {
            kind: MovementGeneratorKind::Effect,
            slot: MovementSlot::Active,
        },
        false,
    );
    motion.push_delayed_payload_like_cpp(MotionMasterDelayedActionPayload::ClearPriority(
        MovementGeneratorPriority::Highest,
    ));

    let resolved = motion.resolve_delayed_action_payloads_like_cpp();
    assert_eq!(
        resolved,
        vec![
            MotionMasterResolvedDelayedAction {
                action_type: MotionMasterDelayedActionType::Add,
                executed: true,
            },
            MotionMasterResolvedDelayedAction {
                action_type: MotionMasterDelayedActionType::RemoveType,
                executed: false,
            },
            MotionMasterResolvedDelayedAction {
                action_type: MotionMasterDelayedActionType::ClearPriority,
                executed: true,
            },
        ]
    );
    assert!(motion.delayed_actions.is_empty());
    assert_eq!(
        motion.current_movement_generator().kind,
        MovementGeneratorKind::Follow
    );
    assert_eq!(
        motion.base_unit_states.get(&UnitState::JUMPING.bits()),
        None
    );
    assert_eq!(
        motion.base_unit_states.get(&UnitState::FOLLOW.bits()),
        Some(&1)
    );
}

#[test]
fn motion_master_update_initializes_updates_pops_and_resolves_like_cpp() {
    let mut motion = MotionSubsystem::default();
    assert_eq!(
        motion.update_motion_master_like_cpp(MotionMasterUpdateContext {
            diff_ms: 10,
            spline_finalized: true,
            ..MotionMasterUpdateContext::default()
        }),
        MotionMasterUpdateOutcome::Stalled
    );
    motion.add_to_world();
    motion.launch_generic_movement(MovementGeneratorKind::Effect, 11, 10, None);
    motion.push_delayed_payload_like_cpp(MotionMasterDelayedActionPayload::Add(
        MovementGeneratorRef::new(MovementGeneratorKind::Follow, MovementSlot::Active)
            .with_priority(MovementGeneratorPriority::Normal)
            .with_base_unit_state(UnitState::FOLLOW.bits()),
    ));

    let outcome = motion.update_motion_master_like_cpp(MotionMasterUpdateContext {
        diff_ms: 10,
        ..MotionMasterUpdateContext::default()
    });

    let mut expected_popped =
        MovementGeneratorRef::new(MovementGeneratorKind::Effect, MovementSlot::Active)
            .with_priority(MovementGeneratorPriority::Normal)
            .with_flags(MOVEMENTGENERATOR_FLAG_INITIALIZED | MOVEMENTGENERATOR_FLAG_INFORM_ENABLED)
            .with_base_unit_state(UnitState::ROAMING.bits())
            .with_movement_id(11)
            .with_duration_ms(10);
    expected_popped.elapsed_ms = 10;
    assert_eq!(
        outcome,
        MotionMasterUpdateOutcome::Updated {
            popped: Some(expected_popped),
            resolved_delayed_actions: vec![MotionMasterResolvedDelayedAction {
                action_type: MotionMasterDelayedActionType::Add,
                executed: true,
            }],
        }
    );
    assert!(!motion.has_motion_master_flag(MOTIONMASTER_FLAG_UPDATE));
    assert_eq!(
        motion.current_movement_generator().kind,
        MovementGeneratorKind::Follow
    );
    assert_eq!(
        motion.base_unit_states.get(&UnitState::ROAMING.bits()),
        None
    );
    assert_eq!(
        motion.base_unit_states.get(&UnitState::FOLLOW.bits()),
        Some(&1)
    );
}

#[test]
fn idle_rotate_and_distract_generators_match_cpp_lifecycle_shape() {
    let mut idle = MotionSubsystem::default().default_generator;
    assert_eq!(
        idle.initialize_idle_like_cpp(),
        IdleMovementAction::StopMoving
    );
    assert_eq!(idle.reset_idle_like_cpp(), IdleMovementAction::StopMoving);
    assert!(idle.update_idle_like_cpp());
    idle.finalize_idle_like_cpp();
    assert!(idle.has_flag(MOVEMENTGENERATOR_FLAG_FINALIZED));

    let mut motion = MotionSubsystem::default();
    assert!(!motion.move_rotate_like_cpp(7, 0, RotateDirection::Left));
    assert!(motion.move_rotate_like_cpp(7, 1_000, RotateDirection::Left));
    let mut rotate = motion.current_movement_generator();
    assert_eq!(rotate.kind, MovementGeneratorKind::Rotate);
    assert_eq!(rotate.priority, MovementGeneratorPriority::Normal);
    assert_eq!(rotate.base_unit_state, UnitState::ROTATING.bits());
    assert_eq!(rotate.movement_id, 7);
    assert_eq!(rotate.duration_ms, Some(1_000));
    assert_eq!(rotate.max_duration_ms, Some(1_000));
    assert_eq!(rotate.rotate_direction, Some(RotateDirection::Left));

    assert_eq!(
        rotate.initialize_rotate_like_cpp(),
        IdleMovementAction::StopMoving
    );
    assert!(rotate.has_flag(MOVEMENTGENERATOR_FLAG_INITIALIZED));
    let update = rotate.update_rotate_like_cpp(true, 250, 0.0);
    assert!(update.keep_running);
    assert_eq!(rotate.duration_ms, Some(750));
    assert!(
        update
            .facing_angle
            .is_some_and(|angle| (angle - std::f32::consts::FRAC_PI_2).abs() < 0.0001)
    );

    let finished = rotate.update_rotate_like_cpp(true, 750, std::f32::consts::FRAC_PI_2);
    assert!(!finished.keep_running);
    assert!(rotate.has_flag(MOVEMENTGENERATOR_FLAG_INFORM_ENABLED));
    assert_eq!(
        rotate.finalize_rotate_like_cpp(true, true),
        RotateMovementFinalize {
            inform: Some(PointMovementInform {
                kind: MovementGeneratorKind::Rotate,
                movement_id: 7,
            }),
        }
    );
    assert!(rotate.has_flag(MOVEMENTGENERATOR_FLAG_FINALIZED));

    let mut right = MovementGeneratorRef::new(MovementGeneratorKind::Rotate, MovementSlot::Active)
        .with_duration_ms(1_000)
        .with_max_duration_ms(1_000)
        .with_rotate_direction(RotateDirection::Right);
    let right_update = right.update_rotate_like_cpp(true, 250, std::f32::consts::PI);
    assert!(
        right_update
            .facing_angle
            .is_some_and(|angle| (angle - std::f32::consts::FRAC_PI_2).abs() < 0.0001)
    );

    let mut distract_motion = MotionSubsystem::default();
    distract_motion.move_distract_like_cpp(500);
    let mut distract = distract_motion.current_movement_generator();
    assert_eq!(distract.kind, MovementGeneratorKind::Distract);
    assert_eq!(distract.priority, MovementGeneratorPriority::Highest);
    assert_eq!(distract.base_unit_state, UnitState::DISTRACTED.bits());
    assert_eq!(distract.duration_ms, Some(500));
    assert_eq!(
        distract.initialize_distract_like_cpp(false),
        DistractMovementAction {
            stand_up: true,
            launch_facing_spline: true,
        }
    );
    assert!(distract.update_distract_like_cpp(true, 500));
    assert_eq!(distract.duration_ms, Some(0));
    assert!(!distract.has_flag(MOVEMENTGENERATOR_FLAG_INFORM_ENABLED));
    assert!(!distract.update_distract_like_cpp(true, 1));
    assert!(distract.has_flag(MOVEMENTGENERATOR_FLAG_INFORM_ENABLED));
    assert_eq!(
        distract.finalize_distract_like_cpp(true, true),
        DistractMovementFinalize {
            set_home_orientation: true,
        }
    );

    let mut deactivated =
        MovementGeneratorRef::new(MovementGeneratorKind::Distract, MovementSlot::Active);
    deactivated.deactivate_timed_idle_like_cpp();
    assert!(deactivated.has_flag(MOVEMENTGENERATOR_FLAG_DEACTIVATED));
}

#[test]
fn motion_move_point_tracks_cpp_point_generator_base_state() {
    let mut motion = MotionSubsystem::default();

    motion.move_point(9);

    let current = motion.current_movement_generator();
    assert_eq!(current.kind, MovementGeneratorKind::Point);
    assert_eq!(current.priority, MovementGeneratorPriority::Normal);
    assert_eq!(current.base_unit_state, UnitState::ROAMING.bits());
    assert_eq!(current.movement_id, 9);
    assert!(current.has_flag(MOVEMENTGENERATOR_FLAG_INITIALIZATION_PENDING));
    assert_eq!(
        motion.base_unit_states.get(&UnitState::ROAMING.bits()),
        Some(&1)
    );

    let removed = motion.clear_by_priority(MovementGeneratorPriority::Normal);
    assert_eq!(removed.len(), 1);
    assert_eq!(
        motion.base_unit_states.get(&UnitState::ROAMING.bits()),
        None
    );
}

#[test]
fn point_movement_generator_lifecycle_matches_cpp_shape() {
    let mut generator =
        MovementGeneratorRef::new(MovementGeneratorKind::Point, MovementSlot::Active)
            .with_priority(MovementGeneratorPriority::Normal)
            .with_flags(
                MOVEMENTGENERATOR_FLAG_INITIALIZATION_PENDING | MOVEMENTGENERATOR_FLAG_DEACTIVATED,
            )
            .with_base_unit_state(UnitState::ROAMING.bits())
            .with_movement_id(9);

    assert_eq!(
        generator.initialize_point_like_cpp(true),
        PointMovementAction::LaunchSpline
    );
    assert!(generator.has_flag(MOVEMENTGENERATOR_FLAG_INITIALIZED));
    assert!(!generator.has_flag(MOVEMENTGENERATOR_FLAG_INITIALIZATION_PENDING));
    assert!(!generator.has_flag(MOVEMENTGENERATOR_FLAG_DEACTIVATED));

    assert_eq!(
        generator.update_point_like_cpp(true, false),
        PointMovementAction::Continue
    );
    assert_eq!(
        generator.update_point_like_cpp(true, true),
        PointMovementAction::Finished
    );
    assert!(generator.has_flag(MOVEMENTGENERATOR_FLAG_INFORM_ENABLED));

    let finalized = generator.finalize_point_like_cpp(true, true);
    assert!(generator.has_flag(MOVEMENTGENERATOR_FLAG_FINALIZED));
    assert_eq!(
        finalized,
        PointMovementFinalize {
            clear_roaming_move: true,
            inform: Some(PointMovementInform {
                kind: MovementGeneratorKind::Point,
                movement_id: 9,
            }),
        }
    );

    let mut blocked = MovementGeneratorRef::new(MovementGeneratorKind::Point, MovementSlot::Active)
        .with_flags(MOVEMENTGENERATOR_FLAG_INITIALIZATION_PENDING);
    assert_eq!(
        blocked.initialize_point_like_cpp(false),
        PointMovementAction::StopMoving
    );
    assert!(blocked.has_flag(MOVEMENTGENERATOR_FLAG_INTERRUPTED));
    assert_eq!(
        blocked.update_point_like_cpp(false, false),
        PointMovementAction::StopMovingAndContinue
    );

    let mut speed_update =
        MovementGeneratorRef::new(MovementGeneratorKind::Point, MovementSlot::Active)
            .with_flags(MOVEMENTGENERATOR_FLAG_SPEED_UPDATE_PENDING);
    assert_eq!(
        speed_update.update_point_like_cpp(true, false),
        PointMovementAction::RelaunchSpline
    );
    assert!(!speed_update.has_flag(MOVEMENTGENERATOR_FLAG_SPEED_UPDATE_PENDING));

    let mut interrupted =
        MovementGeneratorRef::new(MovementGeneratorKind::Point, MovementSlot::Active)
            .with_flags(MOVEMENTGENERATOR_FLAG_INTERRUPTED);
    assert_eq!(
        interrupted.update_point_like_cpp(true, true),
        PointMovementAction::RelaunchSpline
    );
    assert!(!interrupted.has_flag(MOVEMENTGENERATOR_FLAG_INTERRUPTED));
}

#[test]
fn point_movement_charge_prepath_informs_as_event_charge_like_cpp() {
    let mut generator =
        MovementGeneratorRef::new(MovementGeneratorKind::Point, MovementSlot::Active)
            .with_priority(MovementGeneratorPriority::Highest)
            .with_flags(MOVEMENTGENERATOR_FLAG_INITIALIZATION_PENDING)
            .with_base_unit_state(UnitState::CHARGING.bits())
            .with_movement_id(EVENT_CHARGE_PREPATH);

    assert_eq!(
        generator.initialize_point_like_cpp(true),
        PointMovementAction::MarkRoamingMove
    );
    assert!(generator.has_flag(MOVEMENTGENERATOR_FLAG_INITIALIZED));

    assert_eq!(
        generator.update_point_like_cpp(true, false),
        PointMovementAction::Continue
    );
    assert_eq!(
        generator.update_point_like_cpp(true, true),
        PointMovementAction::Finished
    );
    assert!(generator.has_flag(MOVEMENTGENERATOR_FLAG_INFORM_ENABLED));

    assert_eq!(
        generator.finalize_point_like_cpp(true, true),
        PointMovementFinalize {
            clear_roaming_move: true,
            inform: Some(PointMovementInform {
                kind: MovementGeneratorKind::Point,
                movement_id: EVENT_CHARGE,
            }),
        }
    );

    let mut deactivated =
        MovementGeneratorRef::new(MovementGeneratorKind::Point, MovementSlot::Active);
    assert_eq!(
        deactivated.deactivate_point_like_cpp(),
        PointMovementAction::ClearRoamingMove
    );
    assert!(deactivated.has_flag(MOVEMENTGENERATOR_FLAG_DEACTIVATED));
}

#[test]
fn assistance_movement_generators_match_cpp_constructor_and_finalize_shape() {
    let mut motion = MotionSubsystem::default();

    assert_eq!(
        motion.move_seek_assistance_like_cpp(),
        SeekAssistancePlan {
            attack_stop: true,
            cast_stop: true,
            do_not_reacquire_spell_focus_target: true,
            set_react_passive: true,
            generator_added: true,
        }
    );

    let assist = motion.current_movement_generator();
    assert_eq!(assist.kind, MovementGeneratorKind::Assistance);
    assert_eq!(assist.priority, MovementGeneratorPriority::Normal);
    assert_eq!(assist.base_unit_state, UnitState::ROAMING.bits());
    assert_eq!(assist.movement_id, EVENT_ASSIST_MOVE);
    assert!(assist.has_flag(MOVEMENTGENERATOR_FLAG_INITIALIZATION_PENDING));

    let mut finalized = assist.with_flags(MOVEMENTGENERATOR_FLAG_INFORM_ENABLED);
    assert_eq!(
        finalized.finalize_assistance_like_cpp(true, true, true, true),
        AssistanceMovementFinalize {
            clear_roaming_move: true,
            set_no_call_assistance: Some(false),
            call_assistance: true,
            seek_assistance_distract_ms: Some(CREATURE_FAMILY_ASSISTANCE_DELAY_MS_LIKE_CPP),
        }
    );
    assert!(finalized.has_flag(MOVEMENTGENERATOR_FLAG_FINALIZED));

    let mut non_creature = assist.with_flags(MOVEMENTGENERATOR_FLAG_INFORM_ENABLED);
    assert_eq!(
        non_creature.finalize_assistance_like_cpp(true, true, false, true),
        AssistanceMovementFinalize {
            clear_roaming_move: true,
            set_no_call_assistance: None,
            call_assistance: false,
            seek_assistance_distract_ms: None,
        }
    );

    motion.move_seek_assistance_distract_like_cpp(777);
    let distract = motion.current_movement_generator();
    assert_eq!(distract.kind, MovementGeneratorKind::AssistanceDistract);
    assert_eq!(distract.priority, MovementGeneratorPriority::Normal);
    assert_eq!(distract.base_unit_state, UnitState::DISTRACTED.bits());
    assert_eq!(distract.duration_ms, Some(777));
    assert!(distract.has_flag(MOVEMENTGENERATOR_FLAG_INITIALIZATION_PENDING));

    let mut distract_finalized = distract.with_flags(MOVEMENTGENERATOR_FLAG_INFORM_ENABLED);
    assert_eq!(
        distract_finalized.finalize_assistance_distract_like_cpp(true, true),
        AssistanceDistractFinalize {
            set_react_aggressive: true,
        }
    );
    assert!(distract_finalized.has_flag(MOVEMENTGENERATOR_FLAG_FINALIZED));
}
