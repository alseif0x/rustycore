//! Unit values, visibility and health-revision state regression scenarios, part 2 of 3.
//!
//! Moved out of the unit.rs root under #636; every test is unchanged.

use super::*;

#[test]
fn valid_attack_target_represented_applies_cpp_duel_sanctuary_and_pvp_rules() {
    let player_pair = UnitAttackContextLikeCpp {
        attacker_unit_flags: UnitFlags::PLAYER_CONTROLLED.bits(),
        victim_unit_flags: UnitFlags::PLAYER_CONTROLLED.bits(),
        attacker_has_affecting_player: true,
        victim_has_affecting_player: true,
        ..Default::default()
    };

    assert!(Unit::is_valid_attack_target_represented_like_cpp(
        &UnitAttackContextLikeCpp {
            player_player_duel_in_progress: true,
            sanctuary_represented: true,
            attacker_in_sanctuary: true,
            ..player_pair.clone()
        }
    ));
    assert!(!Unit::is_valid_attack_target_represented_like_cpp(
        &UnitAttackContextLikeCpp {
            sanctuary_represented: true,
            victim_in_sanctuary: true,
            ..player_pair.clone()
        }
    ));
    assert!(!Unit::is_valid_attack_target_represented_like_cpp(
        &UnitAttackContextLikeCpp {
            ..player_pair.clone()
        }
    ));
    assert!(!Unit::is_valid_attack_target_represented_like_cpp(
        &UnitAttackContextLikeCpp {
            pvp_represented: true,
            ..player_pair.clone()
        }
    ));
    assert!(Unit::is_valid_attack_target_represented_like_cpp(
        &UnitAttackContextLikeCpp {
            pvp_represented: true,
            victim_is_pvp: true,
            ..player_pair.clone()
        }
    ));
    assert!(Unit::is_valid_attack_target_represented_like_cpp(
        &UnitAttackContextLikeCpp {
            pvp_represented: true,
            attacker_is_ffa_pvp: true,
            victim_is_ffa_pvp: true,
            ..player_pair.clone()
        }
    ));
    assert!(Unit::is_valid_attack_target_represented_like_cpp(
        &UnitAttackContextLikeCpp {
            pvp_represented: true,
            attacker_has_pvp_unk1_flag: true,
            ..player_pair
        }
    ));
}

#[test]
fn attack_like_cpp_rejects_represented_invalid_attack_target() {
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
                victim_unit_flags: UnitFlags::NON_ATTACKABLE.bits(),
                ..Default::default()
            },
        ),
        UnitAttackStartOutcome::InvalidAttackTarget
    );
    assert_eq!(unit.attacking(), None);
    assert_eq!(unit.data().target, ObjectGuid::EMPTY);
}

#[test]
fn attack_like_cpp_removes_unattackable_aura_type_before_melee_state() {
    let mut unit = Unit::new(true);
    let attacker = ObjectGuid::new(1, 9);
    let victim = ObjectGuid::new(1, 10);
    let aura = AppliedAuraRef::new(400, attacker, 0, 0x1);
    unit.world_mut().object_mut().create(attacker);
    unit.subsystems_mut()
        .auras
        .register_applied_aura_type_like_cpp(aura, SPELL_AURA_MOD_UNATTACKABLE_LIKE_CPP);

    assert!(
        unit.subsystems()
            .auras
            .has_aura_type_like_cpp(SPELL_AURA_MOD_UNATTACKABLE_LIKE_CPP)
    );

    assert_eq!(
        unit.attack_like_cpp(victim, true, true, true),
        UnitAttackStartOutcome::NewTarget { previous: None }
    );

    assert!(!unit.subsystems().auras.has_applied(aura));
    assert!(
        !unit
            .subsystems()
            .auras
            .has_aura_type_like_cpp(SPELL_AURA_MOD_UNATTACKABLE_LIKE_CPP)
    );
    assert_eq!(unit.subsystems().auras.removed_count(), 1);
    assert_eq!(unit.attacking(), Some(victim));
}

#[test]
fn attacker_state_update_melee_guards_match_cpp() {
    let mut unit = Unit::new(true);
    let attacker = ObjectGuid::new(1, 30);
    unit.world_mut().object_mut().create(attacker);
    assert!(unit.can_attacker_state_update_melee_like_cpp(false));

    unit.set_unit_flags_like_cpp(UnitFlags::PACIFIED);
    assert!(!unit.can_attacker_state_update_melee_like_cpp(false));
    unit.set_unit_flags_like_cpp(UnitFlags::empty());

    unit.add_unit_state(UnitState::STUNNED.bits());
    assert!(!unit.can_attacker_state_update_melee_like_cpp(false));
    assert!(unit.can_attacker_state_update_melee_like_cpp(true));
    unit.clear_unit_state(UnitState::STUNNED.bits());

    let aura = AppliedAuraRef::new(402, attacker, 0, 0x1);
    unit.subsystems_mut()
        .auras
        .register_applied_aura_type_like_cpp(
            aura,
            SPELL_AURA_DISABLE_ATTACKING_EXCEPT_ABILITIES_LIKE_CPP,
        );
    assert!(!unit.can_attacker_state_update_melee_like_cpp(false));
}

