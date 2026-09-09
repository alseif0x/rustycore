//! Unit-subsystem regressions, part 1 of 3.
//!
//! Moved out of the tests.rs root under #648; every test is unchanged.

use super::*;

#[test]
fn aura_authorities_default_fail_closed() {
    let auras = AuraSubsystem::default();

    assert!(!auras.has_complete_spell_hit_inert_aura_authority_like_cpp());
    assert!(!auras.has_complete_spell_cast_log_aura_authority_like_cpp());
}

#[test]
fn spell_hit_and_cast_log_aura_authorities_are_independent() {
    let mut hit_authority = AuraSubsystem::default();
    hit_authority.set_spell_hit_aura_authority_inert_like_cpp(true);

    assert!(hit_authority.has_complete_spell_hit_inert_aura_authority_like_cpp());
    assert!(!hit_authority.has_complete_spell_cast_log_aura_authority_like_cpp());

    let mut cast_log_authority = AuraSubsystem::default();
    cast_log_authority.set_spell_cast_log_aura_authority_inert_like_cpp(true);

    assert!(!cast_log_authority.has_complete_spell_hit_inert_aura_authority_like_cpp());
    assert!(cast_log_authority.has_complete_spell_cast_log_aura_authority_like_cpp());
}

#[test]
fn spell_hit_inert_aura_authority_requires_explicit_source_proof() {
    let mut auras = AuraSubsystem::default();

    auras.set_spell_hit_aura_authority_inert_like_cpp(true);
    assert!(auras.has_complete_spell_hit_inert_aura_authority_like_cpp());

    auras.invalidate_spell_hit_aura_authority_like_cpp();
    assert!(!auras.has_complete_spell_hit_inert_aura_authority_like_cpp());
}

#[test]
fn complete_spell_hit_aura_authority_rejects_any_represented_aura() {
    let mut auras = AuraSubsystem::default();
    auras.set_spell_hit_aura_authority_inert_like_cpp(true);

    auras.add_owned(OwnedAuraRef::new(100, guid(1), None));

    assert!(!auras.has_complete_spell_hit_inert_aura_authority_like_cpp());
}

#[test]
fn aura_mutation_permanently_revokes_source_proof_until_reaccredited() {
    let mut auras = AuraSubsystem::default();
    let aura = OwnedAuraRef::new(100, guid(1), None);
    auras.set_spell_hit_aura_authority_inert_like_cpp(true);
    auras.set_spell_cast_log_aura_authority_inert_like_cpp(true);

    assert!(auras.has_complete_spell_hit_inert_aura_authority_like_cpp());
    assert!(auras.has_complete_spell_cast_log_aura_authority_like_cpp());

    auras.add_owned(aura);
    assert!(!auras.has_complete_spell_hit_inert_aura_authority_like_cpp());
    assert!(!auras.has_complete_spell_cast_log_aura_authority_like_cpp());

    assert!(auras.remove_owned(aura));

    assert!(auras.owned_auras.is_empty());
    assert!(!auras.has_complete_spell_hit_inert_aura_authority_like_cpp());
    assert!(!auras.has_complete_spell_cast_log_aura_authority_like_cpp());
}

