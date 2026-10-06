// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! The Social-owned query that combines canonical group membership with its
//! existing detached-test fallback.

use crate::SessionSocialLimits;
use wow_world_core::session::PlayerGroupOwnerAccessLikeCpp;

impl SessionSocialLimits {
    pub fn resolved_group_guid_with_access_like_cpp(
        &self,
        owner: &PlayerGroupOwnerAccessLikeCpp<'_>,
        consumer_test: bool,
    ) -> Option<u64> {
        let canonical_group_guid = owner.canonical_group_guid_like_cpp();
        #[cfg(not(any(test, feature = "test-fixtures")))]
        let _ = consumer_test;
        #[cfg(any(test, feature = "test-fixtures"))]
        if consumer_test && canonical_group_guid.is_none() && owner.owner_handle_absent_like_cpp() {
            return self.group_guid_for_test_like_cpp();
        }
        canonical_group_guid.flatten()
    }
}
