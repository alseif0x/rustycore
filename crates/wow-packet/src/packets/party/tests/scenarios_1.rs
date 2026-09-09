//! Party packets regression scenarios, part 1 of 2.
//!
//! Moved out of the party.rs root under #650; every test is unchanged.

use super::*;

#[test]
fn party_result_constants_match_cpp_shared_defines() {
    assert_eq!(party_result::OK, 0);
    assert_eq!(party_result::BAD_PLAYER_NAME, 1);
    assert_eq!(party_result::TARGET_NOT_IN_GROUP, 2);
    assert_eq!(party_result::TARGET_NOT_IN_INSTANCE, 3);
    assert_eq!(party_result::GROUP_FULL, 4);
    assert_eq!(party_result::ALREADY_IN_GROUP, 5);
    assert_eq!(party_result::NOT_IN_GROUP, 6);
    assert_eq!(party_result::NOT_LEADER, 7);
    assert_eq!(party_result::WRONG_FACTION, 8);
    assert_eq!(party_result::IGNORING_YOU, 9);
    assert_eq!(party_result::INVITE_RESTRICTED, 13);
    assert_eq!(party_result::GROUP_SWAP_FAILED, 14);
}

#[test]
fn party_member_full_state_writes_dungeon_score_summary_like_cpp() {
    let member_guid = ObjectGuid::create_player(1, 77);
    let mut pkt = WorldPacket::new_empty();
    PartyMemberFullState {
        member_guid,
        for_enemy: false,
        status: 1,
        power_type: 0,
        current_health: 100,
        max_health: 200,
        current_power: 10,
        max_power: 20,
        level: 80,
        spec_id: 0,
        zone_id: 571,
        position_x: 1,
        position_y: 2,
        position_z: 3,
        vehicle_seat: 0,
        party_type: [0; 2],
        phases: PartyMemberPhaseStates::default(),
        auras: Vec::new(),
        pet_stats: None,
        dungeon_score: DungeonScoreSummary {
            overall_score_current_season: 123.5,
            ladder_score_current_season: 45.25,
            runs: vec![DungeonScoreMapSummary {
                challenge_mode_id: 2,
                map_score: 111.0,
                best_run_level: 7,
                best_run_duration_ms: 900_000,
                finished_success: true,
            }],
        },
    }
    .write(&mut pkt);
    pkt.reset_read();

    assert!(!pkt.read_bit().unwrap());
    assert_eq!(pkt.read_uint8().unwrap(), 0);
    assert_eq!(pkt.read_uint8().unwrap(), 0);
    assert_eq!(pkt.read_int16().unwrap(), 1);
    assert_eq!(pkt.read_uint8().unwrap(), 0);
    assert_eq!(pkt.read_int16().unwrap(), 0);
    assert_eq!(pkt.read_int32().unwrap(), 100);
    assert_eq!(pkt.read_int32().unwrap(), 200);
    assert_eq!(pkt.read_uint16().unwrap(), 10);
    assert_eq!(pkt.read_uint16().unwrap(), 20);
    assert_eq!(pkt.read_uint16().unwrap(), 80);
    assert_eq!(pkt.read_uint16().unwrap(), 0);
    assert_eq!(pkt.read_uint16().unwrap(), 571);
    assert_eq!(pkt.read_uint16().unwrap(), 0);
    assert_eq!(pkt.read_uint32().unwrap(), 0);
    assert_eq!(pkt.read_int16().unwrap(), 1);
    assert_eq!(pkt.read_int16().unwrap(), 2);
    assert_eq!(pkt.read_int16().unwrap(), 3);
    assert_eq!(pkt.read_int32().unwrap(), 0);
    assert_eq!(pkt.read_uint32().unwrap(), 0);
    assert_eq!(pkt.read_uint32().unwrap(), 0);
    assert_eq!(pkt.read_uint32().unwrap(), 0);
    assert_eq!(pkt.read_packed_guid().unwrap(), ObjectGuid::EMPTY);
    assert_eq!(pkt.read_uint32().unwrap(), 0);
    assert_eq!(pkt.read_uint32().unwrap(), 0);
    assert_eq!(pkt.read_int32().unwrap(), 0);
    assert!(!pkt.read_bit().unwrap());
    assert_eq!(pkt.read_float().unwrap(), 123.5);
    assert_eq!(pkt.read_float().unwrap(), 45.25);
    assert_eq!(pkt.read_uint32().unwrap(), 1);
    assert_eq!(pkt.read_int32().unwrap(), 2);
    assert_eq!(pkt.read_float().unwrap(), 111.0);
    assert_eq!(pkt.read_int32().unwrap(), 7);
    assert_eq!(pkt.read_int32().unwrap(), 900_000);
    assert!(pkt.read_bit().unwrap());
    assert_eq!(pkt.read_packed_guid().unwrap(), member_guid);
}

