// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Test-only method entry points for the player cast, the spell handlers,
//! the spell-click plan, the respec wipe and the aura modifier moved to
//! `wow_world_spell` (#1363 slice 9e).
//!
//! The first impl block holds the `#[cfg(test)]` overloads that were nested in
//! the moved production files, verbatim except that their calls to the moved
//! entry points are free calls: they were already test-only. The second holds
//! one-call method forwards for the moved functions the scenarios call, with
//! this session as the host, so no scenario call site changes.

#![cfg(test)]

use crate::session::*;
use wow_core::ObjectGuid;
use wow_world_core::session::mailbox::SendPlayerSpellIfVisibleLikeCppCommand;

impl WorldSession {
    pub(crate) async fn tick_active_spell_cast(&mut self) {
        let generators = self.id_generators_for_test_like_cpp();
        let creature_spawn_catalogs = self.creature_spawn_catalogs_for_test_like_cpp();
        wow_world_spell::tick_active_spell_cast_with_generator_like_cpp(
            self,
            generators.item.as_ref(),
            &creature_spawn_catalogs,
        )
        .await;
    }
    pub(crate) async fn handle_cast_spell(&mut self, pkt: wow_packet::WorldPacket) {
        let area_trigger_catalogs = self.area_trigger_catalogs_for_test_like_cpp();
        let creature_spawn_catalogs = self.creature_spawn_catalogs_for_test_like_cpp();
        let progression = self.progression_catalogs_for_test_like_cpp();
        let generators = self.id_generators_for_test_like_cpp();
        wow_world_spell::handle_cast_spell_with_catalogs_like_cpp(
            self,
            &area_trigger_catalogs,
            &creature_spawn_catalogs,
            &progression,
            &crate::session::SessionHandlerCatalogsLikeCpp::default().player_grid_loader,
            generators.item.as_ref(),
            pkt,
        )
        .await;
    }
    pub(crate) async fn handle_self_res(&mut self, pkt: wow_packet::WorldPacket) {
        let generators = self.id_generators_for_test_like_cpp();
        let creature_spawn_catalogs = self.creature_spawn_catalogs_for_test_like_cpp();
        wow_world_spell::handle_self_res_with_generator_like_cpp(
            self,
            generators.item.as_ref(),
            &creature_spawn_catalogs,
            pkt,
        )
        .await;
    }
    pub(crate) async fn execute_represented_spell_click_plan_like_cpp(
        &mut self,
        creature_guid: ObjectGuid,
        plan: &RepresentedSpellClickPlanLikeCpp,
    ) -> RepresentedSpellClickExecutionOutcomeLikeCpp {
        let generators = self.id_generators_for_test_like_cpp();
        let creature_spawn_catalogs = self.creature_spawn_catalogs_for_test_like_cpp();
        wow_world_spell::execute_represented_spell_click_plan_with_generator_like_cpp(
            self,
            generators.item.as_ref(),
            &creature_spawn_catalogs,
            creature_guid,
            plan,
        )
        .await
    }
}

impl WorldSession {
    pub(crate) fn apply_represented_aura_modifier_like_cpp(
        &mut self,
        spell_id: i32,
        caster_guid: ObjectGuid,
        effect: &wow_data::SpellEffectInfo,
        represented_effect: RepresentedAuraEffectLikeCpp,
        duration_ms: u32,
    ) -> Result<(), &'static str> {
        <Self as wow_world_spell::AuraHostLikeCpp>::apply_represented_aura_modifier_like_cpp(
            self,
            spell_id,
            caster_guid,
            effect,
            represented_effect,
            duration_ms,
        )
    }
    pub(crate) fn search_spell_focus_like_cpp(
        &self,
        focus_id: u32,
    ) -> Option<RepresentedSpellFocusObjectLikeCpp> {
        let (state, hub) = crate::session::split_spell_state_ref(self);
        state.search_spell_focus_like_cpp(hub, focus_id)
    }
    pub(crate) fn take_ready_player_cast_like_cpp(&mut self) -> Option<SpellCastState> {
        wow_world_spell::take_ready_player_cast_like_cpp(self)
    }
    pub(crate) fn handle_player_cast_publication_like_cpp(
        &mut self,
        command: SendPlayerSpellIfVisibleLikeCppCommand,
    ) {
        wow_world_spell::handle_player_cast_publication_like_cpp(self, command)
    }
    pub(crate) fn represented_handle_spell_click_plan_like_cpp(
        &self,
        creature_guid: ObjectGuid,
    ) -> RepresentedSpellClickPlanLikeCpp {
        wow_world_spell::represented_handle_spell_click_plan_like_cpp(self, creature_guid)
    }
    pub(crate) fn represented_handle_spell_click_plan_with_seat_like_cpp(
        &self,
        creature_guid: ObjectGuid,
        seat_id: Option<i8>,
    ) -> RepresentedSpellClickPlanLikeCpp {
        wow_world_spell::represented_handle_spell_click_plan_with_seat_like_cpp(
            self,
            creature_guid,
            seat_id,
        )
    }
}
