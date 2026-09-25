//! Lifecycle and respawn fixtures.
//!
//! These builders retain the original session-test behavior and are
//! visible only within the parent `session::tests` subtree.

use super::*;

pub(in crate::session::tests) struct CreatureMeleeLosTestEnvironment {
    pub(in crate::session::tests) los: bool,
}

impl wow_entities::WorldObjectEnvironment for CreatureMeleeLosTestEnvironment {
    fn map_id(&self) -> u32 {
        0
    }

    fn instance_id(&self) -> u32 {
        0
    }

    fn visibility_range(&self) -> f32 {
        100.0
    }

    fn line_of_sight(&self, _query: wow_entities::LineOfSightQuery<'_>) -> bool {
        self.los
    }

    fn map_height(
        &self,
        _object: &wow_entities::WorldObject,
        _x: f32,
        _y: f32,
        _z: f32,
        _query: wow_entities::WorldObjectHeightQuery,
    ) -> f32 {
        wow_entities::INVALID_HEIGHT
    }

    fn floor_z(
        &self,
        _object: &wow_entities::WorldObject,
        _position: Position,
        _max_search_dist: f32,
    ) -> f32 {
        wow_entities::INVALID_HEIGHT
    }
}

pub(in crate::session::tests) fn melee_los_test_world_object(
    guid: ObjectGuid,
    type_id: TypeId,
    type_mask: wow_constants::TypeMask,
    position: Position,
) -> wow_entities::WorldObject {
    let mut object = wow_entities::WorldObject::new(true, type_id, type_mask);
    object.object_mut().create(guid);
    object.set_map(0, 0).unwrap();
    object.relocate(position);
    object.object_mut().add_to_world();
    object
}

pub(in crate::session::tests) fn lifecycle_test_map_store_like_cpp(
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

pub(in crate::session::tests) fn assert_instanceable_map_does_not_persist_respawn_like_cpp(
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

// ── Slice 4A.2b — respawn queue ownership migrated to MapInstance ──────

/// Prepare a dead creature whose corpse timer has already elapsed.
/// Returns the session and the creature GUID so the caller can drive
/// `run_creatures_tick` to trigger the despawn-then-respawn path.
pub(in crate::session::tests) fn setup_dead_creature_past_despawn(
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
        .mutate_world_creature(guid, |c| {
            c.take_damage(10);
            c.set_corpse_despawn_at(Some(past));
        })
        .unwrap();

    // Mark the creature as client-visible so the DESTROY packet is built.
    session.client_visible_guids_like_cpp.insert(guid);

    (session, send_rx, guid)
}

/// After despawn, force the pending respawn entry to be immediately ready
/// by rewriting its `respawn_at` to the past via the map's queue.
pub(in crate::session::tests) fn force_respawn_ready(session: &mut WorldSession) {
    let map_id = session.player_map_id_like_cpp();
    let past = Instant::now() - Duration::from_secs(1);
    // Drain whatever is in the queue, rewrite respawn_at, push back.
    if let Some(manager) = &session.map_manager {
        let mut mgr = manager.write().unwrap_or_else(|p| p.into_inner());
        let entries =
            mgr.drain_ready_respawns(map_id, 0, Instant::now() + Duration::from_secs(9999));
        for mut r in entries {
            r.respawn_at = past;
            mgr.push_respawn(map_id, 0, r);
        }
    }
}

// ── step_creature_movement_like_cpp unit tests (no WorldSession) ──────────

pub(in crate::session::tests) fn make_test_world_creature(
    guid: ObjectGuid,
) -> crate::map_manager::WorldCreature {
    crate::map_manager::WorldCreature::new(
        guid,
        9999,
        Position::new(10.0, 10.0, 0.0, 0.0),
        25,
        2,
        3,
        5,
        20.0,
        100,
        14,
        0,
        0,
    )
}
