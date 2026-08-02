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
    Arc, Mutex,
    atomic::{AtomicU64, AtomicUsize, Ordering},
};
use std::time::{Duration, Instant};

use dashmap::DashMap;
use sqlx::{MySql, Row, Transaction};
use tokio::sync::{Notify, OnceCell, watch};
use wow_core::{ObjectGuid, ObjectGuidGenerator, guid::HighGuid};
use wow_data::{
    BATTLE_PET_SPECIES_FLAG_LEGACY_ACCOUNT_UNIQUE_LIKE_CPP,
    BATTLE_PET_SPECIES_FLAG_NOT_ACCOUNT_WIDE_LIKE_CPP, BATTLE_PET_SPECIES_FLAG_WELL_KNOWN_LIKE_CPP,
    BattlePetBreedQualityStore, BattlePetBreedStateStore, BattlePetSpeciesStateStore,
    BattlePetSpeciesStore, calculate_battle_pet_stats_like_cpp,
};
use wow_database::{
    DatabaseError, LoginDatabase, LoginStatements, PreparedStatement, SqlTransaction,
    SqlTransactionCommitError,
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
    pub pet: DurableBattlePetRowLikeCpp,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DurableBattlePetAddReceiptLikeCpp {
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
    GuidCollision,
    DuplicateRequest,
}

pub(crate) trait BattlePetPersistenceLikeCpp: Send + Sync {
    fn load_account<'a>(
        &'a self,
        account_id: u32,
        realm_id: u16,
    ) -> PersistenceFuture<
        'a,
        Result<LoadedBattlePetAccountLikeCpp, BattlePetPersistenceErrorLikeCpp>,
    >;

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
        pet: DurableBattlePetRowLikeCpp,
    ) -> PersistenceFuture<'a, Result<(), BattlePetPersistenceErrorLikeCpp>>;

    fn delete_pet<'a>(
        &'a self,
        account_id: u32,
        pet_guid_counter: u64,
        slots: Vec<DurableBattlePetSlotLikeCpp>,
    ) -> PersistenceFuture<'a, Result<(), BattlePetPersistenceErrorLikeCpp>>;

    fn replace_slots<'a>(
        &'a self,
        account_id: u32,
        slots: Vec<DurableBattlePetSlotLikeCpp>,
    ) -> PersistenceFuture<'a, Result<(), BattlePetPersistenceErrorLikeCpp>>;
}

#[derive(Debug)]
pub struct LoginBattlePetPersistenceLikeCpp {
    db: Arc<LoginDatabase>,
}

impl LoginBattlePetPersistenceLikeCpp {
    pub fn new(db: Arc<LoginDatabase>) -> Self {
        Self { db }
    }

