use std::sync::Arc;

#[derive(Clone, Default)]
pub struct SessionAdmissionPersistenceLikeCpp {
    pub(crate) character_administration:
        Option<Arc<dyn wow_persistence::CharacterAdministrationPersistencePortLikeCpp>>,
    pub(crate) character_enumeration:
        Option<Arc<dyn wow_persistence::CharacterEnumerationPersistencePortLikeCpp>>,
    pub(crate) session_account_state:
        Option<Arc<dyn wow_persistence::SessionAccountStatePortLikeCpp>>,
    pub(crate) packet_spoof_ban:
        Option<Arc<dyn wow_persistence::PacketSpoofBanPersistencePortLikeCpp>>,
    pub(crate) player_name_query:
        Option<Arc<dyn wow_persistence::PlayerNameQueryPersistencePortLikeCpp>>,
    pub(crate) support_bug_report:
        Option<Arc<dyn wow_persistence::SupportBugReportPersistencePortLikeCpp>>,
}

#[derive(Clone, Default)]
pub struct PlayerPersistenceCapabilitiesLikeCpp {
    pub(crate) player_lifecycle: Option<Arc<dyn wow_persistence::PlayerLifecyclePortLikeCpp>>,
    pub(crate) void_storage: Option<Arc<dyn wow_persistence::VoidStoragePersistencePortLikeCpp>>,
    pub(crate) social: Option<Arc<dyn wow_persistence::SocialPersistencePortLikeCpp>>,
    pub(crate) stored_item_money:
        Option<Arc<dyn wow_persistence::StoredItemMoneyPersistencePortLikeCpp>>,
    pub(crate) stored_item: Option<Arc<dyn wow_persistence::StoredItemPersistencePortLikeCpp>>,
    pub(crate) player_inventory:
        Option<Arc<dyn wow_persistence::PlayerInventoryPersistencePortLikeCpp>>,
    pub(crate) player_quest: Option<Arc<dyn wow_persistence::PlayerQuestPersistencePortLikeCpp>>,
    /// Commits one complete quest-reward operation as a single character
    /// transaction, the way C++ closes `Player::RewardQuest` with
    /// `SaveToDB(false)` (Player.cpp:14867).
    pub(crate) player_quest_reward:
        Option<Arc<dyn wow_persistence::PlayerQuestRewardPersistencePortLikeCpp>>,
    pub(crate) vendor_trade: Option<Arc<dyn wow_persistence::VendorTradePersistencePortLikeCpp>>,
    pub(crate) player_spell_acquisition:
        Option<Arc<dyn wow_persistence::PlayerSpellAcquisitionPersistencePortLikeCpp>>,
    pub(crate) instance_lock: Option<Arc<dyn wow_persistence::InstanceLockPersistencePortLikeCpp>>,
    pub(crate) battle_pet_purchase:
        Option<Arc<dyn wow_persistence::BattlePetPurchasePersistencePortLikeCpp>>,
}

#[derive(Clone, Default)]
pub struct WorldPersistenceCapabilitiesLikeCpp {
    pub(crate) map_corpse: Option<Arc<dyn wow_persistence::MapCorpsePersistencePortLikeCpp>>,
    pub(crate) group_loot_money:
        Option<Arc<dyn wow_persistence::GroupLootMoneyPersistencePortLikeCpp>>,
    pub(crate) represented_group:
        Option<Arc<dyn wow_persistence::RepresentedGroupPersistencePortLikeCpp>>,
}

#[derive(Clone, Default)]
pub struct CatalogPersistenceCapabilitiesLikeCpp {
    pub(crate) quest_poi: Option<Arc<dyn wow_persistence::QuestPoiPersistencePortLikeCpp>>,
    pub(crate) item_template_addon_catalog:
        Option<Arc<dyn wow_persistence::ItemTemplateAddonCatalogPersistencePortLikeCpp>>,
    pub(crate) loot_template_catalog:
        Option<Arc<dyn wow_persistence::LootTemplateCatalogPersistencePortLikeCpp>>,
    pub(crate) vendor_catalog:
        Option<Arc<dyn wow_persistence::VendorCatalogPersistencePortLikeCpp>>,
    pub(crate) visibility_spawn_catalog:
        Option<Arc<dyn wow_persistence::VisibilitySpawnCatalogPersistencePortLikeCpp>>,
    pub(crate) gossip_catalog:
        Option<Arc<dyn wow_persistence::GossipCatalogPersistencePortLikeCpp>>,
}

#[derive(Clone, Default)]
pub struct SessionPersistencePortsLikeCpp {
    pub(crate) admission: SessionAdmissionPersistenceLikeCpp,
    pub(crate) player: PlayerPersistenceCapabilitiesLikeCpp,
    pub(crate) world: WorldPersistenceCapabilitiesLikeCpp,
    pub(crate) catalogs: CatalogPersistenceCapabilitiesLikeCpp,
}

