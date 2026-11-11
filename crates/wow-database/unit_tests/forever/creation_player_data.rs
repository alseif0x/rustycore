use super::*;
use crate::SqlParam;
use std::collections::BTreeMap;

fn empty() -> NewPlayerDataSave {
    NewPlayerDataSave {
        elements: BTreeMap::new(),
        flags: BTreeMap::new(),
    }
}

#[test]
fn empty_dirty_maps_emit_nothing() {
    let data = empty();
    assert_eq!(statements(17, &data).count(), 0);
    let mut transaction = SqlTransaction::new();
    append(&mut transaction, 17, &data);
    assert!(transaction.sqls_for_test().is_empty());
}

#[test]
fn missing_zero_and_nonzero_projection_matrix_always_deletes_each_dirty_element() {
    // None freezes missing DB2/unsupported type; no lookup is executed here.
    for value in [
        None,
        Some(NewPlayerDataValue::Int64(0)),
        Some(NewPlayerDataValue::Float(0.0)),
        Some(NewPlayerDataValue::Float(-0.0)),
        Some(NewPlayerDataValue::Int64(-7)),
        Some(NewPlayerDataValue::Float(1.25)),
    ] {
        let inserts = matches!(&value, Some(NewPlayerDataValue::Int64(v)) if *v != 0)
            || matches!(&value, Some(NewPlayerDataValue::Float(v)) if *v != 0.0);
        let data = NewPlayerDataSave {
            elements: BTreeMap::from([(90, value)]),
            flags: BTreeMap::new(),
        };
        let rows: Vec<_> = statements(29, &data).collect();
        assert_eq!(rows.len(), if inserts { 2 } else { 1 });
        assert_eq!(
            rows[0].sql(),
            "DELETE FROM character_player_data_element WHERE characterGuid = ? AND playerDataElementCharacterId = ?"
        );
        assert_eq!(rows[0].params(), &[SqlParam::U64(29), SqlParam::U32(90)]);
        if inserts {
            assert_eq!(rows[1].sql(), ELEMENT_INSERT);
        }
    }
}

#[test]
fn int64_encoder_bounds_bind_signed_value_and_float_null_with_dirty_id() {
    // Encoder ranges only, not DB2 membership/storage resolution or admission proof.
    for value in [i64::MIN, -1, 1, i64::MAX] {
        let row = element_insert(
            0x12_0000_0009,
            u32::MAX,
            Some(&NewPlayerDataValue::Int64(value)),
        )
        .unwrap();
        assert_eq!(
            row.sql(),
            "INSERT INTO character_player_data_element (characterGuid, playerDataElementCharacterId, floatValue, int64Value) VALUES (?, ?, ?, ?)"
        );
        assert_eq!(row.sql().matches('?').count(), 4);
        assert_eq!(
            row.params(),
            &[
                SqlParam::U64(0x12_0000_0009),
                SqlParam::U32(u32::MAX),
                SqlParam::Null,
                SqlParam::I64(value)
            ]
        );
    }
}

#[test]
fn float_encoder_preserves_raw_bits_and_int64_null_without_sanitizing() {
    // Raw binding only; not a claim that a database accepts nonfinite floats.
    for value in [
        1.25f32,
        -2.5,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::from_bits(0x7fc0_1234),
    ] {
        let row = element_insert(17, 90, Some(&NewPlayerDataValue::Float(value))).unwrap();
        assert_eq!(row.params().len(), 4);
        assert_eq!(&row.params()[..2], &[SqlParam::U64(17), SqlParam::U32(90)]);
        match row.params()[2] {
            SqlParam::F32(actual) => assert_eq!(actual.to_bits(), value.to_bits()),
            _ => panic!("expected raw F32"),
        }
        assert_eq!(row.params()[3], SqlParam::Null);
    }
    for value in [0.0, -0.0] {
        assert!(element_insert(17, 90, Some(&NewPlayerDataValue::Float(value))).is_none());
    }
}

