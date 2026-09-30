use super::*;

#[test]
fn legacy_creature_lifecycle_tick_once_respawns_ready_queue_and_syncs_canonical_like_cpp() {
    use crate::map_manager::{RuntimeTickOwner, pending_respawn_from_world_creature_like_cpp};

    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    canonical.lock().unwrap().create_world_map(0, 0);
    manager
        .write()
        .unwrap()
        .set_tick_owner(RuntimeTickOwner::GlobalLegacy);

    let now = Instant::now();
    let guid = test_creature_guid(90_011);
    let mut world_creature = crate::map_manager::WorldCreature::new(
        guid,
        9001,
        Position::new(5.0, 6.0, 7.0, 1.0),
        25,
        3,
        4,
        8,
        20.0,
        100,
        14,
        0,
        0,
    );
    world_creature
        .creature
        .unit_mut()
        .world_mut()
        .phase_shift_mut()
        .add_phase_like_cpp(77, wow_constants::PhaseFlags::empty(), 1);
    world_creature.creature.ai_ownership_mut().phase_id = 77;
    world_creature.creature.set_spawn_id(90_011);
    let mut pending = pending_respawn_from_world_creature_like_cpp(
        &world_creature,
        now - Duration::from_secs(1),
        0,
    );
    let addon_spell_id = 70_043;
    pending.addon = Some(wow_entities::CreatureAddonLifecycleRecordLikeCpp {
        auras: vec![addon_spell_id],
        aura_applications: vec![wow_entities::CreatureAddonAuraApplicationLikeCpp {
            spell_id: addon_spell_id,
            spell_visual_id: 7_043,
            effect_mask: 1,
            flags: 0,
            effects: vec![wow_entities::CreatureAddonAuraEffectLikeCpp {
                aura_type: wow_data::spell::aura_types::SPELL_AURA_MOD_DETECT_RANGE,
                amount: 5,
                misc_value: 0,
                effect_index: 0,
            }],
        }],
        ..Default::default()
    });
    {
        let grid = wow_map::compute_grid_coord(pending.home_pos.x, pending.home_pos.y);
        canonical
            .lock()
            .unwrap()
            .find_map_mut(0, 0)
            .unwrap()
            .map_mut()
            .add_respawn_info_like_cpp(wow_map::RespawnInfoLikeCpp {
                object_type: wow_map::SpawnObjectType::Creature,
                spawn_id: pending.spawn_id,
                entry: pending.create_data.entry,
                respawn_time: unix_now() + 1,
                grid_id: grid.get_id(),
            });
    }
    {
        let mut guard = manager.write().unwrap();
        guard.save_pending_respawn_time_like_cpp(0, 0, &pending, now, unix_now());
        guard.push_respawn(0, 0, pending);
    }

    let outcome = run_legacy_creature_lifecycle_tick_once_like_cpp(
        &manager,
        Some(&canonical),
        &lifecycle_test_map_store_like_cpp(0, wow_data::map::MAP_COMMON, 0),
        now,
    );

    assert!(!outcome.skipped_owner_not_global);
    assert_eq!(outcome.maps_seen, 1);
    assert_eq!(outcome.corpses_despawned, 0);
    assert_eq!(outcome.respawns_processed, 1);
    assert_eq!(outcome.respawn_db_mutations.len(), 1);
    assert!(matches!(
        outcome.respawn_db_mutations[0],
        wow_persistence::RespawnPersistenceMutationLikeCpp::Delete { .. }
    ));
    assert_eq!(outcome.canonical_removes, 0);
    assert_eq!(outcome.canonical_inserts, 1);
    assert_eq!(outcome.canonical_respawn_adds, 0);
    assert_eq!(outcome.canonical_respawn_removes, 1);
    assert_eq!(outcome.refresh_map_keys, vec![(0, 0)]);

    let legacy_addon_provenance = {
        let guard = manager.read().unwrap();
        let creature = guard
            .find_creature(0, 0, guid)
            .expect("ready respawn must re-add the legacy creature");
        assert_eq!(creature.position(), Position::new(5.0, 6.0, 7.0, 1.0));
        assert!(
            creature.phase_shift().has_phase_like_cpp(77),
            "resolved phase shift must survive session-free respawn"
        );
        let auras = &creature.creature.unit().subsystems().auras;
        let slot = auras
            .visible_auras
            .iter()
            .find_map(|(slot, aura)| (aura.spell_id == addon_spell_id).then_some(*slot))
            .expect("legacy respawn addon aura slot");
        let provenance = auras.aura_cast_provenance_like_cpp(slot);
        assert_eq!(provenance.cast_id.high_type(), HighGuid::Cast);
        assert_eq!(provenance.cast_id.entry(), addon_spell_id);
        assert_eq!(provenance.spell_visual_id, 7_043);
        assert_eq!(guard.respawn_queue_len(0, 0), 0);
        provenance
    };
    {
        let guard = canonical.lock().unwrap();
        let typed = guard
            .find_map(0, 0)
            .unwrap()
            .map()
            .with_creature_like_cpp(guid, Clone::clone)
            .expect("canonical creature must be inserted outside the legacy lock");
        assert!(
            typed.unit().world().phase_shift().has_phase_like_cpp(77),
            "canonical respawn must preserve the captured phase shift"
        );
        let auras = &typed.unit().subsystems().auras;
        let slot = auras
            .visible_auras
            .iter()
            .find_map(|(slot, aura)| (aura.spell_id == addon_spell_id).then_some(*slot))
            .expect("canonical respawn addon aura slot");
        let provenance = auras.aura_cast_provenance_like_cpp(slot);
        assert_eq!(provenance.cast_id.high_type(), HighGuid::Cast);
        assert_eq!(provenance.cast_id.entry(), addon_spell_id);
        assert_eq!(provenance.spell_visual_id, 7_043);
        assert_eq!(provenance, legacy_addon_provenance);
        assert_eq!(
            guard
                .find_map(0, 0)
                .unwrap()
                .map()
                .get_respawn_time_like_cpp(
                    wow_map::SpawnObjectType::Creature,
                    (guid.low_value() as u64) & 0xFF_FFFF_FFFF,
                ),
            0,
            "ready respawn must clear the canonical timer before the grid can reload stale state"
        );
    }
}

