#![cfg(test)]
//! Instance admission uses real canonical Players, never a manual count.
//! C++ Map::GetPlayersCountExceptGMs (Map.cpp:2648) excludes GM occupants;
//! total MapManager instance population (MapManager.cpp:367) includes them.
use super::*;

fn install_occupant(manager: &mut wow_map::MapManager, guid: ObjectGuid, gm: bool) {
    manager.create_map_entry(
        631,
        9001,
        3,
        wow_map::ManagedMapKind::Dungeon {
            has_reset_schedule: false,
        },
    );
    let mut player = Box::new(Player::new(Some(1), false));
    player.unit_mut().world_mut().object_mut().create(guid);
    player.set_game_master_like_cpp(gm);
    let handle = manager.install_detached_player_like_cpp(player).unwrap();
    manager
        .attach_player_like_cpp(
            handle,
            wow_map::MapKey::new(631, 9001),
            Position::new(3700.0, 1500.0, 120.0, 0.0),
        )
        .unwrap();
}

#[test]
fn canonical_player_existing_instance_map_full_sends_transfer_abort_like_cpp() {
    let (mut session, _pkt_tx, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let leader = ObjectGuid::create_player(1, 70);
    let member = ObjectGuid::create_player(1, 71);

    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        member,
        "DungeonFullReject".to_string(),
        Position::new(3700.0, 1500.0, 120.0, 0.0),
        631,
        1,
        1,
        80,
        0,
    ));
    session.represented_raid_difficulty_id_like_cpp = 3;
    install_create_map_active_lock_stores_with_max_players_like_cpp(&mut session, 631, 3, 77, 0, 1);

    let group_registry = Arc::new(GroupRegistry::default());
    let mut group = GroupInfo::new(leader);
    group.raid_difficulty_id = 3;
    group.add_member(member);
    group.set_recent_instance_like_cpp(631, leader, 9001);
    let group_guid = group.group_guid;
    group_registry.register_group_like_cpp(group_guid, group);
    session.group_guid = Some(group_guid);
    session.set_group_registry(group_registry, Arc::new(PendingInvites::default()));

    install_occupant(&mut canonical.lock().unwrap(), leader, false);

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
            transfer_abort: TRANSFER_ABORT_MAX_PLAYERS_LIKE_CPP,
        }
        .to_bytes()
    );
    assert!(
        canonical
            .lock()
            .unwrap()
            .find_map(631, 9001)
            .unwrap()
            .map()
            .get_typed_player(member)
            .is_none(),
        "full instance rejection must not synchronize the player"
    );
}

#[test]
fn canonical_existing_instance_full_gate_does_not_count_game_masters_like_cpp() {
    let (mut session, _pkt_tx, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let leader = ObjectGuid::create_player(1, 94);
    let member = ObjectGuid::create_player(1, 95);
    let existing_gm = ObjectGuid::create_player(1, 96);

    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        member,
        "DungeonGmOccupant".to_string(),
        Position::new(3700.0, 1500.0, 120.0, 0.0),
        631,
        1,
        1,
        80,
        0,
    ));
    session.represented_raid_difficulty_id_like_cpp = 3;
    install_create_map_active_lock_stores_with_max_players_like_cpp(&mut session, 631, 3, 77, 0, 1);

    let group_registry = Arc::new(GroupRegistry::default());
    let mut group = GroupInfo::new(leader);
    group.raid_difficulty_id = 3;
    group.add_member(member);
    group.set_recent_instance_like_cpp(631, leader, 9001);
    let group_guid = group.group_guid;
    group_registry.register_group_like_cpp(group_guid, group);
    session.group_guid = Some(group_guid);
    session.set_group_registry(group_registry, Arc::new(PendingInvites::default()));

    install_occupant(&mut canonical.lock().unwrap(), existing_gm, true);

    assert_eq!(
        session.ensure_canonical_world_map_for_current_player_like_cpp(),
        Some(wow_map::CreateMapDecision::Existing {
            key: wow_map::MapKey::new(631, 9001),
            difficulty_id: 3,
            side_effects: Vec::new(),
        })
    );
    assert!(send_rx.try_recv().is_err());
    assert!(
        canonical
            .lock()
            .unwrap()
            .find_map(631, 9001)
            .unwrap()
            .map()
            .get_typed_player(member)
            .is_some(),
        "GM occupants must not make a C++ instance full"
    );
}

#[test]
fn canonical_game_master_bypasses_existing_instance_full_gate_like_cpp() {
    let (mut session, _pkt_tx, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let leader = ObjectGuid::create_player(1, 72);
    let gm = ObjectGuid::create_player(1, 73);

    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        gm,
        "DungeonFullGm".to_string(),
        Position::new(3700.0, 1500.0, 120.0, 0.0),
        631,
        1,
        1,
        80,
        0,
    ));
    session.set_player_game_master_like_cpp(true);
    session.represented_raid_difficulty_id_like_cpp = 3;
    install_create_map_active_lock_stores_with_max_players_like_cpp(&mut session, 631, 3, 77, 0, 1);

    let group_registry = Arc::new(GroupRegistry::default());
    let mut group = GroupInfo::new(leader);
    group.raid_difficulty_id = 3;
    group.add_member(gm);
    group.set_recent_instance_like_cpp(631, leader, 9001);
    let group_guid = group.group_guid;
    group_registry.register_group_like_cpp(group_guid, group);
    session.group_guid = Some(group_guid);
    session.set_group_registry(group_registry, Arc::new(PendingInvites::default()));

    install_occupant(&mut canonical.lock().unwrap(), leader, false);

    assert_eq!(
        session.ensure_canonical_world_map_for_current_player_like_cpp(),
        Some(wow_map::CreateMapDecision::Existing {
            key: wow_map::MapKey::new(631, 9001),
            difficulty_id: 3,
            side_effects: Vec::new(),
        })
    );
    assert!(send_rx.try_recv().is_err());
    assert!(
        canonical
            .lock()
            .unwrap()
            .find_map(631, 9001)
            .unwrap()
            .map()
            .get_typed_player(gm)
            .is_some(),
        "GM bypass should still synchronize the represented player into the map"
    );
}
