//! Composition boundary between SQLx-free mount rows and `wow-data` catalogs.

use anyhow::{Result, bail};
use wow_persistence::{MountCatalogLoadOutcomeLikeCpp, MountCatalogPersistencePortLikeCpp};

fn loaded_rows_like_cpp<T>(outcome: MountCatalogLoadOutcomeLikeCpp<T>) -> Result<Vec<T>> {
    match outcome {
        MountCatalogLoadOutcomeLikeCpp::Loaded(rows) => Ok(rows),
        MountCatalogLoadOutcomeLikeCpp::Failed { reason } => bail!(reason),
    }
}

async fn overlay_mount_store_like_cpp(
    mut store: wow_data::MountStore,
    persistence: &dyn MountCatalogPersistencePortLikeCpp,
) -> Result<(wow_data::MountStore, usize)> {
    let rows = loaded_rows_like_cpp(persistence.load_mount_hotfix_rows_like_cpp().await)?;
    let count =
        store.apply_hotfix_entries_like_cpp(rows.into_iter().map(|row| wow_data::MountEntry {
            id: row.id,
            mount_type_id: row.mount_type_id,
            flags: row.flags,
            source_type_enum: row.source_type_enum,
            source_spell_id: row.source_spell_id,
            player_condition_id: row.player_condition_id,
            mount_fly_ride_height: row.mount_fly_ride_height,
            ui_model_scene_id: row.ui_model_scene_id,
        }));
    Ok((store, count))
}

pub(super) async fn load_mount_store_like_cpp(
    data_dir: &str,
    locale: &str,
    persistence: &dyn MountCatalogPersistencePortLikeCpp,
) -> Result<(wow_data::MountStore, usize)> {
    overlay_mount_store_like_cpp(wow_data::MountStore::load(data_dir, locale)?, persistence).await
}

pub(super) async fn load_mount_definition_store_like_cpp(
    mount_store: &wow_data::MountStore,
    persistence: &dyn MountCatalogPersistencePortLikeCpp,
) -> Result<wow_data::MountDefinitionStoreLikeCpp> {
    let rows = loaded_rows_like_cpp(persistence.load_mount_definition_rows_like_cpp().await)?;
    Ok(wow_data::MountDefinitionStoreLikeCpp::from_rows_like_cpp(
        rows.into_iter()
            .map(|row| (row.spell_id, row.other_faction_spell_id)),
        mount_store,
    ))
}

pub(super) async fn load_mount_capability_store_like_cpp(
    data_dir: &str,
    locale: &str,
    persistence: &dyn MountCatalogPersistencePortLikeCpp,
) -> Result<(wow_data::MountCapabilityStore, usize)> {
    let mut store = wow_data::MountCapabilityStore::load(data_dir, locale)?;
    let rows = loaded_rows_like_cpp(
        persistence
            .load_mount_capability_hotfix_rows_like_cpp()
            .await,
    )?;
    let count = store.apply_hotfix_entries_like_cpp(rows.into_iter().map(|row| {
        wow_data::MountCapabilityEntry {
            id: row.id,
            flags: row.flags,
            req_riding_skill: row.req_riding_skill,
            req_area_id: row.req_area_id,
            req_spell_aura_id: row.req_spell_aura_id,
            req_spell_known_id: row.req_spell_known_id,
            mod_spell_aura_id: row.mod_spell_aura_id,
            req_map_id: row.req_map_id,
        }
    }));
    Ok((store, count))
}

pub(super) async fn load_mount_type_x_capability_store_like_cpp(
    data_dir: &str,
    locale: &str,
    persistence: &dyn MountCatalogPersistencePortLikeCpp,
) -> Result<(wow_data::MountTypeXCapabilityStore, usize)> {
    let mut store = wow_data::MountTypeXCapabilityStore::load(data_dir, locale)?;
    let rows = loaded_rows_like_cpp(
        persistence
            .load_mount_type_x_capability_hotfix_rows_like_cpp()
            .await,
    )?;
    let count = store.apply_hotfix_entries_like_cpp(rows.into_iter().map(|row| {
        wow_data::MountTypeXCapabilityEntry {
            id: row.id,
            mount_type_id: row.mount_type_id,
            mount_capability_id: row.mount_capability_id,
            order_index: row.order_index,
        }
    }));
    Ok((store, count))
}

