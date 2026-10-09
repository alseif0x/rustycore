//! #1263 F6-8D2 regressions: the admitted tick drives the canonical engine
//! exactly once, and a recreated incarnation or record is refused before it can.
//!
//! D1 ported the per-creature phase operations onto canonical ownership. D2
//! connects them to an **admitted** transition: the map incarnations, the
//! selected canonical objects and the tick diff captured for one tick. The
//! tests below pin the three contracts of that connection:
//!
//! 1. one diff/clock advancement, one due cast, one swing commit, one canonical
//!    effect and one deferred publication per admitted transition;
//! 2. the reviewer's recreation/ABA falsifier — a map key or a GUID record
//!    recreated as N+1 before execution gets **zero** clock, timer, RNG, health
//!    and publication effects, because admission is a fence evaluated before the
//!    first mutation rather than a post-hoc count;
//! 3. one execution owner per admitted transition: the canonical, legacy and
//!    session candidates attempted together produce exactly one of each
//!    observable and the inactive owners produce none.

use super::*;

/// Coordinator identity the isolated path claims these transitions under.
const F6_8D2_COORDINATOR_LIKE_CPP: u64 = 0x0F6D_2000;
/// The one admitted swing of the fixture attacker.
const F6_8D2_SWING_DAMAGE_LIKE_CPP: u32 = 10;
/// RNG seed of the admitted attacker, so the committed roll is exact.
const F6_8D2_ATTACKER_SEED_LIKE_CPP: u64 = 0x0F6D_2D2;
/// RNG seed of the replacement attacker and its untouched witness.
const F6_8D2_REPLACEMENT_SEED_LIKE_CPP: u64 = 0x0F6D_2D3;

fn f6_8d2_key_like_cpp() -> wow_map::MapKey {
    wow_map::MapKey::new(0, 0)
}

fn f6_8d2_position_like_cpp(x: f32) -> Position {
    Position::new(x, 10.0, 0.0, 0.0)
}

fn f6_8d2_lease_like_cpp() -> SharedCreatureExecutionLeaseLikeCpp {
    std::sync::Arc::new(std::sync::Mutex::new(CreatureExecutionLeaseLikeCpp::new()))
}

/// Arm one canonical creature for exactly one admitted transition: a due
/// `CombatAI::_events` slot, a ready swing, an exact damage roll and an exact
/// creature RNG position.
fn arm_admitted_attacker_like_cpp(
    canonical: &SharedCanonicalMapManager,
    guid: ObjectGuid,
    victim: ObjectGuid,
    seed: u64,
) {
    let mut guard = canonical
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let map = guard
        .find_map_mut(0, 0)
        .expect("the fixture attached the canonical map instance");
    map.map_mut()
        .with_creature_mut_like_cpp(guid, |creature| {
            creature.unit_mut().set_max_health(100);
            creature.unit_mut().set_health(100);
            creature.ai_ownership_mut().min_damage = F6_8D2_SWING_DAMAGE_LIKE_CPP;
            creature.ai_ownership_mut().max_damage = F6_8D2_SWING_DAMAGE_LIKE_CPP;
            creature
                .runtime_like_cpp_mut()
                .seed_runtime_rng_like_cpp(seed);
            // `enter_combat` resets `CombatAI::_events`, so the due slot is
            // scheduled after the engagement, exactly as `CombatAI::JustEngagedWith`.
            creature.enter_combat(victim);
            creature
                .unit_mut()
                .subsystems_mut()
                .combat
                .current_victim_guid = Some(victim);
            creature.ai_ownership_mut().last_swing_ms = 0;
            creature.ai_ownership_mut().swing_timer_ms = 0;
            creature.schedule_creature_spell_slot_after_like_cpp(0, 0);
        })
        .expect("the admitted attacker is a canonical creature");
}

/// The complete observable state the admitted transition would move.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct AdmittedAttackerStateLikeCpp {
    elapsed_ms: u64,
    last_swing_ms: u64,
    swing_timer_ms: u64,
    health: u64,
    due_slot: Option<usize>,
    aura_applications: usize,
}

