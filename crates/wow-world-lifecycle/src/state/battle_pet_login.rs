use wow_world_core::session::{
    HubRef,
    battle_pet_adapter::{BATTLE_PET_SLOT_COUNT_LIKE_CPP, RepresentedBattlePetSlotLikeCpp},
};

use super::SessionLifecycleState;

impl SessionLifecycleState {
    pub fn battle_pet_account_attachment_like_cpp(
        &self,
    ) -> Option<&wow_world_core::battle_pet_account::BattlePetAccountAttachmentLikeCpp> {
        self.battle_pet_account_attachment_like_cpp.as_ref()
    }

    pub fn set_battle_pet_account_attachment_like_cpp(
        &mut self,
        attachment: wow_world_core::battle_pet_account::BattlePetAccountAttachmentLikeCpp,
    ) {
        self.battle_pet_account_attachment_like_cpp = Some(attachment);
    }

    /// C++ login learns spell 125610 when battle-pet slot zero is unlocked.
    /// The account-wide owner is authoritative only after its complete load;
    /// isolated tests must explicitly publish the equivalent three-slot
    /// snapshot instead of relying on the constructor's locked defaults.
    pub fn represented_battle_pet_login_spell_source_is_empty_like_cpp(
        &self,
        hub: HubRef<'_>,
    ) -> bool {
        let slots = if let Some(attachment) = self.battle_pet_account_attachment_like_cpp() {
            attachment
                .owner_like_cpp()
                .journal_like_cpp(attachment.lease_id_like_cpp(), hub.core.player_guid())
                .slots
        } else {
            #[cfg(not(any(test, feature = "test-fixtures")))]
            return false;
            #[cfg(any(test, feature = "test-fixtures"))]
            {
                if !hub
                    .fixtures
                    .pets
                    .battle_pet_test_fixture_like_cpp
                    .represented_battle_pet_slots_authority_complete_like_cpp
                {
                    return false;
                }
                hub.fixtures
                    .pets
                    .battle_pet_test_fixture_like_cpp
                    .represented_battle_pet_slots_like_cpp
                    .iter()
                    .map(RepresentedBattlePetSlotLikeCpp::packet_slot_like_cpp)
                    .collect()
            }
        };

        slots.len() == BATTLE_PET_SLOT_COUNT_LIKE_CPP
            && slots
                .iter()
                .enumerate()
                .all(|(index, slot)| usize::from(slot.index) == index)
            && slots.first().is_some_and(|slot| slot.locked)
    }
}
