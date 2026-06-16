// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Chat packet handlers — CMSG_CHAT_MESSAGE_*.
//!
//! Say / Yell / Emote messages are broadcast to nearby players on the same map
//! via the shared PlayerRegistry. Whispers are forwarded to the named target if
//! they are online; otherwise echoed back as a "not found" message.
//!
//! Broadcast ranges (matching C# WorldConfig defaults):
//!   Say         25 yards
//!   Yell       300 yards
//!   Text emote  25 yards
//!
//! Reference: C# Game/Handlers/ChatHandler.cs, Game/Entities/Player/Player.cs

use tracing::debug;

use wow_chat::hyperlinks::check_all_links_shape_like_cpp;
use wow_chat::validation::validate_message_like_cpp;
use wow_constants::ClientOpcodes;
use wow_core::ObjectGuid;
use wow_core::guid::HighGuid;
use wow_handler::{PacketHandlerEntry, PacketProcessing, SessionStatus};
use wow_network::{GroupInfo, SendAddonIfRegisteredLikeCppCommand, SessionCommand};
use wow_packet::packets::chat::{
    CTextEmote, ChatAddonMessage, ChatAddonMessageTargeted, ChatAddonMessageWhisper, ChatMessage,
    ChatMessageAfk, ChatMessageChannel, ChatMessageDnd, ChatMessageEmote, ChatMessageWhisper,
    ChatMsg, ChatPkt, ChatPlayerNotfound, ChatRegisterAddonPrefixes, ChatReportFiltered,
    ChatReportIgnored, EmoteClient, EmoteMessage, PrintNotification, STextEmote, UpdateAadcStatus,
    UpdateAadcStatusResponse,
};
use wow_packet::{ClientPacket, ServerPacket};

use crate::session::{
    ChatFloodThrottleIndexLikeCpp, PlayerAwayModeLikeCpp, WorldSession, player_team_for_race_cpp,
};

// ── Broadcast range constants (C# WorldCfg defaults) ─────────────
const RANGE_SAY: f32 = 25.0;
const RANGE_YELL: f32 = 300.0;
const RANGE_EMOTE: f32 = 25.0;
const LANG_UNIVERSAL_LIKE_CPP: i32 = 0;
const LANG_ADDON_LIKE_CPP: u32 = 183;
const LANG_ADDON_LOGGED_LIKE_CPP: u32 = 184;
const GM_SILENCE_AURA_LIKE_CPP: i32 = 1852;
const KNOWN_LANGUAGES_LIKE_CPP: &[i32] = &[
    // C++ `LanguageMgr::LoadLanguages` accepts Languages.db2 entries plus the
    // code-only languages registered in `SharedDefines.h`.
    0, 1, 2, 3, 6, 7, 8, 9, 10, 11, 12, 13, 14, 33, 35, 36, 37, 38, 39, 40, 42, 43, 44, 168, 178,
    179, 180, 181, 182, 183, 184, 285, 287, 288, 290, 291, 292, 293, 294, 295, 296, 297, 298,
];

// ── Handler registrations ─────────────────────────────────────────

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::ChatMessageSay,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_chat_say",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::ChatMessageYell,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_chat_yell",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::ChatMessageParty,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_chat_party",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::ChatMessageGuild,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_chat_guild",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::ChatMessageOfficer,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_chat_officer",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::ChatMessageRaid,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_chat_raid",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::ChatMessageRaidWarning,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_chat_raid_warning",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::ChatMessageInstanceChat,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_chat_instance",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::ChatMessageWhisper,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_chat_whisper",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::ChatMessageChannel,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_chat_channel_message",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::ChatMessageAfk,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_chat_afk",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::ChatMessageDnd,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_chat_dnd",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::UpdateAadcStatus,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_update_aadc_status",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::ChatReportIgnored,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_chat_report_ignored",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::ChatReportFiltered,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_chat_report_filtered",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::ChatMessageEmote,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_chat_emote",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::Emote,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_emote",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::SendTextEmote,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_text_emote",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::ChatRegisterAddonPrefixes,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_chat_register_addon_prefixes",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::ChatAddonMessage,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_chat_addon_message",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::ChatAddonMessageWhisper,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_chat_addon_message_whisper",
    }
}

// ── Handler implementations ───────────────────────────────────────

impl WorldSession {
    /// C++ ref: `WorldSession::ValidateHyperlinksAndMaybeKick`.
    fn validate_hyperlinks_and_maybe_kick_like_cpp(&mut self, text: &str, context: &str) -> bool {
        if check_all_links_shape_like_cpp(text) {
            return true;
        }

        tracing::warn!(
            account = self.account_id,
            context,
            "Chat message rejected: invalid hyperlink/control sequence"
        );

        if self.chat_strict_link_checking_kick_like_cpp() {
            self.kick("WorldSession::ValidateHyperlinksAndMaybeKick Invalid chat link");
        }

        false
    }

    /// Handle say/yell/party/guild/raid/instance chat messages.
    pub async fn handle_chat_message(
        &mut self,
        mut pkt: wow_packet::WorldPacket,
        msg_type: ChatMsg,
    ) {
        let mut msg = match ChatMessage::read(&mut pkt) {
            Ok(m) => m,
            Err(e) => {
                tracing::warn!(account = self.account_id, "Bad chat packet: {e}");
                return;
            }
        };

        if msg.language == LANG_UNIVERSAL_LIKE_CPP {
            tracing::warn!(
                account = self.account_id,
                ty = ?msg_type,
                "Chat message rejected: client attempted LANG_UNIVERSAL"
            );
            return;
        }
        if !is_known_language_like_cpp(msg.language) {
            tracing::warn!(
                account = self.account_id,
                ty = ?msg_type,
                language = msg.language,
                "Chat message rejected: unknown language"
            );
            return;
        }
        if self.send_wait_before_speaking_notification_if_muted_like_cpp() {
            return;
        }
        if !matches!(msg_type, ChatMsg::Afk | ChatMsg::Dnd) {
            self.update_speak_time_like_cpp(ChatFloodThrottleIndexLikeCpp::Regular);
        }
        if msg.text.len() > 511 {
            return;
        }
        if msg.text.is_empty() {
            return;
        }
        if !validate_message_like_cpp(&mut msg.text, self.chat_fake_message_preventing_like_cpp()) {
            tracing::warn!(
                account = self.account_id,
                ty = ?msg_type,
                "Chat message rejected: invalid character/control sequence"
            );
            return;
        }
        if msg.text.is_empty() {
            return;
        }

        debug!(
            account = self.account_id,
            ty = ?msg_type,
            text = %msg.text,
            "Chat message"
        );

        if !self.validate_hyperlinks_and_maybe_kick_like_cpp(&msg.text, "chat") {
            return;
        }
        if self.has_gm_silence_aura_like_cpp() {
            self.send_gm_silence_notification_like_cpp();
            return;
        }
        if matches!(msg_type, ChatMsg::Say | ChatMsg::Yell) && !self.player_is_alive_like_cpp() {
            return;
        }
        if !self.meets_chat_level_req_like_cpp(msg_type) {
            if let Some(required_level) = self.required_chat_level_like_cpp(msg_type) {
                self.send_chat_say_level_notification_like_cpp(required_level);
            }
            return;
        }

        let (sender_guid, sender_name) = self.player_name_and_guid();
        let virtual_realm = self.virtual_realm_address();

        if matches!(
            msg_type,
            ChatMsg::Party | ChatMsg::Raid | ChatMsg::RaidWarning | ChatMsg::InstanceChat
        ) {
            self.broadcast_group_chat_like_cpp(
                msg_type,
                msg.language as u32,
                sender_guid,
                sender_name,
                msg.text,
                virtual_realm,
            );
            return;
        }

        if matches!(msg_type, ChatMsg::Guild | ChatMsg::Officer) {
            debug!(
                account = self.account_id,
                ty = ?msg_type,
                "Guild chat ignored until GuildRegistry/BroadcastToGuild is ported"
            );
            return;
        }

        let chat = ChatPkt {
            msg_type,
            language: msg.language as u32,
            sender_guid,
            sender_name,
            target_guid: wow_core::ObjectGuid::EMPTY,
            target_name: String::new(),
            prefix: String::new(),
            channel: String::new(),
            text: msg.text,
            virtual_realm,
        };

        // Echo back to the sender (they need to see their own message).
        self.send_packet(&chat);

        // Broadcast to nearby players on the same map.
        let range = if msg_type == ChatMsg::Yell {
            RANGE_YELL
        } else {
            RANGE_SAY
        };
        self.broadcast_chat_packet(&chat, range);
    }

    /// Handle whisper messages.
    pub async fn handle_chat_whisper(&mut self, mut pkt: wow_packet::WorldPacket) {
        let mut msg = match ChatMessageWhisper::read(&mut pkt) {
            Ok(m) => m,
            Err(e) => {
                tracing::warn!(account = self.account_id, "Bad whisper packet: {e}");
                return;
            }
        };

        if msg.language == LANG_UNIVERSAL_LIKE_CPP {
            tracing::warn!(
                account = self.account_id,
                "Whisper rejected: client attempted LANG_UNIVERSAL"
            );
            return;
        }
        if !is_known_language_like_cpp(msg.language) {
            tracing::warn!(
                account = self.account_id,
                language = msg.language,
                "Whisper rejected: unknown language"
            );
            return;
        }
        if self.send_wait_before_speaking_notification_if_muted_like_cpp() {
            return;
        }
        self.update_speak_time_like_cpp(ChatFloodThrottleIndexLikeCpp::Regular);
        if msg.text.len() > 511 {
            return;
        }
        if msg.text.is_empty() {
            return;
        }
        if !validate_message_like_cpp(&mut msg.text, self.chat_fake_message_preventing_like_cpp()) {
            tracing::warn!(
                account = self.account_id,
                "Whisper rejected: invalid character/control sequence"
            );
            return;
        }
        if msg.text.is_empty() {
            return;
        }
        if !self.validate_hyperlinks_and_maybe_kick_like_cpp(&msg.text, "whisper") {
            return;
        }

        debug!(
            account = self.account_id,
            target = %msg.target,
            text = %msg.text,
            "Whisper"
        );

        if !self.meets_whisper_level_req_like_cpp() {
            self.send_chat_whisper_level_notification_like_cpp(
                self.chat_level_requirements_like_cpp().whisper,
            );
            return;
        }

        let (sender_guid, sender_name) = self.player_name_and_guid();
        let virtual_realm = self.virtual_realm_address();
        let target_name = msg.target.clone();

        // Try to deliver to the target player via the registry.
        let target_info = self.player_registry().and_then(|reg| {
            reg.iter()
                .find(|e| e.value().player_name.eq_ignore_ascii_case(&target_name))
                .map(|e| {
                    (
                        e.value().send_tx.clone(),
                        e.value().is_game_master,
                        e.value().is_afk,
                        e.value().is_dnd,
                        e.value().auto_reply_msg_like_cpp.clone(),
                    )
                })
        });

        if let Some((tx, target_is_game_master, target_is_afk, target_is_dnd, target_auto_reply)) =
            target_info
        {
            if self.has_gm_silence_aura_like_cpp() && !target_is_game_master {
                self.send_gm_silence_notification_like_cpp();
                return;
            }

            // Forward the whisper to the target as a normal Say-whisper.
            let to_target = ChatPkt {
                msg_type: ChatMsg::Whisper,
                language: msg.language as u32,
                sender_guid,
                sender_name: sender_name.clone(),
                target_guid: ObjectGuid::EMPTY,
                target_name: target_name.clone(),
                prefix: String::new(),
                channel: String::new(),
                text: msg.text.clone(),
                virtual_realm,
            };
            let _ = tx.send(to_target.to_bytes());

            // Inform sender their whisper was delivered.
            let inform = ChatPkt {
                msg_type: ChatMsg::WhisperInform,
                language: msg.language as u32,
                sender_guid,
                sender_name: sender_name.clone(),
                target_guid: ObjectGuid::EMPTY,
                target_name: target_name.clone(),
                prefix: String::new(),
                channel: String::new(),
                text: msg.text,
                virtual_realm,
            };
            self.send_packet(&inform);

            if target_is_afk {
                self.send_whisper_away_reply_like_cpp(&target_name, &target_auto_reply, true);
            } else if target_is_dnd {
                self.send_whisper_away_reply_like_cpp(&target_name, &target_auto_reply, false);
            }
        } else {
            self.send_packet(&ChatPlayerNotfound { name: target_name });
        }
    }

