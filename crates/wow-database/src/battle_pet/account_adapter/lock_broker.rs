//! Lock broker packets.
//!
//! Separated from account_adapter.rs under #701.

use super::*;

const BATTLE_PET_PROCESS_LEASE_VERIFY_INTERVAL_LIKE_CPP: Duration = Duration::from_secs(30);

#[derive(Debug, Clone)]
pub(super) struct BattlePetAccountLockBrokerLikeCpp {
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

pub(super) struct LoginBattlePetProcessLeaseLikeCpp {
    pub(super) lock_broker: BattlePetAccountLockBrokerLikeCpp,
    pub(super) lock_name: Option<String>,
    pub(super) broker_epoch: u64,
    pub(super) fence: u64,
}

pub(super) struct BattlePetBrokerLeaseReservationLikeCpp {
    pub(super) lock_broker: BattlePetAccountLockBrokerLikeCpp,
    pub(super) lock_name: Option<String>,
    pub(super) broker_epoch: u64,
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
    pub(super) fn spawn_like_cpp(db: Arc<LoginDatabase>) -> Self {
        let (commands, receiver) = mpsc::unbounded_channel();
        let epoch = Arc::new(AtomicU64::new(1));
        tokio::spawn(run_battle_pet_account_lock_broker_like_cpp(
            db,
            receiver,
            Arc::clone(&epoch),
        ));
        Self { commands, epoch }
    }

    pub(super) async fn acquire_like_cpp(
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

pub(super) fn battle_pet_account_lock_name_like_cpp(
    database_scope: &str,
    account_id: u32,
) -> String {
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
