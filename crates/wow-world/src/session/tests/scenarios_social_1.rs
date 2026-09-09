//! Session scenarios exercising the represented social responsibility.
//!
//! Split out of session_tests.rs under #626; assertions and registrations
//! are unchanged and the shared fixtures stay in the parent module.

use super::*;

#[tokio::test]
async fn apply_group_subgroup_command_updates_current_group_reference_like_cpp() {
    let (mut session, _, _) = make_session();
    let group_guid = 0xABCDEF;
    session.group_guid = Some(group_guid);
    session.state = SessionState::LoggedIn;

    session
        .session_command_tx()
        .try_send(SessionCommand::ApplyGroupSubgroupLikeCpp(
            ApplyGroupSubgroupLikeCppCommand {
                group_guid,
                subgroup: 4,
            },
        ))
        .unwrap();
    session
        .process_represented_session_commands_like_cpp()
        .await;

    assert_eq!(session.represented_subgroup_like_cpp(), Some(4));

    session
        .session_command_tx()
        .try_send(SessionCommand::ApplyGroupSubgroupLikeCpp(
            ApplyGroupSubgroupLikeCppCommand {
                group_guid: group_guid + 1,
                subgroup: 2,
            },
        ))
        .unwrap();
    session
        .process_represented_session_commands_like_cpp()
        .await;

    assert_eq!(session.represented_subgroup_like_cpp(), Some(4));
}
#[tokio::test]
async fn apply_group_subgroup_command_ignores_non_logged_in_session_like_cpp() {
    let (mut session, _, _) = make_session();
    let group_guid = 0xABCDEF;
    session.group_guid = Some(group_guid);

    session
        .session_command_tx()
        .try_send(SessionCommand::ApplyGroupSubgroupLikeCpp(
            ApplyGroupSubgroupLikeCppCommand {
                group_guid,
                subgroup: 4,
            },
        ))
        .unwrap();
    session
        .process_represented_session_commands_like_cpp()
        .await;

    assert_eq!(session.represented_subgroup_like_cpp(), None);
}
#[test]
fn reset_group_update_sequence_does_not_reset_same_group_like_cpp() {
    let (mut session, _, _) = make_session();
    let leader = ObjectGuid::create_player(1, 42);
    let group_registry = Arc::new(GroupRegistry::default());
    let group = GroupInfo::new(leader);
    let group_guid = group.group_guid;
    group_registry.register_group_like_cpp(group_guid, group);
    session.set_group_registry(group_registry, Arc::new(PendingInvites::default()));
    assert!(session.set_owned_player_group_like_cpp(Some((group_guid, 0))));

    assert!(session.reset_group_update_sequence_if_needed_like_cpp());
    assert_eq!(
        session.next_group_update_sequence_number_like_cpp(
            wow_social::group::GROUP_CATEGORY_HOME_LIKE_CPP
        ),
        Some(1)
    );
    assert_eq!(
        session.next_group_update_sequence_number_like_cpp(
            wow_social::group::GROUP_CATEGORY_HOME_LIKE_CPP
        ),
        Some(2)
    );

    assert!(!session.reset_group_update_sequence_if_needed_like_cpp());
    assert_eq!(
        session.next_group_update_sequence_number_like_cpp(
            wow_social::group::GROUP_CATEGORY_HOME_LIKE_CPP
        ),
        Some(3)
    );
}
#[test]
fn reset_group_update_sequence_resets_when_group_changes_like_cpp() {
    let (mut session, _, _) = make_session();
    let first_leader = ObjectGuid::create_player(1, 42);
    let second_leader = ObjectGuid::create_player(1, 43);
    let group_registry = Arc::new(GroupRegistry::default());
    let first_group = GroupInfo::new(first_leader);
    let first_group_guid = first_group.group_guid;
    let second_group = GroupInfo::new(second_leader);
    let second_group_guid = second_group.group_guid;
    group_registry.register_group_like_cpp(first_group_guid, first_group);
    group_registry.register_group_like_cpp(second_group_guid, second_group);
    session.set_group_registry(group_registry, Arc::new(PendingInvites::default()));

    session.group_guid = Some(first_group_guid);
    assert!(session.reset_group_update_sequence_if_needed_like_cpp());
    assert_eq!(
        session.next_group_update_sequence_number_like_cpp(
            wow_social::group::GROUP_CATEGORY_HOME_LIKE_CPP
        ),
        Some(1)
    );
    assert_eq!(
        session.next_group_update_sequence_number_like_cpp(
            wow_social::group::GROUP_CATEGORY_HOME_LIKE_CPP
        ),
        Some(2)
    );

    session.group_guid = Some(second_group_guid);
    assert!(session.reset_group_update_sequence_if_needed_like_cpp());
    assert_eq!(
        session.next_group_update_sequence_number_like_cpp(
            wow_social::group::GROUP_CATEGORY_HOME_LIKE_CPP
        ),
        Some(1)
    );
}
#[test]
fn reset_group_update_sequence_without_group_is_noop_like_cpp() {
    let (mut session, _, _) = make_session();

    assert!(!session.reset_group_update_sequence_if_needed_like_cpp());
    assert_eq!(
        session.next_group_update_sequence_number_like_cpp(
            wow_social::group::GROUP_CATEGORY_HOME_LIKE_CPP
        ),
        Some(0)
    );
    assert_eq!(session.next_group_update_sequence_number_like_cpp(99), None);
}
#[tokio::test]
async fn party_update_command_consumes_receiver_sequence_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let group_registry = Arc::new(GroupRegistry::default());
    let group = GroupInfo::new(player_guid);
    let group_guid = group.group_guid;
    group_registry.register_group_like_cpp(group_guid, group);
    session.set_group_registry(group_registry, Arc::new(PendingInvites::default()));
    assert!(session.set_owned_player_group_like_cpp(Some((group_guid, 0))));
    session.set_player_guid(Some(player_guid));
    session.state = SessionState::LoggedIn;

    assert!(session.reset_group_update_sequence_if_needed_like_cpp());

    for _ in 0..2 {
        session
            .session_command_tx()
            .try_send(SessionCommand::SendPartyUpdateLikeCpp(
                SendPartyUpdateLikeCppCommand {
                    recipient: player_guid,
                    party_update: wow_packet::packets::party::PartyUpdate {
                        party_flags: 0,
                        party_index: wow_social::group::GROUP_CATEGORY_HOME_LIKE_CPP,
                        party_type: wow_social::group::GROUP_TYPE_NORMAL_LIKE_CPP,
                        my_index: 0,
                        party_guid: group_guid,
                        // C++ ignores any caller/global sequence and asks
                        // the receiver Player for the next number.
                        sequence_num: 999,
                        leader_guid: player_guid,
                        leader_faction_group: 0,
                        player_list: Vec::new(),
                        loot_settings: None,
                        difficulty_settings: None,
                    },
                    member_full_state_packets: Vec::new(),
                },
            ))
            .unwrap();
        session
            .process_represented_session_commands_like_cpp()
            .await;
    }

    let first = send_rx.try_recv().unwrap();
    let second = send_rx.try_recv().unwrap();
    assert_eq!(party_update_sequence_num_like_cpp(&first), 1);
    assert_eq!(party_update_sequence_num_like_cpp(&second), 2);
}
#[tokio::test]
async fn group_removal_command_clears_remote_party_type_like_cpp() {
    let (mut session, _, instance_rx) = make_session();
    let (realm_tx, realm_rx) = flume::bounded(8);
    session.install_realm_send_channel_for_test(realm_tx);
    let player_guid = ObjectGuid::create_player(1, 42);
    let group_guid = 0xABCDEF;
    let player_registry = Arc::new(crate::session::directory::PlayerRegistry::default());
    let canonical = shared_canonical_map_manager();
    assert!(player_registry.bind_canonical_map_manager(Arc::clone(&canonical)));
    session.set_canonical_map_manager(Arc::clone(&canonical));
    let (registry_send_tx, _registry_send_rx) = flume::bounded(8);
    let group_registry = Arc::new(GroupRegistry::default());
    let mut group = GroupInfo::new(player_guid);
    group.group_guid = group_guid;
    group_registry.register_group_like_cpp(group_guid, group);

    session.set_player_guid(Some(player_guid));
    session.group_guid = Some(group_guid);
    session.state = SessionState::LoggedIn;
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "Tester".to_string(),
        Position::ZERO,
        571,
        1,
        1,
        80,
        0,
    ));
    add_canonical_test_player_on_map(&canonical, player_guid, Position::ZERO, 571, 0);
    session.set_player_registry(Arc::clone(&player_registry));
    session.set_group_registry(
        Arc::clone(&group_registry),
        Arc::new(PendingInvites::default()),
    );
    let mut registration =
        broadcast_info_with_command(player_guid, registry_send_tx, session.session_command_tx());
    registration.placement.map_id = 571;
    player_registry.register_or_replace(player_guid, registration, Default::default());
    session.sync_player_registry_state_like_cpp();

    let before = canonical_party_type_for_test(&canonical, player_guid);
    assert_eq!(
        before[usize::from(wow_social::group::GROUP_CATEGORY_HOME_LIKE_CPP)],
        wow_social::group::GROUP_TYPE_NORMAL_LIKE_CPP
    );

    session
        .session_command_tx()
        .try_send(SessionCommand::ApplyGroupRemovalLikeCpp(
            ApplyGroupRemovalLikeCppCommand {
                group_guid,
                category: wow_social::group::GROUP_CATEGORY_HOME_LIKE_CPP,
                party_type: wow_social::group::GROUP_TYPE_NONE_LIKE_CPP,
                send_group_destroyed: true,
                send_group_uninvite: false,
                refresh_visible_gameobjects_or_spellclicks: false,
            },
        ))
        .unwrap();
    group_registry.unregister_group_like_cpp(&group_guid);
    session
        .process_represented_session_commands_like_cpp()
        .await;

    assert_eq!(session.group_guid, None);
    let after = canonical_party_type_for_test(&canonical, player_guid);
    assert_eq!(
        after[usize::from(wow_social::group::GROUP_CATEGORY_HOME_LIKE_CPP)],
        wow_social::group::GROUP_TYPE_NONE_LIKE_CPP
    );

    let packets = drain_server_packet_bytes(&instance_rx);
    assert!(packets.iter().any(|bytes| {
        wow_packet::WorldPacket::from_bytes(bytes).server_opcode()
            == Some(ServerOpcodes::UpdateObject)
    }));
    assert!(!packets.iter().any(|bytes| {
        wow_packet::WorldPacket::from_bytes(bytes).server_opcode()
            == Some(ServerOpcodes::GroupDestroyed)
    }));
    let destroyed = realm_rx.try_recv().expect("realm GroupDestroyed");
    assert_eq!(
        wow_packet::WorldPacket::from_bytes(&destroyed).server_opcode(),
        Some(ServerOpcodes::GroupDestroyed)
    );
    // C++ `Group::Disband` sends the destroyed `PartyUpdate` after `GroupDestroyed`.
    let destroyed_update = realm_rx.try_recv().expect("realm destroyed PartyUpdate");
    assert_destroyed_party_update_like_cpp(&destroyed_update, group_guid);
    assert!(realm_rx.try_recv().is_err());
}
#[tokio::test]
async fn group_removal_command_can_send_group_uninvite_like_cpp() {
    let (mut session, _, instance_rx) = make_session();
    let (realm_tx, realm_rx) = flume::bounded(8);
    session.install_realm_send_channel_for_test(realm_tx);
    let player_guid = ObjectGuid::create_player(1, 42);
    let group_guid = 0xBCDEF0;
    let player_registry = Arc::new(crate::session::directory::PlayerRegistry::default());
    let (registry_send_tx, _registry_send_rx) = flume::bounded(8);
    let group_registry = Arc::new(GroupRegistry::default());
    let mut group = GroupInfo::new(player_guid);
    group.group_guid = group_guid;
    group_registry.register_group_like_cpp(group_guid, group);

    session.set_player_guid(Some(player_guid));
    session.group_guid = Some(group_guid);
    session.state = SessionState::LoggedIn;
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "Tester".to_string(),
        Position::ZERO,
        571,
        1,
        1,
        80,
        0,
    ));
    session.set_player_registry(Arc::clone(&player_registry));
    session.set_group_registry(
        Arc::clone(&group_registry),
        Arc::new(PendingInvites::default()),
    );
    player_registry.register_or_replace(
        player_guid,
        broadcast_info(player_guid, registry_send_tx),
        Default::default(),
    );
    group_registry.unregister_group_like_cpp(&group_guid);

    session
        .session_command_tx()
        .try_send(SessionCommand::ApplyGroupRemovalLikeCpp(
            ApplyGroupRemovalLikeCppCommand {
                group_guid,
                category: wow_social::group::GROUP_CATEGORY_HOME_LIKE_CPP,
                party_type: wow_social::group::GROUP_TYPE_NONE_LIKE_CPP,
                send_group_destroyed: false,
                send_group_uninvite: true,
                refresh_visible_gameobjects_or_spellclicks: false,
            },
        ))
        .unwrap();
    session
        .process_represented_session_commands_like_cpp()
        .await;

    let packets = drain_server_packet_bytes(&instance_rx);
    assert!(packets.iter().any(|bytes| {
        wow_packet::WorldPacket::from_bytes(bytes).server_opcode()
            == Some(ServerOpcodes::UpdateObject)
    }));
    assert!(!packets.iter().any(|bytes| {
        wow_packet::WorldPacket::from_bytes(bytes).server_opcode()
            == Some(ServerOpcodes::GroupUninvite)
    }));
    assert!(!packets.iter().any(|bytes| {
        wow_packet::WorldPacket::from_bytes(bytes).server_opcode()
            == Some(ServerOpcodes::GroupDestroyed)
    }));
    let uninvite = realm_rx.try_recv().expect("realm GroupUninvite");
    assert_eq!(
        wow_packet::WorldPacket::from_bytes(&uninvite).server_opcode(),
        Some(ServerOpcodes::GroupUninvite)
    );
    // C++ `Group::RemoveMember` sends the kicked member the destroyed
    // `PartyUpdate` when the group survives (`Group.cpp:654-655`).
    let destroyed_update = realm_rx.try_recv().expect("realm destroyed PartyUpdate");
    assert_destroyed_party_update_like_cpp(&destroyed_update, group_guid);
    assert!(realm_rx.try_recv().is_err());
}
#[test]
fn represented_group_leader_flag_is_removed_for_non_leader_like_cpp() {
    let (mut session, _, player_guid) = session_with_canonical_player_for_away_like_cpp();
    session
        .mutate_canonical_player_like_cpp(|player| {
            player.set_player_flag(PLAYER_FLAGS_GROUP_LEADER_LIKE_CPP);
        })
        .unwrap();
    let leader_guid = ObjectGuid::create_player(1, 99);
    let group_registry = Arc::new(GroupRegistry::default());
    let mut group = GroupInfo::new(leader_guid);
    group.add_member(player_guid);
    let group_guid = group.group_guid;
    group_registry.register_group_like_cpp(group_guid, group);
    session.group_guid = Some(group_guid);
    session.set_group_registry(group_registry, Arc::new(PendingInvites::default()));

    assert!(session.apply_represented_group_leader_flag_like_cpp());

    assert_eq!(
        session.canonical_player_has_player_flag_like_cpp(
            player_guid,
            PLAYER_FLAGS_GROUP_LEADER_LIKE_CPP
        ),
        Some(false)
    );
}
#[test]
fn canonical_player_guild_state_follows_active_detached_and_stale_ownership_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 5_564);

    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_map_store(canonical_player_transfer_test_map_store_like_cpp());
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "GuildOwner".to_string(),
        Position::new(3700.0, 1500.0, 120.0, 0.0),
        571,
        1,
        1,
        80,
        0,
    ));
    session
        .ensure_canonical_world_map_for_current_player_like_cpp()
        .expect("initial world map");
    let old_handle = session.player_handle_like_cpp.expect("canonical handle");

    assert!(session.set_represented_guild_id_like_cpp(12));
    assert!(session.set_represented_guild_id_invited_like_cpp(13));
    assert_eq!(session.resolved_represented_guild_id_like_cpp(), Some(12));
    assert!(session.remove_current_player_from_canonical_current_map_like_cpp());
    assert_eq!(
        canonical
            .lock()
            .unwrap()
            .player_residence_like_cpp(old_handle),
        Some(wow_map::PlayerResidenceLikeCpp::Detached)
    );
    assert_eq!(session.resolved_represented_guild_id_like_cpp(), Some(12));

    let replacement_state = wow_entities::PlayerGuildState {
        guild_id: Some(99),
        invited_guild_id: Some(100),
        rank_id: Some(4),
        authority_complete: true,
    };
    let mut replacement = Box::new(Player::new(Some(2), false));
    replacement
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(player_guid);
    replacement.gameplay_state_mut().guild = replacement_state.clone();
    let replacement_handle = canonical
        .lock()
        .unwrap()
        .install_detached_player_like_cpp(replacement)
        .expect("replacement owner");

    assert_eq!(session.resolved_represented_guild_id_like_cpp(), None);
    assert!(!session.set_represented_guild_id_like_cpp(14));
    assert!(!session.set_represented_guild_id_invited_like_cpp(15));
    assert_eq!(
        canonical
            .lock()
            .unwrap()
            .with_player_like_cpp(replacement_handle, |player| {
                player.gameplay_state().guild.clone()
            }),
        Some(replacement_state)
    );
}
#[test]
fn canonical_player_trade_state_follows_active_detached_and_stale_ownership_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 5_565);
    let partner_guid = ObjectGuid::create_player(1, 5_566);
    let item_guid = ObjectGuid::create_item(1, 70_001);

    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_map_store(canonical_player_transfer_test_map_store_like_cpp());
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "TradeOwner".to_string(),
        Position::new(3700.0, 1500.0, 120.0, 0.0),
        571,
        1,
        1,
        80,
        0,
    ));
    session
        .ensure_canonical_world_map_for_current_player_like_cpp()
        .expect("initial world map");
    let old_handle = session.player_handle_like_cpp.expect("canonical handle");

    assert!(session.set_represented_active_trade_partner_like_cpp(Some(partner_guid)));
    assert!(session.set_represented_partner_trade_server_state_index_like_cpp(7));
    session.set_represented_trade_item_like_cpp_for_test(2, item_guid);
    session.set_represented_trade_spell_like_cpp_for_test(7418, Some(item_guid));
    session.set_represented_trade_accepted_like_cpp_for_test(true);
    assert_eq!(
        session.resolved_represented_active_trade_partner_like_cpp(),
        Some(Some(partner_guid))
    );
    assert!(session.remove_current_player_from_canonical_current_map_like_cpp());
    assert_eq!(
        canonical
            .lock()
            .unwrap()
            .player_residence_like_cpp(old_handle),
        Some(wow_map::PlayerResidenceLikeCpp::Detached)
    );
    assert_eq!(session.represented_trade_item_like_cpp(2), Some(item_guid));
    assert_eq!(session.represented_trade_spell_like_cpp(), 7418);
    assert!(session.represented_trade_accepted_like_cpp());

    let replacement_partner = ObjectGuid::create_player(1, 5_567);
    let replacement_state = wow_entities::PlayerTradeStateLikeCpp {
        partner_guid: replacement_partner,
        accepted: true,
        partner_server_state_index: 12,
        client_state_index: 13,
        server_state_index: 14,
        items: [None; wow_entities::PLAYER_TRADE_SLOT_COUNT_LIKE_CPP],
        money: 15,
        spell_id: 16,
        spell_cast_item_guid: None,
    };
    let mut replacement = Box::new(Player::new(Some(2), false));
    replacement
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(player_guid);
    replacement.gameplay_state_mut().trade = Some(replacement_state.clone());
    let replacement_handle = canonical
        .lock()
        .unwrap()
        .install_detached_player_like_cpp(replacement)
        .expect("replacement owner");

    assert_eq!(
        session.resolved_represented_active_trade_partner_like_cpp(),
        None
    );
    assert!(
        !session.set_represented_active_trade_partner_like_cpp(Some(ObjectGuid::create_player(
            1, 5_568
        )))
    );
    assert!(!session.set_represented_trade_accepted_like_cpp_for_command(false));
    assert_eq!(
        canonical
            .lock()
            .unwrap()
            .with_player_like_cpp(replacement_handle, |player| {
                player.gameplay_state().trade.clone()
            }),
        Some(Some(replacement_state))
    );
}
#[test]
fn canonical_player_group_reference_follows_detached_and_stale_ownership_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 5_571);
    let groups = Arc::new(GroupRegistry::default());
    let group = GroupInfo::new(player_guid);
    let group_guid = group.group_guid;
    groups.register_group_like_cpp(group_guid, group);

    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_map_store(canonical_player_transfer_test_map_store_like_cpp());
    session.set_group_registry(Arc::clone(&groups), Arc::new(PendingInvites::default()));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "GroupOwner".to_string(),
        Position::new(3700.0, 1500.0, 120.0, 0.0),
        571,
        1,
        1,
        80,
        0,
    ));
    session
        .ensure_canonical_world_map_for_current_player_like_cpp()
        .expect("initial world map");
    let old_handle = session.player_handle_like_cpp.expect("canonical handle");

    assert!(session.set_owned_player_group_like_cpp(Some((group_guid, 0))));
    assert!(session.reset_group_update_sequence_if_needed_like_cpp());
    assert_eq!(session.resolved_group_guid_like_cpp(), Some(group_guid));
    assert_eq!(
        session.next_group_update_sequence_number_like_cpp(0),
        Some(1)
    );
    assert!(session.remove_current_player_from_canonical_current_map_like_cpp());
    assert_eq!(
        canonical
            .lock()
            .unwrap()
            .player_residence_like_cpp(old_handle),
        Some(wow_map::PlayerResidenceLikeCpp::Detached)
    );
    assert_eq!(session.resolved_group_guid_like_cpp(), Some(group_guid));
    assert_eq!(
        session.next_group_update_sequence_number_like_cpp(0),
        Some(2)
    );

    let replacement_state = wow_entities::PlayerGroupState {
        group_guid: ObjectGuid::create_group(group_guid + 1),
        leader_guid: ObjectGuid::create_player(1, 9_999),
        role_mask: 4,
        subgroup: 3,
    };
    let mut replacement = Box::new(Player::new(Some(2), false));
    replacement
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(player_guid);
    replacement.gameplay_state_mut().group = Some(replacement_state.clone());
    let replacement_handle = canonical
        .lock()
        .unwrap()
        .install_detached_player_like_cpp(replacement)
        .expect("replacement owner");

    assert_eq!(session.resolved_group_guid_like_cpp(), None);
    assert!(!session.set_owned_player_group_like_cpp(None));
    assert_eq!(session.next_group_update_sequence_number_like_cpp(0), None);
    assert_eq!(
        canonical
            .lock()
            .unwrap()
            .with_player_like_cpp(replacement_handle, |player| {
                player.gameplay_state().group.clone()
            }),
        Some(Some(replacement_state))
    );
}
#[test]
fn canonical_access_requirement_min_level_rejects_before_raid_group_like_cpp() {
    let (mut session, _pkt_tx, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 83);

    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "AccessLevel".to_string(),
        Position::new(3700.0, 1500.0, 120.0, 0.0),
        631,
        1,
        1,
        79,
        0,
    ));
    session.represented_raid_difficulty_id_like_cpp = 3;
    install_create_map_active_lock_stores_with_expansion_and_max_players_like_cpp(
        &mut session,
        631,
        3,
        77,
        2,
        2,
        25,
    );
    let mut requirement = access_requirement_like_cpp(631, 3);
    requirement.level_min = 80;
    install_access_notification_stores_like_cpp(&mut session);
    install_access_requirement_store_like_cpp(&mut session, requirement);

    assert_eq!(
        session.ensure_canonical_world_map_for_current_player_like_cpp(),
        Some(wow_map::CreateMapDecision::Reject {
            side_effects: Vec::new()
        })
    );
    assert_eq!(
        send_rx.try_recv().expect("SMSG_PRINT_NOTIFICATION"),
        PrintNotification {
            notify_text: "You must be at least level 80 to enter.".to_string(),
        }
        .to_bytes()
    );
    assert_eq!(
        send_rx.try_recv().expect("SMSG_TRANSFER_ABORTED"),
        wow_packet::packets::misc::TransferAborted {
            map_id: 631,
            arg: 0,
            map_difficulty_x_condition_id: 0,
            transfer_abort: TRANSFER_ABORT_ERROR_LIKE_CPP,
        }
        .to_bytes()
    );
    assert!(send_rx.try_recv().is_err());
}
#[test]
fn canonical_access_requirement_connected_group_leader_achievement_matches_cpp() {
    let (mut leader_session, _leader_pkt_tx, _leader_send_rx) = make_session();
    let (mut member_session, _member_pkt_tx, member_send_rx) = make_session();
    let player_registry = Arc::new(PlayerRegistry::with_canonical_player_fixtures_like_cpp());
    let canonical = player_registry
        .fixture_canonical_map_manager_like_cpp()
        .expect("canonical player fixture manager");
    let group_registry = Arc::new(GroupRegistry::default());
    let leader_guid = ObjectGuid::create_player(1, 89);
    let member_guid = ObjectGuid::create_player(1, 90);

    leader_session.set_player_registry(Arc::clone(&player_registry));
    leader_session.attach_player_controller_like_cpp(SessionPlayerController::new(
        leader_guid,
        "AccessLeader".to_string(),
        Position::new(3700.0, 1500.0, 120.0, 0.0),
        631,
        1,
        1,
        80,
        0,
    ));
    leader_session.register_in_player_registry();

    let mut group = GroupInfo::new(leader_guid);
    group.raid_difficulty_id = 3;
    group.add_member(member_guid);
    let group_guid = group.group_guid;
    group_registry.register_group_like_cpp(group_guid, group);

    member_session.set_canonical_map_manager(Arc::clone(&canonical));
    member_session.set_player_registry(Arc::clone(&player_registry));
    member_session.set_group_registry(
        Arc::clone(&group_registry),
        Arc::new(PendingInvites::default()),
    );
    member_session.attach_player_controller_like_cpp(SessionPlayerController::new(
        member_guid,
        "AccessMember".to_string(),
        Position::new(3700.0, 1500.0, 120.0, 0.0),
        631,
        1,
        1,
        80,
        0,
    ));
    member_session.group_guid = Some(group_guid);
    member_session.represented_raid_difficulty_id_like_cpp = 3;
    install_create_map_active_lock_stores_like_cpp(&mut member_session, 631, 3, 77, 2);
    let mut requirement = access_requirement_like_cpp(631, 3);
    requirement.completed_achievement = 9001;
    install_access_requirement_store_like_cpp(&mut member_session, requirement);

    assert_eq!(
        member_session.ensure_canonical_world_map_for_current_player_like_cpp(),
        Some(wow_map::CreateMapDecision::Reject {
            side_effects: Vec::new()
        })
    );
    assert_eq!(
        member_send_rx
            .try_recv()
            .expect("missing remote leader achievement abort"),
        wow_packet::packets::misc::TransferAborted {
            map_id: 631,
            arg: 0,
            map_difficulty_x_condition_id: 0,
            transfer_abort: TRANSFER_ABORT_ERROR_LIKE_CPP,
        }
        .to_bytes()
    );
    assert!(member_send_rx.try_recv().is_err());

    member_session
        .mutate_canonical_player_by_guid_like_cpp(leader_guid, |leader| {
            leader
                .gameplay_state_mut()
                .achievements
                .push(wow_entities::PlayerAchievementRecord {
                    achievement_id: 9001,
                    completed_at: None,
                });
        })
        .expect("canonical group leader");
    assert!(matches!(
        member_session.ensure_canonical_world_map_for_current_player_like_cpp(),
        Some(wow_map::CreateMapDecision::Create { .. })
    ));
    assert!(member_send_rx.try_recv().is_err());
}
#[test]
fn canonical_current_expansion_raid_requires_raid_group_like_cpp() {
    let (mut session, _pkt_tx, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 78);

    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "RaidNoGroup".to_string(),
        Position::new(3700.0, 1500.0, 120.0, 0.0),
        631,
        1,
        1,
        80,
        0,
    ));
    session.represented_raid_difficulty_id_like_cpp = 3;
    install_create_map_active_lock_stores_with_expansion_and_max_players_like_cpp(
        &mut session,
        631,
        3,
        77,
        2,
        2,
        25,
    );

    assert_eq!(
        session.ensure_canonical_world_map_for_current_player_like_cpp(),
        Some(wow_map::CreateMapDecision::Reject {
            side_effects: Vec::new()
        })
    );
    assert_eq!(
        send_rx.try_recv().expect("SMSG_TRANSFER_ABORTED"),
        wow_packet::packets::misc::TransferAborted {
            map_id: 631,
            arg: 0,
            map_difficulty_x_condition_id: 0,
            transfer_abort: TRANSFER_ABORT_NEED_GROUP_LIKE_CPP,
        }
        .to_bytes()
    );
    assert!(
        canonical.lock().unwrap().find_map(631, 0).is_none(),
        "C++ rejects current-expansion raid entry before resolving/creating an instance"
    );
}