#[test]
fn aura_spell_history_and_current_spell_helpers_roundtrip() {
    let mut subsystems = UnitSubsystems::default();
    let caster = guid(1);
    let owned = OwnedAuraRef::new(100, caster, None);
    let applied = AppliedAuraRef::new(100, caster, 2, 0x5);

    subsystems.auras.add_owned(owned);
    subsystems.auras.add_applied(applied);
    subsystems.auras.set_visible(2, AuraRef::new(100, caster));
    subsystems.auras.mark_removed(AuraRef::new(100, caster));
    subsystems.auras.interrupt_flags = 0x10;
    subsystems.auras.interrupt_flags2 = 0x20;

    assert!(subsystems.auras.has_owned(owned));
    assert!(subsystems.auras.has_applied(applied));
    assert_eq!(
        subsystems.auras.visible_auras.get(&2).copied(),
        Some(AuraRef::new(100, caster))
    );
    assert_eq!(subsystems.auras.removed_count(), 1);
    assert!(subsystems.auras.remove_owned(owned));
    assert!(subsystems.auras.remove_applied(applied));
    assert_eq!(
        subsystems.auras.clear_visible(2),
        Some(AuraRef::new(100, caster))
    );
    subsystems.auras.clear_removed();
    assert_eq!(subsystems.auras.removed_count(), 0);

    let spell = CurrentSpellRef::new(200, Some(caster), Some(guid(3)));
    subsystems
        .spells
        .set_current_spell(CurrentSpellSlot::Generic, spell);
    assert_eq!(
        subsystems.spells.current_spell(CurrentSpellSlot::Generic),
        Some(spell)
    );
    assert_eq!(
        subsystems
            .spells
            .clear_current_spell(CurrentSpellSlot::Generic),
        Some(spell)
    );

    subsystems.spells.history.set_cooldown(200, 1_000, 30_000);
    subsystems.spells.history.set_charges(200, 2, 1_000, 10_000);
    assert_eq!(
        subsystems.spells.history.cooldown(200),
        Some(SpellCooldown {
            spell_id: 200,
            item_id: 0,
            cooldown_end_ms: 31_000,
            category_id: 0,
            category_end_ms: 1_000,
            on_hold: false,
        })
    );
    assert_eq!(
        subsystems.spells.history.charges(200).map(VecDeque::len),
        Some(2)
    );
    assert!(subsystems.spells.history.clear_cooldown(200));
    subsystems.spells.history.reset();
    assert!(subsystems.spells.history.cooldowns.is_empty());
    assert!(subsystems.spells.history.charges.is_empty());
}

#[test]
fn aura_application_interrupt_state_and_diminishing_match_cpp_shape() {
    let mut auras = AuraSubsystem::default();
    let caster = guid(2);
    let other = guid(3);
    let defensive = AppliedAuraRef::new(200, caster, 0, 0x1);
    let poison = AppliedAuraRef::new(201, caster, 1, 0x2);
    let other_poison = AppliedAuraRef::new(202, other, 2, 0x4);

    auras.register_applied_aura(defensive, Some(AURA_STATE_DEFENSIVE), 0x8, 0);
    assert!(auras.has_applied(defensive));
    assert!(auras.has_interrupt_flag(0x8));
    assert!(auras.has_aura_state(AURA_STATE_DEFENSIVE));
    assert_eq!(
        auras.build_aura_state_update_for_target(other),
        1 << (AURA_STATE_DEFENSIVE - 1)
    );

    auras.register_applied_aura(poison, Some(AURA_STATE_ROGUE_POISONED), 0, 0x20);
    auras.register_applied_aura(other_poison, Some(AURA_STATE_ROGUE_POISONED), 0, 0);
    assert!(auras.has_interrupt_flag2(0x20));
    assert_eq!(
        auras.build_aura_state_update_for_target(caster),
        (1 << (AURA_STATE_DEFENSIVE - 1)) | (1 << (AURA_STATE_ROGUE_POISONED - 1))
    );

    assert_eq!(auras.remove_interruptible_auras(0, 0x20), vec![poison]);
    assert!(!auras.has_applied(poison));
    assert!(auras.has_applied(other_poison));
    assert!(!auras.has_interrupt_flag2(0x20));
    assert_eq!(auras.removed_auras_count, 1);

    assert!(auras.can_proc());
    auras.set_cant_proc(true);
    assert!(!auras.can_proc());
    auras.set_cant_proc(false);
    assert!(auras.can_proc());

    assert_eq!(
        auras.get_diminishing(DIMINISHING_STUN, 1_000),
        DiminishingLevel::Level1
    );
    auras.incr_diminishing(DIMINISHING_STUN, DiminishingLevel::Immune, 1_000);
    assert_eq!(
        auras.get_diminishing(DIMINISHING_STUN, 1_000),
        DiminishingLevel::Level2
    );
    auras.apply_diminishing_aura(DIMINISHING_STUN, true, 2_000);
    auras.apply_diminishing_aura(DIMINISHING_STUN, false, 3_000);
    assert_eq!(auras.diminishing[DIMINISHING_STUN].hit_time_ms, 3_000);
    assert_eq!(
        auras.get_diminishing(DIMINISHING_STUN, 21_001),
        DiminishingLevel::Level1
    );
    auras.clear_diminishings();
    assert_eq!(
        auras.diminishing[DIMINISHING_STUN],
        DiminishingReturnState::default()
    );
}

