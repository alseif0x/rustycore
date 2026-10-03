//! Save and load plans for represented pet and battle-pet state.
//!
//! Moved out of the Session root under #607. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    pub fn set_battle_pet_purchase_persistence_port_like_cpp(
        &mut self,
        store: Arc<dyn wow_persistence::BattlePetPurchasePersistencePortLikeCpp>,
    ) {
        crate::session::cx_pets(self).set_battle_pet_purchase_persistence_port_like_cpp(store)
    }
}

impl crate::session::PetsCx<'_> {
    pub(crate) fn begin_represented_character_pet_authority_load_like_cpp(&mut self) {
        self.lifecycle.pet_load_reset_like_cpp();
        self.hub
            .invalidate_represented_character_pet_empty_authority_like_cpp();
    }

    pub(crate) fn load_represented_pet_declined_names_like_cpp(
        &mut self,
        pet_number: u32,
        row: Option<CharacterPetDeclinedNamesRowLikeCpp>,
    ) -> bool {
        self.hub
            .invalidate_represented_character_pet_empty_authority_like_cpp();
        if let Some(row) = row {
            self.lifecycle
                .pet_load_insert_declined_names_for_pet_number_like_cpp(pet_number, row);
            true
        } else {
            self.lifecycle
                .pet_load_remove_declined_names_for_pet_number_like_cpp(pet_number)
                .is_some()
        }
    }

    /// Install the SQLx-free Character durability port for the recoverable
    /// #161 purchase saga. The composition root owns the MariaDB adapter.
    pub fn set_battle_pet_purchase_persistence_port_like_cpp(
        &mut self,
        store: Arc<dyn wow_persistence::BattlePetPurchasePersistencePortLikeCpp>,
    ) {
        self.lifecycle
            .set_battle_pet_purchase_persistence_port_like_cpp(store);
    }
}

#[cfg(test)]
#[path = "../../../unit_tests/session/pets/persistence/f3_shims.rs"]
mod f3_shims;