pub(super) async fn load_mount_x_display_store_like_cpp(
    data_dir: &str,
    locale: &str,
    persistence: &dyn MountCatalogPersistencePortLikeCpp,
) -> Result<(wow_data::MountXDisplayStore, usize)> {
    let mut store = wow_data::MountXDisplayStore::load(data_dir, locale)?;
    let rows = loaded_rows_like_cpp(
        persistence
            .load_mount_x_display_hotfix_rows_like_cpp()
            .await,
    )?;
    let count = store.apply_hotfix_entries_like_cpp(rows.into_iter().map(|row| {
        wow_data::MountXDisplayEntry {
            id: row.id,
            creature_display_info_id: row.creature_display_info_id,
            player_condition_id: row.player_condition_id,
            mount_id: row.mount_id,
        }
    }));
    Ok((store, count))
}

#[cfg(test)]
mod tests {
    use super::*;
    use wow_persistence::{
        MountCapabilityHotfixRowLikeCpp, MountDefinitionRowLikeCpp, MountHotfixRowLikeCpp,
        MountTypeXCapabilityHotfixRowLikeCpp, MountXDisplayHotfixRowLikeCpp,
        PersistenceFutureLikeCpp,
    };

    #[derive(Clone)]
    struct FakeMountCatalogPersistenceLikeCpp {
        mounts: MountCatalogLoadOutcomeLikeCpp<MountHotfixRowLikeCpp>,
        definitions: MountCatalogLoadOutcomeLikeCpp<MountDefinitionRowLikeCpp>,
        capabilities: MountCatalogLoadOutcomeLikeCpp<MountCapabilityHotfixRowLikeCpp>,
        type_capabilities: MountCatalogLoadOutcomeLikeCpp<MountTypeXCapabilityHotfixRowLikeCpp>,
        displays: MountCatalogLoadOutcomeLikeCpp<MountXDisplayHotfixRowLikeCpp>,
    }

    impl MountCatalogPersistencePortLikeCpp for FakeMountCatalogPersistenceLikeCpp {
        fn load_mount_hotfix_rows_like_cpp(
            &self,
        ) -> PersistenceFutureLikeCpp<'_, MountCatalogLoadOutcomeLikeCpp<MountHotfixRowLikeCpp>>
        {
            Box::pin(async { self.mounts.clone() })
        }

        fn load_mount_definition_rows_like_cpp(
            &self,
        ) -> PersistenceFutureLikeCpp<'_, MountCatalogLoadOutcomeLikeCpp<MountDefinitionRowLikeCpp>>
        {
            Box::pin(async { self.definitions.clone() })
        }

        fn load_mount_capability_hotfix_rows_like_cpp(
            &self,
        ) -> PersistenceFutureLikeCpp<
            '_,
            MountCatalogLoadOutcomeLikeCpp<MountCapabilityHotfixRowLikeCpp>,
        > {
            Box::pin(async { self.capabilities.clone() })
        }

        fn load_mount_type_x_capability_hotfix_rows_like_cpp(
            &self,
        ) -> PersistenceFutureLikeCpp<
            '_,
            MountCatalogLoadOutcomeLikeCpp<MountTypeXCapabilityHotfixRowLikeCpp>,
        > {
            Box::pin(async { self.type_capabilities.clone() })
        }

        fn load_mount_x_display_hotfix_rows_like_cpp(
            &self,
        ) -> PersistenceFutureLikeCpp<
            '_,
            MountCatalogLoadOutcomeLikeCpp<MountXDisplayHotfixRowLikeCpp>,
        > {
            Box::pin(async { self.displays.clone() })
        }
    }

