// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use super::*;
use wow_constants::DeathState;
use wow_core::{Position, guid::HighGuid};
use wow_entities::{Creature, GameObject, OwnedLootAuthorityLifecycle, ReactState};

fn creature_record(counter: i64, spawn_id: u64) -> MapObjectRecord {
    let mut creature = Creature::new(false);
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 7, 42, counter);
    creature.unit_mut().world_mut().object_mut().create(guid);
    creature.unit_mut().world_mut().set_map(571, 7).unwrap();
    creature.unit_mut().set_max_health(100);
    creature.unit_mut().set_health(75);
    creature.set_spawn_id(spawn_id);
    MapObjectRecord::new_creature(creature).unwrap()
}

#[test]
fn snapshot_moves_complete_state_and_reindexes_without_detaching_shared_loot() {
    let mut map = Map::new(571, 7, 1, 1000);
    let original = creature_record(81, 810);
    let guid = original.object().guid();
    let loot = original.creature().unwrap().loot_authority_like_cpp().clone();
    let timeline = original.creature().unwrap().unit().health_state_revision_authority_like_cpp();
    map.insert_map_object_record(original).unwrap();

    // This is the real transport-snapshot operation, not a clone used to fake a move.
    let mut snapshot = map.with_creature_like_cpp(guid, Creature::clone).unwrap();
    snapshot.set_spawn_id(811);
    snapshot.set_respawn_time(12345);
    snapshot.set_respawn_delay(73);
    snapshot.set_react_state(ReactState::Passive);
    snapshot.unit_mut().set_max_health(200);
    snapshot.unit_mut().set_health(0);
    snapshot.unit_mut().set_death_state(DeathState::Corpse);
    snapshot.unit_mut().world_mut().relocate(Position::xyz(8.0, 9.0, 10.0));
    snapshot.unit_mut().world_mut().set_active(true);
    snapshot.unit_mut().subsystems_mut().auras.apply_transform_aura_like_cpp(118, false, None);
    snapshot.unit_mut().subsystems_mut().spells.set_cooldown(133, 500, 1500);
    let target = ObjectGuid::create_global(HighGuid::Player, 0, 99);
    snapshot.unit_mut().subsystems_mut().combat.initialize_threat_list_capability(true);
    snapshot.unit_mut().subsystems_mut().combat.add_threat(target, 17.0);
    let incoming = MapObjectRecord::new_creature(snapshot).unwrap();
    let incoming_pointer = incoming.creature().unwrap() as *const Creature;
    let incoming_revision = incoming.creature().unwrap().unit().health_state_revision_like_cpp();

    map.replace_creature_snapshot(incoming).unwrap();

    let current = map.get_typed_creature(guid).unwrap();
    assert_eq!(current as *const Creature, incoming_pointer);
    assert_eq!(current.current_health(), 0);
    assert_eq!(current.unit().data().max_health, 200);
    assert_eq!(current.unit().death_state(), DeathState::Corpse);
    assert_eq!(current.unit().world().position(), Position::xyz(8.0, 9.0, 10.0));
    assert!(current.unit().world().is_active());
    assert_eq!((current.respawn_time(), current.respawn_delay()), (12345, 73));
    assert_eq!(current.react_state(), ReactState::Passive);
    assert_eq!(current.unit().subsystems().auras.transform_spell_like_cpp(), 118);
    assert_eq!(current.unit().subsystems().spells.remaining_cooldown_ms(133, 0, 500), 1500);
    assert_eq!(current.unit().subsystems().combat.threat_value(target), Some(17.0));
    assert!(current.unit().shares_health_state_revision_authority_like_cpp(&timeline));
    assert_eq!(current.unit().health_state_revision_like_cpp(), incoming_revision);
    assert!(current.loot_authority_like_cpp().shares_storage_like_cpp(&loot));
    assert_ne!(loot.lifecycle_like_cpp(), OwnedLootAuthorityLifecycle::Detached);
    assert!(map.creature_spawn_id_store_guids_like_cpp(810).is_empty());
    assert_eq!(map.creature_spawn_id_store_guids_like_cpp(811), vec![guid]);
    assert_eq!(map.map_object_count(), 1);
}

