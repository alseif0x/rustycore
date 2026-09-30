use crate::catalogs;
use anyhow::Context;
use std::sync::Arc;
use tracing::info;

pub(super) struct ItemAuxiliaryCatalogs {
    pub(super) spell_enchant_proc_outcome: wow_data::SpellEnchantProcLoadOutcomeLikeCpp,
    pub(super) gem_properties_store: Arc<wow_data::GemPropertiesStore>,
    pub(super) spell_item_enchantment_condition_store:
        Arc<wow_data::SpellItemEnchantmentConditionStore>,
    pub(super) spell_item_enchantment_store: Arc<wow_data::SpellItemEnchantmentStore>,
    pub(super) item_random_enchantment_template_store:
        Arc<wow_data::ItemRandomEnchantmentTemplateStore>,
    pub(super) item_disenchant_loot_store: Arc<wow_data::ItemDisenchantLootStore>,
    pub(super) rand_prop_points_store: Arc<wow_data::RandPropPointsStore>,
    pub(super) item_spec_override_store: Arc<wow_data::ItemSpecOverrideStore>,
    pub(super) item_random_properties_store: Arc<wow_data::ItemRandomPropertiesStore>,
    pub(super) item_random_suffix_store: Arc<wow_data::ItemRandomSuffixStore>,
    pub(super) lock_store: Arc<wow_data::LockStore>,
    pub(super) item_effect_store: Arc<wow_data::ItemEffectStore>,
    pub(super) durability_quality_store: Arc<wow_data::DurabilityQualityStore>,
    pub(super) durability_costs_store: Arc<wow_data::DurabilityCostsStore>,
}

