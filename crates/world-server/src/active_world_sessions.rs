// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Active world-session registration, phase participation and cancellation.

use std::collections::BTreeMap;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicU64, Ordering},
};
use std::time::Duration;

use tokio::sync::Notify;
use wow_world::session::mailbox::SessionCommand;

#[derive(Debug, Default)]
pub(super) struct ActiveWorldSessionCancellationLikeCpp {
    cancelled: AtomicBool,
    notify: Notify,
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

#[derive(Clone, Debug)]
pub(super) struct ActiveWorldSessionLikeCpp {
    pub(super) account_id: u32,
    pub(super) command_tx: flume::Sender<SessionCommand>,
    /// The session's phase rail (#787). Registration alone does not mean the
    /// session is consuming it: `ready_for_phases_like_cpp` is set by the task
    /// that owns the session once it actually parks on the rail, so the
    /// producer never counts a session that cannot answer.
    phase_tx: flume::Sender<wow_world::session::mailbox::SessionPhaseRequestLikeCpp>,
    ready_for_phases_like_cpp: Arc<AtomicBool>,
    cancellation: Arc<ActiveWorldSessionCancellationLikeCpp>,
}

#[derive(Debug)]
pub(super) struct ActiveWorldSessionRegistrationGuardLikeCpp {
    pub(super) registry: Arc<ActiveWorldSessionRegistryLikeCpp>,
    pub(super) id: u64,
}

impl Drop for ActiveWorldSessionRegistrationGuardLikeCpp {
    fn drop(&mut self) {
        self.registry.unregister(self.id);
    }
}

/// Minimal Rust equivalent of C++ `World::m_sessions`.
///
/// C++ `World::KickAll` / `World::UpdateSessions` operate on all active
/// `WorldSession` objects, including authenticated sessions still on the
/// character screen. `PlayerRegistry` is not enough because it only contains
/// sessions with a logged-in player. This registry intentionally stores only
/// the command rail needed by world-owned operations; the session task remains
/// the sole owner of `WorldSession` mutation.
#[derive(Debug)]
pub(super) struct ActiveWorldSessionRegistryLikeCpp {
    next_id: AtomicU64,
    sessions: Mutex<BTreeMap<u64, ActiveWorldSessionLikeCpp>>,
    accepting_sessions: AtomicBool,
    stop_sessions: AtomicBool,
}

impl Default for ActiveWorldSessionRegistryLikeCpp {
    fn default() -> Self {
        Self {
            next_id: AtomicU64::new(0),
            sessions: Mutex::new(BTreeMap::new()),
            accepting_sessions: AtomicBool::new(true),
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
            .sessions
            .lock()
            .expect("active world session registry lock poisoned");
        if !self.accepting_sessions.load(Ordering::Acquire) {
            return None;
        }
        let id = self
            .next_id
            .fetch_add(1, Ordering::Relaxed)
            .saturating_add(1);
        let cancellation = Arc::new(ActiveWorldSessionCancellationLikeCpp::default());
        let ready_for_phases_like_cpp = Arc::new(AtomicBool::new(false));
        sessions.insert(
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
            .sessions
            .lock()
            .expect("active world session registry lock poisoned");
        sessions
            .values()
            .filter(|session| session.ready_for_phases_like_cpp.load(Ordering::Acquire))
            .map(|session| session.phase_tx.clone())
            .collect()
    }

    #[cfg(test)]
    pub(super) fn register(
        &self,
        account_id: u32,
        command_tx: flume::Sender<SessionCommand>,
    ) -> u64 {
        let (phase_tx, _phase_rx) = flume::bounded(2);
        self.try_register(account_id, command_tx, phase_tx)
            .expect("test registry must still accept sessions")
            .0
    }

    pub(super) fn begin_shutdown_like_cpp(&self) {
        // Serialize with `try_register`: when this method returns, every
        // registration that won the race is already visible in `sessions`,
        // and every later registration is rejected.
        let _sessions = self
            .sessions
            .lock()
            .expect("active world session registry lock poisoned");
        self.accepting_sessions.store(false, Ordering::Release);
    }

    pub(super) fn is_shutting_down_like_cpp(&self) -> bool {
        !self.accepting_sessions.load(Ordering::Acquire)
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
            .sessions
            .lock()
            .expect("active world session registry lock poisoned");
        sessions.remove(&id)
    }

    pub(super) fn snapshot_like_cpp(&self) -> Vec<(u64, ActiveWorldSessionLikeCpp)> {
        let sessions = self
            .sessions
            .lock()
            .expect("active world session registry lock poisoned");
        sessions
            .iter()
            .map(|(id, session)| (*id, session.clone()))
            .collect()
    }

    pub(super) fn len_like_cpp(&self) -> usize {
        self.sessions
            .lock()
            .expect("active world session registry lock poisoned")
            .len()
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
