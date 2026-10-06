//! Represented battle-pet loadout slots.
//!
//! Moved out of the Session root under #607. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {}

impl crate::session::PetsCx<'_> {
    pub(crate) async fn battle_pet_set_battle_slot_durable_like_cpp(
        &mut self,
        pet_guid: ObjectGuid,
        slot: u8,
    ) -> bool {
        let Some(attachment) = self.lifecycle.battle_pet_account_attachment_like_cpp() else {
            #[cfg(test)]
            return self.hub.battle_pet_set_battle_slot_like_cpp(pet_guid, slot);
            #[cfg(not(test))]
            return false;
        };
        let owner = Arc::clone(attachment.owner_like_cpp());
        let lease = attachment.lease_id_like_cpp();
        match owner.try_set_slot_like_cpp(lease, pet_guid, slot).await {
            Ok(_) => {
                self.hub
                    .core
                    .invalidate_canonical_player_spell_hit_aura_authority_like_cpp();
                true
            }
            Err(error) => {
                self.hub.shared().log_battle_pet_mutation_failure_like_cpp(
                    "set battle slot",
                    pet_guid,
                    &error,
                );
                false
            }
        }
    }
}

#[cfg(test)]
#[path = "../../../unit_tests/session/pets/battle_pet_slots/f3_shims.rs"]
mod f3_shims;
