use super::*;

#[test]
fn legacy_creature_melee_tick_once_removes_attacking_auras_on_compatibility_hit_like_cpp() {
    use wow_world::map_manager::RuntimeTickOwner;
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
        .fixture_melee_mutate_creature(creature_guid, |creature| {
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
