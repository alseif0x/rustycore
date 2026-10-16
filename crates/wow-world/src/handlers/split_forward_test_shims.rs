// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Test-only method entry points for the World forwards and bodies retired or
//! moved in #1363 slice 12c: one-call forwards (or the retired split forward
//! verbatim) for the names the scenarios call, so no scenario call site changes.
#![cfg(test)]

use crate::session::WorldSession;
use wow_world_core::session::PlayerRestRatePolicyLikeCpp;

impl WorldSession {
    // Rest flags (moved to `wow_world_instances::instance_session::rest_flags`).
    pub(crate) fn with_owned_player_mut_for_rest_like_cpp<R>(
        &self,
        f: impl FnOnce(&mut wow_entities::Player) -> R,
    ) -> Option<R> {
        wow_world_instances::with_owned_player_mut_for_rest_like_cpp(self, f)
    }
    pub(crate) fn set_represented_rest_flag_like_cpp(
        &mut self,
        rest_flag: u32,
        trigger_id: u32,
    ) -> bool {
        wow_world_instances::set_player_rest_flag_like_cpp(self, rest_flag, trigger_id)
    }
    pub(crate) fn remove_represented_rest_flag_like_cpp(&mut self, rest_flag: u32) -> bool {
        wow_world_instances::remove_player_rest_flag_like_cpp(self, rest_flag)
    }
    pub(crate) fn take_deferred_rest_flag_update_dirty_like_cpp(&mut self) -> bool {
        wow_world_instances::take_player_deferred_rest_flag_update_dirty_like_cpp(self)
    }
    pub(crate) fn send_represented_resting_player_flag_update_like_cpp(&self) -> bool {
        let (state, hub) = crate::session::split_inventory_ref(self);
        state.send_represented_resting_player_flag_update_like_cpp(hub)
    }
    // Rest bonus (moved to `wow_world_instances::instance_session::rest_bonus`).
    pub(crate) fn represented_recruit_a_friend_xp_rest_state_applies_like_cpp(&self) -> bool {
        <Self as wow_world_instances::InstanceSessionHostLikeCpp>::represented_recruit_a_friend_xp_rest_state_applies_like_cpp(self)
    }
    pub(crate) fn apply_offline_xp_rest_bonus_with_policy_like_cpp(
        &mut self,
        policy: &PlayerRestRatePolicyLikeCpp,
        logout_time_secs: u64,
        now_secs: u64,
        was_logout_resting: bool,
    ) -> f32 {
        wow_world_instances::apply_offline_xp_rest_bonus_with_policy_like_cpp(
            self,
            policy,
            logout_time_secs,
            now_secs,
            was_logout_resting,
        )
    }
    pub(crate) fn update_represented_online_xp_rest_bonus_with_policy_like_cpp(
        &mut self,
        policy: &PlayerRestRatePolicyLikeCpp,
        now_secs: u64,
    ) -> (f32, u8) {
        wow_world_instances::update_represented_online_xp_rest_bonus_with_policy_like_cpp(
            self, policy, now_secs,
        )
    }
    pub(crate) fn tick_represented_online_xp_rest_bonus_with_roll_and_policy_like_cpp(
        &mut self,
        policy: &PlayerRestRatePolicyLikeCpp,
        now_secs: u64,
        update_roll_passed: bool,
    ) {
        wow_world_instances::tick_represented_online_xp_rest_bonus_with_roll_and_policy_like_cpp(
            self,
            policy,
            now_secs,
            update_roll_passed,
        )
    }
    // Player items (the fully-looted removal moved to `wow_world_loot_ops`).
    pub(crate) fn remove_fully_looted_runtime_item(
        &mut self,
        bag: u8,
        slot: u8,
        item_guid: wow_core::ObjectGuid,
    ) {
        wow_world_loot_ops::remove_fully_looted_runtime_item(self, bag, slot, item_guid)
    }
    pub(crate) fn remove_inventory_item_object(
        &mut self,
        item_guid: wow_core::ObjectGuid,
    ) -> Option<wow_entities::Item> {
        let (state, mut hub) = crate::session::split_inventory_mut(self);
        state.remove_inventory_item_object(&mut hub, item_guid)
    }
    pub(crate) fn remove_inventory_item_like_cpp(
        &mut self,
        slot: u8,
    ) -> Option<wow_entities::PlayerInventoryItem> {
        let (state, mut hub) = crate::session::split_inventory_mut(self);
        state.remove_inventory_item_like_cpp(&mut hub, slot)
    }
}
