// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Dormant actor storage moves the real motor; no production actor admission.

use super::*;
use rand::rngs::StdRng;
use wow_constants::DeathState;
use wow_core::{ObjectGuid, Position, guid::HighGuid};
use wow_entities::{Creature, ObjectAccessorError, OwnedLootAuthorityLifecycle};

fn actor_fixture(counter: i64, spawn_id: u64, point: bool) -> (WorldCreature, StdRng) {
    let mut creature = Creature::new(false);
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 7, 42, counter);
    creature.unit_mut().world_mut().object_mut().create(guid);
    creature.unit_mut().world_mut().set_map(571, 7).unwrap();
    creature
        .unit_mut()
        .world_mut()
        .relocate(Position::xyz(1.0, 2.0, 3.0));
    creature.unit_mut().set_max_health(100);
    creature.unit_mut().set_health(75);
    creature.set_spawn_id(spawn_id);
    creature
        .unit_mut()
        .subsystems_mut()
        .auras
        .apply_transform_aura_like_cpp(118, false, None);
    let data = WorldCreature::create_data_from_canonical_like_cpp(&creature);
    // Construct the initial actor once. All following ownership operations move it.
    let mut actor = WorldCreature::from_canonical(creature, data);
    actor.create_data.npc_flags = 0x1234;
    actor.create_data.scale = 1.75;
    let rng = actor.seed_actor_storage_runtime(point);
    (actor, rng)
}

fn actor(entry: &ObjectEntry) -> &WorldCreature {
    match entry {
        ObjectEntry::CreatureActor(actor) => actor.actor(),
        ObjectEntry::Record(_) => panic!("actor fixture must retain its owned variant"),
    }
}

fn actor_mut(entry: &mut ObjectEntry) -> &mut WorldCreature {
    match entry {
        ObjectEntry::CreatureActor(actor) => actor.actor_mut(),
        ObjectEntry::Record(_) => panic!("actor fixture must retain its owned variant"),
    }
}

fn inspect_runtime(
    map: &mut Map,
    guid: ObjectGuid,
    expected_rng: &mut StdRng,
    point: bool,
    pointer: *const WorldCreature,
) {
    let mut entry = map.take_object_entry(guid).unwrap();
    assert_eq!(actor(&entry) as *const WorldCreature, pointer);
    actor_mut(&mut entry).assert_actor_storage_runtime(expected_rng, point);
    assert_eq!(actor(&entry).create_data.npc_flags, 0x1234);
    assert_eq!(actor(&entry).create_data.scale, 1.75);
    assert!(map.insert_object_entry(entry).unwrap().is_none());
}

#[test]
fn actor_constructor_rejects_in_original_priority_and_returns_the_complete_motor() {
    for case in 0..4 {
        let (mut live, mut rng) = actor_fixture(101 + case, 1010, false);
        if case == 0 {
            live.creature.unit_mut().world_mut().reset_map().unwrap();
            live.creature
                .unit_mut()
                .world_mut()
                .object_mut()
                .create(ObjectGuid::EMPTY);
        } else if case == 1 {
            live.creature
                .unit_mut()
                .world_mut()
                .object_mut()
                .create(ObjectGuid::EMPTY);
        } else {
            if case == 2 {
                live.creature
                    .unit_mut()
                    .world_mut()
                    .object_mut()
                    .create(ObjectGuid::create_global(HighGuid::Player, 0, 102));
            }
            live.creature.unit_mut().world_mut().reset_map().unwrap();
            live.creature
                .unit_mut()
                .world_mut()
                .set_map(530, 7)
                .unwrap();
        }
        let loot = live.creature.loot_authority_like_cpp().clone();
        let map = Map::new(571, 7, 1, 1000);
        let (error, mut returned) = map.creature_actor_entry(live).unwrap_err();
        match case {
            0 => assert!(matches!(
                error,
                MapObjectStoreError::InvalidRecord(ObjectAccessorError::ObjectHasNoMap { .. })
            )),
            1 => assert!(matches!(
                error,
                MapObjectStoreError::InvalidRecord(ObjectAccessorError::UnsupportedGuidKind { .. })
            )),
            2 => assert!(matches!(
                error,
                MapObjectStoreError::InvalidRecord(ObjectAccessorError::WrongGuidKind {
                    expected: AccessorObjectKind::Creature,
                    ..
                })
            )),
            3 => assert!(matches!(error, MapObjectStoreError::WrongMap { .. })),
            _ => unreachable!(),
        }
        returned.assert_actor_storage_runtime(&mut rng, false);
        assert!(
            returned
                .creature
                .loot_authority_like_cpp()
                .shares_storage_like_cpp(&loot)
        );
        assert_ne!(
            loot.lifecycle_like_cpp(),
            OwnedLootAuthorityLifecycle::Detached
        );
        assert_eq!(returned.create_data.npc_flags, 0x1234);
        assert_eq!(map.map_object_count(), 0);
    }
}

