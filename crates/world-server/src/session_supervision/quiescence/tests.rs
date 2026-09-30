use super::*;
use std::future::{Future, poll_fn};
use std::pin::Pin;
use std::task::Poll;

const DEADLINE: Duration = Duration::from_secs(2);

fn registry() -> Arc<ActiveWorldSessionRegistryLikeCpp> {
    Arc::new(ActiveWorldSessionRegistryLikeCpp::new())
}

async fn assert_pending<F: Future>(mut future: Pin<&mut F>) {
    poll_fn(|cx| {
        assert!(matches!(future.as_mut().poll(cx), Poll::Pending));
        Poll::Ready(())
    }).await;
}

async fn authorize_drain(registry: &ActiveWorldSessionRegistryLikeCpp) {
    let request = registry.close_tick_admission();
    let receipt = registry.wait_for_quiescence(request, DEADLINE).await.unwrap();
    registry.enable_session_drain(receipt).unwrap();
}

async fn authorize_final(registry: &ActiveWorldSessionRegistryLikeCpp) {
    let request = registry.close_tick_admission();
    let receipt = registry.wait_for_terminal_settlement(request, DEADLINE).await.unwrap();
    registry.authorize_final_respawn_tick(receipt).unwrap();
}

#[test]
fn concurrent_close_serializes_registration_and_tick_admission() {
    let registry = registry();
    let origin = registry.register_producer(ProducerKind::Canonical);
    let barrier = Arc::new(std::sync::Barrier::new(2));
    let admitting = Arc::clone(&registry);
    let admitting_barrier = Arc::clone(&barrier);
    let thread = std::thread::spawn(move || {
        admitting_barrier.wait();
        let (command_tx, _) = flume::unbounded();
        let (phase_tx, _) = flume::unbounded();
        let registration = admitting.try_register(1, command_tx, phase_tx).map(|entry| entry.0);
        (registration, admitting.try_admit_tick(origin, 1, false))
    });
    barrier.wait();
    registry.begin_shutdown_like_cpp();
    let (registration, admitted) = thread.join().unwrap();
    assert!(registry.try_admit_tick(origin, 2, false).is_none());
    let (command_tx, _) = flume::unbounded();
    let (phase_tx, _) = flume::unbounded();
    assert!(registry.try_register(1, command_tx, phase_tx).is_none());
    if let Some(ticket) = admitted {
        assert!(ticket.complete(TickDisposition::AbandonedAfterAccounting));
    }
    if let Some(id) = registration { registry.unregister(id); }
}

#[tokio::test]
async fn object_tail_does_not_settle_the_post_tail_game_event_db_wait() {
    let registry = registry();
    let origin = registry.register_producer(ProducerKind::Canonical);
    let ticket = registry.try_admit_tick(origin, 7, false).unwrap();
    ticket.enter_phase(TickPhase::Objects);
    ticket.enter_phase(TickPhase::PostTail);
    let (db_done, db_wait) = tokio::sync::oneshot::channel();
    let (effects_done, effects_wait) = tokio::sync::oneshot::channel();
    let worker = tokio::spawn(async move {
        db_wait.await.unwrap();
        // This is the later effect/publication boundary, after the DB wait.
        effects_done.send(()).unwrap();
        assert!(ticket.complete(TickDisposition::FullyFinished));
    });
    let request = registry.close_tick_admission();
    let mut receipt = Box::pin(registry.wait_for_quiescence(request, DEADLINE));
    assert_pending(receipt.as_mut()).await;
    assert!(!registry.session_drain_authorized());
    db_done.send(()).unwrap();
    effects_wait.await.unwrap();
    registry.enable_session_drain(receipt.await.unwrap()).unwrap();
    worker.await.unwrap();
    assert!(registry.session_drain_authorized());
}

