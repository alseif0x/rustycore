//! Unit values, visibility and health-revision state regression scenarios, part 1 of 3.
//!
//! Moved out of the unit.rs root under #636; every test is unchanged.

use super::*;

#[test]
fn unit_constructor_matches_cpp_base_state() {
    let unit = Unit::new(true);

    assert_eq!(unit.world().object().type_id(), TypeId::Unit);
    assert_eq!(
        unit.world().object().type_mask(),
        TypeMask::OBJECT | TypeMask::UNIT
    );
    assert!(
        unit.world()
            .object()
            .create_flags()
            .contains(crate::CreateObjectFlags::MOVEMENT_UPDATE)
    );
    assert_eq!(unit.death_state(), DeathState::Alive);
    assert_eq!(unit.unit_state(), 0);
    assert_eq!(unit.attacking(), None);
    assert_eq!(unit.base_attack_speed(), [0; MAX_ATTACK]);
    assert_eq!(unit.mod_attack_speed_pct(), [1.0; MAX_ATTACK]);
    assert_eq!(unit.attack_timer(WeaponAttackType::BaseAttack), 0);
    assert_eq!(unit.attack_timer(WeaponAttackType::OffAttack), 0);
    assert!(!unit.can_dual_wield_like_cpp());
    assert!(!unit.can_parry_like_cpp());
    assert!(!unit.can_block_like_cpp());
    assert_eq!(unit.emote_state_like_cpp(), 0);
    assert_eq!(unit.weapon_damage(WeaponAttackType::BaseAttack), [1.0, 2.0]);
    assert_eq!(unit.speed_rate(), [1.0; MAX_MOVE_TYPE]);
    assert_eq!(unit.collision_height_like_cpp(), 0.0);
    assert_eq!(unit.world().collision_height_like_cpp(), 0.0);
    assert!(unit.subsystems().auras.owned_auras.is_empty());
    assert!(unit.subsystems().auras.applied_auras.is_empty());
    assert!(unit.subsystems().auras.interruptible_auras.is_empty());
    assert!(unit.subsystems().auras.aura_state_auras.is_empty());
    assert_eq!(unit.subsystems().auras.aura_state_mask, 0);
    assert_eq!(unit.subsystems().auras.removed_auras_count, 0);
    assert!(unit.subsystems().auras.can_proc());
    assert!(unit.subsystems().spells.current_spells.is_empty());
    assert!(unit.subsystems().spells.history.cooldowns.is_empty());
    assert!(unit.subsystems().combat.threat.is_empty());
    assert!(unit.subsystems().combat.threat_refs.is_empty());
    assert!(unit.subsystems().combat.threatened_by_me.is_empty());
    assert!(unit.subsystems().combat.pve_refs.is_empty());
    assert!(unit.subsystems().combat.pvp_refs.is_empty());
    assert_eq!(unit.subsystems().combat.current_victim_guid, None);
    assert_eq!(unit.subsystems().combat.fixate_guid, None);
    assert!(unit.subsystems().combat.attackers.is_empty());
    assert!(unit.subsystems().combat.extra_attacks_targets.is_empty());
    assert_eq!(unit.subsystems().combat.attacking_guid, None);
    assert!(!unit.subsystems().combat.combat_disallowed);
    assert_eq!(
        unit.subsystems().motion.current_generator,
        MovementGeneratorKind::Idle
    );
    assert!(!unit.subsystems().motion.paused);
    assert!(!unit.subsystems().motion.spline.enabled);
    assert!(unit.subsystems().motion.spline.finalized);
    assert_eq!(unit.subsystems().control.charmer_guid, None);
    assert_eq!(unit.subsystems().control.owner_guid, None);
    assert_eq!(unit.subsystems().control.minion_guid, None);
    assert_eq!(
        unit.subsystems().control.summon_slots,
        [ObjectGuid::EMPTY; MAX_SUMMON_SLOT]
    );
    assert!(!unit.subsystems().control.has_charm_info());
    assert_eq!(unit.subsystems().vehicle.vehicle_guid, None);
    assert_eq!(unit.subsystems().ai.active_ai, None);
    assert!(!unit.subsystems().ai.locked);
    assert!(!unit.subsystems().ai.scheduled_change_pending);
    assert!(!unit.unit_data_changes_mask().is_any_set());
}

