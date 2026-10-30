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
    MapPhaseAdmissionLikeCpp, PlayerRecoveryStatus, RunMapPhasePassLikeCppCommand,
    RunPlayerRecoveryRequest, SessionPhasePassOutcomeLikeCpp, SessionPhasePermitLikeCpp,
    SessionPhaseRequestLikeCpp, SessionPhaseRevokeLikeCpp,
};

/// What one tick's session phase actually achieved.
#[derive(Debug, Clone, Default)]
pub(crate) struct MapSessionPassSummaryLikeCpp {
    /// Session passes the tick intended to drive, in producer admission order.
    pub(crate) participants: usize,
    /// Requests the control channel accepted.
    pub(crate) requested: usize,
    /// Passes that reached a completed boundary.
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
    /// At least one completed Player pass found the session unavailable, so
    /// this tick's Creature/P3 tail must be abandoned after the pass barrier.
    pub(crate) blocks_tick_tail: bool,
    /// How long the coordinator waited past its deadline for a claimed pass.
    pub(crate) stalled_ms: u64,
    /// The permits of the passes whose effects were never accounted for.
    ///
    /// They are carried out of the pass because the producer may not resume,
    /// may not drive any further session, and may not start another tick until
    /// every one of them reaches a terminal state: an operation whose effects
    /// are unknown is not made safe by moving on from it.
    pub(crate) unresolved_permits: Vec<Arc<SessionPhasePermitLikeCpp>>,
}

impl MapSessionPassSummaryLikeCpp {
    /// Whether every admitted pass reached a state with no live effect.
    ///
    /// Only then may the remaining phases of the tick run.
    pub(crate) fn quiescent_like_cpp(&self) -> bool {
        self.unresolved_after_start == 0 && self.unresolved_permits.is_empty()
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
                |reply: &wow_world::session::mailbox::RunMapPhasePassResultLikeCpp| {
                    Some(PassReply {
                        tick_epoch: reply.tick_epoch,
                        outcome: reply.outcome,
                        dispatched: reply.dispatched,
                        stopped_at_ineligible_head: reply.stopped_at_ineligible_head,
                        blocks_tick_tail: false,
                    })
                },
            )
            .await;

            if !summary.quiescent_like_cpp() {
                // C++ never reaches the next player of a map while the current
                // one's session is still inside `Map::Update`. With an effect
                // of unknown extent live, every later participant of this tick
                // would run beside it, so the pass stops here and the producer
                // holds the barrier.
                error!(
                    map_id = map_key.map_id,
                    instance_id = map_key.instance_id,
                    tick_epoch,
                    remaining_participants = admitted.len(),
                    "Stopping the map-phase pass: an admitted pass left effects this tick cannot account for"
                );
                return summary;
            }
        }
    }

    summary
}

/// Run the admitted Player timer passes serially within each Map, after its
/// respawn prelude and before the Map tick continuation.
#[allow(clippy::too_many_arguments)]
pub(crate) async fn run_player_recovery_passes(
    participants: &[CanonicalMapSessionPassMapLikeCpp],
    player_registry: &PlayerRegistry,
    coordinator_id: u64,
    tick_epoch: u64,
    map_tick_epoch: u64,
    diff_ms: u32,
    unix_time_ns: u128,
    ack_timeout: Duration,
) -> MapSessionPassSummaryLikeCpp {
    let mut summary = MapSessionPassSummaryLikeCpp::default();

    for admitted_map in participants {
        for participant in &admitted_map.participants {
            summary.participants = summary.participants.saturating_add(1);
            let Some(address) = player_registry.session_phase_address_like_cpp(participant.guid)
            else {
                summary.unaddressable = summary.unaddressable.saturating_add(1);
                continue;
            };

            let permit = SessionPhasePermitLikeCpp::new_like_cpp();
            let (response_tx, response_rx) = flume::bounded(1);
            let request = SessionPhaseRequestLikeCpp::PlayerRecovery(RunPlayerRecoveryRequest {
                admission: wow_world::session::mailbox::PlayerRecoveryAdmission {
                    coordinator_id,
                    tick_epoch,
                    map_tick_epoch,
                    registration: address.registration(),
                    handle: participant.handle,
                    map_key: admitted_map.key,
                    map_incarnation: admitted_map.incarnation,
                    residence_revision: participant.residence_revision,
                    diff_ms,
                    unix_time_ns,
                },
                permit: Arc::clone(&permit),
                response_tx,
            });
            if address.try_send(request).is_err() {
                permit.revoke_before_start_like_cpp();
                summary.send_failed = summary.send_failed.saturating_add(1);
                continue;
            }
            summary.requested = summary.requested.saturating_add(1);

            let reply = await_one_admitted_pass_like_cpp(
                &mut summary,
                admitted_map.key,
                tick_epoch,
                &permit,
                &response_rx,
                ack_timeout,
                |reply: &wow_world::session::mailbox::PlayerRecoveryReply| {
                    if reply.coordinator_id != coordinator_id
                        || reply.tick_epoch != tick_epoch
                        || reply.map_tick_epoch != map_tick_epoch
                        || (reply.outcome == SessionPhasePassOutcomeLikeCpp::Ran
                            && reply.status.is_none())
                        || (reply.outcome != SessionPhasePassOutcomeLikeCpp::Ran
                            && reply.status.is_some())
                    {
                        return None;
                    }
                    Some(PassReply {
                        tick_epoch: reply.tick_epoch,
                        outcome: reply.outcome,
                        dispatched: 0,
                        stopped_at_ineligible_head: false,
                        blocks_tick_tail: reply.status == Some(PlayerRecoveryStatus::Unavailable),
                    })
                },
            )
            .await;
            if std::env::var_os("RUSTYCORE_DEATH_TIMER_TRACE").is_some() {
                if let Some(reply) = reply {
                    if reply.status != Some(PlayerRecoveryStatus::Completed) {
                        tracing::info!(
                            map_id = admitted_map.key.map_id,
                            instance_id = admitted_map.key.instance_id,
                            tick_epoch,
                            map_tick_epoch,
                            diff_ms,
                            status = ?reply.status,
                            outcome = ?reply.outcome,
                            "RUST_DEATH_TIMER player_recovery_reply"
                        );
                    }
                }
            }

            if !summary.quiescent_like_cpp() {
                error!(
                    map_id = admitted_map.key.map_id,
                    instance_id = admitted_map.key.instance_id,
                    tick_epoch,
                    remaining_participants = admitted_map.participants.len(),
                    "Stopping Player recovery passes: an admitted pass left effects this tick cannot account for"
                );
                return summary;
            }
            if summary.blocks_tick_tail {
                warn!(
                    map_id = admitted_map.key.map_id,
                    instance_id = admitted_map.key.instance_id,
                    tick_epoch,
                    "Stopping Player recovery passes after a terminal Unavailable result"
                );
                return summary;
            }
        }
    }

    summary
}

