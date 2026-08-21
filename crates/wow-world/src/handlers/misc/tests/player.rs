// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! player capability handler tests.

use super::*;
use wow_constants::UnitStandStateType;

#[test]
fn item_purchase_contents_skip_season_earned_currency_like_cpp() {
    let extended_cost = wow_data::item_extended_cost::ItemExtendedCostEntry {
        id: 1,
        required_arena_rating: 0,
        arena_bracket: 0,
        flags: ItemExtendedCostFlags::REQUIRE_SEASON_EARNED_2,
        min_faction_id: 0,
        min_reputation: 0,
        required_achievement: 0,
        item_id: [100, 0, 0, 0, 0],
        item_count: [2, 0, 0, 0, 0],
        currency_id: [390, 391, 0, 0, 0],
        currency_count: [5, 7, 0, 0, 0],
    };

    let contents = item_purchase_contents_from_extended_cost(&extended_cost, 123);
    assert_eq!(contents.money, 123);
    assert_eq!(contents.items[0].item_id, 100);
    assert_eq!(contents.items[0].item_count, 2);
    assert_eq!(contents.currencies[0].currency_id, 390);
    assert_eq!(contents.currencies[0].currency_count, 5);
    assert_eq!(contents.currencies[1].currency_id, 0);
    assert_eq!(contents.currencies[1].currency_count, 0);
}

