use super::*;
use crate::SqlParam;
use wow_persistence::forever::creation::save::{NewInstanceRestriction, NewTutorialSave};

fn currency(id: u32, state: NewCurrencySaveState) -> NewCurrencySave {
    NewCurrencySave {
        currency_id: id,
        has_currency_entry: true,
        state,
        quantity: 11,
        weekly_quantity: 12,
        tracked_quantity: 13,
        increased_cap_quantity: 14,
        earned_quantity: 15,
        flags: 3,
    }
}

#[test]
fn state_and_frozen_lookup_matrix_skips_missing_unchanged_and_removed_without_deletes() {
    // Frozen outcome consumption only, not CurrencyTypesStore lookup proof.
    for exists in [false, true] {
        for state in [
            NewCurrencySaveState::Unchanged,
            NewCurrencySaveState::Changed,
            NewCurrencySaveState::New,
            NewCurrencySaveState::Removed,
        ] {
            let mut data = currency(17, state);
            data.has_currency_entry = exists;
            let row = currency_statement(29, &data);
            let expected = if !exists {
                None
            } else {
                match state {
                    NewCurrencySaveState::New => Some(REPLACE),
                    NewCurrencySaveState::Changed => Some(UPDATE),
                    NewCurrencySaveState::Unchanged | NewCurrencySaveState::Removed => None,
                }
            };
            assert_eq!(row.as_ref().map(|row| row.sql()), expected);
        }
    }
}

#[test]
fn new_and_changed_currency_bind_exact_source_sql_and_all_eight_fields() {
    let guid = 0x12_0000_0009;
    let row = currency_statement(guid, &currency(90, NewCurrencySaveState::New)).unwrap();
    assert_eq!(
        row.sql(),
        "REPLACE INTO character_currency (CharacterGuid, Currency, Quantity, WeeklyQuantity, TrackedQuantity, IncreasedCapQuantity, EarnedQuantity, Flags) VALUES (?, ?, ?, ?, ?, ?, ?, ?)"
    );
    assert_eq!(row.sql().matches('?').count(), 8);
    assert_eq!(
        row.params(),
        &[
            SqlParam::U64(guid),
            SqlParam::U16(90),
            SqlParam::U32(11),
            SqlParam::U32(12),
            SqlParam::U32(13),
            SqlParam::U32(14),
            SqlParam::U32(15),
            SqlParam::U8(3),
        ]
    );
    let row = currency_statement(guid, &currency(90, NewCurrencySaveState::Changed)).unwrap();
    assert_eq!(
        row.sql(),
        "UPDATE character_currency SET Quantity = ?, WeeklyQuantity = ?, TrackedQuantity = ?, IncreasedCapQuantity = ?, EarnedQuantity = ?, Flags = ? WHERE CharacterGuid = ? AND Currency = ?"
    );
    assert_eq!(row.sql().matches('?').count(), 8);
    assert_eq!(
        row.params(),
        &[
            SqlParam::U32(11),
            SqlParam::U32(12),
            SqlParam::U32(13),
            SqlParam::U32(14),
            SqlParam::U32(15),
            SqlParam::U8(3),
            SqlParam::U64(guid),
            SqlParam::U16(90),
        ]
    );
}

#[test]
fn zero_quantities_still_write_for_valid_new_and_changed_entries() {
    for state in [NewCurrencySaveState::New, NewCurrencySaveState::Changed] {
        let mut data = currency(17, state);
        data.quantity = 0;
        data.weekly_quantity = 0;
        data.tracked_quantity = 0;
        data.increased_cap_quantity = 0;
        data.earned_quantity = 0;
        data.flags = 0;
        let row = currency_statement(29, &data).unwrap();
        let base = if matches!(state, NewCurrencySaveState::New) {
            2
        } else {
            0
        };
        assert!(
            row.params()[base..base + 5]
                .iter()
                .all(|value| *value == SqlParam::U32(0))
        );
        assert_eq!(row.params()[base + 5], SqlParam::U8(0));
    }
}

