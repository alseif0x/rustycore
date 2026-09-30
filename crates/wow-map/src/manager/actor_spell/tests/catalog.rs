//! Exact rejection boundaries and represented signed-power/log metadata.
use super::*;

#[test]
fn turret_strict_raw_boundary_never_resets_but_inner_min_range_rejection_does() {
    for (counter, distance, minimum, expected_swing) in
        [(822_001, 30.0, 0.0, false), (822_003, 1.0, 5.0, true)]
    {
        let (mut manager, guid, victim) = setup(counter);
        let base = actor(&manager, guid).position();
        actor_mut(&mut manager, guid)
            .creature
            .unit_mut()
            .world_mut()
            .set_combat_reach(0.0);
        let player = manager
            .find_map_mut(1, 0)
            .unwrap()
            .map_mut()
            .get_typed_player_mut(victim)
            .unwrap();
        player.unit_mut().world_mut().set_combat_reach(0.0);
        player
            .unit_mut()
            .world_mut()
            .relocate(Position::xyz(base.x + distance, base.y, base.z));
        let (tick, mut token) = fixtures::start(
            &mut manager,
            77,
            MapObjectUpdateSelectionLikeCpp::WholeTypedStores,
        );
        assert!(actor(&manager, guid).can_swing());
        let settings = Settings {
            ai: SpellAiKind::Turret,
            range: SpellRange {
                minimum,
                maximum: 30.0,
                flags: 0,
            },
            ..Settings::default()
        };
        let trace = Trace::default();
        let outcome = complete!(
            &mut manager,
            &tick,
            &mut token,
            settings,
            &trace,
            policy(settings, &trace, |policies| manager
                .prepare_spell(&tick, &mut token, true, policies))
            .unwrap()
        );
        assert_eq!(outcome.spell_range_rejections, 1);
        assert_eq!(!actor(&manager, guid).can_swing(), expected_swing);
        assert!(!trace.borrow().contains(&"attributes"));
    }
}

#[test]
fn disabled_turret_uses_raw_range_to_consume_attempt_without_hit_or_wire() {
    let (mut manager, guid, _) = setup(822_010);
    let (tick, mut token) = fixtures::start(
        &mut manager,
        77,
        MapObjectUpdateSelectionLikeCpp::WholeTypedStores,
    );
    let settings = Settings {
        ai: SpellAiKind::Turret,
        disable: SpellDisable::Disabled,
        ..Settings::default()
    };
    let trace = Trace::default();
    let outcome = complete!(
        &mut manager,
        &tick,
        &mut token,
        settings,
        &trace,
        policy(settings, &trace, |policies| manager
            .prepare_spell(&tick, &mut token, true, policies))
        .unwrap()
    );
    assert_eq!(outcome.spells_disabled, 1);
    assert_eq!(outcome.turret_rejected_attempt_swings, 1);
    assert!(!actor(&manager, guid).can_swing());
    assert!(outcome.completions.is_empty());
    assert!(!trace.borrow().contains(&"cooldown"));
    assert!(!trace.borrow().contains(&"hit_metadata"));
    assert!(actor(&manager, guid).runtime_rng_authority_complete_like_cpp());
}

#[test]
fn canonical_cooldown_rejection_precedes_range_attributes_los_and_hit() {
    let (mut manager, guid, _) = setup(822_020);
    actor_mut(&mut manager, guid)
        .creature
        .unit_mut()
        .subsystems_mut()
        .spells
        .history
        .start_cooldown(0, 70_101, 0, 10_000, 0, 0, false);
    let profile = SpellCooldown {
        spell_id: 70_101,
        category_id: 0,
        recovery_time_ms: 10_000,
        category_recovery_time_ms: 0,
        passive: false,
    };
    let settings = Settings {
        ai: SpellAiKind::Turret,
        cooldown: Some(profile),
        ..Settings::default()
    };
    let (tick, mut token) = fixtures::start(
        &mut manager,
        77,
        MapObjectUpdateSelectionLikeCpp::WholeTypedStores,
    );
    let trace = Trace::default();
    let outcome = complete!(
        &mut manager,
        &tick,
        &mut token,
        settings,
        &trace,
        policy(settings, &trace, |policies| manager
            .prepare_spell(&tick, &mut token, true, policies))
        .unwrap()
    );
    assert_eq!(outcome.canonical_cast_cooldown_rejections, 1);
    assert!(!actor(&manager, guid).can_swing());
    assert!(!trace.borrow().contains(&"attributes"));
    assert!(!trace.borrow().contains(&"hit_metadata"));
}

