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
    LootConditionId, LootConditionLinkReport, LootConditionReferenceUseLikeCpp,
    LootReferenceCheckReport, LootStore, LootStoreKind, LootStores, LootTemplateRow,
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

mod area_trigger_loaded_grid;
mod area_trigger_template_catalog;
mod area_trigger_world_catalog;
mod battle_pet_selection_catalog;
mod condition_disable_catalog;
mod creature_display_hotfix;
mod creature_loaded_grid;
mod difficulty_hotfix;
mod exploration_base_xp_catalog;
mod game_tele_catalog;
mod gameobject_loaded_grid;
mod gameplay_rule_catalog;
mod gossip_startup_catalog;
mod hotfix_delivery_metadata;
mod item_random_enchantment_catalog;
mod jump_charge_catalog;
mod lfg_dungeons_hotfix;
mod lfg_world_catalog;
mod mount_catalog;
mod phase_hotfix_catalog;
mod phase_world_catalog;
mod player_choice_catalog;
mod player_creation_catalog;
mod quest_catalog;
mod quest_item_catalog;
mod reputation_catalog;
mod reserved_name_catalog;
mod session_resources;
mod spawn_store_loader;
mod spell_acquisition_loader;
mod spell_core_db2_hotfix;
mod spell_info_key_hotfix;
mod spell_world_catalog;
mod trainer_catalog;
mod vehicle_catalog;

use session_resources::SessionResources;

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

impl RespawnDbRetryQueueLikeCpp {
    fn enqueue_latest(
        &mut self,
        mutation: RespawnPersistenceMutationLikeCpp,
        now: Instant,
    ) -> RespawnDbMutationKeyLikeCpp {
        let key = mutation.key();
        self.pending.insert(
            key,
            PendingRespawnDbMutationLikeCpp {
                mutation,
                consecutive_failures: 0,
                retry_not_before: now,
            },
        );
        key
    }

    fn next_deadline(&self) -> Option<Instant> {
        self.pending
            .values()
            .map(|pending| pending.retry_not_before)
            .min()
    }

    fn take_due(&mut self, now: Instant) -> Option<RespawnDbAttemptLikeCpp> {
        let key = self
            .pending
            .iter()
            .filter(|(_, pending)| pending.retry_not_before <= now)
            .min_by_key(|(key, pending)| (pending.retry_not_before, **key))
            .map(|(key, _)| *key)?;
        self.pending
            .remove(&key)
            .map(|pending| RespawnDbAttemptLikeCpp { key, pending })
    }

    fn retry_failed(
        &mut self,
        mut attempt: RespawnDbAttemptLikeCpp,
        observed_at: Instant,
    ) -> (Duration, u32) {
        attempt.pending.consecutive_failures =
            attempt.pending.consecutive_failures.saturating_add(1);
        let delay = respawn_db_retry_delay(attempt.pending.consecutive_failures);
        attempt.pending.retry_not_before = observed_at.checked_add(delay).unwrap_or(observed_at);
        let failed_count = attempt.pending.consecutive_failures;
        // Keep the failed operation only until the writer receives a newer
        // operation for the same key; `enqueue_latest` then replaces it.
        self.pending.entry(attempt.key).or_insert(attempt.pending);
        (delay, failed_count)
    }

    fn pending_len(&self) -> usize {
        self.pending.len()
    }

