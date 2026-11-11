//! Ordered CharacterDB/LoginDB acknowledgement followed by journal cleanup.
//! The durable marker is part of CharacterDB's first transaction; these steps
//! do not turn the separate databases into one atomic transaction.
use super::{CreateError, CreationIds};
use crate::SqlTransactionCommitError;
use std::future::Future;

/// Three sequential effects sharing a physical writer. Each effect is polled
/// only after its predecessor has finished; no aliasing writer futures exist.
pub(super) trait CommitSteps: Send {
    fn character(&mut self) -> impl Future<Output = Result<(), SqlTransactionCommitError>> + Send;
    fn login(&mut self) -> impl Future<Output = Result<(), SqlTransactionCommitError>> + Send;
    fn cleanup(&mut self) -> impl Future<Output = Result<(), SqlTransactionCommitError>> + Send;
}

pub(super) struct DatabaseCommits<'a> {
    pub(super) writer: &'a mut super::writer::CreationWriter,
    pub(super) character: Option<crate::SqlTransaction>,
    pub(super) login: Option<(
        crate::SqlTransaction,
        sqlx::Transaction<'static, sqlx::MySql>,
    )>,
    pub(super) cleanup: Option<crate::SqlTransaction>,
}

fn consumed_phase() -> SqlTransactionCommitError {
    SqlTransactionCommitError::DefinitelyRolledBack(crate::DatabaseError::Transaction(
        "creation phase already consumed".into(),
    ))
}

impl CommitSteps for DatabaseCommits<'_> {
    async fn character(&mut self) -> Result<(), SqlTransactionCommitError> {
        let batch = self.character.take().ok_or_else(consumed_phase)?;
        self.writer.commit(batch).await
    }
    async fn login(&mut self) -> Result<(), SqlTransactionCommitError> {
        let (batch, transaction) = self.login.take().ok_or_else(consumed_phase)?;
        super::writer::commit_auth(batch, transaction).await
    }
    async fn cleanup(&mut self) -> Result<(), SqlTransactionCommitError> {
        let batch = self.cleanup.take().ok_or_else(consumed_phase)?;
        self.writer.commit(batch).await
    }
}

#[cfg(test)]
#[path = "../../../unit_tests/forever/creation_commits.rs"]
mod tests;

pub(super) async fn commit(
    ids: &mut CreationIds,
    steps: &mut impl CommitSteps,
) -> Result<(), CreateError> {
    if ids.recovery_required {
        return Err(CreateError::RecoveryRequired);
    }
    // The caller holds its repository's async allocator mutex. Set the fence
    // before polling any durable work: dropping this future must retain it.
    ids.recovery_required = true;
    match steps.character().await {
        Ok(()) => {}
        Err(SqlTransactionCommitError::DefinitelyRolledBack(_)) => {
            ids.recovery_required = false;
            return Err(CreateError::Database);
        }
        Err(SqlTransactionCommitError::CommitOutcomeUnknown(_)) => {
            return Err(CreateError::CommitUnknown);
        }
    }
    // CharacterDB has committed. Even a definite LoginDB rollback is a
    // partial operation, so no LoginDB error may release the fence.
    steps
        .login()
        .await
        .map_err(|_| CreateError::LoginCommitUnknown)?;
    // A failed/unknown cleanup leaves durable evidence, or may have removed it
    // after both databases committed. Neither permits another unchecked write.
    steps
        .cleanup()
        .await
        .map_err(|_| CreateError::RecoveryRequired)?;
    ids.recovery_required = false;
    Ok(())
}