#[test]
fn binding_limits_preserve_u32_quantities_u8_flags_and_source_u16_key_narrowing() {
    // Encoder ranges only, not store membership/canonical or DB admission proof.
    for state in [NewCurrencySaveState::New, NewCurrencySaveState::Changed] {
        for (id, narrowed) in [(0x1234_5678, 0x5678), (u32::MAX, u16::MAX)] {
            let data = NewCurrencySave {
                currency_id: id,
                has_currency_entry: true,
                state,
                quantity: u32::MAX,
                weekly_quantity: u32::MAX,
                tracked_quantity: u32::MAX,
                increased_cap_quantity: u32::MAX,
                earned_quantity: u32::MAX,
                flags: u8::MAX,
            };
            let row = currency_statement(u64::MAX, &data).unwrap();
            let (base, owner) = if matches!(state, NewCurrencySaveState::New) {
                (2, 0)
            } else {
                (0, 6)
            };
            assert_eq!(
                &row.params()[owner..owner + 2],
                &[SqlParam::U64(u64::MAX), SqlParam::U16(narrowed)]
            );
            assert!(
                row.params()[base..base + 5]
                    .iter()
                    .all(|value| *value == SqlParam::U32(u32::MAX))
            );
            assert_eq!(row.params()[base + 5], SqlParam::U8(u8::MAX));
        }
    }
}

#[test]
fn unique_source_ids_keep_supplied_order_and_frozen_currency_states() {
    // IDs remain unique both before and after the source u16 narrowing.
    let data = vec![
        currency(90, NewCurrencySaveState::New),
        currency(2, NewCurrencySaveState::Changed),
        currency(17, NewCurrencySaveState::Removed),
        currency(7, NewCurrencySaveState::New),
    ];
    let rows: Vec<_> = statements(31, &data).collect();
    assert_eq!(
        rows.iter().map(|row| row.sql()).collect::<Vec<_>>(),
        [REPLACE, UPDATE, REPLACE]
    );
    assert_eq!(
        &rows[0].params()[..2],
        &[SqlParam::U64(31), SqlParam::U16(90)]
    );
    assert_eq!(
        &rows[1].params()[6..],
        &[SqlParam::U64(31), SqlParam::U16(2)]
    );
    assert_eq!(
        &rows[2].params()[..2],
        &[SqlParam::U64(31), SqlParam::U16(7)]
    );
    let mut transaction = crate::forever::create::tests::isolated_transaction();
    append(&mut transaction, 31, &data);
    assert_eq!(
        transaction.sqls_for_test(),
        rows.iter().map(|row| row.sql()).collect::<Vec<_>>()
    );
    // Snapshot preservation is not canonical state clearing/acknowledgement.
    assert!(matches!(data[0].state, NewCurrencySaveState::New));
    assert!(matches!(data[1].state, NewCurrencySaveState::Changed));
    assert!(matches!(data[2].state, NewCurrencySaveState::Removed));
    assert!(data.iter().all(|currency| currency.has_currency_entry));
}

#[test]
fn character_save_places_currency_after_cache_and_account_session_saves() {
    let mut character = super::super::tests::character();
    character.guid = 0x12_0000_0009;
    character.session_save.tutorials = NewTutorialSave::Changed {
        loaded_from_db: false,
        values: [1; 8],
    };
    character.session_save.instance_restrictions = vec![NewInstanceRestriction {
        instance_id: 90,
        release_time: 0,
    }];
    character.currencies = vec![currency(17, NewCurrencySaveState::New)];
    let mut transaction = crate::forever::create::tests::isolated_transaction();
    super::super::append_character_save(&mut transaction, character.guid, &character);
    let sqls = transaction.sqls_for_test();
    let cache = super::super::visual_statement(character.guid, &character.visual_items);
    let cache_index = sqls.iter().position(|sql| *sql == cache.sql()).unwrap();
    assert_eq!(
        &sqls[cache_index + 1..],
        &[
            "INSERT INTO account_tutorial(tut0, tut1, tut2, tut3, tut4, tut5, tut6, tut7, accountId) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
            "DELETE FROM account_instance_times WHERE accountId = ?",
            "INSERT INTO account_instance_times (accountId, instanceId, releaseTime) VALUES (?, ?, ?)",
            "REPLACE INTO character_currency (CharacterGuid, Currency, Quantity, WeeklyQuantity, TrackedQuantity, IncreasedCapQuantity, EarnedQuantity, Flags) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
            "DELETE FROM character_cuf_profiles WHERE guid = ? AND id = ?",
            "DELETE FROM character_cuf_profiles WHERE guid = ? AND id = ?",
            "DELETE FROM character_cuf_profiles WHERE guid = ? AND id = ?",
            "DELETE FROM character_cuf_profiles WHERE guid = ? AND id = ?",
            "DELETE FROM character_cuf_profiles WHERE guid = ? AND id = ?",
            "DELETE FROM character_bank_tab_settings WHERE characterGuid = ?",
        ]
    );
    assert_eq!(character.account, 7);
    let row = currency_statement(character.guid, &character.currencies[0]).unwrap();
    assert_eq!(row.params()[0], SqlParam::U64(character.guid));
    assert_eq!(row.params()[1], SqlParam::U16(17));
}
