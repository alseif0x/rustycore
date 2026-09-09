//! Session scenarios exercising the represented combat responsibility.
//!
//! Split out of session_tests.rs under #626; assertions and registrations
//! are unchanged and the shared fixtures stay in the parent module.

use super::*;

#[test]
fn combat_tick_keeps_and_damages_canonical_player_victim_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let attacker = ObjectGuid::create_player(1, 83);
    let victim = ObjectGuid::create_player(1, 84);

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
        "Attacker".to_string(),
        Position::new(10.0, 20.0, 30.0, 0.0),
        0,
        1,
        1,
        80,
        0,
    ));
    let _ = session.ensure_canonical_world_map_for_current_player_like_cpp();

    let mut victim_player = Player::new(Some(10), false);
    victim_player
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(victim);
    victim_player.unit_mut().world_mut().set_map(0, 0).unwrap();
    victim_player
        .unit_mut()
        .world_mut()
        .relocate(Position::new(11.0, 20.0, 30.0, 0.0));
    victim_player
        .unit_mut()
        .world_mut()
        .object_mut()
        .add_to_world();
    victim_player.unit_mut().set_level(80);
    victim_player.unit_mut().set_max_health(40);
    victim_player.unit_mut().set_health(40);
    canonical
        .lock()
        .unwrap()
        .find_map_mut(0, 0)
        .unwrap()
        .map_mut()
        .insert_map_object_record(wow_entities::MapObjectRecord::new_player(victim_player).unwrap())
        .unwrap();

    session
        .mutate_canonical_player_like_cpp(|player| {
            let unit = player.unit_mut();
            unit.set_attacking(Some(victim));
            unit.set_target(victim);
            unit.add_unit_state(UnitState::MELEE_ATTACKING.bits());
            unit.set_base_attack_time_like_cpp(WeaponAttackType::BaseAttack, 2_000);
            unit.set_weapon_damage(WeaponAttackType::BaseAttack, 7.0, 7.0);
        })
        .unwrap();
    session.combat_target = Some(victim);
    session.in_combat = true;

    session.tick_combat_sync();

    let sent = send_rx.try_recv().unwrap();
    let opcode = u16::from_le_bytes([sent[0], sent[1]]);
    assert_eq!(opcode, ServerOpcodes::AttackerStateUpdate as u16);
    let guard = canonical.lock().unwrap();
    let map = guard.find_map(0, 0).unwrap().map();
    let attacker_entity = map.get_typed_player(attacker).unwrap();
    let victim_entity = map.get_typed_player(victim).unwrap();
    assert_eq!(attacker_entity.unit().attacking(), Some(victim));
    assert_eq!(
        attacker_entity.unit().last_damaged_target_like_cpp(),
        Some(victim)
    );
    assert_eq!(victim_entity.unit().data().health, 33);
    assert_eq!(session.combat_target, Some(victim));
    assert!(session.in_combat);
}
#[test]
fn combat_tick_offhand_only_does_not_clear_base_swing_error_like_cpp() {
    let (mut session, _, _) = make_session();
    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    let guid = test_creature_guid(18_032);
    let player = ObjectGuid::create_player(1, 81);

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
        "Offhand".to_string(),
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
            unit.set_attack_timer(WeaponAttackType::BaseAttack, 500);
            unit.set_attack_timer(WeaponAttackType::OffAttack, 0);
            unit.set_weapon_damage(WeaponAttackType::OffAttack, 4.0, 4.0);
        })
        .unwrap();
    session.player_swing_error_msg_like_cpp = Some(0);
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

    assert_eq!(session.player_swing_error_msg_like_cpp, Some(0));
    assert_eq!(
        manager
            .read()
            .unwrap()
            .find_creature(0, 0, guid)
            .unwrap()
            .current_hp(),
        36
    );
}
#[test]
fn player_attack_does_not_create_combat_ref_when_can_begin_combat_fails_like_cpp() {
    let (mut session, _, _) = make_session();
    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    let player = ObjectGuid::create_player(1, 157);
    let victim = test_creature_guid(18_114);

    canonical.lock().unwrap().create_world_map(571, 0);
    session.set_map_manager(manager);
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
    session.register_world_creature(
        571,
        Position::new(11.0, 20.0, 30.0, 0.0),
        test_creature_create_data(victim, 9001, 25),
        3,
        5,
        20.0,
        0,
        0,
        0,
        0,
        None,
        0,
        0,
        0,
        0,
        -1,
    );
    session
        .mutate_world_creature(victim, |creature| {
            creature
                .creature
                .unit_mut()
                .subsystems_mut()
                .combat
                .combat_disallowed = true;
        })
        .unwrap();

    session.start_player_attack_like_cpp(victim);

    let guard = canonical.lock().unwrap();
    let map = guard.find_map(571, 0).unwrap().map();
    let player_entity = map.get_typed_player(player).unwrap();
    let creature = map.with_creature_like_cpp(victim, Clone::clone).unwrap();
    assert_eq!(player_entity.unit().attacking(), Some(victim));
    assert!(creature.unit().has_attacker_like_cpp(player));
    assert!(
        !player_entity
            .unit()
            .subsystems()
            .combat
            .is_in_combat_with(victim)
    );
    assert!(
        !creature
            .unit()
            .subsystems()
            .combat
            .is_in_combat_with(player)
    );
}
#[test]
fn combat_tick_revalidates_and_purges_invalid_combat_refs_like_cpp() {
    let (mut session, _, _) = make_session();
    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    let player = ObjectGuid::create_player(1, 158);
    let victim = test_creature_guid(18_115);

    canonical.lock().unwrap().create_world_map(571, 0);
    session.set_map_manager(manager);
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
    session.register_world_creature(
        571,
        Position::new(11.0, 20.0, 30.0, 0.0),
        test_creature_create_data(victim, 9001, 25),
        3,
        5,
        20.0,
        0,
        0,
        0,
        0,
        None,
        0,
        0,
        0,
        0,
        -1,
    );
    session.start_player_attack_like_cpp(victim);
    {
        let guard = canonical.lock().unwrap();
        let map = guard.find_map(571, 0).unwrap().map();
        assert!(
            map.get_typed_player(player)
                .unwrap()
                .unit()
                .subsystems()
                .combat
                .is_in_combat_with(victim)
        );
        assert!(
            map.with_creature_like_cpp(victim, Clone::clone)
                .unwrap()
                .unit()
                .subsystems()
                .combat
                .is_in_combat_with(player)
        );
    }
    session
        .mutate_world_creature(victim, |creature| {
            let mut victim_phase = PhaseShift::default();
            victim_phase.add_phase_like_cpp(20, wow_constants::PhaseFlags::empty(), 1);
            *creature.creature.unit_mut().world_mut().phase_shift_mut() = victim_phase;
        })
        .unwrap();

    session.tick_combat_sync();

    let guard = canonical.lock().unwrap();
    let map = guard.find_map(571, 0).unwrap().map();
    assert!(
        !map.get_typed_player(player)
            .unwrap()
            .unit()
            .subsystems()
            .combat
            .is_in_combat_with(victim)
    );
    assert!(
        !map.with_creature_like_cpp(victim, Clone::clone)
            .unwrap()
            .unit()
            .subsystems()
            .combat
            .is_in_combat_with(player)
    );
    assert_eq!(
        map.get_typed_player(player).unwrap().unit().attacking(),
        Some(victim)
    );
}
#[test]
fn chat_away_ignored_while_in_combat_like_cpp() {
    let (mut session, _, guid) = session_with_canonical_player_for_away_like_cpp();
    session.in_combat = true;

    assert!(
        !session.apply_chat_away_mode_like_cpp(PlayerAwayModeLikeCpp::Afk, "cannot".to_string())
    );

    assert_eq!(session.auto_reply_msg_like_cpp().as_deref(), Some(""));
    assert_eq!(
        session.canonical_player_has_player_flag_like_cpp(guid, PLAYER_FLAGS_AFK_LIKE_CPP),
        Some(false)
    );
}
/// The phase does nothing at all when this session owns the tick.
///
/// Without this, "missing tick" is unguarded in the other direction: the map
/// could resolve swings that the session is also resolving.
#[test]
fn the_player_melee_phase_is_inert_under_the_session_owner_like_cpp() {
    use crate::map_manager::RuntimeTickOwner;

    let manager = shared_map_manager();
    assert_eq!(
        manager.read().unwrap().tick_owner(),
        RuntimeTickOwner::Session
    );
    let mut phase_state = crate::session::PlayerMeleePhaseStateLikeCpp::default();
    let outcome = crate::session::run_legacy_player_melee_tick_once_like_cpp(
        &manager,
        None,
        &[],
        100,
        &mut phase_state,
    );
    assert!(outcome.skipped_owner_not_global);
    assert_eq!(outcome.attackers_seen, 0);
    assert!(outcome.commands.is_empty());
}
#[test]
fn default_session_owner_preserves_combat_tick_packets() {
    // With Session owner, run_combat_tick must produce the same bytes that
    // tick_combat_sync previously sent directly.
    use crate::map_manager::RuntimeTickOwner;
    let manager = shared_map_manager();
    assert_eq!(
        manager.read().unwrap().tick_owner(),
        RuntimeTickOwner::Session
    );

    let (mut session, _, recv) = make_session();
    let guid = test_creature_guid(90_002);
    let player = ObjectGuid::create_player(1, 90_002);
    session.player_guid = Some(player);
    session.combat_target = Some(guid);
    session.in_combat = true;
    session.client_visible_guids_like_cpp.insert(guid);
    register_test_creature(&mut session, manager.clone(), guid, 40);
    session
        .mutate_world_creature(guid, |creature| {
            creature.enter_combat(player);
            creature.creature.ai_ownership_mut().last_swing_ms = 0;
            creature.creature.ai_ownership_mut().swing_timer_ms = 0;
        })
        .unwrap();

    let output = session.run_combat_tick();
    // Channel must be empty before flush.
    assert!(
        recv.try_recv().is_err(),
        "run_combat_tick must not send directly to the channel"
    );
    // After flush, the AttackerStateUpdate packet must arrive.
    session.flush_runtime_output(output);
    let pkt = recv
        .try_recv()
        .expect("flush must deliver the AttackerStateUpdate packet");
    let opcode = u16::from_le_bytes([pkt[0], pkt[1]]);
    assert_eq!(opcode, ServerOpcodes::AttackerStateUpdate as u16);
}
#[test]
fn legacy_combat_ai_no_melee_fixture_preserves_rng_until_due_15691_hit_like_cpp() {
    use crate::map_manager::RuntimeTickOwner;

    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    let (mut session, _, _) = make_session();
    let creature_guid = test_creature_guid(91_320);
    let victim_guid = ObjectGuid::create_player(1, 91_321);
    let spell_id = 15_691_i32;
    add_canonical_creature_spell_test_pair_like_cpp(&canonical, creature_guid, victim_guid);
    {
        let mut canonical = canonical.lock().unwrap();
        let map = canonical.find_map_mut(0, 0).unwrap().map_mut();
        map.get_typed_creature_mut(creature_guid)
            .unwrap()
            .unit_mut()
            .set_level(64);
        let victim = map.get_typed_player_mut(victim_guid).unwrap();
        victim.unit_mut().set_level(80);
        victim
            .unit_mut()
            .world_mut()
            .relocate(Position::new(14.0, 10.0, 0.0, 0.0));
    }
    register_test_creature_mirrored_like_cpp(
        &mut session,
        manager.clone(),
        &canonical,
        creature_guid,
        25,
    );

    let miss_threshold = creature_melee_spell_miss_threshold_3_3_5_like_cpp();
    assert_eq!(miss_threshold, 500);
    let hit_seed = (0_u64..10_000)
        .find(|seed| {
            let mut rng = StdRng::seed_from_u64(*seed);
            let _initial_delay = rng.gen_range(5_000_u64..=10_000_u64);
            rng.gen_range(0..=9_999_u32) >= miss_threshold
        })
        .unwrap();
    session
        .mutate_world_creature(creature_guid, |creature| {
            creature
                .creature
                .set_ai_identity_names_runtime_like_cpp("CombatAI", String::new());
            creature.creature.set_spell(0, spell_id as u32);
            creature.enter_combat(victim_guid);
            creature.creature.ai_ownership_mut().last_swing_ms = 0;
            creature.creature.ai_ownership_mut().swing_timer_ms = 0;
            let mut static_flags = [0; 8];
            static_flags[0] = wow_constants::creature::CreatureStaticFlags::NO_MELEE_FLEE.bits();
            creature
                .creature
                .set_static_flags_runtime_like_cpp(static_flags);
            creature.seed_runtime_rng_like_cpp(hit_seed);
            assert!(creature.can_swing());
            assert!(creature.runtime_rng_authority_complete_like_cpp());
        })
        .unwrap();
    manager
        .write()
        .unwrap()
        .set_tick_owner(RuntimeTickOwner::GlobalLegacy);

    let mut spell = creature_ai_test_spell_info_like_cpp(spell_id, 6, 0);
    spell.recovery_time_ms = 0;
    spell.effect_base_points = 64;
    spell.effects[0].effect_base_points = 64;
    let mut config = creature_ai_spell_test_config_like_cpp(spell, false, 5.0);
    let mut issue_26_attributes = [0_u32; 15];
    issue_26_attributes[0] = 0x000d_0010;
    Arc::get_mut(config.spell_store.as_mut().unwrap())
        .unwrap()
        .insert_spell_misc_attributes_like_cpp(spell_id, issue_26_attributes);
    let mut misc = spell_misc_entry_like_cpp(8_320, spell_id as u32, 2);
    misc.attributes = issue_26_attributes.map(|attribute| attribute as i32);
    config.spell_misc_store = Some(Arc::new(wow_data::SpellMiscStore::from_entries([misc])));
    let mut range = spell_range_entry_like_cpp(2, 0.0, 5.0);
    range.flags = 1;
    config.spell_range_store = Some(Arc::new(wow_data::SpellRangeStore::from_entries([range])));
    config.spell_cooldowns_store = Some(Arc::new(wow_data::SpellCooldownsStore::from_entries([
        wow_data::SpellCooldownsEntry {
            id: 8_321,
            difficulty_id: 0,
            category_recovery_time: 0,
            recovery_time: 0,
            start_recovery_time: 1_000,
            spell_id: spell_id as u32,
        },
    ])));

    let initialized =
        run_legacy_creature_spell_tick_once_like_cpp(&manager, Some(&canonical), &config);
    assert_eq!(initialized.schedules_initialized, 1);
    assert_eq!(initialized.casts_ready, 0);
    assert!(initialized.plan.events.is_empty());
    let initial_due_in_ms = session
        .mutate_world_creature(creature_guid, |creature| {
            assert!(creature.runtime_rng_authority_complete_like_cpp());
            creature.creature_spell_due_in_ms_for_test(0).unwrap()
        })
        .unwrap();
    assert!((4_750..=10_000).contains(&initial_due_in_ms));

    let melee = run_legacy_creature_melee_tick_once_like_cpp(&manager, Some(&canonical));
    assert_eq!(melee.melee_precondition_rejections, 1);
    assert_eq!(melee.swings_ready, 0);
    assert_eq!(melee.canonical_hits, 0);
    assert!(melee.commands.is_empty());
    session
        .mutate_world_creature(creature_guid, |creature| {
            assert!(
                creature.runtime_rng_authority_complete_like_cpp(),
                "NO_MELEE must return before the melee damage RNG draw"
            );
            assert_eq!(creature.creature.ai_ownership().last_swing_ms, 0);
            assert_eq!(creature.creature.ai_ownership().swing_timer_ms, 0);
            creature.backdate_runtime_clock_for_test(Duration::from_millis(
                initial_due_in_ms.saturating_add(250),
            ));
        })
        .unwrap();

    let cast = run_legacy_creature_spell_tick_once_like_cpp(&manager, Some(&canonical), &config);

    assert_eq!(cast.casts_ready, 1, "spell tick outcome: {cast:?}");
    assert_eq!(cast.canonical_cast_preconditions_passed, 1);
    assert_eq!(cast.spell_hits, 1);
    assert_eq!(cast.runtime_rng_authority_rejections, 1);
    assert_eq!(cast.plan.events.len(), 1);
    let (start, go) = decode_atomic_creature_spell_wire_pair_like_cpp(&cast.plan.events[0]);
    assert_eq!(start.opcode, ServerOpcodes::SpellStart as u16);
    assert_eq!(go.opcode, ServerOpcodes::SpellGo as u16);
    assert_eq!(start.spell_id, spell_id);
    assert_eq!(go.spell_id, spell_id);
    assert_eq!(go.hit_targets, vec![victim_guid]);
    assert!(go.miss_targets.is_empty());
    session
        .mutate_world_creature(creature_guid, |creature| {
            assert!(
                !creature.runtime_rng_authority_complete_like_cpp(),
                "the committed HIT remains in the plan before launch RNG is tombstoned"
            );
            assert_eq!(creature.creature_spell_due_in_ms_for_test(0), None);
        })
        .unwrap();
}
#[test]
fn legacy_turret_ai_can_attack_uses_combat_reaches_and_strict_bounds_like_cpp() {
    let manager = shared_map_manager();
    let (mut session, _, _) = make_session();
    let creature_guid = test_creature_guid(91_304);
    register_test_creature(&mut session, manager.clone(), creature_guid, 25);
    session
        .mutate_world_creature(creature_guid, |creature| {
            creature.creature.set_spell(0, 7_003);
            creature.creature.unit_mut().set_combat_reach(1.0);
        })
        .unwrap();
    let config = LegacyCreatureAggroConfigLikeCpp {
        spell_misc_store: Some(Arc::new(wow_data::SpellMiscStore::from_entries([
            spell_misc_entry_like_cpp(8_301, 7_003, 71),
        ]))),
        spell_range_store: Some(Arc::new(wow_data::SpellRangeStore::from_entries([
            spell_range_entry_like_cpp(71, 1.0, 5.0),
        ]))),
        ..legacy_aggro_hostile_config_like_cpp()
    };
    let guard = manager.read().unwrap();
    let creature = guard.find_creature(0, 0, creature_guid).unwrap();
    let mut candidate =
        legacy_aggro_candidate_like_cpp(ObjectGuid::create_player(1, 91_305), Position::default());
    candidate.player_combat_reach = 1.0;

    candidate.position = Position::new(16.0, 10.0, 0.0, 0.0);
    assert_eq!(
        legacy_creature_ai_can_attack_decision_like_cpp(
            &CreatureAiKindLikeCpp::TurretAI,
            creature,
            &candidate,
            &config,
        ),
        LegacyCreatureAiCanAttackDecisionLikeCpp::Allowed,
        "center distance 6 is inside max 5 + reaches 1 + 1"
    );
    candidate.position = Position::new(17.0, 10.0, 0.0, 0.0);
    assert_eq!(
        legacy_creature_ai_can_attack_decision_like_cpp(
            &CreatureAiKindLikeCpp::TurretAI,
            creature,
            &candidate,
            &config,
        ),
        LegacyCreatureAiCanAttackDecisionLikeCpp::Rejected,
        "IsWithinCombatRange is strict at max + reaches"
    );
    candidate.position = Position::new(12.5, 10.0, 0.0, 0.0);
    assert_eq!(
        legacy_creature_ai_can_attack_decision_like_cpp(
            &CreatureAiKindLikeCpp::TurretAI,
            creature,
            &candidate,
            &config,
        ),
        LegacyCreatureAiCanAttackDecisionLikeCpp::Rejected,
        "center distance 2.5 is inside min 1 + reaches 1 + 1"
    );
    candidate.position = Position::new(13.0, 10.0, 0.0, 0.0);
    assert_eq!(
        legacy_creature_ai_can_attack_decision_like_cpp(
            &CreatureAiKindLikeCpp::TurretAI,
            creature,
            &candidate,
            &config,
        ),
        LegacyCreatureAiCanAttackDecisionLikeCpp::Allowed,
        "the min + reaches boundary itself is not within strict range"
    );
}
