// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! The session side of one admitted map-phase pass (#787).
//!
//! C++ drives this pass from `Map::Update` (`Maps/Map.cpp:669-680`) while the
//! map thread owns the session, so nothing can change the player, its
//! incarnation, its map or its residence between the decision and the pass.
//! RustyCore releases every guard before delivering the request, so this module
//! does what the C++ guard did implicitly: it revalidates the frozen admission
//! and takes the shared permit **before the first effect**, then runs the same
//! tail C++ runs for `MapSessionFilter` (`Server/WorldSession.cpp:488-506`).

use wow_handler::PacketUpdatePhase;

use super::super::{SessionHandlerCatalogsLikeCpp, SessionState, WorldSession};
use crate::session::mailbox::{
    MapPhaseAdmissionLikeCpp, RunMapPhasePassLikeCppCommand, RunMapPhasePassResultLikeCpp,
    RunWorldPhasePassLikeCppRequest, RunWorldPhasePassResultLikeCpp, SessionPhaseClaimLikeCpp,
    SessionPhasePassOutcomeLikeCpp, SessionPhaseRequestLikeCpp,
};

impl WorldSession {
    /// Run one admitted map-phase pass and report its completion boundary.
    ///
    /// The result is produced after the dispatch **and** its tails, never on
    /// observing the command: an acknowledgement of receipt would let the tick
    /// resume while this session is still mutating.
    pub(crate) async fn run_admitted_map_phase_pass_like_cpp(
        &mut self,
        command: RunMapPhasePassLikeCppCommand,
        catalogs: &SessionHandlerCatalogsLikeCpp,
    ) -> RunMapPhasePassResultLikeCpp {
        let admission = command.admission;

        if !self.phase_authority_is_current_like_cpp(
            PacketUpdatePhase::Map,
            admission.coordinator_id,
            admission.tick_epoch,
        ) || !self.admission_still_describes_this_session_like_cpp(&admission)
        {
            command.permit.refuse_before_start_like_cpp();
            return RunMapPhasePassResultLikeCpp {
                tick_epoch: admission.tick_epoch,
                outcome: SessionPhasePassOutcomeLikeCpp::RefusedBeforeStart,
                dispatched: 0,
                stopped_at_ineligible_head: false,
                disconnecting: self.is_disconnecting(),
            };
        }

        // One atomic transition decides between this pass starting and the
        // coordinator revoking it. Reading a flag here and dispatching below
        // would leave exactly the window this permit exists to close.
        match command.permit.claim_like_cpp() {
            SessionPhaseClaimLikeCpp::Claimed => {}
            SessionPhaseClaimLikeCpp::Revoked => {
                return RunMapPhasePassResultLikeCpp {
                    tick_epoch: admission.tick_epoch,
                    outcome: SessionPhasePassOutcomeLikeCpp::RevokedBeforeStart,
                    dispatched: 0,
                    stopped_at_ineligible_head: false,
                    disconnecting: self.is_disconnecting(),
                };
            }
            SessionPhaseClaimLikeCpp::AlreadyResolved(_) => {
                return RunMapPhasePassResultLikeCpp {
                    tick_epoch: admission.tick_epoch,
                    outcome: SessionPhasePassOutcomeLikeCpp::AlreadyResolved,
                    dispatched: 0,
                    stopped_at_ineligible_head: false,
                    disconnecting: self.is_disconnecting(),
                };
            }
        }

        self.mark_map_phase_coordinated_like_cpp();
        let summary = self
            .run_phase_packet_pass_like_cpp(PacketUpdatePhase::Map, catalogs)
            .await;
        self.run_map_phase_tail_like_cpp(admission.diff_ms);

        command.permit.complete_like_cpp();
        RunMapPhasePassResultLikeCpp {
            tick_epoch: admission.tick_epoch,
            outcome: SessionPhasePassOutcomeLikeCpp::Ran,
            dispatched: summary.dispatched,
            stopped_at_ineligible_head: summary.stopped_at_ineligible_head,
            disconnecting: self.is_disconnecting(),
        }
    }

    /// C++ `WorldSession::Update` logout decision, on the `ProcessUnsafe()`
    /// branch reserved for the world filter (`WorldSession.cpp:498-503`).
    pub(crate) fn run_logout_timer_like_cpp(&mut self) {
        let Some(logout_time) = self.logout_time else {
            return;
        };
        self.record_driver_phase_like_cpp(
            crate::session::driver::phases::SessionDriverPhaseLikeCpp::LogoutTimer,
        );
        if std::time::Instant::now() >= logout_time {
            self.logout_time = None;
            self.complete_logout();
        }
    }

    /// C++ `WorldSession::Update` after the packet loop, on the branch
    /// `!updater.ProcessUnsafe()` — the map filter's
    /// (`Server/WorldSession.cpp:488-506`): the periodic time sync runs with
    /// that pass's diff, then the query callbacks. Neither depends on a packet
    /// having been dispatched, so both run for an empty pass too.
    pub(crate) fn run_map_phase_tail_like_cpp(&mut self, diff_ms: u32) {
        if self.state == SessionState::LoggedIn && self.time_sync_timer_ms > 0 {
            if diff_ms >= self.time_sync_timer_ms {
                self.send_time_sync();
            } else {
                self.time_sync_timer_ms -= diff_ms;
            }
        }
        self.process_ready_character_rename_callbacks_like_cpp();
    }