#[test]
fn add_extra_attacks_uses_last_damaged_then_selection_like_cpp() {
    let mut unit = Unit::new(true);
    let selected = ObjectGuid::new(7, 11);
    let last_damaged = ObjectGuid::new(7, 12);

    assert_eq!(unit.add_extra_attacks_like_cpp(2), None);

    unit.set_target(selected);
    assert_eq!(unit.add_extra_attacks_like_cpp(2), Some(selected));
    assert_eq!(unit.extra_attacks_for_like_cpp(selected), 2);

    unit.set_last_damaged_target_like_cpp(Some(last_damaged));
    assert_eq!(unit.add_extra_attacks_like_cpp(3), Some(last_damaged));
    assert_eq!(unit.extra_attacks_for_like_cpp(selected), 2);
    assert_eq!(unit.extra_attacks_for_like_cpp(last_damaged), 3);

    unit.set_last_damaged_target_like_cpp(None);
    assert_eq!(unit.add_extra_attacks_like_cpp(u32::MAX), Some(selected));
    assert_eq!(
        unit.extra_attacks_for_like_cpp(selected),
        u32::MAX,
        "represented storage saturates instead of wrapping the queued count"
    );
}

#[test]
fn unit_defensive_capability_flags_roundtrip_like_cpp() {
    let mut unit = Unit::new(true);

    assert!(!unit.can_parry_like_cpp());
    assert!(!unit.can_block_like_cpp());

    unit.set_can_parry_like_cpp(true);
    unit.set_can_block_like_cpp(true);

    assert!(unit.can_parry_like_cpp());
    assert!(unit.can_block_like_cpp());
}

#[test]
fn unit_add_to_world_like_cpp_removes_enter_world_auras_and_initializes_motion_once() {
    let mut unit = Unit::new(true);
    let caster = ObjectGuid::new(0, 47501);
    let enter_world_aura = AppliedAuraRef::new(47_501, caster, 1, 0x1);
    let attacking_aura = AppliedAuraRef::new(47_502, caster, 2, 0x1);
    unit.subsystems_mut().auras.register_applied_aura(
        enter_world_aura,
        None,
        SPELL_AURA_INTERRUPT_FLAG_ENTER_WORLD_LIKE_CPP,
        0,
    );
    unit.subsystems_mut().auras.register_applied_aura(
        attacking_aura,
        None,
        SPELL_AURA_INTERRUPT_FLAG_ATTACKING_LIKE_CPP,
        0,
    );
    unit.subsystems_mut().motion.push_delayed_payload_like_cpp(
        MotionMasterDelayedActionPayload::Add(MovementGeneratorRef::new(
            MovementGeneratorKind::Point,
            MovementSlot::Active,
        )),
    );

    let first = unit.add_to_world_like_cpp();

    assert_eq!(first.guid, unit.world().object().guid());
    assert!(first.world_object_added);
    assert!(first.is_in_world_after);
    assert_eq!(
        first.aura_interrupt_flags_enter_world,
        SPELL_AURA_INTERRUPT_FLAG_ENTER_WORLD_LIKE_CPP
    );
    assert_eq!(first.removed_enter_world_auras, vec![enter_world_aura]);
    assert!(!unit.subsystems().auras.has_applied(enter_world_aura));
    assert!(unit.subsystems().auras.has_applied(attacking_aura));

    let motion = first.motion_master_add_to_world;
    assert!(motion.had_initialization_pending);
    assert!(motion.entered_initializing);
    assert!(motion.direct_initialize_represented);
    assert!(motion.exited_initializing);
    assert_eq!(motion.resolved_delayed_actions.len(), 1);
    assert!(
        !unit
            .subsystems()
            .motion
            .has_motion_master_flag(MOTIONMASTER_FLAG_INITIALIZATION_PENDING)
    );
    assert!(
        !unit
            .subsystems()
            .motion
            .has_motion_master_flag(MOTIONMASTER_FLAG_INITIALIZING)
    );
    assert_eq!(
        unit.subsystems().motion.current_generator,
        MovementGeneratorKind::Point
    );

    let second = unit.add_to_world_like_cpp();

    assert!(!second.motion_master_add_to_world.had_initialization_pending);
    assert!(
        !second
            .motion_master_add_to_world
            .direct_initialize_represented
    );
    assert!(second.removed_enter_world_auras.is_empty());
    assert!(unit.subsystems().auras.has_applied(attacking_aura));
    assert_eq!(
        unit.subsystems().motion.current_generator,
        MovementGeneratorKind::Point
    );
}

