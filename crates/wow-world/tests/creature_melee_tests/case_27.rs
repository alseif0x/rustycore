use super::*;

#[test]
fn legacy_creature_melee_tick_once_preserves_lethal_creature_outcome_like_cpp() {
    use wow_world::map_manager::RuntimeTickOwner;
    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    let victim = test_creature_guid(91_034);
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
        typed.unit_mut().set_health(1);
    }

    let (mut session, _, _) = make_session();
    session.set_canonical_map_manager(Arc::clone(&canonical));
    let attacker = test_creature_guid(91_035);
    add_canonical_test_creature_on_map(
        &canonical,
        attacker,
        9001,
        Position::new(10.0, 10.0, 0.0, 0.0),
        0,
        0,
        0,
    );
    register_test_creature(&mut session, manager.clone(), victim, 1);
    register_test_creature(&mut session, manager.clone(), attacker, 25);
    session
        .fixture_melee_mutate_creature(attacker, |creature| {
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
    assert_eq!(outcome.canonical_creature_hits, 1);
    assert_eq!(outcome.legacy_creature_victim_syncs, 1);
    assert_eq!(outcome.plan.events.len(), 2);
    let guard = canonical.lock().unwrap();
    let creature = guard
        .find_map(0, 0)
        .unwrap()
        .map()
        .with_creature_like_cpp(victim, Clone::clone)
        .unwrap();
    assert_eq!(creature.unit().data().health, 0);
    assert_eq!(
        creature.unit().death_state(),
        wow_constants::DeathState::Corpse
    );
    drop(guard);
    let legacy_guard = manager.read().unwrap();
    let legacy_creature = legacy_guard.find_creature(0, 0, victim).unwrap();
    assert_eq!(legacy_creature.creature.unit().data().health, 0);
    assert_eq!(
        legacy_creature.creature.unit().death_state(),
        wow_constants::DeathState::Corpse
    );
}
