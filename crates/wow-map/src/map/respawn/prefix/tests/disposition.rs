use super::*;

#[test]
fn capture_keeps_canonical_actor_borrowed_and_legacy_original_moved_until_iteration_end() {
    let (mut map, mut old, guid) = pair(909, 909);
    dead(
        map.creature_actor_mut(guid).unwrap(),
        DeathState::Corpse,
        false,
    );
    dead(
        old.find_creature_mut(571, 7, guid).unwrap(),
        DeathState::Corpse,
        false,
    );
    let time = clocks();
    let before = map.creature_actor(guid).unwrap() as *const WorldCreature;
    let captured = Memory::Canonical(&mut map)
        .cleanup_and_capture(guid, 571, time)
        .unwrap();
    assert!(captured._detached_actor.is_none());
    assert_eq!(
        map.creature_actor(guid).unwrap() as *const WorldCreature,
        before
    );
    assert_eq!(captured.pending.create_data.guid, guid);
    assert_eq!(
        map.creature_actor(guid)
            .unwrap()
            .runtime_elapsed_ms_like_cpp(),
        5_000
    );
    assert_eq!(map.respawn_store.actor_queue_len(), 0);
    let captured = Memory::Legacy {
        manager: &mut old,
        key: (571, 7),
    }
    .cleanup_and_capture(guid, 571, time)
    .unwrap();
    assert!(old.find_creature(571, 7, guid).is_none());
    assert_eq!(captured._detached_actor.as_ref().unwrap().guid(), guid);
    assert_eq!(
        captured
            ._detached_actor
            .as_ref()
            .unwrap()
            .runtime_elapsed_ms_like_cpp(),
        5_000
    );
    assert_eq!(captured.pending.create_data.guid, guid);
    assert_eq!(old.respawn_queue_len(571, 7), 0);
}

#[test]
fn canonical_terminal_remove_failure_retains_upsert_queue_and_info_while_legacy_remove_succeeds() {
    let (mut map, mut old, guid) = pair(910, 910);
    dead(
        map.creature_actor_mut(guid).unwrap(),
        DeathState::Corpse,
        false,
    );
    dead(
        old.find_creature_mut(571, 7, guid).unwrap(),
        DeathState::Corpse,
        false,
    );
    // A real existing binding failure, without a fake callback/failure switch:
    // the Actor is still in storage but its WorldObject has no current Map.
    for actor in [
        map.creature_actor_mut(guid).unwrap(),
        old.find_creature_mut(571, 7, guid).unwrap(),
    ] {
        actor
            .creature
            .unit_mut()
            .world_mut()
            .object_mut()
            .remove_from_world();
        actor.creature.unit_mut().world_mut().reset_map().unwrap();
    }
    let time = clocks();
    let (outcome, ready) = canonical(&mut map, time, true);
    let legacy = run_legacy_prefix(&mut old, time, true);
    assert_eq!(outcome.corpses_removed, 0);
    assert_eq!(outcome.removal_failures, 1);
    assert_save_only(&outcome.respawn_db_mutations, 105);
    assert_eq!(map.respawn_store.actor_queue_len(), 1);
    assert_eq!(
        map.respawn_store
            .saved_row(SpawnObjectType::Creature, 910)
            .unwrap()
            .respawn_time,
        105
    );
    assert!(
        map.get_respawn_info_like_cpp(SpawnObjectType::Creature, 910)
            .is_some()
    );
    assert!(map.creature_actor(guid).is_none());
    assert!(ready.is_empty());
    assert_eq!(legacy.removed_corpses.len(), 1);
    assert_save_only(&legacy.respawn_db_mutations, 105);
    assert_eq!(old.respawn_queue_len(571, 7), 1);
    assert!(old.find_creature(571, 7, guid).is_none());
    assert!(legacy.ready.is_empty());
}

#[test]
fn missing_legacy_capture_does_not_create_row_queue_or_disposition_and_invalid_map_stays_untouched()
{
    let guid = actor(911, 911).guid();
    let time = clocks();
    let mut old = LegacyMapManager::new();
    assert!(
        Memory::Legacy {
            manager: &mut old,
            key: (571, 7)
        }
        .cleanup_and_capture(guid, 571, time)
        .is_none()
    );
    let legacy = run_legacy_prefix(&mut old, time, true);
    assert_eq!(legacy.creatures_seen, 0);
    assert!(legacy.removed_corpses.is_empty());
    assert!(legacy.respawn_db_mutations.is_empty());
    assert!(legacy.ready.is_empty());
    assert!(old.get_map(571, 7).is_none());
    let mut map = Map::new(u32::from(u16::MAX) + 1, 7, 0, 1000);
    let pending = pending_respawn_from_world_creature_like_cpp(&actor(912, 0), time.now, 571);
    map.respawn_store.queue_actor(pending).unwrap();
    let (outcome, ready) = canonical(&mut map, time, true);
    assert!(outcome.invalid_map_id);
    assert_eq!(outcome.creatures_seen, 0);
    assert_eq!(outcome.corpses_removed, 0);
    assert!(outcome.respawn_db_mutations.is_empty());
    assert!(ready.is_empty());
    assert_eq!(map.respawn_store.actor_queue_len(), 1);
    assert!(!map.respawn_store.has_reservations());
}

#[test]
fn corpse_admission_requires_dead_corpse_and_logical_due_on_both_rails() {
    for (state, health, due, expected_removed) in [
        (DeathState::Alive, 100, 0, 0),
        (DeathState::JustDied, 0, 0, 0),
        (DeathState::Corpse, 0, 6_000, 0),
        (DeathState::Corpse, 0, 5_000, 1),
    ] {
        let (mut map, mut old, guid) = pair(913, 913);
        for actor in [
            map.creature_actor_mut(guid).unwrap(),
            old.find_creature_mut(571, 7, guid).unwrap(),
        ] {
            dead(actor, state, false);
            actor.creature.unit_mut().set_health(health);
            actor.creature.set_ai_corpse_despawn_at(Some(due));
        }
        let time = clocks();
        let (outcome, _) = canonical(&mut map, time, true);
        let legacy = run_legacy_prefix(&mut old, time, true);
        assert_eq!(outcome.corpses_removed, expected_removed);
        assert_eq!(legacy.removed_corpses.len(), expected_removed);
        assert_eq!(map.creature_actor(guid).is_none(), expected_removed != 0);
        assert_eq!(
            old.find_creature(571, 7, guid).is_none(),
            expected_removed != 0
        );
    }
}
