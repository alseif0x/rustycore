// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! The Session publishes offline state through the typed lifecycle port.
//!
//! These drive the real production methods against a recording port, so they
//! pin which marks are requested, against which logical database, and that
//! every outcome class is handled without panicking.

#[path = "lifecycle_persistence/account_collections.rs"]
mod account_collections;
#[path = "lifecycle_persistence/character_lifecycle.rs"]
mod character_lifecycle;
#[path = "lifecycle_persistence/player_persistence.rs"]
mod player_persistence;

use super::*;

use std::sync::Mutex;
use wow_persistence::{
    AccountCollectionLoadOutcomeLikeCpp, AccountCollectionLoadRequestLikeCpp,
    AccountCollectionSaveLikeCpp, AccountMaskBlockLikeCpp, LogicalDatabaseLikeCpp,
    PersistenceFutureLikeCpp, PersistenceOutcomeLikeCpp, PlayerBuybackClearRequestLikeCpp,
    PlayerCharacterSaveRequestLikeCpp, PlayerCharacterSaveResultLikeCpp,
    PlayerCurrencySaveRequestLikeCpp, PlayerDurabilityRepairSaveLikeCpp,
    PlayerHomebindPersistenceRequestLikeCpp, PlayerLifecyclePortLikeCpp,
    PlayerLoginAuxiliaryLoadOutcomeLikeCpp, PlayerLoginAuxiliaryLoadRequestLikeCpp,
    PlayerLoginItemRepairRequestLikeCpp, PlayerLoginPetTalentResetOutcomeLikeCpp,
    PlayerMoneyTransactionOutcomeLikeCpp, PlayerMoneyTransactionRequestLikeCpp,
    PlayerMoneyWriteRequestLikeCpp, PlayerOfflineMarkLikeCpp, PlayerOnlineMarkRequestLikeCpp,
    PlayerRealmCharacterCountRefreshRequestLikeCpp, PlayerSpellChargeSaveLikeCpp,
    PlayerSpellCooldownSaveLikeCpp, PlayerTalentResetPersistenceRequestLikeCpp,
    PlayerUncageItemStateLikeCpp, PlayerUncageItemStateLoadOutcomeLikeCpp,
    PlayerUncageItemStateRequestLikeCpp, PlayerXpPersistenceRequestLikeCpp,
};

struct RecordingPortLikeCpp {
    during_save: Mutex<Option<Box<dyn FnOnce() + Send>>>,
    save_pending: std::sync::atomic::AtomicBool,
    seen: Mutex<Vec<PlayerOfflineMarkLikeCpp>>,
    collection_loads: Mutex<Vec<AccountCollectionLoadRequestLikeCpp>>,
    collections: Mutex<Vec<AccountCollectionSaveLikeCpp>>,
    character_saves: Mutex<Vec<PlayerCharacterSaveRequestLikeCpp>>,
    buyback_clears: Mutex<Vec<PlayerBuybackClearRequestLikeCpp>>,
    money_transactions: Mutex<Vec<PlayerMoneyTransactionRequestLikeCpp>>,
    durability_repairs: Mutex<Vec<PlayerDurabilityRepairSaveLikeCpp>>,
    money_writes: Mutex<Vec<PlayerMoneyWriteRequestLikeCpp>>,
    currency_saves: Mutex<Vec<PlayerCurrencySaveRequestLikeCpp>>,
    talent_resets: Mutex<Vec<PlayerTalentResetPersistenceRequestLikeCpp>>,
    xp_saves: Mutex<Vec<PlayerXpPersistenceRequestLikeCpp>>,
    realm_character_count_refreshes: Mutex<Vec<PlayerRealmCharacterCountRefreshRequestLikeCpp>>,
    uncage_item_state_requests: Mutex<Vec<PlayerUncageItemStateRequestLikeCpp>>,
    uncage_item_state_outcome: Mutex<PlayerUncageItemStateLoadOutcomeLikeCpp>,
    outcome: PersistenceOutcomeLikeCpp,
}

