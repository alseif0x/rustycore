//! Player XP and quest reward catalogs produced during ordered startup.

use super::*;

pub(super) struct ProgressionRewardCatalogs {
    pub(super) player_xp_table: Arc<Vec<u32>>,
    pub(super) exploration_base_xp_store: Arc<wow_data::ExplorationBaseXpStoreLikeCpp>,
    pub(super) quest_xp_store: Arc<wow_data::quest_xp::QuestXpStore>,
    pub(super) quest_money_reward_store: Arc<wow_data::progression_rewards::QuestMoneyRewardStore>,
    pub(super) quest_v2_store: Arc<wow_data::progression_rewards::QuestV2Store>,
    pub(super) quest_info_store: Arc<wow_data::progression_rewards::QuestInfoStore>,
    pub(super) quest_package_item_store: Arc<wow_data::progression_rewards::QuestPackageItemStore>,
}

pub(super) async fn load_progression_reward_catalogs(
    world_db: &Arc<WorldDatabase>,
    data_dir: &str,
    locale: &str,
) -> Result<ProgressionRewardCatalogs> {
    // Load player_xp_for_level table
    let player_xp_table = {
        let stmt = world_db.prepare(WorldStatements::SEL_PLAYER_XP_FOR_LEVEL);
        let mut table = vec![0u32; 82]; // index = level, 0=unused, 81=max
        if let Ok(result) = world_db.query(&stmt).await {
            let mut r = result;
            loop {
                let lvl: u8 = r.try_read::<u8>(0).unwrap_or(0);
                let xp: u32 = r
                    .try_read::<u32>(1)
                    .or_else(|| r.try_read::<i32>(1).map(|value| value as u32))
                    .unwrap_or(0);
                if (lvl as usize) < table.len() {
                    table[lvl as usize] = xp;
                }
                if !r.next_row() {
                    break;
                }
            }
        }
        Arc::new(table)
    };
    let exploration_base_xp_persistence =
        wow_database::MariaDbExplorationBaseXpCatalogPersistenceAdapterLikeCpp::new(Arc::clone(
            world_db,
        ));
    let exploration_base_xp_store = Arc::new(
        catalogs::exploration_base_xp::load_exploration_base_xp_catalog_like_cpp(
            &exploration_base_xp_persistence,
        )
        .await?,
    );

    // Load QuestXP.db2 for accurate XP rewards
    let dbc_path = format!("{}/dbc/{}", data_dir, locale);
    let quest_xp_store = Arc::new(
        wow_data::quest_xp::QuestXpStore::load(&dbc_path).unwrap_or_else(|e| {
            tracing::warn!("QuestXP.db2 not loaded ({e}), using fallback XP table");
            wow_data::quest_xp::QuestXpStore::default()
        }),
    );
    let quest_money_reward_store = Arc::new(
        wow_data::progression_rewards::QuestMoneyRewardStore::load(&data_dir, &locale)
            .context("Failed to load QuestMoneyReward.db2 — check DataDir and DBC.Locale config")?,
    );
    let quest_v2_store = Arc::new(
        wow_data::progression_rewards::QuestV2Store::load(&data_dir, &locale)
            .context("Failed to load QuestV2.db2 — check DataDir and DBC.Locale config")?,
    );
    let quest_info_store = Arc::new(
        wow_data::progression_rewards::QuestInfoStore::load(&data_dir, &locale)
            .context("Failed to load QuestInfo.db2 — check DataDir and DBC.Locale config")?,
    );
    let quest_package_item_store = Arc::new(
        wow_data::progression_rewards::QuestPackageItemStore::load(&data_dir, &locale)
            .context("Failed to load QuestPackageItem.db2 — check DataDir and DBC.Locale config")?,
    );

    Ok(ProgressionRewardCatalogs {
        player_xp_table,
        exploration_base_xp_store,
        quest_xp_store,
        quest_money_reward_store,
        quest_v2_store,
        quest_info_store,
        quest_package_item_store,
    })
}
