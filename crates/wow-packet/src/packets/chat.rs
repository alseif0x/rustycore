// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Chat packet definitions (CMSG_CHAT_MESSAGE_* / SMSG_CHAT).

use wow_constants::{ClientOpcodes, ServerOpcodes};
use wow_core::ObjectGuid;

use crate::world_packet::{PacketError, WorldPacket};
use crate::{ClientPacket, ServerPacket};

pub const CHAT_INVALID_NAME_NOTICE_LIKE_CPP: u8 = 0x1B;
pub const MAX_CHANNEL_NAME_STR_LIKE_CPP: usize = 31;
pub const MAX_CHANNEL_PASS_STR_LIKE_CPP: usize = 127;

// ── CMSG_CHAT_JOIN_CHANNEL ────────────────────────────────────────

/// C++ `WorldPackets::Channel::JoinChannel`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JoinChannel {
    pub chat_channel_id: i32,
    pub create_voice_session: bool,
    pub internal: bool,
    pub channel_name: String,
    pub password: String,
}

impl ClientPacket for JoinChannel {
    const OPCODE: ClientOpcodes = ClientOpcodes::ChatJoinChannel;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        let chat_channel_id = pkt.read_int32()?;
        let create_voice_session = pkt.read_bit()?;
        let internal = pkt.read_bit()?;
        let channel_len = pkt.read_bits(7)? as usize;
        let password_len = pkt.read_bits(7)? as usize;
        let channel_name = pkt.read_string(channel_len)?;
        let password = pkt.read_string(password_len)?;

        Ok(Self {
            chat_channel_id,
            create_voice_session,
            internal,
            channel_name,
            password,
        })
    }
}

/// C++ `WorldPackets::Channel::LeaveChannel`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LeaveChannel {
    pub zone_channel_id: i32,
    pub channel_name: String,
}

impl ClientPacket for LeaveChannel {
    const OPCODE: ClientOpcodes = ClientOpcodes::ChatLeaveChannel;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        let zone_channel_id = pkt.read_int32()?;
        let channel_len = pkt.read_bits(7)? as usize;
        let channel_name = pkt.read_string(channel_len)?;

        Ok(Self {
            zone_channel_id,
            channel_name,
        })
    }
}

/// C++ `WorldPackets::Channel::ChannelCommand`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChannelCommand {
    pub channel_name: String,
}

impl ChannelCommand {
    pub fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        let channel_len = pkt.read_bits(7)? as usize;
        let channel_name = pkt.read_string(channel_len)?;
        Ok(Self { channel_name })
    }
}

/// C++ `WorldPackets::Channel::ChannelPlayerCommand`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChannelPlayerCommand {
    pub channel_name: String,
    pub name: String,
}

impl ChannelPlayerCommand {
    pub fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        let channel_len = pkt.read_bits(7)? as usize;
        let name_len = pkt.read_bits(9)? as usize;
        let channel_name = pkt.read_string(channel_len)?;
        let name = pkt.read_string(name_len)?;
        Ok(Self { channel_name, name })
    }
}

/// C++ `WorldPackets::Channel::ChannelPassword`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChannelPassword {
    pub channel_name: String,
    pub password: String,
}

impl ClientPacket for ChannelPassword {
    const OPCODE: ClientOpcodes = ClientOpcodes::ChatChannelPassword;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        let channel_len = pkt.read_bits(7)? as usize;
        let password_len = pkt.read_bits(7)? as usize;
        let channel_name = pkt.read_string(channel_len)?;
        let password = pkt.read_string(password_len)?;
        Ok(Self {
            channel_name,
            password,
        })
    }
}

// ── SMSG_CHANNEL_NOTIFY ───────────────────────────────────────────

/// C++ `WorldPackets::Channel::ChannelNotify`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChannelNotify {
    pub notify_type: u8,
    pub channel: String,
    pub sender: String,
    pub sender_guid: ObjectGuid,
    pub sender_account_id: ObjectGuid,
    pub sender_virtual_realm: u32,
    pub target_guid: ObjectGuid,
    pub target_virtual_realm: u32,
    pub chat_channel_id: i32,
    pub old_flags: u8,
    pub new_flags: u8,
}

