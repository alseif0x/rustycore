// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Canonical Player rest state.
//!
//! C++ keeps this on the `RestMgr` the Player owns
//! (`Entities/Player/RestMgr.h`): `GetRestBonus` (`:65`), `SetRestBonus`
//! (`:67`), `HasRestFlag` (`:70`), `SetRestFlag` (`:71`), `RemoveRestFlag`
//! (`:72`) and `GetInnTriggerID` (`:75`) over `_restTime` (`:86`),
//! `_innAreaTriggerId` (`:87`) and `_restFlagMask` (`:89`), with
//! `Player::SetRestState` (`Player.h:2652`) writing the update field.
//!
//! Separated from `player/mod.rs` under #779, which also closed the members.
//! Three families of state live here that C++ keeps elsewhere, and they are
//! recorded rather than introduced: the deferred flag-sync pair, which
//! RustyCore uses because the resting player flag is published by the session
//! that owns the connection instead of inline; the logout bookkeeping, which
//! C++ derives at save time from the player it is saving; and
//! `location_initialized`, which separates "no rest flags" from "the rest
//! location was never established", a distinction C++ does not need because it
//! only reads the mask while the Player is in world.

#[derive(Debug, Clone, PartialEq, Default)]
pub struct PlayerRestState {
    pub(super) rest_xp: u32,
    pub(super) rest_bonus: f32,
    pub(super) rest_honor_bonus: f32,
    pub(super) rest_state: u8,
    pub(super) rest_flag_mask: u32,
    pub(super) location_initialized: bool,
    pub(super) defer_flag_sync: bool,
    pub(super) deferred_flag_update_dirty: bool,
    pub(super) inn_area_trigger_id: u32,
    pub(super) rest_time_secs: u64,
    pub(super) logout_time: Option<u64>,
    pub(super) logout_was_resting: bool,
    pub(super) is_resting_now: bool,
}

impl PlayerRestState {
    // ---- reads ----------------------------------------------------------

    /// C++ `RestMgr::GetRestBonus(REST_TYPE_XP)` (`RestMgr.h:65`).
    #[must_use]
    pub fn rest_bonus_like_cpp(&self) -> f32 {
        self.rest_bonus
    }

    /// The honour rest bonus, C++ `GetRestBonus(REST_TYPE_HONOR)`.
    #[must_use]
    pub fn rest_honor_bonus_like_cpp(&self) -> f32 {
        self.rest_honor_bonus
    }

    /// The accumulated rested experience.
    #[must_use]
    pub fn rest_xp_like_cpp(&self) -> u32 {
        self.rest_xp
    }

    /// The rest state the client is told about (C++ `Player::SetRestState`,
    /// `Player.h:2652`).
    #[must_use]
    pub fn rest_state_like_cpp(&self) -> u8 {
        self.rest_state
    }

    /// C++ `RestMgr::HasRestFlag` (`RestMgr.h:70`) over `_restFlagMask`.
    #[must_use]
    pub fn has_rest_flag_like_cpp(&self, rest_flag: u32) -> bool {
        (self.rest_flag_mask & rest_flag) != 0
    }

    /// Whether any rest flag is active.
    #[must_use]
    pub fn is_resting_by_flag_like_cpp(&self) -> bool {
        self.rest_flag_mask != 0
    }

    /// Whether the rest location was established at least once; see the module
    /// header for why RustyCore needs this and C++ does not.
    #[must_use]
    pub fn is_location_initialized_like_cpp(&self) -> bool {
        self.location_initialized
    }

    /// C++ `RestMgr::GetInnTriggerID` (`RestMgr.h:75`).
    #[must_use]
    pub fn inn_trigger_id_like_cpp(&self) -> u32 {
        self.inn_area_trigger_id
    }

    /// C++ `RestMgr::_restTime` (`RestMgr.h:86`).
    #[must_use]
    pub fn rest_time_secs_like_cpp(&self) -> u64 {
        self.rest_time_secs
    }

    /// Whether the session is holding the resting flag publication back.
    #[must_use]
    pub fn defers_flag_sync_like_cpp(&self) -> bool {
        self.defer_flag_sync
    }

    /// Whether a held-back resting flag update is still owed to the client.
    #[must_use]
    pub fn deferred_flag_update_dirty_like_cpp(&self) -> bool {
        self.deferred_flag_update_dirty
    }

    /// The stored logout time, which the save projection writes.
    #[must_use]
    pub fn logout_time_like_cpp(&self) -> Option<u64> {
        self.logout_time
    }

    /// Whether the stored logout happened while resting.
    #[must_use]
    pub fn logout_was_resting_like_cpp(&self) -> bool {
        self.logout_was_resting
    }

    /// Whether the Player is resting right now.
    #[must_use]
    pub fn is_resting_now_like_cpp(&self) -> bool {
        self.is_resting_now
    }

    // ---- transitions ----------------------------------------------------

    /// C++ RestMgr::SetRestFlag (RestMgr.cpp:95-109). Read the clock only
    /// when the first rest flag becomes active, preserving represented timing.
    pub fn set_flag_like_cpp(
        &mut self,
        rest_flag: u32,
        trigger_id: u32,
        now: impl FnOnce() -> u64,
    ) -> bool {
        let old_mask = self.rest_flag_mask;
        self.location_initialized = true;
        self.rest_flag_mask |= rest_flag;
        let crossed_zero = old_mask == 0 && self.rest_flag_mask != 0;
        if crossed_zero {
            self.rest_time_secs = now();
        }
        if trigger_id != 0 {
            self.inn_area_trigger_id = trigger_id;
        }
        if crossed_zero && self.defer_flag_sync {
            self.deferred_flag_update_dirty = true;
        }
        crossed_zero
    }

