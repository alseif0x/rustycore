// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Auxiliary Player-login reads cross the typed lifecycle port.

use super::*;

use std::collections::VecDeque;
use std::sync::Mutex;
use wow_packet::packets::update::ChrCustomizationChoiceValuesUpdate;
use wow_persistence::{
    AccountCollectionLoadOutcomeLikeCpp, AccountCollectionLoadRequestLikeCpp,
    AccountCollectionSaveLikeCpp, PersistenceFutureLikeCpp, PersistenceOutcomeLikeCpp,
    PlayerCharacterSaveRequestLikeCpp, PlayerCharacterSaveResultLikeCpp,
    PlayerCustomizationLoadRowLikeCpp, PlayerHomebindPersistenceRequestLikeCpp,
    PlayerInstanceTimeRestrictionLoadRowLikeCpp, PlayerLifecyclePortLikeCpp,
    PlayerLoginAuxiliaryLoadOutcomeLikeCpp, PlayerLoginAuxiliaryLoadRequestLikeCpp,
    PlayerLoginAuxiliaryLoadedLikeCpp, PlayerLoginItemRepairRequestLikeCpp,
    PlayerLoginPetTalentResetOutcomeLikeCpp, PlayerOfflineMarkLikeCpp,
    PlayerOnlineMarkRequestLikeCpp, PlayerSpellChargeLoadRowLikeCpp,
    PlayerSpellCooldownLoadRowLikeCpp, PlayerTraitConfigLoadRowLikeCpp,
    PlayerTraitEntryLoadRowLikeCpp,
};

struct AuxiliaryLoadPortLikeCpp {
    requests: Mutex<Vec<PlayerLoginAuxiliaryLoadRequestLikeCpp>>,
    outcomes: Mutex<VecDeque<PlayerLoginAuxiliaryLoadOutcomeLikeCpp>>,
}

impl AuxiliaryLoadPortLikeCpp {
    fn new(
        outcomes: impl IntoIterator<Item = PlayerLoginAuxiliaryLoadOutcomeLikeCpp>,
    ) -> Arc<Self> {
        Arc::new(Self {
            requests: Mutex::new(Vec::new()),
            outcomes: Mutex::new(outcomes.into_iter().collect()),
        })
    }

    fn requests(&self) -> Vec<PlayerLoginAuxiliaryLoadRequestLikeCpp> {
        self.requests.lock().unwrap().clone()
    }
}

impl PlayerLifecyclePortLikeCpp for AuxiliaryLoadPortLikeCpp {
    fn mark_offline_like_cpp<'a>(
        &'a self,
        _mark: PlayerOfflineMarkLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PersistenceOutcomeLikeCpp> {
        Box::pin(async {
            PersistenceOutcomeLikeCpp::Failed {
                reason: "auxiliary-load-only fixture".to_owned(),
            }
        })
    }

    fn persist_homebind_like_cpp<'a>(
        &'a self,
        _request: PlayerHomebindPersistenceRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PersistenceOutcomeLikeCpp> {
        Box::pin(async {
            PersistenceOutcomeLikeCpp::Failed {
                reason: "auxiliary-load-only fixture".to_owned(),
            }
        })
    }

    fn clear_buyback_like_cpp<'a>(
        &'a self,
        _request: wow_persistence::PlayerBuybackClearRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PersistenceOutcomeLikeCpp> {
        Box::pin(async {
            PersistenceOutcomeLikeCpp::Failed {
                reason: "auxiliary-load-only fixture".to_owned(),
            }
        })
    }

    fn persist_money_transaction_like_cpp<'a>(
        &'a self,
        _request: wow_persistence::PlayerMoneyTransactionRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, wow_persistence::PlayerMoneyTransactionOutcomeLikeCpp> {
        Box::pin(async {
            wow_persistence::PlayerMoneyTransactionOutcomeLikeCpp::DefinitelyRolledBack {
                reason: "auxiliary-load-only fixture".to_owned(),
            }
        })
    }

    fn persist_bank_slot_purchase_like_cpp<'a>(
        &'a self,
        _request: wow_persistence::PlayerBankSlotPurchaseRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, wow_persistence::PlayerMoneyTransactionOutcomeLikeCpp> {
        Box::pin(async {
            wow_persistence::PlayerMoneyTransactionOutcomeLikeCpp::DefinitelyRolledBack {
                reason: "auxiliary-load-only fixture".to_owned(),
            }
        })
    }

    fn load_uncage_item_state_like_cpp<'a>(
        &'a self,
        _request: wow_persistence::PlayerUncageItemStateRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, wow_persistence::PlayerUncageItemStateLoadOutcomeLikeCpp>
    {
        Box::pin(async {
            wow_persistence::PlayerUncageItemStateLoadOutcomeLikeCpp::Failed {
                reason: "auxiliary-load-only fixture".to_owned(),
            }
        })
    }

    fn persist_durability_repair_like_cpp<'a>(
        &'a self,
        _repair: wow_persistence::PlayerDurabilityRepairSaveLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PersistenceOutcomeLikeCpp> {
        Box::pin(async {
            PersistenceOutcomeLikeCpp::Failed {
                reason: "auxiliary-load-only fixture".to_owned(),
            }
        })
    }