#[test]
fn aura_type_removal_matches_cpp_remove_auras_by_type_shape() {
    let mut auras = AuraSubsystem::default();
    let caster = guid(1);
    let unattackable = AppliedAuraRef::new(300, caster, 0, 0x1);
    let other_same_type = AppliedAuraRef::new(301, caster, 1, 0x2);
    let different = AppliedAuraRef::new(302, caster, 2, 0x4);

    auras.register_applied_aura_type_like_cpp(unattackable, 93);
    auras.register_applied_aura_type_like_cpp(other_same_type, 93);
    auras.register_applied_aura_type_like_cpp(different, 8);

    assert!(auras.has_aura_type_like_cpp(93));
    assert_eq!(
        auras.remove_auras_by_type_like_cpp(93),
        vec![unattackable, other_same_type]
    );

    assert!(!auras.has_applied(unattackable));
    assert!(!auras.has_applied(other_same_type));
    assert!(auras.has_applied(different));
    assert!(!auras.has_aura_type_like_cpp(93));
    assert!(auras.has_aura_type_like_cpp(8));
    assert_eq!(auras.removed_count(), 2);
}

#[test]
fn total_aura_modifier_sums_and_removes_amounts_like_cpp() {
    let mut auras = AuraSubsystem::default();
    let caster = guid(1);
    let first = AppliedAuraRef::new(400, caster, 0, 0x1);
    let second = AppliedAuraRef::new(401, caster, 1, 0x2);
    let other = AppliedAuraRef::new(402, caster, 2, 0x4);

    auras.register_applied_aura_modifier_like_cpp(first, 91, 4);
    auras.register_applied_aura_modifier_like_cpp(second, 91, -2);
    auras.register_applied_aura_modifier_like_cpp(other, 152, 7);

    assert_eq!(auras.total_aura_modifier_like_cpp(91), 2);
    assert_eq!(auras.total_aura_modifier_like_cpp(152), 7);

    assert!(auras.remove_applied(first));

    assert_eq!(auras.total_aura_modifier_like_cpp(91), -2);
    assert_eq!(auras.total_aura_modifier_like_cpp(152), 7);
}

#[test]
fn aura_immunity_masks_and_breakable_stun_require_cpp_metadata() {
    let mut auras = AuraSubsystem::default();
    let caster = guid(1);
    let first_immunity = AppliedAuraRef::new(410, caster, 0, 0x1);
    let second_immunity = AppliedAuraRef::new(411, caster, 1, 0x1);
    let durable_stun = AppliedAuraRef::new(412, caster, 2, 0x1);
    let breakable_stun = AppliedAuraRef::new(413, caster, 3, 0x1);
    let fire_threat = AppliedAuraRef::new(414, caster, 4, 0x1);

    auras.register_applied_aura_effect_like_cpp(first_immunity, 39, 99, 0x1);
    auras.register_applied_aura_effect_like_cpp(second_immunity, 39, 77, 0x4);
    auras.register_applied_aura_type_like_cpp(durable_stun, 12);
    auras.register_applied_aura(durable_stun, None, 0, 0);
    assert_eq!(auras.aura_school_mask_like_cpp(39), 0x5);
    assert!(
        !auras.has_breakable_by_damage_aura_type_like_cpp(12),
        "C++ does not suppress for an unbreakable stun"
    );

    auras.register_applied_aura_type_like_cpp(breakable_stun, 12);
    auras.register_applied_aura(
        breakable_stun,
        None,
        wow_constants::SpellAuraInterruptFlags::DAMAGE.bits(),
        0,
    );
    assert!(auras.has_breakable_by_damage_aura_type_like_cpp(12));
    auras.register_applied_aura_effect_like_cpp(fire_threat, 10, -30, 0x4);
    assert_eq!(
        auras.total_aura_multiplier_by_misc_mask_like_cpp(10, 0x4),
        0.7
    );
    assert_eq!(
        auras.total_aura_multiplier_by_misc_mask_like_cpp(10, 0x2),
        1.0
    );
}

