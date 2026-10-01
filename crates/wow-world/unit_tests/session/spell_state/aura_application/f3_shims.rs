// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1241 F3 test shims: WorldSession entry points kept only for unit_tests callers.

#[allow(unused_imports)]
use super::*;

impl crate::session::WorldSession {
    pub(crate) fn same_effect_stack_rule_aura_types_like_cpp(
        &self,
        group_id: u32,
    ) -> Option<&BTreeSet<i32>> {
        self.catalogs
            .same_effect_stack_rule_aura_types_like_cpp(group_id)
    }
}
