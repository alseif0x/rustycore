use super::*;

#[test]
fn legacy_creature_melee_tick_once_failed_auto_attack_uses_retry_timer_like_cpp() {
    use wow_world::map_manager::RuntimeTickOwner;
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
        .fixture_melee_mutate_creature(creature_guid, |creature| {
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
