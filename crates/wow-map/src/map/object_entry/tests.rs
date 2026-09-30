// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Whole-value moves retain entity allocation and authority identity.

use super::*;
use crate::coords::MAX_NUMBER_OF_CELLS;
use crate::map::entity_world::EntityWorld;
use crate::map::{Map, cell_from_world};
use wow_core::{ObjectGuid, Position, guid::HighGuid};
use wow_entities::{
    Creature, HealthStateRevisionAuthorityLikeCpp, OwnedLootAuthority,
    OwnedLootAuthorityLifecycle, Player,
};

fn creature_entry(counter: i64, spawn_id: u64) -> ObjectEntry {
    let mut creature = Creature::new(false);
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 7, 42, counter);
    creature.unit_mut().world_mut().object_mut().create(guid);
    creature.unit_mut().world_mut().set_map(571, 7).unwrap();
    creature.unit_mut().world_mut().relocate(Position::xyz(1.0, 2.0, 3.0));
    creature.unit_mut().set_max_health(100);
    creature.unit_mut().set_health(75);
    creature.set_spawn_id(spawn_id);
    ObjectEntry::Record(MapObjectRecord::new_creature(creature).unwrap())
}

fn identity(creature: &Creature) -> (
    *const Creature,
    OwnedLootAuthority,
    HealthStateRevisionAuthorityLikeCpp,
    u64,
) {
    (
        creature as *const Creature,
        creature.loot_authority_like_cpp().clone(),
        creature.unit().health_state_revision_authority_like_cpp(),
        creature.unit().health_state_revision_like_cpp(),
    )
}

#[test]
fn entity_world_owned_round_trip_retains_allocation_and_authority_timeline() {
    let entry = creature_entry(61, 610);
    let guid = entry.as_ref().object().guid();
    let (pointer, loot, health, revision) = identity(entry.as_ref().creature().unwrap());
    let mut world = EntityWorld::default();
    assert!(world.insert(entry).is_none());
    let mut entry = world.take(&guid).unwrap();
    assert_eq!(world.len(), 0);
    assert_eq!(world.values().count(), 0);
    entry.as_mut().creature_mut().unwrap().unit_mut().set_health(25);
    assert!(world.insert(entry).is_none());
    assert_eq!(world.len(), 1);
    let creature = world.get(&guid).unwrap().creature().unwrap();
    assert_eq!(creature as *const Creature, pointer);
    assert!(creature.loot_authority_like_cpp().shares_storage_like_cpp(&loot));
    assert!(creature.unit().shares_health_state_revision_authority_like_cpp(&health));
    assert!(creature.unit().health_state_revision_like_cpp() > revision);
    assert_eq!(creature.current_health(), 25);
    let record = world.take(&guid).unwrap().into_record_fixture().unwrap();
    assert_eq!(record.creature().unwrap() as *const Creature, pointer);
    assert_eq!(world.iter().count(), 0);
}

#[test]
fn map_owned_take_and_public_round_trip_preserve_spawn_index_and_entity_identity() {
    let entry = creature_entry(62, 620);
    let guid = entry.as_ref().object().guid();
    let (pointer, loot, health, revision) = identity(entry.as_ref().creature().unwrap());
    let mut map = Map::new(571, 7, 1, 1000);
    assert!(map.insert_object_entry(entry).unwrap().is_none());
    assert_eq!(map.creature_spawn_id_store_guids_like_cpp(620), vec![guid]);
    let entry = map.take_object_entry(guid).unwrap();
    assert_eq!(map.map_object_count(), 0);
    assert!(map.creature_spawn_id_store_guids_like_cpp(620).is_empty());
    assert!(map.insert_map_object_record(entry.into_record_fixture().unwrap()).unwrap().is_none());
    let record = map.remove_map_object(guid).unwrap();
    let creature = record.record().unwrap().creature().unwrap();
    assert_eq!(creature as *const Creature, pointer);
    assert!(creature.loot_authority_like_cpp().shares_storage_like_cpp(&loot));
    assert!(creature.unit().shares_health_state_revision_authority_like_cpp(&health));
    assert_eq!(creature.unit().health_state_revision_like_cpp(), revision);
    assert_ne!(loot.lifecycle_like_cpp(), OwnedLootAuthorityLifecycle::Detached);
    assert!(map.creature_spawn_id_store_guids_like_cpp(620).is_empty());
}