#[test]
fn actor_move_reinsert_retains_box_motor_authorities_and_one_guid_entry() {
    for point in [false, true] {
        let (live, mut rng) = actor_fixture(105, 1050, point);
        let mut map = Map::new(571, 7, 1, 1000);
        let mut entry = map.creature_actor_entry(live).unwrap();
        let guid = entry.as_ref().object().guid();
        let pointer = actor(&entry) as *const WorldCreature;
        let creature_pointer = entry.as_ref().creature().unwrap() as *const Creature;
        let spline = format!("{:?}", actor(&entry).active_move_spline_like_cpp());
        let loot = actor(&entry).creature.loot_authority_like_cpp().clone();
        let timeline = actor(&entry)
            .creature
            .unit()
            .health_state_revision_authority_like_cpp();
        actor_mut(&mut entry).assert_actor_storage_runtime(&mut rng, point);
        assert!(map.insert_object_entry(entry).unwrap().is_none());
        let mut entry = map.take_object_entry(guid).unwrap();
        assert_eq!(map.entity_world.iter().count(), 0);
        assert!(map.creature_spawn_id_store_guids_like_cpp(1050).is_empty());
        entry
            .as_mut()
            .creature_mut()
            .unwrap()
            .unit_mut()
            .set_health(25);
        entry.as_mut().creature_mut().unwrap().set_spawn_id(1051);
        actor_mut(&mut entry).assert_actor_storage_runtime(&mut rng, point);
        assert_eq!(
            format!("{:?}", actor(&entry).active_move_spline_like_cpp()),
            spline
        );
        assert!(map.insert_object_entry(entry).unwrap().is_none());
        assert_eq!(
            map.entity_world
                .iter()
                .map(|(guid, _)| *guid)
                .collect::<Vec<_>>(),
            vec![guid]
        );
        assert_eq!(map.entity_world.values().count(), 1);
        let current = map.get_typed_creature(guid).unwrap();
        assert_eq!(current as *const Creature, creature_pointer);
        assert_eq!(current.current_health(), 25);
        assert_eq!(
            current.unit().subsystems().auras.transform_spell_like_cpp(),
            118
        );
        assert!(
            current
                .unit()
                .shares_health_state_revision_authority_like_cpp(&timeline)
        );
        assert!(
            current
                .loot_authority_like_cpp()
                .shares_storage_like_cpp(&loot)
        );
        assert_eq!(map.creature_spawn_id_store_guids_like_cpp(1051), vec![guid]);
        inspect_runtime(&mut map, guid, &mut rng, point, pointer);
    }
}

