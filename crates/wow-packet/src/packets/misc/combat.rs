// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Combat, death and PvP packets.

use super::*;

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