    /// Handle CMSG_CHAT_MESSAGE_CHANNEL.
    ///
    /// C++ routes this through `HandleChatMessage(CHAT_MSG_CHANNEL, ...)`.
    /// The Rust parser and validation are represented, but the final
    /// ChannelMgr lookup/fanout is intentionally parked until live channels
    /// exist.
    pub async fn handle_chat_channel_message(&mut self, mut pkt: wow_packet::WorldPacket) {
        let mut msg = match ChatMessageChannel::read(&mut pkt) {
            Ok(m) => m,
            Err(e) => {
                tracing::warn!(account = self.account_id, "Bad channel chat packet: {e}");
                return;
            }
        };

        if msg.language == LANG_UNIVERSAL_LIKE_CPP {
            tracing::warn!(
                account = self.account_id,
                "Channel chat rejected: client attempted LANG_UNIVERSAL"
            );
            return;
        }
        if !is_known_language_like_cpp(msg.language) {
            tracing::warn!(
                account = self.account_id,
                language = msg.language,
                "Channel chat rejected: unknown language"
            );
            return;
        }
        if self.send_wait_before_speaking_notification_if_muted_like_cpp() {
            return;
        }
        self.update_speak_time_like_cpp(ChatFloodThrottleIndexLikeCpp::Regular);
        if msg.text.len() > 511 || msg.text.is_empty() {
            return;
        }
        if !validate_message_like_cpp(&mut msg.text, self.chat_fake_message_preventing_like_cpp()) {
            tracing::warn!(
                account = self.account_id,
                "Channel chat rejected: invalid character/control sequence"
            );
            return;
        }
        if msg.text.is_empty() {
            return;
        }
        if !self.validate_hyperlinks_and_maybe_kick_like_cpp(&msg.text, "channel_chat") {
            return;
        }
        if self.has_gm_silence_aura_like_cpp() {
            self.send_gm_silence_notification_like_cpp();
            return;
        }
        if !self.meets_chat_level_req_like_cpp(ChatMsg::Channel) {
            if let Some(required_level) = self.required_chat_level_like_cpp(ChatMsg::Channel) {
                self.send_chat_say_level_notification_like_cpp(required_level);
            }
            return;
        }

        debug!(
            account = self.account_id,
            target = %msg.target,
            channel_guid = ?msg.channel_guid,
            secure = ?msg.is_secure,
            "Channel chat ignored until ChannelMgr::Say is ported"
        );
    }

    /// Handle CMSG_UPDATE_AADC_STATUS.
    ///
    /// C++ ignores the requested state because disabling chat is unsupported,
    /// then sends success with ChatDisabled=false so the client restores its cvar.
    pub async fn handle_update_aadc_status(&mut self, mut pkt: wow_packet::WorldPacket) {
        if let Err(e) = UpdateAadcStatus::read(&mut pkt) {
            tracing::warn!(
                account = self.account_id,
                "Bad update AADC status packet: {e}"
            );
            return;
        }

        self.send_packet(&UpdateAadcStatusResponse {
            success: true,
            chat_disabled: false,
        });
    }

    /// Handle CMSG_CHAT_MESSAGE_AFK.
    pub async fn handle_chat_afk(&mut self, mut pkt: wow_packet::WorldPacket) {
        let mut msg = match ChatMessageAfk::read(&mut pkt) {
            Ok(m) => m,
            Err(e) => {
                tracing::warn!(account = self.account_id, "Bad AFK chat packet: {e}");
                return;
            }
        };

        if self.send_wait_before_speaking_notification_if_muted_like_cpp() {
            return;
        }
        self.update_speak_time_like_cpp(ChatFloodThrottleIndexLikeCpp::Regular);
        if msg.text.len() > 511 {
            return;
        }
        if !validate_message_like_cpp(&mut msg.text, self.chat_fake_message_preventing_like_cpp()) {
            tracing::warn!(
                account = self.account_id,
                "AFK message rejected: invalid character/control sequence"
            );
            return;
        }
        if !self.validate_hyperlinks_and_maybe_kick_like_cpp(&msg.text, "afk") {
            return;
        }
        if self.has_gm_silence_aura_like_cpp() {
            self.send_gm_silence_notification_like_cpp();
            return;
        }
        let _ = self.apply_chat_away_mode_like_cpp(PlayerAwayModeLikeCpp::Afk, msg.text);
    }

    /// Handle CMSG_CHAT_MESSAGE_DND.
    pub async fn handle_chat_dnd(&mut self, mut pkt: wow_packet::WorldPacket) {
        let mut msg = match ChatMessageDnd::read(&mut pkt) {
            Ok(m) => m,
            Err(e) => {
                tracing::warn!(account = self.account_id, "Bad DND chat packet: {e}");
                return;
            }
        };

        if self.send_wait_before_speaking_notification_if_muted_like_cpp() {
            return;
        }
        if msg.text.len() > 511 {
            return;
        }
        if !validate_message_like_cpp(&mut msg.text, self.chat_fake_message_preventing_like_cpp()) {
            tracing::warn!(
                account = self.account_id,
                "DND message rejected: invalid character/control sequence"
            );
            return;
        }
        if !self.validate_hyperlinks_and_maybe_kick_like_cpp(&msg.text, "dnd") {
            return;
        }
        if self.has_gm_silence_aura_like_cpp() {
            self.send_gm_silence_notification_like_cpp();
            return;
        }
        let _ = self.apply_chat_away_mode_like_cpp(PlayerAwayModeLikeCpp::Dnd, msg.text);
    }

    /// Handle CMSG_CHAT_REPORT_IGNORED.
    ///
    /// C++ ref: `WorldSession::HandleChatIgnoredOpcode`.
    /// The receiver's client sends this after locally ignoring a chat message;
    /// the server notifies the ignored player with `CHAT_MSG_IGNORED`.
    pub async fn handle_chat_report_ignored(&mut self, mut pkt: wow_packet::WorldPacket) {
        let report = match ChatReportIgnored::read(&mut pkt) {
            Ok(report) => report,
            Err(e) => {
                tracing::warn!(
                    account = self.account_id,
                    "Bad chat report ignored packet: {e}"
                );
                return;
            }
        };

        let (reporter_guid, reporter_name) = self.player_name_and_guid();
        let virtual_realm = self.virtual_realm_address();

        let ignored_tx = self.player_registry().and_then(|reg| {
            reg.get(&report.ignored_guid)
                .map(|entry| entry.send_tx.clone())
        });

        if let Some(tx) = ignored_tx {
            let ignored = ChatPkt {
                msg_type: ChatMsg::Ignored,
                language: 0,
                sender_guid: reporter_guid,
                sender_name: reporter_name.clone(),
                target_guid: reporter_guid,
                target_name: reporter_name.clone(),
                prefix: String::new(),
                channel: String::new(),
                text: reporter_name,
                virtual_realm,
            };
            let _ = tx.send(ignored.to_bytes());
        }
    }

    /// Handle CMSG_CHAT_REPORT_FILTERED.
    ///
    /// C++ ref: `WorldSession::HandleChatReportFiltered`.
    /// TrinityCore currently reads an empty packet and only logs a TODO for the
    /// unimplemented spam reporting system.
    pub async fn handle_chat_report_filtered(&mut self, mut pkt: wow_packet::WorldPacket) {
        if let Err(e) = ChatReportFiltered::read(&mut pkt) {
            tracing::warn!(
                account = self.account_id,
                "Bad chat report filtered packet: {e}"
            );
            return;
        }

        debug!(
            account = self.account_id,
            "ChatReportFiltered received; spam reporting is not represented yet"
        );
    }

    /// Handle emote text (/e).
    pub async fn handle_chat_emote(&mut self, mut pkt: wow_packet::WorldPacket) {
        let mut msg = match ChatMessageEmote::read(&mut pkt) {
            Ok(m) => m,
            Err(e) => {
                tracing::warn!(account = self.account_id, "Bad emote packet: {e}");
                return;
            }
        };

        if self.send_wait_before_speaking_notification_if_muted_like_cpp() {
            return;
        }
        if msg.text.len() > 511 {
            return;
        }
        if msg.text.is_empty() {
            return;
        }
        if !validate_message_like_cpp(&mut msg.text, self.chat_fake_message_preventing_like_cpp()) {
            tracing::warn!(
                account = self.account_id,
                "Text emote rejected: invalid character/control sequence"
            );
            return;
        }
        if msg.text.is_empty() {
            return;
        }
        if !self.validate_hyperlinks_and_maybe_kick_like_cpp(&msg.text, "text_emote") {
            return;
        }
        if self.has_gm_silence_aura_like_cpp() {
            self.send_gm_silence_notification_like_cpp();
            return;
        }
        if !self.player_is_alive_like_cpp() {
            return;
        }
        if self.player_level_like_cpp() < self.chat_level_requirements_like_cpp().emote {
            self.send_chat_say_level_notification_like_cpp(
                self.chat_level_requirements_like_cpp().emote,
            );
            return;
        }

        debug!(
            account = self.account_id,
            text = %msg.text,
            "Text emote"
        );

        let (sender_guid, sender_name) = self.player_name_and_guid();
        let virtual_realm = self.virtual_realm_address();

        let chat = ChatPkt {
            msg_type: ChatMsg::Emote,
            language: 0,
            sender_guid,
            sender_name,
            target_guid: wow_core::ObjectGuid::EMPTY,
            target_name: String::new(),
            prefix: String::new(),
            channel: String::new(),
            text: msg.text,
            virtual_realm,
        };
        self.send_packet(&chat);
        self.broadcast_chat_packet(&chat, RANGE_EMOTE);
    }

    /// Handle CMSG_EMOTE — client notifies us it cleared its emote state.
    ///
    /// C# ref: `ChatHandler.HandleEmote` — sets `EmoteState` to `OneshotNone`.
    /// We have no emote state machine yet, so just log and return.
    pub async fn handle_emote(&mut self, mut pkt: wow_packet::WorldPacket) {
        // EmoteClient has no body — read returns Ok(()) immediately.
        let _ = EmoteClient::read(&mut pkt);
        debug!(account = self.account_id, "CMSG_EMOTE: clear emote state");
    }

    /// Handle CMSG_SEND_TEXT_EMOTE — player performs a text emote (/wave, /dance…).
    ///
    /// C# ref: `ChatHandler.HandleTextEmote`:
    ///  1. Look up `EmotesTextStorage[EmoteID]` → animation `Emote` enum value.
    ///  2. Broadcast `STextEmote` (chat text) to nearby players.
    ///  3. Call `HandleEmoteCommand(emote, ...)` → broadcasts `EmoteMessage` (animation).
    ///
    /// We don't have EmotesText.db2 parsed yet, so we:
    ///  - Always send `STextEmote` (shows text in chat log).
    ///  - Echo `EmoteMessage` back with the raw EmoteID (best-effort animation).
    pub async fn handle_text_emote(&mut self, mut pkt: wow_packet::WorldPacket) {
        let msg = match CTextEmote::read(&mut pkt) {
            Ok(m) => m,
            Err(e) => {
                tracing::warn!(account = self.account_id, "Bad CMSG_SEND_TEXT_EMOTE: {e}");
                return;
            }
        };

        if !self.player_is_alive_like_cpp() {
            return;
        }
        if self.send_wait_before_speaking_notification_if_muted_like_cpp() {
            return;
        }

        debug!(
            account = self.account_id,
            emote_id = msg.emote_id,
            sound_index = msg.sound_index,
            "CMSG_SEND_TEXT_EMOTE"
        );

        let (player_guid, _name) = self.player_name_and_guid();
        let account_guid =
            ObjectGuid::create_global(HighGuid::WowAccount, 0, self.account_id as i64);

        let text_emote = STextEmote {
            source_guid: player_guid,
            source_account_guid: account_guid,
            emote_id: msg.emote_id,
            sound_index: msg.sound_index,
            target_guid: msg.target,
        };
        let anim_emote = EmoteMessage {
            guid: player_guid,
            emote_id: msg.emote_id,
            spell_visual_kit_ids: msg.spell_visual_kit_ids.clone(),
            sequence_variation: msg.sequence_variation,
        };

        // 1. STextEmote — shows the emote text ("PlayerName waves") in the chat log.
        self.send_packet(&text_emote);
        // 2. EmoteMessage — plays the animation (emote_id passed through directly).
        self.send_packet(&anim_emote);

        // Broadcast both packets to nearby players.
        self.broadcast_raw_packet(text_emote.to_bytes(), RANGE_EMOTE);
        self.broadcast_raw_packet(anim_emote.to_bytes(), RANGE_EMOTE);
    }

    /// CMSG_CHAT_REGISTER_ADDON_PREFIXES.
    ///
    /// C++ ref: `WorldSession::HandleAddonRegisteredPrefixesOpcode`.
    pub async fn handle_chat_register_addon_prefixes(&mut self, mut pkt: wow_packet::WorldPacket) {
        let packet = match ChatRegisterAddonPrefixes::read(&mut pkt) {
            Ok(packet) => packet,
            Err(e) => {
                tracing::warn!(account = self.account_id, "Bad addon prefix packet: {e}");
                return;
            }
        };

        self.registered_addon_prefixes.extend(packet.prefixes);
        self.filter_addon_messages =
            self.registered_addon_prefixes.len() <= ChatRegisterAddonPrefixes::MAX_PREFIXES;
        debug!(
            account = self.account_id,
            prefixes = self.registered_addon_prefixes.len(),
            filter = self.filter_addon_messages,
            "Registered addon prefixes"
        );
    }

    /// CMSG_CHAT_ADDON_MESSAGE.
    ///
    /// C++ ref: `WorldSession::HandleChatAddonMessageOpcode`.
    /// Until guild/channel addon routing is ported, parse and validate the C++
    /// packet shape then drop unsupported traffic. This matches disabled addon
    /// channel behavior and prevents unknown-opcode noise during login.
    pub async fn handle_chat_addon_message(&mut self, mut pkt: wow_packet::WorldPacket) {
        let packet = match ChatAddonMessage::read(&mut pkt) {
            Ok(packet) => packet,
            Err(e) => {
                tracing::warn!(account = self.account_id, "Bad addon chat packet: {e}");
                return;
            }
        };

        self.handle_chat_addon_message_params_like_cpp(packet, "", ObjectGuid::EMPTY);
    }