#[test]
fn attacker_state_update_removes_attacking_interrupt_auras_like_cpp() {
    let mut unit = Unit::new(true);
    let attacker = ObjectGuid::new(1, 31);
    unit.world_mut().object_mut().create(attacker);
    let removed_by_attacking = AppliedAuraRef::new(403, attacker, 0, 0x1);
    let kept = AppliedAuraRef::new(404, attacker, 0, 0x2);
    unit.subsystems_mut().auras.register_applied_aura(
        removed_by_attacking,
        None,
        SPELL_AURA_INTERRUPT_FLAG_ATTACKING_LIKE_CPP,
        0,
    );
    unit.subsystems_mut()
        .auras
        .register_applied_aura(kept, None, 0x20, 0);

    assert_eq!(unit.remove_attacking_interrupt_auras_like_cpp(), 1);
    assert!(!unit.subsystems().auras.has_applied(removed_by_attacking));
    assert!(unit.subsystems().auras.has_applied(kept));
}

#[test]
fn attack_timers_update_ready_and_reset_like_cpp() {
    let mut unit = Unit::new(true);
    unit.set_base_attack_time_like_cpp(WeaponAttackType::BaseAttack, 2_000);
    unit.set_attack_timer(WeaponAttackType::BaseAttack, 250);

    assert!(!unit.is_attack_ready_like_cpp(WeaponAttackType::BaseAttack));
    unit.update_attack_timers_like_cpp(100);
    assert_eq!(unit.attack_timer(WeaponAttackType::BaseAttack), 150);
    unit.update_attack_timers_like_cpp(200);
    assert_eq!(unit.attack_timer(WeaponAttackType::BaseAttack), 0);
    assert!(unit.is_attack_ready_like_cpp(WeaponAttackType::BaseAttack));
    unit.reset_attack_timer_like_cpp(WeaponAttackType::BaseAttack);
    assert_eq!(unit.attack_timer(WeaponAttackType::BaseAttack), 2_000);
}

#[test]
fn attack_like_cpp_delays_non_player_offhand_timer_like_cpp() {
    let mut unit = Unit::new(true);
    let attacker = ObjectGuid::new(1, 9);
    let victim = ObjectGuid::new(1, 10);
    unit.world_mut().object_mut().create(attacker);
    unit.set_can_dual_wield_like_cpp(true);
    unit.set_base_attack_time_like_cpp(WeaponAttackType::BaseAttack, 2_000);
    unit.set_attack_timer(WeaponAttackType::BaseAttack, 600);
    unit.set_attack_timer(WeaponAttackType::OffAttack, 100);

    assert_eq!(
        unit.attack_like_cpp(victim, true, true, true),
        UnitAttackStartOutcome::NewTarget { previous: None }
    );
    assert_eq!(unit.attack_timer(WeaponAttackType::OffAttack), 1_600);

    let next_victim = ObjectGuid::new(1, 11);
    unit.set_attack_timer(WeaponAttackType::BaseAttack, 500);
    unit.set_attack_timer(WeaponAttackType::OffAttack, 2_200);
    assert_eq!(
        unit.attack_like_cpp(next_victim, true, true, true),
        UnitAttackStartOutcome::NewTarget {
            previous: Some(victim)
        }
    );
    assert_eq!(unit.attack_timer(WeaponAttackType::OffAttack), 2_200);
}

#[test]
fn attack_like_cpp_does_not_delay_player_offhand_timer() {
    let mut player_unit = Unit::new(true);
    let attacker = ObjectGuid::new(1, 12);
    let victim = ObjectGuid::new(1, 13);
    player_unit.set_type(
        TypeId::Player,
        TypeMask::OBJECT | TypeMask::UNIT | TypeMask::PLAYER,
    );
    player_unit.world_mut().object_mut().create(attacker);
    player_unit.set_can_dual_wield_like_cpp(true);
    player_unit.set_base_attack_time_like_cpp(WeaponAttackType::BaseAttack, 2_000);
    player_unit.set_attack_timer(WeaponAttackType::BaseAttack, 600);
    player_unit.set_attack_timer(WeaponAttackType::OffAttack, 100);

    assert_eq!(
        player_unit.attack_like_cpp(victim, true, true, true),
        UnitAttackStartOutcome::NewTarget { previous: None }
    );
    assert_eq!(player_unit.attack_timer(WeaponAttackType::OffAttack), 100);
}

