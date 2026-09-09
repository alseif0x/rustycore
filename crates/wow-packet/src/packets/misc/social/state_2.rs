//! Duel and social packets state definitions, part 2 of 3.
//!
//! Separated from the social.rs root under #650. Behaviour is preserved.

use super::*;

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
    pub(in crate::packets::misc) fn write_like_cpp(&self, pkt: &mut WorldPacket) {
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
    pub(in crate::packets::misc) fn read_like_cpp(
        pkt: &mut WorldPacket,
    ) -> Result<Self, PacketError> {
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

    pub(in crate::packets::misc) fn write_like_cpp(&self, pkt: &mut WorldPacket) {
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
    pub(in crate::packets::misc) fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
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