impl ChannelNotify {
    pub fn invalid_name(channel: impl Into<String>) -> Self {
        Self {
            notify_type: CHAT_INVALID_NAME_NOTICE_LIKE_CPP,
            channel: channel.into(),
            sender: String::new(),
            sender_guid: ObjectGuid::EMPTY,
            sender_account_id: ObjectGuid::EMPTY,
            sender_virtual_realm: 0,
            target_guid: ObjectGuid::EMPTY,
            target_virtual_realm: 0,
            chat_channel_id: 0,
            old_flags: 0,
            new_flags: 0,
        }
    }
}

impl ServerPacket for ChannelNotify {
    const OPCODE: ServerOpcodes = ServerOpcodes::ChannelNotify;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_bits(u32::from(self.notify_type), 6);
        pkt.write_bits(self.channel.len() as u32, 7);
        pkt.write_bits(self.sender.len() as u32, 6);
        pkt.flush_bits();

        pkt.write_packed_guid(&self.sender_guid);
        pkt.write_packed_guid(&self.sender_account_id);
        pkt.write_uint32(self.sender_virtual_realm);
        pkt.write_packed_guid(&self.target_guid);
        pkt.write_uint32(self.target_virtual_realm);
        pkt.write_int32(self.chat_channel_id);

        if self.notify_type == 0x10 {
            pkt.write_uint8(self.old_flags);
            pkt.write_uint8(self.new_flags);
        }

        pkt.write_string(&self.channel);
        pkt.write_string(&self.sender);
    }
}

// ── Chat message types ────────────────────────────────────────────

/// Chat message type (`ChatMsg` in TrinityCore WotLK Classic).
///
/// C++ anchor:
/// `/home/server/woltk-trinity-legacy/src/server/game/Miscellaneous/SharedDefines.h:5877-5949`.
/// `CHAT_MSG_ADDON = -1` is intentionally omitted because this wire enum is
/// serialized as `uint8` for `SMSG_CHAT` (`ChatPackets.cpp:191`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ChatMsg {
    System = 0x00,
    Say = 0x01,
    Party = 0x02,
    Raid = 0x03,
    Guild = 0x04,
    Officer = 0x05,
    Yell = 0x06,
    Whisper = 0x07,
    WhisperForeign = 0x08,
    WhisperInform = 0x09,
    Emote = 0x0A,
    TextEmote = 0x0B,
    MonsterSay = 0x0C,
    MonsterParty = 0x0D,
    MonsterYell = 0x0E,
    MonsterWhisper = 0x0F,
    MonsterEmote = 0x10,
    Channel = 0x11,
    ChannelJoin = 0x12,
    ChannelLeave = 0x13,
    ChannelList = 0x14,
    ChannelNotice = 0x15,
    ChannelNoticeUser = 0x16,
    Afk = 0x17,
    Dnd = 0x18,
    Ignored = 0x19,
    Skill = 0x1A,
    Loot = 0x1B,
    Money = 0x1C,
    Opening = 0x1D,
    Tradeskills = 0x1E,
    PetInfo = 0x1F,
    CombatMiscInfo = 0x20,
    CombatXpGain = 0x21,
    CombatHonorGain = 0x22,
    CombatFactionChange = 0x23,
    BgSystemNeutral = 0x24,
    BgSystemAlliance = 0x25,
    BgSystemHorde = 0x26,
    RaidLeader = 0x27,
    RaidWarning = 0x28,
    RaidBossEmote = 0x29,
    RaidBossWhisper = 0x2A,
    Filtered = 0x2B,
    Restricted = 0x2C,
    BattleNet = 0x2D,
    Achievement = 0x2E,
    GuildAchievement = 0x2F,
    ArenaPoints = 0x30,
    PartyLeader = 0x31,
    Targeticons = 0x32,
    BnWhisper = 0x33,
    BnWhisperInform = 0x34,
    BnInlineToastAlert = 0x35,
    BnInlineToastBroadcast = 0x36,
    BnInlineToastBroadcastInform = 0x37,
    BnInlineToastConversation = 0x38,
    BnWhisperPlayerOffline = 0x39,
    Currency = 0x3A,
    QuestBossEmote = 0x3B,
    PetBattleCombatLog = 0x3C,
    PetBattleInfo = 0x3D,
    InstanceChat = 0x3E,
    InstanceChatLeader = 0x3F,
    GuildItemLooted = 0x40,
    CommunitiesChannel = 0x41,
    VoiceText = 0x42,
}

