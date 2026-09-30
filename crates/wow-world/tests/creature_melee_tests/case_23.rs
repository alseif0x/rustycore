use super::*;

/// C++ `Unit::MeleeDamageBonusTaken` and `CalcArmorReducedDamage` for a
/// creature victim under the global creature runtime.
///
/// The creature-vs-creature branch keeps the compatibility bridge's outcome,
/// but `CalculateMeleeDamage` (`Unit.cpp:1326-1343`) still runs the victim's
/// taken chain and armour before it, so the committed health must reflect the
/// victim creature's own `GetArmor()` and damage-taken auras.
#[test]
fn legacy_creature_melee_tick_once_mitigates_creature_victim_like_cpp() {
    use wow_world::map_manager::RuntimeTickOwner;

    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    canonical.lock().unwrap().create_world_map(0, 0);
    let attacker_guid = test_creature_guid(91_200);
    let victim_guid = test_creature_guid(91_201);

    let (mut session, _, _) = make_session();
    session.set_canonical_map_manager(Arc::clone(&canonical));
    register_test_creature(&mut session, manager.clone(), attacker_guid, 25);
    register_test_creature(&mut session, manager.clone(), victim_guid, 1_000);
    {
        let mut guard = canonical.lock().unwrap();
        let map = guard.find_map_mut(0, 0).unwrap().map_mut();
        map.get_typed_creature_mut(victim_guid)
            .unwrap()
            .unit_mut()
            .set_level(80);
        map.get_typed_creature_mut(victim_guid)
            .unwrap()
            .set_combat_log_stats_like_cpp(wow_entities::CreatureCombatLogStatsLikeCpp {
                armor: 5_000,
                ..Default::default()
            });
        map.get_typed_creature_mut(attacker_guid)
            .unwrap()
            .unit_mut()
            .set_level(80);
    }
    session
        .fixture_melee_mutate_creature(attacker_guid, |creature| {
            creature.creature.unit_mut().set_level(80);
            creature.creature.ai_ownership_mut().min_damage = 10;
            creature.creature.ai_ownership_mut().max_damage = 10;
            // The stages assert exact mitigated damage, so the attacker's flat
            // 5% critical band stays off.
            creature.creature.set_flags_extra_runtime_like_cpp(
                wow_constants::CreatureFlagsExtra::NO_CRIT.bits(),
            );
            creature.enter_combat(victim_guid);
            creature.creature.ai_ownership_mut().last_swing_ms = 0;
            creature.creature.ai_ownership_mut().swing_timer_ms = 0;
        })
        .unwrap();

    let mut spell_store = wow_data::SpellStore::new();
    for (spell_id, aura_type, amount) in [
        (
            91_200_i32,
            wow_data::spell::aura_types::SPELL_AURA_MOD_MELEE_DAMAGE_TAKEN,
            5_i32,
        ),
        (
            91_201,
            wow_data::spell::aura_types::SPELL_AURA_MOD_TARGET_RESISTANCE,
            -5_000,
        ),
        (
            91_202,
            wow_data::spell::aura_types::SPELL_AURA_MOD_ATTACKER_MELEE_HIT_CHANCE,
            5,
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
                aura_type: Some(aura_type),
                display_flags: 0,
                requires_spell_focus: 0,
                power_costs: Vec::new(),
                effects: vec![wow_data::SpellEffectInfo {
                    effect_index: 0,
                    effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                    effect_aura: aura_type,
                    effect_misc_value_1: 0x01,
                    effect_base_points: amount,
                    ..Default::default()
                }],
            },
        );
    }
    let config = wow_world::session::LegacyCreatureAggroConfigLikeCpp {
        spell_store: Some(Arc::new(spell_store)),
        ..Default::default()
    };
    manager
        .write()
        .unwrap()
        .set_tick_owner(RuntimeTickOwner::GlobalLegacy);

    // The victim's `+5` restricts the flat 5.0 miss band to zero, so the
    // mitigation stages stay deterministic.
    canonical
        .lock()
        .unwrap()
        .find_map_mut(0, 0)
        .unwrap()
        .map_mut()
        .with_creature_mut_like_cpp(victim_guid, |victim| {
            victim.unit_mut().subsystems_mut().auras.add_applied(
                wow_entities::AppliedAuraRef::new(91_202, attacker_guid, 0, 1),
            );
        })
        .unwrap();
    let victim_health = |canonical: &SharedCanonicalMapManager| {
        canonical
            .lock()
            .unwrap()
            .find_map(0, 0)
            .unwrap()
            .map()
            .with_creature_like_cpp(victim_guid, |victim| victim.unit().data().health)
            .unwrap()
    };
    let tick = |session: &mut WorldSession| {
        session
            .fixture_melee_mutate_creature(attacker_guid, |creature| {
                creature.creature.ai_ownership_mut().last_swing_ms = 0;
                creature.creature.ai_ownership_mut().swing_timer_ms = 0;
            })
            .unwrap();
        run_legacy_creature_melee_tick_once_like_cpp(&manager, Some(&canonical), &config)
    };

    // Baseline: 5,000 armour at level 80 -> `ceil(10 * 0.752873) = 8`.
    let outcome = tick(&mut session);
    assert_eq!(outcome.canonical_creature_hits, 1);
    assert_eq!(1_000 - victim_health(&canonical), 8);

    // The victim creature's flat `MOD_MELEE_DAMAGE_TAKEN`: `(10 + 5)` -> 12.
    canonical
        .lock()
        .unwrap()
        .find_map_mut(0, 0)
        .unwrap()
        .map_mut()
        .with_creature_mut_like_cpp(victim_guid, |victim| {
            victim.unit_mut().subsystems_mut().auras.add_applied(
                wow_entities::AppliedAuraRef::new(91_200, attacker_guid, 0, 1),
            );
        })
        .unwrap();
    let before = victim_health(&canonical);
    let outcome = tick(&mut session);
    assert_eq!(outcome.canonical_creature_hits, 1);
    assert_eq!(before - victim_health(&canonical), 12);

    // The attacker's normal-school `MOD_TARGET_RESISTANCE` cancels the armour,
    // so the full `(10 + 5)` lands.
    session
        .fixture_melee_mutate_creature(attacker_guid, |creature| {
            creature
                .creature
                .unit_mut()
                .subsystems_mut()
                .auras
                .add_applied(wow_entities::AppliedAuraRef::new(91_201, victim_guid, 0, 1));
        })
        .unwrap();
    let before = victim_health(&canonical);
    let outcome = tick(&mut session);
    assert_eq!(outcome.canonical_creature_hits, 1);
    assert_eq!(before - victim_health(&canonical), 15);
}
