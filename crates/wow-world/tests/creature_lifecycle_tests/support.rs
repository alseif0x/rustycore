use super::*;

pub(super) fn unix_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

pub(super) fn lifecycle_test_map_store_like_cpp(
    map_id: u32,
    instance_type: i8,
    flags1: u32,
) -> wow_data::MapStore {
    wow_data::MapStore::from_entries([wow_data::MapEntry {
        id: map_id,
        instance_type,
        expansion_id: 0,
        parent_map_id: -1,
        cosmetic_parent_map_id: -1,
        flags1,
        flags2: 0,
    }])
}

pub(super) fn assert_instanceable_map_does_not_persist_respawn_like_cpp(
    map_id: u16,
    instance_id: u32,
    instance_type: i8,
    flags1: u32,
    managed_kind: wow_map::ManagedMapKind,
    spawn_id: u64,
) {
    use crate::map_manager::{RuntimeTickOwner, world_to_grid_coords};

    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    canonical
        .lock()
        .unwrap()
        .create_map_entry(u32::from(map_id), instance_id, 0, managed_kind);

    let guid = test_creature_guid(
        i64::try_from(spawn_id).expect("test spawn id must fit the ObjectGuid counter"),
    );
    let mut creature = crate::map_manager::WorldCreature::new(
        guid,
        9005,
        Position::new(17.0, 18.0, 19.0, 1.0),
        50,
        7,
        8,
        12,
        20.0,
        104,
        14,
        0,
        0,
    );
    creature.creature.set_spawn_id(spawn_id);
    assert!(creature.take_damage(50));
    creature.set_corpse_despawn_at(Some(Instant::now() - Duration::from_secs(1)));
    let (grid_x, grid_y) = world_to_grid_coords(creature.position().x, creature.position().y);
    {
        let mut guard = manager.write().unwrap();
        guard.set_tick_owner(RuntimeTickOwner::GlobalLegacy);
        assert!(guard.add_creature(map_id, instance_id, grid_x, grid_y, creature));
    }

    let outcome = run_legacy_creature_lifecycle_tick_once_like_cpp(
        &manager,
        Some(&canonical),
        &lifecycle_test_map_store_like_cpp(u32::from(map_id), instance_type, flags1),
        Instant::now(),
    );

    assert_eq!(outcome.corpses_despawned, 1);
    assert!(
        outcome.respawn_db_mutations.is_empty(),
        "C++ Map::SaveRespawnInfoDB returns for Instanceable maps"
    );
    assert_eq!(
        manager.read().unwrap().persisted_respawn_time_like_cpp(
            map_id,
            instance_id,
            wow_map::SpawnObjectType::Creature,
            spawn_id,
        ),
        None
    );
}

pub(super) fn setup_dead_creature_past_despawn(
    guid_counter: i64,
) -> (WorldSession, flume::Receiver<Vec<u8>>, ObjectGuid) {
    let (mut session, _, send_rx) = make_session();
    let manager = shared_map_manager();
    let guid = test_creature_guid(guid_counter);
    register_test_creature(&mut session, manager.clone(), guid, 10);

    // Kill the creature and set corpse_despawn_at to the past so that
    // the very next run_creatures_tick triggers despawn.
    let past = Instant::now() - Duration::from_secs(1);
    session
        .fixture_melee_mutate_creature(guid, |c| {
            c.take_damage(10);
            c.set_corpse_despawn_at(Some(past));
        })
        .unwrap();

    // Mark the creature as client-visible so the DESTROY packet is built.
    session.fixture_melee_make_visible(guid);

    (session, send_rx, guid)
}

pub(super) fn force_respawn_ready(session: &mut WorldSession) {
    let map_id = session.fixture_lifecycle_map_id();
    let past = Instant::now() - Duration::from_secs(1);
    // Drain whatever is in the queue, rewrite respawn_at, push back.
    if let Some(manager) = &session.fixture_lifecycle_manager() {
        let mut mgr = manager.write().unwrap_or_else(|p| p.into_inner());
        let entries =
            mgr.drain_ready_respawns(map_id, 0, Instant::now() + Duration::from_secs(9999));
        for mut r in entries {
            r.respawn_at = past;
            mgr.push_respawn(map_id, 0, r);
        }
    }
}


