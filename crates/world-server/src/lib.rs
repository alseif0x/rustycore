// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! World Server — composition library.
//!
//! Accepts WoW client connections after BNet authentication, performs the
//! world-server handshake (challenge → auth → encryption), creates a
//! WorldSession for each client, and dispatches packets to handlers.

use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::future::Future;
use std::net::{Ipv4Addr, SocketAddr};
use std::path::PathBuf;
use std::pin::Pin;
use std::process::ExitCode;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicI32, AtomicU32, AtomicU64, Ordering},
};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, bail};
use tokio::sync::Notify;
use tokio::task::AbortHandle;
use tracing::{debug, info, warn};
use wow_config::{DatabaseInfo, LoadReport, WorldConfigSet};
use wow_core::{
    EQUIPMENT_SET_GUID_LIMIT_LIKE_CPP, EquipmentSetGuidGeneratorLikeCpp, IpLocationStore,
    Ipv4NetworkLikeCpp, ObjectGuid, ObjectGuidGenerator, Position,
    VOID_STORAGE_ITEM_ID_LIMIT_LIKE_PACKET_GUID, VoidStorageItemIdGeneratorLikeCpp, guid::HighGuid,
    scan_local_ipv4_networks_like_cpp,
};
use wow_database::{
    CharStatements, CharacterDatabase, HotfixDatabase, ItemGuidAllocatorAdvisoryLockLikeCpp,
    LoginBattlePetPersistenceLikeCpp, LoginDatabase, LoginStatements, SqlResult, SqlTransaction,
    StatementDef, WorldDatabase, WorldStatements, warn_about_sync_queries_scope_like_cpp,
};
use wow_instances::{InstanceLockMgr, MapDb2Entries, ResetSchedule};
use wow_loot::{
    LootStoreKind,
    check_loot_condition_links_like_cpp, check_loot_condition_references_like_cpp,
    check_loot_references_like_cpp, loot_store_kind_for_condition_source_type_like_cpp,
};
use wow_network::session_mgr::SessionManager;
use wow_network::world_socket::{AccountInfo, AccountLookup};
use wow_network::{SocketTimeoutsLikeCpp, WorldListenerPolicyLikeCpp};
use wow_packet::{
    ServerPacket,
    packets::chat::{ChatMsg, ChatPkt},
};
use wow_persistence::{
    RespawnPersistenceKeyLikeCpp, RespawnPersistenceLoadOutcomeLikeCpp,
    RespawnPersistenceMutationLikeCpp, RespawnPersistenceMutationOutcomeLikeCpp,
    RespawnPersistencePortLikeCpp, RespawnPersistenceRowLikeCpp,
};
use wow_social::group::{
    GroupDbRowLikeCpp, GroupLoadSummaryLikeCpp, GroupMemberCharacterLikeCpp,
    GroupMemberDbRowLikeCpp, GroupRegistry, PendingInvites, ReadyCheckEventLikeCpp,
    load_groups_from_db_rows_like_cpp, tick_all_group_ready_checks_like_cpp,
};
use wow_world::session::directory::PlayerRegistry;
use wow_world::session::mailbox::{
    GameEventQuestCompleteCommandLikeCpp, GameEventQuestCompleteResponseLikeCpp,
    KickLikeCppCommand, ResetSeasonalQuestStatusCommand, SendVisibleObjectValuesUpdateCommand,
    SessionCommand, WorldSessionShutdownFlushLikeCppCommand,
    WorldSessionShutdownFlushResultLikeCpp,
};
use wow_world::{
    BattlePetAccountRegistryLikeCpp, ChatFloodConfigLikeCpp, ChatLevelRequirementsLikeCpp,
    ChatListenRangesLikeCpp, LootDropRatesLikeCpp, MMapRuntimeConfigLikeCpp,
    MapManager as LegacyMapManager, PacketSpoofConfigLikeCpp, ReputationRatesLikeCpp,
    SharedCanonicalMapManager, SharedMapManager, WorldMMapPathfinderWorkerLikeCpp, WorldSession,
    conditions::{
        ConditionMapRef, ConditionMapStateSnapshot, is_spawn_group_meeting_map_conditions_like_cpp,
    },
    entity_update_bridge::unit_values_update_to_packet,
};

