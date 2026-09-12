// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! The session phase of one split canonical map tick (#787).
//!
//! C++ drives the sessions of a map's players from inside `Map::Update`
//! (`Maps/Map.cpp:669-680`) because the map thread owns them. RustyCore keeps
//! each session in its own task, so the canonical tick releases every
//! synchronous guard at the split, asks each admitted session to run its map
//! pass, and waits for the completion boundary before resuming the tick.
//!
//! The deadline here is not a licence to continue. Each request carries a
//! permit whose single atomic transition decides between the session starting
//! the pass and this coordinator revoking it. When the revocation wins, the
//! pass provably never ran and the tick continues; when it loses, the session
//! is mutating right now and the tick keeps waiting, because continuing would
//! mean the next session of the same map, and then the phases that follow,
//! overlapping live effects C++ never overlaps.
//!
//! The stated limit of that choice: an admitted operation that can wait
//! indefinitely makes strict order, absence of overlap and bounded tick
//! progress mutually exclusive. This runner keeps the first two and reports the
//! third as a stall instead of pretending the pass ended.

use std::sync::Arc;
use std::time::{Duration, Instant};

use tracing::{error, warn};

use super::map_tick::CanonicalMapSessionPassMapLikeCpp;
use wow_world::session::directory::PlayerRegistry;
use wow_world::session::mailbox::{
    MapPhaseAdmissionLikeCpp, RunMapPhasePassLikeCppCommand, SessionPhasePassOutcomeLikeCpp,
    SessionPhasePermitLikeCpp, SessionPhaseRequestLikeCpp, SessionPhaseRevokeLikeCpp,
};

/// What one tick's session phase actually achieved.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct MapSessionPassSummaryLikeCpp {
    /// Sessions the tick intended to drive, in map membership order.
    pub(crate) participants: usize,
    /// Requests the control channel accepted.
    pub(crate) requested: usize,
    /// Sessions that reported a completed map pass for this tick.
    pub(crate) completed: usize,
    /// Packets those passes dispatched.
    pub(crate) dispatched: usize,
    /// Sessions whose head was ineligible for the map phase, which C++ leaves
    /// queued for `World::UpdateSessions`.
    pub(crate) stopped_at_ineligible_head: usize,
    /// Sessions with no current control address: replaced, gone, or never
    /// registered. Not a failure of this tick.
    pub(crate) unaddressable: usize,
    /// Requests the bounded control channel refused.
    pub(crate) send_failed: usize,
    /// Passes this coordinator revoked before the session could start them.
    /// Nothing of those requests ran, so the tick may continue.
    pub(crate) revoked_before_start: usize,
    /// Passes the session declined because the frozen admission no longer
    /// described it. Also effect-free.
    pub(crate) refused_before_start: usize,
    /// Passes that were claimed and whose end this tick could not observe. The
    /// tick may not resume: mutations of this map may still be running.
    pub(crate) unresolved_after_start: usize,
    /// How long the coordinator waited past its deadline for a claimed pass.
    pub(crate) stalled_ms: u64,
}

impl MapSessionPassSummaryLikeCpp {
    /// Whether every admitted pass reached a state with no live effect.
    ///
    /// Only then may the remaining phases of the tick run.
    pub(crate) const fn quiescent_like_cpp(&self) -> bool {
        self.unresolved_after_start == 0
    }
}

/// Ask every admitted session of every admitted map to run its map-phase pass,
/// serially within a map, and wait for each completion boundary.
///
/// The caller must hold no map, spawn-metadata or respawn-order guard: a
/// request is a delivery, and delivery under a map guard is the one thing this
/// design may never do.
pub(crate) async fn run_map_phase_session_passes_like_cpp(
    participants: &[CanonicalMapSessionPassMapLikeCpp],
    player_registry: &PlayerRegistry,
    coordinator_id: u64,
    tick_epoch: u64,
    diff_ms: u32,
    ack_timeout: Duration,
) -> MapSessionPassSummaryLikeCpp {
    let mut summary = MapSessionPassSummaryLikeCpp::default();

    for admitted_map in participants {
        let map_key = &admitted_map.key;
        let admitted = &admitted_map.participants;
        // C++ walks one map's players in `m_mapRefManager` order and runs their
        // sessions one after another inside that map's update; two sessions of
        // the same map never run concurrently there.
        for participant in admitted {
            summary.participants = summary.participants.saturating_add(1);

            let Some(address) = player_registry.session_phase_address_like_cpp(participant.guid)
            else {
                summary.unaddressable = summary.unaddressable.saturating_add(1);
                continue;
            };

            let permit = SessionPhasePermitLikeCpp::new_like_cpp();
            let (response_tx, response_rx) = flume::bounded(1);
            let request = SessionPhaseRequestLikeCpp::Map(RunMapPhasePassLikeCppCommand {
                admission: MapPhaseAdmissionLikeCpp {
                    coordinator_id,
                    tick_epoch,
                    phase: wow_handler::PacketUpdatePhase::Map,
                    registration: address.registration(),
                    handle: participant.handle,
                    map_key: *map_key,
                    map_incarnation: admitted_map.incarnation,
                    residence_revision: participant.residence_revision,
                    diff_ms,
                },
                permit: Arc::clone(&permit),
                response_tx,
            });
            if address.try_send(request).is_err() {
                // The request never entered the session's queue, so no permit
                // holder exists but this one; revoking keeps that permanent.
                permit.revoke_before_start_like_cpp();
                summary.send_failed = summary.send_failed.saturating_add(1);
                continue;
            }
            summary.requested = summary.requested.saturating_add(1);

            await_one_admitted_pass_like_cpp(
                &mut summary,
                *map_key,
                tick_epoch,
                &permit,
                &response_rx,
                ack_timeout,
            )
            .await;
        }
    }

    summary
}

