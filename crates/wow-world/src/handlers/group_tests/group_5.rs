//! Group scenarios for [`super`].
//!
//! Split out of group_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

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
    group.group_category = wow_social::group::GROUP_CATEGORY_INSTANCE_LIKE_CPP;
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
            wow_social::group::GROUP_CATEGORY_INSTANCE_LIKE_CPP as i8
        );
        assert_eq!(pkt.read_packed_guid().unwrap(), leader);
    }
}
