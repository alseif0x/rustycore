use super::*;

fn materialization_fixture(
    spawn_id: u64,
) -> (
    wow_map::Map,
    spawn_store_loader::CanonicalSpawnMetadataLikeCpp,
    LoadedGridCreatureRespawnCachesLikeCpp,
) {
    let metadata = fixtures::test_spawn_metadata_with_explicit_spawn_ids([(
        68,
        571,
        SpawnGroupFlags::NONE,
        spawn_id,
    )])
    .with_creature_runtime_rows_like_cpp(BTreeMap::from([(
        spawn_id,
        spawn_store_loader::CreatureSpawnRuntimeRowLikeCpp {
            spawn_id,
            model_id: 999,
            equipment_id: 3,
            wander_distance: 15.0,
            curhealth: 0,
            curmana: 0,
            movement_type: 1,
            npc_flags: None,
            unit_flags: None,
            unit_flags2: None,
            unit_flags3: None,
            ground_movement_type: wow_constants::CreatureGroundMovementType::Run as u8,
            swim_allowed: true,
            flight_movement_type: 0,
            rooted: false,
            chase_movement_type: wow_constants::CreatureChaseMovementType::Run as u8,
            random_movement_type: wow_constants::CreatureRandomMovementType::Walk as u8,
            interaction_pause_timer_ms:
                wow_entities::DEFAULT_CREATURE_INTERACTION_PAUSE_TIMER_MS_LIKE_CPP,
            string_id: "prepared-owned-grid".to_string(),
            spawn_time_secs: 120,
        },
    )]));
    let caches = fixtures::variable_loaded_grid_creature_respawn_caches_with_vehicle_id_and_difficulty_like_cpp(42, 0, 0);
    (wow_map::Map::new(571, 0, 0, 60_000), metadata, caches)
}

#[test]
fn shared_spawn_factory_materializes_once_before_owned_fresh_admission() {
    let (mut map, metadata, caches) = materialization_fixture(54985);
    let pending = prepare_spawn_creature(&mut map, 54985, &metadata, &caches)
        .unwrap()
        .unwrap();
    assert_eq!(map.map_object_count(), 0);
    let guid = pending.actor.guid();
    assert_eq!(guid.counter(), 1);
    assert_eq!(pending.actor.creature.spawn_id(), 54985);
    assert_eq!(pending.actor.creature.respawn_time(), 0);
    assert_eq!(
        pending.actor.create_data.level,
        pending.actor.creature.level()
    );
    assert_eq!(pending.actor.runtime_motion_master_ticks_like_cpp(), 0);
    let next = prepare_spawn_creature(&mut map, 54985, &metadata, &caches)
        .unwrap()
        .unwrap();
    assert_eq!(next.actor.guid().counter(), 2);
    let result = pending.admit(&mut map);
    assert!(matches!(
        result.primary,
        Ok(FreshCreatureActorAdmission::Inserted { .. })
    ));
    assert_eq!(
        map.creature_spawn_id_store_guids_like_cpp(54985),
        vec![guid]
    );
    assert_eq!(next.actor.creature.spawn_id(), 54985);
}

#[test]
fn missing_metadata_or_catalog_inputs_remain_original_loader_none_without_dummy_actor() {
    let (mut map, metadata, caches) = materialization_fixture(54986);
    assert!(
        prepare_spawn_creature(&mut map, 0, &metadata, &caches)
            .unwrap()
            .is_none()
    );
    let empty = fixtures::empty_loaded_grid_creature_respawn_caches_like_cpp();
    assert!(
        prepare_spawn_creature(&mut map, 54986, &metadata, &empty)
            .unwrap()
            .is_none()
    );
    assert_eq!(map.map_object_count(), 0);
}

#[test]
fn catalog_requires_timer_while_grid_event_pool_condition_spawn_does_not() {
    let (mut map, metadata, caches) = materialization_fixture(54987);
    assert!(
        catalog_prepared::prepare_catalog_creature_respawn(&mut map, 54987, &metadata, &caches,)
            .unwrap()
            .is_none()
    );
    assert!(
        prepare_spawn_creature(&mut map, 54987, &metadata, &caches)
            .unwrap()
            .is_some()
    );
    assert_eq!(map.map_object_count(), 0);
}

#[test]
fn catalog_factory_preserves_map_owned_respawn_time_and_does_not_consume_timer() {
    let (mut map, metadata, caches) = materialization_fixture(54988);
    map.add_respawn_info_like_cpp(wow_map::RespawnInfoLikeCpp {
        object_type: SpawnObjectType::Creature,
        spawn_id: 54988,
        entry: 42,
        respawn_time: 12345,
        grid_id: 0,
    });
    let pending =
        catalog_prepared::prepare_catalog_creature_respawn(&mut map, 54988, &metadata, &caches)
            .unwrap()
            .unwrap();
    assert_eq!(pending.actor.creature.respawn_time(), 12345);
    assert_eq!(pending.actor.creature.spawn_id(), 54988);
    assert_eq!(pending.actor.guid().counter(), 1);
    assert_eq!(
        map.get_respawn_time_like_cpp(SpawnObjectType::Creature, 54988),
        12345
    );
    assert_eq!(map.map_object_count(), 0);
    assert!(matches!(
        pending.admit(&mut map).primary,
        Ok(FreshCreatureActorAdmission::Inserted { .. })
    ));
    assert_eq!(
        map.get_respawn_time_like_cpp(SpawnObjectType::Creature, 54988),
        12345
    );
}
