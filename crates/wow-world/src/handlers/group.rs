// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Handlers for Group/Party opcodes: PartyInvite, PartyInviteResponse, LeaveGroup.

use crate::session::directory::{PlayerRegistration, PlayerRegistry};
use crate::session::mailbox::{
    ApplyGroupJoinLikeCppCommand, ApplyGroupRemovalLikeCppCommand, SendPartyUpdateLikeCppCommand,
    SendRealmPacketLikeCppCommand, SessionCommand,
};
use std::time::Duration;
#[cfg(test)]
use tracing::warn;
use wow_constants::ClientOpcodes;
use wow_core::{ObjectGuid, guid::HighGuid};
use wow_handler::{PacketProcessing, SessionStatus};
use wow_persistence::{
    RepresentedGroupDifficultyKindLikeCpp, RepresentedGroupPersistenceCommandLikeCpp,
    RepresentedGroupPersistenceModeLikeCpp, RepresentedGroupPersistenceOutcomeLikeCpp,
    RepresentedGroupPersistenceRequestLikeCpp, SocialPartyInviteLookupOutcomeLikeCpp,
    SocialPersistencePortLikeCpp,
};

use crate::session::registry::PacketHandlerEntry;
#[cfg(test)]
use wow_packet::ClientPacket;
use wow_packet::packets::misc::{RandomRoll, RandomRollClient};
#[cfg(test)]
use wow_packet::packets::party::ClearRaidMarker;
use wow_packet::packets::party::{
    DoReadyCheck, GroupDecline, GroupNewLeader, GroupUninvite, InitiateRolePoll, LowLevelRaid1,
    LowLevelRaid2, MinimapPing, MinimapPingClient, OptOutOfLoot, PartyCommandResult,
    PartyDifficultySettings, PartyInviteServer, PartyLootSettings, PartyMemberFullState,
    PartyPlayerInfo, PartyUpdate, RaidMarker, RaidMarkersChanged, ReadyCheckCompleted,
    ReadyCheckResponse, ReadyCheckResponseClient, ReadyCheckStarted, RequestPartyJoinUpdates,
    RequestPartyMemberStats, RoleChangedInform, RolePollInform, SendRaidTargetUpdateAll,
    SendRaidTargetUpdateSingle, SetAssistantLeader, SetEveryoneIsAssistant, SetLootMethod,
    SetPartyAssignment, SetPartyLeader, SetRole, SilencePartyTalker, UpdateRaidTarget,
    party_result,
};
use wow_social::group::GROUP_CATEGORY_HOME_LIKE_CPP;
use wow_social::group::{
    AcceptGroupInviteResultLikeCpp, CreateGroupInviteResultLikeCpp, GroupAuthorityErrorLikeCpp,
    GroupInfo, GroupMemberRemovalKindLikeCpp, GroupPersistenceIntentLikeCpp, GroupRegistry,
    MEMBER_FLAG_ASSISTANT_LIKE_CPP, ReadyCheckEventLikeCpp,
};

use crate::session::{GroupInvitePolicyLikeCpp, player_team_for_race_cpp};

mod ops_1;
#[cfg(test)]
#[path = "../../unit_tests/handlers/group/ops_2/test_only.rs"]
mod ops_2;
mod state;
#[cfg(test)]
mod test_shims;
#[allow(unused_imports)]
pub use ops_1::*;
#[allow(unused_imports)]
pub use state::*;

// ── canonical group lookup ────────────────────────────────────────────────────

// ── inventory registrations ───────────────────────────────────────────────────

// ── Helpers ───────────────────────────────────────────────────────────────────

// ── Handler implementations ───────────────────────────────────────────────────

#[cfg(test)]
#[path = "../../unit_tests/handlers/group_tests.rs"]
mod tests;
