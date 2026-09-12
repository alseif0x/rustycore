// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Canonical Player queued spell-cast request.
//!
//! C++ `Player` owns `std::unique_ptr<SpellCastRequest> _pendingSpellCastRequest`
//! (`Player.h:3154`) and performs the transitions itself: `RequestSpellCast`
//! (`Player.cpp:29078`), which cancels whatever request it replaces before
//! storing the new one; `CancelPendingCastRequest` (`:29091`), which clears the
//! slot after telling the client the cast failed; and
//! `ExecutePendingSpellCastRequest` (`:29122`), which clears it once the stored
//! request has been consumed.
//!
//! The named transitions live here under #771 so the queued request is not
//! reached through a `&mut Option<...>` from another crate. The `CastFailed`
//! packet stays with the session: C++ sends it from the Player, RustyCore sends
//! it from the session that owns the connection, so these operations hand the
//! caller the request it must report on rather than reporting themselves.

use crate::{PendingSpellCastRequestLikeCpp, Player};
use wow_core::ObjectGuid;

impl Player {
    /// The queued request, if one is waiting.
    #[must_use]
    pub fn pending_spell_cast_like_cpp(&self) -> Option<&PendingSpellCastRequestLikeCpp> {
        self.gameplay_state().pending_spell_cast.as_ref()
    }

    /// A copy of the queued request, for a caller that must own it.
    #[must_use]
    pub fn pending_spell_cast_snapshot_like_cpp(&self) -> Option<PendingSpellCastRequestLikeCpp> {
        self.gameplay_state().pending_spell_cast.clone()
    }

    /// C++ `Player::RequestSpellCast` (`Player.cpp:29078`) storing the new
    /// request. It answers the request this one replaced, which C++ cancels
    /// inline at `:29082`; the caller owns the `CastFailed` that cancellation
    /// owes the client.
    pub fn request_spell_cast_like_cpp(
        &mut self,
        request: PendingSpellCastRequestLikeCpp,
    ) -> Option<PendingSpellCastRequestLikeCpp> {
        self.gameplay_state_mut()
            .pending_spell_cast
            .replace(request)
    }

    /// C++ `Player::CancelPendingCastRequest` (`Player.cpp:29091`) clearing the
    /// slot. It answers the request that was waiting, because C++ reads its
    /// cast id and spell id into `CastFailed` before dropping it (`:29098`),
    /// and answers `None` for the early return at `:29093`.
    pub fn cancel_pending_spell_cast_like_cpp(&mut self) -> Option<PendingSpellCastRequestLikeCpp> {
        self.gameplay_state_mut().pending_spell_cast.take()
    }

    /// Take the queued request only while it is still the one the caller
    /// planned for, as C++ `ExecutePendingSpellCastRequest` (`Player.cpp:29122`)
    /// consumes the request it is holding rather than whatever arrived since.
    ///
    /// The identity is the cast id, the spell id and the casting unit, which is
    /// what C++ reads back off `_pendingSpellCastRequest` before executing.
    pub fn take_matching_pending_spell_cast_like_cpp(
        &mut self,
        cast_id: ObjectGuid,
        spell_id: i32,
        casting_unit_guid: ObjectGuid,
    ) -> Option<PendingSpellCastRequestLikeCpp> {
        let pending = &mut self.gameplay_state_mut().pending_spell_cast;
        let matches = pending.as_ref().is_some_and(|current| {
            current.cast_id == cast_id
                && current.spell_id == spell_id
                && current.casting_unit_guid == casting_unit_guid
        });
        if matches { pending.take() } else { None }
    }
}
