//! Battle-pet packets and updates published to the client.
//!
//! Moved out of the Session root under #607. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {}

impl crate::session::PetsCx<'_> {
    /// C++ `BattlePetMgr::UpdateBattlePetData`, represented at the gate level.
    pub(crate) fn battle_pet_update_notify_like_cpp(&mut self, pet_guid: ObjectGuid) -> bool {
        let Some(pet) = self.shared().represented_battle_pet_like_cpp(pet_guid) else {
            return false;
        };

        if self
            .hub
            .shared()
            .represented_summoned_battle_pet_guid_like_cpp()
            != Some(pet_guid)
        {
            return false;
        }

        let _ = self.hub.core.mutate_canonical_player_like_cpp(|player| {
            player.set_battle_pet_data_like_cpp(pet_guid, pet.quality, pet.level);
        });
        #[cfg(test)]
        self.hub
            .fixtures
            .pets
            .battle_pet_test_fixture_like_cpp
            .represented_battle_pet_data_updates_like_cpp
            .push(pet_guid);
        true
    }
}

#[cfg(test)]
#[path = "../../../unit_tests/session/pets/battle_pet_publication/f3_shims.rs"]
mod f3_shims;
