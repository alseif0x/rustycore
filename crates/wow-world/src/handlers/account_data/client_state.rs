// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Client-state handlers that remain in the World shell.
//!
//! The client-state, telemetry, cinematic and inert movement-ack handlers and
//! their registration moved to `wow-world-application` under #1263 F5. The
//! currency-flags handler and the typed `Ping` entry (registered by the
//! character/account world-query family) stay here: the former publishes
//! through the currency/condition chain that still lives in the shell, and
//! every scenario that drives a moved handler keeps a cfg(test) entry point so
//! it can exercise one handler without composing a dispatch table.

use tracing::warn;
use wow_constants::ClientOpcodes;
use wow_handler::{PacketProcessing, SessionStatus};

use crate::session::registry::PacketHandlerEntry;
use wow_packet::ClientPacket;
use wow_packet::packets::misc::SetCurrencyFlags;

crate::session::registry::register_packet_handler_like_cpp! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::SetCurrencyFlags,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_set_currency_flags",
        handler: |session, _catalogs, pkt| {
            Box::pin(async move { session.handle_set_currency_flags(pkt).await })
        },
    }
}

impl crate::session::WorldSession {
    pub async fn handle_set_currency_flags(&mut self, mut pkt: wow_packet::WorldPacket) {
        let packet = match SetCurrencyFlags::read(&mut pkt) {
            Ok(packet) => packet,
            Err(error) => {
                warn!(
                    account = self.core.account_id,
                    "SetCurrencyFlags parse failed: {error}"
                );
                return;
            }
        };

        self.represented_set_currency_flags_like_cpp(packet.currency_id, packet.flags);
    }
}

#[cfg(test)]
mod session_shims {
    use wow_packet::WorldPacket;
    use wow_world_application::ClientStateHandlerCxLikeCpp;

    use crate::session::WorldSession;

    impl WorldSession {
        fn client_state_test_cx_like_cpp(&mut self) -> ClientStateHandlerCxLikeCpp<'_> {
            ClientStateHandlerCxLikeCpp::new(crate::session::hub_mut(self))
        }

