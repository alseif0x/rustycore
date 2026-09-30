//! Ordered quest admission startup composition.

use crate::catalogs;
use anyhow::Context;
use std::sync::Arc;
use tracing::info;

pub(super) struct QuestAdmissionCatalogs {
    pub(super) lfg_load_report: wow_data::LfgLoadReportLikeCpp,
    pub(super) lfg_dungeon_store_like_cpp: Arc<wow_data::LfgDungeonStoreLikeCpp>,
    pub(super) lfg_world_catalog_persistence:
        wow_database::MariaDbLfgWorldCatalogPersistenceAdapterLikeCpp,
    pub(super) quest_store: Arc<wow_data::quest::QuestStore>,
}

pub(super) async fn load(
    world_db: &Arc<wow_database::WorldDatabase>,
    quest_catalog_persistence: &dyn wow_persistence::QuestCatalogPersistencePortLikeCpp,
    lfg_dungeons_store: &wow_data::LfgDungeonsStore,
    map_difficulty_store: &wow_data::MapDifficultyStore,
) -> anyhow::Result<QuestAdmissionCatalogs> {
    // Load quest store (templates + objectives + NPC relations)
    let quest_store = Arc::new(
        catalogs::quest::load_quests_like_cpp(quest_catalog_persistence)
            .await
            .context("Failed to load quest store")?,
    );
    let lfg_world_catalog_persistence =
        wow_database::MariaDbLfgWorldCatalogPersistenceAdapterLikeCpp::new(Arc::clone(world_db));
    let lfg_load_outcome = catalogs::lfg_world::load_lfg_dungeon_store_like_cpp(
        &lfg_world_catalog_persistence,
        lfg_dungeons_store,
        map_difficulty_store,
        quest_store.as_ref(),
    )
    .await
    .context("Failed to load C++ LFG dungeon store")?;
    let lfg_dungeon_store_like_cpp = Arc::new(lfg_load_outcome.store);
    info!(
        "Loaded {} C++ LFG dungeon rows ({} templates, {} rewards; {} skipped db2 type, {} skipped map difficulty)",
        lfg_dungeon_store_like_cpp.len(),
        lfg_load_outcome.report.loaded_templates,
        lfg_load_outcome.report.loaded_rewards,
        lfg_load_outcome.report.skipped_type.len(),
        lfg_load_outcome.report.skipped_missing_map_difficulty.len(),
    );
    if std::env::var_os("RUSTYCORE_LFG_TRACE").is_some() {
        for id in [
            205_u32, 210, 211, 212, 213, 215, 217, 219, 221, 226, 241, 242, 245, 249, 252, 253,
            254, 255, 256, 259, 260, 2447, 2452, 2471,
        ] {
            match lfg_dungeon_store_like_cpp.get(id) {
                Some(dungeon) => info!(
                    id,
                    entry = dungeon.entry_like_cpp(),
                    type_id = dungeon.type_id,
                    map = dungeon.map,
                    difficulty = dungeon.difficulty,
                    expansion = dungeon.expansion,
                    group = dungeon.group,
                    min_level = dungeon.min_level,
                    max_level = dungeon.max_level,
                    required_item_level = dungeon.required_item_level,
                    seasonal = dungeon.seasonal,
                    "RUST_LFG_TRACE dungeon"
                ),
                None => info!(id, "RUST_LFG_TRACE dungeon missing"),
            }
        }
        let random_ids = lfg_dungeon_store_like_cpp
            .random_and_active_seasonal_dungeon_entries_like_cpp(80, 2, |_| false);
        info!(
            ?random_ids,
            "RUST_LFG_TRACE random entries level80 expansion2"
        );
    }
    Ok(QuestAdmissionCatalogs {
        quest_store,
        lfg_world_catalog_persistence,
        lfg_dungeon_store_like_cpp,
        lfg_load_report: lfg_load_outcome.report,
    })
}

pub(super) struct LfgDb2Catalog {
    pub(super) lfg_dungeons_store: Arc<wow_data::LfgDungeonsStore>,
    pub(super) lfg_dungeons_hotfix_persistence:
        wow_database::MariaDbLfgDungeonsHotfixPersistenceAdapterLikeCpp,
}

pub(super) async fn load_lfg_db2(
    data_dir: &str,
    locale: &str,
    hotfix_db: &Arc<wow_database::HotfixDatabase>,
) -> anyhow::Result<LfgDb2Catalog> {
    let lfg_dungeons_hotfix_persistence =
        wow_database::MariaDbLfgDungeonsHotfixPersistenceAdapterLikeCpp::new(Arc::clone(hotfix_db));
    let lfg_dungeons_store = Arc::new(
        crate::hotfix::lfg_dungeons::load_lfg_dungeons_like_cpp(
            data_dir,
            locale,
            &lfg_dungeons_hotfix_persistence,
        )
        .await
        .context(
            "Failed to load LFGDungeons.db2 / hotfix rows — check DataDir and DBC.Locale config",
        )?,
    );
    info!(
        "Loaded {} LFG dungeons from LFGDungeons.db2 / hotfix rows",
        lfg_dungeons_store.len()
    );
    Ok(LfgDb2Catalog {
        lfg_dungeons_hotfix_persistence,
        lfg_dungeons_store,
    })
}