#[tokio::test]
async fn aborting_outer_task_does_not_acknowledge_its_blocking_ticket() {
    let registry = registry();
    let origin = registry.register_producer(ProducerKind::Legacy);
    let ticket = registry.try_admit_tick(origin, 3, false).unwrap();
    ticket.enter_phase(TickPhase::Legacy);
    let (entered_tx, entered_rx) = flume::bounded(1);
    let (release_tx, release_rx) = flume::bounded(1);
    let outer = tokio::spawn(async move {
        tokio::task::spawn_blocking(move || {
            entered_tx.send(()).unwrap();
            release_rx.recv().unwrap();
            assert!(ticket.complete(TickDisposition::FullyFinished));
        }).await.unwrap();
    });
    entered_rx.recv_async().await.unwrap();
    outer.abort();
    assert!(outer.await.unwrap_err().is_cancelled());
    let request = registry.close_tick_admission();
    assert_eq!(registry.wait_for_quiescence(request, Duration::from_millis(1)).await.unwrap_err(),
        QuiescenceFailure::TimedOut);
    assert!(!registry.session_drain_authorized());
    release_tx.send(()).unwrap();
    authorize_drain(&registry).await;
}

#[tokio::test]
async fn dropping_an_admitted_tick_retains_uncertainty() {
    let registry = registry();
    let origin = registry.register_producer(ProducerKind::Canonical);
    drop(registry.try_admit_tick(origin, 4, false).unwrap());
    let request = registry.close_tick_admission();
    assert_eq!(registry.wait_for_quiescence(request, Duration::from_millis(1)).await.unwrap_err(),
        QuiescenceFailure::TimedOut);
    assert!(!registry.session_drain_authorized());
    let request = registry.close_tick_admission();
    assert_eq!(registry.wait_for_terminal_settlement(request, Duration::from_millis(1)).await.unwrap_err(),
        QuiescenceFailure::TimedOut);
}

#[tokio::test]
async fn panicking_blocking_owner_does_not_complete_its_ticket() {
    let registry = registry();
    let origin = registry.register_producer(ProducerKind::Legacy);
    let ticket = registry.try_admit_tick(origin, 1, false).unwrap();
    ticket.enter_phase(TickPhase::Legacy);
    let result = tokio::task::spawn_blocking(move || {
        let _owned = ticket;
        panic!("controlled actor failure");
    }).await;
    assert!(result.unwrap_err().is_panic());
    let request = registry.close_tick_admission();
    assert_eq!(registry.wait_for_quiescence(request, Duration::from_millis(1)).await.unwrap_err(),
        QuiescenceFailure::TimedOut);
}

#[tokio::test]
async fn receipt_from_another_issuer_cannot_authorize_drain() {
    let first = registry();
    let second = registry();
    let request = first.close_tick_admission();
    let receipt = first.wait_for_quiescence(request, DEADLINE).await.unwrap();
    second.begin_shutdown_like_cpp();
    assert_eq!(second.enable_session_drain(receipt), Err(QuiescenceFailure::WrongClosure));
    assert!(!second.session_drain_authorized());
}

#[test]
fn producer_origin_from_another_registry_cannot_admit_work() {
    let first = registry();
    let second = registry();
    let foreign = first.register_producer(ProducerKind::Canonical);
    let _local = second.register_producer(ProducerKind::Canonical);
    assert!(second.try_admit_tick(foreign, 1, false).is_none());
}

#[tokio::test]
async fn a_world_prefix_cannot_claim_full_simulation_completion() {
    let registry = registry();
    let origin = registry.register_producer(ProducerKind::Canonical);
    let ticket = registry.try_admit_tick(origin, 1, false).unwrap();
    assert!(!ticket.complete(TickDisposition::FullyFinished));
    let request = registry.close_tick_admission();
    assert_eq!(registry.wait_for_quiescence(request, Duration::from_millis(1)).await.unwrap_err(),
        QuiescenceFailure::TimedOut);
}

#[tokio::test]
async fn terminal_receipt_is_bound_to_issuer_and_closure_generation() {
    let first = registry();
    let second = registry();
    authorize_drain(&first).await;
    authorize_drain(&second).await;
    let request = first.close_tick_admission();
    let receipt = first.wait_for_terminal_settlement(request, DEADLINE).await.unwrap();
    assert_eq!(second.authorize_final_respawn_tick(receipt), Err(QuiescenceFailure::WrongClosure));
    let request = first.close_tick_admission();
    let stale = first.wait_for_terminal_settlement(request, DEADLINE).await.unwrap();
    authorize_final(&first).await;
    first.close_final_tick_admission();
    assert_eq!(first.authorize_final_respawn_tick(stale), Err(QuiescenceFailure::WrongClosure));
}

