//! Ordered database connections and read-only runtime schema validation.

use std::sync::Arc;

use anyhow::{Context, Result};
use tracing::info;
use wow_config::DatabaseInfo;
use wow_database::{CharacterDatabase, HotfixDatabase, LoginDatabase, WorldDatabase};

use crate::{database_pool_size_like_cpp, log_database_target_like_cpp};

pub(super) async fn open_primary_databases(
) -> Result<(LoginDatabase, CharacterDatabase, Arc<WorldDatabase>)> {
    // Connect to login database (needed for session key validation)
    let login_info = wow_config::get_database_info_default(
        "Login",
        DatabaseInfo::new("127.0.0.1", 3306, "trinity", "trinity", "auth"),
    );
    log_database_target_like_cpp("login", &login_info);

    let login_connection = wow_database::build_connection_string_with_ssl_like_cpp(
        &login_info.host,
        &login_info.port_or_socket,
        &login_info.username,
        &login_info.password,
        &login_info.database,
        login_info.ssl,
    );
    let login_db =
        LoginDatabase::open_with_pool_size(&login_connection, database_pool_size_like_cpp("Login"))
            .await
            .context("Failed to connect to login database")?;

    info!("Connected to login database");

    // Connect to character database
    let char_info = wow_config::get_database_info_default(
        "Character",
        DatabaseInfo::new("127.0.0.1", 3306, "trinity", "trinity", "characters"),
    );
    log_database_target_like_cpp("character", &char_info);

    let character_connection = wow_database::build_connection_string_with_ssl_like_cpp(
        &char_info.host,
        &char_info.port_or_socket,
        &char_info.username,
        &char_info.password,
        &char_info.database,
        char_info.ssl,
    );
    let char_db = CharacterDatabase::open_with_pool_size(
        &character_connection,
        database_pool_size_like_cpp("Character"),
    )
    .await
    .context("Failed to connect to character database")?;

    info!("Connected to character database");

    // Connect to world database
    let world_info = wow_config::get_database_info_default(
        "World",
        DatabaseInfo::new("127.0.0.1", 3306, "trinity", "trinity", "world"),
    );
    log_database_target_like_cpp("world", &world_info);

    let world_connection = wow_database::build_connection_string_with_ssl_like_cpp(
        &world_info.host,
        &world_info.port_or_socket,
        &world_info.username,
        &world_info.password,
        &world_info.database,
        world_info.ssl,
    );
    let world_db =
        WorldDatabase::open_with_pool_size(&world_connection, database_pool_size_like_cpp("World"))
            .await
            .context("Failed to connect to world database")?;

    info!("Connected to world database");
    let world_db = Arc::new(world_db);
    Ok((login_db, char_db, world_db))
}

pub(super) async fn open_hotfix_database() -> Result<HotfixDatabase> {
    // Connect to hotfix database
    let hotfix_info = wow_config::get_database_info_default(
        "Hotfix",
        DatabaseInfo::new("127.0.0.1", 3306, "trinity", "trinity", "hotfixes"),
    );
    log_database_target_like_cpp("hotfix", &hotfix_info);

    let hotfix_connection = wow_database::build_connection_string_with_ssl_like_cpp(
        &hotfix_info.host,
        &hotfix_info.port_or_socket,
        &hotfix_info.username,
        &hotfix_info.password,
        &hotfix_info.database,
        hotfix_info.ssl,
    );
    let hotfix_db = HotfixDatabase::open_with_pool_size(
        &hotfix_connection,
        database_pool_size_like_cpp("Hotfix"),
    )
    .await
    .context("Failed to connect to hotfix database")?;

    info!("Connected to hotfix database");
    Ok(hotfix_db)
}

pub(super) async fn validate_runtime_schemas(
    login_db: &LoginDatabase,
    char_db: &CharacterDatabase,
    world_db: &WorldDatabase,
    hotfix_db: &HotfixDatabase,
) -> Result<()> {
    let migration_manifest = wow_database::migration::bundled_manifest()?;
    for (database, pool) in [
        (wow_database::migration::DatabaseKind::Auth, login_db.pool()),
        (
            wow_database::migration::DatabaseKind::Characters,
            char_db.pool(),
        ),
        (
            wow_database::migration::DatabaseKind::World,
            world_db.pool(),
        ),
        (
            wow_database::migration::DatabaseKind::Hotfixes,
            hotfix_db.pool(),
        ),
    ] {
        wow_database::migration::validate_runtime_schema(pool, &migration_manifest, database)
            .await?;
    }
    Ok(())
}

pub(super) struct WorldCatalogPorts {
    pub(super) condition_disable_catalog_persistence: wow_database::MariaDbConditionDisableCatalogPersistenceAdapterLikeCpp,
    pub(super) gameplay_rule_catalog_persistence: wow_database::MariaDbGameplayRuleCatalogPersistenceAdapterLikeCpp,
    pub(super) quest_catalog_persistence: wow_database::MariaDbQuestCatalogPersistenceAdapterLikeCpp,
    pub(super) world_object_catalog_persistence: wow_database::MariaDbWorldObjectCatalogPersistenceAdapterLikeCpp,
    pub(super) world_auxiliary_catalog_persistence: wow_database::MariaDbWorldAuxiliaryCatalogPersistenceAdapterLikeCpp,
    pub(super) world_reference_catalog_persistence: wow_database::MariaDbWorldReferenceCatalogPersistenceAdapterLikeCpp,
}

pub(super) fn compose_world_catalog_ports(
    world_db: &Arc<wow_database::WorldDatabase>,
) -> anyhow::Result<WorldCatalogPorts> {
    let world_reference_catalog_persistence =
        wow_database::MariaDbWorldReferenceCatalogPersistenceAdapterLikeCpp::new(Arc::clone(
            world_db,
        ));
    let world_auxiliary_catalog_persistence =
        wow_database::MariaDbWorldAuxiliaryCatalogPersistenceAdapterLikeCpp::new(Arc::clone(
            world_db,
        ));
    let world_object_catalog_persistence =
        wow_database::MariaDbWorldObjectCatalogPersistenceAdapterLikeCpp::new(Arc::clone(
            world_db,
        ));
    let quest_catalog_persistence =
        wow_database::MariaDbQuestCatalogPersistenceAdapterLikeCpp::new(Arc::clone(world_db));
    let gameplay_rule_catalog_persistence =
        wow_database::MariaDbGameplayRuleCatalogPersistenceAdapterLikeCpp::new(Arc::clone(
            world_db,
        ));
    let condition_disable_catalog_persistence =
        wow_database::MariaDbConditionDisableCatalogPersistenceAdapterLikeCpp::new(Arc::clone(
            world_db,
        ));
    Ok(WorldCatalogPorts {
        world_reference_catalog_persistence,
        world_auxiliary_catalog_persistence,
        world_object_catalog_persistence,
        quest_catalog_persistence,
        gameplay_rule_catalog_persistence,
        condition_disable_catalog_persistence,
    })
}