pub(super) async fn load_item_auxiliary_catalogs(
    data_dir: &str,
    locale: &str,
    world_db: &Arc<wow_database::WorldDatabase>,
    static_data_overlay_persistence: &dyn wow_persistence::StaticDataOverlayPersistencePortLikeCpp,
) -> anyhow::Result<ItemAuxiliaryCatalogs> {
    // C++ global DB2 stores used by Item::CalculateDurabilityRepairCost.
    let durability_costs_store = Arc::new(
        wow_data::DurabilityCostsStore::load(&data_dir, &locale)
            .context("Failed to load DurabilityCosts.db2 — check DataDir and DBC.Locale config")?,
    );
    info!(
        "Loaded {} durability cost rows from DurabilityCosts.db2",
        durability_costs_store.len()
    );

    let durability_quality_store = Arc::new(
        wow_data::DurabilityQualityStore::load(&data_dir, &locale).context(
            "Failed to load DurabilityQuality.db2 — check DataDir and DBC.Locale config",
        )?,
    );
    info!(
        "Loaded {} durability quality rows from DurabilityQuality.db2",
        durability_quality_store.len()
    );

    let item_effect_store = Arc::new(
        wow_data::ItemEffectStore::load(&data_dir, &locale)
            .context("Failed to load ItemEffect.db2 — check DataDir and DBC.Locale config")?,
    );
    info!(
        "Loaded {} item effects from ItemEffect.db2",
        item_effect_store.len()
    );

    // Load Lock.db2 for C++ sLockStore existence checks during CMSG_OPEN_ITEM.
    let lock_store = Arc::new(
        wow_data::LockStore::load(&data_dir, &locale)
            .context("Failed to load Lock.db2 — check DataDir and DBC.Locale config")?,
    );
    info!("Loaded {} locks from Lock.db2", lock_store.len());

    // Load ItemRandomSuffix.db2 for C++ ApplyEnchantment random suffix amount resolution.
    let item_random_suffix_store = Arc::new(
        wow_data::ItemRandomSuffixStore::load(&data_dir, &locale)
            .context("Failed to load ItemRandomSuffix.db2 — check DataDir and DBC.Locale config")?,
    );
    info!(
        "Loaded {} item random suffixes from ItemRandomSuffix.db2",
        item_random_suffix_store.len()
    );

    // Load ItemRandomProperties.db2 and RandPropPoints.db2 plus the world-table
    // random enchantment groups for C++ ItemEnchantmentMgr::GenerateRandomProperties.
    let item_random_properties_store = Arc::new(
        wow_data::ItemRandomPropertiesStore::load(&data_dir, &locale).context(
            "Failed to load ItemRandomProperties.db2 — check DataDir and DBC.Locale config",
        )?,
    );
    info!(
        "Loaded {} item random properties from ItemRandomProperties.db2",
        item_random_properties_store.len()
    );

    // Load ItemSpecOverride.db2 for C++ ObjectMgr::LoadItemTemplates ItemSpecClassMask primary path.
    let item_spec_override_store = Arc::new(
        wow_data::ItemSpecOverrideStore::load(&data_dir, &locale)
            .context("Failed to load ItemSpecOverride.db2 — check DataDir and DBC.Locale config")?,
    );
    info!(
        "Loaded {} item spec overrides from ItemSpecOverride.db2",
        item_spec_override_store.len()
    );

    let rand_prop_points_store = Arc::new(
        wow_data::RandPropPointsStore::load(&data_dir, &locale)
            .context("Failed to load RandPropPoints.db2 — check DataDir and DBC.Locale config")?,
    );
    info!(
        "Loaded {} random property point rows from RandPropPoints.db2",
        rand_prop_points_store.len()
    );

    // Load ItemDisenchantLoot.db2 for C++ sItemDisenchantLootStore lookup.
    let item_disenchant_loot_store = Arc::new(
        wow_data::ItemDisenchantLootStore::load(&data_dir, &locale).context(
            "Failed to load ItemDisenchantLoot.db2 — check DataDir and DBC.Locale config",
        )?,
    );
    info!(
        "Loaded {} item disenchant loot rows from ItemDisenchantLoot.db2",
        item_disenchant_loot_store.len()
    );

    let item_random_enchantment_persistence =
        wow_database::MariaDbItemRandomEnchantmentCatalogPersistenceAdapterLikeCpp::new(
            Arc::clone(world_db),
        );
    let item_random_enchantment_template_store = Arc::new(
        catalogs::item_random_enchantment::load_item_random_enchantment_store_like_cpp(
            &item_random_enchantment_persistence,
            &item_random_properties_store,
            &item_random_suffix_store,
        )
        .await
        .context("Failed to load item_random_enchantment_template")?,
    );

    // Load SpellItemEnchantment.db2 for ApplyEnchantment and arena enchantment checks.
    let spell_item_enchantment_store = Arc::new(
        wow_data::SpellItemEnchantmentStore::load(&data_dir, &locale).context(
            "Failed to load SpellItemEnchantment.db2 — check DataDir and DBC.Locale config",
        )?,
    );
    info!(
        "Loaded {} spell item enchantments from SpellItemEnchantment.db2",
        spell_item_enchantment_store.len()
    );
    let spell_item_enchantment_condition_store = Arc::new(
        wow_data::SpellItemEnchantmentConditionStore::load(&data_dir, &locale).context(
            "Failed to load SpellItemEnchantmentCondition.db2 — check DataDir and DBC.Locale config",
        )?,
    );
    info!(
        "Loaded {} spell item enchantment conditions from SpellItemEnchantmentCondition.db2",
        spell_item_enchantment_condition_store.len()
    );
    let gem_properties_store = Arc::new(
        wow_data::GemPropertiesStore::load(&data_dir, &locale)
            .context("Failed to load GemProperties.db2 — check DataDir and DBC.Locale config")?,
    );
    info!(
        "Loaded {} gem properties from GemProperties.db2",
        gem_properties_store.len()
    );
    let spell_enchant_proc_outcome =
        crate::static_data_overlay::load_spell_enchant_proc_store_like_cpp(
            static_data_overlay_persistence,
            spell_item_enchantment_store.as_ref(),
        )
        .await
        .context("Failed to load C++ spell_enchant_proc_data rows")?;
    info!(
        "Loaded {} C++ spell_enchant_proc_data rows ({} missing enchantments)",
        spell_enchant_proc_outcome.loaded_row_count,
        spell_enchant_proc_outcome.errors.len()
    );

    Ok(ItemAuxiliaryCatalogs {
        durability_costs_store,
        durability_quality_store,
        item_effect_store,
        lock_store,
        item_random_suffix_store,
        item_random_properties_store,
        item_spec_override_store,
        rand_prop_points_store,
        item_disenchant_loot_store,
        item_random_enchantment_template_store,
        spell_item_enchantment_store,
        spell_item_enchantment_condition_store,
        gem_properties_store,
        spell_enchant_proc_outcome,
    })
}
