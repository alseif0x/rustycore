// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Client-state, telemetry, cinematic, inert movement-ack and unimplemented
//! purchase-service (BattlePay/VAS) stub handlers.
//!
//! C++ source of truth: the `MiscHandler.cpp` client-state and telemetry
//! branches plus the movement acknowledgements TrinityCore accepts and drops.
//! The family owns the packet bodies; the World session only builds the
//! borrowed hub context (#1263 F5). `#1263 F5 remaining families` moved
//! `CMSG_SET_CURRENCY_FLAGS` here as well: its body still runs in the World
//! shell — it publishes through the currency/condition chain that stays there —
//! and the host lends that operation (`Opcodes.cpp:888` maps
//! `CMSG_SET_CURRENCY_FLAGS` to `STATUS_UNHANDLED`/`Handle_NULL`).

use tracing::{trace, warn};
use wow_constants::ClientOpcodes;
use wow_handler::{
    DuplicateHandlerRegistrationLikeCpp, HandlerFuture, PacketHandlerEntry, PacketProcessing,
    RegistryBuilder, SessionStatus,
};
use wow_packet::ClientPacket;
use wow_packet::WorldPacket;
use wow_packet::packets::auth::{Ping, Pong};
use wow_packet::packets::misc::{
    LoadingScreenNotify, ServerTimeOffset, SetAdvancedCombatLogging, TimeSyncResponse,
    ViolenceLevel,
};
use wow_world_core::session::{HubMut, PacketPublicationAccessLikeCpp};

/// Borrowed inputs of one client-state handler invocation.
pub struct ClientStateHandlerCxLikeCpp<'a> {
    hub: HubMut<'a>,
}

impl<'a> ClientStateHandlerCxLikeCpp<'a> {
    pub fn new(hub: HubMut<'a>) -> Self {
        Self { hub }
    }

    fn account_id_like_cpp(&self) -> u32 {
        self.hub.shared().core.account_id
    }

    /// C++ `WorldSession::HandleTimeSyncResponse`-adjacent server time offset reply.
    pub async fn handle_server_time_offset_request(&mut self) {
        self.publication_like_cpp()
            .send_packet(&ServerTimeOffset::now());
    }

    /// Acknowledge the client's `TimeSyncResponse` so its time-sync state stays
    /// healthy; the periodic timer sends the next request.
    pub async fn handle_time_sync_response(&mut self, resp: TimeSyncResponse) {
        trace!(
            "TimeSyncResponse: seq={}, client_time={} for account {}",
            resp.sequence_index,
            resp.client_time,
            self.account_id_like_cpp()
        );
        self.hub
            .core
            .record_time_sync_response_like_cpp(resp.sequence_index, resp.client_time);
    }

    /// C++ `WorldSession::HandlePing` — reply with `Pong` on the same serial.
    pub async fn handle_ping(&mut self, ping: Ping) {
        trace!(
            "Ping: serial={}, latency={}ms for account {}",
            ping.serial,
            ping.latency,
            self.account_id_like_cpp()
        );
        self.publication_like_cpp().send_packet(&Pong {
            serial: ping.serial,
        });
    }