#[test]
fn set_loot_method_reads_cpp_bit_method_master_threshold_party_index_order() {
    let master = ObjectGuid::create_player(1, 77);
    let mut pkt = WorldPacket::new_empty();
    pkt.write_bit(true);
    pkt.write_uint8(2);
    pkt.write_packed_guid(&master);
    pkt.write_uint32(4);
    pkt.write_uint8(0);
    pkt.reset_read();

    let set_loot = SetLootMethod::read(&mut pkt).unwrap();

    assert_eq!(set_loot.party_index, Some(0));
    assert_eq!(set_loot.loot_method, 2);
    assert_eq!(set_loot.loot_master_guid, master);
    assert_eq!(set_loot.loot_threshold, 4);
}

#[test]
fn opt_out_of_loot_reads_cpp_pass_on_loot_bit() {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_bit(true);
    pkt.flush_bits();
    pkt.reset_read();

    let opt_out = OptOutOfLoot::read(&mut pkt).unwrap();

    assert!(opt_out.pass_on_loot);
}

#[test]
fn convert_raid_reads_cpp_raid_bit() {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_bit(true);
    pkt.flush_bits();
    pkt.reset_read();

    let convert = ConvertRaid::read(&mut pkt).unwrap();

    assert!(convert.raid);
}

#[test]
fn change_subgroup_reads_cpp_guid_subgroup_bit_party_index_order() {
    let target = ObjectGuid::create_player(1, 77);
    let mut pkt = WorldPacket::new_empty();
    pkt.write_packed_guid(&target);
    pkt.write_uint8(6);
    pkt.write_bit(true);
    pkt.write_uint8(0);
    pkt.reset_read();

    let change = ChangeSubGroup::read(&mut pkt).unwrap();

    assert_eq!(change.target_guid, target);
    assert_eq!(change.new_subgroup, 6);
    assert_eq!(change.party_index, Some(0));
}

#[test]
fn set_assistant_leader_reads_cpp_has_party_apply_guid_party_index_order() {
    let target = ObjectGuid::create_player(1, 77);
    let mut pkt = WorldPacket::new_empty();
    pkt.write_bit(true);
    pkt.write_bit(true);
    pkt.write_packed_guid(&target);
    pkt.write_uint8(0);
    pkt.reset_read();

    let set_assistant = SetAssistantLeader::read(&mut pkt).unwrap();

    assert_eq!(set_assistant.target, target);
    assert!(set_assistant.apply);
    assert_eq!(set_assistant.party_index, Some(0));
}

#[test]
fn set_assistant_leader_reads_cpp_optional_none_bit_before_apply() {
    let target = ObjectGuid::create_player(1, 78);
    let mut pkt = WorldPacket::new_empty();
    pkt.write_bit(false);
    pkt.write_bit(false);
    pkt.write_packed_guid(&target);
    pkt.flush_bits();
    pkt.reset_read();

    let set_assistant = SetAssistantLeader::read(&mut pkt).unwrap();

    assert_eq!(set_assistant.target, target);
    assert!(!set_assistant.apply);
    assert_eq!(set_assistant.party_index, None);
}

#[test]
fn set_party_leader_reads_cpp_has_party_guid_party_index_order() {
    let target = ObjectGuid::create_player(1, 79);
    let mut pkt = WorldPacket::new_empty();
    pkt.write_bit(true);
    pkt.write_packed_guid(&target);
    pkt.write_uint8(0);
    pkt.reset_read();

    let set_leader = SetPartyLeader::read(&mut pkt).unwrap();

    assert_eq!(set_leader.target_guid, target);
    assert_eq!(set_leader.party_index, Some(0));
}

#[test]
fn set_party_leader_reads_cpp_optional_none_bit_before_guid() {
    let target = ObjectGuid::create_player(1, 80);
    let mut pkt = WorldPacket::new_empty();
    pkt.write_bit(false);
    pkt.write_packed_guid(&target);
    pkt.reset_read();

    let set_leader = SetPartyLeader::read(&mut pkt).unwrap();

    assert_eq!(set_leader.target_guid, target);
    assert_eq!(set_leader.party_index, None);
}

