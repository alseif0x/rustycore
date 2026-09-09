//! Shared character-handler test fixtures, part 1.
//!
//! Separated from the character_tests root under #662; every fixture is unchanged.

use super::*;

pub(super) fn install_world_query_catalogs_like_cpp(
    session: &mut WorldSession,
    creatures: impl IntoIterator<Item = CreatureQueryTemplateLikeCpp>,
    gameobjects: impl IntoIterator<Item = GameObjectQueryTemplateLikeCpp>,
    pages: impl IntoIterator<Item = PageTextLikeCpp>,
    gameobject_quest_items: impl IntoIterator<Item = (u32, u32, u32)>,
) {
    let quest_items = wow_data::GameObjectQuestItemStoreLikeCpp::from_rows_like_cpp(
        gameobject_quest_items,
        |_| true,
        |_| true,
    )
    .store;
    session.set_object_mgr_catalogs_like_cpp(Arc::new(crate::session::ObjectMgrCatalogsLikeCpp {
        creature: Arc::new(CreatureQueryCatalogLikeCpp::from_rows_like_cpp(
            creatures,
            [],
        )),
        gameobject: Arc::new(GameObjectQueryCatalogLikeCpp::from_rows_like_cpp(
            gameobjects,
            [],
        )),
        gameobject_quest_items: Arc::new(quest_items),
        page_text: Arc::new(PageTextCatalogLikeCpp::from_rows_like_cpp(pages, [])),
    }));
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum GossipCatalogRequestTraceLikeCpp {
    CreatureMenu(GossipCreatureMenuRequestLikeCpp),
    MenuTexts(GossipMenuCatalogRequestLikeCpp),
    NpcText(GossipNpcTextCatalogRequestLikeCpp),
    MenuOptions(GossipMenuCatalogRequestLikeCpp),
    BroadcastLocale(GossipBroadcastTextLocaleRequestLikeCpp),
}

pub(super) struct GossipCatalogPortFixtureLikeCpp {
    pub(super) requests: std::sync::Mutex<Vec<GossipCatalogRequestTraceLikeCpp>>,
    pub(super) creature_menu:
        std::sync::Mutex<std::collections::VecDeque<GossipCatalogReadOutcomeLikeCpp<u32>>>,
    pub(super) menu_texts:
        std::sync::Mutex<std::collections::VecDeque<GossipCatalogReadOutcomeLikeCpp<Vec<u32>>>>,
    pub(super) npc_text:
        std::sync::Mutex<std::collections::VecDeque<GossipCatalogReadOutcomeLikeCpp<i32>>>,
    pub(super) menu_options: std::sync::Mutex<
        std::collections::VecDeque<
            GossipCatalogReadOutcomeLikeCpp<Vec<GossipMenuOptionCatalogRowLikeCpp>>,
        >,
    >,
    pub(super) broadcast_locale:
        std::sync::Mutex<std::collections::VecDeque<GossipCatalogReadOutcomeLikeCpp<String>>>,
}

impl GossipCatalogPortFixtureLikeCpp {
    pub(super) fn new(
        creature_menu: impl IntoIterator<Item = GossipCatalogReadOutcomeLikeCpp<u32>>,
        menu_texts: impl IntoIterator<Item = GossipCatalogReadOutcomeLikeCpp<Vec<u32>>>,
        npc_text: impl IntoIterator<Item = GossipCatalogReadOutcomeLikeCpp<i32>>,
        menu_options: impl IntoIterator<
            Item = GossipCatalogReadOutcomeLikeCpp<Vec<GossipMenuOptionCatalogRowLikeCpp>>,
        >,
        broadcast_locale: impl IntoIterator<Item = GossipCatalogReadOutcomeLikeCpp<String>>,
    ) -> Arc<Self> {
        Arc::new(Self {
            requests: std::sync::Mutex::new(Vec::new()),
            creature_menu: std::sync::Mutex::new(creature_menu.into_iter().collect()),
            menu_texts: std::sync::Mutex::new(menu_texts.into_iter().collect()),
            npc_text: std::sync::Mutex::new(npc_text.into_iter().collect()),
            menu_options: std::sync::Mutex::new(menu_options.into_iter().collect()),
            broadcast_locale: std::sync::Mutex::new(broadcast_locale.into_iter().collect()),
        })
    }

    pub(super) fn requests(&self) -> Vec<GossipCatalogRequestTraceLikeCpp> {
        self.requests.lock().unwrap().clone()
    }
}

impl GossipCatalogPersistencePortLikeCpp for GossipCatalogPortFixtureLikeCpp {
    fn load_creature_gossip_menu_id_like_cpp<'a>(
        &'a self,
        request: GossipCreatureMenuRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, GossipCatalogReadOutcomeLikeCpp<u32>> {
        self.requests
            .lock()
            .unwrap()
            .push(GossipCatalogRequestTraceLikeCpp::CreatureMenu(request));
        let outcome = self
            .creature_menu
            .lock()
            .unwrap()
            .pop_front()
            .expect("one creature-menu outcome per request");
        Box::pin(async move { outcome })
    }

    fn load_gossip_menu_text_ids_like_cpp<'a>(
        &'a self,
        request: GossipMenuCatalogRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, GossipCatalogReadOutcomeLikeCpp<Vec<u32>>> {
        self.requests
            .lock()
            .unwrap()
            .push(GossipCatalogRequestTraceLikeCpp::MenuTexts(request));
        let outcome = self
            .menu_texts
            .lock()
            .unwrap()
            .pop_front()
            .expect("one menu-text outcome per request");
        Box::pin(async move { outcome })
    }

    fn load_npc_text_broadcast_id_like_cpp<'a>(
        &'a self,
        request: GossipNpcTextCatalogRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, GossipCatalogReadOutcomeLikeCpp<i32>> {
        self.requests
            .lock()
            .unwrap()
            .push(GossipCatalogRequestTraceLikeCpp::NpcText(request));
        let outcome = self
            .npc_text
            .lock()
            .unwrap()
            .pop_front()
            .expect("one npc-text outcome per request");
        Box::pin(async move { outcome })
    }

    fn load_gossip_menu_options_like_cpp<'a>(
        &'a self,
        request: GossipMenuCatalogRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<
        'a,
        GossipCatalogReadOutcomeLikeCpp<Vec<GossipMenuOptionCatalogRowLikeCpp>>,
    > {
        self.requests
            .lock()
            .unwrap()
            .push(GossipCatalogRequestTraceLikeCpp::MenuOptions(request));
        let outcome = self
            .menu_options
            .lock()
            .unwrap()
            .pop_front()
            .expect("one menu-options outcome per request");
        Box::pin(async move { outcome })
    }

    fn load_broadcast_text_locale_like_cpp<'a>(
        &'a self,
        request: GossipBroadcastTextLocaleRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, GossipCatalogReadOutcomeLikeCpp<String>> {
        self.requests
            .lock()
            .unwrap()
            .push(GossipCatalogRequestTraceLikeCpp::BroadcastLocale(request));
        let outcome = self
            .broadcast_locale
            .lock()
            .unwrap()
            .pop_front()
            .expect("one broadcast-locale outcome per request");
        Box::pin(async move { outcome })
    }
}

pub(super) struct PlayerNameQueryPortFixtureLikeCpp {
    pub(super) requests: std::sync::Mutex<Vec<PlayerNameQueryRequestLikeCpp>>,
    pub(super) outcomes:
        std::sync::Mutex<std::collections::VecDeque<PlayerNameQueryOutcomeLikeCpp>>,
}

impl PlayerNameQueryPortFixtureLikeCpp {
    pub(super) fn new(
        outcomes: impl IntoIterator<Item = PlayerNameQueryOutcomeLikeCpp>,
    ) -> Arc<Self> {
        Arc::new(Self {
            requests: std::sync::Mutex::new(Vec::new()),
            outcomes: std::sync::Mutex::new(outcomes.into_iter().collect()),
        })
    }

    pub(super) fn requests(&self) -> Vec<PlayerNameQueryRequestLikeCpp> {
        self.requests.lock().unwrap().clone()
    }
}

impl PlayerNameQueryPersistencePortLikeCpp for PlayerNameQueryPortFixtureLikeCpp {
    fn load_player_name_like_cpp<'a>(
        &'a self,
        request: PlayerNameQueryRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PlayerNameQueryOutcomeLikeCpp> {
        self.requests.lock().unwrap().push(request);
        let outcome = self
            .outcomes
            .lock()
            .unwrap()
            .pop_front()
            .expect("one player-name outcome per request");
        Box::pin(async move { outcome })
    }
}

pub(super) struct CharacterEnumerationPortFixtureLikeCpp {
    pub(super) requests: std::sync::Mutex<Vec<CharacterEnumerationRequestLikeCpp>>,
    pub(super) outcomes:
        std::sync::Mutex<std::collections::VecDeque<CharacterEnumerationLoadOutcomeLikeCpp>>,
}

impl CharacterEnumerationPortFixtureLikeCpp {
    pub(super) fn new(
        outcomes: impl IntoIterator<Item = CharacterEnumerationLoadOutcomeLikeCpp>,
    ) -> Arc<Self> {
        Arc::new(Self {
            requests: std::sync::Mutex::new(Vec::new()),
            outcomes: std::sync::Mutex::new(outcomes.into_iter().collect()),
        })
    }

    pub(super) fn requests(&self) -> Vec<CharacterEnumerationRequestLikeCpp> {
        self.requests.lock().unwrap().clone()
    }
}

impl CharacterEnumerationPersistencePortLikeCpp for CharacterEnumerationPortFixtureLikeCpp {
    fn load_character_enumeration_like_cpp<'a>(
        &'a self,
        request: CharacterEnumerationRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, CharacterEnumerationLoadOutcomeLikeCpp> {
        self.requests.lock().unwrap().push(request);
        let outcome = self
            .outcomes
            .lock()
            .unwrap()
            .pop_front()
            .expect("one character-enumeration outcome per request");
        Box::pin(async move { outcome })
    }
}

pub(super) struct MapCorpseLoadPortFixtureLikeCpp {
    pub(super) requests: std::sync::Mutex<Vec<MapCorpseLoadRequestLikeCpp>>,
    pub(super) outcomes:
        std::sync::Mutex<std::collections::VecDeque<PersistedMapCorpseLoadOutcomeLikeCpp>>,
}

impl MapCorpseLoadPortFixtureLikeCpp {
    pub(super) fn new(
        outcomes: impl IntoIterator<Item = PersistedMapCorpseLoadOutcomeLikeCpp>,
    ) -> Arc<Self> {
        Arc::new(Self {
            requests: std::sync::Mutex::new(Vec::new()),
            outcomes: std::sync::Mutex::new(outcomes.into_iter().collect()),
        })
    }

    pub(super) fn requests(&self) -> Vec<MapCorpseLoadRequestLikeCpp> {
        self.requests.lock().unwrap().clone()
    }
}

impl MapCorpsePersistencePortLikeCpp for MapCorpseLoadPortFixtureLikeCpp {
    fn load_map_corpses_like_cpp<'a>(
        &'a self,
        request: MapCorpseLoadRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PersistedMapCorpseLoadOutcomeLikeCpp> {
        self.requests.lock().unwrap().push(request);
        let outcome = self
            .outcomes
            .lock()
            .unwrap()
            .pop_front()
            .expect("one map-corpse outcome per request");
        Box::pin(async move { outcome })
    }
}

pub(super) struct CollectionLoadPortLikeCpp {
    pub(super) requests: std::sync::Mutex<Vec<AccountCollectionLoadRequestLikeCpp>>,
    pub(super) login_transport_requests:
        std::sync::Mutex<Vec<PlayerLoginTransportLoadRequestLikeCpp>>,
    pub(super) outcomes:
        std::sync::Mutex<std::collections::VecDeque<AccountCollectionLoadOutcomeLikeCpp>>,
    pub(super) initial_world_state_outcomes: std::sync::Mutex<
        std::collections::VecDeque<
            PersistenceFutureLikeCpp<'static, PlayerInitialWorldStatesLoadOutcomeLikeCpp>,
        >,
    >,
    pub(super) login_transport_outcomes:
        std::sync::Mutex<std::collections::VecDeque<PlayerLoginTransportLoadOutcomeLikeCpp>>,
    pub(super) bank_slot_purchase_requests:
        std::sync::Mutex<Vec<wow_persistence::PlayerBankSlotPurchaseRequestLikeCpp>>,
    pub(super) bank_slot_purchase_outcomes: std::sync::Mutex<
        std::collections::VecDeque<wow_persistence::PlayerMoneyTransactionOutcomeLikeCpp>,
    >,
}

impl CollectionLoadPortLikeCpp {
    pub(super) fn new(
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

    pub(super) fn for_initial_world_states(
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

    pub(super) fn for_login_transports(
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

    pub(super) fn for_bank_slot_purchase(
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

    pub(super) fn requests(&self) -> Vec<AccountCollectionLoadRequestLikeCpp> {
        self.requests.lock().unwrap().clone()
    }

    pub(super) fn login_transport_requests(&self) -> Vec<PlayerLoginTransportLoadRequestLikeCpp> {
        self.login_transport_requests.lock().unwrap().clone()
    }

    pub(super) fn bank_slot_purchase_requests(
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

pub(super) struct HomebindPortFixtureLikeCpp {
    pub(super) requests: std::sync::Mutex<Vec<PlayerHomebindPersistenceRequestLikeCpp>>,
    pub(super) outcomes: std::sync::Mutex<std::collections::VecDeque<PersistenceOutcomeLikeCpp>>,
}

impl HomebindPortFixtureLikeCpp {
    pub(super) fn new(outcomes: impl IntoIterator<Item = PersistenceOutcomeLikeCpp>) -> Arc<Self> {
        Arc::new(Self {
            requests: std::sync::Mutex::new(Vec::new()),
            outcomes: std::sync::Mutex::new(outcomes.into_iter().collect()),
        })
    }

    pub(super) fn requests(&self) -> Vec<PlayerHomebindPersistenceRequestLikeCpp> {
        self.requests.lock().unwrap().clone()
    }
}
