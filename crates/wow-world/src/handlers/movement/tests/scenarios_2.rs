//! Movement handlers regression scenarios, part 2 of 3.
//!
//! Moved out of the movement.rs root under #654; every test is unchanged.

use super::*;

#[test]
fn validate_movement_info_strips_each_cpp_incompatible_pair() {
    let session = make_session();
    for (left, right) in [
        (MovementFlag::ASCENDING, MovementFlag::DESCENDING),
        (MovementFlag::LEFT, MovementFlag::RIGHT),
        (MovementFlag::STRAFE_LEFT, MovementFlag::STRAFE_RIGHT),
        (MovementFlag::PITCH_UP, MovementFlag::PITCH_DOWN),
        (MovementFlag::FORWARD, MovementFlag::BACKWARD),
    ] {
        let mut info = MovementInfo {
            flags: left | right,
            ..MovementInfo::default()
        };

        let removed = session.sanitize_movement_info_flags_represented_like_cpp(&mut info);

        assert!(removed.contains(left | right), "{left:?} | {right:?}");
        assert!(info.flags.is_empty(), "{left:?} | {right:?}");
    }
}

#[test]
fn validate_movement_info_reports_rule_evidence_from_anticheat_core() {
    let session = make_session();
    let mut info = MovementInfo {
        flags: MovementFlag::HOVER | MovementFlag::WATER_WALK,
        ..MovementInfo::default()
    };

    let result = session.sanitize_movement_info_represented_like_cpp(&mut info);

    assert_eq!(info.flags, MovementFlag::empty());
    assert!(result.removed_flags.contains(MovementFlag::HOVER));
    assert!(result.removed_flags.contains(MovementFlag::WATER_WALK));
    assert_eq!(
        result.stripped_rules,
        vec![
            wow_anticheat::MovementSanitizerRule::HoverWithoutAura,
            wow_anticheat::MovementSanitizerRule::WaterWalkWithoutAuraOrGhost,
        ]
    );
}

#[test]
fn validate_movement_info_root_order_matches_cpp_without_fixed_vehicle() {
    let session = make_session();
    let mut info = MovementInfo {
        flags: MovementFlag::ROOT | MovementFlag::FORWARD,
        ..MovementInfo::default()
    };

    let removed = session.sanitize_movement_info_flags_represented_like_cpp(&mut info);

    assert!(removed.contains(MovementFlag::ROOT));
    assert!(!removed.contains(MovementFlag::FORWARD));
    assert_eq!(info.flags, MovementFlag::FORWARD);
}

#[test]
fn validate_movement_info_keeps_root_for_fixed_position_vehicle_like_cpp() {
    let mut session = make_session();
    session.set_represented_mover_fixed_position_vehicle_like_cpp(true);

    let mut rooted = MovementInfo {
        flags: MovementFlag::ROOT,
        ..MovementInfo::default()
    };
    let removed = session.sanitize_movement_info_flags_represented_like_cpp(&mut rooted);
    assert!(removed.is_empty());
    assert_eq!(rooted.flags, MovementFlag::ROOT);

    let mut rooted_moving = MovementInfo {
        flags: MovementFlag::ROOT | MovementFlag::FORWARD,
        ..MovementInfo::default()
    };
    let removed = session.sanitize_movement_info_flags_represented_like_cpp(&mut rooted_moving);
    assert!(removed.contains(MovementFlag::FORWARD));
    assert!(!removed.contains(MovementFlag::ROOT));
    assert_eq!(rooted_moving.flags, MovementFlag::ROOT);
}

