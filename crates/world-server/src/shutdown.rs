//! Cooperative world-session and network shutdown orchestration.

use super::*;

/// Summary returned by [`kick_all_sessions_like_cpp`].
#[derive(Debug, Default, PartialEq, Eq)]
pub(super) struct KickAllSessionsSummaryLikeCpp {
    /// Active player-session registry entries evaluated.
    pub sessions_seen: usize,
    /// `KickLikeCpp` commands successfully enqueued.
    pub queued: usize,
    /// `try_send` calls that failed because the channel was full or closed.
    pub send_failed: usize,
}

/// Summary returned by [`update_sessions_shutdown_flush_once_like_cpp`].
#[derive(Debug, Default, PartialEq, Eq)]
pub(super) struct UpdateSessionsShutdownFlushSummaryLikeCpp {
    /// Active session registry entries evaluated.
    pub sessions_seen: usize,
    /// Shutdown flush commands successfully enqueued.
    pub queued: usize,
    /// `try_send` calls that failed because the command channel was full/closed.
    pub send_failed: usize,
    /// Sessions that acknowledged the flush command before the timeout.
    pub acked: usize,
    /// Sessions whose response channel closed before an acknowledgement.
    pub ack_failed: usize,
    /// Sessions that accepted the command but did not respond in time.
    pub ack_timeout: usize,
    /// Acknowledged sessions already marked disconnecting after the flush.
    pub disconnecting: usize,
}

/// Summary returned by [`stop_world_network_like_cpp`].
#[derive(Debug, Default, PartialEq, Eq)]
pub(super) struct StopWorldNetworkSummaryLikeCpp {
    /// Listener tasks explicitly stopped.
    pub listeners: usize,
}

/// Queue a C++ `World::KickAll`-style kick for every active Rust session.
///
/// C++ anchor:
/// `/home/server/woltk-trinity-legacy/src/server/game/World/World.cpp:3075`
/// clears the queued-login list and calls `WorldSession::KickPlayer("World::KickAll")`
/// for every session in `m_sessions`. Rust does not yet have the full
/// `WorldSessionMgr::Update` owner or login queue, so this function covers the
/// authenticated active-session registry; the required final
/// `UpdateSessions(1)` shutdown flush remains tracked separately in
/// `docs/migration/worldserver.md`.
pub(super) fn kick_all_sessions_like_cpp(
    registry: &ActiveWorldSessionRegistryLikeCpp,
) -> KickAllSessionsSummaryLikeCpp {
    let mut summary = KickAllSessionsSummaryLikeCpp::default();

    for (session_id, session) in registry.snapshot_like_cpp() {
        summary.sessions_seen = summary.sessions_seen.saturating_add(1);
        let command = SessionCommand::KickLikeCpp(KickLikeCppCommand {
            reason: "World::KickAll".to_string(),
        });

        match session.command_tx.try_send(command) {
            Ok(()) => {
                summary.queued = summary.queued.saturating_add(1);
            }
            Err(error) => {
                summary.send_failed = summary.send_failed.saturating_add(1);
                warn!(
                    account = session.account_id,
                    session_id,
                    error = %error,
                    "Failed to queue World::KickAll-style shutdown kick"
                );
            }
        }
    }

    summary
}