#[test]
fn remove_auras_due_to_spell_matches_cpp_filters() {
    let mut auras = AuraSubsystem::default();
    let caster = guid(1);
    let other = guid(2);
    let exact = AppliedAuraRef::new(400, caster, 0, 0x3);
    let missing_effect = AppliedAuraRef::new(400, caster, 1, 0x1);
    let other_caster = AppliedAuraRef::new(400, other, 2, 0x3);
    let different_spell = AppliedAuraRef::new(401, caster, 3, 0x3);
    let exact_owned = OwnedAuraRef::new(400, caster, None);
    let other_owned = OwnedAuraRef::new(400, other, None);

    for aura in [exact, missing_effect, other_caster, different_spell] {
        auras.add_applied(aura);
    }
    auras.add_owned(exact_owned);
    auras.add_owned(other_owned);

    assert_eq!(
        auras.remove_auras_due_to_spell_like_cpp(400, caster, 0x3),
        vec![exact]
    );
    assert!(!auras.has_applied(exact));
    assert!(!auras.has_owned(exact_owned));
    assert!(auras.has_owned(other_owned));
    assert!(auras.has_applied(missing_effect));
    assert!(auras.has_applied(other_caster));
    assert!(auras.has_applied(different_spell));
    assert_eq!(auras.removed_auras, vec![exact.aura_ref()]);

    assert_eq!(
        auras.remove_auras_due_to_spell_like_cpp(400, ObjectGuid::EMPTY, 0),
        vec![missing_effect, other_caster]
    );
    assert_eq!(auras.removed_count(), 3);
    assert!(!auras.has_owned(other_owned));
    assert!(auras.has_applied(different_spell));
}

#[test]
fn spell_history_cooldowns_track_spell_category_hold_and_update_like_cpp() {
    let mut history = SpellHistory::default();

    assert!(history.start_cooldown(1_000, 100, 7, 3_000, 9, 1_500, false));
    assert!(history.has_cooldown(100, 9, 2_000));
    assert_eq!(history.remaining_cooldown_ms(100, 9, 2_000), 2_000);
    assert_eq!(history.remaining_category_cooldown_ms(9, 2_000), 500);

    assert!(!history.add_cooldown(100, 7, 2_000, 9, 1_500, false));
    assert_eq!(
        history
            .cooldown(100)
            .map(|cooldown| cooldown.cooldown_end_ms),
        Some(4_000)
    );

    assert!(history.start_cooldown(2_000, 101, 0, 1, 11, 1, true));
    let held = history.cooldown(101).expect("on-hold cooldown");
    assert!(held.on_hold);
    assert_eq!(held.cooldown_end_ms, 2_000 + INFINITY_COOLDOWN_DELAY_MS);
    assert_eq!(held.category_end_ms, 2_000 + INFINITY_COOLDOWN_DELAY_MS);

    assert!(history.modify_cooldown(100, -2_000, false, 2_500));
    assert_eq!(history.cooldown(100), None);
    assert!(!history.has_cooldown(100, 9, 2_500));

    history.update(2_501);
    assert!(!history.has_cooldown(100, 9, 2_501));
    assert!(history.has_cooldown(101, 11, 2_501));
}