#[test]
fn validate_movement_info_keeps_represented_allowed_aura_flags() {
    let mut session = make_session();
    session
        .visible_auras
        .insert(1, fall_aura(1, RepresentedAuraEffectLikeCpp::Hover, 0, 1.0));
    session.visible_auras.insert(
        2,
        fall_aura(2, RepresentedAuraEffectLikeCpp::FeatherFall, 0, 1.0),
    );
    session
        .visible_auras
        .insert(3, fall_aura(3, RepresentedAuraEffectLikeCpp::Fly, 0, 1.0));
    session.visible_auras.insert(
        4,
        fall_aura(4, RepresentedAuraEffectLikeCpp::WaterWalk, 0, 1.0),
    );
    let mut info = MovementInfo {
        flags: MovementFlag::HOVER
            | MovementFlag::WATER_WALK
            | MovementFlag::FALLING_SLOW
            | MovementFlag::FLYING
            | MovementFlag::CAN_FLY,
        step_up_start_elevation: 1.0,
        ..MovementInfo::default()
    };

    let removed = session.sanitize_movement_info_flags_represented_like_cpp(&mut info);
    assert!(removed.is_empty());
    assert!(info.flags.contains(MovementFlag::HOVER));
    assert!(info.flags.contains(MovementFlag::WATER_WALK));
    assert!(info.flags.contains(MovementFlag::FALLING_SLOW));
    assert!(
        info.flags
            .contains(MovementFlag::FLYING | MovementFlag::CAN_FLY)
    );
    assert!(info.flags.contains(MovementFlag::SPLINE_ELEVATION));
}

#[test]
fn validate_movement_info_keeps_water_walk_for_ghost_like_cpp() {
    let mut session = make_session();
    session
        .visible_auras
        .insert(1, fall_aura(1, RepresentedAuraEffectLikeCpp::Ghost, 0, 1.0));
    let mut info = MovementInfo {
        flags: MovementFlag::WATER_WALK,
        ..MovementInfo::default()
    };

    let removed = session.sanitize_movement_info_flags_represented_like_cpp(&mut info);

    assert!(removed.is_empty());
    assert!(info.flags.contains(MovementFlag::WATER_WALK));
}

#[test]
fn validate_movement_info_keeps_fly_for_gm_like_cpp() {
    let mut session = make_session();
    session.set_player_game_master_like_cpp(true);
    let mut info = MovementInfo {
        flags: MovementFlag::FLYING | MovementFlag::CAN_FLY,
        ..MovementInfo::default()
    };

    let removed = session.sanitize_movement_info_flags_represented_like_cpp(&mut info);

    assert!(removed.is_empty());
    assert!(
        info.flags
            .contains(MovementFlag::FLYING | MovementFlag::CAN_FLY)
    );
}

#[test]
fn validate_movement_info_keeps_fly_for_mounted_flight_speed_aura_like_cpp() {
    let mut session = make_session();
    session.visible_auras.insert(
        1,
        fall_aura(1, RepresentedAuraEffectLikeCpp::MountedFlightSpeed, 0, 1.0),
    );
    let mut info = MovementInfo {
        flags: MovementFlag::FLYING | MovementFlag::CAN_FLY,
        ..MovementInfo::default()
    };

    let removed = session.sanitize_movement_info_flags_represented_like_cpp(&mut info);

    assert!(removed.is_empty());
    assert!(
        info.flags
            .contains(MovementFlag::FLYING | MovementFlag::CAN_FLY)
    );
}