#[test]
fn attack_like_cpp_applies_creature_ai_side_effects_for_uncontrolled_unit() {
    let mut unit = Unit::new(true);
    let attacker = ObjectGuid::new(1, 14);
    let victim = ObjectGuid::new(1, 15);
    unit.world_mut().object_mut().create(attacker);
    unit.subsystems_mut()
        .combat
        .initialize_threat_list_capability(true);
    unit.set_emote_state_like_cpp(88);
    unit.set_stand_state_like_cpp(UnitStandStateType::Sit);

    assert_eq!(
        unit.attack_like_cpp(victim, true, true, true),
        UnitAttackStartOutcome::NewTarget { previous: None }
    );

    assert!(unit.subsystems().combat.is_threatened_by(victim));
    assert_eq!(unit.subsystems().ai.hostile_reaction_count, 1);
    assert_eq!(unit.subsystems().ai.call_assistance_count, 1);
    assert_eq!(unit.emote_state_like_cpp(), 0);
    assert_eq!(unit.stand_state_like_cpp(), UnitStandStateType::Stand);
}

#[test]
fn attack_like_cpp_skips_creature_ai_side_effects_when_controlled_by_player() {
    let mut unit = Unit::new(true);
    let attacker = ObjectGuid::new(1, 16);
    let victim = ObjectGuid::new(1, 17);
    let charmer = ObjectGuid::create_player(1, 18);
    unit.world_mut().object_mut().create(attacker);
    unit.subsystems_mut().control.apply_charmed_by(
        charmer,
        crate::CharmType::Charm,
        true,
        None,
        false,
    );

    assert_eq!(
        unit.attack_like_cpp(victim, true, true, true),
        UnitAttackStartOutcome::NewTarget { previous: None }
    );

    assert!(!unit.subsystems().combat.is_threatened_by(victim));
    assert_eq!(unit.subsystems().ai.hostile_reaction_count, 0);
    assert_eq!(unit.subsystems().ai.call_assistance_count, 0);
}

#[test]
fn player_attack_like_cpp_notifies_controlled_creature_ai_owner_attacked() {
    let mut player_unit = Unit::new(true);
    let player = ObjectGuid::create_player(1, 19);
    let victim = ObjectGuid::new(1, 20);
    let controlled_creature = ObjectGuid::new(1, 21);
    let controlled_without_ai = ObjectGuid::new(1, 22);
    let uncontrolled_creature = ObjectGuid::new(1, 23);
    player_unit.set_type(
        TypeId::Player,
        TypeMask::OBJECT | TypeMask::UNIT | TypeMask::PLAYER,
    );
    player_unit.world_mut().object_mut().create(player);
    assert!(
        player_unit
            .subsystems_mut()
            .control
            .add_controlled(controlled_creature)
    );
    assert!(
        player_unit
            .subsystems_mut()
            .control
            .add_controlled(controlled_without_ai)
    );

    assert_eq!(
        player_unit.attack_with_context_like_cpp(
            victim,
            true,
            true,
            true,
            UnitAttackContextLikeCpp {
                controlled_creatures_with_ai: vec![controlled_creature, uncontrolled_creature],
                ..Default::default()
            },
        ),
        UnitAttackStartOutcome::NewTarget { previous: None }
    );

    assert_eq!(
        player_unit
            .subsystems()
            .control
            .owner_attacked_notifications,
        vec![crate::ControlledOwnerAttackedNotification {
            controlled: controlled_creature,
            victim,
        }]
    );
}

