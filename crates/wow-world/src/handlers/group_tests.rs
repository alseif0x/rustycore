//! Behaviour tests for [`super`].
//!
//! Extracted from `group.rs`, which was 8,665 lines of which
//! 5,275 — 61% — were this one `mod tests`. The production code and its
//! module boundaries are untouched: moving tests moves no invariant. Dedenting by
//! one level lets rustfmt collapse some argument lists onto a single line, which
//! drops their trailing commas; that is the only difference from the original text.

#![cfg(test)]

use super::{
    PARTY_REALM_COMMAND_TIMEOUT_LIKE_CPP, SOCIAL_FLAG_FRIEND_LIKE_CPP,
    SOCIAL_FLAG_IGNORED_LIKE_CPP, current_group_guid_like_cpp,
    first_connected_group_member_like_cpp, group_delete_statement_like_cpp,
    group_leader_update_statement_like_cpp, group_lfg_data_delete_statement_like_cpp,
    group_member_delete_all_statement_like_cpp, group_member_delete_statement_like_cpp,
    group_member_flag_update_statement_like_cpp, group_member_insert_statement_like_cpp,
    group_member_subgroup_update_statement_like_cpp, group_persistence_statement_like_cpp,
    group_type_update_statement_like_cpp, party_invite_social_friend_match_like_cpp,
    party_invite_social_ignore_match_like_cpp, party_player_info_like_cpp,
    send_group_new_leader_like_cpp, send_party_update, send_ready_check_events_like_cpp,
    sender_can_start_ready_check_like_cpp,
};
use flume::bounded;
use std::{sync::Arc, time::Duration};
use wow_constants::{ClientOpcodes, ServerOpcodes};
use wow_core::{ObjectGuid, Position, guid::HighGuid};
use wow_database::{CharStatements, SqlParam, StatementDef};
use wow_handler::{PacketHandlerEntry, PacketProcessing, SessionStatus};
use wow_network::group_registry::GROUP_CATEGORY_HOME_LIKE_CPP;
use wow_network::{
    GroupInfo, GroupMemberCharacterLikeCpp, GroupRegistry, PendingInviteLikeCpp, PendingInvites,
    PlayerBroadcastInfo, PlayerRegistry, ReadyCheckEventLikeCpp, SendRealmPacketLikeCppCommand,
    SessionCommand,
};
use wow_packet::{ServerPacket, WorldPacket, packets::party::party_result};

use crate::session::WorldSession;

fn test_session_command_dispatcher(
    guid: ObjectGuid,
    send_tx: flume::Sender<Vec<u8>>,
) -> (
    flume::Sender<SessionCommand>,
    flume::Receiver<SessionCommand>,
) {
    let (command_tx, command_rx) = flume::bounded(0);
    let (observed_tx, observed_rx) = flume::unbounded();
    std::thread::spawn(move || {
        let mut party_sequences = std::collections::HashMap::<u8, i32>::new();
        while let Ok(command) = command_rx.recv() {
            match command {
                SessionCommand::SendRealmPacketLikeCpp(command) if command.recipient == guid => {
                    let _ = send_tx.send(command.packet_bytes);
                }
                SessionCommand::SendPartyUpdateLikeCpp(mut command)
                    if command.recipient == guid =>
                {
                    let sequence = party_sequences
                        .entry(command.party_update.party_index)
                        .or_default();
                    *sequence += 1;
                    command.party_update.sequence_num = *sequence;
                    let _ = send_tx.send(command.party_update.to_bytes());
                    for packet in command.member_full_state_packets {
                        let _ = send_tx.send(packet);
                    }
                }
                command => {
                    let _ = observed_tx.send(command);
                }
            }
        }
    });
    (command_tx, observed_rx)
}

fn broadcast_info(guid: ObjectGuid, send_tx: flume::Sender<Vec<u8>>) -> PlayerBroadcastInfo {
    let (command_tx, _observed_rx) = test_session_command_dispatcher(guid, send_tx.clone());
    broadcast_info_with_command_tx(guid, send_tx, command_tx)
}

fn recv_dispatched_packet(rx: &flume::Receiver<Vec<u8>>, label: &str) -> Vec<u8> {
    rx.recv_timeout(std::time::Duration::from_secs(1))
        .unwrap_or_else(|error| panic!("{label}: {error}"))
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
        realm_send_tx: send_tx.clone(),
        send_tx,
        command_tx,
        durable_creature_runtime_commands_like_cpp: Default::default(),
        client_visible_guids_like_cpp: Default::default(),
        advanced_combat_logging_enabled_like_cpp: Default::default(),
        visibility_refresh_pending_like_cpp: Default::default(),
        durable_loot_money_tracker_like_cpp: Default::default(),
        active_loot_rolls: Vec::new(),
        in_combat: false,
        pass_on_group_loot: false,
        enchanting_skill: 0,
        is_alive: true,
        current_health: 100,
        max_health: 100,
        power_type: 0,
        current_power: 0,
        max_power: 0,
        base_mana: 0,
        transport: None,
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
        dungeon_difficulty_id: 1,
        is_contested_pvp: false,
        active_expansion: 2,
        pending_quest_sharing: None,
        known_spells: Vec::new(),
        active_quest_statuses: Default::default(),
        active_quest_objective_counts: Default::default(),
        rewarded_quests: Default::default(),
        completed_achievements: Default::default(),
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
        visible_items: std::sync::Arc::new([(0, 0, 0); 19]),
        customizations: std::sync::Arc::default(),
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

fn party_uninvite_packet(target: ObjectGuid, party_index: Option<u8>, reason: &str) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_bit(party_index.is_some());
    pkt.write_bits(reason.len() as u32, 8);
    pkt.write_packed_guid(&target);
    if let Some(party_index) = party_index {
        pkt.write_uint8(party_index);
    }
    pkt.write_string(reason);
    pkt.flush_bits();
    pkt.reset_read();
    pkt
}

fn party_invite_packet(
    target_guid: ObjectGuid,
    target_name: &str,
    party_index: Option<u8>,
    proposed_roles: u32,
) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_bit(party_index.is_some());
    pkt.flush_bits();
    pkt.write_bits(target_name.len() as u32, 9);
    pkt.write_bits(0, 9);
    pkt.write_uint32(proposed_roles);
    pkt.write_packed_guid(&target_guid);
    pkt.write_string(target_name);
    if let Some(party_index) = party_index {
        pkt.write_uint8(party_index);
    }
    pkt.reset_read();
    pkt
}

fn party_invite_response_packet(
    accept: bool,
    party_index: Option<u8>,
    roles: Option<u8>,
) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_bit(party_index.is_some());
    pkt.write_bit(accept);
    pkt.write_bit(roles.is_some());
    if let Some(party_index) = party_index {
        pkt.write_uint8(party_index);
    }
    if let Some(roles) = roles {
        pkt.write_uint8(roles);
    }
    pkt.flush_bits();
    pkt.reset_read();
    pkt
}

fn party_command_result_code(bytes: &[u8]) -> u8 {
    let mut packet = WorldPacket::from_bytes(bytes);
    assert_eq!(
        packet.read_uint16().expect("opcode"),
        ServerOpcodes::PartyCommandResult as u16
    );
    let _ = packet.read_bits(9).expect("name len");
    let _ = packet.read_bits(4).expect("party operation");
    packet.read_bits(6).expect("party result") as u8
}

fn party_invite_can_accept(bytes: &[u8]) -> bool {
    let mut packet = WorldPacket::from_bytes(bytes);
    assert_eq!(
        packet.read_uint16().expect("opcode"),
        ServerOpcodes::PartyInvite as u16
    );
    packet.read_bit().expect("can accept")
}

fn leave_group_packet(party_index: Option<u8>) -> WorldPacket {
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

fn assert_raid_markers_packet_like_cpp(
    bytes: &[u8],
    expected_active_markers: u32,
    expected_positions: &[Position],
) {
    let mut pkt = WorldPacket::from_bytes(bytes);
    assert_eq!(
        pkt.read_uint16().unwrap(),
        ServerOpcodes::RaidMarkersChanged as u16
    );
    assert_eq!(pkt.read_uint8().unwrap(), GROUP_CATEGORY_HOME_LIKE_CPP);
    assert_eq!(pkt.read_uint32().unwrap(), expected_active_markers);
    assert_eq!(
        pkt.read_bits(4).unwrap(),
        u32::try_from(expected_positions.len()).unwrap()
    );
    pkt.flush_bits();
    for expected_position in expected_positions {
        assert_eq!(pkt.read_packed_guid().unwrap(), ObjectGuid::EMPTY);
        assert_eq!(pkt.read_uint32().unwrap(), 571);
        assert_eq!(pkt.read_float().unwrap(), expected_position.x);
        assert_eq!(pkt.read_float().unwrap(), expected_position.y);
        assert_eq!(pkt.read_float().unwrap(), expected_position.z);
    }
    assert!(pkt.is_empty());
}

fn clear_raid_marker_packet(marker_id: u8) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint8(marker_id);
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
    let mut session = WorldSession::new(
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
    );
    session.set_loaded_player_identity_like_cpp(0, 1, 1, 80, 0);
    (session, send_rx)
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

    let sent = recv_dispatched_packet(&rx, "raid PartyUpdate");
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

    let sent = recv_dispatched_packet(&rx, "non-master-loot PartyUpdate");
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

    let sent = recv_dispatched_packet(&rx, "raid-group PartyUpdate");
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
    let stmt = group_persistence_statement_like_cpp(
        wow_network::GroupPersistenceIntentLikeCpp::InsertGroup {
            db_store_id: 77,
            leader_guid: group.leader_guid,
            loot_method: group.loot_method,
            looter_guid: group.looter_guid,
            loot_threshold: group.loot_threshold,
            group_flags: group.group_flags,
            dungeon_difficulty_id: group.dungeon_difficulty_id,
            raid_difficulty_id: group.raid_difficulty_id,
            legacy_raid_difficulty_id: group.legacy_raid_difficulty_id,
            master_looter_guid: group.master_looter_guid,
        },
    );

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
    group_registry.register_group_like_cpp(group_guid, group);

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
async fn party_invite_party_index_instance_does_not_use_full_home_group_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send();
    let inviter = ObjectGuid::create_player(1, 42);
    let target = ObjectGuid::create_player(1, 77);
    let target_name = format!("Player{}", target.low_value());

    let player_registry = Arc::new(PlayerRegistry::default());
    let (target_tx, target_rx) = bounded(8);
    player_registry.insert(target, broadcast_info(target, target_tx));

    let group_registry = Arc::new(GroupRegistry::default());
    let mut home_group = GroupInfo::new(inviter);
    home_group.add_member(ObjectGuid::create_player(1, 101));
    home_group.add_member(ObjectGuid::create_player(1, 102));
    home_group.add_member(ObjectGuid::create_player(1, 103));
    home_group.add_member(ObjectGuid::create_player(1, 104));
    let home_group_guid = home_group.group_guid;
    group_registry.register_group_like_cpp(home_group_guid, home_group);

    let pending_invites = Arc::new(PendingInvites::default());
    session.set_player_guid(Some(inviter));
    session.group_guid = Some(home_group_guid);
    session.set_player_registry(player_registry);
    session.set_group_registry(Arc::clone(&group_registry), Arc::clone(&pending_invites));

    session
        .handle_party_invite(party_invite_packet(
            target,
            &target_name,
            Some(wow_network::group_registry::GROUP_CATEGORY_INSTANCE_LIKE_CPP),
            0,
        ))
        .await;

    assert!(
        pending_invites.get(&target).is_some(),
        "PartyIndex INSTANCE must not treat the full HOME group as the invite group"
    );
    let invite = recv_dispatched_packet(&target_rx, "target invite packet");
    assert_eq!(
        u16::from_le_bytes([invite[0], invite[1]]),
        ServerOpcodes::PartyInvite as u16
    );
}