#[test]
fn set_everyone_is_assistant_reads_cpp_has_party_apply_party_index_order() {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_bit(true);
    pkt.write_bit(true);
    pkt.write_uint8(0);
    pkt.reset_read();

    let set_everyone = SetEveryoneIsAssistant::read(&mut pkt).unwrap();

    assert!(set_everyone.everyone_is_assistant);
    assert_eq!(set_everyone.party_index, Some(0));
}

#[test]
fn group_new_leader_writes_cpp_party_index_name_bits_string_order() {
    let bytes = GroupNewLeader {
        party_index: 0,
        name: "Player80".to_string(),
    }
    .to_bytes();

    assert_eq!(
        u16::from_le_bytes([bytes[0], bytes[1]]),
        ServerOpcodes::GroupNewLeader as u16
    );
    let mut payload = WorldPacket::from_bytes(&bytes[2..]);
    assert_eq!(payload.read_int8().unwrap(), 0);
    assert_eq!(payload.read_bits(9).unwrap(), 8);
    assert_eq!(payload.read_string(8).unwrap(), "Player80");
}

#[test]
fn set_everyone_is_assistant_reads_cpp_optional_none_bit_before_apply() {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_bit(false);
    pkt.write_bit(false);
    pkt.flush_bits();
    pkt.reset_read();

    let set_everyone = SetEveryoneIsAssistant::read(&mut pkt).unwrap();

    assert!(!set_everyone.everyone_is_assistant);
    assert_eq!(set_everyone.party_index, None);
}

#[test]
fn set_party_assignment_reads_cpp_has_party_set_assignment_guid_party_index_order() {
    let target = ObjectGuid::create_player(1, 77);
    let mut pkt = WorldPacket::new_empty();
    pkt.write_bit(true);
    pkt.write_bit(true);
    pkt.write_uint8(1);
    pkt.write_packed_guid(&target);
    pkt.write_uint8(0);
    pkt.reset_read();

    let assignment = SetPartyAssignment::read(&mut pkt).unwrap();

    assert_eq!(assignment.assignment, 1);
    assert_eq!(assignment.target, target);
    assert!(assignment.apply);
    assert_eq!(assignment.party_index, Some(0));
}

#[test]
fn set_party_assignment_reads_cpp_optional_none_bit_before_set() {
    let target = ObjectGuid::create_player(1, 78);
    let mut pkt = WorldPacket::new_empty();
    pkt.write_bit(false);
    pkt.write_bit(false);
    pkt.write_uint8(0);
    pkt.write_packed_guid(&target);
    pkt.flush_bits();
    pkt.reset_read();

    let assignment = SetPartyAssignment::read(&mut pkt).unwrap();

    assert_eq!(assignment.assignment, 0);
    assert_eq!(assignment.target, target);
    assert!(!assignment.apply);
    assert_eq!(assignment.party_index, None);
}

#[test]
fn set_role_reads_cpp_has_party_guid_role_party_index_order() {
    let target = ObjectGuid::create_player(1, 77);
    let mut pkt = WorldPacket::new_empty();
    pkt.write_bit(true);
    pkt.write_packed_guid(&target);
    pkt.write_uint8(4);
    pkt.write_uint8(0);
    pkt.reset_read();

    let set_role = SetRole::read(&mut pkt).unwrap();

    assert_eq!(set_role.target_guid, target);
    assert_eq!(set_role.role, 4);
    assert_eq!(set_role.party_index, Some(0));
}

#[test]
fn set_role_reads_cpp_optional_none_before_guid_role() {
    let target = ObjectGuid::create_player(1, 78);
    let mut pkt = WorldPacket::new_empty();
    pkt.write_bit(false);
    pkt.write_packed_guid(&target);
    pkt.write_uint8(2);
    pkt.flush_bits();
    pkt.reset_read();

    let set_role = SetRole::read(&mut pkt).unwrap();

    assert_eq!(set_role.target_guid, target);
    assert_eq!(set_role.role, 2);
    assert_eq!(set_role.party_index, None);
}

