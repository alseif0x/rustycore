// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1241 F3 test shims: WorldSession entry points kept only for unit_tests callers.

#[allow(unused_imports)]
use super::*;

impl crate::session::WorldSession {
    #[cfg(test)]
    pub(crate) fn player_interaction_trainer_id_like_cpp(&self) -> u32 {
        let (state, hub) = crate::session::split_interaction_ref(self);
        state.player_interaction_trainer_id_like_cpp(hub)
    }
    pub(crate) fn represented_npc_can_interact_with_like_cpp(
        &self,
        guid: ObjectGuid,
        npc_flags: u32,
        npc_flags2: u32,
    ) -> Option<RepresentedCreatureAccessLikeCpp> {
        crate::session::hub_ref(self)
            .represented_npc_can_interact_with_like_cpp(guid, npc_flags, npc_flags2)
    }
}
