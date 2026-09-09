//! Movement scenarios for [`super`].
//!
//! Split out of group_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

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
    let player_registry = Arc::new(PlayerRegistry::with_canonical_player_fixtures_like_cpp());
    player_registry.register_or_replace(
        leader,
        broadcast_info(leader, leader_tx),
        Default::default(),
    );
    player_registry.register_or_replace(
        target,
        broadcast_info_with_command_tx(target, target_tx, target_command_tx),
        Default::default(),
    );
    player_registry.register_or_replace(
        remaining,
        broadcast_info(remaining, remaining_tx),
        Default::default(),
    );
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
        wow_social::group::GROUP_TYPE_NONE_LIKE_CPP
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
