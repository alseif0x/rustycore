//! Deliver map-selected visibility after releasing the canonical map guard.

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

#[cfg(test)]
mod tests;
