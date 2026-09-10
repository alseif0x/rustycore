//! Rows packets.
//!
//! Separated from account_adapter.rs under #701.

use super::*;

pub(super) async fn load_pet_rows_like_cpp(
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

pub(super) async fn load_slot_rows_like_cpp(
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

pub(super) fn durable_pet_from_result_like_cpp(
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

pub(super) async fn find_request_in_tx_like_cpp(
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

pub(super) fn durable_pet_from_row_like_cpp(
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

pub(super) fn add_request_matches_like_cpp(
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

pub(super) fn database_error_like_cpp(error: DatabaseError) -> BattlePetPersistenceErrorLikeCpp {
    BattlePetPersistenceErrorLikeCpp::Database(error.to_string())
}

pub(super) fn row_decode_error_like_cpp(error: sqlx::Error) -> BattlePetPersistenceErrorLikeCpp {
    BattlePetPersistenceErrorLikeCpp::Database(error.to_string())
}

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

pub(super) fn is_duplicate_key_like_cpp(error: &DatabaseError) -> bool {
    matches!(
        error,
        DatabaseError::Query(sqlx::Error::Database(database_error))
            if database_error.code().as_deref() == Some("1062")
    )
}
