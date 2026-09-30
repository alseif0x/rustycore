use super::*;
use wow_entities::OwnedLootAuthorityLifecycle;

#[test]
fn distinct_pristine_active_retired_and_quarantined_loot_reject_before_any_extraction() {
    for lifecycle in 0..4 {
        let (mut map, mut source, guid, mut rng) = fixtures::pair(811, true);
        let pointer = map.get_typed_creature(guid).unwrap() as *const Creature;
        let canonical_loot = map
            .get_typed_creature(guid)
            .unwrap()
            .loot_authority_like_cpp()
            .clone();
        let authority = if lifecycle == 3 {
            OwnedLootAuthority::new_retired_tombstone_like_cpp()
        } else {
            OwnedLootAuthority::new()
        };
        source_actor_mut(&mut source, guid)
            .creature
            .adopt_loot_authority_for_snapshot_like_cpp(authority.clone());
        if lifecycle == 1 {
            source_actor_mut(&mut source, guid)
                .creature
                .replace_loot_authority_like_cpp(None, std::collections::HashMap::new());
        } else if lifecycle == 2 {
            source_actor_mut(&mut source, guid)
                .creature
                .replace_loot_authority_like_cpp(None, std::collections::HashMap::new());
            authority.retire_like_cpp();
        }
        let canonical_stamp = canonical_loot.stamp_like_cpp();
        let source_stamp = authority.stamp_like_cpp();
        assert_eq!(
            source_stamp.lifecycle,
            match lifecycle {
                0 => OwnedLootAuthorityLifecycle::Pristine,
                1 => OwnedLootAuthorityLifecycle::Active,
                2 => OwnedLootAuthorityLifecycle::Retired,
                _ => OwnedLootAuthorityLifecycle::Quarantined,
            }
        );
        assert_eq!(
            map.transport_legacy_creature_ownership(&mut source),
            Err(CreatureActorTransportError::LootAuthorityMismatch { guid })
        );
        assert_rejected_owners(&map, &source, guid, pointer);
        assert_eq!(canonical_loot.stamp_like_cpp(), canonical_stamp);
        assert_eq!(authority.stamp_like_cpp(), source_stamp);
        assert_ne!(
            authority.lifecycle_like_cpp(),
            OwnedLootAuthorityLifecycle::Detached
        );
        source_actor_mut(&mut source, guid).assert_actor_storage_runtime(&mut rng, true);
    }
}

#[test]
fn independent_health_timeline_rejects_even_when_tuple_and_loot_identity_match() {
    let (mut map, mut source, guid, mut rng) = fixtures::pair(812, false);
    let independent = fixtures::pair(812, false)
        .1
        .grids
        .remove(&LegacyGridCoord::new(0, 0))
        .unwrap()
        .creatures
        .remove(&guid)
        .unwrap();
    let loot = source_actor(&source, guid)
        .creature
        .loot_authority_like_cpp()
        .clone();
    let mut independent_creature = independent.creature;
    independent_creature.adopt_loot_authority_for_snapshot_like_cpp(loot);
    map.replace_creature_snapshot(MapObjectRecord::new_creature(independent_creature).unwrap())
        .unwrap();
    let pointer = map.get_typed_creature(guid).unwrap() as *const Creature;
    assert_eq!(
        map.transport_legacy_creature_ownership(&mut source),
        Err(CreatureActorTransportError::HealthTimelineMismatch { guid })
    );
    assert_rejected_owners(&map, &source, guid, pointer);
    source_actor_mut(&mut source, guid).assert_actor_storage_runtime(&mut rng, false);
}

