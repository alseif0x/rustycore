// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1241 F3 test shims: WorldSession entry points kept only for unit_tests callers.

#[allow(unused_imports)]
use super::*;

impl crate::session::WorldSession {
    pub(crate) fn set_represented_critter_guid_like_cpp(&mut self, guid: Option<ObjectGuid>) {
        crate::session::hub_mut(self).set_represented_critter_guid_like_cpp(guid)
    }
    pub(crate) fn represented_critter_guid_like_cpp(&self) -> Option<ObjectGuid> {
        crate::session::hub_ref(self).represented_critter_guid_like_cpp()
    }
    #[cfg(test)]
    pub(crate) fn represented_dismissed_critter_guids_like_cpp(&self) -> &[ObjectGuid] {
        self.fixtures
            .pets
            .represented_dismissed_critter_guids_like_cpp()
    }
}
