// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! World Server — entry point.
//!
//! Accepts WoW client connections after BNet authentication, performs the
//! world-server handshake (challenge → auth → encryption), creates a
//! WorldSession for each client, and dispatches packets to handlers.

use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::future::Future;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use tracing::{debug, info, warn};
use wow_config::{DatabaseInfo, LoadReport, WorldConfigSet};
use wow_core::{ObjectGuid, ObjectGuidGenerator, guid::HighGuid};
use wow_database::{
    CharStatements, CharacterDatabase, HotfixDatabase, LoginDatabase, LoginStatements,
    PreparedStatement, SqlTransaction, StatementDef, WorldDatabase, WorldStatements,
    build_connection_string,
};
use wow_instances::{InstanceLockMgr, MapDb2Entries, MapDifficultyResetInterval};
use wow_loot::{
    LootConditionId, LootConditionLinkReport, LootConditionReferenceUseLikeCpp,
    LootReferenceCheckReport, LootStore, LootStoreKind, LootStores, LootTemplateRow,
    check_loot_condition_links_like_cpp, check_loot_condition_references_like_cpp,
    check_loot_references_like_cpp, loot_store_kind_for_condition_source_type_like_cpp,
};
use wow_network::session_mgr::SessionManager;
use wow_network::world_socket::{AccountInfo, AccountLookup};
use wow_network::{
    GameEventQuestCompleteCommandLikeCpp, GameEventQuestCompleteResponseLikeCpp, GroupRegistry,
    LootDropRatesLikeCpp, PendingInvites, PlayerRegistry, ReputationRatesLikeCpp,
    ResetSeasonalQuestStatusCommand, SendVisibleObjectValuesUpdateCommand, SessionCommand,
    SessionResources,
};
use wow_packet::{
    ServerPacket,
    packets::chat::{ChatMsg, ChatPkt},
};
use wow_world::{
    MMapRuntimeConfigLikeCpp, MapManager as LegacyMapManager, SharedCanonicalMapManager,
    SharedMapManager, WorldMMapPathfinderWorkerLikeCpp, WorldSession,
    conditions::{
        ConditionMapRef, ConditionMapStateSnapshot, is_spawn_group_meeting_map_conditions_like_cpp,
    },
    entity_update_bridge::unit_values_update_to_packet,
};

mod creature_loaded_grid;
mod gameobject_loaded_grid;
mod spawn_store_loader;

const WORLD_CONFIG_CANDIDATES: &[&str] = &[
    "worldserver.conf",
    "worldserver.conf.dist",
    "WorldServer.conf",
    "WorldServer.conf.dist",
];
const WORLD_CONFIG_DIR: &str = "worldserver.conf.d";
const RUSTYCORE_LEGACY_CREATURE_GLOBAL_RUNTIME_CONFIG: &str =
    "RustyCore.LegacyCreatureGlobalRuntime";
const DEFAULT_RESPAWN_MIN_CHECK_INTERVAL_MS: u32 = 5_000;
const CREATURE_TYPE_MECHANICAL_LIKE_CPP: u32 = 9;
const CREATURE_TYPE_FLAG_BOSS_MOB_LIKE_CPP: u32 = 0x0001_0000;

type SharedCanonicalSpawnMetadataLikeCpp =
    Arc<Mutex<spawn_store_loader::CanonicalSpawnMetadataLikeCpp>>;
type SharedWorldStateMgrLikeCpp = Arc<Mutex<spawn_store_loader::WorldStateMgrLikeCpp>>;

// ── Account lookup implementation ────────────────────────────────

/// Looks up account information from the login database using the realm join ticket.
///
/// The realm join ticket sent by the client in AuthSession is actually the game
/// account username (e.g. "2#1"), NOT the BNet LoginTicket (TC-xxx). The C#
/// RustyCore WorldSocket.HandleAuthSession uses SEL_ACCOUNT_INFO_BY_NAME with
/// `WHERE a.username = ?` to look it up directly.
struct DbAccountLookup {
    login_db: Arc<LoginDatabase>,
    realm_id: u16,
    win64_auth_seed: [u8; 16],
}

impl AccountLookup for DbAccountLookup {
    fn lookup_account(
        &self,
        realm_join_ticket: &str,
    ) -> Pin<Box<dyn Future<Output = Option<AccountInfo>> + Send + '_>> {
        let ticket = realm_join_ticket.to_owned();
        let realm_id = self.realm_id;
        Box::pin(async move {
            // The realm_join_ticket is the game account username (e.g. "2#1").
            // Query SEL_ACCOUNT_INFO_BY_NAME: params are (RealmID, username).
            //
            // Columns returned:
            //  0: a.id                  (account_id)
            //  1: a.session_key_bnet    (hex session key)
            //  2: ba.last_ip
            //  3: ba.locked
            //  4: ba.lock_country
            //  5: a.expansion
            //  6: a.mutetime
            //  7: ba.locale
            //  8: a.recruiter
            //  9: a.os
            // 10: a.timezone_offset
            // 11: ba.id                 (battlenet_account_id)
            // 12: aa.SecurityLevel
            // 13: bab ban expr          (is_banned_bnet)
            // 14: ab ban expr           (is_banned_account)
            // 15: r.id                  (recruiter)
            let mut stmt = self
                .login_db
                .prepare(LoginStatements::SEL_ACCOUNT_INFO_BY_NAME);
            stmt.set_i32(0, i32::from(realm_id));
            stmt.set_string(1, &ticket);

            let result = match self.login_db.query(&stmt).await {
                Ok(r) => r,
                Err(e) => {
                    tracing::error!("DB error looking up account by name '{ticket}': {e}");
                    return None;
                }
            };

            if result.is_empty() {
                tracing::warn!("No account found for realm_join_ticket '{ticket}'");
                return None;
            }

            let account_id: u32 = result.read(0);
            // session_key_bnet is varbinary(64) — read as raw bytes, then hex-encode
            let session_key_raw: Vec<u8> = result.try_read(1).unwrap_or_default();
            let session_key_hex: String =
                session_key_raw.iter().map(|b| format!("{b:02X}")).collect();
            let last_ip: String = result.try_read(2).unwrap_or_default();
            let is_locked: u8 = result.try_read(3).unwrap_or(0);
            let lock_country: String = result.try_read(4).unwrap_or_default();
            let expansion: u8 = result.try_read(5).unwrap_or(2);
            let _mutetime: i64 = result.try_read(6).unwrap_or(0);
            let locale_raw: String = result
                .try_read::<u8>(7)
                .map(|v| v.to_string())
                .unwrap_or_else(|| result.try_read::<String>(7).unwrap_or_default());
            let recruiter: u32 = result.try_read(8).unwrap_or(0);
            let os: String = result.try_read(9).unwrap_or_default();
            let timezone_offset: i16 = result.try_read(10).unwrap_or(0);
            let bnet_id: u32 = result.try_read(11).unwrap_or(0);
            let security: u8 = result.try_read(12).unwrap_or(0);
            let is_banned_bnet: u32 = result.try_read(13).unwrap_or(0);
            let is_banned_account: u32 = result.try_read(14).unwrap_or(0);

            if account_id == 0 {
                tracing::warn!("Account id is 0 for ticket '{ticket}'");
                return None;
            }

            if session_key_hex.is_empty() {
                tracing::warn!("No session key for account {account_id} (ticket '{ticket}')");
                return None;
            }

            let locale_name = locale_id_to_name(&locale_raw);
            tracing::info!(
                "Account lookup OK: id={account_id}, bnet_id={bnet_id}, os={os}, locale_raw='{locale_raw}', locale='{locale_name}'"
            );

            Some(AccountInfo {
                id: account_id,
                session_key_hex,
                last_ip,
                is_locked_to_ip: is_locked != 0,
                lock_country,
                expansion,
                mute_time: 0,
                locale: locale_name,
                recruiter,
                os,
                timezone_offset: i32::from(timezone_offset),
                battlenet_account_id: bnet_id,
                security,
                is_banned_bnet: is_banned_bnet != 0,
                is_banned_account: is_banned_account != 0,
                win64_auth_seed: self.win64_auth_seed,
                client_address: None,            // Set by accept loop after auth
                derived_session_key: Vec::new(), // Set by accept loop after auth
            })
        })
    }
}

// ── Main ─────────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    info!("RustyCore World Server starting...");

    load_world_config()?;
    let world_configs = wow_config::load_world_config_values();

    // Connect to login database (needed for session key validation)
    let login_info = wow_config::get_database_info_default(
        "Login",
        DatabaseInfo::new("127.0.0.1", 3306, "trinity", "trinity", "auth"),
    );

    let conn_str = build_connection_string(
        &login_info.host,
        &login_info.port_or_socket,
        &login_info.username,
        &login_info.password,
        &login_info.database,
    );
    let login_db = LoginDatabase::open(&conn_str)
        .await
        .context("Failed to connect to login database")?;

    info!("Connected to login database");

    // Connect to character database
    let char_info = wow_config::get_database_info_default(
        "Character",
        DatabaseInfo::new("127.0.0.1", 3306, "trinity", "trinity", "characters"),
    );

    let char_conn_str = build_connection_string(
        &char_info.host,
        &char_info.port_or_socket,
        &char_info.username,
        &char_info.password,
        &char_info.database,
    );
    let char_db = CharacterDatabase::open(&char_conn_str)
        .await
        .context("Failed to connect to character database")?;

    info!("Connected to character database");

    // Connect to world database
    let world_info = wow_config::get_database_info_default(
        "World",
        DatabaseInfo::new("127.0.0.1", 3306, "trinity", "trinity", "world"),
    );

    let world_conn_str = build_connection_string(
        &world_info.host,
        &world_info.port_or_socket,
        &world_info.username,
        &world_info.password,
        &world_info.database,
    );
    let world_db = WorldDatabase::open(&world_conn_str)
        .await
        .context("Failed to connect to world database")?;

    info!("Connected to world database");
    let world_db = Arc::new(world_db);

    // Connect to hotfix database
    let hotfix_info = wow_config::get_database_info_default(
        "Hotfix",
        DatabaseInfo::new("127.0.0.1", 3306, "trinity", "trinity", "hotfixes"),
    );

    let hotfix_conn_str = build_connection_string(
        &hotfix_info.host,
        &hotfix_info.port_or_socket,
        &hotfix_info.username,
        &hotfix_info.password,
        &hotfix_info.database,
    );
    let hotfix_db = HotfixDatabase::open(&hotfix_conn_str)
        .await
        .context("Failed to connect to hotfix database")?;

    info!("Connected to hotfix database");

    // ── Database auto-update ──────────────────────────────────────────────
    let auto_setup = wow_config::get_string_default("Updates.AutoSetup", "1");
    if auto_setup != "0" && auto_setup.to_lowercase() != "false" {
        use wow_database::updater::DbUpdater;
        let src = wow_config::get_string_default("Updates.SourcePath", ".");

        let auth_up = DbUpdater::new(
            login_db.pool().clone(),
            &login_info.host,
            &login_info.port_or_socket,
            &login_info.username,
            &login_info.password,
            &login_info.database,
        );
        if let Err(e) = auth_up
            .populate(&format!("{src}/sql/base/auth_database.sql"))
            .await
        {
            tracing::warn!("Auth populate skipped: {e}");
        }
        if let Err(e) = auth_up.update(&src).await {
            tracing::warn!("Auth update error: {e}");
        }

        let char_up = DbUpdater::new(
            char_db.pool().clone(),
            &char_info.host,
            &char_info.port_or_socket,
            &char_info.username,
            &char_info.password,
            &char_info.database,
        );
        if let Err(e) = char_up
            .populate(&format!("{src}/sql/base/characters_database.sql"))
            .await
        {
            tracing::warn!("Characters populate skipped: {e}");
        }
        if let Err(e) = char_up.update(&src).await {
            tracing::warn!("Characters update error: {e}");
        }

        // world + hotfixes: only update (base SQL is the full TDB, downloaded separately)
        let world_up = DbUpdater::new(
            world_db.pool().clone(),
            &world_info.host,
            &world_info.port_or_socket,
            &world_info.username,
            &world_info.password,
            &world_info.database,
        );
        if let Err(e) = world_up.update(&src).await {
            tracing::warn!("World update error: {e}");
        }

        let hotfix_up = DbUpdater::new(
            hotfix_db.pool().clone(),
            &hotfix_info.host,
            &hotfix_info.port_or_socket,
            &hotfix_info.username,
            &hotfix_info.password,
            &hotfix_info.database,
        );
        if let Err(e) = hotfix_up.update(&src).await {
            tracing::warn!("Hotfix update error: {e}");
        }
    }
    // ─────────────────────────────────────────────────────────────────────

    let hotfix_db = Arc::new(hotfix_db);

    // Initialize GUID generator from MAX(guid) in characters table
    let max_guid = {
        let stmt = char_db.prepare(CharStatements::SEL_MAX_GUID);
        match char_db.query(&stmt).await {
            Ok(result) => {
                if result.is_empty() || result.is_null(0) {
                    1i64
                } else {
                    let max_val: u32 = result.try_read(0).unwrap_or(0);
                    (max_val as i64) + 1
                }
            }
            Err(_) => 1i64,
        }
    };

    let guid_generator = Arc::new(ObjectGuidGenerator::new(HighGuid::Player, max_guid));
    info!("GUID generator initialized, next counter: {max_guid}");

    let char_db = Arc::new(char_db);

    // Load Item.db2 for inventory_type lookups (replaces item_type_cache table)
    let data_dir = wow_config::get_string_default("DataDir", "./Data");
    let locale_raw = wow_config::get_string_default("DBC.Locale", "esES");
    let locale = locale_id_to_name(&locale_raw);
    let currency_types_store = Arc::new(
        wow_data::CurrencyTypesStore::load(&data_dir, &locale)
            .context("Failed to load CurrencyTypes.db2 — check DataDir and DBC.Locale config")?,
    );
    info!(
        "Loaded {} currencies from CurrencyTypes.db2",
        currency_types_store.len()
    );

    let import_price_stores = Arc::new(
        wow_data::ImportPriceStores::load(&data_dir, &locale)
            .context("Failed to load ImportPrice*.db2 — check DataDir and DBC.Locale config")?,
    );
    info!("Loaded ImportPrice*.db2 stores");

    let item_class_store = Arc::new(
        wow_data::ItemClassStore::load(&data_dir, &locale)
            .context("Failed to load ItemClass.db2 — check DataDir and DBC.Locale config")?,
    );
    info!(
        "Loaded {} item classes from ItemClass.db2",
        item_class_store.len()
    );

    let item_currency_cost_store = Arc::new(
        wow_data::ItemCurrencyCostStore::load(&data_dir, &locale)
            .context("Failed to load ItemCurrencyCost.db2 — check DataDir and DBC.Locale config")?,
    );
    info!(
        "Loaded {} item currency costs from ItemCurrencyCost.db2",
        item_currency_cost_store.len()
    );

    let item_extended_cost_store = Arc::new(
        wow_data::ItemExtendedCostStore::load(&data_dir, &locale)
            .context("Failed to load ItemExtendedCost.db2 — check DataDir and DBC.Locale config")?,
    );
    info!(
        "Loaded {} item extended costs from ItemExtendedCost.db2",
        item_extended_cost_store.len()
    );

    let item_store = Arc::new(
        wow_data::ItemStore::load(&data_dir, &locale)
            .context("Failed to load Item.db2 — check DataDir and DBC.Locale config")?,
    );
    info!("Loaded {} items from Item.db2", item_store.len());

    let item_price_base_store = Arc::new(
        wow_data::ItemPriceBaseStore::load(&data_dir, &locale)
            .context("Failed to load ItemPriceBase.db2 — check DataDir and DBC.Locale config")?,
    );
    info!(
        "Loaded {} item price base rows from ItemPriceBase.db2",
        item_price_base_store.len()
    );

    let item_limit_category_store = Arc::new(
        wow_data::ItemLimitCategoryStore::load(&data_dir, &locale).context(
            "Failed to load ItemLimitCategory.db2 — check DataDir and DBC.Locale config",
        )?,
    );
    info!(
        "Loaded {} item limit categories from ItemLimitCategory.db2",
        item_limit_category_store.len()
    );

    let item_limit_category_condition_store = Arc::new(
        wow_data::ItemLimitCategoryConditionStore::load(&data_dir, &locale).context(
            "Failed to load ItemLimitCategoryCondition.db2 — check DataDir and DBC.Locale config",
        )?,
    );
    info!(
        "Loaded {} item limit category conditions from ItemLimitCategoryCondition.db2",
        item_limit_category_condition_store.len()
    );

    // Load ChrSpecialization.db2 for C++ loot-specialization validation.
    let chr_specialization_store = Arc::new(
        wow_data::ChrSpecializationStore::load(&data_dir, &locale).context(
            "Failed to load ChrSpecialization.db2 — check DataDir and DBC.Locale config",
        )?,
    );
    info!(
        "Loaded {} chr specializations from ChrSpecialization.db2",
        chr_specialization_store.len()
    );

    // Load DungeonEncounter.db2 for C++ instance encounter lock/loot metadata.
    let dungeon_encounter_store = Arc::new(
        wow_data::DungeonEncounterStore::load(&data_dir, &locale)
            .context("Failed to load DungeonEncounter.db2 — check DataDir and DBC.Locale config")?,
    );
    info!(
        "Loaded {} dungeon encounters from DungeonEncounter.db2",
        dungeon_encounter_store.len()
    );

    // Load Map.db2 + MapDifficulty.db2 for C++ InstanceLockMgr MapDb2Entries resolution.
    let map_store = Arc::new(
        wow_data::MapStore::load(&data_dir, &locale)
            .context("Failed to load Map.db2 — check DataDir and DBC.Locale config")?,
    );
    info!("Loaded {} maps from Map.db2", map_store.len());
    let (world_safe_loc_store, world_safe_loc_report) =
        wow_data::WorldSafeLocStore::load_like_cpp(world_db.as_ref(), &map_store)
            .await
            .context("Failed to load C++ world_safe_locs")?;
    info!(
        "Loaded {} world safe locs ({} missing maps, {} invalid positions)",
        world_safe_loc_store.len(),
        world_safe_loc_report.missing_maps.len(),
        world_safe_loc_report.invalid_positions.len()
    );
    let ui_map_x_map_art_store = Arc::new(
        wow_data::UiMapXMapArtStore::load_with_hotfixes(&data_dir, &locale, &hotfix_db)
            .await
            .context("Failed to load UiMapXMapArt.db2 / hotfix rows")?,
    );
    let area_table_store = Arc::new(
        wow_data::AreaTableStore::load_with_hotfixes(&data_dir, &locale, &hotfix_db)
            .await
            .context("Failed to load AreaTable.db2 / hotfix rows")?,
    );
    let fishing_base_skill_store = Arc::new(
        wow_data::FishingBaseSkillStoreLikeCpp::load(world_db.as_ref(), &area_table_store)
            .await
            .context("Failed to load skill_fishing_base_level")?,
    );
    let phase_store = Arc::new(
        wow_data::PhaseStore::load_with_hotfixes(&data_dir, &locale, &hotfix_db)
            .await
            .context("Failed to load Phase.db2 / hotfix rows")?,
    );
    let phase_group_store = Arc::new(
        wow_data::PhaseGroupStore::load_with_hotfixes(&data_dir, &locale, &phase_store, &hotfix_db)
            .await
            .context("Failed to load PhaseXPhaseGroup.db2 / hotfix rows")?,
    );
    info!(
        "Loaded {} phases and {} phase-group rows",
        phase_store.len(),
        phase_group_store.len()
    );
    let mut phase_info_store = wow_data::PhaseInfoStore::from_phase_store_like_cpp(&phase_store);
    phase_info_store
        .load_area_phases_like_cpp(world_db.as_ref(), &area_table_store, &phase_store)
        .await
        .context("Failed to load phase_area rows")?;
    info!(
        "Seeded {} phase info records and {} phase area rows",
        phase_info_store.phase_info_count(),
        phase_info_store.phase_area_count()
    );
    let terrain_swap_store = Arc::new(
        wow_data::load_terrain_swaps(world_db.as_ref(), &map_store, |phase_id| {
            ui_map_x_map_art_store.is_ui_map_phase(phase_id)
        })
        .await
        .context("Failed to load C++ terrain swap stores")?,
    );
    let mut graveyard_store = wow_data::GraveyardStore::default();
    let graveyard_report = graveyard_store
        .load_graveyard_zones_like_cpp(
            world_db.as_ref(),
            |safe_loc_id| world_safe_loc_store.contains(safe_loc_id),
            |area_id| area_table_store.get(area_id).is_some(),
        )
        .await
        .context("Failed to load C++ graveyard_zone links")?;
    info!(
        "Loaded {} graveyard-zone links ({} missing safe locs, {} missing zones, {} duplicates)",
        graveyard_report.loaded,
        graveyard_report.missing_safe_locs.len(),
        graveyard_report.missing_zones.len(),
        graveyard_report.duplicates.len()
    );
    let (mut gossip_store, gossip_load_report) =
        wow_data::GossipStore::load_like_cpp(world_db.as_ref())
            .await
            .context("Failed to load C++ gossip_menu/gossip_menu_option condition keys")?;
    info!(
        "Loaded {} gossip menu rows and {} gossip menu option keys",
        gossip_load_report.menu_rows, gossip_load_report.menu_item_rows
    );
    let (spawn_group_store, spawn_group_report) =
        wow_data::SpawnGroupTemplateStore::load_like_cpp(world_db.as_ref())
            .await
            .context("Failed to load C++ spawn_group_template rows")?;
    info!(
        "Loaded {} spawn group templates ({} invalid flags, {} system/manual flag fixes, {} inserted defaults)",
        spawn_group_store.len(),
        spawn_group_report.invalid_flags.len(),
        spawn_group_report.system_manual_spawn_flags.len(),
        spawn_group_report.inserted_default_groups.len()
    );
    let creature_template_store = Arc::new(
        wow_data::WorldIdStore::load_like_cpp(
            world_db.as_ref(),
            "creature_template",
            WorldStatements::SEL_CREATURE_TEMPLATE_IDS,
        )
        .await
        .context("Failed to load creature_template ids for C++ ConditionMgr validation")?,
    );
    let gameobject_template_store = Arc::new(
        wow_data::WorldIdStore::load_like_cpp(
            world_db.as_ref(),
            "gameobject_template",
            WorldStatements::SEL_GAMEOBJECT_TEMPLATE_IDS,
        )
        .await
        .context("Failed to load gameobject_template ids for C++ ConditionMgr validation")?,
    );
    info!(
        "Loaded condition validation world id stores: {} creature templates, {} gameobject templates",
        creature_template_store.len(),
        gameobject_template_store.len()
    );
    let creature_template_classification_store = Arc::new(
        wow_data::CreatureTemplateClassificationStoreLikeCpp::load_like_cpp(world_db.as_ref())
            .await
            .context("Failed to load creature_template classifications for C++ creature difficulty damage rates")?,
    );
    let creature_template_lifecycle_store = Arc::new(
        wow_data::CreatureTemplateLifecycleStoreLikeCpp::load_like_cpp(world_db.as_ref())
            .await
            .context("Failed to load DB-backed creature_template lifecycle rows for C++ Creature::LoadFromDB")?,
    );
    info!(
        "Loaded {} DB-backed creature_template lifecycle rows for loaded-grid Creature::LoadFromDB",
        creature_template_lifecycle_store.len()
    );
    let gameobject_template_lifecycle_store = Arc::new(
        wow_data::GameObjectTemplateLifecycleStoreLikeCpp::load_like_cpp(world_db.as_ref())
            .await
            .context("Failed to load DB-backed gameobject_template lifecycle rows for C++ GameObject::LoadFromDB")?,
    );
    let gameobject_override_lifecycle_store = Arc::new(
        wow_data::GameObjectOverrideLifecycleStoreLikeCpp::load_like_cpp(world_db.as_ref())
            .await
            .context("Failed to load DB-backed gameobject_overrides lifecycle rows for C++ GameObject::Create")?,
    );
    info!(
        "Loaded C++ GameObject lifecycle stores: {} template rows, {} spawn override rows",
        gameobject_template_lifecycle_store.len(),
        gameobject_override_lifecycle_store.len()
    );
    let creature_damage_rates = wow_data::CreatureClassificationDamageRatesLikeCpp {
        normal: world_config_f32(&world_configs, "Rate.Creature.Damage.Normal", 1.0),
        elite: world_config_f32(&world_configs, "Rate.Creature.Damage.Elite", 1.0),
        rare_elite: world_config_f32(&world_configs, "Rate.Creature.Damage.RareElite", 1.0),
        obsolete: world_config_f32(&world_configs, "Rate.Creature.Damage.Obsolete", 1.0),
        rare: world_config_f32(&world_configs, "Rate.Creature.Damage.Rare", 1.0),
        trivial: world_config_f32(&world_configs, "Rate.Creature.Damage.Trivial", 1.0),
        minus_mob: world_config_f32(&world_configs, "Rate.Creature.Damage.MinusMob", 1.0),
    };
    let creature_health_rates = wow_data::CreatureClassificationHealthRatesLikeCpp {
        normal: world_config_f32(&world_configs, "Rate.Creature.Health.Normal", 1.0),
        elite: world_config_f32(&world_configs, "Rate.Creature.Health.Elite", 1.0),
        rare_elite: world_config_f32(&world_configs, "Rate.Creature.Health.RareElite", 1.0),
        obsolete: world_config_f32(&world_configs, "Rate.Creature.Health.Obsolete", 1.0),
        rare: world_config_f32(&world_configs, "Rate.Creature.Health.Rare", 1.0),
        trivial: world_config_f32(&world_configs, "Rate.Creature.Health.Trivial", 1.0),
        minus_mob: world_config_f32(&world_configs, "Rate.Creature.Health.MinusMob", 1.0),
    };
    let creature_difficulty_store = Arc::new(
        wow_data::CreatureDifficultyStoreLikeCpp::load_like_cpp(world_db.as_ref(), |entry| {
            // C++ missing-template rows are skipped before insertion. This data-wiring
            // slice does not invent full templates; if the minimal classification row is
            // absent, fall back to classification 1 (elite), matching
            // Creature::GetDamageMod's default switch rate.
            let classification = creature_template_classification_store
                .classification_for_entry(entry)
                .unwrap_or(1);
            creature_damage_rates.modifier_for_classification_like_cpp(classification)
        })
        .await
        .context(
            "Failed to load creature_template_difficulty rows with C++ classification damage rates",
        )?,
    );
    let creature_base_stats_store = Arc::new(
        wow_data::CreatureBaseStatsStoreLikeCpp::load_like_cpp(world_db.as_ref())
            .await
            .context("Failed to load creature_classlevelstats rows")?,
    );
    info!(
        "Loaded C++ creature runtime data stores: {} template classifications, {} difficulty rows, {} base stat rows",
        creature_template_classification_store.len(),
        creature_difficulty_store.len(),
        creature_base_stats_store.len()
    );
    let creature_template_mount_store = Arc::new(
        wow_data::CreatureTemplateMountStoreLikeCpp::load_like_cpp(world_db.as_ref())
            .await
            .context("Failed to load creature_template mount fallback rows")?,
    );
    info!(
        "Loaded {} creature template mount fallback rows",
        creature_template_mount_store.len()
    );
    let creature_display_info_store = Arc::new(
        wow_data::CreatureDisplayInfoStore::load_with_hotfixes(&data_dir, &locale, &hotfix_db)
            .await
            .context("Failed to load CreatureDisplayInfo.db2 / hotfix rows")?,
    );
    info!(
        "Loaded {} creature display info rows",
        creature_display_info_store.len()
    );
    let gameobject_display_info_store = Arc::new(
        wow_data::GameObjectDisplayInfoStore::load(&data_dir, &locale)
            .context("Failed to load GameObjectDisplayInfo.db2")?,
    );
    info!(
        "Loaded {} gameobject display info rows",
        gameobject_display_info_store.len()
    );
    let creature_model_data_store = Arc::new(
        wow_data::CreatureModelDataStore::load_with_hotfixes(&data_dir, &locale, &hotfix_db)
            .await
            .context("Failed to load CreatureModelData.db2 / hotfix rows")?,
    );
    info!(
        "Loaded {} creature model data rows",
        creature_model_data_store.len()
    );
    let vehicle_store = Arc::new(
        wow_data::VehicleStore::load_with_hotfixes(&data_dir, &locale, &hotfix_db)
            .await
            .context("Failed to load Vehicle.db2 / hotfix rows")?,
    );
    info!("Loaded {} vehicle rows", vehicle_store.len());
    let vehicle_seat_store = Arc::new(
        wow_data::VehicleSeatStore::load_with_hotfixes(&data_dir, &locale, &hotfix_db)
            .await
            .context("Failed to load VehicleSeat.db2 / hotfix rows")?,
    );
    info!("Loaded {} vehicle seat rows", vehicle_seat_store.len());
    let vehicle_template_store = Arc::new(
        wow_data::VehicleTemplateStoreLikeCpp::load_like_cpp(world_db.as_ref())
            .await
            .context("Failed to load C++ vehicle_template rows")?,
    );
    let vehicle_accessory_store = Arc::new(
        wow_data::VehicleAccessoryStoreLikeCpp::load_like_cpp(world_db.as_ref())
            .await
            .context("Failed to load C++ vehicle accessory rows")?,
    );
    let creature_spawn_store = Arc::new(
        wow_data::WorldSpawnIdStore::load_like_cpp(
            world_db.as_ref(),
            "creature",
            WorldStatements::SEL_CREATURE_SPAWN_IDS,
        )
        .await
        .context("Failed to load creature spawn ids for C++ ConditionMgr validation")?,
    );
    let gameobject_spawn_store = Arc::new(
        wow_data::WorldSpawnIdStore::load_like_cpp(
            world_db.as_ref(),
            "gameobject",
            WorldStatements::SEL_GAMEOBJECT_SPAWN_IDS,
        )
        .await
        .context("Failed to load gameobject spawn ids for C++ ConditionMgr validation")?,
    );
    info!(
        "Loaded condition validation spawn id stores: {} creature spawns, {} gameobject spawns",
        creature_spawn_store.len(),
        gameobject_spawn_store.len()
    );
    let active_event_store = Arc::new(
        wow_data::WorldIdStore::load_like_cpp(
            world_db.as_ref(),
            "game_event",
            WorldStatements::SEL_VALID_GAME_EVENT_IDS,
        )
        .await
        .context("Failed to load game_event ids for C++ ConditionMgr validation")?,
    );
    let world_state_store = Arc::new(
        wow_data::WorldIdStore::load_like_cpp(
            world_db.as_ref(),
            "world_state",
            WorldStatements::SEL_WORLD_STATE_IDS,
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
        wow_data::WorldIdStore::load_like_cpp(
            world_db.as_ref(),
            "trainer",
            WorldStatements::SEL_TRAINER_IDS,
        )
        .await
        .context("Failed to load trainer ids for C++ ConditionMgr validation")?,
    );
    info!(
        "Loaded condition validation trainer id store: {} trainers",
        trainer_store.len()
    );
    let area_trigger_template_store = Arc::new(
        wow_data::AreaTriggerTemplateStore::load_like_cpp(world_db.as_ref())
            .await
            .context("Failed to load areatrigger_template keys for C++ ConditionMgr validation")?,
    );
    info!(
        "Loaded condition validation area-trigger template store: {} templates",
        area_trigger_template_store.len()
    );

    let map_difficulty_store = Arc::new(
        wow_data::MapDifficultyStore::load(&data_dir, &locale)
            .context("Failed to load MapDifficulty.db2 — check DataDir and DBC.Locale config")?,
    );
    info!(
        "Loaded {} map difficulties from MapDifficulty.db2",
        map_difficulty_store.len()
    );
    let map_difficulty_x_condition_store = Arc::new(
        wow_data::MapDifficultyXConditionStore::load(&data_dir, &locale).context(
            "Failed to load MapDifficultyXCondition.db2 — check DataDir and DBC.Locale config",
        )?,
    );
    info!(
        "Loaded {} map difficulty conditions from MapDifficultyXCondition.db2",
        map_difficulty_x_condition_store.len()
    );
    let lfg_dungeons_store = Arc::new(
        wow_data::LfgDungeonsStore::load(&data_dir, &locale)
            .context("Failed to load LFGDungeons.db2 — check DataDir and DBC.Locale config")?,
    );
    info!(
        "Loaded {} LFG dungeons from LFGDungeons.db2",
        lfg_dungeons_store.len()
    );

    let (canonical_spawn_metadata, canonical_spawn_report) =
        spawn_store_loader::load_canonical_spawn_store_like_cpp(
            world_db.as_ref(),
            &char_db,
            &map_store,
            &map_difficulty_store,
            &spawn_group_store,
        )
        .await
        .context("Failed to load canonical SpawnStore metadata from world DB")?;
    info!(
        "Loaded canonical SpawnStore metadata: creatures rows={} indexed={} event-managed={} empty-difficulty={} missing-map={}; formations rows={} loaded={} missing-leader={} missing-member={} duplicate-member={} pruned-missing-leader-self={}; gameobjects rows={} indexed={} event-managed={} empty-difficulty={} missing-map={}; areatriggers rows={} indexed={} empty-difficulty={} missing-map={}; poolmgr templates rows={} loaded={} creature-members loaded={}/{} gameobject-members loaded={}/{} pool-members loaded={}/{} relation-removals={} map-mismatches={} circular={} empty={} missing-map={} autospawn loaded={}/{} skipped-empty={} skipped-broken={} skipped-child={}; spawn-group rows={} assigned={} missing-spawn={} invalid-type={} missing-group={} map-mismatch={} duplicate={}; represented validations skipped: creature={} gameobject={} areatrigger={}",
        canonical_spawn_report.creature.rows,
        canonical_spawn_report.creature.indexed,
        canonical_spawn_report.creature.skipped_event,
        canonical_spawn_report.creature.skipped_empty_difficulties,
        canonical_spawn_report.creature.skipped_missing_map,
        canonical_spawn_report.creature_formations.rows,
        canonical_spawn_report.creature_formations.loaded,
        canonical_spawn_report
            .creature_formations
            .skipped_missing_leader,
        canonical_spawn_report
            .creature_formations
            .skipped_missing_member,
        canonical_spawn_report
            .creature_formations
            .duplicate_member_ignored,
        canonical_spawn_report
            .creature_formations
            .removed_missing_leader_self,
        canonical_spawn_report.gameobject.rows,
        canonical_spawn_report.gameobject.indexed,
        canonical_spawn_report.gameobject.skipped_event,
        canonical_spawn_report.gameobject.skipped_empty_difficulties,
        canonical_spawn_report.gameobject.skipped_missing_map,
        canonical_spawn_report.area_trigger.rows,
        canonical_spawn_report.area_trigger.indexed,
        canonical_spawn_report
            .area_trigger
            .skipped_empty_difficulties,
        canonical_spawn_report.area_trigger.skipped_missing_map,
        canonical_spawn_report.pool_mgr.template_rows,
        canonical_spawn_report.pool_mgr.templates_loaded,
        canonical_spawn_report.pool_mgr.creature_members.loaded,
        canonical_spawn_report.pool_mgr.creature_members.rows,
        canonical_spawn_report.pool_mgr.gameobject_members.loaded,
        canonical_spawn_report.pool_mgr.gameobject_members.rows,
        canonical_spawn_report.pool_mgr.pool_members.loaded,
        canonical_spawn_report.pool_mgr.pool_members.rows,
        canonical_spawn_report.pool_mgr.relation_removals,
        canonical_spawn_report.pool_mgr.map_mismatches,
        canonical_spawn_report.pool_mgr.circular_relations,
        canonical_spawn_report.pool_mgr.empty_pools,
        canonical_spawn_report.pool_mgr.missing_map_after_non_empty,
        canonical_spawn_report.pool_mgr.autospawn_loaded,
        canonical_spawn_report.pool_mgr.autospawn_rows,
        canonical_spawn_report.pool_mgr.autospawn_skipped_empty,
        canonical_spawn_report.pool_mgr.autospawn_skipped_broken,
        canonical_spawn_report.pool_mgr.autospawn_skipped_child,
        canonical_spawn_report.spawn_group_rows,
        canonical_spawn_report.spawn_group_apply.assigned,
        canonical_spawn_report.spawn_group_apply.missing_spawn,
        canonical_spawn_report.spawn_group_apply.invalid_type,
        canonical_spawn_report.spawn_group_apply.missing_group,
        canonical_spawn_report.spawn_group_apply.map_mismatch,
        canonical_spawn_report
            .spawn_group_apply
            .duplicate_spawn_group,
        canonical_spawn_report.creature.validation_skipped,
        canonical_spawn_report.gameobject.validation_skipped,
        canonical_spawn_report.area_trigger.validation_skipped,
    );
    let (persisted_respawn_times, persisted_respawn_report) =
        load_persisted_respawn_times_like_cpp(&char_db, &canonical_spawn_metadata)
            .await
            .context("Failed to load persisted respawn times from character database")?;
    let persisted_respawn_times = Arc::new(persisted_respawn_times);
    info!(
        "Loaded persisted C++ respawn timers: rows={} loaded={} maps={} timers={} invalid-type={} unsupported-areatrigger={} missing-spawn-metadata={}",
        persisted_respawn_report.rows,
        persisted_respawn_report.loaded,
        persisted_respawn_times.maps_len(),
        persisted_respawn_times.respawns_len(),
        persisted_respawn_report.invalid_type,
        persisted_respawn_report.unsupported_area_trigger,
        persisted_respawn_report.missing_spawn_metadata,
    );
    let canonical_spawn_metadata: SharedCanonicalSpawnMetadataLikeCpp =
        Arc::new(Mutex::new(canonical_spawn_metadata));
    let (world_state_mgr, world_state_mgr_report) =
        spawn_store_loader::load_world_state_mgr_like_cpp(
            world_db.as_ref(),
            char_db.as_ref(),
            &map_store,
            &area_table_store,
        )
        .await
        .context("Failed to load C++ WorldStateMgr startup state")?;
    info!(
        "Loaded C++ WorldStateMgr startup state: template rows={} loaded={} skipped-map-list={} skipped-area-list={} realm-area-ignored={} saved rows={} applied={} skipped-unknown={}",
        world_state_mgr_report.template_rows,
        world_state_mgr_report.templates_loaded,
        world_state_mgr_report.skipped_invalid_map_list,
        world_state_mgr_report.skipped_invalid_area_list,
        world_state_mgr_report.realm_area_requirements_ignored,
        world_state_mgr_report.saved_rows,
        world_state_mgr_report.saved_applied,
        world_state_mgr_report.saved_skipped_unknown,
    );
    let world_state_mgr: SharedWorldStateMgrLikeCpp = Arc::new(Mutex::new(world_state_mgr));

    let mount_store = Arc::new(
        wow_data::MountStore::load_with_hotfixes(&data_dir, &locale, &hotfix_db)
            .await
            .context("Failed to load Mount.db2 / hotfix rows")?,
    );
    info!("Loaded {} mounts from Mount.db2", mount_store.len());
    let mount_capability_store = Arc::new(
        wow_data::MountCapabilityStore::load_with_hotfixes(&data_dir, &locale, &hotfix_db)
            .await
            .context("Failed to load MountCapability.db2 / hotfix rows")?,
    );
    info!(
        "Loaded {} mount capabilities from MountCapability.db2",
        mount_capability_store.len()
    );
    let mount_type_x_capability_store = Arc::new(
        wow_data::MountTypeXCapabilityStore::load_with_hotfixes(&data_dir, &locale, &hotfix_db)
            .await
            .context("Failed to load MountTypeXCapability.db2 / hotfix rows")?,
    );
    info!(
        "Loaded {} mount type capability rows from MountTypeXCapability.db2",
        mount_type_x_capability_store.len()
    );
    let mount_x_display_store = Arc::new(
        wow_data::MountXDisplayStore::load_with_hotfixes(&data_dir, &locale, &hotfix_db)
            .await
            .context("Failed to load MountXDisplay.db2 / hotfix rows")?,
    );
    info!(
        "Loaded {} mount display rows from MountXDisplay.db2",
        mount_x_display_store.len()
    );
    let difficulty_store = Arc::new(
        wow_data::DifficultyStore::load(&data_dir, &locale)
            .context("Failed to load Difficulty.db2 — check DataDir and DBC.Locale config")?,
    );
    info!(
        "Loaded {} difficulties from Difficulty.db2",
        difficulty_store.len()
    );
    let faction_store = Arc::new(
        wow_data::Db2IdStore::load(&data_dir, &locale, "Faction.db2")
            .context("Failed to load Faction.db2 — check DataDir and DBC.Locale config")?,
    );
    let achievement_store = Arc::new(
        wow_data::Db2IdStore::load(&data_dir, &locale, "Achievement.db2")
            .context("Failed to load Achievement.db2 — check DataDir and DBC.Locale config")?,
    );
    let criteria_store = Arc::new(
        wow_data::Db2IdStore::load(&data_dir, &locale, "Criteria.db2")
            .context("Failed to load Criteria.db2 — check DataDir and DBC.Locale config")?,
    );
    let battlemaster_list_store = Arc::new(
        wow_data::Db2IdStore::load(&data_dir, &locale, "BattlemasterList.db2")
            .context("Failed to load BattlemasterList.db2 — check DataDir and DBC.Locale config")?,
    );
    let battlemaster_list_typed_store = Arc::new(
        wow_data::BattlemasterListStore::load(&data_dir, &locale)
            .context("Failed to load typed BattlemasterList.db2 HolidayWorldState store")?,
    );
    let char_titles_store = Arc::new(
        wow_data::Db2IdStore::load(&data_dir, &locale, "CharTitles.db2")
            .context("Failed to load CharTitles.db2 — check DataDir and DBC.Locale config")?,
    );
    let battle_pet_species_store = Arc::new(
        wow_data::Db2IdStore::load(&data_dir, &locale, "BattlePetSpecies.db2")
            .context("Failed to load BattlePetSpecies.db2 — check DataDir and DBC.Locale config")?,
    );
    let scenario_step_store = Arc::new(
        wow_data::Db2IdStore::load(&data_dir, &locale, "ScenarioStep.db2")
            .context("Failed to load ScenarioStep.db2 — check DataDir and DBC.Locale config")?,
    );
    let scene_script_package_store = Arc::new(
        wow_data::Db2IdStore::load(&data_dir, &locale, "SceneScriptPackage.db2").context(
            "Failed to load SceneScriptPackage.db2 — check DataDir and DBC.Locale config",
        )?,
    );
    let player_condition_store = Arc::new(
        wow_data::PlayerConditionStore::load(&data_dir, &locale)
            .context("Failed to load PlayerCondition.db2 — check DataDir and DBC.Locale config")?,
    );
    let content_tuning_store = Arc::new(
        wow_data::progression_rewards::ContentTuningStore::load(&data_dir, &locale)
            .context("Failed to load ContentTuning.db2 — check DataDir and DBC.Locale config")?,
    );
    info!(
        "Loaded {} content tuning rows from ContentTuning.db2",
        content_tuning_store.len()
    );
    let world_state_expression_store = Arc::new(
        wow_data::WorldStateExpressionStore::load(&data_dir, &locale).context(
            "Failed to load WorldStateExpression.db2 — check DataDir and DBC.Locale config",
        )?,
    );
    let conversation_line_store = Arc::new(
        wow_data::Db2IdStore::load(&data_dir, &locale, "ConversationLine.db2")
            .context("Failed to load ConversationLine.db2 — check DataDir and DBC.Locale config")?,
    );
    let conversation_line_template_store = Arc::new(
        wow_data::WorldIdStore::load_filtering_like_cpp(
            world_db.as_ref(),
            "conversation_line_template",
            WorldStatements::SEL_CONVERSATION_LINE_TEMPLATE_IDS,
            |line_id| conversation_line_store.contains(line_id),
        )
        .await
        .context("Failed to load conversation_line_template ids for C++ ConditionMgr validation")?,
    );
    info!(
        "Loaded condition validation DB2 id stores: {} factions, {} achievements, {} criteria, {} battlemaster lists, {} typed battlemaster holiday-world-state rows, {} titles, {} battle pet species, {} scenario steps, {} scene script packages, {} player conditions, {} world state expressions, {} conversation lines",
        faction_store.len(),
        achievement_store.len(),
        criteria_store.len(),
        battlemaster_list_store.len(),
        battlemaster_list_typed_store.len(),
        char_titles_store.len(),
        battle_pet_species_store.len(),
        scenario_step_store.len(),
        scene_script_package_store.len(),
        player_condition_store.len(),
        world_state_expression_store.len(),
        conversation_line_store.len()
    );
    info!(
        "Loaded condition validation conversation line template store: {} templates",
        conversation_line_template_store.len()
    );

    // Load ItemAppearance.db2 for item display-info resolution.
    let item_appearance_store = Arc::new(
        wow_data::ItemAppearanceStore::load(&data_dir, &locale)
            .context("Failed to load ItemAppearance.db2 — check DataDir and DBC.Locale config")?,
    );
    info!(
        "Loaded {} item appearances from ItemAppearance.db2",
        item_appearance_store.len()
    );

    // Load ItemModifiedAppearance.db2 for transmog/visible-item appearance resolution.
    let item_modified_appearance_store = Arc::new(
        wow_data::ItemModifiedAppearanceStore::load(&data_dir, &locale).context(
            "Failed to load ItemModifiedAppearance.db2 — check DataDir and DBC.Locale config",
        )?,
    );
    info!(
        "Loaded {} item modified appearances from ItemModifiedAppearance.db2",
        item_modified_appearance_store.len()
    );

    // Load player level stats from world DB
    let player_stats = Arc::new(
        wow_data::PlayerStatsStore::load(&world_db)
            .await
            .context("Failed to load player_levelstats")?,
    );
    info!("Loaded {} player level stat entries", player_stats.len());

    // Load item stat modifiers from ItemSparse.db2 (gear bonuses: STR, AGI, STA, etc.)
    let item_stats_store = Arc::new(
        wow_data::ItemStatsStore::load(&data_dir, &locale)
            .context("Failed to load ItemSparse.db2 — check DataDir and DBC.Locale config")?,
    );
    info!(
        "Loaded {} items with stat modifiers from ItemSparse.db2",
        item_stats_store.len()
    );

    // C++ global DB2 stores used by Item::CalculateDurabilityRepairCost.
    let durability_costs_store = Arc::new(
        wow_data::DurabilityCostsStore::load(&data_dir, &locale)
            .context("Failed to load DurabilityCosts.db2 — check DataDir and DBC.Locale config")?,
    );
    info!(
        "Loaded {} durability cost rows from DurabilityCosts.db2",
        durability_costs_store.len()
    );

    let durability_quality_store = Arc::new(
        wow_data::DurabilityQualityStore::load(&data_dir, &locale).context(
            "Failed to load DurabilityQuality.db2 — check DataDir and DBC.Locale config",
        )?,
    );
    info!(
        "Loaded {} durability quality rows from DurabilityQuality.db2",
        durability_quality_store.len()
    );

    // Load Lock.db2 for C++ sLockStore existence checks during CMSG_OPEN_ITEM.
    let lock_store = Arc::new(
        wow_data::LockStore::load(&data_dir, &locale)
            .context("Failed to load Lock.db2 — check DataDir and DBC.Locale config")?,
    );
    info!("Loaded {} locks from Lock.db2", lock_store.len());

    // Load ItemRandomSuffix.db2 for C++ ApplyEnchantment random suffix amount resolution.
    let item_random_suffix_store = Arc::new(
        wow_data::ItemRandomSuffixStore::load(&data_dir, &locale)
            .context("Failed to load ItemRandomSuffix.db2 — check DataDir and DBC.Locale config")?,
    );
    info!(
        "Loaded {} item random suffixes from ItemRandomSuffix.db2",
        item_random_suffix_store.len()
    );

    // Load ItemRandomProperties.db2 and RandPropPoints.db2 plus the world-table
    // random enchantment groups for C++ ItemEnchantmentMgr::GenerateRandomProperties.
    let item_random_properties_store = Arc::new(
        wow_data::ItemRandomPropertiesStore::load(&data_dir, &locale).context(
            "Failed to load ItemRandomProperties.db2 — check DataDir and DBC.Locale config",
        )?,
    );
    info!(
        "Loaded {} item random properties from ItemRandomProperties.db2",
        item_random_properties_store.len()
    );

    let rand_prop_points_store = Arc::new(
        wow_data::RandPropPointsStore::load(&data_dir, &locale)
            .context("Failed to load RandPropPoints.db2 — check DataDir and DBC.Locale config")?,
    );
    info!(
        "Loaded {} random property point rows from RandPropPoints.db2",
        rand_prop_points_store.len()
    );

    // Load ItemDisenchantLoot.db2 for C++ sItemDisenchantLootStore lookup.
    let item_disenchant_loot_store = Arc::new(
        wow_data::ItemDisenchantLootStore::load(&data_dir, &locale).context(
            "Failed to load ItemDisenchantLoot.db2 — check DataDir and DBC.Locale config",
        )?,
    );
    info!(
        "Loaded {} item disenchant loot rows from ItemDisenchantLoot.db2",
        item_disenchant_loot_store.len()
    );

    let item_random_enchantment_template_store = Arc::new(
        wow_data::ItemRandomEnchantmentTemplateStore::load_validated(
            &world_db,
            &item_random_properties_store,
            &item_random_suffix_store,
        )
        .await
        .context("Failed to load item_random_enchantment_template")?,
    );

    // Load SpellItemEnchantment.db2 for ApplyEnchantment and arena enchantment checks.
    let spell_item_enchantment_store = Arc::new(
        wow_data::SpellItemEnchantmentStore::load(&data_dir, &locale).context(
            "Failed to load SpellItemEnchantment.db2 — check DataDir and DBC.Locale config",
        )?,
    );
    info!(
        "Loaded {} spell item enchantments from SpellItemEnchantment.db2",
        spell_item_enchantment_store.len()
    );

    // Build hotfix blob cache — pre-loads raw DB2 record bytes and hotfix DB overlays for DBReply.
    let mut hotfix_blob_cache = wow_data::build_hotfix_blob_cache(&data_dir, &locale);
    match hotfix_blob_cache
        .load_hotfix_blobs_from_db(&hotfix_db, &locale)
        .await
    {
        Ok(n) => info!("HotfixBlobCache: loaded {n} hotfix_blob rows"),
        Err(e) => tracing::warn!("HotfixBlobCache: failed to load hotfix_blob rows: {e}"),
    }
    match hotfix_blob_cache
        .load_hotfix_data_from_db(&hotfix_db, &locale)
        .await
    {
        Ok(n) => info!("HotfixBlobCache: loaded {n} hotfix_data rows"),
        Err(e) => tracing::warn!("HotfixBlobCache: failed to load hotfix_data rows: {e}"),
    }
    match hotfix_blob_cache
        .load_hotfix_optional_data_from_db(&hotfix_db, &locale)
        .await
    {
        Ok(n) => info!("HotfixBlobCache: loaded {n} hotfix_optional_data rows"),
        Err(e) => tracing::warn!("HotfixBlobCache: failed to load hotfix_optional_data rows: {e}"),
    }
    let hotfix_blob_cache = Arc::new(hotfix_blob_cache);

    // Load SkillLineAbility.db2 + SkillRaceClassInfo.db2 for auto-learned spells
    let skill_store = Arc::new(
        wow_data::SkillStore::load(&data_dir, &locale)
            .context("Failed to load SkillLineAbility/SkillRaceClassInfo DB2 files")?,
    );
    let skill_line_store = Arc::new(
        wow_data::SkillLineStore::load(&data_dir, &locale)
            .context("Failed to load SkillLine.db2")?,
    );

    // Load spell metadata (cast time, cooldown, effects, etc.) — Phase 2
    let mut spell_store = wow_data::SpellStore::load(&hotfix_db)
        .await
        .context("Failed to load SpellStore")?;
    info!("Loaded {} spells from SpellStore", spell_store.len());
    let spell_misc_store = Arc::new(
        wow_data::SpellMiscStore::load(&data_dir, &locale)
            .context("Failed to load SpellMisc.db2")?,
    );
    info!("Loaded {} spell misc rows", spell_misc_store.len());
    let spell_duration_store = Arc::new(
        wow_data::SpellDurationStore::load(&data_dir, &locale)
            .context("Failed to load SpellDuration.db2")?,
    );
    info!("Loaded {} spell duration rows", spell_duration_store.len());
    let spell_radius_store = Arc::new(
        wow_data::SpellRadiusStore::load(&data_dir, &locale)
            .context("Failed to load SpellRadius.db2")?,
    );
    info!("Loaded {} spell radius rows", spell_radius_store.len());
    let spell_range_store = Arc::new(
        wow_data::SpellRangeStore::load(&data_dir, &locale)
            .context("Failed to load SpellRange.db2")?,
    );
    info!("Loaded {} spell range rows", spell_range_store.len());

    // Load area trigger store (collision detection + teleportation)
    let area_trigger_store = Arc::new(
        wow_data::load_area_triggers(&world_db)
            .await
            .context("Failed to load area triggers")?,
    );

    // Load quest store (templates + objectives + NPC relations)
    let quest_store = Arc::new(
        wow_data::quest::load_quests(&world_db)
            .await
            .context("Failed to load quest store")?,
    );
    let disable_mgr = Arc::new(
        load_disable_mgr_like_cpp(
            world_db.as_ref(),
            &map_store,
            &map_difficulty_store,
            &spell_store,
            quest_store.as_ref(),
            criteria_store.as_ref(),
            battlemaster_list_store.as_ref(),
        )
        .await?,
    );
    let mmap_disabled_map_ids = disable_mgr.disabled_mmap_map_ids_like_cpp();
    info!(
        "Loaded {} C++ mmap disable rows",
        mmap_disabled_map_ids.len()
    );

    let loaded_loot_stores = load_loot_stores_like_cpp(&world_db, &item_store)
        .await
        .context("Failed to load C++ LootTemplates_* foundation stores")?;
    let loot_reference_report = check_loot_references_like_cpp(&loaded_loot_stores);
    log_loot_reference_report_like_cpp(&loot_reference_report);
    let loot_condition_ids = load_loot_condition_ids_like_cpp(&world_db)
        .await
        .context("Failed to load C++ loot-template condition IDs")?;
    let mut loot_condition_report =
        check_loot_condition_links_like_cpp(&loaded_loot_stores, loot_condition_ids, |item_id| {
            item_store.get(item_id).is_some()
        });
    let loot_condition_reference_uses = load_loot_condition_reference_uses_like_cpp(&world_db)
        .await
        .context("Failed to load C++ loot-template condition reference uses")?;
    let condition_reference_template_ids =
        load_condition_reference_template_ids_like_cpp(&world_db)
            .await
            .context("Failed to load C++ condition reference template IDs")?;
    check_loot_condition_references_like_cpp(
        &mut loot_condition_report,
        loot_condition_reference_uses,
        condition_reference_template_ids,
    );
    log_loot_condition_link_report_like_cpp(&loot_condition_report);
    let loot_stores = Arc::new(loaded_loot_stores);
    let loaded_loot_templates: usize = loot_stores
        .values()
        .map(|store| store.templates().len())
        .sum();
    info!(
        "Loaded {} C++ loot-template stores with {} template IDs",
        loot_stores.len(),
        loaded_loot_templates
    );

    // Load player_xp_for_level table
    let player_xp_table = {
        let mut stmt = world_db.prepare(WorldStatements::SEL_PLAYER_XP_FOR_LEVEL);
        let mut table = vec![0u32; 82]; // index = level, 0=unused, 81=max
        if let Ok(result) = world_db.query(&stmt).await {
            let mut r = result;
            loop {
                let lvl: u8 = r.try_read::<u8>(0).unwrap_or(0);
                let xp: u32 = r.try_read::<u32>(1).unwrap_or(0);
                if (lvl as usize) < table.len() {
                    table[lvl as usize] = xp;
                }
                if !r.next_row() {
                    break;
                }
            }
        }
        Arc::new(table)
    };

    // Load QuestXP.db2 for accurate XP rewards
    let dbc_path = format!("{}/dbc/{}", data_dir, locale);
    let quest_xp_store = Arc::new(
        wow_data::quest_xp::QuestXpStore::load(&dbc_path).unwrap_or_else(|e| {
            tracing::warn!("QuestXP.db2 not loaded ({e}), using fallback XP table");
            wow_data::quest_xp::QuestXpStore::default()
        }),
    );
    let quest_v2_store = Arc::new(
        wow_data::progression_rewards::QuestV2Store::load(&data_dir, &locale)
            .context("Failed to load QuestV2.db2 — check DataDir and DBC.Locale config")?,
    );
    let quest_info_store = Arc::new(
        wow_data::progression_rewards::QuestInfoStore::load(&data_dir, &locale)
            .context("Failed to load QuestInfo.db2 — check DataDir and DBC.Locale config")?,
    );
    let quest_package_item_store = Arc::new(
        wow_data::progression_rewards::QuestPackageItemStore::load(&data_dir, &locale)
            .context("Failed to load QuestPackageItem.db2 — check DataDir and DBC.Locale config")?,
    );
    let quest_faction_reward_store = Arc::new(
        wow_data::progression_rewards::QuestFactionRewardStore::load(&data_dir, &locale).context(
            "Failed to load QuestFactionReward.db2 — check DataDir and DBC.Locale config",
        )?,
    );
    let progression_faction_store = Arc::new(
        wow_data::progression_rewards::FactionStore::load(&data_dir, &locale).context(
            "Failed to load Faction.db2 progression store — check DataDir and DBC.Locale config",
        )?,
    );
    let faction_template_store = Arc::new(
        wow_data::progression_rewards::FactionTemplateStore::load(&data_dir, &locale)
            .context("Failed to load FactionTemplate.db2 — check DataDir and DBC.Locale config")?,
    );
    let friendship_rep_reaction_store = Arc::new(
        wow_data::progression_rewards::FriendshipRepReactionStore::load(&data_dir, &locale)
            .context(
                "Failed to load FriendshipRepReaction.db2 — check DataDir and DBC.Locale config",
            )?,
    );
    let paragon_reputation_store = Arc::new(
        wow_data::progression_rewards::ParagonReputationStore::load(&data_dir, &locale).context(
            "Failed to load ParagonReputation.db2 — check DataDir and DBC.Locale config",
        )?,
    );
    let (reputation_reward_rate_store, reputation_reward_rate_report) =
        wow_data::reputation::ReputationRewardRateStoreLikeCpp::load_like_cpp(
            &world_db,
            &progression_faction_store,
        )
        .await
        .context("Failed to load reputation_reward_rate")?;
    let reputation_reward_rate_store = Arc::new(reputation_reward_rate_store);
    tracing::info!(
        loaded = reputation_reward_rate_store.len(),
        skipped = reputation_reward_rate_report.skipped.len(),
        "Loaded reputation_reward_rate like C++"
    );
    let (creature_onkill_reputation_store, creature_onkill_reputation_report) =
        wow_data::reputation::CreatureOnKillReputationStoreLikeCpp::load_like_cpp(
            &world_db,
            &creature_template_lifecycle_store,
            &progression_faction_store,
        )
        .await
        .context("Failed to load creature_onkill_reputation")?;
    let creature_onkill_reputation_store = Arc::new(creature_onkill_reputation_store);
    tracing::info!(
        loaded = creature_onkill_reputation_store.len(),
        skipped = creature_onkill_reputation_report.skipped.len(),
        "Loaded creature_onkill_reputation like C++"
    );
    let (reputation_spillover_template_store, reputation_spillover_template_report) =
        wow_data::reputation::RepSpilloverTemplateStoreLikeCpp::load_like_cpp(
            &world_db,
            &progression_faction_store,
        )
        .await
        .context("Failed to load reputation_spillover_template")?;
    let reputation_spillover_template_store = Arc::new(reputation_spillover_template_store);
    tracing::info!(
        loaded = reputation_spillover_template_store.len(),
        skipped = reputation_spillover_template_report.skipped.len(),
        "Loaded reputation_spillover_template like C++"
    );

    // Get realm ID and load build-specific auth seed
    let realm_id: u16 = wow_config::get_value("RealmID").unwrap_or(1);

    let (realm_build, win64_auth_seed) = load_realm_auth_seed(&login_db, realm_id).await?;
    info!("Realm {realm_id} build {realm_build}, Win64AuthSeed loaded");

    // Load realm addresses from realmlist table (for ConnectTo)
    let (realm_external_address, realm_local_address) =
        load_realm_addresses(&login_db, realm_id).await?;
    info!(
        "Realm addresses: external={}, local={}",
        format_ipv4(realm_external_address),
        format_ipv4(realm_local_address),
    );

    // Wrap login_db in Arc for sharing between account lookup and sessions
    let login_db = Arc::new(login_db);

    // Build handler dispatch table
    let table = wow_handler::build_dispatch_table();
    info!("Loaded {} packet handlers", table.len());

    // Build account lookup
    let account_lookup: Arc<dyn AccountLookup> = Arc::new(DbAccountLookup {
        login_db: Arc::clone(&login_db),
        realm_id,
        win64_auth_seed,
    });

    // Shared player registry for broadcast (chat, emotes, movement)
    let player_registry = Arc::new(PlayerRegistry::new());
    let object_accessor = wow_world::new_shared_object_accessor();

    let mut condition_load_report =
        wow_data::conditions::load_condition_rows_like_cpp(world_db.as_ref(), |_| 0)
            .await
            .context("Failed to load C++ conditions table")?;
    let loot_template_exists = |source_type: wow_constants::ConditionSourceType,
                                source_group: u32| {
        loot_store_kind_for_condition_source_type_like_cpp(source_type as i32)
            .and_then(|kind| loot_stores.get(&kind))
            .is_some_and(|store| store.have_loot_for(source_group))
    };
    let loot_source_entry_exists = |source_type: wow_constants::ConditionSourceType,
                                    source_group: u32,
                                    source_entry: i32| {
        let Some(source_entry) = u32::try_from(source_entry).ok() else {
            return false;
        };
        let Some(store) = loot_store_kind_for_condition_source_type_like_cpp(source_type as i32)
            .and_then(|kind| loot_stores.get(&kind))
        else {
            return false;
        };
        let Some(template) = store.get_loot_for(source_group) else {
            return false;
        };

        item_store.get(source_entry).is_some() || template.is_reference_like_cpp(source_entry)
    };
    let externally_skipped_conditions =
        wow_data::conditions::apply_external_condition_validation_like_cpp(
            &mut condition_load_report,
            wow_data::conditions::ConditionExternalValidationStoresLikeCpp {
                item_store: Some(item_store.as_ref()),
                spell_store: Some(&spell_store),
                area_table_store: Some(area_table_store.as_ref()),
                skill_store: Some(skill_store.as_ref()),
                map_store: Some(map_store.as_ref()),
                phase_store: Some(phase_store.as_ref()),
                quest_store: Some(quest_store.as_ref()),
                area_trigger_store: Some(area_trigger_store.as_ref()),
                graveyard_store: Some(&graveyard_store),
                spawn_group_store: Some(&spawn_group_store),
                creature_template_store: Some(creature_template_store.as_ref()),
                gameobject_template_store: Some(gameobject_template_store.as_ref()),
                trainer_store: Some(trainer_store.as_ref()),
                conversation_line_template_store: Some(conversation_line_template_store.as_ref()),
                area_trigger_template_store: Some(area_trigger_template_store.as_ref()),
                creature_spawn_store: Some(creature_spawn_store.as_ref()),
                gameobject_spawn_store: Some(gameobject_spawn_store.as_ref()),
                active_event_store: Some(active_event_store.as_ref()),
                world_state_store: Some(world_state_store.as_ref()),
                difficulty_store: Some(difficulty_store.as_ref()),
                faction_store: Some(faction_store.as_ref()),
                achievement_store: Some(achievement_store.as_ref()),
                char_titles_store: Some(char_titles_store.as_ref()),
                battle_pet_species_store: Some(battle_pet_species_store.as_ref()),
                scenario_step_store: Some(scenario_step_store.as_ref()),
                scene_script_package_store: Some(scene_script_package_store.as_ref()),
                player_condition_store: Some(player_condition_store.as_ref()),
                max_skill_value: Some(max_skill_value_like_cpp(&world_configs)),
                loot_template_exists: Some(&loot_template_exists),
                loot_source_entry_exists: Some(&loot_source_entry_exists),
            },
        );
    for skipped in &condition_load_report.skipped {
        warn!(
            "Condition row skipped during C++ load-shape parsing: {:?}: {:?}",
            skipped.row, skipped.reason
        );
    }
    for skipped in &externally_skipped_conditions {
        warn!(
            "Condition row skipped during C++ external validation: {:?}: {:?}",
            skipped.condition, skipped.reason
        );
    }
    for warning in &condition_load_report.warnings {
        warn!("Condition load warning: {warning:?}");
    }
    let condition_store = Arc::new(condition_load_report.into_store_like_cpp());
    let condition_attachment_report = wow_data::attach_loaded_conditions_like_cpp(
        condition_store.as_ref(),
        Some(&mut gossip_store),
        Some(&mut spell_store),
        Some(&mut phase_info_store),
        Some(&mut graveyard_store),
    );
    for missing in &condition_attachment_report.gossip_menus.missing_menus {
        warn!(
            "ConditionMgr gossip attachment warning: GossipMenu {} not found for condition id {:?}",
            missing.source_group, missing
        );
    }
    for missing in &condition_attachment_report
        .gossip_menu_items
        .missing_menu_items
    {
        warn!(
            "ConditionMgr gossip attachment warning: GossipMenuId {} Item {} not found for condition id {:?}",
            missing.source_group, missing.source_entry, missing
        );
    }
    info!(
        "Loaded C++ ConditionMgr store: {} buckets, {} externally skipped conditions, {} spell-click aura spell ids, {} spell implicit target condition rows attached ({} deferred), {} gossip menu condition rows attached ({} missing menus), {} gossip menu option condition rows attached ({} missing items), {} phase condition rows attached, {} graveyard condition rows attached",
        condition_store.bucket_count(),
        externally_skipped_conditions.len(),
        condition_attachment_report.spell_click_aura_spell_ids.len(),
        condition_attachment_report.spell_implicit_target_condition_count,
        condition_attachment_report.deferred_spell_implicit_target_condition_count,
        condition_attachment_report
            .gossip_menus
            .attached_condition_count,
        condition_attachment_report.gossip_menus.missing_menus.len(),
        condition_attachment_report
            .gossip_menu_items
            .attached_condition_count,
        condition_attachment_report
            .gossip_menu_items
            .missing_menu_items
            .len(),
        condition_attachment_report.phases.attached_condition_count,
        condition_attachment_report
            .graveyards
            .attached_condition_count
    );
    wow_world::conditions::set_condition_mgr_store_like_cpp(Arc::clone(&condition_store));
    let npc_spell_click_store = Arc::new(
        wow_data::NpcSpellClickStoreLikeCpp::load_like_cpp(
            world_db.as_ref(),
            creature_template_lifecycle_store.as_ref(),
            &spell_store,
        )
        .await
        .context("Failed to load C++ npc_spellclick_spells rows")?,
    );
    info!(
        "Loaded {} C++ npc_spellclick_spells rows ({} missing creature templates, {} missing spells, {} invalid user types logged-but-loaded like C++)",
        npc_spell_click_store.len(),
        npc_spell_click_store
            .load_report_like_cpp()
            .skipped_missing_creature_template,
        npc_spell_click_store
            .load_report_like_cpp()
            .skipped_missing_spell,
        npc_spell_click_store
            .load_report_like_cpp()
            .invalid_user_type_logged_but_loaded_like_cpp
    );
    let spell_store = Arc::new(spell_store);

    // Shared group registry and pending invites
    let group_registry = Arc::new(GroupRegistry::new());
    let pending_invites = Arc::new(PendingInvites::new());

    // Shared world state (creatures/grids visible to every session on the same map).
    // Each session gets a clone of this Arc on creation.
    let shared_map: SharedMapManager = Arc::new(std::sync::RwLock::new(LegacyMapManager::new()));
    let mut loaded_instance_lock_mgr = InstanceLockMgr::default();
    let instance_lock_load_issues = loaded_instance_lock_mgr
        .load_from_database_like_cpp(&char_db, |map_id, difficulty_id| {
            map_db2_entries_from_stores(&map_store, &map_difficulty_store, map_id, difficulty_id)
        })
        .await
        .context("Failed to load instance locks from character database")?;
    for issue in &instance_lock_load_issues {
        warn!("Instance lock load issue: {issue:?}");
    }
    let instance_lock_stats = loaded_instance_lock_mgr.statistics();
    info!(
        "Loaded instance locks: {} shared instances, {} players, {} issues",
        instance_lock_stats.instance_count,
        instance_lock_stats.player_count,
        instance_lock_load_issues.len()
    );
    let registered_instance_ids = loaded_instance_lock_mgr.registered_instance_ids_like_cpp_order();
    let instance_lock_mgr = Arc::new(std::sync::RwLock::new(loaded_instance_lock_mgr));

    let canonical_map_manager = Arc::new(Mutex::new(create_canonical_map_manager(&world_configs)));
    match canonical_map_manager.lock() {
        Ok(mut manager) => install_canonical_spawn_group_initializer_like_cpp(
            &mut manager,
            Arc::clone(&canonical_spawn_metadata),
            Arc::clone(&condition_store),
            Arc::clone(&persisted_respawn_times),
        ),
        Err(_) => {
            warn!("Canonical MapManager lock poisoned; InitSpawnGroupState hook not installed")
        }
    }
    register_loaded_instance_ids(
        &shared_map,
        canonical_map_manager.as_ref(),
        &registered_instance_ids,
    );

    let loaded_grid_creature_respawn_caches = LoadedGridCreatureRespawnCachesLikeCpp {
        template_store: Arc::clone(&creature_template_lifecycle_store),
        difficulty_store: Arc::clone(&creature_difficulty_store),
        base_stats_store: Arc::clone(&creature_base_stats_store),
        health_rates: creature_health_rates,
        display_store: Arc::clone(&creature_display_info_store),
        model_store: Arc::clone(&creature_model_data_store),
        vehicle_store: Arc::clone(&vehicle_store),
        vehicle_seat_store: Arc::clone(&vehicle_seat_store),
        vehicle_accessory_store: Arc::clone(&vehicle_accessory_store),
        gameobject_template_store: Arc::clone(&gameobject_template_lifecycle_store),
        gameobject_override_store: Arc::clone(&gameobject_override_lifecycle_store),
    };

    let game_event_scheduler = {
        let current_time_secs = current_unix_time_secs_like_cpp();
        let (game_event_outcome, active_event_ids, mut db_bridge_summary) = {
            let mut canonical_spawn_metadata = canonical_spawn_metadata.lock().map_err(|_| {
                anyhow::anyhow!(
                    "CanonicalSpawnMetadataLikeCpp mutex poisoned during GameEvent StartSystem"
                )
            })?;
            canonical_spawn_metadata.clear_active_game_events_like_cpp();
            let outcome = canonical_spawn_metadata.update_game_events_like_cpp(
                current_time_secs,
                false,
                represented_game_event_world_conditions_met_like_cpp,
            );
            let db_bridge_summary = materialize_game_event_world_event_state_db_bridge_like_cpp(
                &outcome,
                &canonical_spawn_metadata,
            );
            let active_event_ids = canonical_spawn_metadata
                .game_event_active_set_like_cpp()
                .active_event_ids_like_cpp()
                .collect::<Vec<_>>();
            (outcome, active_event_ids, db_bridge_summary)
        };
        execute_game_event_world_event_state_db_bridge_like_cpp(
            char_db.as_ref(),
            &mut db_bridge_summary,
        )
        .await;
        let mut side_effect_summary = {
            let mut manager = canonical_map_manager.lock().map_err(|_| {
                anyhow::anyhow!("Canonical MapManager mutex poisoned during GameEvent StartSystem")
            })?;
            let mut canonical_spawn_metadata = canonical_spawn_metadata.lock().map_err(|_| {
                anyhow::anyhow!("CanonicalSpawnMetadataLikeCpp mutex poisoned during GameEvent StartSystem side effects")
            })?;
            let mut world_state_mgr = world_state_mgr.lock().map_err(|_| {
                anyhow::anyhow!(
                    "WorldStateMgrLikeCpp mutex poisoned during GameEvent StartSystem side effects"
                )
            })?;
            consume_game_event_live_update_side_effects_like_cpp(
                &mut manager,
                &mut canonical_spawn_metadata,
                &loaded_grid_creature_respawn_caches,
                Some(battlemaster_list_typed_store.as_ref()),
                Some(&mut world_state_mgr),
                Some(player_registry.as_ref()),
                &active_event_ids,
                &game_event_outcome,
                false,
            )
        };
        execute_game_event_seasonal_quest_db_deletes_like_cpp(
            char_db.as_ref(),
            &mut side_effect_summary,
        )
        .await;
        fanout_reset_event_seasonal_quests_to_player_sessions_after_db_delete_like_cpp(
            Some(player_registry.as_ref()),
            &mut side_effect_summary,
        );
        debug!(
            scanned_event_ids = game_event_outcome.scanned_event_ids.len(),
            queued_activation_event_ids = game_event_outcome.queued_activation_event_ids.len(),
            queued_deactivation_event_ids = game_event_outcome.queued_deactivation_event_ids.len(),
            start_outcomes = game_event_outcome.start_outcomes.len(),
            stop_outcomes = game_event_outcome.stop_outcomes.len(),
            negative_spawn_event_ids = game_event_outcome.negative_spawn_event_ids.len(),
            world_nextphase_finished = game_event_outcome.world_nextphase_finished.len(),
            world_conditions_save_requested =
                game_event_outcome.world_conditions_save_requested.len(),
            game_event_db_saves_queued = db_bridge_summary.saves_queued,
            game_event_db_saves_executed = db_bridge_summary.saves_executed,
            game_event_db_saves_failed = db_bridge_summary.saves_failed,
            game_event_db_saves_skipped_event_id_out_of_range =
                db_bridge_summary.saves_skipped_event_id_out_of_range,
            game_event_db_saves_skipped_missing_event =
                db_bridge_summary.saves_skipped_missing_event,
            game_event_db_deletes_queued = db_bridge_summary.deletes_queued,
            game_event_db_deletes_executed = db_bridge_summary.deletes_executed,
            game_event_db_deletes_failed = db_bridge_summary.deletes_failed,
            game_event_db_deletes_skipped_event_id_out_of_range =
                db_bridge_summary.deletes_skipped_event_id_out_of_range,
            game_event_db_condition_delete_rows_queued =
                db_bridge_summary.condition_delete_rows_queued,
            game_event_db_condition_delete_rows_executed =
                db_bridge_summary.condition_delete_rows_executed,
            game_event_db_condition_delete_rows_failed =
                db_bridge_summary.condition_delete_rows_failed,
            invalid_check_outcomes = game_event_outcome.invalid_check_outcomes.len(),
            invalid_next_check_outcomes = game_event_outcome.invalid_next_check_outcomes.len(),
            next_update_delay_millis = game_event_outcome.next_update_delay_millis,
            side_effect_actions = side_effect_summary.actions.len(),
            spawn_actions = side_effect_summary.spawn_actions,
            unspawn_actions = side_effect_summary.unspawn_actions,
            announce_event_actions = side_effect_summary.announce_event_actions,
            announce_event_description_len_total =
                side_effect_summary.announce_event_description_len_total,
            announce_event_world_text_represented =
                side_effect_summary.announce_event_world_text_represented,
            announce_event_lines = side_effect_summary.announce_event_lines,
            announce_event_registry_missing = side_effect_summary.announce_event_registry_missing,
            announce_event_send_attempted = side_effect_summary.announce_event_send_attempted,
            announce_event_send_queued = side_effect_summary.announce_event_send_queued,
            announce_event_send_failed = side_effect_summary.announce_event_send_failed,
            announce_event_localization_unrepresented =
                side_effect_summary.announce_event_localization_unrepresented,
            announce_event_in_world_filter_unrepresented =
                side_effect_summary.announce_event_in_world_filter_unrepresented,
            announce_event_not_in_world_skipped =
                side_effect_summary.announce_event_not_in_world_skipped,
            announce_event_world_text_unimplemented =
                side_effect_summary.announce_event_world_text_unimplemented,
            announce_event_session_fanout_unimplemented =
                side_effect_summary.announce_event_session_fanout_unimplemented,
            change_equip_or_model_actions = side_effect_summary.change_equip_or_model_actions,
            change_equip_or_model_records_seen =
                side_effect_summary.change_equip_or_model_records_seen,
            change_equip_or_model_records_applied =
                side_effect_summary.change_equip_or_model_records_applied,
            change_equip_or_model_maps_matched =
                side_effect_summary.change_equip_or_model_maps_matched,
            change_equip_or_model_live_creatures_mutated =
                side_effect_summary.change_equip_or_model_live_creatures_mutated,
            change_equip_or_model_model_validation_unavailable =
                side_effect_summary.change_equip_or_model_model_validation_unavailable,
            update_event_quests_actions = side_effect_summary.update_event_quests_actions,
            update_event_quests_creature_records_seen =
                side_effect_summary.update_event_quests_creature_records_seen,
            update_event_quests_gameobject_records_seen =
                side_effect_summary.update_event_quests_gameobject_records_seen,
            update_event_quests_creature_inserted =
                side_effect_summary.update_event_quests_creature_inserted,
            update_event_quests_gameobject_inserted =
                side_effect_summary.update_event_quests_gameobject_inserted,
            update_event_quests_creature_removed =
                side_effect_summary.update_event_quests_creature_removed,
            update_event_quests_gameobject_removed =
                side_effect_summary.update_event_quests_gameobject_removed,
            update_event_quests_creature_skipped_active_other_event =
                side_effect_summary.update_event_quests_creature_skipped_active_other_event,
            update_event_quests_gameobject_skipped_active_other_event =
                side_effect_summary.update_event_quests_gameobject_skipped_active_other_event,
            update_world_states_actions = side_effect_summary.update_world_states_actions,
            update_world_states_no_holiday = side_effect_summary.update_world_states_no_holiday,
            update_world_states_missing_event =
                side_effect_summary.update_world_states_missing_event,
            update_world_states_store_missing = side_effect_summary.update_world_states_store_missing,
            update_world_states_holiday_not_weekend_battleground =
                side_effect_summary.update_world_states_holiday_not_weekend_battleground,
            update_world_states_battlemaster_list_missing =
                side_effect_summary.update_world_states_battlemaster_list_missing,
            update_world_states_holiday_world_state_zero =
                side_effect_summary.update_world_states_holiday_world_state_zero,
            update_world_states_holiday_lookup_unrepresented =
                side_effect_summary.update_world_states_holiday_lookup_unrepresented,
            update_world_states_set_value_represented =
                side_effect_summary.update_world_states_set_value_represented,
            update_world_states_last_world_state_id =
                side_effect_summary.update_world_states_last_world_state_id,
            update_world_states_last_world_state_value =
                side_effect_summary.update_world_states_last_world_state_value,
            update_npc_flags_actions = side_effect_summary.update_npc_flags_actions,
            update_npc_flags_records_seen = side_effect_summary.update_npc_flags_records_seen,
            update_npc_flags_maps_matched = side_effect_summary.update_npc_flags_maps_matched,
            update_npc_flags_live_creatures_mutated =
                side_effect_summary.update_npc_flags_live_creatures_mutated,
            update_npc_flags2_applied =
                side_effect_summary.update_npc_flags2_applied,
            update_npc_vendor_actions = side_effect_summary.update_npc_vendor_actions,
            update_npc_vendor_records_seen = side_effect_summary.update_npc_vendor_records_seen,
            update_npc_vendor_items_added = side_effect_summary.update_npc_vendor_items_added,
            update_npc_vendor_items_removed = side_effect_summary.update_npc_vendor_items_removed,
            update_npc_vendor_missing_event_buckets =
                side_effect_summary.update_npc_vendor_missing_event_buckets,
            update_npc_vendor_remove_misses = side_effect_summary.update_npc_vendor_remove_misses,
            update_npc_vendor_no_match = side_effect_summary.update_npc_vendor_no_match,
            reset_event_seasonal_quests_actions =
                side_effect_summary.reset_event_seasonal_quests_actions,
            reset_event_seasonal_quests_event_start_time_zero =
                side_effect_summary.reset_event_seasonal_quests_event_start_time_zero,
            reset_event_seasonal_quests_event_start_time_nonzero =
                side_effect_summary.reset_event_seasonal_quests_event_start_time_nonzero,
            reset_event_seasonal_quests_player_session_runtime_unimplemented = side_effect_summary
                .reset_event_seasonal_quests_player_session_runtime_unimplemented,
            reset_event_seasonal_quests_character_db_statement_unimplemented = side_effect_summary
                .reset_event_seasonal_quests_character_db_statement_unimplemented,
            reset_event_seasonal_quests_character_db_delete_queued = side_effect_summary
                .reset_event_seasonal_quests_character_db_delete_queued,
            reset_event_seasonal_quests_character_db_delete_executed = side_effect_summary
                .reset_event_seasonal_quests_character_db_delete_executed,
            reset_event_seasonal_quests_character_db_delete_failed = side_effect_summary
                .reset_event_seasonal_quests_character_db_delete_failed,
            reset_event_seasonal_quests_character_db_delete_skipped_event_start_time_out_of_range = side_effect_summary
                .reset_event_seasonal_quests_character_db_delete_skipped_event_start_time_out_of_range,
            "Represented C++ GameEventMgr::StartSystem: cleared active events, ran first Update with isSystemInit=false, installed WUPDATE_EVENTS delay, and consumed safe represented GameEventSpawn/GameEventUnspawn plus bounded ChangeEquipOrModel, UpdateEventQuests cache, represented UpdateWorldStates HolidayWorldState -> WorldStateMgr::SetValue evidence, UpdateEventNPCFlags, UpdateEventNPCVendor cache, RunSmartAIScripts evidence, ResetEventSeasonalQuests character DB delete bridge, and represented announcement evidence-only side effects; real SendWorldText/session fanout, full ConditionMgr world-event runtime, quest packets/session gossip refresh, full ObjectMgr quest runtime, real WorldStateMgr storage/session fanout/login/GM worldstate, SmartAI script dispatch, and Player/session seasonal quest reset remain pending"
        );
        CanonicalGameEventSchedulerLikeCpp::start_system(
            game_event_outcome.next_update_delay_millis,
        )
    };

    let (game_event_quest_complete_tx, game_event_quest_complete_rx) = flume::bounded(1024);
    let game_event_quest_complete_handle =
        tokio::spawn(run_game_event_quest_complete_processor_like_cpp(
            game_event_quest_complete_rx,
            Arc::clone(&canonical_spawn_metadata),
            Arc::clone(&char_db),
        ));

    // Build session resources
    let session_resources = Arc::new(SessionResources {
        char_db: Some(Arc::clone(&char_db)),
        login_db: Some(Arc::clone(&login_db)),
        world_db: Some(Arc::clone(&world_db)),
        guid_generator: Some(Arc::clone(&guid_generator)),
        instance_lock_mgr: Some(Arc::clone(&instance_lock_mgr)),
        currency_types_store: Some(Arc::clone(&currency_types_store)),
        import_price_stores: Some(Arc::clone(&import_price_stores)),
        item_class_store: Some(Arc::clone(&item_class_store)),
        item_currency_cost_store: Some(Arc::clone(&item_currency_cost_store)),
        item_extended_cost_store: Some(Arc::clone(&item_extended_cost_store)),
        item_store: Some(Arc::clone(&item_store)),
        item_appearance_store: Some(Arc::clone(&item_appearance_store)),
        item_modified_appearance_store: Some(Arc::clone(&item_modified_appearance_store)),
        item_price_base_store: Some(Arc::clone(&item_price_base_store)),
        item_limit_category_store: Some(Arc::clone(&item_limit_category_store)),
        item_limit_category_condition_store: Some(Arc::clone(&item_limit_category_condition_store)),
        player_stats: Some(Arc::clone(&player_stats)),
        item_stats_store: Some(Arc::clone(&item_stats_store)),
        durability_costs_store: Some(Arc::clone(&durability_costs_store)),
        durability_quality_store: Some(Arc::clone(&durability_quality_store)),
        item_random_suffix_store: Some(Arc::clone(&item_random_suffix_store)),
        item_random_properties_store: Some(Arc::clone(&item_random_properties_store)),
        rand_prop_points_store: Some(Arc::clone(&rand_prop_points_store)),
        item_random_enchantment_template_store: Some(Arc::clone(
            &item_random_enchantment_template_store,
        )),
        item_disenchant_loot_store: Some(Arc::clone(&item_disenchant_loot_store)),
        loot_stores: Some(Arc::clone(&loot_stores)),
        condition_store: Some(Arc::clone(&condition_store)),
        player_condition_store: Some(Arc::clone(&player_condition_store)),
        content_tuning_store: Some(Arc::clone(&content_tuning_store)),
        disable_mgr: Some(Arc::clone(&disable_mgr)),
        lock_store: Some(Arc::clone(&lock_store)),
        spell_item_enchantment_store: Some(Arc::clone(&spell_item_enchantment_store)),
        hotfix_blob_cache: Some(Arc::clone(&hotfix_blob_cache)),
        skill_store: Some(Arc::clone(&skill_store)),
        skill_line_store: Some(Arc::clone(&skill_line_store)),
        spell_store: Some(Arc::clone(&spell_store)),
        npc_spell_click_store: Some(Arc::clone(&npc_spell_click_store)),
        spell_misc_store: Some(Arc::clone(&spell_misc_store)),
        spell_duration_store: Some(Arc::clone(&spell_duration_store)),
        spell_radius_store: Some(Arc::clone(&spell_radius_store)),
        spell_range_store: Some(Arc::clone(&spell_range_store)),
        area_table_store: Some(Arc::clone(&area_table_store)),
        fishing_base_skill_store: Some(Arc::clone(&fishing_base_skill_store)),
        area_trigger_store: Some(Arc::clone(&area_trigger_store)),
        chr_specialization_store: Some(Arc::clone(&chr_specialization_store)),
        dungeon_encounter_store: Some(Arc::clone(&dungeon_encounter_store)),
        map_store: Some(Arc::clone(&map_store)),
        map_difficulty_store: Some(Arc::clone(&map_difficulty_store)),
        map_difficulty_x_condition_store: Some(Arc::clone(&map_difficulty_x_condition_store)),
        lfg_dungeons_store: Some(Arc::clone(&lfg_dungeons_store)),
        creature_template_mount_store: Some(Arc::clone(&creature_template_mount_store)),
        creature_display_info_store: Some(Arc::clone(&creature_display_info_store)),
        gameobject_display_info_store: Some(Arc::clone(&gameobject_display_info_store)),
        creature_model_data_store: Some(Arc::clone(&creature_model_data_store)),
        mount_store: Some(Arc::clone(&mount_store)),
        mount_capability_store: Some(Arc::clone(&mount_capability_store)),
        mount_type_x_capability_store: Some(Arc::clone(&mount_type_x_capability_store)),
        mount_x_display_store: Some(Arc::clone(&mount_x_display_store)),
        vehicle_store: Some(Arc::clone(&vehicle_store)),
        vehicle_seat_store: Some(Arc::clone(&vehicle_seat_store)),
        vehicle_template_store: Some(Arc::clone(&vehicle_template_store)),
        vehicle_accessory_store: Some(Arc::clone(&vehicle_accessory_store)),
        terrain_swap_store: Some(Arc::clone(&terrain_swap_store)),
        phase_store: Some(Arc::clone(&phase_store)),
        phase_group_store: Some(Arc::clone(&phase_group_store)),
        quest_store: Some(Arc::clone(&quest_store)),
        quest_xp_store: Some(Arc::clone(&quest_xp_store)),
        quest_v2_store: Some(Arc::clone(&quest_v2_store)),
        quest_info_store: Some(Arc::clone(&quest_info_store)),
        quest_package_item_store: Some(Arc::clone(&quest_package_item_store)),
        quest_faction_reward_store: Some(Arc::clone(&quest_faction_reward_store)),
        progression_faction_store: Some(Arc::clone(&progression_faction_store)),
        faction_template_store: Some(Arc::clone(&faction_template_store)),
        friendship_rep_reaction_store: Some(Arc::clone(&friendship_rep_reaction_store)),
        paragon_reputation_store: Some(Arc::clone(&paragon_reputation_store)),
        reputation_reward_rate_store: Some(Arc::clone(&reputation_reward_rate_store)),
        creature_onkill_reputation_store: Some(Arc::clone(&creature_onkill_reputation_store)),
        reputation_spillover_template_store: Some(Arc::clone(&reputation_spillover_template_store)),
        player_xp_table: Some(Arc::clone(&player_xp_table)),
        player_registry: Some(Arc::clone(&player_registry)),
        game_event_quest_complete_tx: Some(game_event_quest_complete_tx),
        group_registry: Some(Arc::clone(&group_registry)),
        pending_invites: Some(Arc::clone(&pending_invites)),
        loot_drop_rates: loot_drop_rates_like_cpp(&world_configs),
        reputation_rates: reputation_rates_like_cpp(&world_configs),
        repair_cost_rate: repair_cost_rate_like_cpp(&world_configs),
        quest_low_level_hide_diff: world_config_u32(
            &world_configs,
            "CONFIG_QUEST_LOW_LEVEL_HIDE_DIFF",
            4,
        ),
        quest_high_level_hide_diff: world_config_u32(
            &world_configs,
            "CONFIG_QUEST_HIGH_LEVEL_HIDE_DIFF",
            7,
        ),
        enable_ae_loot: world_config_bool(&world_configs, "CONFIG_ENABLE_AE_LOOT", false),
        realm_id,
        realm_external_address,
        realm_local_address,
    });

    // Create SessionManager for ConnectTo flow
    let session_mgr = Arc::new(SessionManager::new());

    // Network configuration
    let bind_ip = wow_config::get_string_default("BindIP", "0.0.0.0");
    let world_port = world_config_u16(&world_configs, "CONFIG_PORT_WORLD", 8085);
    let instance_port = world_config_u16(&world_configs, "CONFIG_PORT_INSTANCE", 8086);
    let max_expansion = world_config_u8(&world_configs, "CONFIG_EXPANSION", 2);
    let mmap_runtime_config = mmap_runtime_config_like_cpp(&world_configs, mmap_disabled_map_ids);
    info!(
        "WORLD: MMap pathfinding: {}, data directory: {}/mmaps",
        if mmap_runtime_config.enabled {
            "enabled"
        } else {
            "disabled"
        },
        mmap_runtime_config.data_dir
    );
    let mmap_pathfinder = mmap_runtime_config.enabled.then(|| {
        Arc::new(
            WorldMMapPathfinderWorkerLikeCpp::spawn_with_parent_map_data_like_cpp(
                &mmap_runtime_config.data_dir,
                map_store.parent_child_map_data_like_cpp(),
            ),
        )
    });

    let realm_addr: SocketAddr = format!("{bind_ip}:{world_port}")
        .parse()
        .context("Invalid bind address")?;
    let instance_addr: SocketAddr = format!("{bind_ip}:{instance_port}")
        .parse()
        .context("Invalid instance bind address")?;

    info!("Starting realm listener on {realm_addr}");
    info!("Starting instance listener on {instance_addr}");

    // Spawn realm listener (existing world listener)
    let realm_handle = tokio::spawn({
        let lookup = Arc::clone(&account_lookup);
        let resources = Arc::clone(&session_resources);
        let mgr = Arc::clone(&session_mgr);
        let smap = Arc::clone(&shared_map);
        let canonical_map = Arc::clone(&canonical_map_manager);
        let accessor = Arc::clone(&object_accessor);
        let port = instance_port;
        let mmap_config = mmap_runtime_config.clone();
        let mmap_pathfinder = mmap_pathfinder.clone();
        async move {
            if let Err(e) = wow_network::start_world_listener(
                realm_addr,
                lookup,
                resources,
                move |account, pkt_rx, send_tx, res| {
                    let mgr = Arc::clone(&mgr);
                    let smap = Arc::clone(&smap);
                    let canonical_map = Arc::clone(&canonical_map);
                    let accessor = Arc::clone(&accessor);
                    let mmap_pathfinder = mmap_pathfinder.clone();
                    create_session(
                        account,
                        pkt_rx,
                        send_tx,
                        res,
                        mgr,
                        smap,
                        canonical_map,
                        accessor,
                        port,
                        max_expansion,
                        mmap_config.clone(),
                        mmap_pathfinder,
                    )
                },
            )
            .await
            {
                tracing::error!("Realm listener error: {e}");
            }
        }
    });

    // Spawn instance listener
    let instance_handle = tokio::spawn({
        let mgr = Arc::clone(&session_mgr);
        async move {
            if let Err(e) = wow_network::start_instance_listener(instance_addr, mgr).await {
                tracing::error!("Instance listener error: {e}");
            }
        }
    });

    let map_update_interval_ms = world_config_u32(&world_configs, "CONFIG_INTERVAL_MAPUPDATE", 10)
        .max(wow_map::MIN_MAP_UPDATE_DELAY_MS);
    let legacy_creature_global_runtime_enabled =
        legacy_creature_global_runtime_enabled_from_config_like_cpp();
    if legacy_creature_global_runtime_enabled {
        warn!(
            "EXPERIMENTAL: RustyCore.LegacyCreatureGlobalRuntime enabled; legacy creature tick owner set to GlobalLegacy"
        );
        match shared_map.write() {
            Ok(mut manager) => {
                manager.set_tick_owner(wow_world::map_manager::RuntimeTickOwner::GlobalLegacy);
            }
            Err(_) => {
                warn!("Legacy MapManager lock poisoned; cannot enable GlobalLegacy tick owner")
            }
        }
    }
    let respawn_condition_interval_ms = world_config_u32(
        &world_configs,
        "CONFIG_RESPAWN_MINCHECKINTERVALMS",
        DEFAULT_RESPAWN_MIN_CHECK_INTERVAL_MS,
    )
    .max(1);
    let map_update_handle = spawn_canonical_map_update_loop(
        Arc::clone(&canonical_map_manager),
        map_update_interval_ms,
        respawn_condition_interval_ms,
        Arc::clone(&canonical_spawn_metadata),
        Arc::clone(&condition_store),
        Arc::clone(&char_db),
        loaded_grid_creature_respawn_caches.clone(),
        game_event_scheduler,
        Arc::clone(&player_registry),
        Arc::clone(&battlemaster_list_typed_store),
        Arc::clone(&world_state_mgr),
    );
    let mut legacy_creature_aggro_config = legacy_creature_aggro_config_like_cpp(&world_configs);
    legacy_creature_aggro_config.faction_template_store = Some(Arc::clone(&faction_template_store));
    legacy_creature_aggro_config.faction_store = Some(Arc::clone(&progression_faction_store));
    legacy_creature_aggro_config.map_store = Some(Arc::clone(&map_store));
    let legacy_creature_runtime_handle = spawn_legacy_creature_runtime_update_loop_like_cpp(
        legacy_creature_global_runtime_enabled,
        Arc::clone(&shared_map),
        Arc::clone(&canonical_map_manager),
        mmap_runtime_config.clone(),
        mmap_pathfinder.clone(),
        legacy_creature_aggro_config,
        map_update_interval_ms,
        Arc::clone(&player_registry),
    );

    set_realm_online(&login_db, realm_id).await?;

    // Wait for shutdown signal
    tokio::select! {
        _ = shutdown_signal() => {
            info!("Shutdown signal received, stopping...");
        }
        result = realm_handle => {
            if let Err(e) = result {
                tracing::error!("Realm listener task failed: {e}");
            }
        }
        result = instance_handle => {
            if let Err(e) = result {
                tracing::error!("Instance listener task failed: {e}");
            }
        }
        result = map_update_handle => {
            if let Err(e) = result {
                tracing::error!("Map update task failed: {e}");
            }
        }
        result = legacy_creature_runtime_handle => {
            if let Err(e) = result {
                tracing::error!("Legacy creature runtime task failed: {e}");
            }
        }
    }

    game_event_quest_complete_handle.abort();

    if let Err(e) = set_realm_offline(&login_db, realm_id).await {
        tracing::error!("Failed to mark realm {realm_id} offline: {e}");
    }

    info!("World server stopped.");
    Ok(())
}

async fn set_realm_online(login_db: &LoginDatabase, realm_id: u16) -> Result<()> {
    const REALM_FLAG_OFFLINE: u8 = 0x02;

    login_db
        .direct_execute(&format!(
            "UPDATE realmlist SET flag = flag & ~{REALM_FLAG_OFFLINE}, population = 0 WHERE id = {realm_id}"
        ))
        .await
        .context("Failed to mark realm online")?;

    info!("Realm {realm_id} marked online");
    Ok(())
}

async fn set_realm_offline(login_db: &LoginDatabase, realm_id: u16) -> Result<()> {
    const REALM_FLAG_OFFLINE: u8 = 0x02;

    login_db
        .direct_execute(&format!(
            "UPDATE realmlist SET flag = flag | {REALM_FLAG_OFFLINE} WHERE id = {realm_id}"
        ))
        .await
        .context("Failed to mark realm offline")?;

    info!("Realm {realm_id} marked offline");
    Ok(())
}

#[cfg(unix)]
async fn shutdown_signal() {
    use tokio::signal::unix::{SignalKind, signal};

    let mut terminate = signal(SignalKind::terminate()).expect("failed to install SIGTERM handler");

    tokio::select! {
        _ = tokio::signal::ctrl_c() => {}
        _ = terminate.recv() => {}
    }
}

#[cfg(not(unix))]
async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
}

fn load_world_config() -> Result<LoadReport> {
    load_world_config_from(WORLD_CONFIG_CANDIDATES, WORLD_CONFIG_DIR)
}

fn load_world_config_from(config_candidates: &[&str], config_dir: &str) -> Result<LoadReport> {
    let loaded_config = wow_config::load_config_with_fallbacks(config_candidates, config_dir)
        .context("Failed to load worldserver.conf")?;

    if loaded_config.candidate_index > 1 {
        tracing::warn!(
            config = %loaded_config.initial_file,
            "Using legacy Rust config filename; prefer worldserver.conf"
        );
    }

    Ok(loaded_config)
}

fn legacy_creature_global_runtime_enabled_from_config_like_cpp() -> bool {
    wow_config::get_value_default::<u8>(RUSTYCORE_LEGACY_CREATURE_GLOBAL_RUNTIME_CONFIG, 0) != 0
}

fn world_config_u16(configs: &WorldConfigSet, enum_name: &str, default: u16) -> u16 {
    configs
        .get_int(enum_name)
        .map(|value| value as u16)
        .unwrap_or(default)
}

fn world_config_u8(configs: &WorldConfigSet, enum_name: &str, default: u8) -> u8 {
    configs
        .get_int(enum_name)
        .map(|value| value as u8)
        .unwrap_or(default)
}

fn world_config_u32(configs: &WorldConfigSet, enum_name: &str, default: u32) -> u32 {
    configs
        .get_int(enum_name)
        .and_then(|value| u32::try_from(value).ok())
        .unwrap_or(default)
}

fn world_config_f32(configs: &WorldConfigSet, enum_name: &str, default: f32) -> f32 {
    configs.get_float(enum_name).unwrap_or(default)
}

fn world_config_bool(configs: &WorldConfigSet, enum_name: &str, default: bool) -> bool {
    configs.get_bool(enum_name).unwrap_or(default)
}

fn max_skill_value_like_cpp(configs: &WorldConfigSet) -> u32 {
    let max_player_level = u32::from(world_config_u8(configs, "CONFIG_MAX_PLAYER_LEVEL", 80));
    if max_player_level > 60 {
        300 + ((max_player_level - 60) * 75) / 10
    } else {
        max_player_level * 5
    }
}

fn mmap_runtime_config_like_cpp(
    configs: &WorldConfigSet,
    disabled_map_ids: HashSet<u32>,
) -> MMapRuntimeConfigLikeCpp {
    MMapRuntimeConfigLikeCpp {
        data_dir: wow_config::get_string_default("DataDir", "./Data"),
        enabled: world_config_bool(configs, "CONFIG_ENABLE_MMAPS", true),
        disabled_map_ids,
    }
}

async fn load_disable_mgr_like_cpp(
    world_db: &WorldDatabase,
    map_store: &wow_data::MapStore,
    map_difficulty_store: &wow_data::MapDifficultyStore,
    spell_store: &wow_data::SpellStore,
    quest_store: &wow_data::quest::QuestStore,
    criteria_store: &wow_data::Db2IdStore,
    battlemaster_list_store: &wow_data::Db2IdStore,
) -> Result<wow_data::DisableMgrLikeCpp> {
    let (disable_mgr, _) = wow_data::DisableMgrLikeCpp::load_like_cpp(
        world_db,
        wow_data::DisableMgrRefsLikeCpp {
            map_store: Some(map_store),
            map_difficulty_store: Some(map_difficulty_store),
            spell_store: Some(spell_store),
            quest_store: Some(quest_store),
            criteria_store: Some(criteria_store),
            battlemaster_list_store: Some(battlemaster_list_store),
            ..Default::default()
        },
    )
    .await
    .context("Failed to query C++ disables")?;

    Ok(disable_mgr)
}

fn loot_drop_rates_like_cpp(configs: &WorldConfigSet) -> LootDropRatesLikeCpp {
    LootDropRatesLikeCpp {
        item_poor: world_config_f32(configs, "RATE_DROP_ITEM_POOR", 1.0),
        item_normal: world_config_f32(configs, "RATE_DROP_ITEM_NORMAL", 1.0),
        item_uncommon: world_config_f32(configs, "RATE_DROP_ITEM_UNCOMMON", 1.0),
        item_rare: world_config_f32(configs, "RATE_DROP_ITEM_RARE", 1.0),
        item_epic: world_config_f32(configs, "RATE_DROP_ITEM_EPIC", 1.0),
        item_legendary: world_config_f32(configs, "RATE_DROP_ITEM_LEGENDARY", 1.0),
        item_artifact: world_config_f32(configs, "RATE_DROP_ITEM_ARTIFACT", 1.0),
        item_referenced: world_config_f32(configs, "RATE_DROP_ITEM_REFERENCED", 1.0),
        item_referenced_amount: world_config_f32(configs, "RATE_DROP_ITEM_REFERENCED_AMOUNT", 1.0),
        money: world_config_f32(configs, "RATE_DROP_MONEY", 1.0),
    }
}

fn reputation_rates_like_cpp(configs: &WorldConfigSet) -> ReputationRatesLikeCpp {
    ReputationRatesLikeCpp {
        gain: world_config_f32(configs, "RATE_REPUTATION_GAIN", 1.0),
        low_level_kill: world_config_f32(configs, "RATE_REPUTATION_LOWLEVEL_KILL", 1.0),
        low_level_quest: world_config_f32(configs, "RATE_REPUTATION_LOWLEVEL_QUEST", 1.0),
        recruit_a_friend_bonus: world_config_f32(
            configs,
            "RATE_REPUTATION_RECRUIT_A_FRIEND_BONUS",
            0.1,
        ),
        recruit_a_friend_distance: world_config_f32(
            configs,
            "CONFIG_MAX_RECRUIT_A_FRIEND_DISTANCE",
            100.0,
        ),
    }
}

fn repair_cost_rate_like_cpp(configs: &WorldConfigSet) -> f32 {
    world_config_f32(configs, "RATE_REPAIRCOST", 1.0).max(0.0)
}

async fn load_loot_stores_like_cpp(
    world_db: &WorldDatabase,
    item_store: &wow_data::ItemStore,
) -> Result<LootStores> {
    let mut stores = LootStores::new();

    for kind in LootStoreKind::ALL_LIKE_CPP {
        let rows = load_loot_template_rows_like_cpp(world_db, kind).await?;
        let mut store = LootStore::for_kind_like_cpp(kind);
        let accepted = store
            .load_rows_like_cpp(rows, |item_id| item_store.get(item_id).is_some())
            .map_err(|err| anyhow::anyhow!("invalid loot row in {:?}: {:?}", kind, err))?;
        info!(
            table = store.definition().table_name,
            entry_name = store.definition().entry_name,
            rates_allowed = store.definition().rates_allowed,
            accepted_rows = accepted,
            template_ids = store.templates().len(),
            "Loaded C++ loot template store foundation"
        );
        stores.insert(kind, store);
    }

    Ok(stores)
}

fn log_loot_reference_report_like_cpp(report: &LootReferenceCheckReport) {
    if report.is_clean() {
        info!("C++ loot reference verification completed with no gaps");
        return;
    }

    for reference_use in &report.missing_references {
        let store_definition = reference_use.store_kind.definition_like_cpp();
        tracing::warn!(
            table = store_definition.table_name,
            entry = reference_use.entry,
            item_id = reference_use.item_id,
            reference = reference_use.reference,
            "C++ loot reference verification found missing reference_loot_template entry"
        );
    }

    for reference_id in &report.unused_reference_ids {
        tracing::warn!(
            table = LootStoreKind::Reference.definition_like_cpp().table_name,
            entry = *reference_id,
            "C++ loot reference verification found unused reference_loot_template entry"
        );
    }
}

fn log_loot_condition_link_report_like_cpp(report: &LootConditionLinkReport) {
    if report.is_clean() {
        info!(
            linked_conditions = report.linked,
            "C++ loot condition structural linking completed with no gaps"
        );
        return;
    }

    for condition_id in &report.unsupported_source_types {
        tracing::warn!(
            source_type = condition_id.source_type,
            source_group = condition_id.source_group,
            source_entry = condition_id.source_entry,
            "C++ loot condition structural linking found unsupported loot condition source type"
        );
    }

    for missing in &report.missing_templates {
        let store_definition = missing.store_kind.definition_like_cpp();
        tracing::warn!(
            table = store_definition.table_name,
            source_type = missing.condition_id.source_type,
            source_group = missing.condition_id.source_group,
            source_entry = missing.condition_id.source_entry,
            "C++ loot condition structural linking found missing loot template"
        );
    }

    for missing in &report.missing_item_templates {
        let store_definition = missing.store_kind.definition_like_cpp();
        tracing::warn!(
            table = store_definition.table_name,
            source_type = missing.condition_id.source_type,
            source_group = missing.condition_id.source_group,
            source_entry = missing.condition_id.source_entry,
            "C++ loot condition structural linking found missing item template for SourceEntry"
        );
    }

    for missing in &report.missing_template_items {
        let store_definition = missing.store_kind.definition_like_cpp();
        tracing::warn!(
            table = store_definition.table_name,
            source_type = missing.condition_id.source_type,
            source_group = missing.condition_id.source_group,
            source_entry = missing.condition_id.source_entry,
            "C++ loot condition structural linking found SourceEntry absent from loot template"
        );
    }

    for missing in &report.missing_reference_templates {
        tracing::warn!(
            source_type = missing.condition_id.source_type,
            source_group = missing.condition_id.source_group,
            source_entry = missing.condition_id.source_entry,
            reference_id = missing.reference_id,
            "C++ loot condition structural linking found missing condition reference template"
        );
    }
}

async fn load_loot_condition_ids_like_cpp(
    world_db: &WorldDatabase,
) -> Result<Vec<LootConditionId>> {
    let stmt = world_db.prepare(WorldStatements::SEL_LOOT_TEMPLATE_CONDITION_IDS);
    let mut result = world_db.query(&stmt).await?;
    let mut condition_ids = Vec::new();

    if result.is_empty() {
        return Ok(condition_ids);
    }

    loop {
        condition_ids.push(LootConditionId {
            source_type: result.try_read::<i32>(0).unwrap_or(0),
            source_group: result.try_read::<u32>(1).unwrap_or(0),
            source_entry: result.try_read::<u32>(2).unwrap_or(0),
        });

        if !result.next_row() {
            break;
        }
    }

    Ok(condition_ids)
}

async fn load_loot_condition_reference_uses_like_cpp(
    world_db: &WorldDatabase,
) -> Result<Vec<LootConditionReferenceUseLikeCpp>> {
    let stmt = world_db.prepare(WorldStatements::SEL_LOOT_TEMPLATE_CONDITION_REFERENCE_USES);
    let mut result = world_db.query(&stmt).await?;
    let mut reference_uses = Vec::new();

    if result.is_empty() {
        return Ok(reference_uses);
    }

    loop {
        reference_uses.push(LootConditionReferenceUseLikeCpp {
            condition_id: LootConditionId {
                source_type: result.try_read::<i32>(0).unwrap_or(0),
                source_group: result.try_read::<u32>(1).unwrap_or(0),
                source_entry: result.try_read::<u32>(2).unwrap_or(0),
            },
            reference_id: result.try_read::<u32>(3).unwrap_or(0),
        });

        if !result.next_row() {
            break;
        }
    }

    Ok(reference_uses)
}

async fn load_condition_reference_template_ids_like_cpp(
    world_db: &WorldDatabase,
) -> Result<Vec<u32>> {
    let stmt = world_db.prepare(WorldStatements::SEL_CONDITION_REFERENCE_TEMPLATE_IDS);
    let mut result = world_db.query(&stmt).await?;
    let mut template_ids = Vec::new();

    if result.is_empty() {
        return Ok(template_ids);
    }

    loop {
        template_ids.push(result.try_read::<u32>(0).unwrap_or(0));

        if !result.next_row() {
            break;
        }
    }

    Ok(template_ids)
}

async fn load_loot_template_rows_like_cpp(
    world_db: &WorldDatabase,
    kind: LootStoreKind,
) -> Result<Vec<LootTemplateRow>> {
    let statement = loot_store_all_rows_statement_like_cpp(kind);
    let stmt = world_db.prepare(statement);
    let mut result = world_db.query(&stmt).await?;
    let mut rows = Vec::new();

    if result.is_empty() {
        return Ok(rows);
    }

    loop {
        rows.push(LootTemplateRow {
            entry: result.try_read::<u32>(0).unwrap_or(0),
            item: wow_loot::LootStoreItem {
                item_id: result.try_read::<u32>(1).unwrap_or(0),
                reference: result.try_read::<u32>(2).unwrap_or(0),
                chance: result.try_read::<f32>(3).unwrap_or(0.0),
                needs_quest: result.try_read::<u8>(4).unwrap_or(0) != 0,
                loot_mode: result.try_read::<u16>(5).unwrap_or(0),
                group_id: result.try_read::<u8>(6).unwrap_or(0),
                min_count: result.try_read::<u8>(7).unwrap_or(0),
                max_count: result.try_read::<u8>(8).unwrap_or(0),
            },
        });

        if !result.next_row() {
            break;
        }
    }

    Ok(rows)
}

fn loot_store_all_rows_statement_like_cpp(kind: LootStoreKind) -> WorldStatements {
    match kind {
        LootStoreKind::Creature => WorldStatements::SEL_CREATURE_LOOT_TEMPLATE_ALL_ROWS,
        LootStoreKind::Disenchant => WorldStatements::SEL_DISENCHANT_LOOT_TEMPLATE_ALL_ROWS,
        LootStoreKind::Fishing => WorldStatements::SEL_FISHING_LOOT_TEMPLATE_ALL_ROWS,
        LootStoreKind::Gameobject => WorldStatements::SEL_GAMEOBJECT_LOOT_TEMPLATE_ALL_ROWS,
        LootStoreKind::Item => WorldStatements::SEL_ITEM_LOOT_TEMPLATE_ALL_ROWS,
        LootStoreKind::Mail => WorldStatements::SEL_MAIL_LOOT_TEMPLATE_ALL_ROWS,
        LootStoreKind::Milling => WorldStatements::SEL_MILLING_LOOT_TEMPLATE_ALL_ROWS,
        LootStoreKind::Pickpocketing => WorldStatements::SEL_PICKPOCKETING_LOOT_TEMPLATE_ALL_ROWS,
        LootStoreKind::Prospecting => WorldStatements::SEL_PROSPECTING_LOOT_TEMPLATE_ALL_ROWS,
        LootStoreKind::Reference => WorldStatements::SEL_REFERENCE_LOOT_TEMPLATE_ALL_ROWS,
        LootStoreKind::Skinning => WorldStatements::SEL_SKINNING_LOOT_TEMPLATE_ALL_ROWS,
        LootStoreKind::Spell => WorldStatements::SEL_SPELL_LOOT_TEMPLATE_ALL_ROWS,
    }
}

#[derive(Debug, Clone, Default)]
struct PersistedRespawnTimesLikeCpp {
    by_map: BTreeMap<wow_map::MapKey, Vec<wow_map::RespawnInfoLikeCpp>>,
}

impl PersistedRespawnTimesLikeCpp {
    fn push(&mut self, key: wow_map::MapKey, info: wow_map::RespawnInfoLikeCpp) {
        self.by_map.entry(key).or_default().push(info);
    }

    fn for_map(&self, key: wow_map::MapKey) -> &[wow_map::RespawnInfoLikeCpp] {
        self.by_map.get(&key).map_or(&[], Vec::as_slice)
    }

    fn maps_len(&self) -> usize {
        self.by_map.len()
    }

    fn respawns_len(&self) -> usize {
        self.by_map.values().map(Vec::len).sum()
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct PersistedRespawnLoadReportLikeCpp {
    rows: usize,
    loaded: usize,
    invalid_type: usize,
    unsupported_area_trigger: usize,
    missing_spawn_metadata: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PersistedRespawnRowLikeCpp {
    object_type_raw: u16,
    spawn_id: u64,
    respawn_time: i64,
    map_id: u32,
    instance_id: u32,
}

async fn load_persisted_respawn_times_like_cpp(
    character_db: &CharacterDatabase,
    canonical_spawn_metadata: &spawn_store_loader::CanonicalSpawnMetadataLikeCpp,
) -> Result<(
    PersistedRespawnTimesLikeCpp,
    PersistedRespawnLoadReportLikeCpp,
)> {
    let mut result = character_db
        .query(&character_db.prepare(CharStatements::SEL_ALL_RESPAWNS))
        .await?;
    let mut snapshot = PersistedRespawnTimesLikeCpp::default();
    let mut report = PersistedRespawnLoadReportLikeCpp::default();

    if result.is_empty() {
        return Ok((snapshot, report));
    }

    loop {
        let row = PersistedRespawnRowLikeCpp {
            object_type_raw: result
                .try_read::<u16>(0)
                .or_else(|| result.try_read::<u8>(0).map(u16::from))
                .unwrap_or(u16::MAX),
            spawn_id: result
                .try_read::<u64>(1)
                .or_else(|| result.try_read::<i64>(1).map(|value| value as u64))
                .unwrap_or(0),
            respawn_time: result.try_read::<i64>(2).unwrap_or(0),
            map_id: result
                .try_read::<u32>(3)
                .or_else(|| result.try_read::<u16>(3).map(u32::from))
                .unwrap_or(0),
            instance_id: result.try_read::<u32>(4).unwrap_or(0),
        };
        if let Some((key, info)) =
            persisted_respawn_info_from_row_like_cpp(row, canonical_spawn_metadata, &mut report)
        {
            snapshot.push(key, info);
        }
        if !result.next_row() {
            break;
        }
    }

    Ok((snapshot, report))
}

fn persisted_respawn_info_from_row_like_cpp(
    row: PersistedRespawnRowLikeCpp,
    canonical_spawn_metadata: &spawn_store_loader::CanonicalSpawnMetadataLikeCpp,
    report: &mut PersistedRespawnLoadReportLikeCpp,
) -> Option<(wow_map::MapKey, wow_map::RespawnInfoLikeCpp)> {
    report.rows += 1;
    let Ok(object_type_raw) = u8::try_from(row.object_type_raw) else {
        report.invalid_type += 1;
        return None;
    };
    let Some(object_type) = wow_map::SpawnObjectType::from_raw(object_type_raw) else {
        report.invalid_type += 1;
        return None;
    };
    if matches!(object_type, wow_map::SpawnObjectType::AreaTrigger) {
        report.unsupported_area_trigger += 1;
        return None;
    }

    let Some(spawn_data) = canonical_spawn_metadata
        .spawn_store()
        .spawn_data(object_type, row.spawn_id)
    else {
        report.missing_spawn_metadata += 1;
        return None;
    };

    report.loaded += 1;
    Some((
        wow_map::MapKey::new(row.map_id, row.instance_id),
        wow_map::RespawnInfoLikeCpp {
            object_type,
            spawn_id: row.spawn_id,
            entry: spawn_data.id,
            respawn_time: row.respawn_time,
            grid_id: wow_map::compute_grid_coord(
                spawn_data.spawn_point.x,
                spawn_data.spawn_point.y,
            )
            .get_id(),
        },
    ))
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct PersistedRespawnApplyReportLikeCpp {
    candidates: usize,
    inserted: usize,
    replaced_existing: usize,
    rejected_zero_spawn_id: usize,
    rejected_unsupported_type: usize,
    rejected_existing_sooner_or_equal: usize,
    skipped_non_world_map: usize,
}

fn apply_persisted_respawns_to_managed_map_like_cpp(
    managed_map: &mut wow_map::ManagedMap,
    persisted_respawn_times: &PersistedRespawnTimesLikeCpp,
) -> PersistedRespawnApplyReportLikeCpp {
    let key = wow_map::MapKey::new(managed_map.map_id(), managed_map.instance_id());
    let respawns = persisted_respawn_times.for_map(key);
    let mut report = PersistedRespawnApplyReportLikeCpp {
        candidates: respawns.len(),
        ..PersistedRespawnApplyReportLikeCpp::default()
    };

    if !matches!(managed_map.kind(), wow_map::ManagedMapKind::World) {
        report.skipped_non_world_map = respawns.len();
        return report;
    }

    for info in respawns {
        match managed_map
            .map_mut()
            .add_respawn_info_like_cpp(info.clone())
        {
            wow_map::AddRespawnInfoOutcomeLikeCpp::Inserted => report.inserted += 1,
            wow_map::AddRespawnInfoOutcomeLikeCpp::ReplacedExisting => {
                report.replaced_existing += 1
            }
            wow_map::AddRespawnInfoOutcomeLikeCpp::RejectedZeroSpawnId => {
                report.rejected_zero_spawn_id += 1;
            }
            wow_map::AddRespawnInfoOutcomeLikeCpp::RejectedUnsupportedType => {
                report.rejected_unsupported_type += 1;
            }
            wow_map::AddRespawnInfoOutcomeLikeCpp::RejectedExistingSoonerOrEqual => {
                report.rejected_existing_sooner_or_equal += 1;
            }
        }
    }

    report
}

fn install_canonical_spawn_group_initializer_like_cpp(
    manager: &mut wow_map::MapManager,
    canonical_spawn_metadata: SharedCanonicalSpawnMetadataLikeCpp,
    condition_store: Arc<wow_data::ConditionEntriesByTypeStore>,
    persisted_respawn_times: Arc<PersistedRespawnTimesLikeCpp>,
) {
    manager.set_spawn_group_initializer_like_cpp(move |managed_map| {
        let map_id = managed_map.map_id();
        let instance_id = managed_map.instance_id();
        let difficulty_id = u32::from(managed_map.map().spawn_mode());
        let Ok(canonical_spawn_metadata) = canonical_spawn_metadata.lock() else {
            warn!(
                map_id,
                instance_id,
                difficulty_id,
                "CanonicalSpawnMetadataLikeCpp mutex poisoned; skipping InitSpawnGroupState hook"
            );
            return;
        };

        let pool_init_report = managed_map.map_mut().init_pools_for_map_like_cpp(
            canonical_spawn_metadata.pool_mgr_like_cpp(),
            |_kind, _pool_id| 0.0,
            |_candidates, count| (0..count).collect(),
        );
        if pool_init_report.attempted() > 0 || pool_init_report.error_count() > 0 {
            debug!(
                map_id,
                instance_id,
                difficulty_id,
                attempted = pool_init_report.attempted(),
                planned = pool_init_report.planned(),
                errors = pool_init_report.error_count(),
                spawn_one_actions = pool_init_report.spawn_one_actions(),
                respawn_one_actions = pool_init_report.respawn_one_actions(),
                despawn_one_actions = pool_init_report.despawn_one_actions(),
                "Applied represented C++ PoolMgr::InitPoolsForMap autospawn plans to map-owned pool data before LoadRespawnTimes; live entity side effects remain report-only"
            );
        }
        for error in &pool_init_report.errors {
            warn!(
                map_id,
                instance_id,
                difficulty_id,
                pool_id = error.pool_id,
                error = ?error.error,
                "PoolMgr::InitPoolsForMap represented autospawn planning failed for pool; leaving entity side effects unexecuted"
            );
        }

        let respawn_report = apply_persisted_respawns_to_managed_map_like_cpp(
            managed_map,
            persisted_respawn_times.as_ref(),
        );
        if respawn_report.candidates > 0 {
            debug!(
                map_id,
                instance_id,
                difficulty_id,
                candidates = respawn_report.candidates,
                inserted = respawn_report.inserted,
                replaced_existing = respawn_report.replaced_existing,
                rejected_zero_spawn_id = respawn_report.rejected_zero_spawn_id,
                rejected_unsupported_type = respawn_report.rejected_unsupported_type,
                rejected_existing_sooner_or_equal = respawn_report.rejected_existing_sooner_or_equal,
                skipped_non_world_map = respawn_report.skipped_non_world_map,
                "Applied C++ startup LoadRespawnTimes snapshot to canonical map before InitSpawnGroupState"
            );
        }

        let groups = canonical_spawn_metadata.spawn_group_templates_for_map_like_cpp(map_id);
        if groups.is_empty() {
            debug!(
                map_id,
                instance_id,
                difficulty_id,
                "InitSpawnGroupState hook found no spawn groups for map"
            );
            return;
        }

        let group_templates = groups
            .iter()
            .map(|(_group_id, template)| *template)
            .collect::<Vec<_>>();
        let map_ref = ConditionMapRef::new(map_id, instance_id);
        let map_state = ConditionMapStateSnapshot {
            active_event_ids: &[],
            world_states: &[],
            difficulty_id,
            instance_data: &[],
            instance_data64: &[],
            boss_states: &[],
            scenario_step_id: None,
        };
        let changes =
            managed_map
                .map_mut()
                .init_spawn_group_state_like_cpp(group_templates, |group| {
                    is_spawn_group_meeting_map_conditions_like_cpp(
                        condition_store.as_ref(),
                        group.group_id,
                        map_ref,
                        Some(map_state),
                        &[],
                    )
                });
        let toggled = changes
            .iter()
            .filter(|(_group_id, change)| {
                matches!(change, wow_map::SpawnGroupActiveChange::Toggled)
            })
            .count();
        debug!(
            map_id,
            instance_id,
            difficulty_id,
            groups_evaluated = changes.len(),
            toggled,
            "Applied C++ InitSpawnGroupState hook to canonical map"
        );
    });
}

#[derive(Debug, Default, Clone, PartialEq)]
struct GameEventPoolUnspawnSummaryLikeCpp {
    event_pool_ids_seen: usize,
    missing_pool_templates: usize,
    invalid_template_map_ids: usize,
    pools_without_loaded_canonical_maps: usize,
    maps_matched: usize,
    pool_objects_removed: usize,
    pool_respawn_timers_removed: usize,
    pool_respawn_timers_missing: usize,
    pool_stale_index_entries: usize,
    pool_remove_errors: usize,
    pool_unsupported_action_kind: usize,
    blocked_pool_plan_errors: Vec<wow_map::PoolMgrPlanErrorLikeCpp>,
}

#[derive(Debug, Default, Clone, PartialEq)]
struct GameEventPoolEventUnspawnSummaryLikeCpp {
    event_id: i16,
    missing_event_pool_ids: bool,
    pool_summary: GameEventPoolUnspawnSummaryLikeCpp,
}

impl GameEventPoolUnspawnSummaryLikeCpp {
    fn accumulate_despawn_summary_like_cpp(
        &mut self,
        summary: &wow_map::map::ProcessRespawnsSafeSideEffectsSummaryLikeCpp,
    ) {
        self.pool_objects_removed += summary.pool_objects_removed;
        self.pool_respawn_timers_removed += summary.pool_respawn_timers_removed;
        self.pool_respawn_timers_missing += summary.pool_respawn_timers_missing;
        self.pool_stale_index_entries += summary.pool_stale_index_entries;
        self.pool_remove_errors += summary.pool_remove_errors;
        self.pool_unsupported_action_kind += summary.pool_unsupported_action_kind;
        self.blocked_pool_plan_errors
            .extend(summary.blocked_pool_plan_errors.iter().copied());
    }
}

fn game_event_unspawn_pools_like_cpp(
    manager: &mut wow_map::MapManager,
    canonical_spawn_metadata: &spawn_store_loader::CanonicalSpawnMetadataLikeCpp,
    event_pool_ids: &[u32],
) -> GameEventPoolUnspawnSummaryLikeCpp {
    let pool_mgr = canonical_spawn_metadata.pool_mgr_like_cpp();
    let mut summary = GameEventPoolUnspawnSummaryLikeCpp::default();

    for &pool_id in event_pool_ids {
        summary.event_pool_ids_seen += 1;
        let Some(pool_template) = pool_mgr.pool_template_like_cpp(pool_id) else {
            summary.missing_pool_templates += 1;
            continue;
        };
        let Ok(map_id) = u32::try_from(pool_template.map_id) else {
            summary.invalid_template_map_ids += 1;
            continue;
        };

        let mut maps_matched_for_pool = 0usize;
        manager.do_for_all_maps_mut(|managed_map| {
            if managed_map.map_id() != map_id {
                return;
            }
            maps_matched_for_pool += 1;
            match managed_map
                .map_mut()
                .despawn_pool_safe_map_actions_like_cpp(pool_mgr, pool_id, true)
            {
                Ok(map_summary) => summary.accumulate_despawn_summary_like_cpp(&map_summary),
                Err(error) => summary.blocked_pool_plan_errors.push(error),
            }
        });
        summary.maps_matched += maps_matched_for_pool;
        if maps_matched_for_pool == 0 {
            summary.pools_without_loaded_canonical_maps += 1;
        }
    }

    summary
}

fn game_event_unspawn_pools_for_event_like_cpp(
    manager: &mut wow_map::MapManager,
    canonical_spawn_metadata: &spawn_store_loader::CanonicalSpawnMetadataLikeCpp,
    event_id: i16,
) -> GameEventPoolEventUnspawnSummaryLikeCpp {
    let Some(event_pool_ids) = canonical_spawn_metadata.game_event_pool_ids_like_cpp(event_id)
    else {
        return GameEventPoolEventUnspawnSummaryLikeCpp {
            event_id,
            missing_event_pool_ids: true,
            pool_summary: GameEventPoolUnspawnSummaryLikeCpp::default(),
        };
    };

    GameEventPoolEventUnspawnSummaryLikeCpp {
        event_id,
        missing_event_pool_ids: false,
        pool_summary: game_event_unspawn_pools_like_cpp(
            manager,
            canonical_spawn_metadata,
            event_pool_ids,
        ),
    }
}
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
struct GameEventObjectUnspawnBucketSummaryLikeCpp {
    guids_seen: usize,
    skipped_active_in_other_event: usize,
    missing_spawn_metadata: usize,
    represented_object_mgr_grid_removals: usize,
    maps_matched: usize,
    without_loaded_canonical_maps: usize,
    respawn_timers_removed: usize,
    respawn_timers_missing: usize,
    live_objects_queued: usize,
    duplicate_queue_attempts: usize,
    stale_index_entries: usize,
    remove_errors: usize,
    unsupported_live_despawn_type: usize,
}

impl GameEventObjectUnspawnBucketSummaryLikeCpp {
    fn accumulate_despawn_outcome_like_cpp(
        &mut self,
        outcome: wow_map::map::DespawnAllBySpawnIdOutcomeLikeCpp,
    ) {
        self.live_objects_queued += outcome.queued;
        self.duplicate_queue_attempts += outcome.duplicates;
        self.stale_index_entries += outcome.stale_index_entries;
        self.remove_errors += outcome.remove_errors;
        self.unsupported_live_despawn_type += outcome.unsupported_live_despawn_type;
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
struct GameEventCreatureGameObjectUnspawnSummaryLikeCpp {
    event_id: i16,
    missing_event_creature_guids: bool,
    missing_event_gameobject_guids: bool,
    creature: GameEventObjectUnspawnBucketSummaryLikeCpp,
    gameobject: GameEventObjectUnspawnBucketSummaryLikeCpp,
}

#[derive(Debug, Default, Clone, PartialEq)]
struct GameEventUnspawnForEventSummaryLikeCpp {
    event_id: i16,
    non_pool: GameEventCreatureGameObjectUnspawnSummaryLikeCpp,
    pool_skipped_due_to_non_pool_bucket: bool,
    pool: GameEventPoolEventUnspawnSummaryLikeCpp,
}

fn game_event_guid_is_active_in_other_event_like_cpp(
    canonical_spawn_metadata: &spawn_store_loader::CanonicalSpawnMetadataLikeCpp,
    active_event_ids: &[u16],
    event_id: i16,
    object_type: wow_map::SpawnObjectType,
    spawn_id: wow_map::SpawnId,
) -> bool {
    if event_id <= 0 {
        return false;
    }

    active_event_ids.iter().copied().any(|active_event_id| {
        if active_event_id == event_id as u16 {
            return false;
        }
        let Ok(active_event_id) = i16::try_from(active_event_id) else {
            return false;
        };
        let active_guids = match object_type {
            wow_map::SpawnObjectType::Creature => {
                canonical_spawn_metadata.game_event_creature_guids_like_cpp(active_event_id)
            }
            wow_map::SpawnObjectType::GameObject => {
                canonical_spawn_metadata.game_event_gameobject_guids_like_cpp(active_event_id)
            }
            wow_map::SpawnObjectType::AreaTrigger => None,
        };
        active_guids.is_some_and(|guids| guids.contains(&spawn_id))
    })
}

fn game_event_unspawn_object_guid_list_for_event_like_cpp(
    manager: &mut wow_map::MapManager,
    canonical_spawn_metadata: &spawn_store_loader::CanonicalSpawnMetadataLikeCpp,
    active_event_ids: &[u16],
    event_id: i16,
    object_type: wow_map::SpawnObjectType,
    spawn_ids: &[wow_map::SpawnId],
) -> GameEventObjectUnspawnBucketSummaryLikeCpp {
    let mut summary = GameEventObjectUnspawnBucketSummaryLikeCpp::default();

    for &spawn_id in spawn_ids {
        summary.guids_seen += 1;
        if game_event_guid_is_active_in_other_event_like_cpp(
            canonical_spawn_metadata,
            active_event_ids,
            event_id,
            object_type,
            spawn_id,
        ) {
            summary.skipped_active_in_other_event += 1;
            continue;
        }

        let Some(spawn_data) = canonical_spawn_metadata
            .spawn_store()
            .spawn_data(object_type, spawn_id)
        else {
            summary.missing_spawn_metadata += 1;
            continue;
        };

        // C++ anchor: GameEventMgr.cpp:1246-1327 removes ObjectMgr grid metadata
        // before walking loaded maps. RustyCore has no safe ObjectMgr mutation here,
        // so this is represented as a count only and SpawnStore remains immutable.
        summary.represented_object_mgr_grid_removals += 1;

        let mut maps_matched_for_spawn = 0usize;
        manager.do_for_all_maps_mut(|managed_map| {
            if managed_map.map_id() != spawn_data.map_id {
                return;
            }
            maps_matched_for_spawn += 1;
            let map = managed_map.map_mut();
            if map
                .remove_respawn_time_like_cpp(object_type, spawn_id)
                .is_some()
            {
                summary.respawn_timers_removed += 1;
            } else {
                summary.respawn_timers_missing += 1;
            }
            let despawn = map.despawn_all_by_spawn_id_like_cpp(object_type, spawn_id);
            summary.accumulate_despawn_outcome_like_cpp(despawn);
        });

        summary.maps_matched += maps_matched_for_spawn;
        if maps_matched_for_spawn == 0 {
            summary.without_loaded_canonical_maps += 1;
        }
    }

    summary
}

fn game_event_unspawn_creatures_and_gameobjects_for_event_like_cpp(
    manager: &mut wow_map::MapManager,
    canonical_spawn_metadata: &spawn_store_loader::CanonicalSpawnMetadataLikeCpp,
    active_event_ids: &[u16],
    event_id: i16,
) -> GameEventCreatureGameObjectUnspawnSummaryLikeCpp {
    let Some(creature_guids) =
        canonical_spawn_metadata.game_event_creature_guids_like_cpp(event_id)
    else {
        return GameEventCreatureGameObjectUnspawnSummaryLikeCpp {
            event_id,
            missing_event_creature_guids: true,
            missing_event_gameobject_guids: false,
            creature: GameEventObjectUnspawnBucketSummaryLikeCpp::default(),
            gameobject: GameEventObjectUnspawnBucketSummaryLikeCpp::default(),
        };
    };

    let creature = game_event_unspawn_object_guid_list_for_event_like_cpp(
        manager,
        canonical_spawn_metadata,
        active_event_ids,
        event_id,
        wow_map::SpawnObjectType::Creature,
        creature_guids,
    );

    let Some(gameobject_guids) =
        canonical_spawn_metadata.game_event_gameobject_guids_like_cpp(event_id)
    else {
        return GameEventCreatureGameObjectUnspawnSummaryLikeCpp {
            event_id,
            missing_event_creature_guids: false,
            missing_event_gameobject_guids: true,
            creature,
            gameobject: GameEventObjectUnspawnBucketSummaryLikeCpp::default(),
        };
    };

    let gameobject = game_event_unspawn_object_guid_list_for_event_like_cpp(
        manager,
        canonical_spawn_metadata,
        active_event_ids,
        event_id,
        wow_map::SpawnObjectType::GameObject,
        gameobject_guids,
    );

    GameEventCreatureGameObjectUnspawnSummaryLikeCpp {
        event_id,
        missing_event_creature_guids: false,
        missing_event_gameobject_guids: false,
        creature,
        gameobject,
    }
}

fn game_event_unspawn_for_event_like_cpp(
    manager: &mut wow_map::MapManager,
    canonical_spawn_metadata: &spawn_store_loader::CanonicalSpawnMetadataLikeCpp,
    active_event_ids: &[u16],
    event_id: i16,
) -> GameEventUnspawnForEventSummaryLikeCpp {
    let non_pool = game_event_unspawn_creatures_and_gameobjects_for_event_like_cpp(
        manager,
        canonical_spawn_metadata,
        active_event_ids,
        event_id,
    );
    let pool_skipped_due_to_non_pool_bucket =
        non_pool.missing_event_creature_guids || non_pool.missing_event_gameobject_guids;
    let pool = if pool_skipped_due_to_non_pool_bucket {
        GameEventPoolEventUnspawnSummaryLikeCpp {
            event_id,
            missing_event_pool_ids: false,
            pool_summary: GameEventPoolUnspawnSummaryLikeCpp::default(),
        }
    } else {
        game_event_unspawn_pools_for_event_like_cpp(manager, canonical_spawn_metadata, event_id)
    };

    GameEventUnspawnForEventSummaryLikeCpp {
        event_id,
        non_pool,
        pool_skipped_due_to_non_pool_bucket,
        pool,
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
struct GameEventObjectSpawnBucketSummaryLikeCpp {
    guids_seen: usize,
    missing_spawn_metadata: usize,
    represented_object_mgr_grid_additions: usize,
    maps_matched: usize,
    without_loaded_canonical_maps: usize,
    respawn_timers_removed: usize,
    respawn_timers_missing: usize,
    unloaded_grid_skips: usize,
    load_attempts: usize,
    loader_blocked_or_missing: usize,
    successful_loaded_grid_spawns: usize,
    add_to_map_failures: usize,
    gameobject_not_spawned_by_default_skips: usize,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
struct GameEventCreatureGameObjectSpawnSummaryLikeCpp {
    event_id: i16,
    missing_event_creature_guids: bool,
    missing_event_gameobject_guids: bool,
    creature: GameEventObjectSpawnBucketSummaryLikeCpp,
    gameobject: GameEventObjectSpawnBucketSummaryLikeCpp,
}

#[derive(Debug, Default, Clone, PartialEq)]
struct GameEventSpawnForEventSummaryLikeCpp {
    event_id: i16,
    non_pool: GameEventCreatureGameObjectSpawnSummaryLikeCpp,
    pool_skipped_due_to_non_pool_bucket: bool,
    pool: GameEventPoolEventSpawnSummaryLikeCpp,
}

fn game_event_spawn_object_guid_list_for_event_like_cpp(
    manager: &mut wow_map::MapManager,
    canonical_spawn_metadata: &spawn_store_loader::CanonicalSpawnMetadataLikeCpp,
    loaded_grid_creature_respawn_caches: &LoadedGridCreatureRespawnCachesLikeCpp,
    object_type: wow_map::SpawnObjectType,
    spawn_ids: &[wow_map::SpawnId],
) -> GameEventObjectSpawnBucketSummaryLikeCpp {
    let mut summary = GameEventObjectSpawnBucketSummaryLikeCpp::default();

    for &spawn_id in spawn_ids {
        summary.guids_seen += 1;
        let Some(spawn_data) = canonical_spawn_metadata
            .spawn_store()
            .spawn_data(object_type, spawn_id)
        else {
            summary.missing_spawn_metadata += 1;
            continue;
        };

        // C++ anchor: GameEventMgr.cpp:1176-1180 and 1201-1204 add ObjectMgr
        // grid metadata before walking already-loaded maps. RustyCore has no
        // safe ObjectMgr grid-cell mutation in this world-server bridge, so the
        // immutable canonical SpawnStore evidence is represented by this count.
        summary.represented_object_mgr_grid_additions += 1;

        let mut maps_matched_for_spawn = 0usize;
        manager.do_for_all_maps_mut(|managed_map| {
            if managed_map.map_id() != spawn_data.map_id {
                return;
            }
            maps_matched_for_spawn += 1;
            let map = managed_map.map_mut();
            if map
                .remove_respawn_time_like_cpp(object_type, spawn_id)
                .is_some()
            {
                summary.respawn_timers_removed += 1;
            } else {
                summary.respawn_timers_missing += 1;
            }

            let cell = wow_map::cell_from_world(spawn_data.spawn_point.x, spawn_data.spawn_point.y);
            let grid = wow_map::GridCoord::new(cell.grid_x(), cell.grid_y());
            if !map.is_grid_loaded(grid) {
                summary.unloaded_grid_skips += 1;
                return;
            }

            summary.load_attempts += 1;
            let Some(records) = (match object_type {
                wow_map::SpawnObjectType::Creature => {
                    build_loaded_grid_creature_spawn_group_spawn_record_like_cpp(
                        map,
                        object_type,
                        spawn_id,
                        canonical_spawn_metadata,
                        loaded_grid_creature_respawn_caches,
                    )
                }
                wow_map::SpawnObjectType::GameObject => {
                    build_loaded_grid_gameobject_respawn_record_like_cpp(
                        map,
                        object_type,
                        spawn_id,
                        canonical_spawn_metadata,
                        loaded_grid_creature_respawn_caches,
                    )
                }
                wow_map::SpawnObjectType::AreaTrigger => None,
            }) else {
                summary.loader_blocked_or_missing += 1;
                return;
            };

            if object_type == wow_map::SpawnObjectType::GameObject
                && !records
                    .primary_record
                    .game_object()
                    .is_some_and(wow_entities::GameObject::spawned_by_default)
            {
                summary.gameobject_not_spawned_by_default_skips += 1;
                return;
            }

            for pre_add_record in records.pre_add_records {
                let _ = map.add_map_object_record_to_map_like_cpp(pre_add_record);
            }
            match map.add_map_object_record_to_map_like_cpp(records.primary_record) {
                Ok(_outcome) => {
                    summary.successful_loaded_grid_spawns += 1;
                }
                Err(_error) => {
                    summary.add_to_map_failures += 1;
                }
            }
        });
        summary.maps_matched += maps_matched_for_spawn;
        if maps_matched_for_spawn == 0 {
            summary.without_loaded_canonical_maps += 1;
        }
    }

    summary
}

fn game_event_spawn_creatures_and_gameobjects_for_event_like_cpp(
    manager: &mut wow_map::MapManager,
    canonical_spawn_metadata: &spawn_store_loader::CanonicalSpawnMetadataLikeCpp,
    loaded_grid_creature_respawn_caches: &LoadedGridCreatureRespawnCachesLikeCpp,
    event_id: i16,
) -> GameEventCreatureGameObjectSpawnSummaryLikeCpp {
    let Some(creature_guids) =
        canonical_spawn_metadata.game_event_creature_guids_like_cpp(event_id)
    else {
        return GameEventCreatureGameObjectSpawnSummaryLikeCpp {
            event_id,
            missing_event_creature_guids: true,
            missing_event_gameobject_guids: false,
            creature: GameEventObjectSpawnBucketSummaryLikeCpp::default(),
            gameobject: GameEventObjectSpawnBucketSummaryLikeCpp::default(),
        };
    };

    let creature = game_event_spawn_object_guid_list_for_event_like_cpp(
        manager,
        canonical_spawn_metadata,
        loaded_grid_creature_respawn_caches,
        wow_map::SpawnObjectType::Creature,
        creature_guids,
    );

    let Some(gameobject_guids) =
        canonical_spawn_metadata.game_event_gameobject_guids_like_cpp(event_id)
    else {
        return GameEventCreatureGameObjectSpawnSummaryLikeCpp {
            event_id,
            missing_event_creature_guids: false,
            missing_event_gameobject_guids: true,
            creature,
            gameobject: GameEventObjectSpawnBucketSummaryLikeCpp::default(),
        };
    };

    let gameobject = game_event_spawn_object_guid_list_for_event_like_cpp(
        manager,
        canonical_spawn_metadata,
        loaded_grid_creature_respawn_caches,
        wow_map::SpawnObjectType::GameObject,
        gameobject_guids,
    );

    GameEventCreatureGameObjectSpawnSummaryLikeCpp {
        event_id,
        missing_event_creature_guids: false,
        missing_event_gameobject_guids: false,
        creature,
        gameobject,
    }
}

#[derive(Debug, Default, Clone, PartialEq)]
struct GameEventPoolSpawnSummaryLikeCpp {
    event_pool_ids_seen: usize,
    missing_pool_templates: usize,
    invalid_template_map_ids: usize,
    pools_without_loaded_canonical_maps: usize,
    maps_matched: usize,
    executed_loaded_grid_respawns: usize,
    blocked_loaded_grid_respawn_add_to_map: usize,
    pool_spawn_actions_skipped_unloaded_grid: usize,
    pool_spawn_actions_blocked_loaded_grid: usize,
    pool_spawn_action_load_plans: usize,
    pool_spawn_actions_missing_spawn_data: usize,
    pool_objects_removed: usize,
    pool_respawn_timers_removed: usize,
    pool_respawn_timers_missing: usize,
    pool_stale_index_entries: usize,
    pool_remove_errors: usize,
    pool_unsupported_action_kind: usize,
    blocked_pool_plan_errors: Vec<wow_map::PoolMgrPlanErrorLikeCpp>,
}

#[derive(Debug, Default, Clone, PartialEq)]
struct GameEventPoolEventSpawnSummaryLikeCpp {
    event_id: i16,
    missing_event_pool_ids: bool,
    pool_summary: GameEventPoolSpawnSummaryLikeCpp,
}

impl GameEventPoolSpawnSummaryLikeCpp {
    fn accumulate_spawn_summary_like_cpp(
        &mut self,
        summary: &wow_map::map::ProcessRespawnsSafeSideEffectsSummaryLikeCpp,
    ) {
        self.executed_loaded_grid_respawns += summary.executed_loaded_grid_respawns;
        self.blocked_loaded_grid_respawn_add_to_map +=
            summary.blocked_loaded_grid_respawn_add_to_map;
        self.pool_spawn_actions_skipped_unloaded_grid +=
            summary.pool_spawn_actions_skipped_unloaded_grid;
        self.pool_spawn_actions_blocked_loaded_grid +=
            summary.pool_spawn_actions_blocked_loaded_grid;
        self.pool_spawn_action_load_plans += summary.pool_spawn_action_load_plans.len();
        self.pool_spawn_actions_missing_spawn_data += summary.pool_spawn_actions_missing_spawn_data;
        self.pool_objects_removed += summary.pool_objects_removed;
        self.pool_respawn_timers_removed += summary.pool_respawn_timers_removed;
        self.pool_respawn_timers_missing += summary.pool_respawn_timers_missing;
        self.pool_stale_index_entries += summary.pool_stale_index_entries;
        self.pool_remove_errors += summary.pool_remove_errors;
        self.pool_unsupported_action_kind += summary.pool_unsupported_action_kind;
        self.blocked_pool_plan_errors
            .extend(summary.blocked_pool_plan_errors.iter().copied());
    }
}

fn game_event_spawn_pools_like_cpp(
    manager: &mut wow_map::MapManager,
    canonical_spawn_metadata: &spawn_store_loader::CanonicalSpawnMetadataLikeCpp,
    loaded_grid_creature_respawn_caches: &LoadedGridCreatureRespawnCachesLikeCpp,
    event_pool_ids: &[u32],
) -> GameEventPoolSpawnSummaryLikeCpp {
    let pool_mgr = canonical_spawn_metadata.pool_mgr_like_cpp();
    let mut summary = GameEventPoolSpawnSummaryLikeCpp::default();

    for &pool_id in event_pool_ids {
        summary.event_pool_ids_seen += 1;
        let Some(pool_template) = pool_mgr.pool_template_like_cpp(pool_id) else {
            summary.missing_pool_templates += 1;
            continue;
        };
        let Ok(map_id) = u32::try_from(pool_template.map_id) else {
            summary.invalid_template_map_ids += 1;
            continue;
        };

        let mut maps_matched_for_pool = 0usize;
        manager.do_for_all_maps_mut(|managed_map| {
            if managed_map.map_id() != map_id {
                return;
            }
            maps_matched_for_pool += 1;
            match managed_map
                .map_mut()
                .spawn_pool_loaded_grid_records_like_cpp(
                    pool_mgr,
                    pool_id,
                    canonical_spawn_metadata.spawn_store(),
                    |_kind, _pool_id| 0.0,
                    |_candidates, count| (0..count).collect(),
                    |map, object_type, spawn_id| match object_type {
                        wow_map::SpawnObjectType::Creature => {
                            build_loaded_grid_creature_spawn_group_spawn_record_like_cpp(
                                map,
                                object_type,
                                spawn_id,
                                canonical_spawn_metadata,
                                loaded_grid_creature_respawn_caches,
                            )
                        }
                        wow_map::SpawnObjectType::GameObject => {
                            build_loaded_grid_gameobject_respawn_record_like_cpp(
                                map,
                                object_type,
                                spawn_id,
                                canonical_spawn_metadata,
                                loaded_grid_creature_respawn_caches,
                            )
                        }
                        wow_map::SpawnObjectType::AreaTrigger => None,
                    },
                ) {
                Ok(map_summary) => summary.accumulate_spawn_summary_like_cpp(&map_summary),
                Err(error) => summary.blocked_pool_plan_errors.push(error),
            }
        });
        summary.maps_matched += maps_matched_for_pool;
        if maps_matched_for_pool == 0 {
            summary.pools_without_loaded_canonical_maps += 1;
        }
    }

    summary
}

fn game_event_spawn_pools_for_event_like_cpp(
    manager: &mut wow_map::MapManager,
    canonical_spawn_metadata: &spawn_store_loader::CanonicalSpawnMetadataLikeCpp,
    loaded_grid_creature_respawn_caches: &LoadedGridCreatureRespawnCachesLikeCpp,
    event_id: i16,
) -> GameEventPoolEventSpawnSummaryLikeCpp {
    let Some(event_pool_ids) = canonical_spawn_metadata.game_event_pool_ids_like_cpp(event_id)
    else {
        return GameEventPoolEventSpawnSummaryLikeCpp {
            event_id,
            missing_event_pool_ids: true,
            pool_summary: GameEventPoolSpawnSummaryLikeCpp::default(),
        };
    };

    GameEventPoolEventSpawnSummaryLikeCpp {
        event_id,
        missing_event_pool_ids: false,
        pool_summary: game_event_spawn_pools_like_cpp(
            manager,
            canonical_spawn_metadata,
            loaded_grid_creature_respawn_caches,
            event_pool_ids,
        ),
    }
}

fn game_event_spawn_for_event_like_cpp(
    manager: &mut wow_map::MapManager,
    canonical_spawn_metadata: &spawn_store_loader::CanonicalSpawnMetadataLikeCpp,
    loaded_grid_creature_respawn_caches: &LoadedGridCreatureRespawnCachesLikeCpp,
    event_id: i16,
) -> GameEventSpawnForEventSummaryLikeCpp {
    let non_pool = game_event_spawn_creatures_and_gameobjects_for_event_like_cpp(
        manager,
        canonical_spawn_metadata,
        loaded_grid_creature_respawn_caches,
        event_id,
    );
    let pool_skipped_due_to_non_pool_bucket =
        non_pool.missing_event_creature_guids || non_pool.missing_event_gameobject_guids;
    let pool = if pool_skipped_due_to_non_pool_bucket {
        GameEventPoolEventSpawnSummaryLikeCpp {
            event_id,
            missing_event_pool_ids: false,
            pool_summary: GameEventPoolSpawnSummaryLikeCpp::default(),
        }
    } else {
        game_event_spawn_pools_for_event_like_cpp(
            manager,
            canonical_spawn_metadata,
            loaded_grid_creature_respawn_caches,
            event_id,
        )
    };

    GameEventSpawnForEventSummaryLikeCpp {
        event_id,
        non_pool,
        pool_skipped_due_to_non_pool_bucket,
        pool,
    }
}

fn apply_canonical_spawn_group_condition_update_loaded_grid_records_like_cpp(
    managed_map: &mut wow_map::ManagedMap,
    canonical_spawn_metadata: &spawn_store_loader::CanonicalSpawnMetadataLikeCpp,
    condition_store: &wow_data::ConditionEntriesByTypeStore,
    loaded_grid_creature_respawn_caches: &LoadedGridCreatureRespawnCachesLikeCpp,
) -> Vec<wow_map::map::SpawnGroupConditionUpdateOutcomeLikeCpp> {
    let map_id = managed_map.map_id();
    let instance_id = managed_map.instance_id();
    let difficulty_id = u32::from(managed_map.map().spawn_mode());
    let groups = canonical_spawn_metadata.spawn_group_templates_for_map_like_cpp(map_id);
    if groups.is_empty() {
        debug!(
            map_id,
            instance_id,
            difficulty_id,
            "UpdateSpawnGroupConditions loaded-grid helper found no spawn groups for map"
        );
        return Vec::new();
    }

    let group_templates = groups
        .iter()
        .map(|(_group_id, template)| *template)
        .collect::<Vec<_>>();
    let groups_evaluated = group_templates.len();
    let map_ref = ConditionMapRef::new(map_id, instance_id);
    let map_state = ConditionMapStateSnapshot {
        active_event_ids: &[],
        world_states: &[],
        difficulty_id,
        instance_data: &[],
        instance_data64: &[],
        boss_states: &[],
        scenario_step_id: None,
    };
    let outcomes = managed_map
        .map_mut()
        .apply_update_spawn_group_conditions_loaded_grid_records_like_cpp(
            group_templates,
            canonical_spawn_metadata.spawn_store(),
            |group| {
                is_spawn_group_meeting_map_conditions_like_cpp(
                    condition_store,
                    group.group_id,
                    map_ref,
                    Some(map_state),
                    &[],
                )
            },
            |map, object_type, spawn_id, force| match object_type {
                wow_map::SpawnObjectType::Creature => {
                    let _ = force;
                    // C++ `UpdateSpawnGroupConditions -> SpawnGroupSpawn(spawnGroupId)`
                    // uses default `force=false`; `wow-map` has already filtered active
                    // respawn timers before calling this loaded-grid LoadFromDB seam.
                    build_loaded_grid_creature_spawn_group_spawn_record_like_cpp(
                        map,
                        object_type,
                        spawn_id,
                        canonical_spawn_metadata,
                        loaded_grid_creature_respawn_caches,
                    )
                }
                wow_map::SpawnObjectType::GameObject => {
                    let _ = force;
                    build_loaded_grid_gameobject_respawn_record_like_cpp(
                        map,
                        object_type,
                        spawn_id,
                        canonical_spawn_metadata,
                        loaded_grid_creature_respawn_caches,
                    )
                }
                wow_map::SpawnObjectType::AreaTrigger => None,
            },
        );
    let applied_set_inactive = outcomes
        .iter()
        .filter(|outcome| outcome.applied_change.is_some())
        .count();
    let planned_spawn = outcomes
        .iter()
        .filter(|outcome| {
            matches!(
                outcome.action,
                wow_map::map::SpawnGroupConditionActionLikeCpp::Spawn { .. }
            )
        })
        .count();
    let planned_despawn = outcomes
        .iter()
        .filter(|outcome| {
            matches!(
                outcome.action,
                wow_map::map::SpawnGroupConditionActionLikeCpp::Despawn { .. }
            )
        })
        .count();
    debug!(
        map_id,
        instance_id,
        difficulty_id,
        groups_evaluated,
        outcomes = outcomes.len(),
        applied_set_inactive,
        planned_spawn,
        planned_despawn,
        "Applied C++ UpdateSpawnGroupConditions loaded-grid SpawnGroupSpawn helper to canonical map"
    );

    outcomes
}

fn create_canonical_map_manager(configs: &WorldConfigSet) -> wow_map::MapManager {
    let grid_cleanup_delay_ms =
        world_config_u32(configs, "CONFIG_INTERVAL_GRIDCLEAN", 5 * 60 * 1000)
            .max(wow_map::MIN_GRID_DELAY_MS);
    let map_update_interval_ms = world_config_u32(configs, "CONFIG_INTERVAL_MAPUPDATE", 10)
        .max(wow_map::MIN_MAP_UPDATE_DELAY_MS);
    let map_update_threads = world_config_u32(configs, "CONFIG_NUMTHREADS", 1);

    let mut manager = wow_map::MapManager::new(grid_cleanup_delay_ms, map_update_interval_ms);
    if map_update_threads > 0 {
        manager
            .map_updater_mut()
            .activate(map_update_threads as usize);
    }

    info!(
        "Canonical MapManager initialized: grid_cleanup_delay_ms={}, map_update_interval_ms={}, map_update_threads={}",
        grid_cleanup_delay_ms, map_update_interval_ms, map_update_threads,
    );

    manager
}

fn map_db2_entries_from_stores(
    map_store: &wow_data::MapStore,
    map_difficulty_store: &wow_data::MapDifficultyStore,
    map_id: u32,
    difficulty_id: u8,
) -> Option<MapDb2Entries> {
    let map = map_store.get(map_id)?;
    let map_difficulty = map_difficulty_store.get(map_id, difficulty_id)?;

    Some(MapDb2Entries {
        map_id,
        difficulty_id,
        lock_id: u32::from(map_difficulty.lock_id),
        reset_interval: match map_difficulty.reset_interval {
            1 => MapDifficultyResetInterval::Daily,
            2 => MapDifficultyResetInterval::Weekly,
            _ => MapDifficultyResetInterval::Anytime,
        },
        is_flex_locking: map.is_flex_locking(),
        is_using_encounter_locks: map_difficulty.is_using_encounter_locks(),
    })
}

fn register_loaded_instance_ids(
    legacy_map_manager: &SharedMapManager,
    canonical_map_manager: &Mutex<wow_map::MapManager>,
    instance_ids: &[u32],
) {
    let Some(max_instance_id) = instance_ids.iter().copied().max() else {
        return;
    };

    match legacy_map_manager.write() {
        Ok(mut manager) => {
            manager.init_instance_ids_from_max(max_instance_id);
            for &instance_id in instance_ids {
                manager.register_instance_id(instance_id);
            }
        }
        Err(_) => warn!("Legacy MapManager lock poisoned; persisted instance ids not registered"),
    }

    match canonical_map_manager.lock() {
        Ok(mut manager) => {
            manager.init_instance_ids(u64::from(max_instance_id));
            for &instance_id in instance_ids {
                manager.register_instance_id(instance_id);
            }
        }
        Err(_) => {
            warn!("Canonical MapManager lock poisoned; persisted instance ids not registered")
        }
    }

    info!(
        "Registered {} persisted instance ids with MapManager, max_instance_id={}",
        instance_ids.len(),
        max_instance_id
    );
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct CanonicalGameEventSchedulerLikeCpp {
    timer_ms: u32,
    interval_ms: u32,
}

impl CanonicalGameEventSchedulerLikeCpp {
    fn start_system(next_delay_ms: u64) -> Self {
        let interval_ms = clamp_game_event_delay_ms_like_cpp(next_delay_ms).max(1);
        Self {
            timer_ms: interval_ms,
            interval_ms,
        }
    }

    fn update(&mut self, diff_ms: u32) -> bool {
        if self.timer_ms <= diff_ms {
            self.timer_ms = self.interval_ms;
            true
        } else {
            self.timer_ms -= diff_ms;
            false
        }
    }

    fn set_interval_and_reset(&mut self, next_delay_ms: u64) {
        self.interval_ms = clamp_game_event_delay_ms_like_cpp(next_delay_ms).max(1);
        self.timer_ms = self.interval_ms;
    }

    #[cfg(test)]
    const fn timer_ms(&self) -> u32 {
        self.timer_ms
    }

    #[cfg(test)]
    const fn interval_ms(&self) -> u32 {
        self.interval_ms
    }
}

fn clamp_game_event_delay_ms_like_cpp(delay_ms: u64) -> u32 {
    u32::try_from(delay_ms).unwrap_or(u32::MAX)
}

fn current_unix_time_secs_like_cpp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs())
}

fn game_event_quest_complete_response_from_summary_like_cpp(
    quest_id: u32,
    summary: &GameEventQuestCompleteDbBridgeSummaryLikeCpp,
) -> GameEventQuestCompleteResponseLikeCpp {
    GameEventQuestCompleteResponseLikeCpp {
        quest_id,
        condition_save_updates_queued: summary.condition_save_updates_queued,
        condition_save_updates_executed: summary.condition_save_updates_executed,
        condition_save_updates_failed: summary.condition_save_updates_failed,
        condition_save_updates_skipped_non_progress: summary
            .condition_save_updates_skipped_non_progress,
        save_world_event_state_requested: summary.save_world_event_state_requested,
        world_event_state_save_requested: summary.world_event_state_save_requested,
        world_event_state_saves_queued: summary.world_event_state_summary.saves_queued,
        world_event_state_saves_executed: summary.world_event_state_summary.saves_executed,
        world_event_state_saves_failed: summary.world_event_state_summary.saves_failed,
        world_event_state_saves_skipped_event_id_out_of_range: summary
            .world_event_state_summary
            .saves_skipped_event_id_out_of_range,
        world_event_state_saves_skipped_missing_event: summary
            .world_event_state_summary
            .saves_skipped_missing_event,
        force_game_event_update_requested: summary.force_game_event_update_requested_flag,
        force_game_event_update_requests: summary.force_game_event_update_requested,
        processor_failed: false,
    }
}

fn game_event_quest_complete_processor_failed_response_like_cpp(
    quest_id: u32,
) -> GameEventQuestCompleteResponseLikeCpp {
    GameEventQuestCompleteResponseLikeCpp {
        quest_id,
        processor_failed: true,
        ..GameEventQuestCompleteResponseLikeCpp::default()
    }
}

async fn run_game_event_quest_complete_processor_like_cpp(
    command_rx: flume::Receiver<GameEventQuestCompleteCommandLikeCpp>,
    canonical_spawn_metadata: SharedCanonicalSpawnMetadataLikeCpp,
    character_db: Arc<CharacterDatabase>,
) {
    while let Ok(command) = command_rx.recv_async().await {
        let quest_id = command.quest_id;
        let maybe_summary = {
            let Ok(mut metadata) = canonical_spawn_metadata.lock() else {
                tracing::error!(
                    quest_id,
                    "CanonicalSpawnMetadataLikeCpp mutex poisoned during C++ GameEventMgr::HandleQuestComplete bridge"
                );
                let _ = command.response_tx.try_send(
                    game_event_quest_complete_processor_failed_response_like_cpp(quest_id),
                );
                continue;
            };
            let outcome = metadata.represented_handle_game_event_quest_complete_like_cpp(
                quest_id,
                current_unix_time_secs_like_cpp(),
            );
            materialize_game_event_quest_complete_db_bridge_like_cpp(&outcome, &metadata)
        };

        let mut summary = maybe_summary;
        execute_game_event_quest_complete_condition_save_db_bridge_like_cpp(
            character_db.as_ref(),
            &mut summary,
        )
        .await;
        execute_game_event_world_event_state_db_bridge_like_cpp(
            character_db.as_ref(),
            &mut summary.world_event_state_summary,
        )
        .await;

        let response = game_event_quest_complete_response_from_summary_like_cpp(quest_id, &summary);
        let _ = command.response_tx.try_send(response);
    }
}

fn represented_game_event_world_conditions_met_like_cpp(_event_id: u16) -> bool {
    false
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum GameEventLiveUpdateActionLikeCpp {
    Spawn(i16),
    Unspawn(i16),
    AnnounceEvent {
        event_id: u16,
        description: String,
        description_len: usize,
        announce: u8,
        config_event_announce: bool,
    },
    ChangeEquipOrModel {
        event_id: u16,
        activate: bool,
    },
    RunSmartAIScripts {
        event_id: u16,
        activate: bool,
    },
    ResetEventSeasonalQuests {
        event_id: u16,
        event_start_time: u64,
    },
    UpdateEventQuests {
        event_id: u16,
        activate: bool,
    },
    UpdateWorldStates {
        event_id: u16,
        activate: bool,
    },
    UpdateNpcFlags {
        event_id: u16,
    },
    UpdateNpcVendor {
        event_id: u16,
        activate: bool,
    },
}

#[derive(Debug, Clone)]
struct GameEventSeasonalQuestDbDeleteLikeCpp {
    event_id: u16,
    event_start_time: i64,
    statement: PreparedStatement,
}

#[derive(Debug, Default, Clone)]
struct GameEventLiveUpdateSideEffectSummaryLikeCpp {
    actions: Vec<GameEventLiveUpdateActionLikeCpp>,
    spawn_actions: usize,
    unspawn_actions: usize,
    announce_event_actions: usize,
    announce_event_description_len_total: usize,
    announce_event_world_text_represented: usize,
    announce_event_lines: usize,
    announce_event_registry_missing: usize,
    announce_event_send_attempted: usize,
    announce_event_send_queued: usize,
    announce_event_send_failed: usize,
    announce_event_localization_unrepresented: usize,
    announce_event_in_world_filter_unrepresented: usize,
    announce_event_not_in_world_skipped: usize,
    announce_event_world_text_unimplemented: usize,
    announce_event_session_fanout_unimplemented: usize,
    change_equip_or_model_actions: usize,
    change_equip_or_model_records_seen: usize,
    change_equip_or_model_records_applied: usize,
    change_equip_or_model_missing_event_buckets: usize,
    change_equip_or_model_missing_spawn_metadata: usize,
    change_equip_or_model_missing_runtime_rows: usize,
    change_equip_or_model_maps_matched: usize,
    change_equip_or_model_live_creatures_mutated: usize,
    change_equip_or_model_stale_index_or_wrong_kind: usize,
    change_equip_or_model_model_validation_unavailable: usize,
    run_smart_ai_actions: usize,
    run_smart_ai_maps_visited: usize,
    run_smart_ai_creature_candidates: usize,
    run_smart_ai_gameobject_candidates: usize,
    run_smart_ai_creature_ai_enabled_unrepresented: usize,
    run_smart_ai_script_dispatch_unrepresented: usize,
    reset_event_seasonal_quests_actions: usize,
    reset_event_seasonal_quests_event_start_time_zero: usize,
    reset_event_seasonal_quests_event_start_time_nonzero: usize,
    reset_event_seasonal_quests_player_session_runtime_unimplemented: usize,
    reset_event_seasonal_quests_player_session_registry_missing: usize,
    reset_event_seasonal_quests_player_session_send_attempted: usize,
    reset_event_seasonal_quests_player_session_send_queued: usize,
    reset_event_seasonal_quests_player_session_send_failed: usize,
    reset_event_seasonal_quests_character_db_statement_unimplemented: usize,
    reset_event_seasonal_quests_character_db_delete_queued: usize,
    reset_event_seasonal_quests_character_db_delete_executed: usize,
    reset_event_seasonal_quests_character_db_delete_failed: usize,
    reset_event_seasonal_quests_character_db_delete_skipped_event_start_time_out_of_range: usize,
    reset_event_seasonal_quest_db_deletes: Vec<GameEventSeasonalQuestDbDeleteLikeCpp>,
    update_event_quests_actions: usize,
    update_event_quests_creature_records_seen: usize,
    update_event_quests_gameobject_records_seen: usize,
    update_event_quests_creature_inserted: usize,
    update_event_quests_gameobject_inserted: usize,
    update_event_quests_creature_removed: usize,
    update_event_quests_gameobject_removed: usize,
    update_event_quests_creature_remove_misses: usize,
    update_event_quests_gameobject_remove_misses: usize,
    update_event_quests_creature_no_match: usize,
    update_event_quests_gameobject_no_match: usize,
    update_event_quests_creature_missing_event_buckets: usize,
    update_event_quests_gameobject_missing_event_buckets: usize,
    update_event_quests_creature_skipped_active_other_event: usize,
    update_event_quests_gameobject_skipped_active_other_event: usize,
    update_world_states_actions: usize,
    update_world_states_no_holiday: usize,
    update_world_states_missing_event: usize,
    update_world_states_store_missing: usize,
    update_world_states_holiday_not_weekend_battleground: usize,
    update_world_states_battlemaster_list_missing: usize,
    update_world_states_holiday_world_state_zero: usize,
    update_world_states_holiday_lookup_unrepresented: usize,
    update_world_states_set_value_represented: usize,
    update_world_states_set_value_attempts: usize,
    update_world_states_realm_changed_or_inserted: usize,
    update_world_states_realm_unchanged_noop: usize,
    update_world_states_map_specific_no_map_unsupported: usize,
    update_world_states_global_message_represented: usize,
    update_world_states_global_message_registry_missing: usize,
    update_world_states_global_message_send_attempted: usize,
    update_world_states_global_message_send_queued: usize,
    update_world_states_global_message_send_failed: usize,
    update_world_states_global_message_not_in_world_skipped: usize,
    update_world_states_last_world_state_id: Option<i16>,
    update_world_states_last_world_state_value: Option<i32>,
    update_npc_flags_actions: usize,
    update_npc_flags_records_seen: usize,
    update_npc_flags_missing_event_buckets: usize,
    update_npc_flags_missing_spawn_metadata: usize,
    update_npc_flags_template_npcflag_missing: usize,
    update_npc_flags_maps_matched: usize,
    update_npc_flags_indexed_guids: usize,
    update_npc_flags_live_creatures_mutated: usize,
    update_npc_flags_stale_index_or_wrong_kind: usize,
    update_npc_flags_low_applied: usize,
    update_npc_flags2_applied: usize,
    update_npc_flags_values_updates_built: usize,
    update_npc_flags_values_update_empty: usize,
    update_npc_flags_values_update_map_id_out_of_range: usize,
    update_npc_flags_values_update_registry_missing: usize,
    update_npc_flags_values_update_not_in_world_skipped: usize,
    update_npc_flags_values_update_wrong_map_skipped: usize,
    update_npc_flags_values_update_send_attempted: usize,
    update_npc_flags_values_update_send_queued: usize,
    update_npc_flags_values_update_send_failed: usize,
    update_npc_vendor_actions: usize,
    update_npc_vendor_records_seen: usize,
    update_npc_vendor_items_added: usize,
    update_npc_vendor_items_removed: usize,
    update_npc_vendor_missing_event_buckets: usize,
    update_npc_vendor_remove_misses: usize,
    update_npc_vendor_no_match: usize,
}

fn game_event_signed_id_like_cpp(event_id: u16) -> i16 {
    i16::try_from(event_id).unwrap_or(i16::MAX)
}

fn should_announce_game_event_like_cpp(announce: u8, config_event_announce: bool) -> bool {
    announce == 1 || (announce == 2 && config_event_announce)
}

fn game_event_announcement_lines_like_cpp(description: &str) -> Vec<String> {
    // C++ WorldWorldTextBuilder formats LANG_EVENTMESSAGE first and then
    // ChatHandler::LineFromMessage tokenizes the resulting buffer with strtok("\n"),
    // so empty newline runs are skipped. Rust does not have ObjectMgr TrinityString
    // locale storage yet; represent the known enUS fallback format explicitly.
    let formatted = format!("|cffff0000[Event Message]: {description}|r");
    formatted
        .split('\n')
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect()
}

fn fanout_game_event_announcement_to_player_sessions_like_cpp(
    player_registry: Option<&PlayerRegistry>,
    description: &str,
    summary: &mut GameEventLiveUpdateSideEffectSummaryLikeCpp,
) {
    summary.announce_event_world_text_represented += 1;
    summary.announce_event_localization_unrepresented += 1;

    let lines = game_event_announcement_lines_like_cpp(description);
    summary.announce_event_lines += lines.len();
    if lines.is_empty() {
        return;
    }

    let Some(player_registry) = player_registry else {
        summary.announce_event_registry_missing += 1;
        return;
    };

    let packet_bytes: Vec<Vec<u8>> = lines
        .into_iter()
        .map(|text| {
            ChatPkt {
                msg_type: ChatMsg::System,
                language: 0,
                sender_guid: ObjectGuid::EMPTY,
                sender_name: String::new(),
                target_guid: ObjectGuid::EMPTY,
                target_name: String::new(),
                channel: String::new(),
                text,
                virtual_realm: 0,
            }
            .to_bytes()
        })
        .collect();

    for session in player_registry.iter() {
        if !session.is_in_world {
            summary.announce_event_not_in_world_skipped += 1;
            continue;
        }

        for bytes in &packet_bytes {
            summary.announce_event_send_attempted += 1;
            match session.send_tx.try_send(bytes.clone()) {
                Ok(()) => summary.announce_event_send_queued += 1,
                Err(_) => summary.announce_event_send_failed += 1,
            }
        }
    }
}

fn game_event_seasonal_quest_db_delete_like_cpp(
    event_id: u16,
    event_start_time: u64,
    summary: &mut GameEventLiveUpdateSideEffectSummaryLikeCpp,
) {
    let Ok(event_start_time_i64) = i64::try_from(event_start_time) else {
        summary.reset_event_seasonal_quests_character_db_delete_skipped_event_start_time_out_of_range += 1;
        return;
    };

    let mut statement = PreparedStatement::new(
        CharStatements::DEL_RESET_CHARACTER_QUESTSTATUS_SEASONAL_BY_EVENT.sql(),
    );
    statement.set_u16(0, event_id);
    statement.set_i64(1, event_start_time_i64);

    summary.reset_event_seasonal_quests_character_db_delete_queued += 1;
    summary
        .reset_event_seasonal_quest_db_deletes
        .push(GameEventSeasonalQuestDbDeleteLikeCpp {
            event_id,
            event_start_time: event_start_time_i64,
            statement,
        });
}

fn fanout_reset_event_seasonal_quests_to_player_sessions_like_cpp(
    player_registry: Option<&PlayerRegistry>,
    event_id: u16,
    event_start_time: u64,
    summary: &mut GameEventLiveUpdateSideEffectSummaryLikeCpp,
) {
    let Some(player_registry) = player_registry else {
        summary.reset_event_seasonal_quests_player_session_registry_missing += 1;
        return;
    };

    for session in player_registry.iter() {
        summary.reset_event_seasonal_quests_player_session_send_attempted += 1;
        let command = SessionCommand::ResetSeasonalQuestStatus(ResetSeasonalQuestStatusCommand {
            event_id,
            event_start_time,
        });
        match session.command_tx.try_send(command) {
            Ok(()) => summary.reset_event_seasonal_quests_player_session_send_queued += 1,
            Err(_) => summary.reset_event_seasonal_quests_player_session_send_failed += 1,
        }
    }
}

fn fanout_reset_event_seasonal_quests_to_player_sessions_after_db_delete_like_cpp(
    player_registry: Option<&PlayerRegistry>,
    summary: &mut GameEventLiveUpdateSideEffectSummaryLikeCpp,
) {
    let reset_actions: Vec<(u16, u64)> = summary
        .actions
        .iter()
        .filter_map(|action| match action {
            GameEventLiveUpdateActionLikeCpp::ResetEventSeasonalQuests {
                event_id,
                event_start_time,
            } => Some((*event_id, *event_start_time)),
            _ => None,
        })
        .collect();

    for (event_id, event_start_time) in reset_actions {
        fanout_reset_event_seasonal_quests_to_player_sessions_like_cpp(
            player_registry,
            event_id,
            event_start_time,
            summary,
        );
    }
}

async fn execute_game_event_seasonal_quest_db_deletes_like_cpp(
    character_db: &CharacterDatabase,
    summary: &mut GameEventLiveUpdateSideEffectSummaryLikeCpp,
) {
    let db_delete_total = summary.reset_event_seasonal_quest_db_deletes.len();
    for (db_delete_index, db_delete) in summary
        .reset_event_seasonal_quest_db_deletes
        .drain(..)
        .enumerate()
    {
        match character_db.execute(&db_delete.statement).await {
            Ok(_) => {
                summary.reset_event_seasonal_quests_character_db_delete_executed += 1;
            }
            Err(error) => {
                summary.reset_event_seasonal_quests_character_db_delete_failed += 1;
                tracing::error!(
                    error = %error,
                    db_delete_index = db_delete_index + 1,
                    db_delete_total,
                    event_id = db_delete.event_id,
                    event_start_time = db_delete.event_start_time,
                    "Failed to execute C++ World::ResetEventSeasonalQuests character DB delete; continuing live update loop"
                );
            }
        }
    }
}

fn game_event_live_update_actions_like_cpp(
    canonical_spawn_metadata: &spawn_store_loader::CanonicalSpawnMetadataLikeCpp,
    outcome: &spawn_store_loader::GameEventUpdateOutcomeLikeCpp,
    config_event_announce: bool,
) -> Vec<GameEventLiveUpdateActionLikeCpp> {
    let mut actions = Vec::new();
    for &event_id in &outcome.negative_spawn_event_ids {
        actions.push(GameEventLiveUpdateActionLikeCpp::Spawn(event_id));
    }
    for start_outcome in &outcome.start_outcomes {
        if let spawn_store_loader::GameEventStartOutcomeLikeCpp::Started(summary) = start_outcome {
            if summary.apply_new_event_requested {
                let event_id = game_event_signed_id_like_cpp(summary.event_id);
                if let Some(event) = canonical_spawn_metadata.game_event_like_cpp(summary.event_id)
                {
                    if should_announce_game_event_like_cpp(event.announce, config_event_announce) {
                        actions.push(GameEventLiveUpdateActionLikeCpp::AnnounceEvent {
                            event_id: summary.event_id,
                            description: event.description.clone(),
                            description_len: event.description.len(),
                            announce: event.announce,
                            config_event_announce,
                        });
                    }
                }
                actions.push(GameEventLiveUpdateActionLikeCpp::Spawn(event_id));
                actions.push(GameEventLiveUpdateActionLikeCpp::Unspawn(-event_id));
                actions.push(GameEventLiveUpdateActionLikeCpp::ChangeEquipOrModel {
                    event_id: summary.event_id,
                    activate: true,
                });
                actions.push(GameEventLiveUpdateActionLikeCpp::UpdateEventQuests {
                    event_id: summary.event_id,
                    activate: true,
                });
                actions.push(GameEventLiveUpdateActionLikeCpp::UpdateWorldStates {
                    event_id: summary.event_id,
                    activate: true,
                });
                actions.push(GameEventLiveUpdateActionLikeCpp::UpdateNpcFlags {
                    event_id: summary.event_id,
                });
                actions.push(GameEventLiveUpdateActionLikeCpp::UpdateNpcVendor {
                    event_id: summary.event_id,
                    activate: true,
                });
                actions.push(GameEventLiveUpdateActionLikeCpp::RunSmartAIScripts {
                    event_id: summary.event_id,
                    activate: true,
                });
                actions.push(GameEventLiveUpdateActionLikeCpp::ResetEventSeasonalQuests {
                    event_id: summary.event_id,
                    event_start_time: canonical_spawn_metadata.game_event_last_start_time_like_cpp(
                        summary.event_id,
                        outcome.current_time_secs,
                    ),
                });
            }
        }
    }
    for stop_outcome in &outcome.stop_outcomes {
        if let spawn_store_loader::GameEventStopOutcomeLikeCpp::Stopped(summary) = stop_outcome {
            if summary.unapply_event_requested {
                let event_id = game_event_signed_id_like_cpp(summary.event_id);
                actions.push(GameEventLiveUpdateActionLikeCpp::RunSmartAIScripts {
                    event_id: summary.event_id,
                    activate: false,
                });
                actions.push(GameEventLiveUpdateActionLikeCpp::Unspawn(event_id));
                actions.push(GameEventLiveUpdateActionLikeCpp::Spawn(-event_id));
                actions.push(GameEventLiveUpdateActionLikeCpp::ChangeEquipOrModel {
                    event_id: summary.event_id,
                    activate: false,
                });
                actions.push(GameEventLiveUpdateActionLikeCpp::UpdateEventQuests {
                    event_id: summary.event_id,
                    activate: false,
                });
                actions.push(GameEventLiveUpdateActionLikeCpp::UpdateWorldStates {
                    event_id: summary.event_id,
                    activate: false,
                });
                actions.push(GameEventLiveUpdateActionLikeCpp::UpdateNpcFlags {
                    event_id: summary.event_id,
                });
                actions.push(GameEventLiveUpdateActionLikeCpp::UpdateNpcVendor {
                    event_id: summary.event_id,
                    activate: false,
                });
            }
        }
    }
    actions
}

fn game_event_change_equip_or_model_like_cpp(
    manager: &mut wow_map::MapManager,
    canonical_spawn_metadata: &mut spawn_store_loader::CanonicalSpawnMetadataLikeCpp,
    event_id: u16,
    activate: bool,
) -> GameEventLiveUpdateSideEffectSummaryLikeCpp {
    let mut summary = GameEventLiveUpdateSideEffectSummaryLikeCpp::default();
    let records = canonical_spawn_metadata
        .game_event_model_equip_like_cpp(event_id)
        .map_or_else(Vec::new, <[_]>::to_vec);

    for record in &records {
        let Some(spawn_data) = canonical_spawn_metadata
            .spawn_store()
            .spawn_data(wow_map::SpawnObjectType::Creature, record.spawn_id)
        else {
            summary.change_equip_or_model_missing_spawn_metadata += 1;
            continue;
        };

        let (equipment_id, model_id) = if activate {
            (record.equipment_id, record.model_id)
        } else {
            (record.equipment_id_prev, record.model_id_prev)
        };
        let mut maps_matched_for_record = 0usize;
        manager.do_for_all_maps_mut(|map| {
            if map.map_id() == spawn_data.map_id {
                maps_matched_for_record += 1;
                let outcome = map
                    .map_mut()
                    .change_game_event_equip_or_model_by_spawn_id_like_cpp(
                        record.spawn_id,
                        equipment_id,
                        model_id,
                        false,
                    );
                summary.change_equip_or_model_live_creatures_mutated +=
                    outcome.live_creatures_mutated;
                summary.change_equip_or_model_stale_index_or_wrong_kind +=
                    outcome.stale_index_or_wrong_kind;
                summary.change_equip_or_model_model_validation_unavailable +=
                    outcome.model_validation_unavailable;
            }
        });
        summary.change_equip_or_model_maps_matched += maps_matched_for_record;
    }

    let baseline_summary = canonical_spawn_metadata
        .change_game_event_model_equip_baseline_like_cpp(event_id, activate);
    summary.change_equip_or_model_records_seen += baseline_summary.records_seen;
    summary.change_equip_or_model_records_applied += baseline_summary.records_applied;
    if baseline_summary.missing_event_bucket {
        summary.change_equip_or_model_missing_event_buckets += 1;
    }
    summary.change_equip_or_model_missing_spawn_metadata += baseline_summary.missing_spawn_metadata;
    summary.change_equip_or_model_missing_runtime_rows +=
        baseline_summary.missing_creature_runtime_rows;
    summary
}

fn fanout_game_event_npc_flag_values_update_to_visible_sessions_like_cpp(
    player_registry: Option<&PlayerRegistry>,
    values_update: &wow_map::GameEventNpcFlagValuesUpdateLikeCpp,
    summary: &mut GameEventLiveUpdateSideEffectSummaryLikeCpp,
) {
    let Ok(map_id) = u16::try_from(values_update.map_id) else {
        summary.update_npc_flags_values_update_map_id_out_of_range += 1;
        return;
    };
    let Some(packet_update) = unit_values_update_to_packet(&values_update.values_update) else {
        summary.update_npc_flags_values_update_empty += 1;
        return;
    };
    let update = wow_packet::packets::update::UpdateObject::unit_values_update(
        values_update.guid,
        map_id,
        packet_update.clone(),
    );
    summary.update_npc_flags_values_updates_built += 1;

    let Some(player_registry) = player_registry else {
        summary.update_npc_flags_values_update_registry_missing += 1;
        return;
    };

    let packet_bytes = update.to_bytes();
    for session in player_registry.iter() {
        if !session.is_in_world {
            summary.update_npc_flags_values_update_not_in_world_skipped += 1;
            continue;
        }
        if session.map_id != map_id {
            summary.update_npc_flags_values_update_wrong_map_skipped += 1;
            continue;
        }

        summary.update_npc_flags_values_update_send_attempted += 1;
        let command =
            SessionCommand::SendVisibleObjectValuesUpdate(SendVisibleObjectValuesUpdateCommand {
                object_guid: values_update.guid,
                map_id,
                packet_bytes: packet_bytes.clone(),
                unit_values_update: Some(packet_update.clone()),
            });
        match session.command_tx.try_send(command) {
            Ok(()) => summary.update_npc_flags_values_update_send_queued += 1,
            Err(_) => summary.update_npc_flags_values_update_send_failed += 1,
        }
    }
}

fn game_event_update_npc_flags_like_cpp(
    manager: &mut wow_map::MapManager,
    canonical_spawn_metadata: &spawn_store_loader::CanonicalSpawnMetadataLikeCpp,
    player_registry: Option<&PlayerRegistry>,
    event_id: u16,
    active_event_ids: &[u16],
) -> GameEventLiveUpdateSideEffectSummaryLikeCpp {
    let mut summary = GameEventLiveUpdateSideEffectSummaryLikeCpp::default();
    let Some(records) = canonical_spawn_metadata.game_event_npc_flags_like_cpp(event_id) else {
        summary.update_npc_flags_missing_event_buckets += 1;
        return summary;
    };
    summary.update_npc_flags_records_seen = records.len();

    for record in records {
        let Some(spawn_data) = canonical_spawn_metadata
            .spawn_store()
            .spawn_data(wow_map::SpawnObjectType::Creature, record.spawn_id)
        else {
            summary.update_npc_flags_missing_spawn_metadata += 1;
            continue;
        };
        summary.update_npc_flags_template_npcflag_missing += 1;
        let overlay = canonical_spawn_metadata
            .game_event_npc_flag_mask_like_cpp(record.spawn_id, active_event_ids);
        let npcflag_mask_with_template = overlay | 0;

        let mut maps_matched_for_record = 0usize;
        manager.do_for_all_maps_mut(|map| {
            if map.map_id() == spawn_data.map_id {
                maps_matched_for_record += 1;
                let outcome = map
                    .map_mut()
                    .update_game_event_npc_flags_by_spawn_id_like_cpp(
                        record.spawn_id,
                        npcflag_mask_with_template,
                    );
                summary.update_npc_flags_indexed_guids += outcome.indexed_guids;
                summary.update_npc_flags_live_creatures_mutated += outcome.live_creatures_mutated;
                summary.update_npc_flags_stale_index_or_wrong_kind +=
                    outcome.stale_index_or_wrong_kind;
                summary.update_npc_flags_low_applied += outcome.npc_flags_low_applied;
                summary.update_npc_flags2_applied += outcome.npc_flags2_applied;
                for values_update in &outcome.values_updates {
                    fanout_game_event_npc_flag_values_update_to_visible_sessions_like_cpp(
                        player_registry,
                        values_update,
                        &mut summary,
                    );
                }
            }
        });
        summary.update_npc_flags_maps_matched += maps_matched_for_record;
    }

    summary
}

fn fanout_realm_update_world_state_to_player_sessions_like_cpp(
    player_registry: Option<&PlayerRegistry>,
    world_state_id: i32,
    value: i32,
    hidden: bool,
    summary: &mut GameEventLiveUpdateSideEffectSummaryLikeCpp,
) {
    let Some(player_registry) = player_registry else {
        summary.update_world_states_global_message_registry_missing += 1;
        return;
    };

    // C++ assigns signed `int32 worldStateId` into packet `uint32 VariableID`;
    // Rust's `as u32` preserves the same two's-complement wrapping semantics.
    let packet = wow_packet::packets::misc::UpdateWorldState {
        variable_id: world_state_id as u32,
        value,
        hidden,
    };
    let bytes = packet.to_bytes();

    for session in player_registry.iter() {
        if !session.is_in_world {
            summary.update_world_states_global_message_not_in_world_skipped += 1;
            continue;
        }

        summary.update_world_states_global_message_send_attempted += 1;
        match session.send_tx.try_send(bytes.clone()) {
            Ok(()) => summary.update_world_states_global_message_send_queued += 1,
            Err(_) => summary.update_world_states_global_message_send_failed += 1,
        }
    }
}

fn game_event_update_npc_vendor_like_cpp(
    canonical_spawn_metadata: &mut spawn_store_loader::CanonicalSpawnMetadataLikeCpp,
    event_id: u16,
    activate: bool,
) -> GameEventLiveUpdateSideEffectSummaryLikeCpp {
    let vendor_summary =
        canonical_spawn_metadata.update_game_event_npc_vendor_cache_like_cpp(event_id, activate);
    let mut summary = GameEventLiveUpdateSideEffectSummaryLikeCpp::default();
    summary.update_npc_vendor_records_seen = vendor_summary.records_seen;
    summary.update_npc_vendor_items_added = vendor_summary.items_added;
    summary.update_npc_vendor_items_removed = vendor_summary.items_removed;
    summary.update_npc_vendor_remove_misses = vendor_summary.remove_misses;
    summary.update_npc_vendor_no_match = vendor_summary.no_match;
    if vendor_summary.missing_event_bucket {
        summary.update_npc_vendor_missing_event_buckets = 1;
    }
    summary
}

fn game_event_update_world_states_like_cpp(
    canonical_spawn_metadata: &spawn_store_loader::CanonicalSpawnMetadataLikeCpp,
    battlemaster_list_store: Option<&wow_data::BattlemasterListStore>,
    mut world_state_mgr: Option<&mut spawn_store_loader::WorldStateMgrLikeCpp>,
    player_registry: Option<&PlayerRegistry>,
    event_id: u16,
    activate: bool,
) -> GameEventLiveUpdateSideEffectSummaryLikeCpp {
    let mut summary = GameEventLiveUpdateSideEffectSummaryLikeCpp::default();
    let Some(event) = canonical_spawn_metadata.game_event_like_cpp(event_id) else {
        summary.update_world_states_missing_event = 1;
        return summary;
    };

    if event.holiday_id == 0 {
        summary.update_world_states_no_holiday = 1;
        return summary;
    }

    let Some(battlemaster_list_store) = battlemaster_list_store else {
        summary.update_world_states_store_missing = 1;
        summary.update_world_states_holiday_lookup_unrepresented = 1;
        return summary;
    };

    match battlemaster_list_store.holiday_world_state_for_weekend_holiday_like_cpp(event.holiday_id)
    {
        wow_data::HolidayWorldStateLookupLikeCpp::HolidayNone => {
            summary.update_world_states_no_holiday = 1;
        }
        wow_data::HolidayWorldStateLookupLikeCpp::HolidayNotWeekendBattleground { .. } => {
            summary.update_world_states_holiday_not_weekend_battleground = 1;
            summary.update_world_states_holiday_lookup_unrepresented = 1;
        }
        wow_data::HolidayWorldStateLookupLikeCpp::BattlemasterListMissing { .. } => {
            summary.update_world_states_battlemaster_list_missing = 1;
            summary.update_world_states_holiday_lookup_unrepresented = 1;
        }
        wow_data::HolidayWorldStateLookupLikeCpp::HolidayWorldStateZero { .. } => {
            summary.update_world_states_holiday_world_state_zero = 1;
        }
        wow_data::HolidayWorldStateLookupLikeCpp::SetValueRepresented {
            world_state_id, ..
        } => {
            let value = if activate { 1 } else { 0 };
            summary.update_world_states_set_value_attempts = 1;
            summary.update_world_states_last_world_state_id = Some(world_state_id);
            summary.update_world_states_last_world_state_value = Some(value);
            let Some(world_state_mgr) = world_state_mgr.as_deref_mut() else {
                summary.update_world_states_set_value_represented = 1;
                return summary;
            };
            match world_state_mgr.set_value_realm_or_map_null_like_cpp(
                i32::from(world_state_id),
                value,
                false,
            ) {
                spawn_store_loader::WorldStateSetValueOutcomeLikeCpp::RealmInsertedOrChanged {
                    world_state_id,
                    new_value,
                    hidden,
                    global_message_represented,
                    ..
                } => {
                    summary.update_world_states_realm_changed_or_inserted = 1;
                    if global_message_represented {
                        summary.update_world_states_global_message_represented = 1;
                        fanout_realm_update_world_state_to_player_sessions_like_cpp(
                            player_registry,
                            world_state_id,
                            new_value,
                            hidden,
                            &mut summary,
                        );
                    }
                }
                spawn_store_loader::WorldStateSetValueOutcomeLikeCpp::RealmUnchanged { .. } => {
                    summary.update_world_states_realm_unchanged_noop = 1;
                }
                spawn_store_loader::WorldStateSetValueOutcomeLikeCpp::MapSpecificNoMapUnsupported { .. } => {
                    summary.update_world_states_map_specific_no_map_unsupported = 1;
                }
            }
        }
    }

    summary
}

fn game_event_update_quests_like_cpp(
    canonical_spawn_metadata: &mut spawn_store_loader::CanonicalSpawnMetadataLikeCpp,
    event_id: u16,
    activate: bool,
) -> GameEventLiveUpdateSideEffectSummaryLikeCpp {
    let quest_summary = canonical_spawn_metadata
        .update_game_event_quest_relation_cache_like_cpp(event_id, activate);
    let mut summary = GameEventLiveUpdateSideEffectSummaryLikeCpp::default();
    summary.update_event_quests_creature_records_seen = quest_summary.creature_records_seen;
    summary.update_event_quests_gameobject_records_seen = quest_summary.gameobject_records_seen;
    summary.update_event_quests_creature_inserted = quest_summary.creature_inserted;
    summary.update_event_quests_gameobject_inserted = quest_summary.gameobject_inserted;
    summary.update_event_quests_creature_removed = quest_summary.creature_removed;
    summary.update_event_quests_gameobject_removed = quest_summary.gameobject_removed;
    summary.update_event_quests_creature_remove_misses = quest_summary.creature_remove_misses;
    summary.update_event_quests_gameobject_remove_misses = quest_summary.gameobject_remove_misses;
    summary.update_event_quests_creature_no_match = quest_summary.creature_no_match;
    summary.update_event_quests_gameobject_no_match = quest_summary.gameobject_no_match;
    summary.update_event_quests_creature_skipped_active_other_event =
        quest_summary.creature_skipped_active_other_event;
    summary.update_event_quests_gameobject_skipped_active_other_event =
        quest_summary.gameobject_skipped_active_other_event;
    if quest_summary.creature_missing_event_bucket {
        summary.update_event_quests_creature_missing_event_buckets = 1;
    }
    if quest_summary.gameobject_missing_event_bucket {
        summary.update_event_quests_gameobject_missing_event_buckets = 1;
    }
    summary
}

fn game_event_run_smart_ai_scripts_like_cpp(
    manager: &wow_map::MapManager,
    _event_id: u16,
    _activate: bool,
) -> GameEventLiveUpdateSideEffectSummaryLikeCpp {
    let mut summary = GameEventLiveUpdateSideEffectSummaryLikeCpp::default();
    manager.do_for_all_maps(|managed_map| {
        let candidates = managed_map
            .map()
            .game_event_smart_ai_script_candidates_like_cpp();
        summary.run_smart_ai_maps_visited += candidates.maps_visited;
        summary.run_smart_ai_creature_candidates += candidates.in_world_creature_candidates;
        summary.run_smart_ai_gameobject_candidates += candidates.in_world_gameobject_candidates;
        summary.run_smart_ai_creature_ai_enabled_unrepresented +=
            candidates.creature_ai_enabled_unrepresented;
        summary.run_smart_ai_script_dispatch_unrepresented +=
            candidates.script_dispatch_unrepresented;
    });
    summary
}

fn consume_game_event_live_update_side_effects_like_cpp(
    manager: &mut wow_map::MapManager,
    canonical_spawn_metadata: &mut spawn_store_loader::CanonicalSpawnMetadataLikeCpp,
    loaded_grid_creature_respawn_caches: &LoadedGridCreatureRespawnCachesLikeCpp,
    battlemaster_list_store: Option<&wow_data::BattlemasterListStore>,
    mut world_state_mgr: Option<&mut spawn_store_loader::WorldStateMgrLikeCpp>,
    player_registry: Option<&PlayerRegistry>,
    active_event_ids: &[u16],
    outcome: &spawn_store_loader::GameEventUpdateOutcomeLikeCpp,
    config_event_announce: bool,
) -> GameEventLiveUpdateSideEffectSummaryLikeCpp {
    let actions = game_event_live_update_actions_like_cpp(
        canonical_spawn_metadata,
        outcome,
        config_event_announce,
    );
    let mut summary = GameEventLiveUpdateSideEffectSummaryLikeCpp {
        actions,
        ..GameEventLiveUpdateSideEffectSummaryLikeCpp::default()
    };
    for action in summary.actions.clone() {
        match action {
            GameEventLiveUpdateActionLikeCpp::AnnounceEvent {
                event_id: _,
                description,
                description_len,
                announce: _,
                config_event_announce: _,
            } => {
                summary.announce_event_actions += 1;
                summary.announce_event_description_len_total += description_len;
                fanout_game_event_announcement_to_player_sessions_like_cpp(
                    player_registry,
                    &description,
                    &mut summary,
                );
            }
            GameEventLiveUpdateActionLikeCpp::Spawn(event_id) => {
                let _ = game_event_spawn_for_event_like_cpp(
                    manager,
                    canonical_spawn_metadata,
                    loaded_grid_creature_respawn_caches,
                    event_id,
                );
                summary.spawn_actions += 1;
            }
            GameEventLiveUpdateActionLikeCpp::Unspawn(event_id) => {
                let _ = game_event_unspawn_for_event_like_cpp(
                    manager,
                    canonical_spawn_metadata,
                    active_event_ids,
                    event_id,
                );
                summary.unspawn_actions += 1;
            }
            GameEventLiveUpdateActionLikeCpp::ChangeEquipOrModel { event_id, activate } => {
                let change_summary = game_event_change_equip_or_model_like_cpp(
                    manager,
                    canonical_spawn_metadata,
                    event_id,
                    activate,
                );
                summary.change_equip_or_model_actions += 1;
                summary.change_equip_or_model_records_seen +=
                    change_summary.change_equip_or_model_records_seen;
                summary.change_equip_or_model_records_applied +=
                    change_summary.change_equip_or_model_records_applied;
                summary.change_equip_or_model_missing_event_buckets +=
                    change_summary.change_equip_or_model_missing_event_buckets;
                summary.change_equip_or_model_missing_spawn_metadata +=
                    change_summary.change_equip_or_model_missing_spawn_metadata;
                summary.change_equip_or_model_missing_runtime_rows +=
                    change_summary.change_equip_or_model_missing_runtime_rows;
                summary.change_equip_or_model_maps_matched +=
                    change_summary.change_equip_or_model_maps_matched;
                summary.change_equip_or_model_live_creatures_mutated +=
                    change_summary.change_equip_or_model_live_creatures_mutated;
                summary.change_equip_or_model_stale_index_or_wrong_kind +=
                    change_summary.change_equip_or_model_stale_index_or_wrong_kind;
                summary.change_equip_or_model_model_validation_unavailable +=
                    change_summary.change_equip_or_model_model_validation_unavailable;
            }
            GameEventLiveUpdateActionLikeCpp::RunSmartAIScripts { event_id, activate } => {
                let smart_ai_summary =
                    game_event_run_smart_ai_scripts_like_cpp(manager, event_id, activate);
                summary.run_smart_ai_actions += 1;
                summary.run_smart_ai_maps_visited += smart_ai_summary.run_smart_ai_maps_visited;
                summary.run_smart_ai_creature_candidates +=
                    smart_ai_summary.run_smart_ai_creature_candidates;
                summary.run_smart_ai_gameobject_candidates +=
                    smart_ai_summary.run_smart_ai_gameobject_candidates;
                summary.run_smart_ai_creature_ai_enabled_unrepresented +=
                    smart_ai_summary.run_smart_ai_creature_ai_enabled_unrepresented;
                summary.run_smart_ai_script_dispatch_unrepresented +=
                    smart_ai_summary.run_smart_ai_script_dispatch_unrepresented;
            }
            GameEventLiveUpdateActionLikeCpp::ResetEventSeasonalQuests {
                event_id,
                event_start_time,
            } => {
                summary.reset_event_seasonal_quests_actions += 1;
                if event_start_time == 0 {
                    summary.reset_event_seasonal_quests_event_start_time_zero += 1;
                } else {
                    summary.reset_event_seasonal_quests_event_start_time_nonzero += 1;
                }
                game_event_seasonal_quest_db_delete_like_cpp(
                    event_id,
                    event_start_time,
                    &mut summary,
                );
            }
            GameEventLiveUpdateActionLikeCpp::UpdateEventQuests { event_id, activate } => {
                let quest_summary =
                    game_event_update_quests_like_cpp(canonical_spawn_metadata, event_id, activate);
                summary.update_event_quests_actions += 1;
                summary.update_event_quests_creature_records_seen +=
                    quest_summary.update_event_quests_creature_records_seen;
                summary.update_event_quests_gameobject_records_seen +=
                    quest_summary.update_event_quests_gameobject_records_seen;
                summary.update_event_quests_creature_inserted +=
                    quest_summary.update_event_quests_creature_inserted;
                summary.update_event_quests_gameobject_inserted +=
                    quest_summary.update_event_quests_gameobject_inserted;
                summary.update_event_quests_creature_removed +=
                    quest_summary.update_event_quests_creature_removed;
                summary.update_event_quests_gameobject_removed +=
                    quest_summary.update_event_quests_gameobject_removed;
                summary.update_event_quests_creature_remove_misses +=
                    quest_summary.update_event_quests_creature_remove_misses;
                summary.update_event_quests_gameobject_remove_misses +=
                    quest_summary.update_event_quests_gameobject_remove_misses;
                summary.update_event_quests_creature_no_match +=
                    quest_summary.update_event_quests_creature_no_match;
                summary.update_event_quests_gameobject_no_match +=
                    quest_summary.update_event_quests_gameobject_no_match;
                summary.update_event_quests_creature_missing_event_buckets +=
                    quest_summary.update_event_quests_creature_missing_event_buckets;
                summary.update_event_quests_gameobject_missing_event_buckets +=
                    quest_summary.update_event_quests_gameobject_missing_event_buckets;
                summary.update_event_quests_creature_skipped_active_other_event +=
                    quest_summary.update_event_quests_creature_skipped_active_other_event;
                summary.update_event_quests_gameobject_skipped_active_other_event +=
                    quest_summary.update_event_quests_gameobject_skipped_active_other_event;
            }
            GameEventLiveUpdateActionLikeCpp::UpdateWorldStates { event_id, activate } => {
                let world_state_summary = game_event_update_world_states_like_cpp(
                    canonical_spawn_metadata,
                    battlemaster_list_store,
                    world_state_mgr.as_deref_mut(),
                    player_registry,
                    event_id,
                    activate,
                );
                summary.update_world_states_actions += 1;
                summary.update_world_states_no_holiday +=
                    world_state_summary.update_world_states_no_holiday;
                summary.update_world_states_missing_event +=
                    world_state_summary.update_world_states_missing_event;
                summary.update_world_states_store_missing +=
                    world_state_summary.update_world_states_store_missing;
                summary.update_world_states_holiday_not_weekend_battleground +=
                    world_state_summary.update_world_states_holiday_not_weekend_battleground;
                summary.update_world_states_battlemaster_list_missing +=
                    world_state_summary.update_world_states_battlemaster_list_missing;
                summary.update_world_states_holiday_world_state_zero +=
                    world_state_summary.update_world_states_holiday_world_state_zero;
                summary.update_world_states_holiday_lookup_unrepresented +=
                    world_state_summary.update_world_states_holiday_lookup_unrepresented;
                summary.update_world_states_set_value_represented +=
                    world_state_summary.update_world_states_set_value_represented;
                summary.update_world_states_set_value_attempts +=
                    world_state_summary.update_world_states_set_value_attempts;
                summary.update_world_states_realm_changed_or_inserted +=
                    world_state_summary.update_world_states_realm_changed_or_inserted;
                summary.update_world_states_realm_unchanged_noop +=
                    world_state_summary.update_world_states_realm_unchanged_noop;
                summary.update_world_states_map_specific_no_map_unsupported +=
                    world_state_summary.update_world_states_map_specific_no_map_unsupported;
                summary.update_world_states_global_message_represented +=
                    world_state_summary.update_world_states_global_message_represented;
                summary.update_world_states_global_message_registry_missing +=
                    world_state_summary.update_world_states_global_message_registry_missing;
                summary.update_world_states_global_message_send_attempted +=
                    world_state_summary.update_world_states_global_message_send_attempted;
                summary.update_world_states_global_message_send_queued +=
                    world_state_summary.update_world_states_global_message_send_queued;
                summary.update_world_states_global_message_send_failed +=
                    world_state_summary.update_world_states_global_message_send_failed;
                summary.update_world_states_global_message_not_in_world_skipped +=
                    world_state_summary.update_world_states_global_message_not_in_world_skipped;
                summary.update_world_states_last_world_state_id =
                    world_state_summary.update_world_states_last_world_state_id;
                summary.update_world_states_last_world_state_value =
                    world_state_summary.update_world_states_last_world_state_value;
            }
            GameEventLiveUpdateActionLikeCpp::UpdateNpcFlags { event_id } => {
                let npc_flag_summary = game_event_update_npc_flags_like_cpp(
                    manager,
                    canonical_spawn_metadata,
                    player_registry,
                    event_id,
                    active_event_ids,
                );
                summary.update_npc_flags_actions += 1;
                summary.update_npc_flags_records_seen +=
                    npc_flag_summary.update_npc_flags_records_seen;
                summary.update_npc_flags_missing_event_buckets +=
                    npc_flag_summary.update_npc_flags_missing_event_buckets;
                summary.update_npc_flags_missing_spawn_metadata +=
                    npc_flag_summary.update_npc_flags_missing_spawn_metadata;
                summary.update_npc_flags_template_npcflag_missing +=
                    npc_flag_summary.update_npc_flags_template_npcflag_missing;
                summary.update_npc_flags_maps_matched +=
                    npc_flag_summary.update_npc_flags_maps_matched;
                summary.update_npc_flags_indexed_guids +=
                    npc_flag_summary.update_npc_flags_indexed_guids;
                summary.update_npc_flags_live_creatures_mutated +=
                    npc_flag_summary.update_npc_flags_live_creatures_mutated;
                summary.update_npc_flags_stale_index_or_wrong_kind +=
                    npc_flag_summary.update_npc_flags_stale_index_or_wrong_kind;
                summary.update_npc_flags_low_applied +=
                    npc_flag_summary.update_npc_flags_low_applied;
                summary.update_npc_flags2_applied += npc_flag_summary.update_npc_flags2_applied;
                summary.update_npc_flags_values_updates_built +=
                    npc_flag_summary.update_npc_flags_values_updates_built;
                summary.update_npc_flags_values_update_empty +=
                    npc_flag_summary.update_npc_flags_values_update_empty;
                summary.update_npc_flags_values_update_map_id_out_of_range +=
                    npc_flag_summary.update_npc_flags_values_update_map_id_out_of_range;
                summary.update_npc_flags_values_update_registry_missing +=
                    npc_flag_summary.update_npc_flags_values_update_registry_missing;
                summary.update_npc_flags_values_update_not_in_world_skipped +=
                    npc_flag_summary.update_npc_flags_values_update_not_in_world_skipped;
                summary.update_npc_flags_values_update_wrong_map_skipped +=
                    npc_flag_summary.update_npc_flags_values_update_wrong_map_skipped;
                summary.update_npc_flags_values_update_send_attempted +=
                    npc_flag_summary.update_npc_flags_values_update_send_attempted;
                summary.update_npc_flags_values_update_send_queued +=
                    npc_flag_summary.update_npc_flags_values_update_send_queued;
                summary.update_npc_flags_values_update_send_failed +=
                    npc_flag_summary.update_npc_flags_values_update_send_failed;
            }
            GameEventLiveUpdateActionLikeCpp::UpdateNpcVendor { event_id, activate } => {
                let npc_vendor_summary = game_event_update_npc_vendor_like_cpp(
                    canonical_spawn_metadata,
                    event_id,
                    activate,
                );
                summary.update_npc_vendor_actions += 1;
                summary.update_npc_vendor_records_seen +=
                    npc_vendor_summary.update_npc_vendor_records_seen;
                summary.update_npc_vendor_items_added +=
                    npc_vendor_summary.update_npc_vendor_items_added;
                summary.update_npc_vendor_items_removed +=
                    npc_vendor_summary.update_npc_vendor_items_removed;
                summary.update_npc_vendor_missing_event_buckets +=
                    npc_vendor_summary.update_npc_vendor_missing_event_buckets;
                summary.update_npc_vendor_remove_misses +=
                    npc_vendor_summary.update_npc_vendor_remove_misses;
                summary.update_npc_vendor_no_match += npc_vendor_summary.update_npc_vendor_no_match;
            }
        }
    }
    summary
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct CanonicalRespawnConditionSchedulerLikeCpp {
    timer_ms: u32,
    interval_ms: u32,
}

impl CanonicalRespawnConditionSchedulerLikeCpp {
    fn new(interval_ms: u32) -> Self {
        let interval_ms = interval_ms.max(1);
        Self {
            timer_ms: interval_ms,
            interval_ms,
        }
    }

    fn update(&mut self, diff_ms: u32) -> bool {
        if self.timer_ms <= diff_ms {
            self.timer_ms = self.interval_ms;
            true
        } else {
            self.timer_ms -= diff_ms;
            false
        }
    }

    #[cfg(test)]
    const fn timer_ms(&self) -> u32 {
        self.timer_ms
    }
}

#[derive(Debug, Clone)]
struct RespawnDbDeleteLikeCpp {
    object_type: wow_map::SpawnObjectType,
    spawn_id: wow_map::SpawnId,
    map_id: u16,
    instance_id: u32,
    statement: PreparedStatement,
}

#[derive(Debug, Clone)]
enum RespawnDbDeleteQueueOutcomeLikeCpp {
    Queued(RespawnDbDeleteLikeCpp),
    SkippedNonWorldMap,
    SkippedInvalidMapId,
}

#[derive(Debug, Clone)]
struct RespawnDbSaveLikeCpp {
    object_type: wow_map::SpawnObjectType,
    spawn_id: wow_map::SpawnId,
    respawn_time: i64,
    map_id: u16,
    instance_id: u32,
    statement: PreparedStatement,
}

#[derive(Debug, Clone)]
enum RespawnDbSaveQueueOutcomeLikeCpp {
    Queued(RespawnDbSaveLikeCpp),
    SkippedNonWorldMap,
    SkippedInvalidMapId,
}

fn queue_respawn_db_delete_like_cpp(
    map_kind: wow_map::ManagedMapKind,
    map_id: u32,
    instance_id: u32,
    object_type: wow_map::SpawnObjectType,
    spawn_id: wow_map::SpawnId,
) -> RespawnDbDeleteQueueOutcomeLikeCpp {
    if !matches!(map_kind, wow_map::ManagedMapKind::World) {
        return RespawnDbDeleteQueueOutcomeLikeCpp::SkippedNonWorldMap;
    }

    let Ok(map_id) = u16::try_from(map_id) else {
        return RespawnDbDeleteQueueOutcomeLikeCpp::SkippedInvalidMapId;
    };

    let mut statement = PreparedStatement::new(CharStatements::DEL_RESPAWN.sql());
    statement.set_u16(0, u16::from(object_type as u8));
    statement.set_u64(1, spawn_id);
    statement.set_u16(2, map_id);
    statement.set_u32(3, instance_id);
    RespawnDbDeleteQueueOutcomeLikeCpp::Queued(RespawnDbDeleteLikeCpp {
        object_type,
        spawn_id,
        map_id,
        instance_id,
        statement,
    })
}

fn queue_respawn_db_save_like_cpp(
    map_kind: wow_map::ManagedMapKind,
    map_id: u32,
    instance_id: u32,
    info: wow_map::RespawnInfoLikeCpp,
) -> RespawnDbSaveQueueOutcomeLikeCpp {
    if !matches!(map_kind, wow_map::ManagedMapKind::World) {
        return RespawnDbSaveQueueOutcomeLikeCpp::SkippedNonWorldMap;
    }

    let Ok(map_id) = u16::try_from(map_id) else {
        return RespawnDbSaveQueueOutcomeLikeCpp::SkippedInvalidMapId;
    };

    let mut statement = PreparedStatement::new(CharStatements::REP_RESPAWN.sql());
    statement.set_u16(0, u16::from(info.object_type as u8));
    statement.set_u64(1, info.spawn_id);
    statement.set_i64(2, info.respawn_time);
    statement.set_u16(3, map_id);
    statement.set_u32(4, instance_id);
    RespawnDbSaveQueueOutcomeLikeCpp::Queued(RespawnDbSaveLikeCpp {
        object_type: info.object_type,
        spawn_id: info.spawn_id,
        respawn_time: info.respawn_time,
        map_id,
        instance_id,
        statement,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GameEventWorldEventStateDbStatementKindLikeCpp {
    DelGameEventSave,
    InsGameEventSave,
    DelAllGameEventConditionSave,
}

#[derive(Debug, Clone)]
struct GameEventWorldEventStateDbStatementLikeCpp {
    kind: GameEventWorldEventStateDbStatementKindLikeCpp,
    statement: PreparedStatement,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GameEventWorldEventStateDbOperationKindLikeCpp {
    Save,
    Delete,
}

#[derive(Debug, Clone)]
struct GameEventWorldEventStateDbOperationLikeCpp {
    event_id: u8,
    kind: GameEventWorldEventStateDbOperationKindLikeCpp,
    statements: Vec<GameEventWorldEventStateDbStatementLikeCpp>,
}

#[derive(Debug, Default, Clone)]
struct GameEventWorldEventStateDbBridgeSummaryLikeCpp {
    saves_queued: usize,
    saves_executed: usize,
    saves_failed: usize,
    saves_skipped_event_id_out_of_range: usize,
    saves_skipped_missing_event: usize,
    deletes_queued: usize,
    deletes_executed: usize,
    deletes_failed: usize,
    deletes_skipped_event_id_out_of_range: usize,
    condition_delete_rows_queued: usize,
    condition_delete_rows_executed: usize,
    condition_delete_rows_failed: usize,
    operations: Vec<GameEventWorldEventStateDbOperationLikeCpp>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GameEventQuestCompleteConditionSaveDbStatementKindLikeCpp {
    DelGameEventConditionSave,
    InsGameEventConditionSave,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
struct GameEventQuestCompleteConditionSaveDbStatementLikeCpp {
    kind: GameEventQuestCompleteConditionSaveDbStatementKindLikeCpp,
    statement: PreparedStatement,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
struct GameEventQuestCompleteConditionSaveDbOperationLikeCpp {
    event_id: u8,
    condition_id: u32,
    statements: Vec<GameEventQuestCompleteConditionSaveDbStatementLikeCpp>,
}

#[allow(dead_code)]
#[derive(Debug, Default, Clone)]
struct GameEventQuestCompleteDbBridgeSummaryLikeCpp {
    condition_save_updates_queued: usize,
    condition_save_updates_executed: usize,
    condition_save_updates_failed: usize,
    condition_save_updates_skipped_non_progress: usize,
    world_event_state_save_requested: usize,
    force_game_event_update_requested: usize,
    save_world_event_state_requested: bool,
    force_game_event_update_requested_flag: bool,
    world_event_state_summary: GameEventWorldEventStateDbBridgeSummaryLikeCpp,
    operations: Vec<GameEventQuestCompleteConditionSaveDbOperationLikeCpp>,
}

#[allow(dead_code)]
fn materialize_game_event_quest_complete_db_bridge_like_cpp(
    outcome: &spawn_store_loader::GameEventQuestCompleteOutcomeLikeCpp,
    metadata: &spawn_store_loader::CanonicalSpawnMetadataLikeCpp,
) -> GameEventQuestCompleteDbBridgeSummaryLikeCpp {
    let mut summary = GameEventQuestCompleteDbBridgeSummaryLikeCpp::default();
    let spawn_store_loader::GameEventQuestCompleteOutcomeLikeCpp::Progress(
        spawn_store_loader::GameEventConditionProgressOutcomeLikeCpp::Progressed(progress),
    ) = outcome
    else {
        summary.condition_save_updates_skipped_non_progress += 1;
        return summary;
    };

    if progress.save_world_event_state_requested {
        summary.world_event_state_save_requested += 1;
        summary.save_world_event_state_requested = true;
    }
    if progress.force_game_event_update_requested {
        summary.force_game_event_update_requested += 1;
        summary.force_game_event_update_requested_flag = true;
    }

    let mut delete = PreparedStatement::new(progress.del_statement.statement.sql());
    delete.set_u8(0, progress.del_statement.event_id);
    delete.set_u32(1, progress.del_statement.condition_id);

    let mut insert = PreparedStatement::new(progress.ins_statement.statement.sql());
    insert.set_u8(0, progress.ins_statement.event_id);
    insert.set_u32(1, progress.ins_statement.condition_id);
    let done_after = match progress.ins_statement.done {
        Some(done) => done,
        None => progress.done_after,
    };
    insert.set_f32(2, done_after);

    summary.condition_save_updates_queued += 1;
    summary
        .operations
        .push(GameEventQuestCompleteConditionSaveDbOperationLikeCpp {
            event_id: progress.del_statement.event_id,
            condition_id: progress.del_statement.condition_id,
            statements: vec![
                GameEventQuestCompleteConditionSaveDbStatementLikeCpp {
                    kind: GameEventQuestCompleteConditionSaveDbStatementKindLikeCpp::DelGameEventConditionSave,
                    statement: delete,
                },
                GameEventQuestCompleteConditionSaveDbStatementLikeCpp {
                    kind: GameEventQuestCompleteConditionSaveDbStatementKindLikeCpp::InsGameEventConditionSave,
                    statement: insert,
                },
            ],
        });

    if progress.save_world_event_state_requested {
        game_event_world_event_state_db_save_operation_like_cpp(
            progress.event_id,
            metadata,
            &mut summary.world_event_state_summary,
        );
    }

    summary
}

#[allow(dead_code)]
async fn execute_game_event_quest_complete_condition_save_db_bridge_like_cpp(
    character_db: &CharacterDatabase,
    summary: &mut GameEventQuestCompleteDbBridgeSummaryLikeCpp,
) {
    let operation_total = summary.operations.len();
    for (operation_index, operation) in summary.operations.drain(..).enumerate() {
        let mut transaction = SqlTransaction::new();
        for statement in operation.statements.iter().cloned() {
            transaction.append(statement.statement);
        }
        match transaction.commit(character_db.pool()).await {
            Ok(()) => summary.condition_save_updates_executed += 1,
            Err(error) => {
                summary.condition_save_updates_failed += 1;
                tracing::error!(
                    error = %error,
                    operation_index = operation_index + 1,
                    operation_total,
                    event_id = operation.event_id,
                    condition_id = operation.condition_id,
                    "Failed to execute C++ GameEventMgr quest-complete condition-save DB transaction; continuing live update loop"
                );
            }
        }
    }
}

fn game_event_world_event_state_db_save_operation_like_cpp(
    event_id: u16,
    metadata: &spawn_store_loader::CanonicalSpawnMetadataLikeCpp,
    summary: &mut GameEventWorldEventStateDbBridgeSummaryLikeCpp,
) {
    let Ok(event_id_u8) = u8::try_from(event_id) else {
        summary.saves_skipped_event_id_out_of_range += 1;
        return;
    };
    let Some(event) = metadata.game_event_like_cpp(event_id) else {
        summary.saves_skipped_missing_event += 1;
        return;
    };
    let Ok(next_start) = i64::try_from(event.next_start) else {
        summary.saves_skipped_missing_event += 1;
        return;
    };

    let mut delete = PreparedStatement::new(CharStatements::DEL_GAME_EVENT_SAVE.sql());
    delete.set_u8(0, event_id_u8);
    let mut insert = PreparedStatement::new(CharStatements::INS_GAME_EVENT_SAVE.sql());
    insert.set_u8(0, event_id_u8);
    insert.set_u8(1, event.state_raw);
    insert.set_i64(2, next_start);

    summary.saves_queued += 1;
    summary
        .operations
        .push(GameEventWorldEventStateDbOperationLikeCpp {
            event_id: event_id_u8,
            kind: GameEventWorldEventStateDbOperationKindLikeCpp::Save,
            statements: vec![
                GameEventWorldEventStateDbStatementLikeCpp {
                    kind: GameEventWorldEventStateDbStatementKindLikeCpp::DelGameEventSave,
                    statement: delete,
                },
                GameEventWorldEventStateDbStatementLikeCpp {
                    kind: GameEventWorldEventStateDbStatementKindLikeCpp::InsGameEventSave,
                    statement: insert,
                },
            ],
        });
}

fn game_event_world_event_state_db_delete_operation_like_cpp(
    event_id: u16,
    delete_condition_saves_requested: bool,
    delete_world_event_state_requested: bool,
    summary: &mut GameEventWorldEventStateDbBridgeSummaryLikeCpp,
) {
    if !delete_condition_saves_requested && !delete_world_event_state_requested {
        return;
    }
    let Ok(event_id_u8) = u8::try_from(event_id) else {
        summary.deletes_skipped_event_id_out_of_range += 1;
        return;
    };

    let mut statements = Vec::new();
    if delete_condition_saves_requested {
        let mut delete_conditions =
            PreparedStatement::new(CharStatements::DEL_ALL_GAME_EVENT_CONDITION_SAVE.sql());
        delete_conditions.set_u8(0, event_id_u8);
        statements.push(GameEventWorldEventStateDbStatementLikeCpp {
            kind: GameEventWorldEventStateDbStatementKindLikeCpp::DelAllGameEventConditionSave,
            statement: delete_conditions,
        });
        summary.condition_delete_rows_queued += 1;
    }
    if delete_world_event_state_requested {
        let mut delete_save = PreparedStatement::new(CharStatements::DEL_GAME_EVENT_SAVE.sql());
        delete_save.set_u8(0, event_id_u8);
        statements.push(GameEventWorldEventStateDbStatementLikeCpp {
            kind: GameEventWorldEventStateDbStatementKindLikeCpp::DelGameEventSave,
            statement: delete_save,
        });
        summary.deletes_queued += 1;
    }

    summary
        .operations
        .push(GameEventWorldEventStateDbOperationLikeCpp {
            event_id: event_id_u8,
            kind: GameEventWorldEventStateDbOperationKindLikeCpp::Delete,
            statements,
        });
}

fn materialize_game_event_world_event_state_db_bridge_like_cpp(
    outcome: &spawn_store_loader::GameEventUpdateOutcomeLikeCpp,
    metadata: &spawn_store_loader::CanonicalSpawnMetadataLikeCpp,
) -> GameEventWorldEventStateDbBridgeSummaryLikeCpp {
    let mut summary = GameEventWorldEventStateDbBridgeSummaryLikeCpp::default();

    for save in &outcome.world_nextphase_finished {
        if save.save_state_requested {
            game_event_world_event_state_db_save_operation_like_cpp(
                save.event_id,
                metadata,
                &mut summary,
            );
        }
    }
    for save in &outcome.world_conditions_save_requested {
        game_event_world_event_state_db_save_operation_like_cpp(
            save.event_id,
            metadata,
            &mut summary,
        );
    }
    for start_outcome in &outcome.start_outcomes {
        if let spawn_store_loader::GameEventStartOutcomeLikeCpp::Started(start) = start_outcome {
            if start.save_world_event_state_requested {
                game_event_world_event_state_db_save_operation_like_cpp(
                    start.event_id,
                    metadata,
                    &mut summary,
                );
            }
        }
    }
    for stop_outcome in &outcome.stop_outcomes {
        if let spawn_store_loader::GameEventStopOutcomeLikeCpp::Stopped(stop) = stop_outcome {
            game_event_world_event_state_db_delete_operation_like_cpp(
                stop.event_id,
                stop.delete_condition_saves_requested,
                stop.delete_world_event_state_requested,
                &mut summary,
            );
        }
    }

    summary
}

async fn execute_game_event_world_event_state_db_bridge_like_cpp(
    character_db: &CharacterDatabase,
    summary: &mut GameEventWorldEventStateDbBridgeSummaryLikeCpp,
) {
    let operation_total = summary.operations.len();
    for (operation_index, operation) in summary.operations.drain(..).enumerate() {
        let mut transaction = SqlTransaction::new();
        for statement in operation.statements.iter().cloned() {
            transaction.append(statement.statement);
        }
        match transaction.commit(character_db.pool()).await {
            Ok(()) => match operation.kind {
                GameEventWorldEventStateDbOperationKindLikeCpp::Save => summary.saves_executed += 1,
                GameEventWorldEventStateDbOperationKindLikeCpp::Delete => {
                    if operation.statements.iter().any(|statement| {
                        statement.kind
                            == GameEventWorldEventStateDbStatementKindLikeCpp::DelGameEventSave
                    }) {
                        summary.deletes_executed += 1;
                    }
                    if operation.statements.iter().any(|statement| {
                        statement.kind
                            == GameEventWorldEventStateDbStatementKindLikeCpp::DelAllGameEventConditionSave
                    }) {
                        summary.condition_delete_rows_executed += 1;
                    }
                }
            },
            Err(error) => {
                match operation.kind {
                    GameEventWorldEventStateDbOperationKindLikeCpp::Save => {
                        summary.saves_failed += 1;
                    }
                    GameEventWorldEventStateDbOperationKindLikeCpp::Delete => {
                        if operation.statements.iter().any(|statement| {
                            statement.kind
                                == GameEventWorldEventStateDbStatementKindLikeCpp::DelGameEventSave
                        }) {
                            summary.deletes_failed += 1;
                        }
                        if operation.statements.iter().any(|statement| {
                            statement.kind
                                == GameEventWorldEventStateDbStatementKindLikeCpp::DelAllGameEventConditionSave
                        }) {
                            summary.condition_delete_rows_failed += 1;
                        }
                    }
                }
                tracing::error!(
                    error = %error,
                    operation_index = operation_index + 1,
                    operation_total,
                    event_id = operation.event_id,
                    operation_kind = ?operation.kind,
                    "Failed to execute C++ GameEventMgr world-event state DB transaction; continuing live update loop"
                );
            }
        }
    }
}

#[derive(Clone)]
struct LoadedGridCreatureRespawnCachesLikeCpp {
    template_store: Arc<wow_data::CreatureTemplateLifecycleStoreLikeCpp>,
    difficulty_store: Arc<wow_data::CreatureDifficultyStoreLikeCpp>,
    base_stats_store: Arc<wow_data::CreatureBaseStatsStoreLikeCpp>,
    health_rates: wow_data::CreatureClassificationHealthRatesLikeCpp,
    display_store: Arc<wow_data::CreatureDisplayInfoStore>,
    model_store: Arc<wow_data::CreatureModelDataStore>,
    vehicle_store: Arc<wow_data::VehicleStore>,
    vehicle_seat_store: Arc<wow_data::VehicleSeatStore>,
    vehicle_accessory_store: Arc<wow_data::VehicleAccessoryStoreLikeCpp>,
    gameobject_template_store: Arc<wow_data::GameObjectTemplateLifecycleStoreLikeCpp>,
    gameobject_override_store: Arc<wow_data::GameObjectOverrideLifecycleStoreLikeCpp>,
}

#[derive(Debug, Default, Clone)]
struct CanonicalSpawnGroupConditionTickSummaryLikeCpp {
    maps_evaluated: usize,
    outcomes: usize,
    applied_set_inactive: usize,
    planned_spawn: usize,
    condition_spawn_executed_loaded_grid_spawns: usize,
    condition_spawn_blocked_loaded_grid_spawn_loads: usize,
    condition_spawn_blocked_loaded_grid_creature_loads: usize,
    condition_spawn_blocked_loaded_grid_gameobject_loads: usize,
    condition_spawn_blocked_loaded_grid_spawn_add_to_map: usize,
    condition_spawn_load_plan_count: usize,
    condition_spawn_unsupported_spawn_types: usize,
    condition_spawn_skipped_respawn_timer_active: usize,
    condition_spawn_skipped_live_object_active: usize,
    condition_spawn_skipped_unloaded_grid: usize,
    condition_spawn_skipped_difficulty_mismatch: usize,
    planned_despawn: usize,
    despawn_executed: usize,
    despawn_objects_removed: usize,
    despawn_respawn_timers_removed: usize,
    despawn_blocked_missing_group: usize,
    despawn_blocked_system_group: usize,
    despawn_unsupported_live_types: usize,
    despawn_respawn_timer_unsupported_types: usize,
    despawn_stale_index_entries: usize,
    despawn_remove_errors: usize,
    respawn_deleted_inactive_spawn_group: usize,
    respawn_deleted_live_object_blocker: usize,
    respawn_processed_pool_timers: usize,
    respawn_processed_unloaded_grid_respawns: usize,
    respawn_executed_loaded_grid_respawns: usize,
    respawn_blocked_loaded_grid_respawn_loads: usize,
    respawn_blocked_loaded_grid_respawn_add_to_map: usize,
    respawn_pool_update_plans: usize,
    respawn_blocked_pool_plan_errors: usize,
    respawn_blocked_missing_spawn_data: usize,
    respawn_blocked_pool_runtime: usize,
    respawn_blocked_do_respawn_runtime: usize,
    respawn_blocked_linked_respawn_non_future: usize,
    respawn_blocked_unsupported_spawn_type: usize,
    respawn_db_delete_queued: usize,
    respawn_db_delete_executed: usize,
    respawn_db_delete_failed: usize,
    respawn_db_delete_skipped_non_world_map: usize,
    respawn_db_delete_skipped_invalid_map_id: usize,
    respawn_db_deletes: Vec<RespawnDbDeleteLikeCpp>,
    respawn_db_save_queued: usize,
    respawn_db_save_executed: usize,
    respawn_db_save_failed: usize,
    respawn_db_save_skipped_non_world_map: usize,
    respawn_db_save_skipped_invalid_map_id: usize,
    respawn_db_saves: Vec<RespawnDbSaveLikeCpp>,
}

fn build_loaded_grid_creature_respawn_record_like_cpp(
    map: &mut wow_map::Map,
    object_type: wow_map::SpawnObjectType,
    spawn_id: wow_map::SpawnId,
    canonical_spawn_metadata: &spawn_store_loader::CanonicalSpawnMetadataLikeCpp,
    caches: &LoadedGridCreatureRespawnCachesLikeCpp,
) -> Option<wow_map::map::LoadedGridRespawnRecordsLikeCpp> {
    let Some(respawn_time) = map
        .get_respawn_info_like_cpp(object_type, spawn_id)
        .map(|info| info.respawn_time)
    else {
        debug!(
            spawn_id,
            respawn_type = object_type as u8,
            "C++ loaded-grid Creature DoRespawn blocked: missing map-owned respawn timer before LoadFromDB"
        );
        return None;
    };
    build_loaded_grid_creature_record_with_respawn_time_like_cpp(
        map,
        object_type,
        spawn_id,
        canonical_spawn_metadata,
        caches,
        respawn_time,
    )
}

fn build_loaded_grid_creature_spawn_group_spawn_record_like_cpp(
    map: &mut wow_map::Map,
    object_type: wow_map::SpawnObjectType,
    spawn_id: wow_map::SpawnId,
    canonical_spawn_metadata: &spawn_store_loader::CanonicalSpawnMetadataLikeCpp,
    caches: &LoadedGridCreatureRespawnCachesLikeCpp,
) -> Option<wow_map::map::LoadedGridRespawnRecordsLikeCpp> {
    build_loaded_grid_creature_record_with_respawn_time_like_cpp(
        map,
        object_type,
        spawn_id,
        canonical_spawn_metadata,
        caches,
        0,
    )
}

fn build_loaded_grid_creature_record_with_respawn_time_like_cpp(
    map: &mut wow_map::Map,
    object_type: wow_map::SpawnObjectType,
    spawn_id: wow_map::SpawnId,
    canonical_spawn_metadata: &spawn_store_loader::CanonicalSpawnMetadataLikeCpp,
    caches: &LoadedGridCreatureRespawnCachesLikeCpp,
    respawn_time: i64,
) -> Option<wow_map::map::LoadedGridRespawnRecordsLikeCpp> {
    if object_type != wow_map::SpawnObjectType::Creature {
        return None;
    }

    let Some(spawn) = canonical_spawn_metadata
        .spawn_store()
        .spawn_data(object_type, spawn_id)
    else {
        debug!(
            respawn_type = object_type as u8,
            spawn_id, "C++ loaded-grid Creature DoRespawn blocked: missing canonical SpawnData"
        );
        return None;
    };
    let Some(runtime_row) = canonical_spawn_metadata.creature_runtime_row_like_cpp(spawn_id) else {
        debug!(
            spawn_id,
            entry = spawn.id,
            "C++ loaded-grid Creature DoRespawn blocked: missing DB-backed creature runtime row"
        );
        return None;
    };
    let Ok(map_id) = u16::try_from(map.map_id()) else {
        warn!(
            map_id = map.map_id(),
            spawn_id,
            entry = spawn.id,
            "C++ loaded-grid Creature DoRespawn blocked: map id does not fit ObjectGuid world-object map field"
        );
        return None;
    };
    let difficulty_id = map.spawn_mode();
    if caches
        .difficulty_store
        .get_like_cpp(spawn.id, difficulty_id)
        .is_none()
    {
        debug!(
            spawn_id,
            entry = spawn.id,
            difficulty_id,
            "C++ loaded-grid Creature DoRespawn blocked: missing real creature_template_difficulty row"
        );
        return None;
    }
    let inputs = creature_loaded_grid::build_loaded_grid_creature_inputs_from_db_like_cpp(
        spawn,
        runtime_row,
        caches.template_store.as_ref(),
        caches.difficulty_store.as_ref(),
        caches.base_stats_store.as_ref(),
        &caches.health_rates,
        caches.display_store.as_ref(),
        caches.model_store.as_ref(),
        difficulty_id,
        map.instance_id(),
        respawn_time,
        true,
        canonical_spawn_metadata
            .creature_formation_info_like_cpp(spawn_id)
            .copied(),
        |min_level, max_level| map.select_creature_level_like_cpp(min_level, max_level),
    );
    let (template, resolved_spawn, runtime_selection) = match inputs {
        Ok(inputs) => inputs,
        Err(error) => {
            debug!(
                ?error,
                spawn_id,
                entry = spawn.id,
                "C++ loaded-grid Creature DoRespawn blocked: failed to compose DB-backed LoadFromDB inputs"
            );
            return None;
        }
    };

    let low = match map.generate_low_guid_like_cpp(HighGuid::Creature) {
        Ok(low) => low,
        Err(error) => {
            debug!(
                ?error,
                spawn_id,
                entry = spawn.id,
                "C++ loaded-grid Creature DoRespawn blocked: map-owned Creature low-guid generation failed"
            );
            return None;
        }
    };
    let mut template = template;
    if let Some(vehicle_id) = template.vehicle_id {
        if let Some(vehicle_entry) = caches.vehicle_store.get(vehicle_id) {
            template.vehicle_kit_create_input = Some(wow_entities::VehicleKitCreateInputLikeCpp {
                vehicle_id,
                creature_entry: template.entry,
                loading: true,
                seat_defs: caches
                    .vehicle_seat_store
                    .seat_defs_for_vehicle_like_cpp(vehicle_entry),
            });
            template.add_to_world_vehicle_reset_context =
                Some(wow_entities::CreatureAddToWorldVehicleResetContextLikeCpp {
                    is_mechanical_creature: template.creature_type
                        == CREATURE_TYPE_MECHANICAL_LIKE_CPP,
                    is_world_boss: template.type_flags & CREATURE_TYPE_FLAG_BOSS_MOB_LIKE_CPP != 0,
                    accessories: caches
                        .vehicle_accessory_store
                        .accessories_for_vehicle_like_cpp(Some(spawn_id), template.entry)
                        .map(ToOwned::to_owned)
                        .unwrap_or_default(),
                });
        }
    }

    let map_object_high = if template.vehicle_id.is_some() {
        HighGuid::Vehicle
    } else {
        HighGuid::Creature
    };
    let map_object_guid =
        ObjectGuid::create_world_object(map_object_high, 0, 1, map_id, 1, template.entry, low);
    let resolver = creature_loaded_grid::CreatureLoadedGridLifecycleResolverLikeCpp::new(
        [template],
        [resolved_spawn],
        [(spawn.id, runtime_selection)],
    );
    match resolver.resolve_loaded_grid_creature_like_cpp(spawn_id, map_object_guid) {
        Ok(resolved) => resolved.map_object_record.map(|primary_record| {
            wow_map::map::LoadedGridRespawnRecordsLikeCpp::primary_only(primary_record)
        }),
        Err(error) => {
            debug!(
                ?error,
                spawn_id,
                entry = spawn.id,
                guid = ?map_object_guid,
                "C++ loaded-grid Creature DoRespawn blocked: resolver rejected loaded Creature record"
            );
            None
        }
    }
}

fn build_loaded_grid_gameobject_respawn_record_like_cpp(
    map: &mut wow_map::Map,
    object_type: wow_map::SpawnObjectType,
    spawn_id: wow_map::SpawnId,
    canonical_spawn_metadata: &spawn_store_loader::CanonicalSpawnMetadataLikeCpp,
    caches: &LoadedGridCreatureRespawnCachesLikeCpp,
) -> Option<wow_map::map::LoadedGridRespawnRecordsLikeCpp> {
    if object_type != wow_map::SpawnObjectType::GameObject {
        return None;
    }

    let Some(spawn) = canonical_spawn_metadata
        .spawn_store()
        .spawn_data(object_type, spawn_id)
    else {
        debug!(
            respawn_type = object_type as u8,
            spawn_id, "C++ loaded-grid GameObject DoRespawn blocked: missing canonical SpawnData"
        );
        return None;
    };
    let Some(runtime_row) = canonical_spawn_metadata.gameobject_runtime_row_like_cpp(spawn_id)
    else {
        debug!(
            spawn_id,
            entry = spawn.id,
            "C++ loaded-grid GameObject DoRespawn blocked: missing DB-backed gameobject runtime row"
        );
        return None;
    };
    // C++ `Map::ProcessRespawns` erases the due map-owned respawn timer before
    // `DoRespawn -> GameObject::LoadFromDB(addToMap=true)`. Therefore
    // `GetMap()->GetGORespawnTime(m_spawnId)` observes no timer and the newly
    // respawned object's effective `m_respawnTime` is 0.
    let inputs = gameobject_loaded_grid::build_loaded_grid_gameobject_inputs_from_db_like_cpp(
        spawn,
        runtime_row,
        caches.gameobject_template_store.as_ref(),
        caches.gameobject_override_store.as_ref(),
        map.instance_id(),
        0,
        true,
    );
    let (template, resolved_spawn) = match inputs {
        Ok(inputs) => inputs,
        Err(error) => {
            debug!(
                ?error,
                spawn_id,
                entry = spawn.id,
                "C++ loaded-grid GameObject DoRespawn blocked: failed to compose DB-backed LoadFromDB inputs"
            );
            return None;
        }
    };

    let map_object_guid = if template.go_type == wow_entities::GAMEOBJECT_TYPE_TRANSPORT {
        let low = match map.generate_low_guid_like_cpp(HighGuid::Transport) {
            Ok(low) => low,
            Err(error) => {
                debug!(
                    ?error,
                    spawn_id,
                    entry = spawn.id,
                    "C++ loaded-grid GameObject DoRespawn blocked: map-owned Transport low-guid generation failed"
                );
                return None;
            }
        };
        ObjectGuid::create_transport(HighGuid::Transport, low)
    } else {
        let Ok(map_id) = u16::try_from(map.map_id()) else {
            warn!(
                map_id = map.map_id(),
                spawn_id,
                entry = spawn.id,
                "C++ loaded-grid GameObject DoRespawn blocked: map id does not fit ObjectGuid world-object map field"
            );
            return None;
        };
        let low = match map.generate_low_guid_like_cpp(HighGuid::GameObject) {
            Ok(low) => low,
            Err(error) => {
                debug!(
                    ?error,
                    spawn_id,
                    entry = spawn.id,
                    "C++ loaded-grid GameObject DoRespawn blocked: map-owned GameObject low-guid generation failed"
                );
                return None;
            }
        };
        ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, map_id, 1, template.entry, low)
    };
    let mut linked_trap_guid = None;
    let mut resolver_templates = vec![template.clone()];
    let linked_entry = wow_entities::GameObjectTemplateData::new(template.go_type, template.data)
        .get_linked_gameobject_entry_like_cpp();
    if linked_entry != 0 && template.go_type != wow_entities::GAMEOBJECT_TYPE_TRANSPORT {
        if let Some(linked_template_record) = caches.gameobject_template_store.get(linked_entry) {
            let linked_template =
                match gameobject_loaded_grid::resolved_template_from_lifecycle_record_like_cpp(
                    linked_template_record,
                    None,
                ) {
                    Ok(linked_template)
                        if linked_template.go_type != wow_entities::GAMEOBJECT_TYPE_TRANSPORT =>
                    {
                        Some(linked_template)
                    }
                    Ok(_) => {
                        debug!(
                            spawn_id,
                            entry = spawn.id,
                            linked_entry,
                            "C++ loaded-grid GameObject linked trap skipped: linked transport template not represented by this seam"
                        );
                        None
                    }
                    Err(error) => {
                        debug!(
                            ?error,
                            spawn_id,
                            entry = spawn.id,
                            linked_entry,
                            "C++ loaded-grid GameObject linked trap skipped: linked template rejected"
                        );
                        None
                    }
                };
            if let Some(linked_template) = linked_template {
                let Ok(map_id) = u16::try_from(map.map_id()) else {
                    warn!(
                        map_id = map.map_id(),
                        spawn_id,
                        entry = spawn.id,
                        linked_entry,
                        "C++ loaded-grid GameObject linked trap skipped: map id does not fit ObjectGuid world-object map field"
                    );
                    let resolver =
                        gameobject_loaded_grid::GameObjectLoadedGridLifecycleResolverLikeCpp::new(
                            resolver_templates,
                            [resolved_spawn],
                        );
                    return match resolver
                        .resolve_loaded_grid_gameobject_like_cpp(spawn_id, map_object_guid)
                    {
                        Ok(resolved) => resolved.map_object_record.map(|primary_record| {
                            wow_map::map::LoadedGridRespawnRecordsLikeCpp {
                                pre_add_records: resolved.pre_add_records,
                                primary_record,
                            }
                        }),
                        Err(error) => {
                            debug!(
                                ?error,
                                spawn_id,
                                entry = spawn.id,
                                guid = ?map_object_guid,
                                "C++ loaded-grid GameObject DoRespawn blocked: resolver rejected loaded GameObject record"
                            );
                            None
                        }
                    };
                };
                let trap_low = match map.generate_low_guid_like_cpp(HighGuid::GameObject) {
                    Ok(low) => Some(low),
                    Err(error) => {
                        debug!(
                            ?error,
                            spawn_id,
                            entry = spawn.id,
                            linked_entry,
                            "C++ loaded-grid GameObject linked trap skipped: map-owned GameObject low-guid generation failed"
                        );
                        None
                    }
                };
                if let Some(trap_low) = trap_low {
                    linked_trap_guid = Some(ObjectGuid::create_world_object(
                        HighGuid::GameObject,
                        0,
                        1,
                        map_id,
                        1,
                        linked_entry,
                        trap_low,
                    ));
                    resolver_templates.push(linked_template);
                }
            }
        } else {
            debug!(
                spawn_id,
                entry = spawn.id,
                linked_entry,
                "C++ loaded-grid GameObject linked trap skipped: missing linked trap template"
            );
        }
    }
    let resolver = gameobject_loaded_grid::GameObjectLoadedGridLifecycleResolverLikeCpp::new(
        resolver_templates,
        [resolved_spawn],
    );
    match resolver.resolve_loaded_grid_gameobject_with_linked_trap_like_cpp(
        spawn_id,
        map_object_guid,
        linked_trap_guid,
    ) {
        Ok(resolved) => resolved.map_object_record.map(|primary_record| {
            wow_map::map::LoadedGridRespawnRecordsLikeCpp {
                pre_add_records: resolved.pre_add_records,
                primary_record,
            }
        }),
        Err(error) => {
            debug!(
                ?error,
                spawn_id,
                entry = spawn.id,
                guid = ?map_object_guid,
                "C++ loaded-grid GameObject DoRespawn blocked: resolver rejected loaded GameObject record"
            );
            None
        }
    }
}

fn canonical_map_update_tick_set_inactive_like_cpp(
    manager: &mut wow_map::MapManager,
    diff_ms: u32,
    scheduler: &mut CanonicalRespawnConditionSchedulerLikeCpp,
    canonical_spawn_metadata: &spawn_store_loader::CanonicalSpawnMetadataLikeCpp,
    condition_store: &wow_data::ConditionEntriesByTypeStore,
    loaded_grid_creature_respawn_caches: &LoadedGridCreatureRespawnCachesLikeCpp,
) -> Option<CanonicalSpawnGroupConditionTickSummaryLikeCpp> {
    let Some(effective_diff_ms) = manager.update(diff_ms) else {
        return None;
    };
    if !scheduler.update(effective_diff_ms) {
        return None;
    }

    // C++ `Map::Update` runs `ProcessRespawns()` immediately before
    // `UpdateSpawnGroupConditions()` when `_respawnCheckTimer` expires.
    // This tick executes the safe in-memory ProcessRespawns side effects produced
    // by represented composite CheckRespawn guards: zero-delete for inactive
    // spawn-group/live-object blockers, linked-respawn future reschedules, pooled
    // timer UpdatePool plans, and the safe `DoRespawn` unloaded-grid early-return
    // branch after timer removal. DB delete/save effects are queued for async
    // execution after releasing the MapManager lock. Loaded-grid Creature
    // DB-backed loading is wired through the map-owned seam for supported
    // fixed-level and variable-level cases, including DB-backed FormationInfo
    // propagation into the bounded SearchFormation/AddCreatureToGroup seam;
    // AddToWorld ObjectAccessor/fanout, scripts/AI, vehicle runtime beyond local
    // evidence, zonescript, formation movement/combat/full CreatureGroup runtime,
    // dynamic-tree, GameObject DB-backed loading, AreaTrigger runtime and full
    // PoolMgr runtime remain gaps.
    // RustyCore does not yet expose CONFIG_RESPAWN_DYNAMIC_ESCORTNPC
    // or Creature::IsEscorted ownership here, so the bridge passes false/false.
    let now_secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| {
            i64::try_from(duration.as_secs()).unwrap_or(i64::MAX)
        });
    let mut summary = CanonicalSpawnGroupConditionTickSummaryLikeCpp::default();
    manager.do_for_all_maps_mut(|managed_map| {
        summary.maps_evaluated += 1;
        let map_kind = managed_map.kind();
        let map_id = managed_map.map_id();
        let instance_id = managed_map.instance_id();
        let before_respawn_keys = managed_map
            .map()
            .respawn_timer_keys_like_cpp()
            .collect::<BTreeSet<_>>();
        let respawn_summary = managed_map
            .map_mut()
            .process_due_respawns_composite_loaded_grid_respawns_like_cpp(
                now_secs,
                canonical_spawn_metadata.spawn_store(),
                canonical_spawn_metadata.linked_respawns_like_cpp(),
                canonical_spawn_metadata.pool_mgr_like_cpp(),
                5,
                false,
                |_, _| false,
                |_, _| 0.0,
                |_candidates, count| (0..count).collect(),
                |map, object_type, spawn_id| match object_type {
                    wow_map::SpawnObjectType::Creature => {
                        build_loaded_grid_creature_respawn_record_like_cpp(
                            map,
                            object_type,
                            spawn_id,
                            canonical_spawn_metadata,
                            loaded_grid_creature_respawn_caches,
                        )
                    }
                    wow_map::SpawnObjectType::GameObject => {
                        build_loaded_grid_gameobject_respawn_record_like_cpp(
                            map,
                            object_type,
                            spawn_id,
                            canonical_spawn_metadata,
                            loaded_grid_creature_respawn_caches,
                        )
                    }
                    wow_map::SpawnObjectType::AreaTrigger => None,
                },
            );
        summary.respawn_deleted_inactive_spawn_group +=
            respawn_summary.deleted_inactive_spawn_group;
        summary.respawn_deleted_live_object_blocker += respawn_summary.deleted_live_object_blocker;
        for rescheduled in respawn_summary.rescheduled_linked_respawns {
            match queue_respawn_db_save_like_cpp(map_kind, map_id, instance_id, rescheduled) {
                RespawnDbSaveQueueOutcomeLikeCpp::Queued(save) => {
                    summary.respawn_db_save_queued += 1;
                    summary.respawn_db_saves.push(save);
                }
                RespawnDbSaveQueueOutcomeLikeCpp::SkippedNonWorldMap => {
                    summary.respawn_db_save_skipped_non_world_map += 1;
                }
                RespawnDbSaveQueueOutcomeLikeCpp::SkippedInvalidMapId => {
                    summary.respawn_db_save_skipped_invalid_map_id += 1;
                }
            }
        }
        summary.respawn_processed_pool_timers += respawn_summary.processed_pool_timers;
        summary.respawn_processed_unloaded_grid_respawns +=
            respawn_summary.processed_unloaded_grid_respawns;
        summary.respawn_executed_loaded_grid_respawns +=
            respawn_summary.executed_loaded_grid_respawns;
        summary.respawn_blocked_loaded_grid_respawn_loads +=
            respawn_summary.blocked_loaded_grid_respawn_loads;
        summary.respawn_blocked_loaded_grid_respawn_add_to_map +=
            respawn_summary.blocked_loaded_grid_respawn_add_to_map;
        summary.respawn_pool_update_plans += respawn_summary.pool_update_plans.len();
        summary.respawn_blocked_pool_plan_errors += respawn_summary.blocked_pool_plan_errors.len();
        summary.respawn_blocked_missing_spawn_data += respawn_summary.blocked_missing_spawn_data;
        summary.respawn_blocked_pool_runtime += respawn_summary.blocked_pool_runtime;
        summary.respawn_blocked_do_respawn_runtime += respawn_summary.blocked_do_respawn_runtime;
        summary.respawn_blocked_linked_respawn_non_future +=
            respawn_summary.blocked_linked_respawn_non_future;
        summary.respawn_blocked_unsupported_spawn_type +=
            respawn_summary.blocked_unsupported_spawn_type;

        let outcomes = apply_canonical_spawn_group_condition_update_loaded_grid_records_like_cpp(
            managed_map,
            canonical_spawn_metadata,
            condition_store,
            loaded_grid_creature_respawn_caches,
        );
        summary.outcomes += outcomes.len();
        summary.applied_set_inactive += outcomes
            .iter()
            .filter(|outcome| outcome.applied_change.is_some())
            .count();
        summary.planned_spawn += outcomes
            .iter()
            .filter(|outcome| {
                matches!(
                    outcome.action,
                    wow_map::map::SpawnGroupConditionActionLikeCpp::Spawn { .. }
                )
            })
            .count();
        summary.planned_despawn += outcomes
            .iter()
            .filter(|outcome| {
                matches!(
                    outcome.action,
                    wow_map::map::SpawnGroupConditionActionLikeCpp::Despawn { .. }
                )
            })
            .count();
        for spawn in outcomes
            .iter()
            .filter_map(|outcome| outcome.spawn_outcome.as_ref())
        {
            summary.condition_spawn_executed_loaded_grid_spawns +=
                spawn.executed_loaded_grid_spawns;
            summary.condition_spawn_blocked_loaded_grid_spawn_loads +=
                spawn.blocked_loaded_grid_spawn_loads;
            summary.condition_spawn_blocked_loaded_grid_creature_loads +=
                spawn.blocked_loaded_grid_creature_loads;
            summary.condition_spawn_blocked_loaded_grid_gameobject_loads +=
                spawn.blocked_loaded_grid_gameobject_loads;
            summary.condition_spawn_blocked_loaded_grid_spawn_add_to_map +=
                spawn.blocked_loaded_grid_spawn_add_to_map;
            summary.condition_spawn_load_plan_count += spawn.load_plans.len();
            summary.condition_spawn_unsupported_spawn_types += spawn.unsupported_spawn_types;
            summary.condition_spawn_skipped_respawn_timer_active +=
                spawn.skipped_respawn_timer_active;
            summary.condition_spawn_skipped_live_object_active += spawn.skipped_live_object_active;
            summary.condition_spawn_skipped_unloaded_grid += spawn.skipped_unloaded_grid;
            summary.condition_spawn_skipped_difficulty_mismatch +=
                spawn.skipped_difficulty_mismatch;
        }
        for despawn in outcomes
            .iter()
            .filter_map(|outcome| outcome.despawn_outcome)
        {
            if despawn.blocked_missing_group == 0 && despawn.blocked_system_group == 0 {
                summary.despawn_executed += 1;
            }
            summary.despawn_objects_removed += despawn.objects_removed;
            summary.despawn_respawn_timers_removed += despawn.respawn_timers_removed;
            summary.despawn_blocked_missing_group += despawn.blocked_missing_group;
            summary.despawn_blocked_system_group += despawn.blocked_system_group;
            summary.despawn_unsupported_live_types += despawn.unsupported_live_despawn_types;
            summary.despawn_respawn_timer_unsupported_types +=
                despawn.respawn_timer_unsupported_types;
            summary.despawn_stale_index_entries += despawn.stale_index_entries;
            summary.despawn_remove_errors += despawn.remove_errors;
        }
        let after_respawn_keys = managed_map
            .map()
            .respawn_timer_keys_like_cpp()
            .collect::<BTreeSet<_>>();
        for &(object_type, spawn_id) in before_respawn_keys.difference(&after_respawn_keys) {
            match queue_respawn_db_delete_like_cpp(
                map_kind,
                map_id,
                instance_id,
                object_type,
                spawn_id,
            ) {
                RespawnDbDeleteQueueOutcomeLikeCpp::Queued(delete) => {
                    summary.respawn_db_delete_queued += 1;
                    summary.respawn_db_deletes.push(delete);
                }
                RespawnDbDeleteQueueOutcomeLikeCpp::SkippedNonWorldMap => {
                    summary.respawn_db_delete_skipped_non_world_map += 1;
                }
                RespawnDbDeleteQueueOutcomeLikeCpp::SkippedInvalidMapId => {
                    summary.respawn_db_delete_skipped_invalid_map_id += 1;
                }
            }
        }
    });

    Some(summary)
}

fn spawn_canonical_map_update_loop(
    map_manager: SharedCanonicalMapManager,
    tick_interval_ms: u32,
    respawn_condition_interval_ms: u32,
    canonical_spawn_metadata: SharedCanonicalSpawnMetadataLikeCpp,
    condition_store: Arc<wow_data::ConditionEntriesByTypeStore>,
    character_db: Arc<CharacterDatabase>,
    loaded_grid_creature_respawn_caches: LoadedGridCreatureRespawnCachesLikeCpp,
    mut game_event_scheduler: CanonicalGameEventSchedulerLikeCpp,
    player_registry: Arc<PlayerRegistry>,
    battlemaster_list_store: Arc<wow_data::BattlemasterListStore>,
    world_state_mgr: SharedWorldStateMgrLikeCpp,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut interval =
            tokio::time::interval(Duration::from_millis(u64::from(tick_interval_ms)));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

        let mut last_tick = Instant::now();
        let mut respawn_condition_scheduler =
            CanonicalRespawnConditionSchedulerLikeCpp::new(respawn_condition_interval_ms);
        loop {
            interval.tick().await;

            let now = Instant::now();
            let diff_ms = now
                .duration_since(last_tick)
                .as_millis()
                .min(u128::from(u32::MAX)) as u32;
            last_tick = now;

            if diff_ms == 0 {
                continue;
            }

            let tick_summary = {
                let Ok(mut manager) = map_manager.lock() else {
                    tracing::error!(
                        "Canonical MapManager mutex poisoned; stopping map update loop"
                    );
                    break;
                };
                let Ok(canonical_spawn_metadata) = canonical_spawn_metadata.lock() else {
                    tracing::error!(
                        "CanonicalSpawnMetadataLikeCpp mutex poisoned; stopping map update loop"
                    );
                    break;
                };
                canonical_map_update_tick_set_inactive_like_cpp(
                    &mut manager,
                    diff_ms,
                    &mut respawn_condition_scheduler,
                    &canonical_spawn_metadata,
                    condition_store.as_ref(),
                    &loaded_grid_creature_respawn_caches,
                )
            };

            if game_event_scheduler.update(diff_ms) {
                let current_time_secs = current_unix_time_secs_like_cpp();
                let (game_event_outcome, active_event_ids, mut db_bridge_summary) = {
                    let Ok(mut canonical_spawn_metadata) = canonical_spawn_metadata.lock() else {
                        tracing::error!(
                            "CanonicalSpawnMetadataLikeCpp mutex poisoned during GameEvent update; stopping map update loop"
                        );
                        break;
                    };
                    let outcome = canonical_spawn_metadata.update_game_events_like_cpp(
                        current_time_secs,
                        true,
                        represented_game_event_world_conditions_met_like_cpp,
                    );
                    game_event_scheduler.set_interval_and_reset(outcome.next_update_delay_millis);
                    let db_bridge_summary =
                        materialize_game_event_world_event_state_db_bridge_like_cpp(
                            &outcome,
                            &canonical_spawn_metadata,
                        );
                    let active_event_ids = canonical_spawn_metadata
                        .game_event_active_set_like_cpp()
                        .active_event_ids_like_cpp()
                        .collect::<Vec<_>>();
                    (outcome, active_event_ids, db_bridge_summary)
                };
                execute_game_event_world_event_state_db_bridge_like_cpp(
                    character_db.as_ref(),
                    &mut db_bridge_summary,
                )
                .await;
                let mut side_effect_summary = {
                    let Ok(mut manager) = map_manager.lock() else {
                        tracing::error!(
                            "Canonical MapManager mutex poisoned during GameEvent side effects; stopping map update loop"
                        );
                        break;
                    };
                    let Ok(mut canonical_spawn_metadata) = canonical_spawn_metadata.lock() else {
                        tracing::error!(
                            "CanonicalSpawnMetadataLikeCpp mutex poisoned during GameEvent side effects; stopping map update loop"
                        );
                        break;
                    };
                    let Ok(mut world_state_mgr) = world_state_mgr.lock() else {
                        tracing::error!(
                            "WorldStateMgrLikeCpp mutex poisoned during GameEvent side effects; stopping map update loop"
                        );
                        break;
                    };
                    consume_game_event_live_update_side_effects_like_cpp(
                        &mut manager,
                        &mut canonical_spawn_metadata,
                        &loaded_grid_creature_respawn_caches,
                        Some(battlemaster_list_store.as_ref()),
                        Some(&mut world_state_mgr),
                        Some(player_registry.as_ref()),
                        &active_event_ids,
                        &game_event_outcome,
                        false,
                    )
                };
                execute_game_event_seasonal_quest_db_deletes_like_cpp(
                    character_db.as_ref(),
                    &mut side_effect_summary,
                )
                .await;
                fanout_reset_event_seasonal_quests_to_player_sessions_after_db_delete_like_cpp(
                    Some(player_registry.as_ref()),
                    &mut side_effect_summary,
                );
                debug!(
                    scanned_event_ids = game_event_outcome.scanned_event_ids.len(),
                    queued_activation_event_ids =
                        game_event_outcome.queued_activation_event_ids.len(),
                    queued_deactivation_event_ids =
                        game_event_outcome.queued_deactivation_event_ids.len(),
                    start_outcomes = game_event_outcome.start_outcomes.len(),
                    stop_outcomes = game_event_outcome.stop_outcomes.len(),
                    negative_spawn_event_ids = game_event_outcome.negative_spawn_event_ids.len(),
                    world_nextphase_finished = game_event_outcome.world_nextphase_finished.len(),
                    world_conditions_save_requested =
                        game_event_outcome.world_conditions_save_requested.len(),
                    game_event_db_saves_queued = db_bridge_summary.saves_queued,
                    game_event_db_saves_executed = db_bridge_summary.saves_executed,
                    game_event_db_saves_failed = db_bridge_summary.saves_failed,
                    game_event_db_saves_skipped_event_id_out_of_range =
                        db_bridge_summary.saves_skipped_event_id_out_of_range,
                    game_event_db_saves_skipped_missing_event =
                        db_bridge_summary.saves_skipped_missing_event,
                    game_event_db_deletes_queued = db_bridge_summary.deletes_queued,
                    game_event_db_deletes_executed = db_bridge_summary.deletes_executed,
                    game_event_db_deletes_failed = db_bridge_summary.deletes_failed,
                    game_event_db_deletes_skipped_event_id_out_of_range =
                        db_bridge_summary.deletes_skipped_event_id_out_of_range,
                    game_event_db_condition_delete_rows_queued =
                        db_bridge_summary.condition_delete_rows_queued,
                    game_event_db_condition_delete_rows_executed =
                        db_bridge_summary.condition_delete_rows_executed,
                    game_event_db_condition_delete_rows_failed =
                        db_bridge_summary.condition_delete_rows_failed,
                    invalid_check_outcomes = game_event_outcome.invalid_check_outcomes.len(),
                    invalid_next_check_outcomes =
                        game_event_outcome.invalid_next_check_outcomes.len(),
                    next_update_delay_millis = game_event_outcome.next_update_delay_millis,
                    side_effect_actions = side_effect_summary.actions.len(),
                    spawn_actions = side_effect_summary.spawn_actions,
                    unspawn_actions = side_effect_summary.unspawn_actions,
                    announce_event_actions = side_effect_summary.announce_event_actions,
                    announce_event_description_len_total =
                        side_effect_summary.announce_event_description_len_total,
                    announce_event_world_text_unimplemented =
                        side_effect_summary.announce_event_world_text_unimplemented,
                    announce_event_session_fanout_unimplemented =
                        side_effect_summary.announce_event_session_fanout_unimplemented,
                    change_equip_or_model_actions =
                        side_effect_summary.change_equip_or_model_actions,
                    change_equip_or_model_records_seen =
                        side_effect_summary.change_equip_or_model_records_seen,
                    change_equip_or_model_records_applied =
                        side_effect_summary.change_equip_or_model_records_applied,
                    change_equip_or_model_maps_matched =
                        side_effect_summary.change_equip_or_model_maps_matched,
                    change_equip_or_model_live_creatures_mutated =
                        side_effect_summary.change_equip_or_model_live_creatures_mutated,
                    change_equip_or_model_model_validation_unavailable =
                        side_effect_summary.change_equip_or_model_model_validation_unavailable,
                    update_event_quests_actions = side_effect_summary.update_event_quests_actions,
                    update_event_quests_creature_records_seen =
                        side_effect_summary.update_event_quests_creature_records_seen,
                    update_event_quests_gameobject_records_seen =
                        side_effect_summary.update_event_quests_gameobject_records_seen,
                    update_event_quests_creature_inserted =
                        side_effect_summary.update_event_quests_creature_inserted,
                    update_event_quests_gameobject_inserted =
                        side_effect_summary.update_event_quests_gameobject_inserted,
                    update_event_quests_creature_removed =
                        side_effect_summary.update_event_quests_creature_removed,
                    update_event_quests_gameobject_removed =
                        side_effect_summary.update_event_quests_gameobject_removed,
                    update_event_quests_creature_remove_misses =
                        side_effect_summary.update_event_quests_creature_remove_misses,
                    update_event_quests_gameobject_remove_misses =
                        side_effect_summary.update_event_quests_gameobject_remove_misses,
                    update_event_quests_creature_skipped_active_other_event =
                        side_effect_summary.update_event_quests_creature_skipped_active_other_event,
                    update_event_quests_gameobject_skipped_active_other_event = side_effect_summary
                        .update_event_quests_gameobject_skipped_active_other_event,
                    update_world_states_actions = side_effect_summary.update_world_states_actions,
                    update_world_states_no_holiday =
                        side_effect_summary.update_world_states_no_holiday,
                    update_world_states_missing_event =
                        side_effect_summary.update_world_states_missing_event,
                    update_world_states_holiday_lookup_unrepresented =
                        side_effect_summary.update_world_states_holiday_lookup_unrepresented,
                    update_npc_flags_actions = side_effect_summary.update_npc_flags_actions,
                    update_npc_flags_records_seen =
                        side_effect_summary.update_npc_flags_records_seen,
                    update_npc_flags_maps_matched =
                        side_effect_summary.update_npc_flags_maps_matched,
                    update_npc_flags_live_creatures_mutated =
                        side_effect_summary.update_npc_flags_live_creatures_mutated,
                    update_npc_flags2_applied =
                        side_effect_summary.update_npc_flags2_applied,
                    update_npc_vendor_actions = side_effect_summary.update_npc_vendor_actions,
                    update_npc_vendor_records_seen =
                        side_effect_summary.update_npc_vendor_records_seen,
                    update_npc_vendor_items_added =
                        side_effect_summary.update_npc_vendor_items_added,
                    update_npc_vendor_items_removed =
                        side_effect_summary.update_npc_vendor_items_removed,
                    update_npc_vendor_missing_event_buckets =
                        side_effect_summary.update_npc_vendor_missing_event_buckets,
                    update_npc_vendor_remove_misses =
                        side_effect_summary.update_npc_vendor_remove_misses,
                    update_npc_vendor_no_match = side_effect_summary.update_npc_vendor_no_match,
                    reset_event_seasonal_quests_actions =
                        side_effect_summary.reset_event_seasonal_quests_actions,
                    reset_event_seasonal_quests_event_start_time_zero =
                        side_effect_summary.reset_event_seasonal_quests_event_start_time_zero,
                    reset_event_seasonal_quests_event_start_time_nonzero =
                        side_effect_summary.reset_event_seasonal_quests_event_start_time_nonzero,
                    reset_event_seasonal_quests_player_session_runtime_unimplemented =
                        side_effect_summary
                            .reset_event_seasonal_quests_player_session_runtime_unimplemented,
                    reset_event_seasonal_quests_character_db_statement_unimplemented =
                        side_effect_summary
                            .reset_event_seasonal_quests_character_db_statement_unimplemented,
                    reset_event_seasonal_quests_character_db_delete_queued = side_effect_summary
                        .reset_event_seasonal_quests_character_db_delete_queued,
                    reset_event_seasonal_quests_character_db_delete_executed = side_effect_summary
                        .reset_event_seasonal_quests_character_db_delete_executed,
                    reset_event_seasonal_quests_character_db_delete_failed = side_effect_summary
                        .reset_event_seasonal_quests_character_db_delete_failed,
                    reset_event_seasonal_quests_character_db_delete_skipped_event_start_time_out_of_range = side_effect_summary
                        .reset_event_seasonal_quests_character_db_delete_skipped_event_start_time_out_of_range,
                    "C++ WUPDATE_EVENTS represented timer fired; updated canonical GameEvent metadata and consumed represented GameEventSpawn/GameEventUnspawn plus bounded ChangeEquipOrModel, UpdateEventQuests cache, represented UpdateWorldStates HolidayWorldState -> WorldStateMgr::SetValue evidence, UpdateEventNPCFlags, UpdateEventNPCVendor cache, RunSmartAIScripts evidence, ResetEventSeasonalQuests character DB delete bridge, and represented announcement evidence-only side effects; ConditionMgr world-event rows, real SendWorldText/session fanout, quest packets/session gossip refresh, full ObjectMgr quest runtime, real WorldStateMgr storage/session fanout/login/GM worldstate, SmartAI script dispatch, and Player/session seasonal quest reset remain pending"
                );
            }

            if let Some(mut summary) = tick_summary {
                let db_delete_total = summary.respawn_db_deletes.len();
                for (db_delete_index, db_delete) in summary.respawn_db_deletes.drain(..).enumerate()
                {
                    match character_db.execute(&db_delete.statement).await {
                        Ok(_) => {
                            summary.respawn_db_delete_executed += 1;
                        }
                        Err(error) => {
                            summary.respawn_db_delete_failed += 1;
                            tracing::error!(
                                error = %error,
                                db_delete_index = db_delete_index + 1,
                                db_delete_total,
                                map_id = db_delete.map_id,
                                instance_id = db_delete.instance_id,
                                respawn_type = db_delete.object_type as u8,
                                spawn_id = db_delete.spawn_id,
                                "Failed to execute C++ Map::RemoveRespawnTime CHAR_DEL_RESPAWN side effect; continuing canonical map update loop"
                            );
                        }
                    }
                }
                let db_save_total = summary.respawn_db_saves.len();
                for (db_save_index, db_save) in summary.respawn_db_saves.drain(..).enumerate() {
                    match character_db.execute(&db_save.statement).await {
                        Ok(_) => {
                            summary.respawn_db_save_executed += 1;
                        }
                        Err(error) => {
                            summary.respawn_db_save_failed += 1;
                            tracing::error!(
                                error = %error,
                                db_save_index = db_save_index + 1,
                                db_save_total,
                                map_id = db_save.map_id,
                                instance_id = db_save.instance_id,
                                respawn_type = db_save.object_type as u8,
                                spawn_id = db_save.spawn_id,
                                respawn_time = db_save.respawn_time,
                                "Failed to execute C++ Map::SaveRespawnInfoDB CHAR_REP_RESPAWN side effect; continuing canonical map update loop"
                            );
                        }
                    }
                }
                debug!(
                    maps_evaluated = summary.maps_evaluated,
                    outcomes = summary.outcomes,
                    applied_set_inactive = summary.applied_set_inactive,
                    planned_spawn = summary.planned_spawn,
                    condition_spawn_executed_loaded_grid_spawns =
                        summary.condition_spawn_executed_loaded_grid_spawns,
                    condition_spawn_blocked_loaded_grid_spawn_loads =
                        summary.condition_spawn_blocked_loaded_grid_spawn_loads,
                    condition_spawn_blocked_loaded_grid_creature_loads =
                        summary.condition_spawn_blocked_loaded_grid_creature_loads,
                    condition_spawn_blocked_loaded_grid_gameobject_loads =
                        summary.condition_spawn_blocked_loaded_grid_gameobject_loads,
                    condition_spawn_blocked_loaded_grid_spawn_add_to_map =
                        summary.condition_spawn_blocked_loaded_grid_spawn_add_to_map,
                    condition_spawn_load_plan_count = summary.condition_spawn_load_plan_count,
                    condition_spawn_unsupported_spawn_types =
                        summary.condition_spawn_unsupported_spawn_types,
                    condition_spawn_skipped_respawn_timer_active =
                        summary.condition_spawn_skipped_respawn_timer_active,
                    condition_spawn_skipped_live_object_active =
                        summary.condition_spawn_skipped_live_object_active,
                    condition_spawn_skipped_unloaded_grid =
                        summary.condition_spawn_skipped_unloaded_grid,
                    condition_spawn_skipped_difficulty_mismatch =
                        summary.condition_spawn_skipped_difficulty_mismatch,
                    planned_despawn = summary.planned_despawn,
                    despawn_executed = summary.despawn_executed,
                    despawn_objects_removed = summary.despawn_objects_removed,
                    despawn_respawn_timers_removed = summary.despawn_respawn_timers_removed,
                    despawn_blocked_missing_group = summary.despawn_blocked_missing_group,
                    despawn_blocked_system_group = summary.despawn_blocked_system_group,
                    despawn_unsupported_live_types = summary.despawn_unsupported_live_types,
                    despawn_respawn_timer_unsupported_types =
                        summary.despawn_respawn_timer_unsupported_types,
                    despawn_stale_index_entries = summary.despawn_stale_index_entries,
                    despawn_remove_errors = summary.despawn_remove_errors,
                    respawn_deleted_inactive_spawn_group =
                        summary.respawn_deleted_inactive_spawn_group,
                    respawn_deleted_live_object_blocker =
                        summary.respawn_deleted_live_object_blocker,
                    respawn_processed_pool_timers = summary.respawn_processed_pool_timers,
                    respawn_processed_unloaded_grid_respawns =
                        summary.respawn_processed_unloaded_grid_respawns,
                    respawn_executed_loaded_grid_respawns =
                        summary.respawn_executed_loaded_grid_respawns,
                    respawn_blocked_loaded_grid_respawn_loads =
                        summary.respawn_blocked_loaded_grid_respawn_loads,
                    respawn_blocked_loaded_grid_respawn_add_to_map =
                        summary.respawn_blocked_loaded_grid_respawn_add_to_map,
                    respawn_pool_update_plans = summary.respawn_pool_update_plans,
                    respawn_blocked_pool_plan_errors = summary.respawn_blocked_pool_plan_errors,
                    respawn_blocked_missing_spawn_data = summary.respawn_blocked_missing_spawn_data,
                    respawn_blocked_pool_runtime = summary.respawn_blocked_pool_runtime,
                    respawn_blocked_do_respawn_runtime = summary.respawn_blocked_do_respawn_runtime,
                    respawn_blocked_linked_respawn_non_future =
                        summary.respawn_blocked_linked_respawn_non_future,
                    respawn_blocked_unsupported_spawn_type =
                        summary.respawn_blocked_unsupported_spawn_type,
                    respawn_db_delete_queued = summary.respawn_db_delete_queued,
                    respawn_db_delete_executed = summary.respawn_db_delete_executed,
                    respawn_db_delete_failed = summary.respawn_db_delete_failed,
                    respawn_db_delete_skipped_non_world_map =
                        summary.respawn_db_delete_skipped_non_world_map,
                    respawn_db_delete_skipped_invalid_map_id =
                        summary.respawn_db_delete_skipped_invalid_map_id,
                    respawn_db_save_queued = summary.respawn_db_save_queued,
                    respawn_db_save_executed = summary.respawn_db_save_executed,
                    respawn_db_save_failed = summary.respawn_db_save_failed,
                    respawn_db_save_skipped_non_world_map =
                        summary.respawn_db_save_skipped_non_world_map,
                    respawn_db_save_skipped_invalid_map_id =
                        summary.respawn_db_save_skipped_invalid_map_id,
                    "C++ respawn-check timer fired; executed safe ProcessRespawns composite zero-delete branches plus linked future reschedules, represented pooled timer UpdatePool plans, safe DoRespawn unloaded-grid early-return timer removals, map-local SpawnGroupDespawn condition-failure side effects, and bounded loaded-grid SpawnGroupSpawn condition loads; queued/executed DEL_RESPAWN/REP_RESPAWN DB side effects outside the MapManager lock; full SpawnGroupSpawn AreaTrigger/ObjectAccessor/fanout/scripts/AI and Spawn1Object/ReSpawn1Object runtime remain pending"
                );
            }
        }
    })
}

/// Load the realm's gamebuild from `realmlist` and the corresponding
/// Win64AuthSeed from `build_info`. Both are in the login database.
async fn load_realm_auth_seed(login_db: &LoginDatabase, realm_id: u16) -> Result<(u32, [u8; 16])> {
    // Query realmlist for the gamebuild
    let result = login_db
        .direct_query(&format!(
            "SELECT gamebuild FROM realmlist WHERE id = {realm_id}"
        ))
        .await
        .context("Failed to query realmlist")?;

    let build: u32 = if result.is_empty() {
        tracing::warn!("Realm {realm_id} not found in realmlist, using default build");
        51943
    } else {
        result.read(0)
    };

    // Query build_info for the Win64AuthSeed
    let seed_result = login_db
        .direct_query(&format!(
            "SELECT win64AuthSeed FROM build_info WHERE build = {build}"
        ))
        .await
        .context("Failed to query build_info")?;

    let seed_hex: String = if seed_result.is_empty() {
        anyhow::bail!("No build_info entry for build {build}");
    } else {
        seed_result.try_read(0).unwrap_or_default()
    };

    if seed_hex.len() != 32 {
        anyhow::bail!(
            "Invalid Win64AuthSeed for build {build}: expected 32 hex chars, got {}",
            seed_hex.len()
        );
    }

    // Parse hex string into 16 bytes
    let mut seed = [0u8; 16];
    for (i, byte) in seed.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&seed_hex[i * 2..i * 2 + 2], 16)
            .with_context(|| format!("Invalid hex in auth seed at position {i}"))?;
    }

    Ok((build, seed))
}

/// Parse an IPv4 address string into 4 bytes.
fn parse_ipv4(s: &str) -> Option<[u8; 4]> {
    let addr: std::net::Ipv4Addr = s.parse().ok()?;
    Some(addr.octets())
}

/// Create and run a WorldSession for an authenticated connection.
///
/// This is called by the accept loop after auth completes.
/// Runs the session update loop until the packet channel is closed.
async fn create_session(
    account: AccountInfo,
    pkt_rx: flume::Receiver<wow_packet::WorldPacket>,
    send_tx: flume::Sender<Vec<u8>>,
    resources: Arc<SessionResources>,
    session_mgr: Arc<SessionManager>,
    shared_map: SharedMapManager,
    canonical_map_manager: SharedCanonicalMapManager,
    object_accessor: wow_world::SharedObjectAccessor,
    instance_port: u16,
    max_expansion: u8,
    mmap_runtime_config: MMapRuntimeConfigLikeCpp,
    mmap_pathfinder: Option<Arc<WorldMMapPathfinderWorkerLikeCpp>>,
) {
    info!(
        "Creating session for account {} (bnet_id={})",
        account.id, account.battlenet_account_id
    );

    // Use the DERIVED 40-byte session key from realm auth handshake.
    // C# writes this to the DB (UPD_ACCOUNT_INFO_CONTINUED_SESSION) and the
    // instance socket reads it back. We skip the DB roundtrip by passing it directly.
    // NOTE: This is NOT the raw BNet key (64 bytes) from the DB. It's the
    // HMAC-SHA256 derived key used for AuthContinuedSession validation.
    let session_key_raw = account.derived_session_key.clone();

    // C# caps only ActiveExpansionLevel to the server's max expansion,
    // but sends AccountExpansionLevel as the raw DB value (e.g. 9=Dragonflight).
    // The client uses AccountExpansionLevel to unlock classes in the char list.
    let active_expansion = account.expansion.min(max_expansion);
    let account_expansion = account.expansion; // raw from DB, NOT capped

    let mut session = WorldSession::new(
        account.id,
        String::new(), // account_name
        account.security,
        active_expansion,
        account_expansion, // AccountExpansionLevel: raw from DB, like C#
        54261,             // build
        session_key_raw,
        account.locale.clone(),
        pkt_rx,
        send_tx,
    );

    // Configure session with resources
    if let Some(ref db) = resources.char_db {
        session.set_char_db(Arc::clone(db));
    }
    if let Some(ref db) = resources.login_db {
        session.set_login_db(Arc::clone(db));
    }
    session.set_battlenet_account_id(account.battlenet_account_id);
    session.set_recruiter_id_like_cpp(account.recruiter);
    if let Some(ref generator) = resources.guid_generator {
        session.set_guid_generator(Arc::clone(generator));
    }
    if let Some(ref mgr) = resources.instance_lock_mgr {
        session.set_instance_lock_mgr(Arc::clone(mgr));
    }
    if let Some(ref db) = resources.world_db {
        session.set_world_db(Arc::clone(db));
    }
    if let Some(ref store) = resources.currency_types_store {
        session.set_currency_types_store(Arc::clone(store));
    }
    if let Some(ref stores) = resources.import_price_stores {
        session.set_import_price_stores(Arc::clone(stores));
    }
    if let Some(ref store) = resources.item_class_store {
        session.set_item_class_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.item_currency_cost_store {
        session.set_item_currency_cost_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.item_extended_cost_store {
        session.set_item_extended_cost_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.item_store {
        session.set_item_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.item_appearance_store {
        session.set_item_appearance_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.item_modified_appearance_store {
        session.set_item_modified_appearance_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.item_price_base_store {
        session.set_item_price_base_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.item_limit_category_store {
        session.set_item_limit_category_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.item_limit_category_condition_store {
        session.set_item_limit_category_condition_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.player_stats {
        session.set_player_stats(Arc::clone(store));
    }
    if let Some(ref store) = resources.item_stats_store {
        session.set_item_stats_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.durability_costs_store {
        session.set_durability_costs_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.durability_quality_store {
        session.set_durability_quality_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.item_random_suffix_store {
        session.set_item_random_suffix_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.item_random_properties_store {
        session.set_item_random_properties_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.rand_prop_points_store {
        session.set_rand_prop_points_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.item_random_enchantment_template_store {
        session.set_item_random_enchantment_template_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.item_disenchant_loot_store {
        session.set_item_disenchant_loot_store(Arc::clone(store));
    }
    if let Some(ref stores) = resources.loot_stores {
        session.set_loot_stores(Arc::clone(stores));
    }
    if let Some(ref store) = resources.condition_store {
        session.set_condition_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.player_condition_store {
        session.set_player_condition_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.content_tuning_store {
        session.set_content_tuning_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.disable_mgr {
        session.set_disable_mgr(Arc::clone(store));
    }
    if let Some(ref store) = resources.lock_store {
        session.set_lock_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.spell_item_enchantment_store {
        session.set_spell_item_enchantment_store(Arc::clone(store));
    }
    if let Some(ref cache) = resources.hotfix_blob_cache {
        session.set_hotfix_blob_cache(Arc::clone(cache));
    }
    if let Some(ref store) = resources.skill_store {
        session.set_skill_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.skill_line_store {
        session.set_skill_line_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.spell_store {
        session.set_spell_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.npc_spell_click_store {
        session.set_npc_spell_click_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.spell_misc_store {
        session.set_spell_misc_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.spell_duration_store {
        session.set_spell_duration_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.spell_radius_store {
        session.set_spell_radius_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.spell_range_store {
        session.set_spell_range_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.area_table_store {
        session.set_area_table_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.fishing_base_skill_store {
        session.set_fishing_base_skill_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.area_trigger_store {
        session.set_area_trigger_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.chr_specialization_store {
        session.set_chr_specialization_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.dungeon_encounter_store {
        session.set_dungeon_encounter_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.map_store {
        session.set_map_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.map_difficulty_store {
        session.set_map_difficulty_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.map_difficulty_x_condition_store {
        session.set_map_difficulty_x_condition_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.lfg_dungeons_store {
        session.set_lfg_dungeons_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.creature_template_mount_store {
        session.set_creature_template_mount_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.creature_display_info_store {
        session.set_creature_display_info_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.gameobject_display_info_store {
        session.set_gameobject_display_info_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.creature_model_data_store {
        session.set_creature_model_data_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.mount_store {
        session.set_mount_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.mount_capability_store {
        session.set_mount_capability_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.mount_type_x_capability_store {
        session.set_mount_type_x_capability_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.mount_x_display_store {
        session.set_mount_x_display_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.vehicle_store {
        session.set_vehicle_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.vehicle_seat_store {
        session.set_vehicle_seat_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.vehicle_template_store {
        session.set_vehicle_template_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.vehicle_accessory_store {
        session.set_vehicle_accessory_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.terrain_swap_store {
        session.set_terrain_swap_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.phase_store {
        session.set_phase_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.phase_group_store {
        session.set_phase_group_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.quest_store {
        session.set_quest_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.quest_xp_store {
        session.set_quest_xp_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.quest_v2_store {
        session.set_quest_v2_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.quest_info_store {
        session.set_quest_info_store(Arc::clone(store));
    }
    session.set_quest_low_level_hide_diff_like_cpp(resources.quest_low_level_hide_diff);
    session.set_quest_high_level_hide_diff_like_cpp(resources.quest_high_level_hide_diff);
    if let Some(ref store) = resources.quest_package_item_store {
        session.set_quest_package_item_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.quest_faction_reward_store {
        session.set_quest_faction_reward_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.progression_faction_store {
        session.set_faction_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.faction_template_store {
        session.set_faction_template_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.friendship_rep_reaction_store {
        session.set_friendship_rep_reaction_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.paragon_reputation_store {
        session.set_paragon_reputation_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.reputation_reward_rate_store {
        session.set_reputation_reward_rate_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.creature_onkill_reputation_store {
        session.set_creature_onkill_reputation_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.reputation_spillover_template_store {
        session.set_reputation_spillover_template_store(Arc::clone(store));
    }
    if let Some(ref table) = resources.player_xp_table {
        session.set_player_xp_table(Arc::clone(table));
    }
    if let Some(ref registry) = resources.player_registry {
        session.set_player_registry(Arc::clone(registry));
    }
    if let Some(sender) = resources.game_event_quest_complete_tx.as_ref() {
        session.set_game_event_quest_complete_sender_like_cpp(sender.clone());
    }
    session.set_loot_drop_rates_like_cpp(resources.loot_drop_rates);
    session.set_reputation_rates_like_cpp(resources.reputation_rates);
    session.set_repair_cost_rate_like_cpp(resources.repair_cost_rate);
    session.set_enable_ae_loot_like_cpp(resources.enable_ae_loot);
    session.set_mmap_runtime_config_like_cpp(mmap_runtime_config);
    if let Some(pathfinder) = mmap_pathfinder {
        session.set_mmap_pathfinder_like_cpp(pathfinder);
    }
    session.set_object_accessor(object_accessor);
    if let (Some(greg), Some(pinv)) = (&resources.group_registry, &resources.pending_invites) {
        session.set_group_registry(Arc::clone(greg), Arc::clone(pinv));
    }
    session.set_realm_id(resources.realm_id);
    session.set_map_manager(shared_map);
    session.set_canonical_map_manager(canonical_map_manager);

    // Select the correct realm IP for ConnectTo based on client address.
    // C# logic: loopback → localAddress, otherwise → externalAddress.
    // For LAN clients, use localAddress if they're in the same subnet.
    let connect_ip = get_address_for_client(
        account.client_address,
        resources.realm_external_address,
        resources.realm_local_address,
    );

    // Configure ConnectTo flow — client needs an instance connection
    // for movement/interaction packets (UpdateObject, MoveSetActiveMover,
    // all movement opcodes use ConnectionType.Instance in C#).
    session.set_session_mgr(session_mgr);
    session.set_instance_endpoint(connect_ip, instance_port);

    // Send session init packets (AuthResponse + glue screen data).
    // These are the first encrypted packets the client receives.
    session.send_session_init_packets();

    info!("Session ready for account {}", account.id);

    // Session update loop
    loop {
        // Process incoming packets
        let count = session.update(50);

        // Dispatch pending packets (async handlers)
        session.process_pending().await;

        if session.is_disconnecting() {
            info!("Session for account {} disconnecting", account.id);
            break;
        }

        // Sleep to avoid busy-waiting (50ms tick)
        if count == 0 {
            tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
        }
    }
    session
        .cleanup_shared_runtime_state_on_disconnect_like_cpp()
        .await;
}

/// Load realm external and local addresses from the `realmlist` table.
///
/// C# stores these as `address` (external/public) and `localAddress` (LAN).
/// Returns `([external_ip], [local_ip])`.
async fn load_realm_addresses(
    login_db: &LoginDatabase,
    realm_id: u16,
) -> Result<([u8; 4], [u8; 4])> {
    let result = login_db
        .direct_query(&format!(
            "SELECT address, localAddress FROM realmlist WHERE id = {realm_id}"
        ))
        .await
        .context("Failed to query realmlist for addresses")?;

    if result.is_empty() {
        anyhow::bail!("Realm {realm_id} not found in realmlist table");
    }

    let external_str: String = result.read_string(0);
    let local_str: String = result.read_string(1);

    let external = parse_ipv4(&external_str).unwrap_or([127, 0, 0, 1]);
    let local = parse_ipv4(&local_str).unwrap_or([127, 0, 0, 1]);

    Ok((external, local))
}

/// Select the correct realm IP for a client, matching C#'s `GetAddressForClient`.
///
/// - Loopback client (127.0.0.1) → local address
/// - LAN client (same /24 subnet as local address) → local address
/// - Everything else → external (public) address
fn get_address_for_client(
    client_ip: Option<std::net::IpAddr>,
    external: [u8; 4],
    local: [u8; 4],
) -> [u8; 4] {
    let client = match client_ip {
        Some(std::net::IpAddr::V4(v4)) => v4.octets(),
        _ => return external, // unknown or IPv6 → external
    };

    // Loopback → local
    if client[0] == 127 {
        return local;
    }

    // Same /24 subnet as local address → local
    if client[0] == local[0] && client[1] == local[1] && client[2] == local[2] {
        return local;
    }

    external
}

/// Format an IPv4 address for display.
fn format_ipv4(ip: [u8; 4]) -> String {
    format!("{}.{}.{}.{}", ip[0], ip[1], ip[2], ip[3])
}

/// Decode a hex string into raw bytes.
fn hex_to_bytes(hex: &str) -> Vec<u8> {
    let hex = hex.trim();
    if hex.is_empty() {
        return Vec::new();
    }
    (0..hex.len())
        .step_by(2)
        .filter_map(|i| u8::from_str_radix(hex.get(i..i + 2)?, 16).ok())
        .collect()
}

/// Map DBC.Locale config value to the folder name.
///
/// The config can be a numeric ID (C# style) or already a locale name.
/// WoW locale IDs: 0=enUS, 1=koKR, 2=frFR, 3=deDE, 4=zhCN, 5=zhTW,
/// 6=esES, 7=esMX, 8=ruRU, 9=jaJP, 10=ptBR, 11=itIT.
fn locale_id_to_name(raw: &str) -> String {
    match raw.trim() {
        "0" => "enUS".into(),
        "1" => "koKR".into(),
        "2" => "frFR".into(),
        "3" => "deDE".into(),
        "4" => "zhCN".into(),
        "5" => "zhTW".into(),
        "6" => "esES".into(),
        "7" => "esMX".into(),
        "8" => "ruRU".into(),
        "9" => "jaJP".into(),
        "10" => "ptBR".into(),
        "11" => "itIT".into(),
        other => other.into(), // already a name like "esES"
    }
}

// ── Runtime candidate routing + delivery ────────────────────────────────────
//
// These functions started as dormant Slice 4A.1b infrastructure and are now
// reached only through the experimental `RustyCore.LegacyCreatureGlobalRuntime`
// loop, which is disabled by default.
// C++ anchors: `Object.cpp : WorldObject::SendMessageToSet` (~1746-1764),
// `GridNotifiersImpl.h : MessageDistDeliverer::Visit(PlayerMapType&)` (~43-46),
// `GridNotifiers.h : MessageDistDeliverer::SendPacket`.

/// Summary returned by [`deliver_runtime_plan_like_cpp`], testable without I/O.
#[derive(Debug, Default, PartialEq, Eq)]
struct RuntimeDeliverySummaryLikeCpp {
    /// Total `RuntimeEvent`s processed.
    pub events_seen: usize,
    /// Total candidate sessions evaluated across all events.
    pub candidates_seen: usize,
    /// Commands successfully enqueued (`try_send` succeeded).
    pub candidates_queued: usize,
    /// Candidates rejected because their map_id did not match the event.
    pub candidates_skipped_wrong_map: usize,
    /// Candidates rejected because their instance_id did not match the event.
    pub candidates_skipped_wrong_instance: usize,
    /// Candidates rejected because `is_in_world == false`.
    pub candidates_skipped_not_in_world: usize,
    /// Candidates rejected because they were out of distance range.
    pub candidates_skipped_distance: usize,
    /// `SelfOnly` events skipped (no broadcast; session delivers its own packets).
    pub self_only_skipped: usize,
    /// `try_send` calls that returned `Err` (channel full or disconnected).
    pub send_failed: usize,
}

/// Summary for map-wide creature-visibility refresh commands.
#[derive(Debug, Default, PartialEq, Eq)]
struct RuntimeVisibilityRefreshDeliverySummaryLikeCpp {
    /// Total candidate sessions evaluated.
    pub candidates_seen: usize,
    /// Commands successfully enqueued (`try_send` succeeded).
    pub candidates_queued: usize,
    /// Candidates rejected because their map_id did not match.
    pub candidates_skipped_wrong_map: usize,
    /// Candidates rejected because their instance_id did not match.
    pub candidates_skipped_wrong_instance: usize,
    /// Candidates rejected because `is_in_world == false`.
    pub candidates_skipped_not_in_world: usize,
    /// `try_send` calls that returned `Err` (channel full or disconnected).
    pub send_failed: usize,
}

/// Summary for explicit victim-session creature melee commands.
#[derive(Debug, Default, PartialEq, Eq)]
struct RuntimeCreatureMeleeDeliverySummaryLikeCpp {
    /// Total already-resolved creature melee commands processed.
    pub commands_seen: usize,
    /// Victim sessions found in [`PlayerRegistry`] and evaluated.
    pub candidates_seen: usize,
    /// Commands successfully enqueued (`try_send` succeeded).
    pub candidates_queued: usize,
    /// Commands whose victim player was not present in the registry.
    pub candidates_skipped_missing_victim: usize,
    /// Candidates rejected because their map_id did not match the command.
    pub candidates_skipped_wrong_map: usize,
    /// Candidates rejected because their instance_id did not match the command.
    pub candidates_skipped_wrong_instance: usize,
    /// Candidates rejected because `is_in_world == false`.
    pub candidates_skipped_not_in_world: usize,
    /// `try_send` calls that returned `Err` (channel full or disconnected).
    pub send_failed: usize,
}

/// Summary for explicit victim-session creature attack-start commands.
#[derive(Debug, Default, PartialEq, Eq)]
struct RuntimeCreatureAttackStartDeliverySummaryLikeCpp {
    /// Total already-resolved creature aggro commands processed.
    pub commands_seen: usize,
    /// Victim sessions found in [`PlayerRegistry`] and evaluated.
    pub candidates_seen: usize,
    /// Commands successfully enqueued (`try_send` succeeded).
    pub candidates_queued: usize,
    /// Commands whose victim player was not present in the registry.
    pub candidates_skipped_missing_victim: usize,
    /// Candidates rejected because their map_id did not match the command.
    pub candidates_skipped_wrong_map: usize,
    /// Candidates rejected because their instance_id did not match the command.
    pub candidates_skipped_wrong_instance: usize,
    /// Candidates rejected because `is_in_world == false`.
    pub candidates_skipped_not_in_world: usize,
    /// Candidates rejected because the victim was no longer alive at delivery time.
    pub candidates_skipped_dead: usize,
    /// `try_send` calls that returned `Err` (channel full or disconnected).
    pub send_failed: usize,
}

impl RuntimeVisibilityRefreshDeliverySummaryLikeCpp {
    fn merge(&mut self, other: Self) {
        self.candidates_seen += other.candidates_seen;
        self.candidates_queued += other.candidates_queued;
        self.candidates_skipped_wrong_map += other.candidates_skipped_wrong_map;
        self.candidates_skipped_wrong_instance += other.candidates_skipped_wrong_instance;
        self.candidates_skipped_not_in_world += other.candidates_skipped_not_in_world;
        self.send_failed += other.send_failed;
    }
}

/// Collect candidate sessions from `registry` for one `RuntimeEvent` and push
/// [`SessionCommand::SendIfVisibleLikeCpp`] commands via `try_send`.
///
/// Gates applied here (cheap, without session lock):
/// - `is_in_world` — mirrors C++ `Player::IsInWorld()`.
/// - `map_id` / `instance_id` — mirrors C++ `InSamePhase` + map check.
/// - Distance — 2D (`required_3d == false`) or 3D (`required_3d == true`),
///   mirroring `MessageDistDeliverer::Visit` range parameter.
///
/// The final HaveAtClient gate is applied per-session in
/// `handle_send_if_visible_like_cpp_command_like_cpp`.
///
/// **No guards are held during `try_send`**: candidates are collected into a
/// `Vec` first, then commands are sent outside the DashMap iteration.
fn resolve_runtime_event_candidates_like_cpp(
    event: &wow_world::map_manager::RuntimeEvent,
    registry: &wow_network::PlayerRegistry,
    summary: &mut RuntimeDeliverySummaryLikeCpp,
) {
    use wow_world::map_manager::RecipientRule;

    match &event.recipients {
        RecipientRule::SelfOnly => {
            // SelfOnly packets are delivered by the owning session directly
            // (e.g. flush_runtime_output).  Never broadcast globally — the
            // owner session is not identified here.  C++ analogy: self-send
            // path inside `WorldObject::SendMessageToSet` skips the
            // `MessageDistDeliverer` entirely for the source player.
            summary.self_only_skipped += 1;
            return;
        }
        RecipientRule::ExplicitPlayer(guid) => {
            // Send to exactly one player session — no distance or in_world filter.
            // Read map_id/instance_id from the registry entry so that the per-session
            // gate 2/3 (player_map_id_like_cpp / instance_id) accepts the command.
            // C++ analogy: SendDirectMessage / explicit-receiver path in
            // WorldObject::SendMessageToSet does NOT apply a map filter on the sender
            // side; the receiver is already known.  We mirror this by populating the
            // command with the *target* session's own map/instance so the gate passes.
            // If the guid is not in the registry we drop silently (session already gone).
            let candidate = registry
                .get(guid)
                .map(|entry| (entry.command_tx.clone(), entry.map_id, entry.instance_id));
            if let Some((tx, target_map_id, target_instance_id)) = candidate {
                summary.candidates_seen += 1;
                let cmd = wow_network::SessionCommand::SendIfVisibleLikeCpp(
                    wow_network::player_registry::SendIfVisibleLikeCppCommand {
                        source_guid: event.source_guid,
                        map_id: target_map_id,
                        instance_id: target_instance_id,
                        packet_bytes: event.packet_bytes.clone(),
                    },
                );
                if tx.try_send(cmd).is_ok() {
                    summary.candidates_queued += 1;
                } else {
                    summary.send_failed += 1;
                }
            }
        }
        RecipientRule::MapBroadcastVisible {
            map_id,
            instance_id,
        } => {
            // Single pass: classify each session once, then send outside the
            // DashMap iteration (no guards held during try_send).
            // Mirrors the NearbyVisible pattern above.
            struct Candidate {
                tx: flume::Sender<wow_network::SessionCommand>,
                skip_reason: Option<BroadcastSkipReason>,
            }
            enum BroadcastSkipReason {
                NotInWorld,
                WrongMap,
                WrongInstance,
            }
            let candidates: Vec<Candidate> = registry
                .iter()
                .map(|entry| {
                    let info = entry.value();
                    if !info.is_in_world {
                        return Candidate {
                            tx: info.command_tx.clone(),
                            skip_reason: Some(BroadcastSkipReason::NotInWorld),
                        };
                    }
                    if info.map_id != *map_id {
                        return Candidate {
                            tx: info.command_tx.clone(),
                            skip_reason: Some(BroadcastSkipReason::WrongMap),
                        };
                    }
                    if info.instance_id != *instance_id {
                        return Candidate {
                            tx: info.command_tx.clone(),
                            skip_reason: Some(BroadcastSkipReason::WrongInstance),
                        };
                    }
                    Candidate {
                        tx: info.command_tx.clone(),
                        skip_reason: None,
                    }
                })
                .collect();

            for candidate in candidates {
                summary.candidates_seen += 1;
                match candidate.skip_reason {
                    Some(BroadcastSkipReason::NotInWorld) => {
                        summary.candidates_skipped_not_in_world += 1;
                    }
                    Some(BroadcastSkipReason::WrongMap) => {
                        summary.candidates_skipped_wrong_map += 1;
                    }
                    Some(BroadcastSkipReason::WrongInstance) => {
                        summary.candidates_skipped_wrong_instance += 1;
                    }
                    None => {
                        let cmd = wow_network::SessionCommand::SendIfVisibleLikeCpp(
                            wow_network::player_registry::SendIfVisibleLikeCppCommand {
                                source_guid: event.source_guid,
                                map_id: *map_id,
                                instance_id: *instance_id,
                                packet_bytes: event.packet_bytes.clone(),
                            },
                        );
                        if candidate.tx.try_send(cmd).is_ok() {
                            summary.candidates_queued += 1;
                        } else {
                            summary.send_failed += 1;
                        }
                    }
                }
            }
        }
        RecipientRule::NearbyVisible {
            source_guid: _,
            map_id,
            instance_id,
            source_position,
            range,
            required_3d,
        } => {
            let range_sq = range * range;
            // Collect candidates first (avoid holding guards during try_send).
            struct Candidate {
                tx: flume::Sender<wow_network::SessionCommand>,
                skip_reason: Option<SkipReason>,
            }
            enum SkipReason {
                NotInWorld,
                WrongMap,
                WrongInstance,
                Distance,
            }
            let candidates: Vec<Candidate> = registry
                .iter()
                .map(|entry| {
                    let info = entry.value();
                    if !info.is_in_world {
                        return Candidate {
                            tx: info.command_tx.clone(),
                            skip_reason: Some(SkipReason::NotInWorld),
                        };
                    }
                    if info.map_id != *map_id {
                        return Candidate {
                            tx: info.command_tx.clone(),
                            skip_reason: Some(SkipReason::WrongMap),
                        };
                    }
                    if info.instance_id != *instance_id {
                        return Candidate {
                            tx: info.command_tx.clone(),
                            skip_reason: Some(SkipReason::WrongInstance),
                        };
                    }
                    let dist_sq = if *required_3d {
                        let dx = info.position.x - source_position.x;
                        let dy = info.position.y - source_position.y;
                        let dz = info.position.z - source_position.z;
                        dx * dx + dy * dy + dz * dz
                    } else {
                        let dx = info.position.x - source_position.x;
                        let dy = info.position.y - source_position.y;
                        dx * dx + dy * dy
                    };
                    if dist_sq > range_sq {
                        return Candidate {
                            tx: info.command_tx.clone(),
                            skip_reason: Some(SkipReason::Distance),
                        };
                    }
                    Candidate {
                        tx: info.command_tx.clone(),
                        skip_reason: None,
                    }
                })
                .collect();

            for candidate in candidates {
                summary.candidates_seen += 1;
                match candidate.skip_reason {
                    Some(SkipReason::NotInWorld) => {
                        summary.candidates_skipped_not_in_world += 1;
                    }
                    Some(SkipReason::WrongMap) => {
                        summary.candidates_skipped_wrong_map += 1;
                    }
                    Some(SkipReason::WrongInstance) => {
                        summary.candidates_skipped_wrong_instance += 1;
                    }
                    Some(SkipReason::Distance) => {
                        summary.candidates_skipped_distance += 1;
                    }
                    None => {
                        let cmd = wow_network::SessionCommand::SendIfVisibleLikeCpp(
                            wow_network::player_registry::SendIfVisibleLikeCppCommand {
                                source_guid: event.source_guid,
                                map_id: *map_id,
                                instance_id: *instance_id,
                                packet_bytes: event.packet_bytes.clone(),
                            },
                        );
                        if candidate.tx.try_send(cmd).is_ok() {
                            summary.candidates_queued += 1;
                        } else {
                            summary.send_failed += 1;
                        }
                    }
                }
            }
        }
    }
}

/// Route and deliver all events in `plan` to candidate sessions.
///
/// Returns a [`RuntimeDeliverySummaryLikeCpp`] for test assertions.
/// No blocking sends — backpressure via `try_send` only.
fn deliver_runtime_plan_like_cpp(
    plan: &wow_world::map_manager::RuntimePlan,
    registry: &wow_network::PlayerRegistry,
) -> RuntimeDeliverySummaryLikeCpp {
    let mut summary = RuntimeDeliverySummaryLikeCpp::default();
    for event in &plan.events {
        summary.events_seen += 1;
        resolve_runtime_event_candidates_like_cpp(event, registry, &mut summary);
    }
    summary
}

/// Ask all sessions on a map instance to recompute map-owned creature visibility.
///
/// This is dormant 4A.3c infrastructure for global create/destroy/respawn
/// delivery. C++ creates/destroys visibility through `Player::UpdateVisibilityOf`
/// (Player.cpp:23138+) rather than by sending a raw packet that bypasses
/// `m_clientGUIDs`. The session command mirrors that seam by forcing each
/// matching session to run its own visibility pass.
///
/// No map locks are held here; registry candidates are cloned before `try_send`.
fn deliver_refresh_visible_world_creatures_like_cpp(
    map_id: u16,
    instance_id: u32,
    registry: &wow_network::PlayerRegistry,
) -> RuntimeVisibilityRefreshDeliverySummaryLikeCpp {
    struct Candidate {
        tx: flume::Sender<wow_network::SessionCommand>,
        skip_reason: Option<RefreshSkipReason>,
    }
    enum RefreshSkipReason {
        NotInWorld,
        WrongMap,
        WrongInstance,
    }

    let candidates: Vec<Candidate> = registry
        .iter()
        .map(|entry| {
            let info = entry.value();
            if !info.is_in_world {
                return Candidate {
                    tx: info.command_tx.clone(),
                    skip_reason: Some(RefreshSkipReason::NotInWorld),
                };
            }
            if info.map_id != map_id {
                return Candidate {
                    tx: info.command_tx.clone(),
                    skip_reason: Some(RefreshSkipReason::WrongMap),
                };
            }
            if info.instance_id != instance_id {
                return Candidate {
                    tx: info.command_tx.clone(),
                    skip_reason: Some(RefreshSkipReason::WrongInstance),
                };
            }
            Candidate {
                tx: info.command_tx.clone(),
                skip_reason: None,
            }
        })
        .collect();

    let mut summary = RuntimeVisibilityRefreshDeliverySummaryLikeCpp::default();
    for candidate in candidates {
        summary.candidates_seen += 1;
        match candidate.skip_reason {
            Some(RefreshSkipReason::NotInWorld) => {
                summary.candidates_skipped_not_in_world += 1;
            }
            Some(RefreshSkipReason::WrongMap) => {
                summary.candidates_skipped_wrong_map += 1;
            }
            Some(RefreshSkipReason::WrongInstance) => {
                summary.candidates_skipped_wrong_instance += 1;
            }
            None => {
                let cmd = wow_network::SessionCommand::RefreshVisibleWorldCreaturesLikeCpp(
                    wow_network::player_registry::RefreshVisibleWorldCreaturesLikeCppCommand {
                        map_id,
                        instance_id,
                    },
                );
                if candidate.tx.try_send(cmd).is_ok() {
                    summary.candidates_queued += 1;
                } else {
                    summary.send_failed += 1;
                }
            }
        }
    }
    summary
}

/// Snapshot active player positions for the global creature aggro scan.
///
/// The scan itself is map-owned and runs in `wow-world`; this bridge only
/// collects cheap, copyable receiver state from [`PlayerRegistry`] and drops
/// DashMap guards before the legacy map lock is taken.
fn collect_legacy_creature_aggro_candidates_like_cpp(
    registry: &wow_network::PlayerRegistry,
) -> Vec<wow_world::session::LegacyCreatureAggroCandidateLikeCpp> {
    registry
        .iter()
        .filter_map(|entry| {
            let guid = *entry.key();
            let info = entry.value();
            (info.is_in_world && info.is_alive).then_some(
                wow_world::session::LegacyCreatureAggroCandidateLikeCpp {
                    player_guid: guid,
                    map_id: info.map_id,
                    instance_id: info.instance_id,
                    position: info.position,
                    player_combat_reach: info.combat_reach,
                    player_liquid_status_like_cpp: info.liquid_status,
                    player_level: info.level,
                    player_gray_level: info.gray_level,
                    player_unit_flags: info.unit_flags,
                    player_unit_flags2: info.unit_flags2,
                    player_unit_state: info.unit_state,
                    player_is_game_master: info.is_game_master,
                    player_is_contested_pvp: info.is_contested_pvp,
                    player_faction_template_id: info.faction_template_id,
                    player_reputation_standings: info.reputation_standings.clone(),
                    player_reputation_state_flags: info.reputation_state_flags.clone(),
                    player_forced_reputation_ranks: info.forced_reputation_ranks.clone(),
                    player_forced_reputation_faction_ids: info
                        .forced_reputation_faction_ids
                        .clone(),
                },
            )
        })
        .collect()
}

fn legacy_creature_aggro_config_like_cpp(
    configs: &WorldConfigSet,
) -> wow_world::session::LegacyCreatureAggroConfigLikeCpp {
    wow_world::session::LegacyCreatureAggroConfigLikeCpp {
        no_gray_aggro_above: world_config_u32(configs, "CONFIG_NO_GRAY_AGGRO_ABOVE", 0),
        no_gray_aggro_below: world_config_u32(configs, "CONFIG_NO_GRAY_AGGRO_BELOW", 0),
        visibility_distance_continents: legacy_visibility_distance_like_cpp(
            "Visibility.Distance.Continents",
            wow_entities::DEFAULT_VISIBILITY_DISTANCE,
        ),
        visibility_distance_instances: legacy_visibility_distance_like_cpp(
            "Visibility.Distance.Instances",
            wow_entities::DEFAULT_VISIBILITY_INSTANCE,
        ),
        visibility_distance_battlegrounds: legacy_visibility_distance_like_cpp(
            "Visibility.Distance.BG",
            533.0,
        ),
        visibility_distance_arenas: legacy_visibility_distance_like_cpp(
            "Visibility.Distance.Arenas",
            533.0,
        ),
        faction_template_store: None,
        faction_store: None,
        map_store: None,
    }
}

fn legacy_visibility_distance_like_cpp(key: &str, default: f32) -> f32 {
    let configured = wow_config::get_value_default::<f32>(key, default);
    let min = 45.0 * wow_config::get_value_default::<f32>("Rate.Creature.Aggro", 1.0);
    if configured < min {
        min
    } else if configured > wow_entities::MAX_VISIBILITY_DISTANCE {
        wow_entities::MAX_VISIBILITY_DISTANCE
    } else {
        configured
    }
}

/// Deliver map-owned creature aggro starts to their exact victim sessions.
///
/// C++ contrast: `CreatureAI::MoveInLineOfSight`/`Creature::CanStartAttack`
/// decides the engagement from map state; `Unit::SendMeleeAttackStart` sends
/// the visible combat-start packet. This helper routes that already-resolved
/// engagement to the victim session using `try_send`, outside all map locks.
fn deliver_creature_attack_start_commands_like_cpp(
    commands: &[wow_network::player_registry::CreatureAttackStartLikeCppCommand],
    registry: &wow_network::PlayerRegistry,
) -> RuntimeCreatureAttackStartDeliverySummaryLikeCpp {
    struct Candidate {
        tx: flume::Sender<wow_network::SessionCommand>,
        map_id: u16,
        instance_id: u32,
        is_in_world: bool,
        is_alive: bool,
    }

    let mut summary = RuntimeCreatureAttackStartDeliverySummaryLikeCpp::default();
    for command in commands {
        summary.commands_seen += 1;

        let Some(candidate) = registry.get(&command.victim_guid).map(|entry| {
            let info = entry.value();
            Candidate {
                tx: info.command_tx.clone(),
                map_id: info.map_id,
                instance_id: info.instance_id,
                is_in_world: info.is_in_world,
                is_alive: info.is_alive,
            }
        }) else {
            summary.candidates_skipped_missing_victim += 1;
            continue;
        };

        summary.candidates_seen += 1;
        if !candidate.is_in_world {
            summary.candidates_skipped_not_in_world += 1;
            continue;
        }
        if !candidate.is_alive {
            summary.candidates_skipped_dead += 1;
            continue;
        }
        if candidate.map_id != command.map_id {
            summary.candidates_skipped_wrong_map += 1;
            continue;
        }
        if candidate.instance_id != command.instance_id {
            summary.candidates_skipped_wrong_instance += 1;
            continue;
        }

        let cmd = wow_network::SessionCommand::CreatureAttackStartLikeCpp(command.clone());
        if candidate.tx.try_send(cmd).is_ok() {
            summary.candidates_queued += 1;
        } else {
            summary.send_failed += 1;
        }
    }
    summary
}

/// Deliver map-owned creature melee results to their exact victim sessions.
///
/// C++ contrast: `Creature::Update` runs `DoMeleeAttackIfReady()` from the
/// map object update phase; `AttackerStateUpdate` resolves the damage once,
/// mutates the victim, then sends the combat packet. The global Rust driver
/// mirrors that by producing final-health commands once from the map owner.
/// This helper only routes those already-resolved results to the victim
/// session. It never holds map locks and uses `try_send` for backpressure.
fn deliver_creature_melee_damage_commands_like_cpp(
    commands: &[wow_network::player_registry::ApplyCreatureMeleeDamageLikeCppCommand],
    registry: &wow_network::PlayerRegistry,
) -> RuntimeCreatureMeleeDeliverySummaryLikeCpp {
    struct Candidate {
        tx: flume::Sender<wow_network::SessionCommand>,
        map_id: u16,
        instance_id: u32,
        is_in_world: bool,
    }

    let mut summary = RuntimeCreatureMeleeDeliverySummaryLikeCpp::default();
    for command in commands {
        summary.commands_seen += 1;

        let Some(candidate) = registry.get(&command.victim_guid).map(|entry| {
            let info = entry.value();
            Candidate {
                tx: info.command_tx.clone(),
                map_id: info.map_id,
                instance_id: info.instance_id,
                is_in_world: info.is_in_world,
            }
        }) else {
            summary.candidates_skipped_missing_victim += 1;
            continue;
        };

        summary.candidates_seen += 1;
        if !candidate.is_in_world {
            summary.candidates_skipped_not_in_world += 1;
            continue;
        }
        if candidate.map_id != command.map_id {
            summary.candidates_skipped_wrong_map += 1;
            continue;
        }
        if candidate.instance_id != command.instance_id {
            summary.candidates_skipped_wrong_instance += 1;
            continue;
        }

        let cmd = wow_network::SessionCommand::ApplyCreatureMeleeDamageLikeCpp(command.clone());
        if candidate.tx.try_send(cmd).is_ok() {
            summary.candidates_queued += 1;
        } else {
            summary.send_failed += 1;
        }
    }
    summary
}

/// Run one legacy global creature-movement tick and deliver its runtime plan.
///
/// Production reaches this only through the experimental
/// `RustyCore.LegacyCreatureGlobalRuntime` loop, disabled by default. The tick
/// body itself owns all map-lock ordering; delivery happens afterwards through
/// the already-tested `SendIfVisibleLikeCpp` rail.
fn run_legacy_creature_movement_tick_and_deliver_once_like_cpp(
    legacy_map_manager: &SharedMapManager,
    canonical_map_manager: Option<&SharedCanonicalMapManager>,
    mmap_config: &MMapRuntimeConfigLikeCpp,
    mmap_pathfinder: Option<&WorldMMapPathfinderWorkerLikeCpp>,
    registry: &wow_network::PlayerRegistry,
) -> (
    wow_world::session::LegacyCreatureMovementTickOutcomeLikeCpp,
    RuntimeDeliverySummaryLikeCpp,
) {
    let outcome = wow_world::session::run_legacy_creature_movement_tick_once_like_cpp(
        legacy_map_manager,
        canonical_map_manager,
        mmap_config,
        mmap_pathfinder,
    );
    let delivery = deliver_runtime_plan_like_cpp(&outcome.plan, registry);
    (outcome, delivery)
}

/// Run one legacy global creature lifecycle tick and wake affected sessions.
///
/// Production reaches this only through the experimental
/// `RustyCore.LegacyCreatureGlobalRuntime` loop, disabled by default. The
/// lifecycle body mutates legacy/canonical map state and returns map keys whose
/// sessions need to recompute creature visibility; delivery is map-scoped
/// refresh commands via `try_send`.
fn run_legacy_creature_lifecycle_tick_and_refresh_once_like_cpp(
    legacy_map_manager: &SharedMapManager,
    canonical_map_manager: Option<&SharedCanonicalMapManager>,
    now: std::time::Instant,
    registry: &wow_network::PlayerRegistry,
) -> (
    wow_world::session::LegacyCreatureLifecycleTickOutcomeLikeCpp,
    RuntimeVisibilityRefreshDeliverySummaryLikeCpp,
) {
    let outcome = wow_world::session::run_legacy_creature_lifecycle_tick_once_like_cpp(
        legacy_map_manager,
        canonical_map_manager,
        now,
    );
    let mut delivery = RuntimeVisibilityRefreshDeliverySummaryLikeCpp::default();
    for (map_id, instance_id) in &outcome.refresh_map_keys {
        delivery.merge(deliver_refresh_visible_world_creatures_like_cpp(
            *map_id,
            *instance_id,
            registry,
        ));
    }
    (outcome, delivery)
}

/// Run one legacy global creature aggro scan and deliver attack-start commands.
///
/// Production reaches this only through the experimental
/// `RustyCore.LegacyCreatureGlobalRuntime` loop, disabled by default. Candidate
/// player snapshots are collected before taking the legacy map lock; delivery
/// happens after the map-owned aggro result is computed.
fn run_legacy_creature_aggro_tick_and_deliver_once_like_cpp(
    legacy_map_manager: &SharedMapManager,
    registry: &wow_network::PlayerRegistry,
    aggro_config: wow_world::session::LegacyCreatureAggroConfigLikeCpp,
) -> (
    wow_world::session::LegacyCreatureAggroTickOutcomeLikeCpp,
    RuntimeCreatureAttackStartDeliverySummaryLikeCpp,
) {
    let candidates = collect_legacy_creature_aggro_candidates_like_cpp(registry);
    let outcome = wow_world::session::run_legacy_creature_aggro_tick_once_with_config_like_cpp(
        legacy_map_manager,
        &candidates,
        aggro_config,
    );
    let delivery = deliver_creature_attack_start_commands_like_cpp(&outcome.commands, registry);
    (outcome, delivery)
}

/// Run one legacy global creature melee tick and deliver victim commands.
///
/// Production reaches this only through the experimental
/// `RustyCore.LegacyCreatureGlobalRuntime` loop, disabled by default. The tick
/// body mutates canonical victim health and returns final-health commands; this
/// bridge delivers them outside all map locks.
fn run_legacy_creature_melee_tick_and_deliver_once_like_cpp(
    legacy_map_manager: &SharedMapManager,
    canonical_map_manager: Option<&SharedCanonicalMapManager>,
    registry: &wow_network::PlayerRegistry,
) -> (
    wow_world::session::LegacyCreatureMeleeTickOutcomeLikeCpp,
    RuntimeCreatureMeleeDeliverySummaryLikeCpp,
) {
    let outcome = wow_world::session::run_legacy_creature_melee_tick_once_like_cpp(
        legacy_map_manager,
        canonical_map_manager,
    );
    let delivery = deliver_creature_melee_damage_commands_like_cpp(&outcome.commands, registry);
    (outcome, delivery)
}

/// Combined single-shot legacy creature runtime bridge.
///
/// This is the production loop body behind the experimental
/// `RustyCore.LegacyCreatureGlobalRuntime` flag and the same body used by the
/// task-boundary tests. It mirrors the current legacy creature tick split while
/// proving that lifecycle refresh and movement fanout can run without holding
/// map locks during channel delivery.
#[derive(Debug)]
struct LegacyCreatureRuntimeTickBridgeOutcomeLikeCpp {
    pub lifecycle: wow_world::session::LegacyCreatureLifecycleTickOutcomeLikeCpp,
    pub lifecycle_delivery: RuntimeVisibilityRefreshDeliverySummaryLikeCpp,
    pub movement: wow_world::session::LegacyCreatureMovementTickOutcomeLikeCpp,
    pub movement_delivery: RuntimeDeliverySummaryLikeCpp,
    pub aggro: wow_world::session::LegacyCreatureAggroTickOutcomeLikeCpp,
    pub aggro_delivery: RuntimeCreatureAttackStartDeliverySummaryLikeCpp,
    pub melee: wow_world::session::LegacyCreatureMeleeTickOutcomeLikeCpp,
    pub melee_delivery: RuntimeCreatureMeleeDeliverySummaryLikeCpp,
}

fn run_legacy_creature_runtime_tick_and_deliver_once_like_cpp(
    legacy_map_manager: &SharedMapManager,
    canonical_map_manager: Option<&SharedCanonicalMapManager>,
    mmap_config: &MMapRuntimeConfigLikeCpp,
    mmap_pathfinder: Option<&WorldMMapPathfinderWorkerLikeCpp>,
    aggro_config: wow_world::session::LegacyCreatureAggroConfigLikeCpp,
    now: std::time::Instant,
    registry: &wow_network::PlayerRegistry,
) -> LegacyCreatureRuntimeTickBridgeOutcomeLikeCpp {
    let (lifecycle, lifecycle_delivery) =
        run_legacy_creature_lifecycle_tick_and_refresh_once_like_cpp(
            legacy_map_manager,
            canonical_map_manager,
            now,
            registry,
        );
    let (movement, movement_delivery) = run_legacy_creature_movement_tick_and_deliver_once_like_cpp(
        legacy_map_manager,
        canonical_map_manager,
        mmap_config,
        mmap_pathfinder,
        registry,
    );
    let (aggro, aggro_delivery) = run_legacy_creature_aggro_tick_and_deliver_once_like_cpp(
        legacy_map_manager,
        registry,
        aggro_config,
    );
    let (melee, melee_delivery) = run_legacy_creature_melee_tick_and_deliver_once_like_cpp(
        legacy_map_manager,
        canonical_map_manager,
        registry,
    );

    LegacyCreatureRuntimeTickBridgeOutcomeLikeCpp {
        lifecycle,
        lifecycle_delivery,
        movement,
        movement_delivery,
        aggro,
        aggro_delivery,
        melee,
        melee_delivery,
    }
}

/// Spawn the experimental legacy global creature runtime loop.
///
/// C++ contrast: `World::Update` calls `sMapMgr->Update(diff)` and
/// `MapManager::Update` uses `CONFIG_INTERVAL_MAPUPDATE` / `MapUpdateInterval`.
/// This Rust bridge uses the same configured interval, but remains disabled by
/// default and only runs when `RustyCore.LegacyCreatureGlobalRuntime != 0`.
///
/// The actual tick is executed via `spawn_blocking` because the legacy manager
/// uses `std::sync::RwLock` and movement may touch synchronous mmap/pathfinding
/// state. Packet fanout still happens outside map locks inside the single-shot
/// bridge.
fn spawn_legacy_creature_runtime_update_loop_like_cpp(
    enabled: bool,
    legacy_map_manager: SharedMapManager,
    canonical_map_manager: SharedCanonicalMapManager,
    mmap_config: MMapRuntimeConfigLikeCpp,
    mmap_pathfinder: Option<Arc<WorldMMapPathfinderWorkerLikeCpp>>,
    aggro_config: wow_world::session::LegacyCreatureAggroConfigLikeCpp,
    tick_interval_ms: u32,
    player_registry: Arc<PlayerRegistry>,
) -> tokio::task::JoinHandle<()> {
    if !enabled {
        return tokio::spawn(async {
            std::future::pending::<()>().await;
        });
    }

    tokio::spawn(async move {
        let mut interval =
            tokio::time::interval(Duration::from_millis(u64::from(tick_interval_ms.max(1))));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

        loop {
            interval.tick().await;
            let now = Instant::now();
            let legacy_for_tick = Arc::clone(&legacy_map_manager);
            let canonical_for_tick = Arc::clone(&canonical_map_manager);
            let mmap_config_for_tick = mmap_config.clone();
            let mmap_pathfinder_for_tick = mmap_pathfinder.clone();
            let aggro_config_for_tick = aggro_config.clone();
            let registry_for_tick = Arc::clone(&player_registry);

            let tick_result = tokio::task::spawn_blocking(move || {
                run_legacy_creature_runtime_tick_and_deliver_once_like_cpp(
                    &legacy_for_tick,
                    Some(&canonical_for_tick),
                    &mmap_config_for_tick,
                    mmap_pathfinder_for_tick.as_deref(),
                    aggro_config_for_tick,
                    now,
                    registry_for_tick.as_ref(),
                )
            })
            .await;

            let Ok(outcome) = tick_result else {
                tracing::error!("Legacy global creature runtime tick task panicked; stopping loop");
                break;
            };

            let touched_creatures = outcome.lifecycle.corpses_despawned
                + outcome.movement.movement_packets
                + outcome.aggro.aggro_starts
                + outcome.melee.canonical_hits;
            if touched_creatures > 0 {
                debug!(
                    lifecycle_corpses_despawned = outcome.lifecycle.corpses_despawned,
                    lifecycle_respawns_processed = outcome.lifecycle.respawns_processed,
                    lifecycle_refresh_commands = outcome.lifecycle_delivery.candidates_queued,
                    movement_packets = outcome.movement.movement_packets,
                    movement_commands = outcome.movement_delivery.candidates_queued,
                    aggro_starts = outcome.aggro.aggro_starts,
                    aggro_commands = outcome.aggro_delivery.candidates_queued,
                    melee_hits = outcome.melee.canonical_hits,
                    melee_commands = outcome.melee_delivery.candidates_queued,
                    "Legacy global creature runtime tick produced visible work"
                );
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::{
        CanonicalGameEventSchedulerLikeCpp, CanonicalRespawnConditionSchedulerLikeCpp,
        GameEventLiveUpdateActionLikeCpp, GameEventLiveUpdateSideEffectSummaryLikeCpp,
        GameEventQuestCompleteConditionSaveDbStatementKindLikeCpp,
        GameEventWorldEventStateDbOperationKindLikeCpp, GameEventWorldEventStateDbOperationLikeCpp,
        GameEventWorldEventStateDbStatementKindLikeCpp, LoadedGridCreatureRespawnCachesLikeCpp,
        PersistedRespawnLoadReportLikeCpp, PersistedRespawnRowLikeCpp,
        PersistedRespawnTimesLikeCpp, RespawnDbDeleteQueueOutcomeLikeCpp,
        RespawnDbSaveQueueOutcomeLikeCpp,
        apply_canonical_spawn_group_condition_update_loaded_grid_records_like_cpp,
        build_loaded_grid_creature_respawn_record_like_cpp,
        build_loaded_grid_creature_spawn_group_spawn_record_like_cpp,
        build_loaded_grid_gameobject_respawn_record_like_cpp,
        canonical_map_update_tick_set_inactive_like_cpp,
        collect_legacy_creature_aggro_candidates_like_cpp,
        consume_game_event_live_update_side_effects_like_cpp,
        deliver_creature_attack_start_commands_like_cpp,
        deliver_creature_melee_damage_commands_like_cpp,
        deliver_refresh_visible_world_creatures_like_cpp, deliver_runtime_plan_like_cpp,
        fanout_game_event_announcement_to_player_sessions_like_cpp,
        fanout_realm_update_world_state_to_player_sessions_like_cpp,
        fanout_reset_event_seasonal_quests_to_player_sessions_after_db_delete_like_cpp,
        game_event_announcement_lines_like_cpp, game_event_change_equip_or_model_like_cpp,
        game_event_live_update_actions_like_cpp,
        game_event_quest_complete_response_from_summary_like_cpp,
        game_event_spawn_creatures_and_gameobjects_for_event_like_cpp,
        game_event_spawn_for_event_like_cpp, game_event_spawn_pools_for_event_like_cpp,
        game_event_spawn_pools_like_cpp,
        game_event_unspawn_creatures_and_gameobjects_for_event_like_cpp,
        game_event_unspawn_for_event_like_cpp, game_event_unspawn_pools_for_event_like_cpp,
        game_event_unspawn_pools_like_cpp, game_event_update_npc_flags_like_cpp,
        game_event_update_npc_vendor_like_cpp, game_event_update_world_states_like_cpp,
        install_canonical_spawn_group_initializer_like_cpp, legacy_creature_aggro_config_like_cpp,
        legacy_creature_global_runtime_enabled_from_config_like_cpp, load_world_config_from,
        loot_drop_rates_like_cpp, materialize_game_event_quest_complete_db_bridge_like_cpp,
        materialize_game_event_world_event_state_db_bridge_like_cpp, mmap_runtime_config_like_cpp,
        persisted_respawn_info_from_row_like_cpp, queue_respawn_db_delete_like_cpp,
        queue_respawn_db_save_like_cpp, repair_cost_rate_like_cpp, reputation_rates_like_cpp,
        run_legacy_creature_lifecycle_tick_and_refresh_once_like_cpp,
        run_legacy_creature_melee_tick_and_deliver_once_like_cpp,
        run_legacy_creature_movement_tick_and_deliver_once_like_cpp,
        run_legacy_creature_runtime_tick_and_deliver_once_like_cpp, spawn_store_loader,
        world_config_bool, world_config_u8, world_config_u16,
    };
    use std::collections::{BTreeMap, HashSet};
    use std::env;
    use std::fs;
    use std::path::PathBuf;
    use std::sync::{Arc, Mutex};
    use std::time::{SystemTime, UNIX_EPOCH};
    use wow_constants::{ConditionSourceType, ConditionType};
    use wow_core::{ObjectGuid, Position, guid::HighGuid};
    use wow_data::{Condition, ConditionEntriesByTypeStore};
    use wow_database::{CharStatements, SqlParam, StatementDef};
    use wow_entities::{Creature, GameObject, MapObjectRecord, Player};
    use wow_map::{
        LinkedRespawnStoreLikeCpp, PoolGroupLikeCpp, PoolMemberKindLikeCpp, PoolMgrLikeCpp,
        PoolObjectLikeCpp, PoolTemplateDataLikeCpp, RespawnInfoLikeCpp, SpawnData, SpawnGroupFlags,
        SpawnGroupTemplateData, SpawnObjectType, SpawnPosition, SpawnStore,
        spawn::SpawnGroupMemberRow,
    };
    use wow_network::{PlayerBroadcastInfo, PlayerRegistry, SessionCommand};
    use wow_packet::{
        ServerPacket,
        packets::chat::{ChatMsg, ChatPkt},
    };

    fn player_broadcast_info_fixture_like_cpp(
        send_tx: flume::Sender<Vec<u8>>,
        command_tx: flume::Sender<SessionCommand>,
        player_name: &str,
    ) -> PlayerBroadcastInfo {
        PlayerBroadcastInfo {
            map_id: 0,
            instance_id: 0,
            position: wow_core::Position::ZERO,
            combat_reach: 0.0,
            liquid_status: 0,
            is_in_world: true,
            send_tx,
            command_tx,
            active_loot_rolls: Vec::new(),
            pass_on_group_loot: false,
            enchanting_skill: 0,
            is_alive: true,
            unit_flags: 0,
            unit_flags2: 0,
            unit_state: 0,
            is_game_master: false,
            is_contested_pvp: false,
            active_expansion: 2,
            pending_quest_sharing: None,
            known_spells: Vec::new(),
            active_quest_statuses: Default::default(),
            active_quest_objective_counts: Default::default(),
            rewarded_quests: Default::default(),
            daily_quests_completed: Default::default(),
            df_quests: Default::default(),
            faction_template_id: 0,
            reputation_standings: Vec::new(),
            reputation_state_flags: Vec::new(),
            forced_reputation_ranks: Vec::new(),
            forced_reputation_faction_ids: Vec::new(),
            inventory_item_counts: Default::default(),
            party_member_phase_states: Default::default(),
            player_name: player_name.to_string(),
            account_id: 1,
            recruiter_id: 0,
            race: 1,
            class: 1,
            sex: 0,
            level: 1,
            gray_level: 0,
            display_id: 49,
            visible_items: [(0, 0, 0); 19],
        }
    }

    fn insert_player_broadcast_fixture_with_in_world_like_cpp(
        registry: &PlayerRegistry,
        counter: u64,
        send_tx: flume::Sender<Vec<u8>>,
        command_tx: flume::Sender<SessionCommand>,
        is_in_world: bool,
    ) {
        let mut info = player_broadcast_info_fixture_like_cpp(
            send_tx,
            command_tx,
            &format!("Player{counter}"),
        );
        info.is_in_world = is_in_world;
        registry.insert(ObjectGuid::create_player(1, counter as i64), info);
    }

    fn insert_player_broadcast_fixture_like_cpp(
        registry: &PlayerRegistry,
        counter: u64,
        send_tx: flume::Sender<Vec<u8>>,
        command_tx: flume::Sender<SessionCommand>,
    ) {
        insert_player_broadcast_fixture_with_in_world_like_cpp(
            registry, counter, send_tx, command_tx, true,
        );
    }

    fn assert_del_respawn_params_like_cpp(
        statement: &wow_database::PreparedStatement,
        object_type: u16,
        spawn_id: u64,
        map_id: u16,
        instance_id: u32,
    ) {
        let [
            SqlParam::U16(actual_object_type),
            SqlParam::U64(actual_spawn_id),
            SqlParam::U16(actual_map_id),
            SqlParam::U32(actual_instance_id),
        ] = statement.params()
        else {
            panic!(
                "expected DEL_RESPAWN params [U16, U64, U16, U32], got {:?}",
                statement.params()
            );
        };
        assert_eq!(*actual_object_type, object_type);
        assert_eq!(*actual_spawn_id, spawn_id);
        assert_eq!(*actual_map_id, map_id);
        assert_eq!(*actual_instance_id, instance_id);
    }

    fn assert_rep_respawn_params_like_cpp(
        statement: &wow_database::PreparedStatement,
        object_type: u16,
        spawn_id: u64,
        respawn_time: i64,
        map_id: u16,
        instance_id: u32,
    ) {
        let [
            SqlParam::U16(actual_object_type),
            SqlParam::U64(actual_spawn_id),
            SqlParam::I64(actual_respawn_time),
            SqlParam::U16(actual_map_id),
            SqlParam::U32(actual_instance_id),
        ] = statement.params()
        else {
            panic!(
                "expected REP_RESPAWN params [U16, U64, I64, U16, U32], got {:?}",
                statement.params()
            );
        };
        assert_eq!(*actual_object_type, object_type);
        assert_eq!(*actual_spawn_id, spawn_id);
        assert_eq!(*actual_respawn_time, respawn_time);
        assert_eq!(*actual_map_id, map_id);
        assert_eq!(*actual_instance_id, instance_id);
    }

    fn assert_game_event_condition_save_delete_params_like_cpp(
        statement: &wow_database::PreparedStatement,
        event_id: u8,
        condition_id: u32,
    ) {
        let [
            SqlParam::U8(actual_event_id),
            SqlParam::U32(actual_condition_id),
        ] = statement.params()
        else {
            panic!(
                "expected DEL_GAME_EVENT_CONDITION_SAVE params [U8, U32], got {:?}",
                statement.params()
            );
        };
        assert_eq!(*actual_event_id, event_id);
        assert_eq!(*actual_condition_id, condition_id);
    }

    fn assert_game_event_condition_save_insert_params_like_cpp(
        statement: &wow_database::PreparedStatement,
        event_id: u8,
        condition_id: u32,
        done: f32,
    ) {
        let [
            SqlParam::U8(actual_event_id),
            SqlParam::U32(actual_condition_id),
            SqlParam::F32(actual_done),
        ] = statement.params()
        else {
            panic!(
                "expected INS_GAME_EVENT_CONDITION_SAVE params [U8, U32, F32], got {:?}",
                statement.params()
            );
        };
        assert_eq!(*actual_event_id, event_id);
        assert_eq!(*actual_condition_id, condition_id);
        assert_eq!(*actual_done, done);
    }

    fn game_event_quest_complete_progressed_outcome_like_cpp(
        save_world_event_state_requested: bool,
        force_game_event_update_requested: bool,
    ) -> spawn_store_loader::GameEventQuestCompleteOutcomeLikeCpp {
        spawn_store_loader::GameEventQuestCompleteOutcomeLikeCpp::Progress(
            spawn_store_loader::GameEventConditionProgressOutcomeLikeCpp::Progressed(
                spawn_store_loader::GameEventConditionProgressSummaryLikeCpp {
                    event_id: 7,
                    condition_id: 44,
                    done_before: 2.5,
                    done_after: 5.25,
                    req_num: 10.0,
                    del_statement:
                        spawn_store_loader::GameEventConditionSaveStatementEvidenceLikeCpp {
                            statement: CharStatements::DEL_GAME_EVENT_CONDITION_SAVE,
                            event_id: 7,
                            condition_id: 44,
                            done: None,
                        },
                    ins_statement:
                        spawn_store_loader::GameEventConditionSaveStatementEvidenceLikeCpp {
                            statement: CharStatements::INS_GAME_EVENT_CONDITION_SAVE,
                            event_id: 7,
                            condition_id: 44,
                            done: Some(5.25),
                        },
                    completed_event: save_world_event_state_requested,
                    check_outcome:
                        spawn_store_loader::GameEventConditionCheckOutcomeLikeCpp::Completed(
                            spawn_store_loader::GameEventConditionCheckSummaryLikeCpp {
                                event_id: 7,
                                condition_count: 1,
                                state_before_raw: 2,
                                state_after_raw: 3,
                                next_start_before: 0,
                                next_start_after: 1_234,
                            },
                        ),
                    save_world_event_state_requested,
                    force_game_event_update_requested,
                },
            ),
        )
    }

    fn linked_respawn_guid_like_cpp(
        high: wow_core::guid::HighGuid,
        entry: u32,
        spawn_id: u64,
    ) -> wow_core::ObjectGuid {
        wow_core::ObjectGuid::create_world_object(high, 0, 0, 571, 0, entry, spawn_id as i64)
    }

    fn empty_loaded_grid_creature_respawn_caches_like_cpp() -> LoadedGridCreatureRespawnCachesLikeCpp
    {
        LoadedGridCreatureRespawnCachesLikeCpp {
            template_store: Arc::new(wow_data::CreatureTemplateLifecycleStoreLikeCpp::default()),
            difficulty_store: Arc::new(wow_data::CreatureDifficultyStoreLikeCpp::default()),
            base_stats_store: Arc::new(wow_data::CreatureBaseStatsStoreLikeCpp::default()),
            health_rates: wow_data::CreatureClassificationHealthRatesLikeCpp::default(),
            display_store: Arc::new(wow_data::CreatureDisplayInfoStore::from_entries([])),
            model_store: Arc::new(wow_data::CreatureModelDataStore::from_entries([])),
            vehicle_store: Arc::new(wow_data::VehicleStore::from_entries([])),
            vehicle_seat_store: Arc::new(wow_data::VehicleSeatStore::from_entries([])),
            vehicle_accessory_store: Arc::new(wow_data::VehicleAccessoryStoreLikeCpp::from_parts(
                [],
                [],
            )),
            gameobject_template_store: Arc::new(
                wow_data::GameObjectTemplateLifecycleStoreLikeCpp::default(),
            ),
            gameobject_override_store: Arc::new(
                wow_data::GameObjectOverrideLifecycleStoreLikeCpp::default(),
            ),
        }
    }

    static TEST_LOCK: Mutex<()> = Mutex::new(());

    fn respawn_info_like_cpp(
        object_type: SpawnObjectType,
        spawn_id: wow_map::SpawnId,
        respawn_time: i64,
    ) -> RespawnInfoLikeCpp {
        RespawnInfoLikeCpp {
            object_type,
            spawn_id,
            entry: 42,
            respawn_time,
            grid_id: 7,
        }
    }

    fn canonical_spawn_metadata_with_pool_mgr_like_cpp(
        pool_mgr: PoolMgrLikeCpp,
    ) -> spawn_store_loader::CanonicalSpawnMetadataLikeCpp {
        spawn_store_loader::CanonicalSpawnMetadataLikeCpp::new(SpawnStore::new(), BTreeMap::new())
            .with_pool_mgr_like_cpp(pool_mgr)
    }

    fn canonical_spawn_metadata_with_store_and_pool_mgr_like_cpp(
        spawn_store: SpawnStore,
        pool_mgr: PoolMgrLikeCpp,
    ) -> spawn_store_loader::CanonicalSpawnMetadataLikeCpp {
        spawn_store_loader::CanonicalSpawnMetadataLikeCpp::new(spawn_store, BTreeMap::new())
            .with_pool_mgr_like_cpp(pool_mgr)
    }

    fn canonical_spawn_metadata_with_store_pool_mgr_and_game_event_pools_like_cpp(
        spawn_store: SpawnStore,
        pool_mgr: PoolMgrLikeCpp,
        game_event_pools: spawn_store_loader::GameEventPoolIdsLikeCpp,
    ) -> spawn_store_loader::CanonicalSpawnMetadataLikeCpp {
        spawn_store_loader::CanonicalSpawnMetadataLikeCpp::new(spawn_store, BTreeMap::new())
            .with_pool_mgr_like_cpp(pool_mgr)
            .with_game_event_pools_like_cpp(game_event_pools)
    }

    fn pool_mgr_with_creature_pool_like_cpp(
        pool_id: u32,
        map_id: i32,
        spawn_id: wow_map::SpawnId,
    ) -> PoolMgrLikeCpp {
        let mut pool_mgr = PoolMgrLikeCpp::new();
        pool_mgr.insert_template_like_cpp(pool_id, PoolTemplateDataLikeCpp::new(1, map_id));
        let mut group = PoolGroupLikeCpp::with_pool_id(PoolMemberKindLikeCpp::Creature, pool_id);
        group.add_entry_like_cpp(PoolObjectLikeCpp::new(spawn_id, 0.0), 1);
        pool_mgr
            .insert_or_replace_group_like_cpp(PoolMemberKindLikeCpp::Creature, pool_id, group)
            .expect("test creature pool group");
        pool_mgr
    }

    fn spawn_data_like_cpp(
        object_type: SpawnObjectType,
        spawn_id: wow_map::SpawnId,
        map_id: u32,
    ) -> SpawnData {
        SpawnData {
            object_type,
            spawn_id,
            map_id,
            db_data: true,
            spawn_group: SpawnGroupTemplateData {
                group_id: 534,
                name: "game-event-object-guid-unspawn".to_string(),
                map_id,
                flags: SpawnGroupFlags::NONE,
            },
            id: 99,
            spawn_point: SpawnPosition::new(1_000.0, 1_000.0, 0.0, 0.0),
            phase_use_flags: 0,
            phase_id: 0,
            phase_group: 0,
            terrain_swap_map: 0,
            pool_id: 0,
            spawn_time_secs: 0,
            spawn_difficulties: vec![1],
            script_id: 0,
            string_id: String::new(),
        }
    }

    fn add_spawn_data_like_cpp(
        store: &mut SpawnStore,
        object_type: SpawnObjectType,
        spawn_id: wow_map::SpawnId,
        map_id: u32,
    ) {
        store.add_object_spawn(&spawn_data_like_cpp(object_type, spawn_id, map_id), |_| {
            false
        });
    }

    fn game_event_spawn_test_spawn_data_like_cpp(
        object_type: SpawnObjectType,
        spawn_id: wow_map::SpawnId,
        map_id: u32,
        entry: u32,
        x: f32,
        y: f32,
        spawn_time_secs: i32,
    ) -> SpawnData {
        SpawnData {
            object_type,
            spawn_id,
            map_id,
            db_data: true,
            spawn_group: SpawnGroupTemplateData {
                group_id: 535,
                name: "game-event-object-guid-spawn".to_string(),
                map_id,
                flags: SpawnGroupFlags::NONE,
            },
            id: entry,
            spawn_point: SpawnPosition::new(x, y, 0.0, 0.0),
            phase_use_flags: 0,
            phase_id: 0,
            phase_group: 0,
            terrain_swap_map: 0,
            pool_id: 0,
            spawn_time_secs,
            spawn_difficulties: vec![0],
            script_id: 0,
            string_id: String::new(),
        }
    }

    fn game_event_spawn_test_caches_like_cpp(
        creature_entry: u32,
        gameobject_entry: u32,
    ) -> LoadedGridCreatureRespawnCachesLikeCpp {
        let mut caches =
            variable_loaded_grid_creature_respawn_caches_with_vehicle_id_and_difficulty_like_cpp(
                creature_entry,
                0,
                0,
            );
        let mut data = [0; wow_entities::MAX_GAMEOBJECT_DATA];
        data[11] = 1;
        caches.gameobject_template_store = Arc::new(
            wow_data::GameObjectTemplateLifecycleStoreLikeCpp::from_templates([
                wow_data::GameObjectTemplateLifecycleRecordLikeCpp {
                    entry: gameobject_entry,
                    go_type: wow_entities::GAMEOBJECT_TYPE_GOOBER,
                    display_id: 44,
                    name: "GameEventSpawn GO".to_string(),
                    size: 1.0,
                    data,
                    content_tuning_id: 0,
                    ai_name: String::new(),
                    script_name: String::new(),
                    string_id: String::new(),
                    addon: None,
                },
            ]),
        );
        caches
    }

    fn canonical_spawn_metadata_with_store_and_game_event_guids_like_cpp(
        spawn_store: SpawnStore,
        game_event_guids: spawn_store_loader::GameEventSpawnGuidsLikeCpp,
    ) -> spawn_store_loader::CanonicalSpawnMetadataLikeCpp {
        spawn_store_loader::CanonicalSpawnMetadataLikeCpp::new(spawn_store, BTreeMap::new())
            .with_game_event_spawn_guids_like_cpp(game_event_guids)
    }

    fn push_game_event_guid_for_test_like_cpp(
        mut guids: spawn_store_loader::GameEventSpawnGuidsLikeCpp,
        object_type: SpawnObjectType,
        event_id: i16,
        spawn_id: wow_map::SpawnId,
    ) -> spawn_store_loader::GameEventSpawnGuidsLikeCpp {
        assert!(
            guids.push_guid_like_cpp(object_type, event_id, spawn_id),
            "test event id/type must fit C++ GameEvent creature/gameobject GUID range"
        );
        guids
    }

    fn test_guid_like_cpp(high: HighGuid, counter: i64, entry: u32) -> ObjectGuid {
        ObjectGuid::create_world_object(high, 0, 1, 1, 1, entry, counter)
    }

    fn insert_live_creature_for_spawn_like_cpp(
        manager: &mut wow_map::MapManager,
        map_id: u32,
        spawn_id: wow_map::SpawnId,
        counter: i64,
    ) {
        let mut creature = Creature::new(false);
        creature
            .unit_mut()
            .world_mut()
            .object_mut()
            .create(test_guid_like_cpp(HighGuid::Creature, counter, 99));
        creature.unit_mut().world_mut().set_map(map_id, 0).unwrap();
        creature
            .unit_mut()
            .world_mut()
            .relocate(Position::xyz(1_000.0, 1_000.0, 0.0));
        creature.unit_mut().world_mut().object_mut().add_to_world();
        creature.set_spawn_id(spawn_id);
        manager
            .find_map_mut(map_id, 0)
            .expect("test map")
            .map_mut()
            .add_map_object_record_to_map_like_cpp(MapObjectRecord::new_creature(creature).unwrap())
            .expect("test creature add to map");
    }

    fn insert_live_gameobject_for_spawn_like_cpp(
        manager: &mut wow_map::MapManager,
        map_id: u32,
        spawn_id: wow_map::SpawnId,
        counter: i64,
    ) {
        let mut gameobject = GameObject::new();
        gameobject
            .world_mut()
            .object_mut()
            .create(test_guid_like_cpp(HighGuid::GameObject, counter, 99));
        gameobject.world_mut().set_map(map_id, 0).unwrap();
        gameobject
            .world_mut()
            .relocate(Position::xyz(1_000.0, 1_000.0, 0.0));
        gameobject.world_mut().object_mut().add_to_world();
        gameobject.set_spawn_id(spawn_id);
        manager
            .find_map_mut(map_id, 0)
            .expect("test map")
            .map_mut()
            .add_map_object_record_to_map_like_cpp(
                MapObjectRecord::new_game_object(gameobject).unwrap(),
            )
            .expect("test gameobject add to map");
    }

    #[test]
    fn game_event_unspawn_creature_gameobject_guids_queue_loaded_map_records_like_cpp() {
        let mut manager = wow_map::MapManager::default();
        manager.create_world_map(1, 0);
        manager.create_world_map(2, 0);
        let event_id = 1;
        let creature_spawn_id = 534101;
        let gameobject_spawn_id = 534201;
        let mut store = SpawnStore::new();
        add_spawn_data_like_cpp(&mut store, SpawnObjectType::Creature, creature_spawn_id, 1);
        add_spawn_data_like_cpp(
            &mut store,
            SpawnObjectType::GameObject,
            gameobject_spawn_id,
            1,
        );
        let mut guids =
            spawn_store_loader::GameEventSpawnGuidsLikeCpp::from_game_event_max_entry_like_cpp(
                Some(3),
            );
        guids = push_game_event_guid_for_test_like_cpp(
            guids,
            SpawnObjectType::Creature,
            event_id,
            creature_spawn_id,
        );
        guids = push_game_event_guid_for_test_like_cpp(
            guids,
            SpawnObjectType::GameObject,
            event_id,
            gameobject_spawn_id,
        );
        let metadata =
            canonical_spawn_metadata_with_store_and_game_event_guids_like_cpp(store, guids);
        for object_type in [SpawnObjectType::Creature, SpawnObjectType::GameObject] {
            manager
                .find_map_mut(1, 0)
                .expect("test map 1")
                .map_mut()
                .add_respawn_info_like_cpp(respawn_info_like_cpp(
                    object_type,
                    if object_type == SpawnObjectType::Creature {
                        creature_spawn_id
                    } else {
                        gameobject_spawn_id
                    },
                    534000,
                ));
            manager
                .find_map_mut(2, 0)
                .expect("test map 2")
                .map_mut()
                .add_respawn_info_like_cpp(respawn_info_like_cpp(
                    object_type,
                    if object_type == SpawnObjectType::Creature {
                        creature_spawn_id
                    } else {
                        gameobject_spawn_id
                    },
                    534000,
                ));
        }
        insert_live_creature_for_spawn_like_cpp(&mut manager, 1, creature_spawn_id, 5341011);
        insert_live_creature_for_spawn_like_cpp(&mut manager, 1, creature_spawn_id, 5341012);
        insert_live_gameobject_for_spawn_like_cpp(&mut manager, 1, gameobject_spawn_id, 5342011);
        insert_live_gameobject_for_spawn_like_cpp(&mut manager, 1, gameobject_spawn_id, 5342012);

        let summary = game_event_unspawn_creatures_and_gameobjects_for_event_like_cpp(
            &mut manager,
            &metadata,
            &[],
            event_id,
        );

        assert_eq!(summary.event_id, event_id);
        assert!(!summary.missing_event_creature_guids);
        assert!(!summary.missing_event_gameobject_guids);
        assert_eq!(summary.creature.guids_seen, 1);
        assert_eq!(summary.creature.maps_matched, 1);
        assert_eq!(summary.creature.represented_object_mgr_grid_removals, 1);
        assert_eq!(summary.creature.respawn_timers_removed, 1);
        assert_eq!(summary.creature.live_objects_queued, 2);
        assert_eq!(summary.gameobject.guids_seen, 1);
        assert_eq!(summary.gameobject.maps_matched, 1);
        assert_eq!(summary.gameobject.represented_object_mgr_grid_removals, 1);
        assert_eq!(summary.gameobject.respawn_timers_removed, 1);
        assert_eq!(summary.gameobject.live_objects_queued, 2);
        assert!(
            manager
                .find_map(2, 0)
                .expect("test map 2")
                .map()
                .respawn_timer_keys_like_cpp()
                .any(|(_, spawn_id)| spawn_id == creature_spawn_id
                    || spawn_id == gameobject_spawn_id)
        );
        let map_1 = manager.find_map_mut(1, 0).expect("test map 1").map_mut();
        let drained = map_1.remove_all_objects_in_remove_list_like_cpp();
        assert_eq!(drained.removed, 4);
    }

    #[test]
    fn game_event_unspawn_positive_event_skips_guid_active_in_other_event_like_cpp() {
        let mut manager = wow_map::MapManager::default();
        manager.create_world_map(1, 0);
        let event_id = 1;
        let other_event_id = 2;
        let spawn_id = 534301;
        let mut store = SpawnStore::new();
        add_spawn_data_like_cpp(&mut store, SpawnObjectType::Creature, spawn_id, 1);
        let mut guids =
            spawn_store_loader::GameEventSpawnGuidsLikeCpp::from_game_event_max_entry_like_cpp(
                Some(3),
            );
        guids = push_game_event_guid_for_test_like_cpp(
            guids,
            SpawnObjectType::Creature,
            event_id,
            spawn_id,
        );
        guids = push_game_event_guid_for_test_like_cpp(
            guids,
            SpawnObjectType::Creature,
            other_event_id,
            spawn_id,
        );
        let metadata =
            canonical_spawn_metadata_with_store_and_game_event_guids_like_cpp(store, guids);
        manager
            .find_map_mut(1, 0)
            .expect("test map")
            .map_mut()
            .add_respawn_info_like_cpp(respawn_info_like_cpp(
                SpawnObjectType::Creature,
                spawn_id,
                534000,
            ));
        insert_live_creature_for_spawn_like_cpp(&mut manager, 1, spawn_id, 5343011);

        let summary = game_event_unspawn_creatures_and_gameobjects_for_event_like_cpp(
            &mut manager,
            &metadata,
            &[other_event_id as u16],
            event_id,
        );

        assert_eq!(summary.creature.guids_seen, 1);
        assert_eq!(summary.creature.skipped_active_in_other_event, 1);
        assert_eq!(summary.creature.respawn_timers_removed, 0);
        assert_eq!(summary.creature.live_objects_queued, 0);
        assert!(
            manager
                .find_map(1, 0)
                .expect("test map")
                .map()
                .respawn_timer_keys_like_cpp()
                .any(|(_, timer_spawn_id)| timer_spawn_id == spawn_id)
        );
    }

    #[test]
    fn game_event_unspawn_negative_event_does_not_apply_active_event_protection_like_cpp() {
        let mut manager = wow_map::MapManager::default();
        manager.create_world_map(1, 0);
        let event_id = -1;
        let positive_event_id = 1;
        let spawn_id = 534401;
        let mut store = SpawnStore::new();
        add_spawn_data_like_cpp(&mut store, SpawnObjectType::GameObject, spawn_id, 1);
        let mut guids =
            spawn_store_loader::GameEventSpawnGuidsLikeCpp::from_game_event_max_entry_like_cpp(
                Some(3),
            );
        guids = push_game_event_guid_for_test_like_cpp(
            guids,
            SpawnObjectType::GameObject,
            event_id,
            spawn_id,
        );
        guids = push_game_event_guid_for_test_like_cpp(
            guids,
            SpawnObjectType::GameObject,
            positive_event_id,
            spawn_id,
        );
        let metadata =
            canonical_spawn_metadata_with_store_and_game_event_guids_like_cpp(store, guids);
        manager
            .find_map_mut(1, 0)
            .expect("test map")
            .map_mut()
            .add_respawn_info_like_cpp(respawn_info_like_cpp(
                SpawnObjectType::GameObject,
                spawn_id,
                534000,
            ));
        insert_live_gameobject_for_spawn_like_cpp(&mut manager, 1, spawn_id, 5344011);

        let summary = game_event_unspawn_creatures_and_gameobjects_for_event_like_cpp(
            &mut manager,
            &metadata,
            &[positive_event_id as u16],
            event_id,
        );

        assert_eq!(summary.gameobject.guids_seen, 1);
        assert_eq!(summary.gameobject.skipped_active_in_other_event, 0);
        assert_eq!(summary.gameobject.respawn_timers_removed, 1);
        assert_eq!(summary.gameobject.live_objects_queued, 1);
    }

    #[test]
    fn game_event_unspawn_missing_creature_guid_list_returns_before_gameobjects_like_cpp() {
        let mut manager = wow_map::MapManager::default();
        manager.create_world_map(1, 0);
        let event_id = 99;
        let gameobject_spawn_id = 534501;
        let mut store = SpawnStore::new();
        add_spawn_data_like_cpp(
            &mut store,
            SpawnObjectType::GameObject,
            gameobject_spawn_id,
            1,
        );
        let mut guids =
            spawn_store_loader::GameEventSpawnGuidsLikeCpp::from_game_event_max_entry_like_cpp(
                Some(2),
            );
        guids = push_game_event_guid_for_test_like_cpp(
            guids,
            SpawnObjectType::GameObject,
            1,
            gameobject_spawn_id,
        );
        let metadata =
            canonical_spawn_metadata_with_store_and_game_event_guids_like_cpp(store, guids);
        manager
            .find_map_mut(1, 0)
            .expect("test map")
            .map_mut()
            .add_respawn_info_like_cpp(respawn_info_like_cpp(
                SpawnObjectType::GameObject,
                gameobject_spawn_id,
                534000,
            ));

        let summary = game_event_unspawn_creatures_and_gameobjects_for_event_like_cpp(
            &mut manager,
            &metadata,
            &[],
            event_id,
        );

        assert_eq!(summary.event_id, event_id);
        assert!(summary.missing_event_creature_guids);
        assert!(!summary.missing_event_gameobject_guids);
        assert_eq!(summary.gameobject.guids_seen, 0);
        assert!(
            manager
                .find_map(1, 0)
                .expect("test map")
                .map()
                .respawn_timer_keys_like_cpp()
                .any(|(_, spawn_id)| spawn_id == gameobject_spawn_id)
        );
    }

    #[test]
    fn game_event_unspawn_for_event_applies_non_pool_then_pool_like_cpp() {
        let mut manager = wow_map::MapManager::default();
        manager.create_world_map(1, 0);
        manager.create_world_map(2, 0);
        let event_id = 3;
        let creature_spawn_id = 536101;
        let gameobject_spawn_id = 536102;
        let pool_id = 536103;
        let pool_spawn_id = 536104;
        let mut store = SpawnStore::new();
        add_spawn_data_like_cpp(&mut store, SpawnObjectType::Creature, creature_spawn_id, 1);
        add_spawn_data_like_cpp(
            &mut store,
            SpawnObjectType::GameObject,
            gameobject_spawn_id,
            1,
        );
        let mut guids =
            spawn_store_loader::GameEventSpawnGuidsLikeCpp::from_game_event_max_entry_like_cpp(
                Some(10),
            );
        guids = push_game_event_guid_for_test_like_cpp(
            guids,
            SpawnObjectType::Creature,
            event_id,
            creature_spawn_id,
        );
        guids = push_game_event_guid_for_test_like_cpp(
            guids,
            SpawnObjectType::GameObject,
            event_id,
            gameobject_spawn_id,
        );
        let game_event_pools =
            spawn_store_loader::GameEventPoolIdsLikeCpp::from_game_event_max_entry_like_cpp(Some(
                10,
            ))
            .with_pool_ids_for_event_like_cpp(event_id, [pool_id]);
        let metadata =
            spawn_store_loader::CanonicalSpawnMetadataLikeCpp::new(store, BTreeMap::new())
                .with_pool_mgr_like_cpp(pool_mgr_with_creature_pool_like_cpp(
                    pool_id,
                    1,
                    pool_spawn_id,
                ))
                .with_game_event_pools_like_cpp(game_event_pools)
                .with_game_event_spawn_guids_like_cpp(guids);
        for (object_type, spawn_id) in [
            (SpawnObjectType::Creature, creature_spawn_id),
            (SpawnObjectType::GameObject, gameobject_spawn_id),
            (SpawnObjectType::Creature, pool_spawn_id),
        ] {
            manager
                .find_map_mut(1, 0)
                .expect("test map")
                .map_mut()
                .add_respawn_info_like_cpp(respawn_info_like_cpp(object_type, spawn_id, 536000));
        }
        insert_live_creature_for_spawn_like_cpp(&mut manager, 1, creature_spawn_id, 5361011);
        insert_live_gameobject_for_spawn_like_cpp(&mut manager, 1, gameobject_spawn_id, 5361021);
        manager
            .find_map_mut(1, 0)
            .expect("test map")
            .map_mut()
            .pool_data_mut_like_cpp()
            .add_spawn_like_cpp(SpawnObjectType::Creature, pool_spawn_id, pool_id)
            .expect("test spawned creature pool data");

        let summary = game_event_unspawn_for_event_like_cpp(&mut manager, &metadata, &[], event_id);

        assert_eq!(summary.event_id, event_id);
        assert!(!summary.pool_skipped_due_to_non_pool_bucket);
        assert!(!summary.non_pool.missing_event_creature_guids);
        assert!(!summary.non_pool.missing_event_gameobject_guids);
        assert_eq!(summary.non_pool.creature.respawn_timers_removed, 1);
        assert_eq!(summary.non_pool.creature.live_objects_queued, 1);
        assert_eq!(summary.non_pool.gameobject.respawn_timers_removed, 1);
        assert_eq!(summary.non_pool.gameobject.live_objects_queued, 1);
        assert!(!summary.pool.missing_event_pool_ids);
        assert_eq!(summary.pool.pool_summary.event_pool_ids_seen, 1);
        assert_eq!(summary.pool.pool_summary.maps_matched, 1);
        assert!(
            summary
                .pool
                .pool_summary
                .blocked_pool_plan_errors
                .is_empty()
        );
        let map = manager.find_map(1, 0).expect("test map").map();
        assert!(
            !map.pool_data_like_cpp()
                .is_spawned_creature_like_cpp(pool_spawn_id)
        );
        assert_eq!(
            map.get_respawn_time_like_cpp(SpawnObjectType::Creature, creature_spawn_id),
            0
        );
        assert_eq!(
            map.get_respawn_time_like_cpp(SpawnObjectType::GameObject, gameobject_spawn_id),
            0
        );
        let drained = manager
            .find_map_mut(1, 0)
            .expect("test map")
            .map_mut()
            .remove_all_objects_in_remove_list_like_cpp();
        assert_eq!(drained.removed, 2);
    }

    #[test]
    fn game_event_unspawn_for_event_missing_creature_bucket_skips_gameobjects_and_pool_like_cpp() {
        let mut manager = wow_map::MapManager::default();
        manager.create_world_map(1, 0);
        let event_id = 99;
        let pool_id = 536201;
        let pool_spawn_id = 536202;
        let gameobject_spawn_id = 536203;
        let mut store = SpawnStore::new();
        add_spawn_data_like_cpp(
            &mut store,
            SpawnObjectType::GameObject,
            gameobject_spawn_id,
            1,
        );
        let guids = push_game_event_guid_for_test_like_cpp(
            spawn_store_loader::GameEventSpawnGuidsLikeCpp::from_game_event_max_entry_like_cpp(
                Some(2),
            ),
            SpawnObjectType::GameObject,
            1,
            gameobject_spawn_id,
        );
        let game_event_pools =
            spawn_store_loader::GameEventPoolIdsLikeCpp::from_game_event_max_entry_like_cpp(Some(
                100,
            ))
            .with_pool_ids_for_event_like_cpp(event_id, [pool_id]);
        let metadata =
            spawn_store_loader::CanonicalSpawnMetadataLikeCpp::new(store, BTreeMap::new())
                .with_pool_mgr_like_cpp(pool_mgr_with_creature_pool_like_cpp(
                    pool_id,
                    1,
                    pool_spawn_id,
                ))
                .with_game_event_pools_like_cpp(game_event_pools)
                .with_game_event_spawn_guids_like_cpp(guids);
        let map = manager.find_map_mut(1, 0).expect("test map").map_mut();
        map.add_respawn_info_like_cpp(respawn_info_like_cpp(
            SpawnObjectType::GameObject,
            gameobject_spawn_id,
            536200,
        ));
        map.add_respawn_info_like_cpp(respawn_info_like_cpp(
            SpawnObjectType::Creature,
            pool_spawn_id,
            536200,
        ));
        map.pool_data_mut_like_cpp()
            .add_spawn_like_cpp(SpawnObjectType::Creature, pool_spawn_id, pool_id)
            .expect("test spawned creature pool data");

        let summary = game_event_unspawn_for_event_like_cpp(&mut manager, &metadata, &[], event_id);

        assert_eq!(summary.event_id, event_id);
        assert!(summary.non_pool.missing_event_creature_guids);
        assert!(!summary.non_pool.missing_event_gameobject_guids);
        assert_eq!(summary.non_pool.gameobject.guids_seen, 0);
        assert!(summary.pool_skipped_due_to_non_pool_bucket);
        assert!(!summary.pool.missing_event_pool_ids);
        assert_eq!(summary.pool.pool_summary.event_pool_ids_seen, 0);
        let map = manager.find_map(1, 0).expect("test map").map();
        assert!(
            map.respawn_timer_keys_like_cpp()
                .any(|(_, spawn_id)| spawn_id == gameobject_spawn_id)
        );
        assert!(
            map.pool_data_like_cpp()
                .is_spawned_creature_like_cpp(pool_spawn_id)
        );
        assert!(
            map.respawn_timer_keys_like_cpp()
                .any(|(_, spawn_id)| spawn_id == pool_spawn_id)
        );
    }

    #[test]
    fn game_event_unspawn_for_event_missing_pool_bucket_keeps_non_pool_effects_like_cpp() {
        let mut manager = wow_map::MapManager::default();
        manager.create_world_map(1, 0);
        let event_id = 99;
        let creature_spawn_id = 536301;
        let gameobject_spawn_id = 536302;
        let mut store = SpawnStore::new();
        add_spawn_data_like_cpp(&mut store, SpawnObjectType::Creature, creature_spawn_id, 1);
        add_spawn_data_like_cpp(
            &mut store,
            SpawnObjectType::GameObject,
            gameobject_spawn_id,
            1,
        );
        let mut guids =
            spawn_store_loader::GameEventSpawnGuidsLikeCpp::from_game_event_max_entry_like_cpp(
                Some(100),
            );
        guids = push_game_event_guid_for_test_like_cpp(
            guids,
            SpawnObjectType::Creature,
            event_id,
            creature_spawn_id,
        );
        guids = push_game_event_guid_for_test_like_cpp(
            guids,
            SpawnObjectType::GameObject,
            event_id,
            gameobject_spawn_id,
        );
        let metadata =
            spawn_store_loader::CanonicalSpawnMetadataLikeCpp::new(store, BTreeMap::new())
                .with_game_event_pools_like_cpp(
                    spawn_store_loader::GameEventPoolIdsLikeCpp::from_game_event_max_entry_like_cpp(
                        Some(2),
                    ),
                )
                .with_game_event_spawn_guids_like_cpp(guids);
        let map = manager.find_map_mut(1, 0).expect("test map").map_mut();
        map.add_respawn_info_like_cpp(respawn_info_like_cpp(
            SpawnObjectType::Creature,
            creature_spawn_id,
            536300,
        ));
        map.add_respawn_info_like_cpp(respawn_info_like_cpp(
            SpawnObjectType::GameObject,
            gameobject_spawn_id,
            536300,
        ));
        insert_live_creature_for_spawn_like_cpp(&mut manager, 1, creature_spawn_id, 5363011);
        insert_live_gameobject_for_spawn_like_cpp(&mut manager, 1, gameobject_spawn_id, 5363021);

        let summary = game_event_unspawn_for_event_like_cpp(&mut manager, &metadata, &[], event_id);

        assert!(!summary.pool_skipped_due_to_non_pool_bucket);
        assert_eq!(summary.non_pool.creature.respawn_timers_removed, 1);
        assert_eq!(summary.non_pool.creature.live_objects_queued, 1);
        assert_eq!(summary.non_pool.gameobject.respawn_timers_removed, 1);
        assert_eq!(summary.non_pool.gameobject.live_objects_queued, 1);
        assert!(summary.pool.missing_event_pool_ids);
        assert_eq!(summary.pool.pool_summary.event_pool_ids_seen, 0);
        let map = manager.find_map(1, 0).expect("test map").map();
        assert_eq!(
            map.get_respawn_time_like_cpp(SpawnObjectType::Creature, creature_spawn_id),
            0
        );
        assert_eq!(
            map.get_respawn_time_like_cpp(SpawnObjectType::GameObject, gameobject_spawn_id),
            0
        );
    }

    #[test]
    fn game_event_spawn_non_pool_creature_and_gameobject_loaded_grid_adds_records_like_cpp() {
        let mut manager = wow_map::MapManager::default();
        let map = manager.create_world_map(571, 0);
        assert!(map.map_mut().load_grid(0.0, 0.0));
        let event_id = 1;
        let creature_spawn_id = 535101;
        let gameobject_spawn_id = 535201;
        let creature_entry = 42;
        let gameobject_entry = 9001;
        let mut store = SpawnStore::new();
        store.add_object_spawn(
            &game_event_spawn_test_spawn_data_like_cpp(
                SpawnObjectType::Creature,
                creature_spawn_id,
                571,
                creature_entry,
                0.0,
                0.0,
                120,
            ),
            |_| false,
        );
        store.add_object_spawn(
            &game_event_spawn_test_spawn_data_like_cpp(
                SpawnObjectType::GameObject,
                gameobject_spawn_id,
                571,
                gameobject_entry,
                0.0,
                0.0,
                30,
            ),
            |_| false,
        );
        let mut guids =
            spawn_store_loader::GameEventSpawnGuidsLikeCpp::from_game_event_max_entry_like_cpp(
                Some(3),
            );
        guids = push_game_event_guid_for_test_like_cpp(
            guids,
            SpawnObjectType::Creature,
            event_id,
            creature_spawn_id,
        );
        guids = push_game_event_guid_for_test_like_cpp(
            guids,
            SpawnObjectType::GameObject,
            event_id,
            gameobject_spawn_id,
        );
        let metadata =
            spawn_store_loader::CanonicalSpawnMetadataLikeCpp::new(store, BTreeMap::new())
                .with_game_event_spawn_guids_like_cpp(guids)
                .with_creature_runtime_rows_like_cpp(BTreeMap::from([(
                    creature_spawn_id,
                    spawn_store_loader::CreatureSpawnRuntimeRowLikeCpp {
                        spawn_id: creature_spawn_id,
                        model_id: 999,
                        equipment_id: 3,
                        wander_distance: 15.0,
                        curhealth: 0,
                        curmana: 0,
                        movement_type: 1,
                        ground_movement_type: wow_constants::CreatureGroundMovementType::Run as u8,
                        swim_allowed: true,
                        flight_movement_type: 0,
                        string_id: "game-event-spawn-creature".to_string(),
                        spawn_time_secs: 120,
                    },
                )]))
                .with_gameobject_runtime_rows_like_cpp(BTreeMap::from([(
                    gameobject_spawn_id,
                    spawn_store_loader::GameObjectSpawnRuntimeRowLikeCpp {
                        spawn_id: gameobject_spawn_id,
                        rotation: [0.0, 0.0, 0.0, 1.0],
                        anim_progress: 55,
                        state: 1,
                        string_id: "game-event-spawn-go".to_string(),
                        spawn_time_secs: 30,
                    },
                )]));
        let caches = game_event_spawn_test_caches_like_cpp(creature_entry, gameobject_entry);
        let map = manager.find_map_mut(571, 0).expect("test map").map_mut();
        map.add_respawn_info_like_cpp(respawn_info_like_cpp(
            SpawnObjectType::Creature,
            creature_spawn_id,
            535000,
        ));
        map.add_respawn_info_like_cpp(respawn_info_like_cpp(
            SpawnObjectType::GameObject,
            gameobject_spawn_id,
            535000,
        ));

        let summary =
            game_event_spawn_for_event_like_cpp(&mut manager, &metadata, &caches, event_id);

        assert_eq!(summary.event_id, event_id);
        assert!(!summary.non_pool.missing_event_creature_guids);
        assert!(!summary.non_pool.missing_event_gameobject_guids);
        assert_eq!(summary.non_pool.creature.guids_seen, 1);
        assert_eq!(summary.non_pool.creature.respawn_timers_removed, 1);
        assert_eq!(summary.non_pool.creature.load_attempts, 1);
        assert_eq!(summary.non_pool.creature.successful_loaded_grid_spawns, 1);
        assert_eq!(summary.non_pool.gameobject.guids_seen, 1);
        assert_eq!(summary.non_pool.gameobject.respawn_timers_removed, 1);
        assert_eq!(summary.non_pool.gameobject.load_attempts, 1);
        assert_eq!(summary.non_pool.gameobject.successful_loaded_grid_spawns, 1);
        assert!(summary.pool.missing_event_pool_ids);
        let map = manager.find_map(571, 0).expect("test map").map();
        assert_eq!(
            map.get_respawn_time_like_cpp(SpawnObjectType::Creature, creature_spawn_id),
            0
        );
        assert_eq!(
            map.get_respawn_time_like_cpp(SpawnObjectType::GameObject, gameobject_spawn_id),
            0
        );
        let creature = map
            .get_creature_by_spawn_id_like_cpp(creature_spawn_id)
            .expect("GameEventSpawn should add loaded-grid Creature");
        assert_eq!(creature.respawn_time(), 0);
        let gameobject = map
            .get_gameobject_by_spawn_id_like_cpp(gameobject_spawn_id)
            .expect("GameEventSpawn should add spawned-by-default GameObject");
        assert_eq!(gameobject.respawn_time(), 0);
        assert!(gameobject.spawned_by_default());
    }

    #[test]
    fn game_event_spawn_for_event_missing_creature_bucket_skips_gameobjects_and_pool_like_cpp() {
        let mut manager = wow_map::MapManager::default();
        manager.create_world_map(1, 0);
        let event_id = 99;
        let pool_id = 535901;
        let pool_spawn_id = 535902;
        let gameobject_spawn_id = 535903;
        let mut store = SpawnStore::new();
        add_spawn_data_like_cpp(&mut store, SpawnObjectType::Creature, pool_spawn_id, 1);
        add_spawn_data_like_cpp(
            &mut store,
            SpawnObjectType::GameObject,
            gameobject_spawn_id,
            1,
        );
        let mut guids =
            spawn_store_loader::GameEventSpawnGuidsLikeCpp::from_game_event_max_entry_like_cpp(
                Some(2),
            );
        guids = push_game_event_guid_for_test_like_cpp(
            guids,
            SpawnObjectType::GameObject,
            1,
            gameobject_spawn_id,
        );
        let game_event_pools =
            spawn_store_loader::GameEventPoolIdsLikeCpp::from_game_event_max_entry_like_cpp(Some(
                100,
            ))
            .with_pool_ids_for_event_like_cpp(event_id, [pool_id]);
        let metadata =
            spawn_store_loader::CanonicalSpawnMetadataLikeCpp::new(store, BTreeMap::new())
                .with_pool_mgr_like_cpp(pool_mgr_with_creature_pool_like_cpp(
                    pool_id,
                    1,
                    pool_spawn_id,
                ))
                .with_game_event_pools_like_cpp(game_event_pools)
                .with_game_event_spawn_guids_like_cpp(guids);
        let caches = empty_loaded_grid_creature_respawn_caches_like_cpp();

        let summary =
            game_event_spawn_for_event_like_cpp(&mut manager, &metadata, &caches, event_id);

        assert_eq!(summary.event_id, event_id);
        assert!(summary.non_pool.missing_event_creature_guids);
        assert!(!summary.non_pool.missing_event_gameobject_guids);
        assert_eq!(summary.non_pool.gameobject.guids_seen, 0);
        assert!(summary.pool_skipped_due_to_non_pool_bucket);
        assert!(!summary.pool.missing_event_pool_ids);
        assert_eq!(summary.pool.pool_summary.event_pool_ids_seen, 0);
        let map = manager.find_map(1, 0).expect("test map").map();
        assert!(
            !map.pool_data_like_cpp()
                .is_spawned_creature_like_cpp(pool_spawn_id)
        );
        assert_eq!(
            map.get_respawn_time_like_cpp(SpawnObjectType::GameObject, gameobject_spawn_id),
            0
        );
    }

    #[test]
    fn game_event_spawn_for_event_missing_gameobject_bucket_skips_pool_after_creatures_like_cpp() {
        let mut manager = wow_map::MapManager::default();
        let map = manager.create_world_map(571, 0);
        assert!(map.map_mut().load_grid(0.0, 0.0));
        manager.create_world_map(1, 0);
        let event_id = 7;
        let creature_spawn_id = 535904;
        let pool_id = 535905;
        let pool_spawn_id = 535906;
        let creature_entry = 42;
        let mut store = SpawnStore::new();
        store.add_object_spawn(
            &game_event_spawn_test_spawn_data_like_cpp(
                SpawnObjectType::Creature,
                creature_spawn_id,
                571,
                creature_entry,
                0.0,
                0.0,
                120,
            ),
            |_| false,
        );
        add_spawn_data_like_cpp(&mut store, SpawnObjectType::Creature, pool_spawn_id, 1);
        let mut guids =
            spawn_store_loader::GameEventSpawnGuidsLikeCpp::from_game_event_max_entry_like_cpp(
                Some(10),
            );
        guids = push_game_event_guid_for_test_like_cpp(
            guids,
            SpawnObjectType::Creature,
            event_id,
            creature_spawn_id,
        )
        .truncate_gameobject_guid_buckets_for_test_like_cpp(17);
        let game_event_pools =
            spawn_store_loader::GameEventPoolIdsLikeCpp::from_game_event_max_entry_like_cpp(Some(
                10,
            ))
            .with_pool_ids_for_event_like_cpp(event_id, [pool_id]);
        let metadata =
            spawn_store_loader::CanonicalSpawnMetadataLikeCpp::new(store, BTreeMap::new())
                .with_pool_mgr_like_cpp(pool_mgr_with_creature_pool_like_cpp(
                    pool_id,
                    1,
                    pool_spawn_id,
                ))
                .with_game_event_pools_like_cpp(game_event_pools)
                .with_game_event_spawn_guids_like_cpp(guids)
                .with_creature_runtime_rows_like_cpp(BTreeMap::from([(
                    creature_spawn_id,
                    spawn_store_loader::CreatureSpawnRuntimeRowLikeCpp {
                        spawn_id: creature_spawn_id,
                        model_id: 999,
                        equipment_id: 3,
                        wander_distance: 15.0,
                        curhealth: 0,
                        curmana: 0,
                        movement_type: 1,
                        ground_movement_type: wow_constants::CreatureGroundMovementType::Run as u8,
                        swim_allowed: true,
                        flight_movement_type: 0,
                        string_id: "game-event-spawn-creature-before-missing-go".to_string(),
                        spawn_time_secs: 120,
                    },
                )]));
        let caches = game_event_spawn_test_caches_like_cpp(creature_entry, 9001);
        manager
            .find_map_mut(571, 0)
            .expect("test map")
            .map_mut()
            .add_respawn_info_like_cpp(respawn_info_like_cpp(
                SpawnObjectType::Creature,
                creature_spawn_id,
                535000,
            ));

        let summary =
            game_event_spawn_for_event_like_cpp(&mut manager, &metadata, &caches, event_id);

        assert_eq!(summary.event_id, event_id);
        assert!(!summary.non_pool.missing_event_creature_guids);
        assert!(summary.non_pool.missing_event_gameobject_guids);
        assert_eq!(summary.non_pool.creature.guids_seen, 1);
        assert_eq!(summary.non_pool.creature.respawn_timers_removed, 1);
        assert_eq!(summary.non_pool.creature.successful_loaded_grid_spawns, 1);
        assert_eq!(summary.non_pool.gameobject.guids_seen, 0);
        assert!(summary.pool_skipped_due_to_non_pool_bucket);
        assert!(!summary.pool.missing_event_pool_ids);
        assert_eq!(summary.pool.pool_summary.event_pool_ids_seen, 0);
        let creature_map = manager.find_map(571, 0).expect("creature map").map();
        assert!(
            creature_map
                .get_creature_by_spawn_id_like_cpp(creature_spawn_id)
                .is_some()
        );
        let pool_map = manager.find_map(1, 0).expect("pool map").map();
        assert!(
            !pool_map
                .pool_data_like_cpp()
                .is_spawned_creature_like_cpp(pool_spawn_id)
        );
    }

    #[test]
    fn game_event_spawn_non_pool_unloaded_grid_removes_timer_without_fabricating_like_cpp() {
        let mut manager = wow_map::MapManager::default();
        manager.create_world_map(571, 0);
        let event_id = 1;
        let spawn_id = 535301;
        let entry = 42;
        let mut store = SpawnStore::new();
        store.add_object_spawn(
            &game_event_spawn_test_spawn_data_like_cpp(
                SpawnObjectType::Creature,
                spawn_id,
                571,
                entry,
                1_000.0,
                1_000.0,
                120,
            ),
            |_| false,
        );
        let guids = push_game_event_guid_for_test_like_cpp(
            spawn_store_loader::GameEventSpawnGuidsLikeCpp::from_game_event_max_entry_like_cpp(
                Some(2),
            ),
            SpawnObjectType::Creature,
            event_id,
            spawn_id,
        );
        let metadata =
            spawn_store_loader::CanonicalSpawnMetadataLikeCpp::new(store, BTreeMap::new())
                .with_game_event_spawn_guids_like_cpp(guids)
                .with_creature_runtime_rows_like_cpp(BTreeMap::from([(
                    spawn_id,
                    spawn_store_loader::CreatureSpawnRuntimeRowLikeCpp {
                        spawn_id,
                        model_id: 999,
                        equipment_id: 3,
                        wander_distance: 15.0,
                        curhealth: 0,
                        curmana: 0,
                        movement_type: 1,
                        ground_movement_type: wow_constants::CreatureGroundMovementType::Run as u8,
                        swim_allowed: true,
                        flight_movement_type: 0,
                        string_id: "game-event-unloaded-creature".to_string(),
                        spawn_time_secs: 120,
                    },
                )]));
        let caches = game_event_spawn_test_caches_like_cpp(entry, 9001);
        manager
            .find_map_mut(571, 0)
            .expect("test map")
            .map_mut()
            .add_respawn_info_like_cpp(respawn_info_like_cpp(
                SpawnObjectType::Creature,
                spawn_id,
                535000,
            ));

        let summary = game_event_spawn_creatures_and_gameobjects_for_event_like_cpp(
            &mut manager,
            &metadata,
            &caches,
            event_id,
        );

        assert_eq!(summary.creature.guids_seen, 1);
        assert_eq!(summary.creature.maps_matched, 1);
        assert_eq!(summary.creature.respawn_timers_removed, 1);
        assert_eq!(summary.creature.unloaded_grid_skips, 1);
        assert_eq!(summary.creature.load_attempts, 0);
        assert_eq!(summary.creature.successful_loaded_grid_spawns, 0);
        let map = manager.find_map(571, 0).expect("test map").map();
        assert_eq!(
            map.get_respawn_time_like_cpp(SpawnObjectType::Creature, spawn_id),
            0
        );
        assert!(map.get_creature_by_spawn_id_like_cpp(spawn_id).is_none());
    }

    #[test]
    fn game_event_spawn_missing_creature_bucket_returns_before_gameobjects_like_cpp() {
        let mut manager = wow_map::MapManager::default();
        let map = manager.create_world_map(571, 0);
        assert!(map.map_mut().load_grid(0.0, 0.0));
        let event_id = 99;
        let gameobject_spawn_id = 535401;
        let gameobject_entry = 9001;
        let mut store = SpawnStore::new();
        store.add_object_spawn(
            &game_event_spawn_test_spawn_data_like_cpp(
                SpawnObjectType::GameObject,
                gameobject_spawn_id,
                571,
                gameobject_entry,
                0.0,
                0.0,
                30,
            ),
            |_| false,
        );
        let guids = push_game_event_guid_for_test_like_cpp(
            spawn_store_loader::GameEventSpawnGuidsLikeCpp::from_game_event_max_entry_like_cpp(
                Some(2),
            ),
            SpawnObjectType::GameObject,
            1,
            gameobject_spawn_id,
        );
        let metadata =
            spawn_store_loader::CanonicalSpawnMetadataLikeCpp::new(store, BTreeMap::new())
                .with_game_event_spawn_guids_like_cpp(guids)
                .with_gameobject_runtime_rows_like_cpp(BTreeMap::from([(
                    gameobject_spawn_id,
                    spawn_store_loader::GameObjectSpawnRuntimeRowLikeCpp {
                        spawn_id: gameobject_spawn_id,
                        rotation: [0.0, 0.0, 0.0, 1.0],
                        anim_progress: 55,
                        state: 1,
                        string_id: "game-event-missing-creature-bucket-go".to_string(),
                        spawn_time_secs: 30,
                    },
                )]));
        let caches = game_event_spawn_test_caches_like_cpp(42, gameobject_entry);
        manager
            .find_map_mut(571, 0)
            .expect("test map")
            .map_mut()
            .add_respawn_info_like_cpp(respawn_info_like_cpp(
                SpawnObjectType::GameObject,
                gameobject_spawn_id,
                535000,
            ));

        let summary = game_event_spawn_creatures_and_gameobjects_for_event_like_cpp(
            &mut manager,
            &metadata,
            &caches,
            event_id,
        );

        assert_eq!(summary.event_id, event_id);
        assert!(summary.missing_event_creature_guids);
        assert_eq!(summary.gameobject.guids_seen, 0);
        let map = manager.find_map(571, 0).expect("test map").map();
        assert_eq!(
            map.get_respawn_time_like_cpp(SpawnObjectType::GameObject, gameobject_spawn_id),
            535000
        );
        assert!(
            map.get_gameobject_by_spawn_id_like_cpp(gameobject_spawn_id)
                .is_none()
        );
    }

    #[test]
    fn game_event_spawn_non_pool_gameobject_not_spawned_by_default_is_not_added_like_cpp() {
        let mut manager = wow_map::MapManager::default();
        let map = manager.create_world_map(571, 0);
        assert!(map.map_mut().load_grid(0.0, 0.0));
        let event_id = 1;
        let spawn_id = 535501;
        let entry = 9001;
        let mut store = SpawnStore::new();
        store.add_object_spawn(
            &game_event_spawn_test_spawn_data_like_cpp(
                SpawnObjectType::GameObject,
                spawn_id,
                571,
                entry,
                0.0,
                0.0,
                -30,
            ),
            |_| false,
        );
        let guids = push_game_event_guid_for_test_like_cpp(
            spawn_store_loader::GameEventSpawnGuidsLikeCpp::from_game_event_max_entry_like_cpp(
                Some(2),
            ),
            SpawnObjectType::GameObject,
            event_id,
            spawn_id,
        );
        let metadata =
            spawn_store_loader::CanonicalSpawnMetadataLikeCpp::new(store, BTreeMap::new())
                .with_game_event_spawn_guids_like_cpp(guids)
                .with_gameobject_runtime_rows_like_cpp(BTreeMap::from([(
                    spawn_id,
                    spawn_store_loader::GameObjectSpawnRuntimeRowLikeCpp {
                        spawn_id,
                        rotation: [0.0, 0.0, 0.0, 1.0],
                        anim_progress: 55,
                        state: 1,
                        string_id: "game-event-go-not-default".to_string(),
                        spawn_time_secs: -30,
                    },
                )]));
        let caches = game_event_spawn_test_caches_like_cpp(42, entry);
        manager
            .find_map_mut(571, 0)
            .expect("test map")
            .map_mut()
            .add_respawn_info_like_cpp(respawn_info_like_cpp(
                SpawnObjectType::GameObject,
                spawn_id,
                535000,
            ));

        let summary = game_event_spawn_creatures_and_gameobjects_for_event_like_cpp(
            &mut manager,
            &metadata,
            &caches,
            event_id,
        );

        assert_eq!(summary.gameobject.guids_seen, 1);
        assert_eq!(summary.gameobject.respawn_timers_removed, 1);
        assert_eq!(summary.gameobject.load_attempts, 1);
        assert_eq!(
            summary.gameobject.gameobject_not_spawned_by_default_skips,
            1
        );
        assert_eq!(summary.gameobject.successful_loaded_grid_spawns, 0);
        let map = manager.find_map(571, 0).expect("test map").map();
        assert_eq!(
            map.get_respawn_time_like_cpp(SpawnObjectType::GameObject, spawn_id),
            0
        );
        assert!(map.get_gameobject_by_spawn_id_like_cpp(spawn_id).is_none());
    }

    #[test]
    fn game_event_pool_spawn_uses_canonical_event_pool_ids_like_cpp() {
        let mut manager = wow_map::MapManager::default();
        manager.create_world_map(1, 0);
        manager.create_world_map(2, 0);
        let event_id = 7;
        let pool_id = 5321;
        let spawn_id = 532101;
        let mut store = SpawnStore::new();
        store.add_object_spawn(
            &SpawnData {
                object_type: SpawnObjectType::Creature,
                spawn_id,
                map_id: 1,
                db_data: true,
                spawn_group: SpawnGroupTemplateData {
                    group_id: 5321,
                    name: "game-event-canonical-spawn".to_string(),
                    map_id: 1,
                    flags: SpawnGroupFlags::NONE,
                },
                id: 99,
                spawn_point: SpawnPosition::new(1_000.0, 1_000.0, 0.0, 0.0),
                phase_use_flags: 0,
                phase_id: 0,
                phase_group: 0,
                terrain_swap_map: 0,
                pool_id,
                spawn_time_secs: 0,
                spawn_difficulties: vec![1],
                script_id: 0,
                string_id: String::new(),
            },
            |_| false,
        );
        let game_event_pools =
            spawn_store_loader::GameEventPoolIdsLikeCpp::from_game_event_max_entry_like_cpp(Some(
                10,
            ))
            .with_pool_ids_for_event_like_cpp(event_id, [pool_id]);
        let metadata = canonical_spawn_metadata_with_store_pool_mgr_and_game_event_pools_like_cpp(
            store,
            pool_mgr_with_creature_pool_like_cpp(pool_id, 1, spawn_id),
            game_event_pools,
        );
        let caches = empty_loaded_grid_creature_respawn_caches_like_cpp();

        let summary =
            game_event_spawn_pools_for_event_like_cpp(&mut manager, &metadata, &caches, event_id);

        assert_eq!(summary.event_id, event_id);
        assert!(!summary.missing_event_pool_ids);
        assert_eq!(summary.pool_summary.event_pool_ids_seen, 1);
        assert_eq!(summary.pool_summary.maps_matched, 1);
        assert!(
            manager
                .find_map(1, 0)
                .expect("test map 1")
                .map()
                .pool_data_like_cpp()
                .is_spawned_creature_like_cpp(spawn_id)
        );
        assert!(
            !manager
                .find_map(2, 0)
                .expect("test map 2")
                .map()
                .pool_data_like_cpp()
                .is_spawned_creature_like_cpp(spawn_id)
        );
    }

    #[test]
    fn game_event_pool_unspawn_uses_canonical_event_pool_ids_like_cpp() {
        let mut manager = wow_map::MapManager::default();
        manager.create_world_map(1, 0);
        manager.create_world_map(2, 0);
        let event_id = 8;
        let pool_id = 5322;
        let spawn_id = 532201;
        let game_event_pools =
            spawn_store_loader::GameEventPoolIdsLikeCpp::from_game_event_max_entry_like_cpp(Some(
                10,
            ))
            .with_pool_ids_for_event_like_cpp(event_id, [pool_id]);
        let metadata = canonical_spawn_metadata_with_store_pool_mgr_and_game_event_pools_like_cpp(
            SpawnStore::new(),
            pool_mgr_with_creature_pool_like_cpp(pool_id, 1, spawn_id),
            game_event_pools,
        );
        for map_id in [1, 2] {
            let map = manager
                .find_map_mut(map_id, 0)
                .expect("test canonical map")
                .map_mut();
            map.add_respawn_info_like_cpp(respawn_info_like_cpp(
                SpawnObjectType::Creature,
                spawn_id,
                532200,
            ));
            map.pool_data_mut_like_cpp()
                .add_spawn_like_cpp(SpawnObjectType::Creature, spawn_id, pool_id)
                .expect("test spawned creature pool data");
        }

        let summary =
            game_event_unspawn_pools_for_event_like_cpp(&mut manager, &metadata, event_id);

        assert_eq!(summary.event_id, event_id);
        assert!(!summary.missing_event_pool_ids);
        assert_eq!(summary.pool_summary.event_pool_ids_seen, 1);
        assert_eq!(summary.pool_summary.maps_matched, 1);
        assert!(
            !manager
                .find_map(1, 0)
                .expect("test map 1")
                .map()
                .pool_data_like_cpp()
                .is_spawned_creature_like_cpp(spawn_id)
        );
        assert!(
            manager
                .find_map(2, 0)
                .expect("test map 2")
                .map()
                .pool_data_like_cpp()
                .is_spawned_creature_like_cpp(spawn_id)
        );
    }

    #[test]
    fn game_event_pool_missing_event_id_is_noop_like_cpp() {
        let mut manager = wow_map::MapManager::default();
        manager.create_world_map(1, 0);
        let pool_id = 5323;
        let spawn_id = 532301;
        let metadata = canonical_spawn_metadata_with_store_pool_mgr_and_game_event_pools_like_cpp(
            SpawnStore::new(),
            pool_mgr_with_creature_pool_like_cpp(pool_id, 1, spawn_id),
            spawn_store_loader::GameEventPoolIdsLikeCpp::from_game_event_max_entry_like_cpp(Some(
                2,
            )),
        );
        manager
            .find_map_mut(1, 0)
            .expect("test map")
            .map_mut()
            .pool_data_mut_like_cpp()
            .add_spawn_like_cpp(SpawnObjectType::Creature, spawn_id, pool_id)
            .expect("test spawned creature pool data");
        let caches = empty_loaded_grid_creature_respawn_caches_like_cpp();

        let spawn_summary =
            game_event_spawn_pools_for_event_like_cpp(&mut manager, &metadata, &caches, 99);
        let unspawn_summary =
            game_event_unspawn_pools_for_event_like_cpp(&mut manager, &metadata, 99);

        assert!(spawn_summary.missing_event_pool_ids);
        assert_eq!(spawn_summary.pool_summary.event_pool_ids_seen, 0);
        assert!(unspawn_summary.missing_event_pool_ids);
        assert_eq!(unspawn_summary.pool_summary.event_pool_ids_seen, 0);
        assert!(
            manager
                .find_map(1, 0)
                .expect("test map")
                .map()
                .pool_data_like_cpp()
                .is_spawned_creature_like_cpp(spawn_id)
        );
    }

    #[test]
    fn game_event_pool_empty_event_id_list_is_noop_like_cpp() {
        let mut manager = wow_map::MapManager::default();
        manager.create_world_map(1, 0);
        let event_id = 1;
        let game_event_pools =
            spawn_store_loader::GameEventPoolIdsLikeCpp::from_game_event_max_entry_like_cpp(Some(
                2,
            ));
        let metadata = canonical_spawn_metadata_with_store_pool_mgr_and_game_event_pools_like_cpp(
            SpawnStore::new(),
            PoolMgrLikeCpp::new(),
            game_event_pools,
        );
        let caches = empty_loaded_grid_creature_respawn_caches_like_cpp();

        let spawn_summary =
            game_event_spawn_pools_for_event_like_cpp(&mut manager, &metadata, &caches, event_id);
        let unspawn_summary =
            game_event_unspawn_pools_for_event_like_cpp(&mut manager, &metadata, event_id);

        assert!(!spawn_summary.missing_event_pool_ids);
        assert_eq!(spawn_summary.pool_summary.event_pool_ids_seen, 0);
        assert!(!unspawn_summary.missing_event_pool_ids);
        assert_eq!(unspawn_summary.pool_summary.event_pool_ids_seen, 0);
        assert_eq!(spawn_summary.pool_summary.maps_matched, 0);
        assert_eq!(unspawn_summary.pool_summary.maps_matched, 0);
    }

    #[test]
    fn game_event_pool_spawn_filters_by_pool_template_map_id_like_cpp() {
        let mut manager = wow_map::MapManager::default();
        manager.create_world_map(1, 0);
        manager.create_world_map(2, 0);
        let pool_id = 5301;
        let spawn_id = 530101;
        let mut store = SpawnStore::new();
        store.add_object_spawn(
            &SpawnData {
                object_type: SpawnObjectType::Creature,
                spawn_id,
                map_id: 1,
                db_data: true,
                spawn_group: SpawnGroupTemplateData {
                    group_id: 5301,
                    name: "game-event-spawn".to_string(),
                    map_id: 1,
                    flags: SpawnGroupFlags::NONE,
                },
                id: 99,
                spawn_point: SpawnPosition::new(1_000.0, 1_000.0, 0.0, 0.0),
                phase_use_flags: 0,
                phase_id: 0,
                phase_group: 0,
                terrain_swap_map: 0,
                pool_id,
                spawn_time_secs: 0,
                spawn_difficulties: vec![1],
                script_id: 0,
                string_id: String::new(),
            },
            |_| false,
        );
        let metadata = canonical_spawn_metadata_with_store_and_pool_mgr_like_cpp(
            store,
            pool_mgr_with_creature_pool_like_cpp(pool_id, 1, spawn_id),
        );
        let caches = empty_loaded_grid_creature_respawn_caches_like_cpp();

        let summary = game_event_spawn_pools_like_cpp(&mut manager, &metadata, &caches, &[pool_id]);

        assert_eq!(summary.event_pool_ids_seen, 1);
        assert_eq!(summary.missing_pool_templates, 0);
        assert_eq!(summary.maps_matched, 1);
        assert_eq!(summary.pools_without_loaded_canonical_maps, 0);
        assert_eq!(summary.pool_spawn_actions_skipped_unloaded_grid, 1);
        assert_eq!(summary.pool_spawn_actions_blocked_loaded_grid, 0);
        assert!(summary.blocked_pool_plan_errors.is_empty());
        assert!(
            manager
                .find_map(1, 0)
                .expect("test map 1")
                .map()
                .pool_data_like_cpp()
                .is_spawned_creature_like_cpp(spawn_id)
        );
        assert!(
            !manager
                .find_map(2, 0)
                .expect("test map 2")
                .map()
                .pool_data_like_cpp()
                .is_spawned_creature_like_cpp(spawn_id)
        );
    }

    #[test]
    fn game_event_pool_spawn_missing_pool_template_is_counted_noop_like_cpp() {
        let mut manager = wow_map::MapManager::default();
        manager.create_world_map(1, 0);
        let metadata = canonical_spawn_metadata_with_pool_mgr_like_cpp(PoolMgrLikeCpp::new());
        let caches = empty_loaded_grid_creature_respawn_caches_like_cpp();

        let summary = game_event_spawn_pools_like_cpp(&mut manager, &metadata, &caches, &[5302]);

        assert_eq!(summary.event_pool_ids_seen, 1);
        assert_eq!(summary.missing_pool_templates, 1);
        assert_eq!(summary.maps_matched, 0);
        assert_eq!(summary.pool_spawn_actions_skipped_unloaded_grid, 0);
        assert!(summary.blocked_pool_plan_errors.is_empty());
        assert!(
            !manager
                .find_map(1, 0)
                .expect("test map")
                .map()
                .pool_data_like_cpp()
                .is_spawned_creature_like_cpp(530201)
        );
    }

    #[test]
    fn game_event_pool_spawn_loaded_grid_records_blocked_loader_and_unloaded_skips_loader_like_cpp()
    {
        let mut manager = wow_map::MapManager::default();
        manager.create_world_map(1, 0);
        let loaded_spawn_id = 530301;
        let unloaded_spawn_id = 530302;
        let mut store = SpawnStore::new();
        let group = SpawnGroupTemplateData {
            group_id: 5303,
            name: "game-event-spawn-loaded-grid".to_string(),
            map_id: 1,
            flags: SpawnGroupFlags::NONE,
        };
        store.add_object_spawn(
            &SpawnData {
                object_type: SpawnObjectType::Creature,
                spawn_id: loaded_spawn_id,
                map_id: 1,
                db_data: true,
                spawn_group: group.clone(),
                id: 99,
                spawn_point: SpawnPosition::new(0.0, 0.0, 0.0, 0.0),
                phase_use_flags: 0,
                phase_id: 0,
                phase_group: 0,
                terrain_swap_map: 0,
                pool_id: 5303,
                spawn_time_secs: 0,
                spawn_difficulties: vec![1],
                script_id: 0,
                string_id: String::new(),
            },
            |_| false,
        );
        store.add_object_spawn(
            &SpawnData {
                object_type: SpawnObjectType::Creature,
                spawn_id: unloaded_spawn_id,
                map_id: 1,
                db_data: true,
                spawn_group: group,
                id: 99,
                spawn_point: SpawnPosition::new(1_000.0, 1_000.0, 0.0, 0.0),
                phase_use_flags: 0,
                phase_id: 0,
                phase_group: 0,
                terrain_swap_map: 0,
                pool_id: 5303,
                spawn_time_secs: 0,
                spawn_difficulties: vec![1],
                script_id: 0,
                string_id: String::new(),
            },
            |_| false,
        );
        let mut pool_mgr = PoolMgrLikeCpp::new();
        pool_mgr.insert_template_like_cpp(5303, PoolTemplateDataLikeCpp::new(2, 1));
        let mut group = PoolGroupLikeCpp::with_pool_id(PoolMemberKindLikeCpp::Creature, 5303);
        group.add_entry_like_cpp(PoolObjectLikeCpp::new(loaded_spawn_id, 0.0), 2);
        group.add_entry_like_cpp(PoolObjectLikeCpp::new(unloaded_spawn_id, 0.0), 2);
        pool_mgr
            .insert_or_replace_group_like_cpp(PoolMemberKindLikeCpp::Creature, 5303, group)
            .expect("test creature pool group");
        manager
            .find_map_mut(1, 0)
            .expect("test map")
            .map_mut()
            .ensure_grid_loaded(&wow_map::cell_from_world(0.0, 0.0));
        let metadata = canonical_spawn_metadata_with_store_and_pool_mgr_like_cpp(store, pool_mgr);
        let caches = empty_loaded_grid_creature_respawn_caches_like_cpp();

        let summary = game_event_spawn_pools_like_cpp(&mut manager, &metadata, &caches, &[5303]);

        assert_eq!(summary.maps_matched, 1);
        assert_eq!(summary.pool_spawn_actions_blocked_loaded_grid, 1);
        assert_eq!(summary.pool_spawn_action_load_plans, 1);
        assert_eq!(summary.pool_spawn_actions_skipped_unloaded_grid, 1);
        assert_eq!(summary.executed_loaded_grid_respawns, 0);
        let map = manager.find_map(1, 0).expect("test map").map();
        assert!(
            map.pool_data_like_cpp()
                .is_spawned_creature_like_cpp(loaded_spawn_id)
        );
        assert!(
            map.pool_data_like_cpp()
                .is_spawned_creature_like_cpp(unloaded_spawn_id)
        );
    }

    #[test]
    fn game_event_pool_unspawn_filters_by_pool_template_map_id_like_cpp() {
        let mut manager = wow_map::MapManager::default();
        manager.create_world_map(1, 0);
        manager.create_world_map(2, 0);
        let pool_id = 5291;
        let spawn_id = 529101;
        let metadata = canonical_spawn_metadata_with_pool_mgr_like_cpp(
            pool_mgr_with_creature_pool_like_cpp(pool_id, 1, spawn_id),
        );

        for map_id in [1, 2] {
            let map = manager
                .find_map_mut(map_id, 0)
                .expect("test canonical map")
                .map_mut();
            map.add_respawn_info_like_cpp(respawn_info_like_cpp(
                SpawnObjectType::Creature,
                spawn_id,
                200,
            ));
            map.pool_data_mut_like_cpp()
                .add_spawn_like_cpp(SpawnObjectType::Creature, spawn_id, pool_id)
                .expect("test spawned creature pool data");
        }

        let summary = game_event_unspawn_pools_like_cpp(&mut manager, &metadata, &[pool_id]);

        assert_eq!(summary.event_pool_ids_seen, 1);
        assert_eq!(summary.missing_pool_templates, 0);
        assert_eq!(summary.maps_matched, 1);
        assert_eq!(summary.pools_without_loaded_canonical_maps, 0);
        assert_eq!(summary.pool_respawn_timers_removed, 0);
        assert_eq!(summary.pool_respawn_timers_missing, 0);
        assert!(summary.blocked_pool_plan_errors.is_empty());
        let map_1 = manager.find_map(1, 0).expect("test map 1").map();
        assert_eq!(
            map_1.get_respawn_time_like_cpp(SpawnObjectType::Creature, spawn_id),
            200
        );
        assert!(
            !map_1
                .pool_data_like_cpp()
                .is_spawned_creature_like_cpp(spawn_id)
        );
        let map_2 = manager.find_map(2, 0).expect("test map 2").map();
        assert_eq!(
            map_2.get_respawn_time_like_cpp(SpawnObjectType::Creature, spawn_id),
            200
        );
        assert!(
            map_2
                .pool_data_like_cpp()
                .is_spawned_creature_like_cpp(spawn_id)
        );
    }

    #[test]
    fn game_event_pool_unspawn_missing_pool_template_is_counted_noop_like_cpp() {
        let mut manager = wow_map::MapManager::default();
        manager.create_world_map(1, 0);
        let spawn_id = 529201;
        let map = manager.find_map_mut(1, 0).expect("test map").map_mut();
        map.add_respawn_info_like_cpp(respawn_info_like_cpp(
            SpawnObjectType::Creature,
            spawn_id,
            300,
        ));
        let metadata = canonical_spawn_metadata_with_pool_mgr_like_cpp(PoolMgrLikeCpp::new());

        let summary = game_event_unspawn_pools_like_cpp(&mut manager, &metadata, &[5292]);

        assert_eq!(summary.event_pool_ids_seen, 1);
        assert_eq!(summary.missing_pool_templates, 1);
        assert_eq!(summary.maps_matched, 0);
        assert_eq!(summary.pool_respawn_timers_removed, 0);
        assert!(summary.blocked_pool_plan_errors.is_empty());
        assert_eq!(
            manager
                .find_map(1, 0)
                .expect("test map")
                .map()
                .get_respawn_time_like_cpp(SpawnObjectType::Creature, spawn_id),
            300
        );
    }

    #[test]
    fn game_event_pool_unspawn_always_delete_removes_non_spawned_member_timer_like_cpp() {
        let mut manager = wow_map::MapManager::default();
        manager.create_world_map(1, 0);
        let pool_id = 5293;
        let spawn_id = 529301;
        let metadata = canonical_spawn_metadata_with_pool_mgr_like_cpp(
            pool_mgr_with_creature_pool_like_cpp(pool_id, 1, spawn_id),
        );
        manager
            .find_map_mut(1, 0)
            .expect("test map")
            .map_mut()
            .add_respawn_info_like_cpp(respawn_info_like_cpp(
                SpawnObjectType::Creature,
                spawn_id,
                400,
            ));

        let summary = game_event_unspawn_pools_like_cpp(&mut manager, &metadata, &[pool_id]);

        assert_eq!(summary.maps_matched, 1);
        assert_eq!(summary.pool_objects_removed, 0);
        assert_eq!(summary.pool_respawn_timers_removed, 1);
        assert_eq!(summary.pool_respawn_timers_missing, 0);
        assert_eq!(
            manager
                .find_map(1, 0)
                .expect("test map")
                .map()
                .get_respawn_time_like_cpp(SpawnObjectType::Creature, spawn_id),
            0
        );
    }

    #[test]
    fn world_config_resolution_prefers_lowercase_cpp_name() {
        let _guard = TEST_LOCK.lock().expect("test lock poisoned");
        let root = unique_temp_dir("world_config_resolution");
        let lower = root.join("worldserver.conf");
        let legacy = root.join("WorldServer.conf");

        fs::write(&lower, "WorldServerPort = 8085\n").expect("write lower failed");
        fs::write(&legacy, "WorldServerPort = 9000\n").expect("write legacy failed");

        let report = load_world_config_from(
            &[
                lower.to_str().expect("utf8 path"),
                legacy.to_str().expect("utf8 path"),
            ],
            root.join("worldserver.conf.d").to_str().expect("utf8 path"),
        )
        .expect("config should load");

        assert_eq!(report.candidate_index, 0);
        assert_eq!(wow_config::get_value::<u16>("WorldServerPort"), Some(8085));

        fs::remove_dir_all(root).expect("cleanup failed");
    }

    #[test]
    fn world_network_config_uses_resolved_world_configs() {
        let _guard = TEST_LOCK.lock().expect("test lock poisoned");
        wow_config::load_config_from_str(
            r#"
WorldServerPort = 70000
InstanceServerPort = 70001
Expansion = 9
"#,
        )
        .expect("config should load");

        let configs = wow_config::load_world_config_values();
        assert_eq!(world_config_u16(&configs, "CONFIG_PORT_WORLD", 8085), 4464);
        assert_eq!(
            world_config_u16(&configs, "CONFIG_PORT_INSTANCE", 8086),
            4465
        );
        assert_eq!(world_config_u8(&configs, "CONFIG_EXPANSION", 2), 9);
    }

    #[test]
    fn legacy_creature_global_runtime_config_is_numeric_opt_in_like_cpp() {
        let _guard = TEST_LOCK.lock().expect("test lock poisoned");

        wow_config::load_config_from_str("").expect("config should load");
        assert!(!legacy_creature_global_runtime_enabled_from_config_like_cpp());

        wow_config::load_config_from_str("RustyCore.LegacyCreatureGlobalRuntime = 0\n")
            .expect("config should load");
        assert!(!legacy_creature_global_runtime_enabled_from_config_like_cpp());

        wow_config::load_config_from_str("RustyCore.LegacyCreatureGlobalRuntime = 1\n")
            .expect("config should load");
        assert!(legacy_creature_global_runtime_enabled_from_config_like_cpp());
    }

    #[test]
    fn legacy_creature_aggro_config_uses_cpp_no_gray_aggro_keys_like_cpp() {
        let _guard = TEST_LOCK.lock().expect("test lock poisoned");

        wow_config::load_config_from_str(
            r#"
MaxPlayerLevel = 80
NoGrayAggro.Above = 80
NoGrayAggro.Below = 90
Rate.Creature.Aggro = 2
Visibility.Distance.Continents = 20
Visibility.Distance.Instances = 9999
Visibility.Distance.BG = 140
Visibility.Distance.Arenas = 150
"#,
        )
        .expect("config should load");
        let configs = wow_config::load_world_config_values();
        let config = legacy_creature_aggro_config_like_cpp(&configs);

        assert_eq!(config.no_gray_aggro_above, 80);
        // C++ clamps Below down to Above when Above > 0 && Above < Below.
        assert_eq!(config.no_gray_aggro_below, 80);
        assert_eq!(config.visibility_distance_continents, 90.0);
        assert_eq!(
            config.visibility_distance_instances,
            wow_entities::MAX_VISIBILITY_DISTANCE
        );
        assert_eq!(config.visibility_distance_battlegrounds, 140.0);
        assert_eq!(config.visibility_distance_arenas, 150.0);
    }

    #[test]
    fn loot_drop_rates_use_cpp_world_config_keys() {
        let _guard = TEST_LOCK.lock().expect("test lock poisoned");
        wow_config::load_config_from_str(
            r#"
Rate.Drop.Item.Poor = 0.5
Rate.Drop.Item.Rare = 3
Rate.Drop.Item.Referenced = 4
Rate.Drop.Item.ReferencedAmount = 2
Rate.Drop.Money = 6
"#,
        )
        .expect("config should load");

        let configs = wow_config::load_world_config_values();
        let rates = loot_drop_rates_like_cpp(&configs);
        assert_eq!(rates.item_poor, 0.5);
        assert_eq!(rates.item_normal, 1.0);
        assert_eq!(rates.item_rare, 3.0);
        assert_eq!(rates.item_referenced, 4.0);
        assert_eq!(rates.item_referenced_amount, 2.0);
        assert_eq!(rates.money, 6.0);
    }

    #[test]
    fn reputation_rates_use_cpp_world_config_keys() {
        let _guard = TEST_LOCK.lock().expect("test lock poisoned");
        wow_config::load_config_from_str(
            r#"
Rate.Reputation.Gain = 2
Rate.Reputation.LowLevel.Kill = 0.25
Rate.Reputation.LowLevel.Quest = 0.5
Rate.Reputation.RecruitAFriendBonus = 0.2
MaxRecruitAFriendBonusDistance = 45
"#,
        )
        .expect("config should load");

        let configs = wow_config::load_world_config_values();
        let rates = reputation_rates_like_cpp(&configs);
        assert_eq!(rates.gain, 2.0);
        assert_eq!(rates.low_level_kill, 0.25);
        assert_eq!(rates.low_level_quest, 0.5);
        assert_eq!(rates.recruit_a_friend_bonus, 0.2);
        assert_eq!(rates.recruit_a_friend_distance, 45.0);
    }

    #[test]
    fn repair_cost_rate_uses_cpp_world_config_key_and_clamps_negative_like_cpp() {
        let _guard = TEST_LOCK.lock().expect("test lock poisoned");
        wow_config::load_config_from_str("Rate.RepairCost = 2.5\n").expect("config should load");

        let configs = wow_config::load_world_config_values();
        assert_eq!(repair_cost_rate_like_cpp(&configs), 2.5);

        wow_config::load_config_from_str("Rate.RepairCost = -1\n").expect("config should load");
        let configs = wow_config::load_world_config_values();
        assert_eq!(repair_cost_rate_like_cpp(&configs), 0.0);
    }

    #[test]
    fn enable_ae_loot_uses_cpp_world_config_key() {
        let _guard = TEST_LOCK.lock().expect("test lock poisoned");
        wow_config::load_config_from_str("EnableAELoot = 1\n").expect("config should load");

        let configs = wow_config::load_world_config_values();
        assert!(world_config_bool(&configs, "CONFIG_ENABLE_AE_LOOT", false));
    }

    #[test]
    fn mmap_runtime_config_uses_cpp_world_config_key_and_data_dir() {
        let _guard = TEST_LOCK.lock().expect("test lock poisoned");
        wow_config::load_config_from_str(
            r#"
DataDir = "/srv/wow-data"
mmap.enablePathFinding = 0
"#,
        )
        .expect("config should load");

        let configs = wow_config::load_world_config_values();
        let mmap_config = mmap_runtime_config_like_cpp(&configs, HashSet::from([1]));
        assert_eq!(mmap_config.data_dir, "/srv/wow-data");
        assert!(!mmap_config.enabled);
        assert!(!mmap_config.pathfinding_enabled_for_map_like_cpp(0));
        assert!(!mmap_config.pathfinding_enabled_for_map_like_cpp(1));
    }

    #[test]
    fn mmap_runtime_config_applies_cpp_disable_mgr_map_gate() {
        let _guard = TEST_LOCK.lock().expect("test lock poisoned");
        wow_config::load_config_from_str("mmap.enablePathFinding = 1\n")
            .expect("config should load");

        let configs = wow_config::load_world_config_values();
        let mmap_config = mmap_runtime_config_like_cpp(&configs, HashSet::from([571]));
        assert!(mmap_config.pathfinding_enabled_for_map_like_cpp(0));
        assert!(!mmap_config.pathfinding_enabled_for_map_like_cpp(571));
    }

    #[test]
    fn canonical_spawn_group_initializer_applies_mapid_conditions_on_new_maps() {
        let metadata = Arc::new(Mutex::new(test_spawn_metadata([(10, 571), (11, 530)])));
        let condition_store = Arc::new(ConditionEntriesByTypeStore::from_conditions_like_cpp([
            mapid_condition(10, 571),
            mapid_condition(11, 571),
        ]));
        let mut manager = wow_map::MapManager::new(60_000, 10);
        install_canonical_spawn_group_initializer_like_cpp(
            &mut manager,
            Arc::clone(&metadata),
            condition_store,
            Arc::new(PersistedRespawnTimesLikeCpp::default()),
        );

        let group_571 = metadata
            .lock()
            .expect("test metadata lock")
            .spawn_group_templates()
            .get(&10)
            .expect("test group 10")
            .clone();
        let map_571 = manager.create_world_map(571, 0);
        assert!(
            map_571
                .map()
                .is_spawn_group_active_like_cpp(Some(&group_571))
        );

        let group_530 = metadata
            .lock()
            .expect("test metadata lock")
            .spawn_group_templates()
            .get(&11)
            .expect("test group 11")
            .clone();
        let map_530 = manager.create_world_map(530, 0);
        assert!(
            !map_530
                .map()
                .is_spawn_group_active_like_cpp(Some(&group_530))
        );
    }

    #[test]
    fn canonical_spawn_group_initializer_does_not_reexecute_for_existing_map() {
        let metadata = Arc::new(Mutex::new(test_spawn_metadata([(20, 571)])));
        let condition_store = Arc::new(ConditionEntriesByTypeStore::from_conditions_like_cpp([
            mapid_condition(20, 530),
        ]));
        let mut manager = wow_map::MapManager::new(60_000, 10);
        install_canonical_spawn_group_initializer_like_cpp(
            &mut manager,
            Arc::clone(&metadata),
            condition_store,
            Arc::new(PersistedRespawnTimesLikeCpp::default()),
        );

        let group = metadata
            .lock()
            .expect("test metadata lock")
            .spawn_group_templates()
            .get(&20)
            .expect("test group 20")
            .clone();
        let map = manager.create_world_map(571, 0);
        assert!(!map.map().is_spawn_group_active_like_cpp(Some(&group)));
        map.map_mut()
            .set_spawn_group_active_like_cpp(Some(&group), true);
        assert!(map.map().is_spawn_group_active_like_cpp(Some(&group)));

        let existing = manager.create_world_map(571, 0);
        assert!(existing.map().is_spawn_group_active_like_cpp(Some(&group)));
    }

    #[test]
    fn canonical_spawn_group_initializer_no_groups_is_noop() {
        let metadata = Arc::new(Mutex::new(test_spawn_metadata([])));
        let condition_store = Arc::new(ConditionEntriesByTypeStore::default());
        let mut manager = wow_map::MapManager::new(60_000, 10);
        install_canonical_spawn_group_initializer_like_cpp(
            &mut manager,
            metadata,
            condition_store,
            Arc::new(PersistedRespawnTimesLikeCpp::default()),
        );

        let map = manager.create_world_map(999, 0);
        assert!(
            map.map()
                .spawn_group_state()
                .toggled_spawn_group_ids()
                .is_empty()
        );
    }

    #[test]
    fn canonical_map_creation_loads_persisted_respawns_for_world_maps_before_spawn_groups() {
        let mut store = SpawnStore::new();
        let mut creature = test_spawn(77, 571);
        creature.id = 7001;
        creature.spawn_point = SpawnPosition::new(533.0, -533.0, 12.0, 1.0);
        store.add_object_spawn(&creature, |_| false);
        let mut gameobject = test_spawn(88, 571);
        gameobject.object_type = SpawnObjectType::GameObject;
        gameobject.id = 9001;
        gameobject.spawn_point = SpawnPosition::new(-100.0, 200.0, 13.0, 2.0);
        store.add_object_spawn(&gameobject, |_| false);
        let metadata = Arc::new(Mutex::new(
            super::spawn_store_loader::CanonicalSpawnMetadataLikeCpp::new(store, BTreeMap::new()),
        ));
        let mut snapshot = PersistedRespawnTimesLikeCpp::default();
        snapshot.push(
            wow_map::MapKey::new(571, 0),
            RespawnInfoLikeCpp {
                object_type: SpawnObjectType::Creature,
                spawn_id: 77,
                entry: 7001,
                respawn_time: 12345,
                grid_id: wow_map::compute_grid_coord(
                    creature.spawn_point.x,
                    creature.spawn_point.y,
                )
                .get_id(),
            },
        );
        snapshot.push(
            wow_map::MapKey::new(571, 0),
            RespawnInfoLikeCpp {
                object_type: SpawnObjectType::GameObject,
                spawn_id: 88,
                entry: 9001,
                respawn_time: 67890,
                grid_id: wow_map::compute_grid_coord(
                    gameobject.spawn_point.x,
                    gameobject.spawn_point.y,
                )
                .get_id(),
            },
        );
        let mut manager = wow_map::MapManager::new(60_000, 10);
        install_canonical_spawn_group_initializer_like_cpp(
            &mut manager,
            metadata,
            Arc::new(ConditionEntriesByTypeStore::default()),
            Arc::new(snapshot),
        );

        let map = manager.create_world_map(571, 0);
        assert_eq!(
            map.map()
                .get_respawn_time_like_cpp(SpawnObjectType::Creature, 77),
            12345
        );
        assert_eq!(
            map.map()
                .get_respawn_time_like_cpp(SpawnObjectType::GameObject, 88),
            67890
        );
        assert_eq!(
            map.map()
                .get_respawn_info_like_cpp(SpawnObjectType::Creature, 77)
                .expect("creature respawn loaded")
                .grid_id,
            wow_map::compute_grid_coord(creature.spawn_point.x, creature.spawn_point.y).get_id()
        );
    }

    #[test]
    fn canonical_map_creation_init_pools_before_persisted_respawns_and_spawn_groups() {
        let mut pool_mgr = PoolMgrLikeCpp::new();
        pool_mgr.insert_template_like_cpp(10, PoolTemplateDataLikeCpp::new(1, 571));
        let mut group = PoolGroupLikeCpp::with_pool_id(PoolMemberKindLikeCpp::GameObject, 10);
        group.add_entry_like_cpp(PoolObjectLikeCpp::new(88, 0.0), 1);
        pool_mgr
            .insert_or_replace_group_like_cpp(PoolMemberKindLikeCpp::GameObject, 10, group)
            .expect("test pool group");
        pool_mgr.add_auto_spawn_pool_like_cpp(571, 10);

        let mut store = SpawnStore::new();
        let mut gameobject = test_spawn(88, 571);
        gameobject.object_type = SpawnObjectType::GameObject;
        gameobject.id = 9001;
        gameobject.spawn_point = SpawnPosition::new(-100.0, 200.0, 13.0, 2.0);
        store.add_object_spawn(&gameobject, |_| false);
        let metadata = Arc::new(Mutex::new(
            super::spawn_store_loader::CanonicalSpawnMetadataLikeCpp::new(store, BTreeMap::new())
                .with_pool_mgr_like_cpp(pool_mgr),
        ));
        let mut snapshot = PersistedRespawnTimesLikeCpp::default();
        snapshot.push(
            wow_map::MapKey::new(571, 0),
            RespawnInfoLikeCpp {
                object_type: SpawnObjectType::GameObject,
                spawn_id: 88,
                entry: 9001,
                respawn_time: 67890,
                grid_id: wow_map::compute_grid_coord(
                    gameobject.spawn_point.x,
                    gameobject.spawn_point.y,
                )
                .get_id(),
            },
        );
        let mut manager = wow_map::MapManager::new(60_000, 10);
        install_canonical_spawn_group_initializer_like_cpp(
            &mut manager,
            metadata,
            Arc::new(ConditionEntriesByTypeStore::default()),
            Arc::new(snapshot),
        );

        let map = manager.create_world_map(571, 0);
        assert!(
            map.map()
                .pool_data_like_cpp()
                .is_spawned_gameobject_like_cpp(88)
        );
        assert_eq!(
            map.map()
                .pool_data_like_cpp()
                .get_spawned_objects_like_cpp(10),
            1
        );
        assert_eq!(
            map.map()
                .get_respawn_time_like_cpp(SpawnObjectType::GameObject, 88),
            67890
        );
    }

    #[test]
    fn canonical_map_creation_skips_persisted_respawns_for_dungeon_maps() {
        let metadata = Arc::new(Mutex::new(test_spawn_metadata([])));
        let mut snapshot = PersistedRespawnTimesLikeCpp::default();
        snapshot.push(
            wow_map::MapKey::new(571, 1),
            RespawnInfoLikeCpp {
                object_type: SpawnObjectType::Creature,
                spawn_id: 1,
                entry: 42,
                respawn_time: 12345,
                grid_id: 7,
            },
        );
        let mut manager = wow_map::MapManager::new(60_000, 10);
        install_canonical_spawn_group_initializer_like_cpp(
            &mut manager,
            metadata,
            Arc::new(ConditionEntriesByTypeStore::default()),
            Arc::new(snapshot),
        );

        let map = manager.create_map_entry(
            571,
            1,
            0,
            wow_map::ManagedMapKind::Dungeon {
                has_reset_schedule: false,
            },
        );
        assert_eq!(
            map.map()
                .get_respawn_time_like_cpp(SpawnObjectType::Creature, 1),
            0
        );
    }

    #[test]
    fn persisted_respawn_loader_rejects_invalid_areatrigger_and_missing_metadata_rows() {
        let metadata = test_spawn_metadata([]);
        let mut report = PersistedRespawnLoadReportLikeCpp::default();

        assert!(
            persisted_respawn_info_from_row_like_cpp(
                PersistedRespawnRowLikeCpp {
                    object_type_raw: 99,
                    spawn_id: 1,
                    respawn_time: 10,
                    map_id: 571,
                    instance_id: 0,
                },
                &metadata,
                &mut report,
            )
            .is_none()
        );
        assert!(
            persisted_respawn_info_from_row_like_cpp(
                PersistedRespawnRowLikeCpp {
                    object_type_raw: 256,
                    spawn_id: 1,
                    respawn_time: 10,
                    map_id: 571,
                    instance_id: 0,
                },
                &metadata,
                &mut report,
            )
            .is_none()
        );
        assert!(
            persisted_respawn_info_from_row_like_cpp(
                PersistedRespawnRowLikeCpp {
                    object_type_raw: SpawnObjectType::AreaTrigger as u16,
                    spawn_id: 1,
                    respawn_time: 10,
                    map_id: 571,
                    instance_id: 0,
                },
                &metadata,
                &mut report,
            )
            .is_none()
        );
        assert!(
            persisted_respawn_info_from_row_like_cpp(
                PersistedRespawnRowLikeCpp {
                    object_type_raw: SpawnObjectType::Creature as u16,
                    spawn_id: 404,
                    respawn_time: 10,
                    map_id: 571,
                    instance_id: 0,
                },
                &metadata,
                &mut report,
            )
            .is_none()
        );

        assert_eq!(report.rows, 4);
        assert_eq!(report.loaded, 0);
        assert_eq!(report.invalid_type, 2);
        assert_eq!(report.unsupported_area_trigger, 1);
        assert_eq!(report.missing_spawn_metadata, 1);
    }

    // C++ anchors for the focused condition-update helper tests:
    // - Maps/Map.cpp:666-688 (`Map::Update` respawn timer calls `UpdateSpawnGroupConditions`).
    // - Maps/Map.cpp:2471-2502 (`UpdateSpawnGroupConditions` branch order).
    // - Maps/Map.cpp:2427-2453 (map-owned spawn-group toggle state).
    // - GameObject.cpp:772-779 and 4256-4277 (capture-point paths trigger condition updates).
    #[test]
    fn spawn_group_condition_update_set_inactive_applies_for_failed_automatic_group() {
        let metadata = test_spawn_metadata([(30, 571)]);
        let condition_store =
            ConditionEntriesByTypeStore::from_conditions_like_cpp([mapid_condition(30, 530)]);
        let mut manager = wow_map::MapManager::new(60_000, 10);
        let group = metadata
            .spawn_group_templates()
            .get(&30)
            .expect("test group 30");
        let map = manager.create_world_map(571, 0);
        assert!(map.map().is_spawn_group_active_like_cpp(Some(group)));

        let outcomes = apply_canonical_spawn_group_condition_update_loaded_grid_records_like_cpp(
            map,
            &metadata,
            &condition_store,
            &empty_loaded_grid_creature_respawn_caches_like_cpp(),
        );

        assert_eq!(outcomes.len(), 1);
        assert_eq!(outcomes[0].group_id, 30);
        assert_eq!(
            outcomes[0].action,
            wow_map::map::SpawnGroupConditionActionLikeCpp::SetInactive
        );
        assert!(matches!(
            outcomes[0].applied_change,
            Some(
                wow_map::SpawnGroupActiveChange::Toggled
                    | wow_map::SpawnGroupActiveChange::ClearedToggle
            )
        ));
        assert!(!map.map().is_spawn_group_active_like_cpp(Some(group)));
    }

    #[test]
    fn spawn_group_condition_update_set_inactive_executes_spawn_active_seam_and_despawn_toggles() {
        let metadata = test_spawn_metadata_with_flags([
            (40, 571, SpawnGroupFlags::NONE),
            (41, 571, SpawnGroupFlags::DESPAWN_ON_CONDITION_FAILURE),
        ]);
        let condition_store = ConditionEntriesByTypeStore::from_conditions_like_cpp([
            mapid_condition(40, 571),
            mapid_condition(41, 530),
        ]);
        let mut manager = wow_map::MapManager::new(60_000, 10);
        let spawn_group = metadata
            .spawn_group_templates()
            .get(&40)
            .expect("test group 40");
        let despawn_group = metadata
            .spawn_group_templates()
            .get(&41)
            .expect("test group 41");
        let map = manager.create_world_map(571, 0);
        map.map_mut()
            .set_spawn_group_inactive_like_cpp(Some(spawn_group));
        assert!(!map.map().is_spawn_group_active_like_cpp(Some(spawn_group)));
        assert!(
            map.map()
                .is_spawn_group_active_like_cpp(Some(despawn_group))
        );

        let outcomes = apply_canonical_spawn_group_condition_update_loaded_grid_records_like_cpp(
            map,
            &metadata,
            &condition_store,
            &empty_loaded_grid_creature_respawn_caches_like_cpp(),
        );

        let spawn_outcome = outcomes
            .iter()
            .find(|outcome| outcome.group_id == 40)
            .expect("spawn outcome");
        assert_eq!(
            spawn_outcome.action,
            wow_map::map::SpawnGroupConditionActionLikeCpp::spawn_group_spawn_default()
        );
        assert_eq!(spawn_outcome.applied_change, None);
        let spawn = spawn_outcome
            .spawn_outcome
            .as_ref()
            .expect("condition-success spawn executes active-state seam");
        assert_eq!(spawn.blocked_missing_group, 0);
        assert_eq!(spawn.blocked_system_group, 0);
        assert_eq!(
            spawn.applied_active_change,
            Some(wow_map::SpawnGroupActiveChange::ClearedToggle)
        );
        let despawn_outcome = outcomes
            .iter()
            .find(|outcome| outcome.group_id == 41)
            .expect("despawn outcome");
        assert_eq!(
            despawn_outcome.action,
            wow_map::map::SpawnGroupConditionActionLikeCpp::condition_failure_despawn()
        );
        assert_eq!(despawn_outcome.applied_change, None);
        let despawn = despawn_outcome
            .despawn_outcome
            .expect("condition-failure despawn executes");
        assert_eq!(despawn.blocked_missing_group, 0);
        assert_eq!(despawn.blocked_system_group, 0);
        assert_eq!(
            despawn.applied_inactive_change,
            Some(wow_map::SpawnGroupActiveChange::Toggled)
        );
        assert!(map.map().is_spawn_group_active_like_cpp(Some(spawn_group)));
        assert!(
            !map.map()
                .is_spawn_group_active_like_cpp(Some(despawn_group))
        );
    }

    #[test]
    fn spawn_group_condition_update_set_inactive_no_groups_is_noop() {
        let metadata = test_spawn_metadata([]);
        let condition_store = ConditionEntriesByTypeStore::default();
        let mut manager = wow_map::MapManager::new(60_000, 10);
        let map = manager.create_world_map(999, 0);

        let outcomes = apply_canonical_spawn_group_condition_update_loaded_grid_records_like_cpp(
            map,
            &metadata,
            &condition_store,
            &empty_loaded_grid_creature_respawn_caches_like_cpp(),
        );

        assert!(outcomes.is_empty());
        assert!(
            map.map()
                .spawn_group_state()
                .toggled_spawn_group_ids()
                .is_empty()
        );
    }

    #[test]
    fn respawn_condition_scheduler_like_cpp_waits_fires_and_resets() {
        let mut scheduler = CanonicalRespawnConditionSchedulerLikeCpp::new(100);

        assert!(!scheduler.update(40));
        assert_eq!(scheduler.timer_ms(), 60);
        assert!(!scheduler.update(59));
        assert_eq!(scheduler.timer_ms(), 1);
        assert!(scheduler.update(1));
        assert_eq!(scheduler.timer_ms(), 100);
        assert!(scheduler.update(150));
        assert_eq!(scheduler.timer_ms(), 100);
        assert!(!scheduler.update(25));
        assert_eq!(scheduler.timer_ms(), 75);
    }

    #[test]
    fn game_event_scheduler_like_cpp_waits_fires_resets_and_installs_dynamic_delay() {
        let mut scheduler = CanonicalGameEventSchedulerLikeCpp::start_system(100);

        assert_eq!(scheduler.interval_ms(), 100);
        assert!(!scheduler.update(40));
        assert_eq!(scheduler.timer_ms(), 60);
        assert!(!scheduler.update(59));
        assert_eq!(scheduler.timer_ms(), 1);
        assert!(scheduler.update(1));
        assert_eq!(scheduler.timer_ms(), 100);

        scheduler.set_interval_and_reset(250);
        assert_eq!(scheduler.interval_ms(), 250);
        assert_eq!(scheduler.timer_ms(), 250);
        assert!(!scheduler.update(249));
        assert_eq!(scheduler.timer_ms(), 1);
        assert!(scheduler.update(1));
        assert_eq!(scheduler.timer_ms(), 250);

        scheduler.set_interval_and_reset(u64::from(u32::MAX) + 1);
        assert_eq!(scheduler.interval_ms(), u32::MAX);
        assert_eq!(scheduler.timer_ms(), u32::MAX);
        scheduler.set_interval_and_reset(0);
        assert_eq!(scheduler.interval_ms(), 1);
        assert_eq!(scheduler.timer_ms(), 1);
    }

    #[test]
    fn game_event_start_system_first_update_records_negative_spawn_then_init_update_skips_it() {
        let event = spawn_store_loader::GameEventDataLikeCpp {
            event_id: 1,
            start: 100,
            end: 1_000,
            occurence: 10,
            length: 2,
            ..spawn_store_loader::GameEventDataLikeCpp::default()
        };
        let store =
            spawn_store_loader::GameEventDataStoreLikeCpp::from_game_event_max_entry_like_cpp(
                Some(1),
            )
            .with_event_like_cpp(event);
        let mut metadata = spawn_store_loader::CanonicalSpawnMetadataLikeCpp::new(
            SpawnStore::default(),
            BTreeMap::new(),
        )
        .with_game_events_like_cpp(store);

        metadata.clear_active_game_events_like_cpp();
        let start_outcome = metadata.update_game_events_like_cpp(650, false, |_| false);
        assert_eq!(start_outcome.negative_spawn_event_ids, vec![-1]);
        assert_eq!(start_outcome.next_update_delay_millis, 51_000);
        let mut scheduler = CanonicalGameEventSchedulerLikeCpp::start_system(
            start_outcome.next_update_delay_millis,
        );
        assert_eq!(scheduler.interval_ms(), 51_000);

        assert!(scheduler.update(51_000));
        let tick_outcome = metadata.update_game_events_like_cpp(650, true, |_| false);
        scheduler.set_interval_and_reset(tick_outcome.next_update_delay_millis);
        assert!(tick_outcome.negative_spawn_event_ids.is_empty());
        assert_eq!(
            scheduler.interval_ms(),
            tick_outcome.next_update_delay_millis as u32
        );
    }

    fn game_event_world_state_metadata_like_cpp(
        max_event_entry: u32,
        events: &[spawn_store_loader::GameEventDataLikeCpp],
    ) -> spawn_store_loader::CanonicalSpawnMetadataLikeCpp {
        let store = events.iter().cloned().fold(
            spawn_store_loader::GameEventDataStoreLikeCpp::from_game_event_max_entry_like_cpp(
                Some(max_event_entry),
            ),
            spawn_store_loader::GameEventDataStoreLikeCpp::with_event_like_cpp,
        );
        spawn_store_loader::CanonicalSpawnMetadataLikeCpp::new(SpawnStore::new(), BTreeMap::new())
            .with_game_events_like_cpp(store)
    }

    fn game_event_world_state_start_outcome_like_cpp(
        event_id: u16,
    ) -> spawn_store_loader::GameEventUpdateOutcomeLikeCpp {
        spawn_store_loader::GameEventUpdateOutcomeLikeCpp {
            current_time_secs: 650,
            scanned_event_ids: vec![],
            check_outcomes: vec![],
            next_check_outcomes: vec![],
            queued_activation_event_ids: vec![event_id],
            queued_deactivation_event_ids: vec![],
            start_outcomes: vec![spawn_store_loader::GameEventStartOutcomeLikeCpp::Started(
                spawn_store_loader::GameEventStartSummaryLikeCpp {
                    event_id,
                    state_before_raw: 0,
                    state_after_raw: 0,
                    active_added: true,
                    active_was_present: false,
                    apply_new_event_requested: true,
                    save_world_event_state_requested: false,
                    force_game_event_update_requested: false,
                    completed: false,
                },
            )],
            stop_outcomes: vec![],
            negative_spawn_event_ids: vec![],
            world_nextphase_finished: vec![],
            world_conditions_save_requested: vec![],
            invalid_check_outcomes: vec![],
            invalid_next_check_outcomes: vec![],
            next_event_delay_secs_before_padding: 0,
            next_update_delay_millis: 1_000,
        }
    }

    fn empty_game_event_update_outcome_for_db_bridge_like_cpp()
    -> spawn_store_loader::GameEventUpdateOutcomeLikeCpp {
        spawn_store_loader::GameEventUpdateOutcomeLikeCpp {
            current_time_secs: 650,
            scanned_event_ids: vec![],
            check_outcomes: vec![],
            next_check_outcomes: vec![],
            queued_activation_event_ids: vec![],
            queued_deactivation_event_ids: vec![],
            start_outcomes: vec![],
            stop_outcomes: vec![],
            negative_spawn_event_ids: vec![],
            world_nextphase_finished: vec![],
            world_conditions_save_requested: vec![],
            invalid_check_outcomes: vec![],
            invalid_next_check_outcomes: vec![],
            next_event_delay_secs_before_padding: 0,
            next_update_delay_millis: 1_000,
        }
    }

    fn assert_game_event_save_operation_like_cpp(
        operation: &GameEventWorldEventStateDbOperationLikeCpp,
        event_id: u8,
        state: u8,
        next_start: i64,
    ) {
        assert_eq!(operation.event_id, event_id);
        assert_eq!(
            operation.kind,
            GameEventWorldEventStateDbOperationKindLikeCpp::Save
        );
        assert_eq!(operation.statements.len(), 2);
        assert_eq!(
            operation.statements[0].kind,
            GameEventWorldEventStateDbStatementKindLikeCpp::DelGameEventSave
        );
        assert_eq!(
            operation.statements[0].statement.sql(),
            "DELETE FROM game_event_save WHERE eventEntry = ?"
        );
        assert!(matches!(
            operation.statements[0].statement.params(),
            [wow_database::SqlParam::U8(id)] if id == &event_id
        ));
        assert_eq!(
            operation.statements[1].kind,
            GameEventWorldEventStateDbStatementKindLikeCpp::InsGameEventSave
        );
        assert_eq!(
            operation.statements[1].statement.sql(),
            "INSERT INTO game_event_save (eventEntry, state, next_start) VALUES (?, ?, ?)"
        );
        assert!(matches!(
            operation.statements[1].statement.params(),
            [wow_database::SqlParam::U8(id), wow_database::SqlParam::U8(actual_state), wow_database::SqlParam::I64(actual_next_start)]
                if id == &event_id
                    && actual_state == &state
                    && actual_next_start == &next_start
        ));
    }

    #[test]
    fn game_event_db_bridge_materializes_save_delete_insert_with_cpp_sql_and_zero_next_start() {
        let metadata = game_event_world_state_metadata_like_cpp(
            1,
            &[spawn_store_loader::GameEventDataLikeCpp {
                event_id: 1,
                state_raw: 2,
                next_start: 0,
                ..spawn_store_loader::GameEventDataLikeCpp::default()
            }],
        );
        let mut outcome = empty_game_event_update_outcome_for_db_bridge_like_cpp();
        outcome.start_outcomes = vec![spawn_store_loader::GameEventStartOutcomeLikeCpp::Started(
            spawn_store_loader::GameEventStartSummaryLikeCpp {
                event_id: 1,
                state_before_raw: 1,
                state_after_raw: 2,
                active_added: true,
                active_was_present: false,
                apply_new_event_requested: true,
                save_world_event_state_requested: true,
                force_game_event_update_requested: false,
                completed: false,
            },
        )];

        let summary =
            materialize_game_event_world_event_state_db_bridge_like_cpp(&outcome, &metadata);

        assert_eq!(summary.saves_queued, 1);
        assert_eq!(summary.operations.len(), 1);
        assert_game_event_save_operation_like_cpp(&summary.operations[0], 1, 2, 0);
    }

    #[test]
    fn game_event_db_bridge_materializes_world_nextphase_and_conditions_in_cpp_order() {
        let metadata = game_event_world_state_metadata_like_cpp(
            3,
            &[
                spawn_store_loader::GameEventDataLikeCpp {
                    event_id: 1,
                    state_raw: 3,
                    next_start: 10,
                    ..spawn_store_loader::GameEventDataLikeCpp::default()
                },
                spawn_store_loader::GameEventDataLikeCpp {
                    event_id: 2,
                    state_raw: 4,
                    next_start: 20,
                    ..spawn_store_loader::GameEventDataLikeCpp::default()
                },
            ],
        );
        let mut outcome = empty_game_event_update_outcome_for_db_bridge_like_cpp();
        outcome.world_nextphase_finished =
            vec![spawn_store_loader::GameEventWorldNextPhaseFinishedLikeCpp {
                event_id: 2,
                was_active_before_queue: true,
                state_before_raw: 1,
                state_after_raw: 4,
                next_start_before: 0,
                next_start_after: 20,
                save_state_requested: true,
            }];
        outcome.world_conditions_save_requested =
            vec![spawn_store_loader::GameEventWorldStateSaveEvidenceLikeCpp {
                event_id: 1,
                state_after_raw: 3,
                next_start_after: 10,
            }];

        let summary =
            materialize_game_event_world_event_state_db_bridge_like_cpp(&outcome, &metadata);

        assert_eq!(summary.saves_queued, 2);
        assert_eq!(summary.operations.len(), 2);
        assert_game_event_save_operation_like_cpp(&summary.operations[0], 2, 4, 20);
        assert_game_event_save_operation_like_cpp(&summary.operations[1], 1, 3, 10);
    }

    #[test]
    fn game_event_db_bridge_materializes_stop_delete_condition_saves_before_event_save() {
        let metadata = game_event_world_state_metadata_like_cpp(1, &[]);
        let mut outcome = empty_game_event_update_outcome_for_db_bridge_like_cpp();
        outcome.stop_outcomes = vec![spawn_store_loader::GameEventStopOutcomeLikeCpp::Stopped(
            spawn_store_loader::GameEventStopSummaryLikeCpp {
                event_id: 1,
                state_before_raw: 1,
                state_after_raw: 0,
                active_removed: true,
                active_was_present: true,
                unapply_event_requested: true,
                serverwide: true,
                condition_reset_requested: true,
                delete_world_event_state_requested: true,
                delete_condition_saves_requested: true,
            },
        )];

        let summary =
            materialize_game_event_world_event_state_db_bridge_like_cpp(&outcome, &metadata);

        assert_eq!(summary.deletes_queued, 1);
        assert_eq!(summary.condition_delete_rows_queued, 1);
        assert_eq!(summary.operations.len(), 1);
        let operation = &summary.operations[0];
        assert_eq!(
            operation.kind,
            GameEventWorldEventStateDbOperationKindLikeCpp::Delete
        );
        assert_eq!(operation.statements.len(), 2);
        assert_eq!(
            operation.statements[0].kind,
            GameEventWorldEventStateDbStatementKindLikeCpp::DelAllGameEventConditionSave
        );
        assert_eq!(
            operation.statements[0].statement.sql(),
            "DELETE FROM game_event_condition_save WHERE eventEntry = ?"
        );
        assert_eq!(
            operation.statements[1].kind,
            GameEventWorldEventStateDbStatementKindLikeCpp::DelGameEventSave
        );
        assert_eq!(
            operation.statements[1].statement.sql(),
            "DELETE FROM game_event_save WHERE eventEntry = ?"
        );
    }

    #[test]
    fn game_event_db_bridge_finished_no_overwrite_stop_without_delete_flags_is_noop() {
        let metadata = game_event_world_state_metadata_like_cpp(1, &[]);
        let mut outcome = empty_game_event_update_outcome_for_db_bridge_like_cpp();
        outcome.stop_outcomes = vec![spawn_store_loader::GameEventStopOutcomeLikeCpp::Stopped(
            spawn_store_loader::GameEventStopSummaryLikeCpp {
                event_id: 1,
                state_before_raw: 2,
                state_after_raw: 2,
                active_removed: false,
                active_was_present: true,
                unapply_event_requested: false,
                serverwide: true,
                condition_reset_requested: false,
                delete_world_event_state_requested: false,
                delete_condition_saves_requested: false,
            },
        )];

        let summary =
            materialize_game_event_world_event_state_db_bridge_like_cpp(&outcome, &metadata);

        assert_eq!(summary.deletes_queued, 0);
        assert_eq!(summary.condition_delete_rows_queued, 0);
        assert!(summary.operations.is_empty());
    }

    #[test]
    fn game_event_db_bridge_out_of_range_event_id_skips_without_panic() {
        let metadata = game_event_world_state_metadata_like_cpp(
            300,
            &[spawn_store_loader::GameEventDataLikeCpp {
                event_id: 300,
                state_raw: 1,
                next_start: 0,
                ..spawn_store_loader::GameEventDataLikeCpp::default()
            }],
        );
        let mut outcome = empty_game_event_update_outcome_for_db_bridge_like_cpp();
        outcome.world_conditions_save_requested =
            vec![spawn_store_loader::GameEventWorldStateSaveEvidenceLikeCpp {
                event_id: 300,
                state_after_raw: 1,
                next_start_after: 0,
            }];
        outcome.stop_outcomes = vec![spawn_store_loader::GameEventStopOutcomeLikeCpp::Stopped(
            spawn_store_loader::GameEventStopSummaryLikeCpp {
                event_id: 300,
                state_before_raw: 1,
                state_after_raw: 0,
                active_removed: true,
                active_was_present: true,
                unapply_event_requested: true,
                serverwide: true,
                condition_reset_requested: true,
                delete_world_event_state_requested: true,
                delete_condition_saves_requested: true,
            },
        )];

        let summary =
            materialize_game_event_world_event_state_db_bridge_like_cpp(&outcome, &metadata);

        assert_eq!(summary.saves_skipped_event_id_out_of_range, 1);
        assert_eq!(summary.deletes_skipped_event_id_out_of_range, 1);
        assert_eq!(summary.saves_queued, 0);
        assert_eq!(summary.deletes_queued, 0);
        assert!(summary.operations.is_empty());
    }

    #[test]
    fn game_event_quest_complete_db_bridge_materializes_condition_save_then_world_event_save() {
        let metadata = game_event_world_state_metadata_like_cpp(
            7,
            &[spawn_store_loader::GameEventDataLikeCpp {
                event_id: 7,
                state_raw: 3,
                next_start: 1_234,
                ..spawn_store_loader::GameEventDataLikeCpp::default()
            }],
        );
        let outcome = game_event_quest_complete_progressed_outcome_like_cpp(true, true);

        let summary = materialize_game_event_quest_complete_db_bridge_like_cpp(&outcome, &metadata);

        assert_eq!(summary.condition_save_updates_queued, 1);
        assert_eq!(summary.condition_save_updates_skipped_non_progress, 0);
        assert_eq!(summary.world_event_state_save_requested, 1);
        assert_eq!(summary.force_game_event_update_requested, 1);
        assert!(summary.save_world_event_state_requested);
        assert!(summary.force_game_event_update_requested_flag);
        assert_eq!(summary.operations.len(), 1);

        let operation = &summary.operations[0];
        assert_eq!(operation.event_id, 7);
        assert_eq!(operation.condition_id, 44);
        assert_eq!(operation.statements.len(), 2);
        assert_eq!(
            operation.statements[0].kind,
            GameEventQuestCompleteConditionSaveDbStatementKindLikeCpp::DelGameEventConditionSave
        );
        assert_eq!(
            operation.statements[0].statement.sql(),
            "DELETE FROM game_event_condition_save WHERE eventEntry = ? AND condition_id = ?"
        );
        assert_game_event_condition_save_delete_params_like_cpp(
            &operation.statements[0].statement,
            7,
            44,
        );
        assert_eq!(
            operation.statements[1].kind,
            GameEventQuestCompleteConditionSaveDbStatementKindLikeCpp::InsGameEventConditionSave
        );
        assert_eq!(
            operation.statements[1].statement.sql(),
            "INSERT INTO game_event_condition_save (eventEntry, condition_id, done) VALUES (?, ?, ?)"
        );
        assert_game_event_condition_save_insert_params_like_cpp(
            &operation.statements[1].statement,
            7,
            44,
            5.25,
        );

        assert_eq!(summary.world_event_state_summary.saves_queued, 1);
        assert_eq!(
            summary
                .world_event_state_summary
                .saves_skipped_missing_event,
            0
        );
        assert_eq!(
            summary
                .world_event_state_summary
                .saves_skipped_event_id_out_of_range,
            0
        );
        assert_eq!(summary.world_event_state_summary.operations.len(), 1);
        assert_game_event_save_operation_like_cpp(
            &summary.world_event_state_summary.operations[0],
            7,
            3,
            1_234,
        );
    }

    #[test]
    fn game_event_quest_complete_response_dto_includes_condition_and_world_event_flags_like_cpp() {
        let metadata = game_event_world_state_metadata_like_cpp(
            7,
            &[spawn_store_loader::GameEventDataLikeCpp {
                event_id: 7,
                state_raw: 3,
                next_start: 5_000,
                ..Default::default()
            }],
        );
        let outcome = game_event_quest_complete_progressed_outcome_like_cpp(true, true);

        let mut summary =
            materialize_game_event_quest_complete_db_bridge_like_cpp(&outcome, &metadata);
        summary.condition_save_updates_executed = 1;
        summary.world_event_state_summary.saves_executed = 1;
        let response = game_event_quest_complete_response_from_summary_like_cpp(1234, &summary);

        assert_eq!(response.quest_id, 1234);
        assert_eq!(response.condition_save_updates_queued, 1);
        assert_eq!(response.condition_save_updates_executed, 1);
        assert_eq!(response.condition_save_updates_failed, 0);
        assert_eq!(response.condition_save_updates_skipped_non_progress, 0);
        assert!(response.save_world_event_state_requested);
        assert_eq!(response.world_event_state_save_requested, 1);
        assert_eq!(response.world_event_state_saves_queued, 1);
        assert_eq!(response.world_event_state_saves_executed, 1);
        assert_eq!(response.world_event_state_saves_failed, 0);
        assert!(response.force_game_event_update_requested);
        assert_eq!(response.force_game_event_update_requests, 1);
        assert!(!response.processor_failed);
    }

    #[test]
    fn game_event_quest_complete_response_dto_reports_non_progress_noop_like_cpp() {
        let metadata = game_event_world_state_metadata_like_cpp(7, &[]);
        let outcome =
            spawn_store_loader::GameEventQuestCompleteOutcomeLikeCpp::MissingQuestMapping {
                quest_id: 9999,
            };

        let summary = materialize_game_event_quest_complete_db_bridge_like_cpp(&outcome, &metadata);
        let response = game_event_quest_complete_response_from_summary_like_cpp(9999, &summary);

        assert_eq!(response.quest_id, 9999);
        assert_eq!(response.condition_save_updates_queued, 0);
        assert_eq!(response.condition_save_updates_skipped_non_progress, 1);
        assert!(!response.save_world_event_state_requested);
        assert_eq!(response.world_event_state_saves_queued, 0);
        assert!(!response.force_game_event_update_requested);
        assert!(!response.processor_failed);
    }

    #[test]
    fn game_event_quest_complete_db_bridge_preserves_condition_save_without_world_event_save() {
        let metadata = game_event_world_state_metadata_like_cpp(
            7,
            &[spawn_store_loader::GameEventDataLikeCpp {
                event_id: 7,
                state_raw: 2,
                next_start: 0,
                ..spawn_store_loader::GameEventDataLikeCpp::default()
            }],
        );
        let outcome = game_event_quest_complete_progressed_outcome_like_cpp(false, false);

        let summary = materialize_game_event_quest_complete_db_bridge_like_cpp(&outcome, &metadata);

        assert_eq!(summary.condition_save_updates_queued, 1);
        assert_eq!(summary.operations.len(), 1);
        assert_eq!(summary.world_event_state_save_requested, 0);
        assert!(!summary.save_world_event_state_requested);
        assert_eq!(summary.world_event_state_summary.saves_queued, 0);
        assert!(summary.world_event_state_summary.operations.is_empty());
    }

    #[test]
    fn game_event_quest_complete_db_bridge_skips_world_event_save_when_metadata_missing() {
        let metadata = game_event_world_state_metadata_like_cpp(6, &[]);
        let outcome = game_event_quest_complete_progressed_outcome_like_cpp(true, true);

        let summary = materialize_game_event_quest_complete_db_bridge_like_cpp(&outcome, &metadata);

        assert_eq!(summary.condition_save_updates_queued, 1);
        assert_eq!(summary.operations.len(), 1);
        assert_eq!(summary.world_event_state_save_requested, 1);
        assert!(summary.save_world_event_state_requested);
        assert_eq!(summary.world_event_state_summary.saves_queued, 0);
        assert_eq!(
            summary
                .world_event_state_summary
                .saves_skipped_missing_event,
            1
        );
        assert!(summary.world_event_state_summary.operations.is_empty());
    }

    #[test]
    fn game_event_quest_complete_db_bridge_skips_missing_or_non_progress() {
        let metadata = game_event_world_state_metadata_like_cpp(
            7,
            &[spawn_store_loader::GameEventDataLikeCpp {
                event_id: 7,
                state_raw: 3,
                next_start: 1_234,
                ..spawn_store_loader::GameEventDataLikeCpp::default()
            }],
        );
        let missing =
            spawn_store_loader::GameEventQuestCompleteOutcomeLikeCpp::MissingQuestMapping {
                quest_id: 12_345,
            };
        let missing_summary =
            materialize_game_event_quest_complete_db_bridge_like_cpp(&missing, &metadata);
        assert_eq!(missing_summary.condition_save_updates_queued, 0);
        assert_eq!(
            missing_summary.condition_save_updates_skipped_non_progress,
            1
        );
        assert!(missing_summary.operations.is_empty());
        assert_eq!(missing_summary.world_event_state_summary.saves_queued, 0);
        assert!(
            missing_summary
                .world_event_state_summary
                .operations
                .is_empty()
        );

        let inactive = spawn_store_loader::GameEventQuestCompleteOutcomeLikeCpp::Progress(
            spawn_store_loader::GameEventConditionProgressOutcomeLikeCpp::InactiveEvent {
                event_id: 7,
            },
        );
        let inactive_summary =
            materialize_game_event_quest_complete_db_bridge_like_cpp(&inactive, &metadata);
        assert_eq!(inactive_summary.condition_save_updates_queued, 0);
        assert_eq!(
            inactive_summary.condition_save_updates_skipped_non_progress,
            1
        );
        assert!(inactive_summary.operations.is_empty());
        assert_eq!(inactive_summary.world_event_state_summary.saves_queued, 0);
        assert!(
            inactive_summary
                .world_event_state_summary
                .operations
                .is_empty()
        );

        let already_complete = spawn_store_loader::GameEventQuestCompleteOutcomeLikeCpp::Progress(
            spawn_store_loader::GameEventConditionProgressOutcomeLikeCpp::AlreadyComplete {
                event_id: 7,
                condition_id: 44,
                done: 10.0,
                req_num: 10.0,
            },
        );
        let complete_summary =
            materialize_game_event_quest_complete_db_bridge_like_cpp(&already_complete, &metadata);
        assert_eq!(complete_summary.condition_save_updates_queued, 0);
        assert_eq!(
            complete_summary.condition_save_updates_skipped_non_progress,
            1
        );
        assert!(complete_summary.operations.is_empty());
        assert_eq!(complete_summary.world_event_state_summary.saves_queued, 0);
        assert!(
            complete_summary
                .world_event_state_summary
                .operations
                .is_empty()
        );
    }

    #[test]
    fn game_event_world_state_no_holiday_action_is_represented_noop_like_cpp() {
        let mut metadata = game_event_world_state_metadata_like_cpp(
            1,
            &[spawn_store_loader::GameEventDataLikeCpp {
                event_id: 1,
                holiday_id: 0,
                length: 1,
                ..spawn_store_loader::GameEventDataLikeCpp::default()
            }],
        );
        let mut manager = wow_map::MapManager::default();
        let outcome = game_event_world_state_start_outcome_like_cpp(1);

        let summary = consume_game_event_live_update_side_effects_like_cpp(
            &mut manager,
            &mut metadata,
            &empty_loaded_grid_creature_respawn_caches_like_cpp(),
            None,
            None,
            None,
            &[1],
            &outcome,
            false,
        );

        assert!(
            summary
                .actions
                .contains(&GameEventLiveUpdateActionLikeCpp::UpdateWorldStates {
                    event_id: 1,
                    activate: true,
                })
        );
        assert_eq!(summary.update_world_states_actions, 1);
        assert_eq!(summary.update_world_states_no_holiday, 1);
        assert_eq!(summary.update_world_states_missing_event, 0);
        assert_eq!(summary.update_world_states_holiday_lookup_unrepresented, 0);
    }

    #[test]
    fn game_event_world_state_holiday_lookup_remains_unrepresented_like_cpp() {
        let mut metadata = game_event_world_state_metadata_like_cpp(
            1,
            &[spawn_store_loader::GameEventDataLikeCpp {
                event_id: 1,
                holiday_id: 283,
                length: 1,
                ..spawn_store_loader::GameEventDataLikeCpp::default()
            }],
        );
        let mut manager = wow_map::MapManager::default();
        let outcome = game_event_world_state_start_outcome_like_cpp(1);

        let summary = consume_game_event_live_update_side_effects_like_cpp(
            &mut manager,
            &mut metadata,
            &empty_loaded_grid_creature_respawn_caches_like_cpp(),
            None,
            None,
            None,
            &[1],
            &outcome,
            false,
        );

        assert_eq!(summary.update_world_states_actions, 1);
        assert_eq!(summary.update_world_states_no_holiday, 0);
        assert_eq!(summary.update_world_states_missing_event, 0);
        assert_eq!(summary.update_world_states_holiday_lookup_unrepresented, 1);
    }

    #[test]
    fn game_event_world_state_missing_event_is_counted_without_panic_like_cpp() {
        let mut metadata = game_event_world_state_metadata_like_cpp(0, &[]);
        let mut manager = wow_map::MapManager::default();
        let outcome = game_event_world_state_start_outcome_like_cpp(1);

        let summary = consume_game_event_live_update_side_effects_like_cpp(
            &mut manager,
            &mut metadata,
            &empty_loaded_grid_creature_respawn_caches_like_cpp(),
            None,
            None,
            None,
            &[],
            &outcome,
            false,
        );

        assert_eq!(summary.update_world_states_actions, 1);
        assert_eq!(summary.update_world_states_missing_event, 1);
        assert_eq!(summary.update_world_states_no_holiday, 0);
        assert_eq!(summary.update_world_states_holiday_lookup_unrepresented, 0);
    }

    #[test]
    fn game_event_world_state_holiday_set_value_activate_is_represented_like_cpp() {
        let metadata = game_event_world_state_metadata_like_cpp(
            1,
            &[spawn_store_loader::GameEventDataLikeCpp {
                event_id: 1,
                holiday_id: wow_data::HOLIDAY_CALL_TO_ARMS_AV_LIKE_CPP,
                length: 1,
                ..spawn_store_loader::GameEventDataLikeCpp::default()
            }],
        );
        let store =
            wow_data::BattlemasterListStore::from_entries([wow_data::BattlemasterListEntry {
                id: wow_data::BATTLEGROUND_AV_LIKE_CPP,
                holiday_world_state: 777,
            }]);

        let summary =
            game_event_update_world_states_like_cpp(&metadata, Some(&store), None, None, 1, true);

        assert_eq!(summary.update_world_states_set_value_represented, 1);
        assert_eq!(summary.update_world_states_last_world_state_id, Some(777));
        assert_eq!(summary.update_world_states_last_world_state_value, Some(1));
        assert_eq!(summary.update_world_states_holiday_lookup_unrepresented, 0);
    }

    #[test]
    fn game_event_world_state_holiday_set_value_deactivate_is_represented_like_cpp() {
        let metadata = game_event_world_state_metadata_like_cpp(
            1,
            &[spawn_store_loader::GameEventDataLikeCpp {
                event_id: 1,
                holiday_id: wow_data::HOLIDAY_CALL_TO_ARMS_AB_LIKE_CPP,
                length: 1,
                ..spawn_store_loader::GameEventDataLikeCpp::default()
            }],
        );
        let store =
            wow_data::BattlemasterListStore::from_entries([wow_data::BattlemasterListEntry {
                id: wow_data::BATTLEGROUND_AB_LIKE_CPP,
                holiday_world_state: 888,
            }]);

        let summary =
            game_event_update_world_states_like_cpp(&metadata, Some(&store), None, None, 1, false);

        assert_eq!(summary.update_world_states_set_value_represented, 1);
        assert_eq!(summary.update_world_states_last_world_state_id, Some(888));
        assert_eq!(summary.update_world_states_last_world_state_value, Some(0));
        assert_eq!(summary.update_world_states_holiday_lookup_unrepresented, 0);
    }

    #[test]
    fn game_event_world_state_live_consumer_propagates_holiday_lookup_counters_like_cpp() {
        fn consume_world_state_summary_like_cpp(
            metadata: &mut spawn_store_loader::CanonicalSpawnMetadataLikeCpp,
            battlemaster_list_store: Option<&wow_data::BattlemasterListStore>,
        ) -> GameEventLiveUpdateSideEffectSummaryLikeCpp {
            let mut manager = wow_map::MapManager::default();
            let outcome = game_event_world_state_start_outcome_like_cpp(1);
            consume_game_event_live_update_side_effects_like_cpp(
                &mut manager,
                metadata,
                &empty_loaded_grid_creature_respawn_caches_like_cpp(),
                battlemaster_list_store,
                None,
                None,
                &[1],
                &outcome,
                false,
            )
        }

        let mut missing_store_metadata = game_event_world_state_metadata_like_cpp(
            1,
            &[spawn_store_loader::GameEventDataLikeCpp {
                event_id: 1,
                holiday_id: wow_data::HOLIDAY_CALL_TO_ARMS_AV_LIKE_CPP,
                length: 1,
                ..spawn_store_loader::GameEventDataLikeCpp::default()
            }],
        );
        let missing_store_summary =
            consume_world_state_summary_like_cpp(&mut missing_store_metadata, None);
        assert_eq!(missing_store_summary.update_world_states_actions, 1);
        assert_eq!(missing_store_summary.update_world_states_store_missing, 1);
        assert_eq!(
            missing_store_summary.update_world_states_holiday_lookup_unrepresented,
            1
        );
        assert_eq!(
            missing_store_summary.update_world_states_battlemaster_list_missing,
            0
        );
        assert_eq!(
            missing_store_summary.update_world_states_holiday_world_state_zero,
            0
        );

        let missing_battlemaster_store = wow_data::BattlemasterListStore::from_entries([]);
        let mut missing_battlemaster_metadata = game_event_world_state_metadata_like_cpp(
            1,
            &[spawn_store_loader::GameEventDataLikeCpp {
                event_id: 1,
                holiday_id: wow_data::HOLIDAY_CALL_TO_ARMS_AV_LIKE_CPP,
                length: 1,
                ..spawn_store_loader::GameEventDataLikeCpp::default()
            }],
        );
        let missing_battlemaster_summary = consume_world_state_summary_like_cpp(
            &mut missing_battlemaster_metadata,
            Some(&missing_battlemaster_store),
        );
        assert_eq!(
            missing_battlemaster_summary.update_world_states_store_missing,
            0
        );
        assert_eq!(
            missing_battlemaster_summary.update_world_states_battlemaster_list_missing,
            1
        );
        assert_eq!(
            missing_battlemaster_summary.update_world_states_holiday_lookup_unrepresented,
            1
        );
        assert_eq!(
            missing_battlemaster_summary.update_world_states_holiday_world_state_zero,
            0
        );

        let zero_store =
            wow_data::BattlemasterListStore::from_entries([wow_data::BattlemasterListEntry {
                id: wow_data::BATTLEGROUND_AV_LIKE_CPP,
                holiday_world_state: 0,
            }]);
        let mut zero_metadata = game_event_world_state_metadata_like_cpp(
            1,
            &[spawn_store_loader::GameEventDataLikeCpp {
                event_id: 1,
                holiday_id: wow_data::HOLIDAY_CALL_TO_ARMS_AV_LIKE_CPP,
                length: 1,
                ..spawn_store_loader::GameEventDataLikeCpp::default()
            }],
        );
        let zero_summary =
            consume_world_state_summary_like_cpp(&mut zero_metadata, Some(&zero_store));
        assert_eq!(zero_summary.update_world_states_store_missing, 0);
        assert_eq!(
            zero_summary.update_world_states_battlemaster_list_missing,
            0
        );
        assert_eq!(zero_summary.update_world_states_holiday_world_state_zero, 1);
        assert_eq!(
            zero_summary.update_world_states_holiday_lookup_unrepresented,
            0
        );
        assert_eq!(zero_summary.update_world_states_set_value_represented, 0);
    }

    #[test]
    fn game_event_world_state_missing_battlemaster_store_is_explicit_skip_like_cpp() {
        let metadata = game_event_world_state_metadata_like_cpp(
            1,
            &[spawn_store_loader::GameEventDataLikeCpp {
                event_id: 1,
                holiday_id: wow_data::HOLIDAY_CALL_TO_ARMS_AV_LIKE_CPP,
                length: 1,
                ..spawn_store_loader::GameEventDataLikeCpp::default()
            }],
        );

        let summary = game_event_update_world_states_like_cpp(&metadata, None, None, None, 1, true);

        assert_eq!(summary.update_world_states_store_missing, 1);
        assert_eq!(summary.update_world_states_holiday_lookup_unrepresented, 1);
        assert_eq!(summary.update_world_states_set_value_represented, 0);
    }

    #[test]
    fn game_event_world_state_missing_or_zero_battlemaster_row_is_explicit_skip_like_cpp() {
        let metadata = game_event_world_state_metadata_like_cpp(
            1,
            &[spawn_store_loader::GameEventDataLikeCpp {
                event_id: 1,
                holiday_id: wow_data::HOLIDAY_CALL_TO_ARMS_AV_LIKE_CPP,
                length: 1,
                ..spawn_store_loader::GameEventDataLikeCpp::default()
            }],
        );
        let missing_store = wow_data::BattlemasterListStore::from_entries([]);
        let missing_summary = game_event_update_world_states_like_cpp(
            &metadata,
            Some(&missing_store),
            None,
            None,
            1,
            true,
        );
        assert_eq!(
            missing_summary.update_world_states_battlemaster_list_missing,
            1
        );
        assert_eq!(
            missing_summary.update_world_states_holiday_lookup_unrepresented,
            1
        );
        assert_eq!(missing_summary.update_world_states_set_value_represented, 0);

        let zero_store =
            wow_data::BattlemasterListStore::from_entries([wow_data::BattlemasterListEntry {
                id: wow_data::BATTLEGROUND_AV_LIKE_CPP,
                holiday_world_state: 0,
            }]);
        let zero_summary = game_event_update_world_states_like_cpp(
            &metadata,
            Some(&zero_store),
            None,
            None,
            1,
            true,
        );
        assert_eq!(zero_summary.update_world_states_holiday_world_state_zero, 1);
        assert_eq!(
            zero_summary.update_world_states_holiday_lookup_unrepresented,
            0
        );
        assert_eq!(zero_summary.update_world_states_set_value_represented, 0);
    }

    #[test]
    fn game_event_world_state_mgr_realm_default_change_global_message_represented_like_cpp() {
        let metadata = game_event_world_state_metadata_like_cpp(
            1,
            &[spawn_store_loader::GameEventDataLikeCpp {
                event_id: 1,
                holiday_id: wow_data::HOLIDAY_CALL_TO_ARMS_AV_LIKE_CPP,
                length: 1,
                ..spawn_store_loader::GameEventDataLikeCpp::default()
            }],
        );
        let store =
            wow_data::BattlemasterListStore::from_entries([wow_data::BattlemasterListEntry {
                id: wow_data::BATTLEGROUND_AV_LIKE_CPP,
                holiday_world_state: 777,
            }]);
        let mut world_state_mgr =
            spawn_store_loader::WorldStateMgrLikeCpp::from_templates_and_saved_values(
                [spawn_store_loader::WorldStateTemplateLikeCpp::realm_wide(
                    777, 0,
                )],
                [],
            );

        let summary = game_event_update_world_states_like_cpp(
            &metadata,
            Some(&store),
            Some(&mut world_state_mgr),
            None,
            1,
            true,
        );

        assert_eq!(summary.update_world_states_set_value_attempts, 1);
        assert_eq!(summary.update_world_states_realm_changed_or_inserted, 1);
        assert_eq!(summary.update_world_states_global_message_represented, 1);
        assert_eq!(summary.update_world_states_realm_unchanged_noop, 0);
        assert_eq!(summary.update_world_states_last_world_state_id, Some(777));
        assert_eq!(summary.update_world_states_last_world_state_value, Some(1));
        assert_eq!(world_state_mgr.realm_value_like_cpp(777), 1);
    }

    #[test]
    fn game_event_world_state_mgr_realm_same_value_is_noop_like_cpp() {
        let metadata = game_event_world_state_metadata_like_cpp(
            1,
            &[spawn_store_loader::GameEventDataLikeCpp {
                event_id: 1,
                holiday_id: wow_data::HOLIDAY_CALL_TO_ARMS_AV_LIKE_CPP,
                length: 1,
                ..spawn_store_loader::GameEventDataLikeCpp::default()
            }],
        );
        let store =
            wow_data::BattlemasterListStore::from_entries([wow_data::BattlemasterListEntry {
                id: wow_data::BATTLEGROUND_AV_LIKE_CPP,
                holiday_world_state: 778,
            }]);
        let mut world_state_mgr =
            spawn_store_loader::WorldStateMgrLikeCpp::from_templates_and_saved_values(
                [spawn_store_loader::WorldStateTemplateLikeCpp::realm_wide(
                    778, 1,
                )],
                [],
            );

        let summary = game_event_update_world_states_like_cpp(
            &metadata,
            Some(&store),
            Some(&mut world_state_mgr),
            None,
            1,
            true,
        );

        assert_eq!(summary.update_world_states_set_value_attempts, 1);
        assert_eq!(summary.update_world_states_realm_unchanged_noop, 1);
        assert_eq!(summary.update_world_states_global_message_represented, 0);
        assert_eq!(world_state_mgr.realm_value_like_cpp(778), 1);
    }

    #[test]
    fn game_event_world_state_mgr_missing_template_inserts_realm_value_like_cpp() {
        let metadata = game_event_world_state_metadata_like_cpp(
            1,
            &[spawn_store_loader::GameEventDataLikeCpp {
                event_id: 1,
                holiday_id: wow_data::HOLIDAY_CALL_TO_ARMS_AV_LIKE_CPP,
                length: 1,
                ..spawn_store_loader::GameEventDataLikeCpp::default()
            }],
        );
        let store =
            wow_data::BattlemasterListStore::from_entries([wow_data::BattlemasterListEntry {
                id: wow_data::BATTLEGROUND_AV_LIKE_CPP,
                holiday_world_state: 779,
            }]);
        let mut world_state_mgr = spawn_store_loader::WorldStateMgrLikeCpp::default();

        let summary = game_event_update_world_states_like_cpp(
            &metadata,
            Some(&store),
            Some(&mut world_state_mgr),
            None,
            1,
            true,
        );

        assert_eq!(summary.update_world_states_set_value_attempts, 1);
        assert_eq!(summary.update_world_states_realm_changed_or_inserted, 1);
        assert_eq!(summary.update_world_states_global_message_represented, 1);
        assert_eq!(world_state_mgr.realm_value_like_cpp(779), 1);
    }

    #[test]
    fn game_event_world_state_mgr_map_specific_null_map_is_unsupported_like_cpp() {
        let metadata = game_event_world_state_metadata_like_cpp(
            1,
            &[spawn_store_loader::GameEventDataLikeCpp {
                event_id: 1,
                holiday_id: wow_data::HOLIDAY_CALL_TO_ARMS_AV_LIKE_CPP,
                length: 1,
                ..spawn_store_loader::GameEventDataLikeCpp::default()
            }],
        );
        let store =
            wow_data::BattlemasterListStore::from_entries([wow_data::BattlemasterListEntry {
                id: wow_data::BATTLEGROUND_AV_LIKE_CPP,
                holiday_world_state: 780,
            }]);
        let mut world_state_mgr =
            spawn_store_loader::WorldStateMgrLikeCpp::from_templates_and_saved_values(
                [spawn_store_loader::WorldStateTemplateLikeCpp::map_specific(
                    780,
                    0,
                    [1],
                )],
                [],
            );

        let summary = game_event_update_world_states_like_cpp(
            &metadata,
            Some(&store),
            Some(&mut world_state_mgr),
            None,
            1,
            true,
        );

        assert_eq!(summary.update_world_states_set_value_attempts, 1);
        assert_eq!(
            summary.update_world_states_map_specific_no_map_unsupported,
            1
        );
        assert_eq!(summary.update_world_states_global_message_represented, 0);
        assert_eq!(world_state_mgr.realm_value_like_cpp(780), 0);
    }

    #[test]
    fn game_event_world_state_global_fanout_sends_update_to_active_players_like_cpp() {
        let metadata = game_event_world_state_metadata_like_cpp(
            1,
            &[spawn_store_loader::GameEventDataLikeCpp {
                event_id: 1,
                holiday_id: wow_data::HOLIDAY_CALL_TO_ARMS_AV_LIKE_CPP,
                length: 1,
                ..spawn_store_loader::GameEventDataLikeCpp::default()
            }],
        );
        let store =
            wow_data::BattlemasterListStore::from_entries([wow_data::BattlemasterListEntry {
                id: wow_data::BATTLEGROUND_AV_LIKE_CPP,
                holiday_world_state: 777,
            }]);
        let mut world_state_mgr =
            spawn_store_loader::WorldStateMgrLikeCpp::from_templates_and_saved_values(
                [spawn_store_loader::WorldStateTemplateLikeCpp::realm_wide(
                    777, 0,
                )],
                [],
            );
        let registry = PlayerRegistry::default();
        let (send_tx_a, send_rx_a) = flume::bounded(2);
        let (command_tx_a, _command_rx_a) = flume::bounded(1);
        let (send_tx_b, send_rx_b) = flume::bounded(2);
        let (command_tx_b, _command_rx_b) = flume::bounded(1);
        insert_player_broadcast_fixture_like_cpp(&registry, 7001, send_tx_a, command_tx_a);
        insert_player_broadcast_fixture_like_cpp(&registry, 7002, send_tx_b, command_tx_b);

        let summary = game_event_update_world_states_like_cpp(
            &metadata,
            Some(&store),
            Some(&mut world_state_mgr),
            Some(&registry),
            1,
            true,
        );

        let expected = wow_packet::packets::misc::UpdateWorldState {
            variable_id: 777,
            value: 1,
            hidden: false,
        }
        .to_bytes();
        assert_eq!(summary.update_world_states_realm_changed_or_inserted, 1);
        assert_eq!(summary.update_world_states_global_message_represented, 1);
        assert_eq!(summary.update_world_states_global_message_send_attempted, 2);
        assert_eq!(summary.update_world_states_global_message_send_queued, 2);
        assert_eq!(summary.update_world_states_global_message_send_failed, 0);
        assert_eq!(send_rx_a.try_recv().expect("player A update"), expected);
        assert_eq!(send_rx_b.try_recv().expect("player B update"), expected);
        assert!(send_rx_a.try_recv().is_err());
        assert!(send_rx_b.try_recv().is_err());
    }

    #[test]
    fn game_event_world_state_global_fanout_skips_not_in_world_player_like_cpp() {
        let registry = PlayerRegistry::default();
        let (in_world_tx, in_world_rx) = flume::bounded(1);
        let (in_world_command_tx, _in_world_command_rx) = flume::bounded(1);
        let (not_in_world_tx, not_in_world_rx) = flume::bounded(1);
        let (not_in_world_command_tx, _not_in_world_command_rx) = flume::bounded(1);
        insert_player_broadcast_fixture_with_in_world_like_cpp(
            &registry,
            7901,
            in_world_tx,
            in_world_command_tx,
            true,
        );
        insert_player_broadcast_fixture_with_in_world_like_cpp(
            &registry,
            7902,
            not_in_world_tx,
            not_in_world_command_tx,
            false,
        );
        let mut summary = GameEventLiveUpdateSideEffectSummaryLikeCpp::default();

        fanout_realm_update_world_state_to_player_sessions_like_cpp(
            Some(&registry),
            782,
            1,
            false,
            &mut summary,
        );

        let expected = wow_packet::packets::misc::UpdateWorldState {
            variable_id: 782,
            value: 1,
            hidden: false,
        }
        .to_bytes();
        assert_eq!(summary.update_world_states_global_message_send_attempted, 1);
        assert_eq!(summary.update_world_states_global_message_send_queued, 1);
        assert_eq!(summary.update_world_states_global_message_send_failed, 0);
        assert_eq!(
            summary.update_world_states_global_message_not_in_world_skipped,
            1
        );
        assert_eq!(
            in_world_rx.try_recv().expect("in-world player update"),
            expected
        );
        assert!(not_in_world_rx.try_recv().is_err());
    }

    #[test]
    fn game_event_world_state_global_fanout_preserves_signed_value_and_wrapped_variable_like_cpp() {
        let registry = PlayerRegistry::default();
        let (send_tx, send_rx) = flume::bounded(1);
        let (command_tx, _command_rx) = flume::bounded(1);
        insert_player_broadcast_fixture_like_cpp(&registry, 7003, send_tx, command_tx);
        let mut summary = GameEventLiveUpdateSideEffectSummaryLikeCpp::default();

        fanout_realm_update_world_state_to_player_sessions_like_cpp(
            Some(&registry),
            -1,
            -42,
            false,
            &mut summary,
        );

        let expected = wow_packet::packets::misc::UpdateWorldState {
            variable_id: u32::MAX,
            value: -42,
            hidden: false,
        }
        .to_bytes();
        assert_eq!(summary.update_world_states_global_message_send_attempted, 1);
        assert_eq!(summary.update_world_states_global_message_send_queued, 1);
        assert_eq!(
            send_rx.try_recv().expect("wrapped world-state update"),
            expected
        );
    }

    #[test]
    fn game_event_world_state_realm_unchanged_does_not_fanout_like_cpp() {
        let metadata = game_event_world_state_metadata_like_cpp(
            1,
            &[spawn_store_loader::GameEventDataLikeCpp {
                event_id: 1,
                holiday_id: wow_data::HOLIDAY_CALL_TO_ARMS_AV_LIKE_CPP,
                length: 1,
                ..spawn_store_loader::GameEventDataLikeCpp::default()
            }],
        );
        let store =
            wow_data::BattlemasterListStore::from_entries([wow_data::BattlemasterListEntry {
                id: wow_data::BATTLEGROUND_AV_LIKE_CPP,
                holiday_world_state: 778,
            }]);
        let mut world_state_mgr =
            spawn_store_loader::WorldStateMgrLikeCpp::from_templates_and_saved_values(
                [spawn_store_loader::WorldStateTemplateLikeCpp::realm_wide(
                    778, 1,
                )],
                [],
            );
        let registry = PlayerRegistry::default();
        let (send_tx, send_rx) = flume::bounded(1);
        let (command_tx, _command_rx) = flume::bounded(1);
        insert_player_broadcast_fixture_like_cpp(&registry, 7004, send_tx, command_tx);

        let summary = game_event_update_world_states_like_cpp(
            &metadata,
            Some(&store),
            Some(&mut world_state_mgr),
            Some(&registry),
            1,
            true,
        );

        assert_eq!(summary.update_world_states_realm_unchanged_noop, 1);
        assert_eq!(summary.update_world_states_global_message_send_attempted, 0);
        assert!(send_rx.try_recv().is_err());
    }

    #[test]
    fn game_event_world_state_realm_change_without_player_registry_is_counted_like_cpp() {
        let metadata = game_event_world_state_metadata_like_cpp(
            1,
            &[spawn_store_loader::GameEventDataLikeCpp {
                event_id: 1,
                holiday_id: wow_data::HOLIDAY_CALL_TO_ARMS_AV_LIKE_CPP,
                length: 1,
                ..spawn_store_loader::GameEventDataLikeCpp::default()
            }],
        );
        let store =
            wow_data::BattlemasterListStore::from_entries([wow_data::BattlemasterListEntry {
                id: wow_data::BATTLEGROUND_AV_LIKE_CPP,
                holiday_world_state: 779,
            }]);
        let mut world_state_mgr = spawn_store_loader::WorldStateMgrLikeCpp::default();

        let summary = game_event_update_world_states_like_cpp(
            &metadata,
            Some(&store),
            Some(&mut world_state_mgr),
            None,
            1,
            true,
        );

        assert_eq!(summary.update_world_states_realm_changed_or_inserted, 1);
        assert_eq!(summary.update_world_states_global_message_represented, 1);
        assert_eq!(
            summary.update_world_states_global_message_registry_missing,
            1
        );
        assert_eq!(summary.update_world_states_global_message_send_attempted, 0);
    }

    #[test]
    fn game_event_world_state_map_specific_null_map_does_not_fanout_like_cpp() {
        let metadata = game_event_world_state_metadata_like_cpp(
            1,
            &[spawn_store_loader::GameEventDataLikeCpp {
                event_id: 1,
                holiday_id: wow_data::HOLIDAY_CALL_TO_ARMS_AV_LIKE_CPP,
                length: 1,
                ..spawn_store_loader::GameEventDataLikeCpp::default()
            }],
        );
        let store =
            wow_data::BattlemasterListStore::from_entries([wow_data::BattlemasterListEntry {
                id: wow_data::BATTLEGROUND_AV_LIKE_CPP,
                holiday_world_state: 780,
            }]);
        let mut world_state_mgr =
            spawn_store_loader::WorldStateMgrLikeCpp::from_templates_and_saved_values(
                [spawn_store_loader::WorldStateTemplateLikeCpp::map_specific(
                    780,
                    0,
                    [1],
                )],
                [],
            );
        let registry = PlayerRegistry::default();
        let (send_tx, send_rx) = flume::bounded(1);
        let (command_tx, _command_rx) = flume::bounded(1);
        insert_player_broadcast_fixture_like_cpp(&registry, 7005, send_tx, command_tx);

        let summary = game_event_update_world_states_like_cpp(
            &metadata,
            Some(&store),
            Some(&mut world_state_mgr),
            Some(&registry),
            1,
            true,
        );

        assert_eq!(
            summary.update_world_states_map_specific_no_map_unsupported,
            1
        );
        assert_eq!(summary.update_world_states_global_message_send_attempted, 0);
        assert!(send_rx.try_recv().is_err());
    }

    #[test]
    fn game_event_world_state_global_fanout_counts_full_channel_failure_like_cpp() {
        let metadata = game_event_world_state_metadata_like_cpp(
            1,
            &[spawn_store_loader::GameEventDataLikeCpp {
                event_id: 1,
                holiday_id: wow_data::HOLIDAY_CALL_TO_ARMS_AV_LIKE_CPP,
                length: 1,
                ..spawn_store_loader::GameEventDataLikeCpp::default()
            }],
        );
        let store =
            wow_data::BattlemasterListStore::from_entries([wow_data::BattlemasterListEntry {
                id: wow_data::BATTLEGROUND_AV_LIKE_CPP,
                holiday_world_state: 781,
            }]);
        let mut world_state_mgr = spawn_store_loader::WorldStateMgrLikeCpp::default();
        let registry = PlayerRegistry::default();
        let (queued_tx, queued_rx) = flume::bounded(1);
        let (queued_command_tx, _queued_command_rx) = flume::bounded(1);
        let (full_tx, _full_rx) = flume::bounded(0);
        let (full_command_tx, _full_command_rx) = flume::bounded(1);
        insert_player_broadcast_fixture_like_cpp(&registry, 7006, queued_tx, queued_command_tx);
        insert_player_broadcast_fixture_like_cpp(&registry, 7007, full_tx, full_command_tx);

        let summary = game_event_update_world_states_like_cpp(
            &metadata,
            Some(&store),
            Some(&mut world_state_mgr),
            Some(&registry),
            1,
            true,
        );

        assert_eq!(summary.update_world_states_global_message_send_attempted, 2);
        assert_eq!(summary.update_world_states_global_message_send_queued, 1);
        assert_eq!(summary.update_world_states_global_message_send_failed, 1);
        assert!(queued_rx.try_recv().is_ok());
    }

    #[test]
    fn game_event_announce_start_order_before_spawn_and_stop_has_no_announce_like_cpp() {
        let metadata = game_event_world_state_metadata_like_cpp(
            3,
            &[spawn_store_loader::GameEventDataLikeCpp {
                event_id: 2,
                description: "Darkmoon Faire".to_string(),
                announce: 1,
                ..spawn_store_loader::GameEventDataLikeCpp::default()
            }],
        );
        let mut outcome = game_event_world_state_start_outcome_like_cpp(2);
        outcome.stop_outcomes = vec![spawn_store_loader::GameEventStopOutcomeLikeCpp::Stopped(
            spawn_store_loader::GameEventStopSummaryLikeCpp {
                event_id: 3,
                state_before_raw: 0,
                state_after_raw: 0,
                active_removed: true,
                active_was_present: true,
                unapply_event_requested: true,
                serverwide: false,
                condition_reset_requested: false,
                delete_world_event_state_requested: false,
                delete_condition_saves_requested: false,
            },
        )];

        let actions = game_event_live_update_actions_like_cpp(&metadata, &outcome, false);

        assert_eq!(
            actions.first(),
            Some(&GameEventLiveUpdateActionLikeCpp::AnnounceEvent {
                event_id: 2,
                description: "Darkmoon Faire".to_string(),
                description_len: "Darkmoon Faire".len(),
                announce: 1,
                config_event_announce: false,
            })
        );
        assert_eq!(
            actions.get(1),
            Some(&GameEventLiveUpdateActionLikeCpp::Spawn(2))
        );
        assert_eq!(
            actions
                .iter()
                .filter(|action| matches!(
                    action,
                    GameEventLiveUpdateActionLikeCpp::AnnounceEvent { .. }
                ))
                .count(),
            1
        );
        assert!(matches!(
            actions.iter().rev().take(8).last(),
            Some(GameEventLiveUpdateActionLikeCpp::RunSmartAIScripts {
                event_id: 3,
                activate: false
            })
        ));
    }

    #[test]
    fn game_event_announce_gating_matches_cpp_config_like_cpp() {
        let mut event = spawn_store_loader::GameEventDataLikeCpp {
            event_id: 1,
            description: "config gated".to_string(),
            ..spawn_store_loader::GameEventDataLikeCpp::default()
        };
        let outcome = game_event_world_state_start_outcome_like_cpp(1);

        event.announce = 1;
        let metadata = game_event_world_state_metadata_like_cpp(1, &[event.clone()]);
        assert!(matches!(
            game_event_live_update_actions_like_cpp(&metadata, &outcome, false).first(),
            Some(GameEventLiveUpdateActionLikeCpp::AnnounceEvent { announce: 1, .. })
        ));

        event.announce = 2;
        let metadata = game_event_world_state_metadata_like_cpp(1, &[event.clone()]);
        assert!(
            !game_event_live_update_actions_like_cpp(&metadata, &outcome, false)
                .iter()
                .any(|action| matches!(
                    action,
                    GameEventLiveUpdateActionLikeCpp::AnnounceEvent { .. }
                ))
        );
        assert!(matches!(
            game_event_live_update_actions_like_cpp(&metadata, &outcome, true).first(),
            Some(GameEventLiveUpdateActionLikeCpp::AnnounceEvent {
                announce: 2,
                config_event_announce: true,
                ..
            })
        ));

        for announce in [0_u8, 3_u8] {
            event.announce = announce;
            let metadata = game_event_world_state_metadata_like_cpp(1, &[event.clone()]);
            assert!(
                !game_event_live_update_actions_like_cpp(&metadata, &outcome, true)
                    .iter()
                    .any(|action| matches!(
                        action,
                        GameEventLiveUpdateActionLikeCpp::AnnounceEvent { .. }
                    ))
            );
        }
    }

    #[test]
    fn game_event_announce_consumption_fans_out_system_chat_like_cpp() {
        let mut manager = wow_map::MapManager::default();
        let mut metadata = game_event_world_state_metadata_like_cpp(
            1,
            &[spawn_store_loader::GameEventDataLikeCpp {
                event_id: 1,
                description: "Darkmoon Faire".to_string(),
                announce: 1,
                ..spawn_store_loader::GameEventDataLikeCpp::default()
            }],
        );
        let outcome = game_event_world_state_start_outcome_like_cpp(1);
        let registry = PlayerRegistry::new();
        let (send_tx_a, send_rx_a) = flume::bounded(2);
        let (command_tx_a, _command_rx_a) = flume::bounded(1);
        let (send_tx_b, send_rx_b) = flume::bounded(2);
        let (command_tx_b, _command_rx_b) = flume::bounded(1);
        insert_player_broadcast_fixture_like_cpp(&registry, 7101, send_tx_a, command_tx_a);
        insert_player_broadcast_fixture_like_cpp(&registry, 7102, send_tx_b, command_tx_b);

        let summary = consume_game_event_live_update_side_effects_like_cpp(
            &mut manager,
            &mut metadata,
            &empty_loaded_grid_creature_respawn_caches_like_cpp(),
            None,
            None,
            Some(&registry),
            &[1],
            &outcome,
            false,
        );

        let expected_packet = ChatPkt {
            msg_type: ChatMsg::System,
            language: 0,
            sender_guid: ObjectGuid::EMPTY,
            sender_name: String::new(),
            target_guid: ObjectGuid::EMPTY,
            target_name: String::new(),
            channel: String::new(),
            text: "|cffff0000[Event Message]: Darkmoon Faire|r".to_string(),
            virtual_realm: 0,
        };
        let mut expected_payload = wow_packet::world_packet::WorldPacket::new_empty();
        expected_packet.write(&mut expected_payload);
        assert_eq!(
            expected_payload.data()[0],
            0x00,
            "CHAT_MSG_SYSTEM must be 0x00 on wire"
        );
        assert_eq!(&expected_payload.data()[1..5], &[0x00, 0x00, 0x00, 0x00]);
        let expected = expected_packet.to_bytes();

        assert_eq!(summary.announce_event_actions, 1);
        assert_eq!(
            summary.announce_event_description_len_total,
            "Darkmoon Faire".len()
        );
        assert_eq!(summary.announce_event_world_text_represented, 1);
        assert_eq!(summary.announce_event_localization_unrepresented, 1);
        assert_eq!(summary.announce_event_in_world_filter_unrepresented, 0);
        assert_eq!(summary.announce_event_not_in_world_skipped, 0);
        assert_eq!(summary.announce_event_lines, 1);
        assert_eq!(summary.announce_event_send_attempted, 2);
        assert_eq!(summary.announce_event_send_queued, 2);
        assert_eq!(summary.announce_event_send_failed, 0);
        assert_eq!(summary.announce_event_world_text_unimplemented, 0);
        assert_eq!(summary.announce_event_session_fanout_unimplemented, 0);
        let received_a = send_rx_a.try_recv().expect("player A packet");
        let received_b = send_rx_b.try_recv().expect("player B packet");
        let payload_offset = 2; // ServerPacket::to_bytes prepends the u16 opcode.
        assert_eq!(
            received_a[payload_offset], 0x00,
            "received CHAT_MSG_SYSTEM must be 0x00 on wire"
        );
        assert_eq!(
            &received_a[payload_offset + 1..payload_offset + 5],
            &[0x00, 0x00, 0x00, 0x00]
        );
        assert_eq!(
            received_b[payload_offset], 0x00,
            "received CHAT_MSG_SYSTEM must be 0x00 on wire"
        );
        assert_eq!(
            &received_b[payload_offset + 1..payload_offset + 5],
            &[0x00, 0x00, 0x00, 0x00]
        );
        assert_eq!(received_a, expected);
        assert_eq!(received_b, expected);
        assert!(send_rx_a.try_recv().is_err());
        assert!(send_rx_b.try_recv().is_err());
        assert_eq!(summary.spawn_actions, 1);
    }

    #[test]
    fn game_event_announce_fanout_skips_not_in_world_player_like_cpp() {
        let registry = PlayerRegistry::default();
        let (in_world_tx, in_world_rx) = flume::bounded(1);
        let (in_world_command_tx, _in_world_command_rx) = flume::bounded(1);
        let (not_in_world_tx, not_in_world_rx) = flume::bounded(1);
        let (not_in_world_command_tx, _not_in_world_command_rx) = flume::bounded(1);
        insert_player_broadcast_fixture_with_in_world_like_cpp(
            &registry,
            7903,
            in_world_tx,
            in_world_command_tx,
            true,
        );
        insert_player_broadcast_fixture_with_in_world_like_cpp(
            &registry,
            7904,
            not_in_world_tx,
            not_in_world_command_tx,
            false,
        );
        let mut summary = GameEventLiveUpdateSideEffectSummaryLikeCpp::default();

        fanout_game_event_announcement_to_player_sessions_like_cpp(
            Some(&registry),
            "Darkmoon Faire",
            &mut summary,
        );

        let expected = ChatPkt {
            msg_type: ChatMsg::System,
            language: 0,
            sender_guid: ObjectGuid::EMPTY,
            sender_name: String::new(),
            target_guid: ObjectGuid::EMPTY,
            target_name: String::new(),
            channel: String::new(),
            text: "|cffff0000[Event Message]: Darkmoon Faire|r".to_string(),
            virtual_realm: 0,
        }
        .to_bytes();
        assert_eq!(summary.announce_event_world_text_represented, 1);
        assert_eq!(summary.announce_event_localization_unrepresented, 1);
        assert_eq!(summary.announce_event_in_world_filter_unrepresented, 0);
        assert_eq!(summary.announce_event_not_in_world_skipped, 1);
        assert_eq!(summary.announce_event_lines, 1);
        assert_eq!(summary.announce_event_send_attempted, 1);
        assert_eq!(summary.announce_event_send_queued, 1);
        assert_eq!(summary.announce_event_send_failed, 0);
        assert_eq!(
            in_world_rx.try_recv().expect("in-world player chat"),
            expected
        );
        assert!(not_in_world_rx.try_recv().is_err());
    }

    #[test]
    fn game_event_announce_missing_registry_counts_gap_without_panic_like_cpp() {
        let mut summary = GameEventLiveUpdateSideEffectSummaryLikeCpp::default();

        fanout_game_event_announcement_to_player_sessions_like_cpp(
            None,
            "Love is in the Air",
            &mut summary,
        );

        assert_eq!(summary.announce_event_world_text_represented, 1);
        assert_eq!(summary.announce_event_localization_unrepresented, 1);
        assert_eq!(summary.announce_event_registry_missing, 1);
        assert_eq!(summary.announce_event_lines, 1);
        assert_eq!(summary.announce_event_send_attempted, 0);
        assert_eq!(summary.announce_event_send_queued, 0);
        assert_eq!(summary.announce_event_send_failed, 0);
    }

    #[test]
    fn game_event_announce_newline_split_after_fallback_format_like_cpp() {
        assert_eq!(
            game_event_announcement_lines_like_cpp(""),
            vec!["|cffff0000[Event Message]: |r".to_string()]
        );
        assert_eq!(
            game_event_announcement_lines_like_cpp("\n\n"),
            vec!["|cffff0000[Event Message]: ".to_string(), "|r".to_string(),]
        );
        assert_eq!(
            game_event_announcement_lines_like_cpp("A\n\nB"),
            vec![
                "|cffff0000[Event Message]: A".to_string(),
                "B|r".to_string(),
            ]
        );
    }

    #[test]
    fn game_event_smart_ai_game_event_seasonal_start_stop_order_matches_cpp_live_update_like_cpp() {
        let metadata = game_event_world_state_metadata_like_cpp(
            3,
            &[spawn_store_loader::GameEventDataLikeCpp {
                event_id: 2,
                start: 100,
                occurence: 10,
                state_raw: spawn_store_loader::GameEventStateLikeCpp::Normal as u8,
                ..spawn_store_loader::GameEventDataLikeCpp::default()
            }],
        );
        let outcome = spawn_store_loader::GameEventUpdateOutcomeLikeCpp {
            current_time_secs: 1_350,
            scanned_event_ids: vec![],
            check_outcomes: vec![],
            next_check_outcomes: vec![],
            queued_activation_event_ids: vec![2],
            queued_deactivation_event_ids: vec![3],
            start_outcomes: vec![spawn_store_loader::GameEventStartOutcomeLikeCpp::Started(
                spawn_store_loader::GameEventStartSummaryLikeCpp {
                    event_id: 2,
                    state_before_raw: 0,
                    state_after_raw: 0,
                    active_added: true,
                    active_was_present: false,
                    apply_new_event_requested: true,
                    save_world_event_state_requested: false,
                    force_game_event_update_requested: false,
                    completed: false,
                },
            )],
            stop_outcomes: vec![spawn_store_loader::GameEventStopOutcomeLikeCpp::Stopped(
                spawn_store_loader::GameEventStopSummaryLikeCpp {
                    event_id: 3,
                    state_before_raw: 0,
                    state_after_raw: 0,
                    active_removed: true,
                    active_was_present: true,
                    unapply_event_requested: true,
                    serverwide: true,
                    condition_reset_requested: false,
                    delete_world_event_state_requested: false,
                    delete_condition_saves_requested: false,
                },
            )],
            negative_spawn_event_ids: vec![-1],
            world_nextphase_finished: vec![],
            world_conditions_save_requested: vec![],
            invalid_check_outcomes: vec![],
            invalid_next_check_outcomes: vec![],
            next_event_delay_secs_before_padding: 0,
            next_update_delay_millis: 1_000,
        };

        assert_eq!(
            game_event_live_update_actions_like_cpp(&metadata, &outcome, false),
            vec![
                GameEventLiveUpdateActionLikeCpp::Spawn(-1),
                GameEventLiveUpdateActionLikeCpp::Spawn(2),
                GameEventLiveUpdateActionLikeCpp::Unspawn(-2),
                GameEventLiveUpdateActionLikeCpp::ChangeEquipOrModel {
                    event_id: 2,
                    activate: true,
                },
                GameEventLiveUpdateActionLikeCpp::UpdateEventQuests {
                    event_id: 2,
                    activate: true,
                },
                GameEventLiveUpdateActionLikeCpp::UpdateWorldStates {
                    event_id: 2,
                    activate: true,
                },
                GameEventLiveUpdateActionLikeCpp::UpdateNpcFlags { event_id: 2 },
                GameEventLiveUpdateActionLikeCpp::UpdateNpcVendor {
                    event_id: 2,
                    activate: true,
                },
                GameEventLiveUpdateActionLikeCpp::RunSmartAIScripts {
                    event_id: 2,
                    activate: true,
                },
                GameEventLiveUpdateActionLikeCpp::ResetEventSeasonalQuests {
                    event_id: 2,
                    event_start_time: 1_300,
                },
                GameEventLiveUpdateActionLikeCpp::RunSmartAIScripts {
                    event_id: 3,
                    activate: false,
                },
                GameEventLiveUpdateActionLikeCpp::Unspawn(3),
                GameEventLiveUpdateActionLikeCpp::Spawn(-3),
                GameEventLiveUpdateActionLikeCpp::ChangeEquipOrModel {
                    event_id: 3,
                    activate: false,
                },
                GameEventLiveUpdateActionLikeCpp::UpdateEventQuests {
                    event_id: 3,
                    activate: false,
                },
                GameEventLiveUpdateActionLikeCpp::UpdateWorldStates {
                    event_id: 3,
                    activate: false,
                },
                GameEventLiveUpdateActionLikeCpp::UpdateNpcFlags { event_id: 3 },
                GameEventLiveUpdateActionLikeCpp::UpdateNpcVendor {
                    event_id: 3,
                    activate: false,
                },
            ]
        );
    }

    #[test]
    fn game_event_smart_ai_consume_no_maps_missing_event_noops_and_counts_action_like_cpp() {
        let mut manager = wow_map::MapManager::default();
        let mut metadata = game_event_world_state_metadata_like_cpp(0, &[]);
        let outcome = spawn_store_loader::GameEventUpdateOutcomeLikeCpp {
            current_time_secs: 650,
            scanned_event_ids: vec![],
            check_outcomes: vec![],
            next_check_outcomes: vec![],
            queued_activation_event_ids: vec![7],
            queued_deactivation_event_ids: vec![],
            start_outcomes: vec![spawn_store_loader::GameEventStartOutcomeLikeCpp::Started(
                spawn_store_loader::GameEventStartSummaryLikeCpp {
                    event_id: 7,
                    state_before_raw: 0,
                    state_after_raw: 0,
                    active_added: true,
                    active_was_present: false,
                    apply_new_event_requested: true,
                    save_world_event_state_requested: false,
                    force_game_event_update_requested: false,
                    completed: false,
                },
            )],
            stop_outcomes: vec![],
            negative_spawn_event_ids: vec![],
            world_nextphase_finished: vec![],
            world_conditions_save_requested: vec![],
            invalid_check_outcomes: vec![],
            invalid_next_check_outcomes: vec![],
            next_event_delay_secs_before_padding: 0,
            next_update_delay_millis: 1_000,
        };

        let summary = consume_game_event_live_update_side_effects_like_cpp(
            &mut manager,
            &mut metadata,
            &empty_loaded_grid_creature_respawn_caches_like_cpp(),
            None,
            None,
            None,
            &[7],
            &outcome,
            false,
        );

        assert_eq!(summary.run_smart_ai_actions, 1);
        assert_eq!(summary.run_smart_ai_maps_visited, 0);
        assert_eq!(summary.run_smart_ai_creature_candidates, 0);
        assert_eq!(summary.run_smart_ai_gameobject_candidates, 0);
        assert_eq!(summary.run_smart_ai_script_dispatch_unrepresented, 0);
    }

    #[test]
    fn game_event_seasonal_consume_records_evidence_without_player_or_db_mutation_like_cpp() {
        let mut manager = wow_map::MapManager::default();
        let mut metadata = game_event_world_state_metadata_like_cpp(
            7,
            &[spawn_store_loader::GameEventDataLikeCpp {
                event_id: 7,
                start: 100,
                occurence: 10,
                state_raw: spawn_store_loader::GameEventStateLikeCpp::Normal as u8,
                ..spawn_store_loader::GameEventDataLikeCpp::default()
            }],
        );
        let outcome = game_event_world_state_start_outcome_like_cpp(7);

        let mut summary = consume_game_event_live_update_side_effects_like_cpp(
            &mut manager,
            &mut metadata,
            &empty_loaded_grid_creature_respawn_caches_like_cpp(),
            None,
            None,
            None,
            &[7],
            &outcome,
            false,
        );

        assert_eq!(summary.reset_event_seasonal_quests_actions, 1);
        assert_eq!(summary.reset_event_seasonal_quests_event_start_time_zero, 0);
        assert_eq!(
            summary.reset_event_seasonal_quests_event_start_time_nonzero,
            1
        );
        assert_eq!(
            summary.reset_event_seasonal_quests_player_session_runtime_unimplemented,
            0
        );
        assert_eq!(
            summary.reset_event_seasonal_quests_player_session_registry_missing,
            0
        );
        assert_eq!(
            summary.reset_event_seasonal_quests_character_db_statement_unimplemented,
            0
        );
        assert_eq!(
            summary.reset_event_seasonal_quests_character_db_delete_queued,
            1
        );
        assert_eq!(
            summary
                .reset_event_seasonal_quests_character_db_delete_skipped_event_start_time_out_of_range,
            0
        );
        fanout_reset_event_seasonal_quests_to_player_sessions_after_db_delete_like_cpp(
            None,
            &mut summary,
        );
        assert_eq!(
            summary.reset_event_seasonal_quests_player_session_registry_missing,
            1
        );
        let [db_delete] = summary.reset_event_seasonal_quest_db_deletes.as_slice() else {
            panic!("expected exactly one seasonal quest DB delete")
        };
        assert_eq!(
            db_delete.statement.sql(),
            CharStatements::DEL_RESET_CHARACTER_QUESTSTATUS_SEASONAL_BY_EVENT.sql()
        );
        assert_eq!(
            db_delete.statement.sql(),
            "DELETE FROM character_queststatus_seasonal WHERE event = ? AND completedTime < ?"
        );
        let [
            SqlParam::U16(actual_event_id),
            SqlParam::I64(actual_event_start_time),
        ] = db_delete.statement.params()
        else {
            panic!(
                "expected seasonal quest DB delete params [U16(event_id), I64(event_start_time)]"
            )
        };
        assert_eq!(*actual_event_id, 7);
        assert_eq!(*actual_event_start_time, 100);
    }

    #[test]
    fn game_event_seasonal_db_delete_preserves_zero_event_start_time_like_cpp() {
        let mut manager = wow_map::MapManager::default();
        let mut metadata = game_event_world_state_metadata_like_cpp(
            8,
            &[spawn_store_loader::GameEventDataLikeCpp {
                event_id: 8,
                start: 100,
                occurence: 0,
                state_raw: spawn_store_loader::GameEventStateLikeCpp::Normal as u8,
                ..spawn_store_loader::GameEventDataLikeCpp::default()
            }],
        );
        let outcome = game_event_world_state_start_outcome_like_cpp(8);

        let mut summary = consume_game_event_live_update_side_effects_like_cpp(
            &mut manager,
            &mut metadata,
            &empty_loaded_grid_creature_respawn_caches_like_cpp(),
            None,
            None,
            None,
            &[8],
            &outcome,
            false,
        );

        assert_eq!(summary.reset_event_seasonal_quests_actions, 1);
        assert_eq!(summary.reset_event_seasonal_quests_event_start_time_zero, 1);
        assert_eq!(
            summary.reset_event_seasonal_quests_event_start_time_nonzero,
            0
        );
        assert_eq!(
            summary.reset_event_seasonal_quests_player_session_runtime_unimplemented,
            0
        );
        assert_eq!(
            summary.reset_event_seasonal_quests_player_session_registry_missing,
            0
        );
        assert_eq!(
            summary.reset_event_seasonal_quests_character_db_statement_unimplemented,
            0
        );
        assert_eq!(
            summary.reset_event_seasonal_quests_character_db_delete_queued,
            1
        );
        fanout_reset_event_seasonal_quests_to_player_sessions_after_db_delete_like_cpp(
            None,
            &mut summary,
        );
        assert_eq!(
            summary.reset_event_seasonal_quests_player_session_registry_missing,
            1
        );
        let [db_delete] = summary.reset_event_seasonal_quest_db_deletes.as_slice() else {
            panic!("expected exactly one seasonal quest DB delete")
        };
        let [
            SqlParam::U16(actual_event_id),
            SqlParam::I64(actual_event_start_time),
        ] = db_delete.statement.params()
        else {
            panic!(
                "expected seasonal quest DB delete params [U16(event_id), I64(event_start_time)]"
            )
        };
        assert_eq!(*actual_event_id, 8);
        assert_eq!(*actual_event_start_time, 0);
    }

    #[test]
    fn game_event_seasonal_post_db_delete_fanout_queues_session_command_like_cpp() {
        let mut manager = wow_map::MapManager::default();
        let mut metadata = game_event_world_state_metadata_like_cpp(
            9,
            &[spawn_store_loader::GameEventDataLikeCpp {
                event_id: 9,
                start: 345,
                occurence: 10,
                state_raw: spawn_store_loader::GameEventStateLikeCpp::Normal as u8,
                ..spawn_store_loader::GameEventDataLikeCpp::default()
            }],
        );
        let outcome = game_event_world_state_start_outcome_like_cpp(9);
        let registry = PlayerRegistry::default();
        let (send_tx, _send_rx) = flume::bounded(1);
        let (command_tx, command_rx) = flume::bounded(1);
        let player_guid = ObjectGuid::create_player(1, 9009);
        registry.insert(
            player_guid,
            PlayerBroadcastInfo {
                map_id: 0,
                instance_id: 0,
                position: wow_core::Position::ZERO,
                combat_reach: 0.0,
                liquid_status: 0,
                is_in_world: true,
                send_tx,
                command_tx,
                active_loot_rolls: Vec::new(),
                pass_on_group_loot: false,
                enchanting_skill: 0,
                is_alive: true,
                unit_flags: 0,
                unit_flags2: 0,
                unit_state: 0,
                is_game_master: false,
                is_contested_pvp: false,
                active_expansion: 2,
                pending_quest_sharing: None,
                known_spells: Vec::new(),
                active_quest_statuses: Default::default(),
                active_quest_objective_counts: Default::default(),
                rewarded_quests: Default::default(),
                daily_quests_completed: Default::default(),
                df_quests: Default::default(),
                faction_template_id: 0,
                reputation_standings: Vec::new(),
                reputation_state_flags: Vec::new(),
                forced_reputation_ranks: Vec::new(),
                forced_reputation_faction_ids: Vec::new(),
                inventory_item_counts: Default::default(),
                party_member_phase_states: Default::default(),
                player_name: "SeasonalTester".to_string(),
                account_id: 1,
                recruiter_id: 0,
                race: 1,
                class: 1,
                sex: 0,
                level: 1,
                gray_level: 0,
                display_id: 49,
                visible_items: [(0, 0, 0); 19],
            },
        );

        let mut summary = consume_game_event_live_update_side_effects_like_cpp(
            &mut manager,
            &mut metadata,
            &empty_loaded_grid_creature_respawn_caches_like_cpp(),
            None,
            None,
            None,
            &[9],
            &outcome,
            false,
        );

        assert!(command_rx.try_recv().is_err());
        assert_eq!(
            summary.reset_event_seasonal_quests_character_db_delete_queued,
            1
        );
        fanout_reset_event_seasonal_quests_to_player_sessions_after_db_delete_like_cpp(
            Some(&registry),
            &mut summary,
        );

        assert_eq!(
            summary.reset_event_seasonal_quests_player_session_send_attempted,
            1
        );
        assert_eq!(
            summary.reset_event_seasonal_quests_player_session_send_queued,
            1
        );
        assert_eq!(
            summary.reset_event_seasonal_quests_player_session_send_failed,
            0
        );
        let command = command_rx
            .try_recv()
            .expect("post-delete fanout command queued");
        let SessionCommand::ResetSeasonalQuestStatus(command) = command else {
            panic!("expected ResetSeasonalQuestStatus command")
        };
        assert_eq!(command.event_id, 9);
        assert_eq!(command.event_start_time, 345);
    }

    fn game_event_live_update_npc_vendor_record_like_cpp(
        spawn_id: wow_map::SpawnId,
        entry: u32,
        item: u32,
        vendor_type: u8,
    ) -> spawn_store_loader::GameEventNpcVendorRecordLikeCpp {
        spawn_store_loader::GameEventNpcVendorRecordLikeCpp {
            spawn_id,
            guid: spawn_id,
            entry,
            item,
            maxcount: 0,
            incrtime: 0,
            extended_cost: 0,
            vendor_type,
            item_type: vendor_type,
            bonus_list_ids: Vec::new(),
            player_condition_id: 0,
            ignore_filtering: false,
            event_npc_flag_low32: 0,
        }
    }

    fn game_event_live_update_npc_vendor_metadata_like_cpp(
        max_event_entry: u32,
        records: &[(u16, wow_map::SpawnId, u32, u32, u8)],
    ) -> spawn_store_loader::CanonicalSpawnMetadataLikeCpp {
        let mut vendors =
            spawn_store_loader::GameEventNpcVendorsLikeCpp::from_game_event_max_entry_like_cpp(
                Some(max_event_entry),
            );
        for (event_id, spawn_id, entry, item, vendor_type) in records {
            assert!(vendors.push_record_like_cpp(
                *event_id,
                game_event_live_update_npc_vendor_record_like_cpp(
                    *spawn_id,
                    *entry,
                    *item,
                    *vendor_type,
                ),
            ));
        }
        spawn_store_loader::CanonicalSpawnMetadataLikeCpp::new(SpawnStore::new(), BTreeMap::new())
            .with_game_event_npc_vendors_like_cpp(vendors)
    }

    #[test]
    fn game_event_live_update_npc_vendor_activation_adds_represented_cache_like_cpp() {
        let mut metadata = game_event_live_update_npc_vendor_metadata_like_cpp(
            1,
            &[(1, 100, 9001, 6000, 2), (1, 101, 9001, 6001, 2)],
        );

        let summary = game_event_update_npc_vendor_like_cpp(&mut metadata, 1, true);

        assert_eq!(summary.update_npc_vendor_records_seen, 2);
        assert_eq!(summary.update_npc_vendor_items_added, 2);
        assert_eq!(summary.update_npc_vendor_items_removed, 0);
        assert_eq!(
            metadata
                .game_event_active_npc_vendor_items_like_cpp(9001)
                .iter()
                .map(|record| record.item)
                .collect::<Vec<_>>(),
            vec![6000, 6001]
        );
    }

    #[test]
    fn game_event_live_update_npc_vendor_deactivation_removes_represented_cache_like_cpp() {
        let mut metadata = game_event_live_update_npc_vendor_metadata_like_cpp(
            2,
            &[(1, 100, 9001, 6000, 2), (2, 200, 9001, 6000, 2)],
        );
        game_event_update_npc_vendor_like_cpp(&mut metadata, 1, true);
        game_event_update_npc_vendor_like_cpp(&mut metadata, 2, true);

        let summary = game_event_update_npc_vendor_like_cpp(&mut metadata, 2, false);

        assert_eq!(summary.update_npc_vendor_records_seen, 1);
        assert_eq!(summary.update_npc_vendor_items_removed, 2);
        assert!(
            metadata
                .game_event_active_npc_vendor_items_like_cpp(9001)
                .is_empty()
        );
    }

    #[test]
    fn game_event_live_update_npc_vendor_missing_bucket_counted_like_cpp() {
        let mut metadata =
            game_event_live_update_npc_vendor_metadata_like_cpp(1, &[(1, 100, 9001, 6000, 2)]);

        let summary = game_event_update_npc_vendor_like_cpp(&mut metadata, 2, true);

        assert_eq!(summary.update_npc_vendor_missing_event_buckets, 1);
        assert_eq!(summary.update_npc_vendor_records_seen, 0);
        assert_eq!(summary.update_npc_vendor_actions, 0);
    }

    fn live_npc_flags_like_cpp(
        manager: &wow_map::MapManager,
        map_id: u32,
        spawn_id: wow_map::SpawnId,
    ) -> u32 {
        manager
            .find_map(map_id, 0)
            .expect("test map")
            .map()
            .get_creature_by_spawn_id_like_cpp(spawn_id)
            .expect("test live creature")
            .ai_ownership()
            .npc_flags
    }

    fn live_npc_flags2_like_cpp(
        manager: &wow_map::MapManager,
        map_id: u32,
        spawn_id: wow_map::SpawnId,
    ) -> u32 {
        manager
            .find_map(map_id, 0)
            .expect("test map")
            .map()
            .get_creature_by_spawn_id_like_cpp(spawn_id)
            .expect("test live creature")
            .ai_ownership()
            .npc_flags2
    }

    #[test]
    fn game_event_npc_flag_live_activation_applies_active_overlay_without_template_base_like_cpp() {
        let mut manager = wow_map::MapManager::default();
        manager.create_world_map(1, 0);
        manager.create_world_map(2, 0);
        let spawn_id = 547101;
        insert_live_creature_for_spawn_like_cpp(&mut manager, 1, spawn_id, 547101);
        let mut store = SpawnStore::new();
        add_spawn_data_like_cpp(&mut store, SpawnObjectType::Creature, spawn_id, 1);
        let mut npc_flags =
            spawn_store_loader::GameEventNpcFlagsLikeCpp::from_game_event_max_entry_like_cpp(Some(
                2,
            ));
        assert!(npc_flags.push_record_like_cpp(
            1,
            spawn_store_loader::GameEventNpcFlagRecordLikeCpp {
                spawn_id,
                npcflag: 0x20,
            },
        ));
        assert!(npc_flags.push_record_like_cpp(
            2,
            spawn_store_loader::GameEventNpcFlagRecordLikeCpp {
                spawn_id,
                npcflag: 0x1_0000_0040,
            },
        ));
        let metadata =
            spawn_store_loader::CanonicalSpawnMetadataLikeCpp::new(store, BTreeMap::new())
                .with_game_event_npc_flags_like_cpp(npc_flags);

        let summary =
            game_event_update_npc_flags_like_cpp(&mut manager, &metadata, None, 1, &[1, 2]);

        assert_eq!(summary.update_npc_flags_records_seen, 1);
        assert_eq!(summary.update_npc_flags_template_npcflag_missing, 1);
        assert_eq!(summary.update_npc_flags_maps_matched, 1);
        assert_eq!(summary.update_npc_flags_live_creatures_mutated, 1);
        assert_eq!(summary.update_npc_flags_low_applied, 1);
        assert_eq!(summary.update_npc_flags2_applied, 1);
        assert_eq!(live_npc_flags_like_cpp(&manager, 1, spawn_id), 0x60);
        assert_eq!(live_npc_flags2_like_cpp(&manager, 1, spawn_id), 0x1);
    }

    #[test]
    fn game_event_npc_flag_update_queues_visible_session_update_command_like_cpp() {
        let mut manager = wow_map::MapManager::default();
        manager.create_world_map(1, 0);
        let spawn_id = 547102;
        let creature_guid = test_guid_like_cpp(HighGuid::Creature, 547102, 99);
        insert_live_creature_for_spawn_like_cpp(&mut manager, 1, spawn_id, 547102);
        let mut store = SpawnStore::new();
        add_spawn_data_like_cpp(&mut store, SpawnObjectType::Creature, spawn_id, 1);
        let mut npc_flags =
            spawn_store_loader::GameEventNpcFlagsLikeCpp::from_game_event_max_entry_like_cpp(Some(
                1,
            ));
        assert!(npc_flags.push_record_like_cpp(
            1,
            spawn_store_loader::GameEventNpcFlagRecordLikeCpp {
                spawn_id,
                npcflag: 0x1_0000_0040,
            },
        ));
        let metadata =
            spawn_store_loader::CanonicalSpawnMetadataLikeCpp::new(store, BTreeMap::new())
                .with_game_event_npc_flags_like_cpp(npc_flags);
        let registry = PlayerRegistry::new();
        let (send_tx, send_rx) = flume::bounded(1);
        let (command_tx, command_rx) = flume::bounded(1);
        insert_player_broadcast_fixture_like_cpp(&registry, 7201, send_tx, command_tx);
        let player_guid = ObjectGuid::create_player(1, 7201);
        registry
            .get_mut(&player_guid)
            .expect("player registry row")
            .map_id = 1;

        let summary =
            game_event_update_npc_flags_like_cpp(&mut manager, &metadata, Some(&registry), 1, &[1]);

        assert_eq!(summary.update_npc_flags_live_creatures_mutated, 1);
        assert_eq!(summary.update_npc_flags_values_updates_built, 1);
        assert_eq!(summary.update_npc_flags_values_update_send_attempted, 1);
        assert_eq!(summary.update_npc_flags_values_update_send_queued, 1);
        assert!(send_rx.try_recv().is_err());
        let command = command_rx.try_recv().expect("visible update command");
        match command {
            SessionCommand::SendVisibleObjectValuesUpdate(command) => {
                assert_eq!(command.object_guid, creature_guid);
                assert_eq!(command.map_id, 1);
                assert!(!command.packet_bytes.is_empty());
            }
            other => panic!("unexpected command: {other:?}"),
        }
    }

    #[test]
    fn game_event_npc_flag_live_deactivation_recomputes_from_remaining_active_events_like_cpp() {
        let mut manager = wow_map::MapManager::default();
        manager.create_world_map(1, 0);
        let spawn_id = 547201;
        insert_live_creature_for_spawn_like_cpp(&mut manager, 1, spawn_id, 547201);
        let mut store = SpawnStore::new();
        add_spawn_data_like_cpp(&mut store, SpawnObjectType::Creature, spawn_id, 1);
        let mut npc_flags =
            spawn_store_loader::GameEventNpcFlagsLikeCpp::from_game_event_max_entry_like_cpp(Some(
                2,
            ));
        assert!(npc_flags.push_record_like_cpp(
            1,
            spawn_store_loader::GameEventNpcFlagRecordLikeCpp {
                spawn_id,
                npcflag: 0x20,
            },
        ));
        assert!(npc_flags.push_record_like_cpp(
            2,
            spawn_store_loader::GameEventNpcFlagRecordLikeCpp {
                spawn_id,
                npcflag: 0x40,
            },
        ));
        let metadata =
            spawn_store_loader::CanonicalSpawnMetadataLikeCpp::new(store, BTreeMap::new())
                .with_game_event_npc_flags_like_cpp(npc_flags);

        let start_summary =
            game_event_update_npc_flags_like_cpp(&mut manager, &metadata, None, 1, &[1, 2]);
        assert_eq!(start_summary.update_npc_flags_live_creatures_mutated, 1);
        assert_eq!(start_summary.update_npc_flags_template_npcflag_missing, 1);
        assert_eq!(live_npc_flags_like_cpp(&manager, 1, spawn_id), 0x60);

        let stop_summary =
            game_event_update_npc_flags_like_cpp(&mut manager, &metadata, None, 1, &[2]);

        assert_eq!(stop_summary.update_npc_flags_records_seen, 1);
        assert_eq!(stop_summary.update_npc_flags_template_npcflag_missing, 1);
        assert_eq!(stop_summary.update_npc_flags_live_creatures_mutated, 1);
        assert_eq!(live_npc_flags_like_cpp(&manager, 1, spawn_id), 0x40);
    }

    #[test]
    fn game_event_change_equip_or_model_missing_bucket_counted_once_like_cpp() {
        let mut manager = wow_map::MapManager::default();
        let mut metadata = spawn_store_loader::CanonicalSpawnMetadataLikeCpp::new(
            SpawnStore::new(),
            BTreeMap::new(),
        );

        let summary =
            game_event_change_equip_or_model_like_cpp(&mut manager, &mut metadata, 7, true);

        assert_eq!(summary.change_equip_or_model_missing_event_buckets, 1);
        assert_eq!(summary.change_equip_or_model_records_seen, 0);
        assert_eq!(summary.change_equip_or_model_records_applied, 0);
    }

    #[test]
    fn spawn_group_condition_update_tick_uses_effective_map_update_diff_only() {
        let metadata = test_spawn_metadata([(51, 571)]);
        let condition_store =
            ConditionEntriesByTypeStore::from_conditions_like_cpp([mapid_condition(51, 530)]);
        let mut manager = wow_map::MapManager::new(60_000, 10);
        let group = metadata
            .spawn_group_templates()
            .get(&51)
            .expect("test group 51")
            .clone();
        manager.create_world_map(571, 0);
        assert!(
            manager
                .find_map(571, 0)
                .unwrap()
                .map()
                .is_spawn_group_active_like_cpp(Some(&group))
        );
        let mut scheduler = CanonicalRespawnConditionSchedulerLikeCpp::new(10);

        let early = canonical_map_update_tick_set_inactive_like_cpp(
            &mut manager,
            9,
            &mut scheduler,
            &metadata,
            &condition_store,
            &empty_loaded_grid_creature_respawn_caches_like_cpp(),
        );
        assert!(early.is_none());
        assert_eq!(scheduler.timer_ms(), 10);
        assert!(
            manager
                .find_map(571, 0)
                .unwrap()
                .map()
                .is_spawn_group_active_like_cpp(Some(&group))
        );

        let summary = canonical_map_update_tick_set_inactive_like_cpp(
            &mut manager,
            1,
            &mut scheduler,
            &metadata,
            &condition_store,
            &empty_loaded_grid_creature_respawn_caches_like_cpp(),
        )
        .expect("map update accumulates 10ms and scheduler fires with effective diff");
        assert_eq!(summary.maps_evaluated, 1);
        assert_eq!(summary.outcomes, 1);
        assert_eq!(summary.applied_set_inactive, 1);
        assert_eq!(summary.planned_spawn, 0);
        assert_eq!(summary.planned_despawn, 0);
        assert_eq!(scheduler.timer_ms(), 10);
        assert!(
            !manager
                .find_map(571, 0)
                .unwrap()
                .map()
                .is_spawn_group_active_like_cpp(Some(&group))
        );
    }

    #[test]
    fn spawn_group_condition_update_tick_applies_set_inactive_only_when_scheduler_fires() {
        let metadata = test_spawn_metadata([(50, 571)]);
        let condition_store =
            ConditionEntriesByTypeStore::from_conditions_like_cpp([mapid_condition(50, 530)]);
        let mut manager = wow_map::MapManager::new(60_000, 1);
        let group = metadata
            .spawn_group_templates()
            .get(&50)
            .expect("test group 50")
            .clone();
        manager.create_world_map(571, 0);
        assert!(
            manager
                .find_map(571, 0)
                .unwrap()
                .map()
                .is_spawn_group_active_like_cpp(Some(&group))
        );
        let mut scheduler = CanonicalRespawnConditionSchedulerLikeCpp::new(100);

        let early = canonical_map_update_tick_set_inactive_like_cpp(
            &mut manager,
            99,
            &mut scheduler,
            &metadata,
            &condition_store,
            &empty_loaded_grid_creature_respawn_caches_like_cpp(),
        );
        assert!(early.is_none());
        assert!(
            manager
                .find_map(571, 0)
                .unwrap()
                .map()
                .is_spawn_group_active_like_cpp(Some(&group))
        );

        let summary = canonical_map_update_tick_set_inactive_like_cpp(
            &mut manager,
            1,
            &mut scheduler,
            &metadata,
            &condition_store,
            &empty_loaded_grid_creature_respawn_caches_like_cpp(),
        )
        .expect("scheduler fires at interval");
        assert_eq!(summary.maps_evaluated, 1);
        assert_eq!(summary.outcomes, 1);
        assert_eq!(summary.applied_set_inactive, 1);
        assert_eq!(summary.planned_spawn, 0);
        assert_eq!(summary.planned_despawn, 0);
        assert_eq!(scheduler.timer_ms(), 100);
        assert!(
            !manager
                .find_map(571, 0)
                .unwrap()
                .map()
                .is_spawn_group_active_like_cpp(Some(&group))
        );
    }

    #[test]
    fn respawn_db_delete_statement_like_cpp_uses_char_del_respawn_params_without_truncation() {
        let outcome = queue_respawn_db_delete_like_cpp(
            wow_map::ManagedMapKind::World,
            571,
            0,
            SpawnObjectType::Creature,
            1,
        );
        let RespawnDbDeleteQueueOutcomeLikeCpp::Queued(delete) = outcome else {
            panic!("world map delete should queue");
        };

        assert_eq!(delete.object_type, SpawnObjectType::Creature);
        assert_eq!(delete.spawn_id, 1);
        assert_eq!(delete.map_id, 571);
        assert_eq!(delete.instance_id, 0);
        assert_eq!(delete.statement.sql(), CharStatements::DEL_RESPAWN.sql());
        assert_del_respawn_params_like_cpp(&delete.statement, 0, 1, 571, 0);
    }

    #[test]
    fn respawn_db_delete_statement_like_cpp_skips_non_world_and_invalid_map_id() {
        let non_world = queue_respawn_db_delete_like_cpp(
            wow_map::ManagedMapKind::Dungeon {
                has_reset_schedule: false,
            },
            571,
            1,
            SpawnObjectType::GameObject,
            2,
        );
        assert!(matches!(
            non_world,
            RespawnDbDeleteQueueOutcomeLikeCpp::SkippedNonWorldMap
        ));

        let invalid_map_id = queue_respawn_db_delete_like_cpp(
            wow_map::ManagedMapKind::World,
            u32::from(u16::MAX) + 1,
            0,
            SpawnObjectType::Creature,
            1,
        );
        assert!(matches!(
            invalid_map_id,
            RespawnDbDeleteQueueOutcomeLikeCpp::SkippedInvalidMapId
        ));
    }

    #[test]
    fn respawn_db_save_statement_like_cpp_uses_char_rep_respawn_params_without_truncation() {
        let info = RespawnInfoLikeCpp {
            object_type: SpawnObjectType::GameObject,
            spawn_id: u64::from(u32::MAX) + 17,
            entry: 9001,
            respawn_time: 1_777_777_777,
            grid_id: 7,
        };
        let outcome = queue_respawn_db_save_like_cpp(
            wow_map::ManagedMapKind::World,
            571,
            u32::MAX,
            info.clone(),
        );
        let RespawnDbSaveQueueOutcomeLikeCpp::Queued(save) = outcome else {
            panic!("world map save should queue");
        };

        assert_eq!(save.object_type, SpawnObjectType::GameObject);
        assert_eq!(save.spawn_id, info.spawn_id);
        assert_eq!(save.respawn_time, info.respawn_time);
        assert_eq!(save.map_id, 571);
        assert_eq!(save.instance_id, u32::MAX);
        assert_eq!(save.statement.sql(), CharStatements::REP_RESPAWN.sql());
        assert_rep_respawn_params_like_cpp(
            &save.statement,
            1,
            info.spawn_id,
            info.respawn_time,
            571,
            u32::MAX,
        );
    }

    #[test]
    fn respawn_db_save_statement_like_cpp_skips_non_world_and_invalid_map_id() {
        let info = RespawnInfoLikeCpp {
            object_type: SpawnObjectType::Creature,
            spawn_id: 1,
            entry: 42,
            respawn_time: 123,
            grid_id: 7,
        };

        let non_world = queue_respawn_db_save_like_cpp(
            wow_map::ManagedMapKind::Dungeon {
                has_reset_schedule: false,
            },
            571,
            1,
            info.clone(),
        );
        assert!(matches!(
            non_world,
            RespawnDbSaveQueueOutcomeLikeCpp::SkippedNonWorldMap
        ));

        let invalid_map_id = queue_respawn_db_save_like_cpp(
            wow_map::ManagedMapKind::World,
            u32::from(u16::MAX) + 1,
            0,
            info,
        );
        assert!(matches!(
            invalid_map_id,
            RespawnDbSaveQueueOutcomeLikeCpp::SkippedInvalidMapId
        ));
    }

    #[test]
    fn spawn_group_condition_update_tick_process_respawns_delete_only_removes_inactive_due_timer() {
        let metadata = test_spawn_metadata_with_flags([(60, 571, SpawnGroupFlags::MANUAL_SPAWN)]);
        let condition_store = ConditionEntriesByTypeStore::default();
        let mut manager = wow_map::MapManager::new(60_000, 1);
        let map = manager.create_world_map(571, 0);
        map.map_mut().add_respawn_info_like_cpp(RespawnInfoLikeCpp {
            object_type: SpawnObjectType::Creature,
            spawn_id: 1,
            entry: 42,
            respawn_time: 0,
            grid_id: 7,
        });
        let mut scheduler = CanonicalRespawnConditionSchedulerLikeCpp::new(1);

        let summary = canonical_map_update_tick_set_inactive_like_cpp(
            &mut manager,
            1,
            &mut scheduler,
            &metadata,
            &condition_store,
            &empty_loaded_grid_creature_respawn_caches_like_cpp(),
        )
        .expect("scheduler fires");

        assert_eq!(summary.maps_evaluated, 1);
        assert_eq!(summary.respawn_deleted_inactive_spawn_group, 1);
        assert_eq!(summary.respawn_blocked_do_respawn_runtime, 0);
        assert_eq!(summary.respawn_db_delete_queued, 1);
        assert_eq!(summary.respawn_db_delete_skipped_non_world_map, 0);
        assert_eq!(summary.respawn_db_delete_skipped_invalid_map_id, 0);
        assert_eq!(summary.respawn_db_deletes.len(), 1);
        let delete = &summary.respawn_db_deletes[0];
        assert_eq!(delete.object_type, SpawnObjectType::Creature);
        assert_eq!(delete.spawn_id, 1);
        assert_eq!(delete.map_id, 571);
        assert_eq!(delete.instance_id, 0);
        assert_eq!(delete.statement.sql(), CharStatements::DEL_RESPAWN.sql());
        assert_del_respawn_params_like_cpp(&delete.statement, 0, 1, 571, 0);
        assert_eq!(
            manager
                .find_map(571, 0)
                .unwrap()
                .map()
                .get_respawn_time_like_cpp(SpawnObjectType::Creature, 1),
            0
        );
    }

    #[test]
    fn respawn_db_save_tick_queues_linked_future_reschedule_like_cpp() {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("test clock after unix epoch")
            .as_secs() as i64;
        let linked_respawn_time = now + 3_600;
        let expected_respawn_time = linked_respawn_time + 5;
        let mut linked_respawns = LinkedRespawnStoreLikeCpp::new();
        linked_respawns.insert_like_cpp(
            linked_respawn_guid_like_cpp(wow_core::guid::HighGuid::Creature, 42, 1),
            linked_respawn_guid_like_cpp(wow_core::guid::HighGuid::Creature, 42, 2),
        );
        let metadata = test_spawn_metadata_with_flags([
            (62, 571, SpawnGroupFlags::NONE),
            (63, 571, SpawnGroupFlags::NONE),
        ])
        .with_linked_respawns_like_cpp(linked_respawns);
        let condition_store = ConditionEntriesByTypeStore::default();
        let mut manager = wow_map::MapManager::new(60_000, 1);
        let map = manager.create_world_map(571, 0);
        map.map_mut().add_respawn_info_like_cpp(RespawnInfoLikeCpp {
            object_type: SpawnObjectType::Creature,
            spawn_id: 1,
            entry: 42,
            respawn_time: 0,
            grid_id: 7,
        });
        map.map_mut().add_respawn_info_like_cpp(RespawnInfoLikeCpp {
            object_type: SpawnObjectType::Creature,
            spawn_id: 2,
            entry: 42,
            respawn_time: linked_respawn_time,
            grid_id: 8,
        });
        let mut scheduler = CanonicalRespawnConditionSchedulerLikeCpp::new(1);

        let summary = canonical_map_update_tick_set_inactive_like_cpp(
            &mut manager,
            1,
            &mut scheduler,
            &metadata,
            &condition_store,
            &empty_loaded_grid_creature_respawn_caches_like_cpp(),
        )
        .expect("scheduler fires");

        assert_eq!(summary.maps_evaluated, 1);
        assert_eq!(summary.respawn_db_save_queued, 1);
        assert_eq!(summary.respawn_db_save_skipped_non_world_map, 0);
        assert_eq!(summary.respawn_db_save_skipped_invalid_map_id, 0);
        assert_eq!(summary.respawn_db_saves.len(), 1);
        let save = &summary.respawn_db_saves[0];
        assert_eq!(save.object_type, SpawnObjectType::Creature);
        assert_eq!(save.spawn_id, 1);
        assert_eq!(save.respawn_time, expected_respawn_time);
        assert_eq!(save.map_id, 571);
        assert_eq!(save.instance_id, 0);
        assert_eq!(save.statement.sql(), CharStatements::REP_RESPAWN.sql());
        assert_rep_respawn_params_like_cpp(&save.statement, 0, 1, expected_respawn_time, 571, 0);
        let map = manager.find_map(571, 0).expect("world map");
        assert_eq!(
            map.map()
                .get_respawn_time_like_cpp(SpawnObjectType::Creature, 1),
            expected_respawn_time
        );
        assert!(
            map.map()
                .get_respawn_time_like_cpp(SpawnObjectType::Creature, 1)
                > now
        );
    }

    #[test]
    fn spawn_group_condition_update_tick_pool_timer_uses_canonical_pool_mgr_and_queues_delete() {
        let mut pool_mgr = PoolMgrLikeCpp::new();
        pool_mgr.insert_template_like_cpp(70, PoolTemplateDataLikeCpp::new(1, 571));
        let mut group = PoolGroupLikeCpp::with_pool_id(PoolMemberKindLikeCpp::Creature, 70);
        group.add_entry_like_cpp(PoolObjectLikeCpp::new(1, 0.0), 1);
        group.add_entry_like_cpp(PoolObjectLikeCpp::new(101, 0.0), 1);
        pool_mgr
            .insert_or_replace_group_like_cpp(PoolMemberKindLikeCpp::Creature, 70, group)
            .expect("test pool group");
        pool_mgr
            .register_spawn_pool_relation_like_cpp(PoolMemberKindLikeCpp::Creature, 1, 70)
            .expect("test spawn pool relation");
        let metadata = test_spawn_metadata_with_flags([(64, 571, SpawnGroupFlags::NONE)])
            .with_pool_mgr_like_cpp(pool_mgr);
        let condition_store = ConditionEntriesByTypeStore::default();
        let mut manager = wow_map::MapManager::new(60_000, 1);
        let map = manager.create_world_map(571, 0);
        map.map_mut().add_respawn_info_like_cpp(RespawnInfoLikeCpp {
            object_type: SpawnObjectType::Creature,
            spawn_id: 1,
            entry: 42,
            respawn_time: 0,
            grid_id: 7,
        });
        let mut scheduler = CanonicalRespawnConditionSchedulerLikeCpp::new(1);

        let summary = canonical_map_update_tick_set_inactive_like_cpp(
            &mut manager,
            1,
            &mut scheduler,
            &metadata,
            &condition_store,
            &empty_loaded_grid_creature_respawn_caches_like_cpp(),
        )
        .expect("scheduler fires");

        assert_eq!(summary.maps_evaluated, 1);
        assert_eq!(summary.respawn_processed_pool_timers, 1);
        assert_eq!(summary.respawn_processed_unloaded_grid_respawns, 0);
        assert_eq!(summary.respawn_pool_update_plans, 1);
        assert_eq!(summary.respawn_blocked_pool_plan_errors, 0);
        assert_eq!(summary.respawn_blocked_pool_runtime, 0);
        assert_eq!(summary.respawn_blocked_do_respawn_runtime, 0);
        assert_eq!(summary.respawn_db_delete_queued, 1);
        assert_eq!(summary.respawn_db_deletes.len(), 1);
        let delete = &summary.respawn_db_deletes[0];
        assert_eq!(delete.object_type, SpawnObjectType::Creature);
        assert_eq!(delete.spawn_id, 1);
        assert_eq!(delete.map_id, 571);
        assert_eq!(delete.instance_id, 0);
        assert_del_respawn_params_like_cpp(&delete.statement, 0, 1, 571, 0);
        let map = manager.find_map(571, 0).expect("world map");
        assert!(
            map.map()
                .pool_data_like_cpp()
                .is_spawned_creature_like_cpp(101)
        );
        assert_eq!(
            map.map()
                .get_respawn_time_like_cpp(SpawnObjectType::Creature, 1),
            0
        );
        assert!(
            map.map()
                .get_respawn_info_like_cpp(SpawnObjectType::Creature, 1)
                .is_none()
        );
    }

    #[test]
    fn spawn_group_condition_update_tick_process_respawns_unloaded_grid_queues_delete_without_spawn()
     {
        let metadata = test_spawn_metadata_with_flags([(61, 571, SpawnGroupFlags::NONE)]);
        let condition_store = ConditionEntriesByTypeStore::default();
        let mut manager = wow_map::MapManager::new(60_000, 1);
        let map = manager.create_world_map(571, 0);
        map.map_mut().add_respawn_info_like_cpp(RespawnInfoLikeCpp {
            object_type: SpawnObjectType::Creature,
            spawn_id: 1,
            entry: 42,
            respawn_time: 0,
            grid_id: 7,
        });
        let mut scheduler = CanonicalRespawnConditionSchedulerLikeCpp::new(1);

        let summary = canonical_map_update_tick_set_inactive_like_cpp(
            &mut manager,
            1,
            &mut scheduler,
            &metadata,
            &condition_store,
            &empty_loaded_grid_creature_respawn_caches_like_cpp(),
        )
        .expect("scheduler fires");

        assert_eq!(summary.maps_evaluated, 1);
        assert_eq!(summary.respawn_deleted_inactive_spawn_group, 0);
        assert_eq!(summary.respawn_processed_unloaded_grid_respawns, 1);
        assert_eq!(summary.respawn_blocked_do_respawn_runtime, 0);
        assert_eq!(summary.respawn_db_delete_queued, 1);
        assert_eq!(summary.respawn_db_deletes.len(), 1);
        let delete = &summary.respawn_db_deletes[0];
        assert_eq!(delete.object_type, SpawnObjectType::Creature);
        assert_eq!(delete.spawn_id, 1);
        assert_eq!(delete.map_id, 571);
        assert_eq!(delete.instance_id, 0);
        assert_del_respawn_params_like_cpp(&delete.statement, 0, 1, 571, 0);
        assert_eq!(
            manager
                .find_map(571, 0)
                .unwrap()
                .map()
                .get_respawn_time_like_cpp(SpawnObjectType::Creature, 1),
            0
        );
        assert!(
            manager
                .find_map(571, 0)
                .unwrap()
                .map()
                .get_respawn_info_like_cpp(SpawnObjectType::Creature, 1)
                .is_none()
        );
    }

    #[test]
    fn loaded_grid_gameobject_respawn_record_returns_gameobject_record_like_cpp() {
        let spawn_id = 77;
        let entry = 9001;
        let mut store = SpawnStore::new();
        let spawn = SpawnData {
            object_type: SpawnObjectType::GameObject,
            spawn_id,
            map_id: 571,
            db_data: true,
            spawn_group: SpawnGroupTemplateData::default_group(),
            id: entry,
            spawn_point: SpawnPosition::new(1.0, 2.0, 3.0, 1.0),
            phase_use_flags: 0,
            phase_id: 0,
            phase_group: 0,
            terrain_swap_map: -1,
            pool_id: 0,
            spawn_time_secs: 30,
            spawn_difficulties: vec![0],
            script_id: 0,
            string_id: String::new(),
        };
        store.add_object_spawn(&spawn, |_| false);
        let metadata =
            super::spawn_store_loader::CanonicalSpawnMetadataLikeCpp::new(store, BTreeMap::new())
                .with_gameobject_runtime_rows_like_cpp(BTreeMap::from([(
                    spawn_id,
                    super::spawn_store_loader::GameObjectSpawnRuntimeRowLikeCpp {
                        spawn_id,
                        rotation: [0.0, 0.0, 0.0, 1.0],
                        anim_progress: 55,
                        state: 1,
                        string_id: "live-gameobject".to_string(),
                        spawn_time_secs: 30,
                    },
                )]));
        let mut data = [0; wow_entities::MAX_GAMEOBJECT_DATA];
        data[11] = 1;
        let mut caches = empty_loaded_grid_creature_respawn_caches_like_cpp();
        caches.gameobject_template_store = Arc::new(
            wow_data::GameObjectTemplateLifecycleStoreLikeCpp::from_templates([
                wow_data::GameObjectTemplateLifecycleRecordLikeCpp {
                    entry,
                    go_type: wow_entities::GAMEOBJECT_TYPE_GOOBER,
                    display_id: 44,
                    name: "Live Loaded GO".to_string(),
                    size: 1.0,
                    data,
                    content_tuning_id: 0,
                    ai_name: String::new(),
                    script_name: String::new(),
                    string_id: String::new(),
                    addon: None,
                },
            ]),
        );
        let mut map = wow_map::Map::new(571, 0, 0, 60_000);
        map.add_respawn_info_like_cpp(RespawnInfoLikeCpp {
            object_type: SpawnObjectType::GameObject,
            spawn_id,
            entry,
            respawn_time: 1_234,
            grid_id: 7,
        });

        let record = build_loaded_grid_gameobject_respawn_record_like_cpp(
            &mut map,
            SpawnObjectType::GameObject,
            spawn_id,
            &metadata,
            &caches,
        )
        .expect("loaded-grid GameObject builder should return loaded-grid records");
        let game_object = record
            .primary_record
            .game_object()
            .expect("builder should return a typed GameObject MapObjectRecord");

        assert_eq!(
            record.primary_record.kind(),
            wow_entities::AccessorObjectKind::GameObject
        );
        assert_eq!(game_object.spawn_id(), spawn_id);
        assert_eq!(
            game_object.world().guid().high_type(),
            wow_core::guid::HighGuid::GameObject
        );
        assert_eq!(u32::from(game_object.world().guid().map_id()), 571);
        assert_eq!(game_object.world().guid().entry(), entry);
        assert_eq!(game_object.world().guid().counter(), 1);
        assert_eq!(
            game_object.respawn_time(),
            0,
            "ProcessRespawns erases due timer before LoadFromDB, so new GO observes no map respawn time"
        );
    }

    #[test]
    fn loaded_grid_creature_respawn_record_variable_level_returns_creature_record_like_cpp() {
        let mut metadata = test_spawn_metadata_with_flags([(67, 571, SpawnGroupFlags::NONE)]);
        let spawn_id = 1;
        let entry = 42;
        metadata = metadata.with_creature_runtime_rows_like_cpp(BTreeMap::from([(
            spawn_id,
            super::spawn_store_loader::CreatureSpawnRuntimeRowLikeCpp {
                spawn_id,
                model_id: 999,
                equipment_id: 3,
                wander_distance: 15.0,
                curhealth: 0,
                curmana: 0,
                movement_type: 1,
                ground_movement_type: wow_constants::CreatureGroundMovementType::Run as u8,
                swim_allowed: true,
                flight_movement_type: 0,
                string_id: "variable-level-live".to_string(),
                spawn_time_secs: 120,
            },
        )]));
        let caches = variable_loaded_grid_creature_respawn_caches_like_cpp(entry);
        let mut map = wow_map::Map::new(571, 0, 2, 60_000);
        map.add_respawn_info_like_cpp(RespawnInfoLikeCpp {
            object_type: SpawnObjectType::Creature,
            spawn_id,
            entry,
            respawn_time: 0,
            grid_id: 7,
        });

        let record = build_loaded_grid_creature_respawn_record_like_cpp(
            &mut map,
            SpawnObjectType::Creature,
            spawn_id,
            &metadata,
            &caches,
        )
        .expect("variable-level loaded-grid Creature builder should no longer block");
        let creature = record
            .primary_record
            .creature()
            .expect("builder should return a typed Creature MapObjectRecord");
        let level = creature.ai_level();

        assert!((18..=20).contains(&level));
        assert_eq!(
            record.primary_record.kind(),
            wow_entities::AccessorObjectKind::Creature
        );
        assert_eq!(creature.lifecycle_metadata().spawn_id, spawn_id);
        assert_eq!(
            creature.guid().high_type(),
            wow_core::guid::HighGuid::Creature
        );
        assert_eq!(u32::from(creature.guid().map_id()), 571);
        assert_eq!(creature.guid().entry(), entry);
        assert_eq!(creature.guid().counter(), 1);
        assert_eq!(creature.ai_max_health(), u64::from(level) * 20);
        assert_eq!(creature.ai_current_health(), creature.ai_max_health());
    }

    #[test]
    fn loaded_grid_creature_spawn_group_spawn_record_does_not_require_respawn_timer_like_cpp() {
        let mut metadata = test_spawn_metadata_with_flags([(68, 571, SpawnGroupFlags::NONE)]);
        let spawn_id = 1;
        let entry = 42;
        metadata = metadata.with_creature_runtime_rows_like_cpp(BTreeMap::from([(
            spawn_id,
            super::spawn_store_loader::CreatureSpawnRuntimeRowLikeCpp {
                spawn_id,
                model_id: 999,
                equipment_id: 3,
                wander_distance: 15.0,
                curhealth: 0,
                curmana: 0,
                movement_type: 1,
                ground_movement_type: wow_constants::CreatureGroundMovementType::Run as u8,
                swim_allowed: true,
                flight_movement_type: 0,
                string_id: "condition-spawn-no-timer".to_string(),
                spawn_time_secs: 120,
            },
        )]));
        let caches =
            variable_loaded_grid_creature_respawn_caches_with_vehicle_id_and_difficulty_like_cpp(
                entry, 0, 0,
            );
        let mut map = wow_map::Map::new(571, 0, 0, 60_000);
        assert_eq!(
            map.get_respawn_time_like_cpp(SpawnObjectType::Creature, spawn_id),
            0
        );

        let record = build_loaded_grid_creature_spawn_group_spawn_record_like_cpp(
            &mut map,
            SpawnObjectType::Creature,
            spawn_id,
            &metadata,
            &caches,
        )
        .expect("SpawnGroupSpawn loaded-grid Creature loader must not require a respawn timer");
        let creature = record
            .primary_record
            .creature()
            .expect("builder should return a typed Creature MapObjectRecord");

        assert_eq!(creature.respawn_time(), 0);
        assert_eq!(creature.lifecycle_metadata().spawn_id, spawn_id);
        assert_eq!(creature.guid().entry(), entry);
    }

    #[test]
    fn spawn_group_condition_update_spawn_loads_loaded_grid_creature_without_respawn_timer_like_cpp()
     {
        let mut metadata = test_spawn_metadata_with_flags([(69, 571, SpawnGroupFlags::NONE)]);
        let spawn_id = 1;
        let entry = 42;
        metadata = metadata.with_creature_runtime_rows_like_cpp(BTreeMap::from([(
            spawn_id,
            super::spawn_store_loader::CreatureSpawnRuntimeRowLikeCpp {
                spawn_id,
                model_id: 999,
                equipment_id: 3,
                wander_distance: 15.0,
                curhealth: 0,
                curmana: 0,
                movement_type: 1,
                ground_movement_type: wow_constants::CreatureGroundMovementType::Run as u8,
                swim_allowed: true,
                flight_movement_type: 0,
                string_id: "condition-spawn-caller-no-timer".to_string(),
                spawn_time_secs: 120,
            },
        )]));
        let condition_store =
            ConditionEntriesByTypeStore::from_conditions_like_cpp([mapid_condition(69, 571)]);
        let caches =
            variable_loaded_grid_creature_respawn_caches_with_vehicle_id_and_difficulty_like_cpp(
                entry, 0, 0,
            );
        let mut manager = wow_map::MapManager::new(60_000, 10);
        let group = metadata
            .spawn_group_templates()
            .get(&69)
            .expect("test group 69")
            .clone();
        let map = manager.create_world_map(571, 0);
        map.map_mut()
            .set_spawn_group_inactive_like_cpp(Some(&group));
        assert!(map.map_mut().load_grid(0.0, 0.0));
        assert_eq!(
            map.map()
                .get_respawn_time_like_cpp(SpawnObjectType::Creature, spawn_id),
            0
        );

        let outcomes = apply_canonical_spawn_group_condition_update_loaded_grid_records_like_cpp(
            map,
            &metadata,
            &condition_store,
            &caches,
        );

        let spawn_outcome = outcomes
            .iter()
            .find(|outcome| outcome.group_id == 69)
            .and_then(|outcome| outcome.spawn_outcome.as_ref())
            .expect("condition-success SpawnGroupSpawn outcome");
        assert_eq!(spawn_outcome.executed_loaded_grid_spawns, 1);
        assert_eq!(spawn_outcome.blocked_loaded_grid_creature_loads, 0);
        assert_eq!(spawn_outcome.blocked_loaded_grid_spawn_loads, 0);
        assert_eq!(spawn_outcome.skipped_respawn_timer_active, 0);
        assert_eq!(map.map().map_object_count(), 1);
        let creature = map
            .map()
            .get_creature_by_spawn_id_like_cpp(spawn_id)
            .expect("loaded-grid Creature should be indexed by spawn id");
        assert_eq!(creature.respawn_time(), 0);
        assert_eq!(creature.lifecycle_metadata().spawn_id, spawn_id);
    }

    #[test]
    fn loaded_grid_creature_respawn_record_vehicle_template_uses_creature_low_vehicle_high_like_cpp()
     {
        let mut metadata = test_spawn_metadata_with_flags([(67, 571, SpawnGroupFlags::NONE)]);
        let spawn_id = 1;
        let entry = 42;
        metadata = metadata.with_creature_runtime_rows_like_cpp(BTreeMap::from([(
            spawn_id,
            super::spawn_store_loader::CreatureSpawnRuntimeRowLikeCpp {
                spawn_id,
                model_id: 999,
                equipment_id: 3,
                wander_distance: 15.0,
                curhealth: 0,
                curmana: 0,
                movement_type: 1,
                ground_movement_type: wow_constants::CreatureGroundMovementType::Run as u8,
                swim_allowed: true,
                flight_movement_type: 0,
                string_id: "vehicle-template-live".to_string(),
                spawn_time_secs: 120,
            },
        )]));
        let mut caches =
            variable_loaded_grid_creature_respawn_caches_with_vehicle_id_like_cpp(entry, 101);
        let entry_accessory = wow_entities::VehicleAccessory {
            accessory_entry: 7001,
            seat_id: 1,
            is_minion: false,
            summoned_type: 6,
            summon_time_ms: 3_000,
        };
        let spawn_accessory = wow_entities::VehicleAccessory {
            accessory_entry: 8001,
            seat_id: 2,
            is_minion: true,
            summoned_type: 8,
            summon_time_ms: 4_000,
        };
        caches.vehicle_accessory_store =
            Arc::new(wow_data::VehicleAccessoryStoreLikeCpp::from_parts(
                [(spawn_id, vec![spawn_accessory])],
                [(entry, vec![entry_accessory])],
            ));
        let mut map = wow_map::Map::new(571, 0, 2, 60_000);
        map.add_respawn_info_like_cpp(RespawnInfoLikeCpp {
            object_type: SpawnObjectType::Creature,
            spawn_id,
            entry,
            respawn_time: 0,
            grid_id: 7,
        });

        let record = build_loaded_grid_creature_respawn_record_like_cpp(
            &mut map,
            SpawnObjectType::Creature,
            spawn_id,
            &metadata,
            &caches,
        )
        .expect("vehicle-template loaded-grid Creature builder should resolve");
        let creature = record
            .primary_record
            .creature()
            .expect("builder should return a typed Creature MapObjectRecord");

        assert_eq!(
            creature.guid().high_type(),
            wow_core::guid::HighGuid::Vehicle
        );
        assert_eq!(creature.guid().counter(), 1);
        assert_eq!(creature.guid().entry(), entry);
        assert_eq!(creature.lifecycle_metadata().spawn_id, spawn_id);
        assert_eq!(creature.lifecycle_metadata().vehicle_id, Some(101));
        let kit = creature
            .unit()
            .subsystems()
            .vehicle
            .kit
            .as_ref()
            .expect("VehicleEntry-backed template should create a local kit");
        assert_eq!(kit.kit_id(), 101);
        assert!(kit.active());
        assert!(!kit.installed());
        assert_eq!(kit.seat_count(), 2);
        assert_eq!(kit.usable_seat_num(), 1);
        let outcome = creature
            .unit()
            .subsystems()
            .vehicle
            .last_create_outcome
            .as_ref()
            .expect("CreateVehicleKit evidence should be recorded");
        assert!(outcome.created);
        assert_eq!(outcome.seat_count, 2);
        assert_eq!(outcome.usable_seat_num, 1);
        assert!(outcome.update_display_power_represented);
        assert!(!outcome.send_set_vehicle_rec_id_represented);
        let reset_context = creature
            .add_to_world_vehicle_reset_context_like_cpp()
            .expect("VehicleEntry-backed template should build AddToWorld reset context");
        assert!(!reset_context.is_mechanical_creature);
        assert!(!reset_context.is_world_boss);
        assert_eq!(reset_context.accessories, vec![spawn_accessory]);
    }

    #[test]
    fn loaded_grid_creature_respawn_record_vehicle_high_guid_without_kit_when_vehicle_row_missing_like_cpp()
     {
        let mut metadata = test_spawn_metadata_with_flags([(67, 571, SpawnGroupFlags::NONE)]);
        let spawn_id = 1;
        let entry = 42;
        metadata = metadata.with_creature_runtime_rows_like_cpp(BTreeMap::from([(
            spawn_id,
            super::spawn_store_loader::CreatureSpawnRuntimeRowLikeCpp {
                spawn_id,
                model_id: 999,
                equipment_id: 3,
                wander_distance: 15.0,
                curhealth: 0,
                curmana: 0,
                movement_type: 1,
                ground_movement_type: wow_constants::CreatureGroundMovementType::Run as u8,
                swim_allowed: true,
                flight_movement_type: 0,
                string_id: "vehicle-template-missing-row".to_string(),
                spawn_time_secs: 120,
            },
        )]));
        let mut caches =
            variable_loaded_grid_creature_respawn_caches_with_vehicle_id_like_cpp(entry, 101);
        caches.vehicle_store = Arc::new(wow_data::VehicleStore::from_entries([]));
        let mut map = wow_map::Map::new(571, 0, 2, 60_000);
        map.add_respawn_info_like_cpp(RespawnInfoLikeCpp {
            object_type: SpawnObjectType::Creature,
            spawn_id,
            entry,
            respawn_time: 0,
            grid_id: 7,
        });

        let record = build_loaded_grid_creature_respawn_record_like_cpp(
            &mut map,
            SpawnObjectType::Creature,
            spawn_id,
            &metadata,
            &caches,
        )
        .expect("vehicle-template loaded-grid Creature builder should still resolve");
        let creature = record
            .primary_record
            .creature()
            .expect("builder should return a typed Creature MapObjectRecord");

        assert_eq!(
            creature.guid().high_type(),
            wow_core::guid::HighGuid::Vehicle
        );
        assert_eq!(creature.lifecycle_metadata().vehicle_id, Some(101));
        assert!(creature.unit().subsystems().vehicle.kit.is_none());
        let outcome = creature
            .unit()
            .subsystems()
            .vehicle
            .last_create_outcome
            .as_ref()
            .expect("CreateVehicleKit false evidence should be recorded");
        assert_eq!(outcome.kit_id, Some(101));
        assert!(!outcome.created);
        assert!(!outcome.update_display_power_represented);
    }

    fn variable_loaded_grid_creature_respawn_caches_like_cpp(
        entry: u32,
    ) -> LoadedGridCreatureRespawnCachesLikeCpp {
        variable_loaded_grid_creature_respawn_caches_with_vehicle_id_like_cpp(entry, 0)
    }

    fn variable_loaded_grid_creature_respawn_caches_with_vehicle_id_like_cpp(
        entry: u32,
        vehicle_id: u32,
    ) -> LoadedGridCreatureRespawnCachesLikeCpp {
        variable_loaded_grid_creature_respawn_caches_with_vehicle_id_and_difficulty_like_cpp(
            entry, vehicle_id, 2,
        )
    }

    fn variable_loaded_grid_creature_respawn_caches_with_vehicle_id_and_difficulty_like_cpp(
        entry: u32,
        vehicle_id: u32,
        difficulty_id: u8,
    ) -> LoadedGridCreatureRespawnCachesLikeCpp {
        LoadedGridCreatureRespawnCachesLikeCpp {
            template_store: Arc::new(
                wow_data::CreatureTemplateLifecycleStoreLikeCpp::from_templates([
                    wow_data::CreatureTemplateLifecycleRecordLikeCpp {
                        entry,
                        name: "Variable Level Live Creature".to_string(),
                        faction: 35,
                        speed_walk: 1.0,
                        speed_run: 1.14286,
                        scale: 1.0,
                        classification: 0,
                        creature_type: 0,
                        unit_class: 1,
                        vehicle_id,
                        movement_type: 1,
                        ground_movement_type: wow_constants::CreatureGroundMovementType::Run as u8,
                        swim_allowed: true,
                        flight_movement_type: 0,
                        flags_extra: 0,
                        string_id: String::new(),
                        regen_health: true,
                        spells: [0; 8],
                        models: vec![wow_data::CreatureTemplateLifecycleModelLikeCpp {
                            creature_display_id: 111,
                            display_scale: 1.0,
                            probability: 100.0,
                        }],
                    },
                ]),
            ),
            difficulty_store: Arc::new(wow_data::CreatureDifficultyStoreLikeCpp::from_records(
                [wow_data::CreatureDifficultyRecordLikeCpp {
                    entry,
                    difficulty_id,
                    min_level: 18,
                    max_level: 20,
                    health_scaling_expansion: -1,
                    health_modifier: 2.0,
                    mana_modifier: 1.0,
                    armor_modifier: 1.0,
                    damage_modifier: 1.0,
                    creature_difficulty_id: 0,
                    type_flags: 0,
                    type_flags2: 0,
                    loot_id: 0,
                    pickpocket_loot_id: 0,
                    skin_loot_id: 0,
                    gold_min: 0,
                    gold_max: 0,
                    static_flags: [0; 8],
                }],
                |_| 1.0,
            )),
            base_stats_store: Arc::new(wow_data::CreatureBaseStatsStoreLikeCpp::from_records([
                (18, 1, creature_base_stats_record_like_cpp(180)),
                (19, 1, creature_base_stats_record_like_cpp(190)),
                (20, 1, creature_base_stats_record_like_cpp(200)),
            ])),
            health_rates: wow_data::CreatureClassificationHealthRatesLikeCpp::default(),
            display_store: Arc::new(wow_data::CreatureDisplayInfoStore::from_entries([])),
            model_store: Arc::new(wow_data::CreatureModelDataStore::from_entries([])),
            vehicle_store: Arc::new(vehicle_store_for_loaded_grid_test(vehicle_id)),
            vehicle_seat_store: Arc::new(vehicle_seat_store_for_loaded_grid_test()),
            vehicle_accessory_store: Arc::new(wow_data::VehicleAccessoryStoreLikeCpp::from_parts(
                [],
                [],
            )),
            gameobject_template_store: Arc::new(
                wow_data::GameObjectTemplateLifecycleStoreLikeCpp::default(),
            ),
            gameobject_override_store: Arc::new(
                wow_data::GameObjectOverrideLifecycleStoreLikeCpp::default(),
            ),
        }
    }

    fn vehicle_store_for_loaded_grid_test(vehicle_id: u32) -> wow_data::VehicleStore {
        if vehicle_id == 0 {
            return wow_data::VehicleStore::from_entries([]);
        }
        let mut seat_ids = [0u16; 8];
        seat_ids[0] = 700;
        seat_ids[2] = 701;
        wow_data::VehicleStore::from_entries([wow_data::VehicleEntry {
            id: vehicle_id,
            flags: 0,
            flags_b: 0,
            seat_ids,
        }])
    }

    fn vehicle_seat_store_for_loaded_grid_test() -> wow_data::VehicleSeatStore {
        wow_data::VehicleSeatStore::from_entries([
            wow_data::VehicleSeatEntry {
                id: 700,
                attachment_offset_x: 0.0,
                attachment_offset_y: 0.0,
                attachment_offset_z: 0.0,
                flags: wow_data::VEHICLE_SEAT_FLAG_CAN_ENTER_OR_EXIT,
                flags_b: 0,
                flags_c: 0,
            },
            wow_data::VehicleSeatEntry {
                id: 701,
                attachment_offset_x: 0.0,
                attachment_offset_y: 0.0,
                attachment_offset_z: 0.0,
                flags: 0,
                flags_b: 0,
                flags_c: 0,
            },
        ])
    }

    fn creature_base_stats_record_like_cpp(
        base_health: u32,
    ) -> wow_data::CreatureBaseStatsRecordLikeCpp {
        wow_data::CreatureBaseStatsRecordLikeCpp {
            base_health: [base_health / 4, base_health / 2, base_health],
            base_mana: 50,
            base_armor: 0,
            attack_power: 0,
            ranged_attack_power: 0,
            base_damage: [1.0, 2.0, 3.0],
        }
    }

    fn test_spawn_metadata<const N: usize>(
        groups: [(u32, u32); N],
    ) -> super::spawn_store_loader::CanonicalSpawnMetadataLikeCpp {
        test_spawn_metadata_with_flags(
            groups.map(|(group_id, map_id)| (group_id, map_id, SpawnGroupFlags::NONE)),
        )
    }

    fn test_spawn_metadata_with_flags<const N: usize>(
        groups: [(u32, u32, SpawnGroupFlags); N],
    ) -> super::spawn_store_loader::CanonicalSpawnMetadataLikeCpp {
        let mut store = SpawnStore::new();
        let mut templates = BTreeMap::new();
        let mut rows = Vec::new();
        for (index, (group_id, map_id, flags)) in groups.into_iter().enumerate() {
            templates.insert(
                group_id,
                SpawnGroupTemplateData {
                    group_id,
                    name: format!("test group {group_id}"),
                    map_id: wow_map::spawn::SPAWNGROUP_MAP_UNSET,
                    flags,
                },
            );
            let spawn_id = u64::try_from(index).expect("test index fits") + 1;
            let spawn = test_spawn(spawn_id, map_id);
            store.add_object_spawn(&spawn, |_| false);
            rows.push(SpawnGroupMemberRow {
                group_id,
                spawn_type: SpawnObjectType::Creature as u8,
                spawn_id,
            });
        }
        store.apply_spawn_groups_like_cpp(&mut templates, rows);
        super::spawn_store_loader::CanonicalSpawnMetadataLikeCpp::new(store, templates)
    }

    fn test_spawn(spawn_id: u64, map_id: u32) -> SpawnData {
        SpawnData {
            object_type: SpawnObjectType::Creature,
            spawn_id,
            map_id,
            db_data: true,
            spawn_group: SpawnGroupTemplateData::default_group(),
            id: 42,
            spawn_point: SpawnPosition::new(0.0, 0.0, 0.0, 0.0),
            phase_use_flags: 0,
            phase_id: 0,
            phase_group: 0,
            terrain_swap_map: -1,
            pool_id: 0,
            spawn_time_secs: 120,
            spawn_difficulties: vec![0],
            script_id: 0,
            string_id: String::new(),
        }
    }

    fn mapid_condition(spawn_group_id: u32, expected_map_id: u32) -> Condition {
        Condition {
            source_type: ConditionSourceType::SpawnGroup,
            source_group: 0,
            source_entry: spawn_group_id as i32,
            source_id: 0,
            condition_type: ConditionType::MapId,
            condition_value1: expected_map_id,
            ..Condition::default()
        }
    }

    fn unique_temp_dir(name: &str) -> PathBuf {
        let mut path = env::temp_dir();
        path.push(format!(
            "rustycore_world_server_{name}_{}",
            std::process::id()
        ));

        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("temp dir failed");
        path
    }

    // ── Slice 4A.1b: routing tests ───────────────────────────────────────────
    // C++ anchors:
    //   Object.cpp : WorldObject::SendMessageToSet (~1746-1764)
    //   GridNotifiersImpl.h : MessageDistDeliverer::Visit(PlayerMapType&) (~43-46)
    //   GridNotifiers.h : MessageDistDeliverer::SendPacket

    fn make_source_guid() -> ObjectGuid {
        ObjectGuid::create_world_object(wow_core::guid::HighGuid::Creature, 0, 1, 571, 0, 1, 1)
    }

    fn make_nearby_visible_event_like_cpp(
        map_id: u16,
        instance_id: u32,
        source_position: Position,
        range: f32,
        required_3d: bool,
    ) -> wow_world::map_manager::RuntimeEvent {
        wow_world::map_manager::RuntimeEvent {
            source_guid: make_source_guid(),
            recipients: wow_world::map_manager::RecipientRule::NearbyVisible {
                source_guid: make_source_guid(),
                map_id,
                instance_id,
                source_position,
                range,
                required_3d,
            },
            packet_bytes: vec![0xAA, 0xBB],
        }
    }

    fn make_registry_player_like_cpp(
        map_id: u16,
        instance_id: u32,
        position: Position,
        is_in_world: bool,
    ) -> (PlayerBroadcastInfo, flume::Receiver<SessionCommand>) {
        let (send_tx, _send_rx) = flume::bounded(4);
        let (command_tx, command_rx) = flume::bounded(4);
        let mut info = player_broadcast_info_fixture_like_cpp(send_tx, command_tx, "Tester");
        info.map_id = map_id;
        info.instance_id = instance_id;
        info.position = position;
        info.is_in_world = is_in_world;
        (info, command_rx)
    }

    fn add_canonical_test_player_on_map_like_cpp(
        canonical: &wow_world::SharedCanonicalMapManager,
        guid: ObjectGuid,
        position: Position,
        map_id: u32,
        instance_id: u32,
        health: u64,
    ) {
        let mut player = Player::new(Some(1), false);
        player.unit_mut().world_mut().object_mut().create(guid);
        player.unit_mut().world_mut().set_name("RuntimeVictim");
        player
            .unit_mut()
            .world_mut()
            .set_map(map_id, instance_id)
            .unwrap();
        player.unit_mut().world_mut().relocate(position);
        player.unit_mut().world_mut().object_mut().add_to_world();
        player.unit_mut().set_level(80);
        player.unit_mut().set_max_health(health);
        player.unit_mut().set_health(health);

        canonical
            .lock()
            .unwrap()
            .create_world_map(map_id, instance_id)
            .map_mut()
            .insert_map_object_record(MapObjectRecord::new_player(player).unwrap())
            .unwrap();
    }

    /// (1) NearbyVisible: players on a different map_id are not enqueued.
    /// C++ anchor: MessageDistDeliverer::Visit — map-id check before distance.
    #[test]
    fn nearby_visible_filters_by_map_id_like_cpp() {
        let registry = PlayerRegistry::default();
        let guid = ObjectGuid::create_player(1, 1);
        let (info, command_rx) = make_registry_player_like_cpp(530, 0, Position::ZERO, true); // wrong map
        registry.insert(guid, info);

        let event = make_nearby_visible_event_like_cpp(571, 0, Position::ZERO, 100.0, false);
        let plan = wow_world::map_manager::RuntimePlan {
            events: vec![event],
        };
        let summary = deliver_runtime_plan_like_cpp(&plan, &registry);

        assert_eq!(summary.candidates_queued, 0);
        assert_eq!(summary.candidates_skipped_wrong_map, 1);
        assert!(command_rx.try_recv().is_err());
    }

    /// (2) NearbyVisible: players on a different instance_id are not enqueued.
    /// Slice 4A.1b requirement — instance separation.
    #[test]
    fn nearby_visible_filters_by_instance_id_like_cpp() {
        let registry = PlayerRegistry::default();
        let guid = ObjectGuid::create_player(1, 2);
        let (info, command_rx) = make_registry_player_like_cpp(571, 99, Position::ZERO, true); // wrong instance
        registry.insert(guid, info);

        let event = make_nearby_visible_event_like_cpp(571, 0, Position::ZERO, 100.0, false);
        let plan = wow_world::map_manager::RuntimePlan {
            events: vec![event],
        };
        let summary = deliver_runtime_plan_like_cpp(&plan, &registry);

        assert_eq!(summary.candidates_queued, 0);
        assert_eq!(summary.candidates_skipped_wrong_instance, 1);
        assert!(command_rx.try_recv().is_err());
    }

    /// (3) NearbyVisible: players not in world are not enqueued.
    /// C++ anchor: MessageDistDeliverer::Visit — `Player::IsInWorld()` gate.
    #[test]
    fn nearby_visible_filters_is_in_world_like_cpp() {
        let registry = PlayerRegistry::default();
        let guid = ObjectGuid::create_player(1, 3);
        let (info, command_rx) = make_registry_player_like_cpp(571, 0, Position::ZERO, false); // not in world
        registry.insert(guid, info);

        let event = make_nearby_visible_event_like_cpp(571, 0, Position::ZERO, 100.0, false);
        let plan = wow_world::map_manager::RuntimePlan {
            events: vec![event],
        };
        let summary = deliver_runtime_plan_like_cpp(&plan, &registry);

        assert_eq!(summary.candidates_queued, 0);
        assert_eq!(summary.candidates_skipped_not_in_world, 1);
        assert!(command_rx.try_recv().is_err());
    }

    /// (4) NearbyVisible: 2D distance check excludes players beyond range when
    /// `required_3d == false` — Z-axis is ignored.
    /// C++ anchor: GridNotifiersImpl.h MessageDistDeliverer::Visit ~43-46.
    #[test]
    fn nearby_visible_uses_2d_distance_when_required_3d_false_like_cpp() {
        let registry = PlayerRegistry::default();
        // Player is far on Z but close in XY — should be INCLUDED with 2D check.
        let near_guid = ObjectGuid::create_player(1, 4);
        let (near_info, near_rx) =
            make_registry_player_like_cpp(571, 0, Position::new(5.0, 0.0, 1000.0, 0.0), true);
        registry.insert(near_guid, near_info);

        // Player is far in XY — should be EXCLUDED.
        let far_guid = ObjectGuid::create_player(1, 5);
        let (far_info, far_rx) =
            make_registry_player_like_cpp(571, 0, Position::new(200.0, 0.0, 0.0, 0.0), true);
        registry.insert(far_guid, far_info);

        let source = Position::new(0.0, 0.0, 0.0, 0.0);
        let event = make_nearby_visible_event_like_cpp(571, 0, source, 100.0, false);
        let plan = wow_world::map_manager::RuntimePlan {
            events: vec![event],
        };
        let summary = deliver_runtime_plan_like_cpp(&plan, &registry);

        assert_eq!(summary.candidates_queued, 1, "only the XY-near player");
        assert_eq!(summary.candidates_skipped_distance, 1);
        assert!(near_rx.try_recv().is_ok(), "near player got command");
        assert!(far_rx.try_recv().is_err(), "far player did not get command");
    }

    /// (5) NearbyVisible: 3D distance check excludes players beyond range when
    /// `required_3d == true` — Z-axis contributes to distance.
    /// C++ anchor: GridNotifiersImpl.h MessageDistDeliverer::Visit ~43-46.
    #[test]
    fn nearby_visible_uses_3d_distance_when_required_3d_true_like_cpp() {
        let registry = PlayerRegistry::default();
        // Player is close in XY but far on Z — should be EXCLUDED with 3D check.
        let near_xy_guid = ObjectGuid::create_player(1, 6);
        let (near_xy_info, near_xy_rx) =
            make_registry_player_like_cpp(571, 0, Position::new(5.0, 0.0, 200.0, 0.0), true);
        registry.insert(near_xy_guid, near_xy_info);

        // Player is close in 3D — should be INCLUDED.
        let near_3d_guid = ObjectGuid::create_player(1, 7);
        let (near_3d_info, near_3d_rx) =
            make_registry_player_like_cpp(571, 0, Position::new(3.0, 3.0, 3.0, 0.0), true);
        registry.insert(near_3d_guid, near_3d_info);

        let source = Position::new(0.0, 0.0, 0.0, 0.0);
        let event = make_nearby_visible_event_like_cpp(571, 0, source, 10.0, true);
        let plan = wow_world::map_manager::RuntimePlan {
            events: vec![event],
        };
        let summary = deliver_runtime_plan_like_cpp(&plan, &registry);

        assert_eq!(summary.candidates_queued, 1, "only the 3D-near player");
        assert_eq!(summary.candidates_skipped_distance, 1);
        assert!(near_xy_rx.try_recv().is_err(), "far-Z player excluded");
        assert!(near_3d_rx.try_recv().is_ok(), "3D-near player included");
    }

    /// (6) MapBroadcastVisible: enqueues all players on the same map/instance
    /// regardless of distance, but respects map/instance/in_world.
    /// C++ anchor: WorldObject::SendMessageToSet map-wide broadcast path.
    #[test]
    fn map_broadcast_visible_ignores_distance_but_respects_map_instance_in_world_like_cpp() {
        let registry = PlayerRegistry::default();

        // In range player — correct map/instance.
        let in_guid = ObjectGuid::create_player(1, 10);
        let (in_info, in_rx) =
            make_registry_player_like_cpp(571, 0, Position::new(9999.0, 9999.0, 0.0, 0.0), true);
        registry.insert(in_guid, in_info);

        // Wrong map.
        let wrong_map_guid = ObjectGuid::create_player(1, 11);
        let (wrong_map_info, wrong_map_rx) =
            make_registry_player_like_cpp(530, 0, Position::ZERO, true);
        registry.insert(wrong_map_guid, wrong_map_info);

        // Not in world.
        let no_world_guid = ObjectGuid::create_player(1, 12);
        let (no_world_info, no_world_rx) =
            make_registry_player_like_cpp(571, 0, Position::ZERO, false);
        registry.insert(no_world_guid, no_world_info);

        let event = wow_world::map_manager::RuntimeEvent {
            source_guid: make_source_guid(),
            recipients: wow_world::map_manager::RecipientRule::MapBroadcastVisible {
                map_id: 571,
                instance_id: 0,
            },
            packet_bytes: vec![0xCC],
        };
        let plan = wow_world::map_manager::RuntimePlan {
            events: vec![event],
        };
        let summary = deliver_runtime_plan_like_cpp(&plan, &registry);

        assert_eq!(summary.candidates_queued, 1);
        assert!(in_rx.try_recv().is_ok(), "valid player got command");
        assert!(wrong_map_rx.try_recv().is_err(), "wrong-map excluded");
        assert!(no_world_rx.try_recv().is_err(), "not-in-world excluded");
    }

    /// (7) ExplicitPlayer: command sent to exactly one GUID, no other sessions.
    /// C++ anchor: WorldObject::SendMessageToSet explicit receiver path.
    #[test]
    fn explicit_player_routes_only_to_target_guid_like_cpp() {
        let registry = PlayerRegistry::default();
        let target_guid = ObjectGuid::create_player(1, 20);
        let other_guid = ObjectGuid::create_player(1, 21);
        let (target_info, target_rx) = make_registry_player_like_cpp(571, 0, Position::ZERO, true);
        let (other_info, other_rx) = make_registry_player_like_cpp(571, 0, Position::ZERO, true);
        registry.insert(target_guid, target_info);
        registry.insert(other_guid, other_info);

        let event = wow_world::map_manager::RuntimeEvent {
            source_guid: make_source_guid(),
            recipients: wow_world::map_manager::RecipientRule::ExplicitPlayer(target_guid),
            packet_bytes: vec![0xDD],
        };
        let plan = wow_world::map_manager::RuntimePlan {
            events: vec![event],
        };
        let summary = deliver_runtime_plan_like_cpp(&plan, &registry);

        assert_eq!(summary.candidates_queued, 1);
        assert!(target_rx.try_recv().is_ok(), "target received command");
        assert!(other_rx.try_recv().is_err(), "other session NOT notified");
    }

    /// (8) SelfOnly: NO broadcast global; increments self_only_skipped counter.
    /// Guarantees SelfOnly events are not distributed to any registry session.
    /// C++ anchor: WorldObject::SendMessageToSet — self-send path bypasses
    /// MessageDistDeliverer entirely.
    #[test]
    fn self_only_does_not_broadcast_to_any_session_like_cpp() {
        let registry = PlayerRegistry::default();
        // Even with a matching player in registry, SelfOnly must NOT deliver.
        let guid = ObjectGuid::create_player(1, 30);
        let (info, command_rx) = make_registry_player_like_cpp(571, 0, Position::ZERO, true);
        registry.insert(guid, info);

        let event = wow_world::map_manager::RuntimeEvent {
            source_guid: make_source_guid(),
            recipients: wow_world::map_manager::RecipientRule::SelfOnly,
            packet_bytes: vec![0xEE],
        };
        let plan = wow_world::map_manager::RuntimePlan {
            events: vec![event],
        };
        let summary = deliver_runtime_plan_like_cpp(&plan, &registry);

        assert_eq!(summary.self_only_skipped, 1, "must count skipped SelfOnly");
        assert_eq!(
            summary.candidates_queued, 0,
            "must NOT broadcast to registry"
        );
        assert_eq!(summary.candidates_seen, 0, "no candidates should be seen");
        assert!(
            command_rx.try_recv().is_err(),
            "session must NOT receive command"
        );
    }

    /// (9) try_send on a full channel increments send_failed and does NOT block.
    /// Backpressure requirement from Slice 4A.1b spec.
    #[test]
    fn full_command_channel_increments_send_failed_and_does_not_block_like_cpp() {
        let registry = PlayerRegistry::default();
        let guid = ObjectGuid::create_player(1, 40);

        let (send_tx, _send_rx) = flume::bounded::<Vec<u8>>(1);
        // Drop the receiver so try_send returns Err::Disconnected immediately.
        let (command_tx, command_rx) = flume::bounded::<SessionCommand>(1);
        drop(command_rx);

        let mut info = player_broadcast_info_fixture_like_cpp(send_tx, command_tx, "Full");
        info.map_id = 571;
        info.instance_id = 0;
        info.is_in_world = true;
        info.position = Position::ZERO;
        registry.insert(guid, info);

        let event = make_nearby_visible_event_like_cpp(571, 0, Position::ZERO, 1000.0, false);
        let plan = wow_world::map_manager::RuntimePlan {
            events: vec![event],
        };
        let summary = deliver_runtime_plan_like_cpp(&plan, &registry);

        assert_eq!(
            summary.send_failed, 1,
            "disconnected channel counted as send_failed"
        );
        assert_eq!(summary.candidates_queued, 0);
    }

    /// 4A.3c dormant rail: map/instance scoped creature visibility refresh.
    ///
    /// C++ anchor: `Player::UpdateVisibilityOf` (Player.cpp:23138+) is the
    /// seam that mutates `m_clientGUIDs` and emits CREATE/DESTROY. The global
    /// runtime must wake matching sessions to run that seam rather than trying
    /// to send raw CREATE bytes through HaveAtClient.
    #[test]
    fn refresh_visible_world_creatures_routes_by_map_instance_in_world_like_cpp() {
        let registry = PlayerRegistry::default();

        let in_a = ObjectGuid::create_player(1, 50);
        let (in_a_info, in_a_rx) = make_registry_player_like_cpp(571, 7, Position::ZERO, true);
        registry.insert(in_a, in_a_info);

        let in_b = ObjectGuid::create_player(1, 51);
        let (in_b_info, in_b_rx) =
            make_registry_player_like_cpp(571, 7, Position::new(9000.0, 0.0, 0.0, 0.0), true);
        registry.insert(in_b, in_b_info);

        let wrong_map = ObjectGuid::create_player(1, 52);
        let (wrong_map_info, wrong_map_rx) =
            make_registry_player_like_cpp(530, 7, Position::ZERO, true);
        registry.insert(wrong_map, wrong_map_info);

        let wrong_instance = ObjectGuid::create_player(1, 53);
        let (wrong_instance_info, wrong_instance_rx) =
            make_registry_player_like_cpp(571, 8, Position::ZERO, true);
        registry.insert(wrong_instance, wrong_instance_info);

        let not_in_world = ObjectGuid::create_player(1, 54);
        let (not_in_world_info, not_in_world_rx) =
            make_registry_player_like_cpp(571, 7, Position::ZERO, false);
        registry.insert(not_in_world, not_in_world_info);

        let summary = deliver_refresh_visible_world_creatures_like_cpp(571, 7, &registry);

        assert_eq!(summary.candidates_seen, 5);
        assert_eq!(summary.candidates_queued, 2);
        assert_eq!(summary.candidates_skipped_wrong_map, 1);
        assert_eq!(summary.candidates_skipped_wrong_instance, 1);
        assert_eq!(summary.candidates_skipped_not_in_world, 1);

        for command in [
            in_a_rx.try_recv().expect("same-map player A refresh"),
            in_b_rx.try_recv().expect("same-map player B refresh"),
        ] {
            let SessionCommand::RefreshVisibleWorldCreaturesLikeCpp(command) = command else {
                panic!("expected RefreshVisibleWorldCreaturesLikeCpp command");
            };
            assert_eq!(command.map_id, 571);
            assert_eq!(command.instance_id, 7);
        }
        assert!(wrong_map_rx.try_recv().is_err());
        assert!(wrong_instance_rx.try_recv().is_err());
        assert!(not_in_world_rx.try_recv().is_err());
    }

    /// Backpressure on the refresh rail must not block the runtime task.
    #[test]
    fn refresh_visible_world_creatures_full_channel_counts_send_failed_like_cpp() {
        let registry = PlayerRegistry::default();
        let guid = ObjectGuid::create_player(1, 55);

        let (send_tx, _send_rx) = flume::bounded::<Vec<u8>>(1);
        let (command_tx, command_rx) = flume::bounded::<SessionCommand>(1);
        drop(command_rx);

        let mut info = player_broadcast_info_fixture_like_cpp(send_tx, command_tx, "RefreshFull");
        info.map_id = 571;
        info.instance_id = 7;
        info.is_in_world = true;
        registry.insert(guid, info);

        let summary = deliver_refresh_visible_world_creatures_like_cpp(571, 7, &registry);

        assert_eq!(summary.candidates_seen, 1);
        assert_eq!(summary.candidates_queued, 0);
        assert_eq!(summary.send_failed, 1);
    }

    #[test]
    fn collect_legacy_creature_aggro_candidates_uses_living_in_world_players_like_cpp() {
        let registry = PlayerRegistry::default();
        let in_world = ObjectGuid::create_player(1, 64);
        let not_in_world = ObjectGuid::create_player(1, 65);
        let dead_in_world = ObjectGuid::create_player(1, 66);
        let (mut in_world_info, _) =
            make_registry_player_like_cpp(571, 2, Position::new(1.0, 2.0, 3.0, 0.0), true);
        in_world_info.combat_reach = 1.5;
        in_world_info.liquid_status = wow_world::session::LIQUID_MAP_IN_WATER_LIKE_CPP;
        in_world_info.unit_flags2 = wow_constants::unit::UnitFlags2::IGNORE_REPUTATION.bits();
        in_world_info.faction_template_id = 1;
        in_world_info.reputation_standings = vec![(72, -6_000)];
        in_world_info.reputation_state_flags =
            vec![(72, wow_entities::REPUTATION_FLAG_AT_WAR_LIKE_CPP)];
        in_world_info.forced_reputation_ranks =
            vec![(87, wow_data::reputation::ReputationRankLikeCpp::Hostile)];
        in_world_info.forced_reputation_faction_ids = vec![87];
        in_world_info.is_contested_pvp = true;
        let (not_in_world_info, _) =
            make_registry_player_like_cpp(571, 2, Position::new(9.0, 9.0, 9.0, 0.0), false);
        let (mut dead_in_world_info, _) =
            make_registry_player_like_cpp(571, 2, Position::new(4.0, 4.0, 4.0, 0.0), true);
        dead_in_world_info.is_alive = false;
        registry.insert(in_world, in_world_info);
        registry.insert(not_in_world, not_in_world_info);
        registry.insert(dead_in_world, dead_in_world_info);

        let candidates = collect_legacy_creature_aggro_candidates_like_cpp(&registry);

        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].player_guid, in_world);
        assert_eq!(candidates[0].map_id, 571);
        assert_eq!(candidates[0].instance_id, 2);
        assert_eq!(candidates[0].position, Position::new(1.0, 2.0, 3.0, 0.0));
        assert_eq!(candidates[0].player_combat_reach, 1.5);
        assert_eq!(
            candidates[0].player_liquid_status_like_cpp,
            wow_world::session::LIQUID_MAP_IN_WATER_LIKE_CPP
        );
        assert_eq!(candidates[0].player_level, 1);
        assert_eq!(candidates[0].player_gray_level, 0);
        assert_eq!(
            candidates[0].player_unit_flags2,
            wow_constants::unit::UnitFlags2::IGNORE_REPUTATION.bits()
        );
        assert_eq!(candidates[0].player_faction_template_id, 1);
        assert_eq!(
            candidates[0].player_reputation_standings,
            vec![(72, -6_000)]
        );
        assert_eq!(
            candidates[0].player_reputation_state_flags,
            vec![(72, wow_entities::REPUTATION_FLAG_AT_WAR_LIKE_CPP)]
        );
        assert_eq!(
            candidates[0].player_forced_reputation_ranks,
            vec![(87, wow_data::reputation::ReputationRankLikeCpp::Hostile)]
        );
        assert_eq!(candidates[0].player_forced_reputation_faction_ids, vec![87]);
        assert!(candidates[0].player_is_contested_pvp);
    }

    #[test]
    fn creature_attack_start_delivery_routes_only_to_victim_like_cpp() {
        let registry = PlayerRegistry::default();
        let victim = ObjectGuid::create_player(1, 66);
        let other = ObjectGuid::create_player(1, 67);
        let attacker =
            ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 9001, 90_060);
        let (victim_info, victim_rx) = make_registry_player_like_cpp(571, 4, Position::ZERO, true);
        let (other_info, other_rx) = make_registry_player_like_cpp(571, 4, Position::ZERO, true);
        registry.insert(victim, victim_info);
        registry.insert(other, other_info);

        let commands = vec![
            wow_network::player_registry::CreatureAttackStartLikeCppCommand {
                attacker_guid: attacker,
                victim_guid: victim,
                map_id: 571,
                instance_id: 4,
            },
        ];
        let summary = deliver_creature_attack_start_commands_like_cpp(&commands, &registry);

        assert_eq!(summary.commands_seen, 1);
        assert_eq!(summary.candidates_seen, 1);
        assert_eq!(summary.candidates_queued, 1);
        let SessionCommand::CreatureAttackStartLikeCpp(command) =
            victim_rx.try_recv().expect("victim receives attack-start")
        else {
            panic!("expected CreatureAttackStartLikeCpp command");
        };
        assert_eq!(command.attacker_guid, attacker);
        assert_eq!(command.victim_guid, victim);
        assert!(
            other_rx.try_recv().is_err(),
            "non-victim session is untouched"
        );
    }

    #[test]
    fn creature_attack_start_delivery_filters_registry_state_like_cpp() {
        let registry = PlayerRegistry::default();
        let attacker =
            ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 9001, 90_061);
        let wrong_map = ObjectGuid::create_player(1, 68);
        let wrong_instance = ObjectGuid::create_player(1, 69);
        let not_in_world = ObjectGuid::create_player(1, 70);
        let missing = ObjectGuid::create_player(1, 71);
        let dead = ObjectGuid::create_player(1, 72);
        let (wrong_map_info, wrong_map_rx) =
            make_registry_player_like_cpp(530, 0, Position::ZERO, true);
        let (wrong_instance_info, wrong_instance_rx) =
            make_registry_player_like_cpp(571, 9, Position::ZERO, true);
        let (not_in_world_info, not_in_world_rx) =
            make_registry_player_like_cpp(571, 0, Position::ZERO, false);
        let (mut dead_info, dead_rx) = make_registry_player_like_cpp(571, 0, Position::ZERO, true);
        dead_info.is_alive = false;
        registry.insert(wrong_map, wrong_map_info);
        registry.insert(wrong_instance, wrong_instance_info);
        registry.insert(not_in_world, not_in_world_info);
        registry.insert(dead, dead_info);

        let make_command =
            |victim_guid| wow_network::player_registry::CreatureAttackStartLikeCppCommand {
                attacker_guid: attacker,
                victim_guid,
                map_id: 571,
                instance_id: 0,
            };
        let commands = vec![
            make_command(wrong_map),
            make_command(wrong_instance),
            make_command(not_in_world),
            make_command(missing),
            make_command(dead),
        ];
        let summary = deliver_creature_attack_start_commands_like_cpp(&commands, &registry);

        assert_eq!(summary.commands_seen, 5);
        assert_eq!(summary.candidates_seen, 4);
        assert_eq!(summary.candidates_queued, 0);
        assert_eq!(summary.candidates_skipped_wrong_map, 1);
        assert_eq!(summary.candidates_skipped_wrong_instance, 1);
        assert_eq!(summary.candidates_skipped_not_in_world, 1);
        assert_eq!(summary.candidates_skipped_dead, 1);
        assert_eq!(summary.candidates_skipped_missing_victim, 1);
        assert!(wrong_map_rx.try_recv().is_err());
        assert!(wrong_instance_rx.try_recv().is_err());
        assert!(not_in_world_rx.try_recv().is_err());
        assert!(dead_rx.try_recv().is_err());
    }

    /// 4C.4 bridge coverage: the combined global runtime body can perform the
    /// C++ `CreatureAI::MoveInLineOfSight`-style aggro transition once from the
    /// map owner and deliver one attack-start command to the victim session.
    #[test]
    fn legacy_creature_runtime_bridge_delivers_aggro_start_like_cpp() {
        let legacy: wow_world::SharedMapManager =
            Arc::new(std::sync::RwLock::new(wow_world::MapManager::new()));
        let canonical: wow_world::SharedCanonicalMapManager =
            Arc::new(Mutex::new(wow_map::MapManager::default()));
        canonical.lock().unwrap().create_world_map(0, 0);

        let victim = ObjectGuid::create_player(1, 93_001);
        let victim_position = Position::new(10.5, 10.5, 0.0, 0.0);
        add_canonical_test_player_on_map_like_cpp(&canonical, victim, victim_position, 0, 0, 100);

        let attacker =
            ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 9001, 93_002);
        let attacker_position = Position::new(10.0, 10.0, 0.0, 0.0);
        let mut creature = wow_world::map_manager::WorldCreature::new(
            attacker,
            9001,
            attacker_position,
            25,
            2,
            3,
            5,
            5.0,
            100,
            14,
            0,
            0,
        );
        {
            let ai = creature.creature.ai_ownership_mut();
            ai.wander_delay_ms = u64::MAX;
            ai.swing_timer_ms = u64::MAX;
        }

        {
            let mut manager = legacy.write().unwrap();
            manager.add_creature(
                0,
                0,
                wow_world::map_manager::world_to_grid_x(attacker_position.x),
                wow_world::map_manager::world_to_grid_y(attacker_position.y),
                creature,
            );
            manager.set_tick_owner(wow_world::map_manager::RuntimeTickOwner::GlobalLegacy);
        }

        let registry = PlayerRegistry::default();
        let (mut victim_info, victim_rx) =
            make_registry_player_like_cpp(0, 0, victim_position, true);
        victim_info.faction_template_id = 1;
        registry.insert(victim, victim_info);
        let wrong_map = ObjectGuid::create_player(1, 93_003);
        let (wrong_map_info, wrong_map_rx) =
            make_registry_player_like_cpp(1, 0, victim_position, true);
        registry.insert(wrong_map, wrong_map_info);

        let mmap_config = wow_world::MMapRuntimeConfigLikeCpp {
            enabled: false,
            ..Default::default()
        };
        let aggro_config = wow_world::session::LegacyCreatureAggroConfigLikeCpp {
            faction_template_store: Some(Arc::new(
                wow_data::progression_rewards::FactionTemplateStore::from_entries([
                    wow_data::progression_rewards::FactionTemplateEntry {
                        id: 14,
                        faction: 72,
                        flags: 0,
                        faction_group: 0,
                        friend_group: 0,
                        enemy_group: 0,
                        enemies: [930, 0, 0, 0, 0, 0, 0, 0],
                        friend: [0; 8],
                    },
                    wow_data::progression_rewards::FactionTemplateEntry {
                        id: 1,
                        faction: 930,
                        flags: 0,
                        faction_group: 0,
                        friend_group: 0,
                        enemy_group: 0,
                        enemies: [0; 8],
                        friend: [0; 8],
                    },
                ]),
            )),
            faction_store: Some(Arc::new(
                wow_data::progression_rewards::FactionStore::from_entries([
                    wow_data::progression_rewards::FactionEntry::for_test_like_cpp(72, 1),
                ]),
            )),
            ..Default::default()
        };
        let outcome = run_legacy_creature_runtime_tick_and_deliver_once_like_cpp(
            &legacy,
            Some(&canonical),
            &mmap_config,
            None,
            aggro_config,
            std::time::Instant::now(),
            &registry,
        );

        assert!(!outcome.aggro.skipped_owner_not_global);
        assert_eq!(outcome.aggro.maps_seen, 1);
        assert_eq!(outcome.aggro.creatures_seen, 1);
        assert_eq!(outcome.aggro.candidates_seen, 1);
        assert_eq!(outcome.aggro.aggro_starts, 1);
        assert_eq!(outcome.aggro.commands.len(), 1);
        assert_eq!(outcome.aggro_delivery.commands_seen, 1);
        assert_eq!(outcome.aggro_delivery.candidates_seen, 1);
        assert_eq!(outcome.aggro_delivery.candidates_queued, 1);
        assert_eq!(outcome.aggro_delivery.candidates_skipped_wrong_map, 0);
        assert_eq!(outcome.movement.movement_packets, 0);
        assert_eq!(outcome.melee.swings_ready, 0);

        let SessionCommand::CreatureAttackStartLikeCpp(command) = victim_rx
            .try_recv()
            .expect("victim must receive global aggro attack-start")
        else {
            panic!("expected CreatureAttackStartLikeCpp command");
        };
        assert_eq!(command.attacker_guid, attacker);
        assert_eq!(command.victim_guid, victim);
        assert_eq!(command.map_id, 0);
        assert_eq!(command.instance_id, 0);
        assert!(victim_rx.try_recv().is_err());
        assert!(wrong_map_rx.try_recv().is_err());

        let combat_target = {
            let guard = legacy.read().unwrap();
            guard
                .find_creature(0, 0, attacker)
                .unwrap()
                .creature
                .ai_ownership()
                .combat_target
        };
        assert_eq!(combat_target, Some(victim));
    }

    /// 4C.3 dormant rail: map-owned creature melee results route to exactly
    /// the victim session. C++ anchor: `Unit::AttackerStateUpdate` resolves a
    /// single melee hit for one victim, then `Unit::DealDamage` mutates health.
    #[test]
    fn creature_melee_damage_delivery_routes_only_to_victim_like_cpp() {
        let registry = PlayerRegistry::default();
        let victim = ObjectGuid::create_player(1, 56);
        let other = ObjectGuid::create_player(1, 57);
        let attacker =
            ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 9001, 90_056);
        let (victim_info, victim_rx) = make_registry_player_like_cpp(571, 3, Position::ZERO, true);
        let (other_info, other_rx) = make_registry_player_like_cpp(571, 3, Position::ZERO, true);
        registry.insert(victim, victim_info);
        registry.insert(other, other_info);

        let commands = vec![
            wow_network::player_registry::ApplyCreatureMeleeDamageLikeCppCommand {
                attacker_guid: attacker,
                victim_guid: victim,
                map_id: 571,
                instance_id: 3,
                damage: 17,
                over_damage: -1,
                target_level: 80,
                victim_health_after: 83,
            },
        ];
        let summary = deliver_creature_melee_damage_commands_like_cpp(&commands, &registry);

        assert_eq!(summary.commands_seen, 1);
        assert_eq!(summary.candidates_seen, 1);
        assert_eq!(summary.candidates_queued, 1);
        let SessionCommand::ApplyCreatureMeleeDamageLikeCpp(command) =
            victim_rx.try_recv().expect("victim receives melee command")
        else {
            panic!("expected ApplyCreatureMeleeDamageLikeCpp command");
        };
        assert_eq!(command.attacker_guid, attacker);
        assert_eq!(command.victim_guid, victim);
        assert_eq!(command.victim_health_after, 83);
        assert!(
            other_rx.try_recv().is_err(),
            "non-victim session is untouched"
        );
    }

    #[test]
    fn creature_melee_damage_delivery_filters_registry_state_like_cpp() {
        let registry = PlayerRegistry::default();
        let attacker =
            ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 9001, 90_057);
        let wrong_map = ObjectGuid::create_player(1, 58);
        let wrong_instance = ObjectGuid::create_player(1, 59);
        let not_in_world = ObjectGuid::create_player(1, 60);
        let missing = ObjectGuid::create_player(1, 61);
        let (wrong_map_info, wrong_map_rx) =
            make_registry_player_like_cpp(530, 0, Position::ZERO, true);
        let (wrong_instance_info, wrong_instance_rx) =
            make_registry_player_like_cpp(571, 9, Position::ZERO, true);
        let (not_in_world_info, not_in_world_rx) =
            make_registry_player_like_cpp(571, 0, Position::ZERO, false);
        registry.insert(wrong_map, wrong_map_info);
        registry.insert(wrong_instance, wrong_instance_info);
        registry.insert(not_in_world, not_in_world_info);

        let make_command =
            |victim_guid| wow_network::player_registry::ApplyCreatureMeleeDamageLikeCppCommand {
                attacker_guid: attacker,
                victim_guid,
                map_id: 571,
                instance_id: 0,
                damage: 5,
                over_damage: -1,
                target_level: 80,
                victim_health_after: 95,
            };
        let commands = vec![
            make_command(wrong_map),
            make_command(wrong_instance),
            make_command(not_in_world),
            make_command(missing),
        ];
        let summary = deliver_creature_melee_damage_commands_like_cpp(&commands, &registry);

        assert_eq!(summary.commands_seen, 4);
        assert_eq!(summary.candidates_seen, 3);
        assert_eq!(summary.candidates_queued, 0);
        assert_eq!(summary.candidates_skipped_wrong_map, 1);
        assert_eq!(summary.candidates_skipped_wrong_instance, 1);
        assert_eq!(summary.candidates_skipped_not_in_world, 1);
        assert_eq!(summary.candidates_skipped_missing_victim, 1);
        assert!(wrong_map_rx.try_recv().is_err());
        assert!(wrong_instance_rx.try_recv().is_err());
        assert!(not_in_world_rx.try_recv().is_err());
    }

    #[test]
    fn creature_melee_damage_delivery_full_channel_counts_send_failed_like_cpp() {
        let registry = PlayerRegistry::default();
        let victim = ObjectGuid::create_player(1, 62);
        let attacker =
            ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 9001, 90_058);
        let (send_tx, _send_rx) = flume::bounded::<Vec<u8>>(1);
        let (command_tx, command_rx) = flume::bounded::<SessionCommand>(1);
        drop(command_rx);
        let mut info = player_broadcast_info_fixture_like_cpp(send_tx, command_tx, "MeleeFull");
        info.map_id = 571;
        info.instance_id = 0;
        info.is_in_world = true;
        registry.insert(victim, info);

        let commands = vec![
            wow_network::player_registry::ApplyCreatureMeleeDamageLikeCppCommand {
                attacker_guid: attacker,
                victim_guid: victim,
                map_id: 571,
                instance_id: 0,
                damage: 5,
                over_damage: -1,
                target_level: 80,
                victim_health_after: 95,
            },
        ];
        let summary = deliver_creature_melee_damage_commands_like_cpp(&commands, &registry);

        assert_eq!(summary.commands_seen, 1);
        assert_eq!(summary.candidates_seen, 1);
        assert_eq!(summary.candidates_queued, 0);
        assert_eq!(summary.send_failed, 1);
    }

    /// 4C.3 bridge: the global creature melee body applies canonical health
    /// once, then world-server delivers the final-health command to the victim
    /// session outside all map locks.
    #[test]
    fn legacy_creature_melee_tick_delivers_victim_command_like_cpp() {
        let legacy: wow_world::SharedMapManager =
            Arc::new(std::sync::RwLock::new(wow_world::MapManager::new()));
        let canonical: wow_world::SharedCanonicalMapManager =
            Arc::new(Mutex::new(wow_map::MapManager::default()));
        canonical.lock().unwrap().create_world_map(0, 0);

        let victim = ObjectGuid::create_player(1, 63);
        add_canonical_test_player_on_map_like_cpp(&canonical, victim, Position::ZERO, 0, 0, 100);

        let attacker =
            ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 9001, 90_059);
        let attacker_position = Position::new(5.0, 5.0, 0.0, 0.0);
        let mut world_creature = wow_world::map_manager::WorldCreature::new(
            attacker,
            9001,
            attacker_position,
            25,
            2,
            3,
            5,
            20.0,
            100,
            14,
            0,
            0,
        );
        world_creature.enter_combat(victim);
        world_creature.creature.ai_ownership_mut().swing_timer_ms = 0;

        {
            let mut manager = legacy.write().unwrap();
            manager.add_creature(
                0,
                0,
                wow_world::map_manager::world_to_grid_x(attacker_position.x),
                wow_world::map_manager::world_to_grid_y(attacker_position.y),
                world_creature,
            );
            manager.set_tick_owner(wow_world::map_manager::RuntimeTickOwner::GlobalLegacy);
        }

        let registry = PlayerRegistry::default();
        let (victim_info, victim_rx) = make_registry_player_like_cpp(0, 0, Position::ZERO, true);
        registry.insert(victim, victim_info);

        let (outcome, delivery) = run_legacy_creature_melee_tick_and_deliver_once_like_cpp(
            &legacy,
            Some(&canonical),
            &registry,
        );

        assert!(!outcome.skipped_owner_not_global);
        assert_eq!(outcome.maps_seen, 1);
        assert_eq!(outcome.creatures_seen, 1);
        assert_eq!(outcome.swings_ready, 1);
        assert_eq!(outcome.canonical_hits, 1);
        assert_eq!(outcome.commands.len(), 1);
        assert_eq!(delivery.commands_seen, 1);
        assert_eq!(delivery.candidates_seen, 1);
        assert_eq!(delivery.candidates_queued, 1);

        let command = match victim_rx
            .try_recv()
            .expect("victim session receives final-health melee command")
        {
            SessionCommand::ApplyCreatureMeleeDamageLikeCpp(command) => command,
            other => panic!("expected ApplyCreatureMeleeDamageLikeCpp, got {other:?}"),
        };
        assert_eq!(command.attacker_guid, attacker);
        assert_eq!(command.victim_guid, victim);
        assert!((3..=5).contains(&command.damage));

        let canonical_health = canonical
            .lock()
            .unwrap()
            .find_map(0, 0)
            .unwrap()
            .map()
            .get_typed_player(victim)
            .unwrap()
            .unit()
            .data()
            .health;
        assert_eq!(command.victim_health_after, canonical_health);
        assert_eq!(canonical_health, 100 - u64::from(command.damage));
    }

    /// 4A.3c bridge: lifecycle changes happen once under the global owner, then
    /// matching sessions are woken to run their own visibility pass.
    #[test]
    fn legacy_creature_lifecycle_tick_refreshes_sessions_after_ready_respawn_like_cpp() {
        let legacy: wow_world::SharedMapManager =
            Arc::new(std::sync::RwLock::new(wow_world::MapManager::new()));
        let canonical: wow_world::SharedCanonicalMapManager =
            Arc::new(Mutex::new(wow_map::MapManager::default()));
        canonical.lock().unwrap().create_world_map(0, 0);
        legacy
            .write()
            .unwrap()
            .set_tick_owner(wow_world::map_manager::RuntimeTickOwner::GlobalLegacy);

        let now = std::time::Instant::now();
        let creature_guid =
            ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 9001, 90_012);
        let mut world_creature = wow_world::map_manager::WorldCreature::new(
            creature_guid,
            9001,
            Position::new(20.0, 20.0, 0.0, 0.0),
            30,
            4,
            5,
            9,
            20.0,
            100,
            14,
            0,
            0,
        );
        world_creature
            .creature
            .unit_mut()
            .world_mut()
            .phase_shift_mut()
            .add_phase_like_cpp(77, wow_constants::PhaseFlags::empty(), 1);
        let pending = wow_world::map_manager::pending_respawn_from_world_creature_like_cpp(
            &world_creature,
            now - std::time::Duration::from_secs(1),
            0,
        );
        legacy.write().unwrap().push_respawn(0, 0, pending);

        let registry = PlayerRegistry::default();
        let same_a = ObjectGuid::create_player(1, 91_001);
        let (same_a_info, same_a_rx) = make_registry_player_like_cpp(0, 0, Position::ZERO, true);
        registry.insert(same_a, same_a_info);
        let same_b = ObjectGuid::create_player(1, 91_002);
        let (same_b_info, same_b_rx) =
            make_registry_player_like_cpp(0, 0, Position::new(9000.0, 0.0, 0.0, 0.0), true);
        registry.insert(same_b, same_b_info);
        let wrong_map = ObjectGuid::create_player(1, 91_003);
        let (wrong_map_info, wrong_map_rx) =
            make_registry_player_like_cpp(1, 0, Position::ZERO, true);
        registry.insert(wrong_map, wrong_map_info);

        let (outcome, delivery) = run_legacy_creature_lifecycle_tick_and_refresh_once_like_cpp(
            &legacy,
            Some(&canonical),
            now,
            &registry,
        );

        assert!(!outcome.skipped_owner_not_global);
        assert_eq!(outcome.respawns_processed, 1);
        assert_eq!(outcome.canonical_inserts, 1);
        assert_eq!(outcome.refresh_map_keys, vec![(0, 0)]);
        assert_eq!(delivery.candidates_seen, 3);
        assert_eq!(delivery.candidates_queued, 2);
        assert_eq!(delivery.candidates_skipped_wrong_map, 1);

        for command in [
            same_a_rx.try_recv().expect("same-map session A refresh"),
            same_b_rx.try_recv().expect("same-map session B refresh"),
        ] {
            let SessionCommand::RefreshVisibleWorldCreaturesLikeCpp(command) = command else {
                panic!("expected RefreshVisibleWorldCreaturesLikeCpp command");
            };
            assert_eq!(command.map_id, 0);
            assert_eq!(command.instance_id, 0);
        }
        assert!(wrong_map_rx.try_recv().is_err());

        let canonical_guard = canonical.lock().unwrap();
        let typed = canonical_guard
            .find_map(0, 0)
            .unwrap()
            .map()
            .get_typed_creature(creature_guid)
            .expect("lifecycle bridge must sync canonical respawn");
        assert!(typed.unit().world().phase_shift().has_phase_like_cpp(77));
    }

    /// Slice 4A.4 test-only bridge: one gated global movement tick runs from a
    /// spawned task, produces a `RuntimePlan`, syncs canonical state outside
    /// the legacy lock, then delivers `SendIfVisibleLikeCpp` commands to
    /// candidate sessions.
    ///
    /// No production loop calls this yet; this only proves the cross-crate
    /// task/ownership path with `GlobalLegacy` flipped only in the test.
    #[tokio::test]
    async fn legacy_creature_global_tick_task_delivers_movement_plan_like_cpp() {
        let legacy: wow_world::SharedMapManager =
            Arc::new(std::sync::RwLock::new(wow_world::MapManager::new()));
        let canonical: wow_world::SharedCanonicalMapManager =
            Arc::new(Mutex::new(wow_map::MapManager::default()));
        canonical.lock().unwrap().create_world_map(0, 0);

        let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 9001, 90_009);
        let position = Position::new(10.0, 10.0, 0.0, 0.0);
        let mut world_creature = wow_world::map_manager::WorldCreature::new(
            guid, 9001, position, 25, 2, 3, 5, 20.0, 100, 14, 0, 0,
        );
        {
            let ai = world_creature.creature.ai_ownership_mut();
            ai.wander_delay_ms = 0;
            ai.move_start_ms = 0;
            ai.wander_radius = 3.0;
        }

        let mut canonical_creature = world_creature.creature.clone();
        canonical_creature
            .unit_mut()
            .world_mut()
            .object_mut()
            .add_to_world();
        canonical
            .lock()
            .unwrap()
            .find_map_mut(0, 0)
            .unwrap()
            .map_mut()
            .insert_map_object_record(MapObjectRecord::new_creature(canonical_creature).unwrap())
            .unwrap();

        {
            let mut manager = legacy.write().unwrap();
            manager.add_creature(
                0,
                0,
                wow_world::map_manager::world_to_grid_x(position.x),
                wow_world::map_manager::world_to_grid_y(position.y),
                world_creature,
            );
            manager.set_tick_owner(wow_world::map_manager::RuntimeTickOwner::GlobalLegacy);
        }

        let registry = Arc::new(PlayerRegistry::default());
        let near_a = ObjectGuid::create_player(1, 90_001);
        let (near_a_info, near_a_rx) =
            make_registry_player_like_cpp(0, 0, Position::new(11.0, 10.0, 999.0, 0.0), true);
        registry.insert(near_a, near_a_info);
        let near_b = ObjectGuid::create_player(1, 90_002);
        let (near_b_info, near_b_rx) =
            make_registry_player_like_cpp(0, 0, Position::new(12.0, 10.0, -999.0, 0.0), true);
        registry.insert(near_b, near_b_info);
        let wrong_map = ObjectGuid::create_player(1, 90_003);
        let (wrong_map_info, wrong_map_rx) =
            make_registry_player_like_cpp(1, 0, Position::new(10.0, 10.0, 0.0, 0.0), true);
        registry.insert(wrong_map, wrong_map_info);

        let mmap_config = wow_world::MMapRuntimeConfigLikeCpp {
            enabled: false,
            ..Default::default()
        };
        let legacy_for_task = Arc::clone(&legacy);
        let canonical_for_task = Arc::clone(&canonical);
        let registry_for_task = Arc::clone(&registry);
        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_millis(1));
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
            interval.tick().await;
            tokio::task::spawn_blocking(move || {
                run_legacy_creature_movement_tick_and_deliver_once_like_cpp(
                    &legacy_for_task,
                    Some(&canonical_for_task),
                    &mmap_config,
                    None,
                    registry_for_task.as_ref(),
                )
            })
            .await
            .expect("legacy global tick task must not panic")
        });
        let (outcome, delivery) = handle.await.expect("tick task must complete");

        assert!(!outcome.skipped_owner_not_global);
        assert_eq!(outcome.maps_seen, 1);
        assert_eq!(outcome.creatures_seen, 1);
        assert_eq!(outcome.movement_packets, 1);
        assert_eq!(outcome.canonical_syncs, 1);
        assert_eq!(delivery.events_seen, 1);
        assert_eq!(delivery.candidates_seen, 3);
        assert_eq!(delivery.candidates_queued, 2);
        assert_eq!(delivery.candidates_skipped_wrong_map, 1);

        for command in [
            near_a_rx.try_recv().expect("near player A command"),
            near_b_rx.try_recv().expect("near player B command"),
        ] {
            let SessionCommand::SendIfVisibleLikeCpp(command) = command else {
                panic!("expected SendIfVisibleLikeCpp command");
            };
            assert_eq!(command.source_guid, guid);
            assert_eq!(command.map_id, 0);
            assert_eq!(command.instance_id, 0);
            let opcode = u16::from_le_bytes([command.packet_bytes[0], command.packet_bytes[1]]);
            assert_eq!(opcode, wow_constants::ServerOpcodes::OnMonsterMove as u16);
        }
        assert!(wrong_map_rx.try_recv().is_err());

        let guard = canonical.lock().unwrap();
        let typed = guard
            .find_map(0, 0)
            .unwrap()
            .map()
            .get_typed_creature(guid)
            .expect("canonical creature record stays synced by the single-shot driver");
        assert_eq!(
            typed.ai_state(),
            wow_entities::CreatureAiState::WalkingRandom
        );
    }

    /// Combined runtime bridge: one test-only task runs lifecycle first
    /// (map-owned despawn/respawn visibility refresh), then movement
    /// (NearbyVisible MonsterMove fanout), then creature melee (explicit victim
    /// command), all while `GlobalLegacy` is enabled only inside the test.
    #[tokio::test]
    async fn legacy_creature_global_runtime_task_delivers_lifecycle_refresh_and_movement_like_cpp()
    {
        let legacy: wow_world::SharedMapManager =
            Arc::new(std::sync::RwLock::new(wow_world::MapManager::new()));
        let canonical: wow_world::SharedCanonicalMapManager =
            Arc::new(Mutex::new(wow_map::MapManager::default()));
        canonical.lock().unwrap().create_world_map(0, 0);

        let melee_victim = ObjectGuid::create_player(1, 92_004);
        add_canonical_test_player_on_map_like_cpp(
            &canonical,
            melee_victim,
            Position::ZERO,
            0,
            0,
            100,
        );

        let moving_guid =
            ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 9001, 90_013);
        let moving_position = Position::new(10.0, 10.0, 0.0, 0.0);
        let mut moving_creature = wow_world::map_manager::WorldCreature::new(
            moving_guid,
            9001,
            moving_position,
            25,
            2,
            3,
            5,
            20.0,
            100,
            14,
            0,
            0,
        );
        {
            let ai = moving_creature.creature.ai_ownership_mut();
            ai.wander_delay_ms = 0;
            ai.move_start_ms = 0;
            ai.wander_radius = 3.0;
            ai.aggro_radius = 0.0;
        }
        let mut canonical_moving = moving_creature.creature.clone();
        canonical_moving
            .unit_mut()
            .world_mut()
            .object_mut()
            .add_to_world();
        canonical
            .lock()
            .unwrap()
            .find_map_mut(0, 0)
            .unwrap()
            .map_mut()
            .insert_map_object_record(MapObjectRecord::new_creature(canonical_moving).unwrap())
            .unwrap();

        let corpse_guid =
            ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 9001, 90_014);
        let mut corpse_creature = wow_world::map_manager::WorldCreature::new(
            corpse_guid,
            9001,
            Position::new(20.0, 20.0, 0.0, 0.0),
            10,
            2,
            3,
            5,
            20.0,
            100,
            14,
            0,
            0,
        );
        corpse_creature.take_damage(10);
        corpse_creature.set_corpse_despawn_at(Some(
            std::time::Instant::now() - std::time::Duration::from_secs(1),
        ));

        let melee_guid =
            ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 9001, 90_015);
        let melee_position = Position::new(30.0, 30.0, 0.0, 0.0);
        let mut melee_creature = wow_world::map_manager::WorldCreature::new(
            melee_guid,
            9001,
            melee_position,
            25,
            2,
            3,
            5,
            20.0,
            100,
            14,
            0,
            0,
        );
        melee_creature.enter_combat(melee_victim);
        melee_creature.creature.ai_ownership_mut().swing_timer_ms = 0;

        {
            let mut manager = legacy.write().unwrap();
            manager.add_creature(
                0,
                0,
                wow_world::map_manager::world_to_grid_x(moving_position.x),
                wow_world::map_manager::world_to_grid_y(moving_position.y),
                moving_creature,
            );
            manager.add_creature(
                0,
                0,
                wow_world::map_manager::world_to_grid_x(20.0),
                wow_world::map_manager::world_to_grid_y(20.0),
                corpse_creature,
            );
            manager.add_creature(
                0,
                0,
                wow_world::map_manager::world_to_grid_x(melee_position.x),
                wow_world::map_manager::world_to_grid_y(melee_position.y),
                melee_creature,
            );
            manager.set_tick_owner(wow_world::map_manager::RuntimeTickOwner::GlobalLegacy);
        }

        let registry = Arc::new(PlayerRegistry::default());
        let near_a = ObjectGuid::create_player(1, 92_001);
        let (near_a_info, near_a_rx) =
            make_registry_player_like_cpp(0, 0, Position::new(11.0, 10.0, 999.0, 0.0), true);
        registry.insert(near_a, near_a_info);
        let near_b = ObjectGuid::create_player(1, 92_002);
        let (near_b_info, near_b_rx) =
            make_registry_player_like_cpp(0, 0, Position::new(12.0, 10.0, -999.0, 0.0), true);
        registry.insert(near_b, near_b_info);
        let wrong_map = ObjectGuid::create_player(1, 92_003);
        let (wrong_map_info, wrong_map_rx) =
            make_registry_player_like_cpp(1, 0, Position::new(10.0, 10.0, 0.0, 0.0), true);
        registry.insert(wrong_map, wrong_map_info);
        let (victim_info, victim_rx) =
            make_registry_player_like_cpp(0, 0, Position::new(5000.0, 0.0, 0.0, 0.0), true);
        registry.insert(melee_victim, victim_info);

        let mmap_config = wow_world::MMapRuntimeConfigLikeCpp {
            enabled: false,
            ..Default::default()
        };
        let legacy_for_task = Arc::clone(&legacy);
        let canonical_for_task = Arc::clone(&canonical);
        let registry_for_task = Arc::clone(&registry);
        let tick_now = std::time::Instant::now() + std::time::Duration::from_millis(1);
        let handle = tokio::spawn(async move {
            tokio::task::spawn_blocking(move || {
                run_legacy_creature_runtime_tick_and_deliver_once_like_cpp(
                    &legacy_for_task,
                    Some(&canonical_for_task),
                    &mmap_config,
                    None,
                    wow_world::session::LegacyCreatureAggroConfigLikeCpp::default(),
                    tick_now,
                    registry_for_task.as_ref(),
                )
            })
            .await
            .expect("combined legacy runtime tick task must not panic")
        });
        let outcome = handle.await.expect("combined tick task must complete");

        assert!(!outcome.lifecycle.skipped_owner_not_global);
        assert_eq!(outcome.lifecycle.maps_seen, 1);
        assert_eq!(outcome.lifecycle.creatures_seen, 3);
        assert_eq!(outcome.lifecycle.corpses_despawned, 1);
        assert_eq!(outcome.lifecycle.refresh_map_keys, vec![(0, 0)]);
        assert_eq!(outcome.lifecycle_delivery.candidates_seen, 4);
        assert_eq!(outcome.lifecycle_delivery.candidates_queued, 3);
        assert_eq!(outcome.lifecycle_delivery.candidates_skipped_wrong_map, 1);

        assert!(!outcome.movement.skipped_owner_not_global);
        assert_eq!(outcome.movement.maps_seen, 1);
        assert_eq!(outcome.movement.creatures_seen, 2);
        assert_eq!(outcome.movement.movement_packets, 1);
        assert_eq!(outcome.movement_delivery.events_seen, 1);
        assert_eq!(outcome.movement_delivery.candidates_seen, 4);
        assert_eq!(outcome.movement_delivery.candidates_queued, 2);
        assert_eq!(outcome.movement_delivery.candidates_skipped_distance, 1);
        assert_eq!(outcome.movement_delivery.candidates_skipped_wrong_map, 1);

        assert!(!outcome.aggro.skipped_owner_not_global);
        assert_eq!(outcome.aggro.maps_seen, 1);
        assert_eq!(outcome.aggro.creatures_seen, 2);
        assert_eq!(outcome.aggro.candidates_seen, 3);
        assert_eq!(outcome.aggro.aggro_starts, 0);
        assert_eq!(outcome.aggro_delivery.commands_seen, 0);
        assert_eq!(outcome.aggro_delivery.candidates_queued, 0);

        assert!(!outcome.melee.skipped_owner_not_global);
        assert_eq!(outcome.melee.maps_seen, 1);
        assert_eq!(outcome.melee.creatures_seen, 2);
        assert_eq!(outcome.melee.swings_ready, 1);
        assert_eq!(outcome.melee.canonical_hits, 1);
        assert_eq!(outcome.melee_delivery.commands_seen, 1);
        assert_eq!(outcome.melee_delivery.candidates_seen, 1);
        assert_eq!(outcome.melee_delivery.candidates_queued, 1);

        for command_rx in [&near_a_rx, &near_b_rx] {
            let SessionCommand::RefreshVisibleWorldCreaturesLikeCpp(refresh) = command_rx
                .try_recv()
                .expect("same-map player must receive lifecycle refresh")
            else {
                panic!("expected RefreshVisibleWorldCreaturesLikeCpp command");
            };
            assert_eq!(refresh.map_id, 0);
            assert_eq!(refresh.instance_id, 0);

            let SessionCommand::SendIfVisibleLikeCpp(move_command) = command_rx
                .try_recv()
                .expect("same-map player must receive movement command")
            else {
                panic!("expected SendIfVisibleLikeCpp command");
            };
            assert_eq!(move_command.source_guid, moving_guid);
            assert_eq!(move_command.map_id, 0);
            assert_eq!(move_command.instance_id, 0);
            let opcode =
                u16::from_le_bytes([move_command.packet_bytes[0], move_command.packet_bytes[1]]);
            assert_eq!(opcode, wow_constants::ServerOpcodes::OnMonsterMove as u16);
        }

        let SessionCommand::RefreshVisibleWorldCreaturesLikeCpp(refresh) = victim_rx
            .try_recv()
            .expect("victim same-map session must receive lifecycle refresh")
        else {
            panic!("expected RefreshVisibleWorldCreaturesLikeCpp command for victim");
        };
        assert_eq!(refresh.map_id, 0);
        assert_eq!(refresh.instance_id, 0);
        let SessionCommand::ApplyCreatureMeleeDamageLikeCpp(melee_command) = victim_rx
            .try_recv()
            .expect("victim must receive the creature melee command")
        else {
            panic!("expected ApplyCreatureMeleeDamageLikeCpp command for victim");
        };
        assert_eq!(melee_command.attacker_guid, melee_guid);
        assert_eq!(melee_command.victim_guid, melee_victim);
        assert!((3..=5).contains(&melee_command.damage));
        assert!(
            victim_rx.try_recv().is_err(),
            "victim is outside movement range"
        );
        assert!(wrong_map_rx.try_recv().is_err());

        {
            let guard = legacy.read().unwrap();
            assert!(
                guard.find_creature(0, 0, corpse_guid).is_none(),
                "expired corpse must be removed by lifecycle before movement"
            );
            assert!(
                guard.find_creature(0, 0, moving_guid).is_some(),
                "alive moving creature must remain in the legacy map"
            );
            assert!(
                guard.find_creature(0, 0, melee_guid).is_some(),
                "alive melee creature must remain in the legacy map"
            );
        }
        let guard = canonical.lock().unwrap();
        let typed = guard
            .find_map(0, 0)
            .unwrap()
            .map()
            .get_typed_creature(moving_guid)
            .expect("movement phase must keep canonical moving creature synced");
        assert_eq!(
            typed.ai_state(),
            wow_entities::CreatureAiState::WalkingRandom
        );
        let victim_health = guard
            .find_map(0, 0)
            .unwrap()
            .map()
            .get_typed_player(melee_victim)
            .unwrap()
            .unit()
            .data()
            .health;
        assert_eq!(victim_health, melee_command.victim_health_after);
        assert_eq!(victim_health, 100 - u64::from(melee_command.damage));
    }
}
