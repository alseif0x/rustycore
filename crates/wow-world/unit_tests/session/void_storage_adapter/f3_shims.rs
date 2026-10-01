// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1241 F3 test shims: WorldSession entry points kept only for unit_tests callers.

#[allow(unused_imports)]
use super::*;

impl crate::session::WorldSession {
    pub(in crate::session) fn with_owned_void_storage_like_cpp<R>(
        &self,
        f: impl FnMut(&[Option<RepresentedVoidStorageItemLikeCpp>], bool) -> R,
    ) -> Option<R> {
        let (state, hub) = crate::session::split_inventory_ref(self);
        state.with_owned_void_storage_like_cpp(hub, f)
    }
}
