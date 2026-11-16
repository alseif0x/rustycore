use super::super::*;
use crate::Db2HotfixRemovalStoreLikeCpp;

fn bonus(id: u32, parent: u16, order: u8, value: i32) -> ItemBonusRecord {
    ItemBonusRecord {
        id,
        values: [value, -1, i32::MIN, i32::MAX],
        parent,
        bonus_type: 253,
        order,
    }
}

fn azerite(id: u32, item: i32) -> AzeriteEmpoweredItemRecord {
    AzeriteEmpoweredItemRecord {
        id,
        item,
        tier_unlock_set: u32::MAX,
        power_set: 0x8000_0001,
    }
}

fn squish(id: u32, patch: i32) -> ItemSquishEraRecord {
    ItemSquishEraRecord {
        id,
        patch,
        curve: i32::MIN,
        flags: u32::MAX as i32,
    }
}

fn removals(rows: impl IntoIterator<Item = (u32, i32, u8)>) -> Db2HotfixRemovalStoreLikeCpp {
    Db2HotfixRemovalStoreLikeCpp::from_status_rows_like_cpp(rows)
}

#[test]
fn five_tables_overlay_then_remove_before_ordered_and_reverse_indexes() {
    let baseline = ItemRecords {
        item_bonuses: vec![
            bonus(12, 7, 0, 1),
            bonus(3, 7, 255, 3),
            bonus(20, u16::MAX, 4, 20),
        ],
        azerite_empowered_items: vec![azerite(1, 5), azerite(2, 6)],
        item_scaling_configs: vec![ItemScalingConfigRecord {
            id: 4,
            offset_curve: -1,
            item_level: 20,
            required_level: 30,
            squish_era: 40,
            flags: 50,
        }],
        item_offset_curves: vec![ItemOffsetCurveRecord {
            id: 2,
            curve: 3,
            offset: 4,
        }],
        item_squish_eras: vec![squish(2, 1)],
        item_squish_era_file_last_index: Some(4),
        unknown_baseline_records: [1, 2, 3, 4, 5, 6, 7, 8, 9],
        ..Default::default()
    };
    let official = ItemRecords {
        item_bonuses: vec![bonus(12, 7, 0, 12), bonus(5, 7, 1, 5)],
        azerite_empowered_items: vec![azerite(8, 5), azerite(9, 7)],
        item_scaling_configs: vec![ItemScalingConfigRecord {
            id: 4,
            offset_curve: 8,
            item_level: 9,
            required_level: 10,
            squish_era: 11,
            flags: 12,
        }],
        item_offset_curves: vec![ItemOffsetCurveRecord {
            id: 2,
            curve: 7,
            offset: 8,
        }],
        ..Default::default()
    };
    let custom = ItemRecords {
        item_bonuses: vec![bonus(12, 7, 0, 120), bonus(6, 7, 2, 6)],
        azerite_empowered_items: vec![azerite(10, 5), azerite(11, 7)],
        item_scaling_configs: vec![ItemScalingConfigRecord {
            id: 4,
            offset_curve: i32::MIN,
            item_level: -1,
            required_level: 0x0102_0304,
            squish_era: i32::MAX,
            flags: 0x8000_0001u32 as i32,
        }],
        item_offset_curves: vec![ItemOffsetCurveRecord {
            id: 2,
            curve: i32::MIN,
            offset: -1,
        }],
        ..Default::default()
    };
    let removals = removals([
        (ITEM_BONUS_HASH, 5, 2),
        (AZERITE_EMPOWERED_ITEM_HASH, 10, 2),
    ]);
    let catalog = baseline.finish(official, custom, &removals).unwrap();

    let rows = catalog.item_bonuses_for_list(7).collect::<Vec<_>>();
    assert_eq!(
        rows.iter().map(|row| row.id).collect::<Vec<_>>(),
        [3, 6, 12]
    );
    assert_eq!(
        rows.iter().map(|row| row.order).collect::<Vec<_>>(),
        [255, 2, 0]
    );
    assert_eq!(catalog.item_bonuses_for_list(0x1_0007).count(), 0);
    assert_eq!(catalog.item_bonus(12).unwrap().values[0], 120);
    assert_eq!(
        catalog
            .item_bonuses_for_list(u16::MAX as u32)
            .next()
            .unwrap()
            .id,
        20
    );
    assert_eq!(catalog.azerite_empowered_item_for_item(5).unwrap().id, 8);
    assert_eq!(catalog.azerite_empowered_item_for_item(7).unwrap().id, 11);
    assert_eq!(
        catalog.azerite_empowered_item_for_item(u32::MAX).is_none(),
        true
    );
    assert_eq!(
        catalog.item_scaling_config(4).unwrap().flags,
        0x8000_0001u32 as i32
    );
    assert_eq!(catalog.item_offset_curve(2).unwrap().curve, i32::MIN);
    assert_eq!(
        catalog.unknown_baseline_records(),
        [1, 2, 3, 4, 5, 6, 7, 8, 9]
    );
    assert_eq!(catalog.counts(), [0, 0, 0, 0, 4, 5, 1, 1, 1]);
}

fn squish_baseline_and_official() -> (ItemRecords, ItemRecords) {
    (
        ItemRecords {
            item_squish_eras: vec![squish(2, 1)],
            item_squish_era_file_last_index: Some(4),
            ..Default::default()
        },
        ItemRecords {
            item_squish_eras: vec![squish(2, 10)],
            item_squish_era_sql_index_size: Some(12),
            ..Default::default()
        },
    )
}

