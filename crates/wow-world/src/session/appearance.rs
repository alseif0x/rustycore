//! Read the current incarnation's appearance without repeating login hydration.
use super::WorldSession;

impl WorldSession {
    pub(crate) fn owned_player_customizations_like_cpp(
        &self,
    ) -> Option<Vec<wow_entities::PlayerCustomizationChoice>> {
        crate::session::hub_ref(self).owned_player_customizations_like_cpp()
    }
}

impl crate::session::HubRef<'_> {
    pub(crate) fn owned_player_customizations_like_cpp(
        &self,
    ) -> Option<Vec<wow_entities::PlayerCustomizationChoice>> {
        self.core
            .with_owned_player_like_cpp(|player| player.gameplay_state().customizations.clone())
    }
}
