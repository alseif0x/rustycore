// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Test-only entry points for the player handlers moved to
//! `wow-world-application` (#1263 F5).

use wow_packet::WorldPacket;
use wow_world_application::PlayerHandlerCxLikeCpp;

use crate::session::WorldSession;

impl WorldSession {
    fn player_test_cx_like_cpp(&mut self) -> PlayerHandlerCxLikeCpp<'_> {
        let (quest_state, inventory, lifecycle, visibility, instances, hub) =
            crate::session::split_player_handler_states_mut(self);
        PlayerHandlerCxLikeCpp::new(
            hub,
            quest_state,
            inventory,
            lifecycle,
            visibility,
            instances,
        )
    }

    pub async fn handle_far_sight(&mut self, pkt: WorldPacket) {
        self.player_test_cx_like_cpp().handle_far_sight(pkt).await;
        let catalogs = self.creature_spawn_catalogs_for_test_like_cpp();
        self.force_update_visibility_with_catalogs_like_cpp(&catalogs)
            .await;
    }

    pub async fn handle_query_time(&mut self) {
        self.player_test_cx_like_cpp().handle_query_time().await;
    }

    pub async fn handle_query_next_mail_time(&mut self) {
        self.player_test_cx_like_cpp()
            .handle_query_next_mail_time()
            .await;
    }

    pub async fn handle_set_selection(&mut self, pkt: WorldPacket) {
        self.player_test_cx_like_cpp()
            .handle_set_selection(pkt)
            .await;
    }

    pub async fn handle_set_action_button(&mut self, pkt: WorldPacket) {
        self.player_test_cx_like_cpp()
            .handle_set_action_button(pkt)
            .await;
    }

    pub async fn handle_set_title(&mut self, pkt: WorldPacket) {
        self.player_test_cx_like_cpp().handle_set_title(pkt).await;
    }

    pub async fn handle_get_item_purchase_data(&mut self, pkt: WorldPacket) {
        self.player_test_cx_like_cpp()
            .handle_get_item_purchase_data(pkt)
            .await;
    }

    pub async fn handle_stand_state_change(&mut self, pkt: WorldPacket) {
        if let Some(intent) = self
            .player_test_cx_like_cpp()
            .handle_stand_state_change(pkt)
            .await
        {
            let _ = self.apply_represented_live_intent_like_cpp(intent);
        }
    }
}