    /// CMSG_CHAT_ADDON_MESSAGE_TARGETED.
    ///
    /// C++ ref: `WorldSession::HandleChatAddonMessageTargetedOpcode`.
    ///
    /// Not registered in the runtime opcode table yet: the inspected C++ 3.4.3
    /// source marks this packet as `0xBADD`, which is a duplicated unresolved
    /// placeholder in the Rust opcode enum. Keep the parser and handler covered
    /// by direct tests until the real opcode mapping is known.
    pub async fn handle_chat_addon_message_targeted(&mut self, mut pkt: wow_packet::WorldPacket) {
        let packet = match ChatAddonMessageTargeted::read(&mut pkt) {
            Ok(packet) => packet,
            Err(e) => {
                tracing::warn!(
                    account = self.account_id,
                    "Bad targeted addon chat packet: {e}"
                );
                return;
            }
        };

        self.handle_chat_addon_message_params_like_cpp(
            packet.params,
            &packet.target,
            packet.channel_guid,
        );
    }

    fn handle_chat_addon_message_params_like_cpp(
        &mut self,
        packet: ChatAddonMessage,
        target: &str,
        _channel_guid: ObjectGuid,
    ) {
        if packet.prefix.is_empty() || packet.prefix.len() > 16 {
            return;
        }

        if !self.addon_channel_like_cpp() {
            return;
        }
        if !self.can_speak_like_cpp() {
            return;
        }
        self.update_speak_time_like_cpp(ChatFloodThrottleIndexLikeCpp::Addon);

        if packet.text.len() > 255 {
            return;
        }

        let Some(msg_type) = chat_msg_from_i32_like_cpp(packet.msg_type) else {
            debug!(
                account = self.account_id,
                ty = packet.msg_type,
                "Unknown addon chat message type ignored"
            );
            return;
        };

        match msg_type {
            ChatMsg::Whisper => {
                self.send_addon_whisper_like_cpp(
                    target,
                    packet.prefix,
                    packet.text,
                    packet.is_logged,
                    false,
                );
            }
            ChatMsg::Party | ChatMsg::Raid | ChatMsg::InstanceChat => {
                self.broadcast_group_addon_chat_like_cpp(msg_type, packet);
            }
            ChatMsg::Guild | ChatMsg::Officer | ChatMsg::Channel => {
                debug!(
                    account = self.account_id,
                    ty = ?msg_type,
                    prefix = %packet.prefix,
                    logged = packet.is_logged,
                    "Addon chat message ignored until guild/channel/targeted addon routing is ported"
                );
            }
            _ => {
                debug!(
                    account = self.account_id,
                    ty = ?msg_type,
                    "Unsupported addon chat message type ignored"
                );
            }
        }
    }

    /// CMSG_CHAT_ADDON_MESSAGE_WHISPER.
    ///
    /// C++ ref: `WorldSession::HandleChatAddonMessageWhisper`.
    pub async fn handle_chat_addon_message_whisper(&mut self, mut pkt: wow_packet::WorldPacket) {
        let packet = match ChatAddonMessageWhisper::read(&mut pkt) {
            Ok(packet) => packet,
            Err(e) => {
                tracing::warn!(account = self.account_id, "Bad addon whisper packet: {e}");
                return;
            }
        };

        if self.has_gm_silence_aura_like_cpp() {
            return;
        }

        if self.player_level_like_cpp() < self.chat_level_requirements_like_cpp().whisper {
            return;
        }

        self.send_addon_whisper_like_cpp(
            &packet.target,
            packet.prefix,
            packet.message,
            false,
            true,
        );
    }

    fn send_addon_whisper_like_cpp(
        &mut self,
        target_name: &str,
        prefix: String,
        message: String,
        is_logged: bool,
        notify_missing: bool,
    ) {
        let target_info = self.player_registry().and_then(|reg| {
            reg.iter()
                .find(|e| e.value().player_name.eq_ignore_ascii_case(target_name))
                .map(|e| {
                    (
                        *e.key(),
                        e.value().command_tx.clone(),
                        e.value().race,
                        e.value().player_name.clone(),
                    )
                })
        });

        let Some((target_guid, command_tx, target_race, target_name)) = target_info else {
            if notify_missing {
                self.send_packet(&ChatPlayerNotfound {
                    name: target_name.to_string(),
                });
            }
            return;
        };

        if player_team_for_race_cpp(self.player_race_like_cpp())
            != player_team_for_race_cpp(target_race)
        {
            if notify_missing {
                self.send_packet(&ChatPlayerNotfound { name: target_name });
            }
            return;
        }

        let (sender_guid, sender_name) = self.player_name_and_guid();
        let chat = ChatPkt {
            msg_type: ChatMsg::Whisper,
            language: if is_logged {
                LANG_ADDON_LOGGED_LIKE_CPP
            } else {
                LANG_ADDON_LIKE_CPP
            },
            sender_guid,
            sender_name,
            target_guid,
            target_name,
            prefix: prefix.clone(),
            channel: String::new(),
            text: message,
            virtual_realm: self.virtual_realm_address(),
        };

        let command =
            SessionCommand::SendAddonIfRegisteredLikeCpp(SendAddonIfRegisteredLikeCppCommand {
                prefix,
                packet_bytes: chat.to_bytes(),
            });
        let _ = command_tx.try_send(command);
    }

    // ── Helpers ──────────────────────────────────────────────────

    fn player_name_and_guid(&self) -> (wow_core::ObjectGuid, String) {
        let guid = self.player_guid().unwrap_or(wow_core::ObjectGuid::EMPTY);
        let name = self.player_name_like_cpp().unwrap_or_default().to_string();
        (guid, name)
    }

    fn has_gm_silence_aura_like_cpp(&self) -> bool {
        self.visible_auras
            .values()
            .any(|aura| aura.spell_id == GM_SILENCE_AURA_LIKE_CPP)
    }

    fn send_gm_silence_notification_like_cpp(&self) {
        let (_, sender_name) = self.player_name_and_guid();
        self.send_packet(&PrintNotification {
            notify_text: format!("Silence is ON for {sender_name}"),
        });
    }

    fn send_wait_before_speaking_notification_if_muted_like_cpp(&self) -> bool {
        let Some(remaining_secs) = self.mute_time_remaining_secs_like_cpp() else {
            return false;
        };
        self.send_packet(&PrintNotification {
            notify_text: format!(
                "You must wait {} before speaking again.",
                secs_to_full_time_string_like_cpp(remaining_secs)
            ),
        });
        true
    }

    /// Serialize `pkt` and broadcast its bytes to all players on the same map
    /// within `range` yards (excluding the sender).
    fn broadcast_chat_packet(&self, pkt: &ChatPkt, range: f32) {
        self.broadcast_raw_packet(pkt.to_bytes(), range);
    }

    fn broadcast_group_chat_like_cpp(
        &self,
        requested_type: ChatMsg,
        language: u32,
        sender_guid: ObjectGuid,
        sender_name: String,
        text: String,
        virtual_realm: u32,
    ) {
        let Some(group) = self.current_chat_group_like_cpp(sender_guid) else {
            return;
        };

        let (chat_type, subgroup_filter) = match requested_type {
            ChatMsg::Party => {
                let chat_type = if group.is_leader_like_cpp(sender_guid) {
                    ChatMsg::PartyLeader
                } else {
                    ChatMsg::Party
                };
                (chat_type, Some(group.member_group_like_cpp(sender_guid)))
            }
            ChatMsg::Raid => {
                if !group.is_raid_group() {
                    return;
                }
                let chat_type = if group.is_leader_like_cpp(sender_guid) {
                    ChatMsg::RaidLeader
                } else {
                    ChatMsg::Raid
                };
                (chat_type, None)
            }
            ChatMsg::RaidWarning => {
                if !(group.is_raid_group() || self.party_raid_warnings_like_cpp())
                    || !(group.is_leader_like_cpp(sender_guid)
                        || group.is_assistant_like_cpp(sender_guid))
                {
                    return;
                }
                (ChatMsg::RaidWarning, None)
            }
            ChatMsg::InstanceChat => {
                let chat_type = if group.is_leader_like_cpp(sender_guid) {
                    ChatMsg::InstanceChatLeader
                } else {
                    ChatMsg::InstanceChat
                };
                (chat_type, None)
            }
            _ => return,
        };

        let chat = ChatPkt {
            msg_type: chat_type,
            language,
            sender_guid,
            sender_name,
            target_guid: ObjectGuid::EMPTY,
            target_name: String::new(),
            prefix: String::new(),
            channel: String::new(),
            text,
            virtual_realm,
        };
        self.broadcast_group_chat_packet_like_cpp(&group, subgroup_filter, chat.to_bytes());
    }

    fn broadcast_group_addon_chat_like_cpp(&self, msg_type: ChatMsg, packet: ChatAddonMessage) {
        let (sender_guid, sender_name) = self.player_name_and_guid();
        let Some(group) = self.current_chat_group_like_cpp(sender_guid) else {
            return;
        };

        let subgroup_filter = if msg_type == ChatMsg::Party {
            Some(group.member_group_like_cpp(sender_guid))
        } else {
            None
        };

        let chat = ChatPkt {
            msg_type,
            language: if packet.is_logged {
                LANG_ADDON_LOGGED_LIKE_CPP
            } else {
                LANG_ADDON_LIKE_CPP
            },
            sender_guid,
            sender_name,
            target_guid: ObjectGuid::EMPTY,
            target_name: String::new(),
            prefix: packet.prefix.clone(),
            channel: String::new(),
            text: packet.text,
            virtual_realm: self.virtual_realm_address(),
        };
        self.broadcast_group_addon_packet_like_cpp(
            &group,
            subgroup_filter,
            sender_guid,
            packet.prefix,
            chat.to_bytes(),
        );
    }

    fn current_chat_group_like_cpp(&self, sender_guid: ObjectGuid) -> Option<GroupInfo> {
        let registry = self.group_registry()?;
        if let Some(group_guid) = self.group_guid
            && let Some(group) = registry.get(&group_guid)
            && group.members.contains(&sender_guid)
        {
            return Some(group.clone());
        }

        registry
            .iter()
            .find(|entry| entry.value().members.contains(&sender_guid))
            .map(|entry| entry.value().clone())
    }

    fn broadcast_group_chat_packet_like_cpp(
        &self,
        group: &GroupInfo,
        subgroup_filter: Option<u8>,
        bytes: Vec<u8>,
    ) {
        let Some(registry) = self.player_registry() else {
            return;
        };

        for member_guid in &group.members {
            if let Some(subgroup) = subgroup_filter
                && group.member_group_like_cpp(*member_guid) != subgroup
            {
                continue;
            }

            if let Some(member) = registry.get(member_guid) {
                let _ = member.send_tx.send(bytes.clone());
            }
        }
    }

    fn broadcast_group_addon_packet_like_cpp(
        &self,
        group: &GroupInfo,
        subgroup_filter: Option<u8>,
        sender_guid: ObjectGuid,
        prefix: String,
        bytes: Vec<u8>,
    ) {
        let Some(registry) = self.player_registry() else {
            return;
        };

        for member_guid in &group.members {
            if *member_guid == sender_guid {
                continue;
            }
            if let Some(subgroup) = subgroup_filter
                && group.member_group_like_cpp(*member_guid) != subgroup
            {
                continue;
            }

            if let Some(member) = registry.get(member_guid) {
                let command = SessionCommand::SendAddonIfRegisteredLikeCpp(
                    SendAddonIfRegisteredLikeCppCommand {
                        prefix: prefix.clone(),
                        packet_bytes: bytes.clone(),
                    },
                );
                let _ = member.command_tx.try_send(command);
            }
        }
    }

    /// Send pre-serialised packet `bytes` to all players on the same map
    /// within `range` yards, excluding this session's player.
    fn broadcast_raw_packet(&self, bytes: Vec<u8>, range: f32) {
        let registry = match self.player_registry() {
            Some(r) => r,
            None => return,
        };

        let sender_guid = self.player_guid().unwrap_or(ObjectGuid::EMPTY);
        let sender_pos = self.player_position_like_cpp();
        let sender_map = self.player_map_id_like_cpp();
        let range_sq = range * range;

        for entry in registry.iter() {
            // Skip self.
            if *entry.key() == sender_guid {
                continue;
            }

            let info = entry.value();

            // Must be on the same map.
            if info.map_id != sender_map {
                continue;
            }

            // Distance check.
            if let Some(sp) = sender_pos {
                let dx = sp.x - info.position.x;
                let dy = sp.y - info.position.y;
                let dz = sp.z - info.position.z;
                if dx * dx + dy * dy + dz * dz > range_sq {
                    continue;
                }
            }

            let _ = info.send_tx.send(bytes.clone());
        }
    }
}

fn chat_msg_from_i32_like_cpp(value: i32) -> Option<ChatMsg> {
    if value == ChatMsg::Party as i32 {
        Some(ChatMsg::Party)
    } else if value == ChatMsg::Raid as i32 {
        Some(ChatMsg::Raid)
    } else if value == ChatMsg::Guild as i32 {
        Some(ChatMsg::Guild)
    } else if value == ChatMsg::Officer as i32 {
        Some(ChatMsg::Officer)
    } else if value == ChatMsg::Whisper as i32 {
        Some(ChatMsg::Whisper)
    } else if value == ChatMsg::Channel as i32 {
        Some(ChatMsg::Channel)
    } else if value == ChatMsg::InstanceChat as i32 {
        Some(ChatMsg::InstanceChat)
    } else {
        None
    }
}

fn is_known_language_like_cpp(language: i32) -> bool {
    KNOWN_LANGUAGES_LIKE_CPP.contains(&language)
}

