//! Unit-subsystem regressions, part 3 of 3.
//!
//! Moved out of the tests.rs root under #648; every test is unchanged.

use super::*;

#[test]
fn generic_movement_generator_lifecycle_matches_cpp_shape() {
    let mut motion = MotionSubsystem::default();
    let target = guid(88);

    motion.launch_generic_movement(
        MovementGeneratorKind::Effect,
        42,
        1_000,
        Some((1234, target)),
    );

    let mut generator = motion.current_movement_generator();
    assert_eq!(generator.kind, MovementGeneratorKind::Effect);
    assert_eq!(generator.priority, MovementGeneratorPriority::Normal);
    assert_eq!(
        generator.flags,
        MOVEMENTGENERATOR_FLAG_INITIALIZATION_PENDING
    );
    assert_eq!(generator.base_unit_state, UnitState::ROAMING.bits());
    assert_eq!(generator.movement_id, 42);
    assert_eq!(generator.duration_ms, Some(1_000));
    assert_eq!(generator.arrival_spell_id, 1234);
    assert_eq!(generator.arrival_spell_target_guid, target);
    assert_eq!(
        motion.base_unit_states.get(&UnitState::ROAMING.bits()),
        Some(&1)
    );

    generator.initialize_generic_like_cpp();
    assert!(generator.has_flag(MOVEMENTGENERATOR_FLAG_INITIALIZED));
    assert!(!generator.has_flag(MOVEMENTGENERATOR_FLAG_INITIALIZATION_PENDING));

    assert!(generator.update_generic_like_cpp(999, false, false));
    assert_eq!(generator.elapsed_ms, 999);
    assert!(!generator.has_flag(MOVEMENTGENERATOR_FLAG_INFORM_ENABLED));

    assert!(!generator.update_generic_like_cpp(1, false, false));
    assert!(generator.has_flag(MOVEMENTGENERATOR_FLAG_INFORM_ENABLED));
    let inform = generator
        .finalize_generic_like_cpp(true)
        .expect("inform enabled");
    assert_eq!(
        inform,
        GenericMovementInform {
            kind: MovementGeneratorKind::Effect,
            movement_id: 42,
            arrival_spell_id: Some(1234),
            arrival_spell_target_guid: Some(target),
        }
    );
    assert!(generator.has_flag(MOVEMENTGENERATOR_FLAG_FINALIZED));

    let mut cyclic = MovementGeneratorRef::new(MovementGeneratorKind::Effect, MovementSlot::Active)
        .with_flags(MOVEMENTGENERATOR_FLAG_INITIALIZED)
        .with_duration_ms(10);
    assert!(cyclic.update_generic_like_cpp(100, true, false));
    assert_eq!(cyclic.elapsed_ms, 0);
    assert!(!cyclic.update_generic_like_cpp(0, true, true));
    assert!(cyclic.has_flag(MOVEMENTGENERATOR_FLAG_INFORM_ENABLED));

    let mut deactivated =
        MovementGeneratorRef::new(MovementGeneratorKind::Effect, MovementSlot::Active)
            .with_flags(MOVEMENTGENERATOR_FLAG_DEACTIVATED);
    deactivated.initialize_generic_like_cpp();
    assert!(deactivated.has_flag(MOVEMENTGENERATOR_FLAG_FINALIZED));
    assert!(!deactivated.has_flag(MOVEMENTGENERATOR_FLAG_DEACTIVATED));
}

#[test]
fn launch_move_spline_like_cpp_rejects_invalid_generator_types() {
    let mut motion = MotionSubsystem::default();

    assert!(!motion.launch_move_spline_like_cpp(
        MovementGeneratorKind::Custom(3),
        7,
        MovementGeneratorPriority::Highest,
        250
    ));
    assert!(motion.active_generators.is_empty());

    assert!(!motion.launch_move_spline_like_cpp(
        MovementGeneratorKind::Custom(19),
        7,
        MovementGeneratorPriority::Highest,
        250
    ));
    assert!(motion.active_generators.is_empty());

    assert!(motion.launch_move_spline_like_cpp(
        MovementGeneratorKind::Point,
        7,
        MovementGeneratorPriority::Highest,
        250
    ));
    let generator = motion.current_movement_generator();
    assert_eq!(generator.kind, MovementGeneratorKind::Point);
    assert_eq!(generator.priority, MovementGeneratorPriority::Highest);
    assert_eq!(generator.base_unit_state, UnitState::ROAMING.bits());
    assert_eq!(generator.movement_id, 7);
    assert_eq!(generator.duration_ms, Some(250));
    assert!(generator.has_flag(MOVEMENTGENERATOR_FLAG_INITIALIZATION_PENDING));
}

