//! Quest, LFG, spell-area and access-control catalogs produced during ordered startup.

use super::*;

pub(super) struct QuestAccessCatalogs {
    pub(super) quest_store: Arc<wow_data::quest::QuestStore>,
    pub(super) lfg_dungeon_store_like_cpp: Arc<wow_data::LfgDungeonStoreLikeCpp>,
    pub(super) spell_area_store: Arc<wow_data::SpellAreaStoreLikeCpp>,
    pub(super) spell_world_catalog_persistence:
        wow_database::MariaDbSpellWorldCatalogPersistenceAdapterLikeCpp,
    pub(super) access_requirement_store: Arc<wow_data::AccessRequirementStoreLikeCpp>,
    pub(super) disable_mgr: Arc<wow_data::DisableMgrLikeCpp>,
}

pub(super) async fn load_quest_access_catalogs(
    quest_catalog_persistence: &wow_database::MariaDbQuestCatalogPersistenceAdapterLikeCpp,
    world_db: &Arc<WorldDatabase>,
    lfg_dungeons_store: &wow_data::LfgDungeonsStore,
    map_difficulty_store: &wow_data::MapDifficultyStore,
    spell_store: &wow_data::SpellStore,
    area_table_store: &wow_data::AreaTableStore,
    world_auxiliary_catalog_persistence: &wow_database::MariaDbWorldAuxiliaryCatalogPersistenceAdapterLikeCpp,
    map_store: &wow_data::MapStore,
    item_store: &wow_data::ItemStore,
    achievement_store: &wow_data::Db2IdStore,
    condition_disable_catalog_persistence: &wow_database::MariaDbConditionDisableCatalogPersistenceAdapterLikeCpp,
    criteria_store: &wow_data::Db2IdStore,
    battlemaster_list_store: &wow_data::Db2IdStore,
) -> Result<QuestAccessCatalogs> {
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
    let spell_world_catalog_persistence =
        wow_database::MariaDbSpellWorldCatalogPersistenceAdapterLikeCpp::new(Arc::clone(world_db));
    let spell_area_outcome = crate::spell::world_catalog::load_spell_area_like_cpp(
        &spell_world_catalog_persistence,
        |spell_id| spell_store.get(spell_id as i32).is_some(),
        |area_id| area_table_store.get(area_id).is_some(),
        |quest_id| quest_store.get(quest_id).is_some(),
    )
    .await
    .context("Failed to load C++ spell_area rows")?;
    let spell_area_store = Arc::new(spell_area_outcome.store);
    info!(
        "Loaded {} C++ spell_area rows ({} validation issues; SpellInfo no-aura-cancel mutation still pending)",
        spell_area_outcome.loaded_row_count,
        spell_area_outcome.errors.len()
    );
    let access_requirement_outcome =
        crate::world::auxiliary_catalog::load_access_requirements_like_cpp(
            world_auxiliary_catalog_persistence,
            map_store,
            map_difficulty_store,
            item_store,
            quest_store.as_ref(),
            achievement_store,
        )
        .await
        .context("Failed to load C++ access_requirement rows")?;
    let access_requirement_store = Arc::new(access_requirement_outcome.store);
    info!(
        "Loaded {} C++ access requirement rows ({} rows seen; {} map/difficulty skips; {} reference clears)",
        access_requirement_outcome.report.loaded_rows,
        access_requirement_outcome.report.rows_seen,
        access_requirement_outcome.report.skipped_missing_map.len()
            + access_requirement_outcome
                .report
                .skipped_missing_difficulty
                .len(),
        access_requirement_outcome.report.cleared_missing_item.len()
            + access_requirement_outcome
                .report
                .cleared_missing_item2
                .len()
            + access_requirement_outcome
                .report
                .cleared_missing_quest_a
                .len()
            + access_requirement_outcome
                .report
                .cleared_missing_quest_h
                .len()
            + access_requirement_outcome
                .report
                .cleared_missing_achievement
                .len()
    );
    let disable_mgr = Arc::new(
        load_disable_mgr_like_cpp(
            condition_disable_catalog_persistence,
            map_store,
            map_difficulty_store,
            spell_store,
            quest_store.as_ref(),
            criteria_store,
            battlemaster_list_store,
        )
        .await?,
    );

    Ok(QuestAccessCatalogs {
        quest_store,
        lfg_dungeon_store_like_cpp,
        spell_area_store,
        spell_world_catalog_persistence,
        access_requirement_store,
        disable_mgr,
    })
}
