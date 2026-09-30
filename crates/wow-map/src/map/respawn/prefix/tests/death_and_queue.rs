use super::*;

#[test]
fn both_rails_save_death_before_corpse_and_clear_request_without_advancing_clock() {
    let (mut map, mut old, guid) = pair(901, 901);
    dead(map.creature_actor_mut(guid).unwrap(), DeathState::JustDied, true);
    dead(old.find_creature_mut(571, 7, guid).unwrap(), DeathState::JustDied, true);
    let time = clocks();
    let (outcome, ready) = canonical(&mut map, time, true);
    let legacy = run_legacy_prefix(&mut old, time, true);
    assert_eq!(outcome.creatures_seen, 1);
    assert_eq!(legacy.creatures_seen, 1);
    assert_save_only(&outcome.respawn_db_mutations, 105);
    assert_save_only(&legacy.respawn_db_mutations, 105);
    assert_eq!(outcome.corpses_removed, 0);
    assert!(legacy.removed_corpses.is_empty());
    assert!(ready.is_empty());
    assert!(legacy.ready.is_empty());
    assert!(!map.creature_actor(guid).unwrap().creature.runtime_state().save_respawn_requested);
    assert!(!old.find_creature(571, 7, guid).unwrap().creature.runtime_state().save_respawn_requested);
    assert_eq!(map.creature_actor(guid).unwrap().runtime_elapsed_ms_like_cpp(), 5_000);
    assert_eq!(old.find_creature(571, 7, guid).unwrap().runtime_elapsed_ms_like_cpp(), 5_000);
    assert!(map.get_respawn_info_like_cpp(SpawnObjectType::Creature, 901).is_none());
    assert_eq!(map.respawn_store.actor_queue_len(), 0);
    assert_eq!(old.respawn_queue_len(571, 7), 0);
}

#[test]
fn both_rails_keep_conversion_clock_separate_from_due_clock_and_reserve_only_canonical_ready() {
    let (mut map, mut old, guid) = pair(902, 902);
    dead(map.creature_actor_mut(guid).unwrap(), DeathState::Corpse, false);
    dead(old.find_creature_mut(571, 7, guid).unwrap(), DeathState::Corpse, false);
    let time = clocks();
    let (outcome, ready) = canonical(&mut map, time, true);
    let legacy = run_legacy_prefix(&mut old, time, true);
    assert_eq!(outcome.corpses_removed, 1);
    assert_eq!(outcome.removal_failures, 0);
    assert_eq!(legacy.removed_corpses.len(), 1);
    assert_eq!(legacy.removed_corpses[0].guid, guid);
    assert_save_only(&outcome.respawn_db_mutations, 105);
    assert_save_only(&legacy.respawn_db_mutations, 105);
    assert!(ready.is_empty());
    assert!(legacy.ready.is_empty());
    assert!(map.creature_actor(guid).is_none());
    assert!(old.find_creature(571, 7, guid).is_none());
    assert_eq!(map.get_respawn_info_like_cpp(SpawnObjectType::Creature, 902),
        legacy.removed_corpses[0].respawn_info.as_ref());
    assert_eq!(map.respawn_store.actor_queue_len(), 1);
    assert_eq!(old.respawn_queue_len(571, 7), 1);
    let due = Clocks { now: time.conversion_now + Duration::from_secs(6), ..time };
    let (outcome, ready) = canonical(&mut map, due, true);
    let legacy = run_legacy_prefix(&mut old, due, true);
    assert!(outcome.respawn_db_mutations.is_empty());
    assert!(legacy.respawn_db_mutations.is_empty());
    assert_eq!(ready.len(), 1);
    assert_eq!(legacy.ready.len(), 1);
    assert_eq!(ready[0].create_data.guid, guid);
    assert_eq!(legacy.ready[0].create_data.guid, guid);
    assert_eq!(ready[0].respawn_at, time.conversion_now + Duration::from_secs(5));
    assert_eq!(legacy.ready[0].respawn_at, ready[0].respawn_at);
    assert!(map.respawn_store.is_reserved(RespawnKey::Persistent(SpawnObjectType::Creature, 902)));
    old.push_respawn(571, 7, legacy.ready.into_iter().next().unwrap());
    assert_eq!(old.respawn_queue_len(571, 7), 1, "legacy drained entry can be queued again without a reservation");
    map.respawn_store.release_respawn_key(RespawnKey::Persistent(SpawnObjectType::Creature, 902));
}

