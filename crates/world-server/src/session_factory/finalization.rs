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
