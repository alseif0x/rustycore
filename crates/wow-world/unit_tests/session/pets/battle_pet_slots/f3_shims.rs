// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1241 F3 test shims: WorldSession entry points kept only for unit_tests callers.

#[allow(unused_imports)]
use super::*;

impl crate::session::WorldSession {
    #[cfg(test)]
    pub(crate) fn battle_pet_set_battle_slot_like_cpp(
        &mut self,
        pet_guid: ObjectGuid,
        slot: u8,
    ) -> bool {
        crate::session::hub_mut(self).battle_pet_set_battle_slot_like_cpp(pet_guid, slot)
    }
    #[cfg(test)]
    pub(crate) fn battle_pet_unlock_slot_like_cpp(&mut self, slot: u8) -> bool {
        crate::session::hub_mut(self).battle_pet_unlock_slot_like_cpp(slot)
    }
    #[cfg(test)]
    pub(crate) fn complete_represented_battle_pet_slot_authority_load_like_cpp(
        &mut self,
        rows: impl IntoIterator<Item = (u8, Option<ObjectGuid>, bool)>,
    ) -> bool {
        crate::session::hub_mut(self)
            .complete_represented_battle_pet_slot_authority_load_like_cpp(rows)
    }
    #[cfg(test)]
    pub(crate) fn represented_battle_pet_slot_like_cpp(&self, slot: u8) -> Option<ObjectGuid> {
        self.fixtures
            .pets
            .represented_battle_pet_slot_like_cpp(slot)
    }
    #[cfg(test)]
    pub(crate) fn represented_battle_pet_slot_locked_like_cpp(&self, slot: u8) -> Option<bool> {
        self.fixtures
            .pets
            .represented_battle_pet_slot_locked_like_cpp(slot)
    }
}
