// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1241 F3 test shims: WorldSession entry points kept only for unit_tests callers.

#[allow(unused_imports)]
use super::*;

impl crate::session::WorldSession {
    #[cfg(test)]
    pub(in crate::session) fn represented_player_collection_state_like_cpp(
        &self,
    ) -> wow_entities::PlayerCollectionStateLikeCpp {
        crate::session::hub_ref(self).represented_player_collection_state_like_cpp()
    }
    pub(in crate::session) fn player_collection_state_snapshot_like_cpp(
        &self,
    ) -> Option<wow_entities::PlayerCollectionStateLikeCpp> {
        crate::session::hub_ref(self).player_collection_state_snapshot_like_cpp()
    }
    pub(crate) fn add_player_toy_dynamic_field_like_cpp(
        &mut self,
        item_id: u32,
    ) -> Option<wow_entities::PlayerValuesUpdate> {
        crate::session::hub_mut(self).add_player_toy_dynamic_field_like_cpp(item_id)
    }
    pub(crate) fn add_player_heirloom_dynamic_fields_like_cpp(
        &mut self,
        item_id: u32,
        flags: u32,
    ) -> Option<wow_entities::PlayerValuesUpdate> {
        crate::session::hub_mut(self).add_player_heirloom_dynamic_fields_like_cpp(item_id, flags)
    }
}
