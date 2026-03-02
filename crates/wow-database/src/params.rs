//! Dynamic SQL parameter types for prepared statements.

/// A dynamically-typed SQL parameter value.
///
/// Used to collect bind parameters before executing a query, matching the
/// C# `PreparedStatement.SetXxx(index, value)` pattern.
#[derive(Debug, Clone)]
pub enum SqlParam {
    Null,
    Bool(bool),
    I8(i8),
    U8(u8),
    I16(i16),
    U16(u16),
    I32(i32),
    U32(u32),
    I64(i64),
    U64(u64),
    F32(f32),
    F64(f64),
    String(String),
    Bytes(Vec<u8>),
}

/// A prepared statement with SQL text and collected parameters.
///
/// Parameters are set by index (0-based) matching the `?` placeholders in the
/// SQL string. The statement is then passed to [`Database::query`] or
/// [`Database::execute`].
///
/// [`Database::query`]: crate::Database::query
/// [`Database::execute`]: crate::Database::execute
#[derive(Debug, Clone)]
pub struct PreparedStatement {
    sql: &'static str,
    params: Vec<SqlParam>,
}

impl PreparedStatement {
    /// Create a new prepared statement from a static SQL string.
    pub(crate) fn new(sql: &'static str) -> Self {
        Self {
            sql,
            params: Vec::new(),
        }
    }

    /// The SQL text of this statement.
    pub fn sql(&self) -> &'static str {
        self.sql
    }

    /// The collected parameters, in index order.
    pub fn params(&self) -> &[SqlParam] {
        &self.params
    }

    // -- Typed setters matching C# PreparedStatement API ----------------------

    fn ensure_capacity(&mut self, index: usize) {
        if self.params.len() <= index {
            self.params.resize(index + 1, SqlParam::Null);
        }
    }

    pub fn set_bool(&mut self, index: usize, value: bool) {
        self.ensure_capacity(index);
        self.params[index] = SqlParam::Bool(value);
    }

    pub fn set_i8(&mut self, index: usize, value: i8) {
        self.ensure_capacity(index);
        self.params[index] = SqlParam::I8(value);
    }

    pub fn set_u8(&mut self, index: usize, value: u8) {
        self.ensure_capacity(index);
        self.params[index] = SqlParam::U8(value);
    }

    pub fn set_i16(&mut self, index: usize, value: i16) {
        self.ensure_capacity(index);
        self.params[index] = SqlParam::I16(value);
    }

    pub fn set_u16(&mut self, index: usize, value: u16) {
        self.ensure_capacity(index);
        self.params[index] = SqlParam::U16(value);
    }

    pub fn set_i32(&mut self, index: usize, value: i32) {
        self.ensure_capacity(index);
        self.params[index] = SqlParam::I32(value);
    }

    pub fn set_u32(&mut self, index: usize, value: u32) {
        self.ensure_capacity(index);
        self.params[index] = SqlParam::U32(value);
    }

    pub fn set_i64(&mut self, index: usize, value: i64) {
        self.ensure_capacity(index);
        self.params[index] = SqlParam::I64(value);
    }

    pub fn set_u64(&mut self, index: usize, value: u64) {
        self.ensure_capacity(index);
        self.params[index] = SqlParam::U64(value);
    }

    pub fn set_f32(&mut self, index: usize, value: f32) {
        self.ensure_capacity(index);
        self.params[index] = SqlParam::F32(value);
    }

    pub fn set_f64(&mut self, index: usize, value: f64) {
        self.ensure_capacity(index);
        self.params[index] = SqlParam::F64(value);
    }

    pub fn set_string(&mut self, index: usize, value: impl Into<String>) {
        self.ensure_capacity(index);
        self.params[index] = SqlParam::String(value.into());
    }

    pub fn set_bytes(&mut self, index: usize, value: Vec<u8>) {
        self.ensure_capacity(index);
        self.params[index] = SqlParam::Bytes(value);
    }

    pub fn set_null(&mut self, index: usize) {
        self.ensure_capacity(index);
        self.params[index] = SqlParam::Null;
    }

    /// Reset all parameters.
    pub fn clear(&mut self) {
        self.params.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prepared_statement_set_params() {
        let mut stmt = PreparedStatement::new("SELECT * FROM foo WHERE id = ? AND name = ?");
        stmt.set_u32(0, 42);
        stmt.set_string(1, "hello");

        assert_eq!(stmt.sql(), "SELECT * FROM foo WHERE id = ? AND name = ?");
        assert_eq!(stmt.params().len(), 2);
        assert!(matches!(stmt.params()[0], SqlParam::U32(42)));
        assert!(matches!(&stmt.params()[1], SqlParam::String(s) if s == "hello"));
    }

    #[test]
    fn prepared_statement_sparse_indices() {
        let mut stmt = PreparedStatement::new("INSERT INTO foo VALUES (?, ?, ?)");
        // Set index 2 first (skip 0, 1)
        stmt.set_i64(2, 999);
        assert_eq!(stmt.params().len(), 3);
        assert!(matches!(stmt.params()[0], SqlParam::Null));
        assert!(matches!(stmt.params()[1], SqlParam::Null));
        assert!(matches!(stmt.params()[2], SqlParam::I64(999)));
    }

    #[test]
    fn prepared_statement_overwrite_param() {
        let mut stmt = PreparedStatement::new("SELECT ?");
        stmt.set_u32(0, 1);
        stmt.set_u32(0, 2);
        assert!(matches!(stmt.params()[0], SqlParam::U32(2)));
    }

    #[test]
    fn prepared_statement_clear() {
        let mut stmt = PreparedStatement::new("SELECT ?");
        stmt.set_u32(0, 1);
        assert_eq!(stmt.params().len(), 1);
        stmt.clear();
        assert!(stmt.params().is_empty());
    }

    #[test]
    fn prepared_statement_all_types() {
        let mut stmt = PreparedStatement::new("");
        stmt.set_bool(0, true);
        stmt.set_i8(1, -1);
        stmt.set_u8(2, 255);
        stmt.set_i16(3, -1000);
        stmt.set_u16(4, 60000);
        stmt.set_i32(5, -100_000);
        stmt.set_u32(6, 4_000_000);
        stmt.set_i64(7, -1_000_000_000);
        stmt.set_u64(8, 9_999_999_999);
        stmt.set_f32(9, 3.14);
        stmt.set_f64(10, 2.71828);
        stmt.set_string(11, "test");
        stmt.set_bytes(12, vec![0xDE, 0xAD]);
        stmt.set_null(13);

        assert_eq!(stmt.params().len(), 14);
        assert!(matches!(stmt.params()[0], SqlParam::Bool(true)));
        assert!(matches!(stmt.params()[13], SqlParam::Null));
    }
}
