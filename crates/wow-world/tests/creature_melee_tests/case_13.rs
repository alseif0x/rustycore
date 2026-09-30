use super::*;

#[test]
fn legacy_creature_melee_tick_once_rejects_same_guid_attacker_replacement_like_cpp() {
    use wow_world::map_manager::RuntimeTickOwner;

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
        .fixture_melee_mutate_creature(creature_guid, |creature| {
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

    let mut replacement = wow_world::map_manager::WorldCreature::new(
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
        .fixture_melee_mutate_creature(creature_guid, |creature| {
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
