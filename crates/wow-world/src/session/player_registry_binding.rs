// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Player registry binding: private Session responsibility.
//! Relocated under #1233; canonical state, phase order and public paths are unchanged.

use super::{Arc, PendingInvites, PlayerRegistry, PlayerSessionRegistrationLikeCpp, UnitState};
use super::{WorldSession, debug, registry};

impl WorldSession {
    /// Set the shared player registry (used for broadcast).
    pub fn set_player_registry(&mut self, registry: Arc<PlayerRegistry>) {
        if let Some(manager) = &self.canonical_map_manager {
            let _ = registry.bind_canonical_map_manager(Arc::clone(manager));
        } else if let Some(manager) = registry.canonical_map_manager_like_cpp() {
            self.canonical_map_manager = Some(manager);
        }
        self.player_registry = Some(registry);
    }

    /// Get a reference to the shared player registry.
    pub fn player_registry(&self) -> Option<&Arc<PlayerRegistry>> {
        self.player_registry.as_ref()
    }

    /// Get a reference to the shared pending invites map.
    pub fn pending_invites(&self) -> Option<&Arc<PendingInvites>> {
        self.pending_invites.as_ref()
    }

    pub(crate) fn player_is_in_world_for_registry_like_cpp(&self) -> bool {
        let Some(guid) = self.player_guid() else {
            return false;
        };

        if let Some(manager) = &self.canonical_map_manager
            && let Ok(manager) = manager.lock()
        {
            let mut canonical_in_world = None;
            manager.do_for_all_maps(|managed| {
                if canonical_in_world.is_none()
                    && let Some(player) = managed.map().get_typed_player(guid)
                {
                    canonical_in_world = Some(player.unit().world().object().is_in_world());
                }
            });
            if let Some(is_in_world) = canonical_in_world {
                return is_in_world;
            }
        }

        // Registry insertion happens only after successful character login; logout/disconnect
        // unregisters instead of leaving a false/stale row behind.
        true
    }

    pub(crate) fn player_is_strictly_in_world_like_cpp(&self) -> bool {
        let Some(guid) = self.player_guid() else {
            return false;
        };
        let Some(manager) = self
            .canonical_map_manager
            .as_ref()
            .and_then(|manager| manager.lock().ok())
        else {
            return false;
        };
        let mut is_in_world = None;
        manager.do_for_all_maps(|managed| {
            if is_in_world.is_none()
                && let Some(player) = managed.map().get_typed_player(guid)
            {
                is_in_world = Some(player.unit().world().object().is_in_world());
            }
        });
        is_in_world.unwrap_or(false)
    }

    pub(crate) fn player_has_unit_state_like_cpp(&self, state: UnitState) -> bool {
        self.canonical_player_snapshot_like_cpp(|player| player.unit().unit_state())
            .is_some_and(|unit_state| unit_state & state.bits() != 0)
    }

    /// Register this session in the player registry.
    /// Called after player login is complete (player_guid + position both set).
    pub(crate) fn register_in_player_registry(&self) {
        #[cfg(test)]
        crate::canonical_player_sync::hydrate_player_directory_fixture_like_cpp(self);
        let (Some(guid), Some(pos), Some(name), Some(reg)) = (
            self.player_guid(),
            self.player_position_like_cpp(),
            self.player_name_like_cpp(),
            &self.player_registry,
        ) else {
            return;
        };
        let map_id = self.player_map_id_like_cpp();
        let race = self.player_race_like_cpp();
        let class = self.player_class_like_cpp();
        let gender = self.player_gender_like_cpp();
        let level = self.player_level_like_cpp();
        let Some(is_alive) = self.resolved_player_is_alive_like_cpp() else {
            return;
        };
        // Fallback to 0 (world/default instance) when no canonical map key is
        // available — mirrors C++ world-map phase where instance_id == 0.
        let instance_id = self
            .current_canonical_player_map_key_like_cpp()
            .map(|k| k.instance_id)
            .unwrap_or(0);
        let active_loot_rolls = self
            .represented_loot_rolls
            .values()
            .map(|state| state.command_identity.clone())
            .collect();
        reg.register_or_replace(
            guid,
            PlayerSessionRegistrationLikeCpp {
                identity: crate::session::directory::PlayerDirectoryIdentityLikeCpp::new_with_bnet(
                    name.clone(),
                    self.account_id,
                    self.battlenet_account_id(),
                    self.recruiter_id_like_cpp,
                    race,
                    class,
                    gender,
                    self.expansion,
                ),
                placement: crate::session::directory::PlayerDirectoryPlacementLikeCpp {
                    map_id,
                    instance_id,
                    position: pos,
                    is_in_world: self.player_is_in_world_for_registry_like_cpp(),
                    level,
                    is_alive,
                },
                active_loot_rolls,
                send_tx: self.send_tx().clone(),
                realm_send_tx: self.realm_route_tx().clone(),
                command_tx: self.session_command_tx.clone(),
                session_phase_tx: self.session_phase_tx.clone(),
                durable_creature_runtime_commands_like_cpp: Arc::clone(
                    &self.durable_creature_runtime_commands_like_cpp,
                ),
                client_visible_guids_like_cpp: self.client_visible_guids_like_cpp.clone(),
                client_visible_transports_like_cpp: self.client_visible_transports_like_cpp.clone(),
                advanced_combat_logging_enabled_like_cpp: Arc::clone(
                    &self.advanced_combat_logging_enabled_like_cpp,
                ),
                visibility_refresh_pending_like_cpp: Arc::clone(
                    &self.visibility_refresh_pending_like_cpp,
                ),
            },
            Arc::clone(&self.durable_loot_money_persistence_like_cpp),
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
        let (Some(guid), Some(registry)) = (self.player_guid(), &self.player_registry) else {
            return;
        };
        self.update_registry_position();
        #[cfg(test)]
        crate::canonical_player_sync::hydrate_player_directory_fixture_like_cpp(self);
        registry.replace_loot_rolls_for_control_channel(
            guid,
            &self.session_command_tx,
            self.represented_loot_rolls
                .values()
                .map(|state| state.command_identity.clone())
                .collect(),
        );
        self.sync_player_registry_party_member_party_type_like_cpp();
    }
}
