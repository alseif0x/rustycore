//! Whole-operation completion policy and fail-stop retention.
use super::*;

pub(super) fn completed_session_finalization(
    report: &Option<wow_world::FinalizationReport>,
) -> bool {
    finalization_disposition_can_release(report.as_ref().map(|report| report.disposition))
}

fn finalization_disposition_can_release(
    disposition: Option<wow_world::FinalizationDisposition>,
) -> bool {
    disposition == Some(wow_world::FinalizationDisposition::Complete)
}

/// Fail-stop retention, not a recovery worker. The existing session task keeps
/// ownership and remains registered until the Tokio runtime is torn down. Its
/// supervisor has already closed admission and requested terminal error status.
/// Process teardown does not claim that a remote DB operation rolled back.
pub(super) async fn retain_session_until_process_teardown(session: &mut WorldSession) {
    std::future::pending::<()>().await;
    // Keep the mutable borrow live across the wait: no early Session/claim drop.
    let _ = session.finalization_report_like_cpp();
}

/// The production task's finalization and destructor boundary. Taking the
/// session and registration by value keeps both alive on fail-stop retention.
pub(super) async fn finalize_owned_world_session_like_cpp(
    mut session: WorldSession,
    outcome: WorldSessionRunOutcomeLikeCpp,
    account_id: u32,
    active_session_registration: ActiveWorldSessionRegistrationGuardLikeCpp,
    world_runtime_state: &WorldRuntimeStateLikeCpp,
    item_guid_generator: &wow_core::ObjectGuidGenerator,
    step_timeout: Duration,
) {
    let pending_finalization = match outcome {
        WorldSessionRunOutcomeLikeCpp::FinalizeWorldPass(pending) => Some(pending),
        WorldSessionRunOutcomeLikeCpp::ForceCancelled => {
            tracing::error!(
                account_id,
                "Force-cancelled world session after shutdown grace period"
            );
            None
        }
        WorldSessionRunOutcomeLikeCpp::Finished => None,
    };
    let active_session_registry = Arc::clone(&active_session_registration.registry);
    // Retired reads cannot start writes. Join writes already submitted by ready
    // callbacks before saving/discarding this Session. A timeout is fatal, not an
    // acknowledgement that a transaction rolled back or a worker stopped.
    let rename_drain = session.finish_character_rename_callbacks_like_cpp();
    let rename_finished = if active_session_registry.is_shutting_down_like_cpp() {
        run_world_session_shutdown_finalize_step_like_cpp(
            world_runtime_state,
            step_timeout,
            rename_drain,
        )
        .await
            == Some(true)
    } else {
        rename_drain.await
    };
    if !rename_finished {
        active_session_registry.begin_shutdown_like_cpp();
        active_session_registry.request_session_stop_like_cpp();
        world_runtime_state.stop_now_like_cpp(ERROR_EXIT_CODE_LIKE_CPP);
        tracing::error!(
            account_id,
            "Rename writer completion remains owned by session; refusing finalization and release"
        );
        retain_session_until_process_teardown(&mut session).await;
    }
    let attempt = if active_session_registry.is_shutting_down_like_cpp() {
        run_world_session_shutdown_finalize_step_like_cpp(
            world_runtime_state,
            step_timeout,
            session.finalize_session_with_generator_like_cpp(
                wow_world::FinalizationMode::Disconnect,
                item_guid_generator,
            ),
        )
        .await
    } else {
        Some(
            session
                .finalize_session_with_generator_like_cpp(
                    wow_world::FinalizationMode::Disconnect,
                    item_guid_generator,
                )
                .await,
        )
    };
    let report = attempt.or_else(|| session.interrupt_finalization_like_cpp());
    if !completed_session_finalization(&report) {
        // Keep this task's Session, exact claim, remaining operation state and
        // registration alive. Closing admission precedes fail-stop; no other
        // session can race a still-unproven writer through a released claim.
        active_session_registry.begin_shutdown_like_cpp();
        active_session_registry.request_session_stop_like_cpp();
        world_runtime_state.stop_now_like_cpp(ERROR_EXIT_CODE_LIKE_CPP);
        tracing::error!(
            account_id,
            ?report,
            "Finalization unresolved; retaining task-owned session until process teardown"
        );
        retain_session_until_process_teardown(&mut session).await;
    }
    // Session fields include the battle-pet account attachment. Its Drop
    // releases the process lease; that too precedes the next World participant.
    drop(session);
    drop(active_session_registration);
    if let Some(pending) = pending_finalization {
        pending.complete_like_cpp();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wow_world::FinalizationDisposition;

    #[test]
    fn shutdown_finalization_requires_whole_operation_completion() {
        assert!(finalization_disposition_can_release(Some(
            FinalizationDisposition::Complete
        )));
        for disposition in [
            None,
            Some(FinalizationDisposition::InProgress),
            Some(FinalizationDisposition::RetainAndEscalate),
        ] {
            assert!(!finalization_disposition_can_release(disposition));
        }
        assert!(!completed_session_finalization(&None));
    }

    #[tokio::test]
    async fn shutdown_timeout_does_not_authorize_independent_cleanup() {
        let runtime = WorldRuntimeStateLikeCpp::new();
        let result = run_world_session_shutdown_finalize_step_like_cpp(
            &runtime,
            Duration::from_millis(1),
            std::future::pending::<wow_world::FinalizationReport>(),
        )
        .await;
        assert!(!completed_session_finalization(&result));
        assert_eq!(runtime.get_exit_code_like_cpp(), ERROR_EXIT_CODE_LIKE_CPP);
    }
}