fn admitted_attacker_state_like_cpp(
    canonical: &SharedCanonicalMapManager,
    guid: ObjectGuid,
) -> AdmittedAttackerStateLikeCpp {
    let guard = canonical
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    guard
        .find_map(0, 0)
        .expect("the fixture attached the canonical map instance")
        .map()
        .with_creature_like_cpp(guid, |creature| AdmittedAttackerStateLikeCpp {
            elapsed_ms: creature.runtime_elapsed_ms_like_cpp(),
            last_swing_ms: creature.ai_ownership().last_swing_ms,
            swing_timer_ms: creature.ai_ownership().swing_timer_ms,
            health: creature.unit().data().health,
            due_slot: creature.first_due_creature_spell_slot_like_cpp(),
            aura_applications: creature
                .unit()
                .subsystems()
                .auras
                .runtime_applications_like_cpp()
                .len(),
        })
        .expect("the fixture creatue is a canonical creature")
}

fn canonical_creature_health_like_cpp(
    canonical: &SharedCanonicalMapManager,
    guid: ObjectGuid,
) -> u64 {
    let guard = canonical
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    guard
        .find_map(0, 0)
        .expect("the fixture attached the canonical map instance")
        .map()
        .with_creature_like_cpp(guid, |creature| creature.unit().data().health)
        .expect("the fixture creature is a canonical creature")
}

/// The next draw of one creature's own RNG stream. Two creatures seeded
/// identically produce the same value only while neither stream moved, so this
/// is an exact position probe rather than a plausibility check.
fn next_creature_roll_like_cpp(
    canonical: &SharedCanonicalMapManager,
    guid: ObjectGuid,
) -> Option<u32> {
    let mut guard = canonical
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    guard
        .find_map_mut(0, 0)
        .expect("the fixture attached the canonical map instance")
        .map_mut()
        .with_creature_mut_like_cpp(guid, wow_entities::Creature::roll_damage)
        .expect("the fixture creature is a canonical creature")
}

/// Admit one tick on the canonical manager and freeze the transition under test.
fn admitted_tick_and_capture_like_cpp(
    canonical: &SharedCanonicalMapManager,
    objects: &[(wow_map::MapKey, ObjectGuid)],
) -> (
    wow_map::MapTickPlanLikeCpp,
    AdmittedCreatureExecutionLikeCpp,
) {
    let plan = canonical
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .begin_tick_like_cpp(1)
        .into_started()
        .expect("the fixture map update timer has passed");
    assert_eq!(
        canonical
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .tick_coordination_like_cpp(),
        wow_map::MapTickCoordinationStateLikeCpp::AwaitingSessions(plan.epoch_like_cpp()),
        "an admitted tick awaits its sessions before its remaining phases"
    );
    let admission = capture_admitted_creature_execution_like_cpp(
        canonical,
        F6_8D2_COORDINATOR_LIKE_CPP,
        plan.epoch_like_cpp(),
        plan.effective_diff_ms(),
        plan.updated_maps_like_cpp()
            .iter()
            .map(|participant| (participant.key, participant.incarnation)),
        objects.iter().copied(),
    )
    .expect("the canonical owner is readable, so the admission is captured");
    (plan, admission)
}

