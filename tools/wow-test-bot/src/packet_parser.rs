//! Packet parsers for SMSG opcodes
//! Decodes server-to-client packets into structured data

use tracing::debug;

/// Parsed SMSG_LFG_JOIN_RESULT
#[derive(Debug, Clone)]
pub struct LfgJoinResult {
    pub result: u8,
    pub result_detail: u8,
    pub queue_id: u32,
    pub wait_time: u32,
    pub queue_length: u32,
}

/// Parsed SMSG_LFG_PLAYER_INFO
#[derive(Debug, Clone)]
pub struct LfgPlayerInfo {
    pub queue_id: u32,
    pub dungeon_id: u32,
    pub status: u8,
}

#[derive(Debug, Clone)]
pub struct LfgQueueStatus {
    pub queue_id: u32,
    pub dungeon_id: u32,
    pub queued_time: u32,
    pub wait_time_avg: u32,
    pub wait_time_tank: u32,
    pub wait_time_healer: u32,
    pub wait_time_dps: u32,
    pub tanks_needed: u8,
    pub healers_needed: u8,
    pub dps_needed: u8,
}

#[derive(Debug, Clone)]
pub struct LfgUpdateStatus {
    pub dungeon_id: u32,
    pub status: u8,
    pub reason: u8,
    pub slots_count: usize,
    pub suspended_players_count: usize,
}

/// Parsed SMSG_AUTH_RESPONSE
#[derive(Debug, Clone)]
pub struct AuthResponse {
    pub result: u8,
    pub has_account_data: bool,
}

#[derive(Debug, Clone)]
pub struct QuestOffer {
    pub quest_id: u32,
    pub title: String,
}

#[derive(Debug, Clone)]
pub struct QuestDetailsSummary {
    pub quest_id: u32,
}

#[derive(Debug, Clone)]
pub struct QuestRequestItemsSummary {
    pub quest_id: u32,
    pub status_flags: u32,
}

#[derive(Debug, Clone)]
pub struct TrainerListSummary {
    pub trainer_id: i32,
    pub spell_count: u32,
}

/// Decode a TC 9.x packed ObjectGuid (`u8 lowMask | u8 highMask | <packed bytes>`).
/// Returns the number of bytes consumed plus the low/high u64 components.
pub fn parse_packed_guid(data: &[u8]) -> Option<(usize, u64, u64)> {
    if data.len() < 2 {
        return None;
    }
    let low_mask = data[0];
    let high_mask = data[1];
    let mut pos = 2usize;
    let mut low: u64 = 0;
    let mut high: u64 = 0;
    for i in 0..8 {
        if low_mask & (1 << i) != 0 {
            if pos >= data.len() {
                return None;
            }
            low |= (data[pos] as u64) << (i * 8);
            pos += 1;
        }
    }
    for i in 0..8 {
        if high_mask & (1 << i) != 0 {
            if pos >= data.len() {
                return None;
            }
            high |= (data[pos] as u64) << (i * 8);
            pos += 1;
        }
    }
    Some((pos, low, high))
}

fn read_u32(data: &[u8], pos: usize) -> Option<u32> {
    if pos + 4 > data.len() {
        return None;
    }
    Some(u32::from_le_bytes([
        data[pos],
        data[pos + 1],
        data[pos + 2],
        data[pos + 3],
    ]))
}

fn read_i32(data: &[u8], pos: usize) -> Option<i32> {
    read_u32(data, pos).map(|v| v as i32)
}

pub fn parse_gossip_id(data: &[u8]) -> Option<i32> {
    let (pos, _low, _high) = parse_packed_guid(data)?;
    read_i32(data, pos)
}

struct BitReader<'a> {
    data: &'a [u8],
    pos: usize,
    bit_pos: u8,
    bit_buf: u8,
}

impl<'a> BitReader<'a> {
    fn new(data: &'a [u8], pos: usize) -> Self {
        Self {
            data,
            pos,
            bit_pos: 0,
            bit_buf: 0,
        }
    }

    fn read_bit(&mut self) -> Option<bool> {
        if self.bit_pos == 0 {
            self.bit_buf = *self.data.get(self.pos)?;
            self.pos += 1;
            self.bit_pos = 8;
        }
        self.bit_pos -= 1;
        Some((self.bit_buf >> self.bit_pos) & 1 == 1)
    }

    fn read_bits(&mut self, n: u8) -> Option<u32> {
        let mut value = 0u32;
        for _ in 0..n {
            value = (value << 1) | u32::from(self.read_bit()?);
        }
        Some(value)
    }