        pub(crate) async fn handle_add_battlenet_friend(&mut self, pkt: WorldPacket) {
            self.client_state_test_cx_like_cpp()
                .handle_add_battlenet_friend(pkt)
                .await;
        }
        pub(crate) async fn handle_client_telemetry_null_like_cpp(&mut self, pkt: WorldPacket) {
            self.client_state_test_cx_like_cpp()
                .handle_client_telemetry_null_like_cpp(pkt)
                .await;
        }
        pub(crate) async fn handle_complete_cinematic(&mut self, pkt: WorldPacket) {
            self.client_state_test_cx_like_cpp()
                .handle_complete_cinematic(pkt)
                .await;
        }
        pub(crate) async fn handle_complete_movie(&mut self, pkt: WorldPacket) {
            self.client_state_test_cx_like_cpp()
                .handle_complete_movie(pkt)
                .await;
        }
        pub(crate) async fn handle_get_account_character_list(&mut self, pkt: WorldPacket) {
            self.client_state_test_cx_like_cpp()
                .handle_get_account_character_list(pkt)
                .await;
        }
        pub(crate) async fn handle_get_account_notifications(&mut self, pkt: WorldPacket) {
            self.client_state_test_cx_like_cpp()
                .handle_get_account_notifications(pkt)
                .await;
        }
        pub(crate) async fn handle_loading_screen_notify(&mut self, pkt: WorldPacket) {
            self.client_state_test_cx_like_cpp()
                .handle_loading_screen_notify(pkt)
                .await;
        }
        pub(crate) async fn handle_log_streaming_error(&mut self, pkt: WorldPacket) {
            self.client_state_test_cx_like_cpp()
                .handle_log_streaming_error(pkt)
                .await;
        }
        pub(crate) async fn handle_logout_instant(&mut self, pkt: WorldPacket) {
            self.client_state_test_cx_like_cpp()
                .handle_logout_instant(pkt)
                .await;
        }
        pub(crate) async fn handle_next_cinematic_camera(&mut self, pkt: WorldPacket) {
            self.client_state_test_cx_like_cpp()
                .handle_next_cinematic_camera(pkt)
                .await;
        }
        pub(crate) async fn handle_override_screen_flash(&mut self, pkt: WorldPacket) {
            self.client_state_test_cx_like_cpp()
                .handle_override_screen_flash(pkt)
                .await;
        }
        pub(crate) async fn handle_queued_messages_end(&mut self, pkt: WorldPacket) {
            self.client_state_test_cx_like_cpp()
                .handle_queued_messages_end(pkt)
                .await;
        }
        pub(crate) async fn handle_report_client_variables(&mut self, pkt: WorldPacket) {
            self.client_state_test_cx_like_cpp()
                .handle_report_client_variables(pkt)
                .await;
        }
        pub(crate) async fn handle_report_enabled_addons(&mut self, pkt: WorldPacket) {
            self.client_state_test_cx_like_cpp()
                .handle_report_enabled_addons(pkt)
                .await;
        }
        pub(crate) async fn handle_report_frozen_while_loading_map(&mut self, pkt: WorldPacket) {
            self.client_state_test_cx_like_cpp()
                .handle_report_frozen_while_loading_map(pkt)
                .await;
        }
        pub(crate) async fn handle_report_keybinding_execution_counts(&mut self, pkt: WorldPacket) {
            self.client_state_test_cx_like_cpp()
                .handle_report_keybinding_execution_counts(pkt)
                .await;
        }
        pub(crate) async fn handle_request_countdown_timer(&mut self, pkt: WorldPacket) {
            self.client_state_test_cx_like_cpp()
                .handle_request_countdown_timer(pkt)
                .await;
        }
        pub(crate) async fn handle_set_action_bar_toggles(&mut self, pkt: WorldPacket) {
            self.client_state_test_cx_like_cpp()
                .handle_set_action_bar_toggles(pkt)
                .await;
        }
        pub(crate) async fn handle_set_advanced_combat_logging(&mut self, pkt: WorldPacket) {
            self.client_state_test_cx_like_cpp()
                .handle_set_advanced_combat_logging(pkt)
                .await;
        }
        pub(crate) async fn handle_set_ammo(&mut self, pkt: WorldPacket) {
            self.client_state_test_cx_like_cpp()
                .handle_set_ammo(pkt)
                .await;
        }
        pub(crate) async fn handle_set_game_event_debug_view_state(&mut self, pkt: WorldPacket) {
            self.client_state_test_cx_like_cpp()
                .handle_set_game_event_debug_view_state(pkt)
                .await;
        }
        pub(crate) async fn handle_set_insert_items_left_to_right(&mut self, pkt: WorldPacket) {
            self.client_state_test_cx_like_cpp()
                .handle_set_insert_items_left_to_right(pkt)
                .await;
        }
        pub(crate) async fn handle_showing_cloak(&mut self, pkt: WorldPacket) {
            self.client_state_test_cx_like_cpp()
                .handle_showing_cloak(pkt)
                .await;
        }
        pub(crate) async fn handle_showing_helm(&mut self, pkt: WorldPacket) {
            self.client_state_test_cx_like_cpp()
                .handle_showing_helm(pkt)
                .await;
        }
        pub(crate) async fn handle_spawn_tracking_update(&mut self, pkt: WorldPacket) {
            self.client_state_test_cx_like_cpp()
                .handle_spawn_tracking_update(pkt)
                .await;
        }
        pub(crate) async fn handle_time_adjustment_response(&mut self, pkt: WorldPacket) {
            self.client_state_test_cx_like_cpp()
                .handle_time_adjustment_response(pkt)
                .await;
        }
        pub(crate) async fn handle_unhandled_client_null_like_cpp(&mut self, pkt: WorldPacket) {
            self.client_state_test_cx_like_cpp()
                .handle_unhandled_client_null_like_cpp(pkt)
                .await;
        }
        pub(crate) async fn handle_update_spell_visual(&mut self, pkt: WorldPacket) {
            self.client_state_test_cx_like_cpp()
                .handle_update_spell_visual(pkt)
                .await;
        }
        pub(crate) async fn handle_used_follow(&mut self, pkt: WorldPacket) {
            self.client_state_test_cx_like_cpp()
                .handle_used_follow(pkt)
                .await;
        }
        pub(crate) async fn handle_violence_level(&mut self, pkt: WorldPacket) {
            self.client_state_test_cx_like_cpp()
                .handle_violence_level(pkt)
                .await;
        }
    }
}
