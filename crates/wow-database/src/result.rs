//! Query result wrappers.
//!
//! [`SqlResult`] wraps a collection of rows returned by a query, providing a
//! cursor-style API that matches the C# `SQLResult` / `SQLFields` pattern.

use std::any::TypeId;

use sqlx::MySql;
use sqlx::mysql::MySqlRow;
use sqlx::{Column, Row, TypeInfo, ValueRef};

/// TrinityCore-style database field categories derived from MySQL metadata.
///
/// C++ stores this as `DatabaseFieldTypes` on `QueryResultFieldMetadata`.
/// RustyCore receives type names from sqlx, so this enum is a compatibility
/// classifier used by [`read_typed`](SqlResult::read_typed) and friends.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DatabaseFieldTypeLikeCpp {
    Null,
    UInt8,
    Int8,
    UInt16,
    Int16,
    UInt32,
    Int32,
    UInt64,
    Int64,
    Float,
    Double,
    Decimal,
    Date,
    Binary,
    Text,
}

/// Classify a sqlx/MySQL type-name into the closest TC `DatabaseFieldTypes`
/// category.
pub fn database_field_type_like_cpp(type_name: &str) -> DatabaseFieldTypeLikeCpp {
    let upper = type_name.to_ascii_uppercase();
    let unsigned = upper.contains("UNSIGNED");

    if upper == "BIT" || upper.contains("BINARY") || upper.contains("BLOB") || upper == "GEOMETRY" {
        DatabaseFieldTypeLikeCpp::Binary
    } else if upper == "YEAR" {
        DatabaseFieldTypeLikeCpp::UInt16
    } else if upper.contains("TINYINT") || upper == "TINY" || upper == "UNSIGNED TINY" {
        if unsigned {
            DatabaseFieldTypeLikeCpp::UInt8
        } else {
            DatabaseFieldTypeLikeCpp::Int8
        }
    } else if upper.contains("SMALLINT") || upper == "SHORT" || upper == "UNSIGNED SHORT" {
        if unsigned {
            DatabaseFieldTypeLikeCpp::UInt16
        } else {
            DatabaseFieldTypeLikeCpp::Int16
        }
    } else if upper.contains("BIGINT") || upper == "LONGLONG" || upper == "UNSIGNED LONGLONG" {
        if unsigned {
            DatabaseFieldTypeLikeCpp::UInt64
        } else {
            DatabaseFieldTypeLikeCpp::Int64
        }
    } else if upper.contains("MEDIUMINT")
        || upper.contains("INT")
        || upper == "INT24"
        || upper == "UNSIGNED INT24"
        || upper == "LONG"
        || upper == "UNSIGNED LONG"
    {
        if unsigned {
            DatabaseFieldTypeLikeCpp::UInt32
        } else {
            DatabaseFieldTypeLikeCpp::Int32
        }
    } else if upper.contains("FLOAT") {
        DatabaseFieldTypeLikeCpp::Float
    } else if upper.contains("DOUBLE") {
        DatabaseFieldTypeLikeCpp::Double
    } else if upper.contains("DECIMAL") || upper.contains("NUMERIC") {
        DatabaseFieldTypeLikeCpp::Decimal
    } else if upper.contains("CHAR")
        || upper.contains("TEXT")
        || upper.contains("ENUM")
        || upper.contains("SET")
        || upper.contains("JSON")
        || upper == "STRING"
        || upper == "VAR_STRING"
    {
        DatabaseFieldTypeLikeCpp::Text
    } else if upper.contains("DATE") || upper.contains("TIME") || upper.contains("TIMESTAMP") {
        DatabaseFieldTypeLikeCpp::Date
    } else if upper.contains("NULL") {
        DatabaseFieldTypeLikeCpp::Null
    } else {
        DatabaseFieldTypeLikeCpp::Text
    }
}

