// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! World adapter for the battle-pet purchase saga moved to
//! `wow_world_inventory::battle_pet_purchase` (#1363 slice 7a). Every method
//! is a pure forward to the same-named inherent `WorldSession` method, field
//! or pets-context method.

use std::future::Future;
use std::sync::Arc;

use wow_core::ObjectGuid;
use wow_persistence::BattlePetPurchasePersistencePortLikeCpp;
use wow_world_core::session::HubMut;
use wow_world_inventory::BattlePetPurchaseHostLikeCpp;
use wow_world_lifecycle::SessionLifecycleState;

use crate::battle_pet_account::{BattlePetAccountOwnerLikeCpp, BattlePetLeaseIdLikeCpp};
use crate::session::{ExclusivePlayerMoneyPersistenceLikeCpp, WorldSession};

impl BattlePetPurchaseHostLikeCpp for WorldSession {
    fn lifecycle(&self) -> &SessionLifecycleState {
        &self.lifecycle
    }
    fn split_lifecycle_mut(&mut self) -> (&mut SessionLifecycleState, HubMut<'_>) {
        crate::session::split_lifecycle_mut(self)
    }
    fn battle_pet_account_owner_lease_like_cpp(
        &self,
    ) -> Option<(Arc<BattlePetAccountOwnerLikeCpp>, BattlePetLeaseIdLikeCpp)> {
        crate::session::cx_pets_ref(self).battle_pet_account_owner_lease_like_cpp()
    }
    fn battle_pet_purchase_store_like_cpp(
        &self,
    ) -> Option<Arc<dyn BattlePetPurchasePersistencePortLikeCpp>> {
        crate::session::cx_pets_ref(self).battle_pet_purchase_store_like_cpp()
    }
    fn battle_pet_species_entry_like_cpp(
        &self,
        species: u32,
    ) -> Option<wow_data::BattlePetSpeciesEntry> {
        crate::session::cx_pets_ref(self).battle_pet_species_entry_like_cpp(species)
    }
    fn battle_pet_try_acquire_journal_lease_like_cpp(&self) -> impl Future<Output = bool> + Send {
        async move {
            crate::session::cx_pets_ref(self)
                .battle_pet_try_acquire_journal_lease_like_cpp()
                .await
        }
    }
    fn record_battle_pet_trainer_purchase_criteria_like_cpp(&mut self, species: u32) {
        crate::session::cx_pets(self).record_battle_pet_trainer_purchase_criteria_like_cpp(species)
    }
    fn resolved_player_money_like_cpp(&self) -> Option<u64> {
        WorldSession::resolved_player_money_like_cpp(self)
    }
    fn stage_player_money_change_like_cpp(&mut self, old_money: u64, new_money: u64) -> bool {
        WorldSession::stage_player_money_change_like_cpp(self, old_money, new_money)
    }
    fn send_player_values_update_from_entity_bridge(
        &self,
        inv_slot_changes: &[(u8, ObjectGuid)],
        visible_item_changes: &[(u8, i32, u16, u16)],
        virtual_item_changes: &[(u8, i32, u16, u16)],
        buyback_changes: &[(u8, u32, i64)],
        coinage: Option<u64>,
    ) -> bool {
        WorldSession::send_player_values_update_from_entity_bridge(
            self,
            inv_slot_changes,
            visible_item_changes,
            virtual_item_changes,
            buyback_changes,
            coinage,
        )
    }
    fn drain_represented_quest_objective_progress_with_generator_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
    ) -> impl Future<Output = ()> + Send {
        WorldSession::drain_represented_quest_objective_progress_with_generator_like_cpp(
            self,
            item_guid_generator,
        )
    }
    fn learn_dependent_known_spell_like_cpp(&mut self, spell_id: i32) {
        WorldSession::learn_dependent_known_spell_like_cpp(self, spell_id)
    }
    fn begin_exclusive_player_money_persistence_like_cpp(
        &mut self,
    ) -> impl Future<Output = Option<ExclusivePlayerMoneyPersistenceLikeCpp>> + Send {
        WorldSession::begin_exclusive_player_money_persistence_like_cpp(self)
    }
}