impl WorldSession {
    fn required_chat_level_like_cpp(&self, msg_type: ChatMsg) -> Option<u8> {
        let requirements = self.chat_level_requirements_like_cpp();
        match msg_type {
            ChatMsg::Say => Some(requirements.say),
            ChatMsg::Yell => Some(requirements.yell),
            _ => None,
        }
    }

    fn meets_chat_level_req_like_cpp(&self, msg_type: ChatMsg) -> bool {
        self.required_chat_level_like_cpp(msg_type)
            .is_none_or(|required| self.player_level_like_cpp() >= required)
    }

    fn meets_whisper_level_req_like_cpp(&self) -> bool {
        self.player_is_game_master_like_cpp()
            || self.player_level_like_cpp() >= self.chat_level_requirements_like_cpp().whisper
    }

    fn send_whisper_away_reply_like_cpp(&mut self, target_name: &str, auto_reply: &str, afk: bool) {
        let text = if afk {
            // C++ `LANG_PLAYER_AFK`: "%s is Away from Keyboard: %s".
            format!("{target_name} is Away from Keyboard: {auto_reply}")
        } else {
            // C++ `LANG_PLAYER_DND`: "%s wishes to not be disturbed and cannot receive whisper messages: %s".
            format!(
                "{target_name} wishes to not be disturbed and cannot receive whisper messages: {auto_reply}"
            )
        };
        let packet = ChatPkt {
            msg_type: ChatMsg::System,
            language: LANG_UNIVERSAL_LIKE_CPP as u32,
            sender_guid: ObjectGuid::EMPTY,
            sender_name: String::new(),
            target_guid: ObjectGuid::EMPTY,
            target_name: String::new(),
            prefix: String::new(),
            channel: String::new(),
            text,
            virtual_realm: self.virtual_realm_address(),
        };
        self.send_packet(&packet);
    }

    fn send_chat_say_level_notification_like_cpp(&self, required_level: u8) {
        self.send_packet(&PrintNotification {
            notify_text: format!(
                "You cannot say, yell or emote until you become level {required_level}."
            ),
        });
    }

    fn send_chat_whisper_level_notification_like_cpp(&self, required_level: u8) {
        self.send_packet(&PrintNotification {
            notify_text: format!("You cannot whisper until you become level {required_level}."),
        });
    }
}