#[test]
fn isolated_admitted_execution_consumes_the_diff_effects_and_defers_publication_like_cpp() {
    let canonical = shared_canonical_map_manager();
    let attacker = test_creature_guid(94_201);
    let victim = test_creature_guid(94_202);
    add_canonical_test_creature_on_map(
        &canonical,
        attacker,
        9001,
        f6_8d2_position_like_cpp(10.0),
        0,
        0,
        0,
    );
    add_canonical_test_creature_on_map(
        &canonical,
        victim,
        9002,
        f6_8d2_position_like_cpp(11.0),
        0,
        0,
        0,
    );
    arm_admitted_attacker_like_cpp(&canonical, attacker, victim, F6_8D2_ATTACKER_SEED_LIKE_CPP);

    let (_plan, admission) =
        admitted_tick_and_capture_like_cpp(&canonical, &[(f6_8d2_key_like_cpp(), attacker)]);
    assert_eq!(admission.maps.len(), 1);
    assert_eq!(admission.objects.len(), 1, "the tick selected one creature");
    assert!(admission.diff_ms > 0, "the admitted tick saved a real diff");

    let lease = f6_8d2_lease_like_cpp();
    let outcome = run_admitted_creature_execution_isolated_like_cpp(&admission, &canonical, &lease);

    assert_eq!(
        outcome.admission,
        CreatureExecutionAdmissionLikeCpp::Admitted
    );
    assert_eq!(
        outcome.owner,
        CreatureExecutionOwnerLikeCpp::CanonicalAdmitted
    );
    assert_eq!(
        outcome.clock_advanced_ms,
        u64::from(admission.diff_ms),
        "the admitted transition advances the clock exactly once, by the saved diff"
    );
    assert_eq!(
        outcome.due_casts, 1,
        "one due CombatAI event is consumed per admitted update"
    );
    assert_eq!(
        outcome.swing_damage,
        u64::from(F6_8D2_SWING_DAMAGE_LIKE_CPP),
        "one swing commits exactly one resolved roll"
    );
    assert_eq!(outcome.effects_consumed, 1);
    assert_eq!(outcome.publication_events_like_cpp(), 1);
    assert!(
        !outcome.publication_delivered,
        "the publication is deferred to its consumer, not delivered under the map guard"
    );
    let published = &outcome.deferred_publication[0];
    assert_eq!(published.attacker_guid, attacker);
    assert_eq!(published.victim_guid, victim);
    assert_eq!(published.damage, F6_8D2_SWING_DAMAGE_LIKE_CPP);

    // The engine's own observables, not only its self-report.
    let after = admitted_attacker_state_like_cpp(&canonical, attacker);
    assert_eq!(after.elapsed_ms, u64::from(admission.diff_ms));
    assert_eq!(
        after.due_slot, None,
        "the due CombatAI event was consumed exactly once"
    );
    assert_eq!(after.health, 100, "the attacker is not its own victim");
    assert_eq!(
        canonical_creature_health_like_cpp(&canonical, victim),
        100 - u64::from(F6_8D2_SWING_DAMAGE_LIKE_CPP),
        "the one canonical effect landed on the canonical victim"
    );

    // A second owner of the same transition is refused before it touches the
    // engine: nothing advances, no cast is consumed and nothing is published.
    let clock_before = admitted_attacker_state_like_cpp(&canonical, attacker);
    let victim_health_before = canonical_creature_health_like_cpp(&canonical, victim);
    let second = run_admitted_creature_execution_isolated_like_cpp(&admission, &canonical, &lease);
    assert_eq!(
        second.admission,
        CreatureExecutionAdmissionLikeCpp::RefusedOwnerLease {
            held_by: CreatureExecutionOwnerLikeCpp::CanonicalAdmitted,
        }
    );
    assert_eq!(second.clock_advanced_ms, 0);
    assert_eq!(second.due_casts, 0);
    assert_eq!(second.swing_damage, 0);
    assert_eq!(second.effects_consumed, 0);
    assert_eq!(second.publication_events_like_cpp(), 0);
    assert_eq!(
        admitted_attacker_state_like_cpp(&canonical, attacker),
        clock_before,
        "a refused second owner leaves the transition clock and timers untouched"
    );
    assert_eq!(
        canonical_creature_health_like_cpp(&canonical, victim),
        victim_health_before
    );
}

