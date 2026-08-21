// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! pvp capability handler tests.

use super::*;
use wow_constants::unit::NPCFlags1;

#[tokio::test]
async fn request_rated_pvp_info_sends_empty_cpp_default_packet_to_realm() {
    let (mut session, instance_rx, realm_rx) = make_session_with_realm_send();

    session
        .handle_request_rated_pvp_info(WorldPacket::new_empty())
        .await;

    assert!(instance_rx.try_recv().is_err());
    let bytes = realm_rx.try_recv().expect("rated pvp info packet");
    assert_eq!(
        u16::from_le_bytes([bytes[0], bytes[1]]),
        ServerOpcodes::RatedPvpInfo as u16
    );
    assert_eq!(
        bytes.len(),
        2 + wow_packet::packets::misc::RATED_PVP_BRACKET_COUNT_LIKE_CPP * (19 * 4 + 1)
    );
}

#[tokio::test]
async fn request_pvp_rewards_is_silent_like_cpp_commented_send() {
    let (mut session, send_rx) = make_session();

    session
        .handle_request_pvp_rewards(WorldPacket::new_empty())
        .await;

    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn request_battlefield_status_without_queues_is_silent_like_cpp() {
    let (mut session, send_rx) = make_session();

    session
        .handle_request_battlefield_status(WorldPacket::new_empty())
        .await;

    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn battlemaster_hello_missing_or_non_battlemaster_is_silent_like_cpp() {
    let (mut session, send_rx) = make_session();
    let missing = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 90_001, 1);

    session
        .handle_battlemaster_hello(battlemaster_hello_packet(missing))
        .await;

    assert!(
        session
            .represented_battlemaster_hellos_like_cpp()
            .is_empty()
    );
    assert!(send_rx.try_recv().is_err());

    let non_battlemaster =
        ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 90_002, 2);
    register_misc_test_creature(&mut session, non_battlemaster, 90_002, 0);

    session
        .handle_battlemaster_hello(battlemaster_hello_packet(non_battlemaster))
        .await;

    assert!(
        session
            .represented_battlemaster_hellos_like_cpp()
            .is_empty()
    );
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn battlemaster_hello_records_represented_list_intent_like_cpp() {
    let (mut session, send_rx) = make_session();
    let battlemaster = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 90_003, 3);

    register_misc_test_creature(
        &mut session,
        battlemaster,
        90_003,
        NPCFlags1::BATTLE_MASTER.bits() as u64,
    );

    session
        .handle_battlemaster_hello(battlemaster_hello_packet(battlemaster))
        .await;

    assert_eq!(
        session.represented_battlemaster_hellos_like_cpp(),
        &[crate::session::RepresentedBattlemasterHelloLikeCpp {
            unit: battlemaster,
            entry: 90_003,
        }]
    );
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn battlefield_list_invalid_or_missing_store_is_silent_like_cpp() {
    let (mut session, send_rx) = make_session();

    session
        .handle_battlefield_list(battlefield_list_packet(3))
        .await;

    assert!(session.represented_battlefield_lists_like_cpp().is_empty());
    assert!(send_rx.try_recv().is_err());

    session.set_battlemaster_list_store(Arc::new(wow_data::BattlemasterListStore::default()));
    session
        .handle_battlefield_list(battlefield_list_packet(3))
        .await;

    assert!(session.represented_battlefield_lists_like_cpp().is_empty());
    assert!(send_rx.try_recv().is_err());

    session.set_battlemaster_list_store(Arc::new(wow_data::BattlemasterListStore::from_entries([
        battlemaster_entry_like_cpp(3, wow_data::MAP_BATTLEGROUND_LIKE_CPP, 0),
    ])));
    session
        .handle_battlefield_list(battlefield_list_packet(-1))
        .await;

    assert!(session.represented_battlefield_lists_like_cpp().is_empty());
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn battlefield_list_records_represented_list_intent_like_cpp() {
    let (mut session, send_rx) = make_session();
    session.set_battlemaster_list_store(Arc::new(wow_data::BattlemasterListStore::from_entries([
        battlemaster_entry_like_cpp(3, wow_data::MAP_BATTLEGROUND_LIKE_CPP, 0),
    ])));

    session
        .handle_battlefield_list(battlefield_list_packet(3))
        .await;

    assert_eq!(
        session.represented_battlefield_lists_like_cpp(),
        &[crate::session::RepresentedBattlefieldListLikeCpp { list_id: 3 }]
    );
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn battlemaster_join_empty_missing_or_invalid_queue_is_silent_like_cpp() {
    let (mut session, send_rx) = make_session();
    let valid_queue_id = battleground_queue_id_like_cpp(3, 0, false, 0);

    session
        .handle_battlemaster_join(battlemaster_join_packet(&[], 0x07, [10, -1]))
        .await;
    assert!(session.represented_battlemaster_joins_like_cpp().is_empty());
    assert!(send_rx.try_recv().is_err());

    session
        .handle_battlemaster_join(battlemaster_join_packet(&[valid_queue_id], 0x07, [10, -1]))
        .await;
    assert!(session.represented_battlemaster_joins_like_cpp().is_empty());
    assert!(send_rx.try_recv().is_err());

    session.set_battlemaster_list_store(Arc::new(wow_data::BattlemasterListStore::from_entries([
        battlemaster_entry_like_cpp(3, wow_data::MAP_BATTLEGROUND_LIKE_CPP, 0),
    ])));
    let invalid_battleground_team_size = battleground_queue_id_like_cpp(3, 0, false, 2);
    session
        .handle_battlemaster_join(battlemaster_join_packet(
            &[invalid_battleground_team_size],
            0x07,
            [10, -1],
        ))
        .await;

    assert!(session.represented_battlemaster_joins_like_cpp().is_empty());
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn battlemaster_join_internal_disabled_or_already_in_bg_is_silent_like_cpp() {
    let (mut internal_session, internal_rx) = make_session();
    let valid_queue_id = battleground_queue_id_like_cpp(3, 0, false, 0);
    internal_session.set_battlemaster_list_store(Arc::new(
        wow_data::BattlemasterListStore::from_entries([battlemaster_entry_like_cpp(
            3,
            wow_data::MAP_BATTLEGROUND_LIKE_CPP,
            wow_data::BATTLEMASTER_LIST_FLAG_INTERNAL_ONLY_LIKE_CPP,
        )]),
    ));
    internal_session
        .handle_battlemaster_join(battlemaster_join_packet(&[valid_queue_id], 0x07, [10, -1]))
        .await;
    assert!(
        internal_session
            .represented_battlemaster_joins_like_cpp()
            .is_empty()
    );
    assert!(internal_rx.try_recv().is_err());

    let (disable_mgr, report) = wow_data::DisableMgrLikeCpp::from_rows_like_cpp(
        [wow_data::DisableDbRowLikeCpp {
            source_type: wow_data::DISABLE_TYPE_BATTLEGROUND,
            entry: 3,
            flags: 0,
            params_0: String::new(),
            params_1: String::new(),
        }],
        wow_data::DisableMgrRefsLikeCpp::default(),
    );
    assert_eq!(report.loaded_count, 1);
    let (mut disabled_session, disabled_rx) = make_session();
    disabled_session.set_battlemaster_list_store(Arc::new(
        wow_data::BattlemasterListStore::from_entries([battlemaster_entry_like_cpp(
            3,
            wow_data::MAP_BATTLEGROUND_LIKE_CPP,
            0,
        )]),
    ));
    disabled_session.set_disable_mgr(Arc::new(disable_mgr));
    disabled_session
        .handle_battlemaster_join(battlemaster_join_packet(&[valid_queue_id], 0x07, [10, -1]))
        .await;
    assert!(
        disabled_session
            .represented_battlemaster_joins_like_cpp()
            .is_empty()
    );
    assert!(disabled_rx.try_recv().is_err());

    let (mut in_bg_session, in_bg_rx) = make_session();
    in_bg_session.set_battlemaster_list_store(Arc::new(
        wow_data::BattlemasterListStore::from_entries([battlemaster_entry_like_cpp(
            3,
            wow_data::MAP_BATTLEGROUND_LIKE_CPP,
            0,
        )]),
    ));
    in_bg_session.set_player_battleground_type_id_like_cpp(3);
    in_bg_session
        .handle_battlemaster_join(battlemaster_join_packet(&[valid_queue_id], 0x07, [10, -1]))
        .await;
    assert!(
        in_bg_session
            .represented_battlemaster_joins_like_cpp()
            .is_empty()
    );
    assert!(in_bg_rx.try_recv().is_err());
}

#[tokio::test]
async fn battlemaster_join_records_represented_queue_intent_like_cpp() {
    let (mut session, send_rx) = make_session();
    let packed_queue_id = battleground_queue_id_like_cpp(3, 0, false, 0);
    session.set_battlemaster_list_store(Arc::new(wow_data::BattlemasterListStore::from_entries([
        battlemaster_entry_like_cpp(3, wow_data::MAP_BATTLEGROUND_LIKE_CPP, 0),
    ])));

    session
        .handle_battlemaster_join(battlemaster_join_packet(&[packed_queue_id], 0x07, [10, -1]))
        .await;

    assert_eq!(
        session.represented_battlemaster_joins_like_cpp(),
        &[crate::session::RepresentedBattlemasterJoinLikeCpp {
            packed_queue_id,
            queue_type_id: crate::session::RepresentedBattlegroundQueueTypeIdLikeCpp {
                battlemaster_list_id: 3,
                queue_type: 0,
                rated: false,
                team_size: 0,
            },
            roles: 0x07,
            blacklist_map: [10, -1],
        }]
    );
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn battlemaster_join_arena_missing_disabled_or_invalid_gates_are_silent_like_cpp() {
    let (mut missing_template_session, missing_rx) = make_session();
    missing_template_session
        .handle_battlemaster_join_arena(battlemaster_join_arena_packet(1, 0x07))
        .await;
    assert!(
        missing_template_session
            .represented_battlemaster_join_arenas_like_cpp()
            .is_empty()
    );
    assert!(missing_rx.try_recv().is_err());

    let (mut invalid_slot_session, invalid_slot_rx) = make_session();
    invalid_slot_session.set_battlemaster_list_store(Arc::new(
        wow_data::BattlemasterListStore::from_entries([battlemaster_entry_like_cpp(
            wow_data::BATTLEGROUND_AA_LIKE_CPP,
            wow_data::MAP_ARENA_LIKE_CPP,
            0,
        )]),
    ));
    invalid_slot_session
        .handle_battlemaster_join_arena(battlemaster_join_arena_packet(3, 0x07))
        .await;
    assert!(
        invalid_slot_session
            .represented_battlemaster_join_arenas_like_cpp()
            .is_empty()
    );
    assert!(invalid_slot_rx.try_recv().is_err());

    let (disable_mgr, report) = wow_data::DisableMgrLikeCpp::from_rows_like_cpp(
        [wow_data::DisableDbRowLikeCpp {
            source_type: wow_data::DISABLE_TYPE_BATTLEGROUND,
            entry: wow_data::BATTLEGROUND_AA_LIKE_CPP,
            flags: 0,
            params_0: String::new(),
            params_1: String::new(),
        }],
        wow_data::DisableMgrRefsLikeCpp::default(),
    );
    assert_eq!(report.loaded_count, 1);
    let (mut disabled_session, disabled_rx) = make_session();
    disabled_session.set_battlemaster_list_store(Arc::new(
        wow_data::BattlemasterListStore::from_entries([battlemaster_entry_like_cpp(
            wow_data::BATTLEGROUND_AA_LIKE_CPP,
            wow_data::MAP_ARENA_LIKE_CPP,
            0,
        )]),
    ));
    disabled_session.set_disable_mgr(Arc::new(disable_mgr));
    disabled_session
        .handle_battlemaster_join_arena(battlemaster_join_arena_packet(1, 0x07))
        .await;
    assert!(
        disabled_session
            .represented_battlemaster_join_arenas_like_cpp()
            .is_empty()
    );
    assert!(disabled_rx.try_recv().is_err());

    let (mut in_bg_session, in_bg_rx) = make_session();
    in_bg_session.set_battlemaster_list_store(Arc::new(
        wow_data::BattlemasterListStore::from_entries([battlemaster_entry_like_cpp(
            wow_data::BATTLEGROUND_AA_LIKE_CPP,
            wow_data::MAP_ARENA_LIKE_CPP,
            0,
        )]),
    ));
    in_bg_session.set_player_battleground_type_id_like_cpp(wow_data::BATTLEGROUND_AA_LIKE_CPP);
    in_bg_session
        .handle_battlemaster_join_arena(battlemaster_join_arena_packet(1, 0x07))
        .await;
    assert!(
        in_bg_session
            .represented_battlemaster_join_arenas_like_cpp()
            .is_empty()
    );
    assert!(in_bg_rx.try_recv().is_err());
}

#[tokio::test]
async fn battlemaster_join_arena_requires_group_leader_like_cpp() {
    let player = ObjectGuid::create_player(1, 42);
    let leader = ObjectGuid::create_player(1, 99);
    let group_registry = Arc::new(GroupRegistry::default());
    let mut group = GroupInfo::new(leader);
    let group_guid = group.group_guid;
    group.members.push(player);
    group_registry.insert(group_guid, group);

    let (mut session, send_rx) = make_session();
    session.set_player_guid(Some(player));
    session.group_guid = Some(group_guid);
    session.set_group_registry(group_registry, Arc::new(PendingInvites::default()));
    session.set_battlemaster_list_store(Arc::new(wow_data::BattlemasterListStore::from_entries([
        battlemaster_entry_like_cpp(
            wow_data::BATTLEGROUND_AA_LIKE_CPP,
            wow_data::MAP_ARENA_LIKE_CPP,
            0,
        ),
    ])));

    session
        .handle_battlemaster_join_arena(battlemaster_join_arena_packet(1, 0x07))
        .await;

    assert!(
        session
            .represented_battlemaster_join_arenas_like_cpp()
            .is_empty()
    );
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn battlemaster_join_arena_records_represented_rated_queue_intent_like_cpp() {
    let player = ObjectGuid::create_player(1, 42);
    let group_registry = Arc::new(GroupRegistry::default());
    let group = GroupInfo::new(player);
    let group_guid = group.group_guid;
    group_registry.insert(group_guid, group);

    let (mut session, send_rx) = make_session();
    session.set_player_guid(Some(player));
    session.group_guid = Some(group_guid);
    session.set_group_registry(group_registry, Arc::new(PendingInvites::default()));
    session.set_battlemaster_list_store(Arc::new(wow_data::BattlemasterListStore::from_entries([
        battlemaster_entry_like_cpp(
            wow_data::BATTLEGROUND_AA_LIKE_CPP,
            wow_data::MAP_ARENA_LIKE_CPP,
            0,
        ),
    ])));

    session
        .handle_battlemaster_join_arena(battlemaster_join_arena_packet(1, 0x07))
        .await;

    assert_eq!(
        session.represented_battlemaster_join_arenas_like_cpp(),
        &[crate::session::RepresentedBattlemasterJoinArenaLikeCpp {
            team_size_index: 1,
            roles: 0x07,
            arena_type: 3,
            group_guid,
            queue_type_id: crate::session::RepresentedBattlegroundQueueTypeIdLikeCpp {
                battlemaster_list_id: wow_data::BATTLEGROUND_AA_LIKE_CPP as u16,
                queue_type: 1,
                rated: true,
                team_size: 3,
            },
        }]
    );
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn battlemaster_join_skirmish_missing_disabled_or_in_bg_gates_are_silent_like_cpp() {
    let (mut missing_template_session, missing_rx) = make_session();
    missing_template_session
        .handle_battlemaster_join_skirmish(battlemaster_join_skirmish_packet(0, 0, 0, 1))
        .await;
    assert!(
        missing_template_session
            .represented_battlemaster_join_skirmishes_like_cpp()
            .is_empty()
    );
    assert!(missing_rx.try_recv().is_err());

    let (disable_mgr, report) = wow_data::DisableMgrLikeCpp::from_rows_like_cpp(
        [wow_data::DisableDbRowLikeCpp {
            source_type: wow_data::DISABLE_TYPE_BATTLEGROUND,
            entry: wow_data::BATTLEGROUND_AA_LIKE_CPP,
            flags: 0,
            params_0: String::new(),
            params_1: String::new(),
        }],
        wow_data::DisableMgrRefsLikeCpp::default(),
    );
    assert_eq!(report.loaded_count, 1);
    let (mut disabled_session, disabled_rx) = make_session();
    disabled_session.set_battlemaster_list_store(Arc::new(
        wow_data::BattlemasterListStore::from_entries([battlemaster_entry_like_cpp(
            wow_data::BATTLEGROUND_AA_LIKE_CPP,
            wow_data::MAP_ARENA_LIKE_CPP,
            0,
        )]),
    ));
    disabled_session.set_disable_mgr(Arc::new(disable_mgr));
    disabled_session
        .handle_battlemaster_join_skirmish(battlemaster_join_skirmish_packet(0, 0, 0, 0))
        .await;
    assert!(
        disabled_session
            .represented_battlemaster_join_skirmishes_like_cpp()
            .is_empty()
    );
    assert!(disabled_rx.try_recv().is_err());

    let (mut in_bg_session, in_bg_rx) = make_session();
    in_bg_session.set_battlemaster_list_store(Arc::new(
        wow_data::BattlemasterListStore::from_entries([battlemaster_entry_like_cpp(
            wow_data::BATTLEGROUND_AA_LIKE_CPP,
            wow_data::MAP_ARENA_LIKE_CPP,
            0,
        )]),
    ));
    in_bg_session.set_player_battleground_type_id_like_cpp(wow_data::BATTLEGROUND_AA_LIKE_CPP);
    in_bg_session
        .handle_battlemaster_join_skirmish(battlemaster_join_skirmish_packet(0, 0, 0, 0))
        .await;
    assert!(
        in_bg_session
            .represented_battlemaster_join_skirmishes_like_cpp()
            .is_empty()
    );
    assert!(in_bg_rx.try_recv().is_err());
}

#[tokio::test]
async fn battlemaster_join_skirmish_group_request_requires_group_leader_like_cpp() {
    let player = ObjectGuid::create_player(1, 42);
    let leader = ObjectGuid::create_player(1, 99);
    let group_registry = Arc::new(GroupRegistry::default());
    let mut group = GroupInfo::new(leader);
    let group_guid = group.group_guid;
    group.members.push(player);
    group_registry.insert(group_guid, group);

    let (mut session, send_rx) = make_session();
    session.set_player_guid(Some(player));
    session.group_guid = Some(group_guid);
    session.set_group_registry(group_registry, Arc::new(PendingInvites::default()));
    session.set_battlemaster_list_store(Arc::new(wow_data::BattlemasterListStore::from_entries([
        battlemaster_entry_like_cpp(
            wow_data::BATTLEGROUND_AA_LIKE_CPP,
            wow_data::MAP_ARENA_LIKE_CPP,
            0,
        ),
    ])));

    session
        .handle_battlemaster_join_skirmish(battlemaster_join_skirmish_packet(5, 0, 1, 0))
        .await;

    assert!(
        session
            .represented_battlemaster_join_skirmishes_like_cpp()
            .is_empty()
    );
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn battlemaster_join_skirmish_records_solo_and_group_intents_like_cpp() {
    let player = ObjectGuid::create_player(1, 42);
    let group_registry = Arc::new(GroupRegistry::default());
    let group = GroupInfo::new(player);
    let group_guid = group.group_guid;
    group_registry.insert(group_guid, group);

    let (mut session, send_rx) = make_session();
    session.set_player_guid(Some(player));
    session.group_guid = Some(group_guid);
    session.set_group_registry(group_registry, Arc::new(PendingInvites::default()));
    session.set_battlemaster_list_store(Arc::new(wow_data::BattlemasterListStore::from_entries([
        battlemaster_entry_like_cpp(
            wow_data::BATTLEGROUND_AA_LIKE_CPP,
            wow_data::MAP_ARENA_LIKE_CPP,
            0,
        ),
    ])));

    session
        .handle_battlemaster_join_skirmish(battlemaster_join_skirmish_packet(0, 0, 0, 1))
        .await;
    session
        .handle_battlemaster_join_skirmish(battlemaster_join_skirmish_packet(0, 3, 1, 0))
        .await;

    assert_eq!(
        session.represented_battlemaster_join_skirmishes_like_cpp(),
        &[
            crate::session::RepresentedBattlemasterJoinSkirmishLikeCpp {
                bg_type_id: 0,
                bracket_id: 0,
                as_group: false,
                is_rated_packet_value: 1,
                arena_type: 2,
                group_guid: None,
                queue_type_id: crate::session::RepresentedBattlegroundQueueTypeIdLikeCpp {
                    battlemaster_list_id: wow_data::BATTLEGROUND_AA_LIKE_CPP as u16,
                    queue_type: 4,
                    rated: false,
                    team_size: 2,
                },
            },
            crate::session::RepresentedBattlemasterJoinSkirmishLikeCpp {
                bg_type_id: 0,
                bracket_id: 3,
                as_group: true,
                is_rated_packet_value: 0,
                arena_type: 3,
                group_guid: Some(group_guid),
                queue_type_id: crate::session::RepresentedBattlegroundQueueTypeIdLikeCpp {
                    battlemaster_list_id: wow_data::BATTLEGROUND_AA_LIKE_CPP as u16,
                    queue_type: 4,
                    rated: false,
                    team_size: 3,
                },
            },
        ]
    );
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn battlefield_port_not_queued_invalid_slot_or_missing_invite_is_silent_like_cpp() {
    let (mut session, send_rx) = make_session();
    let requester = ObjectGuid::create_player(1, 42);
    let queue_type_id = crate::session::RepresentedBattlegroundQueueTypeIdLikeCpp {
        battlemaster_list_id: 3,
        queue_type: 0,
        rated: false,
        team_size: 0,
    };

    session
        .handle_battlefield_port(battlefield_port_packet(requester, 1, 2, 1_234, false, true))
        .await;
    assert!(session.represented_battlefield_ports_like_cpp().is_empty());
    assert!(send_rx.try_recv().is_err());

    session.add_represented_battleground_queue_slot_like_cpp(1, queue_type_id, 0);
    session
        .handle_battlefield_port(battlefield_port_packet(requester, 2, 2, 1_234, false, true))
        .await;
    assert!(session.represented_battlefield_ports_like_cpp().is_empty());
    assert!(send_rx.try_recv().is_err());

    session
        .handle_battlefield_port(battlefield_port_packet(requester, 1, 2, 1_234, false, true))
        .await;
    assert!(session.represented_battlefield_ports_like_cpp().is_empty());
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn battlefield_port_records_accept_and_leave_intents_like_cpp() {
    let (mut session, send_rx) = make_session();
    let requester = ObjectGuid::create_player(1, 42);
    let queue_type_id = crate::session::RepresentedBattlegroundQueueTypeIdLikeCpp {
        battlemaster_list_id: 3,
        queue_type: 0,
        rated: false,
        team_size: 0,
    };
    session.add_represented_battleground_queue_slot_like_cpp(1, queue_type_id, 77);

    session
        .handle_battlefield_port(battlefield_port_packet(requester, 1, 2, 1_234, true, true))
        .await;
    session
        .handle_battlefield_port(battlefield_port_packet(
            requester, 1, 2, 1_235, false, false,
        ))
        .await;

    let ports = session.represented_battlefield_ports_like_cpp();
    assert_eq!(ports.len(), 2);
    assert_eq!(ports[0].ticket.requester_guid, requester);
    assert_eq!(ports[0].ticket.id, 1);
    assert_eq!(ports[0].ticket.ride_type, 2);
    assert_eq!(ports[0].ticket.time, 1_234);
    assert!(ports[0].ticket.unknown925);
    assert!(ports[0].accepted_invite);
    assert_eq!(ports[0].queue_type_id, queue_type_id);
    assert_eq!(ports[0].invited_instance_guid, 77);
    assert!(!ports[1].accepted_invite);
    assert_eq!(ports[1].ticket.time, 1_235);
    assert_eq!(ports[1].queue_type_id, queue_type_id);
    assert_eq!(ports[1].invited_instance_guid, 77);
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn battlefield_leave_records_request_when_not_in_combat_like_cpp() {
    let (mut session, send_rx) = make_session();
    session.set_player_battleground_type_id_like_cpp(3);
    session.set_represented_battleground_status_like_cpp(Some(2));

    session
        .handle_battlefield_leave(WorldPacket::new_empty())
        .await;

    assert_eq!(
        session.represented_battleground_leave_requests_like_cpp(),
        1
    );
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn battlefield_leave_rejects_in_combat_active_battleground_like_cpp() {
    let (mut session, send_rx) = make_session();
    session.set_player_battleground_type_id_like_cpp(3);
    session.set_represented_battleground_status_like_cpp(Some(2));
    session.in_combat = true;

    session
        .handle_battlefield_leave(WorldPacket::new_empty())
        .await;

    assert_eq!(
        session.represented_battleground_leave_requests_like_cpp(),
        0
    );
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn battlefield_leave_allows_wait_leave_even_in_combat_like_cpp() {
    let (mut session, send_rx) = make_session();
    session.set_player_battleground_type_id_like_cpp(3);
    session.set_represented_battleground_status_like_cpp(Some(4));
    session.in_combat = true;

    session
        .handle_battlefield_leave(WorldPacket::new_empty())
        .await;

    assert_eq!(
        session.represented_battleground_leave_requests_like_cpp(),
        1
    );
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn accept_wargame_invite_missing_inviter_is_silent_like_cpp() {
    let (mut session, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 100);
    let group_registry = Arc::new(GroupRegistry::default());
    let player_registry = Arc::new(PlayerRegistry::default());
    let group = GroupInfo::new(player_guid);
    let group_guid = group.group_guid;
    group_registry.insert(group_guid, group);
    session.set_player_guid(Some(player_guid));
    session.group_guid = Some(group_guid);
    session.set_player_registry(player_registry);
    session.set_group_registry(group_registry, Arc::new(PendingInvites::default()));

    session
        .handle_accept_wargame_invite(accept_wargame_invite_packet("Missing"))
        .await;

    assert!(
        session
            .represented_wargame_invite_acceptances_like_cpp()
            .is_empty()
    );
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn accept_wargame_invite_records_ready_to_queue_when_groups_match_like_cpp() {
    let (mut session, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 100);
    let player_ally_guid = ObjectGuid::create_player(1, 101);
    let inviter_guid = ObjectGuid::create_player(1, 200);
    let inviter_ally_guid = ObjectGuid::create_player(1, 201);
    let group_registry = Arc::new(GroupRegistry::default());
    let player_registry = Arc::new(PlayerRegistry::default());

    let mut player_group = GroupInfo::new(player_guid);
    player_group.members.push(player_ally_guid);
    let player_group_guid = player_group.group_guid;
    group_registry.insert(player_group_guid, player_group);

    let mut inviter_group = GroupInfo::new(inviter_guid);
    inviter_group.members.push(inviter_ally_guid);
    let inviter_group_guid = inviter_group.group_guid;
    group_registry.insert(inviter_group_guid, inviter_group);

    let (command_tx, _command_rx) = flume::bounded::<SessionCommand>(4);
    let mut inviter_info = broadcast_info_with_command_tx(command_tx);
    inviter_info.player_name = "Inviter".to_string();
    player_registry.insert(inviter_guid, inviter_info);

    session.set_player_guid(Some(player_guid));
    session.set_loaded_player_name_like_cpp("Player".to_string());
    session.group_guid = Some(player_group_guid);
    session.set_player_registry(player_registry);
    session.set_group_registry(group_registry, Arc::new(PendingInvites::default()));

    session
        .handle_accept_wargame_invite(accept_wargame_invite_packet("inviter"))
        .await;

    assert_eq!(
        session.represented_wargame_invite_acceptances_like_cpp(),
        &[crate::session::RepresentedWargameInviteAcceptanceLikeCpp {
            inviter_name: "inviter".to_string(),
            inviter_guid,
            player_group_guid,
            inviter_group_guid,
            group_size: 2,
        }]
    );
    assert!(send_rx.try_recv().is_err());
}
