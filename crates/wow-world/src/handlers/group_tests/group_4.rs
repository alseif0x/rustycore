//! Group scenarios for [`super`].
//!
//! Split out of group_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

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
            & wow_social::group::MEMBER_FLAG_ASSISTANT_LIKE_CPP,
        wow_social::group::MEMBER_FLAG_ASSISTANT_LIKE_CPP
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
            & wow_social::group::MEMBER_FLAG_ASSISTANT_LIKE_CPP,
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
        Some(wow_social::group::MEMBER_FLAG_ASSISTANT_LIKE_CPP)
    );
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
        .handle_set_party_leader(set_party_leader_packet(member, Some(0)))
        .await;

    assert!(send_rx.try_recv().is_err());
    let group = group_registry.get(&group_guid).unwrap();
    assert_eq!(group.leader_guid, member);
    assert_eq!(
        group.member_slot_like_cpp(member).unwrap().flags
            & wow_social::group::MEMBER_FLAG_ASSISTANT_LIKE_CPP,
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

    let player_registry = Arc::new(PlayerRegistry::with_canonical_player_fixtures_like_cpp());
    let (member_tx, member_rx) = bounded(8);
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

    let player_registry = Arc::new(PlayerRegistry::with_canonical_player_fixtures_like_cpp());
    let (leader_tx, leader_rx) = bounded(8);
    let (first_tx, first_rx) = bounded(8);
    let (second_tx, second_rx) = bounded(8);
    player_registry.register_or_replace(
        leader,
        broadcast_info(leader, leader_tx),
        Default::default(),
    );
    player_registry.register_or_replace(first, broadcast_info(first, first_tx), Default::default());
    player_registry.register_or_replace(
        second,
        broadcast_info(second, second_tx),
        Default::default(),
    );

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

    let player_registry = Arc::new(PlayerRegistry::with_canonical_player_fixtures_like_cpp());
    let (leader_tx, _leader_rx) = bounded(8);
    let (assistant_tx, _assistant_rx) = bounded(8);
    let (first_tx, first_rx) = bounded(8);
    let (second_tx, second_rx) = bounded(8);
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
    player_registry.register_or_replace(first, broadcast_info(first, first_tx), Default::default());
    player_registry.register_or_replace(
        second,
        broadcast_info(second, second_tx),
        Default::default(),
    );

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
            wow_social::group::MEMBER_FLAG_ASSISTANT_LIKE_CPP,
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

    let player_registry = Arc::new(PlayerRegistry::with_canonical_player_fixtures_like_cpp());
    let (first_tx, first_rx) = bounded(8);
    let (second_tx, second_rx) = bounded(8);
    player_registry.register_or_replace(first, broadcast_info(first, first_tx), Default::default());
    player_registry.register_or_replace(
        second,
        broadcast_info(second, second_tx),
        Default::default(),
    );

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
async fn minimap_ping_party_index_none_keeps_home_fanout_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send();
    let sender = ObjectGuid::create_player(1, 42);
    let other = ObjectGuid::create_player(1, 43);
    let group_registry = Arc::new(GroupRegistry::default());
    let mut group = GroupInfo::new(sender);
    group.add_member(other);
    let group_guid = group.group_guid;
    group_registry.register_group_like_cpp(group_guid, group);

    let player_registry = Arc::new(PlayerRegistry::with_canonical_player_fixtures_like_cpp());
    let (sender_tx, sender_rx) = bounded(8);
    let (other_tx, other_rx) = bounded(8);
    player_registry.register_or_replace(
        sender,
        broadcast_info(sender, sender_tx),
        Default::default(),
    );
    player_registry.register_or_replace(other, broadcast_info(other, other_tx), Default::default());

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

    let player_registry = Arc::new(PlayerRegistry::with_canonical_player_fixtures_like_cpp());
    let (stale_tx, stale_rx) = bounded(8);
    let (real_tx, real_rx) = bounded(8);
    player_registry.register_or_replace(
        stale_member,
        broadcast_info(stale_member, stale_tx),
        Default::default(),
    );
    player_registry.register_or_replace(
        real_member,
        broadcast_info(real_member, real_tx),
        Default::default(),
    );

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

    let player_registry = Arc::new(PlayerRegistry::with_canonical_player_fixtures_like_cpp());
    let (stale_tx, stale_rx) = bounded(8);
    let (real_tx, _real_rx) = bounded(8);
    player_registry.register_or_replace(
        stale_member,
        broadcast_info(stale_member, stale_tx),
        Default::default(),
    );
    player_registry.register_or_replace(
        real_member,
        broadcast_info(real_member, real_tx),
        Default::default(),
    );

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
