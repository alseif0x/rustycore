// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Runtime policy access: private Session responsibility.
//! Relocated under #1233; canonical state, phase order and public paths are unchanged.

use super::{Arc, ChatFloodThrottleIndexLikeCpp, DisableMgrLikeCpp, SocketTimeoutsLikeCpp};
use super::{WaypointPathResolverLikeCpp, WorldSession, registry};

impl WorldSession {
    pub fn set_reset_schedule_like_cpp(&mut self, schedule: wow_instances::ResetSchedule) {
        self.reset_schedule_like_cpp = schedule;
    }

    pub fn set_represented_is_outdoors_like_cpp(&mut self, is_outdoors: bool) {
        let _ = self.set_player_is_outdoors_like_cpp(is_outdoors);
    }

    #[cfg(test)]
    pub fn set_start_all_explored_like_cpp(&mut self, enabled: bool) {
        self.player_bootstrap_catalog_test_fixture_like_cpp
            .start_all_explored_like_cpp = enabled;
    }

    #[cfg(test)]
    pub(crate) fn start_all_explored_like_cpp(&self) -> bool {
        self.player_bootstrap_catalog_test_fixture_like_cpp
            .start_all_explored_like_cpp
    }

    #[cfg(test)]
    pub fn set_addon_channel_like_cpp(&mut self, enabled: bool) {
        self.addon_channel_like_cpp = enabled;
    }

    pub fn set_socket_timeouts_like_cpp(&mut self, timeouts: SocketTimeoutsLikeCpp) {
        self.socket_timeouts_like_cpp = timeouts;
        self.reset_timeout_time_like_cpp(false);
    }

    pub fn set_server_expansion_like_cpp(&mut self, expansion: u8) {
        self.server_expansion_like_cpp = expansion;
    }

    #[cfg(test)]
    pub fn set_characters_per_realm_like_cpp(&mut self, characters_per_realm: u32) {
        self.characters_per_realm_like_cpp = characters_per_realm;
    }

    #[cfg(test)]
    pub fn set_declined_names_used_like_cpp(&mut self, used: bool) {
        self.declined_names_used_like_cpp = used;
    }

    #[cfg(test)]
    pub(crate) fn declined_names_used_like_cpp(&self) -> bool {
        self.declined_names_used_like_cpp
    }

    #[cfg(test)]
    pub fn set_feature_system_character_undelete_enabled_like_cpp(&mut self, enabled: bool) {
        self.feature_system_character_undelete_enabled_like_cpp = enabled;
    }

    pub fn set_waypoint_path_resolver_like_cpp(&mut self, resolver: WaypointPathResolverLikeCpp) {
        self.waypoint_path_resolver_like_cpp = Some(resolver);
    }

    #[cfg(test)]
    pub(crate) fn addon_channel_like_cpp(&self) -> bool {
        self.addon_channel_like_cpp
    }

    #[cfg(test)]
    pub(crate) fn update_speak_time_like_cpp(&mut self, index: ChatFloodThrottleIndexLikeCpp) {
        self.update_speak_time_with_policy_like_cpp(index, self.chat_flood_config_like_cpp)
    }

    /// Set the C++ DisableMgr store loaded from the `disables` table.
    pub fn set_disable_mgr(&mut self, store: Arc<DisableMgrLikeCpp>) {
        self.disable_mgr = Some(store);
    }

    /// Get the loaded DisableMgr store reference.
    pub fn disable_mgr(&self) -> Option<&Arc<DisableMgrLikeCpp>> {
        self.disable_mgr.as_ref()
    }

    /// C++ `sLockStore.LookupEntry(lockId)`.
    pub fn lock_entry_exists_like_cpp(&self, lock_id: u32) -> bool {
        self.lock_store
            .as_ref()
            .is_some_and(|store| store.contains(lock_id))
    }

    /// Share the process-wide trusted module registry with this session.
    ///
    /// Composition calls this once after construction. A session that never
    /// receives one keeps the zero-module no-op path.
    #[cfg(test)]
    pub fn set_module_registry_like_cpp(&mut self, registry: Arc<wow_module_api::ModuleRegistry>) {
        self.module_registry_like_cpp = Some(registry);
    }

    pub(in crate::session) fn trinity_string_like_cpp(&self, entry: u32) -> &str {
        self.trinity_string_store
            .as_ref()
            .map(|store| store.get_like_cpp(entry, &self.locale))
            .unwrap_or("<error>")
    }
}
