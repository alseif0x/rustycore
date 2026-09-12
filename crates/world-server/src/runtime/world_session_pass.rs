// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! The world phase of one canonical step (#787).
//!
//! C++ `World::Update` runs `UpdateSessions(diff)` (`World.cpp:2704`) over
//! every session in `World::m_sessions` — character screen included — and only
//! afterwards `MapManager::Update(diff)` (`World.cpp:2748`). This module is the
//! first half of that order: it drives every session that is consuming its
//! phase rail with `WorldSessionFilter`, waits for each completion boundary,
//! and lets the caller begin the map tick only once the world pass is done.
//!
//! The same permit rule as the map phase applies: the deadline may revoke a
//! pass that has not started, and proves nothing about one that has.

use std::sync::Arc;
use std::time::{Duration, Instant};

use tracing::{error, warn};
use wow_world::session::mailbox::{
    RunWorldPhasePassLikeCppRequest, RunWorldPhasePassResultLikeCpp,
    SessionPhasePassOutcomeLikeCpp, SessionPhasePermitLikeCpp, SessionPhaseRequestLikeCpp,
    SessionPhaseRevokeLikeCpp,
};

/// What one step's world phase achieved.
#[derive(Debug, Clone, Default)]
pub(crate) struct WorldSessionPassSummaryLikeCpp {
    pub(crate) participants: usize,
    pub(crate) completed: usize,
    pub(crate) dispatched: usize,
    pub(crate) send_failed: usize,
    pub(crate) revoked_before_start: usize,
    /// Passes claimed whose end this step could not observe. The step must not
    /// continue into the map phase over them.
    pub(crate) unresolved_after_start: usize,
    pub(crate) stalled_ms: u64,
    /// Permits of passes whose effects were never accounted for; the producer
    /// holds its barrier until every one reaches a terminal state.
    pub(crate) unresolved_permits: Vec<Arc<SessionPhasePermitLikeCpp>>,
}

impl WorldSessionPassSummaryLikeCpp {
    pub(crate) fn quiescent_like_cpp(&self) -> bool {
        self.unresolved_after_start == 0 && self.unresolved_permits.is_empty()
    }
}

/// Drive the world phase of every session that can answer, serially.
///
/// The caller must hold no map or persistence guard: this both delivers and
/// awaits.
pub(crate) async fn run_world_phase_session_passes_like_cpp(
    participants: &[flume::Sender<SessionPhaseRequestLikeCpp>],
    coordinator_id: u64,
    tick_epoch: u64,
    diff_ms: u32,
    ack_timeout: Duration,
) -> WorldSessionPassSummaryLikeCpp {
    let mut summary = WorldSessionPassSummaryLikeCpp::default();

    for phase_tx in participants {
        summary.participants = summary.participants.saturating_add(1);

        let permit = SessionPhasePermitLikeCpp::new_like_cpp();
        let (response_tx, response_rx) = flume::bounded(1);
        let request = SessionPhaseRequestLikeCpp::World(RunWorldPhasePassLikeCppRequest {
            coordinator_id,
            tick_epoch,
            diff_ms,
            permit: Arc::clone(&permit),
            response_tx,
        });
        if phase_tx.try_send(request).is_err() {
            // The rail is full or closed, so the request never reached the
            // session; revoking makes that permanent rather than leaving a
            // permit that a late delivery could still claim.
            permit.revoke_before_start_like_cpp();
            summary.send_failed = summary.send_failed.saturating_add(1);
            continue;
        }

        await_one_world_pass_like_cpp(
            &mut summary,
            coordinator_id,
            tick_epoch,
            &permit,
            &response_rx,
            ack_timeout,
        )
        .await;

        if !summary.quiescent_like_cpp() {
            // The remaining sessions of this step would run beside an effect of
            // unknown extent. C++ drives them one after another on one thread,
            // so the step stops here and the producer holds its barrier.
            error!(
                tick_epoch,
                "Stopping the world-phase pass: an admitted pass left effects this step cannot account for"
            );
            return summary;
        }
    }

    summary
}

