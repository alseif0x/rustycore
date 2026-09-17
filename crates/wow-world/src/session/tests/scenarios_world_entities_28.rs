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

    let outcome = run_legacy_creature_melee_tick_once_like_cpp(
        &manager,
        Some(&canonical),
        &Default::default(),
    );

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

    let outcome = run_legacy_creature_melee_tick_once_like_cpp(
        &manager,
        Some(&canonical),
        &Default::default(),
    );

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

    let outcome = run_legacy_creature_melee_tick_once_like_cpp(
        &manager,
        Some(&canonical),
        &Default::default(),
    );

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

    let rejected = run_legacy_creature_melee_tick_once_like_cpp(
        &manager,
        Some(&canonical),
        &Default::default(),
    );

    assert_eq!(rejected.swings_ready, 1);
    assert_eq!(rejected.melee_range_rejections, 1);
    assert_eq!(rejected.canonical_hits, 0);
    {
        let guard = manager.read().unwrap();
        let creature = guard.find_creature(0, 0, creature_guid).unwrap();
        assert_eq!(creature.creature.ai_ownership().swing_timer_ms, 100);
    }

    let immediate_retry = run_legacy_creature_melee_tick_once_like_cpp(
        &manager,
        Some(&canonical),
        &Default::default(),
    );

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

    let outcome = run_legacy_creature_melee_tick_once_like_cpp(
        &manager,
        Some(&canonical),
        &Default::default(),
    );

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

    let outcome = run_legacy_creature_melee_tick_once_like_cpp(
        &manager,
        Some(&canonical),
        &Default::default(),
    );

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

    let rejected = run_legacy_creature_melee_tick_once_like_cpp(
        &manager,
        Some(&canonical),
        &Default::default(),
    );

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

    let immediate_retry = run_legacy_creature_melee_tick_once_like_cpp(
        &manager,
        Some(&canonical),
        &Default::default(),
    );

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

    let outcome = run_legacy_creature_melee_tick_once_like_cpp(
        &manager,
        Some(&canonical),
        &Default::default(),
    );

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

