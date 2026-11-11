use super::*;
use crate::SqlParam;

#[test]
fn tutorials_unchanged_new_and_loaded_select_source_sql_with_eight_u32_values_then_account() {
    assert!(tutorial_statement(17, &NewTutorialSave::Unchanged).is_none());
    let values = [u32::MAX, 0x8000_0000, 0, 1, 2, 3, 4, 5];
    for loaded_from_db in [false, true] {
        let data = NewTutorialSave::Changed {
            loaded_from_db,
            values,
        };
        let row = tutorial_statement(u32::MAX, &data).unwrap();
        assert_eq!(
            row.sql(),
            if loaded_from_db {
                "UPDATE account_tutorial SET tut0 = ?, tut1 = ?, tut2 = ?, tut3 = ?, tut4 = ?, tut5 = ?, tut6 = ?, tut7 = ? WHERE accountId = ?"
            } else {
                "INSERT INTO account_tutorial(tut0, tut1, tut2, tut3, tut4, tut5, tut6, tut7, accountId) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"
            }
        );
        assert_eq!(row.sql().matches('?').count(), 9);
        assert_eq!(
            row.params(),
            &[
                SqlParam::U32(u32::MAX),
                SqlParam::U32(0x8000_0000),
                SqlParam::U32(0),
                SqlParam::U32(1),
                SqlParam::U32(2),
                SqlParam::U32(3),
                SqlParam::U32(4),
                SqlParam::U32(5),
                SqlParam::U32(u32::MAX),
            ]
        );
    }
}

#[test]
fn empty_restrictions_do_nothing_and_nonempty_preserves_unique_supplied_ids_and_past_values() {
    assert!(restriction_statements(17, &[]).is_empty());
    let data = [
        NewInstanceRestriction {
            instance_id: 90,
            release_time: 0,
        },
        NewInstanceRestriction {
            instance_id: 2,
            release_time: 17,
        },
        NewInstanceRestriction {
            instance_id: 19,
            release_time: 11,
        },
    ];
    let rows = restriction_statements(23, &data);
    assert_eq!(
        rows.iter().map(|row| row.sql()).collect::<Vec<_>>(),
        [
            "DELETE FROM account_instance_times WHERE accountId = ?",
            "INSERT INTO account_instance_times (accountId, instanceId, releaseTime) VALUES (?, ?, ?)",
            "INSERT INTO account_instance_times (accountId, instanceId, releaseTime) VALUES (?, ?, ?)",
            "INSERT INTO account_instance_times (accountId, instanceId, releaseTime) VALUES (?, ?, ?)",
        ]
    );
    assert_eq!(rows[0].params(), &[SqlParam::U32(23)]);
    for (row, (instance, release)) in rows[1..].iter().zip([(90, 0), (2, 17), (19, 11)]) {
        assert_eq!(
            row.params(),
            &[
                SqlParam::U32(23),
                SqlParam::U32(instance),
                SqlParam::U64(release)
            ]
        );
    }
}

#[test]
fn restriction_binding_limits_preserve_account_and_instance_u32_and_projected_u64_release() {
    // Setter bounds only; canonical clock projection/loading is not executed.
    let rows = restriction_statements(
        u32::MAX,
        &[NewInstanceRestriction {
            instance_id: u32::MAX,
            release_time: u64::MAX,
        }],
    );
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].params(), &[SqlParam::U32(u32::MAX)]);
    assert_eq!(rows[1].sql().matches('?').count(), 3);
    assert_eq!(
        rows[1].params(),
        &[
            SqlParam::U32(u32::MAX),
            SqlParam::U32(u32::MAX),
            SqlParam::U64(u64::MAX),
        ]
    );
}