async fn await_one_world_pass_like_cpp(
    summary: &mut WorldSessionPassSummaryLikeCpp,
    coordinator_id: u64,
    tick_epoch: u64,
    permit: &Arc<SessionPhasePermitLikeCpp>,
    response_rx: &flume::Receiver<RunWorldPhasePassResultLikeCpp>,
    ack_timeout: Duration,
) {
    let started = Instant::now();
    let mut waited_past_deadline = false;

    loop {
        match tokio::time::timeout(ack_timeout, response_rx.recv_async()).await {
            Ok(Ok(result)) => {
                if result.coordinator_id != coordinator_id || result.tick_epoch != tick_epoch {
                    summary.unresolved_after_start =
                        summary.unresolved_after_start.saturating_add(1);
                    summary.unresolved_permits.push(Arc::clone(permit));
                    break;
                }
                match result.outcome {
                    SessionPhasePassOutcomeLikeCpp::Ran => {
                        summary.completed = summary.completed.saturating_add(1);
                        summary.dispatched = summary.dispatched.saturating_add(result.dispatched);
                    }
                    SessionPhasePassOutcomeLikeCpp::RevokedBeforeStart
                    | SessionPhasePassOutcomeLikeCpp::RefusedBeforeStart => {
                        summary.revoked_before_start =
                            summary.revoked_before_start.saturating_add(1);
                    }
                    SessionPhasePassOutcomeLikeCpp::AlreadyResolved => {
                        summary.unresolved_after_start =
                            summary.unresolved_after_start.saturating_add(1);
                        summary.unresolved_permits.push(Arc::clone(permit));
                    }
                }
                break;
            }
            Ok(Err(_)) | Err(_) => {
                match permit.revoke_before_start_like_cpp() {
                    SessionPhaseRevokeLikeCpp::Revoked => {
                        summary.revoked_before_start =
                            summary.revoked_before_start.saturating_add(1);
                        break;
                    }
                    SessionPhaseRevokeLikeCpp::AlreadyResolved(state) => {
                        use wow_world::session::mailbox::SessionPhasePermitStateLikeCpp as State;
                        match state {
                            State::Completed => {
                                summary.completed = summary.completed.saturating_add(1);
                            }
                            State::RevokedBeforeStart | State::RefusedBeforeStart => {
                                summary.revoked_before_start =
                                    summary.revoked_before_start.saturating_add(1);
                            }
                            State::InterruptedAfterStart | State::Running | State::Pending => {
                                summary.unresolved_after_start =
                                    summary.unresolved_after_start.saturating_add(1);
                                summary.unresolved_permits.push(Arc::clone(permit));
                                error!(
                                    tick_epoch,
                                    ?state,
                                    "A world-phase pass left no proof that its effects ended"
                                );
                            }
                        }
                        break;
                    }
                    SessionPhaseRevokeLikeCpp::AlreadyRunning => {
                        if response_rx.is_disconnected() && response_rx.is_empty() {
                            // The session task is gone while its pass was
                            // claimed: nothing will ever report its end.
                            summary.unresolved_after_start =
                                summary.unresolved_after_start.saturating_add(1);
                            summary.unresolved_permits.push(Arc::clone(permit));
                            error!(
                                tick_epoch,
                                "A claimed world-phase pass ended without reporting"
                            );
                            break;
                        }
                        waited_past_deadline = true;
                        warn!(
                            tick_epoch,
                            waiting_ms =
                                u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX),
                            "A world-phase pass is past its deadline and still running; the step waits for its completion boundary"
                        );
                    }
                }
            }
        }
    }

    if waited_past_deadline {
        let past_deadline = started.elapsed().saturating_sub(ack_timeout);
        summary.stalled_ms = summary
            .stalled_ms
            .saturating_add(u64::try_from(past_deadline.as_millis()).unwrap_or(u64::MAX));
    }
}
