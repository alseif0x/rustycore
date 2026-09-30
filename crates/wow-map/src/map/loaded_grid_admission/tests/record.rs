use super::*;

// Frozen three-driver insertion sequence as a test oracle, not another
// production planner/admission implementation. Each side creates its own owned
// objects; no Actor clone is used to simulate a move.
fn original_record_operation(
    map: &mut Map,
    records: LoadedGridRespawnRecordsLikeCpp,
) -> (
    Vec<Result<AddToMapOutcome, AddToMapError>>,
    MapObjectRecord,
    Result<AddToMapOutcome, AddToMapError>,
) {
    let mut pre_add = Vec::new();
    for record in records.pre_add_records {
        pre_add.push(map.add_map_object_record_to_map_like_cpp(record));
    }
    let snapshot = records.primary_record.clone();
    let result = map.add_map_object_record_to_map_like_cpp(records.primary_record);
    (pre_add, snapshot, result)
}

#[test]
fn record_operation_matches_original_facets_hooks_snapshot_and_player_order() {
    let mut original = map();
    let mut shared = map();
    let (old_facets, old_snapshot, old_result) =
        original_record_operation(&mut original, records_with_facets(971, false));
    let (facets, snapshot, result) = shared
        .admit_loaded_grid_materialization(LoadedGridMaterialization::records(records_with_facets(
            971, false,
        )))
        .into_record_parts();
    assert_eq!(facets, old_facets);
    assert_eq!(result, old_result);
    let guid = snapshot.object().guid();
    assert_eq!(guid, old_snapshot.object().guid());
    assert!(
        snapshot
            .creature()
            .unwrap()
            .unit()
            .subsystems()
            .auras
            .has_applied(aura())
    );
    assert!(
        old_snapshot
            .creature()
            .unwrap()
            .unit()
            .subsystems()
            .auras
            .has_applied(aura())
    );
    assert!(!snapshot.object().object().is_in_world());
    assert!(
        !shared
            .get_typed_creature(guid)
            .unwrap()
            .unit()
            .subsystems()
            .auras
            .has_applied(aura())
    );
    assert_eq!(
        shared.map_reference_order_like_cpp(),
        original.map_reference_order_like_cpp()
    );
    assert_eq!(
        shared.creature_spawn_id_store_guids_like_cpp(9710),
        vec![guid]
    );
    assert_eq!(shared.map_object_count(), original.map_object_count());
}

#[test]
fn record_primary_failure_keeps_original_snapshot_and_successful_pre_add_effects() {
    let mut original = map();
    let mut shared = map();
    let (old_facets, old_snapshot, old_result) =
        original_record_operation(&mut original, records_with_facets(972, true));
    let (facets, snapshot, result) = shared
        .admit_loaded_grid_materialization(LoadedGridMaterialization::records(records_with_facets(
            972, true,
        )))
        .into_record_parts();
    assert_eq!(facets, old_facets);
    assert_eq!(result, old_result);
    assert!(result.is_err());
    assert_eq!(snapshot.object().guid(), old_snapshot.object().guid());
    assert_eq!(snapshot.object().map_id(), 530);
    assert!(
        snapshot
            .creature()
            .unwrap()
            .unit()
            .subsystems()
            .auras
            .has_applied(aura())
    );
    assert_eq!(
        shared.map_reference_order_like_cpp(),
        original.map_reference_order_like_cpp()
    );
    assert_eq!(shared.map_object_count(), 2);
    assert!(
        shared
            .get_typed_creature(snapshot.object().guid())
            .is_none()
    );
}

#[test]
fn already_in_world_record_keeps_original_refresh_and_snapshot_behavior() {
    let mut original = map();
    let mut shared = map();
    let mut old = record(973);
    let mut incoming = record(973);
    old.object_mut().object_mut().add_to_world();
    incoming.object_mut().object_mut().add_to_world();
    let (_, _, old_result) = original_record_operation(
        &mut original,
        LoadedGridRespawnRecordsLikeCpp::primary_only(old),
    );
    let (_, snapshot, result) = shared
        .admit_loaded_grid_materialization(LoadedGridMaterialization::records(
            LoadedGridRespawnRecordsLikeCpp::primary_only(incoming),
        ))
        .into_record_parts();
    assert_eq!(result, old_result);
    assert!(result.unwrap().already_in_world);
    assert!(snapshot.object().object().is_in_world());
}

#[test]
fn record_player_primary_is_not_filtered_out_as_creature_only() {
    let mut shared = map();
    let incoming = player_record(974, 571);
    let guid = incoming.object().guid();
    let (facets, snapshot, result) = shared
        .admit_loaded_grid_materialization(LoadedGridMaterialization::records(
            LoadedGridRespawnRecordsLikeCpp::primary_only(incoming),
        ))
        .into_record_parts();
    assert!(facets.is_empty());
    assert!(snapshot.player().is_some());
    assert_eq!(result.unwrap().guid, guid);
    assert_eq!(shared.map_reference_order_like_cpp(), &[guid]);
}

#[test]
fn record_gameobject_primary_retains_exact_kind_and_compatibility_snapshot() {
    let mut shared = map();
    let mut object = creature(975).unit().world().clone();
    let guid = ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 7, 43, 975);
    object.object_mut().create(guid);
    let incoming =
        MapObjectRecord::new(wow_entities::AccessorObjectKind::GameObject, object).unwrap();
    let (_, snapshot, result) = shared
        .admit_loaded_grid_materialization(LoadedGridMaterialization::records(
            LoadedGridRespawnRecordsLikeCpp::primary_only(incoming),
        ))
        .into_record_parts();
    assert_eq!(
        snapshot.kind(),
        wow_entities::AccessorObjectKind::GameObject
    );
    assert_eq!(snapshot.object().guid(), guid);
    assert_eq!(result.unwrap().guid, guid);
    assert_eq!(shared.map_object_count(), 1);
}
