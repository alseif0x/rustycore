//! Character-list application operations without Session, packets or SQL access.
//!
//! C++ CharacterHandler.cpp:1550-1610 separates query submission from its ready
//! callback. This owned operation preserves the existing Rust result-before-response
//! fence; it does not yet implement that callback scheduling or authorize detachment.

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

/// Admission (account ownership and name validation) belongs to the caller.
/// No Session/entity borrow, guard, transport handle or catalog crosses either await.
/// Cancellation before the query completes must not start the transaction. Once a
/// commit is submitted, dropping this future is NOT proof of database rollback.
/// The caller still awaits this operation; no background worker is created here.
pub(crate) fn rename(
    port: Arc<dyn CharacterAdministrationPersistencePortLikeCpp>,
    request: RenameRequest,
) -> impl Future<Output = RenameOutcome> + Send + 'static {
    async move {
        let result = persist_rename(port.as_ref(), &request).await;
        RenameOutcome {
            new_name: request.new_name,
            result,
        }
    }
}

async fn persist_rename(
    port: &dyn CharacterAdministrationPersistencePortLikeCpp,
    request: &RenameRequest,
) -> Result<String, RenameFailure> {
    let candidate = match port
        .load_rename_candidate_like_cpp(request.guid, &request.new_name)
        .await
    {
        LoadOutcome::Loaded(candidate) => candidate,
        LoadOutcome::NotFound => return Err(RenameFailure::NotFound),
        LoadOutcome::Failed { reason } => return Err(RenameFailure::QueryFailed(reason)),
    };
    if candidate.at_login_flags & AT_LOGIN_RENAME == 0 {
        return Err(RenameFailure::NotEligible);
    }
    let remaining_flags = candidate.at_login_flags & !AT_LOGIN_RENAME;
    match port
        .commit_rename_like_cpp(request.guid, &request.new_name, remaining_flags)
        .await
    {
        MutationOutcome::Applied => Ok(candidate.old_name),
        MutationOutcome::Failed { reason } => Err(RenameFailure::CommitFailed(reason)),
    }
}
