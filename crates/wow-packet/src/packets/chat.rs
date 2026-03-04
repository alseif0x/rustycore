// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Chat packet definitions (CMSG_CHAT_MESSAGE_* / SMSG_CHAT).

use wow_constants::{ClientOpcodes, ServerOpcodes};
use wow_core::ObjectGuid;

use crate::world_packet::{PacketError, WorldPacket};
use crate::{ClientPacket, ServerPacket};

// ── Chat message types ────────────────────────────────────────────

/// Chat message type (ChatMsg in C#).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ChatMsg {
    Say         = 1,
    Party       = 2,
    Raid        = 3,
    Guild       = 4,
    Officer     = 5,
    Yell        = 6,
    Whisper     = 7,
    WhisperForeign = 8,
    WhisperInform = 9,
    Emote       = 10,
    TextEmote   = 11,
    System      = 12,
    Monster     = 13,  // creature say
    MonsterParty = 14,
    MonsterYell = 15,
    MonsterWhisper = 16,
    MonsterEmote = 17,
    Channel     = 18,
    ChannelJoin = 19,
    ChannelLeave = 20,
    ChannelList = 21,
    ChannelNotice = 22,
    ChannelNoticeUser = 23,
    Afk         = 24,
    Dnd         = 25,
    Ignored     = 26,
    Skill       = 27,
    Loot        = 28,
    Money       = 29,
    Opening     = 30,
    Tradeskills = 31,
    PetInfo     = 32,
    CombatMiscInfo = 33,
    CombatXpGain = 34,
    CombatHonorGain = 35,
    CombatFactionChange = 36,
    BgSystemNeutral = 37,
    BgSystemAlliance = 38,
    BgSystemHorde = 39,
    RaidLeader  = 40,
    RaidWarning = 41,
    RaidBossEmote = 42,
    RaidBossWhisper = 43,
    Filtered    = 44,
    Battleground = 45,
    BattlegroundLeader = 46,
    Restricted  = 47,
    BattleNet   = 48,
    Achievement = 49,
    GuildAchievement = 50,
    ArenaPoints = 51,
    PartyLeader = 52,
    Targeticons = 53,
    BnWhisper   = 54,
    BnWhisperInform = 55,
    BnInlineToast = 56,
    BnInlineToastAlert = 57,
    BnInlineToastBroadcast = 58,
    BnInlineToastBroadcastInform = 59,
    BnInlineToastConversation = 60,
    BnWhisperPlayerOffline = 61,
    CombatGuildXpGain = 62,
    Currency    = 63,
    QuestBossEmote = 64,
    PetBattleCombatLog = 65,
    PetBattleInfo = 66,
    InstanceChat = 67,
    InstanceChatLeader = 68,
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
            true
        };
        // bit reader auto-resets on next byte read
        let text = pkt.read_string(len)?;
        Ok(Self { language, text, is_secure })
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
        Ok(Self { language, target, text })
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

// ── SMSG_CHAT ─────────────────────────────────────────────────────

/// Server sends this to broadcast a chat message to nearby players.
///
/// C# ChatPkt.Write():
/// ```
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
    pub language: i32,
    pub sender_guid: ObjectGuid,
    pub sender_name: String,
    pub target_guid: ObjectGuid,
    pub target_name: String,
    pub channel: String,
    pub text: String,
    pub virtual_realm: u32,
}

impl ServerPacket for ChatPkt {
    const OPCODE: ServerOpcodes = ServerOpcodes::Chat;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_uint8(self.msg_type as u8);
        pkt.write_int32(self.language);
        pkt.write_packed_guid(&self.sender_guid);
        pkt.write_packed_guid(&ObjectGuid::EMPTY); // sender_guild_guid
        pkt.write_packed_guid(&ObjectGuid::EMPTY); // sender_account_guid
        pkt.write_packed_guid(&self.target_guid);
        pkt.write_uint32(self.virtual_realm);   // target_virtual_address
        pkt.write_uint32(self.virtual_realm);   // sender_virtual_address
        pkt.write_int32(0i32);   // achievement_id
        pkt.write_float(0.0f32); // display_time
        pkt.write_int32(0i32);   // spell_id

        let sender_bytes = self.sender_name.len() as u32;
        let target_bytes = self.target_name.len() as u32;
        let prefix_bytes = 0u32;
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
        // prefix (empty)
        pkt.write_string(&self.channel);
        pkt.write_string(&self.text);
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
