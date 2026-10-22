//! Session-facing effects for the finalization application coordinator.
//! No transaction grouping or scheduling policy is defined by this adapter.
//! (#1363 slice 13b; moved from the World `session/lifecycle/finalization.rs`.)
use wow_core::ObjectGuid;
use wow_packet::packets::misc::LogoutComplete;
use wow_world_core::session::SessionState;
use wow_world_entities::PlayerTeleportHostLikeCpp;
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

/// C++ `WorldSession::Update` (`WorldSession.cpp:505-511`): when the logout
/// timer is due, `LogoutPlayer(true)` — the `TimedLogout` finalization, back to
/// character selection.
pub async fn run_logout_timer_with_generator_like_cpp<H: SessionLifecycleHostLikeCpp>(
    host: &mut H,
    item_guid_generator: &wow_core::ObjectGuidGenerator,
) {
    let due = {
        let (state, mut hub) = host.split_lifecycle_mut();
        state.run_logout_timer_like_cpp(&mut hub)
    };
    if due {
        let _ = finalize_session_with_generator_like_cpp(
            host,
            FinalizationMode::TimedLogout,
            item_guid_generator,
        )
        .await;
    }
}

pub async fn finalize_session_with_generator_like_cpp<H: SessionLifecycleHostLikeCpp>(
    host: &mut H,
    mode: FinalizationMode,
    item_guid_generator: &wow_core::ObjectGuidGenerator,
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
        let outcome = execute_finalization_step(host, step, mode, item_guid_generator).await;
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
    use FinalizationStep::*;
    match step {
        NativeTransfer => {
            if !wow_world_character::finish_worldport_native_before_disconnect_like_cpp(host) {
                return FinalizationOutcome::Unavailable;
            }
            if mode.returns_to_character_selection_like_cpp()
                && host.state() == SessionState::Disconnecting
            {
                return FinalizationOutcome::Deferred;
            }
            if host.player_guid().is_some() {
                host.lifecycle_mut().set_player_logout_like_cpp(true);
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
