//! World modules.
//!
//! Grouped from the world_* siblings under #695; each module keeps its
//! items and its re-exported names.

pub mod auxiliary_catalog_adapter;
pub mod object_catalog_adapter;
pub mod query_catalog_adapter;
pub mod reference_catalog_adapter;
pub mod state_startup_adapter;

pub use auxiliary_catalog_adapter::*;
pub use object_catalog_adapter::*;
pub use query_catalog_adapter::*;
pub use reference_catalog_adapter::*;
pub use state_startup_adapter::*;

/// #1436: every world catalog adapter decodes the complete live world DB
/// with the C++ getter widths and fails closed on none of its rows.
#[cfg(test)]
pub(crate) mod live_tests {
    use std::sync::Arc;

    use wow_persistence::*;

    use super::*;
    use anyhow::{Context, Result};

    use crate::WorldDatabase;
    use crate::field_like_cpp::NativeFieldLikeCpp;
    use crate::result::collect_rows_like_cpp;

    /// Read-only MariaDB pool for the `#[ignore]` live probes: a single
    /// connection that never recycles, switched to `READ ONLY` before use.
    async fn live_world_probe_pool_like_cpp() -> Result<sqlx::MySqlPool> {
        let user = std::env::var("RUSTYCORE_DB_IT_USER")
            .context("RUSTYCORE_DB_IT_USER is required for the explicit live probe")?;
        let database = std::env::var("RUSTYCORE_DB_IT_WORLD_DB")
            .context("RUSTYCORE_DB_IT_WORLD_DB is required for the explicit live probe")?;
        let mut options = sqlx::mysql::MySqlConnectOptions::new()
            .username(&user)
            .password(&std::env::var("RUSTYCORE_DB_IT_PASS").unwrap_or_default())
            .host(&std::env::var("RUSTYCORE_DB_IT_HOST").unwrap_or_else(|_| "127.0.0.1".into()))
            .port(
                std::env::var("RUSTYCORE_DB_IT_PORT")
                    .unwrap_or_else(|_| "3306".into())
                    .parse()?,
            )
            .database(&database);
        if let Ok(socket) = std::env::var("RUSTYCORE_DB_IT_SOCKET") {
            options = options.socket(socket);
        }
        let pool = sqlx::mysql::MySqlPoolOptions::new()
            .max_connections(1)
            .min_connections(1)
            .idle_timeout(None)
            .max_lifetime(None)
            .connect_with(options)
            .await?;
        sqlx::query("SET SESSION TRANSACTION READ ONLY")
            .execute(&pool)
            .await?;
        Ok(pool)
    }

    /// Run one SELECT on the read-only probe pool and decode its rows.
    pub(crate) async fn live_world_probe_rows_like_cpp(
        sql: &str,
    ) -> Result<Vec<Vec<NativeFieldLikeCpp>>> {
        let pool = live_world_probe_pool_like_cpp().await?;
        let world_db = WorldDatabase::from_pool(pool);
        let rows = collect_rows_like_cpp(world_db.direct_query(sql).await?, "probe", |fields| {
            Ok(fields.to_vec())
        });
        world_db.pool().close().await;
        rows
    }

    fn loaded<T>(name: &str, outcome: WorldObjectRowsLoadOutcomeLikeCpp<T>) -> T {
        match outcome {
            WorldObjectRowsLoadOutcomeLikeCpp::Loaded(rows) => rows,
            WorldObjectRowsLoadOutcomeLikeCpp::Failed { reason } => panic!("{name}: {reason}"),
        }
    }

    fn query_loaded<T>(name: &str, outcome: WorldQueryCatalogLoadOutcomeLikeCpp<T>) -> T {
        match outcome {
            WorldQueryCatalogLoadOutcomeLikeCpp::Loaded(rows) => rows,
            WorldQueryCatalogLoadOutcomeLikeCpp::Failed { reason } => panic!("{name}: {reason}"),
        }
    }

    /// Every adapter query ran on the single `READ ONLY` probe connection.
    async fn assert_read_only_session(world_db: &WorldDatabase) {
        let rows = collect_rows_like_cpp(
            world_db
                .direct_query("SELECT @@session.tx_read_only")
                .await
                .unwrap(),
            "session",
            |fields| Ok(fields.to_vec()),
        )
        .unwrap();
        assert!(
            matches!(
                rows[0][0],
                NativeFieldLikeCpp::Int64(1)
                    | NativeFieldLikeCpp::Int32(1)
                    | NativeFieldLikeCpp::Int8(1)
            ),
            "{rows:?}"
        );
    }

