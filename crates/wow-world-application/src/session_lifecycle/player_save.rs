//! Retain transfer-blocked save intent on the checked Player, never on Session;
//! and the periodic `Player::SaveToDB` entry of the session update.
//! (#1363 slice 13b; moved from the World `session/lifecycle/persistence/deferred.rs`
//! and `session/persistence/save.rs`.)
use wow_world_core::session::SessionState;
use wow_world_lifecycle::PlayerSaveOutcomeLikeCpp;

use super::SessionLifecycleHostLikeCpp;

pub async fn resume_deferred_player_save_with_generator_like_cpp<H: SessionLifecycleHostLikeCpp>(
    host: &mut H,
    item_guid_generator: &wow_core::ObjectGuidGenerator,
) -> Option<PlayerSaveOutcomeLikeCpp> {
    if host
        .core()
        .player_handle_like_cpp
        .is_some_and(|handle| host.player_guid() != Some(handle.guid()))
    {
        return Some(PlayerSaveOutcomeLikeCpp::Unavailable);
    }
    match host
        .core()
        .with_owned_player_like_cpp(|player| player.has_deferred_player_save_like_cpp())
    {
        Some(false) => None,
        Some(true) => Some(
            wow_world_lifecycle::save_current_player_to_db_with_generator_like_cpp(
                host,
                item_guid_generator,
            )
            .await,
        ),
        None => {
            #[cfg(any(test, feature = "test-fixtures"))]
            if host.test_fixtures_ref_like_cpp().is_some()
                && host.core().player_handle_like_cpp.is_none()
            {
                return None;
            }
            Some(PlayerSaveOutcomeLikeCpp::Unavailable)
        }
    }
}

pub async fn process_pending_periodic_player_save_with_generator_like_cpp<
    H: SessionLifecycleHostLikeCpp,
>(
    host: &mut H,
    item_guid_generator: &wow_core::ObjectGuidGenerator,
) {
    if !host.lifecycle_mut().pending_periodic_player_save_like_cpp()
        || host.core().state != SessionState::LoggedIn
    {
        return;
    }
    if wow_world_entities::pending_teleport_save_destination_like_cpp(host).is_some() {
        return;
    }

    wow_world_lifecycle::save_current_player_to_db_with_generator_like_cpp(
        host,
        item_guid_generator,
    )
    .await;
}