// ── CMSG_CHAT_MESSAGE_SAY / PARTY / YELL / GUILD / INSTANCE_CHAT ─

/// Generic chat message from client (say, party, raid, yell, instance).
#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub language: i32,
    pub text: String,
    pub is_secure: bool,
}

impl ClientPacket for ChatMessage {
    const OPCODE: ClientOpcodes = ClientOpcodes::ChatMessageSay;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        let language = pkt.read_int32()?;
        let len = pkt.read_bits(11)? as usize;
        let opcode = pkt.client_opcode().unwrap_or(ClientOpcodes::ChatMessageSay);
        let is_secure = if matches!(
            opcode,
            ClientOpcodes::ChatMessageSay
                | ClientOpcodes::ChatMessageParty
                | ClientOpcodes::ChatMessageRaid
                | ClientOpcodes::ChatMessageRaidWarning
                | ClientOpcodes::ChatMessageInstanceChat
        ) {
            pkt.has_bit()?
        } else {
            false
        };
        // bit reader auto-resets on next byte read
        let text = pkt.read_string(len)?;
        Ok(Self {
            language,
            text,
            is_secure,
        })
    }
}

// ── CMSG_CHAT_MESSAGE_WHISPER ─────────────────────────────────────

#[derive(Debug, Clone)]
pub struct ChatMessageWhisper {
    pub language: i32,
    pub target: String,
    pub text: String,
}

impl ClientPacket for ChatMessageWhisper {
    const OPCODE: ClientOpcodes = ClientOpcodes::ChatMessageWhisper;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        let language = pkt.read_int32()?;
        let target_len = pkt.read_bits(9)? as usize;
        let text_len = pkt.read_bits(11)? as usize;
        // bit reader auto-resets on next byte read
        let target = pkt.read_string(target_len)?;
        let text = pkt.read_string(text_len)?;
        Ok(Self {
            language,
            target,
            text,
        })
    }
}

// ── CMSG_CHAT_MESSAGE_AFK / DND ──────────────────────────────────

/// C++ `WorldPackets::Chat::ChatMessageAFK::Read`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatMessageAfk {
    pub text: String,
}

impl ClientPacket for ChatMessageAfk {
    const OPCODE: ClientOpcodes = ClientOpcodes::ChatMessageAfk;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        let len = pkt.read_bits(11)? as usize;
        let text = pkt.read_string(len)?;
        Ok(Self { text })
    }
}

/// C++ `WorldPackets::Chat::ChatMessageDND::Read`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatMessageDnd {
    pub text: String,
}

impl ClientPacket for ChatMessageDnd {
    const OPCODE: ClientOpcodes = ClientOpcodes::ChatMessageDnd;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        let len = pkt.read_bits(11)? as usize;
        let text = pkt.read_string(len)?;
        Ok(Self { text })
    }
}

// ── CMSG_CHAT_REPORT_IGNORED ─────────────────────────────────────

/// C++ `WorldPackets::Chat::ChatReportIgnored::Read`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatReportIgnored {
    pub ignored_guid: ObjectGuid,
    pub reason: u8,
}

