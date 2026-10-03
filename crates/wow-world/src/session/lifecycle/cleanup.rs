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

use super::super::WorldSession;
use crate::finalization::FinalizationOutcome;

impl WorldSession {
    pub fn cleanup_shared_runtime_state(&mut self) -> FinalizationOutcome {
        if self
            .lifecycle
            .finalization()
            .is_some_and(|operation| {
                operation.report().disposition == crate::FinalizationDisposition::Complete
            })
        {
            return FinalizationOutcome::NoWork;
        }
        if self
            .lifecycle
            .finalization()
            .is_some_and(|operation| {
                operation.report().disposition != crate::FinalizationDisposition::Complete
            })
        {
            return FinalizationOutcome::Unavailable;
        }
        self.unregister_from_player_registry();
        self.notify_other_players_visibility_changed_like_cpp();
        let outcome = {
            let (s, mut h) = crate::session::split_lifecycle_mut(self);
            s.unregister_canonical_player_from_map_like_cpp(&mut h)
        };
        if !matches!(
            outcome,
            FinalizationOutcome::Applied | FinalizationOutcome::NoWork
        ) {
            return outcome;
        }
        self.lifecycle.release_character_login_claim_like_cpp();
        self.clear_inventory_items_and_objects_like_cpp();
        outcome
    }

    #[cfg(test)]
    pub async fn cleanup_shared_runtime_state_on_disconnect_with_generator_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
    ) -> FinalizationOutcome {
        self.wait_for_active_loot_persistence_with_generator_like_cpp(item_guid_generator)
            .await;
        if let Some(player_guid) = self.player_guid()
            && self.loot.has_active_loot_views_like_cpp()
        {
            self.do_loot_release_all_like_cpp(player_guid).await;
        }
        self.cleanup_shared_runtime_state()
    }

    #[cfg(test)]
    pub async fn cleanup_shared_runtime_state_on_disconnect_like_cpp(&mut self) {
        let generators = self.id_generators_for_test_like_cpp();
        self.cleanup_shared_runtime_state_on_disconnect_with_generator_like_cpp(
            generators.item.as_ref(),
        )
        .await;
    }
    pub(crate) fn unregister_from_player_registry(&self) {
        let (state, hub) = crate::session::split_lifecycle_ref(self);
        state.unregister_from_player_registry(hub)
    }
}
