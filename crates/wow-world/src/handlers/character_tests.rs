//! Behaviour tests for [`super`].
//!
//! Extracted from `character.rs`. Moving tests moves no invariant: the
//! production module boundary, its visibility and its owners are untouched.
//!
//! Dedenting by one level lets rustfmt collapse some argument lists onto a single
//! line, which drops their trailing commas; that is the only difference from the
//! original text.

#![cfg(test)]

#[path = "character_tests/finalization.rs"]
mod finalization;

#[path = "character_tests/post_add_rest.rs"]
mod post_add_rest;
#[path = "character_tests/post_add_scaling.rs"]
mod post_add_scaling;

// Explicit database imports: this module reaches its parent through
// `use super::*`, and the persistence inventory cannot resolve a glob, so
// without these every database access in the file is invisible to the
// ratchet (see #277).

use super::*;
use crate::player_inventory_persistence_test_fixture::PlayerInventoryPersistencePortFixtureLikeCpp;
use crate::session::{
    AuraApplication, InventoryItem, RepresentedAuraEffectLikeCpp, RepresentedHomebindLikeCpp,
    RepresentedTaxiFlightNodeLikeCpp,
};
use wow_constants::{ItemClass, ItemSubClassWeapon, ServerOpcodes};
use wow_core::{EquipmentSetGuidGeneratorLikeCpp, ObjectGuidGenerator};
use wow_data::character_progression::{
    ChrClassesEntry, ChrClassesStore, ChrRacesEntry, ChrRacesStore,
};
use wow_data::item::ItemRecord;
use wow_data::item_stats::{ItemModType, ItemSparseTemplateEntry, ItemStatEntry, ItemStatsStore};
use wow_data::quest::{
    QUEST_ITEM_DROP_COUNT, QUEST_REWARD_CHOICES_COUNT, QUEST_REWARD_DISPLAY_SPELL_COUNT,
    QUEST_REWARD_ITEM_COUNT, QUEST_REWARD_REPUTATIONS_COUNT, QuestObjective, QuestStore,
    QuestTemplate,
};
use wow_data::{
    CreatureQueryCatalogLikeCpp, CreatureQueryDisplayLikeCpp, CreatureQueryTemplateLikeCpp,
    GameObjectQueryCatalogLikeCpp, GameObjectQueryTemplateLikeCpp, ItemChildEquipmentEntry,
    ItemChildEquipmentStore, PageTextCatalogLikeCpp, PageTextLikeCpp, PlayerConditionEntry,
    PlayerLevelStats, PlayerStatsStore, SpellMiscEntry, SpellMiscStore,
};
use wow_entities::{CHILD_EQUIPMENT_SLOT_START, EQUIPMENT_SLOT_MAINHAND};
use wow_packet::packets::loot::{
    CreatureLoot, LOOT_TYPE_CORPSE_LIKE_CPP, LootEntry, LootEntryFlags,
};
use wow_packet::packets::quest::quest_giver_status;
use wow_packet::{ServerPacket, WorldPacket};
use wow_persistence::{
    AccountCollectionLoadOutcomeLikeCpp, AccountCollectionLoadRequestLikeCpp,
    AccountCollectionLoadedLikeCpp, AccountCollectionRowsLikeCpp, AccountCollectionSaveLikeCpp,
    AccountHeirloomLoadRowLikeCpp, AccountMaskBlockLikeCpp, AccountMountLoadRowLikeCpp,
    AccountToyLoadRowLikeCpp, CharacterEnumerationLoadOutcomeLikeCpp,
    CharacterEnumerationPersistencePortLikeCpp, CharacterEnumerationRequestLikeCpp,
    CharacterEnumerationRowLikeCpp, GossipBroadcastTextLocaleRequestLikeCpp,
    GossipCatalogPersistencePortLikeCpp, GossipCatalogReadOutcomeLikeCpp,
    GossipCreatureMenuRequestLikeCpp, GossipMenuCatalogRequestLikeCpp,
    GossipMenuOptionCatalogRowLikeCpp, GossipNpcTextCatalogRequestLikeCpp,
    MapCorpseAuxiliaryLoadOutcomeLikeCpp,
    MapCorpseLoadOutcomeLikeCpp as PersistedMapCorpseLoadOutcomeLikeCpp,
    MapCorpseLoadRequestLikeCpp, MapCorpseLoadRowLikeCpp, MapCorpsePersistencePortLikeCpp,
    PersistenceFutureLikeCpp, PersistenceOutcomeLikeCpp, PlayerCharacterSaveRequestLikeCpp,
    PlayerCharacterSaveResultLikeCpp, PlayerHomebindPersistenceRequestLikeCpp,
    PlayerInitialWorldStateRowsLikeCpp, PlayerInitialWorldStateTemplateRowLikeCpp,
    PlayerInitialWorldStateValueRowLikeCpp, PlayerInitialWorldStatesLoadOutcomeLikeCpp,
    PlayerLifecyclePortLikeCpp, PlayerLoginAuxiliaryLoadOutcomeLikeCpp,
    PlayerLoginAuxiliaryLoadRequestLikeCpp, PlayerLoginItemRepairRequestLikeCpp,
    PlayerLoginPetTalentResetOutcomeLikeCpp, PlayerLoginTransportLoadOutcomeLikeCpp,
    PlayerLoginTransportLoadRequestLikeCpp, PlayerNameQueryOutcomeLikeCpp,
    PlayerNameQueryPersistencePortLikeCpp, PlayerNameQueryRequestLikeCpp,
    PlayerNameQueryRowLikeCpp, PlayerOfflineMarkLikeCpp, PlayerOnlineMarkRequestLikeCpp,
};