#[test]
fn spell_history_charges_school_locks_gcd_and_duel_snapshot_match_cpp_shape() {
    let mut history = SpellHistory::default();

    assert!(history.consume_charge(44, 1_000, 5_000, 2));
    assert!(history.consume_charge(44, 1_500, 5_000, 2));
    assert!(!history.has_charge(44, 2));
    assert_eq!(history.consumed_charges(44), 2);
    assert_eq!(
        history
            .charges(44)
            .and_then(|charges| charges.front())
            .map(|charge| charge.recharge_end_ms),
        Some(6_000)
    );

    assert!(history.modify_charge_recovery_time(44, -1_000, 1_500));
    assert_eq!(
        history
            .charges(44)
            .and_then(|charges| charges.front())
            .map(|charge| charge.recharge_end_ms),
        Some(5_000)
    );
    assert!(history.restore_charge(44));
    assert_eq!(history.consumed_charges(44), 1);
    history.update(5_000);
    assert_eq!(history.consumed_charges(44), 0);

    history.lock_spell_school(0b0010_1000, 10_000, 3_000);
    assert!(history.is_school_locked(0b0000_1000, 12_000));
    assert!(history.is_school_locked(0b0010_0000, 12_000));
    assert!(!history.is_school_locked(0b0000_1000, 13_001));

    history.add_global_cooldown(12, 20_000, 1_500);
    assert!(history.has_global_cooldown(12, 21_000));
    assert_eq!(history.remaining_global_cooldown_ms(12, 21_000), 500);
    history.cancel_global_cooldown(12);
    assert!(!history.has_global_cooldown(12, 21_000));

    history.start_cooldown(30_000, 777, 0, 10_000, 55, 5_000, false);
    history.save_cooldown_state_before_duel();
    history.start_cooldown(31_000, 888, 0, 10_000, 66, 5_000, false);
    history.restore_cooldown_state_after_duel();
    assert!(history.has_cooldown(777, 55, 31_000));
    assert!(!history.has_cooldown(888, 66, 31_000));
    assert_eq!(history.category_cooldowns.get(&55), Some(&777));
}

#[test]
fn spell_history_add_charge_state_preserves_loaded_order_like_cpp() {
    let mut history = SpellHistory::default();

    assert!(history.add_charge_state_like_cpp(88, 1_000, 4_000));
    assert!(history.add_charge_state_like_cpp(88, 2_000, 5_000));
    assert!(!history.add_charge_state_like_cpp(0, 3_000, 6_000));

    let charges = history.charges(88).expect("loaded charge category");
    assert_eq!(charges.len(), 2);
    assert_eq!(charges[0].recharge_start_ms, 1_000);
    assert_eq!(charges[0].recharge_end_ms, 4_000);
    assert_eq!(charges[1].recharge_start_ms, 2_000);
    assert_eq!(charges[1].recharge_end_ms, 5_000);
    assert!(history.charges(0).is_none());
}

