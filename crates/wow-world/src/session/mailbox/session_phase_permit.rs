// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! The shared permit of one admitted map-phase pass (#787).
//!
//! C++ owns the session while it drives it from `Map::Update`
//! (`Maps/Map.cpp:669-680`), so "the map moved on" and "the session is running
//! its pass" cannot both be true. RustyCore delivers the pass to the session's
//! own task, so the two sides need a single decision point: the coordinator
//! revokes, the session claims, and exactly one of them wins.
//!
//! The race is resolved by one atomic transition on one state. Reading a flag
//! and then acting on it is not equivalent: the session could pass the read
//! before the deadline and start its effects after it.

use std::sync::Arc;
use std::sync::atomic::{AtomicU8, Ordering};

const PENDING: u8 = 0;
const RUNNING: u8 = 1;
const COMPLETED: u8 = 2;
const REVOKED_BEFORE_START: u8 = 3;
const REFUSED_BEFORE_START: u8 = 4;
const INTERRUPTED_AFTER_START: u8 = 5;

/// The observable state of one admitted pass.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SessionPhasePermitStateLikeCpp {
    /// Delivered, not yet claimed. No effect of the pass has run.
    Pending,
    /// Claimed by the session: its effects may be running right now.
    Running,
    /// The session finished the pass and its tails.
    Completed,
    /// The coordinator won the race at the deadline: the pass provably never
    /// ran any effect and never will.
    RevokedBeforeStart,
    /// The session declined before any effect, because the admission it was
    /// given no longer describes it.
    RefusedBeforeStart,
    /// The session started and stopped without finishing. Nothing may assume
    /// its mutations completed or rolled back.
    InterruptedAfterStart,
}

/// What a session's claim attempt decided.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SessionPhaseClaimLikeCpp {
    /// The session owns the pass and must resolve it exactly once.
    Claimed,
    /// The coordinator revoked first: the session must run no effect.
    Revoked,
    /// Already claimed or resolved; a duplicate delivery.
    AlreadyResolved(SessionPhasePermitStateLikeCpp),
}

/// What a coordinator's revocation attempt decided.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SessionPhaseRevokeLikeCpp {
    /// The pass will never run: continuing is safe.
    Revoked,
    /// The session already started. The deadline is diagnostic only: the
    /// coordinator may not continue while these effects are live.
    AlreadyRunning,
    /// The pass already reached a terminal state.
    AlreadyResolved(SessionPhasePermitStateLikeCpp),
}

/// One admitted map-phase pass, shared by the coordinator and the session.
#[derive(Debug)]
pub struct SessionPhasePermitLikeCpp {
    state: AtomicU8,
}

impl SessionPhasePermitLikeCpp {
    #[must_use]
    pub fn new_like_cpp() -> Arc<Self> {
        Arc::new(Self {
            state: AtomicU8::new(PENDING),
        })
    }

    #[must_use]
    pub fn state_like_cpp(&self) -> SessionPhasePermitStateLikeCpp {
        match self.state.load(Ordering::Acquire) {
            PENDING => SessionPhasePermitStateLikeCpp::Pending,
            RUNNING => SessionPhasePermitStateLikeCpp::Running,
            COMPLETED => SessionPhasePermitStateLikeCpp::Completed,
            REVOKED_BEFORE_START => SessionPhasePermitStateLikeCpp::RevokedBeforeStart,
            REFUSED_BEFORE_START => SessionPhasePermitStateLikeCpp::RefusedBeforeStart,
            _ => SessionPhasePermitStateLikeCpp::InterruptedAfterStart,
        }
    }

    /// Session side: take exclusive ownership of the pass **before any effect**.
    ///
    /// Everything the pass mutates must happen after a `Claimed` answer; a
    /// handler that runs before it cannot be revoked or rolled back.
    pub fn claim_like_cpp(&self) -> SessionPhaseClaimLikeCpp {
        match self
            .state
            .compare_exchange(PENDING, RUNNING, Ordering::AcqRel, Ordering::Acquire)
        {
            Ok(_) => SessionPhaseClaimLikeCpp::Claimed,
            Err(REVOKED_BEFORE_START) => SessionPhaseClaimLikeCpp::Revoked,
            Err(_) => SessionPhaseClaimLikeCpp::AlreadyResolved(self.state_like_cpp()),
        }
    }

    /// Coordinator side: at the deadline, try to make the pass provably never
    /// run. Only a pass that has not been claimed can be revoked.
    pub fn revoke_before_start_like_cpp(&self) -> SessionPhaseRevokeLikeCpp {
        match self.state.compare_exchange(
            PENDING,
            REVOKED_BEFORE_START,
            Ordering::AcqRel,
            Ordering::Acquire,
        ) {
            Ok(_) => SessionPhaseRevokeLikeCpp::Revoked,
            Err(RUNNING) => SessionPhaseRevokeLikeCpp::AlreadyRunning,
            Err(_) => SessionPhaseRevokeLikeCpp::AlreadyResolved(self.state_like_cpp()),
        }
    }

    /// Session side: decline before any effect, because the frozen admission no
    /// longer describes this session, player, handle, map or residence.
    pub fn refuse_before_start_like_cpp(&self) -> bool {
        self.state
            .compare_exchange(
                PENDING,
                REFUSED_BEFORE_START,
                Ordering::AcqRel,
                Ordering::Acquire,
            )
            .is_ok()
    }

    /// Session side: the pass and its tails finished. Only a claimed pass can
    /// complete, and completion is always an explicit call at the end of the
    /// pass, never a destructor: a dropped guard proves nothing about whether
    /// the mutations finished.
    pub fn complete_like_cpp(&self) -> bool {
        self.state
            .compare_exchange(RUNNING, COMPLETED, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
    }

    /// Session side: the claimed pass stopped without finishing. This is its
    /// own terminal state precisely because it is not equivalent to either
    /// completion or refusal.
    pub fn interrupt_after_start_like_cpp(&self) -> bool {
        self.state
            .compare_exchange(
                RUNNING,
                INTERRUPTED_AFTER_START,
                Ordering::AcqRel,
                Ordering::Acquire,
            )
            .is_ok()
    }
}

#[cfg(test)]
#[path = "session_phase_permit/tests.rs"]
mod tests;
