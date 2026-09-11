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
use rand::Rng;
use std::time::Duration;
use tracing::{info, warn};
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
use wow_packet::packets::misc::{RandomRoll, RandomRollClient};
use wow_packet::packets::party::{
    ClearRaidMarker, DoReadyCheck, GroupDecline, GroupNewLeader, GroupUninvite, InitiateRolePoll,
    LowLevelRaid1, LowLevelRaid2, MinimapPing, MinimapPingClient, OptOutOfLoot, PartyCommandResult,
    PartyDifficultySettings, PartyInviteServer, PartyLootSettings, PartyMemberFullState,
    PartyPlayerInfo, PartyUpdate, RaidMarker, RaidMarkersChanged, ReadyCheckCompleted,
    ReadyCheckResponse, ReadyCheckResponseClient, ReadyCheckStarted, RequestPartyJoinUpdates,
    RequestPartyMemberStats, RoleChangedInform, RolePollInform, SendRaidTargetUpdateAll,
    SendRaidTargetUpdateSingle, SetAssistantLeader, SetEveryoneIsAssistant, SetLootMethod,
    SetPartyAssignment, SetPartyLeader, SetRole, SilencePartyTalker, UpdateRaidTarget,
    party_result,
};
use wow_packet::{ClientPacket, ServerPacket};
use wow_social::group::GROUP_CATEGORY_HOME_LIKE_CPP;
use wow_social::group::{
    AcceptGroupInviteResultLikeCpp, CreateGroupInviteResultLikeCpp, GroupAuthorityErrorLikeCpp,
    GroupInfo, GroupMemberRemovalKindLikeCpp, GroupPersistenceIntentLikeCpp, GroupRegistry,
    MEMBER_FLAG_ASSISTANT_LIKE_CPP, ReadyCheckEventLikeCpp,
};

use crate::session::{GroupInvitePolicyLikeCpp, WorldSession, player_team_for_race_cpp};

mod commands;
mod ops_1;
mod ops_2;
mod ops_3;
mod state;
#[allow(unused_imports)]
pub use ops_1::*;
#[allow(unused_imports)]
pub use ops_2::*;
#[allow(unused_imports)]
pub use ops_3::*;
#[allow(unused_imports)]
pub use state::*;

// ── canonical group lookup ────────────────────────────────────────────────────

// ── inventory registrations ───────────────────────────────────────────────────

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::PartyInvite,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_party_invite",
        handler: |session, catalogs, pkt| Box::pin(async move {
            session
                .handle_party_invite_with_policy_like_cpp(
                    pkt,
                    catalogs.group_invite_policy.as_ref(),
                )
                .await
        }),
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::PartyInviteResponse,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_party_invite_response",
        handler: |session, _catalogs, pkt| {
            Box::pin(async move { session.handle_party_invite_response(pkt).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::PartyUninvite,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_party_uninvite",
        handler: |session, _catalogs, pkt| Box::pin(async move { session.handle_party_uninvite(pkt).await }),
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::LeaveGroup,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_leave_group",
        handler: |session, _catalogs, pkt| Box::pin(async move { session.handle_leave_group(pkt).await }),
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::ConvertRaid,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_convert_raid",
        handler: |session, _catalogs, pkt| Box::pin(async move { session.handle_convert_raid(pkt).await }),
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::ChangeSubGroup,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_change_sub_group",
        handler: |session, _catalogs, pkt| Box::pin(async move { session.handle_change_sub_group(pkt).await }),
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::SwapSubGroups,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_swap_sub_groups",
        handler: |session, _catalogs, pkt| Box::pin(async move { session.handle_swap_sub_groups(pkt).await }),
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::SetLootMethod,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_set_loot_method",
        handler: |session, _catalogs, pkt| Box::pin(async move { session.handle_set_loot_method(pkt).await }),
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::SetPartyLeader,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_set_party_leader",
        handler: |session, _catalogs, pkt| Box::pin(async move { session.handle_set_party_leader(pkt).await }),
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::SetAssistantLeader,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_set_assistant_leader",
        handler: |session, _catalogs, pkt| {
            Box::pin(async move { session.handle_set_assistant_leader(pkt).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::SetEveryoneIsAssistant,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_set_everyone_is_assistant",
        handler: |session, _catalogs, pkt| {
            Box::pin(async move { session.handle_set_everyone_is_assistant(pkt).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::SilencePartyTalker,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_silence_party_talker",
        handler: |session, _catalogs, pkt| {
            Box::pin(async move { session.handle_silence_party_talker(pkt).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::SetPartyAssignment,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_set_party_assignment",
        handler: |session, _catalogs, pkt| {
            Box::pin(async move { session.handle_set_party_assignment(pkt).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::SetRole,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_set_role",
        handler: |session, _catalogs, pkt| Box::pin(async move { session.handle_set_role(pkt).await }),
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::InitiateRolePoll,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_initiate_role_poll",
        handler: |session, _catalogs, pkt| {
            Box::pin(async move { session.handle_initiate_role_poll(pkt).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::UpdateRaidTarget,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_update_raid_target",
        handler: |session, _catalogs, pkt| {
            Box::pin(async move { session.handle_update_raid_target(pkt).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::RequestPartyJoinUpdates,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_request_party_join_updates",
        handler: |session, _catalogs, pkt| {
            Box::pin(async move { session.handle_request_party_join_updates(pkt).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::RequestPartyMemberStats,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_request_party_member_stats",
        handler: |session, _catalogs, pkt| {
            Box::pin(async move { session.handle_request_party_member_stats(pkt).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::DoReadyCheck,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_do_ready_check",
        handler: |session, _catalogs, pkt| Box::pin(async move { session.handle_do_ready_check(pkt).await }),
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::ReadyCheckResponse,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_ready_check_response",
        handler: |session, _catalogs, pkt| {
            Box::pin(async move { session.handle_ready_check_response(pkt).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::OptOutOfLoot,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_opt_out_of_loot",
        handler: |session, _catalogs, pkt| Box::pin(async move { session.handle_opt_out_of_loot(pkt).await }),
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::LowLevelRaid1,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_low_level_raid1",
        handler: |session, _catalogs, pkt| Box::pin(async move { session.handle_low_level_raid1(pkt).await }),
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::LowLevelRaid2,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_low_level_raid2",
        handler: |session, _catalogs, pkt| Box::pin(async move { session.handle_low_level_raid2(pkt).await }),
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::MinimapPing,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_minimap_ping",
        handler: |session, _catalogs, pkt| Box::pin(async move { session.handle_minimap_ping(pkt).await }),
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::RandomRoll,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_random_roll",
        handler: |session, _catalogs, pkt| Box::pin(async move { session.handle_random_roll(pkt).await }),
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

// ── Handler implementations ───────────────────────────────────────────────────

#[cfg(test)]
#[path = "group_tests.rs"]
mod tests;