    /// Freeze this session's current identity the way the canonical tick does
    /// at the split, for regressions that then change one fact and observe the
    /// refusal.
    #[cfg(test)]
    pub(crate) fn current_map_phase_admission_for_test_like_cpp(
        &self,
        coordinator_id: u64,
        tick_epoch: u64,
        diff_ms: u32,
    ) -> Option<MapPhaseAdmissionLikeCpp> {
        let guid = self.player_guid()?;
        let registration = self
            .player_registry()?
            .session_phase_address_like_cpp(guid)?
            .registration();
        let handle = self.player_handle_like_cpp?;
        let manager = self.canonical_map_manager.as_ref()?;
        let manager = manager.lock().ok()?;
        let (map_key, residence_revision) =
            manager.player_active_residence_revision_like_cpp(handle)?;
        Some(MapPhaseAdmissionLikeCpp {
            coordinator_id,
            tick_epoch,
            phase: PacketUpdatePhase::Map,
            registration,
            handle,
            map_key,
            map_incarnation: manager.map_incarnation_like_cpp(map_key)?,
            residence_revision,
            diff_ms,
        })
    }

    /// Whether the identity frozen at admission still describes this session.
    ///
    /// Every check answers a way the world can have moved while no guard was
    /// held: another session took the character, the map replaced the Player,
    /// the player left, transferred, or left and came back to the same map.
    fn admission_still_describes_this_session_like_cpp(
        &self,
        admission: &MapPhaseAdmissionLikeCpp,
    ) -> bool {
        if admission.phase != PacketUpdatePhase::Map {
            return false;
        }

        // The directory incarnation: a `WorldSession` outlives a character, so
        // the same control channel can belong to a later registration.
        let Some(guid) = self.player_guid() else {
            return false;
        };
        if admission.registration.guid() != guid {
            return false;
        }
        let Some(registry) = self.player_registry() else {
            return false;
        };
        if registry.lookup_current(admission.registration).is_none() {
            return false;
        }

        // The map-owned incarnation, a separate generation space.
        if self.player_handle_like_cpp != Some(admission.handle) {
            return false;
        }

        // Residence, map incarnation and revision, observed now rather than
        // trusted from the request. A→B→A is caught by the revision even though
        // the key matches again.
        let Some(manager) = self.canonical_map_manager.as_ref() else {
            return false;
        };
        let Ok(manager) = manager.lock() else {
            return false;
        };
        if manager.map_incarnation_like_cpp(admission.map_key) != Some(admission.map_incarnation) {
            return false;
        }
        manager.player_active_residence_revision_like_cpp(admission.handle)
            == Some((admission.map_key, admission.residence_revision))
    }
}

impl WorldSession {
    /// The rail the canonical producer addresses this session's phases through.
    #[must_use]
    pub fn session_phase_sender_like_cpp(&self) -> flume::Sender<SessionPhaseRequestLikeCpp> {
        self.session_phase_tx.clone()
    }

    /// A receiving handle for the same rail, for the task that owns this
    /// session and parks on it between phases.
    #[must_use]
    pub fn session_phase_receiver_like_cpp(&self) -> flume::Receiver<SessionPhaseRequestLikeCpp> {
        self.session_phase_rx.clone()
    }

    /// Whether a request comes from the authority this session is serving.
    ///
    /// A later producer replaces an earlier one, but an earlier producer never
    /// comes back, and no step of a producer is served twice: the permit is
    /// new on a replay, so only this watermark can refuse it.
    fn phase_authority_is_current_like_cpp(
        &mut self,
        phase: PacketUpdatePhase,
        coordinator_id: u64,
        tick_epoch: u64,
    ) -> bool {
        let slot = match phase {
            PacketUpdatePhase::World => 0,
            PacketUpdatePhase::Map => 1,
        };
        let accepted = match self.last_phase_authority_like_cpp[slot] {
            Some((last_coordinator, _)) if coordinator_id < last_coordinator => false,
            Some((last_coordinator, last_epoch))
                if coordinator_id == last_coordinator && tick_epoch <= last_epoch =>
            {
                false
            }
            _ => true,
        };
        if accepted {
            self.last_phase_authority_like_cpp[slot] = Some((coordinator_id, tick_epoch));
        }
        accepted
    }

