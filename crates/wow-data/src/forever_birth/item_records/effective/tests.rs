use super::*;

fn item(id: u32, class: i32) -> ItemRecord {
    ItemRecord {
        id,
        class,
        subclass: 5,
        material: 0,
        inventory_type: 0,
        sheathe: 0,
        pet_food: 0,
        sound_override: 0,
        icon_file: 0,
        group_sounds: 0,
        content_tuning: 0,
        modified_crafting_reagent: 0,
        unknown_1200: 0,
        crafting_quality: 0,
        squish_era: 0,
        recraft_reagent_percentage: 0.0,
        order_source: 0,
    }
}
fn removals(rows: impl IntoIterator<Item = (u32, i32, u8)>) -> Db2HotfixRemovalStoreLikeCpp {
    Db2HotfixRemovalStoreLikeCpp::from_status_rows_like_cpp(rows)
}

#[test]
fn official_custom_repeated_rows_and_final_removals_are_sequential_not_sorted() {
    let baseline = ItemRecords {
        items: vec![item(1, 1), item(2, 2)],
        unknown_baseline_records: [59, 69, 40, 40],
        ..Default::default()
    };
    let official = ItemRecords {
        items: vec![item(1, 3), item(1, 4), item(3, 9)],
        ..Default::default()
    };
    let custom = ItemRecords {
        items: vec![item(1, 5), item(1, 6), item(2, 10)],
        ..Default::default()
    };
    let final_rows = baseline
        .finish(official, custom, &removals([(ITEM_HASH, 2, 2)]))
        .unwrap();
    assert_eq!(final_rows.item(1).unwrap().class, 6);
    assert!(final_rows.item(2).is_none());
    assert_eq!(final_rows.item(3).unwrap().class, 9);
    assert_eq!(final_rows.unknown_baseline_records(), [59, 69, 40, 40]);
    // Item-only IDs must not become template/quantity existence.
    assert!(!final_rows.quantity_projection().unwrap().contains(1));
}

#[test]
fn duplicate_baselines_fail_but_cross_table_ids_and_signed_removal_bits_do_not_alias() {
    let duplicate = ItemRecords {
        items: vec![item(1, 1), item(1, 2)],
        ..Default::default()
    };
    assert!(
        duplicate
            .finish(Default::default(), Default::default(), &removals([]))
            .is_err()
    );
    let baseline = ItemRecords {
        items: vec![item(u32::MAX, 1)],
        relations: vec![ItemEffectRelationRecord {
            id: u32::MAX,
            effect: -1,
            item: u32::MAX,
        }],
        ..Default::default()
    };
    let result = baseline
        .finish(
            Default::default(),
            Default::default(),
            &removals([(ITEM_HASH, -1, 2)]),
        )
        .unwrap();
    assert!(result.item(u32::MAX).is_none());
    assert_eq!(result.relations().count(), 1);
    assert_eq!(result.relations().next().unwrap().effect, -1);
}

#[test]
fn removals_apply_to_all_four_families_after_custom_overlay() {
    // The same generic operation composes the full sparse numeric rows too;
    // scalar IDs test its final-mask ordering without embedding private rows.
    for hash in [
        ITEM_HASH,
        ITEM_SPARSE_HASH,
        ITEM_EFFECT_HASH,
        ITEM_RELATION_HASH,
    ] {
        let rows = compose(
            vec![1u32],
            vec![1, 2],
            vec![2, 3],
            hash,
            &removals([(hash, 2, 2)]),
            |r| *r,
        )
        .unwrap();
        assert_eq!(rows.keys().copied().collect::<Vec<_>>(), vec![1, 3]);
    }
}
