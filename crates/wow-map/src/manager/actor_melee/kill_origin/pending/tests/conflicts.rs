use super::*;

fn captured_mut(pending: &mut PendingMeleeKills) -> &mut CapturedMeleeKill {
    match &mut pending.batch.occurrences[0].capture {
        MeleeKillCapture::Captured(captured) => captured,
        MeleeKillCapture::Unavailable(_) => panic!("real actor kill required"),
    }
}
fn replace_actor(manager: &mut MapManager, guid: ObjectGuid, counter: i64, active: bool) {
    let map = manager.find_map_mut(1, 0).unwrap().map_mut();
    assert!(map.remove_map_object(guid).is_some());
    let actor = new_actor(counter, Position::xyz(11.0, 20.0, 30.0), active);
    assert_eq!(actor.guid(), guid);
    map.test_fixture_admit_creature_actor(actor);
    place_in_loaded_cell(map, guid, Position::xyz(11.0, 20.0, 30.0));
}

#[test]
fn root_aba_is_rejected_before_target_validation_and_retains_the_old_slot() {
    let (mut manager, root, _) = setup(2860, 5, false, 10);
    let (tick, pending) = execution(&mut manager, root);
    replace_actor(&mut manager, root, 2860, true);
    let (error, pending) = reject(&manager, &tick, pending);
    assert_eq!(
        error,
        MeleeKillPhaseError::RootStale(ActorTickAccessError::WitnessMismatch { guid: root })
    );
    assert!(
        pending
            .token
            .actor_operation
            .as_ref()
            .unwrap()
            .witness
            .same_actor(&pending.batch.root_witness)
    );
    retained(&pending, root);
}

#[test]
fn target_aba_is_rejected_even_when_guid_and_entry_are_reused() {
    let (mut manager, root, victim) = setup(2870, 5, false, 10);
    let (tick, pending) = execution(&mut manager, root);
    replace_actor(&mut manager, victim, 2871, false);
    let (error, pending) = reject(&manager, &tick, pending);
    assert_eq!(
        error,
        MeleeKillPhaseError::TargetActorChanged { guid: victim }
    );
    assert_eq!(health(&manager, victim), 75);
    retained(&pending, root);
}

#[test]
fn missing_target_does_not_skip_or_dispose_the_occurrence() {
    let (mut manager, root, victim) = setup(2880, 5, false, 10);
    let (tick, pending) = execution(&mut manager, root);
    assert!(
        manager
            .find_map_mut(1, 0)
            .unwrap()
            .map_mut()
            .remove_map_object(victim)
            .is_some()
    );
    let (error, pending) = reject(&manager, &tick, pending);
    assert_eq!(
        error,
        MeleeKillPhaseError::TargetUnavailable { guid: victim }
    );
    assert_eq!(
        captured(&pending.batch.occurrences[0]).target_guid(),
        victim
    );
    retained(&pending, root);
}

#[test]
fn stale_map_rejects_before_an_unavailable_record_capture() {
    let (mut manager, root, _) = setup(2890, 5, true, 10);
    let (tick, pending) = execution(&mut manager, root);
    let incarnation = pending.token.incarnation();
    assert!(manager.destroy_map(1, 0));
    manager.create_world_map(1, 0);
    let current = manager.map_incarnation_like_cpp(pending.token.key());
    let (error, pending) = reject(&manager, &tick, pending);
    assert_eq!(
        error,
        MeleeKillPhaseError::RootStale(ActorTickAccessError::StaleParticipant {
            key: pending.token.key(),
            admitted_incarnation: incarnation,
            current_incarnation: current,
        })
    );
    assert!(matches!(
        pending.batch.occurrences[0].capture,
        MeleeKillCapture::Unavailable(MeleeKillCaptureError::NoActor)
    ));
    retained(&pending, root);
}