#[test]
fn spell_history_pet_save_plan_matches_cpp_delete_insert_phases() {
    let mut history = SpellHistory::default();
    history.add_cooldown(100, 7, 12_345, 9, 67_890, false);
    history.cooldowns.get_mut(&100).unwrap().spell_id = 999;
    history.add_cooldown(101, 0, 22_000, 0, 0, true);
    assert!(history.add_charge_state_like_cpp(44, 10_999, 20_001));
    assert!(history.add_charge_state_like_cpp(44, 20_001, 30_999));
    assert!(history.add_charge_state_like_cpp(55, 40_000, 50_000));

    let operations = history.save_pet_spell_history_plan_like_cpp(77);
    assert_eq!(
        operations.first(),
        Some(&SpellHistoryPetSaveOperationLikeCpp::DeleteCooldowns { pet_number: 77 })
    );

    let delete_charges_index = operations
        .iter()
        .position(|operation| {
            matches!(
                operation,
                SpellHistoryPetSaveOperationLikeCpp::DeleteCharges { pet_number: 77 }
            )
        })
        .expect("C++ deletes charge rows after cooldown inserts");
    assert!(delete_charges_index > 0);
    assert!(
        operations[..delete_charges_index]
            .iter()
            .skip(1)
            .all(|operation| matches!(
                operation,
                SpellHistoryPetSaveOperationLikeCpp::InsertCooldown { .. }
            ))
    );
    assert!(
        operations[delete_charges_index + 1..]
            .iter()
            .all(|operation| matches!(
                operation,
                SpellHistoryPetSaveOperationLikeCpp::InsertCharge { .. }
            ))
    );

    assert!(
        operations.contains(&SpellHistoryPetSaveOperationLikeCpp::InsertCooldown {
            pet_number: 77,
            spell_id: 100,
            cooldown_end_time_secs: 12,
            category_id: 9,
            category_end_time_secs: 67,
        })
    );
    assert!(!operations.iter().any(|operation| matches!(
        operation,
        SpellHistoryPetSaveOperationLikeCpp::InsertCooldown { spell_id: 101, .. }
    )));

    let charge_44: Vec<_> = operations
        .iter()
        .filter_map(|operation| match operation {
            SpellHistoryPetSaveOperationLikeCpp::InsertCharge {
                category_id: 44,
                recharge_start_time_secs,
                recharge_end_time_secs,
                ..
            } => Some((*recharge_start_time_secs, *recharge_end_time_secs)),
            _ => None,
        })
        .collect();
    assert_eq!(charge_44, vec![(10, 20), (20, 30)]);
    assert!(
        operations.contains(&SpellHistoryPetSaveOperationLikeCpp::InsertCharge {
            pet_number: 77,
            category_id: 55,
            recharge_start_time_secs: 40,
            recharge_end_time_secs: 50,
        })
    );
}

#[test]
fn current_spell_slots_match_trinity_values_and_roundtrip() {
    assert_eq!(CurrentSpellSlot::Melee as u8, 0);
    assert_eq!(CurrentSpellSlot::Generic as u8, 1);
    assert_eq!(CurrentSpellSlot::Channeled as u8, 2);
    assert_eq!(CurrentSpellSlot::Autorepeat as u8, 3);
    assert_eq!(CURRENT_FIRST_NON_MELEE_SPELL, 1);
    assert_eq!(CURRENT_MAX_SPELL, 4);

    let caster = guid(4);
    let mut spells = SpellSubsystem::default();
    let slots = [
        CurrentSpellSlot::Melee,
        CurrentSpellSlot::Generic,
        CurrentSpellSlot::Channeled,
        CurrentSpellSlot::Autorepeat,
    ];

    for (index, slot) in slots.into_iter().enumerate() {
        let spell = CurrentSpellRef::new(300 + index as u32, Some(caster), None);
        spells.set_current_spell(slot, spell);
        assert_eq!(spells.current_spell(slot), Some(spell));
        assert_eq!(spells.clear_current_spell(slot), Some(spell));
        assert_eq!(spells.current_spell(slot), None);
    }
}

#[test]
fn threat_combat_helpers_roundtrip() {
    let mut combat = CombatSubsystem::default();
    combat.initialize_threat_list_capability(true);
    let attacker = guid(10);

    assert!(!combat.combat_disallowed);
    assert_eq!(combat.threat_update_timer_ms, THREAT_UPDATE_INTERVAL_MS);
    assert_eq!(combat.add_threat(attacker, 5.0), 5.0);
    assert_eq!(combat.add_threat(attacker, 2.5), 7.5);
    assert!(combat.is_threatened_by(attacker));
    assert_eq!(combat.threat_value(attacker), Some(7.5));
    combat.set_threat(attacker, 1.0);
    assert_eq!(combat.remove_threat(attacker), Some(1.0));

    assert!(combat.add_attacker(attacker));
    combat.set_attacking(Some(attacker));
    combat.combat_disallowed = true;
    assert!(combat.attackers.contains(&attacker));
    assert_eq!(combat.attacking_guid, Some(attacker));
    assert!(combat.combat_disallowed);
    assert!(combat.remove_attacker(attacker));
    combat.clear_attackers();
    assert!(combat.attackers.is_empty());
    assert_eq!(combat.attacking_guid, None);
}