#[test]
fn move_jump_generators_match_cpp_priority_state_and_persist_flags() {
    let mut motion = MotionSubsystem::default();
    let target = guid(99);

    assert!(!motion.move_jump_like_cpp(1, 500, 0.009, Some((777, target))));
    assert!(motion.active_generators.is_empty());

    assert!(motion.move_jump_like_cpp(1, 500, 0.01, Some((777, target))));
    let jump = motion.current_movement_generator();
    assert_eq!(jump.kind, MovementGeneratorKind::Effect);
    assert_eq!(jump.priority, MovementGeneratorPriority::Highest);
    assert_eq!(jump.base_unit_state, UnitState::JUMPING.bits());
    assert_eq!(jump.movement_id, 1);
    assert_eq!(jump.duration_ms, Some(500));
    assert_eq!(jump.arrival_spell_id, 777);
    assert_eq!(jump.arrival_spell_target_guid, target);
    assert!(jump.has_flag(MOVEMENTGENERATOR_FLAG_INITIALIZATION_PENDING));
    assert!(!jump.has_flag(MOVEMENTGENERATOR_FLAG_PERSIST_ON_DEATH));
    assert_eq!(
        motion.base_unit_states.get(&UnitState::JUMPING.bits()),
        Some(&1)
    );

    assert!(motion.move_jump_with_gravity_like_cpp(2, 600, 1.0, None));
    let gravity_jump = motion.current_movement_generator();
    assert_eq!(gravity_jump.kind, MovementGeneratorKind::Effect);
    assert_eq!(gravity_jump.priority, MovementGeneratorPriority::Highest);
    assert_eq!(gravity_jump.base_unit_state, UnitState::JUMPING.bits());
    assert_eq!(gravity_jump.movement_id, 2);
    assert_eq!(gravity_jump.duration_ms, Some(600));
    assert_eq!(gravity_jump.arrival_spell_id, 0);
    assert_eq!(gravity_jump.arrival_spell_target_guid, ObjectGuid::EMPTY);
    assert!(gravity_jump.has_flag(MOVEMENTGENERATOR_FLAG_INITIALIZATION_PENDING));
    assert!(gravity_jump.has_flag(MOVEMENTGENERATOR_FLAG_PERSIST_ON_DEATH));
}

#[test]
fn knockback_generator_matches_cpp_player_guard_and_persist_flag() {
    let mut motion = MotionSubsystem::default();

    assert!(!motion.move_knockback_from_like_cpp(true, 300, 1.0));
    assert!(motion.active_generators.is_empty());

    assert!(!motion.move_knockback_from_like_cpp(false, 300, 0.009));
    assert!(motion.active_generators.is_empty());

    assert!(motion.move_knockback_from_like_cpp(false, 300, 0.01));
    let generator = motion.current_movement_generator();
    assert_eq!(generator.kind, MovementGeneratorKind::Effect);
    assert_eq!(generator.priority, MovementGeneratorPriority::Highest);
    assert_eq!(generator.base_unit_state, 0);
    assert_eq!(generator.movement_id, 0);
    assert_eq!(generator.duration_ms, Some(300));
    assert!(generator.has_flag(MOVEMENTGENERATOR_FLAG_INITIALIZATION_PENDING));
    assert!(generator.has_flag(MOVEMENTGENERATOR_FLAG_PERSIST_ON_DEATH));
}

