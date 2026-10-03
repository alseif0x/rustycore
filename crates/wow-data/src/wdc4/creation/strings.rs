//! Checked target name-pattern strings only. Leave legacy getters unchanged.
//! 02245dcd DB2FileLoader.cpp:354-376,798-803: sections' records concatenate
//! first, followed by concatenated string pools. A uint32 displacement is
//! relative to the physical source field, not the section's string table.
use super::{CreationDb2, CreationTable, numeric};
use crate::wdc4::CompressionType;
use anyhow::{Context, Result, ensure};

impl CreationDb2 {
    pub(crate) fn string(&self, id: u32, field: usize) -> Result<String> {
        ensure!(
            field == 0
                && matches!(
                    self.table,
                    CreationTable::NameProfanity
                        | CreationTable::NameReserved
                        | CreationTable::NameReservedLocale
                ),
            "Unported target string field"
        );
        let index = *self
            .records
            .get(&id)
            .context("Target string record absent")?;
        let info = self
            .reader
            .field_info
            .get(field)
            .context("Target string column absent")?;
        ensure!(
            info.compression == CompressionType::None && info.field_offset_bits % 8 == 0,
            "Unsupported target string displacement representation"
        );
        let relative = numeric(&self.reader, index, field, 0, 32, 1)? as usize;
        // Source nullptr is materialized as EmptyDb2String, not corruption.
        if relative == 0 {
            return Ok(String::new());
        }
        let record_bytes = (self.reader.header.record_count as usize)
            .checked_mul(self.reader.header.record_size as usize)
            .context("Target record extent overflow")?;
        ensure!(
            record_bytes == self.reader.record_data.len(),
            "Incomplete target string records"
        );
        let string_bytes = self
            .reader
            .string_tables
            .iter()
            .try_fold(0usize, |sum, table| sum.checked_add(table.len()))
            .context("Target string extent overflow")?;
        ensure!(
            string_bytes == self.reader.header.string_table_size as usize,
            "Incomplete target string pools"
        );
        let extent = record_bytes
            .checked_add(string_bytes)
            .context("Target string extent overflow")?;
        ensure!(
            relative < extent,
            "Target string displacement exceeds source bound"
        );
        let target = index
            .checked_mul(self.reader.header.record_size as usize)
            .and_then(|position| position.checked_add(usize::from(info.field_offset_bits / 8)))
            .and_then(|position| position.checked_add(relative))
            .context("Target string address overflow")?;
        let mut offset = target
            .checked_sub(record_bytes)
            .context("Target string points into record bytes")?;
        ensure!(offset < string_bytes, "Target string address outside pool");
        let mut bytes = Vec::new();
        for table in &self.reader.string_tables {
            if offset >= table.len() {
                offset -= table.len();
                continue;
            }
            let tail = &table[offset..];
            if let Some(end) = tail.iter().position(|&byte| byte == 0) {
                bytes.extend_from_slice(&tail[..end]);
                return String::from_utf8(bytes)
                    .map_err(|_| anyhow::anyhow!("Target string is not UTF-8"));
            }
            // The source materializes one contiguous pool. Do not manufacture
            // a section boundary where its C string reader has none.
            bytes.extend_from_slice(tail);
            offset = 0;
        }
        anyhow::bail!("Unterminated target string")
    }
}
