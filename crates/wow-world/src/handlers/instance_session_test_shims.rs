// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Test-only method entry points for the world-state and instance-admission
//! bodies moved to `wow_world_instances::instance_session` (#1363 slice 11b):
//! one-call method forwards to the moved free functions the scenarios and F3
//! shims call, with this session as the host, so no call site changes; and
//! the two split wrappers of `session/instances/instance.rs` that only the
//! scenarios still call, moved verbatim.

#![cfg(test)]

use crate::session::*;
use wow_core::ObjectGuid;
use wow_entities::PLAYER_EXPLORED_ZONES_SIZE_LIKE_CPP;

impl WorldSession {
    pub(crate) fn player_explored_zones_snapshot_like_cpp(
        &self,
    ) -> Option<[u64; PLAYER_EXPLORED_ZONES_SIZE_LIKE_CPP]> {
        wow_world_instances::player_explored_zones_snapshot_like_cpp(self)
    }
    pub(crate) fn update_area_represented_with_rest_update_like_cpp(
        &mut self,
        new_area: u32,
        send_rest_update: bool,
    ) -> bool {
        wow_world_instances::update_area_represented_with_rest_update_like_cpp(
            self,
            new_area,
            send_rest_update,
        )
    }
    pub(crate) fn update_zone_represented_like_cpp(
        &mut self,
        new_zone: u32,
        new_area: u32,
    ) -> bool {
        wow_world_instances::update_zone_represented_like_cpp(self, new_zone, new_area)
    }
    pub(crate) fn update_zone_represented_without_rest_update_packet_like_cpp(
        &mut self,
        new_zone: u32,
        new_area: u32,
    ) -> bool {
        wow_world_instances::update_zone_represented_without_rest_update_packet_like_cpp(
            self, new_zone, new_area,
        )
    }
    pub(crate) fn set_represented_tavern_resting_like_cpp(
        &mut self,
        trigger_id: u32,
        entered: bool,
    ) -> bool {
        wow_world_instances::set_represented_tavern_resting_like_cpp(self, trigger_id, entered)
    }
    pub(crate) fn apply_create_map_side_effects_like_cpp(
        &mut self,
        map_id: u32,
        decision: &wow_map::CreateMapDecision,
    ) -> CreateMapSideEffectApplySummaryLikeCpp {
        wow_world_instances::apply_create_map_side_effects_like_cpp(self, map_id, decision)
    }
    pub(crate) fn create_map_player_context_like_cpp(
        &self,
        map_id: u32,
        map_entry: wow_data::map::MapEntry,
        player_guid: ObjectGuid,
    ) -> Option<wow_map::CreateMapPlayerContext> {
        wow_world_instances::create_map_player_context_like_cpp(
            self,
            map_id,
            map_entry,
            player_guid,
        )
    }
    pub(crate) fn create_map_active_instance_lock_context_like_cpp(
        &self,
        map_id: u32,
        difficulty_id: wow_map::Difficulty,
    ) -> Option<wow_map::CreateMapInstanceLockContext> {
        wow_world_instances::create_map_active_instance_lock_context_like_cpp(
            self,
            map_id,
            difficulty_id,
        )
    }
    pub(crate) fn create_map_instance_owner_guid_like_cpp(
        &self,
        map_id: u32,
    ) -> Option<ObjectGuid> {
        wow_world_instances::create_map_instance_owner_guid_like_cpp(self, map_id)
    }
    pub(crate) fn resolved_player_recent_instance_id_like_cpp(&self, map_id: u32) -> Option<u32> {
        let (state, hub) = crate::session::split_instances_ref(self);
        state.resolved_player_recent_instance_id_like_cpp(hub, map_id)
    }
    pub(crate) fn set_represented_player_recent_instance_like_cpp(
        &mut self,
        map_id: u32,
        instance_id: u32,
    ) -> bool {
        let (state, mut hub) = crate::session::split_instances_mut(self);
        state.set_represented_player_recent_instance_like_cpp(&mut hub, map_id, instance_id)
    }
}