#[test]
fn move_fall_like_cpp_guards_player_and_creature_spline_paths() {
    let mut motion = MotionSubsystem::default();

    assert_eq!(
        motion.move_fall_like_cpp(3, 400, false, 10.0, false, false),
        MoveFallPlan::Noop
    );
    assert_eq!(
        motion.move_fall_like_cpp(3, 400, true, 0.099, false, false),
        MoveFallPlan::Noop
    );
    assert_eq!(
        motion.move_fall_like_cpp(3, 400, true, 10.0, true, false),
        MoveFallPlan::Noop
    );
    assert!(motion.active_generators.is_empty());

    assert_eq!(
        motion.move_fall_like_cpp(3, 400, true, 10.0, false, true),
        MoveFallPlan::PlayerFallInfo
    );
    assert!(motion.active_generators.is_empty());

    assert_eq!(
        motion.move_fall_like_cpp(3, 400, true, 10.0, false, false),
        MoveFallPlan::SplineStarted
    );
    let generator = motion.current_movement_generator();
    assert_eq!(generator.kind, MovementGeneratorKind::Effect);
    assert_eq!(generator.priority, MovementGeneratorPriority::Highest);
    assert_eq!(generator.base_unit_state, 0);
    assert_eq!(generator.movement_id, 3);
    assert_eq!(generator.duration_ms, Some(400));
    assert!(generator.has_flag(MOVEMENTGENERATOR_FLAG_INITIALIZATION_PENDING));
    assert!(!generator.has_flag(MOVEMENTGENERATOR_FLAG_PERSIST_ON_DEATH));
}

#[test]
fn motion_stop_on_death_preserves_persistent_generators_like_cpp() {
    let mut motion = MotionSubsystem::default();
    motion.add_generator(
        MovementGeneratorRef::new(MovementGeneratorKind::Effect, MovementSlot::Active)
            .with_priority(MovementGeneratorPriority::Highest)
            .with_flags(MOVEMENTGENERATOR_FLAG_PERSIST_ON_DEATH),
    );

    assert!(!motion.stop_on_death());
    assert_eq!(
        motion.current_movement_generator().kind,
        MovementGeneratorKind::Effect
    );

    motion.clear_active();
    motion.move_point(9);
    motion.start_spline(7, 1_000);
    assert!(motion.stop_on_death());
    assert_eq!(motion.current_slot(), MovementSlot::Default);
    assert_eq!(
        motion.current_movement_generator().kind,
        MovementGeneratorKind::Idle
    );
    assert!(motion.stopped);
    assert!(!motion.spline.enabled);
}

#[test]
fn move_spline_runtime_state_tracks_cpp_finalized_cyclic_and_destination_shape() {
    let mut motion = MotionSubsystem::default();

    assert!(motion.spline.finalized);
    motion.launch_spline(77, 1_000, (10, 20, 30), false, true, Some(700));
    assert!(motion.spline.enabled);
    assert!(!motion.spline.finalized);
    assert!(motion.spline.on_transport);
    assert_eq!(motion.spline.final_destination, Some((10, 20, 30)));
    assert_eq!(motion.spline.velocity, Some(700));
    assert!(!motion.update_spline(999));
    assert_eq!(motion.spline.progress_ms, 999);
    assert!(motion.update_spline(1));
    assert!(motion.spline.finalized);
    assert!(!motion.spline.enabled);

    motion.launch_spline(78, 1_000, (1, 2, 3), true, false, None);
    assert!(!motion.update_spline(1_250));
    assert!(motion.spline.enabled);
    assert!(!motion.spline.finalized);
    assert_eq!(motion.spline.progress_ms, 250);
    motion.interrupt_spline();
    assert!(motion.spline.finalized);
    assert_eq!(motion.spline.current_destination, None);
}

