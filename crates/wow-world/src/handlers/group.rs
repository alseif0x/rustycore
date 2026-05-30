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
use wow_network::{GroupInfo, PlayerRegistry};
use wow_packet::packets::party::{
    GroupDecline, GroupDestroyed, GroupUninvite, OptOutOfLoot, PartyCommandResult,
    PartyDifficultySettings, PartyInviteServer, PartyLootSettings, PartyMemberFullState,
    PartyPlayerInfo, PartyUpdate, SetLootMethod, party_result,
};
use wow_packet::{ClientPacket, ServerPacket};

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
        opcode: ClientOpcodes::SetLootMethod,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_set_loot_method",
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
        .filter_map(|&guid| {
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
        })
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
                    phases: other.party_member_phase_states.clone(),
                };
                let _ = member_entry.send_tx.send(full_state.to_bytes());
            }
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
        let group_guid = if let Some(gid) = existing_gid {
            if let Some(mut g) = group_reg.get_mut(&gid) {
                g.add_member(my_guid);
                refresh_visible_gameobjects_or_spellclicks = g.is_raid_group();
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
        if refresh_visible_gameobjects_or_spellclicks {
            let _ = self.update_visible_gameobjects_or_spell_clicks_like_cpp();
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
                group_type_persistence = u32::try_from(group.group_guid)
                    .ok()
                    .map(|db_guid| (group.group_flags, db_guid));
                true
            } else {
                let converted = group.convert_to_group_like_cpp();
                if converted {
                    group_type_persistence = u32::try_from(group.group_guid)
                        .ok()
                        .map(|db_guid| (group.group_flags, db_guid));
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
    use super::{group_type_update_statement_like_cpp, send_party_update};
    use flume::bounded;
    use std::sync::Arc;
    use wow_constants::ServerOpcodes;
    use wow_core::{ObjectGuid, Position};
    use wow_database::{CharStatements, SqlParam, StatementDef};
    use wow_network::{
        GroupInfo, GroupRegistry, PendingInvites, PlayerBroadcastInfo, PlayerRegistry,
        SessionCommand,
    };
    use wow_packet::WorldPacket;

    use crate::session::WorldSession;

    fn broadcast_info(guid: ObjectGuid, send_tx: flume::Sender<Vec<u8>>) -> PlayerBroadcastInfo {
        let (command_tx, _command_rx) = flume::bounded(1);
        PlayerBroadcastInfo {
            map_id: 0,
            instance_id: 0,
            position: Position::ZERO,
            is_in_world: true,
            send_tx,
            command_tx,
            active_loot_rolls: Vec::new(),
            pass_on_group_loot: false,
            enchanting_skill: 0,
            is_alive: true,
            active_expansion: 2,
            pending_quest_sharing: None,
            known_spells: Vec::new(),
            active_quest_statuses: Default::default(),
            active_quest_objective_counts: Default::default(),
            rewarded_quests: Default::default(),
            daily_quests_completed: Default::default(),
            df_quests: Default::default(),
            reputation_standings: Vec::new(),
            inventory_item_counts: Default::default(),
            party_member_phase_states: Default::default(),
            player_name: format!("Player{}", guid.low_value()),
            account_id: 1,
            recruiter_id: 0,
            race: 1,
            class: 1,
            sex: 0,
            level: 1,
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
