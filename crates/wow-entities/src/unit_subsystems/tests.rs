// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Unit subsystem tests for [`super`].
//!
//! Moved from the inline `unit_subsystems_tests` module by issue #226.

#![cfg(test)]

use super::*;

mod unit_subsystems_tests {
    use super::*;

    fn guid(low: i64) -> ObjectGuid {
        ObjectGuid::new(0, low)
    }

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
                .with_flags(
                    MOVEMENTGENERATOR_FLAG_INITIALIZED | MOVEMENTGENERATOR_FLAG_INFORM_ENABLED,
                )
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

        let mut right =
            MovementGeneratorRef::new(MovementGeneratorKind::Rotate, MovementSlot::Active)
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
                    MOVEMENTGENERATOR_FLAG_INITIALIZATION_PENDING
                        | MOVEMENTGENERATOR_FLAG_DEACTIVATED,
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

        let mut blocked =
            MovementGeneratorRef::new(MovementGeneratorKind::Point, MovementSlot::Active)
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

        let mut cyclic =
            MovementGeneratorRef::new(MovementGeneratorKind::Effect, MovementSlot::Active)
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

        assert!(target.apply_charmed_by(
            controller_guid,
            CharmType::Possess,
            true,
            Some(123),
            true,
        ));
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
}
