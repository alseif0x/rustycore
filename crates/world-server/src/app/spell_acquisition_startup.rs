//! Ordered spell acquisition startup composition.

use crate::spell;
use anyhow::Context;
use std::sync::Arc;
use tracing::info;

pub(super) struct SpellAcquisitionCatalogs {
    pub(super) serverside_spell_errors: Vec<wow_data::ServersideSpellLoadErrorLikeCpp>,
    pub(super) serverside_effect_warnings: Vec<wow_data::ServersideSpellEffectLoadWarningLikeCpp>,
    pub(super) serverside_effect_errors: Vec<wow_data::ServersideSpellEffectLoadErrorLikeCpp>,
    pub(super) serverside_spell_effect_store: wow_data::ServersideSpellEffectStoreLikeCpp,
    pub(super) spell_custom_attribute_store: Arc<wow_data::SpellCustomAttributeStoreLikeCpp>,
    pub(super) spell_learn_spell_store: Arc<wow_data::SpellLearnSpellStoreLikeCpp>,
    pub(super) spell_learn_skill_store: Arc<wow_data::SpellLearnSkillStoreLikeCpp>,
    pub(super) spell_chain_store: Arc<wow_data::SpellChainStoreLikeCpp>,
    pub(super) spell_acquisition_catalog: Arc<wow_data::SpellAcquisitionCatalogLikeCpp>,
    pub(super) serverside_spell_store: Arc<wow_data::ServersideSpellStoreLikeCpp>,
    pub(super) spell_range_store: Arc<wow_data::SpellRangeStore>,
    pub(super) spell_radius_store: Arc<wow_data::SpellRadiusStore>,
}

pub(super) async fn load(
    data_dir: &str,
    locale: &str,
    spell_core_hotfix_persistence: &dyn wow_persistence::SpellCoreDb2HotfixPersistencePortLikeCpp,
    spell_acquisition_startup_persistence: &dyn wow_persistence::SpellAcquisitionStartupPersistencePortLikeCpp,
    db2_hotfix_removals: &wow_data::Db2HotfixRemovalStoreLikeCpp,
    spell_store: &mut wow_data::SpellStore,
    spell_name_store: &wow_data::SpellNameStore,
    difficulty_store: &wow_data::DifficultyStore,
    skill_store: &wow_data::SkillStore,
) -> anyhow::Result<SpellAcquisitionCatalogs> {
    // Load spell metadata (cast time, cooldown, effects, etc.) — Phase 2
    let spell_radius_store = Arc::new(
        spell::core_db2_hotfix::load_spell_radius_store_like_cpp(
            data_dir,
            locale,
            spell_core_hotfix_persistence,
            db2_hotfix_removals,
        )
        .await
        .context("Failed to load effective SpellRadius authority")?,
    );
    info!("Loaded {} spell radius rows", spell_radius_store.len());
    let spell_range_store = Arc::new(
        spell::core_db2_hotfix::load_spell_range_store_like_cpp(
            data_dir,
            locale,
            spell_core_hotfix_persistence,
            db2_hotfix_removals,
        )
        .await
        .context("Failed to load effective SpellRange authority")?,
    );
    info!("Loaded {} spell range rows", spell_range_store.len());
    let serverside_spell_effect_outcome =
        spell::acquisition_loader::load_serverside_spell_effects_like_cpp(
            spell_acquisition_startup_persistence,
            |spell_id| spell_store.contains_spell_info_any_difficulty_like_cpp(spell_id),
            |difficulty_id| difficulty_store.get(difficulty_id).is_some(),
            |radius_id| spell_radius_store.get(radius_id).is_some(),
        )
        .await
        .context("Failed to load C++ serverside_spell_effect rows")?;
    let serverside_spell_effect_store = serverside_spell_effect_outcome.store;
    info!(
        "Loaded {} C++ serverside_spell_effect rows ({} validation errors; {} radius warnings)",
        serverside_spell_effect_outcome.loaded_effect_count,
        serverside_spell_effect_outcome.errors.len(),
        serverside_spell_effect_outcome.warnings.len()
    );
    let serverside_spell_outcome = spell::acquisition_loader::load_serverside_spells_like_cpp(
        spell_acquisition_startup_persistence,
        &serverside_spell_effect_store,
        |spell_id| spell_name_store.get(spell_id).is_some(),
    )
    .await
    .context("Failed to load C++ serverside_spell rows")?;
    spell_store.apply_serverside_spell_interrupts_like_cpp(&serverside_spell_outcome.store);
    let serverside_spell_store = Arc::new(serverside_spell_outcome.store);
    info!(
        "Loaded {} C++ serverside_spell rows ({} validation errors; authoritative SpellInfo insertion still pending)",
        serverside_spell_outcome.loaded_spell_count,
        serverside_spell_outcome.errors.len()
    );

    let spell_acquisition_bootstrap = spell::acquisition_loader::load_like_cpp(
        data_dir,
        locale,
        spell_acquisition_startup_persistence,
        db2_hotfix_removals,
        spell_store,
        serverside_spell_store.as_ref(),
        difficulty_store,
        skill_store,
    )
    .await
    .context("Failed to compose effective spell-acquisition stores")?;
    let spell_acquisition_catalog = spell_acquisition_bootstrap.catalog;
    let spell_chain_store = spell_acquisition_bootstrap.chain_store;
    let spell_learn_skill_store = spell_acquisition_bootstrap.learn_skill_store;
    let spell_learn_spell_store = spell_acquisition_bootstrap.learn_spell_store;
    let spell_custom_attribute_store = spell_acquisition_bootstrap.custom_attribute_store;
    Ok(SpellAcquisitionCatalogs {
        spell_radius_store,
        spell_range_store,
        serverside_spell_store,
        spell_acquisition_catalog,
        spell_chain_store,
        spell_learn_skill_store,
        spell_learn_spell_store,
        spell_custom_attribute_store,
        serverside_spell_effect_store,
        serverside_effect_errors: serverside_spell_effect_outcome.errors,
        serverside_effect_warnings: serverside_spell_effect_outcome.warnings,
        serverside_spell_errors: serverside_spell_outcome.errors,
    })
}
