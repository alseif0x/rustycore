//! WorldSession::LogoutPlayer's death-timer release, before the final save.

use wow_map::{MapKey, PlayerHandle};
use wow_world_core::session::SessionState;
use wow_world_entities::{CorpseLifecycleCatalogsLikeCpp, ResurrectionOperationCatalogs};
use wow_world_lifecycle::{FinalizationMode, FinalizationOutcome};

use super::SessionLifecycleHostLikeCpp;

fn current_residence(
    host: &impl SessionLifecycleHostLikeCpp,
    handle: PlayerHandle,
) -> Option<(MapKey, u64)> {
    let core = host.core();
    if core.player_handle_like_cpp != Some(handle) || core.player_guid() != Some(handle.guid()) {
        return None;
    }
    let manager = core.canonical_map_manager.as_ref()?.lock().ok()?;
    let receipt = manager.player_active_residence_revision_like_cpp(handle)?;
    manager.with_player_like_cpp(handle, |player| {
        let world = player.unit().world();
        (player.guid() == handle.guid()
            && world.object().is_in_world()
            && world.has_current_map()
            && MapKey::new(world.map_id(), world.instance_id()) == receipt.0)
            .then_some(receipt)
    })?
}

fn has_unresolved_native_operation(host: &impl SessionLifecycleHostLikeCpp) -> bool {
    host.core()
        .with_owned_player_like_cpp(|player| {
            player.has_unresolved_native_death_operation()
                || player
                    .resurrection_state_like_cpp()
                    .delayed_after_teleport
                    .is_some()
        })
        .unwrap_or(true)
}

pub(super) async fn release_dead_player_before_logout_like_cpp<H: SessionLifecycleHostLikeCpp>(
    host: &mut H,
    mode: FinalizationMode,
    catalogs: Option<ResurrectionOperationCatalogs<'_>>,
) -> FinalizationOutcome {
    let Some(timer) = host.core().with_owned_player_like_cpp(|player| {
        player
            .resurrection_state_like_cpp()
            .death_timer_remaining_ms
    }) else {
        return FinalizationOutcome::Unavailable;
    };
    if has_unresolved_native_operation(host) {
        return FinalizationOutcome::Deferred;
    }
    if timer == 0 {
        return FinalizationOutcome::NoWork;
    }
    if host
        .hub_ref()
        .player_teleport_state_snapshot_like_cpp()
        .is_none_or(|teleport| {
            teleport.near_pending
                || teleport.far_pending
                || teleport.has_delayed
                || teleport.delayed.is_some()
                || teleport.post_add.is_some()
                || teleport.can_delay
        })
    {
        return FinalizationOutcome::Deferred;
    }
    let Some(catalogs) = catalogs else {
        return FinalizationOutcome::Unavailable;
    };
    let Some(graveyards) = catalogs.graveyards else {
        return FinalizationOutcome::Unavailable;
    };
    let Some(handle) = host.core().player_handle_like_cpp else {
        return FinalizationOutcome::Unavailable;
    };
    let Some(before_release) = current_residence(host, handle) else {
        return FinalizationOutcome::Unavailable;
    };
    if !host.player_logout_with_save_like_cpp() || !host.is_logging_out_for_repop() {
        return FinalizationOutcome::Unavailable;
    }
    // WorldSession.cpp:558-565: no client repop/countdown admission gates.
    host.stop_combat_for_logout_death_release_like_cpp();
    if !wow_world_entities::build_player_repop_like_cpp(
        host,
        CorpseLifecycleCatalogsLikeCpp {
            item_guid_generator: catalogs.item_guid_generator,
            creature_spawns: catalogs.creature_spawns,
            graveyards,
        },
    )
    .await
    {
        return FinalizationOutcome::Deferred;
    }
    if current_residence(host, handle) != Some(before_release)
        || has_unresolved_native_operation(host)
    {
        return FinalizationOutcome::Deferred;
    }
    wow_world_entities::repop_at_graveyard_like_cpp(
        host,
        CorpseLifecycleCatalogsLikeCpp {
            item_guid_generator: catalogs.item_guid_generator,
            creature_spawns: catalogs.creature_spawns,
            graveyards,
        },
    )
    .await;
    // The graveyard may intentionally admit a far transfer. Reuse logout's
    // existing native completion, never require a client ACK or the old map revision.
    if host.core().player_handle_like_cpp != Some(handle)
        || host.player_guid() != Some(handle.guid())
        || !wow_world_character::finish_worldport_native_before_disconnect_like_cpp(host, catalogs)
            .await
        || current_residence(host, handle).is_none()
        || has_unresolved_native_operation(host)
    {
        return FinalizationOutcome::Deferred;
    }
    if mode.returns_to_character_selection_like_cpp() && host.state() == SessionState::Disconnecting
    {
        return FinalizationOutcome::Deferred;
    }
    let complete = host
        .core()
        .with_owned_player_like_cpp(|player| {
            let teleport = player.teleport_state_like_cpp();
            player
                .resurrection_state_like_cpp()
                .death_timer_remaining_ms
                == 0
                && !teleport.has_delayed
                && teleport.delayed.is_none()
                && teleport.post_add.is_none()
                && (!teleport.far_pending
                    || teleport.recovery == wow_entities::PlayerTransferRecovery::Terminal)
        })
        .unwrap_or(false);
    if complete {
        FinalizationOutcome::Applied
    } else {
        FinalizationOutcome::Deferred
    }
}