impl SessionAdmissionPersistenceLikeCpp {
    pub fn required_like_cpp(
        character_administration: Arc<
            dyn wow_persistence::CharacterAdministrationPersistencePortLikeCpp,
        >,
        character_enumeration: Arc<dyn wow_persistence::CharacterEnumerationPersistencePortLikeCpp>,
        session_account_state: Arc<dyn wow_persistence::SessionAccountStatePortLikeCpp>,
        packet_spoof_ban: Arc<dyn wow_persistence::PacketSpoofBanPersistencePortLikeCpp>,
        player_name_query: Arc<dyn wow_persistence::PlayerNameQueryPersistencePortLikeCpp>,
        support_bug_report: Arc<dyn wow_persistence::SupportBugReportPersistencePortLikeCpp>,
    ) -> Self {
        Self {
            character_administration: Some(character_administration),
            character_enumeration: Some(character_enumeration),
            session_account_state: Some(session_account_state),
            packet_spoof_ban: Some(packet_spoof_ban),
            player_name_query: Some(player_name_query),
            support_bug_report: Some(support_bug_report),
        }
    }
}

impl PlayerPersistenceCapabilitiesLikeCpp {
    #[allow(clippy::too_many_arguments)]
    pub fn required_like_cpp(
        player_lifecycle: Arc<dyn wow_persistence::PlayerLifecyclePortLikeCpp>,
        void_storage: Arc<dyn wow_persistence::VoidStoragePersistencePortLikeCpp>,
        social: Arc<dyn wow_persistence::SocialPersistencePortLikeCpp>,
        stored_item_money: Arc<dyn wow_persistence::StoredItemMoneyPersistencePortLikeCpp>,
        stored_item: Arc<dyn wow_persistence::StoredItemPersistencePortLikeCpp>,
        player_inventory: Arc<dyn wow_persistence::PlayerInventoryPersistencePortLikeCpp>,
        player_quest: Arc<dyn wow_persistence::PlayerQuestPersistencePortLikeCpp>,
        player_quest_reward: Arc<dyn wow_persistence::PlayerQuestRewardPersistencePortLikeCpp>,
        vendor_trade: Arc<dyn wow_persistence::VendorTradePersistencePortLikeCpp>,
        player_spell_acquisition: Arc<
            dyn wow_persistence::PlayerSpellAcquisitionPersistencePortLikeCpp,
        >,
        instance_lock: Arc<dyn wow_persistence::InstanceLockPersistencePortLikeCpp>,
        battle_pet_purchase: Arc<dyn wow_persistence::BattlePetPurchasePersistencePortLikeCpp>,
    ) -> Self {
        Self {
            player_lifecycle: Some(player_lifecycle),
            void_storage: Some(void_storage),
            social: Some(social),
            stored_item_money: Some(stored_item_money),
            stored_item: Some(stored_item),
            player_inventory: Some(player_inventory),
            player_quest: Some(player_quest),
            player_quest_reward: Some(player_quest_reward),
            vendor_trade: Some(vendor_trade),
            player_spell_acquisition: Some(player_spell_acquisition),
            instance_lock: Some(instance_lock),
            battle_pet_purchase: Some(battle_pet_purchase),
        }
    }
}

impl WorldPersistenceCapabilitiesLikeCpp {
    pub fn required_like_cpp(
        map_corpse: Arc<dyn wow_persistence::MapCorpsePersistencePortLikeCpp>,
        group_loot_money: Arc<dyn wow_persistence::GroupLootMoneyPersistencePortLikeCpp>,
        represented_group: Arc<dyn wow_persistence::RepresentedGroupPersistencePortLikeCpp>,
    ) -> Self {
        Self {
            map_corpse: Some(map_corpse),
            group_loot_money: Some(group_loot_money),
            represented_group: Some(represented_group),
        }
    }
}

impl CatalogPersistenceCapabilitiesLikeCpp {
    #[allow(clippy::too_many_arguments)]
    pub fn required_like_cpp(
        quest_poi: Arc<dyn wow_persistence::QuestPoiPersistencePortLikeCpp>,
        item_template_addon_catalog: Arc<
            dyn wow_persistence::ItemTemplateAddonCatalogPersistencePortLikeCpp,
        >,
        loot_template_catalog: Arc<dyn wow_persistence::LootTemplateCatalogPersistencePortLikeCpp>,
        vendor_catalog: Arc<dyn wow_persistence::VendorCatalogPersistencePortLikeCpp>,
        visibility_spawn_catalog: Arc<
            dyn wow_persistence::VisibilitySpawnCatalogPersistencePortLikeCpp,
        >,
        gossip_catalog: Arc<dyn wow_persistence::GossipCatalogPersistencePortLikeCpp>,
    ) -> Self {
        Self {
            quest_poi: Some(quest_poi),
            item_template_addon_catalog: Some(item_template_addon_catalog),
            loot_template_catalog: Some(loot_template_catalog),
            vendor_catalog: Some(vendor_catalog),
            visibility_spawn_catalog: Some(visibility_spawn_catalog),
            gossip_catalog: Some(gossip_catalog),
        }
    }
}

impl SessionPersistencePortsLikeCpp {
    pub fn required_like_cpp(
        admission: SessionAdmissionPersistenceLikeCpp,
        player: PlayerPersistenceCapabilitiesLikeCpp,
        world: WorldPersistenceCapabilitiesLikeCpp,
        catalogs: CatalogPersistenceCapabilitiesLikeCpp,
    ) -> Self {
        Self {
            admission,
            player,
            world,
            catalogs,
        }
    }
}