#[tokio::test]
async fn handle_movement_broadcasts_sanitized_flags_like_cpp() {
    let mut session = make_session();
    let guid = ObjectGuid::create_player(1, 42);
    let other_guid = ObjectGuid::create_player(1, 43);
    let registry = std::sync::Arc::new(crate::session::directory::PlayerRegistry::default());
    let (self_tx, self_rx) = flume::bounded(1);
    let (other_tx, other_rx) = flume::bounded(1);
    let (self_command_tx, self_command_rx) = flume::bounded(1);
    let (other_command_tx, other_command_rx) = flume::bounded(1);

    session.set_player_guid(Some(guid));
    session.set_player_registry(std::sync::Arc::clone(&registry));
    registry.register_or_replace(
        guid,
        broadcast_info_with_command(guid, self_tx, self_command_tx),
        Default::default(),
    );
    registry.register_or_replace(
        other_guid,
        broadcast_info_with_command(other_guid, other_tx, other_command_tx),
        Default::default(),
    );

    let movement = MovementInfo {
        guid,
        flags: MovementFlag::FORWARD
            | MovementFlag::BACKWARD
            | MovementFlag::HOVER
            | MovementFlag::WATER_WALK,
        position: wow_core::Position::new(10.0, 20.0, 30.0, 1.0),
        ..MovementInfo::default()
    };
    let mut inbound = wow_packet::WorldPacket::new_empty();
    inbound.write_uint16(ClientOpcodes::MoveHeartbeat as u16);
    movement.write(&mut inbound);
    inbound.read_uint16().expect("movement opcode");
    session.handle_movement(inbound).await;

    assert!(self_rx.try_recv().is_err());
    assert!(other_rx.try_recv().is_err());
    assert!(self_command_rx.try_recv().is_err());
    let command = other_command_rx
        .try_recv()
        .expect("visible movement command");
    let crate::session::mailbox::SessionCommand::SendIfVisibleLikeCpp(command) = command else {
        panic!("expected SendIfVisibleLikeCpp movement command");
    };
    assert_eq!(command.source_guid, guid);
    assert_eq!(command.map_id, 0);
    assert_eq!(command.instance_id, 0);
    let bytes = command.packet_bytes;
    let mut packet = wow_packet::WorldPacket::from_bytes(&bytes);
    assert_eq!(
        packet.server_opcode(),
        Some(wow_constants::ServerOpcodes::MoveUpdate)
    );
    packet.read_uint16().expect("move update opcode");
    let sanitized = MovementInfo::read(&mut packet).expect("move update status");
    assert_eq!(sanitized.flags, MovementFlag::empty());
    assert_eq!(
        session.player_movement_flags_like_cpp(),
        MovementFlag::empty()
    );
}

#[test]
fn movement_directory_rejects_replaced_recipient_generation_like_cpp() {
    let source_guid = ObjectGuid::create_player(1, 44);
    let recipient_guid = ObjectGuid::create_player(1, 45);
    let registry = crate::session::directory::PlayerRegistry::default();
    let (old_send_tx, _old_send_rx) = flume::bounded(1);
    let (old_command_tx, old_command_rx) = flume::bounded(1);
    registry.register_or_replace(
        recipient_guid,
        broadcast_info_with_command(recipient_guid, old_send_tx, old_command_tx),
        Default::default(),
    );
    let recipients = registry.movement_recipients_within_range(
        source_guid,
        0,
        0,
        Position::ZERO,
        crate::map_manager::VISIBILITY_RADIUS,
    );
    let [stale] = recipients.as_slice() else {
        panic!("expected the first recipient generation");
    };
    let stale = *stale;

    let (replacement_send_tx, _replacement_send_rx) = flume::bounded(1);
    let (replacement_command_tx, replacement_command_rx) = flume::bounded(1);
    registry.register_or_replace(
        recipient_guid,
        broadcast_info_with_command(recipient_guid, replacement_send_tx, replacement_command_tx),
        Default::default(),
    );

    let result = registry.try_send_current_command(
        stale,
        crate::session::mailbox::SessionCommand::RefreshVisibleGameobjectsOrSpellClicksLikeCpp,
    );
    assert_eq!(
        result,
        Err(crate::session::directory::PlayerDirectorySendError::StaleRegistration)
    );
    assert!(old_command_rx.try_recv().is_err());
    assert!(replacement_command_rx.try_recv().is_err());
}

