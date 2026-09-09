//! Session scenarios exercising the represented world entities responsibility.
//!
//! Split out of session_tests.rs under #626; assertions and registrations
//! are unchanged and the shared fixtures stay in the parent module.

use super::*;

#[test]
fn legacy_creature_melee_spell_serializes_cpp_base_miss_target() {
    use crate::map_manager::RuntimeTickOwner;

    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    let (mut session, _, _) = make_session();
    session.set_canonical_map_manager(Arc::clone(&canonical));
    let creature_guid = test_creature_guid(91_316);
    let victim_guid = ObjectGuid::create_player(1, 91_317);
    let spell_id = 15_691_i32;
    add_canonical_creature_spell_test_pair_like_cpp(&canonical, creature_guid, victim_guid);
    register_test_creature(&mut session, manager.clone(), creature_guid, 25);
    session
        .mutate_world_creature(creature_guid, |creature| {
            creature
                .creature
                .set_ai_identity_names_runtime_like_cpp("CombatAI", String::new());
            creature.creature.set_spell(0, spell_id as u32);
            creature.enter_combat(victim_guid);
        })
        .unwrap();
    manager
        .write()
        .unwrap()
        .set_tick_owner(RuntimeTickOwner::GlobalLegacy);
    let mut config = creature_ai_spell_test_config_like_cpp(
        creature_ai_test_spell_info_like_cpp(spell_id, 6, 0),
        false,
        30.0,
    );
    Arc::get_mut(config.spell_store.as_mut().unwrap())
        .unwrap()
        .insert_spell_misc_attributes_like_cpp(
            spell_id,
            represented_creature_spell_test_attributes_like_cpp(false),
        );

    let initialized =
        run_legacy_creature_spell_tick_once_like_cpp(&manager, Some(&canonical), &config);
    assert_eq!(initialized.schedules_initialized, 1);
    let miss_seed = (0_u64..10_000)
        .find(|seed| {
            let mut cpp_order = StdRng::seed_from_u64(*seed);
            let cpp_hit_roll = cpp_order.gen_range(0..=9_999_u32);
            let mut old_rust_order = StdRng::seed_from_u64(*seed);
            let _old_repeat_delay = old_rust_order.gen_range(6_000_u64..=12_000_u64);
            let old_hit_roll = old_rust_order.gen_range(0..=9_999_u32);
            cpp_hit_roll < 500 && old_hit_roll >= 500
        })
        .unwrap();
    let mut expected_rng = StdRng::seed_from_u64(miss_seed);
    assert!(expected_rng.gen_range(0..=9_999_u32) < 500);
    let expected_repeat_delay = expected_rng.gen_range(6_000_u64..=12_000_u64);
    let expected_next_roll = expected_rng.gen_range(0..=9_999_u32);
    session
        .mutate_world_creature(creature_guid, |creature| {
            creature.seed_runtime_rng_like_cpp(miss_seed);
            creature.backdate_runtime_clock_for_test(Duration::from_secs(60));
        })
        .unwrap();

    let cast = run_legacy_creature_spell_tick_once_like_cpp(&manager, Some(&canonical), &config);

    assert_eq!(cast.casts_ready, 1, "spell tick outcome: {cast:?}");
    assert_eq!(cast.spell_hits, 0);
    assert_eq!(cast.spell_misses, 1);
    assert_eq!(cast.canonical_cast_preconditions_passed, 1);
    assert_eq!(cast.plan.events.len(), 1);
    let (start, go) = decode_atomic_creature_spell_wire_pair_like_cpp(&cast.plan.events[0]);
    assert!(start.hit_targets.is_empty());
    assert!(start.miss_targets.is_empty());
    assert!(go.hit_targets.is_empty());
    assert_eq!(
        go.miss_targets,
        vec![(
            victim_guid,
            wow_packet::packets::spell::SpellMissReason::Miss as u8,
        )]
    );
    let (repeat_due_in_ms, actual_next_roll) = session
        .mutate_world_creature(creature_guid, |creature| {
            (
                creature.creature_spell_due_in_ms_for_test(0).unwrap(),
                creature.random_creature_spell_hit_roll_like_cpp(),
            )
        })
        .unwrap();
    assert!(
        expected_repeat_delay.abs_diff(repeat_due_in_ms) <= 250,
        "repeat delay must be drawn after the hit roll: expected {expected_repeat_delay}, got {repeat_due_in_ms}"
    );
    assert_eq!(actual_next_roll, Some(expected_next_roll));

    canonical
        .lock()
        .unwrap()
        .find_map_mut(0, 0)
        .unwrap()
        .map_mut()
        .get_typed_player_mut(victim_guid)
        .unwrap()
        .unit_mut()
        .subsystems_mut()
        .auras
        .invalidate_spell_hit_aura_authority_like_cpp();
    session
        .mutate_world_creature(creature_guid, |creature| {
            creature.seed_runtime_rng_like_cpp(0xA11_C105ED);
            creature.backdate_runtime_clock_for_test(Duration::from_secs(120));
            creature.creature.ai_ownership_mut().last_swing_ms = 0;
            creature.creature.ai_ownership_mut().swing_timer_ms = 0;
            assert!(creature.can_swing());
        })
        .unwrap();

    let unrepresented =
        run_legacy_creature_spell_tick_once_like_cpp(&manager, Some(&canonical), &config);

    assert_eq!(unrepresented.casts_ready, 0);
    assert_eq!(unrepresented.spell_hit_results_unrepresented, 1);
    assert_eq!(unrepresented.runtime_rng_authority_rejections, 1);
    assert!(unrepresented.plan.events.is_empty());
    let (repeat_due_in_ms, rng_is_authoritative, actual_next_roll) = session
        .mutate_world_creature(creature_guid, |creature| {
            (
                creature.creature_spell_due_in_ms_for_test(0),
                creature.runtime_rng_authority_complete_like_cpp(),
                creature.random_creature_spell_hit_roll_like_cpp(),
            )
        })
        .unwrap();
    assert_eq!(repeat_due_in_ms, None);
    assert!(!rng_is_authoritative);
    assert_eq!(actual_next_roll, None);
    assert!(
        !session
            .mutate_world_creature(creature_guid, |creature| creature.can_swing())
            .unwrap(),
        "a fail-closed hit result still follows C++ Spell::ResetCombatTimers"
    );

    session
        .mutate_world_creature(creature_guid, |creature| {
            let _ = creature.reset_combat();
            creature.enter_combat(victim_guid);
            creature.seed_runtime_rng_like_cpp(0xBEEF);
        })
        .unwrap();
    let next_engagement =
        run_legacy_creature_spell_tick_once_like_cpp(&manager, Some(&canonical), &config);
    assert_eq!(next_engagement.schedules_initialized, 1);
    assert_eq!(next_engagement.runtime_rng_authority_rejections, 1);
    assert_eq!(next_engagement.casts_ready, 0);
    assert!(next_engagement.plan.events.is_empty());
    let (rng_is_authoritative, due) = session
        .mutate_world_creature(creature_guid, |creature| {
            (
                creature.runtime_rng_authority_complete_like_cpp(),
                creature.creature_spell_due_in_ms_for_test(0),
            )
        })
        .unwrap();
    assert!(
        !rng_is_authoritative,
        "a new engagement must not clear the tombstone"
    );
    assert_eq!(due, None);

    session
        .mutate_world_creature(creature_guid, |creature| {
            creature.creature.ai_ownership_mut().last_swing_ms = 0;
            creature.creature.ai_ownership_mut().swing_timer_ms = 0;
            assert!(creature.can_swing());
        })
        .unwrap();
    let melee = run_legacy_creature_melee_tick_once_like_cpp(&manager, Some(&canonical));
    assert_eq!(melee.runtime_rng_authority_rejections, 0);
    assert_eq!(melee.melee_outcomes_unrepresented, 1);
    assert_eq!(melee.swings_ready, 1);
    assert_eq!(melee.canonical_hits, 1);
    assert_eq!(melee.commands.len(), 1);
    assert!(melee.plan.events.is_empty());
}
#[test]
fn legacy_creature_aggro_hit_roll_precedes_later_slot_schedule_rng_like_cpp() {
    use crate::map_manager::RuntimeTickOwner;

    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    let (mut session, _, _) = make_session();
    let creature_guid = test_creature_guid(91_318);
    let victim_guid = ObjectGuid::create_player(1, 91_319);
    let aggro_spell_id = 70_104_i32;
    let combat_spell_id = 70_105_i32;
    add_canonical_creature_spell_test_pair_like_cpp(&canonical, creature_guid, victim_guid);
    register_test_creature_mirrored_like_cpp(
        &mut session,
        manager.clone(),
        &canonical,
        creature_guid,
        25,
    );
    session
        .mutate_world_creature(creature_guid, |creature| {
            creature
                .creature
                .set_ai_identity_names_runtime_like_cpp("CombatAI", String::new());
            creature.creature.set_spell(0, aggro_spell_id as u32);
            creature.creature.set_spell(1, combat_spell_id as u32);
            creature.enter_combat(victim_guid);
        })
        .unwrap();
    manager
        .write()
        .unwrap()
        .set_tick_owner(RuntimeTickOwner::GlobalLegacy);

    let aggro_spell = creature_ai_test_spell_info_like_cpp(aggro_spell_id, 6, 0);
    let combat_spell = creature_ai_test_spell_info_like_cpp(combat_spell_id, 6, 0);
    let mut config = creature_ai_spell_test_config_like_cpp(aggro_spell, true, 30.0);
    let mut aggro_attributes = represented_creature_spell_test_attributes_like_cpp(false);
    aggro_attributes[0] |= wow_data::spell::attributes::SPELL_ATTR0_PASSIVE;
    let spell_store = Arc::get_mut(config.spell_store.as_mut().unwrap()).unwrap();
    spell_store.insert_spell_misc_attributes_like_cpp(aggro_spell_id, aggro_attributes);
    spell_store.insert(combat_spell_id, combat_spell);
    spell_store.insert_spell_misc_attributes_like_cpp(
        combat_spell_id,
        represented_creature_spell_test_attributes_like_cpp(true),
    );
    spell_store.insert_spell_hit_metadata_for_difficulty_like_cpp(
        combat_spell_id,
        0,
        wow_data::SpellHitMetadataLikeCpp {
            category_id: 0,
            charge_category_id: 0,
            defense_type: 2,
            spell_mechanic: 0,
            school_mask: 0x01,
            effect_mechanics: BTreeMap::from([(0, 0)]),
        },
    );
    let mut aggro_misc = spell_misc_entry_like_cpp(8_301, aggro_spell_id as u32, 71);
    aggro_misc.attributes = aggro_attributes.map(|attribute| attribute as i32);
    let combat_misc = spell_misc_entry_like_cpp(8_302, combat_spell_id as u32, 71);
    config.spell_misc_store = Some(Arc::new(wow_data::SpellMiscStore::from_entries([
        aggro_misc,
        combat_misc,
    ])));
    config.spell_cooldowns_store = Some(Arc::new(wow_data::SpellCooldownsStore::from_entries([
        wow_data::SpellCooldownsEntry {
            id: 8_303,
            difficulty_id: 0,
            recovery_time: 6_000,
            spell_id: aggro_spell_id as u32,
            ..Default::default()
        },
        wow_data::SpellCooldownsEntry {
            id: 8_304,
            difficulty_id: 0,
            recovery_time: 6_000,
            spell_id: combat_spell_id as u32,
            ..Default::default()
        },
    ])));
    let visual = |id, spell_id| wow_data::SpellXSpellVisualEntry {
        id,
        difficulty_id: 0,
        spell_visual_id: id + 100,
        probability: 1.0,
        flags: 0,
        priority: 0,
        spell_icon_file_id: 0,
        active_icon_file_id: 0,
        viewer_unit_condition_id: 0,
        viewer_player_condition_id: 0,
        caster_unit_condition_id: 0,
        caster_player_condition_id: 0,
        spell_id,
    };
    config.spell_x_spell_visual_store =
        Some(Arc::new(wow_data::SpellXSpellVisualStore::from_entries([
            visual(8_305, aggro_spell_id as u32),
            visual(8_306, combat_spell_id as u32),
        ])));

    let seed = (0_u64..10_000)
        .find(|seed| {
            let mut cpp_order = StdRng::seed_from_u64(*seed);
            let cpp_hit_roll = cpp_order.gen_range(0..=9_999_u32);
            let mut old_rust_order = StdRng::seed_from_u64(*seed);
            let _old_initial_delay = old_rust_order.gen_range(6_000_u64..=12_000_u64);
            let old_hit_roll = old_rust_order.gen_range(0..=9_999_u32);
            cpp_hit_roll < 500 && old_hit_roll >= 500
        })
        .unwrap();
    let mut expected_rng = StdRng::seed_from_u64(seed);
    assert!(expected_rng.gen_range(0..=9_999_u32) < 500);
    let expected_initial_delay = expected_rng.gen_range(6_000_u64..=12_000_u64);
    let expected_next_roll = expected_rng.gen_range(0..=9_999_u32);
    session
        .mutate_world_creature(creature_guid, |creature| {
            creature.seed_runtime_rng_like_cpp(seed);
        })
        .unwrap();

    let engaged = run_legacy_creature_spell_tick_once_like_cpp(&manager, Some(&canonical), &config);

    assert_eq!(engaged.schedules_initialized, 1);
    assert_eq!(engaged.casts_ready, 1, "spell tick outcome: {engaged:?}");
    assert_eq!(engaged.spell_hits, 0);
    assert_eq!(engaged.spell_misses, 1);
    assert_eq!(engaged.plan.events.len(), 1);
    let (initial_due_in_ms, actual_next_roll) = session
        .mutate_world_creature(creature_guid, |creature| {
            (
                creature.creature_spell_due_in_ms_for_test(1).unwrap(),
                creature.random_creature_spell_hit_roll_like_cpp(),
            )
        })
        .unwrap();
    assert!(
        expected_initial_delay.abs_diff(initial_due_in_ms) <= 250,
        "slot-one delay must be drawn after the slot-zero Aggro hit roll"
    );
    assert_eq!(actual_next_roll, Some(expected_next_roll));
}
#[test]
fn legacy_creature_unrepresented_aggro_spell_stops_later_slots_and_melee_like_cpp() {
    use crate::map_manager::RuntimeTickOwner;

    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    let (mut session, _, _) = make_session();
    let creature_guid = test_creature_guid(91_330);
    let victim_guid = ObjectGuid::create_player(1, 91_331);
    let aggro_spell_id = 70_210_i32;
    let combat_spell_id = 70_211_i32;
    add_canonical_creature_spell_test_pair_like_cpp(&canonical, creature_guid, victim_guid);
    register_test_creature(&mut session, manager.clone(), creature_guid, 25);
    session
        .mutate_world_creature(creature_guid, |creature| {
            creature
                .creature
                .set_ai_identity_names_runtime_like_cpp("CombatAI", String::new());
            creature.creature.set_spell(0, aggro_spell_id as u32);
            creature.creature.set_spell(1, combat_spell_id as u32);
            creature.enter_combat(victim_guid);
            creature.creature.ai_ownership_mut().last_swing_ms = 0;
            creature.creature.ai_ownership_mut().swing_timer_ms = 0;
            assert!(creature.can_swing());
        })
        .unwrap();
    manager
        .write()
        .unwrap()
        .set_tick_owner(RuntimeTickOwner::GlobalLegacy);

    let mut aggro_spell = creature_ai_test_spell_info_like_cpp(aggro_spell_id, 6, 0);
    aggro_spell.effect_type = wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA;
    aggro_spell.aura_type = Some(wow_data::spell::aura_types::SPELL_AURA_MOD_DAMAGE_DONE);
    aggro_spell.effects[0].effect = wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA;
    aggro_spell.effects[0].effect_aura = wow_data::spell::aura_types::SPELL_AURA_MOD_DAMAGE_DONE;
    let combat_spell = creature_ai_test_spell_info_like_cpp(combat_spell_id, 6, 0);
    let mut config = creature_ai_spell_test_config_like_cpp(aggro_spell, true, 30.0);
    let spell_store = Arc::get_mut(config.spell_store.as_mut().unwrap()).unwrap();
    spell_store.insert(combat_spell_id, combat_spell);
    spell_store.insert_spell_misc_attributes_like_cpp(
        combat_spell_id,
        represented_creature_spell_test_attributes_like_cpp(true),
    );
    spell_store.insert_spell_hit_metadata_for_difficulty_like_cpp(
        combat_spell_id,
        0,
        wow_data::SpellHitMetadataLikeCpp {
            category_id: 0,
            charge_category_id: 0,
            defense_type: 2,
            spell_mechanic: 0,
            school_mask: 0x01,
            effect_mechanics: BTreeMap::from([(0, 0)]),
        },
    );
    let mut aggro_misc = spell_misc_entry_like_cpp(8_401, aggro_spell_id as u32, 71);
    aggro_misc.attributes[0] |= wow_data::spell::attributes::SPELL_ATTR0_PASSIVE as i32;
    let combat_misc = spell_misc_entry_like_cpp(8_402, combat_spell_id as u32, 71);
    config.spell_misc_store = Some(Arc::new(wow_data::SpellMiscStore::from_entries([
        aggro_misc,
        combat_misc,
    ])));
    config.spell_cooldowns_store = Some(Arc::new(wow_data::SpellCooldownsStore::from_entries([
        wow_data::SpellCooldownsEntry {
            id: 8_403,
            difficulty_id: 0,
            recovery_time: 6_000,
            spell_id: aggro_spell_id as u32,
            ..Default::default()
        },
        wow_data::SpellCooldownsEntry {
            id: 8_404,
            difficulty_id: 0,
            recovery_time: 6_000,
            spell_id: combat_spell_id as u32,
            ..Default::default()
        },
    ])));

    let first = run_legacy_creature_spell_tick_once_like_cpp(&manager, Some(&canonical), &config);
    assert_eq!(first.schedules_initialized, 1);
    assert_eq!(first.spell_effects_unrepresented, 1);
    assert_eq!(first.casts_ready, 0);
    assert!(first.plan.events.is_empty());
    let (later_due, can_swing) = session
        .mutate_world_creature(creature_guid, |creature| {
            (
                creature.creature_spell_due_in_ms_for_test(1),
                creature.can_swing(),
            )
        })
        .unwrap();
    assert_eq!(
        later_due, None,
        "JustEngagedWith stops after the failed cast"
    );
    assert!(
        !can_swing,
        "CastSpell rearms BASE_ATTACK even when CheckCast fails"
    );

    let next = run_legacy_creature_spell_tick_once_like_cpp(&manager, Some(&canonical), &config);
    assert_eq!(next.casts_ready, 0);
    assert!(next.plan.events.is_empty());
    assert_eq!(
        session
            .mutate_world_creature(creature_guid, |creature| {
                creature.creature_spell_due_in_ms_for_test(1)
            })
            .unwrap(),
        None
    );
}
#[test]
fn legacy_creature_combat_ai_rearms_raw_schedule_but_obeys_category_cooldown_like_cpp() {
    use crate::map_manager::RuntimeTickOwner;

    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    let (mut session, _, _) = make_session();
    let creature_guid = test_creature_guid(91_332);
    let victim_guid = ObjectGuid::create_player(1, 91_333);
    let spell_id = 70_212_i32;
    let category_id = 77_u32;
    add_canonical_creature_spell_test_pair_like_cpp(&canonical, creature_guid, victim_guid);
    register_test_creature_mirrored_like_cpp(
        &mut session,
        manager.clone(),
        &canonical,
        creature_guid,
        25,
    );
    // Registration establishes the shared incarnation exactly as production
    // does. Detach the session mirror afterwards so the later legacy
    // mutations in this test cannot overwrite the canonical spell history
    // whose cooldown deadlines it asserts.
    session.canonical_map_manager = None;
    session
        .mutate_world_creature(creature_guid, |creature| {
            creature
                .creature
                .set_ai_identity_names_runtime_like_cpp("CombatAI", String::new());
            creature.creature.set_spell(0, spell_id as u32);
            creature.enter_combat(victim_guid);
        })
        .unwrap();
    manager
        .write()
        .unwrap()
        .set_tick_owner(RuntimeTickOwner::GlobalLegacy);

    let mut config = creature_ai_spell_test_config_like_cpp(
        creature_ai_test_spell_info_like_cpp(spell_id, 6, 0),
        false,
        30.0,
    );
    let attributes = represented_creature_spell_test_attributes_like_cpp(false);
    let spell_store = Arc::get_mut(config.spell_store.as_mut().unwrap()).unwrap();
    spell_store.insert_spell_misc_attributes_like_cpp(spell_id, attributes);
    spell_store.insert_spell_hit_metadata_for_difficulty_like_cpp(
        spell_id,
        0,
        wow_data::SpellHitMetadataLikeCpp {
            category_id,
            charge_category_id: 0,
            defense_type: 2,
            spell_mechanic: 0,
            school_mask: 0x01,
            effect_mechanics: BTreeMap::from([(0, 0)]),
        },
    );
    let mut misc = spell_misc_entry_like_cpp(8_405, spell_id as u32, 71);
    misc.attributes = attributes.map(|attribute| attribute as i32);
    config.spell_misc_store = Some(Arc::new(wow_data::SpellMiscStore::from_entries([misc])));
    config.spell_cooldowns_store = Some(Arc::new(wow_data::SpellCooldownsStore::from_entries([
        wow_data::SpellCooldownsEntry {
            id: 8_406,
            difficulty_id: 0,
            category_recovery_time: 60_000,
            recovery_time: 0,
            start_recovery_time: 0,
            spell_id: spell_id as u32,
        },
    ])));
    config.spell_category_store = Some(Arc::new(SpellCategoryStore::from_entries([
        wow_data::SpellCategoryEntry {
            id: category_id,
            name: String::new(),
            flags: 0,
            uses_per_week: 0,
            max_charges: 0,
            charge_recovery_time: 0,
            type_mask: 0,
        },
    ])));

    let seed = (0_u64..10_000)
        .find(|seed| {
            let mut rng = StdRng::seed_from_u64(*seed);
            let _initial_delay = rng.gen_range(5_000_u64..=10_000_u64);
            rng.gen_range(0..=9_999_u32) < 500
        })
        .unwrap();
    session
        .mutate_world_creature(creature_guid, |creature| {
            creature.seed_runtime_rng_like_cpp(seed);
        })
        .unwrap();

    let initialized =
        run_legacy_creature_spell_tick_once_like_cpp(&manager, Some(&canonical), &config);
    assert_eq!(initialized.schedules_initialized, 1);
    assert_eq!(initialized.casts_ready, 0);
    session
        .mutate_world_creature(creature_guid, |creature| {
            creature.backdate_runtime_clock_for_test(Duration::from_secs(20));
        })
        .unwrap();

    let first = run_legacy_creature_spell_tick_once_like_cpp(&manager, Some(&canonical), &config);
    assert_eq!(first.casts_ready, 1, "spell tick outcome: {first:?}");
    assert_eq!(first.spell_misses, 1);
    assert_eq!(first.canonical_cast_cooldown_rejections, 0);
    assert_eq!(first.plan.events.len(), 1);
    let repeat_due = session
        .mutate_world_creature(creature_guid, |creature| {
            creature.creature_spell_due_in_ms_for_test(0).unwrap()
        })
        .unwrap();
    assert!((5_000..=10_000).contains(&repeat_due));
    let cooldown = canonical
        .lock()
        .unwrap()
        .find_map(0, 0)
        .unwrap()
        .map()
        .with_creature_like_cpp(creature_guid, Clone::clone)
        .unwrap()
        .unit()
        .subsystems()
        .spells
        .history
        .cooldown(spell_id as u32)
        .unwrap();
    assert_eq!(cooldown.category_id, category_id);
    assert!(cooldown.category_end_ms >= 79_000);

    session
        .mutate_world_creature(creature_guid, |creature| {
            creature.backdate_runtime_clock_for_test(Duration::from_secs(40));
        })
        .unwrap();
    let blocked = run_legacy_creature_spell_tick_once_like_cpp(&manager, Some(&canonical), &config);
    assert_eq!(blocked.casts_ready, 0, "spell tick outcome: {blocked:?}");
    assert_eq!(blocked.canonical_cast_cooldown_rejections, 1);
    assert_eq!(blocked.spell_misses, 0);
    assert!(blocked.plan.events.is_empty());
    assert!(
        session
            .mutate_world_creature(creature_guid, |creature| {
                creature.creature_spell_due_in_ms_for_test(0)
            })
            .unwrap()
            .is_some(),
        "CombatAI rearms its raw EventMap cadence after CheckCast rejects"
    );
}
#[test]
fn legacy_creature_combat_ai_no_attack_miss_consumes_roll_then_tombstones_launch_rng_like_cpp() {
    use crate::map_manager::RuntimeTickOwner;

    let manager = shared_map_manager();
    let (mut session, _, _) = make_session();
    let creature_guid = test_creature_guid(91_200);
    let victim_guid = ObjectGuid::create_player(1, 91_201);
    let spell_id = 70_001_i32;
    let canonical = shared_canonical_map_manager();
    add_canonical_creature_spell_test_pair_like_cpp(&canonical, creature_guid, victim_guid);
    register_test_creature_mirrored_like_cpp(
        &mut session,
        manager.clone(),
        &canonical,
        creature_guid,
        25,
    );
    session
        .mutate_world_creature(creature_guid, |creature| {
            creature
                .creature
                .set_ai_identity_names_runtime_like_cpp("CombatAI", String::new());
            creature
                .creature
                .set_spell(0, u32::try_from(spell_id).unwrap());
            creature.enter_combat(victim_guid);
        })
        .unwrap();
    manager
        .write()
        .unwrap()
        .set_tick_owner(RuntimeTickOwner::GlobalLegacy);
    let config = creature_ai_spell_test_config_like_cpp(
        creature_ai_test_spell_info_like_cpp(spell_id, 6, 0),
        false,
        30.0,
    );

    let initialized =
        run_legacy_creature_spell_tick_once_like_cpp(&manager, Some(&canonical), &config);
    assert_eq!(initialized.schedules_initialized, 1);
    assert_eq!(initialized.casts_ready, 0);
    assert!(initialized.plan.events.is_empty());
    let first_due_in_ms = session
        .mutate_world_creature(creature_guid, |creature| {
            creature.creature_spell_due_in_ms_for_test(0).unwrap()
        })
        .unwrap();
    assert!((6_000..=12_000).contains(&first_due_in_ms));

    session
        .mutate_world_creature(creature_guid, |creature| {
            creature.backdate_runtime_clock_for_test(Duration::from_millis(
                first_due_in_ms.saturating_sub(1),
            ));
        })
        .unwrap();
    let before_minimum =
        run_legacy_creature_spell_tick_once_like_cpp(&manager, Some(&canonical), &config);
    assert_eq!(before_minimum.casts_ready, 0);

    session
        .mutate_world_creature(creature_guid, |creature| {
            creature.backdate_runtime_clock_for_test(Duration::from_secs(60));
            creature
                .creature
                .unit_mut()
                .add_unit_state(UnitState::CASTING.bits());
        })
        .unwrap();
    let casting = run_legacy_creature_spell_tick_once_like_cpp(&manager, Some(&canonical), &config);
    assert_eq!(casting.unit_state_casting_skips, 1);
    assert_eq!(casting.casts_ready, 0);
    assert!(casting.plan.events.is_empty());
    session
        .mutate_world_creature(creature_guid, |creature| {
            creature
                .creature
                .unit_mut()
                .clear_unit_state(UnitState::CASTING.bits());
        })
        .unwrap();
    let guaranteed_seed = (0_u64..10_000)
        .find(|seed| {
            let mut cpp_order = StdRng::seed_from_u64(*seed);
            cpp_order.gen_range(0..=9_999_u32) < 500
        })
        .unwrap();
    session
        .mutate_world_creature(creature_guid, |creature| {
            creature.seed_runtime_rng_like_cpp(guaranteed_seed);
        })
        .unwrap();
    let cast = run_legacy_creature_spell_tick_once_like_cpp(&manager, Some(&canonical), &config);
    assert_eq!(cast.casts_ready, 1, "spell tick outcome: {cast:?}");
    assert_eq!(cast.canonical_cast_preconditions_passed, 1);
    assert_eq!(cast.spell_hits, 1);
    assert_eq!(cast.runtime_rng_authority_rejections, 1);
    assert_eq!(cast.plan.events.len(), 1);
    let (start, go) = decode_atomic_creature_spell_wire_pair_like_cpp(&cast.plan.events[0]);
    assert_eq!(
        start.opcode,
        wow_constants::ServerOpcodes::SpellStart as u16
    );
    assert_eq!(go.opcode, wow_constants::ServerOpcodes::SpellGo as u16);
    assert_eq!(start.caster, creature_guid);
    assert_eq!(start.caster_unit, creature_guid);
    assert_eq!(start.cast_id, go.cast_id);
    assert_eq!(start.cast_id.high_type(), HighGuid::Cast);
    assert_eq!(start.cast_id.sub_type(), SPELL_CAST_SOURCE_NORMAL_LIKE_CPP);
    assert_eq!(start.cast_id.map_id(), 0);
    assert_eq!(start.cast_id.entry(), u32::try_from(spell_id).unwrap());
    assert_ne!(start.cast_id.counter(), 0);
    assert_eq!(start.original_cast_id, ObjectGuid::EMPTY);
    assert_eq!(go.original_cast_id, ObjectGuid::EMPTY);
    assert_eq!(start.spell_id, spell_id);
    assert_eq!(go.spell_id, spell_id);
    assert_eq!(start.spell_x_spell_visual_id, 8_003);
    assert_eq!(go.spell_x_spell_visual_id, 8_003);
    assert_eq!(start.cast_flags, 0x0000_0002);
    assert_eq!(go.cast_flags, 0x0004_0100);
    assert_eq!(start.cast_flags_ex, 0);
    assert_eq!(go.cast_flags_ex, 0);
    assert_eq!(start.cast_time_ms, 0);
    assert_eq!(start.target_flags, 0x2);
    assert_eq!(go.target_flags, 0x2);
    assert_eq!(start.target_unit, victim_guid);
    assert_eq!(go.target_unit, victim_guid);
    assert!(start.hit_targets.is_empty());
    assert!(start.miss_targets.is_empty());
    assert_eq!(go.hit_targets, vec![victim_guid]);
    assert!(go.miss_targets.is_empty());
    let first_cast_id = start.cast_id;
    assert_eq!(
        canonical
            .lock()
            .unwrap()
            .find_map(0, 0)
            .unwrap()
            .map()
            .get_typed_player(victim_guid)
            .unwrap()
            .unit()
            .data()
            .health,
        100,
        "M2.6 emits the cast wire without fabricating M3.2 damage"
    );

    let (repeat_due_in_ms, rng_is_authoritative, actual_next_roll) = session
        .mutate_world_creature(creature_guid, |creature| {
            (
                creature.creature_spell_due_in_ms_for_test(0),
                creature.runtime_rng_authority_complete_like_cpp(),
                creature.random_creature_spell_hit_roll_like_cpp(),
            )
        })
        .unwrap();
    assert_eq!(repeat_due_in_ms, None);
    assert!(!rng_is_authoritative);
    assert_eq!(actual_next_roll, None);

    let no_retry =
        run_legacy_creature_spell_tick_once_like_cpp(&manager, Some(&canonical), &config);
    assert_eq!(no_retry.casts_ready, 0);
    assert!(no_retry.plan.events.is_empty());
    assert_eq!(
        first_cast_id.high_type(),
        HighGuid::Cast,
        "the one accepted HIT remains published before the launch-RNG tombstone"
    );
}
