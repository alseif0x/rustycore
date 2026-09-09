//! Session scenarios exercising the represented world entities responsibility.
//!
//! Split out of session_tests.rs under #626; assertions and registrations
//! are unchanged and the shared fixtures stay in the parent module.

use super::*;

#[test]
fn legacy_creature_combat_ai_successful_cast_resets_swing_before_same_tick_melee_like_cpp() {
    use crate::map_manager::RuntimeTickOwner;

    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    let (mut session, _, _) = make_session();
    let creature_guid = test_creature_guid(91_300);
    let victim_guid = ObjectGuid::create_player(1, 91_301);
    let spell_id = 15_691_i32;
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
    let mut spell = creature_ai_test_spell_info_like_cpp(spell_id, 6, 0);
    spell
        .power_costs
        .push(wow_data::spell::SpellPowerCostInfoLikeCpp {
            order_index: 0,
            power_type: 3,
            mana_cost: 0,
            mana_cost_per_level: 0,
            mana_per_second: 0,
            power_cost_pct: 0.0,
            power_cost_max_pct: 0.0,
            power_pct_per_second: 0.0,
            required_aura_spell_id: 0,
            optional_cost: 0,
        });
    let mut config = creature_ai_spell_test_config_like_cpp(spell, false, 30.0);
    Arc::get_mut(config.spell_store.as_mut().unwrap())
        .unwrap()
        .insert_spell_misc_attributes_like_cpp(
            spell_id,
            represented_creature_spell_test_attributes_like_cpp(false),
        );

    let initialized =
        run_legacy_creature_spell_tick_once_like_cpp(&manager, Some(&canonical), &config);
    assert_eq!(initialized.schedules_initialized, 1);
    assert_eq!(initialized.casts_ready, 0);
    session
        .mutate_world_creature(creature_guid, |creature| {
            creature.backdate_runtime_clock_for_test(Duration::from_secs(60));
            creature.creature.ai_ownership_mut().last_swing_ms = 0;
            creature.creature.ai_ownership_mut().swing_timer_ms = 0;
            assert!(creature.can_swing());
        })
        .unwrap();

    let cast = run_legacy_creature_spell_tick_once_like_cpp(&manager, Some(&canonical), &config);

    assert_eq!(cast.casts_ready, 1, "spell tick outcome: {cast:?}");
    assert_eq!(cast.canonical_cast_preconditions_passed, 1);
    assert_eq!(cast.plan.events.len(), 1);
    let (start, go) = decode_atomic_creature_spell_wire_pair_like_cpp(&cast.plan.events[0]);
    assert_eq!(start.opcode, ServerOpcodes::SpellStart as u16);
    assert_eq!(go.opcode, ServerOpcodes::SpellGo as u16);
    assert_eq!(start.spell_id, spell_id);
    assert_eq!(go.spell_id, spell_id);
    assert!(
        !session
            .mutate_world_creature(creature_guid, |creature| creature.can_swing())
            .unwrap(),
        "successful untriggered 15691 must reset BASE_ATTACK before melee"
    );

    let melee = run_legacy_creature_melee_tick_once_like_cpp(&manager, Some(&canonical));
    assert_eq!(melee.swings_ready, 0);
    assert_eq!(melee.canonical_hits, 0);
    assert!(melee.commands.is_empty());

    let mut implicit_cost_attributes = represented_creature_spell_test_attributes_like_cpp(false);
    implicit_cost_attributes[1] = 0x0000_0002; // SPELL_ATTR1_USE_ALL_MANA
    Arc::get_mut(config.spell_store.as_mut().unwrap())
        .unwrap()
        .insert_spell_misc_attributes_like_cpp(spell_id, implicit_cost_attributes);
    session
        .mutate_world_creature(creature_guid, |creature| {
            creature.backdate_runtime_clock_for_test(Duration::from_secs(120));
        })
        .unwrap();
    let implicit_cost_rejected =
        run_legacy_creature_spell_tick_once_like_cpp(&manager, Some(&canonical), &config);
    assert_eq!(implicit_cost_rejected.casts_ready, 0);
    assert_eq!(implicit_cost_rejected.spell_effects_unrepresented, 1);
    assert!(implicit_cost_rejected.plan.events.is_empty());

    let mut weapon_speed_cost_attributes =
        represented_creature_spell_test_attributes_like_cpp(false);
    weapon_speed_cost_attributes[4] = 0x0000_0400; // SPELL_ATTR4_WEAPON_SPEED_COST_SCALING
    Arc::get_mut(config.spell_store.as_mut().unwrap())
        .unwrap()
        .insert_spell_misc_attributes_like_cpp(spell_id, weapon_speed_cost_attributes);
    session
        .mutate_world_creature(creature_guid, |creature| {
            creature.backdate_runtime_clock_for_test(Duration::from_secs(180));
        })
        .unwrap();
    let weapon_speed_cost_rejected =
        run_legacy_creature_spell_tick_once_like_cpp(&manager, Some(&canonical), &config);
    assert_eq!(weapon_speed_cost_rejected.casts_ready, 0);
    assert_eq!(weapon_speed_cost_rejected.spell_effects_unrepresented, 1);
    assert!(weapon_speed_cost_rejected.plan.events.is_empty());

    Arc::get_mut(config.spell_store.as_mut().unwrap())
        .unwrap()
        .insert_spell_misc_attributes_like_cpp(
            spell_id,
            represented_creature_spell_test_attributes_like_cpp(false),
        );
    session
        .mutate_world_creature(creature_guid, |creature| {
            creature
                .creature
                .unit_mut()
                .subsystems_mut()
                .auras
                .register_applied_aura_type_like_cpp(
                    wow_entities::AppliedAuraRef::new(91_999, creature_guid, 0, 1),
                    63, // SPELL_AURA_MOD_ADDITIONAL_POWER_COST
                );
            creature.backdate_runtime_clock_for_test(Duration::from_secs(240));
        })
        .unwrap();
    let aura_cost_rejected =
        run_legacy_creature_spell_tick_once_like_cpp(&manager, Some(&canonical), &config);
    assert_eq!(aura_cost_rejected.casts_ready, 0);
    assert_eq!(aura_cost_rejected.spell_effects_unrepresented, 1);
    assert!(aura_cost_rejected.plan.events.is_empty());
}
#[test]
fn legacy_creature_spell_tick_rejects_same_guid_caster_replacement_like_cpp() {
    use crate::map_manager::RuntimeTickOwner;

    // The canonical tick can replace a creature between the lifecycle and
    // spell phases while the legacy map still holds the engaged one. The
    // plan carries the incarnation it was captured from, so the replacement
    // must neither cast nor mutate any state.
    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    let (mut session, _, _) = make_session();
    let creature_guid = test_creature_guid(91_324);
    let victim_guid = ObjectGuid::create_player(1, 91_325);
    let spell_id = 15_691_i32;
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
    assert_eq!(initialized.casts_ready, 0);
    session
        .mutate_world_creature(creature_guid, |creature| {
            creature.backdate_runtime_clock_for_test(Duration::from_secs(60));
            creature.creature.ai_ownership_mut().last_swing_ms = 0;
            creature.creature.ai_ownership_mut().swing_timer_ms = 0;
            assert!(creature.can_swing());
        })
        .unwrap();

    let legacy_health_authority = manager
        .read()
        .unwrap()
        .find_creature(0, 0, creature_guid)
        .unwrap()
        .creature
        .unit()
        .health_state_revision_authority_like_cpp();
    let mut replacement = crate::map_manager::WorldCreature::new(
        creature_guid,
        9001,
        Position::new(10.0, 10.0, 0.0, 0.0),
        25,
        80,
        3,
        5,
        20.0,
        1,
        35,
        0,
        0,
    )
    .creature;
    replacement.unit_mut().world_mut().set_map(0, 0).unwrap();
    replacement
        .unit_mut()
        .world_mut()
        .object_mut()
        .add_to_world();
    replacement.unit_mut().set_faction(14);
    replacement
        .unit_mut()
        .subsystems_mut()
        .auras
        .set_spell_cast_log_aura_authority_inert_like_cpp(true);
    assert!(
        !replacement
            .unit()
            .shares_health_state_revision_authority_like_cpp(&legacy_health_authority),
        "the replacement must be a distinct incarnation"
    );
    canonical
        .lock()
        .unwrap()
        .find_map_mut(0, 0)
        .unwrap()
        .map_mut()
        .insert_map_object_record(wow_entities::MapObjectRecord::new_creature(replacement).unwrap())
        .unwrap();

    let rejected =
        run_legacy_creature_spell_tick_once_like_cpp(&manager, Some(&canonical), &config);

    assert_eq!(
        rejected.caster_incarnation_rejections, 1,
        "spell tick outcome: {rejected:?}"
    );
    assert_eq!(rejected.casts_ready, 0);
    assert_eq!(rejected.canonical_cast_preconditions_passed, 0);
    assert_eq!(rejected.spell_hits, 0);
    assert_eq!(rejected.spell_misses, 0);
    assert!(rejected.plan.events.is_empty());
    assert!(
        canonical
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
            .is_none(),
        "no cooldown may start on the replacement"
    );
    assert!(
        session
            .mutate_world_creature(creature_guid, |creature| creature.can_swing())
            .unwrap(),
        "the stale legacy creature keeps its swing because no cast happened"
    );
}
#[test]
fn legacy_creature_combat_ai_mixed_noninstant_template_suppresses_15691_like_cpp() {
    use crate::map_manager::RuntimeTickOwner;

    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    let (mut session, _, _) = make_session();
    let creature_guid = test_creature_guid(91_214);
    let victim_guid = ObjectGuid::create_player(1, 91_215);
    let instant_spell_id = 15_691_i32;
    let noninstant_spell_id = 70_009_i32;
    add_canonical_creature_spell_test_pair_like_cpp(&canonical, creature_guid, victim_guid);
    register_test_creature(&mut session, manager.clone(), creature_guid, 25);
    session
        .mutate_world_creature(creature_guid, |creature| {
            creature
                .creature
                .set_ai_identity_names_runtime_like_cpp("CombatAI", String::new());
            creature
                .creature
                .set_spell(0, u32::try_from(instant_spell_id).unwrap());
            creature
                .creature
                .set_spell(1, u32::try_from(noninstant_spell_id).unwrap());
            creature.enter_combat(victim_guid);
        })
        .unwrap();
    manager
        .write()
        .unwrap()
        .set_tick_owner(RuntimeTickOwner::GlobalLegacy);

    let instant_spell = creature_ai_test_spell_info_like_cpp(instant_spell_id, 6, 0);
    let mut noninstant_spell = creature_ai_test_spell_info_like_cpp(noninstant_spell_id, 6, 0);
    noninstant_spell.cast_time_ms = 1_500;
    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(instant_spell_id, instant_spell.clone());
    spell_store.insert(noninstant_spell_id, noninstant_spell);
    let mut instant_misc =
        spell_misc_entry_like_cpp(8_211, u32::try_from(instant_spell_id).unwrap(), 71);
    let mut noninstant_misc =
        spell_misc_entry_like_cpp(8_212, u32::try_from(noninstant_spell_id).unwrap(), 71);
    instant_misc.attributes[0] |= wow_data::spell::attributes::SPELL_ATTR0_PASSIVE as i32;
    noninstant_misc.attributes[0] |= wow_data::spell::attributes::SPELL_ATTR0_PASSIVE as i32;
    let mut config = creature_ai_spell_test_config_like_cpp(instant_spell, true, 30.0);
    config.spell_store = Some(Arc::new(spell_store));
    config.spell_misc_store = Some(Arc::new(wow_data::SpellMiscStore::from_entries([
        instant_misc,
        noninstant_misc,
    ])));

    let outcome = run_legacy_creature_spell_tick_once_like_cpp(&manager, Some(&canonical), &config);

    assert_eq!(outcome.noninstant_casts_unrepresented, 1);
    assert_eq!(outcome.schedules_initialized, 0);
    assert_eq!(outcome.casts_ready, 0);
    assert!(
        outcome.plan.events.is_empty(),
        "Rust cannot emit instant 15691 while C++ could still own UNIT_STATE_CASTING for the other template slot"
    );
}
#[test]
fn legacy_creature_combat_ai_rearms_after_range_rejection_like_cpp() {
    use crate::map_manager::RuntimeTickOwner;

    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    let (mut session, _, _) = make_session();
    let creature_guid = test_creature_guid(91_206);
    let victim_guid = ObjectGuid::create_player(1, 91_207);
    let spell_id = 70_006_i32;
    add_canonical_creature_spell_test_pair_like_cpp(&canonical, creature_guid, victim_guid);
    canonical
        .lock()
        .unwrap()
        .find_map_mut(0, 0)
        .unwrap()
        .map_mut()
        .get_typed_player_mut(victim_guid)
        .unwrap()
        .unit_mut()
        .world_mut()
        .relocate(Position::new(100.0, 10.0, 0.0, 0.0));
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
    session
        .mutate_world_creature(creature_guid, |creature| {
            creature.backdate_runtime_clock_for_test(Duration::from_secs(60));
        })
        .unwrap();
    let rejected =
        run_legacy_creature_spell_tick_once_like_cpp(&manager, Some(&canonical), &config);

    assert_eq!(rejected.casts_ready, 0);
    assert_eq!(rejected.spell_range_rejections, 1);
    assert_eq!(rejected.canonical_cast_preconditions_passed, 0);
    assert!(rejected.plan.events.is_empty());
    let repeat_due_in_ms = session
        .mutate_world_creature(creature_guid, |creature| {
            creature.creature_spell_due_in_ms_for_test(0).unwrap()
        })
        .unwrap();
    assert!((6_000..=12_000).contains(&repeat_due_in_ms));
}
#[test]
fn legacy_creature_combat_ai_rearms_after_target_rejection_like_cpp() {
    use crate::map_manager::RuntimeTickOwner;

    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    let (mut session, _, _) = make_session();
    let creature_guid = test_creature_guid(91_336);
    let victim_guid = ObjectGuid::create_player(1, 91_337);
    let spell_id = 70_209_i32;
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
    canonical
        .lock()
        .unwrap()
        .find_map_mut(0, 0)
        .unwrap()
        .map_mut()
        .get_typed_player_mut(victim_guid)
        .unwrap()
        .unit_mut()
        .add_unit_state(UnitState::IN_FLIGHT.bits());
    session
        .mutate_world_creature(creature_guid, |creature| {
            creature.backdate_runtime_clock_for_test(Duration::from_secs(60));
        })
        .unwrap();

    let rejected =
        run_legacy_creature_spell_tick_once_like_cpp(&manager, Some(&canonical), &config);
    assert_eq!(rejected.casts_ready, 0);
    assert_eq!(rejected.canonical_cast_target_rejections, 1);
    assert_eq!(rejected.spell_hit_results_unrepresented, 0);
    assert!(rejected.plan.events.is_empty());
    let (repeat_due_in_ms, rng_authoritative) = session
        .mutate_world_creature(creature_guid, |creature| {
            (
                creature.creature_spell_due_in_ms_for_test(0).unwrap(),
                creature.runtime_rng_authority_complete_like_cpp(),
            )
        })
        .unwrap();
    assert!((6_000..=12_000).contains(&repeat_due_in_ms));
    assert!(rng_authoritative);
}
#[test]
fn legacy_creature_combat_ai_rejects_unhydrated_active_difficulty_metadata_like_cpp() {
    use crate::map_manager::RuntimeTickOwner;

    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    let (mut session, _, _) = make_session();
    let creature_guid = test_creature_guid(91_210);
    let victim_guid = ObjectGuid::create_player(1, 91_211);
    let spell_id = 70_008_i32;
    add_canonical_creature_spell_test_pair_with_difficulty_like_cpp(
        &canonical,
        creature_guid,
        victim_guid,
        2,
    );
    assert_eq!(
        canonical
            .lock()
            .unwrap()
            .find_map(0, 0)
            .unwrap()
            .difficulty(),
        2
    );
    register_test_creature(&mut session, manager.clone(), creature_guid, 25);
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

    let spell = creature_ai_test_spell_info_like_cpp(spell_id, 6, 0);
    let spell_id_u32 = u32::try_from(spell_id).unwrap();
    let mut config = creature_ai_spell_test_config_like_cpp(spell, false, 5.0);
    let base_misc = spell_misc_entry_like_cpp(8_101, spell_id_u32, 71);
    let mut active_misc = spell_misc_entry_like_cpp(8_102, spell_id_u32, 72);
    active_misc.difficulty_id = 2;
    config.spell_misc_store = Some(Arc::new(wow_data::SpellMiscStore::from_entries([
        base_misc,
        active_misc,
    ])));
    config.spell_range_store = Some(Arc::new(wow_data::SpellRangeStore::from_entries([
        spell_range_entry_like_cpp(71, 0.0, 0.5),
        spell_range_entry_like_cpp(72, 0.0, 30.0),
    ])));
    config.spell_cooldowns_store = Some(Arc::new(wow_data::SpellCooldownsStore::from_entries([
        wow_data::SpellCooldownsEntry {
            id: 8_103,
            difficulty_id: 0,
            recovery_time: 11_000,
            start_recovery_time: 0,
            spell_id: spell_id_u32,
            ..Default::default()
        },
        wow_data::SpellCooldownsEntry {
            id: 8_104,
            difficulty_id: 2,
            recovery_time: 7_000,
            start_recovery_time: 1_000,
            spell_id: spell_id_u32,
            ..Default::default()
        },
    ])));
    let visual_entry = |id, difficulty_id| wow_data::SpellXSpellVisualEntry {
        id,
        difficulty_id,
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
        spell_id: spell_id_u32,
    };
    config.spell_x_spell_visual_store =
        Some(Arc::new(wow_data::SpellXSpellVisualStore::from_entries([
            visual_entry(8_105, 0),
            visual_entry(8_106, 2),
        ])));
    config.difficulty_store = Some(Arc::new(DifficultyStore::from_entries([DifficultyEntry {
        id: 2,
        instance_type: 0,
        flags: 0,
        fallback_difficulty_id: 0,
        toggle_difficulty_id: 0,
    }])));

    let rejected =
        run_legacy_creature_spell_tick_once_like_cpp(&manager, Some(&canonical), &config);

    assert_eq!(rejected.spell_effects_unrepresented, 1);
    assert_eq!(rejected.schedules_initialized, 0);
    assert_eq!(rejected.casts_ready, 0);
    assert!(rejected.plan.events.is_empty());
}
#[test]
fn legacy_creature_combat_ai_aggro_self_effect_fails_closed_like_cpp() {
    use crate::map_manager::RuntimeTickOwner;

    let manager = shared_map_manager();
    let (mut session, _, _) = make_session();
    let creature_guid = test_creature_guid(91_202);
    let victim_guid = ObjectGuid::create_player(1, 91_203);
    let spell_id = 70_002_i32;
    let canonical = shared_canonical_map_manager();
    add_canonical_creature_spell_test_pair_like_cpp(&canonical, creature_guid, victim_guid);
    register_test_creature(&mut session, manager.clone(), creature_guid, 25);
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
    // AICOND_AGGRO passes `who` as the explicit SpellCastTargets unit, but
    // TargetA=1 still selects the caster in Spell::SelectSpellTargets. The
    // current single-victim wire slice must not claim that `who` was hit.
    let config = creature_ai_spell_test_config_like_cpp(
        creature_ai_test_spell_info_like_cpp(spell_id, 1, 0),
        true,
        30.0,
    );

    let engaged = run_legacy_creature_spell_tick_once_like_cpp(&manager, Some(&canonical), &config);
    assert_eq!(engaged.casts_ready, 0);
    assert_eq!(engaged.spell_effects_unrepresented, 1);
    assert!(engaged.plan.events.is_empty());
    let repeated =
        run_legacy_creature_spell_tick_once_like_cpp(&manager, Some(&canonical), &config);
    assert_eq!(repeated.casts_ready, 0);
    assert!(repeated.plan.events.is_empty());
}
#[test]
fn legacy_creature_ai_target_classifier_uses_target_a_and_nonzero_range_like_cpp() {
    let spell_id = 70_003_u32;
    let target_b_only = creature_ai_test_spell_info_like_cpp(spell_id as i32, 0, 6);
    let ranged = creature_ai_spell_test_config_like_cpp(target_b_only.clone(), false, 30.0);
    assert_eq!(
        creature_ai_spell_target_like_cpp(spell_id, &target_b_only, 0, &ranged),
        CreatureAiSpellTargetLikeCpp::SelfTarget
    );

    let target_a = creature_ai_test_spell_info_like_cpp(spell_id as i32, 6, 0);
    assert_eq!(
        creature_ai_spell_target_like_cpp(spell_id, &target_a, 0, &ranged),
        CreatureAiSpellTargetLikeCpp::Victim
    );
    let area_enemy = creature_ai_test_spell_info_like_cpp(spell_id as i32, 16, 0);
    let area_enemy_target = creature_ai_spell_target_like_cpp(spell_id, &area_enemy, 0, &ranged);
    assert_eq!(area_enemy_target, CreatureAiSpellTargetLikeCpp::Enemy);
    assert!(area_enemy_target.requires_random_threat_selection_like_cpp());

    let mut debuff = creature_ai_test_spell_info_like_cpp(spell_id as i32, 6, 0);
    debuff.effects[0].effect = wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA;
    let debuff_target = creature_ai_spell_target_like_cpp(spell_id, &debuff, 0, &ranged);
    assert_eq!(debuff_target, CreatureAiSpellTargetLikeCpp::Debuff);
    assert!(debuff_target.requires_random_threat_selection_like_cpp());
    let zero_range = creature_ai_spell_test_config_like_cpp(target_a.clone(), false, 0.0);
    assert_eq!(
        creature_ai_spell_target_like_cpp(spell_id, &target_a, 0, &zero_range),
        CreatureAiSpellTargetLikeCpp::SelfTarget
    );
}
#[test]
fn legacy_creature_spell_visual_stops_at_conditional_active_difficulty_like_cpp() {
    let spell_id = 70_006_u32;
    let spell = creature_ai_test_spell_info_like_cpp(spell_id as i32, 6, 0);
    let mut config = creature_ai_spell_test_config_like_cpp(spell, false, 30.0);
    let visual = |id, difficulty_id, viewer_player_condition_id| wow_data::SpellXSpellVisualEntry {
        id,
        difficulty_id,
        spell_visual_id: id + 100,
        probability: 1.0,
        flags: 0,
        priority: 0,
        spell_icon_file_id: 0,
        active_icon_file_id: 0,
        viewer_unit_condition_id: 0,
        viewer_player_condition_id,
        caster_unit_condition_id: 0,
        caster_player_condition_id: 0,
        spell_id,
    };
    config.spell_x_spell_visual_store =
        Some(Arc::new(wow_data::SpellXSpellVisualStore::from_entries([
            visual(8_201, 0, 0),
            visual(8_202, 2, 23),
        ])));
    config.difficulty_store = Some(Arc::new(DifficultyStore::from_entries([DifficultyEntry {
        id: 2,
        instance_type: 0,
        flags: 0,
        fallback_difficulty_id: 0,
        toggle_difficulty_id: 0,
    }])));

    assert_eq!(
        creature_ai_spell_x_spell_visual_id_like_cpp(spell_id, 2, &config),
        Err(()),
        "C++ must evaluate the active difficulty's viewer condition; Rust cannot skip it and borrow the fallback visual"
    );
    assert_eq!(
        creature_ai_spell_x_spell_visual_id_like_cpp(spell_id, 0, &config),
        Ok(8_201)
    );
}
#[test]
fn legacy_creature_spell_wire_slice_fails_closed_for_unrepresented_topology_like_cpp() {
    let victim_guid = ObjectGuid::create_player(1, 91_204);
    let caster_guid = test_creature_guid(91_205);
    let mut spell = creature_ai_test_spell_info_like_cpp(70_005, 6, 0);
    assert_eq!(
        creature_ai_spell_single_unit_topology_like_cpp(&spell, victim_guid, victim_guid, false,),
        Ok(())
    );

    spell.cast_time_ms = 1_000;
    assert_eq!(
        creature_ai_spell_single_unit_topology_like_cpp(&spell, victim_guid, victim_guid, false,),
        Err(CreatureAiSpellRepresentationRejectionLikeCpp::NonInstant)
    );
    spell.cast_time_ms = 0;
    spell.effects[0].implicit_target_1 = 1;
    assert_eq!(
        creature_ai_spell_single_unit_topology_like_cpp(&spell, victim_guid, victim_guid, false,),
        Err(CreatureAiSpellRepresentationRejectionLikeCpp::EffectOrTarget),
        "TargetA=1 selects the caster even when SpellCastTargets names the victim"
    );

    spell.effects[0].implicit_target_1 = 6;
    spell.effects[0].implicit_target_2 = 16;
    assert_eq!(
        creature_ai_spell_single_unit_topology_like_cpp(&spell, victim_guid, victim_guid, false,),
        Err(CreatureAiSpellRepresentationRejectionLikeCpp::EffectOrTarget)
    );
    spell.effects[0].implicit_target_2 = 0;
    spell.effects[0].chain_targets = 2;
    assert_eq!(
        creature_ai_spell_single_unit_topology_like_cpp(&spell, victim_guid, victim_guid, false,),
        Err(CreatureAiSpellRepresentationRejectionLikeCpp::EffectOrTarget)
    );
    spell.effects[0].chain_targets = 0;
    assert_eq!(
        creature_ai_spell_single_unit_topology_like_cpp(&spell, caster_guid, victim_guid, false,),
        Err(CreatureAiSpellRepresentationRejectionLikeCpp::EffectOrTarget)
    );

    spell
        .power_costs
        .push(wow_data::spell::SpellPowerCostInfoLikeCpp {
            order_index: 0,
            power_type: 3,
            mana_cost: 0,
            mana_cost_per_level: 0,
            mana_per_second: 0,
            power_cost_pct: 0.0,
            power_cost_max_pct: 0.0,
            power_pct_per_second: 0.0,
            required_aura_spell_id: 0,
            optional_cost: 0,
        });
    assert_eq!(
        creature_ai_spell_single_unit_topology_like_cpp(&spell, victim_guid, victim_guid, false,),
        Ok(()),
        "a production-shaped zero-value SpellPower row has no effective cost"
    );
    spell.power_costs[0].mana_cost = 1;
    assert_eq!(
        creature_ai_spell_single_unit_topology_like_cpp(&spell, victim_guid, victim_guid, false,),
        Err(CreatureAiSpellRepresentationRejectionLikeCpp::EffectOrTarget),
        "nonzero Creature power cost remains fail-closed"
    );
    spell.power_costs[0].mana_cost = 0;
    spell.power_costs[0].mana_cost_per_level = 1;
    assert_eq!(
        creature_ai_spell_single_unit_topology_like_cpp(&spell, victim_guid, victim_guid, false,),
        Err(CreatureAiSpellRepresentationRejectionLikeCpp::EffectOrTarget),
        "ManaCostPerLevel remains fail-closed"
    );
    spell.power_costs[0].mana_cost_per_level = 0;
    spell.power_costs[0].mana_per_second = 1;
    assert_eq!(
        creature_ai_spell_single_unit_topology_like_cpp(&spell, victim_guid, victim_guid, false,),
        Err(CreatureAiSpellRepresentationRejectionLikeCpp::EffectOrTarget),
        "ManaPerSecond remains fail-closed"
    );
    spell.power_costs[0].mana_per_second = 0;
    spell.power_costs[0].power_pct_per_second = 1.0;
    assert_eq!(
        creature_ai_spell_single_unit_topology_like_cpp(&spell, victim_guid, victim_guid, false,),
        Err(CreatureAiSpellRepresentationRejectionLikeCpp::EffectOrTarget),
        "PowerPctPerSecond remains fail-closed"
    );
    spell.power_costs[0].power_pct_per_second = 0.0;
    spell.power_costs[0].required_aura_spell_id = 123;
    assert_eq!(
        creature_ai_spell_single_unit_topology_like_cpp(&spell, victim_guid, victim_guid, false,),
        Err(CreatureAiSpellRepresentationRejectionLikeCpp::EffectOrTarget),
        "RequiredAura-dependent cost remains fail-closed"
    );
    spell.power_costs[0].required_aura_spell_id = 0;
    spell.power_costs[0].optional_cost = 1;
    assert_eq!(
        creature_ai_spell_single_unit_topology_like_cpp(&spell, victim_guid, victim_guid, false,),
        Err(CreatureAiSpellRepresentationRejectionLikeCpp::EffectOrTarget),
        "OptionalCost remains fail-closed"
    );
}