#[test]
fn role_changed_inform_writes_cpp_party_from_changed_old_new_order() {
    let from = ObjectGuid::create_player(1, 42);
    let changed = ObjectGuid::create_player(1, 43);
    let mut pkt = WorldPacket::new_empty();
    RoleChangedInform {
        party_index: 0,
        from,
        changed_unit: changed,
        old_role: 1,
        new_role: 4,
    }
    .write(&mut pkt);
    pkt.reset_read();

    assert_eq!(pkt.read_uint8().unwrap(), 0);
    assert_eq!(pkt.read_packed_guid().unwrap(), from);
    assert_eq!(pkt.read_packed_guid().unwrap(), changed);
    assert_eq!(pkt.read_uint8().unwrap(), 1);
    assert_eq!(pkt.read_uint8().unwrap(), 4);
}

#[test]
fn role_poll_reads_cpp_optional_party_index() {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_bit(true);
    pkt.write_uint8(0);
    pkt.reset_read();

    let role_poll = InitiateRolePoll::read(&mut pkt).unwrap();

    assert_eq!(role_poll.party_index, Some(0));
}

#[test]
fn role_poll_reads_cpp_absent_party_index() {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_bit(false);
    pkt.flush_bits();
    pkt.reset_read();

    let role_poll = InitiateRolePoll::read(&mut pkt).unwrap();

    assert_eq!(role_poll.party_index, None);
}

#[test]
fn role_poll_inform_writes_cpp_party_index_then_from() {
    let from = ObjectGuid::create_player(1, 42);
    let mut pkt = WorldPacket::new_empty();
    RolePollInform {
        party_index: 0,
        from,
    }
    .write(&mut pkt);
    pkt.reset_read();

    assert_eq!(pkt.read_int8().unwrap(), 0);
    assert_eq!(pkt.read_packed_guid().unwrap(), from);
}

#[test]
fn update_raid_target_reads_cpp_bit_target_symbol_party_index_order() {
    let target = ObjectGuid::create_player(1, 77);
    let mut pkt = WorldPacket::new_empty();
    pkt.write_bit(true);
    pkt.write_packed_guid(&target);
    pkt.write_int8(3);
    pkt.write_uint8(0);
    pkt.reset_read();

    let update = UpdateRaidTarget::read(&mut pkt).unwrap();

    assert_eq!(update.party_index, Some(0));
    assert_eq!(update.target, target);
    assert_eq!(update.symbol, 3);
}

#[test]
fn update_raid_target_reads_cpp_symbol_minus_one_request() {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_bit(false);
    pkt.write_packed_guid(&ObjectGuid::EMPTY);
    pkt.write_int8(-1);
    pkt.flush_bits();
    pkt.reset_read();

    let update = UpdateRaidTarget::read(&mut pkt).unwrap();

    assert_eq!(update.party_index, None);
    assert_eq!(update.target, ObjectGuid::EMPTY);
    assert_eq!(update.symbol, -1);
}

#[test]
fn request_party_join_updates_reads_cpp_optional_party_index() {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_bit(true);
    pkt.write_uint8(0);
    pkt.reset_read();

    let request = RequestPartyJoinUpdates::read(&mut pkt).unwrap();

    assert_eq!(request.party_index, Some(0));
}

#[test]
fn clear_raid_marker_reads_marker_id_like_cpp() {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint8(8);
    pkt.reset_read();

    let clear = ClearRaidMarker::read(&mut pkt).unwrap();

    assert_eq!(clear.marker_id, 8);
    assert!(pkt.is_empty());
}

#[test]
fn request_party_member_stats_reads_cpp_bit_guid_without_party_index() {
    let target = ObjectGuid::create_player(1, 77);
    let mut pkt = WorldPacket::new_empty();
    pkt.write_bit(false);
    pkt.write_packed_guid(&target);
    pkt.flush_bits();
    pkt.reset_read();

    let request = RequestPartyMemberStats::read(&mut pkt).unwrap();

    assert_eq!(request.target_guid, target);
    assert_eq!(request.party_index, None);
}

#[test]
fn request_party_member_stats_reads_cpp_bit_guid_then_party_index() {
    let target = ObjectGuid::create_player(1, 78);
    let mut pkt = WorldPacket::new_empty();
    pkt.write_bit(true);
    pkt.write_packed_guid(&target);
    pkt.write_uint8(1);
    pkt.reset_read();

    let request = RequestPartyMemberStats::read(&mut pkt).unwrap();

    assert_eq!(request.target_guid, target);
    assert_eq!(request.party_index, Some(1));
}

