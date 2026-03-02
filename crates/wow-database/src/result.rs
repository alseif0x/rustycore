//! Query result wrappers.
//!
//! [`SqlResult`] wraps a collection of rows returned by a query, providing a
//! cursor-style API that matches the C# `SQLResult` / `SQLFields` pattern.

use sqlx::mysql::MySqlRow;
use sqlx::{Column, Row, TypeInfo, ValueRef};

/// Result of a database query, holding zero or more rows.
///
/// Rows are accessed sequentially. The first row is available immediately
/// after construction (if any rows exist). Call [`next_row`](Self::next_row)
/// to advance to subsequent rows.
///
/// # Example (conceptual)
///
/// ```ignore
/// let result = db.query(stmt).await?;
/// if !result.is_empty() {
///     let name: String = result.read(0);
///     let level: i32 = result.read(1);
///     while result.next_row() {
///         // ...
///     }
/// }
/// ```
pub struct SqlResult {
    rows: Vec<MySqlRow>,
    current: usize,
}

impl SqlResult {
    /// Create from a vector of rows (typically from `fetch_all`).
    pub(crate) fn new(rows: Vec<MySqlRow>) -> Self {
        Self { rows, current: 0 }
    }

    /// Create an empty result (no rows).
    #[allow(dead_code)]
    pub(crate) fn empty() -> Self {
        Self {
            rows: Vec::new(),
            current: 0,
        }
    }

    /// Returns `true` if the query returned no rows.
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    /// Total number of rows in this result.
    pub fn count(&self) -> usize {
        self.rows.len()
    }

    /// Number of columns in the result (0 if empty).
    pub fn field_count(&self) -> usize {
        self.rows
            .first()
            .map_or(0, |r| r.columns().len())
    }

    /// Read a typed value from the current row at the given column index.
    ///
    /// # Panics
    ///
    /// Panics if the result is empty, the column index is out of range, or the
    /// value cannot be decoded as `T`.
    pub fn read<'r, T>(&'r self, column: usize) -> T
    where
        T: sqlx::Decode<'r, sqlx::MySql> + sqlx::Type<sqlx::MySql>,
    {
        self.rows[self.current].get(column)
    }

    /// Try to read a typed value, returning `None` on failure or `NULL`.
    pub fn try_read<'r, T>(&'r self, column: usize) -> Option<T>
    where
        T: sqlx::Decode<'r, sqlx::MySql> + sqlx::Type<sqlx::MySql>,
    {
        self.rows
            .get(self.current)
            .and_then(|row| row.try_get(column).ok())
    }

    /// Read a string column, handling MySQL binary collation (`VARBINARY`).
    ///
    /// MySQL columns with `COLLATE utf8mb4_bin` are reported as `VARBINARY` by
    /// sqlx, which makes `read::<String>()` fail. This method tries `String`
    /// first, then falls back to reading raw bytes and converting to UTF-8.
    pub fn read_string(&self, column: usize) -> String {
        if let Some(s) = self.try_read::<String>(column) {
            return s;
        }
        if let Some(bytes) = self.try_read::<Vec<u8>>(column) {
            return String::from_utf8_lossy(&bytes).into_owned();
        }
        String::new()
    }

    /// Check if a column in the current row is `NULL`.
    pub fn is_null(&self, column: usize) -> bool {
        self.rows
            .get(self.current)
            .and_then(|row| row.try_get_raw(column).ok())
            .is_none_or(|v| v.is_null())
    }

    /// Advance to the next row. Returns `false` when no more rows remain.
    pub fn next_row(&mut self) -> bool {
        if self.current + 1 < self.rows.len() {
            self.current += 1;
            true
        } else {
            false
        }
    }

    /// Get a snapshot of the current row as [`SqlFields`].
    pub fn fields(&self) -> SqlFields<'_> {
        SqlFields {
            row: &self.rows[self.current],
        }
    }

    /// Get the column name at the given index.
    pub fn column_name(&self, index: usize) -> Option<&str> {
        self.rows
            .first()
            .and_then(|r| r.columns().get(index))
            .map(Column::name)
    }

    /// Get the column type name at the given index.
    pub fn column_type_name(&self, index: usize) -> Option<&str> {
        self.rows
            .first()
            .and_then(|r| r.columns().get(index))
            .map(|c| c.type_info().name())
    }
}

impl std::fmt::Debug for SqlResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SqlResult")
            .field("row_count", &self.rows.len())
            .field("current", &self.current)
            .finish()
    }
}

/// A borrowed view of a single row, allowing typed column access.
///
/// Equivalent to the C# `SQLFields` class.
pub struct SqlFields<'a> {
    row: &'a MySqlRow,
}

impl<'a> SqlFields<'a> {
    /// Read a typed value from the given column index.
    pub fn read<T>(&self, column: usize) -> T
    where
        T: sqlx::Decode<'a, sqlx::MySql> + sqlx::Type<sqlx::MySql>,
    {
        self.row.get(column)
    }

    /// Try to read a typed value, returning `None` on failure or `NULL`.
    pub fn try_read<T>(&self, column: usize) -> Option<T>
    where
        T: sqlx::Decode<'a, sqlx::MySql> + sqlx::Type<sqlx::MySql>,
    {
        self.row.try_get(column).ok()
    }

    /// Read multiple columns of the same type into a `Vec`.
    pub fn read_values<T>(&self, start: usize, count: usize) -> Vec<T>
    where
        T: sqlx::Decode<'a, sqlx::MySql> + sqlx::Type<sqlx::MySql>,
    {
        (start..start + count)
            .map(|i| self.row.get(i))
            .collect()
    }

    /// Check if a column is `NULL`.
    pub fn is_null(&self, column: usize) -> bool {
        self.row
            .try_get_raw(column)
            .map_or(true, |v| v.is_null())
    }

    /// Number of columns in this row.
    pub fn field_count(&self) -> usize {
        self.row.columns().len()
    }
}