#[tokio::test]
async fn party_invite_server_uses_cpp_inviter_values_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send();
    let inviter = ObjectGuid::create_player(1, 42);
    let target = ObjectGuid::create_player(1, 77);
    let target_name = format!("Player{}", target.low_value());
    let inviter_name = "Leader";

    session.set_player_guid(Some(inviter));
    session.set_loaded_player_name_like_cpp(inviter_name.to_string());
    session.set_realm_handle_like_cpp(5, 6, 9);
    session.set_realm_names_like_cpp([(
        0x0506_0009,
        "Ice Crown".to_string(),
        "IceCrown".to_string(),
    )]);

    let player_registry = Arc::new(PlayerRegistry::default());
    let (target_tx, target_rx) = bounded(8);
    player_registry.insert(target, broadcast_info(target, target_tx));
    session.set_player_registry(player_registry);
    session.set_group_registry(
        Arc::new(GroupRegistry::default()),
        Arc::new(PendingInvites::default()),
    );

    session
        .handle_party_invite(party_invite_packet(target, &target_name, None, 0x12))
        .await;

    let invite = recv_dispatched_packet(&target_rx, "target invite packet");
    let mut packet = WorldPacket::from_bytes(&invite);
    assert_eq!(
        packet.read_uint16().expect("opcode"),
        ServerOpcodes::PartyInvite as u16
    );
    assert!(packet.read_bit().expect("can accept"));
    assert!(!packet.read_bit().expect("might CRZ"));
    assert!(!packet.read_bit().expect("is xrealm"));
    assert!(!packet.read_bit().expect("must be bnet friend"));
    assert!(!packet.read_bit().expect("allow multiple roles"));
    assert!(!packet.read_bit().expect("quest session active"));
    let name_len = packet.read_bits(6).expect("inviter name len") as usize;
    assert_eq!(packet.read_uint32().expect("realm address"), 0x0506_0009);
    assert!(packet.read_bit().expect("is local realm"));
    assert!(!packet.read_bit().expect("is internal realm"));
    let realm_len = packet.read_bits(8).expect("realm len") as usize;
    let realm_normalized_len = packet.read_bits(8).expect("realm normalized len") as usize;
    assert_eq!(
        packet.read_string(realm_len).expect("realm name"),
        "Ice Crown"
    );
    assert_eq!(
        packet
            .read_string(realm_normalized_len)
            .expect("normalized realm name"),
        "IceCrown"
    );
    assert_eq!(packet.read_packed_guid().expect("inviter guid"), inviter);
    assert_eq!(
        packet.read_packed_guid().expect("account guid"),
        ObjectGuid::create_global(HighGuid::WowAccount, 0, 1)
    );
    assert_eq!(packet.read_uint16().expect("unk1"), 0);
    assert_eq!(packet.read_uint8().expect("proposed roles"), 0x12);
    assert_eq!(packet.read_int32().expect("lfg slot count"), 0);
    assert_eq!(packet.read_int32().expect("lfg completed mask"), 0);
    assert_eq!(
        packet.read_string(name_len).expect("inviter name"),
        inviter_name
    );
    assert!(packet.is_empty());
}

#[tokio::test]
async fn party_invite_and_result_route_through_realm_like_cpp() {
    let (mut session, instance_rx) = make_session_with_send();
    let (realm_tx, realm_rx) = bounded(8);
    session.install_realm_send_channel_for_test(realm_tx);
    let inviter = ObjectGuid::create_player(1, 42);
    let target = ObjectGuid::create_player(1, 77);
    let target_name = format!("Player{}", target.low_value());
    session.set_player_guid(Some(inviter));
    session.set_loaded_player_name_like_cpp("Leader".to_string());

    let player_registry = Arc::new(PlayerRegistry::default());
    let (target_instance_tx, target_instance_rx) = bounded(8);
    let (target_command_tx, target_command_rx) = bounded(8);
    player_registry.insert(
        target,
        broadcast_info_with_command_tx(target, target_instance_tx, target_command_tx),
    );
    session.set_player_registry(player_registry);
    session.set_group_registry(
        Arc::new(GroupRegistry::default()),
        Arc::new(PendingInvites::default()),
    );

    session
        .handle_party_invite(party_invite_packet(target, &target_name, None, 0))
        .await;

    let SessionCommand::SendRealmPacketLikeCpp(command) =
        target_command_rx.try_recv().expect("remote realm command")
    else {
        panic!("expected remote realm packet command");
    };
    assert_eq!(command.recipient, target);
    assert_eq!(
        u16::from_le_bytes([command.packet_bytes[0], command.packet_bytes[1]]),
        ServerOpcodes::PartyInvite as u16
    );
    assert!(target_instance_rx.try_recv().is_err());

    let result = realm_rx.try_recv().expect("realm PartyCommandResult");
    assert_eq!(
        u16::from_le_bytes([result[0], result[1]]),
        ServerOpcodes::PartyCommandResult as u16
    );
    assert!(instance_rx.try_recv().is_err());
}

#[tokio::test]
async fn party_invite_waits_through_command_backpressure_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    let inviter = ObjectGuid::create_player(1, 42);
    let target = ObjectGuid::create_player(1, 77);
    let target_name = format!("Player{}", target.low_value());
    session.set_player_guid(Some(inviter));
    session.set_loaded_player_name_like_cpp("Leader".to_string());

    let player_registry = Arc::new(PlayerRegistry::default());
    let (target_send_tx, target_send_rx) = bounded(8);
    let (target_command_tx, target_command_rx) = bounded(1);
    target_command_tx
        .try_send(SessionCommand::SendRealmPacketLikeCpp(
            SendRealmPacketLikeCppCommand {
                recipient: target,
                packet_bytes: vec![0xAA],
            },
        ))
        .expect("fill target command queue");
    player_registry.insert(
        target,
        broadcast_info_with_command_tx(target, target_send_tx, target_command_tx.clone()),
    );
    let pending = Arc::new(PendingInvites::default());
    session.set_player_registry(player_registry);
    session.set_group_registry(Arc::new(GroupRegistry::default()), Arc::clone(&pending));

    let invite = session.handle_party_invite(party_invite_packet(target, &target_name, None, 0));
    tokio::pin!(invite);

    assert!(
        tokio::time::timeout(
            PARTY_REALM_COMMAND_TIMEOUT_LIKE_CPP + Duration::from_millis(50),
            &mut invite,
        )
        .await
        .is_err(),
        "temporary command backpressure must not be converted into a failed invite"
    );
    assert!(pending.get(&target).is_some());
    assert!(pending.get(&inviter).is_some());
    assert!(send_rx.try_recv().is_err());
    assert!(target_send_rx.try_recv().is_err());

    let SessionCommand::SendRealmPacketLikeCpp(blocker) = target_command_rx
        .try_recv()
        .expect("release target command capacity")
    else {
        panic!("expected command queue blocker");
    };
    assert_eq!(blocker.packet_bytes, vec![0xAA]);

    tokio::time::timeout(Duration::from_secs(1), &mut invite)
        .await
        .expect("invite resumes when the target command queue drains");

    let SessionCommand::SendRealmPacketLikeCpp(command) = target_command_rx
        .try_recv()
        .expect("queued party invite after backpressure")
    else {
        panic!("expected remote realm packet command");
    };
    assert_eq!(command.recipient, target);
    assert_eq!(
        u16::from_le_bytes([command.packet_bytes[0], command.packet_bytes[1]]),
        ServerOpcodes::PartyInvite as u16
    );

    assert_eq!(
        party_command_result_code(&send_rx.try_recv().expect("successful invite result")),
        party_result::OK
    );
    assert!(pending.get(&target).is_some());
    assert!(pending.get(&inviter).is_some());
    assert!(target_send_rx.try_recv().is_err());
}

#[tokio::test]
async fn party_invite_closed_command_channel_rolls_back_pending_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    let inviter = ObjectGuid::create_player(1, 42);
    let target = ObjectGuid::create_player(1, 77);
    let target_name = format!("Player{}", target.low_value());
    session.set_player_guid(Some(inviter));
    session.set_loaded_player_name_like_cpp("Leader".to_string());

    let player_registry = Arc::new(PlayerRegistry::default());
    let (target_send_tx, target_send_rx) = bounded(8);
    let (target_command_tx, target_command_rx) = bounded(1);
    drop(target_command_rx);
    player_registry.insert(
        target,
        broadcast_info_with_command_tx(target, target_send_tx, target_command_tx),
    );
    let pending = Arc::new(PendingInvites::default());
    session.set_player_registry(player_registry);
    session.set_group_registry(Arc::new(GroupRegistry::default()), Arc::clone(&pending));

    session
        .handle_party_invite(party_invite_packet(target, &target_name, None, 0))
        .await;

    assert_eq!(
        party_command_result_code(&send_rx.try_recv().expect("failed invite result")),
        party_result::BAD_PLAYER_NAME
    );
    assert!(pending.get(&target).is_none());
    assert!(pending.get(&inviter).is_none());
    assert!(target_send_rx.try_recv().is_err());
}

