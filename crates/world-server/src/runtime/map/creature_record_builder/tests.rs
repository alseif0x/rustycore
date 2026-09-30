use super::*;
use std::{collections::BTreeMap, sync::Arc};
use wow_constants::{ConditionSourceType, ConditionType};
use wow_data::Condition;
use wow_map::spawn::{
    SpawnData, SpawnGroupFlags, SpawnGroupMemberRow, SpawnGroupTemplateData, SpawnPosition,
    SpawnStore,
};
use wow_map::{RespawnInfoLikeCpp, SpawnObjectType};

#[path = "../../../main_tests/loaded_grid_fixtures.rs"]
mod fixtures;

fn inputs(
    spawn_id: u64,
) -> (
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
            string_id: "owned-builder".to_string(),
            spawn_time_secs: 120,
        },
    )]));
    let mut caches = fixtures::variable_loaded_grid_creature_respawn_caches_with_vehicle_id_and_difficulty_like_cpp(42, 0, 0);
    // The two Maps own independent entropy-seeded RNG streams. Use a fixed
    // level row for their field comparison without changing production RNG.
    let mut template = caches.template_store.get(42).unwrap().clone();
    template.min_level = 18;
    template.max_level = 18;
    caches.template_store =
        Arc::new(wow_data::CreatureTemplateLifecycleStoreLikeCpp::from_templates([template]));
    (metadata, caches)
}

#[test]
fn owned_spawn_and_compatibility_facade_preserve_creation_fields_and_guid_consumption() {
    let (metadata, caches) = inputs(54990);
    let mut owned_map = wow_map::Map::new(571, 0, 0, 60_000);
    let mut compatibility_map = wow_map::Map::new(571, 0, 0, 60_000);
    let owned = build_creature_spawn_records(
        &mut owned_map,
        SpawnObjectType::Creature,
        54990,
        &metadata,
        &caches,
    )
    .unwrap()
    .unwrap();
    let compatibility = super::super::build_loaded_grid_creature_spawn_group_spawn_record_like_cpp(
        &mut compatibility_map,
        SpawnObjectType::Creature,
        54990,
        &metadata,
        &caches,
    )
    .unwrap();
    let created = owned.primary_record.creature().unwrap();
    let previous = compatibility.primary_record.creature().unwrap();
    assert_eq!(created.lifecycle_metadata(), previous.lifecycle_metadata());
    assert_eq!(created.guid(), previous.guid());
    assert_eq!(
        created.unit().world().position(),
        previous.unit().world().position()
    );
    assert_eq!(
        (
            created.level(),
            created.current_health(),
            created.max_health(),
            created.respawn_time()
        ),
        (
            previous.level(),
            previous.current_health(),
            previous.max_health(),
            previous.respawn_time()
        )
    );
    assert_eq!(
        owned_map.get_max_low_guid_like_cpp(HighGuid::Creature),
        Ok(2)
    );
    assert_eq!(
        compatibility_map.get_max_low_guid_like_cpp(HighGuid::Creature),
        Ok(2)
    );
    assert_eq!(owned_map.map_object_count(), 0);
    assert_eq!(compatibility_map.map_object_count(), 0);
}

#[test]
fn typed_catalog_requires_and_preserves_the_original_timer_before_materialization() {
    let (metadata, caches) = inputs(54991);
    let mut map = wow_map::Map::new(571, 0, 0, 60_000);
    assert!(
        build_creature_respawn_records(
            &mut map,
            SpawnObjectType::Creature,
            54991,
            &metadata,
            &caches,
        )
        .unwrap()
        .is_none()
    );
    assert_eq!(map.get_max_low_guid_like_cpp(HighGuid::Creature), Ok(1));
    map.add_respawn_info_like_cpp(RespawnInfoLikeCpp {
        object_type: SpawnObjectType::Creature,
        spawn_id: 54991,
        entry: 42,
        respawn_time: 12345,
        grid_id: 0,
    });
    let records = build_creature_respawn_records(
        &mut map,
        SpawnObjectType::Creature,
        54991,
        &metadata,
        &caches,
    )
    .unwrap()
    .unwrap();
    assert_eq!(
        records.primary_record.creature().unwrap().respawn_time(),
        12345
    );
    assert_eq!(
        map.get_respawn_time_like_cpp(SpawnObjectType::Creature, 54991),
        12345
    );
    assert_eq!(map.map_object_count(), 0);
}

#[test]
fn unavailable_inputs_and_wrong_object_type_remain_ok_none_without_allocations() {
    let (metadata, caches) = inputs(54992);
    let mut map = wow_map::Map::new(571, 0, 0, 60_000);
    assert!(
        build_creature_spawn_records(&mut map, SpawnObjectType::Creature, 0, &metadata, &caches,)
            .unwrap()
            .is_none()
    );
    assert!(
        build_creature_spawn_records(
            &mut map,
            SpawnObjectType::GameObject,
            54992,
            &metadata,
            &caches,
        )
        .unwrap()
        .is_none()
    );
    let empty = fixtures::empty_loaded_grid_creature_respawn_caches_like_cpp();
    assert!(
        build_creature_spawn_records(
            &mut map,
            SpawnObjectType::Creature,
            54992,
            &metadata,
            &empty,
        )
        .unwrap()
        .is_none()
    );
    assert_eq!(map.get_max_low_guid_like_cpp(HighGuid::Creature), Ok(1));
    assert_eq!(map.get_max_low_guid_like_cpp(HighGuid::Cast), Ok(1));
    assert_eq!(map.map_object_count(), 0);
}