#[tokio::test]
async fn handle_movement_uses_current_mover_guid_like_cpp() {
    let mut session = make_session();
    let player_guid = ObjectGuid::create_player(1, 142);
    let mover_guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 777, 1_142);
    let other_guid = ObjectGuid::create_player(1, 143);
    let player_position = Position::new(1.0, 2.0, 3.0, 0.5);
    let mover_start = Position::new(10.0, 10.0, 0.0, 0.0);
    let moved_position = Position::new(12.0, 13.0, 1.0, 1.25);
    let manager = Arc::new(RwLock::new(crate::map_manager::MapManager::new()));
    let registry = Arc::new(crate::session::directory::PlayerRegistry::default());
    let (self_tx, _self_rx) = flume::bounded(1);
    let (other_tx, other_rx) = flume::bounded(1);
    let (self_command_tx, self_command_rx) = flume::bounded(1);
    let (other_command_tx, other_command_rx) = flume::bounded(1);

    session.set_player_guid(Some(player_guid));
    session.set_player_moved_unit_guid_like_cpp(mover_guid);
    session.set_player_position_like_cpp(player_position);
    session.set_player_movement_time_like_cpp(7_777);
    session.set_player_movement_flags_like_cpp(MovementFlag::SWIMMING);
    session.set_map_manager(Arc::clone(&manager));
    session.set_player_registry(Arc::clone(&registry));

    let (grid_x, grid_y) = crate::map_manager::world_to_grid_coords(mover_start.x, mover_start.y);
    manager.write().unwrap().add_creature(
        0,
        0,
        grid_x,
        grid_y,
        crate::map_manager::WorldCreature::new(
            mover_guid,
            777,
            mover_start,
            100,
            80,
            1,
            2,
            0.0,
            1,
            35,
            0,
            0,
        ),
    );

    registry.register_or_replace(
        player_guid,
        broadcast_info_with_command(player_guid, self_tx, self_command_tx),
        Default::default(),
    );
    let mut other_info = broadcast_info_with_command(other_guid, other_tx, other_command_tx);
    other_info.placement.position = moved_position;
    registry.register_or_replace(other_guid, other_info, Default::default());

    let movement = MovementInfo {
        guid: mover_guid,
        flags: MovementFlag::FORWARD,
        time: 1_234,
        position: moved_position,
        ..MovementInfo::default()
    };
    session
        .handle_movement(movement_packet(ClientOpcodes::MoveHeartbeat, &movement))
        .await;

    assert_eq!(session.player_position_like_cpp(), Some(player_position));
    assert_eq!(session.player_movement_time_like_cpp(), 7_777);
    assert_eq!(
        session.player_movement_flags_like_cpp(),
        MovementFlag::SWIMMING
    );
    let (creature_position, creature_flags, creature_time) = {
        let guard = manager.read().unwrap();
        let creature = guard
            .find_creature(0, 0, mover_guid)
            .expect("controlled mover creature");
        (
            creature.position(),
            creature.creature.unit().movement_flags_like_cpp(),
            creature.creature.unit().movement_time_like_cpp(),
        )
    };
    assert_eq!(creature_position, moved_position);
    assert_eq!(creature_flags, MovementFlag::FORWARD);

    assert!(self_command_rx.try_recv().is_err());
    assert!(other_rx.try_recv().is_err());
    let command = other_command_rx
        .try_recv()
        .expect("visible controlled-mover movement command");
    let crate::session::mailbox::SessionCommand::SendIfVisibleLikeCpp(command) = command else {
        panic!("expected SendIfVisibleLikeCpp movement command");
    };
    assert_eq!(command.source_guid, mover_guid);
    let mut packet = wow_packet::WorldPacket::from_bytes(&command.packet_bytes);
    assert_eq!(
        packet.server_opcode(),
        Some(wow_constants::ServerOpcodes::MoveUpdate)
    );
    packet.read_uint16().expect("move update opcode");
    let status = MovementInfo::read(&mut packet).expect("move update status");
    assert_eq!(status.guid, mover_guid);
    assert_eq!(status.flags, MovementFlag::FORWARD);
    assert_eq!(status.position, moved_position);
    assert_eq!(creature_time, status.time);
}

