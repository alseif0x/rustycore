//! Session-facing effects for the finalization application coordinator.
//! No transaction grouping or scheduling policy is defined by this adapter.
//! (#1363 slice 13b; moved from the World `session/lifecycle/finalization.rs`.)
use wow_core::ObjectGuid;
use wow_entities::{PlayerResurrectionOperation, PlayerResurrectionPhase};
use wow_packet::packets::misc::LogoutComplete;
use wow_world_core::session::SessionState;
use wow_world_entities::PlayerTeleportHostLikeCpp;
use wow_world_entities::ResurrectionOperationCatalogs;
use wow_world_lifecycle::{
    FinalizationDisposition, FinalizationMode, FinalizationOutcome, FinalizationReport,
    FinalizationStep, PlayerSaveOutcomeLikeCpp, SessionFinalization, SessionLifecycleState,
};
use wow_world_loot_ops::{
    LootSessionHostLikeCpp, wait_for_active_loot_persistence_with_generator_like_cpp,
};

use super::SessionLifecycleHostLikeCpp;

fn finalization_result<H: SessionLifecycleHostLikeCpp>(host: &mut H) -> FinalizationReport {
    let (state, mut hub) = host.split_lifecycle_mut();
    state.finalization_result(&mut hub)
}

/// Typed module-level adapter for admitting one finalization step (#1437).
///
/// The persistence inventory cannot type the generic host's `lifecycle_mut`
/// receiver: it is declared on `CharacterLoginHostLikeCpp`, which the bound
/// only reaches through the re-exported `WorldportHostLikeCpp` supertrait.
/// Naming the concrete owner here lets the inventory audit the call.
fn begin_finalization_step_like_cpp(
    lifecycle: &mut SessionLifecycleState,
    step: FinalizationStep,
) -> bool {
    lifecycle.finalization_mut().unwrap().begin(step)
}

/// A logout can cancel C++ `ResurrectUsingRequestData` before it has crossed
/// the near teleport ACK boundary. Only the canonical Player can prove that
/// this is the still-unstarted RequestData operation for the current near
/// transfer; every later or uncertain phase stays fenced.
fn discard_unstarted_near_resurrection_for_finalization<H: SessionLifecycleHostLikeCpp>(
    host: &H,
) -> bool {
    let Some(player_guid) = host.player_guid() else {
        return false;
    };
    let core = host.core();
    let Some(handle) = core.player_handle_like_cpp else {
        return false;
    };
    if handle.guid() != player_guid || core.player_guid() != Some(player_guid) {
        return false;
    }

    core.with_owned_player_mut_like_cpp(|player| {
        if player.guid() != player_guid || !player.teleport_state_like_cpp().near_pending {
            return false;
        }
        let state = player.resurrection_state_mut_like_cpp();
        let unstarted_request = state.delayed_after_teleport.is_some_and(|continuation| {
            continuation.phase == PlayerResurrectionPhase::AwaitingTeleport
                && matches!(
                    continuation.operation,
                    PlayerResurrectionOperation::RequestData(_)
                )
        });
        if unstarted_request {
            state.delayed_after_teleport = None;
        }
        unstarted_request
    })
    .unwrap_or(false)
}

/// C++ `WorldSession::Update` (`WorldSession.cpp:505-511`): when the logout
/// timer is due, `LogoutPlayer(true)` — the `TimedLogout` finalization, back to
/// character selection.
pub async fn run_logout_timer_with_catalogs<H: SessionLifecycleHostLikeCpp>(
    host: &mut H,
    catalogs: ResurrectionOperationCatalogs<'_>,
) {
    let due = {
        let (state, mut hub) = host.split_lifecycle_mut();
        state.run_logout_timer_like_cpp(&mut hub)
    };
    if due {
        let _ = finalize_session_with_resurrection_catalogs(
            host,
            FinalizationMode::TimedLogout,
            catalogs.item_guid_generator,
            catalogs,
        )
        .await;
    }
}

