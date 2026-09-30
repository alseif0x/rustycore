//! Creature templates, effective difficulty and appearance startup catalogs.

use std::sync::Arc;

use anyhow::Context;
use tracing::info;

use crate::{bootstrap::world_config_f32, hotfix};

pub(super) struct CreatureTemplateCatalogs {
    pub(super) creature_template_classification_store: Arc<wow_data::CreatureTemplateClassificationStoreLikeCpp>,
    pub(super) creature_template_lifecycle_store: Arc<wow_data::CreatureTemplateLifecycleStoreLikeCpp>,
    pub(super) creature_template_sparring_store: Arc<wow_data::CreatureTemplateSparringStoreLikeCpp>,
}

pub(super) async fn load_creature_template_catalogs(
    world_object_catalog_persistence: &dyn wow_persistence::WorldObjectCatalogPersistencePortLikeCpp,
) -> anyhow::Result<CreatureTemplateCatalogs> {
    let creature_template_classification_store = Arc::new(
        crate::world::object_catalog::load_creature_classifications_like_cpp(
            world_object_catalog_persistence,
        )
            .await
            .context("Failed to load creature_template classifications for C++ creature difficulty damage rates")?,
    );
    let creature_template_lifecycle_store = Arc::new(
        crate::world::object_catalog::load_creature_templates_like_cpp(
            world_object_catalog_persistence,
        )
            .await
            .context("Failed to load DB-backed creature_template lifecycle rows for C++ Creature::LoadFromDB")?,
    );
    info!(
        "Loaded {} DB-backed creature_template lifecycle rows for loaded-grid Creature::LoadFromDB",
        creature_template_lifecycle_store.len()
    );
    let creature_template_sparring_store = Arc::new(
        crate::world::object_catalog::load_creature_sparring_like_cpp(
            world_object_catalog_persistence,
            creature_template_lifecycle_store.as_ref(),
        )
        .await
        .context("Failed to load creature_template_sparring rows for C++ Creature::LoadCreaturesSparringHealth")?,
    );
    info!(
        "Loaded {} creature template sparring rows",
        creature_template_sparring_store.len()
    );
    Ok(CreatureTemplateCatalogs {
        creature_template_classification_store,
        creature_template_lifecycle_store,
        creature_template_sparring_store,
    })
}

pub(super) struct CreatureRuntimeCatalogs {
    pub(super) creature_display_info_extra_store: Arc<wow_data::CreatureDisplayInfoExtraStore>,
    pub(super) creature_model_info_store: Arc<wow_data::CreatureModelInfoStoreLikeCpp>,
    pub(super) creature_model_data_store: Arc<wow_data::CreatureModelDataStore>,
    pub(super) creature_display_info_store: Arc<wow_data::CreatureDisplayInfoStore>,
    pub(super) creature_display_hotfix_persistence: wow_database::MariaDbCreatureDisplayHotfixPersistenceAdapterLikeCpp,
    pub(super) creature_template_mount_store: Arc<wow_data::CreatureTemplateMountStoreLikeCpp>,
    pub(super) creature_base_stats_store: Arc<wow_data::CreatureBaseStatsStoreLikeCpp>,
    pub(super) creature_difficulty_store: Arc<wow_data::CreatureDifficultyStoreLikeCpp>,
    pub(super) difficulty_store: Arc<wow_data::DifficultyStore>,
    pub(super) difficulty_hotfix_persistence: wow_database::MariaDbDifficultyHotfixPersistenceAdapterLikeCpp,
    pub(super) creature_health_rates: wow_data::CreatureClassificationHealthRatesLikeCpp,
    pub(super) creature_damage_rates: wow_data::CreatureClassificationDamageRatesLikeCpp,
}

