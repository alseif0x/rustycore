//! Composition boundary for the effective C++ Difficulty authority.

use anyhow::{Result, bail};
use wow_persistence::{
    DifficultyHotfixLoadOutcomeLikeCpp, DifficultyHotfixPersistencePortLikeCpp,
    DifficultyHotfixRowLikeCpp,
};

fn difficulty_entry_like_cpp(row: DifficultyHotfixRowLikeCpp) -> wow_data::DifficultyEntry {
    wow_data::DifficultyEntry {
        id: row.id,
        instance_type: row.instance_type,
        fallback_difficulty_id: row.fallback_difficulty_id,
        flags: row.flags,
        toggle_difficulty_id: row.toggle_difficulty_id,
    }
}

fn apply_loaded_hotfix_outcome_like_cpp(
    base: wow_data::DifficultyStore,
    outcome: DifficultyHotfixLoadOutcomeLikeCpp,
    removals: &wow_data::Db2HotfixRemovalStoreLikeCpp,
) -> Result<wow_data::DifficultyStore> {
    let rows = match outcome {
        DifficultyHotfixLoadOutcomeLikeCpp::Loaded(rows) => rows,
        DifficultyHotfixLoadOutcomeLikeCpp::Failed { reason } => bail!(reason),
    };
    base.apply_hotfix_overlays_like_cpp(
        rows.official.into_iter().map(difficulty_entry_like_cpp),
        rows.custom.into_iter().map(difficulty_entry_like_cpp),
        removals,
    )
}

pub(crate) async fn load_difficulty_store_like_cpp(
    data_dir: &str,
    locale: &str,
    persistence: &dyn DifficultyHotfixPersistencePortLikeCpp,
    removals: &wow_data::Db2HotfixRemovalStoreLikeCpp,
) -> Result<wow_data::DifficultyStore> {
    // C++ `LOAD_DB2`: WDC4 precedes official/custom SQL overlays.
    let base = wow_data::DifficultyStore::load(data_dir, locale)?;
    let outcome = persistence.load_difficulty_hotfix_rows_like_cpp().await;
    apply_loaded_hotfix_outcome_like_cpp(base, outcome, removals)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn typed_hotfix_row_preserves_every_consumed_difficulty_field() {
        assert_eq!(
            difficulty_entry_like_cpp(DifficultyHotfixRowLikeCpp {
                id: 1,
                instance_type: 2,
                fallback_difficulty_id: 3,
                flags: 4,
                toggle_difficulty_id: 5,
            }),
            wow_data::DifficultyEntry {
                id: 1,
                instance_type: 2,
                fallback_difficulty_id: 3,
                flags: 4,
                toggle_difficulty_id: 5,
            }
        );
    }

    #[test]
    fn failed_overlay_read_stops_before_domain_application() {
        let base = wow_data::DifficultyStore::from_entries([]);
        let result = apply_loaded_hotfix_outcome_like_cpp(
            base,
            DifficultyHotfixLoadOutcomeLikeCpp::Failed {
                reason: "difficulty hotfix read failed".into(),
            },
            &wow_data::Db2HotfixRemovalStoreLikeCpp::default(),
        );
        let error = match result {
            Ok(_) => panic!("failed overlay read must not publish a Difficulty store"),
            Err(error) => error,
        };
        assert_eq!(error.to_string(), "difficulty hotfix read failed");
    }

    #[test]
    fn app_composes_one_adapter_at_the_existing_difficulty_publication_point() {
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
            continuation.find(delegation).unwrap() < continuation.find("let emotes_store =").unwrap(),
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
            root.matches("MariaDbDifficultyHotfixPersistenceAdapterLikeCpp::new").count(),
            0
        );
        assert_eq!(root.matches("load_difficulty_store_like_cpp").count(), 0);
        assert_eq!(
            source
                .matches("MariaDbDifficultyHotfixPersistenceAdapterLikeCpp::new")
                .count(),
            1
        );
        assert_eq!(source.matches("load_difficulty_store_like_cpp").count(), 1);

        let loader_source = include_str!("difficulty.rs");
        let wdc4_load = loader_source
            .find("DifficultyStore::load(data_dir, locale)")
            .expect("composition must load the WDC4 authority");
        let overlay_load = loader_source
            .find("persistence.load_difficulty_hotfix_rows_like_cpp()")
            .expect("composition must invoke the typed overlay port");
        assert!(wdc4_load < overlay_load, "WDC4 must precede SQL overlays");
    }
}
