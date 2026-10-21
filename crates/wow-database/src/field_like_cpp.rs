//! Fail-closed C++ `Field` getters for startup catalog adapters.
//!
//! C++ `Field::GetUInt8`/`GetInt32`/... (Field.cpp) return 0 for SQL `NULL`
//! and otherwise run `PrimitiveResultValueConverter::GetNumericValue`
//! (FieldValueConverters.h): the value is read in the column's own width
//! (`MysqlTypeToFieldType`, QueryResult.cpp), `static_cast` to the requested
//! width and accepted only if casting it back reproduces the column value.
//! A width that does not round-trip, or a numeric getter on a text/binary
//! column, reaches `LogTruncation` (FieldValueConverter.cpp), which is an
//! `ASSERT(false)`: the C++ server stops instead of loading a wrong value.
//!
//! sqlx instead rejects any signed/unsigned mismatch, and the former
//! `try_read(..).unwrap_or(0)` turned that rejection into a silent 0 (#1430,
//! #1436). These getters read the column at its MariaDB width, apply the C++
//! round-trip, and return an error where C++ asserts, so the catalog load
//! fails closed with the field name instead of storing a default.

use anyhow::{Result, anyhow, bail};

/// One column value at the width MariaDB reports for it.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum NativeFieldLikeCpp {
    Null,
    Int8(i8),
    UInt8(u8),
    Int16(i16),
    UInt16(u16),
    Int32(i32),
    UInt32(u32),
    Int64(i64),
    UInt64(u64),
    Float(f32),
    Double(f64),
    /// C++ `DatabaseFieldTypes::Binary`: CHAR/VARCHAR/TEXT/BLOB columns.
    Text(String),
    /// DECIMAL, DATE, BIT, ...: no C++ numeric/string getter used by these
    /// catalogs accepts them (`COALESCE(uint, literal)` is DECIMAL).
    Unsupported(String),
}

/// A C++ numeric `Field` getter width.
pub trait FieldNumberLikeCpp: Copy + Default {
    const GETTER: &'static str;

    /// `GetNumericValue`: `static_cast` plus the round-trip check; `None` is
    /// a truncation (or a non-numeric column) that C++ asserts on.
    fn cast_like_cpp(native: &NativeFieldLikeCpp) -> Option<Self>;
}

macro_rules! round_trip_like_cpp {
    ($source:expr, $database:ty, $target:ty) => {{
        let source: $database = $source;
        let result = source as $target;
        #[allow(clippy::float_cmp)]
        let exact = (result as $database) == source;
        exact.then_some(result)
    }};
}

macro_rules! field_number_like_cpp {
    ($($target:ty => $getter:literal),* $(,)?) => {$(
        impl FieldNumberLikeCpp for $target {
            const GETTER: &'static str = $getter;

            #[allow(
                clippy::cast_lossless,
                clippy::cast_possible_truncation,
                clippy::cast_possible_wrap,
                clippy::cast_precision_loss,
                clippy::cast_sign_loss,
                clippy::unnecessary_cast
            )]
            fn cast_like_cpp(native: &NativeFieldLikeCpp) -> Option<Self> {
                match *native {
                    NativeFieldLikeCpp::Int8(value) => round_trip_like_cpp!(value, i8, $target),
                    NativeFieldLikeCpp::UInt8(value) => round_trip_like_cpp!(value, u8, $target),
                    NativeFieldLikeCpp::Int16(value) => round_trip_like_cpp!(value, i16, $target),
                    NativeFieldLikeCpp::UInt16(value) => round_trip_like_cpp!(value, u16, $target),
                    NativeFieldLikeCpp::Int32(value) => round_trip_like_cpp!(value, i32, $target),
                    NativeFieldLikeCpp::UInt32(value) => round_trip_like_cpp!(value, u32, $target),
                    NativeFieldLikeCpp::Int64(value) => round_trip_like_cpp!(value, i64, $target),
                    NativeFieldLikeCpp::UInt64(value) => round_trip_like_cpp!(value, u64, $target),
                    NativeFieldLikeCpp::Float(value) => round_trip_like_cpp!(value, f32, $target),
                    NativeFieldLikeCpp::Double(value) => round_trip_like_cpp!(value, f64, $target),
                    NativeFieldLikeCpp::Null
                    | NativeFieldLikeCpp::Text(_)
                    | NativeFieldLikeCpp::Unsupported(_) => None,
                }
            }
        }
    )*};
}

field_number_like_cpp!(
    u8 => "Field::GetUInt8",
    i8 => "Field::GetInt8",
    u16 => "Field::GetUInt16",
    i16 => "Field::GetInt16",
    u32 => "Field::GetUInt32",
    i32 => "Field::GetInt32",
    u64 => "Field::GetUInt64",
    i64 => "Field::GetInt64",
    f32 => "Field::GetFloat",
    f64 => "Field::GetDouble",
);

