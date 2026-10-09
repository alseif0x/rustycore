//! Scenarios for [`super`], part 15.
//!
//! #1263 F6-8D2: the isolated admitted creature execution wiring.
//!
//! These regressions drive the **real** world-server adapter, not a copy of it:
//! the session barrier read, the admission capture, the persistence-order gate
//! held across the mutation window, and the deferred publication handed to the
//! victim session only after every guard is released. Production does not call
//! the adapter — the production owner is still `RuntimeTickOwner::GlobalLegacy`
//! and the legacy scheduler, the stores and the canonical
//! `MapCreatureUpdateOwnerLikeCpp::ExternalRuntime` are unchanged — so the
//! isolation is asserted here rather than exercised by a loop.

use super::*;
use crate::{
    IsolatedCreatureExecutionRefusalLikeCpp, deliver_isolated_admitted_creature_execution_like_cpp,
    run_isolated_admitted_creature_execution_for_tick_like_cpp,
};
use wow_world::session::{
    AdmittedCreatureExecutionLikeCpp, CreatureExecutionAdmissionLikeCpp,
    CreatureExecutionLeaseLikeCpp, SharedCreatureExecutionLeaseLikeCpp,
};

/// Coordinator identity the isolated adapter claims these transitions under.
const F6_8D2_COORDINATOR_LIKE_CPP: u64 = 0x0F6D_2000;
/// The one admitted swing of the fixture attacker.
const F6_8D2_SWING_DAMAGE_LIKE_CPP: u32 = 10;
/// RNG seed of the admitted attacker, so the committed roll is exact.
const F6_8D2_ATTACKER_SEED_LIKE_CPP: u64 = 0x0F6D_2D2;

fn f6_8d2_canonical_map_manager_like_cpp() -> wow_world::SharedCanonicalMapManager {
    Arc::new(Mutex::new(wow_map::MapManager::default()))
}

fn f6_8d2_lease_like_cpp() -> SharedCreatureExecutionLeaseLikeCpp {
    Arc::new(Mutex::new(CreatureExecutionLeaseLikeCpp::new()))
}

fn f6_8d2_key_like_cpp() -> wow_map::MapKey {
    wow_map::MapKey::new(0, 0)
}

fn f6_8d2_position_like_cpp(x: f32) -> Position {
    Position::new(x, 10.0, 0.0, 0.0)
}

