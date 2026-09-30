use super::*;

#[test]
fn legacy_creature_melee_tick_once_prevents_postmortem_cross_kill_like_cpp() {
    use wow_world::map_manager::RuntimeTickOwner;

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
            .fixture_melee_mutate_creature(attacker, |creature| {
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
