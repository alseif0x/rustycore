//! Supervision and ordered shutdown of the running world server.

use anyhow::Result;
use std::process::ExitCode;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::task::{AbortHandle, JoinHandle};
use tracing::info;
use crate::runtime::map::{CanonicalMapProducerExit, stop_canonical_map_producer};

use crate::{
    clear_online_accounts_like_cpp, drain_respawn_db_writer_like_cpp,
    kick_all_sessions_like_cpp, process_exit_code_like_cpp, set_realm_offline,
    shutdown_signal, stop_respawn_db_producer_like_cpp, stop_world_network_like_cpp,
    update_sessions_shutdown_flush_once_like_cpp, ActiveWorldSessionRegistryLikeCpp,
    RespawnDbWriterSenderLikeCpp, WorldRuntimeStateLikeCpp, ERROR_EXIT_CODE_LIKE_CPP,
    RESPAWN_DB_PRODUCER_STOP_TIMEOUT,
    SHUTDOWN_EXIT_CODE_LIKE_CPP, WORLD_SESSION_FORCE_CANCEL_TIMEOUT_LIKE_CPP,
    WORLD_SESSION_SHUTDOWN_DRAIN_TIMEOUT_LIKE_CPP,
    WORLD_SESSION_SHUTDOWN_FLUSH_TIMEOUT_LIKE_CPP,
};

pub(super) struct RuntimeSupervisionInputs<'a> {
    pub(super) realm_handle: &'a mut JoinHandle<Result<()>>,
    pub(super) instance_handle: &'a mut JoinHandle<Result<()>>,
    pub(super) map_update_handle: &'a mut JoinHandle<CanonicalMapProducerExit>,
    pub(super) legacy_creature_runtime_handle: &'a mut JoinHandle<()>,
    pub(super) ready_check_tick_handle: &'a mut JoinHandle<()>,
    pub(super) respawn_db_writer_handle: &'a mut JoinHandle<()>,
    pub(super) realm_network_abort_handle: &'a AbortHandle,
    pub(super) instance_network_abort_handle: &'a AbortHandle,
    pub(super) world_runtime_state: &'a WorldRuntimeStateLikeCpp,
    pub(super) active_session_registry: &'a ActiveWorldSessionRegistryLikeCpp,
    pub(super) battle_pet_account_registry: &'a wow_world::BattlePetAccountRegistryLikeCpp,
    pub(super) respawn_db_producer_stop: &'a AtomicBool,
    pub(super) respawn_db_writer_tx: RespawnDbWriterSenderLikeCpp,
    pub(super) item_guid_allocator_advisory_lock: wow_database::ItemGuidAllocatorAdvisoryLockLikeCpp,
    pub(super) game_event_quest_complete_handle: &'a JoinHandle<()>,
    pub(super) db_keepalive_handle: &'a Option<JoinHandle<()>>,
    pub(super) realm_list_update_handle: &'a Option<JoinHandle<()>>,
    pub(super) login_db: &'a wow_database::LoginDatabase,
    pub(super) char_db: &'a wow_database::CharacterDatabase,
    pub(super) realm_id: u16,
}

