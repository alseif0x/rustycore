//! Typed persistence capabilities installed by the application composition root.

use std::sync::Arc;

use crate::session::{
    CatalogPersistenceCapabilitiesLikeCpp, PlayerPersistenceCapabilitiesLikeCpp,
    SessionAdmissionPersistenceLikeCpp, SessionPersistencePortsLikeCpp,
    WorldPersistenceCapabilitiesLikeCpp, WorldSession,
};

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

impl WorldSession {
    /// Install the complete production persistence graph atomically.
    pub fn set_required_persistence_capabilities_like_cpp(
        &mut self,
        capabilities: SessionPersistencePortsLikeCpp,
    ) {
        self.persistence_ports_like_cpp = Box::new(capabilities);
    }

    pub fn set_vendor_trade_persistence_port_like_cpp(
        &mut self,
        port: Arc<dyn wow_persistence::VendorTradePersistencePortLikeCpp>,
    ) {
        self.persistence_ports_like_cpp.player.vendor_trade = Some(port);
    }

    pub(crate) fn vendor_trade_persistence_port_like_cpp(
        &self,
    ) -> Option<Arc<dyn wow_persistence::VendorTradePersistencePortLikeCpp>> {
        self.persistence_ports_like_cpp.player.vendor_trade.clone()
    }

    pub fn set_player_inventory_persistence_port_like_cpp(
        &mut self,
        port: Arc<dyn wow_persistence::PlayerInventoryPersistencePortLikeCpp>,
    ) {
        self.persistence_ports_like_cpp.player.player_inventory = Some(port);
    }

    pub(crate) fn player_inventory_persistence_port_like_cpp(
        &self,
    ) -> Option<Arc<dyn wow_persistence::PlayerInventoryPersistencePortLikeCpp>> {
        self.persistence_ports_like_cpp
            .player
            .player_inventory
            .clone()
    }

    pub fn set_player_quest_persistence_port_like_cpp(
        &mut self,
        port: Arc<dyn wow_persistence::PlayerQuestPersistencePortLikeCpp>,
    ) {
        self.persistence_ports_like_cpp.player.player_quest = Some(port);
    }

    pub(crate) fn player_quest_persistence_port_like_cpp(
        &self,
    ) -> Option<Arc<dyn wow_persistence::PlayerQuestPersistencePortLikeCpp>> {
        self.persistence_ports_like_cpp.player.player_quest.clone()
    }

    pub fn set_stored_item_persistence_port_like_cpp(
        &mut self,
        port: Arc<dyn wow_persistence::StoredItemPersistencePortLikeCpp>,
    ) {
        self.persistence_ports_like_cpp.player.stored_item = Some(port);
    }

    pub(crate) fn stored_item_persistence_port_like_cpp(
        &self,
    ) -> Option<Arc<dyn wow_persistence::StoredItemPersistencePortLikeCpp>> {
        self.persistence_ports_like_cpp.player.stored_item.clone()
    }

    pub fn set_character_administration_persistence_port_like_cpp(
        &mut self,
        port: Arc<dyn wow_persistence::CharacterAdministrationPersistencePortLikeCpp>,
    ) {
        self.persistence_ports_like_cpp
            .admission
            .character_administration = Some(port);
    }

    pub(crate) fn character_administration_persistence_port_like_cpp(
        &self,
    ) -> Option<Arc<dyn wow_persistence::CharacterAdministrationPersistencePortLikeCpp>> {
        self.persistence_ports_like_cpp
            .admission
            .character_administration
            .clone()
    }

    pub fn set_loot_template_catalog_persistence_port_like_cpp(
        &mut self,
        port: Arc<dyn wow_persistence::LootTemplateCatalogPersistencePortLikeCpp>,
    ) {
        self.persistence_ports_like_cpp
            .catalogs
            .loot_template_catalog = Some(port);
    }

    pub(crate) fn loot_template_catalog_persistence_port_like_cpp(
        &self,
    ) -> Option<Arc<dyn wow_persistence::LootTemplateCatalogPersistencePortLikeCpp>> {
        self.persistence_ports_like_cpp
            .catalogs
            .loot_template_catalog
            .clone()
    }

    pub fn set_vendor_catalog_persistence_port_like_cpp(
        &mut self,
        port: Arc<dyn wow_persistence::VendorCatalogPersistencePortLikeCpp>,
    ) {
        self.persistence_ports_like_cpp.catalogs.vendor_catalog = Some(port);
    }

    pub(crate) fn vendor_catalog_persistence_port_like_cpp(
        &self,
    ) -> Option<Arc<dyn wow_persistence::VendorCatalogPersistencePortLikeCpp>> {
        self.persistence_ports_like_cpp
            .catalogs
            .vendor_catalog
            .clone()
    }

    pub fn set_visibility_spawn_catalog_persistence_port_like_cpp(
        &mut self,
        port: Arc<dyn wow_persistence::VisibilitySpawnCatalogPersistencePortLikeCpp>,
    ) {
        self.persistence_ports_like_cpp
            .catalogs
            .visibility_spawn_catalog = Some(port);
    }

    pub(crate) fn visibility_spawn_catalog_persistence_port_like_cpp(
        &self,
    ) -> Option<Arc<dyn wow_persistence::VisibilitySpawnCatalogPersistencePortLikeCpp>> {
        self.persistence_ports_like_cpp
            .catalogs
            .visibility_spawn_catalog
            .clone()
    }
}
