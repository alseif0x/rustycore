use super::*;

fn reply(request: ActorMovementPending) -> ResolvedMovementQuery {
    match request {
        ActorMovementPending::Path(request) => {
            let (_, continuation) = request.into_parts();
            ResolvedMovementQuery::Path(continuation, None)
        }
        ActorMovementPending::StaticHeight(request) => {
            let (_, continuation) = request.into_parts();
            ResolvedMovementQuery::StaticHeight(continuation, f32::from_bits(0x7fc0_1234))
        }
        ActorMovementPending::GridHeight(request) => {
            let (_, continuation) = request.into_parts();
            ResolvedMovementQuery::GridHeight(continuation, f32::from_bits(0x7fc0_1234))
        }
    }
}

#[test]
fn observed_three_reply_families_dispose_without_running_step_and_keep_packets() {
    for stage in 0..3 {
        let (manager, tick, mut token, guid) = fixture(650_010 + i64::from(stage));
        let response = reply(pending(&manager, &tick, &mut token, guid, stage));
        let before = manager.lock().unwrap().find_map(1, 0).unwrap().map()
            .with_creature_like_cpp(guid, |creature| (creature.ai_position(), creature.ai_state(), creature.unit().data().health)).unwrap();
        let partial = partial();
        let buffers = packet_buffers(&partial);
        let error = CanonicalMovementError::new(partial, MovementFailure::Resume {
            error: ActorMovementError::OperationMismatch { guid }, response,
        });
        let abandoned = error.dispose_owned_request(&manager, &tick, &mut token).unwrap_or_else(|error| panic!("{error}"));
        assert_eq!(abandoned.abandonment, CanonicalMovementAbandonment::ObservedReplyDiscarded);
        assert_partial(&abandoned.partial, &buffers);
        let after = manager.lock().unwrap().find_map(1, 0).unwrap().map()
            .with_creature_like_cpp(guid, |creature| (creature.ai_position(), creature.ai_state(), creature.unit().data().health)).unwrap();
        assert_eq!(after, before);
        assert_busy(&manager);
    }
}

#[tokio::test]
async fn actual_aba_resume_error_can_dispose_observed_worker_reply_without_touching_replacement() {
    let (manager, tick, mut token, guid) = fixture(650_020);
    let request = pending(&manager, &tick, &mut token, guid, 0);
    let response = queries::resolve(guid, request, None, None).await.unwrap_or_else(|_| panic!("worker returns owned reply"));
    {
        let mut manager = manager.lock().unwrap();
        drop(manager.find_map_mut(1, 0).unwrap().map_mut().remove_map_object(guid).unwrap());
        insert(&mut manager, actor(650_020));
    }
    let failure = match queries::resume(&manager, &tick, &mut token, response) {
        Err(failure @ MovementFailure::Resume { error: ActorMovementError::WitnessMismatch { .. }, .. }) => failure,
        _ => panic!("ABA rejects before typed resume"),
    };
    let partial = partial();
    let buffers = packet_buffers(&partial);
    let abandoned = CanonicalMovementError::new(partial, failure).dispose_owned_request(&manager, &tick, &mut token)
        .unwrap_or_else(|error| panic!("{error}"));
    assert_partial(&abandoned.partial, &buffers);
    assert_eq!(manager.lock().unwrap().find_map(1, 0).unwrap().map()
        .with_creature_like_cpp(guid, |creature| creature.ai_position()), Some(Position::xyz(30.0, 10.0, 0.0)));
    assert_busy(&manager);
}

#[test]
fn foreign_tick_disposal_retains_original_error_scalar_and_partial_buffers_for_retry() {
    let (manager, tick, mut token, guid) = fixture(650_021);
    let (_, foreign_tick, _foreign_token, _) = fixture(650_021);
    let response = reply(pending(&manager, &tick, &mut token, guid, 1));
    let original_error = ActorMovementError::ActorUnavailable { guid };
    let partial = partial();
    let buffers = packet_buffers(&partial);
    let error = CanonicalMovementError::new(partial, MovementFailure::Resume { error: original_error, response });
    let error = match error.dispose_owned_request(&manager, &foreign_tick, &mut token) {
        Err(error) => error, Ok(_) => panic!("foreign tick cannot dispose"),
    };
    assert_partial(&error.partial, &buffers);
    assert_eq!(error.actor_error(), Some(original_error));
    match &error.failure {
        MovementFailure::Resume { response: ResolvedMovementQuery::StaticHeight(_, response), .. } =>
            assert_eq!(response.to_bits(), 0x7fc0_1234),
        _ => panic!("owned input remains typed and intact"),
    }
    let abandoned = error.dispose_owned_request(&manager, &tick, &mut token).unwrap_or_else(|error| panic!("{error}"));
    assert_partial(&abandoned.partial, &buffers);
    assert_busy(&manager);
}

#[tokio::test]
async fn poisoned_resume_error_keeps_reply_until_caller_explicitly_restores_lock_access() {
    let (manager, tick, mut token, guid) = fixture(650_022);
    let request = pending(&manager, &tick, &mut token, guid, 0);
    let response = queries::resolve(guid, request, None, None).await.unwrap_or_else(|_| panic!("owned reply"));
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _guard = manager.lock().unwrap();
        panic!("fixture poisons the manager after the worker has joined");
    }));
    let failure = match queries::resume(&manager, &tick, &mut token, response) {
        Err(failure @ MovementFailure::ResumePoisoned(_)) => failure,
        _ => panic!("poison must retain the response"),
    };
    let partial = partial();
    let buffers = packet_buffers(&partial);
    let error = match CanonicalMovementError::new(partial, failure).dispose_owned_request(&manager, &tick, &mut token) {
        Err(error) => error, Ok(_) => panic!("disposer cannot recover poison implicitly"),
    };
    assert_partial(&error.partial, &buffers);
    assert!(matches!(&error.failure, MovementFailure::ResumePoisoned(ResolvedMovementQuery::Path(_, None))));
    manager.clear_poison();
    let abandoned = error.dispose_owned_request(&manager, &tick, &mut token).unwrap_or_else(|error| panic!("{error}"));
    assert_partial(&abandoned.partial, &buffers);
    assert_eq!(abandoned.abandonment, CanonicalMovementAbandonment::ObservedReplyDiscarded);
    assert_busy(&manager);
}
