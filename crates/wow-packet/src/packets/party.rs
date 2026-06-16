//! Party / Group packets (WoTLK 3.4.3).
//! C# reference: Source/Game/Networking/Packets/PartyPackets.cs

use crate::{ClientPacket, ServerPacket, WorldPacket};
use wow_constants::{ClientOpcodes, ServerOpcodes};
use wow_core::{ObjectGuid, Position};

use crate::world_packet::PacketError;

// ── PartyCommandResult (SMSG_PARTY_COMMAND_RESULT 0x2796) ────────────────────

/// Sent to the inviting player to confirm or reject the operation.
pub struct PartyCommandResult {
    pub name: String, // target name
    pub command: u8,  // PartyOperation: 0=Invite, 1=Uninvite, 2=Leave, 4=Swap
    pub result: u8,   // PartyResult enum (see below)
    pub result_data: u32,
    pub result_guid: ObjectGuid,
}

/// PartyResult enum values (result field above)
pub mod party_result {
    pub const OK: u8 = 0;
    pub const BAD_PLAYER_NAME: u8 = 1;
    pub const TARGET_NOT_IN_GROUP: u8 = 2;
    pub const WRONG_FACTION: u8 = 7;
    pub const ALREADY_IN_GROUP: u8 = 8;
    pub const NOT_IN_GROUP: u8 = 6;
    pub const NOT_LEADER_LIKE_CPP: u8 = 7;
    pub const NOT_LEADER: u8 = 14;
    pub const GROUP_FULL: u8 = 3;
}

// ── ConvertRaid (CMSG_CONVERT_RAID) ─────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConvertRaid {
    pub raid: bool,
}

impl ClientPacket for ConvertRaid {
    const OPCODE: ClientOpcodes = ClientOpcodes::ConvertRaid;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self {
            raid: pkt.read_bit()?,
        })
    }
}

// ── ChangeSubGroup (CMSG_CHANGE_SUB_GROUP) ─────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChangeSubGroup {
    pub target_guid: ObjectGuid,
    pub party_index: Option<u8>,
    pub new_subgroup: u8,
}

impl ClientPacket for ChangeSubGroup {
    const OPCODE: ClientOpcodes = ClientOpcodes::ChangeSubGroup;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        let target_guid = pkt.read_packed_guid()?;
        let new_subgroup = pkt.read_uint8()?;
        let party_index = if pkt.read_bit()? {
            Some(pkt.read_uint8()?)
        } else {
            None
        };

        Ok(Self {
            target_guid,
            party_index,
            new_subgroup,
        })
    }
}

// ── SetAssistantLeader (CMSG_SET_ASSISTANT_LEADER) ─────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SetPartyLeader {
    pub target_guid: ObjectGuid,
    pub party_index: Option<u8>,
}

impl ClientPacket for SetPartyLeader {
    const OPCODE: ClientOpcodes = ClientOpcodes::SetPartyLeader;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        let has_party_index = pkt.read_bit()?;
        let target_guid = pkt.read_packed_guid()?;
        let party_index = if has_party_index {
            Some(pkt.read_uint8()?)
        } else {
            None
        };

        Ok(Self {
            target_guid,
            party_index,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SetAssistantLeader {
    pub target: ObjectGuid,
    pub apply: bool,
    pub party_index: Option<u8>,
}

impl ClientPacket for SetAssistantLeader {
    const OPCODE: ClientOpcodes = ClientOpcodes::SetAssistantLeader;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        let has_party_index = pkt.read_bit()?;
        let apply = pkt.read_bit()?;
        let target = pkt.read_packed_guid()?;
        let party_index = if has_party_index {
            Some(pkt.read_uint8()?)
        } else {
            None
        };

        Ok(Self {
            target,
            apply,
            party_index,
        })
    }
}

// ── PartyUninvite (CMSG_PARTY_UNINVITE) ─────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PartyUninvite {
    pub target_guid: ObjectGuid,
    pub party_index: Option<u8>,
    pub reason: String,
}

impl ClientPacket for PartyUninvite {
    const OPCODE: ClientOpcodes = ClientOpcodes::PartyUninvite;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        let has_party_index = pkt.read_bit()?;
        let reason_len = pkt.read_bits(8)? as usize;
        let target_guid = pkt.read_guid()?;
        let party_index = if has_party_index {
            Some(pkt.read_uint8()?)
        } else {
            None
        };
        let reason = pkt.read_string(reason_len)?;

        Ok(Self {
            target_guid,
            party_index,
            reason,
        })
    }
}

// ── SetEveryoneIsAssistant (CMSG_SET_EVERYONE_IS_ASSISTANT) ─────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SetEveryoneIsAssistant {
    pub everyone_is_assistant: bool,
    pub party_index: Option<u8>,
}

impl ClientPacket for SetEveryoneIsAssistant {
    const OPCODE: ClientOpcodes = ClientOpcodes::SetEveryoneIsAssistant;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        let has_party_index = pkt.read_bit()?;
        let everyone_is_assistant = pkt.read_bit()?;
        let party_index = if has_party_index {
            Some(pkt.read_uint8()?)
        } else {
            None
        };

        Ok(Self {
            everyone_is_assistant,
            party_index,
        })
    }
}

// ── SilencePartyTalker (CMSG_SILENCE_PARTY_TALKER) ─────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SilencePartyTalker {
    pub target: ObjectGuid,
    pub silent: bool,
}

impl ClientPacket for SilencePartyTalker {
    const OPCODE: ClientOpcodes = ClientOpcodes::SilencePartyTalker;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        let guid_bytes = pkt.read_bytes(16)?;
        let mut raw = [0u8; 16];
        raw.copy_from_slice(&guid_bytes);
        Ok(Self {
            target: ObjectGuid::from_raw_bytes(&raw),
            silent: pkt.read_bit()?,
        })
    }
}

// ── SetPartyAssignment (CMSG_SET_PARTY_ASSIGNMENT) ─────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SetPartyAssignment {
    pub assignment: u8,
    pub party_index: Option<u8>,
    pub target: ObjectGuid,
    pub apply: bool,
}

impl ClientPacket for SetPartyAssignment {
    const OPCODE: ClientOpcodes = ClientOpcodes::SetPartyAssignment;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        let has_party_index = pkt.read_bit()?;
        let apply = pkt.read_bit()?;
        let assignment = pkt.read_uint8()?;
        let target = pkt.read_packed_guid()?;
        let party_index = if has_party_index {
            Some(pkt.read_uint8()?)
        } else {
            None
        };

        Ok(Self {
            assignment,
            party_index,
            target,
            apply,
        })
    }
}

// ── Role poll / LFG roles (CMSG_SET_ROLE / CMSG_INITIATE_ROLE_POLL) ───────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SetRole {
    pub target_guid: ObjectGuid,
    pub role: u8,
    pub party_index: Option<u8>,
}

impl ClientPacket for SetRole {
    const OPCODE: ClientOpcodes = ClientOpcodes::SetRole;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        let has_party_index = pkt.read_bit()?;
        let target_guid = pkt.read_packed_guid()?;
        let role = pkt.read_uint8()?;
        let party_index = if has_party_index {
            Some(pkt.read_uint8()?)
        } else {
            None
        };

        Ok(Self {
            target_guid,
            role,
            party_index,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InitiateRolePoll {
    pub party_index: Option<u8>,
}

impl ClientPacket for InitiateRolePoll {
    const OPCODE: ClientOpcodes = ClientOpcodes::InitiateRolePoll;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        let party_index = if pkt.read_bit()? {
            Some(pkt.read_uint8()?)
        } else {
            None
        };

        Ok(Self { party_index })
    }
}

// ── Raid target icons / join updates ──────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UpdateRaidTarget {
    pub party_index: Option<u8>,
    pub target: ObjectGuid,
    pub symbol: i8,
}

impl ClientPacket for UpdateRaidTarget {
    const OPCODE: ClientOpcodes = ClientOpcodes::UpdateRaidTarget;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        let has_party_index = pkt.read_bit()?;
        let target = pkt.read_packed_guid()?;
        let symbol = pkt.read_int8()?;
        let party_index = if has_party_index {
            Some(pkt.read_uint8()?)
        } else {
            None
        };

