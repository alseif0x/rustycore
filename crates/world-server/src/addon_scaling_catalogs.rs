//! Creature addon, condition reference and scaling catalogs produced during ordered startup.

use super::*;

pub(super) struct AddonScalingCatalogs {
    pub(super) spell_shapeshift_form_store: Arc<wow_data::SpellShapeshiftFormStore>,
    pub(super) creature_addon_store: Arc<wow_data::CreatureAddonStoreLikeCpp>,
    pub(super) active_event_store: Arc<wow_data::WorldIdStore>,
    pub(super) world_state_store: Arc<wow_data::WorldIdStore>,
    pub(super) trainer_store: Arc<wow_data::WorldIdStore>,
    pub(super) curve_store: Arc<wow_data::progression_rewards::CurveStore>,
    pub(super) curve_point_store: Arc<wow_data::progression_rewards::CurvePointStore>,
    pub(super) scaling_stat_distribution_store:
        Arc<wow_data::progression_rewards::ScalingStatDistributionStore>,
    pub(super) scaling_stat_values_store:
        Arc<wow_data::progression_rewards::ScalingStatValuesStore>,
}

pub(super) async fn load_addon_scaling_catalogs(
    data_dir: &str,
    locale: &str,
    world_object_catalog_persistence: &wow_database::MariaDbWorldObjectCatalogPersistenceAdapterLikeCpp,
    world_reference_catalog_persistence: &wow_database::MariaDbWorldReferenceCatalogPersistenceAdapterLikeCpp,
    creature_template_lifecycle_store: &wow_data::CreatureTemplateLifecycleStoreLikeCpp,
    creature_spawn_store: &wow_data::WorldSpawnIdStore,
    creature_display_info_store: &wow_data::CreatureDisplayInfoStore,
    emotes_store: &wow_data::EmotesStore,
    anim_kit_store: &wow_data::AnimKitStore,
    spell_store: &wow_data::SpellStore,
    spell_misc_store: &wow_data::SpellMiscStore,
    spell_duration_store: &wow_data::SpellDurationStore,
) -> Result<AddonScalingCatalogs> {
    let spell_shapeshift_form_store = Arc::new(
        wow_data::SpellShapeshiftFormStore::load(&data_dir, &locale)
            .context("Failed to load SpellShapeshiftForm.db2")?,
    );
    info!(
        "Loaded {} spell shapeshift form rows",
        spell_shapeshift_form_store.len()
    );
    let creature_addon_store = Arc::new(
        crate::world::object_catalog::load_creature_addons_like_cpp(
            world_object_catalog_persistence,
            creature_template_lifecycle_store,
            creature_spawn_store,
            creature_display_info_store,
            emotes_store,
            anim_kit_store,
            spell_store,
            spell_misc_store,
            spell_duration_store,
        )
        .await
        .context("Failed to load represented creature_addon / creature_template_addon rows for C++ Creature::LoadCreaturesAddon")?,
    );
    info!(
        "Loaded {} represented creature addon rows",
        creature_addon_store.len()
    );
    let active_event_store = Arc::new(
        crate::world::reference_catalog::load_world_id_store_like_cpp(
            world_reference_catalog_persistence,
            wow_persistence::WorldObjectIdCatalogKindLikeCpp::GameEvent,
        )
        .await
        .context("Failed to load game_event ids for C++ ConditionMgr validation")?,
    );
    let world_state_store = Arc::new(
        crate::world::reference_catalog::load_world_id_store_like_cpp(
            world_reference_catalog_persistence,
            wow_persistence::WorldObjectIdCatalogKindLikeCpp::WorldState,
        )
        .await
        .context("Failed to load world_state ids for C++ ConditionMgr validation")?,
    );
    info!(
        "Loaded condition validation world id stores: {} valid game events, {} world states",
        active_event_store.len(),
        world_state_store.len()
    );
    let trainer_store = Arc::new(
        crate::world::reference_catalog::load_world_id_store_like_cpp(
            world_reference_catalog_persistence,
            wow_persistence::WorldObjectIdCatalogKindLikeCpp::Trainer,
        )
        .await
        .context("Failed to load trainer ids for C++ ConditionMgr validation")?,
    );
    info!(
        "Loaded condition validation trainer id store: {} trainers",
        trainer_store.len()
    );
    let curve_store = Arc::new(
        wow_data::progression_rewards::CurveStore::load(&data_dir, &locale)
            .context("Failed to load Curve.db2 for C++ curve validation")?,
    );
    let curve_point_store = Arc::new(
        wow_data::progression_rewards::CurvePointStore::load(&data_dir, &locale)
            .context("Failed to load CurvePoint.db2 for C++ curve evaluation")?,
    );
    let scaling_stat_distribution_store = Arc::new(
        wow_data::progression_rewards::ScalingStatDistributionStore::load(&data_dir, &locale)
            .context(
                "Failed to load ScalingStatDistribution.db2 — check DataDir and DBC.Locale config",
            )?,
    );
    let scaling_stat_values_store = Arc::new(
        wow_data::progression_rewards::ScalingStatValuesStore::load(&data_dir, &locale).context(
            "Failed to load ScalingStatValues.db2 — check DataDir and DBC.Locale config",
        )?,
    );
    info!(
        "Loaded {} curves, {} curve points, {} scaling-stat distributions, and {} scaling-stat values from DB2",
        curve_store.len(),
        curve_point_store.len(),
        scaling_stat_distribution_store.len(),
        scaling_stat_values_store.len()
    );
    Ok(AddonScalingCatalogs {
        spell_shapeshift_form_store,
        creature_addon_store,
        active_event_store,
        world_state_store,
        trainer_store,
        curve_store,
        curve_point_store,
        scaling_stat_distribution_store,
        scaling_stat_values_store,
    })
}