#[test]
fn legacy_creature_lifecycle_tick_once_clears_timer_when_canonical_won_like_cpp() {
    use crate::map_manager::{
        RuntimeTickOwner, pending_respawn_from_world_creature_like_cpp, world_to_grid_coords,
    };

    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    canonical.lock().unwrap().create_world_map(0, 0);
    manager
        .write()
        .unwrap()
        .set_tick_owner(RuntimeTickOwner::GlobalLegacy);

    let now = Instant::now();
    let queued_guid = test_creature_guid(90_012);
    let live_guid = test_creature_guid(90_013);
    let mut queued_world_creature = crate::map_manager::WorldCreature::new(
        queued_guid,
        9002,
        Position::new(8.0, 9.0, 10.0, 1.5),
        35,
        4,
        5,
        9,
        20.0,
        101,
        14,
        0,
        0,
    );
    queued_world_creature.creature.set_spawn_id(90_012);
    let pending = pending_respawn_from_world_creature_like_cpp(
        &queued_world_creature,
        now - Duration::from_secs(1),
        0,
    );
    let spawn_id = pending.spawn_id;
    let mut live_world_creature = crate::map_manager::WorldCreature::new(
        live_guid,
        9002,
        Position::new(8.0, 9.0, 10.0, 1.5),
        35,
        4,
        5,
        9,
        20.0,
        101,
        14,
        0,
        0,
    );
    live_world_creature.creature.set_spawn_id(spawn_id);

    {
        let mut guard = manager.write().unwrap();
        let (grid_x, grid_y) = world_to_grid_coords(
            live_world_creature.position().x,
            live_world_creature.position().y,
        );
        assert!(
            guard.add_creature(0, 0, grid_x, grid_y, live_world_creature),
            "test setup simulates canonical ProcessRespawns already mirroring the spawn under a regenerated GUID"
        );
        assert!(
            guard
                .save_pending_respawn_time_like_cpp(0, 0, &pending, now, unix_now())
                .is_some(),
            "test setup must leave the stale persisted timer that the ready queue clears"
        );
        guard.push_respawn(0, 0, pending);
        assert!(
            guard
                .persisted_respawn_time_like_cpp(
                    0,
                    0,
                    wow_map::SpawnObjectType::Creature,
                    spawn_id,
                )
                .is_some()
        );
    }

    let outcome = run_legacy_creature_lifecycle_tick_once_like_cpp(
        &manager,
        Some(&canonical),
        &lifecycle_test_map_store_like_cpp(0, wow_data::map::MAP_COMMON, 0),
        now,
    );

    assert!(!outcome.skipped_owner_not_global);
    assert_eq!(outcome.corpses_despawned, 0);
    assert_eq!(
        outcome.respawns_processed, 0,
        "legacy must not insert a duplicate when the creature is already present"
    );
    assert_eq!(outcome.respawn_db_mutations.len(), 1);
    assert!(matches!(
        outcome.respawn_db_mutations[0],
        wow_persistence::RespawnPersistenceMutationLikeCpp::Delete { .. }
    ));
    assert_eq!(outcome.canonical_inserts, 0);
    assert_eq!(outcome.canonical_respawn_removes, 0);
    assert_eq!(outcome.refresh_map_keys, vec![(0, 0)]);

    let guard = manager.read().unwrap();
    assert!(
        guard.find_creature(0, 0, live_guid).is_some(),
        "canonical-won creature must remain in legacy"
    );
    assert!(
        guard.find_creature(0, 0, queued_guid).is_none(),
        "the stale queued GUID must not be inserted when its persistent spawn is already live"
    );
    assert_eq!(guard.respawn_queue_len(0, 0), 0);
    assert_eq!(
        guard.persisted_respawn_time_like_cpp(0, 0, wow_map::SpawnObjectType::Creature, spawn_id,),
        None,
        "stale legacy persisted timer must be cleared before the next death"
    );
}

