//! Async MySQL database layer for RustyCore.
//!
//! Provides type-safe database access with prepared statements, matching the
//! C# `MySqlBase<T>` / `PreparedStatement` / `SQLResult` pattern from
//! TrinityCore/RustyCore.
//!
//! # Type Safety
//!
//! Each database connection is parameterized by a statement enum type. This
//! makes it a **compile-time error** to use the wrong statement type on the
//! wrong database:
//!
//! ```ignore
//! use wow_database::*;
//!
//! let login_db: Database<LoginStatements> = Database::open("mysql://...").await?;
//! let world_db: Database<WorldStatements> = Database::open("mysql://...").await?;
//!
//! // This compiles:
//! let mut stmt = login_db.prepare(LoginStatements::SEL_REALMLIST);
//! let result = login_db.query(&stmt).await?;
//!
//! // This would NOT compile:
//! // let stmt = login_db.prepare(WorldStatements::SEL_COMMANDS); // ERROR!
//! ```
//!
//! # Architecture
//!
//! - [`Database<S>`]: Connection pool wrapper, parameterized by statement type
//! - [`PreparedStatement`]: SQL + dynamic parameters (set via `set_u32`, `set_string`, etc.)
//! - [`SqlResult`]: Query result with cursor-style row iteration
//! - [`SqlFields`]: Borrowed view of a single row
//! - [`SqlTransaction`]: Batch of statements executed atomically
//! - Statement enums: [`LoginStatements`], [`WorldStatements`], [`CharStatements`], [`HotfixStatements`]

pub mod battle_pet;
pub mod catalogs;
pub mod character_administration_adapter;
pub mod character_enumeration_adapter;
pub mod database;
pub mod error;
pub mod game;
pub mod group_loot_money_adapter;
pub mod hotfix;
pub mod hotfix_delivery_metadata_adapter;
pub mod instance_lock_persistence_adapter;
pub mod loader;
pub mod map_corpse_adapter;
pub mod migration;
pub mod packet_spoof_ban_adapter;
pub mod params;
pub mod persistence_trace;
pub mod player;
pub mod query_holder;
pub mod quest;
pub mod represented_group_persistence_adapter;
pub mod respawn_persistence_adapter;
pub mod result;
pub mod session_account_state_adapter;
pub mod skill_world_rules_adapter;
pub mod social_adapter;
pub mod spell;
pub mod statements;
pub mod static_data_overlay_adapter;
pub mod stored_item_adapter;
pub mod stored_item_money_adapter;
pub mod support_bug_report_adapter;
pub mod transaction;
pub mod vendor_trade_adapter;
pub mod void_storage_adapter;
pub mod world;