pub async fn finalize_session_with_generator_like_cpp<H: SessionLifecycleHostLikeCpp>(
    host: &mut H,
    mode: FinalizationMode,
    item_guid_generator: &wow_core::ObjectGuidGenerator,
) -> FinalizationReport {
    finalize_session_with_catalogs(host, mode, item_guid_generator, None).await
}

pub async fn finalize_session_with_resurrection_catalogs<H: SessionLifecycleHostLikeCpp>(
    host: &mut H,
    mode: FinalizationMode,
    item_guid_generator: &wow_core::ObjectGuidGenerator,
    resurrection_catalogs: wow_world_entities::ResurrectionOperationCatalogs<'_>,
) -> FinalizationReport {
    finalize_session_with_catalogs(host, mode, item_guid_generator, Some(resurrection_catalogs))
        .await
}

async fn finalize_session_with_catalogs<H: SessionLifecycleHostLikeCpp>(
    host: &mut H,
    mode: FinalizationMode,
    item_guid_generator: &wow_core::ObjectGuidGenerator,
    resurrection_catalogs: Option<wow_world_entities::ResurrectionOperationCatalogs<'_>>,
) -> FinalizationReport {
    let no_player = host.player_guid().is_none();
    if let Some(previous) = host.lifecycle_mut().finalization_mut() {
        let report = previous.report();
        if mode == FinalizationMode::Disconnect
            && report.mode.returns_to_character_selection_like_cpp()
            && report.outcome(FinalizationStep::NativeTransfer) == FinalizationOutcome::Deferred
            && report.outcome(FinalizationStep::CharacterSave) == FinalizationOutcome::NotAttempted
        {
            // Completed terminal native recovery hands its source save to
            // disconnect, before any explicit-logout persistence submission.
        } else if !(report.disposition == FinalizationDisposition::Complete
            && report.mode.returns_to_character_selection_like_cpp()
            && mode == FinalizationMode::Disconnect
            && no_player)
        {
            previous.interrupt();
            return finalization_result(host);
        }
    }
    let (lifecycle, hub) = host.split_lifecycle_mut();
    lifecycle.install_finalization(SessionFinalization::new(
        mode,
        !no_player,
        hub.core.player_handle_like_cpp,
    ));

    while let Some(step) = host.lifecycle_mut().finalization().unwrap().next_step() {
        if !begin_finalization_step_like_cpp(host.lifecycle_mut(), step) {
            return finalization_result(host);
        }
        if !finalization_identity_is_current(host) {
            host.lifecycle_mut()
                .finalization_mut()
                .unwrap()
                .finish(step, FinalizationOutcome::Unavailable);
            return finalization_result(host);
        }
        let outcome = execute_finalization_step_with_catalogs(
            host,
            step,
            mode,
            item_guid_generator,
            resurrection_catalogs,
        )
        .await;
        if !host
            .lifecycle_mut()
            .finalization_mut()
            .unwrap()
            .finish(step, outcome)
        {
            return finalization_result(host);
        }
    }
    host.lifecycle_mut().finalization_mut().unwrap().complete();
    finalization_result(host)
}

fn finalization_identity_is_current<H: SessionLifecycleHostLikeCpp>(host: &H) -> bool {
    let (state, hub) = host.split_lifecycle_ref();
    state.finalization_identity_is_current(hub)
}

pub async fn execute_finalization_step<H: SessionLifecycleHostLikeCpp>(
    host: &mut H,
    step: FinalizationStep,
    mode: FinalizationMode,
    item_guid_generator: &wow_core::ObjectGuidGenerator,
) -> FinalizationOutcome {
    execute_finalization_step_with_catalogs(host, step, mode, item_guid_generator, None).await
}

