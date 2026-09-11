// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! The borrow of Player reputation state these rules operate on.
//!
//! Separated from the `mgr.rs` root so it keeps its reviewed size; the
//! contract is documented there (#735).

use super::*;

/// The C++ `ReputationMgr` operating on borrowed Player reputation state.
///
/// `S` is `&PlayerReputationStateLikeCpp` for the read-only operations and
/// `&mut PlayerReputationStateLikeCpp` for the transitions; a mutable borrow
/// satisfies both, so a caller that mutates can still read.
#[derive(Debug)]
pub struct ReputationMgrLikeCpp<S> {
    state: S,
}

/// Read-only `ReputationMgr` over the Player's reputation state.
pub type ReputationMgrRefLikeCpp<'a> = ReputationMgrLikeCpp<&'a PlayerReputationStateLikeCpp>;

/// Mutating `ReputationMgr` over the Player's reputation state.
pub type ReputationMgrMutLikeCpp<'a> = ReputationMgrLikeCpp<&'a mut PlayerReputationStateLikeCpp>;

/// A manager over its own state, for the unit tests of these rules.
///
/// Production always borrows the canonical Player's state; owning a copy here
/// would be the second authority #735 retires, so this constructor exists only
/// under `cfg(test)`.
#[cfg(test)]
impl ReputationMgrLikeCpp<PlayerReputationStateLikeCpp> {
    #[must_use]
    pub fn new_like_cpp() -> Self {
        Self {
            state: PlayerReputationStateLikeCpp::default(),
        }
    }
}

impl<'a> ReputationMgrLikeCpp<&'a PlayerReputationStateLikeCpp> {
    #[must_use]
    pub fn borrowing_like_cpp(state: &'a PlayerReputationStateLikeCpp) -> Self {
        Self { state }
    }
}

impl<'a> ReputationMgrLikeCpp<&'a mut PlayerReputationStateLikeCpp> {
    #[must_use]
    pub fn borrowing_mut_like_cpp(state: &'a mut PlayerReputationStateLikeCpp) -> Self {
        Self { state }
    }
}

impl<S: std::borrow::Borrow<PlayerReputationStateLikeCpp>> ReputationMgrLikeCpp<S> {
    /// The Player-owned state this manager operates on.
    pub(super) fn state(&self) -> &PlayerReputationStateLikeCpp {
        self.state.borrow()
    }

    /// Copy the borrowed state for a read that must outlive the Player borrow.
    #[must_use]
    pub fn cloned_state_like_cpp(&self) -> PlayerReputationStateLikeCpp {
        self.state().clone()
    }
}

impl<S: std::borrow::BorrowMut<PlayerReputationStateLikeCpp>> ReputationMgrLikeCpp<S> {
    pub(super) fn state_mut(&mut self) -> &mut PlayerReputationStateLikeCpp {
        self.state.borrow_mut()
    }

    /// Install a previously read state, as a fixture that reinstalls a Player
    /// owner does. This is a whole-state assignment and has no production
    /// caller: a transition uses the named operations instead.
    #[cfg(test)]
    pub fn replace_state_like_cpp(&mut self, state: PlayerReputationStateLikeCpp) {
        *self.state_mut() = state;
    }
}