#[test]
fn ai_stack_lock_and_scheduled_change_follow_cpp_unit_ai_shape() {
    let mut ai = AiSubsystem::default();

    assert!(!ai.is_enabled());
    ai.set_active(Some("NullAI"));
    assert!(ai.is_enabled());
    assert!(ai.update_tick(50));
    assert_eq!(ai.update_ticks, 1);
    assert_eq!(ai.last_update_diff_ms, 50);
    assert!(ai.just_summoned_gameobject_like_cpp());
    assert_eq!(ai.just_summoned_gameobject_count, 1);
    assert!(ai.summoned_gameobject_despawn_like_cpp());
    assert_eq!(ai.summoned_gameobject_despawn_count, 1);

    ai.push("CombatAI");
    assert_eq!(ai.active_ai.as_deref(), Some("CombatAI"));
    assert_eq!(ai.ai_stack, vec![String::from("NullAI")]);
    assert_eq!(ai.pop().as_deref(), Some("CombatAI"));
    assert_eq!(ai.active_ai.as_deref(), Some("NullAI"));

    ai.set_locked(true);
    ai.push("ScheduledChangeAI");
    assert_eq!(ai.active_ai.as_deref(), Some("NullAI"));
    assert!(ai.scheduled_change_pending);
    ai.set_locked(false);
    ai.apply_scheduled_change("ScheduledChangeAI", true);
    assert_eq!(ai.active_ai.as_deref(), Some("ScheduledChangeAI"));
    ai.apply_scheduled_change("RestoredAI", false);
    assert_eq!(ai.active_ai.as_deref(), Some("RestoredAI"));
    assert!(!ai.scheduled_change_pending);

    let mut disabled = AiSubsystem::default();
    assert!(!disabled.just_summoned_gameobject_like_cpp());
    assert_eq!(disabled.just_summoned_gameobject_count, 0);
    assert!(!disabled.summoned_gameobject_despawn_like_cpp());
    assert_eq!(disabled.summoned_gameobject_despawn_count, 0);
}

#[test]
fn control_summon_slots_match_cpp_shared_defines() {
    assert_eq!(SUMMON_SLOT_PET, 0);
    assert_eq!(SUMMON_SLOT_TOTEM, 1);
    assert_eq!(SUMMON_SLOT_TOTEM_2, 2);
    assert_eq!(SUMMON_SLOT_TOTEM_3, 3);
    assert_eq!(SUMMON_SLOT_TOTEM_4, 4);
    assert_eq!(SUMMON_SLOT_MINIPET, 5);
    assert_eq!(SUMMON_SLOT_QUEST, 6);
    assert_eq!(MAX_SUMMON_SLOT, 7);
    assert_eq!(MAX_GAMEOBJECT_SLOT, 4);
    assert_eq!(MAX_TOTEM_SLOT, 5);

    let mut control = ControlSubsystem::default();
    let pet = guid(40);
    let totem = guid(41);
    let gameobject = guid(43);

    assert_eq!(control.pet_guid(), ObjectGuid::EMPTY);
    control.set_pet_guid(pet);
    assert_eq!(control.pet_guid(), pet);
    assert!(control.set_summon_slot(SUMMON_SLOT_TOTEM_3, totem));
    assert_eq!(control.summon_slots[SUMMON_SLOT_TOTEM_3], totem);
    assert!(!control.set_summon_slot(MAX_SUMMON_SLOT, guid(42)));
    assert_eq!(control.clear_summon_slot(SUMMON_SLOT_TOTEM_3), Some(totem));
    assert_eq!(control.summon_slots[SUMMON_SLOT_TOTEM_3], ObjectGuid::EMPTY);

    control.register_owned_gameobject_like_cpp(gameobject);
    control.register_owned_gameobject_like_cpp(gameobject);
    assert_eq!(control.owned_gameobjects, vec![gameobject, gameobject]);
    assert!(control.set_gameobject_slot(2, gameobject));
    assert!(!control.set_gameobject_slot(MAX_GAMEOBJECT_SLOT, gameobject));
    assert!(control.clear_gameobject_slot_for_guid_like_cpp(gameobject));
    assert_eq!(control.gameobject_slots[2], ObjectGuid::EMPTY);
    assert!(control.remove_owned_gameobject_like_cpp(gameobject));
    assert!(control.owned_gameobjects.is_empty());
}

