use std::sync::Arc;

use crate::session::HubMut;
use crate::session::state::{SessionCatalogs, SessionWorldConfig};
use wow_data::DisableMgrLikeCpp;

impl SessionCatalogs {
    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn set_start_all_explored_like_cpp(&mut self, enabled: bool) {
        self.player_bootstrap_catalog_test_fixture_like_cpp
            .start_all_explored_like_cpp = enabled;
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn start_all_explored_like_cpp(&self) -> bool {
        self.player_bootstrap_catalog_test_fixture_like_cpp
            .start_all_explored_like_cpp
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
}

impl HubMut<'_> {
    pub fn set_represented_is_outdoors_like_cpp(&mut self, is_outdoors: bool) {
        let _ = self.set_player_is_outdoors_like_cpp(is_outdoors);
    }
}

impl SessionWorldConfig {
    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn set_declined_names_used_like_cpp(&mut self, used: bool) {
        self.declined_names_used_like_cpp = used;
    }
}
