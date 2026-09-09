//! Spawn scenarios for [`super`].
//!
//! Split out of map_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

#[test]
fn dynamic_respawn_unsupported_mode_is_safe_noop() {
    let mut context = dynamic_respawn_context(Some(SpawnObjectType::GameObject));
    context.mode = 2;

    assert_dynamic_respawn_noop(context, DynamicRespawnScalingNoopReason::UnsupportedMode);
}
#[test]
fn despawn_all_by_spawn_id_queues_and_defers_physical_removal_like_cpp() {
    let mut map = test_map();
    let spawn_id = 41905;
    let mut creature = test_creature_for_spawn(spawn_id, 4190501, true);
    let guid = creature.guid();
    creature
        .unit_mut()
        .world_mut()
        .object_mut()
        .remove_from_world();
    map.add_map_object_record_to_map_like_cpp(MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();

    let outcome = map.despawn_all_by_spawn_id_like_cpp(SpawnObjectType::Creature, spawn_id);

    assert_eq!(outcome.queued, 1);
    assert_eq!(outcome.removed, 0);
    assert_eq!(map.objects_to_remove_count_like_cpp(), 1);
    assert!(map.map_object_record(guid).is_some());
    assert_eq!(map.creature_spawn_id_store_count_like_cpp(spawn_id), 1);

    let drain = map.remove_all_objects_in_remove_list_like_cpp();
    assert_eq!(drain.removed, 1);
    assert!(map.map_object_record(guid).is_none());
    assert_eq!(map.creature_spawn_id_store_count_like_cpp(spawn_id), 0);
}
