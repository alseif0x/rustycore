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
use wow_database::{CharStatements, PreparedStatement, StatementDef};
use wow_handler::{PacketProcessing, SessionStatus};
use wow_persistence::{SocialPartyInviteLookupOutcomeLikeCpp, SocialPersistencePortLikeCpp};

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

use crate::session::{WorldSession, player_team_for_race_cpp};

const PARTY_REALM_COMMAND_TIMEOUT_LIKE_CPP: Duration = Duration::from_millis(250);

// ── canonical group lookup ────────────────────────────────────────────────────

/// Canonical represented group lookup matching C++ `Player::GetGroup` semantics.
///
/// C++ anchor: `Player::GetGroup(Optional<uint8> partyIndex)` at
/// `/home/server/woltk-trinity-legacy/src/server/game/Entities/Player/Player.cpp:23429-23444`.
///
/// 1. Validates `cached_group_guid` against canonical `GroupRegistry` membership
///    and represented `PartyIndex`/`GroupCategory`: the cached group must exist,
///    `sender_guid` must be a current member, and the represented category must
///    match when `party_index` is present.
/// 2. If cache is missing, stale, or category-mismatched, scans `GroupRegistry`
///    for a group containing `sender_guid` that also matches `party_index`.
/// 3. Returns `None` when `sender_guid` is not a member of any represented group
///    matching the requested category.
///
/// Boundary: RustyCore currently represents HOME groups only by default.
/// `PartyIndex=Some(1)` / INSTANCE, original-group, BG and BF group ownership do
/// not fall back to HOME and remain unsupported until real state exists.
fn current_group_guid_like_cpp(
    group_reg: &GroupRegistry,
    cached_group_guid: Option<u64>,
    sender_guid: ObjectGuid,
    party_index: Option<u8>,
) -> Option<u64> {
    // 1. Validate cache: group must exist, sender must be a member, and category must match.
    if let Some(gid) = cached_group_guid {
        if let Some(group) = group_reg.get(&gid) {
            if group.members.contains(&sender_guid)
                && group.matches_party_index_like_cpp(party_index)
            {
                return Some(gid);
            }
        }
    }
    // 2. Fallback: scan for any group containing sender in the requested category.
    group_reg
        .snapshots()
        .into_iter()
        .find(|group| {
            group.members.contains(&sender_guid) && group.matches_party_index_like_cpp(party_index)
        })
        .map(|group| group.group_guid)
}