    fn publication_like_cpp(&self) -> PacketPublicationAccessLikeCpp<'_> {
        self.hub.shared().core.packet_publication_access_like_cpp()
    }
    pub async fn handle_add_battlenet_friend(&mut self, _pkt: wow_packet::WorldPacket) {
        // C++ registers CMSG_ADD_BATTLENET_FRIEND as STATUS_UNHANDLED/Handle_NULL.
    }
    pub async fn handle_client_telemetry_null_like_cpp(&mut self, _pkt: wow_packet::WorldPacket) {
        // C++ registers this client telemetry/ack family to WorldSession::Handle_NULL.
    }
    pub async fn handle_complete_cinematic(&mut self, _pkt: wow_packet::WorldPacket) {
        // C++ CinematicMgr::EndCinematic also clears sight binding when the
        // player is bound to a visual waypoint NPC. Rust records the represented
        // end event until the live CinematicMgr/vision runtime is ported.
        self.hub.complete_represented_cinematic_like_cpp();
    }
    pub async fn handle_complete_movie(&mut self, _pkt: wow_packet::WorldPacket) {
        // C++ Player::GetMovie() == 0 returns early; otherwise SetMovie(0)
        // and ScriptMgr::OnMovieComplete(player, movie). Rust records the
        // script hook until the live ScriptMgr runtime is ported.
        self.hub.complete_represented_movie_like_cpp();
    }
    pub async fn handle_get_account_character_list(&mut self, _pkt: wow_packet::WorldPacket) {
        // C++ registers CMSG_GET_ACCOUNT_CHARACTER_LIST as
        // STATUS_UNHANDLED/Handle_NULL.
    }
    pub async fn handle_get_account_notifications(&mut self, _pkt: wow_packet::WorldPacket) {
        // C++ registers CMSG_GET_ACCOUNT_NOTIFICATIONS as
        // STATUS_UNHANDLED/Handle_NULL.
    }
    pub async fn handle_loading_screen_notify(&mut self, mut pkt: wow_packet::WorldPacket) {
        if let Err(error) = LoadingScreenNotify::read(&mut pkt) {
            warn!(
                account = self.account_id_like_cpp(),
                "LoadingScreenNotify parse failed: {error}"
            );
            return;
        }

        // C++ `HandleLoadScreenOpcode` is a TODO after reading MapID + Showing.
    }
    pub async fn handle_log_streaming_error(&mut self, _pkt: wow_packet::WorldPacket) {
        // C++ registers CMSG_LOG_STREAMING_ERROR as STATUS_UNHANDLED/Handle_NULL.
    }
    pub async fn handle_logout_instant(&mut self, _pkt: wow_packet::WorldPacket) {
        // C++ registers CMSG_LOGOUT_INSTANT as STATUS_UNHANDLED/Handle_NULL.
    }
    pub async fn handle_next_cinematic_camera(&mut self, _pkt: wow_packet::WorldPacket) {
        // C++ CinematicMgr::NextCinematicCamera advances the active camera
        // index and may spawn a visual waypoint for remote sight. Rust records
        // the represented camera advance until fly-by camera/TempSummon/viewpoint
        // runtime is ported.
        self.hub.next_represented_cinematic_camera_like_cpp();
    }
    pub async fn handle_override_screen_flash(&mut self, _pkt: wow_packet::WorldPacket) {
        // C++ registers CMSG_OVERRIDE_SCREEN_FLASH as STATUS_UNHANDLED/Handle_NULL.
    }
    pub async fn handle_queued_messages_end(&mut self, _pkt: wow_packet::WorldPacket) {
        // C++ registers CMSG_QUEUED_MESSAGES_END as STATUS_LOGGEDIN/Handle_NULL.
    }
    pub async fn handle_report_client_variables(&mut self, _pkt: wow_packet::WorldPacket) {
        // C++ registers CMSG_REPORT_CLIENT_VARIABLES as
        // STATUS_UNHANDLED/Handle_NULL.
    }
    pub async fn handle_report_enabled_addons(&mut self, _pkt: wow_packet::WorldPacket) {
        // C++ registers CMSG_REPORT_ENABLED_ADDONS as
        // STATUS_UNHANDLED/Handle_NULL.
    }
    pub async fn handle_report_frozen_while_loading_map(&mut self, _pkt: wow_packet::WorldPacket) {
        // C++ registers CMSG_REPORT_FROZEN_WHILE_LOADING_MAP as
        // STATUS_UNHANDLED/Handle_NULL.
    }
    pub async fn handle_report_keybinding_execution_counts(
        &mut self,
        _pkt: wow_packet::WorldPacket,
    ) {
        // C++ registers CMSG_REPORT_KEYBINDING_EXECUTION_COUNTS as
        // STATUS_UNHANDLED/Handle_NULL.
    }
    pub async fn handle_request_countdown_timer(&mut self, _pkt: wow_packet::WorldPacket) {
        // C++ registers CMSG_QUERY_COUNTDOWN_TIMER as
        // STATUS_UNHANDLED/Handle_NULL.
    }
    pub async fn handle_set_action_bar_toggles(&mut self, mut pkt: wow_packet::WorldPacket) {
        let mask = match pkt.read_uint8() {
            Ok(mask) => mask,
            Err(error) => {
                warn!(
                    account = self.account_id_like_cpp(),
                    "SetActionBarToggles parse failed: {error}"
                );
                return;
            }
        };

        self.hub.represented_set_action_bar_toggles_like_cpp(mask);
    }
    pub async fn handle_set_advanced_combat_logging(&mut self, mut pkt: wow_packet::WorldPacket) {
        let packet = match SetAdvancedCombatLogging::read(&mut pkt) {
            Ok(packet) => packet,
            Err(error) => {
                warn!(
                    account = self.account_id_like_cpp(),
                    "SetAdvancedCombatLogging parse failed: {error}"
                );
                return;
            }
        };

        self.hub
            .represented_set_advanced_combat_logging_like_cpp(packet.enable);
    }
    pub async fn handle_set_ammo(&mut self, _pkt: wow_packet::WorldPacket) {
        // C++ `HandleSetAmmoOpcode(WorldPackets::Null&)` only logs the request.
    }
    pub async fn handle_set_game_event_debug_view_state(&mut self, _pkt: wow_packet::WorldPacket) {
        // C++ `HandleSetGameEventDebugViewState(WorldPackets::Null&)` only logs the request.
    }
    pub async fn handle_set_insert_items_left_to_right(&mut self, _pkt: wow_packet::WorldPacket) {
        // C++ registers CMSG_SET_INSERT_ITEMS_LEFT_TO_RIGHT as STATUS_UNHANDLED/Handle_NULL.
    }
    pub async fn handle_showing_cloak(&mut self, _pkt: wow_packet::WorldPacket) {
        // C++ `HandleShowingCloakOpcode(WorldPackets::Null&)` only logs the request.
    }
    pub async fn handle_showing_helm(&mut self, _pkt: wow_packet::WorldPacket) {
        // C++ `HandleShowingHelmOpcode(WorldPackets::Null&)` only logs the request.
    }
    pub async fn handle_spawn_tracking_update(&mut self, _pkt: wow_packet::WorldPacket) {
        // C++ registers CMSG_SPAWN_TRACKING_UPDATE as STATUS_UNHANDLED/Handle_NULL.
    }
    pub async fn handle_time_adjustment_response(&mut self, _pkt: wow_packet::WorldPacket) {
        // C++ registers CMSG_TIME_ADJUSTMENT_RESPONSE as STATUS_UNHANDLED/Handle_NULL.
    }
    pub async fn handle_unhandled_client_null_like_cpp(&mut self, _pkt: wow_packet::WorldPacket) {
        // C++ registers this bounded client packet family as STATUS_UNHANDLED/Handle_NULL.
    }
    pub async fn handle_update_spell_visual(&mut self, _pkt: wow_packet::WorldPacket) {
        // C++ registers CMSG_UPDATE_SPELL_VISUAL as STATUS_UNHANDLED/Handle_NULL.
    }
    pub async fn handle_used_follow(&mut self, _pkt: wow_packet::WorldPacket) {
        // C++ registers CMSG_USED_FOLLOW as STATUS_UNHANDLED/Handle_NULL.
    }
    pub async fn handle_violence_level(&mut self, mut pkt: wow_packet::WorldPacket) {
        if let Err(error) = ViolenceLevel::read(&mut pkt) {
            warn!(
                account = self.account_id_like_cpp(),
                "ViolenceLevel parse failed: {error}"
            );
            return;
        }

        // C++ `HandleViolenceLevel` reads ViolenceLvl and has no observable action.
    }
}

