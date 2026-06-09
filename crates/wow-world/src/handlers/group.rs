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
use wow_network::{
    GROUP_ASSIGN_MAINASSIST_LIKE_CPP, GROUP_ASSIGN_MAINTANK_LIKE_CPP, GroupInfo,
    MEMBER_FLAG_ASSISTANT_LIKE_CPP, MEMBER_FLAG_MAINASSIST_LIKE_CPP, MEMBER_FLAG_MAINTANK_LIKE_CPP,
    PlayerRegistry, ReadyCheckEventLikeCpp, free_group_db_store_id_like_cpp,
    register_group_db_store_id_like_cpp,
};
use wow_packet::packets::party::{
    DoReadyCheck, GroupDecline, GroupDestroyed, GroupUninvite, OptOutOfLoot, PartyCommandResult,
    PartyDifficultySettings, PartyInviteServer, PartyLootSettings, PartyMemberFullState,
    PartyPlayerInfo, PartyUpdate, ReadyCheckCompleted, ReadyCheckResponse,
    ReadyCheckResponseClient, ReadyCheckStarted, SetAssistantLeader, SetEveryoneIsAssistant,
    SetLootMethod, SetPartyAssignment, party_result,
};
use wow_packet::{ClientPacket, ServerPacket};

use crate::session::WorldSession;

const EMPTY_TARGET_ICON_RAW_LIKE_CPP: [u8; 16] = [0; 16];

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
        opcode: ClientOpcodes::SetPartyAssignment,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_set_party_assignment",
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

// ── Helpers ───────────────────────────────────────────────────────────────────

