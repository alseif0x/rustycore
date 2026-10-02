#[cfg(any(test, feature = "test-fixtures"))]
use crate::session::battle_pet_adapter::{
    BATTLE_PET_SLOT_COUNT_LIKE_CPP, RepresentedBattlePetSlotLikeCpp,
};
#[cfg(any(test, feature = "test-fixtures"))]
use wow_core::ObjectGuid;

#[cfg(any(test, feature = "test-fixtures"))]
impl crate::session::state::PetState {
    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn represented_battle_pet_slot_like_cpp(&self, slot: u8) -> Option<ObjectGuid> {
        self.battle_pet_test_fixture_like_cpp
            .represented_battle_pet_slots_like_cpp
            .get(slot as usize)
            .and_then(|slot| slot.pet_guid)
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn represented_battle_pet_slot_locked_like_cpp(&self, slot: u8) -> Option<bool> {
        self.battle_pet_test_fixture_like_cpp
            .represented_battle_pet_slots_like_cpp
            .get(slot as usize)
            .map(|slot| slot.locked)
    }
}

impl crate::session::HubMut<'_> {
    /// C++ `WorldSession::HandleBattlePetSetBattleSlot`.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn battle_pet_set_battle_slot_like_cpp(
        &mut self,
        pet_guid: ObjectGuid,
        slot: u8,
    ) -> bool {
        if !self
            .fixtures
            .pets
            .battle_pet_test_fixture_like_cpp
            .represented_battle_pets_like_cpp
            .contains_key(&pet_guid)
        {
            return false;
        }

        let Some(slot_ref) = self
            .fixtures
            .pets
            .battle_pet_test_fixture_like_cpp
            .represented_battle_pet_slots_like_cpp
            .get_mut(slot as usize)
        else {
            return false;
        };
        slot_ref.pet_guid = Some(pet_guid);
        self.core
            .invalidate_canonical_player_spell_hit_aura_authority_like_cpp();
        true
    }

    /// C++ `BattlePetMgr::UnlockSlot`.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn battle_pet_unlock_slot_like_cpp(&mut self, slot: u8) -> bool {
        let Some(slot_ref) = self
            .fixtures
            .pets
            .battle_pet_test_fixture_like_cpp
            .represented_battle_pet_slots_like_cpp
            .get_mut(slot as usize)
        else {
            return false;
        };

        if !slot_ref.locked {
            return false;
        }

        slot_ref.locked = false;
        let packet_slot = slot_ref.packet_slot_like_cpp();
        self.core
            .invalidate_canonical_player_spell_hit_aura_authority_like_cpp();
        self.core
            .send_packet(&wow_packet::packets::misc::PetBattleSlotUpdates {
                slots: vec![packet_slot],
                auto_slotted: false,
                new_slot: true,
            });
        true
    }

    /// Publish one complete fallback projection of
    /// `battle_pet_slots(id, battlePetGuid, locked)`. The canonical account
    /// attachment owns this authority in production; this seam exists for
    /// isolated represented tests and rejects missing, duplicate, or
    /// out-of-range slot rows.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn complete_represented_battle_pet_slot_authority_load_like_cpp(
        &mut self,
        rows: impl IntoIterator<Item = (u8, Option<ObjectGuid>, bool)>,
    ) -> bool {
        self.core
            .invalidate_canonical_player_spell_hit_aura_authority_like_cpp();
        self.fixtures
            .pets
            .battle_pet_test_fixture_like_cpp
            .represented_battle_pet_slots_authority_complete_like_cpp = false;

        let mut slots =
            std::array::from_fn(|index| RepresentedBattlePetSlotLikeCpp::locked_empty(index as u8));
        let mut seen = [false; BATTLE_PET_SLOT_COUNT_LIKE_CPP];
        for (index, pet_guid, locked) in rows {
            let slot_index = usize::from(index);
            let Some(slot) = slots.get_mut(slot_index) else {
                return false;
            };
            if seen[slot_index] {
                return false;
            }
            seen[slot_index] = true;
            *slot = RepresentedBattlePetSlotLikeCpp {
                pet_guid,
                collar_id: 0,
                index,
                locked,
            };
        }
        if !seen.into_iter().all(|present| present) {
            return false;
        }

        self.fixtures
            .pets
            .battle_pet_test_fixture_like_cpp
            .represented_battle_pet_slots_like_cpp = slots;
        self.fixtures
            .pets
            .battle_pet_test_fixture_like_cpp
            .represented_battle_pet_slots_authority_complete_like_cpp = true;
        true
    }
}
