//! Session scenarios exercising the represented world entities responsibility.
//!
//! Split out of session_tests.rs under #626; assertions and registrations
//! are unchanged and the shared fixtures stay in the parent module.

use super::*;

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
    let config = aggro_hostile_config_like_cpp();
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
fn creature_spell_runtime_hooks_block_all_three_ai_cast_paths_like_cpp() {
    let (aggro_manager, aggro_canonical, aggro_config, _) =
        creature_spell_bound_hook_tick_fixture_like_cpp("CombatAI", true, 70_203, 91_320, 91_321);
    let aggro =
        run_canonical_creature_spell_phase_for_test_like_cpp(&aggro_canonical, &aggro_config);
    assert_eq!(aggro.spell_runtime_hooks_unrepresented, 1);
    assert_eq!(aggro.casts_ready, 0);
    assert!(aggro.plan.events.is_empty());

    let (combat_manager, combat_canonical, combat_config, combat_creature_guid) =
        creature_spell_bound_hook_tick_fixture_like_cpp("CombatAI", false, 70_204, 91_322, 91_323);
    let initialized =
        run_canonical_creature_spell_phase_for_test_like_cpp(&combat_canonical, &combat_config);
    assert_eq!(initialized.schedules_initialized, 1);
    assert_eq!(initialized.spell_runtime_hooks_unrepresented, 0);
    combat_manager
        .lock()
        .unwrap()
        .find_creature_mut(0, 0, combat_creature_guid)
        .unwrap()
        .backdate_runtime_clock_for_test(Duration::from_secs(60));
    let combat =
        run_canonical_creature_spell_phase_for_test_like_cpp(&combat_canonical, &combat_config);
    assert_eq!(combat.spell_runtime_hooks_unrepresented, 1);
    assert_eq!(combat.casts_ready, 0);
    assert!(combat.plan.events.is_empty());
    assert!(
        combat_manager
            .lock()
            .unwrap()
            .find_creature(0, 0, combat_creature_guid)
            .unwrap()
            .creature_spell_due_in_ms_for_test(0)
            .is_some(),
        "CombatAI re-schedules after a failed DoCast attempt in C++"
    );

    let (turret_manager, turret_canonical, turret_config, _) =
        creature_spell_bound_hook_tick_fixture_like_cpp("TurretAI", false, 70_205, 91_324, 91_325);
    let turret =
        run_canonical_creature_spell_phase_for_test_like_cpp(&turret_canonical, &turret_config);
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
    let area = run_canonical_creature_spell_phase_for_test_like_cpp(&area_canonical, &area_config);
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
    let aura = run_canonical_creature_spell_phase_for_test_like_cpp(&aura_canonical, &aura_config);
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

    let outcome = run_canonical_creature_spell_phase_for_test_like_cpp(&canonical, &config);
    assert_eq!(outcome.spells_disabled, 1);
    assert_eq!(outcome.casts_ready, 0);
    assert!(outcome.plan.events.is_empty());
}
