//! Adapter for the canonical map's deferred player visibility obligation.
//!
//! Readiness only marks the Player. The existing map relocation phase selects
//! the recipient; its Session publishes against the real client GUID ledger.

use super::{
    CreatureSpawnCatalogsLikeCpp, PLAYER_LOCAL_FLAG_OVERRIDE_TRANSPORT_SERVER_TIME_LIKE_CPP,
    SessionState, WorldSession,
};
use crate::session::mailbox::DestroyVisibleCreatureLikeCppCommand;
use wow_entities::ObjectNotifyFlags;
use wow_map::PlayerVisibilityRefreshIntentLikeCpp;

impl WorldSession {
    /// Apply one map-owned C++ `WorldObject::DestroyForNearbyPlayers` result
    /// after the map guard has been released. The command covers ordinary
    /// Creature/Pet GUIDs; the session's client-visible set is the final
    /// `HaveAtClient` authority and is mutated atomically with the destroy
    /// packet decision.
    pub(crate) fn handle_destroy_visible_creature_like_cpp_command_like_cpp(
        &mut self,
        command: DestroyVisibleCreatureLikeCppCommand,
    ) {
        if self.state() != SessionState::LoggedIn
            || self.is_disconnecting()
            || !command.creature_guid.is_creature_or_pet()
            || self.player_map_id_like_cpp() != command.map_id
        {
            return;
        }
        let Some(current_key) = self.current_canonical_player_map_key_like_cpp() else {
            return;
        };
        if current_key.instance_id != command.instance_id {
            return;
        }
        let Some(manager) = self.canonical_map_manager.as_ref() else {
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
            .client_visible_guids_like_cpp
            .remove(&command.creature_guid)
        {
            return;
        }
        self.send_packet(&wow_packet::packets::update::UpdateObject::destroy_objects(
            vec![command.creature_guid],
            command.map_id,
        ));
    }

    pub(crate) fn apply_move_init_active_mover_complete_like_cpp(&mut self, ticks: u32) {
        let transport_server_time =
            crate::session_rules::game_time_ms_like_cpp().saturating_sub(ticks) as i32;
        if self
            .mutate_active_player_update_state_like_cpp(|state| {
                state.active_local_flags |=
                    PLAYER_LOCAL_FLAG_OVERRIDE_TRANSPORT_SERVER_TIME_LIKE_CPP;
                state.active_transport_server_time = transport_server_time;
            })
            .is_none()
        {
            return;
        }
        self.sync_current_player_session_visibility_detection_like_cpp();
        // MovementHandler.cpp:808 -> Player::UpdateObjectVisibility(false).
        // No scanner or packet fanout here: ProcessRelocationNotifies owns the
        // active-grid/timer phase and consumes this mark after selection.
        self.with_owned_player_mut_like_cpp(|player| {
            let object = player.unit_mut().world_mut().object_mut();
            if object.is_in_world() {
                object.add_to_notify(ObjectNotifyFlags::VISIBILITY_CHANGED);
            }
        });
        self.send_active_player_transport_server_time_update_like_cpp();
    }

    pub(crate) async fn apply_deferred_player_visibility_refresh_like_cpp(
        &mut self,
        catalogs: &CreatureSpawnCatalogsLikeCpp,
        intent: PlayerVisibilityRefreshIntentLikeCpp,
    ) {
        if self.state() != SessionState::LoggedIn
            || self.is_disconnecting()
            || self.player_handle_like_cpp != Some(intent.handle())
        {
            return;
        }
        let current = self.canonical_map_manager.as_ref().is_some_and(|manager| {
            manager.lock().is_ok_and(|manager| {
                manager.player_visibility_refresh_intent_is_current_like_cpp(intent)
            })
        });
        if !current {
            return;
        }
        self.sync_represented_farsight_clear_from_canonical_like_cpp();
        let viewpoint = self
            .current_seer_guid_like_cpp()
            .filter(|guid| !guid.is_empty())
            .unwrap_or(intent.handle().guid());
        if viewpoint != intent.viewpoint_guid() {
            return;
        }
        // The canonical scanner branch does not suspend or load from the DB.
        // It re-evaluates seer/detection and diffs the actual client ledger;
        // an intent never carries stale candidate GUIDs or serialized bytes.
        // The manager guard above is released before any packet publication.
        self.force_update_visibility_with_catalogs_like_cpp(catalogs)
            .await;
    }
}

#[cfg(test)]
mod tests;
