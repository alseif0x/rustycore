//! MariaDB adapter for account-scoped battle-pet durability.

use std::collections::HashSet;
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};
use std::time::Duration;

use sqlx::{MySql, Row, Transaction};
use tokio::{
    sync::{mpsc, oneshot},
    time::MissedTickBehavior,
};
use wow_persistence::{
    BATTLE_PET_GUID_COUNTER_LIMIT_LIKE_CPP,
    BattlePetAccountPersistencePortLikeCpp as BattlePetPersistenceLikeCpp,
    BattlePetAddRequestKeyLikeCpp, BattlePetDeclinedNamesLikeCpp, BattlePetPersistenceErrorLikeCpp,
    BattlePetProcessLeaseLikeCpp, DurableBattlePetAddLikeCpp, DurableBattlePetAddReceiptLikeCpp,
    DurableBattlePetRowLikeCpp, DurableBattlePetSlotLikeCpp, LoadedBattlePetAccountLikeCpp,
    PersistBattlePetAddOutcomeLikeCpp, PersistenceFutureLikeCpp as PersistenceFuture,
};

use crate::{
    DatabaseError, LoginDatabase, LoginStatements, SqlResult, SqlTransaction,
    SqlTransactionCommitError,
};

const BATTLE_PET_PROCESS_LEASE_VERIFY_INTERVAL_LIKE_CPP: Duration = Duration::from_secs(30);

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
            let generator_limit = BATTLE_PET_GUID_COUNTER_LIMIT_LIKE_CPP;
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
    result: &SqlResult,
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
            Some(genitive) => Some(BattlePetDeclinedNamesLikeCpp {
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

        fn $result_fn(result: &SqlResult, column: usize) -> Option<$unsigned> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signed_column_casts_preserve_non_negative_values_like_cpp() {
        assert_eq!(battle_pet_column_i64_as_u64_like_cpp(0, 0).unwrap(), 0);
        assert_eq!(battle_pet_column_i64_as_u64_like_cpp(42, 0).unwrap(), 42);
        assert_eq!(
            battle_pet_column_i64_as_u64_like_cpp(i64::MAX, 0).unwrap(),
            i64::MAX as u64
        );
        assert_eq!(battle_pet_column_i32_as_u32_like_cpp(70, 1).unwrap(), 70);
        assert_eq!(battle_pet_column_i16_as_u16_like_cpp(25, 2).unwrap(), 25);
        assert_eq!(battle_pet_column_i8_as_u8_like_cpp(3, 7).unwrap(), 3);
    }

    #[test]
    fn signed_column_casts_reject_negative_values_like_cpp() {
        for result in [
            battle_pet_column_i64_as_u64_like_cpp(-1, 0),
            battle_pet_column_i64_as_u64_like_cpp(i64::MIN, 0),
        ] {
            let error = result.unwrap_err();
            assert!(
                matches!(error, BattlePetPersistenceErrorLikeCpp::Database(ref message)
                    if message.contains("negative value") && message.contains("column 0")),
                "unexpected error: {error:?}"
            );
        }
        assert!(battle_pet_column_i32_as_u32_like_cpp(-1, 1).is_err());
        assert!(battle_pet_column_i16_as_u16_like_cpp(-1, 2).is_err());
        assert!(battle_pet_column_i8_as_u8_like_cpp(-1, 7).is_err());
    }

    #[test]
    fn advisory_lock_names_are_scoped_to_one_login_database() {
        let first =
            battle_pet_account_lock_name_like_cpp("0123456789abcdef0123456789abcdef", u32::MAX);
        let second =
            battle_pet_account_lock_name_like_cpp("fedcba9876543210fedcba9876543210", u32::MAX);
        assert_ne!(first, second);
        assert!(first.len() <= 64);
        assert!(second.len() <= 64);
    }
}
