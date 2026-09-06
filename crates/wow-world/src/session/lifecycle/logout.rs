// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Logout finalization and the disconnect save.
//!
//! C++ `WorldSession::LogoutPlayer(true)` saves while `_player` still exists
//! and only then removes the player from the world, so the represented player
//! identity must stay alive until the disconnect-save path has run. The save
//! order here — loot settled first, then character, then the account-wide
//! collections, then the three offline marks — is the observable one and is
//! preserved exactly.
//!
//! The concrete database calls stay as they are behind this seam. #200
//! replaces the persistence boundary once #187 has frozen the focused Player
//! contract; this module exists so that replacement has one place to happen.

use tracing::info;

use super::super::{SessionState, WorldSession};

impl WorldSession {
    pub async fn save_disconnect_player_to_db_with_generator_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
    ) {
        if !self.finish_worldport_native_before_disconnect_like_cpp() {
            self.kick("disconnect save refused incomplete worldport native work");
            return;
        }
        let Some(player_guid) = self.player_guid() else {
            self.mark_login_account_offline_on_disconnect_like_cpp()
                .await;
            return;
        };

        info!(
            account = self.account_id,
            guid = player_guid.counter(),
            "Saving player on disconnect"
        );
        self.set_player_logout_like_cpp(true);
        self.wait_for_active_loot_persistence_with_generator_like_cpp(item_guid_generator)
            .await;
        if self.has_active_loot_views_like_cpp() {
            self.do_loot_release_all_like_cpp(player_guid).await;
        }
        self.clear_buyback_on_logout().await;
        self.save_current_player_to_db_with_generator_like_cpp(item_guid_generator)
            .await;
        self.save_account_mounts_like_cpp().await;
        self.save_account_toys_like_cpp().await;
        self.save_account_heirlooms_like_cpp().await;
        self.save_account_item_appearances_like_cpp().await;
        self.save_account_transmog_illusions_like_cpp().await;
        self.mark_character_offline().await;
        self.mark_character_account_offline_like_cpp().await;
        self.mark_login_account_offline_on_disconnect_like_cpp()
            .await;
        info!(
            account = self.account_id,
            guid = player_guid.counter(),
            "Finished disconnect save"
        );
    }

    #[cfg(test)]
    pub async fn save_disconnect_player_to_db_like_cpp(&mut self) {
        let generators = self.id_generators_for_test_like_cpp();
        self.save_disconnect_player_to_db_with_generator_like_cpp(generators.item.as_ref())
            .await;
    }
    pub(crate) fn set_player_logout_like_cpp(&mut self, player_logout: bool) {
        self.player_logout_like_cpp = player_logout;
        if player_logout {
            self.durable_loot_money_persistence_like_cpp
                .close_admission_permanently_like_cpp();
        }
        self.sync_current_player_session_visibility_detection_like_cpp();
    }

    pub(crate) fn player_logout_like_cpp(&self) -> bool {
        self.player_logout_like_cpp
    }
    /// Complete timed logout.
    ///
    /// C++ `WorldSession::LogoutPlayer(true)` saves while `_player` still
    /// exists, then removes the player from the world. Keep the represented
    /// player identity alive until the session loop runs the disconnect-save
    /// path; clearing it here would make the later save a no-op.
    pub(in crate::session) fn complete_logout(&mut self) {
        use wow_packet::packets::misc::LogoutComplete;

        info!("Logout complete for account {}", self.account_id);
        self.send_packet(&LogoutComplete);
        self.state = SessionState::Disconnecting;
    }
}
