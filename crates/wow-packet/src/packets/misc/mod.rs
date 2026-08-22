// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Miscellaneous packets, organised by protocol family.
//!
//! Issue #227 split the former 13,185-line `misc.rs` by protocol family.
//! Every public type and byte contract is unchanged.

mod character;
mod combat;
mod movement;
mod session;
mod social;
mod spell;
mod world_state;

pub use character::*;
pub use combat::*;
pub use movement::*;
pub use session::*;
pub use social::*;
pub use spell::*;
pub use world_state::*;

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

pub const SUPPORT_SPAM_TYPE_MAIL_LIKE_CPP: u8 = 0;

pub const SUPPORT_SPAM_TYPE_CHAT_LIKE_CPP: u8 = 1;

pub const SUPPORT_SPAM_TYPE_CALENDAR_LIKE_CPP: u8 = 2;

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

/// C++ `WorldPackets::Misc::StandStateUpdate`.
///
/// `Unit::SetStandState` sends this directly to players after updating the
/// canonical `UnitData::StandState` field and removing Standing-interrupt
/// auras. The wire order is `AnimKitID` followed by the stand state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StandStateUpdate {
    pub anim_kit_id: u32,
    pub stand_state: u8,
}

impl ServerPacket for StandStateUpdate {
    const OPCODE: ServerOpcodes = ServerOpcodes::StandStateUpdate;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_uint32(self.anim_kit_id);
        pkt.write_uint8(self.stand_state);
    }
}

pub const ERR_TAXIOK_LIKE_CPP: u8 = 0;

pub const ERR_TAXITOOFARAWAY_LIKE_CPP: u8 = 4;

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

/// Number of `AccountDataType` values in TrinityCore (`NUM_ACCOUNT_DATA_TYPES`).
pub const NUM_ACCOUNT_DATA_TYPES: usize = 15;

pub const MAX_ACCOUNT_DATA_SIZE_LIKE_CPP: u32 = 0xFFFF;

pub const EMPTY_ACCOUNT_DATA_COMPRESS_BOUND_LIKE_CPP: usize = 13;

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

pub const TUTORIAL_ACTION_UPDATE_LIKE_CPP: u8 = 0;

pub const TUTORIAL_ACTION_CLEAR_LIKE_CPP: u8 = 1;

pub const TUTORIAL_ACTION_RESET_LIKE_CPP: u8 = 2;

/// Empty packet sent when a fishing bobber is clicked before a fish is hooked.
pub struct FishNotHooked;

impl ServerPacket for FishNotHooked {
    const OPCODE: ServerOpcodes = ServerOpcodes::FishNotHooked;

    fn write(&self, _pkt: &mut WorldPacket) {}
}

// ── EnableBarberShop (SMSG 0x26bc) ──────────────────────────────────

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

pub const MAX_GLYPH_SLOT_INDEX_LIKE_CPP: usize = 6;

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

/// C++ `EQUIPMENT_SET_SLOTS` / `EQUIPMENT_SLOT_END`.
pub const EQUIPMENT_SET_SLOTS_LIKE_CPP: usize = 19;

pub const MOUNT_RESULT_SHAPESHIFTED_LIKE_CPP: i32 = 8;

// ── MountSpecial (CMSG 0x3280) / SpecialMountAnim (SMSG 0x269f) ─────

pub const MAX_CUF_PROFILES_LIKE_CPP: usize = 5;

pub const CUF_BOOL_OPTIONS_COUNT_LIKE_CPP: usize = 27;

pub const MAX_DECLINED_NAME_CASES_LIKE_CPP: usize = 5;

/// C++ `DeclinedName`, represented for battle-pet rename packets.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeclinedNamesLikeCpp {
    pub names: [String; MAX_DECLINED_NAME_CASES_LIKE_CPP],
}

pub fn empty_battle_pet_guid_like_cpp() -> ObjectGuid {
    ObjectGuid::create_global(HighGuid::BattlePet, 0, 0)
}

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

    /// Reply with Status::RecordRemoved (2) — record was explicitly removed by
    /// the server hotfix stream. The client must not use its local DB2 copy.
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

/// C++ `WorldPackets::Misc::PortGraveyard`: empty packet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PortGraveyard;

impl ClientPacket for PortGraveyard {
    // The inspected TrinityCore 3.4.3 opcode table uses the shared unresolved
    // `0xBADD` placeholder. Route it from the existing 0xBADD slot by empty
    // payload shape in `WorldSession`.
    const OPCODE: ClientOpcodes = ClientOpcodes::SetLootSpecialization;

    fn read(packet: &mut WorldPacket) -> Result<Self, PacketError> {
        if packet.is_empty() {
            Ok(Self)
        } else {
            Err(PacketError::ReadPastEnd {
                wanted: 0,
                available: packet.remaining(),
            })
        }
    }
}

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
// C++ refs: `WorldPackets::NPC::VendorInventory` (`Server/Packets/NPCPackets.cpp:152-160`)
// and item vendor packets (`Server/Packets/ItemPackets.cpp:26-52,130-135`).

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

// ── RequestCemeteryListResponse (SMSG 0x258F) ────────────────────────────────
/// Response to CMSG_REQUEST_CEMETERY_LIST.
/// C++ ref: `WorldPackets::Misc::RequestCemeteryListResponse::Write`.
pub struct RequestCemeteryListResponse {
    pub is_gossip_triggered: bool,
    pub cemetery_ids: Vec<u32>,
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

/// C++ `lfg::LFG_QUEUE_DUNGEON`.
pub const LFG_QUEUE_DUNGEON_LIKE_CPP: u8 = 1;

/// C++ `lfg::LFG_UPDATETYPE_REMOVED_FROM_QUEUE`.
pub const LFG_UPDATE_TYPE_REMOVED_FROM_QUEUE_LIKE_CPP: u8 = 8;

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

/// C++ `TOKEN_RESULT_SUCCESS`.
pub const TOKEN_RESULT_SUCCESS_LIKE_CPP: u32 = 0;

pub const RATED_PVP_BRACKET_COUNT_LIKE_CPP: usize = 7;

/// Area discovery XP notification.
/// C++ `WorldPackets::Misc::ExplorationExperience::Write`.
pub struct ExplorationExperience {
    pub area_id: i32,
    pub experience: i32,
}

impl ServerPacket for ExplorationExperience {
    const OPCODE: ServerOpcodes = ServerOpcodes::ExplorationExperience;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_int32(self.area_id);
        pkt.write_int32(self.experience);
    }
}

#[cfg(test)]
mod exploration_experience_tests {
    use super::*;

    #[test]
    fn exploration_experience_writes_area_then_xp_like_cpp() {
        let bytes = ExplorationExperience {
            area_id: 9_001,
            experience: 345,
        }
        .to_bytes();

        let mut pkt = WorldPacket::from_bytes(&bytes);
        assert_eq!(
            pkt.server_opcode(),
            Some(ServerOpcodes::ExplorationExperience)
        );
        pkt.skip_opcode();
        assert_eq!(pkt.read_int32().unwrap(), 9_001);
        assert_eq!(pkt.read_int32().unwrap(), 345);
        assert_eq!(pkt.remaining(), 0);
    }
}

// ── SMSG_LEVELUP_INFO ────────────────────────────────────────────────────────

/// "Ding!" level-up popup with stat deltas.
/// C++ `WorldPackets::Misc::LevelUpInfo::Write` — `PowerDelta[10]`
/// followed by `StatDelta[5]` and `NumNewTalents`.
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

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
