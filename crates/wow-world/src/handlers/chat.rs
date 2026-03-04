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

use wow_constants::ClientOpcodes;
use wow_core::guid::HighGuid;
use wow_core::ObjectGuid;
use wow_handler::{PacketHandlerEntry, PacketProcessing, SessionStatus};
use wow_packet::packets::chat::{
    ChatMessage, ChatMessageEmote, ChatMessageWhisper, ChatMsg, ChatPkt,
    CTextEmote, EmoteClient, EmoteMessage, STextEmote,
};
use wow_packet::{ClientPacket, ServerPacket};

use crate::session::WorldSession;

// ── Broadcast range constants (C# WorldCfg defaults) ─────────────
const RANGE_SAY:   f32 = 25.0;
const RANGE_YELL:  f32 = 300.0;
const RANGE_EMOTE: f32 = 25.0;

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

// ── Handler implementations ───────────────────────────────────────

impl WorldSession {
    /// Handle say/yell/party/guild/raid/instance chat messages.
    pub async fn handle_chat_message(
        &mut self,
        mut pkt: wow_packet::WorldPacket,
        msg_type: ChatMsg,
    ) {
        let msg = match ChatMessage::read(&mut pkt) {
            Ok(m) => m,
            Err(e) => {
                tracing::warn!(account = self.account_id, "Bad chat packet: {e}");
                return;
            }
        };

        debug!(
            account = self.account_id,
            ty = ?msg_type,
            text = %msg.text,
            "Chat message"
        );

        let (sender_guid, sender_name) = self.player_name_and_guid();
        let virtual_realm = self.virtual_realm_address();

        let chat = ChatPkt {
            msg_type,
            language: msg.language,
            sender_guid,
            sender_name,
            target_guid: wow_core::ObjectGuid::EMPTY,
            target_name: String::new(),
            channel: String::new(),
            text: msg.text,
            virtual_realm,
        };

        // Echo back to the sender (they need to see their own message).
        self.send_packet(&chat);

        // Broadcast to nearby players on the same map.
        let range = if msg_type == ChatMsg::Yell { RANGE_YELL } else { RANGE_SAY };
        self.broadcast_chat_packet(&chat, range);
    }

    /// Handle whisper messages.
    pub async fn handle_chat_whisper(&mut self, mut pkt: wow_packet::WorldPacket) {
        let msg = match ChatMessageWhisper::read(&mut pkt) {
            Ok(m) => m,
            Err(e) => {
                tracing::warn!(account = self.account_id, "Bad whisper packet: {e}");
                return;
            }
        };

        debug!(
            account = self.account_id,
            target = %msg.target,
            text = %msg.text,
            "Whisper"
        );

        let (sender_guid, sender_name) = self.player_name_and_guid();
        let virtual_realm = self.virtual_realm_address();
        let target_name = msg.target.clone();

        // Try to deliver to the target player via the registry.
        let target_tx = self.player_registry().and_then(|reg| {
            reg.iter()
                .find(|e| e.value().player_name.eq_ignore_ascii_case(&target_name))
                .map(|e| e.value().send_tx.clone())
        });

        if let Some(tx) = target_tx {
            // Forward the whisper to the target as a normal Say-whisper.
            let to_target = ChatPkt {
                msg_type: ChatMsg::Whisper,
                language: msg.language,
                sender_guid,
                sender_name: sender_name.clone(),
                target_guid: ObjectGuid::EMPTY,
                target_name: target_name.clone(),
                channel: String::new(),
                text: msg.text.clone(),
                virtual_realm,
            };
            let _ = tx.send(to_target.to_bytes());

            // Inform sender their whisper was delivered.
            let inform = ChatPkt {
                msg_type: ChatMsg::WhisperInform,
                language: msg.language,
                sender_guid,
                sender_name: sender_name.clone(),
                target_guid: ObjectGuid::EMPTY,
                target_name: target_name.clone(),
                channel: String::new(),
                text: msg.text,
                virtual_realm,
            };
            self.send_packet(&inform);
        } else {
            // Target not found online — echo back as inform (vanilla behavior).
            let chat = ChatPkt {
                msg_type: ChatMsg::WhisperInform,
                language: msg.language,
                sender_guid,
                sender_name,
                target_guid: ObjectGuid::EMPTY,
                target_name,
                channel: String::new(),
                text: msg.text,
                virtual_realm,
            };
            self.send_packet(&chat);
        }
    }

    /// Handle emote text (/e).
    pub async fn handle_chat_emote(&mut self, mut pkt: wow_packet::WorldPacket) {
        let msg = match ChatMessageEmote::read(&mut pkt) {
            Ok(m) => m,
            Err(e) => {
                tracing::warn!(account = self.account_id, "Bad emote packet: {e}");
                return;
            }
        };

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

        debug!(
            account = self.account_id,
            emote_id = msg.emote_id,
            sound_index = msg.sound_index,
            "CMSG_SEND_TEXT_EMOTE"
        );

        let (player_guid, _name) = self.player_name_and_guid();
        let account_guid = ObjectGuid::create_global(
            HighGuid::WowAccount,
            0,
            self.account_id as i64,
        );

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

    // ── Helpers ──────────────────────────────────────────────────

    fn player_name_and_guid(&self) -> (wow_core::ObjectGuid, String) {
        let guid = self.player_guid.unwrap_or(wow_core::ObjectGuid::EMPTY);
        let name = self.player_name.clone().unwrap_or_default();
        (guid, name)
    }

    /// Serialize `pkt` and broadcast its bytes to all players on the same map
    /// within `range` yards (excluding the sender).
    fn broadcast_chat_packet(&self, pkt: &ChatPkt, range: f32) {
        self.broadcast_raw_packet(pkt.to_bytes(), range);
    }

    /// Send pre-serialised packet `bytes` to all players on the same map
    /// within `range` yards, excluding this session's player.
    fn broadcast_raw_packet(&self, bytes: Vec<u8>, range: f32) {
        let registry = match self.player_registry() {
            Some(r) => r,
            None => return,
        };

        let sender_guid = self.player_guid.unwrap_or(ObjectGuid::EMPTY);
        let sender_pos  = self.player_position;
        let sender_map  = self.current_map_id;
        let range_sq    = range * range;

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