#[tokio::test]
async fn group_new_leader_fanout_queues_realm_commands_like_cpp() {
    let leader = ObjectGuid::create_player(1, 42);
    let member = ObjectGuid::create_player(1, 77);
    let mut group = GroupInfo::new(leader);
    group.add_member(member);
    let registry = PlayerRegistry::default();
    let (leader_instance_tx, leader_instance_rx) = bounded(4);
    let (leader_command_tx, leader_command_rx) = bounded(4);
    registry.insert(
        leader,
        broadcast_info_with_command_tx(leader, leader_instance_tx, leader_command_tx),
    );
    let (member_instance_tx, member_instance_rx) = bounded(4);
    let (member_command_tx, member_command_rx) = bounded(4);
    registry.insert(
        member,
        broadcast_info_with_command_tx(member, member_instance_tx, member_command_tx),
    );

    send_group_new_leader_like_cpp(&group, &registry, "NewLeader").await;

    for (expected, command_rx) in [(leader, leader_command_rx), (member, member_command_rx)] {
        let SessionCommand::SendRealmPacketLikeCpp(command) =
            command_rx.try_recv().expect("realm GroupNewLeader command")
        else {
            panic!("expected realm command");
        };
        assert_eq!(command.recipient, expected);
        assert_eq!(
            u16::from_le_bytes([command.packet_bytes[0], command.packet_bytes[1]]),
            ServerOpcodes::GroupNewLeader as u16
        );
    }
    assert!(leader_instance_rx.try_recv().is_err());
    assert!(member_instance_rx.try_recv().is_err());
}

#[tokio::test]
async fn party_invite_non_leader_rejects_not_leader_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    let leader = ObjectGuid::create_player(1, 42);
    let inviter = ObjectGuid::create_player(1, 43);
    let target = ObjectGuid::create_player(1, 77);
    let target_name = format!("Player{}", target.low_value());

    let player_registry = Arc::new(PlayerRegistry::default());
    let (target_tx, target_rx) = bounded(8);
    player_registry.insert(target, broadcast_info(target, target_tx));

    let group_registry = Arc::new(GroupRegistry::default());
    let mut group = GroupInfo::new(leader);
    group.add_member(inviter);
    let group_guid = group.group_guid;
    group_registry.register_group_like_cpp(group_guid, group);

    session.set_player_guid(Some(inviter));
    session.group_guid = Some(group_guid);
    session.set_player_registry(player_registry);
    session.set_group_registry(
        Arc::clone(&group_registry),
        Arc::new(PendingInvites::default()),
    );

    session
        .handle_party_invite(party_invite_packet(target, &target_name, None, 0))
        .await;

    assert_eq!(
        party_command_result_code(&send_rx.try_recv().expect("party command result")),
        party_result::NOT_LEADER
    );
    assert!(target_rx.try_recv().is_err());
}

#[tokio::test]
async fn party_invite_target_already_grouped_sends_already_and_cannot_accept_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    let inviter = ObjectGuid::create_player(1, 42);
    let target = ObjectGuid::create_player(1, 77);
    let target_name = format!("Player{}", target.low_value());
    let target_leader = ObjectGuid::create_player(1, 88);

    let player_registry = Arc::new(PlayerRegistry::default());
    let (target_tx, target_rx) = bounded(8);
    player_registry.insert(target, broadcast_info(target, target_tx));

    let group_registry = Arc::new(GroupRegistry::default());
    let mut target_group = GroupInfo::new(target_leader);
    target_group.add_member(target);
    group_registry.register_group_like_cpp(target_group.group_guid, target_group);
    let pending_invites = Arc::new(PendingInvites::default());

    session.set_player_guid(Some(inviter));
    session.set_player_registry(player_registry);
    session.set_group_registry(Arc::clone(&group_registry), Arc::clone(&pending_invites));

    session
        .handle_party_invite(party_invite_packet(target, &target_name, None, 0))
        .await;

    assert_eq!(
        party_command_result_code(&send_rx.try_recv().expect("party command result")),
        party_result::ALREADY_IN_GROUP
    );
    assert!(!party_invite_can_accept(&recv_dispatched_packet(
        &target_rx,
        "target failed invite packet"
    )));
    assert!(pending_invites.get(&target).is_none());
}

#[tokio::test]
async fn party_invite_raid_with_five_members_is_not_full_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send();
    let inviter = ObjectGuid::create_player(1, 42);
    let target = ObjectGuid::create_player(1, 77);
    let target_name = format!("Player{}", target.low_value());

    let player_registry = Arc::new(PlayerRegistry::default());
    let (target_tx, target_rx) = bounded(8);
    player_registry.insert(target, broadcast_info(target, target_tx));

    let group_registry = Arc::new(GroupRegistry::default());
    let mut group = GroupInfo::new(inviter);
    for counter in 43..47 {
        group.add_member(ObjectGuid::create_player(1, counter));
    }
    group.convert_to_raid_like_cpp();
    let group_guid = group.group_guid;
    group_registry.register_group_like_cpp(group_guid, group);
    let pending_invites = Arc::new(PendingInvites::default());

    session.set_player_guid(Some(inviter));
    session.group_guid = Some(group_guid);
    session.set_player_registry(player_registry);
    session.set_group_registry(Arc::clone(&group_registry), Arc::clone(&pending_invites));

    session
        .handle_party_invite(party_invite_packet(target, &target_name, None, 0))
        .await;

    assert!(pending_invites.get(&target).is_some());
    assert!(party_invite_can_accept(&recv_dispatched_packet(
        &target_rx,
        "target invite packet"
    )));
}

#[tokio::test]
async fn party_invite_rejects_gm_target_like_cpp_default_config() {
    let (mut session, send_rx) = make_session_with_send();
    let inviter = ObjectGuid::create_player(1, 42);
    let target = ObjectGuid::create_player(1, 77);
    let target_name = format!("Player{}", target.low_value());

    let player_registry = Arc::new(PlayerRegistry::default());
    let (target_tx, target_rx) = bounded(8);
    let mut target_info = broadcast_info(target, target_tx);
    target_info.is_game_master = true;
    player_registry.insert(target, target_info);
    let pending_invites = Arc::new(PendingInvites::default());

    session.set_player_guid(Some(inviter));
    session.set_loaded_player_identity_like_cpp(0, 1, 1, 80, 0);
    session.set_player_registry(player_registry);
    session.set_group_registry(
        Arc::new(GroupRegistry::default()),
        Arc::clone(&pending_invites),
    );

    session
        .handle_party_invite(party_invite_packet(target, &target_name, None, 0))
        .await;

    assert_eq!(
        party_command_result_code(&send_rx.try_recv().expect("party command result")),
        party_result::BAD_PLAYER_NAME
    );
    assert!(target_rx.try_recv().is_err());
    assert!(pending_invites.get(&target).is_none());
}

#[tokio::test]
async fn party_invite_rejects_cross_faction_like_cpp_default_config() {
    let (mut session, send_rx) = make_session_with_send();
    let inviter = ObjectGuid::create_player(1, 42);
    let target = ObjectGuid::create_player(1, 77);
    let target_name = format!("Player{}", target.low_value());

    let player_registry = Arc::new(PlayerRegistry::default());
    let (target_tx, target_rx) = bounded(8);
    let mut target_info = broadcast_info(target, target_tx);
    target_info.race = 2; // Orc/Horde, while inviter identity below is Human/Alliance.
    player_registry.insert(target, target_info);
    let pending_invites = Arc::new(PendingInvites::default());

    session.set_player_guid(Some(inviter));
    session.set_loaded_player_identity_like_cpp(0, 1, 1, 80, 0);
    session.set_player_registry(player_registry);
    session.set_group_registry(
        Arc::new(GroupRegistry::default()),
        Arc::clone(&pending_invites),
    );

    session
        .handle_party_invite(party_invite_packet(target, &target_name, None, 0))
        .await;

    assert_eq!(
        party_command_result_code(&send_rx.try_recv().expect("party command result")),
        party_result::WRONG_FACTION
    );
    assert!(target_rx.try_recv().is_err());
    assert!(pending_invites.get(&target).is_none());
}

#[tokio::test]
async fn party_invite_allows_gm_target_when_cpp_config_enabled() {
    let (mut session, _send_rx) = make_session_with_send();
    let inviter = ObjectGuid::create_player(1, 42);
    let target = ObjectGuid::create_player(1, 77);
    let target_name = format!("Player{}", target.low_value());

    let player_registry = Arc::new(PlayerRegistry::default());
    let (target_tx, target_rx) = bounded(8);
    let mut target_info = broadcast_info(target, target_tx);
    target_info.is_game_master = true;
    player_registry.insert(target, target_info);
    let pending_invites = Arc::new(PendingInvites::default());

    session.set_player_guid(Some(inviter));
    session.set_loaded_player_identity_like_cpp(0, 1, 1, 80, 0);
    session.set_allow_gm_group_like_cpp(true);
    session.set_player_registry(player_registry);
    session.set_group_registry(
        Arc::new(GroupRegistry::default()),
        Arc::clone(&pending_invites),
    );

    session
        .handle_party_invite(party_invite_packet(target, &target_name, None, 0))
        .await;

    assert!(pending_invites.get(&target).is_some());
    assert!(party_invite_can_accept(&recv_dispatched_packet(
        &target_rx,
        "target invite packet"
    )));
}

#[tokio::test]
async fn party_invite_allows_cross_faction_when_cpp_config_enabled() {
    let (mut session, _send_rx) = make_session_with_send();
    let inviter = ObjectGuid::create_player(1, 42);
    let target = ObjectGuid::create_player(1, 77);
    let target_name = format!("Player{}", target.low_value());

    let player_registry = Arc::new(PlayerRegistry::default());
    let (target_tx, target_rx) = bounded(8);
    let mut target_info = broadcast_info(target, target_tx);
    target_info.race = 2;
    player_registry.insert(target, target_info);
    let pending_invites = Arc::new(PendingInvites::default());

    session.set_player_guid(Some(inviter));
    session.set_loaded_player_identity_like_cpp(0, 1, 1, 80, 0);
    session.set_allow_two_side_interaction_group_like_cpp(true);
    session.set_player_registry(player_registry);
    session.set_group_registry(
        Arc::new(GroupRegistry::default()),
        Arc::clone(&pending_invites),
    );

    session
        .handle_party_invite(party_invite_packet(target, &target_name, None, 0))
        .await;

    assert!(pending_invites.get(&target).is_some());
    assert!(party_invite_can_accept(&recv_dispatched_packet(
        &target_rx,
        "target invite packet"
    )));
}

