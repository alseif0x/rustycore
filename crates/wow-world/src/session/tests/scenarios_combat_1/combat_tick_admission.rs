use super::*;

#[test]
fn combat_tick_uses_canonical_player_victim_when_session_target_is_empty_like_cpp() {
    let (mut session, _, _) = make_session();
    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    let guid = test_creature_guid(18_014);
    let player = ObjectGuid::create_player(1, 63);

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
        "Attacker".to_string(),
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
    session.combat_target = None;
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

    let manager = manager.read().unwrap();
    let world_creature = manager.find_creature(0, 0, guid).unwrap();
    assert!(world_creature.current_hp() < 40);
    assert_eq!(session.combat_target, Some(guid));
}

#[test]
fn combat_tick_uses_canonical_player_base_attack_timer_like_cpp() {
    let (mut session, _, _) = make_session();
    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    let guid = test_creature_guid(18_016);
    let player = ObjectGuid::create_player(1, 65);

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
        "Timer".to_string(),
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
            player
                .unit_mut()
                .set_base_attack_time_like_cpp(WeaponAttackType::BaseAttack, 2_000);
            player
                .unit_mut()
                .set_weapon_damage(WeaponAttackType::BaseAttack, 7.0, 7.0);
            let mut effective = *player.effective_combat_stats_like_cpp();
            effective.weapon_damage[WeaponAttackType::BaseAttack as usize] = [11.0, 11.0];
            player.replace_effective_combat_stats_like_cpp(effective);
        })
        .unwrap();
    session.combat_target = None;
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
    let after_first = manager
        .read()
        .unwrap()
        .find_creature(0, 0, guid)
        .unwrap()
        .current_hp();
    assert_eq!(
        after_first, 29,
        "melee consumes the canonical Player effective weapon range instead of Unit's stale mirror"
    );

    session.tick_combat_sync();
    let after_second = manager
        .read()
        .unwrap()
        .find_creature(0, 0, guid)
        .unwrap()
        .current_hp();
    assert_eq!(after_second, after_first);
}

#[test]
fn combat_tick_los_failure_resets_timer_without_damage_like_cpp() {
    let (mut session, _, _) = make_session();
    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    let guid = test_creature_guid(18_022);
    let player = ObjectGuid::create_player(1, 71);

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
        "Los".to_string(),
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
    register_test_creature(&mut session, manager.clone(), guid, 40);
    session
        .mutate_world_creature(guid, |creature| {
            creature.enter_combat(player);
            creature.creature.ai_ownership_mut().last_swing_ms = 0;
            creature.creature.ai_ownership_mut().swing_timer_ms = 0;
        })
        .unwrap();

    // Drive the swing with line of sight denied. The retired
    // `player_melee_los_to_target_like_cpp` field was the only writer of this
    // value and had no production writer at all, so #28 made it a parameter of
    // the lifted function; this is the branch the field existed to reach.
    let swings = session
        .mutate_canonical_player_like_cpp(|player| {
            take_canonical_player_attack_swings_like_cpp(
                player,
                0,
                true,
                true,
                false,
                [crate::session::RepresentedMeleeDamageBonusLikeCpp::NONE; 2],
                crate::session::combat::RepresentedArmorMitigationLikeCpp::NONE,
                Default::default(),
                crate::session_rules::RepresentedMeleeDamageTakenLikeCpp::NONE,
            )
        })
        .flatten();
    assert!(
        swings.is_none_or(|(swings, _)| swings.is_empty()),
        "a swing with no line of sight must produce no damage"
    );

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
fn combat_tick_canonical_player_without_victim_does_not_use_stale_session_target_like_cpp() {
    let (mut session, _, _) = make_session();
    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    let guid = test_creature_guid(18_028);
    let player = ObjectGuid::create_player(1, 77);

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
        "NoVictim".to_string(),
        Position::new(10.0, 10.0, 0.0, 0.0),
        0,
        1,
        1,
        80,
        0,
    ));
    let _ = session.ensure_canonical_world_map_for_current_player_like_cpp();
    session.combat_target = Some(guid);
    session.in_combat = true;
    register_test_creature(&mut session, manager.clone(), guid, 40);

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
    assert_eq!(session.resolved_combat_target_like_cpp(), Some(None));
    assert_eq!(session.resolved_in_combat_like_cpp(), Some(false));
}

#[test]
fn combat_tick_without_melee_attacking_state_skips_update_like_cpp() {
    let (mut session, _, _) = make_session();
    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    let guid = test_creature_guid(18_029);
    let player = ObjectGuid::create_player(1, 78);

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
        "NotMelee".to_string(),
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