fn class_to_power_type(class: u8) -> u8 {
    match class {
        1 => 1, // Warrior: Rage
        4 => 3, // Rogue: Energy
        6 => 6, // DeathKnight: RunicPower
        _ => 0, // Mana (default)
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
            party_index: 0,
            party_type: 1,
            my_index: my_idx as i32,
            party_guid: group.group_guid,
            sequence_num: group.sequence_num as i32,
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

        let _ = member_entry.send_tx.send(update.to_bytes());

        // PartyMemberFullState for every OTHER member (still excludes self)
        for &other_guid in &group.members {
            if other_guid == member_guid {
                continue;
            }
            if let Some(other) = registry.get(&other_guid) {
                let pos = other.position;
                let full_state = PartyMemberFullState {
                    member_guid: other_guid,
                    for_enemy: false,
                    status: 1,
                    power_type: class_to_power_type(other.class),
                    current_health: 1000,
                    max_health: 1000,
                    current_power: 500,
                    max_power: 500,
                    level: other.level as u16,
                    spec_id: 0,
                    zone_id: 0,
                    position_x: pos.x as i16,
                    position_y: pos.y as i16,
                    position_z: pos.z as i16,
                    phases: other.party_member_phase_states.clone(),
                };
                let _ = member_entry.send_tx.send(full_state.to_bytes());
            }
        }
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
        stmt.set_bytes(5 + index, EMPTY_TARGET_ICON_RAW_LIKE_CPP.to_vec());
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

        if let Some(gid) = self.group_guid {
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
        let gid = match self.group_guid {
            Some(g) => g,
            None => {
                // Fallback: search by guid in case group_guid wasn't set.
                match group_reg
                    .iter()
                    .find(|e| e.value().members.contains(&my_guid))
                    .map(|e| *e.key())
                {
                    Some(g) => g,
                    None => return,
                }
            }
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
                    let _ = last_entry.send_tx.send(GroupDestroyed.to_bytes());
                }
            }
            // Tell self to leave.
            self.group_guid = None;
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
        let group_guid = match self.group_guid {
            Some(group_guid) => group_guid,
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

        let group_guid = match self.group_guid {
            Some(group_guid) => group_guid,
            None => match group_reg
                .iter()
                .find(|entry| entry.value().members.contains(&sender_guid))
                .map(|entry| *entry.key())
            {
                Some(group_guid) => group_guid,
                None => return,
            },
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
        let _represented_party_index_boundary = swap.party_index;

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

        let group_guid = match self.group_guid {
            Some(group_guid) => group_guid,
            None => match group_reg
                .iter()
                .find(|entry| entry.value().members.contains(&sender_guid))
                .map(|entry| *entry.key())
            {
                Some(group_guid) => group_guid,
                None => return,
            },
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
        let _represented_party_index_boundary = set_assistant.party_index;

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

        let group_guid = match self.group_guid {
            Some(group_guid) => group_guid,
            None => match group_reg
                .iter()
                .find(|entry| entry.value().members.contains(&sender_guid))
                .map(|entry| *entry.key())
            {
                Some(group_guid) => group_guid,
                None => return,
            },
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
        let _represented_party_index_boundary = set_everyone.party_index;

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

        let group_guid = match self.group_guid {
            Some(group_guid) => group_guid,
            None => match group_reg
                .iter()
                .find(|entry| entry.value().members.contains(&sender_guid))
                .map(|entry| *entry.key())
            {
                Some(group_guid) => group_guid,
                None => return,
            },
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

    /// CMSG_DO_READY_CHECK.
    ///
    /// C++ resolves `GetPlayer()->GetGroup(packet.PartyIndex)`, returns when no
    /// group exists, requires leader or assistant, then calls
    /// `Group::StartReadyCheck`. Rust represents PartyIndex over the current
    /// GroupRegistry group and approximates offline/no-session via missing
    /// PlayerRegistry entries. Timeout remains represented state only because
    /// there is no full `Group::UpdateReadyCheck` tick loop yet.
    pub async fn handle_do_ready_check(&mut self, mut pkt: wow_packet::WorldPacket) {
        let ready_check = match DoReadyCheck::read(&mut pkt) {
            Ok(ready_check) => ready_check,
            Err(e) => {
                warn!("Bad DoReadyCheck: {e}");
                return;
            }
        };
        let _represented_party_index_boundary = ready_check.party_index;

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

        let group_guid = match self.group_guid {
            Some(group_guid) => group_guid,
            None => match group_reg
                .iter()
                .find(|entry| entry.value().members.contains(&sender_guid))
                .map(|entry| *entry.key())
            {
                Some(group_guid) => group_guid,
                None => return,
            },
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
        let _represented_party_index_boundary = response.party_index;

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

        let group_guid = match self.group_guid {
            Some(group_guid) => group_guid,
            None => match group_reg
                .iter()
                .find(|entry| entry.value().members.contains(&sender_guid))
                .map(|entry| *entry.key())
            {
                Some(group_guid) => group_guid,
                None => return,
            },
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
        let _represented_party_index_boundary = assignment.party_index;

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

        let group_guid = match self.group_guid {
            Some(group_guid) => group_guid,
            None => match group_reg
                .iter()
                .find(|entry| entry.value().members.contains(&sender_guid))
                .map(|entry| *entry.key())
            {
                Some(group_guid) => group_guid,
                None => return,
            },
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
}

#[cfg(test)]
mod tests {
    use super::{
        first_connected_group_member_like_cpp, group_delete_statement_like_cpp,
        group_insert_statement_like_cpp, group_leader_update_statement_like_cpp,
        group_lfg_data_delete_statement_like_cpp, group_member_delete_all_statement_like_cpp,
        group_member_delete_statement_like_cpp, group_member_flag_update_statement_like_cpp,
        group_member_insert_statement_like_cpp, group_member_subgroup_update_statement_like_cpp,
        group_type_update_statement_like_cpp, party_player_info_like_cpp, send_party_update,
        send_ready_check_events_like_cpp, sender_can_start_ready_check_like_cpp,
    };
    use flume::bounded;
    use std::sync::Arc;
    use wow_constants::ServerOpcodes;
    use wow_core::{ObjectGuid, Position};
    use wow_database::{CharStatements, SqlParam, StatementDef};
    use wow_network::{
        GroupInfo, GroupMemberCharacterLikeCpp, GroupRegistry, PendingInvites, PlayerBroadcastInfo,
        PlayerRegistry, ReadyCheckEventLikeCpp, SessionCommand,
    };
    use wow_packet::WorldPacket;

    use crate::session::WorldSession;

    fn broadcast_info(guid: ObjectGuid, send_tx: flume::Sender<Vec<u8>>) -> PlayerBroadcastInfo {
        let (command_tx, _command_rx) = flume::bounded(1);
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
            party_member_phase_states: Default::default(),
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
                party_index: 0,
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
        assert!(
            matches!(
                member_command_rx.try_recv(),
                Ok(SessionCommand::RefreshVisibleGameobjectsOrSpellClicksLikeCpp)
            ),
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
}
