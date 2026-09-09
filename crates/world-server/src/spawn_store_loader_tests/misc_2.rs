//! Misc scenarios for [`super`].
//!
//! Split out of spawn_store_loader_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

#[test]
fn game_event_next_check_periodic_delays_and_end_clamp_like_cpp() {
    let store = game_event_store([
        event(1, GameEventStateLikeCpp::Normal, 100, 1_000, 10, 2),
        event(2, GameEventStateLikeCpp::Normal, 900, 1_000, 10, 2),
        event(3, GameEventStateLikeCpp::Normal, 100, 350, 10, 2),
        event(4, GameEventStateLikeCpp::Normal, 100, 500, 0, 2),
    ]);

    assert_eq!(
        store.next_check_like_cpp(1, 1_001),
        GameEventNextCheckOutcomeLikeCpp::DelaySecs(MAX_GAME_EVENT_CHECK_DELAY_SECS_LIKE_CPP)
    );
    assert_eq!(
        store.next_check_like_cpp(2, 600),
        GameEventNextCheckOutcomeLikeCpp::DelaySecs(300)
    );
    assert_eq!(
        store.next_check_like_cpp(1, 150),
        GameEventNextCheckOutcomeLikeCpp::DelaySecs(70)
    );
    assert_eq!(
        store.next_check_like_cpp(1, 221),
        GameEventNextCheckOutcomeLikeCpp::DelaySecs(479)
    );
    assert_eq!(
        store.next_check_like_cpp(3, 221),
        GameEventNextCheckOutcomeLikeCpp::DelaySecs(129)
    );
    assert_eq!(
        store.next_check_like_cpp(4, 150),
        GameEventNextCheckOutcomeLikeCpp::InvalidTimingZeroOccurrence { event_id: 4 }
    );
    assert_eq!(
        store.next_check_like_cpp(9, 150),
        GameEventNextCheckOutcomeLikeCpp::MissingEvent { event_id: 9 }
    );
}
#[test]
fn game_event_data_store_uses_cpp_master_sizing_and_indexing() {
    let mut events = GameEventDataStoreLikeCpp::from_game_event_max_entry_like_cpp(Some(3));
    let mut report = GameEventDataLoadReportLikeCpp::default();

    apply_game_event_data_row_like_cpp(
        game_event_data_row(1, 10, GameEventStateLikeCpp::Normal as u8, 0),
        &mut events,
        &mut report,
    );
    apply_game_event_data_row_like_cpp(
        game_event_data_row(3, 10, GameEventStateLikeCpp::Normal as u8, 0),
        &mut events,
        &mut report,
    );

    assert_eq!(events.len_like_cpp(), 4);
    assert!(events.event_like_cpp(0).is_some());
    assert_eq!(
        events.event_like_cpp(1).map(|event| event.event_id),
        Some(1)
    );
    assert_eq!(
        events.event_like_cpp(3).map(|event| event.event_id),
        Some(3)
    );
    assert!(events.event_like_cpp(4).is_none());
    assert_eq!(report.rows, 2);
    assert_eq!(report.loaded, 2);
}
#[test]
fn game_event_data_preserves_cpp_field_order_and_next_start_zero() {
    let mut events = GameEventDataStoreLikeCpp::from_game_event_max_entry_like_cpp(Some(3));
    let mut report = GameEventDataLoadReportLikeCpp::default();

    apply_game_event_data_row_like_cpp(
        GameEventDataRowLikeCpp {
            event_id: 2,
            start: 1_700_000_001,
            end: 1_700_000_999,
            occurence: 120,
            length: 45,
            holiday_id: 341,
            holiday_stage: 3,
            description: "Darkmoon metadata".to_string(),
            state_raw: GameEventStateLikeCpp::WorldConditions as u8,
            announce: 2,
        },
        &mut events,
        &mut report,
    );

    let event = events.event_like_cpp(2).unwrap();
    assert_eq!(event.start, 1_700_000_001);
    assert_eq!(event.end, 1_700_000_999);
    assert_eq!(event.occurence, 120);
    assert_eq!(event.length, 45);
    assert_eq!(event.holiday_id, 341);
    assert_eq!(event.holiday_stage, 3);
    assert_eq!(event.description, "Darkmoon metadata");
    assert_eq!(
        event.state_raw,
        GameEventStateLikeCpp::WorldConditions as u8
    );
    assert_eq!(
        event.state_like_cpp(),
        Some(GameEventStateLikeCpp::WorldConditions)
    );
    assert_eq!(event.announce, 2);
    assert_eq!(event.next_start, 0);
    assert_eq!(report.loaded, 1);
}
#[test]
fn game_event_data_validity_matches_cpp_normal_zero_length_rule() {
    let mut events = GameEventDataStoreLikeCpp::from_game_event_max_entry_like_cpp(Some(3));
    let mut report = GameEventDataLoadReportLikeCpp::default();

    apply_game_event_data_row_like_cpp(
        game_event_data_row(1, 0, GameEventStateLikeCpp::Normal as u8, 0),
        &mut events,
        &mut report,
    );
    apply_game_event_data_row_like_cpp(
        game_event_data_row(2, 0, GameEventStateLikeCpp::WorldInactive as u8, 0),
        &mut events,
        &mut report,
    );
    apply_game_event_data_row_like_cpp(
        game_event_data_row(3, 0, GameEventStateLikeCpp::Internal as u8, 0),
        &mut events,
        &mut report,
    );

    assert!(!events.event_like_cpp(1).unwrap().is_valid_like_cpp());
    assert!(events.event_like_cpp(2).unwrap().is_valid_like_cpp());
    assert!(events.event_like_cpp(3).unwrap().is_valid_like_cpp());
    assert_eq!(report.rows, 3);
    assert_eq!(report.loaded, 3);
    assert_eq!(report.invalid_normal_zero_length, 1);
}
#[test]
fn game_event_data_preserves_holiday_values_and_defers_db2_validation() {
    let mut events = GameEventDataStoreLikeCpp::from_game_event_max_entry_like_cpp(Some(3));
    let mut report = GameEventDataLoadReportLikeCpp::default();

    apply_game_event_data_row_like_cpp(
        game_event_data_row(1, 10, GameEventStateLikeCpp::Normal as u8, 777),
        &mut events,
        &mut report,
    );

    let event = events.event_like_cpp(1).unwrap();
    assert_eq!(event.holiday_id, 777);
    assert_eq!(event.holiday_stage, 2);
    assert_eq!(event.start, 100);
    assert_eq!(event.end, 200);
    assert_eq!(report.holiday_validation_deferred, 1);
    assert_eq!(report.loaded, 1);
}
#[test]
fn game_event_data_skip_out_of_range_without_truncation() {
    let mut events = GameEventDataStoreLikeCpp::from_game_event_max_entry_like_cpp(Some(3));
    let mut report = GameEventDataLoadReportLikeCpp::default();

    apply_game_event_data_row_like_cpp(
        game_event_data_row(4, 10, GameEventStateLikeCpp::Normal as u8, 0),
        &mut events,
        &mut report,
    );

    assert_eq!(events.len_like_cpp(), 4);
    assert!(events.event_like_cpp(4).is_none());
    assert_eq!(report.rows, 1);
    assert_eq!(report.loaded, 0);
    assert_eq!(report.skipped_out_of_range, 1);
}
#[test]
fn game_event_pool_ids_preserve_order_and_signed_internal_index_like_cpp() {
    let mgr = game_event_pool_mgr_with_test_pools();
    let mut pools = GameEventPoolIdsLikeCpp::from_game_event_max_entry_like_cpp(Some(3));
    let mut report = GameEventPoolLoadReportLikeCpp::default();

    for row in [
        GameEventPoolRowLikeCpp {
            pool_entry: 10,
            event_id: 1,
        },
        GameEventPoolRowLikeCpp {
            pool_entry: 11,
            event_id: -1,
        },
        GameEventPoolRowLikeCpp {
            pool_entry: 12,
            event_id: 1,
        },
        GameEventPoolRowLikeCpp {
            pool_entry: 13,
            event_id: -1,
        },
    ] {
        apply_game_event_pool_row_like_cpp(row, &mgr, &mut pools, &mut report);
    }

    assert_eq!(pools.game_event_size_like_cpp(), 4);
    assert_eq!(pools.internal_event_id_like_cpp(1), Some(4));
    assert_eq!(pools.internal_event_id_like_cpp(-1), Some(2));
    assert_eq!(pools.pool_ids_like_cpp(1), Some([10, 12].as_slice()));
    assert_eq!(pools.pool_ids_like_cpp(-1), Some([11, 13].as_slice()));
    assert_eq!(report.rows, 4);
    assert_eq!(report.loaded, 4);
}
#[test]
fn game_event_pool_ids_skip_out_of_range_without_panic_like_cpp() {
    let mgr = game_event_pool_mgr_with_test_pools();
    let mut pools = GameEventPoolIdsLikeCpp::from_game_event_max_entry_like_cpp(Some(3));
    let mut report = GameEventPoolLoadReportLikeCpp::default();

    apply_game_event_pool_row_like_cpp(
        GameEventPoolRowLikeCpp {
            pool_entry: 10,
            event_id: -5,
        },
        &mgr,
        &mut pools,
        &mut report,
    );
    apply_game_event_pool_row_like_cpp(
        GameEventPoolRowLikeCpp {
            pool_entry: 11,
            event_id: 4,
        },
        &mgr,
        &mut pools,
        &mut report,
    );

    assert_eq!(pools.pool_ids_like_cpp(-5), None);
    assert_eq!(pools.pool_ids_like_cpp(4), None);
    assert_eq!(report.rows, 2);
    assert_eq!(report.loaded, 0);
    assert_eq!(report.skipped_out_of_range, 2);
}
#[test]
fn game_event_pool_ids_skip_broken_pool_but_keep_pool_mgr_metadata_like_cpp() {
    let mgr = game_event_pool_mgr_with_test_pools();
    let mut pools = GameEventPoolIdsLikeCpp::from_game_event_max_entry_like_cpp(Some(3));
    let mut report = GameEventPoolLoadReportLikeCpp::default();

    apply_game_event_pool_row_like_cpp(
        GameEventPoolRowLikeCpp {
            pool_entry: 99,
            event_id: 1,
        },
        &mgr,
        &mut pools,
        &mut report,
    );
    apply_game_event_pool_row_like_cpp(
        GameEventPoolRowLikeCpp {
            pool_entry: 404,
            event_id: 1,
        },
        &mgr,
        &mut pools,
        &mut report,
    );
    apply_game_event_pool_row_like_cpp(
        GameEventPoolRowLikeCpp {
            pool_entry: 10,
            event_id: 1,
        },
        &mgr,
        &mut pools,
        &mut report,
    );

    assert!(mgr.templates.contains_key(&99));
    assert!(!mgr.check_pool_like_cpp(99));
    assert_eq!(pools.pool_ids_like_cpp(1), Some([10].as_slice()));
    assert_eq!(report.rows, 3);
    assert_eq!(report.loaded, 1);
    assert_eq!(report.skipped_broken_pool, 2);
}
#[test]
fn game_event_npc_flag_get_npc_flag_or_over_active_events_like_cpp() {
    let mut npc_flags = GameEventNpcFlagsLikeCpp::from_game_event_max_entry_like_cpp(Some(3));
    assert!(npc_flags.push_record_like_cpp(
        1,
        GameEventNpcFlagRecordLikeCpp {
            spawn_id: 100,
            npcflag: 0x1,
        },
    ));
    assert!(npc_flags.push_record_like_cpp(
        2,
        GameEventNpcFlagRecordLikeCpp {
            spawn_id: 100,
            npcflag: 0x1_0000_0002,
        },
    ));
    assert!(npc_flags.push_record_like_cpp(
        2,
        GameEventNpcFlagRecordLikeCpp {
            spawn_id: 101,
            npcflag: 0x80,
        },
    ));
    assert!(npc_flags.push_record_like_cpp(
        3,
        GameEventNpcFlagRecordLikeCpp {
            spawn_id: 100,
            npcflag: 0x4,
        },
    ));
    let metadata = CanonicalSpawnMetadataLikeCpp::new(SpawnStore::new(), BTreeMap::new())
        .with_game_event_npc_flags_like_cpp(npc_flags);

    assert_eq!(
        metadata.game_event_npc_flag_mask_like_cpp(100, &[1, 2, 99]),
        0x1_0000_0003
    );
    assert_eq!(
        metadata.game_event_npc_flag_mask_like_cpp(101, &[1, 2]),
        0x80
    );
    assert_eq!(metadata.game_event_npc_flag_mask_like_cpp(100, &[3]), 0x4);
    assert_eq!(metadata.game_event_npc_flag_mask_like_cpp(100, &[]), 0);
    assert_eq!(metadata.game_event_npc_flag_mask_like_cpp(999, &[1, 2]), 0);
}
#[test]
fn game_event_npc_vendor_sizing_records_and_out_of_range_like_cpp() {
    let store = game_event_npc_vendor_store(&[(100, 9001)]);
    let npc_flags = GameEventNpcFlagsLikeCpp::from_game_event_max_entry_like_cpp(Some(2));
    let mut vendors = GameEventNpcVendorsLikeCpp::from_game_event_max_entry_like_cpp(Some(2));
    let mut report = GameEventNpcVendorLoadReportLikeCpp::default();

    apply_game_event_npc_vendor_row_like_cpp(
        game_event_npc_vendor_row(2, 100, 6000),
        &store,
        &npc_flags,
        &mut vendors,
        &mut report,
    );
    apply_game_event_npc_vendor_row_like_cpp(
        game_event_npc_vendor_row(3, 100, 6001),
        &store,
        &npc_flags,
        &mut vendors,
        &mut report,
    );

    assert_eq!(vendors.records_like_cpp(0).unwrap(), &[]);
    assert_eq!(vendors.records_like_cpp(1).unwrap(), &[]);
    assert_eq!(vendors.records_like_cpp(2).unwrap()[0].item, 6000);
    assert_eq!(vendors.records_like_cpp(3), None);
    assert_eq!(report.rows, 2);
    assert_eq!(report.loaded, 1);
    assert_eq!(report.skipped_out_of_range, 1);
    assert_eq!(report.validation_deferred, 1);
}
#[test]
fn game_event_npc_vendor_event_entry_uses_get_uint8_truncation_like_cpp() {
    let store = game_event_npc_vendor_store(&[(100, 9001)]);
    let npc_flags = GameEventNpcFlagsLikeCpp::from_game_event_max_entry_like_cpp(Some(2));
    let mut vendors = GameEventNpcVendorsLikeCpp::from_game_event_max_entry_like_cpp(Some(2));
    let mut report = GameEventNpcVendorLoadReportLikeCpp::default();

    apply_game_event_npc_vendor_row_like_cpp(
        game_event_npc_vendor_row_from_raw_event_entry_get_uint8_like_cpp(258, 100, 6000),
        &store,
        &npc_flags,
        &mut vendors,
        &mut report,
    );

    assert_eq!(vendors.records_like_cpp(2).unwrap()[0].item, 6000);
    assert_eq!(vendors.records_like_cpp(258), None);
    assert_eq!(report.rows, 1);
    assert_eq!(report.loaded, 1);
    assert_eq!(report.skipped_out_of_range, 0);
}
#[test]
fn game_event_npc_vendor_preserves_order_and_lookup_by_entry_like_cpp() {
    let store = game_event_npc_vendor_store(&[(100, 9001), (101, 9001), (102, 9002)]);
    let npc_flags = GameEventNpcFlagsLikeCpp::from_game_event_max_entry_like_cpp(Some(1));
    let mut vendors = GameEventNpcVendorsLikeCpp::from_game_event_max_entry_like_cpp(Some(1));
    let mut report = GameEventNpcVendorLoadReportLikeCpp::default();

    for row in [
        game_event_npc_vendor_row(1, 100, 6000),
        game_event_npc_vendor_row(1, 101, 6001),
        game_event_npc_vendor_row(1, 102, 6002),
    ] {
        apply_game_event_npc_vendor_row_like_cpp(
            row,
            &store,
            &npc_flags,
            &mut vendors,
            &mut report,
        );
    }

    let records = vendors.records_like_cpp(1).unwrap();
    assert_eq!(
        records.iter().map(|record| record.item).collect::<Vec<_>>(),
        vec![6000, 6001, 6002]
    );
    let entry_records = vendors.records_for_entry_like_cpp(1, 9001).unwrap();
    assert_eq!(entry_records.len(), 2);
    assert_eq!(entry_records[0].spawn_id, 100);
    assert_eq!(entry_records[1].spawn_id, 101);
}
#[test]
fn game_event_npc_vendor_event_npc_flag_first_match_low32_or_zero_like_cpp() {
    let store = game_event_npc_vendor_store(&[(100, 9001), (101, 9002)]);
    let mut npc_flags = GameEventNpcFlagsLikeCpp::from_game_event_max_entry_like_cpp(Some(1));
    assert!(npc_flags.push_record_like_cpp(
        1,
        GameEventNpcFlagRecordLikeCpp {
            spawn_id: 100,
            npcflag: 0x1_0000_00AA,
        },
    ));
    assert!(npc_flags.push_record_like_cpp(
        1,
        GameEventNpcFlagRecordLikeCpp {
            spawn_id: 100,
            npcflag: 0xBB,
        },
    ));
    let mut vendors = GameEventNpcVendorsLikeCpp::from_game_event_max_entry_like_cpp(Some(1));
    let mut report = GameEventNpcVendorLoadReportLikeCpp::default();

    apply_game_event_npc_vendor_row_like_cpp(
        game_event_npc_vendor_row(1, 100, 6000),
        &store,
        &npc_flags,
        &mut vendors,
        &mut report,
    );
    apply_game_event_npc_vendor_row_like_cpp(
        game_event_npc_vendor_row(1, 101, 6001),
        &store,
        &npc_flags,
        &mut vendors,
        &mut report,
    );

    let records = vendors.records_like_cpp(1).unwrap();
    assert_eq!(records[0].event_npc_flag_low32, 0xAA);
    assert_eq!(records[1].event_npc_flag_low32, 0);
}
#[test]
fn game_event_npc_vendor_bonus_list_ids_parse_like_cpp() {
    assert_eq!(
        parse_game_event_npc_vendor_bonus_list_ids_like_cpp("7 bad -9 7 0x10 12"),
        vec![7, -9, 7, 12]
    );
}
#[test]
fn game_event_npc_vendor_metadata_accessor_like_cpp() {
    let mut vendors = GameEventNpcVendorsLikeCpp::from_game_event_max_entry_like_cpp(Some(1));
    assert!(
        vendors.push_record_like_cpp(1, game_event_npc_vendor_record_like_cpp(100, 9001, 6000, 2),)
    );
    let metadata = CanonicalSpawnMetadataLikeCpp::new(SpawnStore::new(), BTreeMap::new())
        .with_game_event_npc_vendors_like_cpp(vendors);

    assert_eq!(
        metadata.game_event_npc_vendors_like_cpp(1).unwrap().len(),
        1
    );
    assert_eq!(
        metadata
            .game_event_npc_vendor_records_for_entry_like_cpp(1, 9001)
            .unwrap()[0]
            .item,
        6000
    );
    assert_eq!(metadata.game_event_npc_vendors_like_cpp(2), None);
}
#[test]
fn game_event_npc_vendor_cache_activate_appends_without_dedupe_like_cpp() {
    let mut metadata = game_event_npc_vendor_metadata_with_records_like_cpp(
        1,
        &[
            (1, 100, 9001, 6000, 2),
            (1, 101, 9001, 6000, 2),
            (1, 102, 9001, 6001, 2),
        ],
    );

    let first = metadata.update_game_event_npc_vendor_cache_like_cpp(1, true);
    let second = metadata.update_game_event_npc_vendor_cache_like_cpp(1, true);

    assert_eq!(first.records_seen, 3);
    assert_eq!(first.items_added, 3);
    assert_eq!(second.records_seen, 3);
    assert_eq!(second.items_added, 3);
    assert_eq!(
        metadata
            .game_event_active_npc_vendor_items_like_cpp(9001)
            .iter()
            .map(|record| record.item)
            .collect::<Vec<_>>(),
        vec![6000, 6000, 6001, 6000, 6000, 6001]
    );
}
#[test]
fn game_event_npc_vendor_cache_deactivate_miss_and_no_match_no_panic_like_cpp() {
    let mut metadata = game_event_npc_vendor_metadata_with_records_like_cpp(
        2,
        &[(1, 100, 9001, 6000, 2), (2, 200, 9002, 6001, 2)],
    );
    metadata.update_game_event_npc_vendor_cache_like_cpp(1, true);

    let summary = metadata.update_game_event_npc_vendor_cache_like_cpp(2, false);

    assert_eq!(summary.records_seen, 1);
    assert_eq!(summary.remove_misses, 1);
    assert_eq!(summary.items_removed, 0);
    assert_eq!(
        metadata.game_event_active_npc_vendor_items_like_cpp(9001)[0].item,
        6000
    );

    let mut metadata = game_event_npc_vendor_metadata_with_records_like_cpp(
        2,
        &[(1, 100, 9001, 6000, 2), (2, 200, 9001, 6001, 2)],
    );
    metadata.update_game_event_npc_vendor_cache_like_cpp(1, true);
    let no_match = metadata.update_game_event_npc_vendor_cache_like_cpp(2, false);
    assert_eq!(no_match.no_match, 1);
    assert_eq!(no_match.items_removed, 0);
}
#[test]
fn game_event_npc_vendor_cache_preserves_order_per_entry_like_cpp() {
    let mut metadata = game_event_npc_vendor_metadata_with_records_like_cpp(
        2,
        &[
            (1, 100, 9001, 6000, 2),
            (1, 101, 9002, 7000, 2),
            (1, 102, 9001, 6001, 2),
            (2, 200, 9001, 6002, 2),
        ],
    );

    metadata.update_game_event_npc_vendor_cache_like_cpp(1, true);
    metadata.update_game_event_npc_vendor_cache_like_cpp(2, true);

    assert_eq!(
        metadata
            .game_event_active_npc_vendor_items_like_cpp(9001)
            .iter()
            .map(|record| record.item)
            .collect::<Vec<_>>(),
        vec![6000, 6001, 6002]
    );
    assert_eq!(
        metadata.game_event_active_npc_vendor_items_like_cpp(9002)[0].item,
        7000
    );
}