#[test]
fn tutorial_and_restriction_input_matrix_keeps_source_order_and_frozen_tutorial_flags() {
    for tutorial_state in 0..3 {
        for with_restrictions in [false, true] {
            let data = NewCharacterSessionSave {
                tutorials: if tutorial_state == 0 {
                    NewTutorialSave::Unchanged
                } else {
                    NewTutorialSave::Changed {
                        loaded_from_db: tutorial_state == 2,
                        values: [0; 8],
                    }
                },
                instance_restrictions: if with_restrictions {
                    vec![NewInstanceRestriction {
                        instance_id: 7,
                        release_time: 0,
                    }]
                } else {
                    Vec::new()
                },
            };
            let rows = statements(29, &data);
            let mut expected = Vec::new();
            if tutorial_state != 0 {
                expected.push(if tutorial_state == 2 {
                    TUTORIAL_UPDATE
                } else {
                    TUTORIAL_INSERT
                });
            }
            if with_restrictions {
                expected.extend([RESTRICTIONS_DELETE, RESTRICTION_INSERT]);
            }
            assert_eq!(
                rows.iter().map(|row| row.sql()).collect::<Vec<_>>(),
                expected
            );
            let mut transaction = crate::forever::create::tests::isolated_transaction();
            append(&mut transaction, 29, &data);
            assert_eq!(transaction.sqls_for_test(), expected);
            // Snapshot preservation is not canonical loaded/changed-flag acknowledgement.
            match &data.tutorials {
                NewTutorialSave::Unchanged => assert_eq!(tutorial_state, 0),
                NewTutorialSave::Changed { loaded_from_db, .. } => {
                    assert_eq!(*loaded_from_db, tutorial_state == 2)
                }
            }
        }
    }
}

#[test]
fn character_save_places_characterdb_account_session_writes_after_cache() {
    let mut character = super::super::tests::character();
    character.guid = 0x12_0000_0009;
    assert_eq!(character.account, 7);
    character.session_save = NewCharacterSessionSave {
        tutorials: NewTutorialSave::Changed {
            loaded_from_db: true,
            values: [10, 11, 12, 13, 14, 15, 16, 17],
        },
        instance_restrictions: vec![
            NewInstanceRestriction {
                instance_id: 90,
                release_time: 0,
            },
            NewInstanceRestriction {
                instance_id: 2,
                release_time: 17,
            },
        ],
    };
    let mut transaction = crate::forever::create::tests::isolated_transaction();
    super::super::append_character_save(&mut transaction, character.guid, &character);
    let sqls = transaction.sqls_for_test();
    let cache = super::super::visual_statement(character.guid, &character.visual_items);
    assert_eq!(cache.params()[0], SqlParam::U64(character.guid));
    let cache_index = sqls.iter().position(|sql| *sql == cache.sql()).unwrap();
    assert_eq!(
        sqls[cache_index - 1],
        "DELETE FROM character_select_screen_equipment_cache WHERE guid = ?"
    );
    assert_eq!(
        &sqls[cache_index + 1..],
        &[
            "UPDATE account_tutorial SET tut0 = ?, tut1 = ?, tut2 = ?, tut3 = ?, tut4 = ?, tut5 = ?, tut6 = ?, tut7 = ? WHERE accountId = ?",
            "DELETE FROM account_instance_times WHERE accountId = ?",
            "INSERT INTO account_instance_times (accountId, instanceId, releaseTime) VALUES (?, ?, ?)",
            "INSERT INTO account_instance_times (accountId, instanceId, releaseTime) VALUES (?, ?, ?)",
            "DELETE FROM character_cuf_profiles WHERE guid = ? AND id = ?",
            "DELETE FROM character_cuf_profiles WHERE guid = ? AND id = ?",
            "DELETE FROM character_cuf_profiles WHERE guid = ? AND id = ?",
            "DELETE FROM character_cuf_profiles WHERE guid = ? AND id = ?",
            "DELETE FROM character_cuf_profiles WHERE guid = ? AND id = ?",
            "DELETE FROM character_bank_tab_settings WHERE characterGuid = ?",
        ]
    );
    let rows = statements(character.account, &character.session_save);
    assert_eq!(rows[0].params()[8], SqlParam::U32(7));
    for row in &rows[1..] {
        assert_eq!(row.params()[0], SqlParam::U32(7));
    }
}