pub(super) async fn load_creature_runtime_catalogs(
    data_dir: &str,
    locale: &str,
    hotfix_db: &Arc<wow_database::HotfixDatabase>,
    db2_hotfix_removals: &wow_data::Db2HotfixRemovalStoreLikeCpp,
    world_object_catalog_persistence: &dyn wow_persistence::WorldObjectCatalogPersistencePortLikeCpp,
    creature_template_classification_store: &wow_data::CreatureTemplateClassificationStoreLikeCpp,
    world_configs: &wow_config::WorldConfigSet,
) -> anyhow::Result<CreatureRuntimeCatalogs> {
    let creature_damage_rates = wow_data::CreatureClassificationDamageRatesLikeCpp {
        normal: world_config_f32(world_configs, "Rate.Creature.Damage.Normal", 1.0),
        elite: world_config_f32(world_configs, "Rate.Creature.Damage.Elite", 1.0),
        rare_elite: world_config_f32(world_configs, "Rate.Creature.Damage.RareElite", 1.0),
        obsolete: world_config_f32(world_configs, "Rate.Creature.Damage.Obsolete", 1.0),
        rare: world_config_f32(world_configs, "Rate.Creature.Damage.Rare", 1.0),
        trivial: world_config_f32(world_configs, "Rate.Creature.Damage.Trivial", 1.0),
        minus_mob: world_config_f32(world_configs, "Rate.Creature.Damage.MinusMob", 1.0),
    };
    let creature_health_rates = wow_data::CreatureClassificationHealthRatesLikeCpp {
        normal: world_config_f32(world_configs, "Rate.Creature.Health.Normal", 1.0),
        elite: world_config_f32(world_configs, "Rate.Creature.Health.Elite", 1.0),
        rare_elite: world_config_f32(world_configs, "Rate.Creature.Health.RareElite", 1.0),
        obsolete: world_config_f32(world_configs, "Rate.Creature.Health.Obsolete", 1.0),
        rare: world_config_f32(world_configs, "Rate.Creature.Health.Rare", 1.0),
        trivial: world_config_f32(world_configs, "Rate.Creature.Health.Trivial", 1.0),
        minus_mob: world_config_f32(world_configs, "Rate.Creature.Health.MinusMob", 1.0),
    };
    let difficulty_hotfix_persistence =
        wow_database::MariaDbDifficultyHotfixPersistenceAdapterLikeCpp::new(Arc::clone(hotfix_db));
    let difficulty_store = Arc::new(
        hotfix::difficulty::load_difficulty_store_like_cpp(
            data_dir,
            locale,
            &difficulty_hotfix_persistence,
            db2_hotfix_removals,
        )
        .await
        .context(
            "Failed to load effective Difficulty store — check DataDir and DBC.Locale config",
        )?,
    );
    info!(
        "Loaded {} effective difficulties from Difficulty.db2 and SQL overlays",
        difficulty_store.len()
    );
    let creature_difficulty_store = Arc::new(
        crate::world::object_catalog::load_creature_difficulties_like_cpp(
            world_object_catalog_persistence,
            &difficulty_store,
            |entry| {
                // C++ missing-template rows are skipped before insertion. This data-wiring
                // slice does not invent full templates; if the minimal classification row is
                // absent, fall back to classification 1 (elite), matching
                // Creature::GetDamageMod's default switch rate.
                let classification = creature_template_classification_store
                    .classification_for_entry(entry)
                    .unwrap_or(1);
                creature_damage_rates.modifier_for_classification_like_cpp(classification)
            },
        )
        .await
        .context(
            "Failed to load creature_template_difficulty rows with C++ classification damage rates",
        )?,
    );
    let creature_base_stats_store = Arc::new(
        crate::world::object_catalog::load_creature_base_stats_like_cpp(
            world_object_catalog_persistence,
        )
        .await
        .context("Failed to load creature_classlevelstats rows")?,
    );
    info!(
        "Loaded C++ creature runtime data stores: {} template classifications, {} difficulty rows, {} base stat rows",
        creature_template_classification_store.len(),
        creature_difficulty_store.len(),
        creature_base_stats_store.len()
    );
    let creature_template_mount_store = Arc::new(
        crate::world::object_catalog::load_creature_mounts_like_cpp(
            world_object_catalog_persistence,
        )
        .await
        .context("Failed to load creature_template mount fallback rows")?,
    );
    info!(
        "Loaded {} creature template mount fallback rows",
        creature_template_mount_store.len()
    );
    let creature_display_hotfix_persistence =
        wow_database::MariaDbCreatureDisplayHotfixPersistenceAdapterLikeCpp::new(Arc::clone(
            hotfix_db,
        ));
    let creature_display_info_store = Arc::new(
        hotfix::creature_display::load_creature_display_info_store_like_cpp(
            data_dir,
            locale,
            &creature_display_hotfix_persistence,
        )
        .await
        .context("Failed to load CreatureDisplayInfo.db2 / hotfix rows")?,
    );
    info!(
        "Loaded {} creature display info rows",
        creature_display_info_store.len()
    );
    let creature_model_data_store = Arc::new(
        hotfix::creature_display::load_creature_model_data_store_like_cpp(
            data_dir,
            locale,
            &creature_display_hotfix_persistence,
        )
        .await
        .context("Failed to load CreatureModelData.db2 / hotfix rows")?,
    );
    info!(
        "Loaded {} creature model data rows",
        creature_model_data_store.len()
    );
    let creature_model_info_store = Arc::new(
        crate::world::object_catalog::load_creature_model_info_like_cpp(
            world_object_catalog_persistence,
            creature_display_info_store.as_ref(),
            creature_model_data_store.as_ref(),
        )
        .await
        .context("Failed to load creature_model_info rows")?,
    );
    info!(
        "Loaded {} creature model info rows",
        creature_model_info_store.len()
    );
    let creature_display_info_extra_store = Arc::new(
        wow_data::CreatureDisplayInfoExtraStore::load(data_dir, locale)
            .context("Failed to load CreatureDisplayInfoExtra.db2")?,
    );
    info!(
        "Loaded {} creature display info extra rows",
        creature_display_info_extra_store.len()
    );
    Ok(CreatureRuntimeCatalogs {
        creature_damage_rates,
        creature_health_rates,
        difficulty_hotfix_persistence,
        difficulty_store,
        creature_difficulty_store,
        creature_base_stats_store,
        creature_template_mount_store,
        creature_display_hotfix_persistence,
        creature_display_info_store,
        creature_model_data_store,
        creature_model_info_store,
        creature_display_info_extra_store,
    })
}
