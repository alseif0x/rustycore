// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Test-only method entry points for the session leftovers moved out of the
//! World `session/**` in #1363 slice 12b7: one-call forwards to the moved
//! functions the scenarios call, so no scenario call site changes.
#![cfg(test)]

use crate::session::WorldSession;

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
}
