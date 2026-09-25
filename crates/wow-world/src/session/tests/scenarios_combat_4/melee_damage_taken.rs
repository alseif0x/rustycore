use super::*;

#[test]
fn creature_aura_effects_resolve_the_applied_mask_like_cpp() {
    use wow_data::spell::aura_types::{
        SPELL_AURA_MOD_DAMAGE_TAKEN, SPELL_AURA_MOD_MELEE_DAMAGE_TAKEN,
    };

    let caster = ObjectGuid::create_player(1, 91);
    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        91_130,
        wow_data::SpellInfo {
            spell_id: 91_130,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
            effect_base_points: -50,
            effect_bonus_coefficient: 0.0,
            aura_type: Some(SPELL_AURA_MOD_DAMAGE_TAKEN),
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: vec![
                wow_data::SpellEffectInfo {
                    effect_index: 0,
                    effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                    effect_aura: SPELL_AURA_MOD_DAMAGE_TAKEN,
                    effect_misc_value_1: 0x01,
                    effect_base_points: -50,
                    ..Default::default()
                },
                wow_data::SpellEffectInfo {
                    effect_index: 1,
                    effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                    effect_aura: SPELL_AURA_MOD_MELEE_DAMAGE_TAKEN,
                    effect_misc_value_1: 0,
                    effect_base_points: -5,
                    ..Default::default()
                },
            ],
        },
    );

    // Only effect 0 is applied, so only its aura type is resolved.
    let effects = crate::session_rules::creature_aura_effects_like_cpp(
        &[wow_entities::AppliedAuraRef::new(91_130, caster, 0, 0b1)],
        &spell_store,
        0,
        None,
    );
    assert_eq!(effects.len(), 1);
    assert_eq!(effects[0].aura_type, SPELL_AURA_MOD_DAMAGE_TAKEN);
    assert_eq!(effects[0].misc_value, 0x01);
    assert_eq!(effects[0].amount, -50);
    assert_eq!(effects[0].caster_guid, caster);

    // Both effects when the application's mask covers both slots.
    let effects = crate::session_rules::creature_aura_effects_like_cpp(
        &[wow_entities::AppliedAuraRef::new(91_130, caster, 0, 0b11)],
        &spell_store,
        0,
        None,
    );
    assert_eq!(effects.len(), 2);
    assert_eq!(effects[1].aura_type, SPELL_AURA_MOD_MELEE_DAMAGE_TAKEN);
    assert_eq!(effects[1].amount, -5);
}

