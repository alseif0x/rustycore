//! Real producer executions; preparation preserves the original slot and buffers.
use super::*;
use crate::manager::MapObjectUpdateSelectionLikeCpp;
use crate::manager::actor_melee::kill_origin::MeleeKillPhase;
use crate::manager::actor_tick_access::fixtures::*;
use wow_core::Position;

mod conflicts;
mod fixtures;
use fixtures::*;

fn execution(
    manager: &mut MapManager,
    root: ObjectGuid,
) -> (MapObjectTickContinuation, PendingMeleeKills) {
    let (tick, token) = start(
        manager,
        200,
        MapObjectUpdateSelectionLikeCpp::WholeTypedStores,
    );
    let pending = manager
        .apply_selected_creature_melee_with_kills(&tick, token, root, &Catalogs::default())
        .ok()
        .unwrap()
        .into_pending();
    (tick, pending)
}
fn reject(
    manager: &MapManager,
    tick: &MapObjectTickContinuation,
    pending: PendingMeleeKills,
) -> (MeleeKillPhaseError, PendingMeleeKills) {
    match manager.prepare_next_melee_kill(tick, pending) {
        Ok(_) => panic!("expected preparation rejection"),
        Err(error) => error,
    }
}
fn retained(pending: &PendingMeleeKills, root: ObjectGuid) {
    assert_eq!(pending.cursor(), 0);
    assert_eq!(pending.outcome().canonical_creature_hits, 1);
    assert_eq!(pending.outcome().events.len(), 2);
    assert_eq!(pending.outcome().syncs.len(), 1);
    assert_eq!(pending.batch().root_guid(), root);
    let operation = pending.token().actor_operation.as_ref().unwrap();
    assert_eq!(operation.guid, root);
    assert!(operation.matches_token(pending.token()));
}

#[test]
fn first_occurrence_keeps_full_token_outcome_and_buffers_without_completion() {
    let (mut manager, root, victim) = setup(2800, 5, false, 10);
    let (tick, pending) = execution(&mut manager, root);
    let identity = (
        pending.token.key(),
        pending.token.incarnation(),
        pending.token.effective_diff_ms(),
    );
    let buffers = (
        pending.outcome.events.as_ptr(),
        pending.outcome.syncs.as_ptr(),
        pending.batch.occurrences.as_ptr(),
    );
    let prepared = manager
        .prepare_next_melee_kill(&tick, pending)
        .ok()
        .unwrap();
    assert_eq!(prepared.occurrence().phase(), MeleeKillPhase::Primary);
    assert_eq!(prepared.captured().target_guid(), victim);
    assert!(!prepared.before_hook_source().is_alive());
    retained(prepared.pending(), root);
    let pending = prepared.into_pending();
    assert_eq!(
        (
            pending.token.key(),
            pending.token.incarnation(),
            pending.token.effective_diff_ms()
        ),
        identity
    );
    assert_eq!(
        (
            pending.outcome.events.as_ptr(),
            pending.outcome.syncs.as_ptr(),
            pending.batch.occurrences.as_ptr()
        ),
        buffers
    );
    assert_eq!(pending.batch.occurrences.len(), 1);
    let prepared = manager
        .prepare_next_melee_kill(&tick, pending)
        .ok()
        .unwrap();
    assert_eq!(prepared.pending().cursor(), 0);
    assert_eq!(prepared.captured().target_guid(), victim);
    retained(&prepared.into_pending(), root);
}

#[test]
fn late_damage_target_outside_nearby_selection_remains_valid() {
    let (mut manager, root) = manager_with_actor(2810);
    let (tick, mut token) = start(
        &mut manager,
        200,
        MapObjectUpdateSelectionLikeCpp::NearbyCells,
    );
    assert_eq!(
        manager.selected_actor_guids(&tick, &mut token).unwrap(),
        vec![root]
    );
    let victim = target(&mut manager, 2811, 5, false);
    arm(&mut manager, root, victim, 10);
    let pending = manager
        .apply_selected_creature_melee_with_kills(&tick, token, root, &Catalogs::default())
        .ok()
        .unwrap()
        .into_pending();
    let prepared = manager
        .prepare_next_melee_kill(&tick, pending)
        .ok()
        .unwrap();
    assert_eq!(prepared.captured().target_guid(), victim);
    let mut pending = prepared.into_pending();
    assert_eq!(
        manager
            .selected_actor_guids(&tick, &mut pending.token)
            .unwrap(),
        vec![root]
    );
    retained(&pending, root);
}

#[test]
fn dead_root_is_valid_without_a_late_alive_gate() {
    let (mut manager, root, _) = setup(2820, 75, false, 100);
    arm(&mut manager, root, root, 100);
    let (tick, pending) = execution(&mut manager, root);
    assert_eq!(health(&manager, root), 0);
    let prepared = manager
        .prepare_next_melee_kill(&tick, pending)
        .ok()
        .unwrap();
    assert_eq!(prepared.captured().target_guid(), root);
    retained(&prepared.into_pending(), root);
}

#[test]
fn record_unavailable_retains_actual_damage_and_complete_partial_outcome() {
    let (mut manager, root, victim) = setup(2830, 5, true, 10);
    let (tick, pending) = execution(&mut manager, root);
    let (error, pending) = reject(&manager, &tick, pending);
    assert_eq!(
        error,
        MeleeKillPhaseError::Unavailable(MeleeKillCaptureError::NoActor)
    );
    assert_eq!(health(&manager, victim), 0);
    assert!(matches!(
        pending.batch.occurrences[0].capture,
        MeleeKillCapture::Unavailable(MeleeKillCaptureError::NoActor)
    ));
    retained(&pending, root);
}

#[test]
fn nonlethal_execution_is_not_reserved_and_keeps_its_original_outcome() {
    let (mut manager, root, victim) = setup(2840, 75, false, 10);
    let (tick, pending) = execution(&mut manager, root);
    let (error, pending) = reject(&manager, &tick, pending);
    assert_eq!(error, MeleeKillPhaseError::NotReserved);
    assert_eq!(pending.cursor(), 0);
    assert!(pending.token.actor_operation.is_none());
    assert!(pending.batch.occurrences.is_empty());
    assert_eq!(pending.outcome.canonical_creature_hits, 1);
    assert_eq!(health(&manager, victim), 65);
}

#[test]
fn reserved_empty_ledger_is_explicit_and_does_not_clear_the_original_slot() {
    let (mut manager, root, _) = setup(2850, 5, false, 10);
    let (tick, mut pending) = execution(&mut manager, root);
    // Invalid ledger input exercises the boundary, not a synthetic hook receipt.
    pending.batch.occurrences.clear();
    let (error, pending) = reject(&manager, &tick, pending);
    assert_eq!(error, MeleeKillPhaseError::NoOccurrence);
    assert!(pending.batch.occurrences.is_empty());
    retained(&pending, root);
}