/// Wait for one admitted pass to reach a state in which it has no live effect.
async fn await_one_admitted_pass_like_cpp(
    summary: &mut MapSessionPassSummaryLikeCpp,
    map_key: wow_map::MapKey,
    tick_epoch: u64,
    permit: &Arc<SessionPhasePermitLikeCpp>,
    response_rx: &flume::Receiver<wow_world::session::mailbox::RunMapPhasePassResultLikeCpp>,
    ack_timeout: Duration,
) {
    let started = Instant::now();
    let mut waited_past_deadline = false;

    loop {
        match tokio::time::timeout(ack_timeout, response_rx.recv_async()).await {
            Ok(Ok(result)) => {
                record_reported_outcome_like_cpp(summary, tick_epoch, &result);
                break;
            }
            Ok(Err(_disconnected)) => {
                // The session task ended without reporting. If it never claimed
                // the pass, this revocation proves nothing ran. If it did, the
                // pass stopped mid-way and this tick cannot treat the map as
                // quiescent, whatever the channel says.
                match permit.revoke_before_start_like_cpp() {
                    SessionPhaseRevokeLikeCpp::Revoked => {
                        summary.revoked_before_start =
                            summary.revoked_before_start.saturating_add(1);
                    }
                    SessionPhaseRevokeLikeCpp::AlreadyRunning => {
                        summary.unresolved_after_start =
                            summary.unresolved_after_start.saturating_add(1);
                        error!(
                            map_id = map_key.map_id,
                            instance_id = map_key.instance_id,
                            tick_epoch,
                            "A claimed map-phase pass ended without reporting; the tick cannot resume over live effects"
                        );
                    }
                    SessionPhaseRevokeLikeCpp::AlreadyResolved(state) => {
                        record_resolved_state_like_cpp(summary, map_key, tick_epoch, state);
                    }
                }
                break;
            }
            Err(_elapsed) => {
                // The deadline decides only while the pass is still unclaimed.
                match permit.revoke_before_start_like_cpp() {
                    SessionPhaseRevokeLikeCpp::Revoked => {
                        summary.revoked_before_start =
                            summary.revoked_before_start.saturating_add(1);
                        break;
                    }
                    SessionPhaseRevokeLikeCpp::AlreadyResolved(state) => {
                        record_resolved_state_like_cpp(summary, map_key, tick_epoch, state);
                        break;
                    }
                    SessionPhaseRevokeLikeCpp::AlreadyRunning => {
                        // Diagnostic only: the pass owns the session and is
                        // mutating. Keep waiting for its real boundary.
                        if !waited_past_deadline {
                            waited_past_deadline = true;
                            warn!(
                                map_id = map_key.map_id,
                                instance_id = map_key.instance_id,
                                tick_epoch,
                                "A map-phase pass passed its deadline while running; the tick waits for its completion boundary"
                            );
                        }
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

fn record_reported_outcome_like_cpp(
    summary: &mut MapSessionPassSummaryLikeCpp,
    tick_epoch: u64,
    result: &wow_world::session::mailbox::RunMapPhasePassResultLikeCpp,
) {
    if result.tick_epoch != tick_epoch {
        // A report for another tick says nothing about this one, and the
        // request this coordinator sent has not been accounted for.
        summary.unresolved_after_start = summary.unresolved_after_start.saturating_add(1);
        return;
    }
    match result.outcome {
        SessionPhasePassOutcomeLikeCpp::Ran => {
            summary.completed = summary.completed.saturating_add(1);
            summary.dispatched = summary.dispatched.saturating_add(result.dispatched);
            if result.stopped_at_ineligible_head {
                summary.stopped_at_ineligible_head =
                    summary.stopped_at_ineligible_head.saturating_add(1);
            }
        }
        SessionPhasePassOutcomeLikeCpp::RevokedBeforeStart => {
            summary.revoked_before_start = summary.revoked_before_start.saturating_add(1);
        }
        SessionPhasePassOutcomeLikeCpp::RefusedBeforeStart => {
            summary.refused_before_start = summary.refused_before_start.saturating_add(1);
        }
        SessionPhasePassOutcomeLikeCpp::AlreadyResolved => {
            summary.unresolved_after_start = summary.unresolved_after_start.saturating_add(1);
        }
    }
}

fn record_resolved_state_like_cpp(
    summary: &mut MapSessionPassSummaryLikeCpp,
    map_key: wow_map::MapKey,
    tick_epoch: u64,
    state: wow_world::session::mailbox::SessionPhasePermitStateLikeCpp,
) {
    use wow_world::session::mailbox::SessionPhasePermitStateLikeCpp as State;
    match state {
        State::Completed => summary.completed = summary.completed.saturating_add(1),
        State::RevokedBeforeStart => {
            summary.revoked_before_start = summary.revoked_before_start.saturating_add(1);
        }
        State::RefusedBeforeStart => {
            summary.refused_before_start = summary.refused_before_start.saturating_add(1);
        }
        State::InterruptedAfterStart | State::Running | State::Pending => {
            summary.unresolved_after_start = summary.unresolved_after_start.saturating_add(1);
            error!(
                map_id = map_key.map_id,
                instance_id = map_key.instance_id,
                tick_epoch,
                ?state,
                "A map-phase pass left no proof that its effects ended"
            );
        }
    }
}

#[cfg(test)]
#[path = "map_session_pass/tests.rs"]
mod tests;