pub(super) async fn supervise_and_shutdown(
    inputs: RuntimeSupervisionInputs<'_>,
) -> Result<ExitCode> {
    let RuntimeSupervisionInputs {
        realm_handle,
        instance_handle,
        map_update_handle,
        legacy_creature_runtime_handle,
        ready_check_tick_handle,
        respawn_db_writer_handle,
        realm_network_abort_handle,
        instance_network_abort_handle,
        world_runtime_state,
        active_session_registry,
        battle_pet_account_registry,
        respawn_db_producer_stop,
        respawn_db_writer_tx,
        mut item_guid_allocator_advisory_lock,
        game_event_quest_complete_handle,
        db_keepalive_handle,
        realm_list_update_handle,
        login_db,
        char_db,
        realm_id,
    } = inputs;

    let startup_script_summary = wow_script::lifecycle::on_startup_like_cpp();
    info!(
        callbacks = startup_script_summary.callbacks,
        "Ran ScriptMgr::OnStartup-style lifecycle hooks"
    );

    let mut map_update_finished = false;
    // Keep any returned failure and its same admission through this shutdown
    // scope. Scope-end drop is not settlement and cannot authorize a receipt.
    let mut map_update_exit: Option<CanonicalMapProducerExit> = None;
    let mut legacy_creature_runtime_finished = false;
    let mut respawn_db_writer_finished = false;

    // Wait for shutdown signal or a supervised background task failure.
    tokio::select! {
        _ = shutdown_signal() => {
            world_runtime_state.stop_now_like_cpp(SHUTDOWN_EXIT_CODE_LIKE_CPP);
            info!("Shutdown signal received, stopping...");
        }
        result = &mut *realm_handle => {
            match result {
                Ok(Ok(())) => {
                    world_runtime_state.stop_now_like_cpp(ERROR_EXIT_CODE_LIKE_CPP);
                    tracing::error!("Realm listener stopped unexpectedly");
                }
                Ok(Err(e)) => {
                    world_runtime_state.stop_now_like_cpp(ERROR_EXIT_CODE_LIKE_CPP);
                    tracing::error!("{e:#}");
                }
                Err(e) => {
                    world_runtime_state.stop_now_like_cpp(ERROR_EXIT_CODE_LIKE_CPP);
                    tracing::error!("Realm listener task failed: {e}");
                }
            }
        }
        result = &mut *instance_handle => {
            match result {
                Ok(Ok(())) => {
                    world_runtime_state.stop_now_like_cpp(ERROR_EXIT_CODE_LIKE_CPP);
                    tracing::error!("Instance listener stopped unexpectedly");
                }
                Ok(Err(e)) => {
                    world_runtime_state.stop_now_like_cpp(ERROR_EXIT_CODE_LIKE_CPP);
                    tracing::error!("{e:#}");
                }
                Err(e) => {
                    world_runtime_state.stop_now_like_cpp(ERROR_EXIT_CODE_LIKE_CPP);
                    tracing::error!("Instance listener task failed: {e}");
                }
            }
        }
        result = &mut *map_update_handle => {
            map_update_finished = true;
            match result {
                Ok(exit) => {
                    map_update_exit = Some(exit);
                    world_runtime_state.stop_now_like_cpp(ERROR_EXIT_CODE_LIKE_CPP);
                    tracing::error!("Map update task stopped unexpectedly");
                }
                Err(e) => {
                    world_runtime_state.stop_now_like_cpp(ERROR_EXIT_CODE_LIKE_CPP);
                    tracing::error!("Map update task failed: {e}");
                }
            }
        }
        result = &mut *legacy_creature_runtime_handle => {
            legacy_creature_runtime_finished = true;
            match result {
                Ok(()) => {
                    world_runtime_state.stop_now_like_cpp(ERROR_EXIT_CODE_LIKE_CPP);
                    tracing::error!("Legacy creature runtime task stopped unexpectedly");
                }
                Err(e) => {
                    world_runtime_state.stop_now_like_cpp(ERROR_EXIT_CODE_LIKE_CPP);
                    tracing::error!("Legacy creature runtime task failed: {e}");
                }
            }
        }
        result = &mut *ready_check_tick_handle => {
            match result {
                Ok(()) => {
                    world_runtime_state.stop_now_like_cpp(ERROR_EXIT_CODE_LIKE_CPP);
                    tracing::error!("Ready-check tick task stopped unexpectedly");
                }
                Err(e) => {
                    world_runtime_state.stop_now_like_cpp(ERROR_EXIT_CODE_LIKE_CPP);
                    tracing::error!("Ready-check tick task failed: {e}");
                }
            }
        }
        result = &mut *respawn_db_writer_handle => {
            respawn_db_writer_finished = true;
            world_runtime_state.stop_now_like_cpp(ERROR_EXIT_CODE_LIKE_CPP);
            match result {
                Ok(()) => {
                    tracing::error!("Shared respawn DB writer stopped unexpectedly");
                }
                Err(e) => {
                    tracing::error!("Shared respawn DB writer task failed: {e}");
                }
            }
        }
        result = item_guid_allocator_advisory_lock.wait_until_lost_like_cpp() => {
            world_runtime_state.stop_now_like_cpp(ERROR_EXIT_CODE_LIKE_CPP);
            match result {
                Ok(()) => tracing::error!(
                    "Item GUID allocator advisory-lock monitor stopped unexpectedly"
                ),
                Err(error) => tracing::error!(
                    %error,
                    "Item GUID allocator advisory lock was lost; stopping before another GUID allocation"
                ),
            }
        }
    }

    // Close registration under the same mutex used by `try_register`. An
    // in-flight authenticated connection is therefore either already in the
    // KickAll snapshot or rejected, while C++ KickAll -> UpdateSessions ->
    // StopNetwork ordering remains intact.
    active_session_registry.begin_shutdown_like_cpp();
    let request = active_session_registry.close_tick_admission();
    let quiescence = active_session_registry.wait_for_quiescence(
        request, RESPAWN_DB_PRODUCER_STOP_TIMEOUT,
    ).await.and_then(|receipt| active_session_registry.enable_session_drain(receipt));
    if let Err(failure) = quiescence {
        world_runtime_state.stop_now_like_cpp(ERROR_EXIT_CODE_LIKE_CPP);
        tracing::error!(?failure, "Producer quiescence unproven; shutdown controls cannot authorize session effects");
    }
    // Control/network attempts retain their C++ order even on failure. Only
    // the real receipt above lets session owners execute the queued controls.
    let kick_summary = kick_all_sessions_like_cpp(active_session_registry);
    info!(
        sessions_seen = kick_summary.sessions_seen,
        queued = kick_summary.queued,
        failed = kick_summary.send_failed,
        "Queued World::KickAll-style shutdown kicks"
    );
    if kick_summary.send_failed > 0 {
        world_runtime_state.stop_now_like_cpp(ERROR_EXIT_CODE_LIKE_CPP);
        tracing::error!(
            failed = kick_summary.send_failed,
            "World::KickAll shutdown delivery failed; forcing terminal error status"
        );
    }
    let flush_summary = update_sessions_shutdown_flush_once_like_cpp(
        active_session_registry,
        1,
        WORLD_SESSION_SHUTDOWN_FLUSH_TIMEOUT_LIKE_CPP,
    )
    .await;
    info!(
        sessions_seen = flush_summary.sessions_seen,
        queued = flush_summary.queued,
        failed = flush_summary.send_failed,
        acked = flush_summary.acked,
        ack_failed = flush_summary.ack_failed,
        ack_timeout = flush_summary.ack_timeout,
        disconnecting = flush_summary.disconnecting,
        "Ran World::UpdateSessions(1)-style shutdown flush"
    );
    if flush_summary.send_failed > 0
        || flush_summary.ack_failed > 0
        || flush_summary.ack_timeout > 0
    {
        world_runtime_state.stop_now_like_cpp(ERROR_EXIT_CODE_LIKE_CPP);
        tracing::error!(
            send_failed = flush_summary.send_failed,
            ack_failed = flush_summary.ack_failed,
            ack_timeout = flush_summary.ack_timeout,
            "World::UpdateSessions shutdown flush was incomplete; forcing terminal error status"
        );
    }
    let network_stop_summary = stop_world_network_like_cpp([
        ("realm", realm_network_abort_handle),
        ("instance", instance_network_abort_handle),
    ]);
    info!(
        listeners = network_stop_summary.listeners,
        "Stopped world network listeners like C++ WorldSocketMgr::StopNetwork"
    );
    // Any session that could not receive/ack the explicit commands now sees
    // this cooperative stop at its next update boundary. Registration has
    // already been closed, so the registry can only drain from this point.
    active_session_registry.request_session_stop_like_cpp();
    let sessions_drained = active_session_registry
        .wait_until_empty_like_cpp(WORLD_SESSION_SHUTDOWN_DRAIN_TIMEOUT_LIKE_CPP)
        .await;
    info!(
        drained = sessions_drained,
        remaining = active_session_registry.len_like_cpp(),
        "Waited for task-owned sessions to unregister after shutdown flush"
    );
    if !sessions_drained {
        world_runtime_state.stop_now_like_cpp(ERROR_EXIT_CODE_LIKE_CPP);
        let cancelled = active_session_registry.cancel_all_sessions_like_cpp();
        tracing::error!(
            remaining = active_session_registry.len_like_cpp(),
            cancelled,
            "World-session shutdown grace period expired; force-cancelling registered session futures"
        );
        let forced_sessions_drained = active_session_registry
            .wait_until_empty_like_cpp(WORLD_SESSION_FORCE_CANCEL_TIMEOUT_LIKE_CPP)
            .await;
        if !forced_sessions_drained {
            tracing::error!(
                remaining = active_session_registry.len_like_cpp(),
                timeout_ms = WORLD_SESSION_FORCE_CANCEL_TIMEOUT_LIKE_CPP.as_millis(),
                "Force-cancelled world sessions did not unregister before terminal timeout"
            );
        }
    }

    let battle_pet_operations_drained = battle_pet_account_registry
        .drain_like_cpp(WORLD_SESSION_SHUTDOWN_DRAIN_TIMEOUT_LIKE_CPP)
        .await;
    info!(
        drained = battle_pet_operations_drained,
        "Waited for cancellation-safe battle-pet persistence workers"
    );
    if !battle_pet_operations_drained {
        world_runtime_state.stop_now_like_cpp(ERROR_EXIT_CODE_LIKE_CPP);
        tracing::error!(
            timeout_ms = WORLD_SESSION_SHUTDOWN_DRAIN_TIMEOUT_LIKE_CPP.as_millis(),
            "Battle-pet persistence workers did not drain before the shutdown deadline"
        );
    }

    // A final simulation tick is an explicit new admission, never a bypass
    // around a retained writer or an unresolved producer obligation.
    let request = active_session_registry.close_tick_admission();
    let final_tick_admission = active_session_registry.wait_for_terminal_settlement(
        request, RESPAWN_DB_PRODUCER_STOP_TIMEOUT,
    ).await.and_then(|receipt| active_session_registry.authorize_final_respawn_tick(receipt));
    if let Err(failure) = final_tick_admission {
        world_runtime_state.stop_now_like_cpp(ERROR_EXIT_CODE_LIKE_CPP);
        tracing::error!(?failure, "Final respawn tick refused; retained owners cannot authorize simulation");
    }
    respawn_db_producer_stop.store(true, Ordering::Release);
    let (map_update_stopped, legacy_runtime_stopped) = tokio::join!(
        stop_canonical_map_producer(
            &mut *map_update_handle,
            map_update_finished,
            &mut map_update_exit,
        ),
        stop_respawn_db_producer_like_cpp(
            "legacy-creature-runtime",
            &mut *legacy_creature_runtime_handle,
            legacy_creature_runtime_finished,
        ),
    );
    if !map_update_stopped || !legacy_runtime_stopped {
        world_runtime_state.stop_now_like_cpp(ERROR_EXIT_CODE_LIKE_CPP);
    }

    // Cut off any final admission racing a failed/aborted task handle, then
    // account for tickets still owned inside spawn_blocking. A JoinHandle
    // timeout or abort is never a receipt for that closure.
    active_session_registry.close_final_tick_admission();
    let request = active_session_registry.close_tick_admission();
    let producers_settled = active_session_registry.wait_for_terminal_settlement(
        request, RESPAWN_DB_PRODUCER_STOP_TIMEOUT,
    ).await;
    if producers_settled.is_ok() {
        // The mutation-to-submit fence is unchanged. Closing now makes the
        // writer's existing backoff due and drains retained spawn operations.
        respawn_db_writer_tx.close_like_cpp();
        drop(respawn_db_writer_tx);
        if !drain_respawn_db_writer_like_cpp(&mut *respawn_db_writer_handle, respawn_db_writer_finished)
            .await
        {
            world_runtime_state.stop_now_like_cpp(ERROR_EXIT_CODE_LIKE_CPP);
        }
    } else {
        world_runtime_state.stop_now_like_cpp(ERROR_EXIT_CODE_LIKE_CPP);
        tracing::error!(?producers_settled, "Respawn producers retain unproven effects; refusing mailbox completion/drain claim");
    }

    game_event_quest_complete_handle.abort();
    if let Some(db_keepalive_handle) = db_keepalive_handle {
        db_keepalive_handle.abort();
    }
    if let Some(realm_list_update_handle) = realm_list_update_handle {
        realm_list_update_handle.abort();
    }

    if let Err(e) = clear_online_accounts_like_cpp(login_db, char_db, realm_id).await {
        tracing::error!("Failed to clear online account state for realm {realm_id}: {e}");
    }

    let shutdown_script_summary = wow_script::lifecycle::on_shutdown_like_cpp();
    info!(
        callbacks = shutdown_script_summary.callbacks,
        "Ran ScriptMgr::OnShutdown-style lifecycle hooks"
    );

    if let Err(e) = set_realm_offline(login_db, realm_id).await {
        tracing::error!("Failed to mark realm {realm_id} offline: {e}");
    }

    if let Err(error) = item_guid_allocator_advisory_lock.release_like_cpp().await {
        world_runtime_state.stop_now_like_cpp(ERROR_EXIT_CODE_LIKE_CPP);
        tracing::error!(%error, "Failed to release item GUID allocator advisory lock");
    }
    info!(
        exit_code = world_runtime_state.get_exit_code_like_cpp(),
        "World server stopped."
    );
    Ok(process_exit_code_like_cpp(
        world_runtime_state.get_exit_code_like_cpp(),
    ))
}