        Ok(Self {
            party_index,
            target,
            symbol,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RequestPartyJoinUpdates {
    pub party_index: Option<u8>,
}

impl ClientPacket for RequestPartyJoinUpdates {
    const OPCODE: ClientOpcodes = ClientOpcodes::RequestPartyJoinUpdates;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        let party_index = if pkt.read_bit()? {
            Some(pkt.read_uint8()?)
        } else {
            None
        };

        Ok(Self { party_index })
    }
}

// ── RequestPartyMemberStats (CMSG_REQUEST_PARTY_MEMBER_STATS) ────────────────

/// Client requests one party member's current full-state snapshot.
///
/// C++ anchor: `WorldPackets::Party::RequestPartyMemberStats::Read()`
/// (`PartyPackets.cpp:135-141`) reads bit `hasPartyIndex`, then `TargetGUID`,
/// then optional `PartyIndex`. The handler reads but ignores `PartyIndex`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RequestPartyMemberStats {
    pub target_guid: ObjectGuid,
    pub party_index: Option<u8>,
}

impl ClientPacket for RequestPartyMemberStats {
    const OPCODE: ClientOpcodes = ClientOpcodes::RequestPartyMemberStats;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        let has_party_index = pkt.read_bit()?;
        let target_guid = pkt.read_packed_guid()?;
        let party_index = if has_party_index {
            Some(pkt.read_uint8()?)
        } else {
            None
        };

        Ok(Self {
            target_guid,
            party_index,
        })
    }
}

// ── ReadyCheck (CMSG_DO_READY_CHECK / CMSG_READY_CHECK_RESPONSE) ──────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DoReadyCheck {
    pub party_index: Option<u8>,
}

impl ClientPacket for DoReadyCheck {
    const OPCODE: ClientOpcodes = ClientOpcodes::DoReadyCheck;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        let party_index = if pkt.read_bit()? {
            Some(pkt.read_uint8()?)
        } else {
            None
        };

        Ok(Self { party_index })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReadyCheckResponseClient {
    pub is_ready: bool,
    pub party_index: Option<u8>,
}

impl ClientPacket for ReadyCheckResponseClient {
    const OPCODE: ClientOpcodes = ClientOpcodes::ReadyCheckResponse;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        let is_ready = pkt.read_bit()?;
        let party_index = if pkt.read_bit()? {
            Some(pkt.read_uint8()?)
        } else {
            None
        };

        Ok(Self {
            is_ready,
            party_index,
        })
    }
}

// ── SwapSubGroups (CMSG_SWAP_SUB_GROUPS) ─────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SwapSubGroups {
    pub first_target: ObjectGuid,
    pub second_target: ObjectGuid,
    pub party_index: Option<u8>,
}

impl ClientPacket for SwapSubGroups {
    const OPCODE: ClientOpcodes = ClientOpcodes::SwapSubGroups;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        let has_party_index = pkt.read_bit()?;
        let first_target = pkt.read_packed_guid()?;
        let second_target = pkt.read_packed_guid()?;
        let party_index = if has_party_index {
            Some(pkt.read_uint8()?)
        } else {
            None
        };

        Ok(Self {
            first_target,
            second_target,
            party_index,
        })
    }
}

// ── SetLootMethod (CMSG_SET_LOOT_METHOD) ─────────────────────────

/// Client request to change party loot method.
#[derive(Debug, Clone)]
pub struct SetLootMethod {
    pub party_index: Option<u8>,
    pub loot_master_guid: ObjectGuid,
    pub loot_method: u8,
    pub loot_threshold: u32,
}

impl ClientPacket for SetLootMethod {
    const OPCODE: ClientOpcodes = ClientOpcodes::SetLootMethod;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        let has_party_index = pkt.read_bit()?;
        let loot_method = pkt.read_uint8()?;
        let loot_master_guid = pkt.read_packed_guid()?;
        let loot_threshold = pkt.read_uint32()?;
        let party_index = if has_party_index {
            Some(pkt.read_uint8()?)
        } else {
            None
        };

        Ok(Self {
            party_index,
            loot_master_guid,
            loot_method,
            loot_threshold,
        })
    }
}

// ── OptOutOfLoot (CMSG_OPT_OUT_OF_LOOT) ─────────────────────────

/// Client toggles automatic pass on group-loot rolls.
#[derive(Debug, Clone)]
pub struct OptOutOfLoot {
    pub pass_on_loot: bool,
}

impl ClientPacket for OptOutOfLoot {
    const OPCODE: ClientOpcodes = ClientOpcodes::OptOutOfLoot;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self {
            pass_on_loot: pkt.read_bit()?,
        })
    }
}

// ── MinimapPingClient (CMSG_MINIMAP_PING 0x364E) ────────────────────────

/// Client minimap ping packet.
///
/// C++ anchor: `WorldPackets::Party::MinimapPingClient::Read()`
/// (`PartyPackets.cpp:304-311` / `PartyPackets.h:292-302`)
///
/// Wire order: bit `hasPartyIndex`, float `PositionX`, float `PositionY`,
/// optional u8 `PartyIndex` when bit is set.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MinimapPingClient {
    pub position_x: f32,
    pub position_y: f32,
    pub party_index: Option<u8>,
}

impl ClientPacket for MinimapPingClient {
    const OPCODE: ClientOpcodes = ClientOpcodes::MinimapPing;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        let has_party_index = pkt.read_bit()?;
        let position_x = pkt.read_float()?;
        let position_y = pkt.read_float()?;
        let party_index = if has_party_index {
            Some(pkt.read_uint8()?)
        } else {
            None
        };

        Ok(Self {
            position_x,
            position_y,
            party_index,
        })
    }
}

// ── MinimapPing (SMSG_MINIMAP_PING 0x26CE) ──────────────────────────────

/// Server minimap ping broadcast packet.
///
/// C++ anchor: `WorldPackets::Party::MinimapPing::Write()`
/// (`PartyPackets.cpp:313-319` / `PartyPackets.h:304-314`)
///
/// Wire order: packed `Sender`, float `PositionX`, float `PositionY`.
pub struct MinimapPing {
    pub sender: ObjectGuid,
    pub position_x: f32,
    pub position_y: f32,
}

impl ServerPacket for MinimapPing {
    const OPCODE: ServerOpcodes = ServerOpcodes::MinimapPing;

    fn write(&self, w: &mut WorldPacket) {
        w.write_packed_guid(&self.sender);
        w.write_float(self.position_x);
        w.write_float(self.position_y);
    }
}

// ── LowLevelRaid1 (CMSG_LOW_LEVEL_RAID1 0x36A1) ─────────────────────────

/// No-op: C++ `WorldPackets::Party::LowLevelRaid1` has empty `Read()`.
/// Handler only logs at DEBUG level; no state mutation, no packet send.
#[derive(Debug, Clone, Copy)]
pub struct LowLevelRaid1;

impl ClientPacket for LowLevelRaid1 {
    const OPCODE: ClientOpcodes = ClientOpcodes::LowLevelRaid1;

    fn read(_pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self)
    }
}

// ── LowLevelRaid2 (CMSG_LOW_LEVEL_RAID2 0x3512) ─────────────────────────

/// No-op: C++ `WorldPackets::Party::LowLevelRaid2` has empty `Read()`.
/// Handler only logs at DEBUG level; no state mutation, no packet send.
#[derive(Debug, Clone, Copy)]
pub struct LowLevelRaid2;

impl ClientPacket for LowLevelRaid2 {
    const OPCODE: ClientOpcodes = ClientOpcodes::LowLevelRaid2;

    fn read(_pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self)
    }
}

impl ServerPacket for PartyCommandResult {
    const OPCODE: ServerOpcodes = ServerOpcodes::PartyCommandResult;
    fn write(&self, w: &mut WorldPacket) {
        let name_bytes = self.name.as_bytes();
        w.write_bits(name_bytes.len() as u32, 9);
        w.write_bits(self.command as u32, 4);
        w.write_bits(self.result as u32, 6);
        w.write_uint32(self.result_data);
        w.write_packed_guid(&self.result_guid);
        w.write_bytes(name_bytes);
    }
}

// ── PartyInvite (SMSG_PARTY_INVITE 0x25bd) ────────────────────────────────────

/// Sent to the INVITED player so they see the invite dialog.
pub struct PartyInviteServer {
    pub can_accept: bool,
    pub inviter_name: String,
    pub inviter_guid: ObjectGuid,
    pub inviter_bnet_account_guid: ObjectGuid,
    pub virtual_realm_address: u32,
    pub realm_name: String,
    pub realm_name_normalized: String,
}

