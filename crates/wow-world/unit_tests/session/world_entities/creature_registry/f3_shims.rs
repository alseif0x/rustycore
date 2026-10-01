// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1241 F3 test shims: WorldSession entry points kept only for unit_tests callers.

#[allow(unused_imports)]
use super::*;

impl crate::session::WorldSession {
    pub(crate) fn mutate_world_creature<F, R>(&mut self, guid: ObjectGuid, f: F) -> Option<R>
    where
        F: FnOnce(&mut crate::map_manager::WorldCreature) -> R,
    {
        self.core.mutate_world_creature(guid, f)
    }
}
