//! The original successful personal-money seam through the real typed lifecycle port.
//! Other lifecycle operations are outside this fixture and must not be requested.

use super::money_support::*;
use wow_persistence::*;

struct PersonalMoneyPort {
    player_guid: u64,
    success: bool,
}

pub(super) fn prepare_personal_money_fixture(session: &mut WorldSession, success: bool) {
    prepare_money_player_residence_for_test(session);
    session.set_player_lifecycle_port_like_cpp(Arc::new(PersonalMoneyPort {
        player_guid: session.player_guid().unwrap().counter() as u64,
        success,
    }));
}

impl PlayerLifecyclePortLikeCpp for PersonalMoneyPort {
    fn mark_offline_like_cpp<'a>(
        &'a self,
        _mark: PlayerOfflineMarkLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PersistenceOutcomeLikeCpp> {
        panic!("unexpected mark_offline_like_cpp in personal-money fixture");
    }

    fn persist_homebind_like_cpp<'a>(
        &'a self,
        _request: PlayerHomebindPersistenceRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PersistenceOutcomeLikeCpp> {
        panic!("unexpected persist_homebind_like_cpp in personal-money fixture");
    }

    fn clear_buyback_like_cpp<'a>(
        &'a self,
        _request: PlayerBuybackClearRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PersistenceOutcomeLikeCpp> {
        panic!("unexpected clear_buyback_like_cpp in personal-money fixture");
    }

    fn persist_money_transaction_like_cpp<'a>(
        &'a self,
        request: PlayerMoneyTransactionRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PlayerMoneyTransactionOutcomeLikeCpp> {
        assert_eq!(request.player_guid, self.player_guid);
        assert!(request.durability_repairs.is_empty());
        Box::pin(std::future::ready(if self.success {
            PlayerMoneyTransactionOutcomeLikeCpp::Committed
        } else {
            PlayerMoneyTransactionOutcomeLikeCpp::DefinitelyRolledBack {
                reason: "personal-money fixture persistence failed".to_owned(),
            }
        }))
    }

    fn persist_bank_slot_purchase_like_cpp<'a>(
        &'a self,
        _request: PlayerBankSlotPurchaseRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PlayerMoneyTransactionOutcomeLikeCpp> {
        panic!("unexpected persist_bank_slot_purchase_like_cpp in personal-money fixture");
    }

    fn load_uncage_item_state_like_cpp<'a>(
        &'a self,
        _request: PlayerUncageItemStateRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PlayerUncageItemStateLoadOutcomeLikeCpp> {
        panic!("unexpected load_uncage_item_state_like_cpp in personal-money fixture");
    }

    fn persist_durability_repair_like_cpp<'a>(
        &'a self,
        _repair: PlayerDurabilityRepairSaveLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PersistenceOutcomeLikeCpp> {
        panic!("unexpected persist_durability_repair_like_cpp in personal-money fixture");
    }

    fn persist_money_write_like_cpp<'a>(
        &'a self,
        _request: PlayerMoneyWriteRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PersistenceOutcomeLikeCpp> {
        panic!("unexpected persist_money_write_like_cpp in personal-money fixture");
    }

    fn persist_currency_save_like_cpp<'a>(
        &'a self,
        _request: PlayerCurrencySaveRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PersistenceOutcomeLikeCpp> {
        panic!("unexpected persist_currency_save_like_cpp in personal-money fixture");
    }

    fn persist_talent_reset_like_cpp<'a>(
        &'a self,
        _request: PlayerTalentResetPersistenceRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PersistenceOutcomeLikeCpp> {
        panic!("unexpected persist_talent_reset_like_cpp in personal-money fixture");
    }

    fn persist_xp_like_cpp<'a>(
        &'a self,
        _request: PlayerXpPersistenceRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PersistenceOutcomeLikeCpp> {
        panic!("unexpected persist_xp_like_cpp in personal-money fixture");
    }

    fn refresh_realm_character_count_like_cpp<'a>(
        &'a self,
        _request: PlayerRealmCharacterCountRefreshRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PersistenceOutcomeLikeCpp> {
        panic!("unexpected refresh_realm_character_count_like_cpp in personal-money fixture");
    }

    fn load_initial_world_states_like_cpp<'a>(
        &'a self,
    ) -> PersistenceFutureLikeCpp<'a, PlayerInitialWorldStatesLoadOutcomeLikeCpp> {
        panic!("unexpected load_initial_world_states_like_cpp in personal-money fixture");
    }

    fn load_login_transports_like_cpp<'a>(
        &'a self,
        _request: PlayerLoginTransportLoadRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PlayerLoginTransportLoadOutcomeLikeCpp> {
        panic!("unexpected load_login_transports_like_cpp in personal-money fixture");
    }

    fn load_character_base_like_cpp<'a>(
        &'a self,
        _request: PlayerCharacterBaseLoadRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PlayerCharacterBaseLoadOutcomeLikeCpp> {
        panic!("unexpected load_character_base_like_cpp in personal-money fixture");
    }

    fn load_account_collection_like_cpp<'a>(
        &'a self,
        _request: AccountCollectionLoadRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, AccountCollectionLoadOutcomeLikeCpp> {
        panic!("unexpected load_account_collection_like_cpp in personal-money fixture");
    }

    fn load_login_admission_like_cpp<'a>(
        &'a self,
        _request: PlayerLoginAdmissionLoadRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PlayerLoginAdmissionLoadOutcomeLikeCpp> {
        panic!("unexpected load_login_admission_like_cpp in personal-money fixture");
    }

    fn load_login_auxiliary_like_cpp<'a>(
        &'a self,
        _request: PlayerLoginAuxiliaryLoadRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PlayerLoginAuxiliaryLoadOutcomeLikeCpp> {
        panic!("unexpected load_login_auxiliary_like_cpp in personal-money fixture");
    }

    fn persist_login_item_repairs_like_cpp<'a>(
        &'a self,
        _request: PlayerLoginItemRepairRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PersistenceOutcomeLikeCpp> {
        panic!("unexpected persist_login_item_repairs_like_cpp in personal-money fixture");
    }

    fn reset_login_pet_talents_like_cpp<'a>(
        &'a self,
        _player_guid: u64,
    ) -> PersistenceFutureLikeCpp<'a, PlayerLoginPetTalentResetOutcomeLikeCpp> {
        panic!("unexpected reset_login_pet_talents_like_cpp in personal-money fixture");
    }

    fn mark_player_online_like_cpp<'a>(
        &'a self,
        _request: PlayerOnlineMarkRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PersistenceOutcomeLikeCpp> {
        panic!("unexpected mark_player_online_like_cpp in personal-money fixture");
    }

    fn save_account_collection_like_cpp<'a>(
        &'a self,
        _save: AccountCollectionSaveLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PersistenceOutcomeLikeCpp> {
        panic!("unexpected save_account_collection_like_cpp in personal-money fixture");
    }

    fn save_character_like_cpp<'a>(
        &'a self,
        _request: PlayerCharacterSaveRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PlayerCharacterSaveResultLikeCpp> {
        panic!("unexpected save_character_like_cpp in personal-money fixture");
    }

}
