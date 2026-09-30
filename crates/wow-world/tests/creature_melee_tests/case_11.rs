use super::*;

#[test]
fn legacy_creature_melee_tick_once_two_attackers_commit_only_one_lethal_player_hit_like_cpp() {
    use wow_world::map_manager::RuntimeTickOwner;

    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    let player = ObjectGuid::create_player(1, 91_040);
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
        player.unit_mut().set_level(80);
        player.unit_mut().set_max_health(1);
        player.unit_mut().set_health(1);
    }

    let (mut session, _, _) = make_session();
    session.set_canonical_map_manager(Arc::clone(&canonical));
    let attackers = [test_creature_guid(91_041), test_creature_guid(91_042)];
    for attacker in attackers {
        add_canonical_test_creature_on_map(
            &canonical,
            attacker,
            9001,
            Position::new(10.0, 10.0, 0.0, 0.0),
            0,
            0,
            0,
        );
        register_test_creature(&mut session, Arc::clone(&manager), attacker, 25);
        session
            .fixture_melee_mutate_creature(attacker, |creature| {
                creature.enter_combat(player);
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
    assert_eq!(outcome.melee_precondition_rejections, 1);
    assert_eq!(outcome.commands.len(), 1);
    assert_eq!(outcome.commands[0].damage, 1);
    assert_eq!(outcome.commands[0].over_damage, 0);
    let canonical_guard = canonical.lock().unwrap();
    let player = canonical_guard
        .find_map(0, 0)
        .unwrap()
        .map()
        .get_typed_player(player)
        .unwrap();
    assert_eq!(player.unit().data().health, 0);
    assert_eq!(
        player.unit().death_state(),
        wow_constants::DeathState::JustDied
    );
    assert!(!player.unit().is_alive());
    assert_eq!(
        outcome.commands[0].victim_health_state_revision_after,
        player.unit().health_state_revision_like_cpp()
    );
    drop(canonical_guard);

    let legacy_guard = manager.read().unwrap();
    let tombstoned = attackers
        .into_iter()
        .filter(|attacker| {
            let creature = legacy_guard.find_creature(0, 0, *attacker).unwrap();
            assert_eq!(creature.creature.ai_ownership().swing_timer_ms, 2_000);
            !creature.runtime_rng_authority_complete_like_cpp()
        })
        .count();
    assert_eq!(tombstoned, 1, "only the committed hit consumes melee RNG");
}
