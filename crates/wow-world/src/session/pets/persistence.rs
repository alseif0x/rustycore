//! Save and load plans for represented pet and battle-pet state.
//!
//! Moved out of the Session root under #607. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    pub(crate) fn begin_represented_character_pet_authority_load_like_cpp(&mut self) {
        self.pet_load_query_holder_rows_like_cpp.reset();
        self.invalidate_represented_character_pet_empty_authority_like_cpp();
    }
    pub(crate) fn load_represented_pet_declined_names_like_cpp(
        &mut self,
        pet_number: u32,
        row: Option<CharacterPetDeclinedNamesRowLikeCpp>,
    ) -> bool {
        self.invalidate_represented_character_pet_empty_authority_like_cpp();
        if let Some(row) = row {
            self.pet_load_query_holder_rows_like_cpp
                .declined_names
                .insert(pet_number, row);
            true
        } else {
            self.pet_load_query_holder_rows_like_cpp
                .declined_names
                .remove(&pet_number)
                .is_some()
        }
    }

    /// Install the SQLx-free Character durability port for the recoverable
    /// #161 purchase saga. The composition root owns the MariaDB adapter.
    pub fn set_battle_pet_purchase_persistence_port_like_cpp(
        &mut self,
        store: Arc<dyn wow_persistence::BattlePetPurchasePersistencePortLikeCpp>,
    ) {
        self.persistence_ports_like_cpp.player.battle_pet_purchase = Some(store);
    }
}