#[test]
fn actor_views_keep_all_typed_gates_and_sequential_mutable_borrows() {
    let (live, _) = actor_fixture(106, 1060, true);
    let map = Map::new(571, 7, 1, 1000);
    let mut entry = map.creature_actor_entry(live).unwrap();
    let view = entry.as_ref();
    assert_eq!(view.kind(), AccessorObjectKind::Creature);
    assert!(view.is_unit_owner());
    assert_eq!(view.charmer_guid(), None);
    assert_eq!(
        [
            view.area_trigger().is_some(),
            view.conversation().is_some(),
            view.corpse().is_some(),
            view.creature().is_some(),
            view.dynamic_object().is_some(),
            view.game_object().is_some(),
            view.pet().is_some(),
            view.player().is_some(),
            view.scene_object().is_some(),
            view.transport().is_some(),
        ],
        [
            false, false, false, true, false, false, false, false, false, false
        ]
    );
    fn projected(entry: &ObjectEntry) -> &Creature {
        entry.as_ref().creature().unwrap()
    }
    assert!(std::ptr::eq(projected(&entry), &actor(&entry).creature));
    let mut view = entry.as_mut();
    assert!(view.reborrow().area_trigger_mut().is_none());
    assert!(view.reborrow().conversation_mut().is_none());
    assert!(view.reborrow().corpse_mut().is_none());
    assert!(view.reborrow().dynamic_object_mut().is_none());
    assert!(view.reborrow().game_object_mut().is_none());
    assert!(view.reborrow().pet_mut().is_none());
    assert!(view.reborrow().player_mut().is_none());
    assert!(view.reborrow().scene_object_mut().is_none());
    assert!(view.reborrow().transport_mut().is_none());
    view.reborrow().object_mut().set_active(true);
    view.reborrow()
        .creature_mut()
        .unwrap()
        .unit_mut()
        .set_health(25);
    assert!(view.as_ref().object().is_active());
    assert_eq!(view.as_ref().unit().unwrap().data().health, 25);
    view.unit_mut().unwrap().set_health(5);
    assert_eq!(actor(&entry).creature.current_health(), 5);
}

#[test]
fn actor_relocation_moves_the_same_box_across_cells_and_loaded_grids() {
    let (mut live, mut rng) = actor_fixture(107, 1070, true);
    live.creature.unit_mut().world_mut().set_active(true);
    let mut map = Map::new(571, 7, 1, 1000);
    let entry = map.creature_actor_entry(live).unwrap();
    let guid = entry.as_ref().object().guid();
    let pointer = actor(&entry) as *const WorldCreature;
    let timeline = actor(&entry)
        .creature
        .unit()
        .health_state_revision_authority_like_cpp();
    let loot = actor(&entry).creature.loot_authority_like_cpp().clone();
    map.add_object_entry_to_map(entry).unwrap();
    for (position, moves_cell, loads_grid) in [
        (Position::xyz(2.0, 3.0, 4.0), false, false),
        (Position::xyz(90.0, 20.0, 5.0), true, false),
        (Position::xyz(700.0, 20.0, 5.0), true, true),
    ] {
        let outcome = map.relocate_map_object_like_cpp(guid, position).unwrap();
        assert!(outcome.relocated);
        assert_eq!(outcome.moved_between_cells, moves_cell);
        assert_eq!(outcome.loaded_grid, loads_grid);
        let current = map.get_typed_creature(guid).unwrap();
        assert_eq!(current.unit().world().position(), position);
        assert!(
            current
                .unit()
                .shares_health_state_revision_authority_like_cpp(&timeline)
        );
        assert!(
            current
                .loot_authority_like_cpp()
                .shares_storage_like_cpp(&loot)
        );
        assert_eq!(map.creature_spawn_id_store_guids_like_cpp(1070), vec![guid]);
        inspect_runtime(&mut map, guid, &mut rng, true, pointer);
    }
}

