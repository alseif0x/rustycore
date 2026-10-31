use super::*;
use std::sync::Arc;

fn empty_respawn_caches() -> LoadedGridCreatureRespawnCachesLikeCpp {
    LoadedGridCreatureRespawnCachesLikeCpp {
        realm_id: 1,
        template_store: Default::default(),
        sparring_store: Default::default(),
        difficulty_store: Default::default(),
        base_stats_store: Default::default(),
        chr_classes_store: Arc::new(
            wow_data::character_progression::ChrClassesStore::from_entries([]),
        ),
        power_type_store: Arc::new(
            wow_data::character_progression::PowerTypeStore::from_entries([]),
        ),
        health_rates: Default::default(),
        display_store: Arc::new(wow_data::CreatureDisplayInfoStore::from_entries([])),
        model_store: Arc::new(wow_data::CreatureModelDataStore::from_entries([])),
        model_info_store: Arc::new(wow_data::CreatureModelInfoStoreLikeCpp::from_entries([])),
        creature_equipment_store: Default::default(),
        creature_addon_store: Default::default(),
        spell_x_spell_visual_store: Arc::new(wow_data::SpellXSpellVisualStore::from_entries([])),
        vehicle_store: Arc::new(wow_data::VehicleStore::from_entries([])),
        vehicle_seat_store: Arc::new(wow_data::VehicleSeatStore::from_entries([])),
        vehicle_accessory_store: Arc::new(wow_data::VehicleAccessoryStoreLikeCpp::from_parts(
            [],
            [],
        )),
        gameobject_template_store: Default::default(),
        gameobject_override_store: Default::default(),
    }
}

#[test]
fn map_session_pass_player_recovery_prelude_db_intents_survive_abandon_without_replay() {
    let map_key = wow_map::MapKey::new(571, 0);
    let mut manager = wow_map::MapManager::new(wow_map::MIN_GRID_DELAY_MS, 10);
    manager.create_world_map(map_key.map_id, map_key.instance_id);
    let plan =
        canonical_map_tick_begin_like_cpp(&mut manager, 100).expect("canonical map tick begins");
    let spawn_metadata = crate::spawn_store_loader::CanonicalSpawnMetadataLikeCpp::default();
    let conditions = wow_data::ConditionEntriesByTypeStore::default();
    let maps = wow_data::MapStore::from_entries(std::iter::empty());
    let caches = empty_respawn_caches();
    let mut scheduler = CanonicalRespawnConditionSchedulerLikeCpp::new(1_000);
    let mut prelude = prepare_map_recovery_prelude(
        &mut manager,
        plan.plan,
        &mut scheduler,
        &spawn_metadata,
        &conditions,
        &maps,
        &caches,
    )
    .expect("respawn prelude owns the admitted plan");

    let save_key = wow_persistence::RespawnPersistenceKeyLikeCpp {
        object_type_raw: wow_map::SpawnObjectType::Creature as u16,
        spawn_id: 701,
        map_id: u16::try_from(map_key.map_id).expect("fixture map ID fits persistence key"),
        instance_id: map_key.instance_id,
    };
    let delete_key = wow_persistence::RespawnPersistenceKeyLikeCpp {
        object_type_raw: wow_map::SpawnObjectType::GameObject as u16,
        spawn_id: 702,
        map_id: u16::try_from(map_key.map_id).expect("fixture map ID fits persistence key"),
        instance_id: map_key.instance_id,
    };
    prelude
        .summary_mut()
        .respawn_db_saves
        .push(super::super::RespawnDbSaveLikeCpp {
            object_type: wow_map::SpawnObjectType::Creature,
            spawn_id: save_key.spawn_id,
            respawn_time: 1234,
            map_id: save_key.map_id,
            instance_id: save_key.instance_id,
            mutation: wow_persistence::RespawnPersistenceMutationLikeCpp::Save {
                key: save_key,
                respawn_time: 1234,
            },
        });
    prelude
        .summary_mut()
        .respawn_db_deletes
        .push(super::super::RespawnDbDeleteLikeCpp {
            object_type: wow_map::SpawnObjectType::GameObject,
            spawn_id: delete_key.spawn_id,
            map_id: delete_key.map_id,
            instance_id: delete_key.instance_id,
            mutation: wow_persistence::RespawnPersistenceMutationLikeCpp::Delete {
                key: delete_key,
            },
        });

    let writer = RespawnDbWriterSenderLikeCpp::new_like_cpp();
    submit_respawn_db_mutations(prelude.summary_mut(), &writer);
    assert!(prelude.summary().respawn_db_saves.is_empty());
    assert!(prelude.summary().respawn_db_deletes.is_empty());

    // A second call cannot replay drained prelude work. The queue remains
    // producer-owned across the later tick abandonment; this test performs no
    // database I/O.
    submit_respawn_db_mutations(prelude.summary_mut(), &writer);
    manager.abandon_tick_like_cpp(prelude.into_plan());
    let state = writer
        .mailbox
        .state
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    assert_eq!(state.queue.pending_len(), 2);
    assert!(state.queue.pending.contains_key(&save_key));
    assert!(state.queue.pending.contains_key(&delete_key));
    assert_eq!(
        manager.tick_coordination_like_cpp(),
        wow_map::MapTickCoordinationStateLikeCpp::Idle
    );
}
