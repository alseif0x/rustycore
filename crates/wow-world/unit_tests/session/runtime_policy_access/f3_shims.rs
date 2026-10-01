// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1241 F3 test shims: WorldSession entry points kept only for unit_tests callers.

#[allow(unused_imports)]
use super::*;

impl crate::session::WorldSession {
    #[cfg(test)]
    pub fn set_declined_names_used_like_cpp(&mut self, used: bool) {
        self.config.set_declined_names_used_like_cpp(used)
    }
    #[cfg(test)]
    pub fn set_start_all_explored_like_cpp(&mut self, enabled: bool) {
        self.catalogs.set_start_all_explored_like_cpp(enabled)
    }
    #[cfg(test)]
    pub(crate) fn start_all_explored_like_cpp(&self) -> bool {
        self.catalogs.start_all_explored_like_cpp()
    }
}
