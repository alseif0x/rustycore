//! Canonical account-scoped battle-pet ownership.
//!
//! C++ stores the journal in each `WorldSession::BattlePetMgr` and relies on
//! `World::IsBattlePetJournalLockAcquired` to keep one session authoritative.
//! Its capacity check and `AddPet` mutation are nevertheless separate.  This
//! module preserves the journal/packet model while deliberately closing that
//! race: lease validation, capacity, GUID reservation, durable insert and
//! canonical publication are one result-bearing operation.

use std::collections::{HashMap, HashSet};
use std::future::Future;
use std::pin::Pin;
use std::sync::{
    Arc, Mutex, Weak,
    atomic::{AtomicU64, Ordering},
};
use std::time::{Duration, Instant};

use dashmap::DashMap;
use sqlx::{MySql, Row, Transaction};
use tokio::{
    sync::{Notify, OnceCell, mpsc, oneshot, watch},
    time::MissedTickBehavior,
};
use wow_core::{ObjectGuid, guid::HighGuid};
use wow_data::{
    BATTLE_PET_SPECIES_FLAG_LEGACY_ACCOUNT_UNIQUE_LIKE_CPP,
    BATTLE_PET_SPECIES_FLAG_NOT_ACCOUNT_WIDE_LIKE_CPP, BATTLE_PET_SPECIES_FLAG_WELL_KNOWN_LIKE_CPP,
    BattlePetBreedQualityStore, BattlePetBreedStateStore, BattlePetSpeciesStateStore,
    BattlePetSpeciesStore, calculate_battle_pet_stats_like_cpp,
};
use wow_database::{
    DatabaseError, LoginDatabase, LoginStatements, SqlTransaction, SqlTransactionCommitError,
};
use wow_packet::packets::misc::{
    BattlePetJournal, BattlePetJournalPet, BattlePetJournalPetOwnerInfo, BattlePetJournalSlot,
    DeclinedNamesLikeCpp, empty_battle_pet_guid_like_cpp,
};

use crate::session::{
    BATTLE_PET_SLOT_COUNT_LIKE_CPP, DEFAULT_MAX_BATTLE_PETS_PER_SPECIES_LIKE_CPP,
    RepresentedBattlePetDataLikeCpp, RepresentedBattlePetSaveInfoLikeCpp,
    RepresentedBattlePetSlotLikeCpp,
};

type PersistenceFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

const BATTLE_PET_PROCESS_LEASE_VERIFY_INTERVAL_LIKE_CPP: Duration = Duration::from_secs(30);

pub(crate) trait BattlePetProcessLeaseLikeCpp: Send {
    fn is_valid_like_cpp(&self) -> bool {
        true
    }

    fn fence_like_cpp(&self) -> u64 {
        1
    }
}

struct BattlePetProcessLeaseStateLikeCpp {
    guard: Option<Box<dyn BattlePetProcessLeaseLikeCpp>>,
    acquiring: bool,
    attachments: usize,
    active_operations: usize,
    lease_holder: Option<BattlePetLeaseIdLikeCpp>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct BattlePetAddRequestKeyLikeCpp([u8; 16]);

impl BattlePetAddRequestKeyLikeCpp {
    pub(crate) fn from_bytes(bytes: [u8; 16]) -> Self {
        Self(bytes)
    }

    /// Durable uncage identity. Item GUIDs are allocated from the
    /// process-lifetime guarded item GUID domain and are never reused, unlike
    /// the represented process-local spell cast counter.
    pub(crate) fn from_source_item_guid_like_cpp(source_item_guid: ObjectGuid) -> Option<Self> {
        (!source_item_guid.is_empty()).then(|| Self(source_item_guid.to_raw_bytes()))
    }

    pub(crate) fn as_bytes(self) -> [u8; 16] {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DurableBattlePetRowLikeCpp {
    pub guid_counter: u64,
    pub species: u32,
    pub breed: u16,
    pub display_id: u32,
    pub level: u16,
    pub exp: u16,
    pub health: u32,
    pub quality: u8,
    pub flags: u16,
    pub name: String,
    pub name_timestamp: i64,
    pub owner_guid_counter: Option<u64>,
    pub declined_names: Option<DeclinedNamesLikeCpp>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct DurableBattlePetSlotLikeCpp {
    pub index: u8,
    pub pet_guid_counter: Option<u64>,
    pub locked: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LoadedBattlePetAccountLikeCpp {
    pub pets: Vec<DurableBattlePetRowLikeCpp>,
    pub slots: Vec<DurableBattlePetSlotLikeCpp>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DurableBattlePetAddLikeCpp {
    pub account_id: u32,
    pub realm_id: u16,
    pub request_key: BattlePetAddRequestKeyLikeCpp,
    pub max_per_scope: u8,
    pub fence: u64,
    pub pet: DurableBattlePetRowLikeCpp,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DurableBattlePetAddReceiptLikeCpp {
    pub account_id: u32,
    pub requested_pet: DurableBattlePetRowLikeCpp,
    pub current_pet: Option<DurableBattlePetRowLikeCpp>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PersistBattlePetAddOutcomeLikeCpp {
    Inserted,
    Replayed {
        pet: DurableBattlePetRowLikeCpp,
        still_present: bool,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum BattlePetPersistenceErrorLikeCpp {
    Database(String),
    Capacity,
    GuidCollision,
    DuplicateRequest,
    StaleAuthority,
}

pub(crate) trait BattlePetPersistenceLikeCpp: Send + Sync {
    fn try_acquire_process_lease<'a>(
        &'a self,
        account_id: u32,
    ) -> PersistenceFuture<
        'a,
        Result<Option<Box<dyn BattlePetProcessLeaseLikeCpp>>, BattlePetPersistenceErrorLikeCpp>,
    >;

    fn load_account<'a>(
        &'a self,
        account_id: u32,
        realm_id: u16,
    ) -> PersistenceFuture<
        'a,
        Result<LoadedBattlePetAccountLikeCpp, BattlePetPersistenceErrorLikeCpp>,
    >;

    fn allocate_guid_counter_like_cpp(
        &self,
    ) -> PersistenceFuture<'_, Result<u64, BattlePetPersistenceErrorLikeCpp>>;

    fn insert_pet_idempotently<'a>(
        &'a self,
        request: DurableBattlePetAddLikeCpp,
    ) -> PersistenceFuture<
        'a,
        Result<PersistBattlePetAddOutcomeLikeCpp, BattlePetPersistenceErrorLikeCpp>,
    >;

    fn lookup_add_request<'a>(
        &'a self,
        account_id: u32,
        request_key: BattlePetAddRequestKeyLikeCpp,
    ) -> PersistenceFuture<
        'a,
        Result<Option<DurableBattlePetAddReceiptLikeCpp>, BattlePetPersistenceErrorLikeCpp>,
    >;

    fn update_pet<'a>(
        &'a self,
        account_id: u32,
        fence: u64,
        pet: DurableBattlePetRowLikeCpp,
    ) -> PersistenceFuture<'a, Result<(), BattlePetPersistenceErrorLikeCpp>>;

    fn delete_pet<'a>(
        &'a self,
        account_id: u32,
        fence: u64,
        pet_guid_counter: u64,
        slots: Vec<DurableBattlePetSlotLikeCpp>,
    ) -> PersistenceFuture<'a, Result<(), BattlePetPersistenceErrorLikeCpp>>;

    fn replace_slots<'a>(
        &'a self,
        account_id: u32,
        fence: u64,
        slots: Vec<DurableBattlePetSlotLikeCpp>,
    ) -> PersistenceFuture<'a, Result<(), BattlePetPersistenceErrorLikeCpp>>;
}

#[derive(Debug)]
pub struct LoginBattlePetPersistenceLikeCpp {
    db: Arc<LoginDatabase>,
    lock_broker: BattlePetAccountLockBrokerLikeCpp,
}

#[derive(Debug, Clone)]
struct BattlePetAccountLockBrokerLikeCpp {
    commands: mpsc::UnboundedSender<BattlePetAccountLockCommandLikeCpp>,
    epoch: Arc<AtomicU64>,
}

enum BattlePetAccountLockCommandLikeCpp {
    Acquire {
        account_id: u32,
        result: oneshot::Sender<Result<Option<(String, u64)>, String>>,
    },
    Release {
        lock_name: String,
        epoch: u64,
    },
}

struct LoginBattlePetProcessLeaseLikeCpp {
    lock_broker: BattlePetAccountLockBrokerLikeCpp,
    lock_name: Option<String>,
    broker_epoch: u64,
    fence: u64,
}

struct BattlePetBrokerLeaseReservationLikeCpp {
    lock_broker: BattlePetAccountLockBrokerLikeCpp,
    lock_name: Option<String>,
    broker_epoch: u64,
}

impl Drop for BattlePetBrokerLeaseReservationLikeCpp {
    fn drop(&mut self) {
        if let Some(lock_name) = self.lock_name.take() {
            self.lock_broker
                .release_like_cpp(lock_name, self.broker_epoch);
        }
    }
}

impl BattlePetProcessLeaseLikeCpp for LoginBattlePetProcessLeaseLikeCpp {
    fn is_valid_like_cpp(&self) -> bool {
        !self.lock_broker.commands.is_closed()
            && self.lock_broker.epoch.load(Ordering::Acquire) == self.broker_epoch
    }

    fn fence_like_cpp(&self) -> u64 {
        self.fence
    }
}

impl Drop for LoginBattlePetProcessLeaseLikeCpp {
    fn drop(&mut self) {
        if let Some(lock_name) = self.lock_name.take() {
            let _ = self
                .lock_broker
                .commands
                .send(BattlePetAccountLockCommandLikeCpp::Release {
                    lock_name,
                    epoch: self.broker_epoch,
                });
        }
    }
}

impl BattlePetAccountLockBrokerLikeCpp {
    fn spawn_like_cpp(db: Arc<LoginDatabase>) -> Self {
        let (commands, receiver) = mpsc::unbounded_channel();
        let epoch = Arc::new(AtomicU64::new(1));
        tokio::spawn(run_battle_pet_account_lock_broker_like_cpp(
            db,
            receiver,
            Arc::clone(&epoch),
        ));
        Self { commands, epoch }
    }

    async fn acquire_like_cpp(
        &self,
        account_id: u32,
    ) -> Result<Option<(String, u64)>, BattlePetPersistenceErrorLikeCpp> {
        let (result, response) = oneshot::channel();
        self.commands
            .send(BattlePetAccountLockCommandLikeCpp::Acquire { account_id, result })
            .map_err(|_| {
                BattlePetPersistenceErrorLikeCpp::Database(
                    "battle-pet account lock broker stopped".to_string(),
                )
            })?;
        response
            .await
            .map_err(|_| {
                BattlePetPersistenceErrorLikeCpp::Database(
                    "battle-pet account lock broker dropped acquisition".to_string(),
                )
            })?
            .map_err(BattlePetPersistenceErrorLikeCpp::Database)
    }

    fn release_like_cpp(&self, lock_name: String, epoch: u64) {
        let _ = self
            .commands
            .send(BattlePetAccountLockCommandLikeCpp::Release { lock_name, epoch });
    }
}

async fn open_battle_pet_lock_broker_connection_like_cpp(
    db: &LoginDatabase,
) -> Result<(sqlx::MySqlConnection, String), String> {
    let pooled = db
        .pool()
        .acquire()
        .await
        .map_err(|error| error.to_string())?;
    let mut connection = pooled.detach();
    let database_scope: String =
        sqlx::query_scalar("SELECT LEFT(SHA2(COALESCE(DATABASE(), ''), 256), 32)")
            .fetch_one(&mut connection)
            .await
            .map_err(|error| error.to_string())?;
    Ok((connection, database_scope))
}

fn invalidate_battle_pet_lock_broker_like_cpp(
    connection: &mut Option<(sqlx::MySqlConnection, String)>,
    held_locks: &mut HashSet<String>,
    epoch: &AtomicU64,
) {
    *connection = None;
    held_locks.clear();
    epoch.fetch_add(1, Ordering::AcqRel);
}

fn battle_pet_account_lock_name_like_cpp(database_scope: &str, account_id: u32) -> String {
    format!("rustycore:bp:{database_scope}:{account_id}")
}

async fn run_battle_pet_account_lock_broker_like_cpp(
    db: Arc<LoginDatabase>,
    mut commands: mpsc::UnboundedReceiver<BattlePetAccountLockCommandLikeCpp>,
    epoch: Arc<AtomicU64>,
) {
    let mut connection: Option<(sqlx::MySqlConnection, String)> = None;
    let mut held_locks = HashSet::new();
    let mut verify_interval =
        tokio::time::interval(BATTLE_PET_PROCESS_LEASE_VERIFY_INTERVAL_LIKE_CPP);
    verify_interval.set_missed_tick_behavior(MissedTickBehavior::Skip);
    loop {
        tokio::select! {
            biased;
            command = commands.recv() => {
                let Some(command) = command else { return; };
                match command {
                    BattlePetAccountLockCommandLikeCpp::Acquire { account_id, result } => {
                        if connection.is_none() {
                            match open_battle_pet_lock_broker_connection_like_cpp(&db).await {
                                Ok(opened) => connection = Some(opened),
                                Err(error) => {
                                    let _ = result.send(Err(error));
                                    continue;
                                }
                            }
                        }
                        let (_, database_scope) = connection.as_ref().expect("broker connection opened");
                        let lock_name = battle_pet_account_lock_name_like_cpp(database_scope, account_id);
                        if held_locks.contains(&lock_name) {
                            let _ = result.send(Ok(None));
                            continue;
                        }
                        let acquired = {
                            let (connection, _) = connection.as_mut().expect("broker connection opened");
                            sqlx::query_scalar::<_, Option<i64>>("SELECT GET_LOCK(?, 0)")
                                .bind(&lock_name)
                                .fetch_one(connection)
                                .await
                        };
                        match acquired {
                            Ok(Some(1)) => {
                                held_locks.insert(lock_name.clone());
                                let lease_epoch = epoch.load(Ordering::Acquire);
                                if result.send(Ok(Some((lock_name.clone(), lease_epoch)))).is_err() {
                                    held_locks.remove(&lock_name);
                                    let release = {
                                        let (connection, _) = connection.as_mut().expect("broker connection opened");
                                        sqlx::query_scalar::<_, Option<i64>>("SELECT RELEASE_LOCK(?)")
                                            .bind(&lock_name)
                                            .fetch_one(connection)
                                            .await
                                    };
                                    if release.is_err() {
                                        invalidate_battle_pet_lock_broker_like_cpp(
                                            &mut connection,
                                            &mut held_locks,
                                            &epoch,
                                        );
                                    }
                                }
                            }
                            Ok(_) => { let _ = result.send(Ok(None)); }
                            Err(error) => {
                                invalidate_battle_pet_lock_broker_like_cpp(
                                    &mut connection,
                                    &mut held_locks,
                                    &epoch,
                                );
                                let _ = result.send(Err(error.to_string()));
                            }
                        }
                    }
                    BattlePetAccountLockCommandLikeCpp::Release { lock_name, epoch: lease_epoch } => {
                        if lease_epoch != epoch.load(Ordering::Acquire) || !held_locks.remove(&lock_name) {
                            continue;
                        }
                        let release = if let Some((connection, _)) = connection.as_mut() {
                            sqlx::query_scalar::<_, Option<i64>>("SELECT RELEASE_LOCK(?)")
                                .bind(&lock_name)
                                .fetch_one(connection)
                                .await
                        } else {
                            continue;
                        };
                        if release.is_err() {
                            invalidate_battle_pet_lock_broker_like_cpp(
                                &mut connection,
                                &mut held_locks,
                                &epoch,
                            );
                        }
                    }
                }
            }
            _ = verify_interval.tick(), if connection.is_some() => {
                let ping = {
                    let (connection, _) = connection.as_mut().expect("broker connection exists");
                    sqlx::query_scalar::<_, i32>("SELECT 1").fetch_one(connection).await
                };
                if ping.is_err() {
                    invalidate_battle_pet_lock_broker_like_cpp(
                        &mut connection,
                        &mut held_locks,
                        &epoch,
                    );
                }
            }
        }
    }
}

impl LoginBattlePetPersistenceLikeCpp {
    pub fn new(db: Arc<LoginDatabase>) -> Self {
        let lock_broker = BattlePetAccountLockBrokerLikeCpp::spawn_like_cpp(Arc::clone(&db));
        Self { db, lock_broker }
    }

    async fn find_request_like_cpp(
        &self,
        request_key: BattlePetAddRequestKeyLikeCpp,
    ) -> Result<Option<(u32, DurableBattlePetRowLikeCpp, bool)>, BattlePetPersistenceErrorLikeCpp>
    {
        let mut stmt = self.db.prepare(LoginStatements::SEL_BATTLE_PET_ADD_REQUEST);
        stmt.set_bytes(0, request_key.as_bytes().to_vec());
        let result = self
            .db
            .query(&stmt)
            .await
            .map_err(database_error_like_cpp)?;
        if result.is_empty() {
            return Ok(None);
        }
        let receipt_account_id: u32 = result.try_read(0).ok_or_else(|| {
            BattlePetPersistenceErrorLikeCpp::Database(
                "could not decode battle-pet request account".to_string(),
            )
        })?;
        let still_present: bool = result.try_read(13).ok_or_else(|| {
            BattlePetPersistenceErrorLikeCpp::Database(
                "could not decode battle-pet request live-row marker".to_string(),
            )
        })?;
        Ok(Some((
            receipt_account_id,
            durable_pet_from_result_like_cpp(&result, 1)?,
            still_present,
        )))
    }

    async fn find_live_pet_like_cpp(
        &self,
        account_id: u32,
        pet_guid_counter: u64,
    ) -> Result<Option<DurableBattlePetRowLikeCpp>, BattlePetPersistenceErrorLikeCpp> {
        let row = sqlx::query(
            "SELECT bp.guid, bp.species, bp.breed, bp.displayId, bp.level, bp.exp, bp.health, bp.quality, bp.flags, bp.name, bp.nameTimestamp, bp.owner, dn.genitive, dn.dative, dn.accusative, dn.instrumental, dn.prepositional FROM battle_pets bp LEFT JOIN battle_pet_declinedname dn ON bp.guid = dn.guid WHERE bp.battlenetAccountId = ? AND bp.guid = ?",
        )
        .bind(account_id)
        .bind(pet_guid_counter)
        .fetch_optional(self.db.pool())
        .await
        .map_err(|error| database_error_like_cpp(DatabaseError::from(error)))?;
        row.as_ref().map(durable_pet_from_row_like_cpp).transpose()
    }

    async fn slots_match_like_cpp(
        &self,
        account_id: u32,
        expected: &[DurableBattlePetSlotLikeCpp],
    ) -> Result<bool, BattlePetPersistenceErrorLikeCpp> {
        let mut actual = {
            let mut tx = self
                .db
                .pool()
                .begin()
                .await
                .map_err(|error| database_error_like_cpp(DatabaseError::from(error)))?;
            let actual = load_slot_rows_like_cpp(&mut tx, account_id).await?;
            tx.commit()
                .await
                .map_err(|error| database_error_like_cpp(DatabaseError::from(error)))?;
            actual
        };
        let mut expected = expected.to_vec();
        actual.sort_by_key(|slot| slot.index);
        expected.sort_by_key(|slot| slot.index);
        Ok(actual == expected)
    }
}

impl BattlePetPersistenceLikeCpp for LoginBattlePetPersistenceLikeCpp {
    fn try_acquire_process_lease<'a>(
        &'a self,
        account_id: u32,
    ) -> PersistenceFuture<
        'a,
        Result<Option<Box<dyn BattlePetProcessLeaseLikeCpp>>, BattlePetPersistenceErrorLikeCpp>,
    > {
        Box::pin(async move {
            // MariaDB named locks belong to a connection and are server-wide.
            // One process-level broker multiplexes every online account on a
            // single detached connection, and includes the Login DB identity
            // in each lock name so independent deployments do not contend.
            let Some((lock_name, broker_epoch)) =
                self.lock_broker.acquire_like_cpp(account_id).await?
            else {
                return Ok(None);
            };
            // Keep acquisition cancellation-safe while the durable fence is
            // advanced. If this future is dropped, the reservation releases
            // the named lock through the broker.
            let mut broker_lease = BattlePetBrokerLeaseReservationLikeCpp {
                lock_broker: self.lock_broker.clone(),
                lock_name: Some(lock_name),
                broker_epoch,
            };
            // The advisory lock elects one process, while this durable epoch
            // fences transactions that were already queued when a dead
            // connection released that lock. Taking the row lock means a new
            // owner cannot reload until every transaction from the preceding
            // epoch has either committed or observed that it is stale.
            let fence = async {
                let mut tx = self
                    .db
                    .pool()
                    .begin()
                    .await
                    .map_err(|error| database_error_like_cpp(DatabaseError::from(error)))?;
                sqlx::query(
                    "INSERT INTO battle_pet_account_fences (battlenetAccountId, generation) VALUES (?, 0) ON DUPLICATE KEY UPDATE battlenetAccountId = VALUES(battlenetAccountId)",
                )
                .bind(account_id)
                .execute(&mut *tx)
                .await
                .map_err(|error| database_error_like_cpp(DatabaseError::from(error)))?;
                let current: u64 = sqlx::query_scalar(
                    "SELECT generation FROM battle_pet_account_fences WHERE battlenetAccountId = ? FOR UPDATE",
                )
                .bind(account_id)
                .fetch_one(&mut *tx)
                .await
                .map_err(|error| database_error_like_cpp(DatabaseError::from(error)))?;
                let next = current.checked_add(1).ok_or_else(|| {
                    BattlePetPersistenceErrorLikeCpp::Database(
                        "battle-pet account fence exhausted".to_string(),
                    )
                })?;
                sqlx::query(
                    "UPDATE battle_pet_account_fences SET generation = ? WHERE battlenetAccountId = ?",
                )
                .bind(next)
                .bind(account_id)
                .execute(&mut *tx)
                .await
                .map_err(|error| database_error_like_cpp(DatabaseError::from(error)))?;
                tx.commit()
                    .await
                    .map_err(|error| database_error_like_cpp(DatabaseError::from(error)))?;
                Ok::<_, BattlePetPersistenceErrorLikeCpp>(next)
            }
            .await;
            let fence = match fence {
                Ok(fence) => fence,
                Err(error) => return Err(error),
            };
            let lock_name = broker_lease
                .lock_name
                .take()
                .expect("broker lease reservation lost its lock name");
            Ok(Some(Box::new(LoginBattlePetProcessLeaseLikeCpp {
                lock_broker: self.lock_broker.clone(),
                lock_name: Some(lock_name),
                broker_epoch,
                fence,
            })
                as Box<dyn BattlePetProcessLeaseLikeCpp>))
        })
    }

    fn load_account<'a>(
        &'a self,
        account_id: u32,
        realm_id: u16,
    ) -> PersistenceFuture<
        'a,
        Result<LoadedBattlePetAccountLikeCpp, BattlePetPersistenceErrorLikeCpp>,
    > {
        Box::pin(async move {
            let mut tx = self
                .db
                .pool()
                .begin()
                .await
                .map_err(|error| database_error_like_cpp(DatabaseError::from(error)))?;
            let pets = load_pet_rows_like_cpp(&mut tx, account_id, realm_id).await?;
            let slots = load_slot_rows_like_cpp(&mut tx, account_id).await?;
            tx.commit()
                .await
                .map_err(|error| database_error_like_cpp(DatabaseError::from(error)))?;
            Ok(LoadedBattlePetAccountLikeCpp { pets, slots })
        })
    }

