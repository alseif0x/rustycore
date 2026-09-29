//! Feature-gated player-lifecycle port fixture shared by integration and private handler tests.

use std::sync::Arc;
use wow_persistence::*;

pub struct CollectionLoadPortLikeCpp {
    requests: std::sync::Mutex<Vec<AccountCollectionLoadRequestLikeCpp>>,
    login_transport_requests: std::sync::Mutex<Vec<PlayerLoginTransportLoadRequestLikeCpp>>,
    outcomes: std::sync::Mutex<std::collections::VecDeque<AccountCollectionLoadOutcomeLikeCpp>>,
    initial_world_state_outcomes: std::sync::Mutex<
        std::collections::VecDeque<
            PersistenceFutureLikeCpp<'static, PlayerInitialWorldStatesLoadOutcomeLikeCpp>,
        >,
    >,
    login_transport_outcomes:
        std::sync::Mutex<std::collections::VecDeque<PlayerLoginTransportLoadOutcomeLikeCpp>>,
    bank_slot_purchase_requests:
        std::sync::Mutex<Vec<wow_persistence::PlayerBankSlotPurchaseRequestLikeCpp>>,
    bank_slot_purchase_outcomes: std::sync::Mutex<
        std::collections::VecDeque<wow_persistence::PlayerMoneyTransactionOutcomeLikeCpp>,
    >,
}

impl CollectionLoadPortLikeCpp {
    pub fn new(
        outcomes: impl IntoIterator<Item = AccountCollectionLoadOutcomeLikeCpp>,
    ) -> Arc<Self> {
        Arc::new(Self {
            requests: std::sync::Mutex::new(Vec::new()),
            login_transport_requests: std::sync::Mutex::new(Vec::new()),
            outcomes: std::sync::Mutex::new(outcomes.into_iter().collect()),
            initial_world_state_outcomes: std::sync::Mutex::new(Default::default()),
            login_transport_outcomes: std::sync::Mutex::new(Default::default()),
            bank_slot_purchase_requests: std::sync::Mutex::new(Vec::new()),
            bank_slot_purchase_outcomes: std::sync::Mutex::new(Default::default()),
        })
    }

    pub fn for_initial_world_states(
        outcomes: impl IntoIterator<Item = PlayerInitialWorldStatesLoadOutcomeLikeCpp>,
    ) -> Arc<Self> {
        Arc::new(Self {
            requests: std::sync::Mutex::new(Vec::new()),
            login_transport_requests: std::sync::Mutex::new(Vec::new()),
            outcomes: std::sync::Mutex::new(Default::default()),
            initial_world_state_outcomes: std::sync::Mutex::new(
                outcomes
                    .into_iter()
                    .map(|outcome| {
                        Box::pin(async move { outcome }) as PersistenceFutureLikeCpp<'static, _>
                    })
                    .collect(),
            ),
            login_transport_outcomes: std::sync::Mutex::new(Default::default()),
            bank_slot_purchase_requests: std::sync::Mutex::new(Vec::new()),
            bank_slot_purchase_outcomes: std::sync::Mutex::new(Default::default()),
        })
    }

    pub fn for_login_transports(
        outcomes: impl IntoIterator<Item = PlayerLoginTransportLoadOutcomeLikeCpp>,
    ) -> Arc<Self> {
        Arc::new(Self {
            requests: std::sync::Mutex::new(Vec::new()),
            login_transport_requests: std::sync::Mutex::new(Vec::new()),
            outcomes: std::sync::Mutex::new(Default::default()),
            initial_world_state_outcomes: std::sync::Mutex::new(Default::default()),
            login_transport_outcomes: std::sync::Mutex::new(outcomes.into_iter().collect()),
            bank_slot_purchase_requests: std::sync::Mutex::new(Vec::new()),
            bank_slot_purchase_outcomes: std::sync::Mutex::new(Default::default()),
        })
    }

    pub fn for_bank_slot_purchase(
        outcomes: impl IntoIterator<Item = wow_persistence::PlayerMoneyTransactionOutcomeLikeCpp>,
    ) -> Arc<Self> {
        Arc::new(Self {
            requests: std::sync::Mutex::new(Vec::new()),
            login_transport_requests: std::sync::Mutex::new(Vec::new()),
            outcomes: std::sync::Mutex::new(Default::default()),
            initial_world_state_outcomes: std::sync::Mutex::new(Default::default()),
            login_transport_outcomes: std::sync::Mutex::new(Default::default()),
            bank_slot_purchase_requests: std::sync::Mutex::new(Vec::new()),
            bank_slot_purchase_outcomes: std::sync::Mutex::new(outcomes.into_iter().collect()),
        })
    }