/// A result row whose columns can be read at their MariaDB width.
pub trait FieldRowLikeCpp {
    fn native_field_like_cpp(&self, column: usize) -> Result<NativeFieldLikeCpp>;

    /// C++ `Field::GetUInt32()` & co.: `NULL` is 0, a truncation is an error.
    fn get_like_cpp<T: FieldNumberLikeCpp>(&self, column: usize, field: &str) -> Result<T> {
        Ok(self
            .get_nullable_like_cpp(column, field)?
            .unwrap_or_default())
    }

    /// `IsNull()` followed by the numeric getter, for nullable LEFT JOIN columns.
    fn get_nullable_like_cpp<T: FieldNumberLikeCpp>(
        &self,
        column: usize,
        field: &str,
    ) -> Result<Option<T>> {
        let native = self.native_field_like_cpp(column)?;
        if native == NativeFieldLikeCpp::Null {
            return Ok(None);
        }
        T::cast_like_cpp(&native).map(Some).ok_or_else(|| {
            anyhow!(
                "{} on {field} (column {column}) holding {native:?} would truncate the value (C++ LogTruncation)",
                T::GETTER
            )
        })
    }

    /// C++ `Field::GetBool()`: `GetUInt8() == 1`.
    fn get_bool_like_cpp(&self, column: usize, field: &str) -> Result<bool> {
        Ok(self.get_like_cpp::<u8>(column, field)? == 1)
    }

    /// `IsNull()` followed by `GetBool()`.
    fn get_nullable_bool_like_cpp(&self, column: usize, field: &str) -> Result<Option<bool>> {
        Ok(self
            .get_nullable_like_cpp::<u8>(column, field)?
            .map(|value| value == 1))
    }

    /// C++ `Field::GetString()`: `NULL` is empty; a numeric column asserts.
    fn get_string_like_cpp(&self, column: usize, field: &str) -> Result<String> {
        match self.native_field_like_cpp(column)? {
            NativeFieldLikeCpp::Null => Ok(String::new()),
            NativeFieldLikeCpp::Text(text) => Ok(text),
            native => bail!(
                "Field::GetString on {field} (column {column}) holding {native:?} is not a string column (C++ LogTruncation)"
            ),
        }
    }
}

/// The C++ `MysqlTypeToFieldType` width (QueryResult.cpp) of a sqlx column
/// type name; text/binary columns are read with `GetString()`.
pub(crate) fn native_width_like_cpp(type_name: &str) -> NativeWidthLikeCpp {
    match type_name {
        "TINYINT UNSIGNED" => NativeWidthLikeCpp::UInt8,
        "TINYINT" => NativeWidthLikeCpp::Int8,
        // sqlx names every TINYINT(1) BOOLEAN, signed or not.
        "BOOLEAN" => NativeWidthLikeCpp::Boolean,
        "SMALLINT UNSIGNED" => NativeWidthLikeCpp::UInt16,
        "SMALLINT" => NativeWidthLikeCpp::Int16,
        "INT UNSIGNED" | "MEDIUMINT UNSIGNED" => NativeWidthLikeCpp::UInt32,
        "INT" | "MEDIUMINT" => NativeWidthLikeCpp::Int32,
        "BIGINT UNSIGNED" => NativeWidthLikeCpp::UInt64,
        "BIGINT" => NativeWidthLikeCpp::Int64,
        "FLOAT" => NativeWidthLikeCpp::Float,
        "DOUBLE" => NativeWidthLikeCpp::Double,
        "CHAR" | "VARCHAR" | "BINARY" | "VARBINARY" | "ENUM" | "TINYTEXT" | "TEXT"
        | "MEDIUMTEXT" | "LONGTEXT" | "TINYBLOB" | "BLOB" | "MEDIUMBLOB" | "LONGBLOB" => {
            NativeWidthLikeCpp::Text
        }
        _ => NativeWidthLikeCpp::Unsupported,
    }
}

/// How a column is decoded before a C++ getter converts it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NativeWidthLikeCpp {
    Int8,
    UInt8,
    Boolean,
    Int16,
    UInt16,
    Int32,
    UInt32,
    Int64,
    UInt64,
    Float,
    Double,
    Text,
    Unsupported,
}

/// A column the result does not have.
pub(crate) fn missing_column_error_like_cpp(column: usize, field_count: usize) -> anyhow::Error {
    anyhow!("column {column} is outside the {field_count}-column result")
}

/// A column whose reported type could not be decoded at that width.
pub(crate) fn undecodable_column_error_like_cpp(column: usize, type_name: &str) -> anyhow::Error {
    anyhow!("column {column} ({type_name}) could not be decoded")
}