/// Builds a client-state handler context from a host hub.
pub trait ClientStateHandlerHostLikeCpp<C> {
    fn client_state_handler_cx_like_cpp<'a>(
        &'a mut self,
        catalogs: &'a C,
    ) -> ClientStateHandlerCxLikeCpp<'a>;

    /// C++ `Opcodes.cpp:888`: `CMSG_SET_CURRENCY_FLAGS` is `STATUS_UNHANDLED`
    /// with `Handle_NULL`; the Rust port represents the body in the World shell.
    ///
    /// The legacy registration closure did not read the catalog view, so this
    /// entry point does not carry it. The body publishes through the
    /// shell-owned currency/condition chain and therefore stays in the World
    /// session.
    fn handle_set_currency_flags<'a>(&'a mut self, pkt: WorldPacket) -> HandlerFuture<'a, ()>;
}

fn handle_add_battlenet_friend_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: ClientStateHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .client_state_handler_cx_like_cpp(catalogs)
            .handle_add_battlenet_friend(pkt)
            .await;
    })
}

fn handle_client_telemetry_null_like_cpp_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: ClientStateHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .client_state_handler_cx_like_cpp(catalogs)
            .handle_client_telemetry_null_like_cpp(pkt)
            .await;
    })
}

fn handle_complete_cinematic_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: ClientStateHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .client_state_handler_cx_like_cpp(catalogs)
            .handle_complete_cinematic(pkt)
            .await;
    })
}