#[test]
fn blocked_actor_relocation_leaves_motor_health_and_spawn_membership_intact() {
    let (live, mut rng) = actor_fixture(108, 1080, false);
    let mut map = Map::new(571, 7, 1, 1000);
    let entry = map.creature_actor_entry(live).unwrap();
    let guid = entry.as_ref().object().guid();
    let pointer = actor(&entry) as *const WorldCreature;
    let loot = actor(&entry).creature.loot_authority_like_cpp().clone();
    map.insert_object_entry(entry).unwrap();
    let missing = map
        .relocate_map_object_like_cpp(guid, Position::xyz(90.0, 20.0, 5.0))
        .unwrap();
    assert!(!missing.relocated);
    assert!(missing.blocked_by_unloaded_grid);
    inspect_runtime(&mut map, guid, &mut rng, false, pointer);
    let entry = map.take_object_entry(guid).unwrap();
    let added = map.add_object_entry_to_map(entry).unwrap();
    let blocked = map
        .relocate_map_object_like_cpp(guid, Position::xyz(700.0, 20.0, 5.0))
        .unwrap();
    assert!(!blocked.relocated);
    assert!(blocked.blocked_by_unloaded_grid);
    assert_eq!(map.get_typed_creature(guid).unwrap().current_health(), 75);
    assert_eq!(map.creature_spawn_id_store_guids_like_cpp(1080), vec![guid]);
    let cell = super::super::cell_from_world(1.0, 2.0);
    assert!(
        map.get_ngrid(added.grid)
            .unwrap()
            .get_grid_type(cell.cell_x(), cell.cell_y())
            .unwrap()
            .grid_objects
            .creatures
            .contains(&guid)
    );
    assert_ne!(
        loot.lifecycle_like_cpp(),
        OwnedLootAuthorityLifecycle::Detached
    );
    inspect_runtime(&mut map, guid, &mut rng, false, pointer);
}

#[test]
fn actor_shared_snapshot_moves_full_creature_and_preserves_outer_motor_and_create_data() {
    let (live, mut rng) = actor_fixture(109, 1090, true);
    let mut map = Map::new(571, 7, 1, 1000);
    let entry = map.creature_actor_entry(live).unwrap();
    let guid = entry.as_ref().object().guid();
    let pointer = actor(&entry) as *const WorldCreature;
    let creature_pointer = entry.as_ref().creature().unwrap() as *const Creature;
    let loot = actor(&entry).creature.loot_authority_like_cpp().clone();
    let timeline = actor(&entry)
        .creature
        .unit()
        .health_state_revision_authority_like_cpp();
    let create_health = actor(&entry).create_data.health;
    let create_data = format!("{:?}", actor(&entry).create_data);
    let spline = format!("{:?}", actor(&entry).active_move_spline_like_cpp());
    map.insert_object_entry(entry).unwrap();
    // Clone only the actual incoming Creature transport snapshot, never its motor.
    let mut snapshot = map.with_creature_like_cpp(guid, Creature::clone).unwrap();
    snapshot.set_spawn_id(1091);
    snapshot.set_respawn_time(12345);
    snapshot.unit_mut().set_max_health(200);
    snapshot.unit_mut().set_health(0);
    snapshot.unit_mut().set_death_state(DeathState::Corpse);
    snapshot
        .unit_mut()
        .subsystems_mut()
        .auras
        .apply_transform_aura_like_cpp(116, false, None);
    snapshot
        .unit_mut()
        .subsystems_mut()
        .spells
        .set_cooldown(133, 500, 1500);
    let target = ObjectGuid::create_player(1, 111);
    snapshot
        .unit_mut()
        .subsystems_mut()
        .combat
        .initialize_threat_list_capability(true);
    snapshot
        .unit_mut()
        .subsystems_mut()
        .combat
        .add_threat(target, 17.0);
    let revision = snapshot.unit().health_state_revision_like_cpp();
    map.replace_creature_snapshot(MapObjectRecord::new_creature(snapshot).unwrap())
        .unwrap();
    let current = map.get_typed_creature(guid).unwrap();
    assert_eq!(current as *const Creature, creature_pointer);
    assert_eq!(
        (
            current.current_health(),
            current.unit().data().max_health,
            current.unit().death_state()
        ),
        (0, 200, DeathState::Corpse)
    );
    assert_eq!(current.respawn_time(), 12345);
    assert_eq!(
        current.unit().subsystems().auras.transform_spell_like_cpp(),
        116
    );
    assert_eq!(
        current
            .unit()
            .subsystems()
            .spells
            .remaining_cooldown_ms(133, 0, 500),
        1500
    );
    assert_eq!(
        current.unit().subsystems().combat.threat_value(target),
        Some(17.0)
    );
    assert!(
        current
            .unit()
            .shares_health_state_revision_authority_like_cpp(&timeline)
    );
    assert_eq!(current.unit().health_state_revision_like_cpp(), revision);
    assert!(
        current
            .loot_authority_like_cpp()
            .shares_storage_like_cpp(&loot)
    );
    assert_ne!(
        loot.lifecycle_like_cpp(),
        OwnedLootAuthorityLifecycle::Detached
    );
    assert!(map.creature_spawn_id_store_guids_like_cpp(1090).is_empty());
    assert_eq!(map.creature_spawn_id_store_guids_like_cpp(1091), vec![guid]);
    let entry = map.take_object_entry(guid).unwrap();
    assert_eq!(actor(&entry).create_data.health, create_health);
    assert_eq!(format!("{:?}", actor(&entry).create_data), create_data);
    assert_eq!(
        format!("{:?}", actor(&entry).active_move_spline_like_cpp()),
        spline
    );
    map.insert_object_entry(entry).unwrap();
    inspect_runtime(&mut map, guid, &mut rng, true, pointer);
}