#[test]
fn add_threat_rejects_incapable_owners_like_cpp() {
    let mut combat = CombatSubsystem::default();
    let target = guid(11);

    assert_eq!(combat.add_threat(target, 5.0), 0.0);
    assert_eq!(combat.threat_value(target), None);
    assert!(combat.is_in_combat_with(target));
}

#[test]
fn threat_refs_sort_and_scale_like_cpp_threat_manager_shape() {
    let mut combat = CombatSubsystem::default();
    let low = guid(20);
    let high = guid(21);
    let taunter = guid(22);
    let offline = guid(23);

    combat.initialize_threat_list_capability(true);
    assert!(combat.owner_can_have_threat_list);
    assert_eq!(combat.add_threat(low, 100.0), 100.0);
    assert_eq!(combat.add_threat(high, 120.0), 120.0);
    assert_eq!(combat.add_threat(taunter, 1.0), 1.0);
    assert_eq!(combat.add_threat(offline, 999.0), 999.0);
    assert!(combat.set_threat_taunt_state(taunter, ThreatTauntState::Taunt(1)));
    assert!(combat.set_threat_online_state(offline, ThreatOnlineState::Offline));

    assert_eq!(
        combat.sorted_threat_guids(),
        vec![taunter, high, low, offline]
    );
    assert_eq!(combat.threat_list_size(), 4);
    assert!(!combat.is_threat_list_empty(false));
    assert!(combat.is_threatened_by_with_offline(offline, true));
    assert!(!combat.is_threatened_by(offline));

    assert_eq!(combat.modify_threat_by_percent(high, -50), Some(60.0));
    assert_eq!(combat.scale_threat(low, 2.0), Some(200.0));
    assert_eq!(combat.threat_value(low), Some(200.0));
    assert_eq!(
        combat.threat_ref(low).map(|state| state.threat()),
        Some(200.0)
    );

    combat.reset_all_threat();
    assert_eq!(combat.threat_value(low), Some(0.0));
    assert!(combat.need_client_update);
}

#[test]
fn match_unit_threat_to_highest_threat_matches_cpp_taunt_skip_shape() {
    let mut combat = CombatSubsystem::default();
    combat.initialize_threat_list_capability(true);
    let caster = guid(24);
    let taunter = guid(25);
    let high = guid(26);

    combat.add_threat(taunter, 100.0);
    assert!(combat.set_threat_taunt_state(taunter, ThreatTauntState::Taunt(1)));
    combat.add_threat(high, 150.0);

    assert_eq!(
        combat.match_unit_threat_to_highest_threat_like_cpp(caster),
        Some(150.0)
    );
    assert_eq!(combat.threat_value(caster), Some(150.0));
}

#[test]
fn match_unit_threat_to_highest_threat_uses_available_highest_like_cpp() {
    let mut combat = CombatSubsystem::default();
    combat.initialize_threat_list_capability(true);
    let caster = guid(27);
    let offline = guid(28);
    let high = guid(29);

    combat.add_threat(offline, 999.0);
    assert!(combat.set_threat_online_state(offline, ThreatOnlineState::Offline));
    combat.add_threat(high, 80.0);

    assert_eq!(
        combat.match_unit_threat_to_highest_threat_like_cpp(caster),
        Some(80.0)
    );
    assert_eq!(combat.threat_value(caster), Some(80.0));
}

#[test]
fn offline_threat_reference_accumulates_without_becoming_available_like_cpp() {
    let mut combat = CombatSubsystem::default();
    combat.initialize_threat_list_capability(true);
    let target = guid(298);

    assert_eq!(combat.add_threat(target, 25.0), 25.0);
    assert!(combat.set_threat_online_state(target, ThreatOnlineState::Offline));
    assert_eq!(combat.add_threat(target, 15.0), 40.0);
    assert_eq!(combat.threat_value(target), Some(40.0));
    assert!(
        combat
            .threat_ref(target)
            .is_some_and(|state| state.is_offline())
    );
    assert!(!combat.is_threatened_by(target));
    assert!(combat.is_threatened_by_with_offline(target, true));
}

