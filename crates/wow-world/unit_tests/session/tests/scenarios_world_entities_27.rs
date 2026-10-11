//! Session scenarios exercising the represented world entities responsibility.
//!
//! Split out of session_tests.rs under #626; assertions and registrations
//! are unchanged and the shared fixtures stay in the parent module.

use super::*;

#[test]
fn legacy_creature_spell_projectile_payload_gate_preserves_15691_like_cpp() {
    const SPELL_ATTR0_USES_RANGED_SLOT_LIKE_CPP: u32 = 0x0000_0002;
    const SPELL_ATTR10_USES_RANGED_SLOT_COSMETIC_ONLY_LIKE_CPP: u32 = 0x0000_0004;
    const SPELL_ATTR0_CU_NEEDS_AMMO_DATA_LIKE_CPP: u32 = 0x0008_0000;

    let spell_id = 15_691_u32;
    let victim_guid = ObjectGuid::create_player(1, 91_206);
    let spell = creature_ai_test_spell_info_like_cpp(spell_id as i32, 6, 0);
    let base_config = creature_ai_spell_test_config_like_cpp(spell.clone(), false, 30.0);
    let fixture_requires_projectile =
        creature_ai_spell_requires_projectile_payload_like_cpp(spell_id, 0, &base_config);
    assert!(
        !fixture_requires_projectile,
        "Eviscerate 15691 is the issue-26 fixture and has an ordinary non-projectile START+GO shape"
    );
    assert_eq!(
        creature_ai_spell_single_unit_topology_like_cpp(
            &spell,
            victim_guid,
            victim_guid,
            fixture_requires_projectile,
        ),
        Ok(())
    );

    for (attribute_word, attribute) in [
        (0, SPELL_ATTR0_USES_RANGED_SLOT_LIKE_CPP),
        (10, SPELL_ATTR10_USES_RANGED_SLOT_COSMETIC_ONLY_LIKE_CPP),
    ] {
        let mut attributes = [0_u32; 15];
        attributes[attribute_word] = attribute;
        let mut spell_store = wow_data::SpellStore::new();
        spell_store.insert(spell.spell_id, spell.clone());
        spell_store.insert_spell_misc_attributes_like_cpp(spell.spell_id, attributes);
        let mut config = base_config.clone();
        config.spell_store = Some(Arc::new(spell_store));
        let requires_projectile =
            creature_ai_spell_requires_projectile_payload_like_cpp(spell_id, 0, &config);
        assert!(requires_projectile);
        assert_eq!(
            creature_ai_spell_single_unit_topology_like_cpp(
                &spell,
                victim_guid,
                victim_guid,
                requires_projectile,
            ),
            Err(CreatureAiSpellRepresentationRejectionLikeCpp::ProjectileOrAmmo)
        );
    }

    let mut custom_config = base_config;
    custom_config.spell_custom_attribute_store =
        Some(Arc::new(wow_data::SpellCustomAttributeStoreLikeCpp {
            attributes_by_spell_and_difficulty: std::collections::BTreeMap::from([(
                wow_data::SpellCustomAttributeKeyLikeCpp {
                    spell_id,
                    difficulty: 0,
                },
                SPELL_ATTR0_CU_NEEDS_AMMO_DATA_LIKE_CPP,
            )]),
        }));
    let requires_projectile =
        creature_ai_spell_requires_projectile_payload_like_cpp(spell_id, 0, &custom_config);
    assert!(requires_projectile);
    assert_eq!(
        creature_ai_spell_single_unit_topology_like_cpp(
            &spell,
            victim_guid,
            victim_guid,
            requires_projectile,
        ),
        Err(CreatureAiSpellRepresentationRejectionLikeCpp::ProjectileOrAmmo)
    );
}
#[test]
fn legacy_creature_spell_tick_is_noop_under_session_owner_like_cpp() {
    let manager = shared_map_manager();
    let spell_id = 70_004_i32;
    let config = creature_ai_spell_test_config_like_cpp(
        creature_ai_test_spell_info_like_cpp(spell_id, 6, 0),
        false,
        30.0,
    );

    let outcome = run_canonical_creature_spell_phase_for_test_like_cpp(&manager, &config);
    assert_eq!(outcome.maps_seen, 0);
    assert!(outcome.plan.events.is_empty());
}
#[test]
fn creature_aggro_tick_once_honors_turret_ai_range_like_cpp() {
    let manager = shared_map_manager();
    let (mut session, _, _) = make_session();
    let creature_guid = test_creature_guid(91_116);
    let manager = session
        .core
        .canonical_map_manager
        .clone()
        .unwrap_or_else(|| manager.clone());
    register_test_creature(&mut session, manager.clone(), creature_guid, 25);
    session
        .mutate_world_creature(creature_guid, |creature| {
            creature.ai_ownership_mut().aggro_radius = 100.0;
            creature.unit_mut().set_combat_reach(0.0);
            creature.set_ai_identity_names_runtime_like_cpp("TurretAI", String::new());
            creature.set_spell(0, 7001);
        })
        .unwrap();

    let too_close = ObjectGuid::create_player(1, 91_117);
    let too_far = ObjectGuid::create_player(1, 91_118);
    let valid = ObjectGuid::create_player(1, 91_119);
    let candidates = vec![
        aggro_candidate_like_cpp(too_close, Position::new(10.5, 10.0, 0.0, 0.0)),
        aggro_candidate_like_cpp(too_far, Position::new(16.0, 10.0, 0.0, 0.0)),
        aggro_candidate_like_cpp(valid, Position::new(12.0, 10.0, 0.0, 0.0)),
    ];

    let outcome = run_canonical_creature_aggro_phase_for_test_like_cpp(
        &manager,
        &candidates,
        CreatureAggroConfigLikeCpp {
            spell_misc_store: Some(Arc::new(wow_data::SpellMiscStore::from_entries([
                spell_misc_entry_like_cpp(1, 7001, 71),
            ]))),
            spell_range_store: Some(Arc::new(wow_data::SpellRangeStore::from_entries([
                spell_range_entry_like_cpp(71, 1.0, 5.0),
            ]))),
            ..aggro_hostile_config_like_cpp()
        },
    );

    assert_eq!(outcome.ai_can_attack_unrepresented, 0);
    assert_eq!(outcome.ai_can_attack_rejections, 2);
    assert_eq!(outcome.aggro_starts, 1);
    assert_eq!(outcome.commands.len(), 1);
    assert_eq!(outcome.commands[0].victim_guid, valid);
}
#[test]
fn creature_aggro_tick_once_fails_closed_for_missing_turret_range_like_cpp() {
    let manager = shared_map_manager();
    let (mut session, _, _) = make_session();
    let creature_guid = test_creature_guid(91_120);
    let manager = session
        .core
        .canonical_map_manager
        .clone()
        .unwrap_or_else(|| manager.clone());
    register_test_creature(&mut session, manager.clone(), creature_guid, 25);
    session
        .mutate_world_creature(creature_guid, |creature| {
            creature.ai_ownership_mut().aggro_radius = 5.0;
            creature.set_ai_identity_names_runtime_like_cpp("TurretAI", String::new());
            creature.set_spell(0, 7002);
        })
        .unwrap();

    let player = ObjectGuid::create_player(1, 91_121);
    let candidates = vec![aggro_candidate_like_cpp(
        player,
        Position::new(10.5, 10.5, 0.0, 0.0),
    )];

    let outcome = run_canonical_creature_aggro_phase_for_test_like_cpp(
        &manager,
        &candidates,
        aggro_hostile_config_like_cpp(),
    );

    assert_eq!(outcome.ai_can_attack_unrepresented, 1);
    assert_eq!(outcome.aggro_starts, 0);
    assert!(outcome.commands.is_empty());
}
#[test]
fn creature_aggro_tick_once_rejects_non_positive_radius_like_cpp() {
    let manager = shared_map_manager();
    let (mut session, _, _) = make_session();
    let creature_guid = test_creature_guid(91_012);
    let manager = session
        .core
        .canonical_map_manager
        .clone()
        .unwrap_or_else(|| manager.clone());
    register_test_creature(&mut session, manager.clone(), creature_guid, 25);
    session
        .mutate_world_creature(creature_guid, |creature| {
            creature.ai_ownership_mut().aggro_radius = 0.0;
        })
        .unwrap();

    let player = ObjectGuid::create_player(1, 91_013);
    let candidates = vec![aggro_candidate_like_cpp(
        player,
        Position::new(10.0, 10.0, 0.0, 0.0),
    )];

    let outcome = run_canonical_creature_aggro_phase_for_test_like_cpp(
        &manager,
        &candidates,
        aggro_hostile_config_like_cpp(),
    );
    assert_eq!(outcome.maps_seen, 1);
    assert_eq!(outcome.creatures_seen, 1);
    assert_eq!(outcome.candidates_seen, 1);
    assert_eq!(outcome.aggro_starts, 0);
    assert!(outcome.commands.is_empty());
    let combat_target = {
        let guard = manager.lock().unwrap();
        guard
            .find_creature(0, 0, creature_guid)
            .unwrap()
            .creature
            .ai_ownership()
            .combat_target
    };
    assert_eq!(combat_target, None);
}
#[test]
fn creature_aggro_tick_once_rejects_immune_to_pc_like_cpp() {
    let manager = shared_map_manager();
    let (mut session, _, _) = make_session();
    let creature_guid = test_creature_guid(91_014);
    let manager = session
        .core
        .canonical_map_manager
        .clone()
        .unwrap_or_else(|| manager.clone());
    register_test_creature(&mut session, manager.clone(), creature_guid, 25);
    session
        .mutate_world_creature(creature_guid, |creature| {
            creature.ai_ownership_mut().aggro_radius = 5.0;
            creature
                .unit_mut()
                .set_unit_flags_like_cpp(UnitFlags::IMMUNE_TO_PC);
        })
        .unwrap();

    let player = ObjectGuid::create_player(1, 91_015);
    let candidates = vec![aggro_candidate_like_cpp(
        player,
        Position::new(10.5, 10.5, 0.0, 0.0),
    )];

    let outcome = run_canonical_creature_aggro_phase_for_test_like_cpp(
        &manager,
        &candidates,
        aggro_hostile_config_like_cpp(),
    );
    assert_eq!(outcome.maps_seen, 1);
    assert_eq!(outcome.creatures_seen, 1);
    assert_eq!(outcome.candidates_seen, 1);
    assert_eq!(outcome.aggro_starts, 0);
    assert!(outcome.commands.is_empty());
    let combat_target = {
        let guard = manager.lock().unwrap();
        guard
            .find_creature(0, 0, creature_guid)
            .unwrap()
            .creature
            .ai_ownership()
            .combat_target
    };
    assert_eq!(combat_target, None);
}
#[test]
fn creature_aggro_tick_once_rejects_civilian_like_cpp() {
    let manager = shared_map_manager();
    let (mut session, _, _) = make_session();
    let creature_guid = test_creature_guid(91_016);
    let manager = session
        .core
        .canonical_map_manager
        .clone()
        .unwrap_or_else(|| manager.clone());
    register_test_creature(&mut session, manager.clone(), creature_guid, 25);
    session
        .mutate_world_creature(creature_guid, |creature| {
            creature.ai_ownership_mut().aggro_radius = 5.0;
            creature.set_flags_extra_runtime_like_cpp(
                wow_constants::CreatureFlagsExtra::CIVILIAN.bits(),
            );
        })
        .unwrap();

    let player = ObjectGuid::create_player(1, 91_017);
    let candidates = vec![aggro_candidate_like_cpp(
        player,
        Position::new(10.5, 10.5, 0.0, 0.0),
    )];

    let outcome = run_canonical_creature_aggro_phase_for_test_like_cpp(
        &manager,
        &candidates,
        aggro_hostile_config_like_cpp(),
    );
    assert_eq!(outcome.maps_seen, 1);
    assert_eq!(outcome.creatures_seen, 1);
    assert_eq!(outcome.candidates_seen, 1);
    assert_eq!(outcome.aggro_starts, 0);
    assert!(outcome.commands.is_empty());
    let combat_target = {
        let guard = manager.lock().unwrap();
        guard
            .find_creature(0, 0, creature_guid)
            .unwrap()
            .creature
            .ai_ownership()
            .combat_target
    };
    assert_eq!(combat_target, None);
}
#[test]
fn legacy_creature_melee_tick_once_preserves_compatibility_player_damage_like_cpp() {
    let manager = shared_map_manager();
    let canonical = Arc::clone(&manager);
    let player = ObjectGuid::create_player(1, 91_003);
    add_canonical_test_player_on_map(
        &canonical,
        player,
        Position::new(10.0, 10.0, 0.0, 0.0),
        0,
        0,
    );
    {
        let mut guard = canonical.lock().unwrap();
        let typed = guard
            .find_map_mut(0, 0)
            .unwrap()
            .map_mut()
            .get_typed_player_mut(player)
            .unwrap();
        typed.unit_mut().set_level(80);
        typed.unit_mut().set_max_health(100);
        typed.unit_mut().set_health(100);
    }

    let (mut session, _, _) = make_session();
    session.set_canonical_map_manager(Arc::clone(&canonical));
    let creature_guid = test_creature_guid(91_004);
    add_canonical_test_creature_on_map(
        &canonical,
        creature_guid,
        9001,
        Position::new(10.0, 10.0, 0.0, 0.0),
        0,
        0,
        0,
    );
    let manager = session
        .core
        .canonical_map_manager
        .clone()
        .unwrap_or_else(|| manager.clone());
    register_test_creature(&mut session, manager.clone(), creature_guid, 25);
    session
        .mutate_world_creature(creature_guid, |creature| {
            creature.enter_combat(player);
            creature.ai_ownership_mut().swing_timer_ms = 0;
            crate::creature_projection::test_support::seed_runtime_rng_like_cpp(creature, 0x91_004);
        })
        .unwrap();

    let outcome =
        run_canonical_creature_melee_phase_for_test_like_cpp(&canonical, &Default::default());
    assert_eq!(outcome.maps_seen, 1);
    assert_eq!(outcome.creatures_seen, 1);
    assert_eq!(outcome.swings_ready, 1);
    assert_eq!(outcome.melee_outcomes_unrepresented, 1);
    assert_eq!(outcome.runtime_rng_authority_rejections, 0);
    assert_eq!(outcome.canonical_hits, 1);
    assert_eq!(outcome.commands.len(), 1);
    assert!(outcome.plan.events.is_empty());

    let health = canonical
        .lock()
        .unwrap()
        .find_map(0, 0)
        .unwrap()
        .map()
        .get_typed_player(player)
        .unwrap()
        .unit()
        .data()
        .health;
    assert_eq!(health, 100 - u64::from(outcome.commands[0].damage));
    assert_eq!(outcome.commands[0].victim_health_after, health);
    let guard = manager.lock().unwrap();
    let creature = guard.find_creature(0, 0, creature_guid).unwrap();
    assert!(!creature.runtime_rng_authority_complete_like_cpp());
    assert_eq!(
        creature.creature.ai_ownership().swing_timer_ms,
        2_000,
        "C++ rearms BASE_ATTACK after AttackerStateUpdate"
    );
    assert!(!creature.can_swing());
}
#[test]
fn legacy_creature_melee_tick_once_two_attackers_commit_only_one_lethal_player_hit_like_cpp() {
    let manager = shared_map_manager();
    let canonical = Arc::clone(&manager);
    let player = ObjectGuid::create_player(1, 91_040);
    add_canonical_test_player_on_map(
        &canonical,
        player,
        Position::new(10.0, 10.0, 0.0, 0.0),
        0,
        0,
    );
    {
        let mut guard = canonical.lock().unwrap();
        let player = guard
            .find_map_mut(0, 0)
            .unwrap()
            .map_mut()
            .get_typed_player_mut(player)
            .unwrap();
        player.unit_mut().set_level(80);
        player.unit_mut().set_max_health(1);
        player.unit_mut().set_health(1);
    }

    let (mut session, _, _) = make_session();
    session.set_canonical_map_manager(Arc::clone(&canonical));
    let attackers = [test_creature_guid(91_041), test_creature_guid(91_042)];
    for attacker in attackers {
        add_canonical_test_creature_on_map(
            &canonical,
            attacker,
            9001,
            Position::new(10.0, 10.0, 0.0, 0.0),
            0,
            0,
            0,
        );
        register_test_creature(&mut session, Arc::clone(&manager), attacker, 25);
        session
            .mutate_world_creature(attacker, |creature| {
                creature.enter_combat(player);
                creature.ai_ownership_mut().last_swing_ms = 0;
                creature.ai_ownership_mut().swing_timer_ms = 0;
                creature.ai_ownership_mut().min_damage = 1;
                creature.ai_ownership_mut().max_damage = 1;
            })
            .unwrap();
    }

    let outcome =
        run_canonical_creature_melee_phase_for_test_like_cpp(&canonical, &Default::default());

    assert_eq!(outcome.swings_ready, 2);
    assert_eq!(outcome.melee_outcomes_unrepresented, 1);
    assert_eq!(outcome.canonical_hits, 1);
    assert_eq!(outcome.melee_precondition_rejections, 1);
    assert_eq!(outcome.commands.len(), 1);
    assert_eq!(outcome.commands[0].damage, 1);
    assert_eq!(outcome.commands[0].over_damage, 0);
    let canonical_guard = canonical.lock().unwrap();
    let player = canonical_guard
        .find_map(0, 0)
        .unwrap()
        .map()
        .get_typed_player(player)
        .unwrap();
    assert_eq!(player.unit().data().health, 0);
    assert_eq!(
        player.unit().death_state(),
        wow_constants::DeathState::JustDied
    );
    assert!(!player.unit().is_alive());
    assert_eq!(
        outcome.commands[0].victim_health_state_revision_after,
        player.unit().health_state_revision_like_cpp()
    );
    drop(canonical_guard);

    let legacy_guard = manager.lock().unwrap();
    let tombstoned = attackers
        .into_iter()
        .filter(|attacker| {
            let creature = legacy_guard.find_creature(0, 0, *attacker).unwrap();
            assert_eq!(creature.creature.ai_ownership().swing_timer_ms, 2_000);
            !creature.runtime_rng_authority_complete_like_cpp()
        })
        .count();
    assert_eq!(tombstoned, 1, "only the committed hit consumes melee RNG");
}