/// Arm one canonical creature for exactly one admitted transition.
fn arm_admitted_attacker_like_cpp(
    canonical: &wow_world::SharedCanonicalMapManager,
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

/// Admit one tick on the canonical manager and freeze the transition under test.
fn admitted_tick_and_capture_like_cpp(
    canonical: &wow_world::SharedCanonicalMapManager,
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
    let admission = wow_world::session::capture_admitted_creature_execution_like_cpp(
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

/// The observable admitted state of one canonical creature.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct WiringAttackerStateLikeCpp {
    elapsed_ms: u64,
    last_swing_ms: u64,
    swing_timer_ms: u64,
    health: u64,
    due_slot: Option<usize>,
}

fn wiring_attacker_state_like_cpp(
    canonical: &wow_world::SharedCanonicalMapManager,
    guid: ObjectGuid,
) -> WiringAttackerStateLikeCpp {
    let guard = canonical
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    guard
        .find_map(0, 0)
        .expect("the fixture attached the canonical map instance")
        .map()
        .with_creature_like_cpp(guid, |creature| WiringAttackerStateLikeCpp {
            elapsed_ms: creature.runtime_elapsed_ms_like_cpp(),
            last_swing_ms: creature.ai_ownership().last_swing_ms,
            swing_timer_ms: creature.ai_ownership().swing_timer_ms,
            health: creature.unit().data().health,
            due_slot: creature.first_due_creature_spell_slot_like_cpp(),
        })
        .expect("the fixture creature is a canonical creature")
}

#[test]
fn isolated_admitted_execution_delivers_only_after_every_guard_is_released_like_cpp() {
    let registry = PlayerRegistry::with_canonical_player_fixtures_like_cpp();
    let canonical = registry
        .fixture_canonical_map_manager_like_cpp()
        .expect("the registry fixture binds the canonical manager the delivery resolves against");
    let attacker = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 9002, 94_302);
    let victim = ObjectGuid::create_player(1, 94_301);
    let (victim_info, victim_rx) =
        make_registry_player_like_cpp(0, 0, f6_8d2_position_like_cpp(11.0), true);
    registry.register_or_replace(victim, victim_info, Default::default());
    add_canonical_test_creature_on_map_like_cpp(
        &canonical,
        attacker,
        f6_8d2_position_like_cpp(10.0),
        0,
        0,
        100,
    );
    arm_admitted_attacker_like_cpp(&canonical, attacker, victim, F6_8D2_ATTACKER_SEED_LIKE_CPP);

    let (plan, admission) =
        admitted_tick_and_capture_like_cpp(&canonical, &[(f6_8d2_key_like_cpp(), attacker)]);
    assert_eq!(admission.objects.len(), 1, "the tick selected one creature");

    let respawn_db_mutation_order: crate::SharedRespawnDbMutationOrderLikeCpp =
        Arc::new(Mutex::new(()));
    let lease = f6_8d2_lease_like_cpp();
    let mut outcome = run_isolated_admitted_creature_execution_for_tick_like_cpp(
        &plan,
        &canonical,
        &lease,
        F6_8D2_COORDINATOR_LIKE_CPP,
        Some(&respawn_db_mutation_order),
        &[(f6_8d2_key_like_cpp(), attacker)],
    );

    assert!(
        !outcome.refused_like_cpp(),
        "the admitted tick executes: {:?}",
        outcome.refusal
    );
    assert!(
        outcome.mutation_window_guarded,
        "the persistence-order gate is held across the whole mutation window"
    );
    assert_eq!(outcome.tick_epoch, plan.epoch_like_cpp());
    assert_eq!(outcome.diff_ms, plan.effective_diff_ms());
    assert_eq!(outcome.admitted_maps, 1);
    assert_eq!(outcome.captured_maps, 1);
    assert_eq!(outcome.captured_objects, 1);

    let execution = outcome
        .execution
        .as_ref()
        .expect("the isolated execution outcome is retained");
    assert_eq!(
        execution.admission,
        CreatureExecutionAdmissionLikeCpp::Admitted
    );
    assert_eq!(execution.clock_advanced_ms, u64::from(outcome.diff_ms));
    assert_eq!(execution.due_casts, 1);
    assert_eq!(
        execution.swing_damage,
        u64::from(F6_8D2_SWING_DAMAGE_LIKE_CPP)
    );
    assert_eq!(execution.effects_consumed, 1);
    assert_eq!(outcome.deferred_publication_like_cpp().len(), 1);
    assert!(
        !outcome.publication_delivered_like_cpp(),
        "the deferred batch is owed to the delivery entry, not delivered under the guards"
    );
    assert!(
        victim_rx.try_recv().is_err(),
        "nothing was published to the victim while the guards were held"
    );

    // Every guard the adapter took is released before it returned, so the
    // delivery below cannot run under a map or persistence guard.
    assert!(
        respawn_db_mutation_order.try_lock().is_ok(),
        "the persistence-order gate is free after the mutation window"
    );
    assert!(
        canonical.try_lock().is_ok(),
        "the canonical map guard is free after the execution"
    );

    let delivery = deliver_isolated_admitted_creature_execution_like_cpp(&mut outcome, &registry);
    assert_eq!(delivery.commands, 1);
    assert_eq!(delivery.queued, 1);
    assert_eq!(delivery.failed, 0);
    assert!(outcome.publication_delivered_like_cpp());
    let delivered = drain_durable_creature_runtime_commands_like_cpp(&registry, victim);
    assert_eq!(
        delivered.len(),
        1,
        "the one deferred publication reached the victim session's durable rail after release"
    );
    assert!(matches!(
        delivered.first(),
        Some(SessionCommand::ApplyCreatureMeleeDamageLikeCpp(_))
    ));
    assert!(
        victim_rx.try_recv().is_err(),
        "the general session rail is not the creature melee publication path"
    );

    // The batch is delivered exactly once.
    let repeated = deliver_isolated_admitted_creature_execution_like_cpp(&mut outcome, &registry);
    assert_eq!(repeated.commands, 0);
    assert_eq!(repeated.queued, 0);
    assert!(drain_durable_creature_runtime_commands_like_cpp(&registry, victim).is_empty());
}

#[test]
fn isolated_admitted_execution_refuses_a_recreated_incarnation_like_cpp() {
    let canonical = f6_8d2_canonical_map_manager_like_cpp();
    let registry = PlayerRegistry::default();
    let attacker = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 9002, 94_312);
    let victim = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 9003, 94_313);
    for (guid, x) in [(attacker, 10.0_f32), (victim, 11.0)] {
        add_canonical_test_creature_on_map_like_cpp(
            &canonical,
            guid,
            f6_8d2_position_like_cpp(x),
            0,
            0,
            100,
        );
    }
    arm_admitted_attacker_like_cpp(&canonical, attacker, victim, F6_8D2_ATTACKER_SEED_LIKE_CPP);

    let (plan, admission) =
        admitted_tick_and_capture_like_cpp(&canonical, &[(f6_8d2_key_like_cpp(), attacker)]);
    assert_eq!(
        admission.objects.len(),
        1,
        "the admitted tick froze the selected creature"
    );
    let incarnation_n = canonical
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .map_incarnation_like_cpp(f6_8d2_key_like_cpp())
        .expect("the admitted map key has an incarnation");

    // The reviewer's falsifier at the wiring boundary: the same map key and the
    // same GUID are recreated as N+1 while the committed swing is still owed.
    assert!(
        canonical
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .destroy_map(0, 0),
        "a creature-only fixture map is evacuable"
    );
    for (guid, x) in [(attacker, 10.0_f32), (victim, 11.0)] {
        add_canonical_test_creature_on_map_like_cpp(
            &canonical,
            guid,
            f6_8d2_position_like_cpp(x),
            0,
            0,
            100,
        );
    }
    arm_admitted_attacker_like_cpp(&canonical, attacker, victim, F6_8D2_ATTACKER_SEED_LIKE_CPP);
    let incarnation_n_plus_one = canonical
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .map_incarnation_like_cpp(f6_8d2_key_like_cpp())
        .expect("the recreated map key has a new incarnation");
    assert_ne!(incarnation_n, incarnation_n_plus_one);

    let replacement_before = wiring_attacker_state_like_cpp(&canonical, attacker);
    let respawn_db_mutation_order: crate::SharedRespawnDbMutationOrderLikeCpp =
        Arc::new(Mutex::new(()));
    let lease = f6_8d2_lease_like_cpp();
    let mut outcome = run_isolated_admitted_creature_execution_for_tick_like_cpp(
        &plan,
        &canonical,
        &lease,
        F6_8D2_COORDINATOR_LIKE_CPP,
        Some(&respawn_db_mutation_order),
        &[(f6_8d2_key_like_cpp(), attacker)],
    );

    assert_eq!(
        outcome.refusal,
        Some(IsolatedCreatureExecutionRefusalLikeCpp::Admission(
            CreatureExecutionAdmissionLikeCpp::RefusedMapIncarnation {
                map_id: 0,
                instance_id: 0,
                expected: incarnation_n,
                current: Some(incarnation_n_plus_one),
            }
        )),
        "the recreated key is refused by the admission fence"
    );
    assert_eq!(
        outcome.refusal.map(|refusal| refusal.label_like_cpp()),
        Some("map-incarnation")
    );
    let execution = outcome
        .execution
        .as_ref()
        .expect("a refusal still reports its zeroed execution outcome");
    assert_eq!(execution.clock_advanced_ms, 0);
    assert_eq!(execution.due_casts, 0);
    assert_eq!(execution.swing_damage, 0);
    assert_eq!(execution.effects_consumed, 0);
    assert!(outcome.deferred_publication_like_cpp().is_empty());
    assert!(!outcome.publication_delivered_like_cpp());
    assert_eq!(
        wiring_attacker_state_like_cpp(&canonical, attacker),
        replacement_before,
        "the replacement keeps its clock, timers, due cast and health"
    );
    assert!(
        respawn_db_mutation_order.try_lock().is_ok(),
        "the persistence-order gate is released even on refusal"
    );

    // Delivering a refused transition publishes nothing.
    let delivery = deliver_isolated_admitted_creature_execution_like_cpp(&mut outcome, &registry);
    assert_eq!(delivery.commands, 0);
    assert_eq!(delivery.queued, 0);
}

