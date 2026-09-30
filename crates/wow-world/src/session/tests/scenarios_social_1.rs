//! Session scenarios exercising the represented social responsibility.
//!
//! Split out of session_tests.rs under #626; assertions and registrations
//! are unchanged and the shared fixtures stay in the parent module.

use super::*;

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
    session
        .instance_test_fixture_like_cpp
        .represented_raid_difficulty_id_like_cpp = 3;
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
    member_session
        .instance_test_fixture_like_cpp
        .represented_raid_difficulty_id_like_cpp = 3;
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
    session
        .instance_test_fixture_like_cpp
        .represented_raid_difficulty_id_like_cpp = 3;
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
