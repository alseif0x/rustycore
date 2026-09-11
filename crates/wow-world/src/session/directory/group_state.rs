// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Delivery of a group state change to the member it belongs to.
//!
//! C++ has no equivalent boundary: `Group::RemoveMember` (`Group.cpp:550`),
//! `Group::Disband` (`Group.cpp:713`) and `Group::ChangeMembersGroup` reach
//! every connected member's `Player` directly, so `Player::SetGroup`
//! (`Player.cpp:23440`) runs inside the same operation. RustyCore applies the
//! transition on the member's own session, which preserves its admission
//! phase, canonical Player guard and publication order. This module owns the
//! guarantee that the queue hop between the two cannot lose the change (#743).
//!
//! Nothing here holds group state. It records, per player GUID, that a
//! delivery did not happen, and the marked session converges on `GroupRegistry`
//! itself.

use wow_core::ObjectGuid;

use super::{PlayerDirectorySendError, PlayerRegistration, PlayerRegistry};
use crate::session::mailbox::SessionCommand;

impl PlayerRegistry {
    /// Queue a group state change, or record that its target must reconcile.
    ///
    /// C++ applies the group transition to every connected member inside
    /// `Group::RemoveMember`/`Group::Disband`/`Group::ChangeMembersGroup`, so
    /// the member's `Player::m_group` can never be left behind by a queue.
    /// RustyCore applies it on the owning session instead, which keeps the
    /// publication order of the represented operation; this boundary supplies
    /// the missing guarantee. A command that cannot be handed to the target is
    /// not dropped: the target is marked and converges on the authority in
    /// [`WorldSession::reconcile_group_state_like_cpp`].
    ///
    /// The error is still returned so the caller can log or act on the exact
    /// delivery failure. It is no longer a silent state loss.
    pub fn deliver_group_state_command_like_cpp(
        &self,
        registration: PlayerRegistration,
        command: SessionCommand,
    ) -> Result<(), PlayerDirectorySendError> {
        let result = self.try_send_current_command(registration, command);
        if result.is_err() {
            self.mark_group_state_reconciliation_like_cpp(registration.guid);
        }
        result
    }

    /// Record that `guid` must reconcile its owned group snapshot.
    ///
    /// Used when delivery fails, when the target cannot be addressed at all,
    /// and when a delivered command is dropped by its own admission phase.
    pub fn mark_group_state_reconciliation_like_cpp(&self, guid: ObjectGuid) {
        self.pending_group_state_reconciliation_like_cpp
            .insert(guid, ());
    }

    /// Consume the pending mark for `guid`, if any.
    #[must_use]
    pub fn take_group_state_reconciliation_like_cpp(&self, guid: ObjectGuid) -> bool {
        self.pending_group_state_reconciliation_like_cpp
            .remove(&guid)
            .is_some()
    }

    /// Forget any mark for a session that is gone.
    ///
    /// The successor loads `Player::m_group` from the authority on entry
    /// (C++ `Player::_LoadGroup`), so an unconsumed mark has nothing left to
    /// reconcile once the incarnation it belonged to is unregistered.
    pub(super) fn clear_group_state_reconciliation_like_cpp(&self, guid: ObjectGuid) {
        self.pending_group_state_reconciliation_like_cpp
            .remove(&guid);
    }

    /// Whether `guid` is currently marked. Reads never clear the mark.
    #[must_use]
    pub fn group_state_reconciliation_pending_like_cpp(&self, guid: ObjectGuid) -> bool {
        self.pending_group_state_reconciliation_like_cpp
            .contains_key(&guid)
    }
}
