use super::super::ItemRecords;
use crate::Db2HotfixRemovalStoreLikeCpp;
use std::path::PathBuf;

#[test]
#[ignore = "requires the pinned private Forever item-input corpus"]
fn private_corpus_loads_all_item_bonus_rows_through_public_loader() {
    let directory = PathBuf::from(
        std::env::var_os("RUSTYCORE_FOREVER_ITEM_DATA_DIR")
            .expect("set RUSTYCORE_FOREVER_ITEM_DATA_DIR to the private item corpus"),
    );
    let records = ItemRecords::load_available(&directory).unwrap();

    assert_eq!(records.item_bonuses.len(), 1124);
    assert!(records.azerite_empowered_items.is_empty());
    assert!(records.item_scaling_configs.is_empty());
    assert!(records.item_offset_curves.is_empty());
    assert!(records.item_squish_eras.is_empty());
    assert_eq!(records.item_squish_era_file_last_index, Some(0));
    assert_eq!(records.item_squish_era_sql_index_size, None);
    assert_eq!(&records.unknown_baseline_records[4..], &[0; 5]);

    let selected = *records
        .item_bonuses
        .first()
        .expect("private corpus contains ItemBonus records");
    let official = ItemRecords::default();
    let custom = ItemRecords::default();
    let removals =
        Db2HotfixRemovalStoreLikeCpp::from_status_rows_like_cpp(
            std::iter::empty::<(u32, i32, u8)>(),
        );
    let catalog = records.finish(official, custom, &removals).unwrap();

    assert_eq!(&catalog.counts()[4..], &[1124, 0, 0, 0, 0]);
    let expected = selected;
    assert_eq!(catalog.item_bonus(selected.id), Some(&expected));
    assert_eq!(
        catalog
            .item_bonuses_for_list(u32::from(selected.parent))
            .find(|row| row.id == selected.id),
        Some(&expected)
    );
}