impl ServerPacket for PartyInviteServer {
    const OPCODE: ServerOpcodes = ServerOpcodes::PartyInvite;
    fn write(&self, w: &mut WorldPacket) {
        let name_bytes = self.inviter_name.as_bytes();
        w.write_bit(self.can_accept);
        w.write_bit(false); // MightCRZYou
        w.write_bit(false); // IsXRealm
        w.write_bit(false); // MustBeBNetFriend
        w.write_bit(true); // AllowMultipleRoles
        w.write_bit(false); // QuestSessionActive
        w.write_bits(name_bytes.len() as u32, 6);
        // VirtualRealmInfo.Write():
        w.write_uint32(self.virtual_realm_address); // RealmAddress
        // VirtualRealmNameInfo.Write():
        w.write_bit(true); // IsLocal = true
        w.write_bit(false); // IsInternalRealm = false
        let realm_bytes = self.realm_name.as_bytes();
        let realm_norm_bytes = self.realm_name_normalized.as_bytes();
        w.write_bits(realm_bytes.len() as u32, 8);
        w.write_bits(realm_norm_bytes.len() as u32, 8);
        w.flush_bits();
        w.write_bytes(realm_bytes);
        w.write_bytes(realm_norm_bytes);
        // Back to PartyInvite:
        w.write_packed_guid(&self.inviter_guid);
        w.write_packed_guid(&self.inviter_bnet_account_guid);
        w.write_uint16(0); // Unk1
        w.write_uint8(0); // ProposedRoles
        w.write_int32(0); // LfgSlots.Count
        w.write_int32(0); // LfgCompletedMask
        w.write_bytes(name_bytes);
        // (no LfgSlots)
    }
}

// ── GroupDecline (SMSG_GROUP_DECLINE 0x2791) ─────────────────────────────────

/// Sent to the inviter when the target declines.
pub struct GroupDecline {
    pub name: String, // name of the decliner
}

impl ServerPacket for GroupDecline {
    const OPCODE: ServerOpcodes = ServerOpcodes::GroupDecline;
    fn write(&self, w: &mut WorldPacket) {
        let bytes = self.name.as_bytes();
        w.write_bits(bytes.len() as u32, 9);
        w.flush_bits();
        w.write_bytes(bytes);
    }
}

// ── GroupUninvite (SMSG_GROUP_UNINVITE 0x2793) ────────────────────────────────

pub struct GroupUninvite;
impl ServerPacket for GroupUninvite {
    const OPCODE: ServerOpcodes = ServerOpcodes::GroupUninvite;
    fn write(&self, _w: &mut WorldPacket) {}
}

// ── GroupDestroyed (SMSG_GROUP_DESTROYED 0x2794) ─────────────────────────────

pub struct GroupDestroyed;
impl ServerPacket for GroupDestroyed {
    const OPCODE: ServerOpcodes = ServerOpcodes::GroupDestroyed;
    fn write(&self, _w: &mut WorldPacket) {}
}

// ── PartyPlayerInfo — member entry in PartyUpdate ────────────────────────────

#[derive(Debug, Clone)]
pub struct PartyPlayerInfo {
    pub guid: ObjectGuid,
    pub name: String,
    pub class: u8,
    pub subgroup: u8,
    pub flags: u8, // GroupMemberFlags
    pub roles_assigned: u8,
    pub faction_group: u8,
    pub connected: bool,
}

impl PartyPlayerInfo {
    pub fn write(&self, w: &mut WorldPacket) {
        let name_bytes = self.name.as_bytes();
        w.write_bits(name_bytes.len() as u32, 6);
        w.write_bits(1u32, 6); // VoiceStateID len + 1 = 1 (empty string)
        w.write_bit(self.connected);
        w.write_bit(false); // VoiceChatSilenced
        w.write_bit(false); // FromSocialQueue
        w.write_packed_guid(&self.guid);
        w.write_uint8(self.subgroup);
        w.write_uint8(self.flags);
        w.write_uint8(self.roles_assigned);
        w.write_uint8(self.class);
        w.write_uint8(self.faction_group);
        w.write_bytes(name_bytes);
        // VoiceStateID is empty → nothing written (len=0, +1=1 was the bits value)
    }
}

// ── PartyUpdate (SMSG_PARTY_UPDATE 0x25f4) ───────────────────────────────────

#[derive(Debug, Clone)]
pub struct PartyLootSettings {
    pub method: u8,
    pub loot_master: ObjectGuid,
    pub threshold: u8,
}