    pub fn enqueue_initial_world_state_outcome(
        &self,
        outcome: PersistenceFutureLikeCpp<'static, PlayerInitialWorldStatesLoadOutcomeLikeCpp>,
    ) {
        self.initial_world_state_outcomes
            .lock()
            .unwrap()
            .push_back(outcome);
    }

    pub fn requests(&self) -> Vec<AccountCollectionLoadRequestLikeCpp> {
        self.requests.lock().unwrap().clone()
    }

    pub fn login_transport_requests(&self) -> Vec<PlayerLoginTransportLoadRequestLikeCpp> {
        self.login_transport_requests.lock().unwrap().clone()
    }

    pub fn bank_slot_purchase_requests(
        &self,
    ) -> Vec<wow_persistence::PlayerBankSlotPurchaseRequestLikeCpp> {
        self.bank_slot_purchase_requests.lock().unwrap().clone()
    }
}

impl PlayerLifecyclePortLikeCpp for CollectionLoadPortLikeCpp {
    fn mark_offline_like_cpp<'a>(
        &'a self,
        _mark: PlayerOfflineMarkLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PersistenceOutcomeLikeCpp> {
        Box::pin(async {
            PersistenceOutcomeLikeCpp::Failed {
                reason: "collection-load-only fixture".to_owned(),
            }
        })
    }

    fn persist_homebind_like_cpp<'a>(
        &'a self,
        _request: PlayerHomebindPersistenceRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PersistenceOutcomeLikeCpp> {
        Box::pin(async {
            PersistenceOutcomeLikeCpp::Failed {
                reason: "collection-load-only fixture".to_owned(),
            }
        })
    }

    fn clear_buyback_like_cpp<'a>(
        &'a self,
        _request: wow_persistence::PlayerBuybackClearRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PersistenceOutcomeLikeCpp> {
        Box::pin(async {
            PersistenceOutcomeLikeCpp::Failed {
                reason: "collection-load-only fixture".to_owned(),
            }
        })
    }

    fn persist_money_transaction_like_cpp<'a>(
        &'a self,
        _request: wow_persistence::PlayerMoneyTransactionRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, wow_persistence::PlayerMoneyTransactionOutcomeLikeCpp> {
        Box::pin(async {
            wow_persistence::PlayerMoneyTransactionOutcomeLikeCpp::DefinitelyRolledBack {
                reason: "collection-load-only fixture".to_owned(),
            }
        })
    }

    fn persist_bank_slot_purchase_like_cpp<'a>(
        &'a self,
        request: wow_persistence::PlayerBankSlotPurchaseRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, wow_persistence::PlayerMoneyTransactionOutcomeLikeCpp> {
        self.bank_slot_purchase_requests
            .lock()
            .unwrap()
            .push(request);
        let outcome = self
            .bank_slot_purchase_outcomes
            .lock()
            .unwrap()
            .pop_front()
            .expect("one bank-slot-purchase outcome per request");
        Box::pin(async move { outcome })
    }

    fn load_uncage_item_state_like_cpp<'a>(
        &'a self,
        _request: wow_persistence::PlayerUncageItemStateRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, wow_persistence::PlayerUncageItemStateLoadOutcomeLikeCpp>
    {
        Box::pin(async {
            wow_persistence::PlayerUncageItemStateLoadOutcomeLikeCpp::Failed {
                reason: "collection-load-only fixture".to_owned(),
            }
        })
    }

    fn persist_durability_repair_like_cpp<'a>(
        &'a self,
        _repair: wow_persistence::PlayerDurabilityRepairSaveLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PersistenceOutcomeLikeCpp> {
        Box::pin(async {
            PersistenceOutcomeLikeCpp::Failed {
                reason: "collection-load-only fixture".to_owned(),
            }
        })
    }

    fn persist_money_write_like_cpp<'a>(
        &'a self,
        _request: wow_persistence::PlayerMoneyWriteRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PersistenceOutcomeLikeCpp> {
        Box::pin(async {
            PersistenceOutcomeLikeCpp::Failed {
                reason: "collection-load-only fixture".to_owned(),
            }
        })
    }

    fn persist_currency_save_like_cpp<'a>(
        &'a self,
        _request: wow_persistence::PlayerCurrencySaveRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PersistenceOutcomeLikeCpp> {
        Box::pin(async {
            PersistenceOutcomeLikeCpp::Failed {
                reason: "collection-load-only fixture".to_owned(),
            }
        })
    }

    fn persist_talent_reset_like_cpp<'a>(
        &'a self,
        _request: wow_persistence::PlayerTalentResetPersistenceRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PersistenceOutcomeLikeCpp> {
        Box::pin(async {
            PersistenceOutcomeLikeCpp::Failed {
                reason: "collection-load-only fixture".to_owned(),
            }
        })
    }

    fn persist_xp_like_cpp<'a>(
        &'a self,
        _request: wow_persistence::PlayerXpPersistenceRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PersistenceOutcomeLikeCpp> {
        Box::pin(async {
            PersistenceOutcomeLikeCpp::Failed {
                reason: "collection-load-only fixture".to_owned(),
            }
        })
    }