#[test]
fn actor_distinct_snapshot_detaches_old_loot_and_moves_new_health_timeline() {
    let (live, mut rng) = actor_fixture(110, 1100, false);
    let mut map = Map::new(571, 7, 1, 1000);
    let entry = map.creature_actor_entry(live).unwrap();
    let guid = entry.as_ref().object().guid();
    let pointer = actor(&entry) as *const WorldCreature;
    let old_loot = actor(&entry).creature.loot_authority_like_cpp().clone();
    map.insert_object_entry(entry).unwrap();
    let mut incoming = Creature::new(false);
    incoming.unit_mut().world_mut().object_mut().create(guid);
    incoming.unit_mut().world_mut().set_map(571, 7).unwrap();
    incoming.unit_mut().set_max_health(150);
    incoming.unit_mut().set_health(120);
    incoming.set_spawn_id(1101);
    let new_loot = incoming.loot_authority_like_cpp().clone();
    let timeline = incoming.unit().health_state_revision_authority_like_cpp();
    map.replace_creature_snapshot(MapObjectRecord::new_creature(incoming).unwrap())
        .unwrap();
    let current = map.get_typed_creature(guid).unwrap();
    assert_eq!(current.current_health(), 120);
    assert!(
        current
            .unit()
            .shares_health_state_revision_authority_like_cpp(&timeline)
    );
    assert!(
        current
            .loot_authority_like_cpp()
            .shares_storage_like_cpp(&new_loot)
    );
    assert_eq!(
        old_loot.lifecycle_like_cpp(),
        OwnedLootAuthorityLifecycle::Detached
    );
    assert_ne!(
        new_loot.lifecycle_like_cpp(),
        OwnedLootAuthorityLifecycle::Detached
    );
    assert!(map.creature_spawn_id_store_guids_like_cpp(1100).is_empty());
    assert_eq!(map.creature_spawn_id_store_guids_like_cpp(1101), vec![guid]);
    inspect_runtime(&mut map, guid, &mut rng, false, pointer);
}

#[test]
fn opaque_displacement_and_remove_return_the_entire_actor_without_projection() {
    let (live, mut rng) = actor_fixture(111, 1110, false);
    let mut map = Map::new(571, 7, 1, 1000);
    let entry = map.creature_actor_entry(live).unwrap();
    let guid = entry.as_ref().object().guid();
    let pointer = actor(&entry) as *const WorldCreature;
    let loot = actor(&entry).creature.loot_authority_like_cpp().clone();
    map.insert_object_entry(entry).unwrap();
    let mut replacement = Creature::new(false);
    replacement.unit_mut().world_mut().object_mut().create(guid);
    replacement.unit_mut().world_mut().set_map(571, 7).unwrap();
    let mut displaced = map
        .insert_map_object_record(MapObjectRecord::new_creature(replacement).unwrap())
        .unwrap()
        .unwrap();
    assert!(displaced.record().is_none());
    assert_eq!(actor(&displaced.entry) as *const WorldCreature, pointer);
    actor_mut(&mut displaced.entry).assert_actor_storage_runtime(&mut rng, false);
    assert_eq!(
        loot.lifecycle_like_cpp(),
        OwnedLootAuthorityLifecycle::Detached
    );
    assert!(map.remove_map_object(guid).unwrap().record().is_some());

    let (live, mut rng) = actor_fixture(112, 1120, true);
    let entry = map.creature_actor_entry(live).unwrap();
    let guid = entry.as_ref().object().guid();
    let pointer = actor(&entry) as *const WorldCreature;
    let loot = actor(&entry).creature.loot_authority_like_cpp().clone();
    map.insert_object_entry(entry).unwrap();
    let mut removed = map.remove_map_object(guid).unwrap();
    assert!(removed.record().is_none());
    assert_eq!(actor(&removed.entry) as *const WorldCreature, pointer);
    actor_mut(&mut removed.entry).assert_actor_storage_runtime(&mut rng, true);
    assert_ne!(
        loot.lifecycle_like_cpp(),
        OwnedLootAuthorityLifecycle::Detached
    );
    assert_eq!(map.map_object_count(), 0);
    assert!(map.creature_spawn_id_store_guids_like_cpp(1120).is_empty());
}

