// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Social, group, guild, mail, auction and trade packets.

use super::*;

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

/// C++ `WorldPackets::Duel::DuelRequested`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DuelRequested {
    pub arbiter_guid: ObjectGuid,
    pub requested_by_guid: ObjectGuid,
    pub requested_by_wow_account: ObjectGuid,
    pub to_the_death: bool,
}

impl ServerPacket for DuelRequested {
    const OPCODE: ServerOpcodes = ServerOpcodes::DuelRequested;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_bytes(&self.arbiter_guid.to_raw_bytes());
        pkt.write_bytes(&self.requested_by_guid.to_raw_bytes());
        pkt.write_bytes(&self.requested_by_wow_account.to_raw_bytes());
        pkt.write_bit(self.to_the_death);
        pkt.flush_bits();
    }
}

// ── FarSight (CMSG 0x34e8) ──────────────────────────────────────────

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

/// C++ `WorldPackets::Ticket::SupportTicketHeader`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SupportTicketHeader {
    pub map_id: i32,
    pub position: Position,
    pub facing: f32,
    pub program: i32,
}

impl SupportTicketHeader {
    pub(super) fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
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

/// C++ `WorldPackets::Talent::TalentGroupInfo`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TalentGroupInfoLikeCpp {
    pub spec_id: u8,
    pub talents: Vec<TalentInfoLikeCpp>,
    pub glyph_ids: [u16; MAX_GLYPH_SLOT_INDEX_LIKE_CPP],
}

impl Default for TalentGroupInfoLikeCpp {
    fn default() -> Self {
        Self {
            spec_id: 0,
            talents: Vec::new(),
            glyph_ids: [0; MAX_GLYPH_SLOT_INDEX_LIKE_CPP],
        }
    }
}

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

/// C++ `CMSG_SHOW_TRADE_SKILL` is handled as `WorldPackets::Null`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShowTradeSkill;

impl crate::ClientPacket for ShowTradeSkill {
    const OPCODE: wow_constants::ClientOpcodes = wow_constants::ClientOpcodes::ShowTradeSkill;

    fn read(packet: &mut crate::WorldPacket) -> Result<Self, crate::world_packet::PacketError> {
        let remaining = packet.remaining();
        if remaining > 0 {
            let _ = packet.read_bytes(remaining)?;
        }
        Ok(Self)
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

/// SMSG_AUCTION_HELLO_RESPONSE — opens the auction house UI on the client.
/// C++ ref: `WorldPackets::AuctionHouse::AuctionHelloResponse`.
pub struct AuctionHelloResponse {
    /// GUID of the auctioneer NPC.
    pub auctioneer_guid: wow_core::ObjectGuid,
    /// Delay in ms before purchased items are delivered.
    pub purchased_item_delivery_delay: u32,
    /// Delay in ms before cancelled items are returned.
    pub cancelled_item_delivery_delay: u32,
    /// Whether the auction house is currently open for business.
    pub open_for_business: bool,
}

impl AuctionHelloResponse {
    /// Convenience: open neutral auction house for a given NPC guid.
    pub fn open(auctioneer_guid: wow_core::ObjectGuid) -> Self {
        Self {
            auctioneer_guid,
            purchased_item_delivery_delay: 0,
            cancelled_item_delivery_delay: 0,
            open_for_business: true,
        }
    }
}

impl ServerPacket for AuctionHelloResponse {
    const OPCODE: ServerOpcodes = ServerOpcodes::AuctionHelloResponse;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_packed_guid(&self.auctioneer_guid);
        pkt.write_uint32(self.purchased_item_delivery_delay);
        pkt.write_uint32(self.cancelled_item_delivery_delay);
        pkt.write_bit(self.open_for_business);
        pkt.flush_bits();
    }
}

// ── NpcInteractionOpenResult ──────────────────────────────────────────────────

/// C++ `WorldPackets::Addon::AddOnInfo`, used by auction-house taint metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuctionAddonInfo {
    pub name: String,
    pub version: String,
    pub loaded: bool,
    pub disabled: bool,
}

