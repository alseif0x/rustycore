use super::*;

#[test]
fn legacy_creature_melee_tick_once_rejects_missing_canonical_attacker_before_side_effects_like_cpp()
{
    use wow_world::map_manager::RuntimeTickOwner;

    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    let player = ObjectGuid::create_player(1, 91_036);
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
    let creature_guid = test_creature_guid(91_037);
    let attacking_aura = wow_entities::AppliedAuraRef::new(91_038, creature_guid, 0, 0x1);
    let seed = 0x91_037;
    register_test_creature(&mut session, manager.clone(), creature_guid, 25);
    session
        .fixture_melee_mutate_creature(creature_guid, |creature| {
            creature.enter_combat(player);
            creature.creature.ai_ownership_mut().last_swing_ms = 0;
            creature.creature.ai_ownership_mut().swing_timer_ms = 0;
            creature.seed_runtime_rng_like_cpp(seed);
            creature
                .creature
                .unit_mut()
                .subsystems_mut()
                .auras
                .register_applied_aura(
                    attacking_aura,
                    None,
                    wow_entities::SPELL_AURA_INTERRUPT_FLAG_ATTACKING_LIKE_CPP,
                    0,
                );
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
    assert_eq!(outcome.runtime_rng_authority_rejections, 0);
    assert_eq!(outcome.attacking_interrupt_auras_removed, 0);
    assert!(outcome.commands.is_empty());
    assert!(outcome.plan.events.is_empty());

    let mut expected_rng = StdRng::seed_from_u64(seed);
    let expected_first_roll = expected_rng.gen_range(0..=9_999_u32);
    let (authoritative, swing_timer_ms, aura_still_applied, actual_first_roll) = session
        .fixture_melee_mutate_creature(creature_guid, |creature| {
            (
                creature.runtime_rng_authority_complete_like_cpp(),
                creature.creature.ai_ownership().swing_timer_ms,
                creature
                    .creature
                    .unit()
                    .subsystems()
                    .auras
                    .has_applied(attacking_aura),
                creature.random_creature_spell_hit_roll_like_cpp(),
            )
        })
        .unwrap();
    assert!(authoritative);
    assert_eq!(swing_timer_ms, 0);
    assert!(aura_still_applied);
    assert_eq!(actual_first_roll, Some(expected_first_roll));
}
