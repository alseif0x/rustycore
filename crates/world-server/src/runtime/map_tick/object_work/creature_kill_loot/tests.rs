//! Dormant generation-origin identity, ownership, await and disposal contracts.
use super::*;
use std::future::Future;
use wow_map::{ObjectMapFinishOutcome, ObjectMapTickError};
mod fixtures;
use fixtures::*;

#[test]
fn recoverable_finish_returns_foreign_token_for_exact_origin_retry() {
    let (mut first, mut first_work, first_token, _, _) = setup(94001);
    let (mut second, mut second_work, second_token, _, _) = setup(94002);
    let original = (
        first_token.key(),
        first_token.incarnation(),
        first_token.effective_diff_ms(),
    );
    let returned = match second_work.try_finish_map(&mut second, first_token, None, &mut no_record)
    {
        Err((ObjectMapTickError::OriginMismatch { .. }, token)) => token,
        _ => panic!("foreign token must be returned"),
    };
    assert_eq!(
        (
            returned.key(),
            returned.incarnation(),
            returned.effective_diff_ms()
        ),
        original
    );
    assert!(second_work.prepare_next(&mut second).is_none());
    assert!(first.begin_tick_like_cpp(1).is_busy());
    assert_eq!(
        finish(&mut first_work, &mut first, returned),
        ObjectMapFinishOutcome::Completed
    );
    assert_eq!(
        finish(&mut second_work, &mut second, second_token),
        ObjectMapFinishOutcome::Completed
    );
}

#[test]
fn begin_rejection_returns_original_token_without_reading_foreign_actor_facts() {
    let (mut first, mut first_work, token, guid, _) = setup(94003);
    let (mut second, second_work, _second_token, _, _) = setup(94004);
    let original = (token.key(), token.incarnation(), token.effective_diff_ms());
    let returned = match second_work.begin_reserved_creature_loot(&mut second, token, guid) {
        Err((
            CreatureLootAccessError::Tick(ActorTickAccessError::Tick(
                ObjectMapTickError::OriginMismatch { .. },
            )),
            token,
        )) => token,
        _ => panic!("foreign begin must return its real token"),
    };
    assert_eq!(
        (
            returned.key(),
            returned.incarnation(),
            returned.effective_diff_ms()
        ),
        original
    );
    assert_eq!(
        finish(&mut first_work, &mut first, returned),
        ObjectMapFinishOutcome::Completed
    );
}

#[tokio::test]
async fn controlled_generation_await_keeps_identity_and_original_pending_slot() {
    let (mut manager, mut work, token, guid, authority) = setup(94005);
    let before = health_identity(&manager, guid);
    let mut operation = begin(&work, &mut manager, token, guid);
    assert!(!operation.source().is_alive());
    assert_eq!(operation.source().entry(), 42);
    let lifetime = operation.source().loot_lifecycle_revision();
    let (observed_lifetime, generated) = {
        let (complete_tx, complete_rx) = tokio::sync::oneshot::channel();
        // Owned source is available across the controlled await, with no manager
        // reference or mutex guard captured by this generation future.
        let generation = async {
            let () = complete_rx.await.unwrap();
            (operation.source().loot_lifecycle_revision(), pool(guid))
        };
        tokio::pin!(generation);
        let pending = std::future::poll_fn(|cx| match generation.as_mut().poll(cx) {
            std::task::Poll::Pending => std::task::Poll::Ready(true),
            std::task::Poll::Ready(_) => panic!("controlled generation completed too early"),
        })
        .await;
        assert!(pending);
        assert!(manager.begin_tick_like_cpp(1).is_busy());
        complete_tx.send(()).unwrap();
        generation.await
    };
    assert_eq!(observed_lifetime, lifetime);
    assert_pending(&work, &mut manager, &mut operation, guid);
    assert!(matches!(
        work.install_reserved_creature_loot(
            &mut manager,
            &mut operation,
            &authority,
            0,
            Some(generated),
            HashMap::new()
        ),
        CreatureLootAccess::Ready(true)
    ));
    assert_eq!(health_identity(&manager, guid), before);
    assert_eq!(
        authority
            .snapshot_for_player_like_cpp(ObjectGuid::EMPTY)
            .unwrap()
            .loot
            .coins,
        7
    );
    let token = match work.settle_reserved_creature_loot(&manager, operation) {
        Ok(token) => token,
        Err((error, _operation)) => panic!("settled request must complete: {error:?}"),
    };
    assert_eq!(
        finish(&mut work, &mut manager, token),
        ObjectMapFinishOutcome::Completed
    );
}