#[tokio::test]
async fn handle_movement_does_not_broadcast_outside_visibility_range_like_cpp() {
    let mut session = make_session();
    let guid = ObjectGuid::create_player(1, 42);
    let far_guid = ObjectGuid::create_player(1, 44);
    let registry = std::sync::Arc::new(crate::session::directory::PlayerRegistry::default());
    let (self_tx, _self_rx) = flume::bounded(1);
    let (far_tx, far_rx) = flume::bounded(1);
    let (self_command_tx, self_command_rx) = flume::bounded(1);
    let (far_command_tx, far_command_rx) = flume::bounded(1);

    session.set_player_guid(Some(guid));
    session.set_player_registry(std::sync::Arc::clone(&registry));
    registry.register_or_replace(
        guid,
        broadcast_info_with_command(guid, self_tx, self_command_tx),
        Default::default(),
    );
    let mut far_info = broadcast_info_with_command(far_guid, far_tx, far_command_tx);
    far_info.placement.position =
        wow_core::Position::new(crate::map_manager::VISIBILITY_RADIUS + 10.0, 0.0, 0.0, 0.0);
    registry.register_or_replace(far_guid, far_info, Default::default());

    let movement = MovementInfo {
        guid,
        flags: MovementFlag::FORWARD,
        position: wow_core::Position::ZERO,
        ..MovementInfo::default()
    };
    let mut inbound = wow_packet::WorldPacket::new_empty();
    inbound.write_uint16(ClientOpcodes::MoveHeartbeat as u16);
    movement.write(&mut inbound);
    inbound.read_uint16().expect("movement opcode");
    session.handle_movement(inbound).await;

    assert!(self_command_rx.try_recv().is_err());
    assert!(far_command_rx.try_recv().is_err());
    assert!(far_rx.try_recv().is_err());
}

#[tokio::test]
async fn handle_movement_syncs_canonical_player_position_for_logout_save_like_cpp() {
    let mut session = make_session();
    let canonical = Arc::new(Mutex::new(wow_map::MapManager::default()));
    let guid = ObjectGuid::create_player(1, 1042);
    let login_position = Position::new(1.0, 2.0, 3.0, 0.25);
    let moved_position = Position::new(90.0, 20.0, 30.0, 1.0);

    canonical.lock().unwrap().create_world_map(571, 0);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
        wow_data::MapEntry {
            id: 571,
            instance_type: wow_data::map::MAP_COMMON,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
    ])));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        guid,
        "MovementSaver".to_string(),
        login_position,
        571,
        1,
        3,
        10,
        0,
    ));
    let _ = session.ensure_canonical_world_map_for_current_player_like_cpp();

    let movement = MovementInfo {
        guid,
        flags: MovementFlag::FORWARD,
        time: 12_345,
        position: moved_position,
        ..MovementInfo::default()
    };
    let mut inbound = wow_packet::WorldPacket::new_empty();
    inbound.write_uint16(ClientOpcodes::MoveHeartbeat as u16);
    movement.write(&mut inbound);
    inbound.read_uint16().expect("movement opcode");
    session.handle_movement(inbound).await;

    let canonical_position = canonical
        .lock()
        .unwrap()
        .find_map(571, 0)
        .and_then(|map| map.map().get_typed_player(guid))
        .map(|player| player.unit().world().position())
        .expect("canonical player");
    assert_eq!(canonical_position, moved_position);
    let canonical_cell = canonical
        .lock()
        .unwrap()
        .find_map(571, 0)
        .and_then(|map| map.map().get_typed_player(guid))
        .and_then(|player| player.unit().world().current_cell())
        .expect("canonical player cell");
    let expected_cell = wow_map::cell_from_world(moved_position.x, moved_position.y);
    assert_eq!(
        canonical_cell,
        (expected_cell.cell_x(), expected_cell.cell_y()),
        "C++ Map::PlayerRelocation moves the Player between derived cell indexes"
    );
    let canonical_movement_flags = canonical
        .lock()
        .unwrap()
        .find_map(571, 0)
        .and_then(|map| map.map().get_typed_player(guid))
        .map(|player| player.unit().movement_flags_like_cpp())
        .expect("canonical player movement flags");
    assert_eq!(
        canonical_movement_flags,
        MovementFlag::FORWARD,
        "C++ stores accepted player MovementInfo flags on Unit::m_movementInfo"
    );
    let canonical_movement_time = canonical
        .lock()
        .unwrap()
        .find_map(571, 0)
        .and_then(|map| map.map().get_typed_player(guid))
        .map(|player| player.unit().movement_time_like_cpp())
        .expect("canonical player movement time");
    assert_eq!(
        canonical_movement_time,
        session.player_movement_time_like_cpp(),
        "C++ stores accepted player MovementInfo time on Unit::m_movementInfo"
    );
    assert_eq!(
        session
            .current_player_save_to_db_snapshot_like_cpp()
            .unwrap()
            .position,
        moved_position
    );
}

