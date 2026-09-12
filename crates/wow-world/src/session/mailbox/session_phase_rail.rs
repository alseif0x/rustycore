// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! The phase rail of one session (#787).
//!
//! C++ never asks a session to run a phase: it *is* the caller. `World::Update`
//! runs `UpdateSessions(diff)` with `WorldSessionFilter` (`World.cpp:2704`) and
//! then `MapManager::Update(diff)` (`World.cpp:2748`), whose maps drive their
//! players' sessions with `MapSessionFilter` (`Map.cpp:669-680`). Both calls
//! happen on the one thread that owns the session at that moment.
//!
//! RustyCore sessions own themselves in their own tasks, so the order has to
//! travel: one rail per session carries both phase requests in the order the
//! canonical producer issued them. It is deliberately **not** the command
//! mailbox: a phase pass drains that mailbox, so delivering a phase through it
//! would run one phase inside another and let a later phase overtake the
//! earlier one in the same iteration.

use std::sync::Arc;

use super::SessionPhasePermitLikeCpp;

/// One phase the canonical producer asks this session to run.
#[derive(Clone, Debug)]
pub enum SessionPhaseRequestLikeCpp {
    /// C++ `World::UpdateSessions` with `WorldSessionFilter`
    /// (`World.cpp:2704`, `WorldSession.cpp:88-107`).
    World(RunWorldPhasePassLikeCppRequest),
    /// C++ `Map::Update` driving its players' sessions with `MapSessionFilter`
    /// (`Map.cpp:669-680`, `WorldSession.cpp:64-86`).
    Map(RunMapPhasePassLikeCppCommand),
}

impl SessionPhaseRequestLikeCpp {
    /// The producer instance that issued the request.
    #[must_use]
    pub fn coordinator_id_like_cpp(&self) -> u64 {
        match self {
            Self::World(request) => request.coordinator_id,
            Self::Map(command) => command.admission.coordinator_id,
        }
    }
}

/// One admitted world-phase pass.
///
/// The world phase has no per-player admission: C++ runs it for every session
/// in `World::m_sessions`, including those still on the character screen, so
/// the identity that matters is the producer and its step, not a player.
#[derive(Clone, Debug)]
pub struct RunWorldPhasePassLikeCppRequest {
    pub coordinator_id: u64,
    pub tick_epoch: u64,
    /// The diff of that step, the same one the maps will receive.
    pub diff_ms: u32,
    pub permit: Arc<SessionPhasePermitLikeCpp>,
    pub response_tx: flume::Sender<RunWorldPhasePassResultLikeCpp>,
}

/// The completion boundary of one world-phase pass, sent after the pass and its
/// tails, never on observing the request.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RunWorldPhasePassResultLikeCpp {
    pub coordinator_id: u64,
    pub tick_epoch: u64,
    pub outcome: SessionPhasePassOutcomeLikeCpp,
    pub dispatched: usize,
    pub disconnecting: bool,
}

/// C++ `Map::Update` drives the sessions of the players on that map through
/// `MapSessionFilter` before respawns and the object updaters
/// (`Maps/Map.cpp:669-680`). RustyCore sessions own themselves in their own
/// tasks, so the canonical tick asks each admitted session to run that pass and
/// waits for the completion boundary below before resuming the tick.
///
/// The request carries the tick epoch so a late completion cannot be mistaken
/// for this tick's, and the effective diff saved at the split so the session
/// sees the same diff C++ passes to every map of that tick.
#[derive(Clone, Debug)]
pub struct RunMapPhasePassLikeCppCommand {
    /// Everything the session revalidates before running any effect.
    pub admission: MapPhaseAdmissionLikeCpp,
    /// The shared permit whose single atomic transition decides between this
    /// session starting the pass and the coordinator revoking it.
    pub permit: Arc<SessionPhasePermitLikeCpp>,
    pub response_tx: flume::Sender<RunMapPhasePassResultLikeCpp>,
}

/// The identity one admitted map-phase pass was granted to.
///
/// C++ never needs this: the map thread holds the session it drives, so the
/// player, its incarnation, its map and its residence cannot change underneath
/// the pass. RustyCore releases every guard before delivering, so each of those
/// facts is frozen here and rechecked by the session before its first effect.
/// A→B→A is covered because the residence revision changes even when the map
/// key comes back the same.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MapPhaseAdmissionLikeCpp {
    /// The coordinator instance that admitted the pass. A second coordinator's
    /// request is not this one's, whatever its epoch says.
    pub coordinator_id: u64,
    /// The canonical tick this pass belongs to.
    pub tick_epoch: u64,
    /// The phase C++ would be running here, `MapSessionFilter`'s
    /// (`WorldSession.cpp:64-86`).
    pub phase: wow_handler::PacketUpdatePhase,
    /// The directory incarnation of the session, which survives neither a
    /// replacement session nor a new character selection.
    pub registration: crate::session::directory::PlayerRegistration,
    /// The map-owned Player incarnation, a different generation space from the
    /// registration above.
    pub handle: wow_map::PlayerHandle,
    /// The map whose tick admitted the pass, and the incarnation of that map.
    pub map_key: wow_map::MapKey,
    pub map_incarnation: u64,
    /// The residence revision observed at admission.
    pub residence_revision: u64,
    /// The effective diff of that tick, which feeds this pass's tails.
    pub diff_ms: u32,
}

/// The completion boundary of one map-phase pass.
///
/// This is sent **after** the pass finished dispatching, not when the command
/// was observed: an acknowledgement of receipt is not completion. A session
/// that could not run the pass reports `ran: false` with the reason implied by
/// the other fields, and the tick treats that as "this session did not run its
/// map pass", never as success.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RunMapPhasePassResultLikeCpp {
    pub tick_epoch: u64,
    pub outcome: SessionPhasePassOutcomeLikeCpp,
    pub dispatched: usize,
    pub stopped_at_ineligible_head: bool,
    pub disconnecting: bool,
}

impl RunMapPhasePassResultLikeCpp {
    /// Whether this session actually ran the admitted pass and its tails.
    #[must_use]
    pub fn ran_like_cpp(&self) -> bool {
        self.outcome == SessionPhasePassOutcomeLikeCpp::Ran
    }
}

/// Why one admitted pass ended the way it did.
///
/// The distinctions are load-bearing: only `Ran` means the phase happened, and
/// only `Ran`, `RevokedBeforeStart` and `RefusedBeforeStart` mean the session
/// has no live effect of this request left.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SessionPhasePassOutcomeLikeCpp {
    /// Claimed, dispatched and finished, tails included.
    Ran,
    /// The coordinator revoked the pass before this session claimed it, so no
    /// effect of it ran.
    RevokedBeforeStart,
    /// The frozen admission no longer describes this session, player,
    /// incarnation, map or residence. Refused before any effect.
    RefusedBeforeStart,
    /// The permit was already resolved when the command was observed: a
    /// duplicate delivery, which runs nothing.
    AlreadyResolved,
}
