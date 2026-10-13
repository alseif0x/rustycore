//! Battle-pet purchase test entry points (#1363 slice 7a): the saga moved to
//! `wow_world_inventory::battle_pet_purchase`; these cfg(test) wrappers keep
//! the executor tests' method syntax.

use super::*;

impl WorldSession {
    pub(crate) async fn execute_battle_pet_trainer_purchase_like_cpp(
        &mut self,
        money_persistence: ExclusivePlayerMoneyPersistenceLikeCpp,
        trainer_guid: ObjectGuid,
        trainer_id: u32,
        offer: PreparedBattlePetTrainerOfferLikeCpp,
    ) -> BattlePetPurchaseExecutionLikeCpp {
        let generators = self.id_generators_for_test_like_cpp();
        let battle_pet_selection_store = self
            .battle_pet_selection_store_like_cpp()
            .cloned()
            .unwrap_or_default();
        wow_world_inventory::execute_battle_pet_trainer_purchase_with_generator_like_cpp(
            self,
            generators.item.as_ref(),
            battle_pet_selection_store.as_ref(),
            money_persistence,
            trainer_guid,
            trainer_id,
            offer,
        )
        .await
    }
    pub(crate) async fn recover_battle_pet_trainer_purchases_like_cpp(
        &mut self,
    ) -> Option<BattlePetPurchaseRecoveryLikeCpp> {
        let generators = self.id_generators_for_test_like_cpp();
        wow_world_inventory::recover_battle_pet_trainer_purchases_with_generator_like_cpp(
            self,
            generators.item.as_ref(),
        )
        .await
    }
}
