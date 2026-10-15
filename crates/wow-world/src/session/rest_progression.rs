// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Rest progression: private Session responsibility.
//! Relocated under #1233; canonical state, phase order and public paths are unchanged.
//! The played-time values moved to `SessionLifecycleState` and the test-only
//! rest entry points to `handlers/progression_test_shims.rs` (#1363 slice 7c).

use super::PLAYER_FLAGS_VOID_UNLOCKED_LIKE_CPP;
#[cfg(test)]
use super::RepresentedAuraEffectLikeCpp;
use super::WorldSession;

impl WorldSession {
    pub(crate) fn represented_player_has_flag_like_cpp(&self, flag: u32) -> bool {
        let canonical = self.player_guid().and_then(|guid| {
            self.core
                .canonical_player_has_player_flag_like_cpp(guid, flag)
        });
        if let Some(value) = canonical {
            return value;
        }
        #[cfg(test)]
        if self.core.player_handle_like_cpp.is_none() {
            return self
                .lifecycle
                .represented_loaded_player_flags_for_test_like_cpp()
                .is_some_and(|flags| (flags & flag) != 0);
        }
        false
    }

    pub(crate) fn represented_player_flags_value_like_cpp(&self) -> Option<u32> {
        let canonical = self
            .core
            .canonical_player_snapshot_like_cpp(|player| player.data().player_flags);
        #[cfg(test)]
        if canonical.is_none() && self.core.player_handle_like_cpp.is_none() {
            return self
                .lifecycle
                .represented_loaded_player_flags_for_test_like_cpp();
        }
        canonical
    }

    pub(crate) fn void_storage_is_unlocked_like_cpp(&self) -> bool {
        self.represented_player_has_flag_like_cpp(PLAYER_FLAGS_VOID_UNLOCKED_LIKE_CPP)
    }

    pub(crate) fn set_represented_rest_flag_like_cpp(
        &mut self,
        rest_flag: u32,
        trigger_id: u32,
    ) -> bool {
        self.set_player_rest_flag_like_cpp(rest_flag, trigger_id)
    }

    pub(crate) fn resolved_player_flags_for_create_like_cpp(&self) -> Option<(u32, u32)> {
        let player_flags = self.resolved_player_flags_for_rest_state_save_like_cpp()?;
        let canonical_flags_ex = self
            .core
            .with_owned_player_for_rest_like_cpp(|player| player.data().player_flags_ex);
        #[cfg(test)]
        let canonical_flags_ex =
            if canonical_flags_ex.is_none() && self.core.player_handle_like_cpp.is_none() {
                self.lifecycle
                    .represented_loaded_player_flags_ex_for_test_like_cpp()
                    .or(Some(0))
            } else {
                canonical_flags_ex
            };
        let player_flags_ex = canonical_flags_ex?;
        Some((player_flags, player_flags_ex))
    }

    pub(crate) fn take_deferred_rest_flag_update_dirty_like_cpp(&mut self) -> bool {
        self.take_player_deferred_rest_flag_update_dirty_like_cpp()
    }
}

#[cfg(test)]
#[path = "../../unit_tests/session/rest_progression/f3_shims.rs"]
mod f3_shims;
