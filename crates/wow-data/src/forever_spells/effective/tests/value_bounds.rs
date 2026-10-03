use super::*;
use crate::forever_spells::value_input_fixtures as rows;

#[test]
fn damage_fallback_borrows_exact_lookup_first_else_the_last_reserved_slot() {
    let c = finish(
        SpellRecords {
            rand_prop_points: vec![rows::rand_prop_points(1), rows::rand_prop_points(7)],
            ..Default::default()
        },
        Default::default(),
        Default::default(),
    )
    .unwrap();
    assert!(std::ptr::eq(
        c.rand_prop_points_or_last(1).unwrap(),
        c.rand_prop_points(1).unwrap()
    ));
    assert!(std::ptr::eq(
        c.rand_prop_points_or_last(2).unwrap(),
        c.rand_prop_points(7).unwrap()
    ));
    assert_eq!(c.rand_prop_points_storage_last_index, 7);
}

#[test]
fn final_removal_preserves_index_size_and_never_substitutes_an_earlier_survivor() {
    let c = SpellRecords {
        rand_prop_points: vec![rows::rand_prop_points(1), rows::rand_prop_points(7)],
        ..Default::default()
    }
    .finish(
        Default::default(),
        Default::default(),
        6,
        Default::default(),
        Default::default(),
        &removals([(SPELL_TABLE_HASHES[46], 7, 2)]),
    )
    .unwrap();
    assert_eq!(c.rand_prop_points_storage_last_index, 7);
    assert_eq!(c.rand_prop_points_or_last(1).unwrap().id, 1);
    assert!(c.rand_prop_points_or_last(2).is_err());
}

#[test]
fn unresolved_file_copy_destination_reserves_a_slot_without_fabricating_its_row() {
    let c = finish(
        SpellRecords {
            rand_prop_points: vec![rows::rand_prop_points(7)],
            rand_prop_points_storage_last_index: Some(9),
            ..Default::default()
        },
        Default::default(),
        Default::default(),
    )
    .unwrap();
    assert_eq!(c.rand_prop_points_storage_last_index, 9);
    assert_eq!(c.rand_prop_points_or_last(7).unwrap().id, 7);
    assert!(c.rand_prop_points_or_last(8).is_err());
    assert!(
        finish(
            SpellRecords {
                rand_prop_points: vec![rows::rand_prop_points(7)],
                rand_prop_points_storage_last_index: Some(6),
                ..Default::default()
            },
            Default::default(),
            Default::default()
        )
        .is_err()
    );
}

#[test]
fn complementary_sql_batches_extend_the_bound_before_final_removal() {
    let c = SpellRecords {
        rand_prop_points: vec![rows::rand_prop_points(7)],
        ..Default::default()
    }
    .finish(
        SpellRecords {
            rand_prop_points: vec![rows::rand_prop_points(8)],
            ..Default::default()
        },
        SpellRecords {
            rand_prop_points: vec![rows::rand_prop_points(9)],
            ..Default::default()
        },
        6,
        Default::default(),
        Default::default(),
        &removals([(SPELL_TABLE_HASHES[46], 9, 2)]),
    )
    .unwrap();
    assert_eq!(c.rand_prop_points_storage_last_index, 9);
    assert_eq!(c.rand_prop_points_or_last(8).unwrap().id, 8);
    assert!(c.rand_prop_points_or_last(6).is_err());
    assert!(
        finish(
            Default::default(),
            SpellRecords {
                rand_prop_points_storage_last_index: Some(7),
                ..Default::default()
            },
            Default::default()
        )
        .is_err()
    );
}

#[test]
fn separate_sql_max_reserves_a_missing_last_slot_when_the_batch_adds_a_row() {
    let c = finish(
        SpellRecords {
            rand_prop_points: vec![rows::rand_prop_points(7)],
            ..Default::default()
        },
        SpellRecords {
            rand_prop_points: vec![rows::rand_prop_points(8)],
            rand_prop_points_sql_index_size: Some(12),
            ..Default::default()
        },
        Default::default(),
    )
    .unwrap();
    assert_eq!(c.rand_prop_points_storage_last_index, 11);
    assert_eq!(c.rand_prop_points_or_last(8).unwrap().id, 8);
    assert!(c.rand_prop_points_or_last(9).is_err());
}

#[test]
fn overwrite_only_sql_batch_does_not_publish_its_larger_reserved_size() {
    let mut replacement = rows::rand_prop_points(7);
    replacement.damage_replace_stat_f = 123.5;
    let c = finish(
        SpellRecords {
            rand_prop_points: vec![rows::rand_prop_points(7)],
            ..Default::default()
        },
        SpellRecords {
            rand_prop_points: vec![replacement],
            rand_prop_points_sql_index_size: Some(12),
            ..Default::default()
        },
        Default::default(),
    )
    .unwrap();
    assert_eq!(c.rand_prop_points_storage_last_index, 7);
    assert_eq!(
        c.rand_prop_points_or_last(99)
            .unwrap()
            .damage_replace_stat_f,
        123.5
    );
}

