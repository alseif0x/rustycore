use super::*;

#[test]
fn legacy_creature_melee_tick_once_rejects_blocking_channel_like_cpp() {
    use wow_world::map_manager::RuntimeTickOwner;
    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    let player = ObjectGuid::create_player(1, 91_015);
    add_canonical_test_player_on_map(
        &canonical,
        player,
        Position::new(10.0, 10.0, 0.0, 0.0),
        0,
        0,
    );

    let (mut session, _, _) = make_session();
    let creature_guid = test_creature_guid(91_016);
    register_test_creature(&mut session, manager.clone(), creature_guid, 25);
    session
        .fixture_melee_mutate_creature(creature_guid, |creature| {
            creature.enter_combat(player);
            creature.creature.ai_ownership_mut().last_swing_ms = 0;
            creature.creature.ai_ownership_mut().swing_timer_ms = 0;
            creature.creature.unit_mut().set_current_cast_spell(
                wow_entities::CurrentSpellSlot::Channeled,
                wow_entities::CurrentSpellRef::new(12_347, Some(creature_guid), None),
            );
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
}