#[test]
fn raid_target_update_single_writes_cpp_party_symbol_target_changed_by_order() {
    let target = ObjectGuid::create_player(1, 77);
    let changed_by = ObjectGuid::create_player(1, 42);
    let target_bytes = packed_guid_bytes(target);
    let changed_by_bytes = packed_guid_bytes(changed_by);
    let mut pkt = WorldPacket::new_empty();
    SendRaidTargetUpdateSingle {
        party_index: 0,
        target,
        changed_by,
        symbol: 3,
    }
    .write(&mut pkt);
    let data = pkt.into_data();

    assert_eq!(data[0], 0);
    assert_eq!(data[1], 3);
    assert_eq!(&data[2..2 + target_bytes.len()], target_bytes.as_slice());
    assert_eq!(&data[2 + target_bytes.len()..], changed_by_bytes.as_slice());
}

#[test]
fn raid_target_update_all_writes_cpp_party_count_all_icons_order() {
    let first = ObjectGuid::create_player(1, 77);
    let icons: Vec<_> = (0..8)
        .map(|symbol| {
            (
                symbol,
                if symbol == 0 {
                    first
                } else {
                    ObjectGuid::EMPTY
                },
            )
        })
        .collect();
    let mut pkt = WorldPacket::new_empty();
    SendRaidTargetUpdateAll {
        party_index: 0,
        target_icons: icons,
    }
    .write(&mut pkt);
    pkt.reset_read();

    assert_eq!(pkt.read_uint8().unwrap(), 0);
    assert_eq!(pkt.read_uint32().unwrap(), 8);
    assert_eq!(pkt.read_packed_guid().unwrap(), first);
    assert_eq!(pkt.read_uint8().unwrap(), 0);
    for symbol in 1..8 {
        assert_eq!(pkt.read_packed_guid().unwrap(), ObjectGuid::EMPTY);
        assert_eq!(pkt.read_uint8().unwrap(), symbol);
    }
}

#[test]
fn raid_markers_changed_writes_empty_represented_join_update_shape() {
    let mut pkt = WorldPacket::new_empty();
    RaidMarkersChanged {
        party_index: 0,
        active_markers: 0,
        raid_markers: Vec::new(),
    }
    .write(&mut pkt);
    let data = pkt.into_data();

    assert_eq!(data, vec![0, 0, 0, 0, 0, 0]);
}

#[test]
fn raid_markers_changed_writes_marker_entries_like_cpp() {
    let transport = ObjectGuid::create_transport(wow_core::guid::HighGuid::Transport, 0x0102_0304);
    let mut pkt = WorldPacket::new_empty();
    RaidMarkersChanged {
        party_index: 1,
        active_markers: 1 << 3,
        raid_markers: vec![RaidMarker {
            transport_guid: transport,
            map_id: 571,
            position: Position::xyz(12.25, -34.5, 6.75),
        }],
    }
    .write(&mut pkt);
    pkt.reset_read();

    assert_eq!(pkt.read_uint8().unwrap(), 1);
    assert_eq!(pkt.read_uint32().unwrap(), 1 << 3);
    assert_eq!(pkt.read_bits(4).unwrap(), 1);
    pkt.flush_bits();
    assert_eq!(pkt.read_packed_guid().unwrap(), transport);
    assert_eq!(pkt.read_uint32().unwrap(), 571);
    assert_eq!(pkt.read_float().unwrap(), 12.25);
    assert_eq!(pkt.read_float().unwrap(), -34.5);
    assert_eq!(pkt.read_float().unwrap(), 6.75);
    assert!(pkt.is_empty());
}

#[test]
fn ready_check_do_reads_cpp_has_party_index_then_optional_byte() {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_bit(true);
    pkt.write_uint8(0);
    pkt.reset_read();

    let ready_check = DoReadyCheck::read(&mut pkt).unwrap();

    assert_eq!(ready_check.party_index, Some(0));
}

#[test]
fn ready_check_do_reads_cpp_absent_party_index() {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_bit(false);
    pkt.flush_bits();
    pkt.reset_read();

    let ready_check = DoReadyCheck::read(&mut pkt).unwrap();

    assert_eq!(ready_check.party_index, None);
}

#[test]
fn ready_check_response_reads_cpp_ready_bit_then_has_party_index() {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_bit(true);
    pkt.write_bit(true);
    pkt.write_uint8(0);
    pkt.reset_read();

    let response = ReadyCheckResponseClient::read(&mut pkt).unwrap();

    assert!(response.is_ready);
    assert_eq!(response.party_index, Some(0));
}

