//! Database error types.

/// Errors that can occur during database operations.
#[derive(Debug, thiserror::Error)]
pub enum DatabaseError {
    /// A query or execution failed.
    #[error("query failed: {0}")]
    Query(sqlx::Error),

    /// A table required by the current query does not exist.
    #[error("table missing or database structure is out of date: {0}")]
    TableMissing(String),

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

impl From<sqlx::Error> for DatabaseError {
    fn from(error: sqlx::Error) -> Self {
        Self::from_sqlx_like_cpp(error)
    }
}

impl DatabaseError {
    pub fn from_sqlx_like_cpp(error: sqlx::Error) -> Self {
        if is_table_missing_sqlx_error_like_cpp(&error) {
            return Self::TableMissing(error.to_string());
        }

        Self::Query(error)
    }
}

fn is_table_missing_sqlx_error_like_cpp(error: &sqlx::Error) -> bool {
    match error {
        sqlx::Error::Database(db_err) => {
            is_table_missing_code_like_cpp(db_err.code().as_deref())
                || db_err.message().contains("doesn't exist")
        }
        _ => false,
    }
}

fn is_table_missing_code_like_cpp(code: Option<&str>) -> bool {
    matches!(code, Some("1146") | Some("42S02"))
}

#[cfg(test)]
mod tests {
    use super::is_table_missing_code_like_cpp;

    #[test]
    fn table_missing_codes_match_mysql_like_cpp() {
        assert!(is_table_missing_code_like_cpp(Some("1146")));
        assert!(is_table_missing_code_like_cpp(Some("42S02")));
        assert!(!is_table_missing_code_like_cpp(Some("1213")));
        assert!(!is_table_missing_code_like_cpp(None));
    }
}
