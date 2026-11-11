//! Synthetic acknowledgement/cancellation tests, not real database durability.
use super::*;
use crate::DatabaseError;
use std::{future::pending, sync::Mutex, task::Poll};

struct FutureSteps<C, L, K> {
    character: Option<C>,
    login: Option<L>,
    cleanup: Option<K>,
}

impl<C, L, K> CommitSteps for FutureSteps<C, L, K>
where
    C: Future<Output = Result<(), SqlTransactionCommitError>> + Send,
    L: Future<Output = Result<(), SqlTransactionCommitError>> + Send,
    K: Future<Output = Result<(), SqlTransactionCommitError>> + Send,
{
    fn character(&mut self) -> impl Future<Output = Result<(), SqlTransactionCommitError>> + Send {
        self.character.take().unwrap()
    }
    fn login(&mut self) -> impl Future<Output = Result<(), SqlTransactionCommitError>> + Send {
        self.login.take().unwrap()
    }
    fn cleanup(&mut self) -> impl Future<Output = Result<(), SqlTransactionCommitError>> + Send {
        self.cleanup.take().unwrap()
    }
}

async fn commit(
    ids: &mut CreationIds,
    character: impl Future<Output = Result<(), SqlTransactionCommitError>> + Send,
    login: impl Future<Output = Result<(), SqlTransactionCommitError>> + Send,
    cleanup: impl Future<Output = Result<(), SqlTransactionCommitError>> + Send,
) -> Result<(), CreateError> {
    super::commit(
        ids,
        &mut FutureSteps {
            character: Some(character),
            login: Some(login),
            cleanup: Some(cleanup),
        },
    )
    .await
}

fn failure(unknown: bool) -> SqlTransactionCommitError {
    let error = DatabaseError::Transaction("synthetic commit failure".into());
    if unknown {
        SqlTransactionCommitError::CommitOutcomeUnknown(error)
    } else {
        SqlTransactionCommitError::DefinitelyRolledBack(error)
    }
}

async fn phase(
    events: &Mutex<Vec<&'static str>>,
    name: &'static str,
    outcome: Result<(), SqlTransactionCommitError>,
) -> Result<(), SqlTransactionCommitError> {
    events.lock().unwrap().push(name);
    outcome
}

#[tokio::test]
async fn both_acknowledgements_are_required_in_character_then_login_order() {
    let mut ids = CreationIds::default();
    let events = Mutex::new(vec![]);
    for _ in 0..2 {
        assert_eq!(
            commit(
                &mut ids,
                phase(&events, "character", Ok(())),
                phase(&events, "login", Ok(())),
                phase(&events, "cleanup", Ok(())),
            )
            .await,
            Ok(())
        );
        assert!(!ids.recovery_required);
    }
    assert_eq!(
        *events.lock().unwrap(),
        [
            "character",
            "login",
            "cleanup",
            "character",
            "login",
            "cleanup"
        ]
    );
}

#[tokio::test]
async fn definite_character_rollback_skips_login_and_allows_a_later_operation() {
    let mut ids = CreationIds::default();
    let events = Mutex::new(vec![]);
    assert_eq!(
        commit(
            &mut ids,
            phase(&events, "character", Err(failure(false))),
            phase(&events, "forbidden login", Ok(())),
            phase(&events, "forbidden cleanup", Ok(())),
        )
        .await,
        Err(CreateError::Database)
    );
    assert!(!ids.recovery_required);
    assert_eq!(*events.lock().unwrap(), ["character"]);
    assert_eq!(
        commit(
            &mut ids,
            phase(&events, "retry character", Ok(())),
            phase(&events, "retry login", Ok(())),
            phase(&events, "retry cleanup", Ok(())),
        )
        .await,
        Ok(())
    );
    assert_eq!(
        *events.lock().unwrap(),
        [
            "character",
            "retry character",
            "retry login",
            "retry cleanup"
        ]
    );
}

#[tokio::test]
async fn unknown_character_commit_preserves_the_fence_and_never_polls_login_or_retry() {
    let mut ids = CreationIds::default();
    let events = Mutex::new(vec![]);
    assert_eq!(
        commit(
            &mut ids,
            phase(&events, "character", Err(failure(true))),
            phase(&events, "forbidden login", Ok(())),
            phase(&events, "forbidden cleanup", Ok(())),
        )
        .await,
        Err(CreateError::CommitUnknown)
    );
    assert!(ids.recovery_required);
    assert_eq!(
        commit(
            &mut ids,
            phase(&events, "forbidden retry character", Ok(())),
            phase(&events, "forbidden retry login", Ok(())),
            phase(&events, "forbidden retry cleanup", Ok(())),
        )
        .await,
        Err(CreateError::RecoveryRequired)
    );
    assert_eq!(*events.lock().unwrap(), ["character"]);
}

