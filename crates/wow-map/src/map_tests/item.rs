//! Item scenarios for [`super`].
//!
//! Split out of map_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

#[test]
fn game_event_change_equip_or_model_missing_spawn_id_does_not_panic_like_cpp() {
    let mut map = test_map();

    let outcome = map.change_game_event_equip_or_model_by_spawn_id_like_cpp(158, 9, 123, false);

    assert_eq!(outcome.indexed_guids, 0);
    assert_eq!(outcome.live_creatures_mutated, 0);
    assert_eq!(outcome.model_validation_unavailable, 0);
}
#[test]
fn game_event_change_equip_or_model_model_gate_reports_unavailable_without_display_like_cpp() {
    let mut map = test_map();
    map.insert_map_object_record(
        MapObjectRecord::new_creature(test_creature_for_spawn(159, 15901, true)).unwrap(),
    )
    .unwrap();

    let outcome = map.change_game_event_equip_or_model_by_spawn_id_like_cpp(159, 4, 123, false);

    assert_eq!(outcome.indexed_guids, 1);
    assert_eq!(outcome.live_creatures_mutated, 1);
    assert_eq!(outcome.equipment_changed, 1);
    assert_eq!(outcome.display_changed, 0);
    assert_eq!(outcome.model_validation_unavailable, 1);
}
#[test]
fn game_event_change_equip_or_model_wrong_kind_index_entry_does_not_mutate_like_cpp() {
    let mut map = test_map();
    map.insert_map_object_record(
        MapObjectRecord::new_creature(test_creature_for_spawn(160, 16001, true)).unwrap(),
    )
    .unwrap();
    let guid = guid(HighGuid::Creature, 16001);
    if let Some(creature_index) = map.creatures_by_spawn_id.get_mut(&160) {
        creature_index.retain(|indexed_guid| *indexed_guid != guid);
    }
    map.creatures_by_spawn_id
        .entry(161)
        .or_default()
        .insert(guid);

    let outcome = map.change_game_event_equip_or_model_by_spawn_id_like_cpp(161, 9, 0, false);

    assert_eq!(outcome.indexed_guids, 1);
    assert_eq!(outcome.live_creatures_mutated, 0);
    assert_eq!(outcome.stale_index_or_wrong_kind, 1);
}
