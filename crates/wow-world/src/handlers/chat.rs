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
use wow_core::ObjectGuid;
use wow_core::guid::HighGuid;
use wow_handler::{PacketHandlerEntry, PacketProcessing, SessionStatus};
use wow_network::GroupInfo;
use wow_packet::packets::chat::{
    CTextEmote, ChatAddonMessage, ChatMessage, ChatMessageAfk, ChatMessageDnd, ChatMessageEmote,
    ChatMessageWhisper, ChatMsg, ChatPkt, ChatRegisterAddonPrefixes, ChatReportIgnored,
    EmoteClient, EmoteMessage, STextEmote,
};
use wow_packet::{ClientPacket, ServerPacket};

use crate::session::{PlayerAwayModeLikeCpp, WorldSession};

// ── Broadcast range constants (C# WorldCfg defaults) ─────────────
const RANGE_SAY: f32 = 25.0;
const RANGE_YELL: f32 = 300.0;
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
        opcode: ClientOpcodes::ChatReportIgnored,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_chat_report_ignored",
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

        if msg_type == ChatMsg::Guild {
            debug!(
                account = self.account_id,
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
                language: msg.language as u32,
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
                language: msg.language as u32,
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
                language: msg.language as u32,
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

    /// Handle CMSG_CHAT_MESSAGE_AFK.
    pub async fn handle_chat_afk(&mut self, mut pkt: wow_packet::WorldPacket) {
        let msg = match ChatMessageAfk::read(&mut pkt) {
            Ok(m) => m,
            Err(e) => {
                tracing::warn!(account = self.account_id, "Bad AFK chat packet: {e}");
                return;
            }
        };

        let _ = self.apply_chat_away_mode_like_cpp(PlayerAwayModeLikeCpp::Afk, msg.text);
    }

    /// Handle CMSG_CHAT_MESSAGE_DND.
    pub async fn handle_chat_dnd(&mut self, mut pkt: wow_packet::WorldPacket) {
        let msg = match ChatMessageDnd::read(&mut pkt) {
            Ok(m) => m,
            Err(e) => {
                tracing::warn!(account = self.account_id, "Bad DND chat packet: {e}");
                return;
            }
        };

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
                channel: String::new(),
                text: reporter_name,
                virtual_realm,
            };
            let _ = tx.send(ignored.to_bytes());
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

        if packet.prefix.is_empty() || packet.prefix.len() > 16 || packet.text.len() > 255 {
            return;
        }

        debug!(
            account = self.account_id,
            ty = packet.msg_type,
            prefix = %packet.prefix,
            logged = packet.is_logged,
            "Addon chat message ignored until addon routing is ported"
        );
    }

    // ── Helpers ──────────────────────────────────────────────────

    fn player_name_and_guid(&self) -> (wow_core::ObjectGuid, String) {
        let guid = self.player_guid().unwrap_or(wow_core::ObjectGuid::EMPTY);
        let name = self.player_name_like_cpp().unwrap_or_default().to_string();
        (guid, name)
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
                if !group.is_raid_group()
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
            channel: String::new(),
            text,
            virtual_realm,
        };
        self.broadcast_group_chat_packet_like_cpp(&group, subgroup_filter, chat.to_bytes());
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{HashMap, HashSet};
    use std::sync::Arc;
    use wow_network::{PendingInvites, PlayerBroadcastInfo, PlayerRegistry};

    fn chat_message_packet(opcode: ClientOpcodes, text: &str) -> wow_packet::WorldPacket {
        let mut writer = wow_packet::WorldPacket::new_empty();
        writer.write_uint16(opcode as u16);
        writer.write_int32(0);
        writer.write_bits(text.len() as u32, 11);
        writer.write_bit(false);
        writer.write_string(text);

        let mut reader = wow_packet::WorldPacket::from_bytes(writer.data());
        reader.skip_opcode();
        reader
    }

    fn chat_slash_cmd(bytes: &[u8]) -> u8 {
        let mut packet = wow_packet::WorldPacket::from_bytes(bytes);
        packet.skip_opcode();
        packet.read_uint8().expect("chat slash command")
    }

    fn broadcast_info(guid: ObjectGuid, send_tx: flume::Sender<Vec<u8>>) -> PlayerBroadcastInfo {
        let (command_tx, _command_rx) = flume::bounded(8);
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
            in_vehicle: false,
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
    }
}