#[test]
fn shared_vision_add_empty_to_non_empty_activates_and_requests_world_object_on_like_cpp() {
    let mut unit = Unit::new(false);
    let unit_guid = ObjectGuid::new(1, 42);
    let player_guid = ObjectGuid::new(1, 100);
    unit.world_mut().object_mut().create(unit_guid);

    let outcome = unit.add_player_to_vision_like_cpp(player_guid);

    assert_eq!(outcome.player_guid, player_guid);
    assert!(outcome.inserted_or_removed);
    assert_eq!(
        outcome.set_world_object,
        Some(UnitSharedVisionSetWorldObjectRequestLikeCpp {
            unit_guid,
            on: true,
        })
    );
    assert!(unit.world().is_active());
    assert!(!unit.world().is_world_object());
    assert!(unit.subsystems().control.has_shared_vision());
}

#[test]
fn shared_vision_add_non_empty_or_duplicate_does_not_request_world_object_again_like_cpp() {
    let mut unit = Unit::new(false);
    let unit_guid = ObjectGuid::new(1, 43);
    let first_player = ObjectGuid::new(1, 101);
    let second_player = ObjectGuid::new(1, 102);
    unit.world_mut().object_mut().create(unit_guid);

    let first = unit.add_player_to_vision_like_cpp(first_player);
    assert!(first.set_world_object.is_some());

    let second = unit.add_player_to_vision_like_cpp(second_player);
    assert_eq!(second.player_guid, second_player);
    assert!(second.inserted_or_removed);
    assert_eq!(second.set_world_object, None);
    assert!(unit.world().is_active());

    let duplicate = unit.add_player_to_vision_like_cpp(second_player);
    assert_eq!(duplicate.player_guid, second_player);
    assert!(!duplicate.inserted_or_removed);
    assert_eq!(duplicate.set_world_object, None);
    assert!(unit.world().is_active());
}

#[test]
fn shared_vision_remove_keeps_active_until_last_viewer_then_requests_off_like_cpp() {
    let mut unit = Unit::new(false);
    let unit_guid = ObjectGuid::new(1, 44);
    let first_player = ObjectGuid::new(1, 103);
    let second_player = ObjectGuid::new(1, 104);
    unit.world_mut().object_mut().create(unit_guid);
    unit.add_player_to_vision_like_cpp(first_player);
    unit.add_player_to_vision_like_cpp(second_player);

    let first_remove = unit.remove_player_from_vision_like_cpp(first_player);
    assert_eq!(first_remove.player_guid, first_player);
    assert!(first_remove.inserted_or_removed);
    assert_eq!(first_remove.set_world_object, None);
    assert!(unit.world().is_active());
    assert!(unit.subsystems().control.has_shared_vision());

    let last_remove = unit.remove_player_from_vision_like_cpp(second_player);
    assert_eq!(last_remove.player_guid, second_player);
    assert!(last_remove.inserted_or_removed);
    assert_eq!(
        last_remove.set_world_object,
        Some(UnitSharedVisionSetWorldObjectRequestLikeCpp {
            unit_guid,
            on: false,
        })
    );
    assert!(!unit.world().is_active());
    assert!(!unit.world().is_world_object());
    assert!(!unit.subsystems().control.has_shared_vision());
}

#[test]
fn shared_vision_remove_absent_from_empty_still_requests_off_like_cpp() {
    let mut unit = Unit::new(false);
    let unit_guid = ObjectGuid::new(1, 45);
    let absent_player = ObjectGuid::new(1, 105);
    unit.world_mut().object_mut().create(unit_guid);
    unit.world_mut().set_active(true);

    let outcome = unit.remove_player_from_vision_like_cpp(absent_player);

    assert_eq!(outcome.player_guid, absent_player);
    assert!(!outcome.inserted_or_removed);
    assert_eq!(
        outcome.set_world_object,
        Some(UnitSharedVisionSetWorldObjectRequestLikeCpp {
            unit_guid,
            on: false,
        })
    );
    assert!(!unit.world().is_active());
    assert!(!unit.world().is_world_object());
    assert!(!unit.subsystems().control.has_shared_vision());
}

#[test]
fn shared_vision_remove_absent_keeps_active_when_other_viewers_remain_like_cpp() {
    let mut unit = Unit::new(false);
    let unit_guid = ObjectGuid::new(1, 46);
    let present_player = ObjectGuid::new(1, 106);
    let absent_player = ObjectGuid::new(1, 107);
    unit.world_mut().object_mut().create(unit_guid);
    unit.add_player_to_vision_like_cpp(present_player);

    let outcome = unit.remove_player_from_vision_like_cpp(absent_player);

    assert_eq!(outcome.player_guid, absent_player);
    assert!(!outcome.inserted_or_removed);
    assert_eq!(outcome.set_world_object, None);
    assert!(unit.world().is_active());
    assert!(unit.subsystems().control.has_shared_vision());
}

