//! Read the current incarnation's appearance without repeating login hydration.
use super::WorldSession;

impl WorldSession {
    pub(crate) fn owned_player_customizations_like_cpp(
        &self,
    ) -> Option<Vec<wow_entities::PlayerCustomizationChoice>> {
        self.with_owned_player_like_cpp(|player| player.gameplay_state().customizations.clone())
    }
}