/// C++ `Unit::RollMeleeOutcomeAgainst`'s miss, dodge, parry and crit bands for
/// a creature attacker against a player victim, through the production
/// ownership path.
///
/// C++ `MeleeSpellMissChance` (`Unit.cpp:11652-11685`) starts from the victim's
/// flat `GetUnitMissChance()` of `5.0` and subtracts the creature attacker's
/// zero `m_modMeleeHitChance` (`Unit.cpp:360`), its
/// `SPELL_AURA_MOD_HIT_CHANCE` sum and the victim's
/// `SPELL_AURA_MOD_ATTACKER_MELEE_HIT_CHANCE` sum; `GetUnitDodgeChance` and
/// `GetUnitParryChance` read the victim's published percentages and gate both
/// on `HasInArc(M_PI, attacker)`; the sitting-target rule returns a crit before
/// the avoidance bands (`Unit.cpp:2312-2314`). A missed or avoided swing
/// publishes zero dealt damage and no health transition.
#[test]
fn legacy_creature_melee_tick_once_resolves_player_victim_bands_like_cpp() {
    use crate::map_manager::RuntimeTickOwner;
    use wow_packet::packets::combat::{
        HIT_INFO_AFFECTS_VICTIM, HIT_INFO_CRITICAL_HIT, HIT_INFO_MISS, VICTIM_STATE_DODGE,
        VICTIM_STATE_HIT, VICTIM_STATE_INTACT, VICTIM_STATE_PARRY,
    };

    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    canonical.lock().unwrap().create_world_map(0, 0);

    let player = ObjectGuid::create_player(1, 91_140);
    let creature_guid = test_creature_guid(91_141);

    let (mut session, _, _) = make_session();
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
        "Victim".to_string(),
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
            player.unit_mut().set_max_health(100);
            player.unit_mut().set_health(100);
        })
        .unwrap();
    register_test_creature(&mut session, manager.clone(), creature_guid, 25);
    session
        .mutate_world_creature(creature_guid, |creature| {
            creature
                .creature
                .set_ai_position(Position::new(10.0, 10.0, 0.0, 0.0));
            creature.creature.unit_mut().set_combat_reach(0.0);
            creature.creature.ai_ownership_mut().min_damage = 10;
            creature.creature.ai_ownership_mut().max_damage = 10;
            // The first stages assert an exact landed damage, so the creature's
            // flat 5% critical chance is disabled until the crit stage opts in.
            creature.creature.set_flags_extra_runtime_like_cpp(
                wow_constants::CreatureFlagsExtra::NO_CRIT.bits(),
            );
            creature.enter_combat(player);
            creature.creature.ai_ownership_mut().last_swing_ms = 0;
            creature.creature.ai_ownership_mut().swing_timer_ms = 0;
        })
        .unwrap();

    let mut spell_store = wow_data::SpellStore::new();
    for (spell_id, aura_type, amount, misc_value_b) in [
        (
            91_150_i32,
            wow_data::spell::aura_types::SPELL_AURA_MOD_ATTACKER_MELEE_HIT_CHANCE,
            5_i32,
            0_i32,
        ),
        (
            91_151,
            wow_data::spell::aura_types::SPELL_AURA_MOD_ATTACKER_MELEE_HIT_CHANCE,
            -100,
            0,
        ),
        (
            91_152,
            wow_data::spell::aura_types::SPELL_AURA_MOD_CRIT_CHANCE_VERSUS_TARGET_HEALTH,
            100,
            // `!HealthBelowPct(50)` holds while the victim stays above half
            // health, which every stage keeps.
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
    let spell_store = Arc::new(spell_store);
    session.set_spell_store(Arc::clone(&spell_store));
    let config = crate::session::LegacyCreatureAggroConfigLikeCpp {
        spell_store: Some(Arc::clone(&spell_store)),
        ..Default::default()
    };
    manager
        .write()
        .unwrap()
        .set_tick_owner(RuntimeTickOwner::GlobalLegacy);

    let victim_health = |canonical: &SharedCanonicalMapManager| {
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
            .health
    };
    let reset_swing = |session: &mut WorldSession| {
        session
            .mutate_world_creature(creature_guid, |creature| {
                creature.creature.ai_ownership_mut().last_swing_ms = 0;
                creature.creature.ai_ownership_mut().swing_timer_ms = 0;
            })
            .unwrap();
    };
    // The fixture player's published parry is non-zero, so every stage sets the
    // avoidance it wants explicitly through the canonical victim's stats.
    let set_victim_avoidance = |session: &mut WorldSession, dodge: f32, parry: f32| {
        session
            .mutate_canonical_player_like_cpp(|player| {
                let mut stats = *player.effective_combat_stats_like_cpp();
                stats.dodge_pct = dodge;
                stats.parry_pct = parry;
                player.replace_effective_combat_stats_like_cpp(stats);
            })
            .expect("canonical victim");
    };
    set_victim_avoidance(&mut session, 0.0, 0.0);

    // `+5` cancels the flat 5.0, so the swing is guaranteed to land.
    session
        .apply_aura(91_150, player, 30_000, 1)
        .expect("apply hit-chance aura");
    let outcome = run_legacy_creature_melee_tick_once_like_cpp(&manager, Some(&canonical), &config);
    assert_eq!(outcome.swings_ready, 1);
    assert_eq!(outcome.melee_outcomes_unrepresented, 1);
    assert_eq!(outcome.canonical_hits, 1);
    assert_eq!(outcome.commands.len(), 1);
    assert_eq!(outcome.commands[0].damage, 10);
    assert_eq!(outcome.commands[0].original_damage, 10);
    assert_eq!(outcome.commands[0].hit_info, HIT_INFO_AFFECTS_VICTIM);
    assert_eq!(victim_health(&canonical), 90);

    // The `-100` sum makes the miss band cover the whole roll.
    session
        .apply_aura(91_151, player, 30_000, 1)
        .expect("apply miss aura");
    reset_swing(&mut session);
    let outcome = run_legacy_creature_melee_tick_once_like_cpp(&manager, Some(&canonical), &config);
    assert_eq!(outcome.swings_ready, 1);
    assert_eq!(
        outcome.melee_outcomes_unrepresented, 0,
        "the represented miss band resolves the swing"
    );
    assert_eq!(outcome.canonical_hits, 0, "a miss commits no hit");
    assert_eq!(outcome.commands.len(), 1);
    assert_eq!(outcome.commands[0].damage, 0);
    // C++ keeps the post-armour `OriginalDamage` on an avoided swing.
    assert_eq!(outcome.commands[0].original_damage, 10);
    assert_eq!(outcome.commands[0].hit_info, HIT_INFO_MISS);
    assert_eq!(outcome.commands[0].victim_state, VICTIM_STATE_INTACT);
    assert_eq!(victim_health(&canonical), 90, "a miss deals no damage");

    // The miss aura is removed again: the `+5` victim aura keeps the flat 5.0
    // band at zero, so only the avoidance/crit bands decide the later stages.
    let miss_slot = session
        .canonical_player_snapshot_like_cpp(|player| {
            player
                .unit()
                .subsystems()
                .auras
                .runtime_applications_like_cpp()
                .iter()
                .find(|(_, aura)| aura.spell_id == 91_151)
                .map(|(slot, _)| *slot)
        })
        .flatten()
        .expect("miss aura slot");
    session.remove_aura(miss_slot).expect("remove miss aura");
    let last_command = |outcome: &crate::session::LegacyCreatureMeleeTickOutcomeLikeCpp| {
        outcome.commands.last().expect("one swing command").clone()
    };

    // C++ `GetUnitDodgeChance` reads the victim's published `DodgePercentage`.
    set_victim_avoidance(&mut session, 100.0, 0.0);
    reset_swing(&mut session);
    let outcome = run_legacy_creature_melee_tick_once_like_cpp(&manager, Some(&canonical), &config);
    let command = last_command(&outcome);
    assert_eq!(outcome.canonical_hits, 0);
    assert_eq!(command.damage, 0);
    assert_eq!(command.original_damage, 10);
    assert_eq!(command.hit_info, HIT_INFO_AFFECTS_VICTIM);
    assert_eq!(command.victim_state, VICTIM_STATE_DODGE);
    assert_eq!(victim_health(&canonical), 90);

    // C++ `GetUnitParryChance` uses the published `ParryPercentage`.
    set_victim_avoidance(&mut session, 0.0, 100.0);
    reset_swing(&mut session);
    let outcome = run_legacy_creature_melee_tick_once_like_cpp(&manager, Some(&canonical), &config);
    let command = last_command(&outcome);
    assert_eq!(outcome.canonical_hits, 0);
    assert_eq!(command.damage, 0);
    assert_eq!(command.hit_info, HIT_INFO_AFFECTS_VICTIM);
    assert_eq!(command.victim_state, VICTIM_STATE_PARRY);
    assert_eq!(victim_health(&canonical), 90);

    // C++ returns `MELEE_HIT_CRIT` before the avoidance bands for a player
    // victim that is not in a stand state while the critical chance is
    // non-zero, so the creature's flat 5% is enough once the flag is cleared.
    set_victim_avoidance(&mut session, 100.0, 100.0);
    session
        .mutate_world_creature(creature_guid, |creature| {
            creature.creature.set_flags_extra_runtime_like_cpp(0);
        })
        .unwrap();
    session
        .mutate_canonical_player_like_cpp(|player| {
            player
                .unit_mut()
                .set_stand_state_like_cpp(wow_constants::UnitStandStateType::Sit);
        })
        .unwrap();
    reset_swing(&mut session);
    let outcome = run_legacy_creature_melee_tick_once_like_cpp(&manager, Some(&canonical), &config);
    let command = last_command(&outcome);
    assert_eq!(outcome.canonical_hits, 1);
    assert_eq!(command.damage, 20);
    // C++ assigns `OriginalDamage` after the critical doubling.
    assert_eq!(command.original_damage, 20);
    assert_eq!(
        command.hit_info,
        HIT_INFO_AFFECTS_VICTIM | HIT_INFO_CRITICAL_HIT
    );
    assert_eq!(command.victim_state, VICTIM_STATE_HIT);
    assert_eq!(victim_health(&canonical), 70);

    // Standing again, the victim's
    // `SPELL_AURA_MOD_CRIT_CHANCE_VERSUS_TARGET_HEALTH` aura over the whole
    // health range makes the critical band certain.
    session
        .mutate_canonical_player_like_cpp(|player| {
            player
                .unit_mut()
                .set_stand_state_like_cpp(wow_constants::UnitStandStateType::Stand);
        })
        .unwrap();
    // With the avoidance bands empty the critical band decides.
    set_victim_avoidance(&mut session, 0.0, 0.0);
    session
        .apply_aura(91_152, player, 30_000, 1)
        .expect("apply crit-vs-health aura");
    reset_swing(&mut session);
    let outcome = run_legacy_creature_melee_tick_once_like_cpp(&manager, Some(&canonical), &config);
    let command = last_command(&outcome);
    assert_eq!(outcome.canonical_hits, 1);
    assert_eq!(command.damage, 20);
    assert_eq!(command.original_damage, 20);
    assert_eq!(
        command.hit_info,
        HIT_INFO_AFFECTS_VICTIM | HIT_INFO_CRITICAL_HIT
    );
    assert_eq!(command.victim_state, VICTIM_STATE_HIT);
    assert_eq!(victim_health(&canonical), 50);
}

/// C++ `Unit::CalcArmorReducedDamage` for a creature attacker against a player
/// victim, through the production ownership path.
///
/// C++ `CalculateMeleeDamage` (`Unit.cpp:1326-1343`) mitigates through
/// `CalcArmorReducedDamage` before the outcome switch, reading the victim's
/// `GetArmor()` and the attacker's normal-school `MOD_TARGET_RESISTANCE`, while
/// the victim's `SPELL_AURA_BYPASS_ARMOR_FOR_CASTER` shrinks its own armour for
/// effects the attacker cast (`Unit.cpp:1631-1637`).
#[test]
fn legacy_creature_melee_tick_once_applies_player_victim_armor_like_cpp() {
    use crate::map_manager::RuntimeTickOwner;
    use wow_packet::packets::combat::HIT_INFO_AFFECTS_VICTIM;

    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    canonical.lock().unwrap().create_world_map(0, 0);

    let player = ObjectGuid::create_player(1, 91_160);
    let foreign_caster = ObjectGuid::create_player(1, 91_161);
    let creature_guid = test_creature_guid(91_162);

    let (mut session, _, _) = make_session();
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
        "ArmoredVictim".to_string(),
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
            player.unit_mut().set_max_health(100);
            player.unit_mut().set_health(100);
            let mut stats = *player.effective_combat_stats_like_cpp();
            stats.armor = 5_000;
            stats.dodge_pct = 0.0;
            stats.parry_pct = 0.0;
            player.replace_effective_combat_stats_like_cpp(stats);
        })
        .unwrap();
    register_test_creature(&mut session, manager.clone(), creature_guid, 25);
    session
        .mutate_world_creature(creature_guid, |creature| {
            creature
                .creature
                .set_ai_position(Position::new(10.0, 10.0, 0.0, 0.0));
            creature.creature.unit_mut().set_combat_reach(0.0);
            creature.creature.ai_ownership_mut().min_damage = 10;
            creature.creature.ai_ownership_mut().max_damage = 10;
            // C++ `CalcArmorReducedDamage`'s `levelModifier` is the attacker's
            // level, so the fixture creature is a level-80 attacker.
            creature.creature.unit_mut().set_level(80);
            // The fixture asserts exact mitigated damage, so the flat 5% crit
            // stays off.
            creature.creature.set_flags_extra_runtime_like_cpp(
                wow_constants::CreatureFlagsExtra::NO_CRIT.bits(),
            );
            creature.enter_combat(player);
            creature.creature.ai_ownership_mut().last_swing_ms = 0;
            creature.creature.ai_ownership_mut().swing_timer_ms = 0;
        })
        .unwrap();

    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        91_170,
        wow_data::SpellInfo {
            spell_id: 91_170,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
            effect_base_points: 5,
            effect_bonus_coefficient: 0.0,
            aura_type: Some(wow_data::spell::aura_types::SPELL_AURA_MOD_ATTACKER_MELEE_HIT_CHANCE),
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: vec![wow_data::SpellEffectInfo {
                effect_index: 0,
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                effect_aura: wow_data::spell::aura_types::SPELL_AURA_MOD_ATTACKER_MELEE_HIT_CHANCE,
                effect_base_points: 5,
                ..Default::default()
            }],
        },
    );
    spell_store.insert(
        91_171,
        wow_data::SpellInfo {
            spell_id: 91_171,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
            effect_base_points: 50,
            effect_bonus_coefficient: 0.0,
            aura_type: Some(wow_data::spell::aura_types::SPELL_AURA_BYPASS_ARMOR_FOR_CASTER),
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: vec![wow_data::SpellEffectInfo {
                effect_index: 0,
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                effect_aura: wow_data::spell::aura_types::SPELL_AURA_BYPASS_ARMOR_FOR_CASTER,
                effect_base_points: 50,
                ..Default::default()
            }],
        },
    );
    let spell_store = Arc::new(spell_store);
    session.set_spell_store(Arc::clone(&spell_store));
    let config = crate::session::LegacyCreatureAggroConfigLikeCpp {
        spell_store: Some(spell_store),
        ..Default::default()
    };
    manager
        .write()
        .unwrap()
        .set_tick_owner(RuntimeTickOwner::GlobalLegacy);

    let victim_health = |canonical: &SharedCanonicalMapManager| {
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
            .health
    };
    let reset_swing = |session: &mut WorldSession| {
        session
            .mutate_world_creature(creature_guid, |creature| {
                creature.creature.ai_ownership_mut().last_swing_ms = 0;
                creature.creature.ai_ownership_mut().swing_timer_ms = 0;
            })
            .unwrap();
    };
    let swing = |manager: &crate::map_manager::SharedMapManager, session: &mut WorldSession| {
        reset_swing(session);
        let outcome =
            run_legacy_creature_melee_tick_once_like_cpp(manager, Some(&canonical), &config);
        outcome.commands.last().expect("one swing command").clone()
    };

    // The `+5` victim aura zeroes the flat 5.0 miss band.
    session
        .apply_aura(91_170, player, 30_000, 1)
        .expect("apply hit-chance aura");

    // 5,000 armour at level 80: `ceil(10 * (1 - 0.247127)) = 8`.
    let command = swing(&manager, &mut session);
    assert_eq!(command.damage, 8);
    assert_eq!(command.original_damage, 8);
    assert_eq!(command.hit_info, HIT_INFO_AFFECTS_VICTIM);
    assert_eq!(victim_health(&canonical), 92);

    // A foreign caster's bypass aura is ignored (`GetCasterGUID() == attacker`).
    session
        .apply_aura(91_171, foreign_caster, 30_000, 1)
        .expect("apply foreign bypass aura");
    let command = swing(&manager, &mut session);
    assert_eq!(command.damage, 8);
    assert_eq!(victim_health(&canonical), 84);

    // The attacker's own 50% bypass halves the armour: `ceil(10 * 0.859017) = 9`.
    session
        .apply_aura(91_171, creature_guid, 30_000, 1)
        .expect("apply attacker bypass aura");
    let command = swing(&manager, &mut session);
    assert_eq!(command.damage, 9);
    assert_eq!(victim_health(&canonical), 75);

    // Both bypass effects sum, removing the armour: the full 10 lands.
    let second_bypass = session
        .canonical_player_snapshot_like_cpp(|player| {
            player
                .unit()
                .subsystems()
                .auras
                .runtime_applications_like_cpp()
                .iter()
                .filter(|(_, aura)| aura.spell_id == 91_171 && aura.caster_guid == foreign_caster)
                .map(|(slot, _)| *slot)
                .next()
        })
        .flatten()
        .expect("foreign bypass slot");
    session
        .remove_aura(second_bypass)
        .expect("remove foreign bypass aura");
    session
        .apply_aura(91_171, creature_guid, 30_000, 1)
        .expect("apply second attacker bypass aura");
    let command = swing(&manager, &mut session);
    assert_eq!(command.damage, 10);
    assert_eq!(command.original_damage, 10);
    assert_eq!(victim_health(&canonical), 65);
}

