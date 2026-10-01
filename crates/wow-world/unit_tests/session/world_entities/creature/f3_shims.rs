// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1241 F3 test shims: WorldSession entry points kept only for unit_tests callers.

#[allow(unused_imports)]
use super::*;

impl crate::session::WorldSession {
    pub(crate) fn creature_display_power_for_class_like_cpp(&self, unit_class: u8) -> u8 {
        self.catalogs
            .creature_display_power_for_class_like_cpp(unit_class)
    }
    pub(crate) fn mutate_canonical_creature_by_guid_like_cpp<R>(
        &mut self,
        guid: ObjectGuid,
        f: impl FnOnce(&mut wow_entities::Creature) -> R,
    ) -> Option<R> {
        self.core
            .mutate_canonical_creature_by_guid_like_cpp(guid, f)
    }
}