#[test]
fn ready_check_response_reads_cpp_absent_party_index_after_ready_bit() {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_bit(false);
    pkt.write_bit(false);
    pkt.flush_bits();
    pkt.reset_read();

    let response = ReadyCheckResponseClient::read(&mut pkt).unwrap();

    assert!(!response.is_ready);
    assert_eq!(response.party_index, None);
}

#[test]
fn ready_check_started_writes_cpp_party_guid_initiator_duration_order() {
    let initiator = ObjectGuid::create_player(1, 42);
    let mut pkt = WorldPacket::new_empty();
    ReadyCheckStarted {
        party_index: 0,
        party_guid: 77,
        initiator_guid: initiator,
        duration_ms: 35_000,
    }
    .write(&mut pkt);
    pkt.reset_read();

    assert_eq!(pkt.read_uint8().unwrap(), 0);
    assert_eq!(
        pkt.read_packed_guid().unwrap(),
        ObjectGuid::create_group(77)
    );
    assert_eq!(pkt.read_packed_guid().unwrap(), initiator);
    assert_eq!(pkt.read_int64().unwrap(), 35_000);
}

#[test]
fn ready_check_response_writes_cpp_guids_bit_flush_order() {
    let player = ObjectGuid::create_player(1, 43);
    let mut pkt = WorldPacket::new_empty();
    ReadyCheckResponse {
        party_guid: 78,
        player,
        is_ready: true,
    }
    .write(&mut pkt);
    pkt.reset_read();

    assert_eq!(
        pkt.read_packed_guid().unwrap(),
        ObjectGuid::create_group(78)
    );
    assert_eq!(pkt.read_packed_guid().unwrap(), player);
    assert!(pkt.read_bit().unwrap());
}

#[test]
fn ready_check_completed_writes_cpp_party_index_then_guid() {
    let mut pkt = WorldPacket::new_empty();
    ReadyCheckCompleted {
        party_index: 0,
        party_guid: 79,
    }
    .write(&mut pkt);
    pkt.reset_read();

    assert_eq!(pkt.read_uint8().unwrap(), 0);
    assert_eq!(
        pkt.read_packed_guid().unwrap(),
        ObjectGuid::create_group(79)
    );
}

#[test]
fn swap_subgroups_reads_cpp_bit_first_guid_second_guid_party_index_order() {
    let first = ObjectGuid::create_player(1, 77);
    let second = ObjectGuid::create_player(1, 78);
    let mut pkt = WorldPacket::new_empty();
    pkt.write_bit(true);
    pkt.write_packed_guid(&first);
    pkt.write_packed_guid(&second);
    pkt.write_uint8(0);
    pkt.reset_read();

    let swap = SwapSubGroups::read(&mut pkt).unwrap();

    assert_eq!(swap.first_target, first);
    assert_eq!(swap.second_target, second);
    assert_eq!(swap.party_index, Some(0));
}

#[test]
fn swap_subgroups_reads_cpp_optional_none_bit_before_guids() {
    let first = ObjectGuid::create_player(1, 79);
    let second = ObjectGuid::create_player(1, 80);
    let mut pkt = WorldPacket::new_empty();
    pkt.write_bit(false);
    pkt.write_packed_guid(&first);
    pkt.write_packed_guid(&second);
    pkt.flush_bits();
    pkt.reset_read();

    let swap = SwapSubGroups::read(&mut pkt).unwrap();

    assert_eq!(swap.first_target, first);
    assert_eq!(swap.second_target, second);
    assert_eq!(swap.party_index, None);
}

#[test]
fn change_subgroup_reads_cpp_optional_none_bit() {
    let target = ObjectGuid::create_player(1, 78);
    let mut pkt = WorldPacket::new_empty();
    pkt.write_packed_guid(&target);
    pkt.write_uint8(2);
    pkt.write_bit(false);
    pkt.flush_bits();
    pkt.reset_read();

    let change = ChangeSubGroup::read(&mut pkt).unwrap();

    assert_eq!(change.target_guid, target);
    assert_eq!(change.new_subgroup, 2);
    assert_eq!(change.party_index, None);
}

