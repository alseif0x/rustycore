use std::collections::BTreeMap;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicU64, Ordering},
};
use std::time::Duration;

use super::{
    ActiveWorldSessionCancellationLikeCpp,
    ActiveWorldSessionLikeCpp,
    ActiveWorldSessionRegistryLikeCpp,
    ActiveWorldSessionRegistrationGuardLikeCpp,
    SessionCommand,
};

mod quiescence;
pub(crate) use quiescence::{ProducerKind, TickAdmission, TickDisposition, TickPhase};
use quiescence::{CoordinationLedger, next_coordination_issuer};

#[derive(Debug, Default)]
pub(super) struct ActiveSessionState {
    sessions: BTreeMap<u64, ActiveWorldSessionLikeCpp>,
    coordination: CoordinationLedger,
}

impl ActiveWorldSessionCancellationLikeCpp {
    fn cancel_like_cpp(&self) {
        self.cancelled.store(true, Ordering::Release);
        // One cancellation waiter exists per session; `notify_one` stores a
        // permit when the waiter has not been polled yet, avoiding a lost wake.
        self.notify.notify_one();
    }

    pub(super) async fn cancelled_like_cpp(&self) {
        loop {
            let notified = self.notify.notified();
            if self.cancelled.load(Ordering::Acquire) {
                return;
            }
            notified.await;
        }
    }
}

impl Drop for ActiveWorldSessionRegistrationGuardLikeCpp {
    fn drop(&mut self) {
        self.registry.unregister(self.id);
    }
}

impl Default for ActiveWorldSessionRegistryLikeCpp {
    fn default() -> Self {
        Self {
            next_id: AtomicU64::new(0),
            inner: Mutex::new(ActiveSessionState::default()),
            coordination_changed: tokio::sync::Notify::new(),
            coordination_issuer: next_coordination_issuer(),
            stop_sessions: AtomicBool::new(false),
        }
    }
}

impl ActiveWorldSessionRegistryLikeCpp {
    pub(super) fn new() -> Self {
        Self::default()
    }

    pub(super) fn try_register(
        &self,
        account_id: u32,
        command_tx: flume::Sender<SessionCommand>,
        phase_tx: flume::Sender<wow_world::session::mailbox::SessionPhaseRequestLikeCpp>,
    ) -> Option<(
        u64,
        Arc<ActiveWorldSessionCancellationLikeCpp>,
        Arc<AtomicBool>,
    )> {
        let mut sessions = self
            .inner
            .lock()
            .expect("active world session registry lock poisoned");
        if sessions.coordination.shutdown_requested() {
            return None;
        }
        let id = self
            .next_id
            .fetch_add(1, Ordering::Relaxed)
            .saturating_add(1);
        let cancellation = Arc::new(ActiveWorldSessionCancellationLikeCpp::default());
        let ready_for_phases_like_cpp = Arc::new(AtomicBool::new(false));
        sessions.sessions.insert(
            id,
            ActiveWorldSessionLikeCpp {
                account_id,
                command_tx,
                phase_tx,
                ready_for_phases_like_cpp: Arc::clone(&ready_for_phases_like_cpp),
                cancellation: Arc::clone(&cancellation),
            },
        );
        Some((id, cancellation, ready_for_phases_like_cpp))
    }

    /// The sessions C++ `World::UpdateSessions` would drive this step: every
    /// registered session that is actually consuming its phase rail, including
    /// those still on the character screen (`World.cpp:3394-3420`).
    pub(super) fn world_phase_participants_like_cpp(
        &self,
    ) -> Vec<flume::Sender<wow_world::session::mailbox::SessionPhaseRequestLikeCpp>> {
        let sessions = self
            .inner
            .lock()
            .expect("active world session registry lock poisoned");
        sessions.sessions
            .values()
            .filter(|session| session.ready_for_phases_like_cpp.load(Ordering::Acquire))
            .map(|session| session.phase_tx.clone())
            .collect()
    }

    #[cfg(test)]
    pub(super) fn register(&self, account_id: u32, command_tx: flume::Sender<SessionCommand>) -> u64 {
        let (phase_tx, _phase_rx) = flume::bounded(2);
        self.try_register(account_id, command_tx, phase_tx)
            .expect("test registry must still accept sessions")
            .0
    }

    pub(super) fn begin_shutdown_like_cpp(&self) {
        // Registration and producer admission close under the same mutex.
        let _ = self.close_tick_admission();
    }

    pub(super) fn is_shutting_down_like_cpp(&self) -> bool {
        self.inner.lock().expect("active world session registry lock poisoned")
            .coordination.shutdown_requested()
    }

    pub(super) fn request_session_stop_like_cpp(&self) {
        self.stop_sessions.store(true, Ordering::Release);
    }

    pub(super) fn should_stop_sessions_like_cpp(&self) -> bool {
        self.stop_sessions.load(Ordering::Acquire)
    }

    pub(super) fn cancel_all_sessions_like_cpp(&self) -> usize {
        debug_assert!(self.is_shutting_down_like_cpp());
        let sessions = self.snapshot_like_cpp();
        for (_, session) in &sessions {
            session.cancellation.cancel_like_cpp();
        }
        sessions.len()
    }

    pub(super) fn unregister(&self, id: u64) -> Option<ActiveWorldSessionLikeCpp> {
        let mut sessions = self
            .inner
            .lock()
            .expect("active world session registry lock poisoned");
        let retired = sessions.sessions.remove(&id);
        drop(sessions);
        self.coordination_changed.notify_waiters();
        retired
    }

    pub(super) fn snapshot_like_cpp(&self) -> Vec<(u64, ActiveWorldSessionLikeCpp)> {
        let sessions = self
            .inner
            .lock()
            .expect("active world session registry lock poisoned");
        sessions.sessions
            .iter()
            .map(|(id, session)| (*id, session.clone()))
            .collect()
    }

    pub(super) fn len_like_cpp(&self) -> usize {
        self.inner
            .lock()
            .expect("active world session registry lock poisoned")
            .sessions.len()
    }

    pub(super) fn is_empty_like_cpp(&self) -> bool {
        self.len_like_cpp() == 0
    }

    pub(super) async fn wait_until_empty_like_cpp(&self, wait_timeout: Duration) -> bool {
        tokio::time::timeout(wait_timeout, async {
            while !self.is_empty_like_cpp() {
                // Polling avoids losing a `notify_waiters` wake between an
                // empty check and creation of a `Notified` future.
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .is_ok()
    }

    #[cfg(test)]
    pub(super) fn len(&self) -> usize {
        self.len_like_cpp()
    }
}
