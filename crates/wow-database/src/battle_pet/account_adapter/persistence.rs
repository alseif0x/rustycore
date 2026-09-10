//! Persistence packets.
//!
//! Separated from account_adapter.rs under #701.

use super::*;

#[derive(Debug)]
pub struct LoginBattlePetPersistenceLikeCpp {
    db: Arc<LoginDatabase>,
    lock_broker: BattlePetAccountLockBrokerLikeCpp,
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
