// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! The ordered Session driver phase trace.
//!
//! This enum is the frozen record of what one Session pass actually does, in
//! the order it does it. It exists so tests can assert on the *production*
//! sequence instead of reimplementing it: the driver records each phase as it
//! runs, and a test compares the recorded trace against the expected one.
//!
//! These are Session phases only. They are not the world/map clock: C++
//! `World::UpdateSessions` runs `WorldSession::Update` per session, and
//! `MapManager::Update -> Map::Update -> DelayedUpdate` is a separate sequence
//! owned elsewhere (see `docs/architecture/runtime-clock-phase-trace.md`).
//! Nothing here may become a second scheduler or a gameplay tick owner.

/// One phase of a Session driver pass, in execution order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum SessionDriverPhaseLikeCpp {
    // --- `update(diff_ms)`: synchronous ingestion and Session timers ---
    /// Bounded drain of the primary (instance after ConnectTo) packet channel.
    DrainPrimaryPackets,
    /// Bounded drain of the parked realm channel, sharing the same budget.
    DrainRealmPackets,
    /// `SocketTimeOutTime`-like idle deadline; may set `Disconnecting`.
    ConnectionTimeout,
    /// Session-owned ticks that C++ runs inside `Player::Update`.
    SessionOwnedTicks,
    /// Periodic `SendTimeSync`, every 10s once logged in.
    TimeSync,
    /// Deferred logout completion.
    LogoutTimer,

    // --- `process_pending()`: asynchronous phases ---
    /// Flush a pending packet-spoof ban before anything else observes it.
    FlushPacketSpoofBan,
    /// Drain the Session mailbox in FIFO order.
    SessionCommands,
    /// Settle queued creature kills, loot and rewards.
    CreatureKills,
    /// Logged-in gameplay follow-ups (loot rolls, GameObject, spell casts).
    LoggedInGameplayTicks,
    /// ConnectTo attach point; see `session::connection`.
    PollInstanceLink,
    /// Deferred nearby creature/GameObject spawn needing a DB query.
    PendingCreatureSpawn,
    /// Dispatch the packets ingested earlier in this pass.
    DispatchQueuedPackets,
    /// Session-owned ready rename reads/commit results; never awaits DB work.
    CharacterRenameCallbacks,
    /// Periodic character save.
    PeriodicPlayerSave,
}

impl super::super::WorldSession {
    /// Record one driver phase. Compiles away outside tests: the trace exists
    /// to prove the production order, not to cost anything in production.
    #[inline]
    pub(crate) fn record_driver_phase_like_cpp(&mut self, phase: SessionDriverPhaseLikeCpp) {
        #[cfg(test)]
        self.driver_phase_trace_like_cpp.push(phase);
        #[cfg(not(test))]
        let _ = phase;
    }

    /// The phases recorded since the last reset, in execution order.
    #[cfg(test)]
    pub(crate) fn driver_phase_trace_like_cpp(&self) -> &[SessionDriverPhaseLikeCpp] {
        &self.driver_phase_trace_like_cpp
    }

    /// Clear the recorded trace so one test can assert per pass.
    #[cfg(test)]
    pub(crate) fn reset_driver_phase_trace_like_cpp(&mut self) {
        self.driver_phase_trace_like_cpp.clear();
    }
}
