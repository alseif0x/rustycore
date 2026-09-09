//! Group scenarios for [`super`].
//!
//! Split out of group_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

#[test]
fn party_update_serializes_raid_group_flag_like_cpp() {
    let leader = ObjectGuid::create_player(1, 42);
    let (tx, rx) = bounded(8);
    let registry = PlayerRegistry::with_canonical_player_fixtures_like_cpp();
    registry.register_or_replace(leader, broadcast_info(leader, tx), Default::default());
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
        wow_social::group::GROUP_FLAG_RAID_LIKE_CPP
    );
}
#[test]
fn group_insert_intent_maps_to_sqlx_free_command_like_cpp() {
    let leader = ObjectGuid::create_player(1, 42);
    let group = GroupInfo::new(leader);
    let command = group_persistence_command_like_cpp(
        wow_social::group::GroupPersistenceIntentLikeCpp::InsertGroup {
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
    assert_eq!(
        command,
        wow_persistence::RepresentedGroupPersistenceCommandLikeCpp::InsertGroup {
            db_store_id: 77,
            leader_guid: leader.counter() as u64,
            loot_method: wow_social::group::LOOT_METHOD_PERSONAL_LIKE_CPP,
            looter_guid: leader.counter() as u64,
            loot_threshold: 2,
            group_flags: 0,
            dungeon_difficulty_id: 1,
            raid_difficulty_id: 14,
            legacy_raid_difficulty_id: 3,
            master_looter_guid: 0,
        }
    );
}
#[test]
fn group_leave_intents_map_to_order_preserving_sqlx_free_commands_like_cpp() {
    let old_member = ObjectGuid::create_player(1, 42);
    let new_leader = ObjectGuid::create_player(1, 77);
    assert_eq!(
        [
            group_persistence_command_like_cpp(
                wow_social::group::GroupPersistenceIntentLikeCpp::DeleteMember {
                    member_guid: old_member,
                }
            ),
            group_persistence_command_like_cpp(
                wow_social::group::GroupPersistenceIntentLikeCpp::UpdateLeader {
                    db_store_id: 99,
                    leader_guid: new_leader,
                }
            ),
            group_persistence_command_like_cpp(
                wow_social::group::GroupPersistenceIntentLikeCpp::DeleteGroup { db_store_id: 99 }
            ),
        ],
        [
            wow_persistence::RepresentedGroupPersistenceCommandLikeCpp::DeleteMember {
                member_guid: old_member.counter() as u64,
            },
            wow_persistence::RepresentedGroupPersistenceCommandLikeCpp::UpdateLeader {
                db_store_id: 99,
                leader_guid: new_leader.counter() as u64,
            },
            wow_persistence::RepresentedGroupPersistenceCommandLikeCpp::DeleteGroup {
                db_store_id: 99,
            },
        ]
    );
}
#[tokio::test]
async fn represented_group_persistence_seam_preserves_intent_order_and_sequential_mode() {
    let (mut session, _) = make_session_with_send();
    let port = RecordingGroupPersistencePortLikeCpp::new(
        RepresentedGroupPersistenceOutcomeLikeCpp::Applied { command_count: 2 },
    );
    session.set_represented_group_persistence_port_like_cpp(port.clone());

    session
        .persist_group_intents_like_cpp(
            99,
            vec![
                wow_social::group::GroupPersistenceIntentLikeCpp::DeleteMember {
                    member_guid: ObjectGuid::create_player(1, 42),
                },
                wow_social::group::GroupPersistenceIntentLikeCpp::UpdateLeader {
                    db_store_id: 99,
                    leader_guid: ObjectGuid::create_player(1, 77),
                },
            ],
        )
        .await;

    assert_eq!(
        port.requests.lock().unwrap().as_slice(),
        &[RepresentedGroupPersistenceRequestLikeCpp {
            commands: vec![
                wow_persistence::RepresentedGroupPersistenceCommandLikeCpp::DeleteMember {
                    member_guid: 42,
                },
                wow_persistence::RepresentedGroupPersistenceCommandLikeCpp::UpdateLeader {
                    db_store_id: 99,
                    leader_guid: 77,
                },
            ],
            mode: wow_persistence::RepresentedGroupPersistenceModeLikeCpp::Sequential,
        }]
    );
}
#[tokio::test]
async fn represented_group_persistence_seam_accepts_typed_prefix_failure_without_db_types() {
    let (mut session, _) = make_session_with_send();
    let port = RecordingGroupPersistencePortLikeCpp::new(
        RepresentedGroupPersistenceOutcomeLikeCpp::FailedAfterPrefix {
            applied: 1,
            reason: "second command failed".to_owned(),
        },
    );
    session.set_represented_group_persistence_port_like_cpp(port.clone());

    session
        .persist_group_intents_like_cpp(
            99,
            vec![wow_social::group::GroupPersistenceIntentLikeCpp::DeleteGroup { db_store_id: 99 }],
        )
        .await;

    assert_eq!(port.requests.lock().unwrap().len(), 1);
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

    let registry = PlayerRegistry::with_canonical_player_fixtures_like_cpp();
    let (tx, _rx) = bounded(1);
    registry.register_or_replace(connected, broadcast_info(connected, tx), Default::default());

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
    let player_registry = Arc::new(PlayerRegistry::with_canonical_player_fixtures_like_cpp());
    player_registry.register_or_replace(
        last_guid,
        broadcast_info_with_command_tx(last_guid, last_send_tx, last_command_tx),
        Default::default(),
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
        wow_social::group::GROUP_TYPE_NONE_LIKE_CPP
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

    let player_registry = Arc::new(PlayerRegistry::with_canonical_player_fixtures_like_cpp());
    let (target_tx, target_rx) = bounded(8);
    player_registry.register_or_replace(
        target,
        broadcast_info(target, target_tx),
        Default::default(),
    );

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
            Some(wow_social::group::GROUP_CATEGORY_INSTANCE_LIKE_CPP),
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

    let player_registry = Arc::new(PlayerRegistry::with_canonical_player_fixtures_like_cpp());
    let (target_tx, target_rx) = bounded(8);
    player_registry.register_or_replace(
        target,
        broadcast_info(target, target_tx),
        Default::default(),
    );
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

    let player_registry = Arc::new(PlayerRegistry::with_canonical_player_fixtures_like_cpp());
    let (target_instance_tx, target_instance_rx) = bounded(8);
    let (target_command_tx, target_command_rx) = bounded(8);
    player_registry.register_or_replace(
        target,
        broadcast_info_with_command_tx(target, target_instance_tx, target_command_tx),
        Default::default(),
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

    let player_registry = Arc::new(PlayerRegistry::with_canonical_player_fixtures_like_cpp());
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
    player_registry.register_or_replace(
        target,
        broadcast_info_with_command_tx(target, target_send_tx, target_command_tx.clone()),
        Default::default(),
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
async fn group_new_leader_fanout_queues_realm_commands_like_cpp() {
    let leader = ObjectGuid::create_player(1, 42);
    let member = ObjectGuid::create_player(1, 77);
    let mut group = GroupInfo::new(leader);
    group.add_member(member);
    let registry = PlayerRegistry::with_canonical_player_fixtures_like_cpp();
    let (leader_instance_tx, leader_instance_rx) = bounded(4);
    let (leader_command_tx, leader_command_rx) = bounded(4);
    registry.register_or_replace(
        leader,
        broadcast_info_with_command_tx(leader, leader_instance_tx, leader_command_tx),
        Default::default(),
    );
    let (member_instance_tx, member_instance_rx) = bounded(4);
    let (member_command_tx, member_command_rx) = bounded(4);
    registry.register_or_replace(
        member,
        broadcast_info_with_command_tx(member, member_instance_tx, member_command_tx),
        Default::default(),
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

    let player_registry = Arc::new(PlayerRegistry::with_canonical_player_fixtures_like_cpp());
    let (target_tx, target_rx) = bounded(8);
    player_registry.register_or_replace(
        target,
        broadcast_info(target, target_tx),
        Default::default(),
    );

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

    let player_registry = Arc::new(PlayerRegistry::with_canonical_player_fixtures_like_cpp());
    let (target_tx, target_rx) = bounded(8);
    player_registry.register_or_replace(
        target,
        broadcast_info(target, target_tx),
        Default::default(),
    );

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

    let player_registry = Arc::new(PlayerRegistry::with_canonical_player_fixtures_like_cpp());
    let (target_tx, target_rx) = bounded(8);
    player_registry.register_or_replace(
        target,
        broadcast_info(target, target_tx),
        Default::default(),
    );

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

    let player_registry = Arc::new(PlayerRegistry::with_canonical_player_fixtures_like_cpp());
    let (target_tx, target_rx) = bounded(8);
    let mut target_info = broadcast_info(target, target_tx);
    player_registry.register_or_replace(target, target_info, Default::default());
    let canonical = bind_canonical_party_players_like_cpp(&player_registry, [target]);
    crate::canonical_player_access::with_canonical_player_at_mut_like_cpp(
        &canonical,
        target,
        0,
        0,
        |player| player.set_game_master_like_cpp(true),
    )
    .unwrap();
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

    let player_registry = Arc::new(PlayerRegistry::with_canonical_player_fixtures_like_cpp());
    let (target_tx, target_rx) = bounded(8);
    let mut target_info = broadcast_info(target, target_tx);
    target_info.identity.race = 2; // Orc/Horde, while inviter identity below is Human/Alliance.
    player_registry.register_or_replace(target, target_info, Default::default());
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

    let player_registry = Arc::new(PlayerRegistry::with_canonical_player_fixtures_like_cpp());
    let (target_tx, target_rx) = bounded(8);
    let mut target_info = broadcast_info(target, target_tx);
    player_registry.register_or_replace(target, target_info, Default::default());
    let canonical = bind_canonical_party_players_like_cpp(&player_registry, [target]);
    crate::canonical_player_access::with_canonical_player_at_mut_like_cpp(
        &canonical,
        target,
        0,
        0,
        |player| player.set_game_master_like_cpp(true),
    )
    .unwrap();
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

    let player_registry = Arc::new(PlayerRegistry::with_canonical_player_fixtures_like_cpp());
    let (target_tx, target_rx) = bounded(8);
    let mut target_info = broadcast_info(target, target_tx);
    target_info.identity.race = 2;
    player_registry.register_or_replace(target, target_info, Default::default());
    let pending_invites = Arc::new(PendingInvites::default());

    session.set_player_guid(Some(inviter));
    session.set_loaded_player_identity_like_cpp(0, 1, 1, 80, 0);
    let policy = GroupInvitePolicyLikeCpp {
        allow_two_side_interaction: true,
        ..GroupInvitePolicyLikeCpp::default()
    };
    session.set_player_registry(player_registry);
    session.set_group_registry(
        Arc::new(GroupRegistry::default()),
        Arc::clone(&pending_invites),
    );

    session
        .handle_party_invite_with_policy_like_cpp(
            party_invite_packet(target, &target_name, None, 0),
            &policy,
        )
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

    let player_registry = Arc::new(PlayerRegistry::with_canonical_player_fixtures_like_cpp());
    let (target_tx, target_rx) = bounded(8);
    player_registry.register_or_replace(
        target,
        broadcast_info(target, target_tx),
        Default::default(),
    );
    let pending_invites = Arc::new(PendingInvites::default());

    session.set_player_guid(Some(inviter));
    session.set_loaded_player_identity_like_cpp(0, 1, 1, 1, 0);
    let policy = GroupInvitePolicyLikeCpp {
        minimum_level: 2,
        ..GroupInvitePolicyLikeCpp::default()
    };
    session.set_player_registry(player_registry);
    session.set_group_registry(
        Arc::new(GroupRegistry::default()),
        Arc::clone(&pending_invites),
    );

    session
        .handle_party_invite_with_policy_like_cpp(
            party_invite_packet(target, &target_name, None, 0),
            &policy,
        )
        .await;

    assert_eq!(
        party_command_result_code(&send_rx.try_recv().expect("party command result")),
        party_result::INVITE_RESTRICTED
    );
    assert!(target_rx.try_recv().is_err());
    assert!(pending_invites.get(&target).is_none());
}
#[tokio::test]
async fn party_invite_ignore_port_short_circuits_friend_lookup_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    let inviter = ObjectGuid::create_player(1, 42);
    let target = ObjectGuid::create_player(1, 77);
    let target_name = format!("Player{}", target.low_value());

    let player_registry = Arc::new(PlayerRegistry::with_canonical_player_fixtures_like_cpp());
    let (target_tx, target_rx) = bounded(8);
    player_registry.register_or_replace(
        target,
        broadcast_info(target, target_tx),
        Default::default(),
    );
    let pending_invites = Arc::new(PendingInvites::default());
    let port = PartyInviteSocialPortLikeCpp::new(
        SocialPartyInviteLookupOutcomeLikeCpp::Resolved(true),
        SocialPartyInviteLookupOutcomeLikeCpp::Resolved(true),
    );

    session.set_player_guid(Some(inviter));
    session.set_loaded_player_identity_like_cpp(0, 1, 1, 1, 0);
    session.set_party_level_req_like_cpp(2);
    session.set_player_registry(player_registry);
    session.set_group_registry(
        Arc::new(GroupRegistry::default()),
        Arc::clone(&pending_invites),
    );
    session.set_social_persistence_port_like_cpp(port.clone());

    session
        .handle_party_invite(party_invite_packet(target, &target_name, None, 0))
        .await;

    assert_eq!(
        party_command_result_code(&send_rx.try_recv().expect("party command result")),
        party_result::IGNORING_YOU
    );
    assert_eq!(port.calls(), vec!["ignore:77:42:1"]);
    assert!(target_rx.try_recv().is_err());
    assert!(pending_invites.get(&target).is_none());
}
