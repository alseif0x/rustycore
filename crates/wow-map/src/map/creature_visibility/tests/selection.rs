use super::*;
use crate::map_manager::{MapManager as LegacyMapManager, world_to_grid_coords};

#[test]
fn canonical_create_accepts_exact_creature_and_pet_but_message_excludes_pet() {
    let mut map = Map::new(571, 7, 1, 1000);
    let creature = creature(301, false);
    let creature_guid = creature.guid();
    map.add_map_object_record_to_map_like_cpp(MapObjectRecord::new_creature(creature).unwrap()).unwrap();
    let pet = pet(302);
    let pet_guid = pet.creature().guid();
    map.add_map_object_record_to_map_like_cpp(MapObjectRecord::new_pet(pet).unwrap()).unwrap();
    let captured = collect(&map);
    assert_eq!(captured.len(), 2);
    assert!(captured.iter().any(|candidate| candidate.guid() == creature_guid));
    assert!(captured.iter().any(|candidate| candidate.guid() == pet_guid));
    assert!(map.capture_compatible_creature_message_source(creature_guid).is_some());
    assert!(map.capture_compatible_creature_message_source(pet_guid).is_none());
}

#[test]
fn canonical_removed_object_is_not_a_create_candidate_but_remains_a_message_source() {
    let mut map = Map::new(571, 7, 1, 1000);
    let source = creature(303, false);
    let guid = source.guid();
    map.add_map_object_record_to_map_like_cpp(MapObjectRecord::new_creature(source).unwrap()).unwrap();
    map.with_creature_mut_like_cpp(guid, |creature| {
        creature.unit_mut().world_mut().object_mut().remove_from_world();
    }).unwrap();
    assert!(collect(&map).is_empty());
    assert_eq!(map.capture_compatible_creature_message_source(guid).unwrap().guid(), guid);
    assert!(map.capture_compatible_creature_message_source(ObjectGuid::EMPTY).is_none());
}

#[test]
fn canonical_prefilter_uses_phase_and_horizontal_distance_with_combat_reaches() {
    let mut map = Map::new(571, 7, 1, 1000);
    let mut source = creature(304, false);
    let guid = source.guid();
    source.set_ai_position(Position::xyz(20.0, 10.0, 10_000.0));
    *source.unit_mut().world_mut().phase_shift_mut() = PhaseShift::from_phases([10]);
    map.add_map_object_record_to_map_like_cpp(MapObjectRecord::new_creature(source).unwrap()).unwrap();
    let origin = Position::xyz(10.0, 10.0, 0.0);
    assert!(map.capture_compatible_creature_visibility(&origin, 8.0, 1.5,
        &PhaseShift::from_phases([20])).is_empty());
    let captured = map.capture_compatible_creature_visibility(&origin, 8.0, 1.5,
        &PhaseShift::from_phases([10]));
    assert_eq!(captured.len(), 1);
    assert_eq!(captured[0].guid(), guid);
    assert!(map.capture_compatible_creature_visibility(&origin, 1.0, 1.5,
        &PhaseShift::from_phases([10])).is_empty());
}

#[test]
fn legacy_facts_keep_3x3_dx_dy_order_and_old_snapshot_selection() {
    let mut manager = LegacyMapManager::new();
    let center = Position::xyz(10.0, 10.0, 0.0);
    let (x, y) = world_to_grid_coords(center.x, center.y);
    let mut expected = Vec::new();
    for (index, dx, dy) in [(401, -1, 1), (402, 0, -1), (403, 1, 0)] {
        let source = legacy(index);
        expected.push(source.guid());
        manager.add_creature(571, 7, x + dx, y + dy, source);
    }
    let outside = legacy(404);
    manager.add_creature(571, 7, x + 2, y, outside);
    let snapshots = manager.get_visible_creatures_in_phase(571, 7, 10.0, 10.0, 0.0, 100.0, None);
    let captured = manager.get_visible_creature_facts_in_phase(571, 7, 10.0, 10.0, 0.0, 100.0, None);
    assert_eq!(snapshots.iter().map(WorldCreature::guid).collect::<Vec<_>>(), expected);
    assert_eq!(captured.iter().map(CreatureVisibilityCandidate::guid).collect::<Vec<_>>(), expected);
}

#[test]
fn legacy_phase_range_boundary_and_nan_keep_existing_selection() {
    let mut manager = LegacyMapManager::new();
    let (x, y) = world_to_grid_coords(10.0, 10.0);
    let mut source = legacy(405);
    let guid = source.guid();
    source.creature.set_ai_position(Position::xyz(20.0, 10.0, 10_000.0));
    *source.creature.unit_mut().world_mut().phase_shift_mut() = PhaseShift::from_phases([10]);
    manager.add_creature(571, 7, x, y, source);
    let matching = PhaseShift::from_phases([10]);
    let wrong = PhaseShift::from_phases([20]);
    assert!(manager.get_visible_creature_facts_in_phase(571, 7, 10.0, 10.0, 0.0, 10.0, Some(&wrong)).is_empty());
    let captured = manager.get_visible_creature_facts_in_phase(571, 7, 10.0, 10.0, 0.0, 10.0, Some(&matching));
    assert_eq!(captured.len(), 1);
    assert_eq!(captured[0].guid(), guid);
    assert!(manager.get_visible_creature_facts_in_phase(571, 7, 10.0, 10.0, 0.0, 9.0, Some(&matching)).is_empty());
    assert!(manager.get_visible_creature_facts_in_phase(571, 7, 10.0, 10.0, 0.0, f32::NAN, Some(&matching)).is_empty());
}

#[test]
fn legacy_facts_lookup_does_not_cross_map_or_instance() {
    let mut manager = LegacyMapManager::new();
    let (x, y) = world_to_grid_coords(10.0, 10.0);
    manager.add_creature(571, 7, x, y, legacy(406));
    assert!(manager.get_visible_creature_facts_in_phase(571, 8, 10.0, 10.0, 0.0, 100.0, None).is_empty());
    assert!(manager.get_visible_creature_facts_in_phase(530, 7, 10.0, 10.0, 0.0, 100.0, None).is_empty());
}
