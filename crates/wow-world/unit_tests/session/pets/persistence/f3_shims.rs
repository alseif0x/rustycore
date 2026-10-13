// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1241 F3 test shims: WorldSession entry points kept only for unit_tests callers.

#[allow(unused_imports)]
use crate::session::*;

impl crate::session::WorldSession {
    pub(crate) fn load_represented_pet_declined_names_like_cpp(
        &mut self,
        pet_number: u32,
        row: Option<CharacterPetDeclinedNamesRowLikeCpp>,
    ) -> bool {
        wow_world_lifecycle::load_represented_pet_declined_names_like_cpp(self, pet_number, row)
    }
}

impl WorldSession {
    pub(crate) fn set_battle_pet_purchase_persistence_port_like_cpp(
        &mut self,
        store: Arc<dyn wow_persistence::BattlePetPurchasePersistencePortLikeCpp>,
    ) {
        crate::session::cx_pets(self).set_battle_pet_purchase_persistence_port_like_cpp(store)
    }
}

impl crate::session::PetsCx<'_> {
    /// Install the SQLx-free Character durability port for the recoverable
    /// #161 purchase saga. The composition root owns the MariaDB adapter.
    pub(crate) fn set_battle_pet_purchase_persistence_port_like_cpp(
        &mut self,
        store: Arc<dyn wow_persistence::BattlePetPurchasePersistencePortLikeCpp>,
    ) {
        self.lifecycle
            .set_battle_pet_purchase_persistence_port_like_cpp(store);
    }
}