#[test]
fn terminal_actor_removal_preserves_existing_nondelete_erasure_and_detach() {
    for delete in [false, true] {
        let (live, _) = actor_fixture(113, 1130, false);
        let mut map = Map::new(571, 7, 1, 1000);
        let entry = map.creature_actor_entry(live).unwrap();
        let guid = entry.as_ref().object().guid();
        let loot = actor(&entry).creature.loot_authority_like_cpp().clone();
        map.add_object_entry_to_map(entry).unwrap();
        let removed = map.remove_from_map_like_cpp(guid, delete).unwrap();
        assert_eq!(removed.delete_from_world, delete);
        assert!(removed.player.is_none());
        assert_eq!(removed.object.is_some(), !delete);
        if let Some(object) = removed.object {
            assert_eq!(object.guid(), guid);
            assert!(!object.has_current_map());
        }
        assert_eq!(
            loot.lifecycle_like_cpp(),
            OwnedLootAuthorityLifecycle::Detached
        );
        assert_eq!(map.map_object_count(), 0);
        assert!(map.creature_spawn_id_store_guids_like_cpp(1130).is_empty());
        assert!(removed.removed_from_cell);
    }
}

#[test]
fn rejected_actor_snapshots_leave_the_actor_and_authorities_unmodified() {
    let (live, mut rng) = actor_fixture(114, 1140, true);
    let mut map = Map::new(571, 7, 1, 1000);
    let entry = map.creature_actor_entry(live).unwrap();
    let guid = entry.as_ref().object().guid();
    let pointer = actor(&entry) as *const WorldCreature;
    let loot = actor(&entry).creature.loot_authority_like_cpp().clone();
    map.insert_object_entry(entry).unwrap();
    let generic = MapObjectRecord::new(
        AccessorObjectKind::Creature,
        map.get_creature(guid).unwrap().clone(),
    )
    .unwrap();
    assert!(map.replace_creature_snapshot(generic).is_err());
    inspect_runtime(&mut map, guid, &mut rng, true, pointer);
    let mut incoming = map.with_creature_like_cpp(guid, Creature::clone).unwrap();
    incoming.unit_mut().world_mut().reset_map().unwrap();
    incoming.unit_mut().world_mut().set_map(530, 7).unwrap();
    assert!(
        map.replace_creature_snapshot(MapObjectRecord::new_creature(incoming).unwrap())
            .is_err()
    );
    assert_eq!(map.get_typed_creature(guid).unwrap().current_health(), 75);
    assert_eq!(map.creature_spawn_id_store_guids_like_cpp(1140), vec![guid]);
    assert_ne!(
        loot.lifecycle_like_cpp(),
        OwnedLootAuthorityLifecycle::Detached
    );
    inspect_runtime(&mut map, guid, &mut rng, true, pointer);
}

#[test]
fn record_fixture_unwrap_returns_an_actor_intact_on_wrong_variant() {
    let (live, mut rng) = actor_fixture(115, 1150, false);
    let map = Map::new(571, 7, 1, 1000);
    let entry = map.creature_actor_entry(live).unwrap();
    let pointer = actor(&entry) as *const WorldCreature;
    let mut returned = entry.into_record_fixture().unwrap_err();
    assert_eq!(actor(&returned) as *const WorldCreature, pointer);
    actor_mut(&mut returned).assert_actor_storage_runtime(&mut rng, false);
}

