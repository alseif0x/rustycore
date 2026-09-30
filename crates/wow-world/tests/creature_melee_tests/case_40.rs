use super::*;

/// C++ `Unit::MeleeDamageBonusDone` (`Unit.cpp:7558-7650`) for a creature
/// attacker. The map-owned creature path must apply its own aura effects
/// against the victim's creature type before the victim-side mitigation and
/// outcome stages. This was previously the remaining #29 dead producer: the
/// creature aura list existed, but the melee owner discarded it for damage.
#[test]
fn legacy_creature_melee_tick_once_applies_creature_attacker_done_bonus_like_cpp() {
    use wow_world::map_manager::RuntimeTickOwner;

    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    canonical.lock().unwrap().create_world_map(0, 0);
    let attacker_guid = test_creature_guid(91_920);
    let victim_guid = test_creature_guid(91_921);

    let (mut session, _, _) = make_session();
    session.set_canonical_map_manager(Arc::clone(&canonical));
    register_test_creature(&mut session, manager.clone(), attacker_guid, 100);
    register_test_creature(&mut session, manager.clone(), victim_guid, 100);
    session
        .fixture_melee_mutate_creature(attacker_guid, |creature| {
            creature.creature.unit_mut().set_level(80);
            creature.creature.ai_ownership_mut().min_damage = 10;
            creature.creature.ai_ownership_mut().max_damage = 10;
            creature.creature.set_flags_extra_runtime_like_cpp(
                wow_constants::CreatureFlagsExtra::NO_CRIT.bits(),
            );
            creature.enter_combat(victim_guid);
            creature.creature.ai_ownership_mut().last_swing_ms = 0;
            creature.creature.ai_ownership_mut().swing_timer_ms = 0;
        })
        .unwrap();
    session
        .fixture_melee_mutate_creature(victim_guid, |creature| {
            creature.creature.unit_mut().set_level(80);
            creature.creature.unit_mut().set_combat_reach(0.0);
            creature.creature.set_flags_extra_runtime_like_cpp(
                wow_constants::CreatureFlagsExtra::NO_CRIT.bits(),
            );
            creature
                .creature
                .unit_mut()
                .subsystems_mut()
                .auras
                .add_applied(wow_entities::AppliedAuraRef::new(
                    91_923,
                    attacker_guid,
                    0,
                    1,
                ));
        })
        .unwrap();

    let mut spell_store = wow_data::SpellStore::new();
    for (spell_id, aura_type, amount, misc_value) in [
        (
            91_920_i32,
            wow_data::spell::aura_types::SPELL_AURA_MOD_DAMAGE_DONE_CREATURE,
            5_i32,
            1_i32 << 6,
        ),
        (
            91_921,
            wow_data::spell::aura_types::SPELL_AURA_MOD_DAMAGE_DONE_VERSUS,
            100,
            1_i32 << 6,
        ),
        (
            91_923,
            wow_data::spell::aura_types::SPELL_AURA_MOD_ATTACKER_MELEE_HIT_CHANCE,
            5,
            0,
        ),
    ] {
        spell_store.insert(
            spell_id,
            wow_data::SpellInfo {
                spell_id,
                cast_time_ms: 0,
                cooldown_ms: 0,
                recovery_time_ms: 0,
                effect_type: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                effect_base_points: amount,
                effect_bonus_coefficient: 0.0,
                aura_type: Some(aura_type),
                display_flags: 0,
                requires_spell_focus: 0,
                power_costs: Vec::new(),
                effects: vec![wow_data::SpellEffectInfo {
                    effect_index: 0,
                    effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                    effect_aura: aura_type,
                    effect_misc_value_1: misc_value,
                    effect_base_points: amount,
                    ..Default::default()
                }],
            },
        );
    }
    let spell_store = Arc::new(spell_store);
    session.set_spell_store(Arc::clone(&spell_store));
    session
        .fixture_melee_mutate_creature(attacker_guid, |creature| {
            creature
                .creature
                .unit_mut()
                .subsystems_mut()
                .auras
                .add_applied(wow_entities::AppliedAuraRef::new(
                    91_920,
                    attacker_guid,
                    0,
                    1,
                ));
            creature
                .creature
                .unit_mut()
                .subsystems_mut()
                .auras
                .add_applied(wow_entities::AppliedAuraRef::new(
                    91_921,
                    attacker_guid,
                    0,
                    1,
                ));
        })
        .unwrap();
    let config = wow_world::session::LegacyCreatureAggroConfigLikeCpp {
        spell_store: Some(spell_store),
        creature_template_lifecycle_store: Some(Arc::new(
            wow_data::CreatureTemplateLifecycleStoreLikeCpp::from_templates([
                wow_data::CreatureTemplateLifecycleRecordLikeCpp {
                    entry: 9001,
                    creature_type: 7,
                    ..Default::default()
                },
            ]),
        )),
        ..Default::default()
    };
    manager
        .write()
        .unwrap()
        .set_tick_owner(RuntimeTickOwner::GlobalLegacy);

    let outcome = run_legacy_creature_melee_tick_once_like_cpp(&manager, Some(&canonical), &config);
    assert_eq!(outcome.canonical_creature_hits, 1);
    assert_eq!(
        canonical
            .lock()
            .unwrap()
            .find_map(0, 0)
            .unwrap()
            .map()
            .with_creature_like_cpp(victim_guid, |victim| victim.unit().data().health)
            .unwrap(),
        70
    );
}
