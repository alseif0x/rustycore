// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! World-side adapter for the world-state and instance-admission bodies that
//! moved to `wow_world_instances::instance_session` (#1363 slice 11b). Each
//! method delegates to the same-named session operation (or field or split).

use std::sync::Arc;

use wow_world_core::session::{HubMut, HubRef};
use wow_world_instances::{InstanceSessionHostLikeCpp, InstanceState};

use super::*;

impl InstanceSessionHostLikeCpp for WorldSession {
    fn split_instances_ref(&self) -> (&InstanceState, HubRef<'_>) {
        crate::session::split_instances_ref(self)
    }
    fn split_instances_mut(&mut self) -> (&mut InstanceState, HubMut<'_>) {
        crate::session::split_instances_mut(self)
    }
    fn is_pvp_realm_like_cpp(&self) -> bool {
        self.view.is_pvp_realm_like_cpp
    }
    fn set_represented_rest_flag_like_cpp(&mut self, rest_flag: u32, trigger_id: u32) -> bool {
        WorldSession::set_represented_rest_flag_like_cpp(self, rest_flag, trigger_id)
    }
    fn remove_represented_rest_flag_like_cpp(&mut self, rest_flag: u32) -> bool {
        WorldSession::remove_represented_rest_flag_like_cpp(self, rest_flag)
    }
    fn defer_player_rest_flag_sync_like_cpp(&mut self) -> bool {
        WorldSession::defer_player_rest_flag_sync_like_cpp(self)
    }
    fn end_player_rest_flag_sync_like_cpp(&mut self) -> bool {
        WorldSession::end_player_rest_flag_sync_like_cpp(self)
    }
    fn clear_player_deferred_rest_flag_update_like_cpp(&mut self) -> bool {
        WorldSession::clear_player_deferred_rest_flag_update_like_cpp(self)
    }
    fn send_represented_resting_player_flag_update_like_cpp(&self) -> bool {
        WorldSession::send_represented_resting_player_flag_update_like_cpp(self)
    }
    fn access_requirement_abort_like_cpp(
        &self,
        map_id: u32,
        requested_difficulty: u8,
    ) -> Option<(u32, u8, i32)> {
        WorldSession::access_requirement_abort_like_cpp(self, map_id, requested_difficulty)
    }
    fn current_player_is_in_raid_group_like_cpp(&self) -> bool {
        WorldSession::current_player_is_in_raid_group_like_cpp(self)
    }
    fn authoritative_group_membership_like_cpp(&self) -> Option<u64> {
        WorldSession::authoritative_group_membership_like_cpp(self)
    }
    fn player_loading(&self) -> Option<ObjectGuid> {
        WorldSession::player_loading(self)
    }
    fn ensure_canonical_player_owner_exists_like_cpp(&mut self, key: wow_map::MapKey) -> bool {
        WorldSession::ensure_canonical_player_owner_exists_like_cpp(self, key)
    }
}
