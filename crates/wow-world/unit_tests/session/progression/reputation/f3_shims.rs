// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1241 F3 test shims: WorldSession entry points kept only for unit_tests callers.

#[allow(unused_imports)]
use super::*;

impl crate::session::WorldSession {
    #[cfg(test)]
    pub fn set_start_all_reputation_like_cpp(&mut self, enabled: bool) {
        self.catalogs.set_start_all_reputation_like_cpp(enabled)
    }
    #[cfg(test)]
    pub(crate) fn start_all_reputation_like_cpp(&self) -> bool {
        self.catalogs.start_all_reputation_like_cpp()
    }
    #[cfg(test)]
    pub(crate) fn reputation_mgr_like_cpp(&self) -> ReputationMgrRefLikeCpp<'_> {
        self.fixtures.progression.reputation_mgr_like_cpp()
    }
    #[cfg(test)]
    pub(crate) fn reputation_mgr_like_cpp_mut(&mut self) -> ReputationMgrMutLikeCpp<'_> {
        self.fixtures.progression.reputation_mgr_like_cpp_mut()
    }
    pub(crate) fn with_reputation_mgr_like_cpp<R>(
        &self,
        operation: impl FnOnce(&ReputationMgrRefLikeCpp<'_>) -> R,
    ) -> Option<R> {
        crate::session::hub_ref(self).with_reputation_mgr_like_cpp(operation)
    }
    pub(crate) fn mutate_reputation_mgr_like_cpp<R>(
        &mut self,
        operation: impl FnOnce(&mut ReputationMgrMutLikeCpp<'_>) -> R,
    ) -> Option<R> {
        crate::session::hub_mut(self).mutate_reputation_mgr_like_cpp(operation)
    }
}