#[test]
fn distinct_same_guid_replacement_returns_displaced_value_and_reindexes() {
    let entry = creature_entry(63, 630);
    let guid = entry.as_ref().object().guid();
    let (old_pointer, old_loot, _, _) = identity(entry.as_ref().creature().unwrap());
    let replacement = creature_entry(63, 631);
    let (new_pointer, new_loot, _, _) = identity(replacement.as_ref().creature().unwrap());
    let mut map = Map::new(571, 7, 1, 1000);
    map.insert_object_entry(entry).unwrap();
    let displaced = map.insert_map_object_record(replacement.into_record_fixture().unwrap()).unwrap().unwrap();
    assert_eq!(displaced.record().unwrap().creature().unwrap() as *const Creature, old_pointer);
    assert_eq!(old_loot.lifecycle_like_cpp(), OwnedLootAuthorityLifecycle::Detached);
    assert_ne!(new_loot.lifecycle_like_cpp(), OwnedLootAuthorityLifecycle::Detached);
    assert_eq!(map.get_typed_creature(guid).unwrap() as *const Creature, new_pointer);
    assert!(map.creature_spawn_id_store_guids_like_cpp(630).is_empty());
    assert_eq!(map.creature_spawn_id_store_guids_like_cpp(631), vec![guid]);
    assert_eq!(map.map_object_count(), 1);
}

#[test]
fn invalid_entry_is_rejected_before_displacement_or_authority_detach() {
    let entry = creature_entry(64, 640);
    let guid = entry.as_ref().object().guid();
    let (pointer, loot, _, _) = identity(entry.as_ref().creature().unwrap());
    let mut map = Map::new(571, 7, 1, 1000);
    map.insert_object_entry(entry).unwrap();
    let mut invalid = creature_entry(64, 641);
    invalid.as_mut().object_mut().reset_map().unwrap();
    invalid.as_mut().object_mut().set_map(530, 7).unwrap();
    assert!(map.insert_object_entry(invalid).is_err());
    assert_eq!(map.get_typed_creature(guid).unwrap() as *const Creature, pointer);
    assert_ne!(loot.lifecycle_like_cpp(), OwnedLootAuthorityLifecycle::Detached);
    assert_eq!(map.creature_spawn_id_store_guids_like_cpp(640), vec![guid]);
    assert!(map.creature_spawn_id_store_guids_like_cpp(641).is_empty());
}

#[test]
fn relocation_preserves_owned_identity_through_same_cell_cross_cell_and_grid_load() {
    let mut entry = creature_entry(65, 650);
    entry.as_mut().object_mut().set_active(true);
    let guid = entry.as_ref().object().guid();
    let mut map = Map::new(571, 7, 1, 1000);
    let added = map.add_object_entry_to_map(entry).unwrap();
    assert!(added.inserted_into_cell);
    let (pointer, loot, health, revision) = identity(map.get_typed_creature(guid).unwrap());
    for (position, moves_cell, loads_grid) in [
        (Position::xyz(2.0, 3.0, 4.0), false, false),
        (Position::xyz(90.0, 20.0, 5.0), true, false),
        (Position::xyz(700.0, 20.0, 5.0), true, true),
    ] {
        let outcome = map.relocate_map_object_like_cpp(guid, position).unwrap();
        assert!(outcome.relocated);
        assert_eq!(outcome.moved_between_cells, moves_cell);
        assert_eq!(outcome.loaded_grid, loads_grid);
        let creature = map.get_typed_creature(guid).unwrap();
        assert_eq!(creature as *const Creature, pointer);
        assert!(creature.loot_authority_like_cpp().shares_storage_like_cpp(&loot));
        assert!(creature.unit().shares_health_state_revision_authority_like_cpp(&health));
        assert_eq!(creature.unit().health_state_revision_like_cpp(), revision);
        assert_eq!(creature.unit().world().position(), position);
        assert_eq!(map.creature_spawn_id_store_guids_like_cpp(650), vec![guid]);
        assert_eq!(map.map_object_count(), 1);
        let cell = cell_from_world(position.x, position.y);
        let new_cell = map.get_ngrid(outcome.new_grid).unwrap()
            .get_grid_type(cell.cell_x(), cell.cell_y()).unwrap();
        assert!(new_cell.grid_objects.creatures.contains(&guid));
        if moves_cell {
            let old_cell = map.get_ngrid(outcome.old_grid).unwrap()
                .get_grid_type(
                    outcome.old_cell.x_coord % MAX_NUMBER_OF_CELLS,
                    outcome.old_cell.y_coord % MAX_NUMBER_OF_CELLS,
                ).unwrap();
            assert!(!old_cell.grid_objects.creatures.contains(&guid));
        }
    }
}