#[test]
fn melee_damage_taken_matches_cpp_like_cpp() {
    use crate::session_rules::{
        AppliedAuraEffectLikeCpp as Effect, RepresentedMeleeDamageTakenLikeCpp as Taken,
        melee_damage_taken_apply_like_cpp, melee_damage_taken_flat_pct_like_cpp,
    };
    use wow_data::spell::aura_types::{
        SPELL_AURA_MOD_DAMAGE_PERCENT_TAKEN, SPELL_AURA_MOD_DAMAGE_TAKEN,
        SPELL_AURA_MOD_MELEE_DAMAGE_FROM_CASTER, SPELL_AURA_MOD_MELEE_DAMAGE_TAKEN,
        SPELL_AURA_MOD_MELEE_DAMAGE_TAKEN_PCT,
    };

    let attacker = ObjectGuid::create_player(1, 92);
    let effect = |aura_type, misc_value, amount, caster_guid| Effect {
        slot: 0,
        spell_id: 1,
        caster_guid,
        aura_type,
        misc_value,
        misc_value_b: 0,
        amount,
    };

    // A normal-school `MOD_DAMAGE_TAKEN` and a `MOD_MELEE_DAMAGE_TAKEN` sum
    // into the flat benefit; a fire-school row does not cover normal damage.
    let effects = [
        effect(SPELL_AURA_MOD_DAMAGE_TAKEN, 0x01, -20, attacker),
        effect(SPELL_AURA_MOD_DAMAGE_TAKEN, 0x04, -100, attacker),
        effect(SPELL_AURA_MOD_MELEE_DAMAGE_TAKEN, 0, -5, attacker),
    ];
    let taken = melee_damage_taken_flat_pct_like_cpp(&effects, &[], attacker, 0x01);
    assert_eq!(
        taken,
        Taken {
            flat: -25,
            pct: 1.0
        }
    );
    assert_eq!(melee_damage_taken_apply_like_cpp(taken, 100), 75);
    // C++ returns zero before the arithmetic when the flat benefit absorbs the
    // whole hit.
    assert_eq!(melee_damage_taken_apply_like_cpp(taken, 10), 0);

    // Percent terms multiply: school mask, caster-restricted and the melee
    // taken percentage.
    let effects = [
        effect(SPELL_AURA_MOD_DAMAGE_PERCENT_TAKEN, 0x01, -50, attacker),
        effect(SPELL_AURA_MOD_MELEE_DAMAGE_TAKEN_PCT, 0, -50, attacker),
        effect(SPELL_AURA_MOD_MELEE_DAMAGE_FROM_CASTER, 0, 100, attacker),
        // Another caster's aura must not apply.
        effect(
            SPELL_AURA_MOD_MELEE_DAMAGE_FROM_CASTER,
            0,
            100,
            ObjectGuid::create_player(1, 93),
        ),
    ];
    let taken = melee_damage_taken_flat_pct_like_cpp(&effects, &[], attacker, 0x01);
    // `0.5 * 0.5 * 2.0`.
    assert_eq!(taken.pct, 0.5);
    assert_eq!(melee_damage_taken_apply_like_cpp(taken, 101), 50);

    // The Sanctified Wrath bypass shrinks the reduction with the attacker's
    // normal-school `SPELL_AURA_MOD_IGNORE_TARGET_RESIST`.
    let taken = melee_damage_taken_flat_pct_like_cpp(
        &[effect(
            SPELL_AURA_MOD_DAMAGE_PERCENT_TAKEN,
            0x01,
            -50,
            attacker,
        )],
        &[(0x01, 100)],
        attacker,
        0x01,
    );
    assert_eq!(taken.pct, 1.0);
    // A bypass that does not cover the school leaves the reduction alone.
    let taken = melee_damage_taken_flat_pct_like_cpp(
        &[effect(
            SPELL_AURA_MOD_DAMAGE_PERCENT_TAKEN,
            0x01,
            -50,
            attacker,
        )],
        &[(0x04, 100)],
        attacker,
        0x01,
    );
    assert_eq!(taken.pct, 0.5);
}

