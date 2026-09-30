use super::*;

#[test]
fn legacy_creature_melee_tick_once_is_noop_under_session_owner_like_cpp() {
    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    let player = ObjectGuid::create_player(1, 91_001);
    add_canonical_test_player_on_map(&canonical, player, Position::ZERO, 0, 0);
    {
        let mut guard = canonical.lock().unwrap();
        let typed = guard
            .find_map_mut(0, 0)
            .unwrap()
            .map_mut()
            .get_typed_player_mut(player)
            .unwrap();
        typed.unit_mut().set_level(80);
        typed.unit_mut().set_max_health(100);
        typed.unit_mut().set_health(100);
    }

    let (mut session, _, _) = make_session();
    let creature_guid = test_creature_guid(91_002);
    register_test_creature(&mut session, manager.clone(), creature_guid, 25);
    session
        .fixture_melee_mutate_creature(creature_guid, |creature| {
            creature.enter_combat(player);
            creature.creature.ai_ownership_mut().swing_timer_ms = 0;
        })
        .unwrap();

    let outcome = run_legacy_creature_melee_tick_once_like_cpp(
        &manager,
        Some(&canonical),
        &Default::default(),
    );

    assert!(outcome.skipped_owner_not_global);
    assert_eq!(outcome.maps_seen, 0);
    assert_eq!(outcome.creatures_seen, 0);
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
}