fn handle_complete_movie_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: ClientStateHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .client_state_handler_cx_like_cpp(catalogs)
            .handle_complete_movie(pkt)
            .await;
    })
}

fn handle_get_account_character_list_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: ClientStateHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .client_state_handler_cx_like_cpp(catalogs)
            .handle_get_account_character_list(pkt)
            .await;
    })
}

fn handle_get_account_notifications_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: ClientStateHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .client_state_handler_cx_like_cpp(catalogs)
            .handle_get_account_notifications(pkt)
            .await;
    })
}

fn handle_loading_screen_notify_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: ClientStateHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .client_state_handler_cx_like_cpp(catalogs)
            .handle_loading_screen_notify(pkt)
            .await;
    })
}

fn handle_log_streaming_error_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: ClientStateHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .client_state_handler_cx_like_cpp(catalogs)
            .handle_log_streaming_error(pkt)
            .await;
    })
}

fn handle_logout_instant_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: ClientStateHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .client_state_handler_cx_like_cpp(catalogs)
            .handle_logout_instant(pkt)
            .await;
    })
}

fn handle_next_cinematic_camera_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: ClientStateHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .client_state_handler_cx_like_cpp(catalogs)
            .handle_next_cinematic_camera(pkt)
            .await;
    })
}

fn handle_override_screen_flash_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: ClientStateHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .client_state_handler_cx_like_cpp(catalogs)
            .handle_override_screen_flash(pkt)
            .await;
    })
}

fn handle_queued_messages_end_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: ClientStateHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .client_state_handler_cx_like_cpp(catalogs)
            .handle_queued_messages_end(pkt)
            .await;
    })
}

fn handle_report_client_variables_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: ClientStateHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .client_state_handler_cx_like_cpp(catalogs)
            .handle_report_client_variables(pkt)
            .await;
    })
}

fn handle_report_enabled_addons_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: ClientStateHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .client_state_handler_cx_like_cpp(catalogs)
            .handle_report_enabled_addons(pkt)
            .await;
    })
}

fn handle_report_frozen_while_loading_map_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: ClientStateHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .client_state_handler_cx_like_cpp(catalogs)
            .handle_report_frozen_while_loading_map(pkt)
            .await;
    })
}

fn handle_report_keybinding_execution_counts_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: ClientStateHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .client_state_handler_cx_like_cpp(catalogs)
            .handle_report_keybinding_execution_counts(pkt)
            .await;
    })
}

fn handle_request_countdown_timer_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: ClientStateHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .client_state_handler_cx_like_cpp(catalogs)
            .handle_request_countdown_timer(pkt)
            .await;
    })
}

fn handle_set_action_bar_toggles_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: ClientStateHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .client_state_handler_cx_like_cpp(catalogs)
            .handle_set_action_bar_toggles(pkt)
            .await;
    })
}

fn handle_set_advanced_combat_logging_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: ClientStateHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .client_state_handler_cx_like_cpp(catalogs)
            .handle_set_advanced_combat_logging(pkt)
            .await;
    })
}

fn handle_set_ammo_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: ClientStateHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .client_state_handler_cx_like_cpp(catalogs)
            .handle_set_ammo(pkt)
            .await;
    })
}