#[test]
fn charm_info_init_pet_action_bar_matches_cpp_defaults() {
    let mut charm_info = CharmInfoState::default();

    charm_info.init_pet_action_bar_like_cpp();

    assert_eq!(
        charm_info.action_bar[0],
        make_unit_action_button_like_cpp(COMMAND_ATTACK_LIKE_CPP, ACT_COMMAND_LIKE_CPP)
    );
    assert_eq!(
        charm_info.action_bar[1],
        make_unit_action_button_like_cpp(COMMAND_FOLLOW_LIKE_CPP, ACT_COMMAND_LIKE_CPP)
    );
    assert_eq!(
        charm_info.action_bar[2],
        make_unit_action_button_like_cpp(COMMAND_STAY_LIKE_CPP, ACT_COMMAND_LIKE_CPP)
    );
    for index in ACTION_BAR_INDEX_PET_SPELL_START..ACTION_BAR_INDEX_PET_SPELL_END {
        assert_eq!(
            charm_info.action_bar[index],
            make_unit_action_button_like_cpp(0, ACT_PASSIVE_LIKE_CPP)
        );
    }
    assert_eq!(
        charm_info.action_bar[7],
        make_unit_action_button_like_cpp(COMMAND_ATTACK_LIKE_CPP, ACT_REACTION_LIKE_CPP)
    );
    assert_eq!(
        charm_info.action_bar[8],
        make_unit_action_button_like_cpp(COMMAND_FOLLOW_LIKE_CPP, ACT_REACTION_LIKE_CPP)
    );
    assert_eq!(
        charm_info.action_bar[9],
        make_unit_action_button_like_cpp(COMMAND_STAY_LIKE_CPP, ACT_REACTION_LIKE_CPP)
    );
}

#[test]
fn charm_info_load_pet_action_bar_parses_twenty_tokens_like_cpp() {
    let mut charm_info = CharmInfoState::default();

    assert!(charm_info.load_pet_action_bar_like_cpp(
        "7 2 7 1 7 0 193 12345 129 23456 1 34567 193 45678 6 2 6 1 6 0"
    ));

    assert_eq!(
        charm_info.action_bar[0],
        make_unit_action_button_like_cpp(2, ACT_COMMAND_LIKE_CPP)
    );
    assert_eq!(
        charm_info.action_bar[3],
        make_unit_action_button_like_cpp(12_345, ACT_ENABLED_LIKE_CPP)
    );
    assert_eq!(
        charm_info.action_bar[4],
        make_unit_action_button_like_cpp(23_456, ACT_DISABLED_LIKE_CPP)
    );
    assert_eq!(
        unit_action_button_action_like_cpp(charm_info.action_bar[5]),
        34_567
    );
    assert_eq!(
        charm_info.action_bar[9],
        make_unit_action_button_like_cpp(0, ACT_REACTION_LIKE_CPP)
    );
}

#[test]
fn unit_action_button_type_keeps_low_type_bit_like_trinitycore() {
    let enabled = make_unit_action_button_like_cpp(12_345, ACT_ENABLED_LIKE_CPP);
    let disabled = make_unit_action_button_like_cpp(23_456, ACT_DISABLED_LIKE_CPP);
    let passive = make_unit_action_button_like_cpp(34_567, ACT_PASSIVE_LIKE_CPP);

    assert_eq!(
        unit_action_button_type_like_cpp(enabled),
        ACT_ENABLED_LIKE_CPP
    );
    assert_eq!(
        unit_action_button_type_like_cpp(disabled),
        ACT_DISABLED_LIKE_CPP
    );
    assert_eq!(
        unit_action_button_type_like_cpp(passive),
        ACT_PASSIVE_LIKE_CPP
    );
}

#[test]
fn charm_info_load_pet_action_bar_bad_shape_keeps_cpp_default_bar() {
    let mut charm_info = CharmInfoState::default();

    assert!(!charm_info.load_pet_action_bar_like_cpp("1 2 3"));

    assert_eq!(
        charm_info.action_bar[0],
        make_unit_action_button_like_cpp(COMMAND_ATTACK_LIKE_CPP, ACT_COMMAND_LIKE_CPP)
    );
    assert_eq!(
        charm_info.action_bar[3],
        make_unit_action_button_like_cpp(0, ACT_PASSIVE_LIKE_CPP)
    );
    assert_eq!(
        charm_info.action_bar[9],
        make_unit_action_button_like_cpp(COMMAND_STAY_LIKE_CPP, ACT_REACTION_LIKE_CPP)
    );
}