impl AuctionAddonInfo {
    pub(super) fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
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

/// C++ `WorldPackets::Mail::MailQueryNextTimeResult::MailNextTimeEntry`.
#[derive(Debug, Clone, PartialEq)]
pub struct MailNextTimeEntry {
    pub sender_guid: ObjectGuid,
    pub time_left: f32,
    pub alt_sender_id: i32,
    pub alt_sender_type: i8,
    pub stationery_id: i32,
}

impl MailNextTimeEntry {
    fn write_like_cpp(&self, pkt: &mut WorldPacket) {
        pkt.write_packed_guid(&self.sender_guid);
        pkt.write_float(self.time_left);
        pkt.write_int32(self.alt_sender_id);
        pkt.write_int8(self.alt_sender_type);
        pkt.write_int32(self.stationery_id);
    }
}

/// C++ `WorldPackets::Mail::MailQueryNextTimeResult`.
pub struct MailQueryNextTimeResult {
    pub next_mail_time: f32,
    pub next: Vec<MailNextTimeEntry>,
}

impl MailQueryNextTimeResult {
    pub fn no_mail() -> Self {
        Self {
            next_mail_time: -86_400.0,
            next: Vec::new(),
        }
    }
}

impl ServerPacket for MailQueryNextTimeResult {
    const OPCODE: ServerOpcodes = ServerOpcodes::MailQueryNextTimeResult;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_float(self.next_mail_time);
        pkt.write_int32(self.next.len() as i32);
        for entry in &self.next {
            entry.write_like_cpp(pkt);
        }
    }
}

// ── LFG list status ──────────────────────────────────────────────────────────

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
    pub(super) fn read_like_cpp(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
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

    pub(super) fn write_like_cpp(&self, pkt: &mut WorldPacket) {
        pkt.write_packed_guid(&self.requester_guid);
        pkt.write_uint32(self.id);
        pkt.write_uint32(self.ride_type);
        pkt.write_int64(self.time);
        pkt.write_bit(self.unknown925);
        pkt.flush_bits();
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

/// C++ `WorldPackets::Calendar::CalendarRaidLockoutAdded`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CalendarRaidLockoutAdded {
    pub instance_id: u64,
    pub server_time_packed: u32,
    pub map_id: i32,
    pub difficulty_id: u32,
    pub time_remaining: i32,
}

impl CalendarRaidLockoutAdded {
    pub fn new_at_unix(
        instance_id: u64,
        unix_seconds: i64,
        map_id: i32,
        difficulty_id: u32,
        time_remaining: i32,
    ) -> Self {
        Self {
            instance_id,
            server_time_packed: wow_time_packed_from_unix_seconds(unix_seconds),
            map_id,
            difficulty_id,
            time_remaining,
        }
    }
}

impl ServerPacket for CalendarRaidLockoutAdded {
    const OPCODE: ServerOpcodes = ServerOpcodes::CalendarRaidLockoutAdded;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_uint64(self.instance_id);
        pkt.write_uint32(self.server_time_packed);
        pkt.write_int32(self.map_id);
        pkt.write_uint32(self.difficulty_id);
        pkt.write_int32(self.time_remaining);
    }
}

/// C++ `WorldPackets::Calendar::CalendarRaidLockoutRemoved`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CalendarRaidLockoutRemoved {
    pub instance_id: u64,
    pub map_id: i32,
    pub difficulty_id: u32,
}

impl ServerPacket for CalendarRaidLockoutRemoved {
    const OPCODE: ServerOpcodes = ServerOpcodes::CalendarRaidLockoutRemoved;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_uint64(self.instance_id);
        pkt.write_int32(self.map_id);
        pkt.write_uint32(self.difficulty_id);
    }
}

/// C++ `WorldPackets::Calendar::CalendarRaidLockoutUpdated`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CalendarRaidLockoutUpdated {
    pub server_time_packed: u32,
    pub map_id: i32,
    pub difficulty_id: u32,
    pub old_time_remaining: i32,
    pub new_time_remaining: i32,
}

impl CalendarRaidLockoutUpdated {
    pub fn new_at_unix(
        unix_seconds: i64,
        map_id: i32,
        difficulty_id: u32,
        old_time_remaining: i32,
        new_time_remaining: i32,
    ) -> Self {
        Self {
            server_time_packed: wow_time_packed_from_unix_seconds(unix_seconds),
            map_id,
            difficulty_id,
            old_time_remaining,
            new_time_remaining,
        }
    }
}

impl ServerPacket for CalendarRaidLockoutUpdated {
    const OPCODE: ServerOpcodes = ServerOpcodes::CalendarRaidLockoutUpdated;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_uint32(self.server_time_packed);
        pkt.write_int32(self.map_id);
        pkt.write_uint32(self.difficulty_id);
        pkt.write_int32(self.old_time_remaining);
        pkt.write_int32(self.new_time_remaining);
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