/// Ask every active session task to observe earlier shutdown commands.
///
/// C++ anchor:
/// `/home/server/woltk-trinity-legacy/src/server/game/World/World.cpp:3394`
/// `World::UpdateSessions(diff)` owns the session map, ticks every session,
/// and removes sessions whose `WorldSession::Update` returns false. Rust does
/// not yet have that global owner. This function is an explicit bridge for the
/// shutdown path: after `KickAll`, queue a flush marker behind the kick and wait
/// for the task-owned session to acknowledge that it drained the command rail.
/// It does not claim the final C++ erase/delete semantics.
pub(super) async fn update_sessions_shutdown_flush_once_like_cpp(
    registry: &ActiveWorldSessionRegistryLikeCpp,
    diff_ms: u32,
    ack_timeout: Duration,
) -> UpdateSessionsShutdownFlushSummaryLikeCpp {
    let mut summary = UpdateSessionsShutdownFlushSummaryLikeCpp::default();
    let mut pending_acks = tokio::task::JoinSet::new();

    for (session_id, session) in registry.snapshot_like_cpp() {
        summary.sessions_seen = summary.sessions_seen.saturating_add(1);
        let (response_tx, response_rx) =
            flume::bounded::<WorldSessionShutdownFlushResultLikeCpp>(1);
        let command = SessionCommand::WorldSessionShutdownFlushLikeCpp(
            WorldSessionShutdownFlushLikeCppCommand {
                diff_ms,
                response_tx,
            },
        );

        match session.command_tx.try_send(command) {
            Ok(()) => {
                summary.queued = summary.queued.saturating_add(1);
                let account_id = session.account_id;
                pending_acks
                    .spawn(async move { (session_id, account_id, response_rx.recv_async().await) });
            }
            Err(error) => {
                summary.send_failed = summary.send_failed.saturating_add(1);
                warn!(
                    account = session.account_id,
                    session_id,
                    error = %error,
                    "Failed to queue World::UpdateSessions(1)-style shutdown flush"
                );
            }
        }
    }

    let wait_outcome = tokio::time::timeout(ack_timeout, async {
        while let Some(joined) = pending_acks.join_next().await {
            match joined {
                Ok((_session_id, _account_id, Ok(result))) => {
                    summary.acked = summary.acked.saturating_add(1);
                    if result.disconnecting {
                        summary.disconnecting = summary.disconnecting.saturating_add(1);
                    }
                }
                Ok((session_id, account_id, Err(error))) => {
                    summary.ack_failed = summary.ack_failed.saturating_add(1);
                    warn!(
                        account = account_id,
                        session_id,
                        error = %error,
                        "World::UpdateSessions(1)-style shutdown flush acknowledgement failed"
                    );
                }
                Err(error) => {
                    summary.ack_failed = summary.ack_failed.saturating_add(1);
                    warn!(
                        error = %error,
                        "World::UpdateSessions(1)-style shutdown acknowledgement task failed"
                    );
                }
            }
        }
    })
    .await;

    if wait_outcome.is_err() {
        let timed_out = pending_acks.len();
        summary.ack_timeout = summary.ack_timeout.saturating_add(timed_out);
        pending_acks.abort_all();
        warn!(
            timed_out,
            timeout_ms = ack_timeout.as_millis(),
            "Timed out at the shared World::UpdateSessions shutdown acknowledgement deadline"
        );
    }

    summary
}

/// Stop the realm and instance TCP accept loops like C++ `WorldSocketMgr::StopNetwork`.
///
/// C++ anchor:
/// `/home/server/woltk-trinity-legacy/src/server/worldserver/Main.cpp:393`
/// calls `sWorldSocketMgr.StopNetwork()` after `KickAll` and
/// `UpdateSessions(1)` but before `ClearOnlineAccounts()`. Rust listener loops
/// are Tokio tasks around `TcpListener::accept`; aborting their handles closes
/// the listeners and prevents new accepts during shutdown.
pub(super) fn stop_world_network_like_cpp<'a>(
    listeners: impl IntoIterator<Item = (&'a str, &'a AbortHandle)>,
) -> StopWorldNetworkSummaryLikeCpp {
    let mut summary = StopWorldNetworkSummaryLikeCpp::default();

    for (name, handle) in listeners {
        handle.abort();
        summary.listeners = summary.listeners.saturating_add(1);
        debug!(listener = name, "Stopped world network listener");
    }

    summary
}

#[cfg(unix)]
pub(super) async fn shutdown_signal() {
    use tokio::signal::unix::{SignalKind, signal};

    let mut terminate = signal(SignalKind::terminate()).expect("failed to install SIGTERM handler");

    tokio::select! {
        _ = tokio::signal::ctrl_c() => {}
        _ = terminate.recv() => {}
    }
}

#[cfg(not(unix))]
pub(super) async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
}