#[test]
fn foreign_root_operation_is_preserved_and_not_replaced_or_cleared() {
    let (mut manager, root, victim) = setup(2900, 5, false, 10);
    let foreign = target(&mut manager, 2902, 75, false);
    let (tick, mut pending) = execution(&mut manager, root);
    // Explicit test accounting disposes the original operation, then a real
    // admitted actor starts the foreign operation; preparation must preserve it.
    manager
        .complete_actor_operation(&tick, &mut pending.token, root, &pending.batch.root_witness)
        .unwrap();
    let foreign_witness = manager
        .begin_actor_operation(&tick, &mut pending.token, foreign, None)
        .unwrap();
    let (error, pending) = reject(&manager, &tick, pending);
    assert_eq!(
        error,
        MeleeKillPhaseError::RootStale(ActorTickAccessError::OperationMismatch { guid: root })
    );
    assert_eq!(
        pending.token.actor_operation.as_ref().unwrap().guid,
        foreign
    );
    assert!(
        pending
            .token
            .actor_operation
            .as_ref()
            .unwrap()
            .witness
            .same_actor(&foreign_witness)
    );
    assert!(
        pending
            .token
            .actor_operation
            .as_ref()
            .unwrap()
            .matches_token(&pending.token)
    );
    assert_eq!(pending.cursor(), 0);
    assert_eq!(pending.outcome.canonical_creature_hits, 1);
    assert_eq!(health(&manager, victim), 0);
}

#[test]
fn changed_loot_generation_keeps_the_before_hook_source_and_original_outcome() {
    let (mut manager, root, victim) = setup(2910, 5, false, 10);
    let (tick, pending) = execution(&mut manager, root);
    let expected = captured(&pending.batch.occurrences[0]).object_generation();
    let revision = captured(&pending.batch.occurrences[0])
        .source()
        .loot_lifecycle_revision();
    let current = manager
        .find_map(1, 0)
        .unwrap()
        .map()
        .creature_actor(victim)
        .unwrap()
        .creature
        .loot_authority_like_cpp()
        .retire_like_cpp();
    let (error, pending) = reject(&manager, &tick, pending);
    assert_eq!(
        error,
        MeleeKillPhaseError::GenerationConflict {
            guid: victim,
            captured: expected,
            current
        }
    );
    assert_eq!(
        captured(&pending.batch.occurrences[0])
            .source()
            .loot_lifecycle_revision(),
        revision
    );
    retained(&pending, root);
}

#[test]
fn changed_health_revision_rejects_without_healing_or_advancing_the_cursor() {
    let (mut manager, root, victim) = setup(2920, 5, false, 10);
    let (tick, pending) = execution(&mut manager, root);
    let expected = captured(&pending.batch.occurrences[0]).health_revision();
    let unit = manager
        .find_map_mut(1, 0)
        .unwrap()
        .map_mut()
        .creature_actor_mut(victim)
        .unwrap()
        .creature
        .unit_mut();
    unit.set_max_health(101);
    let current = unit.health_state_revision_like_cpp();
    assert!(current > expected);
    let (error, pending) = reject(&manager, &tick, pending);
    assert_eq!(
        error,
        MeleeKillPhaseError::HealthRevisionConflict {
            guid: victim,
            captured: expected,
            current
        }
    );
    assert_eq!(health(&manager, victim), 0);
    retained(&pending, root);
}

#[test]
fn conflicting_captured_authority_is_rejected_before_generation_or_health() {
    let (mut manager, root, victim) = setup(2930, 5, false, 10);
    let (tick, mut pending) = execution(&mut manager, root);
    // Corrupt only immutable input provenance after a real engine kill.
    // No fake hooks or replacement actor authority is installed.
    captured_mut(&mut pending).authority =
        wow_entities::OwnedLootAuthority::new_retired_tombstone_like_cpp();
    let (error, pending) = reject(&manager, &tick, pending);
    assert_eq!(
        error,
        MeleeKillPhaseError::AuthorityConflict { guid: victim }
    );
    retained(&pending, root);
}