#[test]
fn source_physical_capacity_and_public_counter_diverge_after_concurrent_sql_changes() {
    let baseline = || SpellRecords {
        rand_prop_points: vec![rows::rand_prop_points(7)],
        ..Default::default()
    };
    let official = || SpellRecords {
        rand_prop_points: vec![rows::rand_prop_points(7)],
        rand_prop_points_sql_index_size: Some(12),
        ..Default::default()
    };
    // The second MAX no longer sees the high row. Its physical write fits the
    // previous allocation, but records remains 8: LookupEntry(10) is absent.
    let c = finish(
        baseline(),
        official(),
        SpellRecords {
            rand_prop_points: vec![rows::rand_prop_points(10)],
            rand_prop_points_sql_index_size: Some(8),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(c.rand_prop_points_storage_last_index, 7);
    assert!(c.rand_prop_points(10).is_none());
    assert_eq!(c.rand_prop_points_or_last(10).unwrap().id, 7);
    // If the second MAX exceeds records, source replaces the array, copying
    // only its published prefix. The previous larger capacity is not kept.
    assert!(
        finish(
            baseline(),
            official(),
            SpellRecords {
                rand_prop_points: vec![rows::rand_prop_points(10)],
                rand_prop_points_sql_index_size: Some(9),
                ..Default::default()
            }
        )
        .is_err()
    );
}

#[test]
fn sql_size_observation_is_not_a_baseline_or_empty_batch_declaration() {
    for (baseline, official) in [
        (
            SpellRecords {
                rand_prop_points_sql_index_size: Some(8),
                ..Default::default()
            },
            SpellRecords::default(),
        ),
        (
            SpellRecords::default(),
            SpellRecords {
                rand_prop_points_sql_index_size: Some(8),
                ..Default::default()
            },
        ),
    ] {
        assert!(finish(baseline, official, Default::default()).is_err());
    }
    // A NULL/overflow-to-zero MAX does not make a high row fit an array that
    // has never had capacity for it. Never expose source out-of-bounds data.
    assert!(
        finish(
            Default::default(),
            SpellRecords {
                rand_prop_points: vec![rows::rand_prop_points(8)],
                rand_prop_points_sql_index_size: Some(0),
                ..Default::default()
            },
            Default::default()
        )
        .is_err()
    );
}

#[test]
fn empty_regular_store_reserves_zero_but_empty_is_not_an_assertable_row() {
    let c = finish(Default::default(), Default::default(), Default::default()).unwrap();
    assert_eq!(c.rand_prop_points_storage_last_index, 0);
    assert!(c.rand_prop_points_or_last(0).is_err());
    let c = finish(
        SpellRecords {
            rand_prop_points: vec![rows::rand_prop_points(0)],
            ..Default::default()
        },
        Default::default(),
        Default::default(),
    )
    .unwrap();
    assert_eq!(c.rand_prop_points_or_last(99).unwrap().id, 0);
}

#[test]
fn source_uint32_size_overflow_is_retained_as_raw_bits_but_not_admitted_to_calculation() {
    let c = finish(
        SpellRecords {
            rand_prop_points: vec![rows::rand_prop_points(u32::MAX)],
            ..Default::default()
        },
        Default::default(),
        Default::default(),
    )
    .unwrap();
    assert!(c.rand_prop_points(u32::MAX).is_some());
    assert!(c.rand_prop_points_or_last(u32::MAX).is_err());
    assert!(c.random_property_points(1, 3, 5, 0).is_err());
}

#[test]
fn unknown_random_property_input_cannot_claim_a_known_last_slot() {
    let mut rows = SpellRecords {
        rand_prop_points: vec![rows::rand_prop_points(7)],
        ..Default::default()
    };
    rows.unknown_baseline_records[46] = 1;
    let c = finish(rows, Default::default(), Default::default()).unwrap();
    assert!(c.rand_prop_points(7).is_some());
    assert!(c.rand_prop_points_or_last(7).is_err());
    assert!(c.random_property_points(7, 3, 5, 0).is_err());
}

#[test]
fn tuning_expansion_preserves_signed_bits_and_unknown_is_not_missing_minus_two() {
    let mut row = rows::content_tuning(7);
    row.expansion_id = i32::MIN;
    let c = finish(
        SpellRecords {
            content_tunings: vec![row],
            ..Default::default()
        },
        Default::default(),
        Default::default(),
    )
    .unwrap();
    assert_eq!(c.content_tuning_expansion(7).unwrap(), i32::MIN);
    assert_eq!(c.content_tuning_expansion(9).unwrap(), -2);
    assert_eq!(c.counts().len(), 49);
    let mut raw = SpellRecords::default();
    raw.unknown_baseline_records[44] = 1;
    let c = finish(raw, Default::default(), Default::default()).unwrap();
    assert!(c.content_tuning_expansion(9).is_err());
}