// ── inventory registrations ───────────────────────────────────────────────────

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::PartyInvite,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_party_invite",
        handler: |session, pkt| Box::pin(async move { session.handle_party_invite(pkt).await }),
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::PartyInviteResponse,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_party_invite_response",
        handler: |session, pkt| {
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
        handler: |session, pkt| Box::pin(async move { session.handle_party_uninvite(pkt).await }),
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::LeaveGroup,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_leave_group",
        handler: |session, pkt| Box::pin(async move { session.handle_leave_group(pkt).await }),
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::ConvertRaid,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_convert_raid",
        handler: |session, pkt| Box::pin(async move { session.handle_convert_raid(pkt).await }),
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::ChangeSubGroup,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_change_sub_group",
        handler: |session, pkt| Box::pin(async move { session.handle_change_sub_group(pkt).await }),
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::SwapSubGroups,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_swap_sub_groups",
        handler: |session, pkt| Box::pin(async move { session.handle_swap_sub_groups(pkt).await }),
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::SetLootMethod,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_set_loot_method",
        handler: |session, pkt| Box::pin(async move { session.handle_set_loot_method(pkt).await }),
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::SetPartyLeader,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_set_party_leader",
        handler: |session, pkt| Box::pin(async move { session.handle_set_party_leader(pkt).await }),
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::SetAssistantLeader,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_set_assistant_leader",
        handler: |session, pkt| {
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
        handler: |session, pkt| {
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
        handler: |session, pkt| {
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
        handler: |session, pkt| {
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
        handler: |session, pkt| Box::pin(async move { session.handle_set_role(pkt).await }),
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::InitiateRolePoll,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_initiate_role_poll",
        handler: |session, pkt| {
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
        handler: |session, pkt| {
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
        handler: |session, pkt| {
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
        handler: |session, pkt| {
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
        handler: |session, pkt| Box::pin(async move { session.handle_do_ready_check(pkt).await }),
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::ReadyCheckResponse,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_ready_check_response",
        handler: |session, pkt| {
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
        handler: |session, pkt| Box::pin(async move { session.handle_opt_out_of_loot(pkt).await }),
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::LowLevelRaid1,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_low_level_raid1",
        handler: |session, pkt| Box::pin(async move { session.handle_low_level_raid1(pkt).await }),
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::LowLevelRaid2,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_low_level_raid2",
        handler: |session, pkt| Box::pin(async move { session.handle_low_level_raid2(pkt).await }),
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::MinimapPing,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_minimap_ping",
        handler: |session, pkt| Box::pin(async move { session.handle_minimap_ping(pkt).await }),
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::RandomRoll,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_random_roll",
        handler: |session, pkt| Box::pin(async move { session.handle_random_roll(pkt).await }),
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn party_member_full_state_like_cpp(
    target_guid: ObjectGuid,
    registry: Option<&PlayerRegistry>,
) -> PartyMemberFullState {
    let Some(entry) = registry.and_then(|registry| registry.party_member(target_guid)) else {
        return PartyMemberFullState {
            member_guid: target_guid,
            for_enemy: false,
            status: 0,
            power_type: 0,
            current_health: 0,
            max_health: 0,
            current_power: 0,
            max_power: 0,
            level: 0,
            spec_id: 0,
            zone_id: 0,
            position_x: 0,
            position_y: 0,
            position_z: 0,
            vehicle_seat: 0,
            party_type: [0; 2],
            phases: Default::default(),
            auras: Vec::new(),
            pet_stats: None,
            dungeon_score: Default::default(),
        };
    };

    let pos = entry.position;
    // Represented subset of C++ `PartyMemberFullState::Initialize(Player*)`.
    // Remaining unsupported runtime-owned fields stay explicit instead of
    // being guessed here.
    let mut status = 1u16; // MEMBER_STATUS_ONLINE
    if entry.is_pvp {
        status |= 0x0002; // MEMBER_STATUS_PVP
    }
    if !entry.is_alive {
        if entry.is_ghost {
            status |= 0x0008; // MEMBER_STATUS_GHOST
        } else {
            status |= 0x0004; // MEMBER_STATUS_DEAD
        }
    }
    if entry.is_ffa_pvp {
        status |= 0x0010; // MEMBER_STATUS_PVP_FFA
    }
    if entry.is_afk {
        status |= 0x0040; // MEMBER_STATUS_AFK
    }
    if entry.is_dnd {
        status |= 0x0080; // MEMBER_STATUS_DND
    }
    if entry.in_vehicle {
        status |= 0x0200; // MEMBER_STATUS_VEHICLE
    }

    PartyMemberFullState {
        member_guid: target_guid,
        for_enemy: false,
        status,
        power_type: entry.power_type,
        current_health: i32::try_from(entry.current_health).unwrap_or(i32::MAX),
        max_health: i32::try_from(entry.max_health).unwrap_or(i32::MAX),
        current_power: entry.current_power,
        max_power: entry.max_power,
        level: entry.level as u16,
        spec_id: entry.spec_id.min(u32::from(u16::MAX)) as u16,
        zone_id: entry.zone_id.min(u32::from(u16::MAX)) as u16,
        position_x: pos.x as i16,
        position_y: pos.y as i16,
        position_z: pos.z as i16,
        vehicle_seat: entry.party_member_vehicle_seat,
        party_type: entry.party_member_party_type,
        phases: entry.party_member_phase_states.clone(),
        auras: entry.party_member_auras.clone(),
        pet_stats: entry.party_member_pet_stats.clone(),
        dungeon_score: Default::default(),
    }
}

fn party_player_info_like_cpp(
    group: &GroupInfo,
    registry: &PlayerRegistry,
    guid: ObjectGuid,
) -> Option<PartyPlayerInfo> {
    let slot = group.member_slot_like_cpp(guid);
    registry.social_recipient(guid).map(|entry| {
        let race = if entry.race == 0 {
            slot.map(|slot| slot.race).unwrap_or_default()
        } else {
            entry.race
        };
        PartyPlayerInfo {
            guid,
            name: if entry.player_name.is_empty() {
                slot.map(|slot| slot.name.clone()).unwrap_or_default()
            } else {
                entry.player_name.clone()
            },
            class: if entry.class == 0 {
                slot.map(|slot| slot.class).unwrap_or_default()
            } else {
                entry.class
            },
            subgroup: slot.map(|slot| slot.subgroup).unwrap_or_default(),
            flags: slot.map(|slot| slot.flags).unwrap_or_default(),
            roles_assigned: slot.map(|slot| slot.roles).unwrap_or_default(),
            faction_group: if race <= 5 { 1 } else { 2 },
            connected: true,
        }
    })
}

/// Sends `PartyUpdate` + `PartyMemberFullState` to every member of `group`.
///
/// Each member gets a `PartyUpdate` where their own `my_index` reflects their
/// position in the member list.  A `PartyMemberFullState` is then sent for
/// every *other* member.
fn send_party_update(group: &GroupInfo, registry: &PlayerRegistry, _vra: u32) {
    // Pre-build the full PlayerList (ALL members including each receiver)
    let all_players: Vec<PartyPlayerInfo> = group
        .members
        .iter()
        .filter_map(|&guid| party_player_info_like_cpp(group, registry, guid))
        .collect();

    for (my_idx, &member_guid) in group.members.iter().enumerate() {
        let member_entry = match registry.control_address(member_guid) {
            Some(e) => e,
            None => continue,
        };

        let update = PartyUpdate {
            party_flags: group.group_flags,
            party_index: group.group_category_like_cpp(),
            party_type: 1,
            my_index: my_idx as i32,
            party_guid: group.group_guid,
            // Filled by the receiver's WorldSession from its per-player
            // `NextGroupUpdateSequenceNumber` state.
            sequence_num: 0,
            leader_guid: group.leader_guid,
            leader_faction_group: 0,
            player_list: all_players.clone(), // ALL members, receiver included
            loot_settings: Some(PartyLootSettings {
                method: group.loot_method,
                loot_master: if group.loot_method == 2 {
                    group.master_looter_guid
                } else {
                    ObjectGuid::EMPTY
                },
                threshold: group.loot_threshold,
            }),
            difficulty_settings: Some(PartyDifficultySettings {
                dungeon_difficulty_id: group.dungeon_difficulty_id,
                raid_difficulty_id: group.raid_difficulty_id,
                legacy_raid_difficulty_id: group.legacy_raid_difficulty_id,
            }),
        };

        let mut member_full_state_packets = Vec::new();
        for &other_guid in &group.members {
            if other_guid == member_guid {
                continue;
            }
            if registry.party_member(other_guid).is_some() {
                let full_state = party_member_full_state_like_cpp(other_guid, Some(registry));
                member_full_state_packets.push(full_state.to_bytes());
            }
        }

        let command = SendPartyUpdateLikeCppCommand {
            recipient: member_guid,
            party_update: update,
            member_full_state_packets,
        };
        #[cfg(not(test))]
        if registry
            .try_send_current_command(
                member_entry.registration(),
                SessionCommand::SendPartyUpdateLikeCpp(command),
            )
            .is_err()
        {
            warn!(member = %member_guid, "failed to queue party update for remote session");
        }
        #[cfg(test)]
        {
            registry
                .send_current_command_blocking_timeout(
                    member_entry.registration(),
                    SessionCommand::SendPartyUpdateLikeCpp(command),
                    Duration::from_secs(1),
                )
                .expect("test session dispatcher accepts PartyUpdate command");
        }
    }
}

async fn send_group_new_leader_like_cpp(
    group: &GroupInfo,
    registry: &PlayerRegistry,
    new_leader_name: &str,
) {
    let packet = GroupNewLeader {
        party_index: group.group_category_like_cpp() as i8,
        name: new_leader_name.to_string(),
    }
    .to_bytes();

    let recipients = registry.group_presences_in_order(&group.members);
    for recipient in recipients {
        let _ = send_realm_packet_to_player_like_cpp(
            registry,
            recipient.registration,
            recipient.guid,
            packet.clone(),
        )
        .await;
    }
}

fn first_connected_group_member_like_cpp(
    group: &GroupInfo,
    registry: &PlayerRegistry,
) -> Option<ObjectGuid> {
    group
        .members
        .iter()
        .copied()
        .find(|member_guid| registry.group_presence(*member_guid).is_some())
}

fn sender_can_start_ready_check_like_cpp(group: &GroupInfo, sender_guid: ObjectGuid) -> bool {
    group.leader_guid == sender_guid
        || group
            .member_slot_like_cpp(sender_guid)
            .is_some_and(|slot| (slot.flags & MEMBER_FLAG_ASSISTANT_LIKE_CPP) != 0)
}

fn current_player_party_invite_map_instance_like_cpp(
    session: &WorldSession,
    registry: &PlayerRegistry,
    player_guid: ObjectGuid,
) -> (u16, u32) {
    if let Some(key) = session.current_canonical_player_map_key_like_cpp() {
        return (key.map_id.min(u32::from(u16::MAX)) as u16, key.instance_id);
    }

    registry
        .group_presence(player_guid)
        .map(|entry| (entry.map_id, entry.instance_id))
        .unwrap_or_else(|| (session.player_map_id_like_cpp(), 0))
}

async fn target_social_ignores_inviter_like_cpp(
    port: Option<std::sync::Arc<dyn SocialPersistencePortLikeCpp>>,
    target_guid: ObjectGuid,
    inviter_guid: ObjectGuid,
    inviter_account_id: u32,
) -> bool {
    let Some(port) = port else {
        return false;
    };

    match port
        .party_invite_target_ignores_like_cpp(
            target_guid.counter(),
            inviter_guid.counter(),
            inviter_account_id,
        )
        .await
    {
        SocialPartyInviteLookupOutcomeLikeCpp::Resolved(ignores) => ignores,
        SocialPartyInviteLookupOutcomeLikeCpp::Failed { reason } => {
            warn!(
                error = %reason,
                target = ?target_guid,
                inviter = ?inviter_guid,
                "PartyInvite social ignore lookup failed"
            );
            false
        }
    }
}

async fn target_social_has_inviter_friend_like_cpp(
    port: Option<std::sync::Arc<dyn SocialPersistencePortLikeCpp>>,
    target_guid: ObjectGuid,
    inviter_guid: ObjectGuid,
) -> bool {
    let Some(port) = port else {
        return false;
    };

    match port
        .party_invite_target_has_friend_like_cpp(target_guid.counter(), inviter_guid.counter())
        .await
    {
        SocialPartyInviteLookupOutcomeLikeCpp::Resolved(has_friend) => has_friend,
        SocialPartyInviteLookupOutcomeLikeCpp::Failed { reason } => {
            warn!(
                error = %reason,
                target = ?target_guid,
                inviter = ?inviter_guid,
                "PartyInvite social friend lookup failed"
            );
            false
        }
    }
}

fn connected_group_members_like_cpp(
    group: &GroupInfo,
    registry: &PlayerRegistry,
) -> Vec<ObjectGuid> {
    group
        .members
        .iter()
        .copied()
        .filter(|member_guid| registry.group_presence(*member_guid).is_some())
        .collect()
}

fn send_ready_check_events_like_cpp(
    events: &[ReadyCheckEventLikeCpp],
    group: &GroupInfo,
    registry: &PlayerRegistry,
) {
    let recipients = registry.group_presences_in_order(&group.members);

    for event in events {
        let bytes = match *event {
            ReadyCheckEventLikeCpp::Started {
                party_index,
                party_guid,
                initiator_guid,
                duration_ms,
            } => ReadyCheckStarted {
                party_index,
                party_guid,
                initiator_guid,
                duration_ms,
            }
            .to_bytes(),
            ReadyCheckEventLikeCpp::Response {
                party_guid,
                player,
                is_ready,
            } => ReadyCheckResponse {
                party_guid,
                player,
                is_ready,
            }
            .to_bytes(),
            ReadyCheckEventLikeCpp::Completed {
                party_index,
                party_guid,
            } => ReadyCheckCompleted {
                party_index,
                party_guid,
            }
            .to_bytes(),
        };

        for recipient in &recipients {
            let _ = registry.send_current_packet(recipient.registration, bytes.clone());
        }
    }
}

fn connected_group_member_txs_like_cpp(
    group: &GroupInfo,
    registry: &PlayerRegistry,
) -> Vec<PlayerRegistration> {
    registry
        .group_presences_in_order(&group.members)
        .into_iter()
        .map(|recipient| recipient.registration)
        .collect()
}

fn send_group_packet_bytes_like_cpp(
    registry: &PlayerRegistry,
    bytes: Vec<u8>,
    recipients: &[PlayerRegistration],
) {
    for recipient in recipients {
        let _ = registry.send_current_packet(*recipient, bytes.clone());
    }
}

fn send_party_uninvite_result_like_cpp(session: &WorldSession, result: u8) {
    session.send_packet_realm(&PartyCommandResult {
        name: String::new(),
        command: 1, // C++ PARTY_OP_UNINVITE
        result,
        result_data: 0,
        // C++ `WorldSession::SendPartyResult` always leaves `ResultGUID`
        // empty (`GroupHandler.cpp:53`).
        result_guid: ObjectGuid::EMPTY,
    });
}

/// Queue a realm-routed packet on the target's owning session.
///
/// Legacy C++ routes party-control packets through REALM
/// (`Opcodes.cpp:1826-1832`). This path receives only the target command
/// sender, so the target session performs the final realm-socket routing.
async fn send_realm_packet_to_player_like_cpp(
    registry: &PlayerRegistry,
    registration: PlayerRegistration,
    recipient: ObjectGuid,
    packet_bytes: Vec<u8>,
) -> bool {
    let command = SessionCommand::SendRealmPacketLikeCpp(SendRealmPacketLikeCppCommand {
        recipient,
        packet_bytes,
    });
    match registry
        .send_current_command_timeout(registration, command, PARTY_REALM_COMMAND_TIMEOUT_LIKE_CPP)
        .await
    {
        Ok(()) => {
            // Test fixtures run a real command dispatcher on another task/thread.
            // Yielding here lets that receiver perform the same session-local
            // routing before packet assertions without adding a wrong-socket
            // production fallback.
            #[cfg(test)]
            tokio::task::yield_now().await;
            true
        }
        Err(
            error @ (crate::session::directory::PlayerDirectorySendError::Disconnected
            | crate::session::directory::PlayerDirectorySendError::StaleRegistration),
        ) => {
            warn!(recipient = %recipient, ?error, "realm-routed party command target stale or closed");
            false
        }
        Err(crate::session::directory::PlayerDirectorySendError::Full) => {
            warn!(recipient = %recipient, "timed out queueing realm-routed party packet");
            false
        }
    }
}

/// Queue the actual invite dialog without treating bounded-channel pressure as
/// an offline player.
///
/// C++ `HandlePartyInviteOpcode` stores the `GroupInvite` and then calls
/// `invitedPlayer->SendDirectMessage`; it does not turn a busy socket queue into
/// `ERR_BAD_PLAYER_NAME_S`. Rust therefore waits until the owning session drains
/// capacity or disconnects. Other existing group notifications keep their
/// finite timeout in [`send_realm_packet_to_player_like_cpp`]; this stronger
/// delivery rule is intentionally scoped to the invite state transition.
async fn send_realm_party_invite_to_player_like_cpp(
    registry: &PlayerRegistry,
    registration: PlayerRegistration,
    recipient: ObjectGuid,
    packet_bytes: Vec<u8>,
) -> bool {
    let command = SessionCommand::SendRealmPacketLikeCpp(SendRealmPacketLikeCppCommand {
        recipient,
        packet_bytes,
    });
    match registry.send_current_command(registration, command).await {
        Ok(()) => {
            #[cfg(test)]
            tokio::task::yield_now().await;
            true
        }
        Err(error) => {
            warn!(recipient = %recipient, ?error, "realm-routed party invite target stale or closed");
            false
        }
    }
}

fn role_changed_inform_like_cpp(
    party_index: u8,
    from: ObjectGuid,
    changed_unit: ObjectGuid,
    old_role: u8,
    new_role: u8,
) -> Vec<u8> {
    RoleChangedInform {
        party_index,
        from,
        changed_unit,
        old_role,
        new_role,
    }
    .to_bytes()
}

fn role_poll_inform_like_cpp(party_index: i8, from: ObjectGuid) -> Vec<u8> {
    RolePollInform { party_index, from }.to_bytes()
}

fn raid_target_update_single_like_cpp(
    party_index: u8,
    symbol: u8,
    target: ObjectGuid,
    changed_by: ObjectGuid,
) -> Vec<u8> {
    SendRaidTargetUpdateSingle {
        party_index,
        target,
        changed_by,
        symbol,
    }
    .to_bytes()
}

fn raid_target_update_all_like_cpp(group: &GroupInfo) -> Vec<u8> {
    SendRaidTargetUpdateAll {
        party_index: group.group_category_like_cpp(),
        target_icons: group.target_icon_list_like_cpp(),
    }
    .to_bytes()
}

fn raid_markers_changed_like_cpp(group: &GroupInfo) -> Vec<u8> {
    RaidMarkersChanged {
        party_index: group.group_category_like_cpp(),
        active_markers: group.active_raid_markers_mask_like_cpp(),
        raid_markers: group
            .raid_marker_list_like_cpp()
            .into_iter()
            .map(|marker| RaidMarker {
                transport_guid: marker.transport_guid,
                map_id: marker.map_id,
                position: marker.position,
            })
            .collect(),
    }
    .to_bytes()
}

async fn queue_visible_gameobjects_or_spellclicks_refresh_like_cpp(
    group: &GroupInfo,
    registry: &PlayerRegistry,
    local_guid: ObjectGuid,
) {
    let recipients: Vec<_> = registry
        .group_presences_in_order(&group.members)
        .into_iter()
        .filter(|member| member.guid != local_guid)
        .collect();

    for member in recipients {
        match registry
            .send_current_command_timeout(
                member.registration,
                crate::session::mailbox::SessionCommand::RefreshVisibleGameobjectsOrSpellClicksLikeCpp,
                PARTY_REALM_COMMAND_TIMEOUT_LIKE_CPP,
            )
            .await
        {
            Ok(()) => {}
            Err(error) => warn!(
                member = %member.guid,
                ?error,
                "failed to queue visible gameobject refresh command"
            ),
        }
    }
}

fn group_type_update_statement_like_cpp(group_flags: u16, db_store_id: u32) -> PreparedStatement {
    let mut stmt = PreparedStatement::for_statement(CharStatements::UPD_GROUP_TYPE);
    stmt.set_u16(0, group_flags);
    stmt.set_u32(1, db_store_id);
    stmt
}

fn group_member_insert_statement_like_cpp(
    db_store_id: u32,
    member_guid: ObjectGuid,
    member_flags: u8,
    subgroup: u8,
    roles: u8,
) -> PreparedStatement {
    let mut stmt = PreparedStatement::for_statement(CharStatements::INS_GROUP_MEMBER);
    stmt.set_u32(0, db_store_id);
    stmt.set_u64(1, member_guid.counter() as u64);
    stmt.set_u8(2, member_flags);
    stmt.set_u8(3, subgroup);
    stmt.set_u8(4, roles);
    stmt
}

fn group_member_subgroup_update_statement_like_cpp(
    member_guid: ObjectGuid,
    subgroup: u8,
) -> PreparedStatement {
    let mut stmt = PreparedStatement::for_statement(CharStatements::UPD_GROUP_MEMBER_SUBGROUP);
    stmt.set_u8(0, subgroup);
    stmt.set_u64(1, member_guid.counter() as u64);
    stmt
}

fn group_member_flag_update_statement_like_cpp(
    member_guid: ObjectGuid,
    member_flags: u8,
) -> PreparedStatement {
    let mut stmt = PreparedStatement::for_statement(CharStatements::UPD_GROUP_MEMBER_FLAG);
    stmt.set_u8(0, member_flags);
    stmt.set_u64(1, member_guid.counter() as u64);
    stmt
}

fn group_member_delete_statement_like_cpp(member_guid: ObjectGuid) -> PreparedStatement {
    let mut stmt = PreparedStatement::for_statement(CharStatements::DEL_GROUP_MEMBER);
    stmt.set_u64(0, member_guid.counter() as u64);
    stmt
}

fn group_leader_update_statement_like_cpp(
    new_leader_guid: ObjectGuid,
    db_store_id: u32,
) -> PreparedStatement {
    let mut stmt = PreparedStatement::for_statement(CharStatements::UPD_GROUP_LEADER);
    stmt.set_u64(0, new_leader_guid.counter() as u64);
    stmt.set_u32(1, db_store_id);
    stmt
}

fn group_delete_statement_like_cpp(db_store_id: u32) -> PreparedStatement {
    let mut stmt = PreparedStatement::for_statement(CharStatements::DEL_GROUP);
    stmt.set_u32(0, db_store_id);
    stmt
}

fn group_member_delete_all_statement_like_cpp(db_store_id: u32) -> PreparedStatement {
    let mut stmt = PreparedStatement::for_statement(CharStatements::DEL_GROUP_MEMBER_ALL);
    stmt.set_u32(0, db_store_id);
    stmt
}

fn group_lfg_data_delete_statement_like_cpp(db_store_id: u32) -> PreparedStatement {
    let mut stmt = PreparedStatement::for_statement(CharStatements::DEL_LFG_DATA);
    stmt.set_u32(0, db_store_id);
    stmt
}

/// Application adapter for the database-neutral durability commands emitted
/// by `GroupRegistry`. The vector order is the transaction order selected by
/// the aggregate; no registry guard survives into database execution.
pub(crate) fn group_persistence_statement_like_cpp(
    intent: GroupPersistenceIntentLikeCpp,
) -> PreparedStatement {
    match intent {
        GroupPersistenceIntentLikeCpp::InsertGroup {
            db_store_id,
            leader_guid,
            loot_method,
            looter_guid,
            loot_threshold,
            group_flags,
            dungeon_difficulty_id,
            raid_difficulty_id,
            legacy_raid_difficulty_id,
            master_looter_guid,
        } => {
            let mut stmt = PreparedStatement::for_statement(CharStatements::INS_GROUP);
            stmt.set_u32(0, db_store_id);
            stmt.set_u64(1, leader_guid.counter() as u64);
            stmt.set_u8(2, loot_method);
            stmt.set_u64(3, looter_guid.counter() as u64);
            stmt.set_u8(4, loot_threshold);
            for index in 0..8 {
                stmt.set_bytes(
                    5 + index,
                    wow_social::group::EMPTY_TARGET_ICON_RAW_LIKE_CPP.to_vec(),
                );
            }
            stmt.set_u16(13, group_flags);
            stmt.set_u32(14, dungeon_difficulty_id);
            stmt.set_u32(15, raid_difficulty_id);
            stmt.set_u32(16, legacy_raid_difficulty_id);
            stmt.set_u64(17, master_looter_guid.counter() as u64);
            stmt
        }
        GroupPersistenceIntentLikeCpp::InsertMember {
            db_store_id,
            member_guid,
            member_flags,
            subgroup,
            roles,
        } => group_member_insert_statement_like_cpp(
            db_store_id,
            member_guid,
            member_flags,
            subgroup,
            roles,
        ),
        GroupPersistenceIntentLikeCpp::DeleteGroup { db_store_id } => {
            group_delete_statement_like_cpp(db_store_id)
        }
        GroupPersistenceIntentLikeCpp::DeleteAllMembers { db_store_id } => {
            group_member_delete_all_statement_like_cpp(db_store_id)
        }
        GroupPersistenceIntentLikeCpp::DeleteLfgData { db_store_id } => {
            group_lfg_data_delete_statement_like_cpp(db_store_id)
        }
        GroupPersistenceIntentLikeCpp::DeleteMember { member_guid } => {
            group_member_delete_statement_like_cpp(member_guid)
        }
        GroupPersistenceIntentLikeCpp::UpdateLeader {
            db_store_id,
            leader_guid,
        } => group_leader_update_statement_like_cpp(leader_guid, db_store_id),
        GroupPersistenceIntentLikeCpp::UpdateGroupType {
            db_store_id,
            group_flags,
        } => group_type_update_statement_like_cpp(group_flags, db_store_id),
        GroupPersistenceIntentLikeCpp::UpdateMemberSubgroup {
            member_guid,
            subgroup,
        } => group_member_subgroup_update_statement_like_cpp(member_guid, subgroup),
        GroupPersistenceIntentLikeCpp::UpdateMemberFlags { member_guid, flags } => {
            group_member_flag_update_statement_like_cpp(member_guid, flags)
        }
        GroupPersistenceIntentLikeCpp::UpdateDifficulty {
            db_store_id,
            kind,
            difficulty_id,
        } => {
            let statement = match kind {
                wow_social::group::GroupDifficultyKindLikeCpp::Dungeon => {
                    CharStatements::UPD_GROUP_DIFFICULTY
                }
                wow_social::group::GroupDifficultyKindLikeCpp::Raid => {
                    CharStatements::UPD_GROUP_RAID_DIFFICULTY
                }
                wow_social::group::GroupDifficultyKindLikeCpp::LegacyRaid => {
                    CharStatements::UPD_GROUP_LEGACY_RAID_DIFFICULTY
                }
            };
            let mut stmt = PreparedStatement::new(statement.sql());
            stmt.set_u32(0, difficulty_id);
            stmt.set_u32(1, db_store_id);
            stmt
        }
    }
}

// ── Handler implementations ───────────────────────────────────────────────────

impl WorldSession {
    async fn persist_group_intents_like_cpp(
        &self,
        group_guid: u64,
        intents: Vec<GroupPersistenceIntentLikeCpp>,
    ) {
        let Some(char_db) = self.char_db().map(std::sync::Arc::clone) else {
            return;
        };
        for intent in intents {
            let stmt = group_persistence_statement_like_cpp(intent);
            if let Err(error) = char_db.execute(&stmt).await {
                warn!(
                    group_guid,
                    %error,
                    "failed to persist represented group transition"
                );
                break;
            }
        }
    }

    /// CMSG_PARTY_INVITE (0x3604)
    ///
    /// Parse layout from C++ `WorldPackets::Party::PartyInviteClient::Read`
    /// (`PartyPackets.cpp`):
    ///   HasBit() → has_party_index
    ///   ResetBitPos()
    ///   ReadBits(9) → name_len
    ///   ReadBits(9) → realm_len
    ///   ReadUInt32  → proposed_roles
    ///   ReadPackedGuid → target_guid
    ///   ReadString(name_len)
    ///   ReadString(realm_len)
    ///   [if has_party_index] ReadUInt8
    pub async fn handle_party_invite(&mut self, mut pkt: wow_packet::WorldPacket) {
        info!(account = self.account_id, "handle_party_invite called");
        // — parse —
        let has_party_index = pkt.read_bit().unwrap_or(false);
        let _ = pkt.reset_bits(); // ResetBitPos / flush partial byte

        let name_len = match pkt.read_bits(9) {
            Ok(n) => n as usize,
            Err(e) => {
                warn!("PartyInvite: name_len read error: {}", e);
                return;
            }
        };
        let realm_len = match pkt.read_bits(9) {
            Ok(n) => n as usize,
            Err(e) => {
                warn!("PartyInvite: realm_len read error: {}", e);
                return;
            }
        };

        let proposed_roles = pkt.read_uint32().unwrap_or(0);

        let _target_guid = match pkt.read_packed_guid() {
            Ok(g) => g,
            Err(e) => {
                warn!("PartyInvite: target_guid read error: {}", e);
                return;
            }
        };
        let target_name = match pkt.read_string(name_len) {
            Ok(s) => s,
            Err(e) => {
                warn!("PartyInvite: target_name read error: {}", e);
                return;
            }
        };
        let _realm_name = pkt.read_string(realm_len).unwrap_or_default();
        let party_index = if has_party_index {
            pkt.read_uint8().ok()
        } else {
            None
        };
        info!(account = self.account_id, target_name = %target_name, "PartyInvite parsed");

        // — setup —
        let my_guid = match self.player_guid() {
            Some(g) => g,
            None => return,
        };

        macro_rules! send_result {
            ($result:expr) => {
                self.send_packet_realm(&PartyCommandResult {
                    name: target_name.clone(),
                    command: 0, // Invite
                    result: $result,
                    result_data: 0,
                    result_guid: ObjectGuid::EMPTY,
                });
            };
        }

        // 2. Target must exist in the player registry (lookup by name — robust against GUID mismatch).
        let registry = match self.player_registry() {
            Some(r) => r,
            None => return,
        };

        // Find target by name (case-insensitive), same pattern as whisper handler.
        let target_snapshot = match registry.social_recipient_by_name(&target_name) {
            Some(target) => target,
            None => {
                warn!(
                    "PartyInvite: target '{}' not found in registry",
                    target_name
                );
                send_result!(party_result::BAD_PLAYER_NAME);
                return;
            }
        };
        let real_target_guid = target_snapshot.guid;

        // Don't invite yourself (compare by real GUID from registry).
        if real_target_guid == my_guid {
            send_result!(party_result::BAD_PLAYER_NAME);
            return;
        }

        // C++ `HandlePartyInviteOpcode` rejects inviting GM targets unless
        // `GM.AllowInvite` / `CONFIG_ALLOW_GM_GROUP` is enabled.
        if !self.allow_gm_group_like_cpp()
            && !self.player_is_game_master_like_cpp()
            && target_snapshot.is_game_master
        {
            send_result!(party_result::BAD_PLAYER_NAME);
            return;
        }

        if !self.allow_two_side_interaction_group_like_cpp()
            && !self.player_is_game_master_like_cpp()
            && player_team_for_race_cpp(self.player_race_like_cpp())
                != player_team_for_race_cpp(target_snapshot.race)
        {
            send_result!(party_result::WRONG_FACTION);
            return;
        }

        let (inviter_map_id, inviter_instance_id) =
            current_player_party_invite_map_instance_like_cpp(self, registry, my_guid);
        if inviter_instance_id != 0
            && target_snapshot.instance_id != 0
            && inviter_instance_id != target_snapshot.instance_id
            && inviter_map_id == target_snapshot.map_id
        {
            send_result!(party_result::TARGET_NOT_IN_INSTANCE);
            return;
        }

        if target_snapshot.instance_id != 0
            && target_snapshot.dungeon_difficulty_id
                != self.represented_dungeon_difficulty_id_like_cpp()
        {
            send_result!(party_result::IGNORING_YOU);
            return;
        }

        let social_port = self.social_persistence_port_like_cpp();
        if target_social_ignores_inviter_like_cpp(
            social_port.clone(),
            real_target_guid,
            my_guid,
            self.account_id,
        )
        .await
        {
            send_result!(party_result::IGNORING_YOU);
            return;
        }

        if u32::from(self.player_level_like_cpp()) < self.party_level_req_like_cpp()
            && !target_social_has_inviter_friend_like_cpp(social_port, real_target_guid, my_guid)
                .await
        {
            send_result!(party_result::INVITE_RESTRICTED);
            return;
        }

        // 3. The owner revalidates pending/group/category/capacity state and
        // records the invite as one transition.
        let pending = match self.pending_invites() {
            Some(p) => p,
            None => return,
        };

        let inviter_name = self.player_name_like_cpp().unwrap_or_default().to_string();
        let vra = self.virtual_realm_address();
        let (realm_name, realm_name_normalized) = self
            .realm_names_for_address_like_cpp(vra)
            .map(|(actual, normalized)| (actual.to_string(), normalized.to_string()))
            .unwrap_or_default();

        let group_reg = match self.group_registry() {
            Some(r) => r,
            None => return,
        };

        let inviter_group_guid =
            current_group_guid_like_cpp(group_reg, self.group_guid, my_guid, party_index);
        let lookup_category = party_index.unwrap_or(GROUP_CATEGORY_HOME_LIKE_CPP);
        let invite = match group_reg.create_invite_like_cpp(
            pending,
            my_guid,
            real_target_guid,
            inviter_group_guid,
            lookup_category,
            GROUP_CATEGORY_HOME_LIKE_CPP,
        ) {
            CreateGroupInviteResultLikeCpp::Created(invite) => invite,
            CreateGroupInviteResultLikeCpp::TargetAlreadyInvited => {
                send_result!(party_result::ALREADY_IN_GROUP);
                return;
            }
            CreateGroupInviteResultLikeCpp::TargetAlreadyGrouped => {
                send_result!(party_result::ALREADY_IN_GROUP);
                let invite = PartyInviteServer {
                    can_accept: false,
                    proposed_roles: proposed_roles as u8,
                    inviter_name: inviter_name.clone(),
                    inviter_guid: my_guid,
                    inviter_bnet_account_guid: ObjectGuid::create_global(
                        HighGuid::WowAccount,
                        0,
                        self.account_id as i64,
                    ),
                    virtual_realm_address: vra,
                    realm_name: realm_name.clone(),
                    realm_name_normalized: realm_name_normalized.clone(),
                };
                let _ = send_realm_packet_to_player_like_cpp(
                    registry,
                    target_snapshot.registration,
                    real_target_guid,
                    invite.to_bytes(),
                )
                .await;
                return;
            }
            CreateGroupInviteResultLikeCpp::InviterNotLeaderOrAssistant => {
                send_result!(party_result::NOT_LEADER);
                return;
            }
            CreateGroupInviteResultLikeCpp::GroupFull => {
                send_result!(party_result::GROUP_FULL);
                return;
            }
            CreateGroupInviteResultLikeCpp::MissingInviterGroup
            | CreateGroupInviteResultLikeCpp::WrongCategory => return,
        };

        // 7. Send invite dialog to the target.
        let invite_packet = PartyInviteServer {
            can_accept: true,
            proposed_roles: proposed_roles as u8,
            inviter_name: inviter_name.clone(),
            inviter_guid: my_guid,
            inviter_bnet_account_guid: ObjectGuid::create_global(
                HighGuid::WowAccount,
                0,
                self.account_id as i64,
            ),
            virtual_realm_address: vra,
            realm_name,
            realm_name_normalized,
        };
        if !send_realm_party_invite_to_player_like_cpp(
            registry,
            target_snapshot.registration,
            real_target_guid,
            invite_packet.to_bytes(),
        )
        .await
        {
            group_reg.cancel_invite_like_cpp(pending, real_target_guid, invite);
            send_result!(party_result::BAD_PLAYER_NAME);
            return;
        }

        // 7. Confirm back to self.
        self.send_packet_realm(&PartyCommandResult {
            name: target_name,
            command: 0,
            result: party_result::OK,
            result_data: 0,
            result_guid: ObjectGuid::EMPTY,
        });
    }

    /// CMSG_PARTY_INVITE_RESPONSE (0x3606)
    ///
    /// Parse layout:
    ///   HasBit() → has_party_index
    ///   HasBit() → accept
    ///   HasBit() → has_roles
    ///   [if has_party_index] ReadUInt8
    ///   [if has_roles]       ReadUInt8
    pub async fn handle_party_invite_response(&mut self, mut pkt: wow_packet::WorldPacket) {
        // — parse —
        let has_party_index = pkt.read_bit().unwrap_or(false);
        let accept = pkt.read_bit().unwrap_or(false);
        let has_roles = pkt.read_bit().unwrap_or(false);

        let party_index = if has_party_index {
            pkt.read_uint8().ok()
        } else {
            None
        };
        if has_roles {
            let _ = pkt.read_uint8();
        }

        // — setup —
        let my_guid = match self.player_guid() {
            Some(g) => g,
            None => return,
        };
        let my_name = self.player_name_like_cpp().unwrap_or_default().to_string();

        // Clone Arcs immediately so we hold no borrow on `self` later.
        let pending = match self.pending_invites() {
            Some(p) => std::sync::Arc::clone(p),
            None => return,
        };

        // 1. Must have a pending C++ `GroupInvite`.
        let invite = match pending.get(&my_guid) {
            Some(invite) => invite,
            None => return,
        };

        let registry = match self.player_registry() {
            Some(r) => std::sync::Arc::clone(r),
            None => return,
        };

        let group_reg = match self.group_registry() {
            Some(r) => std::sync::Arc::clone(r),
            None => return,
        };
        // 2. Declined?
        if !accept {
            let Some(invite) = group_reg.decline_invite_like_cpp(&pending, my_guid, party_index)
            else {
                return;
            };
            if let Some(leader) = registry.group_presence(invite.leader_guid) {
                let decline = GroupDecline { name: my_name };
                let _ = send_realm_packet_to_player_like_cpp(
                    &registry,
                    leader.registration,
                    invite.leader_guid,
                    decline.to_bytes(),
                )
                .await;
            }
            return;
        }

        let leader = registry.group_presence(invite.leader_guid);
        let (group, persistence, refresh_visible_gameobjects_or_spellclicks) = match group_reg
            .accept_invite_like_cpp(
                &pending,
                my_guid,
                party_index,
                leader.as_ref().map(|_| invite.leader_guid),
            ) {
            AcceptGroupInviteResultLikeCpp::NoInvite
            | AcceptGroupInviteResultLikeCpp::WrongCategory
            | AcceptGroupInviteResultLikeCpp::AddFailed
            | AcceptGroupInviteResultLikeCpp::AlreadyMember
            | AcceptGroupInviteResultLikeCpp::MissingGroup
            | AcceptGroupInviteResultLikeCpp::MissingLeader => return,
            AcceptGroupInviteResultLikeCpp::SelfInvite => {
                warn!(
                    player = %my_guid,
                    "HandlePartyInviteResponse: player tried to accept an invite to his own group"
                );
                return;
            }
            AcceptGroupInviteResultLikeCpp::GroupFull => {
                self.send_packet_realm(&PartyCommandResult {
                    name: String::new(),
                    command: 0,
                    result: party_result::GROUP_FULL,
                    result_data: 0,
                    result_guid: ObjectGuid::EMPTY,
                });
                return;
            }
            AcceptGroupInviteResultLikeCpp::JoinedExisting {
                group,
                subgroup: _,
                persistence,
            } => {
                let is_raid_group = group.is_raid_group();
                (group, persistence, is_raid_group)
            }
            AcceptGroupInviteResultLikeCpp::Created {
                group,
                subgroup: _,
                persistence,
            } => {
                if let Some(leader) = leader.as_ref() {
                    let _ = registry.try_send_current_command(
                        leader.registration,
                        SessionCommand::ApplyGroupJoinLikeCpp(ApplyGroupJoinLikeCppCommand {
                            group_guid: group.group_guid,
                            category: group.group_category_like_cpp(),
                            party_type: wow_social::group::GROUP_TYPE_NORMAL_LIKE_CPP,
                            subgroup: 0,
                            refresh_visible_gameobjects_or_spellclicks: false,
                        }),
                    );
                }
                (group, persistence, false)
            }
        };
        let group_guid = group.group_guid;

        // Update self's group_guid in session — all Arc borrows are gone now.
        self.group_guid = Some(group_guid);
        let _ = self.load_represented_group_subgroup_like_cpp();
        if let Some(group) = group_reg.get(&group_guid) {
            self.send_player_party_type_update_like_cpp(
                group.group_category_like_cpp(),
                wow_social::group::GROUP_TYPE_NORMAL_LIKE_CPP,
            );
        }
        self.sync_player_registry_party_member_party_type_like_cpp();
        if refresh_visible_gameobjects_or_spellclicks {
            let _ = self.update_visible_gameobjects_or_spell_clicks_like_cpp();
        }

        if !persistence.is_empty() {
            if let Some(char_db) = self.char_db().map(std::sync::Arc::clone) {
                for intent in persistence {
                    let stmt = group_persistence_statement_like_cpp(intent);
                    if let Err(error) = char_db.execute(&stmt).await {
                        warn!(
                            group_guid = group_guid,
                            %error,
                            "failed to persist represented group transition"
                        );
                        break;
                    }
                }
            }
        }

        // 4. Send PartyUpdate + PartyMemberFullState to all members.
        let vra = self.virtual_realm_address();
        if let Some(group) = group_reg.get(&group_guid) {
            send_party_update(&group, &registry, vra);
        }
    }

    /// CMSG_PARTY_UNINVITE.
    ///
    /// C++ `WorldPackets::Party::PartyUninvite::Read` reads an optional
    /// party-index bit, an 8-bit reason length, target GUID, optional party
    /// index, then the reason string. `HandlePartyUninviteOpcode` rejects self,
    /// checks `CanUninviteFromGroup`, and calls
    /// `Player::RemoveFromGroup(... GROUP_REMOVEMETHOD_KICK ...)` when the
    /// target is a current member.
    pub async fn handle_party_uninvite(&mut self, mut pkt: wow_packet::WorldPacket) {
        let uninvite = match wow_packet::packets::party::PartyUninvite::read(&mut pkt) {
            Ok(packet) => packet,
            Err(error) => {
                warn!("Bad PartyUninvite: {error}");
                return;
            }
        };

        let sender_guid = match self.player_guid() {
            Some(guid) => guid,
            None => return,
        };
        if uninvite.target_guid == sender_guid {
            return;
        }

        let group_reg = match self.group_registry() {
            Some(registry) => std::sync::Arc::clone(registry),
            None => {
                send_party_uninvite_result_like_cpp(self, party_result::NOT_IN_GROUP);
                return;
            }
        };
        let registry = match self.player_registry() {
            Some(registry) => std::sync::Arc::clone(registry),
            None => return,
        };
        let pending_invites = self.pending_invites().map(std::sync::Arc::clone);
        let Some(group_guid) = current_group_guid_like_cpp(
            &group_reg,
            self.group_guid,
            sender_guid,
            uninvite.party_index,
        ) else {
            send_party_uninvite_result_like_cpp(self, party_result::NOT_IN_GROUP);
            return;
        };

        let Some(group_snapshot) = group_reg.get(&group_guid) else {
            return;
        };
        let target_has_loot_rolls = registry
            .group_presence(uninvite.target_guid)
            .is_some_and(|target| target.has_active_loot_rolls);
        let sender_map_id = self.player_map_id_like_cpp();
        let sender_instance_id = self
            .current_canonical_player_map_key_like_cpp()
            .map(|key| key.instance_id)
            .unwrap_or(0);
        let any_member_in_combat = group_snapshot.members.iter().any(|member_guid| {
            if *member_guid == sender_guid {
                self.in_combat
            } else {
                registry.group_presence(*member_guid).is_some_and(|member| {
                    member.in_combat
                        && member.map_id == sender_map_id
                        && member.instance_id == sender_instance_id
                })
            }
        });
        let outcome = match group_reg.remove_member_like_cpp(
            group_guid,
            uninvite.target_guid,
            GroupMemberRemovalKindLikeCpp::Kick {
                actor_guid: sender_guid,
                actor_in_battleground: self.player_in_represented_battleground_like_cpp(),
                target_has_loot_rolls,
                any_member_in_actor_map_combat: any_member_in_combat,
            },
            &[],
        ) {
            Ok(outcome) => outcome,
            Err(GroupAuthorityErrorLikeCpp::LfgBootLimit) => {
                send_party_uninvite_result_like_cpp(self, party_result::PARTY_LFG_BOOT_LIMIT);
                return;
            }
            Err(GroupAuthorityErrorLikeCpp::LfgBootTooFewPlayers) => {
                send_party_uninvite_result_like_cpp(
                    self,
                    party_result::PARTY_LFG_BOOT_TOO_FEW_PLAYERS,
                );
                return;
            }
            Err(GroupAuthorityErrorLikeCpp::LfgBootDungeonComplete) => {
                send_party_uninvite_result_like_cpp(
                    self,
                    party_result::PARTY_LFG_BOOT_DUNGEON_COMPLETE,
                );
                return;
            }
            Err(GroupAuthorityErrorLikeCpp::LfgBootLootRolls) => {
                send_party_uninvite_result_like_cpp(self, party_result::PARTY_LFG_BOOT_LOOT_ROLLS);
                return;
            }
            Err(GroupAuthorityErrorLikeCpp::LfgBootInCombat) => {
                send_party_uninvite_result_like_cpp(self, party_result::PARTY_LFG_BOOT_IN_COMBAT);
                return;
            }
            Err(GroupAuthorityErrorLikeCpp::NotLeaderOrAssistant)
            | Err(GroupAuthorityErrorLikeCpp::TargetIsLeader) => {
                send_party_uninvite_result_like_cpp(self, party_result::NOT_LEADER);
                return;
            }
            Err(GroupAuthorityErrorLikeCpp::InviteRestricted) => {
                send_party_uninvite_result_like_cpp(self, party_result::INVITE_RESTRICTED);
                return;
            }
            Err(GroupAuthorityErrorLikeCpp::MissingMember) => {
                if let Some(pending_invites) = pending_invites.as_ref() {
                    if let Some(invite) = pending_invites
                        .get(&uninvite.target_guid)
                        .filter(|invite| invite.group_guid == Some(group_guid))
                    {
                        group_reg.cancel_invite_like_cpp(
                            pending_invites,
                            uninvite.target_guid,
                            invite,
                        );
                        return;
                    }
                }
                send_party_uninvite_result_like_cpp(self, party_result::TARGET_NOT_IN_GROUP);
                return;
            }
            Err(GroupAuthorityErrorLikeCpp::LfgKickOwnedByVote) => return,
            Err(_) => return,
        };
        let should_disband = outcome.facts.disbanded;
        self.persist_group_intents_like_cpp(group_guid, outcome.persistence)
            .await;

        let cleanup_command = ApplyGroupRemovalLikeCppCommand {
            group_guid,
            category: wow_social::group::GROUP_CATEGORY_HOME_LIKE_CPP,
            party_type: wow_social::group::GROUP_TYPE_NONE_LIKE_CPP,
            send_group_destroyed: should_disband,
            send_group_uninvite: !should_disband,
            refresh_visible_gameobjects_or_spellclicks: true,
        };
        if let Some(target) = registry.group_presence(uninvite.target_guid) {
            let _ = registry.try_send_current_command(
                target.registration,
                SessionCommand::ApplyGroupRemovalLikeCpp(cleanup_command),
            );
        }

        if should_disband {
            self.group_guid = None;
            self.clear_represented_group_subgroup_like_cpp();
            self.send_player_party_type_update_like_cpp(
                wow_social::group::GROUP_CATEGORY_HOME_LIKE_CPP,
                wow_social::group::GROUP_TYPE_NONE_LIKE_CPP,
            );
            self.sync_player_registry_state_like_cpp();
            let _ = self.update_visible_gameobjects_or_spell_clicks_like_cpp();
            self.send_packet_realm(&wow_packet::packets::party::GroupDestroyed);
            // C++ `Group::Disband` sends every member the destroyed
            // `PartyUpdate` after `GroupDestroyed` (`Group.cpp:744-746`).
            self.send_destroyed_group_party_update_like_cpp(
                group_guid,
                wow_social::group::GROUP_CATEGORY_HOME_LIKE_CPP,
            );
            return;
        }

        send_party_update(&outcome.group, &registry, self.virtual_realm_address());
    }

    /// CMSG_LEAVE_GROUP (0x364c)
    ///
    /// Parse layout:
    ///   HasBit() → has_party_index
    ///   [if has_party_index] ReadUInt8
    pub async fn handle_leave_group(&mut self, mut pkt: wow_packet::WorldPacket) {
        // — parse —
        let has_party_index = pkt.read_bit().unwrap_or(false);
        let party_index = if has_party_index {
            pkt.read_uint8().ok()
        } else {
            None
        };

        // — setup —
        let my_guid = match self.player_guid() {
            Some(g) => g,
            None => return,
        };

        // Clone Arcs immediately so we hold no borrow on `self` during mutations.
        let group_reg = match self.group_registry() {
            Some(r) => std::sync::Arc::clone(r),
            None => return,
        };
        let registry = match self.player_registry() {
            Some(r) => std::sync::Arc::clone(r),
            None => return,
        };
        let pending_invites = self.pending_invites().map(std::sync::Arc::clone);
        let vra = self.virtual_realm_address();

        // 1. Find the real group or the C++ `GroupInvite` we're currently in.
        let real_group_guid =
            current_group_guid_like_cpp(&group_reg, self.group_guid, my_guid, party_index);
        let pending_invite = pending_invites
            .as_ref()
            .and_then(|pending| pending.get(&my_guid));

        if real_group_guid.is_none() && pending_invite.is_none() {
            return;
        };

        if self.player_in_represented_battleground_like_cpp() {
            self.send_packet_realm(&PartyCommandResult {
                name: String::new(),
                command: 0,
                result: party_result::INVITE_RESTRICTED,
                result_data: 0,
                result_guid: ObjectGuid::EMPTY,
            });
            return;
        }

        let player_name = self.player_name_like_cpp().unwrap_or_default().to_string();

        if real_group_guid.is_none() {
            if let (Some(pending_invites), Some(invite)) =
                (pending_invites.as_ref(), pending_invite)
            {
                if invite.leader_guid == my_guid {
                    self.send_packet_realm(&PartyCommandResult {
                        name: player_name,
                        command: 2,
                        result: party_result::OK,
                        result_data: 0,
                        result_guid: ObjectGuid::EMPTY,
                    });
                    group_reg.cancel_pending_group_like_cpp(pending_invites, invite);
                }
            }
            return;
        }
        let gid = real_group_guid.expect("checked above");

        self.send_packet_realm(&PartyCommandResult {
            name: player_name,
            command: 2,
            result: party_result::OK,
            result_data: 0,
            result_guid: ObjectGuid::EMPTY,
        });

        // 2. Remove self from the group through the canonical owner. Connected
        // successor candidates are facts; membership and leader choice are
        // revalidated under the group shard.
        let connected_members = group_reg
            .get(&gid)
            .map(|group| connected_group_members_like_cpp(&group, &registry))
            .unwrap_or_default();
        let outcome = match group_reg.remove_member_like_cpp(
            gid,
            my_guid,
            GroupMemberRemovalKindLikeCpp::Leave,
            &connected_members,
        ) {
            Ok(outcome) => outcome,
            Err(_) => return,
        };
        let dissolve_remaining = outcome
            .facts
            .disbanded
            .then(|| outcome.facts.remaining_members.clone());
        self.persist_group_intents_like_cpp(gid, outcome.persistence)
            .await;

        if let Some(remaining) = dissolve_remaining {
            // Group dissolved — notify last remaining member (if any).
            if let Some(&last_guid) = remaining.first() {
                if let Some(last) = registry.group_presence(last_guid) {
                    let command = ApplyGroupRemovalLikeCppCommand {
                        group_guid: gid,
                        category: wow_social::group::GROUP_CATEGORY_HOME_LIKE_CPP,
                        party_type: wow_social::group::GROUP_TYPE_NONE_LIKE_CPP,
                        send_group_destroyed: true,
                        send_group_uninvite: false,
                        refresh_visible_gameobjects_or_spellclicks: true,
                    };
                    let _ = registry.try_send_current_command(
                        last.registration,
                        SessionCommand::ApplyGroupRemovalLikeCpp(command),
                    );
                }
            }
            // Tell self to leave.
            self.group_guid = None;
            self.clear_represented_group_subgroup_like_cpp();
            self.send_player_party_type_update_like_cpp(
                wow_social::group::GROUP_CATEGORY_HOME_LIKE_CPP,
                wow_social::group::GROUP_TYPE_NONE_LIKE_CPP,
            );
            self.sync_player_registry_state_like_cpp();
            let _ = self.update_visible_gameobjects_or_spell_clicks_like_cpp();
            self.send_packet_realm(&GroupUninvite);
            return;
        }

        // 3. Send updated PartyUpdate to remaining members.
        send_party_update(&outcome.group, &registry, vra);

        // 4. Uninvite self.
        self.group_guid = None;
        self.clear_represented_group_subgroup_like_cpp();
        self.send_player_party_type_update_like_cpp(
            wow_social::group::GROUP_CATEGORY_HOME_LIKE_CPP,
            wow_social::group::GROUP_TYPE_NONE_LIKE_CPP,
        );
        self.sync_player_registry_state_like_cpp();
        let _ = self.update_visible_gameobjects_or_spell_clicks_like_cpp();
        self.send_packet_realm(&GroupUninvite);
    }

    /// CMSG_CONVERT_RAID.
    ///
    /// C++ `WorldPackets::Party::ConvertRaid::Read` reads a single `Raid` bit.
    pub async fn handle_convert_raid(&mut self, mut pkt: wow_packet::WorldPacket) {
        let convert = match wow_packet::packets::party::ConvertRaid::read(&mut pkt) {
            Ok(convert) => convert,
            Err(e) => {
                warn!("Bad ConvertRaid: {e}");
                return;
            }
        };

        let my_guid = match self.player_guid() {
            Some(guid) => guid,
            None => return,
        };
        let group_reg = match self.group_registry() {
            Some(registry) => std::sync::Arc::clone(registry),
            None => return,
        };
        let Some(group_guid) =
            current_group_guid_like_cpp(&group_reg, self.group_guid, my_guid, None)
        else {
            return;
        };
        let registry = match self.player_registry() {
            Some(registry) => std::sync::Arc::clone(registry),
            None => return,
        };
        let vra = self.virtual_realm_address();

        let outcome = match group_reg.convert_group_like_cpp(group_guid, my_guid, convert.raid) {
            Ok(outcome) => outcome,
            Err(GroupAuthorityErrorLikeCpp::GroupTooLarge) => {
                self.send_packet_realm(&PartyCommandResult {
                    name: String::new(),
                    command: 0,
                    result: party_result::OK,
                    result_data: 0,
                    result_guid: ObjectGuid::EMPTY,
                });
                return;
            }
            Err(_) => return,
        };
        self.send_packet_realm(&PartyCommandResult {
            name: String::new(),
            command: 0,
            result: party_result::OK,
            result_data: 0,
            result_guid: ObjectGuid::EMPTY,
        });
        self.persist_group_intents_like_cpp(group_guid, outcome.persistence)
            .await;

        // `queue_visible...` may wait on a full member command channel. Clone
        // the value and release DashMap's read guard before the first await so
        // unrelated group mutations are never stalled behind that backpressure.
        send_party_update(&outcome.group, &registry, vra);
        queue_visible_gameobjects_or_spellclicks_refresh_like_cpp(
            &outcome.group,
            &registry,
            my_guid,
        )
        .await;
        let _ = self.update_visible_gameobjects_or_spell_clicks_like_cpp();
    }

    /// CMSG_CHANGE_SUB_GROUP.
    ///
    /// C++ `WorldPackets::Party::ChangeSubGroup::Read` reads target GUID,
    /// target subgroup, then an optional party index bit/value.
    pub async fn handle_change_sub_group(&mut self, mut pkt: wow_packet::WorldPacket) {
        let change = match wow_packet::packets::party::ChangeSubGroup::read(&mut pkt) {
            Ok(change) => change,
            Err(e) => {
                warn!("Bad ChangeSubGroup: {e}");
                return;
            }
        };

        let sender_guid = match self.player_guid() {
            Some(guid) => guid,
            None => return,
        };
        if usize::from(change.new_subgroup) >= wow_social::group::MAX_RAID_SUBGROUPS_LIKE_CPP {
            return;
        }

        let group_reg = match self.group_registry() {
            Some(registry) => std::sync::Arc::clone(registry),
            None => return,
        };
        let registry = match self.player_registry() {
            Some(registry) => std::sync::Arc::clone(registry),
            None => return,
        };
        let vra = self.virtual_realm_address();

        let Some(group_guid) = current_group_guid_like_cpp(
            &group_reg,
            self.group_guid,
            sender_guid,
            change.party_index,
        ) else {
            return;
        };

        let outcome = match group_reg.change_member_subgroup_like_cpp(
            group_guid,
            sender_guid,
            change.target_guid,
            change.new_subgroup,
        ) {
            Ok(outcome) => outcome,
            Err(_) => return,
        };
        let (target_guid, new_subgroup) = outcome.facts;
        self.persist_group_intents_like_cpp(group_guid, outcome.persistence)
            .await;

        if target_guid == sender_guid {
            self.apply_group_subgroup_like_cpp(group_guid, new_subgroup);
        } else if let Some(target) = registry.group_presence(target_guid) {
            let _ = registry.try_send_current_command(
                target.registration,
                SessionCommand::ApplyGroupSubgroupLikeCpp(
                    crate::session::mailbox::ApplyGroupSubgroupLikeCppCommand {
                        group_guid,
                        subgroup: new_subgroup,
                    },
                ),
            );
        }

        send_party_update(&outcome.group, &registry, vra);
    }

    /// CMSG_SWAP_SUB_GROUPS.
    ///
    /// C++ `WorldPackets::Party::SwapSubGroups::Read` reads the optional
    /// party-index bit first, then first/second target GUIDs, then `PartyIndex`
    /// when present. `PartyIndex` is parsed but remains a represented boundary
    /// here: BG/BF/original-group selection is not full parity yet. The bounded
    /// source of truth is the represented `GroupRegistry` state; if a character
    /// DB is attached, the two C++ subgroup update statements are executed in
    /// order after the registry mutation. C++ wraps those statements in a
    /// transaction; Rust does not have real transaction/rollback parity yet.
    pub async fn handle_swap_sub_groups(&mut self, mut pkt: wow_packet::WorldPacket) {
        let swap = match wow_packet::packets::party::SwapSubGroups::read(&mut pkt) {
            Ok(swap) => swap,
            Err(e) => {
                warn!("Bad SwapSubGroups: {e}");
                return;
            }
        };
        let sender_guid = match self.player_guid() {
            Some(guid) => guid,
            None => return,
        };
        let group_reg = match self.group_registry() {
            Some(registry) => std::sync::Arc::clone(registry),
            None => return,
        };
        let registry = match self.player_registry() {
            Some(registry) => std::sync::Arc::clone(registry),
            None => return,
        };
        let vra = self.virtual_realm_address();

        let Some(group_guid) =
            current_group_guid_like_cpp(&group_reg, self.group_guid, sender_guid, swap.party_index)
        else {
            return;
        };

        let outcome = match group_reg.swap_member_subgroups_like_cpp(
            group_guid,
            sender_guid,
            swap.first_target,
            swap.second_target,
        ) {
            Ok(outcome) => outcome,
            Err(_) => return,
        };
        let subgroup_updates = outcome.facts;
        self.persist_group_intents_like_cpp(group_guid, outcome.persistence)
            .await;

        for (member_guid, subgroup) in subgroup_updates {
            if member_guid == sender_guid {
                self.apply_group_subgroup_like_cpp(group_guid, subgroup);
            } else if let Some(member) = registry.group_presence(member_guid) {
                let _ = registry.try_send_current_command(
                    member.registration,
                    SessionCommand::ApplyGroupSubgroupLikeCpp(
                        crate::session::mailbox::ApplyGroupSubgroupLikeCppCommand {
                            group_guid,
                            subgroup,
                        },
                    ),
                );
            }
        }

        send_party_update(&outcome.group, &registry, vra);
    }

    /// CMSG_SET_PARTY_LEADER.
    ///
    /// C++ resolves `ObjectAccessor::FindConnectedPlayer(packet.TargetGUID)`,
    /// gets `GetPlayer()->GetGroup(packet.PartyIndex)`, requires the sender to
    /// be current leader and the target to belong to that same group, then
    /// calls `Group::ChangeLeader` followed by `Group::SendUpdate`.
    ///
    /// Rust preserves the represented state transitions available today:
    /// connected target gate via `PlayerRegistry`, member gate via
    /// `GroupRegistry`, leader mutation, assistant flag removal for the new
    /// leader, optional DB persistence, `GroupNewLeader`, and `PartyUpdate`.
    /// Player flag/name/faction/script side effects remain represented
    /// boundaries until live player objects own those fields.
    pub async fn handle_set_party_leader(&mut self, mut pkt: wow_packet::WorldPacket) {
        let set_leader = match SetPartyLeader::read(&mut pkt) {
            Ok(set_leader) => set_leader,
            Err(e) => {
                warn!("Bad SetPartyLeader: {e}");
                return;
            }
        };
        let sender_guid = match self.player_guid() {
            Some(guid) => guid,
            None => return,
        };
        let group_reg = match self.group_registry() {
            Some(registry) => std::sync::Arc::clone(registry),
            None => return,
        };
        let registry = match self.player_registry() {
            Some(registry) => std::sync::Arc::clone(registry),
            None => return,
        };
        let Some(target) = registry.social_recipient(set_leader.target_guid) else {
            return;
        };
        let target_name = target.player_name;
        let vra = self.virtual_realm_address();

        let Some(group_guid) = current_group_guid_like_cpp(
            &group_reg,
            self.group_guid,
            sender_guid,
            set_leader.party_index,
        ) else {
            return;
        };

        let outcome = match group_reg.change_leader_transition_like_cpp(
            group_guid,
            sender_guid,
            set_leader.target_guid,
        ) {
            Ok(outcome) => outcome,
            Err(_) => return,
        };
        self.persist_group_intents_like_cpp(group_guid, outcome.persistence)
            .await;

        send_group_new_leader_like_cpp(&outcome.group, &registry, &target_name).await;
        send_party_update(&outcome.group, &registry, vra);
    }

    /// CMSG_SET_ASSISTANT_LEADER.
    ///
    /// C++ reads has-party-index bit, apply bit, target GUID and optional
    /// PartyIndex, then resolves `GetPlayer()->GetGroup(packet.PartyIndex)`.
    /// Rust parses PartyIndex but keeps BG/BF/original-group selection as a
    /// represented boundary; source of truth is the current `GroupRegistry`
    /// group. Registry mutation happens before optional CharacterDB persistence
    /// and PartyUpdate fanout, and no await is performed while holding the
    /// mutable group guard.
    pub async fn handle_set_assistant_leader(&mut self, mut pkt: wow_packet::WorldPacket) {
        let set_assistant = match SetAssistantLeader::read(&mut pkt) {
            Ok(set_assistant) => set_assistant,
            Err(e) => {
                warn!("Bad SetAssistantLeader: {e}");
                return;
            }
        };
        let sender_guid = match self.player_guid() {
            Some(guid) => guid,
            None => return,
        };
        let group_reg = match self.group_registry() {
            Some(registry) => std::sync::Arc::clone(registry),
            None => return,
        };
        let registry = match self.player_registry() {
            Some(registry) => std::sync::Arc::clone(registry),
            None => return,
        };
        let vra = self.virtual_realm_address();

        let Some(group_guid) = current_group_guid_like_cpp(
            &group_reg,
            self.group_guid,
            sender_guid,
            set_assistant.party_index,
        ) else {
            return;
        };

        let outcome = match group_reg.set_member_flag_transition_like_cpp(
            group_guid,
            sender_guid,
            set_assistant.target,
            set_assistant.apply,
            MEMBER_FLAG_ASSISTANT_LIKE_CPP,
        ) {
            Ok(outcome) => outcome,
            Err(_) => return,
        };
        self.persist_group_intents_like_cpp(group_guid, outcome.persistence)
            .await;

        send_party_update(&outcome.group, &registry, vra);
    }

    /// CMSG_SET_EVERYONE_IS_ASSISTANT.
    ///
    /// C++ resolves `GetPlayer()->GetGroup(packet.PartyIndex)`, rejects missing
    /// group and non-leader senders, then calls `Group::SetEveryoneIsAssistant`.
    /// Rust parses PartyIndex but keeps BG/BF/original-group selection as a
    /// represented boundary over the current `GroupRegistry` group.
    pub async fn handle_set_everyone_is_assistant(&mut self, mut pkt: wow_packet::WorldPacket) {
        let set_everyone = match SetEveryoneIsAssistant::read(&mut pkt) {
            Ok(set_everyone) => set_everyone,
            Err(e) => {
                warn!("Bad SetEveryoneIsAssistant: {e}");
                return;
            }
        };
        let sender_guid = match self.player_guid() {
            Some(guid) => guid,
            None => return,
        };
        let group_reg = match self.group_registry() {
            Some(registry) => std::sync::Arc::clone(registry),
            None => return,
        };
        let registry = match self.player_registry() {
            Some(registry) => std::sync::Arc::clone(registry),
            None => return,
        };
        let vra = self.virtual_realm_address();

        let Some(group_guid) = current_group_guid_like_cpp(
            &group_reg,
            self.group_guid,
            sender_guid,
            set_everyone.party_index,
        ) else {
            return;
        };

        let outcome = match group_reg.set_everyone_assistant_transition_like_cpp(
            group_guid,
            sender_guid,
            set_everyone.everyone_is_assistant,
        ) {
            Ok(outcome) => outcome,
            Err(_) => return,
        };
        self.persist_group_intents_like_cpp(group_guid, outcome.persistence)
            .await;

        send_party_update(&outcome.group, &registry, vra);
    }

    /// CMSG_SILENCE_PARTY_TALKER.
    ///
    /// C++ parses a full `ObjectGuid Target` followed by one `Silent` bit, then
    /// returns unless the sender is in a group and is the group leader or an
    /// assistant. The live silence mutation is still a TODO in the C++ legacy
    /// source, so Rust records only the represented request at the same boundary.
    pub async fn handle_silence_party_talker(&mut self, mut pkt: wow_packet::WorldPacket) {
        let silence = match SilencePartyTalker::read(&mut pkt) {
            Ok(silence) => silence,
            Err(e) => {
                warn!("Bad SilencePartyTalker: {e}");
                return;
            }
        };
        let sender_guid = match self.player_guid() {
            Some(guid) => guid,
            None => return,
        };
        let group_reg = match self.group_registry() {
            Some(registry) => std::sync::Arc::clone(registry),
            None => return,
        };

        let Some(group_guid) =
            current_group_guid_like_cpp(&group_reg, self.group_guid, sender_guid, None)
        else {
            return;
        };
        let Some(group) = group_reg.get(&group_guid) else {
            return;
        };
        if !group.is_leader_like_cpp(sender_guid) && !group.is_assistant_like_cpp(sender_guid) {
            return;
        }

        self.record_represented_silence_party_talker_like_cpp(silence.target, silence.silent);
    }

    /// CMSG_DO_READY_CHECK.
    ///
    /// C++ resolves `GetPlayer()->GetGroup(packet.PartyIndex)`, returns when no
    /// group exists, requires leader or assistant, then calls
    /// `Group::StartReadyCheck`. Rust represents PartyIndex over the current
    /// GroupRegistry group and approximates offline/no-session via missing
    /// PlayerRegistry entries. Timeout expiry is handled by the shared
    /// `tick_all_group_ready_checks_like_cpp` loop driven from world-server
    /// main. PartyIndex BG/BF/original-group remains a boundary if open.
    pub async fn handle_do_ready_check(&mut self, mut pkt: wow_packet::WorldPacket) {
        let ready_check = match DoReadyCheck::read(&mut pkt) {
            Ok(ready_check) => ready_check,
            Err(e) => {
                warn!("Bad DoReadyCheck: {e}");
                return;
            }
        };
        let sender_guid = match self.player_guid() {
            Some(guid) => guid,
            None => return,
        };
        let group_reg = match self.group_registry() {
            Some(registry) => std::sync::Arc::clone(registry),
            None => return,
        };
        let registry = match self.player_registry() {
            Some(registry) => std::sync::Arc::clone(registry),
            None => return,
        };

        let Some(group_guid) = current_group_guid_like_cpp(
            &group_reg,
            self.group_guid,
            sender_guid,
            ready_check.party_index,
        ) else {
            return;
        };

        let connected = group_reg
            .get(&group_guid)
            .map(|group| connected_group_members_like_cpp(&group, &registry))
            .unwrap_or_default();
        let outcome = match group_reg.start_ready_check_transition_like_cpp(
            group_guid,
            sender_guid,
            connected,
        ) {
            Ok(outcome) => outcome,
            Err(_) => return,
        };
        send_ready_check_events_like_cpp(&outcome.facts, &outcome.group, &registry);
    }

    /// CMSG_READY_CHECK_RESPONSE.
    ///
    /// C++ resolves the group and calls `Group::SetMemberReadyCheck` with no
    /// leader/assistant gate. Rust preserves that represented ownership and
    /// returns with no fanout/state change when no ready check is active.
    pub async fn handle_ready_check_response(&mut self, mut pkt: wow_packet::WorldPacket) {
        let response = match ReadyCheckResponseClient::read(&mut pkt) {
            Ok(response) => response,
            Err(e) => {
                warn!("Bad ReadyCheckResponse: {e}");
                return;
            }
        };
        let sender_guid = match self.player_guid() {
            Some(guid) => guid,
            None => return,
        };
        let group_reg = match self.group_registry() {
            Some(registry) => std::sync::Arc::clone(registry),
            None => return,
        };
        let registry = match self.player_registry() {
            Some(registry) => std::sync::Arc::clone(registry),
            None => return,
        };

        let Some(group_guid) = current_group_guid_like_cpp(
            &group_reg,
            self.group_guid,
            sender_guid,
            response.party_index,
        ) else {
            return;
        };

        let outcome = match group_reg.respond_ready_check_transition_like_cpp(
            group_guid,
            sender_guid,
            response.is_ready,
        ) {
            Ok(outcome) => outcome,
            Err(_) => return,
        };
        send_ready_check_events_like_cpp(&outcome.facts, &outcome.group, &registry);
    }

    /// CMSG_SET_PARTY_ASSIGNMENT.
    ///
    /// C++ resolves `GetPlayer()->GetGroup(packet.PartyIndex)`, requires leader
    /// or raid assistant, maps `GROUP_ASSIGN_MAINTANK`/`GROUP_ASSIGN_MAINASSIST`
    /// to the corresponding unique member flag, calls `RemoveUniqueGroupMemberFlag`
    /// before attempting `SetGroupMemberFlag`, then calls `Group::SendUpdate`
    /// after the switch. Rust keeps PartyIndex as a represented boundary over
    /// the current `GroupRegistry` group; represented unique clears are live
    /// in-memory only, and DB persistence is limited to the target row returned
    /// by the C++-like `SetGroupMemberFlag` path before PartyUpdate fanout.
    pub async fn handle_set_party_assignment(&mut self, mut pkt: wow_packet::WorldPacket) {
        let assignment = match SetPartyAssignment::read(&mut pkt) {
            Ok(assignment) => assignment,
            Err(e) => {
                warn!("Bad SetPartyAssignment: {e}");
                return;
            }
        };
        let sender_guid = match self.player_guid() {
            Some(guid) => guid,
            None => return,
        };
        let group_reg = match self.group_registry() {
            Some(registry) => std::sync::Arc::clone(registry),
            None => return,
        };
        let registry = match self.player_registry() {
            Some(registry) => std::sync::Arc::clone(registry),
            None => return,
        };
        let vra = self.virtual_realm_address();

        let Some(group_guid) = current_group_guid_like_cpp(
            &group_reg,
            self.group_guid,
            sender_guid,
            assignment.party_index,
        ) else {
            return;
        };

        let outcome = match group_reg.set_party_assignment_transition_like_cpp(
            group_guid,
            sender_guid,
            assignment.target,
            assignment.assignment,
            assignment.apply,
        ) {
            Ok(outcome) => outcome,
            Err(_) => return,
        };
        self.persist_group_intents_like_cpp(group_guid, outcome.persistence)
            .await;

        send_party_update(&outcome.group, &registry, vra);
    }

    /// CMSG_SET_ROLE.
    ///
    /// C++ resolves `GetPlayer()->GetGroup(packet.PartyIndex)`, compares the
    /// target's current in-memory LFG roles, broadcasts `RoleChangedInform` to
    /// the group before `SetLfgRoles`, or sends only to the caller when no group
    /// exists. Rust represents PartyIndex as the current `GroupRegistry` group
    /// boundary and keeps `GroupInfo.member_slots.roles` as the in-memory role
    /// source of truth without DB persistence.
    pub async fn handle_set_role(&mut self, mut pkt: wow_packet::WorldPacket) {
        let set_role = match SetRole::read(&mut pkt) {
            Ok(set_role) => set_role,
            Err(e) => {
                warn!("Bad SetRole: {e}");
                return;
            }
        };
        let sender_guid = match self.player_guid() {
            Some(guid) => guid,
            None => return,
        };
        let group_reg = match self.group_registry() {
            Some(registry) => std::sync::Arc::clone(registry),
            None => {
                if set_role.role == 0 {
                    return;
                }
                self.send_packet(&RoleChangedInform {
                    party_index: GROUP_CATEGORY_HOME_LIKE_CPP,
                    from: sender_guid,
                    changed_unit: set_role.target_guid,
                    old_role: 0,
                    new_role: set_role.role,
                });
                return;
            }
        };

        let group_guid = current_group_guid_like_cpp(
            &group_reg,
            self.group_guid,
            sender_guid,
            set_role.party_index,
        );

        let Some(group_guid) = group_guid else {
            if set_role.role == 0 {
                return;
            }
            self.send_packet(&RoleChangedInform {
                party_index: GROUP_CATEGORY_HOME_LIKE_CPP,
                from: sender_guid,
                changed_unit: set_role.target_guid,
                old_role: 0,
                new_role: set_role.role,
            });
            return;
        };

        let registry = self.player_registry().map(std::sync::Arc::clone);
        let outcome = match group_reg.set_lfg_role_transition_like_cpp(
            group_guid,
            set_role.target_guid,
            set_role.role,
        ) {
            Ok(outcome) => outcome,
            Err(_) => return,
        };
        let (old_role, lfg_roles_mutated_existing_target) = outcome.facts;
        let recipients = registry
            .as_ref()
            .map(|registry| connected_group_member_txs_like_cpp(&outcome.group, registry))
            .unwrap_or_default();
        let bytes = role_changed_inform_like_cpp(
            outcome.group.group_category_like_cpp(),
            sender_guid,
            set_role.target_guid,
            old_role,
            set_role.role,
        );

        // C++ broadcasts RoleChangedInform, then Group::SetLfgRoles mutates an
        // existing member slot and calls SendUpdate(). Keep both fanouts outside
        // the mutable guard and only send PartyUpdate when the slot existed.
        if let Some(registry) = registry.as_ref() {
            send_group_packet_bytes_like_cpp(registry, bytes, &recipients);
        }

        if lfg_roles_mutated_existing_target {
            if let Some(registry) = registry.as_ref() {
                let vra = self.virtual_realm_address();
                send_party_update(&outcome.group, registry, vra);
            }
        }
    }

    /// CMSG_UPDATE_RAID_TARGET.
    ///
    /// C++ anchor: `WorldSession::HandleUpdateRaidTargetOpcode` resolves
    /// `GetPlayer()->GetGroup(packet.PartyIndex)`. `Symbol == -1` sends only the
    /// caller a full target icon list. Other symbols call `Group::SetTargetIcon`;
    /// only raid groups gate the action to leader/assistant. Rust keeps
    /// `GroupInfo.target_icons` as canonical represented runtime state. Boundary:
    /// full ObjectAccessor/hostility checks are not represented; connected player
    /// GUID targets are accepted only when present in `PlayerRegistry`, non-player
    /// targets remain pass-through object GUIDs.
    pub async fn handle_update_raid_target(&mut self, mut pkt: wow_packet::WorldPacket) {
        let update = match UpdateRaidTarget::read(&mut pkt) {
            Ok(update) => update,
            Err(e) => {
                warn!("Bad UpdateRaidTarget: {e}");
                return;
            }
        };
        let sender_guid = match self.player_guid() {
            Some(guid) => guid,
            None => return,
        };
        let group_reg = match self.group_registry() {
            Some(registry) => std::sync::Arc::clone(registry),
            None => return,
        };
        let Some(group_guid) = current_group_guid_like_cpp(
            &group_reg,
            self.group_guid,
            sender_guid,
            update.party_index,
        ) else {
            return;
        };

        if update.symbol == -1 {
            if let Some(group) = group_reg.get(&group_guid) {
                self.send_raw_packet(&raid_target_update_all_like_cpp(&group));
            }
            return;
        }

        let Ok(symbol) = u8::try_from(update.symbol) else {
            return;
        };
        let registry = match self.player_registry() {
            Some(registry) => std::sync::Arc::clone(registry),
            None => return,
        };

        if update.target.is_player()
            && !update.target.is_empty()
            && registry.group_presence(update.target).is_none()
        {
            return;
        }

        let outcome = match group_reg.set_target_icon_transition_like_cpp(
            group_guid,
            sender_guid,
            symbol,
            update.target,
        ) {
            Ok(outcome) => outcome,
            Err(_) => return,
        };
        let recipients = connected_group_member_txs_like_cpp(&outcome.group, &registry);
        let party_index = outcome.group.group_category_like_cpp();

        for (changed_symbol, target) in outcome.facts {
            send_group_packet_bytes_like_cpp(
                &registry,
                raid_target_update_single_like_cpp(
                    party_index,
                    changed_symbol,
                    target,
                    sender_guid,
                ),
                &recipients,
            );
        }
    }

    /// CMSG_CLEAR_RAID_MARKER.
    ///
    /// C++ `WorldSession::HandleClearRaidMarker` resolves the player's current
    /// HOME group, gates raid groups to leader/assistant, then calls
    /// `Group::DeleteRaidMarker`. Marker id `8` is the C++ "clear all" sentinel.
    pub async fn handle_clear_raid_marker(&mut self, mut pkt: wow_packet::WorldPacket) {
        let clear = match ClearRaidMarker::read(&mut pkt) {
            Ok(clear) => clear,
            Err(e) => {
                warn!("Bad ClearRaidMarker: {e}");
                return;
            }
        };
        let sender_guid = match self.player_guid() {
            Some(guid) => guid,
            None => return,
        };
        let group_reg = match self.group_registry() {
            Some(registry) => std::sync::Arc::clone(registry),
            None => return,
        };
        let registry = match self.player_registry() {
            Some(registry) => std::sync::Arc::clone(registry),
            None => return,
        };
        let Some(group_guid) =
            current_group_guid_like_cpp(&group_reg, self.group_guid, sender_guid, None)
        else {
            return;
        };

        let outcome = match group_reg.delete_raid_marker_transition_like_cpp(
            group_guid,
            sender_guid,
            clear.marker_id,
        ) {
            Ok(outcome) => outcome,
            Err(_) => return,
        };
        let bytes = raid_markers_changed_like_cpp(&outcome.group);
        let recipients = connected_group_member_txs_like_cpp(&outcome.group, &registry);

        send_group_packet_bytes_like_cpp(&registry, bytes, &recipients);
    }

    /// CMSG_REQUEST_PARTY_JOIN_UPDATES.
    ///
    /// C++ sends current target icons and raid markers for the requested party
    /// index. Rust represents raid target icons and raid marker state from
    /// `GroupInfo`.
    pub async fn handle_request_party_join_updates(&mut self, mut pkt: wow_packet::WorldPacket) {
        let request = match RequestPartyJoinUpdates::read(&mut pkt) {
            Ok(request) => request,
            Err(e) => {
                warn!("Bad RequestPartyJoinUpdates: {e}");
                return;
            }
        };
        let sender_guid = match self.player_guid() {
            Some(guid) => guid,
            None => return,
        };
        let group_reg = match self.group_registry() {
            Some(registry) => std::sync::Arc::clone(registry),
            None => return,
        };
        let Some(group_guid) = current_group_guid_like_cpp(
            &group_reg,
            self.group_guid,
            sender_guid,
            request.party_index,
        ) else {
            return;
        };
        if let Some(group) = group_reg.get(&group_guid) {
            self.send_raw_packet(&raid_target_update_all_like_cpp(&group));
            self.send_raw_packet(&raid_markers_changed_like_cpp(&group));
        }
    }

    /// CMSG_REQUEST_PARTY_MEMBER_STATS.
    ///
    /// C++ `HandleRequestPartyMemberStatsOpcode` always replies to the requester
    /// with `SMSG_PARTY_MEMBER_FULL_STATE`: `ObjectAccessor::FindConnectedPlayer`
    /// drives online/offline status. `PartyIndex` is parsed by the packet layer in
    /// the same bit/GUID/index order as C++, but the C++ handler ignores it.
    pub async fn handle_request_party_member_stats(&mut self, mut pkt: wow_packet::WorldPacket) {
        let request = match RequestPartyMemberStats::read(&mut pkt) {
            Ok(request) => request,
            Err(e) => {
                warn!("Bad RequestPartyMemberStats: {e}");
                return;
            }
        };

        let registry = self.player_registry().map(std::sync::Arc::clone);
        let state = party_member_full_state_like_cpp(request.target_guid, registry.as_deref());
        self.send_packet_realm(&state);
    }

    /// CMSG_INITIATE_ROLE_POLL.
    ///
    /// C++ resolves the current group, returns when sender is neither leader nor
    /// assistant, and broadcasts `RolePollInform` to the group with no state
    /// mutation. Rust keeps the same represented current-group boundary and uses
    /// connected PlayerRegistry recipients instead of full ObjectAccessor/sWorld.
    pub async fn handle_initiate_role_poll(&mut self, mut pkt: wow_packet::WorldPacket) {
        let role_poll = match InitiateRolePoll::read(&mut pkt) {
            Ok(role_poll) => role_poll,
            Err(e) => {
                warn!("Bad InitiateRolePoll: {e}");
                return;
            }
        };
        let sender_guid = match self.player_guid() {
            Some(guid) => guid,
            None => return,
        };
        let group_reg = match self.group_registry() {
            Some(registry) => std::sync::Arc::clone(registry),
            None => return,
        };
        let registry = match self.player_registry() {
            Some(registry) => std::sync::Arc::clone(registry),
            None => return,
        };

        let Some(group_guid) = current_group_guid_like_cpp(
            &group_reg,
            self.group_guid,
            sender_guid,
            role_poll.party_index,
        ) else {
            return;
        };

        let Some((bytes, recipients)) = group_reg.get(&group_guid).and_then(|group| {
            if !sender_can_start_ready_check_like_cpp(&group, sender_guid) {
                return None;
            }
            Some((
                role_poll_inform_like_cpp(group.group_category_like_cpp() as i8, sender_guid),
                connected_group_member_txs_like_cpp(&group, &registry),
            ))
        }) else {
            return;
        };

        send_group_packet_bytes_like_cpp(&registry, bytes, &recipients);
    }

    /// CMSG_SET_LOOT_METHOD.
    ///
    /// This Trinity branch parses the packet but has the entire mutation block
    /// disabled with `// not allowed to change`, so represented Rust preserves
    /// that no-op behavior.
    pub async fn handle_set_loot_method(&mut self, mut pkt: wow_packet::WorldPacket) {
        if let Err(e) = SetLootMethod::read(&mut pkt) {
            warn!("Bad SetLootMethod: {e}");
        }
    }

    /// CMSG_OPT_OUT_OF_LOOT — toggle automatic pass on group-loot rolls.
    pub async fn handle_opt_out_of_loot(&mut self, mut pkt: wow_packet::WorldPacket) {
        let opt_out = match OptOutOfLoot::read(&mut pkt) {
            Ok(opt_out) => opt_out,
            Err(e) => {
                warn!("Bad OptOutOfLoot: {e}");
                return;
            }
        };

        if self.player_guid().is_none() {
            if opt_out.pass_on_loot {
                warn!("CMSG_OPT_OUT_OF_LOOT value<>0 for not-loaded character");
            }
            return;
        }

        self.pass_on_group_loot = opt_out.pass_on_loot;
    }

    /// CMSG_LOW_LEVEL_RAID1 — no-op, C++ only logs at DEBUG level.
    /// C++ anchor: GroupHandler.cpp:740-745
    pub async fn handle_low_level_raid1(&mut self, mut pkt: wow_packet::WorldPacket) {
        if let Err(e) = LowLevelRaid1::read(&mut pkt) {
            warn!("Bad LowLevelRaid1: {e}");
            return;
        }
        if let Some(guid) = self.player_guid() {
            tracing::debug!("HandleLowLevelRaid1 - Player {:?}", guid);
        }
    }

    /// CMSG_LOW_LEVEL_RAID2 — no-op, C++ only logs at DEBUG level.
    /// C++ anchor: GroupHandler.cpp:747-751
    pub async fn handle_low_level_raid2(&mut self, mut pkt: wow_packet::WorldPacket) {
        if let Err(e) = LowLevelRaid2::read(&mut pkt) {
            warn!("Bad LowLevelRaid2: {e}");
            return;
        }
        if let Some(guid) = self.player_guid() {
            tracing::debug!("HandleLowLevelRaid2 - Player {:?}", guid);
        }
    }

    /// CMSG_MINIMAP_PING — broadcasts minimap ping to group members excluding sender.
    ///
    /// C++ anchor: `WorldSession::HandleMinimapPingOpcode`
    /// (`GroupHandler.cpp:401-412`)
    ///
    /// Handler reads `MinimapPingClient`, resolves group via `GroupRegistry`
    /// (finds group containing sender_guid), builds `MinimapPing` server packet
    /// with `Sender`, `PositionX`, `PositionY`, and sends to all connected
    /// group members except sender via `PlayerRegistry` send_tx.
    ///
    /// Boundary: `PartyIndex` is parsed as `Option<u8>` but only used as a
    /// represented semantic boundary; the implementation finds the group
    /// containing the sender_guid in `GroupRegistry` (same pattern as other
    /// group handlers). Multi-group/raid-subgroup PartyIndex selection is not
    /// fully modelled.
    pub async fn handle_minimap_ping(&mut self, mut pkt: wow_packet::WorldPacket) {
        let ping = match MinimapPingClient::read(&mut pkt) {
            Ok(ping) => ping,
            Err(e) => {
                warn!("Bad MinimapPing: {e}");
                return;
            }
        };
        let sender_guid = match self.player_guid() {
            Some(guid) => guid,
            None => return,
        };
        let group_reg = match self.group_registry() {
            Some(registry) => std::sync::Arc::clone(registry),
            None => return,
        };
        let registry = match self.player_registry() {
            Some(registry) => std::sync::Arc::clone(registry),
            None => return,
        };

        let Some(group_guid) =
            current_group_guid_like_cpp(&group_reg, self.group_guid, sender_guid, ping.party_index)
        else {
            return;
        };

        let Some(group) = group_reg.get(&group_guid) else {
            return;
        };

        let bytes = MinimapPing {
            sender: sender_guid,
            position_x: ping.position_x,
            position_y: ping.position_y,
        }
        .to_bytes();

        // C++ BroadcastPacket(packet, true, -1, GetPlayer()->GetGUID()) excludes sender.
        for member_guid in &group.members {
            if *member_guid == sender_guid {
                continue;
            }
            if let Some(member) = registry.group_presence(*member_guid) {
                let _ = registry.send_current_packet(member.registration, bytes.clone());
            }
        }
    }

    /// CMSG_RANDOM_ROLL — validates roll bounds and broadcasts the result.
    ///
    /// C++ anchors:
    /// - `WorldSession::HandleRandomRollOpcode` (`GroupHandler.cpp:414-421`)
    /// - `Player::DoRandomRoll` (`Player.cpp:28718-28734`)
    ///
    /// The client packet contains an optional `PartyIndex`, but C++ ignores it
    /// here: the handler only validates `Min`/`Max` and calls
    /// `GetPlayer()->DoRandomRoll(packet.Min, packet.Max)`, whose group lookup
    /// is `GetGroup()` without a party index. Rust preserves that HOME-group
    /// represented behavior by resolving with `party_index=None`.
    ///
    /// Boundary: C++ reads `int32` but passes the values to a `uint32`
    /// `DoRandomRoll`. Negative ranges therefore hit a signed/unsigned edge in
    /// legacy. Rust keeps the explicit C++ gates and produces a signed result
    /// for the received signed range instead of silently wrapping negatives.
    pub async fn handle_random_roll(&mut self, mut pkt: wow_packet::WorldPacket) {
        let roll = match RandomRollClient::read(&mut pkt) {
            Ok(roll) => roll,
            Err(e) => {
                warn!("Bad RandomRoll: {e}");
                return;
            }
        };

        if roll.min > roll.max || roll.max > 1_000_000 {
            return;
        }

        let Some(sender_guid) = self.player_guid() else {
            return;
        };

        let result = rand::thread_rng().gen_range(roll.min..=roll.max);
        let response = RandomRoll {
            roller: sender_guid,
            roller_wow_account: ObjectGuid::new(
                (HighGuid::WowAccount as i64) << 58,
                i64::from(self.account_id),
            ),
            min: roll.min,
            max: roll.max,
            result,
        };
        let bytes = response.to_bytes();

        let Some(group_reg) = self.group_registry().map(std::sync::Arc::clone) else {
            self.send_packet(&response);
            return;
        };

        let Some(group_guid) =
            current_group_guid_like_cpp(&group_reg, self.group_guid, sender_guid, None)
        else {
            self.send_packet(&response);
            return;
        };

        let Some(group) = group_reg.get(&group_guid) else {
            self.send_packet(&response);
            return;
        };

        let Some(registry) = self.player_registry().map(std::sync::Arc::clone) else {
            self.send_packet(&response);
            return;
        };

        let mut sent_to_sender = false;
        // C++ `group->BroadcastPacket(randomRoll.Write(), false)` includes the roller.
        for member_guid in &group.members {
            if let Some(member) = registry.group_presence(*member_guid) {
                let _ = registry.send_current_packet(member.registration, bytes.clone());
                if *member_guid == sender_guid {
                    sent_to_sender = true;
                }
            }
        }

        if !sent_to_sender {
            self.send_packet(&response);
        }
    }
}

#[cfg(test)]
#[path = "group_tests.rs"]
mod tests;
