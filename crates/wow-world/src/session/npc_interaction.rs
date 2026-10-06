// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Npc interaction: private Session responsibility.
//! Relocated under #1233; canonical state, phase order and public paths are unchanged.

use super::{ObjectGuid, WorldSession};
#[cfg(test)]
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

#[cfg(test)]
#[path = "../../unit_tests/session/npc_interaction/f3_shims.rs"]
mod f3_shims;
