//! Real phase consumer, coordinator and task-owned finalizer under controlled I/O.
use super::*;
use std::future::{Future, poll_fn};
use std::pin::Pin;
use std::task::Poll;
use wow_persistence::{PersistenceOutcomeLikeCpp, PlayerOfflineMarkLikeCpp};
use wow_world::session::SessionHandlerCatalogsLikeCpp;
use wow_world::session::mailbox::{
    RunWorldPhasePassLikeCppRequest, RunWorldPhasePassResultLikeCpp,
    SessionPhasePassOutcomeLikeCpp, SessionPhasePermitLikeCpp, SessionPhasePermitStateLikeCpp,
    SessionPhaseRequestLikeCpp,
};

mod fixtures;

#[tokio::test]
async fn a_disconnecting_session_refuses_stale_map_work_and_waits_for_world_retirement() {
    let (mut harness, _packets, _sent) = fixtures::session();
    let (entered, _release, _destroyed) = fixtures::offline_port(&mut harness.session);
    let admission = fixtures::retired_map_admission(&harness.session);
    harness
        .session
        .kick("disconnect before a queued Map request");
    let phase_tx = harness.session.session_phase_sender_like_cpp();
    let rail = harness.session.session_phase_receiver_like_cpp();
    let (map_response_tx, map_reply) = flume::bounded(1);
    phase_tx
        .send(SessionPhaseRequestLikeCpp::Map(
            wow_world::session::mailbox::RunMapPhasePassLikeCppCommand {
                admission,
                permit: SessionPhasePermitLikeCpp::new_like_cpp(),
                response_tx: map_response_tx,
            },
        ))
        .unwrap();
    let catalogs = SessionHandlerCatalogsLikeCpp::default();
    let mut consumer = Box::pin(run_world_session_phase_loop_like_cpp(
        &mut harness.session,
        &catalogs,
        787,
        &harness.registry,
        &harness.cancellation,
        &rail,
        &harness.ready,
    ));
    assert_pending(consumer.as_mut()).await;
    let map_result = map_reply.try_recv().unwrap();
    assert_eq!(
        map_result.outcome,
        SessionPhasePassOutcomeLikeCpp::RefusedBeforeStart
    );
    assert!(map_result.disconnecting);
    assert!(
        entered.is_empty(),
        "Map may not enter the disconnect finalizer"
    );
    assert_eq!(harness.registry.len_like_cpp(), 1);
    let (world, permit, reply) = world_request();
    phase_tx.send(world).unwrap();
    let outcome = consumer.await;
    assert!(matches!(
        outcome,
        WorldSessionRunOutcomeLikeCpp::FinalizeWorldPass(_)
    ));
    assert_eq!(
        permit.state_like_cpp(),
        SessionPhasePermitStateLikeCpp::Running
    );
    assert_eq!(reply.try_recv(), Err(flume::TryRecvError::Empty));
    drop(outcome);
    assert_eq!(reply.try_recv(), Err(flume::TryRecvError::Disconnected));
}

async fn assert_pending<F: Future>(mut future: Pin<&mut F>) {
    poll_fn(|cx| {
        assert!(
            future.as_mut().poll(cx).is_pending(),
            "operation completed too early"
        );
        Poll::Ready(())
    })
    .await;
}

fn world_request() -> (
    SessionPhaseRequestLikeCpp,
    Arc<SessionPhasePermitLikeCpp>,
    flume::Receiver<RunWorldPhasePassResultLikeCpp>,
) {
    let permit = SessionPhasePermitLikeCpp::new_like_cpp();
    let (response_tx, response_rx) = flume::bounded(1);
    (
        SessionPhaseRequestLikeCpp::World(RunWorldPhasePassLikeCppRequest {
            coordinator_id: 787,
            tick_epoch: 1,
            diff_ms: 1,
            permit: Arc::clone(&permit),
            response_tx,
        }),
        permit,
        response_rx,
    )
}

// Inspect the real producer's permit without changing request identity or order.
fn queued_world_permit(session: &WorldSession) -> Arc<SessionPhasePermitLikeCpp> {
    let request = session
        .session_phase_receiver_like_cpp()
        .try_recv()
        .unwrap();
    let SessionPhaseRequestLikeCpp::World(world) = &request else {
        panic!("expected World")
    };
    let permit = Arc::clone(&world.permit);
    session
        .session_phase_sender_like_cpp()
        .try_send(request)
        .unwrap();
    permit
}