impl PartyLootSettings {
    pub fn write(&self, w: &mut WorldPacket) {
        w.write_uint8(self.method);
        w.write_packed_guid(&self.loot_master);
        w.write_uint8(self.threshold);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReadyCheckStarted {
    pub party_index: u8,
    pub party_guid: u64,
    pub initiator_guid: ObjectGuid,
    pub duration_ms: i64,
}

impl ServerPacket for ReadyCheckStarted {
    const OPCODE: ServerOpcodes = ServerOpcodes::ReadyCheckStarted;

    fn write(&self, w: &mut WorldPacket) {
        w.write_uint8(self.party_index);
        w.write_packed_guid(&ObjectGuid::create_group(self.party_guid));
        w.write_packed_guid(&self.initiator_guid);
        w.write_int64(self.duration_ms);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReadyCheckResponse {
    pub party_guid: u64,
    pub player: ObjectGuid,
    pub is_ready: bool,
}

impl ServerPacket for ReadyCheckResponse {
    const OPCODE: ServerOpcodes = ServerOpcodes::ReadyCheckResponse;

    fn write(&self, w: &mut WorldPacket) {
        w.write_packed_guid(&ObjectGuid::create_group(self.party_guid));
        w.write_packed_guid(&self.player);
        w.write_bit(self.is_ready);
        w.flush_bits();
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReadyCheckCompleted {
    pub party_index: u8,
    pub party_guid: u64,
}

impl ServerPacket for ReadyCheckCompleted {
    const OPCODE: ServerOpcodes = ServerOpcodes::ReadyCheckCompleted;

    fn write(&self, w: &mut WorldPacket) {
        w.write_uint8(self.party_index);
        w.write_packed_guid(&ObjectGuid::create_group(self.party_guid));
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RoleChangedInform {
    pub party_index: u8,
    pub from: ObjectGuid,
    pub changed_unit: ObjectGuid,
    pub old_role: u8,
    pub new_role: u8,
}

impl ServerPacket for RoleChangedInform {
    const OPCODE: ServerOpcodes = ServerOpcodes::RoleChangedInform;

    fn write(&self, w: &mut WorldPacket) {
        w.write_uint8(self.party_index);
        w.write_packed_guid(&self.from);
        w.write_packed_guid(&self.changed_unit);
        w.write_uint8(self.old_role);
        w.write_uint8(self.new_role);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RolePollInform {
    pub party_index: i8,
    pub from: ObjectGuid,
}

impl ServerPacket for RolePollInform {
    const OPCODE: ServerOpcodes = ServerOpcodes::RolePollInform;

    fn write(&self, w: &mut WorldPacket) {
        w.write_int8(self.party_index);
        w.write_packed_guid(&self.from);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SendRaidTargetUpdateSingle {
    pub party_index: u8,
    pub target: ObjectGuid,
    pub changed_by: ObjectGuid,
    pub symbol: u8,
}

impl ServerPacket for SendRaidTargetUpdateSingle {
    const OPCODE: ServerOpcodes = ServerOpcodes::SendRaidTargetUpdateSingle;

    fn write(&self, w: &mut WorldPacket) {
        w.write_uint8(self.party_index);
        w.write_uint8(self.symbol);
        w.write_packed_guid(&self.target);
        w.write_packed_guid(&self.changed_by);
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SendRaidTargetUpdateAll {
    pub party_index: u8,
    /// C++ `SendTargetIconList` inserts all eight symbols in ascending order,
    /// including empty GUIDs.
    pub target_icons: Vec<(u8, ObjectGuid)>,
}

impl ServerPacket for SendRaidTargetUpdateAll {
    const OPCODE: ServerOpcodes = ServerOpcodes::SendRaidTargetUpdateAll;

    fn write(&self, w: &mut WorldPacket) {
        w.write_uint8(self.party_index);
        w.write_uint32(self.target_icons.len() as u32);
        for (symbol, target) in &self.target_icons {
            w.write_packed_guid(target);
            w.write_uint8(*symbol);
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RaidMarker {
    pub transport_guid: ObjectGuid,
    pub map_id: u32,
    pub position: Position,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RaidMarkersChanged {
    pub party_index: u8,
    pub active_markers: u32,
    pub raid_markers: Vec<RaidMarker>,
}

impl ServerPacket for RaidMarkersChanged {
    const OPCODE: ServerOpcodes = ServerOpcodes::RaidMarkersChanged;

    fn write(&self, w: &mut WorldPacket) {
        w.write_uint8(self.party_index);
        w.write_uint32(self.active_markers);
        w.write_bits(self.raid_markers.len() as u32, 4);
        w.flush_bits();
        for marker in &self.raid_markers {
            w.write_packed_guid(&marker.transport_guid);
            w.write_uint32(marker.map_id);
            w.write_float(marker.position.x);
            w.write_float(marker.position.y);
            w.write_float(marker.position.z);
        }
    }
}

#[derive(Debug, Clone)]
pub struct PartyDifficultySettings {
    pub dungeon_difficulty_id: u32,
    pub raid_difficulty_id: u32,
    pub legacy_raid_difficulty_id: u32,
}

impl PartyDifficultySettings {
    pub fn write(&self, w: &mut WorldPacket) {
        w.write_uint32(self.dungeon_difficulty_id);
        w.write_uint32(self.raid_difficulty_id);
        w.write_uint32(self.legacy_raid_difficulty_id);
    }
}

pub struct GroupNewLeader {
    pub party_index: i8,
    pub name: String,
}

impl ServerPacket for GroupNewLeader {
    const OPCODE: ServerOpcodes = ServerOpcodes::GroupNewLeader;

    fn write(&self, w: &mut WorldPacket) {
        w.write_int8(self.party_index);
        w.write_bits(self.name.len() as u32, 9);
        w.write_string(&self.name);
    }
}

#[derive(Debug, Clone)]
pub struct PartyUpdate {
    pub party_flags: u16, // 0 = normal
    pub party_index: u8,  // 0
    pub party_type: u8,   // 1 = Normal group
    pub my_index: i32,    // index of the receiving player in PlayerList
    pub party_guid: u64,  // group GUID
    pub sequence_num: i32,
    pub leader_guid: ObjectGuid,
    pub leader_faction_group: u8,
    pub player_list: Vec<PartyPlayerInfo>,
    pub loot_settings: Option<PartyLootSettings>,
    pub difficulty_settings: Option<PartyDifficultySettings>,
}

impl ServerPacket for PartyUpdate {
    const OPCODE: ServerOpcodes = ServerOpcodes::PartyUpdate;
    fn write(&self, w: &mut WorldPacket) {
        w.write_uint16(self.party_flags);
        w.write_uint8(self.party_index);
        w.write_uint8(self.party_type);
        w.write_int32(self.my_index);
        // PartyGUID as ObjectGuid (group GUID uses Party HighGuid)
        let group_guid = ObjectGuid::create_group(self.party_guid);
        w.write_packed_guid(&group_guid);
        w.write_int32(self.sequence_num);
        w.write_packed_guid(&self.leader_guid);
        w.write_uint8(self.leader_faction_group);
        w.write_int32(self.player_list.len() as i32);
        w.write_bit(false); // LfgInfos.HasValue
        w.write_bit(self.loot_settings.is_some());
        w.write_bit(self.difficulty_settings.is_some());
        w.flush_bits();

        for p in &self.player_list {
            p.write(w);
        }

        if let Some(ref ls) = self.loot_settings {
            ls.write(w);
        }
        if let Some(ref ds) = self.difficulty_settings {
            ds.write(w);
        }
        // (no LfgInfos)
    }
}

// ── PartyMemberFullState (SMSG_PARTY_MEMBER_FULL_STATE 0x2759) ───────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PartyMemberPhase {
    pub flags: u32,
    pub id: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PartyMemberPhaseStates {
    pub phase_shift_flags: u32,
    pub personal_guid: ObjectGuid,
    pub phases: Vec<PartyMemberPhase>,
}

impl PartyMemberPhaseStates {
    fn write(&self, w: &mut WorldPacket) {
        w.write_uint32(self.phase_shift_flags);
        w.write_uint32(self.phases.len() as u32);
        w.write_packed_guid(&self.personal_guid);

        for phase in &self.phases {
            w.write_uint32(phase.flags);
            w.write_uint16(phase.id);
        }
    }
}

impl Default for PartyMemberPhaseStates {
    fn default() -> Self {
        Self {
            phase_shift_flags: 0,
            personal_guid: ObjectGuid::EMPTY,
            phases: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PartyMemberAuraState {
    pub spell_id: i32,
    pub flags: u16,
    pub active_flags: u32,
    pub points: Vec<f32>,
}

impl PartyMemberAuraState {
    fn write(&self, w: &mut WorldPacket) {
        w.write_int32(self.spell_id);
        w.write_uint16(self.flags);
        w.write_uint32(self.active_flags);
        w.write_int32(self.points.len() as i32);
        for point in &self.points {
            w.write_float(*point);
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PartyMemberPetStats {
    pub guid: ObjectGuid,
    pub model_id: i32,
    pub current_health: i32,
    pub max_health: i32,
    pub auras: Vec<PartyMemberAuraState>,
    pub name: String,
}

impl PartyMemberPetStats {
    fn write(&self, w: &mut WorldPacket) {
        w.write_packed_guid(&self.guid);
        w.write_int32(self.model_id);
        w.write_int32(self.current_health);
        w.write_int32(self.max_health);
        w.write_uint32(self.auras.len() as u32);
        for aura in &self.auras {
            aura.write(w);
        }
        w.write_bits(self.name.len() as u32, 8);
        w.flush_bits();
        w.write_string(&self.name);
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct DungeonScoreMapSummary {
    pub challenge_mode_id: i32,
    pub map_score: f32,
    pub best_run_level: i32,
    pub best_run_duration_ms: i32,
    pub finished_success: bool,
}

impl DungeonScoreMapSummary {
    fn write(&self, w: &mut WorldPacket) {
        w.write_int32(self.challenge_mode_id);
        w.write_float(self.map_score);
        w.write_int32(self.best_run_level);
        w.write_int32(self.best_run_duration_ms);
        w.write_bit(self.finished_success);
        w.flush_bits();
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct DungeonScoreSummary {
    pub overall_score_current_season: f32,
    pub ladder_score_current_season: f32,
    pub runs: Vec<DungeonScoreMapSummary>,
}

impl DungeonScoreSummary {
    fn write(&self, w: &mut WorldPacket) {
        w.write_float(self.overall_score_current_season);
        w.write_float(self.ladder_score_current_season);
        w.write_uint32(self.runs.len() as u32);
        for run in &self.runs {
            run.write(w);
        }
    }
}

pub struct PartyMemberFullState {
    pub member_guid: ObjectGuid,
    pub for_enemy: bool,
    // Stats
    pub status: u16, // GroupMemberOnlineStatus: 0x0001=online
    pub power_type: u8,
    pub current_health: i32,
    pub max_health: i32,
    pub current_power: u16,
    pub max_power: u16,
    pub level: u16,
    pub spec_id: u16,
    pub zone_id: u16,
    pub position_x: i16,
    pub position_y: i16,
    pub position_z: i16,
    pub vehicle_seat: i32,
    pub party_type: [u8; 2],
    pub phases: PartyMemberPhaseStates,
    pub auras: Vec<PartyMemberAuraState>,
    pub pet_stats: Option<PartyMemberPetStats>,
    pub dungeon_score: DungeonScoreSummary,
}

impl ServerPacket for PartyMemberFullState {
    const OPCODE: ServerOpcodes = ServerOpcodes::PartyMemberFullState;
    fn write(&self, w: &mut WorldPacket) {
        w.write_bit(self.for_enemy);
        w.flush_bits();

        // PartyMemberStats.Write():
        w.write_uint8(self.party_type[0]);
        w.write_uint8(self.party_type[1]);
        w.write_int16(self.status as i16);
        w.write_uint8(self.power_type);
        w.write_int16(0); // PowerDisplayID
        w.write_int32(self.current_health);
        w.write_int32(self.max_health);
        w.write_uint16(self.current_power);
        w.write_uint16(self.max_power);
        w.write_uint16(self.level);
        w.write_uint16(self.spec_id);
        w.write_uint16(self.zone_id);
        w.write_uint16(0); // WmoGroupID
        w.write_uint32(0); // WmoDoodadPlacementID
        w.write_int16(self.position_x);
        w.write_int16(self.position_y);
        w.write_int16(self.position_z);
        w.write_int32(self.vehicle_seat);
        w.write_uint32(self.auras.len() as u32);

        self.phases.write(w);

        // CTROptions.Write() — empty:
        w.write_uint32(0); // ContentTuningConditionMask
        w.write_int32(0); // Unused901
        w.write_uint32(0); // ExpansionLevelMask

        for aura in &self.auras {
            aura.write(w);
        }

        w.write_bit(self.pet_stats.is_some()); // PetStats != null
        w.flush_bits();

        self.dungeon_score.write(w);

        if let Some(pet_stats) = &self.pet_stats {
            pet_stats.write(w);
        }

        w.write_packed_guid(&self.member_guid);
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ChangeSubGroup, ConvertRaid, DoReadyCheck, DungeonScoreMapSummary, DungeonScoreSummary,
        GroupNewLeader, InitiateRolePoll, LowLevelRaid1, LowLevelRaid2, MinimapPingClient,
        OptOutOfLoot, PartyMemberFullState, PartyMemberPhase, PartyMemberPhaseStates,
        PartyUninvite, RaidMarker, RaidMarkersChanged, ReadyCheckCompleted, ReadyCheckResponse,
        ReadyCheckResponseClient, ReadyCheckStarted, RequestPartyJoinUpdates,
        RequestPartyMemberStats, RoleChangedInform, RolePollInform, SendRaidTargetUpdateAll,
        SendRaidTargetUpdateSingle, SetAssistantLeader, SetEveryoneIsAssistant, SetLootMethod,
        SetPartyAssignment, SetPartyLeader, SetRole, SilencePartyTalker, SwapSubGroups,
        UpdateRaidTarget,
    };
    use crate::{ClientPacket, ServerPacket, WorldPacket};
    use wow_constants::ServerOpcodes;
    use wow_core::{ObjectGuid, Position};

    fn packed_guid_bytes(guid: ObjectGuid) -> Vec<u8> {
        let mut pkt = WorldPacket::new_empty();
        pkt.write_packed_guid(&guid);
        pkt.into_data()
    }

    #[test]
    fn party_member_full_state_writes_dungeon_score_summary_like_cpp() {
        let member_guid = ObjectGuid::create_player(1, 77);
        let mut pkt = WorldPacket::new_empty();
        PartyMemberFullState {
            member_guid,
            for_enemy: false,
            status: 1,
            power_type: 0,
            current_health: 100,
            max_health: 200,
            current_power: 10,
            max_power: 20,
            level: 80,
            spec_id: 0,
            zone_id: 571,
            position_x: 1,
            position_y: 2,
            position_z: 3,
            vehicle_seat: 0,
            party_type: [0; 2],
            phases: PartyMemberPhaseStates::default(),
            auras: Vec::new(),
            pet_stats: None,
            dungeon_score: DungeonScoreSummary {
                overall_score_current_season: 123.5,
                ladder_score_current_season: 45.25,
                runs: vec![DungeonScoreMapSummary {
                    challenge_mode_id: 2,
                    map_score: 111.0,
                    best_run_level: 7,
                    best_run_duration_ms: 900_000,
                    finished_success: true,
                }],
            },
        }
        .write(&mut pkt);
        pkt.reset_read();

        assert!(!pkt.read_bit().unwrap());
        assert_eq!(pkt.read_uint8().unwrap(), 0);
        assert_eq!(pkt.read_uint8().unwrap(), 0);
        assert_eq!(pkt.read_int16().unwrap(), 1);
        assert_eq!(pkt.read_uint8().unwrap(), 0);
        assert_eq!(pkt.read_int16().unwrap(), 0);
        assert_eq!(pkt.read_int32().unwrap(), 100);
        assert_eq!(pkt.read_int32().unwrap(), 200);
        assert_eq!(pkt.read_uint16().unwrap(), 10);
        assert_eq!(pkt.read_uint16().unwrap(), 20);
        assert_eq!(pkt.read_uint16().unwrap(), 80);
        assert_eq!(pkt.read_uint16().unwrap(), 0);
        assert_eq!(pkt.read_uint16().unwrap(), 571);
        assert_eq!(pkt.read_uint16().unwrap(), 0);
        assert_eq!(pkt.read_uint32().unwrap(), 0);
        assert_eq!(pkt.read_int16().unwrap(), 1);
        assert_eq!(pkt.read_int16().unwrap(), 2);
        assert_eq!(pkt.read_int16().unwrap(), 3);
        assert_eq!(pkt.read_int32().unwrap(), 0);
        assert_eq!(pkt.read_uint32().unwrap(), 0);
        assert_eq!(pkt.read_uint32().unwrap(), 0);
        assert_eq!(pkt.read_uint32().unwrap(), 0);
        assert_eq!(pkt.read_packed_guid().unwrap(), ObjectGuid::EMPTY);
        assert_eq!(pkt.read_uint32().unwrap(), 0);
        assert_eq!(pkt.read_uint32().unwrap(), 0);
        assert_eq!(pkt.read_int32().unwrap(), 0);
        assert!(!pkt.read_bit().unwrap());
        assert_eq!(pkt.read_float().unwrap(), 123.5);
        assert_eq!(pkt.read_float().unwrap(), 45.25);
        assert_eq!(pkt.read_uint32().unwrap(), 1);
        assert_eq!(pkt.read_int32().unwrap(), 2);
        assert_eq!(pkt.read_float().unwrap(), 111.0);
        assert_eq!(pkt.read_int32().unwrap(), 7);
        assert_eq!(pkt.read_int32().unwrap(), 900_000);
        assert!(pkt.read_bit().unwrap());
        assert_eq!(pkt.read_packed_guid().unwrap(), member_guid);
    }

    #[test]
    fn set_loot_method_reads_cpp_bit_method_master_threshold_party_index_order() {
        let master = ObjectGuid::create_player(1, 77);
        let mut pkt = WorldPacket::new_empty();
        pkt.write_bit(true);
        pkt.write_uint8(2);
        pkt.write_packed_guid(&master);
        pkt.write_uint32(4);
        pkt.write_uint8(0);
        pkt.reset_read();

        let set_loot = SetLootMethod::read(&mut pkt).unwrap();

        assert_eq!(set_loot.party_index, Some(0));
        assert_eq!(set_loot.loot_method, 2);
        assert_eq!(set_loot.loot_master_guid, master);
        assert_eq!(set_loot.loot_threshold, 4);
    }

    #[test]
    fn opt_out_of_loot_reads_cpp_pass_on_loot_bit() {
        let mut pkt = WorldPacket::new_empty();
        pkt.write_bit(true);
        pkt.flush_bits();
        pkt.reset_read();

        let opt_out = OptOutOfLoot::read(&mut pkt).unwrap();

        assert!(opt_out.pass_on_loot);
    }

    #[test]
    fn convert_raid_reads_cpp_raid_bit() {
        let mut pkt = WorldPacket::new_empty();
        pkt.write_bit(true);
        pkt.flush_bits();
        pkt.reset_read();

        let convert = ConvertRaid::read(&mut pkt).unwrap();

        assert!(convert.raid);
    }

    #[test]
    fn change_subgroup_reads_cpp_guid_subgroup_bit_party_index_order() {
        let target = ObjectGuid::create_player(1, 77);
        let mut pkt = WorldPacket::new_empty();
        pkt.write_packed_guid(&target);
        pkt.write_uint8(6);
        pkt.write_bit(true);
        pkt.write_uint8(0);
        pkt.reset_read();

        let change = ChangeSubGroup::read(&mut pkt).unwrap();

        assert_eq!(change.target_guid, target);
        assert_eq!(change.new_subgroup, 6);
        assert_eq!(change.party_index, Some(0));
    }

    #[test]
    fn set_assistant_leader_reads_cpp_has_party_apply_guid_party_index_order() {
        let target = ObjectGuid::create_player(1, 77);
        let mut pkt = WorldPacket::new_empty();
        pkt.write_bit(true);
        pkt.write_bit(true);
        pkt.write_packed_guid(&target);
        pkt.write_uint8(0);
        pkt.reset_read();

        let set_assistant = SetAssistantLeader::read(&mut pkt).unwrap();

        assert_eq!(set_assistant.target, target);
        assert!(set_assistant.apply);
        assert_eq!(set_assistant.party_index, Some(0));
    }

    #[test]
    fn set_assistant_leader_reads_cpp_optional_none_bit_before_apply() {
        let target = ObjectGuid::create_player(1, 78);
        let mut pkt = WorldPacket::new_empty();
        pkt.write_bit(false);
        pkt.write_bit(false);
        pkt.write_packed_guid(&target);
        pkt.flush_bits();
        pkt.reset_read();

        let set_assistant = SetAssistantLeader::read(&mut pkt).unwrap();

        assert_eq!(set_assistant.target, target);
        assert!(!set_assistant.apply);
        assert_eq!(set_assistant.party_index, None);
    }

    #[test]
    fn set_party_leader_reads_cpp_has_party_guid_party_index_order() {
        let target = ObjectGuid::create_player(1, 79);
        let mut pkt = WorldPacket::new_empty();
        pkt.write_bit(true);
        pkt.write_packed_guid(&target);
        pkt.write_uint8(0);
        pkt.reset_read();

        let set_leader = SetPartyLeader::read(&mut pkt).unwrap();

        assert_eq!(set_leader.target_guid, target);
        assert_eq!(set_leader.party_index, Some(0));
    }

    #[test]
    fn set_party_leader_reads_cpp_optional_none_bit_before_guid() {
        let target = ObjectGuid::create_player(1, 80);
        let mut pkt = WorldPacket::new_empty();
        pkt.write_bit(false);
        pkt.write_packed_guid(&target);
        pkt.reset_read();

        let set_leader = SetPartyLeader::read(&mut pkt).unwrap();

        assert_eq!(set_leader.target_guid, target);
        assert_eq!(set_leader.party_index, None);
    }

    #[test]
    fn set_everyone_is_assistant_reads_cpp_has_party_apply_party_index_order() {
        let mut pkt = WorldPacket::new_empty();
        pkt.write_bit(true);
        pkt.write_bit(true);
        pkt.write_uint8(0);
        pkt.reset_read();

        let set_everyone = SetEveryoneIsAssistant::read(&mut pkt).unwrap();

        assert!(set_everyone.everyone_is_assistant);
        assert_eq!(set_everyone.party_index, Some(0));
    }

    #[test]
    fn group_new_leader_writes_cpp_party_index_name_bits_string_order() {
        let bytes = GroupNewLeader {
            party_index: 0,
            name: "Player80".to_string(),
        }
        .to_bytes();

        assert_eq!(
            u16::from_le_bytes([bytes[0], bytes[1]]),
            ServerOpcodes::GroupNewLeader as u16
        );
        let mut payload = WorldPacket::from_bytes(&bytes[2..]);
        assert_eq!(payload.read_int8().unwrap(), 0);
        assert_eq!(payload.read_bits(9).unwrap(), 8);
        assert_eq!(payload.read_string(8).unwrap(), "Player80");
    }

    #[test]
    fn set_everyone_is_assistant_reads_cpp_optional_none_bit_before_apply() {
        let mut pkt = WorldPacket::new_empty();
        pkt.write_bit(false);
        pkt.write_bit(false);
        pkt.flush_bits();
        pkt.reset_read();

        let set_everyone = SetEveryoneIsAssistant::read(&mut pkt).unwrap();

        assert!(!set_everyone.everyone_is_assistant);
        assert_eq!(set_everyone.party_index, None);
    }

    #[test]
    fn set_party_assignment_reads_cpp_has_party_set_assignment_guid_party_index_order() {
        let target = ObjectGuid::create_player(1, 77);
        let mut pkt = WorldPacket::new_empty();
        pkt.write_bit(true);
        pkt.write_bit(true);
        pkt.write_uint8(1);
        pkt.write_packed_guid(&target);
        pkt.write_uint8(0);
        pkt.reset_read();

        let assignment = SetPartyAssignment::read(&mut pkt).unwrap();

        assert_eq!(assignment.assignment, 1);
        assert_eq!(assignment.target, target);
        assert!(assignment.apply);
        assert_eq!(assignment.party_index, Some(0));
    }

    #[test]
    fn set_party_assignment_reads_cpp_optional_none_bit_before_set() {
        let target = ObjectGuid::create_player(1, 78);
        let mut pkt = WorldPacket::new_empty();
        pkt.write_bit(false);
        pkt.write_bit(false);
        pkt.write_uint8(0);
        pkt.write_packed_guid(&target);
        pkt.flush_bits();
        pkt.reset_read();

        let assignment = SetPartyAssignment::read(&mut pkt).unwrap();

        assert_eq!(assignment.assignment, 0);
        assert_eq!(assignment.target, target);
        assert!(!assignment.apply);
        assert_eq!(assignment.party_index, None);
    }

    #[test]
    fn set_role_reads_cpp_has_party_guid_role_party_index_order() {
        let target = ObjectGuid::create_player(1, 77);
        let mut pkt = WorldPacket::new_empty();
        pkt.write_bit(true);
        pkt.write_packed_guid(&target);
        pkt.write_uint8(4);
        pkt.write_uint8(0);
        pkt.reset_read();

        let set_role = SetRole::read(&mut pkt).unwrap();

        assert_eq!(set_role.target_guid, target);
        assert_eq!(set_role.role, 4);
        assert_eq!(set_role.party_index, Some(0));
    }

    #[test]
    fn set_role_reads_cpp_optional_none_before_guid_role() {
        let target = ObjectGuid::create_player(1, 78);
        let mut pkt = WorldPacket::new_empty();
        pkt.write_bit(false);
        pkt.write_packed_guid(&target);
        pkt.write_uint8(2);
        pkt.flush_bits();
        pkt.reset_read();

        let set_role = SetRole::read(&mut pkt).unwrap();

        assert_eq!(set_role.target_guid, target);
        assert_eq!(set_role.role, 2);
        assert_eq!(set_role.party_index, None);
    }

    #[test]
    fn role_changed_inform_writes_cpp_party_from_changed_old_new_order() {
        let from = ObjectGuid::create_player(1, 42);
        let changed = ObjectGuid::create_player(1, 43);
        let mut pkt = WorldPacket::new_empty();
        RoleChangedInform {
            party_index: 0,
            from,
            changed_unit: changed,
            old_role: 1,
            new_role: 4,
        }
        .write(&mut pkt);
        pkt.reset_read();

        assert_eq!(pkt.read_uint8().unwrap(), 0);
        assert_eq!(pkt.read_packed_guid().unwrap(), from);
        assert_eq!(pkt.read_packed_guid().unwrap(), changed);
        assert_eq!(pkt.read_uint8().unwrap(), 1);
        assert_eq!(pkt.read_uint8().unwrap(), 4);
    }

    #[test]
    fn role_poll_reads_cpp_optional_party_index() {
        let mut pkt = WorldPacket::new_empty();
        pkt.write_bit(true);
        pkt.write_uint8(0);
        pkt.reset_read();

        let role_poll = InitiateRolePoll::read(&mut pkt).unwrap();

        assert_eq!(role_poll.party_index, Some(0));
    }

    #[test]
    fn role_poll_reads_cpp_absent_party_index() {
        let mut pkt = WorldPacket::new_empty();
        pkt.write_bit(false);
        pkt.flush_bits();
        pkt.reset_read();

        let role_poll = InitiateRolePoll::read(&mut pkt).unwrap();

        assert_eq!(role_poll.party_index, None);
    }

    #[test]
    fn role_poll_inform_writes_cpp_party_index_then_from() {
        let from = ObjectGuid::create_player(1, 42);
        let mut pkt = WorldPacket::new_empty();
        RolePollInform {
            party_index: 0,
            from,
        }
        .write(&mut pkt);
        pkt.reset_read();

        assert_eq!(pkt.read_int8().unwrap(), 0);
        assert_eq!(pkt.read_packed_guid().unwrap(), from);
    }

    #[test]
    fn update_raid_target_reads_cpp_bit_target_symbol_party_index_order() {
        let target = ObjectGuid::create_player(1, 77);
        let mut pkt = WorldPacket::new_empty();
        pkt.write_bit(true);
        pkt.write_packed_guid(&target);
        pkt.write_int8(3);
        pkt.write_uint8(0);
        pkt.reset_read();

        let update = UpdateRaidTarget::read(&mut pkt).unwrap();

        assert_eq!(update.party_index, Some(0));
        assert_eq!(update.target, target);
        assert_eq!(update.symbol, 3);
    }

    #[test]
    fn update_raid_target_reads_cpp_symbol_minus_one_request() {
        let mut pkt = WorldPacket::new_empty();
        pkt.write_bit(false);
        pkt.write_packed_guid(&ObjectGuid::EMPTY);
        pkt.write_int8(-1);
        pkt.flush_bits();
        pkt.reset_read();

        let update = UpdateRaidTarget::read(&mut pkt).unwrap();

        assert_eq!(update.party_index, None);
        assert_eq!(update.target, ObjectGuid::EMPTY);
        assert_eq!(update.symbol, -1);
    }

    #[test]
    fn request_party_join_updates_reads_cpp_optional_party_index() {
        let mut pkt = WorldPacket::new_empty();
        pkt.write_bit(true);
        pkt.write_uint8(0);
        pkt.reset_read();

        let request = RequestPartyJoinUpdates::read(&mut pkt).unwrap();

        assert_eq!(request.party_index, Some(0));
    }

    #[test]
    fn request_party_member_stats_reads_cpp_bit_guid_without_party_index() {
        let target = ObjectGuid::create_player(1, 77);
        let mut pkt = WorldPacket::new_empty();
        pkt.write_bit(false);
        pkt.write_packed_guid(&target);
        pkt.flush_bits();
        pkt.reset_read();

        let request = RequestPartyMemberStats::read(&mut pkt).unwrap();

        assert_eq!(request.target_guid, target);
        assert_eq!(request.party_index, None);
    }

    #[test]
    fn request_party_member_stats_reads_cpp_bit_guid_then_party_index() {
        let target = ObjectGuid::create_player(1, 78);
        let mut pkt = WorldPacket::new_empty();
        pkt.write_bit(true);
        pkt.write_packed_guid(&target);
        pkt.write_uint8(1);
        pkt.reset_read();

        let request = RequestPartyMemberStats::read(&mut pkt).unwrap();

        assert_eq!(request.target_guid, target);
        assert_eq!(request.party_index, Some(1));
    }

    #[test]
    fn raid_target_update_single_writes_cpp_party_symbol_target_changed_by_order() {
        let target = ObjectGuid::create_player(1, 77);
        let changed_by = ObjectGuid::create_player(1, 42);
        let target_bytes = packed_guid_bytes(target);
        let changed_by_bytes = packed_guid_bytes(changed_by);
        let mut pkt = WorldPacket::new_empty();
        SendRaidTargetUpdateSingle {
            party_index: 0,
            target,
            changed_by,
            symbol: 3,
        }
        .write(&mut pkt);
        let data = pkt.into_data();

        assert_eq!(data[0], 0);
        assert_eq!(data[1], 3);
        assert_eq!(&data[2..2 + target_bytes.len()], target_bytes.as_slice());
        assert_eq!(&data[2 + target_bytes.len()..], changed_by_bytes.as_slice());
    }

    #[test]
    fn raid_target_update_all_writes_cpp_party_count_all_icons_order() {
        let first = ObjectGuid::create_player(1, 77);
        let icons: Vec<_> = (0..8)
            .map(|symbol| {
                (
                    symbol,
                    if symbol == 0 {
                        first
                    } else {
                        ObjectGuid::EMPTY
                    },
                )
            })
            .collect();
        let mut pkt = WorldPacket::new_empty();
        SendRaidTargetUpdateAll {
            party_index: 0,
            target_icons: icons,
        }
        .write(&mut pkt);
        pkt.reset_read();

        assert_eq!(pkt.read_uint8().unwrap(), 0);
        assert_eq!(pkt.read_uint32().unwrap(), 8);
        assert_eq!(pkt.read_packed_guid().unwrap(), first);
        assert_eq!(pkt.read_uint8().unwrap(), 0);
        for symbol in 1..8 {
            assert_eq!(pkt.read_packed_guid().unwrap(), ObjectGuid::EMPTY);
            assert_eq!(pkt.read_uint8().unwrap(), symbol);
        }
    }

    #[test]
    fn raid_markers_changed_writes_empty_represented_join_update_shape() {
        let mut pkt = WorldPacket::new_empty();
        RaidMarkersChanged {
            party_index: 0,
            active_markers: 0,
            raid_markers: Vec::new(),
        }
        .write(&mut pkt);
        let data = pkt.into_data();

        assert_eq!(data, vec![0, 0, 0, 0, 0, 0]);
    }

    #[test]
    fn raid_markers_changed_writes_marker_entries_like_cpp() {
        let transport =
            ObjectGuid::create_transport(wow_core::guid::HighGuid::Transport, 0x0102_0304);
        let mut pkt = WorldPacket::new_empty();
        RaidMarkersChanged {
            party_index: 1,
            active_markers: 1 << 3,
            raid_markers: vec![RaidMarker {
                transport_guid: transport,
                map_id: 571,
                position: Position::xyz(12.25, -34.5, 6.75),
            }],
        }
        .write(&mut pkt);
        pkt.reset_read();

        assert_eq!(pkt.read_uint8().unwrap(), 1);
        assert_eq!(pkt.read_uint32().unwrap(), 1 << 3);
        assert_eq!(pkt.read_bits(4).unwrap(), 1);
        pkt.flush_bits();
        assert_eq!(pkt.read_packed_guid().unwrap(), transport);
        assert_eq!(pkt.read_uint32().unwrap(), 571);
        assert_eq!(pkt.read_float().unwrap(), 12.25);
        assert_eq!(pkt.read_float().unwrap(), -34.5);
        assert_eq!(pkt.read_float().unwrap(), 6.75);
        assert!(pkt.is_empty());
    }

    #[test]
    fn ready_check_do_reads_cpp_has_party_index_then_optional_byte() {
        let mut pkt = WorldPacket::new_empty();
        pkt.write_bit(true);
        pkt.write_uint8(0);
        pkt.reset_read();

        let ready_check = DoReadyCheck::read(&mut pkt).unwrap();

        assert_eq!(ready_check.party_index, Some(0));
    }

    #[test]
    fn ready_check_do_reads_cpp_absent_party_index() {
        let mut pkt = WorldPacket::new_empty();
        pkt.write_bit(false);
        pkt.flush_bits();
        pkt.reset_read();

        let ready_check = DoReadyCheck::read(&mut pkt).unwrap();

        assert_eq!(ready_check.party_index, None);
    }

    #[test]
    fn ready_check_response_reads_cpp_ready_bit_then_has_party_index() {
        let mut pkt = WorldPacket::new_empty();
        pkt.write_bit(true);
        pkt.write_bit(true);
        pkt.write_uint8(0);
        pkt.reset_read();

        let response = ReadyCheckResponseClient::read(&mut pkt).unwrap();

        assert!(response.is_ready);
        assert_eq!(response.party_index, Some(0));
    }

    #[test]
    fn ready_check_response_reads_cpp_absent_party_index_after_ready_bit() {
        let mut pkt = WorldPacket::new_empty();
        pkt.write_bit(false);
        pkt.write_bit(false);
        pkt.flush_bits();
        pkt.reset_read();

        let response = ReadyCheckResponseClient::read(&mut pkt).unwrap();

        assert!(!response.is_ready);
        assert_eq!(response.party_index, None);
    }

    #[test]
    fn ready_check_started_writes_cpp_party_guid_initiator_duration_order() {
        let initiator = ObjectGuid::create_player(1, 42);
        let mut pkt = WorldPacket::new_empty();
        ReadyCheckStarted {
            party_index: 0,
            party_guid: 77,
            initiator_guid: initiator,
            duration_ms: 35_000,
        }
        .write(&mut pkt);
        pkt.reset_read();

        assert_eq!(pkt.read_uint8().unwrap(), 0);
        assert_eq!(
            pkt.read_packed_guid().unwrap(),
            ObjectGuid::create_group(77)
        );
        assert_eq!(pkt.read_packed_guid().unwrap(), initiator);
        assert_eq!(pkt.read_int64().unwrap(), 35_000);
    }

    #[test]
    fn ready_check_response_writes_cpp_guids_bit_flush_order() {
        let player = ObjectGuid::create_player(1, 43);
        let mut pkt = WorldPacket::new_empty();
        ReadyCheckResponse {
            party_guid: 78,
            player,
            is_ready: true,
        }
        .write(&mut pkt);
        pkt.reset_read();

        assert_eq!(
            pkt.read_packed_guid().unwrap(),
            ObjectGuid::create_group(78)
        );
        assert_eq!(pkt.read_packed_guid().unwrap(), player);
        assert!(pkt.read_bit().unwrap());
    }

    #[test]
    fn ready_check_completed_writes_cpp_party_index_then_guid() {
        let mut pkt = WorldPacket::new_empty();
        ReadyCheckCompleted {
            party_index: 0,
            party_guid: 79,
        }
        .write(&mut pkt);
        pkt.reset_read();

        assert_eq!(pkt.read_uint8().unwrap(), 0);
        assert_eq!(
            pkt.read_packed_guid().unwrap(),
            ObjectGuid::create_group(79)
        );
    }

    #[test]
    fn swap_subgroups_reads_cpp_bit_first_guid_second_guid_party_index_order() {
        let first = ObjectGuid::create_player(1, 77);
        let second = ObjectGuid::create_player(1, 78);
        let mut pkt = WorldPacket::new_empty();
        pkt.write_bit(true);
        pkt.write_packed_guid(&first);
        pkt.write_packed_guid(&second);
        pkt.write_uint8(0);
        pkt.reset_read();

        let swap = SwapSubGroups::read(&mut pkt).unwrap();

        assert_eq!(swap.first_target, first);
        assert_eq!(swap.second_target, second);
        assert_eq!(swap.party_index, Some(0));
    }

    #[test]
    fn swap_subgroups_reads_cpp_optional_none_bit_before_guids() {
        let first = ObjectGuid::create_player(1, 79);
        let second = ObjectGuid::create_player(1, 80);
        let mut pkt = WorldPacket::new_empty();
        pkt.write_bit(false);
        pkt.write_packed_guid(&first);
        pkt.write_packed_guid(&second);
        pkt.flush_bits();
        pkt.reset_read();

        let swap = SwapSubGroups::read(&mut pkt).unwrap();

        assert_eq!(swap.first_target, first);
        assert_eq!(swap.second_target, second);
        assert_eq!(swap.party_index, None);
    }

    #[test]
    fn change_subgroup_reads_cpp_optional_none_bit() {
        let target = ObjectGuid::create_player(1, 78);
        let mut pkt = WorldPacket::new_empty();
        pkt.write_packed_guid(&target);
        pkt.write_uint8(2);
        pkt.write_bit(false);
        pkt.flush_bits();
        pkt.reset_read();

        let change = ChangeSubGroup::read(&mut pkt).unwrap();

        assert_eq!(change.target_guid, target);
        assert_eq!(change.new_subgroup, 2);
        assert_eq!(change.party_index, None);
    }

    #[test]
    fn party_member_phase_states_writes_cpp_order() {
        let states = PartyMemberPhaseStates {
            phase_shift_flags: 0x08,
            personal_guid: ObjectGuid::EMPTY,
            phases: vec![PartyMemberPhase {
                flags: 0x02,
                id: 20,
            }],
        };
        let mut pkt = WorldPacket::new_empty();

        states.write(&mut pkt);

        assert_eq!(
            pkt.into_data(),
            vec![
                0x08, 0x00, 0x00, 0x00, // PhaseShiftFlags
                0x01, 0x00, 0x00, 0x00, // List.Count
                0x00, 0x00, // PersonalGUID packed mask + empty payload
                0x02, 0x00, 0x00, 0x00, // phase.Flags
                0x14, 0x00, // phase.Id
            ]
        );
    }

    #[test]
    fn low_level_raid1_accepts_empty_payload_like_cpp() {
        let mut pkt = WorldPacket::new_empty();

        let parsed = LowLevelRaid1::read(&mut pkt).unwrap();

        // Empty struct — no fields. C++ Read() is empty body.
        let _ = parsed;
    }

    #[test]
    fn low_level_raid1_opcode_matches_cpp() {
        assert_eq!(LowLevelRaid1::OPCODE as u16, 0x36A1);
    }

    #[test]
    fn low_level_raid2_accepts_empty_payload_like_cpp() {
        let mut pkt = WorldPacket::new_empty();

        let parsed = LowLevelRaid2::read(&mut pkt).unwrap();

        // Empty struct — no fields. C++ Read() is empty body.
        let _ = parsed;
    }

    #[test]
    fn low_level_raid2_opcode_matches_cpp() {
        assert_eq!(LowLevelRaid2::OPCODE as u16, 0x3512);
    }

    #[test]
    fn minimap_ping_client_reads_bit_xy_optional_party_index_like_cpp() {
        // With party index
        let mut pkt = WorldPacket::new_empty();
        pkt.write_bit(true);
        pkt.write_float(123.456);
        pkt.write_float(-789.012);
        pkt.write_uint8(3);
        pkt.flush_bits();
        pkt.reset_read();

        let ping = MinimapPingClient::read(&mut pkt).unwrap();
        assert_eq!(ping.position_x, 123.456);
        assert_eq!(ping.position_y, -789.012);
        assert_eq!(ping.party_index, Some(3));
    }

    #[test]
    fn minimap_ping_client_reads_bit_xy_no_party_index_like_cpp() {
        // Without party index
        let mut pkt = WorldPacket::new_empty();
        pkt.write_bit(false);
        pkt.write_float(42.0);
        pkt.write_float(99.5);
        pkt.flush_bits();
        pkt.reset_read();

        let ping = MinimapPingClient::read(&mut pkt).unwrap();
        assert_eq!(ping.position_x, 42.0);
        assert_eq!(ping.position_y, 99.5);
        assert!(ping.party_index.is_none());
    }

    #[test]
    fn minimap_ping_client_opcode_matches_cpp() {
        assert_eq!(MinimapPingClient::OPCODE as u16, 0x364E);
    }

    #[test]
    fn party_uninvite_reads_bits_guid_party_index_and_reason_like_cpp() {
        let target = ObjectGuid::create_player(1, 0x1234);
        let mut pkt = WorldPacket::new_empty();
        pkt.write_bit(true);
        pkt.write_bits(3, 8);
        pkt.write_guid(&target);
        pkt.write_uint8(0);
        pkt.write_string("bye");
        pkt.flush_bits();
        pkt.reset_read();

        let parsed = PartyUninvite::read(&mut pkt).unwrap();

        assert_eq!(parsed.target_guid, target);
        assert_eq!(parsed.party_index, Some(0));
        assert_eq!(parsed.reason, "bye");
    }

    #[test]
    fn minimap_ping_server_opcode_and_payload_order_like_cpp() {
        use super::MinimapPing;
        let sender = ObjectGuid::create_player(1, 42);
        let pkt = MinimapPing {
            sender,
            position_x: 111.222,
            position_y: 333.444,
        }
        .to_bytes();

        assert!(!pkt.is_empty());
        let mut reader = WorldPacket::from_bytes(&pkt);
        assert_eq!(
            reader.read_uint16().unwrap(),
            ServerOpcodes::MinimapPing as u16
        );
        let read_guid = reader.read_packed_guid().unwrap();
        assert_eq!(read_guid, sender);
        assert_eq!(reader.read_float().unwrap(), 111.222);
        assert_eq!(reader.read_float().unwrap(), 333.444);
    }

    #[test]
    fn silence_party_talker_reads_guid_and_silent_bit_like_cpp() {
        let target = ObjectGuid::create_player(1, 0x55aa);
        let mut pkt = WorldPacket::new_empty();
        pkt.write_bytes(&target.to_raw_bytes());
        pkt.write_bit(true);
        pkt.flush_bits();
        pkt.reset_read();

        let parsed = SilencePartyTalker::read(&mut pkt).unwrap();

        assert_eq!(parsed.target, target);
        assert!(parsed.silent);
    }

    #[test]
    fn silence_party_talker_opcode_matches_cpp() {
        assert_eq!(SilencePartyTalker::OPCODE as u16, 0x3655);
    }
}
