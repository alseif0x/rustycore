// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Test-only method entry points for the session leftovers moved out of the
//! World `session/**` in #1363 slice 12b7: one-call forwards to the moved
//! functions the scenarios call, so no scenario call site changes.
#![cfg(test)]

use crate::session::{SKILL_RIDING_LIKE_CPP, WorldSession};

impl WorldSession {
    pub(crate) fn apply_represented_live_intent_like_cpp(
        &mut self,
        intent: wow_world_core::session::RepresentedLiveIntentLikeCpp,
    ) -> wow_world_core::session::RepresentedLiveIntentApplyOutcomeLikeCpp {
        wow_world_application::apply_represented_live_intent_like_cpp(self, intent)
    }
    pub(crate) fn interrupt_non_melee_spell_cast_for_loot_like_cpp(&mut self) -> bool {
        wow_world_application::interrupt_non_melee_spell_cast_for_loot_like_cpp(self)
    }
    pub(crate) fn pending_spell_cast_snapshot_like_cpp(
        &self,
    ) -> Option<Option<crate::session::RepresentedPendingSpellCastRequestLikeCpp>> {
        wow_world_application::pending_spell_cast_snapshot_like_cpp(self)
    }
    pub(crate) fn remaining_active_spell_cast_ms_like_cpp(&self) -> Option<u32> {
        wow_world_application::remaining_active_spell_cast_ms_like_cpp(self)
    }
    pub(crate) fn request_represented_spell_cast_like_cpp(
        &mut self,
        request: crate::session::RepresentedPendingSpellCastRequestLikeCpp,
    ) {
        wow_world_application::request_represented_spell_cast_like_cpp(self, request)
    }
    pub(crate) async fn tick_pending_spell_cast_request_with_generator_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        creature_spawn_catalogs: &wow_world_entities::CreatureSpawnCatalogsLikeCpp,
    ) {
        wow_world_application::tick_pending_spell_cast_request_with_generator_like_cpp(
            self,
            item_guid_generator,
            creature_spawn_catalogs,
        )
        .await
    }
    pub(crate) fn cancel_pending_spell_cast_request_like_cpp(&mut self) -> bool {
        let (state, mut hub) = crate::session::split_spell_state_mut(self);
        wow_world_application::cancel_pending_spell_cast_request_like_cpp(&mut hub, state)
    }
    pub(crate) fn update_player_collision_height_like_cpp(&mut self) {
        wow_world_spell::update_player_collision_height_like_cpp(self)
    }
    pub(crate) fn represented_mount_capability_selection_for_type_like_cpp(
        &self,
        mount_type_id: u16,
        riding_skill: u32,
        mount_restriction_flags: Option<u8>,
        is_submerged: bool,
        is_in_water: bool,
    ) -> Result<wow_data::MountCapabilityEntry, wow_data::MountCapabilityRejectLikeCpp> {
        wow_world_spell::represented_mount_capability_selection_for_type_like_cpp(
            self,
            mount_type_id,
            riding_skill,
            mount_restriction_flags,
            is_submerged,
            is_in_water,
        )
    }
    pub(crate) fn represented_mount_capability_for_type_like_cpp(
        &self,
        mount_type_id: u16,
        riding_skill: u32,
        mount_restriction_flags: Option<u8>,
        is_submerged: bool,
        is_in_water: bool,
    ) -> Option<wow_data::MountCapabilityEntry> {
        self.represented_mount_capability_selection_for_type_like_cpp(
            mount_type_id,
            riding_skill,
            mount_restriction_flags,
            is_submerged,
            is_in_water,
        )
        .ok()
    }
    #[allow(dead_code)]
    pub(crate) fn represented_mount_capability_for_type_from_session_like_cpp(
        &self,
        mount_type_id: u16,
        mount_restriction_flags: Option<u8>,
    ) -> Option<wow_data::MountCapabilityEntry> {
        let (is_submerged, is_in_water) =
            crate::session::hub_ref(self).represented_player_mount_liquid_state_like_cpp()?;
        self.represented_mount_capability_for_type_like_cpp(
            mount_type_id,
            u32::from(
                crate::session::hub_ref(self)
                    .resolved_player_skill_value_like_cpp(SKILL_RIDING_LIKE_CPP)?,
            ),
            mount_restriction_flags,
            is_submerged,
            is_in_water,
        )
    }
    pub(crate) fn player_spell_runtime_snapshot_like_cpp(
        &self,
    ) -> Option<wow_world_spell::RepresentedPlayerSpellRuntimeLikeCpp> {
        wow_world_spell::player_spell_runtime_snapshot_like_cpp(self)
    }
    pub(crate) fn with_player_spell_runtime_like_cpp<R>(
        &self,
        query: impl FnOnce(&wow_entities::PlayerSpellRuntimeState) -> R,
    ) -> Option<R> {
        wow_world_spell::with_player_spell_runtime_like_cpp(self, query)
    }
    pub(crate) fn resolved_group_guid_like_cpp(&self) -> Option<u64> {
        wow_world_instances::group_membership::resolved_group_guid_like_cpp(self)
    }
    pub(crate) fn send_new_item_plan(&self, plan: &wow_entities::SendNewItemPlan) {
        wow_world_inventory::send_new_item_plan(self, plan)
    }
}
