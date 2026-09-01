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

pub mod area_trigger_template_catalog_adapter;
pub mod area_trigger_world_catalog_adapter;
pub mod battle_pet_account_adapter;
pub mod battle_pet_purchase_adapter;
pub mod battle_pet_selection_catalog_adapter;
pub mod canonical_spawn_catalog_adapter;
pub mod character_administration_adapter;
pub mod character_enumeration_adapter;
pub mod chr_specialization_hotfix_adapter;
pub mod condition_disable_catalog_adapter;
pub mod creature_display_hotfix_adapter;
pub mod creature_query_catalog_adapter;
pub mod database;
pub mod difficulty_hotfix_adapter;
pub mod error;
pub mod exploration_base_xp_catalog_adapter;
pub mod game_event_persistence_adapter;
pub mod game_event_world_catalog_adapter;
pub mod game_tele_catalog_adapter;
pub mod gameobject_query_catalog_adapter;
pub mod gameobject_use_template_adapter;
pub mod gameplay_rule_catalog_adapter;
pub mod gossip_catalog_adapter;
pub mod group_loot_money_adapter;
pub mod hotfix_delivery_metadata_adapter;
pub mod instance_lock_persistence_adapter;
pub mod item_random_enchantment_catalog_adapter;
pub mod item_template_addon_catalog_adapter;
pub mod jump_charge_catalog_adapter;
pub mod lfg_dungeons_hotfix_adapter;
pub mod lfg_world_catalog_adapter;
pub mod loader;
pub mod loot_template_catalog_adapter;
pub mod map_corpse_adapter;
pub mod migration;
pub mod mount_catalog_adapter;
pub mod next_mail_time_adapter;
pub mod packet_spoof_ban_adapter;
pub mod page_text_catalog_adapter;
pub mod params;
pub mod persistence_trace;
pub mod phase_hotfix_catalog_adapter;
pub mod phase_world_catalog_adapter;
pub mod player_base_stats_adapter;
pub mod player_choice_catalog_adapter;
pub mod player_creation_catalog_adapter;
pub mod player_inventory_adapter;
pub mod player_lifecycle_adapter;
pub mod player_money_transaction_adapter;
pub mod player_name_query_adapter;
pub mod player_spell_acquisition_adapter;
pub mod query_holder;
pub mod quest_catalog_adapter;
pub mod quest_item_catalog_adapter;
pub mod quest_poi_adapter;
pub mod represented_group_persistence_adapter;
pub mod reputation_catalog_adapter;
pub mod reserved_name_catalog_adapter;
pub mod respawn_persistence_adapter;
pub mod result;
pub mod session_account_state_adapter;
pub mod skill_catalog_hotfix_adapter;
pub mod skill_world_rules_adapter;
pub mod social_adapter;
pub mod spell_acquisition_startup_adapter;
pub mod spell_core_db2_hotfix_adapter;
pub mod spell_info_key_hotfix_adapter;
pub mod spell_world_catalog_adapter;
pub mod statements;
pub mod static_data_overlay_adapter;
pub mod stored_item_adapter;
pub mod stored_item_money_adapter;
pub mod support_bug_report_adapter;
pub mod trainer_catalog_adapter;
pub mod transaction;
pub mod vehicle_catalog_adapter;
pub mod vendor_catalog_adapter;
pub mod visibility_spawn_catalog_adapter;
pub mod void_storage_adapter;
pub mod world_auxiliary_catalog_adapter;
pub mod world_object_catalog_adapter;
pub mod world_reference_catalog_adapter;
pub mod world_state_startup_adapter;

