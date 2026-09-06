//! Session-owned read callbacks and supervised commit handles.
//!
//! Workers never retain Session, Player or transport. Read completion is not
//! commit admission: only the Session callback pass may consume a prepared read.

use super::{RenameFailure, RenameOutcome, RenamePreparation, RenameRequest, prepare_rename};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll, Waker};
use tokio::task::JoinHandle;
use wow_core::ObjectGuid;
use wow_persistence::CharacterAdministrationPersistencePortLikeCpp;

struct Read {
    guid: ObjectGuid,
    name: String,
    task: JoinHandle<RenamePreparation>,
}

impl Drop for Read {
    fn drop(&mut self) {
        // No read worker can submit a transaction, even if abort races readiness.
        self.task.abort();
    }
}

struct Commit {
    guid: ObjectGuid,
    name: String,
    task: JoinHandle<RenameOutcome>,
}

#[derive(Default)]
pub(crate) struct RenameCallbacks {
    reads: Vec<Read>,
    commits: Vec<Commit>,
    closed: bool,
    worker_failed: bool,
}

fn ready<T>(task: &mut JoinHandle<T>) -> Option<Result<T, tokio::task::JoinError>> {
    // The owning Session polls each callback pass; a worker cannot reenter it.
    match Pin::new(task).poll(&mut Context::from_waker(Waker::noop())) {
        Poll::Ready(value) => Some(value),
        Poll::Pending => None,
    }
}

impl RenameCallbacks {
    pub(crate) fn submit(
        &mut self,
        port: Arc<dyn CharacterAdministrationPersistencePortLikeCpp>,
        guid: ObjectGuid,
        new_name: String,
    ) -> bool {
        if self.closed {
            return false;
        }
        self.reads.push(Read {
            guid,
            name: new_name.clone(),
            task: tokio::spawn(prepare_rename(
                port,
                RenameRequest {
                    guid: guid.counter() as u64,
                    new_name,
                },
            )),
        });
        true
    }

    pub(crate) fn process_ready(&mut self) -> Vec<(ObjectGuid, RenameOutcome)> {
        if self.closed {
            return Vec::new();
        }
        let mut outcomes = Vec::new();
        // Observe previously submitted commits first. New commits below cannot
        // publish in the same pass merely because a worker happens to run fast.
        let mut index = 0;
        while index < self.commits.len() {
            let Some(result) = ready(&mut self.commits[index].task) else {
                index += 1;
                continue;
            };
            let commit = self.commits.remove(index);
            let outcome = match result {
                Ok(outcome) => outcome,
                Err(error) => {
                    self.worker_failed = true;
                    RenameOutcome {
                        new_name: commit.name,
                        result: Err(RenameFailure::CommitFailed(format!(
                            "rename worker failed; transaction completion unproven: {error}"
                        ))),
                    }
                }
            };
            outcomes.push((commit.guid, outcome));
        }
        // C++ checks the registered snapshot in registration order, skipping
        // pending reads. Completion-order task queues would change that order.
        let mut index = 0;
        while index < self.reads.len() {
            let Some(result) = ready(&mut self.reads[index].task) else {
                index += 1;
                continue;
            };
            let read = self.reads.remove(index);
            match result {
                Ok(RenamePreparation::Ready(prepared)) => self.commits.push(Commit {
                    guid: read.guid,
                    name: read.name.clone(),
                    task: tokio::spawn(prepared.commit()),
                }),
                Ok(RenamePreparation::Rejected(outcome)) => outcomes.push((read.guid, outcome)),
                Err(error) => {
                    self.worker_failed = true;
                    outcomes.push((
                        read.guid,
                        RenameOutcome {
                            new_name: read.name.clone(),
                            result: Err(RenameFailure::QueryFailed(format!(
                                "rename read worker failed: {error}"
                            ))),
                        },
                    ));
                }
            }
        }
        outcomes
    }

    /// Stop read admission and discard all read callbacks, including ready ones.
    /// Keep submitted commit handles on self across cancellation of this drain.
    /// No response is published during retirement. False is not clean quiescence.
    pub(crate) async fn finish(&mut self) -> bool {
        self.closed = true;
        self.reads.clear();
        while let Some(commit) = self.commits.first_mut() {
            if let Err(error) = (&mut commit.task).await {
                tracing::error!(%error, "Rename commit worker failed during retirement; completion unproven");
                self.worker_failed = true;
            }
            self.commits.remove(0);
        }
        !self.worker_failed
    }
}
