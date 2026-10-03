use wow_core::ObjectGuid;
use wow_entities::PlayerInteractionDataLikeCpp;
use wow_world_core::session::HubRef;

use crate::InteractionState;

impl InteractionState {
    fn resolved_player_interaction_data_like_cpp(
        &self,
        hub: HubRef<'_>,
    ) -> Option<PlayerInteractionDataLikeCpp> {
        let canonical = hub
            .core
            .with_owned_player_like_cpp(|player| *player.interaction_data_like_cpp());
        #[cfg(any(test, feature = "test-fixtures"))]
        if canonical.is_none() && hub.core.player_handle_like_cpp.is_none() {
            return Some(self.player_interaction_data_like_cpp);
        }
        canonical
    }

    pub fn reset_player_interaction_data_like_cpp(&mut self, hub: HubRef<'_>) -> bool {
        let canonical = hub
            .core
            .with_owned_player_mut_like_cpp(|player| player.reset_interaction_data_like_cpp())
            .is_some();
        #[cfg(any(test, feature = "test-fixtures"))]
        if canonical || hub.core.player_handle_like_cpp.is_none() {
            self.player_interaction_data_like_cpp.reset();
        }
        canonical || cfg!(any(test, feature = "test-fixtures"))
            && hub.core.player_handle_like_cpp.is_none()
    }

    pub fn set_player_interaction_source_like_cpp(
        &mut self,
        hub: HubRef<'_>,
        source_guid: ObjectGuid,
    ) -> bool {
        let canonical = hub
            .core
            .with_owned_player_mut_like_cpp(|player| {
                player.set_interaction_source_like_cpp(source_guid);
            })
            .is_some();
        #[cfg(any(test, feature = "test-fixtures"))]
        if canonical || hub.core.player_handle_like_cpp.is_none() {
            self.player_interaction_data_like_cpp
                .set_source(source_guid);
        }
        canonical || cfg!(any(test, feature = "test-fixtures"))
            && hub.core.player_handle_like_cpp.is_none()
    }

    pub fn set_player_trainer_interaction_like_cpp(
        &mut self,
        hub: HubRef<'_>,
        source_guid: ObjectGuid,
        trainer_id: u32,
    ) -> bool {
        let canonical = hub
            .core
            .with_owned_player_mut_like_cpp(|player| {
                player.set_trainer_interaction_like_cpp(source_guid, trainer_id);
            })
            .is_some();
        #[cfg(any(test, feature = "test-fixtures"))]
        if canonical || hub.core.player_handle_like_cpp.is_none() {
            self.player_interaction_data_like_cpp
                .set_trainer(source_guid, trainer_id);
        }
        canonical || cfg!(any(test, feature = "test-fixtures"))
            && hub.core.player_handle_like_cpp.is_none()
    }

    pub fn reset_player_interaction_if_source_like_cpp(
        &mut self,
        hub: HubRef<'_>,
        source_guid: ObjectGuid,
    ) -> bool {
        let canonical = hub.core.with_owned_player_mut_like_cpp(|player| {
            player.reset_interaction_if_source_like_cpp(source_guid)
        });
        #[cfg(any(test, feature = "test-fixtures"))]
        if canonical.is_some() || hub.core.player_handle_like_cpp.is_none() {
            let fixture = self
                .player_interaction_data_like_cpp
                .reset_if_source(source_guid);
            return canonical.unwrap_or(fixture);
        }
        canonical.unwrap_or(false)
    }

    pub fn player_interaction_source_guid_like_cpp(
        &self,
        hub: HubRef<'_>,
    ) -> Option<ObjectGuid> {
        let interaction = self.resolved_player_interaction_data_like_cpp(hub)?;
        (!interaction.source_guid.is_empty()).then_some(interaction.source_guid)
    }

    pub fn resolved_player_interaction_trainer_id_like_cpp(
        &self,
        hub: HubRef<'_>,
    ) -> Option<u32> {
        self.resolved_player_interaction_data_like_cpp(hub)
            .map(|interaction| interaction.trainer_id)
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn player_interaction_trainer_id_like_cpp(&self, hub: HubRef<'_>) -> u32 {
        self.resolved_player_interaction_trainer_id_like_cpp(hub)
            .unwrap_or(0)
    }

    pub fn player_trainer_interaction_matches_like_cpp(
        &self,
        hub: HubRef<'_>,
        source_guid: ObjectGuid,
        trainer_id: i32,
    ) -> bool {
        self.resolved_player_interaction_data_like_cpp(hub)
            .is_some_and(|interaction| interaction.trainer_matches(source_guid, trainer_id))
    }
}
