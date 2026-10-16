//! Adapter for the canonical map's deferred player visibility obligation.
//!
//! Readiness only marks the Player. The existing map relocation phase selects
//! the recipient; its Session publishes against the real client GUID ledger.

use super::{CreatureSpawnCatalogsLikeCpp, SessionState, WorldSession};
use crate::session::mailbox::DestroyVisibleObjectLikeCppCommand;
use wow_map::PlayerVisibilityRefreshIntentLikeCpp;

impl WorldSession {
    /// Apply one map-owned C++ `WorldObject::DestroyForNearbyPlayers` result
    /// after the map guard has been released. The command covers ordinary
    /// map-owned object GUIDs; the session's client-visible set is the final
    /// `HaveAtClient` authority and is mutated atomically with the destroy
    /// packet decision.
    pub(crate) fn handle_destroy_visible_object_like_cpp_command_like_cpp(
        &mut self,
        command: DestroyVisibleObjectLikeCppCommand,
    ) {
        if self.state() != SessionState::LoggedIn
            || self.is_disconnecting()
            || (!command.object_guid.is_creature_or_pet() && !command.object_guid.is_corpse())
            || self.core.player_map_id_like_cpp() != command.map_id
        {
            return;
        }
        let Some(current_key) = self.core.current_canonical_player_map_key_like_cpp() else {
            return;
        };
        if current_key.instance_id != command.instance_id {
            return;
        }
        let Some(manager) = self.core.canonical_map_manager.as_ref() else {
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
        if !self
            .core
            .client_visible_guids_like_cpp
            .remove(&command.object_guid)
        {
            return;
        }
        self.send_packet(&wow_packet::packets::update::UpdateObject::destroy_objects(
            vec![command.object_guid],
            command.map_id,
        ));
    }

    pub(crate) async fn apply_deferred_player_visibility_refresh_like_cpp(
        &mut self,
        catalogs: &CreatureSpawnCatalogsLikeCpp,
        intent: PlayerVisibilityRefreshIntentLikeCpp,
    ) {
        if self.state() != SessionState::LoggedIn
            || self.is_disconnecting()
            || self.core.player_handle_like_cpp != Some(intent.handle())
        {
            return;
        }
        let current = self
            .core
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
        self.sync_represented_farsight_clear_from_canonical_like_cpp();
        let viewpoint = wow_world_entities::current_seer_guid_like_cpp(self)
            .filter(|guid| !guid.is_empty())
            .unwrap_or(intent.handle().guid());
        if viewpoint != intent.viewpoint_guid() {
            return;
        }
        // The canonical scanner branch does not suspend or load from the DB.
        // It re-evaluates seer/detection and diffs the actual client ledger;
        // an intent never carries stale candidate GUIDs or serialized bytes.
        // The manager guard above is released before any packet publication.
        wow_world_entities::force_update_visibility_with_catalogs_like_cpp(self, catalogs).await;
    }
}

#[cfg(test)]
#[path = "../../unit_tests/session/deferred_visibility/tests.rs"]
mod tests;
