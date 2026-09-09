//! Combat scenarios for [`super`].
//!
//! Split out of map_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

#[test]
fn map_revalidates_all_typed_combat_refs_like_cpp_multi_owner_sweep() {
    let mut map = test_map();
    let alive_player_guid = guid(HighGuid::Player, 501);
    let dead_player_guid = guid(HighGuid::Player, 502);
    let creature_guid = guid(HighGuid::Creature, 503);

    let mut alive_player = Player::new(Some(7), false);
    alive_player
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(alive_player_guid);
    alive_player.unit_mut().world_mut().set_map(571, 7).unwrap();
    alive_player
        .unit_mut()
        .world_mut()
        .relocate(Position::xyz(10.0, 20.0, 30.0));
    alive_player
        .unit_mut()
        .world_mut()
        .object_mut()
        .add_to_world();

    let mut dead_player = Player::new(Some(7), false);
    dead_player
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(dead_player_guid);
    dead_player.unit_mut().world_mut().set_map(571, 7).unwrap();
    dead_player
        .unit_mut()
        .world_mut()
        .relocate(Position::xyz(11.0, 20.0, 30.0));
    dead_player
        .unit_mut()
        .world_mut()
        .object_mut()
        .add_to_world();
    dead_player.unit_mut().set_death_state(DeathState::Dead);

    let mut creature = Creature::new(false);
    creature
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(creature_guid);
    creature.unit_mut().world_mut().set_map(571, 7).unwrap();
    creature
        .unit_mut()
        .world_mut()
        .relocate(Position::xyz(12.0, 20.0, 30.0));
    creature.unit_mut().world_mut().object_mut().add_to_world();

    map.insert_map_object_record(MapObjectRecord::new_player(alive_player).unwrap())
        .unwrap();
    map.insert_map_object_record(MapObjectRecord::new_player(dead_player).unwrap())
        .unwrap();
    map.insert_map_object_record(MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();

    map.get_typed_player_mut(alive_player_guid)
        .unwrap()
        .unit_mut()
        .subsystems_mut()
        .combat
        .set_in_combat_with(creature_guid, false, false);
    map.get_typed_creature_mut(creature_guid)
        .unwrap()
        .unit_mut()
        .subsystems_mut()
        .combat
        .set_in_combat_with(alive_player_guid, false, false);
    map.get_typed_player_mut(dead_player_guid)
        .unwrap()
        .unit_mut()
        .subsystems_mut()
        .combat
        .set_in_combat_with(creature_guid, false, false);
    map.get_typed_creature_mut(creature_guid)
        .unwrap()
        .unit_mut()
        .subsystems_mut()
        .combat
        .set_in_combat_with(dead_player_guid, false, false);

    let invalid = map.revalidate_all_combat_refs_like_cpp();

    assert!(invalid.contains(&(dead_player_guid, creature_guid)));
    assert!(invalid.contains(&(creature_guid, dead_player_guid)));
    assert!(
        map.get_typed_player(alive_player_guid)
            .unwrap()
            .unit()
            .subsystems()
            .combat
            .is_in_combat_with(creature_guid)
    );
    assert!(
        map.get_typed_creature(creature_guid)
            .unwrap()
            .unit()
            .subsystems()
            .combat
            .is_in_combat_with(alive_player_guid)
    );
    assert!(
        !map.get_typed_player(dead_player_guid)
            .unwrap()
            .unit()
            .subsystems()
            .combat
            .is_in_combat_with(creature_guid)
    );
    assert!(
        !map.get_typed_creature(creature_guid)
            .unwrap()
            .unit()
            .subsystems()
            .combat
            .is_in_combat_with(dead_player_guid)
    );
}
#[test]
fn map_ticks_pvp_combat_refs_and_purges_reciprocal_like_cpp() {
    let mut map = test_map();
    let first_guid = guid(HighGuid::Creature, 504);
    let second_guid = guid(HighGuid::Creature, 505);

    for guid in [first_guid, second_guid] {
        let mut creature = Creature::new(false);
        creature.unit_mut().world_mut().object_mut().create(guid);
        creature.unit_mut().world_mut().set_map(571, 7).unwrap();
        creature.unit_mut().world_mut().object_mut().add_to_world();
        map.insert_map_object_record(MapObjectRecord::new_creature(creature).unwrap())
            .unwrap();
    }

    map.get_typed_creature_mut(first_guid)
        .unwrap()
        .unit_mut()
        .subsystems_mut()
        .combat
        .set_in_combat_with(second_guid, true, false);
    map.get_typed_creature_mut(second_guid)
        .unwrap()
        .unit_mut()
        .subsystems_mut()
        .combat
        .set_in_combat_with(first_guid, true, false);

    assert!(
        map.update_all_pvp_combat_refs_like_cpp(wow_entities::PVP_COMBAT_TIMEOUT_MS - 1)
            .is_empty()
    );
    let expired = map.update_all_pvp_combat_refs_like_cpp(1);
    assert!(!expired.is_empty());
    assert!(
        !map.get_typed_creature(first_guid)
            .unwrap()
            .unit()
            .subsystems()
            .combat
            .is_in_combat_with(second_guid)
    );
    assert!(
        !map.get_typed_creature(second_guid)
            .unwrap()
            .unit()
            .subsystems()
            .combat
            .is_in_combat_with(first_guid)
    );
}
