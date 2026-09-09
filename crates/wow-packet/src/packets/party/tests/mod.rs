//! Party packets regression scenarios.
//!
//! Separated from the party.rs root under #650.

use super::{
    ChangeSubGroup, ClearRaidMarker, ConvertRaid, DoReadyCheck, DungeonScoreMapSummary,
    DungeonScoreSummary, GroupNewLeader, InitiateRolePoll, LowLevelRaid1, LowLevelRaid2,
    MinimapPing, MinimapPingClient, OptOutOfLoot, PartyMemberFullState, PartyMemberPhase,
    PartyMemberPhaseStates, PartyUninvite, RaidMarker, RaidMarkersChanged, ReadyCheckCompleted,
    ReadyCheckResponse, ReadyCheckResponseClient, ReadyCheckStarted, RequestPartyJoinUpdates,
    RequestPartyMemberStats, RoleChangedInform, RolePollInform, SendRaidTargetUpdateAll,
    SendRaidTargetUpdateSingle, SetAssistantLeader, SetEveryoneIsAssistant, SetLootMethod,
    SetPartyAssignment, SetPartyLeader, SetRole, SilencePartyTalker, SwapSubGroups,
    UpdateRaidTarget, party_result,
};
use crate::{ClientPacket, PacketError, ServerPacket, WorldPacket};
use wow_constants::ServerOpcodes;
use wow_core::{ObjectGuid, Position};

fn packed_guid_bytes(guid: ObjectGuid) -> Vec<u8> {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_packed_guid(&guid);
    pkt.into_data()
}

mod scenarios_1;
mod scenarios_2;
