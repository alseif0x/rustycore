//! Party / Group packets (WoTLK 3.4.3).
//! C# reference: Source/Game/Networking/Packets/PartyPackets.cs

use crate::{ServerPacket, WorldPacket};
use wow_constants::ServerOpcodes;
use wow_core::ObjectGuid;

// ── PartyCommandResult (SMSG_PARTY_COMMAND_RESULT 0x2796) ────────────────────

/// Sent to the inviting player to confirm or reject the operation.
pub struct PartyCommandResult {
    pub name: String,     // target name
    pub command: u8,      // 0=Invite, 1=Leave, 2=OfflineLeave, 4=Uninvite
    pub result: u8,       // PartyResult enum (see below)
    pub result_data: u32,
    pub result_guid: ObjectGuid,
}

/// PartyResult enum values (result field above)
pub mod party_result {
    pub const OK: u8               = 0;
    pub const BAD_PLAYER_NAME: u8  = 1;
    pub const WRONG_FACTION: u8    = 7;
    pub const ALREADY_IN_GROUP: u8 = 8;
    pub const NOT_LEADER: u8       = 14;
    pub const GROUP_FULL: u8       = 3;
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
        w.write_bit(true);  // AllowMultipleRoles
        w.write_bit(false); // QuestSessionActive
        w.write_bits(name_bytes.len() as u32, 6);
        // VirtualRealmInfo.Write():
        w.write_uint32(self.virtual_realm_address); // RealmAddress
        // VirtualRealmNameInfo.Write():
        w.write_bit(true);  // IsLocal = true
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
        w.write_uint8(0);  // ProposedRoles
        w.write_int32(0);  // LfgSlots.Count
        w.write_int32(0);  // LfgCompletedMask
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

#[derive(Clone)]
pub struct PartyPlayerInfo {
    pub guid: ObjectGuid,
    pub name: String,
    pub class: u8,
    pub subgroup: u8,
    pub flags: u8,         // GroupMemberFlags
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

pub struct PartyUpdate {
    pub party_flags: u16,    // 0 = normal
    pub party_index: u8,     // 0
    pub party_type: u8,      // 1 = Normal group
    pub my_index: i32,       // index of the receiving player in PlayerList
    pub party_guid: u64,     // group GUID
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

pub struct PartyMemberFullState {
    pub member_guid: ObjectGuid,
    pub for_enemy: bool,
    // Stats
    pub status: u16,   // GroupMemberOnlineStatus: 0x0001=online
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
}

impl ServerPacket for PartyMemberFullState {
    const OPCODE: ServerOpcodes = ServerOpcodes::PartyMemberFullState;
    fn write(&self, w: &mut WorldPacket) {
        w.write_bit(self.for_enemy);
        w.flush_bits();

        // PartyMemberStats.Write():
        w.write_uint8(0); // PartyType[0]
        w.write_uint8(0); // PartyType[1]
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
        w.write_int32(0); // VehicleSeat
        w.write_int32(0); // Auras.Count

        // PartyMemberPhaseStates.Write() — empty:
        w.write_int32(0); // PhaseShiftFlags
        w.write_int32(0); // List.Count
        w.write_packed_guid(&ObjectGuid::EMPTY); // PersonalGUID

        // CTROptions.Write() — empty:
        w.write_uint32(0); // ContentTuningConditionMask
        w.write_int32(0);  // Unused901
        w.write_uint32(0); // ExpansionLevelMask

        // (no Auras)
        w.write_bit(false); // PetStats != null → false
        w.flush_bits();

        // DungeonScoreSummary.Write() — empty:
        w.write_float(0.0); // OverallScoreCurrentSeason
        w.write_float(0.0); // LadderScoreCurrentSeason
        w.write_int32(0);   // Runs.Count

        // (no PetStats)
        w.write_packed_guid(&self.member_guid);
    }
}