impl ClientPacket for ChatReportIgnored {
    const OPCODE: ClientOpcodes = ClientOpcodes::ChatReportIgnored;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        let ignored_guid = pkt.read_packed_guid()?;
        let reason = pkt.read_uint8()?;
        Ok(Self {
            ignored_guid,
            reason,
        })
    }
}

// ── CMSG_CHAT_MESSAGE_EMOTE ───────────────────────────────────────

#[derive(Debug, Clone)]
pub struct ChatMessageEmote {
    pub text: String,
}

impl ClientPacket for ChatMessageEmote {
    const OPCODE: ClientOpcodes = ClientOpcodes::ChatMessageEmote;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        let len = pkt.read_bits(11)? as usize;
        // bit reader auto-resets on next byte read
        let text = pkt.read_string(len)?;
        Ok(Self { text })
    }
}

// ── CMSG_CHAT_REGISTER_ADDON_PREFIXES / CMSG_CHAT_ADDON_MESSAGE ──────────────

/// CMSG_CHAT_REGISTER_ADDON_PREFIXES.
///
/// C++ ref: `WorldPackets::Chat::ChatRegisterAddonPrefixes::Read`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatRegisterAddonPrefixes {
    pub prefixes: Vec<String>,
}

impl ChatRegisterAddonPrefixes {
    pub const MAX_PREFIXES: usize = 64;
}

impl ClientPacket for ChatRegisterAddonPrefixes {
    const OPCODE: ClientOpcodes = ClientOpcodes::ChatRegisterAddonPrefixes;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        let count = pkt.read_uint32()? as usize;
        let mut prefixes = Vec::with_capacity(count);
        for _ in 0..count {
            let len = pkt.read_bits(5)? as usize;
            prefixes.push(pkt.read_string(len)?);
        }
        Ok(Self { prefixes })
    }
}

/// CMSG_CHAT_ADDON_MESSAGE payload.
///
/// C++ ref: `operator>>(ByteBuffer&, ChatAddonMessageParams&)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatAddonMessage {
    pub msg_type: i32,
    pub prefix: String,
    pub text: String,
    pub is_logged: bool,
}

impl ClientPacket for ChatAddonMessage {
    const OPCODE: ClientOpcodes = ClientOpcodes::ChatAddonMessage;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        let prefix_len = pkt.read_bits(5)? as usize;
        let text_len = pkt.read_bits(8)? as usize;
        let is_logged = pkt.read_bit()?;
        let msg_type = pkt.read_int32()?;
        let prefix = pkt.read_string(prefix_len)?;
        let text = pkt.read_string(text_len)?;
        Ok(Self {
            msg_type,
            prefix,
            text,
            is_logged,
        })
    }
}

/// CMSG_CHAT_ADDON_MESSAGE_WHISPER payload.
///
/// C++ ref: `WorldPackets::Chat::ChatAddonMessageWhisper::Read`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatAddonMessageWhisper {
    pub target: String,
    pub prefix: String,
    pub message: String,
}

impl ClientPacket for ChatAddonMessageWhisper {
    const OPCODE: ClientOpcodes = ClientOpcodes::ChatAddonMessageWhisper;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        let target_len = pkt.read_bits(9)? as usize;
        let prefix_len = pkt.read_bits(5)? as usize;
        let message_len = pkt.read_bits(8)? as usize;
        let target = pkt.read_string(target_len)?;
        let prefix = pkt.read_string(prefix_len)?;
        let message = pkt.read_string(message_len)?;
        Ok(Self {
            target,
            prefix,
            message,
        })
    }
}

// ── SMSG_CHAT ─────────────────────────────────────────────────────