    fn flushed_pos(self) -> usize {
        self.pos
    }
}

fn parse_quest_offer(data: &[u8], mut pos: usize) -> Option<(usize, QuestOffer)> {
    let quest_id = read_u32(data, pos)?;
    pos += 4;
    pos += 4; // ContentTuningID
    pos += 4; // QuestType
    pos += 4; // QuestLevel
    pos += 4; // QuestMaxScalingLevel
    pos += 4; // QuestFlags
    pos += 4; // QuestFlagsEx
    if pos > data.len() {
        return None;
    }

    let mut bits = BitReader::new(data, pos);
    bits.read_bit()?; // Repeatable
    bits.read_bit()?; // Important
    let title_len = bits.read_bits(9)? as usize;
    pos = bits.flushed_pos();

    let title_bytes = data.get(pos..pos + title_len)?;
    pos += title_len;
    let title = String::from_utf8_lossy(title_bytes).to_string();
    Some((pos, QuestOffer { quest_id, title }))
}

pub fn parse_gossip_quest_offers(data: &[u8]) -> Option<Vec<QuestOffer>> {
    let (mut pos, _low, _high) = parse_packed_guid(data)?;
    pos += 4; // GossipID
    pos += 4; // FriendshipFactionID
    let option_count = usize::try_from(read_i32(data, pos)?).ok()?;
    pos += 4;
    let quest_count = usize::try_from(read_i32(data, pos)?).ok()?;
    pos += 4;
    if pos > data.len() {
        return None;
    }

    let mut bits = BitReader::new(data, pos);
    let has_text_id = bits.read_bit()?;
    let has_broadcast_text_id = bits.read_bit()?;
    pos = bits.flushed_pos();

    for _ in 0..option_count {
        pos += 4; // GossipOptionID
        pos += 1; // OptionNpc
        pos += 1; // OptionFlags
        pos += 4; // OptionCost
        pos += 4; // OptionLanguage
        pos += 4; // Flags
        pos += 4; // OrderIndex
        if pos > data.len() {
            return None;
        }

        let mut option_bits = BitReader::new(data, pos);
        let text_len = option_bits.read_bits(12)? as usize;
        let confirm_len = option_bits.read_bits(12)? as usize;
        option_bits.read_bits(2)?; // Status
        let has_spell_id = option_bits.read_bit()?;
        let has_override_icon_id = option_bits.read_bit()?;
        pos = option_bits.flushed_pos();

        let items_count = usize::try_from(read_i32(data, pos)?).ok()?;
        pos += 4;
        pos += items_count.saturating_mul(4); // Current Rust writer only emits zero items.
        pos += text_len;
        pos += confirm_len;
        if has_spell_id {
            pos += 4;
        }
        if has_override_icon_id {
            pos += 4;
        }
        if pos > data.len() {
            return None;
        }
    }

    if has_text_id {
        pos += 4;
    }
    if has_broadcast_text_id {
        pos += 4;
    }
    if pos > data.len() {
        return None;
    }

    let mut quests = Vec::with_capacity(quest_count);
    for _ in 0..quest_count {
        let (next, quest) = parse_quest_offer(data, pos)?;
        quests.push(quest);
        pos = next;
    }
    Some(quests)
}

pub fn parse_quest_list_offers(data: &[u8]) -> Option<Vec<QuestOffer>> {
    let (mut pos, _low, _high) = parse_packed_guid(data)?;
    pos += 4; // GreetEmoteDelay
    pos += 4; // GreetEmoteType
    let quest_count = read_u32(data, pos)? as usize;
    pos += 4;
    if pos > data.len() {
        return None;
    }

    let mut bits = BitReader::new(data, pos);
    let greeting_len = bits.read_bits(11)? as usize;
    pos = bits.flushed_pos();

    let mut quests = Vec::with_capacity(quest_count);
    for _ in 0..quest_count {
        let (next, quest) = parse_quest_offer(data, pos)?;
        quests.push(quest);
        pos = next;
    }
    data.get(pos..pos + greeting_len)?;
    Some(quests)
}

pub fn parse_quest_details_summary(data: &[u8]) -> Option<QuestDetailsSummary> {
    let (mut pos, _low, _high) = parse_packed_guid(data)?;
    let (inform_guid_len, _low, _high) = parse_packed_guid(data.get(pos..)?)?;
    pos += inform_guid_len;
    let quest_id = read_u32(data, pos)?;
    Some(QuestDetailsSummary { quest_id })
}