#[tokio::test]
async fn set_action_button_adds_packed_action_like_cpp() {
    let (mut session, send_rx) = make_session();
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint32(12_345 | (0x80 << 24));
    pkt.write_uint8(7);
    pkt.reset_read();

    session.handle_set_action_button(pkt).await;

    assert_eq!(
        session.represented_action_button_like_cpp(7),
        Some(12_345 | (0x80 << 24))
    );
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn set_action_button_zero_removes_action_like_cpp() {
    let (mut session, _send_rx) = make_session();
    let mut add = WorldPacket::new_empty();
    add.write_uint32(1_337 | (0x40 << 24));
    add.write_uint8(9);
    add.reset_read();
    session.handle_set_action_button(add).await;
    assert_eq!(
        session.represented_action_button_like_cpp(9),
        Some(1_337 | (0x40 << 24))
    );

    let mut remove = WorldPacket::new_empty();
    remove.write_uint32(0);
    remove.write_uint8(9);
    remove.reset_read();
    session.handle_set_action_button(remove).await;

    assert_eq!(session.represented_action_button_like_cpp(9), Some(0));
}

#[tokio::test]
async fn set_action_button_short_packet_does_not_mutate_like_cpp() {
    let (mut session, _send_rx) = make_session();
    session
        .handle_set_action_button(WorldPacket::from_bytes(&[0x01, 0x02]))
        .await;

    assert_eq!(session.represented_action_button_like_cpp(0), Some(0));
}

#[tokio::test]
async fn set_difficulty_id_without_runtime_store_is_silent_like_cpp_missing_entry_branch() {
    let (mut session, send_rx) = make_session();

    session
        .handle_set_difficulty_id(set_difficulty_request(23))
        .await;

    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn set_difficulty_id_updates_solo_dungeon_difficulty_like_cpp() {
    let (mut session, send_rx) = make_session();
    session.set_difficulty_store(Arc::new(DifficultyStore::from_entries([difficulty_entry(
        2,
        1,
        DifficultyFlags::CAN_SELECT,
    )])));
    session.set_map_store(Arc::new(MapStore::from_entries([map_entry(
        0,
        wow_data::map::MAP_COMMON,
    )])));

    session
        .handle_set_difficulty_id(set_difficulty_request(2))
        .await;

    assert_eq!(session.represented_dungeon_difficulty_id_like_cpp(), 2);
    let sent = send_rx.try_recv().expect("dungeon difficulty packet");
    assert_eq!(
        WorldPacket::from_bytes(&sent).server_opcode(),
        Some(ServerOpcodes::SetDungeonDifficulty)
    );
    assert_eq!(i32::from_le_bytes([sent[2], sent[3], sent[4], sent[5]]), 2);
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn set_dungeon_difficulty_updates_solo_dungeon_difficulty_like_cpp() {
    let (mut session, send_rx) = make_session();
    session.set_difficulty_store(Arc::new(DifficultyStore::from_entries([difficulty_entry(
        2,
        1,
        DifficultyFlags::CAN_SELECT,
    )])));
    session.set_map_store(Arc::new(MapStore::from_entries([map_entry(
        0,
        wow_data::map::MAP_COMMON,
    )])));

    session
        .handle_set_dungeon_difficulty(set_dungeon_difficulty_request(2))
        .await;

    assert_eq!(session.represented_dungeon_difficulty_id_like_cpp(), 2);
    let sent = send_rx.try_recv().expect("dungeon difficulty packet");
    assert_eq!(
        WorldPacket::from_bytes(&sent).server_opcode(),
        Some(ServerOpcodes::SetDungeonDifficulty)
    );
    assert_eq!(i32::from_le_bytes([sent[2], sent[3], sent[4], sent[5]]), 2);
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn toggle_difficulty_uses_raid_toggle_before_dungeon_like_cpp() {
    let (mut session, send_rx) = make_session();
    session.set_difficulty_store(Arc::new(DifficultyStore::from_entries([
        difficulty_entry_with_toggle(14, 2, DifficultyFlags::CAN_SELECT, 15),
        difficulty_entry(15, 2, DifficultyFlags::CAN_SELECT),
        difficulty_entry_with_toggle(1, 1, DifficultyFlags::CAN_SELECT, 2),
        difficulty_entry(2, 1, DifficultyFlags::CAN_SELECT),
    ])));
    session.set_map_store(Arc::new(MapStore::from_entries([map_entry(
        0,
        wow_data::map::MAP_COMMON,
    )])));

    session
        .handle_toggle_difficulty(WorldPacket::new_empty())
        .await;

    assert_eq!(session.represented_raid_difficulty_id_like_cpp(), 15);
    assert_eq!(session.represented_dungeon_difficulty_id_like_cpp(), 1);
    let sent = send_rx.try_recv().expect("raid difficulty packet");
    assert_eq!(
        WorldPacket::from_bytes(&sent).server_opcode(),
        Some(ServerOpcodes::RaidDifficultySet)
    );
    assert_eq!(i32::from_le_bytes([sent[2], sent[3], sent[4], sent[5]]), 15);
    assert_eq!(sent[6], 0);
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn toggle_difficulty_falls_back_to_dungeon_when_raid_has_no_toggle_like_cpp() {
    let (mut session, send_rx) = make_session();
    session.set_difficulty_store(Arc::new(DifficultyStore::from_entries([
        difficulty_entry(14, 2, DifficultyFlags::CAN_SELECT),
        difficulty_entry_with_toggle(1, 1, DifficultyFlags::CAN_SELECT, 2),
        difficulty_entry(2, 1, DifficultyFlags::CAN_SELECT),
    ])));
    session.set_map_store(Arc::new(MapStore::from_entries([map_entry(
        0,
        wow_data::map::MAP_COMMON,
    )])));

    session
        .handle_toggle_difficulty(WorldPacket::new_empty())
        .await;

    assert_eq!(session.represented_dungeon_difficulty_id_like_cpp(), 2);
    assert_eq!(session.represented_raid_difficulty_id_like_cpp(), 14);
    let sent = send_rx.try_recv().expect("dungeon difficulty packet");
    assert_eq!(
        WorldPacket::from_bytes(&sent).server_opcode(),
        Some(ServerOpcodes::SetDungeonDifficulty)
    );
    assert_eq!(i32::from_le_bytes([sent[2], sent[3], sent[4], sent[5]]), 2);
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn toggle_difficulty_without_available_toggle_is_silent_like_cpp() {
    let (mut session, send_rx) = make_session();
    session.set_difficulty_store(Arc::new(DifficultyStore::from_entries([
        difficulty_entry(14, 2, DifficultyFlags::CAN_SELECT),
        difficulty_entry(1, 1, DifficultyFlags::CAN_SELECT),
    ])));

    session
        .handle_toggle_difficulty(WorldPacket::new_empty())
        .await;

    assert_eq!(session.represented_dungeon_difficulty_id_like_cpp(), 1);
    assert_eq!(session.represented_raid_difficulty_id_like_cpp(), 14);
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn set_difficulty_id_same_dungeon_difficulty_is_silent_like_cpp() {
    let (mut session, send_rx) = make_session();
    session.set_difficulty_store(Arc::new(DifficultyStore::from_entries([difficulty_entry(
        2,
        1,
        DifficultyFlags::CAN_SELECT,
    )])));
    session.set_map_store(Arc::new(MapStore::from_entries([map_entry(
        0,
        wow_data::map::MAP_COMMON,
    )])));

    session
        .handle_set_difficulty_id(set_difficulty_request(2))
        .await;
    send_rx.try_recv().expect("first dungeon difficulty packet");
    session
        .handle_set_difficulty_id(set_difficulty_request(2))
        .await;

    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn set_difficulty_id_updates_solo_raid_difficulty_like_cpp() {
    let (mut session, send_rx) = make_session();
    session.set_difficulty_store(Arc::new(DifficultyStore::from_entries([difficulty_entry(
        15,
        2,
        DifficultyFlags::CAN_SELECT,
    )])));
    session.set_map_store(Arc::new(MapStore::from_entries([map_entry(
        0,
        wow_data::map::MAP_COMMON,
    )])));

    session
        .handle_set_difficulty_id(set_difficulty_request(15))
        .await;

    assert_eq!(session.represented_raid_difficulty_id_like_cpp(), 15);
    let sent = send_rx.try_recv().expect("raid difficulty packet");
    assert_eq!(
        WorldPacket::from_bytes(&sent).server_opcode(),
        Some(ServerOpcodes::RaidDifficultySet)
    );
    assert_eq!(i32::from_le_bytes([sent[2], sent[3], sent[4], sent[5]]), 15);
    assert_eq!(sent[6], 0);
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn set_raid_difficulty_updates_solo_raid_difficulty_like_cpp() {
    let (mut session, send_rx) = make_session();
    session.set_difficulty_store(Arc::new(DifficultyStore::from_entries([difficulty_entry(
        15,
        2,
        DifficultyFlags::CAN_SELECT,
    )])));
    session.set_map_store(Arc::new(MapStore::from_entries([map_entry(
        0,
        wow_data::map::MAP_COMMON,
    )])));

    session
        .handle_set_raid_difficulty(set_raid_difficulty_request(15, 0))
        .await;

    assert_eq!(session.represented_raid_difficulty_id_like_cpp(), 15);
    let sent = send_rx.try_recv().expect("raid difficulty packet");
    assert_eq!(
        WorldPacket::from_bytes(&sent).server_opcode(),
        Some(ServerOpcodes::RaidDifficultySet)
    );
    assert_eq!(i32::from_le_bytes([sent[2], sent[3], sent[4], sent[5]]), 15);
    assert_eq!(sent[6], 0);
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn set_difficulty_id_updates_solo_legacy_raid_difficulty_like_cpp() {
    let (mut session, send_rx) = make_session();
    session.set_difficulty_store(Arc::new(DifficultyStore::from_entries([difficulty_entry(
        4,
        2,
        DifficultyFlags::CAN_SELECT | DifficultyFlags::LEGACY,
    )])));
    session.set_map_store(Arc::new(MapStore::from_entries([map_entry(
        0,
        wow_data::map::MAP_COMMON,
    )])));

    session
        .handle_set_difficulty_id(set_difficulty_request(4))
        .await;

    assert_eq!(session.represented_legacy_raid_difficulty_id_like_cpp(), 4);
    let sent = send_rx.try_recv().expect("legacy raid difficulty packet");
    assert_eq!(
        WorldPacket::from_bytes(&sent).server_opcode(),
        Some(ServerOpcodes::RaidDifficultySet)
    );
    assert_eq!(i32::from_le_bytes([sent[2], sent[3], sent[4], sent[5]]), 4);
    assert_eq!(sent[6], 1);
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn set_raid_difficulty_legacy_flag_mismatch_is_silent_like_cpp() {
    let (mut session, send_rx) = make_session();
    session.set_difficulty_store(Arc::new(DifficultyStore::from_entries([difficulty_entry(
        4,
        2,
        DifficultyFlags::CAN_SELECT | DifficultyFlags::LEGACY,
    )])));
    session.set_map_store(Arc::new(MapStore::from_entries([map_entry(
        0,
        wow_data::map::MAP_COMMON,
    )])));

    session
        .handle_set_raid_difficulty(set_raid_difficulty_request(4, 0))
        .await;

    assert_eq!(session.represented_legacy_raid_difficulty_id_like_cpp(), 3);
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn set_raid_difficulty_negative_id_is_silent_like_cpp() {
    let (mut session, send_rx) = make_session();
    session.set_difficulty_store(Arc::new(DifficultyStore::from_entries([difficulty_entry(
        4,
        2,
        DifficultyFlags::CAN_SELECT | DifficultyFlags::LEGACY,
    )])));

    session
        .handle_set_raid_difficulty(set_raid_difficulty_request(-1, 1))
        .await;

    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn set_difficulty_id_group_leader_updates_group_dungeon_difficulty_like_cpp() {
    let (mut session, send_rx) = make_session();
    let leader = ObjectGuid::create_player(1, 100);
    let member = ObjectGuid::create_player(1, 101);
    let group_registry = Arc::new(GroupRegistry::default());
    let player_registry = Arc::new(PlayerRegistry::default());
    let (member_command_tx, member_command_rx) = flume::bounded::<SessionCommand>(4);
    let mut group = GroupInfo::new(leader);
    group.add_member(member);
    group.db_store_id = 44;
    let group_guid = group.group_guid;
    group_registry.register_group_like_cpp(group_guid, group);
    player_registry.register_or_replace(member, broadcast_info_with_command_tx(member_command_tx));

    session.set_player_guid(Some(leader));
    session.group_guid = Some(group_guid);
    session.set_group_registry(group_registry.clone(), Arc::new(PendingInvites::default()));
    session.set_player_registry(player_registry);
    session.set_difficulty_store(Arc::new(DifficultyStore::from_entries([difficulty_entry(
        2,
        1,
        DifficultyFlags::CAN_SELECT,
    )])));
    session.set_map_store(Arc::new(MapStore::from_entries([map_entry(
        0,
        wow_data::map::MAP_COMMON,
    )])));

    session
        .handle_set_difficulty_id(set_difficulty_request(2))
        .await;

    let group = group_registry.get(&group_guid).expect("group");
    assert_eq!(group.dungeon_difficulty_id, 2);
    assert_eq!(session.represented_dungeon_difficulty_id_like_cpp(), 2);
    let sent = send_rx
        .try_recv()
        .expect("leader dungeon difficulty packet");
    assert_eq!(
        WorldPacket::from_bytes(&sent).server_opcode(),
        Some(ServerOpcodes::SetDungeonDifficulty)
    );
    let command = member_command_rx
        .try_recv()
        .expect("member difficulty command");
    let SessionCommand::ApplyGroupDifficultyLikeCpp(command) = command else {
        panic!("expected ApplyGroupDifficultyLikeCpp");
    };
    assert_eq!(command.group_guid, group_guid);
    assert_eq!(command.difficulty_id, 2);
    assert_eq!(
        command.kind,
        wow_network::player_registry::GroupDifficultyKindLikeCpp::Dungeon
    );
}

#[tokio::test]
async fn set_difficulty_id_group_non_leader_is_silent_like_cpp() {
    let (mut session, send_rx) = make_session();
    let leader = ObjectGuid::create_player(1, 100);
    let member = ObjectGuid::create_player(1, 101);
    let group_registry = Arc::new(GroupRegistry::default());
    let mut group = GroupInfo::new(leader);
    group.add_member(member);
    let group_guid = group.group_guid;
    group_registry.register_group_like_cpp(group_guid, group);

    session.set_player_guid(Some(member));
    session.group_guid = Some(group_guid);
    session.set_group_registry(group_registry.clone(), Arc::new(PendingInvites::default()));
    session.set_difficulty_store(Arc::new(DifficultyStore::from_entries([difficulty_entry(
        2,
        1,
        DifficultyFlags::CAN_SELECT,
    )])));
    session.set_map_store(Arc::new(MapStore::from_entries([map_entry(
        0,
        wow_data::map::MAP_COMMON,
    )])));

    session
        .handle_set_difficulty_id(set_difficulty_request(2))
        .await;

    assert_eq!(
        group_registry
            .get(&group_guid)
            .expect("group")
            .dungeon_difficulty_id,
        DIFFICULTY_NORMAL_LIKE_CPP
    );
    assert_eq!(
        session.represented_dungeon_difficulty_id_like_cpp(),
        DIFFICULTY_NORMAL_LIKE_CPP
    );
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn group_difficulty_command_updates_remote_member_like_cpp() {
    let (mut session, send_rx) = make_session();
    let group_guid = 7001;
    session.group_guid = Some(group_guid);
    session.set_state(crate::session::SessionState::LoggedIn);
    session
        .session_command_tx()
        .try_send(SessionCommand::ApplyGroupDifficultyLikeCpp(
            wow_network::player_registry::ApplyGroupDifficultyLikeCppCommand {
                group_guid,
                difficulty_id: 15,
                kind: wow_network::player_registry::GroupDifficultyKindLikeCpp::Raid,
            },
        ))
        .unwrap();

    session
        .process_represented_session_commands_like_cpp()
        .await;

    assert_eq!(session.represented_raid_difficulty_id_like_cpp(), 15);
    let sent = send_rx.try_recv().expect("remote raid difficulty packet");
    assert_eq!(
        WorldPacket::from_bytes(&sent).server_opcode(),
        Some(ServerOpcodes::RaidDifficultySet)
    );
    assert_eq!(i32::from_le_bytes([sent[2], sent[3], sent[4], sent[5]]), 15);
    assert_eq!(sent[6], 0);
}

#[tokio::test]
async fn set_difficulty_id_unselectable_entry_is_silent_like_cpp() {
    let (mut session, send_rx) = make_session();
    session.set_difficulty_store(Arc::new(DifficultyStore::from_entries([difficulty_entry(
        2,
        1,
        DifficultyFlags::empty(),
    )])));

    session
        .handle_set_difficulty_id(set_difficulty_request(2))
        .await;

    assert_eq!(session.represented_dungeon_difficulty_id_like_cpp(), 1);
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn set_difficulty_id_short_packet_does_not_send_like_cpp() {
    let (mut session, send_rx) = make_session();

    session
        .handle_set_difficulty_id(WorldPacket::from_bytes(&[0x17, 0x00]))
        .await;

    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn set_title_requires_known_positive_title_like_cpp() {
    let (mut session, send_rx) = make_session();
    session.represented_learn_title_like_cpp(42);

    let mut known = WorldPacket::new_empty();
    known.write_int32(42);
    known.reset_read();
    session.handle_set_title(known).await;
    assert_eq!(session.represented_chosen_title_like_cpp(), 42);

    let mut unknown = WorldPacket::new_empty();
    unknown.write_int32(77);
    unknown.reset_read();
    session.handle_set_title(unknown).await;
    assert_eq!(
        session.represented_chosen_title_like_cpp(),
        42,
        "C++ returns before SetChosenTitle when HasTitle fails"
    );

    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn set_title_non_positive_clears_to_zero_like_cpp() {
    let (mut session, send_rx) = make_session();
    session.represented_learn_title_like_cpp(42);
    session.represented_set_chosen_title_like_cpp(42);

    let mut clear = WorldPacket::new_empty();
    clear.write_int32(-1);
    clear.reset_read();
    session.handle_set_title(clear).await;

    assert_eq!(session.represented_chosen_title_like_cpp(), 0);
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn set_title_updates_canonical_player_title_field_like_cpp() {
    let (mut session, send_rx) = make_session();
    let canonical = shared_canonical_map_manager_for_misc_test();
    let player_guid = ObjectGuid::create_player(1, 57);
    let player_position = Position::new(10.0, 0.0, 0.0, 0.0);

    session.set_player_guid(Some(player_guid));
    session.set_loaded_player_identity_like_cpp(571, 1, 1, 80, 0);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.represented_learn_title_like_cpp(42);
    add_canonical_test_player_on_map_for_misc_test(
        &canonical,
        player_guid,
        player_position,
        571,
        0,
    );
    session.mutate_canonical_player_like_cpp(|player| player.clear_data_changes());

    let mut request = WorldPacket::new_empty();
    request.write_int32(42);
    request.reset_read();
    session.handle_set_title(request).await;

    assert_eq!(session.represented_chosen_title_like_cpp(), 42);
    assert_eq!(
        session
            .mutate_canonical_player_like_cpp(|player| player.data().player_title)
            .unwrap(),
        42
    );
    let update_packet = send_rx.try_recv().expect("PlayerTitle values update");
    assert_eq!(
        u16::from_le_bytes([update_packet[0], update_packet[1]]),
        ServerOpcodes::UpdateObject as u16
    );
}

#[tokio::test]
async fn query_next_mail_time_routes_result_to_realm_like_cpp() {
    let (mut session, instance_rx, realm_rx) = make_session_with_realm_send();

    session.handle_query_next_mail_time().await;

    assert!(instance_rx.try_recv().is_err());
    let bytes = realm_rx.try_recv().expect("mail query next time result");
    assert_eq!(
        u16::from_le_bytes([bytes[0], bytes[1]]),
        ServerOpcodes::MailQueryNextTimeResult as u16
    );
}

#[tokio::test]
async fn stand_state_change_bridge_applies_valid_states_and_rejects_others_like_cpp() {
    let (mut session, send_rx) = make_session();
    let canonical = shared_canonical_map_manager_for_misc_test();
    let player_guid = ObjectGuid::create_player(1, 9010);
    let position = Position::new(1.0, 2.0, 3.0, 0.0);
    add_canonical_test_player_on_map_for_misc_test(&canonical, player_guid, position, 571, 0);
    session.set_player_guid(Some(player_guid));
    session.set_player_map_position_like_cpp(571, position);
    session.set_canonical_map_manager(Arc::clone(&canonical));

    for state in [
        UnitStandStateType::Sit,
        UnitStandStateType::Sleep,
        UnitStandStateType::Kneel,
        UnitStandStateType::Stand,
    ] {
        session
            .handle_stand_state_change(stand_state_change_packet(state as u32))
            .await;
        assert_eq!(session.player_stand_state_like_cpp(), state);

        let stand_bytes = send_rx.try_recv().expect("SMSG_STAND_STATE_UPDATE");
        let mut stand_packet = WorldPacket::from_bytes(&stand_bytes);
        assert_eq!(
            stand_packet.server_opcode(),
            Some(ServerOpcodes::StandStateUpdate)
        );
        stand_packet.skip_opcode();
        assert_eq!(stand_packet.read_uint32().unwrap(), 0);
        assert_eq!(stand_packet.read_uint8().unwrap(), state as u8);
        assert_eq!(stand_packet.remaining(), 0);

        let values_bytes = send_rx.try_recv().expect("StandState VALUES update");
        assert_eq!(
            WorldPacket::from_bytes(&values_bytes).server_opcode(),
            Some(ServerOpcodes::UpdateObject)
        );
        assert!(send_rx.try_recv().is_err());

        let manager = canonical.lock().unwrap();
        let player = manager
            .find_map(571, 0)
            .unwrap()
            .map()
            .get_typed_player(player_guid)
            .unwrap();
        assert_eq!(player.unit().stand_state_like_cpp(), state);
    }

    let applied_before_invalid = session.represented_live_applications_like_cpp().len();
    session
        .handle_stand_state_change(stand_state_change_packet(
            UnitStandStateType::SitChair as u32,
        ))
        .await;
    assert_eq!(
        session.player_stand_state_like_cpp(),
        UnitStandStateType::Stand
    );
    assert_eq!(
        session.represented_live_applications_like_cpp().len(),
        applied_before_invalid
    );
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn stand_state_change_missing_live_owner_records_no_false_success_like_cpp() {
    let (mut session, send_rx) = make_session();
    session.set_player_guid(Some(ObjectGuid::create_player(1, 9011)));
    session.set_player_map_position_like_cpp(571, Position::ZERO);

    session
        .handle_stand_state_change(stand_state_change_packet(UnitStandStateType::Sit as u32))
        .await;

    assert_eq!(
        session.player_stand_state_like_cpp(),
        UnitStandStateType::Stand
    );
    assert!(session.represented_live_applications_like_cpp().is_empty());
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn repeated_stand_state_still_sends_direct_cpp_packet_without_values_delta() {
    let (mut session, send_rx) = make_session();
    let canonical = shared_canonical_map_manager_for_misc_test();
    let player_guid = ObjectGuid::create_player(1, 9012);
    let position = Position::new(1.0, 2.0, 3.0, 0.0);
    add_canonical_test_player_on_map_for_misc_test(&canonical, player_guid, position, 571, 0);
    session.set_player_guid(Some(player_guid));
    session.set_player_map_position_like_cpp(571, position);
    session.set_canonical_map_manager(canonical);

    session
        .handle_stand_state_change(stand_state_change_packet(UnitStandStateType::Stand as u32))
        .await;

    let bytes = send_rx.try_recv().expect("repeated StandStateUpdate");
    assert_eq!(
        WorldPacket::from_bytes(&bytes).server_opcode(),
        Some(ServerOpcodes::StandStateUpdate)
    );
    assert!(send_rx.try_recv().is_err());
    assert_eq!(
        session.represented_live_applications_like_cpp(),
        &[crate::session::RepresentedLiveApplicationLikeCpp {
            intent: crate::session::RepresentedLiveIntentLikeCpp::StandStateChanged(
                crate::session::RepresentedStandStateChangedLikeCpp {
                    state: UnitStandStateType::Stand,
                },
            ),
            outcome: crate::session::RepresentedLiveIntentApplyOutcomeLikeCpp::Applied(
                crate::session::RepresentedLiveIntentAppliedLikeCpp::StandStateChanged {
                    canonical_field_changed: false,
                    canonical_auras_removed: 0,
                    represented_auras_removed: 0,
                    channel_cancellation_boundary: None,
                },
            ),
        }]
    );
}

#[test]
fn stand_state_change_handler_metadata_matches_cpp() {
    let entry = inventory::iter::<PacketHandlerEntry>
        .into_iter()
        .find(|entry| entry.opcode == ClientOpcodes::StandStateChange)
        .expect("StandStateChange handler entry");

    assert_eq!(entry.status, SessionStatus::LoggedIn);
    assert_eq!(entry.processing, PacketProcessing::ThreadUnsafe);
    assert_eq!(entry.handler_name, "handle_stand_state_change");
}
