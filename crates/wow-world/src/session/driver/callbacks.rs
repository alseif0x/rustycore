//! Session-owned read callbacks and supervised commit handles.
//!
//! Workers never retain Session, Player or transport. Read completion is not
//! commit admission: only the Session callback pass may consume a prepared read.

use super::super::WorldSession;
use crate::character_administration::{
    RenameFailure, RenameOutcome, RenamePreparation, RenameRequest, prepare_rename,
};
use std::collections::VecDeque;
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

enum Delivery {
    Pending(flume::r#async::SendFut<'static, Vec<u8>>),
    Accepted,
}

#[derive(Default)]
pub(in crate::session) struct RenameCallbacks {
    reads: Vec<Read>,
    commits: Vec<Commit>,
    pending_results: VecDeque<(ObjectGuid, RenameOutcome)>,
    pending_delivery: VecDeque<Delivery>,
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
    fn submit(
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

    fn has_worker_failure(&self) -> bool {
        self.worker_failed
    }

    fn stop_read_admission(&mut self) {
        self.closed = true;
        self.reads.clear();
    }

    /// Only remove a result after the owning Session successfully enqueues it.
    fn acknowledge_result(&mut self) {
        self.pending_delivery.pop_front();
        self.pending_results.pop_front();
    }

    fn process_ready(&mut self) {
        if self.closed || !self.pending_results.is_empty() {
            // A saturated sink must not admit more writes from ready reads.
            // Existing commit workers remain retained for retirement/drain.
            return;
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
        if self.worker_failed {
            self.stop_read_admission();
            self.pending_results.extend(outcomes);
            return;
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
                    break;
                }
            }
        }
        if self.worker_failed {
            self.stop_read_admission();
        }
        self.pending_results.extend(outcomes);
    }

    /// Stop read admission and discard all read callbacks, including ready ones.
    /// Keep submitted commit handles on self across cancellation of this drain.
    /// No response is published during retirement. False is not clean quiescence.
    async fn finish(&mut self) -> bool {
        self.stop_read_admission();
        self.pending_delivery.clear();
        self.pending_results.clear();
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

impl WorldSession {
    pub(crate) fn submit_character_rename_like_cpp(
        &mut self,
        port: std::sync::Arc<dyn wow_persistence::CharacterAdministrationPersistencePortLikeCpp>,
        guid: wow_core::ObjectGuid,
        name: String,
    ) -> bool {
        self.character_rename_callbacks.submit(port, guid, name)
    }

    /// The production driver invokes this after packet dispatch. Full World/Map
    /// coordination remains separate; this method never waits for a DB worker.
    pub fn process_ready_character_rename_callbacks_like_cpp(&mut self) {
        self.character_rename_callbacks.process_ready();
        if self.character_rename_callbacks.has_worker_failure() {
            // Join failure is not an ordinary DB rejection or proven rollback.
            // Retire this Session and let composition drain/classify remaining work.
            self.kick("Character rename worker failed; completion unproven");
            return;
        }
        for index in self.character_rename_callbacks.pending_delivery.len()
            ..self.character_rename_callbacks.pending_results.len()
        {
            let (guid, outcome) = &self.character_rename_callbacks.pending_results[index];
            let delivery = self.enqueue_character_rename_like_cpp(*guid, outcome);
            self.character_rename_callbacks
                .pending_delivery
                .push_back(Delivery::Pending(delivery));
        }
        let mut disconnected = false;
        for delivery in &mut self.character_rename_callbacks.pending_delivery {
            if let Delivery::Pending(future) = delivery {
                // Register the ENTIRE ready batch in the shared channel FIFO,
                // not just its head. Later packets must not overtake its tail.
                match Pin::new(future).poll(&mut Context::from_waker(Waker::noop())) {
                    Poll::Ready(Ok(())) => *delivery = Delivery::Accepted,
                    Poll::Pending => {}
                    Poll::Ready(Err(_)) => {
                        disconnected = true;
                        break;
                    }
                }
            }
        }
        if disconnected {
            self.character_rename_callbacks.stop_read_admission();
            self.character_rename_callbacks.pending_delivery.clear();
            self.character_rename_callbacks.pending_results.clear();
            self.kick("Character rename response channel closed");
            return;
        }
        while matches!(
            self.character_rename_callbacks.pending_delivery.front(),
            Some(Delivery::Accepted)
        ) {
            self.character_rename_callbacks.acknowledge_result();
        }
    }

    /// Composition calls this before disconnect save and Session retirement.
    /// Pending reads cannot admit new commits; submitted writes are joined.
    /// Cancelling this await retains remaining handles for a repeated drain.
    pub async fn finish_character_rename_callbacks_like_cpp(&mut self) -> bool {
        self.character_rename_callbacks.finish().await
    }
}