#[test]
fn attacking_uses_combat_subsystem_as_single_source_of_truth() {
    let mut unit = Unit::new(true);
    let victim = ObjectGuid::new(1, 10);
    let other_victim = ObjectGuid::new(1, 11);

    unit.set_attacking(Some(victim));
    assert_eq!(unit.attacking(), Some(victim));
    assert_eq!(unit.subsystems().combat.attacking_guid, Some(victim));

    unit.subsystems_mut()
        .combat
        .set_attacking(Some(other_victim));
    assert_eq!(unit.attacking(), Some(other_victim));

    unit.subsystems_mut().combat.clear_attackers();
    assert_eq!(unit.attacking(), None);

    unit.set_attacking(Some(victim));
    unit.subsystems_mut().clear_runtime_state();
    assert_eq!(unit.attacking(), None);
}

#[test]
fn attack_like_cpp_records_target_and_melee_state() {
    let mut unit = Unit::new(true);
    let attacker = ObjectGuid::new(1, 9);
    let victim = ObjectGuid::new(1, 10);
    unit.world_mut().object_mut().create(attacker);

    assert_eq!(
        unit.attack_like_cpp(victim, true, true, true),
        UnitAttackStartOutcome::NewTarget { previous: None }
    );
    assert_eq!(unit.attacking(), Some(victim));
    assert_eq!(unit.data().target, victim);
    assert!(unit.has_unit_state(UnitState::MELEE_ATTACKING.bits()));

    assert_eq!(
        unit.attack_like_cpp(victim, true, true, true),
        UnitAttackStartOutcome::NoChangeSameTarget
    );
    assert_eq!(
        unit.attack_like_cpp(victim, true, true, false),
        UnitAttackStartOutcome::MeleeStoppedSameTarget
    );
    assert!(!unit.has_unit_state(UnitState::MELEE_ATTACKING.bits()));
    assert_eq!(
        unit.attack_like_cpp(victim, true, true, true),
        UnitAttackStartOutcome::MeleeStartedSameTarget
    );
    assert!(unit.has_unit_state(UnitState::MELEE_ATTACKING.bits()));
}

#[test]
fn attack_like_cpp_switches_target_and_interrupts_melee_spell() {
    let mut unit = Unit::new(true);
    let attacker = ObjectGuid::new(1, 9);
    let first = ObjectGuid::new(1, 10);
    let second = ObjectGuid::new(1, 11);
    let melee = CurrentSpellRef::new(700, Some(attacker), None).with_cast_time_ms(1_000);
    unit.world_mut().object_mut().create(attacker);

    assert_eq!(
        unit.attack_like_cpp(first, true, true, true),
        UnitAttackStartOutcome::NewTarget { previous: None }
    );
    unit.subsystems_mut()
        .spells
        .set_current_spell(CurrentSpellSlot::Melee, melee);
    assert_eq!(
        unit.attack_like_cpp(second, true, true, false),
        UnitAttackStartOutcome::NewTarget {
            previous: Some(first)
        }
    );

    assert_eq!(unit.attacking(), Some(second));
    assert_eq!(unit.data().target, second);
    assert_eq!(unit.current_spell(CurrentSpellSlot::Melee), None);
    assert!(!unit.has_unit_state(UnitState::MELEE_ATTACKING.bits()));
}

#[test]
fn attack_stop_like_cpp_clears_target_melee_state_and_spell() {
    let mut unit = Unit::new(true);
    let attacker = ObjectGuid::new(1, 9);
    let victim = ObjectGuid::new(1, 10);
    let melee = CurrentSpellRef::new(701, Some(attacker), None).with_cast_time_ms(1_000);
    unit.world_mut().object_mut().create(attacker);
    unit.attack_like_cpp(victim, true, true, true);
    unit.subsystems_mut()
        .spells
        .set_current_spell(CurrentSpellSlot::Melee, melee);

    assert_eq!(
        unit.attack_stop_like_cpp(),
        UnitAttackStopOutcome::Stopped { victim }
    );
    assert_eq!(unit.attacking(), None);
    assert_eq!(unit.data().target, ObjectGuid::EMPTY);
    assert_eq!(unit.current_spell(CurrentSpellSlot::Melee), None);
    assert!(!unit.has_unit_state(UnitState::MELEE_ATTACKING.bits()));
    assert_eq!(unit.attack_stop_like_cpp(), UnitAttackStopOutcome::NoVictim);
}

