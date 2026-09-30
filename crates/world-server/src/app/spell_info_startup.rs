use anyhow::Context;
use std::sync::Arc;
use tracing::info;

pub(super) struct SpellInfoStartup {
    pub(super) spell_category_store: Arc<wow_data::SpellCategoryStore>,
    pub(super) spell_aura_options_store: Arc<wow_data::SpellAuraOptionsStore>,
    pub(super) spell_aura_restrictions_store: Arc<wow_data::SpellAuraRestrictionsStore>,
    pub(super) spell_casting_requirements_store: Arc<wow_data::SpellCastingRequirementsStore>,
    pub(super) spell_class_options_store: Arc<wow_data::SpellClassOptionsStore>,
    pub(super) spell_label_store: Arc<wow_data::SpellLabelStore>,
    pub(super) spell_equipped_items_store: Arc<wow_data::SpellEquippedItemsStore>,
    pub(super) spell_target_restrictions_store: Arc<wow_data::SpellTargetRestrictionsStore>,
    pub(super) spell_misc_store: Arc<wow_data::SpellMiscStore>,
}

pub(super) async fn load_spell_info_startup(
    data_dir: &str,
    locale: &str,
    persistence: &dyn wow_persistence::SpellCoreDb2HotfixPersistencePortLikeCpp,
    removals: &wow_data::Db2HotfixRemovalStoreLikeCpp,
) -> anyhow::Result<SpellInfoStartup> {
    let spell_category_store = Arc::new(
        crate::spell::core_db2_hotfix::load_spell_category_store_like_cpp(
            data_dir,
            locale,
            persistence,
            removals,
        )
        .await
        .context("Failed to load effective SpellCategory authority")?,
    );
    info!(
        "Loaded {} spell categories from SpellCategory.db2",
        spell_category_store.len()
    );
    let spell_aura_options_store = Arc::new(
        wow_data::SpellAuraOptionsStore::load(data_dir, locale)
            .context("Failed to load SpellAuraOptions.db2")?,
    );
    info!(
        "Loaded {} spell aura options rows",
        spell_aura_options_store.len()
    );
    let spell_aura_restrictions_store = Arc::new(
        crate::spell::core_db2_hotfix::load_spell_aura_restrictions_store_like_cpp(
            data_dir,
            locale,
            persistence,
            removals,
        )
        .await
        .context("Failed to load effective SpellAuraRestrictions authority")?,
    );
    info!(
        "Loaded {} spell aura restriction rows",
        spell_aura_restrictions_store.len()
    );
    let spell_casting_requirements_store = Arc::new(
        crate::spell::core_db2_hotfix::load_spell_casting_requirements_store_like_cpp(
            data_dir,
            locale,
            persistence,
            removals,
        )
        .await
        .context("Failed to load effective SpellCastingRequirements authority")?,
    );
    info!(
        "Loaded {} spell casting requirement rows",
        spell_casting_requirements_store.len()
    );
    let spell_class_options_store = Arc::new(
        wow_data::SpellClassOptionsStore::load(data_dir, locale)
            .context("Failed to load SpellClassOptions.db2")?,
    );
    info!(
        "Loaded {} spell class options rows",
        spell_class_options_store.len()
    );
    // C++ `sSpellMgr` label authority, read by `SpellInfo::HasLabel`
    // (`SpellInfo.cpp`) for the damage-taken-from-caster-by-label term.
    let spell_label_store = Arc::new(
        wow_data::SpellLabelStore::load(data_dir, locale)
            .context("Failed to load SpellLabel.db2")?,
    );
    info!("Loaded {} spell label rows", spell_label_store.len());
    let spell_equipped_items_store = Arc::new(
        crate::spell::core_db2_hotfix::load_spell_equipped_items_store_like_cpp(
            data_dir,
            locale,
            persistence,
            removals,
        )
        .await
        .context("Failed to load effective SpellEquippedItems authority")?,
    );
    info!(
        "Loaded {} spell equipped items rows",
        spell_equipped_items_store.len()
    );
    let spell_target_restrictions_store = Arc::new(
        crate::spell::core_db2_hotfix::load_spell_target_restrictions_store_like_cpp(
            data_dir,
            locale,
            persistence,
            removals,
        )
        .await
        .context("Failed to load effective SpellTargetRestrictions authority")?,
    );
    info!(
        "Loaded {} spell target restriction rows",
        spell_target_restrictions_store.len()
    );
    let spell_misc_store = Arc::new(
        crate::spell::core_db2_hotfix::load_spell_misc_store_like_cpp(
            data_dir,
            locale,
            persistence,
            removals,
        )
        .await
        .context("Failed to load effective SpellMisc authority")?,
    );
    info!(
        "Loaded {} effective spell misc rows",
        spell_misc_store.len()
    );

    Ok(SpellInfoStartup {
        spell_category_store,
        spell_aura_options_store,
        spell_aura_restrictions_store,
        spell_casting_requirements_store,
        spell_class_options_store,
        spell_label_store,
        spell_equipped_items_store,
        spell_target_restrictions_store,
        spell_misc_store,
    })
}