#[test]
fn recreated_map_incarnation_refuses_the_admitted_transition_like_cpp() {
    let canonical = shared_canonical_map_manager();
    let attacker = test_creature_guid(94_211);
    let victim = test_creature_guid(94_212);
    let witness = test_creature_guid(94_213);
    add_canonical_test_creature_on_map(
        &canonical,
        attacker,
        9001,
        f6_8d2_position_like_cpp(10.0),
        0,
        0,
        0,
    );
    add_canonical_test_creature_on_map(
        &canonical,
        victim,
        9002,
        f6_8d2_position_like_cpp(11.0),
        0,
        0,
        0,
    );
    arm_admitted_attacker_like_cpp(&canonical, attacker, victim, F6_8D2_ATTACKER_SEED_LIKE_CPP);

    let (_plan, admission) =
        admitted_tick_and_capture_like_cpp(&canonical, &[(f6_8d2_key_like_cpp(), attacker)]);
    let incarnation_n_before = canonical
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .map_incarnation_like_cpp(f6_8d2_key_like_cpp())
        .expect("the admitted map key has an incarnation");

    // The reviewer's falsifier: the same map key and the same GUID are recreated
    // as N+1 while the committed swing and the due cast of the admitted
    // transition are still owed. A post-hoc envelope count would let this
    // replacement take them.
    assert!(
        canonical
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .destroy_map(0, 0),
        "a creature-only fixture map is evacuable"
    );
    add_canonical_test_creature_on_map(
        &canonical,
        attacker,
        9001,
        f6_8d2_position_like_cpp(10.0),
        0,
        0,
        0,
    );
    add_canonical_test_creature_on_map(
        &canonical,
        victim,
        9002,
        f6_8d2_position_like_cpp(11.0),
        0,
        0,
        0,
    );
    add_canonical_test_creature_on_map(
        &canonical,
        witness,
        9003,
        f6_8d2_position_like_cpp(12.0),
        0,
        0,
        0,
    );
    arm_admitted_attacker_like_cpp(
        &canonical,
        attacker,
        victim,
        F6_8D2_REPLACEMENT_SEED_LIKE_CPP,
    );
    arm_admitted_attacker_like_cpp(
        &canonical,
        witness,
        victim,
        F6_8D2_REPLACEMENT_SEED_LIKE_CPP,
    );
    let incarnation_n_plus_one = canonical
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .map_incarnation_like_cpp(f6_8d2_key_like_cpp())
        .expect("the recreated map key has a new incarnation");
    assert_ne!(
        incarnation_n_before, incarnation_n_plus_one,
        "the same map key now resolves to a different incarnation"
    );

    let replacement_before = admitted_attacker_state_like_cpp(&canonical, attacker);
    let victim_health_before = canonical_creature_health_like_cpp(&canonical, victim);

    let outcome = run_admitted_creature_execution_isolated_like_cpp(
        &admission,
        &canonical,
        &f6_8d2_lease_like_cpp(),
    );

    assert_eq!(
        outcome.admission,
        CreatureExecutionAdmissionLikeCpp::RefusedMapIncarnation {
            map_id: 0,
            instance_id: 0,
            expected: incarnation_n_before,
            current: Some(incarnation_n_plus_one),
        },
        "the recreated key is refused before the first mutation"
    );
    assert_eq!(outcome.clock_advanced_ms, 0);
    assert_eq!(outcome.due_casts, 0);
    assert_eq!(outcome.swing_damage, 0);
    assert_eq!(outcome.effects_consumed, 0);
    assert_eq!(outcome.publication_events_like_cpp(), 0);
    assert!(
        !outcome.publication_delivered,
        "a refusal publishes nothing, stale or otherwise"
    );

    // The replacement is untouched, byte for byte, and its RNG stream is at the
    // position an identical witness the transition never named still holds.
    assert_eq!(
        admitted_attacker_state_like_cpp(&canonical, attacker),
        replacement_before,
        "a refused transition mutates no timer, health or aura of the replacement"
    );
    assert_eq!(
        canonical_creature_health_like_cpp(&canonical, victim),
        victim_health_before
    );
    assert_eq!(
        next_creature_roll_like_cpp(&canonical, attacker),
        next_creature_roll_like_cpp(&canonical, witness),
        "a refused transition does not advance the replacement RNG sequence"
    );
}