#[test]
fn unit_subsystem_helpers_do_not_mark_update_fields() {
    let mut unit = Unit::new(true);
    let caster = ObjectGuid::new(1, 1);
    let target = ObjectGuid::new(1, 2);
    unit.subsystems_mut()
        .combat
        .initialize_threat_list_capability(true);

    unit.clear_unit_data_changes();
    let owned = OwnedAuraRef::new(17, caster, None);
    let applied = AppliedAuraRef::new(17, caster, 3, 0x7);
    unit.subsystems_mut().auras.add_owned(owned);
    unit.subsystems_mut().auras.add_applied(applied);
    unit.subsystems_mut()
        .auras
        .set_visible(3, AuraRef::new(17, caster));
    unit.subsystems_mut().spells.set_current_spell(
        CurrentSpellSlot::Channeled,
        CurrentSpellRef::new(42, Some(caster), None),
    );
    unit.subsystems_mut()
        .spells
        .history
        .set_cooldown(42, 100, 1_500);
    unit.subsystems_mut().combat.add_threat(target, 2.0);
    unit.subsystems_mut().motion.start_spline(9, 500);
    unit.subsystems_mut().control.set_charmer(caster, true);
    unit.subsystems_mut().vehicle.enter_vehicle(target, Some(0));
    unit.subsystems_mut().ai.push("TestAI");

    assert!(unit.subsystems().auras.has_owned(owned));
    assert!(unit.subsystems().auras.has_applied(applied));
    assert_eq!(
        unit.subsystems()
            .spells
            .current_spell(CurrentSpellSlot::Channeled)
            .map(|spell| spell.spell_id),
        Some(42)
    );
    assert_eq!(
        unit.subsystems()
            .spells
            .history
            .cooldown(42)
            .map(|cooldown| cooldown.cooldown_end_ms),
        Some(1_600)
    );
    assert!(unit.subsystems().combat.is_threatened_by(target));
    assert!(unit.subsystems().motion.spline.enabled);
    assert!(unit.subsystems().control.is_charmed());
    assert_eq!(unit.subsystems().vehicle.vehicle_guid, Some(target));
    assert_eq!(unit.subsystems().ai.active_ai.as_deref(), Some("TestAI"));
    assert!(!unit.unit_data_changes_mask().is_any_set());
}

#[test]
fn current_spell_slots_follow_cpp_ids_and_breakage_rules() {
    assert_eq!(CurrentSpellSlot::Melee as u8, 0);
    assert_eq!(CurrentSpellSlot::Generic as u8, 1);
    assert_eq!(CurrentSpellSlot::Channeled as u8, 2);
    assert_eq!(CurrentSpellSlot::Autorepeat as u8, 3);

    let mut unit = Unit::new(true);
    let caster = ObjectGuid::new(1, 1);
    let generic = CurrentSpellRef::new(100, Some(caster), None).with_cast_time_ms(1_500);
    let auto_shot = CurrentSpellRef::new(AUTO_SHOT_SPELL_ID, Some(caster), None);
    let other_auto = CurrentSpellRef::new(200, Some(caster), None);

    unit.set_current_cast_spell(CurrentSpellSlot::Autorepeat, auto_shot);
    unit.set_current_cast_spell(CurrentSpellSlot::Generic, generic);
    assert_eq!(unit.current_spell(CurrentSpellSlot::Generic), Some(generic));
    assert_eq!(
        unit.current_spell(CurrentSpellSlot::Autorepeat),
        Some(auto_shot)
    );
    assert!(unit.has_unit_state(UnitState::CASTING.bits()));

    unit.set_current_cast_spell(CurrentSpellSlot::Autorepeat, other_auto);
    assert_eq!(unit.current_spell(CurrentSpellSlot::Generic), None);
    assert_eq!(
        unit.current_spell(CurrentSpellSlot::Autorepeat),
        Some(other_auto)
    );
    assert!(!unit.has_unit_state(UnitState::CASTING.bits()));
}

#[test]
fn current_spell_generic_respects_channels_that_allow_actions() {
    let mut unit = Unit::new(true);
    let caster = ObjectGuid::new(1, 1);
    let channel_with_actions = CurrentSpellRef::new(300, Some(caster), None)
        .with_cast_time_ms(2_000)
        .with_allow_actions_during_channel(true);
    let generic = CurrentSpellRef::new(301, Some(caster), None).with_cast_time_ms(1_000);

    unit.set_current_cast_spell(CurrentSpellSlot::Channeled, channel_with_actions);
    unit.set_current_cast_spell(CurrentSpellSlot::Generic, generic);
    assert_eq!(
        unit.current_spell(CurrentSpellSlot::Channeled),
        Some(channel_with_actions)
    );
    assert_eq!(unit.current_spell(CurrentSpellSlot::Generic), Some(generic));

    let regular_channel = CurrentSpellRef::new(302, Some(caster), None).with_cast_time_ms(2_000);
    let next_generic = CurrentSpellRef::new(303, Some(caster), None).with_cast_time_ms(1_000);
    unit.set_current_cast_spell(CurrentSpellSlot::Channeled, regular_channel);
    unit.set_current_cast_spell(CurrentSpellSlot::Generic, next_generic);
    assert_eq!(unit.current_spell(CurrentSpellSlot::Channeled), None);
    assert_eq!(
        unit.current_spell(CurrentSpellSlot::Generic),
        Some(next_generic)
    );
}

