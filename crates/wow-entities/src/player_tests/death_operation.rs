//! Native death operation ownership and its single-claim phase fence.

use super::*;
use crate::{PlayerResurrectionOperation, PlayerResurrectionPhase};
use std::num::NonZeroU64;
use wow_core::ObjectGuid;

fn live_player_with_health(health: u64) -> Player {
    let mut player = Player::new(Some(1), false);
    player.unit.set_max_health(health);
    player.unit.set_health(health);
    player
}

fn context(guid_counter: i64, durability_loss: bool) -> PlayerDeathOperationContext {
    PlayerDeathOperationContext::new(ObjectGuid::create_player(1, guid_counter), durability_loss)
}

#[test]
fn begin_preserves_full_width_health_and_existing_resurrection_state() {
    let health = u64::from(u32::MAX) + 57;
    let mut player = live_player_with_health(health);
    assert!(player.begin_resurrection_operation(
        PlayerResurrectionOperation::SelfRes {
            health: 25,
            mana: 10,
        },
        PlayerResurrectionPhase::ResurrectPlayerInFlight,
    ));
    player.add_self_res_spell(21169);
    let continuation = player.resurrection_state_like_cpp().delayed_after_teleport;
    let self_res_spells = player.self_res_spells().to_vec();
    let operation_context = context(44, false);

    let id = player
        .begin_native_death_operation(operation_context)
        .expect("living Player should admit native death operation");

    let snapshot = player
        .native_death_operation_snapshot()
        .expect("admitted operation should be visible");
    assert_eq!(snapshot.id(), id);
    assert_eq!(snapshot.context(), operation_context);
    assert_eq!(snapshot.phase(), PlayerDeathOperationPhase::Ready);
    assert_eq!(player.unit.data().health, health);
    assert!(player.unit.is_alive());
    assert_eq!(
        player.resurrection_state_like_cpp().delayed_after_teleport,
        continuation
    );
    assert_eq!(player.self_res_spells(), self_res_spells);
    assert!(player.has_unresolved_native_death_operation());
}

#[test]
fn begin_refuses_alive_zero_health_and_already_dead_players() {
    let mut alive_zero_health = Player::new(Some(1), false);
    assert!(alive_zero_health.unit.is_alive());
    assert_eq!(alive_zero_health.unit.data().health, 0);
    assert_eq!(
        alive_zero_health.begin_native_death_operation(context(1, true)),
        None
    );

    let mut dead = live_player_with_health(80);
    dead.unit.set_death_state(wow_constants::DeathState::Dead);
    assert!(dead.unit.is_dead());
    assert_eq!(dead.unit.data().health, 80);
    assert_eq!(dead.begin_native_death_operation(context(2, true)), None);
    assert_eq!(dead.native_death_operation_snapshot(), None);
}

#[test]
fn duplicate_begin_preserves_the_first_id_and_context() {
    let mut player = live_player_with_health(90);
    let first_context = context(10, false);
    let first_id = player
        .begin_native_death_operation(first_context)
        .expect("first operation should be admitted");

    assert_eq!(player.begin_native_death_operation(context(11, true)), None);
    assert_eq!(
        player.native_death_operation_snapshot(),
        Some(PlayerDeathOperationSnapshot {
            id: first_id,
            context: first_context,
            phase: PlayerDeathOperationPhase::Ready,
        })
    );
    assert_eq!(player.death_operation.next_id, NonZeroU64::new(2));
}

#[test]
fn claim_refuses_changed_native_health_without_discarding_ready_work() {
    let mut player = live_player_with_health(90);
    let context = context(12, true);
    let id = player
        .begin_native_death_operation(context)
        .expect("operation should be admitted while alive");
    player.unit.set_health(0);

    assert_eq!(player.claim_native_death_operation(id), None);
    assert_eq!(
        player.native_death_operation_snapshot(),
        Some(PlayerDeathOperationSnapshot {
            id,
            context,
            phase: PlayerDeathOperationPhase::Ready,
        })
    );
    assert!(player.has_unresolved_native_death_operation());
}