#[test]
fn threat_reselect_victim_matches_cpp_110_130_and_fixate_shape() {
    let mut combat = CombatSubsystem::default();
    combat.initialize_threat_list_capability(true);
    let current = guid(30);
    let ranged = guid(31);
    let melee = guid(32);

    combat.add_threat(current, 100.0);
    combat.current_victim_guid = Some(current);
    combat.add_threat(ranged, 120.0);
    assert_eq!(combat.reselect_victim(&HashSet::new()), Some(current));

    combat.set_threat(ranged, 131.0);
    assert_eq!(combat.reselect_victim(&HashSet::new()), Some(ranged));

    combat.current_victim_guid = Some(current);
    combat.set_threat(ranged, 120.0);
    assert_eq!(
        combat.reselect_victim(&HashSet::from([current])),
        Some(current),
        "C++ tests the challenger's melee range; the old victim being in melee does not lower the ranged threshold"
    );
    assert_eq!(
        combat.reselect_victim(&HashSet::from([ranged])),
        Some(ranged)
    );

    combat.current_victim_guid = Some(current);
    combat.add_threat(melee, 115.0);
    assert_eq!(
        combat.reselect_victim(&HashSet::from([melee])),
        Some(melee),
        "C++ scans below a ranged 110%-130% leader for the first melee challenger above 110%"
    );

    combat.current_victim_guid = Some(current);
    assert!(combat.set_threat_online_state(melee, ThreatOnlineState::Suppressed));
    assert_eq!(
        combat.reselect_victim(&HashSet::from([melee])),
        Some(current),
        "C++ excludes suppressed references from the fallback melee challenger scan"
    );
    assert!(combat.set_threat_online_state(melee, ThreatOnlineState::Online));

    combat.set_threat(melee, 1.0);
    assert!(combat.fixate_target(Some(melee)));
    assert_eq!(combat.reselect_victim(&HashSet::new()), Some(melee));
    assert!(combat.fixate_target(None));
    assert!(!combat.fixate_target(Some(guid(99))));
}

#[test]
fn combat_refs_track_pve_pvp_suppression_and_timeout_like_cpp() {
    let mut combat = CombatSubsystem::default();
    let creature = guid(40);
    let player = guid(41);

    assert!(combat.set_in_combat_with(creature, false, false));
    assert!(combat.has_pve_combat());
    assert!(combat.is_in_combat_with(creature));

    assert!(combat.set_in_combat_with(player, true, false));
    assert!(combat.has_pvp_combat());
    combat.initialize_threat_list_capability(true);
    combat.add_threat(player, 10.0);
    combat.put_threatened_by_me_ref(player, ThreatReferenceState::default());
    assert_eq!(
        combat
            .pvp_refs
            .get(&player)
            .and_then(|reference| reference.timeout_ms),
        Some(PVP_COMBAT_TIMEOUT_MS)
    );

    combat.suppress_pvp_combat();
    assert!(!combat.has_pvp_combat());
    assert!(combat.set_in_combat_with(player, true, false));
    assert!(combat.has_pvp_combat());

    assert!(
        combat
            .update_pvp_combat(PVP_COMBAT_TIMEOUT_MS - 1)
            .is_empty()
    );
    assert_eq!(combat.update_pvp_combat(1), vec![player]);
    assert!(!combat.has_pvp_combat());
    assert_eq!(combat.threat_value(player), None);
    assert!(combat.threatened_by_me_owner_guids().is_empty());
    assert_eq!(combat.current_victim_guid, None);

    combat.end_all_pve_combat();
    assert!(!combat.has_pve_combat());
    assert!(!combat.has_combat());
}
