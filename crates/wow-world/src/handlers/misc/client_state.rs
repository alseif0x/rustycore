// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Private client_state capability handlers extracted from the legacy misc owner.

use tracing::warn;
use wow_constants::ClientOpcodes;
use wow_handler::{PacketProcessing, SessionStatus};

use crate::session::registry::PacketHandlerEntry;
use wow_packet::ClientPacket;
use wow_packet::packets::misc::{
    LoadingScreenNotify, SetAdvancedCombatLogging, SetCurrencyFlags, ViolenceLevel,
};

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::LoadingScreenNotify,
        status: SessionStatus::Authed,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_loading_screen_notify",
        handler: |session, _catalogs, pkt| {
            Box::pin(async move { session.handle_loading_screen_notify(pkt).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::AddBattlenetFriend,
        status: SessionStatus::Authed,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_add_battlenet_friend",
        handler: |session, _catalogs, pkt| {
            Box::pin(async move { session.handle_add_battlenet_friend(pkt).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::BattlenetChallengeResponse,
        status: SessionStatus::Authed,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_unhandled_client_null_like_cpp",
        handler: |session, _catalogs, pkt| {
            Box::pin(async move { session.handle_unhandled_client_null_like_cpp(pkt).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::SetInsertItemsLeftToRight,
        status: SessionStatus::Authed,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_set_insert_items_left_to_right",
        handler: |session, _catalogs, pkt| {
            Box::pin(async move { session.handle_set_insert_items_left_to_right(pkt).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::SaveAccountDataExport,
        status: SessionStatus::Authed,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_unhandled_client_null_like_cpp",
        handler: |session, _catalogs, pkt| {
            Box::pin(async move { session.handle_unhandled_client_null_like_cpp(pkt).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::ChangeBagSlotFlag,
        status: SessionStatus::Authed,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_unhandled_client_null_like_cpp",
        handler: |session, _catalogs, pkt| {
            Box::pin(async move { session.handle_unhandled_client_null_like_cpp(pkt).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::CloseQuestChoice,
        status: SessionStatus::Authed,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_unhandled_client_null_like_cpp",
        handler: |session, _catalogs, pkt| {
            Box::pin(async move { session.handle_unhandled_client_null_like_cpp(pkt).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::QueryQuestItemUsability,
        status: SessionStatus::Authed,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_unhandled_client_null_like_cpp",
        handler: |session, _catalogs, pkt| {
            Box::pin(async move { session.handle_unhandled_client_null_like_cpp(pkt).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::SetPreferredCemetery,
        status: SessionStatus::Authed,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_unhandled_client_null_like_cpp",
        handler: |session, _catalogs, pkt| {
            Box::pin(async move { session.handle_unhandled_client_null_like_cpp(pkt).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::UpdateClientSettings,
        status: SessionStatus::Authed,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_unhandled_client_null_like_cpp",
        handler: |session, _catalogs, pkt| {
            Box::pin(async move { session.handle_unhandled_client_null_like_cpp(pkt).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::DiscardedTimeSyncAcks,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadSafe,
        handler_name: "handle_client_telemetry_null_like_cpp",
        handler: |session, _catalogs, pkt| {
            Box::pin(async move { session.handle_client_telemetry_null_like_cpp(pkt).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::EngineSurvey,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_client_telemetry_null_like_cpp",
        handler: |session, _catalogs, pkt| {
            Box::pin(async move { session.handle_client_telemetry_null_like_cpp(pkt).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::LatencyReport,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_client_telemetry_null_like_cpp",
        handler: |session, _catalogs, pkt| {
            Box::pin(async move { session.handle_client_telemetry_null_like_cpp(pkt).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::ReportServerLag,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_client_telemetry_null_like_cpp",
        handler: |session, _catalogs, pkt| {
            Box::pin(async move { session.handle_client_telemetry_null_like_cpp(pkt).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::SuspendCommsAck,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_client_telemetry_null_like_cpp",
        handler: |session, _catalogs, pkt| {
            Box::pin(async move { session.handle_client_telemetry_null_like_cpp(pkt).await })
        },
    }
}

macro_rules! register_unhandled_threadsafe_null_handler {
    ($opcode:ident) => {
        inventory::submit! {
            PacketHandlerEntry {
                opcode: ClientOpcodes::$opcode,
                status: SessionStatus::Authed,
                processing: PacketProcessing::ThreadSafe,
                handler_name: "handle_unhandled_client_null_like_cpp",
                handler: |session, _catalogs, pkt| {
                    Box::pin(async move { session.handle_unhandled_client_null_like_cpp(pkt).await })
                },
            }
        }
    };
}

register_unhandled_threadsafe_null_handler!(MoveAddImpulseAck);
register_unhandled_threadsafe_null_handler!(MoveApplyInertiaAck);
register_unhandled_threadsafe_null_handler!(MoveRemoveInertiaAck);
register_unhandled_threadsafe_null_handler!(MoveRemoveMovementForces);
register_unhandled_threadsafe_null_handler!(MoveSeamlessTransferComplete);
register_unhandled_threadsafe_null_handler!(MoveSetAdvFly);
register_unhandled_threadsafe_null_handler!(MoveSetAdvFlyingAddImpulseMaxSpeedAck);
register_unhandled_threadsafe_null_handler!(MoveSetAdvFlyingAirFrictionAck);
register_unhandled_threadsafe_null_handler!(MoveSetAdvFlyingBankingRateAck);
register_unhandled_threadsafe_null_handler!(MoveSetAdvFlyingDoubleJumpVelModAck);
register_unhandled_threadsafe_null_handler!(MoveSetAdvFlyingGlideStartMinHeightAck);
register_unhandled_threadsafe_null_handler!(MoveSetAdvFlyingLaunchSpeedCoefficientAck);
register_unhandled_threadsafe_null_handler!(MoveSetAdvFlyingLiftCoefficientAck);
register_unhandled_threadsafe_null_handler!(MoveSetAdvFlyingMaxVelAck);
register_unhandled_threadsafe_null_handler!(MoveSetAdvFlyingOverMaxDecelerationAck);
register_unhandled_threadsafe_null_handler!(MoveSetAdvFlyingPitchingRateDownAck);
register_unhandled_threadsafe_null_handler!(MoveSetAdvFlyingPitchingRateUpAck);
register_unhandled_threadsafe_null_handler!(MoveSetAdvFlyingSurfaceFrictionAck);
register_unhandled_threadsafe_null_handler!(MoveSetAdvFlyingTurnVelocityThresholdAck);

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::ViolenceLevel,
        status: SessionStatus::Authed,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_violence_level",
        handler: |session, _catalogs, pkt| Box::pin(async move { session.handle_violence_level(pkt).await }),
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::OverrideScreenFlash,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_override_screen_flash",
        handler: |session, _catalogs, pkt| {
            Box::pin(async move { session.handle_override_screen_flash(pkt).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::QueuedMessagesEnd,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_queued_messages_end",
        handler: |session, _catalogs, pkt| {
            Box::pin(async move { session.handle_queued_messages_end(pkt).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::SetActionBarToggles,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_set_action_bar_toggles",
        handler: |session, _catalogs, pkt| {
            Box::pin(async move { session.handle_set_action_bar_toggles(pkt).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::SetAdvancedCombatLogging,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_set_advanced_combat_logging",
        handler: |session, _catalogs, pkt| {
            Box::pin(async move { session.handle_set_advanced_combat_logging(pkt).await })
        },
    }
}

inventory::submit! {
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

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::SetAmmo,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_set_ammo",
        handler: |session, _catalogs, pkt| Box::pin(async move { session.handle_set_ammo(pkt).await }),
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::SetGameEventDebugViewState,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_set_game_event_debug_view_state",
        handler: |session, _catalogs, pkt| {
            Box::pin(async move { session.handle_set_game_event_debug_view_state(pkt).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::ShowingHelm,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_showing_helm",
        handler: |session, _catalogs, pkt| Box::pin(async move { session.handle_showing_helm(pkt).await }),
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::ShowingCloak,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_showing_cloak",
        handler: |session, _catalogs, pkt| Box::pin(async move { session.handle_showing_cloak(pkt).await }),
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::GetAccountCharacterList,
        status: SessionStatus::Authed,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_get_account_character_list",
        handler: |session, _catalogs, pkt| {
            Box::pin(async move { session.handle_get_account_character_list(pkt).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::GetAccountNotifications,
        status: SessionStatus::Authed,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_get_account_notifications",
        handler: |session, _catalogs, pkt| {
            Box::pin(async move { session.handle_get_account_notifications(pkt).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::ReportClientVariables,
        status: SessionStatus::Authed,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_report_client_variables",
        handler: |session, _catalogs, pkt| {
            Box::pin(async move { session.handle_report_client_variables(pkt).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::ReportEnabledAddons,
        status: SessionStatus::Authed,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_report_enabled_addons",
        handler: |session, _catalogs, pkt| {
            Box::pin(async move { session.handle_report_enabled_addons(pkt).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::ReportFrozenWhileLoadingMap,
        status: SessionStatus::Authed,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_report_frozen_while_loading_map",
        handler: |session, _catalogs, pkt| {
            Box::pin(async move { session.handle_report_frozen_while_loading_map(pkt).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::LogStreamingError,
        status: SessionStatus::Authed,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_log_streaming_error",
        handler: |session, _catalogs, pkt| {
            Box::pin(async move { session.handle_log_streaming_error(pkt).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::CompleteCinematic,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_complete_cinematic",
        handler: |session, _catalogs, pkt| {
            Box::pin(async move { session.handle_complete_cinematic(pkt).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::NextCinematicCamera,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_next_cinematic_camera",
        handler: |session, _catalogs, pkt| {
            Box::pin(async move { session.handle_next_cinematic_camera(pkt).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::CompleteMovie,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_complete_movie",
        handler: |session, _catalogs, pkt| Box::pin(async move { session.handle_complete_movie(pkt).await }),
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::LogoutInstant,
        status: SessionStatus::Authed,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_logout_instant",
        handler: |session, _catalogs, pkt| Box::pin(async move { session.handle_logout_instant(pkt).await }),
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::SpawnTrackingUpdate,
        status: SessionStatus::Authed,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_spawn_tracking_update",
        handler: |session, _catalogs, pkt| {
            Box::pin(async move { session.handle_spawn_tracking_update(pkt).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::TimeAdjustmentResponse,
        status: SessionStatus::Authed,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_time_adjustment_response",
        handler: |session, _catalogs, pkt| {
            Box::pin(async move { session.handle_time_adjustment_response(pkt).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::UpdateSpellVisual,
        status: SessionStatus::Authed,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_update_spell_visual",
        handler: |session, _catalogs, pkt| {
            Box::pin(async move { session.handle_update_spell_visual(pkt).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::UsedFollow,
        status: SessionStatus::Authed,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_used_follow",
        handler: |session, _catalogs, pkt| Box::pin(async move { session.handle_used_follow(pkt).await }),
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::ReportKeybindingExecutionCounts,
        status: SessionStatus::Authed,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_report_keybinding_execution_counts",
        handler: |session, _catalogs, pkt| {
            Box::pin(async move { session.handle_report_keybinding_execution_counts(pkt).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::QueryCountdownTimer,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_request_countdown_timer",
        handler: |session, _catalogs, pkt| {
            Box::pin(async move { session.handle_request_countdown_timer(pkt).await })
        },
    }
}

impl crate::session::WorldSession {
    // ── Silent-ignore stubs ────────────────────────────────────────────────────
    // These opcodes are sent by the client at login but require no server
    // response at this stage (UI state, client-side settings, system queries
    // that return empty data until the respective subsystems are implemented).

    pub async fn handle_loading_screen_notify(&mut self, mut pkt: wow_packet::WorldPacket) {
        if let Err(error) = LoadingScreenNotify::read(&mut pkt) {
            warn!(
                account = self.account_id,
                "LoadingScreenNotify parse failed: {error}"
            );
            return;
        }

        // C++ `HandleLoadScreenOpcode` is a TODO after reading MapID + Showing.
    }

    pub async fn handle_violence_level(&mut self, mut pkt: wow_packet::WorldPacket) {
        if let Err(error) = ViolenceLevel::read(&mut pkt) {
            warn!(
                account = self.account_id,
                "ViolenceLevel parse failed: {error}"
            );
            return;
        }

        // C++ `HandleViolenceLevel` reads ViolenceLvl and has no observable action.
    }

    pub async fn handle_override_screen_flash(&mut self, _pkt: wow_packet::WorldPacket) {
        // C++ registers CMSG_OVERRIDE_SCREEN_FLASH as STATUS_UNHANDLED/Handle_NULL.
    }

    pub async fn handle_queued_messages_end(&mut self, _pkt: wow_packet::WorldPacket) {
        // C++ registers CMSG_QUEUED_MESSAGES_END as STATUS_LOGGEDIN/Handle_NULL.
    }

    pub async fn handle_set_action_bar_toggles(&mut self, mut pkt: wow_packet::WorldPacket) {
        let mask = match pkt.read_uint8() {
            Ok(mask) => mask,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    "SetActionBarToggles parse failed: {error}"
                );
                return;
            }
        };

        self.represented_set_action_bar_toggles_like_cpp(mask);
    }

    pub async fn handle_set_advanced_combat_logging(&mut self, mut pkt: wow_packet::WorldPacket) {
        let packet = match SetAdvancedCombatLogging::read(&mut pkt) {
            Ok(packet) => packet,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    "SetAdvancedCombatLogging parse failed: {error}"
                );
                return;
            }
        };

        self.represented_set_advanced_combat_logging_like_cpp(packet.enable);
    }

    pub async fn handle_set_currency_flags(&mut self, mut pkt: wow_packet::WorldPacket) {
        let packet = match SetCurrencyFlags::read(&mut pkt) {
            Ok(packet) => packet,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    "SetCurrencyFlags parse failed: {error}"
                );
                return;
            }
        };

        self.represented_set_currency_flags_like_cpp(packet.currency_id, packet.flags);
    }

    pub async fn handle_add_battlenet_friend(&mut self, _pkt: wow_packet::WorldPacket) {
        // C++ registers CMSG_ADD_BATTLENET_FRIEND as STATUS_UNHANDLED/Handle_NULL.
    }

    pub async fn handle_set_insert_items_left_to_right(&mut self, _pkt: wow_packet::WorldPacket) {
        // C++ registers CMSG_SET_INSERT_ITEMS_LEFT_TO_RIGHT as STATUS_UNHANDLED/Handle_NULL.
    }

    pub async fn handle_unhandled_client_null_like_cpp(&mut self, _pkt: wow_packet::WorldPacket) {
        // C++ registers this bounded client packet family as STATUS_UNHANDLED/Handle_NULL.
    }

    pub async fn handle_client_telemetry_null_like_cpp(&mut self, _pkt: wow_packet::WorldPacket) {
        // C++ registers this client telemetry/ack family to WorldSession::Handle_NULL.
    }

    pub async fn handle_set_ammo(&mut self, _pkt: wow_packet::WorldPacket) {
        // C++ `HandleSetAmmoOpcode(WorldPackets::Null&)` only logs the request.
    }

    pub async fn handle_set_game_event_debug_view_state(&mut self, _pkt: wow_packet::WorldPacket) {
        // C++ `HandleSetGameEventDebugViewState(WorldPackets::Null&)` only logs the request.
    }

    pub async fn handle_showing_helm(&mut self, _pkt: wow_packet::WorldPacket) {
        // C++ `HandleShowingHelmOpcode(WorldPackets::Null&)` only logs the request.
    }

    pub async fn handle_showing_cloak(&mut self, _pkt: wow_packet::WorldPacket) {
        // C++ `HandleShowingCloakOpcode(WorldPackets::Null&)` only logs the request.
    }

    pub async fn handle_get_account_character_list(&mut self, _pkt: wow_packet::WorldPacket) {
        // C++ registers CMSG_GET_ACCOUNT_CHARACTER_LIST as
        // STATUS_UNHANDLED/Handle_NULL.
    }

    pub async fn handle_get_account_notifications(&mut self, _pkt: wow_packet::WorldPacket) {
        // C++ registers CMSG_GET_ACCOUNT_NOTIFICATIONS as
        // STATUS_UNHANDLED/Handle_NULL.
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

    pub async fn handle_log_streaming_error(&mut self, _pkt: wow_packet::WorldPacket) {
        // C++ registers CMSG_LOG_STREAMING_ERROR as STATUS_UNHANDLED/Handle_NULL.
    }

    pub async fn handle_complete_cinematic(&mut self, _pkt: wow_packet::WorldPacket) {
        // C++ CinematicMgr::EndCinematic also clears sight binding when the
        // player is bound to a visual waypoint NPC. Rust records the represented
        // end event until the live CinematicMgr/vision runtime is ported.
        self.complete_represented_cinematic_like_cpp();
    }

    pub async fn handle_next_cinematic_camera(&mut self, _pkt: wow_packet::WorldPacket) {
        // C++ CinematicMgr::NextCinematicCamera advances the active camera
        // index and may spawn a visual waypoint for remote sight. Rust records
        // the represented camera advance until fly-by camera/TempSummon/viewpoint
        // runtime is ported.
        self.next_represented_cinematic_camera_like_cpp();
    }

    pub async fn handle_complete_movie(&mut self, _pkt: wow_packet::WorldPacket) {
        // C++ Player::GetMovie() == 0 returns early; otherwise SetMovie(0)
        // and ScriptMgr::OnMovieComplete(player, movie). Rust records the
        // script hook until the live ScriptMgr runtime is ported.
        self.complete_represented_movie_like_cpp();
    }

    pub async fn handle_logout_instant(&mut self, _pkt: wow_packet::WorldPacket) {
        // C++ registers CMSG_LOGOUT_INSTANT as STATUS_UNHANDLED/Handle_NULL.
    }

    pub async fn handle_spawn_tracking_update(&mut self, _pkt: wow_packet::WorldPacket) {
        // C++ registers CMSG_SPAWN_TRACKING_UPDATE as STATUS_UNHANDLED/Handle_NULL.
    }

    pub async fn handle_time_adjustment_response(&mut self, _pkt: wow_packet::WorldPacket) {
        // C++ registers CMSG_TIME_ADJUSTMENT_RESPONSE as STATUS_UNHANDLED/Handle_NULL.
    }

    pub async fn handle_update_spell_visual(&mut self, _pkt: wow_packet::WorldPacket) {
        // C++ registers CMSG_UPDATE_SPELL_VISUAL as STATUS_UNHANDLED/Handle_NULL.
    }

    pub async fn handle_used_follow(&mut self, _pkt: wow_packet::WorldPacket) {
        // C++ registers CMSG_USED_FOLLOW as STATUS_UNHANDLED/Handle_NULL.
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
}
