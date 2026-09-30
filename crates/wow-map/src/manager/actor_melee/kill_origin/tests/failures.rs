use super::*;

#[test]
fn foreign_pending_operation_rejects_before_rng_and_returns_the_original_slot() {
    let (mut manager, root, victim) = setup(2560, 5, false, 10);
    let (tick, mut token) = start(
        &mut manager,
        200,
        MapObjectUpdateSelectionLikeCpp::WholeTypedStores,
    );
    let witness = manager
        .begin_actor_operation(&tick, &mut token, root, None)
        .unwrap();
    let identity = (token.key(), token.incarnation(), token.effective_diff_ms());
    let catalogs = Catalogs::default();
    let (error, mut token) =
        match manager.apply_selected_creature_melee_with_kills(&tick, token, root, &catalogs) {
            Err(rejected) => rejected,
            Ok(_) => panic!("an occupied slot must reject before melee facts"),
        };
    assert_eq!(
        error,
        ActorTickAccessError::Tick(ObjectMapTickError::ActorOperationInFlight { guid: root })
    );
    assert_eq!(
        (token.key(), token.incarnation(), token.effective_diff_ms()),
        identity
    );
    assert!(catalogs.calls.borrow().is_empty());
    assert_eq!(health(&manager, victim), 5);
    assert!(
        manager
            .find_map(1, 0)
            .unwrap()
            .map()
            .creature_actor(root)
            .unwrap()
            .runtime_rng_authority_complete_like_cpp()
    );
    assert!(
        token
            .actor_operation
            .as_ref()
            .unwrap()
            .witness
            .same_actor(&witness)
    );
    manager
        .complete_actor_operation(&tick, &mut token, root, &witness)
        .unwrap();
}

#[test]
fn private_engine_capture_failure_keeps_full_damage_and_does_not_clear_the_foreign_slot() {
    let (mut manager, root, victim) = setup(2570, 5, false, 100);
    let foreign = target(&mut manager, 2572, 5, false);
    let secondary = target(&mut manager, 2573, 5, false);
    let mut catalogs = Catalogs::inert();
    share(&mut manager, victim, secondary, 5, 50, &mut catalogs);
    let (tick, mut token) = start(
        &mut manager,
        200,
        MapObjectUpdateSelectionLikeCpp::WholeTypedStores,
    );
    let root_witness = manager
        .selected_actor_witness(&mut token, root, None)
        .unwrap();
    let foreign_witness = manager
        .begin_actor_operation(&tick, &mut token, foreign, None)
        .unwrap();
    let CreatureMeleeReadiness::Ready(swing) = creature_melee_readiness(
        manager
            .find_map(1, 0)
            .unwrap()
            .map()
            .creature_actor(root)
            .unwrap(),
        1,
        0,
    ) else {
        panic!("ready actor");
    };
    // Exercise the private collector failure branch with the actual shared engine.
    // The public admission case above rejects this setup before entering the engine.
    let mut kills = Some(MeleeKillCollector::new(&mut token, root, root_witness));
    let outcome = manager.apply_canonical_creature_melee_swing_with_kills(
        crate::MapKey::new(1, 0),
        root,
        manager
            .find_map(1, 0)
            .unwrap()
            .map()
            .creature_actor_witness(root)
            .unwrap(),
        swing,
        &catalogs,
        &mut kills,
    );
    let batch = kills.take().unwrap().finish();
    drop(kills);
    let expected =
        ActorTickAccessError::Tick(ObjectMapTickError::ActorOperationInFlight { guid: foreign });
    assert_eq!(batch.reservation_error(), Some(expected));
    assert_eq!(batch.occurrences().len(), 2);
    assert_eq!(batch.occurrences()[0].phase(), MeleeKillPhase::Share);
    assert_eq!(batch.occurrences()[1].phase(), MeleeKillPhase::Primary);
    for occurrence in batch.occurrences() {
        assert!(matches!(occurrence.capture(),
            MeleeKillCapture::Unavailable(MeleeKillCaptureError::Reservation(error)) if *error == expected));
    }
    assert_eq!(outcome.canonical_creature_hits, 1);
    assert_eq!(outcome.syncs.len(), 2);
    assert!(!outcome.events.is_empty());
    assert_eq!(health(&manager, secondary), 0);
    assert_eq!(health(&manager, victim), 0);
    assert_eq!(token.actor_operation.as_ref().unwrap().guid, foreign);
    assert!(
        token
            .actor_operation
            .as_ref()
            .unwrap()
            .witness
            .same_actor(&foreign_witness)
    );
    manager
        .complete_actor_operation(&tick, &mut token, foreign, &foreign_witness)
        .unwrap();
}

