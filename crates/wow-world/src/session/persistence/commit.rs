//! Commit, durability, finalization and rollback for represented Session state.
//!
//! Moved out of the Session root under #609. Behaviour is preserved; the
//! canonical owner of this state is unchanged. The talent-reset commit and its
//! publication moved to `wow_world_application::session_lifecycle` (#1363 slice 13b).

use super::*;

impl WorldSession {
    /// Apply the already-committed void-storage unlock to runtime state and
    /// emit the same PlayerData::Flags values delta that C++ SetPlayerFlag does.
    pub(crate) fn apply_committed_void_storage_unlock_like_cpp(&mut self) {
        let values_update = self.player_values_update_snapshot().and_then(|mut player| {
            player.set_player_flag_like_cpp(PLAYER_FLAGS_VOID_UNLOCKED_LIKE_CPP);
            Some(player.values_update_like_cpp(true))
        });

        let Some(_current_flags) = self.represented_player_flags_value_like_cpp() else {
            return;
        };
        let _canonical = self
            .core
            .mutate_canonical_player_like_cpp(|player| {
                player.set_player_flag(PLAYER_FLAGS_VOID_UNLOCKED_LIKE_CPP);
            })
            .is_some();
        #[cfg(test)]
        if self.core.player_handle_like_cpp.is_none() {
            self.lifecycle
                .set_represented_loaded_player_flags_for_test_like_cpp(Some(
                    _current_flags | PLAYER_FLAGS_VOID_UNLOCKED_LIKE_CPP,
                ));
            self.lifecycle
                .set_represented_loaded_player_flags_applied_for_test_like_cpp(_canonical);
        }

        if let Some(update) = values_update {
            self.core.send_player_values_update_like_cpp(&update);
        }
    }
}
