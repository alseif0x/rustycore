//! Ordered presentation startup composition.

use anyhow::Context;
use std::sync::Arc;
use tracing::info;

pub(super) struct PresentationCatalogs {
    pub(super) gameobject_display_info_store: Arc<wow_data::GameObjectDisplayInfoStore>,
    pub(super) cfg_categories_store: wow_data::CfgCategoriesStore,
    pub(super) movie_store: Arc<wow_data::MovieStore>,
    pub(super) anim_kit_store: Arc<wow_data::AnimKitStore>,
    pub(super) emotes_text_store: Arc<wow_data::EmotesTextStore>,
}

pub(super) fn load(data_dir: &str, locale: &str) -> anyhow::Result<PresentationCatalogs> {
    let emotes_text_store = Arc::new(
        wow_data::EmotesTextStore::load(data_dir, locale)
            .context("Failed to load EmotesText.db2")?,
    );
    info!("Loaded {} emote text rows", emotes_text_store.len());
    let anim_kit_store = Arc::new(
        wow_data::AnimKitStore::load(data_dir, locale).context("Failed to load AnimKit.db2")?,
    );
    info!("Loaded {} anim kit rows", anim_kit_store.len());
    let movie_store =
        Arc::new(wow_data::MovieStore::load(data_dir, locale).context("Failed to load Movie.db2")?);
    info!("Loaded {} movie rows", movie_store.len());
    let cfg_categories_store = wow_data::CfgCategoriesStore::load(data_dir, locale)
        .context("Failed to load Cfg_Categories.db2")?;
    info!(
        "Loaded {} realm categories from Cfg_Categories.db2",
        cfg_categories_store.len()
    );
    let gameobject_display_info_store = Arc::new(
        wow_data::GameObjectDisplayInfoStore::load(data_dir, locale)
            .context("Failed to load GameObjectDisplayInfo.db2")?,
    );
    info!(
        "Loaded {} gameobject display info rows",
        gameobject_display_info_store.len()
    );
    Ok(PresentationCatalogs {
        emotes_text_store,
        anim_kit_store,
        movie_store,
        cfg_categories_store,
        gameobject_display_info_store,
    })
}