#[test]
fn failed_full_map_preflight_does_not_consume_any_valid_sibling() {
    let (mut map, mut source, first, mut first_rng) = fixtures::pair(813, true);
    let (second, mut second_rng) = fixtures::add_pair(&mut map, &mut source, 814, false);
    let first_pointer = map.get_typed_creature(first).unwrap() as *const Creature;
    let second_pointer = map.get_typed_creature(second).unwrap() as *const Creature;
    source_actor_mut(&mut source, second)
        .creature
        .adopt_loot_authority_for_snapshot_like_cpp(OwnedLootAuthority::new());
    assert_eq!(
        map.transport_legacy_creature_ownership(&mut source),
        Err(CreatureActorTransportError::LootAuthorityMismatch { guid: second })
    );
    assert_eq!(source_count(&source), 2);
    assert_rejected_owners(&map, &source, first, first_pointer);
    assert_rejected_owners(&map, &source, second, second_pointer);
    source_actor_mut(&mut source, first).assert_actor_storage_runtime(&mut first_rng, true);
    source_actor_mut(&mut source, second).assert_actor_storage_runtime(&mut second_rng, false);
}

#[test]
fn orphan_source_and_canonical_only_record_are_explicit_and_unconsumed() {
    let (mut map, mut source, guid, _) = fixtures::pair(815, true);
    let removed = map.entity_world.take(&guid).unwrap();
    assert_eq!(
        map.transport_legacy_creature_ownership(&mut source),
        Err(CreatureActorTransportError::MissingCounterpart { guid })
    );
    assert_eq!(source_count(&source), 1);
    map.entity_world
        .restore_transport_entry(guid, removed)
        .unwrap();
    let slot = source
        .preflight_creature_transport_slot(LegacyGridCoord::new(0, 0), guid)
        .unwrap();
    let taken = match source.take_creature_transport_slot(slot) {
        Ok(taken) => taken,
        Err(_) => panic!("fixture slot"),
    };
    assert_eq!(
        map.transport_legacy_creature_ownership(&mut source),
        Err(CreatureActorTransportError::CanonicalOnly { guid })
    );
    assert!(map.creature_actor(guid).is_none());
    assert!(source.restore_creature_transport_slot(taken).is_ok());
    assert_eq!(source_count(&source), 1);
}

#[test]
fn existing_actor_collision_preserves_both_motors_and_the_existing_witness() {
    let (mut map, mut source, guid, mut rng) = fixtures::pair(816, true);
    let other = fixtures::pair(816, false)
        .1
        .grids
        .remove(&LegacyGridCoord::new(0, 0))
        .unwrap()
        .creatures
        .remove(&guid)
        .unwrap();
    let old_record = map.entity_world.take(&guid).unwrap();
    map.entity_world
        .restore_transport_entry(guid, ObjectEntry::from_creature_actor(other))
        .unwrap();
    let witness = map.creature_actor_witness(guid).unwrap();
    let pointer = map.creature_actor(guid).unwrap() as *const WorldCreature;
    assert_eq!(
        map.transport_legacy_creature_ownership(&mut source),
        Err(CreatureActorTransportError::ExistingActor { guid })
    );
    assert_eq!(
        map.creature_actor(guid).unwrap() as *const WorldCreature,
        pointer
    );
    assert!(
        map.creature_actor_witness(guid)
            .unwrap()
            .same_actor(&witness)
    );
    source_actor_mut(&mut source, guid).assert_actor_storage_runtime(&mut rng, true);
    drop(old_record);
}

#[test]
fn duplicate_guid_in_different_source_grids_rejects_without_filtering() {
    let (mut map, mut source, guid, _) = fixtures::pair(817, true);
    let other = fixtures::pair(817, false)
        .1
        .grids
        .remove(&LegacyGridCoord::new(0, 0))
        .unwrap()
        .creatures
        .remove(&guid)
        .unwrap();
    let mut grid = Grid::new(1, 0);
    grid.creatures.insert(guid, other);
    source.grids.insert(LegacyGridCoord::new(1, 0), grid);
    assert_eq!(
        map.transport_legacy_creature_ownership(&mut source),
        Err(CreatureActorTransportError::DuplicateSourceGuid { guid })
    );
    assert_eq!(source_count(&source), 2);
    assert!(map.creature_actor(guid).is_none());
}

