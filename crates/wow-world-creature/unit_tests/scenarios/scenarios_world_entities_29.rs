//! Session scenarios exercising the represented world entities responsibility.
//!
//! Split out of session_tests.rs under #626; assertions and registrations
//! are unchanged and the shared fixtures stay in the parent module.

use super::*;

#[test]
fn legacy_creature_melee_los_gate_delegates_to_map_environment_like_cpp() {
    let attacker = melee_los_test_world_object(
        test_creature_guid(91_026),
        TypeId::Unit,
        wow_constants::TypeMask::UNIT,
        Position::new(10.0, 10.0, 0.0, 0.0),
    );
    let victim = melee_los_test_world_object(
        ObjectGuid::create_player(1, 91_027),
        TypeId::Player,
        wow_constants::TypeMask::PLAYER,
        Position::new(11.0, 10.0, 0.0, 0.0),
    );

    assert!(!is_creature_melee_los_clear_like_cpp(
        &attacker,
        &victim,
        &CreatureMeleeLosTestEnvironment { los: false },
    ));
    assert!(is_creature_melee_los_clear_like_cpp(
        &attacker,
        &victim,
        &CreatureMeleeLosTestEnvironment { los: true },
    ));
}

#[test]
fn legacy_creature_lifecycle_tick_once_respawns_ready_queue_and_syncs_canonical_like_cpp() {
    let manager = shared_map_manager();
    let canonical = Arc::clone(&manager);
    canonical.lock().unwrap().create_world_map(0, 0);

    let now = Instant::now();
    let guid = test_creature_guid(90_011);
    let mut world_creature = crate::creature_projection::new_runtime_creature_like_cpp(
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
        .unit_mut()
        .world_mut()
        .phase_shift_mut()
        .add_phase_like_cpp(77, wow_constants::PhaseFlags::empty(), 1);
    world_creature.ai_ownership_mut().phase_id = 77;
    world_creature.set_spawn_id(90_011);
    let mut pending = crate::creature_respawn::pending_respawn_from_creature_like_cpp(
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
        let mut guard = manager.lock().unwrap();
        guard.save_pending_respawn_time_like_cpp(0, 0, &pending, now, unix_now());
        guard.push_respawn(0, 0, pending);
    }

    let outcome = run_canonical_creature_lifecycle_phase_for_test_like_cpp(
        &canonical,
        &lifecycle_test_map_store_like_cpp(0, wow_data::map::MAP_COMMON, 0),
        now,
    );
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
        let guard = manager.lock().unwrap();
        let creature = guard
            .find_creature(0, 0, guid)
            .expect("ready respawn must re-add the legacy creature");
        assert_eq!(creature.position(), Position::new(5.0, 6.0, 7.0, 1.0));
        assert!(
            creature.phase_shift().has_phase_like_cpp(77),
            "resolved phase shift must survive session-free respawn"
        );
        let auras = &creature.unit().subsystems().auras;
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
