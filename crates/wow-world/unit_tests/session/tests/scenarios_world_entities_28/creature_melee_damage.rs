//! Legacy creature-melee damage and outcome scenarios.

use super::*;

#[test]
fn legacy_creature_melee_tick_once_preserves_compatibility_creature_damage_like_cpp() {
    use crate::map_manager::RuntimeTickOwner;
    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    let victim = test_creature_guid(91_028);
    add_canonical_test_creature_on_map(
        &canonical,
        victim,
        9002,
        Position::new(10.0, 10.0, 0.0, 0.0),
        0,
        0,
        0,
    );
    {
        let mut guard = canonical.lock().unwrap();
        let typed = guard
            .find_map_mut(0, 0)
            .unwrap()
            .map_mut()
            .get_typed_creature_mut(victim)
            .unwrap();
        typed.unit_mut().set_level(80);
        typed.unit_mut().set_max_health(100);
        typed.unit_mut().set_health(100);
    }

    let (mut session, _, _) = make_session();
    session.set_canonical_map_manager(Arc::clone(&canonical));
    let attacker = test_creature_guid(91_029);
    add_canonical_test_creature_on_map(
        &canonical,
        attacker,
        9001,
        Position::new(10.0, 10.0, 0.0, 0.0),
        0,
        0,
        0,
    );
    register_test_creature(&mut session, manager.clone(), victim, 100);
    register_test_creature(&mut session, manager.clone(), attacker, 25);
    session
        .mutate_world_creature(attacker, |creature| {
            creature.enter_combat(victim);
            creature.creature.ai_ownership_mut().last_swing_ms = 0;
            creature.creature.ai_ownership_mut().swing_timer_ms = 0;
        })
        .unwrap();
    manager
        .write()
        .unwrap()
        .set_tick_owner(RuntimeTickOwner::GlobalLegacy);

    let outcome = run_legacy_creature_melee_tick_once_like_cpp(
        &manager,
        Some(&canonical),
        &Default::default(),
    );

    assert_eq!(outcome.swings_ready, 1);
    assert_eq!(outcome.melee_outcomes_unrepresented, 1);
    assert_eq!(outcome.canonical_hits, 1);
    assert_eq!(outcome.canonical_creature_hits, 1);
    assert_eq!(outcome.legacy_creature_victim_syncs, 1);
    assert!(outcome.commands.is_empty());
    assert_eq!(outcome.plan.events.len(), 2);
    let health = canonical
        .lock()
        .unwrap()
        .find_map(0, 0)
        .unwrap()
        .map()
        .with_creature_like_cpp(victim, Clone::clone)
        .unwrap()
        .unit()
        .data()
        .health;
    assert!(health < 100);
    let legacy_health = manager
        .read()
        .unwrap()
        .find_creature(0, 0, victim)
        .unwrap()
        .creature
        .unit()
        .data()
        .health;
    assert_eq!(
        legacy_health, health,
        "the compatibility bridge must keep both creature mirrors coherent"
    );
    assert!(
        !manager
            .read()
            .unwrap()
            .find_creature(0, 0, attacker)
            .unwrap()
            .runtime_rng_authority_complete_like_cpp()
    );
}
#[test]
fn legacy_creature_melee_tick_once_prevents_postmortem_cross_kill_like_cpp() {
    use crate::map_manager::RuntimeTickOwner;

    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    let creatures = [test_creature_guid(91_043), test_creature_guid(91_044)];
    for creature_guid in creatures {
        add_canonical_test_creature_on_map(
            &canonical,
            creature_guid,
            9001,
            Position::new(10.0, 10.0, 0.0, 0.0),
            0,
            0,
            0,
        );
        let mut guard = canonical.lock().unwrap();
        let creature = guard
            .find_map_mut(0, 0)
            .unwrap()
            .map_mut()
            .get_typed_creature_mut(creature_guid)
            .unwrap();
        creature.unit_mut().set_max_health(1);
        creature.unit_mut().set_health(1);
    }

    let (mut session, _, _) = make_session();
    session.set_canonical_map_manager(Arc::clone(&canonical));
    for (attacker, victim) in [(creatures[0], creatures[1]), (creatures[1], creatures[0])] {
        register_test_creature(&mut session, Arc::clone(&manager), attacker, 1);
        session
            .mutate_world_creature(attacker, |creature| {
                creature.enter_combat(victim);
                creature.creature.ai_ownership_mut().last_swing_ms = 0;
                creature.creature.ai_ownership_mut().swing_timer_ms = 0;
                creature.creature.ai_ownership_mut().min_damage = 1;
                creature.creature.ai_ownership_mut().max_damage = 1;
            })
            .unwrap();
    }
    manager
        .write()
        .unwrap()
        .set_tick_owner(RuntimeTickOwner::GlobalLegacy);

    let outcome = run_legacy_creature_melee_tick_once_like_cpp(
        &manager,
        Some(&canonical),
        &Default::default(),
    );

    assert_eq!(outcome.swings_ready, 2);
    assert_eq!(outcome.melee_outcomes_unrepresented, 1);
    assert_eq!(outcome.canonical_hits, 1);
    assert_eq!(outcome.canonical_creature_hits, 1);
    assert_eq!(outcome.melee_precondition_rejections, 1);
    assert_eq!(outcome.legacy_creature_victim_syncs, 1);
    assert_eq!(outcome.legacy_creature_victim_sync_cas_rejections, 0);
    assert!(outcome.commands.is_empty());
    assert_eq!(outcome.plan.events.len(), 2);

    let canonical_guard = canonical.lock().unwrap();
    let legacy_guard = manager.read().unwrap();
    let mut alive = 0;
    let mut dead = 0;
    let mut tombstoned = 0;
    for guid in creatures {
        let canonical_creature = canonical_guard
            .find_map(0, 0)
            .unwrap()
            .map()
            .with_creature_like_cpp(guid, Clone::clone)
            .unwrap();
        let legacy_creature = legacy_guard.find_creature(0, 0, guid).unwrap();
        assert_eq!(
            legacy_creature.creature.unit().data().health,
            canonical_creature.unit().data().health
        );
        assert_eq!(
            legacy_creature.creature.unit().death_state(),
            canonical_creature.unit().death_state()
        );
        match (
            canonical_creature.unit().data().health,
            canonical_creature.unit().death_state(),
        ) {
            (1, wow_constants::DeathState::Alive) => alive += 1,
            (0, wow_constants::DeathState::Corpse) => dead += 1,
            tuple => panic!("unexpected cross-kill final tuple: {tuple:?}"),
        }
        if !legacy_creature.runtime_rng_authority_complete_like_cpp() {
            tombstoned += 1;
        }
    }
    assert_eq!((alive, dead), (1, 1));
    assert_eq!(tombstoned, 1);
}