#[tokio::test]
async fn party_invite_rejects_low_level_non_friend_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    let inviter = ObjectGuid::create_player(1, 42);
    let target = ObjectGuid::create_player(1, 77);
    let target_name = format!("Player{}", target.low_value());

    let player_registry = Arc::new(PlayerRegistry::default());
    let (target_tx, target_rx) = bounded(8);
    player_registry.insert(target, broadcast_info(target, target_tx));
    let pending_invites = Arc::new(PendingInvites::default());

    session.set_player_guid(Some(inviter));
    session.set_loaded_player_identity_like_cpp(0, 1, 1, 1, 0);
    session.set_party_level_req_like_cpp(2);
    session.set_player_registry(player_registry);
    session.set_group_registry(
        Arc::new(GroupRegistry::default()),
        Arc::clone(&pending_invites),
    );

    session
        .handle_party_invite(party_invite_packet(target, &target_name, None, 0))
        .await;

    assert_eq!(
        party_command_result_code(&send_rx.try_recv().expect("party command result")),
        party_result::INVITE_RESTRICTED
    );
    assert!(target_rx.try_recv().is_err());
    assert!(pending_invites.get(&target).is_none());
}

#[test]
fn party_invite_social_matching_covers_guid_account_and_friend_like_cpp() {
    let inviter = ObjectGuid::create_player(1, 42);

    assert!(party_invite_social_ignore_match_like_cpp(
        inviter.counter(),
        7,
        SOCIAL_FLAG_IGNORED_LIKE_CPP,
        inviter,
        1
    ));
    assert!(party_invite_social_ignore_match_like_cpp(
        77,
        1,
        SOCIAL_FLAG_IGNORED_LIKE_CPP,
        inviter,
        1
    ));
    assert!(!party_invite_social_ignore_match_like_cpp(
        77,
        7,
        SOCIAL_FLAG_IGNORED_LIKE_CPP,
        inviter,
        1
    ));
    assert!(!party_invite_social_ignore_match_like_cpp(
        inviter.counter(),
        1,
        SOCIAL_FLAG_FRIEND_LIKE_CPP,
        inviter,
        1
    ));
    assert!(party_invite_social_friend_match_like_cpp(
        inviter.counter(),
        SOCIAL_FLAG_FRIEND_LIKE_CPP,
        inviter
    ));
    assert!(!party_invite_social_friend_match_like_cpp(
        77,
        SOCIAL_FLAG_FRIEND_LIKE_CPP,
        inviter
    ));
}

#[tokio::test]
async fn party_invite_rejects_same_map_different_instances_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    let inviter = ObjectGuid::create_player(1, 42);
    let target = ObjectGuid::create_player(1, 77);
    let target_name = format!("Player{}", target.low_value());

    let player_registry = Arc::new(PlayerRegistry::default());
    let (inviter_tx, _inviter_rx) = bounded(8);
    let mut inviter_info = broadcast_info(inviter, inviter_tx);
    inviter_info.map_id = 571;
    inviter_info.instance_id = 100;
    player_registry.insert(inviter, inviter_info);
    let (target_tx, target_rx) = bounded(8);
    let mut target_info = broadcast_info(target, target_tx);
    target_info.map_id = 571;
    target_info.instance_id = 200;
    player_registry.insert(target, target_info);
    let pending_invites = Arc::new(PendingInvites::default());

    session.set_player_guid(Some(inviter));
    session.set_loaded_player_identity_like_cpp(571, 1, 1, 80, 0);
    session.set_player_registry(player_registry);
    session.set_group_registry(
        Arc::new(GroupRegistry::default()),
        Arc::clone(&pending_invites),
    );

    session
        .handle_party_invite(party_invite_packet(target, &target_name, None, 0))
        .await;

    assert_eq!(
        party_command_result_code(&send_rx.try_recv().expect("party command result")),
        party_result::TARGET_NOT_IN_INSTANCE
    );
    assert!(target_rx.try_recv().is_err());
    assert!(pending_invites.get(&target).is_none());
}

#[tokio::test]
async fn party_invite_rejects_instance_difficulty_mismatch_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    let inviter = ObjectGuid::create_player(1, 42);
    let target = ObjectGuid::create_player(1, 77);
    let target_name = format!("Player{}", target.low_value());

    let player_registry = Arc::new(PlayerRegistry::default());
    let (target_tx, target_rx) = bounded(8);
    let mut target_info = broadcast_info(target, target_tx);
    target_info.map_id = 571;
    target_info.instance_id = 100;
    target_info.dungeon_difficulty_id = 2;
    player_registry.insert(target, target_info);
    let pending_invites = Arc::new(PendingInvites::default());

    session.set_player_guid(Some(inviter));
    session.set_loaded_player_identity_like_cpp(571, 1, 1, 80, 0);
    session.set_represented_dungeon_difficulty_id_for_test_like_cpp(1);
    session.set_player_registry(player_registry);
    session.set_group_registry(
        Arc::new(GroupRegistry::default()),
        Arc::clone(&pending_invites),
    );

    session
        .handle_party_invite(party_invite_packet(target, &target_name, None, 0))
        .await;

    assert_eq!(
        party_command_result_code(&send_rx.try_recv().expect("party command result")),
        party_result::IGNORING_YOU
    );
    assert!(target_rx.try_recv().is_err());
    assert!(pending_invites.get(&target).is_none());
}

#[tokio::test]
async fn party_invite_response_party_index_mismatch_keeps_invite_pending_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    let inviter = ObjectGuid::create_player(1, 42);
    let target = ObjectGuid::create_player(1, 77);

    let player_registry = Arc::new(PlayerRegistry::default());
    let group_registry = Arc::new(GroupRegistry::default());
    let home_group = GroupInfo::new(inviter);
    let home_group_guid = home_group.group_guid;
    group_registry.register_group_like_cpp(home_group_guid, home_group);

    let pending_invites = Arc::new(PendingInvites::default());
    pending_invites.seed_invite_like_cpp(
        target,
        PendingInviteLikeCpp::new_existing_group(
            inviter,
            home_group_guid,
            GROUP_CATEGORY_HOME_LIKE_CPP,
        ),
    );

    session.set_player_guid(Some(target));
    session.set_player_registry(player_registry);
    session.set_group_registry(Arc::clone(&group_registry), Arc::clone(&pending_invites));

    session
        .handle_party_invite_response(party_invite_response_packet(
            true,
            Some(wow_network::group_registry::GROUP_CATEGORY_INSTANCE_LIKE_CPP),
            None,
        ))
        .await;

    assert!(
        pending_invites.get(&target).is_some(),
        "C++ checks group category before removing the invite"
    );
    assert!(
        !group_registry
            .get(&home_group_guid)
            .unwrap()
            .members
            .contains(&target),
        "PartyIndex INSTANCE must not add the invitee to the HOME group"
    );
    assert!(send_rx.try_recv().is_err());
    assert!(session.group_guid.is_none());
}

#[tokio::test]
async fn party_invite_response_reports_group_full_without_adding_member_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    let leader = ObjectGuid::create_player(1, 42);
    let target = ObjectGuid::create_player(1, 77);

    let group_registry = Arc::new(GroupRegistry::default());
    let mut group = GroupInfo::new(leader);
    for counter in 43..47 {
        assert!(group.add_member(ObjectGuid::create_player(1, counter)));
    }
    let group_guid = group.group_guid;
    group_registry.register_group_like_cpp(group_guid, group);

    let pending_invites = Arc::new(PendingInvites::default());
    pending_invites.seed_invite_like_cpp(
        target,
        PendingInviteLikeCpp::new_existing_group(leader, group_guid, GROUP_CATEGORY_HOME_LIKE_CPP),
    );

    session.set_player_guid(Some(target));
    session.set_player_registry(Arc::new(PlayerRegistry::default()));
    session.set_group_registry(Arc::clone(&group_registry), Arc::clone(&pending_invites));

    session
        .handle_party_invite_response(party_invite_response_packet(true, None, None))
        .await;

    assert!(
        pending_invites.get(&target).is_none(),
        "C++ removes the invite before its full-group check"
    );
    assert_eq!(
        party_command_result_code(&send_rx.try_recv().expect("party command result")),
        party_result::GROUP_FULL
    );
    let group = group_registry
        .get(&group_guid)
        .expect("group remains registered");
    assert_eq!(group.members.len(), wow_network::MAX_GROUP_SIZE_LIKE_CPP);
    assert!(!group.members.contains(&target));
    assert!(session.group_guid.is_none());
}

#[tokio::test]
async fn party_invite_response_add_member_failure_returns_silently_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    let leader = ObjectGuid::create_player(1, 42);
    let target = ObjectGuid::create_player(1, 77);

    let group_registry = Arc::new(GroupRegistry::default());
    let mut group = GroupInfo::new(leader);
    group.convert_to_raid_like_cpp();
    group.raid_subgroup_counts = Some(
        [wow_network::MAX_GROUP_SIZE_LIKE_CPP as u8; wow_network::MAX_RAID_SUBGROUPS_LIKE_CPP],
    );
    let group_guid = group.group_guid;
    group_registry.register_group_like_cpp(group_guid, group);

    let pending_invites = Arc::new(PendingInvites::default());
    pending_invites.seed_invite_like_cpp(
        target,
        PendingInviteLikeCpp::new_existing_group(leader, group_guid, GROUP_CATEGORY_HOME_LIKE_CPP),
    );

    session.set_player_guid(Some(target));
    session.set_player_registry(Arc::new(PlayerRegistry::default()));
    session.set_group_registry(Arc::clone(&group_registry), Arc::clone(&pending_invites));

    session
        .handle_party_invite_response(party_invite_response_packet(true, None, None))
        .await;

    assert!(pending_invites.get(&target).is_none());
    assert!(
        send_rx.try_recv().is_err(),
        "C++ sends no GROUP_FULL packet when AddMember itself returns false"
    );
    let group = group_registry
        .get(&group_guid)
        .expect("group remains registered");
    assert_eq!(group.members, vec![leader]);
    assert!(!group.members.contains(&target));
    assert!(session.group_guid.is_none());
}

