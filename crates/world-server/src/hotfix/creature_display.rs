//! Composition boundary for creature display/model DB2 Hotfix overlays.

use anyhow::{Result, bail};
use tracing::info;
use wow_persistence::{
    CreatureDisplayHotfixLoadOutcomeLikeCpp, CreatureDisplayHotfixPersistencePortLikeCpp,
    CreatureDisplayInfoHotfixRowLikeCpp, CreatureModelDataHotfixRowLikeCpp,
};

fn display_entry_like_cpp(
    row: CreatureDisplayInfoHotfixRowLikeCpp,
) -> wow_data::CreatureDisplayInfoEntry {
    wow_data::CreatureDisplayInfoEntry {
        id: row.id,
        model_id: row.model_id,
        extended_display_info_id: row.extended_display_info_id,
        creature_model_scale: row.creature_model_scale,
    }
}

fn model_entry_like_cpp(
    row: CreatureModelDataHotfixRowLikeCpp,
) -> wow_data::CreatureModelDataEntry {
    wow_data::CreatureModelDataEntry {
        id: row.id,
        flags: row.flags,
        file_data_id: row.file_data_id,
        collision_height: row.collision_height,
        hover_height: row.hover_height,
        model_scale: row.model_scale,
        mount_height: row.mount_height,
    }
}

fn loaded_rows_like_cpp<T>(outcome: CreatureDisplayHotfixLoadOutcomeLikeCpp<T>) -> Result<Vec<T>> {
    match outcome {
        CreatureDisplayHotfixLoadOutcomeLikeCpp::Loaded(rows) => Ok(rows),
        CreatureDisplayHotfixLoadOutcomeLikeCpp::Failed { reason } => bail!(reason),
    }
}

pub(crate) async fn load_creature_display_info_store_like_cpp(
    data_dir: &str,
    locale: &str,
    persistence: &dyn CreatureDisplayHotfixPersistencePortLikeCpp,
) -> Result<wow_data::CreatureDisplayInfoStore> {
    let mut store = wow_data::CreatureDisplayInfoStore::load(data_dir, locale)?;
    let rows = loaded_rows_like_cpp(persistence.load_creature_display_info_rows_like_cpp().await)?;
    let count = store.apply_hotfix_entries_like_cpp(rows.into_iter().map(display_entry_like_cpp));
    if count != 0 {
        info!("Loaded {count} CreatureDisplayInfo hotfix rows");
    }
    Ok(store)
}