    #[tokio::test]
    #[ignore = "read-only MariaDB probe; set RUSTYCORE_DB_IT_USER, RUSTYCORE_DB_IT_WORLD_DB and optional HOST/PORT/PASS/SOCKET"]
    async fn live_world_catalog_adapters_decode_every_row_like_cpp() {
        let pool = live_world_probe_pool_like_cpp().await.unwrap();
        let world_db = Arc::new(WorldDatabase::from_pool(pool));
        assert_read_only_session(&world_db).await;

        let query = MariaDbWorldQueryCatalogPersistenceAdapterLikeCpp::new(Arc::clone(&world_db));
        let creatures = query_loaded(
            "creature query catalog",
            query.load_creature_query_catalog_like_cpp().await,
        );
        assert!(creatures.templates.iter().any(|row| row.creature_type != 0));
        assert!(
            creatures
                .templates
                .iter()
                .any(|row| row.classification != 0)
        );
        assert!(
            creatures
                .templates
                .iter()
                .any(|row| row.kill_credits[0] != 0)
        );
        assert!(creatures.templates.iter().any(|row| row.movement_id != 0));
        query_loaded(
            "gameobject query catalog",
            query.load_gameobject_query_catalog_like_cpp().await,
        );
        query_loaded(
            "page text catalog",
            query.load_page_text_catalog_like_cpp().await,
        );

        let object = MariaDbWorldObjectCatalogPersistenceAdapterLikeCpp::new(Arc::clone(&world_db));
        loaded(
            "classifications",
            object.load_creature_classification_rows_like_cpp().await,
        );
        let templates = loaded(
            "creature templates",
            object.load_creature_template_rows_like_cpp().await,
        );
        assert!(
            templates
                .templates
                .iter()
                .any(|row| row.flight_movement_type != 0)
        );
        assert!(templates.templates.iter().any(|row| !row.swim_allowed));
        assert!(templates.templates.iter().any(|row| row.damage_school != 0));
        assert!(templates.templates.iter().any(|row| row.family != 0));
        loaded(
            "sparring",
            object.load_creature_sparring_rows_like_cpp().await,
        );
        let gameobjects = loaded(
            "gameobject templates",
            object.load_gameobject_template_rows_like_cpp().await,
        );
        assert!(
            gameobjects
                .templates
                .iter()
                .any(|row| row.data[1] == u32::MAX)
        );
        loaded(
            "gameobject overrides",
            object.load_gameobject_override_rows_like_cpp().await,
        );
        loaded(
            "difficulty",
            object.load_creature_difficulty_rows_like_cpp().await,
        );
        loaded(
            "base stats",
            object.load_creature_base_stats_rows_like_cpp().await,
        );
        loaded("mounts", object.load_creature_mount_rows_like_cpp().await);
        loaded(
            "model info",
            object.load_creature_model_info_rows_like_cpp().await,
        );
        let addons = loaded(
            "creature addons",
            object.load_creature_addon_rows_like_cpp().await,
        );
        assert!(
            addons
                .template_addons
                .iter()
                .any(|row| row.ai_anim_kit != 0)
        );
        loaded(
            "equipment",
            object.load_creature_equipment_rows_like_cpp().await,
        );

        let auxiliary =
            MariaDbWorldAuxiliaryCatalogPersistenceAdapterLikeCpp::new(Arc::clone(&world_db));
        let failures = [(
            "access_requirement",
            auxiliary.load_access_requirement_rows_like_cpp().await,
        )]
        .into_iter()
        .filter_map(|(name, outcome)| match outcome {
            WorldAuxiliaryRowsLoadOutcomeLikeCpp::Loaded(_) => None,
            WorldAuxiliaryRowsLoadOutcomeLikeCpp::Failed { reason } => {
                Some(format!("{name}: {reason}"))
            }
        })
        .chain(match auxiliary.load_trinity_string_rows_like_cpp().await {
            WorldAuxiliaryRowsLoadOutcomeLikeCpp::Loaded(_) => None,
            WorldAuxiliaryRowsLoadOutcomeLikeCpp::Failed { reason } => {
                Some(format!("trinity_string: {reason}"))
            }
        })
        .collect::<Vec<_>>();
        assert!(failures.is_empty(), "{failures:?}");
        assert_read_only_session(&world_db).await;
    }
}