    async fn find_request_like_cpp(
        &self,
        account_id: u32,
        request_key: BattlePetAddRequestKeyLikeCpp,
    ) -> Result<Option<(DurableBattlePetRowLikeCpp, bool)>, BattlePetPersistenceErrorLikeCpp> {
        let mut stmt = self.db.prepare(LoginStatements::SEL_BATTLE_PET_ADD_REQUEST);
        stmt.set_u32(0, account_id);
        stmt.set_bytes(1, request_key.as_bytes().to_vec());
        let result = self
            .db
            .query(&stmt)
            .await
            .map_err(database_error_like_cpp)?;
        if result.is_empty() {
            return Ok(None);
        }
        let still_present: bool = result.try_read(12).ok_or_else(|| {
            BattlePetPersistenceErrorLikeCpp::Database(
                "could not decode battle-pet request live-row marker".to_string(),
            )
        })?;
        Ok(Some((
            durable_pet_from_result_like_cpp(&result)?,
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

    fn insert_pet_idempotently<'a>(
        &'a self,
        request: DurableBattlePetAddLikeCpp,
    ) -> PersistenceFuture<
        'a,
        Result<PersistBattlePetAddOutcomeLikeCpp, BattlePetPersistenceErrorLikeCpp>,
    > {
        Box::pin(async move {
            if let Some((existing, still_present)) = self
                .find_request_like_cpp(request.account_id, request.request_key)
                .await?
            {
                return if add_request_matches_like_cpp(&request.pet, &existing) {
                    Ok(PersistBattlePetAddOutcomeLikeCpp::Replayed {
                        pet: existing,
                        still_present,
                    })
                } else {
                    Err(BattlePetPersistenceErrorLikeCpp::DuplicateRequest)
                };
            }

            let mut tx = SqlTransaction::new();
            tx.append(insert_pet_statement_like_cpp(&self.db, &request));
            tx.append(insert_add_request_statement_like_cpp(&self.db, &request));
            match tx.commit_with_outcome_like_cpp(self.db.pool()).await {
                Ok(()) => Ok(PersistBattlePetAddOutcomeLikeCpp::Inserted),
                Err(SqlTransactionCommitError::CommitOutcomeUnknown(_)) => {
                    match self
                        .find_request_like_cpp(request.account_id, request.request_key)
                        .await?
                    {
                        Some((existing, still_present))
                            if add_request_matches_like_cpp(&request.pet, &existing) =>
                        {
                            if still_present {
                                Ok(PersistBattlePetAddOutcomeLikeCpp::Inserted)
                            } else {
                                Err(BattlePetPersistenceErrorLikeCpp::DuplicateRequest)
                            }
                        }
                        Some(_) => Err(BattlePetPersistenceErrorLikeCpp::DuplicateRequest),
                        None => Err(BattlePetPersistenceErrorLikeCpp::Database(
                            "battle-pet insert COMMIT outcome could not be reconciled".to_string(),
                        )),
                    }
                }
                Err(SqlTransactionCommitError::DefinitelyRolledBack(error)) => {
                    if is_duplicate_key_like_cpp(&error) {
                        if let Some((existing, still_present)) = self
                            .find_request_like_cpp(request.account_id, request.request_key)
                            .await?
                        {
                            return if add_request_matches_like_cpp(&request.pet, &existing) {
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
                    Err(database_error_like_cpp(error))
                }
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
            let Some((requested_pet, still_present)) =
                self.find_request_like_cpp(account_id, request_key).await?
            else {
                return Ok(None);
            };
            let current_pet = if still_present {
                self.find_live_pet_like_cpp(account_id, requested_pet.guid_counter)
                    .await?
            } else {
                None
            };
            Ok(Some(DurableBattlePetAddReceiptLikeCpp {
                requested_pet,
                current_pet,
            }))
        })
    }

    fn update_pet<'a>(
        &'a self,
        account_id: u32,
        pet: DurableBattlePetRowLikeCpp,
    ) -> PersistenceFuture<'a, Result<(), BattlePetPersistenceErrorLikeCpp>> {
        Box::pin(async move {
            let mut tx = SqlTransaction::new();
            let mut stmt = self.db.prepare(LoginStatements::UPD_BATTLE_PETS);
            stmt.set_u16(0, pet.level);
            stmt.set_u16(1, pet.exp);
            stmt.set_u32(2, pet.health);
            stmt.set_u8(3, pet.quality);
            stmt.set_u16(4, pet.flags);
            stmt.set_string(5, pet.name.clone());
            stmt.set_i64(6, pet.name_timestamp);
            stmt.set_u32(7, account_id);
            stmt.set_u64(8, pet.guid_counter);
            tx.append_expect_rows_affected(stmt, 1);
            let mut delete_declined = self
                .db
                .prepare(LoginStatements::DEL_BATTLE_PET_DECLINED_NAME);
            delete_declined.set_u64(0, pet.guid_counter);
            tx.append(delete_declined);
            if let Some(declined) = &pet.declined_names {
                let mut insert = self
                    .db
                    .prepare(LoginStatements::INS_BATTLE_PET_DECLINED_NAME);
                insert.set_u64(0, pet.guid_counter);
                for (index, name) in declined.names.iter().enumerate() {
                    insert.set_string(index + 1, name.clone());
                }
                tx.append(insert);
            }
            match tx.commit_with_outcome_like_cpp(self.db.pool()).await {
                Ok(()) => Ok(()),
                Err(SqlTransactionCommitError::CommitOutcomeUnknown(_)) => {
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
                Err(SqlTransactionCommitError::DefinitelyRolledBack(error)) => {
                    Err(database_error_like_cpp(error))
                }
            }
        })
    }

    fn delete_pet<'a>(
        &'a self,
        account_id: u32,
        pet_guid_counter: u64,
        slots: Vec<DurableBattlePetSlotLikeCpp>,
    ) -> PersistenceFuture<'a, Result<(), BattlePetPersistenceErrorLikeCpp>> {
        Box::pin(async move {
            let mut tx = SqlTransaction::new();
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
        slots: Vec<DurableBattlePetSlotLikeCpp>,
    ) -> PersistenceFuture<'a, Result<(), BattlePetPersistenceErrorLikeCpp>> {
        Box::pin(async move {
            let mut tx = SqlTransaction::new();
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
            let counter: u64 = row
                .try_get(1)
                .map_err(|error| BattlePetPersistenceErrorLikeCpp::Database(error.to_string()))?;
            Ok(DurableBattlePetSlotLikeCpp {
                index: row.try_get(0).map_err(|error| {
                    BattlePetPersistenceErrorLikeCpp::Database(error.to_string())
                })?,
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
) -> Result<DurableBattlePetRowLikeCpp, BattlePetPersistenceErrorLikeCpp> {
    macro_rules! required {
        ($column:expr) => {
            result.try_read($column).ok_or_else(|| {
                BattlePetPersistenceErrorLikeCpp::Database(format!(
                    "could not decode battle-pet result column {}",
                    $column
                ))
            })?
        };
    }
    Ok(DurableBattlePetRowLikeCpp {
        guid_counter: required!(0),
        species: required!(1),
        breed: required!(2),
        display_id: required!(3),
        level: required!(4),
        exp: required!(5),
        health: required!(6),
        quality: required!(7),
        flags: required!(8),
        name: required!(9),
        name_timestamp: required!(10),
        owner_guid_counter: if result.is_null(11) {
            None
        } else {
            Some(required!(11))
        },
        declined_names: None,
    })
}

fn durable_pet_from_row_like_cpp(
    row: &sqlx::mysql::MySqlRow,
) -> Result<DurableBattlePetRowLikeCpp, BattlePetPersistenceErrorLikeCpp> {
    let genitive: Option<String> = row.try_get(12).map_err(row_decode_error_like_cpp)?;
    Ok(DurableBattlePetRowLikeCpp {
        guid_counter: row.try_get(0).map_err(row_decode_error_like_cpp)?,
        species: row.try_get(1).map_err(row_decode_error_like_cpp)?,
        breed: row.try_get(2).map_err(row_decode_error_like_cpp)?,
        display_id: row.try_get(3).map_err(row_decode_error_like_cpp)?,
        level: row.try_get(4).map_err(row_decode_error_like_cpp)?,
        exp: row.try_get(5).map_err(row_decode_error_like_cpp)?,
        health: row.try_get(6).map_err(row_decode_error_like_cpp)?,
        quality: row.try_get(7).map_err(row_decode_error_like_cpp)?,
        flags: row.try_get(8).map_err(row_decode_error_like_cpp)?,
        name: row.try_get(9).map_err(row_decode_error_like_cpp)?,
        name_timestamp: row.try_get(10).map_err(row_decode_error_like_cpp)?,
        owner_guid_counter: row.try_get(11).map_err(row_decode_error_like_cpp)?,
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

fn insert_pet_statement_like_cpp(
    db: &LoginDatabase,
    request: &DurableBattlePetAddLikeCpp,
) -> PreparedStatement {
    let pet = &request.pet;
    let mut stmt = db.prepare(LoginStatements::INS_BATTLE_PETS);
    stmt.set_u64(0, pet.guid_counter);
    stmt.set_u32(1, request.account_id);
    stmt.set_u32(2, pet.species);
    stmt.set_u16(3, pet.breed);
    stmt.set_u32(4, pet.display_id);
    stmt.set_u16(5, pet.level);
    stmt.set_u16(6, pet.exp);
    stmt.set_u32(7, pet.health);
    stmt.set_u8(8, pet.quality);
    stmt.set_u16(9, pet.flags);
    stmt.set_string(10, pet.name.clone());
    stmt.set_i64(11, pet.name_timestamp);
    match pet.owner_guid_counter {
        Some(owner) => stmt.set_u64(12, owner),
        None => stmt.set_null(12),
    }
    if pet.owner_guid_counter.is_some() {
        stmt.set_u16(13, request.realm_id);
    } else {
        stmt.set_null(13);
    }
    stmt
}

fn insert_add_request_statement_like_cpp(
    db: &LoginDatabase,
    request: &DurableBattlePetAddLikeCpp,
) -> PreparedStatement {
    let pet = &request.pet;
    let mut stmt = db.prepare(LoginStatements::INS_BATTLE_PET_ADD_REQUEST);
    stmt.set_u32(0, request.account_id);
    stmt.set_bytes(1, request.request_key.as_bytes().to_vec());
    stmt.set_u64(2, pet.guid_counter);
    stmt.set_u32(3, pet.species);
    stmt.set_u16(4, pet.breed);
    stmt.set_u32(5, pet.display_id);
    stmt.set_u16(6, pet.level);
    stmt.set_u16(7, pet.exp);
    stmt.set_u32(8, pet.health);
    stmt.set_u8(9, pet.quality);
    stmt.set_u16(10, pet.flags);
    stmt.set_string(11, pet.name.clone());
    stmt.set_i64(12, pet.name_timestamp);
    match pet.owner_guid_counter {
        Some(owner) => stmt.set_u64(13, owner),
        None => stmt.set_null(13),
    }
    stmt
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

fn is_duplicate_key_like_cpp(error: &DatabaseError) -> bool {
    matches!(
        error,
        DatabaseError::Query(sqlx::Error::Database(database_error))
            if database_error.code().as_deref() == Some("1062")
    )
}

fn validate_add_lease_like_cpp(
    state: &BattlePetAccountStateLikeCpp,
    lease_id: BattlePetLeaseIdLikeCpp,
) -> Result<(), BattlePetAddFailureLikeCpp> {
    if state.lease_holder == Some(lease_id) {
        return Ok(());
    }
    Err(if state.lease_holder.is_some() {
        BattlePetAddFailureLikeCpp::JournalLocked
    } else {
        BattlePetAddFailureLikeCpp::MissingAuthority
    })
}

fn add_persistence_error_like_cpp(
    error: BattlePetPersistenceErrorLikeCpp,
) -> BattlePetAddFailureLikeCpp {
    match error {
        BattlePetPersistenceErrorLikeCpp::Database(error) => {
            BattlePetAddFailureLikeCpp::DatabaseFailure(error)
        }
        BattlePetPersistenceErrorLikeCpp::GuidCollision => {
            BattlePetAddFailureLikeCpp::GuidCollision
        }
        BattlePetPersistenceErrorLikeCpp::DuplicateRequest => {
            BattlePetAddFailureLikeCpp::DuplicateRequest
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
    lease_holder: Option<BattlePetLeaseIdLikeCpp>,
    pending_adds: HashMap<BattlePetAddRequestKeyLikeCpp, PendingAddLikeCpp>,
    completed_adds:
        HashMap<BattlePetAddRequestKeyLikeCpp, (ObjectGuid, DurableBattlePetRowLikeCpp)>,
    pending_pet_mutations: HashSet<ObjectGuid>,
    slots_pending: bool,
}

pub(crate) struct BattlePetAccountOwnerLikeCpp {
    account_id: u32,
    realm_id: u16,
    virtual_realm_address: u32,
    persistence: Arc<dyn BattlePetPersistenceLikeCpp>,
    guid_generator: Arc<ObjectGuidGenerator>,
    species_store: Arc<BattlePetSpeciesStore>,
    breed_quality_store: Arc<BattlePetBreedQualityStore>,
    breed_state_store: Arc<BattlePetBreedStateStore>,
    species_state_store: Arc<BattlePetSpeciesStateStore>,
    state: Mutex<BattlePetAccountStateLikeCpp>,
    active_operations: AtomicUsize,
    operations_drained: Notify,
}

impl BattlePetAccountOwnerLikeCpp {
    fn from_loaded_like_cpp(
        account_id: u32,
        realm_id: u16,
        virtual_realm_address: u32,
        persistence: Arc<dyn BattlePetPersistenceLikeCpp>,
        guid_generator: Arc<ObjectGuidGenerator>,
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
            guid_generator,
            species_store,
            breed_quality_store,
            breed_state_store,
            species_state_store,
            state: Mutex::new(BattlePetAccountStateLikeCpp {
                pets,
                slots,
                lease_holder: None,
                pending_adds: HashMap::new(),
                completed_adds: HashMap::new(),
                pending_pet_mutations: HashSet::new(),
                slots_pending: false,
            }),
            active_operations: AtomicUsize::new(0),
            operations_drained: Notify::new(),
        }
    }

    fn begin_operation_like_cpp(self: &Arc<Self>) -> BattlePetOperationGuardLikeCpp {
        self.active_operations.fetch_add(1, Ordering::AcqRel);
        BattlePetOperationGuardLikeCpp {
            owner: Arc::clone(self),
        }
    }

    async fn wait_until_operations_drained_like_cpp(&self) {
        loop {
            let notified = self.operations_drained.notified();
            if self.active_operations.load(Ordering::Acquire) == 0 {
                return;
            }
            notified.await;
        }
    }

    fn try_acquire_lease_like_cpp(&self, lease_id: BattlePetLeaseIdLikeCpp) -> bool {
        let mut state = self
            .state
            .lock()
            .expect("battle-pet account state poisoned");
        match state.lease_holder {
            None => {
                state.lease_holder = Some(lease_id);
                true
            }
            Some(holder) => holder == lease_id,
        }
    }

    fn release_lease_like_cpp(&self, lease_id: BattlePetLeaseIdLikeCpp) {
        let mut state = self
            .state
            .lock()
            .expect("battle-pet account state poisoned");
        if state.lease_holder == Some(lease_id) {
            state.lease_holder = None;
        }
    }

    fn has_lease_like_cpp(&self, lease_id: BattlePetLeaseIdLikeCpp) -> bool {
        self.state
            .lock()
            .expect("battle-pet account state poisoned")
            .lease_holder
            == Some(lease_id)
    }

    pub(crate) fn journal_like_cpp(
        &self,
        lease_id: BattlePetLeaseIdLikeCpp,
        player_guid: Option<ObjectGuid>,
    ) -> BattlePetJournal {
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
            has_journal_lock: state.lease_holder == Some(lease_id),
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

    pub(crate) async fn try_add_pet_like_cpp(
        self: &Arc<Self>,
        lease_id: BattlePetLeaseIdLikeCpp,
        request: BattlePetAddRequestLikeCpp,
    ) -> Result<BattlePetAddOutcomeLikeCpp, BattlePetAddFailureLikeCpp> {
        {
            let state = self
                .state
                .lock()
                .expect("battle-pet account state poisoned");
            validate_add_lease_like_cpp(&state, lease_id)?;
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
            let mut state = self
                .state
                .lock()
                .expect("battle-pet account state poisoned");
            validate_add_lease_like_cpp(&state, lease_id)?;
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

        loop {
            let wait = {
                let mut state = self
                    .state
                    .lock()
                    .expect("battle-pet account state poisoned");
                validate_add_lease_like_cpp(&state, lease_id)?;
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
                    Some(pending.completion.subscribe())
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
                    None
                }
            };
            if let Some(mut wait) = wait {
                if !*wait.borrow() {
                    let _ = wait.changed().await;
                }
                continue;
            }
            break;
        }

        let row_result = u64::try_from(self.guid_generator.generate())
            .map_err(|_| BattlePetAddFailureLikeCpp::GuidCollision)
            .and_then(|counter| self.durable_new_pet_like_cpp(counter, &request));
        let row = match row_result {
            Ok(row) => row,
            Err(error) => {
                let mut state = self
                    .state
                    .lock()
                    .expect("battle-pet account state poisoned");
                if let Some(pending) = state.pending_adds.remove(&request.request_key) {
                    let _ = pending.completion.send(true);
                }
                return Err(error);
            }
        };
        let owner = Arc::clone(self);
        let operation_guard = self.begin_operation_like_cpp();
        tokio::spawn(async move {
            let _operation_guard = operation_guard;
            owner.finish_add_pet_like_cpp(request, row).await
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
        row: DurableBattlePetRowLikeCpp,
    ) -> Result<BattlePetAddOutcomeLikeCpp, BattlePetAddFailureLikeCpp> {
        let persistence_result = self
            .persistence
            .insert_pet_idempotently(DurableBattlePetAddLikeCpp {
                account_id: self.account_id,
                realm_id: self.realm_id,
                request_key: request.request_key,
                pet: row.clone(),
            })
            .await;

        let mut state = self
            .state
            .lock()
            .expect("battle-pet account state poisoned");
        let pending = state
            .pending_adds
            .remove(&request.request_key)
            .expect("battle-pet reservation disappeared before persistence completed");
        let result = match persistence_result {
            Ok(outcome) => {
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
            Err(BattlePetPersistenceErrorLikeCpp::DuplicateRequest) => {
                Err(BattlePetAddFailureLikeCpp::DuplicateRequest)
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
        let mut changed = {
            let mut state = self
                .state
                .lock()
                .expect("battle-pet account state poisoned");
            if state.lease_holder != Some(lease_id) {
                return Err(if state.lease_holder.is_some() {
                    BattlePetMutationFailureLikeCpp::JournalLocked
                } else {
                    BattlePetMutationFailureLikeCpp::MissingAuthority
                });
            }
            if !state.pending_pet_mutations.insert(pet_guid) {
                return Err(BattlePetMutationFailureLikeCpp::Busy);
            }
            let Some(current) = state.pets.get(&pet_guid).cloned() else {
                state.pending_pet_mutations.remove(&pet_guid);
                return Err(BattlePetMutationFailureLikeCpp::UnknownPet);
            };
            current
        };
        let outcome = mutation(&mut changed);
        changed.save_info = RepresentedBattlePetSaveInfoLikeCpp::Unchanged;
        let owner = Arc::clone(self);
        let operation_guard = self.begin_operation_like_cpp();
        tokio::spawn(async move {
            let _operation_guard = operation_guard;
            owner
                .finish_mutate_pet_like_cpp(pet_guid, outcome, changed)
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
    ) -> Result<(R, BattlePetJournalPet), BattlePetMutationFailureLikeCpp> {
        let durable = durable_from_pet_like_cpp(pet_guid, &changed);
        let persistence = self.persistence.update_pet(self.account_id, durable).await;
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
        let changed_slots = {
            let mut state = self
                .state
                .lock()
                .expect("battle-pet account state poisoned");
            if state.lease_holder != Some(lease_id) {
                return Err(if state.lease_holder.is_some() {
                    BattlePetMutationFailureLikeCpp::JournalLocked
                } else {
                    BattlePetMutationFailureLikeCpp::MissingAuthority
                });
            }
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
            changed
        };
        let owner = Arc::clone(self);
        let operation_guard = self.begin_operation_like_cpp();
        tokio::spawn(async move {
            let _operation_guard = operation_guard;
            owner
                .finish_remove_pet_like_cpp(pet_guid, changed_slots)
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
    ) -> Result<(), BattlePetMutationFailureLikeCpp> {
        let persistence = self
            .persistence
            .delete_pet(
                self.account_id,
                pet_guid.counter() as u64,
                changed_slots.iter().map(durable_slot_like_cpp).collect(),
            )
            .await;
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
        let changed = {
            let mut state = self
                .state
                .lock()
                .expect("battle-pet account state poisoned");
            if state.lease_holder != Some(lease_id) {
                return Err(if state.lease_holder.is_some() {
                    BattlePetMutationFailureLikeCpp::JournalLocked
                } else {
                    BattlePetMutationFailureLikeCpp::MissingAuthority
                });
            }
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
            changed
        };
        let owner = Arc::clone(self);
        let operation_guard = self.begin_operation_like_cpp();
        tokio::spawn(async move {
            let _operation_guard = operation_guard;
            owner.finish_set_slot_like_cpp(slot_index, changed).await
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
    ) -> Result<BattlePetJournalSlot, BattlePetMutationFailureLikeCpp> {
        let durable = changed.iter().map(durable_slot_like_cpp).collect();
        let persistence = self
            .persistence
            .replace_slots(self.account_id, durable)
            .await;
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
    BattlePetMutationFailureLikeCpp::DatabaseFailure(match error {
        BattlePetPersistenceErrorLikeCpp::Database(error) => error,
        BattlePetPersistenceErrorLikeCpp::GuidCollision => "unexpected GUID collision".to_string(),
        BattlePetPersistenceErrorLikeCpp::DuplicateRequest => {
            "unexpected duplicate request".to_string()
        }
    })
}

struct BattlePetOperationGuardLikeCpp {
    owner: Arc<BattlePetAccountOwnerLikeCpp>,
}

impl Drop for BattlePetOperationGuardLikeCpp {
    fn drop(&mut self) {
        if self.owner.active_operations.fetch_sub(1, Ordering::AcqRel) == 1 {
            self.owner.operations_drained.notify_waiters();
        }
    }
}

pub struct BattlePetAccountAttachmentLikeCpp {
    owner: Arc<BattlePetAccountOwnerLikeCpp>,
    lease_id: BattlePetLeaseIdLikeCpp,
}

impl BattlePetAccountAttachmentLikeCpp {
    pub(crate) fn try_acquire_lease_like_cpp(&self) -> bool {
        self.owner.try_acquire_lease_like_cpp(self.lease_id)
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
        self.owner.release_lease_like_cpp(self.lease_id);
    }
}

pub struct BattlePetAccountRegistryLikeCpp {
    persistence: Arc<dyn BattlePetPersistenceLikeCpp>,
    guid_generator: Arc<ObjectGuidGenerator>,
    species_store: Arc<BattlePetSpeciesStore>,
    breed_quality_store: Arc<BattlePetBreedQualityStore>,
    breed_state_store: Arc<BattlePetBreedStateStore>,
    species_state_store: Arc<BattlePetSpeciesStateStore>,
    realm_id: u16,
    virtual_realm_address: u32,
    next_lease_id: AtomicU64,
    accounts: DashMap<u32, Arc<OnceCell<Arc<BattlePetAccountOwnerLikeCpp>>>>,
}

impl BattlePetAccountRegistryLikeCpp {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        persistence: Arc<LoginBattlePetPersistenceLikeCpp>,
        guid_generator: Arc<ObjectGuidGenerator>,
        species_store: Arc<BattlePetSpeciesStore>,
        breed_quality_store: Arc<BattlePetBreedQualityStore>,
        breed_state_store: Arc<BattlePetBreedStateStore>,
        species_state_store: Arc<BattlePetSpeciesStateStore>,
        realm_id: u16,
        virtual_realm_address: u32,
    ) -> Self {
        Self::new_with_persistence_like_cpp(
            persistence,
            guid_generator,
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
        guid_generator: Arc<ObjectGuidGenerator>,
        species_store: Arc<BattlePetSpeciesStore>,
        breed_quality_store: Arc<BattlePetBreedQualityStore>,
        breed_state_store: Arc<BattlePetBreedStateStore>,
        species_state_store: Arc<BattlePetSpeciesStateStore>,
        realm_id: u16,
        virtual_realm_address: u32,
    ) -> Self {
        Self {
            persistence,
            guid_generator,
            species_store,
            breed_quality_store,
            breed_state_store,
            species_state_store,
            realm_id,
            virtual_realm_address,
            next_lease_id: AtomicU64::new(1),
            accounts: DashMap::new(),
        }
    }

    pub async fn attach_like_cpp(
        &self,
        account_id: u32,
    ) -> Result<BattlePetAccountAttachmentLikeCpp, String> {
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
                        Arc::clone(&self.guid_generator),
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
        let lease_id = BattlePetLeaseIdLikeCpp(self.next_lease_id.fetch_add(1, Ordering::Relaxed));
        Ok(BattlePetAccountAttachmentLikeCpp { owner, lease_id })
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
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, AtomicUsize};
    use wow_data::{
        BATTLE_PET_STATE_STAT_POWER_LIKE_CPP, BATTLE_PET_STATE_STAT_SPEED_LIKE_CPP,
        BATTLE_PET_STATE_STAT_STAMINA_LIKE_CPP, BattlePetBreedQualityEntry,
        BattlePetBreedStateEntry, BattlePetSpeciesEntry, BattlePetSpeciesStateEntry,
    };

    #[derive(Default)]
    struct FakePersistenceStateLikeCpp {
        pets: Vec<DurableBattlePetRowLikeCpp>,
        slots: Vec<DurableBattlePetSlotLikeCpp>,
        receipts: HashMap<BattlePetAddRequestKeyLikeCpp, DurableBattlePetRowLikeCpp>,
    }

    #[derive(Default)]
    struct FakePersistenceLikeCpp {
        state: Mutex<FakePersistenceStateLikeCpp>,
        insert_calls: AtomicUsize,
        fail_next_insert: AtomicBool,
        fail_next_update: AtomicBool,
        fail_next_delete: AtomicBool,
        fail_next_slots: AtomicBool,
        block_next_insert: AtomicBool,
        insert_started: Notify,
        allow_insert: Notify,
    }

    impl BattlePetPersistenceLikeCpp for FakePersistenceLikeCpp {
        fn load_account<'a>(
            &'a self,
            _account_id: u32,
            _realm_id: u16,
        ) -> PersistenceFuture<
            'a,
            Result<LoadedBattlePetAccountLikeCpp, BattlePetPersistenceErrorLikeCpp>,
        > {
            Box::pin(async move {
                let state = self.state.lock().expect("fake persistence poisoned");
                Ok(LoadedBattlePetAccountLikeCpp {
                    pets: state.pets.clone(),
                    slots: state.slots.clone(),
                })
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
                self.insert_calls.fetch_add(1, Ordering::Relaxed);
                tokio::task::yield_now().await;
                if self.block_next_insert.swap(false, Ordering::AcqRel) {
                    self.insert_started.notify_one();
                    self.allow_insert.notified().await;
                }
                if self.fail_next_insert.swap(false, Ordering::AcqRel) {
                    return Err(BattlePetPersistenceErrorLikeCpp::Database(
                        "injected insert failure".to_string(),
                    ));
                }
                let mut state = self.state.lock().expect("fake persistence poisoned");
                if let Some(existing) = state.receipts.get(&request.request_key).cloned() {
                    let still_present = state
                        .pets
                        .iter()
                        .any(|pet| pet.guid_counter == existing.guid_counter);
                    return if add_request_matches_like_cpp(&request.pet, &existing) {
                        Ok(PersistBattlePetAddOutcomeLikeCpp::Replayed {
                            pet: existing,
                            still_present,
                        })
                    } else {
                        Err(BattlePetPersistenceErrorLikeCpp::DuplicateRequest)
                    };
                }
                if state
                    .pets
                    .iter()
                    .any(|pet| pet.guid_counter == request.pet.guid_counter)
                {
                    return Err(BattlePetPersistenceErrorLikeCpp::GuidCollision);
                }
                state.pets.push(request.pet.clone());
                state.receipts.insert(request.request_key, request.pet);
                Ok(PersistBattlePetAddOutcomeLikeCpp::Inserted)
            })
        }

        fn lookup_add_request<'a>(
            &'a self,
            _account_id: u32,
            request_key: BattlePetAddRequestKeyLikeCpp,
        ) -> PersistenceFuture<
            'a,
            Result<Option<DurableBattlePetAddReceiptLikeCpp>, BattlePetPersistenceErrorLikeCpp>,
        > {
            Box::pin(async move {
                let state = self.state.lock().expect("fake persistence poisoned");
                Ok(state.receipts.get(&request_key).cloned().map(|pet| {
                    let current_pet = state
                        .pets
                        .iter()
                        .find(|existing| existing.guid_counter == pet.guid_counter)
                        .cloned();
                    DurableBattlePetAddReceiptLikeCpp {
                        requested_pet: pet,
                        current_pet,
                    }
                }))
            })
        }

        fn update_pet<'a>(
            &'a self,
            _account_id: u32,
            pet: DurableBattlePetRowLikeCpp,
        ) -> PersistenceFuture<'a, Result<(), BattlePetPersistenceErrorLikeCpp>> {
            Box::pin(async move {
                if self.fail_next_update.swap(false, Ordering::AcqRel) {
                    return Err(BattlePetPersistenceErrorLikeCpp::Database(
                        "injected update failure".to_string(),
                    ));
                }
                let mut state = self.state.lock().expect("fake persistence poisoned");
                let Some(existing) = state
                    .pets
                    .iter_mut()
                    .find(|existing| existing.guid_counter == pet.guid_counter)
                else {
                    return Err(BattlePetPersistenceErrorLikeCpp::Database(
                        "unknown fake pet".to_string(),
                    ));
                };
                *existing = pet;
                Ok(())
            })
        }

        fn delete_pet<'a>(
            &'a self,
            _account_id: u32,
            pet_guid_counter: u64,
            slots: Vec<DurableBattlePetSlotLikeCpp>,
        ) -> PersistenceFuture<'a, Result<(), BattlePetPersistenceErrorLikeCpp>> {
            Box::pin(async move {
                if self.fail_next_delete.swap(false, Ordering::AcqRel) {
                    return Err(BattlePetPersistenceErrorLikeCpp::Database(
                        "injected delete failure".to_string(),
                    ));
                }
                let mut state = self.state.lock().expect("fake persistence poisoned");
                state
                    .pets
                    .retain(|pet| pet.guid_counter != pet_guid_counter);
                state.slots = slots;
                Ok(())
            })
        }

        fn replace_slots<'a>(
            &'a self,
            _account_id: u32,
            slots: Vec<DurableBattlePetSlotLikeCpp>,
        ) -> PersistenceFuture<'a, Result<(), BattlePetPersistenceErrorLikeCpp>> {
            Box::pin(async move {
                if self.fail_next_slots.swap(false, Ordering::AcqRel) {
                    return Err(BattlePetPersistenceErrorLikeCpp::Database(
                        "injected slot failure".to_string(),
                    ));
                }
                self.state.lock().expect("fake persistence poisoned").slots = slots;
                Ok(())
            })
        }
    }

    fn stores_like_cpp(
        species_flags: i32,
    ) -> (
        Arc<BattlePetSpeciesStore>,
        Arc<BattlePetBreedQualityStore>,
        Arc<BattlePetBreedStateStore>,
        Arc<BattlePetSpeciesStateStore>,
    ) {
        let species = Arc::new(BattlePetSpeciesStore::from_entries([
            BattlePetSpeciesEntry {
                id: 11,
                description: String::new(),
                source_text: String::new(),
                creature_id: 99,
                summon_spell_id: 0,
                icon_file_data_id: 0,
                pet_type_enum: 0,
                flags: BATTLE_PET_SPECIES_FLAG_WELL_KNOWN_LIKE_CPP | species_flags,
                source_type_enum: 0,
                card_ui_model_scene_id: 0,
                loadout_ui_model_scene_id: 0,
            },
            BattlePetSpeciesEntry {
                id: 12,
                description: String::new(),
                source_text: String::new(),
                creature_id: 100,
                summon_spell_id: 0,
                icon_file_data_id: 0,
                pet_type_enum: 0,
                flags: BATTLE_PET_SPECIES_FLAG_WELL_KNOWN_LIKE_CPP
                    | BATTLE_PET_SPECIES_FLAG_LEGACY_ACCOUNT_UNIQUE_LIKE_CPP,
                source_type_enum: 0,
                card_ui_model_scene_id: 0,
                loadout_ui_model_scene_id: 0,
            },
        ]));
        let qualities = Arc::new(BattlePetBreedQualityStore::from_entries([
            BattlePetBreedQualityEntry {
                id: 1,
                state_multiplier: 1.0,
                quality_enum: 1,
            },
        ]));
        let breed_states = Arc::new(BattlePetBreedStateStore::from_entries([
            BattlePetBreedStateEntry {
                id: 1,
                battle_pet_state_id: BATTLE_PET_STATE_STAT_STAMINA_LIKE_CPP,
                value: 500,
                battle_pet_breed_id: 7,
            },
            BattlePetBreedStateEntry {
                id: 2,
                battle_pet_state_id: BATTLE_PET_STATE_STAT_POWER_LIKE_CPP,
                value: 300,
                battle_pet_breed_id: 7,
            },
            BattlePetBreedStateEntry {
                id: 3,
                battle_pet_state_id: BATTLE_PET_STATE_STAT_SPEED_LIKE_CPP,
                value: 200,
                battle_pet_breed_id: 7,
            },
        ]));
        let species_states = Arc::new(BattlePetSpeciesStateStore::from_entries([
            BattlePetSpeciesStateEntry {
                id: 1,
                battle_pet_state_id: BATTLE_PET_STATE_STAT_STAMINA_LIKE_CPP,
                value: 100,
                battle_pet_species_id: 11,
            },
        ]));
        (species, qualities, breed_states, species_states)
    }

    fn registry_like_cpp(
        persistence: Arc<FakePersistenceLikeCpp>,
        species_flags: i32,
        next_guid: i64,
    ) -> BattlePetAccountRegistryLikeCpp {
        let (species, qualities, breed_states, species_states) = stores_like_cpp(species_flags);
        BattlePetAccountRegistryLikeCpp::new_with_persistence_like_cpp(
            persistence,
            Arc::new(ObjectGuidGenerator::new(HighGuid::BattlePet, next_guid)),
            species,
            qualities,
            breed_states,
            species_states,
            7,
            0x0102_0007,
        )
    }

    fn add_request_like_cpp(key: u8, species: u32, owner: u64) -> BattlePetAddRequestLikeCpp {
        let mut request_key = [0; 16];
        request_key[0] = key;
        BattlePetAddRequestLikeCpp {
            request_key: BattlePetAddRequestKeyLikeCpp::from_bytes(request_key),
            species,
            display_id: 123,
            breed: 7,
            quality: 1,
            level: 1,
            owner_guid: Some(ObjectGuid::create_player(7, owner as i64)),
        }
    }

    fn durable_pet_row_like_cpp(
        guid_counter: u64,
        species: u32,
        owner_guid_counter: Option<u64>,
    ) -> DurableBattlePetRowLikeCpp {
        DurableBattlePetRowLikeCpp {
            guid_counter,
            species,
            breed: 7,
            display_id: 123,
            level: 1,
            exp: 0,
            health: 100,
            quality: 1,
            flags: 0,
            name: String::new(),
            name_timestamp: 0,
            owner_guid_counter,
            declined_names: None,
        }
    }

    #[test]
    fn source_item_guid_is_a_restart_stable_add_request_identity_like_cpp() {
        let first = ObjectGuid::create_item(7, 41);
        let second = ObjectGuid::create_item(7, 42);
        assert_eq!(
            BattlePetAddRequestKeyLikeCpp::from_source_item_guid_like_cpp(first)
                .expect("nonempty item guid")
                .as_bytes(),
            first.to_raw_bytes()
        );
        assert_ne!(
            BattlePetAddRequestKeyLikeCpp::from_source_item_guid_like_cpp(first),
            BattlePetAddRequestKeyLikeCpp::from_source_item_guid_like_cpp(second)
        );
        assert_eq!(
            BattlePetAddRequestKeyLikeCpp::from_source_item_guid_like_cpp(ObjectGuid::EMPTY),
            None
        );
    }

    #[test]
    fn loaded_rows_apply_cpp_species_owner_capacity_and_slot_validation_like_cpp() {
        let (species, qualities, breed_states, species_states) =
            stores_like_cpp(BATTLE_PET_SPECIES_FLAG_NOT_ACCOUNT_WIDE_LIKE_CPP);
        let loaded = LoadedBattlePetAccountLikeCpp {
            pets: vec![
                durable_pet_row_like_cpp(1, 999, None),
                durable_pet_row_like_cpp(2, 11, None),
                durable_pet_row_like_cpp(3, 11, Some(100)),
                durable_pet_row_like_cpp(4, 11, Some(100)),
                durable_pet_row_like_cpp(5, 11, Some(100)),
                durable_pet_row_like_cpp(6, 11, Some(100)),
                durable_pet_row_like_cpp(7, 11, Some(200)),
                durable_pet_row_like_cpp(8, 12, Some(100)),
                durable_pet_row_like_cpp(9, 12, None),
                durable_pet_row_like_cpp(10, 12, None),
            ],
            slots: vec![
                DurableBattlePetSlotLikeCpp {
                    index: 0,
                    pet_guid_counter: Some(3),
                    locked: false,
                },
                DurableBattlePetSlotLikeCpp {
                    index: 1,
                    pet_guid_counter: Some(6),
                    locked: false,
                },
            ],
        };
        let owner = BattlePetAccountOwnerLikeCpp::from_loaded_like_cpp(
            77,
            7,
            0x0102_0007,
            Arc::new(FakePersistenceLikeCpp::default()),
            Arc::new(ObjectGuidGenerator::new(HighGuid::BattlePet, 100)),
            species,
            qualities,
            breed_states,
            species_states,
            loaded,
        );
        let state = owner.state.lock().expect("battle-pet account state");
        let accepted = state
            .pets
            .keys()
            .map(|guid| guid.counter() as u64)
            .collect::<HashSet<_>>();
        assert_eq!(accepted, HashSet::from([3, 4, 5, 7, 9]));
        assert_eq!(state.slots[0].pet_guid, Some(battle_pet_guid_like_cpp(3)));
        assert_eq!(state.slots[1].pet_guid, None);
    }

    #[tokio::test]
    async fn one_account_lease_and_pending_capacity_are_atomic_like_cpp() {
        let persistence = Arc::new(FakePersistenceLikeCpp::default());
        let registry = registry_like_cpp(Arc::clone(&persistence), 0, 100);
        let first = registry.attach_like_cpp(77).await.expect("first attach");
        let second = registry.attach_like_cpp(77).await.expect("second attach");
        assert!(first.try_acquire_lease_like_cpp());
        assert!(!second.try_acquire_lease_like_cpp());

        let owner = Arc::clone(first.owner_like_cpp());
        let lease = first.lease_id_like_cpp();
        assert!(matches!(
            owner
                .try_add_pet_like_cpp(second.lease_id_like_cpp(), add_request_like_cpp(99, 11, 1),)
                .await,
            Err(BattlePetAddFailureLikeCpp::JournalLocked)
        ));
        let (a, b, c, d) = tokio::join!(
            owner.try_add_pet_like_cpp(lease, add_request_like_cpp(1, 11, 1)),
            owner.try_add_pet_like_cpp(lease, add_request_like_cpp(2, 11, 1)),
            owner.try_add_pet_like_cpp(lease, add_request_like_cpp(3, 11, 1)),
            owner.try_add_pet_like_cpp(lease, add_request_like_cpp(4, 11, 1)),
        );
        let outcomes = [a, b, c, d];
        assert_eq!(outcomes.iter().filter(|result| result.is_ok()).count(), 3);
        assert_eq!(
            outcomes
                .iter()
                .filter(|result| matches!(result, Err(BattlePetAddFailureLikeCpp::Capacity)))
                .count(),
            1
        );
        let state = persistence.state.lock().expect("fake persistence poisoned");
        assert_eq!(state.pets.len(), 3);
        assert_eq!(
            state
                .pets
                .iter()
                .map(|pet| pet.guid_counter)
                .collect::<HashSet<_>>()
                .len(),
            3
        );
    }

    #[tokio::test]
    async fn concurrent_same_request_inserts_and_publishes_once_like_cpp() {
        let persistence = Arc::new(FakePersistenceLikeCpp::default());
        let registry = registry_like_cpp(Arc::clone(&persistence), 0, 200);
        let attachment = registry.attach_like_cpp(77).await.expect("attach");
        assert!(attachment.try_acquire_lease_like_cpp());
        let owner = attachment.owner_like_cpp();
        let lease = attachment.lease_id_like_cpp();
        let request = add_request_like_cpp(9, 11, 1);
        let (first, second) = tokio::join!(
            owner.try_add_pet_like_cpp(lease, request.clone()),
            owner.try_add_pet_like_cpp(lease, request),
        );
        let outcomes = [first, second];
        assert_eq!(
            outcomes
                .iter()
                .filter(|outcome| matches!(outcome, Ok(BattlePetAddOutcomeLikeCpp::Added(_))))
                .count(),
            1
        );
        assert_eq!(
            outcomes
                .iter()
                .filter(|outcome| matches!(outcome, Ok(BattlePetAddOutcomeLikeCpp::Replayed(_))))
                .count(),
            1
        );
        assert_eq!(persistence.insert_calls.load(Ordering::Relaxed), 1);
        assert_eq!(
            persistence
                .state
                .lock()
                .expect("fake persistence poisoned")
                .pets
                .len(),
            1
        );
    }

    #[tokio::test]
    async fn failed_insert_publishes_nothing_and_retry_can_commit_like_cpp() {
        let persistence = Arc::new(FakePersistenceLikeCpp::default());
        persistence.fail_next_insert.store(true, Ordering::Release);
        let registry = registry_like_cpp(Arc::clone(&persistence), 0, 300);
        let attachment = registry.attach_like_cpp(77).await.expect("attach");
        assert!(attachment.try_acquire_lease_like_cpp());
        let owner = attachment.owner_like_cpp();
        let lease = attachment.lease_id_like_cpp();
        let request = add_request_like_cpp(10, 11, 1);
        assert!(matches!(
            owner.try_add_pet_like_cpp(lease, request.clone()).await,
            Err(BattlePetAddFailureLikeCpp::DatabaseFailure(_))
        ));
        assert!(
            owner
                .journal_like_cpp(lease, request.owner_guid)
                .pets
                .is_empty()
        );
        assert!(matches!(
            owner.try_add_pet_like_cpp(lease, request.clone()).await,
            Ok(BattlePetAddOutcomeLikeCpp::Added(_))
        ));
        assert_eq!(
            owner.journal_like_cpp(lease, request.owner_guid).pets.len(),
            1
        );
    }

    #[tokio::test]
    async fn durable_pet_and_slots_reload_without_a_session_mirror_like_cpp() {
        let persistence = Arc::new(FakePersistenceLikeCpp::default());
        let registry = registry_like_cpp(
            Arc::clone(&persistence),
            BATTLE_PET_SPECIES_FLAG_NOT_ACCOUNT_WIDE_LIKE_CPP,
            400,
        );
        let attachment = registry.attach_like_cpp(77).await.expect("attach");
        assert!(attachment.try_acquire_lease_like_cpp());
        let owner = attachment.owner_like_cpp();
        let lease = attachment.lease_id_like_cpp();
        let request = add_request_like_cpp(11, 11, 1);
        let pet_guid = match owner
            .try_add_pet_like_cpp(lease, request.clone())
            .await
            .expect("add")
        {
            BattlePetAddOutcomeLikeCpp::Added(pet) | BattlePetAddOutcomeLikeCpp::Replayed(pet) => {
                pet.guid
            }
        };
        owner
            .try_set_slot_like_cpp(lease, pet_guid, 0)
            .await
            .expect("persist slot");
        drop(attachment);
        drop(registry);

        let restarted = registry_like_cpp(
            Arc::clone(&persistence),
            BATTLE_PET_SPECIES_FLAG_NOT_ACCOUNT_WIDE_LIKE_CPP,
            401,
        );
        let attachment = restarted.attach_like_cpp(77).await.expect("reload");
        assert!(attachment.try_acquire_lease_like_cpp());
        let journal = attachment
            .owner_like_cpp()
            .journal_like_cpp(attachment.lease_id_like_cpp(), request.owner_guid);
        assert_eq!(journal.pets.len(), 1);
        assert_eq!(journal.slots[0].pet_guid, pet_guid);
        assert_eq!(
            journal.pets[0].owner_info.unwrap().player_virtual_realm,
            0x0102_0007
        );
    }

    #[tokio::test]
    async fn legacy_unique_and_owner_specific_caps_use_the_effective_scope_like_cpp() {
        let persistence = Arc::new(FakePersistenceLikeCpp::default());
        let registry = registry_like_cpp(
            Arc::clone(&persistence),
            BATTLE_PET_SPECIES_FLAG_NOT_ACCOUNT_WIDE_LIKE_CPP,
            500,
        );
        let attachment = registry.attach_like_cpp(77).await.expect("attach");
        assert!(attachment.try_acquire_lease_like_cpp());
        let owner = attachment.owner_like_cpp();
        let lease = attachment.lease_id_like_cpp();
        for key in 1..=3 {
            owner
                .try_add_pet_like_cpp(lease, add_request_like_cpp(key, 11, 1))
                .await
                .expect("first owner normal-cap pet");
        }
        assert!(matches!(
            owner
                .try_add_pet_like_cpp(lease, add_request_like_cpp(4, 11, 1))
                .await,
            Err(BattlePetAddFailureLikeCpp::Capacity)
        ));
        owner
            .try_add_pet_like_cpp(lease, add_request_like_cpp(5, 11, 2))
            .await
            .expect("second owner has independent normal cap");
        owner
            .try_add_pet_like_cpp(lease, add_request_like_cpp(6, 12, 1))
            .await
            .expect("first legacy-unique pet");
        assert!(matches!(
            owner
                .try_add_pet_like_cpp(lease, add_request_like_cpp(7, 12, 1))
                .await,
            Err(BattlePetAddFailureLikeCpp::Capacity)
        ));
    }

    #[tokio::test]
    async fn cancelled_session_does_not_cancel_reserved_insert_like_cpp() {
        let persistence = Arc::new(FakePersistenceLikeCpp::default());
        persistence.block_next_insert.store(true, Ordering::Release);
        let registry = registry_like_cpp(Arc::clone(&persistence), 0, 600);
        let attachment = registry.attach_like_cpp(77).await.expect("attach");
        assert!(attachment.try_acquire_lease_like_cpp());
        let owner = Arc::clone(attachment.owner_like_cpp());
        let lease = attachment.lease_id_like_cpp();
        let request = add_request_like_cpp(21, 11, 1);
        let request_for_worker = request.clone();
        let owner_for_worker = Arc::clone(&owner);
        let session_task = tokio::spawn(async move {
            owner_for_worker
                .try_add_pet_like_cpp(lease, request_for_worker)
                .await
        });

        persistence.insert_started.notified().await;
        session_task.abort();
        let _ = session_task.await;
        persistence.allow_insert.notify_one();

        assert!(registry.drain_like_cpp(Duration::from_secs(1)).await);
        assert_eq!(
            owner.journal_like_cpp(lease, request.owner_guid).pets.len(),
            1
        );
        assert!(matches!(
            owner.try_add_pet_like_cpp(lease, request).await,
            Ok(BattlePetAddOutcomeLikeCpp::Replayed(_))
        ));
        assert_eq!(persistence.insert_calls.load(Ordering::Relaxed), 1);
    }

    #[tokio::test]
    async fn historical_retry_receipt_does_not_revive_deleted_pet_like_cpp() {
        let persistence = Arc::new(FakePersistenceLikeCpp::default());
        let registry = registry_like_cpp(Arc::clone(&persistence), 0, 700);
        let attachment = registry.attach_like_cpp(77).await.expect("attach");
        assert!(attachment.try_acquire_lease_like_cpp());
        let owner = attachment.owner_like_cpp();
        let lease = attachment.lease_id_like_cpp();
        let request = add_request_like_cpp(22, 11, 1);
        let pet_guid = match owner
            .try_add_pet_like_cpp(lease, request.clone())
            .await
            .expect("add")
        {
            BattlePetAddOutcomeLikeCpp::Added(pet) | BattlePetAddOutcomeLikeCpp::Replayed(pet) => {
                pet.guid
            }
        };
        owner
            .try_remove_pet_like_cpp(lease, pet_guid)
            .await
            .expect("delete");
        drop(attachment);
        drop(registry);

        let restarted = registry_like_cpp(Arc::clone(&persistence), 0, 701);
        let attachment = restarted.attach_like_cpp(77).await.expect("reload");
        assert!(attachment.try_acquire_lease_like_cpp());
        let owner = attachment.owner_like_cpp();
        assert!(matches!(
            owner
                .try_add_pet_like_cpp(attachment.lease_id_like_cpp(), request.clone())
                .await,
            Err(BattlePetAddFailureLikeCpp::DuplicateRequest)
        ));
        assert!(
            owner
                .journal_like_cpp(attachment.lease_id_like_cpp(), request.owner_guid)
                .pets
                .is_empty()
        );
    }

    #[tokio::test]
    async fn restart_retry_is_replayed_before_capacity_check_like_cpp() {
        let persistence = Arc::new(FakePersistenceLikeCpp::default());
        let registry = registry_like_cpp(Arc::clone(&persistence), 0, 750);
        let attachment = registry.attach_like_cpp(77).await.expect("attach");
        assert!(attachment.try_acquire_lease_like_cpp());
        let owner = attachment.owner_like_cpp();
        let lease = attachment.lease_id_like_cpp();
        let first_request = add_request_like_cpp(31, 11, 1);
        let first_guid = match owner
            .try_add_pet_like_cpp(lease, first_request.clone())
            .await
            .expect("first capped pet")
        {
            BattlePetAddOutcomeLikeCpp::Added(pet) | BattlePetAddOutcomeLikeCpp::Replayed(pet) => {
                pet.guid
            }
        };
        for request in [
            add_request_like_cpp(32, 11, 1),
            add_request_like_cpp(33, 11, 1),
        ] {
            owner
                .try_add_pet_like_cpp(lease, request)
                .await
                .expect("fill species cap");
        }
        owner
            .try_mutate_pet_like_cpp(lease, first_guid, |pet| pet.level = 9)
            .await
            .expect("persist post-add mutation");
        drop(attachment);
        drop(registry);

        let restarted = registry_like_cpp(Arc::clone(&persistence), 0, 751);
        let attachment = restarted.attach_like_cpp(77).await.expect("reload");
        assert!(attachment.try_acquire_lease_like_cpp());
        let replayed = attachment
            .owner_like_cpp()
            .try_add_pet_like_cpp(attachment.lease_id_like_cpp(), first_request)
            .await
            .expect("retry before cap");
        assert!(matches!(
            replayed,
            BattlePetAddOutcomeLikeCpp::Replayed(ref pet) if pet.level == 9
        ));
        assert!(matches!(
            attachment
                .owner_like_cpp()
                .try_add_pet_like_cpp(
                    attachment.lease_id_like_cpp(),
                    add_request_like_cpp(31, 11, 1),
                )
                .await,
            Ok(BattlePetAddOutcomeLikeCpp::Replayed(ref pet)) if pet.level == 9
        ));
    }

    #[tokio::test]
    async fn failed_update_delete_and_slot_persistence_publish_nothing_like_cpp() {
        let persistence = Arc::new(FakePersistenceLikeCpp::default());
        let registry = registry_like_cpp(Arc::clone(&persistence), 0, 800);
        let attachment = registry.attach_like_cpp(77).await.expect("attach");
        assert!(attachment.try_acquire_lease_like_cpp());
        let owner = attachment.owner_like_cpp();
        let lease = attachment.lease_id_like_cpp();
        let request = add_request_like_cpp(23, 11, 1);
        let pet_guid = match owner
            .try_add_pet_like_cpp(lease, request.clone())
            .await
            .expect("add")
        {
            BattlePetAddOutcomeLikeCpp::Added(pet) | BattlePetAddOutcomeLikeCpp::Replayed(pet) => {
                pet.guid
            }
        };
        let original = owner.pet_snapshot_like_cpp(pet_guid).expect("pet");

        persistence.fail_next_update.store(true, Ordering::Release);
        assert!(matches!(
            owner
                .try_mutate_pet_like_cpp(lease, pet_guid, |pet| pet.level = 9)
                .await,
            Err(BattlePetMutationFailureLikeCpp::DatabaseFailure(_))
        ));
        assert_eq!(owner.pet_snapshot_like_cpp(pet_guid), Some(original));

        persistence.fail_next_slots.store(true, Ordering::Release);
        assert!(matches!(
            owner.try_set_slot_like_cpp(lease, pet_guid, 0).await,
            Err(BattlePetMutationFailureLikeCpp::DatabaseFailure(_))
        ));
        assert_eq!(
            owner.journal_like_cpp(lease, request.owner_guid).slots[0].pet_guid,
            empty_battle_pet_guid_like_cpp()
        );

        persistence.fail_next_delete.store(true, Ordering::Release);
        assert!(matches!(
            owner.try_remove_pet_like_cpp(lease, pet_guid).await,
            Err(BattlePetMutationFailureLikeCpp::DatabaseFailure(_))
        ));
        assert!(owner.pet_snapshot_like_cpp(pet_guid).is_some());
    }
}
