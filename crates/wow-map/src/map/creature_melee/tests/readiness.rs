use super::*;

#[test]
fn initial_range_rejection_queries_threat_but_preserves_rng_and_health() {
    let (mut manager, mut attacker, swing) = setup(true, 50.0);
    let catalogs = Catalogs::inert();
    let outcome = manager.apply_legacy_creature_melee_swing(Some(&mut attacker), swing, &catalogs);
    assert_eq!(&*catalogs.calls.borrow(), &["threat", "threat_aura"]);
    assert_eq!(outcome.melee_range_rejections, 1);
    assert_eq!(attacker.creature.ai_ownership().swing_timer_ms, 100);
    assert_eq!(health(&manager, swing.victim_guid), 100);
    assert!(outcome.commands.is_empty() && outcome.events.is_empty() && outcome.syncs.is_empty());
    let mut expected = StdRng::seed_from_u64(17);
    attacker.creature.ai_ownership_mut().min_damage = 1;
    attacker.creature.ai_ownership_mut().max_damage = 10_000;
    assert_eq!(attacker.roll_damage(), Some(expected.gen_range(1..=10_000)));
    assert!(attacker.runtime_rng_authority_complete_like_cpp());
}

#[test]
fn missing_attacker_and_incarnation_rejection_never_query_catalogs_or_write() {
    let (mut manager, mut attacker, swing) = setup(true, 1.0);
    let catalogs = Catalogs::inert();
    assert_eq!(
        manager
            .apply_legacy_creature_melee_swing(None, swing, &catalogs)
            .melee_precondition_rejections,
        1
    );
    let replacement = creature(1, Position::ZERO, 100);
    manager
        .find_map_mut(0, 0)
        .unwrap()
        .map_mut()
        .insert_map_object_record(MapObjectRecord::new_creature(replacement).unwrap())
        .unwrap();
    let outcome = manager.apply_legacy_creature_melee_swing(Some(&mut attacker), swing, &catalogs);
    assert_eq!(outcome.attacker_incarnation_rejections, 1);
    assert!(catalogs.calls.borrow().is_empty());
    assert_eq!(health(&manager, swing.victim_guid), 100);
    assert_eq!(attacker.creature.ai_ownership().swing_timer_ms, 0);
    assert!(attacker.runtime_rng_authority_complete_like_cpp());
}

#[test]
fn charging_and_blocking_cast_reject_before_any_catalog_query() {
    for flag in [
        wow_constants::UnitState::CHARGING,
        wow_constants::UnitState::CASTING,
    ] {
        let (mut manager, mut attacker, swing) = setup(true, 1.0);
        attacker.creature.unit_mut().add_unit_state(flag.bits());
        assert!(matches!(
            creature_melee_readiness(&attacker, 0, 0),
            CreatureMeleeReadiness::Rejected
        ));
        let catalogs = Catalogs::inert();
        let outcome =
            manager.apply_legacy_creature_melee_swing(Some(&mut attacker), swing, &catalogs);
        assert_eq!(outcome.melee_precondition_rejections, 1);
        assert!(catalogs.calls.borrow().is_empty());
        assert_eq!(attacker.creature.ai_ownership().swing_timer_ms, 0);
        assert_eq!(health(&manager, swing.victim_guid), 100);
    }
}

#[test]
fn absent_catalog_still_consumes_equal_bound_draw_and_rearms_after_commit() {
    let (mut manager, mut attacker, swing) = setup(true, 1.0);
    let catalogs = Catalogs::default();
    let outcome = manager.apply_legacy_creature_melee_swing(Some(&mut attacker), swing, &catalogs);
    assert_eq!(outcome.melee_outcomes_unrepresented, 1);
    assert_eq!(outcome.canonical_hits, 1);
    assert_eq!(outcome.commands[0].damage, 10);
    assert_eq!(health(&manager, swing.victim_guid), 90);
    assert_eq!(
        attacker.creature.ai_ownership().swing_timer_ms,
        attacker.create_data.base_attack_time as u64
    );
    assert!(!attacker.runtime_rng_authority_complete_like_cpp());
    assert_eq!(&*catalogs.calls.borrow(), &["threat"]);
    let mut expected = StdRng::seed_from_u64(17);
    let _ = expected.next_u32();
    attacker.creature.ai_ownership_mut().min_damage = 1;
    attacker.creature.ai_ownership_mut().max_damage = 10_000;
    assert_eq!(attacker.roll_damage(), Some(expected.gen_range(1..=10_000)));
}

#[test]
fn invalid_damage_rearms_without_committing_or_visiting_mitigation() {
    let (mut manager, mut attacker, swing) = setup(true, 1.0);
    attacker.creature.ai_ownership_mut().min_damage = 20;
    attacker.creature.ai_ownership_mut().max_damage = 10;
    let catalogs = Catalogs::inert();
    let outcome = manager.apply_legacy_creature_melee_swing(Some(&mut attacker), swing, &catalogs);
    assert_eq!(outcome.melee_precondition_rejections, 1);
    assert_eq!(&*catalogs.calls.borrow(), &["threat", "threat_aura"]);
    assert_eq!(health(&manager, swing.victim_guid), 100);
    assert!(!attacker.runtime_rng_authority_complete_like_cpp());
    assert_eq!(
        attacker.creature.ai_ownership().swing_timer_ms,
        attacker.create_data.base_attack_time as u64
    );
}
