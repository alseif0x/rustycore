// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Handlers for Group/Party opcodes: PartyInvite, PartyInviteResponse, LeaveGroup.

use tracing::{info, warn};
use wow_constants::ClientOpcodes;
use wow_core::ObjectGuid;
use wow_database::{CharStatements, PreparedStatement, StatementDef};
use wow_handler::{PacketHandlerEntry, PacketProcessing, SessionStatus};
use wow_network::group_registry::GROUP_CATEGORY_HOME_LIKE_CPP;
use wow_network::player_registry::ApplyGroupRemovalLikeCppCommand;
use wow_network::{
    GROUP_ASSIGN_MAINASSIST_LIKE_CPP, GROUP_ASSIGN_MAINTANK_LIKE_CPP, GroupInfo, GroupRegistry,
    MEMBER_FLAG_ASSISTANT_LIKE_CPP, MEMBER_FLAG_MAINASSIST_LIKE_CPP, MEMBER_FLAG_MAINTANK_LIKE_CPP,
    PlayerRegistry, ReadyCheckEventLikeCpp, SendPartyUpdateLikeCppCommand, SessionCommand,
    free_group_db_store_id_like_cpp, register_group_db_store_id_like_cpp,
};
use wow_packet::packets::party::{
    DoReadyCheck, GroupDecline, GroupNewLeader, GroupUninvite, InitiateRolePoll, LowLevelRaid1,
    LowLevelRaid2, MinimapPing, MinimapPingClient, OptOutOfLoot, PartyCommandResult,
    PartyDifficultySettings, PartyInviteServer, PartyLootSettings, PartyMemberFullState,
    PartyPlayerInfo, PartyUpdate, RaidMarkersChanged, ReadyCheckCompleted, ReadyCheckResponse,
    ReadyCheckResponseClient, ReadyCheckStarted, RequestPartyJoinUpdates, RequestPartyMemberStats,
    RoleChangedInform, RolePollInform, SendRaidTargetUpdateAll, SendRaidTargetUpdateSingle,
    SetAssistantLeader, SetEveryoneIsAssistant, SetLootMethod, SetPartyAssignment, SetPartyLeader,
    SetRole, SilencePartyTalker, UpdateRaidTarget, party_result,
};
use wow_packet::{ClientPacket, ServerPacket};

use crate::session::WorldSession;

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
        .iter()
        .find(|entry| {
            entry.value().members.contains(&sender_guid)
                && entry.value().matches_party_index_like_cpp(party_index)
        })
        .map(|entry| *entry.key())
}