#[tokio::test]
async fn leave_group_party_index_instance_does_not_leave_home_group_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    let leaving_guid = ObjectGuid::create_player(1, 42);
    let other_guid = ObjectGuid::create_player(1, 77);

    let player_registry = Arc::new(PlayerRegistry::default());
    let group_registry = Arc::new(GroupRegistry::default());
    let mut home_group = GroupInfo::new(leaving_guid);
    home_group.add_member(other_guid);
    let home_group_guid = home_group.group_guid;
    group_registry.register_group_like_cpp(home_group_guid, home_group);

    session.set_player_guid(Some(leaving_guid));
    session.group_guid = Some(home_group_guid);
    session.set_player_registry(player_registry);
    session.set_group_registry(group_registry.clone(), Arc::new(PendingInvites::default()));

    session
        .handle_leave_group(leave_group_packet(Some(
            wow_network::group_registry::GROUP_CATEGORY_INSTANCE_LIKE_CPP,
        )))
        .await;

    assert!(
        group_registry
            .get(&home_group_guid)
            .unwrap()
            .members
            .contains(&leaving_guid),
        "PartyIndex INSTANCE must not resolve and leave the HOME group"
    );
    assert_eq!(session.group_guid, Some(home_group_guid));
    assert!(send_rx.try_recv().is_err());
}

fn lfg_group_like_cpp(leader: ObjectGuid, member_count: usize) -> GroupInfo {
    let mut group = GroupInfo::new(leader);
    for counter in 100..(100 + member_count as i64 - 1) {
        assert!(group.add_member(ObjectGuid::create_player(1, counter)));
    }
    group.group_flags |= wow_network::GROUP_FLAG_LFG_LIKE_CPP;
    group.lfg_kicks_left_like_cpp = wow_network::LFG_GROUP_MAX_KICKS_LIKE_CPP;
    group
}

fn lfg_uninvite_session_like_cpp(
    group: GroupInfo,
    sender_guid: ObjectGuid,
) -> (
    WorldSession,
    flume::Receiver<Vec<u8>>,
    Arc<GroupRegistry>,
    u64,
) {
    let (mut session, send_rx) = make_session_with_send();
    let group_registry = Arc::new(GroupRegistry::default());
    let group_guid = group.group_guid;
    group_registry.register_group_like_cpp(group_guid, group);
    session.set_player_guid(Some(sender_guid));
    session.group_guid = Some(group_guid);
    session.set_player_registry(Arc::new(PlayerRegistry::default()));
    session.set_group_registry(
        Arc::clone(&group_registry),
        Arc::new(PendingInvites::default()),
    );
    (session, send_rx, group_registry, group_guid)
}

#[tokio::test]
async fn lfg_uninvite_by_nonleader_passes_gate_but_kick_is_vote_owned_like_cpp() {
    // C++ `Player::CanUninviteFromGroup` LFG branch requires no
    // leader/assistant role, and `Group::RemoveMember` then returns
    // early for LFG + KICK (`Group.cpp:573-575`): the vote-kick scripts
    // own the removal, so a direct uninvite changes no membership.
    let leader = ObjectGuid::create_player(1, 42);
    let sender = ObjectGuid::create_player(1, 100);
    let target = ObjectGuid::create_player(1, 101);
    let group = lfg_group_like_cpp(leader, 5);
    let (mut session, send_rx, group_registry, group_guid) =
        lfg_uninvite_session_like_cpp(group, sender);

    session
        .handle_party_uninvite(party_uninvite_packet(target, None, "boot"))
        .await;

    assert!(
        group_registry
            .get(&group_guid)
            .expect("group")
            .members
            .contains(&target),
        "a passed LFG boot gate must not remove: C++ swallows direct kicks"
    );
    assert!(
        send_rx.try_recv().is_err(),
        "C++ sends no result packet when the gate passes"
    );
}

#[tokio::test]
async fn lfg_uninvite_boot_limit_returns_code_without_removal_like_cpp() {
    let leader = ObjectGuid::create_player(1, 42);
    let sender = ObjectGuid::create_player(1, 100);
    let target = ObjectGuid::create_player(1, 101);
    let mut group = lfg_group_like_cpp(leader, 5);
    group.lfg_kicks_left_like_cpp = 0;
    let (mut session, send_rx, group_registry, group_guid) =
        lfg_uninvite_session_like_cpp(group, sender);

    session
        .handle_party_uninvite(party_uninvite_packet(target, None, "boot"))
        .await;

    assert_eq!(
        party_command_result_code(&send_rx.try_recv().expect("boot limit result")),
        party_result::PARTY_LFG_BOOT_LIMIT
    );
    assert!(
        group_registry
            .get(&group_guid)
            .expect("group")
            .members
            .contains(&target)
    );
}

#[tokio::test]
async fn lfg_uninvite_too_few_players_returns_code_without_removal_like_cpp() {
    let leader = ObjectGuid::create_player(1, 42);
    let sender = ObjectGuid::create_player(1, 100);
    let target = ObjectGuid::create_player(1, 101);
    let group = lfg_group_like_cpp(leader, 3);
    let (mut session, send_rx, group_registry, group_guid) =
        lfg_uninvite_session_like_cpp(group, sender);

    session
        .handle_party_uninvite(party_uninvite_packet(target, None, "boot"))
        .await;

    assert_eq!(
        party_command_result_code(&send_rx.try_recv().expect("too few players result")),
        party_result::PARTY_LFG_BOOT_TOO_FEW_PLAYERS
    );
    assert!(
        group_registry
            .get(&group_guid)
            .expect("group")
            .members
            .contains(&target)
    );
}

#[tokio::test]
async fn lfg_uninvite_finished_dungeon_returns_code_without_removal_like_cpp() {
    let leader = ObjectGuid::create_player(1, 42);
    let sender = ObjectGuid::create_player(1, 100);
    let target = ObjectGuid::create_player(1, 101);
    let mut group = lfg_group_like_cpp(leader, 5);
    group.lfg_db_state = Some(wow_network::GroupLfgDbStateLikeCpp {
        dungeon_id: 100,
        state: Some(wow_network::LFG_STATE_FINISHED_DUNGEON_LIKE_CPP),
    });
    let (mut session, send_rx, group_registry, group_guid) =
        lfg_uninvite_session_like_cpp(group, sender);

    session
        .handle_party_uninvite(party_uninvite_packet(target, None, "boot"))
        .await;

    assert_eq!(
        party_command_result_code(&send_rx.try_recv().expect("dungeon complete result")),
        party_result::PARTY_LFG_BOOT_DUNGEON_COMPLETE
    );
    assert!(
        group_registry
            .get(&group_guid)
            .expect("group")
            .members
            .contains(&target)
    );
}

#[tokio::test]
async fn lfg_uninvite_target_loot_rolls_returns_code_without_removal_like_cpp() {
    let leader = ObjectGuid::create_player(1, 42);
    let sender = ObjectGuid::create_player(1, 100);
    let target = ObjectGuid::create_player(1, 101);
    let group = lfg_group_like_cpp(leader, 5);
    let (mut session, send_rx, group_registry, group_guid) =
        lfg_uninvite_session_like_cpp(group, sender);
    let player_registry = Arc::new(PlayerRegistry::default());
    let (target_tx, _target_rx) = flume::bounded(8);
    let mut target_info = broadcast_info(target, target_tx);
    target_info
        .active_loot_rolls
        .push(wow_network::LootRollCommandIdentityLikeCpp::new_like_cpp(
            ObjectGuid::create_item(1, 9001),
            1,
            wow_loot::OwnedLootAuthority::default(),
            1,
        ));
    player_registry.insert(target, target_info);
    session.set_player_registry(player_registry);

    session
        .handle_party_uninvite(party_uninvite_packet(target, None, "boot"))
        .await;

    assert_eq!(
        party_command_result_code(&send_rx.try_recv().expect("loot rolls result")),
        party_result::PARTY_LFG_BOOT_LOOT_ROLLS
    );
    assert!(
        group_registry
            .get(&group_guid)
            .expect("group")
            .members
            .contains(&target)
    );
}

#[tokio::test]
async fn lfg_uninvite_in_combat_returns_code_without_removal_like_cpp() {
    let leader = ObjectGuid::create_player(1, 42);
    let sender = ObjectGuid::create_player(1, 100);
    let target = ObjectGuid::create_player(1, 101);
    let group = lfg_group_like_cpp(leader, 5);
    let (mut session, send_rx, group_registry, group_guid) =
        lfg_uninvite_session_like_cpp(group, sender);
    session.in_combat = true;

    session
        .handle_party_uninvite(party_uninvite_packet(target, None, "boot"))
        .await;

    assert_eq!(
        party_command_result_code(&send_rx.try_recv().expect("in combat result")),
        party_result::PARTY_LFG_BOOT_IN_COMBAT
    );
    assert!(
        group_registry
            .get(&group_guid)
            .expect("group")
            .members
            .contains(&target)
    );
}

#[tokio::test]
async fn lfg_uninvite_member_in_combat_returns_code_without_removal_like_cpp() {
    // C++ checks every member on the uninviter's map: another member in
    // combat blocks the kick even when the uninviter is at peace.
    let leader = ObjectGuid::create_player(1, 42);
    let sender = ObjectGuid::create_player(1, 100);
    let target = ObjectGuid::create_player(1, 101);
    let group = lfg_group_like_cpp(leader, 5);
    let (mut session, send_rx, group_registry, group_guid) =
        lfg_uninvite_session_like_cpp(group, sender);
    let player_registry = Arc::new(PlayerRegistry::default());
    let (target_tx, _target_rx) = flume::bounded(8);
    let mut target_info = broadcast_info(target, target_tx);
    target_info.in_combat = true;
    player_registry.insert(target, target_info);
    session.set_player_registry(Arc::clone(&player_registry));
    assert!(!session.in_combat);

    session
        .handle_party_uninvite(party_uninvite_packet(target, None, "boot"))
        .await;

    assert_eq!(
        party_command_result_code(&send_rx.try_recv().expect("in combat result")),
        party_result::PARTY_LFG_BOOT_IN_COMBAT
    );
    assert!(
        group_registry
            .get(&group_guid)
            .expect("group")
            .members
            .contains(&target)
    );
}

