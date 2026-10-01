// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1241 F3 test shims: WorldSession entry points kept only for unit_tests callers.

#[allow(unused_imports)]
use super::*;

impl crate::session::WorldSession {
    #[cfg(test)]
    pub(crate) fn insert_represented_equipment_set_like_cpp(
        &mut self,
        guid: u64,
        equipment_set: RepresentedEquipmentSetLikeCpp,
    ) {
        let (state, mut hub) = crate::session::split_inventory_mut(self);
        state.insert_represented_equipment_set_like_cpp(&mut hub, guid, equipment_set)
    }
    #[cfg(test)]
    pub(crate) fn represented_equipment_set_like_cpp(
        &self,
        guid: u64,
    ) -> Option<RepresentedEquipmentSetLikeCpp> {
        let (state, hub) = crate::session::split_inventory_ref(self);
        state.represented_equipment_set_like_cpp(hub, guid)
    }
    #[cfg(test)]
    pub fn set_equipment_set_guid_generator_like_cpp(
        &mut self,
        generator: Arc<EquipmentSetGuidGeneratorLikeCpp>,
    ) {
        let (state, mut hub) = crate::session::split_inventory_mut(self);
        state.set_equipment_set_guid_generator_like_cpp(&mut hub, generator)
    }
    #[cfg(test)]
    pub(crate) fn equipment_set_guid_generator_for_test_like_cpp(
        &self,
    ) -> Option<Arc<EquipmentSetGuidGeneratorLikeCpp>> {
        let (state, hub) = crate::session::split_inventory_ref(self);
        state.equipment_set_guid_generator_for_test_like_cpp(hub)
    }
    #[cfg(test)]
    pub(in crate::session) fn mark_equipment_sets_saved_like_cpp(&mut self) {
        let (state, mut hub) = crate::session::split_inventory_mut(self);
        state.mark_equipment_sets_saved_like_cpp(&mut hub)
    }
    pub(in crate::session) fn with_owned_equipment_sets_like_cpp<R>(
        &self,
        f: impl FnMut(&wow_entities::PlayerEquipmentSetsLikeCpp) -> R,
    ) -> Option<R> {
        let (state, hub) = crate::session::split_inventory_ref(self);
        state.with_owned_equipment_sets_like_cpp(hub, f)
    }
}