mod area;
mod catalogs;
mod creature_loaded_grid;
mod gameobject_loaded_grid;
mod hotfix;
mod hotfix_delivery_metadata;
mod player;
mod realm_list;
mod respawn_bootstrap;
mod respawn_db_writer;
mod session_supervision;
mod session_resources;
mod spawn_store_loader;
mod spell;

use realm_list::{
    realms_state_update_delay_secs_like_cpp,
    normalize_realm_type_like_cpp,
    is_pvp_realm_type_like_cpp,
    is_ffa_pvp_realm_type_like_cpp,
    normalize_realm_security_level_like_cpp,
    normalized_realm_name_like_cpp,
    realm_list_entry_from_row_like_cpp,
    realm_list_snapshot_from_result_like_cpp,
    update_realm_list_once_like_cpp,
    spawn_realm_list_update_loop_like_cpp,
};
use respawn_bootstrap::{
    load_persisted_respawn_times_like_cpp,
    persisted_respawn_info_from_row_like_cpp,
    install_canonical_spawn_group_initializer_like_cpp,
};
use respawn_db_writer::{
    respawn_db_retry_delay,
    execute_respawn_db_attempt_like_cpp,
    spawn_respawn_db_writer_like_cpp,
    stop_respawn_db_producer_like_cpp,
    drain_respawn_db_writer_like_cpp,
};
use session_resources::{
    SessionCoreCapabilitiesLikeCpp, SessionInventoryCapabilitiesLikeCpp,
    SessionPlayerCatalogCapabilitiesLikeCpp, SessionProgressionCapabilitiesLikeCpp,
    SessionRealmCapabilitiesLikeCpp, SessionResources, SessionRuntimePolicyCapabilitiesLikeCpp,
    SessionSpellCatalogCapabilitiesLikeCpp, SessionWorldCatalogCapabilitiesLikeCpp,
};

const WORLD_CONFIG_CANDIDATES: &[&str] = &[
    "worldserver.conf",
    "worldserver.conf.dist",
    "WorldServer.conf",
    "WorldServer.conf.dist",
];

fn next_item_guid_allocator_start_like_cpp(max_persisted_guid: Option<u64>) -> Result<i64> {
    let next = max_persisted_guid
        .unwrap_or(0)
        .checked_add(1)
        .context("item_instance GUID counter overflow")?;
    let next = i64::try_from(next)
        .context("item_instance GUID counter exceeds the supported integer range")?;
    let generator_limit = ObjectGuid::max_counter(HighGuid::Item) - 1;
    if next >= generator_limit {
        bail!(
            "item_instance GUID allocator start {next} is outside HighGuid::Item generator range (must be below {generator_limit})"
        );
    }
    Ok(next)
}

fn next_equipment_set_guid_allocator_start_like_cpp(
    max_persisted_guid: Option<u64>,
) -> Result<u64> {
    let next = max_persisted_guid
        .unwrap_or(0)
        .checked_add(1)
        .context("equipment-set GUID counter overflow")?;
    if next >= EQUIPMENT_SET_GUID_LIMIT_LIKE_CPP {
        bail!(
            "equipment-set GUID allocator start {next} is outside the C++ generator range (must be below {EQUIPMENT_SET_GUID_LIMIT_LIKE_CPP})"
        );
    }
    Ok(next)
}

fn next_void_storage_item_id_allocator_start_like_cpp(
    max_persisted_id: Option<u64>,
) -> Result<u64> {
    let next = max_persisted_id
        .unwrap_or(0)
        .checked_add(1)
        .context("void-storage item ID counter overflow")?;
    if next >= VOID_STORAGE_ITEM_ID_LIMIT_LIKE_PACKET_GUID {
        bail!(
            "void-storage item ID allocator start {next} is outside the packet GUID counter range (must be below {VOID_STORAGE_ITEM_ID_LIMIT_LIKE_PACKET_GUID})"
        );
    }
    Ok(next)
}