#[test]
fn admitted_non_turret_resets_before_missing_hit_profile_tombstone() {
    let (mut manager, guid, _) = setup(822_030);
    let (tick, mut token) = fixtures::start(
        &mut manager,
        77,
        MapObjectUpdateSelectionLikeCpp::WholeTypedStores,
    );
    let settings = Settings {
        hit_missing: true,
        ..Settings::default()
    };
    let trace = Trace::default();
    let outcome = complete!(
        &mut manager,
        &tick,
        &mut token,
        settings,
        &trace,
        policy(settings, &trace, |policies| manager
            .prepare_spell(&tick, &mut token, false, policies))
        .unwrap()
    );
    assert_eq!(outcome.spell_hit_results_unrepresented, 1);
    assert!(!actor(&manager, guid).can_swing());
    assert!(!actor(&manager, guid).runtime_rng_authority_complete_like_cpp());
    let trace = trace.borrow();
    assert!(
        trace
            .iter()
            .position(|value| *value == "hit_metadata")
            .unwrap()
            < trace
                .iter()
                .position(|value| *value == "reset_attribute")
                .unwrap()
    );
}

#[test]
fn no_attack_miss_still_requires_one_authoritative_bounded_roll() {
    let profile = SpellHitProfile::NoAttackMissAfterRequiredRoll;
    assert_eq!(resolve_hit_profile(profile, None), None);
    assert_eq!(resolve_hit_profile(profile, Some(10_000)), None);
    assert_eq!(resolve_hit_profile(profile, Some(0)), Some(SpellHit::Hit));
    let metadata = SpellHitFacts {
        defense_type: 2,
        school_mask: 1,
        spell_mechanic: 0,
        effect_mechanics: [(0, 0)].into_iter().collect(),
    };
    let mut attributes = [0; 15];
    attributes[0] = 0x10;
    attributes[7] = 0x0200_0000;
    assert_eq!(
        represented_hit_profile(&metadata, &[0], attributes),
        Some(profile)
    );
}

#[test]
fn full_log_preserves_signed_unknown_power_order_dedup_and_primary_row() {
    let (manager, guid, _) = setup(822_040);
    let mut spell = info(70_101);
    for raw in [127_i8, -2, 127, 3] {
        spell.power_costs.push(SpellPowerFacts {
            power_type: raw,
            mana_cost: 0,
            mana_cost_per_level: 0,
            mana_per_second: 0,
            power_cost_pct: 0.0,
            power_cost_max_pct: 0.0,
            power_pct_per_second: 0.0,
            required_aura_spell_id: 0,
            optional_cost: 0,
        });
    }
    let caster = &actor(&manager, guid).creature;
    let log = cast_log(caster, &spell, 0).unwrap();
    let mut expected = vec![127, -2, 3];
    let primary = caster.power_type() as i32;
    if !expected.contains(&primary) {
        expected.insert(0, primary);
    }
    assert_eq!(
        log.power_data
            .iter()
            .map(|row| row.power_type)
            .collect::<Vec<_>>(),
        expected
    );
    assert_eq!(log.health, caster.current_health() as i64);
    assert!(
        log.power_data
            .iter()
            .filter(|row| row.power_type == 127 || row.power_type == -2)
            .all(|row| row.amount == 0 && row.cost == 0)
    );
    assert!(cast_log(caster, &spell, 1).is_none());
    spell.power_costs[0].optional_cost = 1;
    assert!(cast_log(caster, &spell, 0).is_none());
}

#[test]
fn blocked_los_leaves_cooldown_guid_hit_draw_and_following_repeat_lazy() {
    let (mut manager, tick, mut token, guid, _, request) = pending(822_050);
    let trace = Trace::default();
    let outcome = complete!(
        &mut manager,
        &tick,
        &mut token,
        Settings::default(),
        &trace,
        policy(Settings::default(), &trace, |policies| manager
            .resume_spell_los(
                &tick,
                &mut token,
                request.into_continuation(),
                false,
                policies
            ))
        .unwrap()
    );
    assert_eq!(outcome.spell_los_rejections, 1);
    assert_eq!(outcome.casts_ready, 0);
    assert!(outcome.completions.is_empty());
    assert!(trace.borrow().is_empty());
    assert!(actor(&manager, guid).runtime_rng_authority_complete_like_cpp());
}

#[test]
fn successful_cooldown_is_visible_before_owned_cast_guid_completion() {
    let (mut manager, guid, _) = setup(822_060);
    let profile = SpellCooldown {
        spell_id: 70_101,
        category_id: 17,
        recovery_time_ms: 500,
        category_recovery_time_ms: 700,
        passive: false,
    };
    let settings = Settings {
        cooldown: Some(profile),
        no_miss: true,
        ..Settings::default()
    };
    let (tick, mut token) = fixtures::start(
        &mut manager,
        77,
        MapObjectUpdateSelectionLikeCpp::WholeTypedStores,
    );
    let outcome = complete!(
        &mut manager,
        &tick,
        &mut token,
        settings,
        &Trace::default(),
        policy(settings, &Trace::default(), |policies| manager
            .prepare_spell(&tick, &mut token, false, policies))
        .unwrap()
    );
    let completion = &outcome.completions[0];
    assert_eq!(completion.cast_id.high_guid(), wow_core::HighGuid::Cast);
    assert!(
        actor(&manager, guid)
            .creature
            .unit()
            .subsystems()
            .spells
            .history
            .has_cooldown(70_101, 17, 0)
    );
    assert_eq!(completion.position, actor(&manager, guid).position());
}
