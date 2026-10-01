// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1241 F3 test shims: WorldSession entry points kept only for unit_tests callers.

#[allow(unused_imports)]
use super::*;

impl crate::session::WorldSession {
    pub(crate) fn represented_shapeshift_form_like_cpp(&self) -> Option<u32> {
        crate::session::hub_ref(self).represented_shapeshift_form_like_cpp()
    }
    pub(in crate::session) fn player_unit_presentation_snapshot_like_cpp(
        &self,
    ) -> Option<(UnitFlags, i32, f32)> {
        crate::session::hub_ref(self).player_unit_presentation_snapshot_like_cpp()
    }
    pub(crate) fn represented_eject_passenger_like_cpp(
        &mut self,
        passenger_guid: ObjectGuid,
    ) -> bool {
        crate::session::hub_mut(self).represented_eject_passenger_like_cpp(passenger_guid)
    }
}