#[tokio::test]
async fn receipt_from_an_earlier_closure_generation_is_rejected() {
    let registry = registry();
    authorize_drain(&registry).await;
    let request = registry.close_tick_admission();
    let stale = registry.wait_for_quiescence(request, DEADLINE).await.unwrap();
    authorize_final(&registry).await;
    registry.close_final_tick_admission();
    assert_eq!(registry.enable_session_drain(stale), Err(QuiescenceFailure::WrongClosure));
}

#[tokio::test]
async fn shutdown_request_alone_does_not_authorize_a_waiting_finalizer() {
    let registry = registry();
    registry.begin_shutdown_like_cpp();
    let writer = registry.admit_finalization(None);
    let mut admission = Box::pin(writer.wait());
    assert_pending(admission.as_mut()).await;
    authorize_drain(&registry).await;
    admission.await;
    writer.complete();
}

#[tokio::test]
async fn unphased_finalizer_waits_for_both_producers_before_effects() {
    let registry = registry();
    let canonical = registry.register_producer(ProducerKind::Canonical);
    let legacy = registry.register_producer(ProducerKind::Legacy);
    let first = registry.try_admit_tick(canonical, 1, false).unwrap();
    let second = registry.try_admit_tick(legacy, 1, false).unwrap();
    second.enter_phase(TickPhase::Legacy);
    let writer = registry.admit_finalization(None);
    let mut admission = Box::pin(writer.wait());
    assert_pending(admission.as_mut()).await;
    assert!(registry.try_admit_tick(canonical, 2, false).is_none());
    assert!(first.complete(TickDisposition::AbandonedAfterAccounting));
    assert_pending(admission.as_mut()).await;
    assert!(second.complete(TickDisposition::FullyFinished));
    admission.await;
    assert!(registry.try_admit_tick(canonical, 2, false).is_none());
    writer.complete();
    let next = registry.try_admit_tick(canonical, 2, false).unwrap();
    assert!(next.complete(TickDisposition::AbandonedAfterAccounting));
}

#[tokio::test]
async fn concurrent_finalizer_intents_never_reopen_ticks_between_writers() {
    let registry = registry();
    let origin = registry.register_producer(ProducerKind::Canonical);
    let first = registry.admit_finalization(None);
    first.wait().await;
    let second = registry.admit_finalization(None);
    let mut second_wait = Box::pin(second.wait());
    assert_pending(second_wait.as_mut()).await;
    first.complete();
    assert!(registry.try_admit_tick(origin, 1, false).is_none());
    second_wait.await;
    second.complete();
    let tick = registry.try_admit_tick(origin, 1, false).unwrap();
    assert!(tick.complete(TickDisposition::AbandonedAfterAccounting));
}

#[tokio::test]
async fn cancelling_a_waiting_finalizer_retains_its_intent() {
    let registry = registry();
    registry.begin_shutdown_like_cpp();
    let writer = registry.admit_finalization(None);
    let mut admission = Box::pin(writer.wait());
    assert_pending(admission.as_mut()).await;
    drop(admission);
    drop(writer);
    let request = registry.close_tick_admission();
    assert_eq!(registry.wait_for_quiescence(request, Duration::from_millis(1)).await.unwrap_err(),
        QuiescenceFailure::TimedOut);
}

#[tokio::test]
async fn finalizer_completion_after_close_cannot_reopen_normal_ticks() {
    let registry = registry();
    let origin = registry.register_producer(ProducerKind::Canonical);
    let writer = registry.admit_finalization(None);
    writer.wait().await;
    let request = registry.close_tick_admission();
    let mut receipt = Box::pin(registry.wait_for_quiescence(request, DEADLINE));
    assert_pending(receipt.as_mut()).await;
    writer.complete();
    registry.enable_session_drain(receipt.await.unwrap()).unwrap();
    assert!(registry.try_admit_tick(origin, 1, false).is_none());
}