fn install_world_query_catalogs_like_cpp(
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
enum GossipCatalogRequestTraceLikeCpp {
    CreatureMenu(GossipCreatureMenuRequestLikeCpp),
    MenuTexts(GossipMenuCatalogRequestLikeCpp),
    NpcText(GossipNpcTextCatalogRequestLikeCpp),
    MenuOptions(GossipMenuCatalogRequestLikeCpp),
    BroadcastLocale(GossipBroadcastTextLocaleRequestLikeCpp),
}

struct GossipCatalogPortFixtureLikeCpp {
    requests: std::sync::Mutex<Vec<GossipCatalogRequestTraceLikeCpp>>,
    creature_menu:
        std::sync::Mutex<std::collections::VecDeque<GossipCatalogReadOutcomeLikeCpp<u32>>>,
    menu_texts:
        std::sync::Mutex<std::collections::VecDeque<GossipCatalogReadOutcomeLikeCpp<Vec<u32>>>>,
    npc_text: std::sync::Mutex<std::collections::VecDeque<GossipCatalogReadOutcomeLikeCpp<i32>>>,
    menu_options: std::sync::Mutex<
        std::collections::VecDeque<
            GossipCatalogReadOutcomeLikeCpp<Vec<GossipMenuOptionCatalogRowLikeCpp>>,
        >,
    >,
    broadcast_locale:
        std::sync::Mutex<std::collections::VecDeque<GossipCatalogReadOutcomeLikeCpp<String>>>,
}

impl GossipCatalogPortFixtureLikeCpp {
    fn new(
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

    fn requests(&self) -> Vec<GossipCatalogRequestTraceLikeCpp> {
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

struct PlayerNameQueryPortFixtureLikeCpp {
    requests: std::sync::Mutex<Vec<PlayerNameQueryRequestLikeCpp>>,
    outcomes: std::sync::Mutex<std::collections::VecDeque<PlayerNameQueryOutcomeLikeCpp>>,
}

impl PlayerNameQueryPortFixtureLikeCpp {
    fn new(outcomes: impl IntoIterator<Item = PlayerNameQueryOutcomeLikeCpp>) -> Arc<Self> {
        Arc::new(Self {
            requests: std::sync::Mutex::new(Vec::new()),
            outcomes: std::sync::Mutex::new(outcomes.into_iter().collect()),
        })
    }

    fn requests(&self) -> Vec<PlayerNameQueryRequestLikeCpp> {
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

struct CharacterEnumerationPortFixtureLikeCpp {
    requests: std::sync::Mutex<Vec<CharacterEnumerationRequestLikeCpp>>,
    outcomes: std::sync::Mutex<std::collections::VecDeque<CharacterEnumerationLoadOutcomeLikeCpp>>,
}

impl CharacterEnumerationPortFixtureLikeCpp {
    fn new(
        outcomes: impl IntoIterator<Item = CharacterEnumerationLoadOutcomeLikeCpp>,
    ) -> Arc<Self> {
        Arc::new(Self {
            requests: std::sync::Mutex::new(Vec::new()),
            outcomes: std::sync::Mutex::new(outcomes.into_iter().collect()),
        })
    }

    fn requests(&self) -> Vec<CharacterEnumerationRequestLikeCpp> {
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

struct MapCorpseLoadPortFixtureLikeCpp {
    requests: std::sync::Mutex<Vec<MapCorpseLoadRequestLikeCpp>>,
    outcomes: std::sync::Mutex<std::collections::VecDeque<PersistedMapCorpseLoadOutcomeLikeCpp>>,
}

impl MapCorpseLoadPortFixtureLikeCpp {
    fn new(outcomes: impl IntoIterator<Item = PersistedMapCorpseLoadOutcomeLikeCpp>) -> Arc<Self> {
        Arc::new(Self {
            requests: std::sync::Mutex::new(Vec::new()),
            outcomes: std::sync::Mutex::new(outcomes.into_iter().collect()),
        })
    }

    fn requests(&self) -> Vec<MapCorpseLoadRequestLikeCpp> {
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

struct CollectionLoadPortLikeCpp {
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
    fn new(outcomes: impl IntoIterator<Item = AccountCollectionLoadOutcomeLikeCpp>) -> Arc<Self> {
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

    fn for_initial_world_states(
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

    fn for_login_transports(
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

    fn for_bank_slot_purchase(
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

    fn requests(&self) -> Vec<AccountCollectionLoadRequestLikeCpp> {
        self.requests.lock().unwrap().clone()
    }

    fn login_transport_requests(&self) -> Vec<PlayerLoginTransportLoadRequestLikeCpp> {
        self.login_transport_requests.lock().unwrap().clone()
    }

    fn bank_slot_purchase_requests(
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

struct HomebindPortFixtureLikeCpp {
    requests: std::sync::Mutex<Vec<PlayerHomebindPersistenceRequestLikeCpp>>,
    outcomes: std::sync::Mutex<std::collections::VecDeque<PersistenceOutcomeLikeCpp>>,
}

impl HomebindPortFixtureLikeCpp {
    fn new(outcomes: impl IntoIterator<Item = PersistenceOutcomeLikeCpp>) -> Arc<Self> {
        Arc::new(Self {
            requests: std::sync::Mutex::new(Vec::new()),
            outcomes: std::sync::Mutex::new(outcomes.into_iter().collect()),
        })
    }

    fn requests(&self) -> Vec<PlayerHomebindPersistenceRequestLikeCpp> {
        self.requests.lock().unwrap().clone()
    }
}

impl PlayerLifecyclePortLikeCpp for HomebindPortFixtureLikeCpp {
    fn mark_offline_like_cpp<'a>(
        &'a self,
        _mark: PlayerOfflineMarkLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PersistenceOutcomeLikeCpp> {
        Box::pin(async { PersistenceOutcomeLikeCpp::Applied { rows: 0 } })
    }

    fn load_initial_world_states_like_cpp<'a>(
        &'a self,
    ) -> PersistenceFutureLikeCpp<'a, PlayerInitialWorldStatesLoadOutcomeLikeCpp> {
        Box::pin(async {
            PlayerInitialWorldStatesLoadOutcomeLikeCpp {
                templates: PlayerInitialWorldStateRowsLikeCpp::Failed {
                    reason: "homebind-only fixture".to_owned(),
                },
                saved_values: PlayerInitialWorldStateRowsLikeCpp::Failed {
                    reason: "homebind-only fixture".to_owned(),
                },
            }
        })
    }

    fn load_login_transports_like_cpp<'a>(
        &'a self,
        _request: PlayerLoginTransportLoadRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PlayerLoginTransportLoadOutcomeLikeCpp> {
        Box::pin(async {
            PlayerLoginTransportLoadOutcomeLikeCpp::Failed {
                reason: "homebind-only fixture".to_owned(),
            }
        })
    }

    fn persist_homebind_like_cpp<'a>(
        &'a self,
        request: PlayerHomebindPersistenceRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PersistenceOutcomeLikeCpp> {
        self.requests.lock().unwrap().push(request);
        let outcome = self
            .outcomes
            .lock()
            .unwrap()
            .pop_front()
            .expect("one typed homebind outcome per request");
        Box::pin(async move { outcome })
    }

    fn clear_buyback_like_cpp<'a>(
        &'a self,
        _request: wow_persistence::PlayerBuybackClearRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PersistenceOutcomeLikeCpp> {
        Box::pin(async { PersistenceOutcomeLikeCpp::Applied { rows: 0 } })
    }

    fn persist_money_transaction_like_cpp<'a>(
        &'a self,
        _request: wow_persistence::PlayerMoneyTransactionRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, wow_persistence::PlayerMoneyTransactionOutcomeLikeCpp> {
        Box::pin(async { wow_persistence::PlayerMoneyTransactionOutcomeLikeCpp::Committed })
    }

    fn persist_bank_slot_purchase_like_cpp<'a>(
        &'a self,
        _request: wow_persistence::PlayerBankSlotPurchaseRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, wow_persistence::PlayerMoneyTransactionOutcomeLikeCpp> {
        Box::pin(async { wow_persistence::PlayerMoneyTransactionOutcomeLikeCpp::Committed })
    }

    fn load_uncage_item_state_like_cpp<'a>(
        &'a self,
        _request: wow_persistence::PlayerUncageItemStateRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, wow_persistence::PlayerUncageItemStateLoadOutcomeLikeCpp>
    {
        Box::pin(async {
            wow_persistence::PlayerUncageItemStateLoadOutcomeLikeCpp::Failed {
                reason: "homebind-only fixture".to_owned(),
            }
        })
    }

    fn persist_durability_repair_like_cpp<'a>(
        &'a self,
        _repair: wow_persistence::PlayerDurabilityRepairSaveLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PersistenceOutcomeLikeCpp> {
        Box::pin(async { PersistenceOutcomeLikeCpp::Applied { rows: 1 } })
    }

    fn persist_money_write_like_cpp<'a>(
        &'a self,
        _request: wow_persistence::PlayerMoneyWriteRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PersistenceOutcomeLikeCpp> {
        Box::pin(async { PersistenceOutcomeLikeCpp::Applied { rows: 1 } })
    }

    fn persist_currency_save_like_cpp<'a>(
        &'a self,
        _request: wow_persistence::PlayerCurrencySaveRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PersistenceOutcomeLikeCpp> {
        Box::pin(async { PersistenceOutcomeLikeCpp::Applied { rows: 0 } })
    }

    fn persist_talent_reset_like_cpp<'a>(
        &'a self,
        _request: wow_persistence::PlayerTalentResetPersistenceRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PersistenceOutcomeLikeCpp> {
        Box::pin(async { PersistenceOutcomeLikeCpp::Applied { rows: 0 } })
    }

    fn persist_xp_like_cpp<'a>(
        &'a self,
        _request: wow_persistence::PlayerXpPersistenceRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PersistenceOutcomeLikeCpp> {
        Box::pin(async { PersistenceOutcomeLikeCpp::Applied { rows: 0 } })
    }

    fn refresh_realm_character_count_like_cpp<'a>(
        &'a self,
        _request: wow_persistence::PlayerRealmCharacterCountRefreshRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PersistenceOutcomeLikeCpp> {
        Box::pin(async { PersistenceOutcomeLikeCpp::Applied { rows: 0 } })
    }

    fn load_account_collection_like_cpp<'a>(
        &'a self,
        _request: AccountCollectionLoadRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, AccountCollectionLoadOutcomeLikeCpp> {
        Box::pin(async {
            AccountCollectionLoadOutcomeLikeCpp::Failed {
                reason: "homebind-only fixture".to_owned(),
            }
        })
    }

    fn load_character_base_like_cpp<'a>(
        &'a self,
        _request: wow_persistence::PlayerCharacterBaseLoadRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, wow_persistence::PlayerCharacterBaseLoadOutcomeLikeCpp> {
        Box::pin(async {
            wow_persistence::PlayerCharacterBaseLoadOutcomeLikeCpp::Failed {
                reason: "homebind-only fixture".to_owned(),
            }
        })
    }

    fn load_login_admission_like_cpp<'a>(
        &'a self,
        _request: wow_persistence::PlayerLoginAdmissionLoadRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, wow_persistence::PlayerLoginAdmissionLoadOutcomeLikeCpp> {
        Box::pin(async {
            wow_persistence::PlayerLoginAdmissionLoadOutcomeLikeCpp::Failed {
                reason: "homebind-only fixture".to_owned(),
            }
        })
    }

    fn load_login_auxiliary_like_cpp<'a>(
        &'a self,
        _request: PlayerLoginAuxiliaryLoadRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PlayerLoginAuxiliaryLoadOutcomeLikeCpp> {
        Box::pin(async {
            PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Failed {
                reason: "homebind-only fixture".to_owned(),
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
        Box::pin(async { PersistenceOutcomeLikeCpp::Applied { rows: 0 } })
    }

    fn save_character_like_cpp<'a>(
        &'a self,
        _request: PlayerCharacterSaveRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PlayerCharacterSaveResultLikeCpp> {
        Box::pin(async {
            PlayerCharacterSaveResultLikeCpp {
                outcome: PersistenceOutcomeLikeCpp::Applied { rows: 0 },
                committed: wow_persistence::PlayerCharacterCommittedGroupsLikeCpp::default(),
            }
        })
    }
}

fn test_item_enchantments_db_string(entries: &[(usize, i32, u32, i16)]) -> String {
    let mut fields = vec!["0".to_string(); wow_entities::MAX_ENCHANTMENT_SLOT * 3];
    for &(slot, id, duration, charges) in entries {
        let base = slot * 3;
        fields[base] = id.to_string();
        fields[base + 1] = duration.to_string();
        fields[base + 2] = charges.to_string();
    }
    fields.join(" ")
}

fn map_corpse_session_with_port_like_cpp(
    outcome: PersistedMapCorpseLoadOutcomeLikeCpp,
) -> (
    WorldSession,
    Arc<std::sync::Mutex<wow_map::MapManager>>,
    Arc<MapCorpseLoadPortFixtureLikeCpp>,
) {
    let port = MapCorpseLoadPortFixtureLikeCpp::new([outcome]);
    let mut manager = wow_map::MapManager::default();
    manager.create_world_map(571, 9);
    let manager = Arc::new(std::sync::Mutex::new(manager));
    let (mut session, _) = make_session_with_send_capacity(16);
    session.set_canonical_map_manager(Arc::clone(&manager));
    session.set_map_corpse_persistence_port_like_cpp(port.clone());
    (session, manager, port)
}

fn invalid_map_corpse_load_row_like_cpp() -> MapCorpseLoadRowLikeCpp {
    MapCorpseLoadRowLikeCpp {
        pos_x: 10.0,
        pos_y: 20.0,
        pos_z: 30.0,
        orientation: 1.5,
        map_id: 571,
        display_id: 12_345,
        item_cache: String::new(),
        race: 4,
        class: 1,
        sex: 0,
        flags: 0x20,
        dynamic_flags: 0x01,
        ghost_time: 1_000,
        corpse_type: 0,
        instance_id: 9,
        owner_guid: 77,
    }
}

pub(crate) fn make_session_with_send_capacity(
    capacity: usize,
) -> (WorldSession, flume::Receiver<Vec<u8>>) {
    let (_pkt_tx, pkt_rx) = flume::bounded::<WorldPacket>(1);
    let (send_tx, send_rx) = flume::bounded::<Vec<u8>>(capacity);
    let mut session = WorldSession::new(
        1,
        "TestAccount".into(),
        0,
        2,
        9,
        54261,
        vec![0u8; 40],
        "esES".into(),
        pkt_rx,
        send_tx,
    );
    session.set_item_guid_generator_like_cpp(Arc::new(ObjectGuidGenerator::new(HighGuid::Item, 1)));
    session.set_equipment_set_guid_generator_like_cpp(Arc::new(
        EquipmentSetGuidGeneratorLikeCpp::new(1),
    ));
    (session, send_rx)
}

fn inventory_failure_result(packet: &[u8]) -> i32 {
    assert_eq!(
        u16::from_le_bytes([packet[0], packet[1]]),
        ServerOpcodes::InventoryChangeFailure as u16
    );
    i32::from_le_bytes(packet[2..6].try_into().expect("inventory result bytes"))
}

fn run_login_grid_cleanup_test(test: impl FnOnce() + Send + 'static) {
    std::thread::Builder::new()
        .name("login-grid-cleanup".into())
        .stack_size(8 * 1024 * 1024)
        .spawn(test)
        .unwrap()
        .join()
        .unwrap();
}

fn make_session_with_realm_send_capacity(
    capacity: usize,
) -> (
    WorldSession,
    flume::Receiver<Vec<u8>>,
    flume::Receiver<Vec<u8>>,
) {
    let (mut session, instance_rx) = make_session_with_send_capacity(capacity);
    let (realm_tx, realm_rx) = flume::bounded::<Vec<u8>>(capacity);
    session.install_realm_send_channel_for_test(realm_tx);
    (session, instance_rx, realm_rx)
}

fn make_quest_status_session() -> (WorldSession, flume::Receiver<Vec<u8>>) {
    let (mut session, send_rx) = make_session_with_send_capacity(8);
    session.set_player_guid(Some(ObjectGuid::create_player(1, 42)));
    session.set_loaded_player_identity_like_cpp(571, 1, 1, 80, 0);
    session.set_player_faction_template_like_cpp(1);
    session.set_player_position_like_cpp(Position::new(10.0, 0.0, 0.0, 0.0));
    (session, send_rx)
}

fn strength_item_stats_store(entry_id: u32, amount: i16) -> ItemStatsStore {
    ItemStatsStore::from_parts(
        [(
            entry_id,
            ItemStatEntry {
                stats: std::array::from_fn(|i| {
                    if i == 0 {
                        (ItemModType::Strength as i8, amount)
                    } else {
                        (ItemModType::None as i8, 0)
                    }
                }),
                resistances: [0; 7],
                armor: 0,
            },
        )],
        [],
    )
}

fn set_priest_level80_stats(session: &mut WorldSession, base_mana: u32, intellect: u16) {
    session.set_spell_store(Arc::new(wow_data::SpellStore::new()));
    session.set_player_stats(Arc::new(PlayerStatsStore::from_entries([(
        (1, 5, 80),
        PlayerLevelStats {
            strength: 10,
            agility: 10,
            stamina: 10,
            intellect,
            spirit: 30,
            base_mana,
        },
    )])));
    session.set_chr_classes_store(Arc::new(ChrClassesStore::from_entries([chr_class_entry(
        5, 0,
    )])));
}

fn total_stat_percentage_spell_store_like_cpp(
    spell_id: i32,
    is_ability: bool,
) -> wow_data::SpellStore {
    let mut store = wow_data::SpellStore::new();
    store.insert(
        spell_id,
        wow_data::SpellInfo {
            spell_id,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
            effect_base_points: 0,
            effect_bonus_coefficient: 0.0,
            aura_type: Some(wow_data::spell::aura_types::SPELL_AURA_MOD_TOTAL_STAT_PERCENTAGE),
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: vec![wow_data::SpellEffectInfo {
                effect_index: 0,
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                effect_aura: wow_data::spell::aura_types::SPELL_AURA_MOD_TOTAL_STAT_PERCENTAGE,
                effect_base_points: 99,
                effect_die_sides: 1,
                effect_misc_value_2: 1 << 2,
                ..Default::default()
            }],
        },
    );
    let mut attributes = [0; 15];
    if is_ability {
        attributes[0] = wow_data::spell::attributes::SPELL_ATTR0_IS_ABILITY;
    }
    store.insert_spell_misc_attributes_like_cpp(spell_id, attributes);
    store
}

fn passive_combat_capability_spell_store_like_cpp(
    parry_spell_id: i32,
    block_spell_id: i32,
) -> wow_data::SpellStore {
    let mut store = wow_data::SpellStore::new();
    for (spell_id, effect) in [
        (
            parry_spell_id,
            wow_data::spell::spell_effect_types::SPELL_EFFECT_PARRY,
        ),
        (
            block_spell_id,
            wow_data::spell::spell_effect_types::SPELL_EFFECT_BLOCK,
        ),
    ] {
        store.insert(
            spell_id,
            wow_data::SpellInfo {
                spell_id,
                cast_time_ms: 0,
                cooldown_ms: 0,
                recovery_time_ms: 0,
                effect_type: effect,
                effect_base_points: 0,
                effect_bonus_coefficient: 0.0,
                aura_type: None,
                display_flags: 0,
                requires_spell_focus: 0,
                power_costs: Vec::new(),
                effects: vec![wow_data::SpellEffectInfo {
                    effect_index: 0,
                    effect,
                    ..Default::default()
                }],
            },
        );
        let mut attributes = [0; 15];
        attributes[0] = wow_data::spell::attributes::SPELL_ATTR0_PASSIVE;
        store.insert_spell_misc_attributes_like_cpp(spell_id, attributes);
    }
    store
}

fn attach_stat_update_player_with_mana(
    session: &mut WorldSession,
    player_guid: ObjectGuid,
    current_mana: i32,
    max_mana: i32,
) {
    attach_stat_update_player_with_mana_and_health(
        session,
        player_guid,
        current_mana,
        max_mana,
        100,
        100,
    );
}

fn attach_stat_update_player_with_mana_and_health(
    session: &mut WorldSession,
    player_guid: ObjectGuid,
    current_mana: i32,
    max_mana: i32,
    current_health: u32,
    max_health: u32,
) {
    let mut player = wow_entities::Player::new(Some(1), false);
    player
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(player_guid);
    player.unit_mut().world_mut().set_map(571, 0).unwrap();
    player
        .unit_mut()
        .world_mut()
        .relocate(Position::new(10.0, 20.0, 30.0, 0.0));
    player.unit_mut().set_max_health(u64::from(max_health));
    player.unit_mut().set_health(u64::from(current_health));
    player.unit_mut().set_power_index(PowerType::Mana, Some(0));
    player.unit_mut().set_max_power(PowerType::Mana, max_mana);
    player.unit_mut().set_power(PowerType::Mana, current_mana);

    let mut manager = wow_map::MapManager::default();
    manager
        .create_world_map(571, 0)
        .map_mut()
        .insert_map_object_record(wow_entities::MapObjectRecord::new_player(player).unwrap())
        .unwrap();
    attach_map_manager(session, manager);
    assert!(
        session.adopt_registered_canonical_player_fixture_like_cpp(),
        "stat fixture must register the same map-owned Player identity as production"
    );
}

fn drain_server_opcodes(send_rx: &flume::Receiver<Vec<u8>>) -> Vec<ServerOpcodes> {
    let mut opcodes = Vec::new();
    while let Ok(bytes) = send_rx.try_recv() {
        let packet = WorldPacket::from_bytes(&bytes);
        if let Some(opcode) = packet.server_opcode() {
            opcodes.push(opcode);
        }
    }
    opcodes
}

fn alter_appearance_packet(
    new_sex: u8,
    customized_race: i32,
    customized_chr_model_id: i32,
    customizations: &[(i32, i32)],
) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint32(customizations.len() as u32);
    pkt.write_uint8(new_sex);
    pkt.write_int32(customized_race);
    pkt.write_int32(customized_chr_model_id);
    for (option_id, choice_id) in customizations {
        pkt.write_int32(*option_id);
        pkt.write_int32(*choice_id);
    }
    pkt
}

fn confirm_barbers_choice_packet(customizations: &[(u32, u32)]) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint32(customizations.len() as u32);
    for (option_id, choice_id) in customizations {
        pkt.write_uint32(*option_id);
        pkt.write_uint32(*choice_id);
    }
    pkt
}

fn read_barber_shop_result(encoded: Vec<u8>) -> i32 {
    let mut packet = WorldPacket::new_client(encoded.as_slice().into());
    assert_eq!(
        packet.server_opcode(),
        Some(wow_constants::ServerOpcodes::BarberShopResult)
    );
    packet.skip_opcode();
    let result = packet.read_int32().unwrap();
    assert_eq!(packet.remaining(), 0);
    result
}

fn declined_names_packet(player: ObjectGuid, names: [&str; 5]) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_guid(&player);
    for name in names {
        pkt.write_bits(name.len() as u32, 7);
    }
    for name in names {
        pkt.write_string(name);
    }
    pkt
}

fn read_declined_names_result(encoded: Vec<u8>) -> (i32, ObjectGuid) {
    let mut packet = WorldPacket::new_client(encoded.as_slice().into());
    assert_eq!(
        packet.server_opcode(),
        Some(wow_constants::ServerOpcodes::SetPlayerDeclinedNamesResult)
    );
    packet.skip_opcode();
    let result = packet.read_int32().unwrap();
    let player = packet.read_guid().unwrap();
    assert_eq!(packet.remaining(), 0);
    (result, player)
}

fn assign_equipment_set_spec_packet(set_id: u32, spec_index: u32) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint32(set_id);
    pkt.write_uint32(spec_index);
    pkt
}

fn delete_equipment_set_packet(id: u64) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint64(id);
    pkt
}

fn use_equipment_set_packet(
    guid: u64,
    items: [ObjectGuid; wow_packet::packets::misc::EQUIPMENT_SET_SLOTS_LIKE_CPP],
) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_bits(0, 2);
    for (slot, item) in items.iter().enumerate() {
        pkt.write_guid(item);
        pkt.write_uint8(255);
        pkt.write_uint8(slot as u8);
    }
    pkt.write_uint64(guid);
    pkt
}

fn save_equipment_set_packet(
    set_type: i32,
    guid: u64,
    set_id: u32,
    ignore_mask: u32,
    pieces: [ObjectGuid; wow_packet::packets::misc::EQUIPMENT_SET_SLOTS_LIKE_CPP],
    appearances: [i32; wow_packet::packets::misc::EQUIPMENT_SET_SLOTS_LIKE_CPP],
    enchants: [i32; 2],
    assigned_spec_index: Option<i32>,
    name: &str,
    icon: &str,
) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_int32(set_type);
    pkt.write_uint64(guid);
    pkt.write_uint32(set_id);
    pkt.write_uint32(ignore_mask);
    for i in 0..wow_packet::packets::misc::EQUIPMENT_SET_SLOTS_LIKE_CPP {
        pkt.write_guid(&pieces[i]);
        pkt.write_int32(appearances[i]);
    }
    pkt.write_int32(enchants[0]);
    pkt.write_int32(enchants[1]);
    pkt.write_int32(0);
    pkt.write_int32(0);
    pkt.write_int32(0);
    pkt.write_int32(0);
    pkt.write_bit(assigned_spec_index.is_some());
    pkt.write_bits(name.len() as u32, 8);
    pkt.write_bits(icon.len() as u32, 9);
    if let Some(spec_index) = assigned_spec_index {
        pkt.write_int32(spec_index);
    }
    pkt.write_string(name);
    pkt.write_string(icon);
    pkt
}

fn read_equipment_set_id(encoded: Vec<u8>) -> (u64, i32, u32) {
    let mut packet = WorldPacket::new_client(encoded.as_slice().into());
    assert_eq!(
        packet.server_opcode(),
        Some(wow_constants::ServerOpcodes::EquipmentSetId)
    );
    packet.skip_opcode();
    let guid = packet.read_uint64().unwrap();
    let set_type = packet.read_int32().unwrap();
    let set_id = packet.read_uint32().unwrap();
    assert_eq!(packet.remaining(), 0);
    (guid, set_type, set_id)
}

fn read_use_equipment_set_result(encoded: Vec<u8>) -> (u64, u8) {
    let mut packet = WorldPacket::new_client(encoded.as_slice().into());
    assert_eq!(
        packet.server_opcode(),
        Some(wow_constants::ServerOpcodes::UseEquipmentSetResult)
    );
    packet.skip_opcode();
    let guid = packet.read_uint64().unwrap();
    let reason = packet.read_uint8().unwrap();
    assert_eq!(packet.remaining(), 0);
    (guid, reason)
}

fn install_child_equipment_fixture(
    session: &mut WorldSession,
    parent_entry: u32,
    child_entry: u32,
    child_slot: u8,
) {
    session.set_item_child_equipment_store(Arc::new(ItemChildEquipmentStore::from_entries([
        ItemChildEquipmentEntry {
            id: 1,
            child_item_id: child_entry as i32,
            child_item_equip_slot: child_slot,
            parent_item_id: parent_entry,
        },
    ])));
}

fn make_area_spirit_healer_session(
    capacity: usize,
) -> (
    WorldSession,
    flume::Receiver<Vec<u8>>,
    Arc<std::sync::Mutex<wow_map::MapManager>>,
) {
    let (mut session, send_rx) = make_session_with_send_capacity(capacity);
    let canonical = Arc::new(std::sync::Mutex::new(wow_map::MapManager::new(60_000, 10)));
    let player_guid = ObjectGuid::create_player(1, 42);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(crate::session::SessionPlayerController::new(
        player_guid,
        "Tester".to_string(),
        Position::new(0.0, 0.0, 0.0, 0.0),
        571,
        1,
        1,
        80,
        0,
    ));
    session.set_player_alive_like_cpp(false);
    (session, send_rx, canonical)
}

fn make_bank_slot_session(
    capacity: usize,
) -> (
    WorldSession,
    flume::Receiver<Vec<u8>>,
    Arc<std::sync::Mutex<wow_map::MapManager>>,
) {
    let (mut session, send_rx) = make_session_with_send_capacity(capacity);
    let canonical = Arc::new(std::sync::Mutex::new(wow_map::MapManager::new(60_000, 10)));
    let player_guid = ObjectGuid::create_player(1, 42);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(crate::session::SessionPlayerController::new(
        player_guid,
        "Tester".to_string(),
        Position::new(0.0, 0.0, 0.0, 0.0),
        571,
        1,
        1,
        80,
        0,
    ));
    session.set_player_faction_template_like_cpp(1);
    session.set_bank_bag_slot_prices_store(Arc::new(
        wow_data::BankBagSlotPricesStore::from_entries([
            wow_data::BankBagSlotPricesEntry { id: 1, cost: 100 },
            wow_data::BankBagSlotPricesEntry { id: 2, cost: 200 },
        ]),
    ));
    session.set_player_gold_like_cpp(150);
    session.set_player_bank_bag_slot_count_like_cpp(0);
    (session, send_rx, canonical)
}

fn insert_bank_test_player_in_world(
    session: &WorldSession,
    canonical: &Arc<std::sync::Mutex<wow_map::MapManager>>,
) {
    let player_guid = session.player_guid().expect("player guid");
    let mut player = wow_entities::Player::new(Some(1), false);
    player
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(player_guid);
    player.unit_mut().world_mut().set_map(571, 0).unwrap();
    player
        .unit_mut()
        .world_mut()
        .relocate(Position::new(0.0, 0.0, 0.0, 0.0));
    player.unit_mut().world_mut().object_mut().add_to_world();
    canonical
        .lock()
        .unwrap()
        .create_world_map(571, 0)
        .map_mut()
        .insert_map_object_record(wow_entities::MapObjectRecord::new_player(player).unwrap())
        .unwrap();
}

fn seed_represented_feign_death_like_cpp(session: &mut WorldSession, slot: u8) {
    let player_guid = session.player_guid().expect("player guid");
    session
        .mutate_canonical_player_like_cpp(|player| {
            player
                .unit_mut()
                .add_unit_state(wow_constants::unit::UnitState::DIED.bits());
        })
        .expect("canonical player");
    session.visible_auras.insert(
        slot,
        AuraApplication {
            spell_id: 5384,
            difficulty_id: 0,
            caster_guid: player_guid,
            slot,
            duration_total: 0,
            duration_remaining: 0,
            stack_count: 1,
            aura_flags: 0,
            effect_mask: 1,
            aura_interrupt_flags: 0,
            aura_interrupt_flags2: 0,
            represented_effect: Some(RepresentedAuraEffectLikeCpp::FeignDeath),
            represented_amount: 0,
            represented_effect_amounts: Vec::new(),
            represented_misc_value: None,
            represented_multiplier: 1.0,
            applied_at: std::time::Instant::now(),
        },
    );
}

fn canonical_player_has_died_state_like_cpp(session: &mut WorldSession) -> bool {
    session
        .mutate_canonical_player_like_cpp(|player| {
            player
                .unit()
                .has_unit_state(wow_constants::unit::UnitState::DIED.bits())
        })
        .expect("canonical player")
}

fn install_bind_spell_fixture(session: &mut WorldSession) {
    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        3286,
        wow_data::SpellInfo {
            spell_id: 3286,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: wow_data::spell::spell_effect_types::SPELL_EFFECT_BIND,
            effect_base_points: 0,
            effect_bonus_coefficient: 0.0,
            aura_type: None,
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: vec![wow_data::SpellEffectInfo {
                effect_index: 0,
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_BIND,
                ..Default::default()
            }],
        },
    );
    session.set_spell_store(Arc::new(spell_store));
}

fn make_binder_observer(
    guid_counter: u32,
    position: Position,
    innkeeper: ObjectGuid,
    visible: bool,
    registry: &Arc<crate::session::directory::PlayerRegistry>,
    canonical: &Arc<std::sync::Mutex<wow_map::MapManager>>,
) -> (WorldSession, flume::Receiver<Vec<u8>>) {
    let (mut observer, send_rx) = make_session_with_send_capacity(4);
    let guid = ObjectGuid::create_player(1, i64::from(guid_counter));
    observer.set_canonical_map_manager(Arc::clone(canonical));
    observer.attach_player_controller_like_cpp(crate::session::SessionPlayerController::new(
        guid,
        format!("Observer{guid_counter}"),
        position,
        571,
        1,
        1,
        80,
        0,
    ));
    observer.set_state(crate::session::SessionState::LoggedIn);
    observer.set_player_registry(Arc::clone(registry));
    if canonical
        .lock()
        .unwrap()
        .find_map(571, 0)
        .and_then(|map| map.map().get_typed_player(guid))
        .is_none()
    {
        let mut player = wow_entities::Player::new(Some(1), false);
        player.unit_mut().world_mut().object_mut().create(guid);
        player.unit_mut().world_mut().set_map(571, 0).unwrap();
        player.unit_mut().world_mut().relocate(position);
        player.unit_mut().world_mut().object_mut().add_to_world();
        canonical
            .lock()
            .unwrap()
            .create_world_map(571, 0)
            .map_mut()
            .insert_map_object_record(wow_entities::MapObjectRecord::new_player(player).unwrap())
            .unwrap();
    }
    if visible {
        observer.client_visible_guids_like_cpp.insert(innkeeper);
    }
    observer.register_in_player_registry();
    assert!(registry.fixture_update(guid, |placement| {
        placement.is_in_world = true;
        placement.position = position;
    }));
    (observer, send_rx)
}

fn install_bank_move_item_fixture(session: &mut WorldSession, entry_id: u32, max_stack_size: i32) {
    session.set_item_store(Arc::new(wow_data::ItemStore::from_records([ItemRecord {
        id: entry_id,
        class_id: ItemClass::Miscellaneous as u8,
        subclass_id: 0,
        material: 0,
        inventory_type: InventoryType::NonEquip as i8,
        sheathe_type: 0,
        random_select: 0,
        random_suffix_group_id: 0,
        scaling_stat_distribution_id: 0,
        scaling_stat_value: 0,
    }])));
    session.set_item_stats_store(Arc::new(ItemStatsStore::from_sparse_templates([(
        entry_id,
        ItemSparseTemplateEntry {
            flags: [0; 4],
            bag_family: 0,
            start_quest_id: 0,
            stackable: max_stack_size,
            max_count: 0,
            lock_id: 0,
            required_reputation_rank: 0,
            sell_price: 0,
            buy_price: 0,
            vendor_stack_count: 1,
            price_variance: 1.0,
            price_random_value: 1.0,
            max_durability: 0,
            other_faction_item_id: 0,
            content_tuning_id: 0,
            player_level_to_item_level_curve_id: 0,
            limit_category: 0,
            instance_bound: 0,
            zone_bound: [0; 2],
            required_reputation_faction: 0,
            allowable_class: -1,
            required_expansion: 0,
            bonding: ItemBondingType::None as u8,
            container_slots: 0,
            inventory_type: InventoryType::NonEquip as i8,
        },
    )])));
}

fn install_equippable_item_fixture(
    session: &mut WorldSession,
    entry_id: u32,
    inventory_type: InventoryType,
    strength: Option<i16>,
) {
    session.set_item_store(Arc::new(wow_data::ItemStore::from_records([ItemRecord {
        id: entry_id,
        class_id: ItemClass::Weapon as u8,
        subclass_id: ItemSubClassWeapon::Sword as u8,
        material: 0,
        inventory_type: inventory_type as i8,
        sheathe_type: 0,
        random_select: 0,
        random_suffix_group_id: 0,
        scaling_stat_distribution_id: 0,
        scaling_stat_value: 0,
    }])));
    let sparse = ItemSparseTemplateEntry {
        flags: [0; 4],
        bag_family: 0,
        start_quest_id: 0,
        stackable: 1,
        max_count: 0,
        lock_id: 0,
        required_reputation_rank: 0,
        sell_price: 0,
        buy_price: 0,
        vendor_stack_count: 1,
        price_variance: 1.0,
        price_random_value: 1.0,
        max_durability: 100,
        other_faction_item_id: 0,
        content_tuning_id: 0,
        player_level_to_item_level_curve_id: 0,
        limit_category: 0,
        instance_bound: 0,
        zone_bound: [0; 2],
        required_reputation_faction: 0,
        allowable_class: -1,
        required_expansion: 0,
        bonding: ItemBondingType::None as u8,
        container_slots: 0,
        inventory_type: inventory_type as i8,
    };
    let stats = strength.into_iter().map(|amount| {
        (
            entry_id,
            ItemStatEntry {
                stats: std::array::from_fn(|index| {
                    if index == 0 {
                        (ItemModType::Strength as i8, amount)
                    } else {
                        (ItemModType::None as i8, 0)
                    }
                }),
                resistances: [0; 7],
                armor: 0,
            },
        )
    });
    session.set_item_stats_store(Arc::new(
        ItemStatsStore::from_stats_sparse_and_random_property_templates(
            stats,
            [(entry_id, sparse)],
            [],
        ),
    ));
}

fn insert_bank_move_test_item(
    session: &mut WorldSession,
    slot: u8,
    entry_id: u32,
    db_guid: u64,
    count: u32,
) -> ObjectGuid {
    let player_guid = session.player_guid().expect("test player");
    let item_guid = ObjectGuid::create_item(1, db_guid as i64);
    session.insert_inventory_item_like_cpp(
        slot,
        InventoryItem {
            guid: item_guid,
            entry_id,
            db_guid,
            inventory_type: Some(InventoryType::NonEquip as u8),
        },
    );
    let item = session.make_inventory_item_object(
        item_guid,
        entry_id,
        player_guid,
        count,
        0,
        ItemContext::None,
        slot,
    );
    session.insert_inventory_item_object(item);
    item_guid
}

fn insert_equippable_test_item(
    session: &mut WorldSession,
    bag: u8,
    slot: u8,
    entry_id: u32,
    db_guid: u64,
    inventory_type: InventoryType,
) -> ObjectGuid {
    let player_guid = session.player_guid().expect("test player");
    let item_guid = ObjectGuid::create_item(1, db_guid as i64);
    if bag == INVENTORY_SLOT_BAG_0 {
        session.insert_inventory_item_like_cpp(
            slot,
            InventoryItem {
                guid: item_guid,
                entry_id,
                db_guid,
                inventory_type: Some(inventory_type as u8),
            },
        );
    }
    let mut item = session.make_inventory_item_object(
        item_guid,
        entry_id,
        player_guid,
        1,
        0,
        ItemContext::None,
        slot,
    );
    if bag != INVENTORY_SLOT_BAG_0 {
        let bag_guid = session
            .inventory_items_like_cpp()
            .get(&bag)
            .expect("represented bag")
            .guid;
        item.set_container_guid_and_slot(bag_guid, bag);
    }
    session.insert_inventory_item_object(item);
    item_guid
}

fn make_hearth_and_resurrect_session(area_flags: u32) -> (WorldSession, flume::Receiver<Vec<u8>>) {
    let (mut session, send_rx) = make_session_with_send_capacity(4);
    session.set_map_store(crate::teleport_test_fixtures::world_maps([571]));
    session.set_player_guid(Some(ObjectGuid::create_player(1, 42)));
    crate::canonical_player_access::install_canonical_player_owner_for_test(&mut session, 571, 0);
    session.set_loaded_player_identity_like_cpp(571, 1, 1, 80, 0);
    session.set_player_position_like_cpp(Position::new(1.0, 2.0, 3.0, 0.5));
    session.set_player_zone_area_like_cpp(10, 77);
    session.set_player_alive_like_cpp(false);
    session.set_area_table_store(Arc::new(wow_data::AreaTableStore::from_entries([
        wow_data::AreaTableEntry {
            id: 77,
            continent_id: 571,
            parent_area_id: 0,
            area_bit: -1,
            exploration_level: 0,
            mount_flags: 0,
            flags: area_flags,
        },
    ])));
    let _ = session.set_represented_homebind_like_cpp(RepresentedHomebindLikeCpp {
        map_id: 571,
        area_id: 77,
        position: Position::new(10.0, 20.0, 30.0, 1.5),
    });
    (session, send_rx)
}

fn chr_class_entry(id: u32, cinematic_sequence_id: u16) -> ChrClassesEntry {
    ChrClassesEntry {
        id,
        name: String::new(),
        filename: String::new(),
        name_male: String::new(),
        name_female: String::new(),
        pet_name_token: String::new(),
        create_screen_file_data_id: 0,
        select_screen_file_data_id: 0,
        icon_file_data_id: 0,
        low_res_screen_file_data_id: 0,
        flags: 0,
        starting_level: 1,
        armor_type_mask: 0,
        cinematic_sequence_id,
        default_spec: 0,
        has_strength_attack_bonus: 0,
        primary_stat_priority: 0,
        display_power: 0,
        ranged_attack_power_per_agility: 0,
        attack_power_per_agility: 0,
        attack_power_per_strength: 0,
        spell_class_set: 0,
        roles_mask: 0,
        damage_bonus_stat: 0,
        has_relic_slot: 0,
    }
}

fn chr_race_entry(id: u32, cinematic_sequence_id: i16) -> ChrRacesEntry {
    ChrRacesEntry {
        id,
        client_prefix: String::new(),
        client_file_string: String::new(),
        name: String::new(),
        flags: 0,
        male_display_id: 0,
        female_display_id: 0,
        high_res_male_display_id: 0,
        high_res_female_display_id: 0,
        res_sickness_spell_id: 0,
        splash_sound_id: 0,
        create_screen_file_data_id: 0,
        select_screen_file_data_id: 0,
        low_res_screen_file_data_id: 0,
        altered_form_start_visual_kit_id: [0; 3],
        altered_form_finish_visual_kit_id: [0; 3],
        heritage_armor_achievement_id: 0,
        starting_level: 1,
        ui_display_order: 0,
        playable_race_bit: 0,
        female_skeleton_file_data_id: 0,
        male_skeleton_file_data_id: 0,
        helmet_anim_scaling_race_id: 0,
        transmogrify_disabled_slot_mask: 0,
        faction_id: 0,
        cinematic_sequence_id,
        base_language: 0,
        creature_type: 0,
        alliance: 0,
        race_related: 0,
        unaltered_visual_race_id: 0,
        default_class_id: 0,
        neutral_race_id: 0,
    }
}

fn expected_trigger_cinematic(cinematic_id: u32) -> Vec<u8> {
    let mut expected = (wow_constants::ServerOpcodes::TriggerCinematic as u16)
        .to_le_bytes()
        .to_vec();
    expected.extend_from_slice(&cinematic_id.to_le_bytes());
    expected.extend_from_slice(&ObjectGuid::EMPTY.to_raw_bytes());
    expected
}

fn quest_template(id: u32) -> QuestTemplate {
    QuestTemplate {
        id,
        quest_type: 2,
        quest_level: 1,
        quest_max_scaling_level: 0,
        quest_package_id: 0,
        min_level: 1,
        quest_sort_id: 0,
        quest_info_id: 0,
        suggested_group_num: 0,
        reward_next_quest: 0,
        reward_xp_difficulty: 0,
        reward_xp_multiplier: 1.0,
        reward_money_difficulty: 0,
        reward_money_multiplier: 1.0,
        reward_bonus_money: 0,
        reward_display_spell: [0; QUEST_REWARD_DISPLAY_SPELL_COUNT],
        reward_spell: 0,
        reward_honor: 0,
        reward_title_id: 0,
        reward_skill_line_id: 0,
        reward_skill_points: 0,
        reward_mail_template_id: 0,
        reward_mail_delay_secs: 0,
        reward_mail_sender_entry: 0,
        reward_faction_ids: [0; QUEST_REWARD_REPUTATIONS_COUNT],
        reward_faction_values: [0; QUEST_REWARD_REPUTATIONS_COUNT],
        reward_faction_overrides: [0; QUEST_REWARD_REPUTATIONS_COUNT],
        reward_faction_cap_in: [0; QUEST_REWARD_REPUTATIONS_COUNT],
        reward_faction_flags: 0,
        source_item_id: 0,
        source_item_count: 0,
        source_spell_id: 0,
        limit_time_secs: 0,
        expansion: 0,
        flags: 0,
        flags_ex: 0,
        flags_ex2: 0,
        special_flags: 0,
        event_id_for_quest: 0,
        reward_items: [0; QUEST_REWARD_ITEM_COUNT],
        reward_amounts: [0; QUEST_REWARD_ITEM_COUNT],
        reward_currencies: [0; wow_data::quest::QUEST_REWARD_CURRENCY_COUNT],
        reward_currency_amounts: [0; wow_data::quest::QUEST_REWARD_CURRENCY_COUNT],
        item_drop: [0; QUEST_ITEM_DROP_COUNT],
        item_drop_quantity: [0; QUEST_ITEM_DROP_COUNT],
        log_title: format!("Quest {id}"),
        log_description: String::new(),
        quest_description: String::new(),
        area_description: String::new(),
        quest_completion_log: String::new(),
        objectives: Vec::new(),
        allowable_races: 0,
        allowable_classes: 0,
        max_level: 0,
        prev_quest_id: 0,
        next_quest_id: 0,
        exclusive_group: 0,
        breadcrumb_for_quest_id: 0,
        dependent_previous_quests: Vec::new(),
        dependent_breadcrumb_quests: Vec::new(),
        required_min_rep_faction: 0,
        required_min_rep_value: 0,
        required_max_rep_faction: 0,
        required_max_rep_value: 0,
        required_skill_id: 0,
        required_skill_points: 0,
        reward_choice_items: [(0, 0); QUEST_REWARD_CHOICES_COUNT],
        reward_choice_item_types: [0; QUEST_REWARD_CHOICES_COUNT],
    }
}

fn store_with_quests(ids: &[u32]) -> QuestStore {
    QuestStore::from_quests_like_cpp(ids.iter().copied().map(quest_template))
}

fn creature_guid(entry: u32, counter: i64) -> ObjectGuid {
    ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, entry, counter)
}

fn gameobject_guid(entry: u32, counter: i64) -> ObjectGuid {
    ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, entry, counter)
}

fn faction_template_entry(
    id: u32,
    faction: u16,
    faction_group: u8,
    friend_group: u8,
    enemy: u16,
) -> wow_data::progression_rewards::FactionTemplateEntry {
    let mut enemies = [0; 8];
    enemies[0] = enemy;
    wow_data::progression_rewards::FactionTemplateEntry {
        id,
        faction,
        flags: 0,
        faction_group,
        friend_group,
        enemy_group: 0,
        enemies,
        friend: [0; 8],
    }
}

fn insert_creature(manager: &mut wow_map::MapManager, guid: ObjectGuid, entry: u32) {
    let mut creature = wow_entities::Creature::new(false);
    creature.unit_mut().world_mut().object_mut().create(guid);
    creature
        .unit_mut()
        .world_mut()
        .object_mut()
        .set_entry(entry);
    creature.unit_mut().world_mut().set_map(571, 0).unwrap();
    creature
        .unit_mut()
        .world_mut()
        .relocate(Position::new(10.0, 0.0, 0.0, 0.0));
    creature.unit_mut().set_level(80);
    creature.set_ai_identity_runtime(1, 35, NPCFlags1::QUEST_GIVER.bits(), 0);
    manager
        .create_world_map(571, 0)
        .map_mut()
        .insert_map_object_record(wow_entities::MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();
}

fn insert_gameobject(manager: &mut wow_map::MapManager, guid: ObjectGuid, entry: u32) {
    let mut gameobject = wow_entities::GameObject::new();
    gameobject.world_mut().object_mut().create(guid);
    gameobject.world_mut().object_mut().set_entry(entry);
    gameobject.world_mut().set_map(571, 0).unwrap();
    gameobject
        .world_mut()
        .relocate(Position::new(10.0, 0.0, 0.0, 0.0));
    gameobject.world_mut().object_mut().add_to_world();
    manager
        .create_world_map(571, 0)
        .map_mut()
        .insert_map_object_record(
            wow_entities::MapObjectRecord::new_game_object(gameobject).unwrap(),
        )
        .unwrap();
}

fn insert_gossip_gameobject(
    manager: &Arc<std::sync::Mutex<wow_map::MapManager>>,
    guid: ObjectGuid,
    entry: u32,
    position: Position,
    go_type: u8,
    is_in_world: bool,
) {
    let mut gameobject = wow_entities::GameObject::new();
    gameobject.world_mut().object_mut().create(guid);
    gameobject.world_mut().object_mut().set_entry(entry);
    gameobject.world_mut().set_map(571, 0).unwrap();
    gameobject.world_mut().relocate(position);
    gameobject.set_go_type(go_type);
    if is_in_world {
        gameobject.world_mut().object_mut().add_to_world();
    }
    manager
        .lock()
        .unwrap()
        .create_world_map(571, 0)
        .map_mut()
        .insert_map_object_record(
            wow_entities::MapObjectRecord::new_game_object(gameobject).unwrap(),
        )
        .unwrap();
}

fn insert_area_spirit_healer_creature(
    manager: &Arc<std::sync::Mutex<wow_map::MapManager>>,
    guid: ObjectGuid,
    position: Position,
    npc_flags: u32,
    npc_flags2: u32,
) {
    let mut creature = wow_entities::Creature::new(false);
    creature.unit_mut().world_mut().object_mut().create(guid);
    creature.unit_mut().world_mut().object_mut().set_entry(91);
    creature.unit_mut().world_mut().set_map(571, 0).unwrap();
    creature.unit_mut().world_mut().relocate(position);
    creature.unit_mut().world_mut().set_combat_reach(1.0);
    creature.unit_mut().set_level(80);
    creature.unit_mut().set_max_health(100);
    creature.unit_mut().set_health(100);
    creature.set_ai_identity_runtime(1, 35, npc_flags, 0);
    creature.set_npc_flags2_runtime_like_cpp(npc_flags2);
    creature.unit_mut().world_mut().object_mut().add_to_world();

    manager
        .lock()
        .unwrap()
        .create_world_map(571, 0)
        .map_mut()
        .insert_map_object_record(wow_entities::MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();
}

fn insert_banker_creature(
    manager: &Arc<std::sync::Mutex<wow_map::MapManager>>,
    guid: ObjectGuid,
    npc_flags: u32,
) {
    let mut manager = manager.lock().unwrap();
    insert_canonical_creature_with_npc_flags(&mut manager, guid, 2456, npc_flags);
}

fn insert_canonical_creature_with_npc_flags(
    manager: &mut wow_map::MapManager,
    guid: ObjectGuid,
    entry: u32,
    npc_flags: u32,
) {
    let mut creature = wow_entities::Creature::new(false);
    creature.unit_mut().world_mut().object_mut().create(guid);
    creature
        .unit_mut()
        .world_mut()
        .object_mut()
        .set_entry(entry);
    creature.unit_mut().world_mut().set_map(571, 0).unwrap();
    creature
        .unit_mut()
        .world_mut()
        .relocate(Position::new(5.0, 0.0, 0.0, 0.0));
    creature.unit_mut().world_mut().set_combat_reach(1.0);
    creature.unit_mut().set_level(80);
    creature.unit_mut().set_max_health(100);
    creature.unit_mut().set_health(100);
    creature.set_ai_identity_runtime(1, 35, npc_flags, 0);
    creature.unit_mut().world_mut().object_mut().add_to_world();

    manager
        .create_world_map(571, 0)
        .map_mut()
        .insert_map_object_record(wow_entities::MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();
}

fn attach_map_manager(session: &mut WorldSession, manager: wow_map::MapManager) {
    session.set_canonical_map_manager(Arc::new(std::sync::Mutex::new(manager)));
}

fn attach_legacy_creature(
    session: &mut WorldSession,
    guid: ObjectGuid,
    entry: u32,
    npc_flags: u32,
) {
    let manager = Arc::new(std::sync::RwLock::new(crate::map_manager::MapManager::new()));
    manager.write().unwrap().add_creature(
        571,
        0,
        0,
        0,
        crate::map_manager::WorldCreature::new(
            guid,
            entry,
            Position::new(10.0, 0.0, 0.0, 0.0),
            100,
            80,
            1,
            2,
            0.0,
            1,
            35,
            npc_flags,
            0,
        ),
    );
    session.set_map_manager(manager);
}

fn mark_gameobject_questgiver(session: &mut WorldSession, guid: ObjectGuid) {
    let mut state = crate::session::RepresentedGameObjectUseState::default();
    state.go_type = Some(wow_entities::GAMEOBJECT_TYPE_QUESTGIVER as u8);
    session
        .represented_gameobject_use_states
        .insert(guid, state);
}

fn tracked_query_packet(guids: &[ObjectGuid]) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint32(guids.len() as u32);
    for guid in guids {
        pkt.write_packed_guid(guid);
    }
    pkt
}

fn quest_giver_hello_packet(guid: ObjectGuid) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_packed_guid(&guid);
    pkt.reset_read();
    pkt
}

fn gossip_message_counts(bytes: &[u8], expected_guid: ObjectGuid) -> (i32, i32) {
    assert_eq!(
        wow_packet::WorldPacket::from_bytes(bytes).server_opcode(),
        Some(ServerOpcodes::GossipMessage)
    );
    let mut pkt = WorldPacket::from_bytes(&bytes[2..]);
    assert_eq!(pkt.read_packed_guid().unwrap(), expected_guid);
    let _gossip_id = pkt.read_int32().unwrap();
    let _friendship_faction_id = pkt.read_int32().unwrap();
    let option_count = pkt.read_int32().unwrap();
    let quest_count = pkt.read_int32().unwrap();
    (option_count, quest_count)
}

fn gossip_catalog_option_like_cpp(
    menu_id: u32,
    option_id: u32,
    broadcast_text_id: u32,
) -> GossipMenuOptionCatalogRowLikeCpp {
    GossipMenuOptionCatalogRowLikeCpp {
        menu_id,
        gossip_option_id: 77,
        option_id,
        option_npc: 1,
        option_text: "Original option".to_owned(),
        option_broadcast_text_id: broadcast_text_id,
        language: 0,
        flags: 3,
        action_menu_id: 88,
        action_poi_id: 0,
        gossip_npc_option_id: None,
        box_coded: false,
        box_money: 25,
        box_text: "Confirm".to_owned(),
        box_broadcast_text_id: 0,
        spell_id: Some(99),
        override_icon_id: Some(4),
    }
}

fn recv_status_multiple(send_rx: &flume::Receiver<Vec<u8>>) -> Vec<(ObjectGuid, u64)> {
    let bytes = send_rx
        .try_recv()
        .expect("quest giver status multiple packet");
    assert_eq!(
        u16::from_le_bytes([bytes[0], bytes[1]]),
        wow_constants::ServerOpcodes::QuestGiverStatusMultiple as u16
    );
    let mut pkt = WorldPacket::from_bytes(&bytes[2..]);
    let count = pkt.read_int32().unwrap();
    assert!(count >= 0);
    let mut statuses = Vec::new();
    for _ in 0..count {
        statuses.push((pkt.read_packed_guid().unwrap(), pkt.read_uint64().unwrap()));
    }
    statuses
}

fn insert_cancel_temp_enchant_test_item(
    session: &mut WorldSession,
    player_guid: ObjectGuid,
    slot: u8,
    enchantment_id: i32,
) -> ObjectGuid {
    let item_guid = ObjectGuid::create_item(1, 70_000 + i64::from(slot));
    session.insert_inventory_item_like_cpp(
        slot,
        InventoryItem {
            guid: item_guid,
            entry_id: 700,
            db_guid: item_guid.counter() as u64,
            inventory_type: Some(InventoryType::Weapon as u8),
        },
    );
    let mut item = session.make_inventory_item_object(
        item_guid,
        700,
        player_guid,
        1,
        0,
        ItemContext::None,
        slot,
    );
    item.set_enchantment(
        EnchantmentSlot::EnhancementTemporary,
        enchantment_id,
        12_000,
        3,
    );
    session.insert_inventory_item_object(item);
    item_guid
}

fn character_enumeration_row_like_cpp() -> CharacterEnumerationRowLikeCpp {
    CharacterEnumerationRowLikeCpp {
        guid_low: 42,
        name: "PortBoundary".to_owned(),
        race: 1,
        class: 1,
        gender: 0,
        level: 20,
        zone: 12,
        map: 0,
        position_x: 1.0,
        position_y: 2.0,
        position_z: 3.0,
        guild_id: 0,
        player_flags: 0,
        at_login_flags: 0,
        pet_entry: 0,
        pet_display_id: 0,
        pet_level: 0,
        equipment_cache: String::new(),
        banned_guid: 0,
        list_slot: 0,
        last_played_time: 100,
        active_talent_group: 0,
        last_login_build: 54261,
        declined_genitive: "PortBoundaryGenitive".to_owned(),
    }
}

fn creature_query_catalog_row_like_cpp() -> CreatureQueryTemplateLikeCpp {
    CreatureQueryTemplateLikeCpp {
        entry: 42,
        name: "Localized creature".to_owned(),
        subname: "Localized title".to_owned(),
        title_alt: "Localized alternate".to_owned(),
        icon_name: "Directions".to_owned(),
        creature_type: 7,
        creature_family: 8,
        classification: 9,
        kill_credits: [10, 11],
        civilian: true,
        racial_leader: false,
        movement_id: 12,
        required_expansion: 3,
        vignette_id: 13,
        unit_class: 1,
        widget_set_id: 14,
        widget_set_unit_condition_id: 15,
        hp_multi: 1.5,
        energy_multi: 2.5,
        creature_difficulty_id: 16,
        type_flags: [17, 18],
        displays: vec![CreatureQueryDisplayLikeCpp {
            display_id: 19,
            scale: 0.75,
            probability: 0.25,
        }],
    }
}

fn gameobject_query_catalog_row_like_cpp() -> GameObjectQueryTemplateLikeCpp {
    let mut data = [0_i32; wow_data::WORLD_QUERY_GAMEOBJECT_DATA_COUNT_LIKE_CPP];
    data[0] = 7;
    data[34] = 41;
    GameObjectQueryTemplateLikeCpp {
        entry: 42,
        go_type: 3,
        display_id: 4,
        name: "Localized object".to_owned(),
        icon_name: "Directions".to_owned(),
        cast_bar_caption: "Opening".to_owned(),
        unk_string: "Unknown".to_owned(),
        size: 1.25,
        data,
        content_tuning_id: 42,
        min_money: 0,
        max_money: 0,
    }
}

fn enum_pet_template_store(
    entry: u32,
    family: u32,
) -> wow_data::CreatureTemplateLifecycleStoreLikeCpp {
    wow_data::CreatureTemplateLifecycleStoreLikeCpp::from_templates([
        wow_data::CreatureTemplateLifecycleRecordLikeCpp {
            entry,
            name: String::new(),
            ai_name: String::new(),
            script_name: String::new(),
            required_expansion: 0,
            faction: 0,
            npc_flags: 0,
            speed_walk: 1.0,
            speed_run: 1.0,
            scale: 1.0,
            classification: 0,
            damage_school: 0,
            unit_flags: 0,
            unit_flags2: 0,
            unit_flags3: 0,
            creature_type: 0,
            family,
            trainer_class: 0,
            unit_class: 0,
            vehicle_id: 0,
            movement_type: 0,
            ground_movement_type: 1,
            swim_allowed: true,
            flight_movement_type: 0,
            rooted: false,
            chase_movement_type: 0,
            random_movement_type: 0,
            interaction_pause_timer_ms: 180_000,
            flags_extra: 0,
            string_id: String::new(),
            regen_health: true,
            spells: [0; wow_data::MAX_CREATURE_SPELLS_LIKE_CPP],
            models: Vec::new(),
        },
    ])
}

#[path = "character_tests/creature.rs"]
mod creature;
#[path = "character_tests/gameobject.rs"]
mod gameobject;
#[path = "character_tests/group.rs"]
mod group;
#[path = "character_tests/instance.rs"]
mod instance;
#[path = "character_tests/item_1.rs"]
mod item_1;
#[path = "character_tests/item_2.rs"]
mod item_2;
#[path = "character_tests/item_3.rs"]
mod item_3;
#[path = "character_tests/item_4.rs"]
mod item_4;
#[path = "character_tests/login.rs"]
mod login;
#[path = "character_tests/loot.rs"]
mod loot;
#[path = "character_tests/misc_1.rs"]
mod misc_1;
#[path = "character_tests/misc_2.rs"]
mod misc_2;
#[path = "character_tests/misc_3.rs"]
mod misc_3;
#[path = "character_tests/misc_4.rs"]
mod misc_4;
#[path = "character_tests/movement.rs"]
mod movement;
#[path = "character_tests/persistence.rs"]
mod persistence;
#[path = "character_tests/pet.rs"]
mod pet;
#[path = "character_tests/quest.rs"]
mod quest;
#[path = "character_tests/skill.rs"]
mod skill;
#[path = "character_tests/spell.rs"]
mod spell;
#[path = "character_tests/visibility.rs"]
mod visibility;
