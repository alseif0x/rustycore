use super::*;

fn actor_parts(admission: LoadedGridAdmission) -> (
    Vec<Result<AddToMapOutcome, AddToMapError>>,
    Result<FreshCreatureActorAdmission, (FreshCreatureActorAdmissionError, WorldCreature)>,
) {
    let (facets, primary) = admission.into_parts();
    match primary {
        LoadedGridPrimaryAdmission::CreatureActor(result) => (facets, result),
        LoadedGridPrimaryAdmission::Record { .. } => panic!("actor receipt must contain no Record snapshot"),
    }
}

#[test]
fn actor_pre_add_order_and_failed_facet_preserve_whole_motor_and_fresh_hooks() {
    let mut shared = map();
    let (incoming, mut rng) = actor(976, true);
    let guid = incoming.guid();
    let loot = incoming.creature.loot_authority_like_cpp().clone();
    let timeline = incoming.creature.unit().health_state_revision_authority_like_cpp();
    let (facets, primary) = actor_parts(shared.admit_loaded_grid_materialization(
        LoadedGridMaterialization::creature(vec![
            player_record(984, 571), player_record(985, 530), player_record(986, 571),
        ], incoming),
    ));
    assert_eq!(facets.len(), 3);
    assert_eq!(facets[0].as_ref().unwrap().guid, ObjectGuid::create_player(1, 984));
    assert!(facets[1].is_err());
    assert_eq!(facets[2].as_ref().unwrap().guid, ObjectGuid::create_player(1, 986));
    let FreshCreatureActorAdmission::Inserted { outcome } = primary.unwrap()
        else { panic!("fresh admission"); };
    assert_eq!(outcome.guid, guid);
    assert_eq!(outcome.creature_store_inserted_before_add_to_world, Some(true));
    assert_eq!(outcome.creature_spawn_indexed_before_add_to_world, Some(true));
    assert!(outcome.creature_unit_add_to_world.is_some());
    assert!(outcome.add_to_map_tail.is_some());
    assert_eq!(shared.map_reference_order_like_cpp(), &[ObjectGuid::create_player(1, 984), ObjectGuid::create_player(1, 986)]);
    let stored = shared.creature_actor_mut(guid).unwrap();
    assert!(stored.creature.loot_authority_like_cpp().shares_storage_like_cpp(&loot));
    assert!(stored.creature.unit().shares_health_state_revision_authority_like_cpp(&timeline));
    stored.assert_actor_storage_runtime(&mut rng, true);
}

#[test]
fn actor_wrong_map_returns_motor_after_pre_add_effects_without_rollback() {
    let mut shared = map();
    let (mut incoming, mut rng) = actor(977, false);
    let guid = incoming.guid();
    incoming.creature.unit_mut().world_mut().reset_map().unwrap();
    incoming.creature.unit_mut().world_mut().set_map(530, 7).unwrap();
    let (facets, primary) = actor_parts(shared.admit_loaded_grid_materialization(
        LoadedGridMaterialization::creature(vec![player_record(987, 571)], incoming),
    ));
    assert!(facets[0].is_ok());
    let (error, mut returned) = primary.unwrap_err();
    assert!(matches!(error, FreshCreatureActorAdmissionError::Store(_)));
    assert_eq!(returned.guid(), guid);
    returned.assert_actor_storage_runtime(&mut rng, false);
    assert_eq!(shared.map_object_count(), 1);
    assert!(shared.get_typed_creature(guid).is_none());
    assert_eq!(shared.map_reference_order_like_cpp(), &[ObjectGuid::create_player(1, 987)]);
}

#[test]
fn actor_duplicate_record_keeps_stored_value_and_returns_incoming_point_motor() {
    let mut shared = map();
    let current = record(978);
    let guid = current.object().guid();
    shared.add_map_object_record_to_map_like_cpp(current).unwrap();
    let pointer = shared.get_typed_creature(guid).unwrap() as *const Creature;
    let (mut incoming, mut rng) = actor(978, true);
    incoming.creature.unit_mut().set_health(61);
    let (_, primary) = actor_parts(shared.admit_loaded_grid_materialization(
        LoadedGridMaterialization::creature(Vec::new(), incoming),
    ));
    let FreshCreatureActorAdmission::ExistingRecord { mut incoming } = primary.unwrap()
        else { panic!("Record collision"); };
    assert_eq!(incoming.creature.current_health(), 61);
    incoming.assert_actor_storage_runtime(&mut rng, true);
    assert_eq!(shared.get_typed_creature(guid).unwrap() as *const Creature, pointer);
    assert_eq!(shared.get_typed_creature(guid).unwrap().current_health(), 75);
}

#[test]
fn actor_duplicate_actor_retains_both_original_motor_streams_without_cloning() {
    let mut shared = map();
    let (first, mut first_rng) = actor(979, true);
    let guid = first.guid();
    let (_, first_result) = actor_parts(shared.admit_loaded_grid_materialization(
        LoadedGridMaterialization::creature(Vec::new(), first),
    ));
    assert!(matches!(first_result, Ok(FreshCreatureActorAdmission::Inserted { .. })));
    let pointer = shared.creature_actor(guid).unwrap() as *const WorldCreature;
    let (second, mut second_rng) = actor(979, false);
    let (_, second_result) = actor_parts(shared.admit_loaded_grid_materialization(
        LoadedGridMaterialization::creature(Vec::new(), second),
    ));
    let FreshCreatureActorAdmission::ExistingActor { mut incoming } = second_result.unwrap()
        else { panic!("Actor collision"); };
    incoming.assert_actor_storage_runtime(&mut second_rng, false);
    assert_eq!(shared.creature_actor(guid).unwrap() as *const WorldCreature, pointer);
    shared.creature_actor_mut(guid).unwrap().assert_actor_storage_runtime(&mut first_rng, true);
}

#[test]
fn admission_never_consumes_catalog_timer_for_either_payload_branch() {
    let mut shared = map();
    for counter in [980, 988] {
        shared.add_respawn_info_like_cpp(crate::RespawnInfoLikeCpp {
            object_type: crate::SpawnObjectType::Creature, spawn_id: counter as u64 * 10,
            entry: 42, respawn_time: 12345, grid_id: 0,
        });
    }
    let (_, _, record_result) = shared.admit_loaded_grid_materialization(
        LoadedGridMaterialization::records(LoadedGridRespawnRecordsLikeCpp::primary_only(record(980))),
    ).into_record_parts();
    assert!(record_result.is_ok());
    let (incoming, _) = actor(988, false);
    let (_, actor_result) = actor_parts(shared.admit_loaded_grid_materialization(
        LoadedGridMaterialization::creature(Vec::new(), incoming),
    ));
    assert!(matches!(actor_result, Ok(FreshCreatureActorAdmission::Inserted { .. })));
    for counter in [980, 988] {
        assert_eq!(shared.get_respawn_time_like_cpp(crate::SpawnObjectType::Creature, counter as u64 * 10), 12345);
    }
}
