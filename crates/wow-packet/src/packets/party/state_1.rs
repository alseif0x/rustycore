//! Party packets state definitions, part 1 of 2.
//!
//! Separated from the party.rs root under #650. Behaviour is preserved.

use super::*;

/// Sent to the inviting player to confirm or reject the operation.
pub struct PartyCommandResult {
    pub name: String, // target name
    pub command: u8,  // PartyOperation: 0=Invite, 1=Uninvite, 2=Leave, 4=Swap
    pub result: u8,   // PartyResult enum (see below)
    pub result_data: u32,
    pub result_guid: ObjectGuid,
}

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
        let target_guid = pkt.read_packed_guid()?;
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClearRaidMarker {
    pub marker_id: u8,
}

impl ClientPacket for ClearRaidMarker {
    // The inspected TrinityCore 3.4.3 opcode table uses the shared unresolved
    // `0xBADD` placeholder for both `CMSG_CLEAR_RAID_MARKER` and
    // `CMSG_SET_LOOT_SPECIALIZATION`. Rust cannot represent duplicate enum
    // discriminants, so this parser is routed from the existing 0xBADD opcode
    // slot by payload shape in `WorldSession`.
    const OPCODE: ClientOpcodes = ClientOpcodes::SetLootSpecialization;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self {
            marker_id: pkt.read_uint8()?,
        })
    }
}

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

/// Sent to the INVITED player so they see the invite dialog.
pub struct PartyInviteServer {
    pub can_accept: bool,
    pub proposed_roles: u8,
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
        w.write_bit(false); // AllowMultipleRoles
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
        w.write_uint8(self.proposed_roles);
        w.write_int32(0); // LfgSlots.Count
        w.write_int32(0); // LfgCompletedMask
        w.write_bytes(name_bytes);
        // (no LfgSlots)
    }
}

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

pub struct GroupUninvite;
impl ServerPacket for GroupUninvite {
    const OPCODE: ServerOpcodes = ServerOpcodes::GroupUninvite;
    fn write(&self, _w: &mut WorldPacket) {}
}

pub struct GroupDestroyed;
impl ServerPacket for GroupDestroyed {
    const OPCODE: ServerOpcodes = ServerOpcodes::GroupDestroyed;
    fn write(&self, _w: &mut WorldPacket) {}
}

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
    pub(super) fn write(&self, w: &mut WorldPacket) {
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
