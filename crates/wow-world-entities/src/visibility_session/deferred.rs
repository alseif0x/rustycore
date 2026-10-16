// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! The deferred visibility mailbox steps (#1363 slice 12d; moved from the
//! World `session/deferred_visibility.rs` and `session/visibility/operations.rs`):
//! C++ `WorldObject::DestroyForNearbyPlayers` applied after the map guard is
//! released, the map-owned deferred player visibility refresh and the pending
//! refresh flush. Each runs against the session's real client GUID ledger.

use std::sync::atomic::Ordering;

use wow_map::PlayerVisibilityRefreshIntentLikeCpp;
use wow_world_core::session::SessionState;
use wow_world_core::session::mailbox::DestroyVisibleObjectLikeCppCommand;

use super::VisibilitySessionHostLikeCpp;
use crate::CreatureSpawnCatalogsLikeCpp;

/// Apply one map-owned C++ `WorldObject::DestroyForNearbyPlayers` result
/// after the map guard has been released. The command covers ordinary
/// map-owned object GUIDs; the session's client-visible set is the final
/// `HaveAtClient` authority and is mutated atomically with the destroy
/// packet decision.
pub fn handle_destroy_visible_object_like_cpp_command_like_cpp<H: VisibilitySessionHostLikeCpp>(
    host: &mut H,
    command: DestroyVisibleObjectLikeCppCommand,
) {
    if host.state() != SessionState::LoggedIn
        || host.core().state == SessionState::Disconnecting
        || (!command.object_guid.is_creature_or_pet() && !command.object_guid.is_corpse())
        || host.core().player_map_id_like_cpp() != command.map_id
    {
        return;
    }
    let Some(current_key) = host.core().current_canonical_player_map_key_like_cpp() else {
        return;
    };
    if current_key.instance_id != command.instance_id {
        return;
    }
    let Some(manager) = host.core().canonical_map_manager.as_ref() else {
        return;
    };
    let current_map = manager.lock().ok().is_some_and(|manager| {
        manager.map_incarnation_like_cpp(wow_map::MapKey::new(
            u32::from(command.map_id),
            command.instance_id,
        )) == Some(command.map_incarnation)
    });
    if !current_map {
        return;
    }
    if !host
        .core()
        .client_visible_guids_like_cpp
        .remove(&command.object_guid)
    {
        return;
    }
    host.send_packet(&wow_packet::packets::update::UpdateObject::destroy_objects(
        vec![command.object_guid],
        command.map_id,
    ));
}

pub async fn apply_deferred_player_visibility_refresh_like_cpp<H: VisibilitySessionHostLikeCpp>(
    host: &mut H,
    catalogs: &CreatureSpawnCatalogsLikeCpp,
    intent: PlayerVisibilityRefreshIntentLikeCpp,
) {
    if host.state() != SessionState::LoggedIn
        || host.core().state == SessionState::Disconnecting
        || host.core().player_handle_like_cpp != Some(intent.handle())
    {
        return;
    }
    let current = host
        .core()
        .canonical_map_manager
        .as_ref()
        .is_some_and(|manager| {
            manager.lock().is_ok_and(|manager| {
                manager.player_visibility_refresh_intent_is_current_like_cpp(intent)
            })
        });
    if !current {
        return;
    }
    host.sync_represented_farsight_clear_from_canonical_like_cpp();
    let viewpoint = crate::current_seer_guid_like_cpp(host)
        .filter(|guid| !guid.is_empty())
        .unwrap_or(intent.handle().guid());
    if viewpoint != intent.viewpoint_guid() {
        return;
    }
    // The canonical scanner branch does not suspend or load from the DB.
    // It re-evaluates seer/detection and diffs the actual client ledger;
    // an intent never carries stale candidate GUIDs or serialized bytes.
    // The manager guard above is released before any packet publication.
    crate::force_update_visibility_with_catalogs_like_cpp(host, catalogs).await;
}

pub async fn flush_pending_visibility_refresh_with_catalogs_like_cpp<
    H: VisibilitySessionHostLikeCpp,
>(
    host: &mut H,
    creature_spawn_catalogs: &CreatureSpawnCatalogsLikeCpp,
) {
    if host.state() != SessionState::LoggedIn {
        return;
    }
    if host
        .core()
        .flags
        .visibility_refresh_pending_like_cpp
        .swap(false, Ordering::AcqRel)
    {
        crate::force_update_visibility_with_catalogs_like_cpp(host, creature_spawn_catalogs).await;
    }
}