#[test]
fn squish_rejects_rows_outside_the_custom_reallocation() {
    let (baseline, official) = squish_baseline_and_official();
    let custom = ItemRecords {
        item_squish_eras: vec![squish(7, 20), squish(7, 30), squish(10, 40)],
        item_squish_era_sql_index_size: Some(10),
        ..Default::default()
    };
    match baseline.finish(official, custom, &removals([])) {
        Err(error) => assert_eq!(
            error.to_string(),
            "Item squish SQL row exceeds observed source allocation"
        ),
        Ok(_) => panic!("row 10 must not fit the custom allocation of 10"),
    }
}

#[test]
fn squish_new_custom_row_grows_the_published_bound() {
    let (baseline, official) = squish_baseline_and_official();
    let custom = ItemRecords {
        item_squish_eras: vec![squish(7, 20), squish(7, 30), squish(8, 40)],
        item_squish_era_sql_index_size: Some(10),
        ..Default::default()
    };
    let catalog = baseline.finish(official, custom, &removals([])).unwrap();
    assert_eq!(catalog.item_squish_era_storage_last_index(), 9);
    assert_eq!(catalog.item_squish_era(2).unwrap().patch, 10);
    assert_eq!(catalog.item_squish_era(7).unwrap().patch, 30);
    assert!(catalog.item_squish_era(8).is_some());
}

#[test]
fn squish_retains_prior_allocation_but_not_rows_outside_the_published_bound() {
    let (baseline, official) = squish_baseline_and_official();
    let custom = ItemRecords {
        item_squish_eras: vec![squish(7, 20), squish(7, 30), squish(10, 40)],
        item_squish_era_sql_index_size: Some(5),
        ..Default::default()
    };
    let catalog = baseline.finish(official, custom, &removals([])).unwrap();
    assert_eq!(catalog.item_squish_era_storage_last_index(), 4);
    assert_eq!(catalog.item_squish_era(2).unwrap().patch, 10);
    assert!(catalog.item_squish_era(7).is_none());
    assert!(catalog.item_squish_era(10).is_none());
}

#[test]
fn squish_overwrite_only_batches_do_not_grow_the_published_bound() {
    let (baseline, official) = squish_baseline_and_official();
    let catalog = baseline
        .finish(official, Default::default(), &removals([]))
        .unwrap();
    assert_eq!(catalog.item_squish_era_storage_last_index(), 4);
    assert_eq!(catalog.item_squish_era(2).unwrap().patch, 10);
}

#[test]
fn final_squish_removal_does_not_shrink_the_source_storage_bound() {
    let baseline = ItemRecords {
        item_squish_eras: vec![squish(2, 1)],
        item_squish_era_file_last_index: Some(8),
        ..Default::default()
    };
    let custom = ItemRecords {
        item_squish_eras: vec![squish(7, 7)],
        item_squish_era_sql_index_size: Some(12),
        ..Default::default()
    };
    let catalog = baseline
        .finish(
            Default::default(),
            custom,
            &removals([(ITEM_SQUISH_ERA_HASH, 7, 2)]),
        )
        .unwrap();
    assert_eq!(catalog.item_squish_era_storage_last_index(), 11);
    assert!(catalog.item_squish_era(7).is_none());
    assert_eq!(catalog.counts()[8], 1);
}

#[test]
fn squish_requires_explicit_nonempty_bounds_and_keeps_empty_store_index_zero() {
    assert_eq!(
        ItemRecords::default()
            .finish(Default::default(), Default::default(), &removals([]))
            .unwrap()
            .item_squish_era_storage_last_index(),
        0
    );
    let missing_baseline_bound = ItemRecords {
        item_squish_eras: vec![squish(1, 1)],
        ..Default::default()
    };
    assert!(
        missing_baseline_bound
            .finish(Default::default(), Default::default(), &removals([]))
            .is_err()
    );
    let baseline = ItemRecords {
        item_squish_era_file_last_index: Some(4),
        ..Default::default()
    };
    let missing_sql_bound = ItemRecords {
        item_squish_eras: vec![squish(1, 1)],
        ..Default::default()
    };
    assert!(
        baseline
            .finish(Default::default(), missing_sql_bound, &removals([]))
            .is_err()
    );
    let bad_empty_query = ItemRecords {
        item_squish_era_sql_index_size: Some(10),
        ..Default::default()
    };
    assert!(
        ItemRecords {
            item_squish_era_file_last_index: Some(4),
            ..Default::default()
        }
        .finish(Default::default(), bad_empty_query, &removals([]))
        .is_err()
    );
}

#[test]
fn squish_rejects_out_of_bound_and_wrong_provenance_metadata() {
    let bad_baseline = ItemRecords {
        item_squish_eras: vec![squish(5, 1)],
        item_squish_era_file_last_index: Some(4),
        ..Default::default()
    };
    assert!(
        bad_baseline
            .finish(Default::default(), Default::default(), &removals([]))
            .is_err()
    );
    let bad_file_bound_overlay = ItemRecords {
        item_squish_era_file_last_index: Some(0),
        ..Default::default()
    };
    assert!(
        ItemRecords {
            item_squish_era_file_last_index: Some(4),
            ..Default::default()
        }
        .finish(Default::default(), bad_file_bound_overlay, &removals([]))
        .is_err()
    );
    let bad_unknown_overlay = ItemRecords {
        unknown_baseline_records: [1, 0, 0, 0, 0, 0, 0, 0, 0],
        ..Default::default()
    };
    assert!(
        ItemRecords {
            item_squish_era_file_last_index: Some(4),
            ..Default::default()
        }
        .finish(Default::default(), bad_unknown_overlay, &removals([]))
        .is_err()
    );
    let bad_max_index = ItemRecords {
        item_squish_era_file_last_index: Some(u32::MAX),
        ..Default::default()
    };
    assert!(
        bad_max_index
            .finish(Default::default(), Default::default(), &removals([]))
            .is_err()
    );
}
