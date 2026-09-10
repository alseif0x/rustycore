//! Tests for the transaction module.
//!
//! Separated from transaction.rs under #687.

use super::{
    ItemGuidAllocatorAdvisoryLockLikeCpp, SqlTransaction, SqlTransactionCommitError,
    retry_deadlocked_operation_like_cpp, validate_rows_affected,
};
use crate::{DatabaseError, PreparedStatement};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;
use tokio::sync::oneshot;

#[tokio::test]
async fn deadlock_retry_lock_is_process_wide_like_cpp() {
    let (locked_tx, locked_rx) = oneshot::channel();
    let (release_tx, release_rx) = oneshot::channel();

    let holder = tokio::spawn(async move {
        SqlTransaction::with_deadlock_retry_lock_for_test(async move {
            locked_tx.send(()).unwrap();
            release_rx.await.unwrap();
        })
        .await;
    });

    locked_rx.await.unwrap();
    assert!(!SqlTransaction::deadlock_retry_lock_probe_for_test().await);

    release_tx.send(()).unwrap();
    holder.await.unwrap();
    assert!(SqlTransaction::deadlock_retry_lock_probe_for_test().await);

    let attempts = AtomicUsize::new(0);
    let value = retry_deadlocked_operation_like_cpp(
        || {
            let attempt = attempts.fetch_add(1, Ordering::Relaxed);
            async move {
                if attempt < 2 {
                    Err(DatabaseError::Transaction("synthetic deadlock".to_string()))
                } else {
                    Ok(7u8)
                }
            }
        },
        |error| error.to_string().contains("deadlock"),
    )
    .await
    .unwrap();
    assert_eq!(value, 7);
    assert_eq!(attempts.load(Ordering::Relaxed), 3);
}

#[test]
fn deadlock_retry_window_matches_cpp() {
    assert_eq!(
        SqlTransaction::deadlock_max_retry_time_like_cpp_for_test(),
        Duration::from_secs(60)
    );
}

#[test]
fn transaction_accepts_raw_sql_and_cleans_up_once_like_cpp() {
    let mut tx = SqlTransaction::new();
    tx.append_raw_sql_like_cpp("DELETE FROM characters WHERE guid = 7");

    assert_eq!(tx.len(), 1);
    assert_eq!(
        tx.sqls_for_test(),
        vec!["DELETE FROM characters WHERE guid = 7"]
    );
    assert!(!tx.cleaned_up_like_cpp());

    tx.cleanup_like_cpp();
    assert!(tx.is_empty());
    assert!(tx.cleaned_up_like_cpp());

    tx.append_raw_sql_like_cpp("DELETE FROM character_inventory WHERE guid = 7");
    assert_eq!(tx.len(), 1);
    tx.cleanup_like_cpp();
    assert_eq!(tx.len(), 1);
}

#[test]
fn transaction_tracks_affected_row_contracts_fail_closed() {
    let mut tx = SqlTransaction::new();
    tx.append(PreparedStatement::raw_sql_like_cpp(
        "UPDATE item_instance SET count = 2 WHERE guid = 1",
    ));
    tx.append_expect_rows_affected(
        PreparedStatement::raw_sql_like_cpp("DELETE FROM item_loot_items WHERE container_id = 1"),
        1,
    );

    assert_eq!(tx.expected_rows_for_test(), vec![None, Some(1)]);
    assert!(validate_rows_affected(1, 1, 1).is_ok());
    let error = validate_rows_affected(1, 1, 0).unwrap_err();
    assert!(matches!(error, DatabaseError::Transaction(_)));
    assert_eq!(
        error.to_string(),
        "transaction failed: statement 1 affected 0 rows; expected exactly 1"
    );
}

#[test]
fn item_guid_allocator_lock_domain_is_stable_private_and_database_scoped() {
    let first = ItemGuidAllocatorAdvisoryLockLikeCpp::lock_name_for_test("characters");
    let same = ItemGuidAllocatorAdvisoryLockLikeCpp::lock_name_for_test("characters");
    let other = ItemGuidAllocatorAdvisoryLockLikeCpp::lock_name_for_test("characters_qa");

    assert_eq!(first, same);
    assert_ne!(first, other);
    assert!(first.starts_with("rustycore:item-guid:"));
    assert!(!first.contains("characters"));
    assert!(
        first.len() <= 64,
        "MySQL named locks are limited to 64 bytes"
    );
}

#[test]
fn commit_outcome_unknown_remains_distinct_from_definite_rollback() {
    let unknown = SqlTransactionCommitError::CommitOutcomeUnknown(DatabaseError::Transaction(
        "connection lost after COMMIT".to_string(),
    ));
    let rollback = SqlTransactionCommitError::DefinitelyRolledBack(DatabaseError::Transaction(
        "statement failed before COMMIT".to_string(),
    ));

    assert!(unknown.is_commit_outcome_unknown_like_cpp());
    assert!(!rollback.is_commit_outcome_unknown_like_cpp());
    assert!(unknown.to_string().contains("outcome is unknown"));
    assert!(rollback.to_string().contains("rolled back"));
}
