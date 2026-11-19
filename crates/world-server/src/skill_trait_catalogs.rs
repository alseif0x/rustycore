//! Skill, trait and talent catalogs produced during ordered startup.

use super::*;

pub(super) struct SkillTraitCatalogs {
    pub(super) skill_line_store: Arc<wow_data::SkillLineStore>,
    pub(super) skill_store: Arc<wow_data::SkillStore>,
    pub(super) trait_tree_skill_line_index:
        Arc<wow_data::trait_tree::TraitTreeSkillLineIndexLikeCpp>,
    pub(super) trait_definition_store: Arc<wow_data::trait_tree::TraitDefinitionStore>,
    pub(super) trait_definition_locale_store: Arc<wow_data::trait_tree::TraitDefinitionLocaleStore>,
    pub(super) trait_currency_source_locale_store:
        Arc<wow_data::trait_tree::TraitCurrencySourceLocaleStore>,
    pub(super) trait_node_entry_store: Arc<wow_data::trait_tree::TraitNodeEntryStore>,
    pub(super) skill_tiers_store: Arc<wow_data::SkillTiersStoreLikeCpp>,
    pub(super) talent_store: Arc<wow_data::TalentStore>,
    pub(super) talent_tab_store: Arc<wow_data::TalentTabStore>,
    pub(super) num_talents_at_level_store:
        Arc<wow_data::progression_rewards::NumTalentsAtLevelStore>,
    pub(super) glyph_properties_store: Arc<wow_data::GlyphPropertiesStore>,
}

pub(super) async fn load_skill_trait_catalogs(
    data_dir: &str,
    locale: &str,
    hotfix_db: &Arc<HotfixDatabase>,
    db2_hotfix_removals: &wow_data::Db2HotfixRemovalStoreLikeCpp,
    skill_world_rules_persistence: &wow_database::MariaDbSkillWorldRulesPersistenceAdapterLikeCpp,
) -> Result<SkillTraitCatalogs> {
    // C++ skill authority follows table-granular startup order: SkillLine,
    // SkillLineAbility, SkillLineXTraitTree, then SkillRaceClassInfo. Keep the
    // WDC4/SQL stages separate so a failed table cannot be hidden by a combined read.
    let skill_catalog_hotfix_persistence =
        wow_database::MariaDbSkillCatalogHotfixPersistenceAdapterLikeCpp::new(Arc::clone(
            &hotfix_db,
        ));
    let skill_line_store = Arc::new(
        hotfix::skill_catalog::load_skill_line_store_like_cpp(
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
    let skill_catalog_stages = hotfix::skill_catalog::load_skill_catalog_stages_like_cpp(
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
    let skill_tiers_store = Arc::new(
        crate::skill_world_rules::load_skill_tiers_store_like_cpp(skill_world_rules_persistence)
            .await
            .context("Failed to load world.skill_tiers")?,
    );
    let talent_store = Arc::new(
        wow_data::TalentStore::load(&data_dir, &locale).context("Failed to load Talent.db2")?,
    );
    let talent_tab_store = Arc::new(
        wow_data::TalentTabStore::load(&data_dir, &locale)
            .context("Failed to load TalentTab.db2")?,
    );
    let num_talents_at_level_store = Arc::new(
        wow_data::progression_rewards::NumTalentsAtLevelStore::load(&data_dir, &locale)
            .context("Failed to load NumTalentsAtLevel.db2")?,
    );
    let glyph_properties_store = Arc::new(
        wow_data::GlyphPropertiesStore::load(&data_dir, &locale)
            .context("Failed to load GlyphProperties.db2")?,
    );
    info!(
        "Loaded {} talent rows, {} talent tabs, {} talent-level rows, and {} glyph property rows from DB2",
        talent_store.len(),
        talent_tab_store.len(),
        num_talents_at_level_store.len(),
        glyph_properties_store.len()
    );
    Ok(SkillTraitCatalogs {
        skill_line_store,
        skill_store,
        trait_tree_skill_line_index,
        trait_definition_store,
        trait_definition_locale_store,
        trait_currency_source_locale_store,
        trait_node_entry_store,
        skill_tiers_store,
        talent_store,
        talent_tab_store,
        num_talents_at_level_store,
        glyph_properties_store,
    })
}