#[test]
fn control_charm_controller_and_target_state_follow_cpp_set_charm() {
    let mut controller = ControlSubsystem::default();
    let mut target = ControlSubsystem::default();
    let controller_guid = guid(50);
    let target_guid = guid(51);
    let other_guid = guid(52);

    controller.apply_charm_as_controller(target_guid, true);
    assert_eq!(controller.charmed_guid, Some(target_guid));
    assert!(controller.controlled_guids.contains(&target_guid));
    assert!(!controller.has_charm_info());

    assert!(target.apply_charmed_by(controller_guid, CharmType::Possess, true, Some(123), true,));
    assert_eq!(target.charmer_guid, Some(controller_guid));
    assert_eq!(target.charm_type, Some(CharmType::Possess));
    assert_eq!(target.old_faction_id, Some(123));
    assert!(target.walking_before_charm);
    assert!(target.is_charmed());
    assert!(target.is_possessed_by_player());
    assert!(target.has_charm_info());
    assert!(!target.apply_charmed_by(other_guid, CharmType::Charm, false, None, false,));

    assert!(!target.remove_charmed_by(Some(other_guid), false));
    assert!(target.remove_charmed_by(Some(controller_guid), false));
    assert_eq!(target.charmer_guid, None);
    assert_eq!(target.last_charmer_guid, Some(controller_guid));
    assert_eq!(target.charm_type, None);
    assert_eq!(target.old_faction_id, None);
    assert!(!target.has_charm_info());

    controller.remove_charm_as_controller(target_guid, false, true, false);
    assert_eq!(controller.charmed_guid, None);
    assert!(!controller.controlled_guids.contains(&target_guid));
}

#[test]
fn control_remove_charm_preserves_owned_minions_like_cpp() {
    let mut controller = ControlSubsystem::default();
    let minion = guid(60);

    controller.apply_charm_as_controller(minion, false);
    controller.remove_charm_as_controller(minion, true, true, false);
    assert!(controller.controlled_guids.contains(&minion));

    controller.remove_charm_as_controller(minion, true, false, false);
    assert!(!controller.controlled_guids.contains(&minion));
}

#[test]
fn control_remove_vehicle_charm_does_not_mark_last_charmer_like_cpp() {
    let mut passenger = ControlSubsystem::default();
    let vehicle = guid(65);

    assert!(passenger.apply_charmed_by(vehicle, CharmType::Vehicle, true, Some(321), false,));
    assert_eq!(passenger.charmer_guid, Some(vehicle));
    assert_eq!(passenger.charm_type, Some(CharmType::Vehicle));
    assert!(!passenger.has_charm_info());

    assert!(passenger.remove_charmed_by(Some(vehicle), false));
    assert_eq!(passenger.charmer_guid, None);
    assert_eq!(passenger.last_charmer_guid, None);
    assert_eq!(passenger.charm_type, None);
    assert_eq!(passenger.old_faction_id, None);
}

#[test]
fn charm_info_direct_control_and_shared_vision_helpers_roundtrip() {
    let mut control = ControlSubsystem::default();
    let controller = guid(70);
    let controlled = guid(71);
    let observer = guid(72);

    control.set_owner_guid(Some(controller));
    assert_eq!(control.charmer_or_owner_guid(), Some(controller));
    assert_eq!(
        control.charmer_or_owner_or_self_guid(controlled),
        controller
    );

    let charm_info = control.init_charm_info();
    charm_info.pet_number = 9;
    charm_info.command_state = 2;
    charm_info.action_bar[0] = 100;
    charm_info.charm_spells[0] = 200;
    charm_info.is_command_follow = true;
    charm_info.stay_position = Some((1.0, 2.0, 3.0));
    assert!(control.has_charm_info());
    assert_eq!(
        control.charm_info.as_ref().map(|info| info.pet_number),
        Some(9)
    );

    control.add_controlled(controlled);
    control.set_charmed(controlled);
    control.set_moved_unit(Some(controlled));
    control.set_player_moving_me(Some(controller));
    assert!(control.is_possessing_guid(controlled));
    assert_eq!(control.unit_moved_by_me, Some(controlled));
    assert_eq!(control.player_moving_me, Some(controller));

    assert!(control.add_shared_vision(observer));
    assert!(control.has_shared_vision());
    assert!(control.remove_shared_vision(observer));
    assert!(!control.has_shared_vision());

    let removed = control.remove_all_controlled();
    assert_eq!(removed, vec![controlled]);
    assert_eq!(control.charmed_guid, None);
    control.delete_charm_info();
    assert!(!control.has_charm_info());
}