#[test]
fn isolated_admitted_execution_refuses_a_plan_its_coordinator_does_not_await_like_cpp() {
    // The plan belongs to another coordinator: the canonical manager under test
    // never admitted this epoch, so the adapter must refuse before it reads a
    // single map incarnation.
    let other_coordinator = f6_8d2_canonical_map_manager_like_cpp();
    let plan = other_coordinator
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .begin_tick_like_cpp(1)
        .into_started()
        .expect("the fixture map update timer has passed");

    let canonical = f6_8d2_canonical_map_manager_like_cpp();
    let attacker = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 9002, 94_322);
    let victim = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 9003, 94_323);
    for (guid, x) in [(attacker, 10.0_f32), (victim, 11.0)] {
        add_canonical_test_creature_on_map_like_cpp(
            &canonical,
            guid,
            f6_8d2_position_like_cpp(x),
            0,
            0,
            100,
        );
    }
    arm_admitted_attacker_like_cpp(&canonical, attacker, victim, F6_8D2_ATTACKER_SEED_LIKE_CPP);
    let before = wiring_attacker_state_like_cpp(&canonical, attacker);

    let respawn_db_mutation_order: crate::SharedRespawnDbMutationOrderLikeCpp =
        Arc::new(Mutex::new(()));
    let lease = f6_8d2_lease_like_cpp();
    let outcome = run_isolated_admitted_creature_execution_for_tick_like_cpp(
        &plan,
        &canonical,
        &lease,
        F6_8D2_COORDINATOR_LIKE_CPP,
        Some(&respawn_db_mutation_order),
        &[(f6_8d2_key_like_cpp(), attacker)],
    );

    assert_eq!(
        outcome.refusal,
        Some(IsolatedCreatureExecutionRefusalLikeCpp::SessionBarrier {
            expected_epoch: plan.epoch_like_cpp(),
            current: wow_map::MapTickCoordinationStateLikeCpp::Idle,
        }),
        "the adapter refuses before it touches any map"
    );
    assert_eq!(
        outcome.refusal.map(|refusal| refusal.label_like_cpp()),
        Some("session-barrier")
    );
    assert_eq!(
        outcome.captured_maps, 0,
        "the barrier refusal reads no map incarnation at all"
    );
    assert_eq!(outcome.captured_objects, 0);
    assert!(outcome.execution.is_none());
    assert!(outcome.deferred_publication_like_cpp().is_empty());
    assert!(
        !outcome.mutation_window_guarded,
        "the persistence-order gate is not taken for a refused plan"
    );
    assert_eq!(wiring_attacker_state_like_cpp(&canonical, attacker), before);
    assert!(
        lease
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .claim_like_cpp(
                wow_world::session::CreatureExecutionTransitionLikeCpp {
                    coordinator_id: F6_8D2_COORDINATOR_LIKE_CPP,
                    tick_epoch: plan.epoch_like_cpp(),
                    map_id: 0,
                    instance_id: 0,
                    incarnation: 0,
                    creature_guid: attacker,
                },
                wow_world::session::CreatureExecutionOwnerLikeCpp::CanonicalAdmitted,
            ),
        "a refused plan claims no transition"
    );
}
