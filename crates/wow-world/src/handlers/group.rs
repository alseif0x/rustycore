// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Handlers for Group/Party opcodes: PartyInvite, PartyInviteResponse, LeaveGroup.

use tracing::{info, warn};
use wow_constants::ClientOpcodes;
use wow_core::ObjectGuid;
use wow_handler::{PacketHandlerEntry, PacketProcessing, SessionStatus};
use wow_network::{GroupInfo, PlayerRegistry};
use wow_packet::packets::party::{
    GroupDecline, GroupDestroyed, GroupUninvite, PartyCommandResult, PartyDifficultySettings,
    PartyInviteServer, PartyLootSettings, PartyMemberFullState, PartyPlayerInfo, PartyUpdate,
    party_result,
};
use wow_packet::ServerPacket;

use crate::session::WorldSession;

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

// ── Helpers ───────────────────────────────────────────────────────────────────

fn class_to_power_type(class: u8) -> u8 {
    match class {
        1 => 1, // Warrior: Rage
        4 => 3, // Rogue: Energy
        6 => 6, // DeathKnight: RunicPower
        _ => 0, // Mana (default)
    }
}

/// Sends `PartyUpdate` + `PartyMemberFullState` to every member of `group`.
///
/// Each member gets a `PartyUpdate` where their own `my_index` reflects their
/// position in the member list.  A `PartyMemberFullState` is then sent for
/// every *other* member.
fn send_party_update(group: &GroupInfo, registry: &PlayerRegistry, _vra: u32) {
    // Pre-build the full PlayerList (ALL members including each receiver)
    let all_players: Vec<PartyPlayerInfo> = group.members.iter().filter_map(|&guid| {
        registry.get(&guid).map(|entry| PartyPlayerInfo {
            guid,
            name: entry.player_name.clone(),
            class: entry.class,
            subgroup: 0,
            flags: 0,
            roles_assigned: 0,
            faction_group: if entry.race <= 5 { 1 } else { 2 },
            connected: true,
        })
    }).collect();

    for (my_idx, &member_guid) in group.members.iter().enumerate() {
        let member_entry = match registry.get(&member_guid) {
            Some(e) => e,
            None => continue,
        };

        let update = PartyUpdate {
            party_flags: 0,
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
                loot_master: ObjectGuid::EMPTY,
                threshold: 2,
            }),
            difficulty_settings: Some(PartyDifficultySettings {
                dungeon_difficulty_id: 1,
                raid_difficulty_id: 14,
                legacy_raid_difficulty_id: 3,
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
                };
                let _ = member_entry.send_tx.send(full_state.to_bytes());
            }
        }
    }
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
            Err(e) => { warn!("PartyInvite: name_len read error: {}", e); return; }
        };
        let realm_len = match pkt.read_bits(9) {
            Ok(n) => n as usize,
            Err(e) => { warn!("PartyInvite: realm_len read error: {}", e); return; }
        };

        let _proposed_roles = pkt.read_uint32().unwrap_or(0);

        let target_guid = match pkt.read_packed_guid() {
            Ok(g) => g,
            Err(e) => { warn!("PartyInvite: target_guid read error: {}", e); return; }
        };
        let target_name = match pkt.read_string(name_len) {
            Ok(s) => s,
            Err(e) => { warn!("PartyInvite: target_name read error: {}", e); return; }
        };
        let _realm_name = pkt.read_string(realm_len).unwrap_or_default();
        if has_party_index {
            let _ = pkt.read_uint8();
        }
        info!(account = self.account_id, target_name = %target_name, "PartyInvite parsed");

        // — setup —
        let my_guid = match self.player_guid {
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
        let target_entry_opt = registry.iter()
            .find(|e| e.value().player_name.eq_ignore_ascii_case(&target_name));

        let real_target_guid = match target_entry_opt {
            Some(ref e) => *e.key(),
            None => {
                warn!("PartyInvite: target '{}' not found in registry", target_name);
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
        let inviter_name = self.player_name.clone().unwrap_or_default();
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
        let my_guid = match self.player_guid {
            Some(g) => g,
            None => return,
        };
        let my_name = self.player_name.clone().unwrap_or_default();

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

        let group_guid = if let Some(gid) = existing_gid {
            if let Some(mut g) = group_reg.get_mut(&gid) {
                g.add_member(my_guid);
            }
            gid
        } else {
            // Create a new group with the inviter as leader, then add self.
            let mut new_group = GroupInfo::new(inviter_guid);
            new_group.add_member(my_guid);
            let gid = new_group.group_guid;
            group_reg.insert(gid, new_group);
            gid
        };

        // Update self's group_guid in session — all Arc borrows are gone now.
        self.group_guid = Some(group_guid);

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
        let my_guid = match self.player_guid {
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
        {
            let mut group = match group_reg.get_mut(&gid) {
                Some(g) => g,
                None => return,
            };
            group.remove_member(&my_guid);

            if group.members.len() < 2 {
                dissolve_remaining = Some(group.members.clone());
            } else {
                dissolve_remaining = None;
                // Reassign leader if needed.
                if group.leader_guid == my_guid {
                    if let Some(&new_leader) = group.members.first() {
                        group.leader_guid = new_leader;
                    }
                }
            }
        }

        if let Some(remaining) = dissolve_remaining {
            // Group dissolved — notify last remaining member (if any).
            group_reg.remove(&gid);
            if let Some(&last_guid) = remaining.first() {
                if let Some(last_entry) = registry.get(&last_guid) {
                    let _ = last_entry.send_tx.send(GroupDestroyed.to_bytes());
                }
            }
            // Tell self to leave.
            self.send_packet(&GroupUninvite);
            self.group_guid = None;
            return;
        }

        // 3. Send updated PartyUpdate to remaining members.
        if let Some(group) = group_reg.get(&gid) {
            send_party_update(&group, &registry, vra);
        }

        // 4. Uninvite self.
        self.send_packet(&GroupUninvite);
        self.group_guid = None;
    }
}
