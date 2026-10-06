// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use wow_entities::Player;

impl crate::session::HubMut<'_> {
    /// Mirror C++ update-field side effects from `SetXP` / `GiveLevel` until
    /// canonical map-owned `SendObjectUpdates` has complete session fanout.
    pub fn sync_represented_xp_level_to_canonical_and_client_like_cpp(
        &mut self,
        level_changed: bool,
        rest_info_mask: u8,
    ) {
        if self.core.player_guid().is_none() {
            return;
        }

        let level = self.shared().player_level_like_cpp();
        let (Some(xp), Some(next_level_xp), Some(scaling_player_level_delta)) = (
            self.shared().resolved_player_xp_like_cpp(),
            self.shared().resolved_player_next_level_xp_like_cpp(),
            self.shared().resolved_player_scaling_level_delta_like_cpp(),
        ) else {
            return;
        };
        let xp = xp.min(i32::MAX as u32) as i32;
        let next_level_xp = next_level_xp.min(i32::MAX as u32) as i32;
        // `GiveLevel` may publish and clear its stat delta before C++'s final
        // `SetXP(newXP)`. Preserve that final unconditional ModifyValue mark
        // without writing a second progression value back into the owner.
        let owner_marked = self
            .core
            .with_owned_player_mut_like_cpp(|player| {
                player.mark_xp_changed_like_cpp();
                player.mark_scaling_player_level_delta_changed_like_cpp();
            })
            .is_some();
        #[cfg(not(any(test, feature = "test-fixtures")))]
        if !owner_marked {
            return;
        }
        #[cfg(any(test, feature = "test-fixtures"))]
        if !owner_marked && self.core.player_handle_like_cpp.is_some() {
            // A stale handle is an unknown owner in tests too. Only legacy
            // handle-less fixtures may exercise the isolated packet adapter.
            return;
        }

        // C++ mutates RestInfo and XP on the same Player update mask before
        // `Map::SendObjectUpdates`. Build one isolated transitional delta so
        // the explicit session fanout neither splits nor duplicates RestInfo,
        // and does not leak unrelated canonical dirty fields.
        let mut delta = Player::new(None, false);
        delta.clear_data_changes();
        if level_changed {
            delta.unit_mut().set_level(level);
            delta.set_next_level_xp(next_level_xp);
        }
        delta.set_xp(xp);
        delta.mark_xp_changed_like_cpp();
        delta.set_scaling_player_level_delta_like_cpp(scaling_player_level_delta);
        delta.mark_scaling_player_level_delta_changed_like_cpp();
        if rest_info_mask != 0 {
            let (Some(rest_threshold), Some(rest_state)) = (
                self.shared().resolved_xp_rest_threshold_like_cpp(),
                self.shared().resolved_xp_rest_state_like_cpp(),
            ) else {
                return;
            };
            delta.prepare_rest_info_values_update_like_cpp(
                0,
                rest_threshold,
                rest_state,
                rest_info_mask,
            );
        }
        let update = delta.values_update(true);
        self.core.send_player_values_update_like_cpp(&update);
    }
}