/// Return whether a Rust target type matches the TC-style DB field metadata.
pub fn rust_type_compatible_with_database_field_like_cpp<T: 'static>(type_name: &str) -> bool {
    let rust_type = TypeId::of::<T>();

    match database_field_type_like_cpp(type_name) {
        DatabaseFieldTypeLikeCpp::Null => false,
        DatabaseFieldTypeLikeCpp::UInt8 => {
            rust_type == TypeId::of::<u8>() || rust_type == TypeId::of::<bool>()
        }
        DatabaseFieldTypeLikeCpp::Int8 => {
            rust_type == TypeId::of::<i8>() || rust_type == TypeId::of::<bool>()
        }
        DatabaseFieldTypeLikeCpp::UInt16 => rust_type == TypeId::of::<u16>(),
        DatabaseFieldTypeLikeCpp::Int16 => rust_type == TypeId::of::<i16>(),
        DatabaseFieldTypeLikeCpp::UInt32 => rust_type == TypeId::of::<u32>(),
        DatabaseFieldTypeLikeCpp::Int32 => rust_type == TypeId::of::<i32>(),
        DatabaseFieldTypeLikeCpp::UInt64 => rust_type == TypeId::of::<u64>(),
        DatabaseFieldTypeLikeCpp::Int64 => rust_type == TypeId::of::<i64>(),
        DatabaseFieldTypeLikeCpp::Float => rust_type == TypeId::of::<f32>(),
        DatabaseFieldTypeLikeCpp::Double | DatabaseFieldTypeLikeCpp::Decimal => {
            rust_type == TypeId::of::<f64>()
        }
        DatabaseFieldTypeLikeCpp::Date | DatabaseFieldTypeLikeCpp::Text => {
            rust_type == TypeId::of::<String>()
        }
        DatabaseFieldTypeLikeCpp::Binary => rust_type == TypeId::of::<Vec<u8>>(),
    }
}

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

    /// Total number of rows, named after TC `ResultSet::GetRowCount`.
    pub fn row_count_like_cpp(&self) -> usize {
        self.count()
    }

    /// Total number of rows, named after TC `ResultSet::GetRowCount`.
    pub fn get_row_count_like_cpp(&self) -> usize {
        self.count()
    }

    /// Number of columns in the result (0 if empty).
    pub fn field_count(&self) -> usize {
        self.rows.first().map_or(0, |r| r.columns().len())
    }

    /// Number of columns, named after TC `ResultSet::GetFieldCount`.
    pub fn get_field_count_like_cpp(&self) -> usize {
        self.field_count()
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

    /// Read a typed value after checking the column metadata like TC `Field`.
    ///
    /// This is intentionally stricter than [`read`](Self::read): a DB type /
    /// Rust type mismatch is reported before decoding.
    ///
    /// # Panics
    ///
    /// Panics if the result is empty, the column index is out of range, the
    /// metadata does not match `T`, or the value cannot be decoded as `T`.
    pub fn read_typed<'r, T>(&'r self, column: usize) -> T
    where
        T: sqlx::Decode<'r, MySql> + sqlx::Type<MySql> + 'static,
    {
        let type_name = self.column_type_name(column).unwrap_or("<out-of-range>");
        if !rust_type_compatible_with_database_field_like_cpp::<T>(type_name) {
            tracing::warn!(
                target: "sql.sql",
                column,
                db_type = type_name,
                rust_type = std::any::type_name::<T>(),
                "Database field type mismatch"
            );
            panic!(
                "database field type mismatch at column {column}: DB type {type_name:?} cannot be read as {}",
                std::any::type_name::<T>()
            );
        }

        self.read(column)
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

    /// Try to read a typed value after checking column metadata.
    pub fn try_read_typed<'r, T>(&'r self, column: usize) -> Option<T>
    where
        T: sqlx::Decode<'r, MySql> + sqlx::Type<MySql> + 'static,
    {
        let type_name = self.column_type_name(column)?;
        if !rust_type_compatible_with_database_field_like_cpp::<T>(type_name) {
            tracing::warn!(
                target: "sql.sql",
                column,
                db_type = type_name,
                rust_type = std::any::type_name::<T>(),
                "Database field type mismatch"
            );
            return None;
        }

        self.try_read(column)
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

    /// Fetch the current row like TC `ResultSet::Fetch`.
    pub fn fetch_like_cpp(&self) -> Option<SqlFields<'_>> {
        self.rows.get(self.current).map(|row| SqlFields { row })
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

    /// Read a typed value after checking the column metadata like TC `Field`.
    pub fn read_typed<T>(&self, column: usize) -> T
    where
        T: sqlx::Decode<'a, MySql> + sqlx::Type<MySql> + 'static,
    {
        let type_name = self.column_type_name(column).unwrap_or("<out-of-range>");
        if !rust_type_compatible_with_database_field_like_cpp::<T>(type_name) {
            tracing::warn!(
                target: "sql.sql",
                column,
                db_type = type_name,
                rust_type = std::any::type_name::<T>(),
                "Database field type mismatch"
            );
            panic!(
                "database field type mismatch at column {column}: DB type {type_name:?} cannot be read as {}",
                std::any::type_name::<T>()
            );
        }

        self.read(column)
    }

    /// Try to read a typed value, returning `None` on failure or `NULL`.
    pub fn try_read<T>(&self, column: usize) -> Option<T>
    where
        T: sqlx::Decode<'a, sqlx::MySql> + sqlx::Type<sqlx::MySql>,
    {
        self.row.try_get(column).ok()
    }

    /// Try to read a typed value after checking column metadata.
    pub fn try_read_typed<T>(&self, column: usize) -> Option<T>
    where
        T: sqlx::Decode<'a, MySql> + sqlx::Type<MySql> + 'static,
    {
        let type_name = self.column_type_name(column)?;
        if !rust_type_compatible_with_database_field_like_cpp::<T>(type_name) {
            tracing::warn!(
                target: "sql.sql",
                column,
                db_type = type_name,
                rust_type = std::any::type_name::<T>(),
                "Database field type mismatch"
            );
            return None;
        }

        self.try_read(column)
    }

    /// Read multiple columns of the same type into a `Vec`.
    pub fn read_values<T>(&self, start: usize, count: usize) -> Vec<T>
    where
        T: sqlx::Decode<'a, sqlx::MySql> + sqlx::Type<sqlx::MySql>,
    {
        (start..start + count).map(|i| self.row.get(i)).collect()
    }

    /// Check if a column is `NULL`.
    pub fn is_null(&self, column: usize) -> bool {
        self.row.try_get_raw(column).map_or(true, |v| v.is_null())
    }

    /// Number of columns in this row.
    pub fn field_count(&self) -> usize {
        self.row.columns().len()
    }

    /// Get the column type name at the given index.
    pub fn column_type_name(&self, index: usize) -> Option<&str> {
        self.row.columns().get(index).map(|c| c.type_info().name())
    }
}

