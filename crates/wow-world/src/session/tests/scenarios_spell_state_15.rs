//! Session scenarios exercising the represented spell state responsibility.
//!
//! Split out of session_tests.rs under #626; assertions and registrations
//! are unchanged and the shared fixtures stay in the parent module.

use super::*;

#[tokio::test]
async fn teleport_to_far_map_removes_moving_and_turning_interrupt_auras_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 807);
    let source_position = Position::new(10.0, 20.0, 30.0, 0.0);
    let destination = Position::new(112.0, 212.0, 42.0, 2.7);
    let represented_moving_slot = 11;
    let represented_turning_slot = 12;
    let represented_kept_slot = 13;
    let canonical_moving = wow_entities::AppliedAuraRef::new(62_807, player_guid, 0, 0x1);
    let canonical_turning = wow_entities::AppliedAuraRef::new(62_808, player_guid, 1, 0x1);
    let canonical_kept = wow_entities::AppliedAuraRef::new(62_809, player_guid, 2, 0x1);

    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
        wow_data::MapEntry {
            id: 571,
            instance_type: wow_data::map::MAP_COMMON,
            expansion_id: 1,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
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
    session.expansion = 1;
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "TeleportRemoveInterruptAuras".to_string(),
        source_position,
        571,
        1,
        1,
        80,
        0,
    ));
    for (slot, spell_id, flags) in [
        (
            represented_moving_slot,
            62_817,
            SPELL_AURA_INTERRUPT_FLAG_MOVING_LIKE_CPP,
        ),
        (
            represented_turning_slot,
            62_818,
            SPELL_AURA_INTERRUPT_FLAG_TURNING_LIKE_CPP,
        ),
        (represented_kept_slot, 62_819, 0),
    ] {
        session.visible_auras.insert(
            slot,
            AuraApplication {
                spell_id,
                difficulty_id: 0,
                caster_guid: player_guid,
                slot,
                duration_total: 30_000,
                duration_remaining: 30_000,
                stack_count: 1,
                aura_flags: 0x1,
                effect_mask: 0x1,
                aura_interrupt_flags: flags,
                aura_interrupt_flags2: 0,
                represented_effect: None,
                represented_amount: 0,
                represented_effect_amounts: Vec::new(),
                represented_misc_value: None,
                represented_multiplier: 1.0,
                applied_at: Instant::now(),
            },
        );
    }
    add_canonical_test_player_on_map(&canonical, player_guid, source_position, 571, 0);
    session.mutate_canonical_player_like_cpp(|player| {
        let auras = &mut player.unit_mut().subsystems_mut().auras;
        auras.register_applied_aura(
            canonical_moving,
            None,
            SPELL_AURA_INTERRUPT_FLAG_MOVING_LIKE_CPP,
            0,
        );
        auras.register_applied_aura(
            canonical_turning,
            None,
            SPELL_AURA_INTERRUPT_FLAG_TURNING_LIKE_CPP,
            0,
        );
        auras.register_applied_aura(canonical_kept, None, 0, 0);
    });

    session.teleport_to(0, destination).await;

    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![
            ServerOpcodes::CancelCombat,
            ServerOpcodes::AuraUpdate,
            ServerOpcodes::AuraUpdate,
            ServerOpcodes::TransferPending,
            ServerOpcodes::SuspendToken
        ]
    );
    assert_eq!(session.pending_teleport_like_cpp(), Some((0, destination)));
    assert!(!session.visible_auras.contains_key(&represented_moving_slot));
    assert!(
        !session
            .visible_auras
            .contains_key(&represented_turning_slot)
    );
    assert!(session.visible_auras.contains_key(&represented_kept_slot));
    session.mutate_canonical_player_like_cpp(|player| {
        let auras = &player.unit().subsystems().auras;
        assert!(
            !auras.has_applied(canonical_moving),
            "C++ RemoveAurasWithInterruptFlags(Moving|Turning) removes Moving auras before transfer"
        );
        assert!(
            !auras.has_applied(canonical_turning),
            "C++ RemoveAurasWithInterruptFlags(Moving|Turning) removes Turning auras before transfer"
        );
        assert!(auras.has_applied(canonical_kept));
    });
}
#[tokio::test]
async fn teleport_to_allows_death_knight_after_escape_spell_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 797);
    let destination = Position::new(105.0, 205.0, 35.0, 2.0);
    session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
        wow_data::MapEntry {
            id: 571,
            instance_type: wow_data::map::MAP_COMMON,
            expansion_id: 1,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
    ])));
    session.expansion = 1;
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "TeleportDkAllow".to_string(),
        Position::new(10.0, 20.0, 30.0, 0.0),
        DEATH_KNIGHT_START_MAP_LIKE_CPP,
        1,
        CLASS_DEATH_KNIGHT_LIKE_CPP,
        58,
        0,
    ));
    session.learn_known_spell_like_cpp(DEATH_KNIGHT_ESCAPE_SPELL_LIKE_CPP);

    session.teleport_to(571, destination).await;

    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![
            ServerOpcodes::CancelCombat,
            ServerOpcodes::TransferPending,
            ServerOpcodes::SuspendToken
        ]
    );
    assert_eq!(
        session.pending_teleport_like_cpp(),
        Some((571, destination))
    );
    assert_eq!(session.state, SessionState::Transfer);
}
#[tokio::test]
async fn spell_stuck_kills_player_when_hearthstone_has_cooldown_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let spell_id = 784_i32;
    let player_guid = ObjectGuid::create_player(1, 784);
    let position = Position::new(11.0, 21.0, 31.0, 1.0);
    let canonical = shared_canonical_map_manager();
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "StuckCooldown".to_string(),
        position,
        571,
        1,
        1,
        80,
        0,
    ));
    session.set_player_health_like_cpp(100, 100);
    add_canonical_test_player_on_map(&canonical, player_guid, position, 571, 0);
    session.mutate_cast_execution_like_cpp(|state| {
        state.last_cast_time_per_spell.insert(8690, Instant::now());
    });
    set_stuck_spell_store_like_cpp(&mut session, spell_id);

    session
        .execute_spell(spell_id, player_guid)
        .await
        .expect("represented stuck spell cooldown branch should execute");

    assert!(!session.player_is_alive_like_cpp());
    assert_eq!(session.player_health_like_cpp(), 0);
    assert_eq!(
        session
            .mutate_canonical_player_like_cpp(|player| player.unit().data().health)
            .unwrap(),
        0
    );
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::SpellGo, ServerOpcodes::CooldownEvent]
    );
}
#[tokio::test]
async fn spell_stuck_dead_player_without_death_timer_repops_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let spell_id = 785_i32;
    let player_guid = ObjectGuid::create_player(1, 785);
    let position = Position::new(12.0, 22.0, 32.0, 1.0);
    let canonical = shared_canonical_map_manager();
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "StuckDead".to_string(),
        position,
        571,
        1,
        1,
        80,
        0,
    ));
    session.set_player_health_like_cpp(0, 100);
    add_canonical_test_player_on_map(&canonical, player_guid, position, 571, 0);
    set_stuck_spell_store_like_cpp(&mut session, spell_id);

    session
        .execute_spell(spell_id, player_guid)
        .await
        .expect("represented stuck spell dead branch should execute");

    assert_eq!(session.represented_repop_at_graveyard_count, 1);
    assert!(session.player_has_ghost_flag_like_cpp());
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::SpellGo, ServerOpcodes::CooldownEvent]
    );
}
#[tokio::test]
async fn spell_stuck_skips_flight_and_disabled_config_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let spell_id = 786_i32;
    let player_guid = ObjectGuid::create_player(1, 786);
    let position = Position::new(13.0, 23.0, 33.0, 1.0);
    let home = Position::new(101.0, 201.0, 41.0, 2.0);
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "StuckFlight".to_string(),
        position,
        571,
        1,
        1,
        80,
        0,
    ));
    session.set_player_health_like_cpp(100, 100);
    let _ = session.set_represented_homebind_like_cpp(RepresentedHomebindLikeCpp {
        map_id: 0,
        area_id: 12,
        position: home,
    });
    session.set_taxi_flight_state_like_cpp(
        RepresentedTaxiFlightNodeLikeCpp {
            map_id: 571,
            position,
            teleport_flag: false,
        },
        None,
    );
    set_stuck_spell_store_like_cpp(&mut session, spell_id);

    session
        .execute_spell(spell_id, player_guid)
        .await
        .expect("represented stuck spell in-flight branch should execute as no-op");

    assert_eq!(session.pending_teleport_like_cpp(), None);
    assert!(
        session
            .spell_last_cast_time_like_cpp(8690)
            .flatten()
            .is_none(),
        "C++ EffectStuck returns before Hearthstone cooldown while player is in flight"
    );
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::SpellGo, ServerOpcodes::CooldownEvent]
    );

    let (mut disabled_session, _, disabled_rx) = make_session();
    disabled_session.represented_cast_unstuck_enabled_like_cpp = false;
    disabled_session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "StuckDisabled".to_string(),
        position,
        571,
        1,
        1,
        80,
        0,
    ));
    disabled_session.set_player_health_like_cpp(100, 100);
    let _ = disabled_session.set_represented_homebind_like_cpp(RepresentedHomebindLikeCpp {
        map_id: 0,
        area_id: 12,
        position: home,
    });
    set_stuck_spell_store_like_cpp(&mut disabled_session, spell_id);

    disabled_session
        .execute_spell(spell_id, player_guid)
        .await
        .expect("disabled CastUnstuck should execute as C++ no-op");

    assert_eq!(disabled_session.pending_teleport_like_cpp(), None);
    assert!(
        disabled_session
            .spell_last_cast_time_like_cpp(8690)
            .flatten()
            .is_none()
    );
    assert_eq!(
        drain_server_opcodes(&disabled_rx),
        vec![ServerOpcodes::SpellGo, ServerOpcodes::CooldownEvent]
    );
}
#[tokio::test]
async fn spell_taunt_effect_current_victim_is_ineffective_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let spell_id = 794_i32;
    let player_guid = ObjectGuid::create_player(1, 794);
    let other_player_guid = ObjectGuid::create_player(1, 1794);
    let creature_guid = test_creature_guid(18_794);
    let position = Position::new(10.0, 10.0, 0.0, 0.0);
    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "TauntAlreadyVictim".to_string(),
        position,
        0,
        1,
        1,
        80,
        0,
    ));
    session.set_player_health_like_cpp(100, 100);
    add_canonical_test_player_on_map(&canonical, player_guid, position, 0, 0);
    add_canonical_test_creature_indexed_on_map_with_level(
        &canonical,
        creature_guid,
        9001,
        position,
        0,
        0,
        80,
    );
    register_test_creature(&mut session, manager.clone(), creature_guid, 100);
    session
        .mutate_world_creature(creature_guid, |creature| {
            let combat = &mut creature.creature.unit_mut().subsystems_mut().combat;
            combat.add_threat(player_guid, 5.0);
            combat.add_threat(other_player_guid, 120.0);
            combat.current_victim_guid = Some(player_guid);
        })
        .unwrap();
    session.sync_represented_creature_threat_to_canonical_like_cpp(creature_guid, player_guid, 5.0);
    let mut spell_store = wow_data::SpellStore::new();
    let mut taunt_spell = threat_spell_info_like_cpp(
        spell_id,
        wow_data::spell::spell_effect_types::SPELL_EFFECT_ATTACK_ME,
        0,
    );
    taunt_spell.effects.push(wow_data::SpellEffectInfo {
        effect_index: 1,
        effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
        effect_aura: wow_data::spell::aura_types::SPELL_AURA_MOD_TAUNT,
        ..Default::default()
    });
    spell_store.insert(spell_id, taunt_spell);
    session.set_spell_store(Arc::new(spell_store));
    session.set_spell_misc_store(Arc::new(wow_data::SpellMiscStore::from_entries([
        wow_data::SpellMiscEntry {
            id: spell_id as u32,
            duration_index: 7,
            spell_id: spell_id as u32,
            ..Default::default()
        },
    ])));
    session.set_spell_duration_store(Arc::new(wow_data::SpellDurationStore::from_entries([
        wow_data::SpellDurationEntry {
            id: 7,
            duration: 3_000,
            duration_per_level: 0,
            max_duration: 3_000,
        },
    ])));

    session
        .execute_spell(spell_id, creature_guid)
        .await
        .expect("represented EffectTaunt current-victim gate should execute as no-op");

    let legacy_threat = manager
        .read()
        .unwrap()
        .find_creature(0, 0, creature_guid)
        .unwrap()
        .creature
        .unit()
        .subsystems()
        .combat
        .threat_value(player_guid);
    assert_eq!(legacy_threat, Some(5.0));
    assert!(
        !manager
            .read()
            .unwrap()
            .find_creature(0, 0, creature_guid)
            .unwrap()
            .creature
            .unit()
            .subsystems()
            .combat
            .threat_ref(player_guid)
            .is_some_and(wow_entities::ThreatReferenceState::is_taunting),
        "C++ EffectTaunt rejects the already-current victim before its paired taunt aura applies"
    );
    assert_eq!(
        session.canonical_creature_threat_value_like_cpp(creature_guid, player_guid),
        Some(5.0)
    );
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::SpellGo, ServerOpcodes::CooldownEvent]
    );
}
#[tokio::test]
async fn spell_distract_effect_launches_facing_spline_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let spell_id = 795_i32;
    let player_guid = ObjectGuid::create_player(1, 795);
    let creature_guid = test_creature_guid(18_795);
    let position = Position::new(10.0, 10.0, 0.0, 0.0);
    let destination = Position::new(10.0, 20.0, 0.0, 0.0);
    let manager = shared_map_manager();
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "DistractCaster".to_string(),
        position,
        0,
        1,
        1,
        80,
        0,
    ));
    session.set_player_health_like_cpp(100, 100);
    register_test_creature(&mut session, manager.clone(), creature_guid, 100);
    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        spell_id,
        threat_spell_info_like_cpp(
            spell_id,
            wow_data::spell::spell_effect_types::SPELL_EFFECT_DISTRACT,
            5,
        ),
    );
    session.set_spell_store(Arc::new(spell_store));
    let target_data = SpellTargetData {
        flags: 0x2,
        unit: creature_guid,
        item: ObjectGuid::EMPTY,
        dst_location: Some(wow_packet::packets::spell::TargetLocation {
            transport: ObjectGuid::EMPTY,
            position: destination,
        }),
        ..SpellTargetData::default()
    };

    session
        .execute_spell_with_target_data(spell_id, creature_guid, target_data)
        .await
        .expect("represented EffectDistract should execute");

    let guard = manager.read().unwrap();
    let creature = guard.find_creature(0, 0, creature_guid).unwrap();
    let generator = creature
        .creature
        .unit()
        .subsystems()
        .motion
        .current_movement_generator();
    assert_eq!(
        generator.kind,
        wow_entities::MovementGeneratorKind::Distract
    );
    assert_eq!(generator.base_unit_state, UnitState::DISTRACTED.bits());
    assert_eq!(generator.duration_ms, Some(5_000));
    let spline = creature
        .active_move_spline_like_cpp()
        .expect("distract launches a facing spline");
    assert_eq!(
        spline.facing().kind,
        wow_movement::MonsterMoveType::FacingAngle
    );
    assert!((spline.facing().angle - std::f32::consts::FRAC_PI_2).abs() < 0.0001);
    drop(guard);
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![
            ServerOpcodes::SpellGo,
            ServerOpcodes::OnMonsterMove,
            ServerOpcodes::CooldownEvent,
        ]
    );
}
#[tokio::test]
async fn spell_distract_effect_skips_engaged_target_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let spell_id = 796_i32;
    let player_guid = ObjectGuid::create_player(1, 796);
    let creature_guid = test_creature_guid(18_796);
    let position = Position::new(10.0, 10.0, 0.0, 0.0);
    let destination = Position::new(10.0, 20.0, 0.0, 0.0);
    let manager = shared_map_manager();
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "DistractEngaged".to_string(),
        position,
        0,
        1,
        1,
        80,
        0,
    ));
    session.set_player_health_like_cpp(100, 100);
    register_test_creature(&mut session, manager.clone(), creature_guid, 100);
    session
        .mutate_world_creature(creature_guid, |creature| {
            creature.creature.ai_ownership_mut().combat_target = Some(player_guid);
        })
        .unwrap();
    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        spell_id,
        threat_spell_info_like_cpp(
            spell_id,
            wow_data::spell::spell_effect_types::SPELL_EFFECT_DISTRACT,
            5,
        ),
    );
    session.set_spell_store(Arc::new(spell_store));
    let target_data = SpellTargetData {
        flags: 0x2,
        unit: creature_guid,
        item: ObjectGuid::EMPTY,
        dst_location: Some(wow_packet::packets::spell::TargetLocation {
            transport: ObjectGuid::EMPTY,
            position: destination,
        }),
        ..SpellTargetData::default()
    };

    session
        .execute_spell_with_target_data(spell_id, creature_guid, target_data)
        .await
        .expect("represented EffectDistract engaged target gate should execute as no-op");

    let guard = manager.read().unwrap();
    let creature = guard.find_creature(0, 0, creature_guid).unwrap();
    assert!(
        !creature
            .creature
            .unit()
            .has_unit_state(UnitState::DISTRACTED.bits())
    );
    assert!(creature.active_move_spline_like_cpp().is_none());
    drop(guard);
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::SpellGo, ServerOpcodes::CooldownEvent]
    );
}
#[tokio::test]
async fn spell_sanctuary_outside_dungeon_stops_player_pve_combat_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let spell_id = 790_i32;
    let player_guid = ObjectGuid::create_player(1, 790);
    let creature_guid = test_creature_guid(18_790);
    let position = Position::new(10.0, 10.0, 0.0, 0.0);
    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
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
        player_guid,
        "SanctuaryTarget".to_string(),
        position,
        0,
        1,
        1,
        80,
        0,
    ));
    session.set_player_health_like_cpp(100, 100);
    add_canonical_test_player_on_map(&canonical, player_guid, position, 0, 0);
    add_canonical_test_creature_indexed_on_map_with_level(
        &canonical,
        creature_guid,
        9001,
        position,
        0,
        0,
        80,
    );
    register_test_creature(&mut session, manager.clone(), creature_guid, 100);
    session.combat_target = Some(creature_guid);
    session.in_combat = true;
    assert!(session.begin_canonical_player_combat_ref_like_cpp(
        player_guid,
        creature_guid,
        false,
        false,
        false,
    ));
    session
        .mutate_world_creature(creature_guid, |creature| {
            creature.enter_combat(player_guid);
            creature
                .creature
                .unit_mut()
                .subsystems_mut()
                .combat
                .add_threat(player_guid, 40.0);
        })
        .unwrap();
    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        spell_id,
        threat_spell_info_like_cpp(
            spell_id,
            wow_data::spell::spell_effect_types::SPELL_EFFECT_SANCTUARY,
            0,
        ),
    );
    session.set_spell_store(Arc::new(spell_store));

    session
        .execute_spell(spell_id, player_guid)
        .await
        .expect("represented EffectSanctuary should execute");

    assert_eq!(session.combat_target, None);
    assert!(!session.in_combat);
    {
        let canonical_guard = canonical.lock().unwrap();
        let map = canonical_guard.find_map(0, 0).unwrap().map();
        let player = map.get_typed_player(player_guid).unwrap();
        let creature = map
            .with_creature_like_cpp(creature_guid, Clone::clone)
            .unwrap();
        assert!(!player.unit().subsystems().combat.has_pve_combat());
        assert!(
            !creature
                .unit()
                .subsystems()
                .combat
                .is_in_combat_with(player_guid)
        );
    }
    let world_combat_target = manager
        .read()
        .unwrap()
        .find_creature(0, 0, creature_guid)
        .unwrap()
        .creature
        .ai_ownership()
        .combat_target;
    assert_eq!(world_combat_target, None);
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::SpellGo, ServerOpcodes::CooldownEvent]
    );
}
#[tokio::test]
async fn spell_effect_heal_negative_amount_is_noop_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let spell_id = 722_i32;
    let player_guid = ObjectGuid::create_player(1, 47);
    session.set_player_guid(Some(player_guid));
    session.set_player_health_like_cpp(50, 100);
    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        spell_id,
        wow_data::SpellInfo {
            spell_id,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: wow_data::spell::spell_effect_types::SPELL_EFFECT_HEAL,
            effect_base_points: -10,
            effect_bonus_coefficient: 0.0,
            aura_type: None,
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: Vec::new(),
        },
    );
    session.set_spell_store(Arc::new(spell_store));

    session
        .execute_spell(spell_id, player_guid)
        .await
        .expect("negative represented heal should execute as C++ no-op effect");

    assert_eq!(session.player_health_like_cpp(), 50);
    let opcodes = drain_server_opcodes(&send_rx);
    assert_eq!(
        opcodes,
        vec![ServerOpcodes::SpellGo, ServerOpcodes::CooldownEvent]
    );
}
#[tokio::test]
async fn spell_heal_mechanical_effect_row_heals_player_like_cpp_without_type_gate() {
    let (mut session, _, send_rx) = make_session();
    let spell_id = 742_i32;
    let player_guid = ObjectGuid::create_player(1, 59);
    session.set_player_guid(Some(player_guid));
    session.set_player_health_like_cpp(40, 100);
    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        spell_id,
        wow_data::SpellInfo {
            spell_id,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: 0,
            effect_base_points: 0,
            effect_bonus_coefficient: 0.0,
            aura_type: None,
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: vec![wow_data::SpellEffectInfo {
                effect_index: 0,
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_HEAL_MECHANICAL,
                effect_base_points: 35,
                ..Default::default()
            }],
        },
    );
    session.set_spell_store(Arc::new(spell_store));

    session
        .execute_spell(spell_id, player_guid)
        .await
        .expect("represented mechanical heal effect row should execute");

    assert_eq!(session.player_health_like_cpp(), 75);
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![
            ServerOpcodes::SpellGo,
            ServerOpcodes::UpdateObject,
            ServerOpcodes::CooldownEvent,
        ]
    );
}
#[tokio::test]
async fn spell_heal_mechanical_negative_amount_is_noop_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let spell_id = 744_i32;
    let player_guid = ObjectGuid::create_player(1, 61);
    session.set_player_guid(Some(player_guid));
    session.set_player_health_like_cpp(50, 100);
    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        spell_id,
        wow_data::SpellInfo {
            spell_id,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: wow_data::spell::spell_effect_types::SPELL_EFFECT_HEAL_MECHANICAL,
            effect_base_points: -10,
            effect_bonus_coefficient: 0.0,
            aura_type: None,
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: Vec::new(),
        },
    );
    session.set_spell_store(Arc::new(spell_store));

    session
        .execute_spell(spell_id, player_guid)
        .await
        .expect("negative represented mechanical heal should execute as C++ no-op effect");

    assert_eq!(session.player_health_like_cpp(), 50);
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::SpellGo, ServerOpcodes::CooldownEvent]
    );
}
