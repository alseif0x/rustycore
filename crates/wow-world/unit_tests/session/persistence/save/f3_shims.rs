// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1241 F3 test shims: WorldSession entry points kept only for unit_tests callers.

#[allow(unused_imports)]
use super::*;

impl crate::session::WorldSession {
    pub(crate) fn account_heirloom_save_rows_like_cpp(
        &self,
    ) -> Option<Vec<AccountHeirloomSaveRowLikeCpp>> {
        let (state, hub) = crate::session::split_lifecycle_ref(self);
        state.account_heirloom_save_rows_like_cpp(hub)
    }
    pub(crate) fn account_toy_save_rows_like_cpp(&self) -> Option<Vec<AccountToySaveRowLikeCpp>> {
        let (state, hub) = crate::session::split_lifecycle_ref(self);
        state.account_toy_save_rows_like_cpp(hub)
    }
    pub(crate) fn account_mount_save_rows_like_cpp(
        &self,
    ) -> Option<Vec<AccountMountSaveRowLikeCpp>> {
        let (state, hub) = crate::session::split_lifecycle_ref(self);
        state.account_mount_save_rows_like_cpp(hub)
    }
}
