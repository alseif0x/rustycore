//! Spell scenarios for [`super`].
//!
//! Split out of group_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

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
async fn set_role_absent_target_broadcasts_but_does_not_mutate_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send();
    let leader = ObjectGuid::create_player(1, 42);
    let absent = ObjectGuid::create_player(1, 99);
    let group_registry = Arc::new(GroupRegistry::default());
    let group = GroupInfo::new(leader);
    let group_guid = group.group_guid;
    group_registry.register_group_like_cpp(group_guid, group);
    let player_registry = Arc::new(PlayerRegistry::with_canonical_player_fixtures_like_cpp());
    let (leader_tx, leader_rx) = bounded(8);
    player_registry.register_or_replace(
        leader,
        broadcast_info(leader, leader_tx),
        Default::default(),
    );

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
async fn random_roll_with_group_broadcasts_to_all_members_including_sender_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
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