#[test]
fn legacy_creature_lifecycle_tick_once_ignores_dead_spawn_id_duplicate_like_cpp() {
    use crate::map_manager::{
        RuntimeTickOwner, pending_respawn_from_world_creature_like_cpp, world_to_grid_coords,
    };

    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    canonical.lock().unwrap().create_world_map(0, 0);
    manager
        .write()
        .unwrap()
        .set_tick_owner(RuntimeTickOwner::GlobalLegacy);

    let now = Instant::now();
    let queued_guid = test_creature_guid(90_017);
    let dead_guid = test_creature_guid(90_018);
    let spawn_id = 90_017;
    let mut queued = crate::map_manager::WorldCreature::new(
        queued_guid,
        9006,
        Position::new(20.0, 21.0, 22.0, 1.25),
        55,
        8,
        9,
        13,
        20.0,
        105,
        14,
        0,
        0,
    );
    queued.creature.set_spawn_id(spawn_id);
    let pending =
        pending_respawn_from_world_creature_like_cpp(&queued, now - Duration::from_secs(1), 0);

    let mut dead = crate::map_manager::WorldCreature::new(
        dead_guid,
        9006,
        Position::new(23.0, 24.0, 25.0, 1.5),
        55,
        8,
        9,
        13,
        20.0,
        105,
        14,
        0,
        0,
    );
    dead.creature.set_spawn_id(spawn_id);
    assert!(dead.take_damage(55));
    dead.creature.runtime_state_mut().save_respawn_requested = false;
    dead.set_corpse_despawn_at(Some(now + Duration::from_secs(60)));

    {
        let mut guard = manager.write().unwrap();
        let (grid_x, grid_y) = world_to_grid_coords(dead.position().x, dead.position().y);
        assert!(guard.add_creature(0, 0, grid_x, grid_y, dead));
        assert!(
            guard
                .save_pending_respawn_time_like_cpp(0, 0, &pending, now, unix_now())
                .is_some()
        );
        guard.push_respawn(0, 0, pending);
    }

    let outcome = run_legacy_creature_lifecycle_tick_once_like_cpp(
        &manager,
        Some(&canonical),
        &lifecycle_test_map_store_like_cpp(0, wow_data::map::MAP_COMMON, 0),
        now,
    );

    assert_eq!(outcome.respawns_processed, 1);
    assert_eq!(outcome.respawn_db_mutations.len(), 1);
    assert!(matches!(
        outcome.respawn_db_mutations[0],
        wow_persistence::RespawnPersistenceMutationLikeCpp::Delete { .. }
    ));
    let guard = manager.read().unwrap();
    assert!(guard.find_creature(0, 0, queued_guid).unwrap().is_alive());
    assert!(!guard.find_creature(0, 0, dead_guid).unwrap().is_alive());
    assert_eq!(guard.respawn_queue_len(0, 0), 0);
}

