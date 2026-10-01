// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1241 F3 test shims: WorldSession entry points kept only for unit_tests callers.

#[allow(unused_imports)]
use super::*;

impl crate::session::WorldSession {
    #[allow(dead_code)]
    pub(crate) fn record_represented_gameobject_zone_area_like_cpp(
        &mut self,
        guid: ObjectGuid,
        zone_id: u32,
        area_id: u32,
    ) {
        self.world_entities
            .record_represented_gameobject_zone_area_like_cpp(guid, zone_id, area_id)
    }
    #[allow(dead_code)]
    pub(crate) fn record_represented_gameobject_spell_id_like_cpp(
        &mut self,
        guid: ObjectGuid,
        spell_id: u32,
    ) {
        let (state, mut hub) = crate::session::split_world_entities_mut(self);
        state.record_represented_gameobject_spell_id_like_cpp(&mut hub, guid, spell_id)
    }
    pub(crate) fn record_represented_gameobject_phase_shift_like_cpp(
        &mut self,
        guid: ObjectGuid,
        phase_shift: PhaseShift,
    ) {
        self.world_entities
            .record_represented_gameobject_phase_shift_like_cpp(guid, phase_shift)
    }
}