#[test]
fn vehicle_remove_kit_without_kit_returns_before_send_like_cpp() {
    let mut vehicle = VehicleSubsystem::default();

    let remove = vehicle.remove_vehicle_kit_like_cpp(false);

    assert_eq!(remove.kit_id, None);
    assert!(!remove.had_kit);
    assert_eq!(remove.previous_installed, None);
    assert!(!remove.on_remove_from_world);
    assert!(!remove.send_set_vehicle_rec_id_zero_represented);
    assert!(!remove.uninstall_represented);
    assert!(!remove.remove_all_passengers_represented);
    assert!(!remove.script_on_uninstall_represented);
    assert!(!remove.kit_cleared);
    assert_eq!(vehicle.kit, None);
}

#[test]
fn vehicle_remove_existing_kit_sends_rec_id_zero_before_uninstall_like_cpp() {
    let mut vehicle = VehicleSubsystem::default();
    vehicle.set_vehicle_kit(467, true);
    let install = vehicle.install_vehicle_kit_like_cpp();
    assert_eq!(install.kit_id, Some(467));
    assert!(install.installed);

    let remove = vehicle.remove_vehicle_kit_like_cpp(false);

    assert_eq!(remove.kit_id, Some(467));
    assert!(remove.had_kit);
    assert_eq!(remove.previous_installed, Some(true));
    assert!(!remove.on_remove_from_world);
    assert!(remove.send_set_vehicle_rec_id_zero_represented);
    assert!(remove.uninstall_represented);
    assert!(remove.remove_all_passengers_represented);
    assert!(remove.script_on_uninstall_represented);
    assert!(remove.kit_cleared);
    assert_eq!(vehicle.kit, None);
}