#[derive(Clone, Copy)]
pub(super) struct PassReply {
    pub(super) tick_epoch: u64,
    pub(super) outcome: SessionPhasePassOutcomeLikeCpp,
    pub(super) dispatched: usize,
    pub(super) stopped_at_ineligible_head: bool,
    pub(super) blocks_tick_tail: bool,
}

/// Wait for one admitted pass to reach a state in which it has no live effect.
pub(super) async fn await_one_admitted_pass_like_cpp<T, Project>(
    summary: &mut MapSessionPassSummaryLikeCpp,
    map_key: wow_map::MapKey,
    tick_epoch: u64,
    permit: &Arc<SessionPhasePermitLikeCpp>,
    response_rx: &flume::Receiver<T>,
    ack_timeout: Duration,
    project: Project,
) -> Option<T>
where
    Project: Fn(&T) -> Option<PassReply>,
{
    let started = Instant::now();
    let mut waited_past_deadline = false;
    let mut reported = None;

    loop {
        match tokio::time::timeout(ack_timeout, response_rx.recv_async()).await {
            Ok(Ok(result)) => {
                if let Some(result) = project(&result) {
                    record_reported_outcome_like_cpp(summary, map_key, tick_epoch, permit, result);
                } else {
                    reject_untrusted_outcome(summary, map_key, tick_epoch, permit);
                }
                reported = Some(result);
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
                        summary.unresolved_permits.push(Arc::clone(permit));
                        error!(
                            map_id = map_key.map_id,
                            instance_id = map_key.instance_id,
                            tick_epoch,
                            "A claimed map-phase pass ended without reporting; the tick cannot resume over live effects"
                        );
                    }
                    SessionPhaseRevokeLikeCpp::AlreadyResolved(state) => {
                        record_resolved_state_like_cpp(summary, map_key, tick_epoch, permit, state);
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
                        record_resolved_state_like_cpp(summary, map_key, tick_epoch, permit, state);
                        break;
                    }
                    SessionPhaseRevokeLikeCpp::AlreadyRunning => {
                        // Diagnostic only: the pass owns the session and is
                        // mutating. Keep waiting for its real boundary, and say
                        // so on every deadline: a wait that never ends must not
                        // be visible only as one line at its start.
                        waited_past_deadline = true;
                        warn!(
                            map_id = map_key.map_id,
                            instance_id = map_key.instance_id,
                            tick_epoch,
                            waiting_ms =
                                u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX),
                            "A map-phase pass is past its deadline and still running; the tick waits for its completion boundary"
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
    reported
}

fn record_reported_outcome_like_cpp(
    summary: &mut MapSessionPassSummaryLikeCpp,
    map_key: wow_map::MapKey,
    tick_epoch: u64,
    permit: &Arc<SessionPhasePermitLikeCpp>,
    result: PassReply,
) {
    if result.tick_epoch != tick_epoch {
        // A report for another tick says nothing about this one, and the
        // request this coordinator sent has not been accounted for. Revoke a
        // still-pending delivery before abandoning this plan.
        reject_untrusted_outcome(summary, map_key, tick_epoch, permit);
        return;
    }
    use wow_world::session::mailbox::SessionPhasePermitStateLikeCpp as State;
    match permit.state_like_cpp() {
        State::Completed => {
            summary.completed = summary.completed.saturating_add(1);
            if result.outcome == SessionPhasePassOutcomeLikeCpp::Ran {
                summary.dispatched = summary.dispatched.saturating_add(result.dispatched);
                if result.stopped_at_ineligible_head {
                    summary.stopped_at_ineligible_head =
                        summary.stopped_at_ineligible_head.saturating_add(1);
                }
                summary.blocks_tick_tail |= result.blocks_tick_tail;
            }
        }
        State::CompletedUnavailable => {
            summary.completed = summary.completed.saturating_add(1);
            summary.blocks_tick_tail = true;
        }
        State::RevokedBeforeStart => {
            summary.revoked_before_start = summary.revoked_before_start.saturating_add(1);
        }
        State::RefusedBeforeStart => {
            summary.refused_before_start = summary.refused_before_start.saturating_add(1);
        }
        State::Pending => match permit.revoke_before_start_like_cpp() {
            SessionPhaseRevokeLikeCpp::Revoked => {
                summary.revoked_before_start = summary.revoked_before_start.saturating_add(1);
            }
            SessionPhaseRevokeLikeCpp::AlreadyRunning => {
                retain_unresolved_permit(summary, map_key, tick_epoch, permit);
            }
            SessionPhaseRevokeLikeCpp::AlreadyResolved(state) => {
                record_resolved_state_like_cpp(summary, map_key, tick_epoch, permit, state);
            }
        },
        State::Running | State::InterruptedAfterStart => {
            retain_unresolved_permit(summary, map_key, tick_epoch, permit);
        }
    }
}

fn reject_untrusted_outcome(
    summary: &mut MapSessionPassSummaryLikeCpp,
    map_key: wow_map::MapKey,
    tick_epoch: u64,
    permit: &Arc<SessionPhasePermitLikeCpp>,
) {
    use wow_world::session::mailbox::{
        SessionPhasePermitStateLikeCpp as State, SessionPhaseRevokeLikeCpp as Revoke,
    };
    match permit.state_like_cpp() {
        State::Pending => match permit.revoke_before_start_like_cpp() {
            Revoke::Revoked => {
                summary.revoked_before_start = summary.revoked_before_start.saturating_add(1);
                summary.blocks_tick_tail = true;
            }
            Revoke::AlreadyRunning => {
                retain_unresolved_permit(summary, map_key, tick_epoch, permit)
            }
            Revoke::AlreadyResolved(state) => {
                record_resolved_state_like_cpp(summary, map_key, tick_epoch, permit, state);
                summary.blocks_tick_tail = true;
            }
        },
        State::Running | State::InterruptedAfterStart => {
            retain_unresolved_permit(summary, map_key, tick_epoch, permit);
        }
        State::Completed
        | State::CompletedUnavailable
        | State::RevokedBeforeStart
        | State::RefusedBeforeStart => {
            record_resolved_state_like_cpp(
                summary,
                map_key,
                tick_epoch,
                permit,
                permit.state_like_cpp(),
            );
            summary.blocks_tick_tail = true;
        }
    }
}

fn retain_unresolved_permit(
    summary: &mut MapSessionPassSummaryLikeCpp,
    map_key: wow_map::MapKey,
    tick_epoch: u64,
    permit: &Arc<SessionPhasePermitLikeCpp>,
) {
    summary.unresolved_after_start = summary.unresolved_after_start.saturating_add(1);
    summary.unresolved_permits.push(Arc::clone(permit));
    error!(
        map_id = map_key.map_id,
        instance_id = map_key.instance_id,
        tick_epoch,
        state = ?permit.state_like_cpp(),
        "A session phase reply arrived before its shared permit reached a terminal boundary"
    );
}

fn record_resolved_state_like_cpp(
    summary: &mut MapSessionPassSummaryLikeCpp,
    map_key: wow_map::MapKey,
    tick_epoch: u64,
    permit: &Arc<SessionPhasePermitLikeCpp>,
    state: wow_world::session::mailbox::SessionPhasePermitStateLikeCpp,
) {
    use wow_world::session::mailbox::SessionPhasePermitStateLikeCpp as State;
    match state {
        State::Completed => summary.completed = summary.completed.saturating_add(1),
        State::CompletedUnavailable => {
            summary.completed = summary.completed.saturating_add(1);
            summary.blocks_tick_tail = true;
        }
        State::RevokedBeforeStart => {
            summary.revoked_before_start = summary.revoked_before_start.saturating_add(1);
        }
        State::RefusedBeforeStart => {
            summary.refused_before_start = summary.refused_before_start.saturating_add(1);
        }
        State::InterruptedAfterStart | State::Running | State::Pending => {
            summary.unresolved_after_start = summary.unresolved_after_start.saturating_add(1);
            summary.unresolved_permits.push(Arc::clone(permit));
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
