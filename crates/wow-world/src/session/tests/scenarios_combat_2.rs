//! Session scenarios exercising the represented combat responsibility.
//!
//! Split out of session_tests.rs under #626; assertions and registrations
//! are unchanged and the shared fixtures stay in the parent module.

use super::*;

#[test]
fn combat_tick_charging_player_skips_melee_update_without_reset_like_cpp() {
    let (mut session, _, _) = make_session();
    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    let guid = test_creature_guid(18_027);
    let player = ObjectGuid::create_player(1, 76);

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
        player,
        "Charge".to_string(),
        Position::new(10.0, 10.0, 0.0, 0.0),
        0,
        1,
        1,
        80,
        0,
    ));
    let _ = session.ensure_canonical_world_map_for_current_player_like_cpp();
    session
        .mutate_canonical_player_like_cpp(|player| {
            let unit = player.unit_mut();
            unit.set_attacking(Some(guid));
            unit.set_target(guid);
            unit.add_unit_state(UnitState::MELEE_ATTACKING.bits());
            unit.add_unit_state(UnitState::CHARGING.bits());
            unit.set_base_attack_time_like_cpp(WeaponAttackType::BaseAttack, 2_000);
            unit.set_weapon_damage(WeaponAttackType::BaseAttack, 7.0, 7.0);
        })
        .unwrap();
    session.combat_target = Some(guid);
    session.in_combat = true;
    register_test_creature(&mut session, manager.clone(), guid, 40);
    session
        .mutate_world_creature(guid, |creature| {
            creature.enter_combat(player);
            creature.creature.ai_ownership_mut().last_swing_ms = 0;
            creature.creature.ai_ownership_mut().swing_timer_ms = 0;
        })
        .unwrap();

    session.tick_combat_sync();

    assert_eq!(
        manager
            .read()
            .unwrap()
            .find_creature(0, 0, guid)
            .unwrap()
            .current_hp(),
        40
    );
    let guard = canonical.lock().unwrap();
    let player_entity = guard
        .find_map(0, 0)
        .unwrap()
        .map()
        .get_typed_player(player)
        .unwrap();
    assert_eq!(
        player_entity
            .unit()
            .attack_timer(WeaponAttackType::BaseAttack),
        0
    );
}
#[test]
fn combat_tick_pacified_player_resets_timer_without_damage_like_cpp() {
    let (mut session, _, _) = make_session();
    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    let guid = test_creature_guid(18_020);
    let player = ObjectGuid::create_player(1, 69);

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
        player,
        "Pacified".to_string(),
        Position::new(10.0, 10.0, 0.0, 0.0),
        0,
        1,
        1,
        80,
        0,
    ));
    let _ = session.ensure_canonical_world_map_for_current_player_like_cpp();
    session
        .mutate_canonical_player_like_cpp(|player| {
            let unit = player.unit_mut();
            unit.set_attacking(Some(guid));
            unit.set_target(guid);
            unit.add_unit_state(UnitState::MELEE_ATTACKING.bits());
            unit.set_unit_flags_like_cpp(UnitFlags::PLAYER_CONTROLLED | UnitFlags::PACIFIED);
            unit.set_base_attack_time_like_cpp(WeaponAttackType::BaseAttack, 2_000);
            unit.set_weapon_damage(WeaponAttackType::BaseAttack, 7.0, 7.0);
        })
        .unwrap();
    session.combat_target = Some(guid);
    session.in_combat = true;
    register_test_creature(&mut session, manager.clone(), guid, 40);
    session
        .mutate_world_creature(guid, |creature| {
            creature.enter_combat(player);
            creature.creature.ai_ownership_mut().last_swing_ms = 0;
            creature.creature.ai_ownership_mut().swing_timer_ms = 0;
        })
        .unwrap();

    session.tick_combat_sync();

    let hp = manager
        .read()
        .unwrap()
        .find_creature(0, 0, guid)
        .unwrap()
        .current_hp();
    assert_eq!(hp, 40);
    let guard = canonical.lock().unwrap();
    let player_entity = guard
        .find_map(0, 0)
        .unwrap()
        .map()
        .get_typed_player(player)
        .unwrap();
    assert_eq!(
        player_entity
            .unit()
            .attack_timer(WeaponAttackType::BaseAttack),
        2_000
    );
}
#[test]
fn combat_tick_uses_canonical_player_offhand_timer_like_cpp() {
    let (mut session, _, _) = make_session();
    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    let guid = test_creature_guid(18_019);
    let player = ObjectGuid::create_player(1, 68);

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
        player,
        "Dual".to_string(),
        Position::new(10.0, 10.0, 0.0, 0.0),
        0,
        1,
        1,
        80,
        0,
    ));
    let _ = session.ensure_canonical_world_map_for_current_player_like_cpp();
    session
        .mutate_canonical_player_like_cpp(|player| {
            let unit = player.unit_mut();
            unit.set_attacking(Some(guid));
            unit.set_target(guid);
            unit.add_unit_state(UnitState::MELEE_ATTACKING.bits());
            unit.set_can_dual_wield_like_cpp(true);
            unit.set_base_attack_time_like_cpp(WeaponAttackType::BaseAttack, 2_000);
            unit.set_base_attack_time_like_cpp(WeaponAttackType::OffAttack, 2_000);
            unit.set_attack_timer(WeaponAttackType::BaseAttack, 500);
            unit.set_attack_timer(WeaponAttackType::OffAttack, 0);
            unit.set_weapon_damage(WeaponAttackType::OffAttack, 4.0, 4.0);
        })
        .unwrap();
    session.combat_target = Some(guid);
    session.in_combat = true;
    register_test_creature(&mut session, manager.clone(), guid, 40);
    session
        .mutate_world_creature(guid, |creature| {
            creature.enter_combat(player);
            creature.creature.ai_ownership_mut().last_swing_ms = 0;
            creature.creature.ai_ownership_mut().swing_timer_ms = 0;
        })
        .unwrap();

    session.tick_combat_sync();

    let hp = manager
        .read()
        .unwrap()
        .find_creature(0, 0, guid)
        .unwrap()
        .current_hp();
    assert_eq!(hp, 36);
    let guard = canonical.lock().unwrap();
    let player_entity = guard
        .find_map(0, 0)
        .unwrap()
        .map()
        .get_typed_player(player)
        .unwrap();
    assert_eq!(
        player_entity
            .unit()
            .attack_timer(WeaponAttackType::OffAttack),
        2_000
    );
}
#[test]
fn combat_tick_out_of_range_sets_short_retry_timer_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    let guid = test_creature_guid(18_017);
    let player = ObjectGuid::create_player(1, 66);

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
        player,
        "Far".to_string(),
        Position::new(100.0, 100.0, 0.0, 0.0),
        0,
        1,
        1,
        80,
        0,
    ));
    let _ = session.ensure_canonical_world_map_for_current_player_like_cpp();
    session
        .mutate_canonical_player_like_cpp(|player| {
            player.unit_mut().set_attacking(Some(guid));
            player.unit_mut().set_target(guid);
            player
                .unit_mut()
                .add_unit_state(UnitState::MELEE_ATTACKING.bits());
        })
        .unwrap();
    session.combat_target = Some(guid);
    session.in_combat = true;
    register_test_creature(&mut session, manager.clone(), guid, 40);
    session
        .mutate_world_creature(guid, |creature| {
            creature.enter_combat(player);
            creature.creature.ai_ownership_mut().last_swing_ms = 0;
            creature.creature.ai_ownership_mut().swing_timer_ms = 0;
        })
        .unwrap();

    session.tick_combat_sync();

    let hp = manager
        .read()
        .unwrap()
        .find_creature(0, 0, guid)
        .unwrap()
        .current_hp();
    assert_eq!(hp, 40);
    let guard = canonical.lock().unwrap();
    let player_entity = guard
        .find_map(0, 0)
        .unwrap()
        .map()
        .get_typed_player(player)
        .unwrap();
    assert_eq!(
        player_entity
            .unit()
            .attack_timer(WeaponAttackType::BaseAttack),
        100
    );
    let sent = send_rx.try_recv().unwrap();
    let opcode = u16::from_le_bytes([sent[0], sent[1]]);
    assert_eq!(opcode, ServerOpcodes::AttackSwingError as u16);
    let mut pkt = WorldPacket::from_bytes(&sent[2..]);
    assert_eq!(pkt.read_bits(3).unwrap(), 0);
}
#[test]
fn combat_tick_bad_facing_sets_short_retry_timer_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    let guid = test_creature_guid(18_018);
    let player = ObjectGuid::create_player(1, 67);

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
        player,
        "Facing".to_string(),
        Position::new(10.0, 10.0, 0.0, std::f32::consts::PI),
        0,
        1,
        1,
        80,
        0,
    ));
    let _ = session.ensure_canonical_world_map_for_current_player_like_cpp();
    session
        .mutate_canonical_player_like_cpp(|player| {
            player.unit_mut().set_attacking(Some(guid));
            player.unit_mut().set_target(guid);
            player
                .unit_mut()
                .add_unit_state(UnitState::MELEE_ATTACKING.bits());
        })
        .unwrap();
    session.combat_target = Some(guid);
    session.in_combat = true;
    register_test_creature(&mut session, manager.clone(), guid, 40);
    session
        .mutate_world_creature(guid, |creature| {
            creature
                .creature
                .set_ai_position(Position::new(12.0, 10.0, 0.0, 0.0));
            creature.creature.unit_mut().set_bounding_radius(0.0);
            creature.creature.unit_mut().set_combat_reach(0.0);
            creature.enter_combat(player);
            creature.creature.ai_ownership_mut().last_swing_ms = 0;
            creature.creature.ai_ownership_mut().swing_timer_ms = 0;
        })
        .unwrap();

    session.tick_combat_sync();

    let hp = manager
        .read()
        .unwrap()
        .find_creature(0, 0, guid)
        .unwrap()
        .current_hp();
    assert_eq!(hp, 40);
    let guard = canonical.lock().unwrap();
    let player_entity = guard
        .find_map(0, 0)
        .unwrap()
        .map()
        .get_typed_player(player)
        .unwrap();
    assert_eq!(
        player_entity
            .unit()
            .attack_timer(WeaponAttackType::BaseAttack),
        100
    );
    let sent = send_rx.try_recv().unwrap();
    let opcode = u16::from_le_bytes([sent[0], sent[1]]);
    assert_eq!(opcode, ServerOpcodes::AttackSwingError as u16);
    let mut pkt = WorldPacket::from_bytes(&sent[2..]);
    assert_eq!(pkt.read_bits(3).unwrap(), 1);
}
#[test]
fn combat_tick_melee_range_uses_combat_reach_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    let guid = test_creature_guid(18_019);
    let player = ObjectGuid::create_player(1, 68);

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
        player,
        "Reach".to_string(),
        Position::new(10.0, 10.0, 0.0, 0.0),
        0,
        1,
        1,
        80,
        0,
    ));
    let _ = session.ensure_canonical_world_map_for_current_player_like_cpp();
    session
        .mutate_canonical_player_like_cpp(|player| {
            player.unit_mut().set_attacking(Some(guid));
            player.unit_mut().set_target(guid);
            player
                .unit_mut()
                .add_unit_state(UnitState::MELEE_ATTACKING.bits());
        })
        .unwrap();
    session.combat_target = Some(guid);
    session.in_combat = true;
    register_test_creature(&mut session, manager.clone(), guid, 40);
    session
        .mutate_world_creature(guid, |creature| {
            creature
                .creature
                .set_ai_position(Position::new(17.0, 10.0, 0.0, 0.0));
            creature.creature.unit_mut().set_combat_reach(6.0);
            creature.enter_combat(player);
            creature.creature.ai_ownership_mut().last_swing_ms = 0;
            creature.creature.ai_ownership_mut().swing_timer_ms = 0;
        })
        .unwrap();

    session.tick_combat_sync();

    let hp = manager
        .read()
        .unwrap()
        .find_creature(0, 0, guid)
        .unwrap()
        .current_hp();
    assert!(hp < 40);
    let sent = send_rx.try_recv().unwrap();
    let opcode = u16::from_le_bytes([sent[0], sent[1]]);
    assert_eq!(opcode, ServerOpcodes::AttackerStateUpdate as u16);
}
#[test]
fn combat_tick_boundary_radius_suppresses_bad_facing_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    let guid = test_creature_guid(18_020);
    let player = ObjectGuid::create_player(1, 69);

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
        player,
        "Boundary".to_string(),
        Position::new(10.0, 10.0, 0.0, std::f32::consts::PI),
        0,
        1,
        1,
        80,
        0,
    ));
    let _ = session.ensure_canonical_world_map_for_current_player_like_cpp();
    session
        .mutate_canonical_player_like_cpp(|player| {
            player.unit_mut().set_attacking(Some(guid));
            player.unit_mut().set_target(guid);
            player
                .unit_mut()
                .add_unit_state(UnitState::MELEE_ATTACKING.bits());
        })
        .unwrap();
    session.combat_target = Some(guid);
    session.in_combat = true;
    register_test_creature(&mut session, manager.clone(), guid, 40);
    session
        .mutate_world_creature(guid, |creature| {
            creature
                .creature
                .set_ai_position(Position::new(12.0, 10.0, 0.0, 0.0));
            creature.creature.unit_mut().set_bounding_radius(3.0);
            creature.enter_combat(player);
            creature.creature.ai_ownership_mut().last_swing_ms = 0;
            creature.creature.ai_ownership_mut().swing_timer_ms = 0;
        })
        .unwrap();

    session.tick_combat_sync();

    let hp = manager
        .read()
        .unwrap()
        .find_creature(0, 0, guid)
        .unwrap()
        .current_hp();
    assert!(hp < 40);
    let sent = send_rx.try_recv().unwrap();
    let opcode = u16::from_le_bytes([sent[0], sent[1]]);
    assert_eq!(opcode, ServerOpcodes::AttackerStateUpdate as u16);
}
#[test]
fn combat_tick_clears_canonical_player_attack_when_target_dies_like_cpp() {
    let (mut session, _, _) = make_session();
    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    let guid = test_creature_guid(18_015);
    let player = ObjectGuid::create_player(1, 64);

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
        player,
        "Killer".to_string(),
        Position::new(10.0, 10.0, 0.0, 0.0),
        0,
        1,
        1,
        80,
        0,
    ));
    let _ = session.ensure_canonical_world_map_for_current_player_like_cpp();
    session
        .mutate_canonical_player_like_cpp(|player| {
            player.unit_mut().set_attacking(Some(guid));
            player.unit_mut().set_target(guid);
            player
                .unit_mut()
                .add_unit_state(UnitState::MELEE_ATTACKING.bits());
        })
        .unwrap();
    session.combat_target = Some(guid);
    session.in_combat = true;
    register_test_creature(&mut session, manager.clone(), guid, 1);
    session
        .mutate_world_creature(guid, |creature| {
            creature.enter_combat(player);
            creature.creature.unit_mut().add_attacker_like_cpp(player);
            creature.creature.ai_ownership_mut().last_swing_ms = 0;
            creature.creature.ai_ownership_mut().swing_timer_ms = 0;
        })
        .unwrap();
    assert!(session.mirror_canonical_creature_threat_from_attacker_like_cpp(guid, player, 1.0));

    session.tick_combat_sync();

    {
        let manager = manager.read().unwrap();
        let world_creature = manager.find_creature(0, 0, guid).unwrap();
        assert_eq!(world_creature.current_hp(), 0);
        assert!(
            world_creature
                .creature
                .unit()
                .subsystems()
                .combat
                .threat
                .is_empty()
        );
        assert!(
            world_creature
                .creature
                .unit()
                .subsystems()
                .combat
                .attackers
                .is_empty()
        );
    }

    let guard = canonical.lock().unwrap();
    let player_entity = guard
        .find_map(0, 0)
        .unwrap()
        .map()
        .get_typed_player(player)
        .unwrap();
    assert_eq!(player_entity.unit().attacking(), None);
    assert_eq!(player_entity.unit().data().target, ObjectGuid::EMPTY);
    assert!(
        !player_entity
            .unit()
            .subsystems()
            .combat
            .is_in_combat_with(guid)
    );
    assert!(
        !player_entity
            .unit()
            .subsystems()
            .combat
            .is_threatening_to(guid, true)
    );
    let creature_entity = guard
        .find_map(0, 0)
        .unwrap()
        .map()
        .with_creature_like_cpp(guid, Clone::clone)
        .unwrap();
    assert!(
        !creature_entity
            .unit()
            .subsystems()
            .combat
            .is_in_combat_with(player)
    );
    assert_eq!(
        creature_entity
            .unit()
            .subsystems()
            .combat
            .threat_value(player),
        None
    );
    drop(guard);
    assert_eq!(session.resolved_combat_target_like_cpp(), Some(None));
    assert_eq!(session.resolved_in_combat_like_cpp(), Some(false));
}
#[test]
fn combat_tick_reports_cpp_like_over_damage_on_killing_swing() {
    let (mut session, _, send_rx) = make_session();
    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    let guid = test_creature_guid(18_025);
    let player = ObjectGuid::create_player(1, 74);

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
        player,
        "Overkill".to_string(),
        Position::new(10.0, 10.0, 0.0, 0.0),
        0,
        1,
        1,
        80,
        0,
    ));
    let _ = session.ensure_canonical_world_map_for_current_player_like_cpp();
    session
        .mutate_canonical_player_like_cpp(|player| {
            let unit = player.unit_mut();
            unit.set_attacking(Some(guid));
            unit.set_target(guid);
            unit.add_unit_state(UnitState::MELEE_ATTACKING.bits());
            unit.set_base_attack_time_like_cpp(WeaponAttackType::BaseAttack, 2_000);
            unit.set_weapon_damage(WeaponAttackType::BaseAttack, 7.0, 7.0);
        })
        .unwrap();
    session.combat_target = Some(guid);
    session.in_combat = true;
    register_test_creature(&mut session, manager.clone(), guid, 3);
    session
        .mutate_world_creature(guid, |creature| {
            creature.enter_combat(player);
            creature.creature.ai_ownership_mut().last_swing_ms = 0;
            creature.creature.ai_ownership_mut().swing_timer_ms = 0;
        })
        .unwrap();

    session.tick_combat_sync();

    let sent = send_rx.try_recv().unwrap();
    let opcode = u16::from_le_bytes([sent[0], sent[1]]);
    assert_eq!(opcode, ServerOpcodes::AttackerStateUpdate as u16);
    let mut outer = WorldPacket::from_bytes(&sent[2..]);
    assert!(!outer.read_bit().unwrap());
    let size = outer.read_uint32().unwrap() as usize;
    let data = outer.read_bytes(size).unwrap();
    let mut info = WorldPacket::from_bytes(&data);
    assert_eq!(info.read_uint32().unwrap(), 0x0000_0002);
    assert_eq!(info.read_packed_guid().unwrap(), player);
    assert_eq!(info.read_packed_guid().unwrap(), guid);
    assert_eq!(info.read_int32().unwrap(), 7);
    assert_eq!(info.read_int32().unwrap(), 7);
    assert_eq!(info.read_int32().unwrap(), 4);
}
#[test]
fn player_attack_start_stop_updates_canonical_unit_combat_state_like_cpp() {
    let (mut session, _, _) = make_session();
    let canonical = shared_canonical_map_manager();
    let player = ObjectGuid::create_player(1, 45);
    let victim = test_creature_guid(18_006);

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
        player,
        "Warrior".to_string(),
        Position::new(10.0, 20.0, 30.0, 0.0),
        571,
        1,
        1,
        80,
        0,
    ));

    session.start_player_attack_like_cpp(victim);
    {
        let guard = canonical.lock().unwrap();
        let player_entity = guard
            .find_map(571, 0)
            .unwrap()
            .map()
            .get_typed_player(player)
            .expect("player stored as canonical typed Player");
        assert_eq!(player_entity.unit().attacking(), Some(victim));
        assert_eq!(player_entity.unit().data().target, victim);
    }

    assert_eq!(session.stop_player_attack_like_cpp(), Some(victim));
    let guard = canonical.lock().unwrap();
    let player_entity = guard
        .find_map(571, 0)
        .unwrap()
        .map()
        .get_typed_player(player)
        .expect("player remains canonical typed Player");
    assert_eq!(player_entity.unit().attacking(), None);
    assert_eq!(player_entity.unit().data().target, ObjectGuid::EMPTY);
}
#[test]
fn runtime_damage_zero_health_marks_canonical_player_dead_like_cpp() {
    let (mut session, _, _) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 0xE115);
    session.set_canonical_map_manager(Arc::clone(&canonical));

    session.ensure_login_player_controller_like_cpp(
        player_guid,
        "RuntimeDead".to_string(),
        Position::new(1.0, 2.0, 3.0, 0.0),
        1,
        1,
        1,
        80,
        0,
    );
    insert_session_player_into_canonical_map_like_cpp(&session, &canonical, 1, 0);
    let _ = session.sync_canonical_player_health_like_cpp(42, 120);

    session.set_player_health_after_runtime_damage_like_cpp(0);

    assert_eq!(session.player_health_like_cpp(), 0);
    assert_eq!(
        session.canonical_player_health_snapshot_like_cpp(),
        Some((0, 120))
    );
    assert_eq!(
        session.canonical_player_snapshot_like_cpp(|player| player.unit().death_state()),
        Some(wow_constants::DeathState::Corpse)
    );
}
#[test]
fn set_player_skill_values_builds_represented_skill_records_for_tests_like_cpp() {
    let (mut session, _, _) = make_session();

    assert!(!session.player_skill_records_loaded_like_cpp());
    session.set_player_skill_values_like_cpp(HashMap::from([(762, 75), (333, 150)]));

    assert!(session.player_skill_records_loaded_like_cpp());
    assert!(
        session.complete_player_skill_records_like_cpp().is_none(),
        "an active-only value map has no authority for C++ step or SkillUpdateState"
    );
    let skill_records = session.player_skill_records_like_cpp();
    let riding = skill_records.get(&762).unwrap();
    assert_eq!(
        *riding,
        RepresentedPlayerSkillLikeCpp {
            skill_id: 762,
            step: 0,
            value: 75,
            max: 75,
            profession_slot: -1,
            state: RepresentedPlayerSkillStateLikeCpp::Unchanged,
        }
    );
    assert_eq!(session.player_skill_value_like_cpp(333), 150);
}
#[test]
fn authoritative_skill_mutations_preserve_exact_slot_occupancy_like_cpp() {
    let (mut session, _, _) = make_session();
    assert!(session.set_complete_player_skill_records_like_cpp(
        HashMap::from([(
            333,
            RepresentedPlayerSkillLikeCpp {
                skill_id: 333,
                step: 1,
                value: 75,
                max: 150,
                profession_slot: 0,
                state: RepresentedPlayerSkillStateLikeCpp::Unchanged,
            },
        )]),
        1,
    ));

    session.set_represented_player_skill_like_cpp(333, 2, 150, 225);
    assert_eq!(
        session.complete_player_skill_occupied_slots_like_cpp(),
        Some(1)
    );
    session.set_represented_player_skill_like_cpp(164, 1, 1, 75);
    assert_eq!(
        session.complete_player_skill_occupied_slots_like_cpp(),
        Some(2)
    );
}