impl RecordingPortLikeCpp {
    fn new(outcome: PersistenceOutcomeLikeCpp) -> Arc<Self> {
        Arc::new(Self {
            during_save: Mutex::new(None),
            save_pending: std::sync::atomic::AtomicBool::new(false),
            seen: Mutex::new(Vec::new()),
            collection_loads: Mutex::new(Vec::new()),
            collections: Mutex::new(Vec::new()),
            character_saves: Mutex::new(Vec::new()),
            buyback_clears: Mutex::new(Vec::new()),
            money_transactions: Mutex::new(Vec::new()),
            durability_repairs: Mutex::new(Vec::new()),
            money_writes: Mutex::new(Vec::new()),
            currency_saves: Mutex::new(Vec::new()),
            talent_resets: Mutex::new(Vec::new()),
            xp_saves: Mutex::new(Vec::new()),
            realm_character_count_refreshes: Mutex::new(Vec::new()),
            uncage_item_state_requests: Mutex::new(Vec::new()),
            uncage_item_state_outcome: Mutex::new(
                PlayerUncageItemStateLoadOutcomeLikeCpp::Failed {
                    reason: "no uncage-item-state fixture".to_owned(),
                },
            ),
            outcome,
        })
    }
    fn marks(&self) -> Vec<PlayerOfflineMarkLikeCpp> {
        self.seen.lock().unwrap().clone()
    }
    fn collection_saves(&self) -> Vec<AccountCollectionSaveLikeCpp> {
        self.collections.lock().unwrap().clone()
    }
    fn character_saves(&self) -> Vec<PlayerCharacterSaveRequestLikeCpp> {
        self.character_saves.lock().unwrap().clone()
    }
    fn buyback_clears(&self) -> Vec<PlayerBuybackClearRequestLikeCpp> {
        self.buyback_clears.lock().unwrap().clone()
    }
    fn money_transactions(&self) -> Vec<PlayerMoneyTransactionRequestLikeCpp> {
        self.money_transactions.lock().unwrap().clone()
    }
    fn durability_repairs(&self) -> Vec<PlayerDurabilityRepairSaveLikeCpp> {
        self.durability_repairs.lock().unwrap().clone()
    }
    fn money_writes(&self) -> Vec<PlayerMoneyWriteRequestLikeCpp> {
        self.money_writes.lock().unwrap().clone()
    }
    fn currency_saves(&self) -> Vec<PlayerCurrencySaveRequestLikeCpp> {
        self.currency_saves.lock().unwrap().clone()
    }
    fn talent_resets(&self) -> Vec<PlayerTalentResetPersistenceRequestLikeCpp> {
        self.talent_resets.lock().unwrap().clone()
    }
    fn xp_saves(&self) -> Vec<PlayerXpPersistenceRequestLikeCpp> {
        self.xp_saves.lock().unwrap().clone()
    }
    fn realm_character_count_refreshes(
        &self,
    ) -> Vec<PlayerRealmCharacterCountRefreshRequestLikeCpp> {
        self.realm_character_count_refreshes.lock().unwrap().clone()
    }
    fn set_uncage_item_state_outcome(&self, outcome: PlayerUncageItemStateLoadOutcomeLikeCpp) {
        *self.uncage_item_state_outcome.lock().unwrap() = outcome;
    }
    fn uncage_item_state_requests(&self) -> Vec<PlayerUncageItemStateRequestLikeCpp> {
        self.uncage_item_state_requests.lock().unwrap().clone()
    }
}

impl PlayerLifecyclePortLikeCpp for RecordingPortLikeCpp {
    fn mark_offline_like_cpp<'a>(
        &'a self,
        mark: PlayerOfflineMarkLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PersistenceOutcomeLikeCpp> {
        self.seen.lock().unwrap().push(mark);
        let outcome = self.outcome.clone();
        Box::pin(async move { outcome })
    }

    fn persist_homebind_like_cpp<'a>(
        &'a self,
        _request: PlayerHomebindPersistenceRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PersistenceOutcomeLikeCpp> {
        let outcome = self.outcome.clone();
        Box::pin(async move { outcome })
    }

    fn clear_buyback_like_cpp<'a>(
        &'a self,
        request: PlayerBuybackClearRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PersistenceOutcomeLikeCpp> {
        self.buyback_clears.lock().unwrap().push(request);
        let outcome = self.outcome.clone();
        Box::pin(async move { outcome })
    }