    /// C++ RestMgr::RemoveRestFlag (RestMgr.cpp:112-122), retaining Rust's
    /// existing tavern-trigger cleanup and deferred publication bookkeeping.
    pub fn remove_flag_like_cpp(&mut self, rest_flag: u32) -> bool {
        let old_mask = self.rest_flag_mask;
        self.rest_flag_mask &= !rest_flag;
        if old_mask != self.rest_flag_mask {
            self.location_initialized = true;
        }
        let tavern = 0x1; // C++ RestMgr.h:53 REST_FLAG_IN_TAVERN.
        if (rest_flag & tavern) != 0 && (self.rest_flag_mask & tavern) == 0 {
            self.inn_area_trigger_id = 0;
        }
        let crossed_zero = old_mask != 0 && self.rest_flag_mask == 0;
        if crossed_zero {
            self.rest_time_secs = 0;
            if self.defer_flag_sync {
                self.deferred_flag_update_dirty = true;
            }
        }
        crossed_zero
    }
}

impl PlayerRestState {
    /// Rebuild the rest state from one represented payload, for the
    /// handle-less mirror that has no Player to borrow. The values travel
    /// together because they describe one rest location: the bonus and state
    /// the load restored, the flag mask with the clock and inn trigger that
    /// belong to it, and the deferred publication pair.
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn from_represented_parts_like_cpp(
        rest_state: u8,
        rest_bonus: f32,
        rest_flag_mask: u32,
        location_initialized: bool,
        defer_flag_sync: bool,
        deferred_flag_update_dirty: bool,
        inn_area_trigger_id: u32,
        rest_time_secs: u64,
    ) -> Self {
        Self {
            rest_state,
            rest_bonus,
            rest_flag_mask,
            location_initialized,
            defer_flag_sync,
            deferred_flag_update_dirty,
            inn_area_trigger_id,
            rest_time_secs,
            ..Self::default()
        }
    }

    /// The raw flag mask, for the handle-less mirror that stores it back.
    #[must_use]
    pub fn rest_flag_mask_like_cpp(&self) -> u32 {
        self.rest_flag_mask
    }

    /// C++ `RestMgr::SetRestBonus` (`RestMgr.h:67`) for the experience bonus.
    pub fn set_rest_bonus_like_cpp(&mut self, rest_bonus: f32) {
        self.rest_bonus = rest_bonus;
    }

    /// C++ `Player::SetRestState` (`Player.h:2652`) writing the client-visible
    /// rest state.
    pub fn set_rest_state_like_cpp(&mut self, rest_state: u8) {
        self.rest_state = rest_state;
    }

    /// Install the stored rest values the character load read: the bonus and
    /// the state travel together, as C++ restores both from the saved row.
    pub fn install_loaded_rest_like_cpp(&mut self, rest_state: u8, rest_bonus: f32) {
        self.rest_state = rest_state;
        self.rest_bonus = rest_bonus;
    }

    /// Return the rest-location tracking to its pre-world state: no flags, no
    /// inn trigger, no rest clock and nothing deferred. C++ has no equivalent
    /// because its `RestMgr` is constructed with the Player it belongs to;
    /// RustyCore reuses an owner across a load and must clear it explicitly.
    pub fn reset_location_tracking_like_cpp(&mut self) {
        self.rest_flag_mask = 0;
        self.location_initialized = false;
        self.defer_flag_sync = false;
        self.deferred_flag_update_dirty = false;
        self.inn_area_trigger_id = 0;
        self.rest_time_secs = 0;
    }

    /// Hold the resting flag publication back until the world entry finishes.
    /// Kept pending across a same-zone re-entry after a cancelled post-add.
    pub fn defer_flag_sync_like_cpp(&mut self) {
        self.defer_flag_sync = true;
    }

    /// Stop holding the publication back and answer whether an update is still
    /// owed to the client.
    pub fn end_deferred_flag_sync_like_cpp(&mut self) -> bool {
        self.defer_flag_sync = false;
        self.deferred_flag_update_dirty
    }

    /// Record that the owed resting flag update has been sent.
    pub fn clear_deferred_flag_update_like_cpp(&mut self) {
        self.deferred_flag_update_dirty = false;
    }

    /// Take the owed resting flag update, so the same one is not sent twice.
    pub fn take_deferred_flag_update_like_cpp(&mut self) -> bool {
        std::mem::take(&mut self.deferred_flag_update_dirty)
    }

    /// Set the rest clock C++ keeps in `RestMgr::_restTime` (`RestMgr.h:86`).
    pub fn set_rest_time_secs_like_cpp(&mut self, rest_time_secs: u64) {
        self.rest_time_secs = rest_time_secs;
    }

    /// Record the logout the save projection writes: when it happened and
    /// whether the Player was resting then. C++ derives both at save time from
    /// the Player it is saving.
    pub fn set_logout_like_cpp(&mut self, logout_time: Option<u64>, was_resting: bool) {
        self.logout_time = logout_time;
        self.logout_was_resting = was_resting;
    }

    /// Record whether the Player is resting right now.
    pub fn set_resting_now_like_cpp(&mut self, is_resting_now: bool) {
        self.is_resting_now = is_resting_now;
    }

    /// Set the accumulated rested experience.
    pub fn set_rest_xp_like_cpp(&mut self, rest_xp: u32) {
        self.rest_xp = rest_xp;
    }

    /// Set the honour rest bonus (C++ `SetRestBonus(REST_TYPE_HONOR, …)`).
    pub fn set_rest_honor_bonus_like_cpp(&mut self, rest_honor_bonus: f32) {
        self.rest_honor_bonus = rest_honor_bonus;
    }
}
