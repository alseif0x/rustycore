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

mod lock_broker;
mod persistence;
mod rows;

pub use persistence::*;

use lock_broker::{
    BattlePetAccountLockBrokerLikeCpp, BattlePetBrokerLeaseReservationLikeCpp,
    LoginBattlePetProcessLeaseLikeCpp,
};

#[cfg(test)]
use lock_broker::battle_pet_account_lock_name_like_cpp;

use rows::{
    add_request_matches_like_cpp, database_error_like_cpp, durable_pet_from_result_like_cpp,
    durable_pet_from_row_like_cpp, find_request_in_tx_like_cpp, is_duplicate_key_like_cpp,
    load_pet_rows_like_cpp, load_slot_rows_like_cpp, row_decode_error_like_cpp,
};

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

#[cfg(test)]
#[path = "tests/mod.rs"]
mod tests;