    fn persist_money_write_like_cpp<'a>(
        &'a self,
        _request: wow_persistence::PlayerMoneyWriteRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PersistenceOutcomeLikeCpp> {
        Box::pin(async {
            PersistenceOutcomeLikeCpp::Failed {
                reason: "auxiliary-load-only fixture".to_owned(),
            }
        })
    }

    fn persist_currency_save_like_cpp<'a>(
        &'a self,
        _request: wow_persistence::PlayerCurrencySaveRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PersistenceOutcomeLikeCpp> {
        Box::pin(async {
            PersistenceOutcomeLikeCpp::Failed {
                reason: "auxiliary-load-only fixture".to_owned(),
            }
        })
    }

    fn persist_talent_reset_like_cpp<'a>(
        &'a self,
        _request: wow_persistence::PlayerTalentResetPersistenceRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PersistenceOutcomeLikeCpp> {
        Box::pin(async {
            PersistenceOutcomeLikeCpp::Failed {
                reason: "auxiliary-load-only fixture".to_owned(),
            }
        })
    }

    fn persist_xp_like_cpp<'a>(
        &'a self,
        _request: wow_persistence::PlayerXpPersistenceRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PersistenceOutcomeLikeCpp> {
        Box::pin(async {
            PersistenceOutcomeLikeCpp::Failed {
                reason: "auxiliary-load-only fixture".to_owned(),
            }
        })
    }

    fn refresh_realm_character_count_like_cpp<'a>(
        &'a self,
        _request: wow_persistence::PlayerRealmCharacterCountRefreshRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PersistenceOutcomeLikeCpp> {
        Box::pin(async {
            PersistenceOutcomeLikeCpp::Failed {
                reason: "auxiliary-load-only fixture".to_owned(),
            }
        })
    }

    fn load_initial_world_states_like_cpp<'a>(
        &'a self,
    ) -> PersistenceFutureLikeCpp<'a, wow_persistence::PlayerInitialWorldStatesLoadOutcomeLikeCpp>
    {
        Box::pin(async {
            wow_persistence::PlayerInitialWorldStatesLoadOutcomeLikeCpp {
                templates: wow_persistence::PlayerInitialWorldStateRowsLikeCpp::Failed {
                    reason: "auxiliary-load-only fixture".to_owned(),
                },
                saved_values: wow_persistence::PlayerInitialWorldStateRowsLikeCpp::Failed {
                    reason: "auxiliary-load-only fixture".to_owned(),
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
                reason: "auxiliary-load-only fixture".to_owned(),
            }
        })
    }

    fn load_account_collection_like_cpp<'a>(
        &'a self,
        _request: AccountCollectionLoadRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, AccountCollectionLoadOutcomeLikeCpp> {
        Box::pin(async {
            AccountCollectionLoadOutcomeLikeCpp::Failed {
                reason: "auxiliary-load-only fixture".to_owned(),
            }
        })
    }

    fn load_character_base_like_cpp<'a>(
        &'a self,
        _request: wow_persistence::PlayerCharacterBaseLoadRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, wow_persistence::PlayerCharacterBaseLoadOutcomeLikeCpp> {
        Box::pin(async {
            wow_persistence::PlayerCharacterBaseLoadOutcomeLikeCpp::Failed {
                reason: "auxiliary-load-only fixture".to_owned(),
            }
        })
    }

    fn load_login_admission_like_cpp<'a>(
        &'a self,
        _request: wow_persistence::PlayerLoginAdmissionLoadRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, wow_persistence::PlayerLoginAdmissionLoadOutcomeLikeCpp> {
        Box::pin(async {
            wow_persistence::PlayerLoginAdmissionLoadOutcomeLikeCpp::Failed {
                reason: "auxiliary-load-only fixture".to_owned(),
            }
        })
    }

    fn load_login_auxiliary_like_cpp<'a>(
        &'a self,
        request: PlayerLoginAuxiliaryLoadRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PlayerLoginAuxiliaryLoadOutcomeLikeCpp> {
        self.requests.lock().unwrap().push(request);
        let outcome = self
            .outcomes
            .lock()
            .unwrap()
            .pop_front()
            .expect("one typed auxiliary outcome per request");
        Box::pin(async move { outcome })
    }

    fn save_account_collection_like_cpp<'a>(
        &'a self,
        _save: AccountCollectionSaveLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PersistenceOutcomeLikeCpp> {
        Box::pin(async {
            PersistenceOutcomeLikeCpp::Failed {
                reason: "auxiliary-load-only fixture".to_owned(),
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

    fn save_character_like_cpp<'a>(
        &'a self,
        request: PlayerCharacterSaveRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PlayerCharacterSaveResultLikeCpp> {
        let committed = request.committed_groups_like_cpp();
        Box::pin(async move {
            PlayerCharacterSaveResultLikeCpp {
                outcome: PersistenceOutcomeLikeCpp::Failed {
                    reason: "auxiliary-load-only fixture".to_owned(),
                },
                committed,
            }
        })
    }
}

#[path = "login_auxiliary_persistence/auxiliary_login_reads.rs"]
mod auxiliary_login_reads;
#[path = "login_auxiliary_persistence/spell_history.rs"]
mod spell_history;
#[path = "login_auxiliary_persistence/trait_configuration.rs"]
mod trait_configuration;
