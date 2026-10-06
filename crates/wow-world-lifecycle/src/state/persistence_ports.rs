use std::sync::Arc;

use super::SessionLifecycleState;
use crate::SessionPersistencePortsLikeCpp;

impl SessionLifecycleState {
    pub fn battle_pet_purchase_persistence_port_like_cpp(
        &self,
    ) -> Option<&Arc<dyn wow_persistence::BattlePetPurchasePersistencePortLikeCpp>> {
        self.persistence_ports_like_cpp
            .player
            .battle_pet_purchase
            .as_ref()
    }

    pub fn session_account_state_port_like_cpp(
        &self,
    ) -> Option<&Arc<dyn wow_persistence::SessionAccountStatePortLikeCpp>> {
        self.persistence_ports_like_cpp
            .admission
            .session_account_state
            .as_ref()
    }

    pub fn set_session_account_state_port_like_cpp(
        &mut self,
        port: Arc<dyn wow_persistence::SessionAccountStatePortLikeCpp>,
    ) {
        self.persistence_ports_like_cpp
            .admission
            .session_account_state = Some(port);
    }

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

    pub fn vendor_trade_persistence_port_like_cpp(
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

    pub fn player_inventory_persistence_port_like_cpp(
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

    pub fn player_quest_persistence_port_like_cpp(
        &self,
    ) -> Option<Arc<dyn wow_persistence::PlayerQuestPersistencePortLikeCpp>> {
        self.persistence_ports_like_cpp.player.player_quest.clone()
    }

    pub fn set_player_quest_reward_persistence_port_like_cpp(
        &mut self,
        port: Arc<dyn wow_persistence::PlayerQuestRewardPersistencePortLikeCpp>,
    ) {
        self.persistence_ports_like_cpp.player.player_quest_reward = Some(port);
    }

    pub fn player_quest_reward_persistence_port_like_cpp(
        &self,
    ) -> Option<Arc<dyn wow_persistence::PlayerQuestRewardPersistencePortLikeCpp>> {
        self.persistence_ports_like_cpp
            .player
            .player_quest_reward
            .clone()
    }

    pub fn stored_item_persistence_port_like_cpp(
        &self,
    ) -> Option<Arc<dyn wow_persistence::StoredItemPersistencePortLikeCpp>> {
        self.persistence_ports_like_cpp.player.stored_item.clone()
    }

    pub fn set_stored_item_persistence_port_like_cpp(
        &mut self,
        port: Arc<dyn wow_persistence::StoredItemPersistencePortLikeCpp>,
    ) {
        self.persistence_ports_like_cpp.player.stored_item = Some(port);
    }

    pub fn set_character_administration_persistence_port_like_cpp(
        &mut self,
        port: Arc<dyn wow_persistence::CharacterAdministrationPersistencePortLikeCpp>,
    ) {
        self.persistence_ports_like_cpp
            .admission
            .character_administration = Some(port);
    }

    pub fn character_administration_persistence_port_like_cpp(
        &self,
    ) -> Option<Arc<dyn wow_persistence::CharacterAdministrationPersistencePortLikeCpp>> {
        self.persistence_ports_like_cpp
            .admission
            .character_administration
            .clone()
    }

    pub fn loot_template_catalog_persistence_port_like_cpp(
        &self,
    ) -> Option<Arc<dyn wow_persistence::LootTemplateCatalogPersistencePortLikeCpp>> {
        self.persistence_ports_like_cpp
            .catalogs
            .loot_template_catalog
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

    pub fn vendor_catalog_persistence_port_like_cpp(
        &self,
    ) -> Option<Arc<dyn wow_persistence::VendorCatalogPersistencePortLikeCpp>> {
        self.persistence_ports_like_cpp
            .catalogs
            .vendor_catalog
            .clone()
    }

    pub fn set_vendor_catalog_persistence_port_like_cpp(
        &mut self,
        port: Arc<dyn wow_persistence::VendorCatalogPersistencePortLikeCpp>,
    ) {
        self.persistence_ports_like_cpp.catalogs.vendor_catalog = Some(port);
    }

    pub fn visibility_spawn_catalog_persistence_port_like_cpp(
        &self,
    ) -> Option<Arc<dyn wow_persistence::VisibilitySpawnCatalogPersistencePortLikeCpp>> {
        self.persistence_ports_like_cpp
            .catalogs
            .visibility_spawn_catalog
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

    pub fn set_player_lifecycle_port_like_cpp(
        &mut self,
        port: Arc<dyn wow_persistence::PlayerLifecyclePortLikeCpp>,
    ) {
        self.persistence_ports_like_cpp.player.player_lifecycle = Some(port);
    }

    pub fn player_lifecycle_port_like_cpp(
        &self,
    ) -> Option<&Arc<dyn wow_persistence::PlayerLifecyclePortLikeCpp>> {
        self.persistence_ports_like_cpp
            .player
            .player_lifecycle
            .as_ref()
    }

    pub fn set_character_enumeration_persistence_port_like_cpp(
        &mut self,
        port: Arc<dyn wow_persistence::CharacterEnumerationPersistencePortLikeCpp>,
    ) {
        self.persistence_ports_like_cpp
            .admission
            .character_enumeration = Some(port);
    }

    pub fn character_enumeration_persistence_port_like_cpp(
        &self,
    ) -> Option<Arc<dyn wow_persistence::CharacterEnumerationPersistencePortLikeCpp>> {
        self.persistence_ports_like_cpp
            .admission
            .character_enumeration
            .clone()
    }

    pub fn set_packet_spoof_ban_persistence_port_like_cpp(
        &mut self,
        port: Arc<dyn wow_persistence::PacketSpoofBanPersistencePortLikeCpp>,
    ) {
        self.persistence_ports_like_cpp.admission.packet_spoof_ban = Some(port);
    }

    pub fn set_void_storage_persistence_port_like_cpp(
        &mut self,
        port: Arc<dyn wow_persistence::VoidStoragePersistencePortLikeCpp>,
    ) {
        self.persistence_ports_like_cpp.player.void_storage = Some(port);
    }

    pub fn void_storage_persistence_port_like_cpp(
        &self,
    ) -> Option<Arc<dyn wow_persistence::VoidStoragePersistencePortLikeCpp>> {
        self.persistence_ports_like_cpp.player.void_storage.clone()
    }

    pub fn set_social_persistence_port_like_cpp(
        &mut self,
        port: Arc<dyn wow_persistence::SocialPersistencePortLikeCpp>,
    ) {
        self.persistence_ports_like_cpp.player.social = Some(port);
    }

    pub fn social_persistence_port_like_cpp(
        &self,
    ) -> Option<Arc<dyn wow_persistence::SocialPersistencePortLikeCpp>> {
        self.persistence_ports_like_cpp.player.social.clone()
    }

    pub fn set_map_corpse_persistence_port_like_cpp(
        &mut self,
        port: Arc<dyn wow_persistence::MapCorpsePersistencePortLikeCpp>,
    ) {
        self.persistence_ports_like_cpp.world.map_corpse = Some(port);
    }

    pub fn map_corpse_persistence_port_like_cpp(
        &self,
    ) -> Option<&Arc<dyn wow_persistence::MapCorpsePersistencePortLikeCpp>> {
        self.persistence_ports_like_cpp.world.map_corpse.as_ref()
    }

    pub fn set_represented_group_persistence_port_like_cpp(
        &mut self,
        port: Arc<dyn wow_persistence::RepresentedGroupPersistencePortLikeCpp>,
    ) {
        self.persistence_ports_like_cpp.world.represented_group = Some(port);
    }

    pub fn represented_group_persistence_port_like_cpp(
        &self,
    ) -> Option<Arc<dyn wow_persistence::RepresentedGroupPersistencePortLikeCpp>> {
        self.persistence_ports_like_cpp
            .world
            .represented_group
            .clone()
    }

    pub fn set_support_bug_report_persistence_port_like_cpp(
        &mut self,
        port: Arc<dyn wow_persistence::SupportBugReportPersistencePortLikeCpp>,
    ) {
        self.persistence_ports_like_cpp.admission.support_bug_report = Some(port);
    }

    pub fn support_bug_report_persistence_port_like_cpp(
        &self,
    ) -> Option<Arc<dyn wow_persistence::SupportBugReportPersistencePortLikeCpp>> {
        self.persistence_ports_like_cpp
            .admission
            .support_bug_report
            .clone()
    }

    pub fn set_gossip_catalog_persistence_port_like_cpp(
        &mut self,
        port: Arc<dyn wow_persistence::GossipCatalogPersistencePortLikeCpp>,
    ) {
        self.persistence_ports_like_cpp.catalogs.gossip_catalog = Some(port);
    }

    pub fn gossip_catalog_persistence_port_like_cpp(
        &self,
    ) -> Option<Arc<dyn wow_persistence::GossipCatalogPersistencePortLikeCpp>> {
        self.persistence_ports_like_cpp
            .catalogs
            .gossip_catalog
            .clone()
    }

    pub fn set_player_name_query_persistence_port_like_cpp(
        &mut self,
        port: Arc<dyn wow_persistence::PlayerNameQueryPersistencePortLikeCpp>,
    ) {
        self.persistence_ports_like_cpp.admission.player_name_query = Some(port);
    }

    pub fn player_name_query_persistence_port_like_cpp(
        &self,
    ) -> Option<Arc<dyn wow_persistence::PlayerNameQueryPersistencePortLikeCpp>> {
        self.persistence_ports_like_cpp
            .admission
            .player_name_query
            .clone()
    }

    pub fn set_instance_lock_persistence_port_like_cpp(
        &mut self,
        port: Arc<dyn wow_persistence::InstanceLockPersistencePortLikeCpp>,
    ) {
        self.persistence_ports_like_cpp.player.instance_lock = Some(port);
    }

    pub fn instance_lock_persistence_port_like_cpp(
        &self,
    ) -> Option<Arc<dyn wow_persistence::InstanceLockPersistencePortLikeCpp>> {
        self.persistence_ports_like_cpp.player.instance_lock.clone()
    }

    pub fn set_battle_pet_purchase_persistence_port_like_cpp(
        &mut self,
        port: Arc<dyn wow_persistence::BattlePetPurchasePersistencePortLikeCpp>,
    ) {
        self.persistence_ports_like_cpp.player.battle_pet_purchase = Some(port);
    }

    pub fn quest_poi_persistence_port_like_cpp(
        &self,
    ) -> Option<&Arc<dyn wow_persistence::QuestPoiPersistencePortLikeCpp>> {
        self.persistence_ports_like_cpp.catalogs.quest_poi.as_ref()
    }

    pub fn set_quest_poi_persistence_port_like_cpp(
        &mut self,
        port: Arc<dyn wow_persistence::QuestPoiPersistencePortLikeCpp>,
    ) {
        self.persistence_ports_like_cpp.catalogs.quest_poi = Some(port);
    }

    pub fn player_spell_acquisition_persistence_port_like_cpp(
        &self,
    ) -> Option<&Arc<dyn wow_persistence::PlayerSpellAcquisitionPersistencePortLikeCpp>> {
        self.persistence_ports_like_cpp
            .player
            .player_spell_acquisition
            .as_ref()
    }

    pub fn set_player_spell_acquisition_persistence_port_like_cpp(
        &mut self,
        port: Arc<dyn wow_persistence::PlayerSpellAcquisitionPersistencePortLikeCpp>,
    ) {
        self.persistence_ports_like_cpp
            .player
            .player_spell_acquisition = Some(port);
    }

    pub fn group_loot_money_persistence_port_like_cpp(
        &self,
    ) -> Option<&Arc<dyn wow_persistence::GroupLootMoneyPersistencePortLikeCpp>> {
        self.persistence_ports_like_cpp
            .world
            .group_loot_money
            .as_ref()
    }

    pub fn set_group_loot_money_persistence_port_like_cpp(
        &mut self,
        port: Arc<dyn wow_persistence::GroupLootMoneyPersistencePortLikeCpp>,
    ) {
        self.persistence_ports_like_cpp.world.group_loot_money = Some(port);
    }

    pub fn stored_item_money_persistence_port_like_cpp(
        &self,
    ) -> Option<&Arc<dyn wow_persistence::StoredItemMoneyPersistencePortLikeCpp>> {
        self.persistence_ports_like_cpp
            .player
            .stored_item_money
            .as_ref()
    }

    pub fn set_stored_item_money_persistence_port_like_cpp(
        &mut self,
        port: Arc<dyn wow_persistence::StoredItemMoneyPersistencePortLikeCpp>,
    ) {
        self.persistence_ports_like_cpp.player.stored_item_money = Some(port);
    }

    pub fn item_template_addon_catalog_persistence_port_like_cpp(
        &self,
    ) -> Option<&Arc<dyn wow_persistence::ItemTemplateAddonCatalogPersistencePortLikeCpp>> {
        self.persistence_ports_like_cpp
            .catalogs
            .item_template_addon_catalog
            .as_ref()
    }

    pub fn set_item_template_addon_catalog_persistence_port_like_cpp(
        &mut self,
        port: Arc<dyn wow_persistence::ItemTemplateAddonCatalogPersistencePortLikeCpp>,
    ) {
        self.persistence_ports_like_cpp
            .catalogs
            .item_template_addon_catalog = Some(port);
    }
}
