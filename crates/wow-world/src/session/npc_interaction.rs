// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Npc interaction: private Session responsibility.
//! Relocated under #1233; canonical state, phase order and public paths are unchanged.

use super::{ObjectGuid, WorldSession};
#[cfg(test)]
pub(crate) use wow_world_core::session::RepresentedCreatureAccessLikeCpp;

impl WorldSession {
    pub(crate) fn player_interaction_source_guid_like_cpp(&self) -> Option<ObjectGuid> {
        let (state, hub) = crate::session::split_interaction_ref(self);
        state.player_interaction_source_guid_like_cpp(hub)
    }
}

#[cfg(test)]
#[path = "../../unit_tests/session/npc_interaction/f3_shims.rs"]
mod f3_shims;