#[cfg(test)]
mod tests {
    use super::{
        DatabaseFieldTypeLikeCpp, database_field_type_like_cpp,
        rust_type_compatible_with_database_field_like_cpp,
    };

    #[test]
    fn classifies_database_field_types_like_cpp() {
        assert_eq!(
            database_field_type_like_cpp("TINYINT UNSIGNED"),
            DatabaseFieldTypeLikeCpp::UInt8
        );
        assert_eq!(
            database_field_type_like_cpp("UNSIGNED TINY"),
            DatabaseFieldTypeLikeCpp::UInt8
        );
        assert_eq!(
            database_field_type_like_cpp("TINYINT"),
            DatabaseFieldTypeLikeCpp::Int8
        );
        assert_eq!(
            database_field_type_like_cpp("SMALLINT UNSIGNED"),
            DatabaseFieldTypeLikeCpp::UInt16
        );
        assert_eq!(
            database_field_type_like_cpp("UNSIGNED SHORT"),
            DatabaseFieldTypeLikeCpp::UInt16
        );
        assert_eq!(
            database_field_type_like_cpp("YEAR"),
            DatabaseFieldTypeLikeCpp::UInt16
        );
        assert_eq!(
            database_field_type_like_cpp("MEDIUMINT"),
            DatabaseFieldTypeLikeCpp::Int32
        );
        assert_eq!(
            database_field_type_like_cpp("INT UNSIGNED"),
            DatabaseFieldTypeLikeCpp::UInt32
        );
        assert_eq!(
            database_field_type_like_cpp("UNSIGNED LONG"),
            DatabaseFieldTypeLikeCpp::UInt32
        );
        assert_eq!(
            database_field_type_like_cpp("BIGINT"),
            DatabaseFieldTypeLikeCpp::Int64
        );
        assert_eq!(
            database_field_type_like_cpp("UNSIGNED LONGLONG"),
            DatabaseFieldTypeLikeCpp::UInt64
        );
        assert_eq!(
            database_field_type_like_cpp("DOUBLE"),
            DatabaseFieldTypeLikeCpp::Double
        );
        assert_eq!(
            database_field_type_like_cpp("DECIMAL"),
            DatabaseFieldTypeLikeCpp::Decimal
        );
        assert_eq!(
            database_field_type_like_cpp("VARBINARY"),
            DatabaseFieldTypeLikeCpp::Binary
        );
        assert_eq!(
            database_field_type_like_cpp("LONG_BLOB"),
            DatabaseFieldTypeLikeCpp::Binary
        );
        assert_eq!(
            database_field_type_like_cpp("BIT"),
            DatabaseFieldTypeLikeCpp::Binary
        );
        assert_eq!(
            database_field_type_like_cpp("VARCHAR"),
            DatabaseFieldTypeLikeCpp::Text
        );
        assert_eq!(
            database_field_type_like_cpp("VAR_STRING"),
            DatabaseFieldTypeLikeCpp::Text
        );
    }