    fn allocate_guid_counter_like_cpp(
        &self,
    ) -> PersistenceFuture<'_, Result<u64, BattlePetPersistenceErrorLikeCpp>> {
        Box::pin(async move {
            // The namespace is shared by every realm using this Login DB. A
            // short row lock serializes one allocation without preventing a
            // second world-server from running against the same database.
            let mut tx = self
                .db
                .pool()
                .begin()
                .await
                .map_err(|error| database_error_like_cpp(DatabaseError::from(error)))?;
            let row = sqlx::query(
                "SELECT nextGuid FROM battle_pet_guid_sequence WHERE singleton = 1 FOR UPDATE",
            )
            .fetch_optional(&mut *tx)
            .await
            .map_err(|error| database_error_like_cpp(DatabaseError::from(error)))?
            .ok_or_else(|| {
                BattlePetPersistenceErrorLikeCpp::Database(
                    "battle-pet GUID sequence row is missing".to_string(),
                )
            })?;
            let counter: u64 = row.try_get("nextGuid").map_err(|error| {
                BattlePetPersistenceErrorLikeCpp::Database(format!(
                    "could not decode battle-pet GUID sequence: {error}"
                ))
            })?;
            let next = counter
                .checked_add(1)
                .ok_or(BattlePetPersistenceErrorLikeCpp::GuidCollision)?;
            let generator_limit = u64::try_from(ObjectGuid::max_counter(HighGuid::BattlePet) - 1)
                .map_err(|_| BattlePetPersistenceErrorLikeCpp::GuidCollision)?;
            if counter == 0 || counter >= generator_limit {
                return Err(BattlePetPersistenceErrorLikeCpp::GuidCollision);
            }
            sqlx::query("UPDATE battle_pet_guid_sequence SET nextGuid = ? WHERE singleton = 1")
                .bind(next)
                .execute(&mut *tx)
                .await
                .map_err(|error| database_error_like_cpp(DatabaseError::from(error)))?;
            tx.commit()
                .await
                .map_err(|error| database_error_like_cpp(DatabaseError::from(error)))?;
            Ok(counter)
        })
    }

