//! Misc scenarios for [`super`].
//!
//! Split out of group_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

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
    group.lfg_db_state = Some(wow_social::group::GroupLfgDbStateLikeCpp {
        dungeon_id: 100,
        state: Some(wow_social::group::LFG_STATE_FINISHED_DUNGEON_LIKE_CPP),
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
fn ready_check_fanout_sends_events_only_to_connected_members_like_cpp() {
    let leader = ObjectGuid::create_player(1, 42);
    let member = ObjectGuid::create_player(1, 43);
    let offline = ObjectGuid::create_player(1, 44);
    let mut group = GroupInfo::new(leader);
    group.add_member(member);
    group.add_member(offline);

    let registry = PlayerRegistry::with_canonical_player_fixtures_like_cpp();
    let (leader_tx, leader_rx) = bounded(8);
    let (member_tx, member_rx) = bounded(8);
    registry.register_or_replace(
        leader,
        broadcast_info(leader, leader_tx),
        Default::default(),
    );
    registry.register_or_replace(
        member,
        broadcast_info(member, member_tx),
        Default::default(),
    );
    let _canonical = bind_canonical_party_players_like_cpp(&registry, [leader, member]);

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
    let player_registry = Arc::new(PlayerRegistry::with_canonical_player_fixtures_like_cpp());
    let (leader_tx, leader_rx) = bounded(8);
    player_registry.register_or_replace(
        leader,
        broadcast_info(leader, leader_tx),
        Default::default(),
    );

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
    let player_registry = Arc::new(PlayerRegistry::with_canonical_player_fixtures_like_cpp());
    let (leader_tx, leader_rx) = bounded(8);
    let (assistant_tx, assistant_rx) = bounded(8);
    player_registry.register_or_replace(
        leader,
        broadcast_info(leader, leader_tx),
        Default::default(),
    );
    player_registry.register_or_replace(
        assistant,
        broadcast_info(assistant, assistant_tx),
        Default::default(),
    );

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
async fn set_everyone_is_assistant_leader_applies_to_all_members_and_fans_out_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    let leader = ObjectGuid::create_player(1, 42);
    let member = ObjectGuid::create_player(1, 43);
    let group_registry = Arc::new(GroupRegistry::default());
    let mut group = GroupInfo::new(leader);
    group.add_member(member);
    let group_guid = group.group_guid;
    group_registry.register_group_like_cpp(group_guid, group);

    let player_registry = Arc::new(PlayerRegistry::with_canonical_player_fixtures_like_cpp());
    let (leader_tx, leader_rx) = bounded(8);
    let (member_tx, member_rx) = bounded(8);
    player_registry.register_or_replace(
        leader,
        broadcast_info(leader, leader_tx),
        Default::default(),
    );
    player_registry.register_or_replace(
        member,
        broadcast_info(member, member_tx),
        Default::default(),
    );

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
        group.group_flags & wow_social::group::GROUP_FLAG_EVERYONE_ASSISTANT_LIKE_CPP,
        wow_social::group::GROUP_FLAG_EVERYONE_ASSISTANT_LIKE_CPP
    );
    for guid in [leader, member] {
        assert_eq!(
            group.member_slot_like_cpp(guid).unwrap().flags
                & wow_social::group::MEMBER_FLAG_ASSISTANT_LIKE_CPP,
            wow_social::group::MEMBER_FLAG_ASSISTANT_LIKE_CPP
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

    let player_registry = Arc::new(PlayerRegistry::with_canonical_player_fixtures_like_cpp());
    let (leader_tx, leader_rx) = bounded(8);
    let (member_tx, member_rx) = bounded(8);
    player_registry.register_or_replace(
        leader,
        broadcast_info(leader, leader_tx),
        Default::default(),
    );
    player_registry.register_or_replace(
        member,
        broadcast_info(member, member_tx),
        Default::default(),
    );

    session.set_player_guid(Some(leader));
    session.group_guid = Some(group_guid);
    session.set_player_registry(Arc::clone(&player_registry));
    session.set_group_registry(group_registry.clone(), Arc::new(PendingInvites::default()));

    session
        .handle_set_everyone_is_assistant(set_everyone_is_assistant_packet(false, None))
        .await;

    let group = group_registry.get(&group_guid).unwrap();
    assert_eq!(
        group.group_flags & wow_social::group::GROUP_FLAG_EVERYONE_ASSISTANT_LIKE_CPP,
        0
    );
    for guid in [leader, member] {
        assert_eq!(
            group.member_slot_like_cpp(guid).unwrap().flags
                & wow_social::group::MEMBER_FLAG_ASSISTANT_LIKE_CPP,
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

    let player_registry = Arc::new(PlayerRegistry::with_canonical_player_fixtures_like_cpp());
    let (leader_tx, leader_rx) = bounded(8);
    let (member_tx, member_rx) = bounded(8);
    player_registry.register_or_replace(
        leader,
        broadcast_info(leader, leader_tx),
        Default::default(),
    );
    player_registry.register_or_replace(
        member,
        broadcast_info(member, member_tx),
        Default::default(),
    );

    session.set_player_guid(Some(member));
    session.group_guid = Some(group_guid);
    session.set_player_registry(Arc::clone(&player_registry));
    session.set_group_registry(group_registry.clone(), Arc::new(PendingInvites::default()));

    session
        .handle_set_everyone_is_assistant(set_everyone_is_assistant_packet(true, None))
        .await;

    let group = group_registry.get(&group_guid).unwrap();
    assert_eq!(
        group.group_flags & wow_social::group::GROUP_FLAG_EVERYONE_ASSISTANT_LIKE_CPP,
        0
    );
    assert_eq!(
        group.member_slot_like_cpp(member).unwrap().flags
            & wow_social::group::MEMBER_FLAG_ASSISTANT_LIKE_CPP,
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
    group_registry.register_group_like_cpp(group_guid, group);

    let player_registry = Arc::new(PlayerRegistry::with_canonical_player_fixtures_like_cpp());
    let (leader_tx, leader_rx) = bounded(8);
    let (member_tx, member_rx) = bounded(8);
    player_registry.register_or_replace(
        leader,
        broadcast_info(leader, leader_tx),
        Default::default(),
    );
    player_registry.register_or_replace(
        member,
        broadcast_info(member, member_tx),
        Default::default(),
    );

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
        Some(wow_social::group::MEMBER_FLAG_ASSISTANT_LIKE_CPP)
    );
    let group_guid = group.group_guid;
    group_registry.register_group_like_cpp(group_guid, group);

    let player_registry = Arc::new(PlayerRegistry::with_canonical_player_fixtures_like_cpp());
    let (target_tx, target_rx) = bounded(8);
    player_registry.register_or_replace(
        target,
        broadcast_info(target, target_tx),
        Default::default(),
    );

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
