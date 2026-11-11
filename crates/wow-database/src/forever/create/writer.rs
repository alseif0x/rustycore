//! Physical CharacterDB writer ownership and Auth locks shared by save/recovery.
use super::{CreateError, PreparedStatement};
use crate::{
    CharacterDatabase, LoginDatabase, SqlResult, SqlTransaction, SqlTransactionCommitError,
};
use sqlx::{MySql, Transaction, pool::PoolConnection};

pub(super) struct CreationWriter {
    connection: PoolConnection<MySql>,
    lock_name: String,
}

impl CreationWriter {
    pub(super) async fn acquire(database: &CharacterDatabase) -> Result<Self, CreateError> {
        let mut connection = database
            .pool()
            .acquire()
            .await
            .map_err(|_| CreateError::Database)?;
        // An ambiguous GET_LOCK must close this physical session on drop.
        connection.close_on_drop();
        let database_name = sqlx::query_scalar::<_, Option<String>>("SELECT DATABASE()")
            .fetch_one(&mut *connection)
            .await
            .map_err(|_| CreateError::Database)?
            .ok_or(CreateError::InvalidRow)?;
        let lock_name = crate::transaction::item_guid_allocator_lock_name_like_cpp(&database_name);
        // Bounded queueing also covers a predecessor's close/rollback still
        // settling after its cancelled future released the in-memory gate.
        let acquired = sqlx::query_scalar::<_, Option<i64>>("SELECT GET_LOCK(?, 5)")
            .bind(&lock_name)
            .fetch_one(&mut *connection)
            .await
            .map_err(|_| CreateError::Database)?;
        crate::persistence_trace::record_advisory_lock(&lock_name, acquired == Some(1));
        if acquired != Some(1) {
            return Err(CreateError::RecoveryRequired);
        }
        Ok(Self {
            connection,
            lock_name,
        })
    }

    pub(super) async fn query(
        &mut self,
        statement: &PreparedStatement,
    ) -> Result<SqlResult, CreateError> {
        let mut query = sqlx::query(statement.sql());
        for param in statement.params() {
            query = crate::transaction::bind_param(query, param);
        }
        query
            .fetch_all(&mut *self.connection)
            .await
            .map(SqlResult::new)
            .map_err(|_| CreateError::Database)
    }

    pub(super) async fn commit(
        &mut self,
        mut transaction: SqlTransaction,
    ) -> Result<(), SqlTransactionCommitError> {
        transaction.attribute_to_like_cpp(crate::persistence_trace::LogicalDatabase::Character);
        transaction.commit_on_connection(&mut self.connection).await
    }
}

impl Drop for CreationWriter {
    fn drop(&mut self) {
        // close_on_drop releases the advisory lock with this exact session;
        // no successor can acquire it while an old CharacterDB commit is pending.
        crate::persistence_trace::record_advisory_lock(&self.lock_name, false);
    }
}

pub(super) async fn auth_query(
    transaction: &mut Transaction<'static, MySql>,
    statement: &PreparedStatement,
) -> Result<SqlResult, CreateError> {
    let mut query = sqlx::query(statement.sql());
    for param in statement.params() {
        query = crate::transaction::bind_param(query, param);
    }
    query
        .fetch_all(&mut **transaction)
        .await
        .map(SqlResult::new)
        .map_err(|_| CreateError::Database)
}

pub(super) async fn lock_account(
    auth: &LoginDatabase,
    account: u32,
    realm: u32,
) -> Result<Transaction<'static, MySql>, CreateError> {
    let mut transaction = auth
        .pool()
        .begin()
        .await
        .map_err(|_| CreateError::Database)?;
    let mut statement = PreparedStatement::new("SELECT id FROM account WHERE id=? FOR UPDATE");
    statement.set_u32(0, account);
    let row = auth_query(&mut transaction, &statement).await?;
    if row.count() != 1 || row.try_read::<u32>(0) != Some(account) {
        return Err(CreateError::InvalidRow);
    }
    // Hold this lock before any CharacterDB write. Recovery on a successor
    // must wait until an old writer's Auth transaction has settled, even if
    // its CharacterDB connection was lost after committing the character.
    let mut statement = PreparedStatement::new(
        "SELECT accountId FROM account_last_played_character WHERE accountId=? AND region=? AND battlegroup=? FOR UPDATE",
    );
    statement.set_u32(0, account);
    statement.set_u8(1, (realm >> 24) as u8);
    statement.set_u8(2, (realm >> 16) as u8);
    auth_query(&mut transaction, &statement).await?;
    Ok(transaction)
}

pub(super) async fn commit_auth(
    mut batch: SqlTransaction,
    transaction: Transaction<'static, MySql>,
) -> Result<(), SqlTransactionCommitError> {
    batch.attribute_to_like_cpp(crate::persistence_trace::LogicalDatabase::Login);
    batch.commit_started(transaction).await
}

pub(super) fn delete_last_played(entry: super::journal::Entry) -> PreparedStatement {
    let mut statement = PreparedStatement::new(
        "DELETE FROM account_last_played_character WHERE accountId=? AND region=? AND battlegroup=?",
    );
    statement.set_u32(0, entry.account);
    statement.set_u8(1, (entry.realm >> 24) as u8);
    statement.set_u8(2, (entry.realm >> 16) as u8);
    statement
}

pub(super) fn insert_last_played(
    entry: super::journal::Entry,
    name: impl Into<String>,
) -> PreparedStatement {
    let mut statement = PreparedStatement::new(
        "INSERT INTO account_last_played_character (accountId, region, battlegroup, realmId, characterName, characterGUID, lastPlayedTime) VALUES (?,?,?,?,?,?,?)",
    );
    statement.set_u32(0, entry.account);
    statement.set_u8(1, (entry.realm >> 24) as u8);
    statement.set_u8(2, (entry.realm >> 16) as u8);
    statement.set_u32(3, entry.realm & 0xFFFF);
    statement.set_string(4, name);
    statement.set_u64(5, entry.guid);
    statement.set_u32(6, entry.save_time);
    statement
}

pub(super) fn realm_count(entry: super::journal::Entry, count: u8) -> PreparedStatement {
    let mut statement = PreparedStatement::new(
        "REPLACE INTO realmcharacters (numchars, acctid, realmid) VALUES (?,?,?)",
    );
    statement.set_u32(0, u32::from(count));
    statement.set_u32(1, entry.account);
    statement.set_u32(2, entry.realm & 0xFFFF);
    statement
}
