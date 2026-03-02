//! Database error types.

/// Errors that can occur during database operations.
#[derive(Debug, thiserror::Error)]
pub enum DatabaseError {
    /// A query or execution failed.
    #[error("query failed: {0}")]
    Query(#[from] sqlx::Error),

    /// The connection pool could not be created.
    #[error("failed to connect: {0}")]
    Connection(String),

    /// A statement was not registered (empty SQL).
    #[error("statement index {0} has no registered SQL")]
    UnregisteredStatement(usize),

    /// Transaction commit failed.
    #[error("transaction failed: {0}")]
    Transaction(String),
}
