use super::*;
use crate::SqlParam;

#[test]
fn condition_id_queries_match_the_two_hotfix_tables_and_official_custom_bind_order() {
    assert_eq!(
        PLAYER_CONDITION_IDS_SQL,
        "SELECT ID FROM player_condition WHERE (`VerifiedBuild` > 0) = ?"
    );
    assert_eq!(
        UNIT_CONDITION_IDS_SQL,
        "SELECT ID FROM unit_condition WHERE (`VerifiedBuild` > 0) = ?"
    );
    assert_eq!(
        QUERY_PLAN,
        [
            (ConditionTable::Player, true),
            (ConditionTable::Player, false),
            (ConditionTable::Unit, true),
            (ConditionTable::Unit, false),
        ]
    );

    for (table, official) in QUERY_PLAN {
        let statement = query_statement(table, official);
        assert_eq!(statement.sql(), query_sql(table));
        assert_eq!(statement.params(), &[SqlParam::Bool(official)]);
    }
}

#[test]
fn condition_id_decoding_preserves_u32_and_signed_i32_bits_and_rejects_invalid_values() {
    assert_eq!(
        condition_id_from_columns(vec![NativeFieldLikeCpp::UInt32(u32::MAX)], "ID").unwrap(),
        u32::MAX
    );
    assert_eq!(
        condition_id_from_columns(vec![NativeFieldLikeCpp::Int32(-1)], "ID").unwrap(),
        u32::MAX
    );
    assert_eq!(
        condition_id_from_columns(vec![NativeFieldLikeCpp::UInt64(u64::from(u32::MAX))], "ID")
            .unwrap(),
        u32::MAX
    );
    assert_eq!(
        condition_id_from_columns(vec![NativeFieldLikeCpp::Int64(i64::from(u32::MAX))], "ID")
            .unwrap(),
        u32::MAX
    );
    assert_eq!(
        condition_id_from_columns(vec![NativeFieldLikeCpp::Int8(-1)], "ID").unwrap(),
        u32::MAX
    );
    assert!(condition_id_from_columns(vec![], "ID").is_err());
    assert!(condition_id_from_columns(vec![NativeFieldLikeCpp::Null], "ID").is_err());
    assert!(condition_id_from_columns(vec![NativeFieldLikeCpp::Int64(-1)], "ID").is_err());
    assert!(
        condition_id_from_columns(
            vec![NativeFieldLikeCpp::UInt64(u64::from(u32::MAX) + 1)],
            "ID"
        )
        .is_err()
    );
    assert!(
        condition_id_from_columns(
            vec![NativeFieldLikeCpp::Int64(i64::from(u32::MAX) + 1)],
            "ID"
        )
        .is_err()
    );
    assert!(condition_id_from_columns(vec![NativeFieldLikeCpp::Int64(i64::MIN)], "ID").is_err());
    assert!(condition_id_from_columns(vec![NativeFieldLikeCpp::Float(1.0)], "ID").is_err());
    assert!(condition_id_from_columns(vec![NativeFieldLikeCpp::Double(1.0)], "ID").is_err());
    assert!(condition_id_from_columns(vec![NativeFieldLikeCpp::Text("1".into())], "ID").is_err());
    assert!(
        condition_id_from_columns(vec![NativeFieldLikeCpp::Unsupported("BINARY".into())], "ID")
            .is_err()
    );
    assert!(
        condition_id_from_columns(
            vec![NativeFieldLikeCpp::UInt32(1), NativeFieldLikeCpp::UInt32(2)],
            "ID"
        )
        .is_err()
    );
}
