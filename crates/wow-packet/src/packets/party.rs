//! Party / Group packets (WoTLK 3.4.3).
//! C# reference: Source/Game/Networking/Packets/PartyPackets.cs

use crate::{ClientPacket, ServerPacket, WorldPacket};
use wow_constants::{ClientOpcodes, ServerOpcodes};
use wow_core::{ObjectGuid, Position};

use crate::world_packet::PacketError;

// ── PartyCommandResult (SMSG_PARTY_COMMAND_RESULT 0x2796) ────────────────────

mod state_1;
mod state_2;
#[allow(unused_imports)]
pub use state_1::*;
#[allow(unused_imports)]
pub use state_2::*;

#[cfg(test)]
#[path = "party/tests/mod.rs"]
mod tests;

/// PartyResult enum values (result field above)
pub mod party_result {
    pub const OK: u8 = 0;
    pub const BAD_PLAYER_NAME: u8 = 1;
    pub const TARGET_NOT_IN_GROUP: u8 = 2;
    pub const TARGET_NOT_IN_INSTANCE: u8 = 3;
    pub const GROUP_FULL: u8 = 4;
    pub const ALREADY_IN_GROUP: u8 = 5;
    pub const NOT_IN_GROUP: u8 = 6;
    pub const NOT_LEADER: u8 = 7;
    pub const WRONG_FACTION: u8 = 8;
    pub const IGNORING_YOU: u8 = 9;
    pub const INVITE_RESTRICTED: u8 = 13;
    pub const GROUP_SWAP_FAILED: u8 = 14;
    /// C++ `SharedDefines.h`: LFG boot gates from
    /// `Player::CanUninviteFromGroup` (`Player.cpp:25147-25177`).
    pub const PARTY_LFG_BOOT_LIMIT: u8 = 20;
    pub const PARTY_LFG_BOOT_IN_PROGRESS: u8 = 22;
    pub const PARTY_LFG_BOOT_TOO_FEW_PLAYERS: u8 = 23;
    pub const PARTY_LFG_BOOT_IN_COMBAT: u8 = 26;
    pub const PARTY_LFG_BOOT_DUNGEON_COMPLETE: u8 = 28;
    pub const PARTY_LFG_BOOT_LOOT_ROLLS: u8 = 29;
}

// ── ConvertRaid (CMSG_CONVERT_RAID) ─────────────────────────

// ── ChangeSubGroup (CMSG_CHANGE_SUB_GROUP) ─────────────────────────

// ── SetAssistantLeader (CMSG_SET_ASSISTANT_LEADER) ─────────────────────────

// ── PartyUninvite (CMSG_PARTY_UNINVITE) ─────────────────────────

// ── SetEveryoneIsAssistant (CMSG_SET_EVERYONE_IS_ASSISTANT) ─────────────────────────

// ── SilencePartyTalker (CMSG_SILENCE_PARTY_TALKER) ─────────────────────────

// ── SetPartyAssignment (CMSG_SET_PARTY_ASSIGNMENT) ─────────────────────────

// ── Role poll / LFG roles (CMSG_SET_ROLE / CMSG_INITIATE_ROLE_POLL) ───────────

// ── Raid target icons / join updates ──────────────────────────────────────────

// ── RequestPartyMemberStats (CMSG_REQUEST_PARTY_MEMBER_STATS) ────────────────

// ── ReadyCheck (CMSG_DO_READY_CHECK / CMSG_READY_CHECK_RESPONSE) ──────────────

// ── SwapSubGroups (CMSG_SWAP_SUB_GROUPS) ─────────────────────────

// ── SetLootMethod (CMSG_SET_LOOT_METHOD) ─────────────────────────

// ── OptOutOfLoot (CMSG_OPT_OUT_OF_LOOT) ─────────────────────────

// ── MinimapPingClient (CMSG_MINIMAP_PING 0x364E) ────────────────────────

// ── MinimapPing (SMSG_MINIMAP_PING 0x26CE) ──────────────────────────────

// ── LowLevelRaid1 (CMSG_LOW_LEVEL_RAID1 0x36A1) ─────────────────────────

// ── LowLevelRaid2 (CMSG_LOW_LEVEL_RAID2 0x3512) ─────────────────────────

// ── PartyInvite (SMSG_PARTY_INVITE 0x25bd) ────────────────────────────────────

// ── GroupDecline (SMSG_GROUP_DECLINE 0x2791) ─────────────────────────────────

// ── GroupUninvite (SMSG_GROUP_UNINVITE 0x2793) ────────────────────────────────

// ── GroupDestroyed (SMSG_GROUP_DESTROYED 0x2794) ─────────────────────────────

// ── PartyPlayerInfo — member entry in PartyUpdate ────────────────────────────

// ── PartyUpdate (SMSG_PARTY_UPDATE 0x25f4) ───────────────────────────────────

// ── PartyMemberFullState (SMSG_PARTY_MEMBER_FULL_STATE 0x2759) ───────────────