#[test]
fn corpse_upsert_extends_only_older_rows_without_publishing_delete_on_either_rail() {
    for stored in [102, 105, 108] {
        let (mut map, mut old, guid) = pair(903, 903);
        dead(map.creature_actor_mut(guid).unwrap(), DeathState::Corpse, false);
        dead(old.find_creature_mut(571, 7, guid).unwrap(), DeathState::Corpse, false);
        let time = clocks();
        let pending = pending_respawn_from_world_creature_like_cpp(
            map.creature_actor(guid).unwrap(),
            time.conversion_now + Duration::from_secs((stored - 100) as u64), 571,
        );
        map.respawn_store.save_actor_row(&pending, 571, 7, time.conversion_now, 100);
        old.save_pending_respawn_time_like_cpp(571, 7, &pending, time.conversion_now, 100);
        let (outcome, _) = canonical(&mut map, time, true);
        let legacy = run_legacy_prefix(&mut old, time, true);
        if stored < 105 {
            assert_save_only(&outcome.respawn_db_mutations, 105);
            assert_save_only(&legacy.respawn_db_mutations, 105);
        } else {
            assert!(outcome.respawn_db_mutations.is_empty());
            assert!(legacy.respawn_db_mutations.is_empty());
        }
        assert_eq!(map.respawn_store.saved_row(SpawnObjectType::Creature, 903).unwrap().respawn_time,
            stored.max(105));
        assert_eq!(old.persisted_respawn_time_like_cpp(571, 7, SpawnObjectType::Creature, 903),
            Some(stored.max(105)));
    }
}

#[test]
fn synthetic_and_instanceable_corpse_paths_keep_original_persistence_and_info_gates() {
    for (spawn_id, persistent_map) in [(0, true), (904, false)] {
        let (mut map, mut old, guid) = pair(904, spawn_id);
        dead(map.creature_actor_mut(guid).unwrap(), DeathState::Corpse, true);
        dead(old.find_creature_mut(571, 7, guid).unwrap(), DeathState::Corpse, true);
        let time = clocks();
        let (outcome, ready) = canonical(&mut map, time, persistent_map);
        let legacy = run_legacy_prefix(&mut old, time, persistent_map);
        assert!(outcome.respawn_db_mutations.is_empty());
        assert!(legacy.respawn_db_mutations.is_empty());
        assert_eq!(outcome.corpses_removed, 1);
        assert_eq!(legacy.removed_corpses.len(), 1);
        assert!(ready.is_empty());
        assert!(legacy.ready.is_empty());
        assert_eq!(legacy.removed_corpses[0].respawn_info.is_some(), spawn_id != 0);
        assert_eq!(map.get_respawn_info_like_cpp(SpawnObjectType::Creature, spawn_id).is_some(),
            spawn_id != 0);
        assert!(old.persisted_respawn_rows_like_cpp(571, 7).is_empty());
        assert!(map.respawn_store.saved_rows().is_empty());
    }
}

#[test]
fn ready_ordinal_follows_each_rails_enumeration_after_existing_queue_entries() {
    let time = clocks();
    let mut map = Map::new(571, 7, 0, 1000);
    let mut old = LegacyMapManager::new();
    for id in [905, 906, 907] {
        let mut incoming = actor(id, id as u64);
        dead(&mut incoming, DeathState::Corpse, false);
        incoming.creature.ai_ownership_mut().respawn_time_secs = 0;
        map.admit_creature_actor(incoming).unwrap();
        let mut incoming = actor(id, id as u64);
        dead(&mut incoming, DeathState::Corpse, false);
        incoming.creature.ai_ownership_mut().respawn_time_secs = 0;
        let (x, y) = world_to_grid_coords(incoming.position().x, incoming.position().y);
        assert!(old.add_creature(571, 7, x, y, incoming));
    }
    let prior = actor(908, 0);
    let prior_guid = prior.guid();
    let pending = pending_respawn_from_world_creature_like_cpp(&prior, time.now, 571);
    map.respawn_store.queue_actor(pending).unwrap();
    let pending = pending_respawn_from_world_creature_like_cpp(&prior, time.now, 571);
    old.push_respawn(571, 7, pending);
    let mut canonical_order = vec![prior_guid];
    canonical_order.extend(Memory::Canonical(&mut map).actor_guids());
    let mut legacy_order = vec![prior_guid];
    legacy_order.extend(old.creature_guids(571, 7));
    let due = Clocks { now: time.conversion_now, ..time };
    let (outcome, ready) = canonical(&mut map, due, false);
    let legacy = run_legacy_prefix(&mut old, due, false);
    assert_eq!(outcome.corpses_removed, 3);
    assert_eq!(legacy.removed_corpses.len(), 3);
    assert_eq!(ready.iter().map(|p| p.create_data.guid).collect::<Vec<_>>(), canonical_order);
    assert_eq!(legacy.ready.iter().map(|p| p.create_data.guid).collect::<Vec<_>>(), legacy_order);
    assert_eq!(legacy.removed_corpses.iter().map(|c| c.guid).collect::<Vec<_>>(), legacy_order[1..]);
    assert!(map.respawn_store.has_reservations());
    for pending in legacy.ready { old.push_respawn(571, 7, pending); }
    assert_eq!(old.respawn_queue_len(571, 7), 4, "all legacy drained entries remain unreserved");
    for pending in ready {
        map.respawn_store.release_respawn_key(RespawnKey::for_actor(&pending));
    }
}