#[tokio::test]
async fn handle_movement_discovers_current_area_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_rx();
    let canonical = Arc::new(Mutex::new(wow_map::MapManager::default()));
    let guid = ObjectGuid::create_player(1, 1094);
    let login_position = Position::new(1.0, 2.0, 3.0, 0.25);
    let moved_position = Position::new(11.0, 22.0, 33.0, 1.0);

    canonical.lock().unwrap().create_world_map(571, 0);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
        wow_data::MapEntry {
            id: 571,
            instance_type: wow_data::map::MAP_COMMON,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
    ])));
    session.set_area_table_store(Arc::new(wow_data::AreaTableStore::from_entries([
        wow_data::AreaTableEntry {
            id: 9_104,
            continent_id: 571,
            parent_area_id: 0,
            area_bit: 65,
            exploration_level: 12,
            mount_flags: 0,
            flags: 0,
        },
    ])));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        guid,
        "MovementExplorer".to_string(),
        login_position,
        571,
        1,
        3,
        10,
        0,
    ));
    session.set_player_zone_area_like_cpp(9_104, 9_104);
    let _ = session.ensure_canonical_world_map_for_current_player_like_cpp();

    let movement = MovementInfo {
        guid,
        flags: MovementFlag::FORWARD,
        position: moved_position,
        ..MovementInfo::default()
    };
    session
        .handle_movement(movement_packet(ClientOpcodes::MoveHeartbeat, &movement))
        .await;

    assert_eq!(
        session
            .represented_explored_zones_db_string_like_cpp()
            .expect("test Player explored-zones owner resolves")
            .split_whitespace()
            .take(4)
            .collect::<Vec<_>>(),
        vec!["0", "0", "2", "0"]
    );
    assert_eq!(
        session.represented_reveal_world_map_overlay_criteria_like_cpp(),
        &[9_104]
    );
    assert!(drain_server_opcodes(&send_rx).contains(&ServerOpcodes::UpdateObject));

    session
        .handle_movement(movement_packet(ClientOpcodes::MoveHeartbeat, &movement))
        .await;
    assert_eq!(
        session.represented_reveal_world_map_overlay_criteria_like_cpp(),
        &[9_104],
        "C++ discovery criteria only fires when the explored-zone bit changes"
    );
    assert!(!drain_server_opcodes(&send_rx).contains(&ServerOpcodes::UpdateObject));
}