#[test]
fn white_swing_applies_victim_melee_damage_taken_like_cpp() {
    let (mut session, _, _) = make_session();
    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    let guid = test_creature_guid(18_037);
    let player = ObjectGuid::create_player(1, 94);

    canonical.lock().unwrap().create_world_map(0, 0);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
        wow_data::MapEntry {
            id: 0,
            instance_type: wow_data::map::MAP_COMMON,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
    ])));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player,
        "Taken".to_string(),
        Position::new(10.0, 10.0, 0.0, 0.0),
        0,
        1,
        1,
        80,
        0,
    ));
    let _ = session.ensure_canonical_world_map_for_current_player_like_cpp();
    session
        .mutate_canonical_player_like_cpp(|player| {
            let unit = player.unit_mut();
            unit.set_attacking(Some(guid));
            unit.set_target(guid);
            unit.add_unit_state(UnitState::MELEE_ATTACKING.bits());
            unit.set_base_attack_time_like_cpp(WeaponAttackType::BaseAttack, 2_000);
            unit.set_attack_timer(WeaponAttackType::BaseAttack, 0);
            unit.set_weapon_damage(WeaponAttackType::BaseAttack, 100.0, 100.0);
        })
        .unwrap();
    session.combat_target = Some(guid);
    session.in_combat = true;
    register_test_creature(&mut session, manager.clone(), guid, 40);
    session
        .mutate_world_creature(guid, |creature| {
            creature.enter_combat(player);
            creature.creature.ai_ownership_mut().last_swing_ms = 0;
            creature.creature.ai_ownership_mut().swing_timer_ms = 0;
        })
        .unwrap();

    // The victim's `SPELL_AURA_MOD_MELEE_DAMAGE_TAKEN` halves the swing and the
    // attacker's `SPELL_AURA_MOD_IGNORE_TARGET_RESIST` bypasses a
    // `SPELL_AURA_MOD_DAMAGE_PERCENT_TAKEN` reduction.
    let mut spell_store = wow_data::SpellStore::new();
    for (spell_id, aura, misc, amount) in [
        (
            91_131_i32,
            wow_data::spell::aura_types::SPELL_AURA_MOD_MELEE_DAMAGE_TAKEN,
            0,
            -50,
        ),
        (
            91_132,
            wow_data::spell::aura_types::SPELL_AURA_MOD_DAMAGE_PERCENT_TAKEN,
            0x01,
            -50,
        ),
        (
            91_133,
            wow_data::spell::aura_types::SPELL_AURA_MOD_IGNORE_TARGET_RESIST,
            0x01,
            100,
        ),
    ] {
        spell_store.insert(
            spell_id,
            wow_data::SpellInfo {
                spell_id,
                cast_time_ms: 0,
                cooldown_ms: 0,
                recovery_time_ms: 0,
                effect_type: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                effect_base_points: amount,
                effect_bonus_coefficient: 0.0,
                aura_type: Some(aura),
                display_flags: 0,
                requires_spell_focus: 0,
                power_costs: Vec::new(),
                effects: vec![wow_data::SpellEffectInfo {
                    effect_index: 0,
                    effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                    effect_aura: aura,
                    effect_misc_value_1: misc,
                    effect_base_points: amount,
                    ..Default::default()
                }],
            },
        );
    }
    session.set_spell_store(Arc::new(spell_store));

    let swing = |session: &mut WorldSession| {
        let melee_damage_bonus = session.represented_melee_damage_bonus_like_cpp();
        let armor_mitigation = session.represented_melee_armor_mitigation_like_cpp();
        let outcome_facts = session.represented_melee_outcome_facts_like_cpp();
        let damage_taken = session.represented_melee_damage_taken_like_cpp();
        session
            .mutate_canonical_player_like_cpp(|player| {
                player
                    .unit_mut()
                    .set_attack_timer(WeaponAttackType::BaseAttack, 0);
                take_canonical_player_attack_swings_like_cpp(
                    player,
                    0,
                    true,
                    true,
                    true,
                    melee_damage_bonus,
                    armor_mitigation,
                    outcome_facts,
                    damage_taken,
                )
            })
            .flatten()
            .map(|(swings, _)| swings)
    };

    assert_eq!(
        swing(&mut session).map(|swings| swings[0].damage),
        Some(100),
        "no victim modifier"
    );

    // The flat -50 melee-damage-taken aura applies first; taken modifiers live
    // on the victim.
    let apply_victim_aura = |session: &mut WorldSession, spell_id: u32| {
        session
            .mutate_world_creature(guid, |creature| {
                creature
                    .creature
                    .unit_mut()
                    .subsystems_mut()
                    .auras
                    .add_applied(wow_entities::AppliedAuraRef::new(spell_id, player, 0, 1));
            })
            .expect("victim aura");
    };
    apply_victim_aura(&mut session, 91_131);
    assert_eq!(swing(&mut session).map(|swings| swings[0].damage), Some(50));

    // A -50% damage-percent-taken aura on the victim halves `(100 - 50) * 0.5`.
    apply_victim_aura(&mut session, 91_132);
    assert_eq!(swing(&mut session).map(|swings| swings[0].damage), Some(25));

    // The attacker's 100% normal-school ignore-resist aura cancels the
    // reduction, so only the flat term remains.
    session
        .apply_aura(91_133, player, 30_000, 1)
        .expect("apply ignore-target-resist aura");
    assert_eq!(swing(&mut session).map(|swings| swings[0].damage), Some(50));
}
