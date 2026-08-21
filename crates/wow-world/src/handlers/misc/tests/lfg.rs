// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! lfg capability handler tests.

use super::*;

#[tokio::test]
async fn set_difficulty_id_group_lfg_is_silent_like_cpp() {
    let (mut session, send_rx) = make_session();
    let leader = ObjectGuid::create_player(1, 100);
    let group_registry = Arc::new(GroupRegistry::default());
    let mut group = GroupInfo::new(leader);
    group.group_flags |= GROUP_FLAG_LFG_LIKE_CPP;
    let group_guid = group.group_guid;
    group_registry.register_group_like_cpp(group_guid, group);

    session.set_player_guid(Some(leader));
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
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn lfg_list_get_status_sends_removed_from_queue_like_cpp_without_lfg_state() {
    let (mut session, send_rx) = make_session();

    session
        .handle_lfg_list_get_status(WorldPacket::new_empty())
        .await;

    let bytes = send_rx.try_recv().expect("LFG update status packet");
    assert_eq!(
        u16::from_le_bytes([bytes[0], bytes[1]]),
        ServerOpcodes::LfgUpdateStatus as u16
    );

    let mut pkt = WorldPacket::from_bytes(&bytes[2..]);
    assert_eq!(pkt.read_packed_guid().unwrap(), ObjectGuid::EMPTY);
    assert_eq!(pkt.read_uint32().unwrap(), 0); // Ticket.Id
    assert_eq!(pkt.read_uint32().unwrap(), 0); // Ticket.Type
    assert_eq!(pkt.read_int64().unwrap(), 0); // Ticket.Time
    assert!(!pkt.has_bit().unwrap()); // Ticket.Unknown925
    assert_eq!(
        pkt.read_uint8().unwrap(),
        wow_packet::packets::misc::LFG_QUEUE_DUNGEON_LIKE_CPP
    );
    assert_eq!(
        pkt.read_uint8().unwrap(),
        wow_packet::packets::misc::LFG_UPDATE_TYPE_REMOVED_FROM_QUEUE_LIKE_CPP
    );
    assert_eq!(pkt.read_uint32().unwrap(), 0); // Slots.Count
    assert_eq!(pkt.read_uint8().unwrap(), 0); // RequestedRoles
    assert_eq!(pkt.read_uint32().unwrap(), 0); // SuspendedPlayers.Count
    assert_eq!(pkt.read_uint32().unwrap(), 0); // QueueMapID
    assert!(!pkt.has_bit().unwrap()); // IsParty
    assert!(pkt.has_bit().unwrap()); // NotifyUI
    assert!(!pkt.has_bit().unwrap()); // Joined
    assert!(!pkt.has_bit().unwrap()); // LfgJoined
    assert!(!pkt.has_bit().unwrap()); // Queued
    assert!(!pkt.has_bit().unwrap()); // Unused
}

#[tokio::test]
async fn request_lfg_list_blacklist_sends_empty_list_like_cpp_without_locks() {
    let (mut session, send_rx) = make_session();

    session
        .handle_request_lfg_list_blacklist(WorldPacket::new_empty())
        .await;

    let bytes = send_rx.try_recv().expect("LFG blacklist packet");
    assert_eq!(
        u16::from_le_bytes([bytes[0], bytes[1]]),
        ServerOpcodes::LfgListUpdateBlacklist as u16
    );

    let mut pkt = WorldPacket::from_bytes(&bytes[2..]);
    assert_eq!(pkt.read_uint32().unwrap(), 0);
}

#[tokio::test]
async fn df_get_system_info_player_sends_empty_player_info_like_cpp_without_lfg_mgr() {
    let (mut session, send_rx) = make_session();
    let mut request = WorldPacket::new_empty();
    request.write_bit(true); // Player
    request.write_bit(false); // PartyIndex.HasValue
    request.flush_bits();

    session.handle_df_get_system_info(request).await;

    let bytes = send_rx.try_recv().expect("LFG player info packet");
    assert_eq!(
        u16::from_le_bytes([bytes[0], bytes[1]]),
        ServerOpcodes::LfgPlayerInfo as u16
    );

    let mut pkt = WorldPacket::from_bytes(&bytes[2..]);
    assert_eq!(pkt.read_uint32().unwrap(), 0); // Dungeon.Count
    assert!(!pkt.has_bit().unwrap()); // BlackList.PlayerGuid.HasValue
    assert_eq!(pkt.read_uint32().unwrap(), 0); // BlackList.Slot.Count
}

#[test]
fn lfg_lock_status_applies_access_requirement_order_like_cpp() {
    let (mut session, _send_rx) = make_session();
    session.set_loaded_player_identity_like_cpp(571, 1, 1, 80, 0);
    let dungeon = wow_data::LfgDungeonDataLikeCpp {
        id: 205,
        name: "Utgarde Pinnacle".to_string(),
        map: 575,
        type_id: wow_data::LFG_TYPE_DUNGEON_LIKE_CPP,
        expansion: 2,
        group: 5,
        min_level: 80,
        max_level: 83,
        difficulty: 2,
        seasonal: false,
        x: 0.0,
        y: 0.0,
        z: 0.0,
        o: 0.0,
        required_item_level: 0,
        final_dungeon_encounter_id: 0,
    };

    let install_requirement =
        |session: &mut crate::session::WorldSession,
         requirement: wow_data::AccessRequirementLikeCpp| {
            session.set_access_requirement_store(Arc::new(
                wow_data::AccessRequirementStoreLikeCpp::from_entries_like_cpp([requirement]),
            ));
        };

    install_requirement(
        &mut session,
        wow_data::AccessRequirementLikeCpp {
            map_id: dungeon.map,
            difficulty: dungeon.difficulty,
            level_min: 0,
            level_max: 0,
            item: 0,
            item2: 0,
            quest_done_a: 0,
            quest_done_h: 0,
            completed_achievement: 9001,
            quest_failed_text: String::new(),
        },
    );
    assert_eq!(
        session.lfg_lock_status_like_cpp(&dungeon, 80, 2),
        Some(LFG_LOCKSTATUS_MISSING_ACHIEVEMENT_LIKE_CPP)
    );

    session
        .represented_completed_achievements_like_cpp
        .insert(9001);
    install_requirement(
        &mut session,
        wow_data::AccessRequirementLikeCpp {
            map_id: dungeon.map,
            difficulty: dungeon.difficulty,
            level_min: 0,
            level_max: 0,
            item: 0,
            item2: 0,
            quest_done_a: 42,
            quest_done_h: 0,
            completed_achievement: 0,
            quest_failed_text: String::new(),
        },
    );
    assert_eq!(
        session.lfg_lock_status_like_cpp(&dungeon, 80, 2),
        Some(LFG_LOCKSTATUS_QUEST_NOT_COMPLETED_LIKE_CPP)
    );

    session.rewarded_quests.insert(42);
    install_requirement(
        &mut session,
        wow_data::AccessRequirementLikeCpp {
            map_id: dungeon.map,
            difficulty: dungeon.difficulty,
            level_min: 0,
            level_max: 0,
            item: 6948,
            item2: 0,
            quest_done_a: 0,
            quest_done_h: 0,
            completed_achievement: 0,
            quest_failed_text: String::new(),
        },
    );
    assert_eq!(
        session.lfg_lock_status_like_cpp(&dungeon, 80, 2),
        Some(LFG_LOCKSTATUS_MISSING_ITEM_LIKE_CPP)
    );
}

#[test]
fn lfg_reward_uses_other_quest_when_df_first_quest_on_cooldown_like_cpp() {
    let (mut session, _send_rx) = make_session();
    let mut first = quest_template(24_710);
    first.special_flags = QUEST_SPECIAL_FLAGS_DF_QUEST_LIKE_CPP;
    first.reward_currencies[0] = 341;
    first.reward_currency_amounts[0] = 2;
    let mut other = quest_template(24_711);
    other.reward_currencies[0] = 301;
    other.reward_currency_amounts[0] = 5;
    session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([
        first.clone(),
        other.clone(),
    ])));
    session.df_quests_like_cpp.insert(first.id);

    let mut info =
        wow_packet::packets::misc::LfgPlayerDungeonInfo::random_dungeon_like_cpp(100_663_552);
    session.populate_lfg_player_dungeon_reward_like_cpp(
        &mut info,
        &wow_data::LfgDungeonRewardLikeCpp {
            max_level: 80,
            first_quest_id: first.id,
            other_quest_id: other.id,
        },
    );

    assert!(!info.first_reward);
    assert_eq!(info.rewards.currency.len(), 1);
    assert_eq!(info.rewards.currency[0].currency_id, 301);
    assert_eq!(info.rewards.currency[0].quantity, 5);
}