/// Prefix a row-mapper error with the table and row index.
pub(crate) fn row_error_like_cpp(table: &str, row: usize, error: &anyhow::Error) -> anyhow::Error {
    anyhow!("{table} row {row}: {error}")
}

/// A decoded result row: each cell is exactly what MariaDB reports for that
/// column (width, signedness, `NULL`), so row mappers are schema-free testable.
impl FieldRowLikeCpp for [NativeFieldLikeCpp] {
    fn native_field_like_cpp(&self, column: usize) -> Result<NativeFieldLikeCpp> {
        self.get(column)
            .cloned()
            .ok_or_else(|| anyhow!("column {column} is outside the {}-column row", self.len()))
    }
}

#[cfg(test)]
mod tests {
    use super::NativeFieldLikeCpp as Native;
    use super::*;

    #[test]
    fn unsigned_columns_reach_cpp_getters_of_any_signedness() {
        // creature_template.type / Classification: TINYINT UNSIGNED.
        assert_eq!(u8::cast_like_cpp(&Native::UInt8(7)), Some(7));
        assert_eq!(i32::cast_like_cpp(&Native::UInt8(4)), Some(4));
        // KillCredit1/movementId: INT UNSIGNED read through GetUInt32.
        assert_eq!(
            u32::cast_like_cpp(&Native::UInt32(4_000_000_000)),
            Some(4_000_000_000)
        );
        // C++ `static_cast<int32>(uint32)` keeps the bits when the round
        // trip through the column width reproduces the value.
        assert_eq!(i32::cast_like_cpp(&Native::UInt32(u32::MAX)), Some(-1));
    }

    #[test]
    fn signed_columns_keep_cpp_bit_casts_and_reject_truncation() {
        // gameobject_template.Data0..34 is INT; C++ reads it with GetUInt32.
        assert_eq!(u32::cast_like_cpp(&Native::Int32(-1)), Some(u32::MAX));
        assert_eq!(u32::cast_like_cpp(&Native::Int32(2278)), Some(2278));
        // creature_template_difficulty.MinLevel is TINYINT; GetUInt8.
        assert_eq!(u8::cast_like_cpp(&Native::Int8(75)), Some(75));
        assert_eq!(u8::cast_like_cpp(&Native::Int8(-1)), Some(u8::MAX));
        // creature_template.RequiredExpansion is INT; a narrower getter
        // asserts in C++ when the value does not fit.
        assert_eq!(u8::cast_like_cpp(&Native::Int32(2)), Some(2));
        assert_eq!(u8::cast_like_cpp(&Native::Int32(300)), None);
        assert_eq!(u16::cast_like_cpp(&Native::UInt32(70_000)), None);
        assert_eq!(u32::cast_like_cpp(&Native::Int64(-1)), None);
    }

    #[test]
    fn float_columns_round_trip_like_cpp() {
        assert_eq!(f32::cast_like_cpp(&Native::Float(1.5)), Some(1.5));
        assert_eq!(f32::cast_like_cpp(&Native::Double(0.5)), Some(0.5));
        assert_eq!(f32::cast_like_cpp(&Native::Double(0.1)), None);
        assert_eq!(u32::cast_like_cpp(&Native::Float(1.5)), None);
        assert_eq!(f32::cast_like_cpp(&Native::Float(f32::NAN)), None);
    }

    #[test]
    fn null_text_and_decimal_columns_follow_cpp_field_getters() {
        let row = [
            Native::Null,
            Native::Text("Hogger".into()),
            Native::Unsupported("DECIMAL".into()),
            Native::UInt8(1),
            Native::UInt8(2),
        ];
        let row = &row[..];
        assert_eq!(row.get_like_cpp::<u32>(0, "null").unwrap(), 0);
        assert_eq!(row.get_nullable_like_cpp::<u8>(0, "null").unwrap(), None);
        assert_eq!(row.get_string_like_cpp(0, "null").unwrap(), "");
        assert_eq!(row.get_string_like_cpp(1, "name").unwrap(), "Hogger");
        assert!(row.get_like_cpp::<u32>(1, "name").is_err());
        let error = row
            .get_like_cpp::<u8>(2, "COALESCE(ctm.Ground, 1)")
            .unwrap_err();
        assert!(error.to_string().contains("Field::GetUInt8"), "{error}");
        assert!(
            error.to_string().contains("COALESCE(ctm.Ground, 1)"),
            "{error}"
        );
        assert!(row.get_string_like_cpp(3, "flag").is_err());
        // GetBool is `GetUInt8() == 1`.
        assert!(row.get_bool_like_cpp(3, "flag").unwrap());
        assert!(!row.get_bool_like_cpp(4, "flag").unwrap());
        assert!(row.get_like_cpp::<u8>(5, "missing").is_err());
    }
}
