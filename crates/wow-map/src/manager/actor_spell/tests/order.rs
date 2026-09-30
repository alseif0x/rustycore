//! CombatAI due/init ordering, authoritative draws and lazy post-LOS work.
use super::*;
use rand::{Rng, SeedableRng, rngs::StdRng};

#[test]
fn los_resume_does_not_repeat_clock_mm_ai_policy_or_prefix_queries() {
    let (mut manager, guid, _) = setup(820_001);
    let before = prefix(&manager, guid);
    let trace = Trace::default();
    let (tick, mut token) = fixtures::start(
        &mut manager,
        91,
        MapObjectUpdateSelectionLikeCpp::WholeTypedStores,
    );
    let progress = policy(Settings::default(), &trace, |policies| {
        manager.prepare_spell(&tick, &mut token, true, policies)
    })
    .unwrap();
    let ActorSpellProgress::Pending(request) = progress else {
        panic!("LOS")
    };
    assert_eq!(prefix(&manager, guid), before);
    assert_eq!(trace.borrow().last(), Some(&"attributes"));
    assert!(!trace.borrow().contains(&"hit_metadata"));
    let original = trace.borrow().clone();
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
                true,
                policies
            ))
        .unwrap()
    );
    assert_eq!(outcome.canonical_cast_preconditions_passed, 1);
    assert_eq!(prefix(&manager, guid), before);
    assert_eq!(&trace.borrow()[..original.len()], original.as_slice());
    assert_eq!(
        trace
            .borrow()
            .iter()
            .filter(|value| **value == "ai")
            .count(),
        1
    );
    assert_eq!(
        trace
            .borrow()
            .iter()
            .filter(|value| **value == "range")
            .count(),
        1
    );
    assert_eq!(
        trace
            .borrow()
            .iter()
            .filter(|value| **value == "cooldown")
            .count(),
        1
    );
    assert!(token.actor_operation.is_none());
}

#[test]
fn all_primaries_prepare_before_first_consumption_or_los() {
    let (mut manager, guid, victim) = setup(820_010);
    let other =
        fixtures::insert_actor(&mut manager, 820_012, Position::xyz(12.0, 20.0, 30.0), true);
    configure(actor_mut(&mut manager, other), victim);
    let (tick, mut token) = fixtures::start(
        &mut manager,
        77,
        MapObjectUpdateSelectionLikeCpp::WholeTypedStores,
    );
    let trace = Trace::default();
    let ActorSpellProgress::Pending(request) = policy(Settings::default(), &trace, |policies| {
        manager.prepare_spell(&tick, &mut token, true, policies)
    })
    .unwrap() else {
        panic!("query")
    };
    assert_eq!(request.partial().casts_ready, 2);
    assert!(actor(&manager, guid).creature_spell_schedule_initialized_like_cpp());
    assert!(actor(&manager, other).creature_spell_schedule_initialized_like_cpp());
    let trace = trace.borrow();
    let first_cooldown = trace.iter().position(|value| *value == "cooldown").unwrap();
    assert_eq!(
        trace[..first_cooldown]
            .iter()
            .filter(|value| **value == "ai")
            .count(),
        2
    );
    drop(request);
    assert!(token.actor_operation.is_some());
}

#[test]
fn first_due_is_removed_before_missing_metadata_and_no_repeat_is_fabricated() {
    let (mut manager, guid, _) = setup(820_020);
    let current = actor_mut(&mut manager, guid);
    current.mark_creature_spell_schedule_initialized_like_cpp();
    current.schedule_creature_spell_slot_after_like_cpp(0, 0);
    let (tick, mut token) = fixtures::start(
        &mut manager,
        100,
        MapObjectUpdateSelectionLikeCpp::WholeTypedStores,
    );
    let settings = Settings {
        missing: true,
        ..Settings::default()
    };
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
    assert_eq!(outcome.missing_spell_metadata, 1);
    assert_eq!(
        actor(&manager, guid).first_due_creature_spell_slot_like_cpp(),
        None
    );
    assert!(actor(&manager, guid).runtime_rng_authority_complete_like_cpp());
}

#[test]
fn casting_blocks_due_after_initialization_and_schedule_draw_consumption() {
    let (mut manager, guid, _) = setup(820_030);
    actor_mut(&mut manager, guid)
        .creature
        .unit_mut()
        .add_unit_state(wow_constants::UnitState::CASTING.bits());
    let (tick, mut token) = fixtures::start(
        &mut manager,
        77,
        MapObjectUpdateSelectionLikeCpp::WholeTypedStores,
    );
    let settings = Settings {
        condition: SpellCondition::Combat,
        ..Settings::default()
    };
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
    assert_eq!(outcome.schedules_initialized, 1);
    assert_eq!(outcome.unit_state_casting_skips, 1);
    assert!(
        actor(&manager, guid)
            .creature_spell_due_in_ms_for_test(0)
            .is_some()
    );
}