async fn execute_finalization_step_with_catalogs<H: SessionLifecycleHostLikeCpp>(
    host: &mut H,
    step: FinalizationStep,
    mode: FinalizationMode,
    item_guid_generator: &wow_core::ObjectGuidGenerator,
    resurrection_catalogs: Option<wow_world_entities::ResurrectionOperationCatalogs<'_>>,
) -> FinalizationOutcome {
    use FinalizationStep::*;
    // Player::setDeathState is synchronous in C++ (Player.cpp:1158–1200).
    // Its Rust continuation must finish before logout can mutate, save or
    // unpublish this incarnation. A cancelled continuation retains its owner.
    if host
        .core()
        .with_owned_player_like_cpp(|player| player.has_unresolved_native_death_operation())
        == Some(true)
    {
        return FinalizationOutcome::Deferred;
    }
    match step {
        NativeTransfer => {
            discard_unstarted_near_resurrection_for_finalization(host);
            let completed = match resurrection_catalogs {
                Some(catalogs) => {
                    wow_world_character::finish_worldport_native_before_disconnect_like_cpp(
                        host, catalogs,
                    )
                    .await
                }
                None => {
                    wow_world_character::finish_worldport_native_without_resurrection_catalogs(host)
                        .await
                }
            };
            if !completed {
                return FinalizationOutcome::Unavailable;
            }
            if mode.returns_to_character_selection_like_cpp()
                && host.state() == SessionState::Disconnecting
            {
                return FinalizationOutcome::Deferred;
            }
            if host.player_guid().is_some() {
                host.lifecycle_mut().set_player_logout_like_cpp(true);
                host.core().mark_player_logging_out_like_cpp(true);
                PlayerTeleportHostLikeCpp::sync_current_player_session_visibility_detection_like_cpp(
                    host,
                );
                host.lifecycle_mut().clear_logout_time();
            }
            FinalizationOutcome::Applied
        }
        LootSettlement => {
            wait_for_active_loot_persistence_with_generator_like_cpp(host, item_guid_generator)
                .await;
            if let Some(guid) = host.player_guid() {
                if LootSessionHostLikeCpp::loot(host).has_active_loot_views_like_cpp() {
                    host.do_loot_release_all_like_cpp(guid).await;
                }
            }
            FinalizationOutcome::Applied
        }
        Buyback => {
            host.session_account_save_cx_like_cpp()
                .clear_buyback_on_logout()
                .await
        }
        CharacterSave => {
            match wow_world_lifecycle::save_current_player_to_db_with_generator_like_cpp(
                host,
                item_guid_generator,
            )
            .await
            {
                PlayerSaveOutcomeLikeCpp::Applied => FinalizationOutcome::Applied,
                PlayerSaveOutcomeLikeCpp::Failed => FinalizationOutcome::DefinitelyRolledBack,
                PlayerSaveOutcomeLikeCpp::Quarantined => FinalizationOutcome::Unknown,
                PlayerSaveOutcomeLikeCpp::Unavailable => FinalizationOutcome::Unavailable,
                PlayerSaveOutcomeLikeCpp::Deferred => FinalizationOutcome::Deferred,
            }
        }
        Mounts => {
            let (state, mut hub) = host.split_lifecycle_mut();
            state.save_account_mounts_like_cpp(&mut hub).await
        }
        Toys => {
            let (state, mut hub) = host.split_lifecycle_mut();
            state.save_account_toys_like_cpp(&mut hub).await
        }
        Heirlooms => {
            let (state, mut hub) = host.split_lifecycle_mut();
            state.save_account_heirlooms_like_cpp(&mut hub).await
        }
        Appearances => {
            host.session_account_save_cx_like_cpp()
                .save_account_item_appearances_like_cpp()
                .await
        }
        Illusions => {
            host.session_account_save_cx_like_cpp()
                .save_account_transmog_illusions_like_cpp()
                .await
        }
        CharacterOffline => {
            let (state, mut hub) = host.split_lifecycle_mut();
            state.mark_character_offline(&mut hub).await
        }
        CharacterAccountOffline => {
            let (state, mut hub) = host.split_lifecycle_mut();
            state
                .mark_character_account_offline_like_cpp(&mut hub)
                .await
        }
        LoginAccountOffline => {
            let (state, mut hub) = host.split_lifecycle_mut();
            state
                .mark_login_account_offline_on_disconnect_like_cpp(&mut hub)
                .await
        }
        Retirement => {
            if let Some(outcome) = super::cleanup::recovery_retirement_outcome(host) {
                return outcome;
            }
            {
                let (state, hub) = host.split_lifecycle_ref();
                state.unregister_from_player_registry(hub)
            };
            // C++ `sScriptMgr->OnPlayerLogout(_player)` (`WorldSession.cpp:655`;
            // AzerothCore `:828`): after the final save and the social cleanup,
            // before the player leaves the map. The ledger admits `Retirement`
            // once, and only after `CharacterSave` and `CharacterOffline`
            // applied, so the hook fires exactly once per logout (#1444).
            if host.player_logout_hook_wanted_like_cpp()
                && let Some(guid) = host.player_guid()
            {
                let event = crate::PlayerLogoutHookEventLikeCpp {
                    guid,
                    account_id: host.core().account_id,
                    mode,
                };
                host.player_logout_hook_like_cpp(&event);
            }
            // C++ `WorldSession::LogoutPlayer` runs `CleanupsBeforeDelete`
            // (`WorldSession.cpp:664`, `TradeCancel(false)`) after the logout
            // hook and before `RemovePlayerFromMap`.
            host.cancel_trade_before_logout_like_cpp();
            wow_world_spell::notify_other_players_visibility_changed_like_cpp(host);
            {
                let (s, mut h) = host.split_lifecycle_mut();
                s.unregister_canonical_player_from_map_like_cpp(&mut h)
            }
        }
        LogoutPublication => {
            // Separate writers must finish the earlier instance response first.
            // A failed/cancelled fence cannot authorize completion publication.
            if !host
                .core()
                .wait_for_instance_send_before_realm_send_like_cpp()
                .await
            {
                return FinalizationOutcome::Unavailable;
            }
            // C++ Opcodes.cpp:1665 routes LogoutComplete to realm.
            // Saturation yields instead of blocking the executor thread.
            // Success proves channel acceptance, not client receipt.
            let bytes = wow_packet::ServerPacket::to_bytes(&LogoutComplete);
            match host.core().realm_route_tx().send_async(bytes).await {
                Ok(()) => FinalizationOutcome::Applied,
                Err(_) => FinalizationOutcome::Unavailable,
            }
        }
        Release => {
            if mode.returns_to_character_selection_like_cpp() {
                host.set_player_guid(None);
                host.lifecycle_mut()
                    .release_character_login_claim_like_cpp();
                {
                    let (s, mut h) = host.split_inventory_mut();
                    s.clear_all_inventory_runtime_like_cpp(&mut h)
                };
                let _ = host.clear_player_currencies_like_cpp();
                LootSessionHostLikeCpp::loot_mut(host).set_active_loot_guid(ObjectGuid::EMPTY);
                {
                    let (state, mut hub) = host.split_lifecycle_mut();
                    state.restore_realm_channels(&mut hub)
                };
                host.lifecycle_mut().set_player_logout_like_cpp(false);
                PlayerTeleportHostLikeCpp::sync_current_player_session_visibility_detection_like_cpp(
                    host,
                );
                host.set_state(SessionState::Authed);
            } else {
                host.lifecycle_mut()
                    .release_character_login_claim_like_cpp();
                {
                    let (state, mut hub) = host.split_inventory_mut();
                    state.clear_inventory_items_and_objects_like_cpp(&mut hub)
                };
            }
            FinalizationOutcome::Applied
        }
    }
}
