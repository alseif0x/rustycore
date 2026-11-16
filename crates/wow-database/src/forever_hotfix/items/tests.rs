use super::*;

#[test]
fn full_queries_keep_source_order_strings_arrays_five_flags_and_overlay_predicate() {
    let tables = [
        "item",
        "item_sparse",
        "item_effect",
        "item_x_item_effect",
        "item_bonus",
        "azerite_empowered_item",
        "item_scaling_config",
        "item_offset_curve",
        "item_squish_era",
    ];
    let columns_per_table = [17, 104, 10, 3, 8, 4, 6, 3, 4];
    for (i, query) in QUERIES.iter().enumerate() {
        assert!(query.ends_with(&format!(" FROM {} WHERE (VerifiedBuild>0)=?", tables[i])));
        assert_eq!(
            query.split(" FROM ").next().unwrap().split(',').count(),
            columns_per_table[i]
        );
        assert!(!query.contains("ORDER BY") && !query.contains("locale") && !query.contains('*'));
    }
    let columns = QUERIES[1]
        .strip_prefix("SELECT ")
        .unwrap()
        .split(" FROM ")
        .next()
        .unwrap()
        .split(',')
        .collect::<Vec<_>>();
    assert_eq!(
        &columns[..6],
        &[
            "ID",
            "Description",
            "Display3",
            "Display2",
            "Display1",
            "Display"
        ]
    );
    // Keep numeric projection's independent array checks, after five strings.
    let columns = [&columns[..1], &columns[6..]].concat();
    for (start, prefix, count) in [
        (10, "StatPercentageOfSocket", 10),
        (20, "StatPercentEditor", 10),
        (30, "StatModifierBonusStat", 10),
        (44, "AllowableRace", 2),
        (51, "Flags", 5),
        (70, "ZoneBound", 2),
        (84, "SocketType", 3),
    ] {
        for i in 0..count {
            assert_eq!(columns[start + i], format!("{prefix}{}", i + 1));
        }
    }
    assert_eq!(
        &columns[40..44],
        &["Stackable", "MaxCount", "MinReputation", "RequiredAbility"]
    );
    assert_eq!(
        &columns[93..99],
        &[
            "RequiredPVPMedal",
            "RequiredPVPRank",
            "RequiredLevel",
            "InventoryType",
            "OverallQualityID",
            "AmmunitionType"
        ]
    );
    assert!(!columns.contains(&"VerifiedBuild"));
    assert_eq!(
        QUERIES[4],
        "SELECT ID,Value1,Value2,Value3,Value4,ParentItemBonusListID,Type,OrderIndex FROM item_bonus WHERE (VerifiedBuild>0)=?"
    );
    assert_eq!(
        QUERIES[5],
        "SELECT ID,ItemID,AzeriteTierUnlockSetID,AzeritePowerSetID FROM azerite_empowered_item WHERE (VerifiedBuild>0)=?"
    );
    assert_eq!(
        QUERIES[6],
        "SELECT ID,ItemOffsetCurveID,ItemLevel,RequiredLevel,ItemSquishEraID,Flags FROM item_scaling_config WHERE (VerifiedBuild>0)=?"
    );
    assert_eq!(
        QUERIES[7],
        "SELECT ID,CurveID,`Offset` FROM item_offset_curve WHERE (VerifiedBuild>0)=?"
    );
    assert_eq!(
        QUERIES[8],
        "SELECT ID,Patch,CurveID,Flags FROM item_squish_era WHERE (VerifiedBuild>0)=?"
    );
}

#[test]
fn missing_numeric_rows_cannot_decode_or_finish_as_valid_sparse_records() {
    let empty = SqlResult::empty();
    assert!(NumericRow::new(&empty, 104).is_err());
    assert!(sparse::read(&empty).is_err());
    let mut cursor = NumericRow::new(&empty, 0).unwrap();
    assert!(cursor.read::<u32>().is_err());
    assert!(cursor.array::<i32, 5>().is_err());
    assert!(cursor.text().is_err());
    for columns in [8, 4, 6, 3, 4] {
        assert!(NumericRow::new(&empty, columns).is_err());
    }
}
