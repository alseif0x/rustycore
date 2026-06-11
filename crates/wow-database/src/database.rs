//! Core database type: type-safe wrapper around a MySQL connection pool.

use crate::error::DatabaseError;
use crate::params::PreparedStatement;
use crate::result::SqlResult;
use crate::statements::StatementDef;
use crate::transaction::{SqlTransaction, bind_param};
use sqlx::MySqlPool;
use sqlx::mysql::MySqlPoolOptions;
use std::marker::PhantomData;

pub const KEEP_ALIVE_SQL_LIKE_CPP: &str = "SELECT 1";

/// A type-safe database connection wrapping a [`MySqlPool`].
///
/// The type parameter `S` is a statement enum (e.g. `LoginStatements`) that
/// determines which prepared statements can be used with this database.
/// This makes it a compile-time error to use a `WorldStatements` variant on a
/// `Database<LoginStatements>`.
///
/// # Example
///
/// ```ignore
/// let db: Database<LoginStatements> = Database::open("mysql://...").await?;
/// let mut stmt = db.prepare(LoginStatements::SEL_REALMLIST);
/// let result = db.query(&stmt).await?;
/// ```
pub struct Database<S: StatementDef> {
    pool: MySqlPool,
    _marker: PhantomData<S>,
}

impl<S: StatementDef> Database<S> {
    /// Open a connection pool to the given MySQL database.
    ///
    /// `connection_string` should be a MySQL URL like:
    /// `mysql://user:password@host:port/database`
    pub async fn open(connection_string: &str) -> Result<Self, DatabaseError> {
        Self::open_with_pool_size(connection_string, 10).await
    }

    /// Open a connection pool with a specific maximum number of connections.
    pub async fn open_with_pool_size(
        connection_string: &str,
        max_connections: u32,
    ) -> Result<Self, DatabaseError> {
        let pool = MySqlPoolOptions::new()
            .max_connections(max_connections)
            .idle_timeout(std::time::Duration::from_secs(1800))
            .connect(connection_string)
            .await
            .map_err(|e| DatabaseError::Connection(e.to_string()))?;

        tracing::info!(
            database = %connection_string.split('/').next_back().unwrap_or("?"),
            "Connected to MySQL database"
        );

        Ok(Self {
            pool,
            _marker: PhantomData,
        })
    }

    /// Create a database wrapper from an existing pool.
    pub fn from_pool(pool: MySqlPool) -> Self {
        Self {
            pool,
            _marker: PhantomData,
        }
    }

    /// Get a reference to the underlying connection pool.
    pub fn pool(&self) -> &MySqlPool {
        &self.pool
    }

    /// Create a [`PreparedStatement`] for the given statement enum variant.
    ///
    /// The SQL is looked up from the static statement registry. Returns a
    /// statement with no bound parameters; use the `set_*` methods to bind
    /// values before executing.
    pub fn prepare(&self, stmt: S) -> PreparedStatement {
        let sql = stmt.sql();
        PreparedStatement::new(sql)
    }

    /// Execute a query and return the result rows.
    pub async fn query(&self, stmt: &PreparedStatement) -> Result<SqlResult, DatabaseError> {
        let sql = stmt.sql();
        if sql.is_empty() {
            return Err(DatabaseError::UnregisteredStatement(0));
        }

        let mut query = sqlx::query(sql);
        for param in stmt.params() {
            query = bind_param(query, param);
        }

        let rows = query.fetch_all(&self.pool).await?;
        Ok(SqlResult::new(rows))
    }

    /// Execute a statement that does not return rows (INSERT, UPDATE, DELETE).
    ///
    /// Returns the number of affected rows.
    pub async fn execute(&self, stmt: &PreparedStatement) -> Result<u64, DatabaseError> {
        let sql = stmt.sql();
        if sql.is_empty() {
            return Err(DatabaseError::UnregisteredStatement(0));
        }

        let mut query = sqlx::query(sql);
        for param in stmt.params() {
            query = bind_param(query, param);
        }

        let result = query.execute(&self.pool).await?;
        Ok(result.rows_affected())
    }

    /// Execute a raw SQL string directly (no prepared statement).
    pub async fn direct_execute(&self, sql: &str) -> Result<u64, DatabaseError> {
        let result = sqlx::query(sql).execute(&self.pool).await?;
        Ok(result.rows_affected())
    }

    /// Execute a raw SQL query directly (no prepared statement).
    pub async fn direct_query(&self, sql: &str) -> Result<SqlResult, DatabaseError> {
        let rows = sqlx::query(sql).fetch_all(&self.pool).await?;
        Ok(SqlResult::new(rows))
    }

    /// Ping the database connection pool, mirroring TrinityCore's KeepAlive().
    pub async fn keep_alive_like_cpp(&self) -> Result<(), DatabaseError> {
        sqlx::query(KEEP_ALIVE_SQL_LIKE_CPP)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Execute a query or append it to a transaction.
    ///
    /// If `trans` is `None`, the statement is executed immediately.
    /// If `trans` is `Some`, the statement is appended to the transaction batch.
    pub async fn execute_or_append(
        &self,
        trans: Option<&mut SqlTransaction>,
        stmt: PreparedStatement,
    ) -> Result<(), DatabaseError> {
        match trans {
            Some(tx) => {
                tx.append(stmt);
                Ok(())
            }
            None => {
                self.execute(&stmt).await?;
                Ok(())
            }
        }
    }

    /// Commit a transaction batch atomically.
    pub async fn commit_transaction(&self, trans: SqlTransaction) -> Result<(), DatabaseError> {
        trans.commit(&self.pool).await
    }

    /// Close the connection pool.
    pub async fn close(&self) {
        self.pool.close().await;
    }
}

impl<S: StatementDef> std::fmt::Debug for Database<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Database")
            .field("pool_size", &self.pool.size())
            .finish()
    }
}

/// Build a MySQL connection string from TrinityCore `*DatabaseInfo` parts.
///
/// The second field is `port_or_socket` in C++, so numeric values become the
/// URL port and non-numeric values are passed as a unix socket query parameter.
pub fn build_connection_string(
    host: &str,
    port_or_socket: &str,
    user: &str,
    password: &str,
    database: &str,
) -> String {
    if port_or_socket
        .chars()
        .next()
        .is_some_and(|ch| ch.is_ascii_digit())
    {
        return format!("mysql://{user}:{password}@{host}:{port_or_socket}/{database}");
    }

    format!(
        "mysql://{user}:{password}@localhost/{database}?socket={}",
        percent_encode_query(port_or_socket)
    )
}

fn percent_encode_query(value: &str) -> String {
    let mut encoded = String::new();
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' | b'/' => {
                encoded.push(char::from(byte));
            }
            other => encoded.push_str(&format!("%{other:02X}")),
        }
    }
    encoded
}

#[cfg(test)]
mod tests {
    use super::build_connection_string;

    #[test]
    fn build_connection_string_uses_numeric_port() {
        assert_eq!(
            build_connection_string("127.0.0.1", "3306", "trinity", "trinity", "auth"),
            "mysql://trinity:trinity@127.0.0.1:3306/auth"
        );
    }

    #[test]
    fn build_connection_string_uses_socket_for_non_numeric_port_or_socket() {
        assert_eq!(
            build_connection_string(
                ".",
                "/var/run/mysqld/mysqld.sock",
                "trinity",
                "trinity",
                "world",
            ),
            "mysql://trinity:trinity@localhost/world?socket=/var/run/mysqld/mysqld.sock"
        );
    }
}