#[test]
fn attacker_set_helpers_match_cpp_insert_erase_shape() {
    let mut victim = Unit::new(true);
    let attacker = ObjectGuid::new(1, 9);

    assert!(victim.add_attacker_like_cpp(attacker));
    assert!(!victim.add_attacker_like_cpp(attacker));
    assert!(victim.has_attacker_like_cpp(attacker));
    assert!(victim.remove_attacker_like_cpp(attacker));
    assert!(!victim.remove_attacker_like_cpp(attacker));
    assert!(!victim.has_attacker_like_cpp(attacker));
}

#[test]
fn attack_with_context_like_cpp_rejects_mounted_evading_and_gm_targets() {
    let mut unit = Unit::new(true);
    let attacker = ObjectGuid::new(1, 9);
    let victim = ObjectGuid::new(1, 10);
    unit.world_mut().object_mut().create(attacker);

    assert_eq!(
        unit.attack_with_context_like_cpp(
            victim,
            true,
            true,
            true,
            UnitAttackContextLikeCpp {
                attacker_is_mounted_player: true,
                ..Default::default()
            },
        ),
        UnitAttackStartOutcome::InvalidMountedAttacker
    );
    assert_eq!(unit.attacking(), None);

    assert_eq!(
        unit.attack_with_context_like_cpp(
            victim,
            true,
            true,
            true,
            UnitAttackContextLikeCpp {
                attacker_is_evading_creature: true,
                ..Default::default()
            },
        ),
        UnitAttackStartOutcome::InvalidAttackerEvading
    );
    assert_eq!(
        unit.attack_with_context_like_cpp(
            victim,
            true,
            true,
            true,
            UnitAttackContextLikeCpp {
                victim_is_game_master_player: true,
                ..Default::default()
            },
        ),
        UnitAttackStartOutcome::InvalidVictimGameMaster
    );
    assert_eq!(
        unit.attack_with_context_like_cpp(
            victim,
            true,
            true,
            true,
            UnitAttackContextLikeCpp {
                victim_is_evading_creature: true,
                ..Default::default()
            },
        ),
        UnitAttackStartOutcome::InvalidVictimEvading
    );
}

#[test]
fn valid_attack_target_represented_rejects_cpp_unit_state_flags_and_immunities() {
    assert!(!Unit::is_valid_attack_target_represented_like_cpp(
        &UnitAttackContextLikeCpp {
            visibility_represented: true,
            attacker_can_see_or_detect_target: false,
            ..Default::default()
        }
    ));
    assert!(!Unit::is_valid_attack_target_represented_like_cpp(
        &UnitAttackContextLikeCpp {
            victim_unit_state: UnitState::IN_FLIGHT.bits(),
            ..Default::default()
        }
    ));
    for flag in [
        UnitFlags::NON_ATTACKABLE,
        UnitFlags::NON_ATTACKABLE_2,
        UnitFlags::ON_TAXI,
        UnitFlags::NOT_ATTACKABLE_1,
        UnitFlags::UNINTERACTIBLE,
    ] {
        assert!(!Unit::is_valid_attack_target_represented_like_cpp(
            &UnitAttackContextLikeCpp {
                victim_unit_flags: flag.bits(),
                ..Default::default()
            }
        ));
    }
    assert!(!Unit::is_valid_attack_target_represented_like_cpp(
        &UnitAttackContextLikeCpp {
            attacker_is_player_uber: true,
            ..Default::default()
        }
    ));
    assert!(!Unit::is_valid_attack_target_represented_like_cpp(
        &UnitAttackContextLikeCpp {
            victim_unit_flags: UnitFlags::IMMUNE_TO_NPC.bits(),
            ..Default::default()
        }
    ));
    assert!(!Unit::is_valid_attack_target_represented_like_cpp(
        &UnitAttackContextLikeCpp {
            attacker_unit_flags: UnitFlags::IMMUNE_TO_NPC.bits(),
            ..Default::default()
        }
    ));
    assert!(!Unit::is_valid_attack_target_represented_like_cpp(
        &UnitAttackContextLikeCpp {
            attacker_unit_flags: UnitFlags::PLAYER_CONTROLLED.bits(),
            victim_unit_flags: UnitFlags::IMMUNE_TO_PC.bits(),
            ..Default::default()
        }
    ));
    assert!(!Unit::is_valid_attack_target_represented_like_cpp(
        &UnitAttackContextLikeCpp {
            attacker_unit_flags: UnitFlags::IMMUNE_TO_PC.bits(),
            victim_unit_flags: UnitFlags::PLAYER_CONTROLLED.bits(),
            ..Default::default()
        }
    ));
    assert!(Unit::is_valid_attack_target_represented_like_cpp(
        &UnitAttackContextLikeCpp {
            attacker_unit_flags: UnitFlags::PLAYER_CONTROLLED.bits(),
            victim_unit_flags: UnitFlags::IMMUNE_TO_NPC.bits(),
            ..Default::default()
        }
    ));
}