#[test]
fn ascending_dirty_element_ids_precede_all_ascending_flag_storage_indices() {
    let data = NewPlayerDataSave {
        elements: BTreeMap::from([
            (90, Some(NewPlayerDataValue::Float(1.25))),
            (2, None),
            (17, Some(NewPlayerDataValue::Int64(-7))),
        ]),
        flags: BTreeMap::from([(90, 3), (1, 0), (7, 5)]),
    };
    let rows: Vec<_> = statements(31, &data).collect();
    assert_eq!(
        rows.iter().map(|row| row.sql()).collect::<Vec<_>>(),
        [
            ELEMENT_DELETE,
            ELEMENT_DELETE,
            ELEMENT_INSERT,
            ELEMENT_DELETE,
            ELEMENT_INSERT,
            FLAG_DELETE,
            FLAG_DELETE,
            FLAG_INSERT,
            FLAG_DELETE,
            FLAG_INSERT,
        ]
    );
    for (row, key) in rows.iter().zip([2, 17, 17, 90, 90, 1, 7, 7, 90, 90]) {
        assert_eq!(&row.params()[..2], &[SqlParam::U64(31), SqlParam::U32(key)]);
    }
    // Frozen snapshot preservation is not canonical dirty-set clearing.
    assert_eq!(data.elements.len(), 3);
    assert_eq!(data.flags.len(), 3);
}

#[test]
fn flag_zero_deletes_only_and_nonzero_inserts_unsigned_mask_with_exact_scope() {
    for mask in [0, 1, u64::MAX] {
        let data = NewPlayerDataSave {
            elements: BTreeMap::new(),
            flags: BTreeMap::from([(u32::MAX, mask)]),
        };
        let rows: Vec<_> = statements(17, &data).collect();
        assert_eq!(
            rows[0].sql(),
            "DELETE FROM character_player_data_flag WHERE characterGuid = ? AND storageIndex = ?"
        );
        assert_eq!(
            rows[0].params(),
            &[SqlParam::U64(17), SqlParam::U32(u32::MAX)]
        );
        assert_eq!(rows.len(), if mask == 0 { 1 } else { 2 });
        if mask != 0 {
            assert_eq!(
                rows[1].sql(),
                "INSERT INTO character_player_data_flag (characterGuid, storageIndex, mask) VALUES (?, ?, ?)"
            );
            assert_eq!(rows[1].sql().matches('?').count(), 3);
            assert_eq!(
                rows[1].params(),
                &[
                    SqlParam::U64(17),
                    SqlParam::U32(u32::MAX),
                    SqlParam::U64(mask)
                ]
            );
        }
    }
}

#[test]
fn character_save_places_player_data_elements_then_flags_immediately_after_five_cuf_slots() {
    let mut character = super::super::tests::character();
    character.guid = 0x12_0000_0009;
    character.player_data = NewPlayerDataSave {
        elements: BTreeMap::from([(90, Some(NewPlayerDataValue::Int64(-7)))]),
        flags: BTreeMap::from([(2, 5)]),
    };
    let mut transaction = SqlTransaction::new();
    super::super::append_character_save(&mut transaction, character.guid, &character);
    let sqls = transaction.sqls_for_test();
    let cuf = sqls
        .iter()
        .position(|sql| *sql == "DELETE FROM character_cuf_profiles WHERE guid = ? AND id = ?")
        .unwrap();
    assert_eq!(
        &sqls[cuf..],
        &[
            "DELETE FROM character_cuf_profiles WHERE guid = ? AND id = ?",
            "DELETE FROM character_cuf_profiles WHERE guid = ? AND id = ?",
            "DELETE FROM character_cuf_profiles WHERE guid = ? AND id = ?",
            "DELETE FROM character_cuf_profiles WHERE guid = ? AND id = ?",
            "DELETE FROM character_cuf_profiles WHERE guid = ? AND id = ?",
            ELEMENT_DELETE,
            ELEMENT_INSERT,
            FLAG_DELETE,
            FLAG_INSERT,
            "DELETE FROM character_bank_tab_settings WHERE characterGuid = ?",
        ]
    );
}