#[tokio::test]
async fn cancelling_generation_future_requires_explicit_disposal_before_finish() {
    let (mut manager, mut work, token, guid, authority) = setup(94006);
    let mut operation = begin(&work, &mut manager, token, guid);
    {
        let (_sender, receiver) = tokio::sync::oneshot::channel::<()>();
        let generation = async {
            receiver.await.unwrap();
            operation.source().entry()
        };
        tokio::pin!(generation);
        std::future::poll_fn(|cx| match generation.as_mut().poll(cx) {
            std::task::Poll::Pending => std::task::Poll::Ready(()),
            std::task::Poll::Ready(_) => panic!("generation must remain pending"),
        })
        .await;
        // End scope drops only this future. It cannot clear the token slot.
    }
    assert_pending(&work, &mut manager, &mut operation, guid);
    assert!(authority.is_retired_like_cpp());
    let token = dispose(&work, &manager, operation);
    assert_eq!(
        finish(&mut work, &mut manager, token),
        ObjectMapFinishOutcome::Completed
    );
}

#[test]
fn dropping_entire_operation_keeps_map_inflight_and_prevents_new_tick() {
    let (mut manager, mut work, token, guid, authority) = setup(94007);
    let operation = begin(&work, &mut manager, token, guid);
    drop(operation);
    assert!(work.prepare_next(&mut manager).is_none());
    assert!(manager.begin_tick_like_cpp(1).is_busy());
    assert!(authority.is_retired_like_cpp());
    assert!(
        manager
            .find_map(1, 0)
            .unwrap()
            .delayed_update_calls()
            .is_empty()
    );
}

#[test]
fn failed_install_preserves_facts_and_slot_until_explicit_result_disposal() {
    let (mut manager, mut work, token, guid, authority) = setup(94015);
    let mut operation = begin(&work, &mut manager, token, guid);
    let lifetime = operation.source().loot_lifecycle_revision();
    let before = health_identity(&manager, guid);
    assert!(matches!(
        work.install_reserved_creature_loot(
            &mut manager,
            &mut operation,
            &authority,
            1,
            Some(pool(guid)),
            HashMap::new()
        ),
        CreatureLootAccess::Ready(false)
    ));
    assert_eq!(operation.source().loot_lifecycle_revision(), lifetime);
    assert_eq!(health_identity(&manager, guid), before);
    assert!(authority.is_retired_like_cpp());
    assert_pending(&work, &mut manager, &mut operation, guid);
    let token = dispose(&work, &manager, operation);
    assert_eq!(
        finish(&mut work, &mut manager, token),
        ObjectMapFinishOutcome::Completed
    );
}

#[test]
fn foreign_work_install_and_completion_retain_original_operation_for_disposal() {
    let (mut manager, mut work, token, guid, authority) = setup(94008);
    let (mut foreign, foreign_work, _foreign_token, _, _) = setup(94009);
    let mut operation = begin(&work, &mut manager, token, guid);
    let before = health_identity(&manager, guid);
    assert!(matches!(
        foreign_work.install_reserved_creature_loot(
            &mut foreign,
            &mut operation,
            &authority,
            0,
            Some(pool(guid)),
            HashMap::new()
        ),
        CreatureLootAccess::Rejected(CreatureLootAccessError::WrongOrigin)
    ));
    operation = match foreign_work.dispose_reserved_creature_loot(&foreign, operation) {
        Err((CreatureLootAccessError::WrongOrigin, operation)) => operation,
        _ => panic!("foreign disposal must retain operation"),
    };
    assert_eq!(operation.source().entry(), 42);
    assert_eq!(health_identity(&manager, guid), before);
    assert_pending(&work, &mut manager, &mut operation, guid);
    let token = dispose(&work, &manager, operation);
    assert_eq!(
        finish(&mut work, &mut manager, token),
        ObjectMapFinishOutcome::Completed
    );
}

