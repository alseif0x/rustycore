use super::*;

#[test]
fn legacy_creature_melee_tick_once_rejects_out_of_range_victim_like_cpp() {
    use wow_world::map_manager::RuntimeTickOwner;
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
        .fixture_melee_mutate_creature(creature_guid, |creature| {
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
        .fixture_melee_mutate_creature(creature_guid, |creature| {
            assert!(
                creature.runtime_rng_authority_complete_like_cpp(),
                "C++ rejects range before entering its melee RNG surface"
            );
            creature.random_creature_spell_hit_roll_like_cpp()
        })
        .unwrap();
    assert_eq!(actual_first_roll, Some(expected_first_roll));
}
