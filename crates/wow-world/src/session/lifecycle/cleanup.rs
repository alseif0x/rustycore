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

use tracing::{debug, warn};

use super::super::WorldSession;

impl WorldSession {
    pub(crate) fn unregister_canonical_player_from_map_like_cpp(&mut self) {
        let Some(guid) = self.player_guid() else {
            return;
        };
        let Some(manager) = self.canonical_map_manager.as_ref() else {
            return;
        };
        let Ok(mut manager) = manager.lock() else {
            return;
        };

        if let Some(handle) = self.player_handle_like_cpp.take() {
            if manager.retire_player_like_cpp(handle).is_none() {
                warn!(
                    "Failed to retire canonical Player {:?}: stale handle or missing owner value",
                    guid
                );
            }
            return;
        }

        let map_id = u32::from(self.player_map_id_like_cpp());

        let mut instance_id = None;
        manager.do_for_all_maps_with_map_id(map_id, |managed| {
            if instance_id.is_none() && managed.map().get_typed_player(guid).is_some() {
                instance_id = Some(managed.instance_id());
            }
        });

        let Some(instance_id) = instance_id else {
            return;
        };
        let Some(managed) = manager.find_map_mut(map_id, instance_id) else {
            return;
        };

        if let Err(err) = managed.map_mut().remove_from_map_like_cpp(guid, true) {
            match err {
                wow_map::RemoveFromMapError::ObjectNotFound { .. } => {
                    debug!("Canonical Player {:?} already removed from map", guid);
                }
                wow_map::RemoveFromMapError::ResetMap(reset_err) => {
                    warn!(
                        "Failed to remove canonical Player {:?} from map: {reset_err:?}",
                        guid
                    );
                }
            }
        }
    }

    pub fn cleanup_shared_runtime_state(&mut self) {
        self.unregister_from_player_registry();
        self.notify_other_players_visibility_changed_like_cpp();
        self.unregister_canonical_player_from_map_like_cpp();
        self.release_character_login_claim_like_cpp();
        self.clear_inventory_items_and_objects_like_cpp();
    }

    pub async fn cleanup_shared_runtime_state_on_disconnect_with_generator_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
    ) {
        self.wait_for_active_loot_persistence_with_generator_like_cpp(item_guid_generator)
            .await;
        if let Some(player_guid) = self.player_guid()
            && self.has_active_loot_views_like_cpp()
        {
            self.do_loot_release_all_like_cpp(player_guid).await;
        }
        self.cleanup_shared_runtime_state();
    }

    #[cfg(test)]
    pub async fn cleanup_shared_runtime_state_on_disconnect_like_cpp(&mut self) {
        let generators = self.id_generators_for_test_like_cpp();
        self.cleanup_shared_runtime_state_on_disconnect_with_generator_like_cpp(
            generators.item.as_ref(),
        )
        .await;
    }
    /// Remove this session from the player registry.
    /// Called on logout or disconnect.
    pub(crate) fn unregister_from_player_registry(&self) {
        let (Some(guid), Some(reg)) = (self.player_guid(), &self.player_registry) else {
            return;
        };
        if reg.unregister_control_channel(guid, &self.session_command_tx) {
            debug!("Unregistered player {:?} from broadcast registry", guid);
        }
    }
}
