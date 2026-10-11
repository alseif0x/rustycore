// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Runtime policy access: private Session responsibility.
//! Relocated under #1233; canonical state, phase order and public paths are unchanged.

use super::{Arc, DisableMgrLikeCpp, SocketTimeoutsLikeCpp};
use super::{WaypointPathResolverLikeCpp, WorldSession, registry};

impl WorldSession {
    pub fn set_reset_schedule_like_cpp(&mut self, schedule: wow_instances::ResetSchedule) {
        self.config.reset_schedule_like_cpp = schedule;
    }

    pub fn set_socket_timeouts_like_cpp(&mut self, timeouts: SocketTimeoutsLikeCpp) {
        self.core.admission.socket_timeouts_like_cpp = timeouts;
        self.core.reset_timeout_time_like_cpp(false);
    }

    pub fn set_server_expansion_like_cpp(&mut self, expansion: u8) {
        self.core.realm_policy.server_expansion_like_cpp = expansion;
    }

    pub fn set_waypoint_path_resolver_like_cpp(&mut self, resolver: WaypointPathResolverLikeCpp) {
        self.catalogs.waypoint_path_resolver_like_cpp = Some(resolver);
    }

    /// Set the C++ DisableMgr store loaded from the `disables` table.
    pub fn set_disable_mgr(&mut self, store: Arc<DisableMgrLikeCpp>) {
        self.catalogs.disable_mgr = Some(store);
    }

    pub(crate) fn disable_mgr(&self) -> Option<&Arc<DisableMgrLikeCpp>> {
        self.catalogs.disable_mgr()
    }

    pub(crate) fn lock_entry_exists_like_cpp(&self, lock_id: u32) -> bool {
        self.catalogs.lock_entry_exists_like_cpp(lock_id)
    }

    pub(in crate::session) fn trinity_string_like_cpp(&self, entry: u32) -> &str {
        self.catalogs
            .trinity_string_store
            .as_ref()
            .map(|store| store.get_like_cpp(entry, &self.core.locale))
            .unwrap_or("<error>")
    }
}

#[cfg(test)]
#[path = "../../unit_tests/session/runtime_policy_access/f3_shims.rs"]
mod f3_shims;