#[test]
fn legacy_creature_lifecycle_tick_once_respawns_synthetic_spawn_id_collision_like_cpp() {
    use crate::map_manager::{
        RuntimeTickOwner, pending_respawn_from_world_creature_like_cpp, world_to_grid_coords,
    };

    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    canonical.lock().unwrap().create_world_map(0, 0);
    manager
        .write()
        .unwrap()
        .set_tick_owner(RuntimeTickOwner::GlobalLegacy);

    let now = Instant::now();
    let synthetic_guid = test_creature_guid(90_014);
    let persistent_guid = test_creature_guid(90_015);
    let synthetic = crate::map_manager::WorldCreature::new(
        synthetic_guid,
        9003,
        Position::new(11.0, 12.0, 13.0, 0.5),
        40,
        5,
        6,
        10,
        20.0,
        102,
        14,
        0,
        0,
    );
    let pending =
        pending_respawn_from_world_creature_like_cpp(&synthetic, now - Duration::from_secs(1), 0);
    assert!(!pending.persistent_spawn);

    let mut persistent = crate::map_manager::WorldCreature::new(
        persistent_guid,
        9004,
        Position::new(14.0, 15.0, 16.0, 0.75),
        45,
        6,
        7,
        11,
        20.0,
        103,
        14,
        0,
        0,
    );
    persistent.creature.set_spawn_id(pending.spawn_id);

    {
        let mut guard = manager.write().unwrap();
        let (grid_x, grid_y) =
            world_to_grid_coords(persistent.position().x, persistent.position().y);
        assert!(guard.add_creature(0, 0, grid_x, grid_y, persistent));
        guard.push_respawn(0, 0, pending);
    }

    let outcome = run_legacy_creature_lifecycle_tick_once_like_cpp(
        &manager,
        Some(&canonical),
        &lifecycle_test_map_store_like_cpp(0, wow_data::map::MAP_COMMON, 0),
        now,
    );

    assert_eq!(outcome.respawns_processed, 1);
    assert!(outcome.respawn_db_mutations.is_empty());
    assert_eq!(outcome.canonical_inserts, 1);
    assert_eq!(outcome.canonical_respawn_removes, 0);

    let guard = manager.read().unwrap();
    let synthetic = guard
        .find_creature(0, 0, synthetic_guid)
        .expect("GUID-low collision with a DB spawn id must not discard a synthetic respawn");
    assert_eq!(synthetic.creature.spawn_id(), 0);
    assert!(guard.find_creature(0, 0, persistent_guid).is_some());
    assert_eq!(guard.respawn_queue_len(0, 0), 0);
}
