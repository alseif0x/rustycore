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

    let outcome = run_legacy_creature_spell_tick_once_like_cpp(&manager, None, &config);

    assert!(outcome.skipped_owner_not_global);
    assert_eq!(outcome.maps_seen, 0);
    assert!(outcome.plan.events.is_empty());
}
#[test]
fn legacy_creature_aggro_tick_once_honors_turret_ai_range_like_cpp() {
    use crate::map_manager::RuntimeTickOwner;
    let manager = shared_map_manager();
    let (mut session, _, _) = make_session();
    let creature_guid = test_creature_guid(91_116);
    register_test_creature(&mut session, manager.clone(), creature_guid, 25);
    session
        .mutate_world_creature(creature_guid, |creature| {
            creature.creature.ai_ownership_mut().aggro_radius = 100.0;
            creature.creature.unit_mut().set_combat_reach(0.0);
            creature
                .creature
                .set_ai_identity_names_runtime_like_cpp("TurretAI", String::new());
            creature.creature.set_spell(0, 7001);
        })
        .unwrap();
    manager
        .write()
        .unwrap()
        .set_tick_owner(RuntimeTickOwner::GlobalLegacy);

    let too_close = ObjectGuid::create_player(1, 91_117);
    let too_far = ObjectGuid::create_player(1, 91_118);
    let valid = ObjectGuid::create_player(1, 91_119);
    let candidates = vec![
        legacy_aggro_candidate_like_cpp(too_close, Position::new(10.5, 10.0, 0.0, 0.0)),
        legacy_aggro_candidate_like_cpp(too_far, Position::new(16.0, 10.0, 0.0, 0.0)),
        legacy_aggro_candidate_like_cpp(valid, Position::new(12.0, 10.0, 0.0, 0.0)),
    ];

    let outcome = run_legacy_creature_aggro_tick_once_with_config_like_cpp(
        &manager,
        &candidates,
        LegacyCreatureAggroConfigLikeCpp {
            spell_misc_store: Some(Arc::new(wow_data::SpellMiscStore::from_entries([
                spell_misc_entry_like_cpp(1, 7001, 71),
            ]))),
            spell_range_store: Some(Arc::new(wow_data::SpellRangeStore::from_entries([
                spell_range_entry_like_cpp(71, 1.0, 5.0),
            ]))),
            ..legacy_aggro_hostile_config_like_cpp()
        },
    );

    assert_eq!(outcome.ai_can_attack_unrepresented, 0);
    assert_eq!(outcome.ai_can_attack_rejections, 2);
    assert_eq!(outcome.aggro_starts, 1);
    assert_eq!(outcome.commands.len(), 1);
    assert_eq!(outcome.commands[0].victim_guid, valid);
}
#[test]
fn legacy_creature_aggro_tick_once_fails_closed_for_missing_turret_range_like_cpp() {
    use crate::map_manager::RuntimeTickOwner;
    let manager = shared_map_manager();
    let (mut session, _, _) = make_session();
    let creature_guid = test_creature_guid(91_120);
    register_test_creature(&mut session, manager.clone(), creature_guid, 25);
    session
        .mutate_world_creature(creature_guid, |creature| {
            creature.creature.ai_ownership_mut().aggro_radius = 5.0;
            creature
                .creature
                .set_ai_identity_names_runtime_like_cpp("TurretAI", String::new());
            creature.creature.set_spell(0, 7002);
        })
        .unwrap();
    manager
        .write()
        .unwrap()
        .set_tick_owner(RuntimeTickOwner::GlobalLegacy);

    let player = ObjectGuid::create_player(1, 91_121);
    let candidates = vec![legacy_aggro_candidate_like_cpp(
        player,
        Position::new(10.5, 10.5, 0.0, 0.0),
    )];

    let outcome = run_legacy_creature_aggro_tick_once_with_config_like_cpp(
        &manager,
        &candidates,
        legacy_aggro_hostile_config_like_cpp(),
    );

    assert_eq!(outcome.ai_can_attack_unrepresented, 1);
    assert_eq!(outcome.aggro_starts, 0);
    assert!(outcome.commands.is_empty());
}
#[test]
fn legacy_creature_aggro_tick_once_rejects_non_positive_radius_like_cpp() {
    use crate::map_manager::RuntimeTickOwner;
    let manager = shared_map_manager();
    let (mut session, _, _) = make_session();
    let creature_guid = test_creature_guid(91_012);
    register_test_creature(&mut session, manager.clone(), creature_guid, 25);
    session
        .mutate_world_creature(creature_guid, |creature| {
            creature.creature.ai_ownership_mut().aggro_radius = 0.0;
        })
        .unwrap();
    manager
        .write()
        .unwrap()
        .set_tick_owner(RuntimeTickOwner::GlobalLegacy);

    let player = ObjectGuid::create_player(1, 91_013);
    let candidates = vec![legacy_aggro_candidate_like_cpp(
        player,
        Position::new(10.0, 10.0, 0.0, 0.0),
    )];

    let outcome = run_legacy_creature_aggro_tick_once_with_config_like_cpp(
        &manager,
        &candidates,
        legacy_aggro_hostile_config_like_cpp(),
    );

    assert!(!outcome.skipped_owner_not_global);
    assert_eq!(outcome.maps_seen, 1);
    assert_eq!(outcome.creatures_seen, 1);
    assert_eq!(outcome.candidates_seen, 1);
    assert_eq!(outcome.aggro_starts, 0);
    assert!(outcome.commands.is_empty());
    let combat_target = {
        let guard = manager.read().unwrap();
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
fn legacy_creature_aggro_tick_once_rejects_immune_to_pc_like_cpp() {
    use crate::map_manager::RuntimeTickOwner;
    let manager = shared_map_manager();
    let (mut session, _, _) = make_session();
    let creature_guid = test_creature_guid(91_014);
    register_test_creature(&mut session, manager.clone(), creature_guid, 25);
    session
        .mutate_world_creature(creature_guid, |creature| {
            creature.creature.ai_ownership_mut().aggro_radius = 5.0;
            creature
                .creature
                .unit_mut()
                .set_unit_flags_like_cpp(UnitFlags::IMMUNE_TO_PC);
        })
        .unwrap();
    manager
        .write()
        .unwrap()
        .set_tick_owner(RuntimeTickOwner::GlobalLegacy);

    let player = ObjectGuid::create_player(1, 91_015);
    let candidates = vec![legacy_aggro_candidate_like_cpp(
        player,
        Position::new(10.5, 10.5, 0.0, 0.0),
    )];

    let outcome = run_legacy_creature_aggro_tick_once_with_config_like_cpp(
        &manager,
        &candidates,
        legacy_aggro_hostile_config_like_cpp(),
    );

    assert!(!outcome.skipped_owner_not_global);
    assert_eq!(outcome.maps_seen, 1);
    assert_eq!(outcome.creatures_seen, 1);
    assert_eq!(outcome.candidates_seen, 1);
    assert_eq!(outcome.aggro_starts, 0);
    assert!(outcome.commands.is_empty());
    let combat_target = {
        let guard = manager.read().unwrap();
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
fn legacy_creature_aggro_tick_once_rejects_civilian_like_cpp() {
    use crate::map_manager::RuntimeTickOwner;
    let manager = shared_map_manager();
    let (mut session, _, _) = make_session();
    let creature_guid = test_creature_guid(91_016);
    register_test_creature(&mut session, manager.clone(), creature_guid, 25);
    session
        .mutate_world_creature(creature_guid, |creature| {
            creature.creature.ai_ownership_mut().aggro_radius = 5.0;
            creature.creature.set_flags_extra_runtime_like_cpp(
                wow_constants::CreatureFlagsExtra::CIVILIAN.bits(),
            );
        })
        .unwrap();
    manager
        .write()
        .unwrap()
        .set_tick_owner(RuntimeTickOwner::GlobalLegacy);

    let player = ObjectGuid::create_player(1, 91_017);
    let candidates = vec![legacy_aggro_candidate_like_cpp(
        player,
        Position::new(10.5, 10.5, 0.0, 0.0),
    )];

    let outcome = run_legacy_creature_aggro_tick_once_with_config_like_cpp(
        &manager,
        &candidates,
        legacy_aggro_hostile_config_like_cpp(),
    );

    assert!(!outcome.skipped_owner_not_global);
    assert_eq!(outcome.maps_seen, 1);
    assert_eq!(outcome.creatures_seen, 1);
    assert_eq!(outcome.candidates_seen, 1);
    assert_eq!(outcome.aggro_starts, 0);
    assert!(outcome.commands.is_empty());
    let combat_target = {
        let guard = manager.read().unwrap();
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
fn legacy_creature_victim_sync_cas_rejects_same_guid_replacement_like_cpp() {
    let guid = test_creature_guid(91_045);
    let original = crate::map_manager::WorldCreature::new(
        guid,
        9001,
        Position::new(10.0, 10.0, 0.0, 0.0),
        100,
        80,
        10,
        10,
        20.0,
        1,
        35,
        0,
        0,
    );
    let sync = creature_melee_sync_state_for_test_like_cpp(&original, 10);
    let mut replacement = crate::map_manager::WorldCreature::new(
        guid,
        9001,
        Position::new(10.0, 10.0, 0.0, 0.0),
        100,
        80,
        10,
        10,
        20.0,
        1,
        35,
        0,
        0,
    );
    let before = (
        replacement.creature.unit().data().health,
        replacement.creature.unit().death_state(),
        replacement.creature.unit().health_state_revision_like_cpp(),
        replacement.creature.loot_lifecycle_revision_like_cpp(),
    );

    assert!(!apply_creature_melee_victim_sync_to_legacy_like_cpp(
        &mut replacement,
        &sync,
        wow_entities::game_time_secs_like_cpp(),
    ));

    assert_eq!(
        (
            replacement.creature.unit().data().health,
            replacement.creature.unit().death_state(),
            replacement.creature.unit().health_state_revision_like_cpp(),
            replacement.creature.loot_lifecycle_revision_like_cpp(),
        ),
        before
    );
    assert!(
        !replacement
            .creature
            .loot_authority_like_cpp()
            .shares_storage_like_cpp(&sync.identity.authority)
    );
}
#[test]
fn legacy_creature_victim_sync_cas_rejects_stale_aba_health_state_like_cpp() {
    let guid = test_creature_guid(91_046);
    let mut victim = crate::map_manager::WorldCreature::new(
        guid,
        9001,
        Position::new(10.0, 10.0, 0.0, 0.0),
        100,
        80,
        10,
        10,
        20.0,
        1,
        35,
        0,
        0,
    );
    let sync = creature_melee_sync_state_for_test_like_cpp(&victim, 10);
    victim.creature.unit_mut().set_health(90);
    victim.creature.unit_mut().set_health(100);
    let before = (
        victim.creature.unit().data().health,
        victim.creature.unit().death_state(),
        victim.creature.unit().health_state_revision_like_cpp(),
        victim.creature.loot_lifecycle_revision_like_cpp(),
    );

    assert!(!apply_creature_melee_victim_sync_to_legacy_like_cpp(
        &mut victim,
        &sync,
        wow_entities::game_time_secs_like_cpp(),
    ));

    assert_eq!(
        (
            victim.creature.unit().data().health,
            victim.creature.unit().death_state(),
            victim.creature.unit().health_state_revision_like_cpp(),
            victim.creature.loot_lifecycle_revision_like_cpp(),
        ),
        before
    );
    assert!(
        victim
            .creature
            .loot_authority_like_cpp()
            .shares_storage_like_cpp(&sync.identity.authority)
    );
}
#[test]
fn stale_legacy_creature_snapshot_cannot_replace_newer_canonical_health_like_cpp() {
    let canonical = shared_canonical_map_manager();
    let guid = test_creature_guid(91_049);
    add_canonical_test_creature_on_map(&canonical, guid, 9001, Position::ZERO, 0, 0, 0);
    let stale = canonical
        .lock()
        .unwrap()
        .find_map(0, 0)
        .unwrap()
        .map()
        .with_creature_like_cpp(guid, Clone::clone)
        .unwrap()
        .clone();
    let expected = {
        let mut guard = canonical.lock().unwrap();
        let creature = guard
            .find_map_mut(0, 0)
            .unwrap()
            .map_mut()
            .get_typed_creature_mut(guid)
            .unwrap();
        creature.unit_mut().set_health(90);
        (
            creature.unit().data().health,
            creature.unit().death_state(),
            creature.unit().health_state_revision_like_cpp(),
        )
    };

    assert!(sync_canonical_creature_entity_on_map_like_cpp(&canonical, 0, 0, stale).is_some());

    let guard = canonical.lock().unwrap();
    let creature = guard
        .find_map(0, 0)
        .unwrap()
        .map()
        .with_creature_like_cpp(guid, Clone::clone)
        .unwrap();
    assert_eq!(
        (
            creature.unit().data().health,
            creature.unit().death_state(),
            creature.unit().health_state_revision_like_cpp(),
        ),
        expected
    );
}
#[test]
fn stale_legacy_creature_snapshot_cannot_erase_canonical_death_lifecycle_like_cpp() {
    let canonical = shared_canonical_map_manager();
    let guid = test_creature_guid(91_105);
    let target = ObjectGuid::create_player(1, 91_106);
    add_canonical_test_creature_on_map(&canonical, guid, 9001, Position::ZERO, 0, 0, 0);
    let stale = {
        let mut guard = canonical.lock().unwrap();
        let creature = guard
            .find_map_mut(0, 0)
            .unwrap()
            .map_mut()
            .get_typed_creature_mut(guid)
            .unwrap();
        creature.enter_ai_combat(target);
        creature.clone()
    };
    let expected = {
        let mut guard = canonical.lock().unwrap();
        let creature = guard
            .find_map_mut(0, 0)
            .unwrap()
            .map_mut()
            .get_typed_creature_mut(guid)
            .unwrap();
        assert!(
            creature.apply_ai_damage_before_death_state_at_game_time_like_cpp(100, 1_000, 1_000,)
        );
        creature.set_death_state_runtime(wow_constants::DeathState::JustDied, 1_000);
        creature.unit_mut().set_health(0);
        creature.clone()
    };
    assert_eq!(
        stale.ai_ownership().state,
        wow_entities::CreatureAiState::InCombat
    );
    assert_eq!(
        expected.ai_ownership().state,
        wow_entities::CreatureAiState::Dead
    );
    assert!(expected.runtime_state().save_respawn_requested);
    assert!(expected.corpse_remove_time() > 1_000);
    assert!(expected.respawn_time() > 1_000);
    assert!(expected.loot_lifecycle_revision_like_cpp() > stale.loot_lifecycle_revision_like_cpp());

    assert!(sync_canonical_creature_entity_on_map_like_cpp(&canonical, 0, 0, stale).is_some());

    let guard = canonical.lock().unwrap();
    let stored = guard
        .find_map(0, 0)
        .unwrap()
        .map()
        .with_creature_like_cpp(guid, Clone::clone)
        .unwrap();
    assert_eq!(
        stored, expected,
        "a rejected stale snapshot must preserve every canonical kill hook"
    );
}
#[test]
fn legacy_creature_melee_tick_once_preserves_compatibility_player_damage_like_cpp() {
    use crate::map_manager::RuntimeTickOwner;
    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
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
    register_test_creature(&mut session, manager.clone(), creature_guid, 25);
    session
        .mutate_world_creature(creature_guid, |creature| {
            creature.enter_combat(player);
            creature.creature.ai_ownership_mut().swing_timer_ms = 0;
            creature.seed_runtime_rng_like_cpp(0x91_004);
        })
        .unwrap();
    manager
        .write()
        .unwrap()
        .set_tick_owner(RuntimeTickOwner::GlobalLegacy);

    let outcome = run_legacy_creature_melee_tick_once_like_cpp(&manager, Some(&canonical));

    assert!(!outcome.skipped_owner_not_global);
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
    let guard = manager.read().unwrap();
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
    use crate::map_manager::RuntimeTickOwner;

    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
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
                creature.creature.ai_ownership_mut().last_swing_ms = 0;
                creature.creature.ai_ownership_mut().swing_timer_ms = 0;
                creature.creature.ai_ownership_mut().min_damage = 1;
                creature.creature.ai_ownership_mut().max_damage = 1;
            })
            .unwrap();
    }
    manager
        .write()
        .unwrap()
        .set_tick_owner(RuntimeTickOwner::GlobalLegacy);

    let outcome = run_legacy_creature_melee_tick_once_like_cpp(&manager, Some(&canonical));

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

    let legacy_guard = manager.read().unwrap();
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
#[test]
fn legacy_creature_melee_tick_once_rejects_missing_canonical_attacker_before_side_effects_like_cpp()
{
    use crate::map_manager::RuntimeTickOwner;

    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    let player = ObjectGuid::create_player(1, 91_036);
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
        player.unit_mut().set_max_health(100);
        player.unit_mut().set_health(100);
    }

    let (mut session, _, _) = make_session();
    let creature_guid = test_creature_guid(91_037);
    let attacking_aura = wow_entities::AppliedAuraRef::new(91_038, creature_guid, 0, 0x1);
    let seed = 0x91_037;
    register_test_creature(&mut session, manager.clone(), creature_guid, 25);
    session
        .mutate_world_creature(creature_guid, |creature| {
            creature.enter_combat(player);
            creature.creature.ai_ownership_mut().last_swing_ms = 0;
            creature.creature.ai_ownership_mut().swing_timer_ms = 0;
            creature.seed_runtime_rng_like_cpp(seed);
            creature
                .creature
                .unit_mut()
                .subsystems_mut()
                .auras
                .register_applied_aura(
                    attacking_aura,
                    None,
                    wow_entities::SPELL_AURA_INTERRUPT_FLAG_ATTACKING_LIKE_CPP,
                    0,
                );
        })
        .unwrap();
    manager
        .write()
        .unwrap()
        .set_tick_owner(RuntimeTickOwner::GlobalLegacy);

    let outcome = run_legacy_creature_melee_tick_once_like_cpp(&manager, Some(&canonical));

    assert_eq!(outcome.swings_ready, 1);
    assert_eq!(outcome.melee_precondition_rejections, 1);
    assert_eq!(outcome.melee_outcomes_unrepresented, 0);
    assert_eq!(outcome.runtime_rng_authority_rejections, 0);
    assert_eq!(outcome.attacking_interrupt_auras_removed, 0);
    assert!(outcome.commands.is_empty());
    assert!(outcome.plan.events.is_empty());

    let mut expected_rng = StdRng::seed_from_u64(seed);
    let expected_first_roll = expected_rng.gen_range(0..=9_999_u32);
    let (authoritative, swing_timer_ms, aura_still_applied, actual_first_roll) = session
        .mutate_world_creature(creature_guid, |creature| {
            (
                creature.runtime_rng_authority_complete_like_cpp(),
                creature.creature.ai_ownership().swing_timer_ms,
                creature
                    .creature
                    .unit()
                    .subsystems()
                    .auras
                    .has_applied(attacking_aura),
                creature.random_creature_spell_hit_roll_like_cpp(),
            )
        })
        .unwrap();
    assert!(authoritative);
    assert_eq!(swing_timer_ms, 0);
    assert!(aura_still_applied);
    assert_eq!(actual_first_roll, Some(expected_first_roll));
}