// Re-export primary types at crate root for convenience.
pub use area_trigger_template_catalog_adapter::MariaDbAreaTriggerTemplateCatalogPersistenceAdapterLikeCpp;
pub use area_trigger_world_catalog_adapter::MariaDbAreaTriggerWorldCatalogPersistenceAdapterLikeCpp;
pub use battle_pet_account_adapter::LoginBattlePetPersistenceLikeCpp;
pub use battle_pet_purchase_adapter::CharacterBattlePetPurchasePersistenceAdapterLikeCpp;
pub use battle_pet_selection_catalog_adapter::MariaDbBattlePetSelectionCatalogPersistenceAdapterLikeCpp;
pub use canonical_spawn_catalog_adapter::MariaDbCanonicalSpawnCatalogPersistenceAdapterLikeCpp;
pub use character_administration_adapter::MariaDbCharacterAdministrationPersistenceAdapterLikeCpp;
pub use character_enumeration_adapter::MariaDbCharacterEnumerationPersistenceAdapterLikeCpp;
pub use chr_specialization_hotfix_adapter::MariaDbChrSpecializationHotfixPersistenceAdapterLikeCpp;
pub use condition_disable_catalog_adapter::MariaDbConditionDisableCatalogPersistenceAdapterLikeCpp;
pub use creature_display_hotfix_adapter::MariaDbCreatureDisplayHotfixPersistenceAdapterLikeCpp;
pub use creature_query_catalog_adapter::MariaDbCreatureQueryCatalogPersistenceAdapterLikeCpp;
pub use database::{
    Database, build_connection_string, build_connection_string_with_ssl_like_cpp,
    escape_string_like_cpp, warn_about_sync_queries_enabled_like_cpp,
    warn_about_sync_queries_scope_like_cpp,
};
pub use difficulty_hotfix_adapter::MariaDbDifficultyHotfixPersistenceAdapterLikeCpp;
pub use error::DatabaseError;
pub use exploration_base_xp_catalog_adapter::MariaDbExplorationBaseXpCatalogPersistenceAdapterLikeCpp;
pub use game_event_persistence_adapter::MariaDbGameEventPersistenceAdapterLikeCpp;
pub use game_event_world_catalog_adapter::MariaDbGameEventWorldCatalogPersistenceAdapterLikeCpp;
pub use game_tele_catalog_adapter::MariaDbGameTeleCatalogPersistenceAdapterLikeCpp;
pub use gameobject_query_catalog_adapter::MariaDbGameObjectQueryCatalogPersistenceAdapterLikeCpp;
pub use gameplay_rule_catalog_adapter::MariaDbGameplayRuleCatalogPersistenceAdapterLikeCpp;
pub use gossip_catalog_adapter::MariaDbGossipCatalogPersistenceAdapterLikeCpp;
pub use hotfix_delivery_metadata_adapter::MariaDbHotfixDeliveryMetadataPersistenceAdapterLikeCpp;
pub use instance_lock_persistence_adapter::MariaDbInstanceLockPersistenceAdapterLikeCpp;
pub use item_random_enchantment_catalog_adapter::MariaDbItemRandomEnchantmentCatalogPersistenceAdapterLikeCpp;
pub use item_template_addon_catalog_adapter::MariaDbItemTemplateAddonCatalogPersistenceAdapterLikeCpp;
pub use jump_charge_catalog_adapter::MariaDbJumpChargeCatalogPersistenceAdapterLikeCpp;
pub use lfg_dungeons_hotfix_adapter::MariaDbLfgDungeonsHotfixPersistenceAdapterLikeCpp;
pub use lfg_world_catalog_adapter::MariaDbLfgWorldCatalogPersistenceAdapterLikeCpp;
pub use loader::{
    DATABASE_CHARACTER_LIKE_CPP, DATABASE_HOTFIX_LIKE_CPP, DATABASE_LOGIN_LIKE_CPP,
    DATABASE_MASK_ALL_LIKE_CPP, DATABASE_NONE_LIKE_CPP, DATABASE_WORLD_LIKE_CPP,
    DatabaseLoaderLikeCpp,
};
pub use loot_template_catalog_adapter::MariaDbLootTemplateCatalogPersistenceAdapterLikeCpp;
pub use mount_catalog_adapter::MariaDbMountCatalogPersistenceAdapterLikeCpp;
pub use page_text_catalog_adapter::MariaDbPageTextCatalogPersistenceAdapterLikeCpp;
pub use params::{PreparedStatement, SqlParam};
pub use phase_hotfix_catalog_adapter::MariaDbPhaseHotfixPersistenceAdapterLikeCpp;
pub use phase_world_catalog_adapter::MariaDbPhaseWorldCatalogPersistenceAdapterLikeCpp;
pub use player_base_stats_adapter::MariaDbPlayerBaseStatsPersistenceAdapterLikeCpp;
pub use player_choice_catalog_adapter::MariaDbPlayerChoiceCatalogPersistenceAdapterLikeCpp;
pub use player_creation_catalog_adapter::MariaDbPlayerCreationCatalogPersistenceAdapterLikeCpp;
pub use player_inventory_adapter::MariaDbPlayerInventoryPersistenceAdapterLikeCpp;
pub use player_name_query_adapter::MariaDbPlayerNameQueryPersistenceAdapterLikeCpp;
pub use query_holder::{SqlQueryHolder, SqlQueryHolderResult};
pub use quest_catalog_adapter::MariaDbQuestCatalogPersistenceAdapterLikeCpp;
pub use quest_item_catalog_adapter::MariaDbQuestItemCatalogPersistenceAdapterLikeCpp;
pub use reputation_catalog_adapter::MariaDbReputationCatalogPersistenceAdapterLikeCpp;
pub use reserved_name_catalog_adapter::MariaDbReservedNameCatalogPersistenceAdapterLikeCpp;
pub use respawn_persistence_adapter::MariaDbRespawnPersistenceAdapterLikeCpp;
pub use result::{
    DatabaseFieldTypeLikeCpp, SqlFields, SqlResult, database_field_type_like_cpp,
    rust_type_compatible_with_database_field_like_cpp,
};
pub use skill_catalog_hotfix_adapter::MariaDbSkillCatalogHotfixPersistenceAdapterLikeCpp;
pub use skill_world_rules_adapter::MariaDbSkillWorldRulesPersistenceAdapterLikeCpp;
pub use spell_acquisition_startup_adapter::MariaDbSpellAcquisitionStartupPersistenceAdapterLikeCpp;
pub use spell_core_db2_hotfix_adapter::MariaDbSpellCoreDb2HotfixPersistenceAdapterLikeCpp;
pub use spell_info_key_hotfix_adapter::MariaDbSpellInfoKeyHotfixPersistenceAdapterLikeCpp;
pub use spell_world_catalog_adapter::MariaDbSpellWorldCatalogPersistenceAdapterLikeCpp;
pub use statements::{
    CharStatements, HOTFIX_STATEMENT_STRATEGY_LIKE_CPP, HotfixStatementStrategyLikeCpp,
    HotfixStatements, LoginStatements, StatementDef, WorldStatements,
};
pub use static_data_overlay_adapter::MariaDbStaticDataOverlayPersistenceAdapterLikeCpp;
pub use stored_item_adapter::MariaDbStoredItemPersistenceAdapterLikeCpp;
pub use trainer_catalog_adapter::MariaDbTrainerCatalogPersistenceAdapterLikeCpp;
pub use transaction::{
    ItemGuidAllocatorAdvisoryLockLikeCpp, SqlTransaction, SqlTransactionCommitError,
    is_database_deadlock_like_cpp, retry_deadlocked_operation_like_cpp,
};
pub use vehicle_catalog_adapter::{
    MariaDbVehicleHotfixPersistenceAdapterLikeCpp,
    MariaDbVehicleWorldCatalogPersistenceAdapterLikeCpp,
};
pub use vendor_catalog_adapter::MariaDbVendorCatalogPersistenceAdapterLikeCpp;
pub use visibility_spawn_catalog_adapter::MariaDbVisibilitySpawnCatalogPersistenceAdapterLikeCpp;
pub use world_auxiliary_catalog_adapter::MariaDbWorldAuxiliaryCatalogPersistenceAdapterLikeCpp;
pub use world_object_catalog_adapter::MariaDbWorldObjectCatalogPersistenceAdapterLikeCpp;
pub use world_reference_catalog_adapter::MariaDbWorldReferenceCatalogPersistenceAdapterLikeCpp;
pub use world_state_startup_adapter::MariaDbWorldStateStartupPersistenceAdapterLikeCpp;

/// Type aliases for each database connection.
pub type LoginDatabase = Database<LoginStatements>;
pub type WorldDatabase = Database<WorldStatements>;
pub type CharacterDatabase = Database<CharStatements>;
pub type HotfixDatabase = Database<HotfixStatements>;
