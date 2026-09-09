//! Item scenarios for [`super`].
//!
//! Split out of spawn_store_loader_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

#[test]
fn game_event_model_equip_accepts_zero_equipment_and_preserves_order_like_cpp() {
    let mut model_equip = GameEventModelEquipLikeCpp::from_game_event_max_entry_like_cpp(Some(3));
    let equipment_templates = BTreeSet::new();
    let mut report = GameEventModelEquipLoadReportLikeCpp::default();

    apply_game_event_model_equip_row_like_cpp(
        GameEventModelEquipRowLikeCpp {
            spawn_id: 100,
            entry: 10,
            event_id: 1,
            model_id: 111,
            equipment_id: 0,
        },
        &equipment_templates,
        &mut model_equip,
        &mut report,
    );
    apply_game_event_model_equip_row_like_cpp(
        GameEventModelEquipRowLikeCpp {
            spawn_id: 101,
            entry: 11,
            event_id: 1,
            model_id: 112,
            equipment_id: 0,
        },
        &equipment_templates,
        &mut model_equip,
        &mut report,
    );

    let records = model_equip.records_like_cpp(1).expect("event 1 exists");
    assert_eq!(records.len(), 2);
    assert_eq!(records[0].spawn_id, 100);
    assert_eq!(records[0].model_id, 111);
    assert_eq!(records[0].model_id_prev, 0);
    assert_eq!(records[0].equipment_id, 0);
    assert_eq!(records[0].equipment_id_prev, 0);
    assert_eq!(records[1].spawn_id, 101);
    assert_eq!(report.rows, 2);
    assert_eq!(report.loaded, 2);
    assert_eq!(report.missing_equipment_template, 0);
}
#[test]
fn game_event_model_equip_skips_out_of_range_event_id_like_cpp() {
    let mut model_equip = GameEventModelEquipLikeCpp::from_game_event_max_entry_like_cpp(Some(3));
    let equipment_templates = BTreeSet::new();
    let mut report = GameEventModelEquipLoadReportLikeCpp::default();

    apply_game_event_model_equip_row_like_cpp(
        GameEventModelEquipRowLikeCpp {
            spawn_id: 100,
            entry: 10,
            event_id: 4,
            model_id: 111,
            equipment_id: 0,
        },
        &equipment_templates,
        &mut model_equip,
        &mut report,
    );

    assert_eq!(model_equip.records_like_cpp(4), None);
    assert_eq!(report.rows, 1);
    assert_eq!(report.loaded, 0);
    assert_eq!(report.invalid_event_id, 1);
}
#[test]
fn game_event_model_equip_skips_missing_positive_equipment_template_like_cpp() {
    let mut model_equip = GameEventModelEquipLikeCpp::from_game_event_max_entry_like_cpp(Some(3));
    let equipment_templates = BTreeSet::from([(10_u32, 2_u8)]);
    let mut report = GameEventModelEquipLoadReportLikeCpp::default();

    apply_game_event_model_equip_row_like_cpp(
        GameEventModelEquipRowLikeCpp {
            spawn_id: 100,
            entry: 10,
            event_id: 1,
            model_id: 111,
            equipment_id: 1,
        },
        &equipment_templates,
        &mut model_equip,
        &mut report,
    );

    assert_eq!(model_equip.records_like_cpp(1), Some([].as_slice()));
    assert_eq!(report.rows, 1);
    assert_eq!(report.loaded, 0);
    assert_eq!(report.missing_equipment_template, 1);
}
#[test]
fn game_event_model_equip_accepts_existing_positive_equipment_template_like_cpp() {
    let mut model_equip = GameEventModelEquipLikeCpp::from_game_event_max_entry_like_cpp(Some(3));
    let equipment_templates = BTreeSet::from([(10_u32, 1_u8)]);
    let mut report = GameEventModelEquipLoadReportLikeCpp::default();

    apply_game_event_model_equip_row_like_cpp(
        GameEventModelEquipRowLikeCpp {
            spawn_id: 100,
            entry: 10,
            event_id: 1,
            model_id: 111,
            equipment_id: 1,
        },
        &equipment_templates,
        &mut model_equip,
        &mut report,
    );

    let records = model_equip.records_like_cpp(1).expect("event 1 exists");
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].spawn_id, 100);
    assert_eq!(records[0].equipment_id, 1);
    assert_eq!(records[0].equipment_id_prev, 0);
    assert_eq!(report.rows, 1);
    assert_eq!(report.loaded, 1);
    assert_eq!(report.missing_equipment_template, 0);
}
#[test]
fn canonical_metadata_exposes_game_event_model_equip_slices_like_cpp() {
    let mut model_equip = GameEventModelEquipLikeCpp::from_game_event_max_entry_like_cpp(Some(3));
    assert!(model_equip.push_record_like_cpp(
        1,
        GameEventModelEquipRecordLikeCpp {
            spawn_id: 100,
            model_id: 111,
            model_id_prev: 0,
            equipment_id: 0,
            equipment_id_prev: 0,
        },
    ));
    let metadata = CanonicalSpawnMetadataLikeCpp::new(SpawnStore::new(), BTreeMap::new())
        .with_game_event_model_equip_like_cpp(model_equip);

    let records = metadata
        .game_event_model_equip_like_cpp(1)
        .expect("event 1 exists");
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].spawn_id, 100);
    assert_eq!(metadata.game_event_model_equip_like_cpp(4), None);
}
#[test]
fn game_event_npc_vendor_cache_deactivate_removes_all_item_type_matches_like_cpp() {
    let mut metadata = game_event_npc_vendor_metadata_with_records_like_cpp(
        2,
        &[
            (1, 100, 9001, 6000, 2),
            (1, 101, 9001, 6000, 2),
            (1, 102, 9001, 6000, 3),
            (2, 200, 9001, 6000, 2),
        ],
    );
    metadata.update_game_event_npc_vendor_cache_like_cpp(1, true);
    metadata.update_game_event_npc_vendor_cache_like_cpp(2, true);

    let summary = metadata.update_game_event_npc_vendor_cache_like_cpp(2, false);

    assert_eq!(summary.records_seen, 1);
    assert_eq!(summary.items_removed, 3);
    assert_eq!(summary.no_match, 0);
    assert_eq!(
        metadata
            .game_event_active_npc_vendor_items_like_cpp(9001)
            .iter()
            .map(|record| (record.item, record.vendor_type))
            .collect::<Vec<_>>(),
        vec![(6000, 3)]
    );
}
#[test]
fn game_event_change_equip_or_model_baseline_activate_saves_prev_and_applies_new_like_cpp() {
    let mut model_equip = GameEventModelEquipLikeCpp::from_game_event_max_entry_like_cpp(Some(3));
    assert!(model_equip.push_record_like_cpp(
        1,
        GameEventModelEquipRecordLikeCpp {
            spawn_id: 100,
            model_id: 222,
            model_id_prev: 0,
            equipment_id: 7,
            equipment_id_prev: 0,
        },
    ));
    let mut store = SpawnStore::new();
    store.insert_spawn_metadata_like_cpp(&game_event_guid_test_spawn(
        SpawnObjectType::Creature,
        100,
        0,
    ));
    let mut rows = BTreeMap::new();
    rows.insert(
        100,
        game_event_model_equip_runtime_row_like_cpp(100, 111, 3),
    );
    let mut metadata = CanonicalSpawnMetadataLikeCpp::new(store, BTreeMap::new())
        .with_game_event_model_equip_like_cpp(model_equip)
        .with_creature_runtime_rows_like_cpp(rows);

    let summary = metadata.change_game_event_model_equip_baseline_like_cpp(1, true);

    assert_eq!(summary.records_seen, 1);
    assert_eq!(summary.records_applied, 1);
    let record = &metadata.game_event_model_equip_like_cpp(1).unwrap()[0];
    assert_eq!(record.model_id_prev, 111);
    assert_eq!(record.equipment_id_prev, 3);
    let row = metadata.creature_runtime_row_like_cpp(100).unwrap();
    assert_eq!(row.model_id, 222);
    assert_eq!(row.equipment_id, 7);
}
#[test]
fn game_event_change_equip_or_model_baseline_activate_zero_model_resets_display_like_cpp() {
    let mut model_equip = GameEventModelEquipLikeCpp::from_game_event_max_entry_like_cpp(Some(3));
    assert!(model_equip.push_record_like_cpp(
        1,
        GameEventModelEquipRecordLikeCpp {
            spawn_id: 100,
            model_id: 0,
            model_id_prev: 0,
            equipment_id: 7,
            equipment_id_prev: 0,
        },
    ));
    let mut store = SpawnStore::new();
    store.insert_spawn_metadata_like_cpp(&game_event_guid_test_spawn(
        SpawnObjectType::Creature,
        100,
        0,
    ));
    let mut rows = BTreeMap::new();
    rows.insert(
        100,
        game_event_model_equip_runtime_row_like_cpp(100, 111, 3),
    );
    let mut metadata = CanonicalSpawnMetadataLikeCpp::new(store, BTreeMap::new())
        .with_game_event_model_equip_like_cpp(model_equip)
        .with_creature_runtime_rows_like_cpp(rows);

    let summary = metadata.change_game_event_model_equip_baseline_like_cpp(1, true);

    assert_eq!(summary.records_applied, 1);
    let record = &metadata.game_event_model_equip_like_cpp(1).unwrap()[0];
    assert_eq!(record.model_id_prev, 111);
    assert_eq!(record.equipment_id_prev, 3);
    let row = metadata.creature_runtime_row_like_cpp(100).unwrap();
    assert_eq!(row.model_id, 0);
    assert_eq!(row.equipment_id, 7);
}
#[test]
fn game_event_change_equip_or_model_baseline_deactivate_restores_prev_like_cpp() {
    let mut model_equip = GameEventModelEquipLikeCpp::from_game_event_max_entry_like_cpp(Some(3));
    assert!(model_equip.push_record_like_cpp(
        1,
        GameEventModelEquipRecordLikeCpp {
            spawn_id: 100,
            model_id: 222,
            model_id_prev: 111,
            equipment_id: 7,
            equipment_id_prev: 3,
        },
    ));
    let mut store = SpawnStore::new();
    store.insert_spawn_metadata_like_cpp(&game_event_guid_test_spawn(
        SpawnObjectType::Creature,
        100,
        0,
    ));
    let mut rows = BTreeMap::new();
    rows.insert(
        100,
        game_event_model_equip_runtime_row_like_cpp(100, 222, 7),
    );
    let mut metadata = CanonicalSpawnMetadataLikeCpp::new(store, BTreeMap::new())
        .with_game_event_model_equip_like_cpp(model_equip)
        .with_creature_runtime_rows_like_cpp(rows);

    let summary = metadata.change_game_event_model_equip_baseline_like_cpp(1, false);

    assert_eq!(summary.records_applied, 1);
    let row = metadata.creature_runtime_row_like_cpp(100).unwrap();
    assert_eq!(row.model_id, 111);
    assert_eq!(row.equipment_id, 3);
}
#[test]
fn game_event_change_equip_or_model_baseline_deactivate_zero_prev_model_resets_display_like_cpp() {
    let mut model_equip = GameEventModelEquipLikeCpp::from_game_event_max_entry_like_cpp(Some(3));
    assert!(model_equip.push_record_like_cpp(
        1,
        GameEventModelEquipRecordLikeCpp {
            spawn_id: 100,
            model_id: 222,
            model_id_prev: 0,
            equipment_id: 7,
            equipment_id_prev: 3,
        },
    ));
    let mut store = SpawnStore::new();
    store.insert_spawn_metadata_like_cpp(&game_event_guid_test_spawn(
        SpawnObjectType::Creature,
        100,
        0,
    ));
    let mut rows = BTreeMap::new();
    rows.insert(
        100,
        game_event_model_equip_runtime_row_like_cpp(100, 222, 7),
    );
    let mut metadata = CanonicalSpawnMetadataLikeCpp::new(store, BTreeMap::new())
        .with_game_event_model_equip_like_cpp(model_equip)
        .with_creature_runtime_rows_like_cpp(rows);

    let summary = metadata.change_game_event_model_equip_baseline_like_cpp(1, false);

    assert_eq!(summary.records_applied, 1);
    let row = metadata.creature_runtime_row_like_cpp(100).unwrap();
    assert_eq!(row.model_id, 0);
    assert_eq!(row.equipment_id, 3);
}
#[test]
fn game_event_change_equip_or_model_baseline_missing_row_and_bucket_do_not_panic_like_cpp() {
    let mut model_equip = GameEventModelEquipLikeCpp::from_game_event_max_entry_like_cpp(Some(3));
    assert!(model_equip.push_record_like_cpp(
        1,
        GameEventModelEquipRecordLikeCpp {
            spawn_id: 100,
            model_id: 222,
            model_id_prev: 0,
            equipment_id: 7,
            equipment_id_prev: 0,
        },
    ));
    let mut store = SpawnStore::new();
    store.insert_spawn_metadata_like_cpp(&game_event_guid_test_spawn(
        SpawnObjectType::Creature,
        100,
        0,
    ));
    let mut metadata = CanonicalSpawnMetadataLikeCpp::new(store, BTreeMap::new())
        .with_game_event_model_equip_like_cpp(model_equip);

    let missing_row = metadata.change_game_event_model_equip_baseline_like_cpp(1, true);
    let missing_bucket = metadata.change_game_event_model_equip_baseline_like_cpp(4, true);

    assert_eq!(missing_row.records_seen, 1);
    assert_eq!(missing_row.records_applied, 0);
    assert_eq!(missing_row.missing_creature_runtime_rows, 1);
    assert!(missing_bucket.missing_event_bucket);
}
#[test]
fn game_event_change_equip_or_model_baseline_missing_spawn_metadata_does_not_create_dummy_like_cpp()
{
    let mut model_equip = GameEventModelEquipLikeCpp::from_game_event_max_entry_like_cpp(Some(3));
    assert!(model_equip.push_record_like_cpp(
        1,
        GameEventModelEquipRecordLikeCpp {
            spawn_id: 100,
            model_id: 222,
            model_id_prev: 0,
            equipment_id: 7,
            equipment_id_prev: 0,
        },
    ));
    let mut rows = BTreeMap::new();
    rows.insert(
        100,
        game_event_model_equip_runtime_row_like_cpp(100, 111, 3),
    );
    let mut metadata = CanonicalSpawnMetadataLikeCpp::new(SpawnStore::new(), BTreeMap::new())
        .with_game_event_model_equip_like_cpp(model_equip)
        .with_creature_runtime_rows_like_cpp(rows);

    let summary = metadata.change_game_event_model_equip_baseline_like_cpp(1, true);

    assert_eq!(summary.records_seen, 1);
    assert_eq!(summary.records_applied, 0);
    assert_eq!(summary.missing_spawn_metadata, 1);
    let row = metadata.creature_runtime_row_like_cpp(100).unwrap();
    assert_eq!(row.model_id, 111);
    assert_eq!(row.equipment_id, 3);
}