#[tokio::test]
async fn every_login_failure_retains_the_fence_after_character_commit() {
    for unknown in [false, true] {
        let mut ids = CreationIds::default();
        let events = Mutex::new(vec![]);
        assert_eq!(
            commit(
                &mut ids,
                phase(&events, "character", Ok(())),
                phase(&events, "login", Err(failure(unknown))),
                phase(&events, "forbidden cleanup", Ok(())),
            )
            .await,
            Err(CreateError::LoginCommitUnknown)
        );
        assert!(ids.recovery_required);
        assert_eq!(
            commit(
                &mut ids,
                phase(&events, "forbidden retry character", Ok(())),
                phase(&events, "forbidden retry login", Ok(())),
                phase(&events, "forbidden retry cleanup", Ok(())),
            )
            .await,
            Err(CreateError::RecoveryRequired)
        );
        assert_eq!(*events.lock().unwrap(), ["character", "login"]);
    }
}

#[tokio::test]
async fn unpolled_operation_has_no_side_effect_and_does_not_require_recovery() {
    let mut ids = CreationIds::default();
    let events = Mutex::new(vec![]);
    let operation = commit(
        &mut ids,
        phase(&events, "character", Ok(())),
        phase(&events, "login", Ok(())),
        phase(&events, "cleanup", Ok(())),
    );
    drop(operation);
    assert!(!ids.recovery_required);
    assert!(events.lock().unwrap().is_empty());
}

#[tokio::test]
async fn cancellation_during_any_commit_preserves_the_fence() {
    for pending_phase in 0..3 {
        let mut ids = CreationIds::default();
        let events = Mutex::new(vec![]);
        let character = async {
            events.lock().unwrap().push("character");
            if pending_phase == 0 {
                pending().await
            } else {
                Ok(())
            }
        };
        let login = async {
            events.lock().unwrap().push("login");
            if pending_phase == 1 {
                pending().await
            } else {
                Ok(())
            }
        };
        let cleanup = async {
            events.lock().unwrap().push("cleanup");
            pending().await
        };
        let mut operation = Box::pin(commit(&mut ids, character, login, cleanup));
        assert!(
            std::future::poll_fn(|cx| Poll::Ready(operation.as_mut().poll(cx).is_pending())).await
        );
        drop(operation);
        assert!(ids.recovery_required);
        assert_eq!(
            commit(
                &mut ids,
                phase(&events, "forbidden retry character", Ok(())),
                phase(&events, "forbidden retry login", Ok(())),
                phase(&events, "forbidden retry cleanup", Ok(())),
            )
            .await,
            Err(CreateError::RecoveryRequired)
        );
        let expected = &["character", "login", "cleanup"][..pending_phase + 1];
        assert_eq!(*events.lock().unwrap(), expected);
    }
}

#[tokio::test]
async fn cancellation_retains_recovery_state_after_the_repository_mutex_is_released() {
    let ids = tokio::sync::Mutex::new(CreationIds::default());
    let mut operation = Box::pin(async {
        let mut guard = ids.lock().await;
        commit(&mut guard, pending(), async { Ok(()) }, async { Ok(()) }).await
    });
    assert!(std::future::poll_fn(|cx| Poll::Ready(operation.as_mut().poll(cx).is_pending())).await);
    drop(operation);
    let mut next_session = ids.lock().await;
    assert!(next_session.recovery_required);
    assert_eq!(
        commit(
            &mut next_session,
            async { Ok(()) },
            async { Ok(()) },
            async { Ok(()) }
        )
        .await,
        Err(CreateError::RecoveryRequired)
    );
}

#[tokio::test]
async fn cleanup_failure_after_both_databases_commit_still_requires_recovery() {
    for unknown in [false, true] {
        let mut ids = CreationIds::default();
        let events = Mutex::new(vec![]);
        assert_eq!(
            commit(
                &mut ids,
                phase(&events, "character", Ok(())),
                phase(&events, "login", Ok(())),
                phase(&events, "cleanup", Err(failure(unknown))),
            )
            .await,
            Err(CreateError::RecoveryRequired)
        );
        assert!(ids.recovery_required);
        assert_eq!(
            commit(
                &mut ids,
                phase(&events, "forbidden character", Ok(())),
                phase(&events, "forbidden login", Ok(())),
                phase(&events, "forbidden cleanup", Ok(())),
            )
            .await,
            Err(CreateError::RecoveryRequired)
        );
        assert_eq!(*events.lock().unwrap(), ["character", "login", "cleanup"]);
    }
}