#[test]
fn wrong_source_grid_key_guid_map_or_spawn_rejects_before_mutation() {
    for invalid in 0..4 {
        let (mut map, mut source, guid, _) = fixtures::pair(818, true);
        match invalid {
            0 => {
                source
                    .grids
                    .get_mut(&LegacyGridCoord::new(0, 0))
                    .unwrap()
                    .coord = LegacyGridCoord::new(1, 0)
            }
            1 => source_actor_mut(&mut source, guid)
                .creature
                .unit_mut()
                .world_mut()
                .object_mut()
                .create(ObjectGuid::create_player(1, 99)),
            2 => {
                source_actor_mut(&mut source, guid)
                    .creature
                    .unit_mut()
                    .world_mut()
                    .reset_map()
                    .unwrap();
            }
            _ => source_actor_mut(&mut source, guid)
                .creature
                .set_spawn_id(9999),
        }
        let pointer = map.get_typed_creature(guid).unwrap() as *const Creature;
        let error = map
            .transport_legacy_creature_ownership(&mut source)
            .unwrap_err();
        assert!(matches!(
            error,
            CreatureActorTransportError::SourceGridCoordinateMismatch { .. }
                | CreatureActorTransportError::SourceGuidMismatch { .. }
                | CreatureActorTransportError::SourceMapMismatch { .. }
                | CreatureActorTransportError::SpawnMismatch { .. }
        ));
        assert_eq!(
            map.get_typed_creature(guid).unwrap() as *const Creature,
            pointer
        );
        assert_eq!(source_count(&source), 1);
        assert!(map.creature_actor(guid).is_none());
    }
}

#[test]
fn generic_creature_kind_body_is_not_an_admissible_record_counterpart() {
    let (mut map, mut source, guid, _) = fixtures::pair(819, false);
    let generic = MapObjectRecord::new(
        AccessorObjectKind::Creature,
        map.map_object(guid).unwrap().clone(),
    )
    .unwrap();
    map.insert_map_object_record(generic).unwrap();
    assert_eq!(
        map.transport_legacy_creature_ownership(&mut source),
        Err(CreatureActorTransportError::NotExactCreature { guid })
    );
    assert_eq!(source_count(&source), 1);
    assert!(map.get_typed_creature(guid).is_none());
}

#[test]
fn wrong_legacy_instance_rejects_the_whole_map_without_extraction() {
    let (mut map, mut source, guid, _) = fixtures::pair(820, true);
    source.instance_id = 8;
    assert_eq!(
        map.transport_legacy_creature_ownership(&mut source),
        Err(CreatureActorTransportError::WrongLegacyMap)
    );
    assert_eq!(source_count(&source), 1);
    assert!(map.creature_actor(guid).is_none());
}

#[test]
fn missing_or_wrong_spawn_index_membership_rejects_before_any_extraction() {
    for wrong in [false, true] {
        let (mut map, mut source, guid, _) = fixtures::pair(827, true);
        map.creatures_by_spawn_id.remove(&8270);
        if wrong {
            map.creatures_by_spawn_id
                .entry(9999)
                .or_default()
                .insert(guid);
        }
        assert_eq!(
            map.transport_legacy_creature_ownership(&mut source),
            Err(CreatureActorTransportError::SpawnIndexMismatch {
                guid,
                spawn_id: 8270
            })
        );
        assert_eq!(source_count(&source), 1);
        assert!(map.creature_actor(guid).is_none());
    }
}

#[test]
fn mismatching_record_guid_rejects_before_source_extraction() {
    let (mut map, mut source, guid, _) = fixtures::pair(828, true);
    map.get_typed_creature_mut(guid)
        .unwrap()
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(ObjectGuid::create_player(1, 99));
    assert_eq!(
        map.transport_legacy_creature_ownership(&mut source),
        Err(CreatureActorTransportError::GuidMismatch { guid })
    );
    assert_eq!(source_count(&source), 1);
}
