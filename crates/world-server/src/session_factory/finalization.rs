//! Disconnect supervision: attempt completion is not whole-operation durability.
use super::*;
use wow_world::session::{DisconnectSaveAttemptLikeCpp, PlayerSaveOutcomeLikeCpp};

/// Preserve existing retirement policy while exposing the returned classification.
/// Only the established incomplete-native-work gate refuses normal cleanup.
/// Deferred/Unavailable still need cause-specific recovery; neither they nor a
/// completed attempt authorize retries or prove whole-operation durability.
pub(super) fn allow_disconnect_cleanup_after_attempt_like_cpp(
    runtime: &WorldRuntimeStateLikeCpp,
    account_id: u32,
    attempt: Option<DisconnectSaveAttemptLikeCpp>,
) -> bool {
    use DisconnectSaveAttemptLikeCpp::{Character, NativeCompletionUnavailable, NoPlayer};
    use PlayerSaveOutcomeLikeCpp::{Applied, Deferred, Failed, Quarantined, Unavailable};
    match attempt {
        Some(NativeCompletionUnavailable) => {
            runtime.stop_now_like_cpp(ERROR_EXIT_CODE_LIKE_CPP);
            tracing::error!(
                account_id,
                ?attempt,
                "Disconnect save not admitted; refusing normal cleanup"
            );
            false
        }
        Some(Character(Deferred | Unavailable | Failed | Quarantined)) => {
            // Preserve the existing rollback/retirement and quarantine/reload
            // contracts. Never retry an uncertain transaction or label it Applied.
            tracing::warn!(
                account_id,
                ?attempt,
                "Retiring disconnected session without confirmed character save"
            );
            true
        }
        Some(NoPlayer | Character(Applied)) => true,
        None => {
            runtime.stop_now_like_cpp(ERROR_EXIT_CODE_LIKE_CPP);
            true // Timeout is fatal, but cleanup still gets its own bounded attempt.
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn shutdown_preserves_returned_save_classification() {
        use DisconnectSaveAttemptLikeCpp::{Character, NativeCompletionUnavailable, NoPlayer};
        use PlayerSaveOutcomeLikeCpp::{Applied, Deferred, Failed, Quarantined, Unavailable};
        for report in [
            NoPlayer,
            NativeCompletionUnavailable,
            Character(Applied),
            Character(Deferred),
            Character(Unavailable),
            Character(Failed),
            Character(Quarantined),
        ] {
            let runtime = WorldRuntimeStateLikeCpp::new();
            let result = run_world_session_shutdown_finalize_step_like_cpp(
                &runtime,
                Duration::from_secs(1),
                async { report },
            )
            .await;
            assert_eq!(result, Some(report));
            // The timeout helper classifies completion, not semantic success.
            assert_eq!(
                runtime.get_exit_code_like_cpp(),
                SHUTDOWN_EXIT_CODE_LIKE_CPP
            );
            let admitted = !matches!(report, NativeCompletionUnavailable);
            assert_eq!(
                allow_disconnect_cleanup_after_attempt_like_cpp(&runtime, 1, result),
                admitted
            );
            assert_eq!(
                runtime.get_exit_code_like_cpp(),
                if admitted {
                    SHUTDOWN_EXIT_CODE_LIKE_CPP
                } else {
                    ERROR_EXIT_CODE_LIKE_CPP
                }
            );
        }
    }

    #[tokio::test]
    async fn shutdown_timeout_still_allows_independent_cleanup_attempt() {
        let runtime = WorldRuntimeStateLikeCpp::new();
        let result = run_world_session_shutdown_finalize_step_like_cpp(
            &runtime,
            Duration::from_millis(1),
            std::future::pending::<DisconnectSaveAttemptLikeCpp>(),
        )
        .await;
        assert_eq!(result, None);
        assert!(allow_disconnect_cleanup_after_attempt_like_cpp(
            &runtime, 1, result
        ));
        assert_eq!(
            run_world_session_shutdown_finalize_step_like_cpp(
                &runtime,
                Duration::from_secs(1),
                async { "cleanup attempted" },
            )
            .await,
            Some("cleanup attempted")
        );
        assert_eq!(runtime.get_exit_code_like_cpp(), ERROR_EXIT_CODE_LIKE_CPP);
    }
}
