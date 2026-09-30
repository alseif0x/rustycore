use std::sync::Arc;

use wow_database::player::spell_acquisition_adapter::spell_acquisition_port;

pub(super) fn build_session_persistence_ports(
    char_db: &Arc<wow_database::CharacterDatabase>,
    login_db: &Arc<wow_database::LoginDatabase>,
    world_db: &Arc<wow_database::WorldDatabase>,
    character_identity_cache: &Arc<wow_database::CharacterIdentityCacheLikeCpp>,
    player_name_query_persistence_port: &Arc<
        dyn wow_persistence::PlayerNameQueryPersistencePortLikeCpp,
    >,
    gossip_catalog_adapter: &Arc<wow_database::MariaDbGossipCatalogPersistenceAdapterLikeCpp>,
    represented_group_persistence_adapter: &Arc<
        wow_database::represented_group_persistence_adapter::MariaDbRepresentedGroupPersistenceAdapterLikeCpp,
    >,
    instance_lock_persistence_port: &Arc<dyn wow_persistence::InstanceLockPersistencePortLikeCpp>,
) -> wow_world::session::SessionPersistencePortsLikeCpp {
    let player_lifecycle_port: Arc<dyn wow_persistence::PlayerLifecyclePortLikeCpp> = Arc::new(
        wow_database::player::lifecycle_adapter::MariaDbPlayerLifecycleAdapterLikeCpp::new(
            Arc::clone(char_db),
            Arc::clone(login_db),
            Arc::clone(world_db),
        ),
    );
    let character_administration_persistence_port: Arc<
        dyn wow_persistence::CharacterAdministrationPersistencePortLikeCpp,
    > = Arc::new(
        wow_database::MariaDbCharacterAdministrationPersistenceAdapterLikeCpp::new(
            Arc::clone(char_db),
            Arc::clone(world_db),
            Arc::clone(character_identity_cache),
        ),
    );
    let character_enumeration_persistence_port: Arc<
        dyn wow_persistence::CharacterEnumerationPersistencePortLikeCpp,
    > = Arc::new(
        wow_database::MariaDbCharacterEnumerationPersistenceAdapterLikeCpp::new(Arc::clone(
            char_db,
        )),
    );
    let item_template_addon_catalog_persistence_port: Arc<
        dyn wow_persistence::ItemTemplateAddonCatalogPersistencePortLikeCpp,
    > = Arc::new(
        wow_database::MariaDbItemTemplateAddonCatalogPersistenceAdapterLikeCpp::new(Arc::clone(
            world_db,
        )),
    );
    let loot_template_catalog_persistence_port: Arc<
        dyn wow_persistence::LootTemplateCatalogPersistencePortLikeCpp,
    > = Arc::new(
        wow_database::MariaDbLootTemplateCatalogPersistenceAdapterLikeCpp::new(Arc::clone(
            world_db,
        )),
    );
    let vendor_catalog_persistence_port: Arc<
        dyn wow_persistence::VendorCatalogPersistencePortLikeCpp,
    > = Arc::new(
        wow_database::MariaDbVendorCatalogPersistenceAdapterLikeCpp::new(Arc::clone(world_db)),
    );
    let visibility_spawn_catalog_persistence_port: Arc<
        dyn wow_persistence::VisibilitySpawnCatalogPersistencePortLikeCpp,
    > = Arc::new(
        wow_database::MariaDbVisibilitySpawnCatalogPersistenceAdapterLikeCpp::new(Arc::clone(
            world_db,
        )),
    );
    let gossip_catalog_persistence_port: Arc<
        dyn wow_persistence::GossipCatalogPersistencePortLikeCpp,
    > = gossip_catalog_adapter.clone();
    let session_account_state_port: Arc<dyn wow_persistence::SessionAccountStatePortLikeCpp> =
        Arc::new(
            wow_database::session_account_state_adapter::MariaDbSessionAccountStateAdapterLikeCpp::new(
                Arc::clone(char_db),
            ),
        );
    let packet_spoof_ban_persistence_port: Arc<
        dyn wow_persistence::PacketSpoofBanPersistencePortLikeCpp,
    > = Arc::new(
        wow_database::packet_spoof_ban_adapter::MariaDbPacketSpoofBanPersistenceAdapterLikeCpp::new(
            Arc::clone(login_db),
        ),
    );
    let void_storage_persistence_port: Arc<dyn wow_persistence::VoidStoragePersistencePortLikeCpp> =
        Arc::new(
            wow_database::void_storage_adapter::MariaDbVoidStoragePersistenceAdapterLikeCpp::new(
                Arc::clone(char_db),
            ),
        );
    let social_persistence_port: Arc<dyn wow_persistence::SocialPersistencePortLikeCpp> = Arc::new(
        wow_database::social_adapter::MariaDbSocialPersistenceAdapterLikeCpp::new(Arc::clone(
            char_db,
        )),
    );
    let map_corpse_persistence_port: Arc<dyn wow_persistence::MapCorpsePersistencePortLikeCpp> =
        Arc::new(
            wow_database::map_corpse_adapter::MariaDbMapCorpsePersistenceAdapterLikeCpp::new(
                Arc::clone(char_db),
            ),
        );
    let quest_poi_persistence_port: Arc<dyn wow_persistence::QuestPoiPersistencePortLikeCpp> =
        Arc::new(
            wow_database::quest::poi_adapter::MariaDbQuestPoiPersistenceAdapterLikeCpp::new(
                Arc::clone(world_db),
            ),
        );
    let stored_item_money_persistence_port: Arc<
        dyn wow_persistence::StoredItemMoneyPersistencePortLikeCpp,
    > = Arc::new(
        wow_database::stored_item_money_adapter::MariaDbStoredItemMoneyPersistenceAdapterLikeCpp::new(
            Arc::clone(char_db),
        ),
    );
    let group_loot_money_persistence_port: Arc<
        dyn wow_persistence::GroupLootMoneyPersistencePortLikeCpp,
    > = Arc::new(
        wow_database::group_loot_money_adapter::MariaDbGroupLootMoneyPersistenceAdapterLikeCpp::new(
            Arc::clone(char_db),
        ),
    );
    let represented_group_persistence_port: Arc<
        dyn wow_persistence::RepresentedGroupPersistencePortLikeCpp,
    > = represented_group_persistence_adapter.clone();
    let support_bug_report_persistence_port: Arc<
        dyn wow_persistence::SupportBugReportPersistencePortLikeCpp,
    > = Arc::new(
        wow_database::support_bug_report_adapter::MariaDbSupportBugReportPersistenceAdapterLikeCpp::new(
            Arc::clone(char_db),
        ),
    );
    let spell_acquisition_port = spell_acquisition_port(Arc::clone(char_db));
    let battle_pet_purchase_persistence_port: Arc<
        dyn wow_persistence::BattlePetPurchasePersistencePortLikeCpp,
    > = Arc::new(
        wow_database::CharacterBattlePetPurchasePersistenceAdapterLikeCpp::new(Arc::clone(char_db)),
    );
    let stored_item_persistence_port: Arc<dyn wow_persistence::StoredItemPersistencePortLikeCpp> =
        Arc::new(
            wow_database::MariaDbStoredItemPersistenceAdapterLikeCpp::new(Arc::clone(char_db)),
        );
    let player_inventory_persistence_port: Arc<
        dyn wow_persistence::PlayerInventoryPersistencePortLikeCpp,
    > = Arc::new(
        wow_database::MariaDbPlayerInventoryPersistenceAdapterLikeCpp::new(Arc::clone(char_db)),
    );
    let player_quest_persistence_port: Arc<dyn wow_persistence::PlayerQuestPersistencePortLikeCpp> =
        Arc::new(
            wow_database::MariaDbPlayerQuestPersistenceAdapterLikeCpp::new(Arc::clone(char_db)),
        );
    let player_quest_reward_persistence_port: Arc<
        dyn wow_persistence::PlayerQuestRewardPersistencePortLikeCpp,
    > = Arc::new(
        wow_database::MariaDbPlayerQuestRewardPersistenceAdapterLikeCpp::new(Arc::clone(char_db)),
    );
    let vendor_trade_persistence_port: Arc<dyn wow_persistence::VendorTradePersistencePortLikeCpp> =
        Arc::new(
            wow_database::MariaDbVendorTradePersistenceAdapterLikeCpp::new(Arc::clone(char_db)),
        );
    wow_world::session::SessionPersistencePortsLikeCpp::required_like_cpp(
        wow_world::session::SessionAdmissionPersistenceLikeCpp::required_like_cpp(
            character_administration_persistence_port,
            character_enumeration_persistence_port,
            session_account_state_port,
            packet_spoof_ban_persistence_port,
            Arc::clone(player_name_query_persistence_port),
            support_bug_report_persistence_port,
        ),
        wow_world::session::PlayerPersistenceCapabilitiesLikeCpp::required_like_cpp(
            player_lifecycle_port,
            void_storage_persistence_port,
            social_persistence_port,
            stored_item_money_persistence_port,
            stored_item_persistence_port,
            player_inventory_persistence_port,
            player_quest_persistence_port,
            player_quest_reward_persistence_port,
            vendor_trade_persistence_port,
            spell_acquisition_port,
            Arc::clone(instance_lock_persistence_port),
            battle_pet_purchase_persistence_port,
        ),
        wow_world::session::WorldPersistenceCapabilitiesLikeCpp::required_like_cpp(
            map_corpse_persistence_port,
            group_loot_money_persistence_port,
            represented_group_persistence_port,
        ),
        wow_world::session::CatalogPersistenceCapabilitiesLikeCpp::required_like_cpp(
            quest_poi_persistence_port,
            item_template_addon_catalog_persistence_port,
            loot_template_catalog_persistence_port,
            vendor_catalog_persistence_port,
            visibility_spawn_catalog_persistence_port,
            gossip_catalog_persistence_port,
        ),
    )
}