    fn insert_pet_idempotently<'a>(
        &'a self,
        request: DurableBattlePetAddLikeCpp,
    ) -> PersistenceFuture<
        'a,
        Result<PersistBattlePetAddOutcomeLikeCpp, BattlePetPersistenceErrorLikeCpp>,
    > {
        Box::pin(async move {
            if let Some((receipt_account_id, existing, still_present)) =
                self.find_request_like_cpp(request.request_key).await?
            {
                return if receipt_account_id == request.account_id
                    && add_request_matches_like_cpp(&request.pet, &existing)
                {
                    Ok(PersistBattlePetAddOutcomeLikeCpp::Replayed {
                        pet: existing,
                        still_present,
                    })
                } else {
                    Err(BattlePetPersistenceErrorLikeCpp::DuplicateRequest)
                };
            }

            let owner_scope = request.pet.owner_guid_counter.unwrap_or(0);
            let owner_realm_scope = request
                .pet
                .owner_guid_counter
                .map(|_| request.realm_id)
                .unwrap_or(0);
            let mut tx = self
                .db
                .pool()
                .begin()
                .await
                .map_err(|error| database_error_like_cpp(DatabaseError::from(error)))?;

            lock_and_validate_account_fence_like_cpp(&mut tx, request.account_id, request.fence)
                .await?;

            // Every process contending for this exact C++ count scope locks
            // the same durable row. Capacity is re-read while that lock is
            // held and the pet/receipt inserts commit before it is released.
            sqlx::query(
                "INSERT INTO battle_pet_capacity_locks (battlenetAccountId, species, ownerRealmScope, ownerScope) VALUES (?, ?, ?, ?) ON DUPLICATE KEY UPDATE ownerScope = VALUES(ownerScope)",
            )
            .bind(request.account_id)
            .bind(request.pet.species)
            .bind(owner_realm_scope)
            .bind(owner_scope)
            .execute(&mut *tx)
            .await
            .map_err(|error| database_error_like_cpp(DatabaseError::from(error)))?;
            sqlx::query(
                "SELECT ownerScope FROM battle_pet_capacity_locks WHERE battlenetAccountId = ? AND species = ? AND ownerRealmScope = ? AND ownerScope = ? FOR UPDATE",
            )
            .bind(request.account_id)
            .bind(request.pet.species)
            .bind(owner_realm_scope)
            .bind(owner_scope)
            .fetch_one(&mut *tx)
            .await
            .map_err(|error| database_error_like_cpp(DatabaseError::from(error)))?;

            if let Some((receipt_account_id, existing, still_present)) =
                find_request_in_tx_like_cpp(&mut tx, request.request_key).await?
            {
                tx.rollback()
                    .await
                    .map_err(|error| database_error_like_cpp(DatabaseError::from(error)))?;
                return if receipt_account_id == request.account_id
                    && add_request_matches_like_cpp(&request.pet, &existing)
                {
                    Ok(PersistBattlePetAddOutcomeLikeCpp::Replayed {
                        pet: existing,
                        still_present,
                    })
                } else {
                    Err(BattlePetPersistenceErrorLikeCpp::DuplicateRequest)
                };
            }

            let count: i64 = if let Some(owner) = request.pet.owner_guid_counter {
                sqlx::query_scalar(
                    "SELECT COUNT(*) FROM battle_pets WHERE battlenetAccountId = ? AND species = ? AND owner = ? AND ownerRealmId = ? FOR UPDATE",
                )
                .bind(request.account_id)
                .bind(request.pet.species)
                .bind(owner)
                .bind(request.realm_id)
                .fetch_one(&mut *tx)
                .await
            } else {
                sqlx::query_scalar(
                    "SELECT COUNT(*) FROM battle_pets WHERE battlenetAccountId = ? AND species = ? AND owner IS NULL AND ownerRealmId IS NULL FOR UPDATE",
                )
                .bind(request.account_id)
                .bind(request.pet.species)
                .fetch_one(&mut *tx)
                .await
            }
            .map_err(|error| database_error_like_cpp(DatabaseError::from(error)))?;
            if count >= i64::from(request.max_per_scope) {
                tx.rollback()
                    .await
                    .map_err(|error| database_error_like_cpp(DatabaseError::from(error)))?;
                return Err(BattlePetPersistenceErrorLikeCpp::Capacity);
            }

            let pet = &request.pet;
            let insert_result = async {
                sqlx::query(
                    "INSERT INTO battle_pets (guid, battlenetAccountId, species, breed, displayId, level, exp, health, quality, flags, name, nameTimestamp, owner, ownerRealmId) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                )
                .bind(pet.guid_counter)
                .bind(request.account_id)
                .bind(pet.species)
                .bind(pet.breed)
                .bind(pet.display_id)
                .bind(pet.level)
                .bind(pet.exp)
                .bind(pet.health)
                .bind(pet.quality)
                .bind(pet.flags)
                .bind(&pet.name)
                .bind(pet.name_timestamp)
                .bind(pet.owner_guid_counter)
                .bind(pet.owner_guid_counter.map(|_| request.realm_id))
                .execute(&mut *tx)
                .await?;
                sqlx::query(
                    "INSERT INTO battle_pet_add_requests (battlenetAccountId, requestKey, battlePetGuid, species, breed, displayId, level, exp, health, quality, flags, name, nameTimestamp, owner) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                )
                .bind(request.account_id)
                .bind(request.request_key.as_bytes().as_slice())
                .bind(pet.guid_counter)
                .bind(pet.species)
                .bind(pet.breed)
                .bind(pet.display_id)
                .bind(pet.level)
                .bind(pet.exp)
                .bind(pet.health)
                .bind(pet.quality)
                .bind(pet.flags)
                .bind(&pet.name)
                .bind(pet.name_timestamp)
                .bind(pet.owner_guid_counter)
                .execute(&mut *tx)
                .await?;
                Ok::<_, sqlx::Error>(())
            }
            .await;

            if let Err(error) = insert_result {
                let error = DatabaseError::from(error);
                drop(tx);
                if is_duplicate_key_like_cpp(&error) {
                    if let Some((receipt_account_id, existing, still_present)) =
                        self.find_request_like_cpp(request.request_key).await?
                    {
                        return if receipt_account_id == request.account_id
                            && add_request_matches_like_cpp(&request.pet, &existing)
                        {
                            Ok(PersistBattlePetAddOutcomeLikeCpp::Replayed {
                                pet: existing,
                                still_present,
                            })
                        } else {
                            Err(BattlePetPersistenceErrorLikeCpp::DuplicateRequest)
                        };
                    }
                    return Err(BattlePetPersistenceErrorLikeCpp::GuidCollision);
                }
                return Err(database_error_like_cpp(error));
            }

            match tx.commit().await {
                Ok(()) => Ok(PersistBattlePetAddOutcomeLikeCpp::Inserted),
                Err(_) => match self.find_request_like_cpp(request.request_key).await? {
                    Some((receipt_account_id, existing, true))
                        if receipt_account_id == request.account_id
                            && add_request_matches_like_cpp(&request.pet, &existing) =>
                    {
                        Ok(PersistBattlePetAddOutcomeLikeCpp::Inserted)
                    }
                    Some(_) => Err(BattlePetPersistenceErrorLikeCpp::DuplicateRequest),
                    None => Err(BattlePetPersistenceErrorLikeCpp::Database(
                        "battle-pet insert COMMIT outcome could not be reconciled".to_string(),
                    )),
                },
            }
        })
    }

    fn lookup_add_request<'a>(
        &'a self,
        account_id: u32,
        request_key: BattlePetAddRequestKeyLikeCpp,
    ) -> PersistenceFuture<
        'a,
        Result<Option<DurableBattlePetAddReceiptLikeCpp>, BattlePetPersistenceErrorLikeCpp>,
    > {
        Box::pin(async move {
            let Some((receipt_account_id, requested_pet, still_present)) =
                self.find_request_like_cpp(request_key).await?
            else {
                return Ok(None);
            };
            if receipt_account_id != account_id {
                return Err(BattlePetPersistenceErrorLikeCpp::DuplicateRequest);
            }
            let current_pet = if still_present {
                self.find_live_pet_like_cpp(account_id, requested_pet.guid_counter)
                    .await?
            } else {
                None
            };
            Ok(Some(DurableBattlePetAddReceiptLikeCpp {
                account_id: receipt_account_id,
                requested_pet,
                current_pet,
            }))
        })
    }

    fn update_pet<'a>(
        &'a self,
        account_id: u32,
        fence: u64,
        pet: DurableBattlePetRowLikeCpp,
    ) -> PersistenceFuture<'a, Result<(), BattlePetPersistenceErrorLikeCpp>> {
        Box::pin(async move {
            let mut tx = self
                .db
                .pool()
                .begin()
                .await
                .map_err(|error| database_error_like_cpp(DatabaseError::from(error)))?;
            lock_and_validate_account_fence_like_cpp(&mut tx, account_id, fence).await?;

            // MySQL reports changed rows, not matched rows, for UPDATE. Lock
            // and verify the target explicitly so an idempotent main-row
            // update still succeeds and can persist changed declined forms.
            let exists: Option<i32> = sqlx::query_scalar(
                "SELECT 1 FROM battle_pets WHERE battlenetAccountId = ? AND guid = ? FOR UPDATE",
            )
            .bind(account_id)
            .bind(pet.guid_counter)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|error| database_error_like_cpp(DatabaseError::from(error)))?;
            if exists.is_none() {
                return Err(BattlePetPersistenceErrorLikeCpp::Database(
                    "battle-pet update target does not exist".to_string(),
                ));
            }

            sqlx::query(
                "UPDATE battle_pets SET level = ?, exp = ?, health = ?, quality = ?, flags = ?, name = ?, nameTimestamp = ? WHERE battlenetAccountId = ? AND guid = ?",
            )
            .bind(pet.level)
            .bind(pet.exp)
            .bind(pet.health)
            .bind(pet.quality)
            .bind(pet.flags)
            .bind(&pet.name)
            .bind(pet.name_timestamp)
            .bind(account_id)
            .bind(pet.guid_counter)
            .execute(&mut *tx)
            .await
            .map_err(|error| database_error_like_cpp(DatabaseError::from(error)))?;
            sqlx::query("DELETE FROM battle_pet_declinedname WHERE guid = ?")
                .bind(pet.guid_counter)
                .execute(&mut *tx)
                .await
                .map_err(|error| database_error_like_cpp(DatabaseError::from(error)))?;
            if let Some(declined) = &pet.declined_names {
                sqlx::query(
                    "INSERT INTO battle_pet_declinedname (guid, genitive, dative, accusative, instrumental, prepositional) VALUES (?, ?, ?, ?, ?, ?)",
                )
                .bind(pet.guid_counter)
                .bind(&declined.names[0])
                .bind(&declined.names[1])
                .bind(&declined.names[2])
                .bind(&declined.names[3])
                .bind(&declined.names[4])
                .execute(&mut *tx)
                .await
                .map_err(|error| database_error_like_cpp(DatabaseError::from(error)))?;
            }
            match tx.commit().await {
                Ok(()) => Ok(()),
                Err(_) => {
                    if self
                        .find_live_pet_like_cpp(account_id, pet.guid_counter)
                        .await?
                        .as_ref()
                        == Some(&pet)
                    {
                        Ok(())
                    } else {
                        Err(BattlePetPersistenceErrorLikeCpp::Database(
                            "battle-pet update COMMIT outcome could not be reconciled".to_string(),
                        ))
                    }
                }
            }
        })
    }

    fn delete_pet<'a>(
        &'a self,
        account_id: u32,
        fence: u64,
        pet_guid_counter: u64,
        slots: Vec<DurableBattlePetSlotLikeCpp>,
    ) -> PersistenceFuture<'a, Result<(), BattlePetPersistenceErrorLikeCpp>> {
        Box::pin(async move {
            let mut tx = SqlTransaction::new();
            let mut authority = self
                .db
                .prepare(LoginStatements::LOCK_BATTLE_PET_ACCOUNT_FENCE);
            authority.set_u32(0, account_id);
            authority.set_u64(1, fence);
            tx.append_expect_rows_affected(authority, 1);
            let mut delete_declined = self
                .db
                .prepare(LoginStatements::DEL_BATTLE_PET_DECLINED_NAME);
            delete_declined.set_u64(0, pet_guid_counter);
            tx.append(delete_declined);
            let mut delete_pet = self.db.prepare(LoginStatements::DEL_BATTLE_PETS);
            delete_pet.set_u32(0, account_id);
            delete_pet.set_u64(1, pet_guid_counter);
            tx.append_expect_rows_affected(delete_pet, 1);
            let mut delete_slots = self.db.prepare(LoginStatements::DEL_BATTLE_PET_SLOTS);
            delete_slots.set_u32(0, account_id);
            tx.append(delete_slots);
            for slot in &slots {
                let mut insert = self.db.prepare(LoginStatements::INS_BATTLE_PET_SLOTS);
                insert.set_u8(0, slot.index);
                insert.set_u32(1, account_id);
                insert.set_u64(2, slot.pet_guid_counter.unwrap_or(0));
                insert.set_bool(3, slot.locked);
                tx.append(insert);
            }
            match tx.commit_with_outcome_like_cpp(self.db.pool()).await {
                Ok(()) => Ok(()),
                Err(SqlTransactionCommitError::CommitOutcomeUnknown(_)) => {
                    let deleted = self
                        .find_live_pet_like_cpp(account_id, pet_guid_counter)
                        .await?
                        .is_none();
                    if deleted && self.slots_match_like_cpp(account_id, &slots).await? {
                        Ok(())
                    } else {
                        Err(BattlePetPersistenceErrorLikeCpp::Database(
                            "battle-pet delete COMMIT outcome could not be reconciled".to_string(),
                        ))
                    }
                }
                Err(SqlTransactionCommitError::DefinitelyRolledBack(error)) => {
                    Err(database_error_like_cpp(error))
                }
            }
        })
    }

    fn replace_slots<'a>(
        &'a self,
        account_id: u32,
        fence: u64,
        slots: Vec<DurableBattlePetSlotLikeCpp>,
    ) -> PersistenceFuture<'a, Result<(), BattlePetPersistenceErrorLikeCpp>> {
        Box::pin(async move {
            let mut tx = SqlTransaction::new();
            let mut authority = self
                .db
                .prepare(LoginStatements::LOCK_BATTLE_PET_ACCOUNT_FENCE);
            authority.set_u32(0, account_id);
            authority.set_u64(1, fence);
            tx.append_expect_rows_affected(authority, 1);
            let mut delete = self.db.prepare(LoginStatements::DEL_BATTLE_PET_SLOTS);
            delete.set_u32(0, account_id);
            tx.append(delete);
            for slot in &slots {
                let mut insert = self.db.prepare(LoginStatements::INS_BATTLE_PET_SLOTS);
                insert.set_u8(0, slot.index);
                insert.set_u32(1, account_id);
                insert.set_u64(2, slot.pet_guid_counter.unwrap_or(0));
                insert.set_bool(3, slot.locked);
                tx.append(insert);
            }
            match tx.commit_with_outcome_like_cpp(self.db.pool()).await {
                Ok(()) => Ok(()),
                Err(SqlTransactionCommitError::CommitOutcomeUnknown(_)) => {
                    if self.slots_match_like_cpp(account_id, &slots).await? {
                        Ok(())
                    } else {
                        Err(BattlePetPersistenceErrorLikeCpp::Database(
                            "battle-pet slot COMMIT outcome could not be reconciled".to_string(),
                        ))
                    }
                }
                Err(SqlTransactionCommitError::DefinitelyRolledBack(error)) => {
                    Err(database_error_like_cpp(error))
                }
            }
        })
    }
}