pub fn parse_quest_request_items_summary(data: &[u8]) -> Option<QuestRequestItemsSummary> {
    let (mut pos, _low, _high) = parse_packed_guid(data)?;
    pos += 4; // GiverCreatureID
    let quest_id = read_u32(data, pos)?;
    pos += 4;
    pos += 4; // CompEmoteDelay
    pos += 4; // CompEmoteType
    pos += 12; // QuestFlags[3]
    pos += 4; // SuggestedPartyMembers
    pos += 4; // MoneyToGet
    let collect_count = read_i32(data, pos)?.max(0) as usize;
    pos += 4;
    let currency_count = read_i32(data, pos)?.max(0) as usize;
    pos += 4;
    let status_flags = read_u32(data, pos)?;
    pos += 4;
    pos += collect_count.checked_mul(12)?;
    pos += currency_count.checked_mul(8)?;
    data.get(pos)?;
    Some(QuestRequestItemsSummary {
        quest_id,
        status_flags,
    })
}

pub fn parse_trainer_list_summary(data: &[u8]) -> Option<TrainerListSummary> {
    let (mut pos, _low, _high) = parse_packed_guid(data)?;
    pos += 4; // TrainerType
    let trainer_id = read_i32(data, pos)?;
    pos += 4;
    let spell_count_i32 = read_i32(data, pos)?;
    let spell_count = u32::try_from(spell_count_i32).ok()?;
    Some(TrainerListSummary {
        trainer_id,
        spell_count,
    })
}

fn skip_ride_ticket(data: &[u8]) -> Option<usize> {
    let (mut pos, _guid_low, _guid_high) = parse_packed_guid(data)?;
    if pos + 4 + 4 + 8 + 1 > data.len() {
        return None;
    }
    pos += 4; // Id
    pos += 4; // Type
    pos += 8; // Time
    pos += 1; // Unknown925 bit + FlushBits
    Some(pos)
}

pub fn parse_lfg_queue_status(data: &[u8]) -> Option<LfgQueueStatus> {
    let mut pos = skip_ride_ticket(data)?;
    if pos + 31 > data.len() {
        return None;
    }
    let queue_id = read_u32(data, pos)?;
    pos += 4;
    let dungeon_id = read_u32(data, pos)?;
    pos += 4;
    let wait_time_avg = read_u32(data, pos)?;
    pos += 4;
    let wait_time_tank = read_u32(data, pos)?;
    pos += 4;
    let tanks_needed = data[pos];
    pos += 1;
    let wait_time_healer = read_u32(data, pos)?;
    pos += 4;
    let healers_needed = data[pos];
    pos += 1;
    let wait_time_dps = read_u32(data, pos)?;
    pos += 4;
    let dps_needed = data[pos];
    pos += 1;
    let queued_time = read_u32(data, pos)?;
    Some(LfgQueueStatus {
        queue_id,
        dungeon_id,
        queued_time,
        wait_time_avg,
        wait_time_tank,
        wait_time_healer,
        wait_time_dps,
        tanks_needed,
        healers_needed,
        dps_needed,
    })
}

pub fn parse_lfg_update_status(data: &[u8]) -> Option<LfgUpdateStatus> {
    let mut pos = skip_ride_ticket(data)?;
    if pos + 14 > data.len() {
        return None;
    }
    let status = data[pos];
    pos += 1;
    let reason = data[pos];
    pos += 1;
    let slots_count = read_u32(data, pos)? as usize;
    pos += 4;
    let _requested_roles = data[pos];
    pos += 1;
    let suspended_count = read_u32(data, pos)? as usize;
    pos += 4;
    let dungeon_id = read_u32(data, pos)?;
    pos += 4;
    if pos + slots_count.saturating_mul(4) > data.len() {
        return None;
    }
    Some(LfgUpdateStatus {
        dungeon_id,
        status,
        reason,
        slots_count,
        suspended_players_count: suspended_count,
    })
}

