//! Required session persistence graph construction during ordered startup.

use super::*;

pub(super) async fn build_session_persistence_startup(
    char_db: &Arc<CharacterDatabase>,
    login_db: &Arc<LoginDatabase>,
    world_db: &Arc<WorldDatabase>,
    gossip_catalog_adapter: &Arc<wow_database::MariaDbGossipCatalogPersistenceAdapterLikeCpp>,
    represented_group_persistence_adapter: Arc<wow_database::represented_group_persistence_adapter::MariaDbRepresentedGroupPersistenceAdapterLikeCpp>,
    instance_lock_persistence_port: &Arc<dyn wow_persistence::InstanceLockPersistencePortLikeCpp>,
) -> Result<wow_world::session::SessionPersistencePortsLikeCpp> {
    // The Player lifecycle port is composed here, before any session is
    // accepted, so a build that cannot persist lifecycle state fails at
    // startup rather than silently dropping offline marks at logout (#200).
    let (character_identity_cache, player_name_query_persistence_port) =
        wow_database::build_player_name_query_port_like_cpp(Arc::clone(&char_db), &login_db)
            .await
            .context("Failed to load C++ character identity cache")?;
    let player_lifecycle_port: Arc<dyn wow_persistence::PlayerLifecyclePortLikeCpp> = Arc::new(
        wow_database::player::lifecycle_adapter::MariaDbPlayerLifecycleAdapterLikeCpp::new(
            Arc::clone(&char_db),
            Arc::clone(&login_db),
            Arc::clone(&world_db),
        ),
    );
    let character_administration_persistence_port: Arc<
        dyn wow_persistence::CharacterAdministrationPersistencePortLikeCpp,
    > = Arc::new(
        wow_database::MariaDbCharacterAdministrationPersistenceAdapterLikeCpp::new(
            Arc::clone(&char_db),
            Arc::clone(&world_db),
            Arc::clone(&character_identity_cache),
        ),
    );
    let character_enumeration_persistence_port: Arc<
        dyn wow_persistence::CharacterEnumerationPersistencePortLikeCpp,
    > = Arc::new(
        wow_database::MariaDbCharacterEnumerationPersistenceAdapterLikeCpp::new(Arc::clone(
            &char_db,
        )),
    );
    let item_template_addon_catalog_persistence_port: Arc<
        dyn wow_persistence::ItemTemplateAddonCatalogPersistencePortLikeCpp,
    > = Arc::new(
        wow_database::MariaDbItemTemplateAddonCatalogPersistenceAdapterLikeCpp::new(Arc::clone(
            &world_db,
        )),
    );
    let loot_template_catalog_persistence_port: Arc<
        dyn wow_persistence::LootTemplateCatalogPersistencePortLikeCpp,
    > = Arc::new(
        wow_database::MariaDbLootTemplateCatalogPersistenceAdapterLikeCpp::new(Arc::clone(
            &world_db,
        )),
    );
    let vendor_catalog_persistence_port: Arc<
        dyn wow_persistence::VendorCatalogPersistencePortLikeCpp,
    > = Arc::new(
        wow_database::MariaDbVendorCatalogPersistenceAdapterLikeCpp::new(Arc::clone(&world_db)),
    );
    let visibility_spawn_catalog_persistence_port: Arc<
        dyn wow_persistence::VisibilitySpawnCatalogPersistencePortLikeCpp,
    > = Arc::new(
        wow_database::MariaDbVisibilitySpawnCatalogPersistenceAdapterLikeCpp::new(Arc::clone(
            &world_db,
        )),
    );
    let gossip_catalog_persistence_port: Arc<
        dyn wow_persistence::GossipCatalogPersistencePortLikeCpp,
    > = gossip_catalog_adapter.clone();
    let session_account_state_port: Arc<dyn wow_persistence::SessionAccountStatePortLikeCpp> =
        Arc::new(
            wow_database::session_account_state_adapter::MariaDbSessionAccountStateAdapterLikeCpp::new(
                Arc::clone(&char_db),
            ),
        );
    let packet_spoof_ban_persistence_port: Arc<
        dyn wow_persistence::PacketSpoofBanPersistencePortLikeCpp,
    > = Arc::new(
        wow_database::packet_spoof_ban_adapter::MariaDbPacketSpoofBanPersistenceAdapterLikeCpp::new(
            Arc::clone(&login_db),
        ),
    );
    let void_storage_persistence_port: Arc<dyn wow_persistence::VoidStoragePersistencePortLikeCpp> =
        Arc::new(
            wow_database::void_storage_adapter::MariaDbVoidStoragePersistenceAdapterLikeCpp::new(
                Arc::clone(&char_db),
            ),
        );
    let social_persistence_port: Arc<dyn wow_persistence::SocialPersistencePortLikeCpp> = Arc::new(
        wow_database::social_adapter::MariaDbSocialPersistenceAdapterLikeCpp::new(Arc::clone(
            &char_db,
        )),
    );
    let map_corpse_persistence_port: Arc<dyn wow_persistence::MapCorpsePersistencePortLikeCpp> =
        Arc::new(
            wow_database::map_corpse_adapter::MariaDbMapCorpsePersistenceAdapterLikeCpp::new(
                Arc::clone(&char_db),
            ),
        );
    let quest_poi_persistence_port: Arc<dyn wow_persistence::QuestPoiPersistencePortLikeCpp> =
        Arc::new(
            wow_database::quest::poi_adapter::MariaDbQuestPoiPersistenceAdapterLikeCpp::new(
                Arc::clone(&world_db),
            ),
        );
    let stored_item_money_persistence_port: Arc<
        dyn wow_persistence::StoredItemMoneyPersistencePortLikeCpp,
    > = Arc::new(
        wow_database::stored_item_money_adapter::MariaDbStoredItemMoneyPersistenceAdapterLikeCpp::new(
            Arc::clone(&char_db),
        ),
    );
    let group_loot_money_persistence_port: Arc<
        dyn wow_persistence::GroupLootMoneyPersistencePortLikeCpp,
    > = Arc::new(
        wow_database::group_loot_money_adapter::MariaDbGroupLootMoneyPersistenceAdapterLikeCpp::new(
            Arc::clone(&char_db),
        ),
    );
    let represented_group_persistence_port: Arc<
        dyn wow_persistence::RepresentedGroupPersistencePortLikeCpp,
    > = represented_group_persistence_adapter;
    let support_bug_report_persistence_port: Arc<
        dyn wow_persistence::SupportBugReportPersistencePortLikeCpp,
    > = Arc::new(
        wow_database::support_bug_report_adapter::MariaDbSupportBugReportPersistenceAdapterLikeCpp::new(
            Arc::clone(&char_db),
        ),
    );
    let spell_acquisition_port = spell_acquisition_port(Arc::clone(&char_db));
    let battle_pet_purchase_persistence_port: Arc<
        dyn wow_persistence::BattlePetPurchasePersistencePortLikeCpp,
    > = Arc::new(
        wow_database::CharacterBattlePetPurchasePersistenceAdapterLikeCpp::new(Arc::clone(
            &char_db,
        )),
    );
    // Build session resources
    let stored_item_persistence_port: Arc<dyn wow_persistence::StoredItemPersistencePortLikeCpp> =
        Arc::new(
            wow_database::MariaDbStoredItemPersistenceAdapterLikeCpp::new(Arc::clone(&char_db)),
        );
    let player_inventory_persistence_port: Arc<
        dyn wow_persistence::PlayerInventoryPersistencePortLikeCpp,
    > = Arc::new(
        wow_database::MariaDbPlayerInventoryPersistenceAdapterLikeCpp::new(Arc::clone(&char_db)),
    );
    let player_quest_persistence_port: Arc<dyn wow_persistence::PlayerQuestPersistencePortLikeCpp> =
        Arc::new(
            wow_database::MariaDbPlayerQuestPersistenceAdapterLikeCpp::new(Arc::clone(&char_db)),
        );
    let player_quest_reward_persistence_port: Arc<
        dyn wow_persistence::PlayerQuestRewardPersistencePortLikeCpp,
    > = Arc::new(
        wow_database::MariaDbPlayerQuestRewardPersistenceAdapterLikeCpp::new(Arc::clone(&char_db)),
    );
    let vendor_trade_persistence_port: Arc<dyn wow_persistence::VendorTradePersistencePortLikeCpp> =
        Arc::new(
            wow_database::MariaDbVendorTradePersistenceAdapterLikeCpp::new(Arc::clone(&char_db)),
        );
    Ok(
        wow_world::session::SessionPersistencePortsLikeCpp::required_like_cpp(
            wow_world::session::SessionAdmissionPersistenceLikeCpp::required_like_cpp(
                Arc::clone(&character_administration_persistence_port),
                Arc::clone(&character_enumeration_persistence_port),
                Arc::clone(&session_account_state_port),
                Arc::clone(&packet_spoof_ban_persistence_port),
                Arc::clone(&player_name_query_persistence_port),
                Arc::clone(&support_bug_report_persistence_port),
            ),
            wow_world::session::PlayerPersistenceCapabilitiesLikeCpp::required_like_cpp(
                Arc::clone(&player_lifecycle_port),
                Arc::clone(&void_storage_persistence_port),
                Arc::clone(&social_persistence_port),
                Arc::clone(&stored_item_money_persistence_port),
                Arc::clone(&stored_item_persistence_port),
                Arc::clone(&player_inventory_persistence_port),
                Arc::clone(&player_quest_persistence_port),
                Arc::clone(&player_quest_reward_persistence_port),
                Arc::clone(&vendor_trade_persistence_port),
                Arc::clone(&spell_acquisition_port),
                Arc::clone(&instance_lock_persistence_port),
                Arc::clone(&battle_pet_purchase_persistence_port),
            )
            .with_player_trade_like_cpp(Arc::new(
                wow_database::MariaDbPlayerTradePersistenceAdapterLikeCpp::new(Arc::clone(
                    &char_db,
                )),
            )),
            wow_world::session::WorldPersistenceCapabilitiesLikeCpp::required_like_cpp(
                Arc::clone(&map_corpse_persistence_port),
                Arc::clone(&group_loot_money_persistence_port),
                Arc::clone(&represented_group_persistence_port),
            ),
            wow_world::session::CatalogPersistenceCapabilitiesLikeCpp::required_like_cpp(
                Arc::clone(&quest_poi_persistence_port),
                Arc::clone(&item_template_addon_catalog_persistence_port),
                Arc::clone(&loot_template_catalog_persistence_port),
                Arc::clone(&vendor_catalog_persistence_port),
                Arc::clone(&visibility_spawn_catalog_persistence_port),
                Arc::clone(&gossip_catalog_persistence_port),
            ),
        ),
    )
}
