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

#[test]
fn journal_records_the_full_realm_identity_time_and_format_without_personal_data() {
    let statement = entry().insert();
    assert_eq!(
        statement.sql(),
        "INSERT INTO rustycore_forever_creation_journal (player_guid, account_id, realm_address, save_time, format_version) VALUES (?,?,?,?,?)"
    );
    assert_eq!(
        statement.params(),
        &[
            SqlParam::U64(17),
            SqlParam::U32(19),
            SqlParam::U32(0x0201ABCD),
            SqlParam::U32(500),
            SqlParam::U8(1),
        ]
    );
}

#[test]
fn cleanup_requires_the_exact_acknowledged_marker_and_one_affected_row() {
    let statement = entry().delete();
    assert_eq!(
        statement.sql(),
        "DELETE FROM rustycore_forever_creation_journal WHERE player_guid=? AND account_id=? AND realm_address=? AND save_time=? AND format_version=?"
    );
    assert_eq!(statement.params(), entry().insert().params());
    let cleanup = entry().cleanup();
    assert_eq!(cleanup.expected_rows_for_test(), [Some(1)]);
    assert_eq!(cleanup.sqls_for_test(), [statement.sql()]);
}