/// Parse SMSG_LFG_JOIN_RESULT (opcode 0x2A1C).
///
/// Wire layout (TrinityCore `WorldPackets::LFG::LFGJoinResult::Write`):
///   RideTicket {
///     PackedGuid RequesterGuid,    // 2 mask bytes + variable data
///     uint32 Id,
///     uint32 Type,
///     int32  Time,
///     bit    Unknown925,           // followed by FlushBits → 1 byte
///   }
///   uint8  Result,
///   uint8  ResultDetail,
///   uint32 BlackList.size(),
///   uint32 BlackListNames.size(),
///   ...
pub fn parse_lfg_join_result(data: &[u8]) -> Option<LfgJoinResult> {
    let (mut pos, _guid_low, _guid_high) = parse_packed_guid(data)?;
    // RideTicket.Id, .Type, .Time, then a 1-bit Unknown925 padded to a full byte by FlushBits.
    if pos + 4 + 4 + 4 + 1 + 1 + 1 > data.len() {
        return None;
    }
    let queue_id = u32::from_le_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]]);
    pos += 4;
    pos += 4; // RideType (uint32)
    let wait_time = u32::from_le_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]]);
    pos += 4;
    pos += 1; // Unknown925 bit + FlushBits

    let result = data[pos];
    let result_detail = data[pos + 1];

    debug!(
        "Parsed SMSG_LFG_JOIN_RESULT: result={}, detail={}, queue_id={}, time={}",
        result, result_detail, queue_id, wait_time
    );

    Some(LfgJoinResult {
        result,
        result_detail,
        queue_id,
        wait_time,
        queue_length: 0,
    })
}

/// Parse SMSG_AUTH_RESPONSE (opcode 0x5AB6)
pub fn parse_auth_response(data: &[u8]) -> Option<AuthResponse> {
    if data.is_empty() {
        return None;
    }

    let result = data[0];
    let has_account_data = data.len() > 1;

    Some(AuthResponse {
        result,
        has_account_data,
    })
}

/// Get human-readable LFG result string
pub fn lfg_result_to_string(result: u8) -> &'static str {
    match result {
        0 => "OK",
        1 => "ROLE_CHECK_FAILED",
        2 => "GROUP_NOT_FULL",
        3 => "ALREADY_IN_GROUP",
        4 => "ALREADY_IN_LFG",
        5 => "INTERNAL_ERROR",
        _ => "UNKNOWN",
    }
}

/// Get human-readable LFG detail string
pub fn lfg_detail_to_string(detail: u8) -> &'static str {
    match detail {
        0 => "NONE",
        1 => "GROUP_TOO_SMALL",
        2 => "GROUP_TOO_LARGE",
        3 => "ROLE_CHECK_FAILED",
        4 => "INSUFFICIENT_EXPANSION",
        5 => "LEVEL_TOO_LOW",
        _ => "UNKNOWN",
    }
}

pub fn teleport_denied_to_string(reason: u8) -> &'static str {
    match reason {
        0 => "NONE",
        1 => "DEAD",
        2 => "FALLING",
        4 => "FATIGUE",
        6 => "NO_RETURN_LOCATION",
        _ => "UNKNOWN",
    }
}

