//! Publication owns the accepted effect before its RNG tombstone/next action.
use super::*;

fn publication(
    counter: i64,
) -> (
    MapManager,
    MapObjectTickContinuation,
    ObjectMapUpdateToken,
    ObjectGuid,
    ActorSpellPublicationContinuation,
) {
    let (mut manager, guid, _) = setup(counter);
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
    let ActorSpellProgress::Publication {
        completion,
        continuation,
    } = policy(settings, &Trace::default(), |policies| {
        manager.prepare_spell(&tick, &mut token, false, policies)
    })
    .unwrap()
    else {
        panic!("owned publication")
    };
    assert_eq!(completion.caster_guid, guid);
    (manager, tick, token, guid, continuation)
}

#[test]
fn wire_barrier_precedes_hit_tombstone_and_following_schedule() {
    let (mut manager, tick, mut token, guid, continuation) = publication(825_001);
    assert!(actor(&manager, guid).runtime_rng_authority_complete_like_cpp());
    assert_eq!(
        actor(&manager, guid).creature_spell_due_in_ms_for_test(1),
        None
    );
    assert_eq!(continuation.partial().spell_hits, 1);
    assert_eq!(continuation.partial().runtime_rng_authority_rejections, 0);
    assert!(token.actor_operation.is_some());
    let before = prefix(&manager, guid);
    let trace = Trace::default();
    let outcome = complete!(
        &mut manager,
        &tick,
        &mut token,
        Settings::default(),
        &trace,
        policy(Settings::default(), &trace, |policies| manager
            .resume_spell_publication(
                &tick,
                &mut token,
                continuation,
                policies
            ))
        .unwrap()
    );
    assert!(!actor(&manager, guid).runtime_rng_authority_complete_like_cpp());
    assert_eq!(outcome.runtime_rng_authority_rejections, 1);
    assert_eq!(
        actor(&manager, guid).creature_spell_due_in_ms_for_test(1),
        None
    );
    assert!(trace.borrow().is_empty());
    assert_eq!(prefix(&manager, guid), before);
    assert!(token.actor_operation.is_none());
}

#[test]
fn publication_actor_aba_rejects_without_reset_or_tombstone_then_disposes_only_slot() {
    let (mut manager, tick, mut token, guid, continuation) = publication(825_010);
    let owned = manager
        .find_map_mut(1, 0)
        .unwrap()
        .map_mut()
        .remove_map_object(guid)
        .unwrap();
    fixtures::insert_actor(&mut manager, 825_010, Position::xyz(10.0, 20.0, 30.0), true);
    let before = prefix(&manager, guid);
    let trace = Trace::default();
    let failure = policy(Settings::default(), &trace, |policies| {
        manager.resume_spell_publication(&tick, &mut token, continuation, policies)
    })
    .err()
    .unwrap();
    assert_eq!(failure.continuation.partial().spell_hits, 1);
    assert!(trace.borrow().is_empty());
    assert!(token.actor_operation.is_some());
    assert!(actor(&manager, guid).runtime_rng_authority_complete_like_cpp());
    let partial = manager
        .discard_spell_publication(&tick, &mut token, failure.continuation)
        .unwrap();
    assert_eq!(partial.spell_hits, 1);
    assert_eq!(partial.runtime_rng_authority_rejections, 0);
    assert_eq!(prefix(&manager, guid), before);
    assert!(token.actor_operation.is_none());
    assert!(manager.begin_tick_like_cpp(1).is_busy());
    drop(owned);
}

#[test]
fn foreign_publication_resume_and_disposition_preserve_both_slots_and_owned_state() {
    let (mut manager, tick, mut token, _, continuation) = publication(825_020);
    let (mut other, other_tick, mut other_token, _, other_continuation) = publication(825_020);
    let trace = Trace::default();
    let failure = policy(Settings::default(), &trace, |policies| {
        other.resume_spell_publication(&other_tick, &mut other_token, continuation, policies)
    })
    .err()
    .unwrap();
    assert_eq!(failure.continuation.partial().spell_hits, 1);
    let failure = other
        .discard_spell_publication(&other_tick, &mut other_token, failure.continuation)
        .err()
        .unwrap();
    assert!(trace.borrow().is_empty());
    assert!(token.actor_operation.is_some());
    assert!(other_token.actor_operation.is_some());
    manager
        .discard_spell_publication(&tick, &mut token, failure.continuation)
        .unwrap();
    assert!(other_token.actor_operation.is_some());
    drop(other_continuation);
}

#[test]
fn dropping_publication_keeps_busy_and_never_draws_a_repeat() {
    let (mut manager, tick, mut token, guid, continuation) = publication(825_030);
    drop(continuation);
    let trace = Trace::default();
    assert!(
        policy(Settings::default(), &trace, |policies| manager
            .prepare_spell(&tick, &mut token, false, policies))
        .is_err()
    );
    assert!(trace.borrow().is_empty());
    assert!(token.actor_operation.is_some());
    assert!(actor(&manager, guid).runtime_rng_authority_complete_like_cpp());
    assert_eq!(
        actor(&manager, guid).creature_spell_due_in_ms_for_test(1),
        None
    );
    assert!(manager.begin_tick_like_cpp(1).is_busy());
}

#[test]
fn stale_map_publication_disposition_does_not_mutate_replacement() {
    let (mut manager, tick, mut token, guid, continuation) = publication(825_040);
    let old = manager.maps.remove(&crate::MapKey::new(1, 0)).unwrap();
    manager.create_world_map(1, 0);
    fixtures::insert_actor(&mut manager, 825_040, Position::xyz(10.0, 20.0, 30.0), true);
    let before = prefix(&manager, guid);
    let partial = manager
        .discard_spell_publication(&tick, &mut token, continuation)
        .unwrap();
    assert_eq!(partial.spell_hits, 1);
    assert_eq!(prefix(&manager, guid), before);
    assert!(actor(&manager, guid).runtime_rng_authority_complete_like_cpp());
    assert!(token.actor_operation.is_none());
    drop(old);
}
