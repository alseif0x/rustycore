// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Test-only method entry points for the map entry moved to
//! `wow_world_instances::instance_session` and the far-transfer completion
//! moved to `wow_world_character::worldport_session` (#1363 slice 13a):
//! one-call method forwards to the moved free functions the scenarios call,
//! with this session as the host, so no call site changes; and the two
//! instance split wrappers of `session/instances/{instance,difficulty}.rs` and
//! the two account-collection wrappers of `session/lifecycle_ops/operations.rs`
//! that only the scenarios still call, moved verbatim.

#![cfg(test)]

use crate::session::*;
use wow_core::Position;

impl WorldSession {
    pub(crate) fn try_attach_worldport_destination_like_cpp(
        &mut self,
        map_id: u32,
        position: wow_core::Position,
    ) -> bool {
        wow_world_instances::try_attach_worldport_destination_like_cpp(self, map_id, position)
    }
    pub(crate) fn prepare_canonical_map_entry_like_cpp(
        &mut self,
        map_id: u32,
    ) -> Option<wow_map::CreateMapDecision> {
        wow_world_instances::prepare_canonical_map_entry_like_cpp(self, map_id)
    }
    pub(crate) fn ensure_canonical_player_owner_for_map_like_cpp(
        &mut self,
        key: wow_map::MapKey,
        position: Position,
    ) -> bool {
        wow_world_instances::ensure_canonical_player_owner_for_map_like_cpp(self, key, position)
    }
    pub(crate) fn add_instance_enter_time_like_cpp(&mut self, instance_id: u32, enter_time: u64) {
        let (state, mut hub) = crate::session::split_instances_mut(self);
        state.add_instance_enter_time_like_cpp(&mut hub, instance_id, enter_time)
    }
    pub(crate) fn create_map_difficulty_context_like_cpp(
        &self,
        map_id: u32,
        difficulty_id: wow_map::Difficulty,
    ) -> Option<wow_map::CreateMapDifficultyContext> {
        let (state, hub) = crate::session::split_instances_ref(self);
        state.create_map_difficulty_context_like_cpp(hub, map_id, difficulty_id)
    }
    pub(crate) async fn finish_worldport_native_before_disconnect_like_cpp(&mut self) -> bool {
        let generators = self.id_generators_for_test_like_cpp();
        let creature_spawns = self.creature_spawn_catalogs_for_test_like_cpp();
        wow_world_character::finish_worldport_native_before_disconnect_like_cpp(
            self,
            wow_world_entities::ResurrectionOperationCatalogs {
                item_guid_generator: generators.item.as_ref(),
                creature_spawns: &creature_spawns,
                graveyards: None,
            },
        )
        .await
    }
    pub(crate) fn begin_worldport_post_add_like_cpp(
        &mut self,
        map_id: u32,
        position: Position,
    ) -> bool {
        wow_world_character::begin_worldport_post_add_like_cpp(self, map_id, position)
    }
    pub(crate) fn account_heirloom_active_player_rows_like_cpp(&self) -> Vec<(i32, u32)> {
        let (state, hub) = crate::session::split_lifecycle_ref(self);
        state.account_heirloom_active_player_rows_like_cpp(hub)
    }
    pub(crate) fn account_toy_active_player_rows_like_cpp(&self) -> Vec<i32> {
        let (state, hub) = crate::session::split_lifecycle_ref(self);
        state.account_toy_active_player_rows_like_cpp(hub)
    }
}