#[test]
fn blocked_relocations_keep_the_original_entry_authorities_and_spawn_membership() {
    let entry = creature_entry(66, 660);
    let guid = entry.as_ref().object().guid();
    let (pointer, loot, health, revision) = identity(entry.as_ref().creature().unwrap());
    let mut map = Map::new(571, 7, 1, 1000);
    map.insert_object_entry(entry).unwrap();
    // Same-grid target missing: the early fallback must not take the entry.
    let missing = map.relocate_map_object_like_cpp(guid, Position::xyz(90.0, 20.0, 5.0)).unwrap();
    assert!(!missing.relocated);
    assert!(missing.blocked_by_unloaded_grid);
    let entry = map.take_object_entry(guid).unwrap();
    let added = map.add_object_entry_to_map(entry).unwrap();
    // Distant unloaded grid: retain both the original body and old membership.
    let blocked = map.relocate_map_object_like_cpp(guid, Position::xyz(700.0, 20.0, 5.0)).unwrap();
    assert!(!blocked.relocated);
    assert!(blocked.blocked_by_unloaded_grid);
    let creature = map.get_typed_creature(guid).unwrap();
    assert_eq!(creature as *const Creature, pointer);
    assert!(creature.loot_authority_like_cpp().shares_storage_like_cpp(&loot));
    assert!(creature.unit().shares_health_state_revision_authority_like_cpp(&health));
    assert_eq!(creature.unit().health_state_revision_like_cpp(), revision);
    assert_eq!(creature.unit().world().position(), Position::xyz(1.0, 2.0, 3.0));
    assert_eq!(map.creature_spawn_id_store_guids_like_cpp(660), vec![guid]);
    let cell = cell_from_world(1.0, 2.0);
    assert!(map.get_ngrid(added.grid).unwrap()
        .get_grid_type(cell.cell_x(), cell.cell_y()).unwrap()
        .grid_objects.creatures.contains(&guid));
}

#[test]
fn player_entry_move_preserves_exact_value_and_map_reference_order() {
    let mut map = Map::new(571, 7, 1, 1000);
    let mut first_pointer = std::ptr::null();
    let mut guids = Vec::new();
    for counter in [67, 68] {
        let mut player = Player::new(Some(7), false);
        let guid = ObjectGuid::create_global(HighGuid::Player, 0, counter);
        player.unit_mut().world_mut().object_mut().create(guid);
        player.unit_mut().world_mut().set_map(571, 7).unwrap();
        player.unit_mut().world_mut().relocate(Position::xyz(1.0, 2.0, 3.0));
        let record = MapObjectRecord::new_player(player).unwrap();
        if guids.is_empty() {
            first_pointer = record.player().unwrap() as *const Player;
        }
        map.insert_object_entry(ObjectEntry::Record(record)).unwrap();
        guids.push(guid);
    }
    assert_eq!(map.map_reference_order_like_cpp(), &[guids[1], guids[0]]);
    let entry = map.take_object_entry(guids[0]).unwrap();
    assert_eq!(map.map_reference_order_like_cpp(), &[guids[1]]);
    assert_eq!(entry.as_ref().player().unwrap() as *const Player, first_pointer);
    map.insert_object_entry(entry).unwrap();
    assert_eq!(map.map_reference_order_like_cpp(), &[guids[0], guids[1]]);
    let outcome = map.remove_from_map_like_cpp(guids[0], false).unwrap();
    let detached = outcome.player.unwrap();
    assert_eq!(detached.as_ref() as *const Player, first_pointer);
    assert!(!detached.unit().world().has_current_map());
    assert_eq!(map.map_reference_order_like_cpp(), &[guids[1]]);
}