fn secs_to_full_time_string_like_cpp(time_in_secs: u64) -> String {
    const MINUTE: u64 = 60;
    const HOUR: u64 = 60 * MINUTE;
    const DAY: u64 = 24 * HOUR;

    let secs = time_in_secs % MINUTE;
    let minutes = time_in_secs % HOUR / MINUTE;
    let hours = time_in_secs % DAY / HOUR;
    let days = time_in_secs / DAY;

    let mut text = String::new();
    if days != 0 {
        text.push_str(&days.to_string());
        text.push_str(if days == 1 { " Day " } else { " Days " });
    }
    if hours != 0 {
        text.push_str(&hours.to_string());
        text.push_str(if hours <= 1 { " Hour " } else { " Hours " });
    }
    if minutes != 0 {
        text.push_str(&minutes.to_string());
        text.push_str(if minutes == 1 {
            " Minute "
        } else {
            " Minutes "
        });
    }
    if secs != 0 || (days == 0 && hours == 0 && minutes == 0) {
        text.push_str(&secs.to_string());
        text.push_str(if secs <= 1 { " Second." } else { " Seconds." });
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::AuraApplication;
    use std::collections::{HashMap, HashSet};
    use std::sync::Arc;
    use wow_network::{
        ChatFloodConfigLikeCpp, ChatLevelRequirementsLikeCpp, PendingInvites, PlayerBroadcastInfo,
        PlayerRegistry,
    };

    const LANG_COMMON_LIKE_CPP: i32 = 7;

    fn chat_message_packet(opcode: ClientOpcodes, text: &str) -> wow_packet::WorldPacket {
        chat_message_packet_with_language(opcode, LANG_COMMON_LIKE_CPP, text)
    }

    fn chat_message_packet_with_language(
        opcode: ClientOpcodes,
        language: i32,
        text: &str,
    ) -> wow_packet::WorldPacket {
        let mut writer = wow_packet::WorldPacket::new_empty();
        writer.write_uint16(opcode as u16);
        writer.write_int32(language);
        writer.write_bits(text.len() as u32, 11);
        writer.write_bit(false);
        writer.write_string(text);

        let mut reader = wow_packet::WorldPacket::from_bytes(writer.data());
        reader.skip_opcode();
        reader
    }

    fn chat_channel_message_packet(target: &str, text: &str) -> wow_packet::WorldPacket {
        let mut writer = wow_packet::WorldPacket::new_empty();
        writer.write_int32(LANG_COMMON_LIKE_CPP);
        writer.write_packed_guid(&ObjectGuid::EMPTY);
        writer.write_bits(target.len() as u32, 9);
        writer.write_bits(text.len() as u32, 11);
        writer.write_bit(false);
        writer.write_string(target);
        writer.write_string(text);
        wow_packet::WorldPacket::from_bytes(writer.data())
    }

    fn chat_addon_packet(msg_type: ChatMsg, prefix: &str, text: &str) -> wow_packet::WorldPacket {
        let mut writer = wow_packet::WorldPacket::new_empty();
        writer.write_bits(prefix.len() as u32, 5);
        writer.write_bits(text.len() as u32, 8);
        writer.write_bit(false);
        writer.write_int32(msg_type as i32);
        writer.write_string(prefix);
        writer.write_string(text);
        wow_packet::WorldPacket::from_bytes(writer.data())
    }

    fn chat_addon_targeted_packet(
        msg_type: ChatMsg,
        prefix: &str,
        text: &str,
        target: &str,
        channel_guid: ObjectGuid,
        is_logged: bool,
    ) -> wow_packet::WorldPacket {
        let mut writer = wow_packet::WorldPacket::new_empty();
        writer.write_bits(target.len() as u32, 9);
        writer.write_bits(prefix.len() as u32, 5);
        writer.write_bits(text.len() as u32, 8);
        writer.write_bit(is_logged);
        writer.write_int32(msg_type as i32);
        writer.write_string(prefix);
        writer.write_string(text);
        writer.write_packed_guid(&channel_guid);
        writer.write_string(target);
        wow_packet::WorldPacket::from_bytes(writer.data())
    }

    fn chat_addon_whisper_packet(
        target: &str,
        prefix: &str,
        message: &str,
    ) -> wow_packet::WorldPacket {
        let mut writer = wow_packet::WorldPacket::new_empty();
        writer.write_bits(target.len() as u32, 9);
        writer.write_bits(prefix.len() as u32, 5);
        writer.write_bits(message.len() as u32, 8);
        writer.write_string(target);
        writer.write_string(prefix);
        writer.write_string(message);
        wow_packet::WorldPacket::from_bytes(writer.data())
    }

    fn chat_whisper_packet(target: &str, text: &str) -> wow_packet::WorldPacket {
        chat_whisper_packet_with_language(target, LANG_COMMON_LIKE_CPP, text)
    }

    fn chat_whisper_packet_with_language(
        target: &str,
        language: i32,
        text: &str,
    ) -> wow_packet::WorldPacket {
        let mut writer = wow_packet::WorldPacket::new_empty();
        writer.write_int32(language);
        writer.write_bits(target.len() as u32, 9);
        writer.write_bits(text.len() as u32, 11);
        writer.write_string(target);
        writer.write_string(text);
        wow_packet::WorldPacket::from_bytes(writer.data())
    }

    fn chat_away_packet(text: &str) -> wow_packet::WorldPacket {
        let mut writer = wow_packet::WorldPacket::new_empty();
        writer.write_bits(text.len() as u32, 11);
        writer.write_string(text);
        wow_packet::WorldPacket::from_bytes(writer.data())
    }

    fn chat_emote_packet(text: &str) -> wow_packet::WorldPacket {
        let mut writer = wow_packet::WorldPacket::new_empty();
        writer.write_bits(text.len() as u32, 11);
        writer.write_string(text);
        wow_packet::WorldPacket::from_bytes(writer.data())
    }

    fn chat_register_addon_prefixes_packet(prefixes: &[&str]) -> wow_packet::WorldPacket {
        let mut writer = wow_packet::WorldPacket::new_empty();
        writer.write_uint32(prefixes.len() as u32);
        for prefix in prefixes {
            writer.write_bits(prefix.len() as u32, 5);
            writer.write_string(prefix);
        }
        wow_packet::WorldPacket::from_bytes(writer.data())
    }

    fn chat_slash_cmd(bytes: &[u8]) -> u8 {
        let mut packet = wow_packet::WorldPacket::from_bytes(bytes);
        packet.skip_opcode();
        packet.read_uint8().expect("chat slash command")
    }

    fn chat_language(bytes: &[u8]) -> u32 {
        let mut packet = wow_packet::WorldPacket::from_bytes(bytes);
        packet.skip_opcode();
        let _ = packet.read_uint8().expect("chat slash command");
        packet.read_uint32().expect("chat language")
    }

    fn chat_text(bytes: &[u8]) -> String {
        let mut packet = wow_packet::WorldPacket::from_bytes(bytes);
        packet.skip_opcode();
        let _ = packet.read_uint8().expect("chat slash command");
        let _ = packet.read_uint32().expect("chat language");
        let _ = packet.read_packed_guid().expect("sender guid");
        let _ = packet.read_packed_guid().expect("sender guild guid");
        let _ = packet.read_packed_guid().expect("sender account guid");
        let _ = packet.read_packed_guid().expect("target guid");
        let _ = packet.read_uint32().expect("target virtual realm");
        let _ = packet.read_uint32().expect("sender virtual realm");
        let _ = packet.read_int32().expect("achievement id");
        let _ = packet.read_float().expect("display time");
        let _ = packet.read_int32().expect("spell id");
        let sender_len = packet.read_bits(11).expect("sender len") as usize;
        let target_len = packet.read_bits(11).expect("target len") as usize;
        let prefix_len = packet.read_bits(5).expect("prefix len") as usize;
        let channel_len = packet.read_bits(7).expect("channel len") as usize;
        let text_len = packet.read_bits(12).expect("text len") as usize;
        let _ = packet.read_bits(15).expect("chat flags");
        let _ = packet.read_bit().expect("hide chat log");
        let _ = packet.read_bit().expect("fake sender");
        let _ = packet.read_bit().expect("unused 801");
        let _ = packet.read_bit().expect("channel guid");
        packet.flush_bits();
        let _ = packet.read_string(sender_len).expect("sender name");
        let _ = packet.read_string(target_len).expect("target name");
        let _ = packet.read_string(prefix_len).expect("prefix");
        let _ = packet.read_string(channel_len).expect("channel");
        packet.read_string(text_len).expect("text")
    }

    fn print_notification_text(bytes: &[u8]) -> String {
        let mut packet = wow_packet::WorldPacket::from_bytes(bytes);
        assert_eq!(
            packet.read_uint16().expect("opcode"),
            wow_constants::ServerOpcodes::PrintNotification as u16
        );
        let text_len = packet.read_bits(12).expect("notify text len") as usize;
        let text = packet.read_string(text_len).expect("notify text");
        assert!(packet.is_empty());
        text
    }

    fn mute_session_for_seconds_like_cpp(session: &mut WorldSession, seconds: u64) {
        let mute_until = wow_core::GameTime::now().as_secs().saturating_add(seconds) as i64;
        session.set_mute_time_like_cpp(mute_until);
    }

    fn broadcast_info(guid: ObjectGuid, send_tx: flume::Sender<Vec<u8>>) -> PlayerBroadcastInfo {
        let (command_tx, _command_rx) = flume::bounded(8);
        broadcast_info_with_command_tx(guid, send_tx, command_tx)
    }

    fn broadcast_info_with_command_tx(
        guid: ObjectGuid,
        send_tx: flume::Sender<Vec<u8>>,
        command_tx: flume::Sender<SessionCommand>,
    ) -> PlayerBroadcastInfo {
        PlayerBroadcastInfo {
            map_id: 571,
            instance_id: 0,
            position: wow_core::Position::ZERO,
            combat_reach: 0.0,
            liquid_status: 0,
            is_in_world: true,
            send_tx,
            command_tx,
            active_loot_rolls: Vec::new(),
            pass_on_group_loot: false,
            enchanting_skill: 0,
            is_alive: true,
            current_health: 100,
            max_health: 100,
            power_type: 0,
            current_power: 0,
            max_power: 0,
            is_pvp: false,
            is_ffa_pvp: false,
            is_ghost: false,
            is_afk: false,
            is_dnd: false,
            auto_reply_msg_like_cpp: String::new(),
            in_vehicle: false,
            has_vehicle_kit_like_cpp: false,
            party_member_vehicle_seat: 0,
            zone_id: 0,
            spec_id: 0,
            unit_flags: 0,
            unit_flags2: 0,
            unit_state: 0,
            is_game_master: false,
            is_contested_pvp: false,
            active_expansion: 2,
            pending_quest_sharing: None,
            known_spells: Vec::new(),
            active_quest_statuses: HashMap::new(),
            active_quest_objective_counts: HashMap::new(),
            rewarded_quests: HashSet::new(),
            completed_achievements: HashSet::new(),
            daily_quests_completed: HashSet::new(),
            df_quests: HashSet::new(),
            faction_template_id: 0,
            reputation_standings: Vec::new(),
            reputation_state_flags: Vec::new(),
            forced_reputation_ranks: Vec::new(),
            forced_reputation_faction_ids: Vec::new(),
            inventory_item_counts: HashMap::new(),
            party_member_party_type: [0; 2],
            party_member_phase_states: Default::default(),
            party_member_auras: Vec::new(),
            party_member_pet_stats: None,
            player_name: format!("Player{}", guid.counter()),
            account_id: guid.counter() as u32,
            recruiter_id: 0,
            race: 1,
            class: 1,
            sex: 0,
            level: 80,
            gray_level: 0,
            display_id: 49,
            visible_items: [(0, 0, 0); 19],
            lifetime_honorable_kills: 0,
            this_week_contribution: 0,
            yesterday_contribution: 0,
            today_honorable_kills: 0,
            yesterday_honorable_kills: 0,
            lifetime_max_rank: 0,
            honor_level: 0,
        }
    }

    fn gm_silence_aura(slot: u8) -> AuraApplication {
        AuraApplication {
            spell_id: GM_SILENCE_AURA_LIKE_CPP,
            caster_guid: ObjectGuid::EMPTY,
            slot,
            duration_total: 30_000,
            duration_remaining: 30_000,
            stack_count: 1,
            aura_flags: 0x1,
            effect_mask: 0x1,
            aura_interrupt_flags: 0,
            aura_interrupt_flags2: 0,
            represented_effect: None,
            represented_amount: 0,
            represented_effect_amounts: Vec::new(),
            represented_misc_value: None,
            represented_multiplier: 1.0,
            applied_at: std::time::Instant::now(),
        }
    }

    fn expect_addon_command(
        rx: &flume::Receiver<SessionCommand>,
    ) -> SendAddonIfRegisteredLikeCppCommand {
        match rx.try_recv().expect("addon command") {
            SessionCommand::SendAddonIfRegisteredLikeCpp(command) => command,
            other => panic!("expected SendAddonIfRegisteredLikeCpp, got {other:?}"),
        }
    }

    fn session_for_chat_routing_like_cpp(
        sender_guid: ObjectGuid,
    ) -> (WorldSession, Arc<PlayerRegistry>, flume::Receiver<Vec<u8>>) {
        let (packet_tx, packet_rx) = flume::bounded(8);
        drop(packet_tx);
        let (send_tx, send_rx) = flume::bounded(8);
        let mut session = WorldSession::new(
            1,
            "TestAccount".to_string(),
            0,
            2,
            9,
            54261,
            vec![0; 40],
            "enUS".to_string(),
            packet_rx,
            send_tx.clone(),
        );
        session.set_player_guid(Some(sender_guid));
        session.set_loaded_player_name_like_cpp(format!("Player{}", sender_guid.counter()));
        session.set_loaded_player_identity_like_cpp(571, 1, 1, 80, 0);
        session.set_player_map_position_like_cpp(571, wow_core::Position::ZERO);

        let player_registry = Arc::new(PlayerRegistry::default());
        player_registry.insert(sender_guid, broadcast_info(sender_guid, send_tx));
        session.set_player_registry(Arc::clone(&player_registry));
        (session, player_registry, send_rx)
    }

    #[tokio::test]
    async fn chat_report_filtered_empty_stub_sends_no_response_like_cpp() {
        let sender = ObjectGuid::create_player(1, 100);
        let (mut session, _registry, send_rx) = session_for_chat_routing_like_cpp(sender);

        session
            .handle_chat_report_filtered(wow_packet::WorldPacket::new_empty())
            .await;

        assert!(send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn chat_report_ignored_notifies_ignored_player_like_cpp() {
        let reporter = ObjectGuid::create_player(1, 101);
        let ignored = ObjectGuid::create_player(1, 102);
        let (mut session, registry, reporter_rx) = session_for_chat_routing_like_cpp(reporter);
        let (ignored_tx, ignored_rx) = flume::bounded(8);
        registry.insert(ignored, broadcast_info(ignored, ignored_tx));

        let mut writer = wow_packet::WorldPacket::new_empty();
        writer.write_packed_guid(&ignored);
        writer.write_uint8(0);
        session
            .handle_chat_report_ignored(wow_packet::WorldPacket::from_bytes(writer.data()))
            .await;

        assert_eq!(
            chat_slash_cmd(&ignored_rx.try_recv().expect("ignored notification")),
            ChatMsg::Ignored as u8
        );
        assert!(reporter_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn chat_register_addon_prefixes_accumulates_and_updates_filter_like_cpp() {
        let sender = ObjectGuid::create_player(1, 103);
        let (mut session, _registry, send_rx) = session_for_chat_routing_like_cpp(sender);

        session
            .handle_chat_register_addon_prefixes(chat_register_addon_prefixes_packet(&[
                "ABC", "DEF",
            ]))
            .await;
        assert_eq!(session.registered_addon_prefixes, vec!["ABC", "DEF"]);
        assert!(session.filter_addon_messages);

        let too_many = vec!["X"; ChatRegisterAddonPrefixes::MAX_PREFIXES - 1];
        session
            .handle_chat_register_addon_prefixes(chat_register_addon_prefixes_packet(&too_many))
            .await;
        assert_eq!(
            session.registered_addon_prefixes.len(),
            ChatRegisterAddonPrefixes::MAX_PREFIXES + 1
        );
        assert!(!session.filter_addon_messages);
        assert!(send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn party_chat_routes_only_to_sender_subgroup_like_cpp() {
        let leader = ObjectGuid::create_player(1, 101);
        let same_subgroup = ObjectGuid::create_player(1, 102);
        let other_subgroup = ObjectGuid::create_player(1, 103);
        let (mut session, player_registry, leader_rx) = session_for_chat_routing_like_cpp(leader);
        let (same_tx, same_rx) = flume::bounded(8);
        let (other_tx, other_rx) = flume::bounded(8);
        player_registry.insert(same_subgroup, broadcast_info(same_subgroup, same_tx));
        player_registry.insert(other_subgroup, broadcast_info(other_subgroup, other_tx));

        let mut group = GroupInfo::new(leader);
        group.convert_to_raid_like_cpp();
        group.add_member(same_subgroup);
        group.add_member(other_subgroup);
        assert!(group.change_member_group_like_cpp(other_subgroup, 1));
        let group_guid = group.group_guid;
        let group_registry = Arc::new(wow_network::GroupRegistry::default());
        group_registry.insert(group_guid, group);
        session.group_guid = Some(group_guid);
        session.set_group_registry(group_registry, Arc::new(PendingInvites::default()));

        session
            .handle_chat_message(
                chat_message_packet(ClientOpcodes::ChatMessageParty, "party"),
                ChatMsg::Party,
            )
            .await;

        assert_eq!(
            chat_slash_cmd(&leader_rx.try_recv().expect("leader party echo")),
            ChatMsg::PartyLeader as u8
        );
        assert_eq!(
            chat_slash_cmd(&same_rx.try_recv().expect("same subgroup party chat")),
            ChatMsg::PartyLeader as u8
        );
        assert!(other_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn raid_chat_routes_to_all_raid_members_like_cpp() {
        let leader = ObjectGuid::create_player(1, 201);
        let member = ObjectGuid::create_player(1, 202);
        let (mut session, player_registry, leader_rx) = session_for_chat_routing_like_cpp(leader);
        let (member_tx, member_rx) = flume::bounded(8);
        player_registry.insert(member, broadcast_info(member, member_tx));

        let mut group = GroupInfo::new(leader);
        group.convert_to_raid_like_cpp();
        group.add_member(member);
        let group_guid = group.group_guid;
        let group_registry = Arc::new(wow_network::GroupRegistry::default());
        group_registry.insert(group_guid, group);
        session.group_guid = Some(group_guid);
        session.set_group_registry(group_registry, Arc::new(PendingInvites::default()));

        session
            .handle_chat_message(
                chat_message_packet(ClientOpcodes::ChatMessageRaid, "raid"),
                ChatMsg::Raid,
            )
            .await;

        assert_eq!(
            chat_slash_cmd(&leader_rx.try_recv().expect("leader raid echo")),
            ChatMsg::RaidLeader as u8
        );
        assert_eq!(
            chat_slash_cmd(&member_rx.try_recv().expect("member raid chat")),
            ChatMsg::RaidLeader as u8
        );
    }

    #[tokio::test]
    async fn raid_warning_in_party_requires_party_raid_warnings_config_like_cpp() {
        let leader = ObjectGuid::create_player(1, 203);
        let member = ObjectGuid::create_player(1, 204);
        let (mut session, player_registry, leader_rx) = session_for_chat_routing_like_cpp(leader);
        let (member_tx, member_rx) = flume::bounded(8);
        player_registry.insert(member, broadcast_info(member, member_tx));

        let mut group = GroupInfo::new(leader);
        group.add_member(member);
        let group_guid = group.group_guid;
        let group_registry = Arc::new(wow_network::GroupRegistry::default());
        group_registry.insert(group_guid, group);
        session.group_guid = Some(group_guid);
        session.set_group_registry(group_registry, Arc::new(PendingInvites::default()));

        session
            .handle_chat_message(
                chat_message_packet(ClientOpcodes::ChatMessageRaidWarning, "blocked"),
                ChatMsg::RaidWarning,
            )
            .await;

        assert!(leader_rx.try_recv().is_err());
        assert!(member_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn party_raid_warnings_config_allows_party_raid_warning_like_cpp() {
        let leader = ObjectGuid::create_player(1, 205);
        let member = ObjectGuid::create_player(1, 206);
        let (mut session, player_registry, leader_rx) = session_for_chat_routing_like_cpp(leader);
        let (member_tx, member_rx) = flume::bounded(8);
        player_registry.insert(member, broadcast_info(member, member_tx));

        let mut group = GroupInfo::new(leader);
        group.add_member(member);
        let group_guid = group.group_guid;
        let group_registry = Arc::new(wow_network::GroupRegistry::default());
        group_registry.insert(group_guid, group);
        session.group_guid = Some(group_guid);
        session.set_group_registry(group_registry, Arc::new(PendingInvites::default()));
        session.set_party_raid_warnings_like_cpp(true);

        session
            .handle_chat_message(
                chat_message_packet(ClientOpcodes::ChatMessageRaidWarning, "party warning"),
                ChatMsg::RaidWarning,
            )
            .await;

        assert_eq!(
            chat_slash_cmd(&leader_rx.try_recv().expect("leader warning")),
            ChatMsg::RaidWarning as u8
        );
        assert_eq!(
            chat_slash_cmd(&member_rx.try_recv().expect("member warning")),
            ChatMsg::RaidWarning as u8
        );
    }

    #[tokio::test]
    async fn guild_chat_does_not_leak_to_nearby_players_without_guild_registry_like_cpp() {
        let sender = ObjectGuid::create_player(1, 301);
        let nearby = ObjectGuid::create_player(1, 302);
        let (mut session, player_registry, sender_rx) = session_for_chat_routing_like_cpp(sender);
        let (nearby_tx, nearby_rx) = flume::bounded(8);
        player_registry.insert(nearby, broadcast_info(nearby, nearby_tx));

        session
            .handle_chat_message(
                chat_message_packet(ClientOpcodes::ChatMessageGuild, "guild"),
                ChatMsg::Guild,
            )
            .await;

        assert!(sender_rx.try_recv().is_err());
        assert!(nearby_rx.try_recv().is_err());
        assert!(!session.is_disconnecting());
    }

    #[tokio::test]
    async fn channel_chat_does_not_leak_without_channel_mgr_like_cpp() {
        let sender = ObjectGuid::create_player(1, 321);
        let nearby = ObjectGuid::create_player(1, 322);
        let (mut session, player_registry, sender_rx) = session_for_chat_routing_like_cpp(sender);
        let (nearby_tx, nearby_rx) = flume::bounded(8);
        player_registry.insert(nearby, broadcast_info(nearby, nearby_tx));

        session
            .handle_chat_channel_message(chat_channel_message_packet("General", "channel"))
            .await;

        assert!(sender_rx.try_recv().is_err());
        assert!(nearby_rx.try_recv().is_err());
        assert!(!session.is_disconnecting());
    }

    #[tokio::test]
    async fn update_aadc_status_forces_chat_enabled_like_cpp() {
        let sender = ObjectGuid::create_player(1, 331);
        let (mut session, _player_registry, sender_rx) = session_for_chat_routing_like_cpp(sender);
        let mut writer = wow_packet::WorldPacket::new_empty();
        writer.write_bit(true);
        writer.flush_bits();

        session
            .handle_update_aadc_status(wow_packet::WorldPacket::from_bytes(writer.data()))
            .await;

        let bytes = sender_rx.try_recv().expect("AADC status response");
        let mut response = wow_packet::WorldPacket::from_bytes(&bytes);
        assert_eq!(
            response.read_uint16().expect("opcode"),
            wow_constants::ServerOpcodes::UpdateAadcStatusResponse as u16
        );
        assert!(response.read_bit().expect("success"));
        assert!(!response.read_bit().expect("chat disabled"));
        assert!(response.is_empty());
    }

    #[tokio::test]
    async fn officer_chat_does_not_leak_to_nearby_players_without_guild_registry_like_cpp() {
        let sender = ObjectGuid::create_player(1, 311);
        let nearby = ObjectGuid::create_player(1, 312);
        let (mut session, player_registry, sender_rx) = session_for_chat_routing_like_cpp(sender);
        let (nearby_tx, nearby_rx) = flume::bounded(8);
        player_registry.insert(nearby, broadcast_info(nearby, nearby_tx));

        session
            .handle_chat_message(
                chat_message_packet(ClientOpcodes::ChatMessageOfficer, "officer"),
                ChatMsg::Officer,
            )
            .await;

        assert!(sender_rx.try_recv().is_err());
        assert!(nearby_rx.try_recv().is_err());
        assert!(!session.is_disconnecting());
    }

    #[tokio::test]
    async fn chat_strict_link_checking_kick_disconnects_on_invalid_link_like_cpp() {
        let sender = ObjectGuid::create_player(1, 353);
        let nearby = ObjectGuid::create_player(1, 354);
        let (mut session, player_registry, sender_rx) = session_for_chat_routing_like_cpp(sender);
        let (nearby_tx, nearby_rx) = flume::bounded(8);
        player_registry.insert(nearby, broadcast_info(nearby, nearby_tx));
        session.set_chat_strict_link_checking_kick_like_cpp(true);

        session
            .handle_chat_message(
                chat_message_packet(ClientOpcodes::ChatMessageSay, "forged |x control"),
                ChatMsg::Say,
            )
            .await;

        assert!(sender_rx.try_recv().is_err());
        assert!(nearby_rx.try_recv().is_err());
        assert!(session.is_disconnecting());
    }

    #[tokio::test]
    async fn chat_message_truncates_at_newline_like_cpp() {
        let sender = ObjectGuid::create_player(1, 331);
        let (mut session, _player_registry, sender_rx) = session_for_chat_routing_like_cpp(sender);

        session
            .handle_chat_message(
                chat_message_packet(ClientOpcodes::ChatMessageSay, "visible\nhidden"),
                ChatMsg::Say,
            )
            .await;

        let delivered = sender_rx.try_recv().expect("sender echo");
        assert_eq!(chat_text(&delivered), "visible");
    }

    #[tokio::test]
    async fn chat_with_invalid_hyperlink_control_sequence_is_rejected_like_cpp() {
        let sender = ObjectGuid::create_player(1, 351);
        let nearby = ObjectGuid::create_player(1, 352);
        let (mut session, player_registry, sender_rx) = session_for_chat_routing_like_cpp(sender);
        let (nearby_tx, nearby_rx) = flume::bounded(8);
        player_registry.insert(nearby, broadcast_info(nearby, nearby_tx));

        session
            .handle_chat_message(
                chat_message_packet(ClientOpcodes::ChatMessageSay, "forged |x control"),
                ChatMsg::Say,
            )
            .await;

        assert!(sender_rx.try_recv().is_err());
        assert!(nearby_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn chat_message_rejects_client_universal_language_like_cpp() {
        let sender = ObjectGuid::create_player(1, 353);
        let nearby = ObjectGuid::create_player(1, 354);
        let (mut session, player_registry, sender_rx) = session_for_chat_routing_like_cpp(sender);
        let (nearby_tx, nearby_rx) = flume::bounded(8);
        player_registry.insert(nearby, broadcast_info(nearby, nearby_tx));

        session
            .handle_chat_message(
                chat_message_packet_with_language(
                    ClientOpcodes::ChatMessageSay,
                    LANG_UNIVERSAL_LIKE_CPP,
                    "universal",
                ),
                ChatMsg::Say,
            )
            .await;

        assert!(sender_rx.try_recv().is_err());
        assert!(nearby_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn chat_message_rejects_unknown_language_like_cpp() {
        let sender = ObjectGuid::create_player(1, 370);
        let nearby = ObjectGuid::create_player(1, 371);
        let (mut session, player_registry, sender_rx) = session_for_chat_routing_like_cpp(sender);
        let (nearby_tx, nearby_rx) = flume::bounded(8);
        player_registry.insert(nearby, broadcast_info(nearby, nearby_tx));

        session
            .handle_chat_message(
                chat_message_packet_with_language(
                    ClientOpcodes::ChatMessageSay,
                    999,
                    "unknown language",
                ),
                ChatMsg::Say,
            )
            .await;

        assert!(sender_rx.try_recv().is_err());
        assert!(nearby_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn dead_player_cannot_send_say_like_cpp() {
        let sender = ObjectGuid::create_player(1, 357);
        let nearby = ObjectGuid::create_player(1, 358);
        let (mut session, player_registry, sender_rx) = session_for_chat_routing_like_cpp(sender);
        let (nearby_tx, nearby_rx) = flume::bounded(8);
        player_registry.insert(nearby, broadcast_info(nearby, nearby_tx));
        session.set_player_alive_like_cpp(false);

        session
            .handle_chat_message(
                chat_message_packet(ClientOpcodes::ChatMessageSay, "dead say"),
                ChatMsg::Say,
            )
            .await;

        assert!(sender_rx.try_recv().is_err());
        assert!(nearby_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn low_level_player_cannot_send_say_or_yell_like_cpp() {
        let sender = ObjectGuid::create_player(1, 375);
        let nearby = ObjectGuid::create_player(1, 376);
        let (mut session, player_registry, sender_rx) = session_for_chat_routing_like_cpp(sender);
        let (nearby_tx, nearby_rx) = flume::bounded(8);
        player_registry.insert(nearby, broadcast_info(nearby, nearby_tx));
        session.set_player_level_like_cpp(0);

        session
            .handle_chat_message(
                chat_message_packet(ClientOpcodes::ChatMessageSay, "too low say"),
                ChatMsg::Say,
            )
            .await;
        session
            .handle_chat_message(
                chat_message_packet(ClientOpcodes::ChatMessageYell, "too low yell"),
                ChatMsg::Yell,
            )
            .await;

        assert_eq!(
            print_notification_text(&sender_rx.try_recv().expect("say level notification")),
            "You cannot say, yell or emote until you become level 1."
        );
        assert_eq!(
            print_notification_text(&sender_rx.try_recv().expect("yell level notification")),
            "You cannot say, yell or emote until you become level 1."
        );
        assert!(sender_rx.try_recv().is_err());
        assert!(nearby_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn configured_chat_level_requirements_gate_say_yell_and_emote_like_cpp() {
        let sender = ObjectGuid::create_player(1, 385);
        let nearby = ObjectGuid::create_player(1, 386);
        let (mut session, player_registry, sender_rx) = session_for_chat_routing_like_cpp(sender);
        let (nearby_tx, nearby_rx) = flume::bounded(8);
        player_registry.insert(nearby, broadcast_info(nearby, nearby_tx));
        session.set_player_level_like_cpp(1);
        session.set_chat_level_requirements_like_cpp(ChatLevelRequirementsLikeCpp {
            say: 2,
            yell: 2,
            emote: 2,
            ..ChatLevelRequirementsLikeCpp::default()
        });

        session
            .handle_chat_message(
                chat_message_packet(ClientOpcodes::ChatMessageSay, "too low say"),
                ChatMsg::Say,
            )
            .await;
        session
            .handle_chat_message(
                chat_message_packet(ClientOpcodes::ChatMessageYell, "too low yell"),
                ChatMsg::Yell,
            )
            .await;
        session
            .handle_chat_emote(chat_emote_packet("too low emote"))
            .await;

        assert_eq!(
            print_notification_text(&sender_rx.try_recv().expect("say level notification")),
            "You cannot say, yell or emote until you become level 2."
        );
        assert_eq!(
            print_notification_text(&sender_rx.try_recv().expect("yell level notification")),
            "You cannot say, yell or emote until you become level 2."
        );
        assert_eq!(
            print_notification_text(&sender_rx.try_recv().expect("emote level notification")),
            "You cannot say, yell or emote until you become level 2."
        );
        assert!(sender_rx.try_recv().is_err());
        assert!(nearby_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn say_collapses_multiple_spaces_when_fake_message_preventing_enabled_like_cpp() {
        let sender = ObjectGuid::create_player(1, 383);
        let (mut session, _player_registry, sender_rx) = session_for_chat_routing_like_cpp(sender);
        session.set_chat_fake_message_preventing_like_cpp(true);

        session
            .handle_chat_message(
                chat_message_packet(ClientOpcodes::ChatMessageSay, "hello   fake    spacing"),
                ChatMsg::Say,
            )
            .await;

        let echo = sender_rx.try_recv().expect("sender echo");
        assert_eq!(chat_slash_cmd(&echo), ChatMsg::Say as u8);
        assert_eq!(chat_text(&echo), "hello fake spacing");
    }

    #[tokio::test]
    async fn say_preserves_multiple_spaces_when_fake_message_preventing_disabled_like_cpp() {
        let sender = ObjectGuid::create_player(1, 384);
        let (mut session, _player_registry, sender_rx) = session_for_chat_routing_like_cpp(sender);

        session
            .handle_chat_message(
                chat_message_packet(ClientOpcodes::ChatMessageSay, "hello   fake    spacing"),
                ChatMsg::Say,
            )
            .await;

        let echo = sender_rx.try_recv().expect("sender echo");
        assert_eq!(chat_slash_cmd(&echo), ChatMsg::Say as u8);
        assert_eq!(chat_text(&echo), "hello   fake    spacing");
    }

    #[tokio::test]
    async fn dead_player_party_chat_is_not_rejected_by_say_alive_gate_like_cpp() {
        let leader = ObjectGuid::create_player(1, 359);
        let member = ObjectGuid::create_player(1, 360);
        let (mut session, player_registry, leader_rx) = session_for_chat_routing_like_cpp(leader);
        let (member_tx, member_rx) = flume::bounded(8);
        player_registry.insert(member, broadcast_info(member, member_tx));
        let mut group = GroupInfo::new(leader);
        group.add_member(member);
        let group_guid = group.group_guid;
        let group_registry = Arc::new(wow_network::GroupRegistry::default());
        group_registry.insert(group_guid, group);
        session.group_guid = Some(group_guid);
        session.set_group_registry(group_registry, Arc::new(PendingInvites::default()));
        session.set_player_alive_like_cpp(false);

        session
            .handle_chat_message(
                chat_message_packet(ClientOpcodes::ChatMessageParty, "dead party"),
                ChatMsg::Party,
            )
            .await;

        assert_eq!(
            chat_slash_cmd(&leader_rx.try_recv().expect("leader party echo")),
            ChatMsg::PartyLeader as u8
        );
        assert_eq!(
            chat_slash_cmd(&member_rx.try_recv().expect("member party chat")),
            ChatMsg::PartyLeader as u8
        );
    }

    #[tokio::test]
    async fn dead_player_cannot_send_chat_emote_like_cpp() {
        let sender = ObjectGuid::create_player(1, 368);
        let nearby = ObjectGuid::create_player(1, 369);
        let (mut session, player_registry, sender_rx) = session_for_chat_routing_like_cpp(sender);
        let (nearby_tx, nearby_rx) = flume::bounded(8);
        player_registry.insert(nearby, broadcast_info(nearby, nearby_tx));
        session.set_player_alive_like_cpp(false);

        session
            .handle_chat_emote(chat_emote_packet("dead emote"))
            .await;

        assert!(sender_rx.try_recv().is_err());
        assert!(nearby_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn low_level_player_cannot_send_chat_emote_like_cpp() {
        let sender = ObjectGuid::create_player(1, 377);
        let nearby = ObjectGuid::create_player(1, 378);
        let (mut session, player_registry, sender_rx) = session_for_chat_routing_like_cpp(sender);
        let (nearby_tx, nearby_rx) = flume::bounded(8);
        player_registry.insert(nearby, broadcast_info(nearby, nearby_tx));
        session.set_player_level_like_cpp(0);

        session
            .handle_chat_emote(chat_emote_packet("too low emote"))
            .await;

        assert_eq!(
            print_notification_text(&sender_rx.try_recv().expect("emote level notification")),
            "You cannot say, yell or emote until you become level 1."
        );
        assert!(sender_rx.try_recv().is_err());
        assert!(nearby_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn whisper_rejects_client_universal_language_like_cpp() {
        let sender = ObjectGuid::create_player(1, 355);
        let target = ObjectGuid::create_player(1, 356);
        let (mut session, player_registry, sender_rx) = session_for_chat_routing_like_cpp(sender);
        let (target_tx, target_rx) = flume::bounded(8);
        let mut target_info = broadcast_info(target, target_tx);
        target_info.player_name = "Target".to_string();
        player_registry.insert(target, target_info);

        session
            .handle_chat_whisper(chat_whisper_packet_with_language(
                "Target",
                LANG_UNIVERSAL_LIKE_CPP,
                "universal",
            ))
            .await;

        assert!(sender_rx.try_recv().is_err());
        assert!(target_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn whisper_rejects_unknown_language_like_cpp() {
        let sender = ObjectGuid::create_player(1, 372);
        let target = ObjectGuid::create_player(1, 373);
        let (mut session, player_registry, sender_rx) = session_for_chat_routing_like_cpp(sender);
        let (target_tx, target_rx) = flume::bounded(8);
        let mut target_info = broadcast_info(target, target_tx);
        target_info.player_name = "Target".to_string();
        player_registry.insert(target, target_info);

        session
            .handle_chat_whisper(chat_whisper_packet_with_language(
                "Target",
                999,
                "unknown language",
            ))
            .await;

        assert!(sender_rx.try_recv().is_err());
        assert!(target_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn whisper_to_offline_player_sends_notfound_like_cpp() {
        let sender = ObjectGuid::create_player(1, 374);
        let (mut session, _player_registry, sender_rx) = session_for_chat_routing_like_cpp(sender);

        session
            .handle_chat_whisper(chat_whisper_packet("Missing", "hello"))
            .await;

        let bytes = sender_rx.try_recv().expect("notfound packet");
        let mut packet = wow_packet::WorldPacket::from_bytes(&bytes);
        assert_eq!(
            packet.read_uint16().expect("opcode"),
            wow_constants::ServerOpcodes::ChatPlayerNotfound as u16
        );
        assert_eq!(packet.read_bits(9).expect("name len"), 7);
        assert_eq!(packet.read_string(7).expect("name"), "Missing");
        assert!(packet.is_empty());
    }

    #[tokio::test]
    async fn configured_whisper_level_requirement_blocks_non_gm_sender_like_cpp() {
        let sender = ObjectGuid::create_player(1, 387);
        let target = ObjectGuid::create_player(1, 388);
        let (mut session, player_registry, sender_rx) = session_for_chat_routing_like_cpp(sender);
        let (target_tx, target_rx) = flume::bounded(8);
        let mut target_info = broadcast_info(target, target_tx);
        target_info.player_name = "Target".to_string();
        player_registry.insert(target, target_info);
        session.set_player_level_like_cpp(1);
        session.set_chat_level_requirements_like_cpp(ChatLevelRequirementsLikeCpp {
            whisper: 2,
            ..ChatLevelRequirementsLikeCpp::default()
        });

        session
            .handle_chat_whisper(chat_whisper_packet("Target", "too low whisper"))
            .await;

        assert_eq!(
            print_notification_text(&sender_rx.try_recv().expect("whisper level notification")),
            "You cannot whisper until you become level 2."
        );
        assert!(sender_rx.try_recv().is_err());
        assert!(target_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn configured_whisper_level_requirement_allows_gm_sender_like_cpp() {
        let sender = ObjectGuid::create_player(1, 389);
        let target = ObjectGuid::create_player(1, 390);
        let (mut session, player_registry, sender_rx) = session_for_chat_routing_like_cpp(sender);
        let (target_tx, target_rx) = flume::bounded(8);
        let mut target_info = broadcast_info(target, target_tx);
        target_info.player_name = "Target".to_string();
        player_registry.insert(target, target_info);
        session.set_player_level_like_cpp(1);
        session.set_player_game_master_like_cpp(true);
        session.set_chat_level_requirements_like_cpp(ChatLevelRequirementsLikeCpp {
            whisper: 2,
            ..ChatLevelRequirementsLikeCpp::default()
        });

        session
            .handle_chat_whisper(chat_whisper_packet("Target", "gm whisper"))
            .await;

        assert_eq!(
            chat_slash_cmd(&target_rx.try_recv().expect("target whisper")),
            ChatMsg::Whisper as u8
        );
        assert_eq!(
            chat_slash_cmd(&sender_rx.try_recv().expect("sender inform")),
            ChatMsg::WhisperInform as u8
        );
    }

    #[tokio::test]
    async fn gm_silence_aura_rejects_non_whisper_chat_like_cpp() {
        let sender = ObjectGuid::create_player(1, 361);
        let nearby = ObjectGuid::create_player(1, 362);
        let (mut session, player_registry, sender_rx) = session_for_chat_routing_like_cpp(sender);
        let (nearby_tx, nearby_rx) = flume::bounded(8);
        player_registry.insert(nearby, broadcast_info(nearby, nearby_tx));
        session.visible_auras.insert(1, gm_silence_aura(1));

        session
            .handle_chat_message(
                chat_message_packet(ClientOpcodes::ChatMessageSay, "muted"),
                ChatMsg::Say,
            )
            .await;

        assert_eq!(
            print_notification_text(&sender_rx.try_recv().expect("silence notification")),
            "Silence is ON for Player361"
        );
        assert!(sender_rx.try_recv().is_err());
        assert!(nearby_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn account_mute_time_rejects_chat_with_wait_notification_like_cpp() {
        let sender = ObjectGuid::create_player(1, 431);
        let nearby = ObjectGuid::create_player(1, 432);
        let (mut session, player_registry, sender_rx) = session_for_chat_routing_like_cpp(sender);
        let (nearby_tx, nearby_rx) = flume::bounded(8);
        player_registry.insert(nearby, broadcast_info(nearby, nearby_tx));
        mute_session_for_seconds_like_cpp(&mut session, 3_600);

        session
            .handle_chat_message(
                chat_message_packet(ClientOpcodes::ChatMessageSay, "mutetime"),
                ChatMsg::Say,
            )
            .await;

        let text = print_notification_text(&sender_rx.try_recv().expect("mute notification"));
        assert!(text.starts_with("You must wait "));
        assert!(text.ends_with(" before speaking again."));
        assert!(sender_rx.try_recv().is_err());
        assert!(nearby_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn account_mute_time_rejects_afk_without_changing_state_like_cpp() {
        let sender = ObjectGuid::create_player(1, 433);
        let (mut session, _, sender_rx) = session_for_chat_routing_like_cpp(sender);
        mute_session_for_seconds_like_cpp(&mut session, 3_600);

        session.handle_chat_afk(chat_away_packet("away")).await;

        assert!(session.auto_reply_msg_like_cpp().is_empty());
        let text = print_notification_text(&sender_rx.try_recv().expect("mute notification"));
        assert!(text.starts_with("You must wait "));
        assert!(text.ends_with(" before speaking again."));
        assert!(sender_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn dedicated_addon_whisper_ignores_account_mute_time_like_cpp() {
        let sender = ObjectGuid::create_player(1, 434);
        let (mut session, _, sender_rx) = session_for_chat_routing_like_cpp(sender);
        mute_session_for_seconds_like_cpp(&mut session, 3_600);

        session
            .handle_chat_addon_message_whisper(chat_addon_whisper_packet(
                "Missing", "ABC", "payload",
            ))
            .await;

        let bytes = sender_rx
            .try_recv()
            .expect("dedicated addon whisper still sends missing target notice");
        let mut packet = wow_packet::WorldPacket::from_bytes(&bytes);
        assert_eq!(
            packet.read_uint16().expect("opcode"),
            wow_constants::ServerOpcodes::ChatPlayerNotfound as u16
        );
        assert_eq!(packet.read_bits(9).expect("name len"), 7);
        assert_eq!(packet.read_string(7).expect("name"), "Missing");
        assert!(sender_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn chat_flood_regular_mutes_after_limit_for_next_message_like_cpp() {
        let sender = ObjectGuid::create_player(1, 435);
        let nearby = ObjectGuid::create_player(1, 436);
        let (mut session, player_registry, sender_rx) = session_for_chat_routing_like_cpp(sender);
        let (nearby_tx, nearby_rx) = flume::bounded(8);
        player_registry.insert(nearby, broadcast_info(nearby, nearby_tx));
        session.set_chat_flood_config_like_cpp(ChatFloodConfigLikeCpp {
            message_count: 2,
            message_delay_secs: 60,
            addon_message_count: 100,
            addon_message_delay_secs: 1,
            mute_time_secs: 10,
        });

        session
            .handle_chat_message(
                chat_message_packet(ClientOpcodes::ChatMessageSay, "first"),
                ChatMsg::Say,
            )
            .await;
        session
            .handle_chat_message(
                chat_message_packet(ClientOpcodes::ChatMessageSay, "second"),
                ChatMsg::Say,
            )
            .await;
        session
            .handle_chat_message(
                chat_message_packet(ClientOpcodes::ChatMessageSay, "third"),
                ChatMsg::Say,
            )
            .await;

        assert_eq!(
            chat_text(&sender_rx.try_recv().expect("first sender echo")),
            "first"
        );
        assert_eq!(
            chat_text(&nearby_rx.try_recv().expect("first nearby")),
            "first"
        );
        assert_eq!(
            chat_text(&sender_rx.try_recv().expect("second sender echo")),
            "second"
        );
        assert_eq!(
            chat_text(&nearby_rx.try_recv().expect("second nearby")),
            "second"
        );
        let text = print_notification_text(&sender_rx.try_recv().expect("third mute notice"));
        assert!(text.starts_with("You must wait "));
        assert!(text.ends_with(" before speaking again."));
        assert!(sender_rx.try_recv().is_err());
        assert!(nearby_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn chat_flood_addon_mutes_after_limit_for_next_generic_addon_like_cpp() {
        let leader = ObjectGuid::create_player(1, 437);
        let member = ObjectGuid::create_player(1, 438);
        let (mut session, player_registry, leader_rx) = session_for_chat_routing_like_cpp(leader);
        let (member_tx, _member_rx) = flume::bounded(8);
        let (member_command_tx, member_command_rx) = flume::bounded(8);
        player_registry.insert(
            member,
            broadcast_info_with_command_tx(member, member_tx, member_command_tx),
        );
        let mut group = GroupInfo::new(leader);
        group.add_member(member);
        let group_guid = group.group_guid;
        let group_registry = Arc::new(wow_network::GroupRegistry::default());
        group_registry.insert(group_guid, group);
        session.group_guid = Some(group_guid);
        session.set_group_registry(group_registry, Arc::new(PendingInvites::default()));
        session.set_chat_flood_config_like_cpp(ChatFloodConfigLikeCpp {
            message_count: 10,
            message_delay_secs: 1,
            addon_message_count: 2,
            addon_message_delay_secs: 60,
            mute_time_secs: 10,
        });

        session
            .handle_chat_addon_message(chat_addon_packet(ChatMsg::Party, "ABC", "first"))
            .await;
        session
            .handle_chat_addon_message(chat_addon_packet(ChatMsg::Party, "ABC", "second"))
            .await;
        session
            .handle_chat_addon_message(chat_addon_packet(ChatMsg::Party, "ABC", "third"))
            .await;

        assert_eq!(
            chat_text(&expect_addon_command(&member_command_rx).packet_bytes),
            "first"
        );
        assert_eq!(
            chat_text(&expect_addon_command(&member_command_rx).packet_bytes),
            "second"
        );
        assert!(member_command_rx.try_recv().is_err());
        assert!(leader_rx.try_recv().is_err());
    }

    #[test]
    fn secs_to_full_time_string_matches_cpp_full_text_shape() {
        assert_eq!(secs_to_full_time_string_like_cpp(0), "0 Second.");
        assert_eq!(secs_to_full_time_string_like_cpp(1), "1 Second.");
        assert_eq!(secs_to_full_time_string_like_cpp(65), "1 Minute 5 Seconds.");
        assert_eq!(
            secs_to_full_time_string_like_cpp(90_061),
            "1 Day 1 Hour 1 Minute 1 Second."
        );
    }

    #[tokio::test]
    async fn gm_silence_aura_rejects_afk_toggle_like_cpp() {
        let sender = ObjectGuid::create_player(1, 363);
        let (mut session, _, sender_rx) = session_for_chat_routing_like_cpp(sender);
        session.visible_auras.insert(1, gm_silence_aura(1));

        session.handle_chat_afk(chat_away_packet("away")).await;

        assert!(session.auto_reply_msg_like_cpp().is_empty());
        assert_eq!(
            print_notification_text(&sender_rx.try_recv().expect("silence notification")),
            "Silence is ON for Player363"
        );
        assert!(sender_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn gm_silence_aura_rejects_dnd_toggle_like_cpp() {
        let sender = ObjectGuid::create_player(1, 430);
        let (mut session, _, sender_rx) = session_for_chat_routing_like_cpp(sender);
        session.visible_auras.insert(1, gm_silence_aura(1));

        session.handle_chat_dnd(chat_away_packet("busy")).await;

        assert!(session.auto_reply_msg_like_cpp().is_empty());
        assert_eq!(
            print_notification_text(&sender_rx.try_recv().expect("silence notification")),
            "Silence is ON for Player430"
        );
        assert!(sender_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn gm_silence_aura_rejects_chat_emote_with_notification_like_cpp() {
        let sender = ObjectGuid::create_player(1, 431);
        let nearby = ObjectGuid::create_player(1, 432);
        let (mut session, player_registry, sender_rx) = session_for_chat_routing_like_cpp(sender);
        let (nearby_tx, nearby_rx) = flume::bounded(8);
        player_registry.insert(nearby, broadcast_info(nearby, nearby_tx));
        session.visible_auras.insert(1, gm_silence_aura(1));

        session
            .handle_chat_emote(chat_emote_packet("muted emote"))
            .await;

        assert_eq!(
            print_notification_text(&sender_rx.try_recv().expect("silence notification")),
            "Silence is ON for Player431"
        );
        assert!(sender_rx.try_recv().is_err());
        assert!(nearby_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn gm_silence_aura_rejects_whisper_to_non_gm_like_cpp() {
        let sender = ObjectGuid::create_player(1, 364);
        let target = ObjectGuid::create_player(1, 365);
        let (mut session, player_registry, sender_rx) = session_for_chat_routing_like_cpp(sender);
        let (target_tx, target_rx) = flume::bounded(8);
        let mut target_info = broadcast_info(target, target_tx);
        target_info.player_name = "Target".to_string();
        target_info.is_game_master = false;
        player_registry.insert(target, target_info);
        session.visible_auras.insert(1, gm_silence_aura(1));

        session
            .handle_chat_whisper(chat_whisper_packet("Target", "muted"))
            .await;

        assert_eq!(
            print_notification_text(&sender_rx.try_recv().expect("silence notification")),
            "Silence is ON for Player364"
        );
        assert!(sender_rx.try_recv().is_err());
        assert!(target_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn gm_silence_aura_allows_whisper_to_gm_like_cpp() {
        let sender = ObjectGuid::create_player(1, 366);
        let target = ObjectGuid::create_player(1, 367);
        let (mut session, player_registry, sender_rx) = session_for_chat_routing_like_cpp(sender);
        let (target_tx, target_rx) = flume::bounded(8);
        let mut target_info = broadcast_info(target, target_tx);
        target_info.player_name = "Target".to_string();
        target_info.is_game_master = true;
        player_registry.insert(target, target_info);
        session.visible_auras.insert(1, gm_silence_aura(1));

        session
            .handle_chat_whisper(chat_whisper_packet("Target", "gm only"))
            .await;

        assert_eq!(
            chat_slash_cmd(&target_rx.try_recv().expect("target whisper")),
            ChatMsg::Whisper as u8
        );
        assert_eq!(
            chat_slash_cmd(&sender_rx.try_recv().expect("sender inform")),
            ChatMsg::WhisperInform as u8
        );
    }

    #[tokio::test]
    async fn whisper_to_afk_player_sends_auto_reply_to_sender_like_cpp() {
        let sender = ObjectGuid::create_player(1, 379);
        let target = ObjectGuid::create_player(1, 380);
        let (mut session, player_registry, sender_rx) = session_for_chat_routing_like_cpp(sender);
        let (target_tx, target_rx) = flume::bounded(8);
        let mut target_info = broadcast_info(target, target_tx);
        target_info.player_name = "Target".to_string();
        target_info.is_afk = true;
        target_info.auto_reply_msg_like_cpp = "back soon".to_string();
        player_registry.insert(target, target_info);

        session
            .handle_chat_whisper(chat_whisper_packet("Target", "hello"))
            .await;

        assert_eq!(
            chat_slash_cmd(&target_rx.try_recv().expect("target whisper")),
            ChatMsg::Whisper as u8
        );
        assert_eq!(
            chat_slash_cmd(&sender_rx.try_recv().expect("sender inform")),
            ChatMsg::WhisperInform as u8
        );
        let system = sender_rx.try_recv().expect("sender afk auto reply");
        assert_eq!(chat_slash_cmd(&system), ChatMsg::System as u8);
        assert_eq!(
            chat_text(&system),
            "Target is Away from Keyboard: back soon"
        );
    }

    #[tokio::test]
    async fn whisper_to_dnd_player_sends_auto_reply_to_sender_like_cpp() {
        let sender = ObjectGuid::create_player(1, 381);
        let target = ObjectGuid::create_player(1, 382);
        let (mut session, player_registry, sender_rx) = session_for_chat_routing_like_cpp(sender);
        let (target_tx, target_rx) = flume::bounded(8);
        let mut target_info = broadcast_info(target, target_tx);
        target_info.player_name = "Target".to_string();
        target_info.is_dnd = true;
        target_info.auto_reply_msg_like_cpp = "busy".to_string();
        player_registry.insert(target, target_info);

        session
            .handle_chat_whisper(chat_whisper_packet("Target", "hello"))
            .await;

        assert_eq!(
            chat_slash_cmd(&target_rx.try_recv().expect("target whisper")),
            ChatMsg::Whisper as u8
        );
        assert_eq!(
            chat_slash_cmd(&sender_rx.try_recv().expect("sender inform")),
            ChatMsg::WhisperInform as u8
        );
        let system = sender_rx.try_recv().expect("sender dnd auto reply");
        assert_eq!(chat_slash_cmd(&system), ChatMsg::System as u8);
        assert_eq!(
            chat_text(&system),
            "Target wishes to not be disturbed and cannot receive whisper messages: busy"
        );
    }

    #[tokio::test]
    async fn party_addon_routes_to_same_subgroup_except_sender_like_cpp() {
        let leader = ObjectGuid::create_player(1, 401);
        let same_subgroup = ObjectGuid::create_player(1, 402);
        let other_subgroup = ObjectGuid::create_player(1, 403);
        let (mut session, player_registry, _leader_rx) = session_for_chat_routing_like_cpp(leader);
        let (same_tx, _same_rx) = flume::bounded(8);
        let (other_tx, _other_rx) = flume::bounded(8);
        let (same_command_tx, same_command_rx) = flume::bounded(8);
        let (other_command_tx, other_command_rx) = flume::bounded(8);
        player_registry.insert(
            same_subgroup,
            broadcast_info_with_command_tx(same_subgroup, same_tx, same_command_tx),
        );
        player_registry.insert(
            other_subgroup,
            broadcast_info_with_command_tx(other_subgroup, other_tx, other_command_tx),
        );

        let mut group = GroupInfo::new(leader);
        group.convert_to_raid_like_cpp();
        group.add_member(same_subgroup);
        group.add_member(other_subgroup);
        assert!(group.change_member_group_like_cpp(other_subgroup, 1));
        let group_guid = group.group_guid;
        let group_registry = Arc::new(wow_network::GroupRegistry::default());
        group_registry.insert(group_guid, group);
        session.group_guid = Some(group_guid);
        session.set_group_registry(group_registry, Arc::new(PendingInvites::default()));

        session
            .handle_chat_addon_message(chat_addon_packet(ChatMsg::Party, "ABC", "payload"))
            .await;

        let command = expect_addon_command(&same_command_rx);
        assert_eq!(command.prefix, "ABC");
        assert_eq!(chat_slash_cmd(&command.packet_bytes), ChatMsg::Party as u8);
        assert_eq!(chat_language(&command.packet_bytes), LANG_ADDON_LIKE_CPP);
        assert!(other_command_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn addon_channel_config_blocks_addon_delivery_like_cpp() {
        let leader = ObjectGuid::create_player(1, 411);
        let member = ObjectGuid::create_player(1, 412);
        let (mut session, player_registry, _leader_rx) = session_for_chat_routing_like_cpp(leader);
        let (member_tx, _member_rx) = flume::bounded(8);
        let (member_command_tx, member_command_rx) = flume::bounded(8);
        player_registry.insert(
            member,
            broadcast_info_with_command_tx(member, member_tx, member_command_tx),
        );

        let mut group = GroupInfo::new(leader);
        group.add_member(member);
        let group_guid = group.group_guid;
        let group_registry = Arc::new(wow_network::GroupRegistry::default());
        group_registry.insert(group_guid, group);
        session.group_guid = Some(group_guid);
        session.set_group_registry(group_registry, Arc::new(PendingInvites::default()));
        session.set_addon_channel_like_cpp(false);

        session
            .handle_chat_addon_message(chat_addon_packet(ChatMsg::Party, "ABC", "payload"))
            .await;

        assert!(member_command_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn addon_whisper_routes_to_named_registered_target_like_cpp() {
        let sender = ObjectGuid::create_player(1, 421);
        let target = ObjectGuid::create_player(1, 422);
        let (mut session, player_registry, _sender_rx) = session_for_chat_routing_like_cpp(sender);
        let (target_tx, _target_rx) = flume::bounded(8);
        let (target_command_tx, target_command_rx) = flume::bounded(8);
        let mut target_info = broadcast_info_with_command_tx(target, target_tx, target_command_tx);
        target_info.player_name = "Target".to_string();
        player_registry.insert(target, target_info);

        session
            .handle_chat_addon_message_whisper(chat_addon_whisper_packet(
                "Target", "ABC", "payload",
            ))
            .await;

        let command = expect_addon_command(&target_command_rx);
        assert_eq!(command.prefix, "ABC");
        assert_eq!(
            chat_slash_cmd(&command.packet_bytes),
            ChatMsg::Whisper as u8
        );
        assert_eq!(chat_language(&command.packet_bytes), LANG_ADDON_LIKE_CPP);
        assert_eq!(chat_text(&command.packet_bytes), "payload");
    }

    #[tokio::test]
    async fn targeted_addon_whisper_routes_logged_payload_like_cpp() {
        let sender = ObjectGuid::create_player(1, 425);
        let target = ObjectGuid::create_player(1, 426);
        let (mut session, player_registry, _sender_rx) = session_for_chat_routing_like_cpp(sender);
        let (target_tx, _target_rx) = flume::bounded(8);
        let (target_command_tx, target_command_rx) = flume::bounded(8);
        let mut target_info = broadcast_info_with_command_tx(target, target_tx, target_command_tx);
        target_info.player_name = "Target".to_string();
        player_registry.insert(target, target_info);

        session
            .handle_chat_addon_message_targeted(chat_addon_targeted_packet(
                ChatMsg::Whisper,
                "ABC",
                "payload",
                "Target",
                ObjectGuid::EMPTY,
                true,
            ))
            .await;

        let command = expect_addon_command(&target_command_rx);
        assert_eq!(command.prefix, "ABC");
        assert_eq!(
            chat_slash_cmd(&command.packet_bytes),
            ChatMsg::Whisper as u8
        );
        assert_eq!(
            chat_language(&command.packet_bytes),
            LANG_ADDON_LOGGED_LIKE_CPP
        );
        assert_eq!(chat_text(&command.packet_bytes), "payload");
    }

    #[tokio::test]
    async fn targeted_party_addon_uses_group_routing_like_cpp() {
        let leader = ObjectGuid::create_player(1, 427);
        let member = ObjectGuid::create_player(1, 428);
        let (mut session, player_registry, _leader_rx) = session_for_chat_routing_like_cpp(leader);
        let (member_tx, _member_rx) = flume::bounded(8);
        let (member_command_tx, member_command_rx) = flume::bounded(8);
        player_registry.insert(
            member,
            broadcast_info_with_command_tx(member, member_tx, member_command_tx),
        );

        let mut group = GroupInfo::new(leader);
        group.add_member(member);
        let group_guid = group.group_guid;
        let group_registry = Arc::new(wow_network::GroupRegistry::default());
        group_registry.insert(group_guid, group);
        session.group_guid = Some(group_guid);
        session.set_group_registry(group_registry, Arc::new(PendingInvites::default()));

        session
            .handle_chat_addon_message_targeted(chat_addon_targeted_packet(
                ChatMsg::Party,
                "ABC",
                "payload",
                "IgnoredTarget",
                ObjectGuid::EMPTY,
                false,
            ))
            .await;

        let command = expect_addon_command(&member_command_rx);
        assert_eq!(command.prefix, "ABC");
        assert_eq!(chat_slash_cmd(&command.packet_bytes), ChatMsg::Party as u8);
        assert_eq!(chat_language(&command.packet_bytes), LANG_ADDON_LIKE_CPP);
    }

    #[tokio::test]
    async fn addon_whisper_missing_target_sends_notfound_like_cpp() {
        let sender = ObjectGuid::create_player(1, 431);
        let (mut session, _player_registry, sender_rx) = session_for_chat_routing_like_cpp(sender);

        session
            .handle_chat_addon_message_whisper(chat_addon_whisper_packet(
                "Missing", "ABC", "payload",
            ))
            .await;

        let notfound = sender_rx.try_recv().expect("notfound packet");
        let mut packet = wow_packet::WorldPacket::from_bytes(&notfound);
        assert_eq!(
            packet.read_uint16().expect("opcode"),
            wow_constants::ServerOpcodes::ChatPlayerNotfound as u16
        );
    }

    #[tokio::test]
    async fn addon_whisper_level_requirement_blocks_delivery_like_cpp() {
        let sender = ObjectGuid::create_player(1, 441);
        let target = ObjectGuid::create_player(1, 442);
        let (mut session, player_registry, _sender_rx) = session_for_chat_routing_like_cpp(sender);
        let (target_tx, _target_rx) = flume::bounded(8);
        let (target_command_tx, target_command_rx) = flume::bounded(8);
        let mut target_info = broadcast_info_with_command_tx(target, target_tx, target_command_tx);
        target_info.player_name = "Target".to_string();
        player_registry.insert(target, target_info);
        session.set_player_level_like_cpp(1);
        session.set_chat_level_requirements_like_cpp(ChatLevelRequirementsLikeCpp {
            whisper: 2,
            ..ChatLevelRequirementsLikeCpp::default()
        });

        session
            .handle_chat_addon_message_whisper(chat_addon_whisper_packet(
                "Target", "ABC", "payload",
            ))
            .await;

        assert!(target_command_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn addon_command_delivers_only_when_prefix_registered_like_cpp() {
        let receiver = ObjectGuid::create_player(1, 501);
        let (mut session, _, send_rx) = session_for_chat_routing_like_cpp(receiver);
        session.set_state(crate::session::SessionState::LoggedIn);
        session.filter_addon_messages = true;
        session.registered_addon_prefixes = vec!["ABC".to_string()];
        let packet = ChatPkt {
            msg_type: ChatMsg::Raid,
            language: LANG_ADDON_LIKE_CPP,
            sender_guid: ObjectGuid::create_player(1, 502),
            sender_name: "Sender".to_string(),
            target_guid: ObjectGuid::EMPTY,
            target_name: String::new(),
            prefix: "ABC".to_string(),
            channel: String::new(),
            text: "payload".to_string(),
            virtual_realm: 0,
        };

        session
            .session_command_tx()
            .try_send(SessionCommand::SendAddonIfRegisteredLikeCpp(
                SendAddonIfRegisteredLikeCppCommand {
                    prefix: "XYZ".to_string(),
                    packet_bytes: packet.to_bytes(),
                },
            ))
            .expect("command queued");
        session
            .process_represented_session_commands_like_cpp()
            .await;
        assert!(send_rx.try_recv().is_err());

        let packet = ChatPkt {
            prefix: "ABC".to_string(),
            ..packet
        };
        session
            .session_command_tx()
            .try_send(SessionCommand::SendAddonIfRegisteredLikeCpp(
                SendAddonIfRegisteredLikeCppCommand {
                    prefix: "ABC".to_string(),
                    packet_bytes: packet.to_bytes(),
                },
            ))
            .expect("command queued");
        session
            .process_represented_session_commands_like_cpp()
            .await;
        let delivered = send_rx.try_recv().expect("registered prefix delivered");
        assert_eq!(chat_slash_cmd(&delivered), ChatMsg::Raid as u8);
        assert_eq!(chat_language(&delivered), LANG_ADDON_LIKE_CPP);
    }
}
