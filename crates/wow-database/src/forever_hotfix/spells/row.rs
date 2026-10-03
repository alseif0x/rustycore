//! Checked target SQL primitive/text cursor, private to this spell adapter.
use super::LoadError;
use crate::SqlResult;

pub(super) struct SpellRow<'a> {
    row: &'a SqlResult,
    next: usize,
}
impl<'a> SpellRow<'a> {
    pub(super) fn new(row: &'a SqlResult, columns: usize) -> Result<Self, LoadError> {
        if row.field_count() != columns {
            return Err(LoadError::InvalidRow);
        }
        Ok(Self { row, next: 0 })
    }
    pub(super) fn read<T: for<'r> sqlx::Decode<'r, sqlx::MySql> + sqlx::Type<sqlx::MySql>>(
        &mut self,
    ) -> Result<T, LoadError> {
        let value = self.row.try_read(self.next).ok_or(LoadError::InvalidRow)?;
        self.next += 1;
        Ok(value)
    }
    pub(super) fn array<
        T: Copy + Default + for<'r> sqlx::Decode<'r, sqlx::MySql> + sqlx::Type<sqlx::MySql>,
        const N: usize,
    >(
        &mut self,
    ) -> Result<[T; N], LoadError> {
        let mut values = [T::default(); N];
        for value in &mut values {
            *value = self.read()?;
        }
        Ok(values)
    }
    pub(super) fn text(&mut self) -> Result<Vec<u8>, LoadError> {
        if self.next >= self.row.field_count() {
            return Err(LoadError::InvalidRow);
        }
        // Source GetStringView/AddString: SQL NULL is empty; preserve binary
        // bytes, including invalid UTF-8 and embedded NUL, without logging.
        let value = if self.row.is_null(self.next) {
            Vec::new()
        } else {
            self.row
                .try_read::<Vec<u8>>(self.next)
                .or_else(|| {
                    self.row
                        .try_read::<String>(self.next)
                        .map(String::into_bytes)
                })
                .ok_or(LoadError::InvalidRow)?
        };
        self.next += 1;
        Ok(value)
    }
    pub(super) fn finish(self) -> Result<(), LoadError> {
        if self.next == self.row.field_count() {
            Ok(())
        } else {
            Err(LoadError::InvalidRow)
        }
    }
}