fn handle_set_game_event_debug_view_state_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: ClientStateHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .client_state_handler_cx_like_cpp(catalogs)
            .handle_set_game_event_debug_view_state(pkt)
            .await;
    })
}

fn handle_set_insert_items_left_to_right_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: ClientStateHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .client_state_handler_cx_like_cpp(catalogs)
            .handle_set_insert_items_left_to_right(pkt)
            .await;
    })
}

fn handle_showing_cloak_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: ClientStateHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .client_state_handler_cx_like_cpp(catalogs)
            .handle_showing_cloak(pkt)
            .await;
    })
}

fn handle_showing_helm_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: ClientStateHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .client_state_handler_cx_like_cpp(catalogs)
            .handle_showing_helm(pkt)
            .await;
    })
}

fn handle_spawn_tracking_update_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: ClientStateHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .client_state_handler_cx_like_cpp(catalogs)
            .handle_spawn_tracking_update(pkt)
            .await;
    })
}

fn handle_time_adjustment_response_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: ClientStateHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .client_state_handler_cx_like_cpp(catalogs)
            .handle_time_adjustment_response(pkt)
            .await;
    })
}

fn handle_unhandled_client_null_like_cpp_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: ClientStateHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .client_state_handler_cx_like_cpp(catalogs)
            .handle_unhandled_client_null_like_cpp(pkt)
            .await;
    })
}

fn handle_update_spell_visual_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: ClientStateHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .client_state_handler_cx_like_cpp(catalogs)
            .handle_update_spell_visual(pkt)
            .await;
    })
}

fn handle_used_follow_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: ClientStateHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .client_state_handler_cx_like_cpp(catalogs)
            .handle_used_follow(pkt)
            .await;
    })
}

fn handle_violence_level_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: ClientStateHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .client_state_handler_cx_like_cpp(catalogs)
            .handle_violence_level(pkt)
            .await;
    })
}

fn battle_pay_get_product_list_stub_thunk<'a, S, C>(
    _session: &'a mut S,
    _catalogs: &'a C,
    _pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: ClientStateHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        trace!(
            "Stub handler for {:?} (0x{:04X}) — no response needed",
            ClientOpcodes::BattlePayGetProductList,
            ClientOpcodes::BattlePayGetProductList as u32
        );
    })
}

fn battle_pay_get_purchase_list_stub_thunk<'a, S, C>(
    _session: &'a mut S,
    _catalogs: &'a C,
    _pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: ClientStateHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        trace!(
            "Stub handler for {:?} (0x{:04X}) — no response needed",
            ClientOpcodes::BattlePayGetPurchaseList,
            ClientOpcodes::BattlePayGetPurchaseList as u32
        );
    })
}

fn update_vas_purchase_states_stub_thunk<'a, S, C>(
    _session: &'a mut S,
    _catalogs: &'a C,
    _pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: ClientStateHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        trace!(
            "Stub handler for {:?} (0x{:04X}) — no response needed",
            ClientOpcodes::UpdateVasPurchaseStates,
            ClientOpcodes::UpdateVasPurchaseStates as u32
        );
    })
}

fn handle_server_time_offset_request_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    _pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: ClientStateHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .client_state_handler_cx_like_cpp(catalogs)
            .handle_server_time_offset_request()
            .await;
    })
}

fn handle_time_sync_response_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    mut pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: ClientStateHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        match TimeSyncResponse::read(&mut pkt) {
            Ok(resp) => {
                session
                    .client_state_handler_cx_like_cpp(catalogs)
                    .handle_time_sync_response(resp)
                    .await;
            }
            Err(e) => warn!("Failed to read TimeSyncResponse: {e}"),
        }
    })
}

fn handle_ping_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    mut pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: ClientStateHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        match Ping::read(&mut pkt) {
            Ok(ping) => {
                session
                    .client_state_handler_cx_like_cpp(catalogs)
                    .handle_ping(ping)
                    .await;
            }
            Err(e) => warn!("Failed to read Ping: {e}"),
        }
    })
}

