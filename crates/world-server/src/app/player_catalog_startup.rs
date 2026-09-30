//! Ordered Player identity, talent, glyph and power catalog startup.

use std::sync::Arc;

use anyhow::Context;
use tracing::info;

pub(super) struct PlayerCatalogs {
    pub(super) chr_race_x_chr_model_store: wow_data::character_progression::ChrRaceXChrModelStore,
    pub(super) chr_model_store: wow_data::character_progression::ChrModelStore,
    pub(super) power_type_store: Arc<wow_data::character_progression::PowerTypeStore>,
    pub(super) chr_classes_store: Arc<wow_data::character_progression::ChrClassesStore>,
    pub(super) chr_races_store: Arc<wow_data::character_progression::ChrRacesStore>,
    pub(super) glyph_properties_store: Arc<wow_data::GlyphPropertiesStore>,
    pub(super) num_talents_at_level_store: Arc<wow_data::progression_rewards::NumTalentsAtLevelStore>,
    pub(super) talent_tab_store: Arc<wow_data::TalentTabStore>,
    pub(super) talent_store: Arc<wow_data::TalentStore>,
    pub(super) skill_tiers_store: Arc<wow_data::SkillTiersStoreLikeCpp>,
}

pub(super) async fn load_player_catalogs(
    data_dir: &str,
    locale: &str,
    skill_world_rules_persistence: &dyn wow_persistence::SkillWorldRulesPersistencePortLikeCpp,
    static_data_overlay_persistence: &dyn wow_persistence::StaticDataOverlayPersistencePortLikeCpp,
) -> anyhow::Result<PlayerCatalogs> {
    let skill_tiers_store = Arc::new(
        crate::skill_world_rules::load_skill_tiers_store_like_cpp(skill_world_rules_persistence)
            .await
            .context("Failed to load world.skill_tiers")?,
    );
    let talent_store = Arc::new(
        wow_data::TalentStore::load(data_dir, locale).context("Failed to load Talent.db2")?,
    );
    let talent_tab_store = Arc::new(
        wow_data::TalentTabStore::load(data_dir, locale)
            .context("Failed to load TalentTab.db2")?,
    );
    let num_talents_at_level_store = Arc::new(
        wow_data::progression_rewards::NumTalentsAtLevelStore::load(data_dir, locale)
            .context("Failed to load NumTalentsAtLevel.db2")?,
    );
    let glyph_properties_store = Arc::new(
        wow_data::GlyphPropertiesStore::load(data_dir, locale)
            .context("Failed to load GlyphProperties.db2")?,
    );
    info!(
        "Loaded {} talent rows, {} talent tabs, {} talent-level rows, and {} glyph property rows from DB2",
        talent_store.len(),
        talent_tab_store.len(),
        num_talents_at_level_store.len(),
        glyph_properties_store.len()
    );
    let chr_races_store = Arc::new(
        wow_data::character_progression::ChrRacesStore::load(data_dir, locale)
            .context("Failed to load ChrRaces.db2")?,
    );
    info!(
        "Loaded {} race rows from ChrRaces.db2",
        chr_races_store.len()
    );
    // [M0.1/#14] ChrClasses powers the class→display-power map (creature stat setup)
    // and the per-class opening cinematic; without it both fall back to hardcoded
    // defaults. C++ sChrClassesStore (DB2Stores.cpp:94).
    let chr_classes_store = Arc::new(
        wow_data::character_progression::ChrClassesStore::load(data_dir, locale)
            .context("Failed to load ChrClasses.db2")?,
    );
    let power_type_store = Arc::new(
        crate::static_data_overlay::load_power_type_store_like_cpp(
            data_dir,
            locale,
            static_data_overlay_persistence,
        )
        .await
        .context("Failed to load PowerType.db2 / hotfix rows")?,
    );
    info!(
        "Loaded {} class rows and {} effective power-type rows from DB2/hotfixes",
        chr_classes_store.len(),
        power_type_store.len()
    );
    let chr_model_store = wow_data::character_progression::ChrModelStore::load(data_dir, locale)
        .context("Failed to load ChrModel.db2")?;
    let chr_race_x_chr_model_store =
        wow_data::character_progression::ChrRaceXChrModelStore::load(data_dir, locale)
            .context("Failed to load ChrRaceXChrModel.db2")?;
    info!(
        "Loaded {} character models and {} race/model links from DB2",
        chr_model_store.len(),
        chr_race_x_chr_model_store.len()
    );

    Ok(PlayerCatalogs {
        skill_tiers_store,
        talent_store,
        talent_tab_store,
        num_talents_at_level_store,
        glyph_properties_store,
        chr_races_store,
        chr_classes_store,
        power_type_store,
        chr_model_store,
        chr_race_x_chr_model_store,
    })
}

pub(super) struct SpecializationCatalog {
    pub(super) chr_specialization_store: Arc<wow_data::ChrSpecializationStore>,
    pub(super) chr_specialization_hotfix_persistence: wow_database::MariaDbChrSpecializationHotfixPersistenceAdapterLikeCpp,
}

pub(super) async fn load_specialization(
    data_dir: &str,
    locale: &str,
    hotfix_db: &Arc<wow_database::HotfixDatabase>,
    db2_hotfix_removals: &wow_data::Db2HotfixRemovalStoreLikeCpp,
) -> anyhow::Result<SpecializationCatalog> {
    // Load effective ChrSpecialization authority for C++ specialization validation.
    let chr_specialization_hotfix_persistence =
        wow_database::MariaDbChrSpecializationHotfixPersistenceAdapterLikeCpp::new(Arc::clone(
            hotfix_db,
        ));
    let chr_specialization_store = Arc::new(
        crate::hotfix::chr_specialization::load_chr_specialization_store_like_cpp(
            data_dir,
            locale,
            &chr_specialization_hotfix_persistence,
            db2_hotfix_removals,
        )
        .await
        .context(
            "Failed to load effective ChrSpecialization store — check DataDir and DBC.Locale config",
        )?,
    );
    info!(
        "Loaded {} effective chr specializations from ChrSpecialization.db2 and SQL overlays",
        chr_specialization_store.len()
    );
    Ok(SpecializationCatalog {
        chr_specialization_hotfix_persistence,
        chr_specialization_store,
    })
}