// Re-export primary types at crate root for convenience.
pub use battle_pet::CharacterBattlePetPurchasePersistenceAdapterLikeCpp;
pub use battle_pet::LoginBattlePetPersistenceLikeCpp;
pub use battle_pet::MariaDbBattlePetSelectionCatalogPersistenceAdapterLikeCpp;
pub use catalogs::MariaDbAreaTriggerTemplateCatalogPersistenceAdapterLikeCpp;
pub use catalogs::MariaDbAreaTriggerWorldCatalogPersistenceAdapterLikeCpp;
pub use catalogs::MariaDbCanonicalSpawnCatalogPersistenceAdapterLikeCpp;
pub use catalogs::MariaDbConditionDisableCatalogPersistenceAdapterLikeCpp;
pub use catalogs::MariaDbExplorationBaseXpCatalogPersistenceAdapterLikeCpp;
pub use catalogs::MariaDbGameplayRuleCatalogPersistenceAdapterLikeCpp;
pub use catalogs::MariaDbGossipCatalogPersistenceAdapterLikeCpp;
pub use catalogs::MariaDbItemRandomEnchantmentCatalogPersistenceAdapterLikeCpp;
pub use catalogs::MariaDbItemTemplateAddonCatalogPersistenceAdapterLikeCpp;
pub use catalogs::MariaDbJumpChargeCatalogPersistenceAdapterLikeCpp;
pub use catalogs::MariaDbLfgWorldCatalogPersistenceAdapterLikeCpp;
pub use catalogs::MariaDbLootTemplateCatalogPersistenceAdapterLikeCpp;
pub use catalogs::MariaDbMountCatalogPersistenceAdapterLikeCpp;
pub use catalogs::MariaDbPhaseHotfixPersistenceAdapterLikeCpp;
pub use catalogs::MariaDbPhaseWorldCatalogPersistenceAdapterLikeCpp;
pub use catalogs::MariaDbReputationCatalogPersistenceAdapterLikeCpp;
pub use catalogs::MariaDbReservedNameCatalogPersistenceAdapterLikeCpp;
pub use catalogs::MariaDbTrainerCatalogPersistenceAdapterLikeCpp;
pub use catalogs::MariaDbVendorCatalogPersistenceAdapterLikeCpp;
pub use catalogs::MariaDbVisibilitySpawnCatalogPersistenceAdapterLikeCpp;
pub use catalogs::{
    MariaDbVehicleHotfixPersistenceAdapterLikeCpp,
    MariaDbVehicleWorldCatalogPersistenceAdapterLikeCpp,
};
pub use character_administration_adapter::MariaDbCharacterAdministrationPersistenceAdapterLikeCpp;
pub use character_enumeration_adapter::MariaDbCharacterEnumerationPersistenceAdapterLikeCpp;
pub use database::{
    Database, build_connection_string, build_connection_string_with_ssl_like_cpp,
    escape_string_like_cpp, warn_about_sync_queries_enabled_like_cpp,
    warn_about_sync_queries_scope_like_cpp,
};
pub use error::DatabaseError;
pub use game::MariaDbGameEventPersistenceAdapterLikeCpp;
pub use game::MariaDbGameEventWorldCatalogPersistenceAdapterLikeCpp;
pub use game::MariaDbGameTeleCatalogPersistenceAdapterLikeCpp;
pub use hotfix::MariaDbChrSpecializationHotfixPersistenceAdapterLikeCpp;
pub use hotfix::MariaDbCreatureDisplayHotfixPersistenceAdapterLikeCpp;
pub use hotfix::MariaDbDifficultyHotfixPersistenceAdapterLikeCpp;
pub use hotfix::MariaDbLfgDungeonsHotfixPersistenceAdapterLikeCpp;
pub use hotfix::MariaDbSkillCatalogHotfixPersistenceAdapterLikeCpp;
pub use hotfix_delivery_metadata_adapter::MariaDbHotfixDeliveryMetadataPersistenceAdapterLikeCpp;
pub use instance_lock_persistence_adapter::MariaDbInstanceLockPersistenceAdapterLikeCpp;
pub use loader::{
    DATABASE_CHARACTER_LIKE_CPP, DATABASE_HOTFIX_LIKE_CPP, DATABASE_LOGIN_LIKE_CPP,
    DATABASE_MASK_ALL_LIKE_CPP, DATABASE_NONE_LIKE_CPP, DATABASE_WORLD_LIKE_CPP,
    DatabaseLoaderLikeCpp,
};
pub use params::{PreparedStatement, SqlParam};
pub use player::MariaDbPlayerBaseStatsPersistenceAdapterLikeCpp;
pub use player::MariaDbPlayerChoiceCatalogPersistenceAdapterLikeCpp;
pub use player::MariaDbPlayerCreationCatalogPersistenceAdapterLikeCpp;
pub use player::MariaDbPlayerInventoryPersistenceAdapterLikeCpp;
pub use player::MariaDbPlayerNameQueryPersistenceAdapterLikeCpp;
pub use player::MariaDbPlayerQuestPersistenceAdapterLikeCpp;
pub use player::MariaDbPlayerQuestRewardPersistenceAdapterLikeCpp;
pub use query_holder::{SqlQueryHolder, SqlQueryHolderResult};
pub use quest::MariaDbQuestCatalogPersistenceAdapterLikeCpp;
pub use quest::MariaDbQuestItemCatalogPersistenceAdapterLikeCpp;
pub use respawn_persistence_adapter::MariaDbRespawnPersistenceAdapterLikeCpp;
pub use result::{
    DatabaseFieldTypeLikeCpp, SqlFields, SqlResult, database_field_type_like_cpp,
    rust_type_compatible_with_database_field_like_cpp,
};
pub use skill_world_rules_adapter::MariaDbSkillWorldRulesPersistenceAdapterLikeCpp;
pub use spell::MariaDbSpellAcquisitionStartupPersistenceAdapterLikeCpp;
pub use spell::MariaDbSpellCoreDb2HotfixPersistenceAdapterLikeCpp;
pub use spell::MariaDbSpellInfoKeyHotfixPersistenceAdapterLikeCpp;
pub use spell::MariaDbSpellWorldCatalogPersistenceAdapterLikeCpp;
pub use statements::{
    CharStatements, HOTFIX_STATEMENT_STRATEGY_LIKE_CPP, HotfixStatementStrategyLikeCpp,
    HotfixStatements, LoginStatements, StatementDef, WorldStatements,
};
pub use static_data_overlay_adapter::MariaDbStaticDataOverlayPersistenceAdapterLikeCpp;
pub use stored_item_adapter::MariaDbStoredItemPersistenceAdapterLikeCpp;
pub use transaction::{
    ItemGuidAllocatorAdvisoryLockLikeCpp, SqlTransaction, SqlTransactionCommitError,
    is_database_deadlock_like_cpp, retry_deadlocked_operation_like_cpp,
};
pub use vendor_trade_adapter::MariaDbVendorTradePersistenceAdapterLikeCpp;
pub use world::MariaDbWorldAuxiliaryCatalogPersistenceAdapterLikeCpp;
pub use world::MariaDbWorldObjectCatalogPersistenceAdapterLikeCpp;
pub use world::MariaDbWorldReferenceCatalogPersistenceAdapterLikeCpp;
pub use world::MariaDbWorldStateStartupPersistenceAdapterLikeCpp;

/// Type aliases for each database connection.
pub type LoginDatabase = Database<LoginStatements>;
pub type WorldDatabase = Database<WorldStatements>;
pub type CharacterDatabase = Database<CharStatements>;
pub type HotfixDatabase = Database<HotfixStatements>;
