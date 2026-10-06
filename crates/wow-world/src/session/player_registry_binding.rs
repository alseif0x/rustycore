// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Player registry binding: private Session responsibility.
//! Relocated under #1233; canonical state, phase order and public paths are unchanged.

use super::{Arc, PendingInvites, PlayerRegistry, PlayerSessionRegistrationLikeCpp};
use super::{WorldSession, debug};

impl WorldSession {
    /// Set the shared player registry (used for broadcast).
    pub fn set_player_registry(&mut self, registry: Arc<PlayerRegistry>) {
        if let Some(manager) = &self.core.canonical_map_manager {
            let _ = registry.bind_canonical_map_manager(Arc::clone(manager));
        } else if let Some(manager) = registry.canonical_map_manager_like_cpp() {
            self.core.canonical_map_manager = Some(manager);
        }
        self.core.player_registry = Some(registry);
    }

    pub fn player_registry(&self) -> Option<&Arc<PlayerRegistry>> {
        self.core.player_registry()
    }

    pub fn pending_invites(&self) -> Option<&Arc<PendingInvites>> {
        self.core.pending_invites()
    }

    /// Register this session in the player registry.
    /// Called after player login is complete (player_guid + position both set).
    pub(crate) fn register_in_player_registry(&self) {
        #[cfg(test)]
        crate::canonical_player_sync::hydrate_player_directory_fixture_like_cpp(self);
        let (Some(guid), Some(pos), Some(name), Some(reg)) = (
            self.player_guid(),
            crate::session::hub_ref(self).player_position_like_cpp(),
            crate::session::hub_ref(self).player_name_like_cpp(),
            &self.core.player_registry,
        ) else {
            return;
        };
        let map_id = self.core.player_map_id_like_cpp();
        let race = crate::session::hub_ref(self).player_race_like_cpp();
        let class = crate::session::hub_ref(self).player_class_like_cpp();
        let gender = crate::session::hub_ref(self).player_gender_like_cpp();
        let level = crate::session::hub_ref(self).player_level_like_cpp();
        let Some(is_alive) = crate::session::hub_ref(self).resolved_player_is_alive_like_cpp()
        else {
            return;
        };
        // Fallback to 0 (world/default instance) when no canonical map key is
        // available — mirrors C++ world-map phase where instance_id == 0.
        let instance_id = self
            .core
            .current_canonical_player_map_key_like_cpp()
            .map(|k| k.instance_id)
            .unwrap_or(0);
        let active_loot_rolls = self
            .loot
            .represented_loot_roll_command_identities_snapshot_like_cpp();
        reg.register_or_replace(
            guid,
            PlayerSessionRegistrationLikeCpp {
                identity: crate::session::directory::PlayerDirectoryIdentityLikeCpp::new_with_bnet(
                    name.clone(),
                    self.core.account_id,
                    self.battlenet_account_id(),
                    self.core.account_state.recruiter_id_like_cpp,
                    race,
                    class,
                    gender,
                    self.core.expansion,
                ),
                placement: crate::session::directory::PlayerDirectoryPlacementLikeCpp {
                    map_id,
                    instance_id,
                    position: pos,
                    is_in_world: self.core.player_is_in_world_for_registry_like_cpp(),
                    level,
                    is_alive,
                },
                active_loot_rolls,
                send_tx: self.send_tx().clone(),
                realm_send_tx: self.core.realm_route_tx().clone(),
                command_tx: self.core.session_command_tx.clone(),
                session_phase_tx: self.phase.tx.clone(),
                durable_creature_runtime_commands_like_cpp: Arc::clone(
                    &self.core.durable_creature_runtime_commands_like_cpp,
                ),
                client_visible_guids_like_cpp: self.core.client_visible_guids_like_cpp.clone(),
                client_visible_transports_like_cpp: self
                    .visibility
                    .client_visible_transports_like_cpp()
                    .clone(),
                advanced_combat_logging_enabled_like_cpp: Arc::clone(
                    &self.core.flags.advanced_combat_logging_enabled_like_cpp,
                ),
                visibility_refresh_pending_like_cpp: Arc::clone(
                    &self.core.flags.visibility_refresh_pending_like_cpp,
                ),
            },
            Arc::clone(
                self.lifecycle
                    .durable_loot_money_persistence_tracker_like_cpp(),
            ),
        );
        // Production already has the canonical Player before publication. The
        // explicit owner-installing test harness creates it while registering,
        // so repeat the one-way hydration after that seam as well.
        #[cfg(test)]
        crate::canonical_player_sync::hydrate_player_directory_fixture_like_cpp(self);
        self.sync_player_registry_party_member_party_type_like_cpp();
        debug!(
            "Registered player {:?} ({}) in broadcast registry (map {})",
            guid, name, map_id
        );
    }

    pub(crate) fn sync_player_registry_state_like_cpp(&self) {
        wow_world_application::sync_player_registry_state_like_cpp(
            crate::session::hub_ref(self),
            &self.loot,
            #[cfg(any(test, feature = "test-fixtures"))]
            &self.spell_state,
            #[cfg(any(test, feature = "test-fixtures"))]
            &self.quest_state,
            cfg!(test),
        );
    }
}