#[test]
fn snapshot_with_distinct_loot_detaches_only_displaced_authority() {
    let mut map = Map::new(571, 7, 1, 1000);
    let original = creature_record(82, 820);
    let old_loot = original.creature().unwrap().loot_authority_like_cpp().clone();
    map.insert_map_object_record(original).unwrap();
    let incoming = creature_record(82, 821);
    let guid = incoming.object().guid();
    let new_loot = incoming.creature().unwrap().loot_authority_like_cpp().clone();
    let pointer = incoming.creature().unwrap() as *const Creature;
    map.replace_creature_snapshot(incoming).unwrap();
    assert_eq!(old_loot.lifecycle_like_cpp(), OwnedLootAuthorityLifecycle::Detached);
    assert_ne!(new_loot.lifecycle_like_cpp(), OwnedLootAuthorityLifecycle::Detached);
    assert_eq!(map.get_typed_creature(guid).unwrap() as *const Creature, pointer);
    assert!(map.creature_spawn_id_store_guids_like_cpp(820).is_empty());
    assert_eq!(map.creature_spawn_id_store_guids_like_cpp(821), vec![guid]);
}

#[test]
fn generic_incoming_creature_is_rejected_before_displacement() {
    let mut map = Map::new(571, 7, 1, 1000);
    let original = creature_record(83, 830);
    let guid = original.object().guid();
    let pointer = original.creature().unwrap() as *const Creature;
    let loot = original.creature().unwrap().loot_authority_like_cpp().clone();
    let generic = MapObjectRecord::new(AccessorObjectKind::Creature, original.object().clone()).unwrap();
    map.insert_map_object_record(original).unwrap();
    assert_eq!(map.replace_creature_snapshot(generic), Err(CreatureSnapshotReplaceError::NotExactCreature { guid }));
    assert_eq!(map.get_typed_creature(guid).unwrap() as *const Creature, pointer);
    assert_ne!(loot.lifecycle_like_cpp(), OwnedLootAuthorityLifecycle::Detached);
    assert_eq!(map.creature_spawn_id_store_guids_like_cpp(830), vec![guid]);
}

#[test]
fn missing_destination_is_rejected_without_inserting_or_detaching_incoming() {
    let mut map = Map::new(571, 7, 1, 1000);
    let incoming = creature_record(84, 840);
    let guid = incoming.object().guid();
    let loot = incoming.creature().unwrap().loot_authority_like_cpp().clone();
    assert_eq!(map.replace_creature_snapshot(incoming), Err(CreatureSnapshotReplaceError::NotExactCreature { guid }));
    assert_eq!(map.map_object_count(), 0);
    assert!(map.creature_spawn_id_store_guids_like_cpp(840).is_empty());
    assert_ne!(loot.lifecycle_like_cpp(), OwnedLootAuthorityLifecycle::Detached);
}

#[test]
fn generic_destination_remains_generic_after_rejected_snapshot() {
    let mut map = Map::new(571, 7, 1, 1000);
    let incoming = creature_record(85, 850);
    let guid = incoming.object().guid();
    let loot = incoming.creature().unwrap().loot_authority_like_cpp().clone();
    map.insert_map_object(AccessorObjectKind::Creature, incoming.object().clone()).unwrap();
    assert_eq!(map.replace_creature_snapshot(incoming), Err(CreatureSnapshotReplaceError::NotExactCreature { guid }));
    assert_eq!(map.map_object_count(), 1);
    assert!(map.get_creature(guid).is_some());
    assert!(map.get_typed_creature(guid).is_none());
    assert!(map.creature_spawn_id_store_guids_like_cpp(850).is_empty());
    assert_ne!(loot.lifecycle_like_cpp(), OwnedLootAuthorityLifecycle::Detached);
}