/// C++ `Unit::CalcArmorReducedDamage`'s attacker
/// `SPELL_AURA_MOD_IGNORE_TARGET_RESIST` term through the production swing owner.
///
/// The map-owned `run_legacy_player_melee_tick_once_like_cpp` resolves the
/// attacker's live auras, so the normal-school ignore-resist sum must shrink the
/// victim's armour by `std::floor(AddPct(armor, -amount))` before the reduction
/// curve (`Unit.cpp:1646-1651`), not only in the session path.
#[tokio::test]
async fn map_owned_player_melee_applies_attacker_ignore_target_resist_like_cpp() {
    use crate::map_manager::RuntimeTickOwner;

    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    canonical.lock().unwrap().create_world_map(0, 0);
    manager
        .write()
        .unwrap()
        .set_tick_owner(RuntimeTickOwner::GlobalLegacy);

    let creature_guid = test_creature_guid(99_933);
    let player_guid = ObjectGuid::create_player(1, 5_203);
    let map_store = Arc::new(wow_data::MapStore::from_entries([wow_data::MapEntry {
        id: 0,
        instance_type: wow_data::map::MAP_COMMON,
        expansion_id: 0,
        parent_map_id: -1,
        cosmetic_parent_map_id: -1,
        flags1: 0,
        flags2: 0,
    }]));

    let (mut session, _pkt_tx, _send_rx) = make_session();
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_map_store(Arc::clone(&map_store));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "IgnoreResistSolo".to_string(),
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
            unit.set_attacking(Some(creature_guid));
            unit.set_target(creature_guid);
            unit.add_unit_state(UnitState::MELEE_ATTACKING.bits());
            unit.set_base_attack_time_like_cpp(WeaponAttackType::BaseAttack, 2_000);
            unit.set_weapon_damage(WeaponAttackType::BaseAttack, 1_000.0, 1_000.0);
            unit.reset_attack_timer_like_cpp(WeaponAttackType::BaseAttack);
        })
        .unwrap();
    session.set_map_manager(Arc::clone(&manager));
    register_test_creature(&mut session, manager.clone(), creature_guid, 100_000);
    session
        .mutate_world_creature(creature_guid, |creature| {
            // `Unit::IsTotem()` zeroes the victim's dodge, parry and block, so the
            // swing cannot be avoided and the damage term is isolated.
            creature
                .creature
                .add_unit_type_mask_like_cpp(wow_entities::UNIT_MASK_TOTEM);
            creature.creature.unit_mut().set_level(80);
            creature.creature.set_combat_log_stats_like_cpp(
                wow_entities::CreatureCombatLogStatsLikeCpp {
                    armor: 5_000,
                    ..Default::default()
                },
            );
        })
        .unwrap();

    let spell_id = 91_130_i32;
    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        spell_id,
        wow_data::SpellInfo {
            spell_id,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
            effect_base_points: 50,
            effect_bonus_coefficient: 0.0,
            aura_type: Some(wow_data::spell::aura_types::SPELL_AURA_MOD_IGNORE_TARGET_RESIST),
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: vec![wow_data::SpellEffectInfo {
                effect_index: 0,
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                effect_aura: wow_data::spell::aura_types::SPELL_AURA_MOD_IGNORE_TARGET_RESIST,
                effect_misc_value_1: 0x01,
                effect_base_points: 50,
                ..Default::default()
            }],
        },
    );
    let spell_store = Arc::new(spell_store);
    session.set_spell_store(Arc::clone(&spell_store));
    session
        .apply_aura(spell_id, player_guid, 30_000, 1)
        .expect("apply ignore-resist aura");

    let (send_tx, _rx) = flume::bounded::<Vec<u8>>(8);
    let (command_tx, _crx) = flume::bounded::<SessionCommand>(8);
    let registration = PlayerRegistry::new().register_or_replace(
        player_guid,
        broadcast_info_with_command(player_guid, send_tx, command_tx),
        Default::default(),
    );
    let attackers = vec![crate::session::PlayerMeleeAttackerSnapshotLikeCpp {
        registration,
        player_guid,
        map_id: 0,
        instance_id: 0,
        in_combat_mirror: true,
        tap_group_guids: Vec::new(),
    }];
    let config = crate::session::LegacyCreatureAggroConfigLikeCpp {
        spell_store: Some(spell_store),
        ..Default::default()
    };
    let mut phase_state = crate::session::PlayerMeleePhaseStateLikeCpp::default();
    let health = |manager: &crate::map_manager::SharedMapManager| {
        manager
            .read()
            .unwrap()
            .find_creature(0, 0, creature_guid)
            .unwrap()
            .current_hp()
    };
    let before = health(&manager);
    let outcome = crate::session::run_legacy_player_melee_tick_once_like_cpp(
        &manager,
        Some(&canonical),
        &attackers,
        2_000,
        &mut phase_state,
        &config,
    );
    assert!(!outcome.skipped_owner_not_global, "the map owns this tick");
    assert_eq!(outcome.creature_hits, 1);
    assert_eq!(
        before - health(&manager),
        860,
        "50% ignore-resist halves the 5,000 armour before the reduction curve"
    );
}