#[test]
fn lfg_reward_uses_first_df_quest_when_not_on_cooldown_like_cpp() {
    let (mut session, _send_rx) = make_session();
    let mut first = quest_template(24_788);
    first.special_flags = QUEST_SPECIAL_FLAGS_DF_QUEST_LIKE_CPP | 0x1;
    first.reward_currencies[0] = 341;
    first.reward_currency_amounts[0] = 2;
    let mut other = quest_template(24_789);
    other.special_flags = 0x1;
    other.reward_currencies[0] = 301;
    other.reward_currency_amounts[0] = 2;
    session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([
        first.clone(),
        other,
    ])));

    let mut info =
        wow_packet::packets::misc::LfgPlayerDungeonInfo::random_dungeon_like_cpp(100_663_558);
    session.populate_lfg_player_dungeon_reward_like_cpp(
        &mut info,
        &wow_data::LfgDungeonRewardLikeCpp {
            max_level: 80,
            first_quest_id: first.id,
            other_quest_id: 24_789,
        },
    );

    assert!(info.first_reward);
    assert_eq!(info.rewards.currency.len(), 1);
    assert_eq!(info.rewards.currency[0].currency_id, 341);
    assert_eq!(info.rewards.currency[0].quantity, 2);
}

#[tokio::test]
async fn df_get_system_info_party_without_group_is_silent_like_cpp() {
    let (mut session, send_rx) = make_session();
    let mut request = WorldPacket::new_empty();
    request.write_bit(false); // Player
    request.write_bit(false); // PartyIndex.HasValue
    request.flush_bits();

    session.handle_df_get_system_info(request).await;

    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn df_get_join_status_without_active_lfg_is_silent_like_cpp() {
    let (mut session, send_rx) = make_session();

    session
        .handle_df_get_join_status(WorldPacket::new_empty())
        .await;

    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn conquest_formula_constants_is_silent_like_cpp_handle_null() {
    let (mut session, send_rx) = make_session();

    session
        .handle_request_conquest_formula_constants(WorldPacket::new_empty())
        .await;

    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn reset_instances_lfg_group_is_silent_like_cpp() {
    let (mut session, send_rx) = make_session();
    let leader = ObjectGuid::create_player(1, 42);
    let group_registry = Arc::new(GroupRegistry::default());
    let mut group = GroupInfo::new(leader);
    group.group_flags |= GROUP_FLAG_LFG_LIKE_CPP;
    let group_guid = group.group_guid;
    group_registry.register_group_like_cpp(group_guid, group);
    let entries = wow_instances::MapDb2Entries {
        map_id: 631,
        difficulty_id: 4,
        lock_id: 10,
        reset_interval: wow_instances::MapDifficultyResetInterval::Weekly,
        max_players: 10,
        is_flex_locking: true,
        is_using_encounter_locks: false,
    };
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let mut mgr = wow_instances::InstanceLockMgr::default();
    mgr.update_instance_lock_for_player_at(
        leader,
        &entries,
        wow_instances::InstanceLockUpdateEvent {
            instance_id: 100,
            new_data: String::new(),
            instance_completed_encounters_mask: 0,
            completed_encounter_bit: None,
            entrance_world_safe_loc_id: None,
        },
        wow_instances::ResetSchedule::default(),
        now,
    );

    session.set_player_guid(Some(leader));
    session.group_guid = Some(group_guid);
    session.set_group_registry(group_registry, Arc::new(PendingInvites::default()));
    session.set_player_map_position_like_cpp(0, Position::ZERO);
    session.set_map_store(Arc::new(MapStore::from_entries([
        MapEntry {
            id: 0,
            instance_type: 0,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
        MapEntry {
            id: 631,
            instance_type: 2,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: wow_data::map::MAP_FLAG_FLEXIBLE_RAID_LOCKING,
            flags2: 0,
        },
    ])));
    session.set_map_difficulty_store(Arc::new(MapDifficultyStore::from_entries([
        MapDifficultyEntry {
            id: 1,
            message: String::new(),
            map_id: 631,
            difficulty_id: 4,
            lock_id: 10,
            reset_interval: 2,
            max_players: 0,
            flags: 0,
        },
    ])));
    let mgr = Arc::new(std::sync::RwLock::new(mgr));
    session.set_instance_lock_mgr(Arc::clone(&mgr));

    session
        .handle_reset_instances(WorldPacket::from_bytes(&[]))
        .await;

    assert!(send_rx.try_recv().is_err());
    assert!(
        mgr.read()
            .unwrap()
            .find_active_instance_lock_at(leader, &entries, now)
            .is_some()
    );
}