/// C++ `Unit::MeleeDamageBonusTaken` for a creature attacker against a player
/// victim, through the production ownership path.
///
/// C++ `CalculateMeleeDamage` (`Unit.cpp:1326-1343`) runs the victim's
/// `MeleeDamageBonusTaken` before `CalcArmorReducedDamage`. For a white swing
/// the chain is the flat `SPELL_AURA_MOD_MELEE_DAMAGE_TAKEN` benefit, the
/// school-masked `SPELL_AURA_MOD_DAMAGE_PERCENT_TAKEN` and
/// `SPELL_AURA_MOD_MELEE_DAMAGE_TAKEN_PCT` multipliers, and the Sanctified
/// Wrath bypass that the attacker's `SPELL_AURA_MOD_IGNORE_TARGET_RESIST`
/// shrinks (`Unit.cpp:7670-7778`).
#[test]
fn legacy_creature_melee_tick_once_applies_player_victim_taken_like_cpp() {
    use crate::map_manager::RuntimeTickOwner;
    use wow_packet::packets::combat::HIT_INFO_AFFECTS_VICTIM;

    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    canonical.lock().unwrap().create_world_map(0, 0);

    let player = ObjectGuid::create_player(1, 91_180);
    let creature_guid = test_creature_guid(91_181);

    let (mut session, _, _) = make_session();
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
        "TakenVictim".to_string(),
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
            player.unit_mut().set_max_health(100);
            player.unit_mut().set_health(100);
            let mut stats = *player.effective_combat_stats_like_cpp();
            stats.armor = 5_000;
            stats.dodge_pct = 0.0;
            stats.parry_pct = 0.0;
            player.replace_effective_combat_stats_like_cpp(stats);
        })
        .unwrap();
    register_test_creature(&mut session, manager.clone(), creature_guid, 25);
    session
        .mutate_world_creature(creature_guid, |creature| {
            creature
                .creature
                .set_ai_position(Position::new(10.0, 10.0, 0.0, 0.0));
            creature.creature.unit_mut().set_combat_reach(0.0);
            creature.creature.unit_mut().set_level(80);
            creature.creature.ai_ownership_mut().min_damage = 10;
            creature.creature.ai_ownership_mut().max_damage = 10;
            creature.creature.set_flags_extra_runtime_like_cpp(
                wow_constants::CreatureFlagsExtra::NO_CRIT.bits(),
            );
            creature.enter_combat(player);
            creature.creature.ai_ownership_mut().last_swing_ms = 0;
            creature.creature.ai_ownership_mut().swing_timer_ms = 0;
        })
        .unwrap();

    let mut spell_store = wow_data::SpellStore::new();
    for (spell_id, aura_type, amount, misc_value) in [
        (
            91_190_i32,
            wow_data::spell::aura_types::SPELL_AURA_MOD_ATTACKER_MELEE_HIT_CHANCE,
            5_i32,
            0_i32,
        ),
        (
            91_191,
            wow_data::spell::aura_types::SPELL_AURA_MOD_MELEE_DAMAGE_TAKEN,
            5,
            0,
        ),
        (
            91_192,
            wow_data::spell::aura_types::SPELL_AURA_MOD_DAMAGE_PERCENT_TAKEN,
            100,
            // Fire school: the normal-school chain ignores it.
            0x04,
        ),
        (
            91_193,
            wow_data::spell::aura_types::SPELL_AURA_MOD_DAMAGE_PERCENT_TAKEN,
            100,
            // Normal school.
            0x01,
        ),
        (
            91_194,
            wow_data::spell::aura_types::SPELL_AURA_MOD_DAMAGE_PERCENT_TAKEN,
            -50,
            0x01,
        ),
        (
            91_195,
            wow_data::spell::aura_types::SPELL_AURA_MOD_IGNORE_TARGET_RESIST,
            50,
            0x01,
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
                    effect_misc_value_1: misc_value,
                    effect_base_points: amount,
                    ..Default::default()
                }],
            },
        );
    }
    let spell_store = Arc::new(spell_store);
    session.set_spell_store(Arc::clone(&spell_store));
    let config = crate::session::LegacyCreatureAggroConfigLikeCpp {
        spell_store: Some(spell_store),
        ..Default::default()
    };
    manager
        .write()
        .unwrap()
        .set_tick_owner(RuntimeTickOwner::GlobalLegacy);

    let victim_health = |canonical: &SharedCanonicalMapManager| {
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
            .health
    };
    let swing = |manager: &crate::map_manager::SharedMapManager, session: &mut WorldSession| {
        session
            .mutate_world_creature(creature_guid, |creature| {
                creature.creature.ai_ownership_mut().last_swing_ms = 0;
                creature.creature.ai_ownership_mut().swing_timer_ms = 0;
            })
            .unwrap();
        let outcome =
            run_legacy_creature_melee_tick_once_like_cpp(manager, Some(&canonical), &config);
        outcome.commands.last().expect("one swing command").clone()
    };
    let aura_slot = |session: &WorldSession, spell_id: i32| {
        session
            .canonical_player_snapshot_like_cpp(|player| {
                player
                    .unit()
                    .subsystems()
                    .auras
                    .runtime_applications_like_cpp()
                    .iter()
                    .find(|(_, aura)| aura.spell_id == spell_id)
                    .map(|(slot, _)| *slot)
            })
            .flatten()
            .expect("applied aura slot")
    };

    // The `+5` victim aura zeroes the flat 5.0 miss band.
    session
        .apply_aura(91_190, player, 30_000, 1)
        .expect("apply hit-chance aura");

    // Baseline: 5,000 armour at level 80 -> 8.
    let command = swing(&manager, &mut session);
    assert_eq!(command.damage, 8);
    assert_eq!(command.hit_info, HIT_INFO_AFFECTS_VICTIM);
    assert_eq!(victim_health(&canonical), 92);

    // The flat `MOD_MELEE_DAMAGE_TAKEN` benefit: `(10 + 5)` -> armour -> 12.
    session
        .apply_aura(91_191, player, 30_000, 1)
        .expect("apply flat taken aura");
    let command = swing(&manager, &mut session);
    assert_eq!(command.damage, 12);
    assert_eq!(victim_health(&canonical), 80);

    // A non-normal `MOD_DAMAGE_PERCENT_TAKEN` row is filtered out.
    session
        .apply_aura(91_192, player, 30_000, 1)
        .expect("apply fire taken aura");
    let command = swing(&manager, &mut session);
    assert_eq!(command.damage, 12);
    assert_eq!(victim_health(&canonical), 68);

    // The normal-school row doubles `(10 + 5)` -> armour -> 23.
    session
        .apply_aura(91_193, player, 30_000, 1)
        .expect("apply normal taken aura");
    let command = swing(&manager, &mut session);
    assert_eq!(command.damage, 23);
    assert_eq!(victim_health(&canonical), 45);

    // Sanctified Wrath: with the victim's total modifier below one, the
    // attacker's `SPELL_AURA_MOD_IGNORE_TARGET_RESIST` shrinks the reduction
    // (`0.5 * (1 - 0.5) = 0.25`), and the same aura also halves the armour.
    session
        .remove_aura(aura_slot(&session, 91_193))
        .expect("remove normal taken aura");
    session
        .apply_aura(91_194, player, 30_000, 1)
        .expect("apply reduction aura");
    session
        .mutate_world_creature(creature_guid, |creature| {
            creature
                .creature
                .unit_mut()
                .subsystems_mut()
                .auras
                .add_applied(wow_entities::AppliedAuraRef::new(91_195, player, 0, 1));
        })
        .unwrap();
    let command = swing(&manager, &mut session);
    assert_eq!(command.damage, 10);
    assert_eq!(victim_health(&canonical), 35);
}
