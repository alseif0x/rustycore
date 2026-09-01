//! Behaviour tests for [`super`].
//!
//! Extracted from `character.rs`. Moving tests moves no invariant: the
//! production module boundary, its visibility and its owners are untouched.
//!
//! Dedenting by one level lets rustfmt collapse some argument lists onto a single
//! line, which drops their trailing commas; that is the only difference from the
//! original text.

#![cfg(test)]

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
    ItemChildEquipmentEntry, ItemChildEquipmentStore, PlayerConditionEntry, PlayerLevelStats,
    PlayerStatsStore, SpellMiscEntry, SpellMiscStore,
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
    CharacterEnumerationRowLikeCpp, CreatureQueryCatalogOutcomeLikeCpp,
    CreatureQueryCatalogPersistencePortLikeCpp, CreatureQueryCatalogRequestLikeCpp,
    CreatureQueryCatalogRowLikeCpp, CreatureQueryDisplayRowLikeCpp,
    GameObjectQueryCatalogOutcomeLikeCpp, GameObjectQueryCatalogPersistencePortLikeCpp,
    GameObjectQueryCatalogRequestLikeCpp, GameObjectQueryCatalogRowLikeCpp,
    GossipBroadcastTextLocaleRequestLikeCpp, GossipCatalogPersistencePortLikeCpp,
    GossipCatalogReadOutcomeLikeCpp, GossipCreatureMenuRequestLikeCpp,
    GossipMenuCatalogRequestLikeCpp, GossipMenuOptionCatalogRowLikeCpp,
    GossipNpcTextCatalogRequestLikeCpp, MapCorpseAuxiliaryLoadOutcomeLikeCpp,
    MapCorpseLoadOutcomeLikeCpp as PersistedMapCorpseLoadOutcomeLikeCpp,
    MapCorpseLoadRequestLikeCpp, MapCorpseLoadRowLikeCpp, MapCorpsePersistencePortLikeCpp,
    PageTextCatalogDiagnosticLikeCpp, PageTextCatalogOutcomeLikeCpp,
    PageTextCatalogPersistencePortLikeCpp, PageTextCatalogRequestLikeCpp,
    PageTextCatalogRowLikeCpp, PersistenceFutureLikeCpp, PersistenceOutcomeLikeCpp,
    PlayerCharacterSaveRequestLikeCpp, PlayerCharacterSaveResultLikeCpp,
    PlayerHomebindPersistenceRequestLikeCpp, PlayerInitialWorldStateRowsLikeCpp,
    PlayerInitialWorldStateTemplateRowLikeCpp, PlayerInitialWorldStateValueRowLikeCpp,
    PlayerInitialWorldStatesLoadOutcomeLikeCpp, PlayerLifecyclePortLikeCpp,
    PlayerLoginAuxiliaryLoadOutcomeLikeCpp, PlayerLoginAuxiliaryLoadRequestLikeCpp,
    PlayerLoginItemRepairRequestLikeCpp, PlayerLoginPetTalentResetOutcomeLikeCpp,
    PlayerLoginTransportLoadOutcomeLikeCpp, PlayerLoginTransportLoadRequestLikeCpp,
    PlayerNameQueryOutcomeLikeCpp, PlayerNameQueryPersistencePortLikeCpp,
    PlayerNameQueryRequestLikeCpp, PlayerNameQueryRowLikeCpp, PlayerOfflineMarkLikeCpp,
    PlayerOnlineMarkRequestLikeCpp,
};

struct CreatureQueryCatalogPortFixtureLikeCpp {
    requests: std::sync::Mutex<Vec<CreatureQueryCatalogRequestLikeCpp>>,
    outcomes: std::sync::Mutex<std::collections::VecDeque<CreatureQueryCatalogOutcomeLikeCpp>>,
}

impl CreatureQueryCatalogPortFixtureLikeCpp {
    fn new(outcomes: impl IntoIterator<Item = CreatureQueryCatalogOutcomeLikeCpp>) -> Arc<Self> {
        Arc::new(Self {
            requests: std::sync::Mutex::new(Vec::new()),
            outcomes: std::sync::Mutex::new(outcomes.into_iter().collect()),
        })
    }

    fn requests(&self) -> Vec<CreatureQueryCatalogRequestLikeCpp> {
        self.requests.lock().unwrap().clone()
    }
}

impl CreatureQueryCatalogPersistencePortLikeCpp for CreatureQueryCatalogPortFixtureLikeCpp {
    fn load_creature_query_catalog_like_cpp<'a>(
        &'a self,
        request: CreatureQueryCatalogRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, CreatureQueryCatalogOutcomeLikeCpp> {
        self.requests.lock().unwrap().push(request);
        let outcome = self
            .outcomes
            .lock()
            .unwrap()
            .pop_front()
            .expect("one creature-query outcome per request");
        Box::pin(async move { outcome })
    }
}

struct GameObjectQueryCatalogPortFixtureLikeCpp {
    requests: std::sync::Mutex<Vec<GameObjectQueryCatalogRequestLikeCpp>>,
    outcomes: std::sync::Mutex<std::collections::VecDeque<GameObjectQueryCatalogOutcomeLikeCpp>>,
}

impl GameObjectQueryCatalogPortFixtureLikeCpp {
    fn new(outcomes: impl IntoIterator<Item = GameObjectQueryCatalogOutcomeLikeCpp>) -> Arc<Self> {
        Arc::new(Self {
            requests: std::sync::Mutex::new(Vec::new()),
            outcomes: std::sync::Mutex::new(outcomes.into_iter().collect()),
        })
    }

    fn requests(&self) -> Vec<GameObjectQueryCatalogRequestLikeCpp> {
        self.requests.lock().unwrap().clone()
    }
}

impl GameObjectQueryCatalogPersistencePortLikeCpp for GameObjectQueryCatalogPortFixtureLikeCpp {
    fn load_gameobject_query_catalog_like_cpp<'a>(
        &'a self,
        request: GameObjectQueryCatalogRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, GameObjectQueryCatalogOutcomeLikeCpp> {
        self.requests.lock().unwrap().push(request);
        let outcome = self
            .outcomes
            .lock()
            .unwrap()
            .pop_front()
            .expect("one gameobject-query outcome per request");
        Box::pin(async move { outcome })
    }
}

struct PageTextCatalogPortFixtureLikeCpp {
    requests: std::sync::Mutex<Vec<PageTextCatalogRequestLikeCpp>>,
    outcomes: std::sync::Mutex<std::collections::VecDeque<PageTextCatalogOutcomeLikeCpp>>,
}

impl PageTextCatalogPortFixtureLikeCpp {
    fn new(outcomes: impl IntoIterator<Item = PageTextCatalogOutcomeLikeCpp>) -> Arc<Self> {
        Arc::new(Self {
            requests: std::sync::Mutex::new(Vec::new()),
            outcomes: std::sync::Mutex::new(outcomes.into_iter().collect()),
        })
    }

    fn requests(&self) -> Vec<PageTextCatalogRequestLikeCpp> {
        self.requests.lock().unwrap().clone()
    }
}

impl PageTextCatalogPersistencePortLikeCpp for PageTextCatalogPortFixtureLikeCpp {
    fn load_page_text_catalog_like_cpp<'a>(
        &'a self,
        request: PageTextCatalogRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PageTextCatalogOutcomeLikeCpp> {
        self.requests.lock().unwrap().push(request);
        let outcome = self
            .outcomes
            .lock()
            .unwrap()
            .pop_front()
            .expect("one page-text outcome per request");
        Box::pin(async move { outcome })
    }
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
    initial_world_state_outcomes:
        std::sync::Mutex<std::collections::VecDeque<PlayerInitialWorldStatesLoadOutcomeLikeCpp>>,
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
            initial_world_state_outcomes: std::sync::Mutex::new(outcomes.into_iter().collect()),
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
        Box::pin(async move { outcome })
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

#[test]
fn continue_login_no_longer_names_the_core_character_statement() {
    let source = include_str!("character/world_entry.rs");
    let (_, tail) = source
        .split_once("pub async fn handle_continue_player_login")
        .expect("continue-login handler starts");
    let (handler, _) = tail
        .split_once("pub(super) fn player_login_combat_stats_like_cpp")
        .expect("continue-login handler ends before packet helper");
    assert!(handler.contains("load_character_base_like_cpp"));
    assert!(handler.contains("PlayerCharacterBaseLoadOutcomeLikeCpp::Loaded(Some(row))"));
    assert!(handler.contains("PlayerCharacterBaseLoadOutcomeLikeCpp::Loaded(None)"));
    assert!(handler.contains("PlayerCharacterBaseLoadOutcomeLikeCpp::Failed { reason }"));
    assert!(!handler.contains("prepare(CharStatements::SEL_CHARACTER)"));
}

#[test]
fn continue_login_no_longer_names_location_or_guild_statements() {
    let source = include_str!("character/world_entry.rs");
    let (_, tail) = source
        .split_once("pub async fn handle_continue_player_login")
        .expect("continue-login handler starts");
    let (handler, _) = tail
        .split_once("pub(super) fn player_login_combat_stats_like_cpp")
        .expect("continue-login handler ends before packet helper");
    assert!(handler.contains("load_login_admission_like_cpp"));
    assert!(handler.contains("PlayerLoginAdmissionLoadedLikeCpp::BattlegroundLocation"));
    assert!(handler.contains("PlayerLoginAdmissionLoadedLikeCpp::HomebindLocation"));
    assert!(handler.contains("PlayerLoginAdmissionLoadedLikeCpp::GuildMembership"));
    for statement in [
        "CharStatements::SEL_CHARACTER_BGDATA",
        "CharStatements::SEL_CHARACTER_HOMEBIND",
        "CharStatements::SEL_GUILD_MEMBER",
    ] {
        assert!(
            !handler.contains(statement),
            "handler still names {statement}"
        );
    }
}

#[test]
fn continue_login_inventory_reads_cross_the_typed_lifecycle_port() {
    let source = include_str!("character/world_entry.rs");
    let (_, tail) = source
        .split_once("pub async fn handle_continue_player_login")
        .expect("continue-login handler starts");
    let (handler, _) = tail
        .split_once("pub(super) fn player_login_combat_stats_like_cpp")
        .expect("continue-login handler ends before packet helper");

    assert!(handler.contains("PlayerLoginAuxiliaryLoadRequestLikeCpp::EquipmentInventory"));
    assert!(handler.contains("PlayerLoginAuxiliaryLoadRequestLikeCpp::BagInventory"));
    assert!(handler.contains("PlayerLoginAuxiliaryLoadRequestLikeCpp::VoidStorage"));
    for statement in [
        "CharStatements::SEL_CHAR_EQUIPMENT",
        "CharStatements::SEL_CHAR_BAG_CONTENTS",
        "CharStatements::SEL_CHAR_VOID_STORAGE",
    ] {
        assert!(
            !handler.contains(statement),
            "handler still names {statement}"
        );
    }
}

#[test]
fn continue_login_item_repairs_cross_the_typed_lifecycle_port() {
    let source = include_str!("character/world_entry.rs");
    let (_, tail) = source
        .split_once("pub async fn handle_continue_player_login")
        .expect("continue-login handler starts");
    let (handler, _) = tail
        .split_once("pub(super) fn player_login_combat_stats_like_cpp")
        .expect("continue-login handler ends before packet helper");

    assert_eq!(
        handler
            .matches("persist_login_item_repairs_like_cpp")
            .count(),
        2
    );
    assert!(handler.contains("PlayerLoginItemRepairActionLikeCpp::ClearRefundable"));
    assert!(handler.contains("PlayerLoginItemRepairActionLikeCpp::NormalizeOnLoad"));
    assert!(!handler.contains("SqlTransaction::new()"));
    for statement in [
        "CharStatements::DEL_ITEM_REFUND_INSTANCE",
        "CharStatements::UPD_ITEM_INSTANCE_FLAGS",
        "CharStatements::UPD_ITEM_INSTANCE_ON_LOAD",
    ] {
        assert!(
            !handler.contains(statement),
            "handler still names {statement}"
        );
    }
}

#[test]
fn continue_login_has_no_concrete_persistence_after_remaining_writes_move() {
    let source = include_str!("character/world_entry.rs");
    let (_, tail) = source
        .split_once("pub async fn handle_continue_player_login")
        .expect("continue-login handler starts");
    let (handler, _) = tail
        .split_once("pub(super) fn player_login_combat_stats_like_cpp")
        .expect("continue-login handler ends before packet helper");

    assert!(handler.contains("reset_login_pet_talents_like_cpp"));
    assert!(handler.contains("mark_player_online_like_cpp"));
    for concrete in [
        "char_db()",
        "CharStatements::",
        "SqlTransaction::",
        ".prepare(",
        ".execute(",
        ".commit_transaction(",
    ] {
        assert!(
            !handler.contains(concrete),
            "continue-login still contains concrete persistence: {concrete}"
        );
    }
}

#[tokio::test]
async fn account_collection_loads_cross_the_typed_port_in_login_order_like_cpp() {
    let port = CollectionLoadPortLikeCpp::new([
        AccountCollectionLoadOutcomeLikeCpp::Loaded(AccountCollectionLoadedLikeCpp::Toys(vec![
            AccountToyLoadRowLikeCpp {
                item_id: -1,
                is_favorite: true,
                has_fanfare: true,
            },
            AccountToyLoadRowLikeCpp {
                item_id: 42,
                is_favorite: true,
                has_fanfare: false,
            },
        ])),
        AccountCollectionLoadOutcomeLikeCpp::Loaded(AccountCollectionLoadedLikeCpp::Heirlooms(
            vec![
                AccountHeirloomLoadRowLikeCpp {
                    item_id: -1,
                    flags: 1,
                },
                AccountHeirloomLoadRowLikeCpp {
                    item_id: 43,
                    flags: 2,
                },
            ],
        )),
        AccountCollectionLoadOutcomeLikeCpp::Loaded(
            AccountCollectionLoadedLikeCpp::ItemAppearances {
                appearance_blocks: AccountCollectionRowsLikeCpp::Loaded(vec![
                    AccountMaskBlockLikeCpp {
                        block_index: 1,
                        mask: 2,
                    },
                ]),
                favorite_appearance_ids: AccountCollectionRowsLikeCpp::Loaded(vec![9]),
            },
        ),
        AccountCollectionLoadOutcomeLikeCpp::Loaded(
            AccountCollectionLoadedLikeCpp::TransmogIllusions {
                illusion_blocks: vec![AccountMaskBlockLikeCpp {
                    block_index: 2,
                    mask: 4,
                }],
            },
        ),
        AccountCollectionLoadOutcomeLikeCpp::Loaded(AccountCollectionLoadedLikeCpp::Mounts(vec![
            AccountMountLoadRowLikeCpp {
                mount_spell_id: -1,
                flags: 1,
            },
            AccountMountLoadRowLikeCpp {
                mount_spell_id: 123,
                flags: 2,
            },
        ])),
    ]);
    let (mut session, _) = make_session_with_send_capacity(1);
    session.set_battlenet_account_id(77);
    session.set_player_lifecycle_port_like_cpp(port.clone());

    session.load_account_toys_like_cpp().await;
    session.load_account_heirlooms_like_cpp().await;
    session.load_account_item_appearances_like_cpp().await;
    session.load_account_transmog_illusions_like_cpp().await;
    assert!(session.load_account_mounts_like_cpp().await);

    assert_eq!(session.account_toy_rows_like_cpp(), vec![(42, true, false)]);
    assert_eq!(session.account_heirloom_rows_like_cpp(), vec![(43, 2)]);
    assert_eq!(
        session.account_transmog_active_player_rows_like_cpp(),
        vec![0, 2]
    );
    assert!(!session.set_appearance_is_favorite_like_cpp(9, true));
    assert!(session.has_transmog_illusion_like_cpp(66));
    assert_eq!(
        session.account_mount_rows_like_cpp(),
        vec![AccountMount {
            spell_id: 123,
            flags: 2,
        }]
    );
    assert_eq!(
        port.requests(),
        vec![
            AccountCollectionLoadRequestLikeCpp::Toys {
                bnet_account_id: 77
            },
            AccountCollectionLoadRequestLikeCpp::Heirlooms {
                bnet_account_id: 77
            },
            AccountCollectionLoadRequestLikeCpp::ItemAppearances {
                bnet_account_id: 77
            },
            AccountCollectionLoadRequestLikeCpp::TransmogIllusions {
                bnet_account_id: 77
            },
            AccountCollectionLoadRequestLikeCpp::Mounts {
                bnet_account_id: 77
            },
        ]
    );
}

#[tokio::test]
async fn account_item_appearance_load_preserves_independent_query_failure_like_cpp() {
    let port = CollectionLoadPortLikeCpp::new([AccountCollectionLoadOutcomeLikeCpp::Loaded(
        AccountCollectionLoadedLikeCpp::ItemAppearances {
            appearance_blocks: AccountCollectionRowsLikeCpp::Failed {
                reason: "appearance read failed".to_owned(),
            },
            favorite_appearance_ids: AccountCollectionRowsLikeCpp::Loaded(vec![91]),
        },
    )]);
    let (mut session, _) = make_session_with_send_capacity(1);
    session.set_battlenet_account_id(77);
    session.set_player_lifecycle_port_like_cpp(port);

    session.load_account_item_appearances_like_cpp().await;

    assert!(
        session
            .account_transmog_active_player_rows_like_cpp()
            .is_empty()
    );
    assert!(!session.set_appearance_is_favorite_like_cpp(91, true));
}

#[tokio::test]
async fn account_collection_empty_and_adapter_failure_clear_represented_rows_like_cpp() {
    let port = CollectionLoadPortLikeCpp::new([
        AccountCollectionLoadOutcomeLikeCpp::Loaded(AccountCollectionLoadedLikeCpp::Toys(
            Vec::new(),
        )),
        AccountCollectionLoadOutcomeLikeCpp::Failed {
            reason: "heirloom read failed".to_owned(),
        },
    ]);
    let (mut session, _) = make_session_with_send_capacity(1);
    session.set_battlenet_account_id(77);
    session.load_represented_account_toys_like_cpp([(42, true, false)]);
    session.load_represented_account_heirlooms_like_cpp([(43, 2)]);
    session.set_player_lifecycle_port_like_cpp(port.clone());

    session.load_account_toys_like_cpp().await;
    session.load_account_heirlooms_like_cpp().await;

    assert!(session.account_toy_rows_like_cpp().is_empty());
    assert!(session.account_heirloom_rows_like_cpp().is_empty());
    assert_eq!(
        port.requests(),
        vec![
            AccountCollectionLoadRequestLikeCpp::Toys {
                bnet_account_id: 77
            },
            AccountCollectionLoadRequestLikeCpp::Heirlooms {
                bnet_account_id: 77
            },
        ]
    );
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

#[test]
fn void_storage_login_context_preserves_cpp_field_five_bug() {
    let selected_context_column = ItemContext::Timewalking as u8;

    assert_eq!(
        void_storage_login_context_like_cpp(29, selected_context_column),
        29
    );
    assert_ne!(29, selected_context_column);
}

#[test]
fn init_self_orders_transport_attached_player_and_fellow_passenger_like_cpp() {
    let player_guid = ObjectGuid::create_player(1, 42);
    let passenger_guid = ObjectGuid::create_player(1, 43);
    let transport_guid = ObjectGuid::create_transport(HighGuid::Transport, 7_001);
    let mut player_update = UpdateObject::create_player(
        player_guid,
        1,
        8,
        0,
        80,
        49,
        &Position::ZERO,
        571,
        0,
        true,
        [(0, 0, 0); 19],
        [ObjectGuid::EMPTY; 141],
        PlayerCombatStats::default(),
        Vec::new(),
        0,
        Vec::new(),
    );
    player_update.set_player_movement_transport_like_cpp(TransportInfo {
        guid: transport_guid,
        x: 1.0,
        y: 2.0,
        z: 3.0,
        o: 0.5,
        seat: -1,
        time: 0,
        prev_time: None,
        vehicle_id: None,
    });
    let transport_block = UpdateObject::create_transport_block(
        GameObjectCreateData {
            guid: transport_guid,
            entry: 1,
            dynamic_flags: 0,
            display_id: 2,
            go_type: GAMEOBJECT_TYPE_MAP_OBJ_TRANSPORT_LIKE_CPP,
            position: Position::ZERO,
            rotation: [0.0, 0.0, 0.0, 1.0],
            anim_progress: 255,
            state: wow_entities::GoState::Ready as i8,
            art_kit: 0,
            created_by: ObjectGuid::EMPTY,
            faction_template: 0,
            gameobject_flags: 0x0010_0028,
            world_effect_id: 0,
            scale: 1.0,
            level: 1_000,
            parent_rotation: [0.0, 0.0, 0.0, 1.0],
        },
        0,
    );
    let mut passenger_update = UpdateObject::create_player(
        passenger_guid,
        1,
        8,
        0,
        80,
        49,
        &Position::ZERO,
        571,
        0,
        false,
        [(0, 0, 0); 19],
        [ObjectGuid::EMPTY; 141],
        PlayerCombatStats::default(),
        Vec::new(),
        0,
        Vec::new(),
    );
    let passenger_block = passenger_update
        .blocks
        .pop()
        .expect("fellow passenger CREATE");

    assert_eq!(
        compose_init_self_create_blocks_like_cpp(
            &mut player_update,
            Vec::new(),
            Some((transport_guid, transport_block)),
            vec![passenger_block],
        ),
        Some(transport_guid)
    );
    assert_eq!(player_update.num_updates, 3);
    assert!(matches!(
        player_update.blocks.first(),
        Some(UpdateBlock::CreateTransport { guid, .. }) if *guid == transport_guid
    ));
    let Some(UpdateBlock::CreateObject {
        guid,
        movement: Some(movement),
        is_self: true,
        ..
    }) = player_update.blocks.get(1)
    else {
        panic!("expected attached self player after its transport");
    };
    assert_eq!(*guid, player_guid);
    assert_eq!(
        movement.transport.as_ref().map(|transport| transport.guid),
        Some(transport_guid)
    );
    assert!(matches!(
        player_update.blocks.get(2),
        Some(UpdateBlock::CreateObject {
            guid,
            is_self: false,
            ..
        }) if *guid == passenger_guid
    ));
}

#[test]
fn persisted_transport_login_resolves_valid_offset_to_current_world_position_like_cpp() {
    let guid = ObjectGuid::create_transport(HighGuid::Transport, 7_002);
    let offset = Position::new(10.0, 20.0, 3.0, 0.25);
    let transport_create = MapTransportCreateLikeCpp {
        guid_low: 7_002,
        entry: 192_241,
        display_id: 3_012,
        scale: 1.0,
        taxi_path_id: 784,
        move_speed: 30,
        accel_rate: 10,
        allow_stopping: false,
        phase_use_flags: 0,
        phase_id: 0,
        phase_group_id: 0,
        gameobject_flags: 0,
        faction_template: 0,
    };
    let transport_position = TransportCreatePositionLikeCpp {
        map_id: 571,
        position: Position::new(100.0, 200.0, 10.0, PI / 2.0),
        timer_ms: 1,
        total_time_ms: 2,
    };
    let resolved = validate_persisted_transport_login_like_cpp(
        guid,
        offset,
        transport_position,
        transport_create,
    )
    .expect("valid passenger attachment");

    assert_eq!(resolved.guid, guid);
    assert_eq!(resolved.map_id, 571);
    assert_eq!(resolved.offset, offset);
    assert_eq!(
        resolved.transport_create, transport_create,
        "SendInitSelf must not need a second DB query for the own transport CREATE"
    );
    assert_eq!(
        resolved.transport_position, transport_position,
        "SendInitSelf must reuse the validated path-time snapshot"
    );
    assert!((resolved.world_position.x - 80.0).abs() < 0.001);
    assert!((resolved.world_position.y - 210.0).abs() < 0.001);
    assert!((resolved.world_position.z - 13.0).abs() < 0.001);
    assert!((resolved.world_position.orientation - (PI / 2.0 + 0.25)).abs() < 0.001);
}

#[test]
fn persisted_transport_login_rejects_corrupt_offsets_and_world_coordinates_like_cpp() {
    let guid = ObjectGuid::create_transport(HighGuid::Transport, 7_003);
    let transport_create = MapTransportCreateLikeCpp {
        guid_low: 7_003,
        entry: 192_241,
        display_id: 3_012,
        scale: 1.0,
        taxi_path_id: 784,
        move_speed: 30,
        accel_rate: 10,
        allow_stopping: false,
        phase_use_flags: 0,
        phase_id: 0,
        phase_group_id: 0,
        gameobject_flags: 0,
        faction_template: 0,
    };
    let valid_transport = TransportCreatePositionLikeCpp {
        map_id: 571,
        position: Position::ZERO,
        timer_ms: 1,
        total_time_ms: 2,
    };

    for invalid_offset in [
        Position::new(250.01, 0.0, 0.0, 0.0),
        Position::new(0.0, -250.01, 0.0, 0.0),
        Position::new(0.0, 0.0, f32::INFINITY, 0.0),
        Position::new(0.0, 0.0, 0.0, f32::NAN),
    ] {
        assert!(
            validate_persisted_transport_login_like_cpp(
                guid,
                invalid_offset,
                valid_transport,
                transport_create,
            )
            .is_none()
        );
    }

    assert!(
        validate_persisted_transport_login_like_cpp(
            guid,
            Position::new(1.0, 0.0, 0.0, 0.0),
            TransportCreatePositionLikeCpp {
                position: Position::new(Position::MAP_HALFSIZE_LIKE_CPP, 0.0, 0.0, 0.0,),
                ..valid_transport
            },
            transport_create,
        )
        .is_none(),
        "C++ rejects an attachment whose calculated world coordinate is outside the map"
    );
}

#[test]
fn persisted_transport_login_requires_saved_map_in_transport_route_like_cpp() {
    assert!(transport_route_contains_saved_map_like_cpp(
        [0, 1, 571],
        571
    ));
    assert!(
        !transport_route_contains_saved_map_like_cpp([0, 1, 571], 530),
        "C++ GetTransport(savedMap) rejects a same-GUID transport absent from that map"
    );
}

#[test]
fn map_corpse_loader_applies_persisted_phases_and_customizations_once_like_cpp() {
    let mut manager = wow_map::MapManager::default();
    let map = manager.create_world_map(571, 0).map_mut();
    assert!(map.load_grid(10.0, 20.0));
    let row = LoadedMapCorpseRowLikeCpp {
        position: Position::new(10.0, 20.0, 30.0, 1.5),
        map_id: 571,
        display_id: 12_345,
        items: std::array::from_fn(|slot| slot as u32 + 100),
        race: 4,
        class: 1,
        sex: 0,
        flags: 0x20,
        dynamic_flags: 0x01,
        ghost_time: 1_000,
        corpse_type: CorpseType::ResurrectablePve,
        instance_id: 0,
        owner_db_guid: 77,
    };
    let phases = HashMap::from([(77, BTreeSet::from([9, 10]))]);
    let choices = vec![
        CorpseCustomizationChoice {
            option_id: 101,
            choice_id: 201,
        },
        CorpseCustomizationChoice {
            option_id: 102,
            choice_id: 202,
        },
    ];
    let customizations = HashMap::from([(77, choices.clone())]);
    let factions = HashMap::from([(4, 35)]);

    let mut invalid_position_row = row.clone();
    invalid_position_row.position.x = f32::NAN;
    let outcome = materialize_loaded_map_corpses_like_cpp(
        map,
        9,
        vec![invalid_position_row, row.clone()],
        &phases,
        &customizations,
        &factions,
    );

    assert_eq!(outcome.rows_seen, 2);
    assert_eq!(outcome.corpses_added, 1);
    assert_eq!(outcome.invalid_position_rows, 1);
    assert!(map.corpse_data_loaded_like_cpp());
    let corpse_guid = ObjectGuid::create_world_object(HighGuid::Corpse, 0, 9, 571, 0, 0, 2);
    let corpse = map.get_typed_corpse(corpse_guid).unwrap();
    assert_eq!(corpse.data().owner, ObjectGuid::create_player(9, 77));
    assert_eq!(corpse.data().customizations, choices);
    assert_eq!(corpse.data().items, row.items);
    assert_eq!(corpse.data().faction_template, 35);
    assert!(corpse.world().phase_shift().has_phase_like_cpp(9));
    assert!(corpse.world().phase_shift().has_phase_like_cpp(10));
    assert!(!corpse.corpse_data_changes_mask().is_any_set());
    assert!(corpse.world().object().is_in_world());
    assert!(
        map.nearby_cell_guids_like_cpp(10.0, 20.0, 1.0)
            .world
            .corpses
            .contains(&corpse_guid)
    );

    let duplicate = materialize_loaded_map_corpses_like_cpp(
        map,
        9,
        vec![row],
        &phases,
        &customizations,
        &factions,
    );
    assert!(duplicate.already_loaded);
    assert_eq!(duplicate.corpses_added, 0);
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

#[tokio::test]
async fn typed_map_corpse_empty_load_marks_the_map_once_like_cpp() {
    let (session, manager, port) =
        map_corpse_session_with_port_like_cpp(PersistedMapCorpseLoadOutcomeLikeCpp::Loaded {
            corpses: Vec::new(),
            phases: MapCorpseAuxiliaryLoadOutcomeLikeCpp::Loaded(Vec::new()),
            customizations: MapCorpseAuxiliaryLoadOutcomeLikeCpp::Loaded(Vec::new()),
        });

    let outcome = session.load_map_corpse_data_like_cpp(571, 9).await;

    assert_eq!(outcome, MapCorpseLoadOutcomeLikeCpp::default());
    assert_eq!(
        port.requests(),
        vec![MapCorpseLoadRequestLikeCpp {
            map_id: 571,
            instance_id: 9,
        }]
    );
    assert!(
        manager
            .lock()
            .unwrap()
            .find_map(571, 9)
            .unwrap()
            .map()
            .corpse_data_loaded_like_cpp()
    );
}

#[tokio::test]
async fn typed_map_corpse_base_failure_publishes_nothing_like_cpp() {
    let (session, manager, _) =
        map_corpse_session_with_port_like_cpp(PersistedMapCorpseLoadOutcomeLikeCpp::Failed {
            reason: "base query failed".to_owned(),
        });

    let outcome = session.load_map_corpse_data_like_cpp(571, 9).await;

    assert_eq!(outcome, MapCorpseLoadOutcomeLikeCpp::default());
    assert!(
        !manager
            .lock()
            .unwrap()
            .find_map(571, 9)
            .unwrap()
            .map()
            .corpse_data_loaded_like_cpp()
    );
}

#[tokio::test]
async fn typed_map_corpse_auxiliary_failures_are_independent_and_non_fatal_like_cpp() {
    let cases = [
        (
            MapCorpseAuxiliaryLoadOutcomeLikeCpp::Failed {
                reason: "phase query failed".to_owned(),
            },
            MapCorpseAuxiliaryLoadOutcomeLikeCpp::Loaded(Vec::new()),
        ),
        (
            MapCorpseAuxiliaryLoadOutcomeLikeCpp::Loaded(Vec::new()),
            MapCorpseAuxiliaryLoadOutcomeLikeCpp::Failed {
                reason: "customization query failed".to_owned(),
            },
        ),
    ];

    for (phases, customizations) in cases {
        let (session, manager, _) =
            map_corpse_session_with_port_like_cpp(PersistedMapCorpseLoadOutcomeLikeCpp::Loaded {
                corpses: vec![invalid_map_corpse_load_row_like_cpp()],
                phases,
                customizations,
            });

        let outcome = session.load_map_corpse_data_like_cpp(571, 9).await;

        assert_eq!(outcome.invalid_type_rows, 1);
        assert_eq!(outcome.corpses_added, 0);
        assert!(
            manager
                .lock()
                .unwrap()
                .find_map(571, 9)
                .unwrap()
                .map()
                .corpse_data_loaded_like_cpp()
        );
    }
}

#[test]
fn bank_storage_mutable_state_round_trips_loaded_expiration_and_charges_like_cpp() {
    let mut item = wow_entities::Item::default();
    assert!(!apply_loaded_item_storage_mutable_fields_like_cpp(
        &mut item,
        90_000,
        90_000,
        "5 -2 0 7 1 ",
        5,
    ));
    item.set_durability(44);
    item.set_create_played_time(55);

    let persisted = item_storage_mutable_persistence_like_cpp(
        7_777,
        &item,
        3,
        0x1234,
        "901 12000 2 ".to_string(),
        5,
    );

    assert_eq!(persisted.item_guid, 7_777);
    assert_eq!(persisted.count, 3);
    assert_eq!(persisted.expiration, 90_000);
    assert_eq!(persisted.charges, "5 -2 0 7 1 ");
    assert_eq!(persisted.flags, 0x1234);
    assert_eq!(persisted.enchantments, "901 12000 2 ");
    assert_eq!(persisted.durability, 44);
    assert_eq!(persisted.played_time, 55);
}

#[test]
fn loaded_item_storage_normalizes_template_duration_and_effect_charge_scope_like_cpp() {
    let mut item = wow_entities::Item::default();

    assert!(apply_loaded_item_storage_mutable_fields_like_cpp(
        &mut item,
        0,
        45_000,
        "7 -3 99 100 101 ",
        2,
    ));

    assert_eq!(item.data().expiration, 45_000);
    assert_eq!(item.data().spell_charges[0], 7);
    assert_eq!(item.data().spell_charges[1], -3);
    assert_eq!(item.data().spell_charges[2], 0);
    assert_eq!(
        item_spell_charges_db_string(&[7, -3, 99, 100, 101], 2),
        "7 -3 "
    );
}

#[test]
fn loaded_item_instance_fields_preserve_enchantments_and_random_suffix_like_cpp() {
    let mut item = wow_entities::Item::default();
    let suffixes =
        wow_data::ItemRandomSuffixStore::from_entries([wow_data::ItemRandomSuffixEntry {
            id: 77,
            enchantments: [901, 902, 903, 0, 0],
            allocation_pct: [1000, 2000, 3000, 0, 0],
        }]);
    let enchantments = test_item_enchantments_db_string(&[
        (EnchantmentSlot::EnhancementPermanent as usize, 2673, 0, 0),
        (
            EnchantmentSlot::EnhancementTemporary as usize,
            3826,
            30_000,
            3,
        ),
        (EnchantmentSlot::Property2 as usize, 901, 0, 0),
    ]);
    let enchantments =
        loaded_item_enchantments_like_cpp(&enchantments).expect("valid C++ enchantment string");
    let random_properties = loaded_item_random_properties_like_cpp(-77, 456, None, Some(&suffixes));
    let effective_enchantments = loaded_item_effective_enchantments_like_cpp(
        Some(&enchantments),
        -77,
        None,
        Some(&suffixes),
    );

    apply_loaded_item_instance_fields_like_cpp(
        &mut item,
        &effective_enchantments,
        random_properties,
    );

    assert_eq!(item.data().random_properties_id, -77);
    assert_eq!(item.data().property_seed, 456);
    assert_eq!(
        item.data().enchantments[EnchantmentSlot::EnhancementPermanent as usize].id,
        2673
    );
    assert_eq!(
        item.data().enchantments[EnchantmentSlot::EnhancementTemporary as usize].duration,
        30_000
    );
    assert_eq!(
        item.data().enchantments[EnchantmentSlot::EnhancementTemporary as usize].charges,
        3
    );
    assert_eq!(
        item.data().enchantments[EnchantmentSlot::Property2 as usize].id,
        901
    );
}

#[test]
fn loaded_item_instance_fields_ignore_short_enchantment_string_like_cpp() {
    let mut item = wow_entities::Item::default();
    let enchantments = loaded_item_enchantments_like_cpp("2673 0 0");
    assert!(enchantments.is_none());
    let effective_enchantments =
        loaded_item_effective_enchantments_like_cpp(enchantments.as_ref(), 0, None, None);

    apply_loaded_item_instance_fields_like_cpp(&mut item, &effective_enchantments, None);

    assert!(
        item.data()
            .enchantments
            .iter()
            .all(|enchantment| *enchantment == wow_entities::ItemEnchantment::default())
    );
    assert_eq!(item.data().random_properties_id, 0);
    assert_eq!(item.data().property_seed, 0);
}

#[test]
fn loaded_item_instance_fields_rebuild_random_suffix_slots_like_cpp() {
    let mut item = wow_entities::Item::default();
    let suffixes =
        wow_data::ItemRandomSuffixStore::from_entries([wow_data::ItemRandomSuffixEntry {
            id: 77,
            enchantments: [901, 902, 903, 904, 905],
            allocation_pct: [1000, 2000, 3000, 4000, 5000],
        }]);
    let effective_enchantments =
        loaded_item_effective_enchantments_like_cpp(None, -77, None, Some(&suffixes));
    let random_properties = loaded_item_random_properties_like_cpp(-77, 456, None, Some(&suffixes));

    apply_loaded_item_instance_fields_like_cpp(
        &mut item,
        &effective_enchantments,
        random_properties,
    );

    assert_eq!(item.data().random_properties_id, -77);
    assert_eq!(item.data().property_seed, 456);
    assert_eq!(
        item.data().enchantments[EnchantmentSlot::Property0 as usize].id,
        901
    );
    assert_eq!(
        item.data().enchantments[EnchantmentSlot::Property1 as usize].id,
        902
    );
    assert_eq!(
        item.data().enchantments[EnchantmentSlot::Property2 as usize].id,
        903
    );
    assert_eq!(
        item.data().enchantments[EnchantmentSlot::Property3 as usize].id,
        0
    );
}

#[test]
fn loaded_item_instance_fields_rebuild_random_property_slots_like_cpp() {
    let mut item = wow_entities::Item::default();
    let properties =
        wow_data::ItemRandomPropertiesStore::from_entries([wow_data::ItemRandomPropertiesEntry {
            id: 77,
            enchantments: [1001, 1002, 1003, 1004, 1005],
        }]);
    let effective_enchantments =
        loaded_item_effective_enchantments_like_cpp(None, 77, Some(&properties), None);
    let random_properties = loaded_item_random_properties_like_cpp(77, 0, Some(&properties), None);

    apply_loaded_item_instance_fields_like_cpp(
        &mut item,
        &effective_enchantments,
        random_properties,
    );

    assert_eq!(item.data().random_properties_id, 77);
    assert_eq!(
        item.data().enchantments[EnchantmentSlot::Property0 as usize].id,
        0
    );
    assert_eq!(
        item.data().enchantments[EnchantmentSlot::Property2 as usize].id,
        1001
    );
    assert_eq!(
        item.data().enchantments[EnchantmentSlot::Property3 as usize].id,
        1002
    );
    assert_eq!(
        item.data().enchantments[EnchantmentSlot::Property4 as usize].id,
        1003
    );
    assert_eq!(item.data().property_seed, 0);
}

#[test]
fn loaded_positive_random_property_ignores_stale_seed_like_cpp() {
    let mut item = wow_entities::Item::default();
    let properties =
        wow_data::ItemRandomPropertiesStore::from_entries([wow_data::ItemRandomPropertiesEntry {
            id: 77,
            enchantments: [1001, 1002, 1003, 0, 0],
        }]);
    let effective_enchantments =
        [wow_packet::packets::update::ItemEnchantmentValuesUpdate::default();
            wow_entities::MAX_ENCHANTMENT_SLOT];
    let random_properties =
        loaded_item_random_properties_like_cpp(77, 456, Some(&properties), None);

    apply_loaded_item_instance_fields_like_cpp(
        &mut item,
        &effective_enchantments,
        random_properties,
    );

    assert_eq!(item.data().random_properties_id, 77);
    assert_eq!(item.data().property_seed, 0);
}

#[test]
fn loaded_missing_random_property_records_are_rejected_like_cpp() {
    let properties = wow_data::ItemRandomPropertiesStore::from_entries([]);
    let suffixes = wow_data::ItemRandomSuffixStore::from_entries([]);

    assert_eq!(
        loaded_item_random_properties_like_cpp(77, 456, Some(&properties), Some(&suffixes)),
        None
    );
    assert_eq!(
        loaded_item_random_properties_like_cpp(-77, 456, Some(&properties), Some(&suffixes)),
        None
    );
}

#[test]
fn loaded_item_instance_fields_preserve_valid_zero_db_enchantments_like_cpp() {
    let properties =
        wow_data::ItemRandomPropertiesStore::from_entries([wow_data::ItemRandomPropertiesEntry {
            id: 77,
            enchantments: [1001, 1002, 1003, 0, 0],
        }]);
    let enchantments = test_item_enchantments_db_string(&[]);
    let enchantments =
        loaded_item_enchantments_like_cpp(&enchantments).expect("valid C++ enchantment string");

    let effective_enchantments = loaded_item_effective_enchantments_like_cpp(
        Some(&enchantments),
        77,
        Some(&properties),
        None,
    );

    assert!(effective_enchantments.iter().all(|enchantment| {
        *enchantment == wow_packet::packets::update::ItemEnchantmentValuesUpdate::default()
    }));
}

#[test]
fn loaded_item_db_enchantments_override_random_property_slots_like_cpp() {
    let properties =
        wow_data::ItemRandomPropertiesStore::from_entries([wow_data::ItemRandomPropertiesEntry {
            id: 77,
            enchantments: [1001, 1002, 1003, 0, 0],
        }]);
    let enchantments =
        test_item_enchantments_db_string(&[(EnchantmentSlot::Property2 as usize, 555, 0, 0)]);
    let enchantments =
        loaded_item_enchantments_like_cpp(&enchantments).expect("valid C++ enchantment string");

    let effective_enchantments = loaded_item_effective_enchantments_like_cpp(
        Some(&enchantments),
        77,
        Some(&properties),
        None,
    );

    assert_eq!(
        effective_enchantments[EnchantmentSlot::Property2 as usize].id,
        555
    );
    assert_eq!(
        effective_enchantments[EnchantmentSlot::Property3 as usize].id,
        0
    );
}

#[test]
fn loaded_item_slots_apply_equipped_enchantments_for_cpp_apply_all_item_mods_range() {
    assert!(loaded_item_slot_applies_equipped_enchantments_like_cpp(
        wow_entities::EQUIPMENT_SLOT_END - 1
    ));
    assert!(loaded_item_slot_applies_equipped_enchantments_like_cpp(
        INVENTORY_SLOT_BAG_START
    ));
    assert!(loaded_item_slot_applies_equipped_enchantments_like_cpp(
        INVENTORY_SLOT_BAG_END - 1
    ));
    assert!(!loaded_item_slot_applies_equipped_enchantments_like_cpp(
        INVENTORY_SLOT_BAG_END
    ));
}

#[test]
fn loaded_socketed_gems_preserve_cpp_item_ids_context_and_bonus_lists() {
    assert_eq!(
        loaded_socketed_gems_like_cpp([
            (700, "11 12".to_string(), 3),
            (0, "13".to_string(), 4),
            (701, "bad 14".to_string(), 5),
        ]),
        vec![
            SocketedGem {
                item_id: 700,
                context: 3,
                bonus_list_ids: vec![11, 12],
            },
            SocketedGem::default(),
            SocketedGem {
                item_id: 701,
                context: 5,
                bonus_list_ids: vec![14],
            },
        ]
    );
}

#[test]
fn sql_creature_template_speed_defaults_match_cpp_check_creature_template() {
    assert_eq!(normalize_creature_template_speed_walk_like_cpp(0.0), 1.0);
    assert_eq!(normalize_creature_template_speed_run_like_cpp(0.0), 1.14286);
    assert_eq!(normalize_creature_template_speed_walk_like_cpp(0.75), 0.75);
    assert_eq!(normalize_creature_template_speed_run_like_cpp(2.0), 2.0);
}

#[test]
fn pvp_season_world_states_match_cpp_world_state_mgr() {
    // In-progress arena season 32 -> current(3191)=32, previous(3901)=31. Matches the
    // captured C++ INIT_WORLD_STATES (World.cpp:1363-1364). Existing ids stay untouched
    // and the 3191/3901 values are overridden in place (not duplicated).
    let mut states = vec![(3191, 0), (3901, 0), (1000, 5)];
    apply_pvp_season_world_states_like_cpp(&mut states, 32, true);
    assert_eq!(states, vec![(3191, 32), (3901, 31), (1000, 5)]);

    // Default (season not in progress): current=0, previous=season_id.
    let mut states = vec![(3191, 0), (3901, 0)];
    apply_pvp_season_world_states_like_cpp(&mut states, 32, false);
    assert_eq!(states, vec![(3191, 0), (3901, 32)]);

    // Absent ids are appended rather than dropped.
    let mut states: Vec<(i32, i32)> = Vec::new();
    apply_pvp_season_world_states_like_cpp(&mut states, 10, true);
    assert_eq!(states, vec![(3191, 10), (3901, 9)]);
}

#[test]
fn init_world_states_builder_orders_realm_then_map_and_filters_area_like_cpp() {
    let area_store = wow_data::AreaTableStore::from_entries([
        wow_data::AreaTableEntry {
            id: 4395,
            continent_id: 571,
            parent_area_id: 0,
            area_bit: -1,
            exploration_level: 0,
            mount_flags: 0,
            flags: 0,
        },
        wow_data::AreaTableEntry {
            id: 4613,
            continent_id: 571,
            parent_area_id: 4395,
            area_bit: -1,
            exploration_level: 0,
            mount_flags: 0,
            flags: 0,
        },
    ]);
    let templates = [
        LoginWorldStateTemplateLikeCpp {
            id: 10,
            default_value: 1,
            map_ids: BTreeSet::new(),
            area_ids: BTreeSet::new(),
        },
        LoginWorldStateTemplateLikeCpp {
            id: 20,
            default_value: 2,
            map_ids: BTreeSet::from([571]),
            area_ids: BTreeSet::new(),
        },
        LoginWorldStateTemplateLikeCpp {
            id: 30,
            default_value: 3,
            map_ids: BTreeSet::from([571]),
            area_ids: BTreeSet::from([4395]),
        },
        LoginWorldStateTemplateLikeCpp {
            id: 40,
            default_value: 4,
            map_ids: BTreeSet::from([571]),
            area_ids: BTreeSet::from([9999]),
        },
        LoginWorldStateTemplateLikeCpp {
            id: 50,
            default_value: 5,
            map_ids: BTreeSet::from([WORLDSTATE_ANY_MAP_LIKE_CPP]),
            area_ids: BTreeSet::new(),
        },
    ];

    assert_eq!(
        build_initial_world_states_like_cpp(
            templates,
            [(20, 22), (999, 999)],
            571,
            4613,
            Some(&area_store),
        ),
        vec![(10, 1), (50, 5), (20, 22), (30, 3)]
    );
}

#[tokio::test]
async fn initial_world_state_port_applies_saved_overlay_after_templates_like_cpp() {
    let port = CollectionLoadPortLikeCpp::for_initial_world_states([
        PlayerInitialWorldStatesLoadOutcomeLikeCpp {
            templates: PlayerInitialWorldStateRowsLikeCpp::Loaded(vec![
                PlayerInitialWorldStateTemplateRowLikeCpp {
                    id: 10,
                    default_value: 1,
                    map_ids_csv: String::new(),
                    area_ids_csv: String::new(),
                },
            ]),
            saved_values: PlayerInitialWorldStateRowsLikeCpp::Loaded(vec![
                PlayerInitialWorldStateValueRowLikeCpp { id: 10, value: 22 },
            ]),
        },
    ]);
    let (mut session, _) = make_session_with_send_capacity(1);
    session.set_player_lifecycle_port_like_cpp(port);

    let states = session
        .test_load_initial_world_states_for_login_like_cpp(571, 0)
        .await;

    assert!(states.contains(&(10, 22)));
    assert!(!states.contains(&(10, 1)));
}

#[tokio::test]
async fn initial_world_state_port_preserves_independent_read_failures_like_cpp() {
    let port = CollectionLoadPortLikeCpp::for_initial_world_states([
        PlayerInitialWorldStatesLoadOutcomeLikeCpp {
            templates: PlayerInitialWorldStateRowsLikeCpp::Loaded(vec![
                PlayerInitialWorldStateTemplateRowLikeCpp {
                    id: 10,
                    default_value: 1,
                    map_ids_csv: String::new(),
                    area_ids_csv: String::new(),
                },
            ]),
            saved_values: PlayerInitialWorldStateRowsLikeCpp::Failed {
                reason: "character read failed".to_owned(),
            },
        },
        PlayerInitialWorldStatesLoadOutcomeLikeCpp {
            templates: PlayerInitialWorldStateRowsLikeCpp::Failed {
                reason: "world read failed".to_owned(),
            },
            saved_values: PlayerInitialWorldStateRowsLikeCpp::Loaded(vec![
                PlayerInitialWorldStateValueRowLikeCpp { id: 10, value: 22 },
            ]),
        },
    ]);
    let (mut session, _) = make_session_with_send_capacity(1);
    session.set_player_lifecycle_port_like_cpp(port);

    let values_failed = session
        .test_load_initial_world_states_for_login_like_cpp(571, 0)
        .await;
    let templates_failed = session
        .test_load_initial_world_states_for_login_like_cpp(571, 0)
        .await;

    assert!(values_failed.contains(&(10, 1)));
    assert!(!templates_failed.iter().any(|(id, _)| *id == 10));
}

#[tokio::test]
async fn persisted_transport_login_requests_world_row_by_guid_and_keeps_absence_unknown_like_cpp() {
    let port = CollectionLoadPortLikeCpp::for_login_transports([
        PlayerLoginTransportLoadOutcomeLikeCpp::Loaded(Vec::new()),
        PlayerLoginTransportLoadOutcomeLikeCpp::Failed {
            reason: "world transport read failed".to_owned(),
        },
    ]);
    let (mut session, _) = make_session_with_send_capacity(1);
    session.set_player_lifecycle_port_like_cpp(port.clone());

    let empty = session
        .resolve_persisted_transport_login_like_cpp(77, 571, Position::new(1.0, 2.0, 3.0, 4.0))
        .await;
    let failed = session
        .resolve_persisted_transport_login_like_cpp(88, 571, Position::new(1.0, 2.0, 3.0, 4.0))
        .await;

    assert!(empty.is_none());
    assert!(failed.is_none());
    assert_eq!(
        port.login_transport_requests(),
        vec![
            PlayerLoginTransportLoadRequestLikeCpp::ByGuid { guid_low: 77 },
            PlayerLoginTransportLoadRequestLikeCpp::ByGuid { guid_low: 88 },
        ]
    );
}

#[test]
fn creature_spawn_difficulties_filter_matches_spawn_mode_like_cpp() {
    assert!(spawn_difficulties_contains_spawn_mode_like_cpp("0", 0));
    assert!(spawn_difficulties_contains_spawn_mode_like_cpp("0,1", 1));
    assert!(!spawn_difficulties_contains_spawn_mode_like_cpp("1", 0));
    assert!(!spawn_difficulties_contains_spawn_mode_like_cpp("", 0));
}

#[test]
fn creature_spawn_difficulties_invalid_token_maps_to_none_like_cpp() {
    assert!(
        spawn_difficulties_contains_spawn_mode_like_cpp("bad", 0),
        "C++ ObjectMgr::ParseSpawnDifficulties maps invalid tokens to DIFFICULTY_NONE"
    );
    assert!(!spawn_difficulties_contains_spawn_mode_like_cpp("bad", 1));
}

#[test]
fn creature_create_hover_offset_matches_cpp_after_addon() {
    let base = Position::new(1.0, 2.0, 3.0, 4.0);
    let adjusted = creature_create_position_after_hover_offset_like_cpp(
        base,
        MovementFlag::HOVER.bits(),
        1.25,
    );

    assert_eq!(
        adjusted,
        Position::new(1.0, 2.0, 4.25, 4.0),
        "C++ Creature::Create calls LoadCreaturesAddon() then adds GetHoverOffset() to m_positionZ"
    );
    assert_eq!(
        creature_create_position_after_hover_offset_like_cpp(base, 0, 1.25),
        base,
        "C++ GetHoverOffset() is zero unless MOVEMENTFLAG_HOVER is set"
    );
}

#[test]
fn creature_create_rooted_movement_flag_matches_cpp_template_root() {
    let flags = MovementFlag::from_bits_retain(creature_create_movement_flags_like_cpp(0, true));
    assert!(
        flags.contains(MovementFlag::ROOT),
        "C++ Creature::LoadTemplateRoot -> Unit::SetRooted adds MOVEMENTFLAG_ROOT"
    );
    assert!(
        !flags.intersects(MovementFlag::MASK_MOVING),
        "C++ Unit::SetRooted removes MOVEMENTFLAG_MASK_MOVING before adding ROOT"
    );

    let hover_root = MovementFlag::from_bits_retain(creature_create_movement_flags_like_cpp(
        wow_constants::CreatureGroundMovementType::Hover as u8,
        true,
    ));
    assert!(hover_root.contains(MovementFlag::HOVER));
    assert!(hover_root.contains(MovementFlag::ROOT));
}

#[test]
fn creature_flags_choose_spawn_override_and_sanitize_like_cpp() {
    let (npc_flags, unit_flags, unit_flags2, unit_flags3) = choose_creature_flags_like_cpp(
        0x10,
        UnitFlags::CAN_SWIM.bits(),
        0,
        0,
        Some(0x20),
        Some(UnitFlags::IN_COMBAT.bits() | UnitFlags::IMMUNE_TO_PC.bits()),
        Some(u32::MAX),
        Some(u32::MAX),
        CreatureFlagsExtra::TRIGGER.bits(),
    );

    assert_eq!(
        npc_flags, 0x20,
        "C++ ObjectMgr::ChooseCreatureFlags prefers CreatureData optional npcflag over template npcflag"
    );
    assert_eq!(
        unit_flags & UnitFlags::IN_COMBAT.bits(),
        0,
        "C++ Creature::UpdateEntry clears UNIT_FLAG_IN_COMBAT for newly-created creatures"
    );
    assert_ne!(
        unit_flags & UnitFlags::IMMUNE_TO_PC.bits(),
        0,
        "allowed UNIT_FIELD_FLAGS bits survive DB sanitization"
    );
    assert_ne!(
        unit_flags & UnitFlags::UNINTERACTIBLE.bits(),
        0,
        "C++ Creature::UpdateEntry sets uninteractible for trigger creatures"
    );
    assert_eq!(unit_flags2, UNIT_FLAGS2_ALLOWED_LIKE_CPP);
    assert_eq!(unit_flags3, UNIT_FLAGS3_ALLOWED_LIKE_CPP);
}

#[test]
fn sql_creature_movement_type_random_requires_wander_distance_like_cpp() {
    assert_eq!(
        creature_movement_generator_type_from_db_like_cpp(1, 0.0),
        MovementGeneratorType::Idle,
        "C++ Creature::Create forces RANDOM_MOTION_TYPE to IDLE_MOTION_TYPE when m_wanderDistance is zero"
    );
    assert_eq!(
        creature_movement_generator_type_from_db_like_cpp(1, 6.0),
        MovementGeneratorType::Random,
        "C++ preserves RANDOM_MOTION_TYPE only when CreatureData::wander_distance is positive"
    );
    assert_eq!(
        creature_movement_generator_type_from_db_like_cpp(WAYPOINT_MOTION_TYPE_LIKE_CPP, 0.0),
        MovementGeneratorType::Waypoint
    );
    assert_eq!(
        normalized_creature_wander_distance_like_cpp(MovementGeneratorType::Idle, 6.0),
        0.0,
        "C++ ObjectMgr::LoadCreatures clears wander_distance when MovementType is idle"
    );
    assert_eq!(
        normalized_creature_wander_distance_like_cpp(MovementGeneratorType::Random, 6.0),
        6.0,
        "C++ ObjectMgr::LoadCreatures keeps positive wander_distance for random movement"
    );
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

#[test]
fn motd_split_preserves_cpp_empty_and_trailing_lines() {
    assert_eq!(
        motd_lines_like_cpp("first@@third@"),
        vec!["first", "", "third", ""]
    );
}

#[tokio::test]
async fn handle_player_login_prelude_resends_account_state_and_orders_packets_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(32);
    let guid = ObjectGuid::create_player(1, 42);
    let tutorials = [10, 20, 30, 40, 50, 60, 70, 80];
    let mounts = [
        AccountMount {
            spell_id: 100,
            flags: 1,
        },
        AccountMount {
            spell_id: 200,
            flags: 2,
        },
    ];
    session.set_player_guid(Some(guid));
    session.load_tutorials_data_values_like_cpp(Some(tutorials));
    assert!(
        session
            .send_handle_player_login_packets_like_cpp(
                guid,
                &Position::new(1.0, 2.0, 3.0, 4.0),
                571,
                &mounts,
                "first@second",
            )
            .await
    );

    let packets = send_rx.try_iter().collect::<Vec<_>>();
    let opcodes = packets
        .iter()
        .filter_map(|bytes| WorldPacket::from_bytes(bytes).server_opcode())
        .collect::<Vec<_>>();
    assert_eq!(
        opcodes,
        vec![
            ServerOpcodes::AccountMountUpdate,
            ServerOpcodes::AccountMountUpdate,
            ServerOpcodes::AccountDataTimes,
            ServerOpcodes::TutorialFlags,
            ServerOpcodes::SetDungeonDifficulty,
            ServerOpcodes::LoginVerifyWorld,
            ServerOpcodes::AccountDataTimes,
            ServerOpcodes::FeatureSystemStatus,
            ServerOpcodes::ChatServerMessage,
            ServerOpcodes::ChatServerMessage,
            ServerOpcodes::SetTimeZoneInformation,
            ServerOpcodes::BattlePetJournalLockAcquired,
        ]
    );

    for (packet, expected_mount) in packets[..2].iter().zip(mounts) {
        let mut body = WorldPacket::from_bytes(&packet[2..]);
        assert!(!body.read_bit().unwrap());
        assert_eq!(body.read_int32().unwrap(), 1);
        assert_eq!(body.read_int32().unwrap(), expected_mount.spell_id);
        assert_eq!(body.read_bits(4).unwrap(), u32::from(expected_mount.flags));
        assert_eq!(body.remaining(), 0);
    }

    let mut global_account_data = WorldPacket::from_bytes(&packets[2][2..]);
    assert_eq!(
        global_account_data.read_packed_guid().unwrap(),
        ObjectGuid::EMPTY
    );
    let mut tutorial_packet = WorldPacket::from_bytes(&packets[3][2..]);
    for expected in tutorials {
        assert_eq!(tutorial_packet.read_uint32().unwrap(), expected);
    }
    assert_eq!(tutorial_packet.remaining(), 0);

    let mut character_account_data = WorldPacket::from_bytes(&packets[6][2..]);
    assert_eq!(character_account_data.read_packed_guid().unwrap(), guid);

    for (packet, expected_line) in packets[8..10].iter().zip(["first", "second"]) {
        let mut body = WorldPacket::from_bytes(&packet[2..]);
        assert_eq!(body.read_int32().unwrap(), 3);
        let string_len = body.read_bits(11).unwrap() as usize;
        assert_eq!(body.read_string(string_len).unwrap(), expected_line);
        assert_eq!(body.remaining(), 0);
    }

    assert!(session.has_represented_battle_pet_journal_lock_like_cpp());
}

#[tokio::test]
async fn before_add_spell_packets_keep_cpp_order_without_name_query_injection() {
    let (mut session, send_rx) = make_session_with_send_capacity(64);
    let guid = ObjectGuid::create_player(1, 42);
    session.set_player_guid(Some(guid));

    assert!(
        session
            .send_initial_packets_before_add_to_map(
                guid,
                &Position::ZERO,
                571,
                0,
                CharacterLoginLocationLikeCpp {
                    map_id: 571,
                    bind_area_id: Some(0),
                    position: Position::ZERO,
                },
                vec![123],
                Vec::new(),
                Vec::new(),
                Vec::new(),
                [0; 180],
                Vec::new(),
                false,
            )
            .await
    );

    let opcodes = drain_server_opcodes(&send_rx);
    assert!(
        !opcodes.contains(&ServerOpcodes::QueryPlayerNamesResponse),
        "C++ ContactList serialization does not synchronously publish name-query results"
    );
    let expected = [
        ServerOpcodes::ContactList,
        ServerOpcodes::BindPointUpdate,
        ServerOpcodes::UpdateTalentData,
        ServerOpcodes::SendKnownSpells,
        ServerOpcodes::SendUnlearnSpells,
        ServerOpcodes::SendSpellHistory,
        ServerOpcodes::SendSpellCharges,
        ServerOpcodes::ActiveGlyphs,
    ];
    let positions = expected.map(|opcode| {
        opcodes
            .iter()
            .position(|candidate| *candidate == opcode)
            .unwrap_or_else(|| panic!("missing {opcode:?}"))
    });
    assert!(
        positions.windows(2).all(|pair| pair[0] < pair[1]),
        "C++ orders ContactList -> talents -> known/unlearn/history/charges -> ActiveGlyphs"
    );
}

#[test]
fn loaded_fist_weapons_mirrors_unarmed_after_all_skill_rows_like_cpp() {
    fn skill_info(skill_id: u16, rank: u16, max_rank: u16) -> wow_data::SkillInfoEntry {
        wow_data::SkillInfoEntry {
            skill_id,
            step: 0,
            rank,
            starting_rank: 1,
            max_rank,
            temp_bonus: 0,
            perm_bonus: 0,
        }
    }

    let mut records = HashMap::from([
        (
            SKILL_UNARMED_LIKE_CPP,
            crate::session::RepresentedPlayerSkillLikeCpp {
                skill_id: SKILL_UNARMED_LIKE_CPP,
                step: 0,
                value: 37,
                max: 400,
                profession_slot: -1,
                state: crate::session::RepresentedPlayerSkillStateLikeCpp::Unchanged,
            },
        ),
        (
            SKILL_FIST_WEAPONS_LIKE_CPP,
            crate::session::RepresentedPlayerSkillLikeCpp {
                skill_id: SKILL_FIST_WEAPONS_LIKE_CPP,
                step: 0,
                value: 12,
                max: 400,
                profession_slot: -1,
                state: crate::session::RepresentedPlayerSkillStateLikeCpp::Unchanged,
            },
        ),
    ]);
    let mut skill_info_by_id = BTreeMap::from([
        (
            SKILL_UNARMED_LIKE_CPP,
            skill_info(SKILL_UNARMED_LIKE_CPP, 37, 400),
        ),
        (
            SKILL_FIST_WEAPONS_LIKE_CPP,
            skill_info(SKILL_FIST_WEAPONS_LIKE_CPP, 12, 400),
        ),
    ]);

    sync_loaded_fist_weapons_with_unarmed_like_cpp(&mut records, &mut skill_info_by_id, 80);

    assert_eq!(
        skill_info_by_id
            .get(&SKILL_FIST_WEAPONS_LIKE_CPP)
            .expect("loaded Fist Weapons slot")
            .rank,
        37
    );
    assert_eq!(
        records
            .get(&SKILL_FIST_WEAPONS_LIKE_CPP)
            .expect("active persisted Fist Weapons")
            .value,
        37
    );
}

#[test]
fn loaded_fist_weapons_without_unarmed_is_cleared_like_cpp_set_skill_zero() {
    let mut records = HashMap::from([(
        SKILL_FIST_WEAPONS_LIKE_CPP,
        crate::session::RepresentedPlayerSkillLikeCpp {
            skill_id: SKILL_FIST_WEAPONS_LIKE_CPP,
            step: 0,
            value: 12,
            max: 400,
            profession_slot: -1,
            state: crate::session::RepresentedPlayerSkillStateLikeCpp::Unchanged,
        },
    )]);
    let mut skill_info_by_id = BTreeMap::from([(
        SKILL_FIST_WEAPONS_LIKE_CPP,
        wow_data::SkillInfoEntry {
            skill_id: SKILL_FIST_WEAPONS_LIKE_CPP,
            step: 0,
            rank: 12,
            starting_rank: 1,
            max_rank: 400,
            temp_bonus: 0,
            perm_bonus: 0,
        },
    )]);

    sync_loaded_fist_weapons_with_unarmed_like_cpp(&mut records, &mut skill_info_by_id, 80);

    assert!(
        !records.contains_key(&SKILL_FIST_WEAPONS_LIKE_CPP),
        "C++ marks the persisted skill deleted"
    );
    let cleared = skill_info_by_id
        .get(&SKILL_FIST_WEAPONS_LIKE_CPP)
        .expect("C++ retains the cleared initial update-field slot");
    assert_eq!(cleared.rank, 0);
    assert_eq!(cleared.max_rank, 0);
}

#[test]
fn skill_rewarded_quest_fallback_uses_future_player_condition_like_cpp() {
    let (mut session, _) = make_session_with_send_capacity(1);
    session.set_loaded_player_identity_like_cpp(0, 1, 1, 10, 0);
    session.set_spell_misc_store(Arc::new(SpellMiscStore::from_entries([
        SpellMiscEntry {
            id: 1,
            spell_id: 900,
            show_future_spell_player_condition_id: 77,
            ..SpellMiscEntry::default()
        },
        SpellMiscEntry {
            id: 2,
            spell_id: 901,
            show_future_spell_player_condition_id: 0,
            ..SpellMiscEntry::default()
        },
    ])));
    session.set_player_condition_store(Arc::new(wow_data::PlayerConditionStore::from_entries([
        PlayerConditionEntry {
            id: 77,
            class_mask: 1,
            ..PlayerConditionEntry::default()
        },
    ])));

    assert!(session.skill_rewarded_quest_fallback_allowed_like_cpp(900));
    assert!(
        !session.skill_rewarded_quest_fallback_allowed_like_cpp(901),
        "C++ MeetsFutureSpellPlayerCondition returns false when the condition id is zero"
    );

    session.set_loaded_player_identity_like_cpp(0, 1, 2, 10, 0);
    assert!(
        !session.skill_rewarded_quest_fallback_allowed_like_cpp(900),
        "the fallback must evaluate the real PlayerCondition against the current player"
    );
}

#[test]
fn skill_rewarded_login_changes_use_real_spell_levels_and_conditions_like_cpp() {
    fn ability(
        id: u32,
        spell: i32,
        acquire_method: i8,
        min_skill_line_rank: i16,
        flags: i8,
    ) -> wow_data::SkillLineAbilityRecord {
        wow_data::SkillLineAbilityRecord {
            id,
            race_mask: 1,
            skill_line: 164,
            spell,
            min_skill_line_rank,
            class_mask: 1,
            supercedes_spell: 0,
            acquire_method,
            trivial_rank_high: 0,
            trivial_rank_low: 0,
            flags,
            num_skill_ups: 0,
            skillup_skill_line_id: 0,
        }
    }

    fn spell_info(spell_id: i32) -> wow_data::SpellInfo {
        wow_data::SpellInfo {
            spell_id,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: 0,
            effect_base_points: 0,
            effect_bonus_coefficient: 0.0,
            aura_type: None,
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: Vec::new(),
        }
    }

    let (mut session, _) = make_session_with_send_capacity(1);
    session.set_loaded_player_identity_like_cpp(0, 1, 1, 10, 0);
    session.set_skill_store(Arc::new(
        wow_data::SkillStore::from_skill_line_abilities_like_cpp([
            ability(
                1,
                900,
                wow_data::skill::SKILL_LINE_ABILITY_LEARNED_ON_SKILL_VALUE_LIKE_CPP,
                50,
                0,
            ),
            ability(
                2,
                901,
                wow_data::skill::SKILL_LINE_ABILITY_LEARNED_ON_SKILL_LEARN_LIKE_CPP,
                0,
                0,
            ),
            ability(
                3,
                902,
                wow_data::skill::SKILL_LINE_ABILITY_REWARDED_FROM_QUEST_LIKE_CPP,
                0,
                wow_data::skill::SKILL_LINE_ABILITY_CAN_FALLBACK_TO_LEARNED_ON_SKILL_LEARN_LIKE_CPP,
            ),
            ability(
                4,
                903,
                wow_data::skill::SKILL_LINE_ABILITY_LEARNED_ON_SKILL_LEARN_LIKE_CPP,
                0,
                0,
            ),
        ]),
    ));

    let mut spell_store = wow_data::SpellStore::new();
    for spell_id in [900, 901, 902] {
        spell_store.insert(spell_id, spell_info(spell_id));
    }
    session.set_spell_store(Arc::new(spell_store));
    session.set_spell_levels_store(Arc::new(wow_data::SpellLevelsStore::from_entries([
        wow_data::SpellLevelsEntry {
            id: 1,
            difficulty_id: 0,
            base_level: 1,
            max_level: 0,
            spell_level: 1,
            max_passive_aura_level: 0,
            spell_id: 900,
        },
        wow_data::SpellLevelsEntry {
            id: 2,
            difficulty_id: 0,
            base_level: 20,
            max_level: 0,
            spell_level: 1,
            max_passive_aura_level: 0,
            spell_id: 901,
        },
        wow_data::SpellLevelsEntry {
            id: 3,
            difficulty_id: 0,
            base_level: 1,
            max_level: 0,
            spell_level: 1,
            max_passive_aura_level: 0,
            spell_id: 902,
        },
        wow_data::SpellLevelsEntry {
            id: 4,
            difficulty_id: 0,
            base_level: 1,
            max_level: 0,
            spell_level: 1,
            max_passive_aura_level: 0,
            spell_id: 903,
        },
    ])));
    session.set_spell_misc_store(Arc::new(SpellMiscStore::from_entries([SpellMiscEntry {
        id: 1,
        spell_id: 902,
        show_future_spell_player_condition_id: 77,
        ..SpellMiscEntry::default()
    }])));
    session.set_player_condition_store(Arc::new(wow_data::PlayerConditionStore::from_entries([
        PlayerConditionEntry {
            id: 77,
            class_mask: 1,
            ..PlayerConditionEntry::default()
        },
    ])));

    let changes = session.skill_rewarded_spell_changes_for_login_like_cpp(164, 40, 1, 1, 10);

    assert_eq!(
        changes.remove,
        vec![900],
        "C++ removes an OnSkillValue spell while the skill is below its required rank"
    );
    assert_eq!(
        changes.learn,
        vec![902],
        "the level-gated spell and the ability without real SpellInfo must be skipped"
    );
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

#[test]
fn invalid_homebind_repair_selects_cpp_create_mode_and_graveyard_order() {
    let normal = PlayerCreatePositionLikeCpp {
        map_id: 0,
        position: Position::new(-8_946.0, -246.0, 59.0, 0.0),
        transport_guid: None,
    };
    let npe = PlayerCreatePositionLikeCpp {
        map_id: 2_175,
        position: Position::new(1.0, 2.0, 3.0, 4.0),
        transport_guid: None,
    };
    let info = PlayerCreateInfoLikeCpp {
        create_position: normal,
        create_position_npe: Some(npe),
    };
    assert_eq!(
        first_login_creation_homebind_like_cpp(info, wow_data::PLAYER_CREATE_MODE_NORMAL_LIKE_CPP,),
        Some(CharacterLoginLocationLikeCpp {
            map_id: normal.map_id,
            bind_area_id: None,
            position: normal.position,
        })
    );
    assert_eq!(
        first_login_creation_homebind_like_cpp(info, wow_data::PLAYER_CREATE_MODE_NPE_LIKE_CPP,)
            .map(|homebind| homebind.position),
        Some(npe.position)
    );
    assert_eq!(
        first_login_creation_homebind_like_cpp(
            PlayerCreateInfoLikeCpp {
                create_position_npe: None,
                ..info
            },
            wow_data::PLAYER_CREATE_MODE_NPE_LIKE_CPP,
        )
        .map(|homebind| homebind.position),
        Some(normal.position),
        "C++ falls back to the normal class/race creation position when NPE data is invalid"
    );
    assert_eq!(
        first_login_creation_homebind_like_cpp(
            PlayerCreateInfoLikeCpp {
                create_position_npe: Some(PlayerCreatePositionLikeCpp {
                    transport_guid: Some(29),
                    ..npe
                }),
                ..info
            },
            wow_data::PLAYER_CREATE_MODE_NPE_LIKE_CPP,
        ),
        None,
        "C++ does not bind first-login transport offsets and falls through to graveyard"
    );

    assert_eq!(
        default_graveyard_safe_loc_ids_for_race_like_cpp(1),
        [Some(4), None]
    );
    assert_eq!(
        default_graveyard_safe_loc_ids_for_race_like_cpp(2),
        [Some(10), None]
    );
    assert_eq!(
        default_graveyard_safe_loc_ids_for_race_like_cpp(24),
        [Some(4), Some(3295)]
    );

    let area_store = wow_data::AreaTableStore::from_entries([
        wow_data::AreaTableEntry {
            id: 12,
            continent_id: 0,
            parent_area_id: 0,
            area_bit: 0,
            exploration_level: 0,
            mount_flags: 0,
            flags: 0,
        },
        wow_data::AreaTableEntry {
            id: 13,
            continent_id: 0,
            parent_area_id: 12,
            area_bit: 0,
            exploration_level: 0,
            mount_flags: 0,
            flags: 0x4000_0000,
        },
    ]);
    assert_eq!(
        zone_and_area_from_area_id_like_cpp(13, Some(&area_store)),
        (12, 13)
    );

    let scenario_garrison_store = wow_data::MapStore::from_entries([wow_data::MapEntry {
        id: 1_151,
        instance_type: wow_data::map::MAP_SCENARIO,
        expansion_id: 0,
        parent_map_id: -1,
        cosmetic_parent_map_id: -1,
        flags1: wow_data::map::MAP_FLAG_GARRISON,
        flags2: 0,
    }]);
    assert!(!usable_character_homebind_like_cpp(
        CharacterLoginLocationLikeCpp {
            map_id: 1_151,
            bind_area_id: Some(12),
            position: Position::ZERO,
        },
        Some(&scenario_garrison_store),
        2,
    ));
}

#[test]
fn default_homebind_reads_primary_then_neutral_pandaren_from_startup_store_like_cpp() {
    fn map(id: u32) -> wow_data::MapEntry {
        wow_data::MapEntry {
            id,
            instance_type: wow_data::map::MAP_COMMON,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        }
    }
    let maps = wow_data::MapStore::from_entries([map(1), map(870)]);
    let primary = wow_data::WorldSafeLocRow {
        id: 4,
        map_id: 1,
        x: 1.0,
        y: 2.0,
        z: 3.0,
        facing_degrees: 90.0,
    };
    let fallback = wow_data::WorldSafeLocRow {
        id: 3295,
        map_id: 870,
        x: 4.0,
        y: 5.0,
        z: 6.0,
        facing_degrees: 180.0,
    };

    let (mut session, _, _) = make_bank_slot_session(4);
    let (store, report) =
        wow_data::WorldSafeLocStore::from_rows_like_cpp([fallback, primary], &maps);
    assert_eq!(report.loaded, 2);
    session.set_world_safe_loc_store_like_cpp(Arc::new(store));
    assert_eq!(
        session
            .load_default_graveyard_homebind_like_cpp(24)
            .expect("neutral Pandaren uses faction primary first"),
        CharacterLoginLocationLikeCpp {
            map_id: 1,
            bind_area_id: None,
            position: Position::new(1.0, 2.0, 3.0, 90_f32.to_radians()),
        }
    );

    let (fallback_only, report) =
        wow_data::WorldSafeLocStore::from_rows_like_cpp([fallback], &maps);
    assert_eq!(report.loaded, 1);
    session.set_world_safe_loc_store_like_cpp(Arc::new(fallback_only));
    assert_eq!(
        session
            .load_default_graveyard_homebind_like_cpp(24)
            .expect("neutral Pandaren keeps C++ 3295 fallback")
            .map_id,
        870
    );
}

#[tokio::test]
async fn homebind_repair_writes_typed_delete_and_insert_requests_nonfatally_like_cpp() {
    let (mut session, _, _) = make_bank_slot_session(4);
    let port = HomebindPortFixtureLikeCpp::new([
        PersistenceOutcomeLikeCpp::Failed {
            reason: "delete failed".to_owned(),
        },
        PersistenceOutcomeLikeCpp::Failed {
            reason: "insert failed".to_owned(),
        },
    ]);
    session.set_player_lifecycle_port_like_cpp(port.clone());
    let guid = ObjectGuid::create_player(1, 77);

    session
        .delete_invalid_character_homebind_like_cpp(guid)
        .await;
    let create_position = PlayerCreatePositionLikeCpp {
        map_id: 0,
        position: Position::new(1.0, 2.0, 3.0, 4.0),
        transport_guid: None,
    };
    let repaired = session
        .repair_character_homebind_like_cpp(
            guid,
            1,
            PlayerCreateInfoLikeCpp {
                create_position,
                create_position_npe: None,
            },
            wow_data::PLAYER_CREATE_MODE_NORMAL_LIKE_CPP,
            true,
        )
        .await
        .expect("nonfatal persistence failure does not discard selected homebind");
    assert_eq!(repaired.map_id, 0);
    assert_eq!(
        port.requests(),
        vec![
            PlayerHomebindPersistenceRequestLikeCpp::DeleteInvalid {
                player_guid: guid.counter() as u64,
            },
            PlayerHomebindPersistenceRequestLikeCpp::InsertRepaired {
                player_guid: guid.counter() as u64,
                map_id: 0,
                area_id: 0,
                x: 1.0,
                y: 2.0,
                z: 3.0,
                orientation: 4.0,
            },
        ]
    );
}

#[test]
fn battleground_login_fallback_prefers_valid_entry_point_then_homebind_like_cpp() {
    let map_store = wow_data::MapStore::from_entries([
        wow_data::MapEntry {
            id: 1,
            instance_type: wow_data::map::MAP_COMMON,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
        wow_data::MapEntry {
            id: 0,
            instance_type: wow_data::map::MAP_COMMON,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
    ]);
    let entry_point = CharacterLoginLocationLikeCpp {
        map_id: 1,
        bind_area_id: None,
        position: Position::new(10.0, 20.0, 30.0, 1.0),
    };
    let homebind = CharacterLoginLocationLikeCpp {
        map_id: 0,
        bind_area_id: Some(12),
        position: Position::new(-1.0, -2.0, 3.0, 0.0),
    };
    let bg_data = CharacterBattlegroundLoginDataLikeCpp { entry_point };

    assert!(usable_character_homebind_like_cpp(
        homebind,
        Some(&map_store),
        2,
    ));
    assert!(!usable_character_homebind_like_cpp(
        entry_point,
        Some(&map_store),
        2,
    ));

    assert_eq!(
        login_location_zone_area_like_cpp(entry_point, |map_id, position| {
            assert_eq!(map_id, 1);
            assert_eq!(position, entry_point.position);
            Ok((34, 56))
        })
        .unwrap(),
        (34, 56)
    );
    assert_eq!(
        login_location_zone_area_like_cpp(homebind, |map_id, position| {
            assert_eq!(map_id, 0);
            assert_eq!(position, homebind.position);
            Ok((78, 90))
        })
        .unwrap(),
        (78, 90)
    );
    let bind_update = login_bind_point_update_like_cpp(homebind);
    assert_eq!(bind_update.x, homebind.position.x);
    assert_eq!(bind_update.y, homebind.position.y);
    assert_eq!(bind_update.z, homebind.position.z);
    assert_eq!(bind_update.map_id, homebind.map_id);
    assert_eq!(bind_update.area_id, 12);

    assert_eq!(
        battleground_login_fallback_location_like_cpp(
            Some(bg_data),
            Some(homebind),
            Some(&map_store),
        ),
        Some(entry_point)
    );
    assert_eq!(
        battleground_login_fallback_location_like_cpp(
            Some(CharacterBattlegroundLoginDataLikeCpp {
                entry_point: CharacterLoginLocationLikeCpp {
                    map_id: u32::from(u16::MAX),
                    bind_area_id: None,
                    position: Position::ZERO,
                },
                ..bg_data
            }),
            Some(homebind),
            Some(&map_store),
        ),
        Some(homebind)
    );
    assert_eq!(
        battleground_login_fallback_location_like_cpp(
            None,
            Some(CharacterLoginLocationLikeCpp {
                position: Position::new(f32::NAN, 0.0, 0.0, 0.0),
                ..homebind
            }),
            Some(&map_store),
        ),
        None
    );
}

#[test]
fn rejected_instance_login_retries_valid_homebind_before_disconnect_like_cpp() {
    run_login_grid_cleanup_test(|| {
        let (mut session, _send_rx) = make_session_with_send_capacity(2);
        let guid = ObjectGuid::create_player(1, 45);
        let saved_position = Position::new(1.0, 2.0, 3.0, 0.0);
        let homebind_position = Position::new(10.0, 20.0, 30.0, 1.0);
        let canonical: crate::session::SharedCanonicalMapManager =
            Arc::new(std::sync::Mutex::new(wow_map::MapManager::default()));
        session.set_canonical_map_manager(Arc::clone(&canonical));
        session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
            wow_data::MapEntry {
                id: 33,
                instance_type: wow_data::map::MAP_INSTANCE,
                expansion_id: 0,
                parent_map_id: -1,
                cosmetic_parent_map_id: -1,
                flags1: 0,
                flags2: 0,
            },
            wow_data::MapEntry {
                id: 1,
                instance_type: wow_data::map::MAP_COMMON,
                expansion_id: 0,
                parent_map_id: -1,
                cosmetic_parent_map_id: -1,
                flags1: 0,
                flags2: 0,
            },
        ])));
        assert!(session.ensure_login_player_controller_like_cpp(
            guid,
            "InstanceFallback".to_string(),
            saved_position,
            33,
            1,
            1,
            10,
            0,
        ));
        assert!(matches!(
            session.ensure_canonical_world_map_for_current_player_like_cpp(),
            Some(wow_map::CreateMapDecision::Reject { .. })
        ));
        assert!(
            session
                .current_canonical_player_map_key_like_cpp()
                .is_none()
        );

        let mut map_id = 33;
        let mut zone_id = 999;
        let mut position = saved_position;
        assert!(session.retry_login_at_homebind_like_cpp(
            &mut map_id,
            &mut zone_id,
            &mut position,
            CharacterLoginLocationLikeCpp {
                map_id: 1,
                bind_area_id: Some(12),
                position: homebind_position,
            },
        ));

        assert_eq!(map_id, 1);
        assert_eq!(zone_id, 12);
        assert_eq!(session.player_zone_area_like_cpp(), (12, 12));
        assert_eq!(position, homebind_position);
        assert_eq!(
            session.current_canonical_player_map_key_like_cpp(),
            Some(wow_map::MapKey::new(1, 0))
        );
    });
}

#[test]
fn homebind_retry_refreshes_zone_when_saved_coordinates_already_match_like_cpp() {
    run_login_grid_cleanup_test(|| {
        let (mut session, _send_rx) = make_session_with_send_capacity(1);
        let guid = ObjectGuid::create_player(1, 46);
        let homebind_position = Position::new(10.0, 20.0, 30.0, 1.0);
        let canonical: crate::session::SharedCanonicalMapManager =
            Arc::new(std::sync::Mutex::new(wow_map::MapManager::default()));
        session.set_canonical_map_manager(Arc::clone(&canonical));
        session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
            wow_data::MapEntry {
                id: 1,
                instance_type: wow_data::map::MAP_COMMON,
                expansion_id: 0,
                parent_map_id: -1,
                cosmetic_parent_map_id: -1,
                flags1: 0,
                flags2: 0,
            },
        ])));
        assert!(session.ensure_login_player_controller_like_cpp(
            guid,
            "MatchingHomebind".to_string(),
            homebind_position,
            1,
            1,
            1,
            10,
            0,
        ));

        let mut map_id = 1;
        let mut zone_id = 999;
        let mut position = homebind_position;
        assert!(session.retry_login_at_homebind_like_cpp(
            &mut map_id,
            &mut zone_id,
            &mut position,
            CharacterLoginLocationLikeCpp {
                map_id: 1,
                bind_area_id: Some(12),
                position: homebind_position,
            },
        ));

        assert_eq!(map_id, 1);
        assert_eq!(zone_id, 12);
        assert_eq!(position, homebind_position);
        assert_eq!(session.player_zone_area_like_cpp(), (12, 12));
        assert_eq!(
            session.current_canonical_player_map_key_like_cpp(),
            Some(wow_map::MapKey::new(1, 0))
        );
    });
}

#[test]
fn garrison_login_uses_create_map_world_branch_like_cpp() {
    run_login_grid_cleanup_test(|| {
        let (mut session, _send_rx) = make_session_with_send_capacity(1);
        let guid = ObjectGuid::create_player(1, 47);
        let canonical: crate::session::SharedCanonicalMapManager =
            Arc::new(std::sync::Mutex::new(wow_map::MapManager::default()));
        session.set_canonical_map_manager(Arc::clone(&canonical));
        session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
            wow_data::MapEntry {
                id: 1_151,
                instance_type: wow_data::map::MAP_COMMON,
                expansion_id: 2,
                parent_map_id: -1,
                cosmetic_parent_map_id: -1,
                flags1: wow_data::map::MAP_FLAG_GARRISON,
                flags2: 0,
            },
        ])));
        assert!(session.ensure_login_player_controller_like_cpp(
            guid,
            "GarrisonLogin".to_string(),
            Position::ZERO,
            1_151,
            1,
            1,
            10,
            0,
        ));

        assert!(matches!(
            session.ensure_canonical_world_map_for_current_player_like_cpp(),
            Some(wow_map::CreateMapDecision::Create {
                key,
                kind: wow_map::ManagedMapKind::World,
                ..
            }) if key == wow_map::MapKey::new(1_151, 0)
        ));
        assert_eq!(
            session.current_canonical_player_map_key_like_cpp(),
            Some(wow_map::MapKey::new(1_151, 0))
        );
    });
}

#[test]
fn garrison_login_rejects_unsupported_expansion_and_retries_homebind_like_cpp() {
    run_login_grid_cleanup_test(|| {
        let (mut session, _send_rx) = make_session_with_send_capacity(1);
        let guid = ObjectGuid::create_player(1, 48);
        let canonical: crate::session::SharedCanonicalMapManager =
            Arc::new(std::sync::Mutex::new(wow_map::MapManager::default()));
        session.set_canonical_map_manager(Arc::clone(&canonical));
        session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
            wow_data::MapEntry {
                id: 1,
                instance_type: wow_data::map::MAP_COMMON,
                expansion_id: 0,
                parent_map_id: -1,
                cosmetic_parent_map_id: -1,
                flags1: 0,
                flags2: 0,
            },
            wow_data::MapEntry {
                id: 1_151,
                instance_type: wow_data::map::MAP_COMMON,
                expansion_id: 3,
                parent_map_id: -1,
                cosmetic_parent_map_id: -1,
                flags1: wow_data::map::MAP_FLAG_GARRISON,
                flags2: 0,
            },
        ])));
        assert!(session.ensure_login_player_controller_like_cpp(
            guid,
            "UnsupportedGarrisonLogin".to_string(),
            Position::ZERO,
            1_151,
            1,
            1,
            10,
            0,
        ));

        assert!(matches!(
            session.ensure_canonical_world_map_for_current_player_like_cpp(),
            Some(wow_map::CreateMapDecision::Reject { .. })
        ));
        assert!(
            session
                .current_canonical_player_map_key_like_cpp()
                .is_none()
        );
        assert!(canonical.lock().unwrap().find_map(1_151, 0).is_none());

        let mut map_id = 1_151;
        let mut zone_id = 999;
        let mut position = Position::ZERO;
        let homebind_position = Position::new(10.0, 20.0, 30.0, 1.0);
        assert!(session.retry_login_at_homebind_like_cpp(
            &mut map_id,
            &mut zone_id,
            &mut position,
            CharacterLoginLocationLikeCpp {
                map_id: 1,
                bind_area_id: Some(12),
                position: homebind_position,
            },
        ));
        assert_eq!(map_id, 1);
        assert_eq!(zone_id, 12);
        assert_eq!(session.player_zone_area_like_cpp(), (12, 12));
        assert_eq!(position, homebind_position);
        assert_eq!(
            session.current_canonical_player_map_key_like_cpp(),
            Some(wow_map::MapKey::new(1, 0))
        );
    });
}

#[test]
fn unavailable_login_grid_cleans_partial_player_and_kicks_without_failure_packet_like_cpp() {
    run_login_grid_cleanup_test(|| {
        let (mut session, send_rx) = make_session_with_send_capacity(1);
        let guid = ObjectGuid::create_player(1, 42);
        let canonical: crate::session::SharedCanonicalMapManager =
            Arc::new(std::sync::Mutex::new(wow_map::MapManager::default()));
        let registry = Arc::new(crate::session::directory::PlayerRegistry::default());
        session.set_canonical_map_manager(Arc::clone(&canonical));
        session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
            wow_data::MapEntry {
                id: 33,
                instance_type: wow_data::map::MAP_COMMON,
                expansion_id: 0,
                parent_map_id: -1,
                cosmetic_parent_map_id: -1,
                flags1: 0,
                flags2: 0,
            },
        ])));
        session.set_player_registry(Arc::clone(&registry));
        assert!(session.ensure_login_player_controller_like_cpp(
            guid,
            "GridFailure".to_string(),
            Position::ZERO,
            33,
            1,
            1,
            10,
            0,
        ));
        let _ = session.ensure_canonical_world_map_for_current_player_like_cpp();
        session.register_in_player_registry();
        assert!(
            session
                .current_canonical_player_map_key_like_cpp()
                .is_some()
        );
        assert!(registry.runtime_recipient(guid).is_some());

        assert!(!session.continue_login_after_grid_load_like_cpp(
            guid,
            33,
            0,
            Some(crate::session::PlayerGridLoadOutcomeLikeCpp {
                map_unavailable: true,
                ..Default::default()
            }),
        ));

        assert_eq!(session.state(), crate::session::SessionState::Disconnecting);
        assert!(session.player_guid().is_none());
        assert!(
            canonical
                .lock()
                .unwrap()
                .find_map(33, 0)
                .unwrap()
                .map()
                .get_typed_player(guid)
                .is_none()
        );
        assert!(registry.runtime_recipient(guid).is_none());
        assert!(send_rx.try_recv().is_err());
    });
}

#[test]
fn login_identity_hydrates_race_faction_into_registry_and_canonical_player_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send_capacity(1);
    let guid = ObjectGuid::create_player(1, 42_001);
    let canonical: crate::session::SharedCanonicalMapManager =
        Arc::new(std::sync::Mutex::new(wow_map::MapManager::default()));
    let registry = Arc::new(crate::session::directory::PlayerRegistry::default());
    let mut race_entry = chr_race_entry(1, 0);
    race_entry.faction_id = 1;

    session.set_chr_races_store(Arc::new(ChrRacesStore::from_entries([race_entry])));
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_player_registry(Arc::clone(&registry));
    session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
        wow_data::MapEntry {
            id: 571,
            instance_type: wow_data::map::MAP_COMMON,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
    ])));

    // Mirror the real LoadFromDB order: identity is loaded from the
    // character row before the controller/map/registry publication.
    session.set_player_guid(Some(guid));
    session.set_loaded_player_identity_like_cpp(571, 1, 1, 10, 0);
    assert!(session.ensure_login_player_controller_like_cpp(
        guid,
        "FactionLogin".to_string(),
        Position::ZERO,
        571,
        1,
        1,
        10,
        0,
    ));
    let _ = session.ensure_canonical_world_map_for_current_player_like_cpp();
    session.register_in_player_registry();

    assert_eq!(registry.legacy_aggro_candidates()[0].faction_template_id, 1);
    let manager = canonical.lock().unwrap();
    let player = manager
        .find_map(571, 0)
        .expect("login map")
        .map()
        .get_typed_player(guid)
        .expect("canonical login player");
    assert_eq!(player.unit().data().faction_template, 1);
}

#[test]
fn late_login_sequence_failure_releases_claim_and_partial_player_like_cpp() {
    let guid = ObjectGuid::create_player(1, 9_001_701);
    let (mut failed, _failed_rx) = make_session_with_send_capacity(1);
    assert!(failed.try_claim_character_login_like_cpp(guid));
    assert!(failed.ensure_login_player_controller_like_cpp(
        guid,
        "LateFenceFailure".to_string(),
        Position::ZERO,
        1,
        1,
        1,
        10,
        0,
    ));

    failed.abort_partial_login_sequence_like_cpp();

    assert_eq!(failed.state(), crate::session::SessionState::Disconnecting);
    assert!(failed.player_guid().is_none());
    let (mut retry, _retry_rx) = make_session_with_send_capacity(1);
    assert!(
        retry.try_claim_character_login_like_cpp(guid),
        "the failed login must not retain the only process-wide character claim"
    );
    retry.release_character_login_claim_like_cpp();
}

#[tokio::test]
async fn unavailable_login_grid_aborts_before_success_login_packets_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(1);
    let guid = ObjectGuid::create_player(1, 46);
    assert!(session.ensure_login_player_controller_like_cpp(
        guid,
        "PreflightFailure".to_string(),
        Position::ZERO,
        1,
        1,
        1,
        10,
        0,
    ));
    session.set_player_grid_load_resolver_like_cpp(Arc::new(|_, _, _| {
        crate::session::PlayerGridLoadOutcomeLikeCpp {
            map_unavailable: true,
            ..Default::default()
        }
    }));

    assert!(
        !session
            .send_login_sequence(
                guid,
                1,
                1,
                0,
                10,
                49,
                &Position::ZERO,
                1,
                0,
                CharacterLoginLocationLikeCpp {
                    map_id: 1,
                    bind_area_id: Some(0),
                    position: Position::ZERO,
                },
                None,
                [(0, 0, 0); 19],
                [ObjectGuid::EMPTY; 141],
                Vec::new(),
                PlayerCombatStats::default(),
                0,
                0,
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
                [0; 180],
                Vec::new(),
                Vec::new(),
            )
            .await
    );

    assert_eq!(session.state(), crate::session::SessionState::Disconnecting);
    assert!(session.player_guid().is_none());
    assert!(
        send_rx.try_recv().is_err(),
        "C++ LoadFromDB failure happens before DungeonDifficultySet/LoginVerifyWorld"
    );
}

#[test]
fn login_without_grid_resolver_fails_closed_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(1);
    let guid = ObjectGuid::create_player(1, 43);
    assert!(session.ensure_login_player_controller_like_cpp(
        guid,
        "MissingResolver".to_string(),
        Position::ZERO,
        1,
        1,
        1,
        10,
        0,
    ));

    assert!(!session.continue_login_after_grid_load_like_cpp(guid, 1, 0, None));

    assert_eq!(session.state(), crate::session::SessionState::Disconnecting);
    assert!(session.player_guid().is_none());
    assert!(send_rx.try_recv().is_err());
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

#[test]
fn restored_saved_health_preserves_dead_zero_like_cpp() {
    assert_eq!(restored_saved_health_like_cpp(Some(0), 110), 0);
}

#[test]
fn restored_saved_health_clamps_to_recomputed_max_like_cpp() {
    assert_eq!(restored_saved_health_like_cpp(Some(500), 110), 110);
    assert_eq!(restored_saved_health_like_cpp(Some(77), 110), 77);
}

#[test]
fn level_up_deltas_use_cpp_class_race_stats_and_base_mp() {
    let (mut session, _send_rx) = make_session_with_send_capacity(1);
    session.set_loaded_player_identity_like_cpp(571, 1, 5, 1, 0);
    session.set_player_stats(Arc::new(PlayerStatsStore::from_entries([
        (
            (1, 5, 1),
            PlayerLevelStats {
                strength: 10,
                agility: 11,
                stamina: 12,
                intellect: 13,
                spirit: 14,
                base_mana: 155,
            },
        ),
        (
            (1, 5, 2),
            PlayerLevelStats {
                strength: 12,
                agility: 11,
                stamina: 15,
                intellect: 17,
                spirit: 19,
                base_mana: 170,
            },
        ),
    ])));

    assert_eq!(
        session.level_up_stat_deltas_like_cpp(2),
        Some((15, [2, 0, 3, 4, 5]))
    );
    assert_eq!(session.level_up_stat_deltas_like_cpp(3), None);
}

#[test]
fn stat_update_preserves_current_mana_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send_capacity(1);
    let player_guid = ObjectGuid::create_player(1, 77);
    session.set_player_guid(Some(player_guid));
    session.set_loaded_player_identity_like_cpp(571, 1, 5, 80, 0);
    set_priest_level80_stats(&mut session, 1000, 40);
    attach_stat_update_player_with_mana(&mut session, player_guid, 777, 1320);

    let (_, changes) = session
        .player_stat_changes_like_cpp()
        .expect("stat changes");

    assert_eq!(changes.power0, 777);
    assert_eq!(changes.max_power0, 1320);
    assert_eq!(changes.base_mana, 1000);
    assert_eq!(
        session.canonical_player_power_snapshot_like_cpp(PowerType::Mana),
        Some((777, 1320))
    );
}

#[test]
fn level_up_stat_update_refills_health_and_mana_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send_capacity(1);
    let player_guid = ObjectGuid::create_player(1, 76);
    session.set_player_guid(Some(player_guid));
    session.set_loaded_player_identity_like_cpp(571, 1, 5, 80, 0);
    set_priest_level80_stats(&mut session, 1000, 40);
    attach_stat_update_player_with_mana_and_health(&mut session, player_guid, 777, 1320, 3, 10);

    session.send_level_up_stat_update_like_cpp();

    assert_eq!(
        session.canonical_player_health_snapshot_like_cpp(),
        Some((10, 10))
    );
    assert_eq!(
        session.canonical_player_power_snapshot_like_cpp(PowerType::Mana),
        Some((1320, 1320))
    );
    assert_eq!(session.player_health_like_cpp(), 10);
}

#[test]
fn stat_update_clamps_current_mana_to_new_max_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send_capacity(1);
    let player_guid = ObjectGuid::create_player(1, 78);
    session.set_player_guid(Some(player_guid));
    session.set_loaded_player_identity_like_cpp(571, 1, 5, 80, 0);
    set_priest_level80_stats(&mut session, 1000, 40);
    attach_stat_update_player_with_mana(&mut session, player_guid, 2_000, 2_500);

    let (_, changes) = session
        .player_stat_changes_like_cpp()
        .expect("stat changes");

    assert_eq!(changes.power0, 1320);
    assert_eq!(changes.max_power0, 1320);
    assert_eq!(changes.base_mana, 1000);
    assert_eq!(
        session.canonical_player_power_snapshot_like_cpp(PowerType::Mana),
        Some((1320, 1320))
    );
}

#[test]
fn stat_update_preserves_current_health_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send_capacity(1);
    let player_guid = ObjectGuid::create_player(1, 79);
    session.set_player_guid(Some(player_guid));
    session.set_loaded_player_identity_like_cpp(571, 1, 5, 80, 0);
    set_priest_level80_stats(&mut session, 1000, 40);
    attach_stat_update_player_with_mana_and_health(&mut session, player_guid, 777, 1320, 7, 500);

    let (_, changes) = session
        .player_stat_changes_like_cpp()
        .expect("stat changes");

    assert_eq!(changes.health, 7);
    assert_eq!(changes.max_health, 10);
    assert_eq!(session.player_health_like_cpp(), 7);
    assert_eq!(
        session.canonical_player_health_snapshot_like_cpp(),
        Some((7, 10))
    );
}

#[test]
fn stat_update_clamps_current_health_to_new_max_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send_capacity(1);
    let player_guid = ObjectGuid::create_player(1, 80);
    session.set_player_guid(Some(player_guid));
    session.set_loaded_player_identity_like_cpp(571, 1, 5, 80, 0);
    set_priest_level80_stats(&mut session, 1000, 40);
    attach_stat_update_player_with_mana_and_health(&mut session, player_guid, 777, 1320, 500, 500);

    let (_, changes) = session
        .player_stat_changes_like_cpp()
        .expect("stat changes");

    assert_eq!(changes.health, 10);
    assert_eq!(changes.max_health, 10);
    assert_eq!(session.player_health_like_cpp(), 10);
    assert_eq!(
        session.canonical_player_health_snapshot_like_cpp(),
        Some((10, 10))
    );
}

#[test]
fn total_stat_percentage_ability_preserves_health_pct_on_apply_and_remove_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send_capacity(8);
    let player_guid = ObjectGuid::create_player(1, 82);
    let spell_id = 90_082;
    session.set_player_guid(Some(player_guid));
    session.set_loaded_player_identity_like_cpp(571, 1, 5, 80, 0);
    set_priest_level80_stats(&mut session, 1_000, 40);
    attach_stat_update_player_with_mana_and_health(&mut session, player_guid, 777, 1_320, 5, 10);
    session.set_player_health_like_cpp(5, 10);
    session.set_spell_store(Arc::new(total_stat_percentage_spell_store_like_cpp(
        spell_id, true,
    )));
    session.set_state(crate::session::SessionState::LoggedIn);

    session
        .apply_aura(spell_id, player_guid, 30_000, 1)
        .expect("apply stamina ability");
    assert_eq!(
        session.canonical_player_health_snapshot_like_cpp(),
        Some((10, 20)),
        "C++ restores 50% health after the stamina ability raises max health"
    );

    let slot = session
        .visible_aura_slot_for_spell_like_cpp(spell_id)
        .expect("stamina ability aura slot");
    session.remove_aura(slot).expect("remove stamina ability");
    assert_eq!(
        session.canonical_player_health_snapshot_like_cpp(),
        Some((5, 10)),
        "C++ restores 50% health after the stamina ability lowers max health"
    );
}

#[test]
fn total_stat_percentage_non_ability_keeps_current_health_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send_capacity(4);
    let player_guid = ObjectGuid::create_player(1, 83);
    let spell_id = 90_083;
    session.set_player_guid(Some(player_guid));
    session.set_loaded_player_identity_like_cpp(571, 1, 5, 80, 0);
    set_priest_level80_stats(&mut session, 1_000, 40);
    attach_stat_update_player_with_mana_and_health(&mut session, player_guid, 777, 1_320, 5, 10);
    session.set_player_health_like_cpp(5, 10);
    session.set_spell_store(Arc::new(total_stat_percentage_spell_store_like_cpp(
        spell_id, false,
    )));
    session.set_state(crate::session::SessionState::LoggedIn);

    session
        .apply_aura(spell_id, player_guid, 30_000, 1)
        .expect("apply non-ability stamina aura");

    assert_eq!(
        session.canonical_player_health_snapshot_like_cpp(),
        Some((5, 20)),
        "C++ only preserves health percentage for SPELL_ATTR0_IS_ABILITY"
    );
}

#[test]
fn login_passive_total_stat_aura_defers_values_update_until_create_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(4);
    let player_guid = ObjectGuid::create_player(1, 84);
    let spell_id = 90_085;
    session.set_player_guid(Some(player_guid));
    session.set_loaded_player_identity_like_cpp(571, 1, 5, 80, 0);
    set_priest_level80_stats(&mut session, 1_000, 40);
    attach_stat_update_player_with_mana_and_health(&mut session, player_guid, 777, 1_320, 5, 10);
    session.set_known_spells_like_cpp(vec![spell_id]);
    let mut spell_store = total_stat_percentage_spell_store_like_cpp(spell_id, false);
    let mut attributes = [0; 15];
    attributes[0] = wow_data::spell::attributes::SPELL_ATTR0_PASSIVE;
    spell_store.insert_spell_misc_attributes_like_cpp(spell_id, attributes);
    session.set_spell_store(Arc::new(spell_store));

    assert_eq!(session.state(), crate::session::SessionState::Authed);
    assert_eq!(session.apply_login_passive_known_spell_auras_like_cpp(), 1);
    assert_eq!(
        session.represented_total_stat_multipliers_like_cpp(),
        [1.0, 1.0, 2.0, 1.0, 1.0]
    );
    let opcodes = drain_server_opcodes(&send_rx);
    assert!(
        !opcodes.contains(&ServerOpcodes::UpdateObject),
        "C++ folds login passive modifiers into UpdateAllStats/CreateObject instead of sending pre-create VALUES"
    );
}

#[test]
fn login_combat_snapshot_clamps_saved_health_after_persisted_stat_auras_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send_capacity(2);
    let player_guid = ObjectGuid::create_player(1, 85);
    let spell_id = 90_086;
    session.set_player_guid(Some(player_guid));
    session.set_loaded_player_identity_like_cpp(571, 1, 5, 80, 0);
    set_priest_level80_stats(&mut session, 1_000, 40);
    session.set_spell_store(Arc::new(total_stat_percentage_spell_store_like_cpp(
        spell_id, false,
    )));

    assert_eq!(
        session.load_represented_character_auras_like_cpp(
            [crate::session::CharacterAuraRowLikeCpp {
                caster_guid: player_guid,
                spell_id: spell_id as u32,
                effect_mask: 1,
                recalculate_mask: 0,
                difficulty: 0,
                stack_count: 1,
                max_duration_ms: -1,
                remain_time_ms: -1,
                remain_charges: 0,
            }],
            [crate::session::CharacterAuraEffectRowLikeCpp {
                caster_guid: player_guid,
                spell_id: spell_id as u32,
                effect_mask: 1,
                effect_index: 0,
                amount: 100,
                base_amount: 100,
            }],
            0,
        ),
        1
    );

    let (combat, _, current_power0) = session
        .player_login_combat_stats_like_cpp(1, 5, 80, Some(15), 2_000)
        .expect("login combat snapshot");
    assert_eq!(combat.max_health, 20);
    assert_eq!(
        combat.health, 15,
        "saved health valid under the persisted stamina aura must not be pre-clamped to the unbuffed max"
    );
    assert_eq!(
        current_power0, 1_320,
        "saved primary power is clamped after the final aura/item projection like C++"
    );
}

#[test]
fn login_passive_parry_and_block_capabilities_feed_first_stat_projection_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send_capacity(1);
    let player_guid = ObjectGuid::create_player(1, 86);
    session.set_player_guid(Some(player_guid));
    session.set_loaded_player_identity_like_cpp(571, 1, 1, 80, 0);
    session.set_player_stats(Arc::new(PlayerStatsStore::from_entries([(
        (1, 1, 80),
        PlayerLevelStats {
            strength: 50,
            agility: 30,
            stamina: 40,
            intellect: 10,
            spirit: 20,
            base_mana: 0,
        },
    )])));
    session.set_chr_classes_store(Arc::new(ChrClassesStore::from_entries([chr_class_entry(
        1, 0,
    )])));
    attach_stat_update_player_with_mana_and_health(&mut session, player_guid, 0, 0, 100, 220);
    let parry_spell_id = 90_087;
    let block_spell_id = 90_088;
    session.set_spell_store(Arc::new(passive_combat_capability_spell_store_like_cpp(
        parry_spell_id,
        block_spell_id,
    )));

    assert_eq!(
        session.canonical_player_parry_block_snapshot_like_cpp(),
        (false, false)
    );
    assert_eq!(
        session.apply_login_known_spell_combat_capabilities_like_cpp(&[
            parry_spell_id,
            block_spell_id,
        ]),
        2
    );
    assert_eq!(
        session.canonical_player_parry_block_snapshot_like_cpp(),
        (true, true)
    );
    let projection = session
        .player_stat_system_projection_like_cpp(
            1,
            1,
            80,
            &RepresentedPlayerGearStatsLikeCpp::default(),
        )
        .expect("warrior stat projection");
    assert_eq!(projection.parry_pct, 5.0);
    assert_eq!(projection.block_pct, 5.0);
}

#[test]
fn login_stat_update_derives_and_syncs_loaded_enchantment_bonuses_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send_capacity(4);
    let player_guid = ObjectGuid::create_player(1, 81);
    let item_guid = ObjectGuid::create_item(1, 82);
    session.set_player_guid(Some(player_guid));
    session.set_loaded_player_identity_like_cpp(571, 1, 5, 80, 0);
    set_priest_level80_stats(&mut session, 1000, 40);
    attach_stat_update_player_with_mana_and_health(&mut session, player_guid, 777, 1320, 77, 110);
    session
        .mutate_canonical_player_like_cpp(|player| player.unit_mut().set_level(80))
        .unwrap();
    session.set_spell_item_enchantment_store(Arc::new(
        wow_data::SpellItemEnchantmentStore::from_entries([
            wow_data::SpellItemEnchantmentEntry {
                id: 920,
                effect_arg: [
                    wow_constants::ItemModType::Stamina as u32,
                    wow_constants::ItemModType::Mana as u32,
                    wow_constants::ItemModType::Strength as u32,
                ],
                effect_points_min: [3, 4, 2],
                item_visual: 0,
                flags: wow_constants::SpellItemEnchantmentFlags::empty(),
                required_skill_id: 0,
                required_skill_rank: 0,
                item_level: 1,
                charges: 0,
                effect: [wow_constants::ItemEnchantmentType::Stat as u8; 3],
                condition_id: 0,
                min_level: 1,
                max_level: 0,
            },
            wow_data::SpellItemEnchantmentEntry {
                id: 921,
                effect_arg: [wow_constants::ItemModType::ManaRegeneration as u32, 0, 0],
                effect_points_min: [25, 0, 0],
                item_visual: 0,
                flags: wow_constants::SpellItemEnchantmentFlags::empty(),
                required_skill_id: 0,
                required_skill_rank: 0,
                item_level: 1,
                charges: 0,
                effect: [
                    wow_constants::ItemEnchantmentType::Stat as u8,
                    wow_constants::ItemEnchantmentType::None as u8,
                    wow_constants::ItemEnchantmentType::None as u8,
                ],
                condition_id: 0,
                min_level: 1,
                max_level: 0,
            },
        ]),
    ));
    let mut item = session.make_inventory_item_object(
        item_guid,
        700,
        player_guid,
        1,
        0,
        ItemContext::None,
        wow_entities::EQUIPMENT_SLOT_CHEST,
    );
    item.set_enchantment(EnchantmentSlot::EnhancementPermanent, 920, 0, 0);
    item.set_enchantment(EnchantmentSlot::EnhancementTemporary, 921, 0, 0);
    session.insert_inventory_item_object(item);

    let outcome = session.apply_loaded_equipped_item_enchantments_like_cpp(item_guid);
    assert!(outcome.send_stat_update);
    assert_eq!(
        session.represented_item_bonus_state_like_cpp().stats_base,
        [2, 0, 3, 0, 0]
    );
    assert_eq!(session.represented_item_bonus_state_like_cpp().mana_base, 4);
    assert_eq!(
        session
            .represented_item_bonus_state_like_cpp()
            .mana_regen_bonus,
        25
    );
    let (_, changes) = session
        .player_stat_changes_with_represented_item_bonuses_like_cpp(true)
        .expect("login stat changes with loaded enchantments");

    assert_eq!(changes.health, 13, "current health clamps to the new max");
    assert_eq!(
        changes.max_health, 13,
        "CreateHealth is zero and stamina supplies max HP"
    );
    assert_eq!(changes.power0, 777, "current mana remains authoritative");
    assert_eq!(
        changes.max_power0, 1324,
        "flat mana is derived before max power"
    );
    assert_eq!(changes.stats, [12, 10, 13, 40, 30]);
    assert_eq!(
        changes.attack_power, -20,
        "ChrClasses priest AP coefficients"
    );
    assert_eq!(changes.mana_regen_combat, 5.0);
    assert_eq!(changes.mana_regen_mp5, 0.0);
    assert!(changes.mana_regen > changes.mana_regen_combat);
    assert_eq!(
        session.canonical_player_health_snapshot_like_cpp(),
        Some((13, 13))
    );
    assert_eq!(
        session.canonical_player_power_snapshot_like_cpp(PowerType::Mana),
        Some((777, 1324))
    );

    let spell_id = 90_084;
    session.set_spell_store(Arc::new(total_stat_percentage_spell_store_like_cpp(
        spell_id, false,
    )));
    session.set_state(crate::session::SessionState::LoggedIn);
    session
        .apply_aura(spell_id, player_guid, 30_000, 1)
        .expect("apply total-stat aura over loaded enchantments");
    assert_eq!(
        session.canonical_player_health_snapshot_like_cpp(),
        Some((13, 80)),
        "absolute aura recalc keeps the loaded +3 stamina enchant before doubling stamina"
    );

    let slot = session
        .visible_aura_slot_for_spell_like_cpp(spell_id)
        .expect("total-stat aura slot");
    session.remove_aura(slot).expect("remove total-stat aura");
    assert_eq!(
        session.canonical_player_health_snapshot_like_cpp(),
        Some((13, 13)),
        "absolute aura removal keeps the loaded enchantment bonus"
    );
}

#[tokio::test]
async fn show_trade_skill_is_noop_null_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(1);
    session.set_player_guid(Some(ObjectGuid::create_player(1, 42)));

    session.handle_show_trade_skill().await;

    assert!(send_rx.try_recv().is_err());
}

#[test]
fn login_known_spells_include_account_mounts_even_when_use_condition_fails_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send_capacity(8);
    session.set_loaded_player_identity_like_cpp(571, 1, 1, 80, 0);
    session.set_known_spells_like_cpp(vec![635]);
    session.set_mount_store(Arc::new(wow_data::MountStore::from_entries([
        wow_data::MountEntry {
            id: 1,
            mount_type_id: 0,
            flags: 0,
            source_type_enum: 0,
            source_spell_id: 100,
            player_condition_id: 42,
            mount_fly_ride_height: 0.0,
            ui_model_scene_id: 0,
        },
        wow_data::MountEntry {
            id: 2,
            mount_type_id: 0,
            flags: 0,
            source_type_enum: 0,
            source_spell_id: 101,
            player_condition_id: 43,
            mount_fly_ride_height: 0.0,
            ui_model_scene_id: 0,
        },
    ])));
    session.set_player_condition_store(Arc::new(wow_data::PlayerConditionStore::from_entries([
        wow_data::PlayerConditionEntry {
            id: 42,
            class_mask: 1,
            ..Default::default()
        },
        wow_data::PlayerConditionEntry {
            id: 43,
            class_mask: 1 << 1,
            ..Default::default()
        },
    ])));

    session.set_account_mounts_like_cpp(vec![
        AccountMount {
            spell_id: 100,
            flags: 0,
        },
        AccountMount {
            spell_id: 101,
            flags: 0,
        },
    ]);

    let login_spells = session.login_known_spells_after_account_collections_like_cpp();
    assert!(login_spells.contains(&635));
    assert!(login_spells.contains(&100));
    assert!(
        login_spells.contains(&101),
        "C++ CollectionMgr::AddMount stores/learns the mount before evaluating PlayerCondition; the condition applies to using it"
    );
}

#[test]
fn send_known_spells_filters_disabled_and_inactive_like_cpp() {
    assert_eq!(active_known_spell_for_send_like_cpp(118, 1, 0), Some(118));
    assert_eq!(
        active_known_spell_for_send_like_cpp(118, 0, 0),
        None,
        "C++ Player::SendKnownSpells skips inactive spells"
    );
    assert_eq!(
        active_known_spell_for_send_like_cpp(118, 1, 1),
        None,
        "C++ Player::SendKnownSpells skips disabled spells"
    );
    assert_eq!(active_known_spell_for_send_like_cpp(0, 1, 0), None);
}

#[test]
fn login_known_spells_filters_complete_has_spell_mirror_like_cpp() {
    let (mut session, _) = make_session_with_send_capacity(1);
    session.set_known_spells_like_cpp(vec![100, 200]);
    assert!(
        session.set_complete_represented_player_spell_rows_like_cpp([
            crate::session::RepresentedPlayerSpellLikeCpp {
                spell_id: 100,
                active: false,
                disabled: false,
                dependent: false,
                favorite: false,
                state: crate::session::RepresentedPlayerSpellStateLikeCpp::Unchanged,
            },
            crate::session::RepresentedPlayerSpellLikeCpp {
                spell_id: 200,
                active: true,
                disabled: false,
                dependent: false,
                favorite: false,
                state: crate::session::RepresentedPlayerSpellStateLikeCpp::Unchanged,
            },
            crate::session::RepresentedPlayerSpellLikeCpp {
                spell_id: 300,
                active: true,
                disabled: true,
                dependent: false,
                favorite: false,
                state: crate::session::RepresentedPlayerSpellStateLikeCpp::Unchanged,
            },
        ])
    );

    assert_eq!(session.known_spells_like_cpp(), &[100, 200]);
    assert_eq!(
        session.login_known_spells_after_account_collections_like_cpp(),
        vec![200],
        "C++ Player::SendKnownSpells excludes inactive and disabled PlayerSpellMap rows"
    );
}

#[test]
fn load_spells_keeps_inactive_non_disabled_spells_for_add_spell_side_effects_like_cpp() {
    assert_eq!(
        loaded_spell_for_add_spell_side_effects_like_cpp(118, 0),
        Some(118),
        "C++ Player::_LoadSpells still calls AddSpell for inactive rows; SendKnownSpells filters them later"
    );
    assert_eq!(
        loaded_spell_for_add_spell_side_effects_like_cpp(118, 1),
        None,
        "C++ AddSpell returns before cast side effects for disabled spell rows"
    );
    assert_eq!(loaded_spell_for_add_spell_side_effects_like_cpp(0, 0), None);
}

#[test]
fn login_skill_reward_spells_retain_cpp_dependent_ownership() {
    let mut known = vec![100, 200];
    let mut side_effects = vec![100, 200];
    let mut dependent = HashSet::from([200]);
    let mut removed = HashSet::new();

    apply_skill_rewarded_spell_changes_to_login_like_cpp(
        &mut known,
        &mut side_effects,
        &mut dependent,
        &mut removed,
        wow_data::SkillRewardedSpellChangesLikeCpp {
            learn: vec![300],
            remove: vec![200],
        },
    );

    assert_eq!(known, vec![100, 300]);
    assert_eq!(side_effects, vec![100, 300]);
    assert_eq!(dependent, HashSet::from([300]));
    assert_eq!(removed, HashSet::from([200]));
}

#[test]
fn send_known_spells_favorites_are_subset_of_sent_spells_like_cpp() {
    let favorites = HashSet::from([635, 999]);

    assert_eq!(
        favorite_known_spells_for_send_like_cpp(&[118, 635, 133], &favorites),
        vec![635],
        "C++ only marks favorite spells while iterating spells that are actually sent"
    );
}

#[test]
fn spell_history_entry_from_db_splits_spell_and_category_cooldowns_like_cpp() {
    let entry = spell_history_entry_from_db_like_cpp(133, 6948, 1_030, 12, 1_010, 1_000)
        .expect("future cooldown should be serialized");

    assert_eq!(entry.spell_id, 133);
    assert_eq!(entry.item_id, 6948);
    assert_eq!(entry.category, 12);
    assert_eq!(entry.recovery_time_ms, 30_000);
    assert_eq!(entry.category_recovery_time_ms, 10_000);
    assert_eq!(entry.mod_rate, 1.0);
    assert!(!entry.on_hold);
}

#[test]
fn spell_history_entry_omits_recovery_when_category_last_longer_like_cpp() {
    let entry = spell_history_entry_from_db_like_cpp(133, 0, 1_005, 12, 1_010, 1_000)
        .expect("future category cooldown should be serialized");

    assert_eq!(entry.category, 12);
    assert_eq!(entry.recovery_time_ms, 0);
    assert_eq!(entry.category_recovery_time_ms, 10_000);
}

#[test]
fn spell_history_entry_skips_expired_cooldowns_like_cpp() {
    assert_eq!(
        spell_history_entry_from_db_like_cpp(133, 0, 1_000, 12, 1_010, 1_000),
        None
    );
}

#[test]
fn spell_charge_entry_uses_first_recharge_and_consumed_count_like_cpp() {
    let entry = spell_charge_entry_from_db_like_cpp(42, 1_045, 2, 1_000)
        .expect("future charge should be serialized");

    assert_eq!(entry.category, 42);
    assert_eq!(entry.next_recovery_time_ms, 45_000);
    assert_eq!(entry.charge_mod_rate, 1.0);
    assert_eq!(entry.consumed_charges, 2);
}

#[test]
fn spell_charge_entry_skips_expired_recharges_like_cpp() {
    assert_eq!(
        spell_charge_entry_from_db_like_cpp(42, 1_000, 1, 1_000),
        None
    );
}

#[test]
fn account_mount_spells_are_dependent_and_not_saved_to_character_spell_like_cpp() {
    assert!(
        WorldSession::account_mount_spells_are_session_dependent_like_cpp(),
        "C++ CollectionMgr::AddMount calls Player::LearnSpell(spellId, true); Player::_SaveSpells skips dependent spells, so account mounts must not be persisted into character_spell"
    );
}

#[test]
fn create_character_seeds_valid_cpp_rest_state_for_raf_roles() {
    assert_eq!(
        initial_character_rest_state_like_cpp(false, 0),
        REST_STATE_NORMAL_LIKE_CPP
    );
    assert_eq!(
        initial_character_rest_state_like_cpp(false, 7),
        REST_STATE_RAF_LINKED_LIKE_CPP
    );
    assert_eq!(
        initial_character_rest_state_like_cpp(true, 0),
        REST_STATE_RAF_LINKED_LIKE_CPP
    );
}

#[test]
fn default_character_power1_seeds_energy_classes_like_cpp() {
    assert_eq!(
        default_character_power1_like_cpp(4, 0),
        100,
        "C++ level-1 rogues enter with full base Energy, not zeroed mana"
    );
    assert_eq!(default_character_power1_like_cpp(5, 160), 160);
    assert_eq!(default_character_power1_like_cpp(1, 0), 0);
}

#[test]
fn character_rename_name_validation_matches_represented_cpp_gates() {
    assert_eq!(
        WorldSession::represented_character_rename_name_result_like_cpp(""),
        CHAR_NAME_NO_NAME_LIKE_CPP
    );
    assert_eq!(
        WorldSession::represented_character_rename_name_result_like_cpp("A"),
        CHAR_NAME_TOO_SHORT_LIKE_CPP
    );
    assert_eq!(
        WorldSession::represented_character_rename_name_result_like_cpp("VeryLongNameX"),
        CHAR_NAME_TOO_LONG_LIKE_CPP
    );
    assert_eq!(
        WorldSession::represented_character_rename_name_result_like_cpp("Bad1"),
        CHAR_NAME_INVALID_CHARACTER_LIKE_CPP
    );
    assert_eq!(
        WorldSession::represented_character_rename_name_result_like_cpp("Newname"),
        RESPONSE_SUCCESS_LIKE_CPP
    );
}

#[tokio::test]
async fn character_rename_invalid_name_sends_cpp_result_without_guid() {
    let (mut session, send_rx) = make_session_with_send_capacity(2);
    let guid = ObjectGuid::create_player(1, 42);
    session.set_legit_characters(vec![guid]);

    session
        .handle_character_rename_request(CharacterRenameRequest {
            guid,
            new_name: String::new(),
        })
        .await;

    let sent = send_rx.try_recv().expect("rename result");
    let mut pkt = WorldPacket::from_bytes(&sent);
    assert_eq!(
        pkt.server_opcode(),
        Some(ServerOpcodes::CharacterRenameResult)
    );
    pkt.skip_opcode();
    assert_eq!(pkt.read_uint8().unwrap(), CHAR_NAME_NO_NAME_LIKE_CPP);
    assert!(!pkt.read_bit().unwrap());
    assert_eq!(pkt.read_bits(6).unwrap(), 0);
    assert_eq!(pkt.remaining(), 0);
}

#[tokio::test]
async fn character_rename_non_owned_guid_kicks_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(1);

    session
        .handle_character_rename_request(CharacterRenameRequest {
            guid: ObjectGuid::create_player(1, 42),
            new_name: "Newname".to_string(),
        })
        .await;

    assert_eq!(session.state(), crate::session::SessionState::Disconnecting);
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn char_customize_without_character_db_sends_cpp_failure() {
    let (mut session, send_rx) = make_session_with_send_capacity(2);
    let guid = ObjectGuid::create_player(1, 42);
    session.set_legit_characters(vec![guid]);

    session
        .handle_char_customize(CharCustomize {
            guid,
            sex_id: 1,
            customizations: vec![],
            name: "Newname".to_string(),
        })
        .await;

    let sent = send_rx.try_recv().expect("customize failure");
    let mut pkt = WorldPacket::from_bytes(&sent);
    assert_eq!(
        pkt.server_opcode(),
        Some(ServerOpcodes::CharCustomizeFailure)
    );
    pkt.skip_opcode();
    assert_eq!(pkt.read_uint8().unwrap(), CHAR_CREATE_ERROR_LIKE_CPP);
    assert_eq!(pkt.read_guid().unwrap(), guid);
    assert_eq!(pkt.remaining(), 0);
}

#[tokio::test]
async fn char_customize_non_owned_guid_kicks_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(1);

    session
        .handle_char_customize(CharCustomize {
            guid: ObjectGuid::create_player(1, 42),
            sex_id: 1,
            customizations: vec![],
            name: "Newname".to_string(),
        })
        .await;

    assert_eq!(session.state(), crate::session::SessionState::Disconnecting);
    assert!(send_rx.try_recv().is_err());
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

#[tokio::test]
async fn alter_appearance_without_barber_chair_sends_not_on_chair_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(4);
    session.set_player_guid(Some(ObjectGuid::create_player(1, 42)));
    session.set_loaded_player_identity_like_cpp(571, 1, 1, 80, 0);

    session
        .handle_alter_appearance(alter_appearance_packet(1, 1, 0, &[(20, 200)]))
        .await;

    assert_eq!(
        read_barber_shop_result(send_rx.try_recv().unwrap()),
        BARBER_SHOP_RESULT_NOT_ON_CHAIR_LIKE_CPP
    );
    assert!(
        session
            .represented_alter_appearance_requests_like_cpp()
            .is_empty()
    );
}

#[tokio::test]
async fn set_player_declined_names_without_runtime_sends_error_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(1);
    let player = ObjectGuid::create_player(1, 42);

    session
        .handle_set_player_declined_names(declined_names_packet(
            player,
            ["Gen", "Dat", "Acc", "Inst", "Prep"],
        ))
        .await;

    assert_eq!(
        read_declined_names_result(send_rx.try_recv().unwrap()),
        (DECLINED_NAMES_RESULT_ERROR_LIKE_CPP, player)
    );
}

#[tokio::test]
async fn set_player_declined_names_short_packet_does_not_send_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(1);

    session
        .handle_set_player_declined_names(WorldPacket::from_bytes(&[0x2a, 0x00]))
        .await;

    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn save_equipment_set_new_equipment_normalizes_and_sends_id_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(1);
    let item_guid = ObjectGuid::create_item(1, 55);
    session.insert_inventory_item_like_cpp(
        0,
        InventoryItem {
            guid: item_guid,
            entry_id: 100,
            db_guid: 55,
            inventory_type: Some(InventoryType::Head as u8),
        },
    );
    let mut pieces = [ObjectGuid::EMPTY; wow_packet::packets::misc::EQUIPMENT_SET_SLOTS_LIKE_CPP];
    pieces[0] = item_guid;
    let appearances = [77; wow_packet::packets::misc::EQUIPMENT_SET_SLOTS_LIKE_CPP];

    session
        .handle_save_equipment_set(save_equipment_set_packet(
            0,
            0,
            7,
            0,
            pieces,
            appearances,
            [12, 34],
            Some(2),
            "Tank",
            "INV_Helmet_01",
        ))
        .await;

    let (generated_guid, set_type, set_id) = read_equipment_set_id(send_rx.try_recv().unwrap());
    assert_eq!((generated_guid, set_type, set_id), (1, 0, 7));
    let saved = session
        .represented_equipment_set_like_cpp(generated_guid)
        .unwrap();
    assert_eq!(saved.guid, generated_guid);
    assert_eq!(saved.set_id, 7);
    assert_eq!(saved.set_name, "Tank");
    assert_eq!(saved.set_icon, "INV_Helmet_01");
    assert_eq!(saved.pieces[0], item_guid);
    assert_eq!(saved.appearances[0], 0);
    assert_eq!(saved.appearances[1], 0);
    assert_eq!(saved.enchants, [0, 0]);
    assert_eq!(saved.assigned_spec_index, 2);
    assert_eq!(
        saved.state,
        crate::session::RepresentedEquipmentSetUpdateStateLikeCpp::New
    );
    assert_ne!(saved.ignore_mask & (1 << 1), 0);
}

#[tokio::test]
async fn save_equipment_set_requires_process_wide_guid_allocator() {
    let (_pkt_tx, pkt_rx) = flume::bounded::<WorldPacket>(1);
    let (send_tx, send_rx) = flume::bounded::<Vec<u8>>(1);
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
    let ignore_mask = (1_u32 << wow_packet::packets::misc::EQUIPMENT_SET_SLOTS_LIKE_CPP) - 1;

    session
        .handle_save_equipment_set(save_equipment_set_packet(
            0,
            0,
            7,
            ignore_mask,
            [ObjectGuid::EMPTY; wow_packet::packets::misc::EQUIPMENT_SET_SLOTS_LIKE_CPP],
            [0; wow_packet::packets::misc::EQUIPMENT_SET_SLOTS_LIKE_CPP],
            [0, 0],
            None,
            "No allocator",
            "INV_Misc_QuestionMark",
        ))
        .await;

    assert!(send_rx.try_recv().is_err());
    assert!(session.represented_equipment_set_like_cpp(1).is_none());
}

#[tokio::test]
async fn concurrent_sessions_share_equipment_and_transmog_set_guid_namespace_like_cpp() {
    let (mut equipment_session, equipment_rx) = make_session_with_send_capacity(1);
    let (mut transmog_session, transmog_rx) = make_session_with_send_capacity(1);
    let generator = Arc::new(EquipmentSetGuidGeneratorLikeCpp::new(400));
    equipment_session.set_equipment_set_guid_generator_like_cpp(Arc::clone(&generator));
    transmog_session.set_equipment_set_guid_generator_like_cpp(Arc::clone(&generator));
    let ignore_mask = (1_u32 << wow_packet::packets::misc::EQUIPMENT_SET_SLOTS_LIKE_CPP) - 1;

    tokio::join!(
        equipment_session.handle_save_equipment_set(save_equipment_set_packet(
            0,
            0,
            7,
            ignore_mask,
            [ObjectGuid::EMPTY; wow_packet::packets::misc::EQUIPMENT_SET_SLOTS_LIKE_CPP],
            [0; wow_packet::packets::misc::EQUIPMENT_SET_SLOTS_LIKE_CPP],
            [0, 0],
            None,
            "Equipment",
            "INV_Sword_01",
        )),
        transmog_session.handle_save_equipment_set(save_equipment_set_packet(
            1,
            0,
            8,
            ignore_mask,
            [ObjectGuid::EMPTY; wow_packet::packets::misc::EQUIPMENT_SET_SLOTS_LIKE_CPP],
            [0; wow_packet::packets::misc::EQUIPMENT_SET_SLOTS_LIKE_CPP],
            [0, 0],
            None,
            "Transmog",
            "INV_Chest_Cloth_01",
        )),
    );

    let equipment = read_equipment_set_id(equipment_rx.try_recv().unwrap());
    let transmog = read_equipment_set_id(transmog_rx.try_recv().unwrap());
    let mut guids = [equipment.0, transmog.0];
    guids.sort_unstable();
    assert_eq!(guids, [400, 401]);
    assert_eq!(equipment.1, 0);
    assert_eq!(transmog.1, 1);
    assert_eq!(generator.next_after_max_used(), 402);
}

#[tokio::test]
async fn save_equipment_set_existing_marks_changed_without_id_packet_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(1);
    session.insert_represented_equipment_set_like_cpp(
        100,
        crate::session::RepresentedEquipmentSetLikeCpp::equipment(
            7,
            -1,
            crate::session::RepresentedEquipmentSetUpdateStateLikeCpp::Unchanged,
        ),
    );
    let ignore_mask = (1_u32 << wow_packet::packets::misc::EQUIPMENT_SET_SLOTS_LIKE_CPP) - 1;

    session
        .handle_save_equipment_set(save_equipment_set_packet(
            0,
            100,
            7,
            ignore_mask,
            [ObjectGuid::EMPTY; wow_packet::packets::misc::EQUIPMENT_SET_SLOTS_LIKE_CPP],
            [0; wow_packet::packets::misc::EQUIPMENT_SET_SLOTS_LIKE_CPP],
            [0, 0],
            None,
            "Dps",
            "INV_Sword_01",
        ))
        .await;

    assert!(send_rx.try_recv().is_err());
    let saved = session.represented_equipment_set_like_cpp(100).unwrap();
    assert_eq!(saved.set_name, "Dps");
    assert_eq!(saved.assigned_spec_index, -1);
    assert_eq!(
        saved.state,
        crate::session::RepresentedEquipmentSetUpdateStateLikeCpp::Changed
    );
}

#[tokio::test]
async fn save_equipment_set_negative_type_follows_cpp_non_equipment_branch() {
    let (mut session, send_rx) = make_session_with_send_capacity(1);
    let ignore_mask = (1_u32 << wow_packet::packets::misc::EQUIPMENT_SET_SLOTS_LIKE_CPP) - 1;

    session
        .handle_save_equipment_set(save_equipment_set_packet(
            -1,
            0,
            7,
            ignore_mask,
            [ObjectGuid::EMPTY; wow_packet::packets::misc::EQUIPMENT_SET_SLOTS_LIKE_CPP],
            [0; wow_packet::packets::misc::EQUIPMENT_SET_SLOTS_LIKE_CPP],
            [0, 0],
            None,
            "Odd",
            "INV_Odd",
        ))
        .await;

    let (generated_guid, set_type, set_id) = read_equipment_set_id(send_rx.try_recv().unwrap());
    assert_eq!((generated_guid, set_type, set_id), (1, -1, 7));
    let saved = session
        .represented_equipment_set_like_cpp(generated_guid)
        .unwrap();
    assert_eq!(saved.raw_set_type, -1);
    assert_eq!(
        saved.set_type,
        crate::session::RepresentedEquipmentSetTypeLikeCpp::Transmog
    );
}

#[tokio::test]
async fn save_equipment_set_rejects_equipment_guid_mismatch_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(1);
    session.insert_inventory_item_like_cpp(
        0,
        InventoryItem {
            guid: ObjectGuid::create_item(1, 55),
            entry_id: 100,
            db_guid: 55,
            inventory_type: Some(InventoryType::Head as u8),
        },
    );
    let mut pieces = [ObjectGuid::EMPTY; wow_packet::packets::misc::EQUIPMENT_SET_SLOTS_LIKE_CPP];
    pieces[0] = ObjectGuid::create_item(1, 99);

    session
        .handle_save_equipment_set(save_equipment_set_packet(
            0,
            0,
            7,
            0,
            pieces,
            [0; wow_packet::packets::misc::EQUIPMENT_SET_SLOTS_LIKE_CPP],
            [0, 0],
            None,
            "Bad",
            "INV_Bad",
        ))
        .await;

    assert!(send_rx.try_recv().is_err());
    assert!(session.represented_equipment_set_like_cpp(1).is_none());
}

#[tokio::test]
async fn assign_equipment_set_spec_updates_matching_equipment_set_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(1);
    session.insert_represented_equipment_set_like_cpp(
        100,
        crate::session::RepresentedEquipmentSetLikeCpp::equipment(
            7,
            -1,
            crate::session::RepresentedEquipmentSetUpdateStateLikeCpp::Unchanged,
        ),
    );

    session
        .handle_assign_equipment_set_spec(assign_equipment_set_spec_packet(7, 2))
        .await;

    let equipment_set = session.represented_equipment_set_like_cpp(100).unwrap();
    assert_eq!(equipment_set.assigned_spec_index, 2);
    assert_eq!(
        equipment_set.state,
        crate::session::RepresentedEquipmentSetUpdateStateLikeCpp::Changed
    );
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn assign_equipment_set_spec_preserves_new_state_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send_capacity(1);
    session.insert_represented_equipment_set_like_cpp(
        100,
        crate::session::RepresentedEquipmentSetLikeCpp::equipment(
            7,
            -1,
            crate::session::RepresentedEquipmentSetUpdateStateLikeCpp::New,
        ),
    );

    session
        .handle_assign_equipment_set_spec(assign_equipment_set_spec_packet(7, 3))
        .await;

    let equipment_set = session.represented_equipment_set_like_cpp(100).unwrap();
    assert_eq!(equipment_set.assigned_spec_index, 3);
    assert_eq!(
        equipment_set.state,
        crate::session::RepresentedEquipmentSetUpdateStateLikeCpp::New
    );
}

#[tokio::test]
async fn assign_equipment_set_spec_ignores_transmog_missing_and_out_of_range_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send_capacity(1);
    session.insert_represented_equipment_set_like_cpp(
        100,
        crate::session::RepresentedEquipmentSetLikeCpp::transmog(
            7,
            -1,
            crate::session::RepresentedEquipmentSetUpdateStateLikeCpp::Unchanged,
        ),
    );
    session.insert_represented_equipment_set_like_cpp(
        200,
        crate::session::RepresentedEquipmentSetLikeCpp::equipment(
            8,
            -1,
            crate::session::RepresentedEquipmentSetUpdateStateLikeCpp::Unchanged,
        ),
    );

    session
        .handle_assign_equipment_set_spec(assign_equipment_set_spec_packet(7, 4))
        .await;
    session
        .handle_assign_equipment_set_spec(assign_equipment_set_spec_packet(99, 4))
        .await;
    session
        .handle_assign_equipment_set_spec(assign_equipment_set_spec_packet(
            crate::session::MAX_EQUIPMENT_SET_INDEX_LIKE_CPP,
            4,
        ))
        .await;

    assert_eq!(
        session
            .represented_equipment_set_like_cpp(100)
            .unwrap()
            .assigned_spec_index,
        -1
    );
    assert_eq!(
        session
            .represented_equipment_set_like_cpp(200)
            .unwrap()
            .assigned_spec_index,
        -1
    );
}

#[tokio::test]
async fn delete_equipment_set_marks_existing_set_deleted_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(1);
    session.insert_represented_equipment_set_like_cpp(
        100,
        crate::session::RepresentedEquipmentSetLikeCpp::equipment(
            7,
            -1,
            crate::session::RepresentedEquipmentSetUpdateStateLikeCpp::Unchanged,
        ),
    );

    session
        .handle_delete_equipment_set(delete_equipment_set_packet(100))
        .await;

    let equipment_set = session.represented_equipment_set_like_cpp(100).unwrap();
    assert_eq!(
        equipment_set.state,
        crate::session::RepresentedEquipmentSetUpdateStateLikeCpp::Deleted
    );
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn delete_equipment_set_removes_new_set_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(1);
    session.insert_represented_equipment_set_like_cpp(
        100,
        crate::session::RepresentedEquipmentSetLikeCpp::equipment(
            7,
            -1,
            crate::session::RepresentedEquipmentSetUpdateStateLikeCpp::New,
        ),
    );

    session
        .handle_delete_equipment_set(delete_equipment_set_packet(100))
        .await;

    assert!(session.represented_equipment_set_like_cpp(100).is_none());
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn delete_equipment_set_missing_id_is_silent_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(1);

    session
        .handle_delete_equipment_set(delete_equipment_set_packet(404))
        .await;

    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn use_equipment_set_moves_direct_inventory_item_and_sends_result_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(1);
    let item_guid = ObjectGuid::create_item(1, 55);
    session.insert_inventory_item_like_cpp(
        INVENTORY_SLOT_ITEM_START,
        InventoryItem {
            guid: item_guid,
            entry_id: 100,
            db_guid: 55,
            inventory_type: Some(InventoryType::Head as u8),
        },
    );
    let mut items = [ObjectGuid::EMPTY; wow_packet::packets::misc::EQUIPMENT_SET_SLOTS_LIKE_CPP];
    items[0] = item_guid;

    session
        .handle_use_equipment_set(use_equipment_set_packet(0x0102_0304_0506_0708, items))
        .await;

    assert_eq!(
        read_use_equipment_set_result(send_rx.try_recv().unwrap()),
        (0x0102_0304_0506_0708, 0)
    );
    assert_eq!(
        session
            .get_inventory_item_by_pos(INVENTORY_SLOT_BAG_0, 0)
            .unwrap()
            .guid,
        item_guid
    );
    assert!(
        session
            .get_inventory_item_by_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START)
            .is_none()
    );
}

#[tokio::test]
async fn use_equipment_set_applies_represented_item_mods_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(4);
    let player_guid = ObjectGuid::create_player(1, 42);
    let item_guid = ObjectGuid::create_item(1, 66);
    let entry_id = 111;
    session.set_player_guid(Some(player_guid));
    session.set_item_stats_store(Arc::new(strength_item_stats_store(entry_id, 11)));
    session.insert_inventory_item_like_cpp(
        INVENTORY_SLOT_ITEM_START,
        InventoryItem {
            guid: item_guid,
            entry_id,
            db_guid: 66,
            inventory_type: Some(InventoryType::Weapon as u8),
        },
    );
    let item = session.make_inventory_item_object(
        item_guid,
        entry_id,
        player_guid,
        1,
        0,
        ItemContext::None,
        INVENTORY_SLOT_ITEM_START,
    );
    session.insert_inventory_item_object(item);
    let mut items = [ObjectGuid::EMPTY; wow_packet::packets::misc::EQUIPMENT_SET_SLOTS_LIKE_CPP];
    items[EQUIPMENT_SLOT_MAINHAND as usize] = item_guid;

    session
        .handle_use_equipment_set(use_equipment_set_packet(0x0203, items))
        .await;

    assert_eq!(
        session.represented_item_bonus_state_like_cpp().stats_base[0],
        11,
        "C++ HandleUseEquipmentSet reaches SwapItem -> EquipItem -> _ApplyItemMods"
    );
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![
            ServerOpcodes::UpdateObject,
            ServerOpcodes::UseEquipmentSetResult
        ]
    );
}

#[tokio::test]
async fn use_equipment_set_empty_slot_unequips_to_backpack_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(1);
    let item_guid = ObjectGuid::create_item(1, 56);
    session.insert_inventory_item_like_cpp(
        1,
        InventoryItem {
            guid: item_guid,
            entry_id: 101,
            db_guid: 56,
            inventory_type: Some(InventoryType::Neck as u8),
        },
    );

    session
        .handle_use_equipment_set(use_equipment_set_packet(
            0x0102_0304_0506_0709,
            [ObjectGuid::EMPTY; wow_packet::packets::misc::EQUIPMENT_SET_SLOTS_LIKE_CPP],
        ))
        .await;

    assert_eq!(
        read_use_equipment_set_result(send_rx.try_recv().unwrap()),
        (0x0102_0304_0506_0709, 0)
    );
    assert!(
        session
            .get_inventory_item_by_pos(INVENTORY_SLOT_BAG_0, 1)
            .is_none()
    );
    assert_eq!(
        session
            .get_inventory_item_by_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START)
            .unwrap()
            .guid,
        item_guid
    );
}

#[tokio::test]
async fn use_equipment_set_ignored_guid_preserves_slot_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(1);
    let item_guid = ObjectGuid::create_item(1, 57);
    session.insert_inventory_item_like_cpp(
        2,
        InventoryItem {
            guid: item_guid,
            entry_id: 102,
            db_guid: 57,
            inventory_type: Some(InventoryType::Shoulders as u8),
        },
    );
    let mut items = [ObjectGuid::EMPTY; wow_packet::packets::misc::EQUIPMENT_SET_SLOTS_LIKE_CPP];
    items[2] = ObjectGuid::new(0x0C00_0400_0000_0000_i64, -1_i64);

    session
        .handle_use_equipment_set(use_equipment_set_packet(0x0102, items))
        .await;

    assert_eq!(
        read_use_equipment_set_result(send_rx.try_recv().unwrap()),
        (0x0102, 0)
    );
    assert_eq!(
        session
            .get_inventory_item_by_pos(INVENTORY_SLOT_BAG_0, 2)
            .unwrap()
            .guid,
        item_guid
    );
    assert!(
        session
            .get_inventory_item_by_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START)
            .is_none()
    );
}

#[tokio::test]
async fn use_equipment_set_skips_non_weapon_slots_in_combat_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(1);
    session.in_combat = true;
    let head_guid = ObjectGuid::create_item(1, 58);
    let mainhand_guid = ObjectGuid::create_item(1, 59);
    session.insert_inventory_item_like_cpp(
        INVENTORY_SLOT_ITEM_START,
        InventoryItem {
            guid: head_guid,
            entry_id: 103,
            db_guid: 58,
            inventory_type: Some(InventoryType::Head as u8),
        },
    );
    session.insert_inventory_item_like_cpp(
        INVENTORY_SLOT_ITEM_START + 1,
        InventoryItem {
            guid: mainhand_guid,
            entry_id: 104,
            db_guid: 59,
            inventory_type: Some(InventoryType::Weapon as u8),
        },
    );
    let mut items = [ObjectGuid::EMPTY; wow_packet::packets::misc::EQUIPMENT_SET_SLOTS_LIKE_CPP];
    items[0] = head_guid;
    items[EQUIPMENT_SLOT_MAINHAND as usize] = mainhand_guid;

    session
        .handle_use_equipment_set(use_equipment_set_packet(0x0103, items))
        .await;

    assert_eq!(
        read_use_equipment_set_result(send_rx.try_recv().unwrap()),
        (0x0103, 0)
    );
    assert!(
        session
            .get_inventory_item_by_pos(INVENTORY_SLOT_BAG_0, 0)
            .is_none()
    );
    assert_eq!(
        session
            .get_inventory_item_by_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START)
            .unwrap()
            .guid,
        head_guid
    );
    assert_eq!(
        session
            .get_inventory_item_by_pos(INVENTORY_SLOT_BAG_0, EQUIPMENT_SLOT_MAINHAND)
            .unwrap()
            .guid,
        mainhand_guid
    );
}

#[test]
fn direct_inventory_swap_persistence_plan_replaces_both_occupied_positions_like_cpp() {
    let src = InventoryItem {
        guid: ObjectGuid::create_item(1, 58),
        entry_id: 103,
        db_guid: 58,
        inventory_type: None,
    };
    let dst = InventoryItem {
        guid: ObjectGuid::create_item(1, 59),
        entry_id: 104,
        db_guid: 59,
        inventory_type: None,
    };

    assert_eq!(
        plan_direct_inventory_swap_persistence_like_cpp(35, 36, Some(&src), Some(&dst)),
        vec![
            DirectInventoryPositionUpdateLikeCpp {
                slot: 36,
                item_db_guid: 58,
            },
            DirectInventoryPositionUpdateLikeCpp {
                slot: 35,
                item_db_guid: 59,
            },
        ],
        "C++ _SaveInventory replaces each changed item's final position in one transaction"
    );
}

#[test]
fn direct_inventory_swap_persistence_plan_moves_into_empty_position_like_cpp() {
    let src = InventoryItem {
        guid: ObjectGuid::create_item(1, 60),
        entry_id: 105,
        db_guid: 60,
        inventory_type: None,
    };

    assert_eq!(
        plan_direct_inventory_swap_persistence_like_cpp(35, 36, Some(&src), None),
        vec![DirectInventoryPositionUpdateLikeCpp {
            slot: 36,
            item_db_guid: 60,
        }]
    );
}

#[tokio::test]
async fn swap_inv_item_empty_source_returns_without_moving_destination_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(4);
    session.set_player_guid(Some(ObjectGuid::create_player(1, 42)));
    let destination_guid = ObjectGuid::create_item(1, 62);
    session.insert_inventory_item_like_cpp(
        INVENTORY_SLOT_ITEM_START + 1,
        InventoryItem {
            guid: destination_guid,
            entry_id: 107,
            db_guid: 62,
            inventory_type: None,
        },
    );

    session
        .handle_swap_inv_item(SwapInvItem {
            inv_update: InvUpdate {
                items: vec![
                    (INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START),
                    (INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START + 1),
                ],
            },
            src_slot: INVENTORY_SLOT_ITEM_START,
            dst_slot: INVENTORY_SLOT_ITEM_START + 1,
        })
        .await;

    assert!(
        session
            .get_inventory_item_by_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START)
            .is_none()
    );
    assert_eq!(
        session
            .get_inventory_item_by_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START + 1,)
            .map(|item| item.guid),
        Some(destination_guid)
    );
    assert!(
        send_rx.try_recv().is_err(),
        "C++ Player::SwapItem silently returns when the source is empty"
    );
}

#[tokio::test]
async fn swap_inv_item_commit_failure_keeps_runtime_unchanged_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(4);
    session.set_player_guid(Some(ObjectGuid::create_player(1, 42)));
    install_bank_move_item_fixture(&mut session, 106, 1);
    let item_guid = insert_bank_move_test_item(&mut session, INVENTORY_SLOT_ITEM_START, 106, 61, 1);
    session.set_player_inventory_persistence_port_like_cpp(
        PlayerInventoryPersistencePortFixtureLikeCpp::failed(),
    );

    session
        .handle_swap_inv_item(SwapInvItem {
            inv_update: InvUpdate {
                items: vec![
                    (INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START),
                    (INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START + 1),
                ],
            },
            src_slot: INVENTORY_SLOT_ITEM_START,
            dst_slot: INVENTORY_SLOT_ITEM_START + 1,
        })
        .await;

    assert_eq!(
        session
            .get_inventory_item_by_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START)
            .map(|item| item.guid),
        Some(item_guid)
    );
    assert!(
        session
            .get_inventory_item_by_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START + 1)
            .is_none()
    );
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::InventoryChangeFailure]
    );
}

#[tokio::test]
async fn auto_equip_item_slot_without_persistence_keeps_runtime_unchanged_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(4);
    session.set_player_guid(Some(ObjectGuid::create_player(1, 42)));
    install_equippable_item_fixture(&mut session, 105, InventoryType::Weapon, None);
    let item_guid = insert_equippable_test_item(
        &mut session,
        INVENTORY_SLOT_BAG_0,
        INVENTORY_SLOT_ITEM_START,
        105,
        60,
        InventoryType::Weapon,
    );

    session
        .handle_auto_equip_item_slot(AutoEquipItemSlot {
            inv_update: InvUpdate {
                items: vec![(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START)],
            },
            item: item_guid,
            item_dst_slot: EQUIPMENT_SLOT_MAINHAND,
        })
        .await;

    assert_eq!(
        session
            .get_inventory_item_by_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START)
            .unwrap()
            .guid,
        item_guid
    );
    assert!(
        session
            .get_inventory_item_by_pos(INVENTORY_SLOT_BAG_0, EQUIPMENT_SLOT_MAINHAND)
            .is_none()
    );
    let error = send_rx
        .try_recv()
        .expect("missing persistence should report an inventory failure");
    assert_eq!(
        u16::from_le_bytes([error[0], error[1]]),
        ServerOpcodes::InventoryChangeFailure as u16,
    );
}

#[tokio::test]
async fn auto_equip_item_slot_commit_failure_does_not_apply_item_mods_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(4);
    let player_guid = ObjectGuid::create_player(1, 42);
    let entry_id = 109;
    session.set_player_guid(Some(player_guid));
    install_equippable_item_fixture(&mut session, entry_id, InventoryType::Weapon, Some(7));
    session.set_item_set_store(Arc::new(wow_data::ItemSetStore::from_entries([
        wow_data::ItemSetEntry {
            id: 706,
            name: "Auto Equip Set".to_string(),
            set_flags: 0,
            required_skill: 0,
            required_skill_rank: 0,
            item_id: std::array::from_fn(|i| if i == 0 { entry_id } else { 0 }),
        },
    ])));
    session.set_item_set_spell_store(Arc::new(wow_data::ItemSetSpellStore::from_entries([
        wow_data::ItemSetSpellEntry {
            id: 22,
            chr_spec_id: 0,
            spell_id: 9022,
            threshold: 1,
            item_set_id: 706,
        },
    ])));
    let item_guid = insert_equippable_test_item(
        &mut session,
        INVENTORY_SLOT_BAG_0,
        INVENTORY_SLOT_ITEM_START,
        entry_id,
        64,
        InventoryType::Weapon,
    );
    session.set_player_inventory_persistence_port_like_cpp(
        PlayerInventoryPersistencePortFixtureLikeCpp::failed(),
    );

    session
        .handle_auto_equip_item_slot(AutoEquipItemSlot {
            inv_update: InvUpdate {
                items: vec![(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START)],
            },
            item: item_guid,
            item_dst_slot: EQUIPMENT_SLOT_MAINHAND,
        })
        .await;

    assert_eq!(
        session.represented_item_bonus_state_like_cpp().stats_base[0],
        0,
        "item mods must remain unchanged until the inventory transaction commits"
    );
    assert!(
        session
            .represented_item_set_spell_events_like_cpp()
            .is_empty(),
        "set-bonus events must not be exposed after a failed commit"
    );
    assert_eq!(
        session
            .get_inventory_item_by_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START)
            .map(|item| item.guid),
        Some(item_guid)
    );
    assert!(
        session
            .get_inventory_item_by_pos(INVENTORY_SLOT_BAG_0, EQUIPMENT_SLOT_MAINHAND)
            .is_none()
    );
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::InventoryChangeFailure],
        "failed persistence emits only the inventory failure"
    );
}

#[test]
fn direct_inventory_move_from_equipment_removes_represented_item_mods_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send_capacity(1);
    let player_guid = ObjectGuid::create_player(1, 42);
    let item_guid = ObjectGuid::create_item(1, 65);
    let entry_id = 110;
    session.set_player_guid(Some(player_guid));
    session.set_item_stats_store(Arc::new(strength_item_stats_store(entry_id, 9)));
    session.insert_inventory_item_like_cpp(
        INVENTORY_SLOT_ITEM_START,
        InventoryItem {
            guid: item_guid,
            entry_id,
            db_guid: 65,
            inventory_type: Some(InventoryType::Weapon as u8),
        },
    );
    let item = session.make_inventory_item_object(
        item_guid,
        entry_id,
        player_guid,
        1,
        0,
        ItemContext::None,
        INVENTORY_SLOT_ITEM_START,
    );
    session.insert_inventory_item_object(item);
    assert_eq!(
        session.move_represented_direct_inventory_item_with_item_mods_like_cpp(
            INVENTORY_SLOT_ITEM_START,
            EQUIPMENT_SLOT_MAINHAND
        ),
        Some(true)
    );
    assert_eq!(
        session.represented_item_bonus_state_like_cpp().stats_base[0],
        9
    );
    assert_eq!(
        session.move_represented_direct_inventory_item_with_item_mods_like_cpp(
            EQUIPMENT_SLOT_MAINHAND,
            INVENTORY_SLOT_ITEM_START
        ),
        Some(true)
    );
    assert_eq!(
        session.represented_item_bonus_state_like_cpp().stats_base[0],
        0,
        "C++ RemoveItem calls _ApplyItemMods(..., false) before taking equipped items out of storage"
    );
}

#[tokio::test]
async fn auto_equip_item_slot_rejects_bad_inv_count_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send_capacity(1);
    session.set_player_guid(Some(ObjectGuid::create_player(1, 42)));
    let item_guid = ObjectGuid::create_item(1, 61);
    session.insert_inventory_item_like_cpp(
        INVENTORY_SLOT_ITEM_START,
        InventoryItem {
            guid: item_guid,
            entry_id: 106,
            db_guid: 61,
            inventory_type: Some(InventoryType::Weapon as u8),
        },
    );

    session
        .handle_auto_equip_item_slot(AutoEquipItemSlot {
            inv_update: InvUpdate { items: Vec::new() },
            item: item_guid,
            item_dst_slot: EQUIPMENT_SLOT_MAINHAND,
        })
        .await;

    assert!(
        session
            .get_inventory_item_by_pos(INVENTORY_SLOT_BAG_0, EQUIPMENT_SLOT_MAINHAND)
            .is_none()
    );
    assert_eq!(
        session
            .get_inventory_item_by_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START)
            .unwrap()
            .guid,
        item_guid
    );
}

#[tokio::test]
async fn auto_equip_item_slot_rejects_source_position_mismatch_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send_capacity(1);
    session.set_player_guid(Some(ObjectGuid::create_player(1, 42)));
    let item_guid = ObjectGuid::create_item(1, 62);
    session.insert_inventory_item_like_cpp(
        INVENTORY_SLOT_ITEM_START,
        InventoryItem {
            guid: item_guid,
            entry_id: 107,
            db_guid: 62,
            inventory_type: Some(InventoryType::Weapon as u8),
        },
    );

    session
        .handle_auto_equip_item_slot(AutoEquipItemSlot {
            inv_update: InvUpdate {
                items: vec![(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START + 1)],
            },
            item: item_guid,
            item_dst_slot: EQUIPMENT_SLOT_MAINHAND,
        })
        .await;

    assert!(
        session
            .get_inventory_item_by_pos(INVENTORY_SLOT_BAG_0, EQUIPMENT_SLOT_MAINHAND)
            .is_none()
    );
    assert_eq!(
        session
            .get_inventory_item_by_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START)
            .unwrap()
            .guid,
        item_guid
    );
}

#[tokio::test]
async fn auto_equip_item_slot_rejects_non_equipment_destination_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send_capacity(1);
    session.set_player_guid(Some(ObjectGuid::create_player(1, 42)));
    let item_guid = ObjectGuid::create_item(1, 63);
    session.insert_inventory_item_like_cpp(
        INVENTORY_SLOT_ITEM_START,
        InventoryItem {
            guid: item_guid,
            entry_id: 108,
            db_guid: 63,
            inventory_type: Some(InventoryType::Weapon as u8),
        },
    );

    session
        .handle_auto_equip_item_slot(AutoEquipItemSlot {
            inv_update: InvUpdate {
                items: vec![(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START)],
            },
            item: item_guid,
            item_dst_slot: INVENTORY_SLOT_ITEM_START + 1,
        })
        .await;

    assert_eq!(
        session
            .get_inventory_item_by_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START)
            .unwrap()
            .guid,
        item_guid
    );
    assert!(
        session
            .get_inventory_item_by_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START + 1)
            .is_none()
    );
}

#[tokio::test]
async fn swap_inv_item_rejects_bad_inv_update_count_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(1);
    session.set_player_guid(Some(ObjectGuid::create_player(1, 42)));

    session
        .handle_swap_inv_item(SwapInvItem {
            inv_update: InvUpdate {
                items: vec![(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START)],
            },
            src_slot: 250,
            dst_slot: 251,
        })
        .await;

    assert!(
        send_rx.try_recv().is_err(),
        "C++ returns on invalid InvUpdate count before slot validation or equip errors"
    );
}

#[tokio::test]
async fn swap_item_rejects_bad_inv_update_count_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(1);
    session.set_player_guid(Some(ObjectGuid::create_player(1, 42)));

    session
        .handle_swap_item(SwapItem {
            inv_update: InvUpdate { items: Vec::new() },
            container_slot_a: INVENTORY_SLOT_BAG_START,
            container_slot_b: INVENTORY_SLOT_BAG_START,
            slot_a: 0,
            slot_b: 1,
        })
        .await;

    assert!(
        send_rx.try_recv().is_err(),
        "C++ returns on invalid InvUpdate count before container validation"
    );
}

#[tokio::test]
async fn swap_item_validates_source_and_destination_positions_like_cpp() {
    let (mut session, instance_rx, realm_rx) = make_session_with_realm_send_capacity(2);
    session.set_player_guid(Some(ObjectGuid::create_player(1, 42)));

    session
        .handle_swap_item(SwapItem {
            inv_update: InvUpdate {
                items: vec![(200, 0), (INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START)],
            },
            container_slot_a: 200,
            container_slot_b: INVENTORY_SLOT_BAG_0,
            slot_a: 0,
            slot_b: INVENTORY_SLOT_ITEM_START,
        })
        .await;
    assert_eq!(
        inventory_failure_result(&realm_rx.try_recv().expect("invalid source realm error")),
        InventoryResult::ItemNotFound as i32
    );
    assert!(instance_rx.try_recv().is_err());

    session
        .handle_swap_item(SwapItem {
            inv_update: InvUpdate {
                items: vec![(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START), (200, 0)],
            },
            container_slot_a: INVENTORY_SLOT_BAG_0,
            container_slot_b: 200,
            slot_a: INVENTORY_SLOT_ITEM_START,
            slot_b: 0,
        })
        .await;
    assert_eq!(
        inventory_failure_result(
            &realm_rx
                .try_recv()
                .expect("invalid destination realm error")
        ),
        InventoryResult::WrongSlot as i32
    );
    assert!(instance_rx.try_recv().is_err());
}

#[tokio::test]
async fn swap_inv_item_rejects_bank_positions_without_bank_access_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(1);
    session.set_player_guid(Some(ObjectGuid::create_player(1, 42)));
    install_bank_move_item_fixture(&mut session, 120, 1);
    let source_guid =
        insert_bank_move_test_item(&mut session, INVENTORY_SLOT_ITEM_START, 120, 70, 1);

    session
        .handle_swap_inv_item(SwapInvItem {
            inv_update: InvUpdate {
                items: vec![
                    (INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START),
                    (INVENTORY_SLOT_BAG_0, wow_entities::BANK_SLOT_ITEM_START),
                ],
            },
            src_slot: INVENTORY_SLOT_ITEM_START,
            dst_slot: wow_entities::BANK_SLOT_ITEM_START,
        })
        .await;

    assert_eq!(
        session
            .get_inventory_item_by_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START)
            .map(|item| item.guid),
        Some(source_guid)
    );
    assert!(
        send_rx.try_recv().is_err(),
        "C++ silently rejects a bank swap when WorldSession::CanUseBank fails"
    );
}

#[test]
fn equip_destination_uses_visualize_binding_rule_like_cpp() {
    let mut equipped = wow_entities::Item::default();
    equipped.set_bonding(ItemBondingType::OnEquip);
    let unequipped = equipped.clone();
    bind_inventory_item_for_destination_like_cpp(
        &mut equipped,
        wow_entities::make_item_pos(INVENTORY_SLOT_BAG_0, EQUIPMENT_SLOT_MAINHAND),
    );
    assert!(
        equipped.is_soul_bound(),
        "C++ Player::VisualizeItem binds BIND_ON_EQUIP before EquipItem persistence"
    );
    assert!(
        item_dynamic_flags_changed_like_cpp(&unequipped, &equipped),
        "the equip path must publish ITEM_DATA_DYNAMIC_FLAGS after applying binding"
    );

    let mut backpack = wow_entities::Item::default();
    backpack.set_bonding(ItemBondingType::OnEquip);
    bind_inventory_item_for_destination_like_cpp(
        &mut backpack,
        wow_entities::make_item_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START),
    );
    assert!(
        !backpack.is_soul_bound(),
        "ordinary C++ _StoreItem destinations keep BIND_ON_EQUIP unbound"
    );

    let mut equipped_bag = wow_entities::Item::default();
    equipped_bag.set_bonding(ItemBondingType::OnEquip);
    bind_inventory_item_for_destination_like_cpp(
        &mut equipped_bag,
        wow_entities::make_item_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_BAG_START),
    );
    assert!(equipped_bag.is_soul_bound());
}

#[test]
fn autostore_bank_target_depends_on_source_domain_like_cpp() {
    assert_eq!(
        autostore_bank_target_like_cpp(INVENTORY_SLOT_BAG_0, wow_entities::BANK_SLOT_ITEM_START,),
        InventoryStorageTargetLikeCpp::Inventory
    );
    assert_eq!(
        autostore_bank_target_like_cpp(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START,),
        InventoryStorageTargetLikeCpp::Bank
    );
}

#[test]
fn bag_exchange_child_updates_runtime_container_and_wire_field_like_cpp() {
    let player_guid = ObjectGuid::create_player(1, 42);
    let old_bag_guid = ObjectGuid::create_item(1, 80);
    let new_bag_guid = ObjectGuid::create_item(1, 81);
    let mut child = wow_entities::Item::new(900);
    child.set_container_guid_and_slot(old_bag_guid, INVENTORY_SLOT_BAG_START);
    child.set_contained_in(old_bag_guid);
    child.set_slot(7);

    relocate_bag_exchange_child_like_cpp(&mut child, new_bag_guid, 2);

    assert_eq!(child.container_guid(), new_bag_guid);
    assert_eq!(child.data().contained_in, new_bag_guid);
    assert_eq!(child.slot(), 2);
    assert_ne!(child.data().contained_in, player_guid);
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

#[test]
fn child_equip_plan_targets_db2_slot_before_parent_moves_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send_capacity(1);
    let player_guid = ObjectGuid::create_player(1, 42);
    let parent_entry = 123;
    let child_entry = parent_entry;
    session.set_player_guid(Some(player_guid));
    install_equippable_item_fixture(&mut session, parent_entry, InventoryType::Weapon, None);
    install_child_equipment_fixture(
        &mut session,
        parent_entry,
        child_entry,
        wow_entities::EQUIPMENT_SLOT_OFFHAND,
    );
    let parent_guid = insert_equippable_test_item(
        &mut session,
        INVENTORY_SLOT_BAG_0,
        INVENTORY_SLOT_ITEM_START,
        parent_entry,
        77,
        InventoryType::Weapon,
    );
    let child_guid = insert_equippable_test_item(
        &mut session,
        INVENTORY_SLOT_BAG_0,
        CHILD_EQUIPMENT_SLOT_START,
        child_entry,
        78,
        InventoryType::Weapon,
    );
    session.update_inventory_item_object_like_cpp(child_guid, |child| {
        child.set_item_flag(ItemFieldFlags::CHILD);
        child.set_creator(parent_guid);
    });

    assert_eq!(
        session
            .plan_inventory_equip_child_like_cpp(
                INVENTORY_SLOT_BAG_0,
                INVENTORY_SLOT_ITEM_START,
                parent_guid,
            )
            .expect("child equip preflight"),
        Some(InventoryEquipChildPlanLikeCpp {
            child_guid,
            destination_slot: wow_entities::EQUIPMENT_SLOT_OFFHAND,
            displaced_storage: None,
        })
    );
}

#[test]
fn real_swap_plans_child_for_item_entering_source_equipment_slot_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send_capacity(1);
    let player_guid = ObjectGuid::create_player(1, 42);
    let entry = 124;
    session.set_player_guid(Some(player_guid));
    install_equippable_item_fixture(&mut session, entry, InventoryType::Weapon, None);
    install_child_equipment_fixture(
        &mut session,
        entry,
        entry,
        wow_entities::EQUIPMENT_SLOT_OFFHAND,
    );
    let source_guid = insert_equippable_test_item(
        &mut session,
        INVENTORY_SLOT_BAG_0,
        EQUIPMENT_SLOT_MAINHAND,
        entry,
        85,
        InventoryType::Weapon,
    );
    let destination_parent_guid = insert_equippable_test_item(
        &mut session,
        INVENTORY_SLOT_BAG_0,
        INVENTORY_SLOT_ITEM_START,
        entry,
        86,
        InventoryType::Weapon,
    );
    let child_guid = insert_equippable_test_item(
        &mut session,
        INVENTORY_SLOT_BAG_0,
        CHILD_EQUIPMENT_SLOT_START,
        entry,
        87,
        InventoryType::Weapon,
    );
    session.update_inventory_item_object_like_cpp(child_guid, |child| {
        child.set_item_flag(ItemFieldFlags::CHILD);
        child.set_creator(destination_parent_guid);
    });

    let plans = session
        .plan_inventory_real_swap_children_like_cpp(
            INVENTORY_SLOT_BAG_0,
            EQUIPMENT_SLOT_MAINHAND,
            source_guid,
            INVENTORY_SLOT_BAG_0,
            INVENTORY_SLOT_ITEM_START,
            destination_parent_guid,
        )
        .expect("reverse-direction child preflight");

    assert_eq!(
        plans,
        vec![InventoryEquipChildPlanLikeCpp {
            child_guid,
            destination_slot: wow_entities::EQUIPMENT_SLOT_OFFHAND,
            displaced_storage: None,
        }]
    );
}

#[test]
fn child_equip_plan_preflights_displaced_equipment_storage_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send_capacity(1);
    let player_guid = ObjectGuid::create_player(1, 42);
    let parent_entry = 125;
    let child_entry = parent_entry;
    let displaced_entry = parent_entry;
    session.set_player_guid(Some(player_guid));
    install_equippable_item_fixture(&mut session, parent_entry, InventoryType::Weapon, None);
    install_child_equipment_fixture(
        &mut session,
        parent_entry,
        child_entry,
        wow_entities::EQUIPMENT_SLOT_OFFHAND,
    );
    let parent_guid = insert_equippable_test_item(
        &mut session,
        INVENTORY_SLOT_BAG_0,
        INVENTORY_SLOT_ITEM_START,
        parent_entry,
        79,
        InventoryType::Weapon,
    );
    let child_guid = insert_equippable_test_item(
        &mut session,
        INVENTORY_SLOT_BAG_0,
        CHILD_EQUIPMENT_SLOT_START,
        child_entry,
        80,
        InventoryType::Weapon,
    );
    session.update_inventory_item_object_like_cpp(child_guid, |child| {
        child.set_item_flag(ItemFieldFlags::CHILD);
        child.set_creator(parent_guid);
    });
    insert_equippable_test_item(
        &mut session,
        INVENTORY_SLOT_BAG_0,
        wow_entities::EQUIPMENT_SLOT_OFFHAND,
        displaced_entry,
        81,
        InventoryType::Weapon,
    );

    let plan = session
        .plan_inventory_equip_child_like_cpp(
            INVENTORY_SLOT_BAG_0,
            INVENTORY_SLOT_ITEM_START,
            parent_guid,
        )
        .expect("child equip preflight")
        .expect("linked child");
    assert_eq!(
        plan.displaced_storage,
        Some((
            INVENTORY_SLOT_BAG_0,
            NULL_SLOT,
            InventoryStorageTargetLikeCpp::Inventory,
        ))
    );
    assert_eq!(
        session
            .get_inventory_item_by_pos(INVENTORY_SLOT_BAG_0, wow_entities::EQUIPMENT_SLOT_OFFHAND,)
            .map(|item| item.entry_id),
        Some(displaced_entry),
        "CanEquipChildItem must not mutate the destination during preflight"
    );
}

#[test]
fn child_equip_plan_rejects_displacement_when_parent_started_equipped_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send_capacity(1);
    let player_guid = ObjectGuid::create_player(1, 42);
    let entry = 128;
    session.set_player_guid(Some(player_guid));
    install_equippable_item_fixture(&mut session, entry, InventoryType::Weapon, None);
    install_child_equipment_fixture(
        &mut session,
        entry,
        entry,
        wow_entities::EQUIPMENT_SLOT_OFFHAND,
    );
    let parent_guid = insert_equippable_test_item(
        &mut session,
        INVENTORY_SLOT_BAG_0,
        EQUIPMENT_SLOT_MAINHAND,
        entry,
        82,
        InventoryType::Weapon,
    );
    let child_guid = insert_equippable_test_item(
        &mut session,
        INVENTORY_SLOT_BAG_0,
        CHILD_EQUIPMENT_SLOT_START,
        entry,
        83,
        InventoryType::Weapon,
    );
    session.update_inventory_item_object_like_cpp(child_guid, |child| {
        child.set_item_flag(ItemFieldFlags::CHILD);
        child.set_creator(parent_guid);
    });
    insert_equippable_test_item(
        &mut session,
        INVENTORY_SLOT_BAG_0,
        wow_entities::EQUIPMENT_SLOT_OFFHAND,
        entry,
        84,
        InventoryType::Weapon,
    );

    assert_eq!(
        session.plan_inventory_equip_child_like_cpp(
            INVENTORY_SLOT_BAG_0,
            EQUIPMENT_SLOT_MAINHAND,
            parent_guid,
        ),
        Err(InventoryResult::CantSwap)
    );
}

#[test]
fn child_redirect_validates_both_steps_without_mutating_runtime_like_upstream_cpp() {
    let (mut session, _send_rx) = make_session_with_send_capacity(1);
    let player_guid = ObjectGuid::create_player(1, 42);
    let entry_id = 121;
    session.set_player_guid(Some(player_guid));
    install_equippable_item_fixture(&mut session, entry_id, InventoryType::Weapon, None);

    let source_guid = insert_equippable_test_item(
        &mut session,
        INVENTORY_SLOT_BAG_0,
        INVENTORY_SLOT_ITEM_START,
        entry_id,
        71,
        InventoryType::Weapon,
    );
    let parent_guid = insert_equippable_test_item(
        &mut session,
        INVENTORY_SLOT_BAG_0,
        wow_entities::EQUIPMENT_SLOT_OFFHAND,
        entry_id,
        72,
        InventoryType::Weapon,
    );
    let child_guid = insert_equippable_test_item(
        &mut session,
        INVENTORY_SLOT_BAG_0,
        EQUIPMENT_SLOT_MAINHAND,
        entry_id,
        73,
        InventoryType::Weapon,
    );
    session.update_inventory_item_object_like_cpp(child_guid, |child| {
        child.set_item_flag(ItemFieldFlags::CHILD);
        child.set_creator(parent_guid);
    });

    let source = wow_entities::make_item_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START);
    let destination = wow_entities::make_item_pos(INVENTORY_SLOT_BAG_0, EQUIPMENT_SLOT_MAINHAND);
    let redirect = session
        .plan_inventory_swap_preflight_like_cpp(source, destination)
        .expect("player inventory preflight");
    let SwapItemPreflightResult::ChildRedirect {
        first_src,
        first_dst,
        second_src,
        second_dst,
    } = redirect.result
    else {
        panic!("equipped child destination must redirect through its parent");
    };

    assert_eq!(
        session
            .plan_inventory_child_redirect_like_cpp(
                INVENTORY_SLOT_BAG_0,
                EQUIPMENT_SLOT_MAINHAND,
                first_src,
                first_dst,
                second_src,
                second_dst,
            )
            .expect("both redirected moves are legal"),
        CHILD_EQUIPMENT_SLOT_START
    );
    assert_eq!(
        session
            .get_inventory_item_by_pos(INVENTORY_SLOT_BAG_0, EQUIPMENT_SLOT_MAINHAND)
            .map(|item| item.guid),
        Some(child_guid),
        "the validation overlay must restore the visible child slot"
    );
    assert_eq!(
        session
            .get_inventory_item_by_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START)
            .map(|item| item.guid),
        Some(source_guid)
    );
    assert_eq!(
        session
            .get_inventory_item_by_pos(INVENTORY_SLOT_BAG_0, wow_entities::EQUIPMENT_SLOT_OFFHAND,)
            .map(|item| item.guid),
        Some(parent_guid)
    );
    assert!(
        session
            .get_inventory_item_by_pos(INVENTORY_SLOT_BAG_0, CHILD_EQUIPMENT_SLOT_START)
            .is_none()
    );
}

#[test]
fn rejected_child_redirect_restores_validation_overlay_before_error_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send_capacity(1);
    let player_guid = ObjectGuid::create_player(1, 42);
    let entry_id = 122;
    session.set_player_guid(Some(player_guid));
    session.set_player_alive_like_cpp(false);
    install_equippable_item_fixture(&mut session, entry_id, InventoryType::Weapon, None);

    let source_guid = insert_equippable_test_item(
        &mut session,
        INVENTORY_SLOT_BAG_0,
        INVENTORY_SLOT_ITEM_START,
        entry_id,
        74,
        InventoryType::Weapon,
    );
    let parent_guid = insert_equippable_test_item(
        &mut session,
        INVENTORY_SLOT_BAG_0,
        wow_entities::EQUIPMENT_SLOT_OFFHAND,
        entry_id,
        75,
        InventoryType::Weapon,
    );
    let child_guid = insert_equippable_test_item(
        &mut session,
        INVENTORY_SLOT_BAG_0,
        EQUIPMENT_SLOT_MAINHAND,
        entry_id,
        76,
        InventoryType::Weapon,
    );
    session.update_inventory_item_object_like_cpp(child_guid, |child| {
        child.set_item_flag(ItemFieldFlags::CHILD);
        child.set_creator(parent_guid);
    });

    let source = wow_entities::make_item_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START);
    let destination = wow_entities::make_item_pos(INVENTORY_SLOT_BAG_0, EQUIPMENT_SLOT_MAINHAND);
    let redirect = session
        .plan_inventory_swap_preflight_like_cpp(source, destination)
        .expect("player inventory preflight");
    let SwapItemPreflightResult::ChildRedirect {
        first_src,
        first_dst,
        second_src,
        second_dst,
    } = redirect.result
    else {
        panic!("C++ examines child redirects before the player-dead gate");
    };

    assert_eq!(
        session.plan_inventory_child_redirect_like_cpp(
            INVENTORY_SLOT_BAG_0,
            EQUIPMENT_SLOT_MAINHAND,
            first_src,
            first_dst,
            second_src,
            second_dst,
        ),
        Err(InventoryResult::PlayerDead)
    );
    assert_eq!(
        session
            .get_inventory_item_by_pos(INVENTORY_SLOT_BAG_0, EQUIPMENT_SLOT_MAINHAND)
            .map(|item| item.guid),
        Some(child_guid),
        "a rejected redirected move must not persist the child in a hidden slot"
    );
    assert_eq!(
        session
            .get_inventory_item_by_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START)
            .map(|item| item.guid),
        Some(source_guid)
    );
    assert_eq!(
        session
            .get_inventory_item_by_pos(INVENTORY_SLOT_BAG_0, wow_entities::EQUIPMENT_SLOT_OFFHAND,)
            .map(|item| item.guid),
        Some(parent_guid)
    );
    assert!(
        session
            .get_inventory_item_by_pos(INVENTORY_SLOT_BAG_0, CHILD_EQUIPMENT_SLOT_START)
            .is_none()
    );
}

#[test]
fn committed_swap_updates_top_level_and_nested_container_positions_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send_capacity(1);
    let player_guid = ObjectGuid::create_player(1, 42);
    let bag_guid = ObjectGuid::create_item(1, 80);
    let nested_guid = ObjectGuid::create_item(1, 81);
    let backpack_guid = ObjectGuid::create_item(1, 82);
    session.set_player_guid(Some(player_guid));
    session.insert_inventory_item_like_cpp(
        INVENTORY_SLOT_BAG_START,
        InventoryItem {
            guid: bag_guid,
            entry_id: 600,
            db_guid: 80,
            inventory_type: Some(InventoryType::Bag as u8),
        },
    );
    session.insert_inventory_item_like_cpp(
        INVENTORY_SLOT_ITEM_START,
        InventoryItem {
            guid: backpack_guid,
            entry_id: 701,
            db_guid: 82,
            inventory_type: Some(InventoryType::NonEquip as u8),
        },
    );
    let bag = session.make_inventory_item_object(
        bag_guid,
        600,
        player_guid,
        1,
        0,
        ItemContext::None,
        INVENTORY_SLOT_BAG_START,
    );
    session.insert_inventory_item_object(bag);
    let mut nested = session.make_inventory_item_object(
        nested_guid,
        700,
        player_guid,
        1,
        0,
        ItemContext::None,
        0,
    );
    nested.set_container_guid_and_slot(bag_guid, INVENTORY_SLOT_BAG_START);
    session.insert_inventory_item_object(nested);
    let backpack = session.make_inventory_item_object(
        backpack_guid,
        701,
        player_guid,
        1,
        0,
        ItemContext::None,
        INVENTORY_SLOT_ITEM_START,
    );
    session.insert_inventory_item_object(backpack);

    assert!(session.apply_committed_inventory_item_swap_like_cpp(
        INVENTORY_SLOT_BAG_START,
        0,
        INVENTORY_SLOT_BAG_0,
        INVENTORY_SLOT_ITEM_START,
    ));
    assert_eq!(
        session
            .get_inventory_item_by_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START)
            .map(|item| item.guid),
        Some(nested_guid)
    );
    assert_eq!(
        session
            .get_inventory_item_by_pos(INVENTORY_SLOT_BAG_START, 0)
            .map(|item| item.guid),
        Some(backpack_guid)
    );
}

#[test]
fn recursive_destroy_descendants_are_deepest_first_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send_capacity(1);
    let player_guid = ObjectGuid::create_player(1, 42);
    let parent_guid = ObjectGuid::create_item(1, 90);
    let child_bag_guid = ObjectGuid::create_item(1, 91);
    let leaf_guid = ObjectGuid::create_item(1, 92);
    session.set_player_guid(Some(player_guid));

    let parent = session.make_inventory_item_object(
        parent_guid,
        600,
        player_guid,
        1,
        0,
        ItemContext::None,
        INVENTORY_SLOT_ITEM_START,
    );
    session.insert_inventory_item_object(parent);
    let mut child_bag = session.make_inventory_item_object(
        child_bag_guid,
        601,
        player_guid,
        1,
        0,
        ItemContext::None,
        0,
    );
    child_bag.set_container_guid_and_slot(parent_guid, INVENTORY_SLOT_ITEM_START);
    session.insert_inventory_item_object(child_bag);
    let mut leaf =
        session.make_inventory_item_object(leaf_guid, 700, player_guid, 1, 0, ItemContext::None, 0);
    leaf.set_container_guid_and_slot(child_bag_guid, 0);
    session.insert_inventory_item_object(leaf);

    let descendants = session.represented_inventory_descendants_postorder_like_cpp(parent_guid);
    assert_eq!(
        descendants
            .iter()
            .map(|(_, _, item)| item.guid)
            .collect::<Vec<_>>(),
        vec![leaf_guid, child_bag_guid]
    );
}

#[test]
fn recursive_destroy_plans_child_and_parent_quest_removal_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send_capacity(1);
    let quest_id = 91_001;
    let child_entry = 700;
    let parent_entry = 600;
    let mut quest = quest_template(quest_id);
    quest.objectives = [child_entry, parent_entry]
        .into_iter()
        .enumerate()
        .map(|(index, entry_id)| QuestObjective {
            id: quest_id * 10 + index as u32,
            quest_id,
            obj_type: 1,
            order: index as u8,
            storage_index: index as i8,
            object_id: entry_id as i32,
            amount: 1,
            flags: 0,
            flags2: 0,
            progress_bar_weight: 0.0,
            description: String::new(),
        })
        .collect();
    session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
    session.player_quests.insert(
        quest_id,
        crate::handlers::quest::PlayerQuestStatus {
            quest_id,
            status: crate::conditions::QUEST_STATUS_COMPLETE_LIKE_CPP,
            explored: false,
            accept_time_secs: 0,
            end_time_secs: 0,
            objective_counts: vec![1, 1],
            slot: 0,
        },
    );

    let planned = session.plan_destroyed_inventory_quest_persistence_like_cpp(&[
        DestroyQuestItemLikeCpp {
            bag: INVENTORY_SLOT_BAG_START,
            slot: 0,
            entry_id: child_entry,
            count: 1,
        },
        DestroyQuestItemLikeCpp {
            bag: INVENTORY_SLOT_BAG_0,
            slot: INVENTORY_SLOT_BAG_START,
            entry_id: parent_entry,
            count: 1,
        },
    ]);

    assert_eq!(planned.len(), 1);
    assert_eq!(planned[0].objective_counts, vec![0, 0]);
    assert_eq!(
        planned[0].status,
        crate::conditions::QUEST_STATUS_INCOMPLETE_LIKE_CPP
    );
}

#[tokio::test]
async fn recursive_destroy_commit_failure_keeps_bag_and_children_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(1);
    let player_guid = ObjectGuid::create_player(1, 42);
    let bag_guid = ObjectGuid::create_item(1, 93);
    let child_guid = ObjectGuid::create_item(1, 94);
    session.set_player_guid(Some(player_guid));
    session.insert_inventory_item_like_cpp(
        INVENTORY_SLOT_ITEM_START,
        InventoryItem {
            guid: bag_guid,
            entry_id: 600,
            db_guid: 93,
            inventory_type: Some(InventoryType::Bag as u8),
        },
    );
    let bag = session.make_inventory_item_object(
        bag_guid,
        600,
        player_guid,
        1,
        0,
        ItemContext::None,
        INVENTORY_SLOT_ITEM_START,
    );
    session.insert_inventory_item_object(bag.clone());
    let mut child = session.make_inventory_item_object(
        child_guid,
        700,
        player_guid,
        1,
        0,
        ItemContext::None,
        0,
    );
    child.set_container_guid_and_slot(bag_guid, INVENTORY_SLOT_ITEM_START);
    session.insert_inventory_item_object(child);
    session.set_player_inventory_persistence_port_like_cpp(
        PlayerInventoryPersistencePortFixtureLikeCpp::failed(),
    );
    let item = session
        .get_inventory_item_by_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START)
        .expect("bag metadata");

    assert!(
        !session
            .destroy_inventory_full_stack_by_pos_like_cpp(
                INVENTORY_SLOT_BAG_0,
                INVENTORY_SLOT_ITEM_START,
                item,
                Some(bag),
                "recursive destroy test",
            )
            .await
    );
    assert!(
        session
            .inventory_item_objects_like_cpp()
            .contains_key(&bag_guid)
    );
    assert!(
        session
            .inventory_item_objects_like_cpp()
            .contains_key(&child_guid)
    );
    assert_eq!(
        inventory_failure_result(&send_rx.try_recv().expect("transaction failure packet")),
        InventoryResult::InternalBagError as i32
    );
}

#[tokio::test]
async fn auto_equip_item_rejects_bad_inv_update_count_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(1);
    session.set_player_guid(Some(ObjectGuid::create_player(1, 42)));

    session
        .handle_auto_equip_item(AutoEquipItem {
            inv_update: InvUpdate { items: Vec::new() },
            pack_slot: INVENTORY_SLOT_BAG_0,
            slot: INVENTORY_SLOT_ITEM_START,
        })
        .await;

    assert!(
        send_rx.try_recv().is_err(),
        "C++ returns on invalid InvUpdate count before missing-item handling"
    );
}

#[tokio::test]
async fn auto_store_bag_item_rejects_non_empty_inv_update_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(1);
    session.set_player_guid(Some(ObjectGuid::create_player(1, 42)));

    session
        .handle_auto_store_bag_item(AutoStoreBagItem {
            inv_update: InvUpdate {
                items: vec![(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START)],
            },
            container_slot_a: INVENTORY_SLOT_BAG_START,
            container_slot_b: INVENTORY_SLOT_BAG_0,
            slot_a: 0,
        })
        .await;

    assert!(
        send_rx.try_recv().is_err(),
        "C++ returns on non-empty InvUpdate before source-container validation"
    );
}

#[tokio::test]
async fn alter_appearance_on_represented_barber_chair_records_request_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(4);
    let player_guid = ObjectGuid::create_player(1, 42);
    let gameobject_guid =
        ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, 777, 22);
    let chair_position = Position::new(1.0, 2.0, 3.0, 0.0);

    session.set_player_guid(Some(player_guid));
    session.set_loaded_player_identity_like_cpp(571, 1, 1, 80, 0);
    assert!(session.use_represented_gameobject_barber_chair_like_cpp(
        gameobject_guid,
        player_guid,
        chair_position,
        wow_entities::BarberChairUseSource {
            chair_height: 2,
            sit_anim_kit: 0,
            customization_scope: 7,
        },
    ));
    let _enable_barber_shop = send_rx.try_recv().unwrap();

    session
        .handle_alter_appearance(alter_appearance_packet(1, 7, 11, &[(20, 200), (10, 100)]))
        .await;

    assert_eq!(
        read_barber_shop_result(send_rx.try_recv().unwrap()),
        BARBER_SHOP_RESULT_SUCCESS_LIKE_CPP
    );
    assert_eq!(
        session.represented_alter_appearance_requests_like_cpp(),
        &[RepresentedAlterAppearanceLikeCpp {
            new_sex: 1,
            customizations: vec![
                ChrCustomizationChoice {
                    option_id: 10,
                    choice_id: 100,
                },
                ChrCustomizationChoice {
                    option_id: 20,
                    choice_id: 200,
                },
            ],
            customized_race: 7,
            customized_chr_model_id: 11,
            cost: 0,
        }]
    );
}

#[tokio::test]
async fn confirm_barbers_choice_records_request_without_success_packet_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(1);

    session
        .handle_confirm_barbers_choice(confirm_barbers_choice_packet(&[(20, 200), (10, 100)]))
        .await;

    assert!(send_rx.try_recv().is_err());
    assert_eq!(
        session.represented_confirm_barbers_choice_requests_like_cpp(),
        &[RepresentedConfirmBarbersChoiceLikeCpp {
            customizations: vec![
                ChrCustomizationChoice {
                    option_id: 20,
                    choice_id: 200,
                },
                ChrCustomizationChoice {
                    option_id: 10,
                    choice_id: 100,
                },
            ],
            cost: 0,
        }]
    );
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
    session.set_player_guid(Some(ObjectGuid::create_player(1, 42)));
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
    session.set_represented_homebind_like_cpp(RepresentedHomebindLikeCpp {
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

#[tokio::test]
async fn request_stabled_pets_without_stable_master_is_silent_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(1);
    let stable_master = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 22, 1);
    let mut request = WorldPacket::new_empty();
    request.write_packed_guid(&stable_master);

    session.handle_request_stabled_pets(request).await;

    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn spirit_healer_activate_without_interactable_healer_is_silent_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(1);
    let healer = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 9, 1);
    let mut request = WorldPacket::new_empty();
    request.write_packed_guid(&healer);

    session.handle_spirit_healer_activate(request).await;

    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn area_spirit_healer_query_sends_time_for_valid_healer_like_cpp() {
    let (mut session, send_rx, canonical) = make_area_spirit_healer_session(4);
    let healer = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 91, 1);
    insert_area_spirit_healer_creature(
        &canonical,
        healer,
        Position::new(10.0, 0.0, 0.0, 0.0),
        NPCFlags1::AREA_SPIRIT_HEALER.bits(),
        0,
    );
    let mut request = WorldPacket::new_empty();
    request.write_packed_guid(&healer);

    session.handle_area_spirit_healer_query(request).await;

    let bytes = send_rx.try_recv().expect("area spirit healer time");
    assert_eq!(
        u16::from_le_bytes([bytes[0], bytes[1]]),
        wow_constants::ServerOpcodes::AreaSpiritHealerTime as u16
    );
    let mut body = WorldPacket::from_bytes(&bytes[2..]);
    assert_eq!(body.read_packed_guid().unwrap(), healer);
    assert_eq!(body.read_int32().unwrap(), 0);
}

#[tokio::test]
async fn area_spirit_healer_query_rejects_out_of_range_healer_like_cpp() {
    let (mut session, send_rx, canonical) = make_area_spirit_healer_session(1);
    let healer = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 91, 2);
    insert_area_spirit_healer_creature(
        &canonical,
        healer,
        Position::new(20.1, 0.0, 0.0, 0.0),
        NPCFlags1::AREA_SPIRIT_HEALER.bits(),
        0,
    );
    let mut request = WorldPacket::new_empty();
    request.write_packed_guid(&healer);

    session.handle_area_spirit_healer_query(request).await;

    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn area_spirit_healer_queue_records_valid_healer_like_cpp() {
    let (mut session, send_rx, canonical) = make_area_spirit_healer_session(1);
    let healer = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 91, 3);
    insert_area_spirit_healer_creature(
        &canonical,
        healer,
        Position::new(10.0, 0.0, 0.0, 0.0),
        NPCFlags1::AREA_SPIRIT_HEALER.bits(),
        0,
    );
    let mut request = WorldPacket::new_empty();
    request.write_packed_guid(&healer);

    session.handle_area_spirit_healer_queue(request).await;

    assert_eq!(session.area_spirit_healer_guid_like_cpp(), healer);
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn hearth_and_resurrect_allowed_area_resurrects_and_teleports_home_like_cpp() {
    let (mut session, send_rx) = make_hearth_and_resurrect_session(
        wow_data::AREA_FLAG_ALLOW_HEARTH_AND_RESURRECT_FROM_AREA_LIKE_CPP,
    );

    session
        .handle_hearth_and_resurrect(WorldPacket::new_empty())
        .await;

    assert!(session.player_is_alive_like_cpp());
    assert_eq!(
        std::iter::from_fn(|| send_rx.try_recv().ok())
            .map(|bytes| u16::from_le_bytes([bytes[0], bytes[1]]))
            .collect::<Vec<_>>(),
        vec![
            wow_constants::ServerOpcodes::CancelCombat as u16,
            wow_constants::ServerOpcodes::MoveTeleport as u16,
        ]
    );
}

#[tokio::test]
async fn hearth_and_resurrect_rejects_area_without_cpp_flag() {
    let (mut session, send_rx) = make_hearth_and_resurrect_session(0);

    session
        .handle_hearth_and_resurrect(WorldPacket::new_empty())
        .await;

    assert!(!session.player_is_alive_like_cpp());
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn hearth_and_resurrect_rejects_player_in_flight_like_cpp() {
    let (mut session, send_rx) = make_hearth_and_resurrect_session(
        wow_data::AREA_FLAG_ALLOW_HEARTH_AND_RESURRECT_FROM_AREA_LIKE_CPP,
    );
    session.set_taxi_flight_state_like_cpp(
        RepresentedTaxiFlightNodeLikeCpp {
            map_id: 571,
            position: Position::new(1.0, 2.0, 3.0, 0.0),
            teleport_flag: false,
        },
        None,
    );

    session
        .handle_hearth_and_resurrect(WorldPacket::new_empty())
        .await;

    assert!(!session.player_is_alive_like_cpp());
    assert!(send_rx.try_recv().is_err());
}

#[test]
fn committed_money_callers_publish_all_runtime_state_before_reopening_admission() {
    fn assert_publication_segment(
        source: &str,
        operation: &str,
        publication_start: &str,
        required_runtime_publications: &[&str],
    ) {
        let operation_marker = format!("\"{operation}\"");
        let operation_offset = source
            .find(&operation_marker)
            .unwrap_or_else(|| panic!("missing operation marker {operation_marker}"));
        let after_operation = &source[operation_offset..];
        let publication_offset = after_operation.find(publication_start).unwrap_or_else(|| {
            panic!("{operation}: missing publication start {publication_start}")
        });
        let after_publication = &after_operation[publication_offset..];
        let drop_offset = after_publication
            .find("drop(money_persistence);")
            .unwrap_or_else(|| panic!("{operation}: missing money guard drop"));
        let publication = &after_publication[..drop_offset];

        assert!(
            !publication.contains(".await"),
            "{operation}: an await reintroduced a post-COMMIT cancellation point before runtime publication"
        );
        for required in required_runtime_publications {
            assert!(
                publication.contains(required),
                "{operation}: runtime publication `{required}` must precede the money guard drop"
            );
        }
    }

    // #224 split the former `character.rs` into private feature modules; this
    // publication-order scan must still see the whole family's source.
    let character = concat!(
        include_str!("character/mod.rs"),
        include_str!("character/account.rs"),
        include_str!("character/bank.rs"),
        include_str!("character/gossip.rs"),
        include_str!("character/items.rs"),
        include_str!("character/lifecycle.rs"),
        include_str!("character/query.rs"),
        include_str!("character/session_state.rs"),
        include_str!("character/vendor.rs"),
        include_str!("character/visibility.rs"),
        include_str!("character/world_entry.rs"),
    );
    // #236 split the former `session.rs`; the committed-money callers stayed
    // in `mod.rs`. If they move again this scan must follow them - the
    // assertions below fail loudly on a missing marker rather than passing.
    let session = include_str!("../session/mod.rs");
    assert_publication_segment(
        character,
        "bank-slot purchase",
        "self.set_player_gold_like_cpp(new_money);",
        &[
            "self.set_player_bank_bag_slot_count_like_cpp(new_count);",
            "self.sync_player_registry_state_like_cpp();",
        ],
    );
    assert_publication_segment(
        character,
        "vendor item purchase",
        "self.stage_player_money_change_like_cpp",
        &[
            "self.apply_item_turnin_changes",
            "self.set_player_currencies_like_cpp(planned_currencies);",
            "self.insert_inventory_item_like_cpp",
            "self.update_vendor_item_current_count",
        ],
    );
    assert_publication_segment(
        character,
        "vendor currency purchase",
        "self.set_player_currencies_like_cpp(planned_currencies);",
        &["self.apply_item_turnin_changes"],
    );
    assert_publication_segment(
        character,
        "vendor buyback purchase",
        "self.stage_player_money_change_like_cpp",
        &[
            "self.remove_buyback_item_like_cpp",
            "self.insert_inventory_item_like_cpp",
        ],
    );
    assert_publication_segment(
        character,
        "vendor item sale",
        "self.stage_player_money_change_like_cpp",
        &[
            "self.set_buyback_slot_metadata_like_cpp",
            "self.insert_buyback_item_like_cpp",
        ],
    );
    assert_publication_segment(
        character,
        "vendor item purchase refund",
        "self.stage_player_money_change_like_cpp",
        &[
            "self.remove_inventory_item_like_cpp(refund_slot);",
            "self.insert_inventory_item_like_cpp",
        ],
    );
    assert_publication_segment(
        session,
        "single item durability repair",
        "self.stage_player_money_change_like_cpp",
        &["self.apply_inventory_item_durability_repair_runtime_like_cpp(item_guid)"],
    );
    assert_publication_segment(
        session,
        "all-items durability repair",
        "self.stage_player_money_change_like_cpp",
        &["self.apply_inventory_item_durability_repair_runtime_like_cpp(item_guid)"],
    );
}

#[tokio::test]
async fn buy_bank_slot_buys_next_slot_and_spends_money_like_cpp() {
    let (mut session, send_rx, canonical) = make_bank_slot_session(4);
    let banker = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 2456, 1);
    insert_banker_creature(&canonical, banker, NPCFlags1::BANKER.bits());
    let port = CollectionLoadPortLikeCpp::for_bank_slot_purchase([
        wow_persistence::PlayerMoneyTransactionOutcomeLikeCpp::Committed,
    ]);
    session.set_player_lifecycle_port_like_cpp(port.clone());

    session
        .handle_buy_bank_slot(BuyBankSlot { guid: banker })
        .await;

    assert_eq!(session.player_bank_bag_slot_count_like_cpp(), 1);
    assert_eq!(session.player_gold_like_cpp(), 50);
    assert!(
        send_rx.try_recv().is_ok(),
        "bank slot update should be sent"
    );
    assert!(send_rx.try_recv().is_ok(), "money update should be sent");
    assert!(send_rx.try_recv().is_err());
    assert_eq!(
        port.bank_slot_purchase_requests(),
        vec![wow_persistence::PlayerBankSlotPurchaseRequestLikeCpp {
            player_guid: 42,
            money_after: 50,
            bank_slot_count: 1,
        }]
    );
}

#[tokio::test]
async fn buy_bank_slot_definite_rollback_keeps_runtime_and_packets_unchanged_like_cpp() {
    let (mut session, send_rx, canonical) = make_bank_slot_session(4);
    let banker = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 2456, 4);
    insert_banker_creature(&canonical, banker, NPCFlags1::BANKER.bits());
    let port = CollectionLoadPortLikeCpp::for_bank_slot_purchase([
        wow_persistence::PlayerMoneyTransactionOutcomeLikeCpp::DefinitelyRolledBack {
            reason: "fixture rollback".to_owned(),
        },
    ]);
    session.set_player_lifecycle_port_like_cpp(port.clone());

    session
        .handle_buy_bank_slot(BuyBankSlot { guid: banker })
        .await;

    assert_eq!(session.player_bank_bag_slot_count_like_cpp(), 0);
    assert_eq!(session.player_gold_like_cpp(), 150);
    assert!(send_rx.try_recv().is_err());
    assert!(
        session
            .durable_loot_money_persistence_tracker_like_cpp()
            .begin_like_cpp()
            .is_ok(),
        "a definite rollback must reopen payout admission"
    );
    assert_eq!(
        port.bank_slot_purchase_requests(),
        vec![wow_persistence::PlayerBankSlotPurchaseRequestLikeCpp {
            player_guid: 42,
            money_after: 50,
            bank_slot_count: 1,
        }]
    );
}

#[tokio::test]
async fn buy_bank_slot_rejects_non_banker_like_cpp() {
    let (mut session, send_rx, canonical) = make_bank_slot_session(1);
    let creature = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 2456, 2);
    insert_banker_creature(&canonical, creature, NPCFlags1::QUEST_GIVER.bits());

    session
        .handle_buy_bank_slot(BuyBankSlot { guid: creature })
        .await;

    assert_eq!(session.player_bank_bag_slot_count_like_cpp(), 0);
    assert_eq!(session.player_gold_like_cpp(), 150);
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn buy_bank_slot_rejects_missing_price_like_cpp() {
    let (mut session, send_rx, canonical) = make_bank_slot_session(1);
    let banker = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 2456, 3);
    insert_banker_creature(&canonical, banker, NPCFlags1::BANKER.bits());
    session.set_player_bank_bag_slot_count_like_cpp(2);

    session
        .handle_buy_bank_slot(BuyBankSlot { guid: banker })
        .await;

    assert_eq!(session.player_bank_bag_slot_count_like_cpp(), 2);
    assert_eq!(session.player_gold_like_cpp(), 150);
    assert!(send_rx.try_recv().is_err());
}

#[test]
fn bank_move_plan_selects_first_personal_bank_slot_like_cpp() {
    let (mut session, _send_rx, _canonical) = make_bank_slot_session(1);
    install_bank_move_item_fixture(&mut session, 700, 10);
    let source_guid =
        insert_bank_move_test_item(&mut session, INVENTORY_SLOT_ITEM_START, 700, 7_001, 3);

    let plan = session
        .plan_inventory_storage_move_like_cpp(
            INVENTORY_SLOT_BAG_0,
            INVENTORY_SLOT_ITEM_START,
            NULL_BAG,
            NULL_SLOT,
            InventoryStorageTargetLikeCpp::Bank,
        )
        .expect("source item")
        .expect("valid bank plan");

    assert_eq!(plan.source.guid, source_guid);
    assert!(plan.existing_updates.is_empty());
    assert_eq!(
        plan.moved_destination,
        Some((INVENTORY_SLOT_BAG_0, wow_entities::BANK_SLOT_ITEM_START, 3))
    );
}

#[test]
fn bank_move_plan_merges_then_moves_one_remainder_stack_like_cpp() {
    let (mut session, _send_rx, _canonical) = make_bank_slot_session(1);
    install_bank_move_item_fixture(&mut session, 701, 10);
    insert_bank_move_test_item(&mut session, INVENTORY_SLOT_ITEM_START, 701, 7_011, 5);
    let existing_guid = insert_bank_move_test_item(
        &mut session,
        wow_entities::BANK_SLOT_ITEM_START,
        701,
        7_012,
        8,
    );

    let plan = session
        .plan_inventory_storage_move_like_cpp(
            INVENTORY_SLOT_BAG_0,
            INVENTORY_SLOT_ITEM_START,
            NULL_BAG,
            NULL_SLOT,
            InventoryStorageTargetLikeCpp::Bank,
        )
        .expect("source item")
        .expect("valid bank plan");

    assert_eq!(plan.existing_updates.len(), 1);
    assert_eq!(plan.existing_updates[0].item.guid, existing_guid);
    assert_eq!(plan.existing_updates[0].new_count, 10);
    assert_eq!(
        plan.moved_destination,
        Some((
            INVENTORY_SLOT_BAG_0,
            wow_entities::BANK_SLOT_ITEM_START + 1,
            3,
        ))
    );
}

#[test]
fn bank_merge_refreshes_destination_enchant_timer_without_item_duration_like_cpp() {
    let (mut session, send_rx, _canonical) = make_bank_slot_session(2);
    install_bank_move_item_fixture(&mut session, 708, 10);
    attach_stat_update_player_with_mana(&mut session, ObjectGuid::create_player(1, 42), 0, 0);
    let destination_guid = insert_bank_move_test_item(
        &mut session,
        wow_entities::BANK_SLOT_ITEM_START,
        708,
        7_081,
        8,
    );
    session.update_inventory_item_object_like_cpp(destination_guid, |item| {
        item.set_expiration(300);
        item.set_enchantment(EnchantmentSlot::EnhancementTemporary, 940, 12_000, 1);
    });
    let mut tracked_item = session.inventory_item_objects_like_cpp()[&destination_guid].clone();
    session
        .mutate_canonical_player_like_cpp(|player| {
            player.add_enchantment_duration(
                &mut tracked_item,
                EnchantmentSlot::EnhancementTemporary,
                7_000,
            )
        })
        .expect("canonical player");

    session.refresh_inventory_item_enchantment_duration_refs_like_cpp(destination_guid);

    let mut packet = WorldPacket::from_bytes(
        &send_rx
            .try_recv()
            .expect("destination enchantment duration update"),
    );
    assert_eq!(
        packet.read_uint16().unwrap(),
        ServerOpcodes::ItemEnchantTimeUpdate as u16
    );
    assert_eq!(packet.read_packed_guid().unwrap(), destination_guid);
    assert_eq!(packet.read_uint32().unwrap(), 12);
    assert_eq!(
        packet.read_uint32().unwrap(),
        EnchantmentSlot::EnhancementTemporary as u32
    );
    assert_eq!(
        packet.read_packed_guid().unwrap(),
        session.player_guid().unwrap()
    );
    assert!(
        send_rx.try_recv().is_err(),
        "C++ merge branch refreshes AddEnchantmentDurations but does not emit AddItemDurations"
    );
}

#[test]
fn bank_move_plan_can_merge_and_leave_remainder_in_source_slot_like_cpp() {
    let (mut session, _send_rx, _canonical) = make_bank_slot_session(1);
    install_bank_move_item_fixture(&mut session, 705, 10);
    insert_bank_move_test_item(
        &mut session,
        wow_entities::BANK_SLOT_ITEM_START,
        705,
        7_051,
        5,
    );
    let merge_guid = insert_bank_move_test_item(
        &mut session,
        wow_entities::BANK_SLOT_ITEM_START + 1,
        705,
        7_052,
        8,
    );

    let plan = session
        .plan_inventory_storage_move_like_cpp(
            INVENTORY_SLOT_BAG_0,
            wow_entities::BANK_SLOT_ITEM_START,
            NULL_BAG,
            NULL_SLOT,
            InventoryStorageTargetLikeCpp::Bank,
        )
        .expect("source item")
        .expect("valid consolidation plan");

    assert_eq!(plan.existing_updates.len(), 1);
    assert_eq!(plan.existing_updates[0].item.guid, merge_guid);
    assert_eq!(plan.existing_updates[0].new_count, 10);
    assert_eq!(
        plan.moved_destination,
        Some((INVENTORY_SLOT_BAG_0, wow_entities::BANK_SLOT_ITEM_START, 3))
    );
}

#[test]
fn bank_move_plan_reports_bank_full_like_cpp() {
    let (mut session, _send_rx, _canonical) = make_bank_slot_session(1);
    install_bank_move_item_fixture(&mut session, 706, 1);
    insert_bank_move_test_item(&mut session, INVENTORY_SLOT_ITEM_START, 706, 7_061, 1);
    for (index, slot) in
        (wow_entities::BANK_SLOT_ITEM_START..wow_entities::BANK_SLOT_ITEM_END).enumerate()
    {
        insert_bank_move_test_item(&mut session, slot, 706, 7_100 + index as u64, 1);
    }

    assert!(matches!(
        session.plan_inventory_storage_move_like_cpp(
            INVENTORY_SLOT_BAG_0,
            INVENTORY_SLOT_ITEM_START,
            NULL_BAG,
            NULL_SLOT,
            InventoryStorageTargetLikeCpp::Bank,
        ),
        Some(Err(InventoryResult::BankFull))
    ));
}

#[test]
fn autostore_bank_move_plan_returns_item_to_backpack_like_cpp() {
    let (mut session, _send_rx, _canonical) = make_bank_slot_session(1);
    install_bank_move_item_fixture(&mut session, 702, 1);
    insert_bank_move_test_item(
        &mut session,
        wow_entities::BANK_SLOT_ITEM_START,
        702,
        7_021,
        1,
    );

    let plan = session
        .plan_inventory_storage_move_like_cpp(
            INVENTORY_SLOT_BAG_0,
            wow_entities::BANK_SLOT_ITEM_START,
            NULL_BAG,
            NULL_SLOT,
            InventoryStorageTargetLikeCpp::Inventory,
        )
        .expect("source item")
        .expect("valid inventory plan");

    assert_eq!(
        plan.moved_destination,
        Some((INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START, 1))
    );
}

#[test]
fn bank_move_quest_removal_follows_opcode_not_direction_like_cpp() {
    assert_eq!(
        autostore_bank_quest_checks_like_cpp(InventoryStorageTargetLikeCpp::Bank),
        InventoryStorageQuestChecksLikeCpp::None,
        "C++ AutoStore inventory-to-bank must select no quest check even though its target is Bank"
    );
    assert_eq!(
        autostore_bank_quest_checks_like_cpp(InventoryStorageTargetLikeCpp::Inventory),
        InventoryStorageQuestChecksLikeCpp::AutoStoreBankItemAdded
    );
}

#[test]
fn legacy_zero_inventory_slots_loads_base_backpack_capacity() {
    assert_eq!(
        loaded_inventory_slot_count_with_legacy_rust_compat(0),
        INVENTORY_DEFAULT_SIZE
    );
    assert_eq!(loaded_inventory_slot_count_with_legacy_rust_compat(24), 24);
}

#[test]
fn autostore_full_merge_reports_destination_stack_total_like_cpp() {
    let (mut session, _send_rx, _canonical) = make_bank_slot_session(1);
    install_bank_move_item_fixture(&mut session, 709, 10);
    insert_bank_move_test_item(
        &mut session,
        wow_entities::BANK_SLOT_ITEM_START,
        709,
        7_091,
        2,
    );
    insert_bank_move_test_item(&mut session, INVENTORY_SLOT_ITEM_START, 709, 7_092, 8);

    let plan = session
        .plan_inventory_storage_move_like_cpp(
            INVENTORY_SLOT_BAG_0,
            wow_entities::BANK_SLOT_ITEM_START,
            NULL_BAG,
            NULL_SLOT,
            InventoryStorageTargetLikeCpp::Inventory,
        )
        .expect("source item")
        .expect("valid inventory plan");

    assert!(plan.moved_destination.is_none());
    assert_eq!(plan.existing_updates[0].new_count, 10);
    assert_eq!(bank_store_item_added_quest_count_like_cpp(&plan), 10);
}

#[test]
fn inventory_move_quest_checks_only_cross_bank_boundary_like_cpp() {
    assert_eq!(
        inventory_storage_move_quest_directions_like_cpp(
            INVENTORY_SLOT_BAG_0,
            INVENTORY_SLOT_ITEM_START,
            InventoryStorageTargetLikeCpp::Inventory,
        ),
        (false, false),
        "ordinary and child inventory relocations must not re-credit quest items"
    );
    assert_eq!(
        inventory_storage_move_quest_directions_like_cpp(
            INVENTORY_SLOT_BAG_0,
            wow_entities::BANK_SLOT_ITEM_START,
            InventoryStorageTargetLikeCpp::Inventory,
        ),
        (false, true)
    );
    assert_eq!(
        inventory_storage_move_quest_directions_like_cpp(
            INVENTORY_SLOT_BAG_0,
            INVENTORY_SLOT_ITEM_START,
            InventoryStorageTargetLikeCpp::Bank,
        ),
        (true, false)
    );
}

#[test]
fn top_level_bank_destination_applies_obtain_spells_like_cpp_store_item() {
    assert!(bank_store_destination_applies_obtain_spells_like_cpp(
        INVENTORY_SLOT_BAG_0
    ));
    assert!(bank_store_destination_applies_obtain_spells_like_cpp(
        wow_entities::INVENTORY_SLOT_BAG_START
    ));
    assert!(
        !bank_store_destination_applies_obtain_spells_like_cpp(wow_entities::BANK_SLOT_BAG_START),
        "C++ _StoreItem excludes bank-bag containers but not bag-0 bank slots"
    );
}

#[test]
fn mainhand_bank_remove_clears_and_persists_weapon_only_enchant_like_cpp() {
    let (mut session, _send_rx, _canonical) = make_bank_slot_session(1);
    install_bank_move_item_fixture(&mut session, 707, 1);
    let item_guid =
        insert_bank_move_test_item(&mut session, EQUIPMENT_SLOT_MAINHAND, 707, 7_071, 1);
    attach_stat_update_player_with_mana(&mut session, ObjectGuid::create_player(1, 42), 0, 0);
    let enchantment_entry = |id, flags| wow_data::SpellItemEnchantmentEntry {
        id,
        effect_arg: [0; 3],
        effect_points_min: [0; 3],
        item_visual: 0,
        flags,
        required_skill_id: 0,
        required_skill_rank: 0,
        item_level: 1,
        charges: 0,
        effect: [wow_constants::ItemEnchantmentType::None as u8; 3],
        condition_id: 0,
        min_level: 1,
        max_level: 0,
    };
    session.set_spell_item_enchantment_store(Arc::new(
        wow_data::SpellItemEnchantmentStore::from_entries([
            enchantment_entry(930, wow_constants::SpellItemEnchantmentFlags::MAINHAND_ONLY),
            enchantment_entry(931, wow_constants::SpellItemEnchantmentFlags::empty()),
            enchantment_entry(
                932,
                wow_constants::SpellItemEnchantmentFlags::DO_NOT_SAVE_TO_DB,
            ),
        ]),
    ));
    session.update_inventory_item_object_like_cpp(item_guid, |item| {
        item.set_item_flag2(wow_constants::ItemFieldFlags2::EQUIPPED);
        item.set_enchantment(EnchantmentSlot::EnhancementPermanent, 930, 4_000, 2);
        item.set_enchantment(EnchantmentSlot::EnhancementTemporary, 931, 3_000, 1);
        item.set_enchantment(EnchantmentSlot::Property0, 932, 2_000, 3);
        item.set_enchantment(EnchantmentSlot::Property1, 999, 1_000, 4);
    });
    let mut timed_item = session.inventory_item_objects_like_cpp()[&item_guid].clone();
    session
        .mutate_canonical_player_like_cpp(|player| {
            player.add_enchantment_duration(
                &mut timed_item,
                EnchantmentSlot::EnhancementTemporary,
                1_500,
            )
        })
        .expect("canonical player");

    let (persisted, cleared) = session
        .inventory_remove_enchantment_persistence_like_cpp(item_guid, true)
        .expect("main-hand-only enchantment");
    assert_eq!(cleared, vec![EnchantmentSlot::EnhancementPermanent]);
    let fields: Vec<_> = persisted.split_whitespace().collect();
    assert_eq!(&fields[0..3], &["0", "0", "0"]);
    assert_eq!(&fields[3..6], &["931", "1500", "1"]);
    assert_eq!(&fields[24..27], &["0", "0", "0"]);
    assert_eq!(&fields[27..30], &["0", "0", "0"]);

    let _ = session.apply_inventory_item_remove_side_effects_like_cpp(
        INVENTORY_SLOT_BAG_0,
        EQUIPMENT_SLOT_MAINHAND,
        item_guid,
        &cleared,
    );
    let item = &session.inventory_item_objects_like_cpp()[&item_guid];
    assert!(!item.has_item_flag2(wow_constants::ItemFieldFlags2::EQUIPPED));
    assert_eq!(
        item.data().enchantments[EnchantmentSlot::EnhancementPermanent as usize].id,
        0
    );
    let update =
        WorldSession::item_storage_fields_values_update_like_cpp(item, true, true, &cleared);
    let packet_update = crate::entity_update_bridge::item_values_update_to_packet(&update)
        .expect("item values update");
    let expected_mask = (1_u64 << wow_entities::ITEM_DATA_PARENT_BIT)
        | (1_u64 << wow_entities::ITEM_DATA_CONTAINED_IN_BIT)
        | (1_u64 << wow_entities::ITEM_DATA_DYNAMIC_FLAGS2_BIT)
        | (1_u64 << wow_entities::ITEM_DATA_ENCHANTMENT_PARENT_BIT)
        | (1_u64
            << (wow_entities::ITEM_DATA_ENCHANTMENT_FIRST_BIT
                + EnchantmentSlot::EnhancementPermanent as usize));
    assert_eq!(packet_update.item_data_mask, expected_mask);
    assert_eq!(packet_update.dynamic_flags2, 0);
    assert_eq!(
        packet_update.enchantments[EnchantmentSlot::EnhancementPermanent as usize].id,
        0
    );
    assert_eq!(
        session.represented_combat_stat_recalculations_like_cpp(),
        &[
            crate::session::RepresentedCombatStatRecalculationLikeCpp::Expertise {
                attack: wow_constants::WeaponAttackType::BaseAttack,
            },
            crate::session::RepresentedCombatStatRecalculationLikeCpp::Rating { combat_rating: 24 },
        ]
    );
}

#[test]
fn committed_bank_relocation_updates_runtime_only_after_explicit_apply() {
    let (mut session, _send_rx, _canonical) = make_bank_slot_session(1);
    install_bank_move_item_fixture(&mut session, 703, 10);
    let source_guid =
        insert_bank_move_test_item(&mut session, INVENTORY_SLOT_ITEM_START, 703, 7_031, 4);

    assert_eq!(
        session
            .get_inventory_item_by_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START)
            .map(|item| item.guid),
        Some(source_guid)
    );
    assert_eq!(session.represented_non_bank_item_count_like_cpp(703), 4);
    assert!(session.apply_committed_inventory_item_relocation_like_cpp(
        INVENTORY_SLOT_BAG_0,
        INVENTORY_SLOT_ITEM_START,
        INVENTORY_SLOT_BAG_0,
        wow_entities::BANK_SLOT_ITEM_START,
        4,
    ));
    assert!(
        session
            .get_inventory_item_by_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START)
            .is_none()
    );
    assert_eq!(
        session
            .get_inventory_item_by_pos(INVENTORY_SLOT_BAG_0, wow_entities::BANK_SLOT_ITEM_START,)
            .map(|item| item.guid),
        Some(source_guid)
    );
    assert_eq!(session.represented_non_bank_item_count_like_cpp(703), 0);
}

#[tokio::test]
async fn binder_activate_sets_current_homebind_and_sends_bind_packets_like_cpp() {
    let (mut session, instance_rx, canonical) = make_bank_slot_session(16);
    insert_bank_test_player_in_world(&session, &canonical);
    let player_guid = session.player_guid().expect("loaded player");
    let homebind_port = HomebindPortFixtureLikeCpp::new([PersistenceOutcomeLikeCpp::Failed {
        reason: "detached write failure".to_owned(),
    }]);
    session.set_player_lifecycle_port_like_cpp(homebind_port.clone());
    let (realm_tx, realm_rx) = flume::bounded::<Vec<u8>>(16);
    session.install_realm_send_channel_for_test(realm_tx);
    let innkeeper = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 2456, 30);
    insert_banker_creature(&canonical, innkeeper, NPCFlags1::INNKEEPER.bits());
    session.set_player_zone_area_like_cpp(12, 34);
    install_bind_spell_fixture(&mut session);
    session.set_player_trainer_interaction_like_cpp(innkeeper, 77);
    let _ = WorldSession::game_time_ms_like_cpp();
    std::thread::sleep(std::time::Duration::from_millis(2));
    let cast_time_lower_bound = WorldSession::game_time_ms_like_cpp();

    session
        .handle_binder_activate(Hello { unit: innkeeper })
        .await;
    let cast_time_upper_bound = WorldSession::game_time_ms_like_cpp();

    assert_eq!(
        session.represented_homebind_like_cpp(),
        Some(RepresentedHomebindLikeCpp {
            map_id: 571,
            area_id: 34,
            position: Position::new(0.0, 0.0, 0.0, 0.0),
        })
    );
    let packets: Vec<Vec<u8>> = instance_rx.try_iter().collect();
    assert_eq!(
        packets
            .iter()
            .filter_map(|bytes| WorldPacket::from_bytes(bytes).server_opcode())
            .collect::<Vec<_>>(),
        vec![ServerOpcodes::SpellGo, ServerOpcodes::BindPointUpdate,]
    );
    assert_eq!(
        realm_rx
            .try_iter()
            .filter_map(|bytes| WorldPacket::from_bytes(&bytes).server_opcode())
            .collect::<Vec<_>>(),
        vec![ServerOpcodes::PlayerBound, ServerOpcodes::GossipComplete],
        "C++ routes PlayerBound and GossipComplete on realm"
    );
    assert!(
        session.player_interaction_source_guid_like_cpp().is_none(),
        "C++ PlayerMenu::SendCloseGossip resets interaction provenance"
    );
    assert_eq!(session.player_interaction_trainer_id_like_cpp(), 0);
    let mut spell_go = WorldPacket::from_bytes(&packets[0]);
    assert_eq!(
        spell_go.read_uint16().expect("SpellGo opcode"),
        ServerOpcodes::SpellGo as u16
    );
    assert_eq!(
        spell_go.read_packed_guid().expect("SpellGo caster"),
        innkeeper,
        "C++ creature CastSpell keeps the innkeeper as visible caster"
    );
    assert_eq!(
        spell_go.read_packed_guid().expect("SpellGo caster unit"),
        innkeeper
    );
    let _ = spell_go.read_packed_guid().expect("SpellGo cast id");
    let _ = spell_go
        .read_packed_guid()
        .expect("SpellGo original cast id");
    assert_eq!(spell_go.read_int32().expect("SpellGo spell id"), 3286);
    let _ = SpellCastVisual::read(&mut spell_go).expect("SpellGo visual");
    assert_eq!(
        spell_go.read_uint32().expect("SpellGo cast flags"),
        0x0004_0101,
        "C++ bind SpellGo carries UNKNOWN_9 | PENDING | NO_GCD"
    );
    assert_eq!(spell_go.read_uint32().expect("SpellGo cast flags ex"), 0);
    let cast_time_ms = spell_go.read_uint32().expect("SpellGo cast time");
    assert!(
        (cast_time_lower_bound..=cast_time_upper_bound).contains(&cast_time_ms),
        "C++ SpellGo CastTime is the wrapping getMSTime() server timestamp"
    );
    for _ in 0..20 {
        if !homebind_port.requests().is_empty() {
            break;
        }
        tokio::task::yield_now().await;
    }
    assert_eq!(
        homebind_port.requests(),
        vec![PlayerHomebindPersistenceRequestLikeCpp::UpdateLive {
            player_guid: player_guid.counter() as u64,
            map_id: 571,
            area_id: 34,
            x: 0.0,
            y: 0.0,
            z: 0.0,
            orientation: 0.0,
        }],
        "detached persistence failure does not suppress the immediate C++ bind packets"
    );
}

#[tokio::test]
async fn binder_activate_fans_spell_go_to_visible_nearby_observers_like_cpp() {
    let (mut session, sender_rx, canonical) = make_bank_slot_session(16);
    insert_bank_test_player_in_world(&session, &canonical);
    let innkeeper = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 2456, 32);
    insert_banker_creature(&canonical, innkeeper, NPCFlags1::INNKEEPER.bits());
    session.set_player_zone_area_like_cpp(12, 34);
    install_bind_spell_fixture(&mut session);

    let registry = Arc::new(crate::session::directory::PlayerRegistry::default());
    session.set_player_registry(Arc::clone(&registry));
    let (mut nearby_visible, nearby_visible_rx) = make_binder_observer(
        43,
        Position::new(10.0, 0.0, 0.0, 0.0),
        innkeeper,
        true,
        &registry,
        &canonical,
    );
    let (mut nearby_hidden, nearby_hidden_rx) = make_binder_observer(
        44,
        Position::new(12.0, 0.0, 0.0, 0.0),
        innkeeper,
        false,
        &registry,
        &canonical,
    );
    let (mut distant_visible, distant_visible_rx) = make_binder_observer(
        45,
        Position::new(5_000.0, 0.0, 0.0, 0.0),
        innkeeper,
        true,
        &registry,
        &canonical,
    );

    session
        .handle_binder_activate(Hello { unit: innkeeper })
        .await;
    let activating_player_spell_go = sender_rx.try_recv().expect("activator SpellGo");

    nearby_visible
        .process_represented_session_commands_like_cpp()
        .await;
    nearby_hidden
        .process_represented_session_commands_like_cpp()
        .await;
    distant_visible
        .process_represented_session_commands_like_cpp()
        .await;

    assert_eq!(
        nearby_visible_rx
            .try_recv()
            .expect("visible nearby observer SpellGo"),
        activating_player_spell_go
    );
    assert!(nearby_visible_rx.try_recv().is_err());
    assert!(
        nearby_hidden_rx.try_recv().is_err(),
        "C++ HaveAtClient gate rejects a non-visible innkeeper"
    );
    assert!(
        distant_visible_rx.try_recv().is_err(),
        "C++ MessageDistDeliverer rejects observers outside visibility range"
    );
}

#[tokio::test]
async fn binder_activate_rejects_instanceable_map_like_cpp() {
    let (mut session, send_rx, canonical) = make_bank_slot_session(2);
    let innkeeper = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 2456, 31);
    insert_banker_creature(&canonical, innkeeper, NPCFlags1::INNKEEPER.bits());
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
    player
        .unit_mut()
        .add_unit_state(wow_constants::unit::UnitState::DIED.bits());
    canonical
        .lock()
        .unwrap()
        .create_world_map(571, 0)
        .map_mut()
        .insert_map_object_record(wow_entities::MapObjectRecord::new_player(player).unwrap())
        .unwrap();
    const FEIGN_DEATH_SLOT: u8 = 7;
    session.visible_auras.insert(
        FEIGN_DEATH_SLOT,
        AuraApplication {
            spell_id: 5384,
            difficulty_id: 0,
            caster_guid: player_guid,
            slot: FEIGN_DEATH_SLOT,
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
    session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
        wow_data::MapEntry {
            id: 571,
            instance_type: wow_data::map::MAP_INSTANCE,
            expansion_id: 2,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
    ])));

    session
        .handle_binder_activate(Hello { unit: innkeeper })
        .await;

    assert!(session.represented_homebind_like_cpp().is_none());
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::AuraUpdate],
        "C++ removes feign death before SendBindPoint rejects an instanceable map"
    );
    assert!(!session.visible_auras.contains_key(&FEIGN_DEATH_SLOT));
    assert_eq!(
        session
            .mutate_canonical_player_like_cpp(|player| player
                .unit()
                .has_unit_state(wow_constants::unit::UnitState::DIED.bits()))
            .expect("canonical player"),
        false
    );
}

#[tokio::test]
async fn binder_activate_rejects_non_innkeeper_like_cpp() {
    let (mut session, send_rx, canonical) = make_bank_slot_session(1);
    insert_bank_test_player_in_world(&session, &canonical);
    let creature = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 2456, 31);
    insert_banker_creature(&canonical, creature, NPCFlags1::BANKER.bits());
    session.set_player_zone_area_like_cpp(12, 34);

    session
        .handle_binder_activate(Hello { unit: creature })
        .await;

    assert!(session.represented_homebind_like_cpp().is_none());
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn binder_activate_rejects_player_outside_world_like_cpp() {
    let (mut session, send_rx, canonical) = make_bank_slot_session(1);
    insert_bank_test_player_in_world(&session, &canonical);
    let innkeeper = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 2456, 33);
    insert_banker_creature(&canonical, innkeeper, NPCFlags1::INNKEEPER.bits());
    session.set_player_zone_area_like_cpp(12, 34);
    install_bind_spell_fixture(&mut session);
    assert!(
        session
            .mutate_canonical_player_like_cpp(|player| {
                player
                    .unit_mut()
                    .world_mut()
                    .object_mut()
                    .remove_from_world();
            })
            .is_some(),
        "canonical player fixture"
    );
    assert!(session.player_is_alive_like_cpp());

    session
        .handle_binder_activate(Hello { unit: innkeeper })
        .await;

    assert!(session.represented_homebind_like_cpp().is_none());
    assert!(
        send_rx.try_recv().is_err(),
        "C++ returns before interaction, bind mutation, and packets when Player::IsInWorld is false"
    );
}

#[tokio::test]
async fn binder_activate_rejects_player_missing_from_canonical_world_like_cpp() {
    let (mut session, send_rx, canonical) = make_bank_slot_session(1);
    insert_bank_test_player_in_world(&session, &canonical);
    let innkeeper = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 2456, 34);
    insert_banker_creature(&canonical, innkeeper, NPCFlags1::INNKEEPER.bits());
    session.set_player_zone_area_like_cpp(12, 34);
    install_bind_spell_fixture(&mut session);
    let player_guid = session.player_guid().expect("player guid");
    assert!(
        canonical
            .lock()
            .unwrap()
            .find_map_mut(571, 0)
            .expect("canonical map")
            .map_mut()
            .remove_map_object(player_guid)
            .is_some(),
        "remove canonical player fixture"
    );
    assert!(session.player_is_alive_like_cpp());

    session
        .handle_binder_activate(Hello { unit: innkeeper })
        .await;

    assert!(session.represented_homebind_like_cpp().is_none());
    assert!(
        send_rx.try_recv().is_err(),
        "C++ Player::IsInWorld is false after removal even while the session still has an alive player controller"
    );
}

#[tokio::test]
async fn banker_activate_removes_feign_after_validation_before_open_like_cpp() {
    const FEIGN_SLOT: u8 = 17;
    let (mut session, send_rx, canonical) = make_bank_slot_session(4);
    insert_bank_test_player_in_world(&session, &canonical);
    let banker = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 2456, 37);
    insert_banker_creature(&canonical, banker, NPCFlags1::BANKER.bits());
    session.set_player_trainer_interaction_like_cpp(banker, 77);
    seed_represented_feign_death_like_cpp(&mut session, FEIGN_SLOT);

    session.handle_banker_activate(Hello { unit: banker }).await;

    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![
            ServerOpcodes::AuraUpdate,
            ServerOpcodes::NpcInteractionOpenResult,
        ],
        "C++ removes feign death before SendShowBank"
    );
    assert!(!session.visible_auras.contains_key(&FEIGN_SLOT));
    assert!(!canonical_player_has_died_state_like_cpp(&mut session));
    assert_eq!(
        session.player_interaction_source_guid_like_cpp(),
        Some(banker)
    );
    assert_eq!(session.player_interaction_trainer_id_like_cpp(), 0);
}

#[tokio::test]
async fn banker_activate_invalid_source_preserves_feign_and_provenance_like_cpp() {
    const FEIGN_SLOT: u8 = 18;
    let (mut session, send_rx, canonical) = make_bank_slot_session(2);
    insert_bank_test_player_in_world(&session, &canonical);
    let invalid_banker =
        ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 2456, 38);
    let active_source = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 2456, 39);
    insert_banker_creature(&canonical, invalid_banker, NPCFlags1::VENDOR.bits());
    session.set_player_trainer_interaction_like_cpp(active_source, 77);
    seed_represented_feign_death_like_cpp(&mut session, FEIGN_SLOT);

    session
        .handle_banker_activate(Hello {
            unit: invalid_banker,
        })
        .await;

    assert!(send_rx.try_recv().is_err());
    assert!(session.visible_auras.contains_key(&FEIGN_SLOT));
    assert!(canonical_player_has_died_state_like_cpp(&mut session));
    assert!(
        session.player_trainer_interaction_matches_like_cpp(active_source, 77),
        "invalid banker must return before fake-death removal and SendShowBank"
    );
}

#[tokio::test]
async fn autobank_item_without_persistence_keeps_runtime_unchanged_like_cpp() {
    let (mut session, send_rx, canonical) = make_bank_slot_session(4);
    let banker = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 2456, 40);
    insert_banker_creature(&canonical, banker, NPCFlags1::BANKER.bits());
    install_bank_move_item_fixture(&mut session, 704, 1);
    let source_guid =
        insert_bank_move_test_item(&mut session, INVENTORY_SLOT_ITEM_START, 704, 7_041, 1);

    session.handle_banker_activate(Hello { unit: banker }).await;
    assert!(send_rx.try_recv().is_ok(), "bank open should be sent");
    assert_eq!(
        session.player_interaction_source_guid_like_cpp(),
        Some(banker)
    );
    assert_eq!(session.player_interaction_trainer_id_like_cpp(), 0);

    session
        .handle_autobank_item(AutoBankItem {
            inv_update: InvUpdate {
                items: vec![(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START)],
            },
            bag: INVENTORY_SLOT_BAG_0,
            slot: INVENTORY_SLOT_ITEM_START,
        })
        .await;

    assert_eq!(
        session
            .get_inventory_item_by_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START)
            .map(|item| item.guid),
        Some(source_guid),
        "runtime must not move when no character database can commit the plan"
    );
    assert!(session.represented_bank_item_moves_like_cpp().is_empty());
    assert!(send_rx.try_recv().is_err());
}

#[test]
fn bank_authorization_reads_the_single_interaction_source_like_cpp() {
    let (mut session, _send_rx, canonical) = make_bank_slot_session(2);
    let banker = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 2456, 140);
    let vendor = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 2456, 141);
    insert_banker_creature(&canonical, banker, NPCFlags1::BANKER.bits());
    insert_banker_creature(&canonical, vendor, NPCFlags1::VENDOR.bits());

    session.set_player_trainer_interaction_like_cpp(banker, 77);
    assert!(
        session.represented_can_use_current_bank_like_cpp(),
        "C++ CanUseBank reads SourceGuid and does not impose an interaction kind or TrainerId gate"
    );

    session.set_player_interaction_source_like_cpp(vendor);
    assert!(!session.represented_can_use_current_bank_like_cpp());

    session.set_player_interaction_source_like_cpp(banker);
    assert!(session.represented_can_use_current_bank_like_cpp());
    assert!(session.reset_player_interaction_if_source_like_cpp(banker));
    assert!(!session.represented_can_use_current_bank_like_cpp());
}

#[tokio::test]
async fn autobank_item_commit_failure_keeps_runtime_unchanged_like_cpp() {
    let (mut session, send_rx, canonical) = make_bank_slot_session(4);
    let banker = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 2456, 42);
    insert_banker_creature(&canonical, banker, NPCFlags1::BANKER.bits());
    install_bank_move_item_fixture(&mut session, 708, 1);
    let source_guid =
        insert_bank_move_test_item(&mut session, INVENTORY_SLOT_ITEM_START, 708, 7_081, 1);

    session.handle_banker_activate(Hello { unit: banker }).await;
    assert!(send_rx.try_recv().is_ok(), "bank open should be sent");

    session.set_player_inventory_persistence_port_like_cpp(
        PlayerInventoryPersistencePortFixtureLikeCpp::failed(),
    );
    assert!(session.represented_can_use_current_bank_like_cpp());
    let precommit_plan = session
        .plan_inventory_storage_move_like_cpp(
            INVENTORY_SLOT_BAG_0,
            INVENTORY_SLOT_ITEM_START,
            NULL_BAG,
            NULL_SLOT,
            InventoryStorageTargetLikeCpp::Bank,
        )
        .expect("source item")
        .expect("valid bank destination");
    assert_eq!(
        precommit_plan.moved_destination,
        Some((INVENTORY_SLOT_BAG_0, wow_entities::BANK_SLOT_ITEM_START, 1))
    );

    session
        .handle_autobank_item(AutoBankItem {
            inv_update: InvUpdate {
                items: vec![(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START)],
            },
            bag: INVENTORY_SLOT_BAG_0,
            slot: INVENTORY_SLOT_ITEM_START,
        })
        .await;

    assert_eq!(
        session
            .get_inventory_item_by_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START)
            .map(|item| item.guid),
        Some(source_guid),
        "a failed SQL commit must not expose the planned bank location"
    );
    assert!(
        session
            .get_inventory_item_by_pos(INVENTORY_SLOT_BAG_0, wow_entities::BANK_SLOT_ITEM_START,)
            .is_none()
    );
    assert_eq!(session.represented_non_bank_item_count_like_cpp(708), 1);
    assert!(session.represented_bank_item_moves_like_cpp().is_empty());

    let error = send_rx
        .try_recv()
        .expect("commit failure should send an equipment error");
    assert_eq!(
        u16::from_le_bytes([error[0], error[1]]),
        wow_constants::ServerOpcodes::InventoryChangeFailure as u16,
    );
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn autostore_missing_bank_item_does_not_record_unapplied_move_like_cpp() {
    let (mut session, send_rx, canonical) = make_bank_slot_session(4);
    let banker = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 2456, 41);
    insert_banker_creature(&canonical, banker, NPCFlags1::BANKER.bits());

    session.handle_banker_activate(Hello { unit: banker }).await;
    assert!(send_rx.try_recv().is_ok(), "bank open should be sent");

    session
        .handle_autostore_bank_item(AutoStoreBankItem {
            inv_update: InvUpdate {
                items: vec![(255, 39)],
            },
            bag: 255,
            slot: 39,
        })
        .await;

    assert!(session.represented_bank_item_moves_like_cpp().is_empty());
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn auto_bank_item_rejects_without_current_bank_like_cpp() {
    let (mut session, send_rx, _canonical) = make_bank_slot_session(1);

    session
        .handle_autobank_item(AutoBankItem {
            inv_update: InvUpdate {
                items: vec![(255, 19)],
            },
            bag: 255,
            slot: 19,
        })
        .await;
    session
        .handle_autostore_bank_item(AutoStoreBankItem {
            inv_update: InvUpdate {
                items: vec![(255, 39)],
            },
            bag: 255,
            slot: 39,
        })
        .await;

    assert!(session.represented_bank_item_moves_like_cpp().is_empty());
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn change_bank_bag_slot_flag_toggles_flag_after_banker_activation_like_cpp() {
    let (mut session, send_rx, canonical) = make_bank_slot_session(4);
    let banker = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 2456, 4);
    insert_banker_creature(&canonical, banker, NPCFlags1::BANKER.bits());

    session.handle_banker_activate(Hello { unit: banker }).await;
    assert!(send_rx.try_recv().is_ok(), "bank open should be sent");

    session
        .handle_change_bank_bag_slot_flag(ChangeBankBagSlotFlag {
            slot: 2,
            flag: 4,
            enabled: true,
        })
        .await;

    assert_eq!(session.represented_bank_bag_slot_flag_like_cpp(2), Some(16));
    assert!(send_rx.try_recv().is_ok(), "flag update should be sent");

    session
        .handle_change_bank_bag_slot_flag(ChangeBankBagSlotFlag {
            slot: 2,
            flag: 4,
            enabled: false,
        })
        .await;

    assert_eq!(session.represented_bank_bag_slot_flag_like_cpp(2), Some(0));
    assert!(
        send_rx.try_recv().is_ok(),
        "flag clear update should be sent"
    );
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn change_bank_bag_slot_flag_rejects_without_current_bank_like_cpp() {
    let (mut session, send_rx, _canonical) = make_bank_slot_session(1);

    session
        .handle_change_bank_bag_slot_flag(ChangeBankBagSlotFlag {
            slot: 2,
            flag: 4,
            enabled: true,
        })
        .await;

    assert_eq!(session.represented_bank_bag_slot_flag_like_cpp(2), Some(0));
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn change_bank_bag_slot_flag_rejects_invalid_slot_like_cpp() {
    let (mut session, send_rx, canonical) = make_bank_slot_session(2);
    let banker = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 2456, 5);
    insert_banker_creature(&canonical, banker, NPCFlags1::BANKER.bits());
    session.handle_banker_activate(Hello { unit: banker }).await;
    assert!(send_rx.try_recv().is_ok(), "bank open should be sent");

    session
        .handle_change_bank_bag_slot_flag(ChangeBankBagSlotFlag {
            slot: 7,
            flag: 4,
            enabled: true,
        })
        .await;

    assert!(send_rx.try_recv().is_err());
    assert_eq!(session.represented_bank_bag_slot_flag_like_cpp(6), Some(0));
}

#[tokio::test]
async fn opening_cinematic_requires_zero_xp_and_prefers_class_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(4);
    session.set_loaded_player_identity_like_cpp(571, 1, 8, 1, 0);
    session.set_player_xp_like_cpp(1);
    session.set_chr_classes_store(Arc::new(ChrClassesStore::from_entries([chr_class_entry(
        8, 111,
    )])));
    session.set_chr_races_store(Arc::new(ChrRacesStore::from_entries([chr_race_entry(
        1, 222,
    )])));

    session
        .handle_opening_cinematic(WorldPacket::new_empty())
        .await;
    assert!(send_rx.try_recv().is_err());

    session.set_player_xp_like_cpp(0);
    session
        .handle_opening_cinematic(WorldPacket::new_empty())
        .await;
    assert_eq!(send_rx.try_recv().unwrap(), expected_trigger_cinematic(111));

    let (mut fallback, fallback_rx) = make_session_with_send_capacity(4);
    fallback.set_loaded_player_identity_like_cpp(571, 1, 8, 1, 0);
    fallback.set_player_xp_like_cpp(0);
    fallback.set_chr_classes_store(Arc::new(ChrClassesStore::from_entries([chr_class_entry(
        8, 0,
    )])));
    fallback.set_chr_races_store(Arc::new(ChrRacesStore::from_entries([chr_race_entry(
        1, 222,
    )])));

    fallback
        .handle_opening_cinematic(WorldPacket::new_empty())
        .await;
    assert_eq!(
        fallback_rx.try_recv().unwrap(),
        expected_trigger_cinematic(222)
    );
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

#[tokio::test]
async fn gossip_catalog_port_preserves_read_order_and_localized_projection_like_cpp() {
    let (mut session, _) = make_quest_status_session();
    session.locale = "esES".to_owned();
    let menu_id = 700;
    let npc_guid = creature_guid(9001, 701);
    let port = GossipCatalogPortFixtureLikeCpp::new(
        [GossipCatalogReadOutcomeLikeCpp::Found(menu_id)],
        [GossipCatalogReadOutcomeLikeCpp::Found(vec![10, 20])],
        [GossipCatalogReadOutcomeLikeCpp::Found(900)],
        [GossipCatalogReadOutcomeLikeCpp::Found(vec![
            gossip_catalog_option_like_cpp(menu_id, 2, 901),
        ])],
        [GossipCatalogReadOutcomeLikeCpp::Found(
            "Opción localizada".to_owned(),
        )],
    );
    session.set_gossip_catalog_persistence_port_like_cpp(port.clone());

    let message = session
        .build_gossip_menu(9001, 0, npc_guid)
        .await
        .expect("typed catalog produces gossip message");

    assert_eq!(message.gossip_id, menu_id as i32);
    assert_eq!(message.broadcast_text_id, Some(900));
    assert_eq!(message.gossip_options.len(), 1);
    assert_eq!(message.gossip_options[0].text, "Opción localizada");
    assert_eq!(message.gossip_options[0].gossip_option_id, 77);
    assert_eq!(session.gossip_options.len(), 1);
    assert_eq!(
        port.requests(),
        vec![
            GossipCatalogRequestTraceLikeCpp::CreatureMenu(GossipCreatureMenuRequestLikeCpp {
                creature_entry: 9001,
            }),
            GossipCatalogRequestTraceLikeCpp::MenuTexts(GossipMenuCatalogRequestLikeCpp {
                menu_id,
            }),
            GossipCatalogRequestTraceLikeCpp::NpcText(GossipNpcTextCatalogRequestLikeCpp {
                npc_text_id: 20,
            }),
            GossipCatalogRequestTraceLikeCpp::MenuOptions(GossipMenuCatalogRequestLikeCpp {
                menu_id,
            }),
            GossipCatalogRequestTraceLikeCpp::BroadcastLocale(
                GossipBroadcastTextLocaleRequestLikeCpp {
                    broadcast_text_id: 901,
                    locale: "esES".to_owned(),
                },
            ),
        ]
    );
}

#[tokio::test]
async fn gossip_catalog_required_read_failure_stops_before_locale_like_cpp() {
    let (mut session, _) = make_quest_status_session();
    session.locale = "esES".to_owned();
    let port = GossipCatalogPortFixtureLikeCpp::new(
        [GossipCatalogReadOutcomeLikeCpp::Found(701)],
        [GossipCatalogReadOutcomeLikeCpp::Found(vec![21])],
        [GossipCatalogReadOutcomeLikeCpp::Missing],
        [GossipCatalogReadOutcomeLikeCpp::Failed {
            reason: "options unavailable".to_owned(),
        }],
        [],
    );
    session.set_gossip_catalog_persistence_port_like_cpp(port.clone());

    assert!(
        session
            .build_gossip_menu(9002, 0, creature_guid(9002, 702))
            .await
            .is_none()
    );
    assert_eq!(
        port.requests(),
        vec![
            GossipCatalogRequestTraceLikeCpp::CreatureMenu(GossipCreatureMenuRequestLikeCpp {
                creature_entry: 9002,
            }),
            GossipCatalogRequestTraceLikeCpp::MenuTexts(GossipMenuCatalogRequestLikeCpp {
                menu_id: 701,
            }),
            GossipCatalogRequestTraceLikeCpp::NpcText(GossipNpcTextCatalogRequestLikeCpp {
                npc_text_id: 21,
            }),
            GossipCatalogRequestTraceLikeCpp::MenuOptions(GossipMenuCatalogRequestLikeCpp {
                menu_id: 701,
            }),
        ]
    );
}

#[tokio::test]
async fn gossip_catalog_optional_reads_fail_to_existing_fallbacks_like_cpp() {
    let (mut session, _) = make_quest_status_session();
    session.locale = "esES".to_owned();
    let menu_id = 702;
    let port = GossipCatalogPortFixtureLikeCpp::new(
        [GossipCatalogReadOutcomeLikeCpp::Found(menu_id)],
        [GossipCatalogReadOutcomeLikeCpp::Missing],
        [GossipCatalogReadOutcomeLikeCpp::Failed {
            reason: "npc text unavailable".to_owned(),
        }],
        [GossipCatalogReadOutcomeLikeCpp::Found(vec![
            gossip_catalog_option_like_cpp(menu_id, 3, 902),
        ])],
        [GossipCatalogReadOutcomeLikeCpp::Failed {
            reason: "locale unavailable".to_owned(),
        }],
    );
    session.set_gossip_catalog_persistence_port_like_cpp(port);

    let message = session
        .build_gossip_menu(9003, 0, creature_guid(9003, 703))
        .await
        .expect("optional catalog failures retain the menu");
    assert_eq!(message.broadcast_text_id, None);
    assert_eq!(message.gossip_options[0].text, "Original option");
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

#[test]
fn start_positions_are_valid() {
    for race in [1, 2, 3, 4, 5, 6, 7, 8, 10, 11, 22] {
        let (map, x, y, z, _o) = start_position(race);
        assert!(map >= 0, "Race {race} has invalid map");
        // Positions should be non-zero (except possibly orientation)
        assert!(
            x != 0.0 || y != 0.0 || z != 0.0,
            "Race {race} has zero position"
        );
    }
}

#[test]
fn display_ids_are_valid() {
    for race in [1, 2, 3, 4, 5, 6, 7, 8, 10, 11] {
        for sex in [0u8, 1] {
            let id = default_display_id(race, sex);
            assert!(id > 0, "Race {race} sex {sex} has zero display ID");
        }
    }
}

#[tokio::test]
async fn gossip_hello_questgiver_without_db_gossip_menu_opens_quest_like_cpp() {
    let (mut session, send_rx) = make_quest_status_session();
    let entry = 9306;
    let guid = creature_guid(entry, 306);
    let mut store = store_with_quests(&[3006]);
    store.starter_quests.entry(entry).or_default().push(3006);
    session.set_quest_store(Arc::new(store));
    attach_legacy_creature(
        &mut session,
        guid,
        entry,
        NPCFlags1::GOSSIP.bits() | NPCFlags1::QUEST_GIVER.bits(),
    );

    session.handle_gossip_hello(Hello { unit: guid }).await;

    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::QuestGiverQuestDetails]
    );
}

#[tokio::test]
async fn invalid_gossip_hello_preserves_active_player_menu_state_like_cpp() {
    let (mut session, send_rx) = make_quest_status_session();
    let active_source = creature_guid(9306, 305);
    let invalid_source = creature_guid(9306, 999);
    session.set_player_trainer_interaction_like_cpp(active_source, 77);
    session
        .gossip_options
        .push(crate::session::GossipOptionInfo {
            gossip_option_id: 31,
            menu_id: 32,
            order_index: 33,
            option_npc: 6,
            action_menu_id: 0,
        });

    session
        .handle_gossip_hello(Hello {
            unit: invalid_source,
        })
        .await;

    assert!(
        send_rx.try_recv().is_err(),
        "C++ returns before publishing when GetNPCIfCanInteractWith rejects the source"
    );
    assert!(
        session.player_trainer_interaction_matches_like_cpp(active_source, 77),
        "invalid hello must not replace InteractionData"
    );
    assert_eq!(
        session.gossip_options.len(),
        1,
        "C++ clears PlayerMenu only after validating the source"
    );
}

#[tokio::test]
async fn valid_direct_service_hello_replaces_stale_trainer_provenance_like_cpp() {
    const FEIGN_SLOT: u8 = 24;
    let (mut session, send_rx, canonical) = make_bank_slot_session(4);
    insert_bank_test_player_in_world(&session, &canonical);
    let old_trainer = creature_guid(9306, 304);
    let vendor = creature_guid(2456, 305);
    insert_banker_creature(
        &canonical,
        vendor,
        NPCFlags1::GOSSIP.bits() | NPCFlags1::VENDOR.bits(),
    );
    session.set_player_trainer_interaction_like_cpp(old_trainer, 77);
    session
        .gossip_options
        .push(crate::session::GossipOptionInfo {
            gossip_option_id: 21,
            menu_id: 22,
            order_index: 23,
            option_npc: GOSSIP_OPTION_NPC_TRAINER_LIKE_CPP,
            action_menu_id: 0,
        });
    seed_represented_feign_death_like_cpp(&mut session, FEIGN_SLOT);

    session.handle_gossip_hello(Hello { unit: vendor }).await;

    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::AuraUpdate],
        "C++ removes fake death before dispatching the selected direct service"
    );
    assert!(
        !session.visible_auras.contains_key(&FEIGN_SLOT),
        "the direct-service shortcut represents a successful C++ gossip selection"
    );
    assert!(!canonical_player_has_died_state_like_cpp(&mut session));
    assert_eq!(
        session.player_interaction_source_guid_like_cpp(),
        Some(vendor),
        "Rust's direct-service shortcut must preserve C++ SendGossipMenu source ownership"
    );
    assert_eq!(
        session.player_interaction_trainer_id_like_cpp(),
        0,
        "opening another valid service invalidates an earlier trainer window"
    );
    assert!(session.gossip_options.is_empty());
}

#[tokio::test]
async fn gossip_hello_mixed_direct_service_keeps_service_like_cpp() {
    let (mut session, send_rx) = make_quest_status_session();
    let entry = 9309;
    let guid = creature_guid(entry, 309);
    let stale_source = creature_guid(entry, 999);
    let mut store = store_with_quests(&[3009]);
    store.starter_quests.entry(entry).or_default().push(3009);
    session.set_quest_store(Arc::new(store));
    session.set_player_trainer_interaction_like_cpp(stale_source, 77);
    session
        .gossip_options
        .push(crate::session::GossipOptionInfo {
            gossip_option_id: 91,
            menu_id: 92,
            order_index: 93,
            option_npc: 94,
            action_menu_id: 95,
        });
    attach_legacy_creature(
        &mut session,
        guid,
        entry,
        NPCFlags1::GOSSIP.bits() | NPCFlags1::QUEST_GIVER.bits() | NPCFlags1::BANKER.bits(),
    );

    session.handle_gossip_hello(Hello { unit: guid }).await;

    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::NpcInteractionOpenResult]
    );
    assert_eq!(
        session.player_interaction_source_guid_like_cpp(),
        Some(guid)
    );
    assert_eq!(session.player_interaction_trainer_id_like_cpp(), 0);
    assert!(
        session.gossip_options.is_empty(),
        "C++ HandleGossipHelloOpcode clears the prior menu before opening a direct service"
    );
}

#[tokio::test]
async fn gossip_hello_canonical_only_direct_fallback_uses_resolved_flags_like_cpp() {
    let (mut session, send_rx) = make_quest_status_session();
    let entry = 9311;
    let guid = creature_guid(entry, 311);
    let mut manager = wow_map::MapManager::default();
    insert_canonical_creature_with_npc_flags(
        &mut manager,
        guid,
        entry,
        NPCFlags1::GOSSIP.bits() | NPCFlags1::BANKER.bits(),
    );
    attach_map_manager(&mut session, manager);

    session.handle_gossip_hello(Hello { unit: guid }).await;

    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::NpcInteractionOpenResult],
        "C++-resolved canonical NPC flags must drive fallback interactions when the legacy mirror has no creature"
    );
    assert_eq!(
        session.player_interaction_source_guid_like_cpp(),
        Some(guid)
    );
    assert_eq!(session.player_interaction_trainer_id_like_cpp(), 0);
}

#[tokio::test]
async fn gossip_hello_questgiver_without_quest_relation_keeps_empty_gossip_fallback() {
    let (mut session, send_rx) = make_quest_status_session();
    let entry = 9307;
    let guid = creature_guid(entry, 307);
    session.set_quest_store(Arc::new(store_with_quests(&[3007])));
    attach_legacy_creature(
        &mut session,
        guid,
        entry,
        NPCFlags1::GOSSIP.bits() | NPCFlags1::QUEST_GIVER.bits(),
    );

    session.handle_gossip_hello(Hello { unit: guid }).await;

    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::GossipMessage]
    );
}

#[tokio::test]
async fn gossip_hello_hostile_questgiver_fallback_is_rejected_like_cpp() {
    let (mut session, send_rx) = make_quest_status_session();
    let entry = 9310;
    let guid = creature_guid(entry, 310);
    let mut store = store_with_quests(&[3010]);
    store.starter_quests.entry(entry).or_default().push(3010);
    session.set_quest_store(Arc::new(store));
    session.set_player_faction_template_like_cpp(1);
    session.set_faction_template_store(Arc::new(
        wow_data::progression_rewards::FactionTemplateStore::from_entries([
            faction_template_entry(35, 35, 0, 0, 1),
            faction_template_entry(1, 1, 0, 0, 0),
        ]),
    ));
    attach_legacy_creature(
        &mut session,
        guid,
        entry,
        NPCFlags1::GOSSIP.bits() | NPCFlags1::QUEST_GIVER.bits(),
    );

    session.handle_gossip_hello(Hello { unit: guid }).await;

    assert!(
        send_rx.try_recv().is_err(),
        "C++ HandleGossipHelloOpcode returns when GetNPCIfCanInteractWith rejects a hostile questgiver"
    );
}

#[tokio::test]
async fn gossip_hello_trainer_fallback_uses_canonical_access_like_cpp() {
    let (mut session, send_rx) = make_quest_status_session();
    let entry = 15_513;
    let guid = creature_guid(entry, 515);
    let mut manager = wow_map::MapManager::default();
    insert_canonical_creature_with_npc_flags(
        &mut manager,
        guid,
        entry,
        NPCFlags1::TRAINER.bits() | NPCFlags1::TRAINER_CLASS.bits(),
    );
    attach_map_manager(&mut session, manager);

    session.handle_gossip_hello(Hello { unit: guid }).await;

    let bytes = send_rx.try_recv().expect("canonical trainer gossip menu");
    assert_eq!(gossip_message_counts(&bytes, guid), (1, 0));
    assert_eq!(
        session.player_interaction_source_guid_like_cpp(),
        Some(guid)
    );
    assert_eq!(session.player_interaction_trainer_id_like_cpp(), 0);
    assert_eq!(
        session.gossip_options[0].option_npc,
        GOSSIP_OPTION_NPC_TRAINER_LIKE_CPP
    );
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn gossip_hello_trainer_fallback_rejects_canonical_player_out_of_world_like_cpp() {
    let (mut session, send_rx) = make_quest_status_session();
    let player_guid = session.player_guid().unwrap();
    let entry = 15_513;
    let guid = creature_guid(entry, 514);
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
        .relocate(Position::new(10.0, 0.0, 0.0, 0.0));

    let mut manager = wow_map::MapManager::default();
    manager
        .create_world_map(571, 0)
        .map_mut()
        .insert_map_object_record(wow_entities::MapObjectRecord::new_player(player).unwrap())
        .unwrap();
    attach_map_manager(&mut session, manager);
    attach_legacy_creature(&mut session, guid, entry, NPCFlags1::TRAINER.bits());

    session.handle_gossip_hello(Hello { unit: guid }).await;

    assert!(
        send_rx.try_recv().is_err(),
        "C++ HandleGossipHelloOpcode gates the trainer fallback through GetNPCIfCanInteractWith"
    );
}

#[tokio::test]
async fn quest_giver_hello_trainer_questgiver_sends_mixed_gossip_like_cpp() {
    let (mut session, send_rx) = make_quest_status_session();
    let entry = 15_513;
    let guid = creature_guid(entry, 513);
    let mut store = store_with_quests(&[9_393]);
    store.starter_quests.entry(entry).or_default().push(9_393);
    session.set_quest_store(Arc::new(store));
    attach_legacy_creature(
        &mut session,
        guid,
        entry,
        NPCFlags1::GOSSIP.bits()
            | NPCFlags1::QUEST_GIVER.bits()
            | NPCFlags1::TRAINER.bits()
            | NPCFlags1::TRAINER_CLASS.bits(),
    );

    session
        .handle_quest_giver_hello(quest_giver_hello_packet(guid))
        .await;

    let bytes = send_rx.try_recv().expect("mixed prepared gossip menu");
    assert_eq!(gossip_message_counts(&bytes, guid), (1, 1));
    assert_eq!(
        session.player_interaction_source_guid_like_cpp(),
        Some(guid)
    );
    assert_eq!(session.player_interaction_trainer_id_like_cpp(), 0);
    assert_eq!(session.gossip_options.len(), 1);
    assert_eq!(
        session.gossip_options[0].gossip_option_id,
        GOSSIP_OPTION_ID_AUTO_TRAINER_LIKE_CPP
    );
    assert_eq!(
        session.gossip_options[0].option_npc,
        GOSSIP_OPTION_NPC_TRAINER_LIKE_CPP
    );
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn gossip_select_trainer_only_source_opens_resolved_trainer_without_close_like_cpp() {
    const TRAINER_ID: u32 = 77;
    const FEIGN_SLOT: u8 = 21;
    let (mut session, send_rx, canonical) = make_bank_slot_session(4);
    insert_bank_test_player_in_world(&session, &canonical);
    let guid = creature_guid(2_456, 513);
    insert_banker_creature(&canonical, guid, NPCFlags1::TRAINER.bits());
    session.set_trainer_store_like_cpp(Arc::new(
        wow_data::TrainerStoreLikeCpp::from_rows_like_cpp(
            [wow_data::TrainerRowLikeCpp {
                id: TRAINER_ID,
                trainer_type: wow_data::TRAINER_TYPE_TRADESKILL_LIKE_CPP,
                greeting: "Train".to_string(),
            }],
            [],
            [],
            [wow_data::CreatureTrainerRowLikeCpp {
                creature_id: 2_456,
                trainer_id: TRAINER_ID,
                menu_id: 0,
                option_id: 0,
            }],
            |_| true,
            |_| true,
            |_| true,
            |_, _| true,
        )
        .store,
    ));

    session.handle_gossip_hello(Hello { unit: guid }).await;

    let menu_packet = send_rx.try_recv().expect("generated trainer-only menu");
    assert_eq!(
        WorldPacket::from_bytes(&menu_packet).server_opcode(),
        Some(ServerOpcodes::GossipMessage)
    );
    assert_eq!(gossip_message_counts(&menu_packet, guid), (1, 0));
    let option = session
        .gossip_options
        .first()
        .cloned()
        .expect("generated trainer option");
    assert_eq!(option.menu_id, 0);
    assert_eq!(option.order_index, 0);
    assert_eq!(
        option.gossip_option_id,
        GOSSIP_OPTION_ID_AUTO_TRAINER_LIKE_CPP
    );
    assert_eq!(option.option_npc, GOSSIP_OPTION_NPC_TRAINER_LIKE_CPP);
    assert_eq!(
        session.player_interaction_source_guid_like_cpp(),
        Some(guid)
    );
    assert_eq!(session.player_interaction_trainer_id_like_cpp(), 0);

    seed_represented_feign_death_like_cpp(&mut session, FEIGN_SLOT);

    session
        .handle_gossip_select_option(wow_packet::packets::gossip::GossipSelectOption {
            gossip_unit: guid,
            gossip_id: option.menu_id as i32,
            gossip_option_id: option.gossip_option_id,
            promotion_code: String::new(),
        })
        .await;

    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::AuraUpdate, ServerOpcodes::TrainerList],
        "the target fork's trainer-only generated option must be usable and must not pre-send GossipComplete"
    );
    assert!(session.player_trainer_interaction_matches_like_cpp(guid, TRAINER_ID as i32));
    assert!(!session.visible_auras.contains_key(&FEIGN_SLOT));
    assert!(!canonical_player_has_died_state_like_cpp(&mut session));
}

#[tokio::test]
async fn gossip_select_requires_exact_active_source_and_routes_exact_match_like_cpp() {
    let requested = creature_guid(15_513, 520);
    let other = creature_guid(15_513, 521);
    for (active_source, expected_opcodes) in [
        (None, Vec::new()),
        (Some(other), Vec::new()),
        (
            Some(requested),
            vec![ServerOpcodes::NpcInteractionOpenResult],
        ),
    ] {
        let (mut session, send_rx) = make_quest_status_session();
        attach_legacy_creature(
            &mut session,
            requested,
            15_513,
            NPCFlags1::GOSSIP.bits() | NPCFlags1::BANKER.bits(),
        );
        if let Some(active_source) = active_source {
            session.set_player_trainer_interaction_like_cpp(active_source, 77);
        }
        session
            .gossip_options
            .push(crate::session::GossipOptionInfo {
                gossip_option_id: 41,
                menu_id: 42,
                order_index: 43,
                option_npc: 6, // Banker gives an observable packet if routed.
                action_menu_id: 0,
            });

        session
            .handle_gossip_select_option(wow_packet::packets::gossip::GossipSelectOption {
                gossip_unit: requested,
                gossip_id: 42,
                gossip_option_id: 41,
                promotion_code: String::new(),
            })
            .await;

        assert_eq!(drain_server_opcodes(&send_rx), expected_opcodes);
        assert_eq!(session.gossip_options.len(), 1);
        if active_source == Some(requested) {
            assert_eq!(
                session.player_interaction_source_guid_like_cpp(),
                Some(requested)
            );
            assert_eq!(session.player_interaction_trainer_id_like_cpp(), 0);
        } else {
            assert_eq!(
                session.player_interaction_source_guid_like_cpp(),
                active_source
            );
            assert_eq!(
                session.player_interaction_trainer_id_like_cpp(),
                if active_source.is_some() { 77 } else { 0 }
            );
        }
    }
}

#[tokio::test]
async fn gossip_select_requires_exact_active_menu_id_like_cpp() {
    const FEIGN_SLOT: u8 = 22;
    let (mut session, send_rx, canonical) = make_bank_slot_session(2);
    insert_bank_test_player_in_world(&session, &canonical);
    let banker = creature_guid(15_513, 523);
    insert_banker_creature(
        &canonical,
        banker,
        NPCFlags1::GOSSIP.bits() | NPCFlags1::BANKER.bits(),
    );
    session.set_player_trainer_interaction_like_cpp(banker, 77);
    session
        .gossip_options
        .push(crate::session::GossipOptionInfo {
            gossip_option_id: 61,
            menu_id: 62,
            order_index: 63,
            option_npc: 6,
            action_menu_id: 0,
        });
    seed_represented_feign_death_like_cpp(&mut session, FEIGN_SLOT);

    session
        .handle_gossip_select_option(wow_packet::packets::gossip::GossipSelectOption {
            gossip_unit: banker,
            gossip_id: 999,
            gossip_option_id: 61,
            promotion_code: String::new(),
        })
        .await;

    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::AuraUpdate],
        "C++ removes fake death before Player::OnGossipSelect rejects a mismatched GossipID"
    );
    assert!(
        session.player_trainer_interaction_matches_like_cpp(banker, 77),
        "a mismatched packet GossipID must not route or replace InteractionData"
    );
    assert_eq!(session.gossip_options.len(), 1);
    assert!(!session.visible_auras.contains_key(&FEIGN_SLOT));
    assert!(!canonical_player_has_died_state_like_cpp(&mut session));
}

#[tokio::test]
async fn gossip_select_accepts_represented_goober_menu_and_removes_feign_like_cpp() {
    const FEIGN_SLOT: u8 = 19;
    const GOSSIP_ID: u32 = 72;
    let (mut session, send_rx, canonical) = make_bank_slot_session(2);
    insert_bank_test_player_in_world(&session, &canonical);
    let goober = gameobject_guid(9304, 304);
    insert_gossip_gameobject(
        &canonical,
        goober,
        9304,
        Position::new(1.0, 0.0, 0.0, 0.0),
        GAMEOBJECT_TYPE_GOOBER as u8,
        true,
    );
    session.represented_gameobject_use_states.insert(
        goober,
        RepresentedGameObjectUseState {
            map_id: Some(571),
            position: Some(Position::new(1.0, 0.0, 0.0, 0.0)),
            go_type: Some(GAMEOBJECT_TYPE_GOOBER as u8),
            icon_name_allows_interaction_like_cpp: Some(true),
            ..Default::default()
        },
    );
    session.set_player_interaction_source_like_cpp(goober);
    session
        .gossip_options
        .push(crate::session::GossipOptionInfo {
            gossip_option_id: 71,
            menu_id: GOSSIP_ID,
            order_index: 0,
            option_npc: 0,
            action_menu_id: 0,
        });
    seed_represented_feign_death_like_cpp(&mut session, FEIGN_SLOT);

    session
        .handle_gossip_select_option(wow_packet::packets::gossip::GossipSelectOption {
            gossip_unit: goober,
            gossip_id: GOSSIP_ID as i32,
            gossip_option_id: 71,
            promotion_code: String::new(),
        })
        .await;

    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::AuraUpdate],
        "a valid GOOBER follows the C++ generic GameObject interaction path"
    );
    assert!(!session.visible_auras.contains_key(&FEIGN_SLOT));
    assert!(!canonical_player_has_died_state_like_cpp(&mut session));
}

#[tokio::test]
async fn gossip_select_gameobject_revalidates_cpp_interaction_boundaries() {
    const FEIGN_SLOT: u8 = 20;
    const GOSSIP_ID: u32 = 82;
    for (case, go_type, recorded_type, position, in_world, icon_allows, in_taxi, same_phase) in [
        (
            "point-icon",
            GAMEOBJECT_TYPE_GOOBER as u8,
            true,
            Position::new(1.0, 0.0, 0.0, 0.0),
            true,
            Some(false),
            false,
            true,
        ),
        (
            "missing-icon-evidence",
            GAMEOBJECT_TYPE_GOOBER as u8,
            true,
            Position::new(1.0, 0.0, 0.0, 0.0),
            true,
            None,
            false,
            true,
        ),
        (
            "missing-runtime-type",
            GAMEOBJECT_TYPE_GOOBER as u8,
            false,
            Position::new(1.0, 0.0, 0.0, 0.0),
            true,
            Some(true),
            false,
            true,
        ),
        (
            "gameobject-not-in-world",
            GAMEOBJECT_TYPE_GOOBER as u8,
            true,
            Position::new(1.0, 0.0, 0.0, 0.0),
            false,
            Some(true),
            false,
            true,
        ),
        (
            "outside-interaction-distance",
            GAMEOBJECT_TYPE_GOOBER as u8,
            true,
            Position::new(50.0, 0.0, 0.0, 0.0),
            true,
            Some(true),
            false,
            true,
        ),
        (
            "player-in-taxi-flight",
            GAMEOBJECT_TYPE_GOOBER as u8,
            true,
            Position::new(1.0, 0.0, 0.0, 0.0),
            true,
            Some(true),
            true,
            true,
        ),
        (
            "incompatible-phase",
            GAMEOBJECT_TYPE_GOOBER as u8,
            true,
            Position::new(1.0, 0.0, 0.0, 0.0),
            true,
            Some(true),
            false,
            false,
        ),
    ] {
        let (mut session, send_rx, canonical) = make_bank_slot_session(2);
        insert_bank_test_player_in_world(&session, &canonical);
        let gameobject = gameobject_guid(9305, 305);
        insert_gossip_gameobject(&canonical, gameobject, 9305, position, go_type, in_world);
        session.represented_gameobject_use_states.insert(
            gameobject,
            RepresentedGameObjectUseState {
                map_id: Some(571),
                position: Some(position),
                go_type: recorded_type.then_some(go_type),
                icon_name_allows_interaction_like_cpp: icon_allows,
                ..Default::default()
            },
        );
        session.set_player_interaction_source_like_cpp(gameobject);
        session
            .gossip_options
            .push(crate::session::GossipOptionInfo {
                gossip_option_id: 81,
                menu_id: GOSSIP_ID,
                order_index: 0,
                option_npc: 0,
                action_menu_id: 0,
            });
        seed_represented_feign_death_like_cpp(&mut session, FEIGN_SLOT);
        if in_taxi {
            session.set_taxi_flight_state_like_cpp(
                RepresentedTaxiFlightNodeLikeCpp {
                    map_id: 571,
                    position: Position::new(1.0, 0.0, 0.0, 0.0),
                    teleport_flag: false,
                },
                None,
            );
        }
        if !same_phase {
            session.set_represented_player_phase_shift_like_cpp(
                wow_entities::PhaseShift::from_phases([10]),
            );
            session.record_represented_gameobject_phase_shift_like_cpp(
                gameobject,
                wow_entities::PhaseShift::from_phases([20]),
            );
        }

        session
            .handle_gossip_select_option(wow_packet::packets::gossip::GossipSelectOption {
                gossip_unit: gameobject,
                gossip_id: GOSSIP_ID as i32,
                gossip_option_id: 81,
                promotion_code: String::new(),
            })
            .await;

        assert!(
            send_rx.try_recv().is_err(),
            "{case}: rejection must precede fake-death removal and action routing"
        );
        assert!(
            session.visible_auras.contains_key(&FEIGN_SLOT),
            "{case}: rejected source must preserve fake death"
        );
        assert!(
            canonical_player_has_died_state_like_cpp(&mut session),
            "{case}: rejected source must preserve DIED state"
        );
        assert_eq!(
            session.player_interaction_source_guid_like_cpp(),
            Some(gameobject)
        );
    }
}

#[tokio::test]
async fn gossip_select_gameobject_rejects_npc_service_option_after_feign_like_cpp() {
    const FEIGN_SLOT: u8 = 23;
    const GOSSIP_ID: u32 = 92;
    let (mut session, send_rx, canonical) = make_bank_slot_session(2);
    insert_bank_test_player_in_world(&session, &canonical);
    let goober = gameobject_guid(9306, 306);
    insert_gossip_gameobject(
        &canonical,
        goober,
        9306,
        Position::new(1.0, 0.0, 0.0, 0.0),
        GAMEOBJECT_TYPE_GOOBER as u8,
        true,
    );
    session.represented_gameobject_use_states.insert(
        goober,
        RepresentedGameObjectUseState {
            map_id: Some(571),
            position: Some(Position::new(1.0, 0.0, 0.0, 0.0)),
            go_type: Some(GAMEOBJECT_TYPE_GOOBER as u8),
            icon_name_allows_interaction_like_cpp: Some(true),
            ..Default::default()
        },
    );
    session.set_player_interaction_source_like_cpp(goober);
    session
        .gossip_options
        .push(crate::session::GossipOptionInfo {
            gossip_option_id: 91,
            menu_id: GOSSIP_ID,
            order_index: 0,
            option_npc: 6, // Banker is invalid for every C++ GameObject menu.
            action_menu_id: 0,
        });
    seed_represented_feign_death_like_cpp(&mut session, FEIGN_SLOT);

    session
        .handle_gossip_select_option(wow_packet::packets::gossip::GossipSelectOption {
            gossip_unit: goober,
            gossip_id: GOSSIP_ID as i32,
            gossip_option_id: 91,
            promotion_code: String::new(),
        })
        .await;

    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::AuraUpdate],
        "C++ revalidates the GO and removes fake death before rejecting its NPC service option"
    );
    assert_eq!(
        session.player_interaction_source_guid_like_cpp(),
        Some(goober)
    );
    assert!(!session.visible_auras.contains_key(&FEIGN_SLOT));
    assert!(!canonical_player_has_died_state_like_cpp(&mut session));
}

#[tokio::test]
async fn gossip_banker_selection_replaces_trainer_provenance_like_cpp() {
    let (mut session, send_rx) = make_quest_status_session();
    let banker = creature_guid(15_513, 522);
    attach_legacy_creature(
        &mut session,
        banker,
        15_513,
        NPCFlags1::GOSSIP.bits() | NPCFlags1::BANKER.bits(),
    );
    session.set_player_trainer_interaction_like_cpp(banker, 77);
    session
        .gossip_options
        .push(crate::session::GossipOptionInfo {
            gossip_option_id: 51,
            menu_id: 52,
            order_index: 53,
            option_npc: 6,
            action_menu_id: 0,
        });

    session
        .handle_gossip_select_option(wow_packet::packets::gossip::GossipSelectOption {
            gossip_unit: banker,
            gossip_id: 52,
            gossip_option_id: 51,
            promotion_code: String::new(),
        })
        .await;

    assert_eq!(
        WorldPacket::from_bytes(&send_rx.try_recv().unwrap()).server_opcode(),
        Some(ServerOpcodes::NpcInteractionOpenResult)
    );
    assert_eq!(
        session.player_interaction_source_guid_like_cpp(),
        Some(banker)
    );
    assert_eq!(session.player_interaction_trainer_id_like_cpp(), 0);
}

#[tokio::test]
async fn quest_giver_hello_plain_questgiver_keeps_direct_quest_open_like_cpp() {
    let (mut session, send_rx) = make_quest_status_session();
    let entry = 9308;
    let guid = creature_guid(entry, 308);
    let mut store = store_with_quests(&[3008]);
    store.starter_quests.entry(entry).or_default().push(3008);
    session.set_quest_store(Arc::new(store));
    attach_legacy_creature(
        &mut session,
        guid,
        entry,
        NPCFlags1::GOSSIP.bits() | NPCFlags1::QUEST_GIVER.bits(),
    );

    session
        .handle_quest_giver_hello(quest_giver_hello_packet(guid))
        .await;

    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::QuestGiverQuestDetails]
    );
}

#[test]
fn gossip_quest_text_filters_race_class_and_level_like_cpp() {
    let (mut session, _send_rx) = make_quest_status_session();
    session.set_loaded_player_identity_like_cpp(571, 10, 3, 3, 0);
    let entry = 15_278;

    let blood_elf_mask = 1u64 << (10 - 1);
    let hunter_mask = 1u32 << (3 - 1);
    let mage_mask = 1u32 << (8 - 1);
    let human_mask = 1u64 << (1 - 1);

    let mut generic = quest_template(8_325);
    generic.allowable_races = blood_elf_mask;
    generic.min_level = 1;
    generic.log_title = "Reclaiming Sunstrider Isle".into();

    let mut hunter = quest_template(9_393);
    hunter.allowable_races = blood_elf_mask;
    hunter.allowable_classes = hunter_mask;
    hunter.min_level = 1;
    hunter.log_title = "Hunter Training".into();

    let mut mage = quest_template(8_328);
    mage.allowable_races = blood_elf_mask;
    mage.allowable_classes = mage_mask;
    mage.min_level = 1;
    mage.log_title = "Mage Training".into();

    let mut too_high = quest_template(99_001);
    too_high.allowable_races = blood_elf_mask;
    too_high.min_level = 4;

    let mut wrong_race = quest_template(99_002);
    wrong_race.allowable_races = human_mask;
    wrong_race.min_level = 1;

    let mut store = QuestStore::from_quests_like_cpp([generic, hunter, mage, too_high, wrong_race]);
    store
        .starter_quests
        .entry(entry)
        .or_default()
        .extend([8_325, 9_393, 8_328, 99_001, 99_002]);
    session.set_quest_store(Arc::new(store));

    let quest_text = session.represented_creature_gossip_text_like_cpp(entry);

    assert_eq!(
        quest_text
            .iter()
            .map(|text| text.quest_id)
            .collect::<Vec<_>>(),
        vec![8_325, 9_393]
    );
    assert!(quest_text.iter().all(|text| text.quest_type == 2));
}

#[test]
fn gossip_quest_text_offers_sallina_followup_after_hunter_training_rewarded_like_cpp() {
    let (mut session, _send_rx) = make_quest_status_session();
    session.set_loaded_player_identity_like_cpp(530, 10, 3, 3, 0);
    let sallina_entry = 15_513;
    let blood_elf_mask = 1u64 << (10 - 1);
    let hunter_mask = 1u32 << (3 - 1);

    let mut hunter_training = quest_template(9_393);
    hunter_training.allowable_races = blood_elf_mask;
    hunter_training.allowable_classes = hunter_mask;
    hunter_training.min_level = 1;
    hunter_training.log_title = "Hunter Training".into();

    let mut followup = quest_template(10_070);
    followup.allowable_races = blood_elf_mask;
    followup.allowable_classes = hunter_mask;
    followup.min_level = 2;
    followup.prev_quest_id = 9_393;
    followup.exclusive_group = 10_068;
    followup.log_title = "Well Watcher Solanian".into();

    let mut store = QuestStore::from_quests_like_cpp([hunter_training, followup]);
    store
        .ender_quests
        .entry(sallina_entry)
        .or_default()
        .push(9_393);
    store
        .starter_quests
        .entry(sallina_entry)
        .or_default()
        .push(10_070);
    session.set_quest_store(Arc::new(store));
    session.rewarded_quests.insert(9_393);

    let quest_text = session.represented_creature_gossip_text_like_cpp(sallina_entry);

    assert_eq!(
        quest_text
            .iter()
            .map(|text| (text.quest_id, text.quest_title.as_str(), text.quest_type))
            .collect::<Vec<_>>(),
        vec![(10_070, "Well Watcher Solanian", 2)]
    );
}

#[tokio::test]
async fn quest_giver_status_tracked_supplied_creature_not_visible_sends_available_like_cpp() {
    let (mut session, send_rx) = make_quest_status_session();
    let mut store = store_with_quests(&[3001]);
    store.starter_quests.entry(9301).or_default().push(3001);
    session.set_quest_store(Arc::new(store));
    let guid = creature_guid(9301, 301);
    let mut manager = wow_map::MapManager::default();
    insert_creature(&mut manager, guid, 9301);
    attach_map_manager(&mut session, manager);
    assert!(!session.client_visible_guids_like_cpp.contains(&guid));

    session
        .handle_quest_giver_status_tracked_query(tracked_query_packet(&[guid]))
        .await;

    assert_eq!(
        recv_status_multiple(&send_rx),
        vec![(guid, quest_giver_status::TRIVIAL)]
    );
}

#[tokio::test]
async fn quest_giver_status_tracked_supplied_gameobject_uses_uint64_status_like_cpp() {
    let (mut session, send_rx) = make_quest_status_session();
    let mut store = store_with_quests(&[3002]);
    assert!(store.insert_gameobject_starter_relation_like_cpp(9302, 3002));
    session.set_quest_store(Arc::new(store));
    let guid = gameobject_guid(9302, 302);
    let mut manager = wow_map::MapManager::default();
    insert_gameobject(&mut manager, guid, 9302);
    attach_map_manager(&mut session, manager);
    mark_gameobject_questgiver(&mut session, guid);

    session
        .handle_quest_giver_status_tracked_query(tracked_query_packet(&[guid]))
        .await;

    assert_eq!(
        recv_status_multiple(&send_rx),
        vec![(guid, quest_giver_status::TRIVIAL)]
    );
}

#[tokio::test]
async fn quest_giver_status_tracked_duplicate_guid_emits_single_status_like_cpp_set() {
    let (mut session, send_rx) = make_quest_status_session();
    let mut store = store_with_quests(&[3003]);
    store.starter_quests.entry(9303).or_default().push(3003);
    session.set_quest_store(Arc::new(store));
    let guid = creature_guid(9303, 303);
    let mut manager = wow_map::MapManager::default();
    insert_creature(&mut manager, guid, 9303);
    attach_map_manager(&mut session, manager);

    session
        .handle_quest_giver_status_tracked_query(tracked_query_packet(&[guid, guid]))
        .await;

    assert_eq!(recv_status_multiple(&send_rx).len(), 1);
}

#[tokio::test]
async fn quest_giver_status_tracked_count_over_cpp_max_sends_no_packet() {
    let (mut session, send_rx) = make_quest_status_session();
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint32(QUEST_GIVER_STATUS_TRACKED_QUERY_MAX_GUIDS_LIKE_CPP + 1);

    session.handle_quest_giver_status_tracked_query(pkt).await;

    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn quest_giver_status_tracked_short_payload_sends_no_packet() {
    let (mut session, send_rx) = make_quest_status_session();
    let guid = creature_guid(9304, 304);
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint32(1);
    pkt.write_packed_guid(&guid);
    let mut bytes = pkt.into_data();
    bytes.pop();

    session
        .handle_quest_giver_status_tracked_query(WorldPacket::from_bytes(&bytes))
        .await;

    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn quest_giver_status_tracked_unsupported_missing_guid_sends_empty_multiple_like_cpp() {
    let (mut session, send_rx) = make_quest_status_session();
    attach_map_manager(&mut session, wow_map::MapManager::default());
    session.set_quest_store(Arc::new(store_with_quests(&[3005])));
    let missing_guid = creature_guid(9305, 305);
    let player_guid = ObjectGuid::create_player(1, 305);
    let item_guid = ObjectGuid::create_item(1, 305);

    session
        .handle_quest_giver_status_tracked_query(tracked_query_packet(&[
            missing_guid,
            player_guid,
            item_guid,
        ]))
        .await;

    assert!(recv_status_multiple(&send_rx).is_empty());
}

#[tokio::test]
async fn tact_key_db_query_bulk_miss_returns_invalid_like_cpp_client_cache_fallback() {
    let (mut session, instance_rx, realm_rx) = make_session_with_realm_send_capacity(1);

    session
        .handle_db_query_bulk(wow_packet::packets::misc::DbQueryBulk {
            table_hash: TACT_KEY_TABLE_HASH_LIKE_CPP,
            queries: vec![3909],
        })
        .await;

    let bytes = realm_rx.try_recv().expect("db reply");
    assert!(instance_rx.try_recv().is_err());
    assert_eq!(
        u16::from_le_bytes([bytes[0], bytes[1]]),
        wow_constants::ServerOpcodes::DbReply as u16
    );
    let mut pkt = WorldPacket::from_bytes(&bytes[2..]);
    assert_eq!(pkt.read_uint32().unwrap(), TACT_KEY_TABLE_HASH_LIKE_CPP);
    assert_eq!(pkt.read_int32().unwrap(), 3909);
    let _timestamp = pkt.read_int32().unwrap();
    assert_eq!(pkt.read_bits(3).unwrap(), 3);
    assert_eq!(pkt.read_uint32().unwrap(), 0);
}

#[tokio::test]
async fn tact_key_db_query_bulk_hit_returns_typed_valid_write_record_like_cpp() {
    let (mut session, instance_rx, realm_rx) = make_session_with_realm_send_capacity(1);
    let key = [0xA5; wow_data::TACTKEY_SIZE];
    session.set_tact_key_store(Arc::new(wow_data::TactKeyStore::from_entries([
        wow_data::TactKeyEntry { id: 3909, key },
    ])));

    session
        .handle_db_query_bulk(wow_packet::packets::misc::DbQueryBulk {
            table_hash: TACT_KEY_TABLE_HASH_LIKE_CPP,
            queries: vec![3909],
        })
        .await;

    let bytes = realm_rx.try_recv().expect("db reply");
    assert!(instance_rx.try_recv().is_err());
    assert_eq!(
        u16::from_le_bytes([bytes[0], bytes[1]]),
        wow_constants::ServerOpcodes::DbReply as u16
    );
    let mut pkt = WorldPacket::from_bytes(&bytes[2..]);
    assert_eq!(pkt.read_uint32().unwrap(), TACT_KEY_TABLE_HASH_LIKE_CPP);
    assert_eq!(pkt.read_int32().unwrap(), 3909);
    let _timestamp = pkt.read_int32().unwrap();
    assert_eq!(pkt.read_bits(3).unwrap(), 1);
    assert_eq!(pkt.read_uint32().unwrap(), wow_data::TACTKEY_SIZE as u32);
    let data = pkt.read_bytes(wow_data::TACTKEY_SIZE).unwrap();
    assert_eq!(data, key);
}

#[tokio::test]
async fn db_query_bulk_raw_blob_cache_is_not_sent_as_typed_cpp_storage() {
    let (mut session, instance_rx, realm_rx) = make_session_with_realm_send_capacity(1);
    let mut cache = wow_data::HotfixBlobCache::new();
    cache.insert_blob(0x919B_E54E, 198647, vec![0xAA; 408]);
    session.set_hotfix_blob_cache(Arc::new(cache));

    session
        .handle_db_query_bulk(wow_packet::packets::misc::DbQueryBulk {
            table_hash: 0x919B_E54E,
            queries: vec![198647],
        })
        .await;

    let bytes = realm_rx.try_recv().expect("db reply");
    assert!(instance_rx.try_recv().is_err());
    assert_eq!(
        u16::from_le_bytes([bytes[0], bytes[1]]),
        wow_constants::ServerOpcodes::DbReply as u16
    );
    let mut pkt = WorldPacket::from_bytes(&bytes[2..]);
    assert_eq!(pkt.read_uint32().unwrap(), 0x919B_E54E);
    assert_eq!(pkt.read_int32().unwrap(), 198647);
    let _timestamp = pkt.read_int32().unwrap();
    assert_eq!(pkt.read_bits(3).unwrap(), 3);
    assert_eq!(pkt.read_uint32().unwrap(), 0);
    assert!(realm_rx.try_recv().is_err());
}

#[tokio::test]
async fn hotfix_request_local_db2_blob_is_not_sent_as_typed_cpp_storage() {
    let (mut session, send_rx) = make_session_with_send_capacity(1);
    let mut cache = wow_data::HotfixBlobCache::new();
    cache.insert_blob(0x919B_E54E, 198647, vec![0xAA; 408]);
    cache.insert_hotfix_record_like_cpp(wow_data::HotfixRecord {
        table_hash: 0x919B_E54E,
        record_id: 198647,
        id: wow_data::HotfixId {
            push_id: 77,
            unique_id: 88,
        },
        status: wow_data::HotfixRecordStatus::Valid,
        available_locales_mask: wow_data::hotfix_locale_mask("esES"),
    });
    session.set_hotfix_blob_cache(Arc::new(cache));

    session
        .handle_hotfix_request(wow_packet::packets::misc::HotfixRequest {
            client_build: 54261,
            data_build: 54261,
            hotfixes: vec![77],
        })
        .await;

    let bytes = send_rx.try_recv().expect("hotfix connect");
    assert_eq!(
        u16::from_le_bytes([bytes[0], bytes[1]]),
        wow_constants::ServerOpcodes::HotfixConnect as u16
    );
    let mut pkt = WorldPacket::from_bytes(&bytes[2..]);
    assert_eq!(pkt.read_uint32().unwrap(), 1);
    assert_eq!(pkt.read_int32().unwrap(), 77);
    assert_eq!(pkt.read_uint32().unwrap(), 88);
    assert_eq!(pkt.read_uint32().unwrap(), 0x919B_E54E);
    assert_eq!(pkt.read_int32().unwrap(), 198647);
    assert_eq!(pkt.read_uint32().unwrap(), 0);
    assert_eq!(pkt.read_bits(3).unwrap(), 3);
    assert_eq!(pkt.read_uint32().unwrap(), 0);
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn hotfix_request_sql_hotfix_blob_keeps_valid_cpp_blob_path() {
    let (mut session, send_rx) = make_session_with_send_capacity(1);
    let mut cache = wow_data::HotfixBlobCache::new();
    cache.insert_hotfix_blob(0xAABB_CCDD, 123, vec![1, 2, 3, 4]);
    cache.insert_hotfix_record_like_cpp(wow_data::HotfixRecord {
        table_hash: 0xAABB_CCDD,
        record_id: 123,
        id: wow_data::HotfixId {
            push_id: 78,
            unique_id: 89,
        },
        status: wow_data::HotfixRecordStatus::Valid,
        available_locales_mask: wow_data::hotfix_locale_mask("esES"),
    });
    session.set_hotfix_blob_cache(Arc::new(cache));

    session
        .handle_hotfix_request(wow_packet::packets::misc::HotfixRequest {
            client_build: 54261,
            data_build: 54261,
            hotfixes: vec![78],
        })
        .await;

    let bytes = send_rx.try_recv().expect("hotfix connect");
    assert_eq!(
        u16::from_le_bytes([bytes[0], bytes[1]]),
        wow_constants::ServerOpcodes::HotfixConnect as u16
    );
    let mut pkt = WorldPacket::from_bytes(&bytes[2..]);
    assert_eq!(pkt.read_uint32().unwrap(), 1);
    assert_eq!(pkt.read_int32().unwrap(), 78);
    assert_eq!(pkt.read_uint32().unwrap(), 89);
    assert_eq!(pkt.read_uint32().unwrap(), 0xAABB_CCDD);
    assert_eq!(pkt.read_int32().unwrap(), 123);
    assert_eq!(pkt.read_uint32().unwrap(), 4);
    assert_eq!(pkt.read_bits(3).unwrap(), 1);
    assert_eq!(pkt.read_uint32().unwrap(), 4);
    assert_eq!(pkt.read_bytes(4).unwrap(), vec![1, 2, 3, 4]);
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn query_page_text_without_catalog_port_sends_cpp_deny_shape() {
    let (mut session, send_rx) = make_session_with_send_capacity(1);

    session
        .handle_query_page_text(QueryPageText {
            page_text_id: 123,
            item_guid: ObjectGuid::EMPTY,
        })
        .await;

    let bytes = send_rx.try_recv().expect("query page text response");
    assert_eq!(
        u16::from_le_bytes([bytes[0], bytes[1]]),
        wow_constants::ServerOpcodes::QueryPageTextResponse as u16
    );
    assert_eq!(&bytes[2..6], &123_u32.to_le_bytes());
    assert_eq!(bytes[6], 0x00);
    assert_eq!(bytes.len(), 7);
}

#[tokio::test]
async fn query_page_text_uses_typed_catalog_and_preserves_exact_chain_packet_like_cpp() {
    let pages = vec![
        PageTextCatalogRowLikeCpp {
            id: 123,
            next_page_id: 124,
            player_condition_id: -7,
            flags: 3,
            text: "Primera página".to_owned(),
        },
        PageTextCatalogRowLikeCpp {
            id: 124,
            next_page_id: 0,
            player_condition_id: 9,
            flags: 5,
            text: "Segunda página".to_owned(),
        },
    ];
    let port = PageTextCatalogPortFixtureLikeCpp::new([PageTextCatalogOutcomeLikeCpp {
        pages: pages.clone(),
        diagnostics: vec![PageTextCatalogDiagnosticLikeCpp::LocaleReadFailed {
            page_text_id: 124,
            locale: "esES".to_owned(),
            reason: "locale fallback diagnostic".to_owned(),
        }],
    }]);
    let (mut session, send_rx) = make_session_with_send_capacity(1);
    session.set_page_text_catalog_persistence_port_like_cpp(port.clone());

    session
        .handle_query_page_text(QueryPageText {
            page_text_id: 123,
            item_guid: ObjectGuid::EMPTY,
        })
        .await;

    assert_eq!(
        port.requests(),
        vec![PageTextCatalogRequestLikeCpp {
            page_text_id: 123,
            locale: "esES".to_owned(),
        }]
    );
    assert_eq!(
        send_rx.try_recv().unwrap(),
        QueryPageTextResponse {
            page_text_id: 123,
            allow: true,
            pages: pages
                .into_iter()
                .map(|page| PageTextInfo {
                    id: page.id,
                    next_page_id: page.next_page_id,
                    player_condition_id: page.player_condition_id,
                    flags: page.flags,
                    text: page.text,
                })
                .collect(),
        }
        .to_bytes()
    );
}

#[tokio::test]
async fn query_page_text_preserves_partial_chain_and_empty_failure_shapes_like_cpp() {
    for outcome in [
        PageTextCatalogOutcomeLikeCpp {
            pages: vec![PageTextCatalogRowLikeCpp {
                id: 123,
                next_page_id: 124,
                player_condition_id: 0,
                flags: 0,
                text: "partial".to_owned(),
            }],
            diagnostics: vec![PageTextCatalogDiagnosticLikeCpp::PageReadFailed {
                page_text_id: 124,
                reason: "base query failed".to_owned(),
            }],
        },
        PageTextCatalogOutcomeLikeCpp {
            pages: Vec::new(),
            diagnostics: vec![PageTextCatalogDiagnosticLikeCpp::PageReadFailed {
                page_text_id: 123,
                reason: "base query failed".to_owned(),
            }],
        },
    ] {
        let expected_pages = outcome.pages.clone();
        let port = PageTextCatalogPortFixtureLikeCpp::new([outcome]);
        let (mut session, send_rx) = make_session_with_send_capacity(1);
        session.set_page_text_catalog_persistence_port_like_cpp(port);

        session
            .handle_query_page_text(QueryPageText {
                page_text_id: 123,
                item_guid: ObjectGuid::EMPTY,
            })
            .await;

        assert_eq!(
            send_rx.try_recv().unwrap(),
            QueryPageTextResponse {
                page_text_id: 123,
                allow: !expected_pages.is_empty(),
                pages: expected_pages
                    .into_iter()
                    .map(|page| PageTextInfo {
                        id: page.id,
                        next_page_id: page.next_page_id,
                        player_condition_id: page.player_condition_id,
                        flags: page.flags,
                        text: page.text,
                    })
                    .collect(),
            }
            .to_bytes()
        );
    }
}

#[tokio::test]
async fn query_player_names_without_port_preserves_failure_order_and_realm_routing() {
    let first = ObjectGuid::create_player(1, 41);
    let second = ObjectGuid::create_player(1, 42);
    let (mut session, instance_rx, realm_rx) = make_session_with_realm_send_capacity(1);

    session
        .handle_query_player_names(QueryPlayerNames {
            players: vec![first, second],
        })
        .await;

    assert!(instance_rx.try_recv().is_err());
    assert_eq!(
        realm_rx.try_recv().unwrap(),
        QueryPlayerNamesResponse {
            players: vec![
                NameCacheLookupResult {
                    player: first,
                    result: 1,
                    data: None,
                },
                NameCacheLookupResult {
                    player: second,
                    result: 1,
                    data: None,
                },
            ],
        }
        .to_bytes()
    );
}

#[tokio::test]
async fn query_player_names_uses_typed_port_and_preserves_exact_mixed_packet_like_cpp() {
    let found = ObjectGuid::create_player(1, 41);
    let missing = ObjectGuid::create_player(1, 42);
    let failed = ObjectGuid::create_player(1, 43);
    let port = PlayerNameQueryPortFixtureLikeCpp::new([
        PlayerNameQueryOutcomeLikeCpp::Found(PlayerNameQueryRowLikeCpp {
            name: "Target".to_owned(),
            race: 10,
            class: 3,
            sex: 1,
            level: 80,
        }),
        PlayerNameQueryOutcomeLikeCpp::Missing,
        PlayerNameQueryOutcomeLikeCpp::Failed {
            reason: "character query failed".to_owned(),
        },
    ]);
    let (mut session, instance_rx, realm_rx) = make_session_with_realm_send_capacity(1);
    session.set_player_name_query_persistence_port_like_cpp(port.clone());

    session
        .handle_query_player_names(QueryPlayerNames {
            players: vec![found, missing, failed],
        })
        .await;

    assert_eq!(
        port.requests(),
        vec![
            PlayerNameQueryRequestLikeCpp {
                player_guid_counter: 41,
            },
            PlayerNameQueryRequestLikeCpp {
                player_guid_counter: 42,
            },
            PlayerNameQueryRequestLikeCpp {
                player_guid_counter: 43,
            },
        ]
    );
    assert!(instance_rx.try_recv().is_err());

    let account_id = ObjectGuid::new((HighGuid::WowAccount as i64) << 58, 1);
    let bnet_account_id = ObjectGuid::new((HighGuid::BNetAccount as i64) << 58, 1);
    assert_eq!(
        realm_rx.try_recv().unwrap(),
        QueryPlayerNamesResponse {
            players: vec![
                NameCacheLookupResult {
                    player: found,
                    result: 0,
                    data: Some(PlayerGuidLookupData {
                        name: "Target".to_owned(),
                        race: 10,
                        sex: 1,
                        class: 3,
                        level: 80,
                        guid_actual: found,
                        account_id,
                        bnet_account_id,
                        virtual_realm_address: session.virtual_realm_address(),
                        ..Default::default()
                    }),
                },
                NameCacheLookupResult {
                    player: missing,
                    result: 1,
                    data: None,
                },
                NameCacheLookupResult {
                    player: failed,
                    result: 1,
                    data: None,
                },
            ],
        }
        .to_bytes()
    );
}

#[tokio::test]
async fn item_text_query_missing_item_sends_cpp_invalid_shape() {
    let (mut session, send_rx) = make_session_with_send_capacity(1);
    let item_guid = ObjectGuid::create_world_object(HighGuid::Item, 0, 1, 0, 0, 700, 1);

    session
        .handle_item_text_query(ItemTextQuery { id: item_guid })
        .await;

    let bytes = send_rx.try_recv().expect("item text response");
    assert_eq!(
        u16::from_le_bytes([bytes[0], bytes[1]]),
        wow_constants::ServerOpcodes::QueryItemTextResponse as u16
    );
    assert_eq!(bytes[2], 0x00);
    assert_eq!(bytes[3], 0x00);
    assert_eq!(bytes[4], 0x00);
    assert_eq!(&bytes[5..21], &item_guid.to_raw_bytes());
    assert_eq!(bytes.len(), 21);
}

#[tokio::test]
async fn item_text_query_inventory_item_sends_text_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(1);
    let owner_guid = ObjectGuid::create_player(1, 700);
    let item_guid = ObjectGuid::create_world_object(HighGuid::Item, 0, 1, 0, 0, 700, 2);
    let mut item =
        session.make_inventory_item_object(item_guid, 8000, owner_guid, 1, 0, ItemContext::None, 0);
    item.set_text("abc");
    session.insert_inventory_item_object(item);

    session
        .handle_item_text_query(ItemTextQuery { id: item_guid })
        .await;

    let bytes = send_rx.try_recv().expect("item text response");
    assert_eq!(
        u16::from_le_bytes([bytes[0], bytes[1]]),
        wow_constants::ServerOpcodes::QueryItemTextResponse as u16
    );
    assert_eq!(bytes[2], 0x80);
    assert_eq!(bytes[3], 0x00);
    assert_eq!(bytes[4], 0x18);
    assert_eq!(&bytes[5..8], b"abc");
    assert_eq!(&bytes[8..24], &item_guid.to_raw_bytes());
    assert_eq!(bytes.len(), 24);
}

#[tokio::test]
async fn query_corpse_location_without_runtime_corpse_sends_cpp_invalid_shape() {
    let (mut session, send_rx) = make_session_with_send_capacity(1);
    let player = ObjectGuid::create_player(1, 0xAABB_CCDD);

    session
        .handle_query_corpse_location(QueryCorpseLocationFromClient { player })
        .await;

    let bytes = send_rx.try_recv().expect("corpse location response");
    assert_eq!(
        u16::from_le_bytes([bytes[0], bytes[1]]),
        wow_constants::ServerOpcodes::CorpseLocation as u16
    );
    assert_eq!(bytes[2], 0x00);
    assert_eq!(&bytes[3..19], &player.to_raw_bytes());
    assert_eq!(bytes.len(), 55);
}

#[tokio::test]
async fn query_corpse_transport_without_runtime_corpse_sends_cpp_default_shape() {
    let (mut session, send_rx) = make_session_with_send_capacity(1);
    let player = ObjectGuid::create_player(1, 0xAABB_CCDD);
    let transport = ObjectGuid::create_world_object(HighGuid::Transport, 0, 1, 571, 0, 77, 42);

    session
        .handle_query_corpse_transport(QueryCorpseTransport { player, transport })
        .await;

    let bytes = send_rx.try_recv().expect("corpse transport response");
    assert_eq!(
        u16::from_le_bytes([bytes[0], bytes[1]]),
        wow_constants::ServerOpcodes::CorpseTransportQuery as u16
    );
    assert_eq!(&bytes[2..18], &player.to_raw_bytes());
    assert_eq!(&bytes[18..22], &0.0_f32.to_le_bytes());
    assert_eq!(&bytes[22..26], &0.0_f32.to_le_bytes());
    assert_eq!(&bytes[26..30], &0.0_f32.to_le_bytes());
    assert_eq!(&bytes[30..34], &0.0_f32.to_le_bytes());
    assert_eq!(bytes.len(), 34);
}

#[tokio::test]
async fn query_pet_name_missing_unit_sends_cpp_deny_shape() {
    let (mut session, send_rx) = make_session_with_send_capacity(1);
    let guid = ObjectGuid::create_world_object(HighGuid::Pet, 0, 1, 571, 0, 11, 901);

    session
        .handle_query_pet_name(QueryPetName { unit_guid: guid })
        .await;

    let bytes = send_rx.try_recv().expect("query pet name response");
    assert_eq!(
        u16::from_le_bytes([bytes[0], bytes[1]]),
        wow_constants::ServerOpcodes::QueryPetNameResponse as u16
    );
    assert_eq!(&bytes[2..18], &guid.to_raw_bytes());
    assert_eq!(bytes[18], 0x00);
    assert_eq!(bytes.len(), 19);
}

#[tokio::test]
async fn query_pet_name_uses_canonical_owned_pet_name_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(1);
    let player_guid = ObjectGuid::create_player(1, 904);
    let pet_guid = ObjectGuid::create_world_object(HighGuid::Pet, 0, 1, 571, 0, 21, 904);
    session.set_player_guid(Some(player_guid));

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
        .relocate(Position::new(10.0, 0.0, 0.0, 0.0));

    let mut pet = wow_entities::Pet::new(player_guid, wow_entities::PetType::Hunter);
    pet.creature_mut()
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(pet_guid);
    pet.creature_mut()
        .unit_mut()
        .world_mut()
        .set_map(571, 0)
        .unwrap();
    pet.creature_mut().unit_mut().world_mut().set_name("Misha");

    let mut manager = wow_map::MapManager::default();
    let map = manager.create_world_map(571, 0).map_mut();
    map.insert_map_object_record(wow_entities::MapObjectRecord::new_player(player).unwrap())
        .unwrap();
    map.insert_map_object_record(wow_entities::MapObjectRecord::new_pet(pet).unwrap())
        .unwrap();
    attach_map_manager(&mut session, manager);

    session
        .handle_query_pet_name(QueryPetName {
            unit_guid: pet_guid,
        })
        .await;

    let bytes = send_rx.try_recv().expect("query pet name response");
    assert_eq!(
        u16::from_le_bytes([bytes[0], bytes[1]]),
        wow_constants::ServerOpcodes::QueryPetNameResponse as u16
    );
    assert_eq!(&bytes[2..18], &pet_guid.to_raw_bytes());
    assert_eq!(bytes[18] & 0x80, 0x80);
    assert!(bytes.windows(5).any(|window| window == b"Misha"));
    assert!(bytes.windows(4).any(|window| window == 0_u32.to_le_bytes()));
}

#[test]
fn query_pet_name_handler_registration_matches_cpp() {
    let entry = inventory::iter::<PacketHandlerEntry>
        .into_iter()
        .find(|entry| entry.opcode == ClientOpcodes::QueryPetName)
        .expect("QueryPetName handler registration");

    assert_eq!(entry.status, SessionStatus::LoggedIn);
    assert_eq!(entry.processing, PacketProcessing::Inplace);
    assert_eq!(entry.handler_name, "handle_query_pet_name");
}

#[tokio::test]
async fn logout_releases_active_loot_views_like_cpp_remove_from_world() {
    let (mut session, send_rx) = make_session_with_send_capacity(4);
    let player_guid = ObjectGuid::create_player(1, 42);
    let loot_guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 19_030);
    let canonical: crate::session::SharedCanonicalMapManager =
        Arc::new(std::sync::Mutex::new(wow_map::MapManager::default()));
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
        wow_data::MapEntry {
            id: 1,
            instance_type: wow_data::map::MAP_COMMON,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
    ])));
    assert!(session.ensure_login_player_controller_like_cpp(
        player_guid,
        "LogoutOwner".to_string(),
        Position::ZERO,
        1,
        1,
        1,
        10,
        0,
    ));
    let _ = session.ensure_canonical_world_map_for_current_player_like_cpp();
    assert_eq!(
        session.current_canonical_player_map_key_like_cpp(),
        Some(wow_map::MapKey::new(1, 0))
    );
    assert!(session.try_claim_character_login_like_cpp(player_guid));
    session.set_active_loot_guid(loot_guid);
    session.loot_table.insert(
        loot_guid,
        CreatureLoot {
            loot_guid,
            coins: 0,
            unlooted_count: 0,
            loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
            dungeon_encounter_id: 0,
            loot_method: 0,
            loot_master: ObjectGuid::EMPTY,
            round_robin_player: ObjectGuid::EMPTY,
            player_ffa_items: Vec::new(),
            players_looting: Vec::new(),
            allowed_looters: Vec::new(),
            items: vec![LootEntry {
                loot_list_id: 0,
                item_id: 25,
                quantity: 1,
                random_properties_id: 0,
                random_properties_seed: 0,
                item_context: 0,
                flags: LootEntryFlags::default(),
                allowed_looters: vec![player_guid],
                roll_winner: ObjectGuid::EMPTY,
                ffa_looted_by: Vec::new(),
                taken: false,
            }],
            looted_by_player: false,
        },
    );

    session
        .handle_logout_request(LogoutRequest { idle_logout: false })
        .await;

    let sent = send_rx.try_recv().unwrap();
    let mut sent = WorldPacket::from_bytes(&sent);
    assert_eq!(
        sent.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::LootReleaseAll as u16
    );
    assert_eq!(sent.remaining(), 0);

    let sent = send_rx.try_recv().unwrap();
    let mut sent = WorldPacket::from_bytes(&sent);
    assert_eq!(
        sent.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::LogoutResponse as u16
    );

    let sent = send_rx.try_recv().unwrap();
    let mut sent = WorldPacket::from_bytes(&sent);
    assert_eq!(
        sent.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::LootRelease as u16
    );
    assert_eq!(sent.read_packed_guid().unwrap(), loot_guid);
    assert_eq!(sent.read_packed_guid().unwrap(), player_guid);

    let sent = send_rx.try_recv().unwrap();
    let mut sent = WorldPacket::from_bytes(&sent);
    assert_eq!(
        sent.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::LogoutComplete as u16
    );
    assert!(!session.is_active_loot_guid(loot_guid));
    assert!(
        !session.loot_table.contains_key(&loot_guid),
        "full logout release retires the session packet-cache copy like C++"
    );
    assert!(session.player_guid().is_none());
    assert!(
        canonical
            .lock()
            .unwrap()
            .find_map(1, 0)
            .unwrap()
            .map()
            .get_typed_player(player_guid)
            .is_none(),
        "logout must retire the canonical Player before releasing its login claim"
    );
    let (mut replacement, _) = make_session_with_send_capacity(1);
    assert!(
        replacement.try_claim_character_login_like_cpp(player_guid),
        "the claim is released only after logout retired the old Player identity"
    );
    replacement.release_character_login_claim_like_cpp();
}

#[test]
fn start_zones_are_valid() {
    for race in [1, 2, 3, 4, 5, 6, 7, 8, 10, 11] {
        let zone = start_zone(race);
        assert!(zone > 0, "Race {race} has invalid zone");
    }
}

#[test]
fn parse_equipment_cache_empty() {
    let eq = parse_equipment_cache("");
    for slot in &eq {
        assert_eq!(slot.display_id, 0);
        assert_eq!(slot.inv_type, 0);
    }
}

#[test]
fn vendor_buy_price_uses_cpp_buy_count_unit_price() {
    assert_eq!(vendor_buy_quantity_and_price(500, 5, 1), (1, 100));
    assert_eq!(vendor_buy_quantity_and_price(500, 5, 3), (3, 300));
    assert_eq!(vendor_buy_quantity_and_price(500, 0, 2), (2, 1000));
    assert_eq!(vendor_buy_quantity_and_price(0, 5, 3), (3, 0));
    assert_eq!(vendor_buy_quantity_and_price(1, 5, 1), (1, 1));
}

#[test]
fn vendor_buy_price_clamps_count_to_cpp_max_money_amount() {
    let unit_price = (MAX_MONEY_AMOUNT / 2) + 1;

    assert_eq!(
        vendor_buy_quantity_and_price(unit_price, 1, 3),
        (1, unit_price)
    );
}

#[test]
fn vendor_buy_zero_gold_price_does_not_dirty_coinage_like_cpp() {
    assert_eq!(vendor_buy_coinage_update_like_cpp(0, 12_345), None);
    assert_eq!(vendor_buy_coinage_update_like_cpp(1, 12_344), Some(12_344));
}

#[test]
fn vendor_stored_new_item_keeps_cpp_new_and_bonding_flags() {
    assert_eq!(
        vendor_stored_new_item_flags_like_cpp(None, INVENTORY_SLOT_BAG_0, 23),
        ItemFieldFlags::NEW_ITEM.bits()
    );

    let mut template = wow_entities::ItemStorageTemplate::regular_item(700, 1);
    template.bonding = ItemBondingType::OnAcquire;
    assert_eq!(
        vendor_stored_new_item_flags_like_cpp(Some(&template), INVENTORY_SLOT_BAG_0, 23),
        (ItemFieldFlags::NEW_ITEM | ItemFieldFlags::SOULBOUND).bits()
    );
}

#[test]
fn vendor_buy_packet_quantity_uses_cpp_uint8_count_conversion() {
    assert_eq!(vendor_buy_packet_quantity_to_cpp_count(0), 1);
    assert_eq!(vendor_buy_packet_quantity_to_cpp_count(1), 1);
    assert_eq!(vendor_buy_packet_quantity_to_cpp_count(256), 1);
    assert_eq!(vendor_buy_packet_quantity_to_cpp_count(-1), 255);
}

#[test]
fn vendor_buy_currency_preflight_matches_cpp_quantity_guards() {
    assert_eq!(vendor_buy_currency_packet_quantity_to_cpp_count(0), 1);
    assert_eq!(vendor_buy_currency_packet_quantity_to_cpp_count(5), 5);
    assert_eq!(
        vendor_buy_currency_quantity_block_result(5, 3),
        Some(InventoryResult::CantBuyQuantity)
    );
    assert_eq!(vendor_buy_currency_quantity_block_result(5, 10), None);
    assert_eq!(
        vendor_buy_currency_quantity_block_result(0, 10),
        Some(InventoryResult::CantBuyQuantity)
    );
}

#[test]
fn vendor_buy_muid_uses_cpp_one_based_uint32_slot_conversion() {
    assert_eq!(vendor_buy_muid_to_cpp_slot(0), None);
    assert_eq!(vendor_buy_muid_to_cpp_slot(1), Some(0));
    assert_eq!(vendor_buy_muid_to_cpp_slot(2), Some(1));
    assert_eq!(vendor_buy_muid_to_cpp_slot(-1), Some(u32::MAX - 1));
}

#[test]
fn vendor_list_item_limit_matches_cpp_cap() {
    assert!(!vendor_list_reaches_cpp_item_limit(149));
    assert!(vendor_list_reaches_cpp_item_limit(150));
    assert!(vendor_list_reaches_cpp_item_limit(151));
}

#[test]
fn vendor_list_currency_rows_match_cpp_basic_guards() {
    let store = CurrencyTypesStore::from_entries([wow_data::CurrencyTypesEntry {
        id: 395,
        category_id: 0,
        inventory_icon_file_id: 0,
        spell_weight: 0,
        spell_category: 0,
        max_qty: 0,
        max_earnable_per_week: 0,
        quality: 0,
        faction_id: 0,
        award_condition_id: 0,
        flags: wow_constants::CurrencyTypesFlags::empty(),
        flags_b: wow_constants::CurrencyTypesFlagsB::empty(),
    }]);
    assert!(vendor_list_should_skip_currency_row(Some(&store), 395, 0,));
    assert!(!vendor_list_should_skip_currency_row(Some(&store), 395, 10,));
    assert!(vendor_list_should_skip_currency_row(
        Some(&store),
        999_999,
        10
    ));
    assert!(vendor_list_should_skip_currency_row(None, 395, 10));
}

#[test]
fn vendor_player_condition_id_evaluates_player_condition_store_like_cpp() {
    let store = PlayerConditionStore::from_entries([
        wow_data::PlayerConditionEntry {
            id: 42,
            class_mask: 0,
            ..Default::default()
        },
        wow_data::PlayerConditionEntry {
            id: 43,
            class_mask: 1 << 1,
            ..Default::default()
        },
    ]);
    let context = PlayerConditionContextLikeCpp {
        class_mask: 1,
        ..Default::default()
    };

    assert_eq!(
        vendor_player_condition_failed_id_like_cpp(0, Some(&store), Some(context)),
        0
    );
    assert_eq!(
        vendor_player_condition_failed_id_like_cpp(42, Some(&store), Some(context)),
        0
    );
    assert_eq!(
        vendor_player_condition_failed_id_like_cpp(43, Some(&store), Some(context)),
        43
    );
    assert_eq!(
        vendor_player_condition_failed_id_like_cpp(999, Some(&store), Some(context)),
        0
    );
    assert_eq!(
        vendor_buy_player_condition_block_result_like_cpp(42, Some(&store), Some(context)),
        None
    );
    assert_eq!(
        vendor_buy_player_condition_block_result_like_cpp(43, Some(&store), Some(context)),
        Some(InventoryResult::ItemLocked)
    );
    assert_eq!(
        vendor_buy_player_condition_block_result_like_cpp(42, None, Some(context)),
        Some(InventoryResult::ItemLocked)
    );
}

#[test]
fn vendor_condition_presence_fails_closed_until_condition_mgr_exists() {
    assert_eq!(vendor_conditions_block_result(false), None);
    assert_eq!(
        vendor_conditions_block_result(true),
        Some(BuyResult::CantFindItem)
    );
}

#[test]
fn vendor_required_reputation_fails_closed_until_reputation_mgr_exists() {
    assert_eq!(
        vendor_buy_required_reputation_block_result(None, None, -1),
        None
    );
    assert_eq!(
        vendor_buy_required_reputation_block_result(Some(72), Some(5), -1),
        Some(BuyResult::ReputationRequire)
    );
    assert_eq!(
        vendor_buy_required_reputation_block_result(Some(72), Some(5), 5),
        None
    );
}

#[test]
fn vendor_buy_extended_cost_fails_closed_like_cpp_preflight() {
    let currency_store = CurrencyTypesStore::from_entries([wow_data::CurrencyTypesEntry {
        id: 395,
        category_id: 0,
        inventory_icon_file_id: 0,
        spell_weight: 0,
        spell_category: 0,
        max_qty: 0,
        max_earnable_per_week: 0,
        quality: 0,
        faction_id: 0,
        award_condition_id: 0,
        flags: wow_constants::CurrencyTypesFlags::empty(),
        flags_b: wow_constants::CurrencyTypesFlagsB::empty(),
    }]);
    let extended_cost_store =
        ItemExtendedCostStore::from_entries([wow_data::ItemExtendedCostEntry {
            id: 12,
            required_arena_rating: 0,
            arena_bracket: 0,
            flags: wow_constants::ItemExtendedCostFlags::empty(),
            min_faction_id: 0,
            min_reputation: 0,
            required_achievement: 0,
            item_id: [0; wow_data::MAX_ITEM_EXT_COST_ITEMS],
            item_count: [0; wow_data::MAX_ITEM_EXT_COST_ITEMS],
            currency_id: [395, 0, 0, 0, 0],
            currency_count: [10, 0, 0, 0, 0],
        }]);

    assert_eq!(
        vendor_buy_extended_cost_block_result(
            None,
            None,
            |_, _| false,
            |_, _| false,
            false,
            0,
            5,
            3
        ),
        None
    );
    assert_eq!(
        vendor_buy_extended_cost_block_result(
            Some(&extended_cost_store),
            Some(&currency_store),
            |_, _| false,
            |_, _| false,
            false,
            12,
            5,
            3
        ),
        Some(VendorExtendedCostBlock::Equip(
            InventoryResult::CantBuyQuantity
        ))
    );
    assert_eq!(
        vendor_buy_extended_cost_block_result(
            Some(&extended_cost_store),
            Some(&currency_store),
            |_, _| true,
            |currency_id, amount| currency_id == 395 && amount >= 20,
            false,
            12,
            5,
            10
        ),
        Some(VendorExtendedCostBlock::Equip(
            InventoryResult::VendorMissingTurnins
        ))
    );
    assert_eq!(
        vendor_buy_extended_cost_block_result(
            Some(&extended_cost_store),
            Some(&currency_store),
            |_, _| true,
            |currency_id, amount| currency_id == 395 && amount >= 20,
            true,
            12,
            5,
            10
        ),
        None
    );
    assert_eq!(
        vendor_buy_extended_cost_currency_costs(Some(&extended_cost_store), 12, 5, 10),
        vec![(395, 20)]
    );
    let item_turnin_store =
        ItemExtendedCostStore::from_entries([wow_data::ItemExtendedCostEntry {
            id: 13,
            required_arena_rating: 0,
            arena_bracket: 0,
            flags: wow_constants::ItemExtendedCostFlags::empty(),
            min_faction_id: 0,
            min_reputation: 0,
            required_achievement: 0,
            item_id: [700, 0, 0, 0, 0],
            item_count: [3, 0, 0, 0, 0],
            currency_id: [0; wow_data::MAX_ITEM_EXT_COST_CURRENCIES],
            currency_count: [0; wow_data::MAX_ITEM_EXT_COST_CURRENCIES],
        }]);
    assert_eq!(
        vendor_buy_extended_cost_block_result(
            Some(&item_turnin_store),
            Some(&currency_store),
            |item_id, amount| item_id == 700 && amount == 6,
            |_, _| true,
            true,
            13,
            5,
            10
        ),
        None
    );
    assert_eq!(
        vendor_buy_extended_cost_block_result(
            Some(&item_turnin_store),
            Some(&currency_store),
            |_, _| false,
            |_, _| true,
            true,
            13,
            5,
            10
        ),
        Some(VendorExtendedCostBlock::Equip(
            InventoryResult::VendorMissingTurnins
        ))
    );
    assert_eq!(
        vendor_buy_extended_cost_item_costs(Some(&item_turnin_store), 13, 5, 10),
        vec![(700, 6)]
    );
    let checked_currency_amount = std::cell::Cell::new(false);
    assert_eq!(
        vendor_buy_extended_cost_block_result(
            Some(&extended_cost_store),
            Some(&currency_store),
            |_, _| true,
            |currency_id, amount| {
                checked_currency_amount.set(true);
                assert_eq!(currency_id, 395);
                assert_eq!(amount, 20);
                false
            },
            true,
            12,
            5,
            10
        ),
        Some(VendorExtendedCostBlock::Equip(
            InventoryResult::VendorMissingTurnins
        ))
    );
    assert!(checked_currency_amount.get());
    assert_eq!(
        vendor_buy_extended_cost_block_result(
            Some(&extended_cost_store),
            None,
            |_, _| true,
            |_, _| true,
            true,
            12,
            5,
            10
        ),
        Some(VendorExtendedCostBlock::Buy(BuyResult::CantFindItem))
    );
    assert_eq!(
        vendor_buy_extended_cost_block_result(
            Some(&extended_cost_store),
            Some(&currency_store),
            |_, _| true,
            |_, _| true,
            true,
            99,
            5,
            10
        ),
        Some(VendorExtendedCostBlock::Silent)
    );
}

#[test]
fn vendor_buy_direct_store_preflight_matches_cpp_store_branch() {
    assert_eq!(
        vendor_buy_direct_store_block_result(NULL_BAG, NULL_SLOT, 1),
        None
    );
    assert_eq!(
        vendor_buy_direct_store_block_result(INVENTORY_SLOT_BAG_0, 35, 1),
        None
    );
    assert_eq!(
        vendor_buy_direct_store_block_result(NULL_BAG, 35, 1),
        Some(InventoryResult::WrongSlot)
    );
    assert_eq!(
        vendor_buy_direct_store_block_result(INVENTORY_SLOT_BAG_0, 0, 1),
        Some(InventoryResult::NotEquippable)
    );
}

#[test]
fn vendor_buy_stock_refill_matches_cpp_increment_and_full_reset() {
    assert_eq!(vendor_buy_stock_refill_count(2, 20, 10, 5, 20), (12, false));
    assert_eq!(vendor_buy_stock_refill_count(18, 10, 10, 5, 20), (20, true));
    assert_eq!(vendor_buy_stock_refill_count(2, 9, 10, 5, 20), (2, false));
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

#[tokio::test]
async fn cancel_temp_enchantment_clears_equipped_temporary_enchant_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(8);
    let player_guid = ObjectGuid::create_player(1, 42);
    session.set_player_guid(Some(player_guid));
    let item_guid = insert_cancel_temp_enchant_test_item(&mut session, player_guid, 15, 901);

    session
        .handle_cancel_temp_enchantment(CancelTempEnchantment { slot: 15 })
        .await;

    let item = session
        .inventory_item_objects_like_cpp()
        .get(&item_guid)
        .unwrap();
    assert_eq!(
        item.data().enchantments[EnchantmentSlot::EnhancementTemporary as usize].id,
        0
    );
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn cancel_temp_enchantment_ignores_non_equipment_slot_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(8);
    let player_guid = ObjectGuid::create_player(1, 42);
    session.set_player_guid(Some(player_guid));
    let item_guid = insert_cancel_temp_enchant_test_item(&mut session, player_guid, 36, 902);

    session
        .handle_cancel_temp_enchantment(CancelTempEnchantment { slot: 36 })
        .await;

    let item = session
        .inventory_item_objects_like_cpp()
        .get(&item_guid)
        .unwrap();
    assert_eq!(
        item.data().enchantments[EnchantmentSlot::EnhancementTemporary as usize].id,
        902
    );
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn cancel_temp_enchantment_ignores_missing_enchant_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(8);
    let player_guid = ObjectGuid::create_player(1, 42);
    session.set_player_guid(Some(player_guid));
    let item_guid = insert_cancel_temp_enchant_test_item(&mut session, player_guid, 15, 0);

    session
        .handle_cancel_temp_enchantment(CancelTempEnchantment { slot: 15 })
        .await;

    let item = session
        .inventory_item_objects_like_cpp()
        .get(&item_guid)
        .unwrap();
    assert_eq!(
        item.data().enchantments[EnchantmentSlot::EnhancementTemporary as usize].duration,
        12_000
    );
    assert!(send_rx.try_recv().is_err());
}

#[test]
fn extended_cost_item_turnin_plan_matches_cpp_destroy_order() {
    let (_pkt_tx, pkt_rx) = flume::bounded::<wow_packet::WorldPacket>(8);
    let (send_tx, _send_rx) = flume::bounded::<Vec<u8>>(8);
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
    let player_guid = ObjectGuid::create_player(1, 1);
    session.set_player_guid(Some(player_guid));

    for (slot, db_guid, count) in [(35, 10_u64, 4_u32), (36, 11_u64, 5_u32)] {
        let item_guid = ObjectGuid::create_item(1, db_guid as i64);
        session.insert_inventory_item_like_cpp(
            slot,
            InventoryItem {
                guid: item_guid,
                entry_id: 700,
                db_guid,
                inventory_type: None,
            },
        );
        let item = session.make_inventory_item_object(
            item_guid,
            700,
            player_guid,
            count,
            0,
            ItemContext::Vendor,
            slot,
        );
        session.insert_inventory_item_object(item);
    }

    assert!(session.has_item_count_direct_inventory(700, 9));
    assert!(!session.has_item_count_direct_inventory(700, 10));
    assert_eq!(
        session.plan_destroy_item_count_direct_inventory(700, 6),
        Some(vec![
            ExtendedCostItemTurninChange::Delete {
                slot: 35,
                item_guid: ObjectGuid::create_item(1, 10),
                db_guid: 10,
            },
            ExtendedCostItemTurninChange::Update {
                slot: 36,
                item_guid: ObjectGuid::create_item(1, 11),
                db_guid: 11,
                new_count: 3,
            },
        ])
    );
}

#[test]
fn vendor_item_current_count_updates_like_cpp() {
    let (_pkt_tx, pkt_rx) = flume::bounded::<wow_packet::WorldPacket>(8);
    let (send_tx, _send_rx) = flume::bounded::<Vec<u8>>(8);
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
    let vendor_guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 7, 1);

    assert_eq!(
        session.vendor_item_current_count(vendor_guid, 700, 5, 60, 1),
        5
    );
    assert_eq!(
        session.update_vendor_item_current_count(vendor_guid, 700, 5, 60, 1, 2),
        3
    );
    assert_eq!(
        session.vendor_item_current_count(vendor_guid, 700, 5, 60, 1),
        3
    );

    if let Some(count) = session.vendor_item_counts.get_mut(&(vendor_guid, 700)) {
        count.last_increment_time = WorldSession::vendor_stock_now_secs().saturating_sub(120);
    }

    assert_eq!(
        session.vendor_item_current_count(vendor_guid, 700, 5, 60, 1),
        5
    );
    assert!(!session.vendor_item_counts.contains_key(&(vendor_guid, 700)));
}

#[test]
fn vendor_list_sold_out_filter_matches_cpp_gm_branch() {
    assert!(vendor_list_should_skip_sold_out(5, 0, false));
    assert!(!vendor_list_should_skip_sold_out(5, 0, true));
    assert!(!vendor_list_should_skip_sold_out(5, 1, false));
    assert!(!vendor_list_should_skip_sold_out(0, 0, false));
}

#[test]
fn vendor_list_refundable_flag_matches_cpp_template_guard() {
    assert!(vendor_list_item_refundable(
        Some(ItemFlags::ITEM_PURCHASE_RECORD),
        Some(1),
        42
    ));
    assert!(!vendor_list_item_refundable(
        Some(ItemFlags::ITEM_PURCHASE_RECORD),
        Some(2),
        42
    ));
    assert!(!vendor_list_item_refundable(
        Some(ItemFlags::ITEM_PURCHASE_RECORD),
        Some(1),
        0
    ));
    assert!(!vendor_list_item_refundable(None, Some(1), 42));
}

#[test]
fn loaded_refund_metadata_matches_cpp_load_cleanup() {
    let refundable_flags = (ItemFieldFlags::SOULBOUND | ItemFieldFlags::REFUNDABLE).bits();
    assert_eq!(
        loaded_item_refund_decision(refundable_flags, 7_200, Some(123), Some(45)),
        LoadedItemRefundDecision::Valid {
            paid_money: 123,
            paid_extended_cost: 45,
        }
    );
    assert_eq!(
        loaded_item_refund_decision(refundable_flags, 7_201, Some(123), Some(45)),
        LoadedItemRefundDecision::Clear {
            new_flags: ItemFieldFlags::SOULBOUND.bits(),
        }
    );
    assert_eq!(
        loaded_item_refund_decision(refundable_flags, 10, None, Some(45)),
        LoadedItemRefundDecision::Clear {
            new_flags: ItemFieldFlags::SOULBOUND.bits(),
        }
    );
    assert_eq!(
        loaded_item_refund_decision(ItemFieldFlags::SOULBOUND.bits(), 10, Some(123), Some(45)),
        LoadedItemRefundDecision::None
    );
}

#[test]
fn destroy_item_count_action_matches_cpp_direct_item_branch() {
    assert_eq!(
        destroy_item_count_action(5, 0),
        DestroyItemCountAction::FullStack
    );
    assert_eq!(
        destroy_item_count_action(5, 5),
        DestroyItemCountAction::FullStack
    );
    assert_eq!(
        destroy_item_count_action(5, 7),
        DestroyItemCountAction::FullStack
    );
    assert_eq!(
        destroy_item_count_action(5, 2),
        DestroyItemCountAction::PartialStack { new_count: 3 }
    );
}

#[test]
fn sell_item_amount_action_matches_cpp_amount_branch() {
    assert_eq!(
        sell_item_amount_action(5, 0),
        SellItemAmountAction::FullStack { amount: 5 }
    );
    assert_eq!(
        sell_item_amount_action(5, 5),
        SellItemAmountAction::FullStack { amount: 5 }
    );
    assert_eq!(
        sell_item_amount_action(5, 2),
        SellItemAmountAction::PartialStack {
            amount: 2,
            remaining: 3,
        }
    );
    assert_eq!(sell_item_amount_action(5, 6), SellItemAmountAction::Invalid);
    assert_eq!(
        sell_item_amount_action(5, -1),
        SellItemAmountAction::Invalid
    );
}

#[test]
fn player_money_gain_like_cpp_enforces_max_money_amount() {
    assert_eq!(player_money_gain_like_cpp(0, 0), Some(0));
    assert_eq!(
        player_money_gain_like_cpp(MAX_MONEY_AMOUNT - 1, 1),
        Some(MAX_MONEY_AMOUNT)
    );
    assert_eq!(player_money_gain_like_cpp(MAX_MONEY_AMOUNT, 1), None);
    assert_eq!(player_money_gain_like_cpp(MAX_MONEY_AMOUNT - 10, 11), None);
    assert_eq!(player_money_gain_like_cpp(0, MAX_MONEY_AMOUNT + 1), None);
}

#[test]
fn item_currently_looted_guard_uses_runtime_loot_generated_state() {
    let mut item = wow_entities::Item::default();
    assert!(!item_is_currently_looted_like_cpp(&item));

    item.set_loot_generated(true);
    assert!(item_is_currently_looted_like_cpp(&item));
}

#[test]
fn sell_non_empty_bag_guard_matches_cpp_is_not_empty_bag() {
    assert!(item_is_not_empty_bag_like_cpp(
        Some(InventoryType::Bag),
        true
    ));
    assert!(!item_is_not_empty_bag_like_cpp(
        Some(InventoryType::Bag),
        false
    ));
    assert!(!item_is_not_empty_bag_like_cpp(
        Some(InventoryType::Chest),
        true
    ));
    assert!(!item_is_not_empty_bag_like_cpp(None, true));
}

#[test]
fn vendor_list_allowed_class_filter_matches_cpp_bind_on_acquire_branch() {
    let warrior_mask = 1i16 << (1 - 1);
    let mage_mask = 1i16 << (8 - 1);

    assert!(!vendor_list_should_skip_allowed_class(
        Some(warrior_mask),
        Some(ItemBondingType::OnAcquire as u8),
        1,
        false,
    ));
    assert!(vendor_list_should_skip_allowed_class(
        Some(warrior_mask),
        Some(ItemBondingType::OnAcquire as u8),
        8,
        false,
    ));
    assert!(!vendor_list_should_skip_allowed_class(
        Some(warrior_mask),
        Some(ItemBondingType::OnEquip as u8),
        8,
        false,
    ));
    assert!(!vendor_list_should_skip_allowed_class(
        Some(warrior_mask),
        Some(ItemBondingType::OnAcquire as u8),
        8,
        true,
    ));
    assert!(!vendor_list_should_skip_allowed_class(
        Some(warrior_mask | mage_mask),
        Some(ItemBondingType::OnAcquire as u8),
        8,
        false,
    ));
    assert!(!vendor_list_should_skip_allowed_class(
        Some(-1),
        Some(ItemBondingType::OnAcquire as u8),
        8,
        false,
    ));
}

#[test]
fn vendor_list_faction_filter_matches_cpp_team_branch() {
    assert_eq!(player_team_for_race_cpp(1), Team::Alliance);
    assert_eq!(player_team_for_race_cpp(2), Team::Horde);
    assert_eq!(player_team_for_race_cpp(11), Team::Alliance);
    assert_eq!(player_team_for_race_cpp(10), Team::Horde);

    assert!(vendor_list_should_skip_faction_flags(
        Some(ItemFlags2::FactionHorde as u32),
        Team::Alliance,
        false,
    ));
    assert!(!vendor_list_should_skip_faction_flags(
        Some(ItemFlags2::FactionHorde as u32),
        Team::Horde,
        false,
    ));
    assert!(vendor_list_should_skip_faction_flags(
        Some(ItemFlags2::FactionAlliance as u32),
        Team::Horde,
        false,
    ));
    assert!(!vendor_list_should_skip_faction_flags(
        Some(ItemFlags2::FactionAlliance as u32),
        Team::Horde,
        true,
    ));
    assert!(!vendor_list_should_skip_faction_flags(
        None,
        Team::Alliance,
        false
    ));
}

#[test]
fn vendor_buy_template_gates_match_cpp_error_shapes() {
    let warrior_mask = 1i16 << (1 - 1);

    assert_eq!(
        vendor_buy_template_block_result(
            Some(warrior_mask),
            Some(ItemBondingType::OnAcquire as u8),
            None,
            8,
            1,
            false,
        ),
        Some(VendorBuyTemplateBlock::BuyError(BuyResult::CantFindItem))
    );
    assert_eq!(
        vendor_buy_template_block_result(
            Some(warrior_mask),
            Some(ItemBondingType::OnAcquire as u8),
            None,
            8,
            1,
            true,
        ),
        None
    );
    assert_eq!(
        vendor_buy_template_block_result(
            None,
            None,
            Some(ItemFlags2::FactionHorde as u32),
            1,
            1,
            false,
        ),
        Some(VendorBuyTemplateBlock::Silent)
    );
    assert_eq!(
        vendor_buy_template_block_result(
            None,
            None,
            Some(ItemFlags2::FactionHorde as u32),
            1,
            2,
            false,
        ),
        None
    );
}

#[test]
fn vendor_buy_destination_maps_player_container_like_cpp() {
    let player_guid = ObjectGuid::create_player(1, 42);
    let buy = BuyItem {
        vendor_guid: ObjectGuid::EMPTY,
        container_guid: player_guid,
        quantity: 1,
        muid: 1,
        slot: 35,
        item_type: 0,
        item_id: 700,
    };

    assert_eq!(
        vendor_buy_direct_inventory_destination(player_guid, &buy),
        Some((INVENTORY_SLOT_BAG_0, 35))
    );
}

#[test]
fn vendor_buy_destination_rejects_cpp_slot_over_max_bag_size() {
    let player_guid = ObjectGuid::create_player(1, 42);
    let buy = BuyItem {
        vendor_guid: ObjectGuid::EMPTY,
        container_guid: player_guid,
        quantity: 1,
        muid: 1,
        slot: (MAX_BAG_SIZE + 1) as i32,
        item_type: 0,
        item_id: 700,
    };

    assert_eq!(
        vendor_buy_direct_inventory_destination(player_guid, &buy),
        None
    );
}

#[test]
fn vendor_buy_destination_uses_cpp_uint8_slot_conversion() {
    let player_guid = ObjectGuid::create_player(1, 42);
    let buy = BuyItem {
        vendor_guid: ObjectGuid::EMPTY,
        container_guid: player_guid,
        quantity: 1,
        muid: 1,
        slot: 256,
        item_type: 0,
        item_id: 700,
    };

    assert_eq!(
        vendor_buy_direct_inventory_destination(player_guid, &buy),
        Some((INVENTORY_SLOT_BAG_0, 0))
    );
}

#[test]
fn parse_equipment_cache_real_data() {
    // Real data from DB: first slot has inv_type=0, next few slots have gear
    let cache = "0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 4 2470 0 0 0 20 33257 0 1 0";
    let eq = parse_equipment_cache(cache);
    // Slot 0: all zeros
    assert_eq!(eq[0].display_id, 0);
    // Slot 3: inv_type=4, display_id=2470
    assert_eq!(eq[3].inv_type, 4);
    assert_eq!(eq[3].display_id, 2470);
    // Slot 4: inv_type=20, display_id=33257, subclass=1
    assert_eq!(eq[4].inv_type, 20);
    assert_eq!(eq[4].display_id, 33257);
    assert_eq!(eq[4].subclass, 1);
}

#[test]
fn enum_character_flags_do_not_map_resting_like_cpp() {
    let flags = enum_character_flags_like_cpp(0x20, 0, 0, None, false);

    assert_eq!(flags.flags, 0);
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

#[tokio::test]
async fn character_enumeration_uses_typed_rows_and_keeps_cleanup_best_effort_like_cpp() {
    let port = CharacterEnumerationPortFixtureLikeCpp::new([
        CharacterEnumerationLoadOutcomeLikeCpp::Loaded {
            rows: vec![character_enumeration_row_like_cpp()],
            expired_ban_cleanup_error: Some("best-effort cleanup failed".to_owned()),
        },
    ]);
    let (mut session, send_rx) = make_session_with_send_capacity(2);
    session.set_declined_names_used_like_cpp(true);
    session.set_character_enumeration_persistence_port_like_cpp(port.clone());

    session.handle_enum_characters().await;

    assert_eq!(
        port.requests(),
        vec![CharacterEnumerationRequestLikeCpp {
            account_id: 1,
            declined_names_used: true,
        }]
    );
    assert!(session.is_legit_character(&ObjectGuid::create_player(1, 42)));
    assert!(send_rx.try_recv().is_ok());
}

#[tokio::test]
async fn character_enumeration_query_failure_publishes_failure_and_no_legit_guid() {
    let port = CharacterEnumerationPortFixtureLikeCpp::new([
        CharacterEnumerationLoadOutcomeLikeCpp::Failed {
            reason: "query failed".to_owned(),
            expired_ban_cleanup_error: None,
        },
    ]);
    let (mut session, send_rx) = make_session_with_send_capacity(2);
    session.set_character_enumeration_persistence_port_like_cpp(port);

    session.handle_enum_characters().await;

    assert!(!session.is_legit_character(&ObjectGuid::create_player(1, 42)));
    assert!(send_rx.try_recv().is_ok());
}

fn creature_query_catalog_row_like_cpp() -> CreatureQueryCatalogRowLikeCpp {
    CreatureQueryCatalogRowLikeCpp {
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
        displays: vec![CreatureQueryDisplayRowLikeCpp {
            display_id: 19,
            scale: 0.75,
            probability: 0.25,
        }],
    }
}

#[tokio::test]
async fn creature_query_uses_typed_catalog_and_preserves_packet_projection_like_cpp() {
    let row = creature_query_catalog_row_like_cpp();
    let port =
        CreatureQueryCatalogPortFixtureLikeCpp::new([CreatureQueryCatalogOutcomeLikeCpp::Found {
            row: row.clone(),
            locale_error: Some("locale fallback diagnostic".to_owned()),
        }]);
    let (mut session, send_rx) = make_session_with_send_capacity(1);
    session.set_creature_query_catalog_persistence_port_like_cpp(port.clone());

    session
        .handle_query_creature(QueryCreature { creature_id: 42 })
        .await;

    assert_eq!(
        port.requests(),
        vec![CreatureQueryCatalogRequestLikeCpp {
            entry: 42,
            locale: "esES".to_owned(),
        }]
    );
    let mut names: [String; 4] = Default::default();
    names[0] = row.name;
    let expected = QueryCreatureResponse {
        creature_id: 42,
        allow: true,
        stats: Some(CreatureStats {
            title: row.subname,
            title_alt: row.title_alt,
            cursor_name: row.icon_name,
            civilian: row.civilian,
            leader: row.racial_leader,
            names,
            name_alts: Default::default(),
            flags: row.type_flags,
            creature_type: row.creature_type,
            creature_family: row.creature_family,
            classification: row.classification,
            proxy_creature_ids: row.kill_credits,
            display: CreatureDisplayStats {
                displays: vec![CreatureXDisplay {
                    creature_display_id: 19,
                    scale: 0.75,
                    probability: 0.25,
                }],
                total_probability: 0.25,
            },
            hp_multi: row.hp_multi,
            energy_multi: row.energy_multi,
            quest_items: Vec::new(),
            creature_movement_info_id: row.movement_id,
            health_scaling_expansion: 0,
            required_expansion: row.required_expansion,
            vignette_id: row.vignette_id,
            unit_class: row.unit_class,
            creature_difficulty_id: row.creature_difficulty_id,
            widget_set_id: row.widget_set_id,
            widget_set_unit_condition_id: row.widget_set_unit_condition_id,
        }),
    };
    assert_eq!(send_rx.try_recv().unwrap(), expected.to_bytes());
}

#[tokio::test]
async fn creature_query_missing_or_failed_catalog_emits_disallowed_response_like_cpp() {
    for outcome in [
        CreatureQueryCatalogOutcomeLikeCpp::Missing,
        CreatureQueryCatalogOutcomeLikeCpp::Failed {
            reason: "world query failed".to_owned(),
        },
    ] {
        let port = CreatureQueryCatalogPortFixtureLikeCpp::new([outcome]);
        let (mut session, send_rx) = make_session_with_send_capacity(1);
        session.set_creature_query_catalog_persistence_port_like_cpp(port);

        session
            .handle_query_creature(QueryCreature { creature_id: 43 })
            .await;

        assert_eq!(
            send_rx.try_recv().unwrap(),
            QueryCreatureResponse {
                creature_id: 43,
                allow: false,
                stats: None,
            }
            .to_bytes()
        );
    }
}

fn gameobject_query_catalog_row_like_cpp() -> GameObjectQueryCatalogRowLikeCpp {
    let mut data = [0_i32; wow_persistence::GAMEOBJECT_USE_TEMPLATE_DATA_COUNT_LIKE_CPP];
    data[0] = 7;
    data[34] = 41;
    GameObjectQueryCatalogRowLikeCpp {
        go_type: 3,
        display_id: 4,
        name: "Localized object".to_owned(),
        icon_name: "Directions".to_owned(),
        cast_bar_caption: "Opening".to_owned(),
        unk_string: "Unknown".to_owned(),
        size: 1.25,
        data,
        content_tuning_id: 42,
        quest_items: vec![43, 44],
    }
}

#[tokio::test]
async fn gameobject_query_uses_typed_catalog_and_preserves_packet_projection_like_cpp() {
    let row = gameobject_query_catalog_row_like_cpp();
    let port = GameObjectQueryCatalogPortFixtureLikeCpp::new([
        GameObjectQueryCatalogOutcomeLikeCpp::Found {
            row: row.clone(),
            locale_error: Some("locale fallback diagnostic".to_owned()),
            quest_items_error: Some("quest item fallback diagnostic".to_owned()),
        },
    ]);
    let guid = ObjectGuid::create_world_object(HighGuid::GameObject, 0, 0, 571, 0, 42, 99);
    let (mut session, send_rx) = make_session_with_send_capacity(1);
    session.set_gameobject_query_catalog_persistence_port_like_cpp(port.clone());

    session
        .handle_query_game_object(QueryGameObject {
            game_object_id: 42,
            guid,
        })
        .await;

    assert_eq!(
        port.requests(),
        vec![GameObjectQueryCatalogRequestLikeCpp {
            entry: 42,
            locale: "esES".to_owned(),
        }]
    );
    let mut names: [String; 4] = Default::default();
    names[0] = row.name;
    assert_eq!(
        send_rx.try_recv().unwrap(),
        QueryGameObjectResponse {
            game_object_id: 42,
            guid,
            allow: true,
            stats: Some(GameObjectStats {
                names,
                icon_name: row.icon_name,
                cast_bar_caption: row.cast_bar_caption,
                unk_string: row.unk_string,
                go_type: row.go_type,
                display_id: row.display_id,
                data: row.data,
                size: row.size,
                quest_items: row.quest_items,
                content_tuning_id: row.content_tuning_id,
            }),
        }
        .to_bytes()
    );
}

#[tokio::test]
async fn gameobject_query_missing_or_failed_catalog_preserves_guid_and_disallows_like_cpp() {
    let guid = ObjectGuid::create_world_object(HighGuid::GameObject, 0, 0, 571, 0, 43, 100);
    for outcome in [
        GameObjectQueryCatalogOutcomeLikeCpp::Missing,
        GameObjectQueryCatalogOutcomeLikeCpp::Failed {
            reason: "world query failed".to_owned(),
        },
    ] {
        let port = GameObjectQueryCatalogPortFixtureLikeCpp::new([outcome]);
        let (mut session, send_rx) = make_session_with_send_capacity(1);
        session.set_gameobject_query_catalog_persistence_port_like_cpp(port);

        session
            .handle_query_game_object(QueryGameObject {
                game_object_id: 43,
                guid,
            })
            .await;

        assert_eq!(
            send_rx.try_recv().unwrap(),
            QueryGameObjectResponse {
                game_object_id: 43,
                guid,
                allow: false,
                stats: None,
            }
            .to_bytes()
        );
    }
}

#[test]
fn enum_character_flags_map_ghost_rename_billing_and_declined_like_cpp() {
    let flags = enum_character_flags_like_cpp(
        PLAYER_FLAGS_GHOST_LIKE_CPP,
        AT_LOGIN_RENAME_LIKE_CPP,
        42,
        Some("Genitive"),
        true,
    );

    assert_eq!(
        flags.flags,
        CHARACTER_FLAG_GHOST_LIKE_CPP
            | CHARACTER_FLAG_RENAME_LIKE_CPP
            | CHARACTER_FLAG_LOCKED_BY_BILLING_LIKE_CPP
            | CHARACTER_FLAG_DECLINED_LIKE_CPP
    );
    assert_eq!(flags.flags2, 0);
    assert!(!flags.first_login);
}

#[test]
fn enum_character_flags_keep_declined_names_config_gated_like_cpp() {
    let disabled = enum_character_flags_like_cpp(0, 0, 0, Some("Genitive"), false);
    let empty = enum_character_flags_like_cpp(0, 0, 0, Some(""), true);
    let enabled = enum_character_flags_like_cpp(0, 0, 0, Some("Genitive"), true);

    assert_eq!(disabled.flags & CHARACTER_FLAG_DECLINED_LIKE_CPP, 0);
    assert_eq!(empty.flags & CHARACTER_FLAG_DECLINED_LIKE_CPP, 0);
    assert_eq!(
        enabled.flags & CHARACTER_FLAG_DECLINED_LIKE_CPP,
        CHARACTER_FLAG_DECLINED_LIKE_CPP
    );
}

#[test]
fn enum_character_flags_suppress_ghost_by_resurrect_like_cpp() {
    let flags = enum_character_flags_like_cpp(
        PLAYER_FLAGS_GHOST_LIKE_CPP,
        AT_LOGIN_RESURRECT_LIKE_CPP,
        0,
        None,
        false,
    );

    assert_eq!(flags.flags & CHARACTER_FLAG_GHOST_LIKE_CPP, 0);
}

#[test]
fn enum_character_flags2_use_cpp_customize_values_and_priority() {
    let customize = enum_character_flags_like_cpp(0, AT_LOGIN_CUSTOMIZE_LIKE_CPP, 0, None, false);
    let faction = enum_character_flags_like_cpp(
        0,
        AT_LOGIN_CHANGE_FACTION_LIKE_CPP | AT_LOGIN_CHANGE_RACE_LIKE_CPP,
        0,
        None,
        false,
    );
    let race = enum_character_flags_like_cpp(0, AT_LOGIN_CHANGE_RACE_LIKE_CPP, 0, None, false);
    let first = enum_character_flags_like_cpp(0, AT_LOGIN_FIRST_LIKE_CPP, 0, None, false);

    assert_eq!(customize.flags2, CHAR_CUSTOMIZE_FLAG_CUSTOMIZE_LIKE_CPP);
    assert_eq!(faction.flags2, CHAR_CUSTOMIZE_FLAG_FACTION_LIKE_CPP);
    assert_eq!(race.flags2, CHAR_CUSTOMIZE_FLAG_RACE_LIKE_CPP);
    assert!(first.first_login);
}

#[test]
fn raw_player_flags_not_passed_directly() {
    let flags = enum_character_flags_like_cpp(0x02, 0, 0, None, false);

    assert_eq!(flags.flags, 0);
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

#[test]
fn enum_character_pet_family_uses_creature_template_for_pet_classes_like_cpp() {
    let store = enum_pet_template_store(416, 8);

    assert_eq!(
        enum_character_pet_data_like_cpp(0, 0, CLASS_HUNTER_LIKE_CPP, 416, 1234, 27, Some(&store),),
        (1234, 27, 8)
    );
}

#[test]
fn enum_character_pet_data_stays_zero_for_ghost_non_pet_class_or_missing_template_like_cpp() {
    let store = enum_pet_template_store(416, 8);

    assert_eq!(
        enum_character_pet_data_like_cpp(
            PLAYER_FLAGS_GHOST_LIKE_CPP,
            0,
            CLASS_HUNTER_LIKE_CPP,
            416,
            1234,
            27,
            Some(&store),
        ),
        (0, 0, 0)
    );
    assert_eq!(
        enum_character_pet_data_like_cpp(0, 0, 1, 416, 1234, 27, Some(&store)),
        (0, 0, 0)
    );
    assert_eq!(
        enum_character_pet_data_like_cpp(0, 0, CLASS_HUNTER_LIKE_CPP, 999, 1234, 27, Some(&store)),
        (0, 0, 0)
    );
}
