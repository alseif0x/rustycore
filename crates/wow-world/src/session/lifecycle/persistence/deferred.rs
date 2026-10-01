//! Retain transfer-blocked save intent on the checked Player, never on Session.
use super::WorldSession;

/// Transaction classification is distinct from local acknowledgement or client readiness.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerSaveOutcomeLikeCpp {
    /// Transaction confirmed; local acknowledgement may lose an incarnation race.
    Applied,
    /// Transfer retained native save intent; no full-save transaction submitted.
    Deferred,
    /// No coherent projection/owner or lifecycle port; not a classified rollback.
    Unavailable,
    /// Submitted full-save transaction returned a known failure.
    Failed,
    /// Existing durable-work reconciliation/fence forbids save or replay.
    /// Includes uncertain COMMIT; does not assert that this call submitted one.
    Quarantined,
}

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

impl crate::session::state::SessionLifecycleState {
    /// None permits normal save preparation; Some is a completed admission decision.
    pub(in crate::session) fn defer_player_save_for_transfer_like_cpp(
        &mut self,
        hub: &mut crate::session::HubMut<'_>,
    ) -> Option<PlayerSaveOutcomeLikeCpp> {
        #[cfg(test)]
        if hub.core.player_handle_like_cpp.is_none() {
            return None; // Existing ownerless persistence fixtures, never production.
        }
        if hub
            .core
            .player_handle_like_cpp
            .is_none_or(|handle| hub.core.player_guid() != Some(handle.guid()))
        {
            return Some(PlayerSaveOutcomeLikeCpp::Unavailable);
        }
        match hub.core.with_owned_player_mut_like_cpp(|player| {
            player.defer_save_if_transfer_pending_like_cpp()
        }) {
            Some(Some(false)) => None,
            Some(Some(true)) => Some(PlayerSaveOutcomeLikeCpp::Deferred),
            Some(None) => {
                hub.core.kick("deferred player-save revision exhausted");
                Some(PlayerSaveOutcomeLikeCpp::Unavailable)
            }
            None => Some(PlayerSaveOutcomeLikeCpp::Unavailable),
        }
    }
}