async fn load_pet_rows_like_cpp(
    tx: &mut Transaction<'_, MySql>,
    account_id: u32,
    realm_id: u16,
) -> Result<Vec<DurableBattlePetRowLikeCpp>, BattlePetPersistenceErrorLikeCpp> {
    let rows = sqlx::query(
        "SELECT bp.guid, bp.species, bp.breed, bp.displayId, bp.level, bp.exp, bp.health, bp.quality, bp.flags, bp.name, bp.nameTimestamp, bp.owner, dn.genitive, dn.dative, dn.accusative, dn.instrumental, dn.prepositional FROM battle_pets bp LEFT JOIN battle_pet_declinedname dn ON bp.guid = dn.guid WHERE bp.battlenetAccountId = ? AND (bp.ownerRealmId IS NULL OR bp.ownerRealmId = ?)",
    )
    .bind(account_id)
    .bind(realm_id)
    .fetch_all(&mut **tx)
    .await
    .map_err(|error| database_error_like_cpp(DatabaseError::from(error)))?;
    rows.into_iter()
        .map(|row| durable_pet_from_row_like_cpp(&row))
        .collect()
}

async fn lock_and_validate_account_fence_like_cpp(
    tx: &mut Transaction<'_, MySql>,
    account_id: u32,
    expected: u64,
) -> Result<(), BattlePetPersistenceErrorLikeCpp> {
    let actual: Option<u64> = sqlx::query_scalar(
        "SELECT generation FROM battle_pet_account_fences WHERE battlenetAccountId = ? FOR UPDATE",
    )
    .bind(account_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(|error| database_error_like_cpp(DatabaseError::from(error)))?;
    if actual == Some(expected) {
        Ok(())
    } else {
        Err(BattlePetPersistenceErrorLikeCpp::StaleAuthority)
    }
}

async fn load_slot_rows_like_cpp(
    tx: &mut Transaction<'_, MySql>,
    account_id: u32,
) -> Result<Vec<DurableBattlePetSlotLikeCpp>, BattlePetPersistenceErrorLikeCpp> {
    let rows = sqlx::query(
        "SELECT id, battlePetGuid, locked FROM battle_pet_slots WHERE battlenetAccountId = ?",
    )
    .bind(account_id)
    .fetch_all(&mut **tx)
    .await
    .map_err(|error| database_error_like_cpp(DatabaseError::from(error)))?;
    rows.into_iter()
        .map(|row| {
            let counter = row_u64_signed_or_unsigned_like_cpp(&row, 1)?;
            Ok(DurableBattlePetSlotLikeCpp {
                index: row_u8_signed_or_unsigned_like_cpp(&row, 0)?,
                pet_guid_counter: (counter != 0).then_some(counter),
                locked: row.try_get(2).map_err(|error| {
                    BattlePetPersistenceErrorLikeCpp::Database(error.to_string())
                })?,
            })
        })
        .collect()
}

fn durable_pet_from_result_like_cpp(
    result: &wow_database::SqlResult,
    offset: usize,
) -> Result<DurableBattlePetRowLikeCpp, BattlePetPersistenceErrorLikeCpp> {
    let missing = |column: usize| {
        BattlePetPersistenceErrorLikeCpp::Database(format!(
            "could not decode battle-pet result column {column}"
        ))
    };
    Ok(DurableBattlePetRowLikeCpp {
        guid_counter: result_u64_signed_or_unsigned_like_cpp(result, offset)
            .ok_or_else(|| missing(offset))?,
        species: result_u32_signed_or_unsigned_like_cpp(result, offset + 1)
            .ok_or_else(|| missing(offset + 1))?,
        breed: result_u16_signed_or_unsigned_like_cpp(result, offset + 2)
            .ok_or_else(|| missing(offset + 2))?,
        display_id: result_u32_signed_or_unsigned_like_cpp(result, offset + 3)
            .ok_or_else(|| missing(offset + 3))?,
        level: result_u16_signed_or_unsigned_like_cpp(result, offset + 4)
            .ok_or_else(|| missing(offset + 4))?,
        exp: result_u16_signed_or_unsigned_like_cpp(result, offset + 5)
            .ok_or_else(|| missing(offset + 5))?,
        health: result_u32_signed_or_unsigned_like_cpp(result, offset + 6)
            .ok_or_else(|| missing(offset + 6))?,
        quality: result_u8_signed_or_unsigned_like_cpp(result, offset + 7)
            .ok_or_else(|| missing(offset + 7))?,
        flags: result_u16_signed_or_unsigned_like_cpp(result, offset + 8)
            .ok_or_else(|| missing(offset + 8))?,
        name: result
            .try_read(offset + 9)
            .ok_or_else(|| missing(offset + 9))?,
        name_timestamp: result
            .try_read(offset + 10)
            .ok_or_else(|| missing(offset + 10))?,
        owner_guid_counter: if result.is_null(offset + 11) {
            None
        } else {
            Some(
                result_u64_signed_or_unsigned_like_cpp(result, offset + 11)
                    .ok_or_else(|| missing(offset + 11))?,
            )
        },
        declined_names: None,
    })
}

async fn find_request_in_tx_like_cpp(
    tx: &mut Transaction<'_, MySql>,
    request_key: BattlePetAddRequestKeyLikeCpp,
) -> Result<Option<(u32, DurableBattlePetRowLikeCpp, bool)>, BattlePetPersistenceErrorLikeCpp> {
    let row = sqlx::query(
        "SELECT req.battlenetAccountId, req.battlePetGuid, req.species, req.breed, req.displayId, req.level, req.exp, req.health, req.quality, req.flags, req.name, req.nameTimestamp, req.owner, pet.guid IS NOT NULL FROM battle_pet_add_requests req LEFT JOIN battle_pets pet ON pet.guid = req.battlePetGuid WHERE req.requestKey = ?",
    )
    .bind(request_key.as_bytes().as_slice())
    .fetch_optional(&mut **tx)
    .await
    .map_err(|error| database_error_like_cpp(DatabaseError::from(error)))?;
    let Some(row) = row else {
        return Ok(None);
    };
    let account_id = row_u32_signed_or_unsigned_like_cpp(&row, 0)?;
    let pet = DurableBattlePetRowLikeCpp {
        guid_counter: row_u64_signed_or_unsigned_like_cpp(&row, 1)?,
        species: row_u32_signed_or_unsigned_like_cpp(&row, 2)?,
        breed: row_u16_signed_or_unsigned_like_cpp(&row, 3)?,
        display_id: row_u32_signed_or_unsigned_like_cpp(&row, 4)?,
        level: row_u16_signed_or_unsigned_like_cpp(&row, 5)?,
        exp: row_u16_signed_or_unsigned_like_cpp(&row, 6)?,
        health: row_u32_signed_or_unsigned_like_cpp(&row, 7)?,
        quality: row_u8_signed_or_unsigned_like_cpp(&row, 8)?,
        flags: row_u16_signed_or_unsigned_like_cpp(&row, 9)?,
        name: row.try_get(10).map_err(row_decode_error_like_cpp)?,
        name_timestamp: row.try_get(11).map_err(row_decode_error_like_cpp)?,
        owner_guid_counter: row_opt_u64_signed_or_unsigned_like_cpp(&row, 12)?,
        declined_names: None,
    };
    let still_present = row.try_get(13).map_err(row_decode_error_like_cpp)?;
    Ok(Some((account_id, pet, still_present)))
}

fn durable_pet_from_row_like_cpp(
    row: &sqlx::mysql::MySqlRow,
) -> Result<DurableBattlePetRowLikeCpp, BattlePetPersistenceErrorLikeCpp> {
    let genitive: Option<String> = row.try_get(12).map_err(row_decode_error_like_cpp)?;
    Ok(DurableBattlePetRowLikeCpp {
        guid_counter: row_u64_signed_or_unsigned_like_cpp(row, 0)?,
        species: row_u32_signed_or_unsigned_like_cpp(row, 1)?,
        breed: row_u16_signed_or_unsigned_like_cpp(row, 2)?,
        display_id: row_u32_signed_or_unsigned_like_cpp(row, 3)?,
        level: row_u16_signed_or_unsigned_like_cpp(row, 4)?,
        exp: row_u16_signed_or_unsigned_like_cpp(row, 5)?,
        health: row_u32_signed_or_unsigned_like_cpp(row, 6)?,
        quality: row_u8_signed_or_unsigned_like_cpp(row, 7)?,
        flags: row_u16_signed_or_unsigned_like_cpp(row, 8)?,
        name: row.try_get(9).map_err(row_decode_error_like_cpp)?,
        name_timestamp: row.try_get(10).map_err(row_decode_error_like_cpp)?,
        owner_guid_counter: row_opt_u64_signed_or_unsigned_like_cpp(row, 11)?,
        declined_names: match genitive {
            None => None,
            Some(genitive) => Some(DeclinedNamesLikeCpp {
                names: [
                    genitive,
                    row.try_get(13).map_err(row_decode_error_like_cpp)?,
                    row.try_get(14).map_err(row_decode_error_like_cpp)?,
                    row.try_get(15).map_err(row_decode_error_like_cpp)?,
                    row.try_get(16).map_err(row_decode_error_like_cpp)?,
                ],
            }),
        },
    })
}

fn add_request_matches_like_cpp(
    requested: &DurableBattlePetRowLikeCpp,
    persisted: &DurableBattlePetRowLikeCpp,
) -> bool {
    requested.species == persisted.species
        && requested.breed == persisted.breed
        && requested.display_id == persisted.display_id
        && requested.level == persisted.level
        && requested.quality == persisted.quality
        && requested.owner_guid_counter == persisted.owner_guid_counter
}

fn request_matches_durable_like_cpp(
    request: &BattlePetAddRequestLikeCpp,
    persisted: &DurableBattlePetRowLikeCpp,
    species_store: &BattlePetSpeciesStore,
) -> bool {
    let Some(species) = species_store.get(request.species) else {
        return false;
    };
    let requested_owner = species
        .has_flag_like_cpp(BATTLE_PET_SPECIES_FLAG_NOT_ACCOUNT_WIDE_LIKE_CPP)
        .then(|| request.owner_guid.map(|guid| guid.counter() as u64))
        .flatten();
    request.species == persisted.species
        && request.breed == persisted.breed
        && request.display_id == persisted.display_id
        && request.level == persisted.level
        && request.quality == persisted.quality
        && requested_owner == persisted.owner_guid_counter
}

fn database_error_like_cpp(error: DatabaseError) -> BattlePetPersistenceErrorLikeCpp {
    BattlePetPersistenceErrorLikeCpp::Database(error.to_string())
}

fn row_decode_error_like_cpp(error: sqlx::Error) -> BattlePetPersistenceErrorLikeCpp {
    BattlePetPersistenceErrorLikeCpp::Database(error.to_string())
}

/// Tolerant unsigned readers for the legacy battle-pet columns (#175).
///
/// The legacy C++ schema declares these columns signed
/// (`sql/base/auth_database.sql` in woltk-trinity-legacy), while the Rust
/// structs use unsigned types and sqlx rejects the conversion outright. C++
/// reads the same signed columns into `uint64`/`uint32`/`uint16`/`uint8`
/// fields without caring about the declared sign. Mirror that: try the exact
/// unsigned type first (covers rustycore-migrated unsigned schemas), then
/// fall back to the signed column type with a range check.
macro_rules! battle_pet_signed_or_unsigned_readers_like_cpp {
    ($cast_fn:ident, $row_fn:ident, $result_fn:ident, $unsigned:ty, $signed:ty) => {
        /// Checked signed→unsigned conversion shared by both tolerant readers.
        fn $cast_fn(
            raw: $signed,
            column: usize,
        ) -> Result<$unsigned, BattlePetPersistenceErrorLikeCpp> {
            <$unsigned>::try_from(raw).map_err(|_| {
                BattlePetPersistenceErrorLikeCpp::Database(format!(
                    "negative value {raw} in battle-pet column {column}"
                ))
            })
        }

        fn $row_fn(
            row: &sqlx::mysql::MySqlRow,
            column: usize,
        ) -> Result<$unsigned, BattlePetPersistenceErrorLikeCpp> {
            if let Ok(value) = row.try_get::<$unsigned, _>(column) {
                return Ok(value);
            }
            let raw: $signed = row.try_get(column).map_err(row_decode_error_like_cpp)?;
            $cast_fn(raw, column)
        }

        fn $result_fn(result: &wow_database::SqlResult, column: usize) -> Option<$unsigned> {
            result.try_read::<$unsigned>(column).or_else(|| {
                result
                    .try_read::<$signed>(column)
                    .and_then(|raw| $cast_fn(raw, column).ok())
            })
        }
    };
}

battle_pet_signed_or_unsigned_readers_like_cpp!(
    battle_pet_column_i64_as_u64_like_cpp,
    row_u64_signed_or_unsigned_like_cpp,
    result_u64_signed_or_unsigned_like_cpp,
    u64,
    i64
);
battle_pet_signed_or_unsigned_readers_like_cpp!(
    battle_pet_column_i32_as_u32_like_cpp,
    row_u32_signed_or_unsigned_like_cpp,
    result_u32_signed_or_unsigned_like_cpp,
    u32,
    i32
);
battle_pet_signed_or_unsigned_readers_like_cpp!(
    battle_pet_column_i16_as_u16_like_cpp,
    row_u16_signed_or_unsigned_like_cpp,
    result_u16_signed_or_unsigned_like_cpp,
    u16,
    i16
);
battle_pet_signed_or_unsigned_readers_like_cpp!(
    battle_pet_column_i8_as_u8_like_cpp,
    row_u8_signed_or_unsigned_like_cpp,
    result_u8_signed_or_unsigned_like_cpp,
    u8,
    i8
);

/// Nullable variant of `row_u64_signed_or_unsigned_like_cpp` (`battle_pets.owner`).
fn row_opt_u64_signed_or_unsigned_like_cpp(
    row: &sqlx::mysql::MySqlRow,
    column: usize,
) -> Result<Option<u64>, BattlePetPersistenceErrorLikeCpp> {
    match row.try_get::<Option<u64>, _>(column) {
        Ok(value) => Ok(value),
        Err(_) => {
            let raw: Option<i64> = row.try_get(column).map_err(row_decode_error_like_cpp)?;
            raw.map(|value| battle_pet_column_i64_as_u64_like_cpp(value, column))
                .transpose()
        }
    }
}

fn is_duplicate_key_like_cpp(error: &DatabaseError) -> bool {
    matches!(
        error,
        DatabaseError::Query(sqlx::Error::Database(database_error))
            if database_error.code().as_deref() == Some("1062")
    )
}

fn validate_add_lease_like_cpp(
    lease_holder: Option<BattlePetLeaseIdLikeCpp>,
    lease_id: BattlePetLeaseIdLikeCpp,
) -> Result<(), BattlePetAddFailureLikeCpp> {
    if lease_holder == Some(lease_id) {
        return Ok(());
    }
    Err(if lease_holder.is_some() {
        BattlePetAddFailureLikeCpp::JournalLocked
    } else {
        BattlePetAddFailureLikeCpp::MissingAuthority
    })
}

fn validate_mutation_lease_like_cpp(
    lease_holder: Option<BattlePetLeaseIdLikeCpp>,
    lease_id: BattlePetLeaseIdLikeCpp,
) -> Result<(), BattlePetMutationFailureLikeCpp> {
    if lease_holder == Some(lease_id) {
        return Ok(());
    }
    Err(if lease_holder.is_some() {
        BattlePetMutationFailureLikeCpp::JournalLocked
    } else {
        BattlePetMutationFailureLikeCpp::MissingAuthority
    })
}

fn validate_process_add_lease_like_cpp(
    process: &BattlePetProcessLeaseStateLikeCpp,
    lease_id: BattlePetLeaseIdLikeCpp,
) -> Result<u64, BattlePetAddFailureLikeCpp> {
    let Some(guard) = process
        .guard
        .as_ref()
        .filter(|guard| guard.is_valid_like_cpp())
    else {
        return Err(BattlePetAddFailureLikeCpp::MissingAuthority);
    };
    validate_add_lease_like_cpp(process.lease_holder, lease_id)?;
    Ok(guard.fence_like_cpp())
}

fn validate_process_mutation_lease_like_cpp(
    process: &BattlePetProcessLeaseStateLikeCpp,
    lease_id: BattlePetLeaseIdLikeCpp,
) -> Result<u64, BattlePetMutationFailureLikeCpp> {
    let Some(guard) = process
        .guard
        .as_ref()
        .filter(|guard| guard.is_valid_like_cpp())
    else {
        return Err(BattlePetMutationFailureLikeCpp::MissingAuthority);
    };
    validate_mutation_lease_like_cpp(process.lease_holder, lease_id)?;
    Ok(guard.fence_like_cpp())
}

fn add_persistence_error_like_cpp(
    error: BattlePetPersistenceErrorLikeCpp,
) -> BattlePetAddFailureLikeCpp {
    match error {
        BattlePetPersistenceErrorLikeCpp::Database(error) => {
            BattlePetAddFailureLikeCpp::DatabaseFailure(error)
        }
        BattlePetPersistenceErrorLikeCpp::Capacity => BattlePetAddFailureLikeCpp::Capacity,
        BattlePetPersistenceErrorLikeCpp::GuidCollision => {
            BattlePetAddFailureLikeCpp::GuidCollision
        }
        BattlePetPersistenceErrorLikeCpp::DuplicateRequest => {
            BattlePetAddFailureLikeCpp::DuplicateRequest
        }
        BattlePetPersistenceErrorLikeCpp::StaleAuthority => {
            BattlePetAddFailureLikeCpp::MissingAuthority
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct BattlePetLeaseIdLikeCpp(u64);

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BattlePetAddRequestLikeCpp {
    pub request_key: BattlePetAddRequestKeyLikeCpp,
    pub species: u32,
    pub display_id: u32,
    pub breed: u16,
    pub quality: u8,
    pub level: u16,
    pub owner_guid: Option<ObjectGuid>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum BattlePetAddFailureLikeCpp {
    MissingAuthority,
    JournalLocked,
    InvalidSpecies,
    Capacity,
    DuplicateRequest,
    Busy,
    DatabaseFailure(String),
    GuidCollision,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum BattlePetAddOutcomeLikeCpp {
    Added(BattlePetJournalPet),
    Replayed(BattlePetJournalPet),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum BattlePetMutationFailureLikeCpp {
    MissingAuthority,
    JournalLocked,
    UnknownPet,
    Busy,
    DatabaseFailure(String),
}

struct PendingAddLikeCpp {
    request: BattlePetAddRequestLikeCpp,
    completion: watch::Sender<bool>,
}

struct BattlePetAccountStateLikeCpp {
    pets: HashMap<ObjectGuid, RepresentedBattlePetDataLikeCpp>,
    slots: [RepresentedBattlePetSlotLikeCpp; BATTLE_PET_SLOT_COUNT_LIKE_CPP],
    pending_adds: HashMap<BattlePetAddRequestKeyLikeCpp, PendingAddLikeCpp>,
    completed_adds:
        HashMap<BattlePetAddRequestKeyLikeCpp, (ObjectGuid, DurableBattlePetRowLikeCpp)>,
    pending_pet_mutations: HashSet<ObjectGuid>,
    slots_pending: bool,
}

type BattlePetAccountCellLikeCpp = OnceCell<Arc<BattlePetAccountOwnerLikeCpp>>;
type BattlePetAccountMapLikeCpp = DashMap<u32, Arc<BattlePetAccountCellLikeCpp>>;

#[derive(Clone)]
struct BattlePetAccountRegistryIdentityLikeCpp {
    accounts: Weak<BattlePetAccountMapLikeCpp>,
    cell: Weak<BattlePetAccountCellLikeCpp>,
}

pub(crate) struct BattlePetAccountOwnerLikeCpp {
    account_id: u32,
    realm_id: u16,
    virtual_realm_address: u32,
    persistence: Arc<dyn BattlePetPersistenceLikeCpp>,
    species_store: Arc<BattlePetSpeciesStore>,
    breed_quality_store: Arc<BattlePetBreedQualityStore>,
    breed_state_store: Arc<BattlePetBreedStateStore>,
    species_state_store: Arc<BattlePetSpeciesStateStore>,
    state: Mutex<BattlePetAccountStateLikeCpp>,
    process_lease: Mutex<BattlePetProcessLeaseStateLikeCpp>,
    process_lease_changed: Notify,
    operations_drained: Notify,
    registry_identity: Mutex<Option<BattlePetAccountRegistryIdentityLikeCpp>>,
}

/// Outcome of a cross-account receipt probe serialized by the #160 process
/// fence (issue #161): the answer is only meaningful when the fence could be
/// held across the read, so no in-flight original-account insert can race it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BattlePetFencedReceiptProbeLikeCpp {
    Committed,
    Absent,
    AuthorityUnavailable,
}

impl BattlePetAccountOwnerLikeCpp {
    fn from_loaded_like_cpp(
        account_id: u32,
        realm_id: u16,
        virtual_realm_address: u32,
        persistence: Arc<dyn BattlePetPersistenceLikeCpp>,
        species_store: Arc<BattlePetSpeciesStore>,
        breed_quality_store: Arc<BattlePetBreedQualityStore>,
        breed_state_store: Arc<BattlePetBreedStateStore>,
        species_state_store: Arc<BattlePetSpeciesStateStore>,
        loaded: LoadedBattlePetAccountLikeCpp,
    ) -> Self {
        // C++ `BattlePetMgr::LoadFromDB` validates each row before it enters
        // `_pets`; capacity is therefore evaluated against only earlier valid
        // rows in the database result order.
        let mut pets = HashMap::new();
        for row in loaded.pets {
            let Some(species) = species_store.get(row.species) else {
                continue;
            };
            let not_account_wide =
                species.has_flag_like_cpp(BATTLE_PET_SPECIES_FLAG_NOT_ACCOUNT_WIDE_LIKE_CPP);
            if not_account_wide != row.owner_guid_counter.is_some() {
                continue;
            }
            let owner_guid = row
                .owner_guid_counter
                .map(|counter| ObjectGuid::create_player(realm_id, counter as i64));
            let max = if species
                .has_flag_like_cpp(BATTLE_PET_SPECIES_FLAG_LEGACY_ACCOUNT_UNIQUE_LIKE_CPP)
            {
                1
            } else {
                DEFAULT_MAX_BATTLE_PETS_PER_SPECIES_LIKE_CPP
            };
            if count_materialized_species_like_cpp(&pets, row.species, owner_guid, species.flags)
                >= max
            {
                continue;
            }

            let guid = battle_pet_guid_like_cpp(row.guid_counter);
            let pet = materialize_pet_like_cpp(
                &row,
                realm_id,
                virtual_realm_address,
                &species_store,
                &breed_quality_store,
                &breed_state_store,
                &species_state_store,
            );
            pets.insert(guid, pet);
        }
        let mut slots =
            std::array::from_fn(|index| RepresentedBattlePetSlotLikeCpp::locked_empty(index as u8));
        for slot in loaded.slots {
            if let Some(target) = slots.get_mut(slot.index as usize) {
                target.pet_guid = slot
                    .pet_guid_counter
                    .map(battle_pet_guid_like_cpp)
                    .filter(|guid| pets.contains_key(guid));
                target.locked = slot.locked;
            }
        }
        Self {
            account_id,
            realm_id,
            virtual_realm_address,
            persistence,
            species_store,
            breed_quality_store,
            breed_state_store,
            species_state_store,
            state: Mutex::new(BattlePetAccountStateLikeCpp {
                pets,
                slots,
                pending_adds: HashMap::new(),
                completed_adds: HashMap::new(),
                pending_pet_mutations: HashSet::new(),
                slots_pending: false,
            }),
            process_lease: Mutex::new(BattlePetProcessLeaseStateLikeCpp {
                guard: None,
                acquiring: false,
                attachments: 0,
                active_operations: 0,
                lease_holder: None,
            }),
            process_lease_changed: Notify::new(),
            operations_drained: Notify::new(),
            registry_identity: Mutex::new(None),
        }
    }

    fn begin_operation_like_cpp(self: &Arc<Self>) -> BattlePetOperationGuardLikeCpp {
        self.process_lease
            .lock()
            .expect("battle-pet process lease poisoned")
            .active_operations += 1;
        BattlePetOperationGuardLikeCpp {
            owner: Arc::clone(self),
        }
    }

    fn set_registry_identity_like_cpp(
        &self,
        accounts: &Arc<BattlePetAccountMapLikeCpp>,
        cell: &Arc<BattlePetAccountCellLikeCpp>,
    ) {
        let mut identity = self
            .registry_identity
            .lock()
            .expect("battle-pet registry identity poisoned");
        if identity.is_none() {
            *identity = Some(BattlePetAccountRegistryIdentityLikeCpp {
                accounts: Arc::downgrade(accounts),
                cell: Arc::downgrade(cell),
            });
        }
    }

    fn try_evict_if_idle_like_cpp(&self) {
        let identity = self
            .registry_identity
            .lock()
            .expect("battle-pet registry identity poisoned")
            .clone();
        let Some(identity) = identity else {
            return;
        };
        let (Some(accounts), Some(cell)) = (identity.accounts.upgrade(), identity.cell.upgrade())
        else {
            return;
        };
        let dashmap::mapref::entry::Entry::Occupied(entry) = accounts.entry(self.account_id) else {
            return;
        };
        if !Arc::ptr_eq(entry.get(), &cell) {
            return;
        }
        let mut process = self
            .process_lease
            .lock()
            .expect("battle-pet process lease poisoned");
        if process.attachments == 0
            && process.active_operations == 0
            && process.lease_holder.is_none()
            && !process.acquiring
        {
            process.guard.take();
            entry.remove();
        }
    }

    async fn wait_until_operations_drained_like_cpp(&self) {
        loop {
            let notified = self.operations_drained.notified();
            if self
                .process_lease
                .lock()
                .expect("battle-pet process lease poisoned")
                .active_operations
                == 0
            {
                return;
            }
            notified.await;
        }
    }

    async fn ensure_process_lease_like_cpp(self: &Arc<Self>) -> bool {
        loop {
            let wait = {
                let mut process = self
                    .process_lease
                    .lock()
                    .expect("battle-pet process lease poisoned");
                if process
                    .guard
                    .as_ref()
                    .is_some_and(|guard| guard.is_valid_like_cpp())
                {
                    return true;
                }
                process.guard.take();
                if process.acquiring {
                    Some(self.process_lease_changed.notified())
                } else {
                    process.acquiring = true;
                    None
                }
            };
            if let Some(wait) = wait {
                wait.await;
                continue;
            }

            let acquired = self
                .persistence
                .try_acquire_process_lease(self.account_id)
                .await;
            let mut guard = match acquired {
                Ok(guard) => guard,
                Err(_) => None,
            };
            if guard.is_some() {
                let loaded = self
                    .persistence
                    .load_account(self.account_id, self.realm_id)
                    .await;
                if let Ok(loaded) = loaded {
                    let refreshed = Self::from_loaded_like_cpp(
                        self.account_id,
                        self.realm_id,
                        self.virtual_realm_address,
                        Arc::clone(&self.persistence),
                        Arc::clone(&self.species_store),
                        Arc::clone(&self.breed_quality_store),
                        Arc::clone(&self.breed_state_store),
                        Arc::clone(&self.species_state_store),
                        loaded,
                    );
                    let refreshed = refreshed
                        .state
                        .into_inner()
                        .expect("refreshed battle-pet account state poisoned");
                    let mut state = self
                        .state
                        .lock()
                        .expect("battle-pet account state poisoned");
                    if state.pending_adds.is_empty()
                        && state.pending_pet_mutations.is_empty()
                        && !state.slots_pending
                    {
                        state.pets = refreshed.pets;
                        state.slots = refreshed.slots;
                        state.completed_adds.clear();
                    } else {
                        guard = None;
                    }
                } else {
                    guard = None;
                }
            }

            let acquired = guard.is_some();
            let mut process = self
                .process_lease
                .lock()
                .expect("battle-pet process lease poisoned");
            process.guard = guard;
            process.acquiring = false;
            drop(process);
            self.process_lease_changed.notify_waiters();
            return acquired;
        }
    }

    async fn try_acquire_lease_like_cpp(
        self: &Arc<Self>,
        lease_id: BattlePetLeaseIdLikeCpp,
    ) -> bool {
        loop {
            if !self.ensure_process_lease_like_cpp().await {
                return false;
            }
            let mut process = self
                .process_lease
                .lock()
                .expect("battle-pet process lease poisoned");
            if !process
                .guard
                .as_ref()
                .is_some_and(|guard| guard.is_valid_like_cpp())
            {
                process.guard.take();
                continue;
            }
            return match process.lease_holder {
                None => {
                    process.lease_holder = Some(lease_id);
                    true
                }
                Some(holder) => holder == lease_id,
            };
        }
    }

    fn has_lease_like_cpp(&self, lease_id: BattlePetLeaseIdLikeCpp) -> bool {
        let process = self
            .process_lease
            .lock()
            .expect("battle-pet process lease poisoned");
        process.lease_holder == Some(lease_id)
            && process
                .guard
                .as_ref()
                .is_some_and(|guard| guard.is_valid_like_cpp())
    }

    fn has_process_fence_like_cpp(&self, fence: u64) -> bool {
        self.process_lease
            .lock()
            .expect("battle-pet process lease poisoned")
            .guard
            .as_ref()
            .is_some_and(|guard| guard.is_valid_like_cpp() && guard.fence_like_cpp() == fence)
    }

    pub(crate) fn journal_like_cpp(
        &self,
        lease_id: BattlePetLeaseIdLikeCpp,
        player_guid: Option<ObjectGuid>,
    ) -> BattlePetJournal {
        let has_journal_lock = self.has_lease_like_cpp(lease_id);
        let state = self
            .state
            .lock()
            .expect("battle-pet account state poisoned");
        let mut pets: Vec<_> = state
            .pets
            .iter()
            .filter(|(_, pet)| pet.save_info != RepresentedBattlePetSaveInfoLikeCpp::Removed)
            .filter(|(_, pet)| {
                pet.owner_info
                    .is_none_or(|owner| Some(owner.guid) == player_guid)
            })
            .map(|(guid, pet)| pet.packet_info_like_cpp(*guid))
            .collect();
        pets.sort_by_key(|pet| pet.guid.counter());
        BattlePetJournal {
            trap: 0,
            has_journal_lock,
            slots: state
                .slots
                .iter()
                .map(|slot| {
                    let mut packet = slot.packet_slot_like_cpp();
                    if !packet.pet_guid.is_empty() && !state.pets.contains_key(&packet.pet_guid) {
                        packet.pet_guid = empty_battle_pet_guid_like_cpp();
                    }
                    packet
                })
                .collect(),
            pets,
        }
    }

    pub(crate) fn pet_snapshot_like_cpp(
        &self,
        pet_guid: ObjectGuid,
    ) -> Option<RepresentedBattlePetDataLikeCpp> {
        self.state
            .lock()
            .expect("battle-pet account state poisoned")
            .pets
            .get(&pet_guid)
            .cloned()
    }

    pub(crate) fn max_pet_level_like_cpp(&self) -> u16 {
        self.state
            .lock()
            .expect("battle-pet account state poisoned")
            .pets
            .values()
            .filter(|pet| pet.save_info != RepresentedBattlePetSaveInfoLikeCpp::Removed)
            .map(|pet| pet.level)
            .max()
            .unwrap_or(0)
    }

    pub(crate) fn has_max_pet_count_like_cpp(
        &self,
        species: u32,
        owner_guid: Option<ObjectGuid>,
    ) -> bool {
        let Some(entry) = self.species_store.get(species) else {
            return false;
        };
        let max = if entry.has_flag_like_cpp(BATTLE_PET_SPECIES_FLAG_LEGACY_ACCOUNT_UNIQUE_LIKE_CPP)
        {
            1
        } else {
            DEFAULT_MAX_BATTLE_PETS_PER_SPECIES_LIKE_CPP
        };
        let state = self
            .state
            .lock()
            .expect("battle-pet account state poisoned");
        count_species_like_cpp(&state, species, owner_guid, entry.flags) >= max
    }

    pub(crate) fn pet_count_like_cpp(&self, species: u32, owner_guid: Option<ObjectGuid>) -> u8 {
        let Some(entry) = self.species_store.get(species) else {
            return 0;
        };
        count_species_like_cpp(
            &self
                .state
                .lock()
                .expect("battle-pet account state poisoned"),
            species,
            owner_guid,
            entry.flags,
        )
    }

    pub(crate) fn unique_species_count_like_cpp(&self) -> u32 {
        let state = self
            .state
            .lock()
            .expect("battle-pet account state poisoned");
        u32::try_from(
            state
                .pets
                .values()
                .map(|pet| pet.species)
                .collect::<std::collections::BTreeSet<_>>()
                .len(),
        )
        .unwrap_or(u32::MAX)
    }

    /// Receipt probe for an account other than this owner's, serialized by
    /// the original account's process fence. Any in-flight original-account
    /// insert holds that fence through its owner guard, so
    /// `AuthorityUnavailable` must defer rather than guess: without the
    /// fence a negative read is only a snapshot that a detached worker can
    /// falsify immediately afterwards.
    pub(crate) async fn receipt_probe_for_account_fenced_like_cpp(
        &self,
        account_id: u32,
        request_key: BattlePetAddRequestKeyLikeCpp,
    ) -> Result<BattlePetFencedReceiptProbeLikeCpp, BattlePetAddFailureLikeCpp> {
        let guard = self
            .persistence
            .try_acquire_process_lease(account_id)
            .await
            .map_err(add_persistence_error_like_cpp)?;
        let Some(_guard) = guard else {
            return Ok(BattlePetFencedReceiptProbeLikeCpp::AuthorityUnavailable);
        };
        let committed = self
            .persistence
            .lookup_add_request(account_id, request_key)
            .await
            .map_err(add_persistence_error_like_cpp)?
            .is_some();
        Ok(if committed {
            BattlePetFencedReceiptProbeLikeCpp::Committed
        } else {
            BattlePetFencedReceiptProbeLikeCpp::Absent
        })
    }

    /// Receipt probe for an account other than this owner's — used by the
    /// #161 purchase saga when a character changed Battle.net accounts
    /// mid-purchase: the receipt authority stays the original account.
    pub(crate) async fn receipt_committed_for_account_like_cpp(
        &self,
        account_id: u32,
        request_key: BattlePetAddRequestKeyLikeCpp,
    ) -> Result<bool, BattlePetAddFailureLikeCpp> {
        self.persistence
            .lookup_add_request(account_id, request_key)
            .await
            .map(|receipt| receipt.is_some())
            .map_err(add_persistence_error_like_cpp)
    }

    pub(crate) async fn add_request_committed_like_cpp(
        &self,
        request_key: BattlePetAddRequestKeyLikeCpp,
    ) -> Result<bool, BattlePetAddFailureLikeCpp> {
        if self
            .state
            .lock()
            .expect("battle-pet account state poisoned")
            .completed_adds
            .contains_key(&request_key)
        {
            return Ok(true);
        }
        self.persistence
            .lookup_add_request(self.account_id, request_key)
            .await
            .map(|receipt| receipt.is_some())
            .map_err(add_persistence_error_like_cpp)
    }

    pub(crate) async fn try_add_pet_like_cpp(
        self: &Arc<Self>,
        lease_id: BattlePetLeaseIdLikeCpp,
        request: BattlePetAddRequestLikeCpp,
    ) -> Result<BattlePetAddOutcomeLikeCpp, BattlePetAddFailureLikeCpp> {
        let operation_guard = self.begin_operation_like_cpp();
        {
            let process = self
                .process_lease
                .lock()
                .expect("battle-pet process lease poisoned");
            validate_process_add_lease_like_cpp(&process, lease_id)?;
            let state = self
                .state
                .lock()
                .expect("battle-pet account state poisoned");
            if let Some((guid, durable)) = state.completed_adds.get(&request.request_key) {
                if !request_matches_durable_like_cpp(&request, durable, &self.species_store) {
                    return Err(BattlePetAddFailureLikeCpp::DuplicateRequest);
                }
                let pet = state
                    .pets
                    .get(guid)
                    .expect("completed battle-pet request must name canonical pet");
                return Ok(BattlePetAddOutcomeLikeCpp::Replayed(
                    pet.packet_info_like_cpp(*guid),
                ));
            }
        }

        let persisted_request = self
            .persistence
            .lookup_add_request(self.account_id, request.request_key)
            .await
            .map_err(add_persistence_error_like_cpp)?;
        if let Some(receipt) = persisted_request {
            if !request_matches_durable_like_cpp(
                &request,
                &receipt.requested_pet,
                &self.species_store,
            ) {
                return Err(BattlePetAddFailureLikeCpp::DuplicateRequest);
            }
            let Some(current_pet) = receipt.current_pet else {
                return Err(BattlePetAddFailureLikeCpp::DuplicateRequest);
            };
            let guid = battle_pet_guid_like_cpp(current_pet.guid_counter);
            let process = self
                .process_lease
                .lock()
                .expect("battle-pet process lease poisoned");
            validate_process_add_lease_like_cpp(&process, lease_id)?;
            let mut state = self
                .state
                .lock()
                .expect("battle-pet account state poisoned");
            if state.pending_pet_mutations.contains(&guid) {
                return Err(BattlePetAddFailureLikeCpp::Busy);
            }
            let packet = state
                .pets
                .get(&guid)
                .ok_or(BattlePetAddFailureLikeCpp::DuplicateRequest)?
                .packet_info_like_cpp(guid);
            state
                .completed_adds
                .insert(request.request_key, (guid, receipt.requested_pet));
            return Ok(BattlePetAddOutcomeLikeCpp::Replayed(packet));
        }

        let fence = loop {
            let (wait, reserved_fence) = {
                let process = self
                    .process_lease
                    .lock()
                    .expect("battle-pet process lease poisoned");
                let fence = validate_process_add_lease_like_cpp(&process, lease_id)?;
                let mut state = self
                    .state
                    .lock()
                    .expect("battle-pet account state poisoned");
                if let Some((guid, durable)) = state.completed_adds.get(&request.request_key) {
                    if !request_matches_durable_like_cpp(&request, durable, &self.species_store) {
                        return Err(BattlePetAddFailureLikeCpp::DuplicateRequest);
                    }
                    let pet = state
                        .pets
                        .get(guid)
                        .expect("completed battle-pet request must name canonical pet");
                    return Ok(BattlePetAddOutcomeLikeCpp::Replayed(
                        pet.packet_info_like_cpp(*guid),
                    ));
                }
                if let Some(pending) = state.pending_adds.get(&request.request_key) {
                    if pending.request != request {
                        return Err(BattlePetAddFailureLikeCpp::DuplicateRequest);
                    }
                    (Some(pending.completion.subscribe()), None)
                } else {
                    let Some(species) = self.species_store.get(request.species) else {
                        return Err(BattlePetAddFailureLikeCpp::InvalidSpecies);
                    };
                    if !species.has_flag_like_cpp(BATTLE_PET_SPECIES_FLAG_WELL_KNOWN_LIKE_CPP) {
                        return Err(BattlePetAddFailureLikeCpp::InvalidSpecies);
                    }
                    if species.has_flag_like_cpp(BATTLE_PET_SPECIES_FLAG_NOT_ACCOUNT_WIDE_LIKE_CPP)
                        && request.owner_guid.is_none()
                    {
                        return Err(BattlePetAddFailureLikeCpp::MissingAuthority);
                    }
                    let max = if species
                        .has_flag_like_cpp(BATTLE_PET_SPECIES_FLAG_LEGACY_ACCOUNT_UNIQUE_LIKE_CPP)
                    {
                        1
                    } else {
                        DEFAULT_MAX_BATTLE_PETS_PER_SPECIES_LIKE_CPP
                    };
                    let persisted_count = count_species_like_cpp(
                        &state,
                        request.species,
                        request.owner_guid,
                        species.flags,
                    );
                    let reserved_count = state
                        .pending_adds
                        .values()
                        .filter(|pending| {
                            pending.request.species == request.species
                                && owner_matches_like_cpp(
                                    species.flags,
                                    pending.request.owner_guid,
                                    request.owner_guid,
                                )
                        })
                        .count();
                    if usize::from(persisted_count) + reserved_count >= usize::from(max) {
                        return Err(BattlePetAddFailureLikeCpp::Capacity);
                    }
                    let (completion, _) = watch::channel(false);
                    state.pending_adds.insert(
                        request.request_key,
                        PendingAddLikeCpp {
                            request: request.clone(),
                            completion,
                        },
                    );
                    (None, Some(fence))
                }
            };
            if let Some(mut wait) = wait {
                if !*wait.borrow() {
                    let _ = wait.changed().await;
                }
                continue;
            }
            break reserved_fence.expect("new pending add must capture its process fence");
        };

        let owner = Arc::clone(self);
        tokio::spawn(async move {
            let _operation_guard = operation_guard;
            owner.finish_add_pet_like_cpp(request, fence).await
        })
        .await
        .map_err(|error| {
            BattlePetAddFailureLikeCpp::DatabaseFailure(format!(
                "battle-pet add worker failed: {error}"
            ))
        })?
    }

    async fn finish_add_pet_like_cpp(
        self: Arc<Self>,
        request: BattlePetAddRequestLikeCpp,
        fence: u64,
    ) -> Result<BattlePetAddOutcomeLikeCpp, BattlePetAddFailureLikeCpp> {
        let mut persistence_result = async {
            let counter = self.persistence.allocate_guid_counter_like_cpp().await?;
            let row =
                self.durable_new_pet_like_cpp(counter, &request)
                    .map_err(|error| match error {
                        BattlePetAddFailureLikeCpp::GuidCollision => {
                            BattlePetPersistenceErrorLikeCpp::GuidCollision
                        }
                        other => BattlePetPersistenceErrorLikeCpp::Database(format!(
                            "could not materialize allocated battle pet: {other:?}"
                        )),
                    })?;
            let outcome = self
                .persistence
                .insert_pet_idempotently(DurableBattlePetAddLikeCpp {
                    account_id: self.account_id,
                    realm_id: self.realm_id,
                    request_key: request.request_key,
                    max_per_scope: self
                        .species_store
                        .get(request.species)
                        .map(|species| {
                            if species.has_flag_like_cpp(
                                BATTLE_PET_SPECIES_FLAG_LEGACY_ACCOUNT_UNIQUE_LIKE_CPP,
                            ) {
                                1
                            } else {
                                DEFAULT_MAX_BATTLE_PETS_PER_SPECIES_LIKE_CPP
                            }
                        })
                        .ok_or_else(|| {
                            BattlePetPersistenceErrorLikeCpp::Database(
                                "validated battle-pet species disappeared".to_string(),
                            )
                        })?,
                    fence,
                    pet: row.clone(),
                })
                .await?;
            Ok::<_, BattlePetPersistenceErrorLikeCpp>((row, outcome))
        }
        .await;

        if persistence_result.is_ok() && !self.has_process_fence_like_cpp(fence) {
            persistence_result = Err(BattlePetPersistenceErrorLikeCpp::StaleAuthority);
        }

        let mut state = self
            .state
            .lock()
            .expect("battle-pet account state poisoned");
        let pending = state
            .pending_adds
            .remove(&request.request_key)
            .expect("battle-pet reservation disappeared before persistence completed");
        let result = match persistence_result {
            Ok((row, outcome)) => {
                let replayed =
                    matches!(outcome, PersistBattlePetAddOutcomeLikeCpp::Replayed { .. });
                let durable = match outcome {
                    PersistBattlePetAddOutcomeLikeCpp::Inserted => row,
                    PersistBattlePetAddOutcomeLikeCpp::Replayed {
                        pet,
                        still_present: true,
                    } => pet,
                    PersistBattlePetAddOutcomeLikeCpp::Replayed {
                        still_present: false,
                        ..
                    } => {
                        let _ = pending.completion.send(true);
                        return Err(BattlePetAddFailureLikeCpp::DuplicateRequest);
                    }
                };
                let guid = battle_pet_guid_like_cpp(durable.guid_counter);
                let pet = materialize_pet_like_cpp(
                    &durable,
                    self.realm_id,
                    self.virtual_realm_address,
                    &self.species_store,
                    &self.breed_quality_store,
                    &self.breed_state_store,
                    &self.species_state_store,
                );
                let packet = pet.packet_info_like_cpp(guid);
                state.pets.insert(guid, pet);
                state
                    .completed_adds
                    .insert(request.request_key, (guid, durable.clone()));
                if replayed {
                    Ok(BattlePetAddOutcomeLikeCpp::Replayed(packet))
                } else {
                    Ok(BattlePetAddOutcomeLikeCpp::Added(packet))
                }
            }
            Err(BattlePetPersistenceErrorLikeCpp::GuidCollision) => {
                Err(BattlePetAddFailureLikeCpp::GuidCollision)
            }
            Err(BattlePetPersistenceErrorLikeCpp::Capacity) => {
                Err(BattlePetAddFailureLikeCpp::Capacity)
            }
            Err(BattlePetPersistenceErrorLikeCpp::DuplicateRequest) => {
                Err(BattlePetAddFailureLikeCpp::DuplicateRequest)
            }
            Err(BattlePetPersistenceErrorLikeCpp::StaleAuthority) => {
                Err(BattlePetAddFailureLikeCpp::MissingAuthority)
            }
            Err(BattlePetPersistenceErrorLikeCpp::Database(error)) => {
                Err(BattlePetAddFailureLikeCpp::DatabaseFailure(error))
            }
        };
        let _ = pending.completion.send(true);
        result
    }

    pub(crate) async fn try_mutate_pet_like_cpp<R, F>(
        self: &Arc<Self>,
        lease_id: BattlePetLeaseIdLikeCpp,
        pet_guid: ObjectGuid,
        mutation: F,
    ) -> Result<(R, BattlePetJournalPet), BattlePetMutationFailureLikeCpp>
    where
        R: Send + 'static,
        F: FnOnce(&mut RepresentedBattlePetDataLikeCpp) -> R,
    {
        self.try_mutate_pet_with_optional_lease_like_cpp(Some(lease_id), pet_guid, mutation)
            .await
    }

    /// C++ `BattlePetMgr::ClearFanfare` is intentionally the one represented
    /// durable pet mutation without a `HasJournalLock()` gate. It still uses
    /// the canonical owner's per-pet mutation serialization and persistence.
    pub(crate) async fn try_mutate_pet_without_lease_like_cpp<R, F>(
        self: &Arc<Self>,
        pet_guid: ObjectGuid,
        mutation: F,
    ) -> Result<(R, BattlePetJournalPet), BattlePetMutationFailureLikeCpp>
    where
        R: Send + 'static,
        F: FnOnce(&mut RepresentedBattlePetDataLikeCpp) -> R,
    {
        if !self.ensure_process_lease_like_cpp().await {
            return Err(BattlePetMutationFailureLikeCpp::MissingAuthority);
        }
        self.try_mutate_pet_with_optional_lease_like_cpp(None, pet_guid, mutation)
            .await
    }

    async fn try_mutate_pet_with_optional_lease_like_cpp<R, F>(
        self: &Arc<Self>,
        lease_id: Option<BattlePetLeaseIdLikeCpp>,
        pet_guid: ObjectGuid,
        mutation: F,
    ) -> Result<(R, BattlePetJournalPet), BattlePetMutationFailureLikeCpp>
    where
        R: Send + 'static,
        F: FnOnce(&mut RepresentedBattlePetDataLikeCpp) -> R,
    {
        let operation_guard = self.begin_operation_like_cpp();
        let (mut changed, fence) = {
            let process = self
                .process_lease
                .lock()
                .expect("battle-pet process lease poisoned");
            let fence = if let Some(lease_id) = lease_id {
                validate_process_mutation_lease_like_cpp(&process, lease_id)?
            } else if !process
                .guard
                .as_ref()
                .is_some_and(|guard| guard.is_valid_like_cpp())
            {
                return Err(BattlePetMutationFailureLikeCpp::MissingAuthority);
            } else {
                process
                    .guard
                    .as_ref()
                    .expect("validated battle-pet process guard disappeared")
                    .fence_like_cpp()
            };
            let mut state = self
                .state
                .lock()
                .expect("battle-pet account state poisoned");
            if !state.pending_pet_mutations.insert(pet_guid) {
                return Err(BattlePetMutationFailureLikeCpp::Busy);
            }
            let Some(current) = state.pets.get(&pet_guid).cloned() else {
                state.pending_pet_mutations.remove(&pet_guid);
                return Err(BattlePetMutationFailureLikeCpp::UnknownPet);
            };
            (current, fence)
        };
        let outcome = mutation(&mut changed);
        changed.save_info = RepresentedBattlePetSaveInfoLikeCpp::Unchanged;
        let owner = Arc::clone(self);
        tokio::spawn(async move {
            let _operation_guard = operation_guard;
            owner
                .finish_mutate_pet_like_cpp(pet_guid, outcome, changed, fence)
                .await
        })
        .await
        .map_err(|error| {
            BattlePetMutationFailureLikeCpp::DatabaseFailure(format!(
                "battle-pet update worker failed: {error}"
            ))
        })?
    }

    async fn finish_mutate_pet_like_cpp<R: Send + 'static>(
        self: Arc<Self>,
        pet_guid: ObjectGuid,
        outcome: R,
        changed: RepresentedBattlePetDataLikeCpp,
        fence: u64,
    ) -> Result<(R, BattlePetJournalPet), BattlePetMutationFailureLikeCpp> {
        let durable = durable_from_pet_like_cpp(pet_guid, &changed);
        let mut persistence = self
            .persistence
            .update_pet(self.account_id, fence, durable)
            .await;
        if persistence.is_ok() && !self.has_process_fence_like_cpp(fence) {
            persistence = Err(BattlePetPersistenceErrorLikeCpp::StaleAuthority);
        }
        let mut state = self
            .state
            .lock()
            .expect("battle-pet account state poisoned");
        state.pending_pet_mutations.remove(&pet_guid);
        match persistence {
            Ok(()) => {
                let packet = changed.packet_info_like_cpp(pet_guid);
                state.pets.insert(pet_guid, changed);
                Ok((outcome, packet))
            }
            Err(error) => Err(mutation_persistence_error_like_cpp(error)),
        }
    }

    pub(crate) async fn try_remove_pet_like_cpp(
        self: &Arc<Self>,
        lease_id: BattlePetLeaseIdLikeCpp,
        pet_guid: ObjectGuid,
    ) -> Result<(), BattlePetMutationFailureLikeCpp> {
        let operation_guard = self.begin_operation_like_cpp();
        let (changed_slots, fence) = {
            let process = self
                .process_lease
                .lock()
                .expect("battle-pet process lease poisoned");
            let fence = validate_process_mutation_lease_like_cpp(&process, lease_id)?;
            let mut state = self
                .state
                .lock()
                .expect("battle-pet account state poisoned");
            if !state.pets.contains_key(&pet_guid) {
                return Err(BattlePetMutationFailureLikeCpp::UnknownPet);
            }
            if !state.pending_pet_mutations.insert(pet_guid) {
                return Err(BattlePetMutationFailureLikeCpp::Busy);
            }
            if state.slots_pending {
                state.pending_pet_mutations.remove(&pet_guid);
                return Err(BattlePetMutationFailureLikeCpp::Busy);
            }
            state.slots_pending = true;
            let mut changed = state.slots;
            for slot in &mut changed {
                if slot.pet_guid == Some(pet_guid) {
                    slot.pet_guid = None;
                }
            }
            (changed, fence)
        };
        let owner = Arc::clone(self);
        tokio::spawn(async move {
            let _operation_guard = operation_guard;
            owner
                .finish_remove_pet_like_cpp(pet_guid, changed_slots, fence)
                .await
        })
        .await
        .map_err(|error| {
            BattlePetMutationFailureLikeCpp::DatabaseFailure(format!(
                "battle-pet delete worker failed: {error}"
            ))
        })?
    }

    async fn finish_remove_pet_like_cpp(
        self: Arc<Self>,
        pet_guid: ObjectGuid,
        changed_slots: [RepresentedBattlePetSlotLikeCpp; BATTLE_PET_SLOT_COUNT_LIKE_CPP],
        fence: u64,
    ) -> Result<(), BattlePetMutationFailureLikeCpp> {
        let mut persistence = self
            .persistence
            .delete_pet(
                self.account_id,
                fence,
                pet_guid.counter() as u64,
                changed_slots.iter().map(durable_slot_like_cpp).collect(),
            )
            .await;
        if persistence.is_ok() && !self.has_process_fence_like_cpp(fence) {
            persistence = Err(BattlePetPersistenceErrorLikeCpp::StaleAuthority);
        }
        let mut state = self
            .state
            .lock()
            .expect("battle-pet account state poisoned");
        state.pending_pet_mutations.remove(&pet_guid);
        state.slots_pending = false;
        match persistence {
            Ok(()) => {
                state.pets.remove(&pet_guid);
                state.slots = changed_slots;
                Ok(())
            }
            Err(error) => Err(mutation_persistence_error_like_cpp(error)),
        }
    }

    pub(crate) async fn try_set_slot_like_cpp(
        self: &Arc<Self>,
        lease_id: BattlePetLeaseIdLikeCpp,
        pet_guid: ObjectGuid,
        slot_index: u8,
    ) -> Result<BattlePetJournalSlot, BattlePetMutationFailureLikeCpp> {
        let operation_guard = self.begin_operation_like_cpp();
        let (changed, fence) = {
            let process = self
                .process_lease
                .lock()
                .expect("battle-pet process lease poisoned");
            let fence = validate_process_mutation_lease_like_cpp(&process, lease_id)?;
            let mut state = self
                .state
                .lock()
                .expect("battle-pet account state poisoned");
            if !state.pets.contains_key(&pet_guid) {
                return Err(BattlePetMutationFailureLikeCpp::UnknownPet);
            }
            if state.slots_pending {
                return Err(BattlePetMutationFailureLikeCpp::Busy);
            }
            let mut changed = state.slots;
            let Some(slot) = changed.get_mut(slot_index as usize) else {
                return Err(BattlePetMutationFailureLikeCpp::UnknownPet);
            };
            slot.pet_guid = Some(pet_guid);
            state.slots_pending = true;
            (changed, fence)
        };
        let owner = Arc::clone(self);
        tokio::spawn(async move {
            let _operation_guard = operation_guard;
            owner
                .finish_set_slot_like_cpp(slot_index, changed, fence)
                .await
        })
        .await
        .map_err(|error| {
            BattlePetMutationFailureLikeCpp::DatabaseFailure(format!(
                "battle-pet slot worker failed: {error}"
            ))
        })?
    }

    async fn finish_set_slot_like_cpp(
        self: Arc<Self>,
        slot_index: u8,
        changed: [RepresentedBattlePetSlotLikeCpp; BATTLE_PET_SLOT_COUNT_LIKE_CPP],
        fence: u64,
    ) -> Result<BattlePetJournalSlot, BattlePetMutationFailureLikeCpp> {
        let durable = changed.iter().map(durable_slot_like_cpp).collect();
        let mut persistence = self
            .persistence
            .replace_slots(self.account_id, fence, durable)
            .await;
        if persistence.is_ok() && !self.has_process_fence_like_cpp(fence) {
            persistence = Err(BattlePetPersistenceErrorLikeCpp::StaleAuthority);
        }
        let mut state = self
            .state
            .lock()
            .expect("battle-pet account state poisoned");
        state.slots_pending = false;
        match persistence {
            Ok(()) => {
                state.slots = changed;
                Ok(state.slots[slot_index as usize].packet_slot_like_cpp())
            }
            Err(error) => Err(mutation_persistence_error_like_cpp(error)),
        }
    }

    fn durable_new_pet_like_cpp(
        &self,
        guid_counter: u64,
        request: &BattlePetAddRequestLikeCpp,
    ) -> Result<DurableBattlePetRowLikeCpp, BattlePetAddFailureLikeCpp> {
        let species = self
            .species_store
            .get(request.species)
            .ok_or(BattlePetAddFailureLikeCpp::InvalidSpecies)?;
        let stats = calculate_battle_pet_stats_like_cpp(
            request.breed,
            request.species,
            request.quality,
            request.level,
            &self.breed_state_store,
            &self.species_state_store,
            &self.breed_quality_store,
        );
        Ok(DurableBattlePetRowLikeCpp {
            guid_counter,
            species: request.species,
            breed: request.breed,
            display_id: request.display_id,
            level: request.level,
            exp: 0,
            health: stats.map_or(0, |stats| stats.max_health),
            quality: request.quality,
            flags: 0,
            name: String::new(),
            name_timestamp: 0,
            owner_guid_counter: species
                .has_flag_like_cpp(BATTLE_PET_SPECIES_FLAG_NOT_ACCOUNT_WIDE_LIKE_CPP)
                .then(|| request.owner_guid.map(|guid| guid.counter() as u64))
                .flatten(),
            declined_names: None,
        })
    }
}

fn count_species_like_cpp(
    state: &BattlePetAccountStateLikeCpp,
    species: u32,
    owner_guid: Option<ObjectGuid>,
    species_flags: i32,
) -> u8 {
    let count = state
        .pets
        .values()
        .filter(|pet| pet.species == species)
        .filter(|pet| pet.save_info != RepresentedBattlePetSaveInfoLikeCpp::Removed)
        .filter(|pet| {
            if species_flags & BATTLE_PET_SPECIES_FLAG_NOT_ACCOUNT_WIDE_LIKE_CPP != 0 {
                if let (Some(owner_guid), Some(owner_info)) = (owner_guid, pet.owner_info) {
                    return owner_info.guid == owner_guid;
                }
            }
            true
        })
        .count();
    u8::try_from(count).unwrap_or(u8::MAX)
}

fn count_materialized_species_like_cpp(
    pets: &HashMap<ObjectGuid, RepresentedBattlePetDataLikeCpp>,
    species: u32,
    owner_guid: Option<ObjectGuid>,
    species_flags: i32,
) -> u8 {
    let count = pets
        .values()
        .filter(|pet| pet.species == species)
        .filter(|pet| {
            species_flags & BATTLE_PET_SPECIES_FLAG_NOT_ACCOUNT_WIDE_LIKE_CPP == 0
                || pet.owner_info.map(|owner| owner.guid) == owner_guid
        })
        .count();
    u8::try_from(count).unwrap_or(u8::MAX)
}

fn owner_matches_like_cpp(
    species_flags: i32,
    left: Option<ObjectGuid>,
    right: Option<ObjectGuid>,
) -> bool {
    species_flags & BATTLE_PET_SPECIES_FLAG_NOT_ACCOUNT_WIDE_LIKE_CPP == 0 || left == right
}

fn materialize_pet_like_cpp(
    row: &DurableBattlePetRowLikeCpp,
    realm_id: u16,
    virtual_realm_address: u32,
    species_store: &BattlePetSpeciesStore,
    breed_quality_store: &BattlePetBreedQualityStore,
    breed_state_store: &BattlePetBreedStateStore,
    species_state_store: &BattlePetSpeciesStateStore,
) -> RepresentedBattlePetDataLikeCpp {
    let species = species_store.get(row.species);
    let stats = calculate_battle_pet_stats_like_cpp(
        row.breed,
        row.species,
        row.quality,
        row.level,
        breed_state_store,
        species_state_store,
        breed_quality_store,
    );
    RepresentedBattlePetDataLikeCpp {
        species: row.species,
        creature_id: species
            .and_then(|entry| u32::try_from(entry.creature_id).ok())
            .unwrap_or_default(),
        display_id: row.display_id,
        breed: row.breed,
        level: row.level,
        exp: row.exp,
        flags: row.flags,
        power: stats.map_or(0, |stats| stats.power),
        health: row.health,
        max_health: stats.map_or(0, |stats| stats.max_health),
        speed: stats.map_or(0, |stats| stats.speed),
        quality: row.quality,
        owner_info: row
            .owner_guid_counter
            .map(|counter| BattlePetJournalPetOwnerInfo {
                guid: ObjectGuid::create_player(realm_id, counter as i64),
                player_virtual_realm: virtual_realm_address,
                player_native_realm: virtual_realm_address,
            }),
        name: row.name.clone(),
        name_timestamp: row.name_timestamp,
        declined_names: row.declined_names.clone(),
        save_info: RepresentedBattlePetSaveInfoLikeCpp::Unchanged,
    }
}

fn durable_from_pet_like_cpp(
    guid: ObjectGuid,
    pet: &RepresentedBattlePetDataLikeCpp,
) -> DurableBattlePetRowLikeCpp {
    DurableBattlePetRowLikeCpp {
        guid_counter: guid.counter() as u64,
        species: pet.species,
        breed: pet.breed,
        display_id: pet.display_id,
        level: pet.level,
        exp: pet.exp,
        health: pet.health,
        quality: pet.quality,
        flags: pet.flags,
        name: pet.name.clone(),
        name_timestamp: pet.name_timestamp,
        owner_guid_counter: pet.owner_info.map(|owner| owner.guid.counter() as u64),
        declined_names: pet.declined_names.clone(),
    }
}

fn durable_slot_like_cpp(slot: &RepresentedBattlePetSlotLikeCpp) -> DurableBattlePetSlotLikeCpp {
    DurableBattlePetSlotLikeCpp {
        index: slot.index,
        pet_guid_counter: slot.pet_guid.map(|guid| guid.counter() as u64),
        locked: slot.locked,
    }
}

fn battle_pet_guid_like_cpp(counter: u64) -> ObjectGuid {
    ObjectGuid::create_global(HighGuid::BattlePet, 0, counter as i64)
}

fn mutation_persistence_error_like_cpp(
    error: BattlePetPersistenceErrorLikeCpp,
) -> BattlePetMutationFailureLikeCpp {
    if error == BattlePetPersistenceErrorLikeCpp::StaleAuthority {
        return BattlePetMutationFailureLikeCpp::MissingAuthority;
    }
    BattlePetMutationFailureLikeCpp::DatabaseFailure(match error {
        BattlePetPersistenceErrorLikeCpp::Database(error) => error,
        BattlePetPersistenceErrorLikeCpp::Capacity => "unexpected capacity failure".to_string(),
        BattlePetPersistenceErrorLikeCpp::GuidCollision => "unexpected GUID collision".to_string(),
        BattlePetPersistenceErrorLikeCpp::DuplicateRequest => {
            "unexpected duplicate request".to_string()
        }
        BattlePetPersistenceErrorLikeCpp::StaleAuthority => unreachable!(),
    })
}

struct BattlePetOperationGuardLikeCpp {
    owner: Arc<BattlePetAccountOwnerLikeCpp>,
}

impl Drop for BattlePetOperationGuardLikeCpp {
    fn drop(&mut self) {
        let drained = {
            let mut process = self
                .owner
                .process_lease
                .lock()
                .expect("battle-pet process lease poisoned");
            debug_assert!(process.active_operations != 0);
            process.active_operations -= 1;
            let drained = process.active_operations == 0;
            if process.attachments == 0 && drained && process.lease_holder.is_none() {
                process.guard.take();
            }
            drained
        };
        if drained {
            self.owner.operations_drained.notify_waiters();
        }
        self.owner.try_evict_if_idle_like_cpp();
    }
}

pub struct BattlePetAccountAttachmentLikeCpp {
    owner: Arc<BattlePetAccountOwnerLikeCpp>,
    lease_id: BattlePetLeaseIdLikeCpp,
}

impl BattlePetAccountAttachmentLikeCpp {
    pub(crate) async fn try_acquire_lease_like_cpp(&self) -> bool {
        self.owner.try_acquire_lease_like_cpp(self.lease_id).await
    }

    pub(crate) fn has_lease_like_cpp(&self) -> bool {
        self.owner.has_lease_like_cpp(self.lease_id)
    }

    pub(crate) fn owner_like_cpp(&self) -> &Arc<BattlePetAccountOwnerLikeCpp> {
        &self.owner
    }

    pub(crate) fn lease_id_like_cpp(&self) -> BattlePetLeaseIdLikeCpp {
        self.lease_id
    }
}

impl Drop for BattlePetAccountAttachmentLikeCpp {
    fn drop(&mut self) {
        {
            let mut process = self
                .owner
                .process_lease
                .lock()
                .expect("battle-pet process lease poisoned");
            if process.lease_holder == Some(self.lease_id) {
                process.lease_holder = None;
            }
            debug_assert!(
                process.attachments != 0,
                "battle-pet attachment count underflow"
            );
            process.attachments -= 1;
            if process.attachments == 0
                && process.active_operations == 0
                && process.lease_holder.is_none()
            {
                process.guard.take();
            }
        }
        self.owner.try_evict_if_idle_like_cpp();
    }
}

pub struct BattlePetAccountRegistryLikeCpp {
    persistence: Arc<dyn BattlePetPersistenceLikeCpp>,
    species_store: Arc<BattlePetSpeciesStore>,
    breed_quality_store: Arc<BattlePetBreedQualityStore>,
    breed_state_store: Arc<BattlePetBreedStateStore>,
    species_state_store: Arc<BattlePetSpeciesStateStore>,
    realm_id: u16,
    virtual_realm_address: u32,
    next_lease_id: AtomicU64,
    accounts: Arc<BattlePetAccountMapLikeCpp>,
}

impl BattlePetAccountRegistryLikeCpp {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        persistence: Arc<LoginBattlePetPersistenceLikeCpp>,
        species_store: Arc<BattlePetSpeciesStore>,
        breed_quality_store: Arc<BattlePetBreedQualityStore>,
        breed_state_store: Arc<BattlePetBreedStateStore>,
        species_state_store: Arc<BattlePetSpeciesStateStore>,
        realm_id: u16,
        virtual_realm_address: u32,
    ) -> Self {
        Self::new_with_persistence_like_cpp(
            persistence,
            species_store,
            breed_quality_store,
            breed_state_store,
            species_state_store,
            realm_id,
            virtual_realm_address,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new_with_persistence_like_cpp(
        persistence: Arc<dyn BattlePetPersistenceLikeCpp>,
        species_store: Arc<BattlePetSpeciesStore>,
        breed_quality_store: Arc<BattlePetBreedQualityStore>,
        breed_state_store: Arc<BattlePetBreedStateStore>,
        species_state_store: Arc<BattlePetSpeciesStateStore>,
        realm_id: u16,
        virtual_realm_address: u32,
    ) -> Self {
        Self {
            persistence,
            species_store,
            breed_quality_store,
            breed_state_store,
            species_state_store,
            realm_id,
            virtual_realm_address,
            next_lease_id: AtomicU64::new(1),
            accounts: Arc::new(DashMap::new()),
        }
    }

    pub async fn attach_like_cpp(
        &self,
        account_id: u32,
    ) -> Result<BattlePetAccountAttachmentLikeCpp, String> {
        if account_id == 0 {
            return Err("battle-pet journal requires a nonzero Battle.net account".to_string());
        }
        loop {
            let cell = self
                .accounts
                .entry(account_id)
                .or_insert_with(|| Arc::new(OnceCell::new()))
                .clone();
            let owner = cell
                .get_or_try_init(|| async {
                    let loaded = self
                        .persistence
                        .load_account(account_id, self.realm_id)
                        .await
                        .map_err(|error| format!("failed to load battle-pet account: {error:?}"))?;
                    Ok::<_, String>(Arc::new(
                        BattlePetAccountOwnerLikeCpp::from_loaded_like_cpp(
                            account_id,
                            self.realm_id,
                            self.virtual_realm_address,
                            Arc::clone(&self.persistence),
                            Arc::clone(&self.species_store),
                            Arc::clone(&self.breed_quality_store),
                            Arc::clone(&self.breed_state_store),
                            Arc::clone(&self.species_state_store),
                            loaded,
                        ),
                    ))
                })
                .await?
                .clone();
            owner.set_registry_identity_like_cpp(&self.accounts, &cell);

            let dashmap::mapref::entry::Entry::Occupied(entry) = self.accounts.entry(account_id)
            else {
                continue;
            };
            if !Arc::ptr_eq(entry.get(), &cell) {
                continue;
            }
            owner
                .process_lease
                .lock()
                .expect("battle-pet process lease poisoned")
                .attachments += 1;
            drop(entry);
            let lease_id =
                BattlePetLeaseIdLikeCpp(self.next_lease_id.fetch_add(1, Ordering::Relaxed));
            return Ok(BattlePetAccountAttachmentLikeCpp { owner, lease_id });
        }
    }

    /// Wait for cancellation-safe persistence workers after the network has
    /// stopped admitting sessions. The single deadline bounds orderly
    /// shutdown even when a database connection is unhealthy.
    pub async fn drain_like_cpp(&self, timeout: Duration) -> bool {
        let deadline = Instant::now() + timeout;
        let owners: Vec<_> = self
            .accounts
            .iter()
            .filter_map(|entry| entry.value().get().cloned())
            .collect();
        for owner in owners {
            let Some(remaining) = deadline.checked_duration_since(Instant::now()) else {
                return false;
            };
            if tokio::time::timeout(remaining, owner.wait_until_operations_drained_like_cpp())
                .await
                .is_err()
            {
                return false;
            }
        }
        true
    }
}

#[cfg(test)]
#[path = "battle_pet_account_tests.rs"]
mod tests;