#[tokio::test]
async fn final_tick_refuses_pending_finalization() {
    let registry = registry();
    authorize_drain(&registry).await;
    let request = registry.close_tick_admission();
    let receipt = registry.wait_for_terminal_settlement(request, DEADLINE).await.unwrap();
    let writer = registry.admit_finalization(None);
    assert_eq!(registry.authorize_final_respawn_tick(receipt), Err(QuiescenceFailure::Busy));
    writer.wait().await;
    writer.complete();
    authorize_final(&registry).await;
}

#[tokio::test]
async fn busy_registry_cannot_authorize_a_final_tick() {
    let registry = registry();
    let (tx, _rx) = flume::unbounded();
    let id = registry.register(1, tx);
    authorize_drain(&registry).await;
    let request = registry.close_tick_admission();
    assert_eq!(registry.wait_for_terminal_settlement(request, Duration::from_millis(1)).await.unwrap_err(),
        QuiescenceFailure::TimedOut);
    registry.unregister(id);
    authorize_final(&registry).await;
}

#[tokio::test]
async fn registration_drop_wakes_drain_but_terminal_wait_retains_the_finalizer() {
    let registry = registry();
    let origin = registry.register_producer(ProducerKind::Canonical);
    let (tx, _rx) = flume::unbounded();
    let id = registry.register(1, tx);
    let registration = crate::ActiveWorldSessionRegistrationGuardLikeCpp {
        registry: Arc::clone(&registry), id,
    };
    let writer = registry.admit_finalization(None);
    writer.wait().await;
    registry.begin_shutdown_like_cpp();
    drop(registration); // The real guard unregisters and notifies the supervisor.
    assert!(registry.wait_until_empty_like_cpp(DEADLINE).await);
    assert_eq!(registry.len_like_cpp(), 0);
    let request = registry.close_tick_admission();
    let mut terminal = Box::pin(registry.wait_for_terminal_settlement(request, DEADLINE));
    assert_pending(terminal.as_mut()).await;
    assert!(registry.try_admit_tick(origin, 1, true).is_none());
    writer.complete(); // Only the destructor owner's explicit completion settles it.
    authorize_drain(&registry).await;
    registry.authorize_final_respawn_tick(terminal.await.unwrap()).unwrap();
    let tick = registry.try_admit_tick(origin, 1, true).unwrap();
    tick.enter_phase(TickPhase::PostTail);
    assert!(tick.complete(TickDisposition::FullyFinished));
}

#[tokio::test]
async fn waiting_finalizer_permits_initial_handover_but_not_terminal_mailbox_settlement() {
    let registry = registry();
    registry.begin_shutdown_like_cpp();
    let writer = registry.admit_finalization(None);
    let request = registry.close_tick_admission();
    let mut terminal = Box::pin(registry.wait_for_terminal_settlement(request, DEADLINE));
    assert_pending(terminal.as_mut()).await;
    authorize_drain(&registry).await;
    assert_pending(terminal.as_mut()).await;
    writer.wait().await;
    assert_pending(terminal.as_mut()).await;
    writer.complete();
    assert!(terminal.await.is_ok());
}

#[tokio::test]
async fn final_tick_is_single_use_and_cutoff_rejects_late_admission() {
    let registry = registry();
    let canonical = registry.register_producer(ProducerKind::Canonical);
    let legacy = registry.register_producer(ProducerKind::Legacy);
    authorize_drain(&registry).await;
    assert!(registry.try_admit_tick(canonical, 1, true).is_none());
    authorize_final(&registry).await;
    let ticket = registry.try_admit_tick(canonical, 1, true).unwrap();
    assert!(registry.try_admit_tick(legacy, 1, true).is_none());
    ticket.enter_phase(TickPhase::PostTail);
    assert!(ticket.complete(TickDisposition::FullyFinished));
    assert!(registry.try_admit_tick(canonical, 2, true).is_none());
    registry.close_final_tick_admission();
    assert!(registry.try_admit_tick(legacy, 1, true).is_none());
}