    fn persistence_like_cpp() -> FakeMountCatalogPersistenceLikeCpp {
        FakeMountCatalogPersistenceLikeCpp {
            mounts: MountCatalogLoadOutcomeLikeCpp::Loaded(Vec::new()),
            definitions: MountCatalogLoadOutcomeLikeCpp::Loaded(Vec::new()),
            capabilities: MountCatalogLoadOutcomeLikeCpp::Loaded(Vec::new()),
            type_capabilities: MountCatalogLoadOutcomeLikeCpp::Loaded(Vec::new()),
            displays: MountCatalogLoadOutcomeLikeCpp::Loaded(Vec::new()),
        }
    }

    #[tokio::test]
    async fn mount_hotfix_replaces_the_row_and_rebuilds_the_spell_index() {
        let base = wow_data::MountStore::from_entries([wow_data::MountEntry {
            id: 1,
            mount_type_id: 2,
            flags: 3,
            source_type_enum: 4,
            source_spell_id: 100,
            player_condition_id: 5,
            mount_fly_ride_height: 6.0,
            ui_model_scene_id: 7,
        }]);
        let mut persistence = persistence_like_cpp();
        persistence.mounts = MountCatalogLoadOutcomeLikeCpp::Loaded(vec![MountHotfixRowLikeCpp {
            id: 1,
            mount_type_id: 20,
            flags: 30,
            source_type_enum: 40,
            source_spell_id: 200,
            player_condition_id: 50,
            mount_fly_ride_height: 60.0,
            ui_model_scene_id: 70,
        }]);

        let (store, count) = overlay_mount_store_like_cpp(base, &persistence)
            .await
            .unwrap();
        assert_eq!(count, 1);
        assert!(store.get_by_source_spell_id_like_cpp(100).is_none());
        assert_eq!(
            store.get_by_source_spell_id_like_cpp(200),
            store.get_by_id(1)
        );
        assert_eq!(store.get_by_id(1).unwrap().mount_type_id, 20);
    }

    #[tokio::test]
    async fn definition_rows_are_validated_by_the_catalog_owner() {
        let mounts = wow_data::MountStore::from_entries([
            wow_data::MountEntry {
                id: 1,
                mount_type_id: 0,
                flags: 0,
                source_type_enum: 0,
                source_spell_id: 100,
                player_condition_id: 0,
                mount_fly_ride_height: 0.0,
                ui_model_scene_id: 0,
            },
            wow_data::MountEntry {
                id: 2,
                source_spell_id: 200,
                ..wow_data::MountEntry {
                    id: 1,
                    mount_type_id: 0,
                    flags: 0,
                    source_type_enum: 0,
                    source_spell_id: 100,
                    player_condition_id: 0,
                    mount_fly_ride_height: 0.0,
                    ui_model_scene_id: 0,
                }
            },
        ]);
        let mut persistence = persistence_like_cpp();
        persistence.definitions = MountCatalogLoadOutcomeLikeCpp::Loaded(vec![
            MountDefinitionRowLikeCpp {
                spell_id: 100,
                other_faction_spell_id: 200,
            },
            MountDefinitionRowLikeCpp {
                spell_id: 100,
                other_faction_spell_id: 0,
            },
            MountDefinitionRowLikeCpp {
                spell_id: 999,
                other_faction_spell_id: 0,
            },
        ]);

        let definitions = load_mount_definition_store_like_cpp(&mounts, &persistence)
            .await
            .unwrap();
        assert_eq!(definitions.len(), 1);
        assert_eq!(definitions.other_faction_spell_id_like_cpp(100), Some(0));
    }

    #[tokio::test]
    async fn persistence_failure_stops_before_overlay_mutation() {
        let base = wow_data::MountStore::from_entries([]);
        let mut persistence = persistence_like_cpp();
        persistence.mounts = MountCatalogLoadOutcomeLikeCpp::Failed {
            reason: "hotfix query failed".to_string(),
        };

        let result = overlay_mount_store_like_cpp(base, &persistence).await;
        let Err(error) = result else {
            panic!("failed persistence must stop catalog assembly");
        };
        assert_eq!(error.to_string(), "hotfix query failed");
    }
}
