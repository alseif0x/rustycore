//! Duel and social packets state definitions, part 1 of 3.
//!
//! Separated from the social.rs root under #650. Behaviour is preserved.

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
    pub(in crate::packets::misc) fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
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
    pub(in crate::packets::misc) fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
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
    pub(in crate::packets::misc) fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
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
    pub(in crate::packets::misc) fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
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
    pub(in crate::packets::misc) fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
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
    pub(in crate::packets::misc) fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
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
    pub(in crate::packets::misc) fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
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
    pub(in crate::packets::misc) fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
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
    pub(in crate::packets::misc) fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
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
    pub(in crate::packets::misc) fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
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
    pub(in crate::packets::misc) fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
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
    pub(in crate::packets::misc) fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
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
    pub(in crate::packets::misc) fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
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

/// C++ `WorldPackets::Addon::AddOnInfo`, used by auction-house taint metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuctionAddonInfo {
    pub name: String,
    pub version: String,
    pub loaded: bool,
    pub disabled: bool,
}

impl AuctionAddonInfo {
    pub(in crate::packets::misc) fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
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
