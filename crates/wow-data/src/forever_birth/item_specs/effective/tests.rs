use super::*;
fn removals(rows: impl IntoIterator<Item = (u32, i32, u8)>) -> Db2HotfixRemovalStoreLikeCpp {
    Db2HotfixRemovalStoreLikeCpp::from_status_rows_like_cpp(rows)
}
fn override_row(id: u32, item: u32, specialization: u16) -> ItemSpecOverrideRecord {
    ItemSpecOverrideRecord {
        id,
        item,
        specialization,
    }
}
#[test]
fn final_overrides_reindex_after_ordered_repeated_sql_rows_and_removal() {
    let baseline = ItemSpecRecords {
        overrides: vec![override_row(1, 7, 10), override_row(2, 7, 11)],
        ..Default::default()
    };
    let official = ItemSpecRecords {
        overrides: vec![override_row(1, 8, 12), override_row(1, 9, 13)],
        ..Default::default()
    };
    let custom = ItemSpecRecords {
        overrides: vec![override_row(1, 10, 14), override_row(3, 10, 15)],
        ..Default::default()
    };
    let catalog = baseline
        .finish(
            official,
            custom,
            &removals([(ITEM_SPEC_OVERRIDE_HASH, 2, 2)]),
        )
        .unwrap();
    for id in [7, 8, 9] {
        assert!(catalog.overrides(id).is_none());
    }
    assert_eq!(
        catalog
            .overrides(10)
            .unwrap()
            .map(|r| r.specialization)
            .collect::<Vec<_>>(),
        vec![14, 15]
    );
}
#[test]
fn unresolved_override_presence_is_retained_and_signed_gem_bits_survive() {
    let catalog = ItemSpecRecords {
        overrides: vec![override_row(1, 7, 65535)],
        gems: vec![GemPropertiesRecord {
            id: 1,
            enchantment: u16::MAX,
            kind: i32::MIN,
        }],
        ..Default::default()
    }
    .finish(Default::default(), Default::default(), &removals([]))
    .unwrap();
    assert_eq!(
        catalog.overrides(7).unwrap().next().unwrap().specialization,
        u16::MAX
    );
    assert_eq!(catalog.gem(1).unwrap().kind, i32::MIN);
    assert_eq!(catalog.gem(1).unwrap().enchantment, u16::MAX);
}
#[test]
fn duplicate_baselines_fail_and_final_spec_and_gem_removals_do_not_alias() {
    let gem = GemPropertiesRecord {
        id: u32::MAX,
        enchantment: 1,
        kind: -1,
    };
    let duplicate = ItemSpecRecords {
        gems: vec![gem, gem],
        ..Default::default()
    };
    assert!(
        duplicate
            .finish(Default::default(), Default::default(), &removals([]))
            .is_err()
    );
    let spec = ItemSpecRecord {
        id: u32::MAX,
        min_level: 255,
        max_level: 255,
        item_type: 7,
        primary: 40,
        secondary: 40,
        specialization: 65535,
    };
    let catalog = ItemSpecRecords {
        gems: vec![gem],
        specs: vec![spec],
        ..Default::default()
    }
    .finish(
        Default::default(),
        Default::default(),
        &removals([(GEM_PROPERTIES_HASH, -1, 2)]),
    )
    .unwrap();
    assert!(catalog.gem(u32::MAX).is_none());
    assert_eq!(catalog.specs().count(), 1);
}