// The first four statements mirror C++ `ObjectMgr::SetHighestGuids`. C++ does
// not clean orphaned stored-loot rows, but Rust loads those rows by item GUID on
// demand; clean them before publishing the allocator so a future item cannot
// inherit loot left behind by a deleted container.
const ITEM_GUID_DANGLING_REFERENCE_CLEANUP_STATEMENTS_LIKE_CPP: [CharStatements; 6] = [
    CharStatements::DEL_INVALID_CHAR_INVENTORY_ITEM_GUIDS,
    CharStatements::DEL_INVALID_MAIL_ITEM_GUIDS,
    CharStatements::DEL_INVALID_AUCTION_ITEM_GUIDS,
    CharStatements::DEL_INVALID_GUILD_BANK_ITEM_GUIDS,
    CharStatements::DEL_INVALID_ITEM_LOOT_ITEMS_GUIDS,
    CharStatements::DEL_INVALID_ITEM_LOOT_MONEY_GUIDS,
];

fn item_guid_reference_cleanup_transaction_like_cpp(
    char_db: &CharacterDatabase,
    next_item_guid: u64,
) -> SqlTransaction {
    let mut transaction = SqlTransaction::new();
    for statement_id in ITEM_GUID_DANGLING_REFERENCE_CLEANUP_STATEMENTS_LIKE_CPP {
        let mut statement = char_db.prepare(statement_id);
        statement.set_u64(0, next_item_guid);
        transaction.append(statement);
    }
    transaction
}
const WORLD_CONFIG_DIR: &str = "worldserver.conf.d";
const RUSTYCORE_LEGACY_CREATURE_GLOBAL_RUNTIME_CONFIG: &str =
    "RustyCore.LegacyCreatureGlobalRuntime";
const DEFAULT_RESPAWN_MIN_CHECK_INTERVAL_MS: u32 = 5_000;
const RESPAWN_DB_RETRY_INITIAL_DELAY: Duration = Duration::from_secs(1);
const RESPAWN_DB_RETRY_MAX_DELAY: Duration = Duration::from_secs(30);
const RESPAWN_DB_PRODUCER_STOP_TIMEOUT: Duration = Duration::from_secs(10);
const RESPAWN_DB_WRITER_DRAIN_TIMEOUT: Duration = Duration::from_secs(10);
const CREATURE_TYPE_MECHANICAL_LIKE_CPP: u32 = 9;
const CREATURE_TYPE_FLAG_BOSS_MOB_LIKE_CPP: u32 = 0x0001_0000;
const HARDCODED_DEVELOPMENT_REALM_CATEGORY_ID_LIKE_CPP: u32 = 1;
const CFG_CATEGORIES_CHARSET_RUSSIAN_LIKE_CPP: u8 = 0x04;

type RespawnDbMutationKeyLikeCpp = RespawnPersistenceKeyLikeCpp;
type SharedRespawnDbMutationOrderLikeCpp = Arc<Mutex<()>>;
type SharedRespawnDbProducerStopLikeCpp = Arc<AtomicBool>;

/// Keeps the latest respawn DB operation per spawn for the shared DB writer.
///
/// C++ submits each respawn statement once from `Map::SaveRespawnInfoDB` /
/// `Map::DeleteRespawnInfoFromDB` and executes it on the CharacterDatabase
/// worker. RustyCore mirrors that ownership with one writer for canonical and
/// legacy producers, avoiding both map-tick stalls and cross-runtime REP/DEL
/// reordering. Retry cadence is independent per spawn key.
#[derive(Debug)]
struct PendingRespawnDbMutationLikeCpp {
    mutation: RespawnPersistenceMutationLikeCpp,
    consecutive_failures: u32,
    retry_not_before: Instant,
}

#[derive(Debug, Default)]
struct RespawnDbRetryQueueLikeCpp {
    pending: BTreeMap<RespawnDbMutationKeyLikeCpp, PendingRespawnDbMutationLikeCpp>,
}

#[derive(Debug)]
struct RespawnDbAttemptLikeCpp {
    key: RespawnDbMutationKeyLikeCpp,
    pending: PendingRespawnDbMutationLikeCpp,
}

#[derive(Debug, Default)]
struct RespawnDbMailboxStateLikeCpp {
    queue: RespawnDbRetryQueueLikeCpp,
    closed: bool,
}