// ── inventory registrations ───────────────────────────────────────────────────

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::PartyInvite,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_party_invite",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::PartyInviteResponse,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_party_invite_response",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::LeaveGroup,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_leave_group",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::ConvertRaid,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_convert_raid",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::ChangeSubGroup,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_change_sub_group",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::SwapSubGroups,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_swap_sub_groups",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::SetLootMethod,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_set_loot_method",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::SetPartyLeader,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_set_party_leader",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::SetAssistantLeader,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_set_assistant_leader",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::SetEveryoneIsAssistant,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_set_everyone_is_assistant",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::SilencePartyTalker,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_silence_party_talker",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::SetPartyAssignment,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_set_party_assignment",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::SetRole,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_set_role",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::InitiateRolePoll,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_initiate_role_poll",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::UpdateRaidTarget,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_update_raid_target",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::RequestPartyJoinUpdates,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_request_party_join_updates",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::RequestPartyMemberStats,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_request_party_member_stats",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::DoReadyCheck,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_do_ready_check",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::ReadyCheckResponse,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_ready_check_response",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::OptOutOfLoot,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_opt_out_of_loot",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::LowLevelRaid1,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_low_level_raid1",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::LowLevelRaid2,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_low_level_raid2",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::MinimapPing,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_minimap_ping",
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn party_member_full_state_like_cpp(
    target_guid: ObjectGuid,
    registry: Option<&PlayerRegistry>,
) -> PartyMemberFullState {
    let Some(entry) = registry.and_then(|registry| registry.get(&target_guid)) else {
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
    registry.get(&guid).map(|entry| {
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
        let member_entry = match registry.get(&member_guid) {
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
            if registry.contains_key(&other_guid) {
                let full_state = party_member_full_state_like_cpp(other_guid, Some(registry));
                member_full_state_packets.push(full_state.to_bytes());
            }
        }

        let command = SendPartyUpdateLikeCppCommand {
            party_update: update,
            member_full_state_packets,
        };
        if member_entry
            .command_tx
            .try_send(SessionCommand::SendPartyUpdateLikeCpp(command.clone()))
            .is_err()
        {
            #[cfg(test)]
            {
                let mut update = command.party_update;
                update.sequence_num = group.sequence_num as i32;
                let _ = member_entry.send_tx.send(update.to_bytes());
                for packet in command.member_full_state_packets {
                    let _ = member_entry.send_tx.send(packet);
                }
            }
        }
    }
}

fn send_group_new_leader_like_cpp(
    group: &GroupInfo,
    registry: &PlayerRegistry,
    new_leader_name: &str,
) {
    let packet = GroupNewLeader {
        party_index: group.group_category_like_cpp() as i8,
        name: new_leader_name.to_string(),
    }
    .to_bytes();

    for &member_guid in &group.members {
        let Some(member_entry) = registry.get(&member_guid) else {
            continue;
        };
        let _ = member_entry.send_tx.try_send(packet.clone());
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
        .find(|member_guid| registry.contains_key(member_guid))
}

fn sender_can_start_ready_check_like_cpp(group: &GroupInfo, sender_guid: ObjectGuid) -> bool {
    group.leader_guid == sender_guid
        || group
            .member_slot_like_cpp(sender_guid)
            .is_some_and(|slot| (slot.flags & MEMBER_FLAG_ASSISTANT_LIKE_CPP) != 0)
}

fn connected_group_members_like_cpp(
    group: &GroupInfo,
    registry: &PlayerRegistry,
) -> Vec<ObjectGuid> {
    group
        .members
        .iter()
        .copied()
        .filter(|member_guid| registry.contains_key(member_guid))
        .collect()
}

fn send_ready_check_events_like_cpp(
    events: &[ReadyCheckEventLikeCpp],
    group: &GroupInfo,
    registry: &PlayerRegistry,
) {
    let recipients: Vec<_> = group
        .members
        .iter()
        .filter_map(|guid| registry.get(guid).map(|entry| entry.send_tx.clone()))
        .collect();

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

        for tx in &recipients {
            let _ = tx.send(bytes.clone());
        }
    }
}

fn connected_group_member_txs_like_cpp(
    group: &GroupInfo,
    registry: &PlayerRegistry,
) -> Vec<flume::Sender<Vec<u8>>> {
    group
        .members
        .iter()
        .filter_map(|guid| registry.get(guid).map(|entry| entry.send_tx.clone()))
        .collect()
}

fn send_group_packet_bytes_like_cpp(bytes: Vec<u8>, recipients: &[flume::Sender<Vec<u8>>]) {
    for tx in recipients {
        let _ = tx.send(bytes.clone());
    }
}

fn send_party_uninvite_result_like_cpp(
    session: &WorldSession,
    result: u8,
    result_guid: ObjectGuid,
) {
    session.send_packet(&PartyCommandResult {
        name: String::new(),
        command: 1, // C++ PARTY_OP_UNINVITE
        result,
        result_data: 0,
        result_guid,
    });
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

fn raid_markers_changed_empty_like_cpp(party_index: u8) -> Vec<u8> {
    RaidMarkersChanged {
        party_index,
        active_markers: 0,
    }
    .to_bytes()
}

fn queue_visible_gameobjects_or_spellclicks_refresh_like_cpp(
    group: &GroupInfo,
    registry: &PlayerRegistry,
    local_guid: ObjectGuid,
) {
    for &member_guid in &group.members {
        if member_guid == local_guid {
            continue;
        }
        if let Some(member) = registry.get(&member_guid) {
            let _ = member.command_tx.try_send(
                wow_network::SessionCommand::RefreshVisibleGameobjectsOrSpellClicksLikeCpp,
            );
        }
    }
}

fn group_type_update_statement_like_cpp(group_flags: u16, db_store_id: u32) -> PreparedStatement {
    let mut stmt = PreparedStatement::new(CharStatements::UPD_GROUP_TYPE.sql());
    stmt.set_u16(0, group_flags);
    stmt.set_u32(1, db_store_id);
    stmt
}

fn group_insert_statement_like_cpp(group: &GroupInfo, db_store_id: u32) -> PreparedStatement {
    let mut stmt = PreparedStatement::new(CharStatements::INS_GROUP.sql());
    stmt.set_u32(0, db_store_id);
    stmt.set_u64(1, group.leader_guid.counter() as u64);
    stmt.set_u8(2, group.loot_method);
    stmt.set_u64(3, group.looter_guid.counter() as u64);
    stmt.set_u8(4, group.loot_threshold);
    for index in 0..8 {
        stmt.set_bytes(
            5 + index,
            wow_network::EMPTY_TARGET_ICON_RAW_LIKE_CPP.to_vec(),
        );
    }
    stmt.set_u16(13, group.group_flags);
    stmt.set_u32(14, group.dungeon_difficulty_id);
    stmt.set_u32(15, group.raid_difficulty_id);
    stmt.set_u32(16, group.legacy_raid_difficulty_id);
    stmt.set_u64(17, group.master_looter_guid.counter() as u64);
    stmt
}

fn group_member_insert_statement_like_cpp(
    db_store_id: u32,
    member_guid: ObjectGuid,
    member_flags: u8,
    subgroup: u8,
    roles: u8,
) -> PreparedStatement {
    let mut stmt = PreparedStatement::new(CharStatements::INS_GROUP_MEMBER.sql());
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
    let mut stmt = PreparedStatement::new(CharStatements::UPD_GROUP_MEMBER_SUBGROUP.sql());
    stmt.set_u8(0, subgroup);
    stmt.set_u64(1, member_guid.counter() as u64);
    stmt
}

fn group_member_flag_update_statement_like_cpp(
    member_guid: ObjectGuid,
    member_flags: u8,
) -> PreparedStatement {
    let mut stmt = PreparedStatement::new(CharStatements::UPD_GROUP_MEMBER_FLAG.sql());
    stmt.set_u8(0, member_flags);
    stmt.set_u64(1, member_guid.counter() as u64);
    stmt
}

fn group_member_delete_statement_like_cpp(member_guid: ObjectGuid) -> PreparedStatement {
    let mut stmt = PreparedStatement::new(CharStatements::DEL_GROUP_MEMBER.sql());
    stmt.set_u64(0, member_guid.counter() as u64);
    stmt
}

fn group_leader_update_statement_like_cpp(
    new_leader_guid: ObjectGuid,
    db_store_id: u32,
) -> PreparedStatement {
    let mut stmt = PreparedStatement::new(CharStatements::UPD_GROUP_LEADER.sql());
    stmt.set_u64(0, new_leader_guid.counter() as u64);
    stmt.set_u32(1, db_store_id);
    stmt
}

fn group_delete_statement_like_cpp(db_store_id: u32) -> PreparedStatement {
    let mut stmt = PreparedStatement::new(CharStatements::DEL_GROUP.sql());
    stmt.set_u32(0, db_store_id);
    stmt
}

fn group_member_delete_all_statement_like_cpp(db_store_id: u32) -> PreparedStatement {
    let mut stmt = PreparedStatement::new(CharStatements::DEL_GROUP_MEMBER_ALL.sql());
    stmt.set_u32(0, db_store_id);
    stmt
}

fn group_lfg_data_delete_statement_like_cpp(db_store_id: u32) -> PreparedStatement {
    let mut stmt = PreparedStatement::new(CharStatements::DEL_LFG_DATA.sql());
    stmt.set_u32(0, db_store_id);
    stmt
}

// ── Handler implementations ───────────────────────────────────────────────────

impl WorldSession {
    /// CMSG_PARTY_INVITE (0x3604)
    ///
    /// Parse layout (C# reference):
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

        let _proposed_roles = pkt.read_uint32().unwrap_or(0);

        let target_guid = match pkt.read_packed_guid() {
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
        if has_party_index {
            let _ = pkt.read_uint8();
        }
        info!(account = self.account_id, target_name = %target_name, "PartyInvite parsed");

        // — setup —
        let my_guid = match self.player_guid() {
            Some(g) => g,
            None => return,
        };

        macro_rules! send_result {
            ($result:expr) => {
                self.send_packet(&PartyCommandResult {
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
        let target_entry_opt = registry
            .iter()
            .find(|e| e.value().player_name.eq_ignore_ascii_case(&target_name));

        let real_target_guid = match target_entry_opt {
            Some(ref e) => *e.key(),
            None => {
                warn!(
                    "PartyInvite: target '{}' not found in registry",
                    target_name
                );
                send_result!(party_result::BAD_PLAYER_NAME);
                return;
            }
        };

        // Don't invite yourself (compare by real GUID from registry).
        if real_target_guid == my_guid {
            send_result!(party_result::BAD_PLAYER_NAME);
            return;
        }

        // 3. Target must not already have a pending invite.
        let pending = match self.pending_invites() {
            Some(p) => p,
            None => return,
        };

        if pending.contains_key(&real_target_guid) {
            send_result!(party_result::ALREADY_IN_GROUP);
            return;
        }

        // 4. Self must not already lead a full group (5 members).
        let group_reg = match self.group_registry() {
            Some(r) => r,
            None => return,
        };

        if let Some(gid) = current_group_guid_like_cpp(group_reg, self.group_guid, my_guid, None) {
            if let Some(g) = group_reg.get(&gid) {
                if g.members.len() >= 5 {
                    send_result!(party_result::GROUP_FULL);
                    return;
                }
            }
        }

        // 5. Record pending invite: target → inviter.
        pending.insert(real_target_guid, my_guid);

        // 6. Send invite dialog to the target.
        let inviter_name = self.player_name_like_cpp().unwrap_or_default().to_string();
        let vra = self.virtual_realm_address();

        if let Some(target_entry) = registry.get(&real_target_guid) {
            let invite = PartyInviteServer {
                can_accept: true,
                inviter_name: inviter_name.clone(),
                inviter_guid: my_guid,
                inviter_bnet_account_guid: ObjectGuid::EMPTY,
                virtual_realm_address: vra,
                realm_name: String::new(),
                realm_name_normalized: String::new(),
            };
            let _ = target_entry.send_tx.send(invite.to_bytes());
        }

        // 7. Confirm back to self.
        self.send_packet(&PartyCommandResult {
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

        if has_party_index {
            let _ = pkt.read_uint8();
        }
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

        // 1. Must have a pending invite.
        let inviter_guid = match pending.get(&my_guid).map(|e| *e) {
            Some(g) => g,
            None => return,
        };
        pending.remove(&my_guid);

        let registry = match self.player_registry() {
            Some(r) => std::sync::Arc::clone(r),
            None => return,
        };

        // 2. Declined?
        if !accept {
            if let Some(inviter_entry) = registry.get(&inviter_guid) {
                let decline = GroupDecline { name: my_name };
                let _ = inviter_entry.send_tx.send(decline.to_bytes());
            }
            return;
        }

        // 3. Accepted — create or extend the group.
        let group_reg = match self.group_registry() {
            Some(r) => std::sync::Arc::clone(r),
            None => return,
        };

        // Find if inviter already has a group.
        let existing_gid: Option<u64> = group_reg
            .iter()
            .find(|entry| entry.value().members.contains(&inviter_guid))
            .map(|entry| *entry.key());

        let mut refresh_visible_gameobjects_or_spellclicks = false;
        let mut group_creation_statements: Vec<PreparedStatement> = Vec::new();
        let persist_member_row = existing_gid.is_some();
        let mut existing_db_store_id: Option<u32> = None;
        let group_guid = if let Some(gid) = existing_gid {
            if let Some(mut g) = group_reg.get_mut(&gid) {
                g.add_member(my_guid);
                existing_db_store_id = Some(g.db_store_id);
                refresh_visible_gameobjects_or_spellclicks = g.is_raid_group();
            }
            gid
        } else {
            // Create a new group with the inviter as leader, then add self.
            let mut new_group = GroupInfo::new(inviter_guid);
            new_group.add_member(my_guid);
            let gid = new_group.group_guid;
            let db_store_id = new_group.db_store_id;
            group_creation_statements
                .push(group_insert_statement_like_cpp(&new_group, db_store_id));
            group_creation_statements.push(group_member_insert_statement_like_cpp(
                db_store_id,
                inviter_guid,
                0,
                0,
                0,
            ));
            group_creation_statements.push(group_member_insert_statement_like_cpp(
                db_store_id,
                my_guid,
                0,
                0,
                0,
            ));
            group_reg.insert(gid, new_group);
            register_group_db_store_id_like_cpp(db_store_id, gid);
            gid
        };

        // Update self's group_guid in session — all Arc borrows are gone now.
        self.group_guid = Some(group_guid);
        if let Some(group) = group_reg.get(&group_guid) {
            self.send_player_party_type_update_like_cpp(
                group.group_category_like_cpp(),
                wow_network::group_registry::GROUP_TYPE_NORMAL_LIKE_CPP,
            );
        }
        self.sync_player_registry_state_like_cpp();
        if refresh_visible_gameobjects_or_spellclicks {
            let _ = self.update_visible_gameobjects_or_spell_clicks_like_cpp();
        }

        if let (true, Some(db_store_id), Some(char_db)) = (
            persist_member_row,
            existing_db_store_id,
            self.char_db().map(std::sync::Arc::clone),
        ) {
            let stmt = group_member_insert_statement_like_cpp(db_store_id, my_guid, 0, 0, 0);
            if let Err(error) = char_db.execute(&stmt).await {
                warn!(
                    group_guid = db_store_id,
                    member_guid = my_guid.counter(),
                    %error,
                    "failed to persist represented group member"
                );
            }
        }

        if !group_creation_statements.is_empty() {
            if let Some(char_db) = self.char_db().map(std::sync::Arc::clone) {
                for stmt in group_creation_statements {
                    if let Err(error) = char_db.execute(&stmt).await {
                        warn!(
                            group_guid = group_guid,
                            %error,
                            "failed to persist represented group creation"
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
                send_party_uninvite_result_like_cpp(
                    self,
                    party_result::NOT_IN_GROUP,
                    uninvite.target_guid,
                );
                return;
            }
        };
        let registry = match self.player_registry() {
            Some(registry) => std::sync::Arc::clone(registry),
            None => return,
        };
        let Some(group_guid) = current_group_guid_like_cpp(
            &group_reg,
            self.group_guid,
            sender_guid,
            uninvite.party_index,
        ) else {
            send_party_uninvite_result_like_cpp(
                self,
                party_result::NOT_IN_GROUP,
                uninvite.target_guid,
            );
            return;
        };

        let mut group_leave_statements: Vec<PreparedStatement> = Vec::new();
        let mut should_disband = false;
        let mut db_store_to_free: Option<u32> = None;
        {
            let mut group = match group_reg.get_mut(&group_guid) {
                Some(group) => group,
                None => return,
            };
            let sender_is_assistant = group
                .member_slot_like_cpp(sender_guid)
                .is_some_and(|slot| (slot.flags & MEMBER_FLAG_ASSISTANT_LIKE_CPP) != 0);
            if group.leader_guid != sender_guid && !sender_is_assistant {
                send_party_uninvite_result_like_cpp(
                    self,
                    party_result::NOT_LEADER_LIKE_CPP,
                    uninvite.target_guid,
                );
                return;
            }
            if group.leader_guid == uninvite.target_guid {
                send_party_uninvite_result_like_cpp(
                    self,
                    party_result::NOT_LEADER_LIKE_CPP,
                    uninvite.target_guid,
                );
                return;
            }
            if !group.members.contains(&uninvite.target_guid) {
                send_party_uninvite_result_like_cpp(
                    self,
                    party_result::TARGET_NOT_IN_GROUP,
                    uninvite.target_guid,
                );
                return;
            }

            group.remove_member(&uninvite.target_guid);
            let db_store_id = group.db_store_id;
            if group.members.len() < 2 {
                group_leave_statements.push(group_delete_statement_like_cpp(db_store_id));
                group_leave_statements
                    .push(group_member_delete_all_statement_like_cpp(db_store_id));
                group_leave_statements.push(group_lfg_data_delete_statement_like_cpp(db_store_id));
                should_disband = true;
                db_store_to_free = Some(db_store_id);
            } else {
                group_leave_statements
                    .push(group_member_delete_statement_like_cpp(uninvite.target_guid));
            }
        }

        if !group_leave_statements.is_empty() {
            if let Some(char_db) = self.char_db().map(std::sync::Arc::clone) {
                for stmt in group_leave_statements {
                    if let Err(error) = char_db.execute(&stmt).await {
                        warn!(
                            group_guid,
                            %error,
                            "failed to persist represented party uninvite"
                        );
                        break;
                    }
                }
            }
        }

        let cleanup_command = ApplyGroupRemovalLikeCppCommand {
            group_guid,
            category: wow_network::group_registry::GROUP_CATEGORY_HOME_LIKE_CPP,
            party_type: wow_network::group_registry::GROUP_TYPE_NONE_LIKE_CPP,
            send_group_destroyed: should_disband,
            send_group_uninvite: !should_disband,
            refresh_visible_gameobjects_or_spellclicks: true,
        };
        if let Some(target_entry) = registry.get(&uninvite.target_guid) {
            let _ = target_entry
                .command_tx
                .try_send(SessionCommand::ApplyGroupRemovalLikeCpp(cleanup_command));
        }

        if should_disband {
            group_reg.remove(&group_guid);
            if let Some(db_store_id) = db_store_to_free {
                free_group_db_store_id_like_cpp(db_store_id);
            }
            self.group_guid = None;
            self.send_player_party_type_update_like_cpp(
                wow_network::group_registry::GROUP_CATEGORY_HOME_LIKE_CPP,
                wow_network::group_registry::GROUP_TYPE_NONE_LIKE_CPP,
            );
            self.sync_player_registry_state_like_cpp();
            let _ = self.update_visible_gameobjects_or_spell_clicks_like_cpp();
            self.send_packet(&wow_packet::packets::party::GroupDestroyed);
            return;
        }

        if let Some(group) = group_reg.get(&group_guid) {
            send_party_update(&group, &registry, self.virtual_realm_address());
        }
    }

    /// CMSG_LEAVE_GROUP (0x364c)
    ///
    /// Parse layout:
    ///   HasBit() → has_party_index
    ///   [if has_party_index] ReadUInt8
    pub async fn handle_leave_group(&mut self, mut pkt: wow_packet::WorldPacket) {
        // — parse —
        let has_party_index = pkt.read_bit().unwrap_or(false);
        if has_party_index {
            let _ = pkt.read_uint8();
        }

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
        let vra = self.virtual_realm_address();

        // 1. Find the group we're currently in.
        let Some(gid) = current_group_guid_like_cpp(&group_reg, self.group_guid, my_guid, None)
        else {
            return;
        };

        // 2. Remove self from the group.
        let dissolve_remaining: Option<Vec<ObjectGuid>>;
        let mut dissolved_db_store_id: Option<u32> = None;
        let mut group_leave_statements: Vec<PreparedStatement> = Vec::new();
        {
            let mut group = match group_reg.get_mut(&gid) {
                Some(g) => g,
                None => return,
            };
            group.remove_member(&my_guid);
            let db_store_id = group.db_store_id;

            if group.members.len() < 2 {
                group_leave_statements.push(group_delete_statement_like_cpp(db_store_id));
                group_leave_statements
                    .push(group_member_delete_all_statement_like_cpp(db_store_id));
                group_leave_statements.push(group_lfg_data_delete_statement_like_cpp(db_store_id));
                dissolved_db_store_id = Some(db_store_id);
                dissolve_remaining = Some(group.members.clone());
            } else {
                dissolve_remaining = None;
                group_leave_statements.push(group_member_delete_statement_like_cpp(my_guid));
                if group.leader_guid == my_guid {
                    if let Some(new_leader) =
                        first_connected_group_member_like_cpp(&group, &registry)
                    {
                        group_leave_statements.push(group_leader_update_statement_like_cpp(
                            new_leader,
                            db_store_id,
                        ));
                    }
                }
                // Reassign leader if needed.
                if group.leader_guid == my_guid {
                    if let Some(new_leader) =
                        first_connected_group_member_like_cpp(&group, &registry)
                    {
                        group.leader_guid = new_leader;
                    }
                }
            }
        }

        if !group_leave_statements.is_empty() {
            if let Some(char_db) = self.char_db().map(std::sync::Arc::clone) {
                for stmt in group_leave_statements {
                    if let Err(error) = char_db.execute(&stmt).await {
                        warn!(
                            group_guid = gid,
                            %error,
                            "failed to persist represented group leave"
                        );
                        break;
                    }
                }
            }
        }

        if let Some(remaining) = dissolve_remaining {
            // Group dissolved — notify last remaining member (if any).
            group_reg.remove(&gid);
            if let Some(db_store_id) = dissolved_db_store_id {
                free_group_db_store_id_like_cpp(db_store_id);
            }
            if let Some(&last_guid) = remaining.first() {
                if let Some(last_entry) = registry.get(&last_guid) {
                    let command = ApplyGroupRemovalLikeCppCommand {
                        group_guid: gid,
                        category: wow_network::group_registry::GROUP_CATEGORY_HOME_LIKE_CPP,
                        party_type: wow_network::group_registry::GROUP_TYPE_NONE_LIKE_CPP,
                        send_group_destroyed: true,
                        send_group_uninvite: false,
                        refresh_visible_gameobjects_or_spellclicks: true,
                    };
                    let _ = last_entry
                        .command_tx
                        .try_send(SessionCommand::ApplyGroupRemovalLikeCpp(command));
                }
            }
            // Tell self to leave.
            self.group_guid = None;
            self.send_player_party_type_update_like_cpp(
                wow_network::group_registry::GROUP_CATEGORY_HOME_LIKE_CPP,
                wow_network::group_registry::GROUP_TYPE_NONE_LIKE_CPP,
            );
            self.sync_player_registry_state_like_cpp();
            let _ = self.update_visible_gameobjects_or_spell_clicks_like_cpp();
            self.send_packet(&GroupUninvite);
            return;
        }

        // 3. Send updated PartyUpdate to remaining members.
        if let Some(group) = group_reg.get(&gid) {
            send_party_update(&group, &registry, vra);
        }

        // 4. Uninvite self.
        self.group_guid = None;
        self.send_player_party_type_update_like_cpp(
            wow_network::group_registry::GROUP_CATEGORY_HOME_LIKE_CPP,
            wow_network::group_registry::GROUP_TYPE_NONE_LIKE_CPP,
        );
        self.sync_player_registry_state_like_cpp();
        let _ = self.update_visible_gameobjects_or_spell_clicks_like_cpp();
        self.send_packet(&GroupUninvite);
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

        let mut group_type_persistence: Option<(u16, u32)> = None;
        let converted = {
            let mut group = match group_reg.get_mut(&group_guid) {
                Some(group) => group,
                None => return,
            };
            if group.leader_guid != my_guid || group.members.len() < 2 {
                return;
            }

            self.send_packet(&PartyCommandResult {
                name: String::new(),
                command: 0,
                result: party_result::OK,
                result_data: 0,
                result_guid: ObjectGuid::EMPTY,
            });

            if convert.raid {
                group.convert_to_raid_like_cpp();
                group_type_persistence = Some((group.group_flags, group.db_store_id));
                true
            } else {
                let converted = group.convert_to_group_like_cpp();
                if converted {
                    group_type_persistence = Some((group.group_flags, group.db_store_id));
                }
                converted
            }
        };

        if !converted {
            return;
        }

        if let (Some((group_flags, db_store_id)), Some(char_db)) = (
            group_type_persistence,
            self.char_db().map(std::sync::Arc::clone),
        ) {
            let stmt = group_type_update_statement_like_cpp(group_flags, db_store_id);
            if let Err(error) = char_db.execute(&stmt).await {
                warn!(
                    group_guid = db_store_id,
                    group_flags,
                    %error,
                    "failed to persist represented group type"
                );
            }
        }

        if let Some(group) = group_reg.get(&group_guid) {
            send_party_update(&group, &registry, vra);
            queue_visible_gameobjects_or_spellclicks_refresh_like_cpp(&group, &registry, my_guid);
        }
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
        if usize::from(change.new_subgroup) >= wow_network::MAX_RAID_SUBGROUPS_LIKE_CPP {
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

        let mut subgroup_update: Option<(ObjectGuid, u8)> = None;
        {
            let mut group = match group_reg.get_mut(&group_guid) {
                Some(group) => group,
                None => return,
            };
            let sender_is_assistant = group.member_slot_like_cpp(sender_guid).is_some_and(|slot| {
                (slot.flags & wow_network::MEMBER_FLAG_ASSISTANT_LIKE_CPP)
                    == wow_network::MEMBER_FLAG_ASSISTANT_LIKE_CPP
            });
            if group.leader_guid != sender_guid && !sender_is_assistant {
                return;
            }
            if !group.has_free_slot_sub_group_like_cpp(change.new_subgroup) {
                return;
            }
            if group.change_member_group_like_cpp(change.target_guid, change.new_subgroup) {
                subgroup_update = Some((change.target_guid, change.new_subgroup));
            }
        }

        let Some((target_guid, new_subgroup)) = subgroup_update else {
            return;
        };

        if let Some(char_db) = self.char_db().map(std::sync::Arc::clone) {
            let stmt = group_member_subgroup_update_statement_like_cpp(target_guid, new_subgroup);
            if let Err(error) = char_db.execute(&stmt).await {
                warn!(
                    member_guid = target_guid.counter(),
                    subgroup = new_subgroup,
                    %error,
                    "failed to persist represented group subgroup change"
                );
            }
        }

        if let Some(group) = group_reg.get(&group_guid) {
            send_party_update(&group, &registry, vra);
        }
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

        let subgroup_updates = {
            let mut group = match group_reg.get_mut(&group_guid) {
                Some(group) => group,
                None => return,
            };
            let sender_is_assistant = group.member_slot_like_cpp(sender_guid).is_some_and(|slot| {
                (slot.flags & wow_network::MEMBER_FLAG_ASSISTANT_LIKE_CPP)
                    == wow_network::MEMBER_FLAG_ASSISTANT_LIKE_CPP
            });
            if group.leader_guid != sender_guid && !sender_is_assistant {
                return;
            }

            group.swap_members_groups_like_cpp(swap.first_target, swap.second_target)
        };

        let Some(subgroup_updates) = subgroup_updates else {
            return;
        };

        if let Some(char_db) = self.char_db().map(std::sync::Arc::clone) {
            for (member_guid, subgroup) in subgroup_updates {
                let stmt = group_member_subgroup_update_statement_like_cpp(member_guid, subgroup);
                if let Err(error) = char_db.execute(&stmt).await {
                    warn!(
                        member_guid = member_guid.counter(),
                        subgroup,
                        %error,
                        "failed to persist represented group subgroup swap"
                    );
                }
            }
        }

        if let Some(group) = group_reg.get(&group_guid) {
            send_party_update(&group, &registry, vra);
        }
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
        let Some(target_entry) = registry.get(&set_leader.target_guid) else {
            return;
        };
        let target_name = target_entry.player_name.clone();
        drop(target_entry);
        let vra = self.virtual_realm_address();

        let Some(group_guid) = current_group_guid_like_cpp(
            &group_reg,
            self.group_guid,
            sender_guid,
            set_leader.party_index,
        ) else {
            return;
        };

        let (db_store_id, final_flags) = {
            let mut group = match group_reg.get_mut(&group_guid) {
                Some(group) => group,
                None => return,
            };
            if !group.is_leader_like_cpp(sender_guid) {
                return;
            }
            if !group.members.contains(&set_leader.target_guid) {
                return;
            }
            let db_store_id = group.db_store_id;
            let Some(final_flags) = group.change_leader_like_cpp(set_leader.target_guid) else {
                return;
            };
            (db_store_id, final_flags)
        };

        if let Some(char_db) = self.char_db().map(std::sync::Arc::clone) {
            let mut statements = Vec::new();
            if db_store_id != 0 {
                statements.push(group_leader_update_statement_like_cpp(
                    set_leader.target_guid,
                    db_store_id,
                ));
            }
            statements.push(group_member_flag_update_statement_like_cpp(
                set_leader.target_guid,
                final_flags,
            ));
            for stmt in statements {
                if let Err(error) = char_db.execute(&stmt).await {
                    warn!(
                        member_guid = set_leader.target_guid.counter(),
                        %error,
                        "failed to persist represented party leader change"
                    );
                }
            }
        }

        if let Some(group) = group_reg.get(&group_guid) {
            send_group_new_leader_like_cpp(&group, &registry, &target_name);
            send_party_update(&group, &registry, vra);
        }
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

        let final_flags = {
            let mut group = match group_reg.get_mut(&group_guid) {
                Some(group) => group,
                None => return,
            };
            if group.leader_guid != sender_guid {
                return;
            }
            group.set_group_member_flag_like_cpp(
                set_assistant.target,
                set_assistant.apply,
                MEMBER_FLAG_ASSISTANT_LIKE_CPP,
            )
        };

        let Some(final_flags) = final_flags else {
            return;
        };

        if let Some(char_db) = self.char_db().map(std::sync::Arc::clone) {
            let stmt =
                group_member_flag_update_statement_like_cpp(set_assistant.target, final_flags);
            if let Err(error) = char_db.execute(&stmt).await {
                warn!(
                    member_guid = set_assistant.target.counter(),
                    flags = final_flags,
                    %error,
                    "failed to persist represented group member flag change"
                );
            }
        }

        if let Some(group) = group_reg.get(&group_guid) {
            send_party_update(&group, &registry, vra);
        }
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

        let (group_flags, db_store_id) = {
            let mut group = match group_reg.get_mut(&group_guid) {
                Some(group) => group,
                None => return,
            };
            if group.leader_guid != sender_guid {
                return;
            }
            group.set_everyone_is_assistant_like_cpp(set_everyone.everyone_is_assistant)
        };

        if let Some(char_db) = self.char_db().map(std::sync::Arc::clone) {
            let stmt = group_type_update_statement_like_cpp(group_flags, db_store_id);
            if let Err(error) = char_db.execute(&stmt).await {
                warn!(
                    group_flags,
                    db_store_id,
                    %error,
                    "failed to persist represented everyone-assistant group flags"
                );
            }
        }

        if let Some(group) = group_reg.get(&group_guid) {
            send_party_update(&group, &registry, vra);
        }
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

        let events = {
            let mut group = match group_reg.get_mut(&group_guid) {
                Some(group) => group,
                None => return,
            };
            if !sender_can_start_ready_check_like_cpp(&group, sender_guid) {
                return;
            }
            let connected = connected_group_members_like_cpp(&group, &registry);
            group.start_ready_check_like_cpp(sender_guid, connected)
        };

        if events.is_empty() {
            return;
        }
        if let Some(group) = group_reg.get(&group_guid) {
            send_ready_check_events_like_cpp(&events, &group, &registry);
        }
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

        let events = {
            let mut group = match group_reg.get_mut(&group_guid) {
                Some(group) => group,
                None => return,
            };
            group.set_member_ready_check_like_cpp(sender_guid, response.is_ready)
        };

        if events.is_empty() {
            return;
        }
        if let Some(group) = group_reg.get(&group_guid) {
            send_ready_check_events_like_cpp(&events, &group, &registry);
        }
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

        let persist_updates = {
            let mut group = match group_reg.get_mut(&group_guid) {
                Some(group) => group,
                None => return,
            };
            let sender_is_assistant = group
                .member_slot_like_cpp(sender_guid)
                .is_some_and(|slot| (slot.flags & MEMBER_FLAG_ASSISTANT_LIKE_CPP) != 0);
            if group.leader_guid != sender_guid && !sender_is_assistant {
                return;
            }

            match assignment.assignment {
                GROUP_ASSIGN_MAINASSIST_LIKE_CPP => {
                    group.remove_unique_group_member_flag_like_cpp(MEMBER_FLAG_MAINASSIST_LIKE_CPP);
                    group
                        .set_group_member_flag_updates_like_cpp(
                            assignment.target,
                            assignment.apply,
                            MEMBER_FLAG_MAINASSIST_LIKE_CPP,
                        )
                        .unwrap_or_default()
                }
                GROUP_ASSIGN_MAINTANK_LIKE_CPP => {
                    group.remove_unique_group_member_flag_like_cpp(MEMBER_FLAG_MAINTANK_LIKE_CPP);
                    group
                        .set_group_member_flag_updates_like_cpp(
                            assignment.target,
                            assignment.apply,
                            MEMBER_FLAG_MAINTANK_LIKE_CPP,
                        )
                        .unwrap_or_default()
                }
                _ => Vec::new(),
            }
        };

        if let Some(char_db) = self.char_db().map(std::sync::Arc::clone) {
            for (member_guid, final_flags) in persist_updates {
                let stmt = group_member_flag_update_statement_like_cpp(member_guid, final_flags);
                if let Err(error) = char_db.execute(&stmt).await {
                    warn!(
                        member_guid = member_guid.counter(),
                        flags = final_flags,
                        %error,
                        "failed to persist represented party assignment member flag change"
                    );
                }
            }
        }

        if let Some(group) = group_reg.get(&group_guid) {
            send_party_update(&group, &registry, vra);
        }
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
        let Some((bytes, recipients)) = group_reg.get(&group_guid).and_then(|group| {
            let old_role = group.get_lfg_roles_like_cpp(set_role.target_guid);
            if old_role == set_role.role {
                return None;
            }
            let recipients = registry
                .as_ref()
                .map(|registry| connected_group_member_txs_like_cpp(&group, registry))
                .unwrap_or_default();
            Some((
                role_changed_inform_like_cpp(
                    group.group_category_like_cpp(),
                    sender_guid,
                    set_role.target_guid,
                    old_role,
                    set_role.role,
                ),
                recipients,
            ))
        }) else {
            return;
        };

        // C++ broadcasts RoleChangedInform, then Group::SetLfgRoles mutates an
        // existing member slot and calls SendUpdate(). Keep both fanouts outside
        // the mutable guard and only send PartyUpdate when the slot existed.
        send_group_packet_bytes_like_cpp(bytes, &recipients);

        let lfg_roles_mutated_existing_target = group_reg
            .get_mut(&group_guid)
            .map(|mut group| group.set_lfg_roles_like_cpp(set_role.target_guid, set_role.role))
            .unwrap_or(false);

        if lfg_roles_mutated_existing_target {
            if let Some(registry) = registry.as_ref() {
                let vra = self.virtual_realm_address();
                if let Some(group) = group_reg.get(&group_guid) {
                    send_party_update(&group, registry, vra);
                }
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
            && !registry.contains_key(&update.target)
        {
            return;
        }

        let Some((updates, recipients, party_index)) = ({
            let mut group = match group_reg.get_mut(&group_guid) {
                Some(group) => group,
                None => return,
            };
            if group.is_raid_group()
                && !group.is_leader_like_cpp(sender_guid)
                && !group.is_assistant_like_cpp(sender_guid)
            {
                return;
            }
            let recipients = connected_group_member_txs_like_cpp(&group, &registry);
            let party_index = group.group_category_like_cpp();
            group
                .set_target_icon_like_cpp(symbol, update.target)
                .map(|updates| (updates, recipients, party_index))
        }) else {
            return;
        };

        for (changed_symbol, target) in updates {
            send_group_packet_bytes_like_cpp(
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

    /// CMSG_REQUEST_PARTY_JOIN_UPDATES.
    ///
    /// C++ sends current target icons and raid markers for the requested party
    /// index. Rust represents raid target icons fully from `GroupInfo.target_icons`
    /// and emits an empty `SMSG_RAID_MARKERS_CHANGED` shell because raid/world
    /// marker state is outside this slice.
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
            self.send_raw_packet(&raid_markers_changed_empty_like_cpp(
                group.group_category_like_cpp(),
            ));
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
        self.send_packet(&state);
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

        send_group_packet_bytes_like_cpp(bytes, &recipients);
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
            if let Some(entry) = registry.get(member_guid) {
                let _ = entry.send_tx.send(bytes.clone());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        current_group_guid_like_cpp, first_connected_group_member_like_cpp,
        group_delete_statement_like_cpp, group_insert_statement_like_cpp,
        group_leader_update_statement_like_cpp, group_lfg_data_delete_statement_like_cpp,
        group_member_delete_all_statement_like_cpp, group_member_delete_statement_like_cpp,
        group_member_flag_update_statement_like_cpp, group_member_insert_statement_like_cpp,
        group_member_subgroup_update_statement_like_cpp, group_type_update_statement_like_cpp,
        party_player_info_like_cpp, send_party_update, send_ready_check_events_like_cpp,
        sender_can_start_ready_check_like_cpp,
    };
    use flume::bounded;
    use std::sync::Arc;
    use wow_constants::{ClientOpcodes, ServerOpcodes};
    use wow_core::{ObjectGuid, Position};
    use wow_database::{CharStatements, SqlParam, StatementDef};
    use wow_handler::{PacketHandlerEntry, PacketProcessing, SessionStatus};
    use wow_network::group_registry::GROUP_CATEGORY_HOME_LIKE_CPP;
    use wow_network::{
        GroupInfo, GroupMemberCharacterLikeCpp, GroupRegistry, PendingInvites, PlayerBroadcastInfo,
        PlayerRegistry, ReadyCheckEventLikeCpp, SessionCommand,
    };
    use wow_packet::{WorldPacket, packets::party::party_result};

    use crate::session::WorldSession;

    fn broadcast_info(guid: ObjectGuid, send_tx: flume::Sender<Vec<u8>>) -> PlayerBroadcastInfo {
        let (command_tx, _command_rx) = flume::bounded(0);
        broadcast_info_with_command_tx(guid, send_tx, command_tx)
    }

    fn broadcast_info_with_command_tx(
        guid: ObjectGuid,
        send_tx: flume::Sender<Vec<u8>>,
        command_tx: flume::Sender<SessionCommand>,
    ) -> PlayerBroadcastInfo {
        PlayerBroadcastInfo {
            map_id: 0,
            instance_id: 0,
            position: Position::ZERO,
            combat_reach: 0.0,
            liquid_status: 0,
            is_in_world: true,
            send_tx,
            command_tx,
            active_loot_rolls: Vec::new(),
            pass_on_group_loot: false,
            enchanting_skill: 0,
            is_alive: true,
            current_health: 100,
            max_health: 100,
            power_type: 0,
            current_power: 0,
            max_power: 0,
            is_pvp: false,
            is_ffa_pvp: false,
            is_ghost: false,
            is_afk: false,
            is_dnd: false,
            auto_reply_msg_like_cpp: String::new(),
            in_vehicle: false,
            has_vehicle_kit_like_cpp: false,
            party_member_vehicle_seat: 0,
            zone_id: 0,
            spec_id: 0,
            unit_flags: 0,
            unit_flags2: 0,
            unit_state: 0,
            is_game_master: false,
            is_contested_pvp: false,
            active_expansion: 2,
            pending_quest_sharing: None,
            known_spells: Vec::new(),
            active_quest_statuses: Default::default(),
            active_quest_objective_counts: Default::default(),
            rewarded_quests: Default::default(),
            daily_quests_completed: Default::default(),
            df_quests: Default::default(),
            faction_template_id: 0,
            reputation_standings: Vec::new(),
            reputation_state_flags: Vec::new(),
            forced_reputation_ranks: Vec::new(),
            forced_reputation_faction_ids: Vec::new(),
            inventory_item_counts: Default::default(),
            party_member_party_type: [0; 2],
            party_member_phase_states: Default::default(),
            party_member_auras: Vec::new(),
            party_member_pet_stats: None,
            player_name: format!("Player{}", guid.low_value()),
            account_id: 1,
            recruiter_id: 0,
            race: 1,
            class: 1,
            sex: 0,
            level: 1,
            gray_level: 0,
            display_id: 49,
            visible_items: [(0, 0, 0); 19],
            lifetime_honorable_kills: 0,
            this_week_contribution: 0,
            yesterday_contribution: 0,
            today_honorable_kills: 0,
            yesterday_honorable_kills: 0,
            lifetime_max_rank: 0,
            honor_level: 0,
        }
    }

    fn packed_guid_bytes(guid: ObjectGuid) -> Vec<u8> {
        let mut pkt = WorldPacket::new_empty();
        pkt.write_packed_guid(&guid);
        pkt.into_data()
    }

    fn set_loot_method_packet(
        has_party_index: bool,
        method: u8,
        master: ObjectGuid,
        threshold: u32,
    ) -> WorldPacket {
        let mut pkt = WorldPacket::new_empty();
        pkt.write_bit(has_party_index);
        pkt.write_uint8(method);
        pkt.write_packed_guid(&master);
        pkt.write_uint32(threshold);
        if has_party_index {
            pkt.write_uint8(0);
        }
        pkt.reset_read();
        pkt
    }

    fn opt_out_of_loot_packet(pass_on_loot: bool) -> WorldPacket {
        let mut pkt = WorldPacket::new_empty();
        pkt.write_bit(pass_on_loot);
        pkt.flush_bits();
        pkt.reset_read();
        pkt
    }

    fn convert_raid_packet(raid: bool) -> WorldPacket {
        let mut pkt = WorldPacket::new_empty();
        pkt.write_bit(raid);
        pkt.flush_bits();
        pkt.reset_read();
        pkt
    }

    fn change_sub_group_packet(
        target_guid: ObjectGuid,
        new_subgroup: u8,
        party_index: Option<u8>,
    ) -> WorldPacket {
        let mut pkt = WorldPacket::new_empty();
        pkt.write_packed_guid(&target_guid);
        pkt.write_uint8(new_subgroup);
        pkt.write_bit(party_index.is_some());
        if let Some(party_index) = party_index {
            pkt.write_uint8(party_index);
        }
        pkt.flush_bits();
        pkt.reset_read();
        pkt
    }

    fn swap_sub_groups_packet(
        first_target: ObjectGuid,
        second_target: ObjectGuid,
        party_index: Option<u8>,
    ) -> WorldPacket {
        let mut pkt = WorldPacket::new_empty();
        pkt.write_bit(party_index.is_some());
        pkt.write_packed_guid(&first_target);
        pkt.write_packed_guid(&second_target);
        if let Some(party_index) = party_index {
            pkt.write_uint8(party_index);
        } else {
            pkt.flush_bits();
        }
        pkt.reset_read();
        pkt
    }

    fn set_assistant_leader_packet(
        target: ObjectGuid,
        apply: bool,
        party_index: Option<u8>,
    ) -> WorldPacket {
        let mut pkt = WorldPacket::new_empty();
        pkt.write_bit(party_index.is_some());
        pkt.write_bit(apply);
        pkt.write_packed_guid(&target);
        if let Some(party_index) = party_index {
            pkt.write_uint8(party_index);
        } else {
            pkt.flush_bits();
        }
        pkt.reset_read();
        pkt
    }

    fn set_party_leader_packet(target: ObjectGuid, party_index: Option<u8>) -> WorldPacket {
        let mut pkt = WorldPacket::new_empty();
        pkt.write_bit(party_index.is_some());
        pkt.write_packed_guid(&target);
        if let Some(party_index) = party_index {
            pkt.write_uint8(party_index);
        } else {
            pkt.flush_bits();
        }
        pkt.reset_read();
        pkt
    }

    fn party_uninvite_packet(
        target: ObjectGuid,
        party_index: Option<u8>,
        reason: &str,
    ) -> WorldPacket {
        let mut pkt = WorldPacket::new_empty();
        pkt.write_bit(party_index.is_some());
        pkt.write_bits(reason.len() as u32, 8);
        pkt.write_guid(&target);
        if let Some(party_index) = party_index {
            pkt.write_uint8(party_index);
        }
        pkt.write_string(reason);
        pkt.flush_bits();
        pkt.reset_read();
        pkt
    }

    fn set_everyone_is_assistant_packet(
        everyone_is_assistant: bool,
        party_index: Option<u8>,
    ) -> WorldPacket {
        let mut pkt = WorldPacket::new_empty();
        pkt.write_bit(party_index.is_some());
        pkt.write_bit(everyone_is_assistant);
        if let Some(party_index) = party_index {
            pkt.write_uint8(party_index);
        } else {
            pkt.flush_bits();
        }
        pkt.reset_read();
        pkt
    }

    fn silence_party_talker_packet(target: ObjectGuid, silent: bool) -> WorldPacket {
        let mut pkt = WorldPacket::new_empty();
        pkt.write_bytes(&target.to_raw_bytes());
        pkt.write_bit(silent);
        pkt.flush_bits();
        pkt.reset_read();
        pkt
    }

    fn set_party_assignment_packet(
        assignment: u8,
        target: ObjectGuid,
        apply: bool,
        party_index: Option<u8>,
    ) -> WorldPacket {
        let mut pkt = WorldPacket::new_empty();
        pkt.write_bit(party_index.is_some());
        pkt.write_bit(apply);
        pkt.write_uint8(assignment);
        pkt.write_packed_guid(&target);
        if let Some(party_index) = party_index {
            pkt.write_uint8(party_index);
        } else {
            pkt.flush_bits();
        }
        pkt.reset_read();
        pkt
    }

    fn set_role_packet(target: ObjectGuid, role: u8, party_index: Option<u8>) -> WorldPacket {
        let mut pkt = WorldPacket::new_empty();
        pkt.write_bit(party_index.is_some());
        pkt.write_packed_guid(&target);
        pkt.write_uint8(role);
        if let Some(party_index) = party_index {
            pkt.write_uint8(party_index);
        } else {
            pkt.flush_bits();
        }
        pkt.reset_read();
        pkt
    }

    fn update_raid_target_packet(
        target: ObjectGuid,
        symbol: i8,
        party_index: Option<u8>,
    ) -> WorldPacket {
        let mut pkt = WorldPacket::new_empty();
        pkt.write_bit(party_index.is_some());
        pkt.write_packed_guid(&target);
        pkt.write_int8(symbol);
        if let Some(party_index) = party_index {
            pkt.write_uint8(party_index);
        } else {
            pkt.flush_bits();
        }
        pkt.reset_read();
        pkt
    }

    fn request_party_join_updates_packet(party_index: Option<u8>) -> WorldPacket {
        let mut pkt = WorldPacket::new_empty();
        pkt.write_bit(party_index.is_some());
        if let Some(party_index) = party_index {
            pkt.write_uint8(party_index);
        } else {
            pkt.flush_bits();
        }
        pkt.reset_read();
        pkt
    }

    fn initiate_role_poll_packet(party_index: Option<u8>) -> WorldPacket {
        let mut pkt = WorldPacket::new_empty();
        pkt.write_bit(party_index.is_some());
        if let Some(party_index) = party_index {
            pkt.write_uint8(party_index);
        } else {
            pkt.flush_bits();
        }
        pkt.reset_read();
        pkt
    }

    fn request_party_member_stats_packet(
        target_guid: ObjectGuid,
        party_index: Option<u8>,
    ) -> WorldPacket {
        let mut pkt = WorldPacket::new_empty();
        pkt.write_bit(party_index.is_some());
        pkt.write_packed_guid(&target_guid);
        if let Some(party_index) = party_index {
            pkt.write_uint8(party_index);
        } else {
            pkt.flush_bits();
        }
        pkt.reset_read();
        pkt
    }

    fn do_ready_check_packet(party_index: Option<u8>) -> WorldPacket {
        let mut pkt = WorldPacket::new_empty();
        pkt.write_bit(party_index.is_some());
        if let Some(party_index) = party_index {
            pkt.write_uint8(party_index);
        } else {
            pkt.flush_bits();
        }
        pkt.reset_read();
        pkt
    }

    fn make_session_with_send() -> (WorldSession, flume::Receiver<Vec<u8>>) {
        let (_pkt_tx, pkt_rx) = bounded::<WorldPacket>(1);
        let (send_tx, send_rx) = bounded::<Vec<u8>>(4);
        (
            WorldSession::new(
                1,
                "TestAccount".into(),
                0,
                2,
                9,
                54261,
                vec![0u8; 40],
                "esES".into(),
                pkt_rx,
                send_tx,
            ),
            send_rx,
        )
    }

    #[test]
    fn party_update_sends_master_looter_only_for_master_loot_like_cpp() {
        let leader = ObjectGuid::create_player(1, 42);
        let master = ObjectGuid::create_player(1, 77);
        let (tx, rx) = bounded(8);
        let registry = PlayerRegistry::default();
        registry.insert(leader, broadcast_info(leader, tx));
        let mut group = GroupInfo::new(leader);
        group.loot_method = 2;
        group.master_looter_guid = master;
        let master_bytes = packed_guid_bytes(master);

        send_party_update(&group, &registry, 0);

        let sent = rx.try_recv().unwrap();
        let mut pkt = WorldPacket::from_bytes(&sent);
        assert_eq!(
            pkt.read_uint16().unwrap(),
            ServerOpcodes::PartyUpdate as u16
        );
        assert!(
            sent.windows(master_bytes.len())
                .any(|window| window == master_bytes.as_slice())
        );

        let (tx, rx) = bounded(8);
        let registry = PlayerRegistry::default();
        registry.insert(leader, broadcast_info(leader, tx));
        group.loot_method = 0;

        send_party_update(&group, &registry, 0);

        let sent = rx.try_recv().unwrap();
        assert!(
            !sent
                .windows(master_bytes.len())
                .any(|window| window == master_bytes.as_slice())
        );
    }

    #[test]
    fn party_update_serializes_raid_group_flag_like_cpp() {
        let leader = ObjectGuid::create_player(1, 42);
        let (tx, rx) = bounded(8);
        let registry = PlayerRegistry::default();
        registry.insert(leader, broadcast_info(leader, tx));
        let mut group = GroupInfo::new(leader);
        group.convert_to_raid_like_cpp();

        send_party_update(&group, &registry, 0);

        let sent = rx.try_recv().unwrap();
        let mut pkt = WorldPacket::from_bytes(&sent);
        assert_eq!(
            pkt.read_uint16().unwrap(),
            ServerOpcodes::PartyUpdate as u16
        );
        assert_eq!(
            pkt.read_uint16().unwrap(),
            wow_network::GROUP_FLAG_RAID_LIKE_CPP
        );
    }

    #[test]
    fn group_type_update_statement_binds_cpp_group_flags_and_db_guid() {
        let stmt = group_type_update_statement_like_cpp(wow_network::GROUP_FLAG_RAID_LIKE_CPP, 77);

        assert_eq!(stmt.sql(), CharStatements::UPD_GROUP_TYPE.sql());
        assert_eq!(
            stmt.params(),
            &[
                SqlParam::U16(wow_network::GROUP_FLAG_RAID_LIKE_CPP),
                SqlParam::U32(77)
            ]
        );
    }

    #[test]
    fn group_member_insert_statement_binds_cpp_member_row_like_cpp() {
        let member = ObjectGuid::create_player(1, 42);
        let stmt = group_member_insert_statement_like_cpp(77, member, 0, 3, 2);

        assert_eq!(stmt.sql(), CharStatements::INS_GROUP_MEMBER.sql());
        assert_eq!(
            stmt.params(),
            &[
                SqlParam::U32(77),
                SqlParam::U64(member.counter() as u64),
                SqlParam::U8(0),
                SqlParam::U8(3),
                SqlParam::U8(2)
            ]
        );
    }

    #[test]
    fn group_member_subgroup_update_statement_binds_cpp_member_row_like_cpp() {
        let member = ObjectGuid::create_player(1, 42);
        let stmt = group_member_subgroup_update_statement_like_cpp(member, 6);

        assert_eq!(stmt.sql(), CharStatements::UPD_GROUP_MEMBER_SUBGROUP.sql());
        assert_eq!(
            stmt.params(),
            &[SqlParam::U8(6), SqlParam::U64(member.counter() as u64)]
        );
    }

    #[test]
    fn group_member_flag_update_statement_binds_cpp_member_row_like_cpp() {
        let member = ObjectGuid::create_player(1, 42);
        let stmt = group_member_flag_update_statement_like_cpp(member, 0x01);

        assert_eq!(stmt.sql(), CharStatements::UPD_GROUP_MEMBER_FLAG.sql());
        assert_eq!(
            stmt.params(),
            &[SqlParam::U8(0x01), SqlParam::U64(member.counter() as u64)]
        );
    }

    #[test]
    fn group_insert_statement_binds_cpp_group_row_like_cpp() {
        let leader = ObjectGuid::create_player(1, 42);
        let group = GroupInfo::new(leader);
        let stmt = group_insert_statement_like_cpp(&group, 77);

        assert_eq!(stmt.sql(), CharStatements::INS_GROUP.sql());
        assert_eq!(stmt.params().len(), 18);
        assert_eq!(stmt.params()[0], SqlParam::U32(77));
        assert_eq!(stmt.params()[1], SqlParam::U64(leader.counter() as u64));
        assert_eq!(
            stmt.params()[2],
            SqlParam::U8(wow_network::LOOT_METHOD_PERSONAL_LIKE_CPP)
        );
        assert_eq!(stmt.params()[3], SqlParam::U64(leader.counter() as u64));
        assert_eq!(stmt.params()[4], SqlParam::U8(2));
        for param in &stmt.params()[5..13] {
            assert_eq!(param, &SqlParam::Bytes(vec![0; 16]));
        }
        assert_eq!(stmt.params()[13], SqlParam::U16(0));
        assert_eq!(stmt.params()[14], SqlParam::U32(1));
        assert_eq!(stmt.params()[15], SqlParam::U32(14));
        assert_eq!(stmt.params()[16], SqlParam::U32(3));
        assert_eq!(stmt.params()[17], SqlParam::U64(0));
    }

    #[test]
    fn group_leave_statements_bind_cpp_cleanup_rows_like_cpp() {
        let old_member = ObjectGuid::create_player(1, 42);
        let new_leader = ObjectGuid::create_player(1, 77);

        let stmt = group_member_delete_statement_like_cpp(old_member);
        assert_eq!(stmt.sql(), CharStatements::DEL_GROUP_MEMBER.sql());
        assert_eq!(stmt.params(), &[SqlParam::U64(old_member.counter() as u64)]);

        let stmt = group_leader_update_statement_like_cpp(new_leader, 99);
        assert_eq!(stmt.sql(), CharStatements::UPD_GROUP_LEADER.sql());
        assert_eq!(
            stmt.params(),
            &[
                SqlParam::U64(new_leader.counter() as u64),
                SqlParam::U32(99)
            ]
        );

        let stmt = group_delete_statement_like_cpp(99);
        assert_eq!(stmt.sql(), CharStatements::DEL_GROUP.sql());
        assert_eq!(stmt.params(), &[SqlParam::U32(99)]);

        let stmt = group_member_delete_all_statement_like_cpp(99);
        assert_eq!(stmt.sql(), CharStatements::DEL_GROUP_MEMBER_ALL.sql());
        assert_eq!(stmt.params(), &[SqlParam::U32(99)]);

        let stmt = group_lfg_data_delete_statement_like_cpp(99);
        assert_eq!(stmt.sql(), CharStatements::DEL_LFG_DATA.sql());
        assert_eq!(stmt.params(), &[SqlParam::U32(99)]);
    }

    #[test]
    fn group_leave_selects_first_connected_new_leader_like_cpp() {
        let leader = ObjectGuid::create_player(1, 42);
        let disconnected = ObjectGuid::create_player(1, 77);
        let connected = ObjectGuid::create_player(1, 88);
        let mut group = GroupInfo::new(leader);
        group.add_member(disconnected);
        group.add_member(connected);
        group.remove_member(&leader);

        let registry = PlayerRegistry::default();
        let (tx, _rx) = bounded(1);
        registry.insert(connected, broadcast_info(connected, tx));

        assert_eq!(
            first_connected_group_member_like_cpp(&group, &registry),
            Some(connected)
        );
    }

    #[tokio::test]
    async fn leave_group_disband_queues_remote_group_removal_like_cpp() {
        let (mut session, _send_rx) = make_session_with_send();
        let leaving_guid = ObjectGuid::create_player(1, 42);
        let last_guid = ObjectGuid::create_player(1, 77);
        let (last_send_tx, _last_send_rx) = bounded(8);
        let (last_command_tx, last_command_rx) = bounded(8);
        let player_registry = Arc::new(PlayerRegistry::default());
        player_registry.insert(
            last_guid,
            broadcast_info_with_command_tx(last_guid, last_send_tx, last_command_tx),
        );
        let group_registry = Arc::new(GroupRegistry::default());
        let mut group = GroupInfo::new(leaving_guid);
        group.add_member(last_guid);
        let group_guid = group.group_guid;
        group_registry.insert(group_guid, group);

        session.set_player_guid(Some(leaving_guid));
        session.group_guid = Some(group_guid);
        session.set_player_registry(player_registry);
        session.set_group_registry(group_registry, Arc::new(PendingInvites::default()));

        let mut pkt = WorldPacket::new_empty();
        pkt.write_bit(false);
        pkt.flush_bits();
        pkt.reset_read();
        session.handle_leave_group(pkt).await;

        let command = last_command_rx.try_recv().unwrap();
        let SessionCommand::ApplyGroupRemovalLikeCpp(command) = command else {
            panic!("expected ApplyGroupRemovalLikeCpp for remote disband cleanup");
        };
        assert_eq!(command.group_guid, group_guid);
        assert_eq!(command.category, GROUP_CATEGORY_HOME_LIKE_CPP);
        assert_eq!(
            command.party_type,
            wow_network::group_registry::GROUP_TYPE_NONE_LIKE_CPP
        );
        assert!(command.send_group_destroyed);
        assert!(command.refresh_visible_gameobjects_or_spellclicks);
    }

    #[tokio::test]
    async fn party_uninvite_leader_queues_remote_remove_member_cleanup_like_cpp() {
        let (mut session, _send_rx) = make_session_with_send();
        let leader = ObjectGuid::create_player(1, 42);
        let target = ObjectGuid::create_player(1, 77);
        let remaining = ObjectGuid::create_player(1, 88);
        let (leader_tx, leader_rx) = bounded(8);
        let (target_tx, _target_rx) = bounded(8);
        let (target_command_tx, target_command_rx) = bounded(8);
        let (remaining_tx, remaining_rx) = bounded(8);
        let player_registry = Arc::new(PlayerRegistry::default());
        player_registry.insert(leader, broadcast_info(leader, leader_tx));
        player_registry.insert(
            target,
            broadcast_info_with_command_tx(target, target_tx, target_command_tx),
        );
        player_registry.insert(remaining, broadcast_info(remaining, remaining_tx));
        let group_registry = Arc::new(GroupRegistry::default());
        let mut group = GroupInfo::new(leader);
        group.add_member(target);
        group.add_member(remaining);
        let group_guid = group.group_guid;
        group_registry.insert(group_guid, group);

        session.set_player_guid(Some(leader));
        session.group_guid = Some(group_guid);
        session.set_player_registry(Arc::clone(&player_registry));
        session.set_group_registry(
            Arc::clone(&group_registry),
            Arc::new(PendingInvites::default()),
        );

        session
            .handle_party_uninvite(party_uninvite_packet(target, None, "bye"))
            .await;

        let group = group_registry.get(&group_guid).unwrap();
        assert!(!group.members.contains(&target));
        assert!(group.members.contains(&leader));
        assert!(group.members.contains(&remaining));
        drop(group);

        let command = target_command_rx.try_recv().unwrap();
        let SessionCommand::ApplyGroupRemovalLikeCpp(command) = command else {
            panic!("expected ApplyGroupRemovalLikeCpp for kicked member");
        };
        assert_eq!(command.group_guid, group_guid);
        assert_eq!(command.category, GROUP_CATEGORY_HOME_LIKE_CPP);
        assert_eq!(
            command.party_type,
            wow_network::group_registry::GROUP_TYPE_NONE_LIKE_CPP
        );
        assert!(!command.send_group_destroyed);
        assert!(command.send_group_uninvite);
        assert!(command.refresh_visible_gameobjects_or_spellclicks);

        let leader_update = leader_rx.try_recv().expect("leader party update");
        assert_eq!(
            u16::from_le_bytes([leader_update[0], leader_update[1]]),
            ServerOpcodes::PartyUpdate as u16
        );
        let remaining_update = remaining_rx.try_recv().expect("remaining party update");
        assert_eq!(
            u16::from_le_bytes([remaining_update[0], remaining_update[1]]),
            ServerOpcodes::PartyUpdate as u16
        );
    }

    #[tokio::test]
    async fn party_uninvite_non_leader_rejects_with_cpp_result() {
        let (mut session, send_rx) = make_session_with_send();
        let leader = ObjectGuid::create_player(1, 42);
        let sender = ObjectGuid::create_player(1, 77);
        let target = ObjectGuid::create_player(1, 88);
        let player_registry = Arc::new(PlayerRegistry::default());
        let group_registry = Arc::new(GroupRegistry::default());
        let mut group = GroupInfo::new(leader);
        group.add_member(sender);
        group.add_member(target);
        let group_guid = group.group_guid;
        group_registry.insert(group_guid, group);

        session.set_player_guid(Some(sender));
        session.group_guid = Some(group_guid);
        session.set_player_registry(player_registry);
        session.set_group_registry(group_registry, Arc::new(PendingInvites::default()));

        session
            .handle_party_uninvite(party_uninvite_packet(target, None, "bye"))
            .await;

        let result = send_rx.try_recv().expect("party command result");
        assert_eq!(
            u16::from_le_bytes([result[0], result[1]]),
            ServerOpcodes::PartyCommandResult as u16
        );
        let mut payload = WorldPacket::from_bytes(&result[2..]);
        let name_len = payload.read_bits(9).unwrap();
        let command = payload.read_bits(4).unwrap();
        let result_code = payload.read_bits(6).unwrap();

        assert_eq!(name_len, 0);
        assert_eq!(command, 1); // C++ PARTY_OP_UNINVITE
        assert_eq!(result_code as u8, party_result::NOT_LEADER_LIKE_CPP);
        assert!(send_rx.try_recv().is_err());
    }

    #[test]
    fn party_member_full_state_carries_phase_states_like_cpp() {
        let leader = ObjectGuid::create_player(1, 42);
        let member = ObjectGuid::create_player(1, 77);
        let (leader_tx, leader_rx) = bounded(8);
        let (member_tx, _member_rx) = bounded(8);
        let registry = PlayerRegistry::default();
        registry.insert(leader, broadcast_info(leader, leader_tx));
        registry.insert(member, broadcast_info(member, member_tx));
        if let Some(mut info) = registry.get_mut(&member) {
            info.party_member_phase_states = wow_packet::packets::party::PartyMemberPhaseStates {
                phase_shift_flags: 0x08,
                personal_guid: ObjectGuid::EMPTY,
                phases: vec![wow_packet::packets::party::PartyMemberPhase {
                    flags: 0x02,
                    id: 20,
                }],
            };
        }
        let mut group = GroupInfo::new(leader);
        group.members.push(member);

        send_party_update(&group, &registry, 0);

        let _party_update = leader_rx.try_recv().unwrap();
        let full_state = leader_rx.try_recv().unwrap();
        assert_eq!(
            u16::from_le_bytes([full_state[0], full_state[1]]),
            ServerOpcodes::PartyMemberFullState as u16
        );
        let phase_bytes = [
            0x08, 0x00, 0x00, 0x00, // PhaseShiftFlags
            0x01, 0x00, 0x00, 0x00, // List.Count
            0x00, 0x00, // PersonalGUID packed mask + empty payload
            0x02, 0x00, 0x00, 0x00, // phase.Flags
            0x14, 0x00, // phase.Id
        ];
        assert!(
            full_state
                .windows(phase_bytes.len())
                .any(|window| window == phase_bytes)
        );
    }

    #[test]
    fn ready_check_start_gate_allows_leader_or_assistant_only_like_cpp() {
        let leader = ObjectGuid::create_player(1, 42);
        let assistant = ObjectGuid::create_player(1, 43);
        let member = ObjectGuid::create_player(1, 44);
        let mut group = GroupInfo::new(leader);
        group.add_member(assistant);
        group.add_member(member);
        group.convert_to_raid_like_cpp();
        group
            .set_assistant_leader_flag_like_cpp(assistant, true)
            .unwrap();

        assert!(sender_can_start_ready_check_like_cpp(&group, leader));
        assert!(sender_can_start_ready_check_like_cpp(&group, assistant));
        assert!(!sender_can_start_ready_check_like_cpp(&group, member));
    }

    #[test]
    fn ready_check_response_dispatch_metadata_matches_cpp() {
        let entry = inventory::iter::<PacketHandlerEntry>
            .into_iter()
            .find(|entry| entry.opcode == ClientOpcodes::ReadyCheckResponse)
            .expect("ReadyCheckResponse handler entry");

        assert_eq!(entry.status, SessionStatus::LoggedIn);
        assert_eq!(entry.processing, PacketProcessing::Inplace);
        assert_eq!(entry.handler_name, "handle_ready_check_response");
    }

    #[test]
    fn set_party_leader_dispatch_metadata_matches_cpp() {
        let entry = inventory::iter::<PacketHandlerEntry>
            .into_iter()
            .find(|entry| entry.opcode == ClientOpcodes::SetPartyLeader)
            .expect("SetPartyLeader handler entry");

        assert_eq!(entry.status, SessionStatus::LoggedIn);
        assert_eq!(entry.processing, PacketProcessing::Inplace);
        assert_eq!(entry.handler_name, "handle_set_party_leader");
    }

    #[test]
    fn ready_check_fanout_sends_events_only_to_connected_members_like_cpp() {
        let leader = ObjectGuid::create_player(1, 42);
        let member = ObjectGuid::create_player(1, 43);
        let offline = ObjectGuid::create_player(1, 44);
        let mut group = GroupInfo::new(leader);
        group.add_member(member);
        group.add_member(offline);

        let registry = PlayerRegistry::default();
        let (leader_tx, leader_rx) = bounded(8);
        let (member_tx, member_rx) = bounded(8);
        registry.insert(leader, broadcast_info(leader, leader_tx));
        registry.insert(member, broadcast_info(member, member_tx));

        let events = vec![
            ReadyCheckEventLikeCpp::Response {
                party_guid: group.group_guid,
                player: offline,
                is_ready: false,
            },
            ReadyCheckEventLikeCpp::Started {
                party_index: GROUP_CATEGORY_HOME_LIKE_CPP,
                party_guid: group.group_guid,
                initiator_guid: leader,
                duration_ms: 35_000,
            },
        ];

        send_ready_check_events_like_cpp(&events, &group, &registry);

        let leader_first = leader_rx.recv().unwrap();
        let leader_second = leader_rx.recv().unwrap();
        let member_first = member_rx.recv().unwrap();
        let member_second = member_rx.recv().unwrap();
        assert_eq!(
            u16::from_le_bytes([leader_first[0], leader_first[1]]),
            ServerOpcodes::ReadyCheckResponse as u16
        );
        assert_eq!(
            u16::from_le_bytes([leader_second[0], leader_second[1]]),
            ServerOpcodes::ReadyCheckStarted as u16
        );
        assert_eq!(leader_first, member_first);
        assert_eq!(leader_second, member_second);
        assert!(leader_rx.try_recv().is_err());
        assert!(member_rx.try_recv().is_err());
    }

    #[test]
    fn group_party_update_member_info_uses_loaded_member_slot_like_cpp() {
        let leader = ObjectGuid::create_player(1, 42);
        let member = ObjectGuid::create_player(1, 77);
        let mut group = GroupInfo::loaded_from_db_like_cpp(
            900,
            17,
            leader,
            5,
            leader,
            2,
            0,
            1,
            14,
            3,
            ObjectGuid::EMPTY,
        );
        assert!(group.load_member_from_db_like_cpp(
            77,
            0x04,
            3,
            2,
            Some(GroupMemberCharacterLikeCpp {
                name: "LoadedMember".to_string(),
                race: 8,
                class: 9,
            }),
        ));

        let registry = PlayerRegistry::default();
        let (tx, _rx) = bounded(1);
        registry.insert(member, broadcast_info(member, tx));
        if let Some(mut entry) = registry.get_mut(&member) {
            entry.player_name.clear();
            entry.race = 0;
            entry.class = 0;
        }

        let info = party_player_info_like_cpp(&group, &registry, member)
            .expect("connected represented member should produce party info");
        assert_eq!(info.name, "LoadedMember");
        assert_eq!(info.class, 9);
        assert_eq!(info.subgroup, 3);
        assert_eq!(info.flags, 0x04);
        assert_eq!(info.roles_assigned, 2);
        assert_eq!(info.faction_group, 2);
    }

    #[tokio::test]
    async fn raid_target_list_request_sends_all_icons_to_caller_without_mutation_like_cpp() {
        let (mut session, send_rx) = make_session_with_send();
        let leader = ObjectGuid::create_player(1, 42);
        let marked = ObjectGuid::create_player(1, 77);
        let group_registry = Arc::new(GroupRegistry::default());
        let mut group = GroupInfo::new(leader);
        group.target_icons[2] = marked.to_raw_bytes();
        let original_icons = group.target_icons;
        let group_guid = group.group_guid;
        group_registry.insert(group_guid, group);

        session.set_player_guid(Some(leader));
        session.group_guid = Some(group_guid);
        session.set_group_registry(group_registry.clone(), Arc::new(PendingInvites::default()));

        session
            .handle_update_raid_target(update_raid_target_packet(ObjectGuid::EMPTY, -1, None))
            .await;

        let sent = send_rx.try_recv().expect("target icon list to caller");
        let mut pkt = WorldPacket::from_bytes(&sent);
        assert_eq!(
            pkt.read_uint16().unwrap(),
            ServerOpcodes::SendRaidTargetUpdateAll as u16
        );
        assert_eq!(pkt.read_uint8().unwrap(), 0);
        assert_eq!(pkt.read_uint32().unwrap(), 8);
        for symbol in 0..8 {
            let target = pkt.read_packed_guid().unwrap();
            assert_eq!(pkt.read_uint8().unwrap(), symbol);
            if symbol == 2 {
                assert_eq!(target, marked);
            }
        }
        assert_eq!(
            group_registry.get(&group_guid).unwrap().target_icons,
            original_icons
        );
        assert!(send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn raid_target_symbol_out_of_range_does_not_mutate_or_fanout_like_cpp() {
        let (mut session, _send_rx) = make_session_with_send();
        let leader = ObjectGuid::create_player(1, 42);
        let target = ObjectGuid::create_player(1, 77);
        let group_registry = Arc::new(GroupRegistry::default());
        let group = GroupInfo::new(leader);
        let original_icons = group.target_icons;
        let group_guid = group.group_guid;
        group_registry.insert(group_guid, group);
        let player_registry = Arc::new(PlayerRegistry::default());
        let (leader_tx, leader_rx) = bounded(8);
        let (target_tx, _target_rx) = bounded(8);
        player_registry.insert(leader, broadcast_info(leader, leader_tx));
        player_registry.insert(target, broadcast_info(target, target_tx));

        session.set_player_guid(Some(leader));
        session.group_guid = Some(group_guid);
        session.set_player_registry(player_registry);
        session.set_group_registry(group_registry.clone(), Arc::new(PendingInvites::default()));

        session
            .handle_update_raid_target(update_raid_target_packet(target, 8, None))
            .await;

        assert_eq!(
            group_registry.get(&group_guid).unwrap().target_icons,
            original_icons
        );
        assert!(leader_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn raid_target_non_raid_regular_member_can_set_icon_like_cpp() {
        let (mut session, _send_rx) = make_session_with_send();
        let leader = ObjectGuid::create_player(1, 42);
        let member = ObjectGuid::create_player(1, 43);
        let target = ObjectGuid::create_player(1, 77);
        let group_registry = Arc::new(GroupRegistry::default());
        let mut group = GroupInfo::new(leader);
        group.add_member(member);
        let group_guid = group.group_guid;
        group_registry.insert(group_guid, group);
        let player_registry = Arc::new(PlayerRegistry::default());
        let (leader_tx, leader_rx) = bounded(8);
        let (member_tx, member_rx) = bounded(8);
        let (target_tx, _target_rx) = bounded(8);
        player_registry.insert(leader, broadcast_info(leader, leader_tx));
        player_registry.insert(member, broadcast_info(member, member_tx));
        player_registry.insert(target, broadcast_info(target, target_tx));

        session.set_player_guid(Some(member));
        session.group_guid = Some(group_guid);
        session.set_player_registry(player_registry);
        session.set_group_registry(group_registry.clone(), Arc::new(PendingInvites::default()));

        session
            .handle_update_raid_target(update_raid_target_packet(target, 3, None))
            .await;

        assert_eq!(
            group_registry.get(&group_guid).unwrap().target_icons[3],
            target.to_raw_bytes()
        );
        let leader_sent = leader_rx.try_recv().expect("leader raid target fanout");
        let member_sent = member_rx.try_recv().expect("member raid target fanout");
        assert_eq!(leader_sent, member_sent);
        let mut pkt = WorldPacket::from_bytes(&leader_sent);
        assert_eq!(
            pkt.read_uint16().unwrap(),
            ServerOpcodes::SendRaidTargetUpdateSingle as u16
        );
        assert_eq!(pkt.read_uint8().unwrap(), 0);
        assert_eq!(pkt.read_uint8().unwrap(), 3);
        assert_eq!(pkt.read_packed_guid().unwrap(), target);
        assert_eq!(pkt.read_packed_guid().unwrap(), member);
    }

    #[tokio::test]
    async fn raid_target_raid_regular_member_rejected_but_assistant_allowed_like_cpp() {
        let (mut session, _send_rx) = make_session_with_send();
        let leader = ObjectGuid::create_player(1, 42);
        let assistant = ObjectGuid::create_player(1, 43);
        let target = ObjectGuid::create_player(1, 77);
        let group_registry = Arc::new(GroupRegistry::default());
        let mut group = GroupInfo::new(leader);
        group.add_member(assistant);
        group.convert_to_raid_like_cpp();
        let group_guid = group.group_guid;
        group_registry.insert(group_guid, group);
        let player_registry = Arc::new(PlayerRegistry::default());
        let (leader_tx, leader_rx) = bounded(8);
        let (assistant_tx, assistant_rx) = bounded(8);
        let (target_tx, _target_rx) = bounded(8);
        player_registry.insert(leader, broadcast_info(leader, leader_tx));
        player_registry.insert(assistant, broadcast_info(assistant, assistant_tx));
        player_registry.insert(target, broadcast_info(target, target_tx));

        session.set_player_guid(Some(assistant));
        session.group_guid = Some(group_guid);
        session.set_player_registry(Arc::clone(&player_registry));
        session.set_group_registry(group_registry.clone(), Arc::new(PendingInvites::default()));

        session
            .handle_update_raid_target(update_raid_target_packet(target, 4, None))
            .await;
        assert_eq!(
            group_registry.get(&group_guid).unwrap().target_icons[4],
            wow_network::EMPTY_TARGET_ICON_RAW_LIKE_CPP
        );
        assert!(leader_rx.try_recv().is_err());
        assert!(assistant_rx.try_recv().is_err());

        group_registry
            .get_mut(&group_guid)
            .unwrap()
            .set_assistant_leader_flag_like_cpp(assistant, true)
            .unwrap();
        session
            .handle_update_raid_target(update_raid_target_packet(target, 4, None))
            .await;
        assert_eq!(
            group_registry.get(&group_guid).unwrap().target_icons[4],
            target.to_raw_bytes()
        );
        assert!(leader_rx.try_recv().is_ok());
        assert!(assistant_rx.try_recv().is_ok());
    }

    #[tokio::test]
    async fn raid_target_duplicate_target_clears_old_icon_before_final_update_like_cpp() {
        let (mut session, _send_rx) = make_session_with_send();
        let leader = ObjectGuid::create_player(1, 42);
        let target = ObjectGuid::create_player(1, 77);
        let group_registry = Arc::new(GroupRegistry::default());
        let mut group = GroupInfo::new(leader);
        group.convert_to_raid_like_cpp();
        group.target_icons[1] = target.to_raw_bytes();
        let group_guid = group.group_guid;
        group_registry.insert(group_guid, group);
        let player_registry = Arc::new(PlayerRegistry::default());
        let (leader_tx, leader_rx) = bounded(8);
        let (target_tx, _target_rx) = bounded(8);
        player_registry.insert(leader, broadcast_info(leader, leader_tx));
        player_registry.insert(target, broadcast_info(target, target_tx));

        session.set_player_guid(Some(leader));
        session.group_guid = Some(group_guid);
        session.set_player_registry(player_registry);
        session.set_group_registry(group_registry.clone(), Arc::new(PendingInvites::default()));

        session
            .handle_update_raid_target(update_raid_target_packet(target, 5, None))
            .await;

        let first = leader_rx.try_recv().expect("clear old icon update");
        let mut first_pkt = WorldPacket::from_bytes(&first);
        assert_eq!(
            first_pkt.read_uint16().unwrap(),
            ServerOpcodes::SendRaidTargetUpdateSingle as u16
        );
        assert_eq!(first_pkt.read_uint8().unwrap(), 0);
        assert_eq!(first_pkt.read_uint8().unwrap(), 1);
        assert_eq!(first_pkt.read_packed_guid().unwrap(), ObjectGuid::EMPTY);
        assert_eq!(first_pkt.read_packed_guid().unwrap(), leader);
        let second = leader_rx.try_recv().expect("set new icon update");
        let mut second_pkt = WorldPacket::from_bytes(&second);
        assert_eq!(
            second_pkt.read_uint16().unwrap(),
            ServerOpcodes::SendRaidTargetUpdateSingle as u16
        );
        assert_eq!(second_pkt.read_uint8().unwrap(), 0);
        assert_eq!(second_pkt.read_uint8().unwrap(), 5);
        assert_eq!(second_pkt.read_packed_guid().unwrap(), target);
        let group = group_registry.get(&group_guid).unwrap();
        assert_eq!(
            group.target_icons[1],
            wow_network::EMPTY_TARGET_ICON_RAW_LIKE_CPP
        );
        assert_eq!(group.target_icons[5], target.to_raw_bytes());
    }

    #[tokio::test]
    async fn raid_target_party_index_instance_does_not_fall_back_to_home_like_cpp() {
        let (mut session, _send_rx) = make_session_with_send();
        let leader = ObjectGuid::create_player(1, 42);
        let target = ObjectGuid::create_player(1, 77);
        let group_registry = Arc::new(GroupRegistry::default());
        let group = GroupInfo::new(leader);
        let group_guid = group.group_guid;
        group_registry.insert(group_guid, group);
        let player_registry = Arc::new(PlayerRegistry::default());
        let (leader_tx, leader_rx) = bounded(8);
        let (target_tx, _target_rx) = bounded(8);
        player_registry.insert(leader, broadcast_info(leader, leader_tx));
        player_registry.insert(target, broadcast_info(target, target_tx));

        session.set_player_guid(Some(leader));
        session.group_guid = Some(group_guid);
        session.set_player_registry(player_registry);
        session.set_group_registry(group_registry.clone(), Arc::new(PendingInvites::default()));

        session
            .handle_update_raid_target(update_raid_target_packet(target, 2, Some(1)))
            .await;

        assert_eq!(
            group_registry.get(&group_guid).unwrap().target_icons[2],
            wow_network::EMPTY_TARGET_ICON_RAW_LIKE_CPP
        );
        assert!(leader_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn party_join_updates_sends_target_list_and_empty_raid_markers_like_cpp() {
        let (mut session, send_rx) = make_session_with_send();
        let leader = ObjectGuid::create_player(1, 42);
        let marked = ObjectGuid::create_player(1, 77);
        let group_registry = Arc::new(GroupRegistry::default());
        let mut group = GroupInfo::new(leader);
        group.target_icons[6] = marked.to_raw_bytes();
        let group_guid = group.group_guid;
        group_registry.insert(group_guid, group);

        session.set_player_guid(Some(leader));
        session.group_guid = Some(group_guid);
        session.set_group_registry(group_registry, Arc::new(PendingInvites::default()));

        session
            .handle_request_party_join_updates(request_party_join_updates_packet(Some(0)))
            .await;

        let target_list = send_rx.try_recv().expect("target list");
        let mut pkt = WorldPacket::from_bytes(&target_list);
        assert_eq!(
            pkt.read_uint16().unwrap(),
            ServerOpcodes::SendRaidTargetUpdateAll as u16
        );
        assert_eq!(pkt.read_uint8().unwrap(), 0);
        assert_eq!(pkt.read_uint32().unwrap(), 8);
        for symbol in 0..8 {
            let target = pkt.read_packed_guid().unwrap();
            assert_eq!(pkt.read_uint8().unwrap(), symbol);
            if symbol == 6 {
                assert_eq!(target, marked);
            }
        }
        let markers = send_rx.try_recv().expect("empty raid markers");
        assert_eq!(
            u16::from_le_bytes([markers[0], markers[1]]),
            ServerOpcodes::RaidMarkersChanged as u16
        );
        assert_eq!(&markers[2..], &[0, 0, 0, 0, 0, 0]);
        assert!(send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn request_party_member_stats_offline_replies_only_to_requester_like_cpp() {
        let (mut session, send_rx) = make_session_with_send();
        let target = ObjectGuid::create_player(1, 77);
        let registry = Arc::new(PlayerRegistry::default());
        let (_target_tx, target_rx) = bounded::<Vec<u8>>(4);

        session.set_player_registry(registry);

        session
            .handle_request_party_member_stats(request_party_member_stats_packet(target, Some(0)))
            .await;

        let sent = send_rx.try_recv().expect("requester full state");
        let mut target_guid_pkt = WorldPacket::new_empty();
        target_guid_pkt.write_packed_guid(&target);
        let target_guid_bytes = target_guid_pkt.into_data();
        let mut pkt = WorldPacket::from_bytes(&sent);
        assert_eq!(
            pkt.read_uint16().unwrap(),
            ServerOpcodes::PartyMemberFullState as u16
        );
        assert!(!pkt.read_bit().unwrap());
        assert_eq!(pkt.read_uint8().unwrap(), 0);
        assert_eq!(pkt.read_uint8().unwrap(), 0);
        assert_eq!(pkt.read_int16().unwrap(), 0);
        assert_eq!(pkt.read_uint8().unwrap(), 0);
        assert_eq!(pkt.read_int16().unwrap(), 0);
        assert_eq!(pkt.read_int32().unwrap(), 0);
        assert_eq!(pkt.read_int32().unwrap(), 0);
        assert_eq!(pkt.read_uint16().unwrap(), 0);
        assert_eq!(pkt.read_uint16().unwrap(), 0);
        assert_eq!(pkt.read_uint16().unwrap(), 0);
        assert_eq!(pkt.read_uint16().unwrap(), 0);
        assert_eq!(pkt.read_uint16().unwrap(), 0);
        assert_eq!(pkt.read_uint16().unwrap(), 0);
        assert_eq!(pkt.read_uint32().unwrap(), 0);
        assert_eq!(pkt.read_int16().unwrap(), 0);
        assert_eq!(pkt.read_int16().unwrap(), 0);
        assert_eq!(pkt.read_int16().unwrap(), 0);
        assert_eq!(pkt.read_int32().unwrap(), 0);
        assert_eq!(pkt.read_int32().unwrap(), 0);
        assert_eq!(pkt.read_uint32().unwrap(), 0);
        assert_eq!(pkt.read_uint32().unwrap(), 0);
        assert_eq!(pkt.read_packed_guid().unwrap(), ObjectGuid::EMPTY);
        assert_eq!(pkt.read_uint32().unwrap(), 0);
        assert_eq!(pkt.read_uint32().unwrap(), 0);
        assert_eq!(pkt.read_int32().unwrap(), 0);
        assert_eq!(pkt.read_uint32().unwrap(), 0);
        assert!(!pkt.read_bit().unwrap());
        assert!(sent.ends_with(&target_guid_bytes));
        assert!(send_rx.try_recv().is_err());
        assert!(target_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn request_party_member_stats_online_replies_snapshot_without_fanout_like_cpp() {
        let (mut session, send_rx) = make_session_with_send();
        let target = ObjectGuid::create_player(1, 78);
        let (target_tx, target_rx) = bounded::<Vec<u8>>(4);
        let registry = Arc::new(PlayerRegistry::default());
        registry.insert(target, broadcast_info(target, target_tx));
        if let Some(mut info) = registry.get_mut(&target) {
            info.level = 80;
            info.class = 4;
            info.current_health = 77;
            info.max_health = 123;
            info.power_type = 3;
            info.current_power = 42;
            info.max_power = 100;
            info.is_pvp = true;
            info.is_ffa_pvp = true;
            info.is_afk = true;
            info.is_dnd = true;
            info.in_vehicle = true;
            info.party_member_vehicle_seat = 1001;
            info.zone_id = 618;
            info.spec_id = 260;
            info.position = Position::new(11.0, 22.0, 33.0, 0.0);
            info.party_member_party_type = [1, 0];
            info.party_member_phase_states = wow_packet::packets::party::PartyMemberPhaseStates {
                phase_shift_flags: 0x08,
                personal_guid: ObjectGuid::EMPTY,
                phases: vec![wow_packet::packets::party::PartyMemberPhase {
                    flags: 0x02,
                    id: 20,
                }],
            };
            info.party_member_auras = vec![wow_packet::packets::party::PartyMemberAuraState {
                spell_id: 12_345,
                flags: 0x21,
                active_flags: 0x04,
                points: vec![17.5],
            }];
            info.party_member_pet_stats = Some(wow_packet::packets::party::PartyMemberPetStats {
                guid: ObjectGuid::create_world_object(
                    wow_core::guid::HighGuid::Pet,
                    0,
                    1,
                    571,
                    0,
                    42_000,
                    100,
                ),
                model_id: 987,
                current_health: 55,
                max_health: 66,
                auras: Vec::new(),
                name: "Wolf".to_string(),
            });
        }
        session.set_player_registry(registry);

        session
            .handle_request_party_member_stats(request_party_member_stats_packet(target, None))
            .await;

        let sent = send_rx.try_recv().expect("requester full state");
        let mut target_guid_pkt = WorldPacket::new_empty();
        target_guid_pkt.write_packed_guid(&target);
        let target_guid_bytes = target_guid_pkt.into_data();
        let mut pkt = WorldPacket::from_bytes(&sent);
        assert_eq!(
            pkt.read_uint16().unwrap(),
            ServerOpcodes::PartyMemberFullState as u16
        );
        assert!(!pkt.read_bit().unwrap());
        assert_eq!(pkt.read_uint8().unwrap(), 1);
        assert_eq!(pkt.read_uint8().unwrap(), 0);
        assert_eq!(
            pkt.read_int16().unwrap(),
            0x0001 | 0x0002 | 0x0010 | 0x0040 | 0x0080 | 0x0200
        );
        assert_eq!(pkt.read_uint8().unwrap(), 3);
        assert_eq!(pkt.read_int16().unwrap(), 0);
        assert_eq!(pkt.read_int32().unwrap(), 77);
        assert_eq!(pkt.read_int32().unwrap(), 123);
        assert_eq!(pkt.read_uint16().unwrap(), 42);
        assert_eq!(pkt.read_uint16().unwrap(), 100);
        assert_eq!(pkt.read_uint16().unwrap(), 80);
        assert_eq!(pkt.read_uint16().unwrap(), 260);
        assert_eq!(pkt.read_uint16().unwrap(), 618);
        assert_eq!(pkt.read_uint16().unwrap(), 0);
        assert_eq!(pkt.read_uint32().unwrap(), 0);
        assert_eq!(pkt.read_int16().unwrap(), 11);
        assert_eq!(pkt.read_int16().unwrap(), 22);
        assert_eq!(pkt.read_int16().unwrap(), 33);
        assert_eq!(pkt.read_int32().unwrap(), 1001);
        assert_eq!(pkt.read_uint32().unwrap(), 1);
        assert_eq!(pkt.read_uint32().unwrap(), 0x08);
        assert_eq!(pkt.read_uint32().unwrap(), 1);
        assert_eq!(pkt.read_packed_guid().unwrap(), ObjectGuid::EMPTY);
        assert_eq!(pkt.read_uint32().unwrap(), 0x02);
        assert_eq!(pkt.read_uint16().unwrap(), 20);
        assert_eq!(pkt.read_uint32().unwrap(), 0);
        assert_eq!(pkt.read_int32().unwrap(), 0);
        assert_eq!(pkt.read_uint32().unwrap(), 0);
        assert_eq!(pkt.read_int32().unwrap(), 12_345);
        assert_eq!(pkt.read_uint16().unwrap(), 0x21);
        assert_eq!(pkt.read_uint32().unwrap(), 0x04);
        assert_eq!(pkt.read_int32().unwrap(), 1);
        assert_eq!(pkt.read_float().unwrap(), 17.5);
        assert!(pkt.read_bit().unwrap());
        assert_eq!(pkt.read_float().unwrap(), 0.0);
        assert_eq!(pkt.read_float().unwrap(), 0.0);
        assert_eq!(pkt.read_int32().unwrap(), 0);
        let pet_guid = pkt.read_packed_guid().unwrap();
        assert_eq!(pet_guid.high_type(), wow_core::guid::HighGuid::Pet);
        assert_eq!(pkt.read_int32().unwrap(), 987);
        assert_eq!(pkt.read_int32().unwrap(), 55);
        assert_eq!(pkt.read_int32().unwrap(), 66);
        assert_eq!(pkt.read_uint32().unwrap(), 0);
        let pet_name_len = pkt.read_bits(8).unwrap() as usize;
        assert_eq!(pkt.read_string(pet_name_len).unwrap(), "Wolf");
        assert_eq!(pkt.read_packed_guid().unwrap(), target);
        assert!(sent.ends_with(&target_guid_bytes));
        assert!(
            sent.windows([0x08, 0x00, 0x00, 0x00].len())
                .any(|window| window == [0x08, 0x00, 0x00, 0x00])
        );
        assert!(send_rx.try_recv().is_err());
        assert!(target_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn set_role_without_group_sends_only_caller_and_idempotent_zero_returns_like_cpp() {
        let (mut session, send_rx) = make_session_with_send();
        let sender = ObjectGuid::create_player(1, 42);
        let target = ObjectGuid::create_player(1, 43);
        session.set_player_guid(Some(sender));

        session
            .handle_set_role(set_role_packet(target, 0, None))
            .await;
        assert!(send_rx.try_recv().is_err());

        session
            .handle_set_role(set_role_packet(target, 4, Some(0)))
            .await;

        let sent = send_rx.try_recv().expect("caller role changed inform");
        let mut pkt = WorldPacket::from_bytes(&sent);
        assert_eq!(
            pkt.read_uint16().unwrap(),
            ServerOpcodes::RoleChangedInform as u16
        );
        assert_eq!(pkt.read_uint8().unwrap(), 0);
        assert_eq!(pkt.read_packed_guid().unwrap(), sender);
        assert_eq!(pkt.read_packed_guid().unwrap(), target);
        assert_eq!(pkt.read_uint8().unwrap(), 0);
        assert_eq!(pkt.read_uint8().unwrap(), 4);
        assert!(send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn set_role_group_broadcasts_old_new_and_updates_existing_target_like_cpp() {
        let (mut session, _send_rx) = make_session_with_send();
        let leader = ObjectGuid::create_player(1, 42);
        let member = ObjectGuid::create_player(1, 43);
        let group_registry = Arc::new(GroupRegistry::default());
        let mut group = GroupInfo::new(leader);
        group.add_member(member);
        group.set_lfg_roles_like_cpp(member, 1);
        let group_guid = group.group_guid;
        group_registry.insert(group_guid, group);

        let player_registry = Arc::new(PlayerRegistry::default());
        let (leader_tx, leader_rx) = bounded(8);
        let (member_tx, member_rx) = bounded(8);
        player_registry.insert(leader, broadcast_info(leader, leader_tx));
        player_registry.insert(member, broadcast_info(member, member_tx));

        session.set_player_guid(Some(leader));
        session.group_guid = Some(group_guid);
        session.set_player_registry(player_registry);
        session.set_group_registry(group_registry.clone(), Arc::new(PendingInvites::default()));

        session
            .handle_set_role(set_role_packet(member, 4, None))
            .await;

        let leader_sent = leader_rx.try_recv().expect("leader fanout");
        let member_sent = member_rx.try_recv().expect("member fanout");
        assert_eq!(leader_sent, member_sent);
        let mut pkt = WorldPacket::from_bytes(&leader_sent);
        assert_eq!(
            pkt.read_uint16().unwrap(),
            ServerOpcodes::RoleChangedInform as u16
        );
        assert_eq!(pkt.read_uint8().unwrap(), 0);
        assert_eq!(pkt.read_packed_guid().unwrap(), leader);
        assert_eq!(pkt.read_packed_guid().unwrap(), member);
        assert_eq!(pkt.read_uint8().unwrap(), 1);
        assert_eq!(pkt.read_uint8().unwrap(), 4);

        let leader_update = leader_rx
            .try_recv()
            .expect("leader PartyUpdate after SetLfgRoles");
        let member_update = member_rx
            .try_recv()
            .expect("member PartyUpdate after SetLfgRoles");
        let mut leader_update_pkt = WorldPacket::from_bytes(&leader_update);
        let mut member_update_pkt = WorldPacket::from_bytes(&member_update);
        assert_eq!(
            leader_update_pkt.read_uint16().unwrap(),
            ServerOpcodes::PartyUpdate as u16
        );
        assert_eq!(
            member_update_pkt.read_uint16().unwrap(),
            ServerOpcodes::PartyUpdate as u16
        );
        assert_eq!(
            group_registry
                .get(&group_guid)
                .unwrap()
                .get_lfg_roles_like_cpp(member),
            4
        );
    }

    #[tokio::test]
    async fn set_role_group_old_equal_returns_without_packet_or_mutation_like_cpp() {
        let (mut session, _send_rx) = make_session_with_send();
        let leader = ObjectGuid::create_player(1, 42);
        let member = ObjectGuid::create_player(1, 43);
        let group_registry = Arc::new(GroupRegistry::default());
        let mut group = GroupInfo::new(leader);
        group.add_member(member);
        group.set_lfg_roles_like_cpp(member, 2);
        let group_guid = group.group_guid;
        group_registry.insert(group_guid, group);
        let player_registry = Arc::new(PlayerRegistry::default());
        let (member_tx, member_rx) = bounded(8);
        player_registry.insert(member, broadcast_info(member, member_tx));

        session.set_player_guid(Some(leader));
        session.group_guid = Some(group_guid);
        session.set_player_registry(player_registry);
        session.set_group_registry(group_registry.clone(), Arc::new(PendingInvites::default()));

        session
            .handle_set_role(set_role_packet(member, 2, None))
            .await;

        assert_eq!(
            group_registry
                .get(&group_guid)
                .unwrap()
                .get_lfg_roles_like_cpp(member),
            2
        );
        assert!(member_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn set_role_absent_target_broadcasts_but_does_not_mutate_like_cpp() {
        let (mut session, _send_rx) = make_session_with_send();
        let leader = ObjectGuid::create_player(1, 42);
        let absent = ObjectGuid::create_player(1, 99);
        let group_registry = Arc::new(GroupRegistry::default());
        let group = GroupInfo::new(leader);
        let group_guid = group.group_guid;
        group_registry.insert(group_guid, group);
        let player_registry = Arc::new(PlayerRegistry::default());
        let (leader_tx, leader_rx) = bounded(8);
        player_registry.insert(leader, broadcast_info(leader, leader_tx));

        session.set_player_guid(Some(leader));
        session.group_guid = Some(group_guid);
        session.set_player_registry(player_registry);
        session.set_group_registry(group_registry.clone(), Arc::new(PendingInvites::default()));

        session
            .handle_set_role(set_role_packet(absent, 4, None))
            .await;

        let sent = leader_rx.try_recv().expect("broadcast for absent target");
        let mut pkt = WorldPacket::from_bytes(&sent);
        assert_eq!(
            pkt.read_uint16().unwrap(),
            ServerOpcodes::RoleChangedInform as u16
        );
        assert_eq!(pkt.read_uint8().unwrap(), 0);
        assert_eq!(pkt.read_packed_guid().unwrap(), leader);
        assert_eq!(pkt.read_packed_guid().unwrap(), absent);
        assert_eq!(pkt.read_uint8().unwrap(), 0);
        assert_eq!(pkt.read_uint8().unwrap(), 4);
        assert_eq!(
            group_registry
                .get(&group_guid)
                .unwrap()
                .get_lfg_roles_like_cpp(absent),
            0
        );
        assert!(leader_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn initiate_role_poll_rejects_regular_member_without_fanout_like_cpp() {
        let (mut session, _send_rx) = make_session_with_send();
        let leader = ObjectGuid::create_player(1, 42);
        let member = ObjectGuid::create_player(1, 43);
        let group_registry = Arc::new(GroupRegistry::default());
        let mut group = GroupInfo::new(leader);
        group.add_member(member);
        let group_guid = group.group_guid;
        group_registry.insert(group_guid, group);
        let player_registry = Arc::new(PlayerRegistry::default());
        let (leader_tx, leader_rx) = bounded(8);
        player_registry.insert(leader, broadcast_info(leader, leader_tx));

        session.set_player_guid(Some(member));
        session.group_guid = Some(group_guid);
        session.set_player_registry(player_registry);
        session.set_group_registry(group_registry, Arc::new(PendingInvites::default()));

        session
            .handle_initiate_role_poll(initiate_role_poll_packet(None))
            .await;

        assert!(leader_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn initiate_role_poll_allows_leader_and_assistant_and_sends_connected_members_like_cpp() {
        let (mut session, _send_rx) = make_session_with_send();
        let leader = ObjectGuid::create_player(1, 42);
        let assistant = ObjectGuid::create_player(1, 43);
        let offline = ObjectGuid::create_player(1, 44);
        let group_registry = Arc::new(GroupRegistry::default());
        let mut group = GroupInfo::new(leader);
        group.add_member(assistant);
        group.add_member(offline);
        group.convert_to_raid_like_cpp();
        group
            .set_assistant_leader_flag_like_cpp(assistant, true)
            .unwrap();
        let group_guid = group.group_guid;
        group_registry.insert(group_guid, group);
        let player_registry = Arc::new(PlayerRegistry::default());
        let (leader_tx, leader_rx) = bounded(8);
        let (assistant_tx, assistant_rx) = bounded(8);
        player_registry.insert(leader, broadcast_info(leader, leader_tx));
        player_registry.insert(assistant, broadcast_info(assistant, assistant_tx));

        session.set_player_guid(Some(assistant));
        session.group_guid = Some(group_guid);
        session.set_player_registry(player_registry);
        session.set_group_registry(group_registry, Arc::new(PendingInvites::default()));

        session
            .handle_initiate_role_poll(initiate_role_poll_packet(Some(0)))
            .await;

        let leader_sent = leader_rx.try_recv().expect("leader fanout");
        let assistant_sent = assistant_rx.try_recv().expect("assistant fanout");
        assert_eq!(leader_sent, assistant_sent);
        let mut pkt = WorldPacket::from_bytes(&leader_sent);
        assert_eq!(
            pkt.read_uint16().unwrap(),
            ServerOpcodes::RolePollInform as u16
        );
        assert_eq!(pkt.read_int8().unwrap(), 0);
        assert_eq!(pkt.read_packed_guid().unwrap(), assistant);
        assert!(leader_rx.try_recv().is_err());
        assert!(assistant_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn set_loot_method_is_represented_noop_like_this_cpp_branch() {
        let (mut session, send_rx) = make_session_with_send();
        let leader = ObjectGuid::create_player(1, 42);
        let requested_master = ObjectGuid::create_player(1, 77);
        let original_master = ObjectGuid::create_player(1, 88);
        let group_registry = Arc::new(GroupRegistry::default());
        let mut group = GroupInfo::new(leader);
        group.loot_method = 2;
        group.master_looter_guid = original_master;
        let group_guid = group.group_guid;
        group_registry.insert(group_guid, group);
        session.group_guid = Some(group_guid);
        session.set_group_registry(group_registry.clone(), Arc::new(PendingInvites::default()));

        session
            .handle_set_loot_method(set_loot_method_packet(true, 0, requested_master, 4))
            .await;

        let group = group_registry.get(&group_guid).unwrap();
        assert_eq!(group.loot_method, 2);
        assert_eq!(group.master_looter_guid, original_master);
        assert!(send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn convert_raid_sets_flag_and_queues_member_refresh_like_cpp() {
        let (mut session, send_rx) = make_session_with_send();
        let leader = ObjectGuid::create_player(1, 42);
        let member = ObjectGuid::create_player(1, 43);
        let group_registry = Arc::new(GroupRegistry::default());
        let mut group = GroupInfo::new(leader);
        group.add_member(member);
        let group_guid = group.group_guid;
        group_registry.insert(group_guid, group);

        let player_registry = Arc::new(PlayerRegistry::default());
        let (leader_tx, leader_rx) = bounded(8);
        let (member_tx, _member_rx) = bounded(8);
        let (member_command_tx, member_command_rx) = bounded(8);
        player_registry.insert(leader, broadcast_info(leader, leader_tx));
        let mut member_info = broadcast_info(member, member_tx);
        member_info.command_tx = member_command_tx;
        player_registry.insert(member, member_info);

        session.set_player_guid(Some(leader));
        session.group_guid = Some(group_guid);
        session.set_player_registry(Arc::clone(&player_registry));
        session.set_group_registry(group_registry.clone(), Arc::new(PendingInvites::default()));

        session.handle_convert_raid(convert_raid_packet(true)).await;

        assert!(
            group_registry
                .get(&group_guid)
                .is_some_and(|group| group.is_raid_group())
        );
        let mut remote_refresh_queued = false;
        while let Ok(command) = member_command_rx.try_recv() {
            if matches!(
                command,
                SessionCommand::RefreshVisibleGameobjectsOrSpellClicksLikeCpp
            ) {
                remote_refresh_queued = true;
            }
        }
        assert!(
            remote_refresh_queued,
            "remote member visible refresh command queued"
        );
        let command_result = send_rx.try_recv().expect("party command result");
        assert_eq!(
            u16::from_le_bytes([command_result[0], command_result[1]]),
            ServerOpcodes::PartyCommandResult as u16
        );
        assert!(send_rx.try_recv().is_err());
        let party_update = leader_rx.try_recv().expect("leader party update");
        assert_eq!(
            u16::from_le_bytes([party_update[0], party_update[1]]),
            ServerOpcodes::PartyUpdate as u16
        );
        assert_eq!(
            u16::from_le_bytes([party_update[2], party_update[3]]),
            wow_network::GROUP_FLAG_RAID_LIKE_CPP
        );
    }

    #[tokio::test]
    async fn convert_raid_to_group_rejects_over_five_members_like_cpp() {
        let (mut session, _send_rx) = make_session_with_send();
        let leader = ObjectGuid::create_player(1, 42);
        let group_registry = Arc::new(GroupRegistry::default());
        let mut group = GroupInfo::new(leader);
        for counter in 43..48 {
            group.add_member(ObjectGuid::create_player(1, counter));
        }
        group.convert_to_raid_like_cpp();
        let group_guid = group.group_guid;
        group_registry.insert(group_guid, group);

        let player_registry = Arc::new(PlayerRegistry::default());
        session.set_player_guid(Some(leader));
        session.group_guid = Some(group_guid);
        session.set_player_registry(player_registry);
        session.set_group_registry(group_registry.clone(), Arc::new(PendingInvites::default()));

        session
            .handle_convert_raid(convert_raid_packet(false))
            .await;

        assert!(
            group_registry
                .get(&group_guid)
                .is_some_and(|group| group.is_raid_group())
        );
    }

    #[tokio::test]
    async fn change_sub_group_leader_moves_member_and_fans_out_update_like_cpp() {
        let (mut session, send_rx) = make_session_with_send();
        let leader = ObjectGuid::create_player(1, 42);
        let member = ObjectGuid::create_player(1, 43);
        let group_registry = Arc::new(GroupRegistry::default());
        let mut group = GroupInfo::new(leader);
        group.add_member(member);
        group.convert_to_raid_like_cpp();
        let group_guid = group.group_guid;
        group_registry.insert(group_guid, group);

        let player_registry = Arc::new(PlayerRegistry::default());
        let (leader_tx, leader_rx) = bounded(8);
        let (member_tx, member_rx) = bounded(8);
        player_registry.insert(leader, broadcast_info(leader, leader_tx));
        player_registry.insert(member, broadcast_info(member, member_tx));

        session.set_player_guid(Some(leader));
        session.group_guid = Some(group_guid);
        session.set_player_registry(Arc::clone(&player_registry));
        session.set_group_registry(group_registry.clone(), Arc::new(PendingInvites::default()));

        session
            .handle_change_sub_group(change_sub_group_packet(member, 2, Some(0)))
            .await;

        assert!(send_rx.try_recv().is_err());
        let group = group_registry.get(&group_guid).unwrap();
        assert_eq!(group.member_group_like_cpp(member), 2);
        assert!(group.has_free_slot_sub_group_like_cpp(0));
        let leader_update = leader_rx.try_recv().expect("leader party update");
        assert_eq!(
            u16::from_le_bytes([leader_update[0], leader_update[1]]),
            ServerOpcodes::PartyUpdate as u16
        );
        let member_update = member_rx.try_recv().expect("member party update");
        assert_eq!(
            u16::from_le_bytes([member_update[0], member_update[1]]),
            ServerOpcodes::PartyUpdate as u16
        );
    }

    #[tokio::test]
    async fn change_sub_group_assistant_allowed_but_regular_member_rejected_like_cpp() {
        let (mut session, _send_rx) = make_session_with_send();
        let leader = ObjectGuid::create_player(1, 42);
        let assistant = ObjectGuid::create_player(1, 43);
        let target = ObjectGuid::create_player(1, 44);
        let group_registry = Arc::new(GroupRegistry::default());
        let mut group = GroupInfo::new(leader);
        group.add_member(assistant);
        group.add_member(target);
        group.convert_to_raid_like_cpp();
        let group_guid = group.group_guid;
        group_registry.insert(group_guid, group);

        let player_registry = Arc::new(PlayerRegistry::default());
        let (leader_tx, _leader_rx) = bounded(8);
        let (assistant_tx, _assistant_rx) = bounded(8);
        let (target_tx, _target_rx) = bounded(8);
        player_registry.insert(leader, broadcast_info(leader, leader_tx));
        player_registry.insert(assistant, broadcast_info(assistant, assistant_tx));
        player_registry.insert(target, broadcast_info(target, target_tx));

        session.set_player_guid(Some(assistant));
        session.group_guid = Some(group_guid);
        session.set_player_registry(Arc::clone(&player_registry));
        session.set_group_registry(group_registry.clone(), Arc::new(PendingInvites::default()));

        session
            .handle_change_sub_group(change_sub_group_packet(target, 2, None))
            .await;
        assert_eq!(
            group_registry
                .get(&group_guid)
                .unwrap()
                .member_group_like_cpp(target),
            0
        );

        {
            let mut group = group_registry.get_mut(&group_guid).unwrap();
            let slot = group
                .member_slots
                .iter_mut()
                .find(|slot| slot.guid == assistant)
                .unwrap();
            slot.flags |= wow_network::MEMBER_FLAG_ASSISTANT_LIKE_CPP;
        }

        session
            .handle_change_sub_group(change_sub_group_packet(target, 2, None))
            .await;

        assert_eq!(
            group_registry
                .get(&group_guid)
                .unwrap()
                .member_group_like_cpp(target),
            2
        );
    }

    #[tokio::test]
    async fn set_party_assignment_leader_sets_main_tank_and_fans_out_like_cpp() {
        let (mut session, send_rx) = make_session_with_send();
        let leader = ObjectGuid::create_player(1, 42);
        let member = ObjectGuid::create_player(1, 43);
        let group_registry = Arc::new(GroupRegistry::default());
        let mut group = GroupInfo::new(leader);
        group.add_member(member);
        group.convert_to_raid_like_cpp();
        let group_guid = group.group_guid;
        group_registry.insert(group_guid, group);

        let player_registry = Arc::new(PlayerRegistry::default());
        let (leader_tx, leader_rx) = bounded(8);
        let (member_tx, member_rx) = bounded(8);
        player_registry.insert(leader, broadcast_info(leader, leader_tx));
        player_registry.insert(member, broadcast_info(member, member_tx));

        session.set_player_guid(Some(leader));
        session.group_guid = Some(group_guid);
        session.set_player_registry(Arc::clone(&player_registry));
        session.set_group_registry(group_registry.clone(), Arc::new(PendingInvites::default()));

        session
            .handle_set_party_assignment(set_party_assignment_packet(
                wow_network::GROUP_ASSIGN_MAINTANK_LIKE_CPP,
                member,
                true,
                Some(0),
            ))
            .await;

        assert!(send_rx.try_recv().is_err());
        assert_eq!(
            group_registry
                .get(&group_guid)
                .unwrap()
                .member_slot_like_cpp(member)
                .unwrap()
                .flags
                & wow_network::MEMBER_FLAG_MAINTANK_LIKE_CPP,
            wow_network::MEMBER_FLAG_MAINTANK_LIKE_CPP
        );
        let leader_update = leader_rx.try_recv().expect("leader party update");
        assert_eq!(
            u16::from_le_bytes([leader_update[0], leader_update[1]]),
            ServerOpcodes::PartyUpdate as u16
        );
        let member_update = member_rx.try_recv().expect("member party update");
        assert_eq!(
            u16::from_le_bytes([member_update[0], member_update[1]]),
            ServerOpcodes::PartyUpdate as u16
        );
    }

    #[tokio::test]
    async fn set_party_assignment_assistant_sets_main_assist_like_cpp() {
        let (mut session, _send_rx) = make_session_with_send();
        let leader = ObjectGuid::create_player(1, 42);
        let assistant = ObjectGuid::create_player(1, 43);
        let target = ObjectGuid::create_player(1, 44);
        let group_registry = Arc::new(GroupRegistry::default());
        let mut group = GroupInfo::new(leader);
        group.add_member(assistant);
        group.add_member(target);
        group.convert_to_raid_like_cpp();
        group
            .set_group_member_flag_like_cpp(
                assistant,
                true,
                wow_network::MEMBER_FLAG_ASSISTANT_LIKE_CPP,
            )
            .unwrap();
        let group_guid = group.group_guid;
        group_registry.insert(group_guid, group);

        let player_registry = Arc::new(PlayerRegistry::default());
        let (leader_tx, leader_rx) = bounded(8);
        let (assistant_tx, _assistant_rx) = bounded(8);
        let (target_tx, _target_rx) = bounded(8);
        player_registry.insert(leader, broadcast_info(leader, leader_tx));
        player_registry.insert(assistant, broadcast_info(assistant, assistant_tx));
        player_registry.insert(target, broadcast_info(target, target_tx));

        session.set_player_guid(Some(assistant));
        session.group_guid = Some(group_guid);
        session.set_player_registry(Arc::clone(&player_registry));
        session.set_group_registry(group_registry.clone(), Arc::new(PendingInvites::default()));

        session
            .handle_set_party_assignment(set_party_assignment_packet(
                wow_network::GROUP_ASSIGN_MAINASSIST_LIKE_CPP,
                target,
                true,
                None,
            ))
            .await;

        assert_eq!(
            group_registry
                .get(&group_guid)
                .unwrap()
                .member_slot_like_cpp(target)
                .unwrap()
                .flags
                & wow_network::MEMBER_FLAG_MAINASSIST_LIKE_CPP,
            wow_network::MEMBER_FLAG_MAINASSIST_LIKE_CPP
        );
        assert!(leader_rx.try_recv().is_ok());
    }

    #[tokio::test]
    async fn set_party_assignment_rejects_regular_member_without_mutation_or_fanout_like_cpp() {
        let (mut session, _send_rx) = make_session_with_send();
        let leader = ObjectGuid::create_player(1, 42);
        let member = ObjectGuid::create_player(1, 43);
        let target = ObjectGuid::create_player(1, 44);
        let group_registry = Arc::new(GroupRegistry::default());
        let mut group = GroupInfo::new(leader);
        group.add_member(member);
        group.add_member(target);
        group.convert_to_raid_like_cpp();
        let group_guid = group.group_guid;
        group_registry.insert(group_guid, group);

        let player_registry = Arc::new(PlayerRegistry::default());
        let (leader_tx, leader_rx) = bounded(8);
        let (member_tx, member_rx) = bounded(8);
        let (target_tx, target_rx) = bounded(8);
        player_registry.insert(leader, broadcast_info(leader, leader_tx));
        player_registry.insert(member, broadcast_info(member, member_tx));
        player_registry.insert(target, broadcast_info(target, target_tx));

        session.set_player_guid(Some(member));
        session.group_guid = Some(group_guid);
        session.set_player_registry(Arc::clone(&player_registry));
        session.set_group_registry(group_registry.clone(), Arc::new(PendingInvites::default()));

        session
            .handle_set_party_assignment(set_party_assignment_packet(
                wow_network::GROUP_ASSIGN_MAINTANK_LIKE_CPP,
                target,
                true,
                None,
            ))
            .await;

        assert_eq!(
            group_registry
                .get(&group_guid)
                .unwrap()
                .member_slot_like_cpp(target)
                .unwrap()
                .flags,
            0
        );
        assert!(leader_rx.try_recv().is_err());
        assert!(member_rx.try_recv().is_err());
        assert!(target_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn set_party_assignment_non_raid_or_missing_target_fans_out_and_missing_clears_unique_like_cpp()
     {
        let (mut session, _send_rx) = make_session_with_send();
        let leader = ObjectGuid::create_player(1, 42);
        let member = ObjectGuid::create_player(1, 43);
        let missing = ObjectGuid::create_player(1, 44);
        let group_registry = Arc::new(GroupRegistry::default());
        let mut group = GroupInfo::new(leader);
        group.add_member(member);
        let group_guid = group.group_guid;
        group_registry.insert(group_guid, group);

        let player_registry = Arc::new(PlayerRegistry::default());
        let (leader_tx, leader_rx) = bounded(8);
        let (member_tx, member_rx) = bounded(8);
        player_registry.insert(leader, broadcast_info(leader, leader_tx));
        player_registry.insert(member, broadcast_info(member, member_tx));

        session.set_player_guid(Some(leader));
        session.group_guid = Some(group_guid);
        session.set_player_registry(Arc::clone(&player_registry));
        session.set_group_registry(group_registry.clone(), Arc::new(PendingInvites::default()));

        session
            .handle_set_party_assignment(set_party_assignment_packet(
                wow_network::GROUP_ASSIGN_MAINTANK_LIKE_CPP,
                member,
                true,
                None,
            ))
            .await;
        assert_eq!(
            group_registry
                .get(&group_guid)
                .unwrap()
                .member_slot_like_cpp(member)
                .unwrap()
                .flags,
            0
        );
        assert!(leader_rx.try_recv().is_ok());
        assert!(member_rx.try_recv().is_ok());

        {
            let mut group = group_registry.get_mut(&group_guid).unwrap();
            group.convert_to_raid_like_cpp();
            group
                .set_group_member_flag_like_cpp(
                    member,
                    true,
                    wow_network::MEMBER_FLAG_MAINTANK_LIKE_CPP,
                )
                .unwrap();
        }
        session
            .handle_set_party_assignment(set_party_assignment_packet(
                wow_network::GROUP_ASSIGN_MAINTANK_LIKE_CPP,
                missing,
                true,
                None,
            ))
            .await;
        assert_eq!(
            group_registry
                .get(&group_guid)
                .unwrap()
                .member_slot_like_cpp(member)
                .unwrap()
                .flags
                & wow_network::MEMBER_FLAG_MAINTANK_LIKE_CPP,
            0
        );
        assert!(leader_rx.try_recv().is_ok());
        assert!(member_rx.try_recv().is_ok());
    }

    #[tokio::test]
    async fn set_party_assignment_unknown_assignment_fans_out_without_mutation_like_cpp() {
        let (mut session, _send_rx) = make_session_with_send();
        let leader = ObjectGuid::create_player(1, 42);
        let member = ObjectGuid::create_player(1, 43);
        let group_registry = Arc::new(GroupRegistry::default());
        let mut group = GroupInfo::new(leader);
        group.add_member(member);
        group.convert_to_raid_like_cpp();
        let sequence_before = group.sequence_num;
        let group_guid = group.group_guid;
        group_registry.insert(group_guid, group);

        let player_registry = Arc::new(PlayerRegistry::default());
        let (leader_tx, leader_rx) = bounded(8);
        let (member_tx, member_rx) = bounded(8);
        player_registry.insert(leader, broadcast_info(leader, leader_tx));
        player_registry.insert(member, broadcast_info(member, member_tx));

        session.set_player_guid(Some(leader));
        session.group_guid = Some(group_guid);
        session.set_player_registry(Arc::clone(&player_registry));
        session.set_group_registry(group_registry.clone(), Arc::new(PendingInvites::default()));

        session
            .handle_set_party_assignment(set_party_assignment_packet(99, member, true, None))
            .await;

        let group = group_registry.get(&group_guid).unwrap();
        assert_eq!(group.sequence_num, sequence_before);
        assert_eq!(group.member_slot_like_cpp(member).unwrap().flags, 0);
        assert!(leader_rx.try_recv().is_ok());
        assert!(member_rx.try_recv().is_ok());
    }

    #[tokio::test]
    async fn set_everyone_is_assistant_leader_applies_to_all_members_and_fans_out_like_cpp() {
        let (mut session, send_rx) = make_session_with_send();
        let leader = ObjectGuid::create_player(1, 42);
        let member = ObjectGuid::create_player(1, 43);
        let group_registry = Arc::new(GroupRegistry::default());
        let mut group = GroupInfo::new(leader);
        group.add_member(member);
        let group_guid = group.group_guid;
        group_registry.insert(group_guid, group);

        let player_registry = Arc::new(PlayerRegistry::default());
        let (leader_tx, leader_rx) = bounded(8);
        let (member_tx, member_rx) = bounded(8);
        player_registry.insert(leader, broadcast_info(leader, leader_tx));
        player_registry.insert(member, broadcast_info(member, member_tx));

        session.set_player_guid(Some(leader));
        session.group_guid = Some(group_guid);
        session.set_player_registry(Arc::clone(&player_registry));
        session.set_group_registry(group_registry.clone(), Arc::new(PendingInvites::default()));

        session
            .handle_set_everyone_is_assistant(set_everyone_is_assistant_packet(true, Some(0)))
            .await;

        assert!(send_rx.try_recv().is_err());
        let group = group_registry.get(&group_guid).unwrap();
        assert_eq!(
            group.group_flags & wow_network::GROUP_FLAG_EVERYONE_ASSISTANT_LIKE_CPP,
            wow_network::GROUP_FLAG_EVERYONE_ASSISTANT_LIKE_CPP
        );
        for guid in [leader, member] {
            assert_eq!(
                group.member_slot_like_cpp(guid).unwrap().flags
                    & wow_network::MEMBER_FLAG_ASSISTANT_LIKE_CPP,
                wow_network::MEMBER_FLAG_ASSISTANT_LIKE_CPP
            );
        }
        let leader_update = leader_rx.try_recv().expect("leader party update");
        assert_eq!(
            u16::from_le_bytes([leader_update[0], leader_update[1]]),
            ServerOpcodes::PartyUpdate as u16
        );
        let member_update = member_rx.try_recv().expect("member party update");
        assert_eq!(
            u16::from_le_bytes([member_update[0], member_update[1]]),
            ServerOpcodes::PartyUpdate as u16
        );
    }

    #[tokio::test]
    async fn set_everyone_is_assistant_leader_clears_all_members_and_fans_out_like_cpp() {
        let (mut session, _send_rx) = make_session_with_send();
        let leader = ObjectGuid::create_player(1, 42);
        let member = ObjectGuid::create_player(1, 43);
        let group_registry = Arc::new(GroupRegistry::default());
        let mut group = GroupInfo::new(leader);
        group.add_member(member);
        group.set_everyone_is_assistant_like_cpp(true);
        let group_guid = group.group_guid;
        group_registry.insert(group_guid, group);

        let player_registry = Arc::new(PlayerRegistry::default());
        let (leader_tx, leader_rx) = bounded(8);
        let (member_tx, member_rx) = bounded(8);
        player_registry.insert(leader, broadcast_info(leader, leader_tx));
        player_registry.insert(member, broadcast_info(member, member_tx));

        session.set_player_guid(Some(leader));
        session.group_guid = Some(group_guid);
        session.set_player_registry(Arc::clone(&player_registry));
        session.set_group_registry(group_registry.clone(), Arc::new(PendingInvites::default()));

        session
            .handle_set_everyone_is_assistant(set_everyone_is_assistant_packet(false, None))
            .await;

        let group = group_registry.get(&group_guid).unwrap();
        assert_eq!(
            group.group_flags & wow_network::GROUP_FLAG_EVERYONE_ASSISTANT_LIKE_CPP,
            0
        );
        for guid in [leader, member] {
            assert_eq!(
                group.member_slot_like_cpp(guid).unwrap().flags
                    & wow_network::MEMBER_FLAG_ASSISTANT_LIKE_CPP,
                0
            );
        }
        assert!(leader_rx.try_recv().is_ok());
        assert!(member_rx.try_recv().is_ok());
    }

    #[tokio::test]
    async fn set_everyone_is_assistant_rejects_non_leader_without_mutation_or_fanout_like_cpp() {
        let (mut session, _send_rx) = make_session_with_send();
        let leader = ObjectGuid::create_player(1, 42);
        let member = ObjectGuid::create_player(1, 43);
        let group_registry = Arc::new(GroupRegistry::default());
        let mut group = GroupInfo::new(leader);
        group.add_member(member);
        let group_guid = group.group_guid;
        group_registry.insert(group_guid, group);

        let player_registry = Arc::new(PlayerRegistry::default());
        let (leader_tx, leader_rx) = bounded(8);
        let (member_tx, member_rx) = bounded(8);
        player_registry.insert(leader, broadcast_info(leader, leader_tx));
        player_registry.insert(member, broadcast_info(member, member_tx));

        session.set_player_guid(Some(member));
        session.group_guid = Some(group_guid);
        session.set_player_registry(Arc::clone(&player_registry));
        session.set_group_registry(group_registry.clone(), Arc::new(PendingInvites::default()));

        session
            .handle_set_everyone_is_assistant(set_everyone_is_assistant_packet(true, None))
            .await;

        let group = group_registry.get(&group_guid).unwrap();
        assert_eq!(
            group.group_flags & wow_network::GROUP_FLAG_EVERYONE_ASSISTANT_LIKE_CPP,
            0
        );
        assert_eq!(
            group.member_slot_like_cpp(member).unwrap().flags
                & wow_network::MEMBER_FLAG_ASSISTANT_LIKE_CPP,
            0
        );
        assert!(leader_rx.try_recv().is_err());
        assert!(member_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn silence_party_talker_leader_records_request_before_cpp_todo_boundary() {
        let (mut session, send_rx) = make_session_with_send();
        let leader = ObjectGuid::create_player(1, 42);
        let target = ObjectGuid::create_player(1, 43);
        let group_registry = Arc::new(GroupRegistry::default());
        let mut group = GroupInfo::new(leader);
        group.add_member(target);
        let group_guid = group.group_guid;
        group_registry.insert(group_guid, group);

        session.set_player_guid(Some(leader));
        session.group_guid = Some(group_guid);
        session.set_group_registry(group_registry, Arc::new(PendingInvites::default()));

        session
            .handle_silence_party_talker(silence_party_talker_packet(target, true))
            .await;

        assert!(send_rx.try_recv().is_err());
        assert_eq!(session.represented_silence_party_talker_like_cpp().len(), 1);
        assert_eq!(
            session.represented_silence_party_talker_like_cpp()[0].target,
            target
        );
        assert!(session.represented_silence_party_talker_like_cpp()[0].silent);
    }

    #[tokio::test]
    async fn silence_party_talker_assistant_allowed_but_regular_member_rejected_like_cpp() {
        let leader = ObjectGuid::create_player(1, 42);
        let assistant = ObjectGuid::create_player(1, 43);
        let regular = ObjectGuid::create_player(1, 44);
        let target = ObjectGuid::create_player(1, 45);
        let group_registry = Arc::new(GroupRegistry::default());
        let mut group = GroupInfo::new(leader);
        group.add_member(assistant);
        group.add_member(regular);
        group.convert_to_raid_like_cpp();
        group
            .set_assistant_leader_flag_like_cpp(assistant, true)
            .unwrap();
        let group_guid = group.group_guid;
        group_registry.insert(group_guid, group);

        let (mut assistant_session, _assistant_send_rx) = make_session_with_send();
        assistant_session.set_player_guid(Some(assistant));
        assistant_session.group_guid = Some(group_guid);
        assistant_session
            .set_group_registry(group_registry.clone(), Arc::new(PendingInvites::default()));

        assistant_session
            .handle_silence_party_talker(silence_party_talker_packet(target, false))
            .await;
        assert_eq!(
            assistant_session
                .represented_silence_party_talker_like_cpp()
                .len(),
            1
        );
        assert!(!assistant_session.represented_silence_party_talker_like_cpp()[0].silent);

        let (mut regular_session, _regular_send_rx) = make_session_with_send();
        regular_session.set_player_guid(Some(regular));
        regular_session.group_guid = Some(group_guid);
        regular_session.set_group_registry(group_registry, Arc::new(PendingInvites::default()));

        regular_session
            .handle_silence_party_talker(silence_party_talker_packet(target, true))
            .await;
        assert!(
            regular_session
                .represented_silence_party_talker_like_cpp()
                .is_empty()
        );
    }

    #[tokio::test]
    async fn silence_party_talker_without_group_is_noop_like_cpp() {
        let (mut session, send_rx) = make_session_with_send();
        let player = ObjectGuid::create_player(1, 42);
        let target = ObjectGuid::create_player(1, 43);
        session.set_player_guid(Some(player));
        session.set_group_registry(
            Arc::new(GroupRegistry::default()),
            Arc::new(PendingInvites::default()),
        );

        session
            .handle_silence_party_talker(silence_party_talker_packet(target, true))
            .await;

        assert!(send_rx.try_recv().is_err());
        assert!(
            session
                .represented_silence_party_talker_like_cpp()
                .is_empty()
        );
    }

    #[tokio::test]
    async fn set_everyone_is_assistant_idempotent_still_fans_out_like_cpp() {
        let (mut session, _send_rx) = make_session_with_send();
        let leader = ObjectGuid::create_player(1, 42);
        let member = ObjectGuid::create_player(1, 43);
        let group_registry = Arc::new(GroupRegistry::default());
        let mut group = GroupInfo::new(leader);
        group.add_member(member);
        group.set_everyone_is_assistant_like_cpp(true);
        let sequence_after_apply = group.sequence_num;
        let group_guid = group.group_guid;
        group_registry.insert(group_guid, group);

        let player_registry = Arc::new(PlayerRegistry::default());
        let (leader_tx, leader_rx) = bounded(8);
        let (member_tx, member_rx) = bounded(8);
        player_registry.insert(leader, broadcast_info(leader, leader_tx));
        player_registry.insert(member, broadcast_info(member, member_tx));

        session.set_player_guid(Some(leader));
        session.group_guid = Some(group_guid);
        session.set_player_registry(Arc::clone(&player_registry));
        session.set_group_registry(group_registry.clone(), Arc::new(PendingInvites::default()));

        session
            .handle_set_everyone_is_assistant(set_everyone_is_assistant_packet(true, None))
            .await;

        assert_eq!(
            group_registry.get(&group_guid).unwrap().sequence_num,
            sequence_after_apply
        );
        assert!(leader_rx.try_recv().is_ok());
        assert!(member_rx.try_recv().is_ok());
    }

    #[tokio::test]
    async fn set_assistant_leader_leader_marks_and_unmarks_member_with_party_update_like_cpp() {
        let (mut session, send_rx) = make_session_with_send();
        let leader = ObjectGuid::create_player(1, 42);
        let member = ObjectGuid::create_player(1, 43);
        let group_registry = Arc::new(GroupRegistry::default());
        let mut group = GroupInfo::new(leader);
        group.add_member(member);
        group.convert_to_raid_like_cpp();
        let group_guid = group.group_guid;
        group_registry.insert(group_guid, group);

        let player_registry = Arc::new(PlayerRegistry::default());
        let (leader_tx, leader_rx) = bounded(8);
        let (member_tx, member_rx) = bounded(8);
        player_registry.insert(leader, broadcast_info(leader, leader_tx));
        player_registry.insert(member, broadcast_info(member, member_tx));

        session.set_player_guid(Some(leader));
        session.group_guid = Some(group_guid);
        session.set_player_registry(Arc::clone(&player_registry));
        session.set_group_registry(group_registry.clone(), Arc::new(PendingInvites::default()));

        session
            .handle_set_assistant_leader(set_assistant_leader_packet(member, true, Some(0)))
            .await;

        assert!(send_rx.try_recv().is_err());
        assert_eq!(
            group_registry
                .get(&group_guid)
                .unwrap()
                .member_slot_like_cpp(member)
                .unwrap()
                .flags
                & wow_network::MEMBER_FLAG_ASSISTANT_LIKE_CPP,
            wow_network::MEMBER_FLAG_ASSISTANT_LIKE_CPP
        );
        let leader_update = leader_rx.try_recv().expect("leader party update");
        assert_eq!(
            u16::from_le_bytes([leader_update[0], leader_update[1]]),
            ServerOpcodes::PartyUpdate as u16
        );
        let member_update = member_rx.try_recv().expect("member party update");
        assert_eq!(
            u16::from_le_bytes([member_update[0], member_update[1]]),
            ServerOpcodes::PartyUpdate as u16
        );

        session
            .handle_set_assistant_leader(set_assistant_leader_packet(member, false, None))
            .await;
        assert_eq!(
            group_registry
                .get(&group_guid)
                .unwrap()
                .member_slot_like_cpp(member)
                .unwrap()
                .flags
                & wow_network::MEMBER_FLAG_ASSISTANT_LIKE_CPP,
            0
        );
    }

    #[tokio::test]
    async fn set_party_leader_leader_changes_to_connected_member_like_cpp() {
        let (mut session, send_rx) = make_session_with_send();
        let leader = ObjectGuid::create_player(1, 42);
        let member = ObjectGuid::create_player(1, 43);
        let group_registry = Arc::new(GroupRegistry::default());
        let mut group = GroupInfo::new(leader);
        group.add_member(member);
        group.convert_to_raid_like_cpp();
        assert_eq!(
            group.set_assistant_leader_flag_like_cpp(member, true),
            Some(wow_network::MEMBER_FLAG_ASSISTANT_LIKE_CPP)
        );
        let group_guid = group.group_guid;
        group_registry.insert(group_guid, group);

        let player_registry = Arc::new(PlayerRegistry::default());
        let (leader_tx, leader_rx) = bounded(8);
        let (member_tx, member_rx) = bounded(8);
        player_registry.insert(leader, broadcast_info(leader, leader_tx));
        player_registry.insert(member, broadcast_info(member, member_tx));

        session.set_player_guid(Some(leader));
        session.group_guid = Some(group_guid);
        session.set_player_registry(Arc::clone(&player_registry));
        session.set_group_registry(group_registry.clone(), Arc::new(PendingInvites::default()));

        session
            .handle_set_party_leader(set_party_leader_packet(member, Some(0)))
            .await;

        assert!(send_rx.try_recv().is_err());
        let group = group_registry.get(&group_guid).unwrap();
        assert_eq!(group.leader_guid, member);
        assert_eq!(
            group.member_slot_like_cpp(member).unwrap().flags
                & wow_network::MEMBER_FLAG_ASSISTANT_LIKE_CPP,
            0
        );
        drop(group);

        let leader_new_leader = leader_rx.try_recv().expect("leader new-leader packet");
        assert_eq!(
            u16::from_le_bytes([leader_new_leader[0], leader_new_leader[1]]),
            ServerOpcodes::GroupNewLeader as u16
        );
        let leader_update = leader_rx.try_recv().expect("leader party update");
        assert_eq!(
            u16::from_le_bytes([leader_update[0], leader_update[1]]),
            ServerOpcodes::PartyUpdate as u16
        );
        let member_new_leader = member_rx.try_recv().expect("member new-leader packet");
        assert_eq!(
            u16::from_le_bytes([member_new_leader[0], member_new_leader[1]]),
            ServerOpcodes::GroupNewLeader as u16
        );
        let member_update = member_rx.try_recv().expect("member party update");
        assert_eq!(
            u16::from_le_bytes([member_update[0], member_update[1]]),
            ServerOpcodes::PartyUpdate as u16
        );
    }

    #[tokio::test]
    async fn set_party_leader_rejects_non_leader_and_disconnected_target_like_cpp() {
        let (mut session, _send_rx) = make_session_with_send();
        let leader = ObjectGuid::create_player(1, 42);
        let member = ObjectGuid::create_player(1, 43);
        let target = ObjectGuid::create_player(1, 44);
        let group_registry = Arc::new(GroupRegistry::default());
        let mut group = GroupInfo::new(leader);
        group.add_member(member);
        group.add_member(target);
        let group_guid = group.group_guid;
        group_registry.insert(group_guid, group);

        let player_registry = Arc::new(PlayerRegistry::default());
        let (leader_tx, leader_rx) = bounded(8);
        let (member_tx, member_rx) = bounded(8);
        player_registry.insert(leader, broadcast_info(leader, leader_tx));
        player_registry.insert(member, broadcast_info(member, member_tx));

        session.set_player_guid(Some(member));
        session.group_guid = Some(group_guid);
        session.set_player_registry(Arc::clone(&player_registry));
        session.set_group_registry(group_registry.clone(), Arc::new(PendingInvites::default()));

        session
            .handle_set_party_leader(set_party_leader_packet(target, None))
            .await;
        assert_eq!(group_registry.get(&group_guid).unwrap().leader_guid, leader);
        assert!(leader_rx.try_recv().is_err());
        assert!(member_rx.try_recv().is_err());

        session.set_player_guid(Some(leader));
        session
            .handle_set_party_leader(set_party_leader_packet(target, None))
            .await;
        assert_eq!(group_registry.get(&group_guid).unwrap().leader_guid, leader);
        assert!(leader_rx.try_recv().is_err());
        assert!(member_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn set_assistant_leader_rejects_non_leader_even_if_assistant_like_cpp() {
        let (mut session, _send_rx) = make_session_with_send();
        let leader = ObjectGuid::create_player(1, 42);
        let assistant = ObjectGuid::create_player(1, 43);
        let target = ObjectGuid::create_player(1, 44);
        let group_registry = Arc::new(GroupRegistry::default());
        let mut group = GroupInfo::new(leader);
        group.add_member(assistant);
        group.add_member(target);
        group.convert_to_raid_like_cpp();
        assert_eq!(
            group.set_assistant_leader_flag_like_cpp(assistant, true),
            Some(wow_network::MEMBER_FLAG_ASSISTANT_LIKE_CPP)
        );
        let group_guid = group.group_guid;
        group_registry.insert(group_guid, group);

        let player_registry = Arc::new(PlayerRegistry::default());
        let (target_tx, target_rx) = bounded(8);
        player_registry.insert(target, broadcast_info(target, target_tx));

        session.set_player_guid(Some(assistant));
        session.group_guid = Some(group_guid);
        session.set_player_registry(Arc::clone(&player_registry));
        session.set_group_registry(group_registry.clone(), Arc::new(PendingInvites::default()));

        session
            .handle_set_assistant_leader(set_assistant_leader_packet(target, true, None))
            .await;

        assert_eq!(
            group_registry
                .get(&group_guid)
                .unwrap()
                .member_slot_like_cpp(target)
                .unwrap()
                .flags,
            0
        );
        assert!(target_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn set_assistant_leader_non_raid_or_missing_target_noops_like_cpp() {
        let (mut session, _send_rx) = make_session_with_send();
        let leader = ObjectGuid::create_player(1, 42);
        let member = ObjectGuid::create_player(1, 43);
        let missing = ObjectGuid::create_player(1, 44);
        let group_registry = Arc::new(GroupRegistry::default());
        let mut group = GroupInfo::new(leader);
        group.add_member(member);
        let group_guid = group.group_guid;
        group_registry.insert(group_guid, group);

        let player_registry = Arc::new(PlayerRegistry::default());
        let (member_tx, member_rx) = bounded(8);
        player_registry.insert(member, broadcast_info(member, member_tx));

        session.set_player_guid(Some(leader));
        session.group_guid = Some(group_guid);
        session.set_player_registry(Arc::clone(&player_registry));
        session.set_group_registry(group_registry.clone(), Arc::new(PendingInvites::default()));

        session
            .handle_set_assistant_leader(set_assistant_leader_packet(member, true, None))
            .await;
        assert_eq!(
            group_registry
                .get(&group_guid)
                .unwrap()
                .member_slot_like_cpp(member)
                .unwrap()
                .flags,
            0
        );

        group_registry
            .get_mut(&group_guid)
            .unwrap()
            .convert_to_raid_like_cpp();
        session
            .handle_set_assistant_leader(set_assistant_leader_packet(missing, true, None))
            .await;
        assert_eq!(
            group_registry
                .get(&group_guid)
                .unwrap()
                .member_slot_like_cpp(member)
                .unwrap()
                .flags,
            0
        );
        assert!(member_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn swap_sub_groups_leader_swaps_members_and_fans_out_update_like_cpp() {
        let (mut session, send_rx) = make_session_with_send();
        let leader = ObjectGuid::create_player(1, 42);
        let first = ObjectGuid::create_player(1, 43);
        let second = ObjectGuid::create_player(1, 44);
        let group_registry = Arc::new(GroupRegistry::default());
        let mut group = GroupInfo::new(leader);
        group.add_member(first);
        group.add_member(second);
        group.convert_to_raid_like_cpp();
        assert!(group.change_member_group_like_cpp(second, 2));
        let group_guid = group.group_guid;
        group_registry.insert(group_guid, group);

        let player_registry = Arc::new(PlayerRegistry::default());
        let (leader_tx, leader_rx) = bounded(8);
        let (first_tx, first_rx) = bounded(8);
        let (second_tx, second_rx) = bounded(8);
        player_registry.insert(leader, broadcast_info(leader, leader_tx));
        player_registry.insert(first, broadcast_info(first, first_tx));
        player_registry.insert(second, broadcast_info(second, second_tx));

        session.set_player_guid(Some(leader));
        session.group_guid = Some(group_guid);
        session.set_player_registry(Arc::clone(&player_registry));
        session.set_group_registry(group_registry.clone(), Arc::new(PendingInvites::default()));

        session
            .handle_swap_sub_groups(swap_sub_groups_packet(first, second, Some(0)))
            .await;

        assert!(send_rx.try_recv().is_err());
        let group = group_registry.get(&group_guid).unwrap();
        assert_eq!(group.member_group_like_cpp(first), 2);
        assert_eq!(group.member_group_like_cpp(second), 0);
        let leader_update = leader_rx.try_recv().expect("leader party update");
        assert_eq!(
            u16::from_le_bytes([leader_update[0], leader_update[1]]),
            ServerOpcodes::PartyUpdate as u16
        );
        let first_update = first_rx.try_recv().expect("first member party update");
        assert_eq!(
            u16::from_le_bytes([first_update[0], first_update[1]]),
            ServerOpcodes::PartyUpdate as u16
        );
        let second_update = second_rx.try_recv().expect("second member party update");
        assert_eq!(
            u16::from_le_bytes([second_update[0], second_update[1]]),
            ServerOpcodes::PartyUpdate as u16
        );
    }

    #[tokio::test]
    async fn swap_sub_groups_assistant_allowed_but_regular_member_rejected_like_cpp() {
        let (mut session, _send_rx) = make_session_with_send();
        let leader = ObjectGuid::create_player(1, 42);
        let assistant = ObjectGuid::create_player(1, 43);
        let first = ObjectGuid::create_player(1, 44);
        let second = ObjectGuid::create_player(1, 45);
        let group_registry = Arc::new(GroupRegistry::default());
        let mut group = GroupInfo::new(leader);
        group.add_member(assistant);
        group.add_member(first);
        group.add_member(second);
        group.convert_to_raid_like_cpp();
        assert!(group.change_member_group_like_cpp(second, 2));
        let group_guid = group.group_guid;
        group_registry.insert(group_guid, group);

        let player_registry = Arc::new(PlayerRegistry::default());
        let (leader_tx, _leader_rx) = bounded(8);
        let (assistant_tx, _assistant_rx) = bounded(8);
        let (first_tx, first_rx) = bounded(8);
        let (second_tx, second_rx) = bounded(8);
        player_registry.insert(leader, broadcast_info(leader, leader_tx));
        player_registry.insert(assistant, broadcast_info(assistant, assistant_tx));
        player_registry.insert(first, broadcast_info(first, first_tx));
        player_registry.insert(second, broadcast_info(second, second_tx));

        session.set_player_guid(Some(assistant));
        session.group_guid = Some(group_guid);
        session.set_player_registry(Arc::clone(&player_registry));
        session.set_group_registry(group_registry.clone(), Arc::new(PendingInvites::default()));

        session
            .handle_swap_sub_groups(swap_sub_groups_packet(first, second, None))
            .await;
        assert_eq!(
            group_registry
                .get(&group_guid)
                .unwrap()
                .member_group_like_cpp(first),
            0
        );
        assert_eq!(
            group_registry
                .get(&group_guid)
                .unwrap()
                .member_group_like_cpp(second),
            2
        );
        assert!(first_rx.try_recv().is_err());
        assert!(second_rx.try_recv().is_err());

        {
            let mut group = group_registry.get_mut(&group_guid).unwrap();
            let slot = group
                .member_slots
                .iter_mut()
                .find(|slot| slot.guid == assistant)
                .expect("assistant slot exists");
            slot.flags |= wow_network::MEMBER_FLAG_ASSISTANT_LIKE_CPP;
        }

        session
            .handle_swap_sub_groups(swap_sub_groups_packet(first, second, None))
            .await;

        assert_eq!(
            group_registry
                .get(&group_guid)
                .unwrap()
                .member_group_like_cpp(first),
            2
        );
        assert_eq!(
            group_registry
                .get(&group_guid)
                .unwrap()
                .member_group_like_cpp(second),
            0
        );
        let first_update = first_rx
            .try_recv()
            .expect("first update after assistant swap");
        assert_eq!(
            u16::from_le_bytes([first_update[0], first_update[1]]),
            ServerOpcodes::PartyUpdate as u16
        );
        let second_update = second_rx
            .try_recv()
            .expect("second update after assistant swap");
        assert_eq!(
            u16::from_le_bytes([second_update[0], second_update[1]]),
            ServerOpcodes::PartyUpdate as u16
        );
    }

    #[tokio::test]
    async fn swap_sub_groups_missing_or_same_subgroup_does_not_fanout_like_cpp() {
        let (mut session, _send_rx) = make_session_with_send();
        let leader = ObjectGuid::create_player(1, 42);
        let first = ObjectGuid::create_player(1, 43);
        let second = ObjectGuid::create_player(1, 44);
        let missing = ObjectGuid::create_player(1, 45);
        let group_registry = Arc::new(GroupRegistry::default());
        let mut group = GroupInfo::new(leader);
        group.add_member(first);
        group.add_member(second);
        group.convert_to_raid_like_cpp();
        let group_guid = group.group_guid;
        group_registry.insert(group_guid, group);

        let player_registry = Arc::new(PlayerRegistry::default());
        let (first_tx, first_rx) = bounded(8);
        let (second_tx, second_rx) = bounded(8);
        player_registry.insert(first, broadcast_info(first, first_tx));
        player_registry.insert(second, broadcast_info(second, second_tx));

        session.set_player_guid(Some(leader));
        session.group_guid = Some(group_guid);
        session.set_player_registry(Arc::clone(&player_registry));
        session.set_group_registry(group_registry.clone(), Arc::new(PendingInvites::default()));

        session
            .handle_swap_sub_groups(swap_sub_groups_packet(first, missing, None))
            .await;
        session
            .handle_swap_sub_groups(swap_sub_groups_packet(first, second, None))
            .await;

        let group = group_registry.get(&group_guid).unwrap();
        assert_eq!(group.member_group_like_cpp(first), 0);
        assert_eq!(group.member_group_like_cpp(second), 0);
        assert!(first_rx.try_recv().is_err());
        assert!(second_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn opt_out_of_loot_sets_pass_on_group_loot_like_cpp() {
        let (mut session, send_rx) = make_session_with_send();
        session.set_player_guid(Some(ObjectGuid::create_player(1, 42)));
        assert!(!session.pass_on_group_loot);

        session
            .handle_opt_out_of_loot(opt_out_of_loot_packet(true))
            .await;

        assert!(session.pass_on_group_loot);
        assert!(send_rx.try_recv().is_err());

        session
            .handle_opt_out_of_loot(opt_out_of_loot_packet(false))
            .await;

        assert!(!session.pass_on_group_loot);
        assert!(send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn opt_out_of_loot_without_loaded_player_is_ignored_like_cpp() {
        let (mut session, send_rx) = make_session_with_send();

        session
            .handle_opt_out_of_loot(opt_out_of_loot_packet(true))
            .await;

        assert!(!session.pass_on_group_loot);
        assert!(send_rx.try_recv().is_err());
    }

    fn low_level_raid_packet() -> WorldPacket {
        WorldPacket::new_empty()
    }

    #[tokio::test]
    async fn low_level_raid1_is_noop_preserves_state_like_cpp() {
        let (mut session, send_rx) = make_session_with_send();
        let guid = ObjectGuid::create_player(1, 42);
        session.set_player_guid(Some(guid));
        session.pass_on_group_loot = false;

        session
            .handle_low_level_raid1(low_level_raid_packet())
            .await;

        assert!(!session.pass_on_group_loot);
        assert!(session.group_guid.is_none());
        assert!(send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn low_level_raid2_is_noop_preserves_state_like_cpp() {
        let (mut session, send_rx) = make_session_with_send();
        let guid = ObjectGuid::create_player(1, 42);
        session.set_player_guid(Some(guid));
        session.pass_on_group_loot = false;

        session
            .handle_low_level_raid2(low_level_raid_packet())
            .await;

        assert!(!session.pass_on_group_loot);
        assert!(session.group_guid.is_none());
        assert!(send_rx.try_recv().is_err());
    }

    fn minimap_ping_packet(x: f32, y: f32, party_index: Option<u8>) -> WorldPacket {
        let mut pkt = WorldPacket::new_empty();
        pkt.write_bit(party_index.is_some());
        pkt.write_float(x);
        pkt.write_float(y);
        if let Some(idx) = party_index {
            pkt.write_uint8(idx);
        }
        pkt.flush_bits();
        pkt.reset_read();
        pkt
    }

    #[tokio::test]
    async fn minimap_ping_without_group_returns_silently_like_cpp() {
        let (mut session, send_rx) = make_session_with_send();
        let guid = ObjectGuid::create_player(1, 42);
        session.set_player_guid(Some(guid));

        session
            .handle_minimap_ping(minimap_ping_packet(10.0, 20.0, None))
            .await;

        assert!(send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn minimap_ping_without_player_guid_returns_silently_like_cpp() {
        let (mut session, send_rx) = make_session_with_send();

        session
            .handle_minimap_ping(minimap_ping_packet(10.0, 20.0, None))
            .await;

        assert!(send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn minimap_ping_with_group_broadcasts_to_other_members_excluding_sender_like_cpp() {
        use wow_constants::ServerOpcodes;

        let (mut session, _send_rx) = make_session_with_send();
        let sender = ObjectGuid::create_player(1, 42);
        let other = ObjectGuid::create_player(1, 43);
        let group_registry = Arc::new(GroupRegistry::default());
        let mut group = GroupInfo::new(sender);
        group.add_member(other);
        let group_guid = group.group_guid;
        group_registry.insert(group_guid, group);

        let player_registry = Arc::new(PlayerRegistry::default());
        let (sender_tx, sender_rx) = bounded(8);
        let (other_tx, other_rx) = bounded(8);
        player_registry.insert(sender, broadcast_info(sender, sender_tx));
        player_registry.insert(other, broadcast_info(other, other_tx));

        session.set_player_guid(Some(sender));
        session.group_guid = Some(group_guid);
        session.set_player_registry(player_registry);
        session.set_group_registry(group_registry, Arc::new(PendingInvites::default()));

        session
            .handle_minimap_ping(minimap_ping_packet(123.456, -789.012, Some(0)))
            .await;

        // Sender should NOT receive the ping (C++ BroadcastPacket excludes sender).
        assert!(
            sender_rx.try_recv().is_err(),
            "sender must not receive own minimap ping"
        );

        // Other member should receive SMSG_MINIMAP_PING with sender guid + x/y.
        let sent = other_rx
            .try_recv()
            .expect("other member should receive minimap ping");
        let mut pkt = WorldPacket::from_bytes(&sent);
        assert_eq!(
            pkt.read_uint16().unwrap(),
            ServerOpcodes::MinimapPing as u16
        );
        assert_eq!(pkt.read_packed_guid().unwrap(), sender);
        assert_eq!(pkt.read_float().unwrap(), 123.456);
        assert_eq!(pkt.read_float().unwrap(), -789.012);
    }

    #[tokio::test]
    async fn minimap_ping_party_index_none_keeps_home_fanout_like_cpp() {
        let (mut session, _send_rx) = make_session_with_send();
        let sender = ObjectGuid::create_player(1, 42);
        let other = ObjectGuid::create_player(1, 43);
        let group_registry = Arc::new(GroupRegistry::default());
        let mut group = GroupInfo::new(sender);
        group.add_member(other);
        let group_guid = group.group_guid;
        group_registry.insert(group_guid, group);

        let player_registry = Arc::new(PlayerRegistry::default());
        let (sender_tx, sender_rx) = bounded(8);
        let (other_tx, other_rx) = bounded(8);
        player_registry.insert(sender, broadcast_info(sender, sender_tx));
        player_registry.insert(other, broadcast_info(other, other_tx));

        session.set_player_guid(Some(sender));
        session.group_guid = Some(group_guid);
        session.set_player_registry(player_registry);
        session.set_group_registry(group_registry, Arc::new(PendingInvites::default()));

        session
            .handle_minimap_ping(minimap_ping_packet(3.0, 4.0, None))
            .await;

        assert!(
            sender_rx.try_recv().is_err(),
            "sender must not receive own minimap ping"
        );
        assert!(
            other_rx.try_recv().is_ok(),
            "PartyIndex=None must keep represented HOME fanout"
        );
    }

    #[tokio::test]
    async fn minimap_ping_sender_not_in_registry_skips_sending_like_cpp() {
        let (mut session, _send_rx) = make_session_with_send();
        let sender = ObjectGuid::create_player(1, 42);
        let other = ObjectGuid::create_player(1, 43);
        let group_registry = Arc::new(GroupRegistry::default());
        let mut group = GroupInfo::new(sender);
        group.add_member(other);
        let group_guid = group.group_guid;
        group_registry.insert(group_guid, group);

        // Only register 'other' — sender has no PlayerRegistry entry (edge case).
        let player_registry = Arc::new(PlayerRegistry::default());
        let (other_tx, other_rx) = bounded(8);
        player_registry.insert(other, broadcast_info(other, other_tx));

        session.set_player_guid(Some(sender));
        session.group_guid = Some(group_guid);
        session.set_player_registry(player_registry);
        session.set_group_registry(group_registry, Arc::new(PendingInvites::default()));

        session
            .handle_minimap_ping(minimap_ping_packet(1.0, 2.0, None))
            .await;

        // Other should still receive (sender is excluded by guid comparison, not by registry).
        let sent = other_rx
            .try_recv()
            .expect("other should receive even if sender not in registry");
        let mut pkt = WorldPacket::from_bytes(&sent);
        assert_eq!(
            pkt.read_uint16().unwrap(),
            ServerOpcodes::MinimapPing as u16
        );
        assert_eq!(pkt.read_packed_guid().unwrap(), sender);
    }

    // ── canonical group lookup architectural tests ─────────────────────────────

    #[test]
    fn current_group_guid_accepts_valid_cached_group() {
        let sender = ObjectGuid::create_player(1, 42);
        let other = ObjectGuid::create_player(1, 43);
        let group_registry = GroupRegistry::default();
        let mut group = GroupInfo::new(sender);
        group.add_member(other);
        let group_guid = group.group_guid;
        group_registry.insert(group_guid, group);

        let result = current_group_guid_like_cpp(&group_registry, Some(group_guid), sender, None);
        assert_eq!(result, Some(group_guid), "valid cache must be accepted");
    }

    #[test]
    fn current_group_guid_ignores_stale_cache_and_finds_real_group() {
        let sender = ObjectGuid::create_player(1, 42);
        let other = ObjectGuid::create_player(1, 43);
        let stale_leader = ObjectGuid::create_player(1, 99);

        let group_registry = GroupRegistry::default();

        // Stale group: sender is NOT a member.
        let mut stale_group = GroupInfo::new(stale_leader);
        stale_group.add_member(other);
        let stale_guid = stale_group.group_guid;
        group_registry.insert(stale_guid, stale_group);

        // Real group: sender IS a member.
        let mut real_group = GroupInfo::new(sender);
        real_group.add_member(other);
        let real_guid = real_group.group_guid;
        group_registry.insert(real_guid, real_group);

        // Cache points to stale group.
        let result = current_group_guid_like_cpp(&group_registry, Some(stale_guid), sender, None);
        assert_eq!(
            result,
            Some(real_guid),
            "stale cache must be bypassed; real group found by scan"
        );
    }

    #[test]
    fn current_group_guid_returns_none_when_sender_not_in_any_group() {
        let sender = ObjectGuid::create_player(1, 42);
        let other = ObjectGuid::create_player(1, 43);

        let group_registry = GroupRegistry::default();
        let group = GroupInfo::new(other);
        let group_guid = group.group_guid;
        group_registry.insert(group_guid, group);

        let result = current_group_guid_like_cpp(&group_registry, Some(group_guid), sender, None);
        assert_eq!(result, None, "sender not in any group must return None");

        let result_no_cache = current_group_guid_like_cpp(&group_registry, None, sender, None);
        assert_eq!(
            result_no_cache, None,
            "no cache + no membership must return None"
        );
    }

    #[tokio::test]
    async fn minimap_ping_stale_cache_does_not_fanout_to_other_group() {
        // Scenario: self.group_guid points to a group where sender is NOT a member.
        // That group has other members who should NOT receive the ping.
        // A separate group exists where the sender IS a member.
        let (mut session, _send_rx) = make_session_with_send();
        let sender = ObjectGuid::create_player(1, 42);
        let stale_member = ObjectGuid::create_player(1, 43);
        let real_member = ObjectGuid::create_player(1, 44);

        let group_registry = Arc::new(GroupRegistry::default());

        // Stale group: sender NOT a member.
        let stale_group = GroupInfo::new(stale_member);
        let stale_guid = stale_group.group_guid;
        group_registry.insert(stale_guid, stale_group);

        // Real group: sender IS a member.
        let mut real_group = GroupInfo::new(sender);
        real_group.add_member(real_member);
        let real_guid = real_group.group_guid;
        group_registry.insert(real_guid, real_group);

        let player_registry = Arc::new(PlayerRegistry::default());
        let (stale_tx, stale_rx) = bounded(8);
        let (real_tx, real_rx) = bounded(8);
        player_registry.insert(stale_member, broadcast_info(stale_member, stale_tx));
        player_registry.insert(real_member, broadcast_info(real_member, real_tx));

        session.set_player_guid(Some(sender));
        // Cache points to stale group.
        session.group_guid = Some(stale_guid);
        session.set_player_registry(player_registry);
        session.set_group_registry(group_registry, Arc::new(PendingInvites::default()));

        session
            .handle_minimap_ping(minimap_ping_packet(10.0, 20.0, None))
            .await;

        // Stale group member must NOT receive the ping.
        assert!(
            stale_rx.try_recv().is_err(),
            "stale group member must not receive minimap ping"
        );

        // Real group member MUST receive the ping.
        let sent = real_rx
            .try_recv()
            .expect("real group member should receive minimap ping");
        let mut pkt = WorldPacket::from_bytes(&sent);
        assert_eq!(
            pkt.read_uint16().unwrap(),
            ServerOpcodes::MinimapPing as u16
        );
        assert_eq!(pkt.read_packed_guid().unwrap(), sender);
        assert_eq!(pkt.read_float().unwrap(), 10.0);
        assert_eq!(pkt.read_float().unwrap(), 20.0);
    }

    #[tokio::test]
    async fn ready_check_stale_cache_uses_real_group_for_mutation_and_fanout() {
        // Scenario: stale cache points to a group where sender is NOT a member.
        // The real group has sender as leader. Ready check must start on the
        // real group, not the stale one.
        let (mut session, _send_rx) = make_session_with_send();
        let sender = ObjectGuid::create_player(1, 42);
        let stale_member = ObjectGuid::create_player(1, 43);
        let real_member = ObjectGuid::create_player(1, 44);

        let group_registry = Arc::new(GroupRegistry::default());

        // Stale group.
        let stale_group = GroupInfo::new(stale_member);
        let stale_guid = stale_group.group_guid;
        group_registry.insert(stale_guid, stale_group);

        // Real group: sender is leader.
        let mut real_group = GroupInfo::new(sender);
        real_group.add_member(real_member);
        let real_guid = real_group.group_guid;
        group_registry.insert(real_guid, real_group);

        let player_registry = Arc::new(PlayerRegistry::default());
        let (stale_tx, stale_rx) = bounded(8);
        let (real_tx, _real_rx) = bounded(8);
        player_registry.insert(stale_member, broadcast_info(stale_member, stale_tx));
        player_registry.insert(real_member, broadcast_info(real_member, real_tx));

        session.set_player_guid(Some(sender));
        session.group_guid = Some(stale_guid);
        session.set_player_registry(player_registry);
        session.set_group_registry(group_registry.clone(), Arc::new(PendingInvites::default()));

        session
            .handle_do_ready_check(do_ready_check_packet(None))
            .await;

        // Stale group member must NOT receive anything.
        assert!(
            stale_rx.try_recv().is_err(),
            "stale group member must not receive ready check"
        );

        // Verify the real group has a ready check active (mutation happened on
        // the correct group, not the stale one).
        let real_group = group_registry.get(&real_guid).unwrap();
        assert!(
            real_group.ready_check_started,
            "real group must have ready check active"
        );

        // Stale group must NOT have a ready check.
        let stale_group = group_registry.get(&stale_guid).unwrap();
        assert!(
            !stale_group.ready_check_started,
            "stale group must not have ready check"
        );
    }

    #[test]
    fn current_group_guid_respects_party_index_category_like_cpp() {
        let sender = ObjectGuid::create_player(1, 42);
        let home_member = ObjectGuid::create_player(1, 43);
        let instance_member = ObjectGuid::create_player(1, 44);
        let stale_leader = ObjectGuid::create_player(1, 99);

        let group_registry = GroupRegistry::default();

        let mut home_group = GroupInfo::new(sender);
        home_group.add_member(home_member);
        let home_guid = home_group.group_guid;
        group_registry.insert(home_guid, home_group);

        let mut stale_group = GroupInfo::new(stale_leader);
        stale_group.add_member(instance_member);
        let stale_guid = stale_group.group_guid;
        group_registry.insert(stale_guid, stale_group);

        assert_eq!(
            current_group_guid_like_cpp(&group_registry, Some(home_guid), sender, None),
            Some(home_guid),
            "PartyIndex=None keeps represented #791 current-group semantics"
        );
        assert_eq!(
            current_group_guid_like_cpp(&group_registry, Some(home_guid), sender, Some(0)),
            Some(home_guid),
            "PartyIndex HOME resolves represented HOME group"
        );
        assert_eq!(
            current_group_guid_like_cpp(&group_registry, Some(home_guid), sender, Some(1)),
            None,
            "PartyIndex INSTANCE must not fall back to represented HOME group"
        );
        assert_eq!(
            current_group_guid_like_cpp(&group_registry, Some(home_guid), sender, Some(2)),
            None,
            "PartyIndex >= MAX_GROUP_CATEGORY returns None"
        );
        assert_eq!(
            current_group_guid_like_cpp(&group_registry, Some(stale_guid), sender, Some(0)),
            Some(home_guid),
            "stale cache cannot authorize, fallback membership still respects HOME category"
        );
        assert_eq!(
            current_group_guid_like_cpp(&group_registry, Some(stale_guid), sender, Some(1)),
            None,
            "stale cache fallback must not resolve HOME for requested INSTANCE"
        );
    }

    #[tokio::test]
    async fn minimap_ping_party_index_instance_does_not_fanout_home_like_cpp() {
        let (mut session, _send_rx) = make_session_with_send();
        let sender = ObjectGuid::create_player(1, 42);
        let other = ObjectGuid::create_player(1, 43);
        let group_registry = Arc::new(GroupRegistry::default());
        let mut group = GroupInfo::new(sender);
        group.add_member(other);
        let group_guid = group.group_guid;
        group_registry.insert(group_guid, group);

        let player_registry = Arc::new(PlayerRegistry::default());
        let (sender_tx, sender_rx) = bounded(8);
        let (other_tx, other_rx) = bounded(8);
        player_registry.insert(sender, broadcast_info(sender, sender_tx));
        player_registry.insert(other, broadcast_info(other, other_tx));

        session.set_player_guid(Some(sender));
        session.group_guid = Some(group_guid);
        session.set_player_registry(player_registry);
        session.set_group_registry(group_registry.clone(), Arc::new(PendingInvites::default()));

        session
            .handle_minimap_ping(minimap_ping_packet(1.0, 2.0, Some(1)))
            .await;

        assert!(
            sender_rx.try_recv().is_err(),
            "sender must not receive a fanout"
        );
        assert!(
            other_rx.try_recv().is_err(),
            "HOME member must not receive minimap ping for PartyIndex INSTANCE"
        );
    }

    #[tokio::test]
    async fn initiate_role_poll_uses_resolved_group_category_like_cpp() {
        let (mut session, _send_rx) = make_session_with_send();
        let leader = ObjectGuid::create_player(1, 42);
        let member = ObjectGuid::create_player(1, 43);
        let group_registry = Arc::new(GroupRegistry::default());
        let mut group = GroupInfo::new(leader);
        group.group_category = wow_network::group_registry::GROUP_CATEGORY_INSTANCE_LIKE_CPP;
        group.add_member(member);
        let group_guid = group.group_guid;
        group_registry.insert(group_guid, group);

        let player_registry = Arc::new(PlayerRegistry::default());
        let (leader_tx, leader_rx) = bounded(8);
        let (member_tx, member_rx) = bounded(8);
        player_registry.insert(leader, broadcast_info(leader, leader_tx));
        player_registry.insert(member, broadcast_info(member, member_tx));

        session.set_player_guid(Some(leader));
        session.group_guid = Some(group_guid);
        session.set_player_registry(player_registry);
        session.set_group_registry(group_registry, Arc::new(PendingInvites::default()));

        session
            .handle_initiate_role_poll(initiate_role_poll_packet(Some(1)))
            .await;

        for sent in [
            leader_rx.try_recv().expect("leader role poll inform"),
            member_rx.try_recv().expect("member role poll inform"),
        ] {
            let mut pkt = WorldPacket::from_bytes(&sent);
            assert_eq!(
                pkt.read_uint16().unwrap(),
                ServerOpcodes::RolePollInform as u16
            );
            assert_eq!(
                pkt.read_int8().unwrap(),
                wow_network::group_registry::GROUP_CATEGORY_INSTANCE_LIKE_CPP as i8
            );
            assert_eq!(pkt.read_packed_guid().unwrap(), leader);
        }
    }
}
