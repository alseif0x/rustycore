// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1241 F3 test shims: WorldSession entry points kept only for unit_tests callers.

#[allow(unused_imports)]
use super::*;

impl crate::session::WorldSession {
    #[cfg(test)]
    pub(crate) fn add_represented_battle_pet_packet_info_like_cpp(
        &mut self,
        pet_guid: ObjectGuid,
        packet_info: RepresentedBattlePetDataLikeCpp,
    ) {
        self.fixtures
            .pets
            .add_represented_battle_pet_packet_info_like_cpp(pet_guid, packet_info)
    }
    #[cfg(test)]
    pub(crate) fn send_battle_pet_updates_like_cpp(
        &mut self,
        pet_guids: &[ObjectGuid],
        pet_added: bool,
    ) -> usize {
        crate::session::hub_mut(self).send_battle_pet_updates_like_cpp(pet_guids, pet_added)
    }
    #[cfg(test)]
    pub(crate) fn represented_battle_pet_data_updates_like_cpp(&self) -> &[ObjectGuid] {
        self.fixtures
            .pets
            .represented_battle_pet_data_updates_like_cpp()
    }
}
