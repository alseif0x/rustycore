//! Cooperative return transfers retained owners; it does not settle their work.
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::task::JoinHandle;

use crate::runtime::map_tick::object_work::CanonicalObjectResumeFailure;
use crate::session_supervision::TickAdmission;

mod before_objects;
pub(super) use before_objects::BeforeObjectsExitCause;

pub(crate) enum CanonicalMapProducerExit {
    /// No Objects failure was returned. Other obligations still need a receipt.
    WithoutObjectFailure,
    RetainedObjects {
        failure: CanonicalObjectResumeFailure,
        admission: TickAdmission,
    },
    RetainedBeforeObjects {
        cause: BeforeObjectsExitCause,
        plan: crate::runtime::map_tick::CanonicalMapSessionPassPlanLikeCpp,
        admission: TickAdmission,
        unresolved_permits:
            Vec<std::sync::Arc<wow_world::session::mailbox::SessionPhasePermitLikeCpp>>,
    },
}

/// Called only inside the producer's existing held-failure guard.
/// Stop false leaves the exact owners in place; stop true moves them once.
pub(super) fn take_retained_objects_on_stop(
    held: &mut Option<(CanonicalObjectResumeFailure, TickAdmission)>,
    stop: &AtomicBool,
) -> Option<CanonicalMapProducerExit> {
    if !stop.load(Ordering::Acquire) {
        return None;
    }
    held.take().map(
        |(failure, admission)| CanonicalMapProducerExit::RetainedObjects { failure, admission },
    )
}

/// Keep the original return in the supervisor's shutdown scope. A previously
/// consumed select result must never cause a second poll of its JoinHandle.
/// True describes a return without a retained failure, not quiescence.
pub(crate) async fn stop_canonical_map_producer(
    handle: &mut JoinHandle<CanonicalMapProducerExit>,
    already_finished: bool,
    exit: &mut Option<CanonicalMapProducerExit>,
) -> bool {
    if !already_finished && exit.is_none() {
        match tokio::time::timeout(crate::RESPAWN_DB_PRODUCER_STOP_TIMEOUT, &mut *handle).await {
            Ok(Ok(returned)) => *exit = Some(returned),
            Ok(Err(error)) => {
                tracing::error!(%error, "Canonical map producer join failed; settlement is unproven");
                return false;
            }
            Err(_) => {
                // Match the existing producer stop: request abort, without a
                // blocking join or a cleanup/receipt claim. Payload return is
                // not guaranteed after forced cancellation.
                handle.abort();
                tracing::error!(
                    timeout_ms = crate::RESPAWN_DB_PRODUCER_STOP_TIMEOUT.as_millis(),
                    "Canonical map producer stop timed out; abort requested and settlement is unproven"
                );
                return false;
            }
        }
    }

    match exit.as_ref() {
        Some(CanonicalMapProducerExit::WithoutObjectFailure) => true,
        Some(CanonicalMapProducerExit::RetainedObjects { .. }) => {
            tracing::error!(
                "Canonical map producer returned original Objects owners; shutdown remains incomplete"
            );
            false
        }
        Some(CanonicalMapProducerExit::RetainedBeforeObjects { .. }) => {
            tracing::error!(
                "Canonical map producer returned original before-Objects owners; shutdown remains incomplete"
            );
            false
        }
        None => {
            tracing::error!(
                "Canonical map producer has no owned join result; shutdown remains incomplete"
            );
            false
        }
    }
}

#[cfg(test)]
mod tests;