#[test]
fn interrupt_spell_honors_cpp_delayed_instant_and_interruptible_guards() {
    let mut unit = Unit::new(true);
    let caster = ObjectGuid::new(1, 1);
    let instant = CurrentSpellRef::new(400, Some(caster), None);
    let delayed = CurrentSpellRef::new(401, Some(caster), None)
        .with_cast_time_ms(1_000)
        .with_state(SpellState::Delayed);
    let casting_instant =
        CurrentSpellRef::new(402, Some(caster), None).with_state(SpellState::Casting);
    let protected = CurrentSpellRef::new(403, Some(caster), None)
        .with_cast_time_ms(1_000)
        .with_interruptible(false);

    unit.set_current_cast_spell(CurrentSpellSlot::Generic, instant);
    assert_eq!(
        unit.interrupt_spell(CurrentSpellSlot::Generic, true, false),
        None
    );
    assert_eq!(unit.current_spell(CurrentSpellSlot::Generic), Some(instant));

    unit.set_current_cast_spell(CurrentSpellSlot::Generic, delayed);
    assert_eq!(
        unit.interrupt_spell(CurrentSpellSlot::Generic, false, true),
        None
    );
    assert_eq!(unit.current_spell(CurrentSpellSlot::Generic), Some(delayed));

    unit.set_current_cast_spell(CurrentSpellSlot::Generic, casting_instant);
    assert_eq!(
        unit.interrupt_spell(CurrentSpellSlot::Generic, true, false),
        Some(casting_instant)
    );

    unit.set_current_cast_spell(CurrentSpellSlot::Generic, protected);
    assert_eq!(
        unit.interrupt_spell(CurrentSpellSlot::Generic, true, true),
        None
    );
    assert_eq!(
        unit.current_spell(CurrentSpellSlot::Generic),
        Some(protected)
    );
    assert_eq!(
        unit.finish_spell(CurrentSpellSlot::Generic),
        Some(protected)
    );
    assert_eq!(unit.current_spell(CurrentSpellSlot::Generic), None);
}

#[test]
fn interrupt_non_melee_spells_filters_and_forces_channeled_interrupts() {
    let mut unit = Unit::new(true);
    let caster = ObjectGuid::new(1, 1);
    let melee = CurrentSpellRef::new(500, Some(caster), None);
    let generic = CurrentSpellRef::new(501, Some(caster), None).with_cast_time_ms(1_000);
    let auto = CurrentSpellRef::new(502, Some(caster), None);
    let delayed_channel = CurrentSpellRef::new(503, Some(caster), None)
        .with_state(SpellState::Delayed)
        .with_cast_time_ms(1_000);

    unit.subsystems_mut()
        .spells
        .set_current_spell(CurrentSpellSlot::Melee, melee);
    unit.subsystems_mut()
        .spells
        .set_current_spell(CurrentSpellSlot::Generic, generic);
    unit.subsystems_mut()
        .spells
        .set_current_spell(CurrentSpellSlot::Autorepeat, auto);
    unit.subsystems_mut()
        .spells
        .set_current_spell(CurrentSpellSlot::Channeled, delayed_channel);

    assert!(unit.is_non_melee_spell_cast_like_cpp(false, false, false, true));
    let removed = unit.interrupt_non_melee_spells(Some(503), false, false);
    assert_eq!(
        removed,
        vec![(CurrentSpellSlot::Channeled, delayed_channel)]
    );
    assert_eq!(unit.current_spell(CurrentSpellSlot::Melee), Some(melee));
    assert_eq!(unit.current_spell(CurrentSpellSlot::Generic), Some(generic));
    assert_eq!(unit.current_spell(CurrentSpellSlot::Autorepeat), Some(auto));

    let removed = unit.interrupt_non_melee_spells(None, true, true);
    assert_eq!(
        removed,
        vec![
            (CurrentSpellSlot::Generic, generic),
            (CurrentSpellSlot::Autorepeat, auto),
        ]
    );
    assert_eq!(unit.current_spell(CurrentSpellSlot::Melee), Some(melee));

    let mut instant_only = Unit::new(true);
    let instant = CurrentSpellRef::new(504, Some(caster), None);
    instant_only.set_current_cast_spell(CurrentSpellSlot::Generic, instant);
    assert!(!instant_only.is_non_melee_spell_cast_like_cpp(false, false, false, true));
    assert!(instant_only.is_non_melee_spell_cast_like_cpp(false, false, false, false));
}

