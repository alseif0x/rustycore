// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1241 F3 test shims: WorldSession entry points kept only for unit_tests callers.

#[allow(unused_imports)]
use super::*;

impl crate::session::WorldSession {
    #[cfg(test)]
    pub(in crate::session::progression::talents) fn represented_spent_talent_points_count_like_cpp(
        &self,
    ) -> Option<u32> {
        crate::session::hub_ref(self).represented_spent_talent_points_count_like_cpp()
    }
}