    fn persist_money_transaction_like_cpp<'a>(
        &'a self,
        request: PlayerMoneyTransactionRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PlayerMoneyTransactionOutcomeLikeCpp> {
        self.money_transactions.lock().unwrap().push(request);
        let outcome = match self.outcome.clone() {
            PersistenceOutcomeLikeCpp::Applied { .. } => {
                PlayerMoneyTransactionOutcomeLikeCpp::Committed
            }
            PersistenceOutcomeLikeCpp::Failed { reason } => {
                PlayerMoneyTransactionOutcomeLikeCpp::DefinitelyRolledBack { reason }
            }
            PersistenceOutcomeLikeCpp::Unknown { reason } => {
                PlayerMoneyTransactionOutcomeLikeCpp::CommitOutcomeUnknown {
                    reason,
                    observed_money: None,
                }
            }
        };
        Box::pin(async move { outcome })
    }

    fn persist_bank_slot_purchase_like_cpp<'a>(
        &'a self,
        _request: wow_persistence::PlayerBankSlotPurchaseRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PlayerMoneyTransactionOutcomeLikeCpp> {
        let outcome = match self.outcome.clone() {
            PersistenceOutcomeLikeCpp::Applied { .. } => {
                PlayerMoneyTransactionOutcomeLikeCpp::Committed
            }
            PersistenceOutcomeLikeCpp::Failed { reason } => {
                PlayerMoneyTransactionOutcomeLikeCpp::DefinitelyRolledBack { reason }
            }
            PersistenceOutcomeLikeCpp::Unknown { reason } => {
                PlayerMoneyTransactionOutcomeLikeCpp::CommitOutcomeUnknown {
                    reason,
                    observed_money: None,
                }
            }
        };
        Box::pin(async move { outcome })
    }

    fn load_uncage_item_state_like_cpp<'a>(
        &'a self,
        request: PlayerUncageItemStateRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PlayerUncageItemStateLoadOutcomeLikeCpp> {
        self.uncage_item_state_requests
            .lock()
            .unwrap()
            .push(request);
        let outcome = self.uncage_item_state_outcome.lock().unwrap().clone();
        Box::pin(async move { outcome })
    }

    fn persist_durability_repair_like_cpp<'a>(
        &'a self,
        repair: PlayerDurabilityRepairSaveLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PersistenceOutcomeLikeCpp> {
        self.durability_repairs.lock().unwrap().push(repair);
        let outcome = self.outcome.clone();
        Box::pin(async move { outcome })
    }

    fn persist_money_write_like_cpp<'a>(
        &'a self,
        request: PlayerMoneyWriteRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PersistenceOutcomeLikeCpp> {
        self.money_writes.lock().unwrap().push(request);
        let outcome = self.outcome.clone();
        Box::pin(async move { outcome })
    }

    fn persist_currency_save_like_cpp<'a>(
        &'a self,
        request: PlayerCurrencySaveRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PersistenceOutcomeLikeCpp> {
        self.currency_saves.lock().unwrap().push(request);
        let outcome = self.outcome.clone();
        Box::pin(async move { outcome })
    }

    fn persist_talent_reset_like_cpp<'a>(
        &'a self,
        request: PlayerTalentResetPersistenceRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PersistenceOutcomeLikeCpp> {
        self.talent_resets.lock().unwrap().push(request);
        let outcome = self.outcome.clone();
        Box::pin(async move { outcome })
    }

    fn persist_xp_like_cpp<'a>(
        &'a self,
        request: PlayerXpPersistenceRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PersistenceOutcomeLikeCpp> {
        self.xp_saves.lock().unwrap().push(request);
        let outcome = self.outcome.clone();
        Box::pin(async move { outcome })
    }

    fn refresh_realm_character_count_like_cpp<'a>(
        &'a self,
        request: PlayerRealmCharacterCountRefreshRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PersistenceOutcomeLikeCpp> {
        self.realm_character_count_refreshes
            .lock()
            .unwrap()
            .push(request);
        let outcome = self.outcome.clone();
        Box::pin(async move { outcome })
    }

