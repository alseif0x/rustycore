use super::*;

#[test]
fn foreign_token_rejects_before_busy_selection_catalogs_or_actor_writes() {
    let (mut first, first_guid, first_victim) = setup(1801);
    let (mut second, second_guid, second_victim) = setup(1803);
    let (first_tick, mut first_token) = start(
        &mut first,
        200,
        MapObjectUpdateSelectionLikeCpp::WholeTypedStores,
    );
    let (second_tick, second_token) = start(
        &mut second,
        200,
        MapObjectUpdateSelectionLikeCpp::WholeTypedStores,
    );
    let witness = first
        .begin_actor_operation(&first_tick, &mut first_token, first_guid, None)
        .unwrap();
    let catalogs = Catalogs::default();
    assert!(matches!(
        second.apply_selected_creature_melee(&second_tick, &mut first_token, first_guid, &catalogs),
        Err(ActorTickAccessError::Tick(
            ObjectMapTickError::OriginMismatch { .. }
        ))
    ));
    assert!(matches!(
        first.apply_selected_creature_melee(&second_tick, &mut first_token, first_guid, &catalogs),
        Err(ActorTickAccessError::Tick(
            ObjectMapTickError::OriginMismatch { .. }
        ))
    ));
    assert!(catalogs.calls.borrow().is_empty());
    assert_untouched(&first, first_guid, first_victim);
    assert_untouched(&second, second_guid, second_victim);
    first
        .complete_actor_operation(&first_tick, &mut first_token, first_guid, &witness)
        .unwrap();
    drop(second_token);
    drop(first_token);
    assert!(matches!(
        first.tick_coordination_like_cpp(),
        MapTickCoordinationStateLikeCpp::Resuming(_)
    ));
    assert!(matches!(
        second.tick_coordination_like_cpp(),
        MapTickCoordinationStateLikeCpp::Resuming(_)
    ));
}

#[test]
fn pending_operation_rejects_even_its_own_actor_before_witness_and_retains_finish_ownership() {
    let (mut manager, guid, victim) = setup(1805);
    let (mut tick, mut token) = start(
        &mut manager,
        200,
        MapObjectUpdateSelectionLikeCpp::WholeTypedStores,
    );
    let witness = manager
        .begin_actor_operation(&tick, &mut token, guid, None)
        .unwrap();
    let catalogs = Catalogs::default();
    for requested in [guid, ObjectGuid::EMPTY] {
        let error = manager
            .apply_selected_creature_melee(&tick, &mut token, requested, &catalogs)
            .err()
            .unwrap();
        assert_eq!(
            error,
            ActorTickAccessError::Tick(ObjectMapTickError::ActorOperationInFlight { guid })
        );
    }
    assert!(catalogs.calls.borrow().is_empty());
    assert_untouched(&manager, guid, victim);
    let (error, returned) = manager
        .try_finish_object_map::<LoadRecord>(
            &mut tick,
            token,
            None,
            None,
            MapCreatureUpdateOwnerLikeCpp::CanonicalMap,
        )
        .err()
        .unwrap();
    assert_eq!(error, ObjectMapTickError::ActorOperationInFlight { guid });
    token = returned;
    assert_untouched(&manager, guid, victim);
    assert!(token.actor_operation.is_some());
    manager
        .complete_actor_operation(&tick, &mut token, guid, &witness)
        .unwrap();
    assert_eq!(
        finish(&mut manager, &mut tick, token),
        ObjectMapFinishOutcome::Completed
    );
}

#[test]
fn stale_map_incarnation_rejects_without_replaying_the_old_actor() {
    let (mut manager, guid, _) = setup(1807);
    let (tick, mut token) = start(
        &mut manager,
        200,
        MapObjectUpdateSelectionLikeCpp::WholeTypedStores,
    );
    let admitted = token.incarnation();
    let key = token.key();
    let old_map = manager.maps.remove(&key).unwrap();
    manager.map_incarnations_like_cpp.remove(&key);
    manager.create_world_map(1, 0);
    let current = manager.map_incarnation_like_cpp(key);
    let catalogs = Catalogs::default();
    assert_eq!(
        manager
            .apply_selected_creature_melee(&tick, &mut token, guid, &catalogs)
            .err()
            .unwrap(),
        ActorTickAccessError::StaleParticipant {
            key,
            admitted_incarnation: admitted,
            current_incarnation: current
        }
    );
    assert!(catalogs.calls.borrow().is_empty());
    assert_eq!(
        old_map
            .map()
            .creature_actor(guid)
            .unwrap()
            .creature
            .ai_ownership()
            .swing_timer_ms,
        0
    );
    assert!(
        old_map
            .map()
            .creature_actor(guid)
            .unwrap()
            .runtime_rng_authority_complete_like_cpp()
    );
    assert_eq!(
        manager
            .find_map(1, 0)
            .unwrap()
            .last_creatures_update_summary()
            .visited,
        0
    );
}
