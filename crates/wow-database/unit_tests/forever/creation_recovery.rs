//! Projection policy/bindings only; real restart/connection-loss proof is separate.
use super::*;
use crate::SqlParam;

fn entry() -> Entry {
    Entry {
        guid: 17,
        account: 19,
        realm: 0x0201ABCD,
        save_time: 500,
    }
}
fn character() -> CanonicalCharacter {
    CanonicalCharacter {
        guid: 17,
        account: 19,
        name: "Synthetic".into(),
        logout_time: 500,
        deleted: false,
    }
}
fn last(time: u32) -> LastPlayed {
    LastPlayed {
        realm: 0xABCD,
        name: "Synthetic".into(),
        guid: 17,
        time,
    }
}

#[test]
fn missing_projection_is_inserted_and_count_is_the_canonical_absolute_count() {
    assert_eq!(
        plan(entry(), &character(), 3, None),
        Ok(RecoveryPlan {
            count: 3,
            last_played: LastPlayedWrite::Insert
        })
    );
}

#[test]
fn already_projected_receipt_is_idempotent_but_still_refreshes_the_absolute_count() {
    assert_eq!(
        plan(entry(), &character(), 7, Some(&last(500))),
        Ok(RecoveryPlan {
            count: 7,
            last_played: LastPlayedWrite::Keep
        })
    );
    let batch = projection(
        entry(),
        character(),
        RecoveryPlan {
            count: 7,
            last_played: LastPlayedWrite::Keep,
        },
    );
    assert_eq!(
        batch.sqls_for_test(),
        ["REPLACE INTO realmcharacters (numchars, acctid, realmid) VALUES (?,?,?)"]
    );
    assert_eq!(
        writer::realm_count(entry(), 7).params(),
        &[SqlParam::U32(7), SqlParam::U32(19), SqlParam::U32(0xABCD)]
    );
}

#[test]
fn newer_last_played_state_is_preserved_even_when_it_belongs_to_another_realm() {
    let mut last = last(501);
    last.guid = 99;
    last.realm = 7;
    last.name = "Later".into();
    assert_eq!(
        plan(entry(), &character(), 3, Some(&last)),
        Ok(RecoveryPlan {
            count: 3,
            last_played: LastPlayedWrite::Keep
        })
    );
}

#[test]
fn older_projection_has_one_coherent_replace_and_count_batch() {
    let policy = plan(entry(), &character(), 3, Some(&last(499))).unwrap();
    assert_eq!(policy.last_played, LastPlayedWrite::Replace);
    let batch = projection(entry(), character(), policy);
    assert_eq!(
        batch.sqls_for_test(),
        [
            "DELETE FROM account_last_played_character WHERE accountId=? AND region=? AND battlegroup=?",
            "INSERT INTO account_last_played_character (accountId, region, battlegroup, realmId, characterName, characterGUID, lastPlayedTime) VALUES (?,?,?,?,?,?,?)",
            "REPLACE INTO realmcharacters (numchars, acctid, realmid) VALUES (?,?,?)",
        ]
    );
    assert_eq!(batch.expected_rows_for_test(), [Some(1), Some(1), None]);
}

#[test]
fn equal_time_different_identity_or_name_is_not_arbitrarily_overwritten() {
    for field in 0..3 {
        let mut last = last(500);
        match field {
            0 => last.guid = 18,
            1 => last.realm = 7,
            _ => last.name = "Other".into(),
        }
        assert_eq!(
            plan(entry(), &character(), 1, Some(&last)),
            Err(CreateError::RecoveryRequired)
        );
    }
}

#[test]
fn canonical_owner_time_deletion_and_name_must_still_prove_the_marker() {
    for field in 0..6 {
        let mut row = character();
        match field {
            0 => row.guid = 18,
            1 => row.account = 20,
            2 => row.logout_time = 499,
            3 => row.logout_time = -1,
            4 => row.deleted = true,
            _ => row.name.clear(),
        }
        assert_eq!(
            plan(entry(), &row, 1, None),
            Err(CreateError::RecoveryRequired)
        );
    }
}

#[test]
fn a_missing_canonical_count_or_a_count_too_large_for_auth_stops_recovery() {
    assert_eq!(
        plan(entry(), &character(), 0, None),
        Err(CreateError::RecoveryRequired)
    );
    for count in [256, u64::MAX] {
        assert_eq!(
            plan(entry(), &character(), count, None),
            Err(CreateError::InvalidRow)
        );
    }
    assert_eq!(plan(entry(), &character(), 255, None).unwrap().count, 255);
}

#[test]
fn invalid_existing_projection_is_not_treated_as_absence_or_as_newer_state() {
    for field in 0..3 {
        let mut last = last(501);
        match field {
            0 => last.realm = 0,
            1 => last.guid = 0,
            _ => last.name.clear(),
        }
        assert_eq!(
            plan(entry(), &character(), 1, Some(&last)),
            Err(CreateError::RecoveryRequired)
        );
    }
}

#[test]
fn auth_bindings_preserve_full_region_site_and_realm_without_persisting_the_full_address_as_realm_id()
 {
    assert_eq!(
        writer::delete_last_played(entry()).params(),
        &[SqlParam::U32(19), SqlParam::U8(2), SqlParam::U8(1)]
    );
    assert_eq!(
        writer::insert_last_played(entry(), "Synthetic").params(),
        &[
            SqlParam::U32(19),
            SqlParam::U8(2),
            SqlParam::U8(1),
            SqlParam::U32(0xABCD),
            SqlParam::String("Synthetic".into()),
            SqlParam::U64(17),
            SqlParam::U32(500),
        ]
    );
}

#[test]
fn unicode_names_are_preserved_without_a_byte_count_assumption() {
    let mut character = character();
    character.name = "玩家".into();
    let policy = plan(entry(), &character, 1, None).unwrap();
    assert_eq!(policy.last_played, LastPlayedWrite::Insert);
    let statement = writer::insert_last_played(entry(), character.name);
    assert_eq!(statement.params()[4], SqlParam::String("玩家".into()));
}
