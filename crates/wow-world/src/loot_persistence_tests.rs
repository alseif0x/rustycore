// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Durable loot-money persistence tests for [`super`].
//!
//! Moved with their types from the Session mailbox by issue #189. The
//! admission, fence, unknown-commit and exact-once assertions are unchanged.

#![cfg(test)]

use std::sync::Arc;

use super::*;

#[tokio::test]
async fn durable_money_save_fence_closes_admission_then_waits_for_prior_worker() {
    let tracker = Arc::new(DurableLootMoneyPersistenceTrackerLikeCpp::default());
    let worker = tracker.begin_like_cpp().unwrap();
    let save_fence = tracker.close_admission_for_save_like_cpp();

    assert!(tracker.begin_like_cpp().is_err());
    let wait_tracker = Arc::clone(&tracker);
    let waiter = tokio::spawn(async move {
        wait_tracker.wait_until_idle_like_cpp().await;
    });
    tokio::task::yield_now().await;
    assert!(!waiter.is_finished());

    drop(worker);
    tokio::time::timeout(std::time::Duration::from_secs(1), waiter)
        .await
        .expect("prior durable worker must release the save wait")
        .unwrap();
    assert!(tracker.begin_like_cpp().is_err());

    drop(save_fence);
    assert!(tracker.begin_like_cpp().is_ok());
}

#[test]
fn durable_money_permanent_logout_fence_never_reopens() {
    let tracker = Arc::new(DurableLootMoneyPersistenceTrackerLikeCpp::default());
    let save_fence = tracker.close_admission_for_save_like_cpp();
    tracker.close_admission_permanently_like_cpp();
    drop(save_fence);

    assert!(tracker.begin_like_cpp().is_err());
}

#[test]
fn overlapping_durable_money_save_fences_reopen_only_after_last_drop() {
    let tracker = Arc::new(DurableLootMoneyPersistenceTrackerLikeCpp::default());
    let first = tracker.close_admission_for_save_like_cpp();
    let second = tracker.close_admission_for_save_like_cpp();

    drop(first);
    assert!(
        tracker.begin_like_cpp().is_err(),
        "dropping the first fence must not reopen admission under the second"
    );
    drop(second);
    assert!(tracker.begin_like_cpp().is_ok());
}

#[test]
fn durable_money_completion_uses_one_shared_exact_once_gate() {
    let tracker = Arc::new(DurableLootMoneyPersistenceTrackerLikeCpp::default());
    let mut worker = tracker.begin_like_cpp().unwrap();
    let applied = Arc::new(AtomicBool::new(false));
    worker.commit_like_cpp(DurableLootMoneyCompletionLikeCpp {
        durable_money_before: 40,
        durable_money_after: 47,
        durable_applied_amount: 7,
        applied: Arc::clone(&applied),
    });

    let pending = tracker.pending_completions_like_cpp();
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].durable_money_before, 40);
    assert_eq!(pending[0].durable_money_after, 47);
    assert_eq!(pending[0].durable_applied_amount, 7);
    assert!(
        pending[0]
            .applied
            .compare_exchange(
                false,
                true,
                std::sync::atomic::Ordering::AcqRel,
                std::sync::atomic::Ordering::Acquire,
            )
            .is_ok()
    );
    assert!(tracker.pending_completions_like_cpp().is_empty());
}

#[test]
fn indeterminate_money_outcome_permanently_closes_admission_until_relogin() {
    let tracker = Arc::new(DurableLootMoneyPersistenceTrackerLikeCpp::default());
    let fence = tracker.close_admission_for_save_like_cpp();
    tracker.mark_indeterminate_like_cpp();
    drop(fence);

    assert!(tracker.is_indeterminate_like_cpp());
    assert!(tracker.begin_like_cpp().is_err());
}
