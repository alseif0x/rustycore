//! Character-list application operations without Session, packets or SQL access.
//!
//! C++ CharacterHandler.cpp:1550-1610 separates query submission from its ready
//! callback. This owned operation preserves the existing Rust result-before-response
//! fence. Query preparation cannot submit a transaction: the Session must consume
//! its ready continuation to do so. Session owns callback admission/publication;
//! composition drains submitted commits before retiring the Session.

use std::future::Future;
use std::sync::Arc;

use wow_persistence::{
    CharacterAdministrationLoadOutcomeLikeCpp as LoadOutcome,
    CharacterAdministrationMutationOutcomeLikeCpp as MutationOutcome,
    CharacterAdministrationPersistencePortLikeCpp,
};

const AT_LOGIN_RENAME: u16 = 0x001;

pub(crate) struct RenameRequest {
    pub guid: u64,
    pub new_name: String,
}

pub(crate) enum RenameFailure {
    NotFound,
    QueryFailed(String),
    NotEligible,
    CommitFailed(String),
}

pub(crate) struct RenameOutcome {
    pub new_name: String,
    /// The previous name is needed only for the confirmed-result log.
    pub result: Result<String, RenameFailure>,
}

pub(crate) enum RenamePreparation {
    Rejected(RenameOutcome),
    Ready(PreparedRename),
}

/// An owned, single-use continuation, not a submitted transaction. Dropping it
/// cannot write. No Clone or field access allows callers to replay its commit.
pub(crate) struct PreparedRename {
    port: Arc<dyn CharacterAdministrationPersistencePortLikeCpp>,
    request: RenameRequest,
    old_name: String,
    remaining_flags: u16,
}

impl PreparedRename {
    /// The admitted callback consumes this continuation. Construction of the
    /// future is lazy; once polled, cancellation is NOT evidence of rollback.
    pub(crate) fn commit(self) -> impl Future<Output = RenameOutcome> + Send + 'static {
        async move {
            let result = match self
                .port
                .commit_rename_like_cpp(
                    self.request.guid,
                    &self.request.new_name,
                    self.remaining_flags,
                )
                .await
            {
                MutationOutcome::Applied => Ok(self.old_name),
                MutationOutcome::Failed { reason } => Err(RenameFailure::CommitFailed(reason)),
            };
            RenameOutcome {
                new_name: self.request.new_name,
                result,
            }
        }
    }
}

/// The read stage may be polled independently, but its ready value has no write
/// effects. A future Session-owned queue can discard it on retirement, including
/// after the database read completes but before the callback is admitted.
pub(crate) fn prepare_rename(
    port: Arc<dyn CharacterAdministrationPersistencePortLikeCpp>,
    request: RenameRequest,
) -> impl Future<Output = RenamePreparation> + Send + 'static {
    async move {
        let failure = match port
            .load_rename_candidate_like_cpp(request.guid, &request.new_name)
            .await
        {
            LoadOutcome::Loaded(candidate) if candidate.at_login_flags & AT_LOGIN_RENAME != 0 => {
                return RenamePreparation::Ready(PreparedRename {
                    port,
                    request,
                    old_name: candidate.old_name,
                    remaining_flags: candidate.at_login_flags & !AT_LOGIN_RENAME,
                });
            }
            LoadOutcome::Loaded(_) => RenameFailure::NotEligible,
            LoadOutcome::NotFound => RenameFailure::NotFound,
            LoadOutcome::Failed { reason } => RenameFailure::QueryFailed(reason),
        };
        RenamePreparation::Rejected(RenameOutcome {
            new_name: request.new_name,
            result: Err(failure),
        })
    }
}

#[cfg(test)]
mod tests;