/// Producer-visible, latest-per-spawn mailbox for respawn persistence.
///
/// Producers coalesce synchronously before waking the DB writer, so a slow or
/// unavailable CharacterDatabase cannot create an unbounded event backlog.
/// The retained domain is bounded by distinct spawn keys with pending durable
/// state; repeated REP/DEL operations for one key always replace each other.
#[derive(Debug, Default)]
struct RespawnDbMailboxLikeCpp {
    state: Mutex<RespawnDbMailboxStateLikeCpp>,
    notify: Notify,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RespawnDbSubmitErrorLikeCpp {
    Closed,
}

#[derive(Debug, Clone)]
struct RespawnDbWriterSenderLikeCpp {
    mailbox: Arc<RespawnDbMailboxLikeCpp>,
}

struct RespawnDbWriterTaskGuardLikeCpp {
    mailbox: Arc<RespawnDbMailboxLikeCpp>,
}

enum RespawnDbWriterPollLikeCpp {
    Attempt(RespawnDbAttemptLikeCpp),
    WaitForNotification,
    WaitUntil(Instant),
    Finished,
}

type SharedCanonicalSpawnMetadataLikeCpp =
    Arc<Mutex<spawn_store_loader::CanonicalSpawnMetadataLikeCpp>>;
type SharedWorldStateMgrLikeCpp = Arc<Mutex<spawn_store_loader::WorldStateMgrLikeCpp>>;
type SharedRealmListLikeCpp = Arc<Mutex<RealmListSnapshotLikeCpp>>;

const SHUTDOWN_EXIT_CODE_LIKE_CPP: i32 = 0;
const ERROR_EXIT_CODE_LIKE_CPP: i32 = 1;
const RESTART_EXIT_CODE_LIKE_CPP: i32 = 2;
const WORLD_SESSION_SHUTDOWN_FLUSH_TIMEOUT_LIKE_CPP: Duration = Duration::from_millis(500);
const WORLD_SESSION_SHUTDOWN_DRAIN_TIMEOUT_LIKE_CPP: Duration = Duration::from_millis(500);
const WORLD_SESSION_FINALIZE_STEP_TIMEOUT_LIKE_CPP: Duration = Duration::from_secs(5);
const WORLD_SESSION_FORCE_CANCEL_TIMEOUT_LIKE_CPP: Duration = Duration::from_secs(12);
const REALM_TYPE_NORMAL_LIKE_CPP: u8 = 0;
const REALM_TYPE_PVP_LIKE_CPP: u8 = 1;
const REALM_TYPE_RPPVP_LIKE_CPP: u8 = 8;
const MAX_CLIENT_REALM_TYPE_LIKE_CPP: u8 = 14;
const REALM_TYPE_FFA_PVP_LIKE_CPP: u8 = 16;
const SEC_ADMINISTRATOR_LIKE_CPP: u8 = 3;

#[derive(Debug)]
struct WorldRuntimeStateLikeCpp {
    stop_event: AtomicBool,
    exit_code: AtomicI32,
    world_loop_counter: AtomicU32,
}

impl WorldRuntimeStateLikeCpp {
    fn new() -> Self {
        Self {
            stop_event: AtomicBool::new(false),
            exit_code: AtomicI32::new(SHUTDOWN_EXIT_CODE_LIKE_CPP),
            world_loop_counter: AtomicU32::new(0),
        }
    }

    fn is_stopped_like_cpp(&self) -> bool {
        self.stop_event.load(Ordering::Acquire)
    }

    fn stop_now_like_cpp(&self, exit_code: i32) {
        self.exit_code.store(exit_code, Ordering::Release);
        self.stop_event.store(true, Ordering::Release);
    }

    fn get_exit_code_like_cpp(&self) -> i32 {
        self.exit_code.load(Ordering::Acquire)
    }

    fn increment_world_loop_counter_like_cpp(&self) -> u32 {
        self.world_loop_counter.fetch_add(1, Ordering::AcqRel) + 1
    }

