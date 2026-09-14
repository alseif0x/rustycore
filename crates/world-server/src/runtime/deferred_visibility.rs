//! Deliver map-selected visibility after releasing the canonical map guard.

use super::tick_summary::CanonicalObjectVisibilityDestroyLikeCpp;

pub(super) fn deliver_deferred_player_visibility_like_cpp(
    intents: &[wow_map::PlayerVisibilityRefreshIntentLikeCpp],
    registry: &wow_world::session::directory::PlayerRegistry,
) {
    for intent in intents {
        // Stale residences and disconnected recipients are intentionally
        // discarded; a full bounded command queue cannot discard this work.
        registry.request_deferred_player_visibility_refresh_like_cpp(*intent);
    }
}

/// Publish directed object destroys selected by `Map::RemoveFromMap` after
/// all map guards are released. The directory resolves each recipient's
/// current registration; the session performs the final map-incarnation and
/// `HaveAtClient` checks before sending the packet.
pub(super) fn deliver_directed_object_destroy_like_cpp(
    destroys: &[CanonicalObjectVisibilityDestroyLikeCpp],
    registry: &wow_world::session::directory::PlayerRegistry,
) {
    for destroy in destroys {
        let Ok(map_id) = u16::try_from(destroy.map_id) else {
            continue;
        };
        for recipient_guid in &destroy.recipient_guids {
            let Some(recipient) = registry.runtime_recipient(*recipient_guid) else {
                continue;
            };
            if !recipient.is_in_world
                || recipient.map_id != map_id
                || recipient.instance_id != destroy.instance_id
            {
                continue;
            }
            let command = wow_world::session::mailbox::DestroyVisibleObjectLikeCppCommand {
                object_guid: destroy.object_guid,
                map_id,
                instance_id: destroy.instance_id,
                map_incarnation: destroy.map_incarnation,
            };
            let _ =
                registry.publish_current_destroy_visible_object(recipient.registration, command);
        }
    }
}

#[cfg(test)]
mod tests;
