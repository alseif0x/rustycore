//! Session scenarios exercising the represented visibility responsibility.
//!
//! Split out of session_tests.rs under #626; assertions and registrations
//! are unchanged and the shared fixtures stay in the parent module.

use super::*;

#[test]
fn player_attack_uses_canonical_invisibility_detection_like_cpp() {
    let (mut session, _, _) = make_session();
    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    let attacker = ObjectGuid::create_player(1, 95);
    let victim = test_creature_guid(18_095);

    canonical.lock().unwrap().create_world_map(0, 0);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
        wow_data::MapEntry {
            id: 0,
            instance_type: wow_data::map::MAP_COMMON,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
    ])));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        attacker,
        "Rogue".to_string(),
        Position::new(9.0, 10.0, 0.0, 0.0),
        0,
        1,
        4,
        80,
        0,
    ));
    let _ = session.ensure_canonical_world_map_for_current_player_like_cpp();
    register_test_creature(&mut session, manager, victim, 40);
    session
        .mutate_canonical_creature_by_guid_like_cpp(victim, |creature| {
            creature.unit_mut().set_invisibility_like_cpp(1, 50);
        })
        .unwrap();

    assert_eq!(
        session.start_player_attack_like_cpp(victim),
        PlayerAttackStartLikeCppResult::Rejected
    );
    assert_eq!(session.combat_target, None);

    session
        .mutate_canonical_player_by_guid_like_cpp(attacker, |player| {
            player.unit_mut().set_invisibility_detect_like_cpp(1, 50);
        })
        .unwrap();

    assert_eq!(
        session.start_player_attack_like_cpp(victim),
        PlayerAttackStartLikeCppResult::Accepted {
            send_attack_start: true
        }
    );
    let guard = canonical.lock().unwrap();
    let map = guard.find_map(0, 0).unwrap().map();
    assert_eq!(
        map.get_typed_player(attacker).unwrap().unit().attacking(),
        Some(victim)
    );
}
#[test]
fn player_attack_uses_canonical_farsight_can_always_see_like_cpp() {
    let (mut session, _, _) = make_session();
    let canonical = shared_canonical_map_manager();
    let attacker = ObjectGuid::create_player(1, 53);
    let victim = ObjectGuid::create_player(1, 54);

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
        attacker,
        "Hunter".to_string(),
        Position::new(10.0, 20.0, 30.0, 0.0),
        571,
        1,
        3,
        80,
        0,
    ));
    let _ = session.ensure_canonical_world_map_for_current_player_like_cpp();

    let mut victim_player = Player::new(Some(8), false);
    victim_player
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(victim);
    victim_player
        .unit_mut()
        .world_mut()
        .set_map(571, 0)
        .unwrap();
    victim_player
        .unit_mut()
        .world_mut()
        .relocate(Position::new(11.0, 20.0, 30.0, 0.0));
    victim_player
        .unit_mut()
        .world_mut()
        .object_mut()
        .add_to_world();
    victim_player
        .unit_mut()
        .set_pvp_flag_like_cpp(UnitPvpFlags::PVP);
    victim_player.unit_mut().set_invisibility_like_cpp(0, 100);
    canonical
        .lock()
        .unwrap()
        .find_map_mut(571, 0)
        .unwrap()
        .map_mut()
        .insert_map_object_record(wow_entities::MapObjectRecord::new_player(victim_player).unwrap())
        .unwrap();

    assert_eq!(
        session.start_player_attack_like_cpp(victim),
        PlayerAttackStartLikeCppResult::Rejected
    );

    session
        .mutate_canonical_player_by_guid_like_cpp(attacker, |player| {
            player.set_farsight_object_like_cpp(victim);
        })
        .unwrap();
    assert_eq!(
        session.start_player_attack_like_cpp(victim),
        PlayerAttackStartLikeCppResult::Accepted {
            send_attack_start: true
        }
    );
}
#[test]
fn player_attack_uses_owner_group_visibility_like_cpp() {
    let (mut session, _, _) = make_session();
    let canonical = shared_canonical_map_manager();
    let attacker = ObjectGuid::create_player(1, 61);
    let owner = ObjectGuid::create_player(1, 62);
    let victim = ObjectGuid::create_player(1, 63);

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
        attacker,
        "Hunter".to_string(),
        Position::new(10.0, 20.0, 30.0, 0.0),
        571,
        1,
        3,
        80,
        0,
    ));
    let _ = session.ensure_canonical_world_map_for_current_player_like_cpp();

    let mut victim_player = Player::new(Some(8), false);
    victim_player
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(victim);
    victim_player
        .unit_mut()
        .world_mut()
        .set_map(571, 0)
        .unwrap();
    victim_player
        .unit_mut()
        .world_mut()
        .relocate(Position::new(11.0, 20.0, 30.0, 0.0));
    victim_player
        .unit_mut()
        .world_mut()
        .object_mut()
        .add_to_world();
    victim_player
        .unit_mut()
        .set_pvp_flag_like_cpp(UnitPvpFlags::PVP);
    victim_player.unit_mut().set_invisibility_like_cpp(0, 100);
    victim_player
        .unit_mut()
        .subsystems_mut()
        .control
        .set_owner_guid(Some(owner));
    canonical
        .lock()
        .unwrap()
        .find_map_mut(571, 0)
        .unwrap()
        .map_mut()
        .insert_map_object_record(wow_entities::MapObjectRecord::new_player(victim_player).unwrap())
        .unwrap();

    assert_eq!(
        session.start_player_attack_like_cpp(victim),
        PlayerAttackStartLikeCppResult::Rejected
    );

    let group_registry = Arc::new(GroupRegistry::default());
    let mut group = GroupInfo::new(attacker);
    group.add_member(owner);
    let group_guid = group.group_guid;
    group_registry.register_group_like_cpp(group_guid, group);
    session.set_group_registry(group_registry, Arc::new(PendingInvites::default()));
    assert!(session.set_owned_player_group_like_cpp(Some((group_guid, 0))));
    assert_eq!(
        session.start_player_attack_like_cpp(victim),
        PlayerAttackStartLikeCppResult::Accepted {
            send_attack_start: true
        }
    );
}
#[test]
fn player_attack_uses_private_object_group_visibility_like_cpp() {
    let (mut session, _, _) = make_session();
    let canonical = shared_canonical_map_manager();
    let attacker = ObjectGuid::create_player(1, 64);
    let victim = ObjectGuid::create_player(1, 65);

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
        attacker,
        "Hunter".to_string(),
        Position::new(10.0, 20.0, 30.0, 0.0),
        571,
        1,
        3,
        80,
        0,
    ));
    let _ = session.ensure_canonical_world_map_for_current_player_like_cpp();

    let mut victim_player = Player::new(Some(8), false);
    victim_player
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(victim);
    victim_player
        .unit_mut()
        .world_mut()
        .set_map(571, 0)
        .unwrap();
    victim_player
        .unit_mut()
        .world_mut()
        .relocate(Position::new(11.0, 20.0, 30.0, 0.0));
    victim_player
        .unit_mut()
        .world_mut()
        .object_mut()
        .add_to_world();
    victim_player
        .unit_mut()
        .set_pvp_flag_like_cpp(UnitPvpFlags::PVP);
    canonical
        .lock()
        .unwrap()
        .find_map_mut(571, 0)
        .unwrap()
        .map_mut()
        .insert_map_object_record(wow_entities::MapObjectRecord::new_player(victim_player).unwrap())
        .unwrap();

    let group_registry = Arc::new(GroupRegistry::default());
    let group = GroupInfo::new(attacker);
    let group_guid = group.group_guid;
    session
        .mutate_canonical_player_by_guid_like_cpp(victim, |player| {
            player
                .unit_mut()
                .set_private_object_owner_like_cpp(ObjectGuid::create_group(group_guid));
        })
        .unwrap();
    assert_eq!(
        session.start_player_attack_like_cpp(victim),
        PlayerAttackStartLikeCppResult::Rejected
    );

    group_registry.register_group_like_cpp(group_guid, group);
    session.set_group_registry(group_registry, Arc::new(PendingInvites::default()));
    assert!(session.set_owned_player_group_like_cpp(Some((group_guid, 0))));
    assert_eq!(
        session.start_player_attack_like_cpp(victim),
        PlayerAttackStartLikeCppResult::Accepted {
            send_attack_start: true
        }
    );
}
#[test]
fn overlapping_rest_flags_only_report_visible_zero_boundary_like_cpp() {
    let (mut session, _, _) = make_session();

    assert!(session.set_represented_rest_flag_like_cpp(REST_FLAG_IN_CITY_LIKE_CPP, 0));
    assert!(!session.set_represented_rest_flag_like_cpp(REST_FLAG_IN_TAVERN_LIKE_CPP, 42));
    assert!(!session.remove_represented_rest_flag_like_cpp(REST_FLAG_IN_CITY_LIKE_CPP));
    assert!(session.remove_represented_rest_flag_like_cpp(REST_FLAG_IN_TAVERN_LIKE_CPP));
}
#[test]
fn fractional_rest_bonus_only_reports_visible_threshold_or_state_change_like_cpp() {
    let (mut session, _, _) = make_session();
    session.set_loaded_player_identity_like_cpp(1, 1, 8, 10, 0);
    session.set_player_next_level_xp_like_cpp(72_000);
    session.load_represented_xp_rest_bonus_like_cpp(REST_STATE_NORMAL_LIKE_CPP, 0.0);

    assert_eq!(session.add_represented_xp_rest_bonus_like_cpp(0.5), 0);
    assert_eq!(session.represented_xp_rest_threshold_like_cpp(), 0);
    assert_eq!(
        session.represented_xp_rest_state_like_cpp(),
        REST_STATE_NORMAL_LIKE_CPP
    );

    assert_eq!(session.add_represented_xp_rest_bonus_like_cpp(0.5), 0x07);
    assert_eq!(session.represented_xp_rest_threshold_like_cpp(), 1);
    assert_eq!(
        session.represented_xp_rest_state_like_cpp(),
        REST_STATE_RESTED_LIKE_CPP
    );
}
#[test]
fn player_attack_waits_for_active_mover_visibility_flag_like_cpp() {
    let (mut session, _, _) = make_session();
    let canonical = shared_canonical_map_manager();
    let attacker = ObjectGuid::create_player(1, 55);
    let victim = ObjectGuid::create_player(1, 56);

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
        attacker,
        "Warrior".to_string(),
        Position::new(10.0, 20.0, 30.0, 0.0),
        571,
        1,
        1,
        80,
        0,
    ));
    let _ = session.ensure_canonical_world_map_for_current_player_like_cpp();

    let mut victim_player = Player::new(Some(8), false);
    victim_player
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(victim);
    victim_player
        .unit_mut()
        .world_mut()
        .set_map(571, 0)
        .unwrap();
    victim_player
        .unit_mut()
        .world_mut()
        .relocate(Position::new(11.0, 20.0, 30.0, 0.0));
    victim_player
        .unit_mut()
        .world_mut()
        .object_mut()
        .add_to_world();
    victim_player
        .unit_mut()
        .set_pvp_flag_like_cpp(UnitPvpFlags::PVP);
    canonical
        .lock()
        .unwrap()
        .find_map_mut(571, 0)
        .unwrap()
        .map_mut()
        .insert_map_object_record(wow_entities::MapObjectRecord::new_player(victim_player).unwrap())
        .unwrap();

    session.set_active_player_local_flags_like_cpp(0);
    assert_eq!(
        session.start_player_attack_like_cpp(victim),
        PlayerAttackStartLikeCppResult::Rejected
    );
    assert_eq!(session.combat_target, None);

    session.apply_move_init_active_mover_complete_like_cpp(0);
    assert_eq!(
        session.start_player_attack_like_cpp(victim),
        PlayerAttackStartLikeCppResult::Accepted {
            send_attack_start: true
        }
    );
}
#[test]
fn player_attack_rejects_object_id_visibility_conditions_like_cpp() {
    let (mut session, _, _) = make_session();
    let canonical = shared_canonical_map_manager();
    let attacker = ObjectGuid::create_player(1, 57);
    let victim = ObjectGuid::create_player(1, 58);

    session.set_condition_store(Arc::new(
        ConditionEntriesByTypeStore::from_conditions_like_cpp([Condition {
            source_type: ConditionSourceType::ObjectIdVisibility,
            source_group: TypeId::Player as u32,
            source_entry: 0,
            source_id: 0,
            condition_type: ConditionType::Aura,
            condition_value1: 999_999,
            condition_value2: 0,
            ..Condition::default()
        }]),
    ));
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
        attacker,
        "Warrior".to_string(),
        Position::new(10.0, 20.0, 30.0, 0.0),
        571,
        1,
        1,
        80,
        0,
    ));
    let _ = session.ensure_canonical_world_map_for_current_player_like_cpp();

    let mut victim_player = Player::new(Some(8), false);
    victim_player
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(victim);
    victim_player
        .unit_mut()
        .world_mut()
        .set_map(571, 0)
        .unwrap();
    victim_player
        .unit_mut()
        .world_mut()
        .relocate(Position::new(11.0, 20.0, 30.0, 0.0));
    victim_player
        .unit_mut()
        .world_mut()
        .object_mut()
        .add_to_world();
    victim_player
        .unit_mut()
        .set_pvp_flag_like_cpp(UnitPvpFlags::PVP);
    canonical
        .lock()
        .unwrap()
        .find_map_mut(571, 0)
        .unwrap()
        .map_mut()
        .insert_map_object_record(wow_entities::MapObjectRecord::new_player(victim_player).unwrap())
        .unwrap();

    assert_eq!(
        session.start_player_attack_like_cpp(victim),
        PlayerAttackStartLikeCppResult::Rejected
    );
    assert_eq!(session.combat_target, None);
}
#[test]
fn send_update_world_state_like_cpp_visible_preserves_field_order_and_signed_value() {
    let (session, _, send_rx) = make_session();
    let variable_id = 0x1122_3344;
    let value = -1_234_567;
    let expected = wow_packet::packets::misc::UpdateWorldState {
        variable_id,
        value,
        hidden: false,
    }
    .to_bytes();

    session.send_update_world_state_like_cpp(variable_id, value, false);

    let data = send_rx.try_recv().unwrap();
    assert_eq!(data, expected);
    assert_eq!(send_rx.try_recv(), Err(flume::TryRecvError::Empty));
    assert_eq!(data.len(), 11);
    assert_eq!(
        u16::from_le_bytes([data[0], data[1]]),
        ServerOpcodes::UpdateWorldState as u16
    );
    assert_eq!(&data[2..6], &variable_id.to_le_bytes());
    assert_eq!(&data[6..10], &value.to_le_bytes());
    assert_eq!(data[10], 0x00);
}
#[test]
fn far_sight_enable_existing_viewpoint_sets_seer_without_mutating_farsight_like_cpp() {
    let (mut session, _, _) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 4252);
    let target_guid = test_creature_guid(4253);

    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_player_guid(Some(player_guid));
    session.player_name = Some("FarSight".into());
    session.player_position = Some(Position::new(10.0, 10.0, 0.0, 0.0));
    session.current_map_id = 571;
    insert_session_player_into_canonical_map_like_cpp(&session, &canonical, 571, 0);
    add_canonical_test_creature(
        &canonical,
        target_guid,
        9001,
        Position::new(12.0, 10.0, 0.0, 0.0),
        0,
    );
    set_canonical_player_farsight_object_like_cpp(&canonical, player_guid, target_guid);

    session.apply_far_sight_like_cpp(true);

    assert_eq!(session.represented_seer_guid_like_cpp(), Some(target_guid));
    assert_eq!(
        canonical
            .lock()
            .unwrap()
            .find_map(571, 0)
            .unwrap()
            .map()
            .get_typed_player(player_guid)
            .unwrap()
            .active_data()
            .farsight_object,
        target_guid
    );
}
#[tokio::test]
async fn far_sight_empty_or_missing_viewpoint_keeps_seer_and_forces_visibility_like_cpp() {
    let (mut session, _, _) = make_session();
    let player_guid = ObjectGuid::create_player(1, 4254);
    let original_seer = test_creature_guid(4255);
    let pos = Position::new(10.0, 10.0, 0.0, 0.0);

    session.set_player_guid(Some(player_guid));
    session.player_position = Some(pos);
    session.current_map_id = 571;
    session.represented_seer_guid_like_cpp = Some(original_seer);
    session.last_visibility_pos = Some(pos);

    let mut pkt = WorldPacket::new_empty();
    pkt.write_bit(true);
    pkt.flush_bits();
    pkt.reset_read();
    session.handle_far_sight(pkt).await;

    assert_eq!(
        session.represented_seer_guid_like_cpp(),
        Some(original_seer)
    );
    assert_eq!(session.last_visibility_pos, None);
}