#[test]
fn owned_transport_retains_box_and_witness_through_remove_reinsert_and_displacement() {
    let (live, mut rng) = actor_fixture(301, 3010, true);
    let mut map = Map::new(571, 7, 1, 1000);
    let entry = map.creature_actor_entry(live).unwrap();
    let guid = entry.as_ref().object().guid();
    let pointer = actor(&entry) as *const WorldCreature;
    let witness = match &entry {
        ObjectEntry::CreatureActor(actor) => actor.witness(),
        ObjectEntry::Record(_) => unreachable!(),
    };
    map.insert_object_entry(entry).unwrap();
    let mut removed = map.remove_map_object(guid).unwrap();
    assert!(map.creature_actor_witness(guid).is_none());
    assert_eq!(actor(&removed.entry) as *const WorldCreature, pointer);
    match &removed.entry {
        ObjectEntry::CreatureActor(actor) => assert!(witness.same_actor(&actor.witness())),
        ObjectEntry::Record(_) => panic!("removed transport must retain its actor"),
    }
    actor_mut(&mut removed.entry).assert_actor_storage_runtime(&mut rng, true);
    assert!(map.insert_object_entry(removed.entry).unwrap().is_none());
    assert!(witness.same_actor(&map.creature_actor_witness(guid).unwrap()));
    assert_eq!(
        map.creature_actor(guid).unwrap() as *const WorldCreature,
        pointer
    );

    let mut replacement = Creature::new(false);
    replacement.unit_mut().world_mut().object_mut().create(guid);
    replacement.unit_mut().world_mut().set_map(571, 7).unwrap();
    let mut displaced = map
        .insert_map_object_record(MapObjectRecord::new_creature(replacement).unwrap())
        .unwrap()
        .unwrap();
    assert!(map.creature_actor_witness(guid).is_none());
    assert_eq!(actor(&displaced.entry) as *const WorldCreature, pointer);
    match &displaced.entry {
        ObjectEntry::CreatureActor(actor) => assert!(witness.same_actor(&actor.witness())),
        ObjectEntry::Record(_) => panic!("displaced transport must retain its actor"),
    }
    actor_mut(&mut displaced.entry).assert_actor_storage_runtime(&mut rng, true);
    assert_eq!(Arc::strong_count(&witness.0), 2);
    drop(displaced);
    assert_eq!(Arc::strong_count(&witness.0), 1);
}

#[test]
fn terminal_removal_drops_actor_identity_only_after_the_existing_lifecycle_operation() {
    for delete in [false, true] {
        let (live, _) = actor_fixture(302, 3020, false);
        let mut map = Map::new(571, 7, 1, 1000);
        let entry = map.creature_actor_entry(live).unwrap();
        let guid = entry.as_ref().object().guid();
        let witness = match &entry {
            ObjectEntry::CreatureActor(actor) => actor.witness(),
            ObjectEntry::Record(_) => unreachable!(),
        };
        map.add_object_entry_to_map(entry).unwrap();
        assert_eq!(Arc::strong_count(&witness.0), 2);
        let removed = map.remove_from_map_like_cpp(guid, delete).unwrap();
        assert_eq!(removed.delete_from_world, delete);
        assert_eq!(removed.object.is_some(), !delete);
        assert!(map.creature_actor(guid).is_none());
        assert!(map.creature_actor_witness(guid).is_none());
        assert_eq!(Arc::strong_count(&witness.0), 1);
        let (next, _) = actor_fixture(302, 3021, true);
        let current = match map.admit_creature_actor(next).unwrap() {
            super::super::CreatureActorAdmission::Inserted { witness } => witness,
            other => panic!("removed GUID must be available for a new admission: {other:?}"),
        };
        assert!(!witness.same_actor(&current));
        assert_eq!(map.creature_spawn_id_store_guids_like_cpp(3021), vec![guid]);
    }
}
