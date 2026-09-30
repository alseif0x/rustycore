use anyhow::Context;
use std::sync::Arc;
use tracing::info;

pub(super) struct SkillCatalogs {
    pub(super) trait_node_entry_store: Arc<wow_data::trait_tree::TraitNodeEntryStore>,
    pub(super) trait_currency_source_locale_store:
        Arc<wow_data::trait_tree::TraitCurrencySourceLocaleStore>,
    pub(super) trait_definition_locale_store:
        Arc<wow_data::trait_tree::TraitDefinitionLocaleStore>,
    pub(super) trait_definition_store: Arc<wow_data::trait_tree::TraitDefinitionStore>,
    pub(super) trait_tree_skill_line_index:
        Arc<wow_data::trait_tree::TraitTreeSkillLineIndexLikeCpp>,
    pub(super) skill_store: Arc<wow_data::SkillStore>,
    pub(super) skill_line_store: Arc<wow_data::SkillLineStore>,
}

pub(super) async fn load(
    data_dir: &str,
    locale: &str,
    hotfix_db: &Arc<wow_database::HotfixDatabase>,
    db2_hotfix_removals: &wow_data::Db2HotfixRemovalStoreLikeCpp,
) -> anyhow::Result<SkillCatalogs> {
    // C++ skill authority follows table-granular startup order: SkillLine,
    // SkillLineAbility, SkillLineXTraitTree, then SkillRaceClassInfo. Keep the
    // WDC4/SQL stages separate so a failed table cannot be hidden by a combined read.
    let skill_catalog_hotfix_persistence =
        wow_database::MariaDbSkillCatalogHotfixPersistenceAdapterLikeCpp::new(Arc::clone(
            hotfix_db,
        ));
    let skill_line_store = Arc::new(
        crate::hotfix::skill_catalog::load_skill_line_store_like_cpp(
            &data_dir,
            &locale,
            &skill_catalog_hotfix_persistence,
            &db2_hotfix_removals,
        )
        .await
        .context("Failed to load effective SkillLine store")?,
    );
    info!(
        "Loaded {} hydrated SkillLine rows and {} effective C++ lookup identities",
        skill_line_store.len(),
        skill_line_store.effective_record_count_like_cpp()
    );
    let skill_catalog_stages = crate::hotfix::skill_catalog::load_skill_catalog_stages_like_cpp(
        &data_dir,
        &locale,
        &skill_catalog_hotfix_persistence,
        &db2_hotfix_removals,
        skill_line_store.as_ref(),
    )
    .await
    .context("Failed to load effective SkillLineAbility/SkillRaceClassInfo stores")?;
    let skill_store_outcome = skill_catalog_stages.skill_store_outcome;
    let skill_store_report = &skill_store_outcome.report;
    info!(
        "Loaded {} effective SkillLineAbility rows ({} indexed, {} invalid, {} removed) and {} effective SkillRaceClassInfo rows ({} indexed, {} invalid, {} missing SkillLine, {} removed)",
        skill_store_report.skill_line_ability_effective_rows,
        skill_store_report.skill_line_ability_indexed_rows,
        skill_store_report.skill_line_ability_invalid_rows,
        skill_store_report.skill_line_ability_removed_rows,
        skill_store_report.skill_race_class_info_effective_rows,
        skill_store_report.skill_race_class_info_indexed_rows,
        skill_store_report.skill_race_class_info_invalid_rows,
        skill_store_report.skill_race_class_info_missing_skill_line_rows,
        skill_store_report.skill_race_class_info_removed_rows,
    );
    let skill_store = Arc::new(skill_store_outcome.store);
    let trait_tree_skill_line_index = skill_catalog_stages.trait_tree_skill_line_index;
    let trait_definition_store = skill_catalog_stages.trait_definition_store;
    let trait_definition_locale_store = skill_catalog_stages.trait_definition_locale_store;
    let trait_currency_source_locale_store =
        skill_catalog_stages.trait_currency_source_locale_store;
    let trait_node_entry_store = Arc::clone(&skill_catalog_stages.trait_node_entry_store);

    Ok(SkillCatalogs {
        skill_line_store,
        skill_store,
        trait_tree_skill_line_index,
        trait_definition_store,
        trait_definition_locale_store,
        trait_currency_source_locale_store,
        trait_node_entry_store,
    })
}
