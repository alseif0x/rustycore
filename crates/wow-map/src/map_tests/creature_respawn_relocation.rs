use super::*;

#[test]
fn blocked_creature_returns_home_stops_combat_and_marks_visibility() {
    let mut map = test_map();
    let mut creature = test_creature_for_spawn(0, 40801, true);
    let mut victim = test_creature_for_spawn(0, 40802, true);
    let guid = creature.guid();
    let victim_guid = victim.guid();
    let home = creature.position();
    creature.set_ai_home_position(home);
    creature.enter_combat(victim_guid);
    victim.unit_mut().add_attacker_like_cpp(guid);
    victim
        .unit_mut()
        .subsystems_mut()
        .combat
        .set_in_combat_with(guid, false, false);
    creature
        .unit_mut()
        .subsystems_mut()
        .combat
        .set_in_combat_with(victim_guid, false, false);
    for entry in [creature, victim] {
        map.add_map_object_record_to_map_like_cpp(MapObjectRecord::new_creature(entry).unwrap())
            .unwrap();
    }
    map.add_creature_to_move_list_like_cpp(guid, Position::xyz(3000.0, 3000.0, 0.0));
    let summary = map.move_all_creatures_in_move_list_like_cpp();
    assert_eq!(summary.respawn_relocated, 1);
    assert_eq!(summary.remove_list_queued, 0);
    let combat = map.take_relocation_combat();
    assert_eq!(combat.stops.len(), 1);
    assert_eq!(combat.stops[0].attacker, guid);
    assert_eq!(combat.stops[0].victim, victim_guid);
    assert_eq!(combat.stops[0].position, home);
    assert!(!combat.stops[0].victim_dead);
    assert!(map.take_relocation_combat().stops.is_empty());
    let creature = map.get_typed_creature(guid).unwrap();
    assert_eq!(creature.position(), home);
    assert_eq!(creature.ai_ownership().combat_target, None);
    assert_eq!(creature.unit().attacking(), None);
    assert!(
        creature
            .unit()
            .world()
            .object()
            .is_need_notify(ObjectNotifyFlags::VISIBILITY_CHANGED)
    );
    assert!(creature.active_chase_generator_like_cpp().is_none());
    assert!(
        !map.get_typed_creature(victim_guid)
            .unwrap()
            .unit()
            .subsystems()
            .combat
            .has_combat()
    );
}

#[test]
fn blocked_creature_with_unloaded_home_enters_the_remove_list() {
    let mut map = test_map();
    let mut creature = test_creature_for_spawn(0, 40803, true);
    let guid = creature.guid();
    creature.set_ai_home_position(Position::xyz(-3000.0, -3000.0, 0.0));
    map.add_map_object_record_to_map_like_cpp(MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();
    map.add_creature_to_move_list_like_cpp(guid, Position::xyz(3000.0, 3000.0, 0.0));
    let summary = map.move_all_creatures_in_move_list_like_cpp();
    assert_eq!(summary.remove_list_queued, 1);
    assert_eq!(summary.respawn_relocated, 0);
    map.remove_all_objects_in_remove_list_like_cpp();
    assert!(map.get_typed_creature(guid).is_none());
}

#[test]
fn player_owned_creature_loads_target_grid_without_respawn_relocation() {
    let mut map = test_map();
    let mut creature = test_creature_for_spawn(0, 40804, true);
    let mover = creature.guid();
    creature
        .unit_mut()
        .subsystems_mut()
        .control
        .set_owner_guid(Some(guid(HighGuid::Player, 40805)));
    map.add_map_object_record_to_map_like_cpp(MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();
    let target = Position::xyz(3000.0, 3000.0, 0.0);
    map.add_creature_to_move_list_like_cpp(mover, target);
    let summary = map.move_all_creatures_in_move_list_like_cpp();
    assert_eq!(summary.relocated, 1);
    assert_eq!(summary.respawn_relocated, 0);
    assert_eq!(map.get_typed_creature(mover).unwrap().position(), target);
}