    #[test]
    fn validates_rust_getter_types_like_cpp() {
        assert!(rust_type_compatible_with_database_field_like_cpp::<u8>(
            "TINYINT UNSIGNED"
        ));
        assert!(rust_type_compatible_with_database_field_like_cpp::<bool>(
            "TINYINT"
        ));
        assert!(!rust_type_compatible_with_database_field_like_cpp::<u8>(
            "SMALLINT UNSIGNED"
        ));
        assert!(rust_type_compatible_with_database_field_like_cpp::<i16>(
            "SMALLINT"
        ));
        assert!(rust_type_compatible_with_database_field_like_cpp::<u16>(
            "YEAR"
        ));
        assert!(rust_type_compatible_with_database_field_like_cpp::<u32>(
            "INT UNSIGNED"
        ));
        assert!(rust_type_compatible_with_database_field_like_cpp::<u32>(
            "UNSIGNED LONG"
        ));
        assert!(rust_type_compatible_with_database_field_like_cpp::<i64>(
            "BIGINT"
        ));
        assert!(rust_type_compatible_with_database_field_like_cpp::<u64>(
            "UNSIGNED LONGLONG"
        ));
        assert!(rust_type_compatible_with_database_field_like_cpp::<f32>(
            "FLOAT"
        ));
        assert!(rust_type_compatible_with_database_field_like_cpp::<f64>(
            "DOUBLE"
        ));
        assert!(rust_type_compatible_with_database_field_like_cpp::<f64>(
            "DECIMAL"
        ));
        assert!(rust_type_compatible_with_database_field_like_cpp::<String>(
            "TEXT"
        ));
        assert!(rust_type_compatible_with_database_field_like_cpp::<Vec<u8>>("BLOB"));
    }
}
