//! Session scenarios exercising the represented world entities responsibility.
//!
//! Split out of session_tests.rs under #626; assertions and registrations
//! are unchanged and the shared fixtures stay in the parent module.

use super::*;

#[test]
fn legacy_creature_melee_tick_once_rejects_same_guid_attacker_replacement_like_cpp() {
    use crate::map_manager::RuntimeTickOwner;

    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    let player = ObjectGuid::create_player(1, 91_107);
    add_canonical_test_player_on_map(
        &canonical,
        player,
        Position::new(10.0, 10.0, 0.0, 0.0),
        0,
        0,
    );
    {
        let mut guard = canonical.lock().unwrap();
        let player = guard
            .find_map_mut(0, 0)
            .unwrap()
            .map_mut()
            .get_typed_player_mut(player)
            .unwrap();
        player.unit_mut().set_max_health(100);
        player.unit_mut().set_health(100);
    }

    let (mut session, _, _) = make_session();
    session.set_canonical_map_manager(Arc::clone(&canonical));
    let creature_guid = test_creature_guid(91_108);
    let seed = 0x91_108;
    register_test_creature(&mut session, Arc::clone(&manager), creature_guid, 25);
    session
        .mutate_world_creature(creature_guid, |creature| {
            creature.enter_combat(player);
            creature.creature.ai_ownership_mut().last_swing_ms = 0;
            creature.creature.ai_ownership_mut().swing_timer_ms = 0;
            creature.seed_runtime_rng_like_cpp(seed);
        })
        .unwrap();
    let legacy_health_authority = manager
        .read()
        .unwrap()
        .find_creature(0, 0, creature_guid)
        .unwrap()
        .creature
        .unit()
        .health_state_revision_authority_like_cpp();

    let mut replacement = crate::map_manager::WorldCreature::new(
        creature_guid,
        9001,
        Position::new(10.0, 10.0, 0.0, 0.0),
        25,
        80,
        3,
        5,
        20.0,
        1,
        35,
        0,
        0,
    )
    .creature;
    replacement.unit_mut().world_mut().set_map(0, 0).unwrap();
    replacement
        .unit_mut()
        .world_mut()
        .object_mut()
        .add_to_world();
    assert!(
        !replacement
            .unit()
            .shares_health_state_revision_authority_like_cpp(&legacy_health_authority)
    );
    canonical
        .lock()
        .unwrap()
        .find_map_mut(0, 0)
        .unwrap()
        .map_mut()
        .insert_map_object_record(wow_entities::MapObjectRecord::new_creature(replacement).unwrap())
        .unwrap();
    manager
        .write()
        .unwrap()
        .set_tick_owner(RuntimeTickOwner::GlobalLegacy);

    let outcome = run_legacy_creature_melee_tick_once_like_cpp(&manager, Some(&canonical));

    assert_eq!(outcome.swings_ready, 1);
    assert_eq!(outcome.attacker_incarnation_rejections, 1);
    assert_eq!(outcome.melee_precondition_rejections, 1);
    assert_eq!(outcome.melee_outcomes_unrepresented, 0);
    assert_eq!(outcome.canonical_hits, 0);
    assert!(outcome.commands.is_empty());
    assert_eq!(
        canonical
            .lock()
            .unwrap()
            .find_map(0, 0)
            .unwrap()
            .map()
            .get_typed_player(player)
            .unwrap()
            .unit()
            .data()
            .health,
        100
    );
    let mut expected_rng = StdRng::seed_from_u64(seed);
    let expected_first_roll = expected_rng.gen_range(0..=9_999_u32);
    let (timer, exact_rng, actual_first_roll) = session
        .mutate_world_creature(creature_guid, |creature| {
            (
                creature.creature.ai_ownership().swing_timer_ms,
                creature.runtime_rng_authority_complete_like_cpp(),
                creature.random_creature_spell_hit_roll_like_cpp(),
            )
        })
        .unwrap();
    assert_eq!(timer, 0);
    assert!(exact_rng);
    assert_eq!(actual_first_roll, Some(expected_first_roll));
}
#[test]
fn legacy_creature_melee_tick_once_rejects_out_of_range_victim_like_cpp() {
    use crate::map_manager::RuntimeTickOwner;
    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    let player = ObjectGuid::create_player(1, 91_005);
    add_canonical_test_player_on_map(
        &canonical,
        player,
        Position::new(100.0, 100.0, 0.0, 0.0),
        0,
        0,
    );
    {
        let mut guard = canonical.lock().unwrap();
        let typed = guard
            .find_map_mut(0, 0)
            .unwrap()
            .map_mut()
            .get_typed_player_mut(player)
            .unwrap();
        typed.unit_mut().set_max_health(100);
        typed.unit_mut().set_health(100);
        typed.unit_mut().set_combat_reach(0.0);
    }

    let (mut session, _, _) = make_session();
    session.set_canonical_map_manager(Arc::clone(&canonical));
    let creature_guid = test_creature_guid(91_006);
    register_test_creature(&mut session, manager.clone(), creature_guid, 25);
    let seed = 0x91_006;
    session
        .mutate_world_creature(creature_guid, |creature| {
            creature
                .creature
                .set_ai_position(Position::new(10.0, 10.0, 0.0, 0.0));
            creature.creature.unit_mut().set_combat_reach(0.0);
            creature.enter_combat(player);
            creature.creature.ai_ownership_mut().swing_timer_ms = 0;
            creature.seed_runtime_rng_like_cpp(seed);
        })
        .unwrap();
    manager
        .write()
        .unwrap()
        .set_tick_owner(RuntimeTickOwner::GlobalLegacy);

    let outcome = run_legacy_creature_melee_tick_once_like_cpp(&manager, Some(&canonical));

    assert!(!outcome.skipped_owner_not_global);
    assert_eq!(outcome.maps_seen, 1);
    assert_eq!(outcome.creatures_seen, 1);
    assert_eq!(outcome.swings_ready, 1);
    assert_eq!(outcome.melee_range_rejections, 1);
    assert_eq!(outcome.melee_outcomes_unrepresented, 0);
    assert_eq!(outcome.canonical_hits, 0);
    assert!(outcome.commands.is_empty());

    let health = canonical
        .lock()
        .unwrap()
        .find_map(0, 0)
        .unwrap()
        .map()
        .get_typed_player(player)
        .unwrap()
        .unit()
        .data()
        .health;
    assert_eq!(health, 100);
    let mut expected_rng = StdRng::seed_from_u64(seed);
    let expected_first_roll = expected_rng.gen_range(0..=9_999_u32);
    let actual_first_roll = session
        .mutate_world_creature(creature_guid, |creature| {
            assert!(
                creature.runtime_rng_authority_complete_like_cpp(),
                "C++ rejects range before entering its melee RNG surface"
            );
            creature.random_creature_spell_hit_roll_like_cpp()
        })
        .unwrap();
    assert_eq!(actual_first_roll, Some(expected_first_roll));
}
#[test]
fn legacy_creature_melee_tick_once_rejects_bad_facing_victim_like_cpp() {
    use crate::map_manager::RuntimeTickOwner;
    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    let player = ObjectGuid::create_player(1, 91_007);
    add_canonical_test_player_on_map(&canonical, player, Position::new(6.0, 10.0, 0.0, 0.0), 0, 0);
    {
        let mut guard = canonical.lock().unwrap();
        let typed = guard
            .find_map_mut(0, 0)
            .unwrap()
            .map_mut()
            .get_typed_player_mut(player)
            .unwrap();
        typed.unit_mut().set_max_health(100);
        typed.unit_mut().set_health(100);
        typed.unit_mut().set_combat_reach(0.0);
        typed.unit_mut().set_bounding_radius(0.0);
    }

    let (mut session, _, _) = make_session();
    session.set_canonical_map_manager(Arc::clone(&canonical));
    let creature_guid = test_creature_guid(91_008);
    register_test_creature(&mut session, manager.clone(), creature_guid, 25);
    session
        .mutate_world_creature(creature_guid, |creature| {
            creature
                .creature
                .set_ai_position(Position::new(10.0, 10.0, 0.0, 0.0));
            creature.creature.unit_mut().set_combat_reach(0.0);
            creature.enter_combat(player);
            creature.creature.ai_ownership_mut().swing_timer_ms = 0;
        })
        .unwrap();
    manager
        .write()
        .unwrap()
        .set_tick_owner(RuntimeTickOwner::GlobalLegacy);

    let outcome = run_legacy_creature_melee_tick_once_like_cpp(&manager, Some(&canonical));

    assert!(!outcome.skipped_owner_not_global);
    assert_eq!(outcome.maps_seen, 1);
    assert_eq!(outcome.creatures_seen, 1);
    assert_eq!(outcome.swings_ready, 1);
    assert_eq!(outcome.melee_range_rejections, 0);
    assert_eq!(outcome.melee_facing_rejections, 1);
    assert_eq!(outcome.melee_outcomes_unrepresented, 0);
    assert_eq!(outcome.canonical_hits, 0);
    assert!(outcome.commands.is_empty());

    let health = canonical
        .lock()
        .unwrap()
        .find_map(0, 0)
        .unwrap()
        .map()
        .get_typed_player(player)
        .unwrap()
        .unit()
        .data()
        .health;
    assert_eq!(health, 100);
    assert!(
        session
            .mutate_world_creature(creature_guid, |creature| {
                creature.runtime_rng_authority_complete_like_cpp()
            })
            .unwrap(),
        "C++ rejects facing before entering its melee RNG surface"
    );
}
#[test]
fn legacy_creature_melee_tick_once_failed_auto_attack_uses_retry_timer_like_cpp() {
    use crate::map_manager::RuntimeTickOwner;
    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    let player = ObjectGuid::create_player(1, 91_009);
    add_canonical_test_player_on_map(
        &canonical,
        player,
        Position::new(100.0, 100.0, 0.0, 0.0),
        0,
        0,
    );

    let (mut session, _, _) = make_session();
    session.set_canonical_map_manager(Arc::clone(&canonical));
    let creature_guid = test_creature_guid(91_010);
    register_test_creature(&mut session, manager.clone(), creature_guid, 25);
    session
        .mutate_world_creature(creature_guid, |creature| {
            creature.enter_combat(player);
            creature.creature.ai_ownership_mut().last_swing_ms = 0;
            creature.creature.ai_ownership_mut().swing_timer_ms = 0;
        })
        .unwrap();
    manager
        .write()
        .unwrap()
        .set_tick_owner(RuntimeTickOwner::GlobalLegacy);

    let rejected = run_legacy_creature_melee_tick_once_like_cpp(&manager, Some(&canonical));

    assert_eq!(rejected.swings_ready, 1);
    assert_eq!(rejected.melee_range_rejections, 1);
    assert_eq!(rejected.canonical_hits, 0);
    {
        let guard = manager.read().unwrap();
        let creature = guard.find_creature(0, 0, creature_guid).unwrap();
        assert_eq!(creature.creature.ai_ownership().swing_timer_ms, 100);
    }

    let immediate_retry = run_legacy_creature_melee_tick_once_like_cpp(&manager, Some(&canonical));

    assert_eq!(immediate_retry.swings_ready, 0);
    assert_eq!(immediate_retry.melee_range_rejections, 0);
    assert_eq!(immediate_retry.canonical_hits, 0);
    assert!(immediate_retry.commands.is_empty());
}
#[test]
fn legacy_creature_melee_tick_once_invalid_damage_roll_rearms_base_timer_like_cpp() {
    use crate::map_manager::RuntimeTickOwner;
    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    let player = ObjectGuid::create_player(1, 91_101);
    add_canonical_test_player_on_map(
        &canonical,
        player,
        Position::new(10.0, 10.0, 0.0, 0.0),
        0,
        0,
    );
    {
        let mut guard = canonical.lock().unwrap();
        let player = guard
            .find_map_mut(0, 0)
            .unwrap()
            .map_mut()
            .get_typed_player_mut(player)
            .unwrap();
        player.unit_mut().set_max_health(100);
        player.unit_mut().set_health(100);
    }

    let (mut session, _, _) = make_session();
    session.set_canonical_map_manager(Arc::clone(&canonical));
    let creature_guid = test_creature_guid(91_102);
    add_canonical_test_creature_on_map(
        &canonical,
        creature_guid,
        9001,
        Position::new(10.0, 10.0, 0.0, 0.0),
        0,
        0,
        0,
    );
    register_test_creature(&mut session, manager.clone(), creature_guid, 25);
    session
        .mutate_world_creature(creature_guid, |creature| {
            creature.enter_combat(player);
            creature.creature.ai_ownership_mut().last_swing_ms = 0;
            creature.creature.ai_ownership_mut().swing_timer_ms = 0;
            creature.creature.ai_ownership_mut().min_damage = 10;
            creature.creature.ai_ownership_mut().max_damage = 5;
        })
        .unwrap();
    manager
        .write()
        .unwrap()
        .set_tick_owner(RuntimeTickOwner::GlobalLegacy);

    let outcome = run_legacy_creature_melee_tick_once_like_cpp(&manager, Some(&canonical));

    assert_eq!(outcome.swings_ready, 1);
    assert_eq!(outcome.melee_precondition_rejections, 1);
    assert_eq!(outcome.melee_outcomes_unrepresented, 0);
    assert_eq!(outcome.canonical_hits, 0);
    assert!(outcome.commands.is_empty());
    let guard = manager.read().unwrap();
    let creature = guard.find_creature(0, 0, creature_guid).unwrap();
    assert_eq!(creature.creature.ai_ownership().swing_timer_ms, 2_000);
    assert!(
        !creature.runtime_rng_authority_complete_like_cpp(),
        "invalid damage bounds cannot preserve an exact C++ RNG position"
    );
}
#[test]
fn legacy_creature_melee_tick_once_checks_range_before_dead_victim_like_cpp() {
    use crate::map_manager::RuntimeTickOwner;
    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    let player = ObjectGuid::create_player(1, 91_103);
    add_canonical_test_player_on_map(
        &canonical,
        player,
        Position::new(100.0, 100.0, 0.0, 0.0),
        0,
        0,
    );
    {
        let mut guard = canonical.lock().unwrap();
        let player = guard
            .find_map_mut(0, 0)
            .unwrap()
            .map_mut()
            .get_typed_player_mut(player)
            .unwrap();
        player.unit_mut().set_max_health(100);
        player.unit_mut().set_health(0);
        player
            .unit_mut()
            .set_death_state(wow_constants::DeathState::JustDied);
    }

    let (mut session, _, _) = make_session();
    session.set_canonical_map_manager(Arc::clone(&canonical));
    let creature_guid = test_creature_guid(91_104);
    add_canonical_test_creature_on_map(
        &canonical,
        creature_guid,
        9001,
        Position::new(10.0, 10.0, 0.0, 0.0),
        0,
        0,
        0,
    );
    register_test_creature(&mut session, manager.clone(), creature_guid, 25);
    session
        .mutate_world_creature(creature_guid, |creature| {
            creature.enter_combat(player);
            creature.creature.ai_ownership_mut().last_swing_ms = 0;
            creature.creature.ai_ownership_mut().swing_timer_ms = 0;
        })
        .unwrap();
    manager
        .write()
        .unwrap()
        .set_tick_owner(RuntimeTickOwner::GlobalLegacy);

    let outcome = run_legacy_creature_melee_tick_once_like_cpp(&manager, Some(&canonical));

    assert_eq!(outcome.swings_ready, 1);
    assert_eq!(outcome.melee_range_rejections, 1);
    assert_eq!(outcome.melee_precondition_rejections, 0);
    assert_eq!(outcome.canonical_hits, 0);
    assert!(outcome.commands.is_empty());
    let guard = manager.read().unwrap();
    let creature = guard.find_creature(0, 0, creature_guid).unwrap();
    assert_eq!(creature.creature.ai_ownership().swing_timer_ms, 100);
}
#[test]
fn legacy_creature_melee_tick_once_attacker_state_rejection_consumes_swing_like_cpp() {
    use crate::map_manager::RuntimeTickOwner;
    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    let player = ObjectGuid::create_player(1, 91_011);
    add_canonical_test_player_on_map(
        &canonical,
        player,
        Position::new(10.0, 10.0, 0.0, 0.0),
        0,
        0,
    );
    {
        let mut guard = canonical.lock().unwrap();
        let typed = guard
            .find_map_mut(0, 0)
            .unwrap()
            .map_mut()
            .get_typed_player_mut(player)
            .unwrap();
        typed.unit_mut().set_max_health(100);
        typed.unit_mut().set_health(100);
    }

    let (mut session, _, _) = make_session();
    session.set_canonical_map_manager(Arc::clone(&canonical));
    let creature_guid = test_creature_guid(91_012);
    register_test_creature(&mut session, manager.clone(), creature_guid, 25);
    session
        .mutate_world_creature(creature_guid, |creature| {
            creature.enter_combat(player);
            creature.creature.ai_ownership_mut().last_swing_ms = 0;
            creature.creature.ai_ownership_mut().swing_timer_ms = 0;
            creature
                .creature
                .unit_mut()
                .set_unit_flags_like_cpp(UnitFlags::PACIFIED);
        })
        .unwrap();
    manager
        .write()
        .unwrap()
        .set_tick_owner(RuntimeTickOwner::GlobalLegacy);

    let rejected = run_legacy_creature_melee_tick_once_like_cpp(&manager, Some(&canonical));

    assert_eq!(rejected.swings_ready, 1);
    assert_eq!(rejected.melee_range_rejections, 0);
    assert_eq!(rejected.melee_facing_rejections, 0);
    assert_eq!(rejected.attacker_state_rejections, 1);
    assert_eq!(rejected.canonical_hits, 0);
    assert!(rejected.commands.is_empty());
    let health = canonical
        .lock()
        .unwrap()
        .find_map(0, 0)
        .unwrap()
        .map()
        .get_typed_player(player)
        .unwrap()
        .unit()
        .data()
        .health;
    assert_eq!(health, 100);
    {
        let guard = manager.read().unwrap();
        let creature = guard.find_creature(0, 0, creature_guid).unwrap();
        assert_eq!(
            creature.creature.ai_ownership().swing_timer_ms,
            2_000,
            "C++ resets the base attack timer after AttackerStateUpdate returns early"
        );
    }

    let immediate_retry = run_legacy_creature_melee_tick_once_like_cpp(&manager, Some(&canonical));

    assert_eq!(immediate_retry.swings_ready, 0);
    assert_eq!(immediate_retry.attacker_state_rejections, 0);
    assert_eq!(immediate_retry.canonical_hits, 0);
    assert!(immediate_retry.commands.is_empty());
}
#[test]
fn legacy_creature_melee_tick_once_removes_attacking_auras_on_compatibility_hit_like_cpp() {
    use crate::map_manager::RuntimeTickOwner;
    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    let player = ObjectGuid::create_player(1, 91_022);
    add_canonical_test_player_on_map(
        &canonical,
        player,
        Position::new(10.0, 10.0, 0.0, 0.0),
        0,
        0,
    );
    {
        let mut guard = canonical.lock().unwrap();
        let typed = guard
            .find_map_mut(0, 0)
            .unwrap()
            .map_mut()
            .get_typed_player_mut(player)
            .unwrap();
        typed.unit_mut().set_max_health(100);
        typed.unit_mut().set_health(100);
    }

    let (mut session, _, _) = make_session();
    session.set_canonical_map_manager(Arc::clone(&canonical));
    let creature_guid = test_creature_guid(91_023);
    let removed_by_attacking = wow_entities::AppliedAuraRef::new(91_024, creature_guid, 0, 0x1);
    let kept = wow_entities::AppliedAuraRef::new(91_025, creature_guid, 0, 0x2);
    add_canonical_test_creature_on_map(
        &canonical,
        creature_guid,
        9001,
        Position::new(10.0, 10.0, 0.0, 0.0),
        0,
        0,
        0,
    );
    register_test_creature(&mut session, manager.clone(), creature_guid, 25);
    session
        .mutate_world_creature(creature_guid, |creature| {
            creature.enter_combat(player);
            creature.creature.ai_ownership_mut().last_swing_ms = 0;
            creature.creature.ai_ownership_mut().swing_timer_ms = 0;
            creature
                .creature
                .unit_mut()
                .subsystems_mut()
                .auras
                .register_applied_aura(
                    removed_by_attacking,
                    None,
                    wow_entities::SPELL_AURA_INTERRUPT_FLAG_ATTACKING_LIKE_CPP,
                    0,
                );
            creature
                .creature
                .unit_mut()
                .subsystems_mut()
                .auras
                .register_applied_aura(kept, None, 0x20, 0);
        })
        .unwrap();
    manager
        .write()
        .unwrap()
        .set_tick_owner(RuntimeTickOwner::GlobalLegacy);

    let outcome = run_legacy_creature_melee_tick_once_like_cpp(&manager, Some(&canonical));

    assert_eq!(outcome.swings_ready, 1);
    assert_eq!(outcome.melee_outcomes_unrepresented, 1);
    assert_eq!(outcome.canonical_hits, 1);
    assert_eq!(outcome.attacking_interrupt_auras_removed, 1);
    assert_eq!(outcome.commands.len(), 1);
    assert!(outcome.plan.events.is_empty());
    let guard = manager.read().unwrap();
    let creature = guard.find_creature(0, 0, creature_guid).unwrap();
    assert!(!creature.runtime_rng_authority_complete_like_cpp());
    assert!(
        !creature
            .creature
            .unit()
            .subsystems()
            .auras
            .has_applied(removed_by_attacking),
        "C++ Unit::AttackerStateUpdate removes SpellAuraInterruptFlags::Attacking"
    );
    assert!(
        creature
            .creature
            .unit()
            .subsystems()
            .auras
            .has_applied(kept)
    );
}
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

    let outcome = run_legacy_creature_melee_tick_once_like_cpp(&manager, Some(&canonical));

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

    let outcome = run_legacy_creature_melee_tick_once_like_cpp(&manager, Some(&canonical));

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
