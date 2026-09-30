use super::*;

fn creature(mapped: bool) -> Creature {
    let mut creature = Creature::new(false);
    creature.unit_mut().world_mut().object_mut().create(ObjectGuid::create_creature_like_cpp(1, 571, 42, 81));
    if mapped { creature.unit_mut().world_mut().set_map(571, 9).unwrap(); }
    creature.unit_mut().world_mut().relocate(Position::xyz(1.0, 2.0, 3.0));
    creature.unit_mut().set_max_health(100);
    creature.unit_mut().set_health(75);
    creature.set_spawn_id(81);
    creature.set_spawn_string_id_runtime_like_cpp(Some("single-created-creature-owned-buffer".to_string()));
    creature
}

#[test]
fn record_projection_moves_the_same_string_allocation_and_health_loot_authorities() {
    let creature = creature(true);
    // A Creature clone would allocate a second String. An owned move preserves
    // this heap pointer even though the outer value moves from stack into Box.
    let string_pointer = creature.lifecycle_metadata().string_id.as_ref().unwrap().as_ptr();
    let health = creature.unit().health_state_revision_authority_like_cpp();
    let loot = creature.loot_authority_like_cpp().clone();
    let record = project_creature_record(creature, true).unwrap().unwrap();
    let moved = record.creature().unwrap();
    assert_eq!(moved.lifecycle_metadata().string_id.as_ref().unwrap().as_ptr(), string_pointer);
    assert!(moved.unit().shares_health_state_revision_authority_like_cpp(&health));
    assert!(moved.loot_authority_like_cpp().shares_storage_like_cpp(&loot));
    assert_eq!((moved.current_health(), moved.max_health(), moved.spawn_id()), (75, 100, 81));
}

#[test]
fn record_projection_skips_validation_when_insertion_was_not_requested() {
    assert!(project_creature_record(creature(false), false).unwrap().is_none());
}

#[test]
fn record_projection_preserves_original_typed_constructor_error_format() {
    let expected = MapObjectRecord::new_creature(creature(false)).unwrap_err();
    assert_eq!(project_creature_record(creature(false), true).unwrap_err(),
        CreatureLoadedGridResolveErrorLikeCpp::MapObjectRecord(format!("{expected:?}")));
}
