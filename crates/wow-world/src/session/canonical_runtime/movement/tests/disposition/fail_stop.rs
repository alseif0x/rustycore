use super::*;

#[test]
fn errors_without_owned_io_evidence_return_self_and_never_release_pending_slot() {
    let (manager, mut tick, mut token, guid) = fixture(650_030);
    let request = pending(&manager, &tick, &mut token, guid, 0);
    for failure in [MovementFailure::Actor(ActorMovementError::ActorUnavailable { guid }),
        MovementFailure::MapId(u32::MAX), MovementFailure::Poisoned]
    {
        let partial = partial();
        let buffers = packet_buffers(&partial);
        let error = CanonicalMovementError::new(partial, failure);
        let original_message = error.to_string();
        let error = match error.dispose_owned_request(&manager, &tick, &mut token) {
            Err(error) => error, Ok(_) => panic!("no input authorizes no disposal"),
        };
        assert_eq!(error.to_string(), original_message);
        assert_partial(&error.partial, &buffers);
        assert_busy(&manager);
    }
    drop(request);
    assert!(matches!(manager.lock().unwrap().try_finish_object_map::<LoadRecord>(
        &mut tick, token, None, None, MapCreatureUpdateOwnerLikeCpp::ExternalRuntime),
        Err((ObjectMapTickError::ActorOperationInFlight { .. }, _))));
    assert_busy(&manager);
}

#[tokio::test]
async fn query_panic_with_lost_continuation_cannot_masquerade_as_disposable_work() {
    let (manager, mut tick, mut token, guid) = fixture(650_031);
    let request = pending(&manager, &tick, &mut token, guid, 0);
    let failed = tokio::task::spawn_blocking(move || -> () {
        let ActorMovementPending::Path(request) = request else { panic!("path"); };
        let (_query, _continuation) = request.into_parts();
        panic!("fixture worker loses its continuation before returning a reply");
    }).await.err().expect("worker panic yields a JoinError");
    assert!(failed.is_panic());
    let partial = partial();
    let buffers = packet_buffers(&partial);
    let error = CanonicalMovementError::new(partial, MovementFailure::QueryPanicked(failed));
    let error = match error.dispose_owned_request(&manager, &tick, &mut token) {
        Err(error) => error, Ok(_) => panic!("no owned continuation came back from the worker"),
    };
    assert!(matches!(&error.failure, MovementFailure::QueryPanicked(error) if error.is_panic()));
    assert_partial(&error.partial, &buffers);
    assert!(matches!(manager.lock().unwrap().try_finish_object_map::<LoadRecord>(
        &mut tick, token, None, None, MapCreatureUpdateOwnerLikeCpp::ExternalRuntime),
        Err((ObjectMapTickError::ActorOperationInFlight { .. }, _))));
    assert_busy(&manager);
}

#[tokio::test]
async fn dropping_missing_terrain_error_does_not_run_disposer_or_map_tail() {
    let (manager, mut tick, mut token, guid) = fixture(650_032);
    let request = pending(&manager, &tick, &mut token, guid, 1);
    let failure = match queries::resolve(guid, request, None, None).await {
        Err(failure @ MovementFailure::MissingTerrain(_)) => failure,
        _ => panic!("missing terrain"),
    };
    drop(CanonicalMovementError::new(partial(), failure));
    assert!(matches!(manager.lock().unwrap().try_finish_object_map::<LoadRecord>(
        &mut tick, token, None, None, MapCreatureUpdateOwnerLikeCpp::ExternalRuntime),
        Err((ObjectMapTickError::ActorOperationInFlight { .. }, _))));
    assert_busy(&manager);
}