#[test]
fn find_current_spell_by_spell_id_searches_all_cpp_slots() {
    let mut unit = Unit::new(true);
    let caster = ObjectGuid::new(1, 1);
    let melee = CurrentSpellRef::new(600, Some(caster), None);
    let channel = CurrentSpellRef::new(601, Some(caster), None).with_cast_time_ms(1_000);

    unit.set_current_cast_spell(CurrentSpellSlot::Melee, melee);
    unit.set_current_cast_spell(CurrentSpellSlot::Channeled, channel);

    assert_eq!(unit.find_current_spell_by_spell_id(600), Some(melee));
    assert_eq!(unit.find_current_spell_by_spell_id(601), Some(channel));
    assert_eq!(unit.find_current_spell_by_spell_id(602), None);
}

#[test]
fn health_and_max_health_follow_cpp_clamps() {
    let mut unit = Unit::new(true);

    unit.set_max_health(0);
    assert_eq!(unit.data().max_health, 1);
    assert!(
        unit.unit_data_changes_mask()
            .is_set(UNIT_DATA_MAX_HEALTH_BIT)
    );

    unit.clear_unit_data_changes();
    unit.set_max_health(100);
    unit.set_health(150);
    assert_eq!(unit.data().health, 100);
    assert!(unit.unit_data_changes_mask().is_set(UNIT_DATA_HEALTH_BIT));

    unit.clear_unit_data_changes();
    unit.set_max_health(40);
    assert_eq!(unit.data().max_health, 40);
    assert_eq!(unit.data().health, 40);
    assert!(
        unit.unit_data_changes_mask()
            .is_set(UNIT_DATA_MAX_HEALTH_BIT)
    );
    assert!(unit.unit_data_changes_mask().is_set(UNIT_DATA_HEALTH_BIT));

    unit.clear_unit_data_changes();
    unit.set_death_state(DeathState::Corpse);
    unit.set_health(30);
    assert_eq!(unit.data().health, 0);
}

#[test]
fn stand_state_helpers_match_cpp_sit_sleep_kneel_rules() {
    let mut unit = Unit::new(true);
    assert_eq!(unit.stand_state_like_cpp(), UnitStandStateType::Stand);
    assert!(unit.is_stand_state_like_cpp());

    unit.world_mut().object_mut().add_to_world();
    unit.clear_unit_data_changes();
    unit.set_stand_state_like_cpp(UnitStandStateType::SitChair);
    assert_eq!(unit.stand_state_like_cpp(), UnitStandStateType::SitChair);
    assert!(!unit.is_stand_state_like_cpp());
    assert!(
        unit.unit_data_changes_mask()
            .is_set(UNIT_DATA_STAND_STATE_BIT)
    );
    assert!(
        unit.unit_data_changes_mask()
            .is_set(UNIT_DATA_STAND_STATE_PARENT_BIT),
        "C++ UpdateField<uint8, 32, 56> marks its block parent"
    );
    assert!(
        !unit.unit_data_changes_mask().is_set(UNIT_DATA_PARENT_BIT),
        "StandState is in block parent 32, not block parent 0"
    );
    assert!(
        unit.world().object().is_object_updated(),
        "C++ UpdateField assignment queues an in-world Unit for object updates"
    );

    unit.clear_unit_data_changes();
    unit.world_mut().object_mut().clear_update_mask(false);

    unit.set_stand_state_like_cpp(UnitStandStateType::Sleep);
    assert!(!unit.is_stand_state_like_cpp());
    unit.set_stand_state_like_cpp(UnitStandStateType::Kneel);
    assert!(!unit.is_stand_state_like_cpp());
    unit.set_stand_state_like_cpp(UnitStandStateType::Dead);
    assert!(unit.is_stand_state_like_cpp());
    unit.set_stand_state_like_cpp(UnitStandStateType::Submerged);
    assert!(unit.is_stand_state_like_cpp());
}

#[test]
fn npc_flags_setters_mark_cpp_parent_and_element_bits() {
    let mut unit = Unit::new(true);
    unit.clear_unit_data_changes();

    unit.set_npc_flags_like_cpp(0x40);
    unit.set_npc_flags2_like_cpp(0x1);

    assert_eq!(unit.npc_flags_like_cpp(), [0x40, 0x1]);
    assert!(
        unit.unit_data_changes_mask()
            .is_set(UNIT_DATA_NPC_FLAGS_PARENT_BIT)
    );
    assert!(
        unit.unit_data_changes_mask()
            .is_set(UNIT_DATA_NPC_FLAGS_FIRST_BIT)
    );
    assert!(
        unit.unit_data_changes_mask()
            .is_set(UNIT_DATA_NPC_FLAGS_FIRST_BIT + 1)
    );
}