#[test]
fn wrong_ids_cannot_claim_complete_or_retire_the_current_operation() {
    let mut player = live_player_with_health(90);
    let context = context(13, false);
    let id = player
        .begin_native_death_operation(context)
        .expect("operation should be admitted");
    let wrong_id = PlayerDeathOperationId(NonZeroU64::new(999).unwrap());

    assert_eq!(player.claim_native_death_operation(wrong_id), None);
    assert!(!player.complete_native_death_operation(wrong_id));
    assert!(!player.retire_completed_native_death_operation(wrong_id));
    assert_eq!(player.native_death_operation_snapshot().unwrap().id(), id);
    assert_eq!(
        player.native_death_operation_snapshot().unwrap().phase(),
        PlayerDeathOperationPhase::Ready
    );

    assert_eq!(player.claim_native_death_operation(id), Some(context));
    assert!(!player.complete_native_death_operation(wrong_id));
    assert!(!player.retire_completed_native_death_operation(wrong_id));
    assert_eq!(
        player.native_death_operation_snapshot().unwrap().phase(),
        PlayerDeathOperationPhase::InFlight
    );

    assert!(player.complete_native_death_operation(id));
    assert!(!player.retire_completed_native_death_operation(wrong_id));
    assert_eq!(
        player.native_death_operation_snapshot().unwrap().phase(),
        PlayerDeathOperationPhase::Complete
    );
    assert!(player.retire_completed_native_death_operation(id));
    assert_eq!(player.native_death_operation_snapshot(), None);
}

#[test]
fn in_flight_state_stays_unresolved_and_rejects_replay() {
    let mut player = live_player_with_health(90);
    let context = context(14, true);
    let id = player
        .begin_native_death_operation(context)
        .expect("operation should be admitted");

    assert_eq!(player.claim_native_death_operation(id), Some(context));
    assert_eq!(player.claim_native_death_operation(id), None);
    assert!(player.has_unresolved_native_death_operation());

    // This verifies retained state only; it does not exercise async future cancellation.
    assert_eq!(
        player.native_death_operation_snapshot().unwrap().phase(),
        PlayerDeathOperationPhase::InFlight
    );
    assert!(player.has_unresolved_native_death_operation());
    assert_eq!(player.claim_native_death_operation(id), None);
}

#[test]
fn completion_requires_claim_and_retirement_requires_completion() {
    let mut player = live_player_with_health(90);
    let id = player
        .begin_native_death_operation(context(15, false))
        .expect("operation should be admitted");

    assert!(!player.complete_native_death_operation(id));
    assert!(!player.retire_completed_native_death_operation(id));
    assert_eq!(
        player.native_death_operation_snapshot().unwrap().phase(),
        PlayerDeathOperationPhase::Ready
    );

    assert_eq!(
        player.claim_native_death_operation(id),
        Some(context(15, false))
    );
    assert!(player.complete_native_death_operation(id));
    assert!(!player.has_unresolved_native_death_operation());
    assert!(player.retire_completed_native_death_operation(id));
    assert!(!player.retire_completed_native_death_operation(id));
    assert_eq!(player.native_death_operation_snapshot(), None);
}

#[test]
fn next_operation_gets_a_new_id_after_completed_operation_is_retired() {
    let mut player = live_player_with_health(90);
    let first_id = player
        .begin_native_death_operation(context(16, false))
        .expect("first operation should be admitted");
    assert_eq!(
        player.claim_native_death_operation(first_id),
        Some(context(16, false))
    );
    assert!(player.complete_native_death_operation(first_id));
    assert!(player.retire_completed_native_death_operation(first_id));

    let second_id = player
        .begin_native_death_operation(context(17, true))
        .expect("retired operation should allow a later operation");
    assert_ne!(first_id, second_id);
    assert_eq!(
        player.native_death_operation_snapshot().unwrap().id(),
        second_id
    );
}

#[test]
fn ids_are_local_to_each_player_counter() {
    let mut first_player = live_player_with_health(90);
    let mut second_player = live_player_with_health(90);

    let first_id = first_player
        .begin_native_death_operation(context(18, false))
        .expect("first Player should admit an operation");
    let second_id = second_player
        .begin_native_death_operation(context(19, true))
        .expect("second Player should admit an operation");

    // Each Player starts a local counter; future transport must bind IDs to their owner.
    assert_eq!(first_id, second_id);
}

#[test]
fn operation_id_counter_exhaustion_never_wraps_or_reuses_an_id() {
    let mut player = live_player_with_health(90);
    player.death_operation.next_id = NonZeroU64::new(u64::MAX);
    let last_id = player
        .begin_native_death_operation(context(20, false))
        .expect("u64::MAX is a valid final operation id");

    assert_eq!(
        player.claim_native_death_operation(last_id),
        Some(context(20, false))
    );
    assert!(player.complete_native_death_operation(last_id));
    assert!(player.retire_completed_native_death_operation(last_id));
    assert_eq!(player.death_operation.next_id, None);
    assert_eq!(player.begin_native_death_operation(context(21, true)), None);
    assert_eq!(player.native_death_operation_snapshot(), None);
}