/// The `#1263 F5 remaining families` `CMSG_SET_CURRENCY_FLAGS` entry: the legacy
/// closure forwarded the raw packet to the World shell, which still reads it.
fn handle_set_currency_flags_thunk<'a, S, C>(
    session: &'a mut S,
    _catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: ClientStateHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move { session.handle_set_currency_flags(pkt).await })
}

/// Register the client-state packet entries through their application adapter.
pub fn register_client_state_handlers_like_cpp<S, C>(
    builder: &mut RegistryBuilder<S, C>,
) -> Result<(), DuplicateHandlerRegistrationLikeCpp>
where
    S: ClientStateHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::LoadingScreenNotify,
        status: SessionStatus::Authed,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_loading_screen_notify",
        handler: handle_loading_screen_notify_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::AddBattlenetFriend,
        status: SessionStatus::Authed,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_add_battlenet_friend",
        handler: handle_add_battlenet_friend_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::BattlenetChallengeResponse,
        status: SessionStatus::Authed,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_unhandled_client_null_like_cpp",
        handler: handle_unhandled_client_null_like_cpp_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::SetInsertItemsLeftToRight,
        status: SessionStatus::Authed,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_set_insert_items_left_to_right",
        handler: handle_set_insert_items_left_to_right_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::SaveAccountDataExport,
        status: SessionStatus::Authed,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_unhandled_client_null_like_cpp",
        handler: handle_unhandled_client_null_like_cpp_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::ChangeBagSlotFlag,
        status: SessionStatus::Authed,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_unhandled_client_null_like_cpp",
        handler: handle_unhandled_client_null_like_cpp_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::CloseQuestChoice,
        status: SessionStatus::Authed,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_unhandled_client_null_like_cpp",
        handler: handle_unhandled_client_null_like_cpp_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::QueryQuestItemUsability,
        status: SessionStatus::Authed,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_unhandled_client_null_like_cpp",
        handler: handle_unhandled_client_null_like_cpp_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::SetPreferredCemetery,
        status: SessionStatus::Authed,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_unhandled_client_null_like_cpp",
        handler: handle_unhandled_client_null_like_cpp_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::UpdateClientSettings,
        status: SessionStatus::Authed,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_unhandled_client_null_like_cpp",
        handler: handle_unhandled_client_null_like_cpp_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::DiscardedTimeSyncAcks,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadSafe,
        handler_name: "handle_client_telemetry_null_like_cpp",
        handler: handle_client_telemetry_null_like_cpp_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::EngineSurvey,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_client_telemetry_null_like_cpp",
        handler: handle_client_telemetry_null_like_cpp_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::LatencyReport,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_client_telemetry_null_like_cpp",
        handler: handle_client_telemetry_null_like_cpp_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::ReportServerLag,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_client_telemetry_null_like_cpp",
        handler: handle_client_telemetry_null_like_cpp_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::SuspendCommsAck,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_client_telemetry_null_like_cpp",
        handler: handle_client_telemetry_null_like_cpp_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::ViolenceLevel,
        status: SessionStatus::Authed,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_violence_level",
        handler: handle_violence_level_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::OverrideScreenFlash,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_override_screen_flash",
        handler: handle_override_screen_flash_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::QueuedMessagesEnd,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_queued_messages_end",
        handler: handle_queued_messages_end_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::SetActionBarToggles,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_set_action_bar_toggles",
        handler: handle_set_action_bar_toggles_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::SetAdvancedCombatLogging,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_set_advanced_combat_logging",
        handler: handle_set_advanced_combat_logging_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::SetAmmo,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_set_ammo",
        handler: handle_set_ammo_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::SetGameEventDebugViewState,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_set_game_event_debug_view_state",
        handler: handle_set_game_event_debug_view_state_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::ShowingHelm,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_showing_helm",
        handler: handle_showing_helm_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::ShowingCloak,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_showing_cloak",
        handler: handle_showing_cloak_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::GetAccountCharacterList,
        status: SessionStatus::Authed,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_get_account_character_list",
        handler: handle_get_account_character_list_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::GetAccountNotifications,
        status: SessionStatus::Authed,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_get_account_notifications",
        handler: handle_get_account_notifications_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::ReportClientVariables,
        status: SessionStatus::Authed,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_report_client_variables",
        handler: handle_report_client_variables_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::ReportEnabledAddons,
        status: SessionStatus::Authed,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_report_enabled_addons",
        handler: handle_report_enabled_addons_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::ReportFrozenWhileLoadingMap,
        status: SessionStatus::Authed,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_report_frozen_while_loading_map",
        handler: handle_report_frozen_while_loading_map_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::LogStreamingError,
        status: SessionStatus::Authed,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_log_streaming_error",
        handler: handle_log_streaming_error_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::CompleteCinematic,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_complete_cinematic",
        handler: handle_complete_cinematic_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::NextCinematicCamera,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_next_cinematic_camera",
        handler: handle_next_cinematic_camera_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::CompleteMovie,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_complete_movie",
        handler: handle_complete_movie_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::LogoutInstant,
        status: SessionStatus::Authed,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_logout_instant",
        handler: handle_logout_instant_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::SpawnTrackingUpdate,
        status: SessionStatus::Authed,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_spawn_tracking_update",
        handler: handle_spawn_tracking_update_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::TimeAdjustmentResponse,
        status: SessionStatus::Authed,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_time_adjustment_response",
        handler: handle_time_adjustment_response_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::UpdateSpellVisual,
        status: SessionStatus::Authed,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_update_spell_visual",
        handler: handle_update_spell_visual_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::UsedFollow,
        status: SessionStatus::Authed,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_used_follow",
        handler: handle_used_follow_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::ReportKeybindingExecutionCounts,
        status: SessionStatus::Authed,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_report_keybinding_execution_counts",
        handler: handle_report_keybinding_execution_counts_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::QueryCountdownTimer,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_request_countdown_timer",
        handler: handle_request_countdown_timer_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::MoveAddImpulseAck,
        status: SessionStatus::Authed,
        processing: PacketProcessing::ThreadSafe,
        handler_name: "handle_unhandled_client_null_like_cpp",
        handler: handle_unhandled_client_null_like_cpp_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::MoveApplyInertiaAck,
        status: SessionStatus::Authed,
        processing: PacketProcessing::ThreadSafe,
        handler_name: "handle_unhandled_client_null_like_cpp",
        handler: handle_unhandled_client_null_like_cpp_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::MoveRemoveInertiaAck,
        status: SessionStatus::Authed,
        processing: PacketProcessing::ThreadSafe,
        handler_name: "handle_unhandled_client_null_like_cpp",
        handler: handle_unhandled_client_null_like_cpp_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::MoveRemoveMovementForces,
        status: SessionStatus::Authed,
        processing: PacketProcessing::ThreadSafe,
        handler_name: "handle_unhandled_client_null_like_cpp",
        handler: handle_unhandled_client_null_like_cpp_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::MoveSeamlessTransferComplete,
        status: SessionStatus::Authed,
        processing: PacketProcessing::ThreadSafe,
        handler_name: "handle_unhandled_client_null_like_cpp",
        handler: handle_unhandled_client_null_like_cpp_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::MoveSetAdvFly,
        status: SessionStatus::Authed,
        processing: PacketProcessing::ThreadSafe,
        handler_name: "handle_unhandled_client_null_like_cpp",
        handler: handle_unhandled_client_null_like_cpp_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::MoveSetAdvFlyingAddImpulseMaxSpeedAck,
        status: SessionStatus::Authed,
        processing: PacketProcessing::ThreadSafe,
        handler_name: "handle_unhandled_client_null_like_cpp",
        handler: handle_unhandled_client_null_like_cpp_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::MoveSetAdvFlyingAirFrictionAck,
        status: SessionStatus::Authed,
        processing: PacketProcessing::ThreadSafe,
        handler_name: "handle_unhandled_client_null_like_cpp",
        handler: handle_unhandled_client_null_like_cpp_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::MoveSetAdvFlyingBankingRateAck,
        status: SessionStatus::Authed,
        processing: PacketProcessing::ThreadSafe,
        handler_name: "handle_unhandled_client_null_like_cpp",
        handler: handle_unhandled_client_null_like_cpp_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::MoveSetAdvFlyingDoubleJumpVelModAck,
        status: SessionStatus::Authed,
        processing: PacketProcessing::ThreadSafe,
        handler_name: "handle_unhandled_client_null_like_cpp",
        handler: handle_unhandled_client_null_like_cpp_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::MoveSetAdvFlyingGlideStartMinHeightAck,
        status: SessionStatus::Authed,
        processing: PacketProcessing::ThreadSafe,
        handler_name: "handle_unhandled_client_null_like_cpp",
        handler: handle_unhandled_client_null_like_cpp_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::MoveSetAdvFlyingLaunchSpeedCoefficientAck,
        status: SessionStatus::Authed,
        processing: PacketProcessing::ThreadSafe,
        handler_name: "handle_unhandled_client_null_like_cpp",
        handler: handle_unhandled_client_null_like_cpp_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::MoveSetAdvFlyingLiftCoefficientAck,
        status: SessionStatus::Authed,
        processing: PacketProcessing::ThreadSafe,
        handler_name: "handle_unhandled_client_null_like_cpp",
        handler: handle_unhandled_client_null_like_cpp_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::MoveSetAdvFlyingMaxVelAck,
        status: SessionStatus::Authed,
        processing: PacketProcessing::ThreadSafe,
        handler_name: "handle_unhandled_client_null_like_cpp",
        handler: handle_unhandled_client_null_like_cpp_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::MoveSetAdvFlyingOverMaxDecelerationAck,
        status: SessionStatus::Authed,
        processing: PacketProcessing::ThreadSafe,
        handler_name: "handle_unhandled_client_null_like_cpp",
        handler: handle_unhandled_client_null_like_cpp_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::MoveSetAdvFlyingPitchingRateDownAck,
        status: SessionStatus::Authed,
        processing: PacketProcessing::ThreadSafe,
        handler_name: "handle_unhandled_client_null_like_cpp",
        handler: handle_unhandled_client_null_like_cpp_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::MoveSetAdvFlyingPitchingRateUpAck,
        status: SessionStatus::Authed,
        processing: PacketProcessing::ThreadSafe,
        handler_name: "handle_unhandled_client_null_like_cpp",
        handler: handle_unhandled_client_null_like_cpp_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::MoveSetAdvFlyingSurfaceFrictionAck,
        status: SessionStatus::Authed,
        processing: PacketProcessing::ThreadSafe,
        handler_name: "handle_unhandled_client_null_like_cpp",
        handler: handle_unhandled_client_null_like_cpp_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::MoveSetAdvFlyingTurnVelocityThresholdAck,
        status: SessionStatus::Authed,
        processing: PacketProcessing::ThreadSafe,
        handler_name: "handle_unhandled_client_null_like_cpp",
        handler: handle_unhandled_client_null_like_cpp_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::BattlePayGetProductList,
        status: SessionStatus::Authed,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_battle_pay_stub",
        handler: battle_pay_get_product_list_stub_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::BattlePayGetPurchaseList,
        status: SessionStatus::Authed,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_battle_pay_stub",
        handler: battle_pay_get_purchase_list_stub_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::UpdateVasPurchaseStates,
        status: SessionStatus::Authed,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_vas_stub",
        handler: update_vas_purchase_states_stub_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::ServerTimeOffsetRequest,
        status: SessionStatus::Authed,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_server_time_offset_request",
        handler: handle_server_time_offset_request_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::TimeSyncResponse,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadSafe,
        handler_name: "handle_time_sync_response",
        handler: handle_time_sync_response_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::TimeSyncResponseDropped,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadSafe,
        handler_name: "handle_time_sync_response",
        handler: handle_time_sync_response_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::TimeSyncResponseFailed,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadSafe,
        handler_name: "handle_time_sync_response",
        handler: handle_time_sync_response_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::Ping,
        status: SessionStatus::Authed,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_ping",
        handler: handle_ping_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::SetCurrencyFlags,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_set_currency_flags",
        handler: handle_set_currency_flags_thunk::<S, C>,
    })?;
    Ok(())
}