#[test]
fn party_member_phase_states_writes_cpp_order() {
    let states = PartyMemberPhaseStates {
        phase_shift_flags: 0x08,
        personal_guid: ObjectGuid::EMPTY,
        phases: vec![PartyMemberPhase {
            flags: 0x02,
            id: 20,
        }],
    };
    let mut pkt = WorldPacket::new_empty();

    states.write(&mut pkt);

    assert_eq!(
        pkt.into_data(),
        vec![
            0x08, 0x00, 0x00, 0x00, // PhaseShiftFlags
            0x01, 0x00, 0x00, 0x00, // List.Count
            0x00, 0x00, // PersonalGUID packed mask + empty payload
            0x02, 0x00, 0x00, 0x00, // phase.Flags
            0x14, 0x00, // phase.Id
        ]
    );
}

#[test]
fn low_level_raid1_accepts_empty_payload_like_cpp() {
    let mut pkt = WorldPacket::new_empty();

    let parsed = LowLevelRaid1::read(&mut pkt).unwrap();

    // Empty struct — no fields. C++ Read() is empty body.
    let _ = parsed;
}

#[test]
fn low_level_raid1_opcode_matches_cpp() {
    assert_eq!(LowLevelRaid1::OPCODE as u16, 0x36A1);
}

#[test]
fn low_level_raid2_accepts_empty_payload_like_cpp() {
    let mut pkt = WorldPacket::new_empty();

    let parsed = LowLevelRaid2::read(&mut pkt).unwrap();

    // Empty struct — no fields. C++ Read() is empty body.
    let _ = parsed;
}

#[test]
fn low_level_raid2_opcode_matches_cpp() {
    assert_eq!(LowLevelRaid2::OPCODE as u16, 0x3512);
}

#[test]
fn minimap_ping_client_reads_bit_xy_optional_party_index_like_cpp() {
    // With party index
    let mut pkt = WorldPacket::new_empty();
    pkt.write_bit(true);
    pkt.write_float(123.456);
    pkt.write_float(-789.012);
    pkt.write_uint8(3);
    pkt.flush_bits();
    pkt.reset_read();

    let ping = MinimapPingClient::read(&mut pkt).unwrap();
    assert_eq!(ping.position_x, 123.456);
    assert_eq!(ping.position_y, -789.012);
    assert_eq!(ping.party_index, Some(3));
}

#[test]
fn minimap_ping_client_reads_bit_xy_no_party_index_like_cpp() {
    // Without party index
    let mut pkt = WorldPacket::new_empty();
    pkt.write_bit(false);
    pkt.write_float(42.0);
    pkt.write_float(99.5);
    pkt.flush_bits();
    pkt.reset_read();

    let ping = MinimapPingClient::read(&mut pkt).unwrap();
    assert_eq!(ping.position_x, 42.0);
    assert_eq!(ping.position_y, 99.5);
    assert!(ping.party_index.is_none());
}

#[test]
fn minimap_ping_client_opcode_matches_cpp() {
    assert_eq!(MinimapPingClient::OPCODE as u16, 0x364E);
}

#[test]
fn party_uninvite_reads_bits_guid_party_index_and_reason_like_cpp() {
    // C++ `operator>>(ByteBuffer&, ObjectGuid&)` consumes two masks followed
    // by only the non-zero low/high bytes. Keep this fixture independent of
    // Rust's packed-GUID writer so a symmetric encoder bug cannot bless the
    // decoder.
    let target = ObjectGuid::new(0x0000_0000_0000_3400, 0x12);
    let mut pkt = WorldPacket::new_empty();
    pkt.write_bit(true);
    pkt.write_bits(3, 8);
    pkt.write_bytes(&[0x01, 0x02, 0x12, 0x34]);
    pkt.write_uint8(0);
    pkt.write_string("bye");
    pkt.flush_bits();
    pkt.reset_read();

    let parsed = PartyUninvite::read(&mut pkt).unwrap();

    assert_eq!(parsed.target_guid, target);
    assert_eq!(parsed.party_index, Some(0));
    assert_eq!(parsed.reason, "bye");
    assert_eq!(pkt.remaining(), 0);
}

#[test]
fn party_uninvite_rejects_truncated_packed_guid_like_cpp() {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_bit(false);
    pkt.write_bits(0, 8);
    // The low mask promises byte 0, but the packet ends after both masks.
    pkt.write_bytes(&[0x01, 0x00]);
    pkt.reset_read();

    assert!(matches!(
        PartyUninvite::read(&mut pkt),
        Err(PacketError::ReadPastEnd {
            wanted: 1,
            available: 0
        })
    ));
}