pub(crate) async fn load_creature_model_data_store_like_cpp(
    data_dir: &str,
    locale: &str,
    persistence: &dyn CreatureDisplayHotfixPersistencePortLikeCpp,
) -> Result<wow_data::CreatureModelDataStore> {
    let mut store = wow_data::CreatureModelDataStore::load(data_dir, locale)?;
    let rows = loaded_rows_like_cpp(persistence.load_creature_model_data_rows_like_cpp().await)?;
    let count = store.apply_hotfix_entries_like_cpp(rows.into_iter().map(model_entry_like_cpp));
    if count != 0 {
        info!("Loaded {count} CreatureModelData hotfix rows");
    }
    Ok(store)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn typed_rows_preserve_every_consumed_domain_field() {
        assert_eq!(
            display_entry_like_cpp(CreatureDisplayInfoHotfixRowLikeCpp {
                id: 1,
                model_id: 2,
                extended_display_info_id: -3,
                creature_model_scale: 1.25,
            }),
            wow_data::CreatureDisplayInfoEntry {
                id: 1,
                model_id: 2,
                extended_display_info_id: -3,
                creature_model_scale: 1.25,
            }
        );
        assert_eq!(
            model_entry_like_cpp(CreatureModelDataHotfixRowLikeCpp {
                id: 4,
                flags: 5,
                file_data_id: 6,
                collision_height: 1.0,
                hover_height: 2.0,
                model_scale: 3.0,
                mount_height: 4.0,
            }),
            wow_data::CreatureModelDataEntry {
                id: 4,
                flags: 5,
                file_data_id: 6,
                collision_height: 1.0,
                hover_height: 2.0,
                model_scale: 3.0,
                mount_height: 4.0,
            }
        );
    }

    #[test]
    fn failed_stage_exposes_no_rows_for_domain_mutation() {
        assert!(
            loaded_rows_like_cpp::<CreatureDisplayInfoHotfixRowLikeCpp>(
                CreatureDisplayHotfixLoadOutcomeLikeCpp::Loaded(Vec::new())
            )
            .unwrap()
            .is_empty()
        );
        let result = loaded_rows_like_cpp::<CreatureDisplayInfoHotfixRowLikeCpp>(
            CreatureDisplayHotfixLoadOutcomeLikeCpp::Failed {
                reason: "display hotfix unavailable".to_owned(),
            },
        );
        assert_eq!(
            result.unwrap_err().to_string(),
            "display hotfix unavailable"
        );
    }

    #[test]
    fn app_composes_one_adapter_and_keeps_display_before_model() {
        let root = include_str!("../app.rs");
        let continuation = include_str!("../app/world_startup.rs");
        assert_eq!(root.matches("world_startup::run_world_startup(").count(), 1);
        assert_eq!(root.matches("world_template_startup::load(").count(), 0);
        let templates = include_str!("../app/world_template_startup.rs");
        let source = include_str!("../app/creature_catalog_startup.rs");
        let delegation = "world_template_startup::load(";
        assert_eq!(continuation.matches(delegation).count(), 1);
        assert_eq!(continuation.matches("let emotes_store =").count(), 1);
        assert!(
            continuation.find(delegation).unwrap()
                < continuation.find("let emotes_store =").unwrap(),
            "the complete world template phase must precede the original emotes load"
        );
        for call in [
            "creature_catalog_startup::load_creature_template_catalogs(",
            "creature_catalog_startup::load_creature_runtime_catalogs(",
        ] {
            assert_eq!(templates.matches(call).count(), 1);
            assert_eq!(root.matches(call).count(), 0);
            assert_eq!(continuation.matches(call).count(), 0);
        }
        let mut cursor = 0;
        for stage in [
            "creature_catalog_startup::load_creature_template_catalogs(",
            "load_gameobject_templates_like_cpp(",
            "load_gameobject_overrides_like_cpp(",
            "wow_data::build_template_script_name_interner_like_cpp(",
            "load_scene_templates_like_cpp(",
            "let _scene_template_store = Arc::new(scene_template_outcome.store);",
            "creature_catalog_startup::load_creature_runtime_catalogs(",
        ] {
            assert_eq!(templates.matches(stage).count(), 1);
            let offset = templates[cursor..]
                .find(stage)
                .unwrap_or_else(|| panic!("missing or reordered creature startup stage: {stage}"));
            cursor += offset + stage.len();
        }
        assert_eq!(
            root.matches("MariaDbCreatureDisplayHotfixPersistenceAdapterLikeCpp::new")
                .count(),
            0
        );
        for loader in [
            "load_creature_display_info_store_like_cpp",
            "load_creature_model_data_store_like_cpp",
        ] {
            assert_eq!(root.matches(loader).count(), 0);
            assert_eq!(source.matches(loader).count(), 1);
        }
        assert_eq!(
            source
                .matches("MariaDbCreatureDisplayHotfixPersistenceAdapterLikeCpp::new")
                .count(),
            1
        );
        let display = source
            .find("load_creature_display_info_store_like_cpp")
            .unwrap();
        let model = source
            .find("load_creature_model_data_store_like_cpp")
            .unwrap();
        assert!(display < model);

        let loader = include_str!("creature_display.rs");
        let display_wdc4 = loader.find("CreatureDisplayInfoStore::load").unwrap();
        let display_rows = loader
            .find("load_creature_display_info_rows_like_cpp")
            .unwrap();
        let display_apply = loader
            .find("apply_hotfix_entries_like_cpp(rows.into_iter().map(display_entry_like_cpp))")
            .unwrap();
        let model_wdc4 = loader.find("CreatureModelDataStore::load").unwrap();
        let model_rows = loader
            .find("load_creature_model_data_rows_like_cpp")
            .unwrap();
        let model_apply = loader
            .find("apply_hotfix_entries_like_cpp(rows.into_iter().map(model_entry_like_cpp))")
            .unwrap();
        assert!(display_wdc4 < display_rows && display_rows < display_apply);
        assert!(model_wdc4 < model_rows && model_rows < model_apply);
    }
}