#[test]
fn replaced_creature_record_refuses_the_admitted_transition_like_cpp() {
    let canonical = shared_canonical_map_manager();
    let attacker = test_creature_guid(94_221);
    let victim = test_creature_guid(94_222);
    let witness = test_creature_guid(94_223);
    for (guid, entry, x) in [
        (attacker, 9001_u32, 10.0_f32),
        (victim, 9002, 11.0),
        (witness, 9003, 12.0),
    ] {
        add_canonical_test_creature_on_map(
            &canonical,
            guid,
            entry,
            f6_8d2_position_like_cpp(x),
            0,
            0,
            0,
        );
    }
    arm_admitted_attacker_like_cpp(&canonical, attacker, victim, F6_8D2_ATTACKER_SEED_LIKE_CPP);

    let (_plan, admission) =
        admitted_tick_and_capture_like_cpp(&canonical, &[(f6_8d2_key_like_cpp(), attacker)]);
    let incarnation = canonical
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .map_incarnation_like_cpp(f6_8d2_key_like_cpp())
        .expect("the admitted map key has an incarnation");

    // The same map incarnation, the same GUID, a different record allocation:
    // the admitted identity names an object the map no longer owns.
    add_canonical_test_creature_on_map(
        &canonical,
        attacker,
        9001,
        f6_8d2_position_like_cpp(10.0),
        0,
        0,
        0,
    );
    arm_admitted_attacker_like_cpp(
        &canonical,
        attacker,
        victim,
        F6_8D2_REPLACEMENT_SEED_LIKE_CPP,
    );
    arm_admitted_attacker_like_cpp(
        &canonical,
        witness,
        victim,
        F6_8D2_REPLACEMENT_SEED_LIKE_CPP,
    );
    assert_eq!(
        canonical
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .map_incarnation_like_cpp(f6_8d2_key_like_cpp()),
        Some(incarnation),
        "the map key still resolves to the admitted incarnation"
    );

    let replacement_before = admitted_attacker_state_like_cpp(&canonical, attacker);
    let outcome = run_admitted_creature_execution_isolated_like_cpp(
        &admission,
        &canonical,
        &f6_8d2_lease_like_cpp(),
    );

    assert_eq!(
        outcome.admission.refusal_label_like_cpp(),
        Some("object-replaced"),
        "the same GUID under the same incarnation with a new record is refused"
    );
    assert_eq!(outcome.clock_advanced_ms, 0);
    assert_eq!(outcome.due_casts, 0);
    assert_eq!(outcome.swing_damage, 0);
    assert_eq!(outcome.effects_consumed, 0);
    assert_eq!(outcome.publication_events_like_cpp(), 0);
    assert_eq!(
        admitted_attacker_state_like_cpp(&canonical, attacker),
        replacement_before
    );
    assert_eq!(
        next_creature_roll_like_cpp(&canonical, attacker),
        next_creature_roll_like_cpp(&canonical, witness),
        "a refused transition does not advance the replacement RNG sequence"
    );
}

