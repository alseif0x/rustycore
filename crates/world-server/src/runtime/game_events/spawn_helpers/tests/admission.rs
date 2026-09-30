use super::*;

#[test]
fn preparation_moves_full_creature_and_preserves_health_and_loot_authorities() {
    let mut record = creature_record(910);
    let creature = record.creature_mut().unwrap();
    creature.unit_mut().subsystems_mut().auras.apply_transform_aura_like_cpp(118, false, None);
    creature.unit_mut().subsystems_mut().spells.set_cooldown(133, 500, 1500);
    creature.unit_mut().subsystems_mut().combat.initialize_threat_list_capability(true);
    let target = ObjectGuid::create_player(1, 912);
    creature.unit_mut().subsystems_mut().combat.add_threat(target, 31.0);
    let loot = creature.loot_authority_like_cpp().clone();
    let timeline = creature.unit().health_state_revision_authority_like_cpp();
    let revision = creature.unit().health_state_revision_like_cpp();
    let expected_data = WorldCreature::create_data_from_canonical_like_cpp(creature);

    let actor = prepare_loaded_grid_creature(
        LoadedGridRespawnRecordsLikeCpp::primary_only(record),
        &WaypointPathStoreLikeCpp::default(),
    ).unwrap().actor;
    assert_eq!((actor.create_data.health, actor.create_data.max_health,
        actor.create_data.npc_flags, actor.create_data.display_id, actor.create_data.level),
        (expected_data.health, expected_data.max_health, expected_data.npc_flags,
        expected_data.display_id, expected_data.level));
    assert_eq!((actor.creature.current_health(), actor.creature.max_health()), (123, 321));
    assert_eq!(actor.creature.unit().health_state_revision_like_cpp(), revision);
    assert!(actor.creature.unit().shares_health_state_revision_authority_like_cpp(&timeline));
    assert!(actor.creature.loot_authority_like_cpp().shares_storage_like_cpp(&loot));
    assert_eq!(actor.creature.unit().subsystems().auras.transform_spell_like_cpp(), 118);
    assert_eq!(actor.creature.unit().subsystems().spells.remaining_cooldown_ms(133, 0, 1000), 1000);
    assert_eq!(actor.creature.unit().subsystems().combat.threat_value(target), Some(31.0));
    assert_eq!(actor.runtime_motion_master_ticks_like_cpp(), 0);
}

#[test]
fn wrong_primary_body_returns_all_original_owned_facets_before_effects() {
    let primary = player_record(913, 571);
    let primary_pointer = primary.player().unwrap() as *const Player;
    let pre_add = player_record(914, 571);
    let pre_add_pointer = pre_add.player().unwrap() as *const Player;
    let returned = prepare_loaded_grid_creature(
        LoadedGridRespawnRecordsLikeCpp { pre_add_records: vec![pre_add], primary_record: primary },
        &WaypointPathStoreLikeCpp::default(),
    ).unwrap_err();
    assert_eq!(returned.primary_record.player().unwrap() as *const Player, primary_pointer);
    assert_eq!(returned.pre_add_records.len(), 1);
    assert_eq!(returned.pre_add_records[0].player().unwrap() as *const Player, pre_add_pointer);
}

#[test]
fn generic_creature_body_is_rejected_without_promoting_a_world_object() {
    let creature = creature_record(915).into_creature().unwrap();
    let record = MapObjectRecord::new(AccessorObjectKind::Creature, creature.unit().world().clone()).unwrap();
    let guid = record.object().guid();
    let returned = prepare_loaded_grid_creature(
        LoadedGridRespawnRecordsLikeCpp::primary_only(record), &WaypointPathStoreLikeCpp::default(),
    ).unwrap_err();
    assert_eq!(returned.primary_record.object().guid(), guid);
    assert_eq!(returned.primary_record.kind(), AccessorObjectKind::Creature);
    assert!(returned.primary_record.creature().is_none());
}

#[test]
fn gameobject_kind_returns_its_original_owned_record_and_pre_add_facets() {
    let mut object = creature_record(927).into_creature().unwrap().unit().world().clone();
    let guid = ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 7, 43, 927);
    object.object_mut().create(guid);
    let record = MapObjectRecord::new(AccessorObjectKind::GameObject, object).unwrap();
    let returned = prepare_loaded_grid_creature(LoadedGridRespawnRecordsLikeCpp {
        pre_add_records: vec![player_record(928, 571)], primary_record: record,
    }, &WaypointPathStoreLikeCpp::default()).unwrap_err();
    assert_eq!(returned.primary_record.kind(), AccessorObjectKind::GameObject);
    assert_eq!(returned.primary_record.object().guid(), guid);
    assert_eq!(returned.pre_add_records.len(), 1);
}

#[test]
fn fresh_admission_retains_all_represented_add_hooks_and_spawn_membership() {
    let mut map = wow_map::Map::new(571, 7, 1, 1000);
    let result = prepared(916).admit(&mut map);
    assert!(result.pre_add.is_empty());
    let FreshCreatureActorAdmission::Inserted { outcome } = result.primary.unwrap() else { panic!("fresh insertion"); };
    assert!(outcome.inserted && !outcome.already_in_world);
    assert!(outcome.inserted_into_cell);
    assert_eq!(outcome.creature_store_inserted_before_add_to_world, Some(true));
    assert_eq!(outcome.creature_spawn_indexed_before_add_to_world, Some(true));
    assert!(outcome.creature_unit_add_to_world.is_some());
    assert!(outcome.creature_search_formation.is_some());
    assert!(outcome.creature_aim_initialize.is_some());
    assert!(outcome.creature_zone_script_create.is_some());
    assert!(outcome.add_to_map_tail.is_some());
    assert_eq!(map.creature_spawn_id_store_guids_like_cpp(9160), vec![outcome.guid]);
    assert_eq!(map.get_typed_creature(outcome.guid).unwrap().current_health(), 123);
}