#[test]
fn can_see_or_detect_unit_like_cpp_rejects_gm_visibility_above_detect() {
    let mut seer = Unit::new(true);
    let mut target = Unit::new(true);

    target.set_server_side_gm_visibility_like_cpp(2);
    seer.set_server_side_gm_visibility_detect_like_cpp(1);
    assert!(!seer.can_see_or_detect_unit_like_cpp(&target, false, true, false));

    seer.set_server_side_gm_visibility_detect_like_cpp(2);
    assert!(seer.can_see_or_detect_unit_like_cpp(&target, false, true, false));

    target.set_invisibility_like_cpp(0, 100);
    assert!(seer.can_see_or_detect_unit_like_cpp(&target, false, true, false));
}

#[test]
fn can_detect_invisibility_like_cpp_requires_flag_and_sufficient_value() {
    let mut seer = Unit::new(true);
    let mut target = Unit::new(true);

    target.set_invisibility_like_cpp(3, 25);
    assert!(!seer.can_see_or_detect_unit_like_cpp(&target, false, true, false));

    seer.set_invisibility_detect_like_cpp(2, 100);
    assert!(!seer.can_detect_invisibility_of_like_cpp(&target));

    seer.set_invisibility_detect_like_cpp(3, 24);
    assert!(!seer.can_detect_invisibility_of_like_cpp(&target));

    seer.set_invisibility_detect_like_cpp(3, 25);
    assert!(seer.can_see_or_detect_unit_like_cpp(&target, false, true, false));
    assert!(seer.can_see_or_detect_unit_like_cpp(&target, true, true, false));

    target.set_invisibility_like_cpp(37, 12);
    assert!(!seer.can_detect_invisibility_of_like_cpp(&target));
    seer.set_invisibility_detect_like_cpp(37, 12);
    assert!(seer.can_detect_invisibility_of_like_cpp(&target));
}

#[test]
fn can_detect_stealth_like_cpp_uses_front_arc_level_distance_and_player_cap() {
    let mut seer = Unit::new(true);
    let mut target = Unit::new(true);
    seer.set_level(10);
    seer.world_mut()
        .relocate(wow_core::Position::new(0.0, 0.0, 0.0, 0.0));
    target
        .world_mut()
        .relocate(wow_core::Position::new(20.0, 0.0, 0.0, 0.0));
    target.set_stealth_like_cpp(0, 1);
    assert!(seer.can_detect_stealth_of_like_cpp(&target, true, false));

    target.set_stealth_like_cpp(0, 100);
    assert!(!seer.can_detect_stealth_of_like_cpp(&target, true, false));

    seer.set_stealth_detect_like_cpp(0, 100);
    assert!(seer.can_detect_stealth_of_like_cpp(&target, true, false));

    target
        .world_mut()
        .relocate(wow_core::Position::new(-20.0, 0.0, 0.0, 0.0));
    assert!(!seer.can_detect_stealth_of_like_cpp(&target, true, false));

    target
        .world_mut()
        .relocate(wow_core::Position::new(35.0, 0.0, 0.0, 0.0));
    seer.set_level(80);
    target.set_stealth_like_cpp(0, 1);
    assert!(!seer.can_detect_stealth_of_like_cpp(&target, true, false));
    assert!(seer.can_detect_stealth_of_like_cpp(&target, false, false));
}

