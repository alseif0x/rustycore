use super::*;

#[test]
fn legacy_creature_melee_tick_once_rejects_no_melee_static_flag_like_cpp() {
    use wow_world::map_manager::RuntimeTickOwner;
    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    let player = ObjectGuid::create_player(1, 91_020);
    add_canonical_test_player_on_map(
        &canonical,
        player,
        Position::new(10.0, 10.0, 0.0, 0.0),
        0,
        0,
    );

    let (mut session, _, _) = make_session();
    let creature_guid = test_creature_guid(91_021);
    register_test_creature(&mut session, manager.clone(), creature_guid, 25);
    session
        .fixture_melee_mutate_creature(creature_guid, |creature| {
            creature.enter_combat(player);
            creature.creature.ai_ownership_mut().last_swing_ms = 0;
            creature.creature.ai_ownership_mut().swing_timer_ms = 0;
            let mut static_flags = [0; 8];
            static_flags[0] = wow_constants::creature::CreatureStaticFlags::NO_MELEE_FLEE.bits();
            creature
                .creature
                .set_static_flags_runtime_like_cpp(static_flags);
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

    assert_eq!(rejected.creatures_seen, 1);
    assert_eq!(rejected.swings_ready, 0);
    assert_eq!(rejected.melee_precondition_rejections, 1);
    assert_eq!(rejected.canonical_hits, 0);
    assert!(rejected.commands.is_empty());
    let guard = manager.read().unwrap();
    let creature = guard.find_creature(0, 0, creature_guid).unwrap();
    assert!(
        creature.runtime_rng_authority_complete_like_cpp(),
        "NO_MELEE returns before CalculateMeleeDamage consumes runtime RNG"
    );
    assert_eq!(creature.creature.ai_ownership().last_swing_ms, 0);
    assert_eq!(creature.creature.ai_ownership().swing_timer_ms, 0);
}