/// Server sends this to broadcast a chat message to nearby players.
///
/// C# ChatPkt.Write():
/// ```text
/// u8   slash_cmd (ChatMsg)
/// u32  language
/// PackedGuid sender_guid
/// PackedGuid sender_guild_guid
/// PackedGuid sender_account_guid
/// PackedGuid target_guid
/// u32  target_virtual_address
/// u32  sender_virtual_address
/// i32  achievement_id
/// f32  display_time
/// i32  spell_id
/// bits(11) sender_name_len
/// bits(11) target_name_len
/// bits(5)  prefix_len
/// bits(7)  channel_len
/// bits(12) chat_text_len
/// bits(15) chat_flags
/// bit  hide_chat_log
/// bit  fake_sender_name
/// bit  has_unused_801
/// bit  has_channel_guid
/// [flush]
/// string sender_name
/// string target_name
/// string prefix
/// string channel
/// string chat_text
/// [optional: u32 unused_801]
/// [optional: PackedGuid channel_guid]
/// ```
#[derive(Debug, Clone)]
pub struct ChatPkt {
    pub msg_type: ChatMsg,
    pub language: u32,
    pub sender_guid: ObjectGuid,
    pub sender_name: String,
    pub target_guid: ObjectGuid,
    pub target_name: String,
    pub prefix: String,
    pub channel: String,
    pub text: String,
    pub virtual_realm: u32,
}

impl ServerPacket for ChatPkt {
    const OPCODE: ServerOpcodes = ServerOpcodes::Chat;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_uint8(self.msg_type as u8);
        pkt.write_uint32(self.language);
        pkt.write_packed_guid(&self.sender_guid);
        pkt.write_packed_guid(&ObjectGuid::EMPTY); // sender_guild_guid
        pkt.write_packed_guid(&ObjectGuid::EMPTY); // sender_account_guid
        pkt.write_packed_guid(&self.target_guid);
        pkt.write_uint32(self.virtual_realm); // target_virtual_address
        pkt.write_uint32(self.virtual_realm); // sender_virtual_address
        pkt.write_int32(0i32); // achievement_id
        pkt.write_float(0.0f32); // display_time
        pkt.write_int32(0i32); // spell_id

        let sender_bytes = self.sender_name.len() as u32;
        let target_bytes = self.target_name.len() as u32;
        let prefix_bytes = self.prefix.len() as u32;
        let channel_bytes = self.channel.len() as u32;
        let text_bytes = self.text.len() as u32;

        pkt.write_bits(sender_bytes, 11);
        pkt.write_bits(target_bytes, 11);
        pkt.write_bits(prefix_bytes, 5);
        pkt.write_bits(channel_bytes, 7);
        pkt.write_bits(text_bytes, 12);
        pkt.write_bits(0u32, 15); // chat_flags
        pkt.write_bit(false); // hide_chat_log
        pkt.write_bit(false); // fake_sender_name
        pkt.write_bit(false); // has_unused_801
        pkt.write_bit(false); // has_channel_guid
        pkt.flush_bits();

        pkt.write_string(&self.sender_name);
        pkt.write_string(&self.target_name);
        pkt.write_string(&self.prefix);
        pkt.write_string(&self.channel);
        pkt.write_string(&self.text);
    }
}

/// C++ `WorldPackets::Chat::ChatPlayerNotfound`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatPlayerNotfound {
    pub name: String,
}

impl ServerPacket for ChatPlayerNotfound {
    const OPCODE: ServerOpcodes = ServerOpcodes::ChatPlayerNotfound;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_bits(self.name.len() as u32, 9);
        pkt.flush_bits();
        pkt.write_string(&self.name);
    }
}

// ── Emote Packets ─────────────────────────────────────────────────────────────

/// CMSG_EMOTE — client clears its emote state (no body).
///
/// C# ref: `EmoteClient` in `ChatPackets.cs` — `Read()` is empty.
pub struct EmoteClient;

impl ClientPacket for EmoteClient {
    const OPCODE: ClientOpcodes = ClientOpcodes::Emote;
    fn read(_pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self)
    }
}

/// CMSG_SEND_TEXT_EMOTE — player performs a text emote (/wave, /dance, etc.).
///
/// C# ref: `CTextEmote.Read()` in `ChatPackets.cs`.
pub struct CTextEmote {
    pub target: ObjectGuid,
    pub emote_id: i32,
    pub sound_index: i32,
    pub spell_visual_kit_ids: Vec<i32>,
    pub sequence_variation: i32,
}