#[tokio::test]
async fn lfg_uninvite_member_in_combat_on_another_map_passes_gate_like_cpp() {
    // C++ only counts members in the uninviter's map (`IsInMap(this)`):
    // a member fighting on another map does not block the kick.
    let leader = ObjectGuid::create_player(1, 42);
    let sender = ObjectGuid::create_player(1, 100);
    let target = ObjectGuid::create_player(1, 101);
    let group = lfg_group_like_cpp(leader, 5);
    let (mut session, send_rx, group_registry, group_guid) =
        lfg_uninvite_session_like_cpp(group, sender);
    let player_registry = Arc::new(PlayerRegistry::default());
    let (target_tx, _target_rx) = flume::bounded(8);
    let mut target_info = broadcast_info(target, target_tx);
    target_info.in_combat = true;
    target_info.map_id = 1;
    target_info.instance_id = 1;
    player_registry.insert(target, target_info);
    session.set_player_registry(Arc::clone(&player_registry));

    session
        .handle_party_uninvite(party_uninvite_packet(target, None, "boot"))
        .await;

    assert!(
        send_rx.try_recv().is_err(),
        "C++ ignores combat on other maps: the gate passes silently into the vote-owned swallow"
    );
    assert!(
        group_registry
            .get(&group_guid)
            .expect("group")
            .members
            .contains(&target)
    );
}

#[tokio::test]
async fn session_combat_transition_updates_the_registry_member_view_like_cpp() {
    // The LFG combat gate can only read members if their registry view
    // tracks the owning session's transitions.
    let guid = ObjectGuid::create_player(1, 42);
    let (mut session, _send_rx) = make_session_with_send();
    session.set_player_guid(Some(guid));
    let player_registry = Arc::new(PlayerRegistry::default());
    let (tx, _rx) = flume::bounded(8);
    player_registry.insert(guid, broadcast_info(guid, tx));
    session.set_player_registry(Arc::clone(&player_registry));

    session.set_in_combat_like_cpp(true);
    assert!(player_registry.get(&guid).expect("member").in_combat);
    session.set_in_combat_like_cpp(false);
    assert!(!player_registry.get(&guid).expect("member").in_combat);
}

#[tokio::test]
async fn lfg_uninvite_against_leader_passes_gate_without_removal_like_cpp() {
    // C++'s LFG branch has no `IsLeader(guidMember)` rejection, and the
    // same swallowed-kick rule leaves the leader in place: no stale
    // `leader_guid` can ever be produced by this path.
    let leader = ObjectGuid::create_player(1, 42);
    let sender = ObjectGuid::create_player(1, 100);
    let group = lfg_group_like_cpp(leader, 5);
    let (mut session, send_rx, group_registry, group_guid) =
        lfg_uninvite_session_like_cpp(group, sender);

    session
        .handle_party_uninvite(party_uninvite_packet(leader, None, "boot"))
        .await;

    let group = group_registry.get(&group_guid).expect("group");
    assert!(group.members.contains(&leader));
    assert_eq!(group.leader_guid, leader);
    let _ = send_rx;
}

#[tokio::test]
async fn normal_group_uninvite_in_battleground_returns_invite_restricted_like_cpp() {
    // C++ `CanUninviteFromGroup` normal branch returns
    // `ERR_INVITE_RESTRICTED` when the sender is in a battleground
    // (`Player.cpp:25181-25182`).
    let leader = ObjectGuid::create_player(1, 42);
    let target = ObjectGuid::create_player(1, 100);
    let mut group = GroupInfo::new(leader);
    assert!(group.add_member(target));
    let (mut session, send_rx, group_registry, group_guid) =
        lfg_uninvite_session_like_cpp(group, leader);
    session.set_player_battleground_type_id_like_cpp(1);

    session
        .handle_party_uninvite(party_uninvite_packet(target, None, "bye"))
        .await;

    assert_eq!(
        party_command_result_code(&send_rx.try_recv().expect("invite restricted result")),
        party_result::INVITE_RESTRICTED
    );
    assert!(
        group_registry
            .get(&group_guid)
            .expect("group")
            .members
            .contains(&target)
    );
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
    group_registry.register_group_like_cpp(group_guid, group);

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

    let leader_update = recv_dispatched_packet(&leader_rx, "leader party update");
    assert_eq!(
        u16::from_le_bytes([leader_update[0], leader_update[1]]),
        ServerOpcodes::PartyUpdate as u16
    );
    let remaining_update = recv_dispatched_packet(&remaining_rx, "remaining party update");
    assert_eq!(
        u16::from_le_bytes([remaining_update[0], remaining_update[1]]),
        ServerOpcodes::PartyUpdate as u16
    );
}

#[tokio::test]
async fn party_uninvite_disband_sends_destroyed_party_update_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    let leader = ObjectGuid::create_player(1, 42);
    let target = ObjectGuid::create_player(1, 77);
    let (leader_tx, _leader_rx) = bounded(8);
    let (target_tx, _target_rx) = bounded(8);
    let (target_command_tx, target_command_rx) = bounded(8);
    let player_registry = Arc::new(PlayerRegistry::default());
    player_registry.insert(leader, broadcast_info(leader, leader_tx));
    player_registry.insert(
        target,
        broadcast_info_with_command_tx(target, target_tx, target_command_tx),
    );
    let group_registry = Arc::new(GroupRegistry::default());
    let mut group = GroupInfo::new(leader);
    group.add_member(target);
    let group_guid = group.group_guid;
    group_registry.register_group_like_cpp(group_guid, group);

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

    // C++ `Group::RemoveMember` disbands a two-member group instead of
    // keeping it alive (`Group.cpp:660-663`).
    assert!(group_registry.get(&group_guid).is_none());
    assert_eq!(session.group_guid, None);

    let command = target_command_rx.try_recv().unwrap();
    let SessionCommand::ApplyGroupRemovalLikeCpp(command) = command else {
        panic!("expected ApplyGroupRemovalLikeCpp for kicked member");
    };
    assert!(command.send_group_destroyed);
    assert!(!command.send_group_uninvite);

    // C++ `Group::Disband` sends the leader `GroupDestroyed` and then the
    // destroyed `PartyUpdate` (`Group.cpp:744-746`).
    let mut destroyed_index = None;
    let mut update_index = None;
    let mut destroyed_update = None;
    let mut index = 0usize;
    while let Ok(bytes) = send_rx.try_recv() {
        let opcode = WorldPacket::from_bytes(&bytes).server_opcode();
        if opcode == Some(ServerOpcodes::GroupDestroyed) {
            destroyed_index = Some(index);
        }
        if opcode == Some(ServerOpcodes::PartyUpdate) {
            update_index = Some(index);
            destroyed_update = Some(bytes);
        }
        index += 1;
    }
    let destroyed_index = destroyed_index.expect("leader GroupDestroyed");
    let update_index = update_index.expect("leader destroyed PartyUpdate");
    assert!(destroyed_index < update_index);

    let mut packet = WorldPacket::from_bytes(&destroyed_update.unwrap());
    assert_eq!(
        packet.read_uint16().expect("opcode"),
        ServerOpcodes::PartyUpdate as u16
    );
    assert_eq!(
        packet.read_uint16().expect("party flags"),
        wow_network::group_registry::GROUP_FLAG_DESTROYED_LIKE_CPP
    );
    assert_eq!(
        packet.read_uint8().expect("party index"),
        GROUP_CATEGORY_HOME_LIKE_CPP
    );
    assert_eq!(
        packet.read_uint8().expect("party type"),
        wow_network::group_registry::GROUP_TYPE_NONE_LIKE_CPP
    );
    assert_eq!(packet.read_int32().expect("my index"), -1);
    assert_eq!(
        packet.read_packed_guid().expect("party guid"),
        ObjectGuid::create_group(group_guid)
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
    group_registry.register_group_like_cpp(group_guid, group);

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
    assert_eq!(result_code as u8, party_result::NOT_LEADER);
    payload.flush_bits();
    assert_eq!(payload.read_uint32().unwrap(), 0); // C++ ResultData
    // C++ `WorldSession::SendPartyResult` always leaves `ResultGUID`
    // empty (`GroupHandler.cpp:53`).
    assert_eq!(payload.read_packed_guid().unwrap(), ObjectGuid::EMPTY);
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn party_uninvite_removes_pending_group_invite_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    let leader = ObjectGuid::create_player(1, 42);
    let inviter = ObjectGuid::create_player(1, 77);
    let target = ObjectGuid::create_player(1, 88);
    let player_registry = Arc::new(PlayerRegistry::default());
    let group_registry = Arc::new(GroupRegistry::default());
    let pending_invites = Arc::new(PendingInvites::default());
    let mut group = GroupInfo::new(leader);
    group.add_member(inviter);
    let group_guid = group.group_guid;
    group_registry.register_group_like_cpp(group_guid, group);
    pending_invites.seed_invite_like_cpp(
        target,
        PendingInviteLikeCpp::new_existing_group(leader, group_guid, GROUP_CATEGORY_HOME_LIKE_CPP),
    );

    session.set_player_guid(Some(leader));
    session.group_guid = Some(group_guid);
    session.set_player_registry(player_registry);
    session.set_group_registry(Arc::clone(&group_registry), Arc::clone(&pending_invites));

    session
        .handle_party_uninvite(party_uninvite_packet(target, None, "revoked"))
        .await;

    assert!(
        pending_invites.get(&target).is_none(),
        "C++ Player::UninviteFromGroup clears the pending group invite"
    );
    let group = group_registry.get(&group_guid).unwrap();
    assert!(group.members.contains(&leader));
    assert!(group.members.contains(&inviter));
    assert!(!group.members.contains(&target));
    drop(group);
    assert!(
        send_rx.try_recv().is_err(),
        "pending invite removal returns without ERR_TARGET_NOT_IN_GROUP_S"
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

    let _party_update = recv_dispatched_packet(&leader_rx, "leader PartyUpdate");
    let full_state = recv_dispatched_packet(&leader_rx, "leader PartyMemberFullState");
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
    group_registry.register_group_like_cpp(group_guid, group);

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
    group_registry.register_group_like_cpp(group_guid, group);
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
    group_registry.register_group_like_cpp(group_guid, group);
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
    group_registry.register_group_like_cpp(group_guid, group);
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
        .set_member_flag_transition_like_cpp(
            group_guid,
            leader,
            assistant,
            true,
            wow_network::MEMBER_FLAG_ASSISTANT_LIKE_CPP,
        )
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
    group_registry.register_group_like_cpp(group_guid, group);
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
    group_registry.register_group_like_cpp(group_guid, group);
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
async fn party_join_updates_sends_target_list_and_raid_markers_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    let leader = ObjectGuid::create_player(1, 42);
    let marked = ObjectGuid::create_player(1, 77);
    let marker_position = Position::xyz(12.25, -34.5, 6.75);
    let group_registry = Arc::new(GroupRegistry::default());
    let mut group = GroupInfo::new(leader);
    group.target_icons[6] = marked.to_raw_bytes();
    group.add_raid_marker_like_cpp(3, 571, marker_position, ObjectGuid::EMPTY);
    let group_guid = group.group_guid;
    group_registry.register_group_like_cpp(group_guid, group);

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
    let markers = send_rx.try_recv().expect("raid markers");
    assert_raid_markers_packet_like_cpp(&markers, 1 << 3, &[marker_position]);
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn clear_raid_marker_removes_one_slot_and_fanouts_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send();
    let leader = ObjectGuid::create_player(1, 42);
    let member = ObjectGuid::create_player(1, 43);
    let remaining_position = Position::xyz(4.0, 5.0, 6.0);
    let group_registry = Arc::new(GroupRegistry::default());
    let player_registry = Arc::new(PlayerRegistry::default());
    let (leader_tx, leader_rx) = bounded::<Vec<u8>>(4);
    let (member_tx, member_rx) = bounded::<Vec<u8>>(4);
    let mut group = GroupInfo::new(leader);
    group.add_member(member);
    group.add_raid_marker_like_cpp(1, 571, Position::xyz(1.0, 2.0, 3.0), ObjectGuid::EMPTY);
    group.add_raid_marker_like_cpp(3, 571, remaining_position, ObjectGuid::EMPTY);
    let group_guid = group.group_guid;
    group_registry.register_group_like_cpp(group_guid, group);
    player_registry.insert(leader, broadcast_info(leader, leader_tx));
    player_registry.insert(member, broadcast_info(member, member_tx));

    session.set_player_guid(Some(leader));
    session.group_guid = Some(group_guid);
    session.set_group_registry(
        Arc::clone(&group_registry),
        Arc::new(PendingInvites::default()),
    );
    session.set_player_registry(player_registry);

    session
        .handle_clear_raid_marker(clear_raid_marker_packet(1))
        .await;

    assert_eq!(
        group_registry
            .get(&group_guid)
            .unwrap()
            .active_raid_markers_mask_like_cpp(),
        1 << 3
    );
    for rx in [&leader_rx, &member_rx] {
        let bytes = rx.try_recv().expect("marker changed");
        assert_raid_markers_packet_like_cpp(&bytes, 1 << 3, &[remaining_position]);
    }
}

#[tokio::test]
async fn clear_raid_marker_id_eight_removes_all_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send();
    let leader = ObjectGuid::create_player(1, 42);
    let group_registry = Arc::new(GroupRegistry::default());
    let player_registry = Arc::new(PlayerRegistry::default());
    let (leader_tx, leader_rx) = bounded::<Vec<u8>>(4);
    let mut group = GroupInfo::new(leader);
    group.add_raid_marker_like_cpp(1, 571, Position::xyz(1.0, 2.0, 3.0), ObjectGuid::EMPTY);
    group.add_raid_marker_like_cpp(3, 571, Position::xyz(4.0, 5.0, 6.0), ObjectGuid::EMPTY);
    let group_guid = group.group_guid;
    group_registry.register_group_like_cpp(group_guid, group);
    player_registry.insert(leader, broadcast_info(leader, leader_tx));

    session.set_player_guid(Some(leader));
    session.group_guid = Some(group_guid);
    session.set_group_registry(
        Arc::clone(&group_registry),
        Arc::new(PendingInvites::default()),
    );
    session.set_player_registry(player_registry);

    session
        .handle_clear_raid_marker(clear_raid_marker_packet(8))
        .await;

    assert_eq!(
        group_registry
            .get(&group_guid)
            .unwrap()
            .active_raid_markers_mask_like_cpp(),
        0
    );
    let bytes = leader_rx.try_recv().expect("marker changed");
    assert_raid_markers_packet_like_cpp(&bytes, 0, &[]);
}