#[tokio::test]
async fn handle_movement_resolves_zone_area_for_cemetery_flow_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send_rx();
    let canonical = Arc::new(Mutex::new(wow_map::MapManager::default()));
    let guid = ObjectGuid::create_player(1, 1095);
    let map_id = 1_u32;
    let login_position = Position::new(1.0, 2.0, 3.0, 0.25);
    let moved_position = Position::new(1922.0, -4345.0, 25.0, 1.0);
    let data_dir = unique_temp_data_dir("zone-area-cemetery");
    write_single_area_map_tile_like_cpp(
        &data_dir,
        map_id,
        moved_position.x,
        moved_position.y,
        5170,
    );

    canonical.lock().unwrap().create_world_map(map_id, 0);
    session.set_mmap_runtime_config_like_cpp(MMapRuntimeConfigLikeCpp {
        data_dir: data_dir.to_string_lossy().into_owned(),
        ..MMapRuntimeConfigLikeCpp::default()
    });
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
        wow_data::MapEntry {
            id: map_id,
            instance_type: wow_data::map::MAP_COMMON,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
    ])));
    session.set_area_table_store(Arc::new(wow_data::AreaTableStore::from_entries([
        wow_data::AreaTableEntry {
            id: 1637,
            continent_id: map_id as u16,
            parent_area_id: 0,
            area_bit: -1,
            exploration_level: 0,
            mount_flags: 0,
            flags: 0,
        },
        wow_data::AreaTableEntry {
            id: 5170,
            continent_id: map_id as u16,
            parent_area_id: 1637,
            area_bit: -1,
            exploration_level: 0,
            mount_flags: 0,
            flags: 0x4000_0000,
        },
    ])));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        guid,
        "MovementZone".to_string(),
        login_position,
        map_id as u16,
        10,
        5,
        80,
        0,
    ));
    session.set_player_moved_unit_guid_like_cpp(guid);
    session.set_player_zone_area_like_cpp(1, 1);
    let _ = session.ensure_canonical_world_map_for_current_player_like_cpp();

    let movement = MovementInfo {
        guid,
        flags: MovementFlag::FORWARD,
        position: moved_position,
        ..MovementInfo::default()
    };
    session
        .handle_movement(movement_packet(ClientOpcodes::MoveHeartbeat, &movement))
        .await;

    assert_eq!(
        session.player_zone_area_like_cpp(),
        Some((1637, 5170)),
        "C++ Player::Update uses terrain GetZoneAndAreaId, so cemetery requests after movement must use the Orgrimmar zone, not stale DB zone"
    );
}

#[tokio::test]
async fn logout_save_snapshot_uses_canonical_position_not_stale_session_mirror_like_cpp() {
    let mut session = make_session();
    let canonical = Arc::new(Mutex::new(wow_map::MapManager::default()));
    let guid = ObjectGuid::create_player(1, 1043);
    let login_position = Position::new(1.0, 2.0, 3.0, 0.25);
    let latest_session_position = Position::new(4.0, 5.0, 6.0, 0.5);
    let stale_canonical_position = Position::new(10.0, 20.0, 30.0, 1.0);

    canonical.lock().unwrap().create_world_map(571, 0);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
        wow_data::MapEntry {
            id: 571,
            instance_type: wow_data::map::MAP_COMMON,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
    ])));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        guid,
        "LogoutSaver".to_string(),
        login_position,
        571,
        1,
        3,
        10,
        0,
    ));
    let _ = session.ensure_canonical_world_map_for_current_player_like_cpp();
    session.set_player_position_like_cpp(latest_session_position);
    session.mutate_canonical_player_like_cpp(|player| {
        player
            .unit_mut()
            .world_mut()
            .relocate(stale_canonical_position);
    });

    let snapshot = session
        .current_player_save_to_db_snapshot_like_cpp()
        .expect("save snapshot");

    assert_eq!(snapshot.position, stale_canonical_position);
    assert_eq!(
        session.player_position_like_cpp(),
        Some(stale_canonical_position)
    );
}

#[tokio::test]
async fn handle_movement_rejects_guid_mismatch_without_state_or_broadcast_like_cpp() {
    let mut session = make_session();
    let guid = ObjectGuid::create_player(1, 42);
    let spoofed_guid = ObjectGuid::create_player(1, 99);
    let other_guid = ObjectGuid::create_player(1, 43);
    let original_position = wow_core::Position::new(1.0, 2.0, 3.0, 0.5);
    let registry = std::sync::Arc::new(crate::session::directory::PlayerRegistry::default());
    let (other_tx, other_rx) = flume::bounded(1);

    session.set_player_guid(Some(guid));
    session.set_player_position_like_cpp(original_position);
    session.set_player_movement_flags_like_cpp(MovementFlag::SWIMMING);
    session.set_player_registry(std::sync::Arc::clone(&registry));
    registry.register_or_replace(
        other_guid,
        broadcast_info(other_guid, other_tx),
        Default::default(),
    );

    let movement = MovementInfo {
        guid: spoofed_guid,
        flags: MovementFlag::FORWARD | MovementFlag::BACKWARD,
        position: wow_core::Position::new(10.0, 20.0, 30.0, 1.0),
        ..MovementInfo::default()
    };
    let mut inbound = wow_packet::WorldPacket::new_empty();
    inbound.write_uint16(ClientOpcodes::MoveHeartbeat as u16);
    movement.write(&mut inbound);
    inbound.read_uint16().expect("movement opcode");
    session.handle_movement(inbound).await;

    assert_eq!(session.player_position_like_cpp(), Some(original_position));
    assert_eq!(
        session.player_movement_flags_like_cpp(),
        MovementFlag::SWIMMING
    );
    assert!(other_rx.try_recv().is_err());
}