    fn world_loop_counter_like_cpp(&self) -> u32 {
        self.world_loop_counter.load(Ordering::Acquire)
    }
}

#[derive(Debug, Clone, Copy)]
struct RealmHandleLikeCpp {
    region: u8,
    site: u8,
    realm: u32,
}

#[derive(Debug, Clone, PartialEq)]
struct RealmListEntryLikeCpp {
    id: RealmHandleLikeCpp,
    build: u32,
    name: String,
    normalized_name: String,
    address: String,
    local_address: String,
    port: u16,
    icon: u8,
    flag: u8,
    timezone: u8,
    allowed_security_level: u8,
    population: f32,
}

#[derive(Debug, Clone, Default, PartialEq)]
struct RealmListSnapshotLikeCpp {
    realms: BTreeMap<RealmHandleLikeCpp, RealmListEntryLikeCpp>,
    sub_regions: BTreeSet<String>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct RealmListRefreshSummaryLikeCpp {
    realms: usize,
    sub_regions: usize,
    added: usize,
    updated: usize,
    removed: usize,
}

#[derive(Debug, Clone, PartialEq)]
struct RealmListRawRowLikeCpp {
    realm_id: u32,
    name: String,
    address: String,
    local_address: String,
    port: u16,
    icon: u8,
    flag: u8,
    timezone: u8,
    allowed_security_level: u8,
    population: f32,
    build: u32,
    region: u8,
    battlegroup: u8,
}

fn process_exit_code_like_cpp(exit_code: i32) -> ExitCode {
    let exit_code = u8::try_from(exit_code).unwrap_or(1);
    ExitCode::from(exit_code)
}

#[derive(Debug, Default)]
struct ActiveWorldSessionCancellationLikeCpp {
    cancelled: AtomicBool,
    notify: Notify,
}

#[derive(Clone, Debug)]
struct ActiveWorldSessionLikeCpp {
    account_id: u32,
    command_tx: flume::Sender<SessionCommand>,
    /// The session's phase rail (#787). Registration alone does not mean the
    /// session is consuming it: `ready_for_phases_like_cpp` is set by the task
    /// that owns the session once it actually parks on the rail, so the
    /// producer never counts a session that cannot answer.
    phase_tx: flume::Sender<wow_world::session::mailbox::SessionPhaseRequestLikeCpp>,
    ready_for_phases_like_cpp: Arc<AtomicBool>,
    cancellation: Arc<ActiveWorldSessionCancellationLikeCpp>,
}

#[derive(Debug)]
struct ActiveWorldSessionRegistrationGuardLikeCpp {
    registry: Arc<ActiveWorldSessionRegistryLikeCpp>,
    id: u64,
}

/// Minimal Rust equivalent of C++ `World::m_sessions`.
///
/// C++ `World::KickAll` / `World::UpdateSessions` operate on all active
/// `WorldSession` objects, including authenticated sessions still on the
/// character screen. `PlayerRegistry` is not enough because it only contains
/// sessions with a logged-in player. This registry intentionally stores only
/// the command rail needed by world-owned operations; the session task remains
/// the sole owner of `WorldSession` mutation.
#[derive(Debug)]
struct ActiveWorldSessionRegistryLikeCpp {
    next_id: AtomicU64,
    inner: Mutex<session_supervision::ActiveSessionState>,
    coordination_changed: tokio::sync::Notify,
    coordination_issuer: u64,
    stop_sessions: AtomicBool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FreezeDetectorPollOutcomeLikeCpp {
    Advanced,
    StillAlive,
    Abort { stuck_ms: u32 },
}

#[derive(Debug)]
struct FreezeDetectorLikeCpp {
    world_loop_counter: u32,
    last_change_ms_time: u32,
    max_core_stuck_time_in_ms: u32,
}

impl FreezeDetectorLikeCpp {
    fn new(max_core_stuck_time_in_ms: u32, start_ms_time: u32) -> Self {
        Self {
            world_loop_counter: 0,
            last_change_ms_time: start_ms_time,
            max_core_stuck_time_in_ms,
        }
    }