#[test]
fn ordered_pre_add_facets_are_reported_before_primary_even_after_a_facet_error() {
    let mut map = wow_map::Map::new(571, 7, 1, 1000);
    let first = player_record(917, 571);
    let last = player_record(919, 571);
    let first_guid = first.object().guid();
    let last_guid = last.object().guid();
    let pending = prepare_loaded_grid_creature(LoadedGridRespawnRecordsLikeCpp {
        pre_add_records: vec![first, player_record(918, 530), last],
        primary_record: creature_record(920),
    }, &WaypointPathStoreLikeCpp::default()).unwrap();
    let result = pending.admit(&mut map);
    assert_eq!(result.pre_add.len(), 3);
    assert_eq!(result.pre_add[0].as_ref().unwrap().guid, first_guid);
    assert!(result.pre_add[1].is_err());
    assert_eq!(result.pre_add[2].as_ref().unwrap().guid, last_guid);
    assert!(matches!(result.primary, Ok(FreshCreatureActorAdmission::Inserted { .. })));
    assert_eq!(map.map_reference_order_like_cpp(), &[first_guid, last_guid]);
    assert_eq!(map.map_object_count(), 3);
}

#[test]
fn duplicate_record_keeps_stored_record_and_returns_incoming_motor() {
    let mut map = wow_map::Map::new(571, 7, 1, 1000);
    let record = creature_record(921);
    let guid = record.object().guid();
    map.add_map_object_record_to_map_like_cpp(record).unwrap();
    let pointer = map.get_typed_creature(guid).unwrap() as *const Creature;
    let FreshCreatureActorAdmission::ExistingRecord { incoming } = prepared(921).admit(&mut map).primary.unwrap()
        else { panic!("Record collision must remain explicit"); };
    assert_eq!(incoming.guid(), guid);
    assert_eq!(incoming.create_data.health, 123);
    assert_eq!(incoming.runtime_motion_master_ticks_like_cpp(), 0);
    assert_eq!(map.get_typed_creature(guid).unwrap() as *const Creature, pointer);
    assert_eq!(map.map_object_count(), 1);
}

#[test]
fn duplicate_actor_does_not_replace_stored_actor_or_discard_incoming() {
    let mut map = wow_map::Map::new(571, 7, 1, 1000);
    let FreshCreatureActorAdmission::Inserted { outcome } = prepared(922).admit(&mut map).primary.unwrap()
        else { panic!("fresh insertion"); };
    let pointer = map.get_typed_creature(outcome.guid).unwrap() as *const Creature;
    let mut next = prepared(922);
    next.actor.creature.unit_mut().set_health(99);
    let FreshCreatureActorAdmission::ExistingActor { incoming } = next.admit(&mut map).primary.unwrap()
        else { panic!("Actor collision must remain explicit"); };
    assert_eq!(incoming.creature.current_health(), 99);
    assert_eq!(map.get_typed_creature(outcome.guid).unwrap().current_health(), 123);
    assert_eq!(map.get_typed_creature(outcome.guid).unwrap() as *const Creature, pointer);
}

#[test]
fn wrong_map_rejection_returns_the_same_actor_payload_with_pre_add_results() {
    let mut map = wow_map::Map::new(530, 7, 1, 1000);
    let pending = prepared(923);
    let guid = pending.actor.guid();
    let timeline = pending.actor.creature.unit().health_state_revision_authority_like_cpp();
    let (error, incoming) = pending.admit(&mut map).primary.unwrap_err();
    assert!(matches!(error, FreshCreatureActorAdmissionError::Store(_)));
    assert_eq!(incoming.guid(), guid);
    assert_eq!(incoming.create_data.health, 123);
    assert!(incoming.creature.unit().shares_health_state_revision_authority_like_cpp(&timeline));
    assert_eq!(incoming.runtime_motion_master_ticks_like_cpp(), 0);
    assert_eq!(map.map_object_count(), 0);
}

#[test]
fn invalid_coordinates_and_already_in_world_return_owned_incoming_without_effects() {
    let mut map = wow_map::Map::new(571, 7, 1, 1000);
    let mut invalid = prepared(924);
    invalid.actor.creature.unit_mut().world_mut().relocate(Position::xyz(f32::NAN, 2.0, 3.0));
    let (error, incoming) = invalid.admit(&mut map).primary.unwrap_err();
    assert!(matches!(error, FreshCreatureActorAdmissionError::InvalidCoordinates { .. }));
    assert!(incoming.position().x.is_nan());
    let mut live = prepared(925);
    live.actor.creature.unit_mut().world_mut().object_mut().add_to_world();
    let (error, incoming) = live.admit(&mut map).primary.unwrap_err();
    assert!(matches!(error, FreshCreatureActorAdmissionError::AlreadyInWorld { .. }));
    assert!(incoming.creature.unit().world().object().is_in_world());
    assert_eq!(map.map_object_count(), 0);
}

#[test]
fn waypoint_default_is_initialized_once_without_clock_or_motion_master_tick() {
    let mut record = creature_record(926);
    record.creature_mut().unwrap().set_default_movement_type_runtime_like_cpp(wow_entities::MovementGeneratorType::Waypoint);
    let pending = prepare_loaded_grid_creature(
        LoadedGridRespawnRecordsLikeCpp::primary_only(record), &WaypointPathStoreLikeCpp::default(),
    ).unwrap();
    assert!(pending.actor.active_waypoint_generator_like_cpp().is_some());
    assert_eq!(pending.actor.runtime_motion_master_ticks_like_cpp(), 0);
}