#[test]
fn production_composition_admits_exactly_one_execution_owner_like_cpp() {
    use crate::map_manager::RuntimeTickOwner;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering};

    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    let (mut session, _, _) = make_session();
    let attacker = test_creature_guid(94_231);
    let victim = test_creature_guid(94_232);
    register_test_creature_mirrored_like_cpp(
        &mut session,
        manager.clone(),
        &canonical,
        attacker,
        100,
    );
    add_canonical_test_creature_on_map(
        &canonical,
        victim,
        9002,
        f6_8d2_position_like_cpp(11.0),
        0,
        0,
        0,
    );
    arm_admitted_attacker_like_cpp(&canonical, attacker, victim, F6_8D2_ATTACKER_SEED_LIKE_CPP);
    manager
        .write()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .set_tick_owner(RuntimeTickOwner::GlobalLegacy);

    let (_plan, admission) =
        admitted_tick_and_capture_like_cpp(&canonical, &[(f6_8d2_key_like_cpp(), attacker)]);
    assert_eq!(
        admission.objects.len(),
        1,
        "the mirrored fixture creature is one canonical incarnation"
    );
    let lease = f6_8d2_lease_like_cpp();

    let session_arm_invocations = Arc::new(AtomicUsize::new(0));
    let session_arm_counter = Arc::clone(&session_arm_invocations);
    let mut session_arm = move || {
        session_arm_counter.fetch_add(1, AtomicOrdering::Relaxed);
        IsolatedSessionArmOutcomeLikeCpp { publications: 1 }
    };

    // The three production owners are attempted together for one admitted
    // transition, with the canonical admitted path armed exactly as D3 will arm
    // it. Exactly one of them may execute.
    let composed = run_isolated_admitted_creature_execution_composition_like_cpp(
        &admission,
        true,
        &canonical,
        &manager,
        &lease,
        RuntimeTickOwner::GlobalLegacy,
        &LegacyCreatureAggroConfigLikeCpp::default(),
        Some(&mut session_arm),
    );

    let canonical_outcome = composed
        .canonical
        .as_ref()
        .expect("the canonical admitted owner executed");
    assert_eq!(
        composed.admitted_owner,
        Some(CreatureExecutionOwnerLikeCpp::CanonicalAdmitted),
        "exactly one owner is admitted to the transition"
    );
    assert!(
        composed.legacy.is_none(),
        "the legacy global owner produced no effect while the canonical claim held the transition"
    );
    assert!(
        composed.session.is_none(),
        "the session owner produced no effect while the canonical claim held the transition"
    );
    assert_eq!(
        session_arm_invocations.load(AtomicOrdering::Relaxed),
        0,
        "an inactive owner is not touched at all"
    );
    assert_eq!(
        canonical_outcome.clock_advanced_ms,
        u64::from(admission.diff_ms),
        "exactly one clock advancement"
    );
    assert_eq!(canonical_outcome.due_casts, 1, "exactly one due cast");
    assert_eq!(
        canonical_outcome.swing_damage,
        u64::from(F6_8D2_SWING_DAMAGE_LIKE_CPP),
        "exactly one swing commit"
    );
    assert_eq!(canonical_outcome.effects_consumed, 1);
    assert_eq!(
        composed.publication_events_like_cpp(),
        1,
        "exactly one publication across every candidate owner"
    );
    let after_canonical = admitted_attacker_state_like_cpp(&canonical, attacker);
    assert_eq!(
        after_canonical.elapsed_ms,
        u64::from(admission.diff_ms),
        "the single clock advancement is observable on the canonical owner itself"
    );
    assert_eq!(
        canonical_creature_health_like_cpp(&canonical, victim),
        100 - u64::from(F6_8D2_SWING_DAMAGE_LIKE_CPP),
        "the single swing reached the canonical victim exactly once"
    );
    let refused_owners: Vec<_> = composed
        .refusals
        .iter()
        .map(|(owner, reason)| (*owner, *reason))
        .collect();
    assert!(
        refused_owners.contains(&(
            CreatureExecutionOwnerLikeCpp::LegacyGlobal,
            "canonical-admitted-claim"
        )),
        "the legacy owner is refused by the canonical claim: {refused_owners:?}"
    );
    assert!(
        refused_owners.contains(&(
            CreatureExecutionOwnerLikeCpp::Session,
            "canonical-admitted-claim"
        )),
        "the session owner is refused by the canonical claim: {refused_owners:?}"
    );

    // A repeated composition of the SAME admitted transition is closed for
    // every owner: the canonical claim is already held, so neither a second
    // canonical execution nor a legacy fallback may consume the diff again.
    let repeated = run_isolated_admitted_creature_execution_composition_like_cpp(
        &admission,
        true,
        &canonical,
        &manager,
        &lease,
        RuntimeTickOwner::GlobalLegacy,
        &LegacyCreatureAggroConfigLikeCpp::default(),
        Some(&mut session_arm),
    );
    assert_eq!(
        repeated.admitted_owner, None,
        "no owner executes an admitted transition twice"
    );
    assert!(repeated.legacy.is_none());
    assert!(repeated.session.is_none());
    assert_eq!(
        repeated
            .canonical
            .as_ref()
            .and_then(|canonical| canonical.admission.refusal_label_like_cpp()),
        Some("owner-lease"),
        "the held claim is what closes the repeated attempt"
    );
    assert_eq!(
        session_arm_invocations.load(AtomicOrdering::Relaxed),
        0,
        "a closed transition does not touch the session owner either"
    );
    assert_eq!(
        admitted_attacker_state_like_cpp(&canonical, attacker),
        after_canonical,
        "a closed repeated attempt advances no clock, cast, swing or health"
    );
    assert_eq!(repeated.publication_events_like_cpp(), 0);

    // The production default does not arm the canonical path: the legacy global
    // owner executes instead, and the canonical admitted engine stays out of it.
    // This is what makes the exclusivity above a decision rather than a shape.
    let default_owner = run_isolated_admitted_creature_execution_composition_like_cpp(
        &admission,
        false,
        &canonical,
        &manager,
        &lease,
        RuntimeTickOwner::GlobalLegacy,
        &LegacyCreatureAggroConfigLikeCpp::default(),
        None,
    );
    assert_eq!(
        default_owner.admitted_owner,
        Some(CreatureExecutionOwnerLikeCpp::LegacyGlobal),
        "the production default still admits the legacy global owner"
    );
    assert!(
        default_owner.canonical.is_none(),
        "an unarmed admitted transition never reaches the canonical engine"
    );
    assert!(
        default_owner.refusals.contains(&(
            CreatureExecutionOwnerLikeCpp::CanonicalAdmitted,
            "transition-not-armed"
        )),
        "the canonical owner is refused as unarmed: {:?}",
        default_owner.refusals
    );
}
