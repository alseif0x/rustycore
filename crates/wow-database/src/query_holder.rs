//! SQL query holder support.
//!
//! TrinityCore's `SQLQueryHolder` is a fixed-size set of prepared-statement
//! slots. The holder is executed as one asynchronous DB task, but the task
//! itself walks the slots in order on one connection.

use crate::params::PreparedStatement;
use crate::result::SqlResult;

/// Fixed-size prepared-query holder mirroring TrinityCore `SQLQueryHolder<T>`.
#[derive(Debug, Default)]
pub struct SqlQueryHolder {
    queries: Vec<Option<PreparedStatement>>,
}

impl SqlQueryHolder {
    pub fn new(size: usize) -> Self {
        let mut holder = Self::default();
        holder.set_size(size);
        holder
    }

    /// Resize the holder slots, mirroring `SQLQueryHolderBase::SetSize`.
    pub fn set_size(&mut self, size: usize) {
        self.queries.resize_with(size, || None);
    }

    pub fn len(&self) -> usize {
        self.queries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.queries.is_empty()
    }

    /// Set a prepared query at a fixed slot.
    ///
    /// Returns `false` for out-of-range indexes, like C++ `SetPreparedQueryImpl`.
    pub fn set_prepared_query(&mut self, index: usize, stmt: PreparedStatement) -> bool {
        let Some(slot) = self.queries.get_mut(index) else {
            tracing::error!(
                target: "sql.sql",
                index,
                size = self.queries.len(),
                "Query index out of range for prepared statement"
            );
            return false;
        };

        *slot = Some(stmt);
        true
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = Option<&PreparedStatement>> {
        self.queries.iter().map(Option::as_ref)
    }
}

/// Results for a completed [`SqlQueryHolder`].
#[derive(Debug)]
pub struct SqlQueryHolderResult {
    results: Vec<Option<SqlResult>>,
}

impl SqlQueryHolderResult {
    pub(crate) fn new(results: Vec<Option<SqlResult>>) -> Self {
        Self { results }
    }

    pub fn len(&self) -> usize {
        self.results.len()
    }

    pub fn is_empty(&self) -> bool {
        self.results.is_empty()
    }

    /// Borrow a result by slot. Empty SQL results are represented as `None`,
    /// mirroring C++ `SetPreparedResult`, which stores a null result pointer
    /// when `GetRowCount() == 0`.
    pub fn get_prepared_result(&self, index: usize) -> Option<&SqlResult> {
        self.results.get(index).and_then(Option::as_ref)
    }

    pub fn into_prepared_result(mut self, index: usize) -> Option<SqlResult> {
        self.results.get_mut(index).and_then(Option::take)
    }
}

pub(crate) fn prepared_result_slot_like_cpp(result: SqlResult) -> Option<SqlResult> {
    (!result.is_empty()).then_some(result)
}

#[cfg(test)]
mod tests {
    use super::{SqlQueryHolder, prepared_result_slot_like_cpp};
    use crate::{PreparedStatement, result::SqlResult};

    #[test]
    fn holder_set_size_creates_fixed_slots_like_cpp() {
        let holder = SqlQueryHolder::new(3);

        assert_eq!(holder.len(), 3);
        assert_eq!(holder.iter().filter(Option::is_some).count(), 0);
    }

    #[test]
    fn holder_set_prepared_query_rejects_out_of_range_like_cpp() {
        let mut holder = SqlQueryHolder::new(1);

        assert!(holder.set_prepared_query(0, PreparedStatement::new("SELECT 1")));
        assert!(!holder.set_prepared_query(1, PreparedStatement::new("SELECT 2")));
        assert_eq!(holder.iter().filter(Option::is_some).count(), 1);
    }

    #[test]
    fn holder_empty_prepared_result_is_null_like_cpp() {
        assert!(prepared_result_slot_like_cpp(SqlResult::empty()).is_none());
    }
}
