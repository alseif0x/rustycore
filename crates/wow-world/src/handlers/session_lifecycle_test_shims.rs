// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Test-only method entry points for the lifecycle orchestration moved to
//! `wow_world_application::session_lifecycle` (#1363 slice 13b): one-call
//! method forwards to the moved free functions and account saves the
//! scenarios call, with this session as the host, so no call site changes;
//! and the account, registry, logout-flag, realm-channel, buy-error and
//! talent-reset plan wrappers and the two disconnect-cleanup test entries of
//! `session/lifecycle/{account,cleanup,logout}.rs`,
//! `session/{connection,publication/operations}.rs` and
//! `session/persistence/plans.rs` that only the scenarios still call, moved
//! verbatim. It also mounts the `session/persistence/commit.rs` F3 shim.

#![cfg(test)]

use crate::finalization::{FinalizationMode, FinalizationOutcome, FinalizationStep};
use crate::session::{LifecycleCx, WorldSession};
use std::collections::BTreeSet;
use wow_constants::BuyResult;
use wow_core::ObjectGuid;
use wow_packet::WorldPacket;
use wow_world_application::{CommittedRepresentedTalentResetLikeCpp, SessionAccountSaveCxLikeCpp};
use wow_world_lifecycle::RepresentedTalentResetStatePlanLikeCpp;
use wow_world_loot_ops::wait_for_active_loot_persistence_with_generator_like_cpp;

#[path = "../../unit_tests/session/persistence/commit/f3_shims.rs"]
mod commit_f3_shims;

impl WorldSession {
    pub(crate) async fn execute_finalization_step(
        &mut self,
        step: FinalizationStep,
        mode: FinalizationMode,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
    ) -> FinalizationOutcome {
        wow_world_application::execute_finalization_step(self, step, mode, item_guid_generator)
            .await
    }
    pub(crate) async fn process_pending_periodic_player_save_with_generator_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
    ) {
        wow_world_application::process_pending_periodic_player_save_with_generator_like_cpp(
            self,
            item_guid_generator,
        )
        .await
    }
    pub(crate) async fn commit_represented_talent_reset_at_like_cpp(
        &mut self,
        now_secs: u64,
        no_reset_talent_cost: bool,
    ) -> Option<CommittedRepresentedTalentResetLikeCpp> {
        wow_world_application::commit_represented_talent_reset_at_like_cpp(
            self,
            now_secs,
            no_reset_talent_cost,
        )
        .await
    }
    pub(crate) async fn handle_confirm_respec_wipe_with_generator_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        no_reset_talent_cost: bool,
        packet: WorldPacket,
    ) {
        wow_world_application::handle_confirm_respec_wipe_with_generator_like_cpp(
            self,
            item_guid_generator,
            no_reset_talent_cost,
            packet,
        )
        .await
    }

    pub(crate) async fn mark_character_offline(&mut self) -> FinalizationOutcome {
        let (state, mut hub) = crate::session::split_lifecycle_mut(self);
        state.mark_character_offline(&mut hub).await
    }

    pub(crate) async fn mark_character_account_offline_like_cpp(&mut self) -> FinalizationOutcome {
        let (state, mut hub) = crate::session::split_lifecycle_mut(self);
        state
            .mark_character_account_offline_like_cpp(&mut hub)
            .await
    }

    pub(crate) async fn mark_login_account_offline_on_disconnect_like_cpp(
        &mut self,
    ) -> FinalizationOutcome {
        let (state, mut hub) = crate::session::split_lifecycle_mut(self);
        state
            .mark_login_account_offline_on_disconnect_like_cpp(&mut hub)
            .await
    }

    pub(crate) async fn save_account_mounts_like_cpp(&mut self) -> FinalizationOutcome {
        let (state, mut hub) = crate::session::split_lifecycle_mut(self);
        state.save_account_mounts_like_cpp(&mut hub).await
    }

    pub(crate) async fn save_account_toys_like_cpp(&mut self) -> FinalizationOutcome {
        let (state, mut hub) = crate::session::split_lifecycle_mut(self);
        state.save_account_toys_like_cpp(&mut hub).await
    }

    pub(crate) async fn save_account_heirlooms_like_cpp(&mut self) -> FinalizationOutcome {
        let (state, mut hub) = crate::session::split_lifecycle_mut(self);
        state.save_account_heirlooms_like_cpp(&mut hub).await
    }

    pub(crate) async fn cleanup_shared_runtime_state_on_disconnect_with_generator_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
    ) -> FinalizationOutcome {
        wait_for_active_loot_persistence_with_generator_like_cpp(self, item_guid_generator).await;
        if let Some(player_guid) = self.player_guid()
            && self.loot.has_active_loot_views_like_cpp()
        {
            self.do_loot_release_all_like_cpp(player_guid).await;
        }
        self.cleanup_shared_runtime_state()
    }

    pub(crate) async fn cleanup_shared_runtime_state_on_disconnect_like_cpp(&mut self) {
        let generators = self.id_generators_for_test_like_cpp();
        self.cleanup_shared_runtime_state_on_disconnect_with_generator_like_cpp(
            generators.item.as_ref(),
        )
        .await;
    }

    pub(crate) fn unregister_from_player_registry(&self) {
        let (state, hub) = crate::session::split_lifecycle_ref(self);
        state.unregister_from_player_registry(hub)
    }

    pub(crate) fn set_player_logout_like_cpp(&mut self, player_logout: bool) {
        self.lifecycle.set_player_logout_like_cpp(player_logout);
        self.sync_current_player_session_visibility_detection_like_cpp();
    }

    pub(crate) fn restore_realm_channels(&mut self) {
        let (state, mut hub) = crate::session::split_lifecycle_mut(self);
        state.restore_realm_channels(&mut hub)
    }

    pub(crate) fn send_buy_error(
        &self,
        result: BuyResult,
        creature_guid: Option<ObjectGuid>,
        item: u32,
    ) {
        self.core.send_buy_error(result, creature_guid, item)
    }

    pub(crate) fn represented_talent_reset_persistence_plan_like_cpp(
        &self,
        guid_counter: u64,
        old_money: u64,
        new_money: u64,
        cost: u32,
        reset_time_secs: u64,
    ) -> Option<(
        RepresentedTalentResetStatePlanLikeCpp,
        wow_persistence::PlayerTalentResetPersistenceRequestLikeCpp,
    )> {
        let (state, hub) = crate::session::split_lifecycle_ref(self);
        state.represented_talent_reset_persistence_plan_like_cpp(
            hub,
            guid_counter,
            old_money,
            new_money,
            cost,
            reset_time_secs,
        )
    }
}

impl LifecycleCx<'_> {
    fn account_save_cx_like_cpp(&mut self) -> SessionAccountSaveCxLikeCpp<'_> {
        SessionAccountSaveCxLikeCpp {
            lifecycle: &mut *self.lifecycle,
            inventory: &mut *self.inventory,
            hub: self.hub.reborrow_like_cpp(),
        }
    }
    pub(crate) async fn clear_buyback_on_logout(&mut self) -> FinalizationOutcome {
        self.account_save_cx_like_cpp()
            .clear_buyback_on_logout()
            .await
    }
    pub(crate) async fn save_account_item_appearances_like_cpp(&mut self) -> FinalizationOutcome {
        self.account_save_cx_like_cpp()
            .save_account_item_appearances_like_cpp()
            .await
    }
    pub(crate) async fn save_account_transmog_illusions_like_cpp(&mut self) -> FinalizationOutcome {
        self.account_save_cx_like_cpp()
            .save_account_transmog_illusions_like_cpp()
            .await
    }
}