    fn poll_once_like_cpp(
        &mut self,
        current_ms_time: u32,
        world_loop_counter: u32,
    ) -> FreezeDetectorPollOutcomeLikeCpp {
        if self.world_loop_counter != world_loop_counter {
            self.last_change_ms_time = current_ms_time;
            self.world_loop_counter = world_loop_counter;
            return FreezeDetectorPollOutcomeLikeCpp::Advanced;
        }

        let ms_time_diff = current_ms_time.wrapping_sub(self.last_change_ms_time);
        if ms_time_diff > self.max_core_stuck_time_in_ms {
            FreezeDetectorPollOutcomeLikeCpp::Abort {
                stuck_ms: ms_time_diff,
            }
        } else {
            FreezeDetectorPollOutcomeLikeCpp::StillAlive
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WorldUpdateLoopStepOutcomeLikeCpp {
    Sleep {
        sleep_ms: u32,
        log_waiting_like_cpp: bool,
    },
    Update {
        diff_ms: u32,
        next_real_prev_time_ms: u32,
    },
}

fn half_max_core_stuck_time_like_cpp(max_core_stuck_time_ms: u32) -> u32 {
    let half = max_core_stuck_time_ms / 2;
    if half == 0 { u32::MAX } else { half }
}

fn world_update_loop_step_like_cpp(
    world: &WorldRuntimeStateLikeCpp,
    real_prev_time_ms: u32,
    real_curr_time_ms: u32,
    min_update_diff_ms: u32,
    max_core_stuck_time_ms: u32,
) -> WorldUpdateLoopStepOutcomeLikeCpp {
    world.increment_world_loop_counter_like_cpp();

    let diff_ms = real_curr_time_ms.wrapping_sub(real_prev_time_ms);
    if diff_ms < min_update_diff_ms {
        let sleep_ms = min_update_diff_ms - diff_ms;
        return WorldUpdateLoopStepOutcomeLikeCpp::Sleep {
            sleep_ms,
            log_waiting_like_cpp: sleep_ms
                >= half_max_core_stuck_time_like_cpp(max_core_stuck_time_ms),
        };
    }

    WorldUpdateLoopStepOutcomeLikeCpp::Update {
        diff_ms,
        next_real_prev_time_ms: real_curr_time_ms,
    }
}

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
            //  1: a.session_key_bnet    (64 raw bytes; hex-encoded below for auth helper)
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
            let mutetime: i64 = result.try_read(6).unwrap_or(0);
            let locale_raw: String = result
                .try_read::<u8>(7)
                .map(|v| v.to_string())
                .unwrap_or_else(|| result.try_read::<String>(7).unwrap_or_default());
            let recruiter: u32 = result.try_read(8).unwrap_or(0);
            let os: String = result.try_read(9).unwrap_or_default();
            let timezone_offset: i16 = result.try_read(10).unwrap_or(0);
            let Some(bnet_id) = result.try_read::<u32>(11).filter(|id| *id != 0) else {
                tracing::warn!(
                    "Game account {account_id} has no valid Battle.net account link; rejecting world authentication"
                );
                return None;
            };
            let security: u8 = result.try_read(12).unwrap_or(0);
            let is_banned_bnet: u32 = result.try_read(13).unwrap_or(0);
            let is_banned_account: u32 = result.try_read(14).unwrap_or(0);
            let is_a_recruiter = result.try_read::<u32>(15).unwrap_or(0) != 0;

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
                mute_time: mutetime,
                locale: locale_name,
                recruiter,
                is_a_recruiter,
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

mod app;
pub use app::{run, run_with_modules};

async fn set_realm_online(login_db: &LoginDatabase, realm_id: u16) -> Result<()> {
    login_db
        .direct_execute(&set_realm_online_sql_like_cpp(realm_id))
        .await
        .context("Failed to mark realm online")?;

    info!("Realm {realm_id} marked online");
    Ok(())
}

async fn clear_online_accounts_like_cpp(
    login_db: &LoginDatabase,
    character_db: &CharacterDatabase,
    realm_id: u16,
) -> Result<()> {
    let [account_sql, character_sql, battleground_sql] =
        clear_online_accounts_sql_like_cpp(realm_id);

    login_db
        .direct_execute(&account_sql)
        .await
        .context("Failed to clear stale online account flags")?;
    character_db
        .direct_execute(&character_sql)
        .await
        .context("Failed to clear stale online character flags")?;
    character_db
        .direct_execute(&battleground_sql)
        .await
        .context("Failed to clear stale battleground instance ids")?;

    info!("Cleared stale online account state for realm {realm_id}");
    Ok(())
}

fn clear_online_accounts_sql_like_cpp(realm_id: u16) -> [String; 3] {
    [
        format!(
            "UPDATE account SET online = 0 WHERE online > 0 AND id IN (SELECT acctid FROM realmcharacters WHERE realmid = {realm_id})"
        ),
        "UPDATE characters SET online = 0 WHERE online <> 0".to_string(),
        "UPDATE character_battleground_data SET instanceId = 0".to_string(),
    ]
}

fn create_pid_file_from_config_like_cpp() -> Result<Option<u32>> {
    let pid_file = wow_config::get_string_default("PidFile", "");
    if pid_file.is_empty() {
        return Ok(None);
    }

    let pid = create_pid_file_like_cpp(&pid_file)
        .with_context(|| format!("Cannot create PID file {pid_file}"))?;
    info!("Daemon PID: {pid}");
    Ok(Some(pid))
}

fn create_pid_file_like_cpp(path: impl AsRef<std::path::Path>) -> std::io::Result<u32> {
    let pid = std::process::id();
    std::fs::write(path, pid.to_string())?;
    Ok(pid)
}

fn load_ip_location_from_config_like_cpp() -> IpLocationStore {
    info!("Loading IP Location Database...");
    let database_file_path = wow_config::get_string_default("IPLocationFile", "");
    if database_file_path.is_empty() {
        return IpLocationStore::default();
    }

    if !PathBuf::from(&database_file_path).exists() {
        tracing::error!("IPLocation: No ip database file exists ({database_file_path}).");
        return IpLocationStore::default();
    }

    let contents = match std::fs::read_to_string(&database_file_path) {
        Ok(contents) => contents,
        Err(error) => {
            tracing::error!(
                "IPLocation: Ip database file ({database_file_path}) can not be opened: {error}"
            );
            return IpLocationStore::default();
        }
    };

    let store = IpLocationStore::from_csv_like_cpp(&contents);
    info!(">> Loaded {} ip location entries.", store.len());
    store
}

async fn set_realm_offline(login_db: &LoginDatabase, realm_id: u16) -> Result<()> {
    login_db
        .direct_execute(&set_realm_offline_sql_like_cpp(realm_id))
        .await
        .context("Failed to mark realm offline")?;

    info!("Realm {realm_id} marked offline");
    Ok(())
}

fn set_realm_offline_sql_like_cpp(realm_id: u16) -> String {
    const REALM_FLAG_OFFLINE: u8 = 0x02;
    format!("UPDATE realmlist SET flag = flag | {REALM_FLAG_OFFLINE} WHERE id = {realm_id}")
}

fn set_realm_online_sql_like_cpp(realm_id: u16) -> String {
    const REALM_FLAG_OFFLINE: u8 = 0x02;
    format!(
        "UPDATE realmlist SET flag = flag & ~{REALM_FLAG_OFFLINE}, population = 0 WHERE id = {realm_id}"
    )
}

fn db_keepalive_interval_minutes_like_cpp(configs: &WorldConfigSet) -> u32 {
    world_config_u32(configs, "CONFIG_DB_PING_INTERVAL", 30)
}

const REQUIRED_TDB_VERSION_LIKE_CPP: &str = "TDB 343.24081";
const REQUIRED_TDB_CACHE_ID_LIKE_CPP: i32 = 24081;
const UNKNOWN_WORLD_DATABASE_LIKE_CPP: &str = "Unknown world database.";

#[derive(Debug, Clone, PartialEq, Eq)]
struct WorldDbVersionLikeCpp {
    db_version: String,
    cache_id: i32,
}

fn world_db_version_matches_required_like_cpp(version: &WorldDbVersionLikeCpp) -> bool {
    version.db_version == REQUIRED_TDB_VERSION_LIKE_CPP
        && version.cache_id == REQUIRED_TDB_CACHE_ID_LIKE_CPP
}

fn world_db_version_mismatch_message_like_cpp(version: Option<&WorldDbVersionLikeCpp>) -> String {
    let found = version
        .map(|version| format!("{} / cache_id {}", version.db_version, version.cache_id))
        .unwrap_or_else(|| UNKNOWN_WORLD_DATABASE_LIKE_CPP.to_string());

    format!(
        "World database version mismatch: expected {REQUIRED_TDB_VERSION_LIKE_CPP} / cache_id {REQUIRED_TDB_CACHE_ID_LIKE_CPP}, found {found}"
    )
}

async fn load_world_db_version_like_cpp(
    world_db: &WorldDatabase,
) -> Result<Option<WorldDbVersionLikeCpp>> {
    let stmt = world_db.prepare(WorldStatements::SEL_WORLD_DB_VERSION);
    let result = world_db
        .query(&stmt)
        .await
        .context("Failed to query world database version")?;

    if result.is_empty() {
        return Ok(None);
    }

    let db_version = result.read_string(0);
    if db_version.is_empty() {
        return Ok(None);
    }

    Ok(Some(WorldDbVersionLikeCpp {
        db_version,
        cache_id: result.try_read(1).unwrap_or(0),
    }))
}

async fn verify_world_db_version_like_cpp(world_db: &WorldDatabase) -> Result<()> {
    let version = load_world_db_version_like_cpp(world_db).await?;
    if version
        .as_ref()
        .is_some_and(world_db_version_matches_required_like_cpp)
    {
        let version = version.expect("checked Some above");
        info!(
            db_version = %version.db_version,
            cache_id = version.cache_id,
            "Using World DB"
        );
        return Ok(());
    }

    anyhow::bail!(
        "{}",
        world_db_version_mismatch_message_like_cpp(version.as_ref())
    );
}

#[cfg(test)]
fn db_keepalive_sql_like_cpp() -> &'static str {
    wow_database::database::KEEP_ALIVE_SQL_LIKE_CPP
}

fn db_keepalive_database_names_like_cpp() -> [&'static str; 3] {
    ["Character", "Login", "World"]
}

fn spawn_db_keepalive_loop_like_cpp(
    character_db: Arc<CharacterDatabase>,
    login_db: Arc<LoginDatabase>,
    world_db: Arc<WorldDatabase>,
    interval_minutes: u32,
) -> Option<tokio::task::JoinHandle<()>> {
    if interval_minutes == 0 {
        warn!("MaxPingTime is 0; database keep-alive loop disabled");
        return None;
    }

    Some(tokio::spawn(async move {
        let [character_name, login_name, world_name] = db_keepalive_database_names_like_cpp();
        let interval = Duration::from_secs(u64::from(interval_minutes) * 60);
        loop {
            tokio::time::sleep(interval).await;
            debug!("Ping MySQL to keep connection alive");
            keepalive_mysql_database_like_cpp(character_name, &character_db).await;
            keepalive_mysql_database_like_cpp(login_name, &login_db).await;
            keepalive_mysql_database_like_cpp(world_name, &world_db).await;
        }
    }))
}

async fn keepalive_mysql_database_like_cpp<S: StatementDef>(
    name: &str,
    db: &wow_database::Database<S>,
) {
    if let Err(error) = db.keep_alive_like_cpp().await {
        warn!("MySQL keep-alive failed for {name} database: {error}");
    }
}

mod shutdown;
use shutdown::*;

mod bootstrap;
mod skill_world_rules;
mod static_data_overlay;
mod loot_catalog;
mod world;
use bootstrap::*;

pub(crate) use loot_catalog::{
    load_condition_reference_template_ids_like_cpp,
    load_loot_condition_ids_like_cpp,
    load_loot_condition_reference_uses_like_cpp,
    load_loot_stores_like_cpp,
    load_loot_template_rows_like_cpp,
    log_loot_condition_link_report_like_cpp,
    log_loot_reference_report_like_cpp,
    loot_quest_required_from_signed_db_like_cpp,
    loot_store_all_rows_statement_like_cpp,
};

#[derive(Debug, Clone, Default)]
struct PersistedRespawnTimesLikeCpp {
    by_map: BTreeMap<wow_map::MapKey, Vec<wow_map::RespawnInfoLikeCpp>>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct PersistedRespawnLoadReportLikeCpp {
    rows: usize,
    loaded: usize,
    invalid_type: usize,
    unsupported_area_trigger: usize,
    missing_spawn_metadata: usize,
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
    skipped_instanceable_map: usize,
}

mod session_factory;
use session_factory::*;

mod runtime;
use runtime::*;

#[cfg(test)]
#[path = "main_tests.rs"]
mod tests;
