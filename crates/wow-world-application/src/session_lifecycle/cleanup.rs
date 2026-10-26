// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Disconnect and logout cleanup, in the order it must happen.
//!
//! Publication is torn down before ownership: the session leaves the player
//! directory, tells nearby players their view changed, leaves the canonical
//! map and the object accessor, releases its character login claim and only
//! then drops inventory objects. Reordering these is observable — a session
//! still published while its map entry is gone is exactly the window that
//! produces stale broadcasts.
//! (#1363 slice 13b; moved from the World `session/lifecycle/cleanup.rs`.)

use wow_world_lifecycle::{FinalizationDisposition, FinalizationOutcome};

use super::SessionLifecycleHostLikeCpp;

pub(super) fn recovery_retirement_outcome<H: SessionLifecycleHostLikeCpp>(
    host: &H,
) -> Option<FinalizationOutcome> {
    let core = host.core();
    let Some(handle) = core.player_handle_like_cpp else {
        return core.player_guid().is_some().then_some(FinalizationOutcome::Unavailable);
    };
    if core.player_guid() != Some(handle.guid()) {
        return Some(FinalizationOutcome::Unavailable);
    }
    let Some(manager) = core.canonical_map_manager.as_ref() else {
        return Some(FinalizationOutcome::Unavailable);
    };
    let Ok(manager) = manager.lock() else {
        return Some(FinalizationOutcome::Unavailable);
    };
    match manager
        .with_player_like_cpp(handle, |player| {
            (player.guid() == handle.guid()).then_some(
                player
                    .resurrection_state_like_cpp()
                    .delayed_after_teleport
                    .is_some(),
            )
        })
        .flatten()
    {
        Some(true) => Some(FinalizationOutcome::Deferred),
        Some(false) => None,
        None => Some(FinalizationOutcome::Unavailable),
    }
}

pub fn cleanup_shared_runtime_state<H: SessionLifecycleHostLikeCpp>(
    host: &mut H,
) -> FinalizationOutcome {
    if let Some(outcome) = recovery_retirement_outcome(host) {
        return outcome;
    }
    if host
        .lifecycle_mut()
        .finalization()
        .is_some_and(|operation| {
            operation.report().disposition == FinalizationDisposition::Complete
        })
    {
        return FinalizationOutcome::NoWork;
    }
    if host
        .lifecycle_mut()
        .finalization()
        .is_some_and(|operation| {
            operation.report().disposition != FinalizationDisposition::Complete
        })
    {
        return FinalizationOutcome::Unavailable;
    }
    {
        let (state, hub) = host.split_lifecycle_ref();
        state.unregister_from_player_registry(hub)
    };
    wow_world_spell::notify_other_players_visibility_changed_like_cpp(host);
    let outcome = {
        let (s, mut h) = host.split_lifecycle_mut();
        s.unregister_canonical_player_from_map_like_cpp(&mut h)
    };
    if !matches!(
        outcome,
        FinalizationOutcome::Applied | FinalizationOutcome::NoWork
    ) {
        return outcome;
    }
    host.lifecycle_mut()
        .release_character_login_claim_like_cpp();
    {
        let (state, mut hub) = host.split_inventory_mut();
        state.clear_inventory_items_and_objects_like_cpp(&mut hub)
    };
    outcome
}