impl ClientPacket for CTextEmote {
    const OPCODE: ClientOpcodes = ClientOpcodes::SendTextEmote;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        let target = pkt.read_packed_guid()?;
        let emote_id = pkt.read_int32()?;
        let sound_index = pkt.read_int32()?;
        let count = pkt.read_int32()? as usize;
        let sequence_variation = pkt.read_int32()?;
        let mut spell_visual_kit_ids = Vec::with_capacity(count);
        for _ in 0..count {
            spell_visual_kit_ids.push(pkt.read_int32()?);
        }
        Ok(Self {
            target,
            emote_id,
            sound_index,
            spell_visual_kit_ids,
            sequence_variation,
        })
    }
}

/// SMSG_TEXT_EMOTE — broadcasts text emote to nearby players (chat text).
///
/// C# ref: `STextEmote.Write()` in `ChatPackets.cs`.
pub struct STextEmote {
    pub source_guid: ObjectGuid,
    pub source_account_guid: ObjectGuid,
    pub emote_id: i32,
    pub sound_index: i32,
    pub target_guid: ObjectGuid,
}

impl ServerPacket for STextEmote {
    const OPCODE: ServerOpcodes = ServerOpcodes::TextEmote;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_packed_guid(&self.source_guid);
        pkt.write_packed_guid(&self.source_account_guid);
        pkt.write_int32(self.emote_id);
        pkt.write_int32(self.sound_index);
        pkt.write_packed_guid(&self.target_guid);
    }
}

/// SMSG_EMOTE — plays the emote animation on the unit.
///
/// C# ref: `EmoteMessage.Write()` in `ChatPackets.cs`.
/// Also sent by `Unit.HandleEmoteCommand()`.
pub struct EmoteMessage {
    pub guid: ObjectGuid,
    pub emote_id: i32,
    pub spell_visual_kit_ids: Vec<i32>,
    pub sequence_variation: i32,
}

