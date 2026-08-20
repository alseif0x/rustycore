//! Core database type: type-safe wrapper around a MySQL connection pool.

use crate::error::DatabaseError;
use crate::params::PreparedStatement;
use crate::query_holder::{SqlQueryHolder, SqlQueryHolderResult, prepared_result_slot_like_cpp};
use crate::result::SqlResult;
use crate::statements::StatementDef;
use crate::transaction::{SqlTransaction, bind_param};
use sqlx::MySqlPool;
use sqlx::mysql::MySqlPoolOptions;
use std::future::Future;
use std::marker::PhantomData;

pub const KEEP_ALIVE_SQL_LIKE_CPP: &str = "SELECT 1";

tokio::task_local! {
    static WARN_SYNC_QUERIES_LIKE_CPP: bool;
}

/// Run a future under the same diagnostic mode that TC enables around
/// `WorldUpdateLoop()`: DB calls made inside the scope emit a warning.
pub async fn warn_about_sync_queries_scope_like_cpp<F>(future: F) -> F::Output
where
    F: Future,
{
    WARN_SYNC_QUERIES_LIKE_CPP.scope(true, future).await
}

pub fn warn_about_sync_queries_enabled_like_cpp() -> bool {
    WARN_SYNC_QUERIES_LIKE_CPP
        .try_with(|enabled| *enabled)
        .unwrap_or(false)
}

fn warn_if_sync_query_like_cpp(operation: &str) {
    if warn_about_sync_queries_enabled_like_cpp() {
        tracing::warn!(
            target: "sql.performances",
            operation,
            "Sync-style DB query executed inside a world update tick"
        );
    }
}