#[test]
fn collector_retains_repeated_occurrences_and_their_distinct_frozen_revisions() {
    let (mut manager, root, victim) = setup(2580, 5, false, 10);
    let (tick, mut token) = start(
        &mut manager,
        200,
        MapObjectUpdateSelectionLikeCpp::WholeTypedStores,
    );
    let witness = manager
        .selected_actor_witness(&mut token, root, None)
        .unwrap();
    let CreatureMeleeReadiness::Ready(swing) = creature_melee_readiness(
        manager
            .find_map(1, 0)
            .unwrap()
            .map()
            .creature_actor(root)
            .unwrap(),
        1,
        0,
    ) else {
        panic!("ready actor");
    };
    let mut kills = Some(MeleeKillCollector::new(&mut token, root, witness.clone()));
    let outcome = manager.apply_canonical_creature_melee_swing_with_kills(
        crate::MapKey::new(1, 0),
        root,
        witness,
        swing,
        &Catalogs::default(),
        &mut kills,
    );
    assert_eq!(outcome.canonical_creature_hits, 1);
    // Ledger-only repeated callback coverage after the real engine kill. The
    // real engine's alive guards do not invent a second death for a dead actor.
    manager
        .find_map_mut(1, 0)
        .unwrap()
        .map_mut()
        .get_typed_creature_mut(victim)
        .unwrap()
        .unit_mut()
        .set_max_health(101);
    let map = manager.find_map(1, 0).unwrap().map();
    kills
        .as_mut()
        .unwrap()
        .capture(map, crate::MapKey::new(1, 0), MeleeKillPhase::Share, victim);
    kills.as_mut().unwrap().capture(
        map,
        crate::MapKey::new(1, 0),
        MeleeKillPhase::Primary,
        victim,
    );
    let batch = kills.take().unwrap().finish();
    drop(kills);
    assert_eq!(batch.occurrences().len(), 3);
    assert_eq!(
        batch
            .occurrences()
            .iter()
            .map(|o| o.phase())
            .collect::<Vec<_>>(),
        vec![
            MeleeKillPhase::Primary,
            MeleeKillPhase::Share,
            MeleeKillPhase::Primary
        ]
    );
    let first = captured(&batch.occurrences()[0]);
    let second = captured(&batch.occurrences()[1]);
    assert_eq!(first.target_guid(), second.target_guid());
    assert!(first.target_witness.same_actor(&second.target_witness));
    assert!(first.health_revision() < second.health_revision());
    assert_eq!(
        second.health_revision(),
        captured(&batch.occurrences()[2]).health_revision()
    );
    assert_eq!(token.actor_operation.as_ref().unwrap().guid, root);
}

#[test]
fn rejected_reservation_is_not_retried_after_explicit_foreign_disposal() {
    let (mut manager, root, victim) = setup(2590, 5, false, 10);
    let foreign = target(&mut manager, 2592, 75, false);
    let (tick, mut token) = start(
        &mut manager,
        200,
        MapObjectUpdateSelectionLikeCpp::WholeTypedStores,
    );
    let witness = manager
        .selected_actor_witness(&mut token, root, None)
        .unwrap();
    let foreign_witness = manager
        .begin_actor_operation(&tick, &mut token, foreign, None)
        .unwrap();
    let CreatureMeleeReadiness::Ready(swing) = creature_melee_readiness(
        manager
            .find_map(1, 0)
            .unwrap()
            .map()
            .creature_actor(root)
            .unwrap(),
        1,
        0,
    ) else {
        panic!("ready actor");
    };
    let mut kills = Some(MeleeKillCollector::new(&mut token, root, witness.clone()));
    let outcome = manager.apply_canonical_creature_melee_swing_with_kills(
        crate::MapKey::new(1, 0),
        root,
        witness,
        swing,
        &Catalogs::default(),
        &mut kills,
    );
    assert_eq!(outcome.canonical_creature_hits, 1);
    let collector = kills.as_mut().unwrap();
    manager
        .complete_actor_operation(&tick, collector.token, foreign, &foreign_witness)
        .unwrap();
    collector.capture(
        manager.find_map(1, 0).unwrap().map(),
        crate::MapKey::new(1, 0),
        MeleeKillPhase::Share,
        victim,
    );
    let batch = kills.take().unwrap().finish();
    drop(kills);
    assert_eq!(batch.occurrences().len(), 2);
    assert!(!batch.is_reserved());
    assert!(batch.reservation_error().is_some());
    assert!(batch.occurrences().iter().all(|o| matches!(
        o.capture(),
        MeleeKillCapture::Unavailable(MeleeKillCaptureError::Reservation(_))
    )));
    assert!(token.actor_operation.is_none());
}
