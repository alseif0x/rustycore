//! Session scenarios exercising the represented world entities responsibility.
//!
//! Split out of session_tests.rs under #626; assertions and registrations
//! are unchanged and the shared fixtures stay in the parent module.

use super::*;

#[test]
fn creature_spell_casting_and_aura_restrictions_fail_closed_like_cpp() {
    const SPELL_ID: u32 = 70_190;
    let base = creature_ai_spell_test_config_like_cpp(
        creature_ai_test_spell_info_like_cpp(SPELL_ID as i32, 6, 0),
        false,
        30.0,
    );
    assert!(base.spell_has_no_unrepresented_casting_requirements_like_cpp(SPELL_ID));
    assert!(base.spell_has_no_unrepresented_aura_restrictions_like_cpp(SPELL_ID, 0));

    let mut missing = base.clone();
    missing.spell_casting_requirements_store = None;
    missing.spell_aura_restrictions_store = None;
    assert!(!missing.spell_has_no_unrepresented_casting_requirements_like_cpp(SPELL_ID));
    assert!(!missing.spell_has_no_unrepresented_aura_restrictions_like_cpp(SPELL_ID, 0));

    let mut required_area = base.clone();
    required_area.spell_casting_requirements_store = Some(Arc::new(
        wow_data::SpellCastingRequirementsStore::from_entries([
            wow_data::SpellCastingRequirementsEntry {
                id: 1,
                spell_id: SPELL_ID as i32,
                facing_caster_flags: 0,
                min_faction_id: 0,
                min_reputation: 0,
                required_areas_id: 42,
                required_aura_vision: 0,
                requires_spell_focus: 0,
            },
        ]),
    ));
    assert!(
        !required_area.spell_has_no_unrepresented_casting_requirements_like_cpp(SPELL_ID),
        "C++ SpellInfo::CheckLocation must reject an unevaluated RequiredAreasID"
    );

    let mut required_spell_focus = base.clone();
    required_spell_focus.spell_casting_requirements_store = Some(Arc::new(
        wow_data::SpellCastingRequirementsStore::from_entries([
            wow_data::SpellCastingRequirementsEntry {
                id: 2,
                spell_id: SPELL_ID as i32,
                facing_caster_flags: 0,
                min_faction_id: 0,
                min_reputation: 0,
                required_areas_id: 0,
                required_aura_vision: 0,
                requires_spell_focus: 181,
            },
        ]),
    ));
    assert!(
        !required_spell_focus.spell_has_no_unrepresented_casting_requirements_like_cpp(SPELL_ID),
        "C++ Spell::CheckCast must reject an unevaluated required spell focus"
    );

    let mut required_aura = base;
    required_aura.spell_aura_restrictions_store = Some(Arc::new(
        wow_data::SpellAuraRestrictionsStore::from_entries([
            wow_data::SpellAuraRestrictionsEntry {
                id: 1,
                difficulty_id: 0,
                caster_aura_state: 0,
                target_aura_state: 0,
                exclude_caster_aura_state: 0,
                exclude_target_aura_state: 0,
                caster_aura_spell: 0,
                target_aura_spell: 123,
                exclude_caster_aura_spell: 0,
                exclude_target_aura_spell: 0,
                spell_id: SPELL_ID,
            },
        ]),
    ));
    assert!(
        !required_aura.spell_has_no_unrepresented_aura_restrictions_like_cpp(SPELL_ID, 0),
        "required/excluded aura state is not represented by the bounded AI cast"
    );
}
#[test]
fn creature_spell_canonical_targetability_honors_cpp_untargetable_override() {
    let attributes = [0_u32; 15];
    for flag in [
        UnitFlags::NON_ATTACKABLE,
        UnitFlags::UNINTERACTIBLE,
        UnitFlags::ON_TAXI,
        UnitFlags::NOT_ATTACKABLE_1,
        UnitFlags::IMMUNE_TO_NPC,
        UnitFlags::NON_ATTACKABLE_2,
    ] {
        assert!(!creature_spell_target_accepts_npc_attack_like_cpp(
            flag,
            &attributes
        ));
    }

    let mut can_target_untargetable = attributes;
    can_target_untargetable[6] = 0x0100_0000; // SPELL_ATTR6_CAN_TARGET_UNTARGETABLE
    assert!(creature_spell_target_accepts_npc_attack_like_cpp(
        UnitFlags::NON_ATTACKABLE_2,
        &can_target_untargetable,
    ));
    assert!(!creature_spell_target_accepts_npc_attack_like_cpp(
        UnitFlags::NON_ATTACKABLE | UnitFlags::NON_ATTACKABLE_2,
        &can_target_untargetable,
    ));
}
#[test]
fn creature_spell_canonical_targetability_rechecks_full_cpp_state() {
    let canonical = shared_canonical_map_manager();
    let creature_guid = test_creature_guid(91_334);
    let victim_guid = ObjectGuid::create_player(1, 91_335);
    add_canonical_creature_spell_test_pair_like_cpp(&canonical, creature_guid, victim_guid);
    let config = legacy_aggro_hostile_config_like_cpp();
    let attributes = [0_u32; 15];

    let target_is_valid = |canonical: &SharedCanonicalMapManager| {
        let manager = canonical.lock().unwrap();
        let map = manager.find_map(0, 0).unwrap().map();
        map.with_creature_like_cpp(creature_guid, |creature| {
            creature_spell_target_is_valid_attack_target_like_cpp(
                creature,
                map.get_typed_player(victim_guid).unwrap(),
                &attributes,
                &config,
            )
        })
        .unwrap()
    };
    assert!(target_is_valid(&canonical));

    canonical
        .lock()
        .unwrap()
        .find_map_mut(0, 0)
        .unwrap()
        .map_mut()
        .get_typed_player_mut(victim_guid)
        .unwrap()
        .set_game_master_like_cpp(true);
    assert!(
        !target_is_valid(&canonical),
        "a player entering GM mode after aggro must fail the cast-time C++ target check"
    );

    let mut manager = canonical.lock().unwrap();
    let victim = manager
        .find_map_mut(0, 0)
        .unwrap()
        .map_mut()
        .get_typed_player_mut(victim_guid)
        .unwrap();
    victim.set_game_master_like_cpp(false);
    victim
        .unit_mut()
        .add_unit_state(UnitState::IN_FLIGHT.bits());
    drop(manager);
    assert!(
        !target_is_valid(&canonical),
        "UNIT_STATE_UNATTACKABLE changes must also be observed at cast time"
    );
}
#[test]
fn creature_spell_runtime_hook_authority_rejects_every_binding_source_like_cpp() {
    const ROOT_SPELL_ID: u32 = 70_200;
    const CANDIDATE_SPELL_ID: u32 = 70_201;
    const LINKED_EFFECT_SPELL_ID: u32 = 70_202;

    let base = creature_ai_spell_test_config_like_cpp(
        creature_ai_test_spell_info_like_cpp(CANDIDATE_SPELL_ID as i32, 6, 0),
        false,
        30.0,
    );
    assert!(base.spell_has_no_unrepresented_runtime_hooks_like_cpp(CANDIDATE_SPELL_ID));

    for missing_authority in 0..7 {
        let mut config = base.clone();
        match missing_authority {
            0 => config.spell_script_exact_spell_ids_like_cpp = None,
            1 => config.spell_script_all_rank_root_spell_ids_like_cpp = None,
            2 => config.legacy_spell_script_spell_ids_like_cpp = None,
            3 => config.spell_linked_rejected_trigger_spell_ids_like_cpp = None,
            4 => config.spell_chain_store = None,
            5 => config.spell_linked_store = None,
            6 => config.spell_condition_store = None,
            _ => unreachable!(),
        }
        assert!(
            !config.spell_has_no_unrepresented_runtime_hooks_like_cpp(CANDIDATE_SPELL_ID),
            "missing runtime-hook authority source {missing_authority} must fail closed"
        );
    }

    let mut exact = base.clone();
    exact.spell_script_exact_spell_ids_like_cpp =
        Some(Arc::new(BTreeSet::from([CANDIDATE_SPELL_ID])));
    assert!(!exact.spell_has_no_unrepresented_runtime_hooks_like_cpp(CANDIDATE_SPELL_ID));

    let mut all_ranks = base.clone();
    all_ranks.spell_chain_store = Some(Arc::new(
        SpellChainStoreLikeCpp::from_skill_line_ability_supercedes_like_cpp(
            [wow_data::SpellRankEdgeLikeCpp {
                spell_id: CANDIDATE_SPELL_ID,
                supercedes_spell_id: ROOT_SPELL_ID,
            }],
            |_| true,
        ),
    ));
    all_ranks.spell_script_all_rank_root_spell_ids_like_cpp =
        Some(Arc::new(BTreeSet::from([ROOT_SPELL_ID])));
    assert!(
        !all_ranks.spell_has_no_unrepresented_runtime_hooks_like_cpp(CANDIDATE_SPELL_ID),
        "a negative spell_script_names root applies to every represented rank"
    );

    let mut legacy = base.clone();
    legacy.legacy_spell_script_spell_ids_like_cpp =
        Some(Arc::new(BTreeSet::from([CANDIDATE_SPELL_ID])));
    assert!(!legacy.spell_has_no_unrepresented_runtime_hooks_like_cpp(CANDIDATE_SPELL_ID));

    let mut rejected_link = base.clone();
    rejected_link.spell_linked_rejected_trigger_spell_ids_like_cpp =
        Some(Arc::new(BTreeSet::from([CANDIDATE_SPELL_ID])));
    assert!(
        !rejected_link.spell_has_no_unrepresented_runtime_hooks_like_cpp(CANDIDATE_SPELL_ID),
        "a rejected linked row cannot prove that its trigger has no C++ hook"
    );

    let mut conditioned = base.clone();
    conditioned.spell_condition_store = Some(Arc::new(
        ConditionEntriesByTypeStore::from_conditions_like_cpp([wow_data::Condition {
            source_type: wow_constants::ConditionSourceType::Spell,
            source_entry: CANDIDATE_SPELL_ID as i32,
            condition_type: wow_constants::ConditionType::Aura,
            condition_value1: 123,
            ..Default::default()
        }]),
    ));
    assert!(
        !conditioned.spell_has_no_unrepresented_runtime_hooks_like_cpp(CANDIDATE_SPELL_ID),
        "C++ SourceType 17 conditions must reject the bounded cast"
    );

    for link_type in 0..=3 {
        let outcome = SpellLinkedStoreLikeCpp::from_rows_like_cpp(
            [wow_data::SpellLinkedRowLikeCpp {
                spell_trigger: CANDIDATE_SPELL_ID as i32,
                spell_effect: LINKED_EFFECT_SPELL_ID as i32,
                link_type,
            }],
            |_| {
                Some(wow_data::SpellLinkedSpellInfoLikeCpp {
                    effect_calc_values_by_index: Vec::new(),
                })
            },
        );
        assert!(outcome.errors.is_empty());
        let mut linked = base.clone();
        linked.spell_linked_store = Some(Arc::new(outcome.store));
        assert!(
            !linked.spell_has_no_unrepresented_runtime_hooks_like_cpp(CANDIDATE_SPELL_ID),
            "linked-spell hook type {link_type} must reject the source cast"
        );
    }
}
#[test]
fn creature_spell_runtime_hooks_block_all_three_ai_cast_paths_like_cpp() {
    let (aggro_manager, aggro_canonical, aggro_config, _) =
        creature_spell_bound_hook_tick_fixture_like_cpp("CombatAI", true, 70_203, 91_320, 91_321);
    let aggro = run_legacy_creature_spell_tick_once_like_cpp(
        &aggro_manager,
        Some(&aggro_canonical),
        &aggro_config,
    );
    assert_eq!(aggro.spell_runtime_hooks_unrepresented, 1);
    assert_eq!(aggro.casts_ready, 0);
    assert!(aggro.plan.events.is_empty());

    let (combat_manager, combat_canonical, combat_config, combat_creature_guid) =
        creature_spell_bound_hook_tick_fixture_like_cpp("CombatAI", false, 70_204, 91_322, 91_323);
    let initialized = run_legacy_creature_spell_tick_once_like_cpp(
        &combat_manager,
        Some(&combat_canonical),
        &combat_config,
    );
    assert_eq!(initialized.schedules_initialized, 1);
    assert_eq!(initialized.spell_runtime_hooks_unrepresented, 0);
    combat_manager
        .write()
        .unwrap()
        .find_creature_mut(0, 0, combat_creature_guid)
        .unwrap()
        .backdate_runtime_clock_for_test(Duration::from_secs(60));
    let combat = run_legacy_creature_spell_tick_once_like_cpp(
        &combat_manager,
        Some(&combat_canonical),
        &combat_config,
    );
    assert_eq!(combat.spell_runtime_hooks_unrepresented, 1);
    assert_eq!(combat.casts_ready, 0);
    assert!(combat.plan.events.is_empty());
    assert!(
        combat_manager
            .read()
            .unwrap()
            .find_creature(0, 0, combat_creature_guid)
            .unwrap()
            .creature_spell_due_in_ms_for_test(0)
            .is_some(),
        "CombatAI re-schedules after a failed DoCast attempt in C++"
    );

    let (turret_manager, turret_canonical, turret_config, _) =
        creature_spell_bound_hook_tick_fixture_like_cpp("TurretAI", false, 70_205, 91_324, 91_325);
    let turret = run_legacy_creature_spell_tick_once_like_cpp(
        &turret_manager,
        Some(&turret_canonical),
        &turret_config,
    );
    assert_eq!(turret.spell_runtime_hooks_unrepresented, 1);
    assert_eq!(turret.casts_ready, 0);
    assert!(turret.plan.events.is_empty());
}
#[test]
fn creature_spell_runtime_blocks_required_area_and_aura_restrictions_like_cpp() {
    let area_spell_id = 70_207_u32;
    let (area_manager, area_canonical, mut area_config, _) =
        creature_spell_bound_hook_tick_fixture_like_cpp(
            "CombatAI",
            true,
            area_spell_id as i32,
            91_328,
            91_329,
        );
    area_config.spell_script_exact_spell_ids_like_cpp = Some(Arc::new(BTreeSet::new()));
    area_config.spell_casting_requirements_store = Some(Arc::new(
        wow_data::SpellCastingRequirementsStore::from_entries([
            wow_data::SpellCastingRequirementsEntry {
                id: 1,
                spell_id: area_spell_id as i32,
                facing_caster_flags: 0,
                min_faction_id: 0,
                min_reputation: 0,
                required_areas_id: 42,
                required_aura_vision: 0,
                requires_spell_focus: 0,
            },
        ]),
    ));
    let area = run_legacy_creature_spell_tick_once_like_cpp(
        &area_manager,
        Some(&area_canonical),
        &area_config,
    );
    assert_eq!(area.spell_casting_requirements_unrepresented, 1);
    assert_eq!(area.casts_ready, 0);
    assert!(area.plan.events.is_empty());

    let aura_spell_id = 70_208_u32;
    let (aura_manager, aura_canonical, mut aura_config, _) =
        creature_spell_bound_hook_tick_fixture_like_cpp(
            "TurretAI",
            false,
            aura_spell_id as i32,
            91_330,
            91_331,
        );
    aura_config.spell_script_exact_spell_ids_like_cpp = Some(Arc::new(BTreeSet::new()));
    aura_config.spell_aura_restrictions_store = Some(Arc::new(
        wow_data::SpellAuraRestrictionsStore::from_entries([
            wow_data::SpellAuraRestrictionsEntry {
                id: 1,
                difficulty_id: 0,
                caster_aura_state: 0,
                target_aura_state: 0,
                exclude_caster_aura_state: 0,
                exclude_target_aura_state: 0,
                caster_aura_spell: 0,
                target_aura_spell: 123,
                exclude_caster_aura_spell: 0,
                exclude_target_aura_spell: 0,
                spell_id: aura_spell_id,
            },
        ]),
    ));
    let aura = run_legacy_creature_spell_tick_once_like_cpp(
        &aura_manager,
        Some(&aura_canonical),
        &aura_config,
    );
    assert_eq!(aura.spell_effects_unrepresented, 1);
    assert_eq!(aura.casts_ready, 0);
    assert!(aura.plan.events.is_empty());
}
#[test]
fn creature_spell_disable_row_blocks_runtime_plan_like_cpp() {
    let (manager, canonical, mut config, _) =
        creature_spell_bound_hook_tick_fixture_like_cpp("CombatAI", true, 70_206, 91_326, 91_327);
    config.spell_script_exact_spell_ids_like_cpp = Some(Arc::new(BTreeSet::new()));
    let (disable_mgr, report) = wow_data::DisableMgrLikeCpp::from_rows_like_cpp(
        [wow_data::DisableDbRowLikeCpp {
            source_type: wow_data::DISABLE_TYPE_SPELL,
            entry: 70_206,
            flags: wow_data::SPELL_DISABLE_CREATURE,
            params_0: String::new(),
            params_1: String::new(),
        }],
        wow_data::DisableMgrRefsLikeCpp::default(),
    );
    assert_eq!(report.loaded_count, 1);
    config.disable_mgr = Some(Arc::new(disable_mgr));

    let outcome = run_legacy_creature_spell_tick_once_like_cpp(&manager, Some(&canonical), &config);
    assert_eq!(outcome.spells_disabled, 1);
    assert_eq!(outcome.casts_ready, 0);
    assert!(outcome.plan.events.is_empty());
}
#[test]
fn creature_spell_cast_log_snapshot_preserves_cpp_power_rows_like_cpp() {
    use wow_data::SpellPowerCostInfoLikeCpp;
    use wow_packet::packets::spell::SpellLogPowerData;

    // `SharedDefines.h` declares this signed-enum sentinel explicitly.
    // The zero-cost CalcPowerCost path retains it in `Spell::m_powerCost`.
    const POWER_ALL_LIKE_CPP: i8 = 127;

    let mut caster = wow_entities::Creature::new(false);
    caster.unit_mut().set_max_health(900);
    caster.unit_mut().set_health(765);
    caster.set_power_type(PowerType::Rage);
    caster.unit_mut().set_max_power(PowerType::Rage, 100);
    caster.unit_mut().set_power(PowerType::Rage, 55);
    caster.set_combat_log_stats_like_cpp(wow_entities::CreatureCombatLogStatsLikeCpp {
        attack_power: 111,
        ranged_attack_power: 222,
        spell_power: 333,
        armor: 444,
    });
    caster
        .unit_mut()
        .subsystems_mut()
        .auras
        .set_spell_cast_log_aura_authority_inert_like_cpp(true);

    let zero_cost = |order_index, power_type| SpellPowerCostInfoLikeCpp {
        order_index,
        power_type,
        mana_cost: 0,
        mana_cost_per_level: 0,
        mana_per_second: 0,
        power_cost_pct: 0.0,
        power_cost_max_pct: 0.0,
        power_pct_per_second: 0.0,
        required_aura_spell_id: 0,
        optional_cost: 0,
    };
    let mut spell = creature_ai_test_spell_info_like_cpp(70_200, 6, 0);
    spell.power_costs = vec![
        zero_cost(0, PowerType::Energy as i8),
        zero_cost(1, PowerType::Energy as i8),
        zero_cost(2, PowerType::Health as i8),
        zero_cost(3, POWER_ALL_LIKE_CPP),
    ];

    let log = creature_spell_cast_log_data_like_cpp(&caster, &spell, 0)
        .expect("accredited base-difficulty zero-cost snapshot");

    assert_eq!(log.health, 765);
    assert_eq!(log.attack_power, 111);
    assert_eq!(log.spell_power, 333);
    assert_eq!(log.armor, 444);
    assert_eq!(
        log.power_data,
        vec![
            SpellLogPowerData {
                power_type: PowerType::Rage as i32,
                amount: 55,
                cost: 0,
            },
            SpellLogPowerData {
                power_type: PowerType::Energy as i32,
                amount: 0,
                cost: 0,
            },
            SpellLogPowerData {
                power_type: PowerType::Health as i32,
                amount: 0,
                cost: 0,
            },
            SpellLogPowerData {
                power_type: i32::from(POWER_ALL_LIKE_CPP),
                amount: 0,
                cost: 0,
            },
        ]
    );
    assert!(
        creature_spell_cast_log_data_like_cpp(&caster, &spell, 1).is_none(),
        "difficulty-specific SpellPower rows are not hydrated in M2.6"
    );
}
#[test]
fn represented_creature_melee_spell_hit_profile_keeps_cpp_base_miss_across_levels() {
    let metadata = wow_data::SpellHitMetadataLikeCpp {
        category_id: 0,
        charge_category_id: 0,
        defense_type: 2,
        spell_mechanic: 0,
        school_mask: 0x01,
        effect_mechanics: BTreeMap::from([(0, 0)]),
    };
    let profile = represented_creature_spell_hit_profile_like_cpp(
        &metadata,
        &[0],
        represented_creature_spell_test_attributes_like_cpp(false),
    )
    .unwrap();

    assert_eq!(
        profile,
        CreatureSpellHitProfileLikeCpp::BaseMeleeMiss {
            miss_threshold_per_ten_thousand: 500,
        }
    );
    assert_eq!(
        resolve_creature_spell_hit_profile_like_cpp(profile, Some(499)),
        Some(CreatureSpellTargetHitResultLikeCpp::Miss)
    );
    assert_eq!(
        resolve_creature_spell_hit_profile_like_cpp(profile, Some(500)),
        Some(CreatureSpellTargetHitResultLikeCpp::Hit)
    );
    assert_eq!(
        resolve_creature_spell_hit_profile_like_cpp(profile, None),
        None
    );

    let higher_level_victim = represented_creature_spell_hit_profile_like_cpp(
        &metadata,
        &[0],
        represented_creature_spell_test_attributes_like_cpp(false),
    )
    .unwrap();
    assert_eq!(
        higher_level_victim,
        CreatureSpellHitProfileLikeCpp::BaseMeleeMiss {
            miss_threshold_per_ten_thousand: 500,
        }
    );
    assert_eq!(
        resolve_creature_spell_hit_profile_like_cpp(higher_level_victim, Some(499)),
        Some(CreatureSpellTargetHitResultLikeCpp::Miss)
    );
    assert_eq!(
        resolve_creature_spell_hit_profile_like_cpp(higher_level_victim, Some(500)),
        Some(CreatureSpellTargetHitResultLikeCpp::Hit)
    );

    let lower_level_victim = represented_creature_spell_hit_profile_like_cpp(
        &metadata,
        &[0],
        represented_creature_spell_test_attributes_like_cpp(false),
    )
    .unwrap();
    assert_eq!(
        lower_level_victim,
        CreatureSpellHitProfileLikeCpp::BaseMeleeMiss {
            miss_threshold_per_ten_thousand: 500,
        }
    );
    assert_eq!(
        resolve_creature_spell_hit_profile_like_cpp(lower_level_victim, Some(499)),
        Some(CreatureSpellTargetHitResultLikeCpp::Miss)
    );
    assert_eq!(
        resolve_creature_spell_hit_profile_like_cpp(lower_level_victim, Some(500)),
        Some(CreatureSpellTargetHitResultLikeCpp::Hit)
    );

    let guaranteed = represented_creature_spell_hit_profile_like_cpp(
        &metadata,
        &[0],
        represented_creature_spell_test_attributes_like_cpp(true),
    )
    .unwrap();
    assert_eq!(
        guaranteed,
        CreatureSpellHitProfileLikeCpp::NoAttackMissAfterRequiredRoll
    );
    assert_eq!(
        resolve_creature_spell_hit_profile_like_cpp(guaranteed, Some(0)),
        Some(CreatureSpellTargetHitResultLikeCpp::Hit)
    );
    assert_eq!(
        resolve_creature_spell_hit_profile_like_cpp(guaranteed, None),
        None
    );
}
#[test]
fn represented_creature_melee_spell_hit_profile_fails_closed_on_incomplete_metadata() {
    let mut metadata = wow_data::SpellHitMetadataLikeCpp {
        category_id: 0,
        charge_category_id: 0,
        defense_type: 2,
        spell_mechanic: 0,
        school_mask: 0x01,
        effect_mechanics: BTreeMap::from([(0, 0)]),
    };
    let attributes = represented_creature_spell_test_attributes_like_cpp(false);

    metadata.spell_mechanic = 1;
    assert_eq!(
        represented_creature_spell_hit_profile_like_cpp(&metadata, &[0], attributes,),
        None
    );
    metadata.spell_mechanic = 0;
    metadata.effect_mechanics.clear();
    assert_eq!(
        represented_creature_spell_hit_profile_like_cpp(&metadata, &[0], attributes,),
        None
    );
    metadata.effect_mechanics.insert(0, 0);
    let mut reflection_attributes = attributes;
    reflection_attributes[7] |= 0x0000_0001; // SPELL_ATTR7_ALLOW_SPELL_REFLECTION
    assert_eq!(
        represented_creature_spell_hit_profile_like_cpp(&metadata, &[0], reflection_attributes,),
        None
    );
}
#[test]
fn creature_spell_fanout_uses_canonical_commit_snapshot_like_cpp() {
    use crate::map_manager::RuntimeTickOwner;

    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    let (mut session, _, _) = make_session();
    let creature_guid = test_creature_guid(91_318);
    let victim_guid = ObjectGuid::create_player(1, 91_319);
    let spell_id = 70_106_i32;
    let legacy_position = Position::new(10.0, 10.0, 0.0, 0.0);
    let canonical_position = Position::new(40.0, 10.0, 0.0, 0.0);
    let canonical_victim_position = Position::new(41.0, 10.0, 0.0, 0.0);

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
            assert_eq!(creature.position(), legacy_position);
            creature
                .creature
                .unit_mut()
                .world_mut()
                .set_visibility_distance_override_like_cpp(
                    wow_entities::VisibilityDistanceTypeLikeCpp::Tiny,
                );
            creature
                .creature
                .set_ai_identity_names_runtime_like_cpp("CombatAI", String::new());
            creature.creature.unit_mut().set_health(17);
            creature.creature.set_power_type(PowerType::Energy);
            creature
                .creature
                .unit_mut()
                .set_max_power(PowerType::Energy, 100);
            creature
                .creature
                .unit_mut()
                .set_power(PowerType::Energy, 19);
            creature.creature.set_combat_log_stats_like_cpp(
                wow_entities::CreatureCombatLogStatsLikeCpp {
                    attack_power: 911,
                    ranged_attack_power: 922,
                    spell_power: 933,
                    armor: 944,
                },
            );
            creature
                .creature
                .set_spell(0, u32::try_from(spell_id).unwrap());
            creature.enter_combat(victim_guid);
        })
        .unwrap();
    // Registration mirrors the legacy creature into the canonical map, so
    // establish the diverging canonical commit snapshot afterwards.
    {
        let mut canonical = canonical.lock().unwrap();
        let map = canonical.find_map_mut(0, 0).unwrap().map_mut();
        map.relocate_map_object_like_cpp(creature_guid, canonical_position)
            .unwrap();
        map.relocate_map_object_like_cpp(victim_guid, canonical_victim_position)
            .unwrap();
        let caster = map.get_typed_creature_mut(creature_guid).unwrap();
        caster
            .unit_mut()
            .world_mut()
            .set_visibility_distance_override_like_cpp(
                wow_entities::VisibilityDistanceTypeLikeCpp::Large,
            );
        caster.unit_mut().set_max_health(5_000);
        caster.unit_mut().set_health(4_321);
        caster.set_power_type(PowerType::Rage);
        caster.unit_mut().set_max_power(PowerType::Rage, 100);
        caster.unit_mut().set_power(PowerType::Rage, 55);
        caster.set_combat_log_stats_like_cpp(wow_entities::CreatureCombatLogStatsLikeCpp {
            attack_power: 111,
            ranged_attack_power: 222,
            spell_power: 333,
            armor: 444,
        });
    }
    manager
        .write()
        .unwrap()
        .set_tick_owner(RuntimeTickOwner::GlobalLegacy);
    let config = creature_ai_spell_test_config_like_cpp(
        creature_ai_test_spell_info_like_cpp(spell_id, 6, 0),
        true,
        30.0,
    );

    let cast = run_legacy_creature_spell_tick_once_like_cpp(&manager, Some(&canonical), &config);

    assert_eq!(cast.canonical_cast_preconditions_passed, 1);
    let [event] = cast.plan.events.as_slice() else {
        panic!("one committed creature cast expected: {cast:?}");
    };
    let crate::map_manager::RecipientRule::NearbyVisibleDurableSpellCast {
        source_position,
        range,
        full_go_packet_bytes,
        ..
    } = &event.recipients
    else {
        panic!("committed cast must use the atomic fanout rule");
    };
    assert_eq!(*source_position, canonical_position);
    assert_ne!(*source_position, legacy_position);
    assert_eq!(
        *range,
        wow_entities::VisibilityDistanceTypeLikeCpp::Large.distance_like_cpp(),
        "fanout range must come from the same canonical caster snapshot"
    );
    assert_eq!(
        decode_creature_spell_full_log_like_cpp(full_go_packet_bytes),
        wow_packet::packets::spell::SpellCastLogData {
            health: 4_321,
            attack_power: 111,
            spell_power: 333,
            armor: 444,
            power_data: vec![wow_packet::packets::spell::SpellLogPowerData {
                power_type: PowerType::Rage as i32,
                amount: 55,
                cost: 0,
            }],
        },
        "full GO must be built from the same canonical caster snapshot, not the legacy mirror"
    );
}
