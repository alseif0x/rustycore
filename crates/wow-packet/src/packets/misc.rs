// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Miscellaneous login-sequence packets sent to the client during character login.

use std::io::{Read, Write};

use flate2::{Compression, read::ZlibDecoder, write::ZlibEncoder};
use wow_constants::{ClientOpcodes, ServerOpcodes};
use wow_core::guid::HighGuid;
use wow_core::{ObjectGuid, Position};

use crate::packets::item::InvUpdate;
use crate::packets::spell::CastSpellRequest;
use crate::world_packet::PacketError;
use crate::{ClientPacket, ServerPacket, WorldPacket};

pub use wow_constants::{BuyResult, SellResult};

// ── CanDuel (CMSG 0x3664 / SMSG 0x2947) ───────────────────────────

/// C++ `WorldPackets::Duel::CanDuel`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CanDuel {
    pub target_guid: ObjectGuid,
    pub to_the_death: bool,
}

impl ClientPacket for CanDuel {
    const OPCODE: ClientOpcodes = ClientOpcodes::CanDuel;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        let guid_bytes = pkt.read_bytes(16)?;
        let mut raw = [0u8; 16];
        raw.copy_from_slice(&guid_bytes);
        Ok(Self {
            target_guid: ObjectGuid::from_raw_bytes(&raw),
            to_the_death: pkt.read_bit()?,
        })
    }
}

/// C++ `WorldPackets::Duel::CanDuelResult`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CanDuelResult {
    pub target_guid: ObjectGuid,
    pub result: bool,
}

impl ServerPacket for CanDuelResult {
    const OPCODE: ServerOpcodes = ServerOpcodes::CanDuelResult;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_bytes(&self.target_guid.to_raw_bytes());
        pkt.write_bit(self.result);
        pkt.flush_bits();
    }
}

/// C++ `WorldPackets::Duel::DuelResponse`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DuelResponse {
    pub arbiter_guid: ObjectGuid,
    pub accepted: bool,
    pub forfeited: bool,
}

impl ClientPacket for DuelResponse {
    const OPCODE: ClientOpcodes = ClientOpcodes::DuelResponse;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        let guid_bytes = pkt.read_bytes(16)?;
        let mut raw = [0u8; 16];
        raw.copy_from_slice(&guid_bytes);
        Ok(Self {
            arbiter_guid: ObjectGuid::from_raw_bytes(&raw),
            accepted: pkt.read_bit()?,
            forfeited: pkt.read_bit()?,
        })
    }
}

/// C++ `WorldPackets::Duel::DuelCountdown`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DuelCountdown {
    pub countdown_ms: u32,
}

impl ServerPacket for DuelCountdown {
    const OPCODE: ServerOpcodes = ServerOpcodes::DuelCountdown;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_uint32(self.countdown_ms);
    }
}

// ── FarSight (CMSG 0x34e8) ──────────────────────────────────────────

/// C++ `WorldPackets::Misc::FarSight`: one bit toggling seer to current viewpoint/self.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FarSight {
    pub enable: bool,
}

impl ClientPacket for FarSight {
    const OPCODE: ClientOpcodes = ClientOpcodes::FarSight;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self {
            enable: pkt.read_bit()?,
        })
    }
}

// ── Bank (CMSG 0x3997 / 0x3996 / 0x34B4) ──────────────────────────

/// C++ `WorldPackets::Bank::AutoBankItem`: `InvUpdate`, source bag and slot.
#[derive(Debug, Clone)]
pub struct AutoBankItem {
    pub inv_update: InvUpdate,
    pub bag: u8,
    pub slot: u8,
}

impl ClientPacket for AutoBankItem {
    const OPCODE: ClientOpcodes = ClientOpcodes::AutobankItem;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self {
            inv_update: InvUpdate::read(pkt)?,
            bag: pkt.read_uint8()?,
            slot: pkt.read_uint8()?,
        })
    }
}

/// C++ `WorldPackets::Bank::AutoStoreBankItem`: `InvUpdate`, source bag and slot.
#[derive(Debug, Clone)]
pub struct AutoStoreBankItem {
    pub inv_update: InvUpdate,
    pub bag: u8,
    pub slot: u8,
}

impl ClientPacket for AutoStoreBankItem {
    const OPCODE: ClientOpcodes = ClientOpcodes::AutostoreBankItem;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self {
            inv_update: InvUpdate::read(pkt)?,
            bag: pkt.read_uint8()?,
            slot: pkt.read_uint8()?,
        })
    }
}

// ── Guild Bank ─────────────────────────────────────────────────────

/// C++ `WorldPackets::Guild::GuildBankActivate`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GuildBankActivate {
    pub banker: ObjectGuid,
    pub full_update: bool,
}

impl ClientPacket for GuildBankActivate {
    const OPCODE: ClientOpcodes = ClientOpcodes::GuildBankActivate;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self {
            banker: pkt.read_guid()?,
            full_update: pkt.read_bit()?,
        })
    }
}

/// C++ `WorldPackets::Guild::GuildBankQueryTab`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GuildBankQueryTab {
    pub banker: ObjectGuid,
    pub tab: u8,
    pub full_update: bool,
}

impl ClientPacket for GuildBankQueryTab {
    const OPCODE: ClientOpcodes = ClientOpcodes::GuildBankQueryTab;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self {
            banker: pkt.read_guid()?,
            tab: pkt.read_uint8()?,
            full_update: pkt.read_bit()?,
        })
    }
}

/// C++ `WorldPackets::Guild::GuildCommandResult`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuildCommandResult {
    pub name: String,
    pub result: i32,
    pub command: i32,
}

impl GuildCommandResult {
    pub const COMMAND_VIEW_TAB_LIKE_CPP: i32 = 21;
    pub const ERR_PLAYER_NOT_IN_GUILD_LIKE_CPP: i32 = 9;

    pub fn player_not_in_guild_view_tab_like_cpp() -> Self {
        Self {
            name: String::new(),
            result: Self::ERR_PLAYER_NOT_IN_GUILD_LIKE_CPP,
            command: Self::COMMAND_VIEW_TAB_LIKE_CPP,
        }
    }
}

impl ServerPacket for GuildCommandResult {
    const OPCODE: ServerOpcodes = ServerOpcodes::GuildCommandResult;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_int32(self.result);
        pkt.write_int32(self.command);
        pkt.write_bits(self.name.len() as u32, 8);
        pkt.flush_bits();
        pkt.write_string(&self.name);
    }
}

/// C++ `WorldPackets::Guild::AutoGuildBankItem`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AutoGuildBankItem {
    pub banker: ObjectGuid,
    pub bank_tab: u8,
    pub bank_slot: u8,
    pub container_item_slot: u8,
    pub container_slot: Option<u8>,
}

impl ClientPacket for AutoGuildBankItem {
    const OPCODE: ClientOpcodes = ClientOpcodes::AutoGuildBankItem;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        let banker = pkt.read_guid()?;
        let bank_tab = pkt.read_uint8()?;
        let bank_slot = pkt.read_uint8()?;
        let container_item_slot = pkt.read_uint8()?;
        let has_container_slot = pkt.read_bit()?;
        let container_slot = if has_container_slot {
            Some(pkt.read_uint8()?)
        } else {
            None
        };

        Ok(Self {
            banker,
            bank_tab,
            bank_slot,
            container_item_slot,
            container_slot,
        })
    }
}

/// C++ `WorldPackets::Guild::AutoStoreGuildBankItem`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AutoStoreGuildBankItem {
    pub banker: ObjectGuid,
    pub bank_tab: u8,
    pub bank_slot: u8,
}

impl ClientPacket for AutoStoreGuildBankItem {
    const OPCODE: ClientOpcodes = ClientOpcodes::AutoStoreGuildBankItem;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self {
            banker: pkt.read_guid()?,
            bank_tab: pkt.read_uint8()?,
            bank_slot: pkt.read_uint8()?,
        })
    }
}

/// C++ `WorldPackets::Bank::BuyBankSlot`: a single banker `ObjectGuid`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BuyBankSlot {
    pub guid: ObjectGuid,
}

impl ClientPacket for BuyBankSlot {
    const OPCODE: ClientOpcodes = ClientOpcodes::BuyBankSlot;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self {
            guid: pkt.read_guid()?,
        })
    }
}

/// C++ `WorldPackets::Bank::ChangeBankBagSlotFlag`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChangeBankBagSlotFlag {
    pub slot: u32,
    pub flag: u32,
    pub enabled: bool,
}

impl ClientPacket for ChangeBankBagSlotFlag {
    const OPCODE: ClientOpcodes = ClientOpcodes::ChangeBankBagSlotFlag;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self {
            slot: pkt.read_uint32()?,
            flag: pkt.read_uint32()?,
            enabled: pkt.read_bit()?,
        })
    }
}

// ── BugReport (CMSG 0x3687) ───────────────────────────────────────

/// C++ `WorldPackets::Ticket::BugReport`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BugReport {
    pub report_type: u32,
    pub text: String,
    pub diag_info: String,
}

impl ClientPacket for BugReport {
    const OPCODE: ClientOpcodes = ClientOpcodes::BugReport;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        let report_type = u32::from(pkt.read_bit()?);
        let diag_len = pkt.read_bits(12)? as usize;
        let text_len = pkt.read_bits(10)? as usize;
        let diag_info = pkt.read_string(diag_len)?;
        let text = pkt.read_string(text_len)?;
        Ok(Self {
            report_type,
            text,
            diag_info,
        })
    }
}

/// C++ `WorldPackets::Ticket::GMTicketAcknowledgeSurvey`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GmTicketAcknowledgeSurvey {
    pub case_id: i32,
}

impl ClientPacket for GmTicketAcknowledgeSurvey {
    const OPCODE: ClientOpcodes = ClientOpcodes::GmTicketAcknowledgeSurvey;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self {
            case_id: pkt.read_int32()?,
        })
    }
}

pub const SUPPORT_SPAM_TYPE_MAIL_LIKE_CPP: u8 = 0;
pub const SUPPORT_SPAM_TYPE_CHAT_LIKE_CPP: u8 = 1;
pub const SUPPORT_SPAM_TYPE_CALENDAR_LIKE_CPP: u8 = 2;

/// C++ `WorldPackets::Ticket::SupportTicketHeader`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SupportTicketHeader {
    pub map_id: i32,
    pub position: Position,
    pub facing: f32,
    pub program: i32,
}

impl SupportTicketHeader {
    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        let map_id = pkt.read_int32()?;
        let position = Position::xyz(pkt.read_float()?, pkt.read_float()?, pkt.read_float()?);
        let facing = pkt.read_float()?;
        let program = pkt.read_int32()?;
        Ok(Self {
            map_id,
            position,
            facing,
            program,
        })
    }
}

/// C++ `WorldPackets::Ticket::SupportTicketSubmitBug`.
#[derive(Debug, Clone, PartialEq)]
pub struct SupportTicketSubmitBug {
    pub header: SupportTicketHeader,
    pub message: String,
}

impl ClientPacket for SupportTicketSubmitBug {
    const OPCODE: ClientOpcodes = ClientOpcodes::SupportTicketSubmitBug;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        let header = SupportTicketHeader::read(pkt)?;
        let message_len = pkt.read_bits(10)? as usize;
        let message = pkt.read_string(message_len)?;
        Ok(Self { header, message })
    }
}

/// C++ `WorldPackets::Ticket::SubmitUserFeedback`.
#[derive(Debug, Clone, PartialEq)]
pub struct SubmitUserFeedback {
    pub header: SupportTicketHeader,
    pub note: String,
    pub is_suggestion: bool,
}

impl ClientPacket for SubmitUserFeedback {
    const OPCODE: ClientOpcodes = ClientOpcodes::SubmitUserFeedback;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        let header = SupportTicketHeader::read(pkt)?;
        let note_len_with_null = pkt.read_bits(24)? as usize;
        let is_suggestion = pkt.read_bit()?;
        let note = if note_len_with_null > 0 {
            let note = pkt.read_string(note_len_with_null - 1)?;
            pkt.read_uint8()?;
            note
        } else {
            String::new()
        };
        Ok(Self {
            header,
            note,
            is_suggestion,
        })
    }
}

/// C++ `WorldPackets::Ticket::SupportTicketSubmitSuggestion`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SupportTicketSubmitSuggestion {
    pub message: String,
}

impl ClientPacket for SupportTicketSubmitSuggestion {
    const OPCODE: ClientOpcodes = ClientOpcodes::SupportTicketSubmitSuggestion;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        let message_len = pkt.read_bits(10)? as usize;
        let message = pkt.read_string(message_len)?;
        Ok(Self { message })
    }
}

/// C++ `WorldPackets::Ticket::SupportTicketChatLine`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SupportTicketChatLine {
    pub timestamp: i64,
    pub text: String,
}

impl SupportTicketChatLine {
    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        let timestamp = pkt.read_int64()?;
        let text_len = pkt.read_bits(12)? as usize;
        let text = pkt.read_string(text_len)?;
        Ok(Self { timestamp, text })
    }
}

/// C++ `WorldPackets::Ticket::SupportTicketChatLog`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SupportTicketChatLog {
    pub lines: Vec<SupportTicketChatLine>,
    pub report_line_index: Option<u32>,
}

impl SupportTicketChatLog {
    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        let lines_count = pkt.read_uint32()? as usize;
        let has_report_line_index = pkt.read_bit()?;
        pkt.reset_bits();
        let mut lines = Vec::with_capacity(lines_count);
        for _ in 0..lines_count {
            lines.push(SupportTicketChatLine::read(pkt)?);
        }
        let report_line_index = if has_report_line_index {
            Some(pkt.read_uint32()?)
        } else {
            None
        };
        Ok(Self {
            lines,
            report_line_index,
        })
    }
}

/// C++ `WorldPackets::Ticket::SupportTicketHorusChatLine::SenderRealm`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SupportTicketHorusSenderRealm {
    pub virtual_realm_address: u32,
    pub field_4: u16,
    pub field_6: u8,
}

/// C++ `WorldPackets::Ticket::SupportTicketHorusChatLine`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SupportTicketHorusChatLine {
    pub timestamp: i64,
    pub author_guid: ObjectGuid,
    pub club_id: Option<u64>,
    pub channel_guid: Option<ObjectGuid>,
    pub realm_address: Option<SupportTicketHorusSenderRealm>,
    pub slash_cmd: Option<i32>,
    pub text: String,
}

impl SupportTicketHorusChatLine {
    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        let timestamp = pkt.read_int64()?;
        let author_guid = pkt.read_packed_guid()?;
        let has_club_id = pkt.read_bit()?;
        let has_channel_guid = pkt.read_bit()?;
        let has_realm_address = pkt.read_bit()?;
        let has_slash_cmd = pkt.read_bit()?;
        let text_len = pkt.read_bits(12)? as usize;

        let club_id = if has_club_id {
            Some(pkt.read_uint64()?)
        } else {
            None
        };
        let channel_guid = if has_channel_guid {
            Some(pkt.read_packed_guid()?)
        } else {
            None
        };
        let realm_address = if has_realm_address {
            Some(SupportTicketHorusSenderRealm {
                virtual_realm_address: pkt.read_uint32()?,
                field_4: pkt.read_uint16()?,
                field_6: pkt.read_uint8()?,
            })
        } else {
            None
        };
        let slash_cmd = if has_slash_cmd {
            Some(pkt.read_int32()?)
        } else {
            None
        };
        let text = pkt.read_string(text_len)?;

        Ok(Self {
            timestamp,
            author_guid,
            club_id,
            channel_guid,
            realm_address,
            slash_cmd,
            text,
        })
    }
}

/// C++ `WorldPackets::Ticket::SupportTicketHorusChatLog`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SupportTicketHorusChatLog {
    pub lines: Vec<SupportTicketHorusChatLine>,
}

impl SupportTicketHorusChatLog {
    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        let lines_count = pkt.read_uint32()? as usize;
        let mut lines = Vec::with_capacity(lines_count);
        for _ in 0..lines_count {
            lines.push(SupportTicketHorusChatLine::read(pkt)?);
        }
        Ok(Self { lines })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SupportTicketMailInfo {
    pub mail_id: i64,
    pub mail_subject: String,
    pub mail_body: String,
}

impl SupportTicketMailInfo {
    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        let mail_id = pkt.read_int64()?;
        let body_len = pkt.read_bits(13)? as usize;
        let subject_len = pkt.read_bits(9)? as usize;
        let mail_body = pkt.read_string(body_len)?;
        let mail_subject = pkt.read_string(subject_len)?;
        Ok(Self {
            mail_id,
            mail_subject,
            mail_body,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SupportTicketCalendarEventInfo {
    pub event_id: u64,
    pub invite_id: u64,
    pub event_title: String,
}

impl SupportTicketCalendarEventInfo {
    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        let event_id = pkt.read_uint64()?;
        let invite_id = pkt.read_uint64()?;
        let title_len = pkt.read_bits(8)? as usize;
        let event_title = pkt.read_string(title_len)?;
        Ok(Self {
            event_id,
            invite_id,
            event_title,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SupportTicketPetInfo {
    pub pet_id: ObjectGuid,
    pub pet_name: String,
}

impl SupportTicketPetInfo {
    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        let pet_id = pkt.read_packed_guid()?;
        let name_len = pkt.read_bits(8)? as usize;
        let pet_name = pkt.read_string(name_len)?;
        Ok(Self { pet_id, pet_name })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SupportTicketGuildInfo {
    pub guild_id: ObjectGuid,
    pub guild_name: String,
}

impl SupportTicketGuildInfo {
    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        let name_len = pkt.read_bits(7)? as usize;
        let guild_id = pkt.read_packed_guid()?;
        let guild_name = pkt.read_string(name_len)?;
        Ok(Self {
            guild_id,
            guild_name,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SupportTicketLfgListSearchResult {
    pub ride_ticket: LfgRideTicket,
    pub group_finder_activity_id: u32,
    pub unknown1007: u8,
    pub last_title_author_guid: ObjectGuid,
    pub last_description_author_guid: ObjectGuid,
    pub last_voice_chat_author_guid: ObjectGuid,
    pub listing_creator_guid: ObjectGuid,
    pub unknown735: ObjectGuid,
    pub title: String,
    pub description: String,
    pub voice_chat: String,
}

impl SupportTicketLfgListSearchResult {
    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        let ride_ticket = LfgRideTicket::read_like_cpp(pkt)?;
        let group_finder_activity_id = pkt.read_uint32()?;
        let unknown1007 = pkt.read_uint8()?;
        let last_title_author_guid = pkt.read_packed_guid()?;
        let last_description_author_guid = pkt.read_packed_guid()?;
        let last_voice_chat_author_guid = pkt.read_packed_guid()?;
        let listing_creator_guid = pkt.read_packed_guid()?;
        let unknown735 = pkt.read_packed_guid()?;
        let title_len = pkt.read_bits(10)? as usize;
        let description_len = pkt.read_bits(11)? as usize;
        let voice_chat_len = pkt.read_bits(8)? as usize;
        let title = pkt.read_string(title_len)?;
        let description = pkt.read_string(description_len)?;
        let voice_chat = pkt.read_string(voice_chat_len)?;
        Ok(Self {
            ride_ticket,
            group_finder_activity_id,
            unknown1007,
            last_title_author_guid,
            last_description_author_guid,
            last_voice_chat_author_guid,
            listing_creator_guid,
            unknown735,
            title,
            description,
            voice_chat,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SupportTicketLfgListApplicant {
    pub ride_ticket: LfgRideTicket,
    pub comment: String,
}

impl SupportTicketLfgListApplicant {
    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        let ride_ticket = LfgRideTicket::read_like_cpp(pkt)?;
        let comment_len = pkt.read_bits(9)? as usize;
        let comment = pkt.read_string(comment_len)?;
        Ok(Self {
            ride_ticket,
            comment,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SupportTicketCommunityMessage {
    pub is_player_using_voice: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SupportTicketClubFinderResult {
    pub club_finder_posting_id: u64,
    pub club_id: u64,
    pub club_finder_guid: ObjectGuid,
    pub club_name: String,
}

impl SupportTicketClubFinderResult {
    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        let club_finder_posting_id = pkt.read_uint64()?;
        let club_id = pkt.read_uint64()?;
        let club_finder_guid = pkt.read_packed_guid()?;
        let name_len = pkt.read_bits(12)? as usize;
        let club_name = pkt.read_string(name_len)?;
        Ok(Self {
            club_finder_posting_id,
            club_id,
            club_finder_guid,
            club_name,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SupportTicketUnused910 {
    pub field_0: String,
    pub field_104: ObjectGuid,
}

impl SupportTicketUnused910 {
    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        let field_0_len = pkt.read_bits(7)? as usize;
        let field_104 = pkt.read_packed_guid()?;
        let field_0 = pkt.read_string(field_0_len)?;
        Ok(Self { field_0, field_104 })
    }
}

/// C++ `WorldPackets::Ticket::SupportTicketSubmitComplaint`.
#[derive(Debug, Clone, PartialEq)]
pub struct SupportTicketSubmitComplaint {
    pub header: SupportTicketHeader,
    pub chat_log: SupportTicketChatLog,
    pub target_character_guid: ObjectGuid,
    pub report_type: i32,
    pub major_category: i32,
    pub minor_category_flags: i32,
    pub horus_chat_log: SupportTicketHorusChatLog,
    pub note: String,
    pub mail_info: Option<SupportTicketMailInfo>,
    pub calendar_info: Option<SupportTicketCalendarEventInfo>,
    pub pet_info: Option<SupportTicketPetInfo>,
    pub guild_info: Option<SupportTicketGuildInfo>,
    pub lfg_list_search_result: Option<SupportTicketLfgListSearchResult>,
    pub lfg_list_applicant: Option<SupportTicketLfgListApplicant>,
    pub community_message: Option<SupportTicketCommunityMessage>,
    pub club_finder_result: Option<SupportTicketClubFinderResult>,
    pub unused910: Option<SupportTicketUnused910>,
}

impl ClientPacket for SupportTicketSubmitComplaint {
    const OPCODE: ClientOpcodes = ClientOpcodes::SupportTicketSubmitComplaint;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        let header = SupportTicketHeader::read(pkt)?;
        let target_character_guid = pkt.read_packed_guid()?;
        let report_type = pkt.read_int32()?;
        let major_category = pkt.read_int32()?;
        let minor_category_flags = pkt.read_int32()?;
        let chat_log = SupportTicketChatLog::read(pkt)?;

        let note_len = pkt.read_bits(10)? as usize;
        let has_mail_info = pkt.read_bit()?;
        let has_calendar_info = pkt.read_bit()?;
        let has_pet_info = pkt.read_bit()?;
        let has_guild_info = pkt.read_bit()?;
        let has_lfg_list_search_result = pkt.read_bit()?;
        let has_lfg_list_applicant = pkt.read_bit()?;
        let has_club_message = pkt.read_bit()?;
        let has_club_finder_result = pkt.read_bit()?;
        let has_unused910 = pkt.read_bit()?;

        pkt.reset_bits();
        let community_message = if has_club_message {
            let message = SupportTicketCommunityMessage {
                is_player_using_voice: pkt.read_bit()?,
            };
            pkt.reset_bits();
            Some(message)
        } else {
            None
        };

        let horus_chat_log = SupportTicketHorusChatLog::read(pkt)?;
        let note = pkt.read_string(note_len)?;
        let mail_info = if has_mail_info {
            Some(SupportTicketMailInfo::read(pkt)?)
        } else {
            None
        };
        let calendar_info = if has_calendar_info {
            Some(SupportTicketCalendarEventInfo::read(pkt)?)
        } else {
            None
        };
        let pet_info = if has_pet_info {
            Some(SupportTicketPetInfo::read(pkt)?)
        } else {
            None
        };
        let guild_info = if has_guild_info {
            Some(SupportTicketGuildInfo::read(pkt)?)
        } else {
            None
        };
        let lfg_list_search_result = if has_lfg_list_search_result {
            Some(SupportTicketLfgListSearchResult::read(pkt)?)
        } else {
            None
        };
        let lfg_list_applicant = if has_lfg_list_applicant {
            Some(SupportTicketLfgListApplicant::read(pkt)?)
        } else {
            None
        };
        let club_finder_result = if has_club_finder_result {
            Some(SupportTicketClubFinderResult::read(pkt)?)
        } else {
            None
        };
        let unused910 = if has_unused910 {
            Some(SupportTicketUnused910::read(pkt)?)
        } else {
            None
        };

        Ok(Self {
            header,
            chat_log,
            target_character_guid,
            report_type,
            major_category,
            minor_category_flags,
            horus_chat_log,
            note,
            mail_info,
            calendar_info,
            pet_info,
            guild_info,
            lfg_list_search_result,
            lfg_list_applicant,
            community_message,
            club_finder_result,
            unused910,
        })
    }
}

/// C++ `WorldPackets::Ticket::Complaint::ComplaintOffender`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ComplaintOffender {
    pub player_guid: ObjectGuid,
    pub realm_address: u32,
    pub time_since_offence: u32,
}

/// C++ `WorldPackets::Ticket::Complaint::ComplaintChat`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComplaintChat {
    pub command: u32,
    pub channel_id: u32,
    pub message_log: String,
}

/// C++ `WorldPackets::Ticket::Complaint`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Complaint {
    pub complaint_type: u8,
    pub offender: ComplaintOffender,
    pub mail_id: Option<u64>,
    pub chat: Option<ComplaintChat>,
    pub calendar_event_guid: Option<u64>,
    pub calendar_invite_guid: Option<u64>,
}

impl ClientPacket for Complaint {
    const OPCODE: ClientOpcodes = ClientOpcodes::Complaint;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        let complaint_type = pkt.read_uint8()?;
        let offender = ComplaintOffender {
            player_guid: pkt.read_packed_guid()?,
            realm_address: pkt.read_uint32()?,
            time_since_offence: pkt.read_uint32()?,
        };

        let mut mail_id = None;
        let mut chat = None;
        let mut calendar_event_guid = None;
        let mut calendar_invite_guid = None;

        match complaint_type {
            SUPPORT_SPAM_TYPE_MAIL_LIKE_CPP => {
                mail_id = Some(pkt.read_uint64()?);
            }
            SUPPORT_SPAM_TYPE_CHAT_LIKE_CPP => {
                let command = pkt.read_uint32()?;
                let channel_id = pkt.read_uint32()?;
                let message_len = pkt.read_bits(12)? as usize;
                let message_log = pkt.read_string(message_len)?;
                chat = Some(ComplaintChat {
                    command,
                    channel_id,
                    message_log,
                });
            }
            SUPPORT_SPAM_TYPE_CALENDAR_LIKE_CPP => {
                calendar_event_guid = Some(pkt.read_uint64()?);
                calendar_invite_guid = Some(pkt.read_uint64()?);
            }
            _ => {}
        }

        Ok(Self {
            complaint_type,
            offender,
            mail_id,
            chat,
            calendar_event_guid,
            calendar_invite_guid,
        })
    }
}

// ── Object update recovery (CMSG 0x3183 / 0x3184) ───────────────────────────

/// C++ `WorldPackets::Misc::ObjectUpdateFailed`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObjectUpdateFailed {
    pub object_guid: ObjectGuid,
}

impl ClientPacket for ObjectUpdateFailed {
    const OPCODE: ClientOpcodes = ClientOpcodes::ObjectUpdateFailed;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self {
            object_guid: pkt.read_packed_guid()?,
        })
    }
}

/// C++ `WorldPackets::Misc::ObjectUpdateRescued`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObjectUpdateRescued {
    pub object_guid: ObjectGuid,
}

impl ClientPacket for ObjectUpdateRescued {
    const OPCODE: ClientOpcodes = ClientOpcodes::ObjectUpdateRescued;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self {
            object_guid: pkt.read_packed_guid()?,
        })
    }
}

// ── StandStateChange (CMSG 0x318c) ──────────────────────────────────────────

/// C++ `WorldPackets::Misc::StandStateChange`: raw uint32, validated by handler.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StandStateChange {
    pub stand_state: u32,
}

impl ClientPacket for StandStateChange {
    const OPCODE: ClientOpcodes = ClientOpcodes::StandStateChange;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self {
            stand_state: pkt.read_uint32()?,
        })
    }
}

/// C++ `WorldPackets::Misc::SetTaxiBenchmarkMode`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SetTaxiBenchmarkMode {
    pub enable: bool,
}

impl ClientPacket for SetTaxiBenchmarkMode {
    const OPCODE: ClientOpcodes = ClientOpcodes::SetTaxiBenchmarkMode;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self {
            enable: pkt.read_bit()?,
        })
    }
}

/// C++ `WorldPackets::Taxi::ActivateTaxi`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActivateTaxi {
    pub vendor: ObjectGuid,
    pub node: u32,
    pub ground_mount_id: u32,
    pub flying_mount_id: u32,
}

impl ClientPacket for ActivateTaxi {
    const OPCODE: ClientOpcodes = ClientOpcodes::ActivateTaxi;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self {
            vendor: pkt.read_packed_guid()?,
            node: pkt.read_uint32()?,
            ground_mount_id: pkt.read_uint32()?,
            flying_mount_id: pkt.read_uint32()?,
        })
    }
}

pub const ERR_TAXIOK_LIKE_CPP: u8 = 0;
pub const ERR_TAXITOOFARAWAY_LIKE_CPP: u8 = 4;

pub struct ActivateTaxiReply {
    pub reply: u8,
}

impl ServerPacket for ActivateTaxiReply {
    const OPCODE: ServerOpcodes = ServerOpcodes::ActivateTaxiReply;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_bits(u32::from(self.reply), 4);
        pkt.flush_bits();
    }
}

/// C++ `WorldPackets::ClientConfig::SetAdvancedCombatLogging`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SetAdvancedCombatLogging {
    pub enable: bool,
}

impl ClientPacket for SetAdvancedCombatLogging {
    const OPCODE: ClientOpcodes = ClientOpcodes::SetAdvancedCombatLogging;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self {
            enable: pkt.read_bit()?,
        })
    }
}

/// C++ `WorldPackets::Misc::SetCurrencyFlags`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SetCurrencyFlags {
    pub currency_id: u32,
    pub flags: u8,
}

impl ClientPacket for SetCurrencyFlags {
    const OPCODE: ClientOpcodes = ClientOpcodes::SetCurrencyFlags;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self {
            currency_id: pkt.read_uint32()?,
            flags: pkt.read_uint8()?,
        })
    }
}

/// C++ `WorldPackets::Misc::SetDungeonDifficulty`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SetDungeonDifficulty {
    pub difficulty_id: u32,
}

impl ClientPacket for SetDungeonDifficulty {
    const OPCODE: ClientOpcodes = ClientOpcodes::SetDungeonDifficulty;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self {
            difficulty_id: pkt.read_uint32()?,
        })
    }
}

/// C++ `WorldPackets::Misc::SetRaidDifficulty`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SetRaidDifficulty {
    pub difficulty_id: i32,
    pub legacy: u8,
}

impl ClientPacket for SetRaidDifficulty {
    const OPCODE: ClientOpcodes = ClientOpcodes::SetRaidDifficulty;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self {
            difficulty_id: pkt.read_int32()?,
            legacy: pkt.read_uint8()?,
        })
    }
}

/// C++ `WorldPackets::Misc::SetDifficultyId`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SetDifficultyId {
    pub difficulty_id: u32,
}

impl ClientPacket for SetDifficultyId {
    const OPCODE: ClientOpcodes = ClientOpcodes::SetDifficultyId;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self {
            difficulty_id: pkt.read_uint32()?,
        })
    }
}

/// C++ `WorldPackets::Null` for `CMSG_TOGGLE_DIFFICULTY`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ToggleDifficulty;

impl ClientPacket for ToggleDifficulty {
    const OPCODE: ClientOpcodes = ClientOpcodes::ToggleDifficulty;

    fn read(_pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self)
    }
}

/// C++ `WorldPackets::Misc::AddonList`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddonList {
    pub addons: Vec<String>,
}

impl ClientPacket for AddonList {
    const OPCODE: ClientOpcodes = ClientOpcodes::AddonList;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        let count = pkt.read_uint32()?;
        let mut addons = Vec::new();

        for _ in 0..count {
            if pkt.remaining() == 0 {
                break;
            }

            let name_len = pkt.read_bits(10)? as usize;
            pkt.flush_bits();
            addons.push(pkt.read_string(name_len)?);
        }

        Ok(Self { addons })
    }
}

/// C++ `WorldPackets::Character::LoadingScreenNotify`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LoadingScreenNotify {
    pub map_id: u32,
    pub showing: bool,
}

impl ClientPacket for LoadingScreenNotify {
    const OPCODE: ClientOpcodes = ClientOpcodes::LoadingScreenNotify;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self {
            map_id: pkt.read_uint32()?,
            showing: pkt.read_bit()?,
        })
    }
}

/// C++ `WorldPackets::Misc::ViolenceLevel`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ViolenceLevel {
    pub violence_level: u8,
}

impl ClientPacket for ViolenceLevel {
    const OPCODE: ClientOpcodes = ClientOpcodes::ViolenceLevel;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self {
            violence_level: pkt.read_uint8()?,
        })
    }
}

/// C++ `WorldPackets::Misc::RandomRollClient`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RandomRollClient {
    pub min: i32,
    pub max: i32,
    pub party_index: Option<u8>,
}

impl ClientPacket for RandomRollClient {
    const OPCODE: ClientOpcodes = ClientOpcodes::RandomRoll;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        let has_party_index = pkt.read_bit()?;
        let min = pkt.read_int32()?;
        let max = pkt.read_int32()?;
        let party_index = if has_party_index {
            Some(pkt.read_uint8()?)
        } else {
            None
        };
        Ok(Self {
            min,
            max,
            party_index,
        })
    }
}

/// C++ `WorldPackets::Misc::RandomRoll`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RandomRoll {
    pub roller: ObjectGuid,
    pub roller_wow_account: ObjectGuid,
    pub min: i32,
    pub max: i32,
    pub result: i32,
}

impl ServerPacket for RandomRoll {
    const OPCODE: ServerOpcodes = ServerOpcodes::RandomRoll;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_guid(&self.roller);
        pkt.write_guid(&self.roller_wow_account);
        pkt.write_int32(self.min);
        pkt.write_int32(self.max);
        pkt.write_int32(self.result);
    }
}

pub const MAX_GUILD_ACHIEVEMENT_TRACKING_IDS_LIKE_CPP: usize = 10;

/// C++ `WorldPackets::Guild::DeclineGuildInvites`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeclineGuildInvites {
    pub allow: bool,
}

impl ClientPacket for DeclineGuildInvites {
    const OPCODE: ClientOpcodes = ClientOpcodes::DeclineGuildInvites;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self {
            allow: pkt.read_bit()?,
        })
    }
}

/// C++ `WorldPackets::Guild::AcceptGuildInvite`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct AcceptGuildInvite;

impl ClientPacket for AcceptGuildInvite {
    const OPCODE: ClientOpcodes = ClientOpcodes::AcceptGuildInvite;

    fn read(_pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self)
    }
}

/// C++ `WorldPackets::Guild::GuildSetAchievementTracking`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuildSetAchievementTracking {
    pub achievement_ids: Vec<u32>,
}

impl ClientPacket for GuildSetAchievementTracking {
    const OPCODE: ClientOpcodes = ClientOpcodes::GuildSetAchievementTracking;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        let count = pkt.read_uint32()? as usize;
        if count > MAX_GUILD_ACHIEVEMENT_TRACKING_IDS_LIKE_CPP {
            return Err(PacketError::StringError(format!(
                "GuildSetAchievementTracking count {count} exceeds C++ Array<10>"
            )));
        }

        let mut achievement_ids = Vec::with_capacity(count);
        for _ in 0..count {
            achievement_ids.push(pkt.read_uint32()?);
        }

        Ok(Self { achievement_ids })
    }
}

/// C++ `WorldPackets::Misc::CloseInteraction`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CloseInteraction {
    pub source_guid: ObjectGuid,
}

impl ClientPacket for CloseInteraction {
    const OPCODE: ClientOpcodes = ClientOpcodes::CloseInteraction;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self {
            source_guid: pkt.read_packed_guid()?,
        })
    }
}

// ── AccountDataTimes (SMSG 0x270a) ──────────────────────────────────

/// Number of AccountDataTypes (from C# AccountDataTypes.Max = 15).
pub const NUM_ACCOUNT_DATA_TYPES: usize = 15;
pub const MAX_ACCOUNT_DATA_SIZE_LIKE_CPP: u32 = 0xFFFF;
pub const EMPTY_ACCOUNT_DATA_COMPRESS_BOUND_LIKE_CPP: usize = 13;

/// C++ `WorldPackets::ClientConfig::RequestAccountData`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RequestAccountData {
    pub player_guid: ObjectGuid,
    pub data_type: u8,
}

impl ClientPacket for RequestAccountData {
    const OPCODE: ClientOpcodes = ClientOpcodes::RequestAccountData;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self {
            player_guid: pkt.read_packed_guid()?,
            data_type: pkt.read_bits(4)? as u8,
        })
    }
}

/// C++ `WorldPackets::ClientConfig::UserClientUpdateAccountData`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserClientUpdateAccountData {
    pub player_guid: ObjectGuid,
    pub time: i64,
    pub size: u32,
    pub data_type: u8,
    pub compressed_data: Vec<u8>,
}

impl ClientPacket for UserClientUpdateAccountData {
    const OPCODE: ClientOpcodes = ClientOpcodes::UpdateAccountData;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        let player_guid = pkt.read_packed_guid()?;
        let time = pkt.read_int64()?;
        let size = pkt.read_uint32()?;
        let data_type = pkt.read_bits(4)? as u8;
        let compressed_size = pkt.read_uint32()? as usize;
        let compressed_data = pkt.read_bytes(compressed_size)?;

        Ok(Self {
            player_guid,
            time,
            size,
            data_type,
            compressed_data,
        })
    }
}

/// C++ `WorldPackets::ClientConfig::UpdateAccountData`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateAccountData {
    pub player_guid: ObjectGuid,
    pub time: i64,
    pub size: u32,
    pub data_type: u8,
    pub compressed_data: Vec<u8>,
}

impl ServerPacket for UpdateAccountData {
    const OPCODE: ServerOpcodes = ServerOpcodes::UpdateAccountData;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_packed_guid(&self.player_guid);
        pkt.write_int64(self.time);
        pkt.write_uint32(self.size);
        pkt.write_bits(u32::from(self.data_type & 0x0F), 4);
        pkt.write_uint32(self.compressed_data.len() as u32);
        pkt.write_bytes(&self.compressed_data);
    }
}

pub fn compress_account_data_like_cpp(data: &str) -> Result<Vec<u8>, PacketError> {
    if data.is_empty() {
        return Ok(vec![0; EMPTY_ACCOUNT_DATA_COMPRESS_BOUND_LIKE_CPP]);
    }

    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder
        .write_all(data.as_bytes())
        .map_err(|e| PacketError::StringError(e.to_string()))?;
    encoder
        .finish()
        .map_err(|e| PacketError::StringError(e.to_string()))
}

pub fn decompress_account_data_like_cpp(
    compressed_data: &[u8],
    decompressed_size: u32,
) -> Result<String, PacketError> {
    if decompressed_size == 0 {
        return Ok(String::new());
    }
    if decompressed_size > MAX_ACCOUNT_DATA_SIZE_LIKE_CPP {
        return Err(PacketError::StringError(format!(
            "account data size {decompressed_size} exceeds C++ 0xFFFF limit"
        )));
    }

    let mut decoder = ZlibDecoder::new(compressed_data);
    let mut decompressed = Vec::new();
    decoder
        .read_to_end(&mut decompressed)
        .map_err(|e| PacketError::StringError(e.to_string()))?;

    let expected = decompressed_size as usize;
    if decompressed.len() > expected {
        return Err(PacketError::StringError(format!(
            "account data inflated to {} bytes, exceeds declared size {expected}",
            decompressed.len()
        )));
    }
    decompressed.resize(expected, 0);

    let mut pkt = WorldPacket::new_empty();
    pkt.write_bytes(&decompressed);
    pkt.reset_read();
    pkt.read_cstring()
}

/// Account data cache timestamps. Sent twice during login:
/// once with a global (empty) guid and once with the player's guid.
pub struct AccountDataTimes {
    pub player_guid: ObjectGuid,
    pub server_time: i64,
    pub account_times: [i64; NUM_ACCOUNT_DATA_TYPES],
}

impl AccountDataTimes {
    pub fn for_times(
        player_guid: ObjectGuid,
        account_times: [i64; NUM_ACCOUNT_DATA_TYPES],
    ) -> Self {
        Self {
            player_guid,
            server_time: unix_timestamp(),
            account_times,
        }
    }

    /// Global account data (no player).
    pub fn global() -> Self {
        Self::for_times(ObjectGuid::EMPTY, [0i64; NUM_ACCOUNT_DATA_TYPES])
    }

    /// Per-character account data.
    pub fn for_player(guid: ObjectGuid) -> Self {
        Self::for_times(guid, [0i64; NUM_ACCOUNT_DATA_TYPES])
    }
}

impl ServerPacket for AccountDataTimes {
    const OPCODE: ServerOpcodes = ServerOpcodes::AccountDataTimes;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_packed_guid(&self.player_guid);
        pkt.write_int64(self.server_time);
        for t in &self.account_times {
            pkt.write_int64(*t);
        }
    }
}

// ── TutorialFlags (SMSG 0x27be) ─────────────────────────────────────

/// Tutorial flags. All 0xFFFFFFFF means all tutorials are shown/completed.
pub struct TutorialFlags {
    pub tutorial_data: [u32; 8],
}

impl TutorialFlags {
    /// All tutorials shown (client won't display any tutorial pop-ups).
    pub fn all_shown() -> Self {
        Self {
            tutorial_data: [0xFFFFFFFF; 8],
        }
    }
}

impl ServerPacket for TutorialFlags {
    const OPCODE: ServerOpcodes = ServerOpcodes::TutorialFlags;

    fn write(&self, pkt: &mut WorldPacket) {
        for val in &self.tutorial_data {
            pkt.write_uint32(*val);
        }
    }
}

// ── UpdateWorldState (SMSG 0x2748) ──────────────────────────────────

/// C++ `WorldPackets::WorldState::UpdateWorldState`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UpdateWorldState {
    pub variable_id: u32,
    pub value: i32,
    pub hidden: bool,
}

impl UpdateWorldState {
    pub fn new(variable_id: u32, value: i32) -> Self {
        Self {
            variable_id,
            value,
            hidden: false,
        }
    }
}

impl ServerPacket for UpdateWorldState {
    const OPCODE: ServerOpcodes = ServerOpcodes::UpdateWorldState;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_uint32(self.variable_id);
        pkt.write_int32(self.value);
        pkt.write_bit(self.hidden);
        pkt.flush_bits();
    }
}

// ── FishNotHooked (SMSG 0x26cf) ─────────────────────────────────────

/// Empty packet sent when a fishing bobber is clicked before a fish is hooked.
pub struct FishNotHooked;

impl ServerPacket for FishNotHooked {
    const OPCODE: ServerOpcodes = ServerOpcodes::FishNotHooked;

    fn write(&self, _pkt: &mut WorldPacket) {}
}

// ── EnableBarberShop (SMSG 0x26bc) ──────────────────────────────────

/// Opens the barber shop/customization UI for the requested customization scope.
pub struct EnableBarberShop {
    pub customization_scope: u8,
}

impl ServerPacket for EnableBarberShop {
    const OPCODE: ServerOpcodes = ServerOpcodes::EnableBarberShop;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_uint8(self.customization_scope);
    }
}

// ── GameObjectInteraction (SMSG 0x288b) ─────────────────────────────

/// Opens a gameobject-backed interaction UI.
pub struct GameObjectInteraction {
    pub object_guid: ObjectGuid,
    pub interaction_type: i32,
}

impl ServerPacket for GameObjectInteraction {
    const OPCODE: ServerOpcodes = ServerOpcodes::GameObjectInteraction;

    fn write(&self, pkt: &mut WorldPacket) {
        for byte in self.object_guid.to_raw_bytes() {
            pkt.write_uint8(byte);
        }
        pkt.write_int32(self.interaction_type);
    }
}

// ── GameObjectCustomAnim (SMSG 0x25c4) ───────────────────────────────

/// Broadcasts a custom animation for a gameobject.
pub struct GameObjectCustomAnim {
    pub object_guid: ObjectGuid,
    pub custom_anim: u32,
    pub play_as_despawn: bool,
}

impl ServerPacket for GameObjectCustomAnim {
    const OPCODE: ServerOpcodes = ServerOpcodes::GameObjectCustomAnim;

    fn write(&self, pkt: &mut WorldPacket) {
        for byte in self.object_guid.to_raw_bytes() {
            pkt.write_uint8(byte);
        }
        pkt.write_uint32(self.custom_anim);
        pkt.write_bit(self.play_as_despawn);
        pkt.flush_bits();
    }
}

// ── GameObjectDespawn (SMSG 0x25c5) ─────────────────────────────────

/// Notifies the client that a gameobject despawned.
pub struct GameObjectDespawn {
    pub object_guid: ObjectGuid,
}

impl ServerPacket for GameObjectDespawn {
    const OPCODE: ServerOpcodes = ServerOpcodes::GameObjectDespawn;

    fn write(&self, pkt: &mut WorldPacket) {
        for byte in self.object_guid.to_raw_bytes() {
            pkt.write_uint8(byte);
        }
    }
}

// ── CapturePointRemoved (SMSG 0xbadd/UNKNOWN placeholder) ────────────

/// C++ `WorldPackets::Battleground::CapturePointRemoved`.
///
/// The legacy C++ opcode table still marks this battleground packet as
/// `0xBADD`; the archived TrinityCore source marks it as `UNKNOWN_OPCODE` too.
/// Rust cannot model two `ServerOpcodes` enum variants with the same numeric
/// placeholder, so this serializer intentionally shares the current
/// `UpdateCapturePoint` placeholder while preserving the distinct packet type
/// and payload shape.
pub struct CapturePointRemoved {
    pub capture_point_guid: ObjectGuid,
}

impl ServerPacket for CapturePointRemoved {
    const OPCODE: ServerOpcodes = ServerOpcodes::UpdateCapturePoint;

    fn write(&self, pkt: &mut WorldPacket) {
        for byte in self.capture_point_guid.to_raw_bytes() {
            pkt.write_uint8(byte);
        }
    }
}

// ── GameObjectSetStateLocal (SMSG 0x2806) ───────────────────────────

/// Sets a gameobject state only for the receiving client.
pub struct GameObjectSetStateLocal {
    pub object_guid: ObjectGuid,
    pub state: u8,
}

impl ServerPacket for GameObjectSetStateLocal {
    const OPCODE: ServerOpcodes = ServerOpcodes::GameObjectSetStateLocal;

    fn write(&self, pkt: &mut WorldPacket) {
        for byte in self.object_guid.to_raw_bytes() {
            pkt.write_uint8(byte);
        }
        pkt.write_uint8(self.state);
    }
}

// ── AnimKit control packets ────────────────────────────────────────

/// C++ `WorldPackets::Misc::SetAIAnimKit`: ObjectGuid + uint16 AnimKitID.
pub struct SetAiAnimKit {
    pub unit: ObjectGuid,
    pub anim_kit_id: u16,
}

impl ServerPacket for SetAiAnimKit {
    const OPCODE: ServerOpcodes = ServerOpcodes::SetAiAnimKit;

    fn write(&self, pkt: &mut WorldPacket) {
        for byte in self.unit.to_raw_bytes() {
            pkt.write_uint8(byte);
        }
        pkt.write_uint16(self.anim_kit_id);
    }
}

/// C++ `WorldPackets::Misc::SetMovementAnimKit`: ObjectGuid + uint16 AnimKitID.
pub struct SetMovementAnimKit {
    pub unit: ObjectGuid,
    pub anim_kit_id: u16,
}

impl ServerPacket for SetMovementAnimKit {
    const OPCODE: ServerOpcodes = ServerOpcodes::SetMovementAnimKit;

    fn write(&self, pkt: &mut WorldPacket) {
        for byte in self.unit.to_raw_bytes() {
            pkt.write_uint8(byte);
        }
        pkt.write_uint16(self.anim_kit_id);
    }
}

/// C++ `WorldPackets::Misc::SetMeleeAnimKit`: ObjectGuid + uint16 AnimKitID.
pub struct SetMeleeAnimKit {
    pub unit: ObjectGuid,
    pub anim_kit_id: u16,
}

impl ServerPacket for SetMeleeAnimKit {
    const OPCODE: ServerOpcodes = ServerOpcodes::SetMeleeAnimKit;

    fn write(&self, pkt: &mut WorldPacket) {
        for byte in self.unit.to_raw_bytes() {
            pkt.write_uint8(byte);
        }
        pkt.write_uint16(self.anim_kit_id);
    }
}

// ── UpdateCapturePoint (SMSG 0xbadd) ───────────────────────────────

/// C++ `WorldPackets::Battleground::UpdateCapturePoint`.
pub struct UpdateCapturePoint {
    pub guid: ObjectGuid,
    pub position: Position,
    pub state: u8,
    pub capture_time_ms: u32,
    pub capture_total_duration_ms: u32,
}

impl ServerPacket for UpdateCapturePoint {
    const OPCODE: ServerOpcodes = ServerOpcodes::UpdateCapturePoint;

    fn write(&self, pkt: &mut WorldPacket) {
        for byte in self.guid.to_raw_bytes() {
            pkt.write_uint8(byte);
        }
        pkt.write_float(self.position.x);
        pkt.write_float(self.position.y);
        pkt.write_uint8(self.state);

        if matches!(self.state, 2 | 3) {
            pkt.write_uint32(self.capture_time_ms);
            pkt.write_uint32(self.capture_total_duration_ms);
        }
    }
}

// ── PageText (SMSG 0x2719) ───────────────────────────────────────────

/// Opens a page-text object; the client queries the page contents separately.
pub struct PageText {
    pub gameobject_guid: ObjectGuid,
}

impl ServerPacket for PageText {
    const OPCODE: ServerOpcodes = ServerOpcodes::PageText;

    fn write(&self, pkt: &mut WorldPacket) {
        for byte in self.gameobject_guid.to_raw_bytes() {
            pkt.write_uint8(byte);
        }
    }
}

// ── TriggerCinematic (SMSG 0x27ca) ──────────────────────────────────

/// Starts a cinematic sequence for the player.
pub struct TriggerCinematic {
    pub cinematic_id: u32,
    pub conversation_guid: ObjectGuid,
}

impl ServerPacket for TriggerCinematic {
    const OPCODE: ServerOpcodes = ServerOpcodes::TriggerCinematic;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_uint32(self.cinematic_id);
        for byte in self.conversation_guid.to_raw_bytes() {
            pkt.write_uint8(byte);
        }
    }
}

// ── TriggerMovie (SMSG 0x26cb) ──────────────────────────────────────

/// C++ `WorldPackets::Misc::TriggerMovie`: starts a movie by Movie.db2 id.
pub struct TriggerMovie {
    pub movie_id: u32,
}

impl ServerPacket for TriggerMovie {
    const OPCODE: ServerOpcodes = ServerOpcodes::TriggerMovie;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_uint32(self.movie_id);
    }
}

// ── FeatureSystemStatus (SMSG 0x25bf) — IN-GAME version ─────────────

/// Feature system status sent AFTER entering the world.
/// This is the in-game variant; for the character select screen use
/// [`FeatureSystemStatusGlueScreen`].
pub struct FeatureSystemStatus {
    pub cfg_realm_id: u32,
    pub cfg_realm_rec_id: i32,
}

impl FeatureSystemStatus {
    pub fn default_wotlk() -> Self {
        Self {
            cfg_realm_id: 1,
            cfg_realm_rec_id: 0,
        }
    }
}

impl ServerPacket for FeatureSystemStatus {
    const OPCODE: ServerOpcodes = ServerOpcodes::FeatureSystemStatus;

    fn write(&self, pkt: &mut WorldPacket) {
        // ── Fixed-size fields (exact C# order) ──
        pkt.write_uint8(2); // ComplaintStatus
        pkt.write_uint32(self.cfg_realm_id); // CfgRealmID
        pkt.write_int32(self.cfg_realm_rec_id); // CfgRealmRecID

        // RAFSystem (5 fields)
        pkt.write_uint32(0); // RAFSystem.MaxRecruits
        pkt.write_uint32(0); // RAFSystem.MaxRecruitMonths
        pkt.write_uint32(0); // RAFSystem.MaxRecruitmentUses
        pkt.write_uint32(0); // RAFSystem.DaysInCycle
        pkt.write_uint32(0); // RAFSystem.Unknown1007

        // Token/Kiosk/Store
        pkt.write_uint32(0); // TokenPollTimeSeconds
        pkt.write_uint32(0); // KioskSessionMinutes
        pkt.write_int64(0); // TokenBalanceAmount
        pkt.write_uint32(0); // BpayStoreProductDeliveryDelay
        pkt.write_uint32(0); // ClubsPresenceUpdateTimer
        pkt.write_uint32(0); // HiddenUIClubsPresenceUpdateTimer

        // Season/Rules/Query
        pkt.write_int32(0); // ActiveSeason
        pkt.write_int32(0); // GameRuleValues.Count
        pkt.write_int16(50); // MaxPlayerNameQueriesPerPacket
        pkt.write_int16(0); // PlayerNameQueryTelemetryInterval
        pkt.write_uint32(60); // PlayerNameQueryInterval (seconds)

        // GameRuleValues (empty, count=0)

        // ── Bit flags (42 boolean fields — exact C# order) ──
        pkt.write_bit(false); // VoiceEnabled
        pkt.write_bit(false); // EuropaTicketSystemStatus.HasValue
        pkt.write_bit(false); // BpayStoreEnabled
        pkt.write_bit(false); // BpayStoreAvailable
        pkt.write_bit(false); // BpayStoreDisabledByParentalControls
        pkt.write_bit(false); // ItemRestorationButtonEnabled
        pkt.write_bit(false); // BrowserEnabled
        pkt.write_bit(false); // SessionAlert.HasValue
        pkt.write_bit(false); // RAFSystem.Enabled
        pkt.write_bit(false); // RAFSystem.RecruitingEnabled
        pkt.write_bit(false); // CharUndeleteEnabled
        pkt.write_bit(false); // RestrictedAccount
        pkt.write_bit(false); // CommerceSystemEnabled
        pkt.write_bit(true); // TutorialsEnabled
        pkt.write_bit(false); // Unk67
        pkt.write_bit(false); // WillKickFromWorld
        pkt.write_bit(false); // KioskModeEnabled
        pkt.write_bit(false); // CompetitiveModeEnabled
        pkt.write_bit(false); // TokenBalanceEnabled
        pkt.write_bit(false); // WarModeFeatureEnabled
        pkt.write_bit(false); // ClubsEnabled
        pkt.write_bit(false); // ClubsBattleNetClubTypeAllowed
        pkt.write_bit(false); // ClubsCharacterClubTypeAllowed
        pkt.write_bit(false); // ClubsPresenceUpdateEnabled
        pkt.write_bit(false); // VoiceChatDisabledByParentalControl
        pkt.write_bit(false); // VoiceChatMutedByParentalControl
        pkt.write_bit(false); // QuestSessionEnabled
        pkt.write_bit(false); // IsMuted
        pkt.write_bit(false); // ClubFinderEnabled
        pkt.write_bit(false); // Unknown901CheckoutRelated
        pkt.write_bit(false); // TextToSpeechFeatureEnabled
        pkt.write_bit(false); // ChatDisabledByDefault
        pkt.write_bit(false); // ChatDisabledByPlayer
        pkt.write_bit(false); // LFGListCustomRequiresAuthenticator
        pkt.write_bit(false); // AddonsDisabled
        pkt.write_bit(false); // WarGamesEnabled
        pkt.write_bit(false); // ContentTrackingEnabled
        pkt.write_bit(false); // IsSellAllJunkEnabled
        pkt.write_bit(false); // IsGroupFinderEnabled
        pkt.write_bit(false); // IsLFDEnabled
        pkt.write_bit(false); // IsLFREnabled
        pkt.write_bit(false); // IsPremadeGroupEnabled
        pkt.flush_bits();

        // ── QuickJoinConfig ──
        pkt.write_bit(false); // QuickJoinConfig.ToastsDisabled
        pkt.write_float(0.0); // QuickJoinConfig.ToastDuration
        pkt.write_float(0.0); // QuickJoinConfig.DelayDuration
        pkt.write_float(0.0); // QuickJoinConfig.QueueMultiplier
        pkt.write_float(0.0); // QuickJoinConfig.PlayerMultiplier
        pkt.write_float(0.0); // QuickJoinConfig.PlayerFriendValue
        pkt.write_float(0.0); // QuickJoinConfig.PlayerGuildValue
        pkt.write_float(0.0); // QuickJoinConfig.ThrottleInitialThreshold
        pkt.write_float(0.0); // QuickJoinConfig.ThrottleDecayTime
        pkt.write_float(0.0); // QuickJoinConfig.ThrottlePrioritySpike
        pkt.write_float(0.0); // QuickJoinConfig.ThrottleMinThreshold
        pkt.write_float(0.0); // QuickJoinConfig.ThrottlePvPPriorityNormal
        pkt.write_float(0.0); // QuickJoinConfig.ThrottlePvPPriorityLow
        pkt.write_float(0.0); // QuickJoinConfig.ThrottlePvPHonorThreshold
        pkt.write_float(0.0); // QuickJoinConfig.ThrottleLfgListPriorityDefault
        pkt.write_float(0.0); // QuickJoinConfig.ThrottleLfgListPriorityAbove
        pkt.write_float(0.0); // QuickJoinConfig.ThrottleLfgListPriorityBelow
        pkt.write_float(0.0); // QuickJoinConfig.ThrottleLfgListIlvlScalingAbove
        pkt.write_float(0.0); // QuickJoinConfig.ThrottleLfgListIlvlScalingBelow
        pkt.write_float(0.0); // QuickJoinConfig.ThrottleRfPriorityAbove
        pkt.write_float(0.0); // QuickJoinConfig.ThrottleRfIlvlScalingAbove
        pkt.write_float(0.0); // QuickJoinConfig.ThrottleDfMaxItemLevel
        pkt.write_float(0.0); // QuickJoinConfig.ThrottleDfBestPriority

        // SessionAlert (optional — not present, bit was false)

        // Squelch
        pkt.write_bit(false); // Squelch.IsSquelched
        pkt.write_packed_guid(&ObjectGuid::EMPTY); // Squelch.BnetAccountGuid
        pkt.write_packed_guid(&ObjectGuid::EMPTY); // Squelch.GuildGuid

        // EuropaTicketSystemStatus (optional — not present, bit was false)
    }
}

// ── FeatureSystemStatusGlueScreen (SMSG 0x25c0) — CHARACTER SELECT ──

/// Feature system status for the glue screen (character select).
/// This is the version sent during session init, BEFORE entering the world.
/// Different opcode and format from [`FeatureSystemStatus`].
pub struct FeatureSystemStatusGlueScreen {
    pub max_characters_per_realm: i32,
}

impl FeatureSystemStatusGlueScreen {
    /// Default values matching C# SendFeatureSystemStatusGlueScreen.
    pub fn default_wotlk() -> Self {
        Self {
            max_characters_per_realm: 60,
        }
    }
}

impl ServerPacket for FeatureSystemStatusGlueScreen {
    const OPCODE: ServerOpcodes = ServerOpcodes::FeatureSystemStatusGlueScreen;

    fn write(&self, pkt: &mut WorldPacket) {
        // ── 27 bit flags (exact C# order) ──
        pkt.write_bit(false); // BpayStoreEnabled
        pkt.write_bit(false); // BpayStoreAvailable
        pkt.write_bit(false); // BpayStoreDisabledByParentalControls
        pkt.write_bit(false); // CharUndeleteEnabled
        pkt.write_bit(false); // CommerceSystemEnabled
        pkt.write_bit(false); // Unk14
        pkt.write_bit(false); // WillKickFromWorld
        pkt.write_bit(false); // IsExpansionPreorderInStore

        pkt.write_bit(false); // KioskModeEnabled
        pkt.write_bit(false); // CompetitiveModeEnabled
        pkt.write_bit(false); // unused 10.0.2
        pkt.write_bit(false); // TrialBoostEnabled
        pkt.write_bit(false); // TokenBalanceEnabled
        pkt.write_bit(false); // LiveRegionCharacterListEnabled
        pkt.write_bit(false); // LiveRegionCharacterCopyEnabled
        pkt.write_bit(false); // LiveRegionAccountCopyEnabled

        pkt.write_bit(false); // LiveRegionKeyBindingsCopyEnabled
        pkt.write_bit(false); // Unknown901CheckoutRelated
        pkt.write_bit(false); // unused 10.0.2
        pkt.write_bit(true); // EuropaTicketSystemStatus.HasValue (C# sets this!)
        pkt.write_bit(false); // unused 10.0.2
        pkt.write_bit(false); // LaunchETA.HasValue
        pkt.write_bit(false); // AddonsDisabled
        pkt.write_bit(false); // Unused1000

        pkt.write_bit(false); // AccountSaveDataExportEnabled
        pkt.write_bit(false); // AccountLockedByExport
        pkt.write_bit(false); // RealmHiddenAlert (not empty = false)

        // No RealmHiddenAlert bits (it's empty)
        pkt.flush_bits();

        // ── EuropaTicketSystemStatus (present — bit was true) ──
        // EuropaTicketConfig.Write():
        //   4 bits (TicketsEnabled, BugsEnabled, ComplaintsEnabled, SuggestionsEnabled)
        //   then SavedThrottleObjectState (4 × u32)
        pkt.write_bit(false); // TicketsEnabled (SupportTicketsEnabled config, default false)
        pkt.write_bit(false); // BugsEnabled (SupportBugsEnabled config, default false)
        pkt.write_bit(false); // ComplaintsEnabled (SupportComplaintsEnabled config, default false)
        pkt.write_bit(false); // SuggestionsEnabled (SupportSuggestionsEnabled config, default false)
        // SavedThrottleObjectState — C# hardcodes these in SendFeatureSystemStatusGlueScreen:
        pkt.write_uint32(10); // MaxTries
        pkt.write_uint32(60000); // PerMilliseconds
        pkt.write_uint32(1); // TryCount
        pkt.write_uint32(111111); // LastResetTimeBeforeNow

        // ── Sequential numeric fields (exact C# order) ──
        pkt.write_uint32(0); // TokenPollTimeSeconds
        pkt.write_uint32(0); // KioskSessionMinutes
        pkt.write_int64(0); // TokenBalanceAmount
        pkt.write_int32(self.max_characters_per_realm); // MaxCharactersPerRealm
        pkt.write_int32(0); // LiveRegionCharacterCopySourceRegions.Count
        pkt.write_uint32(0); // BpayStoreProductDeliveryDelay
        pkt.write_int32(0); // ActiveCharacterUpgradeBoostType
        pkt.write_int32(0); // ActiveClassTrialBoostType
        pkt.write_int32(0); // MinimumExpansionLevel (Classic=0)
        pkt.write_int32(2); // MaximumExpansionLevel (WotLK=2)
        pkt.write_int32(0); // ActiveSeason
        pkt.write_int32(0); // GameRuleValues.Count
        pkt.write_int16(50); // MaxPlayerNameQueriesPerPacket
        pkt.write_int16(600); // PlayerNameQueryTelemetryInterval (C# default=600)
        pkt.write_uint32(10); // PlayerNameQueryInterval (C# default=10 seconds)
        pkt.write_int32(0); // DebugTimeEvents.Count
        pkt.write_int32(0); // Unused1007

        // LaunchETA (optional — not present)
        // RealmHiddenAlert (optional — empty)
        // LiveRegionCharacterCopySourceRegions (empty, count=0)
        // GameRuleValues (empty, count=0)
        // DebugTimeEvents (empty, count=0)
    }
}

// ── ClientCacheVersion (SMSG 0x291c) ────────────────────────────────

/// Client cache version sent during session init.
pub struct ClientCacheVersion {
    pub cache_version: u32,
}

impl ServerPacket for ClientCacheVersion {
    const OPCODE: ServerOpcodes = ServerOpcodes::CacheVersion;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_uint32(self.cache_version);
    }
}

// ── AvailableHotfixes (SMSG 0x290f) ────────────────────────────────

/// C++ `DB2Manager::HotfixId`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HotfixId {
    pub push_id: i32,
    pub unique_id: u32,
}

/// Available hotfixes sent during session init.
pub struct AvailableHotfixes {
    pub virtual_realm_address: u32,
    pub hotfixes: Vec<HotfixId>,
}

impl ServerPacket for AvailableHotfixes {
    const OPCODE: ServerOpcodes = ServerOpcodes::AvailableHotfixes;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_uint32(self.virtual_realm_address);
        pkt.write_uint32(self.hotfixes.len() as u32);
        for hotfix_id in &self.hotfixes {
            pkt.write_int32(hotfix_id.push_id);
            pkt.write_uint32(hotfix_id.unique_id);
        }
    }
}

// ── ConnectionStatus (SMSG 0x2809) ─────────────────────────────────

/// BattleNet connection status sent at end of session init.
pub struct ConnectionStatus {
    pub state: u8,
    pub suppress_notification: bool,
}

impl ServerPacket for ConnectionStatus {
    const OPCODE: ServerOpcodes = ServerOpcodes::BattleNetConnectionStatus;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_bits(u32::from(self.state), 2);
        pkt.write_bit(self.suppress_notification);
        pkt.flush_bits();
    }
}

// ── SetTimeZoneInformation (SMSG 0x2677) ────────────────────────────

/// Time zone info sent to the client.
pub struct SetTimeZoneInformation {
    pub server_timezone: String,
    pub game_timezone: String,
    pub server_regional_timezone: String,
}

impl SetTimeZoneInformation {
    pub fn utc() -> Self {
        Self {
            server_timezone: "Etc/UTC".into(),
            game_timezone: "Etc/UTC".into(),
            server_regional_timezone: "Etc/UTC".into(),
        }
    }
}

impl ServerPacket for SetTimeZoneInformation {
    const OPCODE: ServerOpcodes = ServerOpcodes::SetTimeZoneInformation;

    fn write(&self, pkt: &mut WorldPacket) {
        // 7-bit length-prefixed strings
        pkt.write_bits(self.server_timezone.len() as u32, 7);
        pkt.write_bits(self.game_timezone.len() as u32, 7);
        pkt.write_bits(self.server_regional_timezone.len() as u32, 7);
        pkt.flush_bits();

        pkt.write_string(&self.server_timezone);
        pkt.write_string(&self.game_timezone);
        pkt.write_string(&self.server_regional_timezone);
    }
}

// ── LoginSetTimeSpeed (SMSG 0x270d) ─────────────────────────────────

/// Set game time and speed at login.
pub struct LoginSetTimeSpeed {
    pub server_time: i32,
    pub game_time: i32,
    pub new_speed: f32,
    pub server_time_holiday_offset: i32,
    pub game_time_holiday_offset: i32,
}

impl LoginSetTimeSpeed {
    /// Current time with standard speed (1/24 = real-time game day).
    pub fn now() -> Self {
        let t = wow_core::GameTime::now().to_packed() as i32;
        Self {
            server_time: t,
            game_time: t,
            new_speed: 1.0 / 24.0,
            server_time_holiday_offset: 0,
            game_time_holiday_offset: 0,
        }
    }
}

impl ServerPacket for LoginSetTimeSpeed {
    const OPCODE: ServerOpcodes = ServerOpcodes::LoginSetTimeSpeed;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_int32(self.server_time);
        pkt.write_int32(self.game_time);
        pkt.write_float(self.new_speed);
        pkt.write_int32(self.server_time_holiday_offset);
        pkt.write_int32(self.game_time_holiday_offset);
    }
}

// ── SetupCurrency (SMSG 0x2573) ─────────────────────────────────────

/// One C++ `WorldPackets::Misc::SetupCurrency::Record`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SetupCurrencyRecord {
    pub type_id: i32,
    pub quantity: i32,
    pub weekly_quantity: Option<u32>,
    pub max_weekly_quantity: Option<u32>,
    pub tracked_quantity: Option<u32>,
    pub max_quantity: Option<i32>,
    pub total_earned: Option<i32>,
    pub next_recharge_time: Option<u64>,
    pub recharge_cycle_start_time: Option<u64>,
    pub flags: u8,
}

/// C++ `WorldPackets::Misc::SetupCurrency`.
pub struct SetupCurrency {
    pub data: Vec<SetupCurrencyRecord>,
}

impl SetupCurrency {
    pub fn empty() -> Self {
        Self { data: Vec::new() }
    }

    pub fn from_records(data: Vec<SetupCurrencyRecord>) -> Self {
        Self { data }
    }
}

impl ServerPacket for SetupCurrency {
    const OPCODE: ServerOpcodes = ServerOpcodes::SetupCurrency;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_uint32(self.data.len() as u32);

        for record in &self.data {
            pkt.write_int32(record.type_id);
            pkt.write_int32(record.quantity);

            pkt.write_bit(record.weekly_quantity.is_some());
            pkt.write_bit(record.max_weekly_quantity.is_some());
            pkt.write_bit(record.tracked_quantity.is_some());
            pkt.write_bit(record.max_quantity.is_some());
            pkt.write_bit(record.total_earned.is_some());
            pkt.write_bit(record.next_recharge_time.is_some());
            pkt.write_bit(record.recharge_cycle_start_time.is_some());
            pkt.write_bits(u32::from(record.flags), 5);
            pkt.flush_bits();

            if let Some(value) = record.weekly_quantity {
                pkt.write_uint32(value);
            }
            if let Some(value) = record.max_weekly_quantity {
                pkt.write_uint32(value);
            }
            if let Some(value) = record.tracked_quantity {
                pkt.write_uint32(value);
            }
            if let Some(value) = record.max_quantity {
                pkt.write_int32(value);
            }
            if let Some(value) = record.total_earned {
                pkt.write_int32(value);
            }
            if let Some(value) = record.next_recharge_time {
                pkt.write_uint64(value);
            }
            if let Some(value) = record.recharge_cycle_start_time {
                pkt.write_uint64(value);
            }
        }
    }
}

// ── SetCurrency (SMSG 0x2574) ───────────────────────────────────────

/// Currency delta update.
///
/// Mirrors C++ `WorldPackets::Misc::SetCurrency::Write`.
pub struct SetCurrency {
    pub type_id: i32,
    pub quantity: i32,
    pub flags: u32,
    pub weekly_quantity: Option<i32>,
    pub tracked_quantity: Option<i32>,
    pub max_quantity: Option<i32>,
    pub total_earned: Option<i32>,
    pub suppress_chat_log: bool,
    pub quantity_change: Option<i32>,
    pub quantity_gain_source: Option<i32>,
    pub quantity_lost_source: Option<i32>,
    pub first_craft_operation_id: Option<u32>,
    pub next_recharge_time: Option<u64>,
    pub recharge_cycle_start_time: Option<u64>,
    pub overflown_currency_id: Option<i32>,
}

impl SetCurrency {
    pub fn vendor_gain(type_id: i32, quantity: i32, amount: i32) -> Self {
        Self {
            type_id,
            quantity,
            flags: 0,
            weekly_quantity: None,
            tracked_quantity: None,
            max_quantity: None,
            total_earned: None,
            suppress_chat_log: false,
            quantity_change: Some(amount),
            quantity_gain_source: Some(5),
            quantity_lost_source: None,
            first_craft_operation_id: None,
            next_recharge_time: None,
            recharge_cycle_start_time: None,
            overflown_currency_id: None,
        }
    }

    pub fn item_refund_gain(
        type_id: i32,
        quantity: i32,
        amount: i32,
        weekly_quantity: Option<i32>,
        max_quantity: Option<i32>,
        total_earned: Option<i32>,
        suppress_chat_log: bool,
    ) -> Self {
        Self {
            type_id,
            quantity,
            flags: 0,
            weekly_quantity,
            tracked_quantity: None,
            max_quantity,
            total_earned,
            suppress_chat_log,
            quantity_change: Some(amount),
            quantity_gain_source: Some(2),
            quantity_lost_source: None,
            first_craft_operation_id: None,
            next_recharge_time: None,
            recharge_cycle_start_time: None,
            overflown_currency_id: None,
        }
    }

    pub fn vendor_loss(type_id: i32, quantity: i32, amount: i32) -> Self {
        Self {
            type_id,
            quantity,
            flags: 0,
            weekly_quantity: None,
            tracked_quantity: None,
            max_quantity: None,
            total_earned: None,
            suppress_chat_log: false,
            quantity_change: Some(-amount),
            quantity_gain_source: None,
            quantity_lost_source: Some(4),
            first_craft_operation_id: None,
            next_recharge_time: None,
            recharge_cycle_start_time: None,
            overflown_currency_id: None,
        }
    }
}

impl ServerPacket for SetCurrency {
    const OPCODE: ServerOpcodes = ServerOpcodes::SetCurrency;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_int32(self.type_id);
        pkt.write_int32(self.quantity);
        pkt.write_uint32(self.flags);
        pkt.write_uint32(0);

        pkt.write_bit(self.weekly_quantity.is_some());
        pkt.write_bit(self.tracked_quantity.is_some());
        pkt.write_bit(self.max_quantity.is_some());
        pkt.write_bit(self.total_earned.is_some());
        pkt.write_bit(self.suppress_chat_log);
        pkt.write_bit(self.quantity_change.is_some());
        pkt.write_bit(self.quantity_gain_source.is_some());
        pkt.write_bit(self.quantity_lost_source.is_some());
        pkt.write_bit(self.first_craft_operation_id.is_some());
        pkt.write_bit(self.next_recharge_time.is_some());
        pkt.write_bit(self.recharge_cycle_start_time.is_some());
        pkt.write_bit(self.overflown_currency_id.is_some());
        pkt.flush_bits();

        if let Some(value) = self.weekly_quantity {
            pkt.write_int32(value);
        }
        if let Some(value) = self.tracked_quantity {
            pkt.write_int32(value);
        }
        if let Some(value) = self.max_quantity {
            pkt.write_int32(value);
        }
        if let Some(value) = self.total_earned {
            pkt.write_int32(value);
        }
        if let Some(value) = self.quantity_change {
            pkt.write_int32(value);
        }
        if let Some(value) = self.quantity_gain_source {
            pkt.write_int32(value);
        }
        if let Some(value) = self.quantity_lost_source {
            pkt.write_int32(value);
        }
        if let Some(value) = self.first_craft_operation_id {
            pkt.write_uint32(value);
        }
        if let Some(value) = self.next_recharge_time {
            pkt.write_uint64(value);
        }
        if let Some(value) = self.recharge_cycle_start_time {
            pkt.write_uint64(value);
        }
        if let Some(value) = self.overflown_currency_id {
            pkt.write_int32(value);
        }
    }
}

// ── UndeleteCooldownStatusResponse (SMSG 0x27ce) ────────────────────

/// Response to GetUndeleteCharacterCooldownStatus.
/// Tells the client whether character undelete is on cooldown.
pub struct UndeleteCooldownStatusResponse {
    pub on_cooldown: bool,
    pub max_cooldown: i32,
    pub current_cooldown: i32,
}

impl UndeleteCooldownStatusResponse {
    /// No cooldown — character undelete is available.
    pub fn no_cooldown() -> Self {
        Self {
            on_cooldown: false,
            max_cooldown: 0,
            current_cooldown: 0,
        }
    }
}

impl ServerPacket for UndeleteCooldownStatusResponse {
    const OPCODE: ServerOpcodes = ServerOpcodes::UndeleteCooldownStatusResponse;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_bit(self.on_cooldown);
        pkt.write_int32(self.max_cooldown);
        pkt.write_int32(self.current_cooldown);
    }
}

// ── ServerTimeOffset (SMSG 0x2714) ───────────────────────────────────

/// Response to ServerTimeOffsetRequest. Sends the current realm time.
pub struct ServerTimeOffset {
    pub time: i64,
}

impl ServerTimeOffset {
    /// Current time.
    pub fn now() -> Self {
        Self {
            time: unix_timestamp(),
        }
    }
}

impl ServerPacket for ServerTimeOffset {
    const OPCODE: ServerOpcodes = ServerOpcodes::ServerTimeOffset;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_int64(self.time);
    }
}

// ── InitWorldStates (SMSG 0x2746) ─────────────────────────────────

/// World state variables for the current zone. Sent after UpdateObject.
/// For a minimal login, we send an empty list.
pub struct InitWorldStates {
    pub map_id: i32,
    pub area_id: i32,
    pub subarea_id: i32,
}

impl InitWorldStates {
    pub fn new(map_id: i32, zone_id: i32) -> Self {
        Self {
            map_id,
            area_id: zone_id,
            subarea_id: 0,
        }
    }
}

impl ServerPacket for InitWorldStates {
    const OPCODE: ServerOpcodes = ServerOpcodes::InitWorldStates;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_int32(self.map_id);
        pkt.write_int32(self.area_id);
        pkt.write_int32(self.subarea_id);
        pkt.write_int32(0); // Worldstates.Count = 0
    }
}

// ── UpdateTalentData (SMSG 0x25d7) ──────────────────────────────────

/// Talent data sent during login. Empty for fresh characters.
pub struct UpdateTalentData;

impl ServerPacket for UpdateTalentData {
    const OPCODE: ServerOpcodes = ServerOpcodes::UpdateTalentData;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_int32(0); // UnspentTalentPoints
        pkt.write_uint8(0); // ActiveGroup
        pkt.write_int32(1); // TalentGroupInfos.Count (1 spec group)

        // TalentGroupInfo[0] — C# writes count twice (uint8 + uint32):
        pkt.write_uint8(0); // (byte)Talents.Count
        pkt.write_uint32(0); // (uint)Talents.Count
        pkt.write_uint8(6); // (byte)MaxGlyphSlotIndex
        pkt.write_uint32(6); // (uint)MaxGlyphSlotIndex
        pkt.write_uint8(0); // SpecID = 0 (no spec)
        // 0 talent entries
        // 6 glyph entries (all 0):
        for _ in 0..6 {
            pkt.write_uint16(0);
        }

        pkt.write_bit(false); // IsPetTalents
        pkt.flush_bits();
    }
}

// ── SendKnownSpells (SMSG 0x2c27) ──────────────────────────────────

/// Known spells list sent during login.
///
/// C# format:
/// ```text
/// [bit]  InitialLogin
/// [i32]  KnownSpells.Count
/// [i32]  FavoriteSpells.Count
/// [i32 × N] KnownSpells (spell IDs)
/// [i32 × M] FavoriteSpells (spell IDs)
/// ```
pub struct SendKnownSpells {
    pub initial_login: bool,
    pub known_spells: Vec<i32>,
    pub favorite_spells: Vec<i32>,
}

impl SendKnownSpells {
    /// Empty spell list for fresh characters.
    pub fn empty() -> Self {
        Self {
            initial_login: true,
            known_spells: Vec::new(),
            favorite_spells: Vec::new(),
        }
    }
}

impl ServerPacket for SendKnownSpells {
    const OPCODE: ServerOpcodes = ServerOpcodes::SendKnownSpells;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_bit(self.initial_login);
        pkt.write_int32(self.known_spells.len() as i32);
        pkt.write_int32(self.favorite_spells.len() as i32);
        for &spell_id in &self.known_spells {
            pkt.write_int32(spell_id);
        }
        for &spell_id in &self.favorite_spells {
            pkt.write_int32(spell_id);
        }
    }
}

// ── SendUnlearnSpells (SMSG 0x2c2b) ────────────────────────────────

/// Unlearned spells list. Empty for fresh characters.
pub struct SendUnlearnSpells;

impl ServerPacket for SendUnlearnSpells {
    const OPCODE: ServerOpcodes = ServerOpcodes::SendUnlearnSpells;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_int32(0); // Spells.Count
    }
}

// ── SendSpellHistory (SMSG 0x2c28) ──────────────────────────────────

/// Spell cooldown history. Empty for fresh characters.
pub struct SendSpellHistory;

impl ServerPacket for SendSpellHistory {
    const OPCODE: ServerOpcodes = ServerOpcodes::SendSpellHistory;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_int32(0); // Entries.Count
    }
}

// ── SendSpellCharges (SMSG 0x2c2a) ──────────────────────────────────

/// Spell charges. Empty for fresh characters.
pub struct SendSpellCharges;

impl ServerPacket for SendSpellCharges {
    const OPCODE: ServerOpcodes = ServerOpcodes::SendSpellCharges;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_int32(0); // Entries.Count
    }
}

// ── UpdateActionButtons (SMSG 0x25e0) ───────────────────────────────

/// Maximum number of action bar buttons.
pub const MAX_ACTION_BUTTONS: usize = 180;

/// Action bar buttons. 180 slots (MaxActionButtons).
///
/// Each slot is a packed i64:
/// - Bits [0:23] = action ID (spell ID, item ID, macro ID)
/// - Bits [24:31] = ActionButtonType (0=Spell, 0x80=Item, etc.)
/// - Bits [32:63] = unused (0)
///
/// Reason: 0=Initialization, 1=AfterSpecSwap, 2=SpecSwap
pub struct UpdateActionButtons {
    pub buttons: [i64; MAX_ACTION_BUTTONS],
    pub reason: u8,
}

impl UpdateActionButtons {
    /// All slots empty (fresh character or initialization).
    pub fn empty() -> Self {
        Self {
            buttons: [0i64; MAX_ACTION_BUTTONS],
            reason: 0,
        }
    }

    /// Pack an action + type into the player action-button format.
    ///
    /// C++ `MAKE_ACTION_BUTTON`: `action | (type << 24)`.
    pub fn pack_button(action: i32, button_type: u8) -> i64 {
        let packed = (action & 0x00FF_FFFF) | ((button_type as i32) << 24);
        packed as u32 as i64
    }
}

impl ServerPacket for UpdateActionButtons {
    const OPCODE: ServerOpcodes = ServerOpcodes::UpdateActionButtons;

    fn write(&self, pkt: &mut WorldPacket) {
        for &btn in &self.buttons {
            pkt.write_int64(btn);
        }
        pkt.write_uint8(self.reason);
    }
}

pub use super::reputation::InitializeFactions;

// ── BindPointUpdate (SMSG 0x257d) ───────────────────────────────────

/// Hearthstone bind point. Sent during login.
pub struct BindPointUpdate {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub map_id: i32,
    pub area_id: i32,
}

impl ServerPacket for BindPointUpdate {
    const OPCODE: ServerOpcodes = ServerOpcodes::BindPointUpdate;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_float(self.x);
        pkt.write_float(self.y);
        pkt.write_float(self.z);
        pkt.write_int32(self.map_id);
        pkt.write_int32(self.area_id);
    }
}

// ── PlayerBound (SMSG 0x2ff8) ───────────────────────────────────────

/// Sent after the player's home bind has changed.
///
/// C++ `WorldPackets::Misc::PlayerBound::Write`: ObjectGuid stream + uint32 AreaID.
pub struct PlayerBound {
    pub binder_id: wow_core::ObjectGuid,
    pub area_id: u32,
}

impl ServerPacket for PlayerBound {
    const OPCODE: ServerOpcodes = ServerOpcodes::PlayerBound;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_packed_guid(&self.binder_id);
        pkt.write_uint32(self.area_id);
    }
}

// ── WorldServerInfo (SMSG 0x25ad) ───────────────────────────────────

/// World server info sent during login.
pub struct WorldServerInfo {
    pub difficulty_id: i32,
}

impl WorldServerInfo {
    pub fn default_open_world() -> Self {
        Self { difficulty_id: 0 }
    }
}

impl ServerPacket for WorldServerInfo {
    const OPCODE: ServerOpcodes = ServerOpcodes::WorldServerInfo;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_int32(self.difficulty_id);
        pkt.write_bit(false); // IsTournamentRealm
        pkt.write_bit(false); // XRealmPvpAlert
        pkt.write_bit(false); // RestrictedAccountMaxLevel.HasValue
        pkt.write_bit(false); // RestrictedAccountMaxMoney.HasValue
        pkt.write_bit(false); // InstanceGroupSize.HasValue
        pkt.flush_bits();
        // No optional fields written (all HasValue=false)
    }
}

// ── InitialSetup (SMSG 0x2580) ─────────────────────────────────────

/// Expansion level info sent during login.
pub struct InitialSetup {
    pub server_expansion_level: u8,
    pub server_expansion_tier: u8,
}

impl InitialSetup {
    pub fn wotlk() -> Self {
        Self {
            server_expansion_level: 2, // WotLK
            server_expansion_tier: 0,
        }
    }
}

impl ServerPacket for InitialSetup {
    const OPCODE: ServerOpcodes = ServerOpcodes::InitialSetup;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_uint8(self.server_expansion_level);
        pkt.write_uint8(self.server_expansion_tier);
    }
}

// ── TimeSyncRequest (SMSG 0x2dd2) ────────────────────────────────────

/// Time synchronization request. The client uses this to sync its clock.
/// Critical for loading — client expects this before it can finish.
pub struct TimeSyncRequest {
    pub sequence_index: u32,
}

impl ServerPacket for TimeSyncRequest {
    const OPCODE: ServerOpcodes = ServerOpcodes::TimeSyncRequest;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_uint32(self.sequence_index);
    }
}

// ── TimeSyncResponse (CMSG 0x3a3d) ──────────────────────────────────

/// Client response to a TimeSyncRequest. Contains the client's time
/// at the moment it received the request, plus the server's sequence index.
///
/// The server must keep sending periodic TimeSyncRequests (every 5-10s)
/// or the client's internal time sync state becomes inconsistent and crashes.
pub struct TimeSyncResponse {
    pub client_time: u32,
    pub sequence_index: u32,
}

impl ClientPacket for TimeSyncResponse {
    const OPCODE: ClientOpcodes = ClientOpcodes::TimeSyncResponse;

    fn read(packet: &mut WorldPacket) -> Result<Self, PacketError> {
        let client_time = packet.read_uint32()?;
        let sequence_index = packet.read_uint32()?;
        Ok(Self {
            client_time,
            sequence_index,
        })
    }
}

// ── ContactList (SMSG 0x278c) ────────────────────────────────────────

/// Social/Friends list. Sent during login with SocialFlag::All (0x07).
pub struct ContactList {
    pub flags: u32,
}

impl ContactList {
    /// All social flags (Friend | Ignored | Muted).
    pub fn all() -> Self {
        Self { flags: 7 }
    }
}

impl ServerPacket for ContactList {
    const OPCODE: ServerOpcodes = ServerOpcodes::ContactList;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_uint32(self.flags);
        pkt.write_bits(0u32, 8); // Contacts.Count
        pkt.flush_bits();
    }
}

// ── ActiveGlyphs (SMSG 0x2c51) ──────────────────────────────────────

/// Active glyphs. Sent during login with IsFullUpdate=true.
pub struct ActiveGlyphs {
    pub is_full_update: bool,
}

impl ServerPacket for ActiveGlyphs {
    const OPCODE: ServerOpcodes = ServerOpcodes::ActiveGlyphs;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_int32(0); // Glyphs.Count
        pkt.write_bit(self.is_full_update);
        pkt.flush_bits();
    }
}

// ── LoadEquipmentSet (SMSG 0x270e) ───────────────────────────────────

/// C++ `EQUIPMENT_SET_SLOTS` / `EQUIPMENT_SLOT_END`.
pub const EQUIPMENT_SET_SLOTS_LIKE_CPP: usize = 19;

/// Equipment set list. Empty for fresh characters.
pub struct LoadEquipmentSet;

impl ServerPacket for LoadEquipmentSet {
    const OPCODE: ServerOpcodes = ServerOpcodes::LoadEquipmentSet;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_int32(0); // SetData.Count
    }
}

/// C++ `WorldPackets::EquipmentSet::EquipmentSetID`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EquipmentSetId {
    pub guid: u64,
    pub set_type: i32,
    pub set_id: u32,
}

impl ServerPacket for EquipmentSetId {
    const OPCODE: ServerOpcodes = ServerOpcodes::EquipmentSetId;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_uint64(self.guid);
        pkt.write_int32(self.set_type);
        pkt.write_uint32(self.set_id);
    }
}

// ── SaveEquipmentSet (CMSG 0x3509) ───────────────────────────────────

/// C++ `EquipmentSetInfo::EquipmentSetData`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EquipmentSetDataLikeCpp {
    pub set_type: i32,
    pub guid: u64,
    pub set_id: u32,
    pub ignore_mask: u32,
    pub pieces: [ObjectGuid; EQUIPMENT_SET_SLOTS_LIKE_CPP],
    pub appearances: [i32; EQUIPMENT_SET_SLOTS_LIKE_CPP],
    pub enchants: [i32; 2],
    pub secondary_shoulder_appearance_id: i32,
    pub secondary_shoulder_slot: i32,
    pub secondary_weapon_appearance_id: i32,
    pub secondary_weapon_slot: i32,
    pub assigned_spec_index: i32,
    pub set_name: String,
    pub set_icon: String,
}

/// C++ `WorldPackets::EquipmentSet::SaveEquipmentSet`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SaveEquipmentSet {
    pub set: EquipmentSetDataLikeCpp,
}

impl ClientPacket for SaveEquipmentSet {
    const OPCODE: ClientOpcodes = ClientOpcodes::SaveEquipmentSet;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        let set_type = pkt.read_int32()?;
        let guid = pkt.read_uint64()?;
        let set_id = pkt.read_uint32()?;
        let ignore_mask = pkt.read_uint32()?;

        let mut pieces = [ObjectGuid::EMPTY; EQUIPMENT_SET_SLOTS_LIKE_CPP];
        let mut appearances = [0_i32; EQUIPMENT_SET_SLOTS_LIKE_CPP];
        for i in 0..EQUIPMENT_SET_SLOTS_LIKE_CPP {
            pieces[i] = pkt.read_guid()?;
            appearances[i] = pkt.read_int32()?;
        }

        let enchants = [pkt.read_int32()?, pkt.read_int32()?];
        let secondary_shoulder_appearance_id = pkt.read_int32()?;
        let secondary_shoulder_slot = pkt.read_int32()?;
        let secondary_weapon_appearance_id = pkt.read_int32()?;
        let secondary_weapon_slot = pkt.read_int32()?;

        let has_spec_index = pkt.read_bit()?;
        let set_name_len = pkt.read_bits(8)? as usize;
        let set_icon_len = pkt.read_bits(9)? as usize;
        let assigned_spec_index = if has_spec_index {
            pkt.read_int32()?
        } else {
            -1
        };

        let set_name = pkt.read_string(set_name_len)?;
        let set_icon = pkt.read_string(set_icon_len)?;

        Ok(Self {
            set: EquipmentSetDataLikeCpp {
                set_type,
                guid,
                set_id,
                ignore_mask,
                pieces,
                appearances,
                enchants,
                secondary_shoulder_appearance_id,
                secondary_shoulder_slot,
                secondary_weapon_appearance_id,
                secondary_weapon_slot,
                assigned_spec_index,
                set_name,
                set_icon,
            },
        })
    }
}

// ── AssignEquipmentSetSpec (CMSG 0x3207) ─────────────────────────────

/// C++ `WorldPackets::EquipmentSet::AssignEquipmentSetSpec`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AssignEquipmentSetSpec {
    pub set_id: u32,
    pub spec_index: u32,
}

impl ClientPacket for AssignEquipmentSetSpec {
    const OPCODE: ClientOpcodes = ClientOpcodes::AssignEquipmentSetSpec;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self {
            set_id: pkt.read_uint32()?,
            spec_index: pkt.read_uint32()?,
        })
    }
}

// ── DeleteEquipmentSet (CMSG 0x350a) ─────────────────────────────────

/// C++ `WorldPackets::EquipmentSet::DeleteEquipmentSet`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeleteEquipmentSet {
    pub id: u64,
}

impl ClientPacket for DeleteEquipmentSet {
    const OPCODE: ClientOpcodes = ClientOpcodes::DeleteEquipmentSet;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self {
            id: pkt.read_uint64()?,
        })
    }
}

// ── UseEquipmentSet (CMSG 0x3995 / SMSG 0x274f) ──────────────────────

/// C++ `WorldPackets::EquipmentSet::UseEquipmentSet::EquipmentSetItem`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UseEquipmentSetItemLikeCpp {
    pub item: ObjectGuid,
    pub container_slot: u8,
    pub slot: u8,
}

/// C++ `WorldPackets::EquipmentSet::UseEquipmentSet`.
#[derive(Debug, Clone)]
pub struct UseEquipmentSet {
    pub inv_update: InvUpdate,
    pub items: [UseEquipmentSetItemLikeCpp; EQUIPMENT_SET_SLOTS_LIKE_CPP],
    pub guid: u64,
}

impl ClientPacket for UseEquipmentSet {
    const OPCODE: ClientOpcodes = ClientOpcodes::UseEquipmentSet;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        let inv_update = InvUpdate::read(pkt)?;
        let mut items = [UseEquipmentSetItemLikeCpp {
            item: ObjectGuid::EMPTY,
            container_slot: 0,
            slot: 0,
        }; EQUIPMENT_SET_SLOTS_LIKE_CPP];
        for item in &mut items {
            item.item = pkt.read_guid()?;
            item.container_slot = pkt.read_uint8()?;
            item.slot = pkt.read_uint8()?;
        }
        let guid = pkt.read_uint64()?;

        Ok(Self {
            inv_update,
            items,
            guid,
        })
    }
}

/// C++ `WorldPackets::EquipmentSet::UseEquipmentSetResult`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UseEquipmentSetResult {
    pub guid: u64,
    pub reason: u8,
}

impl ServerPacket for UseEquipmentSetResult {
    const OPCODE: ServerOpcodes = ServerOpcodes::UseEquipmentSetResult;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_uint64(self.guid);
        pkt.write_uint8(self.reason);
    }
}

// ── AllAccountCriteria (SMSG 0x2571) ─────────────────────────────────

/// Account-wide achievement criteria. Empty for fresh accounts.
pub struct AllAccountCriteria;

impl ServerPacket for AllAccountCriteria {
    const OPCODE: ServerOpcodes = ServerOpcodes::AllAccountCriteria;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_int32(0); // Progress.Count
    }
}

// ── AllAchievementData (SMSG 0x2570) ─────────────────────────────────

/// Account-wide achievements. Empty for fresh accounts.
pub struct AllAchievementData;

impl ServerPacket for AllAchievementData {
    const OPCODE: ServerOpcodes = ServerOpcodes::AllAchievementData;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_int32(0); // Earned.Count
        pkt.write_int32(0); // Progress.Count
    }
}

// ── AccountMountUpdate (SMSG 0x25ae) ─────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AccountMount {
    pub spell_id: i32,
    pub flags: u8,
}

/// Account-wide mount collection. Sent with IsFullUpdate=true on login.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountMountUpdate {
    pub is_full_update: bool,
    pub mounts: Vec<AccountMount>,
}

impl AccountMountUpdate {
    pub fn full(mounts: Vec<AccountMount>) -> Self {
        Self {
            is_full_update: true,
            mounts,
        }
    }

    pub fn partial(mounts: Vec<AccountMount>) -> Self {
        Self {
            is_full_update: false,
            mounts,
        }
    }

    pub fn empty_full() -> Self {
        Self::full(Vec::new())
    }
}

impl ServerPacket for AccountMountUpdate {
    const OPCODE: ServerOpcodes = ServerOpcodes::AccountMountUpdate;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_bit(self.is_full_update);
        pkt.write_int32(self.mounts.len() as i32);
        for mount in &self.mounts {
            pkt.write_int32(mount.spell_id);
            pkt.write_bits(u32::from(mount.flags & 0x0f), 4);
        }
        pkt.flush_bits();
    }
}

// ── MountSpecial (CMSG 0x3280) / SpecialMountAnim (SMSG 0x269f) ─────

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MountSpecial {
    pub spell_visual_kit_ids: Vec<i32>,
    pub sequence_variation: i32,
}

impl ClientPacket for MountSpecial {
    const OPCODE: ClientOpcodes = ClientOpcodes::MountSpecialAnim;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        pkt.skip_opcode();
        let count = pkt.read_uint32()? as usize;
        let sequence_variation = pkt.read_int32()?;
        let mut spell_visual_kit_ids = Vec::with_capacity(count);
        for _ in 0..count {
            spell_visual_kit_ids.push(pkt.read_int32()?);
        }
        Ok(Self {
            spell_visual_kit_ids,
            sequence_variation,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpecialMountAnim {
    pub unit_guid: ObjectGuid,
    pub spell_visual_kit_ids: Vec<i32>,
    pub sequence_variation: i32,
}

impl ServerPacket for SpecialMountAnim {
    const OPCODE: ServerOpcodes = ServerOpcodes::SpecialMountAnim;

    fn write(&self, pkt: &mut WorldPacket) {
        for byte in self.unit_guid.to_raw_bytes() {
            pkt.write_uint8(byte);
        }
        pkt.write_uint32(self.spell_visual_kit_ids.len() as u32);
        pkt.write_int32(self.sequence_variation);
        for spell_visual_kit_id in &self.spell_visual_kit_ids {
            pkt.write_int32(*spell_visual_kit_id);
        }
    }
}

// ── AccountHeirloomUpdate (SMSG 0xBADD placeholder) ─────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AccountHeirloom {
    pub item_id: i32,
    pub flags: u32,
}

/// C++ `WorldPackets::Misc::AccountHeirloomUpdate`.
///
/// The archived C++ opcode table uses the shared `0xBADD` placeholder for this
/// packet, so Rust reuses the existing `UpdateCapturePoint` discriminant while
/// keeping a distinct packet type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountHeirloomUpdate {
    pub is_full_update: bool,
    pub unk: i32,
    pub heirlooms: Vec<AccountHeirloom>,
}

impl AccountHeirloomUpdate {
    pub fn full(heirlooms: Vec<AccountHeirloom>) -> Self {
        Self {
            is_full_update: true,
            unk: 0,
            heirlooms,
        }
    }
}

impl ServerPacket for AccountHeirloomUpdate {
    const OPCODE: ServerOpcodes = ServerOpcodes::UpdateCapturePoint;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_bit(self.is_full_update);
        pkt.flush_bits();
        pkt.write_int32(self.unk);
        pkt.write_uint32(self.heirlooms.len() as u32);
        pkt.write_uint32(self.heirlooms.len() as u32);
        for heirloom in &self.heirlooms {
            pkt.write_int32(heirloom.item_id);
        }
        for heirloom in &self.heirlooms {
            pkt.write_uint32(heirloom.flags);
        }
    }
}

// ── MountSetFavorite (CMSG 0x3633) ─────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MountSetFavorite {
    pub mount_spell_id: u32,
    pub is_favorite: bool,
}

impl ClientPacket for MountSetFavorite {
    const OPCODE: ClientOpcodes = ClientOpcodes::MountSetFavorite;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        pkt.skip_opcode();
        let mount_spell_id = pkt.read_uint32()?;
        let is_favorite = pkt.read_bit()?;
        Ok(Self {
            mount_spell_id,
            is_favorite,
        })
    }
}

// ── AccountToyUpdate (SMSG 0x25b0) ───────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AccountToy {
    pub item_id: u32,
    pub is_favorite: bool,
    pub has_fanfare: bool,
}

/// Account-wide toy collection. Sent with IsFullUpdate=true on login.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountToyUpdate {
    pub is_full_update: bool,
    pub toys: Vec<AccountToy>,
}

impl AccountToyUpdate {
    pub fn full(toys: Vec<AccountToy>) -> Self {
        Self {
            is_full_update: true,
            toys,
        }
    }
}

impl ServerPacket for AccountToyUpdate {
    const OPCODE: ServerOpcodes = ServerOpcodes::AccountToyUpdate;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_bit(self.is_full_update);
        pkt.flush_bits();
        pkt.write_int32(self.toys.len() as i32);
        pkt.write_int32(self.toys.len() as i32);
        pkt.write_int32(self.toys.len() as i32);
        for toy in &self.toys {
            pkt.write_uint32(toy.item_id);
        }
        for toy in &self.toys {
            pkt.write_bit(toy.is_favorite);
        }
        for toy in &self.toys {
            pkt.write_bit(toy.has_fanfare);
        }
        pkt.flush_bits();
    }
}

// ── AddToy (CMSG 0x3299) ─────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AddToy {
    pub item_guid: ObjectGuid,
}

impl ClientPacket for AddToy {
    const OPCODE: ClientOpcodes = ClientOpcodes::AddToy;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        pkt.skip_opcode();
        Ok(Self {
            item_guid: pkt.read_packed_guid()?,
        })
    }
}

// ── ToyClearFanfare (CMSG 0x3128) ────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ToyClearFanfare {
    pub item_id: u32,
}

impl ClientPacket for ToyClearFanfare {
    const OPCODE: ClientOpcodes = ClientOpcodes::ToyClearFanfare;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        pkt.skip_opcode();
        Ok(Self {
            item_id: pkt.read_uint32()?,
        })
    }
}

// ── UseToy (CMSG 0x329a) ─────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct UseToy {
    pub cast: CastSpellRequest,
}

impl ClientPacket for UseToy {
    const OPCODE: ClientOpcodes = ClientOpcodes::UseToy;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        pkt.skip_opcode();
        Ok(Self {
            cast: CastSpellRequest::read(pkt)?,
        })
    }
}

// ── Compact Unit Frame profiles ──────────────────────────────────────

pub const MAX_CUF_PROFILES_LIKE_CPP: usize = 5;
pub const CUF_BOOL_OPTIONS_COUNT_LIKE_CPP: usize = 27;

/// C++ `CUFProfile`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CufProfile {
    pub profile_name: String,
    pub frame_height: u16,
    pub frame_width: u16,
    pub sort_by: u8,
    pub health_text: u8,
    pub top_point: u8,
    pub bottom_point: u8,
    pub left_point: u8,
    pub top_offset: u16,
    pub bottom_offset: u16,
    pub left_offset: u16,
    pub bool_options: u32,
}

/// C++ `WorldPackets::Misc::SaveCUFProfiles`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SaveCufProfiles {
    pub profiles: Vec<CufProfile>,
}

impl ClientPacket for SaveCufProfiles {
    const OPCODE: ClientOpcodes = ClientOpcodes::SaveCufProfiles;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        pkt.skip_opcode();
        let count = pkt.read_uint32()? as usize;
        let mut profiles = Vec::with_capacity(count);
        for _ in 0..count {
            let name_len = pkt.read_bits(7)? as usize;
            let mut bool_options = 0u32;
            for option in 0..CUF_BOOL_OPTIONS_COUNT_LIKE_CPP {
                if pkt.read_bit()? {
                    bool_options |= 1 << option;
                }
            }

            profiles.push(CufProfile {
                frame_height: pkt.read_uint16()?,
                frame_width: pkt.read_uint16()?,
                sort_by: pkt.read_uint8()?,
                health_text: pkt.read_uint8()?,
                top_point: pkt.read_uint8()?,
                bottom_point: pkt.read_uint8()?,
                left_point: pkt.read_uint8()?,
                top_offset: pkt.read_uint16()?,
                bottom_offset: pkt.read_uint16()?,
                left_offset: pkt.read_uint16()?,
                profile_name: pkt.read_string(name_len)?,
                bool_options,
            });
        }

        Ok(Self { profiles })
    }
}

/// C++ `WorldPackets::Misc::LoadCUFProfiles`.
pub struct LoadCufProfiles {
    pub profiles: Vec<CufProfile>,
}

impl LoadCufProfiles {
    pub fn empty() -> Self {
        Self {
            profiles: Vec::new(),
        }
    }
}

impl Default for LoadCufProfiles {
    fn default() -> Self {
        Self::empty()
    }
}

impl ServerPacket for LoadCufProfiles {
    const OPCODE: ServerOpcodes = ServerOpcodes::LoadCufProfiles;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_uint32(self.profiles.len() as u32);
        for profile in &self.profiles {
            pkt.write_bits(profile.profile_name.len() as u32, 7);
            for option in 0..CUF_BOOL_OPTIONS_COUNT_LIKE_CPP {
                pkt.write_bit(profile.bool_options & (1 << option) != 0);
            }

            pkt.write_uint16(profile.frame_height);
            pkt.write_uint16(profile.frame_width);
            pkt.write_uint8(profile.sort_by);
            pkt.write_uint8(profile.health_text);
            pkt.write_uint8(profile.top_point);
            pkt.write_uint8(profile.bottom_point);
            pkt.write_uint8(profile.left_point);
            pkt.write_uint16(profile.top_offset);
            pkt.write_uint16(profile.bottom_offset);
            pkt.write_uint16(profile.left_offset);
            pkt.write_string(&profile.profile_name);
        }
    }
}

// ── AuraUpdate (SMSG 0x2c1f) ─────────────────────────────────────────

/// Aura update for a unit. On login, sent with UpdateAll=true and no auras.
pub struct AuraUpdate {
    pub unit_guid: ObjectGuid,
    pub update_all: bool,
}

impl AuraUpdate {
    /// Full aura update with no auras (fresh login).
    pub fn empty_for(guid: ObjectGuid) -> Self {
        Self {
            unit_guid: guid,
            update_all: true,
        }
    }
}

impl ServerPacket for AuraUpdate {
    const OPCODE: ServerOpcodes = ServerOpcodes::AuraUpdate;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_bit(self.update_all);
        pkt.write_bits(0u32, 9); // Auras.Count
        // No aura entries
        // write_packed_guid auto-flushes the 10 pending bits
        pkt.write_packed_guid(&self.unit_guid);
    }
}

// ── Battle pet journal lock packets ─────────────────────────────────

/// C++ `WorldPackets::BattlePet::BattlePetRequestJournal`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BattlePetRequestJournal;

impl ClientPacket for BattlePetRequestJournal {
    const OPCODE: ClientOpcodes = ClientOpcodes::BattlePetRequestJournal;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        pkt.skip_opcode();
        Ok(Self)
    }
}

/// C++ `WorldPackets::BattlePet::BattlePetRequestJournalLock`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BattlePetRequestJournalLock;

impl ClientPacket for BattlePetRequestJournalLock {
    const OPCODE: ClientOpcodes = ClientOpcodes::BattlePetRequestJournalLock;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        pkt.skip_opcode();
        Ok(Self)
    }
}

/// C++ `WorldPackets::BattlePet::BattlePetSetBattleSlot`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BattlePetSetBattleSlot {
    pub pet_guid: ObjectGuid,
    pub slot: u8,
}

impl ClientPacket for BattlePetSetBattleSlot {
    const OPCODE: ClientOpcodes = ClientOpcodes::BattlePetSetBattleSlot;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        pkt.skip_opcode();
        Ok(Self {
            pet_guid: pkt.read_packed_guid()?,
            slot: pkt.read_uint8()?,
        })
    }
}

/// C++ `WorldPackets::BattlePet::BattlePetSummon`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BattlePetSummon {
    pub pet_guid: ObjectGuid,
}

impl ClientPacket for BattlePetSummon {
    const OPCODE: ClientOpcodes = ClientOpcodes::BattlePetSummon;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        pkt.skip_opcode();
        Ok(Self {
            pet_guid: pkt.read_packed_guid()?,
        })
    }
}

/// C++ `WorldPackets::BattlePet::BattlePetUpdateNotify`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BattlePetUpdateNotify {
    pub pet_guid: ObjectGuid,
}

impl ClientPacket for BattlePetUpdateNotify {
    const OPCODE: ClientOpcodes = ClientOpcodes::BattlePetUpdateNotify;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        pkt.skip_opcode();
        Ok(Self {
            pet_guid: pkt.read_packed_guid()?,
        })
    }
}

pub const MAX_DECLINED_NAME_CASES_LIKE_CPP: usize = 5;

/// C++ `WorldPackets::BattlePet::BattlePetDeletePet`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BattlePetDeletePet {
    pub pet_guid: ObjectGuid,
}

impl BattlePetDeletePet {
    /// Reads C++ `BattlePetDeletePet::Read`.
    ///
    /// The archived C++ opcode table maps `CMSG_BATTLE_PET_DELETE_PET` to the
    /// shared `0xBADD` placeholder. Rust must not register production dispatch
    /// until the real opcode mapping is known.
    pub fn read_like_cpp(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        pkt.skip_opcode();
        Ok(Self {
            pet_guid: pkt.read_packed_guid()?,
        })
    }
}

/// C++ `WorldPackets::BattlePet::CageBattlePet`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CageBattlePet {
    pub pet_guid: ObjectGuid,
}

impl CageBattlePet {
    /// Reads C++ `CageBattlePet::Read`.
    ///
    /// The archived C++ opcode table maps `CMSG_CAGE_BATTLE_PET` to the shared
    /// `0xBADD` placeholder. Rust must not register production dispatch until
    /// the real opcode mapping is known.
    pub fn read_like_cpp(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        pkt.skip_opcode();
        Ok(Self {
            pet_guid: pkt.read_packed_guid()?,
        })
    }
}

/// C++ `DeclinedName`, represented for battle-pet rename packets.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeclinedNamesLikeCpp {
    pub names: [String; MAX_DECLINED_NAME_CASES_LIKE_CPP],
}

/// C++ `WorldPackets::BattlePet::BattlePetModifyName`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BattlePetModifyName {
    pub pet_guid: ObjectGuid,
    pub name: String,
    pub declined_names: Option<DeclinedNamesLikeCpp>,
}

impl BattlePetModifyName {
    /// Reads C++ `BattlePetModifyName::Read`.
    ///
    /// The archived C++ opcode table maps `CMSG_BATTLE_PET_MODIFY_NAME` to the
    /// shared `0xBADD` placeholder. Rust must not register production dispatch
    /// until the real opcode mapping is known.
    pub fn read_like_cpp(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        pkt.skip_opcode();
        let pet_guid = pkt.read_packed_guid()?;
        let name_length = pkt.read_bits(7)? as usize;
        let has_declined_names = pkt.read_bit()?;

        let declined_names = if has_declined_names {
            let mut lengths = [0usize; MAX_DECLINED_NAME_CASES_LIKE_CPP];
            for length in &mut lengths {
                *length = pkt.read_bits(7)? as usize;
            }

            let names_vec: Vec<String> = lengths
                .iter()
                .map(|length| pkt.read_string(*length))
                .collect::<Result<_, _>>()?;
            let names: [String; MAX_DECLINED_NAME_CASES_LIKE_CPP] =
                names_vec.try_into().map_err(|_| PacketError::TooLarge {
                    size: MAX_DECLINED_NAME_CASES_LIKE_CPP + 1,
                })?;
            Some(DeclinedNamesLikeCpp { names })
        } else {
            None
        };

        let name = pkt.read_string(name_length)?;

        Ok(Self {
            pet_guid,
            name,
            declined_names,
        })
    }
}

/// C++ `WorldPackets::BattlePet::QueryBattlePetName`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QueryBattlePetName {
    pub battle_pet_id: ObjectGuid,
    pub unit_guid: ObjectGuid,
}

impl ClientPacket for QueryBattlePetName {
    const OPCODE: ClientOpcodes = ClientOpcodes::QueryBattlePetName;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        pkt.skip_opcode();
        Ok(Self {
            battle_pet_id: pkt.read_packed_guid()?,
            unit_guid: pkt.read_packed_guid()?,
        })
    }
}

/// C++ `WorldPackets::BattlePet::QueryBattlePetNameResponse`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryBattlePetNameResponse {
    pub battle_pet_id: ObjectGuid,
    pub creature_id: i32,
    pub timestamp: i64,
    pub allow: bool,
    pub name: String,
    pub declined_names: Option<DeclinedNamesLikeCpp>,
}

impl QueryBattlePetNameResponse {
    pub fn not_allowed(battle_pet_id: ObjectGuid) -> Self {
        Self {
            battle_pet_id,
            creature_id: 0,
            timestamp: 0,
            allow: false,
            name: String::new(),
            declined_names: None,
        }
    }

    pub fn allowed(
        battle_pet_id: ObjectGuid,
        creature_id: i32,
        timestamp: i64,
        name: String,
        declined_names: Option<DeclinedNamesLikeCpp>,
    ) -> Self {
        Self {
            battle_pet_id,
            creature_id,
            timestamp,
            allow: true,
            name,
            declined_names,
        }
    }
}

impl ServerPacket for QueryBattlePetNameResponse {
    const OPCODE: ServerOpcodes = ServerOpcodes::QueryBattlePetNameResponse;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_packed_guid(&self.battle_pet_id);
        pkt.write_int32(self.creature_id);
        pkt.write_int64(self.timestamp);
        pkt.write_bit(self.allow);
        if self.allow {
            pkt.write_bits(self.name.len() as u32, 8);
            pkt.write_bit(self.declined_names.is_some());

            let declined_names = self.declined_names.as_ref().map(|declined| &declined.names);
            for index in 0..MAX_DECLINED_NAME_CASES_LIKE_CPP {
                let length = declined_names
                    .map(|names| names[index].len())
                    .unwrap_or_default();
                pkt.write_bits(length as u32, 7);
            }

            if let Some(names) = declined_names {
                for name in names {
                    pkt.write_string(name);
                }
            }
            pkt.write_string(&self.name);
        }
        pkt.flush_bits();
    }
}

/// C++ `WorldPackets::BattlePet::BattlePetSetFlags`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BattlePetSetFlags {
    pub pet_guid: ObjectGuid,
    pub flags: u16,
    pub control_type: u8,
}

impl ClientPacket for BattlePetSetFlags {
    const OPCODE: ClientOpcodes = ClientOpcodes::BattlePetSetFlags;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        pkt.skip_opcode();
        let pet_guid = pkt.read_packed_guid()?;
        let flags = pkt.read_uint16()?;
        let control_type = pkt.read_bits(2)? as u8;
        Ok(Self {
            pet_guid,
            flags,
            control_type,
        })
    }
}

/// C++ `WorldPackets::BattlePet::BattlePetClearFanfare`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BattlePetClearFanfare {
    pub pet_guid: ObjectGuid,
}

impl ClientPacket for BattlePetClearFanfare {
    const OPCODE: ClientOpcodes = ClientOpcodes::BattlePetClearFanfare;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        pkt.skip_opcode();
        Ok(Self {
            pet_guid: pkt.read_packed_guid()?,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BattlePetJournalSlot {
    pub pet_guid: ObjectGuid,
    pub collar_id: u32,
    pub index: u8,
    pub locked: bool,
}

impl BattlePetJournalSlot {
    pub fn locked_empty(index: u8) -> Self {
        Self {
            pet_guid: empty_battle_pet_guid_like_cpp(),
            collar_id: 0,
            index,
            locked: true,
        }
    }
}

pub fn empty_battle_pet_guid_like_cpp() -> ObjectGuid {
    ObjectGuid::create_global(HighGuid::BattlePet, 0, 0)
}

/// C++ `WorldPackets::BattlePet::BattlePetOwnerInfo`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BattlePetJournalPetOwnerInfo {
    pub guid: ObjectGuid,
    pub player_virtual_realm: u32,
    pub player_native_realm: u32,
}

/// C++ `WorldPackets::BattlePet::BattlePet`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BattlePetJournalPet {
    pub guid: ObjectGuid,
    pub species: u32,
    pub creature_id: u32,
    pub display_id: u32,
    pub breed: u16,
    pub level: u16,
    pub exp: u16,
    pub flags: u16,
    pub power: u32,
    pub health: u32,
    pub max_health: u32,
    pub speed: u32,
    pub quality: u8,
    pub owner_info: Option<BattlePetJournalPetOwnerInfo>,
    pub name: String,
}

impl BattlePetJournalPet {
    fn write_like_cpp(&self, pkt: &mut WorldPacket) {
        pkt.write_packed_guid(&self.guid);
        pkt.write_uint32(self.species);
        pkt.write_uint32(self.creature_id);
        pkt.write_uint32(self.display_id);
        pkt.write_uint16(self.breed);
        pkt.write_uint16(self.level);
        pkt.write_uint16(self.exp);
        pkt.write_uint16(self.flags);
        pkt.write_uint32(self.power);
        pkt.write_uint32(self.health);
        pkt.write_uint32(self.max_health);
        pkt.write_uint32(self.speed);
        pkt.write_uint8(self.quality);
        pkt.write_bits(self.name.len() as u32, 7);
        pkt.write_bit(self.owner_info.is_some());
        pkt.write_bit(false); // NoRename
        pkt.flush_bits();
        pkt.write_string(&self.name);

        if let Some(owner_info) = self.owner_info {
            pkt.write_packed_guid(&owner_info.guid);
            pkt.write_uint32(owner_info.player_virtual_realm);
            pkt.write_uint32(owner_info.player_native_realm);
        }
    }
}

/// C++ `WorldPackets::BattlePet::BattlePetJournal`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BattlePetJournal {
    pub trap: u16,
    pub has_journal_lock: bool,
    pub slots: Vec<BattlePetJournalSlot>,
    pub pets: Vec<BattlePetJournalPet>,
}

impl BattlePetJournal {
    pub fn empty_with_default_slots(has_journal_lock: bool) -> Self {
        Self {
            trap: 0,
            has_journal_lock,
            slots: (0..3).map(BattlePetJournalSlot::locked_empty).collect(),
            pets: Vec::new(),
        }
    }
}

impl ServerPacket for BattlePetJournal {
    const OPCODE: ServerOpcodes = ServerOpcodes::BattlePetJournal;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_uint16(self.trap);
        pkt.write_uint32(self.slots.len() as u32);
        pkt.write_uint32(self.pets.len() as u32);
        pkt.write_bit(self.has_journal_lock);
        pkt.flush_bits();

        for slot in &self.slots {
            pkt.write_packed_guid(&slot.pet_guid);
            pkt.write_uint32(slot.collar_id);
            pkt.write_uint8(slot.index);
            pkt.write_bit(slot.locked);
            pkt.flush_bits();
        }

        for pet in &self.pets {
            pet.write_like_cpp(pkt);
        }
    }
}

/// C++ `WorldPackets::BattlePet::BattlePetUpdates`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BattlePetUpdates {
    pub pets: Vec<BattlePetJournalPet>,
    pub pet_added: bool,
}

impl ServerPacket for BattlePetUpdates {
    const OPCODE: ServerOpcodes = ServerOpcodes::BattlePetUpdates;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_uint32(self.pets.len() as u32);
        pkt.write_bit(self.pet_added);
        pkt.flush_bits();

        for pet in &self.pets {
            pet.write_like_cpp(pkt);
        }
    }
}

/// C++ `WorldPackets::BattlePet::PetBattleSlotUpdates`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PetBattleSlotUpdates {
    pub slots: Vec<BattlePetJournalSlot>,
    pub auto_slotted: bool,
    pub new_slot: bool,
}

impl ServerPacket for PetBattleSlotUpdates {
    const OPCODE: ServerOpcodes = ServerOpcodes::PetBattleSlotUpdates;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_uint32(self.slots.len() as u32);
        pkt.write_bit(self.new_slot);
        pkt.write_bit(self.auto_slotted);
        pkt.flush_bits();

        for slot in &self.slots {
            pkt.write_packed_guid(&slot.pet_guid);
            pkt.write_uint32(slot.collar_id);
            pkt.write_uint8(slot.index);
            pkt.write_bit(slot.locked);
            pkt.flush_bits();
        }
    }
}

/// Tells the client that the battle pet journal lock has been acquired.
/// Empty packet (opcode only, no payload).
pub struct BattlePetJournalLockAcquired;

impl ServerPacket for BattlePetJournalLockAcquired {
    const OPCODE: ServerOpcodes = ServerOpcodes::BattlePetJournalLockAcquired;

    fn write(&self, _pkt: &mut WorldPacket) {
        // Empty packet — no payload
    }
}

/// Tells the client that the battle pet journal lock was denied.
/// Empty packet (opcode only, no payload).
pub struct BattlePetJournalLockDenied;

impl ServerPacket for BattlePetJournalLockDenied {
    const OPCODE: ServerOpcodes = ServerOpcodes::BattlePetJournalLockDenied;

    fn write(&self, _pkt: &mut WorldPacket) {
        // Empty packet — no payload
    }
}

/// C++ `WorldPackets::BattlePet::BattlePetDeleted`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BattlePetDeleted {
    pub pet_guid: ObjectGuid,
}

impl ServerPacket for BattlePetDeleted {
    const OPCODE: ServerOpcodes = ServerOpcodes::BattlePetDeleted;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_packed_guid(&self.pet_guid);
    }
}

/// C++ `BattlePets::BattlePetError` values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BattlePetErrorCodeLikeCpp {
    CantHaveMorePetsOfType = 3,
    CantHaveMorePets = 4,
    TooHighLevelToUncage = 7,
}

/// C++ `WorldPackets::BattlePet::BattlePetError`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BattlePetError {
    pub result: u8,
    pub creature_id: i32,
}

impl BattlePetError {
    pub fn new(result: BattlePetErrorCodeLikeCpp, creature_id: i32) -> Self {
        Self {
            result: result as u8,
            creature_id,
        }
    }
}

impl ServerPacket for BattlePetError {
    const OPCODE: ServerOpcodes = ServerOpcodes::BattlePetError;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_bits(self.result as u32, 4);
        pkt.write_int32(self.creature_id);
    }
}

// ── DungeonDifficultySet (SMSG 0x26a4) ───────────────────────────────

/// Sets the current dungeon difficulty. Sent BEFORE LoginVerifyWorld.
/// C# sends this via `Player.SendDungeonDifficulty()` during HandlePlayerLogin.
pub struct DungeonDifficultySet {
    pub difficulty_id: i32,
}

impl DungeonDifficultySet {
    /// Normal dungeon difficulty (default for fresh characters).
    pub fn normal() -> Self {
        Self { difficulty_id: 0 }
    }
}

impl ServerPacket for DungeonDifficultySet {
    const OPCODE: ServerOpcodes = ServerOpcodes::SetDungeonDifficulty;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_int32(self.difficulty_id);
    }
}

// ── RaidDifficultySet (SMSG 0x27ad) ──────────────────────────────────

/// Sets the current raid difficulty.
///
/// C++ `WorldPackets::Misc::RaidDifficultySet::Write`:
/// `int32 DifficultyID` followed by `uint8 Legacy`.
pub struct RaidDifficultySet {
    pub difficulty_id: i32,
    pub legacy: bool,
}

impl ServerPacket for RaidDifficultySet {
    const OPCODE: ServerOpcodes = ServerOpcodes::RaidDifficultySet;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_int32(self.difficulty_id);
        pkt.write_uint8(u8::from(self.legacy));
    }
}

// ── DbQueryBulk (CMSG 0x35e5) ─────────────────────────────────────

/// Client request for DB2 records. The server must respond with one
/// [`DBReply`] per requested record, even if the record doesn't exist.
pub struct DbQueryBulk {
    pub table_hash: u32,
    pub queries: Vec<i32>,
}

impl ClientPacket for DbQueryBulk {
    const OPCODE: ClientOpcodes = ClientOpcodes::DbQueryBulk;

    fn read(packet: &mut WorldPacket) -> Result<Self, PacketError> {
        let table_hash = packet.read_uint32()?;
        let count = packet.read_bits(13)? as usize;
        let mut queries = Vec::with_capacity(count.min(8192));
        for _ in 0..count {
            queries.push(packet.read_int32()?);
        }
        Ok(Self {
            table_hash,
            queries,
        })
    }
}

// ── DBReply (SMSG 0x290e) ──────────────────────────────────────────

/// Response to a single [`DbQueryBulk`] record request.
/// Status: 0=NotSet, 1=Valid, 2=RecordRemoved, 3=Invalid.
pub struct DBReply {
    pub table_hash: u32,
    pub record_id: i32,
    pub timestamp: i32,
    pub status: u8,
    pub data: Vec<u8>,
}

impl DBReply {
    /// Reply with Status::Invalid (no data). The client will use its local DB2.
    pub fn not_found(table_hash: u32, record_id: i32) -> Self {
        Self {
            table_hash,
            record_id,
            timestamp: unix_timestamp() as i32,
            status: 3, // HotfixRecord.Status.Invalid
            data: Vec::new(),
        }
    }

    /// Reply with Status::RecordRemoved (2) — record is not on the server;
    /// client should use its local DB2 copy and NOT retry.
    pub fn record_removed(table_hash: u32, record_id: i32) -> Self {
        Self {
            table_hash,
            record_id,
            timestamp: unix_timestamp() as i32,
            status: 2, // HotfixRecord.Status.RecordRemoved
            data: Vec::new(),
        }
    }

    /// Reply with Status::Valid and raw blob data from hotfix_blob table.
    pub fn found(table_hash: u32, record_id: i32, data: Vec<u8>) -> Self {
        Self {
            table_hash,
            record_id,
            timestamp: unix_timestamp() as i32,
            status: 1, // HotfixRecord.Status.Valid
            data,
        }
    }
}

impl ServerPacket for DBReply {
    const OPCODE: ServerOpcodes = ServerOpcodes::DbReply;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_uint32(self.table_hash);
        pkt.write_int32(self.record_id);
        pkt.write_int32(self.timestamp);
        pkt.write_bits(u32::from(self.status), 3);
        // write_uint32 auto-flushes the 3 pending bits
        pkt.write_uint32(self.data.len() as u32);
        if !self.data.is_empty() {
            pkt.write_bytes(&self.data);
        }
    }
}

// ── HotfixRequest (CMSG 0x35e6) ───────────────────────────────────

/// Client request for hotfix data after receiving [`AvailableHotfixes`].
pub struct HotfixRequest {
    pub client_build: u32,
    pub data_build: u32,
    pub hotfixes: Vec<i32>,
}

impl ClientPacket for HotfixRequest {
    const OPCODE: ClientOpcodes = ClientOpcodes::HotfixRequest;

    fn read(packet: &mut WorldPacket) -> Result<Self, PacketError> {
        let client_build = packet.read_uint32()?;
        let data_build = packet.read_uint32()?;
        let count = packet.read_uint32()? as usize;
        let mut hotfixes = Vec::with_capacity(count.min(8192));
        for _ in 0..count {
            hotfixes.push(packet.read_int32()?);
        }
        Ok(Self {
            client_build,
            data_build,
            hotfixes,
        })
    }
}

// ── HotfixConnect (SMSG 0x2911) ───────────────────────────────────

/// One C++ `WorldPackets::Hotfix::HotfixConnect::HotfixData` header.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HotfixConnectData {
    pub id: HotfixId,
    pub table_hash: u32,
    pub record_id: i32,
    pub size: u32,
    pub status: u8,
}

/// Response to [`HotfixRequest`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HotfixConnect {
    pub hotfixes: Vec<HotfixConnectData>,
    pub content: Vec<u8>,
}

impl HotfixConnect {
    pub fn empty() -> Self {
        Self::default()
    }
}

impl ServerPacket for HotfixConnect {
    const OPCODE: ServerOpcodes = ServerOpcodes::HotfixConnect;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_uint32(self.hotfixes.len() as u32);
        for hotfix in &self.hotfixes {
            pkt.write_int32(hotfix.id.push_id);
            pkt.write_uint32(hotfix.id.unique_id);
            pkt.write_uint32(hotfix.table_hash);
            pkt.write_int32(hotfix.record_id);
            pkt.write_uint32(hotfix.size);
            pkt.write_bits(u32::from(hotfix.status), 3);
            pkt.flush_bits();
        }

        pkt.write_uint32(self.content.len() as u32);
        if !self.content.is_empty() {
            pkt.write_bytes(&self.content);
        }
    }
}

// ── MoveSetActiveMover (SMSG 0x2dd5) ───────────────────────────────

/// Tells the client which unit it controls for movement input.
///
/// **Critical**: Without this packet the client's `m_mover` pointer is null.
/// Any camera/movement processing will dereference null → ACCESS_VIOLATION.
///
/// C# format: just a single PackedGuid.
pub struct MoveSetActiveMover {
    pub mover_guid: ObjectGuid,
}

impl ServerPacket for MoveSetActiveMover {
    const OPCODE: ServerOpcodes = ServerOpcodes::MoveSetActiveMover;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_packed_guid(&self.mover_guid);
    }
}

// ── SetSpellModifier (SMSG 0x2c33 / 0x2c34) ───────────────────────

/// Spell modifier data: empty for fresh characters with no talents/auras.
///
/// The same struct is used for both `SetFlatSpellModifier` (0x2c33) and
/// `SetPctSpellModifier` (0x2c34) — only the opcode differs.
///
/// C# format:
/// ```text
/// [i32] Modifiers.Count
/// for each SpellModifierInfo:
///     [u8]  ModIndex
///     [i32] ModifierData.Count
///     for each SpellModifierData:
///         [f32] ModifierValue
///         [u8]  ClassIndex
/// ```
pub struct SetSpellModifier {
    /// Which opcode to use (Flat or Pct).
    opcode: ServerOpcodes,
}

impl SetSpellModifier {
    /// Empty flat spell modifiers (no modifier entries).
    pub fn flat_empty() -> Self {
        Self {
            opcode: ServerOpcodes::SetFlatSpellModifier,
        }
    }

    /// Empty percent spell modifiers (no modifier entries).
    pub fn pct_empty() -> Self {
        Self {
            opcode: ServerOpcodes::SetPctSpellModifier,
        }
    }

    /// Build the packet bytes (custom opcode, can't use the trait const).
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut pkt = WorldPacket::new_server(self.opcode);
        pkt.write_int32(0); // Modifiers.Count = 0
        pkt.data().to_vec()
    }
}

// ── SetProficiency (SMSG 0x2735) ───────────────────────────────────

/// Tells the client what weapon/armor types the player can use.
///
/// C# format:
/// ```text
/// [i32] ProficiencyMask  (bitmask of sub-classes)
/// [u8]  ProficiencyClass (ItemClass enum: 2=Weapon, 4=Armor)
/// ```
pub struct SetProficiency {
    pub proficiency_mask: u32,
    pub proficiency_class: u8,
}

impl ServerPacket for SetProficiency {
    const OPCODE: ServerOpcodes = ServerOpcodes::SetProficiency;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_uint32(self.proficiency_mask);
        pkt.write_uint8(self.proficiency_class);
    }
}

impl SetProficiency {
    /// Default weapon proficiency for a given class.
    ///
    /// Masks from C# InitDataForForm() / proficiency spell effects.
    /// Class 2 = Weapon (ItemClass.Weapon).
    pub fn default_weapons(class_id: u8) -> Self {
        // Weapon subclass bit positions (1 << subclass):
        //  0=Axe1H     0x00001   7=Sword1H   0x00080   15=Dagger    0x08000
        //  1=Axe2H     0x00002   8=Sword2H   0x00100   16=Thrown    0x10000
        //  2=Bow       0x00004  10=Staff     0x00400   18=Crossbow  0x40000
        //  3=Gun       0x00008  13=Fist      0x02000   19=Wand      0x80000
        //  4=Mace1H    0x00010
        //  5=Mace2H    0x00020
        //  6=Polearm   0x00040
        let mask = match class_id {
            1 => 0x0005_A5FF, // Warrior: Axe12,Bow,Gun,Mace12,Polearm,Sword12,Staff,Fist,Dagger,Thrown,Xbow
            2 => 0x0000_01F3, // Paladin: Axe12,Mace12,Polearm,Sword12
            3 => 0x0005_A5CF, // Hunter: Axe12,Bow,Gun,Polearm,Sword12,Staff,Fist,Dagger,Thrown,Xbow
            4 => 0x0005_A09C, // Rogue: Bow,Gun,Mace1H,Sword1H,Fist,Dagger,Thrown,Xbow
            5 => 0x0008_8410, // Priest: Mace1H,Staff,Dagger,Wand
            6 => 0x0000_01F3, // DK: Axe12,Mace12,Polearm,Sword12
            7 => 0x0000_A433, // Shaman: Axe12,Mace12,Staff,Fist,Dagger
            8 => 0x0008_8480, // Mage: Sword1H,Staff,Dagger,Wand
            9 => 0x0008_8480, // Warlock: Sword1H,Staff,Dagger,Wand
            11 => 0x0000_A470, // Druid: Mace12,Polearm,Staff,Fist,Dagger
            _ => 0x0000_2000, // Fists only
        };
        Self {
            proficiency_mask: mask,
            proficiency_class: 2, // Weapon
        }
    }

    /// Default armor proficiency for a given class.
    ///
    /// Class 4 = Armor (ItemClass.Armor).
    /// Subclass bit positions: Cloth=1(0x02), Leather=2(0x04), Mail=3(0x08),
    /// Plate=4(0x10), Shield=6(0x40).
    pub fn default_armor(class_id: u8) -> Self {
        let mask = match class_id {
            1 => 0x5E,  // Warrior: Cloth+Leather+Mail+Plate+Shield
            2 => 0x5E,  // Paladin: Cloth+Leather+Mail+Plate+Shield
            3 => 0x0E,  // Hunter: Cloth+Leather+Mail
            4 => 0x06,  // Rogue: Cloth+Leather
            5 => 0x02,  // Priest: Cloth
            6 => 0x1E,  // DK: Cloth+Leather+Mail+Plate
            7 => 0x4E,  // Shaman: Cloth+Leather+Mail+Shield
            8 => 0x02,  // Mage: Cloth
            9 => 0x02,  // Warlock: Cloth
            11 => 0x06, // Druid: Cloth+Leather
            _ => 0x02,  // Cloth
        };
        Self {
            proficiency_mask: mask,
            proficiency_class: 4, // Armor
        }
    }
}

// ── SuspendToken (SMSG 0x25a8) ───────────────────────────────────────

/// Sent on the instance connection after TransferPending.
/// Tells the client to pause movement processing during map transfer.
/// C# ref: MovementPackets.SuspendToken (ConnectionType.Instance)
pub struct SuspendToken {
    /// Movement counter (sequence index). Send 1 for simple teleports.
    pub sequence_index: u32,
    /// 1 = Normal teleport, 2 = Seamless teleport.
    pub reason: u32,
}

impl ServerPacket for SuspendToken {
    const OPCODE: ServerOpcodes = ServerOpcodes::SuspendToken;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_uint32(self.sequence_index);
        pkt.write_bits(self.reason, 2);
        pkt.flush_bits();
    }
}

// ── ResumeToken (SMSG 0x25a9) ────────────────────────────────────────

/// Sent after WorldPortResponse to resume movement processing.
/// C# ref: MovementPackets.ResumeToken (ConnectionType.Instance)
pub struct ResumeToken {
    pub sequence_index: u32,
    /// 1 = Normal, 2 = Seamless.
    pub reason: u32,
}

impl ServerPacket for ResumeToken {
    const OPCODE: ServerOpcodes = ServerOpcodes::ResumeToken;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_uint32(self.sequence_index);
        pkt.write_bits(self.reason, 2);
        pkt.flush_bits();
    }
}

// ── NewWorld (SMSG 0x2594) ────────────────────────────────────────────

/// Sent after WorldPortResponse to place the player in the new world.
/// C# ref: MovementPackets.NewWorld
pub struct NewWorld {
    pub map_id: u32,
    pub pos: wow_core::Position,
    /// 0 = Normal teleport, 1 = Seamless.
    pub reason: u32,
}

impl ServerPacket for NewWorld {
    const OPCODE: ServerOpcodes = ServerOpcodes::NewWorld;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_uint32(self.map_id);
        // TeleportLocation: Pos (XYZO) + two unused int32 fields (-1, -1)
        pkt.write_float(self.pos.x);
        pkt.write_float(self.pos.y);
        pkt.write_float(self.pos.z);
        pkt.write_float(self.pos.orientation);
        pkt.write_int32(-1); // Unused901_1
        pkt.write_int32(-1); // Unused901_2
        pkt.write_uint32(self.reason);
        // MovementOffset (all zeros)
        pkt.write_float(0.0);
        pkt.write_float(0.0);
        pkt.write_float(0.0);
    }
}

// ── TransferAborted (SMSG 0x2703) ───────────────────────────────────

/// C++ `WorldPackets::Movement::TransferAborted`.
pub struct TransferAborted {
    pub map_id: u32,
    pub arg: u8,
    pub map_difficulty_x_condition_id: i32,
    pub transfer_abort: u32,
}

impl ServerPacket for TransferAborted {
    const OPCODE: ServerOpcodes = ServerOpcodes::TransferAborted;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_uint32(self.map_id);
        pkt.write_uint8(self.arg);
        pkt.write_int32(self.map_difficulty_x_condition_id);
        pkt.write_bits(self.transfer_abort, 6);
        pkt.flush_bits();
    }
}

// ── LogoutRequest (CMSG 0x34d6) ─────────────────────────────────────

/// Client requests to log out.
pub struct LogoutRequest {
    pub idle_logout: bool,
}

impl ClientPacket for LogoutRequest {
    const OPCODE: ClientOpcodes = ClientOpcodes::LogoutRequest;

    fn read(packet: &mut WorldPacket) -> Result<Self, PacketError> {
        let idle_logout = packet.read_bit()?;
        Ok(Self { idle_logout })
    }
}

/// C++ `WorldPackets::Misc::RepopRequest`: one `CheckInstance` bit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RepopRequest {
    pub check_instance: bool,
}

impl ClientPacket for RepopRequest {
    const OPCODE: ClientOpcodes = ClientOpcodes::RepopRequest;

    fn read(packet: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self {
            check_instance: packet.read_bit()?,
        })
    }
}

/// C++ `WorldPackets::Misc::ReclaimCorpse`: full raw corpse GUID.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReclaimCorpse {
    pub corpse_guid: ObjectGuid,
}

impl ClientPacket for ReclaimCorpse {
    const OPCODE: ClientOpcodes = ClientOpcodes::ReclaimCorpse;

    fn read(packet: &mut WorldPacket) -> Result<Self, PacketError> {
        let raw = packet.read_bytes(16)?;
        let mut guid = [0u8; 16];
        guid.copy_from_slice(&raw);
        Ok(Self {
            corpse_guid: ObjectGuid::from_raw_bytes(&guid),
        })
    }
}

// ── LogoutCancel (CMSG 0x34d8) ──────────────────────────────────────

/// Client cancels a pending logout.
pub struct LogoutCancel;

impl ClientPacket for LogoutCancel {
    const OPCODE: ClientOpcodes = ClientOpcodes::LogoutCancel;

    fn read(_packet: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self)
    }
}

// ── LogoutResponse (SMSG 0x2683) ────────────────────────────────────

/// Server responds to a logout request.
pub struct LogoutResponse {
    pub logout_result: i32,
    pub instant: bool,
}

impl LogoutResponse {
    /// Successful instant logout.
    pub fn instant_ok() -> Self {
        Self {
            logout_result: 0,
            instant: true,
        }
    }

    /// Successful delayed logout (20s timer).
    pub fn delayed_ok() -> Self {
        Self {
            logout_result: 0,
            instant: false,
        }
    }
}

impl ServerPacket for LogoutResponse {
    const OPCODE: ServerOpcodes = ServerOpcodes::LogoutResponse;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_int32(self.logout_result);
        pkt.write_bit(self.instant);
        pkt.flush_bits();
    }
}

// ── TransferPending (SMSG 0x25cd) ────────────────────────────────────

/// Sent when the player is being teleported to a new map.
/// C# ref: MovePackets.cs - TransferPending
pub struct TransferPending {
    pub map_id: u32,
    pub old_map_position: wow_core::Position,
    pub ship: Option<ShipTransferPending>,
    pub transfer_spell_id: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct ShipTransferPending {
    pub id: u32,
    pub origin_map_id: u32,
}

impl ServerPacket for TransferPending {
    const OPCODE: ServerOpcodes = ServerOpcodes::TransferPending;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_uint32(self.map_id);
        pkt.write_float(self.old_map_position.x);
        pkt.write_float(self.old_map_position.y);
        pkt.write_float(self.old_map_position.z);
        pkt.write_bit(self.ship.is_some());
        pkt.write_bit(self.transfer_spell_id.is_some());
        pkt.flush_bits();

        if let Some(ref ship) = self.ship {
            pkt.write_uint32(ship.id);
            pkt.write_uint32(ship.origin_map_id);
        }

        if let Some(spell_id) = self.transfer_spell_id {
            pkt.write_uint32(spell_id);
        }
    }
}

// ── LogoutComplete (SMSG 0x2684) ────────────────────────────────────

/// Server tells client logout is complete — return to character select.
pub struct LogoutComplete;

impl ServerPacket for LogoutComplete {
    const OPCODE: ServerOpcodes = ServerOpcodes::LogoutComplete;

    fn write(&self, _pkt: &mut WorldPacket) {}
}

// ── LogoutCancelAck (SMSG 0x2685) ───────────────────────────────────

/// Server acknowledges logout cancellation.
pub struct LogoutCancelAck;

impl ServerPacket for LogoutCancelAck {
    const OPCODE: ServerOpcodes = ServerOpcodes::LogoutCancelAck;

    fn write(&self, _pkt: &mut WorldPacket) {}
}

// ── Helper ──────────────────────────────────────────────────────────

fn unix_timestamp() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

fn wow_time_packed_from_unix_seconds(unix_seconds: i64) -> u32 {
    let days = unix_seconds.div_euclid(86_400);
    let seconds_of_day = unix_seconds.rem_euclid(86_400);
    let (year, month, month_day) = civil_from_days(days);
    let week_day = (days + 4).rem_euclid(7) as u32;
    let hour = (seconds_of_day / 3_600) as u32;
    let minute = ((seconds_of_day % 3_600) / 60) as u32;

    let year_field = ((year - 2000).rem_euclid(100) as u32) & 0x1f;
    let month_field = (month - 1) & 0x0f;
    let month_day_field = (month_day - 1) & 0x3f;

    (year_field << 24)
        | (month_field << 20)
        | (month_day_field << 14)
        | ((week_day & 0x07) << 11)
        | ((hour & 0x1f) << 6)
        | (minute & 0x3f)
}

fn civil_from_days(days_since_unix_epoch: i64) -> (i32, u32, u32) {
    let z = days_since_unix_epoch + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = mp + if mp < 10 { 3 } else { -9 };
    let year = y + i64::from(month <= 2);

    (year as i32, month as u32, day as u32)
}

// ── ShowTradeSkill (client → server) ────────────────────────────────────────
// Sent when the player opens their own profession window from the spellbook,
// or when clicking a trade skill link to view another player's profession.

/// Parsed `CMSG_SHOW_TRADE_SKILL` (0x36CA).
#[derive(Debug, Clone)]
pub struct ShowTradeSkill {
    pub caster_guid: wow_core::ObjectGuid,
    pub spell_id: i32,
    pub skill_id: u16,
}

impl crate::ClientPacket for ShowTradeSkill {
    const OPCODE: wow_constants::ClientOpcodes = wow_constants::ClientOpcodes::ShowTradeSkill;

    fn read(packet: &mut crate::WorldPacket) -> Result<Self, crate::world_packet::PacketError> {
        let caster_guid = packet.read_packed_guid()?;
        let spell_id = packet.read_int32()?;
        let skill_id = packet.read_int32()? as u16;
        Ok(Self {
            caster_guid,
            spell_id,
            skill_id,
        })
    }
}

// ── ShowTradeSkillResponse (server → client) ─────────────────────────────────
// Response to ShowTradeSkill — tells the client which recipes the player knows.

/// `SMSG_SHOW_TRADE_SKILL_RESPONSE` (0x2774).
///
/// C# struct: CasterGUID, SpellId, SkillLineId, SkillRank, SkillMaxRank,
///            SkillLineIDs[], SkillRanks[], SkillMaxRanks[], KnownAbilitySpellIDs[]
pub struct ShowTradeSkillResponse {
    pub caster_guid: wow_core::ObjectGuid,
    pub spell_id: i32,
    pub skill_line_id: u16,
    pub skill_rank: i32,
    pub skill_max_rank: i32,
    /// Known recipe/ability spell IDs for this profession.
    pub known_ability_spell_ids: Vec<i32>,
}

impl ShowTradeSkillResponse {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = crate::WorldPacket::new_server(ServerOpcodes::ShowTradeSkillResponse);
        buf.write_packed_guid(&self.caster_guid);
        buf.write_int32(self.spell_id);

        // SkillLineIDs[] — secondary lines (default [0])
        buf.write_int32(1);
        // SkillRanks[] — secondary ranks (default [0])
        buf.write_int32(1);
        // SkillMaxRanks[] — secondary max ranks (default [0])
        buf.write_int32(1);
        // KnownAbilitySpellIDs count
        buf.write_int32(self.known_ability_spell_ids.len() as i32);

        // secondary lists (each 1 entry = 0)
        buf.write_int32(0); // SkillLineIDs[0]
        buf.write_int32(0); // SkillRanks[0]
        buf.write_int32(0); // SkillMaxRanks[0]

        buf.write_int32(self.skill_line_id as i32); // SkillLineId
        buf.write_int32(self.skill_rank); // SkillRank
        buf.write_int32(self.skill_max_rank); // SkillMaxRank

        for spell_id in &self.known_ability_spell_ids {
            buf.write_int32(*spell_id);
        }

        buf.into_data()
    }
}

// ── PhaseShiftChange (SMSG 0x2578) ───────────────────────────────────────────
//
// Sent after AddToMap so the client knows which phases the player is in.
// Without this, the client may not render any world objects.
//
// C++ ref: `PhasingHandler::SendToPlayer` + `MiscPackets.cpp::PhaseShiftChange::Write`.
// Format:
//   WritePackedGuid(Client)         — player GUID
//   Phaseshift.Write():
//     WriteUInt32(PhaseShiftFlags)  — 0x08 = Unphased (default, no special phase)
//     WriteUInt32(Phases.Count)     — 0
//     WritePackedGuid(PersonalGUID) — empty
//   WriteUInt32(VisibleMapIDs * 2)  — size in bytes, followed by u16 ids
//   WriteUInt32(PreloadMapIDs * 2)  — size in bytes, followed by u16 ids
//   WriteUInt32(UiMapPhaseIDs * 2)  — size in bytes, followed by u16 ids

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PhaseShiftDataPhase {
    pub phase_flags: u16,
    pub id: u16,
}

pub struct PhaseShiftChange {
    pub player_guid: ObjectGuid,
    pub phase_shift_flags: u32,
    pub phases: Vec<PhaseShiftDataPhase>,
    pub personal_guid: ObjectGuid,
    pub visible_map_ids: Vec<u16>,
    pub preload_map_ids: Vec<u16>,
    pub ui_map_phase_ids: Vec<u16>,
}

impl PhaseShiftChange {
    pub fn default_for(player_guid: ObjectGuid) -> Self {
        Self {
            player_guid,
            phase_shift_flags: 0x08,
            phases: Vec::new(),
            personal_guid: ObjectGuid::EMPTY,
            visible_map_ids: Vec::new(),
            preload_map_ids: Vec::new(),
            ui_map_phase_ids: Vec::new(),
        }
    }

    pub fn with_visible_map_ids(player_guid: ObjectGuid, visible_map_ids: Vec<u16>) -> Self {
        Self {
            visible_map_ids,
            ..Self::default_for(player_guid)
        }
    }
}

impl ServerPacket for PhaseShiftChange {
    const OPCODE: ServerOpcodes = ServerOpcodes::PhaseShiftChange;

    fn write(&self, pkt: &mut crate::WorldPacket) {
        // Client GUID
        pkt.write_packed_guid(&self.player_guid);
        // Phaseshift block: flags + phases count + personal guid
        pkt.write_uint32(self.phase_shift_flags);
        pkt.write_uint32(self.phases.len() as u32);
        pkt.write_packed_guid(&self.personal_guid);
        for phase in &self.phases {
            pkt.write_uint16(phase.phase_flags);
            pkt.write_uint16(phase.id);
        }
        // VisibleMapIDs size in bytes
        pkt.write_uint32((self.visible_map_ids.len() * 2) as u32);
        for visible_map_id in &self.visible_map_ids {
            pkt.write_uint16(*visible_map_id);
        }
        // PreloadMapIDs size in bytes
        pkt.write_uint32((self.preload_map_ids.len() * 2) as u32);
        for preload_map_id in &self.preload_map_ids {
            pkt.write_uint16(*preload_map_id);
        }
        // UiMapPhaseIDs size in bytes
        pkt.write_uint32((self.ui_map_phase_ids.len() * 2) as u32);
        for ui_map_phase_id in &self.ui_map_phase_ids {
            pkt.write_uint16(*ui_map_phase_id);
        }
    }
}

// ── Vendor packets ───────────────────────────────────────────────────────────
//
// C# ref: NpcPackets.cs — VendorInventory, BuyItem, BuySucceeded, BuyFailed, SellItem

/// One item in the vendor's inventory list.
/// C#: VendorItemPkt
#[derive(Debug, Clone)]
pub struct VendorItem {
    pub muid: i32, // slot/muid index
    pub item_id: i32,
    pub item_type: i32, // 1 = item, 2 = currency
    pub quantity: i32,  // max stack on vendor (-1 = unlimited)
    pub price: u64,     // buy price (copper)
    pub durability: i32,
    pub stack_count: i32, // VendorStackCount from item_sparse
    pub extended_cost: i32,
    pub player_condition_failed: i32,
    pub locked: bool,
    pub do_not_filter: bool,
    pub refundable: bool,
}

/// SMSG_VENDOR_INVENTORY — list of items a vendor is selling.
/// C#: VendorInventory
pub struct VendorInventory {
    pub vendor_guid: ObjectGuid,
    pub reason: u8, // 0 = ok, non-0 = error (no items etc)
    pub items: Vec<VendorItem>,
}

impl ServerPacket for VendorInventory {
    const OPCODE: ServerOpcodes = ServerOpcodes::VendorInventory;

    fn write(&self, pkt: &mut crate::WorldPacket) {
        pkt.write_packed_guid(&self.vendor_guid);
        pkt.write_uint8(self.reason);
        pkt.write_int32(self.items.len() as i32);

        for (i, item) in self.items.iter().enumerate() {
            pkt.write_uint64(item.price);
            pkt.write_int32(item.muid);
            pkt.write_int32(item.item_type);
            pkt.write_int32(item.durability);
            pkt.write_int32(item.stack_count);
            pkt.write_int32(item.quantity);
            pkt.write_int32(item.extended_cost);
            pkt.write_int32(item.player_condition_failed);
            // 3 bits: Locked, DoNotFilterOnVendor, Refundable
            pkt.write_bit(item.locked);
            pkt.write_bit(item.do_not_filter);
            pkt.write_bit(item.refundable);
            pkt.flush_bits();
            // ItemInstance inline:
            //   ItemID (i32), RandomPropertiesSeed (i32), RandomPropertiesID (i32)
            //   bit(ItemBonus != null) = false, FlushBits
            //   ItemModList: WriteBits(0, 6) + FlushBits  (no mods)
            pkt.write_int32(item.item_id);
            pkt.write_int32(0i32); // RandomPropertiesSeed
            pkt.write_int32(0i32); // RandomPropertiesID
            pkt.write_bit(false); // has ItemBonus = false
            pkt.flush_bits();
            pkt.write_bits(0u32, 6); // ItemModList count = 0
            pkt.flush_bits();
            // no ItemMod entries, no ItemBonus
            let _ = i; // suppress unused
        }
    }
}

/// CMSG_BUY_ITEM — client wants to buy an item from a vendor.
/// C#: BuyItem
#[derive(Debug)]
pub struct BuyItem {
    pub vendor_guid: ObjectGuid,
    pub container_guid: ObjectGuid,
    pub quantity: i32,
    pub muid: i32,
    pub slot: i32,
    pub item_type: i32,
    pub item_id: i32,
}

impl ClientPacket for BuyItem {
    const OPCODE: wow_constants::ClientOpcodes = wow_constants::ClientOpcodes::BuyItem;

    fn read(pkt: &mut crate::WorldPacket) -> Result<Self, PacketError> {
        let vendor_guid = pkt.read_packed_guid()?;
        let container_guid = pkt.read_packed_guid()?;
        let quantity = pkt.read_int32()?;
        let muid = pkt.read_int32()?;
        let slot = pkt.read_int32()?;
        let item_type = pkt.read_int32()?;
        // ItemInstance.Read: ItemID, RandomPropertiesSeed, RandomPropertiesID, bit(hasBonus), FlushBits, ItemModList
        let item_id = pkt.read_int32()?;
        let _seed = pkt.read_int32()?;
        let _rand_prop = pkt.read_int32()?;
        let has_bonus = pkt.read_bit()?;
        pkt.reset_bits();
        let mod_count = pkt.read_bits(6)? as u32;
        for _ in 0..mod_count {
            let _val = pkt.read_int32()?;
            let _ty = pkt.read_uint8()?;
        }
        if has_bonus {
            // ItemBonuses: Context (u8) + BonusListIDs count + entries
            let _ctx = pkt.read_uint8()?;
            let bonus_count = pkt.read_uint32()?;
            for _ in 0..bonus_count {
                let _bid = pkt.read_uint16()?;
            }
        }
        Ok(Self {
            vendor_guid,
            container_guid,
            quantity,
            muid,
            slot,
            item_type,
            item_id,
        })
    }
}

/// CMSG_BUY_BACK_ITEM — client buys back an item from a vendor buyback slot.
/// C++: WorldPackets::Item::BuyBackItem
#[derive(Debug)]
pub struct BuyBackItem {
    pub vendor_guid: ObjectGuid,
    pub slot: u32,
}

impl ClientPacket for BuyBackItem {
    const OPCODE: wow_constants::ClientOpcodes = wow_constants::ClientOpcodes::BuyBackItem;

    fn read(pkt: &mut crate::WorldPacket) -> Result<Self, PacketError> {
        let vendor_guid = pkt.read_packed_guid()?;
        let slot = pkt.read_uint32()?;
        Ok(Self { vendor_guid, slot })
    }
}

/// SMSG_BUY_SUCCEEDED — item bought successfully.
/// C#: BuySucceeded
pub struct BuySucceeded {
    pub vendor_guid: ObjectGuid,
    pub muid: i32,
    pub new_quantity: i32,
    pub quantity_bought: i32,
}

impl ServerPacket for BuySucceeded {
    const OPCODE: ServerOpcodes = ServerOpcodes::BuySucceeded;

    fn write(&self, pkt: &mut crate::WorldPacket) {
        pkt.write_packed_guid(&self.vendor_guid);
        pkt.write_int32(self.muid);
        pkt.write_int32(self.new_quantity);
        pkt.write_int32(self.quantity_bought);
    }
}

/// SMSG_BUY_FAILED — buy failed with reason code.
/// C#: BuyFailed
pub struct BuyFailed {
    pub vendor_guid: ObjectGuid,
    pub muid: i32,
    pub reason: BuyResult,
}

impl ServerPacket for BuyFailed {
    const OPCODE: ServerOpcodes = ServerOpcodes::BuyFailed;

    fn write(&self, pkt: &mut crate::WorldPacket) {
        pkt.write_packed_guid(&self.vendor_guid);
        pkt.write_int32(self.muid);
        pkt.write_uint8(self.reason as u8);
    }
}

/// CMSG_SELL_ITEM — client wants to sell an item to a vendor.
/// C#: SellItem
#[derive(Debug)]
pub struct SellItem {
    pub vendor_guid: ObjectGuid,
    pub item_guid: ObjectGuid,
    pub amount: i32,
}

impl ClientPacket for SellItem {
    const OPCODE: wow_constants::ClientOpcodes = wow_constants::ClientOpcodes::SellItem;

    fn read(pkt: &mut crate::WorldPacket) -> Result<Self, PacketError> {
        let vendor_guid = pkt.read_packed_guid()?;
        let item_guid = pkt.read_packed_guid()?;
        let amount = pkt.read_int32()?;
        Ok(Self {
            vendor_guid,
            item_guid,
            amount,
        })
    }
}

/// CMSG_REPAIR_ITEM — client repairs one item or all items at a repair NPC.
/// C++: WorldPackets::Item::RepairItem.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RepairItem {
    pub npc_guid: ObjectGuid,
    pub item_guid: ObjectGuid,
    pub use_guild_bank: bool,
}

impl ClientPacket for RepairItem {
    const OPCODE: wow_constants::ClientOpcodes = wow_constants::ClientOpcodes::RepairItem;

    fn read(pkt: &mut crate::WorldPacket) -> Result<Self, PacketError> {
        let npc_guid = pkt.read_packed_guid()?;
        let item_guid = pkt.read_packed_guid()?;
        let use_guild_bank = pkt.read_bit()?;
        Ok(Self {
            npc_guid,
            item_guid,
            use_guild_bank,
        })
    }
}

/// C++ `WorldPackets::NPC::SpiritHealerActivate`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpiritHealerActivate {
    pub healer: ObjectGuid,
}

impl ClientPacket for SpiritHealerActivate {
    const OPCODE: wow_constants::ClientOpcodes = wow_constants::ClientOpcodes::SpiritHealerActivate;

    fn read(pkt: &mut crate::WorldPacket) -> Result<Self, PacketError> {
        Ok(Self {
            healer: pkt.read_packed_guid()?,
        })
    }
}

/// C++ `WorldPackets::Battleground::AreaSpiritHealerQuery`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AreaSpiritHealerQuery {
    pub healer_guid: ObjectGuid,
}

impl ClientPacket for AreaSpiritHealerQuery {
    const OPCODE: wow_constants::ClientOpcodes =
        wow_constants::ClientOpcodes::AreaSpiritHealerQuery;

    fn read(pkt: &mut crate::WorldPacket) -> Result<Self, PacketError> {
        Ok(Self {
            healer_guid: pkt.read_packed_guid()?,
        })
    }
}

/// C++ `WorldPackets::Battleground::AreaSpiritHealerQueue`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AreaSpiritHealerQueue {
    pub healer_guid: ObjectGuid,
}

impl ClientPacket for AreaSpiritHealerQueue {
    const OPCODE: wow_constants::ClientOpcodes =
        wow_constants::ClientOpcodes::AreaSpiritHealerQueue;

    fn read(pkt: &mut crate::WorldPacket) -> Result<Self, PacketError> {
        Ok(Self {
            healer_guid: pkt.read_packed_guid()?,
        })
    }
}

/// C++ `WorldPackets::Battleground::AreaSpiritHealerTime`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AreaSpiritHealerTime {
    pub healer_guid: ObjectGuid,
    pub time_left_ms: i32,
}

impl ServerPacket for AreaSpiritHealerTime {
    const OPCODE: ServerOpcodes = ServerOpcodes::AreaSpiritHealerTime;

    fn write(&self, pkt: &mut crate::WorldPacket) {
        pkt.write_packed_guid(&self.healer_guid);
        pkt.write_int32(self.time_left_ms);
    }
}

/// C++ `WorldPackets::Battleground::HearthAndResurrect`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct HearthAndResurrect;

impl ClientPacket for HearthAndResurrect {
    const OPCODE: ClientOpcodes = ClientOpcodes::HearthAndResurrect;

    fn read(_pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self)
    }
}

/// C++ `WorldPackets::Battleground::BattlefieldLeave`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BattlefieldLeave;

impl ClientPacket for BattlefieldLeave {
    const OPCODE: ClientOpcodes = ClientOpcodes::BattlefieldLeave;

    fn read(_pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self)
    }
}

/// C++ `WorldPackets::Battleground::BattlefieldPort`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BattlefieldPort {
    pub ticket: LfgRideTicket,
    pub accepted_invite: bool,
}

impl ClientPacket for BattlefieldPort {
    const OPCODE: ClientOpcodes = ClientOpcodes::BattlefieldPort;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        let ticket = LfgRideTicket::read_like_cpp(pkt)?;
        let accepted_invite = pkt.read_bit()?;
        pkt.reset_bits();
        Ok(Self {
            ticket,
            accepted_invite,
        })
    }
}

/// C++ `WorldPackets::Battleground::BattlefieldListRequest`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BattlefieldListRequest {
    pub list_id: i32,
}

impl ClientPacket for BattlefieldListRequest {
    const OPCODE: ClientOpcodes = ClientOpcodes::BattlefieldList;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self {
            list_id: pkt.read_int32()?,
        })
    }
}

/// C++ `WorldPackets::Battleground::BattlemasterJoin`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct BattlemasterJoin {
    pub queue_ids: Vec<u64>,
    pub roles: u8,
    pub blacklist_map: [i32; 2],
}

impl ClientPacket for BattlemasterJoin {
    const OPCODE: ClientOpcodes = ClientOpcodes::BattlemasterJoin;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        let queue_count = pkt.read_uint32()? as usize;
        let roles = pkt.read_uint8()?;
        let blacklist_map = [pkt.read_int32()?, pkt.read_int32()?];
        let mut queue_ids = Vec::with_capacity(queue_count);
        for _ in 0..queue_count {
            queue_ids.push(pkt.read_uint64()?);
        }

        Ok(Self {
            queue_ids,
            roles,
            blacklist_map,
        })
    }
}

/// C++ `WorldPackets::Battleground::BattlemasterJoinArena`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BattlemasterJoinArena {
    pub team_size_index: u8,
    pub roles: u8,
}

impl ClientPacket for BattlemasterJoinArena {
    const OPCODE: ClientOpcodes = ClientOpcodes::BattlemasterJoinArena;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self {
            team_size_index: pkt.read_uint8()?,
            roles: pkt.read_uint8()?,
        })
    }
}

/// C++ `WorldPackets::Battleground::BattlemasterJoinSkirmish`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BattlemasterJoinSkirmish {
    pub bg_type_id: u32,
    pub bracket_id: u32,
    pub as_group: u8,
    pub is_rated: u8,
}

impl ClientPacket for BattlemasterJoinSkirmish {
    const OPCODE: ClientOpcodes = ClientOpcodes::BattlemasterJoinSkirmish;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self {
            bg_type_id: pkt.read_uint32()?,
            bracket_id: pkt.read_uint32()?,
            as_group: pkt.read_uint8()?,
            is_rated: pkt.read_uint8()?,
        })
    }
}

/// C++ `WorldPackets::Battleground::AcceptWargameInvite`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AcceptWargameInvite {
    pub inviter_name: String,
}

impl ClientPacket for AcceptWargameInvite {
    const OPCODE: ClientOpcodes = ClientOpcodes::AcceptWargameInvite;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self {
            inviter_name: pkt.read_cstring()?,
        })
    }
}

/// C++ `WorldPackets::Misc::ResurrectResponse`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResurrectResponse {
    pub resurrecter: ObjectGuid,
    /// C++: Accept = 0, Decline = 1, Timeout = 2.
    pub response: u32,
}

impl ClientPacket for ResurrectResponse {
    const OPCODE: ClientOpcodes = ClientOpcodes::ResurrectResponse;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self {
            resurrecter: pkt.read_packed_guid()?,
            response: pkt.read_uint32()?,
        })
    }
}

/// C++ `WorldPackets::NPC::RequestStabledPets`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RequestStabledPets {
    pub stable_master: ObjectGuid,
}

impl ClientPacket for RequestStabledPets {
    const OPCODE: wow_constants::ClientOpcodes = wow_constants::ClientOpcodes::RequestStabledPets;

    fn read(pkt: &mut crate::WorldPacket) -> Result<Self, PacketError> {
        Ok(Self {
            stable_master: pkt.read_packed_guid()?,
        })
    }
}

/// SMSG_SELL_RESPONSE — result of a sell operation.
/// C#: SellResponse
pub struct SellResponse {
    pub vendor_guid: ObjectGuid,
    pub item_guids: Vec<ObjectGuid>,
    pub reason: i32,
}

impl ServerPacket for SellResponse {
    const OPCODE: ServerOpcodes = ServerOpcodes::SellResponse;

    fn write(&self, pkt: &mut crate::WorldPacket) {
        pkt.write_packed_guid(&self.vendor_guid);
        pkt.write_uint32(self.item_guids.len() as u32);
        pkt.write_int32(self.reason);
        for item_guid in &self.item_guids {
            pkt.write_packed_guid(item_guid);
        }
    }
}

impl SellResponse {
    pub fn error(vendor_guid: ObjectGuid, item_guid: ObjectGuid, reason: SellResult) -> Self {
        Self {
            vendor_guid,
            item_guids: vec![item_guid],
            reason: reason as i32,
        }
    }

    pub fn success(vendor_guid: ObjectGuid, item_guid: ObjectGuid) -> Self {
        Self {
            vendor_guid,
            item_guids: vec![item_guid],
            reason: 0,
        }
    }
}

// ── PlayedTime (SMSG 0x26d5) ─────────────────────────────────────────────────

/// Server response to CMSG_REQUEST_PLAYED_TIME.
///
/// C# ref: `MiscHandler.HandlePlayedTime` → `PlayedTime` packet.
/// Fields: TotalTime (u32), LevelTime (u32), TriggerEvent (bool).
pub struct PlayedTime {
    /// Total time the character has been played (seconds).
    pub total_time: u32,
    /// Time played at the current level (seconds).
    pub level_time: u32,
    /// Mirror of the client's TriggerScriptEvent flag.
    pub trigger_event: bool,
}

impl ServerPacket for PlayedTime {
    const OPCODE: ServerOpcodes = ServerOpcodes::PlayedTime;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_uint32(self.total_time);
        pkt.write_uint32(self.level_time);
        pkt.write_bit(self.trigger_event);
        pkt.flush_bits();
    }
}

// ── TaxiNodeStatusPkt (SMSG 0x267C) ─────────────────────────────────────────
/// Response to CMSG_TAXI_NODE_STATUS_QUERY.
/// C# ref: TaxiPackets.TaxiNodeStatusPkt
/// Status bits: 0=None, 1=Learned, 2=Unlearned, 3=NotEligible
pub struct TaxiNodeStatusPkt {
    pub unit_guid: wow_core::ObjectGuid,
    /// 2-bit field: 0=None 1=Learned 2=Unlearned 3=NotEligible
    pub status: u8,
}

impl ServerPacket for TaxiNodeStatusPkt {
    const OPCODE: ServerOpcodes = ServerOpcodes::TaxiNodeStatus;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_packed_guid(&self.unit_guid);
        pkt.write_bits(self.status as u32, 2);
        pkt.flush_bits();
    }
}

// ── RequestCemeteryListResponse (SMSG 0x258F) ────────────────────────────────
/// Response to CMSG_REQUEST_CEMETERY_LIST.
/// C# ref: MiscPackets.RequestCemeteryListResponse (ConnectionType.Instance)
pub struct RequestCemeteryListResponse {
    pub is_gossip_triggered: bool,
    pub cemetery_ids: Vec<u32>,
}

impl RequestCemeteryListResponse {
    /// Empty response — no graveyards in this zone.
    pub fn empty(is_gossip_triggered: bool) -> Self {
        Self {
            is_gossip_triggered,
            cemetery_ids: vec![],
        }
    }
}

impl ServerPacket for RequestCemeteryListResponse {
    const OPCODE: ServerOpcodes = ServerOpcodes::RequestCemeteryListResponse;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_bit(self.is_gossip_triggered);
        pkt.flush_bits();
        pkt.write_uint32(self.cemetery_ids.len() as u32);
        for id in &self.cemetery_ids {
            pkt.write_uint32(*id);
        }
    }
}

// ── AuctionHelloResponse ─────────────────────────────────────────────────────

/// SMSG_AUCTION_HELLO_RESPONSE — opens the auction house UI on the client.
/// C# ref: AuctionHousePackets.AuctionHelloResponse
pub struct AuctionHelloResponse {
    /// GUID of the auctioneer NPC.
    pub auctioneer_guid: wow_core::ObjectGuid,
    /// AuctionHouse.db2 entry id (1=Alliance, 2=Horde, 7=Neutral).
    pub auction_house_id: i32,
    /// Delay in ms before purchased items are delivered.
    pub purchased_item_delivery_delay: i32,
    /// Delay in ms before cancelled items are returned.
    pub cancelled_item_delivery_delay: i32,
    /// Whether the auction house is currently open for business.
    pub open_for_business: bool,
}

impl AuctionHelloResponse {
    /// Convenience: open neutral auction house for a given NPC guid.
    pub fn open(auctioneer_guid: wow_core::ObjectGuid) -> Self {
        Self {
            auctioneer_guid,
            auction_house_id: 7,                      // neutral
            purchased_item_delivery_delay: 3_600_000, // 1 hour
            cancelled_item_delivery_delay: 3_600_000,
            open_for_business: true,
        }
    }
}

impl ServerPacket for AuctionHelloResponse {
    const OPCODE: ServerOpcodes = ServerOpcodes::AuctionHelloResponse;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_packed_guid(&self.auctioneer_guid);
        pkt.write_int32(self.purchased_item_delivery_delay);
        pkt.write_int32(self.cancelled_item_delivery_delay);
        pkt.write_int32(self.auction_house_id);
        pkt.write_bit(self.open_for_business);
        pkt.flush_bits();
    }
}

// ── NpcInteractionOpenResult ──────────────────────────────────────────────────

/// SMSG_NPC_INTERACTION_OPEN_RESULT — opens an NPC interaction UI on client.
/// C# ref: NPCPackets.NPCInteractionOpenResult
/// PlayerInteractionType values: Banker=8, Binder=20, Auctioneer=21,
/// StableMaster=22, GuildTabardVendor=14, TaxiNode=6, Merchant=5, Trainer=7.
pub struct NpcInteractionOpenResult {
    pub npc: wow_core::ObjectGuid,
    pub interaction_type: i32,
    pub success: bool,
}

impl NpcInteractionOpenResult {
    pub fn new(npc: wow_core::ObjectGuid, interaction_type: i32) -> Self {
        Self {
            npc,
            interaction_type,
            success: true,
        }
    }
}

impl ServerPacket for NpcInteractionOpenResult {
    const OPCODE: ServerOpcodes = ServerOpcodes::NpcInteractionOpenResult;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_packed_guid(&self.npc);
        pkt.write_int32(self.interaction_type);
        pkt.write_bit(self.success);
        pkt.flush_bits();
    }
}

// ── Auction empty results ─────────────────────────────────────────────────────

/// C++ `WorldPackets::AuctionHouse::AuctionListItems`.
///
/// This legacy opcode's `Read()` intentionally consumes no fields in the
/// current C++ source; the handler is also a legacy no-op.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AuctionListItems;

impl ClientPacket for AuctionListItems {
    const OPCODE: ClientOpcodes = ClientOpcodes::AuctionListItems;

    fn read(_pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self)
    }
}

/// C++ `WorldPackets::Addon::AddOnInfo`, used by auction-house taint metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuctionAddonInfo {
    pub name: String,
    pub version: String,
    pub loaded: bool,
    pub disabled: bool,
}

impl AuctionAddonInfo {
    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        // C++ `operator>>(AddOnInfo&)` starts with ResetBitPos().
        pkt.reset_bits();

        let name_len = pkt.read_bits(10)? as usize;
        let version_len = pkt.read_bits(10)? as usize;
        let loaded = pkt.read_bit()?;
        let disabled = pkt.read_bit()?;
        let name = if name_len > 1 {
            let value = pkt.read_string(name_len - 1)?;
            pkt.skip(1)?;
            value
        } else {
            String::new()
        };
        let version = if version_len > 1 {
            let value = pkt.read_string(version_len - 1)?;
            pkt.skip(1)?;
            value
        } else {
            String::new()
        };

        Ok(Self {
            name,
            version,
            loaded,
            disabled,
        })
    }
}

/// C++ `WorldPackets::AuctionHouse::AuctionReplicateItems`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuctionReplicateItems {
    pub auctioneer: ObjectGuid,
    pub change_number_global: u32,
    pub change_number_cursor: u32,
    pub change_number_tombstone: u32,
    pub count: u32,
    pub tainted_by: Option<AuctionAddonInfo>,
}

impl ClientPacket for AuctionReplicateItems {
    const OPCODE: ClientOpcodes = ClientOpcodes::AuctionReplicateItems;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        let auctioneer = pkt.read_guid()?;
        let change_number_global = pkt.read_uint32()?;
        let change_number_cursor = pkt.read_uint32()?;
        let change_number_tombstone = pkt.read_uint32()?;
        let count = pkt.read_uint32()?;
        let tainted_by = if pkt.read_bit()? {
            Some(AuctionAddonInfo::read(pkt)?)
        } else {
            None
        };

        Ok(Self {
            auctioneer,
            change_number_global,
            change_number_cursor,
            change_number_tombstone,
            count,
            tainted_by,
        })
    }
}

/// C++ `WorldPackets::AuctionHouse::AuctionPlaceBid`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuctionPlaceBid {
    pub auctioneer: ObjectGuid,
    pub auction_id: i32,
    pub bid_amount: u64,
    pub tainted_by: Option<AuctionAddonInfo>,
}

impl ClientPacket for AuctionPlaceBid {
    const OPCODE: ClientOpcodes = ClientOpcodes::AuctionPlaceBid;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        let auctioneer = pkt.read_guid()?;
        let auction_id = pkt.read_int32()?;
        let bid_amount = pkt.read_uint64()?;
        let tainted_by = if pkt.read_bit()? {
            Some(AuctionAddonInfo::read(pkt)?)
        } else {
            None
        };

        Ok(Self {
            auctioneer,
            auction_id,
            bid_amount,
            tainted_by,
        })
    }
}

/// C++ `WorldPackets::AuctionHouse::AuctionRemoveItem`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuctionRemoveItem {
    pub auctioneer: ObjectGuid,
    pub auction_id: i32,
    pub item_id: i32,
    pub tainted_by: Option<AuctionAddonInfo>,
}

impl ClientPacket for AuctionRemoveItem {
    const OPCODE: ClientOpcodes = ClientOpcodes::AuctionRemoveItem;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        let auctioneer = pkt.read_guid()?;
        let auction_id = pkt.read_int32()?;
        let item_id = pkt.read_int32()?;
        let tainted_by = if pkt.read_bit()? {
            Some(AuctionAddonInfo::read(pkt)?)
        } else {
            None
        };

        Ok(Self {
            auctioneer,
            auction_id,
            item_id,
            tainted_by,
        })
    }
}

/// C++ `WorldPackets::AuctionHouse::AuctionItemForSale`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AuctionItemForSale {
    pub guid: ObjectGuid,
    pub use_count: u32,
}

impl AuctionItemForSale {
    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self {
            guid: pkt.read_guid()?,
            use_count: pkt.read_uint32()?,
        })
    }
}

/// C++ `WorldPackets::AuctionHouse::AuctionSellItem`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuctionSellItem {
    pub auctioneer: ObjectGuid,
    pub min_bid: u64,
    pub buyout_price: u64,
    pub runtime: u32,
    pub tainted_by: Option<AuctionAddonInfo>,
    pub items: Vec<AuctionItemForSale>,
}

impl ClientPacket for AuctionSellItem {
    const OPCODE: ClientOpcodes = ClientOpcodes::AuctionSellItem;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        let auctioneer = pkt.read_guid()?;
        let min_bid = pkt.read_uint64()?;
        let buyout_price = pkt.read_uint64()?;
        let runtime = pkt.read_uint32()?;
        let tainted_by_present = pkt.read_bit()?;
        let item_count = pkt.read_bits(6)? as usize;
        let tainted_by = if tainted_by_present {
            Some(AuctionAddonInfo::read(pkt)?)
        } else {
            None
        };
        let mut items = Vec::with_capacity(item_count);
        for _ in 0..item_count {
            items.push(AuctionItemForSale::read(pkt)?);
        }

        Ok(Self {
            auctioneer,
            min_bid,
            buyout_price,
            runtime,
            tainted_by,
            items,
        })
    }
}

/// SMSG_AUCTION_LIST_BIDDER_ITEMS_RESULT — empty bidder list.
pub struct AuctionListBidderItemsResult;
impl ServerPacket for AuctionListBidderItemsResult {
    const OPCODE: ServerOpcodes = ServerOpcodes::AuctionListBidderItemsResult;
    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_int32(0); // Items.Count
        pkt.write_int32(0); // TotalCount
        pkt.write_int32(0); // DesiredDelay (ms)
    }
}

/// SMSG_AUCTION_LIST_OWNER_ITEMS_RESULT — empty owner list.
pub struct AuctionListOwnerItemsResult;
impl ServerPacket for AuctionListOwnerItemsResult {
    const OPCODE: ServerOpcodes = ServerOpcodes::AuctionListOwnerItemsResult;
    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_int32(0); // Items.Count
        pkt.write_int32(0); // TotalCount
        pkt.write_int32(0); // DesiredDelay
    }
}

/// SMSG_AUCTION_LIST_PENDING_SALES_RESULT — empty pending sales.
pub struct AuctionListPendingSalesResult;
impl ServerPacket for AuctionListPendingSalesResult {
    const OPCODE: ServerOpcodes = ServerOpcodes::AuctionListPendingSalesResult;
    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_int32(0); // Mails.Count
        pkt.write_int32(0); // TotalNumRecords
    }
}

// ── QueryTimeResponse ────────────────────────────────────────────────────────

/// SMSG_QUERY_TIME_RESPONSE — server time response to CMSG_QUERY_TIME.
/// C# ref: QueryPackets.QueryTimeResponse → WriteInt64(CurrentTime)
pub struct QueryTimeResponse {
    /// Current server Unix timestamp (seconds).
    pub current_time: i64,
}

impl ServerPacket for QueryTimeResponse {
    const OPCODE: ServerOpcodes = ServerOpcodes::QueryTimeResponse;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_int64(self.current_time);
    }
}

// ── MailQueryNextTimeResult ──────────────────────────────────────────────────

/// SMSG_MAIL_QUERY_NEXT_TIME_RESULT — tells client when next mail arrives.
/// C# ref: MailPackets.MailQueryNextTimeResult
/// next_mail_time = -1.0 means "no mail pending".
pub struct MailQueryNextTimeResult {
    /// -1.0 = no mail, 0.0 = mail now, >0 = seconds until delivery.
    pub next_mail_time: f32,
}

impl MailQueryNextTimeResult {
    /// Convenience: "no mail pending" response.
    pub fn no_mail() -> Self {
        Self {
            next_mail_time: -1.0,
        }
    }
}

impl ServerPacket for MailQueryNextTimeResult {
    const OPCODE: ServerOpcodes = ServerOpcodes::MailQueryNextTimeResult;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_float(self.next_mail_time);
        pkt.write_int32(0); // Next.Count = 0
    }
}

// ── LFG list status ──────────────────────────────────────────────────────────

/// C++ `lfg::LFG_QUEUE_DUNGEON`.
pub const LFG_QUEUE_DUNGEON_LIKE_CPP: u8 = 1;
/// C++ `lfg::LFG_UPDATETYPE_REMOVED_FROM_QUEUE`.
pub const LFG_UPDATE_TYPE_REMOVED_FROM_QUEUE_LIKE_CPP: u8 = 8;

/// C++ `WorldPackets::LFG::RideTicket`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LfgRideTicket {
    pub requester_guid: ObjectGuid,
    pub id: u32,
    pub ride_type: u32,
    pub time: i64,
    pub unknown925: bool,
}

impl Default for LfgRideTicket {
    fn default() -> Self {
        Self {
            requester_guid: ObjectGuid::EMPTY,
            id: 0,
            ride_type: 0,
            time: 0,
            unknown925: false,
        }
    }
}

impl LfgRideTicket {
    fn read_like_cpp(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        let requester_guid = pkt.read_packed_guid()?;
        let id = pkt.read_uint32()?;
        let ride_type = pkt.read_uint32()?;
        let time = pkt.read_int64()?;
        let unknown925 = pkt.read_bit()?;
        pkt.reset_bits();
        Ok(Self {
            requester_guid,
            id,
            ride_type,
            time,
            unknown925,
        })
    }

    fn write_like_cpp(&self, pkt: &mut WorldPacket) {
        pkt.write_packed_guid(&self.requester_guid);
        pkt.write_uint32(self.id);
        pkt.write_uint32(self.ride_type);
        pkt.write_int64(self.time);
        pkt.write_bit(self.unknown925);
        pkt.flush_bits();
    }
}

/// C++ `WorldPackets::LFG::LFGUpdateStatus`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LfgUpdateStatus {
    pub ticket: LfgRideTicket,
    pub sub_type: u8,
    pub reason: u8,
    pub slots: Vec<u32>,
    pub requested_roles: u8,
    pub suspended_players: Vec<ObjectGuid>,
    pub queue_map_id: u32,
    pub notify_ui: bool,
    pub is_party: bool,
    pub joined: bool,
    pub lfg_joined: bool,
    pub queued: bool,
    pub unused: bool,
}

impl LfgUpdateStatus {
    /// C++ `HandleLfgListGetStatus` branch when `sLFGMgr` has no state/ticket.
    pub fn removed_from_queue() -> Self {
        Self {
            ticket: LfgRideTicket::default(),
            sub_type: LFG_QUEUE_DUNGEON_LIKE_CPP,
            reason: LFG_UPDATE_TYPE_REMOVED_FROM_QUEUE_LIKE_CPP,
            slots: Vec::new(),
            requested_roles: 0,
            suspended_players: Vec::new(),
            queue_map_id: 0,
            notify_ui: true,
            is_party: false,
            joined: false,
            lfg_joined: false,
            queued: false,
            unused: false,
        }
    }
}

impl ServerPacket for LfgUpdateStatus {
    const OPCODE: ServerOpcodes = ServerOpcodes::LfgUpdateStatus;

    fn write(&self, pkt: &mut WorldPacket) {
        self.ticket.write_like_cpp(pkt);
        pkt.write_uint8(self.sub_type);
        pkt.write_uint8(self.reason);
        pkt.write_uint32(self.slots.len() as u32);
        pkt.write_uint8(self.requested_roles);
        pkt.write_uint32(self.suspended_players.len() as u32);
        pkt.write_uint32(self.queue_map_id);

        for slot in &self.slots {
            pkt.write_uint32(*slot);
        }

        for suspended_player in &self.suspended_players {
            pkt.write_packed_guid(suspended_player);
        }

        pkt.write_bit(self.is_party);
        pkt.write_bit(self.notify_ui);
        pkt.write_bit(self.joined);
        pkt.write_bit(self.lfg_joined);
        pkt.write_bit(self.queued);
        pkt.write_bit(self.unused);
        pkt.flush_bits();
    }
}

/// C++ `WorldPackets::LFG::LFGListBlacklist::BlacklistEntry`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct LfgListBlacklistEntry {
    pub slot: u32,
    pub reason: u32,
    pub sub_reason1: i32,
    pub sub_reason2: i32,
    pub soft_lock: u32,
}

/// C++ `WorldPackets::LFG::LFGListBlacklist`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LfgListBlacklist {
    pub entries: Vec<LfgListBlacklistEntry>,
}

impl LfgListBlacklist {
    pub fn empty() -> Self {
        Self::default()
    }
}

impl ServerPacket for LfgListBlacklist {
    const OPCODE: ServerOpcodes = ServerOpcodes::LfgListUpdateBlacklist;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_uint32(self.entries.len() as u32);
        for entry in &self.entries {
            pkt.write_uint32(entry.slot);
            pkt.write_uint32(entry.reason);
            pkt.write_int32(entry.sub_reason1);
            pkt.write_int32(entry.sub_reason2);
            pkt.write_uint32(entry.soft_lock);
        }
    }
}

/// C++ `WorldPackets::LFG::DFGetSystemInfo`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DfGetSystemInfo {
    pub player: bool,
    pub party_index: Option<u8>,
}

impl ClientPacket for DfGetSystemInfo {
    const OPCODE: ClientOpcodes = ClientOpcodes::DfGetSystemInfo;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        let player = pkt.read_bit()?;
        let has_party_index = pkt.read_bit()?;
        let party_index = if has_party_index {
            Some(pkt.read_uint8()?)
        } else {
            None
        };
        Ok(Self {
            player,
            party_index,
        })
    }
}

/// C++ `WorldPackets::LFG::DFGetJoinStatus`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DfGetJoinStatus;

impl ClientPacket for DfGetJoinStatus {
    const OPCODE: ClientOpcodes = ClientOpcodes::DfGetJoinStatus;

    fn read(_pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self)
    }
}

/// C++ `WorldPackets::Misc::TogglePvP`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TogglePvp;

impl ClientPacket for TogglePvp {
    const OPCODE: ClientOpcodes = ClientOpcodes::TogglePvp;

    fn read(_pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self)
    }
}

/// C++ `WorldPackets::Misc::SetPvP`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SetPvp {
    pub enable_pvp: bool,
}

impl ClientPacket for SetPvp {
    const OPCODE: ClientOpcodes = ClientOpcodes::SetPvp;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self {
            enable_pvp: pkt.read_bit()?,
        })
    }
}

/// C++ `WorldPackets::LFG::LFGBlackList`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LfgBlackList {
    pub player_guid: Option<ObjectGuid>,
    pub slots: Vec<LfgListBlacklistEntry>,
}

impl LfgBlackList {
    fn write_like_cpp(&self, pkt: &mut WorldPacket) {
        pkt.write_bit(self.player_guid.is_some());
        pkt.write_uint32(self.slots.len() as u32);
        if let Some(player_guid) = self.player_guid {
            pkt.write_packed_guid(&player_guid);
        }
        for slot in &self.slots {
            pkt.write_uint32(slot.slot);
            pkt.write_uint32(slot.reason);
            pkt.write_int32(slot.sub_reason1);
            pkt.write_int32(slot.sub_reason2);
            pkt.write_uint32(slot.soft_lock);
        }
    }
}

/// C++ `WorldPackets::LFG::LfgPlayerInfo`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LfgPlayerInfo {
    pub blacklist: LfgBlackList,
    /// Full dungeon/reward rows depend on `sLFGMgr`; empty is the well-defined
    /// response when no random/seasonal dungeon data is represented.
    pub dungeon_count: u32,
}

impl LfgPlayerInfo {
    pub fn empty() -> Self {
        Self::default()
    }
}

impl ServerPacket for LfgPlayerInfo {
    const OPCODE: ServerOpcodes = ServerOpcodes::LfgPlayerInfo;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_uint32(self.dungeon_count);
        self.blacklist.write_like_cpp(pkt);
    }
}

/// C++ `WorldPackets::LFG::LfgPartyInfo`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LfgPartyInfo {
    pub players: Vec<LfgBlackList>,
}

impl LfgPartyInfo {
    pub fn empty() -> Self {
        Self::default()
    }
}

impl ServerPacket for LfgPartyInfo {
    const OPCODE: ServerOpcodes = ServerOpcodes::LfgPartyInfo;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_uint32(self.players.len() as u32);
        for player in &self.players {
            player.write_like_cpp(pkt);
        }
    }
}

/// C++ `WorldPackets::Ticket::GMTicketCaseStatus`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct GmTicketCaseStatus {
    /// Full case rows are not ported yet; C++'s current handler is itself a
    /// TODO and sends an empty status packet.
    pub case_count: u32,
}

impl GmTicketCaseStatus {
    pub fn empty() -> Self {
        Self::default()
    }
}

impl ServerPacket for GmTicketCaseStatus {
    const OPCODE: ServerOpcodes = ServerOpcodes::GmTicketCaseStatus;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_uint32(self.case_count);
    }
}

/// C++ `WorldPackets::Ticket::ComplaintResult`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ComplaintResult {
    pub complaint_type: u32,
    pub result: u8,
}

impl ComplaintResult {
    pub const OK_LIKE_CPP: u8 = 0;
}

impl ServerPacket for ComplaintResult {
    const OPCODE: ServerOpcodes = ServerOpcodes::ComplaintResult;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_uint32(self.complaint_type);
        pkt.write_uint8(self.result);
    }
}

/// C++ `WorldPackets::Ticket::GMTicketSystemStatus`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GmTicketSystemStatus {
    /// C++ `GMTicketSystemStatus` enum: `0` disabled, `1` enabled.
    pub status: i32,
}

impl GmTicketSystemStatus {
    pub const DISABLED: i32 = 0;
    pub const ENABLED: i32 = 1;

    pub fn from_support_enabled_like_cpp(enabled: bool) -> Self {
        Self {
            status: if enabled {
                Self::ENABLED
            } else {
                Self::DISABLED
            },
        }
    }
}

impl ServerPacket for GmTicketSystemStatus {
    const OPCODE: ServerOpcodes = ServerOpcodes::GmTicketSystemStatus;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_int32(self.status);
    }
}

/// C++ `WorldPackets::Calendar::CalendarSendNumPending`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CalendarSendNumPending {
    pub num_pending: u32,
}

impl ServerPacket for CalendarSendNumPending {
    const OPCODE: ServerOpcodes = ServerOpcodes::CalendarSendNumPending;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_uint32(self.num_pending);
    }
}

/// C++ `WorldPackets::Calendar::CalendarSendCalendar`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CalendarSendCalendar {
    pub server_time_packed: u32,
    pub invite_count: u32,
    pub event_count: u32,
    pub raid_lockout_count: u32,
}

impl CalendarSendCalendar {
    /// Represent the empty calendar state used until calendar/event/lockout
    /// managers are wired into the session.
    pub fn empty_now() -> Self {
        Self::empty_at_unix(unix_timestamp())
    }

    pub fn empty_at_unix(unix_seconds: i64) -> Self {
        Self {
            server_time_packed: wow_time_packed_from_unix_seconds(unix_seconds),
            invite_count: 0,
            event_count: 0,
            raid_lockout_count: 0,
        }
    }
}

impl ServerPacket for CalendarSendCalendar {
    const OPCODE: ServerOpcodes = ServerOpcodes::CalendarSendCalendar;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_uint32(self.server_time_packed);
        pkt.write_uint32(self.invite_count);
        pkt.write_uint32(self.event_count);
        pkt.write_uint32(self.raid_lockout_count);
    }
}

/// C++ `WorldPackets::Calendar::CalendarCommunityInviteRequest`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CalendarCommunityInvite {
    pub club_id: u64,
    pub min_level: u8,
    pub max_level: u8,
    pub max_rank_order: u8,
}

impl ClientPacket for CalendarCommunityInvite {
    const OPCODE: ClientOpcodes = ClientOpcodes::CalendarCommunityInvite;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self {
            club_id: pkt.read_uint64()?,
            min_level: pkt.read_uint8()?,
            max_level: pkt.read_uint8()?,
            max_rank_order: pkt.read_uint8()?,
        })
    }
}

/// C++ `WorldPackets::Calendar::CalendarInvite`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CalendarInvite {
    pub event_id: u64,
    pub moderator_id: u64,
    pub club_id: u64,
    pub creating: bool,
    pub is_sign_up: bool,
    pub name: String,
}

impl ClientPacket for CalendarInvite {
    const OPCODE: ClientOpcodes = ClientOpcodes::CalendarInvite;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        let event_id = pkt.read_uint64()?;
        let moderator_id = pkt.read_uint64()?;
        let club_id = pkt.read_uint64()?;
        let name_len = pkt.read_bits(9)? as usize;
        let creating = pkt.read_bit()?;
        let is_sign_up = pkt.read_bit()?;
        let name = pkt.read_string(name_len)?;
        Ok(Self {
            event_id,
            moderator_id,
            club_id,
            creating,
            is_sign_up,
            name,
        })
    }
}

/// C++ `WorldPackets::Calendar::CalendarAddEventInviteInfo`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CalendarAddEventInviteInfo {
    pub guid: ObjectGuid,
    pub status: u8,
    pub moderator: u8,
    pub unused_801_1: Option<ObjectGuid>,
    pub unused_801_2: Option<u64>,
    pub unused_801_3: Option<u64>,
}

impl CalendarAddEventInviteInfo {
    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        let guid = pkt.read_guid()?;
        let status = pkt.read_uint8()?;
        let moderator = pkt.read_uint8()?;
        let has_unused_801_1 = pkt.read_bit()?;
        let has_unused_801_2 = pkt.read_bit()?;
        let has_unused_801_3 = pkt.read_bit()?;
        let unused_801_1 = if has_unused_801_1 {
            Some(pkt.read_guid()?)
        } else {
            None
        };
        let unused_801_2 = if has_unused_801_2 {
            Some(pkt.read_uint64()?)
        } else {
            None
        };
        let unused_801_3 = if has_unused_801_3 {
            Some(pkt.read_uint64()?)
        } else {
            None
        };
        Ok(Self {
            guid,
            status,
            moderator,
            unused_801_1,
            unused_801_2,
            unused_801_3,
        })
    }
}

/// C++ `WorldPackets::Calendar::CalendarAddEvent`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CalendarAddEvent {
    pub club_id: u64,
    pub event_type: u8,
    pub texture_id: i32,
    pub time_packed: u32,
    pub flags: u32,
    pub invites: Vec<CalendarAddEventInviteInfo>,
    pub title: String,
    pub description: String,
    pub max_size: u32,
}

impl ClientPacket for CalendarAddEvent {
    const OPCODE: ClientOpcodes = ClientOpcodes::CalendarAddEvent;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        let club_id = pkt.read_uint64()?;
        let event_type = pkt.read_uint8()?;
        let texture_id = pkt.read_int32()?;
        let time_packed = pkt.read_uint32()?;
        let flags = pkt.read_uint32()?;
        let invite_count = pkt.read_uint32()? as usize;
        let title_len = pkt.read_bits(8)? as usize;
        let description_len = pkt.read_bits(11)? as usize;
        let mut invites = Vec::with_capacity(invite_count);
        for _ in 0..invite_count {
            invites.push(CalendarAddEventInviteInfo::read(pkt)?);
        }
        let title = pkt.read_string(title_len)?;
        let description = pkt.read_string(description_len)?;
        let max_size = pkt.read_uint32()?;
        Ok(Self {
            club_id,
            event_type,
            texture_id,
            time_packed,
            flags,
            invites,
            title,
            description,
            max_size,
        })
    }
}

/// C++ `WorldPackets::Calendar::CalendarUpdateEvent`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CalendarUpdateEvent {
    pub club_id: u64,
    pub event_id: u64,
    pub moderator_id: u64,
    pub event_type: u8,
    pub texture_id: u32,
    pub time_packed: u32,
    pub flags: u32,
    pub title: String,
    pub description: String,
    pub max_size: u32,
}

impl ClientPacket for CalendarUpdateEvent {
    const OPCODE: ClientOpcodes = ClientOpcodes::CalendarUpdateEvent;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        let club_id = pkt.read_uint64()?;
        let event_id = pkt.read_uint64()?;
        let moderator_id = pkt.read_uint64()?;
        let event_type = pkt.read_uint8()?;
        let texture_id = pkt.read_uint32()?;
        let time_packed = pkt.read_uint32()?;
        let flags = pkt.read_uint32()?;
        let title_len = pkt.read_bits(8)? as usize;
        let description_len = pkt.read_bits(11)? as usize;
        let title = pkt.read_string(title_len)?;
        let description = pkt.read_string(description_len)?;
        let max_size = pkt.read_uint32()?;
        Ok(Self {
            club_id,
            event_id,
            moderator_id,
            event_type,
            texture_id,
            time_packed,
            flags,
            title,
            description,
            max_size,
        })
    }
}

/// C++ `WorldPackets::Calendar::CalendarGetEvent`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CalendarGetEvent {
    pub event_id: u64,
}

impl ClientPacket for CalendarGetEvent {
    const OPCODE: ClientOpcodes = ClientOpcodes::CalendarGetEvent;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self {
            event_id: pkt.read_uint64()?,
        })
    }
}

/// C++ `WorldPackets::Calendar::CalendarRemoveEvent`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CalendarRemoveEvent {
    pub event_id: u64,
    pub moderator_id: u64,
    pub club_id: u64,
    pub flags: u32,
}

impl ClientPacket for CalendarRemoveEvent {
    const OPCODE: ClientOpcodes = ClientOpcodes::CalendarRemoveEvent;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self {
            event_id: pkt.read_uint64()?,
            moderator_id: pkt.read_uint64()?,
            club_id: pkt.read_uint64()?,
            flags: pkt.read_uint32()?,
        })
    }
}

/// C++ `WorldPackets::Calendar::CalendarCopyEvent`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CalendarCopyEvent {
    pub event_id: u64,
    pub moderator_id: u64,
    pub event_club_id: u64,
    pub date: u32,
}

impl ClientPacket for CalendarCopyEvent {
    const OPCODE: ClientOpcodes = ClientOpcodes::CalendarCopyEvent;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self {
            event_id: pkt.read_uint64()?,
            moderator_id: pkt.read_uint64()?,
            event_club_id: pkt.read_uint64()?,
            date: pkt.read_uint32()?,
        })
    }
}

/// C++ `WorldPackets::Calendar::CalendarRemoveInvite`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CalendarRemoveInvite {
    pub guid: ObjectGuid,
    pub invite_id: u64,
    pub moderator_id: u64,
    pub event_id: u64,
}

impl ClientPacket for CalendarRemoveInvite {
    const OPCODE: ClientOpcodes = ClientOpcodes::CalendarRemoveInvite;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self {
            guid: pkt.read_guid()?,
            invite_id: pkt.read_uint64()?,
            moderator_id: pkt.read_uint64()?,
            event_id: pkt.read_uint64()?,
        })
    }
}

/// C++ `WorldPackets::Calendar::CalendarEventSignUp`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CalendarEventSignUp {
    pub event_id: u64,
    pub club_id: u64,
    pub tentative: bool,
}

impl ClientPacket for CalendarEventSignUp {
    const OPCODE: ClientOpcodes = ClientOpcodes::CalendarEventSignUp;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        let event_id = pkt.read_uint64()?;
        let club_id = pkt.read_uint64()?;
        let tentative = pkt.read_bit()?;
        Ok(Self {
            event_id,
            club_id,
            tentative,
        })
    }
}

/// C++ `WorldPackets::Calendar::CalendarRSVP`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CalendarRsvp {
    pub event_id: u64,
    pub invite_id: u64,
    pub status: u8,
}

impl ClientPacket for CalendarRsvp {
    const OPCODE: ClientOpcodes = ClientOpcodes::CalendarRsvp;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self {
            event_id: pkt.read_uint64()?,
            invite_id: pkt.read_uint64()?,
            status: pkt.read_uint8()?,
        })
    }
}

/// C++ `WorldPackets::Calendar::CalendarStatus`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CalendarStatus {
    pub guid: ObjectGuid,
    pub event_id: u64,
    pub invite_id: u64,
    pub moderator_id: u64,
    pub status: u8,
}

impl ClientPacket for CalendarStatus {
    const OPCODE: ClientOpcodes = ClientOpcodes::CalendarStatus;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self {
            guid: pkt.read_guid()?,
            event_id: pkt.read_uint64()?,
            invite_id: pkt.read_uint64()?,
            moderator_id: pkt.read_uint64()?,
            status: pkt.read_uint8()?,
        })
    }
}

/// C++ `WorldPackets::Calendar::CalendarModeratorStatusQuery`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CalendarModeratorStatusQuery {
    pub guid: ObjectGuid,
    pub event_id: u64,
    pub invite_id: u64,
    pub moderator_id: u64,
    pub status: u8,
}

impl ClientPacket for CalendarModeratorStatusQuery {
    const OPCODE: ClientOpcodes = ClientOpcodes::CalendarModeratorStatus;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self {
            guid: pkt.read_guid()?,
            event_id: pkt.read_uint64()?,
            invite_id: pkt.read_uint64()?,
            moderator_id: pkt.read_uint64()?,
            status: pkt.read_uint8()?,
        })
    }
}

/// C++ `WorldPackets::Calendar::CalendarCommandResult`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CalendarCommandResult {
    pub command: u8,
    pub result: u8,
    pub name: String,
}

impl CalendarCommandResult {
    pub const COMMAND_LIKE_CPP: u8 = 1;
    pub const ERROR_EVENT_INVALID_LIKE_CPP: u8 = 6;
    pub const ERROR_GUILD_PLAYER_NOT_IN_GUILD_LIKE_CPP: u8 = 9;
    pub const ERROR_NO_INVITE_LIKE_CPP: u8 = 29;

    pub fn with_result_like_cpp(result: u8) -> Self {
        Self {
            command: Self::COMMAND_LIKE_CPP,
            result,
            name: String::new(),
        }
    }

    pub fn event_invalid_like_cpp() -> Self {
        Self::with_result_like_cpp(Self::ERROR_EVENT_INVALID_LIKE_CPP)
    }

    pub fn no_invite_like_cpp() -> Self {
        Self::with_result_like_cpp(Self::ERROR_NO_INVITE_LIKE_CPP)
    }
}

impl ServerPacket for CalendarCommandResult {
    const OPCODE: ServerOpcodes = ServerOpcodes::CalendarCommandResult;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_uint8(self.command);
        pkt.write_uint8(self.result);
        pkt.write_bits(self.name.len() as u32, 9);
        pkt.flush_bits();
        pkt.write_string(&self.name);
    }
}

/// C++ `WorldPackets::Calendar::CalendarComplain`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CalendarComplain {
    pub invited_by_guid: ObjectGuid,
    pub event_id: u64,
    pub invite_id: u64,
}

impl ClientPacket for CalendarComplain {
    const OPCODE: ClientOpcodes = ClientOpcodes::CalendarComplain;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self {
            invited_by_guid: pkt.read_guid()?,
            event_id: pkt.read_uint64()?,
            invite_id: pkt.read_uint64()?,
        })
    }
}

/// C++ `WorldPackets::ArenaTeam::ArenaTeamRoster`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArenaTeamRoster {
    pub team_id: u32,
}

impl ClientPacket for ArenaTeamRoster {
    const OPCODE: ClientOpcodes = ClientOpcodes::ArenaTeamRoster;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self {
            team_id: pkt.read_uint32()?,
        })
    }
}

/// C++ `WorldPackets::ArenaTeam::ArenaTeamAccept`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ArenaTeamAccept;

impl ClientPacket for ArenaTeamAccept {
    const OPCODE: ClientOpcodes = ClientOpcodes::ArenaTeamAccept;

    fn read(_pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self)
    }
}

/// C++ `WorldPackets::ArenaTeam::ArenaTeamDecline`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ArenaTeamDecline;

impl ClientPacket for ArenaTeamDecline {
    const OPCODE: ClientOpcodes = ClientOpcodes::ArenaTeamDecline;

    fn read(_pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self)
    }
}

/// C++ `WorldPackets::ArenaTeam::ArenaTeamLeave`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ArenaTeamLeave;

impl ClientPacket for ArenaTeamLeave {
    const OPCODE: ClientOpcodes = ClientOpcodes::ArenaTeamLeave;

    fn read(_pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self)
    }
}

/// C++ `WorldPackets::ArenaTeam::ArenaTeamRemove`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArenaTeamRemove {
    pub team_id: u32,
    pub target_name: String,
}

impl ClientPacket for ArenaTeamRemove {
    const OPCODE: ClientOpcodes = ClientOpcodes::ArenaTeamRemove;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        let team_id = pkt.read_uint32()?;
        let target_name_len = pkt.read_bits(9)? as usize;
        let target_name = pkt.read_string(target_name_len)?;

        Ok(Self {
            team_id,
            target_name,
        })
    }
}

/// C++ `WorldPackets::ArenaTeam::ArenaTeamDisband`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArenaTeamDisband {
    pub team_id: u32,
}

impl ClientPacket for ArenaTeamDisband {
    const OPCODE: ClientOpcodes = ClientOpcodes::ArenaTeamDisband;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self {
            team_id: pkt.read_uint32()?,
        })
    }
}

/// C++ `WorldPackets::ArenaTeam::ArenaTeamLeader`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArenaTeamLeader {
    pub team_id: u32,
    pub target_name: String,
}

impl ClientPacket for ArenaTeamLeader {
    const OPCODE: ClientOpcodes = ClientOpcodes::ArenaTeamLeader;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        let team_id = pkt.read_uint32()?;
        let target_name_len = pkt.read_bits(9)? as usize;
        let target_name = pkt.read_string(target_name_len)?;

        Ok(Self {
            team_id,
            target_name,
        })
    }
}

/// C++ `WorldPackets::ArenaTeam::QueryArenaTeam`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QueryArenaTeam {
    pub team_id: u32,
}

impl ClientPacket for QueryArenaTeam {
    const OPCODE: ClientOpcodes = ClientOpcodes::QueryArenaTeam;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self {
            team_id: pkt.read_uint32()?,
        })
    }
}

/// C++ `WorldPackets::Trade::BusyTrade`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BusyTrade;

impl ClientPacket for BusyTrade {
    const OPCODE: ClientOpcodes = ClientOpcodes::BusyTrade;

    fn read(_pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self)
    }
}

/// C++ `WorldPackets::Trade::AcceptTrade`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct AcceptTrade {
    pub state_index: u32,
}

impl ClientPacket for AcceptTrade {
    const OPCODE: ClientOpcodes = ClientOpcodes::AcceptTrade;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self {
            state_index: pkt.read_uint32()?,
        })
    }
}

/// C++ `WorldPackets::Trade::ClearTradeItem`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ClearTradeItem {
    pub trade_slot: u8,
}

impl ClientPacket for ClearTradeItem {
    const OPCODE: ClientOpcodes = ClientOpcodes::ClearTradeItem;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self {
            trade_slot: pkt.read_uint8()?,
        })
    }
}

/// C++ `WorldPackets::Trade::SetTradeItem`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SetTradeItem {
    pub trade_slot: u8,
    pub pack_slot: u8,
    pub item_slot_in_pack: u8,
}

impl ClientPacket for SetTradeItem {
    const OPCODE: ClientOpcodes = ClientOpcodes::SetTradeItem;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self {
            trade_slot: pkt.read_uint8()?,
            pack_slot: pkt.read_uint8()?,
            item_slot_in_pack: pkt.read_uint8()?,
        })
    }
}

/// C++ `WorldPackets::Trade::SetTradeSpell`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SetTradeSpell {
    pub spell_id: u32,
    pub pack_slot: u8,
    pub item_slot_in_pack: u8,
}

impl ClientPacket for SetTradeSpell {
    const OPCODE: ClientOpcodes = ClientOpcodes::SetTradeSpell;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self {
            spell_id: pkt.read_uint32()?,
            pack_slot: pkt.read_uint8()?,
            item_slot_in_pack: pkt.read_uint8()?,
        })
    }
}

/// C++ `WorldPackets::Petition::SignPetition`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SignPetition {
    pub petition_guid: ObjectGuid,
    pub choice: u8,
}

impl ClientPacket for SignPetition {
    const OPCODE: ClientOpcodes = ClientOpcodes::SignPetition;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        let guid_bytes = pkt.read_bytes(16)?;
        let mut raw = [0u8; 16];
        raw.copy_from_slice(&guid_bytes);
        Ok(Self {
            petition_guid: ObjectGuid::from_raw_bytes(&raw),
            choice: pkt.read_uint8()?,
        })
    }
}

/// C++ `WorldPackets::Petition::DeclinePetition`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DeclinePetition {
    pub petition_guid: ObjectGuid,
}

impl ClientPacket for DeclinePetition {
    const OPCODE: ClientOpcodes = ClientOpcodes::DeclinePetition;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        let guid_bytes = pkt.read_bytes(16)?;
        let mut raw = [0u8; 16];
        raw.copy_from_slice(&guid_bytes);
        Ok(Self {
            petition_guid: ObjectGuid::from_raw_bytes(&raw),
        })
    }
}

/// C++ `WorldPackets::Petition::QueryPetition`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct QueryPetition {
    pub petition_id: u32,
    pub item_guid: ObjectGuid,
}

impl ClientPacket for QueryPetition {
    const OPCODE: ClientOpcodes = ClientOpcodes::QueryPetition;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        let petition_id = pkt.read_uint32()?;
        let guid_bytes = pkt.read_bytes(16)?;
        let mut raw = [0u8; 16];
        raw.copy_from_slice(&guid_bytes);
        Ok(Self {
            petition_id,
            item_guid: ObjectGuid::from_raw_bytes(&raw),
        })
    }
}

/// C++ `WorldPackets::Petition::QueryPetitionResponse` without `PetitionInfo`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct QueryPetitionResponse {
    pub petition_id: u32,
    pub allow: bool,
}

impl QueryPetitionResponse {
    pub fn not_found_like_cpp(item_guid: ObjectGuid) -> Self {
        Self {
            petition_id: item_guid.counter() as u32,
            allow: false,
        }
    }
}

impl ServerPacket for QueryPetitionResponse {
    const OPCODE: ServerOpcodes = ServerOpcodes::QueryPetitionResponse;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_uint32(self.petition_id);
        pkt.write_bit(self.allow);
        pkt.flush_bits();
    }
}

/// C++ `WorldPackets::Trade::SetTradeGold`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SetTradeGold {
    pub coinage: u64,
}

impl ClientPacket for SetTradeGold {
    const OPCODE: ClientOpcodes = ClientOpcodes::SetTradeGold;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self {
            coinage: pkt.read_uint64()?,
        })
    }
}

/// C++ `WorldPackets::Trade::UnacceptTrade`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct UnacceptTrade;

impl ClientPacket for UnacceptTrade {
    const OPCODE: ClientOpcodes = ClientOpcodes::UnacceptTrade;

    fn read(_pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self)
    }
}

/// C++ `WorldPackets::Trade::BeginTrade`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BeginTrade;

impl ClientPacket for BeginTrade {
    const OPCODE: ClientOpcodes = ClientOpcodes::BeginTrade;

    fn read(_pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self)
    }
}

/// C++ `WorldPackets::Trade::IgnoreTrade`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct IgnoreTrade;

impl ClientPacket for IgnoreTrade {
    const OPCODE: ClientOpcodes = ClientOpcodes::IgnoreTrade;

    fn read(_pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self)
    }
}

/// C++ `TRADE_STATUS_PLAYER_BUSY`.
pub const TRADE_STATUS_PLAYER_BUSY_LIKE_CPP: u8 = 0;

/// C++ `TRADE_STATUS_INITIATED`.
pub const TRADE_STATUS_INITIATED_LIKE_CPP: u8 = 2;

/// C++ `TRADE_STATUS_CANCELLED`.
pub const TRADE_STATUS_CANCELLED_LIKE_CPP: u8 = 3;

/// C++ `TRADE_STATUS_ACCEPTED`.
pub const TRADE_STATUS_ACCEPTED_LIKE_CPP: u8 = 4;

/// C++ `TRADE_STATUS_UNACCEPTED`.
pub const TRADE_STATUS_UNACCEPTED_LIKE_CPP: u8 = 7;

/// C++ `TRADE_STATUS_STATE_CHANGED`.
pub const TRADE_STATUS_STATE_CHANGED_LIKE_CPP: u8 = 9;

/// C++ `TRADE_STATUS_FAILED`.
pub const TRADE_STATUS_FAILED_LIKE_CPP: u8 = 12;

/// C++ `TRADE_STATUS_PLAYER_IGNORED`.
pub const TRADE_STATUS_PLAYER_IGNORED_LIKE_CPP: u8 = 14;

/// C++ `TRADE_SLOT_COUNT`.
pub const TRADE_SLOT_COUNT_LIKE_CPP: u8 = 7;

/// C++ `EQUIP_ERR_NOT_ENOUGH_MONEY`.
pub const EQUIP_ERR_NOT_ENOUGH_MONEY_LIKE_CPP: i32 = 30;

/// Bounded C++ `WorldPackets::Trade::TradeStatus` writer.
///
/// C++ writes `PartnerIsSameBnetAccount`, then five status bits. The bounded
/// Rust writer currently represents the `TRADE_STATUS_INITIATED` payload and
/// cancel-like statuses that only flush bits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TradeStatus {
    pub status: u8,
    pub partner_is_same_bnet_account: bool,
    pub id: u32,
    pub failure_for_you: bool,
    pub bag_result: i32,
    pub item_id: i32,
}

impl TradeStatus {
    pub fn cancel_like_cpp(status: u8) -> Self {
        Self {
            status,
            partner_is_same_bnet_account: false,
            id: 0,
            failure_for_you: false,
            bag_result: 0,
            item_id: 0,
        }
    }

    pub fn initiated_like_cpp(id: u32) -> Self {
        Self {
            status: TRADE_STATUS_INITIATED_LIKE_CPP,
            partner_is_same_bnet_account: false,
            id,
            failure_for_you: false,
            bag_result: 0,
            item_id: 0,
        }
    }

    pub fn status_only_like_cpp(status: u8) -> Self {
        Self {
            status,
            partner_is_same_bnet_account: false,
            id: 0,
            failure_for_you: false,
            bag_result: 0,
            item_id: 0,
        }
    }

    pub fn failed_like_cpp(bag_result: i32, item_id: i32) -> Self {
        Self {
            status: TRADE_STATUS_FAILED_LIKE_CPP,
            partner_is_same_bnet_account: false,
            id: 0,
            failure_for_you: false,
            bag_result,
            item_id,
        }
    }
}

impl ServerPacket for TradeStatus {
    const OPCODE: ServerOpcodes = ServerOpcodes::TradeStatus;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_bit(self.partner_is_same_bnet_account);
        pkt.write_bits(u32::from(self.status), 5);
        match self.status {
            TRADE_STATUS_FAILED_LIKE_CPP => {
                pkt.write_bit(self.failure_for_you);
                pkt.write_int32(self.bag_result);
                pkt.write_int32(self.item_id);
            }
            TRADE_STATUS_INITIATED_LIKE_CPP => {
                pkt.write_uint32(self.id);
            }
            _ => {
                pkt.flush_bits();
            }
        }
    }
}

/// C++ `WorldPackets::Guild::GuildBankRemainingWithdrawMoney`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct GuildBankRemainingWithdrawMoney {
    pub remaining_withdraw_money: i64,
}

impl ServerPacket for GuildBankRemainingWithdrawMoney {
    const OPCODE: ServerOpcodes = ServerOpcodes::GuildBankRemainingWithdrawMoney;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_int64(self.remaining_withdraw_money);
    }
}

/// C++ `WorldPackets::Token::CommerceTokenGetLog`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommerceTokenGetLog {
    pub unk_int: u32,
}

impl ClientPacket for CommerceTokenGetLog {
    const OPCODE: ClientOpcodes = ClientOpcodes::CommerceTokenGetLog;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self {
            unk_int: pkt.read_uint32()?,
        })
    }
}

/// C++ `WorldPackets::AuctionHouse::AuctionableTokenSell`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct AuctionableTokenSell;

impl ClientPacket for AuctionableTokenSell {
    const OPCODE: ClientOpcodes = ClientOpcodes::AuctionableTokenSell;

    fn read(_pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self)
    }
}

/// C++ `WorldPackets::AuctionHouse::AuctionableTokenSellAtMarketPrice`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct AuctionableTokenSellAtMarketPrice;

impl ClientPacket for AuctionableTokenSellAtMarketPrice {
    const OPCODE: ClientOpcodes = ClientOpcodes::AuctionableTokenSellAtMarketPrice;

    fn read(_pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self)
    }
}

/// C++ `TOKEN_RESULT_SUCCESS`.
pub const TOKEN_RESULT_SUCCESS_LIKE_CPP: u32 = 0;

/// C++ `WorldPackets::Token::CommerceTokenGetLogResponse`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CommerceTokenGetLogResponse {
    pub unk_int: u32,
    pub result: u32,
    /// Auctionable token rows are unimplemented in this C++ branch too; the
    /// handler sends a success response with an empty list.
    pub auctionable_token_count: u32,
}

impl CommerceTokenGetLogResponse {
    pub fn success_empty(unk_int: u32) -> Self {
        Self {
            unk_int,
            result: TOKEN_RESULT_SUCCESS_LIKE_CPP,
            auctionable_token_count: 0,
        }
    }
}

impl ServerPacket for CommerceTokenGetLogResponse {
    const OPCODE: ServerOpcodes = ServerOpcodes::CommerceTokenGetLogResponse;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_uint32(self.unk_int);
        pkt.write_uint32(self.result);
        pkt.write_uint32(self.auctionable_token_count);
    }
}

// ── RatedPvpInfo ─────────────────────────────────────────────────────────────

/// C++ `WorldPackets::Battleground::RequestBattlefieldStatus`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RequestBattlefieldStatus;

impl ClientPacket for RequestBattlefieldStatus {
    const OPCODE: ClientOpcodes = ClientOpcodes::RequestBattlefieldStatus;

    fn read(_pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self)
    }
}

/// C++ `WorldPackets::Battleground::RatedPvpInfo`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RatedPvpBracketInfo {
    pub personal_rating: i32,
    pub ranking: i32,
    pub season_played: i32,
    pub season_won: i32,
    pub unused1: i32,
    pub unused2: i32,
    pub weekly_played: i32,
    pub weekly_won: i32,
    pub rounds_season_played: i32,
    pub rounds_season_won: i32,
    pub rounds_weekly_played: i32,
    pub rounds_weekly_won: i32,
    pub best_weekly_rating: i32,
    pub last_weeks_best_rating: i32,
    pub best_season_rating: i32,
    pub pvp_tier_id: i32,
    pub unused3: i32,
    pub unused4: i32,
    pub rank: i32,
    pub disqualified: bool,
}

impl RatedPvpBracketInfo {
    fn write_like_cpp(&self, pkt: &mut WorldPacket) {
        pkt.write_int32(self.personal_rating);
        pkt.write_int32(self.ranking);
        pkt.write_int32(self.season_played);
        pkt.write_int32(self.season_won);
        pkt.write_int32(self.unused1);
        pkt.write_int32(self.unused2);
        pkt.write_int32(self.weekly_played);
        pkt.write_int32(self.weekly_won);
        pkt.write_int32(self.rounds_season_played);
        pkt.write_int32(self.rounds_season_won);
        pkt.write_int32(self.rounds_weekly_played);
        pkt.write_int32(self.rounds_weekly_won);
        pkt.write_int32(self.best_weekly_rating);
        pkt.write_int32(self.last_weeks_best_rating);
        pkt.write_int32(self.best_season_rating);
        pkt.write_int32(self.pvp_tier_id);
        pkt.write_int32(self.unused3);
        pkt.write_int32(self.unused4);
        pkt.write_int32(self.rank);
        pkt.write_bit(self.disqualified);
        pkt.flush_bits();
    }
}

pub const RATED_PVP_BRACKET_COUNT_LIKE_CPP: usize = 7;

pub struct RatedPvpInfo {
    pub brackets: [RatedPvpBracketInfo; RATED_PVP_BRACKET_COUNT_LIKE_CPP],
}

impl Default for RatedPvpInfo {
    fn default() -> Self {
        Self {
            brackets: [RatedPvpBracketInfo::default(); RATED_PVP_BRACKET_COUNT_LIKE_CPP],
        }
    }
}

impl ServerPacket for RatedPvpInfo {
    const OPCODE: ServerOpcodes = ServerOpcodes::RatedPvpInfo;

    fn write(&self, pkt: &mut WorldPacket) {
        for bracket in &self.brackets {
            bracket.write_like_cpp(pkt);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn can_duel_reads_raw_guid_then_bit_like_cpp() {
        let guid = ObjectGuid::create_player(1, 42);
        let mut pkt = WorldPacket::new_empty();
        pkt.write_bytes(&guid.to_raw_bytes());
        pkt.write_bit(true);
        pkt.flush_bits();
        pkt.reset_read();

        let parsed = CanDuel::read(&mut pkt).unwrap();

        assert_eq!(parsed.target_guid, guid);
        assert!(parsed.to_the_death);
        assert_eq!(pkt.remaining(), 0);
    }

    #[test]
    fn can_duel_result_writes_raw_guid_then_bit_like_cpp() {
        let guid = ObjectGuid::create_player(1, 42);
        let packet = CanDuelResult {
            target_guid: guid,
            result: true,
        };
        let bytes = packet.to_bytes();
        let mut body = WorldPacket::from_bytes(&bytes);

        assert_eq!(body.server_opcode(), Some(ServerOpcodes::CanDuelResult));
        assert_eq!(
            body.read_uint16().unwrap(),
            ServerOpcodes::CanDuelResult as u16
        );
        let guid_bytes = body.read_bytes(16).unwrap();
        let mut raw = [0u8; 16];
        raw.copy_from_slice(&guid_bytes);
        assert_eq!(ObjectGuid::from_raw_bytes(&raw), guid);
        assert!(body.read_bit().unwrap());
        assert_eq!(body.remaining(), 0);
    }

    #[test]
    fn duel_response_reads_raw_arbiter_guid_then_bits_like_cpp() {
        let arbiter_guid =
            ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, 9, 1);
        let mut pkt = WorldPacket::new_empty();
        pkt.write_bytes(&arbiter_guid.to_raw_bytes());
        pkt.write_bit(true);
        pkt.write_bit(false);
        pkt.flush_bits();
        pkt.reset_read();

        let parsed = DuelResponse::read(&mut pkt).unwrap();

        assert_eq!(parsed.arbiter_guid, arbiter_guid);
        assert!(parsed.accepted);
        assert!(!parsed.forfeited);
        assert_eq!(pkt.remaining(), 0);
    }

    #[test]
    fn duel_countdown_writes_uint32_like_cpp() {
        let bytes = DuelCountdown { countdown_ms: 3000 }.to_bytes();
        let mut body = WorldPacket::from_bytes(&bytes);

        assert_eq!(body.server_opcode(), Some(ServerOpcodes::DuelCountdown));
        assert_eq!(
            body.read_uint16().unwrap(),
            ServerOpcodes::DuelCountdown as u16
        );
        assert_eq!(body.read_uint32().unwrap(), 3000);
        assert_eq!(body.remaining(), 0);
    }

    #[test]
    fn account_data_times_global() {
        let pkt = AccountDataTimes::global();
        let bytes = pkt.to_bytes();
        // opcode(2) + packed_guid(2 for empty) + server_time(8) + 15*i64(120) = 132
        assert_eq!(bytes.len(), 132);
    }

    #[test]
    fn account_data_times_player() {
        let guid = ObjectGuid::create_player(1, 42);
        let pkt = AccountDataTimes::for_player(guid);
        let bytes = pkt.to_bytes();
        assert!(bytes.len() > 76); // Bigger than empty GUID version
    }

    #[test]
    fn repop_request_reads_check_instance_bit_like_cpp() {
        let mut pkt = WorldPacket::new_empty();
        pkt.write_bit(true);
        pkt.flush_bits();
        pkt.reset_read();

        let parsed = RepopRequest::read(&mut pkt).unwrap();

        assert!(parsed.check_instance);
        assert_eq!(pkt.remaining(), 0);
    }

    #[test]
    fn reclaim_corpse_reads_raw_corpse_guid_like_cpp() {
        let corpse_guid = ObjectGuid::create_world_object(HighGuid::Corpse, 0, 1, 571, 0, 0, 42);
        let mut pkt = WorldPacket::new_empty();
        pkt.write_bytes(&corpse_guid.to_raw_bytes());
        pkt.reset_read();

        let parsed = ReclaimCorpse::read(&mut pkt).unwrap();

        assert_eq!(parsed.corpse_guid, corpse_guid);
        assert_eq!(pkt.remaining(), 0);
    }

    #[test]
    fn request_account_data_reads_cpp_shape() {
        let guid = ObjectGuid::create_player(1, 42);
        let mut pkt = WorldPacket::new_empty();
        pkt.write_packed_guid(&guid);
        pkt.write_bits(7, 4);
        pkt.flush_bits();
        pkt.reset_read();

        let parsed = RequestAccountData::read(&mut pkt).unwrap();

        assert_eq!(parsed.player_guid, guid);
        assert_eq!(parsed.data_type, 7);
        assert_eq!(pkt.remaining(), 0);
    }

    #[test]
    fn user_client_update_account_data_reads_cpp_shape() {
        let guid = ObjectGuid::create_player(1, 42);
        let compressed_data = compress_account_data_like_cpp("layout-cache").unwrap();
        let mut pkt = WorldPacket::new_empty();
        pkt.write_packed_guid(&guid);
        pkt.write_int64(1234);
        pkt.write_uint32("layout-cache".len() as u32);
        pkt.write_bits(6, 4);
        pkt.write_uint32(compressed_data.len() as u32);
        pkt.write_bytes(&compressed_data);
        pkt.reset_read();

        let parsed = UserClientUpdateAccountData::read(&mut pkt).unwrap();

        assert_eq!(parsed.player_guid, guid);
        assert_eq!(parsed.time, 1234);
        assert_eq!(parsed.size, "layout-cache".len() as u32);
        assert_eq!(parsed.data_type, 6);
        assert_eq!(parsed.compressed_data, compressed_data);
        assert_eq!(pkt.remaining(), 0);
    }

    #[test]
    fn update_account_data_writes_cpp_shape_and_roundtrips_zlib_cstring() {
        let guid = ObjectGuid::create_player(1, 42);
        let payload = "cache body without nul";
        let compressed_data = compress_account_data_like_cpp(payload).unwrap();
        let pkt = UpdateAccountData {
            player_guid: guid,
            time: 5678,
            size: payload.len() as u32,
            data_type: 4,
            compressed_data: compressed_data.clone(),
        };
        let encoded = pkt.to_bytes();
        let mut bytes = WorldPacket::new_client(encoded.as_slice().into());
        bytes.skip_opcode();

        assert_eq!(bytes.read_packed_guid().unwrap(), guid);
        assert_eq!(bytes.read_int64().unwrap(), 5678);
        assert_eq!(bytes.read_uint32().unwrap(), payload.len() as u32);
        assert_eq!(bytes.read_bits(4).unwrap(), 4);
        assert_eq!(bytes.read_uint32().unwrap(), compressed_data.len() as u32);
        assert_eq!(
            bytes.read_bytes(compressed_data.len()).unwrap(),
            compressed_data
        );
        assert_eq!(
            decompress_account_data_like_cpp(&pkt.compressed_data, pkt.size).unwrap(),
            payload
        );
        assert_eq!(bytes.remaining(), 0);
    }

    #[test]
    fn loading_screen_notify_reads_cpp_map_and_showing_bit() {
        let mut pkt = WorldPacket::new_empty();
        pkt.write_uint32(571);
        pkt.write_bit(true);
        pkt.flush_bits();
        pkt.reset_read();

        let parsed = LoadingScreenNotify::read(&mut pkt).unwrap();
        assert_eq!(parsed.map_id, 571);
        assert!(parsed.showing);
    }

    #[test]
    fn set_taxi_benchmark_mode_reads_cpp_enable_bit() {
        for enable in [false, true] {
            let mut pkt = WorldPacket::new_empty();
            pkt.write_bit(enable);
            pkt.flush_bits();
            pkt.reset_read();

            let parsed = SetTaxiBenchmarkMode::read(&mut pkt).unwrap();
            assert_eq!(parsed.enable, enable);
        }
    }

    #[test]
    fn activate_taxi_reads_cpp_vendor_node_ground_and_flying_mount_order() {
        let vendor = ObjectGuid::create_world_object(
            wow_core::guid::HighGuid::Creature,
            0,
            1,
            571,
            0,
            9,
            12_345,
        );
        let mut pkt = WorldPacket::new_empty();
        pkt.write_packed_guid(&vendor);
        pkt.write_uint32(7);
        pkt.write_uint32(111);
        pkt.write_uint32(222);
        pkt.reset_read();

        let parsed = ActivateTaxi::read(&mut pkt).unwrap();

        assert_eq!(parsed.vendor, vendor);
        assert_eq!(parsed.node, 7);
        assert_eq!(parsed.ground_mount_id, 111);
        assert_eq!(parsed.flying_mount_id, 222);
    }

    #[test]
    fn activate_taxi_reply_writes_cpp_four_bit_reply() {
        let bytes = ActivateTaxiReply { reply: 4 }.to_bytes();

        assert_eq!(
            u16::from_le_bytes([bytes[0], bytes[1]]),
            ServerOpcodes::ActivateTaxiReply as u16
        );
        let mut payload = WorldPacket::from_bytes(&bytes[2..]);
        assert_eq!(payload.read_bits(4).unwrap(), 4);
    }

    #[test]
    fn set_advanced_combat_logging_reads_cpp_enable_bit() {
        for enable in [false, true] {
            let mut pkt = WorldPacket::new_empty();
            pkt.write_bit(enable);
            pkt.flush_bits();
            pkt.reset_read();

            let parsed = SetAdvancedCombatLogging::read(&mut pkt).unwrap();
            assert_eq!(parsed.enable, enable);
        }
    }

    #[test]
    fn set_currency_flags_reads_cpp_uint32_then_uint8() {
        let mut pkt = WorldPacket::new_empty();
        pkt.write_uint32(395);
        pkt.write_uint8(0x1f);
        pkt.reset_read();

        let parsed = SetCurrencyFlags::read(&mut pkt).unwrap();
        assert_eq!(parsed.currency_id, 395);
        assert_eq!(parsed.flags, 0x1f);
    }

    #[test]
    fn random_roll_client_reads_optional_party_index_then_signed_bounds_like_cpp() {
        let mut pkt = WorldPacket::new_empty();
        pkt.write_bit(true);
        pkt.write_int32(1);
        pkt.write_int32(100);
        pkt.write_uint8(0);
        pkt.reset_read();

        let parsed = RandomRollClient::read(&mut pkt).unwrap();

        assert_eq!(
            parsed,
            RandomRollClient {
                min: 1,
                max: 100,
                party_index: Some(0),
            }
        );
        assert_eq!(pkt.remaining(), 0);
    }

    #[test]
    fn random_roll_client_reads_absent_party_index_like_cpp() {
        let mut pkt = WorldPacket::new_empty();
        pkt.write_bit(false);
        pkt.write_int32(-5);
        pkt.write_int32(5);
        pkt.reset_read();

        let parsed = RandomRollClient::read(&mut pkt).unwrap();

        assert_eq!(
            parsed,
            RandomRollClient {
                min: -5,
                max: 5,
                party_index: None,
            }
        );
        assert_eq!(pkt.remaining(), 0);
    }

    #[test]
    fn random_roll_writes_full_guids_then_signed_values_like_cpp() {
        let roller = ObjectGuid::create_player(1, 42);
        let account = ObjectGuid::new((HighGuid::WowAccount as i64) << 58, 7);
        let bytes = RandomRoll {
            roller,
            roller_wow_account: account,
            min: 1,
            max: 100,
            result: 77,
        }
        .to_bytes();
        let mut pkt = WorldPacket::from_bytes(&bytes);

        assert_eq!(pkt.server_opcode(), Some(ServerOpcodes::RandomRoll));
        assert_eq!(pkt.read_uint16().unwrap(), ServerOpcodes::RandomRoll as u16);
        assert_eq!(pkt.read_guid().unwrap(), roller);
        assert_eq!(pkt.read_guid().unwrap(), account);
        assert_eq!(pkt.read_int32().unwrap(), 1);
        assert_eq!(pkt.read_int32().unwrap(), 100);
        assert_eq!(pkt.read_int32().unwrap(), 77);
        assert_eq!(pkt.remaining(), 0);
    }

    #[test]
    fn set_difficulty_id_reads_cpp_uint32() {
        let mut pkt = WorldPacket::new_empty();
        pkt.write_uint32(23);
        pkt.reset_read();

        let parsed = SetDifficultyId::read(&mut pkt).unwrap();

        assert_eq!(parsed.difficulty_id, 23);
    }

    #[test]
    fn toggle_difficulty_reads_cpp_null_packet() {
        let mut pkt = WorldPacket::new_empty();

        let parsed = ToggleDifficulty::read(&mut pkt).unwrap();

        assert_eq!(parsed, ToggleDifficulty);
        assert_eq!(pkt.remaining(), 0);
    }

    #[test]
    fn set_dungeon_difficulty_reads_cpp_uint32() {
        let mut pkt = WorldPacket::new_empty();
        pkt.write_uint32(2);
        pkt.reset_read();

        let parsed = SetDungeonDifficulty::read(&mut pkt).unwrap();

        assert_eq!(parsed.difficulty_id, 2);
        assert_eq!(pkt.remaining(), 0);
    }

    #[test]
    fn set_raid_difficulty_reads_cpp_int32_then_legacy_u8() {
        let mut pkt = WorldPacket::new_empty();
        pkt.write_int32(4);
        pkt.write_uint8(1);
        pkt.reset_read();

        let parsed = SetRaidDifficulty::read(&mut pkt).unwrap();

        assert_eq!(parsed.difficulty_id, 4);
        assert_eq!(parsed.legacy, 1);
        assert_eq!(pkt.remaining(), 0);
    }

    #[test]
    fn addon_list_reads_cpp_count_bits_flush_and_names() {
        let mut pkt = WorldPacket::new_empty();
        pkt.write_uint32(3);
        pkt.write_bits(5, 10);
        pkt.flush_bits();
        pkt.write_string("Atlas");
        pkt.write_bits(7, 10);
        pkt.flush_bits();
        pkt.write_string("Questie");
        pkt.reset_read();

        let parsed = AddonList::read(&mut pkt).unwrap();
        assert_eq!(parsed.addons, vec!["Atlas", "Questie"]);
    }

    #[test]
    fn violence_level_reads_cpp_uint8() {
        let mut pkt = WorldPacket::new_empty();
        pkt.write_uint8(2);
        pkt.reset_read();

        let parsed = ViolenceLevel::read(&mut pkt).unwrap();
        assert_eq!(parsed.violence_level, 2);
    }

    #[test]
    fn decline_guild_invites_reads_cpp_allow_bit() {
        let mut pkt = WorldPacket::new_empty();
        pkt.write_bit(true);
        pkt.flush_bits();
        pkt.reset_read();

        let parsed = DeclineGuildInvites::read(&mut pkt).unwrap();
        assert!(parsed.allow);
    }

    #[test]
    fn decline_guild_invites_rejects_missing_allow_bit() {
        let mut pkt = WorldPacket::new_empty();

        assert!(DeclineGuildInvites::read(&mut pkt).is_err());
    }

    #[test]
    fn accept_guild_invite_reads_empty_cpp_packet() {
        let mut pkt = WorldPacket::new_empty();

        AcceptGuildInvite::read(&mut pkt).unwrap();
    }

    #[test]
    fn guild_set_achievement_tracking_reads_cpp_counted_ids() {
        let mut pkt = WorldPacket::new_empty();
        pkt.write_uint32(3);
        pkt.write_uint32(100);
        pkt.write_uint32(200);
        pkt.write_uint32(300);
        pkt.reset_read();

        let parsed = GuildSetAchievementTracking::read(&mut pkt).unwrap();
        assert_eq!(parsed.achievement_ids, vec![100, 200, 300]);
    }

    #[test]
    fn guild_set_achievement_tracking_rejects_above_cpp_array_limit() {
        let mut pkt = WorldPacket::new_empty();
        pkt.write_uint32((MAX_GUILD_ACHIEVEMENT_TRACKING_IDS_LIKE_CPP + 1) as u32);
        pkt.reset_read();

        assert!(GuildSetAchievementTracking::read(&mut pkt).is_err());
    }

    #[test]
    fn close_interaction_reads_cpp_source_guid() {
        let source_guid = ObjectGuid::create_player(1, 42);
        let mut pkt = WorldPacket::new_empty();
        pkt.write_packed_guid(&source_guid);
        pkt.reset_read();

        let parsed = CloseInteraction::read(&mut pkt).unwrap();
        assert_eq!(parsed.source_guid, source_guid);
    }

    #[test]
    fn rated_pvp_info_empty_matches_cpp_default_shape() {
        let bytes = RatedPvpInfo::default().to_bytes();
        assert_eq!(
            u16::from_le_bytes([bytes[0], bytes[1]]),
            ServerOpcodes::RatedPvpInfo as u16
        );
        assert_eq!(
            bytes.len(),
            2 + RATED_PVP_BRACKET_COUNT_LIKE_CPP * (19 * 4 + 1)
        );

        let mut pkt = WorldPacket::from_bytes(&bytes[2..]);
        for _ in 0..RATED_PVP_BRACKET_COUNT_LIKE_CPP {
            for _ in 0..19 {
                assert_eq!(pkt.read_int32().unwrap(), 0);
            }
            assert!(!pkt.has_bit().unwrap());
        }
    }

    #[test]
    fn request_battlefield_status_reads_empty_cpp_packet() {
        let mut pkt = WorldPacket::new_empty();
        RequestBattlefieldStatus::read(&mut pkt).unwrap();
        assert_eq!(pkt.remaining(), 0);
    }

    #[test]
    fn lfg_update_status_removed_from_queue_matches_cpp_empty_branch() {
        let bytes = LfgUpdateStatus::removed_from_queue().to_bytes();
        assert_eq!(
            u16::from_le_bytes([bytes[0], bytes[1]]),
            ServerOpcodes::LfgUpdateStatus as u16
        );

        let mut pkt = WorldPacket::from_bytes(&bytes[2..]);
        assert_eq!(pkt.read_packed_guid().unwrap(), ObjectGuid::EMPTY);
        assert_eq!(pkt.read_uint32().unwrap(), 0);
        assert_eq!(pkt.read_uint32().unwrap(), 0);
        assert_eq!(pkt.read_int64().unwrap(), 0);
        assert!(!pkt.has_bit().unwrap());
        assert_eq!(pkt.read_uint8().unwrap(), LFG_QUEUE_DUNGEON_LIKE_CPP);
        assert_eq!(
            pkt.read_uint8().unwrap(),
            LFG_UPDATE_TYPE_REMOVED_FROM_QUEUE_LIKE_CPP
        );
        assert_eq!(pkt.read_uint32().unwrap(), 0);
        assert_eq!(pkt.read_uint8().unwrap(), 0);
        assert_eq!(pkt.read_uint32().unwrap(), 0);
        assert_eq!(pkt.read_uint32().unwrap(), 0);
        assert!(!pkt.has_bit().unwrap());
        assert!(pkt.has_bit().unwrap());
        assert!(!pkt.has_bit().unwrap());
        assert!(!pkt.has_bit().unwrap());
        assert!(!pkt.has_bit().unwrap());
        assert!(!pkt.has_bit().unwrap());
    }

    #[test]
    fn lfg_list_blacklist_empty_matches_cpp_shape() {
        let bytes = LfgListBlacklist::empty().to_bytes();
        assert_eq!(
            u16::from_le_bytes([bytes[0], bytes[1]]),
            ServerOpcodes::LfgListUpdateBlacklist as u16
        );
        assert_eq!(bytes.len(), 2 + 4);

        let mut pkt = WorldPacket::from_bytes(&bytes[2..]);
        assert_eq!(pkt.read_uint32().unwrap(), 0);
    }

    #[test]
    fn lfg_list_blacklist_entry_matches_cpp_order() {
        let bytes = LfgListBlacklist {
            entries: vec![LfgListBlacklistEntry {
                slot: 42,
                reason: 3,
                sub_reason1: 123,
                sub_reason2: -7,
                soft_lock: 0,
            }],
        }
        .to_bytes();

        let mut pkt = WorldPacket::from_bytes(&bytes[2..]);
        assert_eq!(pkt.read_uint32().unwrap(), 1);
        assert_eq!(pkt.read_uint32().unwrap(), 42);
        assert_eq!(pkt.read_uint32().unwrap(), 3);
        assert_eq!(pkt.read_int32().unwrap(), 123);
        assert_eq!(pkt.read_int32().unwrap(), -7);
        assert_eq!(pkt.read_uint32().unwrap(), 0);
    }

    #[test]
    fn df_get_system_info_reads_cpp_bits() {
        let mut pkt = WorldPacket::new_empty();
        pkt.write_bit(true); // Player
        pkt.write_bit(true); // PartyIndex.HasValue
        pkt.write_uint8(7);

        let request = DfGetSystemInfo::read(&mut pkt).unwrap();
        assert!(request.player);
        assert_eq!(request.party_index, Some(7));
    }

    #[test]
    fn df_get_join_status_reads_empty_cpp_packet() {
        let mut pkt = WorldPacket::new_empty();
        DfGetJoinStatus::read(&mut pkt).unwrap();
        assert_eq!(pkt.remaining(), 0);
    }

    #[test]
    fn toggle_pvp_reads_empty_cpp_packet() {
        let mut pkt = WorldPacket::new_empty();
        TogglePvp::read(&mut pkt).unwrap();
        assert_eq!(pkt.remaining(), 0);
    }

    #[test]
    fn set_pvp_reads_cpp_enable_bit() {
        let mut pkt = WorldPacket::new_empty();
        pkt.write_bit(true);
        pkt.flush_bits();
        pkt.reset_read();

        let parsed = SetPvp::read(&mut pkt).unwrap();

        assert!(parsed.enable_pvp);
    }

    #[test]
    fn assign_equipment_set_spec_reads_cpp_uint32_pair() {
        let mut pkt = WorldPacket::new_empty();
        pkt.write_uint32(7);
        pkt.write_uint32(2);
        pkt.reset_read();

        let parsed = AssignEquipmentSetSpec::read(&mut pkt).unwrap();

        assert_eq!(parsed.set_id, 7);
        assert_eq!(parsed.spec_index, 2);
        assert_eq!(pkt.remaining(), 0);
    }

    #[test]
    fn save_equipment_set_reads_cpp_equipment_set_data_shape() {
        let item_guid = ObjectGuid::create_item(1, 55);
        let mut pkt = WorldPacket::new_empty();
        pkt.write_int32(0);
        pkt.write_uint64(0x0102_0304_0506_0708);
        pkt.write_uint32(7);
        pkt.write_uint32(0);
        for i in 0..EQUIPMENT_SET_SLOTS_LIKE_CPP {
            let guid = if i == 0 { item_guid } else { ObjectGuid::EMPTY };
            pkt.write_guid(&guid);
            pkt.write_int32(i as i32 + 10);
        }
        pkt.write_int32(123);
        pkt.write_int32(456);
        pkt.write_int32(11);
        pkt.write_int32(2);
        pkt.write_int32(22);
        pkt.write_int32(16);
        pkt.write_bit(true);
        pkt.write_bits(4, 8);
        pkt.write_bits(6, 9);
        pkt.write_int32(3);
        pkt.write_string("Tank");
        pkt.write_string("INV_01");
        pkt.reset_read();

        let parsed = SaveEquipmentSet::read(&mut pkt).unwrap();

        assert_eq!(parsed.set.set_type, 0);
        assert_eq!(parsed.set.guid, 0x0102_0304_0506_0708);
        assert_eq!(parsed.set.set_id, 7);
        assert_eq!(parsed.set.pieces[0], item_guid);
        assert_eq!(parsed.set.appearances[2], 12);
        assert_eq!(parsed.set.enchants, [123, 456]);
        assert_eq!(parsed.set.secondary_shoulder_appearance_id, 11);
        assert_eq!(parsed.set.secondary_shoulder_slot, 2);
        assert_eq!(parsed.set.secondary_weapon_appearance_id, 22);
        assert_eq!(parsed.set.secondary_weapon_slot, 16);
        assert_eq!(parsed.set.assigned_spec_index, 3);
        assert_eq!(parsed.set.set_name, "Tank");
        assert_eq!(parsed.set.set_icon, "INV_01");
        assert_eq!(pkt.remaining(), 0);
    }

    #[test]
    fn delete_equipment_set_reads_cpp_uint64_id() {
        let mut pkt = WorldPacket::new_empty();
        pkt.write_uint64(0x0102_0304_0506_0708);
        pkt.reset_read();

        let parsed = DeleteEquipmentSet::read(&mut pkt).unwrap();

        assert_eq!(parsed.id, 0x0102_0304_0506_0708);
        assert_eq!(pkt.remaining(), 0);
    }

    #[test]
    fn use_equipment_set_reads_cpp_inv_items_and_guid() {
        let item_guid = ObjectGuid::create_item(1, 55);
        let mut pkt = WorldPacket::new_empty();
        pkt.write_bits(1, 2);
        pkt.write_uint8(255);
        pkt.write_uint8(36);
        for i in 0..EQUIPMENT_SET_SLOTS_LIKE_CPP {
            let guid = if i == 0 { item_guid } else { ObjectGuid::EMPTY };
            pkt.write_guid(&guid);
            pkt.write_uint8(255);
            pkt.write_uint8(i as u8);
        }
        pkt.write_uint64(0x0102_0304_0506_0708);
        pkt.reset_read();

        let parsed = UseEquipmentSet::read(&mut pkt).unwrap();

        assert_eq!(parsed.inv_update.items, vec![(255, 36)]);
        assert_eq!(parsed.items[0].item, item_guid);
        assert_eq!(parsed.items[0].container_slot, 255);
        assert_eq!(parsed.items[0].slot, 0);
        assert_eq!(parsed.guid, 0x0102_0304_0506_0708);
        assert_eq!(pkt.remaining(), 0);
    }

    #[test]
    fn use_equipment_set_result_writes_cpp_guid_and_reason() {
        let bytes = UseEquipmentSetResult {
            guid: 0x0102_0304_0506_0708,
            reason: 4,
        }
        .to_bytes();
        assert_eq!(
            u16::from_le_bytes([bytes[0], bytes[1]]),
            ServerOpcodes::UseEquipmentSetResult as u16
        );

        let mut pkt = WorldPacket::from_bytes(&bytes[2..]);
        assert_eq!(pkt.read_uint64().unwrap(), 0x0102_0304_0506_0708);
        assert_eq!(pkt.read_uint8().unwrap(), 4);
        assert_eq!(pkt.remaining(), 0);
    }

    #[test]
    fn gm_ticket_system_status_matches_cpp_int32_shape() {
        let bytes = GmTicketSystemStatus::from_support_enabled_like_cpp(true).to_bytes();
        assert_eq!(
            u16::from_le_bytes([bytes[0], bytes[1]]),
            ServerOpcodes::GmTicketSystemStatus as u16
        );

        let mut pkt = WorldPacket::from_bytes(&bytes[2..]);
        assert_eq!(pkt.read_int32().unwrap(), GmTicketSystemStatus::ENABLED);
        assert_eq!(pkt.remaining(), 0);

        let bytes = GmTicketSystemStatus::from_support_enabled_like_cpp(false).to_bytes();
        let mut pkt = WorldPacket::from_bytes(&bytes[2..]);
        assert_eq!(pkt.read_int32().unwrap(), GmTicketSystemStatus::DISABLED);
        assert_eq!(pkt.remaining(), 0);
    }

    #[test]
    fn gm_ticket_acknowledge_survey_reads_case_id_like_cpp() {
        let mut pkt = WorldPacket::new_empty();
        pkt.write_int32(42);

        let survey = GmTicketAcknowledgeSurvey::read(&mut pkt).unwrap();
        assert_eq!(survey.case_id, 42);
        assert_eq!(pkt.remaining(), 0);
    }

    #[test]
    fn complaint_reads_chat_variant_like_cpp() {
        let offender_guid = ObjectGuid::create_player(1, 42);
        let mut pkt = WorldPacket::new_empty();
        pkt.write_uint8(SUPPORT_SPAM_TYPE_CHAT_LIKE_CPP);
        pkt.write_packed_guid(&offender_guid);
        pkt.write_uint32(0x0102_0304);
        pkt.write_uint32(55);
        pkt.write_uint32(7);
        pkt.write_uint32(9);
        pkt.write_bits(11, 12);
        pkt.write_string("hello world");

        let complaint = Complaint::read(&mut pkt).unwrap();

        assert_eq!(complaint.complaint_type, SUPPORT_SPAM_TYPE_CHAT_LIKE_CPP);
        assert_eq!(complaint.offender.player_guid, offender_guid);
        assert_eq!(complaint.offender.realm_address, 0x0102_0304);
        assert_eq!(complaint.offender.time_since_offence, 55);
        assert!(complaint.mail_id.is_none());
        let chat = complaint.chat.expect("chat complaint payload");
        assert_eq!(chat.command, 7);
        assert_eq!(chat.channel_id, 9);
        assert_eq!(chat.message_log, "hello world");
        assert!(complaint.calendar_event_guid.is_none());
        assert!(complaint.calendar_invite_guid.is_none());
        assert_eq!(pkt.remaining(), 0);
    }

    #[test]
    fn submit_user_feedback_reads_header_note_and_suggestion_bit_like_cpp() {
        let mut pkt = WorldPacket::new_empty();
        pkt.write_int32(571);
        pkt.write_float(1.25);
        pkt.write_float(2.5);
        pkt.write_float(3.75);
        pkt.write_float(4.0);
        pkt.write_int32(9);
        pkt.write_bits(6, 24); // "hello" plus null terminator
        pkt.write_bit(true);
        pkt.write_string("hello");
        pkt.write_uint8(0);

        let feedback = SubmitUserFeedback::read(&mut pkt).unwrap();

        assert_eq!(feedback.header.map_id, 571);
        assert_eq!(feedback.header.position, Position::xyz(1.25, 2.5, 3.75));
        assert_eq!(feedback.header.facing, 4.0);
        assert_eq!(feedback.header.program, 9);
        assert!(feedback.is_suggestion);
        assert_eq!(feedback.note, "hello");
        assert_eq!(pkt.remaining(), 0);
    }

    #[test]
    fn support_ticket_submit_suggestion_reads_10_bit_message_like_cpp() {
        let mut pkt = WorldPacket::new_empty();
        let message = "future idea text";
        pkt.write_bits(message.len() as u32, 10);
        pkt.write_string(message);

        let suggestion = SupportTicketSubmitSuggestion::read(&mut pkt).unwrap();

        assert_eq!(suggestion.message, message);
        assert_eq!(pkt.remaining(), 0);
    }

    #[test]
    fn support_ticket_submit_bug_reads_header_and_10_bit_message_like_cpp() {
        let mut pkt = WorldPacket::new_empty();
        let message = "broken thing";
        pkt.write_int32(571);
        pkt.write_float(1.25);
        pkt.write_float(2.5);
        pkt.write_float(3.75);
        pkt.write_float(4.0);
        pkt.write_int32(9);
        pkt.write_bits(message.len() as u32, 10);
        pkt.write_string(message);

        let bug = SupportTicketSubmitBug::read(&mut pkt).unwrap();

        assert_eq!(bug.header.map_id, 571);
        assert_eq!(bug.header.position, Position::xyz(1.25, 2.5, 3.75));
        assert_eq!(bug.header.facing, 4.0);
        assert_eq!(bug.header.program, 9);
        assert_eq!(bug.message, message);
        assert_eq!(pkt.remaining(), 0);
    }

    #[test]
    fn support_ticket_submit_complaint_reads_chatlog_note_and_mail_like_cpp() {
        let mut pkt = WorldPacket::new_empty();
        let target = ObjectGuid::create_player(1, 42);
        let note = "report note";
        let chat_text = "bad text";
        let mail_body = "mail body";
        let mail_subject = "subject";

        pkt.write_int32(571);
        pkt.write_float(1.25);
        pkt.write_float(2.5);
        pkt.write_float(3.75);
        pkt.write_float(4.0);
        pkt.write_int32(9);
        pkt.write_packed_guid(&target);
        pkt.write_int32(1);
        pkt.write_int32(2);
        pkt.write_int32(4);
        pkt.write_uint32(1); // ChatLog.Lines.Count
        pkt.write_bit(true); // ReportLineIndex.HasValue
        pkt.write_int64(12345);
        pkt.write_bits(chat_text.len() as u32, 12);
        pkt.write_string(chat_text);
        pkt.write_uint32(0);
        pkt.write_bits(note.len() as u32, 10);
        pkt.write_bit(true); // MailInfo
        pkt.write_bit(false); // CalendarInfo
        pkt.write_bit(false); // PetInfo
        pkt.write_bit(false); // GuildInfo
        pkt.write_bit(false); // LFGListSearchResult
        pkt.write_bit(false); // LFGListApplicant
        pkt.write_bit(false); // ClubMessage
        pkt.write_bit(false); // ClubFinderResult
        pkt.write_bit(false); // Unused910
        pkt.flush_bits();
        pkt.write_uint32(0); // HorusChatLog.Lines.Count
        pkt.write_string(note);
        pkt.write_int64(77);
        pkt.write_bits(mail_body.len() as u32, 13);
        pkt.write_bits(mail_subject.len() as u32, 9);
        pkt.write_string(mail_body);
        pkt.write_string(mail_subject);

        let complaint = SupportTicketSubmitComplaint::read(&mut pkt).unwrap();

        assert_eq!(complaint.header.map_id, 571);
        assert_eq!(complaint.target_character_guid, target);
        assert_eq!(complaint.report_type, 1);
        assert_eq!(complaint.major_category, 2);
        assert_eq!(complaint.minor_category_flags, 4);
        assert_eq!(complaint.chat_log.lines.len(), 1);
        assert_eq!(complaint.chat_log.lines[0].timestamp, 12345);
        assert_eq!(complaint.chat_log.lines[0].text, chat_text);
        assert_eq!(complaint.chat_log.report_line_index, Some(0));
        assert!(complaint.horus_chat_log.lines.is_empty());
        assert_eq!(complaint.note, note);
        let mail = complaint.mail_info.expect("mail info");
        assert_eq!(mail.mail_id, 77);
        assert_eq!(mail.mail_body, mail_body);
        assert_eq!(mail.mail_subject, mail_subject);
        assert!(complaint.calendar_info.is_none());
        assert!(complaint.pet_info.is_none());
        assert!(complaint.guild_info.is_none());
        assert!(complaint.lfg_list_search_result.is_none());
        assert!(complaint.lfg_list_applicant.is_none());
        assert!(complaint.community_message.is_none());
        assert!(complaint.club_finder_result.is_none());
        assert!(complaint.unused910.is_none());
        assert_eq!(pkt.remaining(), 0);
    }

    #[test]
    fn lfg_player_info_empty_matches_cpp_shape() {
        let bytes = LfgPlayerInfo::empty().to_bytes();
        assert_eq!(
            u16::from_le_bytes([bytes[0], bytes[1]]),
            ServerOpcodes::LfgPlayerInfo as u16
        );

        let mut pkt = WorldPacket::from_bytes(&bytes[2..]);
        assert_eq!(pkt.read_uint32().unwrap(), 0); // Dungeon.Count
        assert!(!pkt.has_bit().unwrap()); // BlackList.PlayerGuid.HasValue
        assert_eq!(pkt.read_uint32().unwrap(), 0); // BlackList.Slot.Count
    }

    #[test]
    fn lfg_party_info_empty_matches_cpp_shape() {
        let bytes = LfgPartyInfo::empty().to_bytes();
        assert_eq!(
            u16::from_le_bytes([bytes[0], bytes[1]]),
            ServerOpcodes::LfgPartyInfo as u16
        );

        let mut pkt = WorldPacket::from_bytes(&bytes[2..]);
        assert_eq!(pkt.read_uint32().unwrap(), 0);
    }

    #[test]
    fn gm_ticket_case_status_empty_matches_cpp_todo_handler_shape() {
        let bytes = GmTicketCaseStatus::empty().to_bytes();
        assert_eq!(
            u16::from_le_bytes([bytes[0], bytes[1]]),
            ServerOpcodes::GmTicketCaseStatus as u16
        );
        assert_eq!(bytes.len(), 2 + 4);

        let mut pkt = WorldPacket::from_bytes(&bytes[2..]);
        assert_eq!(pkt.read_uint32().unwrap(), 0);
    }

    #[test]
    fn complaint_result_matches_cpp_shape() {
        let bytes = ComplaintResult {
            complaint_type: SUPPORT_SPAM_TYPE_CHAT_LIKE_CPP as u32,
            result: ComplaintResult::OK_LIKE_CPP,
        }
        .to_bytes();
        assert_eq!(
            u16::from_le_bytes([bytes[0], bytes[1]]),
            ServerOpcodes::ComplaintResult as u16
        );
        assert_eq!(bytes.len(), 2 + 5);

        let mut pkt = WorldPacket::from_bytes(&bytes[2..]);
        assert_eq!(
            pkt.read_uint32().unwrap(),
            SUPPORT_SPAM_TYPE_CHAT_LIKE_CPP as u32
        );
        assert_eq!(pkt.read_uint8().unwrap(), ComplaintResult::OK_LIKE_CPP);
    }

    #[test]
    fn calendar_send_num_pending_matches_cpp_shape() {
        let bytes = CalendarSendNumPending { num_pending: 3 }.to_bytes();
        assert_eq!(
            u16::from_le_bytes([bytes[0], bytes[1]]),
            ServerOpcodes::CalendarSendNumPending as u16
        );
        assert_eq!(bytes.len(), 2 + 4);

        let mut pkt = WorldPacket::from_bytes(&bytes[2..]);
        assert_eq!(pkt.read_uint32().unwrap(), 3);
    }

    #[test]
    fn calendar_send_calendar_empty_matches_cpp_header_shape() {
        let bytes = CalendarSendCalendar::empty_at_unix(946_684_800).to_bytes();
        assert_eq!(
            u16::from_le_bytes([bytes[0], bytes[1]]),
            ServerOpcodes::CalendarSendCalendar as u16
        );
        assert_eq!(bytes.len(), 2 + 4 + 4 + 4 + 4);

        let mut pkt = WorldPacket::from_bytes(&bytes[2..]);
        assert_eq!(pkt.read_uint32().unwrap(), 0x0000_3000); // 2000-01-01 00:00 UTC
        assert_eq!(pkt.read_uint32().unwrap(), 0); // Invites.Count
        assert_eq!(pkt.read_uint32().unwrap(), 0); // Events.Count
        assert_eq!(pkt.read_uint32().unwrap(), 0); // RaidLockouts.Count
    }

    #[test]
    fn calendar_community_invite_reads_cpp_field_order() {
        let mut pkt = WorldPacket::new_empty();
        pkt.write_uint64(0x0102_0304_0506_0708);
        pkt.write_uint8(10);
        pkt.write_uint8(70);
        pkt.write_uint8(3);

        let query = CalendarCommunityInvite::read(&mut pkt).unwrap();
        assert_eq!(query.club_id, 0x0102_0304_0506_0708);
        assert_eq!(query.min_level, 10);
        assert_eq!(query.max_level, 70);
        assert_eq!(query.max_rank_order, 3);
    }

    #[test]
    fn calendar_get_event_reads_cpp_event_id() {
        let mut pkt = WorldPacket::new_empty();
        pkt.write_uint64(0x0102_0304_0506_0708);

        let query = CalendarGetEvent::read(&mut pkt).unwrap();
        assert_eq!(query.event_id, 0x0102_0304_0506_0708);
    }

    #[test]
    fn calendar_remove_event_reads_cpp_field_order() {
        let mut pkt = WorldPacket::new_empty();
        pkt.write_uint64(0x1111_2222_3333_4444);
        pkt.write_uint64(0x5555_6666_7777_8888);
        pkt.write_uint64(0x9999_AAAA_BBBB_CCCC);
        pkt.write_uint32(0xDEAD_BEEF);

        let query = CalendarRemoveEvent::read(&mut pkt).unwrap();
        assert_eq!(query.event_id, 0x1111_2222_3333_4444);
        assert_eq!(query.moderator_id, 0x5555_6666_7777_8888);
        assert_eq!(query.club_id, 0x9999_AAAA_BBBB_CCCC);
        assert_eq!(query.flags, 0xDEAD_BEEF);
    }

    #[test]
    fn calendar_copy_event_reads_cpp_field_order() {
        let mut pkt = WorldPacket::new_empty();
        pkt.write_uint64(0x1111_2222_3333_4444);
        pkt.write_uint64(0x5555_6666_7777_8888);
        pkt.write_uint64(0x9999_AAAA_BBBB_CCCC);
        pkt.write_uint32(0xDEAD_BEEF);

        let query = CalendarCopyEvent::read(&mut pkt).unwrap();
        assert_eq!(query.event_id, 0x1111_2222_3333_4444);
        assert_eq!(query.moderator_id, 0x5555_6666_7777_8888);
        assert_eq!(query.event_club_id, 0x9999_AAAA_BBBB_CCCC);
        assert_eq!(query.date, 0xDEAD_BEEF);
    }

    #[test]
    fn calendar_remove_invite_reads_cpp_field_order() {
        let guid = ObjectGuid::new(0x0102_0304_0506_0708, 0x1111_2222_3333_4444);
        let mut pkt = WorldPacket::new_empty();
        pkt.write_guid(&guid);
        pkt.write_uint64(0x5555_6666_7777_8888);
        pkt.write_uint64(0x9999_AAAA_BBBB_CCCC);
        pkt.write_uint64(0xDEAD_BEEF_CAFE_BABE);

        let query = CalendarRemoveInvite::read(&mut pkt).unwrap();
        assert_eq!(query.guid, guid);
        assert_eq!(query.invite_id, 0x5555_6666_7777_8888);
        assert_eq!(query.moderator_id, 0x9999_AAAA_BBBB_CCCC);
        assert_eq!(query.event_id, 0xDEAD_BEEF_CAFE_BABE);
    }

    #[test]
    fn calendar_event_sign_up_reads_cpp_field_order_and_tentative_bit() {
        let mut pkt = WorldPacket::new_empty();
        pkt.write_uint64(0x1111_2222_3333_4444);
        pkt.write_uint64(0x5555_6666_7777_8888);
        pkt.write_bit(true);
        pkt.flush_bits();

        let query = CalendarEventSignUp::read(&mut pkt).unwrap();
        assert_eq!(query.event_id, 0x1111_2222_3333_4444);
        assert_eq!(query.club_id, 0x5555_6666_7777_8888);
        assert!(query.tentative);
    }

    #[test]
    fn calendar_invite_reads_cpp_field_order() {
        let mut pkt = WorldPacket::new_empty();
        pkt.write_uint64(0x1111_2222_3333_4444);
        pkt.write_uint64(0x5555_6666_7777_8888);
        pkt.write_uint64(0x9999_AAAA_BBBB_CCCC);
        pkt.write_bits(4, 9);
        pkt.write_bit(false);
        pkt.write_bit(true);
        pkt.flush_bits();
        pkt.write_string("Test");

        let query = CalendarInvite::read(&mut pkt).unwrap();
        assert_eq!(query.event_id, 0x1111_2222_3333_4444);
        assert_eq!(query.moderator_id, 0x5555_6666_7777_8888);
        assert_eq!(query.club_id, 0x9999_AAAA_BBBB_CCCC);
        assert!(!query.creating);
        assert!(query.is_sign_up);
        assert_eq!(query.name, "Test");
    }

    #[test]
    fn calendar_add_event_reads_cpp_field_order_with_invite_optionals() {
        let mut pkt = WorldPacket::new_empty();
        let invite_guid = ObjectGuid::new(0x0102_0304_0506_0708_i64, 0x1112_1314_1516_1718_i64);
        let optional_guid = ObjectGuid::new(0x2122_2324_2526_2728_i64, 0x3132_3334_3536_3738_i64);

        pkt.write_uint64(0x1111_2222_3333_4444);
        pkt.write_uint8(7);
        pkt.write_int32(-1234);
        pkt.write_uint32(0x0102_0304);
        pkt.write_uint32(0x0000_0440);
        pkt.write_uint32(1);
        pkt.write_bits(5, 8);
        pkt.write_bits(4, 11);
        pkt.write_guid(&invite_guid);
        pkt.write_uint8(3);
        pkt.write_uint8(2);
        pkt.write_bit(true);
        pkt.write_bit(true);
        pkt.write_bit(true);
        pkt.write_guid(&optional_guid);
        pkt.write_uint64(0x4142_4344_4546_4748);
        pkt.write_uint64(0x5152_5354_5556_5758);
        pkt.write_string("Title");
        pkt.write_string("Desc");
        pkt.write_uint32(99);

        let query = CalendarAddEvent::read(&mut pkt).unwrap();
        assert_eq!(query.club_id, 0x1111_2222_3333_4444);
        assert_eq!(query.event_type, 7);
        assert_eq!(query.texture_id, -1234);
        assert_eq!(query.time_packed, 0x0102_0304);
        assert_eq!(query.flags, 0x0000_0440);
        assert_eq!(query.title, "Title");
        assert_eq!(query.description, "Desc");
        assert_eq!(query.max_size, 99);
        assert_eq!(query.invites.len(), 1);
        assert_eq!(query.invites[0].guid, invite_guid);
        assert_eq!(query.invites[0].status, 3);
        assert_eq!(query.invites[0].moderator, 2);
        assert_eq!(query.invites[0].unused_801_1, Some(optional_guid));
        assert_eq!(query.invites[0].unused_801_2, Some(0x4142_4344_4546_4748));
        assert_eq!(query.invites[0].unused_801_3, Some(0x5152_5354_5556_5758));
    }

    #[test]
    fn calendar_update_event_reads_cpp_field_order() {
        let mut pkt = WorldPacket::new_empty();
        pkt.write_uint64(0x1111_2222_3333_4444);
        pkt.write_uint64(0x5555_6666_7777_8888);
        pkt.write_uint64(0x9999_AAAA_BBBB_CCCC);
        pkt.write_uint8(7);
        pkt.write_uint32(0x0102_0304);
        pkt.write_uint32(0x0506_0708);
        pkt.write_uint32(0x090A_0B0C);
        pkt.write_bits(5, 8);
        pkt.write_bits(4, 11);
        pkt.flush_bits();
        pkt.write_string("Title");
        pkt.write_string("Desc");
        pkt.write_uint32(99);

        let query = CalendarUpdateEvent::read(&mut pkt).unwrap();
        assert_eq!(query.club_id, 0x1111_2222_3333_4444);
        assert_eq!(query.event_id, 0x5555_6666_7777_8888);
        assert_eq!(query.moderator_id, 0x9999_AAAA_BBBB_CCCC);
        assert_eq!(query.event_type, 7);
        assert_eq!(query.texture_id, 0x0102_0304);
        assert_eq!(query.time_packed, 0x0506_0708);
        assert_eq!(query.flags, 0x090A_0B0C);
        assert_eq!(query.title, "Title");
        assert_eq!(query.description, "Desc");
        assert_eq!(query.max_size, 99);
    }

    #[test]
    fn calendar_rsvp_reads_cpp_field_order() {
        let mut pkt = WorldPacket::new_empty();
        pkt.write_uint64(0x1111_2222_3333_4444);
        pkt.write_uint64(0x5555_6666_7777_8888);
        pkt.write_uint8(9);

        let query = CalendarRsvp::read(&mut pkt).unwrap();
        assert_eq!(query.event_id, 0x1111_2222_3333_4444);
        assert_eq!(query.invite_id, 0x5555_6666_7777_8888);
        assert_eq!(query.status, 9);
    }

    #[test]
    fn calendar_status_reads_cpp_field_order() {
        let guid = ObjectGuid::new(0x0102_0304_0506_0708, 0x1111_2222_3333_4444);
        let mut pkt = WorldPacket::new_empty();
        pkt.write_guid(&guid);
        pkt.write_uint64(0x5555_6666_7777_8888);
        pkt.write_uint64(0x9999_AAAA_BBBB_CCCC);
        pkt.write_uint64(0xDEAD_BEEF_CAFE_BABE);
        pkt.write_uint8(9);

        let query = CalendarStatus::read(&mut pkt).unwrap();
        assert_eq!(query.guid, guid);
        assert_eq!(query.event_id, 0x5555_6666_7777_8888);
        assert_eq!(query.invite_id, 0x9999_AAAA_BBBB_CCCC);
        assert_eq!(query.moderator_id, 0xDEAD_BEEF_CAFE_BABE);
        assert_eq!(query.status, 9);
    }

    #[test]
    fn calendar_moderator_status_query_reads_cpp_field_order() {
        let guid = ObjectGuid::new(0x0102_0304_0506_0708, 0x1111_2222_3333_4444);
        let mut pkt = WorldPacket::new_empty();
        pkt.write_guid(&guid);
        pkt.write_uint64(0x5555_6666_7777_8888);
        pkt.write_uint64(0x9999_AAAA_BBBB_CCCC);
        pkt.write_uint64(0xDEAD_BEEF_CAFE_BABE);
        pkt.write_uint8(9);

        let query = CalendarModeratorStatusQuery::read(&mut pkt).unwrap();
        assert_eq!(query.guid, guid);
        assert_eq!(query.event_id, 0x5555_6666_7777_8888);
        assert_eq!(query.invite_id, 0x9999_AAAA_BBBB_CCCC);
        assert_eq!(query.moderator_id, 0xDEAD_BEEF_CAFE_BABE);
        assert_eq!(query.status, 9);
    }

    #[test]
    fn calendar_command_result_event_invalid_matches_cpp_shape() {
        let bytes = CalendarCommandResult::event_invalid_like_cpp().to_bytes();
        assert_eq!(
            u16::from_le_bytes([bytes[0], bytes[1]]),
            ServerOpcodes::CalendarCommandResult as u16
        );
        assert_eq!(bytes.len(), 2 + 4);

        let mut pkt = WorldPacket::from_bytes(&bytes[2..]);
        assert_eq!(pkt.read_uint8().unwrap(), 1);
        assert_eq!(pkt.read_uint8().unwrap(), 6);
        assert_eq!(pkt.read_bits(9).unwrap(), 0);
    }

    #[test]
    fn calendar_command_result_no_invite_matches_cpp_shape() {
        let bytes = CalendarCommandResult::no_invite_like_cpp().to_bytes();
        assert_eq!(
            u16::from_le_bytes([bytes[0], bytes[1]]),
            ServerOpcodes::CalendarCommandResult as u16
        );
        assert_eq!(bytes.len(), 2 + 4);

        let mut pkt = WorldPacket::from_bytes(&bytes[2..]);
        assert_eq!(pkt.read_uint8().unwrap(), 1);
        assert_eq!(pkt.read_uint8().unwrap(), 29);
        assert_eq!(pkt.read_bits(9).unwrap(), 0);
    }

    #[test]
    fn calendar_complain_reads_cpp_guid_event_invite_order() {
        let invited_by_guid = ObjectGuid::create_player(1, 0xAABB_CCDD);
        let mut pkt = WorldPacket::new_empty();
        pkt.write_guid(&invited_by_guid);
        pkt.write_uint64(0x0102_0304_0506_0708);
        pkt.write_uint64(0x1112_1314_1516_1718);

        let complain = CalendarComplain::read(&mut pkt).unwrap();
        assert_eq!(complain.invited_by_guid, invited_by_guid);
        assert_eq!(complain.event_id, 0x0102_0304_0506_0708);
        assert_eq!(complain.invite_id, 0x1112_1314_1516_1718);
    }

    #[test]
    fn arena_team_roster_reads_cpp_team_id() {
        let mut pkt = WorldPacket::new_empty();
        pkt.write_uint32(0x0102_0304);

        let request = ArenaTeamRoster::read(&mut pkt).unwrap();
        assert_eq!(request.team_id, 0x0102_0304);
    }

    #[test]
    fn arena_team_decline_reads_empty_cpp_packet() {
        let mut pkt = WorldPacket::new_empty();

        ArenaTeamDecline::read(&mut pkt).unwrap();
    }

    #[test]
    fn arena_team_accept_reads_empty_cpp_packet() {
        let mut pkt = WorldPacket::new_empty();

        ArenaTeamAccept::read(&mut pkt).unwrap();
    }

    #[test]
    fn arena_team_leave_reads_empty_cpp_packet() {
        let mut pkt = WorldPacket::new_empty();

        ArenaTeamLeave::read(&mut pkt).unwrap();
    }

    #[test]
    fn arena_team_remove_reads_team_id_and_9bit_target_name_like_cpp() {
        let mut pkt = WorldPacket::new_empty();
        pkt.write_uint32(0x0102_0304);
        pkt.write_bits(7, 9);
        pkt.write_string("Playerx");
        pkt.reset_read();

        let request = ArenaTeamRemove::read(&mut pkt).unwrap();

        assert_eq!(request.team_id, 0x0102_0304);
        assert_eq!(request.target_name, "Playerx");
    }

    #[test]
    fn arena_team_disband_reads_team_id_like_cpp() {
        let mut pkt = WorldPacket::new_empty();
        pkt.write_uint32(0x1122_3344);
        pkt.reset_read();

        let request = ArenaTeamDisband::read(&mut pkt).unwrap();

        assert_eq!(request.team_id, 0x1122_3344);
    }

    #[test]
    fn arena_team_leader_reads_team_id_and_9bit_target_name_like_cpp() {
        let mut pkt = WorldPacket::new_empty();
        pkt.write_uint32(0x5566_7788);
        pkt.write_bits(6, 9);
        pkt.write_string("Leader");
        pkt.reset_read();

        let request = ArenaTeamLeader::read(&mut pkt).unwrap();

        assert_eq!(request.team_id, 0x5566_7788);
        assert_eq!(request.target_name, "Leader");
    }

    #[test]
    fn query_arena_team_reads_team_id_like_cpp() {
        let mut pkt = WorldPacket::new_empty();
        pkt.write_uint32(0xAABB_CCDD);
        pkt.reset_read();

        let request = QueryArenaTeam::read(&mut pkt).unwrap();

        assert_eq!(request.team_id, 0xAABB_CCDD);
    }

    #[test]
    fn busy_trade_reads_empty_cpp_packet() {
        let mut pkt = WorldPacket::new_empty();

        BusyTrade::read(&mut pkt).unwrap();
    }

    #[test]
    fn begin_trade_reads_empty_cpp_packet() {
        let mut pkt = WorldPacket::new_empty();

        BeginTrade::read(&mut pkt).unwrap();
    }

    #[test]
    fn accept_trade_reads_state_index_like_cpp() {
        let mut pkt = WorldPacket::new_empty();
        pkt.write_uint32(0x1122_3344);
        pkt.reset_read();

        let packet = AcceptTrade::read(&mut pkt).unwrap();

        assert_eq!(packet.state_index, 0x1122_3344);
    }

    #[test]
    fn clear_trade_item_reads_trade_slot_like_cpp() {
        let mut pkt = WorldPacket::new_empty();
        pkt.write_uint8(5);
        pkt.reset_read();

        let packet = ClearTradeItem::read(&mut pkt).unwrap();

        assert_eq!(packet.trade_slot, 5);
    }

    #[test]
    fn set_trade_item_reads_slots_like_cpp() {
        let mut pkt = WorldPacket::new_empty();
        pkt.write_uint8(2);
        pkt.write_uint8(255);
        pkt.write_uint8(18);
        pkt.reset_read();

        let packet = SetTradeItem::read(&mut pkt).unwrap();

        assert_eq!(packet.trade_slot, 2);
        assert_eq!(packet.pack_slot, 255);
        assert_eq!(packet.item_slot_in_pack, 18);
    }

    #[test]
    fn set_trade_spell_reads_spell_and_slots_like_cpp() {
        let mut pkt = WorldPacket::new_empty();
        pkt.write_uint32(7418);
        pkt.write_uint8(255);
        pkt.write_uint8(23);
        pkt.reset_read();

        let packet = SetTradeSpell::read(&mut pkt).unwrap();

        assert_eq!(packet.spell_id, 7418);
        assert_eq!(packet.pack_slot, 255);
        assert_eq!(packet.item_slot_in_pack, 23);
    }

    #[test]
    fn sign_petition_reads_guid_and_choice_like_cpp() {
        let petition_guid = ObjectGuid::create_item(1, 0x0102_0304_0506_0708);
        let mut pkt = WorldPacket::new_empty();
        pkt.write_bytes(&petition_guid.to_raw_bytes());
        pkt.write_uint8(1);
        pkt.reset_read();

        let packet = SignPetition::read(&mut pkt).unwrap();

        assert_eq!(packet.petition_guid, petition_guid);
        assert_eq!(packet.choice, 1);
    }

    #[test]
    fn decline_petition_reads_guid_like_cpp() {
        let petition_guid = ObjectGuid::create_item(1, 0x1112_1314_1516_1718);
        let mut pkt = WorldPacket::new_empty();
        pkt.write_bytes(&petition_guid.to_raw_bytes());
        pkt.reset_read();

        let packet = DeclinePetition::read(&mut pkt).unwrap();

        assert_eq!(packet.petition_guid, petition_guid);
    }

    #[test]
    fn query_petition_reads_id_then_guid_like_cpp() {
        let item_guid = ObjectGuid::create_item(1, 0x2122_2324_2526_2728);
        let mut pkt = WorldPacket::new_empty();
        pkt.write_uint32(0x1122_3344);
        pkt.write_bytes(&item_guid.to_raw_bytes());
        pkt.reset_read();

        let packet = QueryPetition::read(&mut pkt).unwrap();

        assert_eq!(packet.petition_id, 0x1122_3344);
        assert_eq!(packet.item_guid, item_guid);
    }

    #[test]
    fn query_petition_not_found_response_writes_id_and_allow_false_like_cpp() {
        let item_guid = ObjectGuid::create_item(1, 0x3132_3334_3536_3738);
        let bytes = QueryPetitionResponse::not_found_like_cpp(item_guid).to_bytes();
        let mut body = WorldPacket::from_bytes(&bytes);

        assert_eq!(
            body.server_opcode(),
            Some(ServerOpcodes::QueryPetitionResponse)
        );
        assert_eq!(
            body.read_uint16().unwrap(),
            ServerOpcodes::QueryPetitionResponse as u16
        );
        assert_eq!(body.read_uint32().unwrap(), item_guid.counter() as u32);
        assert!(!body.read_bit().unwrap());
        assert_eq!(body.remaining(), 0);
    }

    #[test]
    fn set_trade_gold_reads_coinage_like_cpp() {
        let mut pkt = WorldPacket::new_empty();
        pkt.write_uint64(0x1122_3344_5566_7788);
        pkt.reset_read();

        let packet = SetTradeGold::read(&mut pkt).unwrap();

        assert_eq!(packet.coinage, 0x1122_3344_5566_7788);
    }

    #[test]
    fn unaccept_trade_reads_empty_cpp_packet() {
        let mut pkt = WorldPacket::new_empty();

        UnacceptTrade::read(&mut pkt).unwrap();
    }

    #[test]
    fn ignore_trade_reads_empty_cpp_packet() {
        let mut pkt = WorldPacket::new_empty();

        IgnoreTrade::read(&mut pkt).unwrap();
    }

    #[test]
    fn trade_status_player_busy_writes_cancel_status_bits_like_cpp() {
        let bytes = TradeStatus::cancel_like_cpp(TRADE_STATUS_PLAYER_BUSY_LIKE_CPP).to_bytes();

        assert_eq!(
            u16::from_le_bytes([bytes[0], bytes[1]]),
            ServerOpcodes::TradeStatus as u16
        );
        assert_eq!(bytes.len(), 3);
        assert_eq!(bytes[2], TRADE_STATUS_PLAYER_BUSY_LIKE_CPP << 1);
    }

    #[test]
    fn trade_status_initiated_writes_id_payload_like_cpp() {
        let bytes = TradeStatus::initiated_like_cpp(0x1122_3344).to_bytes();

        assert_eq!(
            u16::from_le_bytes([bytes[0], bytes[1]]),
            ServerOpcodes::TradeStatus as u16
        );
        assert_eq!(bytes.len(), 7);
        assert_eq!(bytes[2], TRADE_STATUS_INITIATED_LIKE_CPP << 2);
        assert_eq!(
            u32::from_le_bytes([bytes[3], bytes[4], bytes[5], bytes[6]]),
            0x1122_3344
        );
    }

    #[test]
    fn trade_status_failed_writes_bag_result_like_cpp() {
        let bytes = TradeStatus::failed_like_cpp(EQUIP_ERR_NOT_ENOUGH_MONEY_LIKE_CPP, 0).to_bytes();

        assert_eq!(
            u16::from_le_bytes([bytes[0], bytes[1]]),
            ServerOpcodes::TradeStatus as u16
        );
        assert_eq!(bytes.len(), 11);
        assert_eq!(bytes[2], TRADE_STATUS_FAILED_LIKE_CPP << 2);
        assert_eq!(
            i32::from_le_bytes([bytes[3], bytes[4], bytes[5], bytes[6]]),
            EQUIP_ERR_NOT_ENOUGH_MONEY_LIKE_CPP
        );
        assert_eq!(
            i32::from_le_bytes([bytes[7], bytes[8], bytes[9], bytes[10]]),
            0
        );
    }

    #[test]
    fn trade_status_cancelled_writes_cancel_status_bits_like_cpp() {
        let bytes = TradeStatus::cancel_like_cpp(TRADE_STATUS_CANCELLED_LIKE_CPP).to_bytes();

        assert_eq!(
            u16::from_le_bytes([bytes[0], bytes[1]]),
            ServerOpcodes::TradeStatus as u16
        );
        assert_eq!(bytes.len(), 3);
        assert_eq!(bytes[2], TRADE_STATUS_CANCELLED_LIKE_CPP << 2);
    }

    #[test]
    fn trade_status_player_ignored_writes_cancel_status_bits_like_cpp() {
        let bytes = TradeStatus::cancel_like_cpp(TRADE_STATUS_PLAYER_IGNORED_LIKE_CPP).to_bytes();

        assert_eq!(
            u16::from_le_bytes([bytes[0], bytes[1]]),
            ServerOpcodes::TradeStatus as u16
        );
        assert_eq!(bytes.len(), 3);
        assert_eq!(bytes[2], TRADE_STATUS_PLAYER_IGNORED_LIKE_CPP << 2);
    }

    #[test]
    fn guild_bank_remaining_withdraw_money_matches_cpp_shape() {
        let bytes = GuildBankRemainingWithdrawMoney {
            remaining_withdraw_money: 123_456_789,
        }
        .to_bytes();
        assert_eq!(
            u16::from_le_bytes([bytes[0], bytes[1]]),
            ServerOpcodes::GuildBankRemainingWithdrawMoney as u16
        );
        assert_eq!(bytes.len(), 2 + 8);

        let mut pkt = WorldPacket::from_bytes(&bytes[2..]);
        assert_eq!(pkt.read_int64().unwrap(), 123_456_789);
    }

    #[test]
    fn commerce_token_get_log_reads_cpp_uint32() {
        let mut pkt = WorldPacket::new_empty();
        pkt.write_uint32(0x1122_3344);

        let request = CommerceTokenGetLog::read(&mut pkt).unwrap();
        assert_eq!(request.unk_int, 0x1122_3344);
    }

    #[test]
    fn auctionable_token_sell_reads_empty_stub_like_cpp_wotlk() {
        let mut pkt = WorldPacket::new_empty();

        let request = AuctionableTokenSell::read(&mut pkt).unwrap();
        assert_eq!(request, AuctionableTokenSell);
    }

    #[test]
    fn auction_list_items_reads_empty_legacy_packet_like_cpp() {
        let mut pkt = WorldPacket::new_empty();

        let request = AuctionListItems::read(&mut pkt).unwrap();
        assert_eq!(request, AuctionListItems);
        assert_eq!(pkt.remaining(), 0);
    }

    #[test]
    fn auction_replicate_items_reads_no_tainted_by_like_cpp() {
        let auctioneer =
            ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 9_001, 7);
        let mut pkt = WorldPacket::new_empty();
        pkt.write_guid(&auctioneer);
        pkt.write_uint32(11);
        pkt.write_uint32(22);
        pkt.write_uint32(33);
        pkt.write_uint32(44);
        pkt.write_bit(false);
        pkt.flush_bits();
        pkt.reset_read();

        let request = AuctionReplicateItems::read(&mut pkt).unwrap();
        assert_eq!(request.auctioneer, auctioneer);
        assert_eq!(request.change_number_global, 11);
        assert_eq!(request.change_number_cursor, 22);
        assert_eq!(request.change_number_tombstone, 33);
        assert_eq!(request.count, 44);
        assert!(request.tainted_by.is_none());
        assert_eq!(pkt.remaining(), 0);
    }

    #[test]
    fn auction_replicate_items_reads_tainted_by_like_cpp() {
        let auctioneer =
            ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 9_002, 8);
        let mut pkt = WorldPacket::new_empty();
        pkt.write_guid(&auctioneer);
        pkt.write_uint32(1);
        pkt.write_uint32(2);
        pkt.write_uint32(3);
        pkt.write_uint32(4);
        pkt.write_bit(true);
        pkt.flush_bits();
        pkt.write_bits(6, 10); // "Trade" + '\0'
        pkt.write_bits(4, 10); // "1.0" + '\0'
        pkt.write_bit(true);
        pkt.write_bit(false);
        pkt.flush_bits();
        pkt.write_string("Trade");
        pkt.write_uint8(0);
        pkt.write_string("1.0");
        pkt.write_uint8(0);
        pkt.reset_read();

        let request = AuctionReplicateItems::read(&mut pkt).unwrap();
        assert_eq!(request.auctioneer, auctioneer);
        assert_eq!(request.change_number_global, 1);
        assert_eq!(request.change_number_cursor, 2);
        assert_eq!(request.change_number_tombstone, 3);
        assert_eq!(request.count, 4);
        assert_eq!(
            request.tainted_by,
            Some(AuctionAddonInfo {
                name: "Trade".to_string(),
                version: "1.0".to_string(),
                loaded: true,
                disabled: false,
            })
        );
        assert_eq!(pkt.remaining(), 0);
    }

    #[test]
    fn auction_place_bid_reads_no_tainted_by_like_cpp() {
        let auctioneer =
            ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 9_003, 9);
        let mut pkt = WorldPacket::new_empty();
        pkt.write_guid(&auctioneer);
        pkt.write_int32(1234);
        pkt.write_uint64(12_300);
        pkt.write_bit(false);
        pkt.flush_bits();
        pkt.reset_read();

        let request = AuctionPlaceBid::read(&mut pkt).unwrap();
        assert_eq!(request.auctioneer, auctioneer);
        assert_eq!(request.auction_id, 1234);
        assert_eq!(request.bid_amount, 12_300);
        assert!(request.tainted_by.is_none());
        assert_eq!(pkt.remaining(), 0);
    }

    #[test]
    fn auction_place_bid_reads_tainted_by_like_cpp() {
        let auctioneer =
            ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 9_004, 10);
        let mut pkt = WorldPacket::new_empty();
        pkt.write_guid(&auctioneer);
        pkt.write_int32(5678);
        pkt.write_uint64(45_600);
        pkt.write_bit(true);
        pkt.flush_bits();
        pkt.write_bits(6, 10); // "Trade" + '\0'
        pkt.write_bits(4, 10); // "1.0" + '\0'
        pkt.write_bit(true);
        pkt.write_bit(false);
        pkt.flush_bits();
        pkt.write_string("Trade");
        pkt.write_uint8(0);
        pkt.write_string("1.0");
        pkt.write_uint8(0);
        pkt.reset_read();

        let request = AuctionPlaceBid::read(&mut pkt).unwrap();
        assert_eq!(request.auctioneer, auctioneer);
        assert_eq!(request.auction_id, 5678);
        assert_eq!(request.bid_amount, 45_600);
        assert_eq!(
            request.tainted_by,
            Some(AuctionAddonInfo {
                name: "Trade".to_string(),
                version: "1.0".to_string(),
                loaded: true,
                disabled: false,
            })
        );
        assert_eq!(pkt.remaining(), 0);
    }

    #[test]
    fn auction_remove_item_reads_no_tainted_by_like_cpp() {
        let auctioneer =
            ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 9_005, 11);
        let mut pkt = WorldPacket::new_empty();
        pkt.write_guid(&auctioneer);
        pkt.write_int32(1234);
        pkt.write_int32(19019);
        pkt.write_bit(false);
        pkt.flush_bits();
        pkt.reset_read();

        let request = AuctionRemoveItem::read(&mut pkt).unwrap();
        assert_eq!(request.auctioneer, auctioneer);
        assert_eq!(request.auction_id, 1234);
        assert_eq!(request.item_id, 19019);
        assert!(request.tainted_by.is_none());
        assert_eq!(pkt.remaining(), 0);
    }

    #[test]
    fn auction_remove_item_reads_tainted_by_like_cpp() {
        let auctioneer =
            ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 9_006, 12);
        let mut pkt = WorldPacket::new_empty();
        pkt.write_guid(&auctioneer);
        pkt.write_int32(5678);
        pkt.write_int32(4306);
        pkt.write_bit(true);
        pkt.flush_bits();
        pkt.write_bits(6, 10); // "Trade" + '\0'
        pkt.write_bits(4, 10); // "1.0" + '\0'
        pkt.write_bit(true);
        pkt.write_bit(false);
        pkt.flush_bits();
        pkt.write_string("Trade");
        pkt.write_uint8(0);
        pkt.write_string("1.0");
        pkt.write_uint8(0);
        pkt.reset_read();

        let request = AuctionRemoveItem::read(&mut pkt).unwrap();
        assert_eq!(request.auctioneer, auctioneer);
        assert_eq!(request.auction_id, 5678);
        assert_eq!(request.item_id, 4306);
        assert_eq!(
            request.tainted_by,
            Some(AuctionAddonInfo {
                name: "Trade".to_string(),
                version: "1.0".to_string(),
                loaded: true,
                disabled: false,
            })
        );
        assert_eq!(pkt.remaining(), 0);
    }

    #[test]
    fn auction_sell_item_reads_single_item_no_tainted_by_like_cpp() {
        let auctioneer =
            ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 9_007, 13);
        let item_guid = ObjectGuid::create_item(1, 19_019);
        let mut pkt = WorldPacket::new_empty();
        pkt.write_guid(&auctioneer);
        pkt.write_uint64(10_000);
        pkt.write_uint64(25_000);
        pkt.write_uint32(720);
        pkt.write_bit(false);
        pkt.write_bits(1, 6);
        pkt.flush_bits();
        pkt.write_guid(&item_guid);
        pkt.write_uint32(1);
        pkt.reset_read();

        let request = AuctionSellItem::read(&mut pkt).unwrap();
        assert_eq!(request.auctioneer, auctioneer);
        assert_eq!(request.min_bid, 10_000);
        assert_eq!(request.buyout_price, 25_000);
        assert_eq!(request.runtime, 720);
        assert!(request.tainted_by.is_none());
        assert_eq!(
            request.items,
            vec![AuctionItemForSale {
                guid: item_guid,
                use_count: 1,
            }]
        );
        assert_eq!(pkt.remaining(), 0);
    }

    #[test]
    fn auction_sell_item_reads_tainted_by_like_cpp() {
        let auctioneer =
            ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 9_008, 14);
        let item_guid = ObjectGuid::create_item(1, 43_006);
        let mut pkt = WorldPacket::new_empty();
        pkt.write_guid(&auctioneer);
        pkt.write_uint64(20_000);
        pkt.write_uint64(50_000);
        pkt.write_uint32(1440);
        pkt.write_bit(true);
        pkt.write_bits(1, 6);
        pkt.flush_bits();
        pkt.write_bits(6, 10); // "Trade" + '\0'
        pkt.write_bits(4, 10); // "1.0" + '\0'
        pkt.write_bit(true);
        pkt.write_bit(false);
        pkt.flush_bits();
        pkt.write_string("Trade");
        pkt.write_uint8(0);
        pkt.write_string("1.0");
        pkt.write_uint8(0);
        pkt.write_guid(&item_guid);
        pkt.write_uint32(1);
        pkt.reset_read();

        let request = AuctionSellItem::read(&mut pkt).unwrap();
        assert_eq!(request.auctioneer, auctioneer);
        assert_eq!(request.min_bid, 20_000);
        assert_eq!(request.buyout_price, 50_000);
        assert_eq!(request.runtime, 1440);
        assert_eq!(
            request.tainted_by,
            Some(AuctionAddonInfo {
                name: "Trade".to_string(),
                version: "1.0".to_string(),
                loaded: true,
                disabled: false,
            })
        );
        assert_eq!(
            request.items,
            vec![AuctionItemForSale {
                guid: item_guid,
                use_count: 1,
            }]
        );
        assert_eq!(pkt.remaining(), 0);
    }

    #[test]
    fn auctionable_token_sell_at_market_price_reads_empty_stub_like_cpp_wotlk() {
        let mut pkt = WorldPacket::new_empty();

        let request = AuctionableTokenSellAtMarketPrice::read(&mut pkt).unwrap();
        assert_eq!(request, AuctionableTokenSellAtMarketPrice);
    }

    #[test]
    fn commerce_token_get_log_response_success_empty_matches_cpp_todo_handler() {
        let bytes = CommerceTokenGetLogResponse::success_empty(0x1122_3344).to_bytes();
        assert_eq!(
            u16::from_le_bytes([bytes[0], bytes[1]]),
            ServerOpcodes::CommerceTokenGetLogResponse as u16
        );
        assert_eq!(bytes.len(), 2 + 12);

        let mut pkt = WorldPacket::from_bytes(&bytes[2..]);
        assert_eq!(pkt.read_uint32().unwrap(), 0x1122_3344);
        assert_eq!(pkt.read_uint32().unwrap(), TOKEN_RESULT_SUCCESS_LIKE_CPP);
        assert_eq!(pkt.read_uint32().unwrap(), 0);
    }

    #[test]
    fn tutorial_flags_all_shown() {
        let pkt = TutorialFlags::all_shown();
        let bytes = pkt.to_bytes();
        // opcode(2) + 8*u32(32) = 34
        assert_eq!(bytes.len(), 34);
    }

    #[test]
    fn feature_system_status_serializes() {
        let pkt = FeatureSystemStatus::default_wotlk();
        let bytes = pkt.to_bytes();
        assert!(bytes.len() > 20);
        // Verify opcode is FeatureSystemStatus (0x25bf)
        let opcode = u16::from_le_bytes([bytes[0], bytes[1]]);
        assert_eq!(opcode, 0x25bf);
    }

    #[test]
    fn feature_system_status_glue_screen_serializes() {
        let pkt = FeatureSystemStatusGlueScreen::default_wotlk();
        let bytes = pkt.to_bytes();
        assert!(bytes.len() > 20);
        // Verify opcode is FeatureSystemStatusGlueScreen (0x25c0)
        let opcode = u16::from_le_bytes([bytes[0], bytes[1]]);
        assert_eq!(opcode, 0x25c0);
    }

    #[test]
    fn transfer_aborted_matches_cpp_layout() {
        let bytes = TransferAborted {
            map_id: 571,
            arg: 0,
            map_difficulty_x_condition_id: 0,
            transfer_abort: 16,
        }
        .to_bytes();

        assert_eq!(bytes.len(), 12);
        assert_eq!(u16::from_le_bytes([bytes[0], bytes[1]]), 0x2703);
        assert_eq!(&bytes[2..6], &571u32.to_le_bytes());
        assert_eq!(bytes[6], 0);
        assert_eq!(&bytes[7..11], &0i32.to_le_bytes());
        assert_eq!(bytes[11], 0x40);
    }

    #[test]
    fn client_cache_version_serializes() {
        let pkt = ClientCacheVersion { cache_version: 42 };
        let bytes = pkt.to_bytes();
        // opcode(2) + uint32(4) = 6
        assert_eq!(bytes.len(), 6);
        let opcode = u16::from_le_bytes([bytes[0], bytes[1]]);
        assert_eq!(opcode, 0x291c);
    }

    #[test]
    fn phase_shift_change_default_matches_cpp_empty_layout() {
        let pkt = PhaseShiftChange::default_for(ObjectGuid::EMPTY);
        let mut body = crate::WorldPacket::new_empty();
        pkt.write(&mut body);
        let bytes = body.into_data();

        assert_eq!(bytes.len(), 24);
        assert_eq!(&bytes[0..2], &[0, 0]); // packed Client GUID
        assert_eq!(u32::from_le_bytes(bytes[2..6].try_into().unwrap()), 0x08);
        assert_eq!(u32::from_le_bytes(bytes[6..10].try_into().unwrap()), 0);
        assert_eq!(&bytes[10..12], &[0, 0]); // packed PersonalGUID
        assert_eq!(u32::from_le_bytes(bytes[12..16].try_into().unwrap()), 0);
        assert_eq!(u32::from_le_bytes(bytes[16..20].try_into().unwrap()), 0);
        assert_eq!(u32::from_le_bytes(bytes[20..24].try_into().unwrap()), 0);
    }

    #[test]
    fn phase_shift_change_visible_map_ids_use_cpp_byte_size_prefix() {
        let pkt = PhaseShiftChange::with_visible_map_ids(ObjectGuid::EMPTY, vec![609, 700]);
        let mut body = crate::WorldPacket::new_empty();
        pkt.write(&mut body);
        let bytes = body.into_data();

        assert_eq!(bytes.len(), 28);
        assert_eq!(u32::from_le_bytes(bytes[12..16].try_into().unwrap()), 4);
        assert_eq!(u16::from_le_bytes(bytes[16..18].try_into().unwrap()), 609);
        assert_eq!(u16::from_le_bytes(bytes[18..20].try_into().unwrap()), 700);
        assert_eq!(u32::from_le_bytes(bytes[20..24].try_into().unwrap()), 0);
        assert_eq!(u32::from_le_bytes(bytes[24..28].try_into().unwrap()), 0);
    }

    #[test]
    fn available_hotfixes_empty_serializes() {
        let pkt = AvailableHotfixes {
            virtual_realm_address: 1,
            hotfixes: Vec::new(),
        };
        let bytes = pkt.to_bytes();
        // opcode(2) + uint32(4) + int32(4) = 10
        assert_eq!(bytes.len(), 10);
        let opcode = u16::from_le_bytes([bytes[0], bytes[1]]);
        assert_eq!(opcode, 0x290f);
    }

    #[test]
    fn available_hotfixes_serializes_ids() {
        let pkt = AvailableHotfixes {
            virtual_realm_address: 0x1122_3344,
            hotfixes: vec![HotfixId {
                push_id: 7,
                unique_id: 9,
            }],
        };
        let bytes = pkt.to_bytes();
        assert_eq!(bytes.len(), 18);
        assert_eq!(
            u32::from_le_bytes([bytes[2], bytes[3], bytes[4], bytes[5]]),
            0x1122_3344
        );
        assert_eq!(
            u32::from_le_bytes([bytes[6], bytes[7], bytes[8], bytes[9]]),
            1
        );
        assert_eq!(
            i32::from_le_bytes([bytes[10], bytes[11], bytes[12], bytes[13]]),
            7
        );
        assert_eq!(
            u32::from_le_bytes([bytes[14], bytes[15], bytes[16], bytes[17]]),
            9
        );
    }

    #[test]
    fn connection_status_serializes() {
        let pkt = ConnectionStatus {
            state: 1,
            suppress_notification: true,
        };
        let bytes = pkt.to_bytes();
        // opcode(2) + 3 bits flushed to 1 byte = 3
        assert_eq!(bytes.len(), 3);
        let opcode = u16::from_le_bytes([bytes[0], bytes[1]]);
        assert_eq!(opcode, 0x2809);
    }

    #[test]
    fn set_timezone_utc() {
        let pkt = SetTimeZoneInformation::utc();
        let bytes = pkt.to_bytes();
        // Should contain "Etc/UTC" x3
        assert!(bytes.len() > 20);
    }

    #[test]
    fn login_set_time_speed_now() {
        let pkt = LoginSetTimeSpeed::now();
        let bytes = pkt.to_bytes();
        // opcode(2) + 4*i32(16) + float(4) = 22
        assert_eq!(bytes.len(), 22);
    }

    #[test]
    fn setup_currency_empty() {
        let pkt = SetupCurrency::empty();
        let bytes = pkt.to_bytes();
        // opcode(2) + i32(4) = 6
        assert_eq!(bytes.len(), 6);
    }

    #[test]
    fn setup_currency_record_matches_cpp_bit_and_field_order() {
        let pkt = SetupCurrency::from_records(vec![SetupCurrencyRecord {
            type_id: 395,
            quantity: 123,
            weekly_quantity: Some(20),
            max_weekly_quantity: Some(50),
            tracked_quantity: Some(7),
            max_quantity: Some(200),
            total_earned: Some(300),
            next_recharge_time: None,
            recharge_cycle_start_time: None,
            flags: 0x0c,
        }]);
        let bytes = pkt.to_bytes();
        let mut body = WorldPacket::from_bytes(&bytes);
        assert_eq!(body.read_uint16().unwrap(), 0x2573);
        assert_eq!(body.read_uint32().unwrap(), 1);
        assert_eq!(body.read_int32().unwrap(), 395);
        assert_eq!(body.read_int32().unwrap(), 123);
        assert!(body.read_bit().unwrap());
        assert!(body.read_bit().unwrap());
        assert!(body.read_bit().unwrap());
        assert!(body.read_bit().unwrap());
        assert!(body.read_bit().unwrap());
        assert!(!body.read_bit().unwrap());
        assert!(!body.read_bit().unwrap());
        assert_eq!(body.read_bits(5).unwrap(), 0x0c);
        assert_eq!(body.read_uint32().unwrap(), 20);
        assert_eq!(body.read_uint32().unwrap(), 50);
        assert_eq!(body.read_uint32().unwrap(), 7);
        assert_eq!(body.read_int32().unwrap(), 200);
        assert_eq!(body.read_int32().unwrap(), 300);
    }

    #[test]
    fn set_currency_vendor_loss_matches_cpp_field_order() {
        let pkt = SetCurrency::vendor_loss(395, 90, 10);
        let bytes = pkt.to_bytes();
        assert_eq!(bytes.len(), 28);
        assert_eq!(u16::from_le_bytes([bytes[0], bytes[1]]), 0x2574);
        assert_eq!(i32::from_le_bytes(bytes[2..6].try_into().unwrap()), 395);
        assert_eq!(i32::from_le_bytes(bytes[6..10].try_into().unwrap()), 90);
        assert_eq!(u32::from_le_bytes(bytes[10..14].try_into().unwrap()), 0);
        assert_eq!(u32::from_le_bytes(bytes[14..18].try_into().unwrap()), 0);
        assert_eq!(bytes[18], 0x05);
        assert_eq!(bytes[19], 0x00);
        assert_eq!(i32::from_le_bytes(bytes[20..24].try_into().unwrap()), -10);
        assert_eq!(i32::from_le_bytes(bytes[24..28].try_into().unwrap()), 4);
    }

    #[test]
    fn set_currency_vendor_gain_matches_cpp_source() {
        let pkt = SetCurrency::vendor_gain(395, 110, 10);
        let bytes = pkt.to_bytes();
        assert_eq!(bytes.len(), 28);
        assert_eq!(u16::from_le_bytes([bytes[0], bytes[1]]), 0x2574);
        assert_eq!(i32::from_le_bytes(bytes[2..6].try_into().unwrap()), 395);
        assert_eq!(i32::from_le_bytes(bytes[6..10].try_into().unwrap()), 110);
        assert_eq!(u32::from_le_bytes(bytes[10..14].try_into().unwrap()), 0);
        assert_eq!(u32::from_le_bytes(bytes[14..18].try_into().unwrap()), 0);
        assert_eq!(bytes[18], 0x06);
        assert_eq!(bytes[19], 0x00);
        assert_eq!(i32::from_le_bytes(bytes[20..24].try_into().unwrap()), 10);
        assert_eq!(i32::from_le_bytes(bytes[24..28].try_into().unwrap()), 5);
    }

    #[test]
    fn set_currency_item_refund_gain_matches_cpp_source() {
        let pkt = SetCurrency::item_refund_gain(395, 110, 10, None, None, None, false);
        let bytes = pkt.to_bytes();
        assert_eq!(bytes.len(), 28);
        assert_eq!(u16::from_le_bytes([bytes[0], bytes[1]]), 0x2574);
        assert_eq!(i32::from_le_bytes(bytes[2..6].try_into().unwrap()), 395);
        assert_eq!(i32::from_le_bytes(bytes[6..10].try_into().unwrap()), 110);
        assert_eq!(bytes[18], 0x06);
        assert_eq!(bytes[19], 0x00);
        assert_eq!(i32::from_le_bytes(bytes[20..24].try_into().unwrap()), 10);
        assert_eq!(i32::from_le_bytes(bytes[24..28].try_into().unwrap()), 2);
    }

    #[test]
    fn init_world_states_empty() {
        let pkt = InitWorldStates::new(0, 12);
        let bytes = pkt.to_bytes();
        // opcode(2) + 4*i32(16) = 18
        assert_eq!(bytes.len(), 18);
        let opcode = u16::from_le_bytes([bytes[0], bytes[1]]);
        assert_eq!(opcode, 0x2746);
    }

    #[test]
    fn update_talent_data_empty() {
        let pkt = UpdateTalentData;
        let bytes = pkt.to_bytes();
        // opcode(2) + int32(4) + uint8(1) + int32(4) +
        // TalentGroupInfo: uint8(1)+uint32(4)+uint8(1)+uint32(4)+uint8(1)+6*uint16(12) +
        // bit(IsPetTalents) flushed to 1 byte = 2+4+1+4+1+4+1+4+1+12+1 = 35
        assert_eq!(bytes.len(), 35);
        let opcode = u16::from_le_bytes([bytes[0], bytes[1]]);
        assert_eq!(opcode, 0x25d7);
    }

    #[test]
    fn send_known_spells_empty() {
        let pkt = SendKnownSpells::empty();
        let bytes = pkt.to_bytes();
        // opcode(2) + bit(flush)+int32(4)+int32(4) = 2+1+4+4 = 11
        assert_eq!(bytes.len(), 11);
    }

    #[test]
    fn send_known_spells_with_data() {
        let pkt = SendKnownSpells {
            initial_login: true,
            known_spells: vec![6603, 78, 2457],
            favorite_spells: vec![],
        };
        let bytes = pkt.to_bytes();
        // opcode(2) + bit(flush)(1) + count(4) + fav_count(4) + 3*i32(12) = 23
        assert_eq!(bytes.len(), 23);
    }

    #[test]
    fn send_spell_history_empty() {
        let pkt = SendSpellHistory;
        let bytes = pkt.to_bytes();
        // opcode(2) + int32(4) = 6
        assert_eq!(bytes.len(), 6);
    }

    #[test]
    fn update_action_buttons_empty() {
        let pkt = UpdateActionButtons::empty();
        let bytes = pkt.to_bytes();
        // opcode(2) + 180*i64(1440) + uint8(1) = 1443
        assert_eq!(bytes.len(), 1443);
        let opcode = u16::from_le_bytes([bytes[0], bytes[1]]);
        assert_eq!(opcode, 0x25e0);
    }

    #[test]
    fn update_action_buttons_pack() {
        // Spell 6603 (Auto Attack) as type 0 (Spell)
        let packed = UpdateActionButtons::pack_button(6603, 0);
        assert_eq!(packed, 6603);

        // Spell 78 (Heroic Strike) as type 0
        let packed = UpdateActionButtons::pack_button(78, 0);
        assert_eq!(packed, 78);

        // Item action as type 2
        let packed = UpdateActionButtons::pack_button(12345, 2);
        // C++ player action buttons use `action | (type << 24)`.
        assert_eq!(packed, 12345 | (2i64 << 24));
    }

    #[test]
    fn initialize_factions_empty() {
        let pkt = InitializeFactions::default();
        let bytes = pkt.to_bytes();
        // opcode(2) + 1000*(uint16+int32) + ceil(1000/8) = 2 + 6000 + 125 = 6127
        assert_eq!(bytes.len(), 6127);
        let opcode = u16::from_le_bytes([bytes[0], bytes[1]]);
        assert_eq!(opcode, 0x2724);
    }

    #[test]
    fn bind_point_update_serializes() {
        let pkt = BindPointUpdate {
            x: 1.0,
            y: 2.0,
            z: 3.0,
            map_id: 0,
            area_id: 12,
        };
        let bytes = pkt.to_bytes();
        // opcode(2) + 3*f32(12) + 2*i32(8) = 22
        assert_eq!(bytes.len(), 22);
        let opcode = u16::from_le_bytes([bytes[0], bytes[1]]);
        assert_eq!(opcode, 0x257d);
    }

    #[test]
    fn player_bound_serializes_packed_guid_and_area_like_cpp() {
        let binder_id = wow_core::ObjectGuid::new(0x0102_0304_0506_0708, 0x1112_1314_1516_1718);
        let pkt = PlayerBound {
            binder_id,
            area_id: 42,
        };

        let bytes = pkt.to_bytes();
        let opcode = u16::from_le_bytes([bytes[0], bytes[1]]);
        assert_eq!(opcode, 0x2ff8);

        let mut payload = WorldPacket::from_bytes(&bytes[2..]);
        assert_eq!(payload.read_packed_guid().unwrap(), binder_id);
        assert_eq!(payload.read_uint32().unwrap(), 42);
        assert_eq!(payload.remaining(), 0);
    }

    #[test]
    fn world_server_info_serializes() {
        let pkt = WorldServerInfo::default_open_world();
        let bytes = pkt.to_bytes();
        // opcode(2) + int32(4) + 5 bits flushed to 1 byte = 7
        assert_eq!(bytes.len(), 7);
    }

    #[test]
    fn initial_setup_wotlk() {
        let pkt = InitialSetup::wotlk();
        let bytes = pkt.to_bytes();
        // opcode(2) + uint8(1) + uint8(1) = 4
        assert_eq!(bytes.len(), 4);
        let opcode = u16::from_le_bytes([bytes[0], bytes[1]]);
        assert_eq!(opcode, 0x2580);
    }

    #[test]
    fn time_sync_request_serializes() {
        let pkt = TimeSyncRequest { sequence_index: 0 };
        let bytes = pkt.to_bytes();
        // opcode(2) + u32(4) = 6
        assert_eq!(bytes.len(), 6);
        let opcode = u16::from_le_bytes([bytes[0], bytes[1]]);
        assert_eq!(opcode, 0x2dd2);
    }

    #[test]
    fn contact_list_empty() {
        let pkt = ContactList::all();
        let bytes = pkt.to_bytes();
        // opcode(2) + u32(4) + bits(8→1 byte) = 7
        assert_eq!(bytes.len(), 7);
        let opcode = u16::from_le_bytes([bytes[0], bytes[1]]);
        assert_eq!(opcode, 0x278c);
        // Flags = 7 (All)
        let flags = u32::from_le_bytes([bytes[2], bytes[3], bytes[4], bytes[5]]);
        assert_eq!(flags, 7);
    }

    #[test]
    fn active_glyphs_empty() {
        let pkt = ActiveGlyphs {
            is_full_update: true,
        };
        let bytes = pkt.to_bytes();
        // opcode(2) + i32(4) + 1 bit flushed to 1 byte = 7
        assert_eq!(bytes.len(), 7);
        let opcode = u16::from_le_bytes([bytes[0], bytes[1]]);
        assert_eq!(opcode, 0x2c51);
    }

    #[test]
    fn load_equipment_set_empty() {
        let pkt = LoadEquipmentSet;
        let bytes = pkt.to_bytes();
        // opcode(2) + i32(4) = 6
        assert_eq!(bytes.len(), 6);
        let opcode = u16::from_le_bytes([bytes[0], bytes[1]]);
        assert_eq!(opcode, 0x270e);
    }

    #[test]
    fn all_account_criteria_empty() {
        let pkt = AllAccountCriteria;
        let bytes = pkt.to_bytes();
        // opcode(2) + i32(4) = 6
        assert_eq!(bytes.len(), 6);
        let opcode = u16::from_le_bytes([bytes[0], bytes[1]]);
        assert_eq!(opcode, 0x2571);
    }

    #[test]
    fn all_achievement_data_empty() {
        let pkt = AllAchievementData;
        let bytes = pkt.to_bytes();
        // opcode(2) + i32(4) + i32(4) = 10
        assert_eq!(bytes.len(), 10);
        let opcode = u16::from_le_bytes([bytes[0], bytes[1]]);
        assert_eq!(opcode, 0x2570);
    }

    #[test]
    fn account_mount_update_empty() {
        let pkt = AccountMountUpdate::empty_full();
        let bytes = pkt.to_bytes();
        // opcode(2) + 1 bit(padded to 1 byte) + i32(4) = 7
        // wait: write_bit(true) → 1 bit buffered, then write_int32(0)
        // auto-flushes → 1 byte (bit), then 4 bytes (i32), then flush_bits (no-op) = 7
        assert_eq!(bytes.len(), 7);
        let opcode = u16::from_le_bytes([bytes[0], bytes[1]]);
        assert_eq!(opcode, 0x25ae);
    }

    #[test]
    fn account_mount_update_writes_mount_entries_like_cpp() {
        let pkt = AccountMountUpdate::full(vec![
            AccountMount {
                spell_id: 100,
                flags: 0x01,
            },
            AccountMount {
                spell_id: 200,
                flags: 0x12,
            },
        ]);
        let bytes = pkt.to_bytes();

        assert_eq!(u16::from_le_bytes([bytes[0], bytes[1]]), 0x25ae);
        assert_eq!(bytes[2], 0x80);
        assert_eq!(
            i32::from_le_bytes([bytes[3], bytes[4], bytes[5], bytes[6]]),
            2
        );
        assert_eq!(
            i32::from_le_bytes([bytes[7], bytes[8], bytes[9], bytes[10]]),
            100
        );
        assert_eq!(bytes[11], 0x10);
        assert_eq!(
            i32::from_le_bytes([bytes[12], bytes[13], bytes[14], bytes[15]]),
            200
        );
        assert_eq!(bytes[16], 0x20);
    }

    #[test]
    fn account_heirloom_update_writes_items_then_flags_like_cpp() {
        let pkt = AccountHeirloomUpdate::full(vec![
            AccountHeirloom {
                item_id: 44_000,
                flags: 0x01,
            },
            AccountHeirloom {
                item_id: 44_001,
                flags: 0x04,
            },
        ]);
        let bytes = pkt.to_bytes();

        assert_eq!(
            u16::from_le_bytes([bytes[0], bytes[1]]),
            ServerOpcodes::UpdateCapturePoint as u16
        );
        assert_eq!(bytes[2], 0x80);
        assert_eq!(
            i32::from_le_bytes([bytes[3], bytes[4], bytes[5], bytes[6]]),
            0
        );
        assert_eq!(
            u32::from_le_bytes([bytes[7], bytes[8], bytes[9], bytes[10]]),
            2
        );
        assert_eq!(
            u32::from_le_bytes([bytes[11], bytes[12], bytes[13], bytes[14]]),
            2
        );
        assert_eq!(
            i32::from_le_bytes([bytes[15], bytes[16], bytes[17], bytes[18]]),
            44_000
        );
        assert_eq!(
            i32::from_le_bytes([bytes[19], bytes[20], bytes[21], bytes[22]]),
            44_001
        );
        assert_eq!(
            u32::from_le_bytes([bytes[23], bytes[24], bytes[25], bytes[26]]),
            0x01
        );
        assert_eq!(
            u32::from_le_bytes([bytes[27], bytes[28], bytes[29], bytes[30]]),
            0x04
        );
    }

    #[test]
    fn account_mount_update_partial_clears_full_update_bit_like_cpp() {
        let pkt = AccountMountUpdate::partial(vec![AccountMount {
            spell_id: 100,
            flags: 0x01,
        }]);
        let bytes = pkt.to_bytes();

        assert_eq!(u16::from_le_bytes([bytes[0], bytes[1]]), 0x25ae);
        assert_eq!(bytes[2], 0x00);
        assert_eq!(
            i32::from_le_bytes([bytes[3], bytes[4], bytes[5], bytes[6]]),
            1
        );
    }

    #[test]
    fn mount_set_favorite_reads_cpp_field_order() {
        let mut pkt = WorldPacket::new_empty();
        pkt.write_uint16(ClientOpcodes::MountSetFavorite as u16);
        pkt.write_uint32(1234);
        pkt.write_bit(true);
        pkt.flush_bits();

        let decoded = MountSetFavorite::read(&mut pkt).unwrap();
        assert_eq!(
            decoded,
            MountSetFavorite {
                mount_spell_id: 1234,
                is_favorite: true,
            }
        );
    }

    #[test]
    fn mount_special_reads_count_sequence_and_visual_kits_like_cpp() {
        let mut pkt = WorldPacket::new_empty();
        pkt.write_uint16(ClientOpcodes::MountSpecialAnim as u16);
        pkt.write_uint32(2);
        pkt.write_int32(-7);
        pkt.write_int32(111);
        pkt.write_int32(222);

        let decoded = MountSpecial::read(&mut pkt).unwrap();
        assert_eq!(
            decoded,
            MountSpecial {
                spell_visual_kit_ids: vec![111, 222],
                sequence_variation: -7,
            }
        );
    }

    #[test]
    fn special_mount_anim_writes_guid_count_sequence_and_visual_kits_like_cpp() {
        let guid =
            ObjectGuid::create_world_object(wow_core::guid::HighGuid::Player, 0, 1, 571, 0, 0, 42);
        let bytes = SpecialMountAnim {
            unit_guid: guid,
            spell_visual_kit_ids: vec![111, -222],
            sequence_variation: 3,
        }
        .to_bytes();

        assert_eq!(
            bytes[0..2],
            (ServerOpcodes::SpecialMountAnim as u16).to_le_bytes()
        );
        assert_eq!(&bytes[2..18], &guid.to_raw_bytes());
        assert_eq!(&bytes[18..22], &2_u32.to_le_bytes());
        assert_eq!(&bytes[22..26], &3_i32.to_le_bytes());
        assert_eq!(&bytes[26..30], &111_i32.to_le_bytes());
        assert_eq!(&bytes[30..34], &(-222_i32).to_le_bytes());
        assert_eq!(bytes.len(), 34);
    }

    #[test]
    fn account_toy_update_empty() {
        let pkt = AccountToyUpdate::full(Vec::new());
        let bytes = pkt.to_bytes();
        // opcode(2) + 1 bit(padded to 1 byte) + 3*i32(12) = 15
        assert_eq!(bytes.len(), 15);
        let opcode = u16::from_le_bytes([bytes[0], bytes[1]]);
        assert_eq!(opcode, 0x25b0);
    }

    #[test]
    fn save_cuf_profiles_reads_cpp_shape() {
        let mut pkt = WorldPacket::new_empty();
        pkt.write_uint16(ClientOpcodes::SaveCufProfiles as u16);
        pkt.write_uint32(1);
        pkt.write_bits(4, 7);
        for option in 0..CUF_BOOL_OPTIONS_COUNT_LIKE_CPP {
            pkt.write_bit(matches!(option, 0 | 5 | 26));
        }
        pkt.write_uint16(72);
        pkt.write_uint16(128);
        pkt.write_uint8(2);
        pkt.write_uint8(3);
        pkt.write_uint8(4);
        pkt.write_uint8(5);
        pkt.write_uint8(6);
        pkt.write_uint16(7);
        pkt.write_uint16(8);
        pkt.write_uint16(9);
        pkt.write_string("Raid");

        let mut packet = WorldPacket::from_bytes(pkt.data());
        let parsed = SaveCufProfiles::read(&mut packet).expect("valid SaveCUFProfiles");

        assert_eq!(parsed.profiles.len(), 1);
        let profile = &parsed.profiles[0];
        assert_eq!(profile.profile_name, "Raid");
        assert_eq!(profile.frame_height, 72);
        assert_eq!(profile.frame_width, 128);
        assert_eq!(profile.sort_by, 2);
        assert_eq!(profile.health_text, 3);
        assert_eq!(profile.top_point, 4);
        assert_eq!(profile.bottom_point, 5);
        assert_eq!(profile.left_point, 6);
        assert_eq!(profile.top_offset, 7);
        assert_eq!(profile.bottom_offset, 8);
        assert_eq!(profile.left_offset, 9);
        assert_eq!(profile.bool_options, (1 << 0) | (1 << 5) | (1 << 26));
    }

    #[test]
    fn load_cuf_profiles_writes_count_and_fields_like_cpp() {
        let bytes = LoadCufProfiles {
            profiles: vec![CufProfile {
                profile_name: "Raid".to_string(),
                frame_height: 72,
                frame_width: 128,
                sort_by: 2,
                health_text: 3,
                top_point: 4,
                bottom_point: 5,
                left_point: 6,
                top_offset: 7,
                bottom_offset: 8,
                left_offset: 9,
                bool_options: (1 << 0) | (1 << 5) | (1 << 26),
            }],
        }
        .to_bytes();

        assert_eq!(
            u16::from_le_bytes([bytes[0], bytes[1]]),
            ServerOpcodes::LoadCufProfiles as u16
        );
        assert_eq!(
            u32::from_le_bytes([bytes[2], bytes[3], bytes[4], bytes[5]]),
            1
        );
    }

    #[test]
    fn account_toy_update_writes_ids_then_flag_bits_like_cpp() {
        let pkt = AccountToyUpdate::full(vec![
            AccountToy {
                item_id: 30_000,
                is_favorite: true,
                has_fanfare: false,
            },
            AccountToy {
                item_id: 30_001,
                is_favorite: false,
                has_fanfare: true,
            },
        ]);
        let bytes = pkt.to_bytes();

        assert_eq!(u16::from_le_bytes([bytes[0], bytes[1]]), 0x25b0);
        assert_eq!(bytes[2], 0x80);
        assert_eq!(
            i32::from_le_bytes([bytes[3], bytes[4], bytes[5], bytes[6]]),
            2
        );
        assert_eq!(
            i32::from_le_bytes([bytes[7], bytes[8], bytes[9], bytes[10]]),
            2
        );
        assert_eq!(
            i32::from_le_bytes([bytes[11], bytes[12], bytes[13], bytes[14]]),
            2
        );
        assert_eq!(
            u32::from_le_bytes([bytes[15], bytes[16], bytes[17], bytes[18]]),
            30_000
        );
        assert_eq!(
            u32::from_le_bytes([bytes[19], bytes[20], bytes[21], bytes[22]]),
            30_001
        );
        assert_eq!(bytes[23], 0b1001_0000);
    }

    #[test]
    fn add_toy_reads_cpp_guid_payload() {
        let guid = ObjectGuid::create_item(1, 99);
        let mut pkt = WorldPacket::new_empty();
        pkt.write_uint16(ClientOpcodes::AddToy as u16);
        pkt.write_packed_guid(&guid);

        let decoded = AddToy::read(&mut pkt).unwrap();
        assert_eq!(decoded.item_guid, guid);
    }

    #[test]
    fn toy_clear_fanfare_reads_cpp_item_id_payload() {
        let mut pkt = WorldPacket::new_empty();
        pkt.write_uint16(ClientOpcodes::ToyClearFanfare as u16);
        pkt.write_uint32(30_000);

        let decoded = ToyClearFanfare::read(&mut pkt).unwrap();
        assert_eq!(decoded.item_id, 30_000);
    }

    fn write_minimal_toy_spell_cast(
        pkt: &mut WorldPacket,
        cast_id: ObjectGuid,
        item_id: i32,
        spell_id: i32,
    ) {
        use crate::packets::spell::{SpellCastVisual, SpellTargetData};

        pkt.write_packed_guid(&cast_id);
        pkt.write_int32(item_id);
        pkt.write_int32(0);
        pkt.write_int32(spell_id);
        SpellCastVisual::default().write(pkt);
        pkt.write_float(0.0);
        pkt.write_float(0.0);
        pkt.write_packed_guid(&ObjectGuid::EMPTY);
        pkt.write_uint32(0);
        pkt.write_uint32(0);
        pkt.write_uint32(0);
        pkt.write_bits(0, 5);
        pkt.write_bit(false);
        pkt.write_bits(0, 2);
        pkt.write_bit(false);
        pkt.flush_bits();
        SpellTargetData::default().write(pkt);
    }

    #[test]
    fn use_toy_reads_spell_cast_request_like_cpp() {
        let cast_id = ObjectGuid::create_player(1, 123);
        let mut pkt = WorldPacket::new_empty();
        pkt.write_uint16(ClientOpcodes::UseToy as u16);
        write_minimal_toy_spell_cast(&mut pkt, cast_id, 30_000, 12_345);

        let decoded = UseToy::read(&mut pkt).unwrap();
        assert_eq!(decoded.cast.cast_id, cast_id);
        assert_eq!(decoded.cast.misc[0], 30_000);
        assert_eq!(decoded.cast.spell_id, 12_345);
    }

    #[test]
    fn load_cuf_profiles_empty() {
        let pkt = LoadCufProfiles::empty();
        let bytes = pkt.to_bytes();
        // opcode(2) + i32(4) = 6
        assert_eq!(bytes.len(), 6);
        let opcode = u16::from_le_bytes([bytes[0], bytes[1]]);
        assert_eq!(opcode, 0x25bc);
    }

    #[test]
    fn aura_update_empty() {
        let guid = ObjectGuid::create_player(1, 42);
        let pkt = AuraUpdate::empty_for(guid);
        let bytes = pkt.to_bytes();
        // opcode(2) + 10 bits(padded to 2 bytes) + packed_guid(variable)
        assert!(bytes.len() > 4);
        let opcode = u16::from_le_bytes([bytes[0], bytes[1]]);
        assert_eq!(opcode, 0x2c1f);
        // Byte 2: UpdateAll=1(MSB) + first 7 bits of count(0) = 0x80
        assert_eq!(bytes[2], 0x80);
    }

    #[test]
    fn battle_pet_journal_lock_acquired_empty() {
        let pkt = BattlePetJournalLockAcquired;
        let bytes = pkt.to_bytes();
        // opcode(2) + no payload = 2
        assert_eq!(bytes.len(), 2);
        let opcode = u16::from_le_bytes([bytes[0], bytes[1]]);
        assert_eq!(opcode, 0x25ed);
    }

    #[test]
    fn battle_pet_journal_lock_denied_empty() {
        let pkt = BattlePetJournalLockDenied;
        let bytes = pkt.to_bytes();
        assert_eq!(bytes.len(), 2);
        let opcode = u16::from_le_bytes([bytes[0], bytes[1]]);
        assert_eq!(opcode, 0x25ee);
    }

    #[test]
    fn battle_pet_deleted_writes_packed_guid_like_cpp() {
        let pet_guid = ObjectGuid::new(0, 0x4330);
        let bytes = BattlePetDeleted { pet_guid }.to_bytes();
        assert_eq!(
            u16::from_le_bytes([bytes[0], bytes[1]]),
            ServerOpcodes::BattlePetDeleted as u16
        );

        let mut body = WorldPacket::from_bytes(&bytes[2..]);
        assert_eq!(body.read_packed_guid().unwrap(), pet_guid);
        assert_eq!(body.remaining(), 0);
    }

    #[test]
    fn battle_pet_error_writes_result_bits_then_creature_id_like_cpp() {
        let bytes =
            BattlePetError::new(BattlePetErrorCodeLikeCpp::TooHighLevelToUncage, 12_345).to_bytes();
        assert_eq!(
            u16::from_le_bytes([bytes[0], bytes[1]]),
            ServerOpcodes::BattlePetError as u16
        );

        let mut body = WorldPacket::from_bytes(&bytes[2..]);
        assert_eq!(
            body.read_bits(4).unwrap(),
            BattlePetErrorCodeLikeCpp::TooHighLevelToUncage as u32
        );
        assert_eq!(body.read_int32().unwrap(), 12_345);
        assert_eq!(body.remaining(), 0);
    }

    #[test]
    fn battle_pet_request_journal_reads_empty_payload_like_cpp() {
        let mut pkt = WorldPacket::new_empty();
        pkt.write_uint16(ClientOpcodes::BattlePetRequestJournal as u16);

        assert_eq!(
            BattlePetRequestJournal::read(&mut pkt).unwrap(),
            BattlePetRequestJournal
        );
        assert_eq!(pkt.remaining(), 0);
    }

    #[test]
    fn battle_pet_request_journal_lock_reads_empty_payload_like_cpp() {
        let mut pkt = WorldPacket::new_empty();
        pkt.write_uint16(ClientOpcodes::BattlePetRequestJournalLock as u16);

        assert_eq!(
            BattlePetRequestJournalLock::read(&mut pkt).unwrap(),
            BattlePetRequestJournalLock
        );
        assert_eq!(pkt.remaining(), 0);
    }

    #[test]
    fn battle_pet_journal_writes_empty_default_slots_like_cpp() {
        let bytes = BattlePetJournal::empty_with_default_slots(true).to_bytes();
        assert_eq!(
            u16::from_le_bytes([bytes[0], bytes[1]]),
            ServerOpcodes::BattlePetJournal as u16
        );

        let mut body = WorldPacket::from_bytes(&bytes[2..]);
        assert_eq!(body.read_uint16().unwrap(), 0);
        assert_eq!(body.read_uint32().unwrap(), 3);
        assert_eq!(body.read_uint32().unwrap(), 0);
        assert!(body.read_bit().unwrap());

        for index in 0..3 {
            let slot_guid = body.read_packed_guid().unwrap();
            assert_eq!(slot_guid, empty_battle_pet_guid_like_cpp());
            assert_eq!(slot_guid.high_type(), HighGuid::BattlePet);
            assert_eq!(body.read_uint32().unwrap(), 0);
            assert_eq!(body.read_uint8().unwrap(), index);
            assert!(body.read_bit().unwrap());
        }
        assert_eq!(body.remaining(), 0);
    }

    fn sample_battle_pet_journal_pet_like_cpp(
        pet_guid: ObjectGuid,
        owner_guid: ObjectGuid,
    ) -> BattlePetJournalPet {
        BattlePetJournalPet {
            guid: pet_guid,
            species: 11,
            creature_id: 22,
            display_id: 33,
            breed: 44,
            level: 55,
            exp: 66,
            flags: 77,
            power: 88,
            health: 99,
            max_health: 111,
            speed: 222,
            quality: 3,
            owner_info: Some(BattlePetJournalPetOwnerInfo {
                guid: owner_guid,
                player_virtual_realm: 123,
                player_native_realm: 456,
            }),
            name: "Misha".to_string(),
        }
    }

    fn assert_sample_battle_pet_journal_pet_like_cpp(
        body: &mut WorldPacket,
        pet_guid: ObjectGuid,
        owner_guid: ObjectGuid,
    ) {
        assert_eq!(body.read_packed_guid().unwrap(), pet_guid);
        assert_eq!(body.read_uint32().unwrap(), 11);
        assert_eq!(body.read_uint32().unwrap(), 22);
        assert_eq!(body.read_uint32().unwrap(), 33);
        assert_eq!(body.read_uint16().unwrap(), 44);
        assert_eq!(body.read_uint16().unwrap(), 55);
        assert_eq!(body.read_uint16().unwrap(), 66);
        assert_eq!(body.read_uint16().unwrap(), 77);
        assert_eq!(body.read_uint32().unwrap(), 88);
        assert_eq!(body.read_uint32().unwrap(), 99);
        assert_eq!(body.read_uint32().unwrap(), 111);
        assert_eq!(body.read_uint32().unwrap(), 222);
        assert_eq!(body.read_uint8().unwrap(), 3);
        assert_eq!(body.read_bits(7).unwrap(), 5);
        assert!(body.read_bit().unwrap());
        assert!(!body.read_bit().unwrap());
        assert_eq!(body.read_string(5).unwrap(), "Misha");
        assert_eq!(body.read_packed_guid().unwrap(), owner_guid);
        assert_eq!(body.read_uint32().unwrap(), 123);
        assert_eq!(body.read_uint32().unwrap(), 456);
    }

    #[test]
    fn battle_pet_journal_writes_pet_rows_like_cpp() {
        let pet_guid = ObjectGuid::new(0, 0x4335);
        let owner_guid = ObjectGuid::create_player(1, 77);
        let bytes = BattlePetJournal {
            trap: 9,
            has_journal_lock: true,
            slots: Vec::new(),
            pets: vec![sample_battle_pet_journal_pet_like_cpp(pet_guid, owner_guid)],
        }
        .to_bytes();

        let mut body = WorldPacket::from_bytes(&bytes[2..]);
        assert_eq!(body.read_uint16().unwrap(), 9);
        assert_eq!(body.read_uint32().unwrap(), 0);
        assert_eq!(body.read_uint32().unwrap(), 1);
        assert!(body.read_bit().unwrap());
        assert_sample_battle_pet_journal_pet_like_cpp(&mut body, pet_guid, owner_guid);
        assert_eq!(body.remaining(), 0);
    }

    #[test]
    fn battle_pet_updates_writes_count_flag_then_pet_rows_like_cpp() {
        let pet_guid = ObjectGuid::new(0, 0x4336);
        let owner_guid = ObjectGuid::create_player(1, 78);
        let bytes = BattlePetUpdates {
            pets: vec![sample_battle_pet_journal_pet_like_cpp(pet_guid, owner_guid)],
            pet_added: true,
        }
        .to_bytes();

        assert_eq!(
            u16::from_le_bytes([bytes[0], bytes[1]]),
            ServerOpcodes::BattlePetUpdates as u16
        );
        let mut body = WorldPacket::from_bytes(&bytes[2..]);
        assert_eq!(body.read_uint32().unwrap(), 1);
        assert!(body.read_bit().unwrap());
        assert_sample_battle_pet_journal_pet_like_cpp(&mut body, pet_guid, owner_guid);
        assert_eq!(body.remaining(), 0);
    }

    #[test]
    fn pet_battle_slot_updates_writes_flags_then_slots_like_cpp() {
        let pet_guid = ObjectGuid::new(0, 0x4337);
        let bytes = PetBattleSlotUpdates {
            slots: vec![BattlePetJournalSlot {
                pet_guid,
                collar_id: 10,
                index: 2,
                locked: false,
            }],
            auto_slotted: false,
            new_slot: true,
        }
        .to_bytes();

        assert_eq!(
            u16::from_le_bytes([bytes[0], bytes[1]]),
            ServerOpcodes::PetBattleSlotUpdates as u16
        );
        let mut body = WorldPacket::from_bytes(&bytes[2..]);
        assert_eq!(body.read_uint32().unwrap(), 1);
        assert!(body.read_bit().unwrap());
        assert!(!body.read_bit().unwrap());
        assert_eq!(body.read_packed_guid().unwrap(), pet_guid);
        assert_eq!(body.read_uint32().unwrap(), 10);
        assert_eq!(body.read_uint8().unwrap(), 2);
        assert!(!body.read_bit().unwrap());
        assert_eq!(body.remaining(), 0);
    }

    #[test]
    fn battle_pet_set_battle_slot_reads_cpp_shape() {
        let pet_guid = ObjectGuid::new(0, 0x4323);
        let mut pkt = WorldPacket::new_empty();
        pkt.write_uint16(ClientOpcodes::BattlePetSetBattleSlot as u16);
        pkt.write_packed_guid(&pet_guid);
        pkt.write_uint8(2);

        let decoded = BattlePetSetBattleSlot::read(&mut pkt).unwrap();
        assert_eq!(decoded, BattlePetSetBattleSlot { pet_guid, slot: 2 });
    }

    #[test]
    fn battle_pet_summon_reads_packed_guid_like_cpp() {
        let pet_guid = ObjectGuid::new(0, 0x4324);
        let mut pkt = WorldPacket::new_empty();
        pkt.write_uint16(ClientOpcodes::BattlePetSummon as u16);
        pkt.write_packed_guid(&pet_guid);

        let decoded = BattlePetSummon::read(&mut pkt).unwrap();
        assert_eq!(decoded, BattlePetSummon { pet_guid });
    }

    #[test]
    fn battle_pet_update_notify_reads_packed_guid_like_cpp() {
        let pet_guid = ObjectGuid::new(0, 0x4325);
        let mut pkt = WorldPacket::new_empty();
        pkt.write_uint16(ClientOpcodes::BattlePetUpdateNotify as u16);
        pkt.write_packed_guid(&pet_guid);

        let decoded = BattlePetUpdateNotify::read(&mut pkt).unwrap();
        assert_eq!(decoded, BattlePetUpdateNotify { pet_guid });
    }

    #[test]
    fn battle_pet_delete_pet_reads_placeholder_cpp_shape() {
        let pet_guid = ObjectGuid::new(0, 0x4331);
        let mut pkt = WorldPacket::new_empty();
        pkt.write_uint16(0xBADD);
        pkt.write_packed_guid(&pet_guid);

        let decoded = BattlePetDeletePet::read_like_cpp(&mut pkt).unwrap();
        assert_eq!(decoded, BattlePetDeletePet { pet_guid });
        assert_eq!(pkt.remaining(), 0);
    }

    #[test]
    fn cage_battle_pet_reads_placeholder_cpp_shape() {
        let pet_guid = ObjectGuid::new(0, 0x4334);
        let mut pkt = WorldPacket::new_empty();
        pkt.write_uint16(0xBADD);
        pkt.write_packed_guid(&pet_guid);

        let decoded = CageBattlePet::read_like_cpp(&mut pkt).unwrap();
        assert_eq!(decoded, CageBattlePet { pet_guid });
        assert_eq!(pkt.remaining(), 0);
    }

    #[test]
    fn battle_pet_modify_name_reads_without_declined_names_like_cpp() {
        let pet_guid = ObjectGuid::new(0, 0x4332);
        let mut pkt = WorldPacket::new_empty();
        pkt.write_uint16(0xBADD);
        pkt.write_packed_guid(&pet_guid);
        pkt.write_bits(5, 7);
        pkt.write_bit(false);
        pkt.write_string("Misha");

        let decoded = BattlePetModifyName::read_like_cpp(&mut pkt).unwrap();
        assert_eq!(
            decoded,
            BattlePetModifyName {
                pet_guid,
                name: "Misha".to_string(),
                declined_names: None,
            }
        );
        assert_eq!(pkt.remaining(), 0);
    }

    #[test]
    fn battle_pet_modify_name_reads_declined_names_before_name_like_cpp() {
        let pet_guid = ObjectGuid::new(0, 0x4333);
        let declined = ["Mishy", "Mishys", "Mishyu", "Mishy2", "Mishy3"];
        let mut pkt = WorldPacket::new_empty();
        pkt.write_uint16(0xBADD);
        pkt.write_packed_guid(&pet_guid);
        pkt.write_bits(5, 7);
        pkt.write_bit(true);
        for name in declined {
            pkt.write_bits(name.len() as u32, 7);
        }
        for name in declined {
            pkt.write_string(name);
        }
        pkt.write_string("Misha");

        let decoded = BattlePetModifyName::read_like_cpp(&mut pkt).unwrap();
        assert_eq!(decoded.pet_guid, pet_guid);
        assert_eq!(decoded.name, "Misha");
        assert_eq!(
            decoded.declined_names.unwrap().names,
            declined.map(str::to_string)
        );
        assert_eq!(pkt.remaining(), 0);
    }

    #[test]
    fn query_battle_pet_name_reads_cpp_shape() {
        let battle_pet_id = ObjectGuid::new(0, 0x4326);
        let unit_guid = ObjectGuid::new(0, 0x4327);
        let mut pkt = WorldPacket::new_empty();
        pkt.write_uint16(ClientOpcodes::QueryBattlePetName as u16);
        pkt.write_packed_guid(&battle_pet_id);
        pkt.write_packed_guid(&unit_guid);

        let decoded = QueryBattlePetName::read(&mut pkt).unwrap();
        assert_eq!(
            decoded,
            QueryBattlePetName {
                battle_pet_id,
                unit_guid,
            }
        );
    }

    #[test]
    fn query_battle_pet_name_response_writes_negative_cpp_shape() {
        let battle_pet_id = ObjectGuid::new(0, 0x4328);
        let response = QueryBattlePetNameResponse::not_allowed(battle_pet_id);
        let bytes = response.to_bytes();

        assert_eq!(
            u16::from_le_bytes([bytes[0], bytes[1]]),
            ServerOpcodes::QueryBattlePetNameResponse as u16
        );
        let mut body = WorldPacket::from_bytes(&bytes[2..]);
        assert_eq!(body.read_packed_guid().unwrap(), battle_pet_id);
        assert_eq!(body.read_int32().unwrap(), 0);
        assert_eq!(body.read_int64().unwrap(), 0);
        assert!(!body.read_bit().unwrap());
        assert_eq!(body.remaining(), 0);
    }

    #[test]
    fn query_battle_pet_name_response_writes_positive_without_declined_names_like_cpp() {
        let battle_pet_id = ObjectGuid::new(0, 0x4329);
        let response = QueryBattlePetNameResponse::allowed(
            battle_pet_id,
            91_001,
            1_717_000_123,
            "Rusty".to_string(),
            None,
        );
        let bytes = response.to_bytes();

        assert_eq!(
            u16::from_le_bytes([bytes[0], bytes[1]]),
            ServerOpcodes::QueryBattlePetNameResponse as u16
        );
        let mut body = WorldPacket::from_bytes(&bytes[2..]);
        assert_eq!(body.read_packed_guid().unwrap(), battle_pet_id);
        assert_eq!(body.read_int32().unwrap(), 91_001);
        assert_eq!(body.read_int64().unwrap(), 1_717_000_123);
        assert!(body.read_bit().unwrap());
        assert_eq!(body.read_bits(8).unwrap(), 5);
        assert!(!body.read_bit().unwrap());
        for _ in 0..MAX_DECLINED_NAME_CASES_LIKE_CPP {
            assert_eq!(body.read_bits(7).unwrap(), 0);
        }
        assert_eq!(body.read_string(5).unwrap(), "Rusty");
        assert_eq!(body.remaining(), 0);
    }

    #[test]
    fn query_battle_pet_name_response_writes_positive_with_declined_names_like_cpp() {
        let battle_pet_id = ObjectGuid::new(0, 0x432a);
        let declined = ["Alpha", "Betas", "Gamma", "Delta", "Epsil"].map(str::to_string);
        let response = QueryBattlePetNameResponse::allowed(
            battle_pet_id,
            91_002,
            1_717_000_456,
            "Companion".to_string(),
            Some(DeclinedNamesLikeCpp {
                names: declined.clone(),
            }),
        );
        let bytes = response.to_bytes();

        assert_eq!(
            u16::from_le_bytes([bytes[0], bytes[1]]),
            ServerOpcodes::QueryBattlePetNameResponse as u16
        );
        let mut body = WorldPacket::from_bytes(&bytes[2..]);
        assert_eq!(body.read_packed_guid().unwrap(), battle_pet_id);
        assert_eq!(body.read_int32().unwrap(), 91_002);
        assert_eq!(body.read_int64().unwrap(), 1_717_000_456);
        assert!(body.read_bit().unwrap());
        assert_eq!(body.read_bits(8).unwrap(), 9);
        assert!(body.read_bit().unwrap());
        for name in &declined {
            assert_eq!(body.read_bits(7).unwrap(), name.len() as u32);
        }
        for name in &declined {
            assert_eq!(body.read_string(name.len()).unwrap(), *name);
        }
        assert_eq!(body.read_string(9).unwrap(), "Companion");
        assert_eq!(body.remaining(), 0);
    }

    #[test]
    fn battle_pet_clear_fanfare_reads_packed_guid_like_cpp() {
        let pet_guid = ObjectGuid::new(0, 0x4321);
        let mut pkt = WorldPacket::new_empty();
        pkt.write_uint16(ClientOpcodes::BattlePetClearFanfare as u16);
        pkt.write_packed_guid(&pet_guid);

        let decoded = BattlePetClearFanfare::read(&mut pkt).unwrap();
        assert_eq!(decoded.pet_guid, pet_guid);
    }

    #[test]
    fn battle_pet_set_flags_reads_cpp_shape() {
        let pet_guid = ObjectGuid::new(0, 0x4322);
        let mut pkt = WorldPacket::new_empty();
        pkt.write_uint16(ClientOpcodes::BattlePetSetFlags as u16);
        pkt.write_packed_guid(&pet_guid);
        pkt.write_uint16(0x12);
        pkt.write_bits(1, 2);
        pkt.flush_bits();

        let decoded = BattlePetSetFlags::read(&mut pkt).unwrap();
        assert_eq!(
            decoded,
            BattlePetSetFlags {
                pet_guid,
                flags: 0x12,
                control_type: 1,
            }
        );
    }

    #[test]
    fn db_reply_not_found() {
        let pkt = DBReply::not_found(0xDF2F53CF, 42);
        let bytes = pkt.to_bytes();
        // opcode(2) + u32(4) + i32(4) + i32(4) + 3 bits flushed(1) + u32(4) = 19
        assert_eq!(bytes.len(), 19);
        let opcode = u16::from_le_bytes([bytes[0], bytes[1]]);
        assert_eq!(opcode, 0x290e);
        // table_hash
        let th = u32::from_le_bytes([bytes[2], bytes[3], bytes[4], bytes[5]]);
        assert_eq!(th, 0xDF2F53CF);
        // record_id
        let rid = i32::from_le_bytes([bytes[6], bytes[7], bytes[8], bytes[9]]);
        assert_eq!(rid, 42);
        // status byte: 3 bits MSB-first for value 3 = 0b011 → in MSB-first bit layout: 0_1_1_00000 = 0x60
        assert_eq!(bytes[14], 0x60);
        // data size = 0
        let ds = u32::from_le_bytes([bytes[15], bytes[16], bytes[17], bytes[18]]);
        assert_eq!(ds, 0);
    }

    #[test]
    fn db_query_bulk_roundtrip() {
        // Build a DbQueryBulk packet manually with 13-bit count.
        // Use a WorldPacket's bit writer to produce correctly-encoded bits.
        let mut writer = WorldPacket::new_server(ServerOpcodes::DbReply);
        // Overwrite opcode with client opcode (we'll skip it anyway)
        // Just append the payload fields after a dummy 2-byte opcode:
        writer.write_uint32(0xAABBCCDD); // table_hash
        writer.write_bits(3, 13); // count = 3 (13 bits)
        writer.flush_bits();
        writer.write_int32(100);
        writer.write_int32(200);
        writer.write_int32(300);

        // Read it back: from_bytes includes the 2-byte opcode from new_server
        let mut reader = WorldPacket::from_bytes(writer.data());
        reader.skip_opcode(); // skip the 2-byte dummy opcode
        let parsed = DbQueryBulk::read(&mut reader).unwrap();
        assert_eq!(parsed.table_hash, 0xAABBCCDD);
        assert_eq!(parsed.queries, vec![100, 200, 300]);
    }

    #[test]
    fn hotfix_connect_empty() {
        let pkt = HotfixConnect::empty();
        let bytes = pkt.to_bytes();
        // opcode(2) + i32(4) + u32(4) = 10
        assert_eq!(bytes.len(), 10);
        let opcode = u16::from_le_bytes([bytes[0], bytes[1]]);
        assert_eq!(opcode, 0x2911);
        // count = 0
        let count = i32::from_le_bytes([bytes[2], bytes[3], bytes[4], bytes[5]]);
        assert_eq!(count, 0);
        // content size = 0
        let size = u32::from_le_bytes([bytes[6], bytes[7], bytes[8], bytes[9]]);
        assert_eq!(size, 0);
    }

    #[test]
    fn hotfix_connect_serializes_headers_and_content() {
        let pkt = HotfixConnect {
            hotfixes: vec![HotfixConnectData {
                id: HotfixId {
                    push_id: 11,
                    unique_id: 12,
                },
                table_hash: 0xDF2F_53CF,
                record_id: 67,
                size: 3,
                status: 1,
            }],
            content: vec![1, 2, 3],
        };
        let bytes = pkt.to_bytes();
        assert_eq!(u16::from_le_bytes([bytes[0], bytes[1]]), 0x2911);
        assert_eq!(
            u32::from_le_bytes([bytes[2], bytes[3], bytes[4], bytes[5]]),
            1
        );
        assert_eq!(
            i32::from_le_bytes([bytes[6], bytes[7], bytes[8], bytes[9]]),
            11
        );
        assert_eq!(
            u32::from_le_bytes([bytes[10], bytes[11], bytes[12], bytes[13]]),
            12
        );
        assert_eq!(
            u32::from_le_bytes([bytes[14], bytes[15], bytes[16], bytes[17]]),
            0xDF2F_53CF
        );
        assert_eq!(
            i32::from_le_bytes([bytes[18], bytes[19], bytes[20], bytes[21]]),
            67
        );
        assert_eq!(
            u32::from_le_bytes([bytes[22], bytes[23], bytes[24], bytes[25]]),
            3
        );
        assert_eq!(bytes[26] >> 5, 1);
        assert_eq!(
            u32::from_le_bytes([bytes[27], bytes[28], bytes[29], bytes[30]]),
            3
        );
        assert_eq!(&bytes[31..34], &[1, 2, 3]);
    }

    #[test]
    fn dungeon_difficulty_set_normal() {
        let pkt = DungeonDifficultySet::normal();
        let bytes = pkt.to_bytes();
        // opcode(2) + i32(4) = 6
        assert_eq!(bytes.len(), 6);
        let opcode = u16::from_le_bytes([bytes[0], bytes[1]]);
        assert_eq!(opcode, 0x26a4);
        let difficulty = i32::from_le_bytes([bytes[2], bytes[3], bytes[4], bytes[5]]);
        assert_eq!(difficulty, 0);
    }

    #[test]
    fn raid_difficulty_set_writes_legacy_flag_like_cpp() {
        let pkt = RaidDifficultySet {
            difficulty_id: 4,
            legacy: true,
        };
        let bytes = pkt.to_bytes();
        // opcode(2) + i32(4) + uint8(1) = 7
        assert_eq!(bytes.len(), 7);
        let opcode = u16::from_le_bytes([bytes[0], bytes[1]]);
        assert_eq!(opcode, 0x27ad);
        let difficulty = i32::from_le_bytes([bytes[2], bytes[3], bytes[4], bytes[5]]);
        assert_eq!(difficulty, 4);
        assert_eq!(bytes[6], 1);
    }

    #[test]
    fn move_set_active_mover() {
        let guid = ObjectGuid::create_player(1, 42);
        let pkt = MoveSetActiveMover { mover_guid: guid };
        let bytes = pkt.to_bytes();
        let opcode = u16::from_le_bytes([bytes[0], bytes[1]]);
        assert_eq!(opcode, 0x2dd5);
        // After opcode: packed guid (variable length, but > 0)
        assert!(bytes.len() > 2);
    }

    #[test]
    fn set_spell_modifier_flat_empty() {
        let bytes = SetSpellModifier::flat_empty().to_bytes();
        let opcode = u16::from_le_bytes([bytes[0], bytes[1]]);
        assert_eq!(opcode, 0x2c33);
        // opcode(2) + i32(4) = 6
        assert_eq!(bytes.len(), 6);
        let count = i32::from_le_bytes([bytes[2], bytes[3], bytes[4], bytes[5]]);
        assert_eq!(count, 0);
    }

    #[test]
    fn set_spell_modifier_pct_empty() {
        let bytes = SetSpellModifier::pct_empty().to_bytes();
        let opcode = u16::from_le_bytes([bytes[0], bytes[1]]);
        assert_eq!(opcode, 0x2c34);
        assert_eq!(bytes.len(), 6);
    }

    #[test]
    fn set_proficiency_weapon() {
        let pkt = SetProficiency::default_weapons(1); // Warrior
        let bytes = pkt.to_bytes();
        let opcode = u16::from_le_bytes([bytes[0], bytes[1]]);
        assert_eq!(opcode, 0x2735);
        // opcode(2) + u32(4) + u8(1) = 7
        assert_eq!(bytes.len(), 7);
        // Class byte = 2 (Weapon)
        assert_eq!(bytes[6], 2);
    }

    #[test]
    fn logout_request_read() {
        let mut writer = WorldPacket::new_server(ServerOpcodes::DbReply); // dummy opcode
        writer.write_bit(true); // idle_logout
        writer.flush_bits();
        let mut reader = WorldPacket::from_bytes(writer.data());
        reader.skip_opcode();
        let req = LogoutRequest::read(&mut reader).unwrap();
        assert!(req.idle_logout);
    }

    #[test]
    fn logout_response_instant_ok() {
        let pkt = LogoutResponse::instant_ok();
        let bytes = pkt.to_bytes();
        let opcode = u16::from_le_bytes([bytes[0], bytes[1]]);
        assert_eq!(opcode, 0x2683);
        // i32(4) + 1 bit flushed(1) = 7 total
        assert_eq!(bytes.len(), 7);
        // result = 0
        let result = i32::from_le_bytes([bytes[2], bytes[3], bytes[4], bytes[5]]);
        assert_eq!(result, 0);
        // instant = true → MSB bit set
        assert_eq!(bytes[6], 0x80);
    }

    #[test]
    fn logout_response_delayed_ok() {
        let pkt = LogoutResponse::delayed_ok();
        let bytes = pkt.to_bytes();
        assert_eq!(bytes.len(), 7);
        // instant = false → 0x00
        assert_eq!(bytes[6], 0x00);
    }

    #[test]
    fn logout_complete_empty() {
        let pkt = LogoutComplete;
        let bytes = pkt.to_bytes();
        assert_eq!(bytes.len(), 2); // opcode only
        let opcode = u16::from_le_bytes([bytes[0], bytes[1]]);
        assert_eq!(opcode, 0x2684);
    }

    #[test]
    fn logout_cancel_ack_empty() {
        let pkt = LogoutCancelAck;
        let bytes = pkt.to_bytes();
        assert_eq!(bytes.len(), 2); // opcode only
        let opcode = u16::from_le_bytes([bytes[0], bytes[1]]);
        assert_eq!(opcode, 0x2685);
    }

    #[test]
    fn buy_failed_serializes_cpp_reason_byte() {
        let pkt = BuyFailed {
            vendor_guid: ObjectGuid::EMPTY,
            muid: 123,
            reason: BuyResult::DistanceTooFar,
        };
        let bytes = pkt.to_bytes();

        assert_eq!(bytes[bytes.len() - 1], BuyResult::DistanceTooFar as u8);
    }

    #[test]
    fn buy_back_item_reads_cpp_guid_and_slot() {
        let vendor_guid = ObjectGuid::create_world_object(
            wow_core::guid::HighGuid::Creature,
            0,
            1,
            0,
            1,
            123,
            456,
        );
        let mut writer = WorldPacket::new_server(ServerOpcodes::DbReply);
        writer.write_packed_guid(&vendor_guid);
        writer.write_uint32(94);

        let mut reader = WorldPacket::from_bytes(writer.data());
        reader.skip_opcode();
        let pkt = BuyBackItem::read(&mut reader).unwrap();

        assert_eq!(pkt.vendor_guid, vendor_guid);
        assert_eq!(pkt.slot, 94);
    }

    #[test]
    fn repair_item_reads_cpp_guids_and_guild_bank_bit() {
        let npc_guid = ObjectGuid::create_world_object(
            wow_core::guid::HighGuid::Creature,
            0,
            1,
            0,
            1,
            123,
            456,
        );
        let item_guid = ObjectGuid::create_item(1, 777);
        let mut writer = WorldPacket::new_server(ServerOpcodes::DbReply);
        writer.write_packed_guid(&npc_guid);
        writer.write_packed_guid(&item_guid);
        writer.write_bit(true);
        writer.flush_bits();

        let mut reader = WorldPacket::from_bytes(writer.data());
        reader.skip_opcode();
        let pkt = RepairItem::read(&mut reader).unwrap();

        assert_eq!(pkt.npc_guid, npc_guid);
        assert_eq!(pkt.item_guid, item_guid);
        assert!(pkt.use_guild_bank);
    }

    #[test]
    fn request_stabled_pets_reads_cpp_stable_master_guid() {
        let stable_master = ObjectGuid::create_world_object(
            wow_core::guid::HighGuid::Creature,
            0,
            1,
            571,
            0,
            345,
            678,
        );
        let mut writer = WorldPacket::new_server(ServerOpcodes::DbReply);
        writer.write_packed_guid(&stable_master);

        let mut reader = WorldPacket::from_bytes(writer.data());
        reader.skip_opcode();
        let pkt = RequestStabledPets::read(&mut reader).unwrap();

        assert_eq!(pkt.stable_master, stable_master);
    }

    #[test]
    fn spirit_healer_activate_reads_cpp_healer_guid() {
        let healer =
            ObjectGuid::create_world_object(wow_core::guid::HighGuid::Creature, 0, 1, 571, 0, 9, 1);
        let mut writer = WorldPacket::new_server(ServerOpcodes::DbReply);
        writer.write_packed_guid(&healer);

        let mut reader = WorldPacket::from_bytes(writer.data());
        reader.skip_opcode();
        let pkt = SpiritHealerActivate::read(&mut reader).unwrap();

        assert_eq!(pkt.healer, healer);
    }

    #[test]
    fn area_spirit_healer_query_reads_cpp_healer_guid() {
        let healer =
            ObjectGuid::create_world_object(wow_core::guid::HighGuid::Creature, 0, 1, 571, 0, 9, 2);
        let mut writer = WorldPacket::new_server(ServerOpcodes::DbReply);
        writer.write_packed_guid(&healer);

        let mut reader = WorldPacket::from_bytes(writer.data());
        reader.skip_opcode();
        let pkt = AreaSpiritHealerQuery::read(&mut reader).unwrap();

        assert_eq!(pkt.healer_guid, healer);
    }

    #[test]
    fn area_spirit_healer_queue_reads_cpp_healer_guid() {
        let healer =
            ObjectGuid::create_world_object(wow_core::guid::HighGuid::Creature, 0, 1, 571, 0, 9, 3);
        let mut writer = WorldPacket::new_server(ServerOpcodes::DbReply);
        writer.write_packed_guid(&healer);

        let mut reader = WorldPacket::from_bytes(writer.data());
        reader.skip_opcode();
        let pkt = AreaSpiritHealerQueue::read(&mut reader).unwrap();

        assert_eq!(pkt.healer_guid, healer);
    }

    #[test]
    fn area_spirit_healer_time_writes_cpp_guid_and_time_left() {
        let healer =
            ObjectGuid::create_world_object(wow_core::guid::HighGuid::Creature, 0, 1, 571, 0, 9, 4);
        let packet = AreaSpiritHealerTime {
            healer_guid: healer,
            time_left_ms: 12_345,
        };

        let mut bytes = (ServerOpcodes::AreaSpiritHealerTime as u16)
            .to_le_bytes()
            .to_vec();
        let mut payload = WorldPacket::new_empty();
        payload.write_packed_guid(&healer);
        payload.write_int32(12_345);
        bytes.extend_from_slice(payload.data());

        assert_eq!(packet.to_bytes(), bytes);
    }

    #[test]
    fn hearth_and_resurrect_reads_empty_cpp_packet() {
        let mut pkt = WorldPacket::new_empty();

        HearthAndResurrect::read(&mut pkt).unwrap();
    }

    #[test]
    fn resurrect_response_reads_guid_and_response_like_cpp() {
        let resurrecter = ObjectGuid::create_player(1, 77);
        let mut pkt = WorldPacket::new_empty();
        pkt.write_packed_guid(&resurrecter);
        pkt.write_uint32(1);

        let parsed = ResurrectResponse::read(&mut pkt).unwrap();

        assert_eq!(parsed.resurrecter, resurrecter);
        assert_eq!(parsed.response, 1);
    }

    #[test]
    fn battlefield_leave_reads_empty_cpp_packet() {
        let mut pkt = WorldPacket::new_empty();

        BattlefieldLeave::read(&mut pkt).unwrap();
    }

    #[test]
    fn battlefield_port_reads_ticket_and_accepted_bit_like_cpp() {
        let requester = ObjectGuid::create_player(1, 42);
        let mut pkt = WorldPacket::new_empty();
        pkt.write_packed_guid(&requester);
        pkt.write_uint32(1);
        pkt.write_uint32(2);
        pkt.write_int64(1_234_567);
        pkt.write_bit(true);
        pkt.flush_bits();
        pkt.write_bit(true);
        pkt.flush_bits();
        pkt.reset_read();

        let parsed = BattlefieldPort::read(&mut pkt).unwrap();

        assert_eq!(parsed.ticket.requester_guid, requester);
        assert_eq!(parsed.ticket.id, 1);
        assert_eq!(parsed.ticket.ride_type, 2);
        assert_eq!(parsed.ticket.time, 1_234_567);
        assert!(parsed.ticket.unknown925);
        assert!(parsed.accepted_invite);
        assert_eq!(pkt.remaining(), 0);
    }

    #[test]
    fn battlefield_list_request_reads_list_id_like_cpp() {
        let mut pkt = WorldPacket::new_empty();
        pkt.write_int32(3);
        pkt.reset_read();

        let parsed = BattlefieldListRequest::read(&mut pkt).unwrap();

        assert_eq!(parsed.list_id, 3);
        assert_eq!(pkt.remaining(), 0);
    }

    #[test]
    fn battlemaster_join_reads_queue_roles_blacklist_like_cpp() {
        let mut pkt = WorldPacket::new_empty();
        pkt.write_uint32(2);
        pkt.write_uint8(0x07);
        pkt.write_int32(10);
        pkt.write_int32(-1);
        pkt.write_uint64(0x1F10_0000_0000_0003);
        pkt.write_uint64(0x1F10_0000_0001_0003);
        pkt.reset_read();

        let parsed = BattlemasterJoin::read(&mut pkt).unwrap();

        assert_eq!(
            parsed.queue_ids,
            [0x1F10_0000_0000_0003, 0x1F10_0000_0001_0003]
        );
        assert_eq!(parsed.roles, 0x07);
        assert_eq!(parsed.blacklist_map, [10, -1]);
        assert_eq!(pkt.remaining(), 0);
    }

    #[test]
    fn battlemaster_join_arena_reads_team_size_index_and_roles_like_cpp() {
        let mut pkt = WorldPacket::new_empty();
        pkt.write_uint8(1);
        pkt.write_uint8(0x07);
        pkt.reset_read();

        let parsed = BattlemasterJoinArena::read(&mut pkt).unwrap();

        assert_eq!(parsed.team_size_index, 1);
        assert_eq!(parsed.roles, 0x07);
        assert_eq!(pkt.remaining(), 0);
    }

    #[test]
    fn battlemaster_join_skirmish_reads_ids_group_and_rated_like_cpp() {
        let mut pkt = WorldPacket::new_empty();
        pkt.write_uint32(5);
        pkt.write_uint32(3);
        pkt.write_uint8(1);
        pkt.write_uint8(0);
        pkt.reset_read();

        let parsed = BattlemasterJoinSkirmish::read(&mut pkt).unwrap();

        assert_eq!(parsed.bg_type_id, 5);
        assert_eq!(parsed.bracket_id, 3);
        assert_eq!(parsed.as_group, 1);
        assert_eq!(parsed.is_rated, 0);
        assert_eq!(pkt.remaining(), 0);
    }

    #[test]
    fn accept_wargame_invite_reads_cstring_inviter_name_like_cpp() {
        let mut pkt = WorldPacket::new_empty();
        pkt.write_string("Inviter");
        pkt.write_uint8(0);
        pkt.reset_read();

        let parsed = AcceptWargameInvite::read(&mut pkt).unwrap();

        assert_eq!(parsed.inviter_name, "Inviter");
    }

    #[test]
    fn buy_item_resets_bitpos_between_item_bonus_and_mod_list_like_cpp() {
        let vendor_guid = ObjectGuid::create_world_object(
            wow_core::guid::HighGuid::Creature,
            0,
            1,
            0,
            1,
            123,
            456,
        );
        let container_guid = ObjectGuid::create_player(1, 42);
        let mut writer = WorldPacket::new_server(ServerOpcodes::DbReply);
        writer.write_packed_guid(&vendor_guid);
        writer.write_packed_guid(&container_guid);
        writer.write_int32(2);
        writer.write_int32(7);
        writer.write_int32(3);
        writer.write_int32(1);
        writer.write_int32(700);
        writer.write_int32(11);
        writer.write_int32(-22);
        writer.write_bit(false);
        writer.flush_bits();
        writer.write_bits(1, 6);
        writer.flush_bits();
        writer.write_int32(1234);
        writer.write_uint8(5);

        let mut reader = WorldPacket::from_bytes(writer.data());
        reader.skip_opcode();
        let pkt = BuyItem::read(&mut reader).unwrap();

        assert_eq!(pkt.vendor_guid, vendor_guid);
        assert_eq!(pkt.container_guid, container_guid);
        assert_eq!(pkt.quantity, 2);
        assert_eq!(pkt.muid, 7);
        assert_eq!(pkt.slot, 3);
        assert_eq!(pkt.item_type, 1);
        assert_eq!(pkt.item_id, 700);
    }

    #[test]
    fn sell_response_serializes_cpp_count_and_reason_before_item_guids() {
        let pkt = SellResponse {
            vendor_guid: ObjectGuid::EMPTY,
            item_guids: Vec::new(),
            reason: SellResult::CantSellItem as i32,
        };
        let bytes = pkt.to_bytes();

        assert_eq!(
            &bytes[bytes.len() - 8..bytes.len() - 4],
            &0u32.to_le_bytes()
        );
        assert_eq!(
            &bytes[bytes.len() - 4..],
            &(SellResult::CantSellItem as i32).to_le_bytes()
        );

        let error = SellResponse::error(
            ObjectGuid::EMPTY,
            ObjectGuid::EMPTY,
            SellResult::YouDontOwnThatItem,
        );
        assert_eq!(error.item_guids.len(), 1);
        assert_eq!(error.reason, SellResult::YouDontOwnThatItem as i32);
    }

    #[test]
    fn set_proficiency_armor() {
        let pkt = SetProficiency::default_armor(1); // Warrior
        let bytes = pkt.to_bytes();
        assert_eq!(bytes.len(), 7);
        // Class byte = 4 (Armor)
        assert_eq!(bytes[6], 4);
        // Mask = 0x5E for warrior (Cloth+Leather+Mail+Plate+Shield)
        let mask = u32::from_le_bytes([bytes[2], bytes[3], bytes[4], bytes[5]]);
        assert_eq!(mask, 0x5E);
    }

    #[test]
    fn fish_not_hooked_is_empty_server_packet_like_cpp() {
        let bytes = FishNotHooked.to_bytes();
        assert_eq!(bytes, (ServerOpcodes::FishNotHooked as u16).to_le_bytes());
    }

    #[test]
    fn enable_barber_shop_writes_customization_scope_like_cpp() {
        let bytes = EnableBarberShop {
            customization_scope: 7,
        }
        .to_bytes();
        assert_eq!(
            bytes[0..2],
            (ServerOpcodes::EnableBarberShop as u16).to_le_bytes()
        );
        assert_eq!(bytes[2], 7);
        assert_eq!(bytes.len(), 3);
    }

    #[test]
    fn gameobject_interaction_writes_raw_guid_and_interaction_type_like_cpp() {
        let guid = ObjectGuid::create_world_object(
            wow_core::guid::HighGuid::GameObject,
            0,
            1,
            571,
            0,
            777,
            23,
        );
        let bytes = GameObjectInteraction {
            object_guid: guid,
            interaction_type: 40,
        }
        .to_bytes();
        assert_eq!(
            bytes[0..2],
            (ServerOpcodes::GameObjectInteraction as u16).to_le_bytes()
        );
        assert_eq!(&bytes[2..18], &guid.to_raw_bytes());
        assert_eq!(&bytes[18..22], &40_i32.to_le_bytes());
        assert_eq!(bytes.len(), 22);
    }

    #[test]
    fn gameobject_custom_anim_writes_guid_anim_and_despawn_bit_like_cpp() {
        let guid = ObjectGuid::create_world_object(
            wow_core::guid::HighGuid::GameObject,
            0,
            1,
            571,
            0,
            777,
            23,
        );
        let bytes = GameObjectCustomAnim {
            object_guid: guid,
            custom_anim: 255,
            play_as_despawn: false,
        }
        .to_bytes();
        assert_eq!(
            bytes[0..2],
            (ServerOpcodes::GameObjectCustomAnim as u16).to_le_bytes()
        );
        assert_eq!(&bytes[2..18], &guid.to_raw_bytes());
        assert_eq!(&bytes[18..22], &255_u32.to_le_bytes());
        assert_eq!(bytes[22], 0x00);
        assert_eq!(bytes.len(), 23);

        let despawn_bytes = GameObjectCustomAnim {
            object_guid: guid,
            custom_anim: 7,
            play_as_despawn: true,
        }
        .to_bytes();
        assert_eq!(despawn_bytes[22], 0x80);
    }

    #[test]
    fn gameobject_despawn_writes_raw_guid_like_cpp() {
        let guid = ObjectGuid::create_world_object(
            wow_core::guid::HighGuid::GameObject,
            0,
            1,
            571,
            0,
            777,
            23,
        );
        let bytes = GameObjectDespawn { object_guid: guid }.to_bytes();
        assert_eq!(
            bytes[0..2],
            (ServerOpcodes::GameObjectDespawn as u16).to_le_bytes()
        );
        assert_eq!(&bytes[2..18], &guid.to_raw_bytes());
        assert_eq!(bytes.len(), 18);
    }

    #[test]
    fn capture_point_removed_writes_only_raw_guid_like_cpp() {
        let guid = ObjectGuid::create_world_object(
            wow_core::guid::HighGuid::GameObject,
            0,
            1,
            571,
            0,
            777,
            24,
        );
        let bytes = CapturePointRemoved {
            capture_point_guid: guid,
        }
        .to_bytes();
        assert_eq!(
            bytes[0..2],
            (ServerOpcodes::UpdateCapturePoint as u16).to_le_bytes()
        );
        assert_eq!(&bytes[2..18], &guid.to_raw_bytes());
        assert_eq!(bytes.len(), 18);
    }

    #[test]
    fn gameobject_set_state_local_writes_raw_guid_and_state_like_cpp() {
        let guid = ObjectGuid::create_world_object(
            wow_core::guid::HighGuid::GameObject,
            0,
            1,
            571,
            0,
            777,
            23,
        );
        let bytes = GameObjectSetStateLocal {
            object_guid: guid,
            state: 2,
        }
        .to_bytes();
        assert_eq!(
            bytes[0..2],
            (ServerOpcodes::GameObjectSetStateLocal as u16).to_le_bytes()
        );
        assert_eq!(&bytes[2..18], &guid.to_raw_bytes());
        assert_eq!(bytes[18], 2);
        assert_eq!(bytes.len(), 19);
    }

    #[test]
    fn update_world_state_writes_visible_default_false_layout_like_cpp() {
        let bytes = UpdateWorldState::new(0x1234_5678, 42).to_bytes();

        assert_eq!(
            bytes[0..2],
            (ServerOpcodes::UpdateWorldState as u16).to_le_bytes()
        );
        assert_eq!(&bytes[2..6], &0x1234_5678_u32.to_le_bytes());
        assert_eq!(&bytes[6..10], &42_i32.to_le_bytes());
        assert_eq!(bytes[10], 0x00);
        assert_eq!(bytes.len(), 11);
    }

    #[test]
    fn update_world_state_writes_hidden_true_bit_like_cpp() {
        let bytes = UpdateWorldState {
            variable_id: 9001,
            value: -7,
            hidden: true,
        }
        .to_bytes();

        assert_eq!(
            bytes[0..2],
            (ServerOpcodes::UpdateWorldState as u16).to_le_bytes()
        );
        assert_eq!(&bytes[2..6], &9001_u32.to_le_bytes());
        assert_eq!(&bytes[6..10], &(-7_i32).to_le_bytes());
        assert_eq!(bytes[10], 0x80);
        assert_eq!(bytes.len(), 11);
    }

    #[test]
    fn update_capture_point_writes_cpp_capture_point_info() {
        let guid = ObjectGuid::create_world_object(
            wow_core::guid::HighGuid::GameObject,
            0,
            1,
            571,
            0,
            777,
            24,
        );
        let bytes = UpdateCapturePoint {
            guid,
            position: Position::new(12.5, 34.25, 56.0, 1.0),
            state: 2,
            capture_time_ms: 15_000,
            capture_total_duration_ms: 60_000,
        }
        .to_bytes();
        assert_eq!(
            bytes[0..2],
            (ServerOpcodes::UpdateCapturePoint as u16).to_le_bytes()
        );
        assert_eq!(&bytes[2..18], &guid.to_raw_bytes());
        assert_eq!(&bytes[18..22], &12.5_f32.to_le_bytes());
        assert_eq!(&bytes[22..26], &34.25_f32.to_le_bytes());
        assert_eq!(bytes[26], 2);
        assert_eq!(&bytes[27..31], &15_000_u32.to_le_bytes());
        assert_eq!(&bytes[31..35], &60_000_u32.to_le_bytes());
        assert_eq!(bytes.len(), 35);

        let captured_bytes = UpdateCapturePoint {
            guid,
            position: Position::new(12.5, 34.25, 56.0, 1.0),
            state: 4,
            capture_time_ms: 0,
            capture_total_duration_ms: 60_000,
        }
        .to_bytes();
        assert_eq!(captured_bytes[26], 4);
        assert_eq!(captured_bytes.len(), 27);
    }

    #[test]
    fn page_text_writes_gameobject_guid_like_cpp() {
        let guid = ObjectGuid::create_world_object(
            wow_core::guid::HighGuid::GameObject,
            0,
            1,
            571,
            0,
            777,
            23,
        );
        let bytes = PageText {
            gameobject_guid: guid,
        }
        .to_bytes();
        assert_eq!(bytes[0..2], (ServerOpcodes::PageText as u16).to_le_bytes());
        assert_eq!(&bytes[2..18], &guid.to_raw_bytes());
        assert_eq!(bytes.len(), 18);
    }

    #[test]
    fn anim_kit_packets_write_unit_guid_and_anim_kit_id_like_cpp() {
        let guid = ObjectGuid::create_world_object(
            wow_core::guid::HighGuid::Creature,
            0,
            1,
            571,
            0,
            1234,
            99,
        );

        for (bytes, opcode, anim_kit_id) in [
            (
                SetAiAnimKit {
                    unit: guid,
                    anim_kit_id: 11,
                }
                .to_bytes(),
                ServerOpcodes::SetAiAnimKit,
                11_u16,
            ),
            (
                SetMovementAnimKit {
                    unit: guid,
                    anim_kit_id: 22,
                }
                .to_bytes(),
                ServerOpcodes::SetMovementAnimKit,
                22_u16,
            ),
            (
                SetMeleeAnimKit {
                    unit: guid,
                    anim_kit_id: 33,
                }
                .to_bytes(),
                ServerOpcodes::SetMeleeAnimKit,
                33_u16,
            ),
        ] {
            assert_eq!(bytes[0..2], (opcode as u16).to_le_bytes());
            assert_eq!(&bytes[2..18], &guid.to_raw_bytes());
            assert_eq!(&bytes[18..20], &anim_kit_id.to_le_bytes());
            assert_eq!(bytes.len(), 20);
        }
    }

    #[test]
    fn trigger_cinematic_writes_id_and_conversation_guid_like_cpp() {
        let bytes = TriggerCinematic {
            cinematic_id: 444,
            conversation_guid: ObjectGuid::EMPTY,
        }
        .to_bytes();
        assert_eq!(
            bytes[0..2],
            (ServerOpcodes::TriggerCinematic as u16).to_le_bytes()
        );
        assert_eq!(&bytes[2..6], &444_u32.to_le_bytes());
        assert_eq!(&bytes[6..22], &ObjectGuid::EMPTY.to_raw_bytes());
        assert_eq!(bytes.len(), 22);
    }

    #[test]
    fn trigger_movie_writes_movie_id_like_cpp() {
        let bytes = TriggerMovie { movie_id: 7788 }.to_bytes();
        assert_eq!(
            bytes[0..2],
            (ServerOpcodes::TriggerMovie as u16).to_le_bytes()
        );
        assert_eq!(&bytes[2..6], &7788_u32.to_le_bytes());
        assert_eq!(bytes.len(), 6);
    }

    #[test]
    fn far_sight_reads_enable_bit_true_and_false_like_cpp() {
        for enable in [false, true] {
            let mut pkt = WorldPacket::new_empty();
            pkt.write_bit(enable);
            pkt.flush_bits();
            pkt.reset_read();

            let far_sight = FarSight::read(&mut pkt).unwrap();
            assert_eq!(far_sight.enable, enable);
        }
    }

    #[test]
    fn buy_bank_slot_reads_full_guid_like_cpp() {
        let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 12, 34);
        let mut pkt = WorldPacket::new_empty();
        pkt.write_guid(&guid);
        pkt.reset_read();

        let buy = BuyBankSlot::read(&mut pkt).unwrap();
        assert_eq!(buy.guid, guid);
        assert_eq!(pkt.remaining(), 0);
    }

    #[test]
    fn change_bank_bag_slot_flag_reads_slot_flag_and_enabled_bit_like_cpp() {
        let mut pkt = WorldPacket::new_empty();
        pkt.write_uint32(3);
        pkt.write_uint32(5);
        pkt.write_bit(true);
        pkt.flush_bits();
        pkt.reset_read();

        let change = ChangeBankBagSlotFlag::read(&mut pkt).unwrap();
        assert_eq!(change.slot, 3);
        assert_eq!(change.flag, 5);
        assert!(change.enabled);
        assert_eq!(pkt.remaining(), 0);
    }

    #[test]
    fn auto_bank_item_reads_inv_bag_slot_like_cpp() {
        let mut pkt = WorldPacket::new_empty();
        pkt.write_bits(1, 2);
        pkt.flush_bits();
        pkt.write_uint8(255);
        pkt.write_uint8(19);
        pkt.write_uint8(255);
        pkt.write_uint8(19);
        pkt.reset_read();

        let packet = AutoBankItem::read(&mut pkt).unwrap();
        assert_eq!(packet.inv_update.items, vec![(255, 19)]);
        assert_eq!(packet.bag, 255);
        assert_eq!(packet.slot, 19);
        assert_eq!(pkt.remaining(), 0);
    }

    #[test]
    fn auto_store_bank_item_reads_inv_bag_slot_like_cpp() {
        let mut pkt = WorldPacket::new_empty();
        pkt.write_bits(1, 2);
        pkt.flush_bits();
        pkt.write_uint8(255);
        pkt.write_uint8(39);
        pkt.write_uint8(255);
        pkt.write_uint8(39);
        pkt.reset_read();

        let packet = AutoStoreBankItem::read(&mut pkt).unwrap();
        assert_eq!(packet.inv_update.items, vec![(255, 39)]);
        assert_eq!(packet.bag, 255);
        assert_eq!(packet.slot, 39);
        assert_eq!(pkt.remaining(), 0);
    }

    #[test]
    fn guild_bank_activate_reads_guid_then_full_update_bit_like_cpp() {
        let banker = ObjectGuid::new(0x0102_0304_0506_0708_i64, 0x1112_1314_1516_1718_i64);
        let mut pkt = WorldPacket::new_empty();
        pkt.write_guid(&banker);
        pkt.write_bit(true);
        pkt.flush_bits();
        pkt.reset_read();

        let parsed = GuildBankActivate::read(&mut pkt).unwrap();

        assert_eq!(parsed.banker, banker);
        assert!(parsed.full_update);
        assert_eq!(pkt.remaining(), 0);
    }

    #[test]
    fn guild_bank_query_tab_reads_guid_tab_then_full_update_bit_like_cpp() {
        let banker = ObjectGuid::new(0x2122_2324_2526_2728_i64, 0x3132_3334_3536_3738_i64);
        let mut pkt = WorldPacket::new_empty();
        pkt.write_guid(&banker);
        pkt.write_uint8(4);
        pkt.write_bit(false);
        pkt.flush_bits();
        pkt.reset_read();

        let parsed = GuildBankQueryTab::read(&mut pkt).unwrap();

        assert_eq!(parsed.banker, banker);
        assert_eq!(parsed.tab, 4);
        assert!(!parsed.full_update);
        assert_eq!(pkt.remaining(), 0);
    }

    #[test]
    fn guild_command_result_player_not_in_guild_view_tab_matches_cpp_shape() {
        let bytes = GuildCommandResult::player_not_in_guild_view_tab_like_cpp().to_bytes();
        assert_eq!(
            u16::from_le_bytes([bytes[0], bytes[1]]),
            ServerOpcodes::GuildCommandResult as u16
        );
        let mut pkt = WorldPacket::from_bytes(&bytes[2..]);

        assert_eq!(
            pkt.read_int32().unwrap(),
            GuildCommandResult::ERR_PLAYER_NOT_IN_GUILD_LIKE_CPP
        );
        assert_eq!(
            pkt.read_int32().unwrap(),
            GuildCommandResult::COMMAND_VIEW_TAB_LIKE_CPP
        );
        assert_eq!(pkt.read_bits(8).unwrap(), 0);
        assert_eq!(pkt.remaining(), 0);
    }

    #[test]
    fn auto_guild_bank_item_reads_cpp_field_order_with_optional_container_slot() {
        let banker = ObjectGuid::new(0x0102_0304_0506_0708_i64, 0x1112_1314_1516_1718_i64);
        let mut pkt = WorldPacket::new_empty();
        pkt.write_guid(&banker);
        pkt.write_uint8(2);
        pkt.write_uint8(14);
        pkt.write_uint8(22);
        pkt.write_bit(true);
        pkt.flush_bits();
        pkt.write_uint8(5);
        pkt.reset_read();

        let parsed = AutoGuildBankItem::read(&mut pkt).unwrap();

        assert_eq!(parsed.banker, banker);
        assert_eq!(parsed.bank_tab, 2);
        assert_eq!(parsed.bank_slot, 14);
        assert_eq!(parsed.container_item_slot, 22);
        assert_eq!(parsed.container_slot, Some(5));
        assert_eq!(pkt.remaining(), 0);
    }

    #[test]
    fn auto_store_guild_bank_item_reads_cpp_field_order() {
        let banker = ObjectGuid::new(0x2122_2324_2526_2728_i64, 0x3132_3334_3536_3738_i64);
        let mut pkt = WorldPacket::new_empty();
        pkt.write_guid(&banker);
        pkt.write_uint8(3);
        pkt.write_uint8(19);
        pkt.reset_read();

        let parsed = AutoStoreGuildBankItem::read(&mut pkt).unwrap();

        assert_eq!(parsed.banker, banker);
        assert_eq!(parsed.bank_tab, 3);
        assert_eq!(parsed.bank_slot, 19);
        assert_eq!(pkt.remaining(), 0);
    }

    #[test]
    fn bug_report_reads_type_diag_and_text_like_cpp() {
        let mut pkt = WorldPacket::new_empty();
        pkt.write_bit(true);
        pkt.write_bits(4, 12);
        pkt.write_bits(3, 10);
        pkt.flush_bits();
        pkt.write_string("diag");
        pkt.write_string("bug");
        pkt.reset_read();

        let report = BugReport::read(&mut pkt).unwrap();
        assert_eq!(report.report_type, 1);
        assert_eq!(report.diag_info, "diag");
        assert_eq!(report.text, "bug");
        assert_eq!(pkt.remaining(), 0);
    }

    #[test]
    fn object_update_recovery_reads_guid_like_cpp() {
        let guid = ObjectGuid::create_player(1, 42);
        let mut failed = WorldPacket::new_empty();
        failed.write_packed_guid(&guid);
        failed.reset_read();
        assert_eq!(
            ObjectUpdateFailed::read(&mut failed).unwrap(),
            ObjectUpdateFailed { object_guid: guid }
        );

        let rescued_guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 7, 9);
        let mut rescued = WorldPacket::new_empty();
        rescued.write_packed_guid(&rescued_guid);
        rescued.reset_read();
        assert_eq!(
            ObjectUpdateRescued::read(&mut rescued).unwrap(),
            ObjectUpdateRescued {
                object_guid: rescued_guid
            }
        );
    }

    #[test]
    fn stand_state_change_reads_raw_uint32_like_cpp() {
        let mut pkt = WorldPacket::new_empty();
        pkt.write_uint32(8);
        pkt.reset_read();

        assert_eq!(
            StandStateChange::read(&mut pkt).unwrap(),
            StandStateChange { stand_state: 8 }
        );
        assert_eq!(pkt.remaining(), 0);
    }
}

// ── SMSG_LOG_XP_GAIN ─────────────────────────────────────────────────────────

/// Floating text "+XP" on screen when player earns experience.
/// C# ref: LogXPGain
pub struct LogXpGain {
    pub victim: ObjectGuid,
    pub original: i32, // XP before bonuses
    pub reason: u8,    // 0=Kill, 1=NoKill(quest/explore)
    pub amount: i32,   // XP after bonuses (what actually counts)
    pub group_bonus: f32,
}

impl ServerPacket for LogXpGain {
    const OPCODE: ServerOpcodes = ServerOpcodes::LogXpGain;
    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_packed_guid(&self.victim);
        pkt.write_int32(self.original);
        pkt.write_uint8(self.reason);
        pkt.write_int32(self.amount);
        pkt.write_float(self.group_bonus);
    }
}

// ── SMSG_LEVELUP_INFO ────────────────────────────────────────────────────────

/// "Ding!" level-up popup with stat deltas.
/// C# ref: LevelUpInfo — PowerDelta[10] + StatDelta[5]
pub struct LevelUpInfo {
    pub level: i32,
    pub health_delta: i32,
    pub power_delta: [i32; 10], // PowerType::MaxPerClass = 10
    pub stat_delta: [i32; 5],   // Stats::Max = 5 (Str/Agi/Sta/Int/Spi)
    pub num_new_talents: i32,
}

impl ServerPacket for LevelUpInfo {
    const OPCODE: ServerOpcodes = ServerOpcodes::LevelUpInfo;
    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_int32(self.level);
        pkt.write_int32(self.health_delta);
        for p in &self.power_delta {
            pkt.write_int32(*p);
        }
        for s in &self.stat_delta {
            pkt.write_int32(*s);
        }
        pkt.write_int32(self.num_new_talents);
    }
}