    fn refresh_realm_character_count_like_cpp<'a>(
        &'a self,
        _request: wow_persistence::PlayerRealmCharacterCountRefreshRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PersistenceOutcomeLikeCpp> {
        Box::pin(async {
            PersistenceOutcomeLikeCpp::Failed {
                reason: "collection-load-only fixture".to_owned(),
            }
        })
    }

    fn load_initial_world_states_like_cpp<'a>(
        &'a self,
    ) -> PersistenceFutureLikeCpp<'a, PlayerInitialWorldStatesLoadOutcomeLikeCpp> {
        let outcome = self
            .initial_world_state_outcomes
            .lock()
            .unwrap()
            .pop_front()
            .expect("one typed initial-world-state outcome per request");
        outcome
    }

    fn load_login_transports_like_cpp<'a>(
        &'a self,
        request: PlayerLoginTransportLoadRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PlayerLoginTransportLoadOutcomeLikeCpp> {
        self.login_transport_requests.lock().unwrap().push(request);
        let outcome = self
            .login_transport_outcomes
            .lock()
            .unwrap()
            .pop_front()
            .expect("one typed login-transport outcome per request");
        Box::pin(async move { outcome })
    }

    fn load_account_collection_like_cpp<'a>(
        &'a self,
        request: AccountCollectionLoadRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, AccountCollectionLoadOutcomeLikeCpp> {
        self.requests.lock().unwrap().push(request);
        let outcome = self
            .outcomes
            .lock()
            .unwrap()
            .pop_front()
            .expect("one typed load outcome per request");
        Box::pin(async move { outcome })
    }

    fn load_character_base_like_cpp<'a>(
        &'a self,
        _request: wow_persistence::PlayerCharacterBaseLoadRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, wow_persistence::PlayerCharacterBaseLoadOutcomeLikeCpp> {
        Box::pin(async {
            wow_persistence::PlayerCharacterBaseLoadOutcomeLikeCpp::Failed {
                reason: "collection-load-only fixture".to_owned(),
            }
        })
    }

    fn load_login_admission_like_cpp<'a>(
        &'a self,
        _request: wow_persistence::PlayerLoginAdmissionLoadRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, wow_persistence::PlayerLoginAdmissionLoadOutcomeLikeCpp> {
        Box::pin(async {
            wow_persistence::PlayerLoginAdmissionLoadOutcomeLikeCpp::Failed {
                reason: "collection-load-only fixture".to_owned(),
            }
        })
    }

    fn load_login_auxiliary_like_cpp<'a>(
        &'a self,
        _request: PlayerLoginAuxiliaryLoadRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PlayerLoginAuxiliaryLoadOutcomeLikeCpp> {
        Box::pin(async {
            PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Failed {
                reason: "collection-load-only fixture".to_owned(),
            }
        })
    }

    fn persist_login_item_repairs_like_cpp<'a>(
        &'a self,
        _request: PlayerLoginItemRepairRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PersistenceOutcomeLikeCpp> {
        Box::pin(async { PersistenceOutcomeLikeCpp::Applied { rows: 0 } })
    }

    fn reset_login_pet_talents_like_cpp<'a>(
        &'a self,
        _player_guid: u64,
    ) -> PersistenceFutureLikeCpp<'a, PlayerLoginPetTalentResetOutcomeLikeCpp> {
        Box::pin(async {
            PlayerLoginPetTalentResetOutcomeLikeCpp {
                spell_delete: PersistenceOutcomeLikeCpp::Applied { rows: 0 },
                specialization_reset: PersistenceOutcomeLikeCpp::Applied { rows: 0 },
            }
        })
    }

    fn mark_player_online_like_cpp<'a>(
        &'a self,
        _request: PlayerOnlineMarkRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PersistenceOutcomeLikeCpp> {
        Box::pin(async { PersistenceOutcomeLikeCpp::Applied { rows: 0 } })
    }

    fn save_account_collection_like_cpp<'a>(
        &'a self,
        _save: AccountCollectionSaveLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PersistenceOutcomeLikeCpp> {
        Box::pin(async {
            PersistenceOutcomeLikeCpp::Failed {
                reason: "collection-load-only fixture".to_owned(),
            }
        })
    }

    fn save_character_like_cpp<'a>(
        &'a self,
        request: PlayerCharacterSaveRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PlayerCharacterSaveResultLikeCpp> {
        let committed = request.committed_groups_like_cpp();
        Box::pin(async move {
            PlayerCharacterSaveResultLikeCpp {
                outcome: PersistenceOutcomeLikeCpp::Failed {
                    reason: "collection-load-only fixture".to_owned(),
                },
                committed,
            }
        })
    }
}