#[test]
fn represented_miss_draw_precedes_repeat_delay_then_next_shared_roll() {
    let (mut manager, guid, _) = setup(820_040);
    let seed = (0..10_000_u64)
        .find(|seed| StdRng::seed_from_u64(*seed).gen_range(0..=9_999_u32) < 500)
        .unwrap();
    let mut expected = StdRng::seed_from_u64(seed);
    assert!(expected.gen_range(0..=9_999_u32) < 500);
    let delay = expected.gen_range(6_000_u64..=12_000);
    let next = expected.gen_range(0..=9_999_u32);
    let current = actor_mut(&mut manager, guid);
    current.seed_runtime_rng_like_cpp(seed);
    current.mark_creature_spell_schedule_initialized_like_cpp();
    current.schedule_creature_spell_slot_after_like_cpp(0, 0);
    let (tick, mut token) = fixtures::start(
        &mut manager,
        77,
        MapObjectUpdateSelectionLikeCpp::WholeTypedStores,
    );
    let outcome = complete!(
        &mut manager,
        &tick,
        &mut token,
        Settings::default(),
        &Trace::default(),
        policy(Settings::default(), &Trace::default(), |policies| manager
            .prepare_spell(&tick, &mut token, false, policies))
        .unwrap()
    );
    assert_eq!(outcome.spell_misses, 1);
    let current = actor_mut(&mut manager, guid);
    assert_eq!(current.creature_spell_due_in_ms_for_test(0), Some(delay));
    assert_eq!(
        current.random_creature_spell_hit_roll_like_cpp(),
        Some(next)
    );
}

#[test]
fn hit_tombstone_prevents_later_initial_schedule_from_drawing() {
    let (mut manager, guid, _) = setup(820_050);
    actor_mut(&mut manager, guid).creature.set_spell(1, 70_102);
    let (tick, mut token) = fixtures::start(
        &mut manager,
        77,
        MapObjectUpdateSelectionLikeCpp::WholeTypedStores,
    );
    let settings = Settings {
        mixed: true,
        no_miss: true,
        ..Settings::default()
    };
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
    assert_eq!(outcome.spell_hits, 1);
    assert_eq!(outcome.runtime_rng_authority_rejections, 1);
    assert_eq!(
        actor(&manager, guid).creature_spell_due_in_ms_for_test(1),
        None
    );
    assert!(!actor(&manager, guid).runtime_rng_authority_complete_like_cpp());
}

#[test]
fn unrepresented_random_threat_clears_due_tombstones_and_never_schedules() {
    let (mut manager, guid, _) = setup(820_060);
    let current = actor_mut(&mut manager, guid);
    current.mark_creature_spell_schedule_initialized_like_cpp();
    current.schedule_creature_spell_slot_after_like_cpp(0, 0);
    let (tick, mut token) = fixtures::start(
        &mut manager,
        77,
        MapObjectUpdateSelectionLikeCpp::WholeTypedStores,
    );
    let settings = Settings {
        target: SpellTarget::Enemy,
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
    assert_eq!(outcome.spell_effects_unrepresented, 1);
    assert!(!trace.borrow().contains(&"minimum"));
    assert!(!actor(&manager, guid).runtime_rng_authority_complete_like_cpp());
    assert_eq!(
        actor(&manager, guid).creature_spell_due_in_ms_for_test(0),
        None
    );
}

#[test]
fn ignore_los_and_terrain_none_fastpaths_never_create_pending() {
    for (counter, ignore, terrain) in [(820_070, true, true), (820_072, false, false)] {
        let (mut manager, _, _) = setup(counter);
        let (tick, mut token) = fixtures::start(
            &mut manager,
            77,
            MapObjectUpdateSelectionLikeCpp::WholeTypedStores,
        );
        let settings = Settings {
            ignore_los: ignore,
            ..Settings::default()
        };
        let outcome = complete!(
            &mut manager,
            &tick,
            &mut token,
            settings,
            &Trace::default(),
            policy(settings, &Trace::default(), |policies| manager
                .prepare_spell(&tick, &mut token, terrain, policies))
            .unwrap()
        );
        assert_eq!(outcome.canonical_cast_preconditions_passed, 1);
    }
}
