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
use wow_world_application::{ResurrectionOperationCatalogs, run_logout_timer_with_catalogs};

use super::super::{SessionHandlerCatalogsLikeCpp, SessionState, WorldSession};
use crate::session::mailbox::{
    MapPhaseAdmissionLikeCpp, PendingWorldPhaseFinalizationLikeCpp, PlayerRecoveryReply,
    RunMapPhasePassLikeCppCommand, RunMapPhasePassResultLikeCpp, RunPlayerRecoveryRequest,
    RunWorldPhasePassLikeCppRequest, RunWorldPhasePassResultLikeCpp, SessionPassKind,
    SessionPhaseClaimLikeCpp, SessionPhasePassOutcomeLikeCpp, SessionPhaseRequestLikeCpp,
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

        if !self.core.admission.accept_phase_authority(
            SessionPassKind::Map,
            admission.coordinator_id,
            admission.tick_epoch,
        ) || !self.core.is_map_phase_admission_current(&admission)
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

    /// C++ `WorldSession::Update` after the packet loop, on the branch
    /// `!updater.ProcessUnsafe()` — the map filter's
    /// (`Server/WorldSession.cpp:488-506`): the periodic time sync runs with
    /// that pass's diff, then the query callbacks. Neither depends on a packet
    /// having been dispatched, so both run for an empty pass too.
    pub(crate) fn run_map_phase_tail_like_cpp(&mut self, diff_ms: u32) {
        if self.core.state == SessionState::LoggedIn
            && self.core.driver.time_synchronization.timer_ms > 0
        {
            if diff_ms >= self.core.driver.time_synchronization.timer_ms {
                self.core.send_time_sync();
            } else {
                self.core.driver.time_synchronization.timer_ms -= diff_ms;
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
        let handle = self.core.player_handle_like_cpp?;
        let manager = self.core.canonical_map_manager.as_ref()?;
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
}

impl WorldSession {
    /// The rail the canonical producer addresses this session's phases through.
    #[must_use]
    pub fn session_phase_sender_like_cpp(&self) -> flume::Sender<SessionPhaseRequestLikeCpp> {
        self.phase.tx.clone()
    }

    /// A receiving handle for the same rail, for the task that owns this
    /// session and parks on it between phases.
    #[must_use]
    pub fn session_phase_receiver_like_cpp(&self) -> flume::Receiver<SessionPhaseRequestLikeCpp> {
        self.phase.rx.clone()
    }

    /// Refuse every phase request already on the rail, before any effect.
    ///
    /// Used at the handover to shutdown: from that point this session serves no
    /// phase, and a refusal is the answer that leaves the producer nothing to
    /// wait for and nothing half-done. Returns how many were refused.
    pub fn refuse_pending_phase_requests_like_cpp(&mut self) -> usize {
        let mut refused = 0;
        while let Ok(request) = self.phase.rx.try_recv() {
            refused += 1;
            self.refuse_requested_session_phase_like_cpp(request);
        }
        refused
    }

    /// Refuse a request already received when the task observes shutdown.
    /// The rail drain cannot see a request held by `recv_async`'s caller.
    pub fn refuse_requested_session_phase_like_cpp(&mut self, request: SessionPhaseRequestLikeCpp) {
        use crate::session::mailbox::SessionPhasePermitStateLikeCpp as State;
        let permit = match &request {
            SessionPhaseRequestLikeCpp::World(request) => &request.permit,
            SessionPhaseRequestLikeCpp::Map(command) => &command.permit,
            SessionPhaseRequestLikeCpp::PlayerRecovery(request) => &request.permit,
        };
        let outcome = if permit.refuse_before_start_like_cpp() {
            SessionPhasePassOutcomeLikeCpp::RefusedBeforeStart
        } else {
            match permit.state_like_cpp() {
                State::RevokedBeforeStart => SessionPhasePassOutcomeLikeCpp::RevokedBeforeStart,
                State::RefusedBeforeStart => SessionPhasePassOutcomeLikeCpp::RefusedBeforeStart,
                _ => SessionPhasePassOutcomeLikeCpp::AlreadyResolved,
            }
        };
        match request {
            SessionPhaseRequestLikeCpp::World(request) => {
                let _ = request
                    .response_tx
                    .try_send(RunWorldPhasePassResultLikeCpp {
                        coordinator_id: request.coordinator_id,
                        tick_epoch: request.tick_epoch,
                        outcome,
                        dispatched: 0,
                        disconnecting: self.is_disconnecting(),
                    });
            }
            SessionPhaseRequestLikeCpp::Map(command) => {
                let _ = command.response_tx.try_send(RunMapPhasePassResultLikeCpp {
                    tick_epoch: command.admission.tick_epoch,
                    outcome,
                    dispatched: 0,
                    stopped_at_ineligible_head: false,
                    disconnecting: self.is_disconnecting(),
                });
            }
            SessionPhaseRequestLikeCpp::PlayerRecovery(request) => {
                let admission = request.admission;
                let _ = request.response_tx.try_send(PlayerRecoveryReply {
                    coordinator_id: admission.coordinator_id,
                    tick_epoch: admission.tick_epoch,
                    map_tick_epoch: admission.map_tick_epoch,
                    outcome,
                    status: None,
                    disconnecting: self.is_disconnecting(),
                });
            }
        }
    }

    /// Run one requested phase and report its completion boundary. A world
    /// pass that disconnects returns its outstanding finalization reply to
    /// the task owner; neither its permit nor its reply completes here.
    pub async fn run_requested_session_phase_like_cpp(
        &mut self,
        request: SessionPhaseRequestLikeCpp,
        catalogs: &SessionHandlerCatalogsLikeCpp,
    ) -> Option<PendingWorldPhaseFinalizationLikeCpp> {
        match request {
            SessionPhaseRequestLikeCpp::World(request) => {
                let response_tx = request.response_tx.clone();
                let permit = std::sync::Arc::clone(&request.permit);
                let result = self
                    .run_admitted_world_phase_pass_like_cpp(request, catalogs)
                    .await;
                if result.outcome == SessionPhasePassOutcomeLikeCpp::Ran {
                    if result.disconnecting {
                        return Some(PendingWorldPhaseFinalizationLikeCpp {
                            permit,
                            response_tx,
                            result,
                        });
                    }
                    permit.complete_like_cpp();
                }
                let _ = response_tx.try_send(result);
            }
            SessionPhaseRequestLikeCpp::Map(command) => {
                let response_tx = command.response_tx.clone();
                let result = self
                    .run_admitted_map_phase_pass_like_cpp(command, catalogs)
                    .await;
                let _ = response_tx.try_send(result);
            }
            SessionPhaseRequestLikeCpp::PlayerRecovery(request) => {
                crate::session::driver::player_recovery::run_requested_player_recovery(
                    self, request, catalogs,
                )
                .await;
            }
        }
        None
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
        if !self.core.admission.accept_phase_authority(
            SessionPassKind::World,
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
        // (`WorldSession.cpp:505-511`): a `LogoutCancel` queued in this same
        // pass is dispatched above and must be seen before the decision.
        run_logout_timer_with_catalogs(
            self,
            ResurrectionOperationCatalogs {
                item_guid_generator: catalogs.id_generators.item.as_ref(),
                creature_spawns: catalogs.creature_spawns.as_ref(),
                graveyards: Some(catalogs.graveyards.as_ref()),
            },
        )
        .await;

        RunWorldPhasePassResultLikeCpp {
            coordinator_id: request.coordinator_id,
            tick_epoch: request.tick_epoch,
            outcome: SessionPhasePassOutcomeLikeCpp::Ran,
            dispatched,
            disconnecting: self.is_disconnecting(),
        }
    }
}