#[tokio::test]
async fn clear_raid_marker_raid_requires_leader_or_assistant_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send();
    let leader = ObjectGuid::create_player(1, 42);
    let member = ObjectGuid::create_player(1, 43);
    let group_registry = Arc::new(GroupRegistry::default());
    let player_registry = Arc::new(PlayerRegistry::default());
    let (member_tx, member_rx) = bounded::<Vec<u8>>(4);
    let mut group = GroupInfo::new(leader);
    group.convert_to_raid_like_cpp();
    group.add_member(member);
    group.add_raid_marker_like_cpp(3, 571, Position::xyz(4.0, 5.0, 6.0), ObjectGuid::EMPTY);
    let group_guid = group.group_guid;
    group_registry.register_group_like_cpp(group_guid, group);
    player_registry.insert(member, broadcast_info(member, member_tx));

    session.set_player_guid(Some(member));
    session.group_guid = Some(group_guid);
    session.set_group_registry(
        Arc::clone(&group_registry),
        Arc::new(PendingInvites::default()),
    );
    session.set_player_registry(player_registry);

    session
        .handle_clear_raid_marker(clear_raid_marker_packet(3))
        .await;

    assert_eq!(
        group_registry
            .get(&group_guid)
            .unwrap()
            .active_raid_markers_mask_like_cpp(),
        1 << 3
    );
    assert!(member_rx.try_recv().is_err());
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
async fn request_party_member_stats_routes_reply_through_realm_like_cpp() {
    let (mut session, instance_rx) = make_session_with_send();
    let (realm_tx, realm_rx) = bounded(4);
    session.install_realm_send_channel_for_test(realm_tx);
    let target = ObjectGuid::create_player(1, 77);
    session.set_player_registry(Arc::new(PlayerRegistry::default()));

    session
        .handle_request_party_member_stats(request_party_member_stats_packet(target, None))
        .await;

    let packet = realm_rx.try_recv().expect("realm PartyMemberFullState");
    assert_eq!(
        u16::from_le_bytes([packet[0], packet[1]]),
        ServerOpcodes::PartyMemberFullState as u16
    );
    assert!(instance_rx.try_recv().is_err());
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
    group_registry.register_group_like_cpp(group_guid, group);

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

    let leader_update = recv_dispatched_packet(&leader_rx, "leader PartyUpdate after SetLfgRoles");
    let member_update = recv_dispatched_packet(&member_rx, "member PartyUpdate after SetLfgRoles");
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
    group_registry.register_group_like_cpp(group_guid, group);
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
    group_registry.register_group_like_cpp(group_guid, group);
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
    group_registry.register_group_like_cpp(group_guid, group);
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
    group_registry.register_group_like_cpp(group_guid, group);
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
    group_registry.register_group_like_cpp(group_guid, group);
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
    group_registry.register_group_like_cpp(group_guid, group);

    let player_registry = Arc::new(PlayerRegistry::default());
    let (leader_tx, leader_rx) = bounded(8);
    let (member_tx, _member_rx) = bounded(8);
    let (member_command_tx, member_command_rx) =
        test_session_command_dispatcher(member, member_tx.clone());
    player_registry.insert(leader, broadcast_info(leader, leader_tx));
    player_registry.insert(
        member,
        broadcast_info_with_command_tx(member, member_tx, member_command_tx),
    );

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
    let remote_refresh = member_command_rx
        .recv_timeout(Duration::from_secs(1))
        .expect("remote member visible refresh command queued");
    assert!(matches!(
        remote_refresh,
        SessionCommand::RefreshVisibleGameobjectsOrSpellClicksLikeCpp
    ));
    let command_result = send_rx.try_recv().expect("party command result");
    assert_eq!(
        u16::from_le_bytes([command_result[0], command_result[1]]),
        ServerOpcodes::PartyCommandResult as u16
    );
    assert!(send_rx.try_recv().is_err());
    let party_update = recv_dispatched_packet(&leader_rx, "leader party update");
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
async fn convert_raid_releases_group_guard_before_refresh_backpressure_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send();
    let leader = ObjectGuid::create_player(1, 142);
    let member = ObjectGuid::create_player(1, 143);
    let group_registry = Arc::new(GroupRegistry::default());
    let mut group = GroupInfo::new(leader);
    group.add_member(member);
    let group_guid = group.group_guid;
    group_registry.register_group_like_cpp(group_guid, group);

    let player_registry = Arc::new(PlayerRegistry::default());
    let (leader_tx, _leader_rx) = bounded(8);
    player_registry.insert(leader, broadcast_info(leader, leader_tx));
    let (member_tx, _member_rx) = bounded(8);
    // PartyUpdate fills this single slot; the following async refresh then
    // waits for its timeout and exposes any DashMap guard held across it.
    let (member_command_tx, _member_command_rx) = flume::bounded(1);
    player_registry.insert(
        member,
        broadcast_info_with_command_tx(member, member_tx, member_command_tx),
    );

    session.set_player_guid(Some(leader));
    session.group_guid = Some(group_guid);
    session.set_player_registry(player_registry);
    session.set_group_registry(
        Arc::clone(&group_registry),
        Arc::new(PendingInvites::default()),
    );

    let mut conversion = Box::pin(session.handle_convert_raid(convert_raid_packet(true)));
    tokio::select! {
        () = &mut conversion => panic!("refresh should be waiting on the full command channel"),
        () = tokio::time::sleep(Duration::from_millis(20)) => {}
    }

    let writer_registry = Arc::clone(&group_registry);
    let writer = tokio::task::spawn_blocking(move || {
        writer_registry
            .set_everyone_assistant_transition_like_cpp(group_guid, leader, true)
            .ok()
            .map(|outcome| outcome.group.group_flags)
    });
    let observed_flags = tokio::time::timeout(Duration::from_secs(1), writer)
        .await
        .expect("the group write must not wait for refresh channel backpressure")
        .expect("group writer task should not panic")
        .expect("converted group should remain registered");
    assert_ne!(observed_flags & wow_network::GROUP_FLAG_RAID_LIKE_CPP, 0);

    conversion.await;
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
    group_registry.register_group_like_cpp(group_guid, group);

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
    group_registry.register_group_like_cpp(group_guid, group);

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
    let leader_update = recv_dispatched_packet(&leader_rx, "leader party update");
    assert_eq!(
        u16::from_le_bytes([leader_update[0], leader_update[1]]),
        ServerOpcodes::PartyUpdate as u16
    );
    let member_update = recv_dispatched_packet(&member_rx, "member party update");
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
    group_registry.register_group_like_cpp(group_guid, group);

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

    group_registry
        .set_member_flag_transition_like_cpp(
            group_guid,
            leader,
            assistant,
            true,
            wow_network::MEMBER_FLAG_ASSISTANT_LIKE_CPP,
        )
        .unwrap();

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
    group_registry.register_group_like_cpp(group_guid, group);

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
    let leader_update = recv_dispatched_packet(&leader_rx, "leader party update");
    assert_eq!(
        u16::from_le_bytes([leader_update[0], leader_update[1]]),
        ServerOpcodes::PartyUpdate as u16
    );
    let member_update = recv_dispatched_packet(&member_rx, "member party update");
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
    group_registry.register_group_like_cpp(group_guid, group);

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
    let _ = recv_dispatched_packet(&leader_rx, "leader party update");
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
    group_registry.register_group_like_cpp(group_guid, group);

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
    group_registry.register_group_like_cpp(group_guid, group);

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
    let _ = recv_dispatched_packet(&leader_rx, "leader party update");
    let _ = recv_dispatched_packet(&member_rx, "member party update");

    group_registry
        .convert_group_like_cpp(group_guid, leader, true)
        .unwrap();
    group_registry
        .set_member_flag_transition_like_cpp(
            group_guid,
            leader,
            member,
            true,
            wow_network::MEMBER_FLAG_MAINTANK_LIKE_CPP,
        )
        .unwrap();
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
    let _ = recv_dispatched_packet(&leader_rx, "leader party update");
    let _ = recv_dispatched_packet(&member_rx, "member party update");
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
    group_registry.register_group_like_cpp(group_guid, group);

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
    let _ = recv_dispatched_packet(&leader_rx, "leader party update");
    let _ = recv_dispatched_packet(&member_rx, "member party update");
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
    group_registry.register_group_like_cpp(group_guid, group);

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
    let leader_update = recv_dispatched_packet(&leader_rx, "leader party update");
    assert_eq!(
        u16::from_le_bytes([leader_update[0], leader_update[1]]),
        ServerOpcodes::PartyUpdate as u16
    );
    let member_update = recv_dispatched_packet(&member_rx, "member party update");
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
    group_registry.register_group_like_cpp(group_guid, group);

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
    let _ = recv_dispatched_packet(&leader_rx, "leader party update");
    let _ = recv_dispatched_packet(&member_rx, "member party update");
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
    group_registry.register_group_like_cpp(group_guid, group);

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
    group_registry.register_group_like_cpp(group_guid, group);

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
    group_registry.register_group_like_cpp(group_guid, group);

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
    group_registry.register_group_like_cpp(group_guid, group);

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
    let _ = recv_dispatched_packet(&leader_rx, "leader party update");
    let _ = recv_dispatched_packet(&member_rx, "member party update");
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
    group_registry.register_group_like_cpp(group_guid, group);

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
    let leader_update = recv_dispatched_packet(&leader_rx, "leader party update");
    assert_eq!(
        u16::from_le_bytes([leader_update[0], leader_update[1]]),
        ServerOpcodes::PartyUpdate as u16
    );
    let member_update = recv_dispatched_packet(&member_rx, "member party update");
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
    group_registry.register_group_like_cpp(group_guid, group);

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

    let leader_new_leader = recv_dispatched_packet(&leader_rx, "leader new-leader packet");
    assert_eq!(
        u16::from_le_bytes([leader_new_leader[0], leader_new_leader[1]]),
        ServerOpcodes::GroupNewLeader as u16
    );
    let leader_update = recv_dispatched_packet(&leader_rx, "leader party update");
    assert_eq!(
        u16::from_le_bytes([leader_update[0], leader_update[1]]),
        ServerOpcodes::PartyUpdate as u16
    );
    let member_new_leader = recv_dispatched_packet(&member_rx, "member new-leader packet");
    assert_eq!(
        u16::from_le_bytes([member_new_leader[0], member_new_leader[1]]),
        ServerOpcodes::GroupNewLeader as u16
    );
    let member_update = recv_dispatched_packet(&member_rx, "member party update");
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
    group_registry.register_group_like_cpp(group_guid, group);

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
    group_registry.register_group_like_cpp(group_guid, group);

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
    group_registry.register_group_like_cpp(group_guid, group);

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
        .convert_group_like_cpp(group_guid, leader, true)
        .unwrap();
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
    group_registry.register_group_like_cpp(group_guid, group);

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
    let leader_update = recv_dispatched_packet(&leader_rx, "leader party update");
    assert_eq!(
        u16::from_le_bytes([leader_update[0], leader_update[1]]),
        ServerOpcodes::PartyUpdate as u16
    );
    let first_update = recv_dispatched_packet(&first_rx, "first member party update");
    assert_eq!(
        u16::from_le_bytes([first_update[0], first_update[1]]),
        ServerOpcodes::PartyUpdate as u16
    );
    let second_update = recv_dispatched_packet(&second_rx, "second member party update");
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
    group_registry.register_group_like_cpp(group_guid, group);

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

    group_registry
        .set_member_flag_transition_like_cpp(
            group_guid,
            leader,
            assistant,
            true,
            wow_network::MEMBER_FLAG_ASSISTANT_LIKE_CPP,
        )
        .unwrap();

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
    let first_update = recv_dispatched_packet(&first_rx, "first update after assistant swap");
    assert_eq!(
        u16::from_le_bytes([first_update[0], first_update[1]]),
        ServerOpcodes::PartyUpdate as u16
    );
    let second_update = recv_dispatched_packet(&second_rx, "second update after assistant swap");
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
    group_registry.register_group_like_cpp(group_guid, group);

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

fn random_roll_packet(min: i32, max: i32, party_index: Option<u8>) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_bit(party_index.is_some());
    pkt.write_int32(min);
    pkt.write_int32(max);
    if let Some(idx) = party_index {
        pkt.write_uint8(idx);
    }
    pkt.reset_read();
    pkt
}