#[test]
fn motion_charm_vehicle_and_ai_helpers_roundtrip() {
    let mut subsystems = UnitSubsystems::default();
    let controller = guid(20);
    let controlled = guid(21);
    let vehicle = guid(30);

    subsystems
        .motion
        .set_current_generator(MovementGeneratorKind::Chase);
    subsystems.motion.start_spline(7, 1_000);
    subsystems.motion.set_spline_progress(1_500);
    assert_eq!(
        subsystems.motion.current_generator,
        MovementGeneratorKind::Chase
    );
    assert_eq!(subsystems.motion.spline.progress_ms, 1_000);
    subsystems.motion.pause_movement();
    assert!(subsystems.motion.paused);
    subsystems.motion.resume_movement();
    subsystems.motion.stop_moving();
    assert!(!subsystems.motion.paused);
    assert!(subsystems.motion.stopped);
    assert!(!subsystems.motion.spline.enabled);

    subsystems.control.set_charmer(controller, true);
    subsystems.control.set_charmed(controlled);
    subsystems.control.unit_moved_by_me = Some(controlled);
    subsystems.control.player_moving_me = Some(controller);
    assert!(subsystems.control.is_charmed());
    assert!(subsystems.control.controlled_by_player);
    assert!(subsystems.control.controlled_guids.contains(&controlled));
    assert!(subsystems.control.add_shared_vision(controlled));
    subsystems.control.remove_charmed();
    subsystems.control.remove_charmer();
    assert!(!subsystems.control.is_charmed());
    assert_eq!(subsystems.control.last_charmer_guid, Some(controller));

    subsystems.vehicle.enter_vehicle(vehicle, Some(1));
    subsystems.vehicle.base_vehicle_guid = Some(vehicle);
    subsystems.vehicle.set_vehicle_kit(42, true);
    assert_eq!(subsystems.vehicle.vehicle_guid, Some(vehicle));
    assert_eq!(subsystems.vehicle.seat_id, Some(1));
    assert_eq!(
        subsystems.vehicle.kit.as_ref().map(|kit| kit.kit_id),
        Some(42)
    );
    assert_eq!(
        subsystems.vehicle.kit.as_ref().map(|kit| kit.installed),
        Some(false)
    );
    let install = subsystems.vehicle.install_vehicle_kit_like_cpp();
    assert_eq!(install.kit_id, Some(42));
    assert!(install.had_kit);
    assert_eq!(install.previous_installed, Some(false));
    assert!(install.installed);
    assert!(install.script_on_install_represented);
    let reinstall = subsystems.vehicle.install_vehicle_kit_like_cpp();
    assert_eq!(reinstall.previous_installed, Some(true));
    assert!(reinstall.installed);
    subsystems.vehicle.exit_vehicle();
    subsystems.vehicle.clear_vehicle_kit();
    assert_eq!(subsystems.vehicle.vehicle_guid, None);
    assert_eq!(subsystems.vehicle.kit, None);
    let missing_install = subsystems.vehicle.install_vehicle_kit_like_cpp();
    assert_eq!(missing_install.kit_id, None);
    assert!(!missing_install.had_kit);
    assert_eq!(missing_install.previous_installed, None);
    assert!(!missing_install.installed);
    assert!(!missing_install.script_on_install_represented);

    subsystems.vehicle.set_vehicle_kit(43, true);
    let install_before_remove = subsystems.vehicle.install_vehicle_kit_like_cpp();
    assert!(install_before_remove.installed);
    subsystems.vehicle.vehicle_guid = Some(vehicle);
    subsystems.vehicle.base_vehicle_guid = Some(vehicle);
    subsystems.vehicle.seat_id = Some(2);
    let remove = subsystems.vehicle.remove_vehicle_kit_like_cpp(true);
    assert_eq!(remove.kit_id, Some(43));
    assert!(remove.had_kit);
    assert_eq!(remove.previous_installed, Some(true));
    assert!(remove.on_remove_from_world);
    assert!(!remove.send_set_vehicle_rec_id_zero_represented);
    assert!(remove.uninstall_represented);
    assert!(remove.remove_all_passengers_represented);
    assert!(remove.script_on_uninstall_represented);
    assert!(remove.kit_cleared);
    assert_eq!(subsystems.vehicle.kit, None);
    assert_eq!(subsystems.vehicle.vehicle_guid, Some(vehicle));
    assert_eq!(subsystems.vehicle.base_vehicle_guid, Some(vehicle));
    assert_eq!(subsystems.vehicle.seat_id, Some(2));
    let missing_remove = subsystems.vehicle.remove_vehicle_kit_like_cpp(true);
    assert_eq!(missing_remove.kit_id, None);
    assert!(!missing_remove.had_kit);
    assert_eq!(missing_remove.previous_installed, None);
    assert!(missing_remove.on_remove_from_world);
    assert!(!missing_remove.send_set_vehicle_rec_id_zero_represented);
    assert!(!missing_remove.uninstall_represented);
    assert!(!missing_remove.remove_all_passengers_represented);
    assert!(!missing_remove.script_on_uninstall_represented);
    assert!(!missing_remove.kit_cleared);

    subsystems.ai.set_active(Some("NullAI"));
    subsystems.ai.push("CombatAI");
    assert_eq!(subsystems.ai.active_ai.as_deref(), Some("CombatAI"));
    assert_eq!(subsystems.ai.ai_stack, vec![String::from("NullAI")]);
    assert_eq!(subsystems.ai.pop().as_deref(), Some("CombatAI"));
    assert_eq!(subsystems.ai.active_ai.as_deref(), Some("NullAI"));
    subsystems.ai.set_locked(true);
    assert!(subsystems.ai.locked);
}