#[tokio::test]
async fn world_coordinator_waits_for_session_finalization_destruction_and_retirement() {
    let (mut harness, _packets, _sent) = fixtures::session();
    let (entered, release, destroyed) = fixtures::offline_port(&mut harness.session);
    let registry = Arc::clone(&harness.registry);
    let ready = Arc::clone(&harness.ready);
    let rail = harness.session.session_phase_receiver_like_cpp();
    let first_tx = harness.session.session_phase_sender_like_cpp();
    let (second_tx, second_rx) = flume::bounded(1);
    let participants = [first_tx.clone(), second_tx];
    let mut producer = Box::pin(crate::runtime::run_world_phase_session_passes_like_cpp(
        &participants,
        787,
        1,
        1,
        Duration::from_secs(1),
    ));
    // Kick is consumed by the actual World pass, not by test-side finalization.
    assert_eq!(kick_all_sessions_like_cpp(&registry).queued, 1);
    assert_pending(producer.as_mut()).await;
    let permit = queued_world_permit(&harness.session);
    let runtime = Arc::new(WorldRuntimeStateLikeCpp::new());
    let mut consumer = Box::pin(harness.drive(runtime));
    assert_pending(consumer.as_mut()).await;
    assert_eq!(
        entered.try_recv().unwrap(),
        PlayerOfflineMarkLikeCpp::LoginAccount { account_id: 787 }
    );
    assert_eq!(
        permit.state_like_cpp(),
        SessionPhasePermitStateLikeCpp::Running
    );
    assert!(!ready.load(Ordering::Acquire));
    assert_eq!(registry.len_like_cpp(), 1);
    assert!(!destroyed.load(Ordering::Acquire));
    assert_pending(producer.as_mut()).await;
    assert!(
        second_rx.is_empty(),
        "another participant overtook the finalizer"
    );
    assert!(rail.is_empty());

    release
        .send(PersistenceOutcomeLikeCpp::Applied { rows: 1 })
        .unwrap();
    consumer.await;
    assert!(
        destroyed.load(Ordering::Acquire),
        "session-owned fields must be dropped"
    );
    assert_eq!(registry.len_like_cpp(), 0);
    assert_eq!(
        permit.state_like_cpp(),
        SessionPhasePermitStateLikeCpp::Completed
    );
    assert_pending(producer.as_mut()).await;
    let SessionPhaseRequestLikeCpp::World(second) = second_rx.try_recv().unwrap() else {
        panic!("expected second World participant");
    };
    assert!(matches!(
        second.permit.claim_like_cpp(),
        wow_world::session::mailbox::SessionPhaseClaimLikeCpp::Claimed
    ));
    assert!(second.permit.complete_like_cpp());
    second
        .response_tx
        .send(RunWorldPhasePassResultLikeCpp {
            coordinator_id: second.coordinator_id,
            tick_epoch: second.tick_epoch,
            outcome: SessionPhasePassOutcomeLikeCpp::Ran,
            dispatched: 0,
            disconnecting: false,
        })
        .unwrap();
    let summary = producer.await;
    assert_eq!(summary.completed, 2);
    assert!(summary.quiescent_like_cpp());
}

#[tokio::test]
async fn shutdown_drains_kick_and_flush_after_the_phase_producer_disappears() {
    let (mut harness, _packets, _sent) = fixtures::session();
    let (entered, release, destroyed) = fixtures::offline_port(&mut harness.session);
    let registry = Arc::clone(&harness.registry);
    let ready = Arc::clone(&harness.ready);
    let phase_tx = harness.session.session_phase_sender_like_cpp();
    let rail = harness.session.session_phase_receiver_like_cpp();
    let runtime = Arc::new(WorldRuntimeStateLikeCpp::new());
    let mut consumer = Box::pin(harness.drive(runtime));
    assert_pending(consumer.as_mut()).await;
    assert!(ready.load(Ordering::Acquire));
    let participants = [phase_tx.clone()];
    let mut producer = Box::pin(crate::runtime::run_world_phase_session_passes_like_cpp(
        &participants,
        787,
        1,
        1,
        Duration::from_secs(1),
    ));
    assert_pending(producer.as_mut()).await;
    let request = rail.try_recv().unwrap();
    let SessionPhaseRequestLikeCpp::World(world) = &request else {
        panic!("expected World")
    };
    let permit = Arc::clone(&world.permit);
    phase_tx.try_send(request).unwrap();
    drop(producer); // Real coordinator future is cancelled with its request queued.
    registry.begin_shutdown_like_cpp();
    assert_eq!(kick_all_sessions_like_cpp(&registry).queued, 1);
    let mut flush = Box::pin(update_sessions_shutdown_flush_once_like_cpp(
        &registry,
        1,
        Duration::from_secs(1),
    ));
    assert_pending(flush.as_mut()).await;
    // The consumer resumes its parked recv; it must recheck shutdown before claim.
    assert_pending(consumer.as_mut()).await;
    assert_eq!(
        permit.state_like_cpp(),
        SessionPhasePermitStateLikeCpp::RefusedBeforeStart
    );
    assert!(!ready.load(Ordering::Acquire));
    assert_eq!(
        entered.try_recv().unwrap(),
        PlayerOfflineMarkLikeCpp::LoginAccount { account_id: 787 }
    );
    let summary = flush.await;
    assert_eq!(summary.acked, 1);
    assert_eq!(summary.disconnecting, 1);
    assert_eq!(summary.ack_failed + summary.ack_timeout, 0);
    assert_eq!(registry.len_like_cpp(), 1, "flush is not finalization");
    release
        .send(PersistenceOutcomeLikeCpp::Applied { rows: 1 })
        .unwrap();
    consumer.await;
    assert!(destroyed.load(Ordering::Acquire));
    assert_eq!(registry.len_like_cpp(), 0);
}