/// A type-safe database connection wrapping a [`MySqlPool`].
///
/// The type parameter `S` is a statement enum (e.g. `LoginStatements`) that
/// determines which prepared statements can be used with this database.
/// This makes it a compile-time error to use a `WorldStatements` variant on a
/// `Database<LoginStatements>`.
///
/// # Example
///
/// ```no_run
/// # use wow_database::{Database, statements::LoginStatements};
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let db: Database<LoginStatements> =
///     Database::open("mysql://user:pass@127.0.0.1:3306/auth").await?;
/// let mut stmt = db.prepare(LoginStatements::SEL_REALMLIST);
/// let result = db.query(&stmt).await?;
/// # Ok(())
/// # }
/// ```
///
/// A statement enum from a different database cannot be prepared on this
/// typed pool. This mirrors TC's `DatabaseWorkerPool<T>` / `PreparedStatement<T>`
/// binding at the Rust type boundary.
///
/// ```compile_fail
/// use wow_database::{Database, statements::{LoginStatements, WorldStatements}};
///
/// fn wrong_statement_type(db: &Database<WorldStatements>) {
///     let _stmt = db.prepare(LoginStatements::SEL_REALMLIST);
/// }
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
        let pool = connect_pool_like_cpp(connection_string, max_connections).await?;

        tracing::info!(
            database = %connection_string.split('/').next_back().unwrap_or("?"),
            "Connected to MySQL database"
        );

        Ok(Self {
            pool,
            _marker: PhantomData,
        })
    }

    /// Open a pool and, if enabled, mirror TC's DBUpdater::Create fallback for
    /// missing databases before retrying the connection.
    pub async fn open_with_pool_size_and_auto_create_like_cpp(
        host: &str,
        port_or_socket: &str,
        user: &str,
        password: &str,
        database: &str,
        ssl: bool,
        max_connections: u32,
        auto_create: bool,
    ) -> Result<Self, DatabaseError> {
        let connection_string = build_connection_string_with_ssl_like_cpp(
            host,
            port_or_socket,
            user,
            password,
            database,
            ssl,
        );

        match connect_pool_sqlx_like_cpp(&connection_string, max_connections).await {
            Ok(pool) => {
                tracing::info!(database = %database, "Connected to MySQL database");
                Ok(Self {
                    pool,
                    _marker: PhantomData,
                })
            }
            Err(err) if auto_create && is_unknown_database_error_like_cpp(&err) => {
                tracing::info!(
                    database = %database,
                    "Database does not exist; creating it before reconnecting"
                );
                create_database_like_cpp(host, port_or_socket, user, password, database, ssl)
                    .await?;
                let pool = connect_pool_sqlx_like_cpp(&connection_string, max_connections)
                    .await
                    .map_err(|e| DatabaseError::Connection(e.to_string()))?;
                tracing::info!(database = %database, "Connected to MySQL database");
                Ok(Self {
                    pool,
                    _marker: PhantomData,
                })
            }
            Err(err) => Err(DatabaseError::Connection(err.to_string())),
        }
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
        let prepared = PreparedStatement::new(sql);
        // Deriving the identity allocates, and this is on every query path, so
        // production pays one relaxed load instead.
        if crate::persistence_trace::recording_enabled() {
            return prepared
                .with_trace_identity(stmt.trace_identity())
                .with_trace_database(stmt.logical_database());
        }
        prepared
    }

    /// Record a statement that runs on its own pooled connection.
    ///
    /// Single statements are not incidental: a read can gate a write, and
    /// whether it shared the writer's transaction or took its own connection
    /// is precisely the fact a port extraction can change without moving a
    /// row. `Pooled` is that distinction.
    /// Whether a pooled failure happened before MySQL could receive the
    /// statement.
    ///
    /// Acquiring a connection fails without sending anything, so that outcome is
    /// definite: the statement did not run. Everything else — a protocol error,
    /// an I/O error, a server error — may have been received and applied, and
    /// only those are genuinely ambiguous. Collapsing the two directions loses
    /// the crash semantics this contract exists to freeze, in one direction or
    /// the other.
    fn pooled_failure_is_definite(error: &sqlx::Error) -> bool {
        matches!(
            error,
            sqlx::Error::PoolTimedOut | sqlx::Error::PoolClosed | sqlx::Error::Configuration(_)
        )
    }

    fn record_pooled_statement(
        stmt: &PreparedStatement,
        observed_rows_affected: Option<u64>,
        definitely_not_run: bool,
    ) {
        let succeeded = observed_rows_affected.is_some();
        let Some(recorder) = crate::persistence_trace::ambient_recorder() else {
            return;
        };
        let Some(database) = stmt.trace_database() else {
            // Raw SQL executed outside a transaction carries no statement enum
            // and therefore no database; guessing one would be worse than a
            // gap the reader can see.
            return;
        };
        let params = stmt
            .params()
            .iter()
            .map(crate::persistence_trace::TracedParam::from_param)
            .collect();
        match stmt.trace_identity() {
            Some(statement) => {
                recorder.record(crate::persistence_trace::PersistenceEvent::Statement {
                    database,
                    connection: crate::persistence_trace::ConnectionAffinity::Pooled,
                    statement: statement.to_owned(),
                    params,
                    expected_rows_affected: None,
                    observed_rows_affected,
                });
                if !succeeded && definitely_not_run {
                    // Never reached the server, so the statement definitely did
                    // not run — a rollback in the only sense available without a
                    // transaction.
                    recorder
                        .record(crate::persistence_trace::PersistenceEvent::Rollback { database });
                } else if !succeeded {
                    // Deliberately not a `Rollback`: there is no transaction
                    // here, and a transport error on an autocommit write may
                    // have applied it, so the outcome is unknown rather than
                    // reverted. A false record is worse than a missing one — a
                    // golden could approve a refactor on the strength of a
                    // revert that never happened. Carrying the ambiguity as an
                    // outcome on the pooled event itself is tracked in #213.
                    recorder.record(crate::persistence_trace::PersistenceEvent::Fence {
                        label: format!("pooled-statement-outcome-unknown:{}", database.as_str()),
                    });
                }
            }
            None => {
                recorder.record(crate::persistence_trace::PersistenceEvent::RawStatement {
                    database,
                    connection: crate::persistence_trace::ConnectionAffinity::Pooled,
                    digest: crate::persistence_trace::raw_statement_digest(stmt.sql()),
                    params: stmt
                        .params()
                        .iter()
                        .map(crate::persistence_trace::TracedParam::from_param)
                        .collect(),
                });
            }
        }
    }

    /// Execute a query and return the result rows.
    pub async fn query(&self, stmt: &PreparedStatement) -> Result<SqlResult, DatabaseError> {
        warn_if_sync_query_like_cpp("query");
        let sql = stmt.sql();
        if sql.is_empty() {
            return Err(DatabaseError::UnregisteredStatement(0));
        }

        let mut query = sqlx::query(sql);
        for param in stmt.params() {
            query = bind_param(query, param);
        }

        // Recorded after the call so the trace carries whether it ran.
        let rows = query.fetch_all(&self.pool).await;
        // A read has no affected-row count; `Some(0)` marks "it ran".
        Self::record_pooled_statement(
            stmt,
            rows.as_ref().ok().map(|_| 0),
            rows.as_ref()
                .err()
                .is_some_and(Self::pooled_failure_is_definite),
        );
        Ok(SqlResult::new(rows?))
    }

    /// Execute a statement that does not return rows (INSERT, UPDATE, DELETE).
    ///
    /// Returns the number of affected rows.
    pub async fn execute(&self, stmt: &PreparedStatement) -> Result<u64, DatabaseError> {
        warn_if_sync_query_like_cpp("execute");
        let sql = stmt.sql();
        if sql.is_empty() {
            return Err(DatabaseError::UnregisteredStatement(0));
        }

        let mut query = sqlx::query(sql);
        for param in stmt.params() {
            query = bind_param(query, param);
        }

        // Recorded after the call so the trace carries whether it ran.
        let result = query.execute(&self.pool).await;
        // Callers branch on this count — a save matching no character row is a
        // different outcome from one that matched — so the trace carries it.
        Self::record_pooled_statement(
            stmt,
            result.as_ref().ok().map(|r| r.rows_affected()),
            result
                .as_ref()
                .err()
                .is_some_and(Self::pooled_failure_is_definite),
        );
        Ok(result?.rows_affected())
    }

    /// Execute a raw SQL string directly (no prepared statement).
    pub async fn direct_execute(&self, sql: &str) -> Result<u64, DatabaseError> {
        warn_if_sync_query_like_cpp("direct_execute");
        let result = sqlx::query(sql).execute(&self.pool).await?;
        Ok(result.rows_affected())
    }

    /// Execute a raw SQL query directly (no prepared statement).
    pub async fn direct_query(&self, sql: &str) -> Result<SqlResult, DatabaseError> {
        warn_if_sync_query_like_cpp("direct_query");
        let rows = sqlx::query(sql).fetch_all(&self.pool).await?;
        Ok(SqlResult::new(rows))
    }

    /// Escape a string for legacy raw-SQL fragments.
    ///
    /// Prefer prepared statements whenever possible. This exists for C++ parity
    /// with `DatabaseWorkerPool<T>::EscapeString` and `mysql_real_escape_string`
    /// call sites that build SQL fragments dynamically.
    pub fn escape_string_like_cpp(&self, value: &str) -> String {
        escape_string_like_cpp(value)
    }

    /// Ping the database connection pool, mirroring TrinityCore's KeepAlive().
    pub async fn keep_alive_like_cpp(&self) -> Result<(), DatabaseError> {
        warn_if_sync_query_like_cpp("keep_alive");
        sqlx::query(KEEP_ALIVE_SQL_LIKE_CPP)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Execute a fixed-size holder of prepared queries.
    ///
    /// Mirrors `DatabaseWorkerPool<T>::DelayQueryHolder`: the holder is awaited
    /// asynchronously by the caller, while the queries themselves are executed
    /// in slot order. Empty result sets are stored as `None`, matching
    /// `SQLQueryHolderBase::SetPreparedResult`.
    pub async fn delay_query_holder_like_cpp(
        &self,
        holder: &SqlQueryHolder,
    ) -> Result<SqlQueryHolderResult, DatabaseError> {
        warn_if_sync_query_like_cpp("delay_query_holder");

        let mut results = Vec::with_capacity(holder.len());
        for stmt in holder.iter() {
            let Some(stmt) = stmt else {
                results.push(None);
                continue;
            };

            let result = self.query(stmt).await?;
            results.push(prepared_result_slot_like_cpp(result));
        }

        Ok(SqlQueryHolderResult::new(results))
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
    pub async fn commit_transaction(&self, mut trans: SqlTransaction) -> Result<(), DatabaseError> {
        warn_if_sync_query_like_cpp("commit_transaction");
        // A batch built entirely from raw SQL never names a database, so this
        // adapter is the only thing that can say which one it committed
        // against. Without it the boundary and outcome events were dropped and
        // flows like the bank-slot purchase left a statement with no begin,
        // commit or rollback around it.
        trans.attribute_to_like_cpp(S::DATABASE);
        trans.commit(&self.pool).await
    }

    /// Close the connection pool.
    pub async fn close(&self) {
        self.pool.close().await;
    }
}

async fn connect_pool_like_cpp(
    connection_string: &str,
    max_connections: u32,
) -> Result<MySqlPool, DatabaseError> {
    connect_pool_sqlx_like_cpp(connection_string, max_connections)
        .await
        .map_err(|e| DatabaseError::Connection(e.to_string()))
}

async fn connect_pool_sqlx_like_cpp(
    connection_string: &str,
    max_connections: u32,
) -> Result<MySqlPool, sqlx::Error> {
    MySqlPoolOptions::new()
        .max_connections(max_connections)
        .idle_timeout(std::time::Duration::from_secs(1800))
        .connect(connection_string)
        .await
}

async fn create_database_like_cpp(
    host: &str,
    port_or_socket: &str,
    user: &str,
    password: &str,
    database: &str,
    ssl: bool,
) -> Result<(), DatabaseError> {
    let server_connection =
        build_server_connection_string_like_cpp(host, port_or_socket, user, password, ssl);
    let pool = connect_pool_like_cpp(&server_connection, 1).await?;
    let sql = format!(
        "CREATE DATABASE `{}` DEFAULT CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci",
        escape_mysql_identifier_like_cpp(database)
    );
    sqlx::query(&sql).execute(&pool).await?;
    pool.close().await;
    Ok(())
}

fn is_unknown_database_error_like_cpp(error: &sqlx::Error) -> bool {
    match error {
        sqlx::Error::Database(db_error) => db_error.code().as_deref() == Some("1049"),
        sqlx::Error::Configuration(source) => source
            .to_string()
            .to_ascii_lowercase()
            .contains("unknown database"),
        _ => false,
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
    build_connection_string_with_ssl_like_cpp(host, port_or_socket, user, password, database, false)
}

/// Build a MySQL connection string including TC's optional `;ssl` flag.
///
/// TrinityCore only enables TLS when the sixth `*DatabaseInfo` field is exactly
/// `ssl`; otherwise it disables TLS. sqlx's default is `PREFERRED`, so RustyCore
/// writes an explicit `ssl-mode` to preserve the C++ behavior.
pub fn build_connection_string_with_ssl_like_cpp(
    host: &str,
    port_or_socket: &str,
    user: &str,
    password: &str,
    database: &str,
    ssl: bool,
) -> String {
    let query = mysql_connection_query_suffix_like_cpp(ssl);
    if port_or_socket
        .chars()
        .next()
        .is_some_and(|ch| ch.is_ascii_digit())
    {
        return format!("mysql://{user}:{password}@{host}:{port_or_socket}/{database}?{query}");
    }

    format!(
        "mysql://{user}:{password}@localhost/{database}?socket={}&{query}",
        percent_encode_query(port_or_socket),
    )
}

fn build_server_connection_string_like_cpp(
    host: &str,
    port_or_socket: &str,
    user: &str,
    password: &str,
    ssl: bool,
) -> String {
    let query = mysql_connection_query_suffix_like_cpp(ssl);
    if port_or_socket
        .chars()
        .next()
        .is_some_and(|ch| ch.is_ascii_digit())
    {
        return format!("mysql://{user}:{password}@{host}:{port_or_socket}?{query}");
    }

    format!(
        "mysql://{user}:{password}@localhost?socket={}&{query}",
        percent_encode_query(port_or_socket),
    )
}

fn mysql_connection_query_suffix_like_cpp(ssl: bool) -> String {
    // C++ sets MYSQL_SET_CHARSET_NAME/mysql_set_character_set("utf8mb4").
    // sqlx accepts `timezone` or `time-zone`; `time_zone` is ignored by its URL parser.
    format!(
        "ssl-mode={}&charset=utf8mb4&collation=utf8mb4_unicode_ci&timezone={}",
        ssl_mode_query_value_like_cpp(ssl),
        percent_encode_query("+00:00")
    )
}

fn ssl_mode_query_value_like_cpp(ssl: bool) -> &'static str {
    if ssl { "REQUIRED" } else { "DISABLED" }
}

/// Escape a string using MySQL's `mysql_real_escape_string` byte mapping.
///
/// TrinityCore calls this on a sync connection after setting the connection
/// character set to `utf8mb4`. For UTF-8 Rust strings the special-byte mapping
/// is deterministic: NUL, newline, carriage-return, backslash, single quote,
/// double quote and Ctrl-Z are escaped; all other bytes are copied through.
pub fn escape_string_like_cpp(value: &str) -> String {
    if value.is_empty() {
        return String::new();
    }

    let mut escaped = String::with_capacity(value.len() * 2);
    for ch in value.chars() {
        match ch {
            '\0' => escaped.push_str("\\0"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\\' => escaped.push_str("\\\\"),
            '\'' => escaped.push_str("\\'"),
            '"' => escaped.push_str("\\\""),
            '\u{1A}' => escaped.push_str("\\Z"),
            _ => escaped.push(ch),
        }
    }

    escaped
}

fn escape_mysql_identifier_like_cpp(identifier: &str) -> String {
    identifier.replace('`', "``")
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
    use super::*;
    use crate::persistence_trace::{
        ConnectionAffinity, LogicalDatabase, PersistenceEvent, PersistenceRecorder,
        RecordingSession,
    };
    use crate::statements::CharStatements;

    /// The pool is deliberately unreachable. Recording happens before the
    /// connection attempt, so a statement's identity and affinity are
    /// observable without a database — which is the only way these paths can
    /// be covered outside a MariaDB fixture.
    fn unreachable_pool() -> MySqlPool {
        sqlx::mysql::MySqlPoolOptions::new()
            .max_connections(1)
            .acquire_timeout(std::time::Duration::from_millis(1))
            .connect_lazy("mysql://rustycore:rustycore@127.0.0.1:1/characters")
            .expect("syntactically valid lazy pool")
    }

    #[tokio::test]
    async fn a_statement_outside_a_transaction_is_recorded_as_pooled() {
        let _serialized = crate::persistence_trace::capture_flag_test_lock();
        let recorder = PersistenceRecorder::new();
        let _recording = RecordingSession::install(recorder.clone());

        let db: Database<CharStatements> = Database::from_pool(unreachable_pool());
        let stmt = db.prepare(CharStatements::SEL_ENUM);
        let _ = db.query(&stmt).await;

        assert_eq!(
            recorder.take().events,
            vec![
                PersistenceEvent::Statement {
                    database: LogicalDatabase::Character,
                    connection: ConnectionAffinity::Pooled,
                    statement: "SEL_ENUM".to_owned(),
                    // C++ `PreparedStatementBase(index, capacity)` pre-allocates
                    // one slot per `?`, and this statement was never bound, so the
                    // trace faithfully shows the unbound placeholder rather than
                    // an empty parameter list.
                    params: vec![crate::persistence_trace::TracedParam::Bool { value: false }],
                    expected_rows_affected: None,
                    observed_rows_affected: None,
                },
                PersistenceEvent::Rollback {
                    database: LogicalDatabase::Character,
                }
            ],
            "a read on its own connection must not look like part of a transaction, \
             and a pool-acquisition failure is a definite non-execution rather than \
             an ambiguous outcome"
        );
    }

    #[tokio::test]
    async fn a_failed_pooled_statement_carries_no_observed_row_count() {
        let _serialized = crate::persistence_trace::capture_flag_test_lock();
        let recorder = PersistenceRecorder::new();
        let _recording = RecordingSession::install(recorder.clone());

        let db: Database<CharStatements> = Database::from_pool(unreachable_pool());
        let stmt = db.prepare(CharStatements::UPD_CHAR_MONEY);
        let _ = db.execute(&stmt).await;

        // A statement that never reached the server has no count to report, and
        // `None` has to stay distinguishable from a successful `Some(0)` — the
        // difference between "did not run" and "matched no row", which callers
        // such as `save_player_position_like_cpp` branch on.
        match recorder.take().events.first() {
            Some(PersistenceEvent::Statement {
                observed_rows_affected,
                ..
            }) => assert_eq!(*observed_rows_affected, None),
            other => panic!("expected a pooled statement, got {other:?}"),
        }
    }

    #[test]
    fn only_pool_acquisition_failures_are_definite() {
        // A statement that never left the process definitely did not run. One
        // that failed after being sent may have been applied, and calling that a
        // rollback would assert a revert that never happened — the two must not
        // collapse in either direction.
        assert!(Database::<CharStatements>::pooled_failure_is_definite(
            &sqlx::Error::PoolTimedOut
        ));
        assert!(Database::<CharStatements>::pooled_failure_is_definite(
            &sqlx::Error::PoolClosed
        ));
        assert!(!Database::<CharStatements>::pooled_failure_is_definite(
            &sqlx::Error::Protocol("server hung up mid-statement".to_owned())
        ));
        assert!(!Database::<CharStatements>::pooled_failure_is_definite(
            &sqlx::Error::WorkerCrashed
        ));
    }

    #[tokio::test]
    async fn nothing_is_recorded_without_an_installed_recording() {
        let _serialized = crate::persistence_trace::capture_flag_test_lock();
        let recorder = PersistenceRecorder::new();

        let db: Database<CharStatements> = Database::from_pool(unreachable_pool());
        let stmt = db.prepare(CharStatements::SEL_ENUM);
        let _ = db.query(&stmt).await;

        assert!(
            recorder.take().events.is_empty(),
            "production must not pay for tracing it did not ask for"
        );
    }

    use super::{
        build_connection_string, build_connection_string_with_ssl_like_cpp,
        build_server_connection_string_like_cpp, escape_mysql_identifier_like_cpp,
        escape_string_like_cpp, warn_about_sync_queries_enabled_like_cpp,
        warn_about_sync_queries_scope_like_cpp,
    };
    use sqlx::mysql::MySqlConnectOptions;
    use std::str::FromStr;

    #[test]
    fn build_connection_string_uses_numeric_port() {
        assert_eq!(
            build_connection_string("127.0.0.1", "3306", "trinity", "trinity", "auth"),
            "mysql://trinity:trinity@127.0.0.1:3306/auth?ssl-mode=DISABLED&charset=utf8mb4&collation=utf8mb4_unicode_ci&timezone=%2B00%3A00"
        );
    }

    #[test]
    fn build_connection_string_honors_ssl_flag_like_cpp() {
        assert_eq!(
            build_connection_string_with_ssl_like_cpp(
                "127.0.0.1",
                "3306",
                "trinity",
                "trinity",
                "auth",
                true,
            ),
            "mysql://trinity:trinity@127.0.0.1:3306/auth?ssl-mode=REQUIRED&charset=utf8mb4&collation=utf8mb4_unicode_ci&timezone=%2B00%3A00"
        );
    }

    #[test]
    fn build_connection_string_sets_utf8mb4_session_options_like_cpp() {
        let url = build_connection_string("127.0.0.1", "3306", "trinity", "trinity", "characters");
        let options = MySqlConnectOptions::from_str(&url).expect("sqlx should parse generated URL");

        assert_eq!(options.get_charset(), "utf8mb4");
        assert_eq!(options.get_collation(), Some("utf8mb4_unicode_ci"));
        assert!(
            url.contains("timezone=%2B00%3A00"),
            "sqlx-mysql parses `timezone`/`time-zone`, not `time_zone`"
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
            "mysql://trinity:trinity@localhost/world?socket=/var/run/mysqld/mysqld.sock&ssl-mode=DISABLED&charset=utf8mb4&collation=utf8mb4_unicode_ci&timezone=%2B00%3A00"
        );
    }

    #[test]
    fn build_server_connection_string_omits_database_for_create_like_cpp() {
        assert_eq!(
            build_server_connection_string_like_cpp(
                "127.0.0.1",
                "3306",
                "trinity",
                "trinity",
                false,
            ),
            "mysql://trinity:trinity@127.0.0.1:3306?ssl-mode=DISABLED&charset=utf8mb4&collation=utf8mb4_unicode_ci&timezone=%2B00%3A00"
        );
        assert_eq!(
            build_server_connection_string_like_cpp(
                ".",
                "/var/run/mysqld/mysqld.sock",
                "trinity",
                "trinity",
                true,
            ),
            "mysql://trinity:trinity@localhost?socket=/var/run/mysqld/mysqld.sock&ssl-mode=REQUIRED&charset=utf8mb4&collation=utf8mb4_unicode_ci&timezone=%2B00%3A00"
        );
    }

    #[test]
    fn mysql_identifier_escape_doubles_backticks_like_cpp_create() {
        assert_eq!(escape_mysql_identifier_like_cpp("world"), "world");
        assert_eq!(escape_mysql_identifier_like_cpp("bad`name"), "bad``name");
    }

    #[test]
    fn escape_string_matches_mysql_real_escape_string_special_bytes_like_cpp() {
        assert_eq!(escape_string_like_cpp(""), "");
        assert_eq!(
            escape_string_like_cpp("a\0b\nc\rd\\e'f\"g\u{1A}h"),
            "a\\0b\\nc\\rd\\\\e\\'f\\\"g\\Zh"
        );
        assert_eq!(escape_string_like_cpp("Grüße"), "Grüße");
    }

    #[tokio::test]
    async fn sync_query_warning_scope_is_task_local_like_cpp() {
        assert!(!warn_about_sync_queries_enabled_like_cpp());

        let scoped = warn_about_sync_queries_scope_like_cpp(async {
            warn_about_sync_queries_enabled_like_cpp()
        })
        .await;

        assert!(scoped);
        assert!(!warn_about_sync_queries_enabled_like_cpp());
    }
}