/// C++ `Unit::MeleeDamageBonusTaken` and `CalcArmorReducedDamage` for a
/// creature victim under the global creature runtime.
///
/// The creature-vs-creature branch keeps the compatibility bridge's outcome,
/// but `CalculateMeleeDamage` (`Unit.cpp:1326-1343`) still runs the victim's
/// taken chain and armour before it, so the committed health must reflect the
/// victim creature's own `GetArmor()` and damage-taken auras.
#[test]
fn legacy_creature_melee_tick_once_mitigates_creature_victim_like_cpp() {
    use crate::map_manager::RuntimeTickOwner;

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
        .mutate_world_creature(attacker_guid, |creature| {
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
    let config = crate::session::LegacyCreatureAggroConfigLikeCpp {
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
            .mutate_world_creature(attacker_guid, |creature| {
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
        .mutate_world_creature(attacker_guid, |creature| {
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

/// C++ `Unit::RollMeleeOutcomeAgainst`'s bands for a creature victim under the
/// global creature runtime.
///
/// `MeleeSpellMissChance` (`Unit.cpp:11652-11685`) starts from the victim's flat
/// `GetUnitMissChance()` of `5.0` and subtracts the attacker's
/// `SPELL_AURA_MOD_HIT_CHANCE` sum and the victim's
/// `SPELL_AURA_MOD_ATTACKER_MELEE_HIT_CHANCE` sum; the victim's
/// `CreatureAvoidanceLikeCpp` dodge base and the critical band from
/// `GetUnitCriticalChanceAgainst` follow (`Unit.cpp:2272-2360`). A missed or
/// avoided swing publishes zero dealt damage and commits no health transition
/// (`Unit.cpp:1348-1355`).
#[test]
fn legacy_creature_melee_tick_once_resolves_creature_victim_bands_like_cpp() {
    use crate::map_manager::RuntimeTickOwner;
    use wow_packet::packets::combat::{HIT_INFO_AFFECTS_VICTIM, HIT_INFO_MISS};

    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    canonical.lock().unwrap().create_world_map(0, 0);
    let attacker_guid = test_creature_guid(91_210);
    let victim_guid = test_creature_guid(91_211);

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
        .mutate_world_creature(attacker_guid, |creature| {
            creature.creature.unit_mut().set_level(80);
            creature.creature.ai_ownership_mut().min_damage = 10;
            creature.creature.ai_ownership_mut().max_damage = 10;
            // The landed and dodge stages assert exact numbers, so the flat 5%
            // critical band is disabled; the critical stage supplies its own
            // `+100` victim aura.
            creature.creature.set_flags_extra_runtime_like_cpp(
                wow_constants::CreatureFlagsExtra::NO_CRIT.bits(),
            );
            creature.enter_combat(victim_guid);
            creature.creature.ai_ownership_mut().last_swing_ms = 0;
            creature.creature.ai_ownership_mut().swing_timer_ms = 0;
        })
        .unwrap();

    let mut spell_store = wow_data::SpellStore::new();
    for (spell_id, aura_type, amount, misc_value_b) in [
        (
            91_220_i32,
            wow_data::spell::aura_types::SPELL_AURA_MOD_ATTACKER_MELEE_HIT_CHANCE,
            5_i32,
            0_i32,
        ),
        (
            91_221,
            wow_data::spell::aura_types::SPELL_AURA_MOD_ATTACKER_MELEE_HIT_CHANCE,
            -200,
            0,
        ),
        (
            91_222,
            wow_data::spell::aura_types::SPELL_AURA_MOD_CRIT_CHANCE_VERSUS_TARGET_HEALTH,
            100,
            50,
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
                    effect_misc_value_2: misc_value_b,
                    effect_base_points: amount,
                    ..Default::default()
                }],
            },
        );
    }
    spell_store.insert(
        91_223,
        wow_data::SpellInfo {
            spell_id: 91_223,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
            effect_base_points: 0,
            effect_bonus_coefficient: 0.0,
            aura_type: Some(wow_data::spell::aura_types::SPELL_AURA_SCHOOL_IMMUNITY),
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: vec![wow_data::SpellEffectInfo {
                effect_index: 0,
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                effect_aura: wow_data::spell::aura_types::SPELL_AURA_SCHOOL_IMMUNITY,
                // `MiscValue` is the school mask: the normal school.
                effect_misc_value_1: 0x01,
                effect_base_points: 0,
                ..Default::default()
            }],
        },
    );
    let config = crate::session::LegacyCreatureAggroConfigLikeCpp {
        spell_store: Some(Arc::new(spell_store)),
        ..Default::default()
    };
    let apply_victim_aura = |canonical: &SharedCanonicalMapManager, spell_id: u32| {
        canonical
            .lock()
            .unwrap()
            .find_map_mut(0, 0)
            .unwrap()
            .map_mut()
            .with_creature_mut_like_cpp(victim_guid, |victim| {
                victim.unit_mut().subsystems_mut().auras.add_applied(
                    wow_entities::AppliedAuraRef::new(spell_id, attacker_guid, 0, 1),
                );
            })
            .unwrap();
    };
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
            .mutate_world_creature(attacker_guid, |creature| {
                creature.creature.ai_ownership_mut().last_swing_ms = 0;
                creature.creature.ai_ownership_mut().swing_timer_ms = 0;
            })
            .unwrap();
        run_legacy_creature_melee_tick_once_like_cpp(&manager, Some(&canonical), &config)
    };
    let wire_hit_info = |outcome: &crate::session::LegacyCreatureMeleeTickOutcomeLikeCpp| {
        let event = outcome
            .plan
            .events
            .iter()
            .find(|event| {
                event.packet_bytes.len() > 2
                    && u16::from_le_bytes([event.packet_bytes[0], event.packet_bytes[1]])
                        == wow_constants::ServerOpcodes::AttackerStateUpdate as u16
            })
            .expect("attacker state update event");
        let mut packet = wow_packet::world_packet::WorldPacket::from_bytes(&event.packet_bytes);
        packet.read_uint16().expect("opcode");
        packet.read_bit().expect("has_log_data");
        let info_len = packet.read_uint32().expect("attackRoundInfo size") as usize;
        let info_bytes = packet.read_bytes(info_len).expect("attackRoundInfo bytes");
        let mut attack_round_info = wow_packet::world_packet::WorldPacket::from_bytes(&info_bytes);
        attack_round_info.read_uint32().expect("hitInfo")
    };

    // `CombatLogPackets.cpp:355-365`: the presence byte, then the school mask,
    // float and integer sub-damage. The absorbed amount is only serialized when
    // the hit carries one of the absorb bits.
    let read_sub_damage = |info: &mut wow_packet::world_packet::WorldPacket, hit_info: u32| {
        if info.read_uint8().expect("sub damage present") != 0 {
            info.read_int32().expect("sub damage school mask");
            info.read_float().expect("sub damage float");
            info.read_int32().expect("sub damage");
            if hit_info
                & (wow_packet::packets::combat::HIT_INFO_FULL_ABSORB
                    | wow_packet::packets::combat::HIT_INFO_PARTIAL_ABSORB)
                != 0
            {
                info.read_int32().expect("absorbed");
            }
        }
    };

    // Decode `victimState` sequentially: hitInfo, both packed guids, damage,
    // original, over, the sub-damage block and then the victim state.
    let wire_victim_state = |outcome: &crate::session::LegacyCreatureMeleeTickOutcomeLikeCpp| {
        let event = outcome
            .plan
            .events
            .iter()
            .find(|event| {
                event.packet_bytes.len() > 2
                    && u16::from_le_bytes([event.packet_bytes[0], event.packet_bytes[1]])
                        == wow_constants::ServerOpcodes::AttackerStateUpdate as u16
            })
            .expect("attacker state update event");
        let mut packet = wow_packet::world_packet::WorldPacket::from_bytes(&event.packet_bytes);
        packet.read_uint16().expect("opcode");
        packet.read_bit().expect("has_log_data");
        let info_len = packet.read_uint32().expect("attackRoundInfo size") as usize;
        let info_bytes = packet.read_bytes(info_len).expect("attackRoundInfo bytes");
        let mut info = wow_packet::world_packet::WorldPacket::from_bytes(&info_bytes);
        let hit_info = info.read_uint32().expect("hitInfo");
        info.read_packed_guid().expect("attacker");
        info.read_packed_guid().expect("victim");
        info.read_int32().expect("damage");
        info.read_int32().expect("original damage");
        info.read_int32().expect("over damage");
        read_sub_damage(&mut info, hit_info);
        info.read_uint8().expect("victim state")
    };

    // Decode the appended `int32(BlockAmount)` exactly like the packet
    // writer's own block test.
    let wire_blocked = |outcome: &crate::session::LegacyCreatureMeleeTickOutcomeLikeCpp| {
        let event = outcome
            .plan
            .events
            .iter()
            .find(|event| {
                event.packet_bytes.len() > 2
                    && u16::from_le_bytes([event.packet_bytes[0], event.packet_bytes[1]])
                        == wow_constants::ServerOpcodes::AttackerStateUpdate as u16
            })
            .expect("attacker state update event");
        let mut packet = wow_packet::world_packet::WorldPacket::from_bytes(&event.packet_bytes);
        packet.read_uint16().expect("opcode");
        packet.read_bit().expect("has_log_data");
        let info_len = packet.read_uint32().expect("attackRoundInfo size") as usize;
        let info_bytes = packet.read_bytes(info_len).expect("attackRoundInfo bytes");
        let mut info = wow_packet::world_packet::WorldPacket::from_bytes(&info_bytes);
        let hit_info = info.read_uint32().expect("hitInfo");
        info.read_packed_guid().expect("attacker");
        info.read_packed_guid().expect("victim");
        info.read_int32().expect("damage");
        info.read_int32().expect("original damage");
        info.read_int32().expect("over damage");
        read_sub_damage(&mut info, hit_info);
        info.read_uint8().expect("victim state");
        info.read_uint32().expect("attacker state");
        info.read_uint32().expect("melee spell id");
        info.read_int32().expect("blocked")
    };

    manager
        .write()
        .unwrap()
        .set_tick_owner(RuntimeTickOwner::GlobalLegacy);

    // `+5` cancels the flat 5.0, so the mitigated `ceil(10 * 0.752873) = 8`
    // lands.
    apply_victim_aura(&canonical, 91_220_u32);
    let outcome = tick(&mut session);
    assert_eq!(outcome.melee_outcomes_unrepresented, 0);
    assert_eq!(outcome.canonical_creature_hits, 1);
    assert_eq!(1_000 - victim_health(&canonical), 8);
    assert_eq!(wire_hit_info(&outcome), HIT_INFO_AFFECTS_VICTIM);

    // The victim creature's own `CreatureAvoidanceLikeCpp` dodge base is the
    // first avoidance band.
    canonical
        .lock()
        .unwrap()
        .find_map_mut(0, 0)
        .unwrap()
        .map_mut()
        .with_creature_mut_like_cpp(victim_guid, |victim| {
            victim.set_avoidance_like_cpp(wow_entities::CreatureAvoidanceLikeCpp {
                dodge_pct: 100.0,
                ..Default::default()
            });
        })
        .unwrap();
    let before = victim_health(&canonical);
    let outcome = tick(&mut session);
    assert_eq!(outcome.melee_outcomes_unrepresented, 0);
    assert_eq!(outcome.canonical_creature_hits, 0, "a dodge commits no hit");
    assert_eq!(victim_health(&canonical), before);

    // C++ `GetUnitParryChance`'s creature base (`CreatureAvoidanceLikeCpp`) is
    // the next avoidance band; a parry publishes `VICTIMSTATE_PARRY`.
    canonical
        .lock()
        .unwrap()
        .find_map_mut(0, 0)
        .unwrap()
        .map_mut()
        .with_creature_mut_like_cpp(victim_guid, |victim| {
            victim.set_avoidance_like_cpp(wow_entities::CreatureAvoidanceLikeCpp {
                parry_pct: 100.0,
                ..Default::default()
            });
        })
        .unwrap();
    let before = victim_health(&canonical);
    let outcome = tick(&mut session);
    assert_eq!(outcome.melee_outcomes_unrepresented, 0);
    assert_eq!(outcome.canonical_creature_hits, 0, "a parry commits no hit");
    assert_eq!(victim_health(&canonical), before);
    assert_eq!(
        wire_victim_state(&outcome),
        wow_packet::packets::combat::VICTIM_STATE_PARRY
    );

    // C++ `RollMeleeOutcomeAgainst` returns `MELEE_HIT_EVADE` before every band
    // when the creature victim is evading (`Unit.cpp:2274-2275`), and
    // `CalculateMeleeDamage`'s evade arm publishes `HITINFO_MISS |
    // HITINFO_SWINGNOHITSOUND` with `VICTIMSTATE_EVADES` and zero damage
    // (`Unit.cpp:1345-1355`).
    canonical
        .lock()
        .unwrap()
        .find_map_mut(0, 0)
        .unwrap()
        .map_mut()
        .with_creature_mut_like_cpp(victim_guid, |victim| {
            victim.set_avoidance_like_cpp(wow_entities::CreatureAvoidanceLikeCpp::default());
            victim.set_in_evade_mode_like_cpp(true);
        })
        .unwrap();
    let before = victim_health(&canonical);
    let outcome = tick(&mut session);
    assert_eq!(outcome.melee_outcomes_unrepresented, 0);
    assert_eq!(
        outcome.canonical_creature_hits, 0,
        "an evade commits no hit"
    );
    assert_eq!(victim_health(&canonical), before);
    assert_eq!(
        wire_hit_info(&outcome),
        wow_packet::packets::combat::HIT_INFO_MISS
            | wow_packet::packets::combat::HIT_INFO_SWING_NO_HIT_SOUND
    );
    canonical
        .lock()
        .unwrap()
        .find_map_mut(0, 0)
        .unwrap()
        .map_mut()
        .with_creature_mut_like_cpp(victim_guid, |victim| {
            victim.set_in_evade_mode_like_cpp(false);
        })
        .unwrap();

    // The victim's flat 30% creature block (`Unit.h:947`) reduces the
    // mitigated 8 by `CalculatePct(8, 30) = 2`, and the packet carries
    // `HITINFO_BLOCK` plus the blocked amount.
    canonical
        .lock()
        .unwrap()
        .find_map_mut(0, 0)
        .unwrap()
        .map_mut()
        .with_creature_mut_like_cpp(victim_guid, |victim| {
            victim.set_avoidance_like_cpp(wow_entities::CreatureAvoidanceLikeCpp {
                block_pct: 100.0,
                ..Default::default()
            });
        })
        .unwrap();
    let before = victim_health(&canonical);
    let outcome = tick(&mut session);
    assert_eq!(outcome.canonical_creature_hits, 1);
    assert_eq!(before - victim_health(&canonical), 6);
    assert_eq!(
        wire_hit_info(&outcome) & wow_packet::packets::combat::HIT_INFO_BLOCK,
        wow_packet::packets::combat::HIT_INFO_BLOCK
    );
    assert_eq!(wire_blocked(&outcome), 2);

    // The victim's `SPELL_AURA_MOD_CRIT_CHANCE_VERSUS_TARGET_HEALTH` over the
    // whole health range makes the critical band certain: the mitigated 8
    // doubles to 16.
    canonical
        .lock()
        .unwrap()
        .find_map_mut(0, 0)
        .unwrap()
        .map_mut()
        .with_creature_mut_like_cpp(victim_guid, |victim| {
            victim.set_avoidance_like_cpp(wow_entities::CreatureAvoidanceLikeCpp::default());
        })
        .unwrap();
    apply_victim_aura(&canonical, 91_222_u32);
    let before = victim_health(&canonical);
    let outcome = tick(&mut session);
    assert_eq!(outcome.canonical_creature_hits, 1);
    assert_eq!(before - victim_health(&canonical), 16);

    // The `-200` sum makes the miss band cover the whole roll: no health
    // transition and the miss presentation on the wire.
    apply_victim_aura(&canonical, 91_221_u32);
    let before = victim_health(&canonical);
    let outcome = tick(&mut session);
    assert_eq!(outcome.melee_outcomes_unrepresented, 0);
    assert_eq!(outcome.canonical_creature_hits, 0, "a miss commits no hit");
    assert_eq!(victim_health(&canonical), before);
    assert!(outcome.plan.events.iter().any(|event| {
        event.packet_bytes.len() > 2
            && u16::from_le_bytes([event.packet_bytes[0], event.packet_bytes[1]])
                == wow_constants::ServerOpcodes::AttackerStateUpdate as u16
    }));
    assert_eq!(wire_hit_info(&outcome), HIT_INFO_MISS);

    // C++ `IsImmunedToDamage(SPELL_SCHOOL_MASK_NORMAL)` ends the swing before
    // every band (`Unit.cpp:1315-1324`), so a normal-school immunity aura on the
    // victim commits nothing and publishes the zero `HITINFO_NORMALSWING` with
    // `VICTIMSTATE_IS_IMMUNE`.
    apply_victim_aura(&canonical, 91_223_u32);
    let before = victim_health(&canonical);
    let outcome = tick(&mut session);
    assert_eq!(outcome.melee_outcomes_unrepresented, 0);
    assert_eq!(
        outcome.canonical_creature_hits, 0,
        "an immune swing commits no hit"
    );
    assert_eq!(victim_health(&canonical), before);
    assert_eq!(wire_hit_info(&outcome), 0, "HITINFO_NORMALSWING is 0x0");
    assert_eq!(
        wire_victim_state(&outcome),
        wow_packet::packets::combat::VICTIM_STATE_IS_IMMUNE
    );
}