#[tokio::test]
async fn handle_movement_rejects_invalid_position_without_state_or_broadcast_like_cpp() {
    let mut session = make_session();
    let guid = ObjectGuid::create_player(1, 42);
    let other_guid = ObjectGuid::create_player(1, 43);
    let original_position = wow_core::Position::new(1.0, 2.0, 3.0, 0.5);
    let registry = std::sync::Arc::new(crate::session::directory::PlayerRegistry::default());
    let (other_tx, other_rx) = flume::bounded(1);

    session.set_player_guid(Some(guid));
    session.set_player_position_like_cpp(original_position);
    session.set_player_movement_flags_like_cpp(MovementFlag::SWIMMING);
    session.set_player_registry(std::sync::Arc::clone(&registry));
    registry.register_or_replace(
        other_guid,
        broadcast_info(other_guid, other_tx),
        Default::default(),
    );

    let movement = MovementInfo {
        guid,
        flags: MovementFlag::HOVER | MovementFlag::WATER_WALK,
        position: wow_core::Position::new(f32::NAN, 20.0, 30.0, 1.0),
        ..MovementInfo::default()
    };
    let mut inbound = wow_packet::WorldPacket::new_empty();
    inbound.write_uint16(ClientOpcodes::MoveHeartbeat as u16);
    movement.write(&mut inbound);
    inbound.read_uint16().expect("movement opcode");
    session.handle_movement(inbound).await;

    assert_eq!(session.player_position_like_cpp(), Some(original_position));
    assert_eq!(
        session.player_movement_flags_like_cpp(),
        MovementFlag::SWIMMING
    );
    assert!(other_rx.try_recv().is_err());
}

#[test]
fn movement_ack_validation_sanitizes_status_flags_like_cpp() {
    let mut session = make_session();
    let guid = ObjectGuid::create_player(1, 42);
    session.set_player_guid(Some(guid));
    let mut ack = wow_packet::packets::movement::MovementAck {
        status: MovementInfo {
            guid,
            flags: MovementFlag::HOVER | MovementFlag::WATER_WALK,
            position: wow_core::Position::new(10.0, 20.0, 30.0, 1.5),
            ..MovementInfo::default()
        },
        ack_index: 12,
    };

    assert!(session.record_validated_movement_ack_like_cpp(
        ClientOpcodes::MoveHoverAck,
        &mut ack,
        None
    ));
    assert!(ack.status.flags.is_empty());
    assert!(session.movement_ack_events_like_cpp()[0].accepted);
}

#[test]
fn move_set_vehicle_rec_ack_only_sanitizes_status_like_cpp() {
    let mut session = make_session();
    let mut ack = wow_packet::packets::movement::MovementAck {
        status: MovementInfo {
            guid: ObjectGuid::create_player(1, 77),
            flags: MovementFlag::HOVER | MovementFlag::WATER_WALK,
            position: wow_core::Position::new(f32::NAN, 20.0, 30.0, 1.5),
            ..MovementInfo::default()
        },
        ack_index: 77,
    };

    session.apply_move_set_vehicle_rec_id_ack_like_cpp(&mut ack);

    assert!(
        ack.status.flags.is_empty(),
        "C++ Player::ValidateMovementInfo strips invalid flags for this ACK"
    );
    assert!(
        session.movement_ack_events_like_cpp().is_empty(),
        "C++ HandleMoveSetVehicleRecAck does not run the generic movement ACK path"
    );
}
