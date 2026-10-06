//! Retain transfer-blocked save intent on the checked Player, never on Session.
pub use wow_world_lifecycle::PlayerSaveOutcomeLikeCpp;

use super::WorldSession;

impl WorldSession {
    pub(in crate::session) fn defer_player_save_for_transfer_like_cpp(
        &mut self,
    ) -> Option<PlayerSaveOutcomeLikeCpp> {
        let (state, mut hub) = crate::session::split_lifecycle_mut(self);
        state.defer_player_save_for_transfer_like_cpp(&mut hub)
    }

    pub(crate) async fn resume_deferred_player_save_with_generator_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
    ) -> Option<PlayerSaveOutcomeLikeCpp> {
        if self
            .core
            .player_handle_like_cpp
            .is_some_and(|handle| self.player_guid() != Some(handle.guid()))
        {
            return Some(PlayerSaveOutcomeLikeCpp::Unavailable);
        }
        match self
            .core
            .with_owned_player_like_cpp(|player| player.has_deferred_player_save_like_cpp())
        {
            Some(false) => None,
            Some(true) => Some(
                self.save_current_player_to_db_with_generator_like_cpp(item_guid_generator)
                    .await,
            ),
            None => {
                #[cfg(test)]
                if self.core.player_handle_like_cpp.is_none() {
                    return None;
                }
                Some(PlayerSaveOutcomeLikeCpp::Unavailable)
            }
        }
    }
}