    /// Refuse every phase request already on the rail, before any effect.
    ///
    /// Used at the handover to shutdown: from that point this session serves no
    /// phase, and a refusal is the answer that leaves the producer nothing to
    /// wait for and nothing half-done. Returns how many were refused.
    pub fn refuse_pending_phase_requests_like_cpp(&mut self) -> usize {
        let mut refused = 0;
        while let Ok(request) = self.session_phase_rx.try_recv() {
            refused += 1;
            match request {
                SessionPhaseRequestLikeCpp::World(request) => {
                    request.permit.refuse_before_start_like_cpp();
                    let _ = request
                        .response_tx
                        .try_send(RunWorldPhasePassResultLikeCpp {
                            coordinator_id: request.coordinator_id,
                            tick_epoch: request.tick_epoch,
                            outcome: SessionPhasePassOutcomeLikeCpp::RefusedBeforeStart,
                            dispatched: 0,
                            disconnecting: self.is_disconnecting(),
                        });
                }
                SessionPhaseRequestLikeCpp::Map(command) => {
                    command.permit.refuse_before_start_like_cpp();
                    let _ = command.response_tx.try_send(RunMapPhasePassResultLikeCpp {
                        tick_epoch: command.admission.tick_epoch,
                        outcome: SessionPhasePassOutcomeLikeCpp::RefusedBeforeStart,
                        dispatched: 0,
                        stopped_at_ineligible_head: false,
                        disconnecting: self.is_disconnecting(),
                    });
                }
            }
        }
        refused
    }

    /// Run one requested phase and report its completion boundary.
    pub async fn run_requested_session_phase_like_cpp(
        &mut self,
        request: SessionPhaseRequestLikeCpp,
        catalogs: &SessionHandlerCatalogsLikeCpp,
    ) {
        match request {
            SessionPhaseRequestLikeCpp::World(request) => {
                let response_tx = request.response_tx.clone();
                let result = self
                    .run_admitted_world_phase_pass_like_cpp(request, catalogs)
                    .await;
                let _ = response_tx.try_send(result);
            }
            SessionPhaseRequestLikeCpp::Map(command) => {
                let response_tx = command.response_tx.clone();
                let result = self
                    .run_admitted_map_phase_pass_like_cpp(command, catalogs)
                    .await;
                let _ = response_tx.try_send(result);
            }
        }
    }

    /// Run one admitted world-phase pass: C++ `World::UpdateSessions`
    /// (`World.cpp:2704`, `World.cpp:3394-3420`) with `WorldSessionFilter`.
    ///
    /// This is the whole of what the session used to do on its own clock, now
    /// driven by the producer's diff: the periodic session work, the mailbox
    /// drain, the world-eligible packet dispatch and the world tail. The
    /// map-eligible head stays queued for the map pass, exactly as C++ splits
    /// them (`WorldSession.cpp:64-107`).
    pub(crate) async fn run_admitted_world_phase_pass_like_cpp(
        &mut self,
        request: RunWorldPhasePassLikeCppRequest,
        catalogs: &SessionHandlerCatalogsLikeCpp,
    ) -> RunWorldPhasePassResultLikeCpp {
        if !self.phase_authority_is_current_like_cpp(
            PacketUpdatePhase::World,
            request.coordinator_id,
            request.tick_epoch,
        ) {
            request.permit.refuse_before_start_like_cpp();
            return RunWorldPhasePassResultLikeCpp {
                coordinator_id: request.coordinator_id,
                tick_epoch: request.tick_epoch,
                outcome: SessionPhasePassOutcomeLikeCpp::RefusedBeforeStart,
                dispatched: 0,
                disconnecting: self.is_disconnecting(),
            };
        }
        match request.permit.claim_like_cpp() {
            SessionPhaseClaimLikeCpp::Claimed => {}
            SessionPhaseClaimLikeCpp::Revoked => {
                return RunWorldPhasePassResultLikeCpp {
                    coordinator_id: request.coordinator_id,
                    tick_epoch: request.tick_epoch,
                    outcome: SessionPhasePassOutcomeLikeCpp::RevokedBeforeStart,
                    dispatched: 0,
                    disconnecting: self.is_disconnecting(),
                };
            }
            SessionPhaseClaimLikeCpp::AlreadyResolved(_) => {
                return RunWorldPhasePassResultLikeCpp {
                    coordinator_id: request.coordinator_id,
                    tick_epoch: request.tick_epoch,
                    outcome: SessionPhasePassOutcomeLikeCpp::AlreadyResolved,
                    dispatched: 0,
                    disconnecting: self.is_disconnecting(),
                };
            }
        }

        self.mark_map_phase_coordinated_like_cpp();
        let dispatched = self
            .update_with_catalogs_like_cpp(request.diff_ms, catalogs)
            .await;
        self.process_pending_with_catalogs_like_cpp(catalogs).await;
        // C++ `WorldSession::Update` decides the logout after the packet loop
        // and the query callbacks, on the `ProcessUnsafe()` branch
        // (`WorldSession.cpp:498-503`): a `LogoutCancel` queued in this same
        // pass is dispatched above and must be seen before the decision.
        self.run_logout_timer_like_cpp();

        request.permit.complete_like_cpp();
        RunWorldPhasePassResultLikeCpp {
            coordinator_id: request.coordinator_id,
            tick_epoch: request.tick_epoch,
            outcome: SessionPhasePassOutcomeLikeCpp::Ran,
            dispatched,
            disconnecting: self.is_disconnecting(),
        }
    }
}
