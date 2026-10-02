// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Npc interaction: private Session responsibility.
//! Relocated under #1233; canonical state, phase order and public paths are unchanged.

use super::{ObjectGuid, PlayerInteractionDataLikeCpp, WorldSession};
pub(crate) use wow_world_core::session::RepresentedCreatureAccessLikeCpp;

impl WorldSession {
    pub(crate) fn set_player_interaction_source_like_cpp(
        &mut self,
        source_guid: ObjectGuid,
    ) -> bool {
        let (state, hub) = crate::session::split_interaction(self);
        state.set_player_interaction_source_like_cpp(hub, source_guid)
    }

    pub(crate) fn set_player_trainer_interaction_like_cpp(
        &mut self,
        source_guid: ObjectGuid,
        trainer_id: u32,
    ) -> bool {
        let (state, hub) = crate::session::split_interaction(self);
        state.set_player_trainer_interaction_like_cpp(hub, source_guid, trainer_id)
    }

    pub(crate) fn reset_player_interaction_if_source_like_cpp(
        &mut self,
        source_guid: ObjectGuid,
    ) -> bool {
        let (state, hub) = crate::session::split_interaction(self);
        state.reset_player_interaction_if_source_like_cpp(hub, source_guid)
    }

    pub(crate) fn player_interaction_source_guid_like_cpp(&self) -> Option<ObjectGuid> {
        let (state, hub) = crate::session::split_interaction_ref(self);
        state.player_interaction_source_guid_like_cpp(hub)
    }

    pub(crate) fn resolved_player_interaction_trainer_id_like_cpp(&self) -> Option<u32> {
        let (state, hub) = crate::session::split_interaction_ref(self);
        state.resolved_player_interaction_trainer_id_like_cpp(hub)
    }

    pub(crate) fn player_trainer_interaction_matches_like_cpp(
        &self,
        source_guid: ObjectGuid,
        trainer_id: i32,
    ) -> bool {
        let (state, hub) = crate::session::split_interaction_ref(self);
        state.player_trainer_interaction_matches_like_cpp(hub, source_guid, trainer_id)
    }
}

impl crate::session::state::InteractionState {
    pub(in crate::session) fn resolved_player_interaction_data_like_cpp(
        &self,
        hub: crate::session::HubRef<'_>,
    ) -> Option<PlayerInteractionDataLikeCpp> {
        let canonical = hub
            .core
            .with_owned_player_like_cpp(|player| *player.interaction_data_like_cpp());
        #[cfg(test)]
        if canonical.is_none() && hub.core.player_handle_like_cpp.is_none() {
            return Some(self.player_interaction_data_like_cpp);
        }
        canonical
    }

    pub(crate) fn reset_player_interaction_data_like_cpp(
        &mut self,
        hub: crate::session::HubRef<'_>,
    ) -> bool {
        let canonical = hub
            .core
            .with_owned_player_mut_like_cpp(|player| player.reset_interaction_data_like_cpp())
            .is_some();
        #[cfg(test)]
        if canonical || hub.core.player_handle_like_cpp.is_none() {
            self.player_interaction_data_like_cpp.reset();
        }
        canonical || cfg!(test) && hub.core.player_handle_like_cpp.is_none()
    }

    pub(crate) fn set_player_interaction_source_like_cpp(
        &mut self,
        hub: crate::session::HubRef<'_>,
        source_guid: ObjectGuid,
    ) -> bool {
        let canonical = hub
            .core
            .with_owned_player_mut_like_cpp(|player| {
                player.set_interaction_source_like_cpp(source_guid);
            })
            .is_some();
        #[cfg(test)]
        if canonical || hub.core.player_handle_like_cpp.is_none() {
            self.player_interaction_data_like_cpp
                .set_source(source_guid);
        }
        canonical || cfg!(test) && hub.core.player_handle_like_cpp.is_none()
    }

    pub(crate) fn set_player_trainer_interaction_like_cpp(
        &mut self,
        hub: crate::session::HubRef<'_>,
        source_guid: ObjectGuid,
        trainer_id: u32,
    ) -> bool {
        let canonical = hub
            .core
            .with_owned_player_mut_like_cpp(|player| {
                player.set_trainer_interaction_like_cpp(source_guid, trainer_id);
            })
            .is_some();
        #[cfg(test)]
        if canonical || hub.core.player_handle_like_cpp.is_none() {
            self.player_interaction_data_like_cpp
                .set_trainer(source_guid, trainer_id);
        }
        canonical || cfg!(test) && hub.core.player_handle_like_cpp.is_none()
    }

    pub(crate) fn reset_player_interaction_if_source_like_cpp(
        &mut self,
        hub: crate::session::HubRef<'_>,
        source_guid: ObjectGuid,
    ) -> bool {
        let canonical = hub.core.with_owned_player_mut_like_cpp(|player| {
            player.reset_interaction_if_source_like_cpp(source_guid)
        });
        #[cfg(test)]
        if canonical.is_some() || hub.core.player_handle_like_cpp.is_none() {
            let fixture = self
                .player_interaction_data_like_cpp
                .reset_if_source(source_guid);
            return canonical.unwrap_or(fixture);
        }
        canonical.unwrap_or(false)
    }

    pub(crate) fn player_interaction_source_guid_like_cpp(
        &self,
        hub: crate::session::HubRef<'_>,
    ) -> Option<ObjectGuid> {
        let interaction = self.resolved_player_interaction_data_like_cpp(hub)?;
        (!interaction.source_guid.is_empty()).then_some(interaction.source_guid)
    }

    pub(crate) fn resolved_player_interaction_trainer_id_like_cpp(
        &self,
        hub: crate::session::HubRef<'_>,
    ) -> Option<u32> {
        self.resolved_player_interaction_data_like_cpp(hub)
            .map(|interaction| interaction.trainer_id)
    }

    #[cfg(test)]
    pub(crate) fn player_interaction_trainer_id_like_cpp(
        &self,
        hub: crate::session::HubRef<'_>,
    ) -> u32 {
        self.resolved_player_interaction_trainer_id_like_cpp(hub)
            .unwrap_or(0)
    }

    pub(crate) fn player_trainer_interaction_matches_like_cpp(
        &self,
        hub: crate::session::HubRef<'_>,
        source_guid: ObjectGuid,
        trainer_id: i32,
    ) -> bool {
        self.resolved_player_interaction_data_like_cpp(hub)
            .is_some_and(|interaction| interaction.trainer_matches(source_guid, trainer_id))
    }
}

#[cfg(test)]
#[path = "../../unit_tests/session/npc_interaction/f3_shims.rs"]
mod f3_shims;
