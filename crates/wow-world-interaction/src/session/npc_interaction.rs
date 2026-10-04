use wow_core::ObjectGuid;
use wow_entities::PlayerInteractionDataLikeCpp;
use wow_world_core::session::HubRef;
use wow_world_core::session::TrainerInteractionRoleAccessLikeCpp;

use crate::InteractionState;

impl InteractionState {
    /// Set the trainer role through the typed canonical-player access used by
    /// the application boundary. The existing handle-less fixture fallback
    /// stays owned by InteractionState and keeps its feature gate.
    pub fn set_trainer_interaction_role_with_access_like_cpp(
        &mut self,
        role: &TrainerInteractionRoleAccessLikeCpp<'_>,
        source_guid: ObjectGuid,
        trainer_id: u32,
    ) -> bool {
        let canonical = role.set_trainer_interaction_like_cpp(source_guid, trainer_id);
        #[cfg(any(test, feature = "test-fixtures"))]
        if canonical || role.player_handle_absent_like_cpp() {
            self.player_interaction_data_like_cpp
                .set_trainer(source_guid, trainer_id);
        }
        canonical
            || cfg!(any(test, feature = "test-fixtures"))
                && role.player_handle_absent_like_cpp()
    }

    /// Re-read the current trainer role through Core on every call. A prior
    /// match is not retained across an await or other admission boundary.
    pub fn trainer_interaction_role_matches_with_access_like_cpp(
        &self,
        role: &TrainerInteractionRoleAccessLikeCpp<'_>,
        source_guid: ObjectGuid,
        trainer_id: i32,
    ) -> bool {
        let canonical = role.trainer_interaction_matches_like_cpp(source_guid, trainer_id);
        #[cfg(any(test, feature = "test-fixtures"))]
        if canonical.is_none() && role.player_handle_absent_like_cpp() {
            return self
                .player_interaction_data_like_cpp
                .trainer_matches(source_guid, trainer_id);
        }
        canonical.unwrap_or(false)
    }

    fn resolved_player_interaction_data_with_access_like_cpp(
        &self,
        access: &wow_world_core::session::NpcInteractionAccessLikeCpp<'_>,
    ) -> Option<PlayerInteractionDataLikeCpp> {
        let canonical = access.player_interaction_data_snapshot_like_cpp();
        #[cfg(any(test, feature = "test-fixtures"))]
        if canonical.is_none() && access.player_handle_absent_like_cpp() {
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
        let role = hub.core.trainer_interaction_role_access_like_cpp();
        self.set_trainer_interaction_role_with_access_like_cpp(&role, source_guid, trainer_id)
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
        let access = hub.trainer_npc_interaction_access_like_cpp();
        self.player_interaction_source_guid_with_access_like_cpp(&access)
    }

    pub fn player_interaction_source_guid_with_access_like_cpp(
        &self,
        access: &wow_world_core::session::NpcInteractionAccessLikeCpp<'_>,
    ) -> Option<ObjectGuid> {
        let interaction = self.resolved_player_interaction_data_with_access_like_cpp(access)?;
        (!interaction.source_guid.is_empty()).then_some(interaction.source_guid)
    }

    pub fn resolved_player_interaction_trainer_id_like_cpp(
        &self,
        hub: HubRef<'_>,
    ) -> Option<u32> {
        let access = hub.trainer_npc_interaction_access_like_cpp();
        self.resolved_player_interaction_trainer_id_with_access_like_cpp(&access)
    }

    pub fn resolved_player_interaction_trainer_id_with_access_like_cpp(
        &self,
        access: &wow_world_core::session::NpcInteractionAccessLikeCpp<'_>,
    ) -> Option<u32> {
        self.resolved_player_interaction_data_with_access_like_cpp(access)
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
        let role = hub.core.trainer_interaction_role_access_like_cpp();
        self.trainer_interaction_role_matches_with_access_like_cpp(
            &role,
            source_guid,
            trainer_id,
        )
    }
}