#[test]
fn power_setters_use_derived_power_index_and_cpp_clamps() {
    let mut unit = Unit::new(true);

    assert_eq!(unit.get_power(PowerType::Energy), 0);
    unit.set_power(PowerType::Energy, 10);
    assert!(
        !unit
            .unit_data_changes_mask()
            .is_set(UNIT_DATA_POWER_PARENT_BIT)
    );

    unit.set_power_index(PowerType::Energy, Some(3));
    unit.set_max_power(PowerType::Energy, 100);
    unit.set_power(PowerType::Energy, 150);

    assert_eq!(unit.get_power_index(PowerType::Energy), Some(3));
    assert_eq!(unit.get_power(PowerType::Energy), 100);
    assert_eq!(unit.get_max_power(PowerType::Energy), 100);
    assert!(
        unit.unit_data_changes_mask()
            .is_set(UNIT_DATA_POWER_PARENT_BIT)
    );
    assert!(
        unit.unit_data_changes_mask()
            .is_set(UNIT_DATA_POWER_FIRST_BIT + 3)
    );
    assert!(
        unit.unit_data_changes_mask()
            .is_set(UNIT_DATA_MAX_POWER_FIRST_BIT + 3)
    );
}

#[test]
fn virtual_item_updates_mark_cpp_parent_and_element_bits() {
    let mut unit = Unit::new(true);
    unit.clear_unit_data_changes();

    unit.set_virtual_item(
        1,
        Some(VisibleItemValues {
            item_id: 19019,
            item_appearance_mod_id: 2,
            item_visual: 3,
        }),
    );

    assert_eq!(unit.data().virtual_items[1].item_id, 19019);
    assert!(
        unit.unit_data_changes_mask()
            .is_set(UNIT_DATA_VIRTUAL_ITEMS_PARENT_BIT)
    );
    assert!(
        unit.unit_data_changes_mask()
            .is_set(UNIT_DATA_VIRTUAL_ITEMS_FIRST_BIT + 1)
    );
    assert!(
        !unit
            .unit_data_changes_mask()
            .is_set(UNIT_DATA_VIRTUAL_ITEMS_FIRST_BIT)
    );

    unit.clear_unit_data_changes();
    unit.set_virtual_item(1, None);
    assert_eq!(unit.data().virtual_items[1], VisibleItemValues::default());
    assert!(
        unit.unit_data_changes_mask()
            .is_set(UNIT_DATA_VIRTUAL_ITEMS_PARENT_BIT)
    );
    assert!(
        unit.unit_data_changes_mask()
            .is_set(UNIT_DATA_VIRTUAL_ITEMS_FIRST_BIT + 1)
    );
}

#[test]
fn virtual_item_mark_changed_forces_default_value_delta() {
    let mut unit = Unit::new(true);
    unit.clear_unit_data_changes();

    unit.mark_virtual_item_changed(2);

    assert_eq!(unit.data().virtual_items[2], VisibleItemValues::default());
    assert!(
        unit.unit_data_changes_mask()
            .is_set(UNIT_DATA_VIRTUAL_ITEMS_PARENT_BIT)
    );
    assert!(
        unit.unit_data_changes_mask()
            .is_set(UNIT_DATA_VIRTUAL_ITEMS_FIRST_BIT + 2)
    );
}

#[test]
fn unit_collision_height_setter_delegates_to_embedded_world_object() {
    let mut unit = Unit::new(true);

    unit.set_collision_height_like_cpp(2.03128);
    assert_eq!(unit.collision_height_like_cpp(), 2.03128);
    assert_eq!(unit.world().collision_height_like_cpp(), 2.03128);

    unit.set_collision_height_like_cpp(-10.0);
    assert_eq!(unit.collision_height_like_cpp(), 0.0);
    assert_eq!(unit.world().collision_height_like_cpp(), 0.0);
}

#[test]
fn unit_set_combat_reach_keeps_unit_data_and_world_object_coherent() {
    let mut unit = Unit::new(true);

    unit.set_combat_reach(1.75);
    assert_eq!(unit.data().combat_reach, 1.75);
    assert_eq!(unit.world().combat_reach(), 1.75);

    unit.set_combat_reach(-1.0);
    assert_eq!(unit.data().combat_reach, 0.0);
    assert_eq!(unit.world().combat_reach(), 0.0);
}