#[test]
fn same_manager_wrong_tick_keeps_original_slot_until_correct_work_disposes() {
    let (mut manager, mut work, token, guid, authority) = setup(94010);
    let (mut foreign, foreign_work, _foreign_token, _, _) = setup(94011);
    let mut operation = begin(&work, &mut manager, token, guid);
    assert!(matches!(
        foreign_work.install_reserved_creature_loot(
            &mut manager,
            &mut operation,
            &authority,
            0,
            None,
            HashMap::new()
        ),
        CreatureLootAccess::Rejected(_)
    ));
    operation = match foreign_work.settle_reserved_creature_loot(&manager, operation) {
        Err((_error, operation)) => operation,
        _ => panic!("foreign tick must not clear the real operation"),
    };
    assert_pending(&work, &mut manager, &mut operation, guid);
    assert!(authority.is_retired_like_cpp());
    assert!(foreign.begin_tick_like_cpp(1).is_busy());
    let token = dispose(&work, &manager, operation);
    assert_eq!(
        finish(&mut work, &mut manager, token),
        ObjectMapFinishOutcome::Completed
    );
}

#[test]
fn stale_map_rejects_install_but_disposal_returns_token_for_stale_finish() {
    let (mut manager, mut work, token, guid, authority) = setup(94012);
    let mut operation = begin(&work, &mut manager, token, guid);
    let old_incarnation = operation.token.incarnation();
    assert!(manager.destroy_map(1, 0));
    manager.create_world_map(1, 0);
    admit(&mut manager, 94012);
    let replacement = health_identity(&manager, guid);
    assert!(matches!(
        work.install_reserved_creature_loot(
            &mut manager,
            &mut operation,
            &authority,
            0,
            Some(pool(guid)),
            HashMap::new()
        ),
        CreatureLootAccess::Rejected(_)
    ));
    assert_eq!(health_identity(&manager, guid), replacement);
    assert!(authority.is_retired_like_cpp());
    let token = dispose(&work, &manager, operation);
    assert_eq!(token.incarnation(), old_incarnation);
    assert!(matches!(
        finish(&mut work, &mut manager, token),
        ObjectMapFinishOutcome::StaleParticipant { .. }
    ));
    assert!(
        !manager
            .find_map(1, 0)
            .unwrap()
            .last_map_update_tail_summary_like_cpp()
            .script_hook
            .invoked
    );
}

#[test]
fn same_guid_actor_readmission_rejects_install_without_touching_replacement() {
    let (mut manager, mut work, token, guid, authority) = setup(94013);
    let mut operation = begin(&work, &mut manager, token, guid);
    let removed = manager
        .find_map_mut(1, 0)
        .unwrap()
        .map_mut()
        .remove_map_object(guid)
        .unwrap();
    admit(&mut manager, 94013);
    let replacement = health_identity(&manager, guid);
    assert!(matches!(
        work.install_reserved_creature_loot(
            &mut manager,
            &mut operation,
            &authority,
            0,
            Some(pool(guid)),
            HashMap::new()
        ),
        CreatureLootAccess::Rejected(CreatureLootAccessError::Tick(
            ActorTickAccessError::WitnessMismatch { .. }
        ))
    ));
    assert_eq!(health_identity(&manager, guid), replacement);
    assert!(authority.is_retired_like_cpp());
    assert_pending(&work, &mut manager, &mut operation, guid);
    let token = dispose(&work, &manager, operation);
    assert_eq!(
        finish(&mut work, &mut manager, token),
        ObjectMapFinishOutcome::Completed
    );
    drop(removed);
}

#[test]
fn pending_finish_returns_same_token_and_cannot_clear_reserved_slot() {
    let (mut manager, mut work, token, guid, _) = setup(94014);
    let operation = begin(&work, &mut manager, token, guid);
    let ReservedCreatureLootGeneration {
        token,
        handle,
        source,
    } = operation;
    let returned = match work.try_finish_map(&mut manager, token, None, &mut no_record) {
        Err((ObjectMapTickError::ActorOperationInFlight { guid: pending }, token)) => {
            assert_eq!(pending, guid);
            token
        }
        _ => panic!("finish must return original pending token"),
    };
    let mut operation = ReservedCreatureLootGeneration {
        token: returned,
        handle,
        source,
    };
    assert_pending(&work, &mut manager, &mut operation, guid);
    let token = dispose(&work, &manager, operation);
    assert_eq!(
        finish(&mut work, &mut manager, token),
        ObjectMapFinishOutcome::Completed
    );
}