/// Parse packet based on opcode
pub fn parse_packet(opcode: u16, data: &[u8]) -> String {
    match opcode {
        0x2A1C => {
            if let Some(result) = parse_lfg_join_result(data) {
                format!(
                    "SMSG_LFG_JOIN_RESULT: {} ({}), detail: {} ({}), queue_id={}, wait={}s",
                    result.result,
                    lfg_result_to_string(result.result),
                    result.result_detail,
                    lfg_detail_to_string(result.result_detail),
                    result.queue_id,
                    result.wait_time
                )
            } else {
                format!("SMSG_LFG_JOIN_RESULT: <parse failed, {} bytes>", data.len())
            }
        }
        0x5AB6 => {
            if let Some(auth) = parse_auth_response(data) {
                format!(
                    "SMSG_AUTH_RESPONSE: result={}, has_data={}",
                    auth.result, auth.has_account_data
                )
            } else {
                format!("SMSG_AUTH_RESPONSE: <parse failed, {} bytes>", data.len())
            }
        }
        0x5AED => "SMSG_ENTER_ENCRYPTED_MODE".to_string(),
        0x3048 => "SMSG_AUTH_CHALLENGE".to_string(),
        // Opcodes verified against src/server/game/Server/Protocol/Opcodes.h.
        // The previous 0x2A24 → SMSG_LFG_PROPOSAL_UPDATE mapping was wrong:
        // 0x2A24 = SMSG_LFG_UPDATE_STATUS, 0x2A2D = SMSG_LFG_PROPOSAL_UPDATE.
        0x2A2D => format!("SMSG_LFG_PROPOSAL_UPDATE ({} bytes)", data.len()),
        0x2A24 => {
            if let Some(status) = parse_lfg_update_status(data) {
                format!(
                    "SMSG_LFG_UPDATE_STATUS: dungeon_id={}, status={}, reason={}, slots={}, suspended_players={}",
                    status.dungeon_id,
                    status.status,
                    status.reason,
                    status.slots_count,
                    status.suspended_players_count
                )
            } else {
                format!("SMSG_LFG_UPDATE_STATUS ({} bytes)", data.len())
            }
        }
        0x2A20 => {
            if let Some(status) = parse_lfg_queue_status(data) {
                format!("SMSG_LFG_QUEUE_STATUS: queue_id={}, dungeon_id={}, queued={}s, wait_avg={}s, wait_tank={}s, wait_healer={}s, wait_dps={}s, needs={}/{}/{}",
                    status.queue_id, status.dungeon_id, status.queued_time, status.wait_time_avg,
                    status.wait_time_tank, status.wait_time_healer, status.wait_time_dps,
                    status.tanks_needed, status.healers_needed, status.dps_needed)
            } else {
                format!("SMSG_LFG_QUEUE_STATUS ({} bytes)", data.len())
            }
        }
        0x2A22 => format!("SMSG_LFG_READY_CHECK_UPDATE ({} bytes)", data.len()),
        0x2A36 => format!("SMSG_LFG_PARTY_INFO ({} bytes)", data.len()),
        0x2A37 => format!("SMSG_LFG_PLAYER_INFO ({} bytes)", data.len()),
        0x2A38 => format!("SMSG_LFG_PLAYER_REWARD ({} bytes)", data.len()),
        0x2A35 => format!("SMSG_LFG_BOOT_PLAYER ({} bytes)", data.len()),
        0x2A3A => format!("SMSG_LFG_READY_CHECK_RESULT ({} bytes)", data.len()),
        0x2683 => format!("SMSG_LOGOUT_RESPONSE ({} bytes)", data.len()),
        0x2684 => "SMSG_LOGOUT_COMPLETE".to_string(),
        0x2A32 => {
            let raw = data.first().copied().unwrap_or(0);
            let reason = raw >> 4;
            format!(
                "SMSG_LFG_TELEPORT_DENIED: reason={} ({}) raw=0x{:02X}",
                reason,
                teleport_denied_to_string(reason),
                raw
            )
        }
        0x2A91 => format!("SMSG_QUEST_GIVER_STATUS_MULTIPLE ({} bytes)", data.len()),
        0x2A83 => {
            if data.len() >= 4 {
                let quest_id = u32::from_le_bytes(data[0..4].try_into().ok().unwrap_or([0; 4]));
                format!("SMSG_QUEST_GIVER_QUEST_COMPLETE: quest_id={}", quest_id)
            } else {
                format!("SMSG_QUEST_GIVER_QUEST_COMPLETE ({} bytes)", data.len())
            }
        }
        0x2A92 => {
            if let Some(details) = parse_quest_details_summary(data) {
                format!(
                    "SMSG_QUEST_GIVER_QUEST_DETAILS: quest_id={}",
                    details.quest_id
                )
            } else {
                format!("SMSG_QUEST_GIVER_QUEST_DETAILS ({} bytes)", data.len())
            }
        }
        0x2A93 => {
            if let Some(summary) = parse_quest_request_items_summary(data) {
                format!(
                    "SMSG_QUEST_GIVER_REQUEST_ITEMS: quest_id={} status_flags=0x{:X}",
                    summary.quest_id, summary.status_flags
                )
            } else {
                format!("SMSG_QUEST_GIVER_REQUEST_ITEMS ({} bytes)", data.len())
            }
        }
        0x2A98 => {
            if let Some(offers) = parse_gossip_quest_offers(data) {
                let ids: Vec<u32> = offers.iter().map(|offer| offer.quest_id).collect();
                format!("SMSG_GOSSIP_MESSAGE: quests={:?}", ids)
            } else {
                format!("SMSG_GOSSIP_MESSAGE ({} bytes)", data.len())
            }
        }
        0x2A9A => {
            if let Some(offers) = parse_quest_list_offers(data) {
                let ids: Vec<u32> = offers.iter().map(|offer| offer.quest_id).collect();
                format!("SMSG_QUEST_GIVER_QUEST_LIST: quests={:?}", ids)
            } else {
                format!("SMSG_QUEST_GIVER_QUEST_LIST ({} bytes)", data.len())
            }
        }
        0x2A9B => format!("SMSG_QUEST_GIVER_STATUS ({} bytes)", data.len()),
        0x26DF => {
            if let Some(summary) = parse_trainer_list_summary(data) {
                format!(
                    "SMSG_TRAINER_LIST: trainer_id={} spells={}",
                    summary.trainer_id, summary.spell_count
                )
            } else {
                format!("SMSG_TRAINER_LIST ({} bytes)", data.len())
            }
        }
        _ => format!("Unknown opcode 0x{:04X} ({} bytes)", opcode, data.len()),
    }
}