fn assert_random_roll_packet(
    bytes: &[u8],
    roller: ObjectGuid,
    account_id: u32,
    min: i32,
    max: i32,
) -> i32 {
    let mut pkt = WorldPacket::from_bytes(bytes);
    assert_eq!(pkt.read_uint16().unwrap(), ServerOpcodes::RandomRoll as u16);
    assert_eq!(pkt.read_guid().unwrap(), roller);
    assert_eq!(
        pkt.read_guid().unwrap(),
        ObjectGuid::new((HighGuid::WowAccount as i64) << 58, i64::from(account_id))
    );
    assert_eq!(pkt.read_int32().unwrap(), min);
    assert_eq!(pkt.read_int32().unwrap(), max);
    let result = pkt.read_int32().unwrap();
    assert!((min..=max).contains(&result));
    assert_eq!(pkt.remaining(), 0);
    result
}

#[tokio::test]
async fn random_roll_without_group_sends_to_self_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    let sender = ObjectGuid::create_player(1, 42);
    session.set_player_guid(Some(sender));

    session
        .handle_random_roll(random_roll_packet(1, 100, None))
        .await;

    let sent = send_rx
        .try_recv()
        .expect("solo random roll should be sent to self");
    assert_random_roll_packet(&sent, sender, 1, 1, 100);
}

#[tokio::test]
async fn random_roll_with_group_broadcasts_to_all_members_including_sender_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    let sender = ObjectGuid::create_player(1, 42);
    let other = ObjectGuid::create_player(1, 43);
    let group_registry = Arc::new(GroupRegistry::default());
    let mut group = GroupInfo::new(sender);
    group.add_member(other);
    let group_guid = group.group_guid;
    group_registry.register_group_like_cpp(group_guid, group);

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
        .handle_random_roll(random_roll_packet(1, 100, Some(0)))
        .await;

    assert!(
        send_rx.try_recv().is_err(),
        "group random roll should use group fanout, not direct self channel"
    );
    let sender_sent = sender_rx
        .try_recv()
        .expect("sender should receive group random roll");
    let other_sent = other_rx
        .try_recv()
        .expect("other member should receive group random roll");
    assert_random_roll_packet(&sender_sent, sender, 1, 1, 100);
    assert_random_roll_packet(&other_sent, sender, 1, 1, 100);
}

#[tokio::test]
async fn random_roll_ignores_party_index_for_home_group_lookup_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send();
    let sender = ObjectGuid::create_player(1, 42);
    let other = ObjectGuid::create_player(1, 43);
    let group_registry = Arc::new(GroupRegistry::default());
    let mut group = GroupInfo::new(sender);
    group.add_member(other);
    let group_guid = group.group_guid;
    group_registry.register_group_like_cpp(group_guid, group);

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
        .handle_random_roll(random_roll_packet(1, 2, Some(1)))
        .await;

    assert!(
        sender_rx.try_recv().is_ok(),
        "RandomRoll parses but ignores PartyIndex like C++ DoRandomRoll/GetGroup"
    );
    assert!(
        other_rx.try_recv().is_ok(),
        "RandomRoll must still broadcast to represented HOME group"
    );
}

#[tokio::test]
async fn random_roll_rejects_invalid_bounds_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    session.set_player_guid(Some(ObjectGuid::create_player(1, 42)));

    session
        .handle_random_roll(random_roll_packet(100, 1, None))
        .await;
    session
        .handle_random_roll(random_roll_packet(1, 1_000_001, None))
        .await;

    assert!(send_rx.try_recv().is_err());
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
    group_registry.register_group_like_cpp(group_guid, group);

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
    group_registry.register_group_like_cpp(group_guid, group);

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
    group_registry.register_group_like_cpp(group_guid, group);

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
    group_registry.register_group_like_cpp(group_guid, group);

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
    group_registry.register_group_like_cpp(stale_guid, stale_group);

    // Real group: sender IS a member.
    let mut real_group = GroupInfo::new(sender);
    real_group.add_member(other);
    let real_guid = real_group.group_guid;
    group_registry.register_group_like_cpp(real_guid, real_group);

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
    group_registry.register_group_like_cpp(group_guid, group);

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
    group_registry.register_group_like_cpp(stale_guid, stale_group);

    // Real group: sender IS a member.
    let mut real_group = GroupInfo::new(sender);
    real_group.add_member(real_member);
    let real_guid = real_group.group_guid;
    group_registry.register_group_like_cpp(real_guid, real_group);

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
    group_registry.register_group_like_cpp(stale_guid, stale_group);

    // Real group: sender is leader.
    let mut real_group = GroupInfo::new(sender);
    real_group.add_member(real_member);
    let real_guid = real_group.group_guid;
    group_registry.register_group_like_cpp(real_guid, real_group);

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
    group_registry.register_group_like_cpp(home_guid, home_group);

    let mut stale_group = GroupInfo::new(stale_leader);
    stale_group.add_member(instance_member);
    let stale_guid = stale_group.group_guid;
    group_registry.register_group_like_cpp(stale_guid, stale_group);

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
    group_registry.register_group_like_cpp(group_guid, group);

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
    group_registry.register_group_like_cpp(group_guid, group);

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