#[test]
fn can_see_or_detect_unit_like_cpp_applies_cpp_visibility_gates_before_detection() {
    let mut seer = Unit::new(true);
    let mut target = Unit::new(true);
    let seer_guid = ObjectGuid::new(1, 11);
    let owner_guid = ObjectGuid::new(1, 12);
    seer.world_mut().object_mut().create(seer_guid);
    target
        .world_mut()
        .object_mut()
        .create(ObjectGuid::new(1, 13));

    target.set_never_visible_for_seer_like_cpp(true);
    target.set_always_visible_for_seer_like_cpp(true);
    assert!(!seer.can_see_or_detect_unit_like_cpp(&target, false, true, false));

    target.set_never_visible_for_seer_like_cpp(false);
    seer.set_seer_can_never_see_target_like_cpp(true);
    target.set_always_visible_for_seer_like_cpp(true);
    assert!(!seer.can_see_or_detect_unit_like_cpp(&target, false, true, false));

    seer.set_seer_can_never_see_target_like_cpp(false);
    target.set_always_visible_for_seer_like_cpp(false);
    target
        .subsystems_mut()
        .control
        .set_owner_guid(Some(seer_guid));
    target.set_invisibility_like_cpp(0, 100);
    assert!(seer.can_see_or_detect_unit_like_cpp(&target, false, true, false));

    target.subsystems_mut().control.set_owner_guid(None);
    assert!(!seer.can_see_or_detect_unit_like_cpp(&target, false, true, false));
    target.set_target_owner_group_visible_for_seer_like_cpp(true);
    assert!(seer.can_see_or_detect_unit_like_cpp(&target, false, true, false));

    target.set_target_owner_group_visible_for_seer_like_cpp(false);
    target.set_invisibility_like_cpp(0, 0);
    target.set_private_object_owner_like_cpp(owner_guid);
    assert!(!seer.can_see_or_detect_unit_like_cpp(&target, false, true, false));

    seer.set_seer_group_visible_for_private_owner_like_cpp(true);
    assert!(seer.can_see_or_detect_unit_like_cpp(&target, false, true, false));

    seer.set_seer_group_visible_for_private_owner_like_cpp(false);
    seer.set_seer_private_object_owner_like_cpp(owner_guid);
    assert!(seer.can_see_or_detect_unit_like_cpp(&target, false, true, false));

    seer.set_seer_private_object_owner_like_cpp(ObjectGuid::EMPTY);
    target.set_private_object_owner_like_cpp(seer_guid);
    assert!(seer.can_see_or_detect_unit_like_cpp(&target, false, true, false));

    target.set_private_object_owner_like_cpp(ObjectGuid::EMPTY);
    target
        .world_mut()
        .get_or_create_smooth_phasing_like_cpp()
        .set_viewer_dependent_info_like_cpp(seer_guid, crate::SmoothPhasingInfoLikeCpp::default());
    target.set_always_detectable_for_seer_like_cpp(true);
    assert!(!seer.can_see_or_detect_unit_like_cpp(&target, false, true, false));

    target
        .world_mut()
        .smooth_phasing_mut_like_cpp()
        .unwrap()
        .disable_replacement_for_seer_like_cpp(seer_guid);
    assert!(seer.can_see_or_detect_unit_like_cpp(&target, false, true, false));

    target.set_object_id_visibility_conditions_met_like_cpp(false);
    assert!(!seer.can_see_or_detect_unit_like_cpp(&target, false, true, false));

    target.set_private_object_owner_like_cpp(seer_guid);
    assert!(seer.can_see_or_detect_unit_like_cpp(&target, false, true, false));

    target.set_private_object_owner_like_cpp(ObjectGuid::EMPTY);
    target.set_object_id_visibility_conditions_met_like_cpp(true);
    target.set_always_detectable_for_seer_like_cpp(false);
    target.set_invisibility_like_cpp(0, 100);
    assert!(!seer.can_see_or_detect_unit_like_cpp(&target, false, true, false));

    target
        .subsystems_mut()
        .auras
        .register_applied_aura_type_like_cpp(
            AppliedAuraRef::new(53338, seer_guid, 0, 0x1),
            SPELL_AURA_MOD_STALKED_LIKE_CPP,
        );
    assert!(seer.can_see_or_detect_unit_like_cpp(&target, false, true, false));

    target
        .subsystems_mut()
        .auras
        .remove_auras_by_type_like_cpp(SPELL_AURA_MOD_STALKED_LIKE_CPP);
    seer.set_seer_can_always_see_target_guid_like_cpp(target.world().object().guid());
    assert!(seer.can_see_or_detect_unit_like_cpp(&target, false, true, false));
}

#[test]
fn can_see_or_detect_unit_like_cpp_applies_ghost_despawn_and_always_detectable_gates() {
    let mut seer = Unit::new(true);
    let mut target = Unit::new(true);

    target.set_server_side_ghost_visibility_like_cpp(0x2);
    assert!(!seer.can_see_or_detect_unit_like_cpp(&target, false, true, false));

    target.set_ghost_visible_to_seer_by_group_like_cpp(true);
    assert!(seer.can_see_or_detect_unit_like_cpp(&target, false, true, false));

    target.set_ghost_visible_to_seer_by_group_like_cpp(false);
    seer.set_server_side_ghost_visibility_detect_like_cpp(0x2);
    assert!(seer.can_see_or_detect_unit_like_cpp(&target, false, true, false));

    target.set_invisible_due_to_despawn_like_cpp(true);
    assert!(!seer.can_see_or_detect_unit_like_cpp(&target, false, true, false));

    target.set_invisible_due_to_despawn_like_cpp(false);
    target.set_invisibility_like_cpp(0, 100);
    assert!(!seer.can_see_or_detect_unit_like_cpp(&target, false, true, false));

    target.set_always_detectable_for_seer_like_cpp(true);
    assert!(seer.can_see_or_detect_unit_like_cpp(&target, false, true, false));
}