#[tokio::test]
async fn failed_unknown_and_timed_out_finalization_retain_the_world_reply_and_session() {
    for outcome in [
        Some(PersistenceOutcomeLikeCpp::Failed {
            reason: "controlled rollback".into(),
        }),
        Some(PersistenceOutcomeLikeCpp::Unknown {
            reason: "controlled lost reply".into(),
        }),
        None,
    ] {
        let (mut harness, _packets, _sent) = fixtures::session();
        let (entered, release, destroyed) = fixtures::offline_port(&mut harness.session);
        harness.session.kick("controlled disconnect");
        let catalogs = SessionHandlerCatalogsLikeCpp::default();
        let (request, permit, reply) = world_request();
        let pending = harness
            .session
            .run_requested_session_phase_like_cpp(request, &catalogs)
            .await
            .expect("a disconnecting World pass retains finalization");
        let registry = Arc::clone(&harness.registry);
        let runtime = WorldRuntimeStateLikeCpp::new();
        if outcome.is_none() {
            registry.begin_shutdown_like_cpp();
        }
        let mut finalizer = Box::pin(finalization::finalize_owned_world_session_like_cpp(
            harness.session,
            WorldSessionRunOutcomeLikeCpp::FinalizeWorldPass(pending),
            787,
            harness.registration,
            &runtime,
            catalogs.id_generators.item.as_ref(),
            Duration::from_millis(1),
        ));
        assert_pending(finalizer.as_mut()).await;
        assert!(entered.try_recv().is_ok());
        if let Some(outcome) = outcome {
            release.send(outcome).unwrap();
            assert_pending(finalizer.as_mut()).await;
        } else {
            assert!(
                tokio::time::timeout(Duration::from_millis(10), finalizer.as_mut())
                    .await
                    .is_err()
            );
        }
        assert_eq!(runtime.get_exit_code_like_cpp(), ERROR_EXIT_CODE_LIKE_CPP);
        assert!(registry.should_stop_sessions_like_cpp());
        assert_eq!(registry.len_like_cpp(), 1);
        assert_eq!(
            permit.state_like_cpp(),
            SessionPhasePermitStateLikeCpp::Running
        );
        assert_eq!(reply.try_recv(), Err(flume::TryRecvError::Empty));
        assert!(!destroyed.load(Ordering::Acquire));
        // Only test teardown cancels fail-stop retention; this is never success.
        drop(finalizer);
        assert!(destroyed.load(Ordering::Acquire));
        assert_eq!(reply.try_recv(), Err(flume::TryRecvError::Disconnected));
        assert_eq!(
            permit.state_like_cpp(),
            SessionPhasePermitStateLikeCpp::Running
        );
    }
}

#[tokio::test]
async fn successful_finalization_replies_once_after_session_destruction() {
    let (mut harness, _packets, _sent) = fixtures::session();
    let (entered, release, destroyed) = fixtures::offline_port(&mut harness.session);
    harness.session.kick("controlled disconnect");
    let catalogs = SessionHandlerCatalogsLikeCpp::default();
    let (request, permit, reply) = world_request();
    let pending = harness
        .session
        .run_requested_session_phase_like_cpp(request, &catalogs)
        .await
        .unwrap();
    let registry = Arc::clone(&harness.registry);
    let runtime = WorldRuntimeStateLikeCpp::new();
    let mut finalizer = Box::pin(finalization::finalize_owned_world_session_like_cpp(
        harness.session,
        WorldSessionRunOutcomeLikeCpp::FinalizeWorldPass(pending),
        787,
        harness.registration,
        &runtime,
        catalogs.id_generators.item.as_ref(),
        Duration::from_secs(1),
    ));
    assert_pending(finalizer.as_mut()).await;
    assert!(entered.try_recv().is_ok());
    assert_eq!(reply.try_recv(), Err(flume::TryRecvError::Empty));
    release
        .send(PersistenceOutcomeLikeCpp::Applied { rows: 1 })
        .unwrap();
    finalizer.await;
    assert!(destroyed.load(Ordering::Acquire));
    assert_eq!(registry.len_like_cpp(), 0);
    assert_eq!(
        permit.state_like_cpp(),
        SessionPhasePermitStateLikeCpp::Completed
    );
    let result = reply.try_recv().unwrap();
    assert_eq!(result.outcome, SessionPhasePassOutcomeLikeCpp::Ran);
    assert!(result.disconnecting);
    assert_eq!(reply.try_recv(), Err(flume::TryRecvError::Disconnected));
}