impl ServerPacket for EmoteMessage {
    const OPCODE: ServerOpcodes = ServerOpcodes::Emote;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_packed_guid(&self.guid);
        pkt.write_int32(self.emote_id);
        pkt.write_int32(self.spell_visual_kit_ids.len() as i32);
        pkt.write_int32(self.sequence_variation);
        for &id in &self.spell_visual_kit_ids {
            pkt.write_int32(id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chat_register_addon_prefixes_reads_cpp_layout() {
        let mut writer = WorldPacket::new_empty();
        writer.write_uint32(2);
        writer.write_bits(3, 5);
        writer.write_string("ABC");
        writer.write_bits(4, 5);
        writer.write_string("DEFG");

        let mut reader = WorldPacket::from_bytes(writer.data());
        let packet = ChatRegisterAddonPrefixes::read(&mut reader).unwrap();
        assert_eq!(packet.prefixes, vec!["ABC", "DEFG"]);
    }

    #[test]
    fn join_channel_reads_cpp_layout() {
        let mut writer = WorldPacket::new_empty();
        writer.write_int32(0);
        writer.write_bit(false);
        writer.write_bit(false);
        writer.write_bits(5, 7);
        writer.write_bits(4, 7);
        writer.write_string("Trade");
        writer.write_string("pass");

        let mut reader = WorldPacket::from_bytes(writer.data());
        let packet = JoinChannel::read(&mut reader).unwrap();
        assert_eq!(packet.chat_channel_id, 0);
        assert!(!packet.create_voice_session);
        assert!(!packet.internal);
        assert_eq!(packet.channel_name, "Trade");
        assert_eq!(packet.password, "pass");
    }

    #[test]
    fn leave_channel_reads_cpp_layout() {
        let mut writer = WorldPacket::new_empty();
        writer.write_int32(2);
        writer.write_bits(5, 7);
        writer.write_string("Trade");

        let mut reader = WorldPacket::from_bytes(writer.data());
        let packet = LeaveChannel::read(&mut reader).unwrap();
        assert_eq!(packet.zone_channel_id, 2);
        assert_eq!(packet.channel_name, "Trade");
    }

    #[test]
    fn channel_command_reads_cpp_layout() {
        let mut writer = WorldPacket::new_empty();
        writer.write_bits(7, 7);
        writer.write_string("Looking");

        let mut reader = WorldPacket::from_bytes(writer.data());
        let packet = ChannelCommand::read(&mut reader).unwrap();
        assert_eq!(packet.channel_name, "Looking");
    }

    #[test]
    fn channel_player_command_reads_cpp_layout() {
        let mut writer = WorldPacket::new_empty();
        writer.write_bits(5, 7);
        writer.write_bits(6, 9);
        writer.write_string("Trade");
        writer.write_string("Player");

        let mut reader = WorldPacket::from_bytes(writer.data());
        let packet = ChannelPlayerCommand::read(&mut reader).unwrap();
        assert_eq!(packet.channel_name, "Trade");
        assert_eq!(packet.name, "Player");
    }

    #[test]
    fn channel_password_reads_cpp_layout() {
        let mut writer = WorldPacket::new_empty();
        writer.write_bits(5, 7);
        writer.write_bits(4, 7);
        writer.write_string("Trade");
        writer.write_string("pass");

        let mut reader = WorldPacket::from_bytes(writer.data());
        let packet = ChannelPassword::read(&mut reader).unwrap();
        assert_eq!(packet.channel_name, "Trade");
        assert_eq!(packet.password, "pass");
    }

    #[test]
    fn channel_notify_invalid_name_uses_cpp_notice_type() {
        let bytes = ChannelNotify::invalid_name("1bad").to_bytes();
        assert_eq!(
            u16::from_le_bytes([bytes[0], bytes[1]]),
            ServerOpcodes::ChannelNotify as u16
        );
        // C++ `operator<<(ObjectGuid)` writes packed GUIDs. Empty sender/account/target
        // GUIDs are 2 bytes each, not three raw 16-byte values.
        assert_eq!(bytes.len(), 27);

        let mut payload = WorldPacket::from_bytes(&bytes[2..]);
        assert_eq!(
            payload.read_bits(6).unwrap() as u8,
            CHAT_INVALID_NAME_NOTICE_LIKE_CPP
        );
        assert_eq!(payload.read_bits(7).unwrap(), 4);
        assert_eq!(payload.read_bits(6).unwrap(), 0);
        assert_eq!(payload.read_packed_guid().unwrap(), ObjectGuid::EMPTY);
        assert_eq!(payload.read_packed_guid().unwrap(), ObjectGuid::EMPTY);
        assert_eq!(payload.read_uint32().unwrap(), 0);
        assert_eq!(payload.read_packed_guid().unwrap(), ObjectGuid::EMPTY);
        assert_eq!(payload.read_uint32().unwrap(), 0);
        assert_eq!(payload.read_int32().unwrap(), 0);
        assert_eq!(payload.read_string(4).unwrap(), "1bad");
        assert!(payload.is_empty());
    }

    #[test]
    fn chat_addon_message_reads_cpp_layout() {
        let mut writer = WorldPacket::new_empty();
        writer.write_bits(3, 5);
        writer.write_bits(5, 8);
        writer.write_bit(true);
        writer.write_int32(ChatMsg::Guild as i32);
        writer.write_string("ABC");
        writer.write_string("hello");

        let mut reader = WorldPacket::from_bytes(writer.data());
        let packet = ChatAddonMessage::read(&mut reader).unwrap();
        assert_eq!(packet.msg_type, ChatMsg::Guild as i32);
        assert_eq!(packet.prefix, "ABC");
        assert_eq!(packet.text, "hello");
        assert!(packet.is_logged);
    }

    #[test]
    fn chat_addon_message_whisper_reads_cpp_layout() {
        let mut writer = WorldPacket::new_empty();
        writer.write_bits(6, 9);
        writer.write_bits(3, 5);
        writer.write_bits(5, 8);
        writer.write_string("Target");
        writer.write_string("ABC");
        writer.write_string("hello");

        let mut reader = WorldPacket::from_bytes(writer.data());
        let packet = ChatAddonMessageWhisper::read(&mut reader).unwrap();
        assert_eq!(packet.target, "Target");
        assert_eq!(packet.prefix, "ABC");
        assert_eq!(packet.message, "hello");
    }

    #[test]
    fn chat_message_without_secure_bit_defaults_like_cpp() {
        let mut writer = WorldPacket::new_empty();
        writer.write_uint16(ClientOpcodes::ChatMessageYell as u16);
        writer.write_int32(0);
        writer.write_bits(5, 11);
        writer.write_string("hello");

        let mut reader = WorldPacket::from_bytes(writer.data());
        reader.skip_opcode();
        let packet = ChatMessage::read(&mut reader).unwrap();

        assert_eq!(packet.text, "hello");
        assert!(!packet.is_secure);
    }

    #[test]
    fn chat_player_notfound_writes_cpp_layout() {
        let packet = ChatPlayerNotfound {
            name: "Missing".to_string(),
        };
        let data = packet.to_bytes();
        let mut payload = WorldPacket::from_bytes(&data);

        assert_eq!(
            payload.read_uint16().unwrap(),
            ServerOpcodes::ChatPlayerNotfound as u16
        );
        assert_eq!(payload.read_bits(9).unwrap(), 7);
        assert_eq!(payload.read_string(7).unwrap(), "Missing");
        assert!(payload.is_empty());
    }

    #[test]
    fn chat_report_ignored_reads_cpp_layout() {
        let ignored_guid = ObjectGuid::create_player(0, 0x12345);
        let mut writer = WorldPacket::new_empty();
        writer.write_packed_guid(&ignored_guid);
        writer.write_uint8(2);

        let mut reader = WorldPacket::from_bytes(writer.data());
        let packet = ChatReportIgnored::read(&mut reader).unwrap();

        assert_eq!(packet.ignored_guid, ignored_guid);
        assert_eq!(packet.reason, 2);
        assert!(reader.is_empty());
    }

    #[test]
    fn chat_message_afk_reads_cpp_layout() {
        let mut writer = WorldPacket::new_empty();
        writer.write_bits(9, 11);
        writer.write_string("bio break");

        let mut reader = WorldPacket::from_bytes(writer.data());
        let packet = ChatMessageAfk::read(&mut reader).unwrap();

        assert_eq!(packet.text, "bio break");
        assert!(reader.is_empty());
    }

    #[test]
    fn chat_message_dnd_reads_cpp_layout() {
        let mut writer = WorldPacket::new_empty();
        writer.write_bits(4, 11);
        writer.write_string("busy");

        let mut reader = WorldPacket::from_bytes(writer.data());
        let packet = ChatMessageDnd::read(&mut reader).unwrap();

        assert_eq!(packet.text, "busy");
        assert!(reader.is_empty());
    }

    #[test]
    fn chat_pkt_system_universal_writes_cpp_wire_values() {
        let packet = ChatPkt {
            msg_type: ChatMsg::System,
            language: 0,
            sender_guid: ObjectGuid::EMPTY,
            sender_name: String::new(),
            target_guid: ObjectGuid::EMPTY,
            target_name: String::new(),
            prefix: String::new(),
            channel: String::new(),
            text: "hello".to_string(),
            virtual_realm: 0,
        };
        let mut writer = WorldPacket::new_empty();
        packet.write(&mut writer);
        let payload = writer.data();

        assert_eq!(payload[0], 0x00, "CHAT_MSG_SYSTEM must be 0x00 on wire");
        assert_eq!(&payload[1..5], &[0x00, 0x00, 0x00, 0x00]);
    }
}