    fn load_initial_world_states_like_cpp<'a>(
        &'a self,
    ) -> PersistenceFutureLikeCpp<'a, wow_persistence::PlayerInitialWorldStatesLoadOutcomeLikeCpp>
    {
        Box::pin(async {
            wow_persistence::PlayerInitialWorldStatesLoadOutcomeLikeCpp {
                templates: wow_persistence::PlayerInitialWorldStateRowsLikeCpp::Failed {
                    reason: "recording port has no initial-world-state fixture".to_owned(),
                },
                saved_values: wow_persistence::PlayerInitialWorldStateRowsLikeCpp::Failed {
                    reason: "recording port has no initial-world-state fixture".to_owned(),
                },
            }
        })
    }

    fn load_login_transports_like_cpp<'a>(
        &'a self,
        _request: wow_persistence::PlayerLoginTransportLoadRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, wow_persistence::PlayerLoginTransportLoadOutcomeLikeCpp> {
        Box::pin(async {
            wow_persistence::PlayerLoginTransportLoadOutcomeLikeCpp::Failed {
                reason: "recording port has no login-transport fixture".to_owned(),
            }
        })
    }

    fn load_account_collection_like_cpp<'a>(
        &'a self,
        request: AccountCollectionLoadRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, AccountCollectionLoadOutcomeLikeCpp> {
        self.collection_loads.lock().unwrap().push(request);
        Box::pin(async {
            AccountCollectionLoadOutcomeLikeCpp::Failed {
                reason: "recording port has no collection load fixture".to_owned(),
            }
        })
    }

    fn load_character_base_like_cpp<'a>(
        &'a self,
        _request: wow_persistence::PlayerCharacterBaseLoadRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, wow_persistence::PlayerCharacterBaseLoadOutcomeLikeCpp> {
        Box::pin(async {
            wow_persistence::PlayerCharacterBaseLoadOutcomeLikeCpp::Failed {
                reason: "recording port has no character-base fixture".to_owned(),
            }
        })
    }

    fn load_login_admission_like_cpp<'a>(
        &'a self,
        _request: wow_persistence::PlayerLoginAdmissionLoadRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, wow_persistence::PlayerLoginAdmissionLoadOutcomeLikeCpp> {
        Box::pin(async {
            wow_persistence::PlayerLoginAdmissionLoadOutcomeLikeCpp::Failed {
                reason: "recording port has no login-admission fixture".to_owned(),
            }
        })
    }

    fn load_login_auxiliary_like_cpp<'a>(
        &'a self,
        _request: PlayerLoginAuxiliaryLoadRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PlayerLoginAuxiliaryLoadOutcomeLikeCpp> {
        Box::pin(async {
            PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Failed {
                reason: "recording port has no auxiliary login fixture".to_owned(),
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
        save: AccountCollectionSaveLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PersistenceOutcomeLikeCpp> {
        self.collections.lock().unwrap().push(save);
        let outcome = self.outcome.clone();
        Box::pin(async move { outcome })
    }

    fn save_character_like_cpp<'a>(
        &'a self,
        request: PlayerCharacterSaveRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PlayerCharacterSaveResultLikeCpp> {
        let committed = request.committed_groups_like_cpp();
        self.character_saves.lock().unwrap().push(request);
        let outcome = self.outcome.clone();
        Box::pin(async move {
            let during_save = self.during_save.lock().unwrap().take();
            if let Some(during_save) = during_save {
                during_save();
            }
            if self.save_pending.load(std::sync::atomic::Ordering::SeqCst) {
                std::future::pending::<()>().await;
            }
            PlayerCharacterSaveResultLikeCpp { outcome, committed }
        })
    }
}

#[path = "lifecycle_persistence/deferred_transfer.rs"]
mod deferred_transfer;
#[path = "lifecycle_persistence/save_interleaving.rs"]
mod save_interleaving;

fn session_with_port(
    outcome: PersistenceOutcomeLikeCpp,
) -> (WorldSession, Arc<RecordingPortLikeCpp>) {
    let (mut session, _, _) = make_session();
    let port = RecordingPortLikeCpp::new(outcome);
    session.set_player_lifecycle_port_like_cpp(port.clone());
    (session, port)
}

fn talent_reset_session_with_port(
    outcome: PersistenceOutcomeLikeCpp,
    guid_counter: i64,
) -> (WorldSession, Arc<RecordingPortLikeCpp>) {
    let (mut session, port) = session_with_port(outcome);
    session.set_player_guid(Some(ObjectGuid::create_player(1, guid_counter)));
    session.set_player_gold_like_cpp(100_000);
    session.mark_represented_talents_loaded_like_cpp();
    session.set_represented_talent_reset_state_like_cpp(0, 0);
    (session, port)
}