#[test]
fn wrong_kind_snapshot_does_not_displace_existing_gameobject() {
    let mut map = Map::new(571, 7, 1, 1000);
    let mut gameobject = GameObject::new();
    let guid = ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 7, 42, 86);
    gameobject.world_mut().object_mut().create(guid);
    gameobject.world_mut().set_map(571, 7).unwrap();
    let loot = gameobject.loot_authority_like_cpp().clone();
    map.insert_map_object_record(MapObjectRecord::new_game_object(gameobject).unwrap()).unwrap();
    let generic = MapObjectRecord::new(AccessorObjectKind::GameObject, map.get_game_object(guid).unwrap().clone()).unwrap();
    assert_eq!(map.replace_creature_snapshot(generic), Err(CreatureSnapshotReplaceError::NotExactCreature { guid }));
    assert!(map.get_typed_game_object(guid).is_some());
    assert_eq!(map.map_object_count(), 1);
    assert_ne!(loot.lifecycle_like_cpp(), OwnedLootAuthorityLifecycle::Detached);
}

#[test]
fn wrong_map_snapshot_is_rejected_before_loot_detach_or_index_changes() {
    let mut map = Map::new(571, 7, 1, 1000);
    let original = creature_record(87, 870);
    let guid = original.object().guid();
    let pointer = original.creature().unwrap() as *const Creature;
    let loot = original.creature().unwrap().loot_authority_like_cpp().clone();
    map.insert_map_object_record(original).unwrap();
    let mut incoming = creature_record(87, 871);
    incoming.object_mut().reset_map().unwrap();
    incoming.object_mut().set_map(530, 7).unwrap();
    assert!(matches!(map.replace_creature_snapshot(incoming), Err(CreatureSnapshotReplaceError::Store(MapObjectStoreError::WrongMap { .. }))));
    assert_eq!(map.get_typed_creature(guid).unwrap() as *const Creature, pointer);
    assert_ne!(loot.lifecycle_like_cpp(), OwnedLootAuthorityLifecycle::Detached);
    assert_eq!(map.creature_spawn_id_store_guids_like_cpp(870), vec![guid]);
    assert!(map.creature_spawn_id_store_guids_like_cpp(871).is_empty());
}

#[test]
fn creature_extractor_moves_full_entity_and_keeps_authority_identity() {
    let mut record = creature_record(88, 880);
    record.creature_mut().unwrap().set_respawn_time(321);
    record.creature_mut().unwrap().unit_mut().subsystems_mut().auras.apply_transform_aura_like_cpp(118, false, None);
    let guid = record.object().guid();
    let loot = record.creature().unwrap().loot_authority_like_cpp().clone();
    let timeline = record.creature().unwrap().unit().health_state_revision_authority_like_cpp();
    let creature = record.into_creature().unwrap();
    assert_eq!(creature.guid(), guid);
    assert_eq!((creature.spawn_id(), creature.current_health(), creature.respawn_time()), (880, 75, 321));
    assert_eq!(creature.unit().subsystems().auras.transform_spell_like_cpp(), 118);
    assert!(creature.loot_authority_like_cpp().shares_storage_like_cpp(&loot));
    assert!(creature.unit().shares_health_state_revision_authority_like_cpp(&timeline));
}

#[test]
fn creature_extractor_returns_wrong_body_intact() {
    let mut gameobject = GameObject::new();
    let guid = ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 7, 42, 89);
    gameobject.world_mut().object_mut().create(guid);
    gameobject.world_mut().set_map(571, 7).unwrap();
    let loot = gameobject.loot_authority_like_cpp().clone();
    let record = MapObjectRecord::new_game_object(gameobject).unwrap();
    let record = record.into_creature().unwrap_err();
    assert_eq!(record.kind(), AccessorObjectKind::GameObject);
    assert_eq!(record.object().guid(), guid);
    assert!(record.game_object().unwrap().loot_authority_like_cpp().shares_storage_like_cpp(&loot));
    assert_ne!(loot.lifecycle_like_cpp(), OwnedLootAuthorityLifecycle::Detached);
}

#[test]
fn creature_extractor_does_not_promote_generic_creature_body() {
    let typed = creature_record(90, 900);
    let guid = typed.object().guid();
    let record = MapObjectRecord::new(AccessorObjectKind::Creature, typed.into_object()).unwrap();
    let record = record.into_creature().unwrap_err();
    assert_eq!(record.kind(), AccessorObjectKind::Creature);
    assert_eq!(record.object().guid(), guid);
    assert_eq!((record.object().map_id(), record.object().instance_id()), (571, 7));
    assert!(record.creature().is_none());
}
