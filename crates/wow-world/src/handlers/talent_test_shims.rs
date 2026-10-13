// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Test-only method entry points for the talent bodies moved to
//! `wow_world_spell` (#1363 slice 9b).
//!
//! The first impl block holds the `#[cfg(test)]` method of
//! `handlers/talent/state.rs`, verbatim: it was already test-only. The second
//! holds the talents' #1241 F3 test entry points (from
//! `unit_tests/session/progression/talents/f3_shims.rs`), verbatim except that
//! `pub(in crate::session)` reads `pub(crate)`. The third holds one-call method
//! forwards for the moved functions the scenarios and these fixtures call,
//! with this session as the host, so no scenario call site changes.

#![cfg(test)]

use crate::session::*;
use wow_data::TalentTabStore;
use wow_packet::WorldPacket;

impl WorldSession {
    pub(crate) async fn handle_confirm_respec_wipe(&mut self, packet: WorldPacket) {
        let generators = self.id_generators_for_test_like_cpp();
        self.handle_confirm_respec_wipe_with_generator_like_cpp(
            generators.item.as_ref(),
            false,
            packet,
        )
        .await;
    }
}

impl WorldSession {
    #[cfg(test)]
    pub(crate) fn mutate_player_talent_runtime_for_test_like_cpp<R>(
        &mut self,
        apply: impl FnOnce(&mut wow_entities::PlayerTalentRuntimeState) -> R,
    ) -> Option<R> {
        crate::session::hub_mut(self).mutate_player_talent_runtime_for_test_like_cpp(apply)
    }
    #[cfg(test)]
    pub(crate) fn represented_update_talent_data_packet_like_cpp(
        &self,
    ) -> wow_packet::packets::misc::UpdateTalentData {
        crate::session::hub_ref(self).represented_update_talent_data_packet_like_cpp()
    }
    pub(crate) fn represented_talent_reset_cost_like_cpp(&self) -> Option<u32> {
        crate::session::hub_ref(self).represented_talent_reset_cost_like_cpp()
    }
    pub(crate) fn represented_talent_reset_time_secs_like_cpp(&self) -> Option<u64> {
        crate::session::hub_ref(self).represented_talent_reset_time_secs_like_cpp()
    }
    #[cfg(test)]
    pub(crate) fn represented_talent_reset_script_hooks_like_cpp(
        &self,
    ) -> &[RepresentedTalentResetScriptHookLikeCpp] {
        self.fixtures
            .progression
            .represented_talent_reset_script_hooks_like_cpp()
    }
    #[cfg(test)]
    pub(crate) fn represented_talent_respec_criteria_events_like_cpp(
        &self,
    ) -> &[RepresentedTalentRespecCriteriaEventLikeCpp] {
        self.fixtures
            .progression
            .represented_talent_respec_criteria_events_like_cpp()
    }
    pub(crate) fn reset_represented_glyphs_like_cpp(&mut self) {
        crate::session::hub_mut(self).reset_represented_glyphs_like_cpp()
    }
    pub(crate) fn represented_talent_info_like_cpp(
        &self,
        talent_id: u32,
        rank: u8,
    ) -> Option<wow_packet::packets::misc::TalentInfoLikeCpp> {
        crate::session::hub_ref(self).represented_talent_info_like_cpp(talent_id, rank)
    }
    pub(crate) fn represented_next_reset_talents_cost_like_cpp(
        &self,
        now_secs: u64,
    ) -> Option<u32> {
        crate::session::hub_ref(self).represented_next_reset_talents_cost_like_cpp(now_secs)
    }
    pub(crate) fn player_talent_runtime_snapshot_like_cpp(
        &self,
    ) -> Option<wow_entities::PlayerTalentRuntimeState> {
        crate::session::hub_ref(self).player_talent_runtime_snapshot_like_cpp()
    }
    pub(crate) fn set_represented_talent_reset_state_like_cpp(
        &mut self,
        reset_cost: u32,
        reset_time_secs: u64,
    ) -> bool {
        crate::session::hub_mut(self)
            .set_represented_talent_reset_state_like_cpp(reset_cost, reset_time_secs)
    }
}

impl WorldSession {
    pub(crate) async fn handle_learn_talent(
        &mut self,
        talent_tabs: &TalentTabStore,
        packet: WorldPacket,
    ) {
        wow_world_spell::handle_learn_talent(self, talent_tabs, packet).await
    }
    pub(crate) async fn handle_confirm_respec_wipe_with_generator_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        no_reset_talent_cost: bool,
        packet: WorldPacket,
    ) {
        wow_world_spell::handle_confirm_respec_wipe_with_generator_like_cpp(
            self,
            item_guid_generator,
            no_reset_talent_cost,
            packet,
        )
        .await
    }
    pub(crate) fn reset_represented_talents_like_cpp(&mut self) {
        wow_world_spell::reset_represented_talents_like_cpp(self)
    }
    pub(crate) fn apply_represented_login_talent_reset_if_needed_like_cpp(&mut self) -> bool {
        wow_world_spell::apply_represented_login_talent_reset_if_needed_like_cpp(self)
    }
    pub(crate) fn learn_represented_talent_like_cpp(
        &mut self,
        talent_tabs: &TalentTabStore,
        talent_id: u32,
        requested_rank: u16,
    ) -> bool {
        wow_world_spell::learn_represented_talent_like_cpp(
            self,
            talent_tabs,
            talent_id,
            requested_rank,
        )
    }
}