#[test]
fn valid_attack_target_represented_applies_cpp_relation_rules_when_known() {
    assert!(!Unit::is_valid_attack_target_represented_like_cpp(
        &UnitAttackContextLikeCpp {
            relation_represented: true,
            ..Default::default()
        }
    ));
    assert!(Unit::is_valid_attack_target_represented_like_cpp(
        &UnitAttackContextLikeCpp {
            relation_represented: true,
            attacker_is_hostile_to_victim: true,
            ..Default::default()
        }
    ));
    assert!(Unit::is_valid_attack_target_represented_like_cpp(
        &UnitAttackContextLikeCpp {
            relation_represented: true,
            victim_is_hostile_to_attacker: true,
            ..Default::default()
        }
    ));
    assert!(!Unit::is_valid_attack_target_represented_like_cpp(
        &UnitAttackContextLikeCpp {
            relation_represented: true,
            attacker_unit_flags: UnitFlags::PLAYER_CONTROLLED.bits(),
            victim_is_friendly_to_attacker: true,
            ..Default::default()
        }
    ));
    assert!(Unit::is_valid_attack_target_represented_like_cpp(
        &UnitAttackContextLikeCpp {
            relation_represented: true,
            attacker_unit_flags: UnitFlags::PLAYER_CONTROLLED.bits(),
            victim_unit_flags: UnitFlags::PLAYER_CONTROLLED.bits(),
            attacker_is_hostile_to_victim: true,
            ..Default::default()
        }
    ));
}

#[test]
fn valid_attack_target_represented_rejects_npc_attacking_mounted_player_pet_like_cpp() {
    assert!(!Unit::is_valid_attack_target_represented_like_cpp(
        &UnitAttackContextLikeCpp {
            victim_unit_flags: UnitFlags::PLAYER_CONTROLLED.bits(),
            victim_has_affecting_player: true,
            victim_is_pet: true,
            victim_affecting_player_is_mounted: true,
            ..Default::default()
        }
    ));
    assert!(!Unit::is_valid_attack_target_represented_like_cpp(
        &UnitAttackContextLikeCpp {
            attacker_has_affecting_player: true,
            victim_unit_flags: UnitFlags::PLAYER_CONTROLLED.bits(),
            victim_has_affecting_player: true,
            victim_is_pet: true,
            victim_affecting_player_is_mounted: true,
            ..Default::default()
        }
    ));
    assert!(Unit::is_valid_attack_target_represented_like_cpp(
        &UnitAttackContextLikeCpp {
            attacker_has_affecting_player: true,
            victim_unit_flags: UnitFlags::PLAYER_CONTROLLED.bits(),
            victim_has_affecting_player: true,
            victim_is_pet: true,
            victim_affecting_player_is_mounted: true,
            pvp_represented: true,
            victim_is_pvp: true,
            ..Default::default()
        }
    ));
}

#[test]
fn valid_attack_target_represented_applies_cpp_player_creature_reputation_rules() {
    assert!(!Unit::is_valid_attack_target_represented_like_cpp(
        &UnitAttackContextLikeCpp {
            attacker_has_affecting_player: true,
            player_creature_reputation_represented: true,
            ..Default::default()
        }
    ));
    assert!(Unit::is_valid_attack_target_represented_like_cpp(
        &UnitAttackContextLikeCpp {
            attacker_has_affecting_player: true,
            player_creature_reputation_represented: true,
            player_at_war_with_creature_faction: true,
            ..Default::default()
        }
    ));
    assert!(Unit::is_valid_attack_target_represented_like_cpp(
        &UnitAttackContextLikeCpp {
            attacker_has_affecting_player: true,
            player_creature_reputation_represented: true,
            creature_has_forced_reputation_rank: true,
            ..Default::default()
        }
    ));
    assert!(Unit::is_valid_attack_target_represented_like_cpp(
        &UnitAttackContextLikeCpp {
            attacker_has_affecting_player: true,
            player_creature_reputation_represented: true,
            creature_is_contested_guard: true,
            player_has_contested_pvp_flag: true,
            ..Default::default()
        }
    ));
}