#[test]
fn display_level_faction_and_reach_mark_unitdata_bits() {
    let mut unit = Unit::new(true);

    unit.set_level(70);
    unit.set_race(1);
    unit.set_class(2);
    unit.set_player_class(2);
    unit.set_gender(Gender::Female);
    unit.set_target(ObjectGuid::new(7, 11));
    unit.set_faction(35);
    unit.set_bounding_radius(0.5);
    unit.set_combat_reach(1.5);
    unit.set_display_id(1234, true);
    unit.set_hover_height_like_cpp(1.25);

    assert_eq!(unit.data().level, 70);
    assert_eq!(unit.data().race, 1);
    assert_eq!(unit.data().class_id, 2);
    assert_eq!(unit.data().player_class_id, 2);
    assert_eq!(unit.data().sex, Gender::Female as u8);
    assert_eq!(unit.data().target, ObjectGuid::new(7, 11));
    assert_eq!(unit.data().faction_template, 35);
    assert_eq!(unit.data().bounding_radius, 0.5);
    assert_eq!(unit.data().combat_reach, 1.5);
    assert_eq!(unit.world().combat_reach(), 1.5);
    assert_eq!(unit.data().display_id, 1234);
    assert_eq!(unit.data().display_scale, DEFAULT_PLAYER_DISPLAY_SCALE);
    assert_eq!(unit.data().native_display_id, 1234);
    assert_eq!(unit.data().hover_height, 1.25);
    assert_eq!(
        unit.data().native_display_scale,
        DEFAULT_PLAYER_DISPLAY_SCALE
    );
    assert!(unit.unit_data_changes_mask().is_set(UNIT_DATA_PARENT_BIT));
    assert!(unit.unit_data_changes_mask().is_set(UNIT_DATA_LEVEL_BIT));
    assert!(unit.unit_data_changes_mask().is_set(UNIT_DATA_RACE_BIT));
    assert!(unit.unit_data_changes_mask().is_set(UNIT_DATA_CLASS_ID_BIT));
    assert!(
        unit.unit_data_changes_mask()
            .is_set(UNIT_DATA_PLAYER_CLASS_ID_BIT)
    );
    assert!(unit.unit_data_changes_mask().is_set(UNIT_DATA_SEX_BIT));
    assert!(unit.unit_data_changes_mask().is_set(UNIT_DATA_TARGET_BIT));
    assert!(
        unit.unit_data_changes_mask()
            .is_set(UNIT_DATA_FACTION_TEMPLATE_BIT)
    );
    assert!(
        unit.unit_data_changes_mask()
            .is_set(UNIT_DATA_BOUNDING_RADIUS_BIT)
    );
    assert!(
        unit.unit_data_changes_mask()
            .is_set(UNIT_DATA_COMBAT_REACH_BIT)
    );
    assert!(
        unit.unit_data_changes_mask()
            .is_set(UNIT_DATA_DISPLAY_ID_BIT)
    );
    assert!(
        unit.unit_data_changes_mask()
            .is_set(UNIT_DATA_DISPLAY_SCALE_BIT)
    );
    assert!(
        unit.unit_data_changes_mask()
            .is_set(UNIT_DATA_NATIVE_DISPLAY_ID_BIT)
    );
    assert!(
        unit.unit_data_changes_mask()
            .is_set(UNIT_DATA_NATIVE_DISPLAY_SCALE_BIT)
    );
    assert!(
        unit.unit_data_changes_mask()
            .is_set(UNIT_DATA_HOVER_HEIGHT_BIT)
    );
}

#[test]
fn critter_guid_uses_canonical_unitdata_and_change_bit_like_cpp() {
    let mut unit = Unit::new(true);
    let critter = ObjectGuid::new(7, 12);
    let battle_pet = ObjectGuid::new(7, 13);
    unit.clear_unit_data_changes();

    unit.set_critter_guid_like_cpp(Some(critter));
    unit.set_battle_pet_companion_guid_like_cpp(Some(battle_pet));
    unit.set_battle_pet_companion_name_timestamp_like_cpp(1234);

    assert_eq!(unit.critter_guid_like_cpp(), Some(critter));
    assert_eq!(unit.battle_pet_companion_guid_like_cpp(), Some(battle_pet));
    assert_eq!(unit.battle_pet_companion_name_timestamp_like_cpp(), 1234);
    assert!(unit.unit_data_changes_mask().is_set(UNIT_DATA_PARENT_BIT));
    assert!(unit.unit_data_changes_mask().is_set(UNIT_DATA_CRITTER_BIT));
    assert!(
        unit.unit_data_changes_mask()
            .is_set(UNIT_DATA_BATTLE_PET_COMPANION_GUID_BIT)
    );
    assert!(
        unit.unit_data_changes_mask()
            .is_set(UNIT_DATA_BATTLE_PET_COMPANION_NAME_TIMESTAMP_BIT)
    );

    unit.set_critter_guid_like_cpp(None);
    unit.set_battle_pet_companion_guid_like_cpp(None);
    assert_eq!(unit.critter_guid_like_cpp(), None);
    assert_eq!(unit.battle_pet_companion_guid_like_cpp(), None);
}