    fn make_all_due(&mut self, now: Instant) {
        for pending in self.pending.values_mut() {
            pending.retry_not_before = now;
        }
    }
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

impl RespawnDbMailboxLikeCpp {
    fn close_like_cpp(&self) {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        state.closed = true;
        state.queue.make_all_due(Instant::now());
        drop(state);
        self.notify.notify_one();
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RespawnDbSubmitErrorLikeCpp {
    Closed,
}

#[derive(Debug, Clone)]
struct RespawnDbWriterSenderLikeCpp {
    mailbox: Arc<RespawnDbMailboxLikeCpp>,
}

impl RespawnDbWriterSenderLikeCpp {
    fn new_like_cpp() -> Self {
        Self {
            mailbox: Arc::new(RespawnDbMailboxLikeCpp::default()),
        }
    }

    fn send(
        &self,
        mutation: RespawnPersistenceMutationLikeCpp,
    ) -> std::result::Result<(), RespawnDbSubmitErrorLikeCpp> {
        let mut state = self
            .mailbox
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if state.closed {
            return Err(RespawnDbSubmitErrorLikeCpp::Closed);
        }
        state.queue.enqueue_latest(mutation, Instant::now());
        drop(state);
        self.mailbox.notify.notify_one();
        Ok(())
    }

    fn close_like_cpp(&self) {
        self.mailbox.close_like_cpp();
    }
}

struct RespawnDbWriterTaskGuardLikeCpp {
    mailbox: Arc<RespawnDbMailboxLikeCpp>,
}

impl Drop for RespawnDbWriterTaskGuardLikeCpp {
    fn drop(&mut self) {
        // Reject further producer submissions if the supervised writer exits,
        // panics, or is aborted unexpectedly.
        self.mailbox.close_like_cpp();
    }
}

enum RespawnDbWriterPollLikeCpp {
    Attempt(RespawnDbAttemptLikeCpp),
    WaitForNotification,
    WaitUntil(Instant),
    Finished,
}

fn respawn_db_retry_delay(consecutive_failed_flushes: u32) -> Duration {
    let exponent = consecutive_failed_flushes.saturating_sub(1).min(31);
    let multiplier = 1_u32.checked_shl(exponent).unwrap_or(u32::MAX);
    RESPAWN_DB_RETRY_INITIAL_DELAY
        .saturating_mul(multiplier)
        .min(RESPAWN_DB_RETRY_MAX_DELAY)
}

async fn execute_respawn_db_attempt_like_cpp(
    attempt: RespawnDbAttemptLikeCpp,
    mailbox: &RespawnDbMailboxLikeCpp,
    respawn_persistence: &dyn RespawnPersistencePortLikeCpp,
) {
    if let RespawnPersistenceMutationOutcomeLikeCpp::Failed { reason } = respawn_persistence
        .execute_mutation_like_cpp(attempt.pending.mutation)
        .await
    {
        let key = attempt.key;
        let (retry_delay, failed_attempts) = mailbox
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .queue
            .retry_failed(attempt, Instant::now());
        warn!(
            error = %reason,
            object_type = key.object_type_raw,
            spawn_id = key.spawn_id,
            map_id = key.map_id,
            instance_id = key.instance_id,
            retry_in_ms = retry_delay.as_millis(),
            failed_attempts,
            "Failed to persist respawn operation like C++; shared DB-writer retry deferred"
        );
    }
}

fn spawn_respawn_db_writer_like_cpp(
    respawn_persistence: Arc<dyn RespawnPersistencePortLikeCpp>,
) -> (RespawnDbWriterSenderLikeCpp, tokio::task::JoinHandle<()>) {
    let sender = RespawnDbWriterSenderLikeCpp::new_like_cpp();
    let mailbox = Arc::clone(&sender.mailbox);
    let handle = tokio::spawn(async move {
        let _writer_guard = RespawnDbWriterTaskGuardLikeCpp {
            mailbox: Arc::clone(&mailbox),
        };

        loop {
            // Arm the notification before inspecting the mailbox. A producer
            // racing this check therefore leaves a permit instead of losing
            // the idle-writer wakeup.
            let notified = mailbox.notify.notified();
            let poll = {
                let mut state = mailbox
                    .state
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                let now = Instant::now();
                if let Some(attempt) = state.queue.take_due(now) {
                    RespawnDbWriterPollLikeCpp::Attempt(attempt)
                } else if state.closed && state.queue.pending_len() == 0 {
                    RespawnDbWriterPollLikeCpp::Finished
                } else if let Some(deadline) = state.queue.next_deadline() {
                    RespawnDbWriterPollLikeCpp::WaitUntil(deadline)
                } else {
                    RespawnDbWriterPollLikeCpp::WaitForNotification
                }
            };

            match poll {
                RespawnDbWriterPollLikeCpp::Attempt(attempt) => {
                    // Never hold the mailbox mutex across a database await.
                    // If a newer same-key operation arrives while this SQL is
                    // in flight, `retry_failed(...).or_insert(...)` preserves
                    // that newer operation instead of restoring stale state.
                    execute_respawn_db_attempt_like_cpp(
                        attempt,
                        mailbox.as_ref(),
                        respawn_persistence.as_ref(),
                    )
                    .await;
                }
                RespawnDbWriterPollLikeCpp::WaitForNotification => notified.await,
                RespawnDbWriterPollLikeCpp::WaitUntil(deadline) => {
                    tokio::select! {
                        _ = notified => {}
                        _ = tokio::time::sleep_until(tokio::time::Instant::from_std(deadline)) => {}
                    }
                }
                RespawnDbWriterPollLikeCpp::Finished => break,
            }
        }
    });
    (sender, handle)
}

async fn stop_respawn_db_producer_like_cpp(
    task_name: &'static str,
    handle: &mut tokio::task::JoinHandle<()>,
    already_finished: bool,
) -> bool {
    if already_finished {
        return true;
    }

    match tokio::time::timeout(RESPAWN_DB_PRODUCER_STOP_TIMEOUT, &mut *handle).await {
        Ok(Ok(())) => true,
        Ok(Err(error)) => {
            tracing::error!(task_name, %error, "Respawn DB producer task failed during shutdown");
            false
        }
        Err(_) => {
            // `spawn_blocking` cannot be force-cancelled. Aborting the outer
            // task prevents further async iterations; a still-running closure
            // may attempt a late mailbox submission, which explicit writer
            // close rejects before the separately bounded drain.
            handle.abort();
            tracing::error!(
                task_name,
                timeout_ms = RESPAWN_DB_PRODUCER_STOP_TIMEOUT.as_millis(),
                "Respawn DB producer shutdown timed out; abort requested and terminal error status required"
            );
            false
        }
    }
}

async fn drain_respawn_db_writer_like_cpp(
    handle: &mut tokio::task::JoinHandle<()>,
    already_finished: bool,
) -> bool {
    if already_finished {
        return false;
    }

    match tokio::time::timeout(RESPAWN_DB_WRITER_DRAIN_TIMEOUT, &mut *handle).await {
        Ok(Ok(())) => true,
        Ok(Err(error)) => {
            tracing::error!(%error, "Shared respawn DB writer failed while draining");
            false
        }
        Err(_) => {
            tracing::error!(
                timeout_ms = RESPAWN_DB_WRITER_DRAIN_TIMEOUT.as_millis(),
                "Shared respawn DB writer drain timed out; aborting with persistence work still pending"
            );
            handle.abort();
            let _ = (&mut *handle).await;
            false
        }
    }
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

impl RealmHandleLikeCpp {
    fn new_like_cpp(region: u8, site: u8, realm: u32) -> Self {
        Self {
            region,
            site,
            realm,
        }
    }

    fn address_like_cpp(self) -> u32 {
        (u32::from(self.region) << 24) | (u32::from(self.site) << 16) | (self.realm & 0xFFFF)
    }

    #[cfg(test)]
    fn address_string_like_cpp(self) -> String {
        format!("{}-{}-{}", self.region, self.site, self.realm)
    }

    fn sub_region_address_like_cpp(self) -> String {
        format!("{}-{}-0", self.region, self.site)
    }
}

impl PartialEq for RealmHandleLikeCpp {
    fn eq(&self, other: &Self) -> bool {
        self.realm == other.realm
    }
}

impl Eq for RealmHandleLikeCpp {}

impl PartialOrd for RealmHandleLikeCpp {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for RealmHandleLikeCpp {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.realm.cmp(&other.realm)
    }
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

impl RealmListSnapshotLikeCpp {
    fn replace_like_cpp(&mut self, next: Self) -> RealmListRefreshSummaryLikeCpp {
        let added = next
            .realms
            .keys()
            .filter(|handle| !self.realms.contains_key(handle))
            .count();
        let updated = next
            .realms
            .keys()
            .filter(|handle| self.realms.contains_key(handle))
            .count();
        let removed = self
            .realms
            .keys()
            .filter(|handle| !next.realms.contains_key(handle))
            .count();
        let realms = next.realms.len();
        let sub_regions = next.sub_regions.len();

        *self = next;

        RealmListRefreshSummaryLikeCpp {
            realms,
            sub_regions,
            added,
            updated,
            removed,
        }
    }

    #[cfg(test)]
    fn get_realm_like_cpp(&self, handle: RealmHandleLikeCpp) -> Option<&RealmListEntryLikeCpp> {
        self.realms.get(&handle)
    }

    fn get_realm_by_id_like_cpp(&self, realm_id: u32) -> Option<&RealmListEntryLikeCpp> {
        self.realms
            .get(&RealmHandleLikeCpp::new_like_cpp(0, 0, realm_id))
    }
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

impl ActiveWorldSessionCancellationLikeCpp {
    fn cancel_like_cpp(&self) {
        self.cancelled.store(true, Ordering::Release);
        // One cancellation waiter exists per session; `notify_one` stores a
        // permit when the waiter has not been polled yet, avoiding a lost wake.
        self.notify.notify_one();
    }

    async fn cancelled_like_cpp(&self) {
        loop {
            let notified = self.notify.notified();
            if self.cancelled.load(Ordering::Acquire) {
                return;
            }
            notified.await;
        }
    }
}

#[derive(Clone, Debug)]
struct ActiveWorldSessionLikeCpp {
    account_id: u32,
    command_tx: flume::Sender<SessionCommand>,
    cancellation: Arc<ActiveWorldSessionCancellationLikeCpp>,
}

#[derive(Debug)]
struct ActiveWorldSessionRegistrationGuardLikeCpp {
    registry: Arc<ActiveWorldSessionRegistryLikeCpp>,
    id: u64,
}

impl Drop for ActiveWorldSessionRegistrationGuardLikeCpp {
    fn drop(&mut self) {
        self.registry.unregister(self.id);
    }
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
    sessions: Mutex<BTreeMap<u64, ActiveWorldSessionLikeCpp>>,
    accepting_sessions: AtomicBool,
    stop_sessions: AtomicBool,
}

impl Default for ActiveWorldSessionRegistryLikeCpp {
    fn default() -> Self {
        Self {
            next_id: AtomicU64::new(0),
            sessions: Mutex::new(BTreeMap::new()),
            accepting_sessions: AtomicBool::new(true),
            stop_sessions: AtomicBool::new(false),
        }
    }
}

impl ActiveWorldSessionRegistryLikeCpp {
    fn new() -> Self {
        Self::default()
    }

    fn try_register(
        &self,
        account_id: u32,
        command_tx: flume::Sender<SessionCommand>,
    ) -> Option<(u64, Arc<ActiveWorldSessionCancellationLikeCpp>)> {
        let mut sessions = self
            .sessions
            .lock()
            .expect("active world session registry lock poisoned");
        if !self.accepting_sessions.load(Ordering::Acquire) {
            return None;
        }
        let id = self
            .next_id
            .fetch_add(1, Ordering::Relaxed)
            .saturating_add(1);
        let cancellation = Arc::new(ActiveWorldSessionCancellationLikeCpp::default());
        sessions.insert(
            id,
            ActiveWorldSessionLikeCpp {
                account_id,
                command_tx,
                cancellation: Arc::clone(&cancellation),
            },
        );
        Some((id, cancellation))
    }

    #[cfg(test)]
    fn register(&self, account_id: u32, command_tx: flume::Sender<SessionCommand>) -> u64 {
        self.try_register(account_id, command_tx)
            .expect("test registry must still accept sessions")
            .0
    }

    fn begin_shutdown_like_cpp(&self) {
        // Serialize with `try_register`: when this method returns, every
        // registration that won the race is already visible in `sessions`,
        // and every later registration is rejected.
        let _sessions = self
            .sessions
            .lock()
            .expect("active world session registry lock poisoned");
        self.accepting_sessions.store(false, Ordering::Release);
    }

    fn is_shutting_down_like_cpp(&self) -> bool {
        !self.accepting_sessions.load(Ordering::Acquire)
    }

    fn request_session_stop_like_cpp(&self) {
        self.stop_sessions.store(true, Ordering::Release);
    }

    fn should_stop_sessions_like_cpp(&self) -> bool {
        self.stop_sessions.load(Ordering::Acquire)
    }

    fn cancel_all_sessions_like_cpp(&self) -> usize {
        debug_assert!(self.is_shutting_down_like_cpp());
        let sessions = self.snapshot_like_cpp();
        for (_, session) in &sessions {
            session.cancellation.cancel_like_cpp();
        }
        sessions.len()
    }

    fn unregister(&self, id: u64) -> Option<ActiveWorldSessionLikeCpp> {
        let mut sessions = self
            .sessions
            .lock()
            .expect("active world session registry lock poisoned");
        sessions.remove(&id)
    }

    fn snapshot_like_cpp(&self) -> Vec<(u64, ActiveWorldSessionLikeCpp)> {
        let sessions = self
            .sessions
            .lock()
            .expect("active world session registry lock poisoned");
        sessions
            .iter()
            .map(|(id, session)| (*id, session.clone()))
            .collect()
    }

    fn len_like_cpp(&self) -> usize {
        self.sessions
            .lock()
            .expect("active world session registry lock poisoned")
            .len()
    }

    fn is_empty_like_cpp(&self) -> bool {
        self.len_like_cpp() == 0
    }

    async fn wait_until_empty_like_cpp(&self, wait_timeout: Duration) -> bool {
        tokio::time::timeout(wait_timeout, async {
            while !self.is_empty_like_cpp() {
                // Polling avoids losing a `notify_waiters` wake between an
                // empty check and creation of a `Notified` future.
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .is_ok()
    }

    #[cfg(test)]
    fn len(&self) -> usize {
        self.len_like_cpp()
    }
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

fn realms_state_update_delay_secs_like_cpp() -> u32 {
    wow_config::get_value_default("RealmsStateUpdateDelay", 10i32).max(0) as u32
}

fn normalize_realm_type_like_cpp(icon: u8) -> u8 {
    if icon == REALM_TYPE_FFA_PVP_LIKE_CPP {
        return REALM_TYPE_PVP_LIKE_CPP;
    }

    if icon >= MAX_CLIENT_REALM_TYPE_LIKE_CPP {
        return REALM_TYPE_NORMAL_LIKE_CPP;
    }

    icon
}

fn is_pvp_realm_type_like_cpp(icon: u32) -> bool {
    matches!(
        icon,
        value if value == u32::from(REALM_TYPE_PVP_LIKE_CPP)
            || value == u32::from(REALM_TYPE_RPPVP_LIKE_CPP)
            || value == u32::from(REALM_TYPE_FFA_PVP_LIKE_CPP)
    )
}

fn is_ffa_pvp_realm_type_like_cpp(icon: u32) -> bool {
    icon == u32::from(REALM_TYPE_FFA_PVP_LIKE_CPP)
}

fn normalize_realm_security_level_like_cpp(level: u8) -> u8 {
    level.min(SEC_ADMINISTRATOR_LIKE_CPP)
}

fn normalized_realm_name_like_cpp(name: &str) -> String {
    name.chars()
        .filter(|ch| !ch.is_ascii_whitespace())
        .collect()
}

fn realm_list_entry_from_row_like_cpp(row: RealmListRawRowLikeCpp) -> RealmListEntryLikeCpp {
    let id = RealmHandleLikeCpp::new_like_cpp(row.region, row.battlegroup, row.realm_id);
    let normalized_name = normalized_realm_name_like_cpp(&row.name);
    RealmListEntryLikeCpp {
        id,
        build: row.build,
        name: row.name,
        normalized_name,
        address: row.address,
        local_address: row.local_address,
        port: row.port,
        icon: normalize_realm_type_like_cpp(row.icon),
        flag: row.flag,
        timezone: row.timezone,
        allowed_security_level: normalize_realm_security_level_like_cpp(row.allowed_security_level),
        population: row.population,
    }
}

fn realm_list_snapshot_from_result_like_cpp(result: &mut SqlResult) -> RealmListSnapshotLikeCpp {
    let mut snapshot = RealmListSnapshotLikeCpp::default();
    if result.is_empty() {
        return snapshot;
    }

    loop {
        let Some(fields) = result.fetch_like_cpp() else {
            break;
        };
        let entry = realm_list_entry_from_row_like_cpp(RealmListRawRowLikeCpp {
            realm_id: fields.try_read(0).unwrap_or(0),
            name: fields.read_string(1),
            address: fields.read_string(2),
            local_address: fields.read_string(3),
            port: fields.try_read(4).unwrap_or(0),
            icon: fields.try_read(5).unwrap_or(REALM_TYPE_NORMAL_LIKE_CPP),
            flag: fields.try_read(6).unwrap_or(0),
            timezone: fields.try_read(7).unwrap_or(0),
            allowed_security_level: fields.try_read(8).unwrap_or(SEC_ADMINISTRATOR_LIKE_CPP),
            population: fields.try_read(9).unwrap_or(0.0),
            build: fields.try_read(10).unwrap_or(0),
            region: fields.try_read(11).unwrap_or(0),
            battlegroup: fields.try_read(12).unwrap_or(0),
        });

        snapshot
            .sub_regions
            .insert(entry.id.sub_region_address_like_cpp());
        snapshot.realms.insert(entry.id, entry);

        if !result.next_row() {
            break;
        }
    }

    snapshot
}

async fn update_realm_list_once_like_cpp(
    login_db: &LoginDatabase,
    realm_list: &SharedRealmListLikeCpp,
) -> Result<RealmListRefreshSummaryLikeCpp> {
    let stmt = login_db.prepare(LoginStatements::SEL_REALMLIST);
    let mut result = login_db
        .query(&stmt)
        .await
        .context("Failed to query C++ LOGIN_SEL_REALMLIST")?;
    let next_snapshot = realm_list_snapshot_from_result_like_cpp(&mut result);
    let mut realm_list = realm_list.lock().expect("realm list mutex poisoned");
    Ok(realm_list.replace_like_cpp(next_snapshot))
}

fn spawn_realm_list_update_loop_like_cpp(
    login_db: LoginDatabase,
    realm_list: SharedRealmListLikeCpp,
    update_interval_secs: u32,
) -> Option<tokio::task::JoinHandle<()>> {
    if update_interval_secs == 0 {
        warn!("RealmsStateUpdateDelay is 0; RealmList background refresh disabled");
        return None;
    }

    Some(tokio::spawn(async move {
        let interval = Duration::from_secs(u64::from(update_interval_secs));
        loop {
            tokio::time::sleep(interval).await;
            match update_realm_list_once_like_cpp(&login_db, &realm_list).await {
                Ok(summary) => {
                    debug!(
                        realms = summary.realms,
                        sub_regions = summary.sub_regions,
                        added = summary.added,
                        updated = summary.updated,
                        removed = summary.removed,
                        "Updated RealmList from realmlist like C++"
                    );
                }
                Err(error) => {
                    warn!("RealmList background refresh failed: {error:#}");
                }
            }
        }
    }))
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
mod chr_specialization_hotfix;
mod player_base_stats;
mod skill_catalog_hotfix;
mod skill_world_rules;
mod static_data_overlay;
mod world_auxiliary_catalog;
mod world_object_catalog;
mod world_reference_catalog;
use bootstrap::*;

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
        // Trinity's QuestRequired column is a signed TINYINT(1). Reading it
        // as u8 makes sqlx reject the signed MySQL type; defaulting that
        // decode failure to zero turns every quest-only drop into normal
        // loot. Fail startup instead of silently disabling the quest gate.
        let quest_required = result
            .try_read::<i8>(4)
            .with_context(|| format!("failed to decode {kind:?} loot QuestRequired as TINYINT"))?;
        rows.push(LootTemplateRow {
            entry: result.try_read::<u32>(0).unwrap_or(0),
            item: wow_loot::LootStoreItem {
                item_id: result.try_read::<u32>(1).unwrap_or(0),
                reference: result.try_read::<u32>(2).unwrap_or(0),
                chance: result.try_read::<f32>(3).unwrap_or(0.0),
                needs_quest: loot_quest_required_from_signed_db_like_cpp(quest_required),
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

const fn loot_quest_required_from_signed_db_like_cpp(value: i8) -> bool {
    value != 0
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

async fn load_persisted_respawn_times_like_cpp(
    respawn_persistence: &dyn RespawnPersistencePortLikeCpp,
    canonical_spawn_metadata: &spawn_store_loader::CanonicalSpawnMetadataLikeCpp,
) -> Result<(
    PersistedRespawnTimesLikeCpp,
    PersistedRespawnLoadReportLikeCpp,
)> {
    let rows = match respawn_persistence.load_all_like_cpp().await {
        RespawnPersistenceLoadOutcomeLikeCpp::Loaded(rows) => rows,
        RespawnPersistenceLoadOutcomeLikeCpp::Failed { reason } => {
            bail!("failed to load persisted respawn times: {reason}")
        }
    };
    let mut snapshot = PersistedRespawnTimesLikeCpp::default();
    let mut report = PersistedRespawnLoadReportLikeCpp::default();

    for row in rows {
        if let Some((key, info)) =
            persisted_respawn_info_from_row_like_cpp(row, canonical_spawn_metadata, &mut report)
        {
            snapshot.push(key, info);
        }
    }

    Ok((snapshot, report))
}

fn persisted_respawn_info_from_row_like_cpp(
    row: RespawnPersistenceRowLikeCpp,
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
    skipped_instanceable_map: usize,
}

fn apply_persisted_respawns_to_managed_map_like_cpp(
    managed_map: &mut wow_map::ManagedMap,
    persisted_respawn_times: &PersistedRespawnTimesLikeCpp,
    map_store: &wow_data::MapStore,
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
    if map_store
        .get(managed_map.map_id())
        .is_some_and(|entry| entry.is_instanceable_like_cpp())
    {
        // C++ `Map::LoadRespawnTimes` returns immediately for every
        // `MapEntry::Instanceable()` map. `ManagedMapKind::World` alone is not
        // sufficient because garrisons are represented by that kind too.
        report.skipped_instanceable_map = respawns.len();
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
    map_store: Arc<wow_data::MapStore>,
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
            map_store.as_ref(),
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
                skipped_instanceable_map = respawn_report.skipped_instanceable_map,
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

mod session_factory;
use session_factory::*;

mod runtime;
use runtime::*;

#[cfg(test)]
#[path = "main_tests.rs"]
mod tests;
